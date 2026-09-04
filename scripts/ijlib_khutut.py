#!/usr/bin/env python3
"""Stages the font faces the Studio webview embeds, verified against the lock.

The webview is a second consumer of the fonts. `taarib-tajmee` stages the whole
ten-family tree into a bundle's resource directory for the renderer; this tool
stages only the handful of faces `apps/studio/src/nizam/tokens.css` names first
in its family stacks, into `src/` where Vite fingerprints them and emits them
into the interface asset graph. Both read the same lock, so the two copies of a
face are the same bytes or the run fails.

Standard library only, by design: this runs before `pnpm install` on a clean
checkout and must not need one.

    python3 scripts/ijlib_khutut.py            # fetch what is missing, verify, report
    python3 scripts/ijlib_khutut.py --tahaqquq # verify what is on disk, touch no network
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json
import os
import re
import struct
import sys
import tempfile
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Final

JIDHR: Final = Path(__file__).resolve().parents[1]
QUFL: Final = JIDHR / 'assets' / 'aqfal' / 'qufl_khutut.json'
WIJHA: Final = JIDHR / 'apps' / 'studio' / 'src' / 'khutut'
QAIDA: Final = WIJHA / 'khutut.css'

MUHLAT_JALB: Final = 60
MUDIFAT_MASMUHA: Final = frozenset({'raw.githubusercontent.com'})

# Categories a font is not wrong for omitting: unassigned, control, format,
# surrogate and private-use codepoints carry no glyph of their own.
ASNAF_GHAYR_MARIYA: Final = frozenset({'Cn', 'Cc', 'Cf', 'Cs', 'Co'})


class KhataIjlib(RuntimeError):
    """Anything that must stop the run before a single byte reaches disk."""


@dataclass(frozen=True)
class Wajh:
    """One embedded face: its lock identifier and how `khutut.css` must name it."""

    muarrif: str
    aila: str
    wazn: int
    namat: str = 'normal'


# The interface set, and the whole of it. The other families in the lock are the
# game-side glyph atlas: they are rasterized into a patch by the Rust renderer
# and never reach the webview, so embedding them here would add megabytes to a
# bundle that no stylesheet can address.
#
# Plex Sans is here for one reason. Plex Sans Arabic carries a metric-matched
# Latin, but only 15 of the 128 codepoints in Latin Extended-A, and this
# interface renders game titles the user did not choose: Ōkami, Braid Anniversary
# Edition, Amnésie. Without the three Latin faces the stack's second name
# resolves to nothing and a single macron falls through to whatever the operating
# system offers — which on a bare Linux box is tofu, mid-title, in the product
# whose stated gate is that no glyph reaches a system face.
WUJUH: Final[tuple[Wajh, ...]] = (
    Wajh('IBMPlexSansArabic/IBMPlexSansArabic-Regular.ttf', 'IBM Plex Sans Arabic', 400),
    Wajh('IBMPlexSansArabic/IBMPlexSansArabic-Medium.ttf', 'IBM Plex Sans Arabic', 500),
    Wajh('IBMPlexSansArabic/IBMPlexSansArabic-SemiBold.ttf', 'IBM Plex Sans Arabic', 600),
    Wajh('IBMPlexSans/IBMPlexSans-Regular.ttf', 'IBM Plex Sans', 400),
    Wajh('IBMPlexSans/IBMPlexSans-Medium.ttf', 'IBM Plex Sans', 500),
    Wajh('IBMPlexSans/IBMPlexSans-SemiBold.ttf', 'IBM Plex Sans', 600),
    Wajh('IBMPlexMono/IBMPlexMono-Regular.ttf', 'IBM Plex Mono', 400),
    Wajh('IBMPlexMono/IBMPlexMono-Medium.ttf', 'IBM Plex Mono', 500),
)

# OFL-1.1 §2: the licence travels with every copy of the Font Software, and a
# copy compiled into the webview's asset bundle is a copy. Both Plex families
# resolve to the same upstream LICENSE.txt, so one file discharges both.
RUKHSA_MUARRIF: Final = 'IBMPlexSansArabic/rukhsa'
RUKHSA_MALAF: Final = 'IBMPlex-OFL.txt'

# What the interface actually has to render. Presentation forms are listed
# because a webview that cannot use the font's GSUB falls back to them; Taarib's
# own pipeline decomposes them and never requests one.
NITAQAT: Final[tuple[tuple[str, int, int], ...]] = (
    ('U+0600-06FF Arabic', 0x0600, 0x06FF),
    ('U+0660-0669 Arabic-Indic digits', 0x0660, 0x0669),
    ('U+0750-077F Arabic Supplement', 0x0750, 0x077F),
    ('U+08A0-08FF Arabic Extended-A', 0x08A0, 0x08FF),
    ('U+FB50-FDFF Presentation Forms-A', 0xFB50, 0xFDFF),
    ('U+FE70-FEFF Presentation Forms-B', 0xFE70, 0xFEFF),
    ('U+0020-007E Basic Latin', 0x0020, 0x007E),
    ('U+00A0-00FF Latin-1 Supplement', 0x00A0, 0x00FF),
    ('U+0100-017F Latin Extended-A', 0x0100, 0x017F),
    ('U+2000-206F General Punctuation', 0x2000, 0x206F),
)


# --------------------------------------------------------------------------
# الجلب — fetching, with the lock as the only authority on what is acceptable
# --------------------------------------------------------------------------


def tahaqquq_rabt(rabt: str) -> None:
    """Refuses a URL that is not HTTPS on a host the lock is allowed to name."""
    mufakkak = urllib.parse.urlsplit(rabt)
    if mufakkak.scheme != 'https':
        raise KhataIjlib(f'{rabt}: not https')
    if mufakkak.hostname not in MUDIFAT_MASMUHA:
        raise KhataIjlib(f'{rabt}: host {mufakkak.hostname!r} is not in the allow-list')


class MuwajjihMuqayyad(urllib.request.HTTPRedirectHandler):
    """Applies the host allow-list to redirect targets, not only to the first URL."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        tahaqquq_rabt(newurl)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


FATIH: Final = urllib.request.build_opener(MuwajjihMuqayyad)


def jalb(rabt: str, hajm: int) -> bytes:
    """Downloads exactly `hajm` bytes, refusing a response of any other length.

    The read is capped at one byte over the declared size so a redirected or
    hostile origin cannot stream an unbounded body into memory before the hash
    ever gets a chance to reject it.
    """
    tahaqquq_rabt(rabt)
    talab = urllib.request.Request(rabt, headers={'User-Agent': 'taarib-ijlib-khutut/1'})
    try:
        with FATIH.open(talab, timeout=MUHLAT_JALB) as jawab:
            bayt = jawab.read(hajm + 1)
    except urllib.error.URLError as sabab:
        raise KhataIjlib(f'{rabt}: {sabab}') from sabab
    if len(bayt) != hajm:
        raise KhataIjlib(f'{rabt}: {len(bayt)} bytes, lock declares {hajm}')
    return bayt


def basma(bayt: bytes) -> str:
    return hashlib.sha256(bayt).hexdigest()


def tahaqquq_bayt(bayt: bytes, madkhal: dict, masdar: str) -> None:
    """Both checks the lock offers, applied together and before anything is kept."""
    hajm = madkhal['hajm']
    if len(bayt) != hajm:
        raise KhataIjlib(f'{masdar}: {len(bayt)} bytes, lock declares {hajm}')
    matluba = madkhal['sha256']
    fiiliya = basma(bayt)
    if not hmac.compare_digest(fiiliya, matluba):
        raise KhataIjlib(
            f'{masdar}: sha256 mismatch\n'
            f'  lock  {matluba}\n'
            f'  bytes {fiiliya}'
        )


def iqra_qufl() -> dict[str, dict]:
    """The lock, indexed by `muarrif`, with every entry structurally checked."""
    try:
        khaam = QUFL.read_text(encoding='utf-8')
    except OSError as sabab:
        raise KhataIjlib(f'{QUFL}: {sabab}') from sabab
    try:
        wathiqa = json.loads(khaam)
    except json.JSONDecodeError as sabab:
        raise KhataIjlib(f'{QUFL}: {sabab}') from sabab

    madakhil = wathiqa.get('madakhil')
    if not isinstance(madakhil, list) or not madakhil:
        raise KhataIjlib(f'{QUFL}: madakhil is missing or empty')

    fahras: dict[str, dict] = {}
    for madkhal in madakhil:
        muarrif = madkhal.get('muarrif')
        if not isinstance(muarrif, str):
            raise KhataIjlib(f'{QUFL}: an entry has no muarrif')
        sijil = madkhal.get('sha256')
        if not isinstance(sijil, str) or not re.fullmatch(r'[0-9a-f]{64}', sijil):
            raise KhataIjlib(f'{QUFL}: {muarrif} has no usable sha256')
        if not isinstance(madkhal.get('hajm'), int) or madkhal['hajm'] <= 0:
            raise KhataIjlib(f'{QUFL}: {muarrif} has no usable hajm')
        if not isinstance(madkhal.get('rabt'), str):
            raise KhataIjlib(f'{QUFL}: {muarrif} has no rabt')
        if muarrif in fahras:
            raise KhataIjlib(f'{QUFL}: {muarrif} appears twice')
        fahras[muarrif] = madkhal
    return fahras


def ism_malaf(madkhal: dict) -> str:
    """The staged file name: the lock's `wajha` basename, flattened.

    The bundle tree keeps `sans/` and `latin/` apart because ten families share
    it; five faces in one directory do not need the split, and a flat name is
    what `khutut.css` can reference with a relative URL Vite will rewrite.
    """
    return str(madkhal['wajha']).rsplit('/', 1)[-1]


def hifz(masar: Path, bayt: bytes) -> None:
    """Writes through a temporary file in the same directory, so a killed run
    leaves either the previous bytes or the new ones and never half of either."""
    masar.parent.mkdir(parents=True, exist_ok=True)
    waqti = tempfile.NamedTemporaryFile(
        dir=masar.parent, prefix=f'.{masar.name}.', suffix='.juzi', delete=False
    )
    try:
        with waqti as maftuh:
            maftuh.write(bayt)
            maftuh.flush()
            os.fsync(maftuh.fileno())
        os.replace(waqti.name, masar)
    except BaseException:
        Path(waqti.name).unlink(missing_ok=True)
        raise


# --------------------------------------------------------------------------
# التغطية — the cmap, parsed here rather than trusted from the manifest
# --------------------------------------------------------------------------


def jadwal_sfnt(bayt: bytes) -> dict[str, tuple[int, int]]:
    """The sfnt table directory: tag -> (offset, length)."""
    if len(bayt) < 12:
        raise KhataIjlib('not a font: shorter than an sfnt header')
    naskha = struct.unpack_from('>I', bayt, 0)[0]
    if naskha not in (0x00010000, 0x4F54544F, 0x74727565):
        raise KhataIjlib(f'not a TrueType/OpenType font: sfntVersion 0x{naskha:08x}')
    adad = struct.unpack_from('>H', bayt, 4)[0]
    jadwal: dict[str, tuple[int, int]] = {}
    for i in range(adad):
        matla = 12 + i * 16
        if matla + 16 > len(bayt):
            raise KhataIjlib('truncated sfnt table directory')
        alama, _, izaha, tul = struct.unpack_from('>4sIII', bayt, matla)
        if izaha + tul > len(bayt):
            raise KhataIjlib(f'table {alama!r} runs past the end of the file')
        jadwal[alama.decode('latin-1')] = (izaha, tul)
    return jadwal


def _sigha_0(bayt: bytes, izaha: int) -> set[int]:
    return {i for i in range(256) if bayt[izaha + 6 + i] != 0}


def _sigha_4(bayt: bytes, izaha: int) -> set[int]:
    adad_x2 = struct.unpack_from('>H', bayt, izaha + 6)[0]
    adad = adad_x2 // 2
    nihayat = izaha + 14
    bidayat = nihayat + adad_x2 + 2
    furuq = bidayat + adad_x2
    izahat = furuq + adad_x2
    mughatta: set[int] = set()
    for i in range(adad):
        nihaya = struct.unpack_from('>H', bayt, nihayat + i * 2)[0]
        bidaya = struct.unpack_from('>H', bayt, bidayat + i * 2)[0]
        farq = struct.unpack_from('>h', bayt, furuq + i * 2)[0]
        izahat_i = izahat + i * 2
        izahat_qima = struct.unpack_from('>H', bayt, izahat_i)[0]
        if bidaya > nihaya:
            continue
        for harf in range(bidaya, nihaya + 1):
            if harf == 0xFFFF:
                continue
            if izahat_qima == 0:
                shakl = (harf + farq) & 0xFFFF
            else:
                unwan = izahat_i + izahat_qima + (harf - bidaya) * 2
                if unwan + 2 > len(bayt):
                    continue
                shakl = struct.unpack_from('>H', bayt, unwan)[0]
                if shakl != 0:
                    shakl = (shakl + farq) & 0xFFFF
            if shakl != 0:
                mughatta.add(harf)
    return mughatta


def _sigha_6(bayt: bytes, izaha: int) -> set[int]:
    awwal, adad = struct.unpack_from('>HH', bayt, izaha + 6)
    mughatta: set[int] = set()
    for i in range(adad):
        shakl = struct.unpack_from('>H', bayt, izaha + 10 + i * 2)[0]
        if shakl != 0:
            mughatta.add(awwal + i)
    return mughatta


def _sigha_12(bayt: bytes, izaha: int) -> set[int]:
    adad = struct.unpack_from('>I', bayt, izaha + 12)[0]
    mughatta: set[int] = set()
    for i in range(adad):
        bidaya, nihaya, _ = struct.unpack_from('>III', bayt, izaha + 16 + i * 12)
        if nihaya < bidaya or nihaya > 0x10FFFF:
            continue
        mughatta.update(range(bidaya, nihaya + 1))
    return mughatta


def huruf_mughattat(bayt: bytes) -> set[int]:
    """Every codepoint the font's Unicode cmaps map to a non-zero glyph.

    Every Unicode subtable is unioned rather than one being picked as "best":
    a face whose BMP table is format 4 and whose supplementary table is format
    12 covers the union of the two, and choosing one would under-report it.
    """
    izaha_cmap, _ = jadwal_sfnt(bayt).get('cmap', (0, 0))
    if izaha_cmap == 0:
        raise KhataIjlib('font has no cmap table')
    adad = struct.unpack_from('>H', bayt, izaha_cmap + 2)[0]
    mughatta: set[int] = set()
    for i in range(adad):
        minassa, tarmiz, izaha_juz = struct.unpack_from('>HHI', bayt, izaha_cmap + 4 + i * 8)
        # Unicode cmaps only: a Macintosh (1, 0) table is a legacy byte encoding
        # and reading it as Unicode invents coverage the font does not have.
        unicodi = minassa == 0 or (minassa == 3 and tarmiz in (1, 10))
        if not unicodi:
            continue
        izaha = izaha_cmap + izaha_juz
        sigha = struct.unpack_from('>H', bayt, izaha)[0]
        if sigha == 0:
            mughatta |= _sigha_0(bayt, izaha)
        elif sigha == 4:
            mughatta |= _sigha_4(bayt, izaha)
        elif sigha == 6:
            mughatta |= _sigha_6(bayt, izaha)
        elif sigha == 12:
            mughatta |= _sigha_12(bayt, izaha)
    if not mughatta:
        raise KhataIjlib('font has no readable Unicode cmap subtable')
    return mughatta


def huruf_matluba(bidaya: int, nihaya: int) -> list[int]:
    """The codepoints in a range a font is actually expected to draw."""
    return [
        harf
        for harf in range(bidaya, nihaya + 1)
        if unicodedata.category(chr(harf)) not in ASNAF_GHAYR_MARIYA
    ]


def taqrir_tughtiya(ism: str, bayt: bytes, sakit: bool) -> bool:
    """Prints one face's coverage against every range the interface needs."""
    mughatta = huruf_mughattat(bayt)
    salim = True
    if not sakit:
        print(f'\n  {ism}')
    for unwan, bidaya, nihaya in NITAQAT:
        matluba = huruf_matluba(bidaya, nihaya)
        naqisa = [harf for harf in matluba if harf not in mughatta]
        hala = 'full' if not naqisa else f'{len(matluba) - len(naqisa)}/{len(matluba)}'
        if naqisa:
            salim = False
        if sakit:
            continue
        print(f'    {unwan:<34} {hala}')
        if naqisa:
            asmaa = ', '.join(
                f'U+{harf:04X} {unicodedata.name(chr(harf), "?")}' for harf in naqisa[:6]
            )
            zayid = f' (+{len(naqisa) - 6} more)' if len(naqisa) > 6 else ''
            print(f'      missing: {asmaa}{zayid}')
    return salim


# --------------------------------------------------------------------------
# التشغيل
# --------------------------------------------------------------------------


def tasrihat_qaida() -> list[dict[str, str]]:
    """Every `@font-face` block in `khutut.css`, flattened to its declarations."""
    nass = re.sub(r'/\*.*?\*/', '', QAIDA.read_text(encoding='utf-8'), flags=re.DOTALL)
    kutal = re.findall(r'@font-face\s*\{([^}]*)\}', nass)
    tasrihat: list[dict[str, str]] = []
    for kutla in kutal:
        wajh: dict[str, str] = {}
        for satr in kutla.split(';'):
            miftah, faasil, qima = satr.partition(':')
            if faasil:
                wajh[miftah.strip().lower()] = qima.strip()
        tasrihat.append(wajh)
    return tasrihat


def tadqiq_qaida(asmaa: dict[str, str]) -> None:
    """Holds `khutut.css` to the table above: same families, same weights, same files.

    The bug this whole directory exists to fix was a family name in the token
    layer that no `@font-face` answered, and a typo here would reintroduce it in
    a form nothing else catches — the interface would simply render in a
    different font on every operating system and no build would fail.
    """
    if not QAIDA.is_file():
        raise KhataIjlib(f'{QAIDA}: missing; the staged files have nothing declaring them')

    tasrihat = tasrihat_qaida()
    if len(tasrihat) != len(WUJUH):
        raise KhataIjlib(
            f'{QAIDA.name}: {len(tasrihat)} @font-face blocks, {len(WUJUH)} faces staged'
        )

    shakawa: list[str] = []
    for wajh, tasrih in zip(WUJUH, tasrihat, strict=True):
        ism = asmaa[wajh.muarrif]
        aila = tasrih.get('font-family', '').strip('\'"')
        rabt = re.search(r"url\(\s*['\"]?\./([^'\")]+)['\"]?\s*\)", tasrih.get('src', ''))
        mutawaqqa = {
            'font-family': (aila, wajh.aila),
            'font-weight': (tasrih.get('font-weight', ''), str(wajh.wazn)),
            'font-style': (tasrih.get('font-style', ''), wajh.namat),
            # Not swap: a fallback face in an RTL interface reflows the whole
            # line, so a brief blank is the cheaper of the two.
            'font-display': (tasrih.get('font-display', ''), 'block'),
            'src': (rabt.group(1) if rabt else tasrih.get('src', ''), ism),
        }
        for khasiya, (fiili, matlub) in mutawaqqa.items():
            if fiili != matlub:
                shakawa.append(f'  {ism}: {khasiya} is {fiili!r}, must be {matlub!r}')

    if shakawa:
        raise KhataIjlib(f'{QAIDA.name} does not match the staged faces:\n' + '\n'.join(shakawa))


def nafidh(tahaqquq_faqat: bool, sakit: bool) -> int:
    fahras = iqra_qufl()
    matlub: list[tuple[str, str]] = [(wajh.muarrif, '') for wajh in WUJUH]
    matlub.append((RUKHSA_MUARRIF, RUKHSA_MALAF))

    # Phase one: every byte is fetched and verified before any of it is kept, so
    # a mismatch on the last face cannot leave the first four staged.
    maqbula: list[tuple[Path, bytes, bool]] = []
    asmaa: dict[str, str] = {}
    for muarrif, ism_mufrad in matlub:
        madkhal = fahras.get(muarrif)
        if madkhal is None:
            raise KhataIjlib(f'{QUFL}: {muarrif} is not in the lock')
        ism = ism_mufrad or ism_malaf(madkhal)
        asmaa[muarrif] = ism
        masar = WIJHA / ism

        if masar.is_file():
            mawjud = masar.read_bytes()
            try:
                tahaqquq_bayt(mawjud, madkhal, str(masar))
            except KhataIjlib:
                if tahaqquq_faqat:
                    raise
                if not sakit:
                    print(f'  refetch {ism}: on-disk bytes do not match the lock')
            else:
                maqbula.append((masar, mawjud, False))
                continue
        elif tahaqquq_faqat:
            raise KhataIjlib(f'{masar}: missing; run without --tahaqquq to stage it')

        bayt = jalb(madkhal['rabt'], madkhal['hajm'])
        tahaqquq_bayt(bayt, madkhal, madkhal['rabt'])
        maqbula.append((masar, bayt, True))

    # Phase two: nothing above raised, so the whole set is known good.
    jadid = 0
    for masar, bayt, yuktab in maqbula:
        if yuktab:
            hifz(masar, bayt)
            jadid += 1
        if not sakit:
            hal = 'staged ' if yuktab else 'present'
            print(f'  {hal} {masar.name:<38} {len(bayt):>7} bytes')

    if not sakit:
        print('\nCoverage')
    salim = True
    for masar, bayt, _ in maqbula:
        if masar.suffix != '.ttf':
            continue
        if not taqrir_tughtiya(masar.name, bayt, sakit):
            salim = False

    majmu = sum(len(bayt) for _, bayt, _ in maqbula)
    khutut_faqat = sum(len(b) for m, b, _ in maqbula if m.suffix == '.ttf')
    if not sakit:
        print(
            f'\n{len(maqbula)} files, {jadid} newly staged, {majmu} bytes total '
            f'({khutut_faqat} of them font data)'
        )
        if not salim:
            print('Coverage is not complete for every range; see the gaps above.')

    tadqiq_qaida(asmaa)
    return 0


def main() -> int:
    muhallil = argparse.ArgumentParser(
        prog='ijlib_khutut',
        description='Stage and verify the font faces the Studio webview embeds.',
    )
    muhallil.add_argument(
        '--tahaqquq',
        action='store_true',
        help='verify what is already on disk and touch no network; fail if anything is off',
    )
    muhallil.add_argument('--sakit', action='store_true', help='report only failures')
    hujaj = muhallil.parse_args()

    try:
        return nafidh(hujaj.tahaqquq, hujaj.sakit)
    except KhataIjlib as sabab:
        print(f'ijlib_khutut: {sabab}', file=sys.stderr)
        print('ijlib_khutut: nothing was written.', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
