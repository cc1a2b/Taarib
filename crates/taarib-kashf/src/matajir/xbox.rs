//! إكس بوكس — the Microsoft Store and Xbox app, read out of package manifests.
//!
//! Microsoft Store games are MSIX packages, not ordinary installations. There
//! is no catalogue file listing them and no registry key naming their
//! directories; the packages themselves are the catalogue, and each one carries
//! an `AppxManifest.xml` that declares its identity, its logos and the
//! applications it publishes.
//!
//! Packages live in two places:
//!
//! - On the system drive, under `%ProgramFiles%\WindowsApps`, one directory per
//!   installed package, named with the package **full** name.
//! - On any other drive, under the folder that drive's `.GamingRoot` file names
//!   — conventionally `<drive>\XboxGames` — with each game in its own directory
//!   and the manifest one level down in `Content\`.
//!
//! ## Reading `.GamingRoot`
//!
//! `.GamingRoot` sits at the root of every volume the Xbox app has ever been
//! told to install to. It is a very small binary file: the four ASCII bytes
//! `RGBX`, then one or more NUL-terminated UTF-16 little-endian strings naming
//! folders on that same volume, relative to its root. It is read here rather
//! than assumed, because a user who moved their library to `D:\Games\Xbox` has
//! no `D:\XboxGames` at all, and a scanner that only knows the convention finds
//! nothing on their machine.
//!
//! The reader is bounds-checked at every step: the magic is compared against a
//! four-byte window taken with `get`, code units are read two bytes at a time
//! from windows taken with `get`, and a file that ends mid-character or without
//! a terminator produces a warning naming the byte offset rather than a
//! truncated path or an out-of-bounds read. `.GamingRoot` is written by the
//! Xbox app but lives on removable media that gets yanked mid-write, so a
//! partial one is an ordinary thing to meet.
//!
//! ## Denied reads are normal here, and are not scan failures
//!
//! `WindowsApps` is installed with restricted ACLs. Its owner is
//! `NT SERVICE\TrustedInstaller` and its DACL grants ordinary read access to no
//! one — not to the interactive user, and **not to an administrator either**:
//! an elevated process still gets `ERROR_ACCESS_DENIED` until it takes
//! ownership, which Taarib will never do. Some packages are additionally
//! sealed with per-package ACLs even when the parent directory can be listed.
//!
//! So a denied read here is the expected case, not an exception. Every one of
//! them becomes a [`TanbihFahs`] naming the package that could not be read,
//! and the scan continues with the packages that could. Failing the whole Xbox
//! scan on the first denied directory would be failing it on almost every
//! machine, and telling the user to run Taarib as administrator would be
//! telling them to do something that does not work.
//!
//! ## The container note
//!
//! An MSIX game may run in a full-trust desktop process or inside an
//! `AppContainer`, and which one it is decides what Phase 15 can do to it: a
//! containerised process reads its files through a virtualised view, refuses to
//! load a library whose ACL does not carry the package's own capability SID,
//! and is started by a shim rather than by its own executable. That is a
//! different installation strategy, not a different flag, so every package this
//! adapter believes is containerised carries a
//! [`SimatLuba::TabaqatTawafuq`] note naming the evidence that says so.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use sha2::Digest;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "xbox";

/// Microsoft Store packages exist on Windows and nowhere else.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The four bytes every `.GamingRoot` begins with.
const SIHR_GAMING_ROOT: [u8; 4] = *b"RGBX";

/// A `.GamingRoot` larger than this is not a `.GamingRoot`. The real files are
/// a few dozen bytes; a kilobyte is already three times the longest plausible
/// list of folder names.
const AQSA_HAJM_GAMING_ROOT: u64 = 4096;

/// The manifest file name inside every package.
const ISM_BAYAN: &str = "AppxManifest.xml";

/// The folder Xbox games sit in on a secondary drive when `.GamingRoot` is
/// missing or unreadable, which is the convention the Xbox app follows.
const MUJALLAD_MUTAARAF: &str = "XboxGames";

/// The shim Xbox PC titles declare as their package executable.
///
/// It is not the game. It is a launcher stub that sets the container up and
/// then starts the real binary, and its presence is one of the three signals
/// that a package is containerised.
const SHIM_TASHGHIL: &str = "gamelaunchhelper.exe";

/// Packages published by Microsoft that are infrastructure rather than games.
///
/// They carry manifests indistinguishable in shape from a game's, and without
/// this list the library fills up with runtimes the user never installed on
/// purpose. Matched as a prefix of the package identity name.
const BIDAYAT_GHAYR_LUBA: &[&str] = &[
    "Microsoft.VCLibs",
    "Microsoft.NET",
    "Microsoft.UI.Xaml",
    "Microsoft.Services.Store",
    "Microsoft.WindowsAppRuntime",
    "Microsoft.DirectXRuntime",
    "Microsoft.GamingServices",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.XboxGameOverlay",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.Xbox.TCUI",
    "Microsoft.HEIFImageExtension",
    "Microsoft.VP9VideoExtensions",
    "Microsoft.WebMediaExtensions",
    "Microsoft.WebpImageExtension",
];

/// The Microsoft Store and the Xbox app.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarXbox;

impl MatjarXbox {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarXbox {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "إكس بوكس"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Xbox"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        if let Some(tajawuz) = siyaq.manassat.xbox.as_ref() {
            return Some(tajawuz.clone());
        }
        let jidhr = mujallad_windows_apps(siyaq)?;
        // `exists` rather than `is_dir`: WindowsApps denies the metadata query
        // that `is_dir` needs on some configurations while still answering
        // whether the name is there at all.
        jidhr.exists().then_some(jidhr)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when the user configured a
    /// package root that is not there. Nothing else in this adapter is fatal:
    /// a `WindowsApps` directory that cannot be listed, a package whose
    /// manifest is unreadable, a `.GamingRoot` that is truncated — each is a
    /// [`TanbihFahs`], because on Windows every one of them is an ordinary
    /// consequence of the ACLs Microsoft ships and none of them says anything
    /// about the packages that *did* read.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        if let Some(tajawuz) = siyaq.manassat.xbox.as_ref()
            && !tajawuz.exists()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: tajawuz.clone(),
            }
            .into());
        }

        let mut tanbihat: Vec<TanbihFahs> = Vec::new();
        let mut mujalladat: Vec<PathBuf> = Vec::new();
        let jidhr_nizam = siyaq
            .manassat
            .xbox
            .clone()
            .or_else(|| mujallad_windows_apps(siyaq))
            .filter(|masar| masar.exists());
        if let Some(jidhr_nizam) = jidhr_nizam.as_ref() {
            mujalladat.push(jidhr_nizam.clone());
        }
        mujalladat.extend(judhur_al_aqrass(siyaq, &mut tanbihat));
        // No package root anywhere, and no drive whose `.GamingRoot` refused
        // to say where one is: nothing on this machine holds Xbox packages.
        if jidhr_nizam.is_none() && mujalladat.is_empty() && tanbihat.is_empty() {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, jidhr_nizam);
        natija.tanbihat = tanbihat;

        // One package can be reachable through two roots — a `.GamingRoot`
        // naming a folder that is also the conventional one, most commonly —
        // and the same game must not appear twice.
        let mut mazurat: BTreeSet<PathBuf> = BTreeSet::new();
        for mujallad in mujalladat {
            for ruzma in ruzam_fi(&mujallad, &mut natija.tanbihat) {
                if !mazurat.insert(muwahhad(&ruzma)) {
                    continue;
                }
                if let Some(luba) = luba_min_ruzma(&ruzma, &mut natija.tanbihat) {
                    natija.alaab.push(luba);
                }
            }
        }

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let mut judhur = Vec::new();
        let jidhr_nizam = siyaq
            .manassat
            .xbox
            .clone()
            .or_else(|| mujallad_windows_apps(siyaq))
            .filter(|masar| masar.exists());
        judhur.extend(jidhr_nizam);
        // Warnings raised while locating the roots are discarded here on
        // purpose: this function answers "what should be watched", and a
        // diagnostic about a drive belongs to the scan that produced it, not to
        // the watch list, where nothing would ever read it.
        let mut muhmal = Vec::new();
        judhur.extend(judhur_al_aqrass(siyaq, &mut muhmal));
        judhur
    }
}

/// The system-drive package root.
///
/// The native program directory, not the 32-bit one: `WindowsApps` exists only
/// under `%ProgramFiles%`, and a package is never installed beside a 32-bit
/// application. [`None`] on a context that has no program directories at all,
/// which is every context that is not Windows.
fn mujallad_windows_apps(siyaq: &SiyaqFahs) -> Option<PathBuf> {
    Some(siyaq.mujallad_baramij_asli()?.join("WindowsApps"))
}

/// A case-folded form of a package directory, used only to notice that two
/// roots led to the same package. Windows paths are case-insensitive, so
/// comparing them verbatim would let `D:\XboxGames\Game` and
/// `D:\xboxgames\Game` both become library entries.
fn muwahhad(masar: &Path) -> PathBuf {
    PathBuf::from(masar.as_os_str().to_string_lossy().to_lowercase())
}

// ---------------------------------------------------------------------------
// secondary drives and .GamingRoot
// ---------------------------------------------------------------------------

/// Every games folder on every drive other than the system one.
///
/// Each mounted volume is asked two questions: what its `.GamingRoot` names,
/// and whether it has the conventional `XboxGames` folder. Both answers are
/// kept, because a drive can legitimately have both after a library move.
fn judhur_al_aqrass(siyaq: &SiyaqFahs, tanbihat: &mut Vec<TanbihFahs>) -> Vec<PathBuf> {
    let mut judhur = Vec::new();
    for harf in ahruf_al_aqrass() {
        let jidhr_qurs = PathBuf::from(format!("{harf}:\\"));
        judhur.extend(mujalladat_gaming_root(&jidhr_qurs, tanbihat));
        let mutaaraf = jidhr_qurs.join(MUJALLAD_MUTAARAF);
        if mutaaraf.is_dir() {
            judhur.push(mutaaraf);
        }
    }
    // The user's own extra folders are scanned for packages too: somebody who
    // keeps an Xbox library on a network share or a mount point with no drive
    // letter has no other way to be found.
    for idafi in &siyaq.manassat.mujalladat_idafiya {
        let mutaaraf = idafi.join(MUJALLAD_MUTAARAF);
        if mutaaraf.is_dir() {
            judhur.push(mutaaraf);
        }
    }
    judhur
}

/// The drive letters currently mounted.
#[cfg(windows)]
fn ahruf_al_aqrass() -> Vec<char> {
    // SAFETY: GetLogicalDrives takes no arguments, reads and writes no memory
    // the caller owns, and returns a bitmask by value. A zero result means no
    // volumes are mounted, which the filter below handles as an empty list.
    let qinaa = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
    (0u32..26)
        .filter(|raqm| qinaa.checked_shr(*raqm).is_some_and(|munzah| munzah & 1 == 1))
        .filter_map(|raqm| char::from_u32(u32::from(b'A').saturating_add(raqm)))
        .collect()
}

/// The drive letters currently mounted.
///
/// There are none anywhere but Windows. The function exists on the other
/// platforms so the module compiles everywhere and the adapter's shape does not
/// change with the target.
#[cfg(not(windows))]
const fn ahruf_al_aqrass() -> Vec<char> {
    Vec::new()
}

/// A failure while reading a `.GamingRoot`, with the offset it happened at.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KhataGamingRoot {
    /// The byte offset the reader stopped at.
    mawdi: usize,
    /// What was wrong there.
    tafsil: String,
}

impl KhataGamingRoot {
    fn jadeed(mawdi: usize, tafsil: impl Into<String>) -> Self {
        Self { mawdi, tafsil: tafsil.into() }
    }
}

/// The games folders a volume's `.GamingRoot` names.
fn mujalladat_gaming_root(jidhr_qurs: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<PathBuf> {
    let masar = jidhr_qurs.join(".GamingRoot");
    let Ok(bayanat) = std::fs::metadata(&masar) else {
        return Vec::new();
    };
    if !bayanat.is_file() {
        return Vec::new();
    }
    if bayanat.len() > AQSA_HAJM_GAMING_ROOT {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            masar.display().to_string(),
            format!(
                "this .GamingRoot is {} bytes, far larger than the format ever is, so it was not \
                 read; the conventional XboxGames folder on this drive is still scanned",
                bayanat.len()
            ),
        ));
        return Vec::new();
    }

    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) => {
            tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                masar.display().to_string(),
                format!(
                    "cannot read this drive's .GamingRoot ({sabab}); the conventional XboxGames \
                     folder on it is still scanned"
                ),
            ));
            return Vec::new();
        },
    };

    let nisabi = match masarat_min_gaming_root(&bayt) {
        Ok(nisabi) => nisabi,
        Err(khata) => {
            tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                masar.display().to_string(),
                format!(
                    "this .GamingRoot stops making sense at byte {}: {}. The drive's conventional \
                     XboxGames folder is still scanned.",
                    khata.mawdi, khata.tafsil
                ),
            ));
            return Vec::new();
        },
    };

    let mut mujalladat = Vec::new();
    for nisbi in nisabi {
        // The folder name comes out of a binary file on removable media, so it
        // goes through the same containment check as any other untrusted path
        // rather than being joined directly.
        match dakhil(jidhr_qurs, nisbi.trim_end_matches(['\\', '/'])) {
            Ok(mujallad) if mujallad.is_dir() => mujalladat.push(mujallad),
            Ok(_) => {},
            Err(khata) => tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                masar.display().to_string(),
                format!(
                    "this .GamingRoot names a folder Taarib will not follow: {}",
                    khata.injilizi
                ),
            )),
        }
    }
    mujalladat
}

/// Parses the `.GamingRoot` container.
///
/// Layout, in full: the ASCII bytes `RGBX`, then a sequence of NUL-terminated
/// UTF-16 little-endian strings, ending either at the end of the file or at an
/// empty string. Each string is a folder path relative to the volume root.
///
/// Every read is a `get` over a bounded window, so a file truncated anywhere —
/// inside the magic, inside a code unit, or before a terminator — returns the
/// offset it failed at instead of reading past the end.
fn masarat_min_gaming_root(bayt: &[u8]) -> Result<Vec<String>, KhataGamingRoot> {
    let sihr = bayt
        .get(..SIHR_GAMING_ROOT.len())
        .ok_or_else(|| KhataGamingRoot::jadeed(0, "file is shorter than the four-byte signature"))?;
    if sihr != SIHR_GAMING_ROOT {
        return Err(KhataGamingRoot::jadeed(
            0,
            format!(
                "signature is {sihr:02x?} rather than the RGBX the format declares, so this is \
                 not a .GamingRoot at all"
            ),
        ));
    }

    let mut masarat = Vec::new();
    let mut mawdi = SIHR_GAMING_ROOT.len();
    while mawdi < bayt.len() {
        let bidaya = mawdi;
        let mut wahdat: Vec<u16> = Vec::new();
        let mut muntahi = false;
        while mawdi < bayt.len() {
            let zawj = bayt.get(mawdi..mawdi.saturating_add(2)).ok_or_else(|| {
                KhataGamingRoot::jadeed(mawdi, "file ends in the middle of a UTF-16 code unit")
            })?;
            let wahda = <[u8; 2]>::try_from(zawj)
                .map(u16::from_le_bytes)
                .map_err(|_| KhataGamingRoot::jadeed(mawdi, "cannot read a UTF-16 code unit"))?;
            mawdi = mawdi.saturating_add(2);
            if wahda == 0 {
                muntahi = true;
                break;
            }
            wahdat.push(wahda);
        }
        if !muntahi {
            return Err(KhataGamingRoot::jadeed(
                bidaya,
                "the last folder name has no NUL terminator, so the file was truncated",
            ));
        }
        if wahdat.is_empty() {
            // An empty string terminates the list; anything after it is
            // padding the format does not define.
            break;
        }
        let nass = char::decode_utf16(wahdat.iter().copied())
            .collect::<Result<String, _>>()
            .map_err(|_| {
                KhataGamingRoot::jadeed(bidaya, "folder name is not valid UTF-16")
            })?;
        masarat.push(nass);
    }
    Ok(masarat)
}

// ---------------------------------------------------------------------------
// finding packages
// ---------------------------------------------------------------------------

/// Every package directory directly under a root.
///
/// A package directory is one that holds an `AppxManifest.xml`, either at its
/// top level — which is how `WindowsApps` is laid out — or one level down in
/// `Content`, which is how the Xbox app lays a game out on a secondary drive.
fn ruzam_fi(mujallad: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<PathBuf> {
    let qaima = match std::fs::read_dir(mujallad) {
        Ok(qaima) => qaima,
        Err(sabab) => {
            // A denial is the state of every Windows machine, and nothing Taarib
            // could ever list lives behind it, so the scan is not incomplete by
            // its own standard — it is one named place that is never readable.
            // Any other refusal is a root whose packages this scan did not see.
            tanbihat.push(if sabab.kind() == std::io::ErrorKind::PermissionDenied {
                TanbihFahs::jadeed(
                    MUARRIF,
                    mujallad.display().to_string(),
                    "Windows denies listing this package folder. That is how Microsoft ships it: \
                     WindowsApps is owned by TrustedInstaller and grants read access to nobody, \
                     administrators included. Games installed to other drives are still found.",
                )
            } else {
                TanbihFahs::fahras(
                    MUARRIF,
                    mujallad.display().to_string(),
                    format!("cannot list this package folder ({sabab})"),
                )
            });
            return Vec::new();
        },
    };

    let mut ruzam = Vec::new();
    for madkhal in qaima.flatten() {
        let masar = madkhal.path();
        if !masar.is_dir() {
            continue;
        }
        if masar.join(ISM_BAYAN).is_file() {
            ruzam.push(masar);
            continue;
        }
        let dakhili = masar.join("Content");
        if dakhili.join(ISM_BAYAN).is_file() {
            ruzam.push(dakhili);
        }
    }
    ruzam.sort();
    ruzam
}

// ---------------------------------------------------------------------------
// AppxManifest.xml
// ---------------------------------------------------------------------------

/// One `<Application>` entry inside a package manifest.
///
/// A package can publish several: a game and its editor, a game and its
/// dedicated server, a game and a settings tool. They share one identity and
/// one directory, so they are one library entry, and the entry's executable is
/// chosen from among them rather than from the first one the file happens to
/// list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TatbeeqRuzma {
    /// The `Id` attribute, unique inside the package.
    muarrif: Option<String>,
    /// The `Executable` attribute, relative to the package root.
    tanfidhi: Option<String>,
    /// The `EntryPoint` attribute. `windows.fullTrustApplication` is the
    /// declaration that this application is *not* containerised.
    madkhal: Option<String>,
    /// The display name from `<uap:VisualElements>`.
    ism_azhar: Option<String>,
    /// `Square150x150Logo`.
    shiar_murabba: Option<String>,
    /// `Wide310x150Logo`.
    shiar_areed: Option<String>,
    /// `Square310x310Logo`.
    shiar_kabir: Option<String>,
}

/// A package manifest, reduced to what discovery needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BayanRuzma {
    /// `<Identity Name>` — the first half of the package family name.
    ism_huwiya: Option<String>,
    /// `<Identity Publisher>` — the publisher's distinguished name, which the
    /// second half of the package family name is the hash of.
    nashir: Option<String>,
    /// `<Identity Version>`.
    isdar: Option<String>,
    /// `<Identity ProcessorArchitecture>`.
    mimariya: Option<String>,
    /// `<Properties><DisplayName>`.
    ism_azhar: Option<String>,
    /// `<Properties><Logo>`.
    shiar: Option<String>,
    /// Every `<Application>`, in document order.
    tatbiqat: Vec<TatbeeqRuzma>,
    /// Every `<TargetDeviceFamily Name>`.
    aailat_ajhiza: Vec<String>,
    /// Every capability the package declares, however it is namespaced.
    qudurat: Vec<String>,
    /// Every `<Extension Category>`, which is where the desktop bridge and the
    /// mutable-directory declarations live.
    imtidadat: Vec<String>,
}

impl BayanRuzma {
    /// Whether the manifest declares any capability by that name.
    fn laha_qudra(&self, ism: &str) -> bool {
        self.qudurat.iter().any(|qudra| qudra.eq_ignore_ascii_case(ism))
    }

    /// The application entry whose executable is most likely to be the game.
    ///
    /// The shim is skipped when anything else is on offer, because
    /// `gamelaunchhelper.exe` is the same file in every Xbox package and
    /// probing it would tell Phase 5 about Microsoft's launcher rather than
    /// about the game.
    fn tatbeeq_mufaddal(&self) -> Option<&TatbeeqRuzma> {
        self.tatbiqat
            .iter()
            .find(|tatbeeq| {
                tatbeeq.tanfidhi.as_deref().is_some_and(|ism| !huwa_shim(ism))
            })
            .or_else(|| self.tatbiqat.first())
    }
}

/// Whether an executable name is the Xbox container shim.
fn huwa_shim(tanfidhi: &str) -> bool {
    tanfidhi
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(tanfidhi)
        .eq_ignore_ascii_case(SHIM_TASHGHIL)
}

/// Reads and parses a package manifest.
///
/// Element and attribute names are matched on their **local** part, so the
/// manifest's namespace prefixes — `uap:`, `uap10:`, `desktop6:`, `rescap:`,
/// which change with every Windows SDK revision — do not have to be enumerated
/// and a package built against a newer schema still reads.
fn iqra_bayan(masar: &Path) -> Result<BayanRuzma, String> {
    let bayt = std::fs::read(masar).map_err(|sabab| {
        if sabab.kind() == std::io::ErrorKind::PermissionDenied {
            "Windows denies reading this package's manifest. Packages are installed with \
             restricted ACLs and this is the normal result even for an administrator, so the \
             package is skipped rather than treated as broken."
                .to_owned()
        } else {
            format!("cannot read the package manifest ({sabab})")
        }
    })?;
    let nass = String::from_utf8_lossy(&bayt);
    hallil_bayan(bila_bom(nass.as_ref()))
}

/// Strips a UTF-8 byte order mark, which the packaging tools sometimes emit and
/// which no XML reader accepts as content.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}

/// Parses manifest text.
fn hallil_bayan(nass: &str) -> Result<BayanRuzma, String> {
    let mut qari = quick_xml::Reader::from_str(nass);
    // Not `trim_text(true)`: an entity reference arrives as its own event, so a
    // value is trimmed once it is whole rather than at every seam inside it.
    qari.config_mut().trim_text(false);

    let mut bayan = BayanRuzma::default();
    let mut fi_khasais = false;
    let mut amud_khasais: Option<String> = None;
    let mut madad = String::new();

    loop {
        match qari.read_event() {
            Ok(quick_xml::events::Event::Eof) => break,
            Ok(quick_xml::events::Event::Start(marka)) => {
                let ism = ism_mahalli(marka.local_name().as_ref());
                if ism == "Properties" {
                    fi_khasais = true;
                } else if fi_khasais {
                    amud_khasais = Some(ism.clone());
                }
                sajjil_marka(&mut bayan, &ism, &marka);
                madad.clear();
            },
            Ok(quick_xml::events::Event::Empty(marka)) => {
                let ism = ism_mahalli(marka.local_name().as_ref());
                sajjil_marka(&mut bayan, &ism, &marka);
            },
            Ok(quick_xml::events::Event::Text(nass_marka)) => {
                madad.push_str(&nass_marka.xml10_content());
            },
            Ok(quick_xml::events::Event::GeneralRef(marja)) => {
                match taarib_usus::kayanat::hall_marja(&marja) {
                    Some(hall) => madad.push_str(&hall),
                    None => madad.push_str(&taarib_usus::kayanat::nass_marja(&marja)),
                }
            },
            Ok(quick_xml::events::Event::End(marka)) => {
                let ism = ism_mahalli(marka.local_name().as_ref());
                let qeema = std::mem::take(&mut madad).trim().to_owned();
                if fi_khasais
                    && !qeema.is_empty()
                    && amud_khasais.as_deref() == Some(ism.as_str())
                {
                    match ism.as_str() {
                        "DisplayName" => bayan.ism_azhar = Some(qeema),
                        "Logo" => bayan.shiar = Some(qeema),
                        _ => {},
                    }
                }
                if ism == "Properties" {
                    fi_khasais = false;
                }
                if amud_khasais.as_deref() == Some(ism.as_str()) {
                    amud_khasais = None;
                }
            },
            Ok(_) => {},
            Err(khata) => {
                return Err(format!(
                    "the package manifest is not well-formed XML at byte {}: {khata}",
                    qari.buffer_position()
                ));
            },
        }
    }

    if bayan.ism_huwiya.is_none() {
        return Err("the package manifest declares no <Identity Name>, so the package has no \
                    family name and cannot be identified"
            .to_owned());
    }
    Ok(bayan)
}

/// Records whatever one element contributes.
fn sajjil_marka(bayan: &mut BayanRuzma, ism: &str, marka: &quick_xml::events::BytesStart<'_>) {
    match ism {
        "Identity" => {
            bayan.ism_huwiya = khasisa(marka, "Name");
            bayan.nashir = khasisa(marka, "Publisher");
            bayan.isdar = khasisa(marka, "Version");
            bayan.mimariya = khasisa(marka, "ProcessorArchitecture");
        },
        "Application" => bayan.tatbiqat.push(TatbeeqRuzma {
            muarrif: khasisa(marka, "Id"),
            tanfidhi: khasisa(marka, "Executable"),
            madkhal: khasisa(marka, "EntryPoint"),
            ..TatbeeqRuzma::default()
        }),
        "VisualElements" => {
            if let Some(tatbeeq) = bayan.tatbiqat.last_mut() {
                tatbeeq.ism_azhar = khasisa(marka, "DisplayName");
                tatbeeq.shiar_murabba = khasisa(marka, "Square150x150Logo");
                tatbeeq.shiar_areed = khasisa(marka, "Wide310x150Logo");
                tatbeeq.shiar_kabir = khasisa(marka, "Square310x310Logo");
            }
        },
        "TargetDeviceFamily" => {
            if let Some(qeema) = khasisa(marka, "Name") {
                bayan.aailat_ajhiza.push(qeema);
            }
        },
        "Capability" | "DeviceCapability" | "CustomCapability" => {
            if let Some(qeema) = khasisa(marka, "Name") {
                bayan.qudurat.push(qeema);
            }
        },
        "Extension" => {
            if let Some(qeema) = khasisa(marka, "Category") {
                bayan.imtidadat.push(qeema);
            }
        },
        _ => {},
    }
}

/// An element or attribute's local name as text, lossily — a name that is not
/// UTF-8 is not a name this manifest schema defines, and matching it against
/// nothing is the correct outcome.
fn ism_mahalli(ism: &str) -> String {
    ism.to_owned()
}

/// One attribute of an element, by local name.
fn khasisa(marka: &quick_xml::events::BytesStart<'_>, ism: &str) -> Option<String> {
    for natija in marka.attributes() {
        let Ok(khasisa) = natija else { continue };
        if khasisa.key.local_name().as_ref() != ism {
            continue;
        }
        if let Ok(qeema) = khasisa.normalized_value(quick_xml::XmlVersion::Implicit1_0) {
            let qeema = qeema.trim();
            if !qeema.is_empty() {
                return Some(qeema.to_owned());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// identity
// ---------------------------------------------------------------------------

/// The alphabet a package publisher id is written in: Crockford base32, digits
/// first, and without the four letters that can be misread as a digit — there
/// is no `i`, no `l`, no `o` and no `u` in it.
///
/// The order is part of the encoding, not a presentation choice.
/// `8wekyb3d8bbwe` begins with a digit because the first five bits of
/// Microsoft's publisher hash are `01000`, which is index eight, and index
/// eight in this alphabet is `8`.
const HURUF_MUARRIF_NASHIR: &str = "0123456789abcdefghjkmnpqrstvwxyz";

/// The length of every package publisher id.
const TUL_MUARRIF_NASHIR: usize = 13;

/// The bits one base32 character carries.
const BITAT_HARF: usize = 5;

/// Selects one character's worth of bits.
const QINAA_HARF: u64 = (1 << BITAT_HARF) - 1;

/// How much of the digest a publisher id is built from. Everything past the
/// eighth byte is discarded.
const BAYTAT_BASMA: usize = 8;

/// That same prefix measured in bits, which is what the encoder counts in.
const BITAT_BASMA: usize = 64;

/// The package family name, and any warning raised on the way to it.
///
/// The family name is `<Identity Name>_<PublisherId>`, and it is what the
/// registry shards Xbox patches on. Both halves come out of the manifest: the
/// name is the `<Identity Name>` attribute, and the publisher id is derived
/// from the `<Identity Publisher>` distinguished name by [`muarrif_nashir`].
///
/// Deriving it rather than reading it back out of the directory name is what
/// makes a secondary-drive install work. Under `WindowsApps` a package's
/// directory is its **full** name — `Name_Version_Architecture__PublisherId` —
/// so the publisher id can simply be read off the end of it; on any other drive
/// the directory is named after the *game*, carries no identity at all, and
/// there is nothing there to read. The hash answers in both places.
///
/// The directory name is still parsed, for two reasons. It is a cross-check:
/// where it does carry a publisher id, that id and the derived one are two
/// independent statements about one package, and when they differ the derived
/// one wins — it is the package's own — while the disagreement becomes a
/// warning, because one package name with two publisher ids is two packages.
/// And it is the answer for a manifest whose `<Identity>` omits `Publisher`
/// altogether, which is malformed but still recoverable as long as the folder
/// is named after the package; that recovery is silent, because the family
/// name it produces is the right one.
///
/// The one case left with no answer is both at once: no publisher declared and
/// a folder named after something else.
fn huwiyat_ruzma(masar: &Path, bayan: &BayanRuzma) -> (String, Option<TanbihFahs>) {
    let ism_huwiya = bayan.ism_huwiya.clone().unwrap_or_default();
    let mudawwan = asmaa_mujalladat(masar)
        .into_iter()
        .find_map(|murashah| muarrif_nashir_min_ism_kamil(&murashah, &ism_huwiya));
    let mushtaqq = bayan.nashir.as_deref().map(muarrif_nashir);

    match (mushtaqq, mudawwan) {
        (Some(mushtaqq), Some(mudawwan)) if !mudawwan.eq_ignore_ascii_case(&mushtaqq) => {
            let tanbih = TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                format!(
                    "this package's folder is named for {ism_huwiya} with publisher id \
                     {mudawwan}, while the publisher its own manifest declares hashes to \
                     {mushtaqq}. The manifest is believed, since it is the package's own \
                     identity, and the game is listed as {ism_huwiya}_{mushtaqq}. One package \
                     name carrying two publisher ids means two different packages, so this \
                     folder is likely left over from an earlier install of the same title."
                ),
            );
            (format!("{ism_huwiya}_{mushtaqq}"), Some(tanbih))
        },
        // The derived id when there is one — including when the folder agrees
        // with it — and the folder's own only when there is not.
        (Some(nashir), _) | (None, Some(nashir)) => (format!("{ism_huwiya}_{nashir}"), None),
        (None, None) => {
            let tanbih = TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                format!(
                    "the manifest for {ism_huwiya} declares no <Identity Publisher>, so there is \
                     no distinguished name to hash into a publisher id, and this package's \
                     folder is not named after it either. The game is listed under its package \
                     name alone, so a patch published against its full package family name will \
                     not be offered for it."
                ),
            );
            (ism_huwiya, Some(tanbih))
        },
    }
}

/// Derives a package publisher id from the publisher's distinguished name.
///
/// This is the computation the Microsoft Store itself performs when it builds a
/// package family name, reproduced step for step, because an id that differs
/// from the store's in a single character names no package and matches no
/// patch:
///
/// 1. The distinguished name is taken verbatim — every space, every comma — and
///    encoded as UTF-16 little-endian with no byte order mark. That is
///    `Encoding.Unicode` on the Windows side, and it is why an ASCII-looking
///    string is hashed as two bytes per character.
/// 2. Those bytes are hashed with SHA-256.
/// 3. The first eight bytes of the digest are kept, most significant first, and
///    the remaining twenty-four are thrown away.
/// 4. The 64 bits that survive are written as thirteen base32 characters.
///
/// Step four is the one worth spelling out, because thirteen characters carry
/// **65** bits and the digest supplies 64. The missing bit is not borrowed from
/// further into the digest and it is not prepended: a single zero bit is
/// appended on the *right*, and the resulting 65 bits are then split into
/// thirteen groups of five, most significant group first. So the first twelve
/// characters come straight out of the digest, and the thirteenth carries the
/// four bits the digest has left over **shifted up by one**, with the appended
/// zero sitting underneath them.
///
/// Encoding those last four bits where they lie is the mistake this comment
/// exists to prevent. It halves the final character's index and yields an
/// identifier that is right in twelve places out of thirteen, looks exactly
/// like a publisher id, and matches nothing at all.
///
/// The vector to check the whole of it against is public knowledge. Microsoft's
/// `CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US`
/// hashes to `471d3f2c6d42d7c7…`; its first five bits are `01000`, index eight,
/// character `8`, and its last four are `0111`, which shifted up by one is
/// index fourteen, character `e`. The id is `8wekyb3d8bbwe` — the one written
/// into every Microsoft package directory name on every Windows machine. Take
/// the last four bits where they lie instead and the same input produces
/// `8wekyb3d8bbw7`: twelve characters of a real publisher id, and one that
/// belongs to nobody. The second vector, for a machine that has both on it, is
/// `CN=Microsoft Windows, O=Microsoft Corporation, L=Redmond, S=Washington, C=US`,
/// which is `cw5n1h2txyewy`.
fn muarrif_nashir(nashir: &str) -> String {
    let mudkhal: Vec<u8> = nashir.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let basma = sha2::Sha256::digest(mudkhal);
    let thamaniya = basma
        .iter()
        .take(BAYTAT_BASMA)
        .fold(0_u64, |majmu, bayt| (majmu << u8::BITS) | u64::from(*bayt));

    let mut muarrif = String::with_capacity(TUL_MUARRIF_NASHIR);
    let mut baqi = BITAT_BASMA;
    while baqi >= BITAT_HARF {
        baqi = baqi.saturating_sub(BITAT_HARF);
        muarrif.push(harf_qaeda32(thamaniya >> baqi));
    }
    // `baqi` is four here, because 64 is not a multiple of five: these are the
    // bits no whole character consumed. Shifting the digest up by the one bit
    // it is short lifts them into the top of the final group and slides the
    // appended zero in beneath them, which is what padding to 65 bits means.
    muarrif.push(harf_qaeda32(thamaniya << BITAT_HARF.saturating_sub(baqi)));
    muarrif
}

/// One character of the publisher id alphabet, for the low five bits of a
/// value.
///
/// Total by construction. The mask holds the index inside an alphabet of
/// exactly thirty-two characters, and the index reaches `u8` through the masked
/// value's own last byte rather than through a cast, so the fallback cannot be
/// reached. It is written as a fallback rather than an assertion because
/// nothing in this workspace may panic, and `0` — the alphabet's own first
/// character — keeps the result a well-formed publisher id rather than
/// something every later stage would have to be taught to reject.
fn harf_qaeda32(qeema: u64) -> char {
    let [.., fihris] = (qeema & QINAA_HARF).to_be_bytes();
    HURUF_MUARRIF_NASHIR.chars().nth(usize::from(fihris)).unwrap_or('0')
}

/// The directory names worth testing as a package full name: the package
/// directory itself and, when that is the `Content` folder of a secondary-drive
/// install, its parent.
fn asmaa_mujalladat(masar: &Path) -> Vec<String> {
    let mut asmaa = Vec::new();
    if let Some(ism) = masar.file_name().map(|ism| ism.to_string_lossy().into_owned()) {
        asmaa.push(ism);
    }
    if let Some(ism) =
        masar.parent().and_then(Path::file_name).map(|ism| ism.to_string_lossy().into_owned())
    {
        asmaa.push(ism);
    }
    asmaa
}

/// The publisher id a package full name carries, when the name really is one
/// and really belongs to this identity.
///
/// Both halves are checked, not just the shape: a directory whose first segment
/// is some other package's name is not this package's full name, and taking a
/// publisher id out of it would give one game another game's identity. That
/// check is also what makes a disagreement with the derived id worth reporting
/// — the folder and the manifest have already been shown to name the same
/// package, so a publisher id that differs is a genuine conflict rather than a
/// stale neighbour.
fn muarrif_nashir_min_ism_kamil(ism_kamil: &str, ism_huwiya: &str) -> Option<String> {
    let awwal = ism_kamil.split('_').next()?;
    if ism_huwiya.is_empty() || !awwal.eq_ignore_ascii_case(ism_huwiya) {
        return None;
    }
    let akhir = ism_kamil.rsplit('_').next()?;
    if akhir.len() != TUL_MUARRIF_NASHIR
        || !akhir.chars().all(|harf| HURUF_MUARRIF_NASHIR.contains(harf))
    {
        return None;
    }
    Some(akhir.to_owned())
}

// ---------------------------------------------------------------------------
// package → game
// ---------------------------------------------------------------------------

/// Turns one package directory into a discovered game.
///
/// Returns nothing when the manifest cannot be read at all — which on Windows
/// is usually an ACL, not a fault — after recording why.
fn luba_min_ruzma(masar: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Option<LubaMuktashafa> {
    let bayan = match iqra_bayan(&masar.join(ISM_BAYAN)) {
        Ok(bayan) => bayan,
        Err(sabab) => {
            tanbihat.push(TanbihFahs::jadeed(MUARRIF, masar.display().to_string(), sabab));
            return None;
        },
    };

    let (aila, tanbih) = huwiyat_ruzma(masar, &bayan);
    if let Some(tanbih) = tanbih {
        tanbihat.push(tanbih);
    }

    let tatbeeq = bayan.tatbeeq_mufaddal().cloned().unwrap_or_default();
    let ism = ism_azhar(masar, &bayan, &tatbeeq);

    let tanfidhi = tatbeeq
        .tanfidhi
        .as_deref()
        .and_then(|nisbi| dakhil(masar, &nisbi.replace('\\', std::path::MAIN_SEPARATOR_STR)).ok())
        .filter(|masar_tanfidhi| masar_tanfidhi.is_file());
    if tanfidhi.is_none()
        && let Some(nisbi) = tatbeeq.tanfidhi.as_deref()
    {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            format!(
                "the manifest names {nisbi} as this package's executable and it is not on disk; \
                 the package may be staged but not fully downloaded, or its files may be behind \
                 an ACL Taarib cannot read"
            ),
        ));
    }

    let mut simat = Vec::new();
    if ghayr_luba(&bayan) {
        simat.push(SimatLuba::LaysatLuba(
            "a Microsoft Store framework or system package, not a game".to_owned(),
        ));
    }
    if let Some(dalil) = dalil_hawiya(&bayan, &tatbeeq) {
        simat.push(SimatLuba::TabaqatTawafuq(dalil));
    }

    let bina_manassa = match (bayan.isdar.as_deref(), bayan.mimariya.as_deref()) {
        (Some(isdar), Some(mimariya)) => Some(format!("{isdar}-{mimariya}")),
        (Some(isdar), None) => Some(isdar.to_owned()),
        (None, _) => None,
    };

    Some(LubaMuktashafa {
        masdar: MasdarLuba::Xbox(aila),
        hala_matjar: None,
        ism,
        jidhr: masar.to_path_buf(),
        tanfidhi,
        hajm: 0,
        bina_manassa,
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: suwar_ruzma(masar, &bayan, &tatbeeq),
        khiyarat_tashghil: None,
        // A package whose manifest read and whose executable is present is a
        // complete install. MSIX has no partial state Taarib can observe from
        // the outside: a package being downloaded has no manifest yet.
        muktamila: true,
        simat,
    })
}

/// The name to show.
///
/// `<Properties><DisplayName>` is very often `ms-resource:AppDisplayName`, an
/// indirection into the package's compiled resource index that only the Windows
/// resource loader can resolve. Rather than shipping a `.pri` reader for it,
/// the name falls back through the sources that carry real text: the
/// application's own visual elements, the install folder's name — which for a
/// secondary-drive install *is* the game's title — and finally the last dotted
/// segment of the package identity.
fn ism_azhar(masar: &Path, bayan: &BayanRuzma, tatbeeq: &TatbeeqRuzma) -> String {
    let mubashir = |qeema: &Option<String>| -> Option<String> {
        qeema
            .as_deref()
            .map(str::trim)
            .filter(|nass| !nass.is_empty() && !nass.starts_with("ms-resource:"))
            .map(str::to_owned)
    };

    if let Some(ism) = mubashir(&bayan.ism_azhar) {
        return ism;
    }
    if let Some(ism) = mubashir(&tatbeeq.ism_azhar) {
        return ism;
    }
    // `Content` names the layout, not the game, so its parent is the title.
    let mut mujallad = masar;
    if mujallad.file_name().is_some_and(|ism| ism.eq_ignore_ascii_case("Content"))
        && let Some(walid) = mujallad.parent()
    {
        mujallad = walid;
    }
    if let Some(ism) = mujallad.file_name().map(|ism| ism.to_string_lossy().into_owned())
        && !ism.contains('_')
    {
        return ism;
    }
    bayan
        .ism_huwiya
        .as_deref()
        .and_then(|huwiya| huwiya.rsplit('.').next())
        .unwrap_or("Microsoft Store package")
        .to_owned()
}

/// Whether this package is infrastructure rather than a game.
fn ghayr_luba(bayan: &BayanRuzma) -> bool {
    if bayan.tatbiqat.is_empty() {
        return true;
    }
    bayan.ism_huwiya.as_deref().is_some_and(|huwiya| {
        BIDAYAT_GHAYR_LUBA.iter().any(|bidaya| {
            huwiya.len() >= bidaya.len()
                && huwiya.get(..bidaya.len()).is_some_and(|juz| juz.eq_ignore_ascii_case(bidaya))
        })
    })
}

/// The evidence that this package runs inside a container, or nothing when it
/// plainly does not.
///
/// Four independent signals, reported together rather than reduced to a
/// boolean, because Phase 15 needs to know *which* of them holds: a package
/// that is merely started through the shim is installed differently from one
/// that has no full-trust declaration at all.
fn dalil_hawiya(bayan: &BayanRuzma, tatbeeq: &TatbeeqRuzma) -> Option<String> {
    let mut dalail: Vec<&str> = Vec::new();

    let thiqa_kamila = bayan.laha_qudra("runFullTrust")
        || bayan
            .tatbiqat
            .iter()
            .any(|wahid| wahid.madkhal.as_deref().is_some_and(|madkhal| {
                madkhal.eq_ignore_ascii_case("Windows.FullTrustApplication")
            }))
        || bayan
            .imtidadat
            .iter()
            .any(|fia| fia.eq_ignore_ascii_case("windows.fullTrustProcess"));
    if !thiqa_kamila {
        dalail.push(
            "the manifest declares no full-trust entry point, so the game runs inside an \
             AppContainer",
        );
    }

    if tatbeeq.tanfidhi.as_deref().is_some_and(huwa_shim) {
        dalail.push(
            "the package executable is the Xbox launch shim rather than the game's own binary",
        );
    }

    if bayan
        .imtidadat
        .iter()
        .any(|fia| fia.eq_ignore_ascii_case("windows.mutablePackageDirectories"))
    {
        dalail.push("the package declares mutable directories, so its files are virtualised");
    }

    let universal = bayan
        .aailat_ajhiza
        .iter()
        .any(|aila| aila.eq_ignore_ascii_case("Windows.Universal"));
    let maktabi = bayan
        .aailat_ajhiza
        .iter()
        .any(|aila| aila.eq_ignore_ascii_case("Windows.Desktop"));
    if universal && !maktabi {
        dalail.push("the package targets the universal device family only");
    }

    (!dalail.is_empty()).then(|| dalail.join("; "))
}

/// The package's own logos, when they are on disk.
fn suwar_ruzma(masar: &Path, bayan: &BayanRuzma, tatbeeq: &TatbeeqRuzma) -> MasadirSuwar {
    let hall = |nisbi: Option<&str>| -> Option<MasdarSura> {
        hall_shiar(masar, nisbi?).map(MasdarSura::Malaf)
    };
    MasadirSuwar {
        ghilaf: hall(tatbeeq.shiar_kabir.as_deref()).or_else(|| hall(bayan.shiar.as_deref())),
        batl: hall(tatbeeq.shiar_areed.as_deref()),
        shiar: hall(tatbeeq.shiar_murabba.as_deref()).or_else(|| hall(bayan.shiar.as_deref())),
    }
}

/// Resolves a manifest logo reference to a file that exists.
///
/// MSIX logo attributes name an *unqualified* asset: the manifest says
/// `Assets\Logo.png` and the package ships `Assets\Logo.scale-200.png`,
/// `Assets\Logo.scale-400.png` and nothing called `Logo.png` at all. The exact
/// name is tried first, and the scale-qualified variants are searched only when
/// it is absent, taking the largest — the library grid downscales far better
/// than it upscales.
fn hall_shiar(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munaqqa = nisbi.replace('\\', std::path::MAIN_SEPARATOR_STR);
    let mubashir = dakhil(jidhr, &munaqqa).ok()?;
    if mubashir.is_file() {
        return Some(mubashir);
    }

    let mujallad = mubashir.parent()?;
    let jidhr_ism = mubashir.file_stem()?.to_string_lossy().into_owned();
    let imtidad = mubashir.extension().map(|q| q.to_string_lossy().into_owned())?;
    let bidaya = format!("{jidhr_ism}.");
    let nihaya = format!(".{imtidad}");

    let mut murashahat: Vec<PathBuf> = std::fs::read_dir(mujallad)
        .ok()?
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            masar.file_name().is_some_and(|ism| {
                let ism = ism.to_string_lossy();
                ism.starts_with(&bidaya) && ism.ends_with(&nihaya) && ism.contains(".scale-")
            })
        })
        .collect();
    murashahat.sort();
    murashahat.pop()
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    #[test]
    fn jidhr_al_ruzam_min_mujallad_al_baramij_al_asli() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let baramij86 = masrah.path().join("Program Files (x86)");
        let baramij = masrah.path().join("Program Files");
        let jidhr = baramij.join("WindowsApps");
        fs::create_dir_all(&jidhr)?;
        // Deliberately present under the 32-bit directory too, so that an
        // adapter taking the head of the list would find *something* and the
        // assertion below would still catch it.
        fs::create_dir_all(baramij86.join("WindowsApps"))?;

        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.mujalladat_baramij = vec![baramij86, baramij];
        assert_eq!(mujallad_windows_apps(&siyaq), Some(jidhr.clone()));
        assert_eq!(MatjarXbox::jadeed().mawqi(&siyaq), Some(jidhr));
        Ok(())
    }

    #[test]
    fn mujallad_wahid_yujib_an_al_sualayn() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let baramij = masrah.path().join("Program Files");

        // A 32-bit Windows reports the same directory in both variables, so the
        // context deduplicates to one entry — which is then both ends of the
        // list, and the right answer to either question asked of it.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.mujalladat_baramij = vec![baramij.clone()];
        assert_eq!(mujallad_windows_apps(&siyaq), Some(baramij.join("WindowsApps")));
        assert_eq!(siyaq.mujallad_baramij_x86(), siyaq.mujallad_baramij_asli());
        Ok(())
    }

    #[test]
    fn bila_mujalladat_baramij_la_jidhr_nizam() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // The old resolver answered `C:\Program Files\WindowsApps` on every
        // machine that asked, whatever drive its Windows was on.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        assert_eq!(mujallad_windows_apps(&siyaq), None);
        assert_eq!(MatjarXbox::jadeed().mawqi(&siyaq), None);
        Ok(())
    }
}
