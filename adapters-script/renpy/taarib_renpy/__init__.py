# -*- coding: utf-8 -*-
"""taarib_renpy — the in-engine adapter, and the branch between two engines.

Ren'Py is two engines wearing one name. From the release that bundles
HarfBuzz and FriBidi it shapes and reorders Arabic itself and does it well;
below that — and in any build of that release compiled without the optional
libraries — its text layout is a per-character blit with no joining. The Rust
side decided which of those this game is, wrote the answer into
``game/taarib/idad.json``, and this package reads it. It does not re-decide,
and it does not guess: a module that inferred the rung from
``renpy.version_tuple`` would be answering a question that already has an
answer, and would disagree with it on exactly the builds that matter.

Two paths, and only two
-----------------------
:func:`_rakkib_khadim`
    The engine shapes. Register the font through ``config.font_replacement_map``
    and the ``gui`` variables, set the language and direction properties, and
    stop. Nothing in that path lays out a glyph, and if anything ever does it
    is a bug: it would be a second, worse implementation of something the
    engine already does correctly, and the two would disagree the first time a
    font's ``GSUB`` did something clever.

:func:`_rakkib_istila`
    The engine cannot shape. Install a text filter that wraps Arabic in a
    custom text tag, and register a handler for that tag which lays the run out
    through Taarib's C ABI — :mod:`taarib_renpy.jisr` — and returns a
    displayable that blits Taarib's own glyphs. Correct on screen, and no
    longer text to anything that inspects it.

If the takeover cannot be installed for any reason at all, this module falls
back to the configuration path, records why in :func:`athar`, and lets the game
run. A patch that refuses to load is a patch that has taken somebody's game
away from them; a patch that installs the font and says plainly that it could
not do the rest is one they can still play and still report.

No evaluation, ever
-------------------
The settings are JSON, read with :mod:`json`. The translation is delivered as
``game/tl/arabic/*.rpy``, which Ren'Py compiles itself through its own
localization mechanism. Nothing in this package calls ``eval`` or ``exec``, and
there is no code path through which patch content becomes code.

Interpreter target
------------------
Written for the CPython 3.9 that Ren'Py 8 bundles, and — deliberately —
written so that it still *parses* on the Python 2.7 that older builds run:
no f-strings, no annotations, no ``nonlocal``, no keyword-only arguments.
That is not nostalgia. The takeover needs :mod:`taarib_renpy.jisr`, which is
Python 3 only, and a module that raised ``SyntaxError`` at import on a Python 2
build would take the whole game down at the title screen instead of reporting
that it needs the Python 3 build of the engine. Parsing everywhere is what buys
the ability to decline politely.
"""

from __future__ import unicode_literals

import json
import os
import sys

__all__ = [
    "RUTBA_IDAD", "RUTBA_TASHEEH", "RUTBA_ISTILA",
    "LUGHA", "MALAF_BAYANAT",
    "Idad", "NassTaarib",
    "rakkib", "sajjil_khatt", "sajjil_hajm", "sajjil_ittijah",
    "hal_jahiz", "hal_istila", "athar", "idad", "awqif",
]

# ---------------------------------------------------------------------------
# The rungs, as the Rust side numbers them
#
# The numbers are the contract. They reach the capability report, the
# diagnostics bundle and the registry's per-game records, where a value written
# by an older build has to keep meaning what it meant, so they are compared as
# integers here and never re-spelled as strings.
# ---------------------------------------------------------------------------

#: The engine shapes correctly: font, direction, translation, stop.
RUTBA_IDAD = 1

#: The engine shapes and lays right-to-left text out wrongly.
RUTBA_TASHEEH = 2

#: The engine cannot shape. Taarib lays out and draws.
RUTBA_ISTILA = 3

#: The language name the translation is installed under. Ren'Py's own
#: convention is a language *name* rather than a code: ``game/tl/<name>/`` and
#: ``renpy.change_language(<name>)`` have to agree, and every Ren'Py game that
#: ships a translation spells it this way.
LUGHA = "arabic"

#: Where the settings live, relative to the game directory.
MALAF_BAYANAT = os.path.join("taarib", "idad.json")

#: The custom text tag the takeover registers. Deliberately unlikely to
#: collide: a game that already defines ``{taarib}`` is a game that has already
#: been patched, and the installer refuses that case before this module runs.
WASM = "taarib"

# ---------------------------------------------------------------------------
# Module state
#
# Held at module level rather than on an object because Ren'Py imports this
# once per process and the adapter is a singleton by construction. Every entry
# point below is safe to call twice: reinstalling is what a developer does
# while iterating, and a second call that doubled a replacement map or
# registered a second text filter would be a slow leak nobody would trace.
# ---------------------------------------------------------------------------

_idad = None
_athar = []
_jisr = None
_siyaq = None
_silsila = None
_lawha = None
_khatt = None
_makhzan = {}
_istila = False
_rukkiba = False


def _sajjil(satr):
    """Records one line of what the adapter did, for the game's own log.

    Kept in memory and echoed into Ren'Py's log when one is available. This is
    the only diagnostic channel a player can be asked for, so it records
    successes as well as failures: "the takeover was installed" and "the
    takeover declined because the library would not load" are equally useful,
    and the second is useless without the first for contrast.
    """
    _athar.append(satr)
    try:
        sys.stderr.write("taarib: %s\n" % (satr,))
    except Exception:
        # A Windows GUI build has no usable stderr, and a game must not die
        # because the adapter tried to describe itself. The line is still in
        # `athar()`, which is where the diagnostics bundle reads it from.
        pass


def athar():
    """Every line the adapter recorded, oldest first."""
    return list(_athar)


def idad():
    """The loaded settings, or ``None`` before :func:`rakkib` has run."""
    return _idad


def hal_jahiz():
    """Whether the adapter installed anything at all."""
    return _rukkiba


def hal_istila():
    """Whether the glyph takeover is the path that actually installed.

    False on the configuration path *and* on a takeover that declined, which
    is the distinction a bug report needs: the same rung can end in two very
    different places and only this says which.
    """
    return _istila


# ---------------------------------------------------------------------------
# The settings
# ---------------------------------------------------------------------------


class Idad(object):
    """What the patch decided, as the Rust side wrote it.

    Every field has a default that is safe on its own, because a settings file
    written by a newer Taarib may carry keys this build does not know and may
    lack ones it expects. Refusing to start over a missing optional key would
    turn a forward-compatibility problem into an unplayable game.
    """

    def __init__(self, khaam=None):
        khaam = khaam or {}
        #: The rung, as one of the ``RUTBA_*`` numbers.
        self.rutba = _raqm(khaam.get("rutba"), RUTBA_IDAD)
        #: How sure the probe was, 0..=100.
        self.thiqa = _raqm(khaam.get("thiqa"), 0)
        #: The font file, relative to the game directory.
        self.khatt = khaam.get("khatt") or ""
        #: A size override, or None.
        self.hajm = _raqm(khaam.get("hajm"), 0) or None
        #: Where the native library lives, relative to the game directory.
        self.dalil_jisr = khaam.get("dalil_jisr") or os.path.join("taarib", "jisr")
        #: The atlas, relative to the game directory. Unused on the
        #: configuration path and required by the takeover.
        self.lawha = khaam.get("lawha") or os.path.join("taarib", "lawha.png")
        #: The language name the translation was installed under.
        self.lugha = khaam.get("lugha") or LUGHA
        #: Every line the probe produced.
        self.athar = list(khaam.get("athar") or [])

    def yastawli(self):
        """Whether these settings ask for the glyph takeover."""
        return self.rutba >= RUTBA_ISTILA


def _raqm(qeema, badeel):
    """An integer from a JSON value, or ``badeel`` for anything else."""
    try:
        return int(qeema)
    except (TypeError, ValueError):
        return badeel


def _iqra_bayanat(dalil_luba):
    """Reads the settings file. JSON, parsed — never evaluated.

    A missing or malformed file is not fatal: the adapter falls back to the
    configuration path with the defaults, which registers the font and changes
    nothing else. That is the failure mode a player can still play through.
    """
    masar = os.path.join(dalil_luba, MALAF_BAYANAT)
    try:
        malaf = open(masar, "rb")
    except (IOError, OSError):
        _sajjil("no settings file at %s; using the configuration path with defaults"
                % (masar,))
        return Idad()
    try:
        khaam = json.loads(malaf.read().decode("utf-8"))
    except (ValueError, UnicodeDecodeError):
        _sajjil("the settings file at %s is not readable JSON; using defaults" % (masar,))
        return Idad()
    finally:
        malaf.close()
    if not isinstance(khaam, dict):
        _sajjil("the settings file at %s is not an object; using defaults" % (masar,))
        return Idad()
    return Idad(khaam)


def _dalil_luba(mumarrar):
    """The game directory, from the caller or from Ren'Py itself."""
    if mumarrar:
        return mumarrar
    try:
        import renpy
        return renpy.config.gamedir
    except Exception:
        return os.getcwd()

# ---------------------------------------------------------------------------
# Installation
# ---------------------------------------------------------------------------


def rakkib(dalil_luba=None):
    """Installs the adapter. Called once, from the generated ``.rpy``.

    Safe to call twice: the second call reports that it is already installed
    and changes nothing. Ren'Py reloads a game's scripts on every developer
    reload, and an adapter that re-registered its text filter each time would
    stack them until the frame budget noticed.

    Returns the loaded :class:`Idad` so the caller can log it.
    """
    global _idad, _rukkiba, _istila

    if _rukkiba:
        _sajjil("already installed; this call changed nothing")
        return _idad

    dalil_luba = _dalil_luba(dalil_luba)
    _idad = _iqra_bayanat(dalil_luba)
    for satr in _idad.athar:
        _athar.append("probe: %s" % (satr,))
    _sajjil("rung %d, probe confidence %d" % (_idad.rutba, _idad.thiqa))

    # The configuration path runs on *both* rungs. On the shaping generation it
    # is the whole adapter; on the takeover it is the floor underneath it, so
    # that a takeover which declines still leaves the game with the right font
    # rather than with tofu.
    _rakkib_khadim(dalil_luba, _idad)

    if _idad.yastawli():
        if sys.version_info[0] < 3:
            _sajjil(
                "this build runs Python 2 and the glyph takeover needs the Python 3 "
                "build of the engine, so the font and the translation were installed "
                "and the text will not join")
        else:
            try:
                _rakkib_istila(dalil_luba, _idad)
                _istila = True
                _sajjil("the glyph takeover is installed; text is drawn by Taarib and is "
                        "not selectable inside the game")
            except Exception as sabab:
                # Deliberately broad. Every failure here — a missing library, a
                # mismatched ABI, an engine build without the hook this needs —
                # has the same correct response: keep the font, keep the
                # translation, and say what happened.
                _sajjil("the glyph takeover declined: %s" % (sabab,))

    _rukkiba = True
    return _idad


def awqif():
    """Removes what can be removed and forgets the rest.

    Ren'Py has no uninstall hook and this is not one: the real uninstall is
    deleting ``game/tl/arabic``, which is the Rust side's job. This exists for
    a developer reloading a game with the patch half-installed, and for the
    shutdown path, where the native context has to be destroyed or a long
    session leaks the atlas it built.
    """
    global _jisr, _siyaq, _silsila, _lawha, _khatt, _istila, _rukkiba

    _makhzan.clear()
    for maqbad in (_lawha, _silsila, _khatt):
        if maqbad is not None:
            try:
                maqbad.ihdham()
            except Exception:
                pass
    if _siyaq is not None:
        try:
            _siyaq.ihdham()
        except Exception:
            pass
    _lawha = None
    _silsila = None
    _khatt = None
    _siyaq = None
    _jisr = None
    _istila = False
    _rukkiba = False
    _sajjil("the adapter released its native resources")

# ---------------------------------------------------------------------------
# The configuration path
#
# Every property below is set through a presence check rather than assumed.
# Ren'Py's configuration surface is large and version-dependent — a variable
# that exists in 8.2 may not exist in 7.4, and the whole point of this adapter
# is that it runs on both — and a module that named one this engine does not
# have would raise during `init`, which Ren'Py reports as a game that will not
# start. Setting what is there and recording what is not is the behaviour that
# degrades instead of breaking.
# ---------------------------------------------------------------------------


def _ayyin(kaain, ism, qeema):
    """Sets an attribute only if it already exists, and says which it did."""
    if not hasattr(kaain, ism):
        return False
    try:
        setattr(kaain, ism, qeema)
        return True
    except Exception:
        return False


def _rakkib_khadim(dalil_luba, idad_hali):
    """Registers the font, the language and the direction, and nothing else.

    Three mechanisms, in the order of how much they cover:

    1. ``config.font_replacement_map`` — Ren'Py's own documented redirection
       from a font a style asks for to one it gets. This is the only mechanism
       that reaches a font named inside a screen this patch never saw, which is
       why it is populated from the ``gui`` variables rather than from a fixed
       list.
    2. The ``gui`` variables themselves, for the text that resolves its font
       through them.
    3. ``config.language`` and the direction properties, which is the part the
       shaping generation still needs: an engine that joins Arabic correctly
       still aligns the paragraph to the left until it is told otherwise, and
       left-aligned Arabic is wrong even when every letter in it joins.
    """
    try:
        import renpy
    except ImportError:
        _sajjil("not running inside Ren'Py; nothing was configured")
        return

    khatt = idad_hali.khatt
    if not khatt:
        _sajjil("no font was named in the settings; the game's own fonts are unchanged")
    else:
        masar = os.path.join(dalil_luba, khatt)
        if not os.path.exists(masar):
            _sajjil("the font named in the settings is not at %s; the game's own fonts "
                    "are unchanged" % (masar,))
            khatt = ""

    if khatt:
        _sajjil_ibdal(renpy, khatt)
        sajjil_khatt(khatt)
    if idad_hali.hajm:
        sajjil_hajm(idad_hali.hajm)
    sajjil_ittijah()


def _asmaa_khutut_gui(renpy):
    """Every font the game's ``gui`` names, by variable and by value.

    Walked rather than listed. The standard GUI defines about a dozen
    ``*_font`` variables and a game that customised its interface defines more;
    a fixed list would miss exactly the ones a translator notices.
    """
    natija = []
    gui = getattr(renpy.store, "gui", None)
    if gui is None:
        return natija
    for ism in dir(gui):
        if not ism.endswith("_font") or ism.startswith("_"):
            continue
        try:
            qeema = getattr(gui, ism)
        except Exception:
            continue
        if isinstance(qeema, str) or (bytes is not str and isinstance(qeema, bytes)):
            natija.append((ism, qeema))
    return natija


def _sajjil_ibdal(renpy, khatt):
    """Fills ``config.font_replacement_map`` for every face the game names.

    All four bold/italic combinations per face, because Ren'Py keys the map on
    the triple and a game that renders a name in bold would otherwise fall back
    to its own font for that one style — which is how a patch ends up with one
    unjoined word in an otherwise correct line.
    """
    kharita = getattr(renpy.config, "font_replacement_map", None)
    if kharita is None:
        _sajjil("this engine has no config.font_replacement_map; the font is set through "
                "styles only")
        return
    adad = 0
    for _, qadeem in _asmaa_khutut_gui(renpy):
        if not qadeem or qadeem == khatt:
            continue
        for ghaliz in (False, True):
            for maail in (False, True):
                kharita[(qadeem, ghaliz, maail)] = (khatt, ghaliz, maail)
                adad += 1
    _sajjil("registered %d font replacements onto %s" % (adad, khatt))


def sajjil_khatt(khatt):
    """Points the standard GUI's font variables and the default style at
    ``khatt``.

    Called from the generated ``.rpy`` as well as from :func:`rakkib`, because
    the ``gui`` variables are only meaningful after the game's own ``init``
    has defined them and the generated file is what runs at the right moment.
    """
    try:
        import renpy
    except ImportError:
        return
    gui = getattr(renpy.store, "gui", None)
    adad = 0
    if gui is not None:
        for ism, _ in _asmaa_khutut_gui(renpy):
            if _ayyin(gui, ism, khatt):
                adad += 1
    uslub = getattr(renpy.store, "style", None)
    if uslub is not None and hasattr(uslub, "default"):
        if _ayyin(uslub.default, "font", khatt):
            adad += 1
    _sajjil("set %d font variables to %s" % (adad, khatt))


def sajjil_hajm(hajm):
    """Applies the patch's size override to the default text style."""
    try:
        import renpy
    except ImportError:
        return
    uslub = getattr(renpy.store, "style", None)
    if uslub is not None and hasattr(uslub, "default") and _ayyin(uslub.default, "size", hajm):
        _sajjil("set the default text size to %d" % (hajm,))


def sajjil_ittijah():
    """Sets the direction, alignment and line-breaking properties.

    This is the half of the configuration path that the shaping generation
    still needs, and the reason a game whose engine joins Arabic perfectly can
    still look wrong: joining and *layout direction* are different features,
    and Ren'Py ships the first without turning on the second.

    What is set, and why each one:

    ``config.rtl``
        Turns on the engine's own right-to-left paragraph handling where the
        build has it. Absent on older builds, hence the presence check.
    ``style.default.text_align = 1.0``
        Aligns the paragraph to the trailing edge, which for Arabic is the
        right. Without it, correctly joined Arabic sits against the left margin
        with a ragged right edge, which is the wrong edge to be ragged.
    ``style.default.layout = "subtitle"``
        Ren'Py's line-breaking mode that balances lines rather than filling
        greedily. Not a direction setting, and it matters here anyway: a greedy
        break on an RTL paragraph puts the short line at the visual start.
    ``style.default.language = "unicode"``
        Selects the Unicode line-breaking algorithm rather than the western
        space-only one, which is what stops a break landing inside an Arabic
        word joined across a zero-width joiner.
    ``config.language``
        The language a new game starts in. Deliberately *not*
        ``renpy.change_language``: a player who switches back to the original
        in preferences must keep that choice, and forcing the language on every
        interaction would take it away from them on every screen.
    """
    try:
        import renpy
    except ImportError:
        return
    fuil = []
    if _ayyin(renpy.config, "rtl", True):
        fuil.append("config.rtl")
    if _ayyin(renpy.config, "language", LUGHA):
        fuil.append("config.language")
    uslub = getattr(renpy.store, "style", None)
    if uslub is not None and hasattr(uslub, "default"):
        if _ayyin(uslub.default, "text_align", 1.0):
            fuil.append("text_align")
        if _ayyin(uslub.default, "layout", "subtitle"):
            fuil.append("layout")
        if _ayyin(uslub.default, "language", "unicode"):
            fuil.append("text language")
    if fuil:
        _sajjil("direction and layout set: %s" % (", ".join(fuil),))
    else:
        _sajjil("no direction property on this engine could be set from Python; the "
                "generated translate block sets what it can")

# ---------------------------------------------------------------------------
# The takeover path
#
# Reached only when the recorded rung says the engine cannot shape. It has
# three parts:
#
#   1. the native side — the library, a context, the font, a chain, an atlas;
#   2. a text filter that wraps Arabic in a custom text tag;
#   3. a handler for that tag that returns a displayable per run.
#
# Every one of the three is a documented Ren'Py mechanism. Nothing here patches
# a method on an engine class, and that is not fastidiousness: Ren'Py's text
# pipeline is rewritten between minor releases, and an adapter built on a
# monkey-patch would work on the build it was written against and fail
# silently — with the text still on screen, still unjoined — on every other.
# ---------------------------------------------------------------------------

#: Unicode blocks whose presence means a run has to be laid out by Taarib.
#: Arabic, Arabic Supplement, Arabic Extended-A and -B, the presentation form
#: blocks, plus Thaana and Syriac, which join by the same rules and which the
#: same shaping path handles correctly.
_MADAYAT_YAMEEN = (
    (0x0590, 0x05FF),   # Hebrew
    (0x0600, 0x06FF),   # Arabic
    (0x0700, 0x074F),   # Syriac
    (0x0750, 0x077F),   # Arabic Supplement
    (0x0780, 0x07BF),   # Thaana
    (0x08A0, 0x08FF),   # Arabic Extended-A
    (0xFB50, 0xFDFF),   # Arabic Presentation Forms-A
    (0xFE70, 0xFEFF),   # Arabic Presentation Forms-B
    (0x10E60, 0x10E7F),  # Rumi numeral symbols
    (0x1EC70, 0x1ECBF),  # Indic Siyaq numbers
    (0x1EE00, 0x1EEFF),  # Arabic mathematical alphabetic symbols
)


def _fihi_yameen(nass):
    """Whether a string contains anything this adapter has to lay out."""
    for harf in nass:
        raqm = ord(harf)
        for adna, aqsa in _MADAYAT_YAMEEN:
            if adna <= raqm <= aqsa:
                return True
    return False


def _rakkib_istila(dalil_luba, idad_hali):
    """Brings up the native side and registers the filter and the tag.

    Raises on any failure, and the caller turns that into a recorded reason and
    a fall back to the configuration path. Nothing here is caught locally,
    because a partially installed takeover — a context with no atlas, a tag
    handler with no font — would produce a game that draws nothing where its
    dialogue used to be, which is worse than not joining.
    """
    global _jisr, _siyaq, _silsila, _lawha, _khatt

    import renpy
    from . import jisr as _wahdat_jisr

    naw_qabil = getattr(renpy, "TEXT_DISPLAYABLE", None)
    if naw_qabil is None:
        raise RuntimeError(
            "this engine has no renpy.TEXT_DISPLAYABLE, so a custom text tag cannot "
            "return a displayable and there is no supported way to draw the run")
    if getattr(renpy.config, "custom_text_tags", None) is None:
        raise RuntimeError("this engine has no config.custom_text_tags")

    # The one capability that cannot be checked from a version number: the
    # takeover turns a byte buffer into a surface once per distinct run, and an
    # SDL binding without `frombuffer` leaves no supported way to do it. Proved
    # here on four bytes, at install time, so the failure is a recorded reason
    # and a fall back rather than a black rectangle where the dialogue was.
    import pygame_sdl2
    if not hasattr(pygame_sdl2.image, "frombuffer"):
        raise RuntimeError(
            "this engine's pygame_sdl2 has no image.frombuffer, so a rasterized run "
            "cannot be turned into a surface")
    pygame_sdl2.image.frombuffer(b"\x00\x00\x00\x00", (1, 1), "RGBA")

    _jisr = _wahdat_jisr.hammil(os.path.join(dalil_luba, idad_hali.dalil_jisr))
    _sajjil("loaded %s (ABI %d.%d)" % (_jisr.masar, _jisr.kabir, _jisr.sagheer))

    _siyaq = _wahdat_jisr.Siyaq(_jisr)
    masar_khatt = os.path.join(dalil_luba, idad_hali.khatt)
    malaf = open(masar_khatt, "rb")
    try:
        bayt = malaf.read()
    finally:
        malaf.close()
    _khatt = _siyaq.khatt_min_dhakira(bayt, 0, True)
    _silsila = _siyaq.silsila([_khatt])
    _lawha = _siyaq.lawha()
    _sajjil("font %s loaded and an atlas opened" % (_khatt.aila(),))

    renpy.config.custom_text_tags[WASM] = _muallij_wasm
    _sajjil("registered the {%s} text tag" % (WASM,))

    # `say_menu_text_filter` covers dialogue and menu choices, which is where
    # the great majority of a visual novel's Arabic is. `replace_text` covers
    # everything else that reaches the text engine, including interface strings
    # inside screens this patch never saw. Both are documented hooks and both
    # are chained rather than replaced, so a game that already installed one
    # keeps it.
    renpy.config.say_menu_text_filter = _sallsil(
        getattr(renpy.config, "say_menu_text_filter", None))
    if hasattr(renpy.config, "replace_text"):
        renpy.config.replace_text = _sallsil(getattr(renpy.config, "replace_text", None))
        _sajjil("chained onto config.say_menu_text_filter and config.replace_text")
    else:
        _sajjil("chained onto config.say_menu_text_filter; this engine has no "
                "config.replace_text, so interface text outside dialogue keeps the "
                "engine's own layout")


def _sallsil(sabiq):
    """Wraps an existing text filter rather than replacing it.

    A game that already sets ``say_menu_text_filter`` — to expand an
    abbreviation, to censor, to add furigana — is a game whose filter has to
    keep running. Replacing it would break a feature that has nothing to do
    with Arabic, and the player would have no way to connect the two.
    """

    def _murashshih(nass):
        if sabiq is not None:
            try:
                nass = sabiq(nass)
            except Exception:
                pass
        return _laff(nass)

    return _murashshih


def _laff(nass):
    """Wraps a string in the custom tag when it needs laying out.

    Whole-string, not per-word. The bidirectional algorithm is defined over a
    paragraph: deciding a word's direction without its neighbours produces text
    that is individually correct and collectively in the wrong order, which is
    the classic way a naive right-to-left patch fails.
    """
    if not nass or not _fihi_yameen(nass):
        return nass
    if ("{%s}" % (WASM,)) in nass:
        return nass
    return "{%s}%s{/%s}" % (WASM, nass, WASM)


def _muallij_wasm(wasm, muamil, muhtawa):
    """Turns the text inside ``{taarib}`` into displayables Taarib draws.

    ``muhtawa`` is Ren'Py's own tokenisation of the wrapped span: a list of
    ``(kind, value)`` pairs where the kind is one of ``TEXT_TEXT``,
    ``TEXT_TAG``, ``TEXT_PARAGRAPH`` or ``TEXT_DISPLAYABLE``. Consecutive text
    tokens are joined and replaced with one displayable; everything else is
    passed through untouched, in the order it arrived.

    **The limitation, stated rather than hidden.** Markup splits a
    bidirectional paragraph. ``{i}`` in the middle of an Arabic sentence yields
    two runs, each laid out correctly on its own and placed left to right
    relative to each other, because reordering the token list would put a
    closing tag before its opening one and Ren'Py would raise. Unmarked Arabic
    — which is nearly all dialogue — is one run and is fully correct. The
    shaping generation has none of this, which is one more reason the tier
    probe exists.

    An argument — ``{taarib=ff8800}`` — sets the run's colour. An embedded
    displayable is composited as it was drawn rather than tinted by the
    surrounding text colour, so a run inside a ``{color}`` tag would otherwise
    come out in the default colour; naming it explicitly is the way to override
    that.
    """
    try:
        import renpy
    except ImportError:
        return muhtawa

    naw_nass = getattr(renpy, "TEXT_TEXT", 0)
    naw_qabil = getattr(renpy, "TEXT_DISPLAYABLE", None)
    if naw_qabil is None or not _istila:
        return muhtawa

    lawn = _lawn_min_muamil(muamil)
    natija = []
    tarakum = []

    def _afrigh():
        if not tarakum:
            return
        jumla = "".join(tarakum)
        del tarakum[:]
        if _fihi_yameen(jumla):
            natija.append((naw_qabil, NassTaarib(jumla, lawn=lawn)))
        else:
            natija.append((naw_nass, jumla))

    for band in muhtawa:
        try:
            naw, qeema = band
        except (TypeError, ValueError):
            natija.append(band)
            continue
        if naw == naw_nass:
            tarakum.append(qeema)
        else:
            _afrigh()
            natija.append(band)
    _afrigh()
    return natija


def _lawn_min_muamil(muamil):
    """The run's colour: the tag's argument, the style's, or opaque white."""
    if muamil:
        khaam = muamil.lstrip("#")
        if len(khaam) in (6, 8):
            try:
                qiyam = [int(khaam[fahras:fahras + 2], 16) for fahras in (0, 2, 4)]
                shaffafiya = int(khaam[6:8], 16) if len(khaam) == 8 else 255
                return (qiyam[0], qiyam[1], qiyam[2], shaffafiya)
            except ValueError:
                pass
    return _lawn_uslub()


def _lawn_uslub():
    """The default text colour this game uses, as an RGBA tuple.

    Read from the game's own style rather than assumed, because a visual novel
    with a dark interface draws its dialogue in near-white and one with a light
    interface in near-black, and a takeover that hard-coded either would make
    the text invisible on half the games it runs on.
    """
    try:
        import renpy
    except ImportError:
        return (255, 255, 255, 255)
    for masdar, ism in (
        (getattr(renpy.store, "gui", None), "text_color"),
        (getattr(getattr(renpy.store, "style", None), "default", None), "color"),
    ):
        if masdar is None:
            continue
        khaam = getattr(masdar, ism, None)
        if khaam is None:
            continue
        try:
            return renpy.easy.color(khaam)
        except Exception:
            continue
    return (255, 255, 255, 255)

# ---------------------------------------------------------------------------
# The displayable
# ---------------------------------------------------------------------------

#: How many rasterized runs are kept. A visual novel's dialogue is drawn one
#: character at a time as it types, so the same run is laid out on every frame
#: of a line: without a cache, a sixty-character line costs sixty layouts and
#: sixty rasterizations per line. With one, it costs one.
AQSA_MAKHZAN = 512


def _sath_min_takhtit(nass, hajm, lawn, ard_mutah):
    """Lays a run out through the C ABI and rasterizes it into one surface.

    The glyph placement convention is stated here, once, because it is the one
    thing in this file that cannot be read off the ABI headers:

    * a line's pen starts at ``(satr.bidaya, satr.asas)`` — the leading edge and
      the baseline, in the layout's own coordinates, already in visual order;
    * a glyph's position ``(harf.s, harf.a)`` is an offset from that pen;
    * the glyph's *bitmap* corner is that position plus ``(izaha_s, -izaha_a)``,
      which is FreeType's ``bitmap_left`` and ``bitmap_top`` convention: the
      horizontal offset is to the right and the vertical one is *up* from the
      baseline.

    Everything else follows from those three. If a future ABI changes any of
    them this is the function to change, and it is the only one.
    """
    import math
    import pygame_sdl2

    # One frame boundary per run, before any lookup for it. Inside the run the
    # atlas may not reclaim a rectangle it has already promised, which is what
    # makes it safe to hold every glyph's position until the last one is
    # blitted.
    _lawha.ibda_itar()

    khiyarat = _khiyarat_takhtit()
    takhtit = _siyaq.khattit(nass, _silsila, float(hajm), float(ard_mutah), 0.0,
                             None, khiyarat)
    ard = max(1, int(math.ceil(takhtit.ard)))
    irtifa = max(1, int(math.ceil(takhtit.irtifa)))
    hajm_rubi = int(round(float(hajm) * 4.0))
    ahmar, akhdar, azraq, shaffafiya = lawn
    mistara = bytearray(ard * irtifa * 4)

    for fahras_satr in range(takhtit.adad_sutur):
        satr = takhtit.satr(fahras_satr)
        awwal = satr.awwal_harf
        for fahras in range(awwal, awwal + satr.adad_huruf):
            if fahras >= takhtit.adad_huruf:
                break
            harf = takhtit.harf(fahras)
            mawdi = _lawha.shakl(_silsila, harf.muarrif, hajm_rubi, harf.khatt, 0)
            if mawdi.ard == 0 or mawdi.irtifa == 0:
                continue
            safha, ard_safha, irtifa_safha, _ = _lawha.safha(mawdi.safha)
            if not len(safha) or ard_safha == 0:
                continue
            s0 = int(round(satr.bidaya + harf.s)) + mawdi.izaha_s
            a0 = int(round(satr.asas + harf.a)) - mawdi.izaha_a
            _albit(mistara, ard, irtifa, safha, ard_safha, irtifa_safha, mawdi,
                   s0, a0, ahmar, akhdar, azraq, shaffafiya)

    return pygame_sdl2.image.frombuffer(bytes(mistara), (ard, irtifa), "RGBA")


def _albit(mistara, ard, irtifa, safha, ard_safha, irtifa_safha, mawdi,
           s0, a0, ahmar, akhdar, azraq, shaffafiya):
    """Copies one glyph's coverage into the run's RGBA buffer.

    Written out as an explicit loop with every bound checked rather than as a
    slice assignment, because the rectangle comes from the atlas and the
    destination from a layout, and a glyph that hangs off the edge of its own
    run — an initial form with a long tail, a mark positioned left of its base —
    is ordinary rather than exceptional. Clipping is the normal case here.
    """
    for saf in range(mawdi.irtifa):
        hadaf_a = a0 + saf
        if hadaf_a < 0 or hadaf_a >= irtifa:
            continue
        masdar_a = mawdi.a + saf
        if masdar_a < 0 or masdar_a >= irtifa_safha:
            continue
        asas_masdar = masdar_a * ard_safha
        asas_hadaf = hadaf_a * ard * 4
        for amud in range(mawdi.ard):
            hadaf_s = s0 + amud
            if hadaf_s < 0 or hadaf_s >= ard:
                continue
            masdar_s = mawdi.s + amud
            if masdar_s < 0 or masdar_s >= ard_safha:
                continue
            taghtiya = safha[asas_masdar + masdar_s]
            if not taghtiya:
                continue
            khana = asas_hadaf + hadaf_s * 4
            # Straight alpha, and the maximum rather than a sum where two
            # glyphs overlap: adding coverage where a mark sits on its base
            # produces a bright seam exactly along the join, which is the one
            # place an Arabic reader is looking.
            sabiq = mistara[khana + 3]
            jadeed = taghtiya * shaffafiya // 255
            if jadeed <= sabiq:
                continue
            mistara[khana] = ahmar
            mistara[khana + 1] = akhdar
            mistara[khana + 2] = azraq
            mistara[khana + 3] = jadeed


def _khiyarat_takhtit():
    """The layout options this adapter asks for.

    Right-to-left base direction and Arabic language are forced rather than
    detected: this run reached here *because* it contains Arabic, and letting
    automatic detection decide would make a line that opens with a Latin
    character — a name, a number — lay out left to right with the Arabic
    trailing it.
    """
    from . import jisr as _wahdat_jisr

    khiyarat = _wahdat_jisr.Khiyarat()
    khiyarat.ittijah = _wahdat_jisr.ITTIJAH_YAMEEN
    khiyarat.lugha = _wahdat_jisr.LUGHA_ARABI
    khiyarat.muhadhaha = _wahdat_jisr.MUHADHAHA_BIDAYA
    khiyarat.tajawuz = _wahdat_jisr.TAJAWUZ_BALLAGH
    return khiyarat


def _min_makhzan(nass, hajm, lawn, ard_mutah):
    """A rasterized run, from the cache or freshly built.

    The cache is bounded and evicts wholesale rather than by age. That is
    deliberate and it is not laziness: a visual-novel scene's working set is
    the screenful of text in front of the player, and when it changes it
    changes completely. Tracking recency would cost a dictionary write per
    lookup to reproduce, at the moment of a scene change, exactly what clearing
    does for nothing.
    """
    miftah = (nass, hajm, lawn, ard_mutah)
    sath = _makhzan.get(miftah)
    if sath is not None:
        return sath
    if len(_makhzan) >= AQSA_MAKHZAN:
        _makhzan.clear()
    sath = _sath_min_takhtit(nass, hajm, lawn, ard_mutah)
    _makhzan[miftah] = sath
    return sath


def _hajm_hali():
    """The size a run is drawn at.

    The patch's override when it set one, otherwise the game's own default text
    size, otherwise Ren'Py's. Read at render time rather than at install time
    because a game that changes its text size in preferences changes this
    variable, and a takeover that cached it would ignore the setting.
    """
    if _idad is not None and _idad.hajm:
        return int(_idad.hajm)
    try:
        import renpy
        uslub = getattr(renpy.store, "style", None)
        if uslub is not None and hasattr(uslub, "default"):
            hajm = getattr(uslub.default, "size", None)
            if hajm:
                return int(hajm)
        gui = getattr(renpy.store, "gui", None)
        if gui is not None:
            hajm = getattr(gui, "text_size", None)
            if hajm:
                return int(hajm)
    except Exception:
        pass
    return 22


def _qaida_qabil():
    """Ren'Py's displayable base class, or ``object`` outside the engine.

    Resolved at class-definition time so that importing this module for a
    lint, a test, or a packaging step does not require Ren'Py to be present.
    The class is only ever *instantiated* from the takeover path, which cannot
    be reached outside the engine.
    """
    try:
        import renpy
        return renpy.Displayable
    except Exception:
        return object


class NassTaarib(_qaida_qabil()):
    """One shaped run, drawn by Taarib and embedded in Ren'Py's own text.

    Everything about it is deliberate:

    * It is **not** text to the engine. It is a rectangle with pixels in it, so
      it is not selectable, not searchable, and not visible to a screen reader.
      That is the cost the tier probe reports before anything is installed, and
      it is why this class only ever exists on the rung where the alternative
      is Arabic that does not join.
    * It measures itself from the width it is offered, so it participates in
      the surrounding line's wrapping the way any other embedded displayable
      does.
    * It holds no native handle. The atlas rectangles it used are gone by the
      time :meth:`render` returns; what survives is a surface, and the surface
      belongs to the cache rather than to the instance. An instance is
      therefore free to be created and dropped per frame, which is exactly what
      Ren'Py's text engine does with it.
    """

    def __init__(self, nass, lawn=None, hajm=None, **muamalat):
        try:
            super(NassTaarib, self).__init__(**muamalat)
        except TypeError:
            # `object.__init__` refuses keyword arguments; outside the engine
            # there is no base to pass them to and nothing to configure.
            super(NassTaarib, self).__init__()
        #: The run's text, in logical order.
        self.nass = nass
        #: The RGBA colour it is drawn in.
        self.lawn = lawn or (255, 255, 255, 255)
        #: The size, or None to read the game's own at render time.
        self.hajm = hajm

    def render(self, ard, irtifa, st, at):
        """Lays the run out, rasterizes it, and returns it as one blit."""
        import renpy

        hajm = self.hajm or _hajm_hali()
        mutah = float(ard) if ard and ard > 0 else 0.0
        try:
            sath = _min_makhzan(self.nass, hajm, self.lawn, mutah)
        except Exception as sabab:
            # A failure here is a failure on one frame, and the run is one line
            # of dialogue. Reporting it once and drawing nothing is survivable;
            # raising takes the whole game down mid-scene.
            _sajjil("a run could not be drawn and was skipped: %s" % (sabab,))
            return renpy.Render(0, 0)
        maqas = sath.get_size()
        natija = renpy.Render(maqas[0], maqas[1])
        natija.blit(sath, (0, 0))
        return natija

    def visit(self):
        """No child displayables: this one draws itself and owns nothing."""
        return []

    def __repr__(self):
        return "<NassTaarib %r>" % (self.nass[:32],)
