//! رن‌باي — Ren'Py, which is two engines wearing one name.
//!
//! Every other engine in this crate has one strategy and a set of
//! version-specific corrections. Ren'Py, like Godot, has **two strategies that
//! share nothing but a directory layout**, and picking between them is the first
//! thing this module does. [`crate::tabaqa`] states the general form of that
//! problem; [`taarib_muharrik::dalail::nusus::HADD_TASHKEEL_RENPY`] states this
//! engine's threshold and why the two sides are different adapters, and neither
//! is restated here.
//!
//! What this module adds is the part the threshold cannot answer on its own.
//!
//! ## A version number is not the evidence
//!
//! The release that matters bundles `HarfBuzz` and `FriBidi`. A build of that same
//! release compiled without them falls back to `renpy.text.ftfont`, which
//! positions one `FreeType` glyph per character with no joining and no
//! substitution — and reports the same `version_tuple`. So the probe here looks
//! for the *shipped shaping code by name*: the `renpy/text/hbfont` extension
//! module the `HarfBuzz` path lives in, and the `HarfBuzz` and `FriBidi` shared
//! libraries beside the interpreter. The version is evidence, weighted as
//! evidence; the presence of the shaping module is much stronger evidence, and a
//! disagreement between them is exactly the case [`crate::tabaqa::Hukm::tanaqud`]
//! exists to make visible.
//!
//! ## Delivery is additive, and that is not a compromise
//!
//! Ren'Py has a first-class localization mechanism, and Taarib uses it rather
//! than editing a single line of the game's own script:
//!
//! * `game/tl/arabic/*.rpy` holding `translate arabic <label>:` blocks for
//!   dialogue and `translate arabic strings:` blocks for interface text;
//! * one generated `.rpy` registering the font through `style`/`gui` overrides
//!   and a `config.font_replacement_map` entry;
//! * nothing else written, and nothing at all modified.
//!
//! Three properties follow, and they are the reason this is the correct path
//! rather than the convenient one. Uninstalling is deleting one directory. A
//! game update that rewrites `script.rpy` does not invalidate the translation,
//! because translation blocks are addressed by label and by translation
//! identifier rather than by file offset. And the engine itself decides when a
//! block applies, so a string the developer changed simply falls back to the
//! original instead of displaying a translation of text that is no longer there.
//!
//! ## What is read, and what is refused
//!
//! * **`.rpa` archives** — RPA-2.0, RPA-3.0 and RPA-3.2, read and written,
//!   including the index obfuscation: a zlib-compressed pickle keyed by a value
//!   in the header line. See [`Hawiya`].
//! * **`.rpy` source** — parsed for `label` statements, say statements and
//!   `_("...")` interface strings.
//! * **`.rpyc` compiled scripts** — read only where the source is absent, and
//!   read through an **inert** pickle machine: [`Silsila`] evaluates the
//!   container opcodes and refuses to import a module, call a function or
//!   construct an object. An object-construction opcode yields
//!   [`Qeema::Kaain`], a placeholder carrying the class name it was told and the
//!   state dictionary it was handed. Nothing in a game's archive can therefore
//!   run code inside this process, which is the property that makes reading an
//!   untrusted pickle acceptable at all.
//! * **`.rpyc` writing is refused outright.** The compiled form is the engine's
//!   own build product; Taarib never writes one, because the additive path makes
//!   it unnecessary and because emitting a byte-compiled AST for an engine whose
//!   AST classes change between releases is a way to produce a game that will
//!   not start.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::nusus::{HADD_TASHKEEL_RENPY, HadafNusus};

use crate::hifz::Hafiz;
use crate::khata::{KhataNusus, hajm_usize, tul_u64};
use crate::tabaqa::{Dalil, Mifhas, Rutba, SiyaqTabaqa};
use crate::tarkeeb::{Mutarjim, masar_bila_hala};

// ---------------------------------------------------------------------------
// Ceilings
//
// Every number a game's own file states is a number somebody else chose, and
// each one below decides how much memory this process is about to reserve.
// They are checked against the declared value before a byte is allocated.
// ---------------------------------------------------------------------------

/// The largest `.rpa` index this reader will expand.
///
/// A visual novel with two hundred thousand packed assets has an index of a few
/// megabytes once expanded. Sixty-four is generous by an order of magnitude and
/// still far below the point where a corrupt length field could exhaust a
/// 32-bit game process.
pub const AQSA_FAHRAS_RPA: u64 = 64 * 1024 * 1024;

/// The largest single member this reader will extract from an `.rpa`.
///
/// Ren'Py packs video and audio, so this has to be generous. Two gibibytes is
/// above any real member and below the point where the extraction cannot be
/// addressed on a 32-bit build.
pub const AQSA_UDW_RPA: u64 = 2 * 1024 * 1024 * 1024;

/// The largest compiled script slot this reader will expand.
pub const AQSA_RPYC: u64 = 256 * 1024 * 1024;

/// How deep a pickle stream may nest before the reader refuses.
///
/// A pickle is a stack machine, and a hostile one nests to whatever depth it
/// likes. Two hundred is far past anything Ren'Py's own AST reaches and shallow
/// enough that the reader cannot exhaust the native stack.
pub const AQSA_UMQ_SILSILA: u32 = 200;

/// How many objects one pickle stream may create.
///
/// The memo table is addressed by index, so a stream can otherwise ask for an
/// unbounded number of entries with a handful of bytes each.
pub const AQSA_KAINAT_SILSILA: usize = 4_000_000;

/// The locale directory Taarib generates under `game/tl/`.
///
/// `arabic` rather than `ar`, because Ren'Py's own convention is a language
/// *name*: `renpy.change_language("arabic")` and the `tl/<name>/` directory have
/// to agree, and every Ren'Py game that ships a translation spells it this way.
pub const LUGHA: &str = "arabic";

/// Where the generated translation lives, relative to the game root.
///
/// This and the two constants below are **relative paths resolved through
/// [`crate::tarkeeb::masar_bila_hala`]**, never joined literally. A depot that
/// ships `Game/` gets the patch inside the directory the engine already loads;
/// a literal join would create a second `game/` beside it holding everything
/// Taarib wrote and nothing the engine reads — an install that reports success
/// and changes nothing the player sees, and under Wine a directory that can
/// shadow the game's own content, because an exact match wins a case-insensitive
/// lookup.
pub const MUJALLAD_TARJAMA: &str = "game/tl/arabic";

/// The generated file that registers the font and the direction.
pub const MALAF_IDAD: &str = "game/tl/arabic/taarib_idad.rpy";

/// The generated file holding interface strings.
pub const MALAF_MUSTALAHAT: &str = "game/tl/arabic/taarib_mustalahat.rpy";

/// The marker every generated file opens with.
///
/// Uninstalling deletes files carrying it and refuses to delete anything else.
/// A translator who wrote their own `game/tl/arabic/dialogue.rpy` by hand must
/// not lose it because Taarib decided the directory was its own.
pub const ALAMAT_TAARIB: &str = "# taarib:generated";

// ---------------------------------------------------------------------------
// The generation split
// ---------------------------------------------------------------------------

/// Which of this module's two adapters applies.
///
/// The same shape as the Godot adapter's split and for the same reason: the two
/// paths share a package format and nothing else, so the choice is a type rather
/// than a flag. A boolean would eventually be read as "shape harder".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Masar {
    /// The engine ships `HarfBuzz` and `FriBidi` and uses them.
    ///
    /// Register the font, set the text direction and language properties,
    /// install the translation through `game/tl/arabic/`, and stop. Nothing in
    /// this module shapes a glyph on this path, and if anything ever does it is
    /// a bug: it would be a second implementation of something the engine
    /// already does correctly.
    Khadim,

    /// The engine cannot shape, so Taarib lays out and draws.
    ///
    /// The in-engine Python installs a text filter, lays out through the C ABI
    /// and returns a displayable that blits Taarib's own glyphs. Correct on
    /// screen, and no longer text to anything that inspects it.
    Istila,
}

impl Masar {
    /// The name for a log line and the capability report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Khadim => "Ren'Py engine shaping",
            Self::Istila => "Ren'Py glyph takeover",
        }
    }

    /// What a player should be told about this path before they install.
    ///
    /// The takeover's sentence names its cost plainly, because a player who
    /// installs a patch and later finds they cannot copy a line of dialogue out
    /// of the game has been surprised by something they should have been told.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Khadim => {
                "This game's Ren'Py ships HarfBuzz and FriBidi and shapes Arabic correctly on \
                 its own. Taarib supplies the font and the translation through Ren'Py's own \
                 localization mechanism and changes nothing about how text is drawn."
            },
            Self::Istila => {
                "This game's Ren'Py was built without a shaper, so Taarib lays the Arabic out \
                 and draws it. It will look correct, it will NOT be selectable or copyable \
                 inside the game, and it needs the Python 3 build of the engine: on a Python 2 \
                 build the adapter installs the font and the translation only and says so."
            },
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Khadim => {
                "نسخة رن‌باي في هذه اللعبة تحمل HarfBuzz وFriBidi وتشكّل العربية بنفسها. \
                 يزوّدها تعريب بالخطّ والترجمة عبر آلية الترجمة الخاصة بالمحرّك ولا يغيّر \
                 طريقة الرسم."
            },
            Self::Istila => {
                "بُنيت نسخة رن‌باي هذه بلا مشكّل، فيتولّى تعريب تخطيط العربية ورسمها. سيظهر \
                 النصّ صحيحًا ولن يكون قابلًا للتحديد أو النسخ داخل اللعبة."
            },
        }
    }

    /// Whether this path costs the user a capability they had before.
    #[must_use]
    pub const fn yukallif(self) -> bool {
        matches!(self, Self::Istila)
    }

    /// The rung this path corresponds to.
    ///
    /// The mapping is one-way on purpose. A rung is what the probe concluded
    /// about the engine; a path is what this adapter does about it, and
    /// [`Rutba::Tasheeh`] resolves here to [`Masar::Khadim`] because correcting
    /// direction and alignment on a shaping engine is still configuration.
    #[must_use]
    pub const fn min_rutba(rutba: Rutba) -> Self {
        match rutba {
            Rutba::Idad | Rutba::Tasheeh => Self::Khadim,
            Rutba::Istila => Self::Istila,
        }
    }
}

// ---------------------------------------------------------------------------
// The tier probe
// ---------------------------------------------------------------------------

/// File-name stems of the `HarfBuzz` shaping module Ren'Py compiles into
/// `renpy/text/`.
///
/// This is the single strongest thing that can be observed about a Ren'Py
/// build. The module exists only when the engine was compiled against `HarfBuzz`,
/// which is the exact fact the version number cannot state.
const ASMAA_HBFONT: [&str; 6] = [
    "hbfont.pyd",
    "hbfont.so",
    "hbfont.cpython-39-x86_64-linux-gnu.so",
    "hbfont.cpython-38-x86_64-linux-gnu.so",
    "hbfont.cpython-311-x86_64-linux-gnu.so",
    "hbfont.dylib",
];

/// The FreeType-only text module, which is what a build without `HarfBuzz` falls
/// back to.
///
/// Present in every build, including the shaping ones, so its presence proves
/// nothing on its own. It earns a line in the evidence only when `hbfont` is
/// absent beside it, because then it names what will actually draw the text.
const ASMAA_FTFONT: [&str; 4] = [
    "ftfont.pyd",
    "ftfont.so",
    "ftfont.cpython-39-x86_64-linux-gnu.so",
    "ftfont.dylib",
];

/// Substrings that identify a shipped `HarfBuzz` shared library.
///
/// Matched as a lowercase substring rather than as a whole name: the same
/// library ships as `libharfbuzz.so.0`, `libharfbuzz.0.dylib` and
/// `harfbuzz.dll` depending on the platform, and the version suffix moves.
const QITA_HARFBUZZ: [&str; 2] = ["libharfbuzz", "harfbuzz."];

/// Substrings that identify a shipped `FriBidi` shared library.
const QITA_FRIBIDI: [&str; 2] = ["libfribidi", "fribidi."];

/// Where a Ren'Py game keeps the interpreter and its shared libraries.
///
/// `lib/` is the classic layout and `lib/py3-<platform>-<arch>/` the one the
/// engine adopted when it grew a Python 3 branch. Both are listed because a
/// game shipped for one platform carries exactly one of them and a game shipped
/// for three carries several.
const JUDHUR_MAKTABAT: [&str; 2] = ["lib", "renpy"];

/// How many entries the probe will look at in any one directory.
///
/// A Ren'Py `game/` directory on a large visual novel holds tens of thousands
/// of loose files. Nothing here needs to see all of them.
const AQSA_MADAKHIL: usize = 4_096;

/// The Ren'Py tier probe.
///
/// Holds nothing: constructing one costs nothing and running it twice gives the
/// same answer, which is what lets the caller run all five probes on every game
/// without thinking about order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MifhasRenPy;

impl MifhasRenPy {
    /// A probe for Ren'Py.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

/// One directory's file names, lowercased, bounded, and never recursive.
///
/// A symbolic link reports as neither file nor directory here, which is
/// deliberate: a link inside a game directory is not followed, so a game cannot
/// make this probe describe a library that lives somewhere else on the machine.
fn asmaa_mujallad(masar: &Path) -> Vec<String> {
    let Ok(qaima) = fs::read_dir(masar) else {
        return Vec::new();
    };
    qaima
        .take(AQSA_MADAKHIL)
        .flatten()
        .filter_map(|madkhal| {
            let ism = madkhal.file_name().into_string().ok()?;
            madkhal
                .file_type()
                .is_ok_and(|naw| naw.is_file())
                .then(|| ism.to_lowercase())
        })
        .collect()
}

/// One directory's subdirectory names, bounded.
fn mujalladat_far(masar: &Path) -> Vec<String> {
    let Ok(qaima) = fs::read_dir(masar) else {
        return Vec::new();
    };
    qaima
        .take(AQSA_MADAKHIL)
        .flatten()
        .filter_map(|madkhal| {
            let ism = madkhal.file_name().into_string().ok()?;
            madkhal
                .file_type()
                .is_ok_and(|naw| naw.is_dir())
                .then_some(ism)
        })
        .collect()
}

/// Reads at most `hadd` bytes from the start of a file.
///
/// Returns `None` for a file that is missing, denied or unreadable, because in
/// a probe all three are the same thing: the absence of evidence.
fn iqra_muhaddad(masar: &Path, hadd: u64) -> Option<Vec<u8>> {
    let malaf = fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(hadd).read_to_end(&mut bayt).ok()?;
    Some(bayt)
}

/// The version triple `renpy/__init__.py` declares, when it declares a readable
/// one.
///
/// The fourth element of `version_tuple` is a build counter from version
/// control rather than a version component, so only the first three are taken.
fn isdar_renpy(jidhr: &Path) -> Option<(u16, u16, u16)> {
    let bayt = iqra_muhaddad(&masar_bila_hala(jidhr, "renpy/__init__.py"), 256 * 1024)?;
    let nass = String::from_utf8_lossy(&bayt);
    let satr = nass.lines().map(str::trim).find(|satr| {
        satr.starts_with("version_tuple") && satr.contains('=') && satr.contains('(')
    })?;
    let bidaya = satr.find('(')?;
    let baqi = satr.get(bidaya.checked_add(1)?..)?;
    let nihaya = baqi.find(')')?;
    let mut arqam = baqi
        .get(..nihaya)?
        .split(',')
        .filter_map(|juz| juz.trim().parse::<u16>().ok());
    Some((arqam.next()?, arqam.next()?, arqam.next().unwrap_or(0)))
}

/// Whether a version is at or above the release that bundles a shaper.
///
/// The threshold itself lives in Phase 5 and is not restated; this is only the
/// comparison against it.
const fn yablugh_hadd(kabir: u16, sagheer: u16) -> bool {
    let (hadd_kabir, hadd_sagheer) = HADD_TASHKEEL_RENPY;
    kabir > hadd_kabir || (kabir == hadd_kabir && sagheer >= hadd_sagheer)
}

/// Every directory the probe will look in for a shared library.
///
/// One level below `lib/` as well as `lib/` itself, because the per-platform
/// build directories are where the libraries actually sit, and two levels below
/// nothing: this is a probe, not a search.
fn amakin_maktabat(jidhr: &Path) -> Vec<PathBuf> {
    let mut amakin = vec![jidhr.to_path_buf()];
    for far in JUDHUR_MAKTABAT {
        let masar = masar_bila_hala(jidhr, far);
        if !masar.is_dir() {
            continue;
        }
        for ibn in mujalladat_far(&masar) {
            amakin.push(masar.join(ibn));
        }
        amakin.push(masar);
    }
    amakin
}

/// Where a shared library matching one of `qita` was found, if anywhere.
fn ibhath_maktaba(amakin: &[PathBuf], qita: &[&str]) -> Option<(PathBuf, String)> {
    for makan in amakin {
        for ism in asmaa_mujallad(makan) {
            if qita.iter().any(|juz| ism.contains(juz)) {
                return Some((makan.clone(), ism));
            }
        }
    }
    None
}

/// A path rendered relative to the game root, with forward slashes, for the
/// `masdar` of a piece of evidence.
fn mawqi_nisbi(jidhr: &Path, masar: &Path) -> String {
    masar
        .strip_prefix(jidhr)
        .unwrap_or(masar)
        .to_string_lossy()
        .replace('\\', "/")
}

impl Mifhas for MifhasRenPy {
    fn hadaf(&self) -> HadafNusus {
        HadafNusus::RenPy
    }

    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool {
        // The same gate Phase 5 uses, and for the same reason: a `game/`
        // directory on its own belongs to half the engines in existence, so the
        // gate is a `renpy/` package or a compiled script, each of which belongs
        // to this engine and to nothing else.
        //
        // Case-folded because the gate has to answer for the depot as shipped:
        // Ren'Py writes `game/` and `renpy/` in lower case, a repacker does not
        // always keep them, and Wine hides the difference from the player while
        // a literal join on Linux turns it into "this is not a Ren'Py game".
        masar_bila_hala(siyaq.jidhr, "renpy").is_dir()
            || masar_bila_hala(siyaq.jidhr, "game/script.rpyc").is_file()
            || masar_bila_hala(siyaq.jidhr, "renpy/__init__.py").is_file()
    }

    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus> {
        let jidhr = siyaq.jidhr;
        let mut adilla: Vec<Dalil> = Vec::new();

        // 1. The version, which is suggestive and never conclusive. A build of
        //    a shaping release compiled without the optional libraries reports
        //    exactly the same triple as one compiled with them.
        match isdar_renpy(jidhr) {
            Some((kabir, sagheer, tasheeh)) => {
                let (hadd_kabir, hadd_sagheer) = HADD_TASHKEEL_RENPY;
                let (yushir, wujid) = if yablugh_hadd(kabir, sagheer) {
                    (
                        Some(Rutba::Idad),
                        format!(
                            "version_tuple ({kabir}, {sagheer}, {tasheeh}), at or above \
                             {hadd_kabir}.{hadd_sagheer}, so this release can carry a shaper"
                        ),
                    )
                } else {
                    (
                        Some(Rutba::Istila),
                        format!(
                            "version_tuple ({kabir}, {sagheer}, {tasheeh}), below \
                             {hadd_kabir}.{hadd_sagheer}, so no release-level shaper exists"
                        ),
                    )
                };
                adilla.push(Dalil::jadeed(
                    "renpy/__init__.py",
                    wujid,
                    yushir,
                    WAZN_ISDAR,
                ));
            },
            None => adilla.push(Dalil::siyaq(
                "renpy/__init__.py",
                "absent or carrying no readable version_tuple, so the release says nothing \
                 here and the shipped modules have to answer on their own",
            )),
        }

        // 2. The shaping module by name, which is the strongest observation
        //    available anywhere in this family: it exists only when the engine
        //    was compiled against HarfBuzz.
        let mujallad_nass = masar_bila_hala(jidhr, "renpy/text");
        let asmaa_nass = asmaa_mujallad(&mujallad_nass);
        if asmaa_nass.is_empty() {
            adilla.push(Dalil::siyaq(
                "renpy/text",
                "not readable, so which text module this build compiled could not be seen",
            ));
        } else if let Some(ism) = asmaa_nass
            .iter()
            .find(|ism| ASMAA_HBFONT.contains(&ism.as_str()))
        {
            adilla.push(Dalil::jadeed(
                "renpy/text",
                format!(
                    "{ism} is present, which is the HarfBuzz text module; this build shapes \
                     and reorders Arabic itself"
                ),
                Some(Rutba::Idad),
                WAZN_HBFONT,
            ));
        } else {
            let badeel = asmaa_nass
                .iter()
                .find(|ism| ASMAA_FTFONT.contains(&ism.as_str()))
                .map_or_else(
                    || "and no FreeType module either".to_owned(),
                    |ism| format!("and {ism} is, so FreeType alone will draw the text"),
                );
            adilla.push(Dalil::jadeed(
                "renpy/text",
                format!(
                    "no HarfBuzz text module is present {badeel}: this build positions one \
                     glyph per character with no joining"
                ),
                Some(Rutba::Istila),
                WAZN_LA_HBFONT,
            ));
        }

        // 3. The shared libraries, which corroborate the module and survive the
        //    case where `renpy/text` was packed into an archive.
        let amakin = amakin_maktabat(jidhr);
        match ibhath_maktaba(&amakin, &QITA_HARFBUZZ) {
            Some((makan, ism)) => adilla.push(Dalil::jadeed(
                mawqi_nisbi(jidhr, &makan),
                format!("{ism} ships with the game"),
                Some(Rutba::Idad),
                WAZN_MAKTABA_HB,
            )),
            None => adilla.push(Dalil::jadeed(
                "lib",
                "no HarfBuzz shared library ships with the game",
                Some(Rutba::Istila),
                WAZN_LA_MAKTABA_HB,
            )),
        }
        match ibhath_maktaba(&amakin, &QITA_FRIBIDI) {
            Some((makan, ism)) => adilla.push(Dalil::jadeed(
                mawqi_nisbi(jidhr, &makan),
                format!("{ism} ships with the game, so bidirectional reordering is available"),
                Some(Rutba::Idad),
                WAZN_MAKTABA_BIDI,
            )),
            None => adilla.push(Dalil::jadeed(
                "lib",
                "no FriBidi shared library ships with the game",
                Some(Rutba::Istila),
                WAZN_LA_MAKTABA_BIDI,
            )),
        }

        // 4. Context. None of this points at a rung; all of it is what a
        //    maintainer needs when a verdict later turns out wrong.
        let tl = masar_bila_hala(jidhr, "game/tl");
        if tl.is_dir() {
            let mut lughat = mujalladat_far(&tl);
            lughat.sort();
            lughat.retain(|ism| ism.as_str() != "None");
            if !lughat.is_empty() {
                adilla.push(Dalil::siyaq(
                    "game/tl",
                    format!(
                        "the game already ships translations for {}",
                        lughat.join(", ")
                    ),
                ));
            }
        }
        if let Some(tanfidhi) = siyaq.tanfidhi {
            adilla.push(Dalil::siyaq("executable", mawqi_nisbi(jidhr, tanfidhi)));
        }
        adilla.push(Dalil::siyaq("engine family", siyaq.aila.ism()));

        Ok(adilla)
    }
}

/// Weight of the release version, in either direction.
///
/// Suggestive and no more, which is the whole point of this probe: a release
/// at or above the threshold *can* carry a shaper and a build of it can have
/// been compiled without one, so the number alone never settles anything. It is
/// deliberately below every module and library weight, so that a version and
/// the files it disagrees with resolve in favour of the files.
const WAZN_ISDAR: u8 = 40;

/// Weight of finding the `HarfBuzz` text module by name.
const WAZN_HBFONT: u8 = 70;

/// Weight of that module being absent from a readable `renpy/text`.
const WAZN_LA_HBFONT: u8 = 60;

/// Weight of a shipped `HarfBuzz` shared library.
const WAZN_MAKTABA_HB: u8 = 45;

/// Weight of no `HarfBuzz` shared library shipping anywhere the probe looked.
const WAZN_LA_MAKTABA_HB: u8 = 35;

/// Weight of a shipped `FriBidi` shared library.
const WAZN_MAKTABA_BIDI: u8 = 30;

/// Weight of no `FriBidi` shared library shipping anywhere the probe looked.
const WAZN_LA_MAKTABA_BIDI: u8 = 25;

// ---------------------------------------------------------------------------
// The pickle machine
//
// Ren'Py stores its archive index and its compiled scripts as Python pickles,
// so this crate has to read pickles produced by somebody else's game. A pickle
// is a stack machine whose instruction set includes "import this module",
// "call this callable" and "run this object's __setstate__", and a reader that
// implemented those would be an arbitrary-code-execution hole with a game
// directory as its input.
//
// So this machine is inert. GLOBAL and STACK_GLOBAL push a *name* and import
// nothing. REDUCE, NEWOBJ, NEWOBJ_EX, OBJ and INST pop their arguments and push
// an opaque Kaain carrying that name; nothing is called. BUILD attaches a state
// value to a Kaain; no __setstate__ runs. PERSID and the EXT opcodes are refused
// outright, because both resolve a value through a table this process would
// have to consult and neither has an inert reading.
//
// The result is a value tree that describes what the pickle would have built,
// which is exactly what a translation extractor needs and strictly less than
// what an unpickler would give it.
// ---------------------------------------------------------------------------

/// A handle into a [`Silsila`]'s arena.
///
/// The arena exists because a pickle's memo makes back-references by index, and
/// a reader that resolved them by copying would duplicate every shared object
/// and loop forever on a cyclic one — Ren'Py's AST has parent pointers, so
/// cyclic is the ordinary case rather than the exotic one. Resolving a
/// back-reference to a handle preserves identity and terminates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Muashir(u32);

impl Muashir {
    /// The handle as an index, for a caller keeping its own side table.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        self.0
    }
}

/// One value a pickle stream describes.
///
/// Deliberately not a Python object model: there is no callable, no class and
/// no module here, because none of those can be represented without resolving a
/// name into this process.
#[derive(Debug, Clone)]
pub enum Uqda {
    /// `None`.
    Faragh,
    /// `True` or `False`.
    Mantiq(bool),
    /// An integer that fits.
    Sahih(i64),
    /// An integer that does not, kept as sign and little-endian magnitude.
    Kabir {
        /// Whether the value is negative.
        salib: bool,
        /// The magnitude, least significant byte first.
        adad: Vec<u8>,
    },
    /// A float.
    Ashari(f64),
    /// Text.
    Nass(String),
    /// Bytes, which are not text and are never decoded as text.
    Bayt(Vec<u8>),
    /// A global the stream named — `renpy.ast.Say` — resolved to nothing.
    Ism(String),
    /// A list.
    Qaima(Vec<Muashir>),
    /// A tuple.
    Thulathi(Vec<Muashir>),
    /// A mapping, in the order the stream set its items.
    Kharita(Vec<(Muashir, Muashir)>),
    /// A set or frozenset.
    Majmua(Vec<Muashir>),
    /// An object the stream would have constructed, constructed by nothing.
    Kaain {
        /// The global name the stream gave for the class.
        ism: String,
        /// The arguments it would have been called with.
        muamalat: Vec<Muashir>,
        /// The state a later `BUILD` attached, when one did.
        hala: Option<Muashir>,
    },
}

/// A decoded pickle stream: an arena of values and the handle of its result.
#[derive(Debug, Clone)]
pub struct Silsila {
    uqad: Vec<Uqda>,
    jidhr: Muashir,
}

impl Silsila {
    /// The value the stream produced.
    #[must_use]
    pub const fn jidhr(&self) -> Muashir {
        self.jidhr
    }

    /// One value by handle, or [`None`] for a handle from another stream.
    #[must_use]
    pub fn uqda(&self, muashir: Muashir) -> Option<&Uqda> {
        self.uqad.get(usize::try_from(muashir.0).ok()?)
    }

    /// How many values the stream created.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.uqad.len()
    }

    /// The text of a value, when it is text.
    #[must_use]
    pub fn nass(&self, muashir: Muashir) -> Option<&str> {
        match self.uqda(muashir)? {
            Uqda::Nass(nass) => Some(nass.as_str()),
            _ => None,
        }
    }

    /// The integer of a value, when it is one that fits.
    #[must_use]
    pub fn sahih(&self, muashir: Muashir) -> Option<i64> {
        match self.uqda(muashir)? {
            Uqda::Sahih(qeema) => Some(*qeema),
            _ => None,
        }
    }

    /// The members of a list or tuple.
    #[must_use]
    pub fn anasir(&self, muashir: Muashir) -> Option<&[Muashir]> {
        match self.uqda(muashir)? {
            Uqda::Qaima(anasir) | Uqda::Thulathi(anasir) | Uqda::Majmua(anasir) => {
                Some(anasir.as_slice())
            },
            _ => None,
        }
    }

    /// The value a mapping holds for a text key, comparing keys as text.
    ///
    /// Text keys are what every structure this module reads uses — a pickled
    /// `__dict__` and Ren'Py's archive index both — and comparing on the decoded
    /// text rather than on the handle is what makes a key that was memoized once
    /// and referenced later resolve the same way as one written out twice.
    #[must_use]
    pub fn min_kharita(&self, muashir: Muashir, miftah: &str) -> Option<Muashir> {
        let Uqda::Kharita(azwaj) = self.uqda(muashir)? else {
            return None;
        };
        azwaj
            .iter()
            .find(|(ism, _)| self.nass(*ism) == Some(miftah))
            .map(|(_, qeema)| *qeema)
    }

    /// The class name and state of an object placeholder.
    #[must_use]
    pub fn kaain(&self, muashir: Muashir) -> Option<(&str, Option<Muashir>)> {
        match self.uqda(muashir)? {
            Uqda::Kaain { ism, hala, .. } => Some((ism.as_str(), *hala)),
            _ => None,
        }
    }

    /// Every handle in the arena, in creation order.
    ///
    /// Creation order is emission order, which is what lets an extractor walk a
    /// whole stream once without recursing into a structure that may be cyclic.
    pub fn kul(&self) -> impl Iterator<Item = (Muashir, &Uqda)> {
        self.uqad
            .iter()
            .enumerate()
            .filter_map(|(fahras, uqda)| Some((Muashir(u32::try_from(fahras).ok()?), uqda)))
    }
}

/// A bounds-checked cursor over a byte stream.
///
/// Every read goes through here so that a truncated stream produces one named
/// refusal rather than a different panic per opcode.
#[derive(Debug)]
struct Qari<'a> {
    bayt: &'a [u8],
    mawqi: usize,
}

impl<'a> Qari<'a> {
    /// A cursor at the start of a stream.
    const fn jadeed(bayt: &'a [u8]) -> Self {
        Self { bayt, mawqi: 0 }
    }

    /// How many bytes are left.
    fn baqi(&self) -> u64 {
        tul_u64(self.bayt.len().saturating_sub(self.mawqi))
    }

    /// The refusal a short read produces.
    fn qaseer(&self, haql: &'static str, matlub: u64) -> KhataNusus {
        KhataNusus::MalafQaseer {
            haql,
            tul: self.baqi(),
            matlub,
        }
    }

    /// One byte.
    fn wahid(&mut self, haql: &'static str) -> Result<u8, KhataNusus> {
        let qeema = *self
            .bayt
            .get(self.mawqi)
            .ok_or_else(|| self.qaseer(haql, 1))?;
        self.mawqi = self.mawqi.saturating_add(1);
        Ok(qeema)
    }

    /// A run of bytes.
    ///
    /// The backing slice is copied out of `self` before it is indexed. That is
    /// not style: the return borrows the stream and not the cursor, so a caller
    /// can hold a decoded run while the cursor moves on, which is what every
    /// opcode in this module does.
    fn nitaq(&mut self, haql: &'static str, tul: usize) -> Result<&'a [u8], KhataNusus> {
        let masdar: &'a [u8] = self.bayt;
        let Some(nihaya) = self.mawqi.checked_add(tul) else {
            return Err(self.qaseer(haql, tul_u64(tul)));
        };
        let Some(juz) = masdar.get(self.mawqi..nihaya) else {
            return Err(self.qaseer(haql, tul_u64(tul)));
        };
        self.mawqi = nihaya;
        Ok(juz)
    }

    /// A little-endian unsigned integer of `adad` bytes, widened to `u64`.
    fn adad_le(&mut self, haql: &'static str, adad: usize) -> Result<u64, KhataNusus> {
        let juz = self.nitaq(haql, adad)?;
        Ok(juz.iter().enumerate().fold(0_u64, |majmu, (rutba, bayt)| {
            let izaha = u32::try_from(rutba).unwrap_or(u32::MAX).saturating_mul(8);
            majmu | (u64::from(*bayt) << izaha.min(56))
        }))
    }

    /// A newline-terminated line, without its terminator.
    fn satr(&mut self, haql: &'static str) -> Result<&'a [u8], KhataNusus> {
        let masdar: &'a [u8] = self.bayt;
        let Some(baqi) = masdar.get(self.mawqi..) else {
            return Err(self.qaseer(haql, 1));
        };
        let Some(tul) = baqi.iter().position(|bayt| *bayt == b'\n') else {
            return Err(KhataNusus::MalafQaseer {
                haql,
                tul: tul_u64(baqi.len()),
                matlub: tul_u64(baqi.len().saturating_add(1)),
            });
        };
        let Some(satr) = baqi.get(..tul) else {
            return Err(self.qaseer(haql, tul_u64(tul)));
        };
        self.mawqi = self.mawqi.saturating_add(tul).saturating_add(1);
        Ok(satr)
    }
}

/// A size a stream declared, checked against a ceiling before it is reserved.
fn hajm_maqbul(haql: &'static str, qeema: u64, saqf: u64) -> Result<usize, KhataNusus> {
    if qeema > saqf {
        return Err(KhataNusus::HajmMufrit { haql, qeema, saqf });
    }
    hajm_usize(qeema).ok_or(KhataNusus::HajmMufrit { haql, qeema, saqf })
}

/// The refusal an opcode this machine will not run produces.
fn ramz_marfud(ramz: u8, sabab: &str) -> KhataNusus {
    KhataNusus::BunyaGhayrMutawaqqaa {
        malaf: "a Ren'Py pickle stream".to_owned(),
        haql: format!("opcode {ramz:#04x} ({sabab})"),
    }
}

/// The refusal a stream that violates the stack machine's own rules produces.
fn silsila_talifa(sabab: &str) -> KhataNusus {
    KhataNusus::BunyaGhayrMutawaqqaa {
        malaf: "a Ren'Py pickle stream".to_owned(),
        haql: sabab.to_owned(),
    }
}

/// The machine's mutable state, kept together so the opcode loop stays flat.
#[derive(Debug)]
struct AlatSilsila {
    uqad: Vec<Uqda>,
    kudsa: Vec<Muashir>,
    alamat: Vec<usize>,
    dhakira: BTreeMap<u64, Muashir>,
}

impl AlatSilsila {
    /// An empty machine.
    const fn jadeed() -> Self {
        Self {
            uqad: Vec::new(),
            kudsa: Vec::new(),
            alamat: Vec::new(),
            dhakira: BTreeMap::new(),
        }
    }

    /// Interns a value and returns its handle.
    fn adhif(&mut self, uqda: Uqda) -> Result<Muashir, KhataNusus> {
        if self.uqad.len() >= AQSA_KAINAT_SILSILA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the objects a pickle stream creates",
                qeema: tul_u64(self.uqad.len().saturating_add(1)),
                saqf: tul_u64(AQSA_KAINAT_SILSILA),
            });
        }
        let raqm = u32::try_from(self.uqad.len()).map_err(|_| KhataNusus::HajmMufrit {
            haql: "the objects a pickle stream creates",
            qeema: tul_u64(self.uqad.len()),
            saqf: u64::from(u32::MAX),
        })?;
        self.uqad.push(uqda);
        Ok(Muashir(raqm))
    }

    /// Interns a value and pushes it.
    fn idfa(&mut self, uqda: Uqda) -> Result<(), KhataNusus> {
        let muashir = self.adhif(uqda)?;
        self.kudsa.push(muashir);
        Ok(())
    }

    /// Pops one value.
    fn isjab(&mut self) -> Result<Muashir, KhataNusus> {
        self.kudsa
            .pop()
            .ok_or_else(|| silsila_talifa("an opcode popped an empty stack"))
    }

    /// The topmost value without popping it.
    fn qimma(&self) -> Result<Muashir, KhataNusus> {
        self.kudsa
            .last()
            .copied()
            .ok_or_else(|| silsila_talifa("an opcode read an empty stack"))
    }

    /// Everything pushed since the last mark, with the mark consumed.
    fn min_alama(&mut self) -> Result<Vec<Muashir>, KhataNusus> {
        let alama = self
            .alamat
            .pop()
            .ok_or_else(|| silsila_talifa("an opcode closed a mark that was never opened"))?;
        if alama > self.kudsa.len() {
            return Err(silsila_talifa(
                "a mark points above the stack it was set on",
            ));
        }
        Ok(self.kudsa.split_off(alama))
    }

    /// Replaces a container's contents in place, for `APPEND(S)` and
    /// `SETITEM(S)`, which mutate an object that is already memoized.
    fn haddith(&mut self, muashir: Muashir, tabdeel: Uqda) -> Result<(), KhataNusus> {
        let khana = usize::try_from(muashir.0)
            .ok()
            .and_then(|fahras| self.uqad.get_mut(fahras))
            .ok_or_else(|| silsila_talifa("an opcode mutated a value that does not exist"))?;
        *khana = tabdeel;
        Ok(())
    }
}

/// Decodes a pickle stream without importing, calling or constructing anything.
///
/// The opcodes that would resolve a name into this process are handled as
/// follows, and the handling is the security property of this whole module:
///
/// * `GLOBAL` and `STACK_GLOBAL` push [`Uqda::Ism`], a name, and import nothing.
/// * `REDUCE`, `NEWOBJ`, `NEWOBJ_EX`, `OBJ` and `INST` push [`Uqda::Kaain`]
///   carrying that name and the arguments; nothing is called.
/// * `BUILD` records the state on the placeholder; no `__setstate__` runs.
/// * `PERSID`, `BINPERSID` and the three `EXT` opcodes are refused, because each
///   resolves a value through a table this process would have to consult and
///   none of them has an inert reading.
///
/// # Errors
///
/// [`KhataNusus::MalafQaseer`] when the stream ends inside an opcode,
/// [`KhataNusus::HajmMufrit`] when it declares a length or an object count above
/// this build's ceiling, [`KhataNusus::NassGhayrSalih`] when a text opcode's
/// payload is not valid UTF-8, and [`KhataNusus::BunyaGhayrMutawaqqaa`] for an
/// opcode this machine refuses or a stack the stream corrupted.
pub fn iqra_silsila(bayt: &[u8]) -> Result<Silsila, KhataNusus> {
    let mut qari = Qari::jadeed(bayt);
    let mut ala = AlatSilsila::jadeed();
    let mut umq: u32 = 0;

    loop {
        let ramz = qari.wahid("a pickle opcode")?;
        match ramz {
            // -- framing ---------------------------------------------------
            0x80 => {
                let isdar = qari.wahid("the pickle protocol")?;
                if isdar > 5 {
                    return Err(KhataNusus::IsdarGhayrMadum {
                        sigha: "a Ren'Py pickle stream",
                        wujid: u32::from(isdar),
                        adna: 0,
                        aqsa: 5,
                    });
                }
            },
            0x95 => {
                // FRAME is an optimisation hint whose payload is a length; the
                // bytes it frames are read by the opcodes inside it.
                let _ = qari.adad_le("a pickle frame length", 8)?;
            },
            b'.' => break,

            // -- marks and the stack ---------------------------------------
            b'(' => {
                umq = umq.saturating_add(1);
                if umq > AQSA_UMQ_SILSILA {
                    return Err(KhataNusus::HajmMufrit {
                        haql: "pickle nesting depth",
                        qeema: u64::from(umq),
                        saqf: u64::from(AQSA_UMQ_SILSILA),
                    });
                }
                ala.alamat.push(ala.kudsa.len());
            },
            b'0' => {
                let _ = ala.isjab()?;
            },
            b'1' => {
                umq = umq.saturating_sub(1);
                let _ = ala.min_alama()?;
            },
            b'2' => {
                let qimma = ala.qimma()?;
                ala.kudsa.push(qimma);
            },

            // -- atoms -----------------------------------------------------
            b'N' => ala.idfa(Uqda::Faragh)?,
            0x88 => ala.idfa(Uqda::Mantiq(true))?,
            0x89 => ala.idfa(Uqda::Mantiq(false))?,
            b'J' => {
                let khaam = qari.adad_le("a 32-bit pickle integer", 4)?;
                let masfufa = u32::try_from(khaam).unwrap_or(0).to_le_bytes();
                ala.idfa(Uqda::Sahih(i64::from(i32::from_le_bytes(masfufa))))?;
            },
            b'K' => {
                let khaam = qari.wahid("an 8-bit pickle integer")?;
                ala.idfa(Uqda::Sahih(i64::from(khaam)))?;
            },
            b'M' => {
                let khaam = qari.adad_le("a 16-bit pickle integer", 2)?;
                ala.idfa(Uqda::Sahih(i64::try_from(khaam).unwrap_or(0)))?;
            },
            b'I' => {
                let satr = qari.satr("a text pickle integer")?;
                // Protocol 0 spells the booleans as integers, and Ren'Py's own
                // older archives do exactly that.
                let uqda = match satr {
                    b"01" => Uqda::Mantiq(true),
                    b"00" => Uqda::Mantiq(false),
                    _ => Uqda::Sahih(nass_ascii(satr)?.trim().parse::<i64>().map_err(|_| {
                        silsila_talifa("a text integer opcode carried something that is not one")
                    })?),
                };
                ala.idfa(uqda)?;
            },
            b'L' => {
                let satr = qari.satr("a text pickle long")?;
                let munaqqa = nass_ascii(satr)?;
                let qeema = munaqqa.trim().trim_end_matches('L');
                ala.idfa(Uqda::Sahih(qeema.parse::<i64>().unwrap_or(0)))?;
            },
            0x8A | 0x8B => {
                let adad = if ramz == 0x8A {
                    u64::from(qari.wahid("a pickle long length")?)
                } else {
                    qari.adad_le("a pickle long length", 4)?
                };
                let tul = hajm_maqbul("a pickle long", adad, 4096)?;
                let juz = qari.nitaq("a pickle long", tul)?;
                ala.idfa(kabir_min_bayt(juz))?;
            },
            b'G' => {
                let juz = qari.nitaq("a pickle float", 8)?;
                let masfufa: [u8; 8] = juz
                    .try_into()
                    .map_err(|_| silsila_talifa("a binary float opcode is not eight bytes wide"))?;
                ala.idfa(Uqda::Ashari(f64::from_be_bytes(masfufa)))?;
            },
            b'F' => {
                let satr = qari.satr("a text pickle float")?;
                ala.idfa(Uqda::Ashari(
                    nass_ascii(satr)?.trim().parse::<f64>().unwrap_or(0.0),
                ))?;
            },

            // -- text and bytes --------------------------------------------
            b'X' | 0x8C | 0x8D => {
                let adad = match ramz {
                    0x8C => u64::from(qari.wahid("a short unicode length")?),
                    b'X' => qari.adad_le("a unicode length", 4)?,
                    _ => qari.adad_le("a long unicode length", 8)?,
                };
                let tul = hajm_maqbul("a pickle string", adad, AQSA_UDW_RPA)?;
                let juz = qari.nitaq("a pickle string", tul)?;
                ala.idfa(Uqda::Nass(nass_utf8(juz, qari.mawqi)?))?;
            },
            b'T' | b'U' => {
                // Protocol 0 and 1 `str`, which is Python 2 bytes. Kept as
                // bytes when it is not valid UTF-8: an archive index written by
                // a Python 2 Ren'Py holds path names that are, and a packed
                // member's prefix that is not.
                let adad = if ramz == b'U' {
                    u64::from(qari.wahid("a short string length")?)
                } else {
                    qari.adad_le("a string length", 4)?
                };
                let tul = hajm_maqbul("a pickle string", adad, AQSA_UDW_RPA)?;
                let juz = qari.nitaq("a pickle string", tul)?;
                ala.idfa(match core::str::from_utf8(juz) {
                    Ok(nass) => Uqda::Nass(nass.to_owned()),
                    Err(_) => Uqda::Bayt(juz.to_vec()),
                })?;
            },
            b'B' | b'C' | 0x8E => {
                let adad = match ramz {
                    b'C' => u64::from(qari.wahid("a short bytes length")?),
                    b'B' => qari.adad_le("a bytes length", 4)?,
                    _ => qari.adad_le("a long bytes length", 8)?,
                };
                let tul = hajm_maqbul("a pickle bytes object", adad, AQSA_UDW_RPA)?;
                let juz = qari.nitaq("a pickle bytes object", tul)?;
                ala.idfa(Uqda::Bayt(juz.to_vec()))?;
            },
            b'S' | b'V' => {
                let satr = qari.satr("a text pickle string")?;
                ala.idfa(Uqda::Nass(nass_muqtabas(satr)))?;
            },

            // -- containers ------------------------------------------------
            b']' => ala.idfa(Uqda::Qaima(Vec::new()))?,
            b')' => ala.idfa(Uqda::Thulathi(Vec::new()))?,
            b'}' => ala.idfa(Uqda::Kharita(Vec::new()))?,
            0x8F => ala.idfa(Uqda::Majmua(Vec::new()))?,
            b'l' => {
                umq = umq.saturating_sub(1);
                let anasir = ala.min_alama()?;
                ala.idfa(Uqda::Qaima(anasir))?;
            },
            b't' => {
                umq = umq.saturating_sub(1);
                let anasir = ala.min_alama()?;
                ala.idfa(Uqda::Thulathi(anasir))?;
            },
            0x85..=0x87 => {
                let adad = usize::from(ramz.saturating_sub(0x84));
                let mut anasir = Vec::with_capacity(adad);
                for _ in 0..adad {
                    anasir.push(ala.isjab()?);
                }
                anasir.reverse();
                ala.idfa(Uqda::Thulathi(anasir))?;
            },
            b'd' => {
                umq = umq.saturating_sub(1);
                let anasir = ala.min_alama()?;
                ala.idfa(Uqda::Kharita(azwaj_min_qaima(&anasir)?))?;
            },
            0x91 => {
                umq = umq.saturating_sub(1);
                let anasir = ala.min_alama()?;
                ala.idfa(Uqda::Majmua(anasir))?;
            },

            _ => {
                idara_silsila(ramz, &mut qari, &mut ala, &mut umq)?;
            },
        }
    }

    let jidhr = ala.isjab()?;
    Ok(Silsila {
        uqad: ala.uqad,
        jidhr,
    })
}

/// The opcodes that touch the memo, mutate a container, or name a class.
///
/// Split out of [`iqra_silsila`] only so neither half is a thousand-line match;
/// the two are one machine and share every invariant.
fn idara_silsila(
    ramz: u8,
    qari: &mut Qari<'_>,
    ala: &mut AlatSilsila,
    umq: &mut u32,
) -> Result<(), KhataNusus> {
    match ramz {
        // -- the memo ------------------------------------------------------
        b'p' | b'q' | b'r' | 0x94 => {
            let raqm = match ramz {
                b'q' => u64::from(qari.wahid("a memo index")?),
                b'r' => qari.adad_le("a memo index", 4)?,
                b'p' => nass_ascii(qari.satr("a memo index")?)?
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| silsila_talifa("a text memo index is not a number"))?,
                _ => tul_u64(ala.dhakira.len()),
            };
            let qimma = ala.qimma()?;
            let _ = ala.dhakira.insert(raqm, qimma);
        },
        b'g' | b'h' | b'j' => {
            let raqm = match ramz {
                b'h' => u64::from(qari.wahid("a memo index")?),
                b'j' => qari.adad_le("a memo index", 4)?,
                _ => nass_ascii(qari.satr("a memo index")?)?
                    .trim()
                    .parse::<u64>()
                    .map_err(|_| silsila_talifa("a text memo index is not a number"))?,
            };
            // The back-reference resolves to the same handle rather than to a
            // copy. That is what preserves identity across the stream and what
            // lets a cyclic structure terminate.
            let muashir =
                ala.dhakira.get(&raqm).copied().ok_or_else(|| {
                    silsila_talifa("a memo reference names an index never stored")
                })?;
            ala.kudsa.push(muashir);
        },

        // -- mutation ------------------------------------------------------
        b'a' | b'e' => {
            let mudaf = if ramz == b'a' {
                vec![ala.isjab()?]
            } else {
                *umq = umq.saturating_sub(1);
                ala.min_alama()?
            };
            let hadaf = ala.qimma()?;
            let Some(Uqda::Qaima(hali)) = ala.uqad.get(usize::try_from(hadaf.0).unwrap_or(0))
            else {
                return Err(silsila_talifa(
                    "an append opcode targeted something that is not a list",
                ));
            };
            let mut jadeed = hali.clone();
            jadeed.extend_from_slice(&mudaf);
            ala.haddith(hadaf, Uqda::Qaima(jadeed))?;
        },
        0x90 => {
            *umq = umq.saturating_sub(1);
            let mudaf = ala.min_alama()?;
            let hadaf = ala.qimma()?;
            let Some(Uqda::Majmua(hali)) = ala.uqad.get(usize::try_from(hadaf.0).unwrap_or(0))
            else {
                return Err(silsila_talifa(
                    "an additems opcode targeted something that is not a set",
                ));
            };
            let mut jadeed = hali.clone();
            jadeed.extend_from_slice(&mudaf);
            ala.haddith(hadaf, Uqda::Majmua(jadeed))?;
        },
        b's' | b'u' => {
            let mudaf = if ramz == b's' {
                let qeema = ala.isjab()?;
                let miftah = ala.isjab()?;
                vec![(miftah, qeema)]
            } else {
                *umq = umq.saturating_sub(1);
                let anasir = ala.min_alama()?;
                azwaj_min_qaima(&anasir)?
            };
            let hadaf = ala.qimma()?;
            let Some(Uqda::Kharita(hali)) = ala.uqad.get(usize::try_from(hadaf.0).unwrap_or(0))
            else {
                return Err(silsila_talifa(
                    "a setitem opcode targeted something that is not a mapping",
                ));
            };
            let mut jadeed = hali.clone();
            jadeed.extend_from_slice(&mudaf);
            ala.haddith(hadaf, Uqda::Kharita(jadeed))?;
        },

        // -- names, which stay names ---------------------------------------
        b'c' => {
            let wahda = nass_ascii(qari.satr("a global's module")?)?;
            let ism = nass_ascii(qari.satr("a global's name")?)?;
            ala.idfa(Uqda::Ism(format!("{wahda}.{ism}")))?;
        },
        0x93 => {
            let ism = ala.isjab()?;
            let wahda = ala.isjab()?;
            let mansub = format!(
                "{}.{}",
                nass_min_uqda(ala, wahda).unwrap_or_else(|| "?".to_owned()),
                nass_min_uqda(ala, ism).unwrap_or_else(|| "?".to_owned())
            );
            ala.idfa(Uqda::Ism(mansub))?;
        },

        // -- construction, which constructs nothing -------------------------
        b'R' => {
            let muamalat = ala.isjab()?;
            let ism = ala.isjab()?;
            let asmaa = nass_min_uqda(ala, ism).unwrap_or_else(|| "?".to_owned());
            ala.idfa(Uqda::Kaain {
                ism: asmaa,
                muamalat: vec![muamalat],
                hala: None,
            })?;
        },
        0x81 | 0x92 => {
            // NEWOBJ leaves (class, args) on the stack; NEWOBJ_EX leaves
            // (class, args, keywords). Both are popped in reverse and none of
            // the three is ever called.
            let akhir = ala.isjab()?;
            let awsat = ala.isjab()?;
            let (fasila, mawad) = if ramz == 0x92 {
                (ala.isjab()?, vec![awsat, akhir])
            } else {
                (awsat, vec![akhir])
            };
            let asmaa = nass_min_uqda(ala, fasila).unwrap_or_else(|| "?".to_owned());
            ala.idfa(Uqda::Kaain {
                ism: asmaa,
                muamalat: mawad,
                hala: None,
            })?;
        },
        b'o' | b'i' => {
            *umq = umq.saturating_sub(1);
            let mut anasir = ala.min_alama()?;
            let asmaa = if ramz == b'i' {
                let wahda = nass_ascii(qari.satr("an instance's module")?)?;
                let ism = nass_ascii(qari.satr("an instance's class")?)?;
                format!("{wahda}.{ism}")
            } else {
                // OBJ takes the class as the first item after the mark.
                if anasir.is_empty() {
                    return Err(silsila_talifa("an obj opcode named no class"));
                }
                let ism = anasir.remove(0);
                nass_min_uqda(ala, ism).unwrap_or_else(|| "?".to_owned())
            };
            ala.idfa(Uqda::Kaain {
                ism: asmaa,
                muamalat: anasir,
                hala: None,
            })?;
        },
        b'b' => {
            let hala = ala.isjab()?;
            let hadaf = ala.qimma()?;
            let Some(Uqda::Kaain { ism, muamalat, .. }) =
                ala.uqad.get(usize::try_from(hadaf.0).unwrap_or(0))
            else {
                // A BUILD onto anything else would be __setstate__ on a value
                // this machine deliberately did not construct; recording it
                // would be pretending the state was applied.
                return Err(silsila_talifa(
                    "a build opcode targeted something that is not an object placeholder",
                ));
            };
            let tabdeel = Uqda::Kaain {
                ism: ism.clone(),
                muamalat: muamalat.clone(),
                hala: Some(hala),
            };
            ala.haddith(hadaf, tabdeel)?;
        },

        // -- refused --------------------------------------------------------
        b'P' | b'Q' => {
            return Err(ramz_marfud(
                ramz,
                "a persistent id resolves through a table this process would have to supply",
            ));
        },
        0x82..=0x84 => {
            return Err(ramz_marfud(
                ramz,
                "an extension code resolves through the copyreg registry, which is a global \
                 lookup and has no inert reading",
            ));
        },
        _ => {
            return Err(ramz_marfud(
                ramz,
                "not part of the subset this reader implements",
            ));
        },
    }
    Ok(())
}

/// The text of a value, for the opcodes that build a name out of two pushes.
fn nass_min_uqda(ala: &AlatSilsila, muashir: Muashir) -> Option<String> {
    match ala.uqad.get(usize::try_from(muashir.0).ok()?)? {
        Uqda::Nass(nass) | Uqda::Ism(nass) => Some(nass.clone()),
        Uqda::Bayt(bayt) => core::str::from_utf8(bayt).ok().map(str::to_owned),
        _ => None,
    }
}

/// A flat run of stack items read as alternating keys and values.
fn azwaj_min_qaima(anasir: &[Muashir]) -> Result<Vec<(Muashir, Muashir)>, KhataNusus> {
    if anasir.len().checked_rem(2) != Some(0) {
        return Err(silsila_talifa(
            "a mapping opcode was handed an odd number of items",
        ));
    }
    Ok(anasir
        .chunks_exact(2)
        .filter_map(|zawj| Some((*zawj.first()?, *zawj.get(1)?)))
        .collect())
}

/// A big integer as sign and little-endian magnitude, from two's complement.
fn kabir_min_bayt(juz: &[u8]) -> Uqda {
    if juz.is_empty() {
        return Uqda::Sahih(0);
    }
    let salib = juz.last().is_some_and(|akhir| *akhir & 0x80 != 0);
    if !salib && juz.len() <= 8 {
        let mut masfufa = [0_u8; 8];
        for (khana, bayt) in juz.iter().enumerate() {
            if let Some(hadaf) = masfufa.get_mut(khana) {
                *hadaf = *bayt;
            }
        }
        let qeema = u64::from_le_bytes(masfufa);
        if let Ok(sahih) = i64::try_from(qeema) {
            return Uqda::Sahih(sahih);
        }
    }
    let mut adad: Vec<u8> = juz.to_vec();
    if salib {
        // Negate in place: invert every byte and add one, which turns two's
        // complement into a magnitude without ever widening past the stream's
        // own length.
        let mut hamil = 1_u16;
        for bayt in &mut adad {
            let jadeed = u16::from(!*bayt).saturating_add(hamil);
            *bayt = u8::try_from(jadeed & 0xFF).unwrap_or(0);
            hamil = jadeed >> 8;
        }
    }
    while adad.len() > 1 && adad.last() == Some(&0) {
        let _ = adad.pop();
    }
    Uqda::Kabir { salib, adad }
}

/// Bytes that must be ASCII text, as text.
fn nass_ascii(juz: &[u8]) -> Result<String, KhataNusus> {
    core::str::from_utf8(juz)
        .map(str::to_owned)
        .map_err(|khata| KhataNusus::NassGhayrSalih {
            malaf: "a Ren'Py pickle stream".to_owned(),
            tarmiz: "ASCII",
            mawqi: tul_u64(khata.valid_up_to()),
        })
}

/// Bytes that must be UTF-8, as text, naming where they stopped being valid.
fn nass_utf8(juz: &[u8], mawqi: usize) -> Result<String, KhataNusus> {
    core::str::from_utf8(juz)
        .map(str::to_owned)
        .map_err(|khata| KhataNusus::NassGhayrSalih {
            malaf: "a Ren'Py pickle stream".to_owned(),
            tarmiz: "UTF-8",
            mawqi: tul_u64(mawqi.saturating_add(khata.valid_up_to())),
        })
}

/// A protocol-0 quoted string, with the escapes Python's repr produces.
///
/// Lossy on purpose and only here: these opcodes appear in archives written by
/// a Python 2 Ren'Py, where the payload is a path name in whatever the author's
/// filesystem used, and refusing an archive because one asset has a byte that is
/// not UTF-8 would refuse the archive over a file nothing is going to translate.
fn nass_muqtabas(juz: &[u8]) -> String {
    let khaam = String::from_utf8_lossy(juz);
    let munaqqa = khaam.trim();
    let bila_iqtibas = munaqqa
        .strip_prefix('\'')
        .and_then(|baqi| baqi.strip_suffix('\''))
        .or_else(|| {
            munaqqa
                .strip_prefix('"')
                .and_then(|baqi| baqi.strip_suffix('"'))
        })
        .unwrap_or(munaqqa);
    let mut kharj = String::with_capacity(bila_iqtibas.len());
    let mut huruf = bila_iqtibas.chars();
    while let Some(harf) = huruf.next() {
        if harf != '\\' {
            kharj.push(harf);
            continue;
        }
        match huruf.next() {
            Some('n') => kharj.push('\n'),
            Some('t') => kharj.push('\t'),
            Some('r') => kharj.push('\r'),
            Some('0') => kharj.push('\0'),
            Some(akhar) => kharj.push(akhar),
            None => kharj.push('\\'),
        }
    }
    kharj
}

/// Serializes an inert value tree back into a protocol 2 pickle.
///
/// Protocol 2 rather than the newest one, because the archive this output goes
/// into is read by whichever Python the game ships, and a Ren'Py old enough to
/// need a rebuilt archive is old enough to be running Python 2.7, which cannot
/// load protocol 3 or above. Protocol 2 loads everywhere.
///
/// [`Uqda::Ism`] and [`Uqda::Kaain`] are refused. Emitting either means emitting
/// a `GLOBAL` opcode, which is an instruction to import a module and call
/// something — writing code into a file the game will later execute. This crate
/// does not do that, so the writer has no way to.
///
/// # Errors
///
/// [`KhataNusus::BunyaGhayrMutawaqqaa`] for a value this writer refuses to
/// emit, a handle from another stream, or a structure nested past
/// [`AQSA_UMQ_SILSILA`].
pub fn uktub_silsila(silsila: &Silsila) -> Result<Vec<u8>, KhataNusus> {
    let mut kharj: Vec<u8> = vec![0x80, 0x02];
    let mut dhakira: BTreeMap<u32, u64> = BTreeMap::new();
    uktub_qeema(silsila, silsila.jidhr(), &mut kharj, &mut dhakira, 0)?;
    kharj.push(b'.');
    Ok(kharj)
}

/// Emits the memo instruction for a value that has just been written.
fn uktub_dhakira(kharj: &mut Vec<u8>, raqm: u64) {
    if let Ok(sagheer) = u8::try_from(raqm) {
        kharj.push(b'q');
        kharj.push(sagheer);
    } else {
        kharj.push(b'r');
        kharj.extend_from_slice(&u32::try_from(raqm).unwrap_or(u32::MAX).to_le_bytes());
    }
}

/// Emits a reference to an already-written value.
fn uktub_marja(kharj: &mut Vec<u8>, raqm: u64) {
    if let Ok(sagheer) = u8::try_from(raqm) {
        kharj.push(b'h');
        kharj.push(sagheer);
    } else {
        kharj.push(b'j');
        kharj.extend_from_slice(&u32::try_from(raqm).unwrap_or(u32::MAX).to_le_bytes());
    }
}

/// Emits an integer in the narrowest opcode that holds it.
fn uktub_sahih(kharj: &mut Vec<u8>, qeema: i64) {
    if let Ok(sagheer) = u8::try_from(qeema) {
        kharj.push(b'K');
        kharj.push(sagheer);
        return;
    }
    if let Ok(mutawassit) = u16::try_from(qeema) {
        kharj.push(b'M');
        kharj.extend_from_slice(&mutawassit.to_le_bytes());
        return;
    }
    if let Ok(arbaa) = i32::try_from(qeema) {
        kharj.push(b'J');
        kharj.extend_from_slice(&arbaa.to_le_bytes());
        return;
    }
    // Above a 32-bit integer, protocol 2 has only LONG1, which carries a
    // little-endian two's complement magnitude.
    let bayt = qeema.to_le_bytes();
    kharj.push(0x8A);
    kharj.push(8);
    kharj.extend_from_slice(&bayt);
}

/// Emits a length-prefixed payload with the narrowest opcode that holds it.
fn uktub_tul(kharj: &mut Vec<u8>, qaseer: u8, taweel: u8, juz: &[u8]) {
    if let Ok(sagheer) = u8::try_from(juz.len()) {
        kharj.push(qaseer);
        kharj.push(sagheer);
    } else {
        kharj.push(taweel);
        kharj.extend_from_slice(&u32::try_from(juz.len()).unwrap_or(u32::MAX).to_le_bytes());
    }
    kharj.extend_from_slice(juz);
}

/// Writes one value, memoizing every container so shared structure stays shared.
fn uktub_qeema(
    silsila: &Silsila,
    muashir: Muashir,
    kharj: &mut Vec<u8>,
    dhakira: &mut BTreeMap<u32, u64>,
    umq: u32,
) -> Result<(), KhataNusus> {
    if umq > AQSA_UMQ_SILSILA {
        return Err(silsila_talifa(
            "a value nests deeper than this writer will emit",
        ));
    }
    if let Some(raqm) = dhakira.get(&muashir.raqm()).copied() {
        uktub_marja(kharj, raqm);
        return Ok(());
    }
    let uqda = silsila
        .uqda(muashir)
        .ok_or_else(|| silsila_talifa("a handle from another stream reached the writer"))?;

    // A container is memoized *before* its members are written, so a member
    // that refers back to it emits a reference rather than recursing forever.
    let sajjil = |kharj: &mut Vec<u8>, dhakira: &mut BTreeMap<u32, u64>| {
        let raqm = tul_u64(dhakira.len());
        let _ = dhakira.insert(muashir.raqm(), raqm);
        uktub_dhakira(kharj, raqm);
    };

    match uqda {
        Uqda::Faragh => kharj.push(b'N'),
        Uqda::Mantiq(true) => kharj.push(0x88),
        Uqda::Mantiq(false) => kharj.push(0x89),
        Uqda::Sahih(qeema) => uktub_sahih(kharj, *qeema),
        Uqda::Ashari(qeema) => {
            kharj.push(b'G');
            kharj.extend_from_slice(&qeema.to_be_bytes());
        },
        Uqda::Kabir { salib, adad } => {
            let mut bayt = adad.clone();
            if *salib {
                let mut hamil = 1_u16;
                for wahid in &mut bayt {
                    let jadeed = u16::from(!*wahid).saturating_add(hamil);
                    *wahid = u8::try_from(jadeed & 0xFF).unwrap_or(0);
                    hamil = jadeed >> 8;
                }
                bayt.push(0xFF);
            } else if bayt.last().is_some_and(|akhir| *akhir & 0x80 != 0) {
                bayt.push(0x00);
            }
            let tul = u8::try_from(bayt.len()).map_err(|_| {
                silsila_talifa("a big integer is wider than protocol 2's LONG1 opcode holds")
            })?;
            kharj.push(0x8A);
            kharj.push(tul);
            kharj.extend_from_slice(&bayt);
        },
        Uqda::Nass(nass) => {
            let juz = nass.as_bytes();
            kharj.push(b'X');
            kharj.extend_from_slice(&u32::try_from(juz.len()).unwrap_or(u32::MAX).to_le_bytes());
            kharj.extend_from_slice(juz);
            sajjil(kharj, dhakira);
        },
        Uqda::Bayt(bayt) => {
            // Protocol 2 has no BYTES opcode; Python 2's `str` and Python 3's
            // `bytes` share the SHORT_BINSTRING/BINSTRING pair, which is what
            // Ren'Py's own index writer emits for a member prefix.
            uktub_tul(kharj, b'U', b'T', bayt);
            sajjil(kharj, dhakira);
        },
        Uqda::Thulathi(anasir) if anasir.is_empty() => kharj.push(b')'),
        Uqda::Thulathi(anasir) => {
            // A tuple is immutable, so it is written *before* it is memoized:
            // it cannot contain itself, and protocol 2 builds it from the
            // stack. Which stack instruction applies depends on the arity, and
            // the four-or-more form needs a mark that has to precede the
            // members — so the arity is decided before anything is emitted
            // rather than discovered afterwards and rolled back.
            let bil_alama = anasir.len() > 3;
            if bil_alama {
                kharj.push(b'(');
            }
            for udw in anasir {
                uktub_qeema(silsila, *udw, kharj, dhakira, umq.saturating_add(1))?;
            }
            match anasir.len() {
                1 => kharj.push(0x85),
                2 => kharj.push(0x86),
                3 => kharj.push(0x87),
                _ => kharj.push(b't'),
            }
            sajjil(kharj, dhakira);
        },
        Uqda::Qaima(anasir) => {
            kharj.push(b']');
            sajjil(kharj, dhakira);
            if !anasir.is_empty() {
                kharj.push(b'(');
                for udw in anasir {
                    uktub_qeema(silsila, *udw, kharj, dhakira, umq.saturating_add(1))?;
                }
                kharj.push(b'e');
            }
        },
        Uqda::Majmua(anasir) => {
            // Protocol 2 has no set opcode, so a set is emitted as the list
            // Ren'Py's index never contains one of anyway; refusing is honest.
            let _ = anasir;
            return Err(silsila_talifa(
                "protocol 2 cannot represent a set, and no structure this crate writes has one",
            ));
        },
        Uqda::Kharita(azwaj) => {
            kharj.push(b'}');
            sajjil(kharj, dhakira);
            if !azwaj.is_empty() {
                kharj.push(b'(');
                for (miftah, qeema) in azwaj {
                    uktub_qeema(silsila, *miftah, kharj, dhakira, umq.saturating_add(1))?;
                    uktub_qeema(silsila, *qeema, kharj, dhakira, umq.saturating_add(1))?;
                }
                kharj.push(b'u');
            }
        },
        Uqda::Ism(ism) => {
            return Err(silsila_talifa(&format!(
                "writing the global {ism} would mean emitting an import instruction into a \
                 file the game executes, which this crate never does"
            )));
        },
        Uqda::Kaain { ism, .. } => {
            return Err(silsila_talifa(&format!(
                "writing a constructed {ism} would mean emitting a call instruction into a \
                 file the game executes, which this crate never does"
            )));
        },
    }
    Ok(())
}

/// Builds a value tree from scratch, for the archive index writer.
///
/// A [`Silsila`] read from a file has a root and an arena; one being *built*
/// has an arena and no root until the last value is interned, which is what
/// this exists to express. Nothing here can produce [`Uqda::Ism`] or
/// [`Uqda::Kaain`], so a tree built through this type is one the writer can
/// always emit.
#[derive(Debug, Clone, Default)]
pub struct BinaSilsila {
    uqad: Vec<Uqda>,
}

impl BinaSilsila {
    /// An empty builder.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { uqad: Vec::new() }
    }

    /// Interns a value and returns its handle.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the tree passes
    /// [`AQSA_KAINAT_SILSILA`], which is the same ceiling the reader enforces
    /// and for the same reason.
    pub fn adhif(&mut self, uqda: Uqda) -> Result<Muashir, KhataNusus> {
        if self.uqad.len() >= AQSA_KAINAT_SILSILA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the objects a pickle stream creates",
                qeema: tul_u64(self.uqad.len().saturating_add(1)),
                saqf: tul_u64(AQSA_KAINAT_SILSILA),
            });
        }
        let raqm = u32::try_from(self.uqad.len()).map_err(|_| KhataNusus::HajmMufrit {
            haql: "the objects a pickle stream creates",
            qeema: tul_u64(self.uqad.len()),
            saqf: u64::from(u32::MAX),
        })?;
        self.uqad.push(uqda);
        Ok(Muashir(raqm))
    }

    /// Interns text.
    ///
    /// # Errors
    ///
    /// As [`BinaSilsila::adhif`].
    pub fn nass(&mut self, nass: impl Into<String>) -> Result<Muashir, KhataNusus> {
        self.adhif(Uqda::Nass(nass.into()))
    }

    /// Interns an integer.
    ///
    /// # Errors
    ///
    /// As [`BinaSilsila::adhif`].
    pub fn sahih(&mut self, qeema: i64) -> Result<Muashir, KhataNusus> {
        self.adhif(Uqda::Sahih(qeema))
    }

    /// Interns bytes.
    ///
    /// # Errors
    ///
    /// As [`BinaSilsila::adhif`].
    pub fn bayt(&mut self, bayt: Vec<u8>) -> Result<Muashir, KhataNusus> {
        self.adhif(Uqda::Bayt(bayt))
    }

    /// Finishes the tree, naming which handle is its result.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the handle is not one this
    /// builder issued.
    pub fn ikhtim(self, jidhr: Muashir) -> Result<Silsila, KhataNusus> {
        if usize::try_from(jidhr.0)
            .ok()
            .is_none_or(|fahras| fahras >= self.uqad.len())
        {
            return Err(silsila_talifa(
                "a builder was closed on a handle it never issued",
            ));
        }
        Ok(Silsila {
            uqad: self.uqad,
            jidhr,
        })
    }
}

// ---------------------------------------------------------------------------
// .rpa archives
//
// One text line, then the packed members, then a zlib-compressed pickled index
// at an offset the line names. From RPA-3.0 the index is obfuscated: every
// offset and length in it is XORed with a key that is *also* in that same line,
// which makes it obfuscation rather than encryption and is exactly why Taarib
// can read it without ever attempting to derive a key the game did not publish.
//
//   RPA-2.0 <16 hex digits: index offset>\n
//   RPA-3.0 <16 hex digits: index offset> <8 hex digits: key>\n
//   RPA-3.2 <16 hex digits: index offset> <8 hex digits> <8 hex digits: key>\n
//
// For 3.0 the key is the exclusive-or of every whitespace-separated field after
// the offset; for 3.2 it is the exclusive-or of every field after the *second*
// one. That difference is the whole of the 3.2 change and the reason a reader
// written for 3.0 produces an index full of enormous offsets on a 3.2 archive
// rather than failing outright — so the two are separate variants here and the
// field is skipped by version rather than by heuristic.
// ---------------------------------------------------------------------------

/// Which RPA generation an archive is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsdarRpa {
    /// RPA-2.0: an unobfuscated index.
    Thani,
    /// RPA-3.0: the index is `XORed` with the key in the header line.
    Thalith,
    /// RPA-3.2: the same, with one extra header field before the key.
    ThalithNuqtatan,
}

impl IsdarRpa {
    /// The marker the header line opens with.
    #[must_use]
    pub const fn alama(self) -> &'static str {
        match self {
            Self::Thani => "RPA-2.0",
            Self::Thalith => "RPA-3.0",
            Self::ThalithNuqtatan => "RPA-3.2",
        }
    }

    /// Which header field the key starts at, counting the marker as field zero.
    ///
    /// Two for RPA-3.0 and three for RPA-3.2. The value is a property of the
    /// format rather than of the file, which is why it is a method on the
    /// version and not something recovered from the line.
    #[must_use]
    pub const fn awwal_miftah(self) -> usize {
        match self {
            Self::Thani => usize::MAX,
            Self::Thalith => 2,
            Self::ThalithNuqtatan => 3,
        }
    }

    /// How many bytes this version's header line occupies.
    ///
    /// Fixed per version, which is what lets a writer reserve the line, stream
    /// every member past it, and fill the index offset in afterwards without
    /// moving a single byte of payload.
    #[must_use]
    pub const fn tul_tarwisa(self) -> usize {
        match self {
            // "RPA-2.0 " + 16 + "\n"
            Self::Thani => 25,
            // "RPA-3.0 " + 16 + " " + 8 + "\n"
            Self::Thalith => 34,
            // "RPA-3.2 " + 16 + " " + 8 + " " + 8 + "\n"
            Self::ThalithNuqtatan => 43,
        }
    }

    /// The version a header line declares, or [`None`] for anything else.
    #[must_use]
    pub fn min_satr(satr: &str) -> Option<Self> {
        let alama = satr.split_whitespace().next()?;
        match alama {
            "RPA-2.0" => Some(Self::Thani),
            "RPA-3.0" => Some(Self::Thalith),
            "RPA-3.2" => Some(Self::ThalithNuqtatan),
            _ => None,
        }
    }
}

/// One member of an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadkhalRpa {
    /// The member's path inside the archive, with forward slashes.
    pub masar: String,
    /// Where its stored bytes begin.
    pub izaha: u64,
    /// How many bytes the member is in total, *including* its prefix.
    pub tul: u64,
    /// The bytes Ren'Py stores in the index instead of in the archive body.
    ///
    /// A packing trick rather than a security measure: the first few bytes of a
    /// member live in the index, so `tul` minus this length is how much is
    /// actually read from `izaha`. A reader that ignored it would return every
    /// member short by a handful of bytes at the front, which for a `.rpyc` is
    /// exactly the magic.
    pub sabiqa: Vec<u8>,
}

impl MadkhalRpa {
    /// How many bytes are read from the archive body for this member.
    #[must_use]
    pub fn tul_mukhazzan(&self) -> u64 {
        self.tul.saturating_sub(tul_u64(self.sabiqa.len()))
    }
}

/// An opened archive: its header, its index, and nothing else.
///
/// The member bytes are deliberately *not* held. A Ren'Py archive routinely
/// runs to gigabytes of packed audio and video, and a reader that loaded one to
/// find the scripts in it would be a reader nobody can run on a laptop.
#[derive(Debug, Clone)]
pub struct Hawiya {
    /// Where the archive is.
    pub masar: PathBuf,
    /// Which generation it is.
    pub isdar: IsdarRpa,
    /// The obfuscation key from the header line; zero for RPA-2.0.
    pub miftah: u32,
    /// Where the index begins.
    pub izahat_fahras: u64,
    madakhil: Vec<MadkhalRpa>,
}

impl Hawiya {
    /// Every member, in the order the index listed them.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalRpa] {
        &self.madakhil
    }

    /// One member by path, compared exactly.
    #[must_use]
    pub fn madkhal(&self, masar: &str) -> Option<&MadkhalRpa> {
        self.madakhil.iter().find(|madkhal| madkhal.masar == masar)
    }

    /// Every member whose path ends with `lahiqa`.
    pub fn bi_lahiqa<'a>(&'a self, lahiqa: &'a str) -> impl Iterator<Item = &'a MadkhalRpa> + 'a {
        self.madakhil
            .iter()
            .filter(move |madkhal| madkhal.masar.ends_with(lahiqa))
    }
}

/// The name this format uses in a refusal.
const SIGHA_RPA: &str = "RPA";

/// An I/O failure, carrying the path that produced it.
fn khata_malaf(masar: &Path, sabab: std::io::Error) -> KhataNusus {
    KhataNusus::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    }
}

/// A field in a container that points outside it.
const fn hawiya_talifa(haql: &'static str, qeema: u64, hadd: u64) -> KhataNusus {
    KhataNusus::HawiyaTalifa {
        sigha: SIGHA_RPA,
        haql,
        qeema,
        hadd,
    }
}

impl Hawiya {
    /// Opens an archive and reads its index, and nothing else.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the file cannot be read,
    /// [`KhataNusus::SihrGhayrMutabaq`] when it does not begin with an RPA
    /// header line, [`KhataNusus::HawiyaTalifa`] when the index offset or an
    /// entry points outside the file, [`KhataNusus::HajmMufrit`] when the index
    /// expands past [`AQSA_FAHRAS_RPA`], [`KhataNusus::FakkFashil`] when it
    /// will not decompress, and [`KhataNusus::BunyaGhayrMutawaqqaa`] when the
    /// decompressed index is not the mapping this format defines.
    pub fn iqra(masar: &Path) -> Result<Self, KhataNusus> {
        use std::io::{Seek as _, SeekFrom};

        let mut malaf = fs::File::open(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
        let tul_malaf = malaf
            .metadata()
            .map_err(|sabab| khata_malaf(masar, sabab))?
            .len();

        let mut tarwisa = vec![0_u8; 64.min(hajm_usize(tul_malaf).unwrap_or(64))];
        {
            let mut mala = 0_usize;
            while mala < tarwisa.len() {
                let Some(baqi) = tarwisa.get_mut(mala..) else {
                    break;
                };
                match malaf.read(baqi) {
                    Ok(0) => break,
                    Ok(adad) => mala = mala.saturating_add(adad),
                    Err(sabab) if sabab.kind() == std::io::ErrorKind::Interrupted => {},
                    Err(sabab) => return Err(khata_malaf(masar, sabab)),
                }
            }
            tarwisa.truncate(mala);
        }
        let nihayat_satr = tarwisa
            .iter()
            .position(|bayt| *bayt == b'\n')
            .ok_or_else(|| KhataNusus::SihrGhayrMutabaq {
                masar: masar.to_path_buf(),
                sigha: SIGHA_RPA,
            })?;
        let satr = tarwisa
            .get(..nihayat_satr)
            .map(String::from_utf8_lossy)
            .ok_or_else(|| KhataNusus::SihrGhayrMutabaq {
                masar: masar.to_path_buf(),
                sigha: SIGHA_RPA,
            })?;
        let isdar = IsdarRpa::min_satr(&satr).ok_or_else(|| KhataNusus::SihrGhayrMutabaq {
            masar: masar.to_path_buf(),
            sigha: SIGHA_RPA,
        })?;

        let huqul: Vec<&str> = satr.split_whitespace().collect();
        let izahat_fahras = huqul
            .get(1)
            .and_then(|haql| u64::from_str_radix(haql, 16).ok())
            .ok_or_else(|| hawiya_talifa("the index offset in the header line", 0, tul_malaf))?;
        if izahat_fahras >= tul_malaf {
            return Err(hawiya_talifa("the index offset", izahat_fahras, tul_malaf));
        }
        // Every field from the version's own starting position, exclusive-ored
        // together. Ren'Py writes one field in practice and the format allows
        // several; folding them is what the engine's own reader does.
        let miftah = match isdar {
            IsdarRpa::Thani => 0_u32,
            _ => huqul
                .get(isdar.awwal_miftah()..)
                .unwrap_or_default()
                .iter()
                .filter_map(|haql| u32::from_str_radix(haql, 16).ok())
                .fold(0_u32, |majmu, juz| majmu ^ juz),
        };

        let _ = malaf
            .seek(SeekFrom::Start(izahat_fahras))
            .map_err(|sabab| khata_malaf(masar, sabab))?;
        let madghut = tul_malaf.saturating_sub(izahat_fahras);
        if madghut > AQSA_FAHRAS_RPA {
            return Err(KhataNusus::HajmMufrit {
                haql: "the stored archive index",
                qeema: madghut,
                saqf: AQSA_FAHRAS_RPA,
            });
        }
        let khaam = fukk_zlib(malaf, AQSA_FAHRAS_RPA, SIGHA_RPA)?;
        let silsila = iqra_silsila(&khaam)?;
        let madakhil = fahras_min_silsila(&silsila, miftah, tul_malaf, isdar)?;

        Ok(Self {
            masar: masar.to_path_buf(),
            isdar,
            miftah,
            izahat_fahras,
            madakhil,
        })
    }

    /// Reads one member's bytes.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the archive cannot be read,
    /// [`KhataNusus::HajmMufrit`] when the member declares more than
    /// [`AQSA_UDW_RPA`], and [`KhataNusus::HawiyaTalifa`] when its range leaves
    /// the file — which is checked against the file's real length here rather
    /// than trusted from the index, because the index is data from the same
    /// untrusted file.
    pub fn istakhrij(&self, madkhal: &MadkhalRpa) -> Result<Vec<u8>, KhataNusus> {
        use std::io::{Seek as _, SeekFrom};

        if madkhal.tul > AQSA_UDW_RPA {
            return Err(KhataNusus::HajmMufrit {
                haql: "an archive member",
                qeema: madkhal.tul,
                saqf: AQSA_UDW_RPA,
            });
        }
        let mut malaf =
            fs::File::open(&self.masar).map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        let tul_malaf = malaf
            .metadata()
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?
            .len();
        let tul_jism = madkhal.tul_mukhazzan();
        let nihaya = madkhal
            .izaha
            .checked_add(tul_jism)
            .ok_or_else(|| hawiya_talifa("a member's end", madkhal.izaha, tul_malaf))?;
        if nihaya > tul_malaf {
            return Err(hawiya_talifa("a member's end", nihaya, tul_malaf));
        }
        let _ = malaf
            .seek(SeekFrom::Start(madkhal.izaha))
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?;

        let mut kharj = Vec::with_capacity(hajm_usize(madkhal.tul).unwrap_or(0).min(1024 * 1024));
        kharj.extend_from_slice(&madkhal.sabiqa);
        let mut jism = vec![0_u8; hajm_usize(tul_jism).unwrap_or(0)];
        malaf
            .read_exact(&mut jism)
            .map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        kharj.append(&mut jism);
        Ok(kharj)
    }
}

/// Expands a zlib stream, refusing anything above `saqf` before it is held.
///
/// Bounded twice on purpose: once on the compressed input, so a decompressor is
/// never handed an unbounded stream, and once on the output, so a small input
/// that expands enormously is refused at the ceiling rather than after it has
/// been allocated. A compression bomb inside a game archive is not exotic.
fn fukk_zlib(
    masdar: impl std::io::Read,
    saqf: u64,
    sigha: &'static str,
) -> Result<Vec<u8>, KhataNusus> {
    let fakk = flate2::read::ZlibDecoder::new(masdar.take(saqf));
    let mut khaam = Vec::new();
    let adad = fakk
        .take(saqf.saturating_add(1))
        .read_to_end(&mut khaam)
        .map_err(|sabab| KhataNusus::FakkFashil {
            sigha,
            tafsil: sabab.to_string(),
        })?;
    if tul_u64(adad) > saqf {
        return Err(KhataNusus::HajmMufrit {
            haql: "an expanded archive index",
            qeema: tul_u64(adad),
            saqf,
        });
    }
    Ok(khaam)
}

/// Turns a decoded index into entries, de-obfuscating and bounds-checking each.
///
/// Only the first segment of each member is kept. The index's value is a *list*
/// of segments because the format once allowed a member to be stored in pieces;
/// Ren'Py's own reader has always used the first and nothing has written more
/// than one in a decade, so honouring the rest would be implementing a feature
/// against no producer.
fn fahras_min_silsila(
    silsila: &Silsila,
    miftah: u32,
    tul_malaf: u64,
    isdar: IsdarRpa,
) -> Result<Vec<MadkhalRpa>, KhataNusus> {
    let bunya = |haql: &str| KhataNusus::BunyaGhayrMutawaqqaa {
        malaf: format!("a {} index", isdar.alama()),
        haql: haql.to_owned(),
    };
    let Some(Uqda::Kharita(azwaj)) = silsila.uqda(silsila.jidhr()) else {
        return Err(bunya("the index is not a mapping of paths to segments"));
    };

    let miftah64 = u64::from(miftah);
    let mut madakhil = Vec::with_capacity(azwaj.len());
    for (ism, qeema) in azwaj {
        let masar = match silsila.uqda(*ism) {
            Some(Uqda::Nass(nass)) => nass.clone(),
            Some(Uqda::Bayt(bayt)) => String::from_utf8_lossy(bayt).into_owned(),
            _ => return Err(bunya("an index key is neither text nor bytes")),
        };
        let Some(qita) = silsila.anasir(*qeema) else {
            return Err(bunya("an index value is not a list of segments"));
        };
        let Some(awwal) = qita.first() else {
            continue;
        };
        let Some(huqul) = silsila.anasir(*awwal) else {
            return Err(bunya("an index segment is not a tuple"));
        };

        let khaam_izaha = huqul
            .first()
            .and_then(|udw| silsila.sahih(*udw))
            .ok_or_else(|| bunya("a segment's offset is not an integer"))?;
        let khaam_tul = huqul
            .get(1)
            .and_then(|udw| silsila.sahih(*udw))
            .ok_or_else(|| bunya("a segment's length is not an integer"))?;
        let sabiqa = match huqul.get(2).and_then(|udw| silsila.uqda(*udw)) {
            Some(Uqda::Bayt(bayt)) => bayt.clone(),
            Some(Uqda::Nass(nass)) => nass.as_bytes().to_vec(),
            Some(Uqda::Faragh) | None => Vec::new(),
            Some(_) => return Err(bunya("a segment's prefix is neither bytes nor text")),
        };

        // The de-obfuscation. Both fields are XORed with the same key, which is
        // in the header line of the same file — hence obfuscation, not
        // encryption, and hence nothing here derives a key the game withheld.
        let izaha = u64::try_from(khaam_izaha)
            .map_err(|_| bunya("a segment's offset is negative"))?
            ^ miftah64;
        let tul = u64::try_from(khaam_tul).map_err(|_| bunya("a segment's length is negative"))?
            ^ miftah64;

        if izaha > tul_malaf {
            return Err(hawiya_talifa("a member's offset", izaha, tul_malaf));
        }
        let nihaya = izaha
            .checked_add(tul.saturating_sub(tul_u64(sabiqa.len())))
            .ok_or_else(|| hawiya_talifa("a member's end", izaha, tul_malaf))?;
        if nihaya > tul_malaf {
            return Err(hawiya_talifa("a member's end", nihaya, tul_malaf));
        }

        madakhil.push(MadkhalRpa {
            masar,
            izaha,
            tul,
            sabiqa,
        });
    }
    Ok(madakhil)
}

/// Builds a new archive, streaming members past a reserved header line.
///
/// The header line's length is fixed per version, so the line is written as
/// filler first, every member is appended after it, the index is appended last,
/// and the line is then rewritten in place with the offset the index actually
/// landed at. Nothing is buffered but the index, which is the one part that has
/// to be complete before it can be written.
///
/// ## Why this one does not take a [`Hafiz`]
///
/// Every other writer in this crate goes through the reversibility guard,
/// because every other writer modifies a game. This one does not: it produces a
/// fresh archive at a path the packager chose, in a staging directory, and there
/// is no original anywhere near it to preserve. Handing it a guard would record
/// a manifest line for a file that is not part of anybody's installation.
///
/// The hole that argument leaves — a caller pointing it at `game/archive.rpa` —
/// is closed by [`KatibRpa::jadeed`] refusing a target that already exists,
/// rather than by asking callers to be careful. It cannot overwrite, so there is
/// nothing for it to make irreversible.
#[derive(Debug)]
pub struct KatibRpa {
    masar: PathBuf,
    muaqqat: PathBuf,
    malaf: fs::File,
    isdar: IsdarRpa,
    miftah: u32,
    mawqi: u64,
    madakhil: Vec<MadkhalRpa>,
}

impl KatibRpa {
    /// Opens a new archive beside `masar`, to be renamed into place at the end.
    ///
    /// Refuses when anything is already at `masar`. This type is the one writer
    /// in the crate that does not hold a [`Hafiz`], and the reason it does not
    /// need one is exactly this check: a builder that cannot overwrite cannot
    /// destroy an original, so there is no original for a manifest to record.
    /// Removing this refusal would silently turn a staging-directory tool into
    /// an unrecorded game patcher.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when something is already at the target,
    /// and [`KhataNusus::KhataMalaf`] when the temporary file cannot be created
    /// or the reserved header line cannot be written.
    pub fn jadeed(masar: &Path, isdar: IsdarRpa) -> Result<Self, KhataNusus> {
        use std::io::Write as _;

        if masar.exists() {
            return Err(KhataNusus::NuskhaMafquda {
                masar: masar.to_path_buf(),
                sabab: "something is already at this path, and this builder writes new \
                        archives only. Overwriting an archive a game ships is a modification, \
                        and modifications go through the reversibility guard."
                    .to_owned(),
            });
        }
        let muaqqat = masar.with_extension("rpa-taarib");
        let mut malaf = fs::File::create(&muaqqat).map_err(|sabab| khata_malaf(&muaqqat, sabab))?;
        let hashw = vec![b' '; isdar.tul_tarwisa().saturating_sub(1)];
        malaf
            .write_all(&hashw)
            .map_err(|sabab| khata_malaf(&muaqqat, sabab))?;
        malaf
            .write_all(b"\n")
            .map_err(|sabab| khata_malaf(&muaqqat, sabab))?;
        Ok(Self {
            masar: masar.to_path_buf(),
            muaqqat,
            malaf,
            isdar,
            miftah: 0,
            mawqi: tul_u64(isdar.tul_tarwisa()),
            madakhil: Vec::new(),
        })
    }

    /// Appends one member.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when the write fails, and
    /// [`KhataNusus::HajmMufrit`] when the member is larger than
    /// [`AQSA_UDW_RPA`].
    pub fn adhif(&mut self, masar: &str, bayt: &[u8]) -> Result<(), KhataNusus> {
        use std::io::Write as _;

        let tul = tul_u64(bayt.len());
        if tul > AQSA_UDW_RPA {
            return Err(KhataNusus::HajmMufrit {
                haql: "an archive member",
                qeema: tul,
                saqf: AQSA_UDW_RPA,
            });
        }
        self.malaf
            .write_all(bayt)
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;
        self.madakhil.push(MadkhalRpa {
            masar: masar.replace('\\', "/"),
            izaha: self.mawqi,
            tul,
            // Taarib writes no prefix. The trick exists to save a seek on the
            // engine's side and costs a reader nothing to honour on read; a
            // writer that used it would be making its own output harder to
            // check for no benefit.
            sabiqa: Vec::new(),
        });
        self.mawqi = self.mawqi.saturating_add(tul);
        Ok(())
    }

    /// Writes the index, fixes the header line, and renames over the target.
    ///
    /// The key is derived from a BLAKE3 of every member's path and length
    /// rather than drawn from a random source. Two consequences, and both are
    /// deliberate: rebuilding the same archive twice produces byte-identical
    /// output, so a patch can be diffed and a round-trip can be verified; and
    /// nothing about the key is secret, which is honest, because the key is
    /// written into the header line of the same file.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] for any failed write, rename or sync, and
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the index cannot be built —
    /// which can only happen if the member list passed
    /// [`AQSA_KAINAT_SILSILA`].
    pub fn ikhtim(mut self) -> Result<(), KhataNusus> {
        use std::io::{Seek as _, SeekFrom, Write as _};

        let mut basma = blake3::Hasher::new();
        for madkhal in &self.madakhil {
            basma.update(madkhal.masar.as_bytes());
            basma.update(&madkhal.tul.to_le_bytes());
        }
        let khulasa = basma.finalize();
        let mut arbaa = [0_u8; 4];
        if let Some(juz) = khulasa.as_bytes().get(..4) {
            arbaa.copy_from_slice(juz);
        }
        self.miftah = match self.isdar {
            IsdarRpa::Thani => 0,
            // Zero would be a valid key and an invisible one, and an archive
            // whose key happens to be zero is one where a broken de-obfuscator
            // still appears to work. Forcing a non-zero key keeps that bug
            // visible the first time it happens rather than on one archive in
            // four billion.
            _ => u32::from_le_bytes(arbaa) | 1,
        };

        let fahras = self.ibn_fahras()?;
        let madghut = udghut_zlib(&fahras)?;
        let izahat_fahras = self.mawqi;
        self.malaf
            .write_all(&madghut)
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;

        let satr = match self.isdar {
            IsdarRpa::Thani => format!("{} {izahat_fahras:016x}", self.isdar.alama()),
            IsdarRpa::Thalith => {
                format!(
                    "{} {izahat_fahras:016x} {:08x}",
                    self.isdar.alama(),
                    self.miftah
                )
            },
            IsdarRpa::ThalithNuqtatan => format!(
                "{} {izahat_fahras:016x} {:08x} {:08x}",
                self.isdar.alama(),
                1_u32,
                self.miftah
            ),
        };
        let _ = self
            .malaf
            .seek(SeekFrom::Start(0))
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;
        self.malaf
            .write_all(satr.as_bytes())
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;
        self.malaf
            .flush()
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;
        self.malaf
            .sync_all()
            .map_err(|sabab| khata_malaf(&self.muaqqat, sabab))?;
        drop(self.malaf);

        fs::rename(&self.muaqqat, &self.masar).map_err(|sabab| khata_malaf(&self.masar, sabab))?;
        Ok(())
    }

    /// Builds the pickled index for everything written so far.
    fn ibn_fahras(&self) -> Result<Vec<u8>, KhataNusus> {
        let mut bina = BinaSilsila::jadeed();
        let miftah64 = u64::from(self.miftah);
        let mut azwaj: Vec<(Muashir, Muashir)> = Vec::with_capacity(self.madakhil.len());
        for madkhal in &self.madakhil {
            let ism = bina.nass(madkhal.masar.clone())?;
            let izaha = bina.sahih(sahih_min_u64(madkhal.izaha ^ miftah64))?;
            let tul = bina.sahih(sahih_min_u64(madkhal.tul ^ miftah64))?;
            let sabiqa = bina.bayt(madkhal.sabiqa.clone())?;
            let juz = bina.adhif(Uqda::Thulathi(vec![izaha, tul, sabiqa]))?;
            let qita = bina.adhif(Uqda::Qaima(vec![juz]))?;
            azwaj.push((ism, qita));
        }
        let jidhr = bina.adhif(Uqda::Kharita(azwaj))?;
        uktub_silsila(&bina.ikhtim(jidhr)?)
    }
}

/// A `u64` as the signed integer a pickle carries, saturating rather than
/// wrapping so a corrupt value stays visibly enormous instead of turning
/// negative and being read as a different number.
fn sahih_min_u64(qeema: u64) -> i64 {
    i64::try_from(qeema).unwrap_or(i64::MAX)
}

/// Compresses the index the way Ren'Py's own writer does.
fn udghut_zlib(khaam: &[u8]) -> Result<Vec<u8>, KhataNusus> {
    use std::io::Write as _;

    let mut daght = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    daght
        .write_all(khaam)
        .map_err(|sabab| KhataNusus::FakkFashil {
            sigha: SIGHA_RPA,
            tafsil: sabab.to_string(),
        })?;
    daght.finish().map_err(|sabab| KhataNusus::FakkFashil {
        sigha: SIGHA_RPA,
        tafsil: sabab.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Compiled scripts
//
// A `.rpyc` is a slot container: the magic, then a table of (slot, start,
// length) triples ended by a zero slot, then zlib-compressed payloads. Slot 1
// holds a pickle of the module's abstract syntax tree, and that is the only
// slot this module interprets. Every other slot is reported as present and its
// bytes are never decoded, because their contents are a Ren'Py implementation
// detail that has changed between point releases and this crate has no reason
// to depend on any of them.
//
// Writing a `.rpyc` is refused outright — there is no writer in this module and
// no path to one. The additive delivery makes it unnecessary, and emitting a
// byte-compiled AST for an engine whose AST classes change between releases is
// a way to produce a game that will not start.
// ---------------------------------------------------------------------------

/// The magic every compiled script from Ren'Py 6.18 onward opens with.
pub const SIHR_RPYC: &[u8] = b"RENPY RPC2";

/// The slot the abstract syntax tree lives in.
pub const KHANAT_SHAJARA: u32 = 1;

/// The name this format uses in a refusal.
const SIGHA_RPYC: &str = "RPYC";

/// One slot of a compiled script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KhanatRpyc {
    /// Which slot this is.
    pub raqm: u32,
    /// Where its stored bytes begin.
    pub bidaya: u32,
    /// How many stored bytes it has.
    pub tul: u32,
}

/// A compiled script's container, read without decoding any payload.
#[derive(Debug, Clone)]
pub struct MalafRpyc {
    /// Where it came from, for a refusal a person can read.
    pub masdar: String,
    /// Every slot the table declared, in order.
    pub khanat: Vec<KhanatRpyc>,
}

impl MalafRpyc {
    /// Reads the container framing.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::SihrGhayrMutabaq`] when the magic is absent — which for a
    /// `.rpyc` means a script from before Ren'Py 6.18, a bare zlib stream this
    /// build does not read — [`KhataNusus::MalafQaseer`] when the slot table
    /// runs past the file, and [`KhataNusus::HawiyaTalifa`] when a slot points
    /// outside it.
    pub fn iqra(bayt: &[u8], masdar: &str) -> Result<Self, KhataNusus> {
        if !bayt.starts_with(SIHR_RPYC) {
            return Err(KhataNusus::SihrGhayrMutabaq {
                masar: PathBuf::from(masdar),
                sigha: SIGHA_RPYC,
            });
        }
        let tul_malaf = tul_u64(bayt.len());
        let mut qari = Qari::jadeed(bayt);
        let _ = qari.nitaq("the RPYC magic", SIHR_RPYC.len())?;

        let mut khanat = Vec::new();
        loop {
            let raqm = u32::try_from(qari.adad_le("a slot number", 4)?).unwrap_or(0);
            if raqm == 0 {
                break;
            }
            let bidaya = u32::try_from(qari.adad_le("a slot offset", 4)?).unwrap_or(0);
            let tul = u32::try_from(qari.adad_le("a slot length", 4)?).unwrap_or(0);
            let nihaya = u64::from(bidaya)
                .checked_add(u64::from(tul))
                .ok_or_else(|| hawiya_talifa("a slot's end", u64::from(bidaya), tul_malaf))?;
            if nihaya > tul_malaf {
                return Err(KhataNusus::HawiyaTalifa {
                    sigha: SIGHA_RPYC,
                    haql: "a slot's end",
                    qeema: nihaya,
                    hadd: tul_malaf,
                });
            }
            khanat.push(KhanatRpyc { raqm, bidaya, tul });
            if khanat.len() > 64 {
                return Err(KhataNusus::HajmMufrit {
                    haql: "the RPYC slot table",
                    qeema: tul_u64(khanat.len()),
                    saqf: 64,
                });
            }
        }
        Ok(Self {
            masdar: masdar.to_owned(),
            khanat,
        })
    }

    /// The abstract syntax tree, decoded through the inert pickle machine.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::BunyaGhayrMutawaqqaa`] when the file carries no slot
    /// [`KHANAT_SHAJARA`], [`KhataNusus::FakkFashil`] when that slot will not
    /// decompress, [`KhataNusus::HajmMufrit`] when it expands past
    /// [`AQSA_RPYC`], and whatever [`iqra_silsila`] refuses.
    pub fn shajara(&self, bayt: &[u8]) -> Result<Silsila, KhataNusus> {
        let khana = self
            .khanat
            .iter()
            .find(|khana| khana.raqm == KHANAT_SHAJARA)
            .ok_or_else(|| KhataNusus::BunyaGhayrMutawaqqaa {
                malaf: self.masdar.clone(),
                haql: format!("no slot {KHANAT_SHAJARA} holding the syntax tree"),
            })?;
        let bidaya = hajm_usize(u64::from(khana.bidaya)).unwrap_or(usize::MAX);
        let nihaya = bidaya.saturating_add(hajm_usize(u64::from(khana.tul)).unwrap_or(0));
        let juz = bayt
            .get(bidaya..nihaya)
            .ok_or_else(|| KhataNusus::HawiyaTalifa {
                sigha: SIGHA_RPYC,
                haql: "the syntax tree slot",
                qeema: tul_u64(nihaya),
                hadd: tul_u64(bayt.len()),
            })?;
        let khaam = fukk_zlib(juz, AQSA_RPYC, SIGHA_RPYC)?;
        iqra_silsila(&khaam)
    }
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// What kind of string a record holds, because the three are delivered through
/// different Ren'Py mechanisms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NawSijill {
    /// A line of dialogue.
    Hiwar,
    /// A menu choice.
    Ikhtiyar,
    /// Interface text — a button, a label, anything inside `_()`.
    Mustalah,
}

impl NawSijill {
    /// The name for a log line and a generated file's comment.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Hiwar => "dialogue",
            Self::Ikhtiyar => "menu choice",
            Self::Mustalah => "interface",
        }
    }
}

/// One translatable string, with everything needed to deliver it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sijill {
    /// Which mechanism carries it.
    pub naw: NawSijill,
    /// Ren'Py's own translation identifier, when it is known exactly.
    ///
    /// [`None`] is the ordinary case and not a defect. Ren'Py derives the
    /// identifier from a hash of the statement's regenerated source, and
    /// regenerating that source byte for byte is something only the engine can
    /// do — so this crate never invents one. It is [`Some`] only when it was
    /// read out of a translation the game itself ships, where it is exact.
    pub muarrif: Option<String>,
    /// The speaker, when the statement named one.
    pub mutakallim: Option<String>,
    /// The text as the game has it.
    pub asl: String,
    /// Where it was found, for the diagnostics bundle.
    pub masdar: String,
    /// The line it was found on, when the source was text.
    pub satr: u32,
}

/// Every quoted string on a line, with the quote character that delimited it.
///
/// Written by hand rather than with a regular expression because Ren'Py's
/// escaping is Python's — a backslash escapes the next character, including a
/// quote and including another backslash — and a scanner that got that wrong
/// would silently split one string into two at an escaped quote and translate
/// half a sentence.
fn iqtibasat(satr: &str) -> Vec<(usize, String)> {
    let mut natija = Vec::new();
    let mut huruf = satr.char_indices();
    while let Some((mawqi, harf)) = huruf.next() {
        if harf == '#' {
            break;
        }
        if harf != '"' && harf != '\'' {
            continue;
        }
        let mut jism = String::new();
        let mut mughlaq = false;
        while let Some((_, dakhil)) = huruf.next() {
            if dakhil == '\\' {
                if let Some((_, taali)) = huruf.next() {
                    match taali {
                        'n' => jism.push('\n'),
                        't' => jism.push('\t'),
                        _ => jism.push(taali),
                    }
                }
                continue;
            }
            if dakhil == harf {
                mughlaq = true;
                break;
            }
            jism.push(dakhil);
        }
        if mughlaq {
            natija.push((mawqi, jism));
        }
    }
    natija
}

/// Whether a statement's text is worth translating at all.
///
/// A string that is empty, or that is nothing but markup and substitutions, is
/// a string a translator would be shown and could do nothing with. Filtering
/// those out here keeps the generated files readable and keeps a translation
/// memory from filling with `{w}` and `[player_name]`.
fn yustahaqq(nass: &str) -> bool {
    let munaqqa = nass.trim();
    if munaqqa.is_empty() {
        return false;
    }
    let mut fi_wasm = false;
    let mut fi_badil = false;
    munaqqa.chars().any(|harf| {
        match harf {
            '{' => fi_wasm = true,
            '}' => fi_wasm = false,
            '[' => fi_badil = true,
            ']' => fi_badil = false,
            _ => {
                return !fi_wasm && !fi_badil && !harf.is_whitespace() && harf != '.';
            },
        }
        false
    })
}

/// Scans `.rpy` source for everything translatable in it.
///
/// Deliberately a scanner and not a parser. Ren'Py's grammar is Python's plus
/// a screen language plus an ATL dialect, and a partial parser for all three
/// would be a large piece of code that is wrong in ways nobody notices until a
/// game uses the corner it got wrong. A scanner that reads statements it
/// recognises and ignores everything else is smaller, and its failure mode is
/// missing a string rather than mis-attributing one.
#[must_use]
pub fn iltiqat_min_rpy(nass: &str, masdar: &str) -> Vec<Sijill> {
    let mut sijillat = Vec::new();
    let mut fi_qaima = false;
    let mut masafat_qaima = 0_usize;

    for (raqm, satr) in nass.lines().enumerate() {
        let munaqqa = satr.trim_start();
        let masafat = satr.len().saturating_sub(munaqqa.len());
        let raqm_satr = u32::try_from(raqm.saturating_add(1)).unwrap_or(u32::MAX);
        if munaqqa.starts_with('#') || munaqqa.is_empty() {
            continue;
        }
        if fi_qaima && masafat <= masafat_qaima {
            fi_qaima = false;
        }
        // A `translate` block in the source is an existing translation, and
        // re-extracting it would offer a translator somebody else's Japanese as
        // the English to translate from.
        if munaqqa.starts_with("translate ") {
            continue;
        }
        if munaqqa.starts_with("menu") && munaqqa.ends_with(':') {
            fi_qaima = true;
            masafat_qaima = masafat;
            continue;
        }

        let mawadi = iqtibasat(munaqqa);
        if mawadi.is_empty() {
            continue;
        }

        // `_("...")` marks interface text explicitly, wherever it appears.
        if munaqqa.contains("_(") {
            for (_, jism) in &mawadi {
                if yustahaqq(jism) {
                    sijillat.push(Sijill {
                        naw: NawSijill::Mustalah,
                        muarrif: None,
                        mutakallim: None,
                        asl: jism.clone(),
                        masdar: masdar.to_owned(),
                        satr: raqm_satr,
                    });
                }
            }
            continue;
        }

        if fi_qaima && munaqqa.ends_with(':') {
            if let Some((_, jism)) = mawadi.first()
                && yustahaqq(jism)
            {
                sijillat.push(Sijill {
                    naw: NawSijill::Ikhtiyar,
                    muarrif: None,
                    mutakallim: None,
                    asl: jism.clone(),
                    masdar: masdar.to_owned(),
                    satr: raqm_satr,
                });
            }
            continue;
        }

        // A say statement is a line whose last element is a quoted string and
        // whose prefix is either empty or a bare speaker name.
        if let Some((mawqi, jism)) = mawadi.last() {
            let sabiq = munaqqa.get(..*mawqi).unwrap_or_default().trim();
            let mutakallim = if sabiq.is_empty() {
                None
            } else if sabiq.split_whitespace().count() <= 2
                && sabiq
                    .chars()
                    .all(|harf| harf.is_alphanumeric() || harf == '_')
            {
                Some(sabiq.to_owned())
            } else {
                continue;
            };
            if yustahaqq(jism) {
                sijillat.push(Sijill {
                    naw: NawSijill::Hiwar,
                    muarrif: None,
                    mutakallim,
                    asl: jism.clone(),
                    masdar: masdar.to_owned(),
                    satr: raqm_satr,
                });
            }
        }
    }
    sijillat
}

/// Whether an inert class name is one of Ren'Py's AST nodes, by leaf name.
fn huwa_uqda(ism: &str, waraqa: &str) -> bool {
    ism.rsplit('.').next().is_some_and(|akhir| akhir == waraqa)
        && (ism.starts_with("renpy.ast") || ism.starts_with("store.") || !ism.contains('.'))
}

/// Text out of an AST node's state dictionary, when the member is text.
fn nass_hala(silsila: &Silsila, hala: Option<Muashir>, miftah: &str) -> Option<String> {
    let hala = hala?;
    let qeema = silsila.min_kharita(hala, miftah)?;
    silsila.nass(qeema).map(str::to_owned)
}

/// Scans a compiled script's syntax tree for everything translatable in it.
///
/// Walks the arena in creation order rather than recursing, because a Ren'Py
/// AST refers to its own parents and a recursive walk over it does not
/// terminate. Creation order is emission order, so every node is visited
/// exactly once whatever the shape of the graph.
///
/// # Errors
///
/// Whatever [`MalafRpyc::iqra`] and [`MalafRpyc::shajara`] refuse: a missing
/// magic, a slot table that leaves the file, a payload that will not expand,
/// or a pickle opcode this build will not run.
pub fn iltiqat_min_rpyc(bayt: &[u8], masdar: &str) -> Result<Vec<Sijill>, KhataNusus> {
    let malaf = MalafRpyc::iqra(bayt, masdar)?;
    let silsila = malaf.shajara(bayt)?;
    let mut sijillat = Vec::new();

    for (_, uqda) in silsila.kul() {
        let Uqda::Kaain { ism, hala, .. } = uqda else {
            continue;
        };
        let satr = hala
            .and_then(|hala| silsila.min_kharita(hala, "linenumber"))
            .and_then(|qeema| silsila.sahih(qeema))
            .and_then(|qeema| u32::try_from(qeema).ok())
            .unwrap_or(0);

        if huwa_uqda(ism, "Say") {
            if let Some(asl) = nass_hala(&silsila, *hala, "what")
                && yustahaqq(&asl)
            {
                sijillat.push(Sijill {
                    naw: NawSijill::Hiwar,
                    muarrif: None,
                    mutakallim: nass_hala(&silsila, *hala, "who"),
                    asl,
                    masdar: masdar.to_owned(),
                    satr,
                });
            }
            continue;
        }

        if huwa_uqda(ism, "Menu") {
            let banud = hala
                .and_then(|hala| silsila.min_kharita(hala, "items"))
                .and_then(|qeema| silsila.anasir(qeema))
                .unwrap_or_default();
            for band in banud {
                let Some(huqul) = silsila.anasir(*band) else {
                    continue;
                };
                let Some(asl) = huqul.first().and_then(|udw| silsila.nass(*udw)) else {
                    continue;
                };
                if yustahaqq(asl) {
                    sijillat.push(Sijill {
                        naw: NawSijill::Ikhtiyar,
                        muarrif: None,
                        mutakallim: None,
                        asl: asl.to_owned(),
                        masdar: masdar.to_owned(),
                        satr,
                    });
                }
            }
            continue;
        }

        if huwa_uqda(ism, "TranslateString")
            && let Some(asl) = nass_hala(&silsila, *hala, "old")
            && yustahaqq(&asl)
        {
            sijillat.push(Sijill {
                naw: NawSijill::Mustalah,
                muarrif: None,
                mutakallim: None,
                asl,
                masdar: masdar.to_owned(),
                satr,
            });
        }
    }
    Ok(sijillat)
}

/// Recovers Ren'Py's own translation identifiers out of a translation the game
/// already ships.
///
/// This is the one way this crate ever learns an identifier, and it is exact.
/// The identifier is derived from the *source* statement, not from the target
/// language, so `translate french abc_1234abcd:` and
/// `translate arabic abc_1234abcd:` name the same statement. A game that ships
/// any translation at all therefore hands Taarib a complete, correct map from
/// identifier to source line — and a game that ships none gets the string
/// mechanism instead, which matches on the text itself.
///
/// Returns `(identifier, source text)` pairs in the order they appeared.
#[must_use]
pub fn muarrifat_min_tarjama(nass: &str) -> Vec<(String, String)> {
    let mut azwaj = Vec::new();
    let mut muarrif: Option<String> = None;
    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.starts_with('#') && muarrif.is_none() {
            // A comment outside a block is a comment. Inside one it is the
            // source line Ren'Py preserves above the translated statement, and
            // that is the whole point of reading this file.
            continue;
        }
        if let Some(baqi) = munaqqa.strip_prefix("translate ") {
            // `translate <language> <identifier>:` — the language varies and
            // the identifier does not, so the *last* word before the colon is
            // what is wanted.
            muarrif = baqi
                .trim_end_matches(':')
                .split_whitespace()
                .nth(1)
                .filter(|ism| *ism != "strings" && *ism != "python" && *ism != "style")
                .map(str::to_owned);
            continue;
        }
        let Some(hali) = muarrif.clone() else {
            continue;
        };
        // Inside a translation block the source line is the commented-out
        // original Ren'Py writes above the translated one.
        if let Some(taliq) = munaqqa.strip_prefix('#') {
            let mawadi = iqtibasat(taliq.trim());
            if let Some((_, jism)) = mawadi.last()
                && yustahaqq(jism)
            {
                azwaj.push((hali, jism.clone()));
                muarrif = None;
            }
        }
    }
    azwaj
}

// ---------------------------------------------------------------------------
// Delivery
//
// Everything below writes files that did not exist and modifies none that did.
// Uninstalling is `izal`, which deletes exactly the files carrying ALAMAT_TAARIB
// and refuses to touch anything else — a translator's own hand-written
// `game/tl/arabic/dialogue.rpy` is theirs, and losing it because Taarib decided
// the directory belonged to it would be the kind of damage this crate exists
// not to do.
// ---------------------------------------------------------------------------

/// Where the in-engine Python reads its settings from.
///
/// A data file, read with `json`, never evaluated. The generated `.rpy` does
/// nothing but import the package and call into it; every decision the patch
/// made travels as data.
pub const MALAF_BAYANAT: &str = "game/taarib/idad.json";

/// What the patch decided, as both the generator and the in-engine Python see
/// it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdadRenPy {
    /// Which adapter applies.
    ///
    /// Serialized as the rung number so the in-engine side branches on an
    /// integer it can compare rather than on a spelling it has to match.
    pub rutba: u8,
    /// How sure the probe was, 0..=100, carried so the game's own log can say
    /// so when a player reports that something looks wrong.
    pub thiqa: u8,
    /// The font the patch installed, relative to `game/`.
    pub khatt: String,
    /// A size override, when the patch made one.
    pub hajm: Option<u32>,
    /// Where the native library lives, relative to `game/`, for the takeover.
    pub dalil_jisr: String,
    /// The atlas the takeover blits from, relative to `game/`.
    pub lawha: String,
    /// The language name the translation was installed under.
    pub lugha: String,
    /// Every line the probe produced, so the game's log can explain itself.
    pub athar: Vec<String>,
}

impl IdadRenPy {
    /// Settings for one game.
    #[must_use]
    pub fn jadeed(masar: Masar, rutba: Rutba, thiqa: u8, khatt: impl Into<String>) -> Self {
        Self {
            rutba: match masar {
                Masar::Khadim => rutba as u8,
                Masar::Istila => Rutba::Istila as u8,
            },
            thiqa,
            khatt: khatt.into(),
            hajm: None,
            dalil_jisr: "taarib/jisr".to_owned(),
            lawha: "taarib/lawha.png".to_owned(),
            lugha: LUGHA.to_owned(),
            athar: Vec::new(),
        }
    }

    /// Which adapter these settings select.
    #[must_use]
    pub const fn masar(&self) -> Masar {
        if self.rutba >= Rutba::Istila as u8 {
            Masar::Istila
        } else {
            Masar::Khadim
        }
    }
}

/// A string as Ren'Py source spells it, with every character that would end the
/// literal escaped.
///
/// The escapes are Ren'Py's, which are Python's plus one: a literal `[` and a
/// literal `{` begin a substitution and a text tag respectively, and both are
/// escaped by doubling. A generator that missed those would turn an Arabic
/// sentence containing a bracket into a reference to a variable that does not
/// exist, and Ren'Py would raise at the moment the player reached that line.
///
/// **For displayed text only.** A value the engine never draws — a file name,
/// most of all — takes [`iqtibas_masar_rpy`] instead.
#[must_use]
pub fn iqtibas_rpy(nass: &str) -> String {
    let mut kharj = String::with_capacity(nass.len().saturating_add(2));
    kharj.push('"');
    for harf in nass.chars() {
        match harf {
            '"' => kharj.push_str("\\\""),
            '\\' => kharj.push_str("\\\\"),
            '\n' => kharj.push_str("\\n"),
            '\t' => kharj.push_str("\\t"),
            '[' => kharj.push_str("[["),
            '{' => kharj.push_str("{{"),
            _ => kharj.push(harf),
        }
    }
    kharj.push('"');
    kharj
}

/// A file name as Ren'Py source spells it: a Python literal and nothing more.
///
/// The one difference from [`iqtibas_rpy`] is `[` and `{`, and it decides
/// whether a font loads. Those two are doubled in text the engine *draws*,
/// where `[name]` is an interpolation and `{b}` a tag. A `font` property and a
/// name assigned inside an `init python` block are neither: they are Python
/// string literals handed to Ren'Py's asset loader, which resolves them against
/// `game/` verbatim. Doubling a bracket there asks the loader for a file whose
/// name carries two of them, and it finds none.
///
/// This is not hypothetical. Every Noto face the product bundles is a variable
/// font whose file name states its axis — `NotoNaskhArabic[wght].ttf` — so the
/// difference between the two functions is the whole Arabic face loading or the
/// game starting with the developer's Latin one still in place.
#[must_use]
pub fn iqtibas_masar_rpy(masar: &str) -> String {
    let mut kharj = String::with_capacity(masar.len().saturating_add(2));
    kharj.push('"');
    for harf in masar.chars() {
        match harf {
            '"' => kharj.push_str("\\\""),
            '\\' => kharj.push_str("\\\\"),
            '\n' => kharj.push_str("\\n"),
            '\t' => kharj.push_str("\\t"),
            _ => kharj.push(harf),
        }
    }
    kharj.push('"');
    kharj
}

/// Adds one of this module's four files, and its directory if Taarib made it.
///
/// [`Hafiz::ansha`] and not [`Hafiz::iktub`], because every file this module
/// delivers is a file Ren'Py has never heard of. That is the whole point of the
/// additive route: there is no original to preserve, uninstalling is a delete,
/// and a game update that rewrites what it ships leaves these alone. It also
/// means the guard refuses outright if something is already at the path — a
/// translator's own `taarib_mustalahat.rpy` is not this patch's to overwrite.
///
/// The write underneath is atomic, which matters here more than in most places:
/// Ren'Py compiles every file in `game/tl/arabic/` at startup, so a truncated
/// one is a game that will not launch.
fn adif_malaf(hafiz: &mut dyn Hafiz, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
    if let Some(walid) = masar.parent() {
        hafiz.ansha_mujallad(walid)?;
    }
    hafiz.ansha(masar, bayt)
}

/// The header every generated file opens with.
fn tarwisa_muwallada(wasf: &str) -> String {
    format!(
        "{ALAMAT_TAARIB}\n\
         # This file was generated by Taarib. {wasf}\n\
         # Deleting the game/tl/{LUGHA} directory removes the translation completely and\n\
         # restores the game exactly as it was: nothing outside it was modified.\n\n"
    )
}

/// Writes the file that registers the font, the direction and the adapter.
///
/// Four things, and no more:
///
/// 1. `config.font_replacement_map` — the engine's own documented redirection
///    from a font the game asks for to one it gets. Every face the game names
///    resolves through it, including faces set inside screens this crate never
///    sees, which is why it is used in preference to editing styles one by one.
/// 2. `style.default.font` and the `gui` variables, for the text that does not
///    go through a replacement because it names no font at all.
/// 3. `style.default.text_align` and the layout properties, which is the
///    *direction* half: an engine that shapes Arabic correctly still aligns the
///    paragraph to the left until it is told otherwise, and left-aligned Arabic
///    is wrong even when every letter in it joins.
/// 4. One call into the in-engine package, which reads its settings from
///    [`MALAF_BAYANAT`] as data.
///
/// Every property is set through a `hasattr` guard on the Python side rather
/// than assumed here, because Ren'Py's configuration surface is large and
/// version-dependent, and a generated file that named a variable this engine
/// does not have would raise during startup and take the game with it.
///
/// Both files are delivered as *additions* through [`Hafiz::ansha`], so
/// uninstalling deletes them and there is no original to hold. That also means
/// the guard refuses outright if something is already at either path: a
/// translator's own file in `game/tl/arabic/` is not this patch's to overwrite.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] when the file or its directory cannot be written,
/// [`KhataNusus::NuskhaMafquda`] when something already occupies one of the two
/// paths or the manifest cannot be flushed first, and
/// [`KhataNusus::HimlMarfud`] when the settings could not be serialized — a rung
/// declining rather than the adapter failing, so the caller descends.
pub fn iktub_idad(
    hafiz: &mut dyn Hafiz,
    jidhr: &Path,
    idad: &IdadRenPy,
) -> Result<Vec<PathBuf>, KhataNusus> {
    let bayanat = serde_json::to_vec_pretty(idad).map_err(|sabab| KhataNusus::HimlMarfud {
        alia: "the Ren'Py settings file",
        sabab: sabab.to_string(),
    })?;
    let masar_bayanat = masar_bila_hala(jidhr, MALAF_BAYANAT);
    adif_malaf(hafiz, &masar_bayanat, &bayanat)?;

    let masar = idad.masar();
    let mut nass = tarwisa_muwallada(
        "It registers the Arabic font and the text direction, and hands the rest to the \
         taarib_renpy package.",
    );
    let _ = writeln!(
        nass,
        "# Path: {} ({}), probe confidence {}.",
        masar.ism(),
        if masar.yukallif() {
            "text is drawn, not typeset by the engine"
        } else {
            "configuration"
        },
        idad.thiqa
    );
    for satr in &idad.athar {
        let _ = writeln!(nass, "#   {satr}");
    }
    nass.push('\n');

    // An empty name is not "no font". `sajjil_khatt` assigns whatever it is
    // handed to every `gui` font variable and to `style.default.font`, so
    // registering an empty string is a game with no font rather than a game with
    // its own. A patch that ships no face therefore registers none, and the
    // direction half — which is the half that is wrong even on an engine that
    // joins every letter correctly — is still installed.
    let khatt = idad.khatt.replace('\\', "/");
    let lahu_khatt = !khatt.trim().is_empty();
    let _ = writeln!(nass, "init -100 python:");
    let _ = writeln!(nass, "    import taarib_renpy");
    let _ = writeln!(nass, "    taarib_renpy.rakkib(renpy.config.gamedir)");
    nass.push('\n');
    let _ = writeln!(nass, "init 100 python:");
    if lahu_khatt {
        let _ = writeln!(nass, "    _taarib_khatt = {}", iqtibas_masar_rpy(&khatt));
        let _ = writeln!(nass, "    taarib_renpy.sajjil_khatt(_taarib_khatt)");
    }
    if let Some(hajm) = idad.hajm {
        let _ = writeln!(nass, "    taarib_renpy.sajjil_hajm({hajm})");
    }
    let _ = writeln!(nass, "    taarib_renpy.sajjil_ittijah()");
    nass.push('\n');

    // Style overrides live in a translate block so the engine applies them only
    // while Arabic is selected. A player who switches back to the original
    // language gets the original styles, untouched, with no restart.
    let _ = writeln!(nass, "translate {LUGHA} style default:");
    if lahu_khatt {
        let _ = writeln!(nass, "    font {}", iqtibas_masar_rpy(&khatt));
    }
    if let Some(hajm) = idad.hajm {
        let _ = writeln!(nass, "    size {hajm}");
    }
    let _ = writeln!(nass, "    text_align 1.0");
    let _ = writeln!(nass, "    layout \"subtitle\"");
    let _ = writeln!(nass, "    language \"unicode\"");
    nass.push('\n');
    for uslub in [
        "say_dialogue",
        "say_label",
        "input_text",
        "menu_choice_button_text",
    ] {
        let _ = writeln!(nass, "translate {LUGHA} style {uslub}:");
        if lahu_khatt {
            let _ = writeln!(nass, "    font {}", iqtibas_masar_rpy(&khatt));
        }
        let _ = writeln!(nass, "    text_align 1.0");
        let _ = writeln!(nass, "    layout \"subtitle\"");
        nass.push('\n');
    }

    let masar_idad = masar_bila_hala(jidhr, MALAF_IDAD);
    adif_malaf(hafiz, &masar_idad, nass.as_bytes())?;
    Ok(vec![masar_bayanat, masar_idad])
}

/// Writes the interface strings as a `translate arabic strings:` block.
///
/// The string mechanism matches on the text itself rather than on an
/// identifier, which is exactly right for interface text: the same
/// `"Start Game"` appears in three screens and should be translated once. It is
/// also what carries dialogue when the game ships no translation of its own to
/// recover identifiers from, and the generated file says which case it is in.
///
/// Only strings the patch actually has an Arabic for are emitted. A rule whose
/// `new` repeats its `old` is not a translation and is not harmless: Ren'Py
/// matches on the source text, so an identity rule shadows the same string in
/// any other translation file the game ships and makes it permanently
/// untranslatable. Anything `mutarjim` has no answer for is left for the game's
/// own text to render, and counted.
///
/// Returns the path written, how many rules it holds, and how many source
/// strings had no translation.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] when the file cannot be written, and
/// [`KhataNusus::NuskhaMafquda`] when something is already at the path — this
/// file is an addition, and overwriting somebody's work is not one.
pub fn iktub_mustalahat(
    hafiz: &mut dyn Hafiz,
    jidhr: &Path,
    sijillat: &[Sijill],
    mutarjim: &dyn Mutarjim,
) -> Result<(PathBuf, usize, usize), KhataNusus> {
    let mut nass = tarwisa_muwallada(
        "It holds the strings Ren'Py matches by content rather than by statement \
         identifier: interface text, menu choices, and any dialogue whose identifier \
         could not be recovered exactly.",
    );
    let _ = writeln!(nass, "translate {LUGHA} strings:\n");

    // Deduplicated on the source text, keeping the first record's provenance.
    // The same button label appears in a dozen screens and Ren'Py matches on
    // the text, so emitting it a dozen times would be a dozen identical rules
    // and one warning per duplicate at compile time.
    let mut mashhud: BTreeMap<&str, &Sijill> = BTreeMap::new();
    for sijill in sijillat {
        if sijill.muarrif.is_some() && sijill.naw == NawSijill::Hiwar {
            continue;
        }
        let _ = mashhud.entry(sijill.asl.as_str()).or_insert(sijill);
    }

    let mut adad: usize = 0;
    let mut matruka: usize = 0;
    for (asl, sijill) in &mashhud {
        let Some(tarjama) = mutarjim.tarjim(asl) else {
            matruka = matruka.saturating_add(1);
            continue;
        };
        let _ = writeln!(
            nass,
            "    # {} — {}:{}",
            sijill.naw.ism(),
            sijill.masdar,
            sijill.satr
        );
        let _ = writeln!(nass, "    old {}", iqtibas_rpy(asl));
        let _ = writeln!(nass, "    new {}\n", iqtibas_rpy(tarjama));
        adad = adad.saturating_add(1);
    }
    if matruka > 0 {
        let _ = writeln!(
            nass,
            "# {matruka} string(s) found in this game have no translation in the patch and are \
             deliberately absent from this file: an `old`/`new` pair repeating itself would \
             shadow the string rather than translate it."
        );
    }

    let masar = masar_bila_hala(jidhr, MALAF_MUSTALAHAT);
    adif_malaf(hafiz, &masar, nass.as_bytes())?;
    Ok((masar, adad, matruka))
}

/// Writes the dialogue whose Ren'Py identifiers are known exactly.
///
/// One `translate arabic <identifier>:` block per statement, which is the
/// engine's own dialogue mechanism and the one that survives the game
/// substituting a variable into the line: the block replaces the statement, not
/// the rendered text, so `[player_name]` still expands.
///
/// Records with no identifier are not written here and are not lost — they go
/// through [`iktub_mustalahat`] instead. That split is the whole reason
/// [`Sijill::muarrif`] is an [`Option`] rather than a guess.
///
/// A statement the patch has no Arabic for is omitted rather than written back
/// in its own language. An identifier block whose body repeats the original is
/// not a translation: Ren'Py would replace the statement with an identical one
/// and the line would be marked as translated for the rest of the project's
/// life, which is how a game ends up half-Arabic and reporting itself complete.
///
/// The comment above each block carries the source line, so the generated file
/// is still readable beside the game's own script.
///
/// Returns the path written, how many blocks it holds, and how many statements
/// with a recovered identifier had no translation.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] when the file cannot be written, and
/// [`KhataNusus::NuskhaMafquda`] when something is already at the path — this
/// file is an addition, and overwriting somebody's work is not one.
pub fn iktub_hiwar(
    hafiz: &mut dyn Hafiz,
    jidhr: &Path,
    sijillat: &[Sijill],
    mutarjim: &dyn Mutarjim,
) -> Result<(PathBuf, usize, usize), KhataNusus> {
    let mut nass = tarwisa_muwallada(
        "It holds the dialogue Ren'Py matches by statement identifier. Every identifier \
         here was read out of a translation this game already ships, never derived: the \
         identifier is a hash of the statement's own source, which only the engine can \
         regenerate.",
    );

    let mut mashhud: BTreeMap<&str, &Sijill> = BTreeMap::new();
    for sijill in sijillat {
        let Some(muarrif) = sijill.muarrif.as_deref() else {
            continue;
        };
        let _ = mashhud.entry(muarrif).or_insert(sijill);
    }

    let mut adad: usize = 0;
    let mut matruka: usize = 0;
    for (muarrif, sijill) in &mashhud {
        let Some(tarjama) = mutarjim.tarjim(&sijill.asl) else {
            matruka = matruka.saturating_add(1);
            continue;
        };
        let _ = writeln!(nass, "# {}:{}", sijill.masdar, sijill.satr);
        let _ = writeln!(nass, "translate {LUGHA} {muarrif}:\n");
        let _ = writeln!(nass, "    # {}", iqtibas_rpy(&sijill.asl));
        match sijill.mutakallim.as_deref() {
            Some(mutakallim) if !mutakallim.is_empty() => {
                let _ = writeln!(nass, "    {mutakallim} {}\n", iqtibas_rpy(tarjama));
            },
            _ => {
                let _ = writeln!(nass, "    {}\n", iqtibas_rpy(tarjama));
            },
        }
        adad = adad.saturating_add(1);
    }
    if matruka > 0 {
        let _ = writeln!(
            nass,
            "# {matruka} statement(s) with a recovered identifier have no translation in the \
             patch and are deliberately absent: a block repeating the original would mark the \
             line translated for the rest of the project's life."
        );
    }

    let masar = masar_bila_hala(jidhr, MUJALLAD_TARJAMA).join("taarib_hiwar.rpy");
    adif_malaf(hafiz, &masar, nass.as_bytes())?;
    Ok((masar, adad, matruka))
}

/// Removes everything this module wrote, and nothing else.
///
/// Every candidate is opened and checked for [`ALAMAT_TAARIB`] before it is
/// deleted. A file in `game/tl/arabic/` that does not carry the marker is
/// somebody's own work, and the uninstall leaves it exactly where it is — which
/// is why this returns the files it removed rather than a count: the caller
/// records them, and a translator can see that their own file survived.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] when a file that does carry the marker cannot be
/// removed, and [`KhataNusus::NuskhaMafquda`] when a file carries the marker
/// but the manifest does not record Taarib as having added it — which is a
/// refusal to delete, not a failure to. A file that cannot be *read* is left
/// alone rather than deleted, because a file whose marker could not be checked
/// is a file whose ownership is unknown.
pub fn izal(hafiz: &mut dyn Hafiz, jidhr: &Path) -> Result<Vec<PathBuf>, KhataNusus> {
    let mut muzala = Vec::new();
    let mut murashahat: Vec<PathBuf> = [MALAF_BAYANAT, MALAF_IDAD, MALAF_MUSTALAHAT]
        .iter()
        .map(|nisbi| masar_bila_hala(jidhr, nisbi))
        .collect();
    let mujallad = masar_bila_hala(jidhr, MUJALLAD_TARJAMA);
    if let Ok(qaima) = fs::read_dir(&mujallad) {
        for madkhal in qaima.take(AQSA_MADAKHIL).flatten() {
            let masar = madkhal.path();
            if masar.is_file() && !murashahat.contains(&masar) {
                murashahat.push(masar);
            }
        }
    }

    for masar in murashahat {
        if !masar.is_file() {
            continue;
        }
        let Some(bayt) = iqra_muhaddad(&masar, 4096) else {
            continue;
        };
        // The settings file is JSON and cannot carry a comment marker, so it is
        // matched on its own name instead — the one exception, and it is the
        // one file whose name this module owns outright.
        let malna =
            masar.ends_with("idad.json") || String::from_utf8_lossy(&bayt).contains(ALAMAT_TAARIB);
        if !malna {
            continue;
        }
        // Two independent conditions have to agree before anything is
        // deleted: the file carries this module's marker, *and* the manifest
        // records Taarib as having added it. Either alone is one mistake away
        // from deleting somebody's work — a translator who copied a generated
        // file as a starting point keeps the marker in it, and a manifest
        // edited by hand can name anything.
        if hafiz.ihdhif(&masar)? {
            muzala.push(masar);
        }
    }

    // Only when it is empty, and only when Taarib created it. A directory
    // holding somebody else's translation stays, and so does its content.
    let _ = hafiz.ihdhif_mujallad(&mujallad)?;
    Ok(muzala)
}
