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
//! the result and the other games still arrive.
//!
//! A warning also says *what kind* of gap it is — one named entry, or a whole
//! catalogue, library or folder that could not be read — because the second
//! kind changes what the scan may conclude afterwards: see [`NawTanbih`] and
//! [`HalatFahsMatjar`]. This crate persists nothing, so the warnings are only as
//! visible as the caller makes them. Studio writes every one to its log and to
//! the scan ledger beside the result, and hands them to the interface with the
//! library, so that a game missing from the grid has its reason recorded
//! somewhere a person can read.

use std::path::{Path, PathBuf};
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

/// What a warning says about the rest of the scan.
///
/// The distinction decides what an absence means afterwards. When every warning
/// on a result names one entry, a game missing from that result is a game the
/// launcher does not list, and the store may mark it absent. When one warning
/// says a catalogue, a library root or a folder could not be read, a missing
/// game may simply be one the scan never saw — and marking it absent on that
/// evidence is how a disconnected drive empties somebody's library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawTanbih {
    /// One enumerated entry could not be turned into a game. The entry is named
    /// in the warning; nothing unnamed was missed.
    Madkhal,
    /// A source of entries — the catalogue itself, one library or drive, a
    /// nominated folder, an index file — could not be read end to end. Games
    /// may exist that this scan did not see.
    Fahras,
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
    /// Whether this is about one entry or about a whole source of entries.
    pub naw: NawTanbih,
}

impl TanbihFahs {
    /// A warning about one entry. Everything else in the catalogue was read.
    #[must_use]
    pub fn jadeed(
        matjar: &'static str,
        mawdi: impl Into<String>,
        sabab: impl Into<String>,
    ) -> Self {
        Self { matjar, mawdi: mawdi.into(), sabab: sabab.into(), naw: NawTanbih::Madkhal }
    }

    /// A warning that a source of entries could not be read end to end, so the
    /// result may be missing games it never saw.
    ///
    /// Pushing one of these onto a [`NatijatMatjar`] turns its verdict into
    /// [`HalatFahsMatjar::Naqisa`]; that is the point of the constructor, and
    /// the reason an adapter must choose between the two rather than reach for
    /// [`Self::jadeed`] by habit.
    #[must_use]
    pub fn fahras(
        matjar: &'static str,
        mawdi: impl Into<String>,
        sabab: impl Into<String>,
    ) -> Self {
        Self { matjar, mawdi: mawdi.into(), sabab: sabab.into(), naw: NawTanbih::Fahras }
    }

    /// Whether games may exist that the scan did not see because of this.
    #[must_use]
    pub const fn yukhfi_alaab(&self) -> bool {
        matches!(self.naw, NawTanbih::Fahras)
    }
}

/// What a scan may conclude from one launcher's result.
///
/// Three answers, and the third exists because the first two used to be one:
/// "not installed" and "installed, catalogue unreadable" both produced an empty
/// list, and the absence sweep downstream marked every stored game of a launcher
/// it had never actually read as gone. On a machine whose Epic data folder had
/// lost its `Manifests` directory that greyed out the whole Epic library on
/// every scan, under a launcher the interface said it had searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatFahsMatjar {
    /// The launcher is not on this machine. Nothing was owed, and nothing may be
    /// concluded about the games it once listed.
    GhayrMuthabbat,
    /// The launcher is installed and its catalogue was read end to end. Every
    /// warning names one entry, so a stored game missing from the result is one
    /// the launcher no longer lists. The only verdict an absence sweep may act on.
    Tamma,
    /// The launcher is installed and some part of its catalogue could not be
    /// read. The games that were found are real; the games that were not found
    /// may be too, so nothing may be marked absent on this result's word.
    Naqisa,
}

/// What one launcher's scan produced.
///
/// Built only through [`Self::ghayr_mutah`], [`Self::muthabbat`] and
/// [`Self::naqisa`], each of which has to say whether the launcher was found.
/// There is deliberately no `Default`: a result nobody finished filling in used
/// to look exactly like a launcher that was scanned and had nothing in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatijatMatjar {
    /// The launcher's identifier.
    pub matjar: &'static str,
    /// Where the launcher itself is installed, when a root was found. A
    /// launcher can be installed with no single root to name — Rockstar and
    /// GOG both keep a catalogue in the registry — so this is not the
    /// installed flag; [`Self::hala`] is.
    pub jidhr_matjar: Option<PathBuf>,
    /// The games.
    pub alaab: Vec<LubaMuktashafa>,
    /// Every entry that degraded, and every source of entries that could not be
    /// read. Which is which is on each warning, and [`Self::hala`] reads it.
    pub tanbihat: Vec<TanbihFahs>,
    /// How long the scan took, for the diagnostics screen — a launcher whose
    /// scan takes seconds is a launcher whose catalogue lives on a slow or
    /// disconnected drive, and that is worth seeing.
    pub muddat: Duration,
    /// Whether the launcher was found on this machine at all.
    muthabbat: bool,
}

impl NatijatMatjar {
    /// An empty result for a launcher that is not installed. Not a failure: most
    /// machines have two or three of the ten.
    ///
    /// Only for a launcher whose root could not be located. A root that is
    /// there with no catalogue under it is [`Self::naqisa`], because "Epic is
    /// not here" and "Epic is here and its manifest folder is not" are
    /// different sentences and lead to different repairs.
    #[must_use]
    pub const fn ghayr_mutah(matjar: &'static str) -> Self {
        Self {
            matjar,
            jidhr_matjar: None,
            alaab: Vec::new(),
            tanbihat: Vec::new(),
            muddat: Duration::ZERO,
            muthabbat: false,
        }
    }

    /// An empty result for a launcher that was found, ready for the adapter to
    /// fill. The verdict is [`HalatFahsMatjar::Tamma`] until a
    /// [`TanbihFahs::fahras`] is pushed onto it.
    #[must_use]
    pub const fn muthabbat(matjar: &'static str, jidhr_matjar: Option<PathBuf>) -> Self {
        Self {
            matjar,
            jidhr_matjar,
            alaab: Vec::new(),
            tanbihat: Vec::new(),
            muddat: Duration::ZERO,
            muthabbat: true,
        }
    }

    /// A result for a launcher that was found and whose catalogue could not be
    /// read at all, carrying the one warning that says so.
    ///
    /// The shape every adapter used to collapse into [`Self::ghayr_mutah`]: the
    /// root exists, the file or directory the catalogue lives in does not. The
    /// warning names what was looked for, so the user can put it back or
    /// correct the configured root.
    #[must_use]
    pub fn naqisa(
        matjar: &'static str,
        jidhr_matjar: Option<PathBuf>,
        mawdi: impl Into<String>,
        sabab: impl Into<String>,
    ) -> Self {
        let mut natija = Self::muthabbat(matjar, jidhr_matjar);
        natija.tanbihat.push(TanbihFahs::fahras(matjar, mawdi, sabab));
        natija
    }

    /// What may be concluded from this result.
    ///
    /// Derived rather than stored, so that a catalogue-level warning cannot be
    /// pushed without the verdict changing with it. A result is
    /// [`HalatFahsMatjar::Tamma`] only while every warning on it names one
    /// entry.
    #[must_use]
    pub fn hala(&self) -> HalatFahsMatjar {
        if !self.muthabbat {
            HalatFahsMatjar::GhayrMuthabbat
        } else if self.tanbihat.iter().any(TanbihFahs::yukhfi_alaab) {
            HalatFahsMatjar::Naqisa
        } else {
            HalatFahsMatjar::Tamma
        }
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
/// that belongs to one tool and is read by the one adapter that owns it is that
/// tool speaking about itself, exactly like a file in its own configuration
/// directory. There is nobody for it to disagree with, and hoisting it here
/// would put one launcher's vocabulary on the struct the other sixteen are
/// handed. Four reads are left on that ground:
///
/// - `LEGENDARY_CONFIG_PATH` and `LEGENDARY_WINE_PREFIX`, in `legendary`;
/// - `WINEPREFIX`, Wine's own name for the prefix, read in the two places that
///   resolve a prefix — `legendary` and [`crate::beea::iktashif_beeat`], which
///   *is* the prefix resolver rather than an adapter;
/// - `USER`/`USERNAME` in `crate::beea`, which is not a location at all. It is
///   compared against the directory names inside somebody's Wine prefix to
///   guess which profile is theirs, and a prefix can perfectly well have been
///   created by another account. Modelling it here would put a per-process
///   identity on a struct that answers "where are things", and would invite an
///   adapter to treat it as one.
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
    /// The whole XDG data hierarchy to *search*, in specification order:
    /// [`Self::khazina_bayanat`] first, then every entry of `$XDG_DATA_DIRS`,
    /// defaulting to `/usr/local/share:/usr/share`.
    ///
    /// Separate from [`Self::khazina_bayanat`] because the two answer different
    /// questions and only one of them is a single directory. "Where does this
    /// user's data go" has exactly one answer; "where might a `.desktop` file or
    /// an icon theme have been installed" is a list, and on a machine with
    /// Flatpak, Nix or a distribution that stages `/usr/local` it is a list
    /// whose later entries are where the answer actually is.
    ///
    /// Resolved here rather than at the point of use for the reason every other
    /// field is: the icon resolver used to read `XDG_DATA_HOME` and `HOME` for
    /// itself, which made it a second answer to a question this struct already
    /// owns — and it accepted a relative value, which the specification says is
    /// invalid and which [`crate::siyaq_fahs`] refuses.
    ///
    /// Empty off Linux, where nothing consults it.
    pub judhur_bayanat: Vec<PathBuf>,
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

    /// Where a 32-bit installer lands, or [`None`] off Windows.
    ///
    /// [`Self::mujalladat_baramij`] is `%ProgramFiles(x86)%` then
    /// `%ProgramFiles%`, deduplicated, so the first entry answers this on both
    /// widths of Windows: a 64-bit machine has two entries and the 32-bit
    /// directory leads, and a 32-bit machine reports one directory in both
    /// variables and so keeps one entry, which is the directory every installer
    /// lands in there. An adapter that reached for `.first()` itself would have
    /// to restate that argument, and the one that restated it wrongly would be
    /// wrong only on the machines nobody testing this owns.
    #[must_use]
    pub fn mujallad_baramij_x86(&self) -> Option<&Path> {
        self.mujalladat_baramij.first().map(PathBuf::as_path)
    }

    /// Where a 64-bit installer lands, or [`None`] off Windows.
    ///
    /// The *last* entry, by the deduplication argument on
    /// [`Self::mujallad_baramij_x86`]: two entries on a 64-bit Windows with the
    /// native directory trailing, one entry on a 32-bit Windows which is then
    /// both ends of the list.
    #[must_use]
    pub fn mujallad_baramij_asli(&self) -> Option<&Path> {
        self.mujalladat_baramij.last().map(PathBuf::as_path)
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
    pub(crate) fn lil_ikhtibar(nizam: NizamTashghil, manzil: &Path) -> Self {
        Self {
            nizam,
            manassat: IdadatManassat::default(),
            manzil: manzil.to_path_buf(),
            mujalladat_baramij: Vec::new(),
            bayanat_barnamij: None,
            bayanat_mutajawwila: None,
            bayanat_mahalliya: None,
            khazina_bayanat: manzil.join(".local").join("share"),
            judhur_bayanat: vec![manzil.join(".local").join("share")],
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

    /// The launchers that were found installed, whether or not their catalogue
    /// could be read.
    #[must_use]
    pub fn matajir_mutaha(&self) -> Vec<&'static str> {
        self.matajir
            .iter()
            .filter(|natija| natija.hala() != HalatFahsMatjar::GhayrMuthabbat)
            .map(|natija| natija.matjar)
            .collect()
    }

    /// The launchers whose catalogue was read end to end — the only ones an
    /// absence sweep may act on. See [`HalatFahsMatjar::Tamma`].
    #[must_use]
    pub fn matajir_tamma(&self) -> Vec<&'static str> {
        self.matajir
            .iter()
            .filter(|natija| natija.hala() == HalatFahsMatjar::Tamma)
            .map(|natija| natija.matjar)
            .collect()
    }
}

#[cfg(test)]
mod ikhtibarat {
    use super::*;

    /// The three verdicts, from the two facts that produce them.
    #[test]
    fn al_hala_tushtaqq_min_al_tathbeet_wa_naw_al_tanbihat() {
        assert_eq!(NatijatMatjar::ghayr_mutah("epic").hala(), HalatFahsMatjar::GhayrMuthabbat);

        let mut natija = NatijatMatjar::muthabbat("epic", Some(PathBuf::from("/epic")));
        assert_eq!(natija.hala(), HalatFahsMatjar::Tamma);

        // One entry that would not parse leaves the verdict alone: everything
        // else in the catalogue was read, and a missing game is really missing.
        natija.tanbihat.push(TanbihFahs::jadeed("epic", "a.item", "not JSON"));
        assert_eq!(natija.hala(), HalatFahsMatjar::Tamma);

        // A source that could not be read does not.
        natija.tanbihat.push(TanbihFahs::fahras("epic", "Manifests", "not there"));
        assert_eq!(natija.hala(), HalatFahsMatjar::Naqisa);
    }

    /// The constructor for the shape that used to be reported as "not
    /// installed": the root is named, the warning is catalogue-level, and the
    /// verdict follows from it without the adapter setting anything else.
    #[test]
    fn naqisa_tahmil_al_jidhr_wa_tanbihan_yukhfi_al_alaab() {
        let natija = NatijatMatjar::naqisa(
            "epic",
            Some(PathBuf::from("/epic")),
            "/epic/Manifests",
            "no manifest directory",
        );
        assert_eq!(natija.jidhr_matjar.as_deref(), Some(Path::new("/epic")));
        assert_eq!(natija.hala(), HalatFahsMatjar::Naqisa);
        assert_eq!(natija.tanbihat.len(), 1);
        assert!(natija.tanbihat.iter().all(TanbihFahs::yukhfi_alaab));
        assert!(natija.alaab.is_empty());
    }

    /// Only a complete read is a launcher the sweep may believe.
    #[test]
    fn matajir_tamma_tastathni_al_naqisa_wa_ghayr_al_muthabbata() {
        let natija = NatijatFahs {
            matajir: vec![
                NatijatMatjar::muthabbat("steam", Some(PathBuf::from("/steam"))),
                NatijatMatjar::naqisa("epic", Some(PathBuf::from("/epic")), "Manifests", "gone"),
                NatijatMatjar::ghayr_mutah("gog"),
            ],
            muddat: Duration::ZERO,
        };
        assert_eq!(natija.matajir_tamma(), ["steam"]);
        assert_eq!(natija.matajir_mutaha(), ["steam", "epic"]);
    }
}
