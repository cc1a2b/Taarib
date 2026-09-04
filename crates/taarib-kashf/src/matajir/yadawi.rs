//! يدوي — games the user added by hand, and how the right executable is chosen.
//!
//! This is not a scanner. There is no launcher to find, no catalogue to parse
//! and no directory to enumerate: it reads back the paths the user has already
//! pointed Taarib at, and turns each one into a [`LubaMuktashafa`] that is
//! identical in every respect to a discovered one. Phase 5 probes it the same
//! way, Phase 15 installs into it the same way, and the library screen shows it
//! in the same grid. Nothing downstream treats a manually added game as second
//! class, because nothing downstream can tell.
//!
//! It is also the floor under the whole of discovery. There are more PC game
//! launchers than the nine this crate reads, there are DRM-free games that were
//! never in a launcher, and there are games somebody copied off an old drive.
//! Every one of those lands here, so the quality of this module's guessing is
//! the quality of Taarib's coverage of everything it does not otherwise know.
//!
//! ## The surface it needs, and why it is this small
//!
//! Two inputs, no state of its own:
//!
//! 1. **`IdadatManassat::mujalladat_idafiya`** — the extra folders in the
//!    user's settings. Each one is treated as a *container of games*: every
//!    immediate subdirectory is a candidate game, which is what makes a folder
//!    of DRM-free installs work with one setting rather than one per game.
//! 2. **An explicit list the store keeps** — [`MudkhalYadawi`] records, supplied
//!    through the [`SijillYadawi`] trait. Each names one path the user added
//!    deliberately, optionally with a name they typed and a flag for entries
//!    they removed from the library without deleting the record.
//!
//! The trait exists so this module never touches the database. `taarib-makhzan`
//! owns the rows; the adapter is handed a reader for them and nothing else, and
//! a caller with no store at all can pass [`SijillThabit`] with a literal list.
//!
//! ## What counts as a candidate, and why the host does not decide it
//!
//! A directory's contents say what it holds. The machine reading the directory
//! does not, and must not: a Windows game sitting on a Linux filesystem is an
//! ordinary situation — Proton, a shared drive, a dual boot — so a `.exe` is a
//! candidate on every host, and `hollow_knight.exe` beside a
//! `hollow_knight_Data` folder is a Unity game whoever is looking at it.
//!
//! The execute bit is the signal that could not be left to stand alone. It is
//! real on a filesystem that has permissions of its own and meaningless on one
//! that does not: an NTFS or exFAT volume mounted on Linux reports every file
//! as executable, and that is how a Unity scene file —
//! `hollow_knight_Data/level470`, thirteen megabytes, no extension, execute bit
//! set — came to outrank the game. A file whose extension settles nothing is
//! admitted only when its first bytes are a program's: ELF, Mach-O, a shebang,
//! or the `MZ` a PE opens with. A mount can forge the bit. It cannot forge the
//! header.
//!
//! The host is kept as a preference rather than a gate. It adds to a build
//! native to it and never subtracts from a foreign one, so a game shipping a
//! Linux build and a Windows build in one folder resolves to the one the user
//! would actually launch, while a game shipping only the foreign one is still
//! found.
//!
//! ## Choosing the executable, in full
//!
//! Given a path the user chose:
//!
//! - If it is a file, that file is the executable and its **directory** is the
//!   install root — never the file's own path, because a patch installs beside
//!   the game, not into it.
//! - If it is a directory, the tree below it is searched and every candidate is
//!   scored. The ranking, in descending order of how much it is worth:
//!
//! | signal | weight | why |
//! | --- | ---: | --- |
//! | the executable's name matches the folder's name | +1000 exact, +400 partial | `Hollow Knight/Hollow Knight.exe` is not a coincidence; it is what every publisher does |
//! | a sibling `<name>_Data` folder, or `UnityPlayer`/`GameAssembly` beside it | +800 / +500 | decisive engine evidence, and the same file Phase 5 will want |
//! | sitting in an Unreal `Binaries/<Platform>/` path | +600 | Unreal's shipping binary is never at the tree root |
//! | built for the system doing the scanning | +200 | between two builds of one game the host is the tie-break; it is never a reason to prefer a tool over the game |
//! | shallower in the tree | −120 per level | the game is at the root or one level down; tools are buried |
//! | larger | +1 per megabyte, capped at 512 | the game binary is nearly always the biggest executable present |
//! | named like an installer, uninstaller, crash handler or redistributable | −5000 | `unins000.exe`, `vcredist_x64.exe` and `UnityCrashHandler64.exe` ship *inside* games and are never the game |
//! | a bootstrap standing above a binary the engine vouches for | −1000 | Unreal's packaging shim is named after the game and wins the name test outright, which is precisely why its name cannot be trusted |
//! | named like a launcher, when a non-launcher candidate exists no deeper | −300 | conditional on purpose: plenty of games ship only `Launcher.exe`, and punishing it unconditionally would leave those games with nothing |
//!
//! Ties break on the shorter path and then alphabetically, so the same folder
//! always yields the same answer. When the best candidate still scores below
//! the acceptance floor — a folder holding nothing but an uninstaller and a
//! redistributable — the answer is [`KhataKashf::LaYujadTanfidhi`], because
//! offering to patch an uninstaller is worse than admitting nothing was found.
//!
//! ## Identity
//!
//! The source is `MasdarLuba::Yadawi(<hash>)`, where the hash is BLAKE3 over
//! the **canonical** path of the chosen executable, folded to lowercase on the
//! systems whose filesystems are case-insensitive. Two consequences, both
//! intended:
//!
//! - Adding the same game twice — by folder once and by executable once — gives
//!   one entry, because both resolve to the same canonical executable.
//! - Moving the game gives it a **new** identity, and its patches, projects and
//!   backups do not follow it. That is correct rather than unfortunate: a
//!   launcher-managed game keeps its identity across a move because its
//!   launcher vouches that it is the same game. Nobody vouches for this one. A
//!   path is the only fact Taarib has, so the path is the identity, and
//!   pretending otherwise would silently apply one game's patch to another.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use taarib_mustalahat::luba::MasdarLuba;
use taarib_mustalahat::wahhid_ism;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "yadawi";

/// Manual addition works everywhere, by construction.
const MANASSAT: [NizamTashghil; 3] =
    [NizamTashghil::Windows, NizamTashghil::Linux, NizamTashghil::Mac];

/// One entry the user added by hand, as the local store keeps it.
///
/// Persisted by `taarib-makhzan`, never by this module: discovery is read-only,
/// and that includes its own records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MudkhalYadawi {
    /// What the user pointed at: an executable, or the folder holding one.
    pub masar: PathBuf,
    /// A name the user typed, which always wins over a derived one. Absent
    /// when they accepted whatever Taarib worked out.
    pub ism: Option<String>,
    /// The user removed this from their library without deleting the record,
    /// so it is skipped but not forgotten — re-adding the same path restores
    /// whatever was attached to it.
    pub mukhfi: bool,
}

impl MudkhalYadawi {
    /// Builds an entry for a path, with no name and not hidden.
    #[must_use]
    pub fn jadeed(masar: impl Into<PathBuf>) -> Self {
        Self { masar: masar.into(), ism: None, mukhfi: false }
    }
}

/// Where the manually added entries come from.
///
/// The one seam between this adapter and the local database. Implemented by
/// `taarib-makhzan`'s repository in the running product, and by
/// [`SijillThabit`] anywhere a literal list is enough.
pub trait SijillYadawi: std::fmt::Debug + Send + Sync {
    /// Every entry the user has added, in whatever order the store keeps them.
    fn madakhil(&self) -> Vec<MudkhalYadawi>;
}

/// A fixed list of entries, for callers with no database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SijillThabit(Vec<MudkhalYadawi>);

impl SijillThabit {
    /// Wraps a list.
    #[must_use]
    pub const fn jadeed(madakhil: Vec<MudkhalYadawi>) -> Self {
        Self(madakhil)
    }
}

impl SijillYadawi for SijillThabit {
    fn madakhil(&self) -> Vec<MudkhalYadawi> {
        self.0.clone()
    }
}

/// Manually added games.
#[derive(Debug, Clone)]
pub struct MatjarYadawi {
    sijill: Arc<dyn SijillYadawi>,
}

impl Default for MatjarYadawi {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl MatjarYadawi {
    /// Builds the adapter with no stored entries, so it reads only the extra
    /// folders in the user's settings.
    #[must_use]
    pub fn jadeed() -> Self {
        Self { sijill: Arc::new(SijillThabit::default()) }
    }

    /// Builds the adapter over a store.
    #[must_use]
    pub fn bi_sijill(sijill: Arc<dyn SijillYadawi>) -> Self {
        Self { sijill }
    }
}

impl Matjar for MatjarYadawi {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "مضافة يدويًا"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Added manually"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    /// There is no launcher to locate.
    ///
    /// The first extra folder the user configured stands in as a root so that
    /// the Diagnostics screen has somewhere to point, and `None` means the user
    /// has added nothing — which is not a failure and is the common case.
    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        siyaq
            .manassat
            .mujalladat_idafiya
            .iter()
            .find(|mujallad| mujallad.is_dir())
            .cloned()
            .or_else(|| {
                self.sijill
                    .madakhil()
                    .into_iter()
                    .find(|madkhal| madkhal.masar.exists())
                    .and_then(|madkhal| jidhr_min_masar(&madkhal.masar))
            })
    }

    /// # Errors
    ///
    /// Never. A path that has gone away, a folder with no executable in it and
    /// a duplicate of something already added are each a [`TanbihFahs`]: the
    /// user asked for these entries by hand, so every one that does not work is
    /// something they need told about individually, and none of them is a
    /// reason to withhold the ones that do.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: self.mawqi(siyaq),
            ..NatijatMatjar::default()
        };

        let mut maruf: Vec<String> = Vec::new();
        for madkhal in self.sijill.madakhil() {
            if madkhal.mukhfi {
                continue;
            }
            adif(&madkhal, siyaq.nizam, &mut maruf, &mut natija);
        }

        // Every immediate subdirectory of an extra folder is a candidate game.
        // The folder itself is not: a folder holding twelve games is not a
        // game, and treating it as one would produce a single entry rooted at
        // the whole library.
        for mujallad in &siyaq.manassat.mujalladat_idafiya {
            let Ok(qaima) = std::fs::read_dir(mujallad) else {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    mujallad.display().to_string(),
                    "this extra folder from Settings cannot be listed; correct it or remove it \
                     so it stops being scanned",
                ));
                continue;
            };
            let mut abnaa: Vec<PathBuf> =
                qaima
                    .flatten()
                    .map(|madkhal| madkhal.path())
                    .filter(|masar| masar.is_dir())
                    .collect();
            abnaa.sort();
            for ibn in abnaa {
                adif(&MudkhalYadawi::jadeed(ibn), siyaq.nizam, &mut maruf, &mut natija);
            }
        }

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur: Vec<PathBuf> = siyaq
            .manassat
            .mujalladat_idafiya
            .iter()
            .filter(|mujallad| mujallad.is_dir())
            .cloned()
            .collect();
        judhur.extend(
            self.sijill
                .madakhil()
                .iter()
                .filter(|madkhal| !madkhal.mukhfi)
                .filter_map(|madkhal| jidhr_min_masar(&madkhal.masar)),
        );
        judhur.sort();
        judhur.dedup();
        judhur
    }
}

/// Resolves one entry and appends it, or records why it could not be.
///
/// `maruf` carries the identities already produced by this scan, so the same
/// game reached through an explicit entry and through an extra folder becomes
/// one library row rather than two.
fn adif(
    madkhal: &MudkhalYadawi,
    nizam: NizamTashghil,
    maruf: &mut Vec<String>,
    natija: &mut NatijatMatjar,
) {
    match hall_masar(&madkhal.masar, madkhal.ism.as_deref(), nizam) {
        Ok((luba, mabtur)) => {
            let huwiya = luba.masdar.muarrif();
            if maruf.contains(&huwiya) {
                return;
            }
            if mabtur {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    madkhal.masar.display().to_string(),
                    "this folder holds more files than one search will look at, so the \
                     executable was chosen from the part that was examined. If Taarib picked \
                     the wrong one, point it at the game's own folder rather than at the \
                     folder above it.",
                ));
            }
            maruf.push(huwiya);
            natija.alaab.push(luba);
        },
        Err(khata) => natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            madkhal.masar.display().to_string(),
            khata.injilizi,
        )),
    }
}

/// The install root implied by a path: the directory itself, or an
/// executable's parent.
fn jidhr_min_masar(masar: &Path) -> Option<PathBuf> {
    if masar.is_dir() {
        Some(masar.to_path_buf())
    } else {
        masar.parent().map(Path::to_path_buf)
    }
}

// ---------------------------------------------------------------------------
// one path → one game
// ---------------------------------------------------------------------------

/// Artwork file names worth looking for beside a manually added game.
///
/// DRM-free installers drop these routinely — GOG writes a `.ico` next to the
/// executable, and people who organise their own libraries add `cover.png`
/// themselves. Finding one costs four `is_file` calls and is the only artwork
/// this source will ever have.
///
/// Visible to the crate because [`crate::matajir::mahmul`] classifies the same
/// files out of a directory listing it already holds, and two lists of artwork
/// names that could drift apart would be one list too many.
pub(crate) const ASMAA_SUWAR: [&str; 5] =
    ["cover.png", "cover.jpg", "folder.jpg", "poster.png", "grid.png"];

/// Turns a path the user chose into a discovered game.
///
/// A file is taken as the executable and its directory as the install root. A
/// directory is searched, and the highest-ranked candidate wins.
///
/// # Errors
///
/// Returns [`KhataKashf::LaYujadTanfidhi`] when the path does not exist, when
/// the directory holds no executable at all, and when everything it does hold
/// is an installer, an uninstaller, a crash reporter or a redistributable —
/// which is the same answer, because none of those is a game.
pub fn luba_min_masar(
    masar: &Path,
    ism: Option<&str>,
    nizam: NizamTashghil,
) -> Natija<LubaMuktashafa> {
    hall_masar(masar, ism, nizam).map(|(luba, _)| luba)
}

/// Resolves a path, and says whether the search that found the executable had
/// to stop early.
///
/// The truncation flag is separated from the public entry point on purpose: a
/// caller adding one game does not want a diagnostic about walk limits, while a
/// scan that walked somebody's whole extra-folders tree very much does.
fn hall_masar(
    masar: &Path,
    ism: Option<&str>,
    nizam: NizamTashghil,
) -> Natija<(LubaMuktashafa, bool)> {
    if !masar.exists() {
        return Err(KhataKashf::LaYujadTanfidhi { jidhr: masar.to_path_buf() }.into());
    }

    let mulaff = masar.is_file();
    let jidhr = jidhr_min_masar(masar)
        .ok_or_else(|| KhataKashf::LaYujadTanfidhi { jidhr: masar.to_path_buf() })?;

    let mashhad = imsah_mujallad(&jidhr, nizam);

    let tanfidhi = if mulaff {
        masar.to_path_buf()
    } else {
        mashhad
            .murashahat
            .first()
            .filter(|murashah| murashah.natija >= HADD_QUBUL)
            .map(|murashah| murashah.masar.clone())
            .ok_or_else(|| KhataKashf::LaYujadTanfidhi { jidhr: jidhr.clone() })?
    };

    let ism = ism
        .map(str::trim)
        .filter(|ism| !ism.is_empty())
        .map(str::to_owned)
        .or_else(|| ism_min_mujallad(&jidhr))
        .or_else(|| {
            tanfidhi.file_stem().map(|jidhr_ism| jidhr_ism.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Added game".to_owned());

    let mut simat = Vec::new();
    if nizam != NizamTashghil::Windows
        && tanfidhi.extension().is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("exe"))
    {
        simat.push(SimatLuba::TabaqatTawafuq(
            "a Windows executable on a system that is not Windows, so it runs through Wine or \
             Proton; Taarib does not yet know which prefix"
                .to_owned(),
        ));
    }

    let luba = LubaMuktashafa {
        masdar: MasdarLuba::Yadawi(muarrif_yadawi(&tanfidhi, nizam)),
        hala_matjar: None,
        ism,
        jidhr: jidhr.clone(),
        tanfidhi: Some(tanfidhi.clone()),
        hajm: mashhad.hajm,
        bina_manassa: None,
        akhir_tahdith: waqt_tadeel(&tanfidhi),
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: suwar_mujawira(&jidhr),
        khiyarat_tashghil: None,
        // Nobody is downloading this in the background. If the user pointed at
        // it, it is as complete as it is ever going to be.
        muktamila: true,
        simat,
    };
    Ok((luba, mashhad.mabtur))
}

/// The stable identity of a manually added game.
///
/// BLAKE3 over the canonical path of the executable, folded to lowercase where
/// the filesystem is case-insensitive so that `C:\Games\Game.exe` and
/// `c:\games\game.exe` are one game rather than two. The first sixteen bytes of
/// the digest are kept: 128 bits, which is far past the point where a collision
/// between two paths on one machine is worth a line of code, and short enough
/// that a user can paste it into a bug report.
///
/// Windows' verbatim prefix is stripped first. `fs::canonicalize` returns
/// `\\?\C:\Games\Game.exe` while every other source of the same path returns
/// `C:\Games\Game.exe`, and hashing the two would produce two identities for
/// one file.
#[must_use]
pub fn muarrif_yadawi(tanfidhi: &Path, nizam: NizamTashghil) -> String {
    let mutlaq =
        std::fs::canonicalize(tanfidhi).unwrap_or_else(|_| tanfidhi.to_path_buf());
    let nass = mutlaq.to_string_lossy();
    let munaqqa = nass.strip_prefix(r"\\?\").unwrap_or_else(|| nass.as_ref());
    let mabni =
        if nizam.hassas_lil_ahruf() { munaqqa.to_owned() } else { munaqqa.to_lowercase() };

    let basma = blake3::hash(mabni.as_bytes());
    basma.to_hex().get(..32).unwrap_or_default().to_owned()
}

/// A folder's name, cleaned of the noise installers leave in it.
///
/// A folder called `Hollow Knight` is the game's name. One called
/// `Hollow Knight v1.5.78.11 GOG` is the game's name plus a version and a
/// store, and showing that verbatim in the library is showing the user their
/// download filename rather than their game.
///
/// Visible to the crate because [`crate::matajir::mahmul`] names a ROM directory
/// the same way and must clean it the same way — a second copy of this rule
/// would drift, and the two sources would then spell one folder's name two
/// different ways in one library.
pub(crate) fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    let kham = jidhr.file_name()?.to_string_lossy().replace(['_', '.'], " ");
    let mut kalimat: Vec<&str> = Vec::new();
    for kalima in kham.split_whitespace() {
        let munaqqa = kalima.trim_matches(|harf: char| matches!(harf, '(' | ')' | '[' | ']'));
        if munaqqa.is_empty() {
            continue;
        }
        // A version marker ends the title: everything after `v1.5.78` is
        // packaging metadata, never part of the name.
        let raqmi = munaqqa
            .strip_prefix(['v', 'V'])
            .unwrap_or(munaqqa)
            .chars()
            .all(|harf| harf.is_ascii_digit() || harf == '-');
        if raqmi && !kalimat.is_empty() {
            break;
        }
        kalimat.push(munaqqa);
    }
    let ism = kalimat.join(" ");
    (!ism.trim().is_empty()).then(|| ism.trim().to_owned())
}

/// Artwork sitting beside the game, when there is any.
fn suwar_mujawira(jidhr: &Path) -> MasadirSuwar {
    let ghilaf = ASMAA_SUWAR
        .iter()
        .map(|ism| jidhr.join(ism))
        .find(|masar| masar.is_file())
        .map(MasdarSura::Malaf);
    MasadirSuwar { ghilaf, batl: None, shiar: None }
}

/// A file's modification time, as RFC 3339.
///
/// It is the closest thing a manually added game has to a build date: nothing
/// vouches for the version, but the executable's own timestamp changes when the
/// user patches or reinstalls, which is enough for the detail screen to say
/// something true.
///
/// Visible to the crate because [`crate::matajir::mahmul`] needs the same answer
/// for a ROM directory, where the directory's own timestamp is the only date
/// anything on disk offers.
pub(crate) fn waqt_tadeel(masar: &Path) -> Option<String> {
    let mubaddal = std::fs::metadata(masar).ok()?.modified().ok()?;
    let mudda = mubaddal.duration_since(std::time::UNIX_EPOCH).ok()?;
    let thawani = i64::try_from(mudda.as_secs()).ok()?;
    jiff::Timestamp::from_second(thawani).ok().map(|waqt| waqt.to_string())
}

// ---------------------------------------------------------------------------
// the ranking
// ---------------------------------------------------------------------------

/// How far below the install root the search goes.
///
/// Five levels reaches an Unreal shipping binary at
/// `Game/Binaries/Win64/Game-Win64-Shipping.exe` with room to spare, and stops
/// well short of the asset trees where a walk would spend real time.
const AQSA_UMQ: usize = 5;

/// How many filesystem entries one search will look at.
///
/// A bound, not a target. A user who points Taarib at their entire `D:` drive
/// gets an answer in a moment instead of a frozen window, and the truncation is
/// reported so the result is never silently partial.
const AQSA_MADAKHIL: usize = 20_000;

/// The score a candidate must reach to be accepted as the game.
const HADD_QUBUL: i64 = -1_000;

/// Bytes in a megabyte, for the size component of the score.
const BAYT_FI_MIGA: u64 = 1_048_576;

/// The executable's name is exactly the folder's name.
const WAZN_ISM_MUTABIQ: i64 = 1_000;

/// One name contains the other.
const WAZN_ISM_JUZI: i64 = 400;

/// A `<name>_Data` folder sits beside it: this is a Unity player, and that
/// folder is the game's own data directory named after this very executable.
const WAZN_BAYANAT_UNITY: i64 = 800;

/// An engine runtime library sits beside it.
const WAZN_MUHARRIK_MUJAWIR: i64 = 500;

/// It sits where Unreal puts a shipping binary.
const WAZN_UNREAL: i64 = 600;

/// It is built for the system doing the scanning.
///
/// Deliberately small. The host is the tie-break between two builds of one
/// game, so it sits below every piece of engine evidence and below even a
/// partial name match: it must never be able to lift a native tool over a
/// foreign binary that the engine's own layout points at.
const WAZN_MANASSA_ASLIYA: i64 = 200;

/// Cost of each level of depth below the install root.
const WAZN_UMQ: i64 = -120;

/// The most a candidate can earn for being large.
const AQSA_WAZN_HAJM: i64 = 512;

/// Cost of being named like an installer, uninstaller, crash reporter or
/// redistributable. Large enough that such a file is never chosen while a real
/// candidate exists, and large enough that a folder holding only these falls
/// below [`HADD_QUBUL`].
const WAZN_MARFUD: i64 = -5_000;

/// Cost of being named like a launcher, applied only when something else is
/// available.
const WAZN_MUSHAGHGHIL: i64 = -300;

/// Cost of being a bootstrap standing above the binary it starts.
///
/// Large enough to cancel an exact name match, because carrying the game's name
/// is exactly what a packaging shim does — Unreal's is a renamed copy of Epic's
/// `BootstrapPackagedGame`, so the one signal that would otherwise settle the
/// question is the one signal it is engineered to satisfy.
const WAZN_GHILAF: i64 = -1_000;

/// How much larger the real binary must be before a candidate counts as dwarfed
/// by it.
///
/// The gap this is aimed at is two orders of magnitude — a 195 KB shim beside
/// an 82 MB shipping binary — so a factor of eight is far short of the case it
/// catches and far past any pair of genuine game binaries in one install.
const NISBAT_GHILAF: u64 = 8;

/// Fragments that mean a file is not the game.
///
/// Matched as substrings of the normalized stem, so `vc_redist.x64` and
/// `VCREDIST_X86` are the same rule. These files ship *inside* games — every
/// Unity game carries a crash handler, every Windows game with a launcher-based
/// installer carries an uninstaller — so this list is what stops the fallback
/// path from confidently patching Microsoft's Visual C++ redistributable.
const AJZAA_MARFUDA: &[&str] = &[
    "unins",
    "uninstall",
    "setup",
    "install",
    "redist",
    "vcredist",
    "dxsetup",
    "dxwebsetup",
    "directx",
    "dotnetfx",
    "dotnet",
    "oalinst",
    "webview",
    "prereq",
    "unitycrashhandler",
    "crashreportclient",
    "crashhandler",
    "crashreport",
    "crashpad",
    "crashsender",
    "notificationhelper",
    "burstdebuginformation",
];

/// Fragments that mean a file is probably a launcher rather than the game.
///
/// `bootstrap` and `startprotectedgame` are here for the same reason as
/// `launcher`: both name a program whose entire job is to start another one.
/// The first is Epic's packaging shim under its own name, the second is the
/// wrapper Steam's DRM leaves at the install root.
const AJZAA_MUSHAGHGHIL: &[&str] = &["launcher", "launch", "bootstrap", "startprotectedgame"];

/// Extensions admitted on the execute bit alone.
///
/// A shell script may open with a shebang, with a comment, or with the first
/// line of the script; its first bytes prove nothing, so the bit is the whole
/// of the evidence there is.
const IMTIDADAT_BILA_TARWISA: &[&str] = &["sh"];

/// Extensions a native build carries, admitted only when the file's own header
/// agrees that it is a program.
///
/// `.bin` is as common a name for game data as it is for a program, and on a
/// mount that reports every file as executable the extension would otherwise be
/// the only thing standing between a data blob and the library screen.
const IMTIDADAT_BINA: &[&str] = &["x86_64", "x86", "appimage", "bin"];

/// Bytes read from a file to see whether it is a program at all.
///
/// Four is the whole of every magic this module recognises: ELF and the six
/// Mach-O forms are exactly four bytes, and `MZ` and `#!` are prefixes of it.
const TUL_SIHR: usize = 4;

/// What kind of program a candidate is, as the file itself says.
///
/// Read from the extension, and from the first bytes wherever the extension
/// settles nothing. Deliberately not read from the host: one directory has to
/// yield one answer on every machine that can see it, or the identity derived
/// from that answer moves when the drive is plugged into a different computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NawBarnamaj {
    /// A Windows PE — a `.exe`, or an `MZ` image under some other name.
    Windows,
    /// Anything a Unix kernel loads: ELF, Mach-O, a script, or one of the
    /// extensions a native game build carries.
    Yuniks,
    /// A macOS application bundle, which is a directory rather than a file.
    HazmaMac,
}

impl NawBarnamaj {
    /// Whether the system doing the scanning could run this directly.
    const fn asliya(self, nizam: NizamTashghil) -> bool {
        matches!(
            (self, nizam),
            (Self::Windows, NizamTashghil::Windows)
                | (Self::Yuniks, NizamTashghil::Linux | NizamTashghil::Mac)
                | (Self::HazmaMac, NizamTashghil::Mac)
        )
    }
}

/// One executable that could be the game, with the score that ranked it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MurashahTanfidhi {
    /// The file, absolute.
    pub masar: PathBuf,
    /// Its score. Higher is more likely to be the game.
    pub natija: i64,
    /// Its size in bytes.
    pub hajm: u64,
    /// How far below the install root it sits; 1 is directly inside it.
    pub umq: usize,
}

/// What one search of an install root found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MashhadMujallad {
    /// Every candidate, best first.
    murashahat: Vec<MurashahTanfidhi>,
    /// Total bytes of every file seen, which is the closest thing this source
    /// has to a reported install size.
    hajm: u64,
    /// Whether the walk stopped at [`AQSA_MADAKHIL`] rather than finishing.
    mabtur: bool,
}

/// Ranks every executable under a directory, best first.
///
/// Public because the interface offers the choice: when Taarib picks wrong, the
/// user is shown this list rather than a file dialog, and picking from it is
/// one click instead of a hunt through `Binaries/Win64`.
#[must_use]
pub fn rattib_tanfidhiyat(jidhr: &Path, nizam: NizamTashghil) -> Vec<MurashahTanfidhi> {
    imsah_mujallad(jidhr, nizam).murashahat
}

/// Walks an install root once, collecting candidates and the total size.
fn imsah_mujallad(jidhr: &Path, nizam: NizamTashghil) -> MashhadMujallad {
    let ism_mujallad = jidhr
        .file_name()
        .map(|ism| wahhid_ism(&ism.to_string_lossy()))
        .unwrap_or_default();

    let mut mashhad = MashhadMujallad::default();
    let mut adad = 0usize;

    let mashi = walkdir::WalkDir::new(jidhr)
        .max_depth(AQSA_UMQ)
        .follow_links(false)
        .sort_by_file_name();

    for madkhal in mashi.into_iter().filter_map(Result::ok) {
        adad = adad.saturating_add(1);
        if adad > AQSA_MADAKHIL {
            mashhad.mabtur = true;
            break;
        }

        let masar = madkhal.path();
        let Ok(bayanat) = madkhal.metadata() else {
            continue;
        };
        if bayanat.is_file() {
            mashhad.hajm = mashhad.hajm.saturating_add(bayanat.len());
        }

        // A macOS application bundle is one thing, not a directory tree: its
        // contents count towards the size but are never candidates in their own
        // right, because the thing the user runs is the bundle.
        if dakhil_hazma(masar) {
            continue;
        }
        let Some(naw) = naw_murashah(masar, &bayanat) else {
            continue;
        };

        let umq = madkhal.depth().max(1);
        mashhad.murashahat.push(MurashahTanfidhi {
            natija: qayyim(masar, naw, nizam, umq, bayanat.len(), &ism_mujallad),
            masar: masar.to_path_buf(),
            hajm: bayanat.len(),
            umq,
        });
    }

    aqib_al_mushaghghilat(&mut mashhad.murashahat);
    aqib_al_aghlifa(&mut mashhad.murashahat);
    mashhad.murashahat.sort_by(|awwal, thani| {
        thani
            .natija
            .cmp(&awwal.natija)
            .then_with(|| awwal.umq.cmp(&thani.umq))
            .then_with(|| awwal.masar.cmp(&thani.masar))
    });
    mashhad
}

/// Applies the launcher penalty, but only where it is deserved.
///
/// A game that ships nothing but `Launcher.exe` must still be found, so the
/// penalty is conditional: it applies to a launcher-named candidate only when
/// some other candidate exists that is not launcher-named and is no deeper in
/// the tree. That is the case where the launcher really is a wrapper around a
/// binary Taarib can see for itself.
fn aqib_al_mushaghghilat(murashahat: &mut [MurashahTanfidhi]) {
    let aqal_ghayr_mushaghghil = murashahat
        .iter()
        .filter(|murashah| !huwa_mushaghghil(&murashah.masar))
        .map(|murashah| murashah.umq)
        .min();
    let Some(aqal) = aqal_ghayr_mushaghghil else {
        return;
    };
    for murashah in murashahat.iter_mut() {
        if huwa_mushaghghil(&murashah.masar) && murashah.umq >= aqal {
            murashah.natija = murashah.natija.saturating_add(WAZN_MUSHAGHGHIL);
        }
    }
}

/// Sinks a bootstrap: a small program with nothing engine-shaped around it,
/// standing above a binary the engine's own layout vouches for.
///
/// Unreal packages a game as `<Root>/<Project>.exe` — a renamed copy of Epic's
/// `BootstrapPackagedGame`, whose only job is to start
/// `<Root>/<Project>/Binaries/<Platform>/<Project>-Win64-Shipping.exe` — and
/// the shim carries the game's name while the game carries the engine's. Left
/// alone the shim wins on the name and the game loses on its depth, which is
/// how a 195 KB stub came to be reported as an 82 MB UE 4.27 title, with no
/// version at all: nothing downstream re-opens the question once discovery has
/// named a file.
///
/// Two tells, because a shim is not always tiny:
///
/// - it is dwarfed — under a [`NISBAT_GHILAF`]th — by a candidate the engine
///   vouches for;
/// - it sits at the install root while such a candidate sits under
///   `Binaries/<Platform>/`, where Unreal puts the shipping binary and puts
///   nothing else, and it is smaller than that binary.
///
/// Both require that the candidate carries no engine evidence *of its own* —
/// see [`wazn_muharrik_khass`] — which is what keeps a Unity player out of it.
/// A player is small, sits at the root and is named after the game, so it
/// answers the shape exactly; what separates it is that its `<name>_Data`
/// folder is the engine naming that very file.
///
/// The version resource would say so outright: the stub's `FileDescription`
/// reads `BootstrapPackagedGame`. It is not read here, because reaching it
/// means walking a PE resource tree, which this crate does once already in
/// [`crate::ayquna`] and should not do twice — and because the layout above is
/// evidence a stripped or packed binary cannot take away.
fn aqib_al_aghlifa(murashahat: &mut [MurashahTanfidhi]) {
    let khass: Vec<i64> =
        murashahat.iter().map(|murashah| wazn_muharrik_khass(&murashah.masar)).collect();

    let marajii = || murashahat.iter().zip(&khass).filter(|(_, wazn)| **wazn > 0);
    let akbar_marja = marajii().map(|(murashah, _)| murashah.hajm).max();
    let akbar_fi_binaries = marajii()
        .filter(|(murashah, _)| fi_mujallad_binaries(&murashah.masar))
        .map(|(murashah, _)| murashah.hajm)
        .max();

    for (murashah, wazn) in murashahat.iter_mut().zip(&khass) {
        if *wazn > 0 {
            continue;
        }
        let mudhawwab = akbar_marja
            .is_some_and(|akbar| murashah.hajm.saturating_mul(NISBAT_GHILAF) < akbar);
        let fawq_al_bina = akbar_fi_binaries
            .is_some_and(|akbar| murashah.umq <= 1 && murashah.hajm < akbar);
        if mudhawwab || fawq_al_bina {
            murashah.natija = murashah.natija.saturating_add(WAZN_GHILAF);
        }
    }
}

/// Scores one candidate. Every component is documented in the module header's
/// table, and the two are kept in step deliberately.
fn qayyim(
    masar: &Path,
    naw: NawBarnamaj,
    nizam: NizamTashghil,
    umq: usize,
    hajm: u64,
    ism_mujallad: &str,
) -> i64 {
    let mut natija: i64 = 0;

    let jidhr_ism =
        masar.file_stem().map(|ism| wahhid_ism(&ism.to_string_lossy())).unwrap_or_default();

    // 1 — the name.
    if !jidhr_ism.is_empty() && !ism_mujallad.is_empty() {
        if jidhr_ism == ism_mujallad {
            natija = natija.saturating_add(WAZN_ISM_MUTABIQ);
        } else if jidhr_ism.len() >= 3
            && (ism_mujallad.contains(&jidhr_ism) || jidhr_ism.contains(ism_mujallad))
        {
            natija = natija.saturating_add(WAZN_ISM_JUZI);
        }
    }

    // 2 — engine evidence.
    natija = natija.saturating_add(wazn_muharrik(masar));

    // 3 — depth.
    let khutuwat = i64::try_from(umq.saturating_sub(1)).unwrap_or(i64::MAX);
    natija = natija.saturating_add(khutuwat.saturating_mul(WAZN_UMQ));

    // 4 — size.
    let miga = i64::try_from(hajm.checked_div(BAYT_FI_MIGA).unwrap_or(0)).unwrap_or(AQSA_WAZN_HAJM);
    natija = natija.saturating_add(miga.min(AQSA_WAZN_HAJM));

    // 5 — the machine doing the scanning, which is a preference and not a gate.
    if naw.asliya(nizam) {
        natija = natija.saturating_add(WAZN_MANASSA_ASLIYA);
    }

    // 6 — the refusals.
    if AJZAA_MARFUDA.iter().any(|juz| jidhr_ism.contains(juz)) {
        natija = natija.saturating_add(WAZN_MARFUD);
    }

    natija
}

/// Engine evidence that names this executable rather than its folder.
///
/// Both halves point at one file: `<name>_Data` is Unity's naming rule applied
/// to this very executable, and `Binaries/<Platform>/` is where Unreal puts a
/// shipping binary and nothing else. A runtime library merely sitting in the
/// same folder is deliberately absent — it says the folder holds a game, not
/// which file in it is the game, and every helper beside the player would
/// inherit it, which is exactly the mistake [`aqib_al_aghlifa`] has to avoid.
fn wazn_muharrik_khass(masar: &Path) -> i64 {
    let Some(mujallad) = masar.parent() else {
        return 0;
    };
    let mut wazn: i64 = 0;

    // Unity names the data folder after the player executable, so this is not a
    // guess about the engine — it is the engine's own naming rule.
    if let Some(jidhr_ism) = masar.file_stem()
        && mujallad.join(format!("{}_Data", jidhr_ism.to_string_lossy())).is_dir()
    {
        wazn = wazn.saturating_add(WAZN_BAYANAT_UNITY);
    }

    // Unreal: `.../Binaries/Win64/Game-Win64-Shipping.exe`. Either half of the
    // shape is evidence on its own, and neither is worth double-counting.
    let shipping = masar
        .file_stem()
        .map(|ism| ism.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|ism| ism.ends_with("-shipping"));
    if fi_mujallad_binaries(masar) || shipping {
        wazn = wazn.saturating_add(WAZN_UNREAL);
    }

    wazn
}

/// What the files around a candidate say about it.
fn wazn_muharrik(masar: &Path) -> i64 {
    const JIRAN_MUHARRIK: [&str; 5] = [
        "UnityPlayer.dll",
        "GameAssembly.dll",
        "UnityPlayer.so",
        "libUnityPlayer.so",
        "UnityPlayer.dylib",
    ];

    let mut wazn = wazn_muharrik_khass(masar);
    if let Some(mujallad) = masar.parent()
        && JIRAN_MUHARRIK.iter().any(|jar| mujallad.join(jar).is_file())
    {
        wazn = wazn.saturating_add(WAZN_MUHARRIK_MUJAWIR);
    }
    wazn
}

/// Whether a path sits directly inside a `Binaries/<Platform>/` directory.
///
/// Evidence both ways, which is why it is named once and used twice: for the
/// file that is in it, because Unreal puts the shipping binary there; and
/// against the file that is not, because Unreal puts nothing runnable at the
/// packaging root except the bootstrap.
fn fi_mujallad_binaries(masar: &Path) -> bool {
    masar
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .is_some_and(|ism| ism.eq_ignore_ascii_case("Binaries"))
}

/// Whether a path is named like a launcher.
fn huwa_mushaghghil(masar: &Path) -> bool {
    let jidhr_ism =
        masar.file_stem().map(|ism| wahhid_ism(&ism.to_string_lossy())).unwrap_or_default();
    AJZAA_MUSHAGHGHIL.iter().any(|juz| jidhr_ism.contains(juz))
}

/// Whether a path sits inside a macOS application bundle.
fn dakhil_hazma(masar: &Path) -> bool {
    let mut ajzaa: Vec<_> = masar.components().collect();
    let _ = ajzaa.pop();
    ajzaa.iter().any(|juz| {
        juz.as_os_str().to_string_lossy().to_ascii_lowercase().ends_with(".app")
    })
}

/// What kind of program this entry is, or [`None`] when it is not one.
///
/// The directory's contents decide, never the host. A `.exe` is a Windows
/// program on every machine that can read the disk, and a `.exe` beside a
/// `<name>_Data` folder is a Unity game whether the reader is Windows, a Linux
/// desktop that will run it through Proton, or a Mac with the drive mounted:
/// the folder is the engine's own naming rule applied to that executable, and a
/// naming rule does not stop holding because of who is looking at it. Gating
/// this on the host instead refused both Unreal titles outright on a Linux
/// machine, for no better reason than the machine.
///
/// The execute bit is asked for where it means something and never trusted
/// alone. A filesystem that carries no permissions of its own — NTFS, exFAT,
/// FAT — reports every file as executable once it is mounted on Linux, so a
/// file whose extension settles nothing must show a program's header before it
/// is admitted. Without that, thirteen megabytes of Unity scene data outscored
/// `hollow_knight.exe`.
fn naw_murashah(masar: &Path, bayanat: &std::fs::Metadata) -> Option<NawBarnamaj> {
    let imtidad =
        masar.extension().map(|imtidad| imtidad.to_string_lossy().to_ascii_lowercase());

    if bayanat.is_dir() {
        // A `.app` bundle is a directory and is the thing the user runs.
        return (imtidad.as_deref() == Some("app")).then_some(NawBarnamaj::HazmaMac);
    }
    if !bayanat.is_file() {
        return None;
    }

    match imtidad.as_deref() {
        Some("exe") => Some(NawBarnamaj::Windows),
        Some(imtidad) if IMTIDADAT_BILA_TARWISA.contains(&imtidad) => {
            qabil_lil_tanfidh(bayanat).then_some(NawBarnamaj::Yuniks)
        },
        Some(imtidad) if IMTIDADAT_BINA.contains(&imtidad) => naw_min_tarwisa(masar, bayanat),
        Some(_) => None,
        None => naw_min_tarwisa(masar, bayanat),
    }
}

/// The kind a file's own header says it is, for an entry whose name did not
/// settle it.
fn naw_min_tarwisa(masar: &Path, bayanat: &std::fs::Metadata) -> Option<NawBarnamaj> {
    if qabil_lil_tanfidh(bayanat) { sihr_barnamaj(masar) } else { None }
}

/// The kind a file's first bytes say it is.
///
/// The header is the one fact a mount cannot forge, which is why it decides
/// wherever the extension does not.
fn sihr_barnamaj(masar: &Path) -> Option<NawBarnamaj> {
    use std::io::Read as _;

    /// Both endiannesses of the thin 32- and 64-bit magics, and both fat ones.
    const ASHKAL_MACH: [[u8; TUL_SIHR]; 6] = [
        [0xFE, 0xED, 0xFA, 0xCE],
        [0xFE, 0xED, 0xFA, 0xCF],
        [0xCE, 0xFA, 0xED, 0xFE],
        [0xCF, 0xFA, 0xED, 0xFE],
        [0xCA, 0xFE, 0xBA, 0xBE],
        [0xBE, 0xBA, 0xFE, 0xCA],
    ];
    /// The four bytes every ELF opens with.
    const SIHR_ELF: [u8; TUL_SIHR] = [0x7F, b'E', b'L', b'F'];

    let mut sihr = [0_u8; TUL_SIHR];
    // A file too short to hold a magic is too short to be a program.
    std::fs::File::open(masar).ok()?.read_exact(&mut sihr).ok()?;

    if sihr == SIHR_ELF || ASHKAL_MACH.contains(&sihr) || sihr.starts_with(b"#!") {
        return Some(NawBarnamaj::Yuniks);
    }
    // Every PE opens with the DOS stub, so a Windows program answers to `MZ`
    // under whatever name it was given.
    sihr.starts_with(b"MZ").then_some(NawBarnamaj::Windows)
}

/// Whether any execute bit is set.
#[cfg(unix)]
fn qabil_lil_tanfidh(bayanat: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    bayanat.permissions().mode() & 0o111 != 0
}

/// Whether any execute bit is set.
///
/// Windows has no execute bit, so there is nothing to read and the answer is
/// yes. This is not a hole: a Windows host reaches [`naw_murashah`]'s Unix arms
/// for the same files a Linux host does — the contents decide — and on those
/// arms the file's own header carries the whole of the decision anyway.
#[cfg(not(unix))]
fn qabil_lil_tanfidh(_bayanat: &std::fs::Metadata) -> bool {
    true
}
