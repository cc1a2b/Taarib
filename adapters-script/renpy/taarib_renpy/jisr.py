# -*- coding: utf-8 -*-
"""jisr — the ctypes binding to Taarib's native ABI, for Ren'Py.

This module is the only way the Ren'Py adapter reaches the engine. It binds
the C surface of ``taarib_jisr`` exactly as ``crates/taarib-jisr/src/anwa.rs``
froze it: every structure below matches that file field for field, in order,
at the same widths, and nothing here reinterprets, reorders, or "improves" a
layout. A binding that drifted from the frozen layout would not fail loudly —
it would read a glyph's advance out of its neighbour's cluster index and draw
text that is subtly, irreproducibly wrong on someone else's machine. That is
the failure this file exists to prevent, and it is why the module verifies
every structure's size at import time and refuses to continue on a mismatch.

Interpreter target
    Ren'Py ships its own CPython. Ren'Py 8 bundles CPython 3.9, so this file
    is written for **Python 3.9 and nothing newer**: no ``match``, no
    ``X | Y`` union syntax, no dataclasses, no slots tricks, no walrus-free
    zone violations that a newer parser would forgive. Ren'Py 7 runs Python
    2.7 and is *not* served by this file — on those builds Phase 10's version
    detection takes the engine-native HarfBuzz path instead, which is why the
    floor here is 3.9 rather than 2.7.

Loading
    The library is loaded from a path the caller supplies — the installed
    patch's own directory — never from the system search path, because a
    game machine's PATH is a lottery and the one guarantee Taarib has is the
    files it installed itself. :func:`hammil` resolves the platform's naming
    (``taarib_jisr.dll`` / ``libtaarib_jisr.so`` / ``libtaarib_jisr.dylib``)
    and the *process* architecture: some older Ren'Py builds run a 32-bit
    interpreter on a 64-bit OS, and loading a 64-bit library into them fails.
    The pointer width of the running interpreter, not the OS, decides.

Packing
    No structure here sets ``_pack_``. The Rust side is ``repr(C)`` with
    natural alignment; ctypes' default is the platform C ABI's natural
    alignment, which is the same thing. Forcing a pack value would change
    ``TaaribTalab``'s size on 64-bit (where its trailing options struct is
    8-aligned after three floats) and the two sides would silently disagree.

Buffers and the frame budget
    Ren'Py asks for layout per displayable per frame. :meth:`Siyaq.khattit`
    therefore reuses one caller-owned glyph buffer, grows it only when the
    library reports ``SIAT_QASIRA``, and hands the result back as a
    memoryview over that same buffer — never as a list. A list of tuples
    would allocate thousands of Python objects per frame, and the garbage
    collector's pause for reclaiming them is a frame hitch the player sees.
    The price of zero-copy is a lifetime rule, stated on :class:`Takhtit`:
    a result is valid only until the next layout call on the same context.

Errors
    Any entry point that returns non-zero raises :class:`KhataTaarib`,
    built from the library's thread-local error stash: the permanent code
    (``TAARIB-E-2500``), the Arabic sentence, the English sentence, and the
    next-action number. ``str()`` of the exception is the Arabic sentence,
    because Arabic is the primary text of this product.
"""

import ctypes
import os
import platform
import sys

__all__ = [
    "ISDAR_KABIR",
    "NAJAH", "KHATA_AAM", "MUASHIR_BATIL", "MAQBAD_BATIL", "SIAT_QASIRA",
    "ISDAR_GHAYR_MUTAWAFIQ", "TARMIZ_BATIL", "KHATT_MARFUD",
    "TASHKEEL_FASHIL", "DHAKIRA", "QEEMA_BATILA", "INHIYAR",
    "LAWHA_MUMTALIA", "GHAYR_MADUM", "GHAYR_MUHAYYAA",
    "HARF_ALAMA", "SATR_AKHIR", "SATR_YAMEEN",
    "KHIYAR_HIWAR", "KHIYAR_SATR_WAHID",
    "TAKHTIT_YAMEEN", "TAKHTIT_MAQSUS", "TAKHTIT_TAJAWUZ", "TAKHTIT_MAKHZAN",
    "USLUB_MAAIL", "USLUB_DHARRA", "USLUB_KHATT", "USLUB_WAZN",
    "USLUB_HAJM", "USLUB_LAWN",
    "ITTIJAH_TILQAI", "ITTIJAH_YAMEEN", "ITTIJAH_YASAR",
    "LUGHA_TILQAI", "LUGHA_ARABI", "LUGHA_FARISI", "LUGHA_URDU",
    "LUGHA_LATINI",
    "DABT_BILA", "DABT_MASAFAT", "DABT_KASHIDA", "DABT_KASHIDA_MASAFAT",
    "MUHADHAHA_BIDAYA", "MUHADHAHA_NIHAYA", "MUHADHAHA_WASAT",
    "MUHADHAHA_DABT",
    "TASHKEEL_IBQA", "TASHKEEL_HADHF", "TASHKEEL_HIWAR",
    "ARQAM_KAMA_HIYA", "ARQAM_LATINI", "ARQAM_ARABI", "ARQAM_FARISI",
    "TAJAWUZ_BALLAGH", "TAJAWUZ_TAQLIS", "TAJAWUZ_IKHTISAR",
    "NAMAT_TAGHTIYA", "NAMAT_MISAFA",
    "TaaribHarf", "TaaribSatr", "TaaribTaqreerTajawuz", "TaaribMakhzanTakhtit",
    "TaaribQiyasNass", "TaaribNitaqUslub", "TaaribSifa", "TaaribKhiyarat",
    "TaaribTalab", "TaaribKhiyaratSiyaq", "TaaribQiyasatKhatt",
    "TaaribMiftahShakl", "TaaribMawdiShakl", "TaaribSafha",
    "TaaribIhsaatLawha", "TaaribIhsaatKhazina", "TaaribRaddIltiqat",
    "KhataTaarib", "Jisr", "hammil",
    "Siyaq", "Khatt", "Silsila", "Lawha", "Takhtit", "Khiyarat", "Nitaq",
]

# ---------------------------------------------------------------------------
# The ABI version this binding was written against
# ---------------------------------------------------------------------------

#: The major version of the ABI this file matches. A library reporting any
#: other major is refused at load: the frozen layouts below are the layouts
#: of major 1, and calling into a different major with them is memory
#: corruption, not a compatibility mode.
ISDAR_KABIR = 1

# ---------------------------------------------------------------------------
# Status codes — crates/taarib-jisr/src/khata_c.rs, verbatim
# ---------------------------------------------------------------------------

#: The operation succeeded.
NAJAH = 0
#: Something failed that no more specific code describes.
KHATA_AAM = -1
#: A required pointer argument was null.
MUASHIR_BATIL = -2
#: A handle was not one the library issued, or was destroyed already.
MAQBAD_BATIL = -3
#: The caller's buffer is too small; the required capacity was written out.
SIAT_QASIRA = -4
#: The caller was built against a different major version of the ABI.
ISDAR_GHAYR_MUTAWAFIQ = -5
#: A string argument was not valid UTF-8.
TARMIZ_BATIL = -6
#: A font was rejected, or could not be read.
KHATT_MARFUD = -7
#: Shaping produced nothing for text that is not empty.
TASHKEEL_FASHIL = -8
#: An allocation failed inside the library.
DHAKIRA = -9
#: An argument was structurally fine but its value is not usable.
QEEMA_BATILA = -10
#: A panic was caught at the boundary and converted into a status.
INHIYAR = -11
#: The atlas is full and nothing in it may be evicted this frame.
LAWHA_MUMTALIA = -12
#: The operation is not available in this build.
GHAYR_MADUM = -13
#: The library has not been initialised, or was shut down.
GHAYR_MUHAYYAA = -14

# ---------------------------------------------------------------------------
# Flags — crates/taarib-jisr/src/anwa.rs, verbatim
# ---------------------------------------------------------------------------

#: TaaribHarf.alam: this glyph is a combining mark; it does not advance the pen.
HARF_ALAMA = 1 << 0

#: TaaribSatr.alam: the last line of its paragraph, exempt from justification.
SATR_AKHIR = 1 << 0
#: TaaribSatr.alam: this line's base direction is right to left.
SATR_YAMEEN = 1 << 1

#: TaaribKhiyarat.alam: this text is dialogue, for the diacritics policy.
KHIYAR_HIWAR = 1 << 0
#: TaaribKhiyarat.alam: wrapping is forbidden, as in a single-line field.
KHIYAR_SATR_WAHID = 1 << 1

#: TaaribMakhzanTakhtit.alam: the layout's overall direction is right to left.
TAKHTIT_YAMEEN = 1 << 0
#: TaaribMakhzanTakhtit.alam: the overflow policy truncated the text.
TAKHTIT_MAQSUS = 1 << 1
#: TaaribMakhzanTakhtit.alam: the text did not fit; the report says by how much.
TAKHTIT_TAJAWUZ = 1 << 2
#: TaaribMakhzanTakhtit.alam: this layout was served from the cache.
TAKHTIT_MAKHZAN = 1 << 3

#: TaaribNitaqUslub.alam: italic or slanted.
USLUB_MAAIL = 1 << 0
#: TaaribNitaqUslub.alam: an opaque atom — a placeholder or inline sprite.
USLUB_DHARRA = 1 << 1
#: TaaribNitaqUslub.alam: the span sets a font index.
USLUB_KHATT = 1 << 2
#: TaaribNitaqUslub.alam: the span sets a variable-font weight.
USLUB_WAZN = 1 << 3
#: TaaribNitaqUslub.alam: the span sets a size.
USLUB_HAJM = 1 << 4
#: TaaribNitaqUslub.alam: the span sets a colour.
USLUB_LAWN = 1 << 5

# ---------------------------------------------------------------------------
# Option discriminants — the u32 values TaaribKhiyarat carries
# ---------------------------------------------------------------------------

#: Base direction from the first strong character.
ITTIJAH_TILQAI = 0
#: Base direction forced right to left.
ITTIJAH_YAMEEN = 1
#: Base direction forced left to right.
ITTIJAH_YASAR = 2

#: Detect the language from the text.
LUGHA_TILQAI = 0
LUGHA_ARABI = 1
LUGHA_FARISI = 2
LUGHA_URDU = 3
LUGHA_LATINI = 4

#: No justification.
DABT_BILA = 0
#: Stretch spaces only.
DABT_MASAFAT = 1
#: Elongate letters only.
DABT_KASHIDA = 2
#: Elongate letters, then stretch spaces — the classical Arabic default.
DABT_KASHIDA_MASAFAT = 3

MUHADHAHA_BIDAYA = 0
MUHADHAHA_NIHAYA = 1
MUHADHAHA_WASAT = 2
MUHADHAHA_DABT = 3

#: Keep diacritics.
TASHKEEL_IBQA = 0
#: Strip diacritics.
TASHKEEL_HADHF = 1
#: Keep diacritics in dialogue only.
TASHKEEL_HIWAR = 2

#: Leave digits as written.
ARQAM_KAMA_HIYA = 0
#: Map digits to European.
ARQAM_LATINI = 1
#: Map digits to Arabic-Indic.
ARQAM_ARABI = 2
#: Map digits to Eastern Arabic-Indic, for Persian and Urdu.
ARQAM_FARISI = 3

#: Report overflow honestly and lay out anyway.
TAJAWUZ_BALLAGH = 0
#: Shrink to fit, no smaller than the declared floor.
TAJAWUZ_TAQLIS = 1
#: Truncate with the correct Arabic ellipsis behaviour.
TAJAWUZ_IKHTISAR = 2

#: Atlas pages hold 8-bit antialiased coverage.
NAMAT_TAGHTIYA = 0
#: Atlas pages hold a signed distance field.
NAMAT_MISAFA = 1

#: The next-action numbers ``taarib_khata_khutwa`` returns, as the ABI
#: freezes them. A number this table does not know means a newer library;
#: show the sentence without a button rather than showing nothing.
KHUTWA_ASMA = {
    0: "la_shay",
    1: "aada_muhawala",
    2: "aada_fahs_maktaba",
    3: "aada_fahs_muharrik",
    4: "ikhtiyar_masar",
    5: "ikhtiyar_khatt_aakhar",
    6: "fath_idadat",
    7: "fath_tashkhis",
    8: "fath_taqreer_tajawuz",
    9: "fath_nusus",
    10: "tahdith_taarib",
    11: "iadat_tarkib_ittar",
    12: "ilgha_tathbeet",
    13: "iadat_mutabaqa_bina",
    14: "tahaqquq_salamat_luba",
    15: "iblagh_lil_musahim",
    16: "iblagh_lil_malik",
    17: "tahrir_masaha",
    18: "manh_salahiya",
}

# ---------------------------------------------------------------------------
# Structures — anwa.rs field for field, in order, at the same widths.
# No structure sets _pack_: the Rust side is repr(C) with natural alignment,
# and ctypes' default is the same C ABI. Forcing a pack here would silently
# change offsets on one side only.
# ---------------------------------------------------------------------------


class TaaribTaqreerTajawuz(ctypes.Structure):
    """What overflowed, by how much. Twenty-four bytes.

    Not an error: a measurement the caller asked for, valid only when the
    layout's ``alam`` carries ``TAKHTIT_TAJAWUZ``.
    """

    _fields_ = [
        ("ard", ctypes.c_float),
        ("ard_mutah", ctypes.c_float),
        ("irtifa", ctypes.c_float),
        ("irtifa_mutah", ctypes.c_float),
        ("awwal_satr", ctypes.c_uint32),
        ("adad_sutur", ctypes.c_uint32),
    ]


class TaaribHarf(ctypes.Structure):
    """One positioned glyph, ready to become two triangles. Twenty-four bytes.

    Note what is absent: a codepoint. Nothing downstream of shaping is given
    the character a glyph came from, because a field that carried it would
    eventually be drawn from — the presentation-form pipeline this product
    exists to make impossible.
    """

    _fields_ = [
        ("muarrif", ctypes.c_uint32),
        ("anqud", ctypes.c_uint32),
        ("s", ctypes.c_float),
        ("a", ctypes.c_float),
        ("taqaddum", ctypes.c_float),
        ("nitaq", ctypes.c_uint16),
        ("khatt", ctypes.c_uint8),
        ("alam", ctypes.c_uint8),
    ]


class TaaribSatr(ctypes.Structure):
    """One laid-out line. Forty-eight bytes."""

    _fields_ = [
        ("awwal_harf", ctypes.c_uint32),
        ("adad_huruf", ctypes.c_uint32),
        ("bidayat_mantiqi", ctypes.c_uint32),
        ("nihayat_mantiqi", ctypes.c_uint32),
        ("asas", ctypes.c_float),
        ("bidaya", ctypes.c_float),
        ("ard", ctypes.c_float),
        ("irtifa", ctypes.c_float),
        ("suud", ctypes.c_float),
        ("hubut", ctypes.c_float),
        ("dabt", ctypes.c_float),
        ("alam", ctypes.c_uint32),
    ]


class TaaribMakhzanTakhtit(ctypes.Structure):
    """The caller-owned buffer a layout is written into.

    The caller owns both arrays; the library never allocates them, never
    frees them, and never keeps a pointer to them past the call. When either
    array is too small nothing is written, the ``adad_*`` fields receive the
    required counts, and the call returns ``SIAT_QASIRA`` — the negotiation
    the allocation-free hot path is built on.
    """

    _fields_ = [
        ("huruf", ctypes.POINTER(TaaribHarf)),
        ("siaat_huruf", ctypes.c_size_t),
        ("adad_huruf", ctypes.c_size_t),
        ("sutur", ctypes.POINTER(TaaribSatr)),
        ("siaat_sutur", ctypes.c_size_t),
        ("adad_sutur", ctypes.c_size_t),
        ("ard", ctypes.c_float),
        ("irtifa", ctypes.c_float),
        ("hajm", ctypes.c_float),
        ("alam", ctypes.c_uint32),
        ("tajawuz", TaaribTaqreerTajawuz),
    ]


class TaaribQiyasNass(ctypes.Structure):
    """Text measured without positioning a single glyph."""

    _fields_ = [
        ("ard", ctypes.c_float),
        ("irtifa", ctypes.c_float),
        ("suud", ctypes.c_float),
        ("hubut", ctypes.c_float),
        ("adad_sutur", ctypes.c_uint32),
        ("hashw", ctypes.c_uint32),
    ]


class TaaribNitaqUslub(ctypes.Structure):
    """A style span over the caller's text. Fifty-two bytes.

    Only what changes layout is read. Colour is carried through untouched so
    it survives visual reordering — a coloured word inside a sentence must
    keep its colour after the line is put into visual order.
    """

    _fields_ = [
        ("bidaya", ctypes.c_uint32),
        ("tul", ctypes.c_uint32),
        ("lawn", ctypes.c_uint32),
        ("alam", ctypes.c_uint32),
        ("hajm", ctypes.c_float),
        ("tabaud", ctypes.c_float),
        ("izaha", ctypes.c_float),
        ("ard_dharra", ctypes.c_float),
        ("irtifa_dharra", ctypes.c_float),
        ("asas_dharra", ctypes.c_float),
        ("marja_dharra", ctypes.c_uint32),
        ("wazn", ctypes.c_uint16),
        ("id", ctypes.c_uint16),
        ("khatt", ctypes.c_uint8),
        ("hashw", ctypes.c_uint8 * 3),
    ]


class TaaribSifa(ctypes.Structure):
    """An OpenType feature setting beyond the defaults. Eight bytes."""

    _fields_ = [
        ("wasm", ctypes.c_uint8 * 4),
        ("qeema", ctypes.c_uint32),
    ]


class TaaribKhiyarat(ctypes.Structure):
    """The decisions a caller makes once and records into a patch."""

    _fields_ = [
        ("sifat", ctypes.POINTER(TaaribSifa)),
        ("adad_sifat", ctypes.c_size_t),
        ("ittijah", ctypes.c_uint32),
        ("lugha", ctypes.c_uint32),
        ("dabt", ctypes.c_uint32),
        ("muhadhaha", ctypes.c_uint32),
        ("tashkeel", ctypes.c_uint32),
        ("arqam", ctypes.c_uint32),
        ("tajawuz", ctypes.c_uint32),
        ("hajm_adna", ctypes.c_float),
        ("irtifa_satr", ctypes.c_float),
        ("tabaud_ahruf", ctypes.c_float),
        ("tabaud_kalimat", ctypes.c_float),
        ("alam", ctypes.c_uint32),
    ]


class TaaribTalab(ctypes.Structure):
    """One layout request.

    On 64-bit this structure carries four bytes of alignment padding before
    ``khiyarat`` (three floats end at offset 52; the options struct starts
    with a pointer and is 8-aligned). ctypes inserts the same padding repr(C)
    does, which is exactly why ``_pack_`` must never be set here.
    """

    _fields_ = [
        ("nass", ctypes.POINTER(ctypes.c_uint8)),
        ("tul_nass", ctypes.c_size_t),
        ("nitaqat", ctypes.POINTER(TaaribNitaqUslub)),
        ("adad_nitaqat", ctypes.c_size_t),
        ("silsila", ctypes.c_void_p),
        ("hajm", ctypes.c_float),
        ("ard_mutah", ctypes.c_float),
        ("irtifa_mutah", ctypes.c_float),
        ("khiyarat", TaaribKhiyarat),
    ]


class TaaribKhiyaratSiyaq(ctypes.Structure):
    """How a context is built."""

    _fields_ = [
        ("mizaniyat_makhzan", ctypes.c_size_t),
        ("adad_makhazin", ctypes.c_uint32),
        ("hashw", ctypes.c_uint32),
    ]


class TaaribQiyasatKhatt(ctypes.Structure):
    """A font's metrics, scaled to a pixel size. Thirty-two bytes."""

    _fields_ = [
        ("suud", ctypes.c_float),
        ("hubut", ctypes.c_float),
        ("fajwa", ctypes.c_float),
        ("irtifa_satr", ctypes.c_float),
        ("uluw_kabital", ctypes.c_float),
        ("uluw_saghir", ctypes.c_float),
        ("wahdat", ctypes.c_uint32),
        ("hashw", ctypes.c_uint32),
    ]


class TaaribMiftahShakl(ctypes.Structure):
    """Everything that makes one rasterized image of a glyph distinct."""

    _fields_ = [
        ("muarrif", ctypes.c_uint32),
        ("hajm_rubi", ctypes.c_uint16),
        ("khatt", ctypes.c_uint8),
        ("bakat", ctypes.c_uint8),
    ]


class TaaribMawdiShakl(ctypes.Structure):
    """Where a glyph lives in the atlas, and how to draw it. Twenty bytes."""

    _fields_ = [
        ("taqaddum", ctypes.c_float),
        ("s", ctypes.c_uint16),
        ("a", ctypes.c_uint16),
        ("ard", ctypes.c_uint16),
        ("irtifa", ctypes.c_uint16),
        ("izaha_s", ctypes.c_int16),
        ("izaha_a", ctypes.c_int16),
        ("safha", ctypes.c_uint16),
        ("hashw", ctypes.c_uint16),
    ]


class TaaribSafha(ctypes.Structure):
    """One texture page, borrowed.

    ``bayt`` points into the atlas and stays valid only until the next call
    that can change the atlas — any glyph lookup, or destroying it. Upload
    it, then let go.
    """

    _fields_ = [
        ("bayt", ctypes.POINTER(ctypes.c_uint8)),
        ("tul", ctypes.c_size_t),
        ("ard", ctypes.c_uint16),
        ("irtifa", ctypes.c_uint16),
        ("namat", ctypes.c_uint32),
    ]


class TaaribIhsaatLawha(ctypes.Structure):
    """What the atlas has been doing. Fifty-six bytes."""

    _fields_ = [
        ("isabat", ctypes.c_uint64),
        ("ikhfaqat", ctypes.c_uint64),
        ("ikhlaat", ctypes.c_uint64),
        ("ahdath_namu", ctypes.c_uint64),
        ("bayt", ctypes.c_uint64),
        ("mizaniya", ctypes.c_uint64),
        ("ashkal", ctypes.c_uint32),
        ("safahat", ctypes.c_uint32),
    ]


class TaaribIhsaatKhazina(ctypes.Structure):
    """What the layout cache has been doing. Forty-eight bytes.

    Appended to the ABI together with ``taarib_makhzan_ihsaat``, so on the
    Rust side it lives in ``awamir.rs`` rather than the frozen ``anwa.rs``
    — under the same layout law: widest fields first, explicit padding.
    The tail is two ``u32`` fields, not a sixth ``u64``: the wrong shape
    is also 48 bytes and, with ``hashw`` zeroed, even reads the same
    numbers on little-endian — it would work until the padding is given a
    meaning, then fail silently on the field quietly absorbing it.
    """

    _fields_ = [
        ("isabat", ctypes.c_uint64),
        ("ikhfaqat", ctypes.c_uint64),
        ("ikhlaat", ctypes.c_uint64),
        ("bayt", ctypes.c_uint64),
        ("mizaniya", ctypes.c_uint64),
        ("madkhalat", ctypes.c_uint32),
        ("hashw", ctypes.c_uint32),
    ]


def _tahaqqaq_ahjam():
    """Verifies every structure's size against the frozen layout, at import.

    A size mismatch means this interpreter's ctypes disagrees with repr(C)
    about a layout — and every call after that would be reading fields out
    of their neighbours. Failing the import is the loud version of a bug
    that is otherwise silent, data-dependent, and reported as 'Arabic looks
    wrong on my machine'.
    """
    muashir = ctypes.sizeof(ctypes.c_void_p)
    thabit = {
        TaaribHarf: 24,
        TaaribSatr: 48,
        TaaribTaqreerTajawuz: 24,
        TaaribQiyasNass: 24,
        TaaribNitaqUslub: 52,
        TaaribSifa: 8,
        TaaribQiyasatKhatt: 32,
        TaaribMiftahShakl: 8,
        TaaribMawdiShakl: 20,
        TaaribIhsaatLawha: 56,
        TaaribIhsaatKhazina: 48,
    }
    if muashir == 8:
        mutaghayyir = {
            TaaribKhiyaratSiyaq: 16,
            TaaribKhiyarat: 64,
            TaaribTalab: 120,
            TaaribMakhzanTakhtit: 88,
            TaaribSafha: 24,
        }
    else:
        mutaghayyir = {
            TaaribKhiyaratSiyaq: 12,
            TaaribKhiyarat: 56,
            TaaribTalab: 88,
            TaaribMakhzanTakhtit: 64,
            TaaribSafha: 16,
        }
    thabit.update(mutaghayyir)
    for naw, mutawaqqa in thabit.items():
        fielisi = ctypes.sizeof(naw)
        if fielisi != mutawaqqa:
            raise ImportError(
                "taarib_renpy.jisr: %s is %d bytes here but the frozen ABI "
                "layout is %d (pointer width %d). This interpreter's C ABI "
                "does not match taarib_jisr; refusing to continue rather "
                "than corrupt memory." % (naw.__name__, fielisi, mutawaqqa, muashir)
            )


_tahaqqaq_ahjam()

# ---------------------------------------------------------------------------
# The capture callback type — hayat.rs's TaaribRaddIltiqat
# ---------------------------------------------------------------------------

#: The C signature of the runtime capture sink, mirroring
#: ``TaaribRaddIltiqat`` in ``crates/taarib-jisr/src/hayat.rs``:
#: ``void (*)(void *mustakhdim, const uint8_t *nass, size_t tul)``.
#: ``nass`` is UTF-8, ``tul`` bytes long, not NUL-terminated, and borrowed
#: only for the duration of the invocation. CFUNCTYPE and not WINFUNCTYPE
#: because the export is ``extern "C"`` — cdecl — everywhere, including
#: the 32-bit Windows builds where the two conventions actually differ.
TaaribRaddIltiqat = ctypes.CFUNCTYPE(
    None, ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t)

# ---------------------------------------------------------------------------
# The error that crosses back into Python
# ---------------------------------------------------------------------------

#: Fallback sentences for failures that carry no stashed Khata — a null
#: pointer, a stale handle, a caught panic are recorded as a bare status
#: code on the Rust side, and the retrieval calls honestly return nothing.
#: The caller still deserves a sentence in both languages, so the binding
#: supplies one per code rather than raising an exception whose str() is
#: empty.
_JUMAL_IHTIYAT = {
    KHATA_AAM: (u"فشلت العملية داخل محرك تعريب.",
                "The operation failed inside the Taarib engine."),
    MUASHIR_BATIL: (u"مُرِّر مؤشر فارغ إلى جسر تعريب.",
                    "A null pointer was passed to the Taarib bridge."),
    MAQBAD_BATIL: (u"استُخدم مقبض بعد تدميره أو لم يصدر عن هذه المكتبة.",
                   "A handle was used after destruction, or was never "
                   "issued by this library."),
    SIAT_QASIRA: (u"السعة الممنوحة أصغر من المطلوب.",
                  "The supplied capacity is smaller than required."),
    ISDAR_GHAYR_MUTAWAFIQ: (u"إصدار مكتبة تعريب لا يطابق هذا الرابط.",
                            "The Taarib library's version does not match "
                            "this binding."),
    TARMIZ_BATIL: (u"النص المُمرَّر ليس UTF-8 صالحًا.",
                   "The supplied text is not valid UTF-8."),
    KHATT_MARFUD: (u"رُفض الخط لأنه لا يحمل جداول العربية المطلوبة.",
                   "The font was rejected: it lacks the required Arabic "
                   "tables."),
    TASHKEEL_FASHIL: (u"لم يُنتج التشكيل شيئًا لنص غير فارغ.",
                      "Shaping produced nothing for non-empty text."),
    DHAKIRA: (u"فشل حجز الذاكرة داخل المكتبة.",
              "An allocation failed inside the library."),
    QEEMA_BATILA: (u"قيمة وسيطة غير صالحة للاستخدام.",
                   "An argument's value is not usable."),
    INHIYAR: (u"التُقط انهيار داخلي عند الحدود؛ هذا خلل يستحق البلاغ.",
              "An internal panic was caught at the boundary; this is a "
              "bug worth reporting."),
    LAWHA_MUMTALIA: (u"اللوحة ممتلئة ولا شيء فيها قابل للإخلاء الآن.",
                     "The atlas is full and nothing in it may be evicted "
                     "right now."),
    GHAYR_MADUM: (u"العملية غير متاحة في هذه النسخة.",
                  "The operation is not available in this build."),
    GHAYR_MUHAYYAA: (u"المكتبة لم تُهيَّأ أو أُغلقت.",
                     "The library is not initialised, or was shut down."),
}


class KhataTaarib(Exception):
    """A failure that crossed the ABI, carrying everything the caller needs.

    Four parts, mirroring ``taarib-usus``'s error model exactly: the
    permanent machine code (``ramz``, e.g. ``TAARIB-E-2500``), the Arabic
    sentence (``arabi``), the English sentence (``injilizi``), and the
    next-action number (``khutwa``). ``halat`` is the raw ``int32`` status.

    ``str()`` is the Arabic sentence, because Arabic is the primary text of
    this product and the sentence was written for the person reading it —
    the English sentence and the code are attributes for logs and reports.

    Built from the library's thread-local stash immediately after the
    failing call, before any other ABI call, because any other successful
    entry point clears the stash for its thread.
    """

    def __init__(self, halat, ramz, arabi, injilizi, khutwa):
        badeel = _JUMAL_IHTIYAT.get(halat, _JUMAL_IHTIYAT[KHATA_AAM])
        #: The int32 status the failing call returned.
        self.halat = halat
        #: The permanent code, e.g. "TAARIB-E-2500". Empty when the failure
        #: carried no structured error (null pointer, stale handle, panic).
        self.ramz = ramz
        #: The Arabic sentence a player or translator reads.
        self.arabi = arabi if arabi else badeel[0]
        #: The same sentence in English, for logs.
        self.injilizi = injilizi if injilizi else badeel[1]
        #: The next-action number; see KHUTWA_ASMA for the frozen meanings.
        self.khutwa = khutwa
        Exception.__init__(self, self.arabi)

    def __str__(self):
        return self.arabi

    def ism_khutwa(self):
        """The next action's stable name, or None for a number this binding
        does not know — which means a newer library, not an error."""
        return KHUTWA_ASMA.get(self.khutwa)


# ---------------------------------------------------------------------------
# The loaded library
# ---------------------------------------------------------------------------

def _ism_maktaba():
    """The platform's file name for the library. One name per platform —
    the architecture is a directory, not a suffix, so a patch can carry
    every build side by side without renaming any of them."""
    if sys.platform.startswith("win") or sys.platform == "cygwin":
        return "taarib_jisr.dll"
    if sys.platform == "darwin":
        return "libtaarib_jisr.dylib"
    return "libtaarib_jisr.so"


def _ism_bina():
    """The running *process* architecture, which is what decides the build.

    The pointer width of this interpreter, not the operating system: some
    older Ren'Py builds run a 32-bit Python on a 64-bit Windows, and asking
    the OS would pick a 64-bit library the process cannot load.
    """
    if ctypes.sizeof(ctypes.c_void_p) == 4:
        return "i686"
    aala = platform.machine().lower()
    if aala in ("arm64", "aarch64"):
        return "aarch64"
    return "x86_64"


class Jisr(object):
    """One loaded ``taarib_jisr`` library: the raw calls, typed.

    Everything here is a thin, complete, one-to-one binding of the exported
    C surface, with ``argtypes`` and ``restype`` declared for every function
    — ctypes without declared types guesses, and a guess that happens to
    work on 64-bit truncates pointers on the 32-bit builds this module
    explicitly supports. The idiomatic layer (:class:`Siyaq` and friends)
    sits on top; adapters normally never touch this class directly.

    Use :func:`hammil` to construct one.
    """

    def __init__(self, maktaba, masar):
        #: The ctypes.CDLL underneath.
        self.maktaba = maktaba
        #: The path the library was actually loaded from, for diagnostics.
        self.masar = masar
        self._ayyin_anwa()
        self.kabir, self.sagheer = self.abi_isdar()
        if self.kabir != ISDAR_KABIR:
            # No stash exists for a refusal the caller makes, so the
            # sentence is synthesized here — the one place this binding
            # fabricates an error rather than retrieving one.
            raise KhataTaarib(
                ISDAR_GHAYR_MUTAWAFIQ,
                "",
                u"مكتبة تعريب المحمّلة إصدارها الأكبر %d وهذا الرابط يتطلب %d."
                % (self.kabir, ISDAR_KABIR),
                "The loaded Taarib library reports major version %d but "
                "this binding requires %d." % (self.kabir, ISDAR_KABIR),
                10,  # tahdith_taarib: update Taarib itself.
            )

    # -- typing ------------------------------------------------------------

    def _ayyin_anwa(self):
        """Declares the signatures of all twenty-eight exported functions,
        in the frozen surface's order. The count is stated as a number so
        it can be checked against ``awamir.rs`` instead of trusted: this
        docstring once said "every" while three exports were missing, and
        a claim of completeness is exactly the kind that survives being
        false. A function missing from the library raises here, at load,
        rather than as an AttributeError mid-frame."""
        m = self.maktaba
        muashir = ctypes.c_void_p
        hajm = ctypes.c_size_t

        m.taarib_abi_isdar.argtypes = [
            ctypes.POINTER(ctypes.c_uint32), ctypes.POINTER(ctypes.c_uint32)]
        m.taarib_abi_isdar.restype = ctypes.c_int32

        m.taarib_isdar_nass.argtypes = [
            ctypes.c_char_p, hajm, ctypes.POINTER(hajm)]
        m.taarib_isdar_nass.restype = ctypes.c_int32

        m.taarib_khata_akhir.argtypes = []
        m.taarib_khata_akhir.restype = ctypes.c_int32

        m.taarib_khata_khutwa.argtypes = []
        m.taarib_khata_khutwa.restype = ctypes.c_int32

        m.taarib_khata_ramz.argtypes = [
            ctypes.c_char_p, hajm, ctypes.POINTER(hajm)]
        m.taarib_khata_ramz.restype = ctypes.c_int32

        m.taarib_khata_nass.argtypes = [
            ctypes.c_uint32, ctypes.c_char_p, hajm, ctypes.POINTER(hajm)]
        m.taarib_khata_nass.restype = ctypes.c_int32

        m.taarib_siyaq_insha.argtypes = [
            ctypes.POINTER(TaaribKhiyaratSiyaq), ctypes.POINTER(muashir)]
        m.taarib_siyaq_insha.restype = ctypes.c_int32

        m.taarib_siyaq_ihdham.argtypes = [muashir]
        m.taarib_siyaq_ihdham.restype = ctypes.c_int32

        m.taarib_siyaq_amsah.argtypes = [muashir]
        m.taarib_siyaq_amsah.restype = ctypes.c_int32

        m.taarib_khatt_min_dhakira.argtypes = [
            muashir, ctypes.POINTER(ctypes.c_uint8), hajm,
            ctypes.c_uint32, ctypes.c_uint32, ctypes.POINTER(muashir)]
        m.taarib_khatt_min_dhakira.restype = ctypes.c_int32

        m.taarib_khatt_ihdham.argtypes = [muashir, muashir]
        m.taarib_khatt_ihdham.restype = ctypes.c_int32

        m.taarib_khatt_huwiya.argtypes = [
            muashir, muashir, ctypes.POINTER(ctypes.c_uint64)]
        m.taarib_khatt_huwiya.restype = ctypes.c_int32

        m.taarib_khatt_qiyasat.argtypes = [
            muashir, muashir, ctypes.c_float,
            ctypes.POINTER(TaaribQiyasatKhatt)]
        m.taarib_khatt_qiyasat.restype = ctypes.c_int32

        m.taarib_khatt_aila.argtypes = [
            muashir, muashir, ctypes.c_char_p, hajm, ctypes.POINTER(hajm)]
        m.taarib_khatt_aila.restype = ctypes.c_int32

        m.taarib_silsila_insha.argtypes = [
            muashir, ctypes.POINTER(muashir), hajm, ctypes.POINTER(muashir)]
        m.taarib_silsila_insha.restype = ctypes.c_int32

        m.taarib_silsila_ihdham.argtypes = [muashir, muashir]
        m.taarib_silsila_ihdham.restype = ctypes.c_int32

        m.taarib_takhtit.argtypes = [
            muashir, ctypes.POINTER(TaaribTalab),
            ctypes.POINTER(TaaribMakhzanTakhtit)]
        m.taarib_takhtit.restype = ctypes.c_int32

        m.taarib_qiyas.argtypes = [
            muashir, ctypes.POINTER(TaaribTalab),
            ctypes.POINTER(TaaribQiyasNass)]
        m.taarib_qiyas.restype = ctypes.c_int32

        m.taarib_makhzan_ihsaat.argtypes = [
            muashir, ctypes.POINTER(TaaribIhsaatKhazina)]
        m.taarib_makhzan_ihsaat.restype = ctypes.c_int32

        m.taarib_lawha_insha.argtypes = [
            muashir, ctypes.c_uint16, ctypes.c_uint16, ctypes.c_uint16,
            ctypes.c_uint32, hajm, ctypes.POINTER(muashir)]
        m.taarib_lawha_insha.restype = ctypes.c_int32

        m.taarib_lawha_ihdham.argtypes = [muashir, muashir]
        m.taarib_lawha_ihdham.restype = ctypes.c_int32

        m.taarib_lawha_ibda_itar.argtypes = [muashir, muashir]
        m.taarib_lawha_ibda_itar.restype = ctypes.c_int32

        m.taarib_lawha_shakl.argtypes = [
            muashir, muashir, muashir, ctypes.POINTER(TaaribMiftahShakl),
            ctypes.POINTER(TaaribMawdiShakl)]
        m.taarib_lawha_shakl.restype = ctypes.c_int32

        m.taarib_lawha_safha.argtypes = [
            muashir, muashir, ctypes.c_uint16, ctypes.POINTER(TaaribSafha)]
        m.taarib_lawha_safha.restype = ctypes.c_int32

        m.taarib_lawha_adad_safahat.argtypes = [
            muashir, muashir, ctypes.POINTER(ctypes.c_uint32)]
        m.taarib_lawha_adad_safahat.restype = ctypes.c_int32

        m.taarib_lawha_ihsaat.argtypes = [
            muashir, muashir, ctypes.POINTER(TaaribIhsaatLawha)]
        m.taarib_lawha_ihsaat.restype = ctypes.c_int32

        m.taarib_iltiqat_shaghghil.argtypes = [
            muashir, TaaribRaddIltiqat, muashir]
        m.taarib_iltiqat_shaghghil.restype = ctypes.c_int32

        m.taarib_iltiqat_awqif.argtypes = [muashir]
        m.taarib_iltiqat_awqif.restype = ctypes.c_int32

    # -- errors ------------------------------------------------------------

    def _iqra_nass(self, nida, *awail):
        """Runs one capacity-negotiated string retrieval to completion.

        ``nida`` is a khata/version function taking ``(buffer, capacity,
        required*)`` after ``awail``. The loop is bounded: a library that
        keeps demanding more than it was just given is broken, and looping
        on it forever inside a game process is worse than an empty string.
        """
        siaa = 128
        for _ in range(4):
            hadaf = ctypes.create_string_buffer(siaa)
            matlub = ctypes.c_size_t(0)
            halat = nida(*(list(awail) + [hadaf, siaa, ctypes.byref(matlub)]))
            if halat == NAJAH:
                return hadaf.value.decode("utf-8", "replace")
            if halat == SIAT_QASIRA and matlub.value > siaa:
                siaa = matlub.value
                continue
            return u""
        return u""

    def irfa(self, halat):
        """Raises :class:`KhataTaarib` for a non-zero status, built from the
        thread-local stash.

        All four parts are read *now*, before returning to the caller,
        because the stash belongs to this thread and the next successful
        call on it clears the failure. The retrieval functions themselves
        never modify the stash, so reading the code cannot erase the
        sentence.
        """
        ramz = self._iqra_nass(self.maktaba.taarib_khata_ramz)
        arabi = self._iqra_nass(self.maktaba.taarib_khata_nass, 0)
        injilizi = self._iqra_nass(self.maktaba.taarib_khata_nass, 1)
        khutwa = int(self.maktaba.taarib_khata_khutwa())
        raise KhataTaarib(halat, ramz, arabi, injilizi, khutwa)

    def tahaqqaq(self, halat):
        """Turns a status into either silence or a raised KhataTaarib."""
        if halat != NAJAH:
            self.irfa(halat)

    # -- version -----------------------------------------------------------

    def abi_isdar(self):
        """The library's (major, minor) ABI pair."""
        kabir = ctypes.c_uint32(0)
        sagheer = ctypes.c_uint32(0)
        halat = self.maktaba.taarib_abi_isdar(
            ctypes.byref(kabir), ctypes.byref(sagheer))
        if halat != NAJAH:
            self.irfa(halat)
        return int(kabir.value), int(sagheer.value)

    def isdar_nass(self):
        """The library's human-readable version string, for diagnostics."""
        return self._iqra_nass(self.maktaba.taarib_isdar_nass)


def hammil(dalil):
    """Loads ``taarib_jisr`` from the patch's own directory and returns a
    :class:`Jisr`.

    ``dalil`` is the directory the installed patch placed its libraries in.
    Resolution tries, in order:

    1. ``dalil/<arch>/<platform name>`` — the layout patches ship, where
       ``<arch>`` is ``i686``, ``x86_64`` or ``aarch64`` chosen by this
       *process*'s pointer width and machine, and
    2. ``dalil/<platform name>`` — a flat layout, for hand-built setups.

    Never the system search path: what is on a player's PATH is unknowable,
    and a same-named library from somewhere else loading successfully would
    be far worse than a clean failure naming the paths that were tried.
    """
    ism = _ism_maktaba()
    bina = _ism_bina()
    murashshahun = [
        os.path.join(dalil, bina, ism),
        os.path.join(dalil, ism),
    ]
    ilal = []
    for masar in murashshahun:
        if not os.path.isfile(masar):
            ilal.append("%s: not present" % masar)
            continue
        try:
            if hasattr(os, "add_dll_directory") and sys.platform.startswith("win"):
                # Lets a same-directory dependency resolve on Python 3.8+
                # Windows, where the DLL directory is no longer implicit.
                os.add_dll_directory(os.path.dirname(masar))
            maktaba = ctypes.CDLL(masar)
        except OSError as sabab:
            ilal.append("%s: %s" % (masar, sabab))
            continue
        return Jisr(maktaba, masar)
    tafsil = "; ".join(ilal) if ilal else "no candidate paths"
    raise KhataTaarib(
        GHAYR_MUHAYYAA,
        "",
        u"تعذّر تحميل مكتبة تعريب (%s) من: %s" % (bina, tafsil),
        "Could not load the Taarib library (%s) from: %s" % (bina, tafsil),
        11,  # iadat_tarkib_ittar: reinstall the framework for this game.
    )

# ---------------------------------------------------------------------------
# The idiomatic layer
# ---------------------------------------------------------------------------


class Khiyarat(object):
    """The layout decisions, as attributes with the ABI's own defaults.

    Every default is the zero the ABI's decoders document — automatic
    direction, detected language, no justification, leading-edge alignment,
    diacritics kept, digits untouched, overflow reported. Zeros on purpose:
    this binding does not make policy, and a wrapper that quietly defaulted
    to kashida justification would be a wrapper deciding how a patch looks.

    ``sifat`` is a list of ``(tag, value)`` pairs, e.g. ``("ss01", 1)``.
    """

    def __init__(self):
        self.ittijah = ITTIJAH_TILQAI
        self.lugha = LUGHA_TILQAI
        self.dabt = DABT_BILA
        self.muhadhaha = MUHADHAHA_BIDAYA
        self.tashkeel = TASHKEEL_IBQA
        self.arqam = ARQAM_KAMA_HIYA
        self.tajawuz = TAJAWUZ_BALLAGH
        self.hajm_adna = 0.0
        self.irtifa_satr = 0.0
        self.tabaud_ahruf = 0.0
        self.tabaud_kalimat = 0.0
        self.hiwar = False
        self.satr_wahid = False
        self.sifat = []

    def imla(self, hadaf, hafiz):
        """Writes this object into a ctypes ``TaaribKhiyarat``.

        ``hafiz`` is a list the caller keeps alive for the duration of the
        ABI call: the feature array is a separate allocation the struct only
        points at, and without a held reference Python would be free to
        collect it mid-call.
        """
        if self.sifat:
            saff_sifat = (TaaribSifa * len(self.sifat))()
            for fahras, zawj in enumerate(self.sifat):
                wasm, qeema = zawj
                bayt = wasm.encode("ascii") if not isinstance(wasm, bytes) else wasm
                if len(bayt) != 4:
                    raise ValueError(
                        "OpenType feature tag must be exactly four bytes: %r"
                        % (wasm,))
                for mawqi in range(4):
                    saff_sifat[fahras].wasm[mawqi] = bayt[mawqi]
                saff_sifat[fahras].qeema = qeema
            hafiz.append(saff_sifat)
            hadaf.sifat = ctypes.cast(
                saff_sifat, ctypes.POINTER(TaaribSifa))
            hadaf.adad_sifat = len(self.sifat)
        else:
            hadaf.sifat = ctypes.POINTER(TaaribSifa)()
            hadaf.adad_sifat = 0
        hadaf.ittijah = self.ittijah
        hadaf.lugha = self.lugha
        hadaf.dabt = self.dabt
        hadaf.muhadhaha = self.muhadhaha
        hadaf.tashkeel = self.tashkeel
        hadaf.arqam = self.arqam
        hadaf.tajawuz = self.tajawuz
        hadaf.hajm_adna = self.hajm_adna
        hadaf.irtifa_satr = self.irtifa_satr
        hadaf.tabaud_ahruf = self.tabaud_ahruf
        hadaf.tabaud_kalimat = self.tabaud_kalimat
        alam = 0
        if self.hiwar:
            alam |= KHIYAR_HIWAR
        if self.satr_wahid:
            alam |= KHIYAR_SATR_WAHID
        hadaf.alam = alam


class Nitaq(object):
    """One style span, in byte offsets over the UTF-8 text.

    Byte offsets, not character indices — the ABI speaks bytes, and a span
    computed over ``len(text)`` in characters lands mid-glyph the moment the
    text contains anything outside ASCII, which Arabic always does. Use
    ``len(prefix.encode("utf-8"))`` when converting.

    Set only what the span changes; the flags are derived from what was set,
    so a span cannot claim a colour it does not carry.
    """

    def __init__(self, bidaya, tul, id=0):
        self.bidaya = bidaya
        self.tul = tul
        self.id = id
        self.mail = False
        self.lawn = None
        self.khatt = None
        self.wazn = None
        self.hajm = None
        self.tabaud = 0.0
        self.izaha = 0.0
        #: None, or an atom (ard, irtifa, asas, marja): an opaque
        #: placeholder or inline sprite that is measured, never shaped.
        self.dharra = None

    def imla(self, hadaf):
        """Writes this span into a ctypes ``TaaribNitaqUslub``."""
        hadaf.bidaya = self.bidaya
        hadaf.tul = self.tul
        hadaf.id = self.id
        hadaf.tabaud = self.tabaud
        hadaf.izaha = self.izaha
        alam = 0
        if self.mail:
            alam |= USLUB_MAAIL
        if self.lawn is not None:
            alam |= USLUB_LAWN
            hadaf.lawn = self.lawn
        if self.khatt is not None:
            alam |= USLUB_KHATT
            hadaf.khatt = self.khatt
        if self.wazn is not None:
            alam |= USLUB_WAZN
            hadaf.wazn = self.wazn
        if self.hajm is not None:
            alam |= USLUB_HAJM
            hadaf.hajm = self.hajm
        if self.dharra is not None:
            alam |= USLUB_DHARRA
            ard, irtifa, asas, marja = self.dharra
            hadaf.ard_dharra = ard
            hadaf.irtifa_dharra = irtifa
            hadaf.asas_dharra = asas
            hadaf.marja_dharra = marja
        hadaf.alam = alam


class Takhtit(object):
    """One finished layout: a zero-copy view over the context's buffer.

    ``huruf`` and ``sutur`` are memoryviews over the caller-owned arrays the
    library wrote into — not lists, deliberately. Ren'Py asks for layout per
    displayable per frame; materialising even a modest dialogue line into a
    list would create hundreds of Python objects sixty times a second, and
    the collector's pause reclaiming them is a visible frame hitch. A
    memoryview costs one object however many glyphs it spans.

    The lifetime rule that zero-copy buys: **this result is valid only until
    the next** :meth:`Siyaq.khattit` **on the same context**, which reuses
    and may reallocate the same buffer. Read what you need, or take an
    owning copy with :meth:`insakh` if the result must outlive the frame.

    Field access goes through the ctypes array (``harf(i)`` / iteration),
    because CPython's memoryview cannot index structured formats; the
    memoryview earns its keep for ``len``, slicing, and handing the raw
    bytes to a blitter in one piece.
    """

    def __init__(self, saff_huruf, saff_sutur, makhzan):
        self._saff_huruf = saff_huruf
        self._saff_sutur = saff_sutur
        self._makhzan = makhzan
        #: Zero-copy view of exactly the written glyphs.
        self.huruf = memoryview(saff_huruf)[: makhzan.adad_huruf]
        #: Zero-copy view of exactly the written lines.
        self.sutur = memoryview(saff_sutur)[: makhzan.adad_sutur]
        self.adad_huruf = int(makhzan.adad_huruf)
        self.adad_sutur = int(makhzan.adad_sutur)
        #: The width of the widest line, in pixels.
        self.ard = makhzan.ard
        #: The total height of every line box.
        self.irtifa = makhzan.irtifa
        #: The size the text was finally laid out at — smaller than asked
        #: when the overflow policy shrank it to fit.
        self.hajm = makhzan.hajm
        self.alam = int(makhzan.alam)
        #: The overflow report, or None when nothing overflowed.
        self.tajawuz = None
        if self.alam & TAKHTIT_TAJAWUZ:
            self.tajawuz = TaaribTaqreerTajawuz.from_buffer_copy(
                makhzan.tajawuz)

    def harf(self, fahras):
        """The glyph at ``fahras``, as a ctypes view into the buffer —
        no copy, so its fields go stale with the rest of the result."""
        if fahras < 0 or fahras >= self.adad_huruf:
            raise IndexError("harf %d of %d" % (fahras, self.adad_huruf))
        return self._saff_huruf[fahras]

    def satr(self, fahras):
        """The line at ``fahras``, as a ctypes view into the buffer."""
        if fahras < 0 or fahras >= self.adad_sutur:
            raise IndexError("satr %d of %d" % (fahras, self.adad_sutur))
        return self._saff_sutur[fahras]

    def __len__(self):
        return self.adad_huruf

    def __iter__(self):
        """Yields each written glyph as a ctypes view, in visual order."""
        for fahras in range(self.adad_huruf):
            yield self._saff_huruf[fahras]

    def kul_satr(self):
        """Yields each written line as a ctypes view."""
        for fahras in range(self.adad_sutur):
            yield self._saff_sutur[fahras]

    def yameen(self):
        """Whether the layout's overall direction is right to left."""
        return bool(self.alam & TAKHTIT_YAMEEN)

    def maqsus(self):
        """Whether the overflow policy truncated the text."""
        return bool(self.alam & TAKHTIT_MAQSUS)

    def min_makhzan(self):
        """Whether this layout came from the cache — nothing was shaped."""
        return bool(self.alam & TAKHTIT_MAKHZAN)

    def insakh(self):
        """An owning copy that survives the next layout call.

        The one escape from the lifetime rule, and it pays the copy the hot
        path avoids — which is exactly why it is a separate, named step
        rather than the default.
        """
        huruf = (TaaribHarf * max(self.adad_huruf, 1))()
        sutur = (TaaribSatr * max(self.adad_sutur, 1))()
        ctypes.memmove(huruf, self._saff_huruf,
                       ctypes.sizeof(TaaribHarf) * self.adad_huruf)
        ctypes.memmove(sutur, self._saff_sutur,
                       ctypes.sizeof(TaaribSatr) * self.adad_sutur)
        makhzan = TaaribMakhzanTakhtit()
        makhzan.adad_huruf = self.adad_huruf
        makhzan.adad_sutur = self.adad_sutur
        makhzan.ard = self.ard
        makhzan.irtifa = self.irtifa
        makhzan.hajm = self.hajm
        makhzan.alam = self.alam
        if self.tajawuz is not None:
            makhzan.tajawuz = self.tajawuz
        return Takhtit(huruf, sutur, makhzan)

class Khatt(object):
    """A loaded, validated font handle.

    The library copied the bytes at load; the Python bytes object that fed
    it may be released immediately. Destroying a Khatt that a chain still
    uses is safe — the chain holds its own reference to the font resource,
    and the handle here only drops one.
    """

    def __init__(self, siyaq, maqbad):
        self._siyaq = siyaq
        self._maqbad = maqbad

    def maqbad(self):
        """The raw handle, for calls this wrapper does not cover."""
        self._tahaqqaq_hay()
        return self._maqbad

    def _tahaqqaq_hay(self):
        if self._maqbad is None:
            raise KhataTaarib(
                MAQBAD_BATIL, "",
                u"استُخدم خط بعد تدميره.",
                "A font was used after it was destroyed.", 0)

    def huwiya(self):
        """The font's stable identity — the hash of its bytes, which is what
        atlas glyph keys and layout cache keys carry. Two handles over the
        same bytes report the same identity."""
        self._tahaqqaq_hay()
        qeema = ctypes.c_uint64(0)
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_khatt_huwiya(
                self._siyaq.maqbad(), self._maqbad, ctypes.byref(qeema)))
        return int(qeema.value)

    def qiyasat(self, hajm):
        """The font's metrics scaled to ``hajm`` pixels, as a
        ``TaaribQiyasatKhatt`` the caller owns."""
        self._tahaqqaq_hay()
        hadaf = TaaribQiyasatKhatt()
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_khatt_qiyasat(
                self._siyaq.maqbad(), self._maqbad, hajm,
                ctypes.byref(hadaf)))
        return hadaf

    def aila(self):
        """The font's family name, as text."""
        self._tahaqqaq_hay()
        return self._siyaq.jisr._iqra_nass(
            self._siyaq.jisr.maktaba.taarib_khatt_aila,
            self._siyaq.maqbad(), self._maqbad)

    def ihdham(self):
        """Destroys the handle. Idempotent here: the second call is a no-op
        in Python rather than a reported stale handle, because a Ren'Py
        shutdown path often runs twice."""
        if self._maqbad is not None:
            maqbad, self._maqbad = self._maqbad, None
            self._siyaq.jisr.tahaqqaq(
                self._siyaq.jisr.maktaba.taarib_khatt_ihdham(
                    self._siyaq.maqbad(), maqbad))


class Silsila(object):
    """An ordered chain of fonts, tried in order per character.

    The chain is how fallback works without a fallback *shaper*: a character
    the first font lacks is shaped with the next font in the chain, per run,
    and every glyph in a layout names its chain index so the atlas knows
    which font it came from.
    """

    def __init__(self, siyaq, maqbad):
        self._siyaq = siyaq
        self._maqbad = maqbad

    def maqbad(self):
        """The raw handle, passed inside every layout request."""
        if self._maqbad is None:
            raise KhataTaarib(
                MAQBAD_BATIL, "",
                u"استُخدمت سلسلة خطوط بعد تدميرها.",
                "A font chain was used after it was destroyed.", 0)
        return self._maqbad

    def ihdham(self):
        """Destroys the chain handle. The fonts it referenced survive if
        their own handles do; the chain held references, not ownership."""
        if self._maqbad is not None:
            maqbad, self._maqbad = self._maqbad, None
            self._siyaq.jisr.tahaqqaq(
                self._siyaq.jisr.maktaba.taarib_silsila_ihdham(
                    self._siyaq.maqbad(), maqbad))


class Lawha(object):
    """A glyph atlas that grows and evicts at runtime.

    The Ren'Py adapter rasterizes into this and uploads pages as textures.
    Call :meth:`ibda_itar` once per frame before any lookups: it is what
    lets the atlas evict rectangles from *previous* frames while never
    reclaiming one the current frame has already been promised.
    """

    def __init__(self, siyaq, maqbad):
        self._siyaq = siyaq
        self._maqbad = maqbad

    def _tahaqqaq_hay(self):
        if self._maqbad is None:
            raise KhataTaarib(
                MAQBAD_BATIL, "",
                u"استُخدمت لوحة بعد تدميرها.",
                "An atlas was used after it was destroyed.", 0)

    def ibda_itar(self):
        """Marks a frame boundary for eviction accounting."""
        self._tahaqqaq_hay()
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_lawha_ibda_itar(
                self._siyaq.maqbad(), self._maqbad))

    def shakl(self, silsila, muarrif, hajm_rubi, khatt=0, bakat=0):
        """Where the glyph lives, rasterizing and packing it on a miss.

        ``hajm_rubi`` is the pixel size in quarter-pixels, ``khatt`` the
        index into the chain, ``bakat`` the subpixel bucket — together the
        exact key ``TaaribMiftahShakl`` freezes. Returns a
        ``TaaribMawdiShakl`` the caller owns.
        """
        self._tahaqqaq_hay()
        miftah = TaaribMiftahShakl()
        miftah.muarrif = muarrif
        miftah.hajm_rubi = hajm_rubi
        miftah.khatt = khatt
        miftah.bakat = bakat
        hadaf = TaaribMawdiShakl()
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_lawha_shakl(
                self._siyaq.maqbad(), self._maqbad, silsila.maqbad(),
                ctypes.byref(miftah), ctypes.byref(hadaf)))
        return hadaf

    def safha(self, fahras):
        """One texture page: ``(memoryview, ard, irtifa, namat)``.

        The memoryview is **borrowed** — it aliases the atlas's own memory
        and is valid only until the next call that can change the atlas:
        any :meth:`shakl`, or destroying it. Upload it into a texture in
        the same breath and let the view go; holding it across a lookup is
        reading a rectangle that may have been reassigned.
        """
        self._tahaqqaq_hay()
        hadaf = TaaribSafha()
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_lawha_safha(
                self._siyaq.maqbad(), self._maqbad, fahras,
                ctypes.byref(hadaf)))
        unwan = ctypes.cast(hadaf.bayt, ctypes.c_void_p).value
        if not unwan or hadaf.tul == 0:
            return memoryview(b""), int(hadaf.ard), int(hadaf.irtifa), \
                int(hadaf.namat)
        naw = ctypes.c_uint8 * hadaf.tul
        bayt = memoryview(naw.from_address(unwan))
        return bayt, int(hadaf.ard), int(hadaf.irtifa), int(hadaf.namat)

    def adad_safahat(self):
        """How many pages are open."""
        self._tahaqqaq_hay()
        adad = ctypes.c_uint32(0)
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_lawha_adad_safahat(
                self._siyaq.maqbad(), self._maqbad, ctypes.byref(adad)))
        return int(adad.value)

    def ihsaat(self):
        """The atlas's counters, as a ``TaaribIhsaatLawha`` the caller owns.
        A miss count that keeps climbing after the first minutes of play
        means the patch compiler missed strings — surface it."""
        self._tahaqqaq_hay()
        hadaf = TaaribIhsaatLawha()
        self._siyaq.jisr.tahaqqaq(
            self._siyaq.jisr.maktaba.taarib_lawha_ihsaat(
                self._siyaq.maqbad(), self._maqbad, ctypes.byref(hadaf)))
        return hadaf

    def ihdham(self):
        """Destroys the atlas and every borrowed page view with it."""
        if self._maqbad is not None:
            maqbad, self._maqbad = self._maqbad, None
            self._siyaq.jisr.tahaqqaq(
                self._siyaq.jisr.maktaba.taarib_lawha_ihdham(
                    self._siyaq.maqbad(), maqbad))


class Siyaq(object):
    """An engine context, as a context manager.

    Owns the layout cache, the pooled buffers, and — on this side — the one
    reusable glyph/line buffer every :meth:`khattit` writes into. One
    context serves one thread at a time; Ren'Py's interaction runs on one
    thread, so the adapter holds exactly one.

    ``with Siyaq(jisr) as siyaq:`` guarantees ``taarib_siyaq_ihdham`` runs
    even when the adapter's init raises halfway — a leaked context inside a
    long visual-novel session is memory the player paid for and never got
    back.
    """

    #: Where the reusable buffer starts. Sized for a full dialogue window;
    #: SIAT_QASIRA grows it once and it never shrinks, so after the first
    #: few interactions layout allocates nothing on this side either.
    _SIAA_HURUF = 512
    _SIAA_SUTUR = 16

    def __init__(self, jisr, mizaniyat_makhzan=4 * 1024 * 1024,
                 adad_makhazin=2):
        khiyarat = TaaribKhiyaratSiyaq()
        khiyarat.mizaniyat_makhzan = mizaniyat_makhzan
        khiyarat.adad_makhazin = adad_makhazin
        khiyarat.hashw = 0
        maqbad = ctypes.c_void_p(None)
        #: The loaded library this context lives in.
        self.jisr = jisr
        jisr.tahaqqaq(jisr.maktaba.taarib_siyaq_insha(
            ctypes.byref(khiyarat), ctypes.byref(maqbad)))
        self._maqbad = maqbad.value
        self._saff_huruf = (TaaribHarf * self._SIAA_HURUF)()
        self._saff_sutur = (TaaribSatr * self._SIAA_SUTUR)()
        #: The registered capture thunk, or None. Held here so the
        #: CFUNCTYPE object cannot be collected while the native library
        #: holds its pointer — see :meth:`shaghghil_iltiqat`.
        self._radd_iltiqat = None

    def maqbad(self):
        """The raw context handle."""
        if self._maqbad is None:
            raise KhataTaarib(
                MAQBAD_BATIL, "",
                u"استُخدم سياق بعد تدميره.",
                "A context was used after it was destroyed.", 0)
        return self._maqbad

    def __enter__(self):
        return self

    def __exit__(self, naw, qeema, athar):
        self.ihdham()
        return False

    def ihdham(self):
        """Destroys the context and everything the library holds for it."""
        if self._maqbad is not None:
            maqbad, self._maqbad = self._maqbad, None
            self.jisr.tahaqqaq(
                self.jisr.maktaba.taarib_siyaq_ihdham(maqbad))
            # Destruction dropped the capture channel with the context, so
            # nothing native holds the thunk's pointer any more and the
            # reference may finally be released.
            self._radd_iltiqat = None

    def amsah(self):
        """Empties the layout cache, keeping the context. For a scene
        change that retires a screenful of strings the cache would
        otherwise keep warm for nobody."""
        self.jisr.tahaqqaq(
            self.jisr.maktaba.taarib_siyaq_amsah(self.maqbad()))

    # -- fonts -------------------------------------------------------------

    def khatt_min_dhakira(self, bayt, fahras=0, fahs_arabi=True):
        """Loads a font from bytes and returns a :class:`Khatt`.

        ``bayt`` is the font file's content — read by the caller, because
        this library takes bytes, not paths. ``fahras`` selects a face
        inside a collection. ``fahs_arabi`` runs the Decision 6 validation:
        leave it on for any font that will shape Arabic, because a font
        without the joining tables does not degrade — it produces isolated
        forms silently, and the player reads 'the patch is broken'.
        """
        if not isinstance(bayt, (bytes, bytearray)):
            raise TypeError("font bytes must be bytes or bytearray")
        bayt = bytes(bayt)
        maqbad = ctypes.c_void_p(None)
        # c_char_p points into the bytes object; the reference held by this
        # frame keeps it alive across the call, and the library copies.
        muashir = ctypes.cast(
            ctypes.c_char_p(bayt), ctypes.POINTER(ctypes.c_uint8))
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_khatt_min_dhakira(
            self.maqbad(), muashir, len(bayt), fahras,
            1 if fahs_arabi else 0, ctypes.byref(maqbad)))
        return Khatt(self, maqbad.value)

    def silsila(self, khutut):
        """Builds a fallback chain from :class:`Khatt` handles, in the order
        they should be tried, and returns a :class:`Silsila`."""
        if not khutut:
            raise ValueError("a font chain needs at least one font")
        saff = (ctypes.c_void_p * len(khutut))()
        for fahras, khatt in enumerate(khutut):
            saff[fahras] = khatt.maqbad()
        maqbad = ctypes.c_void_p(None)
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_silsila_insha(
            self.maqbad(), saff, len(khutut), ctypes.byref(maqbad)))
        return Silsila(self, maqbad.value)

    # -- layout ------------------------------------------------------------

    def _talab(self, nass, silsila, hajm, ard_mutah, irtifa_mutah,
               nitaqat, khiyarat, hafiz):
        """Builds a ``TaaribTalab``, appending every backing allocation to
        ``hafiz`` so nothing the struct points at can be collected before
        the call returns."""
        bayt = nass.encode("utf-8") if not isinstance(nass, bytes) else nass
        hafiz.append(bayt)
        talab = TaaribTalab()
        talab.nass = ctypes.cast(
            ctypes.c_char_p(bayt), ctypes.POINTER(ctypes.c_uint8))
        talab.tul_nass = len(bayt)
        if nitaqat:
            saff_nitaqat = (TaaribNitaqUslub * len(nitaqat))()
            for fahras, nitaq in enumerate(nitaqat):
                nitaq.imla(saff_nitaqat[fahras])
            hafiz.append(saff_nitaqat)
            talab.nitaqat = ctypes.cast(
                saff_nitaqat, ctypes.POINTER(TaaribNitaqUslub))
            talab.adad_nitaqat = len(nitaqat)
        else:
            talab.nitaqat = ctypes.POINTER(TaaribNitaqUslub)()
            talab.adad_nitaqat = 0
        talab.silsila = silsila.maqbad()
        talab.hajm = hajm
        talab.ard_mutah = ard_mutah
        talab.irtifa_mutah = irtifa_mutah
        (khiyarat or Khiyarat()).imla(talab.khiyarat, hafiz)
        return talab

    def khattit(self, nass, silsila, hajm, ard_mutah=0.0, irtifa_mutah=0.0,
                nitaqat=None, khiyarat=None):
        """Lays text out and returns a :class:`Takhtit` over the context's
        reusable buffer.

        The negotiation loop is written out here so no adapter writes its
        own wrong one: on ``SIAT_QASIRA`` the required counts are read from
        the buffer struct, the arrays grow to exactly those counts, and the
        call is retried. After the first few frames the arrays never grow
        again, which is what makes this path allocation-free in practice.

        The returned view goes stale on the next ``khattit`` call — see
        :class:`Takhtit` for why that trade is worth a frame budget.
        """
        hafiz = []
        talab = self._talab(nass, silsila, hajm, ard_mutah, irtifa_mutah,
                            nitaqat, khiyarat, hafiz)
        makhzan = TaaribMakhzanTakhtit()
        for _ in range(4):
            makhzan.huruf = ctypes.cast(
                self._saff_huruf, ctypes.POINTER(TaaribHarf))
            makhzan.siaat_huruf = len(self._saff_huruf)
            makhzan.sutur = ctypes.cast(
                self._saff_sutur, ctypes.POINTER(TaaribSatr))
            makhzan.siaat_sutur = len(self._saff_sutur)
            makhzan.adad_huruf = 0
            makhzan.adad_sutur = 0
            halat = self.jisr.maktaba.taarib_takhtit(
                self.maqbad(), ctypes.byref(talab), ctypes.byref(makhzan))
            if halat == NAJAH:
                return Takhtit(self._saff_huruf, self._saff_sutur, makhzan)
            if halat != SIAT_QASIRA:
                self.jisr.irfa(halat)
            if makhzan.adad_huruf > len(self._saff_huruf):
                self._saff_huruf = (TaaribHarf * makhzan.adad_huruf)()
            if makhzan.adad_sutur > len(self._saff_sutur):
                self._saff_sutur = (TaaribSatr * makhzan.adad_sutur)()
        # Four rounds of growing to the exact demanded size and still short
        # means the library's demand is not stable; report rather than spin.
        self.jisr.irfa(SIAT_QASIRA)

    def qis(self, nass, silsila, hajm, ard_mutah=0.0, irtifa_mutah=0.0,
            nitaqat=None, khiyarat=None):
        """Measures text without positioning a glyph, returning a
        ``TaaribQiyasNass`` the caller owns. Same pipeline, same policies —
        a measurement from a different path than the layout would disagree
        with it, and reflow decisions built on it would be wrong."""
        hafiz = []
        talab = self._talab(nass, silsila, hajm, ard_mutah, irtifa_mutah,
                            nitaqat, khiyarat, hafiz)
        hadaf = TaaribQiyasNass()
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_qiyas(
            self.maqbad(), ctypes.byref(talab), ctypes.byref(hadaf)))
        return hadaf

    def ihsaat_makhzan(self):
        """The layout cache's counters, as a ``TaaribIhsaatKhazina`` the
        caller owns. Diagnostics, not control: a hit count that stays
        flat while a screen redraws the same strings every frame means
        the cache key is absorbing something that changes per frame, and
        this is the number the diagnostics overlay says it with."""
        hadaf = TaaribIhsaatKhazina()
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_makhzan_ihsaat(
            self.maqbad(), ctypes.byref(hadaf)))
        return hadaf

    # -- atlas -------------------------------------------------------------

    def lawha(self, aqsa_ard=2048, aqsa_irtifa=2048, hashw=1,
              namat=NAMAT_TAGHTIYA, mizaniya=16 * 1024 * 1024):
        """Creates a runtime glyph atlas and returns a :class:`Lawha`.

        The conservative 2048 default is the lowest common denominator of
        the GPUs Ren'Py games actually run on; the patch's own settings
        override it. ``mizaniya`` bounds the pages in bytes, because an
        atlas bounded in pages is an atlas that grows a page at a time
        until somebody notices.
        """
        maqbad = ctypes.c_void_p(None)
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_lawha_insha(
            self.maqbad(), aqsa_ard, aqsa_irtifa, hashw, namat, mizaniya,
            ctypes.byref(maqbad)))
        return Lawha(self, maqbad.value)

    # -- runtime string capture --------------------------------------------

    def shaghghil_iltiqat(self, radd):
        """Turns runtime string capture on: ``radd`` is called as
        ``radd(nass)`` — one owned ``str`` per layout request this
        context has not seen before — until :meth:`awqif_iltiqat` or
        :meth:`ihdham`. Cache misses are precisely "new text", so the
        stream self-deduplicates and steady-state frames report nothing.
        Registering again replaces the previous sink.

        **The keep-alive rule.** The CFUNCTYPE thunk built here is stored
        on this context for as long as it is registered, and that storage
        is load-bearing, not bookkeeping: ctypes backs every callback
        with a small piece of native code that is freed when the Python
        object is collected, while the library holds a raw pointer to it.
        A thunk garbage-collected while native code still holds its
        pointer is a call through freed memory — a crash on whichever
        frame next captures a string, long after the actual mistake — and
        it is the single most common ctypes defect. Callers keep no
        reference of their own; this method keeps the one that matters.

        **The locking contract.** The sink runs on whichever thread
        called :meth:`khattit`, while that context's lock is held. Both
        halves of the registration contract are therefore the caller's to
        keep: return promptly, because a frame is waiting on it, and
        never call back into Taarib from inside it — not layout, not
        measurement, not any wrapper in this module on any handle —
        because the lock it would need is the lock it is already running
        under, and the deadlock lands on the calling thread mid-frame.

        The text is copied and decoded before ``radd`` sees it, so
        keeping it costs nothing and borrows nothing. An exception raised
        by ``radd`` cannot cross the C boundary: ctypes reports it to
        ``sys.stderr`` and the layout call continues, so a sink that can
        fail should catch and record its own failures.
        """
        if not callable(radd):
            raise TypeError("the capture sink must be callable")

        def _jisr_radd(mustakhdim, nass, tul):
            # string_at copies: the native bytes die when this returns,
            # and the sink receives text it owns.
            radd(ctypes.string_at(nass, tul).decode("utf-8", "replace"))

        thunk = TaaribRaddIltiqat(_jisr_radd)
        self.jisr.tahaqqaq(self.jisr.maktaba.taarib_iltiqat_shaghghil(
            self.maqbad(), thunk, None))
        # Replace the held reference only after the library accepted the
        # new registration: on failure the previous sink, if any, is
        # still the one registered and still the one being kept alive.
        self._radd_iltiqat = thunk

    def awqif_iltiqat(self):
        """Turns runtime string capture off. Delivery happens under the
        same context lock this call takes, so when it returns the last
        report has already been delivered and none will follow — which is
        the one moment the held thunk reference may be dropped, and it is
        dropped here. Stopping capture that was never started succeeds
        and does nothing."""
        self.jisr.tahaqqaq(
            self.jisr.maktaba.taarib_iltiqat_awqif(self.maqbad()))
        self._radd_iltiqat = None
