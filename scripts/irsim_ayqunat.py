#!/usr/bin/env python3
"""
irsim_ayqunat — rasterise every icon artefact Taarib ships, from two vector marks.

    python3 scripts/irsim_ayqunat.py

The sources of truth are `assets/huwiya/taarib-alama.svg`, the app mark, and
`assets/huwiya/taarib-ruqaa.svg`, the document mark a file manager draws for a
`.ruqaa` patch. Everything under `apps/studio/src-tauri/ayqunat/` is derived;
nothing in there is hand-authored and nothing in there should be edited by hand.
Before this script existed the rasters came out of one undocumented `npx tauri
icon` run against a placeholder mark, which meant they could not be regenerated,
could not be reviewed as a diff against an intent, and had drifted from the mark
the product actually uses.

WHAT IT WRITES

    ayqunat/32x32.png 128x128.png 128x128@2x.png icon.png   the bundle PNGs
    ayqunat/icon.ico                                        Windows, 9 sizes
    ayqunat/icon.icns                                       macOS, 12 chunks
    ayqunat/hicolor/<N>x<N>/apps/taarib.png                 freedesktop tree
    apps/studio/public/ayquna.png                           the web favicon

    ayqunat/ruqaa/taarib-ruqaa.svg                          the scalable copy
    ayqunat/ruqaa/<N>x<N>.png                               7 sizes, mimetypes

and removes the eleven Windows-Store (MSIX) tiles plus the old placeholder SVG
that the `tauri icon` run left behind — `bundle.targets` has never contained
`msix`, so nothing ever consumed them.

The `ruqaa/` set is what `bundle.linux.deb.files` in tauri.conf.json installs
under `hicolor/<size>/mimetypes/application-vnd.taarib.ruqaa.*`. The sizes are
the ones that mapping names; after generation this script reads the mapping back
and refuses to finish if it names an `ayqunat/` file that does not exist, because
that is precisely how the mapping and the pipeline once drifted apart and broke
`tauri build --bundles deb`.

OPTICAL CORRECTION — WHY A PIPELINE AND NOT A RESIZE

Correction one, and the largest: every size is rendered from the vector at its
native resolution. Nothing is downsampled from a big raster. A 512px master
box-filtered to 16px smears an 88-unit stroke across three pixels at partial
opacity; rasterising the same stroke directly at 16px puts its full width on the
grid with only the two edge pixels ramped. That difference is most of the fight.

The rest is craft, and it is a floor rule: no drawn feature may be thinner than
its minimum on the physical pixel grid. For a feature whose width on the 512
grid is `w`, the width it lands on at raster size `S` is `w * S / 512`, and the
gain applied is

    gain(f, S) = max(1.0, FLOOR_PX[f] / (w * S / 512))

so the correction is exactly 1.00 wherever the mark is already big enough, and
grows only as the feature approaches the grid. Three floors, three reasons:

    stroke        3.0 px   An antialiased edge eats about half a pixel per side.
                           Below 3px the stroke has under 2px of full-opacity
                           core and starts reading as grey rather than as ink.
                           On the app mark this binds at 16px only (2.75px
                           natural -> gain 1.09); the mark's own comment already
                           tuned 88 for a taskbar. The document mark's stroke is
                           64, a line on a page rather than a bar on a tile, so
                           it lands at 2.00px and takes a 1.50 gain at 16px,
                           then 3.00px exactly at 24 and nothing above.

    dot diameter  3.0 px   Same argument, and a disc is worse off than a run: it
                           is narrow in both axes at once, so it loses ink from
                           four sides. Binds at 16px on the app mark (2.50px ->
                           gain 1.20); the document's 28-radius dots bind at 16
                           (1.75px -> 1.71) and at 24 (2.63px -> 1.14).

    hairline      1.0 px   The rim exists to separate a near-black ground from a
                           dark desktop — and, turned round on the document, a
                           near-white page from a white file list. At 3 units it
                           is 0.09px at 16 and still only 0.75px at 128 — a smear
                           at a tenth opacity, which is no rim at all. Floored to
                           one whole pixel it draws what it was drawn to do. This
                           is the one feature whose floor binds above 32px: it
                           binds at every size below 171, and is a no-op at 256
                           and up where the authored 3 units already exceed a
                           pixel. Both marks author 3, so both bind alike.

Thickening alone would make the mark *worse* at 16px, so there is a fourth rule,
a negative-space floor:

    dot gap       1.0 px   The app mark's dots sit 96 units apart centre to
                           centre with 40-unit radii — 16 units of daylight,
                           which is half a pixel at 16px and vanishes entirely
                           once the discs are gained up. Two dots merged into one
                           bar destroy the only part of the mark that says
                           "Arabic" at a glance. So after gain, the centres are
                           pushed apart symmetrically about their midpoint until
                           at least one whole pixel of ground survives between
                           them. On the app mark this binds at 16, 20, 22 and
                           24px; at 32px the natural gap is already exactly
                           1.0px and nothing moves. The document's dots are 68
                           apart with 12 units of daylight, so the guard binds at
                           16, 24 and 32 and moves each dot 30 units at 16px —
                           the pair becomes seven pixels on a page ten wide,
                           which is why the mark places their midpoint where it
                           does rather than over the run's centre.

The curve is one function of five numbers — grid, stroke, dot radius, dot
centres, rim width — and both marks feed it the same way. What differs is what
the rim rides on. The app mark's ground corner radius and its hairline rect's
inset are recomputed from the corrected hairline width rather than scaled,
because that mark defines the rim as sitting wholly inside the silhouette
(inset = half the stroke width, so its outer edge lands on the canvas edge).
Widening a stroke without moving the path it rides would push half the rim
outside the viewBox to be clipped, and the visible rim would come out at half
the intended weight. The document mark is a portrait page with a folded corner,
a silhouette no single inset number describes, so it authors the rim the other
way round: a stroke on the silhouette itself, twice the wanted width, clipped to
the page, so the half that survives is the rim and lies wholly inside. For that
mark the pipeline reads the visible width as half the authored stroke and writes
back twice the corrected one. Same floor, same pixel on disk, two constructions.

The gains actually applied are printed on every run, per mark, so the curve is
auditable rather than merely asserted here.

FAILURE

Loud, and as early as it can be: a missing or structurally unexpected source
aborts before anything is rendered, a zero-byte payload aborts before it can be
written, pruning only happens after a complete successful generation, and every
artefact is re-opened and its real pixel dimensions asserted after the write.
"""

from __future__ import annotations

import json
import struct
import sys
import xml.etree.ElementTree as ET
from abc import ABC, abstractmethod
from dataclasses import dataclass
from io import BytesIO
from pathlib import Path

try:
    import cairosvg
    from PIL import Image
except ImportError as khata:  # pragma: no cover - an environment fault, not logic
    raise SystemExit(
        f"irsim_ayqunat: missing dependency ({khata}). "
        "This pipeline needs `cairosvg` and `Pillow`."
    ) from khata


JIDHR = Path(__file__).resolve().parents[1]

HUWIYA = JIDHR / "assets" / "huwiya"
ALAMA = HUWIYA / "taarib-alama.svg"
RUQAA = HUWIYA / "taarib-ruqaa.svg"
TAURI = JIDHR / "apps" / "studio" / "src-tauri"
AYQUNAT = TAURI / "ayqunat"
HICOLOR = AYQUNAT / "hicolor"
AYQUNAT_RUQAA = AYQUNAT / "ruqaa"
AMM = JIDHR / "apps" / "studio" / "public"
TAURI_CONF = TAURI / "tauri.conf.json"

SVG_NS = "http://www.w3.org/2000/svg"


# ---------------------------------------------------------------------------
# The output set.
# ---------------------------------------------------------------------------

# Tauri reads a PNG's real pixel dimensions, not its filename, when it lays out
# the Linux hicolor tree at bundle time — but the filenames still have to match
# what `bundle.icon` names. The 512 is `icon.png` rather than a new
# `512x512.png` because a 512 raster already lives at that path, and shipping
# the same bytes twice under two names is a duplicate inside every bundle.
AYQUNAT_PNG: tuple[tuple[str, int], ...] = (
    ("32x32.png", 32),
    ("128x128.png", 128),
    ("128x128@2x.png", 256),
    ("icon.png", 512),
)

# 20 and 40 are the sizes Windows asks for at 125% and 250% display scaling.
# Neither was in the shipped .ico, so both of those DPI steps were being served
# by Windows shrinking the 24 and the 48 itself, at run time, badly.
ICO_HUJUM: tuple[int, ...] = (16, 20, 24, 32, 40, 48, 64, 128, 256)

# ASCII order, which is also roughly Apple's own. `ic04`/`ic05` are additionally
# defined as ARGB blobs rather than PNG and readers differ on whether they sniff
# the payload, so `icp4`/`icp5` carry the identical 16 and 32 rasters as
# unambiguous PNG: a reader that rejects one still finds the other. The file
# this replaces had neither pair, which is why Finder's list and sidebar views
# were downsampling from the 128.
ICNS_ANWA: tuple[tuple[bytes, int], ...] = (
    (b"ic04", 16),
    (b"ic05", 32),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),  # 16 @2x
    (b"ic12", 64),  # 32 @2x
    (b"ic13", 256),  # 128 @2x
    (b"ic14", 512),  # 256 @2x
    (b"icp4", 16),
    (b"icp5", 32),
)

HICOLOR_HUJUM: tuple[int, ...] = (16, 22, 24, 32, 48, 64, 128, 256, 512)

# Must match `Icon=taarib` in src-tauri/linux/taarib.desktop, or the desktop
# entry names an icon the installed theme does not contain.
HICOLOR_ISM = "taarib"

# 32 is what a browser tab actually asks for, and what a 16pt tab wants on a 2x
# display. It goes into Vite's `public/`, copied verbatim into `dist/`, rather
# than being referenced across into the Rust tree.
AMM_AYQUNA: tuple[str, int] = ("ayquna.png", 32)

# The document mark is not in `bundle.icon` — Tauri has no notion of a mimetype
# icon — so it reaches the deb only through the explicit `bundle.linux.deb.files`
# mapping, which names these files one by one. The scalable copy is the source
# itself, verbatim: a theme's `scalable/` entry is what a toolkit renders for any
# size the fixed directories lack, and the correction curve is 1.00 there.
RUQAA_SVG = "taarib-ruqaa.svg"
RUQAA_HUJUM: tuple[int, ...] = (16, 24, 32, 48, 64, 128, 256)

# Written by the one `npx tauri icon` run, referenced by nothing. Named
# explicitly rather than globbed: this script may only delete files it knows.
MAHDHUFAT: tuple[str, ...] = (
    "Square30x30Logo.png",
    "Square44x44Logo.png",
    "Square71x71Logo.png",
    "Square89x89Logo.png",
    "Square107x107Logo.png",
    "Square142x142Logo.png",
    "Square150x150Logo.png",
    "Square284x284Logo.png",
    "Square310x310Logo.png",
    "StoreLogo.png",
    "taarib.svg",
)


# ---------------------------------------------------------------------------
# Optical correction.
# ---------------------------------------------------------------------------

FLOOR_STROKE_PX = 3.0
FLOOR_DOT_PX = 3.0
FLOOR_HAIR_PX = 1.0
FLOOR_GAP_PX = 1.0


class KhataIrsim(RuntimeError):
    """Anything that would leave the icon set unreproducible, wrong, or absent."""


@dataclass(frozen=True, slots=True)
class Handasa:
    """The numbers the correction curve reads, taken off the source rather than assumed."""

    shabaka: float  # the design grid — 512
    stroke: float  # the run's stroke-width
    dot_r: float  # dot radius
    dot_x: tuple[float, float]  # the two dot centres, in document order
    hair: float  # the rim's visible width


@dataclass(frozen=True, slots=True)
class Tashih:
    """The correction resolved for one raster size."""

    hajm: int
    kasb_stroke: float  # multiplier on stroke-width
    kasb_dot: float  # multiplier on dot radius
    hair: float  # absolute visible rim width in grid units — floored, not gained
    izaha: float  # grid units each dot centre moves out from their midpoint

    def rim_px(self, shabaka: float) -> float:
        return self.hair * self.hajm / shabaka

    def gap_px(self, h: Handasa) -> float:
        masafa = abs(h.dot_x[0] - h.dot_x[1]) + 2.0 * self.izaha
        return (masafa - 2.0 * h.dot_r * self.kasb_dot) * self.hajm / h.shabaka


def tashih(h: Handasa, hajm: int) -> Tashih:
    """Resolve the correction curve documented at the top of this file."""
    k = hajm / h.shabaka  # grid units -> device pixels

    kasb_stroke = max(1.0, FLOOR_STROKE_PX / (h.stroke * k))
    kasb_dot = max(1.0, FLOOR_DOT_PX / (2.0 * h.dot_r * k))
    hair = max(h.hair, FLOOR_HAIR_PX / k)

    # Negative space is floored after the discs have grown, never before, or the
    # gain would eat the very gap the guard was computed to protect.
    nisf = h.dot_r * kasb_dot
    matlub = 2.0 * nisf + FLOOR_GAP_PX / k
    izaha = max(0.0, (matlub - abs(h.dot_x[0] - h.dot_x[1])) / 2.0)

    return Tashih(hajm, kasb_stroke, kasb_dot, hair, izaha)


# ---------------------------------------------------------------------------
# Reading and re-drawing the marks.
# ---------------------------------------------------------------------------


def _tag(unsur: ET.Element) -> str:
    return unsur.tag.rsplit("}", 1)[-1]


def _adad(unsur: ET.Element, ism: str) -> float:
    qima = unsur.get(ism)
    if qima is None:
        raise KhataIrsim(f"<{_tag(unsur)}> in the mark has no `{ism}` attribute")
    try:
        return float(qima)
    except ValueError as khata:
        raise KhataIrsim(f'`{ism}="{qima}"` on <{_tag(unsur)}> is not a number') from khata


def _raqm(qima: float) -> str:
    """Compact SVG number — no exponent, no trailing zeros."""
    return f"{qima:.4f}".rstrip("0").rstrip(".") or "0"


def _wahid(anasir: list[ET.Element], wasf: str) -> ET.Element:
    if len(anasir) != 1:
        raise KhataIrsim(f"expected exactly 1 {wasf}, found {len(anasir)}")
    return anasir[0]


def _fi_defs(jidhr: ET.Element, unsur: ET.Element) -> bool:
    return any(unsur in list(d.iter()) for d in jidhr if _tag(d) == "defs")


def _al_masar(jidhr: ET.Element) -> ET.Element:
    """The run: the one stroked, unfilled <path> — a silhouette path carries neither."""
    return _wahid(
        [
            u
            for u in jidhr.iter()
            if _tag(u) == "path" and u.get("stroke") is not None and u.get("fill") == "none"
        ],
        "stroked <path>",
    )


def _al_dawair(jidhr: ET.Element) -> list[ET.Element]:
    dawair = [u for u in jidhr.iter() if _tag(u) == "circle"]
    if len(dawair) != 2:
        raise KhataIrsim(f"expected exactly 2 dot <circle>, found {len(dawair)}")
    return dawair


class Marsam(ABC):
    """
    One mark, parsed once, re-drawn per size.

    Both marks are a stepped run, two dots and a hairline rim; they differ only
    in what the rim rides on — the tile insets a square rect, the page clips a
    stroke to a silhouette with a fold. So this base owns everything the curve
    touches, the run and the dots, and asks a subclass two questions: what is
    the rim's visible width as authored, and how is the rim rebuilt at a
    corrected width.

    Features are selected by role — a filled shape with no stroke is the
    ground, a stroked shape with no fill is a rim or the run — rather than by
    document position, so a reordered source still works. Everything is
    asserted, because a correction computed against a mark that has since been
    redrawn is worse than no correction at all: it would silently thicken the
    wrong thing.
    """

    def __init__(self, masdar: Path) -> None:
        if not masdar.is_file():
            raise KhataIrsim(
                f"the source mark is missing: {masdar}\n"
                "Every icon in this repository is derived from the marks under "
                f"{_nisbi(HUWIYA)}; there is nothing to fall back to."
            )
        try:
            jidhr = ET.parse(masdar).getroot()
        except ET.ParseError as khata:
            raise KhataIrsim(f"{masdar} is not well-formed XML: {khata}") from khata

        if _tag(jidhr) != "svg":
            raise KhataIrsim(f"{masdar} root is <{_tag(jidhr)}>, expected <svg>")

        view = (jidhr.get("viewBox") or "").split()
        if len(view) != 4 or view[0] != "0" or view[1] != "0" or view[2] != view[3]:
            raise KhataIrsim(
                "expected a square viewBox anchored at the origin, "
                f"got {jidhr.get('viewBox')!r}"
            )

        try:
            masar = _al_masar(jidhr)
            dawair = _al_dawair(jidhr)
            hair = self._iqra_shaara(jidhr, float(view[2]))
        except KhataIrsim as khata:
            raise KhataIrsim(f"{_nisbi(masdar)}: {khata}") from khata

        nisf = {_adad(d, "r") for d in dawair}
        if len(nisf) != 1:
            raise KhataIrsim(f"{_nisbi(masdar)}: the two dots have different radii: {sorted(nisf)}")
        if len({_adad(d, "cy") for d in dawair}) != 1:
            raise KhataIrsim(
                f"{_nisbi(masdar)}: the two dots are not on one horizontal — "
                "the gap guard assumes they are"
            )

        self.masdar = masdar
        self.handasa = Handasa(
            shabaka=float(view[2]),
            stroke=_adad(masar, "stroke-width"),
            dot_r=nisf.pop(),
            dot_x=(_adad(dawair[0], "cx"), _adad(dawair[1], "cx")),
            hair=hair,
        )

        ET.register_namespace("", SVG_NS)
        self._nass = ET.tostring(jidhr, encoding="utf-8")
        self._khazina: dict[int, tuple[bytes, Tashih]] = {}

    # -- what differs between the marks ------------------------------------

    @abstractmethod
    def _iqra_shaara(self, jidhr: ET.Element, shabaka: float) -> float:
        """Assert this mark's ground and rim structure; return the rim's visible width."""

    @abstractmethod
    def _ubni_shaara(self, jidhr: ET.Element, hair: float) -> None:
        """Redraw the rim in `jidhr` so that `hair` grid units are visible, wholly inside."""

    # -- rendering ----------------------------------------------------------

    def biksilat(self, hajm: int) -> tuple[bytes, Tashih]:
        """
        PNG bytes for the corrected mark at `hajm` device pixels square.

        Memoised: 32px is wanted by five different artefacts and every one of
        them has to receive byte-identical pixels.
        """
        mukhazzan = self._khazina.get(hajm)
        if mukhazzan is None:
            mukhazzan = self._ursum(hajm)
            self._khazina[hajm] = mukhazzan
        return mukhazzan

    def sura(self, hajm: int) -> Image.Image:
        """The corrected mark as an RGBA image, for the container writers."""
        with Image.open(BytesIO(self.biksilat(hajm)[0])) as maftuh:
            return maftuh.convert("RGBA")

    @property
    def hujum_musahhaha(self) -> list[int]:
        return sorted(self._khazina)

    def _ursum(self, hajm: int) -> tuple[bytes, Tashih]:
        h = self.handasa
        t = tashih(h, hajm)
        jidhr = ET.fromstring(self._nass)

        _al_masar(jidhr).set("stroke-width", _raqm(h.stroke * t.kasb_stroke))

        wasat = sum(h.dot_x) / 2.0
        for dawra, markaz in zip(_al_dawair(jidhr), h.dot_x, strict=True):
            ittijah = 1.0 if markaz >= wasat else -1.0
            dawra.set("r", _raqm(h.dot_r * t.kasb_dot))
            dawra.set("cx", _raqm(markaz + ittijah * t.izaha))

        self._ubni_shaara(jidhr, t.hair)

        kham = cairosvg.svg2png(
            bytestring=ET.tostring(jidhr, encoding="utf-8", xml_declaration=True),
            output_width=hajm,
            output_height=hajm,
        )
        if not kham:
            raise KhataIrsim(f"cairosvg produced no bytes at {hajm}px")
        with Image.open(BytesIO(kham)) as im:
            if im.size != (hajm, hajm):
                raise KhataIrsim(f"cairosvg produced {im.size} when asked for {hajm}x{hajm}")

        return kham, t


class MarsamAlama(Marsam):
    """The app mark: a square ground spanning the grid, rimmed by an inset rect."""

    def _iqra_shaara(self, jidhr: ET.Element, shabaka: float) -> float:
        rects = [u for u in jidhr.iter() if _tag(u) == "rect"]
        ard = _wahid(
            [u for u in rects if u.get("fill", "none") != "none" and u.get("stroke") is None],
            "filled ground <rect>",
        )
        shaara = _wahid(
            [u for u in rects if u.get("stroke") is not None], "stroked hairline <rect>"
        )

        # The rim is rebuilt as a full-grid square; a ground that is anything
        # else would get a rim that no longer traces its edge.
        if _adad(ard, "width") != shabaka or _adad(ard, "height") != shabaka:
            raise KhataIrsim("the ground <rect> does not span the grid")
        self._rukn = _adad(ard, "rx")
        return _adad(shaara, "stroke-width")

    def _ubni_shaara(self, jidhr: ET.Element, hair: float) -> None:
        shaara = next(
            u for u in jidhr.iter() if _tag(u) == "rect" and u.get("stroke") is not None
        )
        shabaka = self.handasa.shabaka
        nisf_hair = hair / 2.0
        shaara.set("stroke-width", _raqm(hair))
        shaara.set("x", _raqm(nisf_hair))
        shaara.set("y", _raqm(nisf_hair))
        shaara.set("width", _raqm(shabaka - hair))
        shaara.set("height", _raqm(shabaka - hair))
        shaara.set("rx", _raqm(max(0.0, self._rukn - nisf_hair)))


class MarsamRuqaa(Marsam):
    """
    The document mark: a page silhouette defined once in <defs>, drawn by two
    <use>s — one filled for the ground, one stroked and clipped to the same
    silhouette for the rim. Half the rim's stroke is thrown away by the clip,
    which is the whole point: it is the only way a shape with a fold gets a
    rim of exact width that never leaks outside its own edge.
    """

    def _iqra_shaara(self, jidhr: ET.Element, shabaka: float) -> float:
        defs = _wahid([u for u in jidhr if _tag(u) == "defs"], "<defs>")
        safha = _wahid(
            [
                u
                for u in defs.iter()
                if _tag(u) == "path" and u.get("fill") is None and u.get("stroke") is None
            ],
            "unpainted silhouette <path> in <defs>",
        )
        ism = safha.get("id")
        if not ism:
            raise KhataIrsim("the silhouette <path> has no id for the <use>s to reference")

        qass = _wahid([u for u in defs.iter() if _tag(u) == "clipPath"], "<clipPath> in <defs>")
        qass_ism = qass.get("id")
        if not qass_ism:
            raise KhataIrsim("the <clipPath> has no id for the rim to reference")
        muhtawa = _wahid(list(qass), "child of <clipPath>")
        if _tag(muhtawa) != "use" or muhtawa.get("href") != f"#{ism}":
            raise KhataIrsim(f"the <clipPath> must hold one <use href=\"#{ism}\"> and nothing else")

        mustakhdam = [
            u for u in jidhr.iter() if _tag(u) == "use" and not _fi_defs(jidhr, u)
        ]
        if any(u.get("href") != f"#{ism}" for u in mustakhdam):
            raise KhataIrsim(f"every drawn <use> must reference the silhouette #{ism}")
        _wahid(
            [u for u in mustakhdam if u.get("fill", "none") != "none" and u.get("stroke") is None],
            "filled ground <use>",
        )
        shaara = self._al_shaara(mustakhdam)
        if shaara.get("clip-path") != f"url(#{qass_ism})":
            raise KhataIrsim(
                f"the rim <use> must carry clip-path=\"url(#{qass_ism})\", or half of it "
                "lands outside the page"
            )

        # Only the inner half of the stroke survives the clip: the visible rim
        # is half of what is authored, and that half is what the floor governs.
        return _adad(shaara, "stroke-width") / 2.0

    @staticmethod
    def _al_shaara(mustakhdam: list[ET.Element]) -> ET.Element:
        return _wahid(
            [u for u in mustakhdam if u.get("stroke") is not None and u.get("fill") == "none"],
            "stroked rim <use>",
        )

    def _ubni_shaara(self, jidhr: ET.Element, hair: float) -> None:
        mustakhdam = [
            u for u in jidhr.iter() if _tag(u) == "use" and not _fi_defs(jidhr, u)
        ]
        self._al_shaara(mustakhdam).set("stroke-width", _raqm(2.0 * hair))


# ---------------------------------------------------------------------------
# Containers.
# ---------------------------------------------------------------------------


def ibn_ico(marsam: Marsam, hujum: tuple[int, ...]) -> bytes:
    """
    A multi-size .ico with one natively-rendered frame per size.

    Pillow reuses a supplied frame only when its size matches a requested entry
    exactly, and otherwise falls back to Lanczos-shrinking the base image —
    which is precisely the downsampling this pipeline exists to avoid. So every
    size is handed in through `append_images` with the largest as the base,
    which guarantees an exact match for all nine and no resampling anywhere.

    Frames are PNG-compressed at every size: that is what the file this replaces
    already did, what Windows 10+ and NSIS 3 both read, and what keeps the 256
    entry from costing a quarter of a megabyte as a raw bottom-up DIB.
    """
    itarat = [marsam.sura(hajm) for hajm in sorted(hujum, reverse=True)]
    hafiza = BytesIO()
    itarat[0].save(
        hafiza,
        format="ICO",
        sizes=[(hajm, hajm) for hajm in hujum],
        append_images=itarat[1:],
    )
    return hafiza.getvalue()


def ibn_icns(marsam: Marsam, anwa: tuple[tuple[bytes, int], ...]) -> bytes:
    """
    An .icns container, written directly.

    The format is the magic `icns`, a big-endian total length that includes that
    8-byte header, then a flat sequence of `<4-byte OSType><be32 length><payload>`
    chunks whose length likewise counts its own 8-byte header. The leading
    `TOC ` chunk is an index of everything after it — type and length per entry —
    which lets a reader seek straight to one representation instead of walking
    the whole file. It is optional; it is written because the file being replaced
    carried one and dropping it would be a silent regression for anything that
    depends on it.
    """
    qitaa = [(naw, marsam.biksilat(hajm)[0]) for naw, hajm in anwa]

    def uqda(naw: bytes, himl: bytes) -> bytes:
        if len(naw) != 4:
            raise KhataIrsim(f"{naw!r} is not a 4-byte icns OSType")
        if not himl:
            raise KhataIrsim(f"icns chunk {naw!r} would be empty")
        return naw + struct.pack(">I", 8 + len(himl)) + himl

    fihris = b"".join(naw + struct.pack(">I", 8 + len(himl)) for naw, himl in qitaa)
    jism = uqda(b"TOC ", fihris) + b"".join(uqda(naw, himl) for naw, himl in qitaa)
    return b"icns" + struct.pack(">I", 8 + len(jism)) + jism


# ---------------------------------------------------------------------------
# Writing.
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class Satr:
    """One row of the report."""

    masar: str
    wasf: str
    bayt: int
    hal: str


def _nisbi(masar: Path) -> str:
    try:
        return masar.resolve().relative_to(JIDHR).as_posix()
    except ValueError:
        return str(masar)


def uktub(masar: Path, mahtawa: bytes, wasf: str) -> Satr:
    """
    Write `mahtawa` to `masar`, skipping the write when the bytes already match.

    The content compare is what makes re-running idempotent in the sense that
    matters here: an unchanged mark leaves mtimes alone, so a second run
    produces no git churn and triggers no rebuild in anything watching the tree.
    The write itself goes through a sibling temp file and an atomic replace, so
    an interrupted run can never leave a half-written icon on disk — a truncated
    .icns is a file every reader accepts the header of and then chokes on.
    """
    if not mahtawa:
        raise KhataIrsim(f"refusing to write a zero-byte artefact: {masar}")

    if masar.is_file() and masar.read_bytes() == mahtawa:
        return Satr(_nisbi(masar), wasf, len(mahtawa), "unchanged")

    masar.parent.mkdir(parents=True, exist_ok=True)
    muaqqat = masar.with_name(f".{masar.name}.tmp")
    try:
        muaqqat.write_bytes(mahtawa)
        muaqqat.replace(masar)
    finally:
        muaqqat.unlink(missing_ok=True)

    return Satr(_nisbi(masar), wasf, len(mahtawa), "written")


# ---------------------------------------------------------------------------
# Verification — every artefact is read back off disk, not trusted.
# ---------------------------------------------------------------------------


def tahaqqaq_png(masar: Path, hajm: int) -> None:
    with Image.open(masar) as im:
        if im.size != (hajm, hajm):
            raise KhataIrsim(f"{_nisbi(masar)} is {im.size}, expected ({hajm}, {hajm})")
        if im.mode not in {"RGBA", "LA", "PA"}:
            raise KhataIrsim(
                f"{_nisbi(masar)} is mode {im.mode}; the mark needs an alpha channel"
            )
        im.load()


def tahaqqaq_svg(masar: Path, masdar: Path) -> None:
    """The shipped scalable copy must be the source, byte for byte, and must parse as it."""
    if masar.read_bytes() != masdar.read_bytes():
        raise KhataIrsim(f"{_nisbi(masar)} is not a verbatim copy of {_nisbi(masdar)}")
    try:
        jidhr = ET.parse(masar).getroot()
    except ET.ParseError as khata:
        raise KhataIrsim(f"{_nisbi(masar)} is not well-formed XML: {khata}") from khata
    if _tag(jidhr) != "svg" or not jidhr.get("viewBox"):
        raise KhataIrsim(f"{_nisbi(masar)} is not an <svg> with a viewBox")


def tahaqqaq_ico(masar: Path, hujum: tuple[int, ...]) -> None:
    kham = masar.read_bytes()
    hajiz, naw, adad = struct.unpack_from("<HHH", kham, 0)
    if hajiz != 0 or naw != 1:
        raise KhataIrsim(f"{_nisbi(masar)} is not an ICO (reserved={hajiz}, type={naw})")
    if adad != len(hujum):
        raise KhataIrsim(f"{_nisbi(masar)} holds {adad} entries, expected {len(hujum)}")

    mawjud: list[int] = []
    for i in range(adad):
        ard, irtifa, _lawn, _hajiz, _tabaqat, _bit, tul, izaha = struct.unpack_from(
            "<BBBBHHII", kham, 6 + 16 * i
        )
        ard, irtifa = ard or 256, irtifa or 256
        if tul == 0 or izaha + tul > len(kham):
            raise KhataIrsim(f"{_nisbi(masar)} entry {i} points outside the file")
        with Image.open(BytesIO(kham[izaha : izaha + tul])) as im:
            if im.size != (ard, irtifa):
                raise KhataIrsim(
                    f"{_nisbi(masar)} entry {i} declares {ard}x{irtifa} but decodes {im.size}"
                )
            im.load()
        mawjud.append(ard)

    if sorted(mawjud) != sorted(hujum):
        raise KhataIrsim(f"{_nisbi(masar)} holds {sorted(mawjud)}, expected {sorted(hujum)}")


def tahaqqaq_icns(masar: Path, anwa: tuple[tuple[bytes, int], ...]) -> None:
    kham = masar.read_bytes()
    if kham[:4] != b"icns":
        raise KhataIrsim(f"{_nisbi(masar)} does not start with the icns magic")
    (mualan,) = struct.unpack_from(">I", kham, 4)
    if mualan != len(kham):
        raise KhataIrsim(f"{_nisbi(masar)} declares {mualan} bytes but is {len(kham)}")

    wujida: dict[bytes, int] = {}
    izaha = 8
    while izaha < len(kham):
        naw = kham[izaha : izaha + 4]
        (tul,) = struct.unpack_from(">I", kham, izaha + 4)
        if tul < 8 or izaha + tul > len(kham):
            raise KhataIrsim(f"{_nisbi(masar)} chunk {naw!r} has an impossible length {tul}")
        if naw != b"TOC ":
            with Image.open(BytesIO(kham[izaha + 8 : izaha + tul])) as im:
                if im.size[0] != im.size[1]:
                    raise KhataIrsim(f"{_nisbi(masar)} chunk {naw!r} is {im.size}, not square")
                im.load()
                wujida[naw] = im.size[0]
        izaha += tul

    for naw, hajm in anwa:
        if wujida.get(naw) != hajm:
            raise KhataIrsim(
                f"{_nisbi(masar)} chunk {naw.decode()} is "
                f"{wujida.get(naw, 'absent')}, expected {hajm}px"
            )
    zaid = set(wujida) - {naw for naw, _ in anwa}
    if zaid:
        raise KhataIrsim(f"{_nisbi(masar)} holds unexpected chunks: {sorted(zaid)}")


def tahaqqaq_deb(conf: Path) -> list[tuple[str, ...]]:
    """
    Every `ayqunat/` source that `bundle.linux.deb.files` maps must exist.

    `tauri build --bundles deb` resolves those sources relative to src-tauri and
    fails, late and after a full compile, on the first one it cannot find. This
    is the check that catches a mapping added without its artefact — or an
    artefact renamed out from under its mapping — at the moment the icons are
    generated, when it is cheap.
    """
    try:
        with conf.open(encoding="utf-8") as maftuh:
            khutta = json.load(maftuh)
    except (OSError, ValueError) as khata:
        raise KhataIrsim(f"{_nisbi(conf)} could not be read as JSON: {khata}") from khata

    milaffat = khutta.get("bundle", {}).get("linux", {}).get("deb", {}).get("files", {})
    if not isinstance(milaffat, dict):
        raise KhataIrsim(f"{_nisbi(conf)}: bundle.linux.deb.files is not an object")

    sufuf: list[tuple[str, ...]] = []
    naqis: list[str] = []
    for hadaf, masdar in sorted(milaffat.items()):
        if not isinstance(masdar, str) or not masdar.startswith("ayqunat/"):
            continue
        mawjud = (conf.parent / masdar).is_file()
        sufuf.append((masdar, hadaf, "present" if mawjud else "MISSING"))
        if not mawjud:
            naqis.append(masdar)
    if naqis:
        raise KhataIrsim(
            f"{_nisbi(conf)} maps {len(naqis)} icon file(s) that do not exist: "
            + ", ".join(naqis)
        )
    return sufuf


# ---------------------------------------------------------------------------
# Report.
# ---------------------------------------------------------------------------

_YAMIN = {"bytes", "px", "ink x", "dots x", "rim px", "gap px"}


def jadwal(unwan: str, ruus: tuple[str, ...], sufuf: list[tuple[str, ...]]) -> None:
    if not sufuf:
        return
    urud = [max(len(ruus[i]), max(len(s[i]) for s in sufuf)) for i in range(len(ruus))]
    mahadhat = [">" if ras in _YAMIN else "<" for ras in ruus]

    def satr(qiyam: tuple[str, ...]) -> str:
        return "  ".join(f"{q:{mahadhat[i]}{urud[i]}}" for i, q in enumerate(qiyam))

    print(f"\n{unwan}")
    print("  " + satr(ruus))
    print("  " + "  ".join("-" * u for u in urud))
    for s in sufuf:
        print("  " + satr(s))


def jadwal_tashih(marsam: Marsam) -> None:
    handasa = marsam.handasa
    jadwal(
        f"optical correction applied to {_nisbi(marsam.masdar)} "
        "(1.00 = the mark exactly as drawn)",
        ("px", "ink x", "dots x", "rim px", "gap px"),
        [
            (
                str(hajm),
                f"{t.kasb_stroke:.2f}",
                f"{t.kasb_dot:.2f}",
                f"{t.rim_px(handasa.shabaka):.2f}",
                f"{t.gap_px(handasa):.2f}",
            )
            for hajm, t in ((h, marsam.biksilat(h)[1]) for h in marsam.hujum_musahhaha)
        ],
    )


def main() -> int:
    try:
        alama = MarsamAlama(ALAMA)
        ruqaa = MarsamRuqaa(RUQAA)
    except KhataIrsim as khata:
        print(f"irsim_ayqunat: {khata}", file=sys.stderr)
        return 1

    sufuf: list[Satr] = []
    try:
        for ism, hajm in AYQUNAT_PNG:
            sufuf.append(uktub(AYQUNAT / ism, alama.biksilat(hajm)[0], f"png {hajm}"))

        sufuf.append(
            uktub(
                AYQUNAT / "icon.ico",
                ibn_ico(alama, ICO_HUJUM),
                "ico " + ",".join(map(str, ICO_HUJUM)),
            )
        )
        sufuf.append(
            uktub(
                AYQUNAT / "icon.icns",
                ibn_icns(alama, ICNS_ANWA),
                "icns " + ",".join(naw.decode() for naw, _ in ICNS_ANWA),
            )
        )

        for hajm in HICOLOR_HUJUM:
            hadaf = HICOLOR / f"{hajm}x{hajm}" / "apps" / f"{HICOLOR_ISM}.png"
            sufuf.append(uktub(hadaf, alama.biksilat(hajm)[0], f"hicolor {hajm}"))

        ism, hajm = AMM_AYQUNA
        sufuf.append(uktub(AMM / ism, alama.biksilat(hajm)[0], f"favicon {hajm}"))

        sufuf.append(uktub(AYQUNAT_RUQAA / RUQAA_SVG, RUQAA.read_bytes(), "svg scalable"))
        for hajm in RUQAA_HUJUM:
            hadaf = AYQUNAT_RUQAA / f"{hajm}x{hajm}.png"
            sufuf.append(uktub(hadaf, ruqaa.biksilat(hajm)[0], f"mimetype {hajm}"))
    except (KhataIrsim, OSError) as khata:
        print(f"irsim_ayqunat: {khata}", file=sys.stderr)
        return 1

    # Pruning runs only after a complete successful generation, so a failed run
    # never leaves the directory holding neither the old files nor the new ones.
    mahdhuf: list[tuple[str, ...]] = []
    for ism in MAHDHUFAT:
        hadaf = AYQUNAT / ism
        mawjud = hadaf.is_file()
        if mawjud:
            hadaf.unlink()
        mahdhuf.append((_nisbi(hadaf), "removed" if mawjud else "absent"))

    try:
        for ism, hajm in AYQUNAT_PNG:
            tahaqqaq_png(AYQUNAT / ism, hajm)
        for hajm in HICOLOR_HUJUM:
            tahaqqaq_png(HICOLOR / f"{hajm}x{hajm}" / "apps" / f"{HICOLOR_ISM}.png", hajm)
        tahaqqaq_png(AMM / AMM_AYQUNA[0], AMM_AYQUNA[1])
        tahaqqaq_ico(AYQUNAT / "icon.ico", ICO_HUJUM)
        tahaqqaq_icns(AYQUNAT / "icon.icns", ICNS_ANWA)
        tahaqqaq_svg(AYQUNAT_RUQAA / RUQAA_SVG, RUQAA)
        for hajm in RUQAA_HUJUM:
            tahaqqaq_png(AYQUNAT_RUQAA / f"{hajm}x{hajm}.png", hajm)
        deb = tahaqqaq_deb(TAURI_CONF)
    except (KhataIrsim, OSError) as khata:
        print(f"irsim_ayqunat: verification failed: {khata}", file=sys.stderr)
        return 1

    jadwal(
        f"rendered from {_nisbi(ALAMA)} and {_nisbi(RUQAA)}",
        ("artefact", "kind", "bytes", "status"),
        [(s.masar, s.wasf, f"{s.bayt:,}", s.hal) for s in sufuf],
    )
    jadwal_tashih(alama)
    jadwal_tashih(ruqaa)

    jadwal(
        f"{_nisbi(TAURI_CONF)} bundle.linux.deb.files — icon sources resolved against src-tauri",
        ("source", "installs to", "status"),
        deb,
    )

    jadwal(
        "pruned — MSIX tiles and the placeholder mark, referenced by nothing",
        ("artefact", "status"),
        mahdhuf,
    )

    print(
        f"\n{sum(1 for s in sufuf if s.hal == 'written')} written, "
        f"{sum(1 for s in sufuf if s.hal == 'unchanged')} unchanged, "
        f"{sum(1 for m in mahdhuf if m[1] == 'removed')} removed.\n"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
