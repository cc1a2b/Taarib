//! الفحص — what a scan is, and what every launcher adapter must answer.
//!
//! Ten sources, one trait. A launcher adapter reads that launcher's own real
//! catalogue format and returns a list of games; it does not decide identity, it
//! does not merge duplicates, it does not touch the database, and it never
//! writes anything anywhere. Everything downstream of a scan works on
//! [`LubaMuktashafa`] regardless of which launcher produced it, which is what
//! keeps the ten adapters from growing ten different notions of what a game is.
//!
//! The rule that shapes the whole module: **a malformed catalogue entry degrades
//! that one entry, never the scan.** A launcher that has been half-uninstalled,
//! a manifest truncated by a power cut, a game whose install directory was
//! deleted from underneath its launcher — all of these are ordinary on real
//! machines, and a scan that aborted on the first one would leave a user with an
//! empty library and no explanation. Each becomes a [`TanbihFahs`] attached to
//! the result, visible in Diagnostics, and the other games still arrive.

use std::path::PathBuf;
use std::time::Duration;

use taarib_mustalahat::ghiyab::SababGhiyab;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::idadat::IdadatManassat;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

/// Where a piece of artwork can be obtained.
///
/// A local file is always preferred and is tried first: the launcher already
/// downloaded it, it costs nothing, and it works with no network. A URL is the
/// fallback for what the local cache does not have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasdarSura {
    /// A file the launcher already has on disk.
    Malaf(PathBuf),
    /// An address the launcher publishes.
    Rabt(String),
}

/// Where a game's three artwork pieces can be found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MasadirSuwar {
    /// The vertical cover, which is what the library grid shows.
    pub ghilaf: Option<MasdarSura>,
    /// The wide banner behind the detail header.
    pub batl: Option<MasdarSura>,
    /// The small logo overlaid on the banner.
    pub shiar: Option<MasdarSura>,
}

impl MasadirSuwar {
    /// Whether anything at all was found.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ghilaf.is_none() && self.batl.is_none() && self.shiar.is_none()
    }
}

/// Something a launcher's own metadata says about a game that the safety layer
/// will later need.
///
/// Collected during discovery rather than probed later because this is where the
/// information lives: Steam knows a game is VAC-secured, and nothing on disk
/// says so. These are **hints**, not conclusions — Phase 16 detects anti-cheat
/// by evidence on disk and refuses on that basis. A hint here makes the refusal
/// faster and lets the interface warn before the user clicks.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimatLuba {
    /// The launcher lists this game as playable with others locally.
    JamaiMahalli,
    /// The launcher lists online multiplayer.
    JamaiOnline,
    /// The launcher associates the game with an anti-cheat service.
    HimayaMuhtamala(String),
    /// Steam reports the game as VAC-secured.
    MuammanaVac,
    /// The launcher marks this entry as something other than a game — a tool, a
    /// runtime, a soundtrack, a demo — so the library can hide it by default.
    LaysatLuba(String),
    /// The launcher records that this title runs through a compatibility layer.
    TabaqatTawafuq(String),
    /// No launcher's catalogue named this game. Taarib found it by scanning a
    /// folder the user nominated and recognising an engine's own layout on disk.
    ///
    /// The string names the evidence that admitted it — `Unity: Game.exe beside
    /// Game_Data`, `Godot: data.pck` — because a heuristic that cannot say why
    /// it fired is a heuristic nobody can correct.
    ///
    /// A game marked this way is a real game and is patched exactly like any
    /// other. What is different is upstream of the patch: nobody vouches for it,
    /// its identity is a hash of its executable's path rather than a store
    /// identifier, and moving the folder gives it a new identity. The interface
    /// says so, so that a user who wonders why their patch state did not follow
    /// a moved game has the answer on the card.
    MuktashafaBilIstidlal(String),
    /// This entry is a directory of ROM or disc images for an emulator, not a
    /// program on disk.
    ///
    /// It exists so that a library is complete, **not** so that it can be
    /// translated. Taarib patches files a game reads at runtime; a ROM is a
    /// single opaque image whose text lives inside a cartridge dump in an
    /// encoding the emulator never sees separately, and there is no supported
    /// path by which this product modifies one. The string names the console or
    /// emulator, however precisely the source knew it — a launcher that records
    /// the runner names it exactly, and a directory scan names the family its
    /// file extensions belong to — so the interface can say what it is looking
    /// at rather than "an emulated game".
    MuhakatRum(String),
}

/// A game exactly as one launcher describes it, before anything merges or
/// identifies it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LubaMuktashafa {
    /// The launcher's own identity for it.
    pub masdar: MasdarLuba,
    /// The name the launcher gives, kept verbatim.
    pub ism: String,
    /// The installation root.
    pub jidhr: PathBuf,
    /// The executable, when the catalogue names one. Many launchers do not, and
    /// Phase 5 finds it by probing the directory instead.
    pub tanfidhi: Option<PathBuf>,
    /// Size on disk in bytes, as the launcher reports it. Zero when it does not.
    pub hajm: u64,
    /// The launcher's build identifier, where it has one. Steam reports a
    /// numeric `buildid`; several launchers report nothing at all, which is
    /// exactly why content fingerprints exist.
    pub bina_manassa: Option<String>,
    /// When the launcher last updated the game, RFC 3339.
    pub akhir_tahdith: Option<String>,
    /// When the user last played, RFC 3339, where the launcher records it.
    pub akhir_laab: Option<String>,
    /// The compatibility layer this game runs behind.
    pub beea: BeeatTawafuq,
    /// Where its artwork can be found.
    pub suwar: MasadirSuwar,
    /// The launch options the user already set, so an installer extends them
    /// rather than overwriting them.
    pub khiyarat_tashghil: Option<String>,
    /// Whether the game is fully downloaded. A partially downloaded game is
    /// discovered and shown, but is never offered as installable — patching a
    /// half-downloaded game would be patching files the launcher is about to
    /// overwrite.
    pub muktamila: bool,
    /// A state the launcher is reporting about this game right now.
    ///
    /// The existence gate's first check, and the only one that can say *why* a
    /// game is unavailable in the launcher's own terms rather than the
    /// filesystem's. A launcher that knows a download is paused at four percent
    /// knows something no amount of directory inspection recovers: the
    /// filesystem sees a small folder, which is
    /// [`SababGhiyab::TathbeetNaqis`] — true, and useless next to "paused at
    /// four percent, press resume".
    ///
    /// [`None`] from every launcher that records no such state, which is most
    /// of them. That is not a gap to be filled by inference: a scanner that
    /// guessed "downloading" from a small directory would tell users to wait
    /// for a download that is not running.
    ///
    /// [`SababGhiyab::TathbeetNaqis`]:
    ///     taarib_mustalahat::ghiyab::SababGhiyab::TathbeetNaqis
    pub hala_matjar: Option<SababGhiyab>,
    /// What the launcher's metadata says about it.
    pub simat: Vec<SimatLuba>,
}

impl LubaMuktashafa {
    /// Whether this entry looks like a game rather than a tool or a runtime.
    #[must_use]
    pub fn hiya_luba(&self) -> bool {
        !self.simat.iter().any(|sima| matches!(sima, SimatLuba::LaysatLuba(_)))
    }

    /// The evidence that admitted a heuristically discovered game, when it was
    /// one.
    ///
    /// [`None`] for everything a launcher's own catalogue named. Callers use it
    /// to badge the entry and to explain, when a folder moves and the identity
    /// changes with it, why that happened — see
    /// [`SimatLuba::MuktashafaBilIstidlal`].
    #[must_use]
    pub fn dalil_istidlal(&self) -> Option<&str> {
        self.simat.iter().find_map(|sima| match sima {
            SimatLuba::MuktashafaBilIstidlal(dalil) => Some(dalil.as_str()),
            _ => None,
        })
    }

    /// The ROM family this entry holds, when it is an emulated title rather than
    /// a program.
    ///
    /// Present means Taarib will not patch it, whatever the engine probe later
    /// says, because there is nothing on disk for a patch to replace. It is in
    /// the library so that the library is complete.
    #[must_use]
    pub fn ailat_rum(&self) -> Option<&str> {
        self.simat.iter().find_map(|sima| match sima {
            SimatLuba::MuhakatRum(aila) => Some(aila.as_str()),
            _ => None,
        })
    }

    /// Whether the launcher's metadata suggests the safety layer will have
    /// something to say.
    #[must_use]
    pub fn tahtaj_fahs_aman(&self) -> bool {
        self.simat.iter().any(|sima| {
            matches!(
                sima,
                SimatLuba::JamaiOnline | SimatLuba::HimayaMuhtamala(_) | SimatLuba::MuammanaVac
            )
        })
    }
}

/// One entry a scan could not read, and why.
///
/// Carries enough to act on: which launcher, which file, and what was wrong with
/// it. A user whose game is missing from the library can look here and find the
/// answer instead of filing a bug that says "my game is not showing up".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TanbihFahs {
    /// The launcher whose catalogue it came from.
    pub matjar: &'static str,
    /// The file or entry, as specifically as it can be named.
    pub mawdi: String,
    /// What was wrong, in a sentence.
    pub sabab: String,
}

impl TanbihFahs {
    /// Builds a warning.
    #[must_use]
    pub fn jadeed(
        matjar: &'static str,
        mawdi: impl Into<String>,
        sabab: impl Into<String>,
    ) -> Self {
        Self { matjar, mawdi: mawdi.into(), sabab: sabab.into() }
    }
}

/// What one launcher's scan produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NatijatMatjar {
    /// The launcher's identifier.
    pub matjar: &'static str,
    /// Where the launcher itself is installed, when it was found.
    pub jidhr_matjar: Option<PathBuf>,
    /// The games.
    pub alaab: Vec<LubaMuktashafa>,
    /// Every entry that degraded.
    pub tanbihat: Vec<TanbihFahs>,
    /// How long the scan took, for the diagnostics screen — a launcher whose
    /// scan takes seconds is a launcher whose catalogue lives on a slow or
    /// disconnected drive, and that is worth seeing.
    pub muddat: Duration,
}

impl NatijatMatjar {
    /// An empty result for a launcher that is not installed. Not a failure: most
    /// machines have two or three of the ten.
    #[must_use]
    pub fn ghayr_mutah(matjar: &'static str) -> Self {
        Self { matjar, ..Self::default() }
    }
}

/// Everything an adapter is given.
///
/// Passed by reference to every adapter so that none of them resolves an
/// *ambient* fact for itself: the platform, the user's home directory, Windows'
/// program and data directories, the XDG base directories. Seventeen adapters
/// consult those, so seventeen private resolvers would be seventeen chances to
/// disagree — and the one that disagreed would be the one nobody could test,
/// because since edition 2024 `std::env::set_var` is `unsafe` and racy, so a
/// resolver that reads the environment inline cannot be pointed anywhere by a
/// test without mutating the process every other test is sharing.
///
/// The rule is about *ambient* facts, and it is exactly that narrow. A variable
/// that belongs to one launcher and is read by the one adapter that owns it —
/// `LEGENDARY_CONFIG_PATH` is the only one left — is that launcher speaking
/// about itself, exactly like a file in its own configuration directory. There
/// is nobody for it to disagree with, and hoisting it here would put one
/// launcher's vocabulary on the struct the other sixteen are handed.
#[derive(Debug, Clone)]
pub struct SiyaqFahs {
    /// The operating system.
    pub nizam: NizamTashghil,
    /// Launcher path overrides and scanning behaviour, from the user's settings.
    pub manassat: IdadatManassat,
    /// The user's home directory, resolved once.
    pub manzil: PathBuf,
    /// Windows' machine-wide program directories, in the order a launcher is
    /// looked for in them. Empty on Linux and macOS, which have no such folder.
    ///
    /// Resolved once by [`crate::siyaq_fahs`] for the same reason [`Self::manzil`]
    /// is: both folders are relocatable at Windows setup time and
    /// `%ProgramFiles%` is the only record of where they went, so an adapter
    /// that wrote `C:\Program Files` instead would be right on every machine
    /// whose Windows is on `C:` and quietly wrong on the rest.
    pub mujalladat_baramij: Vec<PathBuf>,
    /// Windows' machine-wide application data directory, `%PROGRAMDATA%`.
    ///
    /// Five launchers keep their catalogue under it — the EA app, Origin,
    /// Battle.net's Agent, Epic and GOG Galaxy — because one machine has one
    /// copy of each however many people log in. [`None`] on Linux and macOS,
    /// which have no such folder; the `C:\ProgramData` default stands in only
    /// on a Windows machine whose variable is unset, which happens in stripped
    /// service environments.
    pub bayanat_barnamij: Option<PathBuf>,
    /// Windows' per-user roaming application data directory, `%APPDATA%`.
    ///
    /// [`None`] off Windows. Read from the environment rather than assembled
    /// under [`Self::manzil`] because a domain profile can redirect it to a
    /// network share, and the home-relative layout is the fallback for the
    /// ordinary case where it is not set.
    pub bayanat_mutajawwila: Option<PathBuf>,
    /// Windows' per-user local application data directory, `%LOCALAPPDATA%`.
    ///
    /// [`None`] off Windows, and redirectable for the same reason
    /// [`Self::bayanat_mutajawwila`] is.
    pub bayanat_mahalliya: Option<PathBuf>,
    /// `$XDG_DATA_HOME`, or the home-relative default the specification names.
    ///
    /// Unlike the Windows folders above this always has a value: the XDG
    /// specification defines the fallback for every machine, so there is no
    /// state to represent with [`None`]. Which platform consults it is the
    /// adapters' business — Bottles and Lutris do, and nothing on Windows does.
    ///
    /// A relative value is ignored rather than resolved, here and for the two
    /// below, because the specification says a relative value is invalid and
    /// because accepting one would make discovery depend on the directory this
    /// process happened to start in.
    pub khazina_bayanat: PathBuf,
    /// `$XDG_CONFIG_HOME`, or the home-relative default.
    pub khazina_idadat: PathBuf,
    /// `$XDG_CACHE_HOME`, or the home-relative default.
    pub khazina_makhbaa: PathBuf,
    /// Whether to look inside Flatpak and Snap layouts as well as native ones.
    /// On by default on Linux, where a large share of Steam installations are
    /// Flatpak and a scanner that only knows the native path finds nothing.
    pub yashmal_hawiyat: bool,
}

impl SiyaqFahs {
    /// Resolves a launcher root: the user's override when they set one,
    /// otherwise whatever the adapter found.
    #[must_use]
    pub fn jidhr_mufaddal(
        tajawuz: Option<&PathBuf>,
        muktashaf: Option<PathBuf>,
    ) -> Option<PathBuf> {
        tajawuz.cloned().or(muktashaf)
    }

    /// A context that knows a platform and a home directory and nothing else,
    /// for a test that then sets the one field it is about.
    ///
    /// Every Windows directory is [`None`] and every XDG directory is the
    /// specification's default *under the given home*, so a test that forgets
    /// to set the field it is exercising gets an empty answer rather than the
    /// developer's own machine — which is the difference between a test that
    /// proves resolution comes off the context and one that passes because this
    /// machine happened to agree with the hardcoded fallback it replaced.
    #[cfg(test)]
    pub(crate) fn lil_ikhtibar(nizam: NizamTashghil, manzil: &std::path::Path) -> Self {
        Self {
            nizam,
            manassat: IdadatManassat::default(),
            manzil: manzil.to_path_buf(),
            mujalladat_baramij: Vec::new(),
            bayanat_barnamij: None,
            bayanat_mutajawwila: None,
            bayanat_mahalliya: None,
            khazina_bayanat: manzil.join(".local").join("share"),
            khazina_idadat: manzil.join(".config"),
            khazina_makhbaa: manzil.join(".cache"),
            yashmal_hawiyat: false,
        }
    }
}

/// One launcher.
///
/// Seventeen implementations, one shape. An adapter is read-only, is never
/// required to have its launcher running, and never launches one to find out
/// what it knows — a scanner that started Steam to enumerate games would be a
/// scanner nobody leaves enabled.
pub trait Matjar: Send + Sync {
    /// The stable identifier, matching the registry's shard family: `steam`,
    /// `epic`, `gog`, `ea`, `ubisoft`, `battlenet`, `xbox`, `itch`, `amazon`,
    /// `rockstar`, `riot`, `heroic`, `legendary`, `lutris`, `bottles`,
    /// `playnite`, `yadawi`.
    ///
    /// The four managers — Heroic, Legendary, Lutris and Playnite — report a
    /// family here and a *different* one through
    /// [`taarib_mustalahat::luba::MasdarLuba::aila`], which resolves to the
    /// store behind the game. That is deliberate: this identifier names who
    /// found the game, and `aila` names whose patch shard it belongs in.
    fn muarrif(&self) -> &'static str;

    /// The launcher's name as the interface writes it in Arabic.
    fn ism_arabi(&self) -> &'static str;

    /// The launcher's name in English.
    fn ism_injilizi(&self) -> &'static str;

    /// Which platforms this adapter can find anything on.
    fn manassat_maduma(&self) -> &'static [NizamTashghil];

    /// Where the launcher is installed, or `None` when it is not.
    ///
    /// Cheap: this is called before every scan to decide whether to scan at all,
    /// so it checks for existence rather than parsing anything.
    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf>;

    /// Reads the launcher's catalogue.
    ///
    /// # Errors
    ///
    /// Only for a failure that makes the whole launcher unreadable — its root
    /// exists but its catalogue cannot be opened at all. Anything narrower is a
    /// [`TanbihFahs`] on the result, because one bad manifest must not cost the
    /// user the other two hundred games.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar>;

    /// The directories worth watching for changes, so a refresh is reactive
    /// rather than polled.
    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf>;
}

/// What a whole scan produced, across every launcher.
#[derive(Debug, Clone, Default)]
pub struct NatijatFahs {
    /// One result per launcher that was scanned, in the order they ran.
    pub matajir: Vec<NatijatMatjar>,
    /// How long the whole scan took.
    pub muddat: Duration,
}

impl NatijatFahs {
    /// Every game found, across every launcher.
    #[must_use]
    pub fn alaab(&self) -> Vec<&LubaMuktashafa> {
        self.matajir.iter().flat_map(|natija| natija.alaab.iter()).collect()
    }

    /// How many games were found.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.matajir.iter().map(|natija| natija.alaab.len()).sum()
    }

    /// Every warning, across every launcher.
    #[must_use]
    pub fn tanbihat(&self) -> Vec<&TanbihFahs> {
        self.matajir.iter().flat_map(|natija| natija.tanbihat.iter()).collect()
    }

    /// The launchers that were actually found installed.
    #[must_use]
    pub fn matajir_mutaha(&self) -> Vec<&'static str> {
        self.matajir
            .iter()
            .filter(|natija| natija.jidhr_matjar.is_some())
            .map(|natija| natija.matjar)
            .collect()
    }
}
