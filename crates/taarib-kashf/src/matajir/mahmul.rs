//! محمول — portable, emulated, and user-nominated installations.
//!
//! Nine adapters read a launcher's own catalogue and one — [`super::yadawi`] —
//! reads back the paths the user pointed Taarib at. This one reads neither. It
//! is given a list of folders and asked what is in them, and it answers by
//! looking at the files.
//!
//! That is a different job from every other adapter in this crate, and it is the
//! one that decides whether Taarib covers the games nobody's launcher knows
//! about: a portable copy on an external drive, a DRM-free build extracted from
//! an archive, a game bought on a site with no client, an itch download the itch
//! app never installed, a folder somebody copied off an old machine. None of
//! those is in any catalogue anywhere, and all of them are perfectly ordinary.
//!
//! ## What it does, in order
//!
//! 1. Takes the scan roots: `IdadatManassat::mujalladat_idafiya` from the user's
//!    settings, plus anything the caller adds through
//!    [`MatjarMahmul::bi_judhur`].
//! 2. Walks each root to [`AQSA_UMQ`] levels with [`AQSA_MADAKHIL_LIL_JIDHR`] as
//!    a hard ceiling on entries, both bounded for the reasons written on the
//!    constants.
//! 3. **Signature-scans** every directory it reaches: does this folder hold an
//!    engine's own layout, or is it a folder of documents?
//! 4. Skips anything a real launcher already owns, so nominating a Steam library
//!    folder does not produce a second copy of every Steam game.
//! 5. Hands each admitted directory to [`super::yadawi::luba_min_masar`], which
//!    already knows how to pick the executable, clean the name, find the artwork
//!    and derive the identity — and marks the result
//!    [`SimatLuba::MuktashafaBilIstidlal`], because the user is entitled to know
//!    which of their games nothing vouched for.
//!
//! ## The signature scan is a pre-filter, not engine detection
//!
//! It looks the same and it is not. Phase 5 (`taarib-muharrik`) identifies an
//! engine on a game that has already been admitted to the library: it opens
//! files, reads headers, resolves conflicting evidence between detectors and
//! produces a report the user reads. This runs on **every directory under a
//! folder somebody nominated**, which may be a whole drive, and its entire job is
//! to answer one cheap question — is this a game at all — from a single
//! directory listing plus a handful of `is_file` probes.
//!
//! So Phase 5's detector is deliberately not imported here. Importing it would
//! mean opening files in every folder on a nominated drive to decide whether to
//! look at it, which is backwards, and it would make discovery depend on a crate
//! that depends on discovery. The two overlap in what they recognise and agree
//! on nothing else: this one is allowed to be wrong in the cheap direction
//! (admit a folder that turns out not to be a game, and Phase 5 will say so),
//! and Phase 5 is not allowed to be wrong at all.
//!
//! The evidence it accepts, all of it visible from one listing:
//!
//! | engine | what proves it |
//! | --- | --- |
//! | Unity | an executable with a sibling `<stem>_Data/` directory — the engine's own naming rule, not a guess |
//! | Unreal | an `Engine/` directory with `Engine/Binaries` inside it, or a `.pak` beside it |
//! | Godot | a `.pck` file, and the executable of the same stem when there is one |
//! | GameMaker | `data.win`, or `game.unx` on a Linux build |
//! | Ren'Py | a `.rpa` archive, or a `renpy/` directory |
//! | RPG Maker | `Game.exe` with `Data/` and an `.rvdata2`, a `Game.rgss3a` archive, or `www/js/rpg_core.js` and `js/rmmz_core.js` for MV and MZ |
//! | NW.js | `nw.exe`, which no non-NW.js game ships |
//! | Electron | `package.json` beside `resources/app.asar` |
//!
//! ## Emulated titles are listed, and they are not translatable
//!
//! A directory holding a cluster of ROM or disc images is reported as a game
//! with [`SimatLuba::MuhakatRum`] on it, and this is worth being blunt about:
//! **Taarib cannot patch a ROM.** A cartridge dump is one opaque image; its text
//! is inside a format the emulator never sees separately, in an encoding chosen
//! by a developer in 1994, and there is no supported path by which this product
//! rewrites one. These entries exist so that a user's library is their library —
//! so the grid shows what they actually have rather than the subset this
//! product can act on — and so the interface can say plainly why the Arabize
//! button is not there. Anything else would mean a user's emulated collection
//! silently vanishing from a product that claims to find their games.
//!
//! ## Identity, and why a moved folder is a new game
//!
//! `MasdarLuba::Yadawi(<hash>)`, from [`super::yadawi::muarrif_yadawi`] — the
//! same function, called rather than reimplemented, so a game found by this
//! scanner and the same game added by hand through Settings are one library row
//! and not two. For a ROM directory there is no executable, so the directory's
//! own canonical path is hashed by that same function: same rule, same
//! lowercase folding on the filesystems that need it, and no possibility of
//! collision with an executable's hash because a path is either a file or a
//! directory and never both.
//!
//! The consequence is the one [`super::yadawi`] already documents and it is
//! worth repeating here, because this adapter produces far more of these
//! identities than that one does: **moving the folder gives the game a new
//! identity**, and its patches and backups do not follow it. A launcher-managed
//! game keeps its identity across a move because its launcher vouches that it is
//! the same game. Nothing vouches for this one. The path is the only fact
//! Taarib has, so the path is the identity.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatFahs, NatijatMatjar, SimatLuba,
    SiyaqFahs, TanbihFahs,
};
use crate::matajir::yadawi;

/// The stable identifier.
///
/// Its games shard under `yadawi`, not under this name: their identity is
/// `MasdarLuba::Yadawi`, and a patch published against a path hash is the same
/// patch whether the path was typed by a user or found by this scanner.
const MUARRIF: &str = "mahmul";

/// Scanning a folder works the same on every system.
const MANASSAT: [NizamTashghil; 3] =
    [NizamTashghil::Windows, NizamTashghil::Linux, NizamTashghil::Mac];

/// How far below a nominated root the walk goes.
///
/// Three levels, and the number comes from how people actually arrange games
/// rather than from a preference. One level is `Games/Hollow Knight`. Two is
/// `Games/GOG/Hollow Knight`, which is what an organised library looks like.
/// Three is `D:/Games/GOG/Adventure/Hollow Knight`, which is as deep as
/// hand-sorted libraries get. A fourth level would mean walking into the games
/// themselves — `Games/X/Engine/Binaries` — where every directory is a false
/// candidate and the cost per root multiplies.
pub const AQSA_UMQ: usize = 3;

/// How many filesystem entries one root's walk will look at.
///
/// Two hundred thousand, per root rather than shared. Per root because a shared
/// budget means the first folder in the list eats it and the other four are
/// silently never scanned — the user would see games from one of their five
/// folders and no indication why. Truncation is reported as a
/// [`TanbihFahs`] naming the folder, so a partial answer is never a silent one.
pub const AQSA_MADAKHIL_LIL_JIDHR: usize = 200_000;

/// How many directories one root will signature-test.
///
/// Four thousand. This is the bound that matters: the walk itself is cheap
/// because a directory entry costs no syscall, while every signature test costs
/// one `read_dir` and every *admitted* directory then costs a full executable
/// ranking walk inside [`super::yadawi`]. A nominated drive root with four
/// thousand folders under three levels is already an unusual machine.
pub const AQSA_MURASHAHAT_LIL_JIDHR: usize = 4_000;

/// How many entries one directory listing keeps.
///
/// The signature test needs the whole listing at once. A directory with two
/// hundred thousand files in it — a downloads folder, an extracted asset dump —
/// would otherwise be read into memory in full to answer a question the first
/// few hundred entries already settle.
const AQSA_MADAKHIL_MUJALLAD: usize = 4_000;

/// How many ROM or disc images make a directory a ROM directory.
const ADNA_RUM: usize = 3;

/// How many *generic* disc images make a directory a ROM directory on their own.
///
/// Higher than [`ADNA_RUM`] on purpose. A `.iso` is a disc image of anything —
/// a Linux installer, a backup, a driver disc — so one of them proves nothing
/// and four in one folder is a collection. A cartridge dump has no such
/// ambiguity, which is why two of those are enough.
const ADNA_RUM_AAM: usize = 4;

/// Directory names never worth descending into.
///
/// Not a blocklist of applications: three of these cannot contain a game by
/// construction, and `node_modules` is the one directory on a developer's
/// machine that reliably holds tens of thousands of files and no game at all.
const ASMAA_MATWIYA: [&str; 4] =
    ["node_modules", "$recycle.bin", "system volume information", "__macosx"];

/// Executable extensions the signature test will accept as a game's own binary.
///
/// An empty entry means "no extension", which is how a Linux game binary is
/// usually named.
const IMTIDADAT_TANFIDH: [&str; 5] = ["exe", "x86_64", "x86", "app", ""];

/// ROM extensions that are decisive on their own.
///
/// Each one names a console and is used by nothing else that lands in a games
/// folder. Deliberately missing: `.md`, because it is Markdown far more often
/// than Mega Drive; `.wad`, because it is Doom far more often than Wii; `.bin`
/// and `.vb`, because they are not extensions so much as the absence of one.
const AILAT_RUM: [(&str, &str); 29] = [
    ("nes", "Nintendo Entertainment System"),
    ("fds", "Famicom Disk System"),
    ("sfc", "Super Nintendo"),
    ("smc", "Super Nintendo"),
    ("gb", "Game Boy"),
    ("gbc", "Game Boy Color"),
    ("gba", "Game Boy Advance"),
    ("nds", "Nintendo DS"),
    ("3ds", "Nintendo 3DS"),
    ("cia", "Nintendo 3DS"),
    ("n64", "Nintendo 64"),
    ("z64", "Nintendo 64"),
    ("v64", "Nintendo 64"),
    ("gcm", "GameCube"),
    ("gcz", "GameCube"),
    ("rvz", "GameCube or Wii"),
    ("wbfs", "Wii"),
    ("nsp", "Nintendo Switch"),
    ("xci", "Nintendo Switch"),
    ("sms", "Sega Master System"),
    ("gg", "Sega Game Gear"),
    ("smd", "Sega Mega Drive"),
    ("32x", "Sega 32X"),
    ("pce", "PC Engine"),
    ("ws", "WonderSwan"),
    ("wsc", "WonderSwan Color"),
    ("a26", "Atari 2600"),
    ("lnx", "Atari Lynx"),
    // MAME's and RetroArch's compressed disc format. Decisive rather than
    // generic: nothing outside emulation writes a `.chd`.
    ("chd", "compressed disc image"),
];

/// Disc-image extensions that mean something only in a cluster.
const IMTIDADAT_RUM_AAMA: [&str; 6] = ["iso", "cue", "cso", "img", "nrg", "mdf"];

/// What a family of generic disc images is called when nothing narrows it.
const AILA_AAMA: &str = "disc images";

// ---------------------------------------------------------------------------
// the adapter
// ---------------------------------------------------------------------------

/// Portable, emulated and user-nominated installations.
#[derive(Debug, Clone, Default)]
pub struct MatjarMahmul {
    /// Extra roots the caller supplied, beyond the ones in the user's settings.
    judhur: Vec<PathBuf>,
    /// Directories a real launcher has already claimed.
    ///
    /// Set by the caller from a scan that has already run — see
    /// [`MatjarMahmul::mahjuza_min_fahs`]. Nothing in [`SiyaqFahs`] carries this,
    /// because a scan context is built before any adapter runs and this is a
    /// fact only produced by running them.
    masarat_mahjuza: Vec<PathBuf>,
}

impl MatjarMahmul {
    /// Builds the adapter with no extra roots and nothing claimed.
    ///
    /// It still scans, because `IdadatManassat::mujalladat_idafiya` reaches it
    /// through [`SiyaqFahs`]. A default-constructed adapter on a machine with no
    /// extra folders configured does no work at all.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { judhur: Vec::new(), masarat_mahjuza: Vec::new() }
    }

    /// Builds the adapter over extra scan roots.
    ///
    /// The caller supplies these from settings. They are *added* to
    /// `mujalladat_idafiya` rather than replacing it, so a caller that passes
    /// nothing still scans what the user configured.
    #[must_use]
    pub const fn bi_judhur(judhur: Vec<PathBuf>) -> Self {
        Self { judhur, masarat_mahjuza: Vec::new() }
    }

    /// Marks directories as already claimed by a launcher.
    ///
    /// A candidate under any of these is skipped outright. This is the
    /// mechanism that stops a user who nominated `D:\SteamLibrary` from getting
    /// a second, path-identified copy of every Steam game they own — one that
    /// would carry no store identity, match no patch published against an app
    /// id, and sit beside the real entry in the grid.
    #[must_use]
    pub fn maa_mahjuza(mut self, masarat: Vec<PathBuf>) -> Self {
        self.masarat_mahjuza.extend(masarat);
        self
    }

    /// Claims every install root a completed scan found.
    ///
    /// The intended wiring: run [`crate::Kashif`] over the launcher adapters,
    /// then build this one from the result and run it second through
    /// [`crate::Kashif::min_matajir`]. That ordering is what
    /// [`crate::matajir::kul`] already documents for the manual adapter, and it
    /// is what makes the deduplication exact rather than heuristic.
    #[must_use]
    pub fn mahjuza_min_fahs(self, natija: &NatijatFahs) -> Self {
        let masarat: Vec<PathBuf> =
            natija.alaab().into_iter().map(|luba| luba.jidhr.clone()).collect();
        self.maa_mahjuza(masarat)
    }

    /// The extra roots this adapter was built with.
    #[must_use]
    pub fn judhur_mudafa(&self) -> &[PathBuf] {
        &self.judhur
    }

    /// The directories this adapter has been told to leave alone.
    #[must_use]
    pub fn masarat_mahjuza(&self) -> &[PathBuf] {
        &self.masarat_mahjuza
    }

    /// Every root to scan, deduplicated, in a stable order.
    ///
    /// Settings first, then the caller's, because the settings list is the one
    /// the user can see and correct.
    fn judhur_fahs(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur: Vec<PathBuf> = Vec::new();
        let mut ruit: BTreeSet<String> = BTreeSet::new();
        for masar in siyaq.manassat.mujalladat_idafiya.iter().chain(self.judhur.iter()) {
            if ruit.insert(miftah_masar(masar, siyaq.nizam)) {
                judhur.push(masar.clone());
            }
        }
        judhur
    }

    /// The claimed directories as comparison keys.
    ///
    /// Case-folded on the filesystems that are, because a launcher writes
    /// `C:\Program Files (x86)\Steam` and a user types `c:\program files
    /// (x86)\steam`, and on Windows those are one directory.
    fn mafatih_mahjuza(&self, siyaq: &SiyaqFahs) -> BTreeSet<String> {
        let mut mafatih: BTreeSet<String> = self
            .masarat_mahjuza
            .iter()
            .map(|masar| miftah_masar(masar, siyaq.nizam))
            .collect();

        // A launcher root the user configured is claimed too. They told Taarib
        // where that launcher lives; the launcher's own adapter reads it, and
        // this one has no business reporting its games a second time.
        let idadat = &siyaq.manassat;
        let muhaddada = [
            idadat.steam.as_ref(),
            idadat.epic.as_ref(),
            idadat.gog.as_ref(),
            idadat.ea.as_ref(),
            idadat.ubisoft.as_ref(),
            idadat.battlenet.as_ref(),
            idadat.xbox.as_ref(),
            idadat.itch.as_ref(),
            idadat.heroic.as_ref(),
        ];
        for masar in muhaddada.into_iter().flatten() {
            let _ = mafatih.insert(miftah_masar(masar, siyaq.nizam));
        }
        mafatih
    }
}

impl Matjar for MatjarMahmul {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "مجلّدات مفحوصة"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Scanned folders"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    /// There is no launcher to locate.
    ///
    /// The first scan root that exists stands in as a root so the Diagnostics
    /// screen has somewhere to point. [`None`] means the user has nominated
    /// nothing, which is the default and is not a failure.
    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        self.judhur_fahs(siyaq).into_iter().find(|masar| masar.is_dir())
    }

    /// # Errors
    ///
    /// Never. A nominated folder that has gone away, a folder that cannot be
    /// listed, a candidate whose executable could not be chosen and a walk that
    /// hit its ceiling are each a [`TanbihFahs`]: every one of them is
    /// information the user needs about a folder they chose, and none of them is
    /// a reason to withhold the games from the folders that worked.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: self.mawqi(siyaq),
            ..NatijatMatjar::default()
        };

        let judhur = self.judhur_fahs(siyaq);
        if judhur.is_empty() {
            natija.muddat = bidaya.elapsed();
            return Ok(natija);
        }

        let mahjuza = self.mafatih_mahjuza(siyaq);
        let mut maruf: BTreeSet<String> = BTreeSet::new();

        for jidhr in judhur {
            if !jidhr.is_dir() {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    jidhr.display().to_string(),
                    "this scan folder from Settings is not there. Correct it or remove it, so it \
                     stops costing a scan every time the library refreshes.",
                ));
                continue;
            }
            imsah_jidhr(&jidhr, siyaq.nizam, &mahjuza, &mut maruf, &mut natija);
        }

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur: Vec<PathBuf> =
            self.judhur_fahs(siyaq).into_iter().filter(|masar| masar.is_dir()).collect();
        judhur.sort();
        judhur.dedup();
        judhur
    }
}

/// The comparison key for a directory.
///
/// Case-folded where the filesystem is, and with a trailing separator removed,
/// so `D:\Games\` and `d:/games` are recognised as one place. Not canonicalized:
/// a root on a drive that is currently unplugged still has to compare equal to
/// itself, and canonicalizing a missing path fails.
fn miftah_masar(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy();
    let maqsus = nass.trim_end_matches(['/', '\\']);
    if nizam.hassas_lil_ahruf() { maqsus.to_owned() } else { maqsus.to_lowercase() }
}

/// Whether a path sits at or under any of a set of claimed directories.
///
/// Walks the path's own ancestors rather than comparing against every claimed
/// entry, so the cost is the path's depth times a set lookup instead of the
/// number of claimed roots.
fn taht_ay(mafatih: &BTreeSet<String>, masar: &Path, nizam: NizamTashghil) -> bool {
    masar.ancestors().any(|jadd| mafatih.contains(&miftah_masar(jadd, nizam)))
}

// ---------------------------------------------------------------------------
// the walk
// ---------------------------------------------------------------------------

/// Walks one nominated root and appends everything it admits.
///
/// `maruf` carries the identities this scan has already produced, so a game
/// reached through two overlapping nominated folders — `D:\Games` and
/// `D:\Games\GOG` both configured — becomes one library row rather than two.
///
/// The root itself is tested before the walk starts. A user who nominates the
/// game's own folder rather than the folder above it is doing something
/// reasonable, and a scan that only ever looked at children would find nothing
/// and say nothing.
///
/// Directories under an admitted game are skipped rather than pruned. Pruning
/// would mean holding the admitted set inside the walker's own filter closure,
/// which trades a handful of wasted `read_dir` calls — a Unity game costs about
/// three — for a shared mutable borrow that can fail at runtime. The syscalls
/// are cheaper than the failure mode.
fn imsah_jidhr(
    jidhr: &Path,
    nizam: NizamTashghil,
    mahjuza: &BTreeSet<String>,
    maruf: &mut BTreeSet<String>,
    natija: &mut NatijatMatjar,
) {
    let mut maqbula: BTreeSet<String> = BTreeSet::new();
    let mut adad: usize = 0;
    let mut murashahat: usize = 0;
    let mut mabtur = false;
    let mut mutakhkham = false;

    // The root itself first. It is not added to `maqbula`: a user who nominated
    // one game's folder gets that game, and a user who nominated a library
    // folder that happens to hold a stray `.pck` still gets the games inside it.
    if let Some(hukm) = sannif_mujallad(jidhr) {
        adif(jidhr, hukm, nizam, maruf, natija);
    }

    let mashi = walkdir::WalkDir::new(jidhr)
        .min_depth(1)
        .max_depth(AQSA_UMQ)
        .follow_links(false)
        .sort_by_file_name();

    for madkhal in mashi.into_iter().filter_map(Result::ok) {
        adad = adad.saturating_add(1);
        if adad > AQSA_MADAKHIL_LIL_JIDHR {
            mabtur = true;
            break;
        }
        if !madkhal.file_type().is_dir() {
            continue;
        }

        let masar = madkhal.path();
        if matwi(masar) || taht_ay(&maqbula, masar, nizam) || taht_ay(mahjuza, masar, nizam) {
            continue;
        }

        murashahat = murashahat.saturating_add(1);
        if murashahat > AQSA_MURASHAHAT_LIL_JIDHR {
            mutakhkham = true;
            break;
        }

        let Some(hukm) = sannif_mujallad(masar) else {
            continue;
        };
        let _ = maqbula.insert(miftah_masar(masar, nizam));
        adif(masar, hukm, nizam, maruf, natija);
    }

    if mabtur {
        natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "this folder holds more than {AQSA_MADAKHIL_LIL_JIDHR} files and folders within \
                 {AQSA_UMQ} levels, so the scan stopped there and what it found is partial. \
                 Nominate the folder that holds your games rather than the drive above it."
            ),
        ));
    }
    if mutakhkham {
        natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "this folder holds more than {AQSA_MURASHAHAT_LIL_JIDHR} candidate folders within \
                 {AQSA_UMQ} levels, so the scan stopped examining them and what it found is \
                 partial. Nominate a narrower folder."
            ),
        ));
    }
}

/// Whether a directory is one the walk never descends into.
fn matwi(masar: &Path) -> bool {
    let Some(ism) = masar.file_name().map(|ism| ism.to_string_lossy().to_lowercase()) else {
        return true;
    };
    // A dot-directory is configuration, version control or a launcher's own
    // bookkeeping. None of the three is a game, and `.git` alone can hold more
    // entries than the budget.
    ism.starts_with('.') || ASMAA_MATWIYA.contains(&ism.as_str())
}

/// Turns one admitted directory into a game and appends it.
///
/// A duplicate identity is dropped silently rather than warned about: two
/// nominated folders overlapping is a configuration a user is entitled to have,
/// and it is not a fault worth a line in Diagnostics.
fn adif(
    mujallad: &Path,
    hukm: HukmMujallad,
    nizam: NizamTashghil,
    maruf: &mut BTreeSet<String>,
    natija: &mut NatijatMatjar,
) {
    let luba = match hukm {
        HukmMujallad::Luba { dalil, tanfidhi } => {
            match bina_luba(mujallad, &dalil, tanfidhi.as_deref(), nizam) {
                Ok(luba) => luba,
                Err(sabab) => {
                    natija.tanbihat.push(TanbihFahs::jadeed(
                        MUARRIF,
                        mujallad.display().to_string(),
                        format!(
                            "this folder carries a game's fingerprint ({dalil}) and {sabab} It \
                             is probably a half-extracted archive."
                        ),
                    ));
                    return;
                },
            }
        },
        HukmMujallad::Rum { aila, adad, hajm } => {
            bina_rum(mujallad, &aila, adad, hajm, nizam)
        },
    };

    if maruf.insert(luba.masdar.muarrif()) {
        natija.alaab.push(luba);
    }
}

/// Builds a discovered game from an admitted directory.
///
/// The whole of the work is [`super::yadawi::luba_min_masar`]: it ranks the
/// executables, cleans the folder name, totals the size, finds artwork sitting
/// beside the game and derives the identity. Calling it rather than repeating it
/// is what makes a game found here and the same game added by hand through
/// Settings resolve to one identity.
///
/// When the signature proved *which* executable it is — Unity names its data
/// folder after the player binary, Godot names its pack after the exporter's
/// output — that file is handed over directly rather than ranked, which is both
/// exact and the only way a Windows executable is chosen on a Linux host: the
/// ranking will not consider a `.exe` there, and the proof does not need it to.
///
/// # Errors
///
/// Returns the sentence for the warning when nothing under the directory scores
/// high enough to be the game — a folder that held an engine's fingerprint and
/// nothing runnable, which happens with a half-extracted archive.
fn bina_luba(
    mujallad: &Path,
    dalil: &str,
    tanfidhi: Option<&Path>,
    nizam: NizamTashghil,
) -> Result<LubaMuktashafa, String> {
    let hadaf = tanfidhi.unwrap_or(mujallad);
    let mut luba =
        yadawi::luba_min_masar(hadaf, None, nizam).map_err(|khata| khata.injilizi)?;
    luba.simat.push(SimatLuba::MuktashafaBilIstidlal(dalil.to_owned()));
    Ok(luba)
}

/// Builds a discovered entry from a directory of ROM or disc images.
///
/// Assembled here rather than through [`super::yadawi::luba_min_masar`] because
/// that function's whole purpose is to choose an executable and there is none:
/// an emulated title has images and an emulator, and the emulator is somewhere
/// else on the machine entirely. What is shared is the identity —
/// [`super::yadawi::muarrif_yadawi`] over the directory's canonical path — the
/// name cleaning, and the artwork names, all called rather than repeated.
fn bina_rum(
    mujallad: &Path,
    aila: &str,
    adad: usize,
    hajm: u64,
    nizam: NizamTashghil,
) -> LubaMuktashafa {
    let ism = yadawi::ism_min_mujallad(mujallad)
        .unwrap_or_else(|| "Emulated titles".to_owned());

    LubaMuktashafa {
        masdar: MasdarLuba::Yadawi(yadawi::muarrif_yadawi(mujallad, nizam)),
        hala_matjar: None,
        ism,
        jidhr: mujallad.to_path_buf(),
        // Deliberately none. There is nothing here to launch and nothing here to
        // patch, and offering a `.iso` as an executable would send Phase 5 to
        // open a disc image as though it were a program.
        tanfidhi: None,
        hajm,
        bina_manassa: None,
        akhir_tahdith: yadawi::waqt_tadeel(mujallad),
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: ghilaf_mujawir(mujallad),
        khiyarat_tashghil: None,
        muktamila: true,
        simat: vec![
            SimatLuba::MuktashafaBilIstidlal(format!(
                "{adad} ROM or disc images in one folder, and no engine layout"
            )),
            SimatLuba::MuhakatRum(aila.to_owned()),
        ],
    }
}

/// Cover art sitting beside a ROM directory, when there is any.
///
/// The same five names [`super::yadawi`] looks for, from that module's own
/// constant, because a folder of ROMs gets a `folder.jpg` for exactly the same
/// reason a folder of DRM-free games does.
fn ghilaf_mujawir(mujallad: &Path) -> MasadirSuwar {
    let ghilaf = yadawi::ASMAA_SUWAR
        .iter()
        .map(|ism| mujallad.join(ism))
        .find(|masar| masar.is_file())
        .map(MasdarSura::Malaf);
    MasadirSuwar { ghilaf, batl: None, shiar: None }
}

// ---------------------------------------------------------------------------
// the signature scan
// ---------------------------------------------------------------------------

/// What a directory's own contents say it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HukmMujallad {
    /// A game.
    Luba {
        /// The evidence that admitted it, phrased for a person: the engine and
        /// the files that proved it. A heuristic that cannot say why it fired is
        /// a heuristic nobody can correct.
        dalil: String,
        /// The executable an engine's own naming rule identified, when that rule
        /// places it at the install root. [`None`] means nothing proved which
        /// file it is and the ranking in [`super::yadawi`] must choose.
        tanfidhi: Option<PathBuf>,
    },
    /// A directory of ROM or disc images for an emulator.
    Rum {
        /// The console family, or `disc images` when nothing narrows it.
        aila: String,
        /// How many images were counted.
        adad: usize,
        /// Their total size in bytes.
        hajm: u64,
    },
}

/// Classifies one directory from a single listing.
///
/// Public because the interface asks the same question the scanner does: when a
/// user picks a folder in the add-game dialog, this is what says "that looks
/// like a Unity game" or "that is a folder of Game Boy Advance ROMs" before
/// anything is committed.
///
/// Returns [`None`] for a folder that shows no engine layout and no ROM cluster,
/// and for one a launcher already owns — an Epic install carries `.egstore`, a
/// GOG install carries `goggame-<id>.info`, a Steam game sits under
/// `steamapps/common`, and every one of those is a directory whose own adapter
/// reports it with a real store identity. Reporting it here as well would put a
/// second, path-identified card beside the real one in the user's grid.
#[must_use]
pub fn sannif_mujallad(mujallad: &Path) -> Option<HukmMujallad> {
    let qaima = QaimatMujallad::iqra(mujallad)?;
    if taht_matjar(mujallad, &qaima).is_some() {
        return None;
    }
    if let Some((dalil, tanfidhi)) = dalil_muharrik(mujallad, &qaima) {
        return Some(HukmMujallad::Luba { dalil, tanfidhi });
    }
    dalil_rum(&qaima).map(|(aila, adad, hajm)| HukmMujallad::Rum { aila, adad, hajm })
}

/// Which launcher already owns this directory, when one does.
///
/// Every test here is a **marker the owning launcher writes into the game's own
/// folder**, or the launcher's own directory layout — never a folder name that
/// merely sounds like a launcher. `Epic Games` as a folder name proves nothing;
/// a `.egstore` directory inside a game is Epic's manifest store and is there
/// because Epic put it there.
fn taht_matjar(mujallad: &Path, qaima: &QaimatMujallad) -> Option<&'static str> {
    if qaima.mujallad(".egstore") {
        return Some("Epic Games");
    }
    if qaima.mujallad(".itch") {
        return Some("itch.io");
    }
    if qaima.malaf("uplay_install.state") {
        return Some("Ubisoft Connect");
    }
    if qaima
        .madakhil
        .iter()
        .any(|madkhal| !madkhal.huwa_mujallad && madkhal.saghir.starts_with("goggame-"))
    {
        return Some("GOG");
    }

    // Steam's own layout: `<library>/steamapps/common/<game>`. Two levels, both
    // named by Steam, and no other program arranges a directory that way.
    let fi_steam = mujallad
        .parent()
        .filter(|walid| walid.file_name().is_some_and(|ism| ism.eq_ignore_ascii_case("common")))
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .is_some_and(|ism| ism.eq_ignore_ascii_case("steamapps"));
    if fi_steam {
        return Some("Steam");
    }

    // A Microsoft Store game lives under `WindowsApps`, where its files are not
    // even readable without an ACL change — and where the Xbox adapter reads it
    // out of the package manifest instead.
    let fi_mutajar = mujallad
        .ancestors()
        .any(|jadd| jadd.file_name().is_some_and(|ism| ism.eq_ignore_ascii_case("WindowsApps")));
    fi_mutajar.then_some("Microsoft Store")
}

/// The engine fingerprint a directory carries, when it carries one.
///
/// Order is by how decisive the evidence is, not by how common the engine is.
/// Unity's `<stem>_Data` is the engine's own naming rule and cannot be a
/// coincidence; Electron's `app.asar` is shared with every non-game Electron
/// application on the machine, so it is tried last and only after RPG Maker MV
/// and NW.js, both of which ship the same two files and are games.
fn dalil_muharrik(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    dalil_unity(mujallad, qaima)
        .or_else(|| dalil_unreal(mujallad, qaima))
        .or_else(|| dalil_godot(mujallad, qaima))
        .or_else(|| dalil_gamemaker(qaima))
        .or_else(|| dalil_rpgmaker(mujallad, qaima))
        .or_else(|| dalil_renpy(mujallad, qaima))
        .or_else(|| dalil_nwjs(qaima))
        .or_else(|| dalil_electron(mujallad, qaima))
}

/// Unity: an executable with a sibling `<stem>_Data` directory.
///
/// The strongest signature in the table, and the only one that identifies the
/// executable as a side effect: Unity names the data folder after the player
/// binary, so `Hollow Knight_Data` beside `Hollow Knight.exe` is not evidence
/// that a game is here — it is the engine stating which file the game is.
///
/// `UnityPlayer` beside a binary is accepted as a weaker fallback, for the
/// builds where the pairing does not line up because somebody renamed the
/// executable after export.
fn dalil_unity(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    const JIRAN: [&str; 4] =
        ["unityplayer.dll", "unityplayer.so", "libunityplayer.so", "unityplayer.dylib"];

    for madkhal in qaima.malaffat() {
        if !IMTIDADAT_TANFIDH.contains(&madkhal.imtidad.as_str()) {
            continue;
        }
        let matlub = format!("{}_data", madkhal.jidhr_ism);
        if let Some(bayanat) = qaima.mujallad_madkhal(&matlub) {
            return Some((
                format!("Unity: {} beside {}", madkhal.ism, bayanat.ism),
                Some(mujallad.join(&madkhal.ism)),
            ));
        }
    }

    let jar = JIRAN.iter().find(|ism| qaima.malaf(ism))?;
    Some((format!("Unity: {jar} at the install root"), None))
}

/// Unreal: an `Engine` directory with the engine's own binaries inside it.
///
/// The brief's `*.pak` half is accepted too, and is the weaker of the two: a
/// packaged Unreal game keeps its `.pak` files under
/// `<Project>/Content/Paks`, several levels below the root, so at the root there
/// is usually no `.pak` at all and the `Engine/Binaries` directory carries the
/// test. The executable is left to the ranking, which knows that Unreal's
/// shipping binary lives at `<Project>/Binaries/Win64/<Project>-Win64-Shipping`
/// and scores it accordingly.
fn dalil_unreal(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    let muharrik = qaima.mujallad_madkhal("engine")?;
    let binaries = mujallad.join(&muharrik.ism).join("Binaries").is_dir();
    if binaries {
        return Some((format!("Unreal Engine: {}/Binaries", muharrik.ism), None));
    }
    let pak = qaima.awwal_bi_imtidad("pak")?;
    Some((format!("Unreal Engine: {} beside {}", pak.ism, muharrik.ism), None))
}

/// Godot: a `.pck` pack file.
///
/// The executable is the file of the same stem when the export produced one,
/// which is Godot's default layout — `Game.exe` beside `Game.pck`. A
/// single-file export embeds the pack in the binary and leaves no `.pck` at all,
/// which is why this test cannot be the only Godot evidence in the product; it
/// is enough here, because a `.pck` in a folder is never anything else.
fn dalil_godot(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    let pack = qaima.awwal_bi_imtidad("pck")?;
    let tanfidhi = qaima
        .tanfidhi_bi_jidhr_ism(&pack.jidhr_ism)
        .map(|madkhal| mujallad.join(&madkhal.ism));
    Some((format!("Godot: {}", pack.ism), tanfidhi))
}

/// GameMaker: the runner's data file, under whichever name the target uses.
///
/// `data.win` on Windows, `game.unx` on Linux, `game.ios` and `game.droid` on
/// the mobile exports that occasionally end up in a desktop folder. The
/// executable is left to the ranking: GameMaker names the runner after the
/// project, which is what the ranking's folder-name match already rewards.
fn dalil_gamemaker(qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    const ASMAA: [&str; 4] = ["data.win", "game.unx", "game.ios", "game.droid"];
    let ism = ASMAA.iter().find(|ism| qaima.malaf(ism))?;
    Some((format!("GameMaker: {ism}"), None))
}

/// RPG Maker, across the four generations that reach a desktop folder.
///
/// | generation | evidence |
/// | --- | --- |
/// | XP, VX, VX Ace | `Game.exe` with a `Data` directory holding an `.rvdata2`, `.rvdata` or `.rxdata` |
/// | XP, VX, VX Ace, encrypted | `Game.rgss3a`, `Game.rgss2a` or `Game.rgssad` at the root |
/// | MV | `www/js/rpg_core.js` |
/// | MZ | `js/rmmz_core.js` |
///
/// The `Data` directory is listed rather than assumed, which is one extra
/// `read_dir` — taken only when `Game.exe` and `Data` are both already there, so
/// it costs nothing on a folder that is not an RPG Maker game.
///
/// This runs before NW.js and Electron deliberately: MV and MZ are NW.js
/// applications and ship both of those engines' fingerprints, and "RPG Maker MV"
/// is a far more useful thing to tell a user than "Electron".
fn dalil_rpgmaker(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    const ARSHIFAT: [&str; 3] = ["rgss3a", "rgss2a", "rgssad"];
    const IMTIDADAT: [&str; 3] = ["rvdata2", "rvdata", "rxdata"];

    if let Some(arshif) =
        ARSHIFAT.iter().find_map(|imtidad| qaima.awwal_bi_imtidad(imtidad))
    {
        let tanfidhi = qaima.tanfidhi_bi_jidhr_ism("game").map(|m| mujallad.join(&m.ism));
        return Some((format!("RPG Maker: {}", arshif.ism), tanfidhi));
    }

    if let Some(www) = qaima.mujallad_madkhal("www")
        && mujallad.join(&www.ism).join("js").join("rpg_core.js").is_file()
    {
        return Some(("RPG Maker MV: www/js/rpg_core.js".to_owned(), None));
    }

    if let Some(js) = qaima.mujallad_madkhal("js")
        && mujallad.join(&js.ism).join("rmmz_core.js").is_file()
    {
        return Some(("RPG Maker MZ: js/rmmz_core.js".to_owned(), None));
    }

    let bayanat = qaima.mujallad_madkhal("data")?;
    let tanfidhi = qaima.tanfidhi_bi_jidhr_ism("game")?;
    let dakhili = imtidad_dakhil(&mujallad.join(&bayanat.ism), &IMTIDADAT)?;
    Some((
        format!("RPG Maker: {} with {}/{}", tanfidhi.ism, bayanat.ism, dakhili),
        Some(mujallad.join(&tanfidhi.ism)),
    ))
}

/// Ren'Py: the interpreter's own directory, or one of its archives.
///
/// A Ren'Py game's `.rpa` archives live under `game/`, not at the root, so the
/// root-level evidence is the `renpy/` directory the interpreter ships. Both are
/// accepted, and `game/` is listed only when `lib/` is beside it — the pair
/// Ren'Py always exports together — so the extra `read_dir` never happens on a
/// folder that is not one.
///
/// The executable is left to the ranking. Ren'Py names it after the project and
/// puts it at the root beside a `.sh` of the same name, which is exactly the
/// shape the folder-name match was written for.
fn dalil_renpy(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    if let Some(mufassir) = qaima.mujallad_madkhal("renpy") {
        return Some((format!("Ren'Py: a {} folder at the install root", mufassir.ism), None));
    }
    if let Some(arshif) = qaima.awwal_bi_imtidad("rpa") {
        return Some((format!("Ren'Py: {}", arshif.ism), None));
    }
    let luba = qaima.mujallad_madkhal("game")?;
    if !qaima.mujallad("lib") {
        return None;
    }
    let dakhili = imtidad_dakhil(&mujallad.join(&luba.ism), &["rpa"])?;
    Some((format!("Ren'Py: {}/{}", luba.ism, dakhili), None))
}

/// NW.js: the runtime's own executable.
///
/// `nw.exe` is the NW.js binary under its export name, and nothing that is not
/// an NW.js application ships it. A game that renamed it — which most do — is
/// caught by [`dalil_electron`] instead, because the `package.json` and
/// `app.asar` pair survives the rename.
fn dalil_nwjs(qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    const ASMAA: [&str; 3] = ["nw.exe", "nw.pak", "package.nw"];
    let ism = ASMAA.iter().find(|ism| qaima.malaf(ism))?;
    Some((format!("NW.js: {ism}"), None))
}

/// Electron: `package.json` beside `resources/app.asar`.
///
/// The weakest signature in the table and last for that reason: every Electron
/// application on the machine carries it, games and chat clients alike. It earns
/// its place because a large number of small commercial games are Electron
/// applications and nothing else would find them — and because a false positive
/// here costs a folder in the library that Phase 5 then reports honestly, while
/// a false negative costs the user a game.
fn dalil_electron(mujallad: &Path, qaima: &QaimatMujallad) -> Option<(String, Option<PathBuf>)> {
    if !qaima.malaf("package.json") {
        return None;
    }
    let mawarid = qaima.mujallad_madkhal("resources")?;
    let asar = mujallad.join(&mawarid.ism).join("app.asar");
    asar.is_file().then(|| (format!("Electron: {}/app.asar", mawarid.ism), None))
}

/// The first file inside a directory with one of the given extensions.
///
/// One listing, stopped as soon as it matches. Used only by the two signatures
/// whose decisive file is one level below the install root, and gated behind
/// evidence that the directory is worth opening at all.
fn imtidad_dakhil(mujallad: &Path, imtidadat: &[&str]) -> Option<String> {
    let qaima = std::fs::read_dir(mujallad).ok()?;
    for madkhal in qaima.flatten().take(AQSA_MADAKHIL_MUJALLAD) {
        let masar = madkhal.path();
        let Some(imtidad) =
            masar.extension().map(|imtidad| imtidad.to_string_lossy().to_lowercase())
        else {
            continue;
        };
        if imtidadat.contains(&imtidad.as_str()) {
            return masar.file_name().map(|ism| ism.to_string_lossy().into_owned());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// emulated titles
// ---------------------------------------------------------------------------

/// Whether a directory is a cluster of ROM or disc images, and of what.
///
/// Three ways to qualify, and the asymmetry between them is the whole design:
///
/// - **Two decisive images.** A `.gba` is a Game Boy Advance cartridge dump and
///   nothing else; two of them in one folder is a ROM set with no other
///   explanation.
/// - **One decisive image among three.** A folder holding a `.nes`, a `.cue` and
///   an `.iso` is somebody's emulation folder.
/// - **Four generic images.** A `.iso` on its own is a disc image of anything —
///   a Linux installer, a backup, a driver disc — so it takes a collection
///   before the folder means something.
///
/// Returns the family, the count and the total bytes. The family is the most
/// common decisive extension's console, resolved to `disc images` when only
/// generic images were found, and ties break alphabetically so that the same
/// folder always produces the same answer.
fn dalil_rum(qaima: &QaimatMujallad) -> Option<(String, usize, u64)> {
    let mut ailat: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut qati: usize = 0;
    let mut aam: usize = 0;
    let mut hajm: u64 = 0;

    for madkhal in qaima.malaffat() {
        let mutabiq = AILAT_RUM
            .iter()
            .find(|(imtidad, _)| *imtidad == madkhal.imtidad.as_str())
            .map(|(_, aila)| *aila);

        if let Some(aila) = mutabiq {
            qati = qati.saturating_add(1);
            hajm = hajm.saturating_add(madkhal.hajm);
            let adad = ailat.entry(aila).or_insert(0);
            *adad = adad.saturating_add(1);
        } else if IMTIDADAT_RUM_AAMA.contains(&madkhal.imtidad.as_str()) {
            aam = aam.saturating_add(1);
            hajm = hajm.saturating_add(madkhal.hajm);
        }
    }

    let majmu = qati.saturating_add(aam);
    let maqbul = qati >= 2 || (qati >= 1 && majmu >= ADNA_RUM) || aam >= ADNA_RUM_AAM;
    if !maqbul {
        return None;
    }

    // Highest count wins. The map iterates alphabetically and the comparison is
    // strict, so a folder holding four Game Boy dumps and four Mega Drive dumps
    // reports the alphabetically first of the two — every time, on every
    // machine, which is the only property that matters here.
    let mut afdal: Option<(&'static str, usize)> = None;
    for (aila, adad) in &ailat {
        if afdal.is_none_or(|(_, akbar)| *adad > akbar) {
            afdal = Some((*aila, *adad));
        }
    }

    let aila = afdal.map_or(AILA_AAMA, |(aila, _)| aila);
    Some((aila.to_owned(), majmu, hajm))
}

// ---------------------------------------------------------------------------
// one directory listing
// ---------------------------------------------------------------------------

/// One entry inside a candidate directory, reduced to what the tests ask about.
///
/// The original name is kept alongside the lowercased one because every test
/// compares in lowercase and every message quotes what the filesystem actually
/// spells — a user reading `Unity: HollowKnight.exe beside HollowKnight_Data`
/// can go and look at those two files, and one reading the lowercased form
/// cannot find them on a case-sensitive filesystem.
#[derive(Debug, Clone)]
struct MadkhalMujallad {
    /// The name as the filesystem spells it.
    ism: String,
    /// The same name, lowercased.
    saghir: String,
    /// The lowercased stem.
    jidhr_ism: String,
    /// The lowercased extension, empty when there is none.
    imtidad: String,
    /// Size in bytes, for the entries whose size is asked about.
    hajm: u64,
    /// Whether it is a directory, following a symlink to decide.
    huwa_mujallad: bool,
}

/// One directory's listing, read once.
///
/// The signature test needs the whole listing at once — every one of its
/// questions is about what sits *beside* what — and a streaming walk delivers a
/// directory's entries interleaved with its subtrees. So the walk enumerates
/// candidate directories and this lists each one on its own, which is one
/// `read_dir` per candidate and no buffering of the walk.
#[derive(Debug, Clone, Default)]
struct QaimatMujallad {
    /// Every entry, sorted by lowercased name so that two scans of one folder
    /// can never disagree about which of two matching files came first.
    madakhil: Vec<MadkhalMujallad>,
}

impl QaimatMujallad {
    /// Lists a directory, or nothing when it cannot be read.
    ///
    /// Sizes are collected only for the extensions the ROM test asks about.
    /// A `metadata` call is a syscall per entry, and this runs on every
    /// directory under every nominated root — paying it for the `.dll` files
    /// inside a Unity game would double the cost of the whole scan to answer a
    /// question nothing asks.
    fn iqra(mujallad: &Path) -> Option<Self> {
        let qaima = std::fs::read_dir(mujallad).ok()?;
        let mut madakhil: Vec<MadkhalMujallad> = Vec::new();

        for madkhal in qaima.flatten().take(AQSA_MADAKHIL_MUJALLAD) {
            let Ok(naw) = madkhal.file_type() else {
                continue;
            };
            let ism = madkhal.file_name().to_string_lossy().into_owned();
            let saghir = ism.to_lowercase();
            let (jidhr_ism, imtidad) = match saghir.rsplit_once('.') {
                Some((jidhr, imtidad)) if !jidhr.is_empty() => {
                    (jidhr.to_owned(), imtidad.to_owned())
                },
                _ => (saghir.clone(), String::new()),
            };

            // A symlink reports as neither file nor directory, and a launcher
            // that symlinks a shared runtime into a game's folder would
            // otherwise make that folder look empty. Following it here is safe:
            // nothing recurses through this listing.
            let huwa_mujallad = naw.is_dir() || (naw.is_symlink() && madkhal.path().is_dir());

            let yuhimm_hajmuh = !huwa_mujallad && huwa_imtidad_rum(&imtidad);
            let hajm = if yuhimm_hajmuh {
                madkhal.metadata().map_or(0, |bayanat| bayanat.len())
            } else {
                0
            };

            madakhil.push(MadkhalMujallad {
                ism,
                saghir,
                jidhr_ism,
                imtidad,
                hajm,
                huwa_mujallad,
            });
        }

        madakhil.sort_by(|awwal, thani| awwal.saghir.cmp(&thani.saghir));
        Some(Self { madakhil })
    }

    /// Every file, in name order.
    fn malaffat(&self) -> impl Iterator<Item = &MadkhalMujallad> {
        self.madakhil.iter().filter(|madkhal| !madkhal.huwa_mujallad)
    }

    /// Whether a file of this lowercased name is here.
    fn malaf(&self, ism: &str) -> bool {
        self.madakhil.iter().any(|madkhal| !madkhal.huwa_mujallad && madkhal.saghir == ism)
    }

    /// Whether a directory of this lowercased name is here.
    fn mujallad(&self, ism: &str) -> bool {
        self.mujallad_madkhal(ism).is_some()
    }

    /// The directory entry of this lowercased name, for the messages that quote
    /// the name the filesystem actually spells.
    fn mujallad_madkhal(&self, ism: &str) -> Option<&MadkhalMujallad> {
        self.madakhil.iter().find(|madkhal| madkhal.huwa_mujallad && madkhal.saghir == ism)
    }

    /// The first file with this lowercased extension.
    fn awwal_bi_imtidad(&self, imtidad: &str) -> Option<&MadkhalMujallad> {
        self.madakhil
            .iter()
            .find(|madkhal| !madkhal.huwa_mujallad && madkhal.imtidad == imtidad)
    }

    /// The executable of this lowercased stem, when one is here.
    ///
    /// Every executable extension is accepted regardless of which system the
    /// scan is running on, and that is deliberate: a Windows game sitting on a
    /// Linux drive is exactly the case this adapter exists to find, and the
    /// proof of which file it is comes from the engine's naming rule rather than
    /// from the host's idea of what is runnable.
    fn tanfidhi_bi_jidhr_ism(&self, jidhr_ism: &str) -> Option<&MadkhalMujallad> {
        self.madakhil.iter().find(|madkhal| {
            !madkhal.huwa_mujallad
                && madkhal.jidhr_ism == jidhr_ism
                && IMTIDADAT_TANFIDH.contains(&madkhal.imtidad.as_str())
        })
    }
}

/// Whether an extension is one the ROM test counts.
fn huwa_imtidad_rum(imtidad: &str) -> bool {
    IMTIDADAT_RUM_AAMA.contains(&imtidad)
        || AILAT_RUM.iter().any(|(mawjud, _)| *mawjud == imtidad)
}
