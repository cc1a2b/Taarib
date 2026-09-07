//! بلاي‌نايت — Playnite, and the honest limit of what this build can read.
//!
//! Playnite is not a store. It is a library manager that installs nothing and
//! owns nothing: it sits on top of Steam, GOG, Epic, EA, Ubisoft, Battle.net,
//! Xbox, itch, Amazon and a dozen emulators, imports their catalogues through
//! plugins, and gives the user one grid over all of them. A Playnite user's
//! games are, almost without exception, games some other launcher installed.
//!
//! ## The central design note
//!
//! **Playnite's library is a `LiteDB` database, and this build does not parse
//! `LiteDB`.** `library/games.db`, `library/platforms.db`, `library/sources.db`
//! and `library/emulators.db` are `LiteDB` v5 files — a page-structured,
//! BSON-valued, undocumented-outside-its-own-C#-implementation format with its
//! own index pages, its own collection catalogue, and its own
//! optionally-encrypted header. There is no maintained Rust reader for it.
//! Writing one would mean reverse-engineering a storage engine in order to
//! recover a game list, and then re-verifying that reverse engineering against
//! every `LiteDB` bump a Playnite release brings.
//!
//! Three things follow, and all three are deliberate:
//!
//! 1. **This adapter does not pretend to read the game list.** There is no
//!    speculative byte-scraping of `games.db`, no "search the file for UTF-16
//!    runs that look like titles". A scraper like that produces a plausible
//!    list that is wrong in ways nobody can see, which is worse than an empty
//!    list with a sentence explaining it.
//! 2. **The games are not lost.** Playnite aggregates launchers Taarib already
//!    scans directly, so a Playnite user's Steam games arrive through
//!    [`crate::matajir::steam`], their GOG games through
//!    [`crate::matajir::gog`], and so on. The one category Playnite holds that
//!    nothing else does is its **manually added** and **emulated** entries, and
//!    those are what this adapter tries hardest to recover.
//! 3. **[`Matjar::ifhas`] always says so.** Every scan of a Playnite
//!    installation attaches a [`TanbihFahs`] that states plainly that the
//!    library is a `LiteDB` database this build does not parse, what *was* read
//!    instead, and where the games are coming from. A user who sees Playnite
//!    listed in Diagnostics with zero games gets the reason on the same screen.
//!
//! ## What is actually read
//!
//! | path | what it gives |
//! | --- | --- |
//! | `config.json` | the data root's own settings, and `DatabasePath` — which is how a user relocates their library off the system drive |
//! | `library/files/<game guid>/*` | every piece of artwork Playnite has downloaded, keyed by the game's own GUID; directly what Phase 20B wants |
//! | `ExtensionsData/<plugin guid>/*.json` | whatever a plugin chose to keep as JSON, which is where a recoverable game list can appear |
//! | `taarib-playnite.json` at the data root | an export the user placed there deliberately — **Taarib's own convention, not Playnite's**, documented below |
//!
//! ## Installed mode and portable mode
//!
//! Both are detected, and missing the second would cost those users everything.
//! Playnite ships as an installer *and* as a portable archive, and the portable
//! build is what people put on the drive that holds their games:
//!
//! - **Installed** — the program lives in `%LOCALAPPDATA%\Playnite` and its data
//!   in `%APPDATA%\Playnite`.
//! - **Portable** — one directory holds both, so `library`, `ExtensionsData`
//!   and `config.json` sit beside `Playnite.DesktopApp.exe`.
//!
//! The two are told apart **structurally**, by what is in the directory, never
//! by a marker file. Playnite's own portable test has changed spelling across
//! releases, and a data root is a data root whether or not the executable that
//! made it is still beside it — a folder copied off an old machine has no
//! program in it at all and still holds a perfectly readable library.
//!
//! Portable roots cannot be enumerated: they can be anywhere. So a small set of
//! conventional locations is probed, and everything else arrives through
//! [`MatjarPlaynite::bi_judhur`], which the caller fills from the user's
//! settings. That is the same seam [`crate::matajir::yadawi`] uses for its
//! stored entries, and it exists here for the same reason — the adapter must
//! not read the settings store itself.
//!
//! ## Linux, through Wine
//!
//! Playnite is a .NET desktop application with no Linux build, so a Linux user
//! who runs it runs it inside a Wine prefix — and its data then lives at
//! `<prefix>/drive_c/users/<user>/AppData/Roaming/Playnite`, which no amount of
//! looking under `~/.config` will find. Every prefix [`crate::beea`] knows about
//! is probed, and paths recovered from inside one are translated back to host
//! paths through that prefix's own drive map rather than string-joined, because
//! `C:\Games\X` is only `drive_c/Games/X` when `dosdevices/c:` says it is.
//!
//! ## Identity
//!
//! `MasdarLuba::Playnite(Box::new(MasdarPlaynite { muarrif, asl }))`, where
//! `muarrif` is Playnite's own GUID for the entry and `asl` is the store behind
//! it when one can be determined **without guessing**. `asl` resolves the shard
//! family and the deterministic identity, exactly as Heroic's wrapper does, so a
//! patch published against `steam:427520` is offered to a user whose entry
//! reached Taarib through Playnite. `asl` is `None` for the manually added and
//! emulated entries that have no store identity at all, and the GUID then stands
//! alone — which is what [`MasdarPlaynite`] exists for.
//!
//! The store behind an entry is resolved from its **source name plus its
//! plugin-side game id**, never from the plugin's GUID. Plugin GUIDs are
//! constants living in somebody else's C# source; getting one wrong would mint a
//! confident, wrong identity and offer a Steam patch to a GOG install. The
//! source name and `GameId` are the pair Playnite itself round-trips, and when
//! either is missing or unrecognised the answer is `None` rather than a guess.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;
use taarib_mustalahat::luba::{MasdarLuba, MasdarPlaynite};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier. It names the *adapter*; an entry with a store behind
/// it shards under that store, and only a store-less entry shards under
/// `playnite`.
const MUARRIF: &str = "playnite";

/// Windows natively, Linux through Wine, and macOS only for a root the user
/// nominates — there is no Playnite build for macOS and therefore no path worth
/// probing there, but a data folder copied onto one still reads.
const MANASSAT: [NizamTashghil; 3] = [
    NizamTashghil::Windows,
    NizamTashghil::Linux,
    NizamTashghil::Mac,
];

/// The directory holding the `LiteDB` files and the media tree.
const MUJALLAD_MAKTABA: &str = "library";

/// The per-game media directory inside the library, one subdirectory per game
/// GUID.
const MUJALLAD_MALAFFAT: &str = "files";

/// Per-plugin storage, one subdirectory per extension GUID.
const MUJALLAD_IMTIDADAT: &str = "ExtensionsData";

/// Playnite's own settings file at the data root.
const MALAF_IDADAT: &str = "config.json";

/// The `LiteDB` files this build deliberately does not open.
///
/// Listed so the warning can name the exact files rather than gesturing at "the
/// library", and so [`huwa_jidhr_bayanat`] can recognise a data root by the
/// presence of any one of them.
const ASMAA_QAWAID: [&str; 4] = ["games.db", "platforms.db", "sources.db", "emulators.db"];

/// The token Playnite substitutes for its own program directory inside
/// `DatabasePath`.
///
/// A portable installation writes `{PlayniteDir}\library` there, and a reader
/// that joined that string literally would look for a directory whose name
/// contains a brace.
const RAMZ_MUJALLAD: &str = "{PlayniteDir}";

/// The export file this adapter reads if the user places one.
///
/// **This is Taarib's convention, not Playnite's.** Playnite has no built-in
/// JSON export at a fixed path, and inventing one and calling it Playnite's
/// would be a lie in a doc comment. What this is instead is a documented
/// escape hatch: a user whose manually added Playnite entries matter to them can
/// drop a JSON array of game records at `<data root>/taarib-playnite.json` and
/// Taarib will read it. The shape accepted is Playnite's own — `Id`, `Name`,
/// `InstallDirectory`, `Source`, `GameId`, `IsInstalled` — which is what every
/// Playnite export extension already emits.
const MALAF_TASDIR: &str = "taarib-playnite.json";

/// How many per-game media directories are examined.
///
/// Five thousand. A large Playnite library is a few thousand games and each one
/// costs a directory listing, so this is a ceiling on a scan's worst case rather
/// than a limit anybody real will meet. Truncation is reported.
const AQSA_MUJALLADAT_WASAIT: usize = 5_000;

/// How many plugin directories under `ExtensionsData` are opened.
const AQSA_MUJALLADAT_IMTIDAD: usize = 256;

/// How many JSON files are read inside one plugin directory.
const AQSA_MALAFFAT_IMTIDAD: usize = 64;

/// The largest JSON file this adapter will read into memory.
///
/// Thirty-two megabytes. A plugin's cache can be arbitrarily large and a scan
/// that ran at startup must not allocate a gigabyte because somebody's metadata
/// plugin kept every cover's base64 beside its records.
const AQSA_HAJM_JSON: u64 = 32 * 1024 * 1024;

/// The largest image file treated as artwork.
///
/// Sixty-four megabytes, which no cover art reaches and a mis-detected archive
/// with a `.png` extension would.
const AQSA_HAJM_SURA: u64 = 64 * 1024 * 1024;

/// Image extensions Playnite stores artwork as.
const IMTIDADAT_SUWAR: [&str; 6] = ["png", "jpg", "jpeg", "webp", "bmp", "ico"];

/// Name fragments that mark a media file as the vertical cover.
const AJZAA_GHILAF: [&str; 5] = ["cover", "box", "grid", "portrait", "poster"];

/// Name fragments that mark a media file as the wide banner.
const AJZAA_BATL: [&str; 5] = ["background", "backdrop", "hero", "banner", "fanart"];

/// Name fragments that mark a media file as the small logo.
const AJZAA_SHIAR: [&str; 3] = ["icon", "logo", "clearlogo"];

/// The largest edge a near-square image may have and still be taken for an icon.
const AQSA_HAFFAT_SHIAR: u32 = 512;

// ---------------------------------------------------------------------------
// the adapter
// ---------------------------------------------------------------------------

/// Playnite.
#[derive(Debug, Clone, Default)]
pub struct MatjarPlaynite {
    /// Data roots the caller supplied from the user's settings.
    ///
    /// [`taarib_usus::idadat::IdadatManassat`] has no `playnite` field, so this
    /// is the seam: the caller reads the setting and hands the paths over,
    /// exactly as [`crate::matajir::yadawi::MatjarYadawi::bi_sijill`] is handed
    /// its records. Portable installations are the reason it exists — they live
    /// wherever the user put the folder, and no probe can enumerate that.
    judhur: Vec<PathBuf>,
}

impl MatjarPlaynite {
    /// Builds the adapter with no nominated roots, so it probes only the
    /// conventional locations.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { judhur: Vec::new() }
    }

    /// Builds the adapter over data roots the user nominated.
    ///
    /// Each one is taken as a Playnite **data** root — the directory holding
    /// `library` and `config.json` — and a path that is not one is skipped with
    /// a warning rather than silently ignored, because the user typed it.
    #[must_use]
    pub const fn bi_judhur(judhur: Vec<PathBuf>) -> Self {
        Self { judhur }
    }

    /// The roots the caller nominated.
    #[must_use]
    pub fn judhur_mudafa(&self) -> &[PathBuf] {
        &self.judhur
    }

    /// Every data root worth looking at, in the order they are tried.
    ///
    /// Nominated roots come first: a user who told Taarib where their Playnite
    /// is has said something the probe cannot know. After them come the
    /// installed-mode location, then the conventional portable ones, then — on a
    /// system that is not Windows — every Wine prefix [`crate::beea`] found.
    fn judhur_muhtamala(&self, siyaq: &SiyaqFahs) -> Vec<JidhrPlaynite> {
        let mut judhur: Vec<JidhrPlaynite> = Vec::new();

        // The user's settings override, ahead of everything including the roots
        // this adapter was constructed with. A portable Playnite lives wherever
        // its owner put the folder, so a setting that named it and then lost to
        // an automatic probe would be a setting that does nothing on exactly
        // the installation it exists for.
        if let Some(masar) = siyaq.manassat.playnite.as_ref() {
            judhur.push(JidhrPlaynite::mudaf(masar.clone()));
        }

        for masar in &self.judhur {
            judhur.push(JidhrPlaynite::mudaf(masar.clone()));
        }

        match siyaq.nizam {
            NizamTashghil::Windows => {
                judhur.extend(
                    siyaq
                        .bayanat_mutajawwila
                        .as_ref()
                        .map(|bayanat| JidhrPlaynite::muthabbat(bayanat.join("Playnite"))),
                );
                for masar in judhur_mahmula_windows(siyaq) {
                    judhur.push(JidhrPlaynite::mahmul(masar));
                }
            },
            NizamTashghil::Linux | NizamTashghil::Mac => {
                for masar in judhur_mahmula_yuniks(&siyaq.manzil) {
                    judhur.push(JidhrPlaynite::mahmul(masar));
                }
                judhur.extend(judhur_dakhil_al_beeat(siyaq));
            },
        }

        judhur
    }
}

/// One candidate data root, and what is known about how it was reached.
///
/// The prefix matters downstream and nowhere else would remember it: a root
/// found inside a Wine prefix holds Windows paths, and a Windows path is only
/// resolvable through the drive map of the prefix it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct JidhrPlaynite {
    /// The data root.
    masar: PathBuf,
    /// Whether it looks like a portable installation rather than the
    /// installed-mode data directory.
    mahmul: bool,
    /// Whether the user nominated it, which changes a miss from "not installed"
    /// into something worth warning about.
    mudaf: bool,
    /// The Wine prefix it was found inside, when it was.
    beea: Option<PathBuf>,
}

impl JidhrPlaynite {
    /// A root the user nominated.
    const fn mudaf(masar: PathBuf) -> Self {
        Self {
            masar,
            mahmul: false,
            mudaf: true,
            beea: None,
        }
    }

    /// The installed-mode data directory.
    const fn muthabbat(masar: PathBuf) -> Self {
        Self {
            masar,
            mahmul: false,
            mudaf: false,
            beea: None,
        }
    }

    /// A conventional portable location.
    const fn mahmul(masar: PathBuf) -> Self {
        Self {
            masar,
            mahmul: true,
            mudaf: false,
            beea: None,
        }
    }
}

impl Matjar for MatjarPlaynite {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "بلاي‌نايت"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Playnite"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        self.judhur_muhtamala(siyaq)
            .into_iter()
            .find(|murashah| huwa_jidhr_bayanat(&murashah.masar))
            .map(|murashah| murashah.masar)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when **every** root the user
    /// nominated is missing and no root was found automatically. That is the one
    /// case where silence would hide the user's own mistake: they typed a path,
    /// nothing is there, and reporting "Playnite is not installed" would send
    /// them looking in the wrong place.
    ///
    /// Nothing else fails. A `config.json` that will not parse costs only the
    /// relocated-library setting; a media directory that cannot be listed costs
    /// that game's artwork; a plugin's JSON that is not a game list costs
    /// nothing at all, because most of them are not.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();

        let murashahat = self.judhur_muhtamala(siyaq);
        let Some(jidhr) = murashahat
            .iter()
            .find(|murashah| huwa_jidhr_bayanat(&murashah.masar))
            .cloned()
        else {
            if let Some(mafqud) = murashahat.iter().find(|murashah| murashah.mudaf) {
                return Err(KhataKashf::JidhrMuhaddadMafqud {
                    matjar: MUARRIF,
                    masar: mafqud.masar.clone(),
                }
                .into());
            }
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, Some(jidhr.masar.clone()));

        let maktaba = iqra_maktaba(&jidhr.masar);
        natija.tanbihat.extend(maktaba.tanbihat.iter().cloned());

        let sijillat = ijma_sijillat(&jidhr.masar, &mut natija.tanbihat);
        let mut ghayr_muthabbata: usize = 0;
        for sijill in &sijillat {
            if !sijill.muthabbata {
                ghayr_muthabbata = ghayr_muthabbata.saturating_add(1);
                continue;
            }
            match luba_min_sijill(sijill, &jidhr, &maktaba, siyaq.nizam) {
                Ok(luba) => natija.alaab.push(luba),
                Err(sabab) => natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    format!("{}: {}", sijill.mustanad, sijill.ism),
                    sabab,
                )),
            }
        }

        // Catalogue-level by the header's own admission: the game list is a
        // database this build does not read, so a Playnite entry missing from
        // this result is never evidence that it is gone.
        natija.tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            jidhr.masar.join(MUJALLAD_MAKTABA).display().to_string(),
            tanbih_qaida(&maktaba, &jidhr, natija.alaab.len(), ghayr_muthabbata),
        ));

        natija
            .alaab
            .sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    /// The library directory, which is where every change Playnite makes lands.
    ///
    /// `library` rather than the data root: `config.json` is rewritten whenever
    /// a window moves, and a watch on the data root would fire on that. The
    /// media tree sits inside `library`, so an artwork download is caught too.
    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let Some(jidhr) = self.mawqi(siyaq) else {
            return Vec::new();
        };
        let idadat = iqra_idadat(&jidhr);
        let mut judhur = vec![jidhr_maktaba(&jidhr, &idadat)];
        let imtidadat = jidhr.join(MUJALLAD_IMTIDADAT);
        if imtidadat.is_dir() {
            judhur.push(imtidadat);
        }
        judhur.retain(|masar| masar.is_dir());
        judhur.sort();
        judhur.dedup();
        judhur
    }
}

/// Whether a directory is a Playnite data root.
///
/// Structural, and satisfied by any one of three things: a `library` directory
/// holding one of the `LiteDB` files, an `ExtensionsData` directory, or a
/// `config.json` sitting beside a `library` directory. Three tests rather than
/// one because all three states occur — a fresh installation has `config.json`
/// and an empty `library`, a heavily extended one has `ExtensionsData` before
/// anything else, and a folder copied off another machine has the databases and
/// nothing else.
#[must_use]
pub fn huwa_jidhr_bayanat(jidhr: &Path) -> bool {
    if !jidhr.is_dir() {
        return false;
    }
    let maktaba = jidhr.join(MUJALLAD_MAKTABA);
    if ASMAA_QAWAID.iter().any(|ism| maktaba.join(ism).is_file()) {
        return true;
    }
    if jidhr.join(MUJALLAD_IMTIDADAT).is_dir() {
        return true;
    }
    maktaba.is_dir() && jidhr.join(MALAF_IDADAT).is_file()
}

/// Conventional portable locations on Windows.
///
/// The installed build keeps its program in `%LOCALAPPDATA%\Playnite` and its
/// data elsewhere; a portable build keeps both together, and people put it in
/// the same place or beside their games. None of these is guaranteed and all of
/// them cost one `is_dir` call, which is why probing them is worth more than the
/// four lines it takes.
///
/// The local application data candidate is dropped, rather than assembled under
/// the home directory, on a context that has no such folder: the remaining
/// four are all home-relative or absolute and stand on their own.
fn judhur_mahmula_windows(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
    let manzil = &siyaq.manzil;
    let mut judhur = Vec::with_capacity(5);
    judhur.extend(
        siyaq
            .bayanat_mahalliya
            .as_ref()
            .map(|mahalli| mahalli.join("Playnite")),
    );
    judhur.extend([
        manzil.join("Playnite"),
        manzil.join("Games").join("Playnite"),
        PathBuf::from(r"C:\Playnite"),
        PathBuf::from(r"D:\Playnite"),
    ]);
    judhur
}

/// Conventional portable locations on a Unix system.
///
/// A portable folder copied onto a Linux or macOS machine — off a USB drive, out
/// of a backup — reads perfectly well even though nothing there can run it, and
/// the artwork it carries is the same artwork Phase 20B wants.
fn judhur_mahmula_yuniks(manzil: &Path) -> Vec<PathBuf> {
    vec![
        manzil.join("Playnite"),
        manzil.join("Games").join("Playnite"),
        manzil.join(".local").join("share").join("Playnite"),
    ]
}

/// Playnite data roots inside every Wine prefix on the machine.
///
/// A prefix's `C:` drive is resolved through its own `dosdevices` map rather
/// than assumed to be `<prefix>/drive_c`, and each user profile inside it is
/// checked, because a prefix made by Lutris and a prefix made by Bottles do not
/// agree on the profile name.
fn judhur_dakhil_al_beeat(siyaq: &SiyaqFahs) -> Vec<JidhrPlaynite> {
    let mut judhur: Vec<JidhrPlaynite> = Vec::new();
    for beea in crate::beea::iktashif_beeat(&siyaq.manzil) {
        let Ok(maalumat) = crate::beea::hal_beea(&beea) else {
            continue;
        };
        let mustakhdimun = maalumat.drive_c().join("users");
        let Ok(qaima) = std::fs::read_dir(&mustakhdimun) else {
            continue;
        };
        let mut asmaa: Vec<PathBuf> = qaima
            .flatten()
            .map(|madkhal| madkhal.path())
            .filter(|masar| masar.is_dir())
            .collect();
        asmaa.sort();
        for mustakhdim in asmaa {
            let bayanat = mustakhdim.join("AppData").join("Roaming").join("Playnite");
            if bayanat.is_dir() {
                judhur.push(JidhrPlaynite {
                    masar: bayanat,
                    mahmul: false,
                    mudaf: false,
                    beea: Some(beea.clone()),
                });
            }
        }
    }
    judhur
}

// ---------------------------------------------------------------------------
// config.json
// ---------------------------------------------------------------------------

/// The parts of Playnite's settings that change where things are.
///
/// Deliberately narrow. Playnite's `config.json` holds a hundred keys about
/// window sizes and grid columns, and a reader that deserialized the lot would
/// break on the first release that renamed one of them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdadatPlaynite {
    /// `DatabasePath`, when the user relocated their library.
    ///
    /// May carry the `{PlayniteDir}` token, which is resolved against the data
    /// root before this field is filled.
    pub masar_qaida: Option<PathBuf>,
    /// `Version`, when the file records one, so Diagnostics can say which
    /// Playnite wrote the library that could not be read.
    pub isdar: Option<String>,
    /// Whether the settings file existed and parsed.
    pub maqrua: bool,
}

/// Reads a data root's `config.json`.
///
/// Never fails: an absent or malformed settings file means only that the library
/// is wherever the default says it is, which is right far more often than it is
/// wrong.
fn iqra_idadat(jidhr: &Path) -> IdadatPlaynite {
    let Some(qeema) = iqra_json(&jidhr.join(MALAF_IDADAT)) else {
        return IdadatPlaynite::default();
    };
    let masar_qaida = nass_haql(&qeema, "DatabasePath")
        .or_else(|| nass_haql(&qeema, "databasePath"))
        .map(|nass| hall_ramz_mujallad(&nass, jidhr));
    IdadatPlaynite {
        masar_qaida,
        isdar: nass_haql(&qeema, "Version").or_else(|| nass_haql(&qeema, "version")),
        maqrua: true,
    }
}

/// Resolves `{PlayniteDir}` and normalizes separators.
///
/// Playnite writes the token with a backslash after it even on a path that will
/// be read on a system whose separator is not one, because it is a Windows
/// application. Both separators are accepted and the result is rebuilt component
/// by component, so a value copied between machines still resolves.
fn hall_ramz_mujallad(nass: &str, jidhr: &Path) -> PathBuf {
    let munaqqa = nass.trim();
    let Some(baqi) = munaqqa.strip_prefix(RAMZ_MUJALLAD) else {
        return PathBuf::from(munaqqa.replace('\\', std::path::MAIN_SEPARATOR_STR));
    };
    let mut mabni = jidhr.to_path_buf();
    for juz in baqi.split(['\\', '/']) {
        if juz.is_empty() || juz == "." {
            continue;
        }
        mabni.push(juz);
    }
    mabni
}

/// Where the library directory actually is.
///
/// `DatabasePath` when it names a directory that exists, `<data root>/library`
/// otherwise. A configured path that is *not* there falls back rather than being
/// honoured, because the fallback finds a library and the configured path finds
/// nothing — and the mismatch surfaces in the warning either way.
fn jidhr_maktaba(jidhr: &Path, idadat: &IdadatPlaynite) -> PathBuf {
    idadat
        .masar_qaida
        .as_ref()
        .filter(|masar| masar.is_dir())
        .cloned()
        .unwrap_or_else(|| jidhr.join(MUJALLAD_MAKTABA))
}

// ---------------------------------------------------------------------------
// what one data root yielded
// ---------------------------------------------------------------------------

/// Everything readable at one Playnite data root.
///
/// Public because the game list is the part that is missing, not the part that
/// is useful: Phase 20B wants the artwork map on its own, keyed by the GUID a
/// game carries in `MasdarPlaynite::muarrif`, and a caller that already has a
/// game from some other adapter can look its cover up here.
#[derive(Debug, Clone, Default)]
pub struct MaktabatPlaynite {
    /// The data root this came from.
    pub jidhr: PathBuf,
    /// The library directory, after `DatabasePath` was applied.
    pub jidhr_maktaba: PathBuf,
    /// The settings that were readable.
    pub idadat: IdadatPlaynite,
    /// Which of the `LiteDB` files are present, by file name.
    ///
    /// Named rather than counted so the warning can say `games.db` instead of
    /// "a database", which is the difference between a sentence a user can act
    /// on and one they cannot.
    pub qawaid: Vec<String>,
    /// Artwork, keyed by the game's Playnite GUID in lowercase.
    pub suwar: BTreeMap<String, MasadirSuwar>,
    /// How many media directories were examined.
    pub adad_wasait: usize,
    /// Whether the media walk stopped at [`AQSA_MUJALLADAT_WASAIT`].
    pub wasait_mabtura: bool,
    /// Warnings raised while reading, ready to attach to a scan result.
    pub tanbihat: Vec<TanbihFahs>,
}

/// Reads everything at a data root that does not require a `LiteDB` parser.
///
/// This is the whole of what this adapter can honestly do with a Playnite
/// installation: the settings, which of the databases exist, and the media tree.
/// It is separated from [`Matjar::ifhas`] so that a caller who wants only the
/// artwork — Phase 20B — does not have to run a scan to get it.
#[must_use]
pub fn iqra_maktaba(jidhr: &Path) -> MaktabatPlaynite {
    let idadat = iqra_idadat(jidhr);
    let jidhr_maktaba = jidhr_maktaba(jidhr, &idadat);

    let mut tanbihat: Vec<TanbihFahs> = Vec::new();
    if let Some(muhaddad) = idadat.masar_qaida.as_ref()
        && !muhaddad.is_dir()
    {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            muhaddad.display().to_string(),
            "Playnite's config.json points DatabasePath at this directory and it is not there, \
             so the default library folder beside the settings was read instead. The library may \
             be on a drive that is not mounted.",
        ));
    }

    let qawaid: Vec<String> = ASMAA_QAWAID
        .iter()
        .filter(|ism| jidhr_maktaba.join(ism).is_file())
        .map(|ism| (*ism).to_owned())
        .collect();

    let (suwar, adad_wasait, wasait_mabtura) = suwar_maktaba(&jidhr_maktaba);
    if wasait_mabtura {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr_maktaba.join(MUJALLAD_MALAFFAT).display().to_string(),
            format!(
                "this media folder holds more than {AQSA_MUJALLADAT_WASAIT} per-game \
                 directories, so artwork was collected from the first {AQSA_MUJALLADAT_WASAIT} \
                 only"
            ),
        ));
    }

    MaktabatPlaynite {
        jidhr: jidhr.to_path_buf(),
        jidhr_maktaba,
        idadat,
        qawaid,
        suwar,
        adad_wasait,
        wasait_mabtura,
        tanbihat,
    }
}

/// The sentence attached to every Playnite scan.
///
/// It is written out here rather than assembled at the call site because it is
/// the most important thing this adapter produces, and it has to stay true as
/// the surrounding code changes: it names the format, names the files, says what
/// was read instead, and says where the games are actually coming from.
fn tanbih_qaida(
    maktaba: &MaktabatPlaynite,
    jidhr: &JidhrPlaynite,
    adad_alaab: usize,
    ghayr_muthabbata: usize,
) -> String {
    let qawaid = if maktaba.qawaid.is_empty() {
        "no library database file was found".to_owned()
    } else {
        format!("{} is a LiteDB database", maktaba.qawaid.join(", "))
    };

    let namat = if jidhr.mahmul {
        "portable"
    } else {
        "installed"
    };
    let beea = jidhr.beea.as_ref().map_or_else(String::new, |beea| {
        format!(
            " It was found inside the Wine prefix at {}.",
            beea.display()
        )
    });

    let wasait = if maktaba.suwar.is_empty() {
        "No artwork was cached there yet.".to_owned()
    } else {
        format!(
            "Artwork was read for {} entries out of {} media folders, and is available to the \
             library grid.",
            maktaba.suwar.len(),
            maktaba.adad_wasait
        )
    };

    let alaab = if adad_alaab == 0 {
        "No game list could be recovered: nothing under ExtensionsData and no \
         taarib-playnite.json export at the data root held records this reader understands, so \
         this adapter contributed no games."
            .to_owned()
    } else {
        format!(
            "{adad_alaab} entries were recovered from readable JSON beside the database \
             ({ghayr_muthabbata} more were listed as not installed and skipped)."
        )
    };

    format!(
        "Playnite's library ({namat} mode) cannot be enumerated by this build: {qawaid}, and \
         Taarib does not parse LiteDB. Its settings and its artwork were read instead. {wasait} \
         {alaab} This costs almost nothing: Playnite does not install games, it aggregates the \
         launchers that do — Steam, GOG, Epic, EA, Ubisoft, Battle.net, Xbox, itch and Amazon — \
         and Taarib scans every one of those directly, so the same games arrive through their own \
         adapters with their own store identities. What is genuinely only in Playnite is its \
         manually added and emulated entries; export those to taarib-playnite.json at the data \
         root to bring them in.{beea}"
    )
}

// ---------------------------------------------------------------------------
// artwork
// ---------------------------------------------------------------------------

/// Every game's artwork under `library/files`, keyed by lowercase GUID.
///
/// Playnite names a media file after whatever it was downloaded as, so the
/// classification is by name fragment first — `cover`, `background`, `icon` and
/// their synonyms are what its own metadata plugins write — and by image shape
/// second, read from the file's header rather than by decoding it. A tall image
/// is a cover, a wide one is a banner, and a small near-square one is an icon.
/// Nothing else is guessed: a directory whose files match neither test
/// contributes its first image as the cover, because the grid needs one and
/// showing the wrong artwork is recoverable while showing none is what the user
/// notices.
#[must_use]
pub fn suwar_alaab(jidhr_maktaba: &Path) -> BTreeMap<String, MasadirSuwar> {
    suwar_maktaba(jidhr_maktaba).0
}

/// The artwork map, plus how much of the tree was actually visited.
fn suwar_maktaba(jidhr_maktaba: &Path) -> (BTreeMap<String, MasadirSuwar>, usize, bool) {
    let mut suwar: BTreeMap<String, MasadirSuwar> = BTreeMap::new();
    let malaffat = jidhr_maktaba.join(MUJALLAD_MALAFFAT);
    let Ok(qaima) = std::fs::read_dir(&malaffat) else {
        return (suwar, 0, false);
    };

    let mut mujalladat: Vec<PathBuf> = qaima
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| masar.is_dir())
        .collect();
    mujalladat.sort();

    let mabtur = mujalladat.len() > AQSA_MUJALLADAT_WASAIT;
    let mut adad: usize = 0;
    for mujallad in mujalladat {
        if adad >= AQSA_MUJALLADAT_WASAIT {
            break;
        }
        adad = adad.saturating_add(1);
        let Some(muarrif) = mujallad
            .file_name()
            .map(|ism| ism.to_string_lossy().to_lowercase())
        else {
            continue;
        };
        let masadir = suwar_mujallad(&mujallad);
        if !masadir.khali() {
            let _ = suwar.insert(muarrif, masadir);
        }
    }

    (suwar, adad, mabtur)
}

/// The three artwork pieces inside one game's media directory.
fn suwar_mujallad(mujallad: &Path) -> MasadirSuwar {
    let Ok(qaima) = std::fs::read_dir(mujallad) else {
        return MasadirSuwar::default();
    };

    let mut malaffat: Vec<PathBuf> = qaima
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| huwa_malaf_sura(masar))
        .collect();
    malaffat.sort();

    let mut masadir = MasadirSuwar::default();
    let mut baqi: Vec<PathBuf> = Vec::new();

    // Pass one: the name says what it is. This is the only classification that
    // is evidence rather than inference, so it wins outright.
    for masar in malaffat {
        let sanf = sanf_bil_ism(&masar);
        match sanf {
            Some(SanfSura::Ghilaf) if masadir.ghilaf.is_none() => {
                masadir.ghilaf = Some(MasdarSura::Malaf(masar));
            },
            Some(SanfSura::Batl) if masadir.batl.is_none() => {
                masadir.batl = Some(MasdarSura::Malaf(masar));
            },
            Some(SanfSura::Shiar) if masadir.shiar.is_none() => {
                masadir.shiar = Some(MasdarSura::Malaf(masar));
            },
            _ => baqi.push(masar),
        }
    }

    // Pass two: the shape says what it is, from the header alone.
    let mut ghayr_musannafa: Vec<PathBuf> = Vec::new();
    for masar in baqi {
        let sanf = sanf_bil_shakl(&masar);
        match sanf {
            Some(SanfSura::Ghilaf) if masadir.ghilaf.is_none() => {
                masadir.ghilaf = Some(MasdarSura::Malaf(masar));
            },
            Some(SanfSura::Batl) if masadir.batl.is_none() => {
                masadir.batl = Some(MasdarSura::Malaf(masar));
            },
            Some(SanfSura::Shiar) if masadir.shiar.is_none() => {
                masadir.shiar = Some(MasdarSura::Malaf(masar));
            },
            _ => ghayr_musannafa.push(masar),
        }
    }

    // Pass three: whatever is left fills the cover, and only the cover.
    if masadir.ghilaf.is_none()
        && let Some(awwal) = ghayr_musannafa.into_iter().next()
    {
        masadir.ghilaf = Some(MasdarSura::Malaf(awwal));
    }

    masadir
}

/// Which of the three artwork slots a file belongs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SanfSura {
    /// The vertical cover.
    Ghilaf,
    /// The wide banner.
    Batl,
    /// The small logo.
    Shiar,
}

/// Whether a path is an image small enough to be artwork.
fn huwa_malaf_sura(masar: &Path) -> bool {
    let Some(imtidad) = masar
        .extension()
        .map(|imtidad| imtidad.to_string_lossy().to_lowercase())
    else {
        return false;
    };
    if !IMTIDADAT_SUWAR.contains(&imtidad.as_str()) {
        return false;
    }
    std::fs::metadata(masar)
        .is_ok_and(|bayanat| bayanat.is_file() && bayanat.len() <= AQSA_HAJM_SURA)
}

/// The slot a file's name declares, when it declares one.
fn sanf_bil_ism(masar: &Path) -> Option<SanfSura> {
    let ism = masar.file_stem()?.to_string_lossy().to_lowercase();
    // Logo before cover: `clearlogo` contains neither, but a file called
    // `logo_cover` is a logo and matching covers first would take it.
    if AJZAA_SHIAR.iter().any(|juz| ism.contains(juz)) {
        return Some(SanfSura::Shiar);
    }
    if AJZAA_BATL.iter().any(|juz| ism.contains(juz)) {
        return Some(SanfSura::Batl);
    }
    if AJZAA_GHILAF.iter().any(|juz| ism.contains(juz)) {
        return Some(SanfSura::Ghilaf);
    }
    None
}

/// The slot a file's proportions imply.
///
/// Only the header is read — `ImageReader::into_dimensions` stops before any
/// pixel data — so this costs one open and a few dozen bytes per file rather
/// than a decode. The comparisons are integer cross-multiplications rather than
/// a ratio, because a ratio would be a float and floats do not compare.
fn sanf_bil_shakl(masar: &Path) -> Option<SanfSura> {
    let qari = image::ImageReader::open(masar)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let (ard, irtifa) = qari.into_dimensions().ok()?;
    if ard == 0 || irtifa == 0 {
        return None;
    }
    let ard = u64::from(ard);
    let irtifa = u64::from(irtifa);

    // Taller than 5:4 is a cover; Playnite's grid art is close to 2:3.
    if irtifa.saturating_mul(4) > ard.saturating_mul(5) {
        return Some(SanfSura::Ghilaf);
    }
    // Wider than 14:9 is a banner; a screenshot-derived background is 16:9.
    if ard.saturating_mul(9) > irtifa.saturating_mul(14) {
        return Some(SanfSura::Batl);
    }
    // What is left is near-square, and is an icon only if it is also small.
    let hadd = u64::from(AQSA_HAFFAT_SHIAR);
    (ard <= hadd && irtifa <= hadd).then_some(SanfSura::Shiar)
}

// ---------------------------------------------------------------------------
// the entries that can be recovered
// ---------------------------------------------------------------------------

/// One game record recovered from JSON beside the database.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SijillPlaynite {
    /// Playnite's own GUID for the entry, lowercased.
    muarrif: String,
    /// The title.
    ism: String,
    /// The install directory, as the record spells it — which inside a Wine
    /// prefix is a Windows path.
    jidhr: Option<String>,
    /// The executable the record names, relative to the install directory.
    tanfidhi: Option<String>,
    /// The library source's display name: `Steam`, `GOG`, `Epic`.
    ism_masdar: Option<String>,
    /// The store's own identifier for the game, as the plugin recorded it.
    muarrif_matjar: Option<String>,
    /// Whether Playnite considers it installed.
    muthabbata: bool,
    /// When it was last played, when the record says.
    akhir_laab: Option<String>,
    /// Size on disk, when the record says. Zero otherwise.
    hajm: u64,
    /// A cover path relative to the media tree, when the record names one.
    ghilaf_nisbi: Option<String>,
    /// A background path relative to the media tree.
    batl_nisbi: Option<String>,
    /// An icon path relative to the media tree.
    shiar_nisbi: Option<String>,
    /// Which file it came out of, for the warning that names it.
    mustanad: String,
}

/// Every recoverable record at a data root, from both sources.
///
/// The export is read first so that a record the user placed deliberately wins
/// over one a plugin happened to cache, and duplicates are dropped by GUID.
fn ijma_sijillat(jidhr: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<SijillPlaynite> {
    let mut sijillat: Vec<SijillPlaynite> = Vec::new();
    let mut maruf: Vec<String> = Vec::new();

    for sijill in sijillat_min_tasdir(jidhr, tanbihat) {
        if !maruf.contains(&sijill.muarrif) {
            maruf.push(sijill.muarrif.clone());
            sijillat.push(sijill);
        }
    }
    for sijill in sijillat_min_imtidadat(jidhr, tanbihat) {
        if !maruf.contains(&sijill.muarrif) {
            maruf.push(sijill.muarrif.clone());
            sijillat.push(sijill);
        }
    }

    sijillat
}

/// Records from the export file the user placed at the data root.
fn sijillat_min_tasdir(jidhr: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<SijillPlaynite> {
    let masar = jidhr.join(MALAF_TASDIR);
    if !masar.is_file() {
        return Vec::new();
    }
    let Some(qeema) = iqra_json(&masar) else {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            masar.display().to_string(),
            "this export could not be read as JSON, or is larger than this build will load. \
             Re-export it as a plain JSON array of Playnite game records.",
        ));
        return Vec::new();
    };

    let sijillat = sijillat_min_qeema(&qeema, &masar.display().to_string(), tanbihat);
    if sijillat.is_empty() {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            masar.display().to_string(),
            "this export parsed as JSON but held no records with both an Id and a name, so \
             nothing in it could be identified. Each record needs Playnite's own Id field: it is \
             what launches the game through Playnite, and a fabricated one would launch nothing.",
        ));
    }
    sijillat
}

/// Records from whatever plugins under `ExtensionsData` kept as JSON.
///
/// Most of them keep configuration, not games, and finding nothing here is the
/// normal outcome rather than a fault. The walk is bounded in three directions —
/// plugin count, files per plugin, and bytes per file — because this directory
/// belongs to third-party extensions and nothing constrains what they write.
fn sijillat_min_imtidadat(jidhr: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<SijillPlaynite> {
    let imtidadat = jidhr.join(MUJALLAD_IMTIDADAT);
    let Ok(qaima) = std::fs::read_dir(&imtidadat) else {
        return Vec::new();
    };

    let mut mujalladat: Vec<PathBuf> = qaima
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| masar.is_dir())
        .collect();
    mujalladat.sort();
    mujalladat.truncate(AQSA_MUJALLADAT_IMTIDAD);

    let mut sijillat: Vec<SijillPlaynite> = Vec::new();
    for mujallad in mujalladat {
        let Ok(dakhili) = std::fs::read_dir(&mujallad) else {
            continue;
        };
        let mut malaffat: Vec<PathBuf> = dakhili
            .flatten()
            .map(|madkhal| madkhal.path())
            .filter(|masar| {
                masar
                    .extension()
                    .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("json"))
            })
            .collect();
        malaffat.sort();
        malaffat.truncate(AQSA_MALAFFAT_IMTIDAD);

        for masar in malaffat {
            // A plugin's own settings file is never a game list, and skipping it
            // by name saves parsing the one file every extension has.
            if masar
                .file_name()
                .is_some_and(|ism| ism.eq_ignore_ascii_case(MALAF_IDADAT))
            {
                continue;
            }
            let Some(qeema) = iqra_json(&masar) else {
                continue;
            };
            sijillat.extend(sijillat_min_qeema(
                &qeema,
                &masar.display().to_string(),
                tanbihat,
            ));
        }
    }
    sijillat
}

/// Every game record inside one JSON document.
///
/// Three shapes are accepted, because three shapes occur: a bare array, an
/// object with the array under `Games`, `Library` or `Items`, and an object
/// keyed by GUID whose values are the records. Anything else contributes
/// nothing and is not a fault — a plugin's settings file is JSON too.
fn sijillat_min_qeema(
    qeema: &Value,
    mustanad: &str,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Vec<SijillPlaynite> {
    let mut kham: Vec<&Value> = Vec::new();

    if let Some(qaima) = qeema.as_array() {
        kham.extend(qaima.iter());
    } else if let Some(kaain) = qeema.as_object() {
        let mut wujida = false;
        for miftah in ["Games", "games", "Library", "library", "Items", "items"] {
            if let Some(qaima) = kaain.get(miftah).and_then(Value::as_array) {
                kham.extend(qaima.iter());
                wujida = true;
                break;
            }
        }
        if !wujida {
            kham.extend(kaain.values().filter(|dakhili| dakhili.is_object()));
        }
    }

    let mut sijillat: Vec<SijillPlaynite> = Vec::new();
    let mut bila_muarrif: usize = 0;
    for wahid in kham {
        match sijill_min_qeema(wahid, mustanad) {
            Some(sijill) => sijillat.push(sijill),
            None => bila_muarrif = bila_muarrif.saturating_add(1),
        }
    }

    // Only worth a warning when the document plainly *was* a game list and some
    // of it was unusable. A settings file rejecting every one of its three keys
    // is not news.
    if !sijillat.is_empty() && bila_muarrif > 0 {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            mustanad.to_owned(),
            format!(
                "{bila_muarrif} records in this file carry no Playnite Id or no name, so they \
                 could not be identified and were skipped; {} were read.",
                sijillat.len()
            ),
        ));
    }
    sijillat
}

/// One record, when it carries enough to be one.
///
/// An `Id` and a name are both required. The Id is Playnite's own GUID and is
/// what [`MasdarLuba::miftah_tashghil`] hands back to launch the game — a record
/// without one describes a game nothing can start, and deriving a substitute
/// would produce an identity that changes the next time the file is exported.
fn sijill_min_qeema(qeema: &Value, mustanad: &str) -> Option<SijillPlaynite> {
    let muarrif = nass_haql(qeema, "Id")
        .or_else(|| nass_haql(qeema, "id"))
        .map(|nass| nass.trim_matches(['{', '}']).to_lowercase())
        .filter(|nass| !nass.is_empty())?;

    let ism = nass_haql(qeema, "Name")
        .or_else(|| nass_haql(qeema, "name"))
        .or_else(|| nass_haql(qeema, "Title"))
        .or_else(|| nass_haql(qeema, "title"))?;

    Some(SijillPlaynite {
        muarrif,
        ism,
        jidhr: nass_haql(qeema, "InstallDirectory")
            .or_else(|| nass_haql(qeema, "installDirectory"))
            .or_else(|| nass_haql(qeema, "InstallDir"))
            .or_else(|| nass_haql(qeema, "installDir")),
        tanfidhi: nass_haql(qeema, "GameImagePath")
            .or_else(|| nass_haql(qeema, "gameImagePath"))
            .or_else(|| nass_haql(qeema, "Executable")),
        ism_masdar: ism_masdar(qeema),
        muarrif_matjar: nass_haql(qeema, "GameId").or_else(|| nass_haql(qeema, "gameId")),
        // Absent means installed. Playnite writes `IsInstalled` on every record
        // it exports, and a hand-written list that omits it is a list of games
        // somebody has — treating those as uninstalled would drop the whole file.
        muthabbata: sawab_haql(qeema, "IsInstalled")
            .or_else(|| sawab_haql(qeema, "isInstalled"))
            .unwrap_or(true),
        akhir_laab: nass_haql(qeema, "LastActivity").or_else(|| nass_haql(qeema, "lastActivity")),
        hajm: raqm_haql(qeema, "InstallSize")
            .or_else(|| raqm_haql(qeema, "installSize"))
            .unwrap_or(0),
        ghilaf_nisbi: nass_haql(qeema, "CoverImage").or_else(|| nass_haql(qeema, "coverImage")),
        batl_nisbi: nass_haql(qeema, "BackgroundImage")
            .or_else(|| nass_haql(qeema, "backgroundImage")),
        shiar_nisbi: nass_haql(qeema, "Icon").or_else(|| nass_haql(qeema, "icon")),
        mustanad: mustanad.to_owned(),
    })
}

/// The library source's display name.
///
/// Playnite serializes `Source` as an object with a `Name` on a full export and
/// as a plain string on a reduced one, and `SourceName` appears on records
/// flattened by an exporter. All three are read rather than one being declared
/// correct, because the cost of accepting all three is this function.
fn ism_masdar(qeema: &Value) -> Option<String> {
    if let Some(nass) = nass_haql(qeema, "SourceName").or_else(|| nass_haql(qeema, "sourceName")) {
        return Some(nass);
    }
    let masdar = qeema.get("Source").or_else(|| qeema.get("source"))?;
    if let Some(nass) = masdar.as_str() {
        let munaqqa = nass.trim();
        return (!munaqqa.is_empty()).then(|| munaqqa.to_owned());
    }
    nass_haql(masdar, "Name").or_else(|| nass_haql(masdar, "name"))
}

// ---------------------------------------------------------------------------
// identity
// ---------------------------------------------------------------------------

/// The store behind a Playnite entry, when one can be determined without
/// guessing.
///
/// Resolved from the source's display name *and* the plugin-side game id
/// together. Both are required: a source name alone says which launcher, not
/// which game, and a game id alone says nothing about whose namespace it is in.
/// A source Taarib does not recognise, an id in the wrong shape for that
/// store — a Steam entry whose id is not a number, a GOG entry whose id is not
/// a product number — and a record with neither field all return [`None`], and
/// the entry then keeps only its Playnite GUID.
///
/// That is the conservative direction on purpose. A `None` here costs the user a
/// patch match they might have had; a wrong answer offers them somebody else's
/// patch for their game.
fn masdar_asli(ism_masdar: Option<&str>, muarrif_matjar: Option<&str>) -> Option<MasdarLuba> {
    let masdar = ism_masdar?.trim().to_lowercase();
    let raqm = muarrif_matjar?.trim();
    if raqm.is_empty() {
        return None;
    }

    match masdar.as_str() {
        "steam" => raqm.parse::<u32>().ok().map(MasdarLuba::Steam),
        "gog" | "gog.com" | "gog galaxy" => raqm.parse::<u64>().ok().map(MasdarLuba::Gog),
        "epic" | "epic games" | "epic games store" => Some(MasdarLuba::Epic(raqm.to_owned())),
        "origin" | "ea" | "ea app" | "electronic arts" => Some(MasdarLuba::Ea(raqm.to_owned())),
        "uplay" | "ubisoft" | "ubisoft connect" => {
            raqm.parse::<u32>().ok().map(MasdarLuba::Ubisoft)
        },
        "battle.net" | "battlenet" | "blizzard" => Some(MasdarLuba::BattleNet(raqm.to_owned())),
        "xbox" | "microsoft store" | "microsoft" | "windows store" => {
            Some(MasdarLuba::Xbox(raqm.to_owned()))
        },
        "itch" | "itch.io" => raqm.parse::<i64>().ok().map(MasdarLuba::Itch),
        "amazon" | "amazon games" => Some(MasdarLuba::Amazon(raqm.to_owned())),
        "rockstar" | "rockstar games" => raqm.parse::<u32>().ok().map(MasdarLuba::Rockstar),
        "riot" | "riot games" => Some(MasdarLuba::Riot(raqm.to_owned())),
        _ => None,
    }
}

/// Turns one recovered record into a discovered game.
///
/// # Errors
///
/// Returns the sentence for the warning when the record names no install
/// directory, when the directory it names is not there, and when a path inside a
/// Wine prefix does not resolve through that prefix's drive map. All three mean
/// the same thing to the user — the entry is in Playnite and the files are not
/// where it says — and all three are worth saying separately because the
/// remedies differ.
fn luba_min_sijill(
    sijill: &SijillPlaynite,
    jidhr: &JidhrPlaynite,
    maktaba: &MaktabatPlaynite,
    nizam: NizamTashghil,
) -> Result<LubaMuktashafa, String> {
    let Some(kham) = sijill
        .jidhr
        .as_deref()
        .map(str::trim)
        .filter(|nass| !nass.is_empty())
    else {
        return Err(
            "this Playnite entry records no install directory, so there is nothing to probe. \
             Entries with no directory are usually ones whose launcher was uninstalled."
                .to_owned(),
        );
    };

    let jidhr_luba = hall_masar_luba(kham, jidhr)?;
    if !jidhr_luba.is_dir() {
        return Err(format!(
            "Playnite lists this game as installed at {}, and the folder is not there. It was \
             removed outside Playnite, or lives on a drive that is not mounted.",
            jidhr_luba.display()
        ));
    }

    let masdar = MasdarLuba::Playnite(Box::new(MasdarPlaynite {
        muarrif: sijill.muarrif.clone(),
        asl: masdar_asli(
            sijill.ism_masdar.as_deref(),
            sijill.muarrif_matjar.as_deref(),
        ),
    }));

    let mut simat: Vec<SimatLuba> = Vec::new();
    let beea = beeat_sijill(jidhr, &mut simat);
    if nizam != NizamTashghil::Windows && jidhr.beea.is_none() {
        simat.push(SimatLuba::TabaqatTawafuq(
            "this entry came from a Playnite data folder that is not inside a Wine prefix, so \
             the paths in it are Windows paths with no prefix to resolve them"
                .to_owned(),
        ));
    }

    let tanfidhi = sijill
        .tanfidhi
        .as_deref()
        .and_then(|nisbi| tanfidhi_dakhil(&jidhr_luba, nisbi))
        .or_else(|| tanfidhi_wahid(&jidhr_luba));

    Ok(LubaMuktashafa {
        masdar,
        hala_matjar: None,
        ism: sijill.ism.clone(),
        jidhr: jidhr_luba,
        tanfidhi,
        hajm: sijill.hajm,
        // Playnite records no build identity of its own. It stores whatever the
        // library plugin gave it, which for every plugin is the store's version
        // string — and this reader cannot reach it without the database.
        bina_manassa: None,
        akhir_tahdith: None,
        akhir_laab: sijill.akhir_laab.clone(),
        beea,
        suwar: suwar_sijill(sijill, maktaba),
        khiyarat_tashghil: None,
        muktamila: true,
        simat,
    })
}

/// Resolves a record's install directory to a host path.
///
/// Inside a Wine prefix the record holds a Windows path, and a Windows path is
/// only meaningful through that prefix's own drive map — `C:` is
/// `<prefix>/drive_c` by convention and by nothing stronger, and `D:` is
/// routinely a directory somewhere else entirely. So the translation goes
/// through [`crate::beea`] rather than through string surgery.
fn hall_masar_luba(kham: &str, jidhr: &JidhrPlaynite) -> Result<PathBuf, String> {
    let Some(beea) = jidhr.beea.as_ref() else {
        return Ok(PathBuf::from(kham));
    };
    let maalumat = crate::beea::hal_beea(beea)
        .map_err(|khata| format!("{}: {}", beea.display(), khata.injilizi))?;
    maalumat.ila_mudif(kham).map_err(|khata| {
        format!(
            "{kham} does not resolve inside the Wine prefix at {}: {}",
            beea.display(),
            khata.injilizi
        )
    })
}

/// The compatibility environment a Playnite entry runs in.
///
/// A data root found inside a prefix means the whole installation is a Windows
/// one living in that prefix, so every game it lists runs there too. Nothing
/// else in this adapter knows which Wine build the prefix is, so it is read once
/// here from the prefix itself.
fn beeat_sijill(jidhr: &JidhrPlaynite, simat: &mut Vec<SimatLuba>) -> BeeatTawafuq {
    let Some(beea) = jidhr.beea.clone() else {
        return BeeatTawafuq::Asli;
    };
    let isdar = crate::beea::hal_beea(&beea)
        .ok()
        .and_then(|maalumat| maalumat.isdar_wine);
    simat.push(SimatLuba::TabaqatTawafuq(
        isdar.clone().unwrap_or_else(|| "Wine".to_owned()),
    ));
    BeeatTawafuq::Wine { isdar, beea }
}

/// The executable a record names, resolved inside the install root.
///
/// Goes through [`taarib_usus::masarat::dakhil`] rather than a join, because the
/// value comes out of a JSON file this adapter did not write: a relative path
/// with `..` in it would otherwise point Phase 5's probe at a file outside the
/// game.
fn tanfidhi_dakhil(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munaqqa = nisbi.trim().replace('\\', std::path::MAIN_SEPARATOR_STR);
    if munaqqa.is_empty() {
        return None;
    }
    let murashah = Path::new(&munaqqa);
    let kamil = if murashah.is_absolute() {
        murashah
            .starts_with(jidhr)
            .then(|| murashah.to_path_buf())?
    } else {
        taarib_usus::masarat::dakhil(jidhr, &munaqqa).ok()?
    };
    kamil.is_file().then_some(kamil)
}

/// The executable at an install root, when there is exactly one.
///
/// Playnite records `GameImagePath` only for entries the user added by hand, so
/// most recovered records name no executable at all. One `.exe` sitting at the
/// root of the install directory is not a guess — it is the answer, and it is
/// the file Phase 5 would pick anyway. Two or more *is* a guess, and Phase 5
/// makes better ones than this adapter can from a directory listing, so it is
/// left to make them.
fn tanfidhi_wahid(jidhr_luba: &Path) -> Option<PathBuf> {
    let qaima = std::fs::read_dir(jidhr_luba).ok()?;
    let mut murashahat: Vec<PathBuf> = qaima
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            masar.is_file()
                && masar
                    .extension()
                    .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("exe"))
        })
        .collect();
    murashahat.sort();
    if murashahat.len() != 1 {
        return None;
    }
    murashahat.into_iter().next()
}

/// A record's artwork: whatever it names, falling back to the media tree.
///
/// The paths a record carries are relative to `library/files`, so they are
/// resolved through the containment check and accepted only when they land
/// inside it. When a record names nothing, the media directory keyed by the
/// game's own GUID is used, which is where Playnite actually keeps it.
fn suwar_sijill(sijill: &SijillPlaynite, maktaba: &MaktabatPlaynite) -> MasadirSuwar {
    let malaffat = maktaba.jidhr_maktaba.join(MUJALLAD_MALAFFAT);
    let hall = |nisbi: Option<&String>| -> Option<MasdarSura> {
        let nass = nisbi?.trim().replace('\\', std::path::MAIN_SEPARATOR_STR);
        if nass.is_empty() {
            return None;
        }
        let kamil = taarib_usus::masarat::dakhil(&malaffat, &nass).ok()?;
        kamil.is_file().then_some(MasdarSura::Malaf(kamil))
    };

    let mut masadir = MasadirSuwar {
        ghilaf: hall(sijill.ghilaf_nisbi.as_ref()),
        batl: hall(sijill.batl_nisbi.as_ref()),
        shiar: hall(sijill.shiar_nisbi.as_ref()),
    };

    if let Some(mukhazzana) = maktaba.suwar.get(&sijill.muarrif) {
        if masadir.ghilaf.is_none() {
            masadir.ghilaf.clone_from(&mukhazzana.ghilaf);
        }
        if masadir.batl.is_none() {
            masadir.batl.clone_from(&mukhazzana.batl);
        }
        if masadir.shiar.is_none() {
            masadir.shiar.clone_from(&mukhazzana.shiar);
        }
    }
    masadir
}

// ---------------------------------------------------------------------------
// small JSON readers
// ---------------------------------------------------------------------------

/// Reads and parses a JSON file within the size ceiling, or nothing.
///
/// Absent, too large and malformed are not distinguished, because the callers
/// treat them identically: all three mean this file contributed nothing, and the
/// caller that cares reports the path it tried.
fn iqra_json(masar: &Path) -> Option<Value> {
    let bayanat = std::fs::metadata(masar).ok()?;
    if !bayanat.is_file() || bayanat.len() > AQSA_HAJM_JSON {
        return None;
    }
    let bayt = std::fs::read(masar).ok()?;
    let nass = String::from_utf8_lossy(&bayt);
    serde_json::from_str(nass.trim_start_matches('\u{feff}')).ok()
}

/// A string field, trimmed, or nothing when it is absent or empty.
fn nass_haql(qeema: &Value, miftah: &str) -> Option<String> {
    let nass = qeema.get(miftah)?.as_str()?.trim();
    (!nass.is_empty()).then(|| nass.to_owned())
}

/// A boolean field, however the writer encoded it.
///
/// Playnite writes real booleans; an exporter that went through a spreadsheet
/// writes `"true"` and `"False"`. Both are read, because rejecting the second
/// would silently drop every game in such a file as uninstalled.
fn sawab_haql(qeema: &Value, miftah: &str) -> Option<bool> {
    let haql = qeema.get(miftah)?;
    haql.as_bool()
        .or_else(|| match haql.as_str()?.trim().to_lowercase().as_str() {
            "true" | "yes" | "1" => Some(true),
            "false" | "no" | "0" => Some(false),
            _ => None,
        })
}

/// A whole-number field, however the writer encoded it.
fn raqm_haql(qeema: &Value, miftah: &str) -> Option<u64> {
    let haql = qeema.get(miftah)?;
    haql.as_u64().or_else(|| haql.as_str()?.trim().parse().ok())
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The data roots one candidate list names, in order.
    fn masarat(judhur: &[JidhrPlaynite]) -> Vec<PathBuf> {
        judhur.iter().map(|jidhr| jidhr.masar.clone()).collect()
    }

    #[test]
    fn judhur_windows_min_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let mutajawwila = masrah.path().join("Roaming");
        let mahalliya = masrah.path().join("Local");

        // Neither `%APPDATA%` nor `%LOCALAPPDATA%` is set here, and since
        // edition 2024 neither could be: both directories now arrive on the
        // context, which is the only reason this pair is checkable at all.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_mutajawwila = Some(mutajawwila.clone());
        siyaq.bayanat_mahalliya = Some(mahalliya.clone());

        let judhur = masarat(&MatjarPlaynite::jadeed().judhur_muhtamala(&siyaq));
        assert!(judhur.contains(&mutajawwila.join("Playnite")), "{judhur:?}");
        assert!(judhur.contains(&mahalliya.join("Playnite")), "{judhur:?}");
        Ok(())
    }

    #[test]
    fn bila_mujalladat_bayanat_tabqa_al_judhur_al_mahmula() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // A portable folder is found by paths that are home-relative or
        // absolute, so only the two application-data candidates fall away on a
        // context that has no such directories.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        let judhur = masarat(&MatjarPlaynite::jadeed().judhur_muhtamala(&siyaq));
        assert_eq!(judhur.len(), 4, "{judhur:?}");
        assert!(
            judhur.contains(&masrah.path().join("Playnite")),
            "{judhur:?}"
        );
        Ok(())
    }
}
