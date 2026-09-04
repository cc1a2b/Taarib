//! اللغة الرسمية — the Arabic a game's publisher already ships.
//!
//! A game that ships official Arabic must never be offered for arabization.
//! Patching one would replace paid translators, QA and native review with
//! machine output, and the user would not find out until they were inside the
//! game looking at it. This module decides that question, and it carries its
//! reasons with it, because a verdict that cannot be argued with is a verdict
//! nobody can correct.
//!
//! ## Three independent ways to know
//!
//! | way | what it reads | how much it is worth |
//! | --- | --- | --- |
//! | [`NawDaleelLugha::LughatMatjar`] | the launcher's own declared language metadata | strong for presence, weak for absence |
//! | [`NawDaleelLugha::MawridMuharrik`] | the engine's compiled localization resources | strongest — the game's own files |
//! | [`NawDaleelLugha::MasarThaqafa`] | a locale-named path in the game's own data | strong when the path shape is unambiguous |
//!
//! They are independent on purpose. A store listing can be years out of date in
//! either direction: a publisher who adds Arabic in a patch and forgets the
//! store page, and a publisher who lists Arabic for a console SKU that the PC
//! build never shipped. The game's own files cannot be out of date with
//! themselves.
//!
//! ## Which launchers declare a language list at all
//!
//! Assessed per adapter, and the answer is [`MasdarLughat`]. Steam is the only
//! one this module reads today, because it is the only one whose declaration
//! could be verified against a real installation while this was written; the
//! rest are classified rather than guessed at, so that a caller can tell
//! "this launcher has nothing to say" from "this launcher was not asked".
//!
//! ## Why the deep probe is a trait and not a call
//!
//! The strongest evidence lives inside `.pak`, `.utoc` and `.assets`
//! containers, and the readers for those are
//! `taarib-muhawwil-unreal::mawarid` and `taarib-istikhraj::unity`. This crate
//! cannot call them: `taarib-muhawwil-unreal` depends on `taarib-muharrik`,
//! which depends on this crate, so a direct dependency is a cycle Cargo
//! refuses. Reimplementing a pak reader here would be worse than the cycle —
//! [`crate::fann_luba`] already states the rule this crate lives by, that
//! archives of every kind are left closed and Phase 6's asset reader is where
//! containers are opened.
//!
//! So the deep probe arrives as [`FahisMawarid`], implemented by whoever owns
//! the engine readers, and this module composes it with the two probes
//! discovery can honestly run itself. With no deep probe supplied, a game whose
//! text is sealed inside a container reports what it could not see in
//! [`HukmLughaRasmiya::majhul`] rather than reporting "no Arabic".
//!
//! ## What the cache is keyed on, and why
//!
//! [`MiftahLugha`] is a digest over five things: the detector version, the
//! launcher identity, the launcher's build token, the declared-language list,
//! and the install root.
//!
//! The declared-language list is the one that is easy to leave out and must not
//! be. A publisher who adds Arabic to a shipped game changes two things — the
//! files, which move the build token, and the store listing, which moves this
//! list — but they do not always change them in the same week, and the listing
//! usually moves first. A cache keyed on the build alone would keep offering to
//! arabize a game that had just stopped needing it, for as long as the user
//! declined to update. Keying on both means either one invalidates.
//!
//! The list is cheap to obtain — one read of `appinfo.vdf` per scan, shared
//! across every Steam game — and the walk it guards is the expensive part, so
//! the key costs nothing to compute and saves everything.
//!
//! ## Where the cache is not
//!
//! Nowhere in this crate. [`crate::Kashif`] persists nothing and does not
//! depend on `taarib-makhzan`, for the reasons stated at the crate root, and
//! this module does not change that: [`KhazinatLugha`] is a port, and
//! [`KhazinaDhakira`] is the in-memory implementation used when a caller has
//! not supplied a persistent one.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use parking_lot::Mutex;

use taarib_mustalahat::luba::{
    DaleelLugha, HukmLughaRasmiya, LubaId, MasdarLuba, NawDaleelLugha, TughtiyaLugha,
};
use walkdir::WalkDir;

use crate::fahs::TanbihFahs;
use crate::matajir::vdf::{self, QeemaVdf};

// ---------------------------------------------------------------------------
// Bounds and constants
// ---------------------------------------------------------------------------

/// The version of this detector.
///
/// Part of [`MiftahLugha`], so that a verdict produced by an older and wronger
/// detector is recomputed rather than served. Bumped whenever a rule below
/// changes in a way that could change an answer.
pub const ISDAR_FAHS: u32 = 1;

/// How deep the locale-path walk descends below the install root.
///
/// Six. `Atlas/Content/Localization/Game/ar/Game.locres` is five, and
/// `<Game>_Data/StreamingAssets/aa/StandaloneWindows64/<bundle>` is four; six
/// leaves one level of slack for a project directory nested one deeper than
/// usual without opening the walk onto an asset tree.
const HADD_UMQ: usize = 6;

/// How many directory entries the walk looks at before it stops.
///
/// Forty thousand. A localization directory is small and near the top of a
/// game; grinding a `Content/Paks` tree or a Unity `level*` set to prove there
/// is no `ar` folder in it costs the user a pause and answers nothing, because
/// the answer for those games is inside the containers and only
/// [`FahisMawarid`] can read it.
const HADD_MUDAKHALAT: usize = 40_000;

/// How many locale-shaped paths are kept.
///
/// A game with more than this many locale directories has a directory shape
/// that is not a localization layout, and reading all of it is work nobody
/// asked for.
const HADD_MASARAT: usize = 256;

/// Directory names the walk refuses to descend into.
///
/// Every one of them is an asset tree with six figures of entries and no
/// localization directory anywhere inside it.
const MUJALLADAT_MASTUBAADA: [&str; 10] = [
    "paks",
    "movies",
    "shaders",
    "shadercache",
    "audio",
    "sound",
    "textures",
    "meshes",
    "animations",
    "mono",
];

/// The share of the reference culture's entries at which an Arabic culture is
/// treated as covering everything the game localizes.
///
/// Eighty percent. Below it, a culture is a partial localization; at or above
/// it, the publisher translated the game rather than its menus. The figure is a
/// judgement and is stated here so it can be argued with: Little Nightmares
/// Enhanced Edition ships `ar` at 345 entries against a 350-entry reference,
/// which is ninety-eight percent and unambiguous, while a menus-only
/// localization sits an order of magnitude below its reference.
const HADD_TAKAFU: u64 = 80;

/// Weight of a compiled Arabic culture found inside the game's own resources.
const WAZN_MAWRID: u8 = 95;

/// Weight of a compiled Arabic culture that turned out to hold no Arabic script.
const WAZN_MAWRID_FARIGH: u8 = 90;

/// Weight of a locale-shaped path found under the install root.
const WAZN_MASAR: u8 = 70;

/// Weight of a launcher declaring Arabic.
const WAZN_MATJAR: u8 = 75;

/// Weight of a launcher declaring a language list that does not contain Arabic.
///
/// Lower than the positive, deliberately. A listing that names Arabic is an
/// assertion somebody made; a listing that omits it is equally consistent with
/// a publisher who added Arabic last month and has not touched the store page.
const WAZN_MATJAR_NAFI: u8 = 55;

/// How much confidence each unanswered question costs.
const KHASM_MAJHUL: u8 = 15;

/// The most confidence unanswered questions can cost between them.
///
/// Forty-five. Past this the deduction would swamp the evidence: a verdict
/// resting on a compiled Arabic culture in the game's own files is worth
/// something even when four other probes had nothing to say.
const AQSA_KHASM: u8 = 45;

// ---------------------------------------------------------------------------
// Recognising Arabic, and recognising a culture code at all
// ---------------------------------------------------------------------------

/// Culture codes and language names that mean Arabic, lowercased.
///
/// Exact tokens rather than prefixes: `ar` is Arabic and `art` is a directory
/// somebody named after their art assets, and a prefix match would confuse
/// them. Region-qualified codes are handled by [`huwa_arabi`] separately,
/// because there are two dozen of them and listing them would be a list that
/// goes stale.
const RUMUZ_ARABIYA: [&str; 5] = ["ar", "ara", "arb", "arabic", "arabe"];

/// The spelled-out language names that count as culture positions.
///
/// This is Steam's own closed vocabulary, taken from the `supported_languages`
/// subtables in a real `appinfo.vdf`, and it doubles as the list of directory
/// names an engine spells out instead of coding. A closed list rather than "any
/// alphabetic word": without one, `Localization/Default`, `Languages/readme`
/// and `Localization/Game.tsv` all read as cultures, and the negative evidence
/// then names three languages that do not exist.
const ASMAA_MAKTUBA: [&str; 31] = [
    "arabic",
    "brazilian",
    "bulgarian",
    "czech",
    "danish",
    "dutch",
    "english",
    "finnish",
    "french",
    "german",
    "greek",
    "hungarian",
    "indonesian",
    "italian",
    "japanese",
    "koreana",
    "korean",
    "latam",
    "norwegian",
    "polish",
    "portuguese",
    "romanian",
    "russian",
    "schinese",
    "spanish",
    "swedish",
    "tchinese",
    "thai",
    "turkish",
    "ukrainian",
    "vietnamese",
];

/// Short words that are never cultures, however code-shaped they look.
///
/// A three-letter ISO 639-2 code and a three-letter interface word are the same
/// shape, and a `Localizations/Default/HUD.tsv` sitting beside `Menu.tsv` and
/// `Game.tsv` is not a culture named `hud`. Naming them is cheaper than the
/// alternative, which is refusing every three-letter code and losing `ara`.
const ASMAA_MUSTABAADA: [&str; 20] = [
    "hud", "ui", "gui", "map", "npc", "pak", "log", "tmp", "bin", "dat", "cfg", "ini", "res",
    "src", "doc", "img", "sfx", "bgm", "vfx", "all",
];

/// Whether a culture code names Arabic.
///
/// Accepts the bare code, any region qualification of it, and the spelled-out
/// name in the forms engines and stores actually use. Separators are
/// normalized, because `ar_SA`, `ar-SA` and `ar-sa` are the same culture
/// written by three different tools.
#[must_use]
pub fn huwa_arabi(ramz: &str) -> bool {
    let munaqqa = wahhid_ramz(ramz);
    if RUMUZ_ARABIYA.contains(&munaqqa.as_str()) {
        return true;
    }
    // `ar-sa`, `ar-eg`, `arabic-sa`. The hyphen is required: without it `arm`
    // and `arabidopsis` would both be Arabic.
    munaqqa.starts_with("ar-") || munaqqa.starts_with("arabic-")
}

/// Whether a token is shaped like a culture code at all.
///
/// Either a BCP 47 code — two or three ASCII letters, optionally followed by
/// script and region subtags — or one of the spelled-out names in
/// [`ASMAA_MAKTUBA`]. Used to decide whether a directory sits in a locale
/// position, which is what makes a bare `ar` directory evidence rather than
/// coincidence.
#[must_use]
pub fn huwa_ramz_thaqafa(ramz: &str) -> bool {
    let munaqqa = wahhid_ramz(ramz);
    if munaqqa.is_empty() || ASMAA_MUSTABAADA.contains(&munaqqa.as_str()) {
        return false;
    }
    if ASMAA_MAKTUBA.contains(&munaqqa.as_str()) {
        return true;
    }
    let mut ajza = munaqqa.split('-');
    let Some(lugha) = ajza.next() else {
        return false;
    };
    if !(2..=3).contains(&lugha.len()) || !lugha.chars().all(|harf| harf.is_ascii_lowercase()) {
        return false;
    }
    ajza.all(|juz| {
        !juz.is_empty() && juz.len() <= 8 && juz.chars().all(|harf| harf.is_ascii_alphanumeric())
    })
}

/// Normalizes a culture code for comparison: lowercased, underscores folded to
/// hyphens, surrounding whitespace and separators removed.
fn wahhid_ramz(ramz: &str) -> String {
    ramz.trim()
        .trim_matches(|harf: char| harf == '.' || harf == '/' || harf == '\\')
        .to_lowercase()
        .replace('_', "-")
}

/// Pulls the culture code out of a Unity Addressables localization bundle name.
///
/// Unity writes `localization-string-tables-arabic(saudiarabia)(ar-sa)_assets_all.bundle`,
/// and the parenthesised tail is the only unambiguous part: the leading name is
/// the locale's English display name and is localized itself in some projects.
/// Returns the last parenthesised group, which is always the code.
#[must_use]
pub fn ramz_min_huzma(ism: &str) -> Option<String> {
    let munaqqa = ism.to_lowercase();
    let bidaya = munaqqa.rfind('(')?;
    let baqi = munaqqa.get(bidaya + 1..)?;
    let nihaya = baqi.find(')')?;
    let ramz = baqi.get(..nihaya)?;
    if ramz.is_empty() || !huwa_ramz_thaqafa(ramz) {
        return None;
    }
    Some(ramz.to_owned())
}

// ---------------------------------------------------------------------------
// المتاجر — what a launcher declares
// ---------------------------------------------------------------------------

/// Whether a launcher's own catalogue declares which languages a game supports.
///
/// One value per adapter in [`crate::matajir`], so that a caller can tell an
/// honest "this launcher records nothing" from an unexamined gap. See
/// [`masdar_lughat`] for the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MasdarLughat {
    /// The launcher declares a language list and this module reads it.
    Maqru,
    /// The launcher's own metadata declares a language list, in a file this
    /// module does not read. The distinction is deliberate: it is a gap with a
    /// known shape, not an absence.
    Muallan,
    /// The launcher records nothing about languages anywhere in its catalogue.
    La,
    /// The entry belongs to a manager that wraps another store; the answer is
    /// whatever the wrapped store's is.
    Munsarif,
}

/// What each launcher adapter can say about a game's languages.
///
/// | launcher | verdict | where it is, or why it is not |
/// | --- | --- | --- |
/// | `steam` | [`MasdarLughat::Maqru`] | `appcache/appinfo.vdf`, `common/supported_languages`, with `supported`, `subtitles` and `full_audio` flags per language |
/// | `gog` | [`MasdarLughat::Muallan`] | `goggame-<id>.info` carries a `languages` map, and Galaxy's `GamePieces` a `languages` piece |
/// | `xbox` | [`MasdarLughat::Muallan`] | `AppxManifest.xml` declares `<Resources><Resource Language="…"/>`, which the adapter parses past today |
/// | `epic` | [`MasdarLughat::La`] | a `.item` manifest carries install tags and no language list |
/// | `ea` | [`MasdarLughat::La`] | the installer XML names a locale for the *title string*, not for the game's content |
/// | `ubisoft` | [`MasdarLughat::La`] | the registry entries record install paths and versions only |
/// | `battlenet` | [`MasdarLughat::La`] | `.product.db` records an installed locale, not a supported set — the user's choice, which says nothing about what exists |
/// | `itch` | [`MasdarLughat::La`] | butler's database has no language column |
/// | `amazon` | [`MasdarLughat::La`] | the `SQLite` catalogue records product identity and install state |
/// | `rockstar` | [`MasdarLughat::La`] | the registry records a language *setting*, again the user's own |
/// | `riot` | [`MasdarLughat::La`] | the product settings record the installed locale, not the available ones |
/// | `bottles` | [`MasdarLughat::La`] | a bottle is a prefix; it knows nothing about the program inside it |
/// | `yadawi`, `mahmul` | [`MasdarLughat::La`] | nobody's catalogue named these games at all |
/// | `heroic`, `legendary`, `lutris`, `playnite` | [`MasdarLughat::Munsarif`] | managers; ask the store behind the entry |
///
/// Battle.net, Rockstar and Riot are the interesting refusals: all three record
/// a language, and it is the language the user installed. Reading it would
/// produce a confident, wrong answer for every player who happens to have
/// picked Arabic in a game that has no Arabic — and for every Arabic-speaking
/// player who plays in English, the mirror of that mistake.
#[must_use]
pub fn masdar_lughat(matjar: &str) -> MasdarLughat {
    match matjar {
        "steam" => MasdarLughat::Maqru,
        "gog" | "xbox" => MasdarLughat::Muallan,
        "heroic" | "legendary" | "lutris" | "playnite" => MasdarLughat::Munsarif,
        _ => MasdarLughat::La,
    }
}

/// One language a launcher declares, with the three facets stores record
/// separately.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LughaMuallana {
    /// The launcher's own token for the language, verbatim: Steam writes
    /// `arabic`, an `AppxManifest` writes `ar-SA`.
    pub ramz: String,
    /// The interface is available in this language.
    pub wajiha: bool,
    /// Subtitles are available in this language.
    pub nusus: bool,
    /// Spoken audio is available in this language.
    pub sawt: bool,
}

/// Every language one launcher declares for one game.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LughatMuallana {
    /// The launcher that declared them.
    pub matjar: &'static str,
    /// The languages, sorted by token.
    pub lughat: Vec<LughaMuallana>,
}

impl LughatMuallana {
    /// The declared language tokens, sorted — the form the cache key uses.
    #[must_use]
    pub fn rumuz(&self) -> Vec<String> {
        self.lughat.iter().map(|lugha| lugha.ramz.clone()).collect()
    }

    /// The Arabic entry, when the listing has one.
    #[must_use]
    pub fn arabiya(&self) -> Option<&LughaMuallana> {
        self.lughat.iter().find(|lugha| huwa_arabi(&lugha.ramz))
    }

    /// Whether any language at all declares subtitles.
    ///
    /// The guard against a degenerate listing. A publisher who fills only the
    /// interface column leaves every language with `subtitles` unset, and
    /// reading that as "no language has subtitles" would turn a fully
    /// translated game into an interface-only one. When this is `false` the
    /// subtitles column carries no information and the verdict says so.
    #[must_use]
    pub fn amud_nusus(&self) -> bool {
        self.lughat.iter().any(|lugha| lugha.nusus)
    }

    /// Whether any language at all declares interface support.
    #[must_use]
    pub fn amud_wajiha(&self) -> bool {
        self.lughat.iter().any(|lugha| lugha.wajiha)
    }
}

/// Reads Steam's declared languages for the applications asked for.
///
/// `matlub` is the sorted list of application identifiers actually installed.
/// `appinfo.vdf` holds a quarter of a million apps and a machine holds a few
/// dozen, so the filter runs on the way through rather than after — the same
/// bargain [`crate::matajir::steam::fahras_appinfo`] makes, and for the same
/// reason.
///
/// Reads the store's `common/supported_languages` subtable, which is where the
/// three facets live. Falls back to `common/languages`, the older flat list, for
/// an app whose entry predates the subtable; that list carries presence only, so
/// every language in it is recorded as interface support with subtitles and
/// audio unknown.
///
/// Every failure degrades. An `appinfo.vdf` that will not parse costs the
/// launcher-metadata probe and nothing else; the other two probes still run.
#[must_use]
pub fn lughat_steam(
    jidhr: &Path,
    matlub: &[u32],
    tanbihat: &mut Vec<TanbihFahs>,
) -> BTreeMap<u32, LughatMuallana> {
    let mut fahras = BTreeMap::new();
    let masar = jidhr.join("appcache").join("appinfo.vdf");
    if !masar.is_file() || matlub.is_empty() {
        return fahras;
    }

    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) => {
            tanbihat.push(TanbihFahs::jadeed(
                "steam",
                masar.display().to_string(),
                format!("cannot read Steam's app metadata cache for languages: {:?}", sabab.kind()),
            ));
            return fahras;
        }
    };

    let natija = vdf::murur_appinfo(&masar, &bayt, &mut |madkhal| {
        if let Ok(madkhal) = madkhal
            && matlub.binary_search(&madkhal.app).is_ok()
            && let Some(lughat) = lughat_min_appinfo(&madkhal.bayanat)
        {
            let _ = fahras.insert(madkhal.app, lughat);
        }
    });

    if let Err(khata) = natija {
        tanbihat.push(TanbihFahs::jadeed("steam", masar.display().to_string(), khata.injilizi));
    }

    fahras
}

/// Projects one app's `appinfo.vdf` tree into its declared language list.
#[must_use]
pub fn lughat_min_appinfo(bayanat: &QeemaVdf) -> Option<LughatMuallana> {
    let mut lughat: Vec<LughaMuallana> = Vec::new();

    for (ramz, madkhal) in
        bayanat.kain_bi_masar(&["appinfo", "common", "supported_languages"]).unwrap_or(&[])
    {
        lughat.push(LughaMuallana {
            ramz: ramz.clone(),
            wajiha: raya(madkhal, "supported"),
            nusus: raya(madkhal, "subtitles"),
            sawt: raya(madkhal, "full_audio"),
        });
    }

    if lughat.is_empty() {
        // The older flat list. Presence only: `english 1`, `arabic 1`. Recorded
        // as interface support because that is the weakest reading of a bare
        // listing, and claiming subtitles from it would be inventing a fact.
        let qadeema = bayanat.kain_bi_masar(&["appinfo", "common", "languages"]).unwrap_or(&[]);
        for (ramz, qeema) in qadeema {
            if qeema.raqm().unwrap_or(1) == 0 {
                continue;
            }
            lughat.push(LughaMuallana {
                ramz: ramz.clone(),
                wajiha: true,
                nusus: false,
                sawt: false,
            });
        }
    }

    if lughat.is_empty() {
        return None;
    }
    lughat.sort();
    lughat.dedup();
    Some(LughatMuallana { matjar: "steam", lughat })
}

/// Reads one `"true"`/`"1"` flag out of a `supported_languages` entry.
fn raya(madkhal: &QeemaVdf, ism: &str) -> bool {
    madkhal.nass_bi_masar(&[ism]).is_some_and(|qeema| {
        matches!(qeema.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
    }) || madkhal.raqm_bi_masar(&[ism]).is_some_and(|qeema| qeema != 0)
}

// ---------------------------------------------------------------------------
// موارد المحرّك — the deep probe, supplied from outside
// ---------------------------------------------------------------------------

/// One culture a game's own localization resources carry.
///
/// Produced by a [`FahisMawarid`], which owns the container readers this crate
/// deliberately does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawridLugha {
    /// The engine, for the evidence sentence: `Unreal`, `Unity`.
    pub muharrik: &'static str,
    /// The localization target or table collection the culture belongs to —
    /// Unreal's `Game`, a Unity string-table collection's name.
    pub hadaf: String,
    /// The culture, exactly as the resource spells it: `ar`, `ar-SA`.
    pub thaqafa: String,
    /// Where it was found, as a path inside the container.
    pub mawqi: String,
    /// How many entries the culture carries.
    pub adad: u64,
    /// How many of those entries actually contain Arabic script.
    ///
    /// The guard against a culture directory that exists and is untranslated.
    /// An `ar` folder full of English strings is a stub the publisher shipped
    /// and never filled, and treating it as official Arabic would deny the user
    /// the only translation they were ever going to get.
    pub arabi: u64,
    /// How many entries the reference culture carries — the target's native
    /// culture, or its largest.
    ///
    /// [`None`] when the probe could not establish one, in which case parity
    /// cannot be computed and the scope stays unknown.
    pub marja: Option<u64>,
    /// Whether this target belongs to the game rather than to the engine or one
    /// of its stock plugins.
    ///
    /// Load-bearing. Unreal ships `OnlineSubsystem` and `OnlineSubsystemSteam`
    /// with Arabic in every project built since 2019, and Little Nightmares —
    /// which has no Arabic at all — carries an `ar` culture in both of them. A
    /// probe that counted engine plugin targets would report official Arabic
    /// for a large fraction of every Unreal game ever shipped.
    pub li_luba: bool,
    /// Every culture the target's manifest declares, when it has one — an
    /// Unreal `.locmeta` names all of them.
    pub thaqafat: Vec<String>,
}

impl MawridLugha {
    /// The share of the reference culture's entries this culture carries, as a
    /// percentage. [`None`] when there is no reference to compare against.
    #[must_use]
    pub const fn takafu(&self) -> Option<u64> {
        match self.marja {
            Some(marja) => self.adad.saturating_mul(100).checked_div(marja),
            None => None,
        }
    }

    /// Whether this culture holds real translated text rather than an
    /// untranslated stub.
    #[must_use]
    pub const fn mutarjama(&self) -> bool {
        self.arabi > 0
    }
}

/// A look inside an engine's own localization containers.
///
/// Implemented by whoever owns the container readers — the Unreal adapter for
/// `.pak` and IoStore, the extraction crate for Unity's serialized files — and
/// handed to [`Fahis`]. See the module header for why this is a trait rather
/// than a call.
///
/// An implementation never fails: a container it cannot open contributes
/// nothing to [`imshi`](FahisMawarid::imshi) and a sentence to
/// [`majhul`](FahisMawarid::majhul), and the two other probes still answer.
pub trait FahisMawarid: Send + Sync + std::fmt::Debug {
    /// The engine this probe reads, for diagnostics: `unreal`, `unity`.
    fn muarrif(&self) -> &'static str;

    /// Every localization culture found under this install root.
    ///
    /// Returns the Arabic ones and enough of the others to establish a
    /// reference count; a probe that returned only Arabic would make parity
    /// impossible to compute.
    fn imshi(&self, jidhr: &Path) -> Vec<MawridLugha>;

    /// What this probe could not see, one sentence each.
    ///
    /// An encrypted `.pak`, a pruned directory index, a Unity build whose
    /// string tables need a type tree the probe does not have. Empty when the
    /// probe saw everything it went looking for.
    fn majhul(&self, _jidhr: &Path) -> Vec<String> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// مسارات الثقافات — locale-shaped paths, read without opening anything
// ---------------------------------------------------------------------------

/// A locale-shaped path found under the install root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MasarThaqafa {
    /// The culture the path names.
    pub thaqafa: String,
    /// The path, relative to the install root.
    pub masar: String,
    /// What kind of layout it is, for the evidence sentence.
    pub shakl: ShaklMasar,
}

/// The layouts a locale-shaped path can belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShaklMasar {
    /// `<Project>/Content/Localization/<Target>/<culture>/` — Unreal, loose.
    LocalizationUnreal,
    /// A Unity Addressables localization bundle whose name carries the locale.
    HuzmatUnity,
    /// A `Localization`, `Languages` or `locale` directory with a culture-named
    /// child.
    MujalladLughat,
    /// A file named for a culture — `ar.json`, `arabic.csv`, `strings_ar.xml`.
    MalafLugha,
}

impl ShaklMasar {
    /// The layout's name, for the evidence sentence.
    const fn wasf(self) -> &'static str {
        match self {
            Self::LocalizationUnreal => "an Unreal localization culture directory",
            Self::HuzmatUnity => "a Unity Addressables localization bundle",
            Self::MujalladLughat => "a locale directory inside a localization folder",
            Self::MalafLugha => "a locale-named localization file",
        }
    }
}

/// Directory names that put their children in a locale position.
const ASMAA_LUGHAT: [&str; 8] = [
    "localization",
    "localisation",
    "localizations",
    "localisations",
    "languages",
    "language",
    "locale",
    "locales",
];

/// Walks the install root for locale-shaped paths.
///
/// Opens nothing. Every judgement here is made from a path and a file name,
/// which is what makes it cheap enough to run on every game in a library and
/// what keeps this crate out of the business of decoding engine containers.
///
/// The shapes it recognises are in [`ShaklMasar`]. The walk is bounded on depth,
/// entry count and result count, and skips the directory names in
/// [`MUJALLADAT_MASTUBAADA`], because an install root is not a trusted
/// directory: it can hold half a million files, a symlink loop, or live on a
/// network mount that answers slowly.
#[must_use]
pub fn imsah_masarat(jidhr: &Path) -> Vec<MasarThaqafa> {
    let mut wujida: Vec<MasarThaqafa> = Vec::new();
    let mut mudakhalat: usize = 0;

    let mashi = WalkDir::new(jidhr)
        .max_depth(HADD_UMQ)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|madkhal| !mustabaad(madkhal));

    for madkhal in mashi.flatten() {
        mudakhalat += 1;
        if mudakhalat > HADD_MUDAKHALAT || wujida.len() >= HADD_MASARAT {
            break;
        }
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr) else {
            continue;
        };
        let Some(ism) = madkhal.file_name().to_str() else {
            continue;
        };
        let Some(shakl) = shakl_masar(nisbi, ism, madkhal.file_type().is_dir()) else {
            continue;
        };
        let thaqafa = match shakl {
            ShaklMasar::HuzmatUnity => ramz_min_huzma(ism),
            ShaklMasar::MalafLugha => thaqafat_malaf(ism),
            _ => Some(ism.to_owned()),
        };
        let Some(thaqafa) = thaqafa else {
            continue;
        };
        if !huwa_arabi(&thaqafa) && !huwa_ramz_thaqafa(&thaqafa) {
            continue;
        }
        wujida.push(MasarThaqafa {
            thaqafa,
            masar: nisbi.to_string_lossy().replace('\\', "/"),
            shakl,
        });
    }

    wujida.sort();
    wujida.dedup();
    wujida
}

/// Whether the walk refuses to descend into this entry.
fn mustabaad(madkhal: &walkdir::DirEntry) -> bool {
    if !madkhal.file_type().is_dir() {
        return false;
    }
    madkhal
        .file_name()
        .to_str()
        .is_some_and(|ism| MUJALLADAT_MASTUBAADA.contains(&ism.to_lowercase().as_str()))
}

/// Classifies one path, or refuses it.
fn shakl_masar(nisbi: &Path, ism: &str, mujallad: bool) -> Option<ShaklMasar> {
    let munkhafid = ism.to_lowercase();

    if !mujallad {
        if munkhafid.starts_with("localization-string-tables-") && munkhafid.ends_with(".bundle") {
            return Some(ShaklMasar::HuzmatUnity);
        }
        // A locale-named file only counts inside a localization directory:
        // `ar.json` at an install root is as likely to be somebody's initials.
        return fi_mujallad_lughat(nisbi).then_some(ShaklMasar::MalafLugha);
    }

    let walid = nisbi.parent()?.file_name()?.to_str()?.to_lowercase();
    if ASMAA_LUGHAT.contains(&walid.as_str()) {
        return Some(ShaklMasar::MujalladLughat);
    }
    // `Content/Localization/<Target>/<culture>` — the culture's parent is the
    // target and the grandparent is the localization directory.
    let jadd = nisbi.parent()?.parent()?.file_name()?.to_str()?.to_lowercase();
    if ASMAA_LUGHAT.contains(&jadd.as_str()) {
        return Some(ShaklMasar::LocalizationUnreal);
    }
    None
}

/// Whether any component of a path is a localization directory.
fn fi_mujallad_lughat(nisbi: &Path) -> bool {
    nisbi.components().any(|juz| {
        juz.as_os_str()
            .to_str()
            .is_some_and(|ism| ASMAA_LUGHAT.contains(&ism.to_lowercase().as_str()))
    })
}

/// Pulls a culture out of a locale-named file: `ar.json`, `arabic.csv`,
/// `strings_ar.xml`, `ar-SA.po`.
fn thaqafat_malaf(ism: &str) -> Option<String> {
    let jidhr = Path::new(ism).file_stem()?.to_str()?;
    if huwa_arabi(jidhr) || huwa_ramz_thaqafa(jidhr) {
        return Some(jidhr.to_owned());
    }
    // `strings_ar`, `ui-ar`: the culture is the last separated segment.
    let akhir = jidhr.rsplit(['_', '-', '.']).next()?;
    (huwa_arabi(akhir) || huwa_ramz_thaqafa(akhir)).then(|| akhir.to_owned())
}

// ---------------------------------------------------------------------------
// الخزينة — the cache, and what invalidates it
// ---------------------------------------------------------------------------

/// The value a cached verdict is keyed on.
///
/// See the module header for the argument. In one line: the build token alone
/// is not enough, because a store listing that gains Arabic moves before the
/// files do.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiftahLugha(String);

impl MiftahLugha {
    /// Derives the key.
    ///
    /// `bina` is the launcher's own build token, where it has one — Steam's
    /// `buildid`. `lughat` is the declared-language list, in the launcher's own
    /// spelling; it is sorted here rather than by the caller so that two
    /// callers cannot derive two different keys for one game.
    #[must_use]
    pub fn jadeed(
        masdar: &MasdarLuba,
        jidhr: &Path,
        bina: Option<&str>,
        lughat: &[String],
    ) -> Self {
        let mut hadhi = blake3::Hasher::new();
        hadhi.update(&ISDAR_FAHS.to_le_bytes());
        hadhi.update(b"\0");
        hadhi.update(masdar.muarrif().as_bytes());
        hadhi.update(b"\0");
        hadhi.update(miftah_jidhr(jidhr).as_bytes());
        hadhi.update(b"\0");
        hadhi.update(bina.unwrap_or("").as_bytes());
        hadhi.update(b"\0");
        let mut murattaba: Vec<&str> = lughat.iter().map(String::as_str).collect();
        murattaba.sort_unstable();
        murattaba.dedup();
        for lugha in murattaba {
            hadhi.update(lugha.as_bytes());
            hadhi.update(b"\x1f");
        }
        Self(hadhi.finalize().to_hex().to_string())
    }

    /// The key as text, which is what a store column holds.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// Wraps a key that was already computed and stored.
    #[must_use]
    pub const fn min_nass(nass: String) -> Self {
        Self(nass)
    }
}

/// The comparison form of an install root: case-folded where the filesystem is.
fn miftah_jidhr(jidhr: &Path) -> String {
    let nass = jidhr.to_string_lossy();
    let maqsus = nass.trim_end_matches(['/', '\\']);
    if taarib_usus::manassa::NizamTashghil::hali().hassas_lil_ahruf() {
        maqsus.to_owned()
    } else {
        maqsus.to_lowercase()
    }
}

/// A cached verdict and the key it was computed under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SijillLughaRasmiya {
    /// The game.
    pub luba: LubaId,
    /// What the verdict was computed from. A record whose key does not match a
    /// freshly derived one is stale and is recomputed.
    pub miftah: MiftahLugha,
    /// The verdict.
    pub hukm: HukmLughaRasmiya,
}

/// Somewhere to keep verdicts between runs.
///
/// A port, not an implementation. This crate persists nothing — see the crate
/// root — so a caller that wants verdicts to survive a restart implements this
/// against `taarib-makhzan` and hands it to [`Fahis`]. A caller that does not
/// gets [`KhazinaDhakira`] and pays the walk once per process.
pub trait KhazinatLugha: Send + Sync + std::fmt::Debug {
    /// The record for a game, when there is one. A stale record may be returned:
    /// [`Fahis`] compares the key and discards it.
    fn jalb(&self, luba: LubaId) -> Option<SijillLughaRasmiya>;

    /// Stores a record, replacing any earlier one for the same game.
    fn sajjil(&self, sijill: &SijillLughaRasmiya);
}

/// The in-memory store, used when a caller supplies no other.
///
/// Lives for as long as the caller holds it, which for a single scan is exactly
/// long enough: the walk runs once per game per process rather than once per
/// game per query.
#[derive(Debug, Default)]
pub struct KhazinaDhakira {
    sijillat: Mutex<BTreeMap<LubaId, SijillLughaRasmiya>>,
}

impl KhazinaDhakira {
    /// An empty store.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// How many verdicts are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.sijillat.lock().len()
    }
}

impl KhazinatLugha for KhazinaDhakira {
    fn jalb(&self, luba: LubaId) -> Option<SijillLughaRasmiya> {
        self.sijillat.lock().get(&luba).cloned()
    }

    fn sajjil(&self, sijill: &SijillLughaRasmiya) {
        let _ = self.sijillat.lock().insert(sijill.luba, sijill.clone());
    }
}

// ---------------------------------------------------------------------------
// الفحص — the detector
// ---------------------------------------------------------------------------

/// One half of the verdict while it is still being decided, with the weight of
/// the observation that currently holds it.
///
/// The fold is by weight, not by polarity. Three probes can disagree, and the
/// one that should win is the one that saw the most: a store listing that names
/// Arabic for a game whose own files compile thirteen cultures and no `ar` is a
/// listing describing a console SKU, a future patch, or somebody's mistake, and
/// letting it overrule the files would put "Full Arabic" on a card underneath
/// an evidence list that says the opposite. Equal weights fall back to
/// [`TughtiyaLugha::wa`], where a positive beats a negative.
#[derive(Debug, Clone, Copy)]
struct Mizan {
    hala: TughtiyaLugha,
    wazn: u8,
}

impl Mizan {
    const fn jadeed() -> Self {
        Self { hala: TughtiyaLugha::Majhula, wazn: 0 }
    }

    const fn min_hala(hala: TughtiyaLugha, wazn: u8) -> Self {
        Self { hala, wazn }
    }

    /// Folds in one weighted observation. Silence carries no weight and is
    /// dropped: a probe with nothing to say must not displace one that spoke.
    const fn dammij(&mut self, akhar: Self) {
        if matches!(akhar.hala, TughtiyaLugha::Majhula) {
            return;
        }
        if matches!(self.hala, TughtiyaLugha::Majhula) || akhar.wazn > self.wazn {
            self.hala = akhar.hala;
            self.wazn = akhar.wazn;
        } else if akhar.wazn == self.wazn {
            self.hala = self.hala.wa(akhar.hala);
        }
    }
}

/// Everything the detector is asked about one game.
#[derive(Debug, Clone)]
pub struct TalabLugha<'a> {
    /// Taarib's identity for the game, which is what the cache keys on.
    pub luba: LubaId,
    /// The launcher identity, unwrapped past any manager.
    pub masdar: &'a MasdarLuba,
    /// The installation root.
    pub jidhr: &'a Path,
    /// The launcher's build token, where it has one.
    pub bina: Option<&'a str>,
    /// What the launcher declares, where it declares anything.
    pub lughat: Option<&'a LughatMuallana>,
}

/// The detector.
///
/// Holds no state of its own: the deep probes and the cache are borrowed, so a
/// caller builds one, runs a library through it, and drops it.
#[derive(Debug, Default, Clone, Copy)]
pub struct Fahis<'a> {
    massah: &'a [&'a dyn FahisMawarid],
    khazina: Option<&'a dyn KhazinatLugha>,
}

impl<'a> Fahis<'a> {
    /// A detector with no deep probe and no cache: the two probes discovery can
    /// run itself, computed fresh every time.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { massah: &[], khazina: None }
    }

    /// Adds the deep engine probes.
    #[must_use]
    pub const fn bi_massah(self, massah: &'a [&'a dyn FahisMawarid]) -> Self {
        Self { massah, khazina: self.khazina }
    }

    /// Adds a cache.
    #[must_use]
    pub const fn bi_khazina(self, khazina: &'a dyn KhazinatLugha) -> Self {
        Self { massah: self.massah, khazina: Some(khazina) }
    }

    /// Decides whether a game already speaks Arabic, reading the cache first.
    ///
    /// A cached verdict is used only when its key matches a freshly derived
    /// one, so a store update that changes the declared languages, a game
    /// update that changes the build, a move to another drive, and a new
    /// detector version each cause a recomputation.
    #[must_use]
    pub fn ifhas(&self, talab: &TalabLugha<'_>) -> HukmLughaRasmiya {
        let rumuz = talab.lughat.map(LughatMuallana::rumuz).unwrap_or_default();
        let miftah = MiftahLugha::jadeed(talab.masdar, talab.jidhr, talab.bina, &rumuz);

        if let Some(khazina) = self.khazina
            && let Some(sijill) = khazina.jalb(talab.luba)
            && sijill.miftah == miftah
        {
            tracing::debug!(luba = %talab.luba, "official-language verdict served from cache");
            return sijill.hukm;
        }

        let hukm = self.ihsib(talab, rumuz);
        if let Some(khazina) = self.khazina {
            khazina.sajjil(&SijillLughaRasmiya {
                luba: talab.luba,
                miftah,
                hukm: hukm.clone(),
            });
        }
        hukm
    }

    /// Computes a verdict, ignoring the cache entirely.
    #[must_use]
    pub fn ihsib_bila_khazina(&self, talab: &TalabLugha<'_>) -> HukmLughaRasmiya {
        let rumuz = talab.lughat.map(LughatMuallana::rumuz).unwrap_or_default();
        self.ihsib(talab, rumuz)
    }

    /// The three probes, folded into one verdict.
    fn ihsib(&self, talab: &TalabLugha<'_>, rumuz: Vec<String>) -> HukmLughaRasmiya {
        let mut dalail: Vec<DaleelLugha> = Vec::new();
        let mut majhul: Vec<String> = Vec::new();
        let mut wajiha = Mizan::jadeed();
        let mut nusus = Mizan::jadeed();

        // --- 1. what the launcher declares ---------------------------------
        match talab.lughat {
            Some(lughat) => {
                let (w, n) = min_matjar(lughat, &mut dalail, &mut majhul);
                wajiha.dammij(w);
                nusus.dammij(n);
            }
            None => majhul.push(sabab_bila_matjar(talab.masdar)),
        }

        // --- 2. what the engine ships --------------------------------------
        let mut mawarid: Vec<MawridLugha> = Vec::new();
        for fahis in self.massah {
            mawarid.extend(fahis.imshi(talab.jidhr));
            majhul.extend(fahis.majhul(talab.jidhr));
        }
        if self.massah.is_empty() {
            majhul.push(
                "no engine-resource probe was supplied, so any Arabic compiled into this game's \
                 own localization containers was not looked for"
                    .to_owned(),
            );
        } else {
            let (w, n) = min_mawarid(&mawarid, &mut dalail);
            wajiha.dammij(w);
            nusus.dammij(n);
        }

        // --- 3. locale-shaped paths ----------------------------------------
        let masarat = imsah_masarat(talab.jidhr);
        wajiha.dammij(min_masarat(&masarat, &mut dalail));

        dalail.sort_by_key(|daleel| Reverse(daleel.wazn));
        let thiqa = thiqa(&dalail, &majhul);

        HukmLughaRasmiya {
            wajiha: wajiha.hala,
            nusus: nusus.hala,
            thiqa,
            dalail,
            majhul,
            lughat_muallana: rattib(rumuz),
            isdar_fahs: ISDAR_FAHS,
            waqt: jiff::Timestamp::now().to_string(),
        }
    }
}

/// Sorts and deduplicates a declared-language list.
fn rattib(mut rumuz: Vec<String>) -> Vec<String> {
    rumuz.sort();
    rumuz.dedup();
    rumuz
}

/// Why a game has no launcher-declared language list.
fn sabab_bila_matjar(masdar: &MasdarLuba) -> String {
    let matjar = masdar.aila().slug();
    match masdar_lughat(matjar) {
        MasdarLughat::Maqru => format!(
            "{matjar} declares supported languages but none were supplied for this game, so the \
             store's own answer is missing"
        ),
        MasdarLughat::Muallan => format!(
            "{matjar} declares supported languages in its own metadata, which this detector does \
             not read yet"
        ),
        MasdarLughat::Munsarif => format!(
            "{matjar} manages games belonging to another store and declares no languages of its \
             own"
        ),
        MasdarLughat::La => {
            format!("{matjar} records nothing about which languages a game supports")
        }
    }
}

/// Reads the launcher's declaration into evidence.
fn min_matjar(
    lughat: &LughatMuallana,
    dalail: &mut Vec<DaleelLugha>,
    majhul: &mut Vec<String>,
) -> (Mizan, Mizan) {
    let matjar = lughat.matjar;
    let Some(arabiya) = lughat.arabiya() else {
        dalail.push(DaleelLugha {
            naw: NawDaleelLugha::LughatMatjar,
            wasf: format!(
                "{matjar} lists {} supported languages for this game and Arabic is not among \
                 them: {}",
                lughat.lughat.len(),
                lughat.rumuz().join(", ")
            ),
            mawqi: Some(format!("{matjar}:supported_languages")),
            wazn: WAZN_MATJAR_NAFI,
        });
        let nafi = Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MATJAR_NAFI);
        return (nafi, nafi);
    };

    let mut facets: Vec<&str> = Vec::new();
    if arabiya.wajiha {
        facets.push("interface");
    }
    if arabiya.nusus {
        facets.push("subtitles");
    }
    if arabiya.sawt {
        facets.push("full audio");
    }
    let masrud =
        if facets.is_empty() { "with no facet flags set".to_owned() } else { facets.join(", ") };

    dalail.push(DaleelLugha {
        naw: NawDaleelLugha::LughatMatjar,
        wasf: format!(
            "{matjar} lists Arabic as a supported language for this game ({masrud}), as `{}`",
            arabiya.ramz
        ),
        mawqi: Some(format!("{matjar}:supported_languages/{}", arabiya.ramz)),
        wazn: WAZN_MATJAR,
    });

    // A column no language in the listing fills is a column the publisher never
    // filled, and an empty cell in it says nothing about this game.
    let wajiha = if arabiya.wajiha {
        Mizan::min_hala(TughtiyaLugha::Muakkada, WAZN_MATJAR)
    } else if lughat.amud_wajiha() {
        Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MATJAR_NAFI)
    } else {
        majhul.push(format!(
            "{matjar}'s listing sets no interface flag on any language, so its silence about \
             Arabic menus carries no information"
        ));
        Mizan::jadeed()
    };

    let nusus = if arabiya.nusus {
        Mizan::min_hala(TughtiyaLugha::Muakkada, WAZN_MATJAR)
    } else if lughat.amud_nusus() {
        Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MATJAR_NAFI)
    } else {
        majhul.push(format!(
            "{matjar}'s listing sets no subtitle flag on any language, so its silence about \
             Arabic subtitles carries no information"
        ));
        Mizan::jadeed()
    };

    (wajiha, nusus)
}

/// Reads the engine's own resources into evidence.
///
/// Only targets belonging to the game count. Unreal ships `OnlineSubsystem` and
/// `OnlineSubsystemSteam` with a full culture set including `ar` in every
/// project built against a modern engine, and Little Nightmares — a game with
/// no Arabic whatsoever — carries both.
fn min_mawarid(mawarid: &[MawridLugha], dalail: &mut Vec<DaleelLugha>) -> (Mizan, Mizan) {
    let arabiya: Vec<&MawridLugha> = mawarid
        .iter()
        .filter(|mawrid| mawrid.li_luba && huwa_arabi(&mawrid.thaqafa))
        .collect();

    if arabiya.is_empty() {
        let ahdaf: BTreeSet<&str> = mawarid
            .iter()
            .filter(|mawrid| mawrid.li_luba)
            .map(|mawrid| mawrid.hadaf.as_str())
            .collect();
        if ahdaf.is_empty() {
            return (Mizan::jadeed(), Mizan::jadeed());
        }
        let thaqafat: BTreeSet<&str> = mawarid
            .iter()
            .filter(|mawrid| mawrid.li_luba)
            .map(|mawrid| mawrid.thaqafa.as_str())
            .collect();
        let muharrik = mawarid.first().map_or("the engine", |mawrid| mawrid.muharrik);
        dalail.push(DaleelLugha {
            naw: NawDaleelLugha::MawridMuharrik,
            wasf: format!(
                "{muharrik} compiles {} culture(s) into this game's own localization target(s) \
                 {:?} and none of them is Arabic: {}",
                thaqafat.len(),
                ahdaf.iter().copied().collect::<Vec<_>>(),
                thaqafat.iter().copied().collect::<Vec<_>>().join(", ")
            ),
            mawqi: mawarid.iter().find(|mawrid| mawrid.li_luba).map(|mawrid| mawrid.mawqi.clone()),
            wazn: WAZN_MAWRID,
        });
        let nafi = Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MAWRID);
        return (nafi, nafi);
    }

    let mut wajiha = Mizan::jadeed();
    let mut nusus = Mizan::jadeed();

    for mawrid in arabiya {
        if !mawrid.mutarjama() {
            dalail.push(DaleelLugha {
                naw: NawDaleelLugha::MawridMuharrik,
                wasf: format!(
                    "{} compiles culture `{}` into target `{}` with {} entries, and not one of \
                     them contains Arabic script — the culture is a stub the publisher never \
                     filled",
                    mawrid.muharrik, mawrid.thaqafa, mawrid.hadaf, mawrid.adad
                ),
                mawqi: Some(mawrid.mawqi.clone()),
                wazn: WAZN_MAWRID_FARIGH,
            });
            let farigh = Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MAWRID_FARIGH);
            wajiha.dammij(farigh);
            nusus.dammij(farigh);
            continue;
        }

        let takafu = mawrid.takafu();
        let masrud = match (takafu, mawrid.marja) {
            (Some(nisba), Some(marja)) => {
                format!(", {nisba}% of the {marja} its reference culture carries")
            }
            _ => String::new(),
        };
        let muallana = if mawrid.thaqafat.is_empty() {
            String::new()
        } else {
            format!("; the target's manifest declares {} cultures", mawrid.thaqafat.len())
        };

        dalail.push(DaleelLugha {
            naw: NawDaleelLugha::MawridMuharrik,
            wasf: format!(
                "{} compiles culture `{}` into this game's own localization target `{}` with {} \
                 entries{masrud}, {} of them in Arabic script{muallana}",
                mawrid.muharrik,
                mawrid.thaqafa,
                mawrid.hadaf,
                mawrid.adad,
                mawrid.arabi
            ),
            mawqi: Some(mawrid.mawqi.clone()),
            wazn: WAZN_MAWRID,
        });

        // Parity with the reference culture is what separates "the game is
        // translated" from "the menus are". A target the publisher filled to
        // within a fifth of its reference has no separate untranslated half
        // left for subtitles to hide in.
        wajiha.dammij(Mizan::min_hala(TughtiyaLugha::Muakkada, WAZN_MAWRID));
        if takafu.is_some_and(|nisba| nisba >= HADD_TAKAFU) {
            nusus.dammij(Mizan::min_hala(TughtiyaLugha::Muakkada, WAZN_MAWRID));
        }
    }

    (wajiha, nusus)
}

/// Reads locale-shaped paths into evidence.
///
/// Contributes to the interface half only. A locale directory says a language
/// is present; nothing about a path says whether the strings inside it are
/// menus or dialogue, and claiming otherwise from a directory name would be
/// inventing a fact.
fn min_masarat(masarat: &[MasarThaqafa], dalail: &mut Vec<DaleelLugha>) -> Mizan {
    let arabiya: Vec<&MasarThaqafa> =
        masarat.iter().filter(|masar| huwa_arabi(&masar.thaqafa)).collect();

    if let Some(awwal) = arabiya.first() {
        dalail.push(DaleelLugha {
            naw: NawDaleelLugha::MasarThaqafa,
            wasf: format!(
                "the install carries {} — `{}` — naming culture `{}`{}",
                awwal.shakl.wasf(),
                awwal.masar,
                awwal.thaqafa,
                if arabiya.len() > 1 {
                    format!(", and {} more Arabic locale path(s)", arabiya.len() - 1)
                } else {
                    String::new()
                }
            ),
            mawqi: Some(awwal.masar.clone()),
            wazn: WAZN_MASAR,
        });
        return Mizan::min_hala(TughtiyaLugha::Muakkada, WAZN_MASAR);
    }

    if masarat.is_empty() {
        return Mizan::jadeed();
    }

    let thaqafat: BTreeSet<&str> = masarat.iter().map(|masar| masar.thaqafa.as_str()).collect();
    dalail.push(DaleelLugha {
        naw: NawDaleelLugha::MasarThaqafa,
        wasf: format!(
            "the install carries {} locale-named path(s) and none of them is Arabic: {}",
            masarat.len(),
            thaqafat.iter().copied().collect::<Vec<_>>().join(", ")
        ),
        mawqi: masarat.first().map(|masar| masar.masar.clone()),
        wazn: WAZN_MASAR,
    });
    Mizan::min_hala(TughtiyaLugha::Ghaiba, WAZN_MASAR)
}

/// How much the verdict is worth.
///
/// The weight of the heaviest observation, less a fixed amount for each
/// question that went unanswered, and zero when nothing was observed at all. A
/// scale a maintainer can reproduce by hand from the evidence list is worth
/// more than a scale that is more precisely wrong.
fn thiqa(dalail: &[DaleelLugha], majhul: &[String]) -> u8 {
    let Some(aqwa) = dalail.iter().map(|daleel| daleel.wazn).max() else {
        return 0;
    };
    let khasm = u8::try_from(majhul.len())
        .unwrap_or(u8::MAX)
        .saturating_mul(KHASM_MAJHUL)
        .min(AQSA_KHASM);
    aqwa.saturating_sub(khasm)
}

#[cfg(test)]
mod ikhtibarat {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use taarib_mustalahat::luba::HalatLughaRasmiya;

    use super::*;

    /// A deep probe that answers from a fixed list, for testing the fold
    /// without a game on disk. Counts its calls so that a cache hit can be
    /// asserted rather than inferred from a timestamp.
    #[derive(Debug)]
    struct MassahThabit {
        mawarid: Vec<MawridLugha>,
        majhul: Vec<String>,
        marrat: AtomicUsize,
    }

    impl MassahThabit {
        fn jadeed(mawarid: Vec<MawridLugha>) -> Self {
            Self { mawarid, majhul: Vec::new(), marrat: AtomicUsize::new(0) }
        }

        fn marrat(&self) -> usize {
            self.marrat.load(Ordering::Relaxed)
        }
    }

    impl FahisMawarid for MassahThabit {
        fn muarrif(&self) -> &'static str {
            "thabit"
        }
        fn imshi(&self, _jidhr: &Path) -> Vec<MawridLugha> {
            let _ = self.marrat.fetch_add(1, Ordering::Relaxed);
            self.mawarid.clone()
        }
        fn majhul(&self, _jidhr: &Path) -> Vec<String> {
            self.majhul.clone()
        }
    }

    fn mawrid(thaqafa: &str, adad: u64, arabi: u64, marja: u64, li_luba: bool) -> MawridLugha {
        MawridLugha {
            muharrik: "Unreal",
            hadaf: "Game".to_owned(),
            thaqafa: thaqafa.to_owned(),
            mawqi: format!("Game/Localization/Game/{thaqafa}/Game.locres"),
            adad,
            arabi,
            marja: Some(marja),
            li_luba,
            thaqafat: vec!["ar".to_owned(), "en".to_owned()],
        }
    }

    fn lughat(bunud: &[(&str, bool, bool, bool)]) -> LughatMuallana {
        LughatMuallana {
            matjar: "steam",
            lughat: bunud
                .iter()
                .map(|(ramz, wajiha, nusus, sawt)| LughaMuallana {
                    ramz: (*ramz).to_owned(),
                    wajiha: *wajiha,
                    nusus: *nusus,
                    sawt: *sawt,
                })
                .collect(),
        }
    }

    #[test]
    fn ramz_arabi_yuqbal_bi_kul_sighah() {
        for ramz in ["ar", "AR", "ar-SA", "ar_SA", "Arabic", "arabic", "ara", "ar-EG"] {
            assert!(huwa_arabi(ramz), "{ramz} should read as Arabic");
        }
    }

    #[test]
    fn ramz_ghayr_arabi_yurfad() {
        for ramz in ["art", "arm", "en", "en-US", "armenian", "brazilian", "schinese", ""] {
            assert!(!huwa_arabi(ramz), "{ramz} should not read as Arabic");
        }
    }

    #[test]
    fn shakl_al_ramz_yumayyiz_al_thaqafat_min_asma_al_wajiha() {
        for ramz in ["ar", "en-US", "zh-Hans", "pt-BR", "en-US-POSIX", "ara", "arabic", "koreana"] {
            assert!(huwa_ramz_thaqafa(ramz), "{ramz} is a culture position");
        }
        for ramz in ["HUD", "Menu", "Game", "Default", "readme", "shared", "", "x"] {
            assert!(!huwa_ramz_thaqafa(ramz), "{ramz} is not a culture position");
        }
    }

    #[test]
    fn ramz_huzmat_unity_yustakhraj() {
        let arabiya = "localization-string-tables-arabic(saudiarabia)(ar-sa)_assets_all.bundle";
        assert_eq!(ramz_min_huzma(arabiya).as_deref(), Some("ar-sa"));
        assert_eq!(
            ramz_min_huzma("localization-string-tables-danish(denmark)(da-dk)_assets_all.bundle")
                .as_deref(),
            Some("da-dk")
        );
        assert_eq!(ramz_min_huzma("localization-assets-shared_assets_all.bundle"), None);
    }

    #[test]
    fn mawrid_luba_bi_takafu_kamil_yueti_hukman_kamilan() {
        let massah = MassahThabit::jadeed(vec![mawrid("ar", 345, 186, 349, true)]);
        let marja: [&dyn FahisMawarid; 1] = [&massah];
        let fahis = Fahis::jadeed().bi_massah(&marja);
        let masdar = MasdarLuba::Steam(2_149_010);
        let muallana = lughat(&[("arabic", true, false, false), ("english", true, false, false)]);
        let hukm = fahis.ihsib_bila_khazina(&TalabLugha {
            luba: LubaId::min_masdar(&masdar, "Little Nightmares Enhanced Edition"),
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: Some("1"),
            lughat: Some(&muallana),
        });
        assert_eq!(hukm.hala(), HalatLughaRasmiya::Kamila);
        assert!(hukm.yatakallam_arabi());
        assert!(hukm.hasim());
    }

    #[test]
    fn mawrid_al_muharrik_wahdah_la_yahsib() {
        // An engine plugin's stock `ar` culture must never count as the game's.
        let massah = MassahThabit::jadeed(vec![
            mawrid("ar", 12, 12, 12, false),
            mawrid("en", 300, 0, 300, true),
        ]);
        let marja: [&dyn FahisMawarid; 1] = [&massah];
        let fahis = Fahis::jadeed().bi_massah(&marja);
        let masdar = MasdarLuba::Steam(424_840);
        let hukm = fahis.ihsib_bila_khazina(&TalabLugha {
            luba: LubaId::min_masdar(&masdar, "Little Nightmares"),
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: Some("1"),
            lughat: Some(&lughat(&[("english", true, true, false)])),
        });
        assert_eq!(hukm.hala(), HalatLughaRasmiya::Ghaib);
        assert!(!hukm.yatakallam_arabi());
    }

    #[test]
    fn thaqafa_farigha_tuqra_ka_ghayr_mutarjama() {
        let massah = MassahThabit::jadeed(vec![mawrid("ar", 40, 0, 300, true)]);
        let marja: [&dyn FahisMawarid; 1] = [&massah];
        let fahis = Fahis::jadeed().bi_massah(&marja);
        let masdar = MasdarLuba::Steam(1);
        let hukm = fahis.ihsib_bila_khazina(&TalabLugha {
            luba: LubaId::min_masdar(&masdar, "stub"),
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: None,
            lughat: None,
        });
        assert_eq!(hukm.hala(), HalatLughaRasmiya::Ghaib);
    }

    #[test]
    fn amud_nusus_farigh_la_yunfi() {
        // Every language bare `supported`: the subtitles column is unfilled, so
        // Arabic's empty cell must not become "no Arabic subtitles".
        let mut dalail = Vec::new();
        let mut majhul = Vec::new();
        let (wajiha, nusus) = min_matjar(
            &lughat(&[("arabic", true, false, false), ("english", true, false, false)]),
            &mut dalail,
            &mut majhul,
        );
        assert_eq!(wajiha.hala, TughtiyaLugha::Muakkada);
        assert_eq!(nusus.hala, TughtiyaLugha::Majhula);
        assert_eq!(majhul.len(), 1);
    }

    #[test]
    fn amud_nusus_mamlu_yunfi() {
        let mut dalail = Vec::new();
        let mut majhul = Vec::new();
        let (wajiha, nusus) = min_matjar(
            &lughat(&[("arabic", true, false, false), ("english", true, true, false)]),
            &mut dalail,
            &mut majhul,
        );
        assert_eq!(wajiha.hala, TughtiyaLugha::Muakkada);
        assert_eq!(nusus.hala, TughtiyaLugha::Ghaiba);
        assert!(majhul.is_empty());
    }

    #[test]
    fn dalil_al_muharrik_yaghlib_qaimat_al_matjar() {
        // The store says Arabic; the game's own files compile a culture set
        // without it. The files win, in both directions.
        let massah = MassahThabit::jadeed(vec![
            mawrid("en", 300, 0, 300, true),
            mawrid("fr", 298, 0, 300, true),
        ]);
        let marja: [&dyn FahisMawarid; 1] = [&massah];
        let fahis = Fahis::jadeed().bi_massah(&marja);
        let masdar = MasdarLuba::Steam(424_840);
        let hukm = fahis.ihsib_bila_khazina(&TalabLugha {
            luba: LubaId::min_masdar(&masdar, "Little Nightmares"),
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: Some("1"),
            lughat: Some(&lughat(&[("arabic", true, true, false), ("english", true, true, false)])),
        });
        assert_eq!(hukm.hala(), HalatLughaRasmiya::Ghaib);
        assert_eq!(hukm.dalail.len(), 2, "both observations stay in the evidence list");
    }

    #[test]
    fn miftah_yatabaddal_bi_taghyeer_al_lughat() {
        let masdar = MasdarLuba::Steam(2_149_010);
        let jidhr = Path::new("/games/lnee");
        let qabl = MiftahLugha::jadeed(
            &masdar,
            jidhr,
            Some("23363152"),
            &["english".to_owned(), "french".to_owned()],
        );
        let baad = MiftahLugha::jadeed(
            &masdar,
            jidhr,
            Some("23363152"),
            &["arabic".to_owned(), "english".to_owned(), "french".to_owned()],
        );
        assert_ne!(qabl, baad, "adding a declared language must invalidate the key");
    }

    #[test]
    fn miftah_yatabaddal_bi_taghyeer_al_bina() {
        let masdar = MasdarLuba::Steam(2_149_010);
        let jidhr = Path::new("/games/lnee");
        let lughat = ["english".to_owned()];
        assert_ne!(
            MiftahLugha::jadeed(&masdar, jidhr, Some("1"), &lughat),
            MiftahLugha::jadeed(&masdar, jidhr, Some("2"), &lughat)
        );
    }

    #[test]
    fn miftah_thabit_ala_tarteeb_al_lughat() {
        let masdar = MasdarLuba::Steam(1);
        let jidhr = Path::new("/g");
        assert_eq!(
            MiftahLugha::jadeed(&masdar, jidhr, None, &["b".to_owned(), "a".to_owned()]),
            MiftahLugha::jadeed(&masdar, jidhr, None, &["a".to_owned(), "b".to_owned()])
        );
    }

    #[test]
    fn al_khazina_tukhdim_thumma_tabtul() {
        let massah = MassahThabit::jadeed(vec![mawrid("ar", 345, 186, 349, true)]);
        let marja: [&dyn FahisMawarid; 1] = [&massah];
        let khazina = KhazinaDhakira::jadeeda();
        let fahis = Fahis::jadeed().bi_massah(&marja).bi_khazina(&khazina);
        let masdar = MasdarLuba::Steam(2_149_010);
        let luba = LubaId::min_masdar(&masdar, "lnee");
        let qadeema = lughat(&[("arabic", true, false, false)]);
        let talab = TalabLugha {
            luba,
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: Some("1"),
            lughat: Some(&qadeema),
        };

        let awwal = fahis.ifhas(&talab);
        assert_eq!(khazina.adad(), 1);
        assert_eq!(massah.marrat(), 1);

        let thani = fahis.ifhas(&talab);
        assert_eq!(awwal, thani, "the second run must be the cached verdict");
        assert_eq!(massah.marrat(), 1, "the second run must not walk the containers again");

        // A store update that adds a language changes the key, so the verdict is
        // recomputed rather than served.
        let jadeeda = lughat(&[("arabic", true, false, false), ("polish", true, false, false)]);
        let talab_jadeed = TalabLugha { lughat: Some(&jadeeda), ..talab.clone() };
        let thalith = fahis.ifhas(&talab_jadeed);
        assert_eq!(massah.marrat(), 2, "a changed declared-language list must invalidate");
        assert_ne!(awwal.lughat_muallana, thalith.lughat_muallana);

        // And a game update that changes the build does the same.
        let talab_bina = TalabLugha { bina: Some("2"), ..talab.clone() };
        let _ = fahis.ifhas(&talab_bina);
        assert_eq!(massah.marrat(), 3, "a changed build must invalidate");
    }

    #[test]
    fn bila_massah_yuqal_ma_lam_yura() {
        let fahis = Fahis::jadeed();
        let masdar = MasdarLuba::Steam(1);
        let hukm = fahis.ihsib_bila_khazina(&TalabLugha {
            luba: LubaId::min_masdar(&masdar, "x"),
            masdar: &masdar,
            jidhr: Path::new("/nowhere"),
            bina: None,
            lughat: None,
        });
        assert!(!hukm.hasim());
        assert_eq!(hukm.thiqa, 0);
        assert!(hukm.majhul.len() >= 2);
    }

    #[test]
    fn jadwal_al_matajir_yughatti_kul_muarrif() {
        assert_eq!(masdar_lughat("steam"), MasdarLughat::Maqru);
        assert_eq!(masdar_lughat("gog"), MasdarLughat::Muallan);
        assert_eq!(masdar_lughat("xbox"), MasdarLughat::Muallan);
        assert_eq!(masdar_lughat("heroic"), MasdarLughat::Munsarif);
        assert_eq!(masdar_lughat("battlenet"), MasdarLughat::La);
        for matjar in crate::matajir::kul() {
            let _ = masdar_lughat(matjar.muarrif());
        }
    }
}
