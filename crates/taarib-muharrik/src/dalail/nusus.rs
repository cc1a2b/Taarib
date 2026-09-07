//! نصوص — the six engines whose content is script or data rather than compiled
//! code, and the one detector in the probe that has to look inside a wrapper.
//!
//! RPG Maker MV, RPG Maker MZ, RPG Maker VX Ace, Ren'Py, GameMaker Studio, and
//! the Chromium wrappers — Electron and NW.js. They belong together because
//! Phase 10 patches all six by rewriting their own data rather than by taking
//! over a rendering pipeline, and because every one of them is found by reading
//! a file the engine itself must find at startup. A marker the runtime has to
//! locate before it can draw a frame is a marker near the top of the directory,
//! spelled the same way in every export, in a format that has to stay readable.
//! That is why this family is the cheapest and the most certain part of the
//! probe, and why its evidence carries the highest weights in the crate.
//!
//! ## Five detectors, six engines
//!
//! [`HadafNusus`] names what one instance is hunting. MV and MZ share a
//! detector because they share a directory shape and are told apart by which
//! core script is present, not by which files exist; the other four each get
//! their own. [`crate::tahdid`] runs all five and combines what they return,
//! so a game that is two things at once reports as two things at once.
//!
//! **Registration.** Unlike the other detector files, this one contributes five
//! detectors rather than one, so it is registered with [`FahisNusus::jamee`] and
//! not with a single no-argument constructor. That is the whole reason the type
//! carries a target: one `HasilatFahs` holds one engine family, and a game that
//! is an RPG Maker project inside NW.js has two.
//!
//! ## The wrapper is not the game, and the game is not the wrapper
//!
//! An RPG Maker MZ game is JavaScript running inside NW.js. A Godot HTML5
//! export shipped on the desktop is a Godot game running inside Electron. A
//! visual novel written directly against the DOM is only Electron. All three
//! ship `chrome_100_percent.pak`, and only the third of them is *just* a
//! browser.
//!
//! So [`HadafNusus::Ghilaf`] reports Electron with its own weight and never
//! suppresses the inner engine, and the inner detectors look inside
//! `resources/app/` as well as at the game root so that a repackaged RPG Maker
//! project is still found. Both answers reach [`crate::tahdid`] and both are
//! true, because they answer different questions:
//!
//! - **The inner engine decides how text is drawn.** `Bitmap.prototype.drawText`
//!   on a canvas, a Ren'Py displayable, `draw_text` against a GameMaker font
//!   resource — that is what Phase 10's runtime has to take over, and it is a
//!   property of the game, not of the shell around it.
//! - **The wrapper decides how the patch is installed.** An `app.asar` has to be
//!   unpacked, injected into, and repacked; a plain `www/` directory is written
//!   into directly. That is a property of the shell, not of the game.
//!
//! A detector that resolved the ambiguity here would be choosing one of those
//! two questions to answer and silently dropping the other. Resolution has the
//! whole picture and this does not, which is the rule the whole crate is built
//! on.
//!
//! ## Bounded, always
//!
//! A `data.win` can be four gigabytes. An `app.asar` can be two. Neither is ever
//! read whole:
//!
//! - The GameMaker reader reads the eight-byte `FORM` header and then walks the
//!   chunk table by seeking — name, length, skip — never touching a chunk
//!   payload except the first hundred and sixty bytes of `GEN8` and the kilobyte
//!   of `STRG` that holds the game's name.
//! - The asar reader reads the sixteen-byte pickle framing, refuses a header
//!   that claims more than [`AQSA_TARWISAT_ASAR`], checks the claim against the
//!   real file length, and then reads exactly the JSON directory it declared.
//! - Everything parsed as JSON is capped at [`AQSA_MALAF_KAMIL`], and a JSON
//!   file that will not parse is the absence of evidence rather than an error.
//!   `System.json`, `plugins.js` and `package.json` are edited by hand by game
//!   developers all day long, and a probe that failed on a trailing comma would
//!   fail on a great many real games.
//!
//! Every offset is checked against the real file length before it is used, and
//! every arithmetic step that could leave the file is done with `checked_add`,
//! because a corrupt length field in an untrusted container is the ordinary
//! case here rather than the exotic one.
//!
//! ## What each detector reads
//!
//! - `nusus:rpgmaker` — conclusive: `js/rmmz_core.js` or `www/js/rpg_core.js`.
//!   Supporting: `data/System.json`, `js/plugins.js`, the project file, and the
//!   NW.js shape around them.
//! - `nusus:vxace` — conclusive: the RGSSAD version 3 header of `Game.rgss3a`.
//!   Supporting: `Game.ini`'s `Library=` line and `Data/*.rvdata2`.
//! - `nusus:renpy` — conclusive: `version_tuple` in `renpy/__init__.py`.
//!   Supporting: `lib/py3-*` build names, `game/*.rpa`, `game/script.rpyc`.
//! - `nusus:gamemaker` — conclusive: a `FORM` header with a `GEN8` chunk.
//!   Supporting: the rest of the chunk table and `GEN8`'s runtime version.
//! - `nusus:ghilaf` — conclusive: an `app.asar` header, or `nw.pak`.
//!   Supporting: `icudtl.dat`, `chrome_100_percent.pak`, the V8 snapshots.
//!
//! `package.json` is deliberately absent from every conclusive line. Every
//! JavaScript project on earth has one, and a game directory containing nothing
//! else is a game directory that has told you nothing.

use std::fs::{self, File};
use std::io::{ErrorKind, Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;
use taarib_mustalahat::muharrik::{
    AilatMuharrik, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel,
};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{Mimariya, NizamTashghil};

use crate::dalail::imtidad;
use crate::fahs::{Fahis, HAJM_TARWISA, HasilatFahs, SiyaqFahs};
use crate::khata::KhataMuharrik;

/// The most bytes read from a file that has to be parsed in full.
///
/// `System.json`, `plugins.js` and an Electron `package.json` are documents: a
/// truncated one does not parse, and a truncated parse is a false negative
/// rather than a partial answer. Four megabytes is far above anything these
/// files reach in practice — a `System.json` with every switch and variable
/// named runs to a few hundred kilobytes — and far below anything that would
/// make a probe expensive.
pub const AQSA_MALAF_KAMIL: u64 = 4 * 1024 * 1024;

/// The most bytes read from a text file whose value is one line or one
/// assignment.
///
/// `Game.ini`, `renpy/__init__.py`, and the header of a core script. The value
/// being looked for is in the first few hundred bytes of all of them; the
/// ceiling exists for the file that was replaced with something else entirely.
pub const AQSA_NASS: u64 = 256 * 1024;

/// The largest asar directory header this reader will accept.
///
/// The header length is read out of the file being examined, which makes it
/// attacker-controlled input in the ordinary sense: a game directory is not a
/// trusted source. A real Electron application with fifty thousand packed files
/// has a header of a couple of megabytes; eight is generous. A file claiming
/// more is not read, and the refusal is recorded as evidence so that a genuinely
/// enormous application is visible in diagnostics rather than silently skipped.
pub const AQSA_TARWISAT_ASAR: u64 = 8 * 1024 * 1024;

/// How many `FORM` chunks the GameMaker reader will walk before stopping.
///
/// A real `data.win` has between fifteen and forty. A file whose chunk table
/// loops or whose lengths are nonsense would otherwise walk forever, so the
/// walk is bounded rather than trusted.
pub const AQSA_QITA: usize = 256;

/// How many entries a single directory listing here will look at.
///
/// Well under the crate's own [`crate::fahs::AQSA_MADAKHIL`] ceiling, because
/// these listings are per-directory and one of them is a Ren'Py `game/` folder,
/// which on a large visual novel holds tens of thousands of loose images.
pub const AQSA_MADAKHIL_MUJALLAD: usize = 4_096;

/// How many names one observation will quote before it starts summarising.
const AQSA_ASMAA: usize = 32;

/// How long a quoted value may be before it is cut.
const AQSA_IQTIBAS: usize = 160;

/// The Ren'Py release from which the engine bundles `HarfBuzz` and `FriBidi` and can
/// shape and reorder Arabic by itself.
///
/// Below this, Ren'Py's text layout is a per-character blit with no joining, no
/// ligatures and no bidirectional resolution, and Phase 10 has to install its
/// own text filter and displayable. At or above it, the adapter sets the
/// language and direction properties and lets the engine do the work. Those are
/// two entirely different adapters, which is the whole reason this detector
/// exists.
pub const HADD_TASHKEEL_RENPY: (u16, u16) = (7, 4);

/// Which script engine one instance of [`FahisNusus`] is looking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HadafNusus {
    /// RPG Maker MV and MZ, which share a directory shape and are told apart by
    /// which core script the export shipped.
    RpgMakerJs,
    /// RPG Maker VX Ace, and with it the RGSSAD version 3 archive.
    VxAce,
    /// Ren'Py, where the point of the probe is the version rather than the
    /// identification.
    RenPy,
    /// GameMaker Studio, in all three of its container filenames.
    GameMaker,
    /// The Chromium shell — Electron or NW.js — around whatever it wraps.
    Ghilaf,
}

impl HadafNusus {
    /// The detector's stable name, as it appears in the evidence trail.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::RpgMakerJs => "nusus:rpgmaker",
            Self::VxAce => "nusus:vxace",
            Self::RenPy => "nusus:renpy",
            Self::GameMaker => "nusus:gamemaker",
            Self::Ghilaf => "nusus:ghilaf",
        }
    }

    /// How the game's code runs, when this detector finds its engine at all.
    #[must_use]
    pub const fn khalfiya(self) -> KhalfiyaBarmajiya {
        match self {
            Self::RpgMakerJs | Self::Ghilaf => KhalfiyaBarmajiya::JavaScript,
            Self::VxAce => KhalfiyaBarmajiya::Ruby,
            Self::RenPy => KhalfiyaBarmajiya::Python,
            Self::GameMaker => KhalfiyaBarmajiya::GameMakerVm,
        }
    }
}

/// One script-engine detector.
///
/// Holds nothing but which engine it is looking for, so constructing one costs
/// nothing and running two of them in either order gives the same answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FahisNusus {
    hadaf: HadafNusus,
}

impl FahisNusus {
    /// A detector for one script engine.
    #[must_use]
    pub const fn jadeed(hadaf: HadafNusus) -> Self {
        Self { hadaf }
    }

    /// What this instance is looking for.
    #[must_use]
    pub const fn hadaf(self) -> HadafNusus {
        self.hadaf
    }

    /// Every script-engine detector, in the order they should run.
    ///
    /// The inner engines run before the wrapper so that a diagnostics bundle
    /// reads in the order a person would investigate: what the game is, and then
    /// what it is wrapped in.
    #[must_use]
    pub const fn jamee() -> [Self; 5] {
        [
            Self::jadeed(HadafNusus::RpgMakerJs),
            Self::jadeed(HadafNusus::VxAce),
            Self::jadeed(HadafNusus::RenPy),
            Self::jadeed(HadafNusus::GameMaker),
            Self::jadeed(HadafNusus::Ghilaf),
        ]
    }
}

impl Fahis for FahisNusus {
    fn ism(&self) -> &'static str {
        self.hadaf.ism()
    }

    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs> {
        if !siyaq.jidhr.is_dir() {
            return Err(KhataMuharrik::JidhrMafqud {
                jidhr: siyaq.jidhr.to_path_buf(),
            }
            .into());
        }
        Ok(match self.hadaf {
            HadafNusus::RpgMakerJs => rpg_maker(siyaq),
            HadafNusus::VxAce => vx_ace(siyaq),
            HadafNusus::RenPy => renpy(siyaq),
            HadafNusus::GameMaker => gamemaker(siyaq),
            HadafNusus::Ghilaf => ghilaf(siyaq),
        })
    }
}

// ---------------------------------------------------------------------------
// Bounded reading
//
// Every read in this module goes through one of these. None of them returns an
// error: a file that is missing, denied, truncated, or lying about its own
// length is the absence of evidence, and the caller records what it did see.
// ---------------------------------------------------------------------------

/// The size of a file, or `None` when it is not a readable ordinary file.
fn hajm(masar: &Path) -> Option<u64> {
    let bayanat = fs::metadata(masar).ok()?;
    bayanat.is_file().then_some(bayanat.len())
}

/// Reads at most `hadd` bytes from the start of a file.
fn iqra_muhaddad(masar: &Path, hadd: u64) -> Option<Vec<u8>> {
    let malaf = File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(hadd).read_to_end(&mut bayt).ok()?;
    Some(bayt)
}

/// Reads `tul` bytes from `izaha`, after checking that the range is inside the
/// file.
///
/// The bounds check is against the length observed when the file was opened, and
/// the read is still allowed to return fewer bytes than asked for: a game's data
/// file can be replaced by an updater between the two, and a short read is a
/// short read rather than a reason to invent bytes.
fn iqra_izaha(malaf: &mut File, tul_malaf: u64, izaha: u64, tul: usize) -> Option<Vec<u8>> {
    let nihaya = izaha.checked_add(u64::try_from(tul).ok()?)?;
    if nihaya > tul_malaf {
        return None;
    }
    let _ = malaf.seek(SeekFrom::Start(izaha)).ok()?;
    let mut bayt = vec![0_u8; tul];
    let mut mala = 0_usize;
    while mala < tul {
        match malaf.read(bayt.get_mut(mala..)?) {
            Ok(0) => break,
            Ok(adad) => mala = mala.checked_add(adad)?,
            Err(khata) if khata.kind() == ErrorKind::Interrupted => {},
            Err(_) => return None,
        }
    }
    bayt.truncate(mala);
    Some(bayt)
}

/// Decodes bytes to text, honouring a byte order mark.
///
/// `Game.ini` is written by a Japanese editor and turns up in three encodings;
/// a `package.json` produced on Windows can carry a UTF-8 mark that makes a
/// strict parser reject the whole document. Neither is a reason to see nothing.
fn nass_min_bayt(bayt: &[u8]) -> String {
    match bayt.get(..2) {
        Some([0xFF, 0xFE]) => {
            let wahdat: Vec<u16> = bayt
                .get(2..)
                .unwrap_or_default()
                .chunks_exact(2)
                .filter_map(|juz| <[u8; 2]>::try_from(juz).ok())
                .map(u16::from_le_bytes)
                .collect();
            String::from_utf16_lossy(&wahdat)
        },
        Some([0xFE, 0xFF]) => {
            let wahdat: Vec<u16> = bayt
                .get(2..)
                .unwrap_or_default()
                .chunks_exact(2)
                .filter_map(|juz| <[u8; 2]>::try_from(juz).ok())
                .map(u16::from_be_bytes)
                .collect();
            String::from_utf16_lossy(&wahdat)
        },
        _ => String::from_utf8_lossy(bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bayt))
            .into_owned(),
    }
}

/// Reads at most `hadd` bytes of a file as text.
fn iqra_nass(masar: &Path, hadd: u64) -> Option<String> {
    iqra_muhaddad(masar, hadd).map(|bayt| nass_min_bayt(&bayt))
}

/// Reads a file as JSON, bounded.
///
/// A document that will not parse returns `None`, which is the same answer as a
/// document that is not there. These files are hand-edited by game developers —
/// a trailing comma in `plugins.js` and a comment in `package.json` are both
/// ordinary — and a probe that treated a malformed one as a failure would fail
/// on games that run perfectly well.
fn iqra_json(masar: &Path, hadd: u64) -> Option<Value> {
    let bayt = iqra_muhaddad(masar, hadd)?;
    let munaqqa = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bayt);
    serde_json::from_slice(munaqqa).ok()
}

/// One directory's entries as `(name, is_directory)`, bounded and never
/// recursive.
///
/// Symbolic links report as neither file nor directory, which is deliberate: a
/// link inside a game directory is not followed here, so a game that ships a
/// link to somewhere else on the machine cannot make a probe describe an engine
/// the game does not use.
fn madakhil(masar: &Path) -> Vec<(String, bool)> {
    let Ok(qaima) = fs::read_dir(masar) else {
        return Vec::new();
    };
    qaima
        .take(AQSA_MADAKHIL_MUJALLAD)
        .flatten()
        .filter_map(|madkhal| {
            let ism = madkhal.file_name().into_string().ok()?;
            Some((ism, madkhal.file_type().is_ok_and(|naw| naw.is_dir())))
        })
        .collect()
}

/// The first candidate that exists as a file, with the relative path it was
/// named by.
fn awwal_malaf(siyaq: &SiyaqFahs<'_>, murashahat: &[&str]) -> Option<(String, PathBuf)> {
    murashahat.iter().find_map(|nisbi| {
        let masar = siyaq.dakhil(nisbi).ok()?;
        masar.is_file().then(|| ((*nisbi).to_owned(), masar))
    })
}

/// The first candidate that exists as a directory.
fn awwal_mujallad(siyaq: &SiyaqFahs<'_>, murashahat: &[&str]) -> Option<(String, PathBuf)> {
    murashahat.iter().find_map(|nisbi| {
        let masar = siyaq.dakhil(nisbi).ok()?;
        masar.is_dir().then(|| ((*nisbi).to_owned(), masar))
    })
}

/// A path rendered relative to the game root, with forward slashes, for the
/// `mawqi` field of a piece of evidence.
fn mawqi_nisbi(siyaq: &SiyaqFahs<'_>, masar: &Path) -> String {
    masar
        .strip_prefix(siyaq.jidhr)
        .unwrap_or(masar)
        .to_string_lossy()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------
// Small conversions
// ---------------------------------------------------------------------------

/// A little-endian `u32` at a byte offset, bounds-checked.
fn u32_le(bayt: &[u8], izaha: usize) -> Option<u32> {
    let nihaya = izaha.checked_add(4)?;
    bayt.get(izaha..nihaya)
        .and_then(|juz| <[u8; 4]>::try_from(juz).ok())
        .map(u32::from_le_bytes)
}

/// A little-endian `u64` at a byte offset, bounds-checked.
fn u64_le(bayt: &[u8], izaha: usize) -> Option<u64> {
    let nihaya = izaha.checked_add(8)?;
    bayt.get(izaha..nihaya)
        .and_then(|juz| <[u8; 8]>::try_from(juz).ok())
        .map(u64::from_le_bytes)
}

/// A signed little-endian `i32` at a byte offset, bounds-checked.
fn i32_le(bayt: &[u8], izaha: usize) -> Option<i32> {
    u32_le(bayt, izaha).map(|khaam| i32::from_le_bytes(khaam.to_le_bytes()))
}

/// A dotted version string parsed into the triple, keeping the original.
///
/// The raw string is what a signature database keys on, so it survives whatever
/// the parse makes of it — `1.6.2`, `7.4.11`, `2.3.1.542` and `RGSS300` all keep
/// the spelling they were found with.
fn isdar_min_nass(khaam: &str) -> IsdarMuharrik {
    let mut ajzaa = khaam
        .split(['.', '-', '+', '_'])
        .filter_map(|juz| juz.trim().parse::<u16>().ok());
    IsdarMuharrik {
        kabir: ajzaa.next().unwrap_or(0),
        sagheer: ajzaa.next().unwrap_or(0),
        tasheeh: ajzaa.next().unwrap_or(0),
        khaam: khaam.to_owned(),
        mushtaqq: false,
    }
}

/// A value quoted into an observation: control characters flattened, length
/// capped, and a marker left when something was cut.
fn iqtibas(khaam: &str) -> String {
    let munaqqa: String = khaam
        .chars()
        .map(|harf| if harf.is_control() { ' ' } else { harf })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ");
    if munaqqa.chars().count() <= AQSA_IQTIBAS {
        return munaqqa;
    }
    let mut maqtu: String = munaqqa.chars().take(AQSA_IQTIBAS).collect();
    maqtu.push_str("...");
    maqtu
}

/// A list of names joined for an observation, with a count when it is cut.
fn asmaa_mujmaa(asmaa: &[String]) -> String {
    if asmaa.len() <= AQSA_ASMAA {
        return asmaa.join(", ");
    }
    let muqtatafa: Vec<&str> = asmaa.iter().take(AQSA_ASMAA).map(String::as_str).collect();
    let baqi = asmaa.len().saturating_sub(AQSA_ASMAA);
    format!("{} and {baqi} more", muqtatafa.join(", "))
}

/// A string field of a JSON object.
fn nass_json<'a>(qeema: &'a Value, miftah: &str) -> Option<&'a str> {
    qeema.get(miftah)?.as_str()
}

/// An integer field of a JSON object.
fn raqm_json(qeema: &Value, miftah: &str) -> Option<i64> {
    qeema.get(miftah)?.as_i64()
}

// ---------------------------------------------------------------------------
// RPG Maker MV and MZ
//
// MV and MZ are separate adapters, not one adapter with a flag: the core script
// names differ, the window text pipeline differs, the plugin registration
// differs, and the NW.js generation underneath them differs by four years of
// Chromium. Getting the two confused would send an ES5 bundle at a runtime that
// wants a module, or the reverse. So this detector never guesses between them —
// it reads which core script is on disk, and then reads that script's own
// statement of what it is.
// ---------------------------------------------------------------------------

/// The core scripts an MV export ships, in the order the engine loads them.
const LAWAHIQ_MV: [&str; 6] = [
    "js/rpg_core.js",
    "js/rpg_managers.js",
    "js/rpg_objects.js",
    "js/rpg_scenes.js",
    "js/rpg_sprites.js",
    "js/rpg_windows.js",
];

/// The core scripts an MZ export ships.
const LAWAHIQ_MZ: [&str; 6] = [
    "js/rmmz_core.js",
    "js/rmmz_managers.js",
    "js/rmmz_objects.js",
    "js/rmmz_scenes.js",
    "js/rmmz_sprites.js",
    "js/rmmz_windows.js",
];

/// Where an RPG Maker project can sit relative to the game root.
///
/// `www/` is MV's deployment layout and the one most MV games on a store use.
/// The flat layout is MZ's, and is also what an MV *project* folder looks like
/// when a developer uploads one without deploying — which happens constantly on
/// itch.io. The two `resources/app` entries are the same project repackaged
/// into an Electron shell, which is a real shape and the reason this detector
/// looks inside a wrapper rather than leaving the wrapper detector to guess.
const QAWAID_RPG: [&str; 4] = ["", "www", "resources/app", "resources/app/www"];

/// One candidate project root and what was found under it.
#[derive(Debug, Clone)]
struct QaidaRpg {
    /// The relative prefix, empty for a flat export.
    asas: String,
    /// MV core scripts present.
    mv: Vec<String>,
    /// MZ core scripts present.
    mz: Vec<String>,
    /// Whether `data/System.json` is there.
    nizam: bool,
    /// Whether `js/plugins.js` is there.
    mulhaqat: bool,
}

impl QaidaRpg {
    /// How much of a project this candidate looks like, used only to choose
    /// between candidates.
    fn quwwa(&self) -> usize {
        self.mv
            .len()
            .saturating_add(self.mz.len())
            .saturating_add(usize::from(self.nizam))
            .saturating_add(usize::from(self.mulhaqat))
    }
}

/// Joins a project prefix onto a relative path.
fn taht(asas: &str, lahiqa: &str) -> String {
    if asas.is_empty() {
        lahiqa.to_owned()
    } else {
        format!("{asas}/{lahiqa}")
    }
}

/// The entries of a marker list that actually exist under a prefix.
fn mawjud_taht(siyaq: &SiyaqFahs<'_>, asas: &str, lawahiq: &[&str]) -> Vec<String> {
    lawahiq
        .iter()
        .map(|lahiqa| taht(asas, lahiqa))
        .filter(|nisbi| siyaq.yujad(nisbi))
        .collect()
}

/// Finds the project root, when one of the four layouts is present.
fn qaida_rpg(siyaq: &SiyaqFahs<'_>) -> Option<QaidaRpg> {
    let mut afdal: Option<QaidaRpg> = None;
    for asas in QAWAID_RPG {
        let mv = mawjud_taht(siyaq, asas, &LAWAHIQ_MV);
        let mz = mawjud_taht(siyaq, asas, &LAWAHIQ_MZ);
        let nizam = siyaq.yujad(&taht(asas, "data/System.json"));
        let mulhaqat = siyaq.yujad(&taht(asas, "js/plugins.js"));

        // `data/System.json` on its own is not enough: the name is generic and
        // a `data` directory next to a `js` directory is not rare. Both of the
        // engine's own bookkeeping files together are.
        if mv.is_empty() && mz.is_empty() && !(nizam && mulhaqat) {
            continue;
        }
        let murashah = QaidaRpg {
            asas: asas.to_owned(),
            mv,
            mz,
            nizam,
            mulhaqat,
        };
        if afdal
            .as_ref()
            .is_none_or(|sabiq| murashah.quwwa() > sabiq.quwwa())
        {
            afdal = Some(murashah);
        }
    }
    afdal
}

/// What a core script says about the runtime that wrote it.
///
/// Both MV and MZ open their core script with a comment naming the file and its
/// version, and both then define `Utils.RPGMAKER_NAME` and
/// `Utils.RPGMAKER_VERSION` in the first pages. That pair is the runtime's own
/// statement of which product it is and which build, and it is the strongest
/// evidence available anywhere in this family — stronger than the filename,
/// because a repack can rename a file and cannot rewrite what it declares
/// without breaking the plugins that read it.
#[derive(Debug, Default, Clone)]
struct BayanNawat {
    /// `Utils.RPGMAKER_NAME` — `MV` or `MZ`.
    ism: Option<String>,
    /// `Utils.RPGMAKER_VERSION` — a dotted triple.
    isdar: Option<String>,
    /// The first line of the header comment, when it carries a version.
    tarwisa: Option<String>,
}

/// Reads a quoted value that follows a key, within a short window of it.
///
/// The window matters: `RPGMAKER_NAME` also appears inside plugin code and
/// inside comments, and an unbounded search for the next quotation mark would
/// happily walk a thousand lines and return a sprite filename.
fn qeema_muqtabasa(nass: &str, miftah: &str) -> Option<String> {
    let baqi = nass.split_once(miftah)?.1;
    let (mawqi, alama) = baqi
        .char_indices()
        .take(96)
        .find(|(_, harf)| matches!(*harf, '"' | '\''))?;
    let baad = baqi.get(mawqi.checked_add(alama.len_utf8())?..)?;
    let nihaya = baad
        .char_indices()
        .take(256)
        .find(|(_, harf)| *harf == alama)
        .map(|(mawdi, _)| mawdi)?;
    Some(baad.get(..nihaya)?.to_owned())
}

/// Reads the runtime's self-description out of the first pages of a core script.
fn bayan_nawat(masar: &Path) -> BayanNawat {
    let Some(nass) = iqra_nass(masar, u64::try_from(HAJM_TARWISA).unwrap_or(AQSA_NASS)) else {
        return BayanNawat::default();
    };
    let tarwisa = nass
        .lines()
        .take(8)
        .map(str::trim)
        .find(|satr| {
            satr.starts_with("//")
                && satr.contains(".js")
                && satr.chars().any(|harf| harf.is_ascii_digit())
        })
        .map(|satr| iqtibas(satr.trim_start_matches('/').trim()));
    BayanNawat {
        ism: qeema_muqtabasa(&nass, "RPGMAKER_NAME"),
        isdar: qeema_muqtabasa(&nass, "RPGMAKER_VERSION"),
        tarwisa,
    }
}

/// How many plugins `js/plugins.js` registers, and how many are switched on.
///
/// Phase 10 has to coexist with every one of them: it appends `taarib.js` to
/// this same list, and it overrides `Bitmap.prototype.drawText` and the
/// `Window_Base` text chain that a good number of plugins also override. A
/// project with sixty plugins is a project where the injection order matters and
/// where the capability report should say so before a translator starts work.
#[derive(Debug, Clone, Copy)]
struct AdadMulhaqat {
    /// Entries in the `$plugins` array.
    kul: usize,
    /// Entries whose `status` is true.
    mufaala: usize,
    /// Whether the array parsed as JSON, or the count is an approximation
    /// recovered from a file the developer hand-edited into invalidity.
    daqiq: bool,
}

/// Counts the entries in a `plugins.js`.
fn adad_mulhaqat(masar: &Path) -> Option<AdadMulhaqat> {
    let nass = iqra_nass(masar, AQSA_MALAF_KAMIL)?;
    let baad = nass.split_once("$plugins")?.1;
    let bidaya = baad.find('[')?;
    let nihaya = baad.rfind(']')?;
    if nihaya <= bidaya {
        return None;
    }
    let masfufa = baad.get(bidaya..nihaya.checked_add(1)?)?;

    if let Ok(Value::Array(qeem)) = serde_json::from_str::<Value>(masfufa) {
        let mufaala = qeem
            .iter()
            .filter(|madkhal| madkhal.get("status").and_then(Value::as_bool) == Some(true))
            .count();
        return Some(AdadMulhaqat {
            kul: qeem.len(),
            mufaala,
            daqiq: true,
        });
    }

    // A hand-edited `plugins.js` is common and still runs, because the engine
    // evaluates it as JavaScript rather than parsing it as JSON. Every entry
    // carries exactly one `status` key, so counting those recovers the shape
    // without pretending the file was well formed.
    let kul = masfufa.matches("\"status\"").count();
    if kul == 0 {
        return None;
    }
    let mufaala =
        masfufa.matches("\"status\":true").count() + masfufa.matches("\"status\": true").count();
    Some(AdadMulhaqat {
        kul,
        mufaala: mufaala.min(kul),
        daqiq: false,
    })
}

/// Library files only MZ ships.
const ALAMAT_MZ: [&str; 4] = [
    "js/libs/effekseer.min.js",
    "js/libs/vorbisdecoder.js",
    "js/libs/localforage.min.js",
    "js/libs/pako.min.js",
];

/// Library files only MV ships.
const ALAMAT_MV: [&str; 3] = [
    "js/libs/fpsmeter.js",
    "js/libs/iphone-inline-video.browser.js",
    "js/libs/lz-string.js",
];

/// What `data/System.json` says.
///
/// `System.json` is the one data file every RPG Maker game has and the one
/// Phase 10 always rewrites, so reading it here is both identification and a
/// dry run of the patcher's first step. Nothing is written and nothing is kept
/// beyond what appears in the evidence.
#[derive(Debug, Default, Clone)]
struct BayanNizam {
    /// `versionId` — the project's save-compatibility stamp, bumped every time
    /// the editor saves. Not an engine version, and reported as what it is.
    raqm_hifz: Option<i64>,
    /// `gameTitle`.
    unwan: Option<String>,
    /// `locale`, where the export carries one.
    lugha: Option<String>,
    /// Whether the MZ-only `advanced` block is present.
    mutaqaddim: bool,
    /// `advanced.mainFontFilename` — the font the runtime loads through CSS.
    khatt: Option<String>,
    /// `advanced.fontSize`.
    hajm_khatt: Option<i64>,
    /// `advanced.screenWidth` and `advanced.screenHeight`.
    shasha: Option<(i64, i64)>,
    /// An editor version, when the export happens to carry one.
    muharrir: Option<String>,
}

/// Reads `data/System.json`.
fn bayan_nizam(masar: &Path) -> Option<BayanNizam> {
    let jidhr = iqra_json(masar, AQSA_MALAF_KAMIL)?;
    if !jidhr.is_object() {
        return None;
    }
    let mutaqaddim = jidhr.get("advanced").is_some_and(Value::is_object);
    let khatt = jidhr
        .get("advanced")
        .and_then(|kutla| nass_json(kutla, "mainFontFilename"));
    let hajm_khatt = jidhr
        .get("advanced")
        .and_then(|kutla| raqm_json(kutla, "fontSize"));
    let shasha = jidhr.get("advanced").and_then(|kutla| {
        Some((
            raqm_json(kutla, "screenWidth")?,
            raqm_json(kutla, "screenHeight")?,
        ))
    });
    // The editor version is not in every export, which is why it is looked for
    // in three shapes and reported only when one of them is actually there.
    let muharrir = jidhr
        .get("editor")
        .and_then(|kutla| match kutla {
            Value::String(nass) => Some(nass.clone()),
            Value::Object(_) => nass_json(kutla, "version")
                .or_else(|| nass_json(kutla, "name"))
                .map(str::to_owned),
            _ => None,
        })
        .or_else(|| nass_json(&jidhr, "editorVersion").map(str::to_owned));

    Some(BayanNizam {
        raqm_hifz: raqm_json(&jidhr, "versionId"),
        unwan: nass_json(&jidhr, "gameTitle").map(str::to_owned),
        lugha: nass_json(&jidhr, "locale").map(str::to_owned),
        mutaqaddim,
        khatt: khatt.map(str::to_owned),
        hajm_khatt,
        shasha,
        muharrir,
    })
}

/// What `package.json` says about the shell the game runs in.
///
/// On its own this is worth almost nothing — every JavaScript project on earth
/// has a `package.json`, and finding one says only that somebody used npm's file
/// format. It earns its place here for two fields: `main`, which names the
/// layout the runtime expects, and `chromium-args`, which is the switch list
/// NW.js passes to Chromium and therefore the closest thing to a statement of
/// which Chromium generation is underneath.
#[derive(Debug, Default, Clone)]
struct BayanHuzma {
    /// `name`.
    ism: Option<String>,
    /// `version` — the *game's* version, never the shell's.
    isdar: Option<String>,
    /// `main` — `www/index.html` for a deployed MV project, `index.html` for MZ
    /// and for a flat layout.
    ra_isiy: Option<String>,
    /// `chromium-args`, verbatim.
    muamalat: Option<String>,
    /// `js-flags`, which MV and MZ both set to `--expose-gc`.
    aalam_js: Option<String>,
    /// `window.title`.
    unwan_nafidha: Option<String>,
    /// An `electron` dependency, when the manifest declares one.
    electron: Option<String>,
    /// An `nw` or `nw-builder` dependency, when the manifest declares one.
    nwjs: Option<String>,
}

/// Reads a `package.json`.
fn bayan_huzma(masar: &Path) -> Option<BayanHuzma> {
    let jidhr = iqra_json(masar, AQSA_MALAF_KAMIL)?;
    if !jidhr.is_object() {
        return None;
    }
    let taba = |miftah: &str| -> Option<String> {
        ["dependencies", "devDependencies", "optionalDependencies"]
            .iter()
            .find_map(|kutla| nass_json(jidhr.get(*kutla)?, miftah).map(str::to_owned))
    };
    Some(BayanHuzma {
        ism: nass_json(&jidhr, "name").map(str::to_owned),
        isdar: nass_json(&jidhr, "version").map(str::to_owned),
        ra_isiy: nass_json(&jidhr, "main").map(str::to_owned),
        muamalat: nass_json(&jidhr, "chromium-args").map(str::to_owned),
        aalam_js: nass_json(&jidhr, "js-flags").map(str::to_owned),
        unwan_nafidha: jidhr
            .get("window")
            .and_then(|nafidha| nass_json(nafidha, "title"))
            .map(str::to_owned),
        electron: taba("electron"),
        nwjs: taba("nw").or_else(|| taba("nw-builder")),
    })
}

/// Chromium runtime files that pin the generation underneath a shell.
///
/// These are not NW.js versions and are not read as such. They are Chromium's
/// own history, which is public and which every shell inherits: `natives_blob.bin`
/// was removed in Chromium 74, the `swiftshader/` directory was replaced by a
/// single `vk_swiftshader` library around Chromium 90. Where a game sits in that
/// sequence is what decides whether Phase 10 injects the ES5 bundle or the
/// modern one, and that decision needs a bound rather than a guess.
///
/// This is inference, not a version string, and every observation it produces
/// says so in its own text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JeelChromium {
    /// Chromium 73 or older: the generation MV shipped with.
    Qadeem,
    /// Chromium 74 to 89.
    Wasat,
    /// Chromium 90 or newer: the generation later MZ exports ship with.
    Hadeeth,
}

impl JeelChromium {
    /// The bound as an observation writes it.
    const fn wasf(self) -> &'static str {
        match self {
            Self::Qadeem => "Chromium 73 or older",
            Self::Wasat => "Chromium 74 to 89",
            Self::Hadeeth => "Chromium 90 or newer",
        }
    }
}

/// Reads the Chromium generation off the shell's own runtime files.
fn jeel_chromium(siyaq: &SiyaqFahs<'_>) -> Option<(JeelChromium, String)> {
    const HADEETHA: [&str; 3] = [
        "vk_swiftshader.dll",
        "vk_swiftshader_icd.json",
        "libvk_swiftshader.so",
    ];
    if let Some((nisbi, _)) = awwal_malaf(siyaq, &HADEETHA) {
        return Some((JeelChromium::Hadeeth, nisbi));
    }
    if let Some((nisbi, _)) = awwal_malaf(siyaq, &["natives_blob.bin"]) {
        return Some((JeelChromium::Qadeem, nisbi));
    }
    if let Some((nisbi, _)) = awwal_mujallad(siyaq, &["swiftshader"]) {
        return Some((JeelChromium::Wasat, nisbi));
    }
    None
}

/// Keeps the strongest family claim a detector has seen so far.
///
/// Detectors in this module gather several independent claims about which
/// engine they are looking at — a filename, a library file, a declaration
/// inside a script — and the strongest wins. Weaker claims are still recorded
/// as evidence, because a disagreement between two of them is exactly what a
/// maintainer needs to see when an identification turns out wrong.
fn arjah(hali: &mut Option<(AilatMuharrik, u8)>, aila: AilatMuharrik, wazn: u8) {
    if hali.as_ref().is_none_or(|(_, sabiq)| wazn > *sabiq) {
        *hali = Some((aila, wazn));
    }
}

/// The absolute path of a project prefix.
fn masar_asas(siyaq: &SiyaqFahs<'_>, asas: &str) -> Option<PathBuf> {
    if asas.is_empty() {
        Some(siyaq.jidhr.to_path_buf())
    } else {
        siyaq.dakhil(asas).ok()
    }
}

/// The RPG Maker MV and MZ detector.
fn rpg_maker(siyaq: &SiyaqFahs<'_>) -> HasilatFahs {
    let mut hasila = HasilatFahs::la_shay();
    let Some(qaida) = qaida_rpg(siyaq) else {
        return hasila;
    };
    let asas = qaida.asas.clone();
    let mut mutalab: Option<(AilatMuharrik, u8)> = None;

    if !qaida.mz.is_empty() {
        arjah(&mut mutalab, AilatMuharrik::RpgMakerMz, 96);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "{} of MZ's six core scripts are present: {}",
                qaida.mz.len(),
                asmaa_mujmaa(&qaida.mz)
            ),
            qaida.mz.first().cloned(),
            96,
        );
    }
    if !qaida.mv.is_empty() {
        arjah(&mut mutalab, AilatMuharrik::RpgMakerMv, 96);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "{} of MV's six core scripts are present: {}",
                qaida.mv.len(),
                asmaa_mujmaa(&qaida.mv)
            ),
            qaida.mv.first().cloned(),
            96,
        );
    }
    if !qaida.mv.is_empty() && !qaida.mz.is_empty() {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "both MV and MZ core scripts are present, which is a repack or a project \
             upgraded in place; the runtime's own declaration decides which one runs",
            (!asas.is_empty()).then(|| asas.clone()),
            45,
        );
    }

    // The runtime's own statement of what it is. Stronger than any filename:
    // a repack can rename a script, and cannot change what the script declares
    // without breaking every plugin that reads it.
    let nawat = qaida.mz.first().or_else(|| qaida.mv.first()).cloned();
    if let Some(nisbi) = nawat
        && let Ok(masar) = siyaq.dakhil(&nisbi)
    {
        let bayan = bayan_nawat(&masar);
        match bayan.ism.as_deref() {
            Some("MZ") => {
                arjah(&mut mutalab, AilatMuharrik::RpgMakerMz, 99);
                hasila.sajjil(
                    NawDaleel::BayanatMudmaja,
                    "the core script declares Utils.RPGMAKER_NAME = \"MZ\"",
                    Some(nisbi.clone()),
                    99,
                );
            },
            Some("MV") => {
                arjah(&mut mutalab, AilatMuharrik::RpgMakerMv, 99);
                hasila.sajjil(
                    NawDaleel::BayanatMudmaja,
                    "the core script declares Utils.RPGMAKER_NAME = \"MV\"",
                    Some(nisbi.clone()),
                    99,
                );
            },
            Some(gharib) => {
                let wasf = format!("RPGMAKER_NAME is neither MV nor MZ: {}", iqtibas(gharib));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 35);
            },
            None => {},
        }
        if let Some(khaam) = bayan.isdar.as_deref() {
            hasila.isdar = Some(isdar_min_nass(khaam));
            let wasf = format!(
                "Utils.RPGMAKER_VERSION in the core script is {}",
                iqtibas(khaam)
            );
            hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 92);
        }
        if let Some(tarwisa) = bayan.tarwisa.as_deref() {
            hasila.sajjil(
                NawDaleel::BayanatMudmaja,
                format!(
                    "the core script's header comment reads: {}",
                    iqtibas(tarwisa)
                ),
                Some(nisbi),
                60,
            );
        }
    }

    // Library files, which survive when the core scripts have been renamed,
    // concatenated into one bundle, or encrypted by a plugin.
    let mz_libs = mawjud_taht(siyaq, &asas, &ALAMAT_MZ);
    let mv_libs = mawjud_taht(siyaq, &asas, &ALAMAT_MV);
    if !mz_libs.is_empty() {
        arjah(&mut mutalab, AilatMuharrik::RpgMakerMz, 74);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "libraries only MZ ships are present: {}",
                asmaa_mujmaa(&mz_libs)
            ),
            mz_libs.first().cloned(),
            74,
        );
    }
    if !mv_libs.is_empty() {
        arjah(&mut mutalab, AilatMuharrik::RpgMakerMv, 72);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            format!(
                "libraries only MV ships are present: {}",
                asmaa_mujmaa(&mv_libs)
            ),
            mv_libs.first().cloned(),
            72,
        );
    }

    rpg_maker_bayanat(siyaq, &qaida, &mut hasila, &mut mutalab);
    rpg_maker_ghilaf(siyaq, &asas, &mut hasila);

    if let Some((aila, _)) = mutalab {
        hasila.aila = Some(aila);
        hasila.khalfiya = Some(KhalfiyaBarmajiya::JavaScript);
        hasila.daa_itar(ItarNusus::NafidhatRpg);
    } else if qaida.nizam && qaida.mulhaqat {
        // A `data/System.json` and a `js/plugins.js` together are an RPG Maker
        // project and nothing else — but which product wrote them is genuinely
        // unknown here, and naming one would be a guess. The evidence goes
        // through without a family, and resolution decides what to do with it.
        hasila.khalfiya = Some(KhalfiyaBarmajiya::JavaScript);
        hasila.daa_itar(ItarNusus::NafidhatRpg);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "an RPG Maker data and plugin pair is present, but no core script, library \
             or project file names which product produced it",
            Some(taht(&asas, "js/plugins.js")),
            55,
        );
    }
    hasila
}

/// Parses the one line a `.rpgproject` or `.rmmzproject` file contains.
///
/// The whole file is a marker and a version — `RPGMV 1.6.2`, `RPGMZ 1.9.0` —
/// written by the editor that owns the project. It is absent from a proper
/// deployment and present in every project folder uploaded as-is, which on
/// itch.io is most of them.
fn bayan_mashru(nass: &str) -> Option<(AilatMuharrik, Option<String>)> {
    let mut ajzaa = nass.split_whitespace();
    let aila = match ajzaa.next()?.to_ascii_uppercase().as_str() {
        "RPGMV" => AilatMuharrik::RpgMakerMv,
        "RPGMZ" => AilatMuharrik::RpgMakerMz,
        _ => return None,
    };
    Some((aila, ajzaa.next().map(str::to_owned)))
}

/// Reads the project's own data: `System.json`, `plugins.js`, and the editor's
/// project file where the developer shipped one.
fn rpg_maker_bayanat(
    siyaq: &SiyaqFahs<'_>,
    qaida: &QaidaRpg,
    hasila: &mut HasilatFahs,
    mutalab: &mut Option<(AilatMuharrik, u8)>,
) {
    let asas = qaida.asas.as_str();

    let nisbi_nizam = taht(asas, "data/System.json");
    if let Ok(masar) = siyaq.dakhil(&nisbi_nizam)
        && masar.is_file()
    {
        match bayan_nizam(&masar) {
            Some(bayan) => {
                let unwan = bayan
                    .unwan
                    .as_deref()
                    .map_or_else(String::new, |nass| format!(", gameTitle {}", iqtibas(nass)));
                let raqm = bayan
                    .raqm_hifz
                    .map_or_else(String::new, |qeema| format!(", versionId {qeema}"));
                let wasf = format!("data/System.json parses as a system record{raqm}{unwan}");
                hasila.sajjil(
                    NawDaleel::BayanatMudmaja,
                    wasf,
                    Some(nisbi_nizam.clone()),
                    74,
                );

                if bayan.mutaqaddim {
                    arjah(mutalab, AilatMuharrik::RpgMakerMz, 62);
                    hasila.sajjil(
                        NawDaleel::BayanatMudmaja,
                        "System.json carries the advanced block, which MZ writes and MV \
                         does not",
                        Some(nisbi_nizam.clone()),
                        62,
                    );
                }
                if let Some(khatt) = bayan.khatt.as_deref() {
                    let hajm = bayan.hajm_khatt.unwrap_or_default();
                    let wasf = format!("the runtime loads {} at size {hajm}", iqtibas(khatt));
                    hasila.sajjil(
                        NawDaleel::BayanatMudmaja,
                        wasf,
                        Some(nisbi_nizam.clone()),
                        40,
                    );
                }
                if let Some((ard, irtifa)) = bayan.shasha {
                    let wasf = format!("the declared screen is {ard} by {irtifa}");
                    hasila.sajjil(
                        NawDaleel::BayanatMudmaja,
                        wasf,
                        Some(nisbi_nizam.clone()),
                        30,
                    );
                }
                if let Some(lugha) = bayan.lugha.as_deref() {
                    let wasf = format!("the declared locale is {}", iqtibas(lugha));
                    hasila.sajjil(
                        NawDaleel::BayanatMudmaja,
                        wasf,
                        Some(nisbi_nizam.clone()),
                        25,
                    );
                }
                if let Some(muharrir) = bayan.muharrir.as_deref() {
                    if hasila.isdar.is_none() {
                        hasila.isdar = Some(isdar_min_nass(muharrir));
                    }
                    let wasf = format!("System.json names editor version {}", iqtibas(muharrir));
                    hasila.sajjil(
                        NawDaleel::BayanatMudmaja,
                        wasf,
                        Some(nisbi_nizam.clone()),
                        70,
                    );
                }
            },
            None => hasila.sajjil(
                NawDaleel::BinyatMujallad,
                "data/System.json is present but does not parse as JSON; the file's presence \
                 still counts and its contents do not",
                Some(nisbi_nizam.clone()),
                50,
            ),
        }
    }

    let nisbi_mulhaqat = taht(asas, "js/plugins.js");
    if let Ok(masar) = siyaq.dakhil(&nisbi_mulhaqat)
        && masar.is_file()
    {
        match adad_mulhaqat(&masar) {
            Some(adad) => {
                let daqqa = if adad.daqiq {
                    ""
                } else {
                    " (approximate: the array is not valid JSON)"
                };
                let wasf = format!(
                    "js/plugins.js registers {} plugins, {} enabled{daqqa}",
                    adad.kul, adad.mufaala
                );
                hasila.sajjil(
                    NawDaleel::BayanatMudmaja,
                    wasf,
                    Some(nisbi_mulhaqat.clone()),
                    82,
                );
            },
            None => hasila.sajjil(
                NawDaleel::BinyatMujallad,
                "js/plugins.js is present and its $plugins array could not be read",
                Some(nisbi_mulhaqat.clone()),
                60,
            ),
        }
    }

    let Some(jidhr_asas) = masar_asas(siyaq, asas) else {
        return;
    };
    let mashru = madakhil(&jidhr_asas)
        .into_iter()
        .find_map(|(ism, mujallad)| {
            let munkhafid = ism.to_ascii_lowercase();
            let yutabiq = munkhafid.ends_with(".rpgproject") || munkhafid.ends_with(".rmmzproject");
            (!mujallad && yutabiq).then_some(ism)
        });
    if let Some(ism) = mashru {
        let nisbi = taht(asas, &ism);
        let nass = siyaq
            .dakhil(&nisbi)
            .ok()
            .and_then(|masar| iqra_nass(&masar, AQSA_NASS));
        if let Some((aila, isdar)) = nass.as_deref().and_then(bayan_mashru) {
            arjah(mutalab, aila, 88);
            if let Some(khaam) = isdar.as_deref() {
                if hasila.isdar.is_none() {
                    hasila.isdar = Some(isdar_min_nass(khaam));
                }
                let wasf = format!("the project file names editor {} {}", aila.ism(), khaam);
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 88);
            } else {
                let wasf = format!("the project file names editor {}", aila.ism());
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 84);
            }
        }
    }
}

/// NW.js runtime files, which sit beside the project rather than inside it.
const ALAMAT_NWJS: [&str; 7] = [
    "nw.dll",
    "nw.pak",
    "nw_100_percent.pak",
    "nw_200_percent.pak",
    "nw_elf.dll",
    "lib/libnw.so",
    "nwjs Framework.framework",
];

/// Reads the shell the project is deployed inside: the NW.js manifest, the
/// Chromium generation underneath it, and the runner executable.
///
/// None of this identifies RPG Maker. All of it decides what Phase 10 injects:
/// an ES5 bundle for the Chromium generation MV shipped with, a modern one for
/// MZ's, and a repack rather than a plain file write when the project turns out
/// to live inside an asar.
fn rpg_maker_ghilaf(siyaq: &SiyaqFahs<'_>, asas: &str, hasila: &mut HasilatFahs) {
    let mut murashahat: Vec<String> = Vec::new();
    if let Some(juz) = asas.strip_suffix("www") {
        murashahat.push(taht(juz.trim_end_matches('/'), "package.json"));
    }
    murashahat.push("package.json".to_owned());
    murashahat.push("resources/app/package.json".to_owned());
    murashahat.push(taht(asas, "package.json"));
    let marja: Vec<&str> = murashahat.iter().map(String::as_str).collect();

    if let Some((nisbi, masar)) = awwal_malaf(siyaq, &marja) {
        let bayan = bayan_huzma(&masar);
        let laqab = bayan
            .as_ref()
            .and_then(|qeema| qeema.ism.clone())
            .map_or_else(String::new, |ism| format!(" naming {}", iqtibas(&ism)));
        let wasf = format!(
            "a package.json is present{laqab}; on its own that says only that this is a \
             JavaScript project"
        );
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi.clone()), 10);

        if let Some(bayan) = bayan {
            if let Some(rais) = bayan.ra_isiy.as_deref() {
                let shakl = if rais.contains("www/") {
                    "MV's deployed layout"
                } else {
                    "a flat layout, which is MZ's and an undeployed MV project's"
                };
                let wasf = format!("package.json main is {}, {shakl}", iqtibas(rais));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 45);
            }
            if let Some(muamalat) = bayan.muamalat.as_deref() {
                let wasf = format!("the shell is launched with {}", iqtibas(muamalat));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 35);
            }
            if let Some(aalam) = bayan.aalam_js.as_deref() {
                let wasf = format!("js-flags is {}", iqtibas(aalam));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 20);
            }
            if let Some(unwan) = bayan.unwan_nafidha.as_deref() {
                let wasf = format!("the window title is {}", iqtibas(unwan));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 15);
            }
            if let Some(qeema) = bayan.electron.as_deref() {
                let wasf = format!("the manifest depends on electron {}", iqtibas(qeema));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 55);
            }
            if let Some(qeema) = bayan.nwjs.as_deref() {
                let wasf = format!("the manifest depends on nw {}", iqtibas(qeema));
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 55);
            }
        }
    }

    let nwjs = mawjud_taht(siyaq, "", &ALAMAT_NWJS);
    if !nwjs.is_empty() {
        let wasf = format!(
            "NW.js runtime files sit beside the project: {}",
            asmaa_mujmaa(&nwjs)
        );
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, nwjs.first().cloned(), 55);
    }
    if let Some((jeel, mawqi)) = jeel_chromium(siyaq) {
        let wasf = format!(
            "the shell's Chromium files place it at {}, inferred from {mawqi} rather than \
             read from any version string",
            jeel.wasf()
        );
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(mawqi), 42);
    }

    let murashah_tanfidhi: &[&str] = match siyaq.nizam {
        NizamTashghil::Windows => &["Game.exe", "game.exe", "nw.exe"],
        NizamTashghil::Linux => &["Game", "game", "nw"],
        NizamTashghil::Mac => &[],
    };
    if hasila.tanfidhi.is_none()
        && let Some((nisbi, masar)) = awwal_malaf(siyaq, murashah_tanfidhi)
    {
        let wasf = format!("the runner executable is {nisbi}");
        hasila.tanfidhi = Some(masar);
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 20);
    }
}

// ---------------------------------------------------------------------------
// RPG Maker VX Ace
//
// Two markers, both cheap, both unambiguous: the RGSSAD version 3 archive and
// the `Library=` line in `Game.ini` that names the exact RGSS3 runtime the game
// loads. Phase 10 needs the second even when it has the first, because the
// Ruby script it injects binds to the C ABI through `Win32API`, and which RGSS3
// build is loaded decides what that binding sees.
// ---------------------------------------------------------------------------

/// The header of an RGSSAD archive.
///
/// The layout is eight bytes and has not changed since RPG Maker XP:
///
/// ```text
/// offset  size  meaning
/// 0       6     the ASCII bytes RGSSAD
/// 6       1     a zero byte
/// 7       1     the archive version: 1 for XP, 2 for VX, 3 for VX Ace
/// 8       4     version 3 only: the little-endian key seed
/// ```
///
/// For version 3 the file table's obfuscation key is `seed * 9 + 3`, and every
/// entry in the table is `XORed` with it. The key is derived here and reported;
/// nothing is decrypted, because reading a game's archive contents is Phase 12's
/// job and rewriting them is Phase 10's, and a capability probe that unpacked an
/// archive would be doing both.
#[derive(Debug, Clone, Copy)]
struct TarwisatRgssad {
    /// The version byte.
    isdar: u8,
    /// The key seed, for a version 3 archive.
    bidhra: Option<u32>,
}

impl TarwisatRgssad {
    /// The obfuscation key a version 3 archive derives from its seed.
    fn miftah(self) -> Option<u32> {
        self.bidhra
            .map(|bidhra| bidhra.wrapping_mul(9).wrapping_add(3))
    }

    /// Which editor an archive of this version came from.
    const fn muharrir(self) -> &'static str {
        match self.isdar {
            1 => "RPG Maker XP",
            2 => "RPG Maker VX",
            3 => "RPG Maker VX Ace",
            _ => "an unknown RGSSAD generation",
        }
    }
}

/// Reads an RGSSAD header.
fn tarwisat_rgssad(masar: &Path) -> Option<TarwisatRgssad> {
    let bayt = iqra_muhaddad(masar, 16)?;
    if !bayt.starts_with(b"RGSSAD\0") {
        return None;
    }
    let isdar = *bayt.get(7)?;
    let bidhra = if isdar == 3 { u32_le(&bayt, 8) } else { None };
    Some(TarwisatRgssad { isdar, bidhra })
}

/// One key out of an INI file, matched case-insensitively.
///
/// Deliberately not an INI parser. `Game.ini` has one section, four keys, and is
/// written by a Japanese editor that emits it in whatever the system code page
/// was; reading one value out of it does not justify a parser, and a parser
/// would reject files the engine itself reads happily.
fn qeemat_ini(nass: &str, miftah: &str) -> Option<String> {
    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.starts_with(';') || munaqqa.starts_with('#') || munaqqa.starts_with('[') {
            continue;
        }
        let Some((ism, qeema)) = munaqqa.split_once('=') else {
            continue;
        };
        if ism.trim().eq_ignore_ascii_case(miftah) {
            let munaqqah = qeema.trim().trim_matches('"').trim();
            if !munaqqah.is_empty() {
                return Some(munaqqah.to_owned());
            }
        }
    }
    None
}

/// The final component of a Windows or Unix path written inside a data file.
fn ism_malaf_min_nass(masar: &str) -> &str {
    masar.rsplit(['\\', '/']).next().unwrap_or(masar)
}

/// Turns an RGSS library filename into a version.
///
/// `RGSS300.dll` is RGSS generation 3, runtime build 00. The trailing digits are
/// a build number rather than a minor version, and the raw name is kept so that
/// nothing downstream has to trust this reading of it.
fn isdar_rgss(ism: &str) -> Option<IsdarMuharrik> {
    let jidhr = ism.split('.').next().unwrap_or(ism);
    let baqi = jidhr
        .strip_prefix("RGSS")
        .or_else(|| jidhr.strip_prefix("rgss"))?;
    let arqam: String = baqi.chars().take_while(char::is_ascii_digit).collect();
    let mut huruf = arqam.chars();
    let kabir = huruf
        .next()?
        .to_digit(10)
        .and_then(|raqm| u16::try_from(raqm).ok())?;
    let bina: String = huruf.collect();
    Some(IsdarMuharrik {
        kabir,
        sagheer: bina.parse::<u16>().unwrap_or(0),
        tasheeh: 0,
        khaam: jidhr.to_owned(),
        mushtaqq: false,
    })
}

/// Archive names, newest generation first.
const ARSHIFAT_RGSS: [&str; 6] = [
    "Game.rgss3a",
    "game.rgss3a",
    "Game.rgss2a",
    "game.rgss2a",
    "Game.rgssad",
    "game.rgssad",
];

/// The RPG Maker VX Ace detector.
fn vx_ace(siyaq: &SiyaqFahs<'_>) -> HasilatFahs {
    let mut hasila = HasilatFahs::la_shay();
    let mut mutalab: Option<(AilatMuharrik, u8)> = None;

    if let Some((nisbi, masar)) = awwal_malaf(siyaq, &ARSHIFAT_RGSS) {
        match tarwisat_rgssad(&masar) {
            Some(tarwisa) if tarwisa.isdar == 3 => {
                arjah(&mut mutalab, AilatMuharrik::RpgMakerVxAce, 96);
                let baytat = hajm(&masar).unwrap_or_default();
                let miftah = tarwisa
                    .miftah()
                    .map_or_else(|| "unreadable".to_owned(), |qeema| format!("{qeema:#010x}"));
                let wasf = format!(
                    "{nisbi} is an RGSSAD version 3 archive of {baytat} bytes; its file \
                     table key derives to {miftah}"
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 96);
            },
            Some(tarwisa) => {
                let wasf = format!(
                    "{nisbi} is an RGSSAD version {} archive, which is {}; Taarib has no \
                     adapter for that generation",
                    tarwisa.isdar,
                    tarwisa.muharrir()
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 45);
            },
            None => {
                let wasf = format!("{nisbi} is present and carries no RGSSAD header");
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 30);
            },
        }
    }

    if let Some((nisbi, masar)) = awwal_malaf(siyaq, &["Game.ini", "game.ini", "GAME.INI"])
        && let Some(nass) = iqra_nass(&masar, AQSA_NASS)
    {
        if let Some(maktaba) = qeemat_ini(&nass, "Library") {
            let ism = ism_malaf_min_nass(&maktaba).to_owned();
            let kabira = ism.to_ascii_uppercase();
            if kabira.starts_with("RGSS3") {
                arjah(&mut mutalab, AilatMuharrik::RpgMakerVxAce, 94);
                if hasila.isdar.is_none() {
                    hasila.isdar = isdar_rgss(&ism);
                }
                let wasf = format!("Game.ini loads {ism}, which is the VX Ace RGSS3 runtime");
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 94);
            } else if kabira.starts_with("RGSS") {
                let wasf = format!(
                    "Game.ini loads {ism}, which is an RGSS1 or RGSS2 runtime — RPG Maker \
                     XP or VX, neither of which Taarib has an adapter for"
                );
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 45);
            }
        }
        if let Some(scripts) = qeemat_ini(&nass, "Scripts") {
            let wasf = format!("Game.ini names the script list {}", iqtibas(&scripts));
            hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 60);
        }
        if let Some(unwan) = qeemat_ini(&nass, "Title") {
            let wasf = format!("Game.ini names the game {}", iqtibas(&unwan));
            hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 20);
        }
    }

    if let Some((nisbi, masar)) = awwal_mujallad(siyaq, &["Data", "data"]) {
        let bayanat: Vec<String> = madakhil(&masar)
            .into_iter()
            .filter(|(ism, mujallad)| !*mujallad && ism.to_ascii_lowercase().ends_with(".rvdata2"))
            .map(|(ism, _)| ism)
            .collect();
        if !bayanat.is_empty() {
            arjah(&mut mutalab, AilatMuharrik::RpgMakerVxAce, 84);
            let wasf = format!(
                "{} unencrypted VX Ace data files are in {nisbi}: {}",
                bayanat.len(),
                asmaa_mujmaa(&bayanat)
            );
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 84);
        }
    }

    if let Some((nisbi, masar)) = awwal_mujallad(siyaq, &["System", "system"]) {
        let maktabat: Vec<String> = madakhil(&masar)
            .into_iter()
            .filter(|(ism, mujallad)| {
                let kabira = ism.to_ascii_uppercase();
                !*mujallad && kabira.starts_with("RGSS") && imtidad(ism, "dll")
            })
            .map(|(ism, _)| ism)
            .collect();
        if !maktabat.is_empty() {
            let wasf = format!(
                "the RGSS runtime ships beside it: {}",
                asmaa_mujmaa(&maktabat)
            );
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 70);
        }
    }

    if let Some((aila, _)) = mutalab {
        hasila.aila = Some(aila);
        hasila.khalfiya = Some(KhalfiyaBarmajiya::Ruby);
        hasila.daa_itar(ItarNusus::NafidhatRpg);
        // RGSS3 exists only as a 32-bit Windows library, so a VX Ace game is a
        // 32-bit process on every machine it has ever run on. That is inferred
        // from the runtime rather than read from the executable's own header,
        // and the binary detector's reading wins if the two ever disagree.
        hasila.mimariya = Some(Mimariya::X86);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "RGSS3 is a 32-bit Windows runtime, so the game process is 32-bit; inferred \
             from the runtime rather than read from the executable",
            None,
            30,
        );
        if let Some((nisbi, masar)) = awwal_malaf(siyaq, &["Game.exe", "game.exe"]) {
            let wasf = format!("the runner executable is {nisbi}");
            hasila.tanfidhi = Some(masar);
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 25);
        }
    }
    hasila
}

// ---------------------------------------------------------------------------
// Ren'Py
//
// Identifying Ren'Py is trivial. Reading its version is the entire point of
// this detector, because Phase 10 has two Ren'Py adapters and the version picks
// between them:
//
// - From 7.4 the engine bundles HarfBuzz and FriBidi. It can shape Arabic and
//   resolve direction by itself, and the adapter's whole job is to hand it the
//   right language and direction properties, register the font, and get out of
//   the way. That path is small, native, and correct.
// - Below 7.4 there is no shaper in the engine at all. The adapter has to
//   install a text filter, lay out through the C ABI over ctypes, and return a
//   custom displayable that draws Taarib's own glyphs. That path is large and
//   it replaces the engine's text rendering rather than configuring it.
//
// Those are not two settings of one adapter. Sending the first at a 7.3 game
// produces unjoined isolated letters in visual disorder and no error, which is
// the worst possible failure: it looks like it worked. So the version is parsed
// out of `version_tuple` rather than inferred, and the directory-name fallback
// below is used only when that file cannot be read at all — and says so.
// ---------------------------------------------------------------------------

/// One `lib/<platform>` build directory a Ren'Py game ships.
#[derive(Debug, Clone)]
struct BinaRenpy {
    /// The directory name, verbatim.
    ism: String,
    /// The Python major version the `py2-`/`py3-` prefix declares.
    python: Option<u16>,
    /// The platform segment: `windows`, `linux`, `mac`, `android`, `ios`, `web`.
    manassa: String,
    /// The architecture segment, when it maps onto one Taarib knows.
    mimariya: Option<Mimariya>,
    /// The architecture segment verbatim, including the ones that map onto
    /// nothing — `universal`, `armv7l`.
    mimariya_khaam: String,
}

/// Platform segments a Ren'Py build directory can carry.
const MANASSAT_RENPY: [&str; 7] = ["windows", "linux", "mac", "darwin", "android", "ios", "web"];

/// Parses a `lib/` entry into a build description.
///
/// The `py2-` and `py3-` prefixes are themselves a version signal: they were
/// introduced when Ren'Py grew a Python 3 branch at 7.4, and a build directory
/// without one is from 7.3 or earlier. That is the only fallback this detector
/// has when `renpy/__init__.py` is unreadable, and it is treated as a bound
/// rather than a version everywhere it is used.
fn bina_renpy(ism: &str) -> Option<BinaRenpy> {
    let (python, baqi) = match ism.split_once('-') {
        Some(("py2", baqi)) => (Some(2), baqi),
        Some(("py3", baqi)) => (Some(3), baqi),
        _ => (None, ism),
    };
    let (manassa, mimariya_khaam) = baqi.split_once('-')?;
    if !MANASSAT_RENPY.contains(&manassa) {
        return None;
    }
    let mimariya = match mimariya_khaam {
        "x86_64" | "amd64" => Some(Mimariya::X8664),
        "i686" | "i386" | "x86" => Some(Mimariya::X86),
        "aarch64" | "arm64" => Some(Mimariya::Aarch64),
        _ => None,
    };
    Some(BinaRenpy {
        ism: ism.to_owned(),
        python,
        manassa: manassa.to_owned(),
        mimariya,
        mimariya_khaam: mimariya_khaam.to_owned(),
    })
}

/// Reads `version_tuple` out of `renpy/__init__.py`.
///
/// The line is an ordinary Python assignment —
/// `version_tuple = (8, 1, 3, vc_version)` — and the fourth element is a build
/// counter from version control rather than a version component, so only the
/// first three are taken. Matching the line rather than the first occurrence of
/// the identifier keeps a mention in a comment or a docstring from being read
/// as the assignment.
fn isdar_renpy(nass: &str) -> Option<(u16, u16, u16)> {
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

/// Whether a version is at or above the release where Ren'Py can shape Arabic
/// itself.
const fn yashkul_renpy(kabir: u16, sagheer: u16) -> bool {
    let (hadd_kabir, hadd_sagheer) = HADD_TASHKEEL_RENPY;
    kabir > hadd_kabir || (kabir == hadd_kabir && sagheer >= hadd_sagheer)
}

/// The version marker at the head of an RPA archive.
///
/// An RPA file opens with one text line — `RPA-3.0 <hex offset> <hex key>` —
/// naming the archive format. Reading it costs sixty-four bytes and confirms
/// both that the file is what its extension claims and which archive generation
/// Phase 12 will have to open.
fn isdar_rpa(masar: &Path) -> Option<String> {
    let bayt = iqra_muhaddad(masar, 64)?;
    let nass = String::from_utf8_lossy(&bayt);
    let alama = nass.lines().next()?.split_whitespace().next()?;
    alama.starts_with("RPA-").then(|| alama.to_owned())
}

/// Whether a compiled script carries the `RPyC` version 2 magic.
///
/// `RENPY RPC2` opens every compiled script from Ren'Py 6.18 onward. An older
/// `RPyC` is a bare zlib stream with no magic at all, which is why this answers
/// with a boolean rather than treating the absence as a failure.
fn rpyc_thani(masar: &Path) -> bool {
    iqra_muhaddad(masar, 16).is_some_and(|bayt| bayt.starts_with(b"RENPY RPC2"))
}

/// Finds a Ren'Py launcher: the platform script whose name matches one of the
/// `.py` entry points the engine ships beside it.
fn tanfidhi_renpy(siyaq: &SiyaqFahs<'_>) -> Option<(String, PathBuf)> {
    let judhur: Vec<String> = madakhil(siyaq.jidhr)
        .into_iter()
        .filter_map(|(ism, mujallad)| {
            if mujallad {
                return None;
            }
            ism.strip_suffix(".py").map(str::to_owned)
        })
        .filter(|jidhr_ism| jidhr_ism.as_str() != "renpy")
        .collect();
    let lawahiq: &[&str] = match siyaq.nizam {
        NizamTashghil::Windows => &[".exe"],
        NizamTashghil::Linux | NizamTashghil::Mac => &[".sh"],
    };
    for jidhr_ism in judhur {
        for lahiqa in lawahiq {
            let nisbi = format!("{jidhr_ism}{lahiqa}");
            if let Ok(masar) = siyaq.dakhil(&nisbi)
                && masar.is_file()
            {
                return Some((nisbi, masar));
            }
        }
    }
    None
}

/// The Ren'Py detector.
fn renpy(siyaq: &SiyaqFahs<'_>) -> HasilatFahs {
    let mut hasila = HasilatFahs::la_shay();
    let mut mutalab: Option<(AilatMuharrik, u8)> = None;

    let mujallad_renpy = awwal_mujallad(siyaq, &["renpy"]);
    let mujallad_luba = awwal_mujallad(siyaq, &["game"]);
    let mut arshifa: Vec<String> = Vec::new();
    if let Some((_, masar)) = mujallad_luba.as_ref() {
        for (ism, mujallad) in madakhil(masar) {
            if !mujallad && ism.to_ascii_lowercase().ends_with(".rpa") {
                arshifa.push(ism);
            }
        }
        arshifa.sort();
    }
    let script = awwal_malaf(siyaq, &["game/script.rpyc"]);

    // A `game/` directory on its own is far too generic to act on — plenty of
    // engines ship one. The gate is a `renpy/` package, a compiled script, or an
    // RPA archive, each of which belongs to this engine and to nothing else.
    if mujallad_renpy.is_none() && script.is_none() && arshifa.is_empty() {
        return hasila;
    }

    if let Some((nisbi, _)) = mujallad_renpy.as_ref() {
        arjah(&mut mutalab, AilatMuharrik::Renpy, 88);
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "a renpy/ engine package sits at the game root",
            Some(nisbi.clone()),
            88,
        );
    }

    let mut isdar_maqru: Option<(u16, u16, u16)> = None;
    if let Some((nisbi, masar)) = awwal_malaf(siyaq, &["renpy/__init__.py"]) {
        match iqra_nass(&masar, AQSA_NASS)
            .as_deref()
            .and_then(isdar_renpy)
        {
            Some((kabir, sagheer, tasheeh)) => {
                isdar_maqru = Some((kabir, sagheer, tasheeh));
                arjah(&mut mutalab, AilatMuharrik::Renpy, 97);
                hasila.isdar = Some(IsdarMuharrik {
                    kabir,
                    sagheer,
                    tasheeh,
                    khaam: format!("{kabir}.{sagheer}.{tasheeh}"),
                    mushtaqq: false,
                });
                let wasf = format!(
                    "renpy/__init__.py declares version_tuple ({kabir}, {sagheer}, {tasheeh})"
                );
                hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 97);
            },
            None => hasila.sajjil(
                NawDaleel::BayanatMudmaja,
                "renpy/__init__.py is present and carries no readable version_tuple, so \
                 the version has to be bounded from the build directory names instead",
                Some(nisbi),
                60,
            ),
        }
    }

    let mut bina: Vec<BinaRenpy> = Vec::new();
    if let Some((nisbi, masar)) = awwal_mujallad(siyaq, &["lib"]) {
        for (ism, mujallad) in madakhil(&masar) {
            if mujallad && let Some(wahda) = bina_renpy(&ism) {
                bina.push(wahda);
            }
        }
        if !bina.is_empty() {
            arjah(&mut mutalab, AilatMuharrik::Renpy, 80);
            let asmaa: Vec<String> = bina.iter().map(|wahda| wahda.ism.clone()).collect();
            let wasf = format!(
                "lib/ holds {} platform builds: {}",
                bina.len(),
                asmaa_mujmaa(&asmaa)
            );
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 80);

            let mut mimariyat: Vec<String> = bina
                .iter()
                .map(|wahda| wahda.mimariya_khaam.clone())
                .collect();
            mimariyat.sort();
            mimariyat.dedup();
            if mimariyat.len() == 1 {
                hasila.mimariya = bina.first().and_then(|wahda| wahda.mimariya);
            }
            let mut manassat: Vec<String> =
                bina.iter().map(|wahda| wahda.manassa.clone()).collect();
            manassat.sort();
            manassat.dedup();
            let wasf = format!(
                "the builds cover {} for {}",
                asmaa_mujmaa(&manassat),
                asmaa_mujmaa(&mimariyat)
            );
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, None, 45);
        }
    }

    renpy_tashkeel(&mut hasila, isdar_maqru, &bina);
    renpy_bayanat(siyaq, &mut hasila, &mut mutalab, &arshifa, script.as_ref());

    if let Some((aila, _)) = mutalab {
        hasila.aila = Some(aila);
        hasila.khalfiya = Some(KhalfiyaBarmajiya::Python);
        hasila.daa_itar(ItarNusus::NassRenpy);
        if let Some((nisbi, masar)) = tanfidhi_renpy(siyaq) {
            let wasf = format!("the launcher is {nisbi}, beside its Python entry point");
            hasila.tanfidhi = Some(masar);
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 25);
        }
    }
    hasila
}

/// Records which of Phase 10's two Ren'Py paths this game needs, and how that
/// was decided.
///
/// The distinction between reading the version and inferring it is carried in
/// the evidence text and in its weight, because a maintainer looking at a wrong
/// answer needs to know which of the two produced it.
fn renpy_tashkeel(hasila: &mut HasilatFahs, isdar: Option<(u16, u16, u16)>, bina: &[BinaRenpy]) {
    if let Some((kabir, sagheer, tasheeh)) = isdar {
        let wasf = if yashkul_renpy(kabir, sagheer) {
            format!(
                "Ren'Py {kabir}.{sagheer}.{tasheeh} is at or above 7.4, so the engine \
                 bundles HarfBuzz and FriBidi and shapes and reorders Arabic itself"
            )
        } else {
            format!(
                "Ren'Py {kabir}.{sagheer}.{tasheeh} is below 7.4, so the engine has no \
                 shaper and Arabic has to be laid out and drawn by Taarib"
            )
        };
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            wasf,
            Some("renpy/__init__.py".to_owned()),
            95,
        );
        return;
    }
    if bina.is_empty() {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "neither renpy/__init__.py nor a lib/ build directory was readable, so nothing \
             here bounds the Ren'Py version and the adapter path cannot be chosen from \
             this probe alone",
            None,
            20,
        );
        return;
    }
    let wasf = if bina.iter().any(|wahda| wahda.python.is_some()) {
        let lugha = if bina.iter().any(|wahda| wahda.python == Some(3)) {
            "Python 3"
        } else {
            "Python 2"
        };
        format!(
            "no version_tuple was readable; lib/ carries the py2 and py3 prefixes Ren'Py \
             introduced at 7.4, on {lugha}, so this build is 7.4 or later and the engine \
             can shape Arabic itself — inferred from directory names, not read"
        )
    } else {
        "no version_tuple was readable; lib/ carries no py2 or py3 prefix, and Ren'Py \
         introduced those at 7.4, so this build is 7.3 or earlier and has no shaper — \
         inferred from directory names, not read"
            .to_owned()
    };
    hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some("lib".to_owned()), 55);
}

/// Reads the containers that confirm the shape: the archives, the compiled
/// script, and any translation the game already ships.
fn renpy_bayanat(
    siyaq: &SiyaqFahs<'_>,
    hasila: &mut HasilatFahs,
    mutalab: &mut Option<(AilatMuharrik, u8)>,
    arshifa: &[String],
    script: Option<&(String, PathBuf)>,
) {
    if !arshifa.is_empty() {
        arjah(mutalab, AilatMuharrik::Renpy, 90);
        let mut isdarat: Vec<String> = Vec::new();
        for ism in arshifa.iter().take(AQSA_ASMAA) {
            let Ok(masar) = siyaq.dakhil(&format!("game/{ism}")) else {
                continue;
            };
            if let Some(isdar) = isdar_rpa(&masar)
                && !isdarat.contains(&isdar)
            {
                isdarat.push(isdar);
            }
        }
        let wasf = if isdarat.is_empty() {
            format!(
                "game/ holds {} .rpa archives with no readable header",
                arshifa.len()
            )
        } else {
            format!(
                "game/ holds {} archives headed {}: {}",
                arshifa.len(),
                asmaa_mujmaa(&isdarat),
                asmaa_mujmaa(arshifa)
            )
        };
        hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some("game".to_owned()), 90);
    }

    if let Some((nisbi, masar)) = script {
        let thani = rpyc_thani(masar);
        let wazn = if thani { 92 } else { 78 };
        arjah(mutalab, AilatMuharrik::Renpy, wazn);
        let wasf = if thani {
            format!("{nisbi} opens with the RENPY RPC2 magic")
        } else {
            format!("{nisbi} is present with no RPC2 magic, so it predates Ren'Py 6.18")
        };
        hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.clone()), wazn);
    }

    if let Some((nisbi, masar)) = awwal_mujallad(siyaq, &["game/tl"]) {
        let mut lughat: Vec<String> = madakhil(&masar)
            .into_iter()
            .filter(|(ism, mujallad)| *mujallad && ism.as_str() != "None")
            .map(|(ism, _)| ism)
            .collect();
        lughat.sort();
        if !lughat.is_empty() {
            let wasf = format!(
                "the game already ships translations for {}",
                asmaa_mujmaa(&lughat)
            );
            hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 70);
        }
    }
}

// ---------------------------------------------------------------------------
// GameMaker Studio
//
// One file holds the entire game: `data.win` on Windows, `game.unx` on Linux,
// `game.ios` on the Apple platforms. It is an IFF-like container — a `FORM`
// header and then a flat table of four-character chunks — and it is routinely
// gigabytes, so it is walked rather than read.
//
// Phase 10 rewrites `STRG`, appends a texture page to `TXTR` and generates a
// `FONT` entry addressing it. All three of those chunks changed layout between
// runtime generations, and the bytecode version in `GEN8` is what says which
// generation this is. A game whose container has no `FONT` chunk at all is a
// game that draws its text some other way, and Phase 10 has to find out how
// before it can do anything — so the chunk list is reported in full rather than
// reduced to a yes or no.
// ---------------------------------------------------------------------------

/// One chunk of a `FORM` container.
#[derive(Debug, Clone)]
struct QitaForm {
    /// The four-character chunk name.
    ism: String,
    /// Where its payload starts in the file.
    izaha: u64,
    /// How long the payload is, as the chunk header declares.
    tul: u64,
}

/// The result of walking a `FORM` chunk table.
#[derive(Debug, Clone)]
struct TabulForm {
    /// The payload length the `FORM` header declares.
    mualan: u64,
    /// Every chunk the walk reached.
    qita: Vec<QitaForm>,
    /// Whether the walk stopped early — a length that leaves the file, a name
    /// that is not four printable characters, a read that came up short.
    mabtura: bool,
}

impl TabulForm {
    /// One chunk by name.
    fn qita_bi_ism(&self, ism: &str) -> Option<&QitaForm> {
        self.qita.iter().find(|qita| qita.ism == ism)
    }
}

/// Walks the `FORM` chunk table.
///
/// The header is eight bytes — the ASCII `FORM` and a little-endian payload
/// length — and each chunk that follows is another eight: a four-character name
/// and a little-endian payload length. The walk seeks from one chunk header to
/// the next and never reads a payload, so the cost of examining a four-gigabyte
/// container is a few dozen eight-byte reads.
///
/// Every step is checked against the real file length rather than against the
/// length the container claims for itself, because the two disagree in practice:
/// a `data.win` truncated by a failed download, and a `data.win` a modding tool
/// rebuilt without fixing the `FORM` size, are both things people have on disk.
fn qita_form(malaf: &mut File, tul_malaf: u64) -> Option<TabulForm> {
    let tarwisa = iqra_izaha(malaf, tul_malaf, 0, 8)?;
    if !tarwisa.starts_with(b"FORM") {
        return None;
    }
    let mualan = u64::from(u32_le(&tarwisa, 4)?);
    let nihaya = 8_u64.saturating_add(mualan).min(tul_malaf);
    let mut qita: Vec<QitaForm> = Vec::new();
    let mut izaha = 8_u64;
    let mut mabtura = false;

    while izaha < nihaya && qita.len() < AQSA_QITA {
        let Some(ras) = iqra_izaha(malaf, tul_malaf, izaha, 8) else {
            mabtura = true;
            break;
        };
        let (Some(ism_bayt), Some(tul)) = (ras.get(..4), u32_le(&ras, 4).map(u64::from)) else {
            mabtura = true;
            break;
        };
        if !ism_bayt.iter().all(u8::is_ascii_graphic) {
            mabtura = true;
            break;
        }
        let Some(bidaya) = izaha.checked_add(8) else {
            mabtura = true;
            break;
        };
        let Some(baad) = bidaya.checked_add(tul) else {
            mabtura = true;
            break;
        };
        if baad > nihaya {
            mabtura = true;
            break;
        }
        qita.push(QitaForm {
            ism: String::from_utf8_lossy(ism_bayt).into_owned(),
            izaha: bidaya,
            tul,
        });
        izaha = baad;
    }
    Some(TabulForm {
        mualan,
        qita,
        mabtura,
    })
}

/// What the `GEN8` chunk carries.
///
/// The layout below is the one every GameMaker runtime from Studio 1.4 onward
/// writes, byte for byte:
///
/// ```text
/// offset  size  meaning
/// 0       1     debugger disabled
/// 1       1     bytecode version
/// 2       2     padding
/// 4       4     pointer into STRG: the project filename
/// 8       4     pointer into STRG: the configuration name
/// 12      4     last object id
/// 16      4     last tile id
/// 20      4     game id
/// 24      16    legacy DirectPlay GUID, zeroed by every modern runtime
/// 40      4     pointer into STRG: the game name
/// 44      4     runtime major
/// 48      4     runtime minor
/// 52      4     runtime release
/// 56      4     runtime build
/// 60      4     default window width
/// 64      4     default window height
/// 68      4     info flags
/// 72      4     licence CRC32
/// 76      16    licence MD5
/// 92      8     build timestamp, Unix seconds
/// 100     4     pointer into STRG: the display name
/// 104     8     active targets
/// 112     8     function classifications
/// 120     4     Steam application id
/// 124     4     debugger port, only when the bytecode version is 14 or above
/// ```
///
/// Everything from offset 92 onward is read as optional, because the oldest
/// containers stop short of it and a probe that required the whole structure
/// would see nothing at all in a game that is otherwise perfectly readable.
#[derive(Debug, Clone)]
struct BayanGen8 {
    /// The bytecode version, which is what actually decides chunk layout.
    bytecode: u8,
    /// Whether the runtime shipped with the debugger switched off.
    munaqqih_muattal: bool,
    /// The project's game id.
    muarrif: u32,
    /// Runtime major, minor, release and build.
    isdar: (u32, u32, u32, u32),
    /// The default window size.
    nafidha: (u32, u32),
    /// The build timestamp in Unix seconds, when the chunk reaches that far.
    waqt: Option<u64>,
    /// The Steam application id, when the chunk reaches that far. Zero means
    /// the project has none.
    steam: Option<i32>,
    /// Pointer into `STRG` to the game's name.
    mu_ashir_ism: u32,
    /// Pointer into `STRG` to the display name.
    mu_ashir_ard: Option<u32>,
}

/// Parses a `GEN8` payload.
fn bayan_gen8(bayt: &[u8]) -> Option<BayanGen8> {
    Some(BayanGen8 {
        munaqqih_muattal: *bayt.first()? != 0,
        bytecode: *bayt.get(1)?,
        muarrif: u32_le(bayt, 20)?,
        mu_ashir_ism: u32_le(bayt, 40)?,
        isdar: (
            u32_le(bayt, 44)?,
            u32_le(bayt, 48)?,
            u32_le(bayt, 52)?,
            u32_le(bayt, 56)?,
        ),
        nafidha: (u32_le(bayt, 60)?, u32_le(bayt, 64)?),
        waqt: u64_le(bayt, 92),
        mu_ashir_ard: u32_le(bayt, 100),
        steam: i32_le(bayt, 120),
    })
}

/// Resolves a `STRG` pointer to the string it names.
///
/// GameMaker stores a string as a little-endian length followed by its UTF-8
/// bytes and a terminating zero, and every pointer to it addresses the bytes
/// rather than the length — so the length is four bytes *behind* the pointer.
/// Both the pointer and the length it yields are checked against the file before
/// anything is read, no more than a kilobyte is taken whatever the length
/// claims, and the result is cut at the first zero byte regardless of both.
fn nass_strg(malaf: &mut File, tul_malaf: u64, mu_ashir: u32) -> Option<String> {
    let izaha = u64::from(mu_ashir);
    let bidaya = izaha.checked_sub(4)?;
    let bayt = match iqra_izaha(malaf, tul_malaf, bidaya, 1028) {
        Some(bayt) => bayt,
        None => iqra_izaha(malaf, tul_malaf, bidaya, 68)?,
    };
    let tul = usize::try_from(u32_le(&bayt, 0)?).ok()?;
    let mutah = bayt.get(4..)?;
    let juz = mutah.get(..tul.min(mutah.len()))?;
    let nihaya = juz
        .iter()
        .position(|wahda| *wahda == 0)
        .unwrap_or(juz.len());
    let nass = String::from_utf8_lossy(juz.get(..nihaya)?).into_owned();
    (!nass.is_empty()).then_some(nass)
}

/// Container names, in the order they are worth trying.
const HAWIYAT_GAMEMAKER: [&str; 9] = [
    "data.win",
    "Data.win",
    "game.unx",
    "game.ios",
    "game.droid",
    "assets/game.unx",
    "assets/data.win",
    "assets/game.droid",
    "assets/game.ios",
];

/// The chunks Phase 10 touches, reported by name whether present or absent.
const QITA_MUHIMMA: [&str; 6] = ["STRG", "TXTR", "FONT", "CODE", "SPRT", "AUDO"];

/// Executable name prefixes that are never the game.
const MUSTATHNAYAT_TANFIDH: [&str; 7] = [
    "unins",
    "vcredist",
    "dxwebsetup",
    "dotnetfx",
    "oalinst",
    "directx",
    "crashhandler",
];

/// Finds the container, including inside a macOS application bundle.
fn hawiyat_gamemaker(siyaq: &SiyaqFahs<'_>) -> Option<(String, PathBuf)> {
    if let Some(natija) = awwal_malaf(siyaq, &HAWIYAT_GAMEMAKER) {
        return Some(natija);
    }
    for (ism, mujallad) in madakhil(siyaq.jidhr) {
        if !mujallad || !ism.to_ascii_lowercase().ends_with(".app") {
            continue;
        }
        for lahiqa in ["game.ios", "game.unx", "data.win"] {
            let nisbi = format!("{ism}/Contents/Resources/{lahiqa}");
            if let Ok(masar) = siyaq.dakhil(&nisbi)
                && masar.is_file()
            {
                return Some((nisbi, masar));
            }
        }
    }
    None
}

/// The single Windows executable in a directory, when there is exactly one that
/// could be a game.
///
/// Exactly one, or none: a directory with two candidates is a directory where
/// choosing would be guessing, and discovery already names the executable for
/// most games anyway.
fn tanfidhi_wahid(nizam: NizamTashghil, mujallad: &Path) -> Option<PathBuf> {
    if !matches!(nizam, NizamTashghil::Windows) {
        return None;
    }
    let mut wahid: Option<String> = None;
    for (ism, dakhili) in madakhil(mujallad) {
        let munkhafid = ism.to_ascii_lowercase();
        if dakhili || !imtidad(&munkhafid, "exe") {
            continue;
        }
        if MUSTATHNAYAT_TANFIDH
            .iter()
            .any(|mustathna| munkhafid.starts_with(mustathna))
        {
            continue;
        }
        if wahid.is_some() {
            return None;
        }
        wahid = Some(ism);
    }
    wahid.map(|ism| mujallad.join(ism))
}

/// The GameMaker Studio detector.
fn gamemaker(siyaq: &SiyaqFahs<'_>) -> HasilatFahs {
    let mut hasila = HasilatFahs::la_shay();
    let Some((nisbi, masar)) = hawiyat_gamemaker(siyaq) else {
        return hasila;
    };
    let (Some(tul_malaf), Ok(mut malaf)) = (hajm(&masar), File::open(&masar)) else {
        let wasf = format!("{nisbi} is present and could not be opened");
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 30);
        return hasila;
    };
    let Some(tabul) = qita_form(&mut malaf, tul_malaf) else {
        let wasf = format!("{nisbi} is present and does not open with a FORM header");
        hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 30);
        return hasila;
    };
    let Some(qita_gen8) = tabul.qita_bi_ism("GEN8").cloned() else {
        let wasf = format!(
            "{nisbi} is a FORM container with no GEN8 chunk, so it is an IFF file of \
             some other kind rather than a GameMaker game"
        );
        hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 25);
        return hasila;
    };

    hasila.aila = Some(AilatMuharrik::GameMaker);
    hasila.khalfiya = Some(KhalfiyaBarmajiya::GameMakerVm);
    hasila.daa_itar(ItarNusus::RasmGameMaker);
    let wasf = format!(
        "{nisbi} is a FORM container of {tul_malaf} bytes declaring {} and carrying a \
         GEN8 chunk of {} bytes",
        tabul.mualan, qita_gen8.tul
    );
    hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.clone()), 97);

    gamemaker_gen8(&mut malaf, tul_malaf, &qita_gen8, &nisbi, &mut hasila);
    gamemaker_qita(&tabul, &nisbi, &mut hasila);

    if let Some(mujallad) = masar.parent()
        && let Some(tanfidhi) = tanfidhi_wahid(siyaq.nizam, mujallad)
    {
        let wasf = format!("the runner executable is {}", mawqi_nisbi(siyaq, &tanfidhi));
        hasila.tanfidhi = Some(tanfidhi);
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 25);
    }
    hasila
}

/// Reads the `GEN8` chunk and records what it says.
fn gamemaker_gen8(
    malaf: &mut File,
    tul_malaf: u64,
    qita: &QitaForm,
    nisbi: &str,
    hasila: &mut HasilatFahs,
) {
    let hadd = usize::try_from(qita.tul.min(160)).unwrap_or(160);
    let bayt = iqra_izaha(malaf, tul_malaf, qita.izaha, hadd);
    let Some(bayan) = bayt.as_deref().and_then(bayan_gen8) else {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "the GEN8 chunk is shorter than the fields the runtime version lives in, so \
             the version could not be read from it",
            Some(nisbi.to_owned()),
            60,
        );
        return;
    };
    let (kabir, sagheer, tasheeh, bina) = bayan.isdar;
    let khaam = format!("{kabir}.{sagheer}.{tasheeh}.{bina}");
    hasila.isdar = Some(IsdarMuharrik {
        kabir: u16::try_from(kabir).unwrap_or(0),
        sagheer: u16::try_from(sagheer).unwrap_or(0),
        tasheeh: u16::try_from(tasheeh).unwrap_or(0),
        khaam: khaam.clone(),
        mushtaqq: false,
    });
    let wasf = format!("GEN8 records runtime version {khaam}");
    hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 92);

    let wasf = format!(
        "the container's bytecode version is {}, which is what decides the layout of \
         the STRG, TXTR and FONT chunks Phase 10 rewrites",
        bayan.bytecode
    );
    hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 88);

    if let Some(ism) = nass_strg(malaf, tul_malaf, bayan.mu_ashir_ism) {
        let wasf = format!("GEN8 names the game {}", iqtibas(&ism));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 70);
    }
    if let Some(mu_ashir) = bayan.mu_ashir_ard
        && let Some(ard) = nass_strg(malaf, tul_malaf, mu_ashir)
    {
        let wasf = format!("its display name is {}", iqtibas(&ard));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 45);
    }
    let (ard, irtifa) = bayan.nafidha;
    let wasf = format!(
        "game id {}, default window {ard} by {irtifa}, debugger {}",
        bayan.muarrif,
        if bayan.munaqqih_muattal {
            "disabled"
        } else {
            "enabled"
        }
    );
    hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 40);

    if let Some(waqt) = bayan.waqt.filter(|qeema| *qeema > 0) {
        let wasf = format!("the container was built at Unix time {waqt}");
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 30);
    }
    if let Some(steam) = bayan.steam.filter(|qeema| *qeema > 0) {
        let wasf = format!("the project declares Steam application {steam}");
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.to_owned()), 55);
    }
}

/// Records the chunk table, and what its absences mean.
fn gamemaker_qita(tabul: &TabulForm, nisbi: &str, hasila: &mut HasilatFahs) {
    let asmaa: Vec<String> = tabul.qita.iter().map(|qita| qita.ism.clone()).collect();
    let wasf = format!(
        "the container holds {} chunks: {}",
        asmaa.len(),
        asmaa_mujmaa(&asmaa)
    );
    hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.to_owned()), 88);

    for ism in QITA_MUHIMMA {
        if let Some(qita) = tabul.qita_bi_ism(ism) {
            let wasf = format!("{ism} is present, {} bytes", qita.tul);
            hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.to_owned()), 50);
        }
    }
    if tabul.qita_bi_ism("FONT").is_none() {
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            "there is no FONT chunk, so this game's fonts are not GameMaker font \
             resources and Phase 10 cannot generate one alongside them — the text is \
             drawn from sprites, from an extension, or from a runtime the container \
             does not describe, and it has to be found before anything can be patched",
            Some(nisbi.to_owned()),
            65,
        );
    }
    if tabul.qita_bi_ism("STRG").is_none() {
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            "there is no STRG chunk, so there is no string pool to translate into",
            Some(nisbi.to_owned()),
            60,
        );
    }
    if tabul.mabtura {
        hasila.sajjil(
            NawDaleel::TarwisatHawiya,
            "the chunk walk stopped early — a chunk length ran past the end of the file, \
             a chunk name was not four printable characters, or the file is truncated; \
             the chunks listed are the ones reached before that point",
            Some(nisbi.to_owned()),
            35,
        );
    }
}

// ---------------------------------------------------------------------------
// Electron and NW.js
//
// The hardest case in the phase, and the only one where the honest answer is
// two answers. A Chromium shell wraps something, and what it wraps is a
// different question from what wraps it:
//
// - **The inner engine decides how text is drawn.** RPG Maker MZ inside NW.js
//   draws through `Bitmap.prototype.drawText` onto a canvas. A Godot HTML5
//   export inside Electron draws through its own WebGL renderer. A visual novel
//   written against the DOM draws through the browser's own text layout, which
//   already shapes Arabic correctly and only needs direction set.
// - **The wrapper decides how the patch is installed.** An `app.asar` is
//   unpacked, injected into and repacked. An unpacked `resources/app/` is
//   written into directly. A preload script is registered one way for Electron
//   and another for NW.js.
//
// Both are true at once, and downstream needs both. So this detector claims
// `Electron` with its own weight and records what it saw inside the archive,
// and the RPG Maker detector independently claims its own family after looking
// in `resources/app/`. Neither suppresses the other, and [`crate::tahdid`] gets
// the whole picture instead of one detector's summary of it.
// ---------------------------------------------------------------------------

/// How deep the asar directory tree is walked.
const AQSA_UMQ_ASAR: usize = 4;

/// How many asar directory nodes are visited before the walk stops.
const AQSA_UQAD_ASAR: usize = 20_000;

/// Filenames that mean the game draws its own text into a canvas, where the
/// browser's shaping never runs and Phase 10 has to route through the
/// WebAssembly core.
const ALAMAT_LAWHA: [&str; 16] = [
    "rpg_core.js",
    "rmmz_core.js",
    "phaser.js",
    "phaser.min.js",
    "pixi.js",
    "pixi.min.js",
    "pixi-legacy.js",
    "pixi-legacy.min.js",
    "c2runtime.js",
    "c3runtime.js",
    "love.js",
    "melonjs.min.js",
    "createjs.min.js",
    "easeljs.min.js",
    "unityloader.js",
    "excalibur.js",
];

/// What a walk of an asar directory found.
#[derive(Debug, Default, Clone)]
struct HasadAsar {
    /// The top-level entry names, which is what gets reported.
    madakhil: Vec<String>,
    /// Canvas-runtime filenames found anywhere in the walked depth.
    alamat: Vec<String>,
    /// Whether any HTML document is packed at all.
    mustanad: bool,
    /// How many nodes were visited.
    uqad: usize,
    /// Whether the walk hit its node ceiling before finishing.
    mabtur: bool,
}

/// Walks an asar directory listing, in memory, without extracting anything.
fn amshi_asar(fihris: &Value) -> HasadAsar {
    let mut hasad = HasadAsar::default();
    let Some(Value::Object(judhur)) = fihris.get("files") else {
        return hasad;
    };
    hasad.madakhil = judhur.keys().cloned().collect();
    let mut kudsa: Vec<(&String, &Value, usize)> = judhur
        .iter()
        .map(|(ism, qeema)| (ism, qeema, 1_usize))
        .collect();

    while let Some((ism, qeema, umq)) = kudsa.pop() {
        hasad.uqad = hasad.uqad.saturating_add(1);
        if hasad.uqad > AQSA_UQAD_ASAR {
            hasad.mabtur = true;
            break;
        }
        let munkhafid = ism.to_ascii_lowercase();
        if imtidad(&munkhafid, "html") || imtidad(&munkhafid, "htm") {
            hasad.mustanad = true;
        }
        if alamat_lawha(&munkhafid) && !hasad.alamat.contains(ism) {
            hasad.alamat.push(ism.clone());
        }
        if umq < AQSA_UMQ_ASAR
            && let Some(Value::Object(abna)) = qeema.get("files")
        {
            let asfal = umq.saturating_add(1);
            kudsa.extend(abna.iter().map(|(far, qeema)| (far, qeema, asfal)));
        }
    }
    hasad
}

/// What reading an asar header produced.
#[derive(Debug, Clone)]
enum QiraatAsar {
    /// The header framed correctly and its directory parsed.
    Maqru(Box<MaalumatAsar>),
    /// The header declared a length past [`AQSA_TARWISAT_ASAR`] and was refused
    /// unread.
    Mubalagh(u64),
    /// The framing is sound and the directory is not valid JSON.
    LaYuqra,
    /// The file does not carry asar framing at all.
    Ghayr,
}

/// An asar header, read and understood.
#[derive(Debug, Clone)]
struct MaalumatAsar {
    /// The header pickle's length, from offset 4.
    hajm_tarwisa: u64,
    /// The declared length of the JSON directory, from offset 12.
    tul_json: u64,
    /// Where packed file contents begin: eight plus the header length.
    bidayat_bayanat: u64,
    /// The file's own length.
    tul_malaf: u64,
    /// What the directory walk found.
    hasad: HasadAsar,
}

/// Reads an asar header.
///
/// An asar begins with two Chromium `Pickle` structures, and the framing is
/// fixed:
///
/// ```text
/// offset  size  meaning
/// 0       4     the first pickle's payload size, always 4
/// 4       4     the header pickle's total length
/// 8       4     the header pickle's payload size, four less than the above
/// 12      4     the length of the JSON directory string
/// 16      n     the JSON directory
/// ```
///
/// Packed file contents begin at `8 + header length`, padded to a four-byte
/// boundary. Every one of those numbers comes out of a file inside a game
/// directory, which is not a trusted source: the declared header length is
/// checked against [`AQSA_TARWISAT_ASAR`] before any allocation, the JSON
/// length is checked against the header length, and the start of the contents
/// is checked against the real file length. Nothing is extracted, and the
/// directory is read for its shape rather than its contents.
fn qira_asar(masar: &Path) -> QiraatAsar {
    let (Some(tul_malaf), Ok(mut malaf)) = (hajm(masar), File::open(masar)) else {
        return QiraatAsar::Ghayr;
    };
    let Some(itar) = iqra_izaha(&mut malaf, tul_malaf, 0, 16) else {
        return QiraatAsar::Ghayr;
    };
    let (Some(4), Some(hajm_tarwisa), Some(tul_json)) = (
        u32_le(&itar, 0),
        u32_le(&itar, 4).map(u64::from),
        u32_le(&itar, 12).map(u64::from),
    ) else {
        return QiraatAsar::Ghayr;
    };
    if hajm_tarwisa > AQSA_TARWISAT_ASAR {
        return QiraatAsar::Mubalagh(hajm_tarwisa);
    }
    let Some(bidayat_bayanat) = 8_u64.checked_add(hajm_tarwisa) else {
        return QiraatAsar::Ghayr;
    };
    let sahih =
        bidayat_bayanat <= tul_malaf && tul_json.saturating_add(4) <= hajm_tarwisa && tul_json > 0;
    if !sahih {
        return QiraatAsar::Ghayr;
    }
    let Ok(tul) = usize::try_from(tul_json) else {
        return QiraatAsar::Ghayr;
    };
    let Some(bayt) = iqra_izaha(&mut malaf, tul_malaf, 16, tul) else {
        return QiraatAsar::Ghayr;
    };
    let Ok(fihris) = serde_json::from_slice::<Value>(&bayt) else {
        return QiraatAsar::LaYuqra;
    };
    QiraatAsar::Maqru(Box::new(MaalumatAsar {
        hajm_tarwisa,
        tul_json,
        bidayat_bayanat,
        tul_malaf,
        hasad: amshi_asar(&fihris),
    }))
}

/// Every shell marker, with what it is worth and what it actually means.
const ALAMAT_GHILAF: [(&str, u8, &str); 17] = [
    ("resources/app.asar", 88, "an Electron application archive"),
    (
        "Contents/Resources/app.asar",
        88,
        "an application archive inside a macOS bundle",
    ),
    (
        "resources/electron.asar",
        92,
        "Electron's own bundled archive",
    ),
    (
        "resources/default_app.asar",
        90,
        "Electron's default application archive",
    ),
    (
        "resources/app/package.json",
        76,
        "an unpacked Electron application",
    ),
    (
        "Contents/Frameworks/Electron Framework.framework",
        92,
        "the Electron framework",
    ),
    ("nw.pak", 88, "NW.js's own resource pack"),
    ("nw_100_percent.pak", 66, "NW.js's interface resources"),
    (
        "LICENSES.chromium.html",
        70,
        "the Chromium licence file a shell ships",
    ),
    ("chrome-sandbox", 60, "Chromium's Linux sandbox helper"),
    (
        "chrome_100_percent.pak",
        65,
        "Chromium's interface resources",
    ),
    (
        "chrome_200_percent.pak",
        60,
        "Chromium's high-density interface resources",
    ),
    (
        "icudtl.dat",
        45,
        "Chromium's ICU data, which other embedders ship too",
    ),
    ("v8_context_snapshot.bin", 55, "a V8 context snapshot"),
    ("snapshot_blob.bin", 50, "a V8 startup snapshot"),
    ("resources.pak", 40, "a Chromium resource pack"),
    ("libffmpeg.so", 35, "Chromium's bundled media library"),
];

/// The markers that make this a shell around an application rather than a
/// browser embedded inside something else.
///
/// The distinction is not pedantry. A Unity game with an embedded Chromium view
/// ships `icudtl.dat`, `resources.pak` and a V8 snapshot and is not an Electron
/// game in any sense that matters to Phase 10 — there is no `app.asar` to
/// repack and no renderer to preload into. Claiming the family from the
/// Chromium files alone would send an asar patcher at a game that has no asar.
const ALAMAT_QATIA: [&str; 7] = [
    "resources/app.asar",
    "Contents/Resources/app.asar",
    "resources/electron.asar",
    "resources/default_app.asar",
    "resources/app/package.json",
    "Contents/Frameworks/Electron Framework.framework",
    "nw.pak",
];

/// Walks an unpacked `resources/app/` two levels deep, looking for the same
/// markers the asar walk looks for.
fn hasad_mujallad(siyaq: &SiyaqFahs<'_>, asas: &str) -> HasadAsar {
    let mut hasad = HasadAsar::default();
    let Ok(masar) = siyaq.dakhil(asas) else {
        return hasad;
    };
    for (ism, mujallad) in madakhil(&masar) {
        hasad.uqad = hasad.uqad.saturating_add(1);
        hasad.madakhil.push(ism.clone());
        let munkhafid = ism.to_ascii_lowercase();
        if !mujallad && (imtidad(&munkhafid, "html") || imtidad(&munkhafid, "htm")) {
            hasad.mustanad = true;
        }
        if !mujallad {
            if alamat_lawha(&munkhafid) && !hasad.alamat.contains(&ism) {
                hasad.alamat.push(ism);
            }
            continue;
        }
        let Ok(dakhili) = siyaq.dakhil(&format!("{asas}/{ism}")) else {
            continue;
        };
        for (ibn, _) in madakhil(&dakhili) {
            hasad.uqad = hasad.uqad.saturating_add(1);
            if alamat_lawha(&ibn.to_ascii_lowercase()) && !hasad.alamat.contains(&ibn) {
                hasad.alamat.push(ibn);
            }
        }
    }
    hasad.madakhil.sort();
    hasad
}

/// Whether a lowercased filename names a canvas runtime or a compiled one.
fn alamat_lawha(munkhafid: &str) -> bool {
    ALAMAT_LAWHA.contains(&munkhafid) || imtidad(munkhafid, "pck") || imtidad(munkhafid, "wasm")
}

/// Records which text surfaces the packed application has.
///
/// Both entries can be recorded for one game, and often are: a canvas game with
/// a DOM main menu is an ordinary shape. They are not alternatives — they are
/// two surfaces, and Phase 10 hooks each of them differently. `Dom` means the
/// browser's own layout is already doing correct shaping and only needs
/// direction and a font. `Canvas` means nothing is shaping anything and every
/// glyph has to come from the WebAssembly core.
fn itarat_ghilaf(hasad: &HasadAsar, mawqi: &str, hasila: &mut HasilatFahs) {
    if hasad.mustanad {
        hasila.daa_itar(ItarNusus::Dom);
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "the application packs an HTML document, so there is a DOM whose own layout \
             shapes Arabic correctly once direction and a font are set on it",
            Some(mawqi.to_owned()),
            62,
        );
    }
    if !hasad.alamat.is_empty() {
        hasila.daa_itar(ItarNusus::Canvas);
        let wasf = format!(
            "the application packs {}, which draws text into a canvas — the browser's \
             own shaping never runs there, so that surface goes through the WebAssembly \
             core rather than through CSS",
            asmaa_mujmaa(&hasad.alamat)
        );
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(mawqi.to_owned()), 80);
    }
    if !hasad.mustanad && hasad.alamat.is_empty() {
        hasila.sajjil(
            NawDaleel::BayanatMudmaja,
            "the listing shows neither an HTML document nor a canvas runtime this build \
             recognises, so which text surface the game uses is not decidable from the \
             directory alone and Phase 10 will have to look at runtime",
            Some(mawqi.to_owned()),
            25,
        );
    }
}

/// The Electron and NW.js detector.
fn ghilaf(siyaq: &SiyaqFahs<'_>) -> HasilatFahs {
    let mut hasila = HasilatFahs::la_shay();
    let mut qatia = false;
    let mut adad = 0_usize;

    for (nisbi, wazn, wasf) in ALAMAT_GHILAF {
        if !siyaq.yujad(nisbi) {
            continue;
        }
        adad = adad.saturating_add(1);
        if ALAMAT_QATIA.contains(&nisbi) {
            qatia = true;
        }
        let jumla = format!("{nisbi} is present: {wasf}");
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            jumla,
            Some(nisbi.to_owned()),
            wazn,
        );
    }
    if adad == 0 {
        return hasila;
    }
    if !qatia {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "Chromium runtime files are present with no application archive and no \
             unpacked application beside them, which is what an embedded browser inside \
             another engine looks like as much as a stripped shell; no engine is claimed \
             from these files alone",
            None,
            30,
        );
        return hasila;
    }

    hasila.aila = Some(AilatMuharrik::Electron);
    hasila.khalfiya = Some(KhalfiyaBarmajiya::JavaScript);

    let nwjs = ["nw.pak", "nw.dll", "nw_100_percent.pak", "nw_elf.dll"]
        .iter()
        .any(|nisbi| siyaq.yujad(nisbi));
    let electron = [
        "resources/electron.asar",
        "resources/default_app.asar",
        "LICENSES.chromium.html",
        "Contents/Frameworks/Electron Framework.framework",
        "chrome_100_percent.pak",
    ]
    .iter()
    .any(|nisbi| siyaq.yujad(nisbi));
    let wasf = match (nwjs, electron) {
        (true, true) => {
            "both NW.js and Electron runtime files are present, which is one repacked \
             inside the other; the installer has to identify the shell that actually starts"
        },
        (true, false) => "the shell is NW.js",
        (false, true) => "the shell is Electron",
        (false, false) => "the shell is a Chromium application whose files do not name it",
    };
    hasila.sajjil(NawDaleel::BinyatMujallad, wasf, None, 66);

    ghilaf_hawiya(siyaq, &mut hasila);
    ghilaf_huzma(siyaq, &mut hasila);

    if let Some((jeel, mawqi)) = jeel_chromium(siyaq) {
        let wasf = format!(
            "the shell's Chromium files place it at {}, inferred from {mawqi} rather than \
             read from any version string",
            jeel.wasf()
        );
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(mawqi), 42);
    }
    if let Some(tanfidhi) = tanfidhi_wahid(siyaq.nizam, siyaq.jidhr) {
        let wasf = format!("the shell executable is {}", mawqi_nisbi(siyaq, &tanfidhi));
        hasila.tanfidhi = Some(tanfidhi);
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, None, 20);
    }
    hasila
}

/// Reads the packed application: the asar directory, or the unpacked tree that
/// replaces it.
fn ghilaf_hawiya(siyaq: &SiyaqFahs<'_>, hasila: &mut HasilatFahs) {
    const ARSHIFA: [&str; 2] = ["resources/app.asar", "Contents/Resources/app.asar"];
    let mut qura = false;

    if let Some((nisbi, masar)) = awwal_malaf(siyaq, &ARSHIFA) {
        match qira_asar(&masar) {
            QiraatAsar::Maqru(maal) => {
                qura = true;
                let wasf = format!(
                    "{nisbi} is an asar of {} bytes: a {}-byte header framing a {}-byte \
                     JSON directory, with packed contents starting at offset {}",
                    maal.tul_malaf, maal.hajm_tarwisa, maal.tul_json, maal.bidayat_bayanat
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.clone()), 90);

                let hasad = &maal.hasad;
                let wasf = format!(
                    "its top level holds {} entries: {}",
                    hasad.madakhil.len(),
                    asmaa_mujmaa(&hasad.madakhil)
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.clone()), 82);
                if hasad.mabtur {
                    let wasf = format!(
                        "the directory walk stopped at {} nodes, so the listing above is \
                         the part of the archive that was examined",
                        hasad.uqad
                    );
                    hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi.clone()), 25);
                }
                itarat_ghilaf(hasad, &nisbi, hasila);
            },
            QiraatAsar::Mubalagh(mualan) => {
                let saqf = AQSA_TARWISAT_ASAR;
                let wasf = format!(
                    "{nisbi} declares a {mualan}-byte directory header, past the \
                     {saqf}-byte ceiling this probe reads, so it was refused unread"
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 60);
            },
            QiraatAsar::LaYuqra => {
                let wasf = format!(
                    "{nisbi} frames as an asar and its directory is not valid JSON, so \
                     the archive is there and its contents are unknown"
                );
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 62);
            },
            QiraatAsar::Ghayr => {
                let wasf = format!("{nisbi} is present and carries no asar framing");
                hasila.sajjil(NawDaleel::TarwisatHawiya, wasf, Some(nisbi), 35);
            },
        }
    }

    for asas in ["resources/app", "Contents/Resources/app"] {
        if !siyaq.yujad(asas) {
            continue;
        }
        let hasad = hasad_mujallad(siyaq, asas);
        if hasad.madakhil.is_empty() {
            continue;
        }
        qura = true;
        let wasf = format!(
            "{asas}/ is unpacked and holds {} entries: {}",
            hasad.madakhil.len(),
            asmaa_mujmaa(&hasad.madakhil)
        );
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(asas.to_owned()), 78);
        itarat_ghilaf(&hasad, asas, hasila);
    }

    if !qura {
        hasila.sajjil(
            NawDaleel::BinyatMujallad,
            "the shell is here and nothing inside it could be listed, so the application \
             it wraps is unidentified — the wrapper is still patchable and what it wraps \
             is not yet known",
            None,
            30,
        );
    }
}

/// Reads the application's own manifest, which is where a shell version turns up
/// when it turns up anywhere.
fn ghilaf_huzma(siyaq: &SiyaqFahs<'_>, hasila: &mut HasilatFahs) {
    const HUZAM: [&str; 2] = [
        "resources/app/package.json",
        "Contents/Resources/app/package.json",
    ];
    let Some((nisbi, masar)) = awwal_malaf(siyaq, &HUZAM) else {
        return;
    };
    let Some(bayan) = bayan_huzma(&masar) else {
        let wasf = format!("{nisbi} is present and does not parse as JSON");
        hasila.sajjil(NawDaleel::BinyatMujallad, wasf, Some(nisbi), 30);
        return;
    };
    if let Some(ism) = bayan.ism.as_deref() {
        let isdar = bayan.isdar.as_deref().unwrap_or("no version");
        let wasf = format!(
            "the application calls itself {} {}",
            iqtibas(ism),
            iqtibas(isdar)
        );
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 35);
    }
    if let Some(rais) = bayan.ra_isiy.as_deref() {
        let wasf = format!("its entry point is {}", iqtibas(rais));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 40);
    }
    if let Some(qeema) = bayan.electron.as_deref() {
        let munaqqa = qeema.trim_start_matches(['^', '~', '=', '>', '<', 'v', ' ']);
        if hasila.isdar.is_none() {
            hasila.isdar = Some(isdar_min_nass(munaqqa));
        }
        let wasf = format!("the manifest declares electron {}", iqtibas(qeema));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 70);
    }
    if let Some(qeema) = bayan.nwjs.as_deref() {
        let wasf = format!("the manifest declares nw {}", iqtibas(qeema));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi.clone()), 70);
    }
    if let Some(qeema) = bayan.muamalat.as_deref() {
        let wasf = format!("the shell is launched with {}", iqtibas(qeema));
        hasila.sajjil(NawDaleel::BayanatMudmaja, wasf, Some(nisbi), 35);
    }
}
