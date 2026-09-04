//! المنصّة — the operating system, the architecture, the process list, and the
//! compatibility layers a game can be running behind.
//!
//! Taarib installs into three operating systems and injects into processes that
//! may be a different architecture from itself, may be a Windows program running
//! under Proton on Linux, and may be an Intel program running under Rosetta on
//! Apple Silicon. Every one of those facts changes which library is loaded and
//! where files are written, so all of them are answered here rather than guessed
//! at the call site.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::khata::{
    Khata, Khutwa, MasarMatlub, Natija, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use crate::khata_min;
use crate::masarat::qira;

/// The operating system Taarib is running on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NizamTashghil {
    /// Windows 10 1809 and newer.
    Windows,
    /// Any current Linux distribution, including `SteamOS` on the Steam Deck.
    Linux,
    /// macOS 12 and newer.
    Mac,
}

impl NizamTashghil {
    /// The system this build is running on.
    #[must_use]
    pub const fn hali() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }

    /// The extension an executable carries, empty on the Unix systems.
    #[must_use]
    pub const fn imtidad_tanfidh(self) -> &'static str {
        match self {
            Self::Windows => "exe",
            Self::Linux | Self::Mac => "",
        }
    }

    /// Builds the file name of a shared library from its base name.
    #[must_use]
    pub fn ism_maktaba(self, asas: &str) -> String {
        match self {
            Self::Windows => format!("{asas}.dll"),
            Self::Linux => format!("lib{asas}.so"),
            Self::Mac => format!("lib{asas}.dylib"),
        }
    }

    /// Whether path comparison on this system ignores case.
    ///
    /// It matters for real reasons, not pedantry: a patch manifest that lists
    /// `Data/Game.exe` and a game that ships `data/game.exe` are the same file
    /// on Windows and two different files on Linux, and an installer that gets
    /// this wrong either misses a backup or creates a duplicate.
    #[must_use]
    pub const fn hassas_lil_ahruf(self) -> bool {
        matches!(self, Self::Linux)
    }
}

/// A processor architecture, of Taarib or of a game process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Mimariya {
    /// 32-bit x86. Still extremely common in shipped games.
    X86,
    /// 64-bit x86.
    X8664,
    /// 64-bit ARM: Apple Silicon, Windows on ARM, ARM handhelds.
    Aarch64,
}

impl Mimariya {
    /// The architecture this build was compiled for.
    #[must_use]
    pub const fn hali() -> Self {
        if cfg!(target_arch = "x86_64") {
            Self::X8664
        } else if cfg!(target_arch = "aarch64") {
            Self::Aarch64
        } else {
            Self::X86
        }
    }

    /// The directory name Taarib stores this architecture's payloads under.
    #[must_use]
    pub const fn mujallad(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X8664 => "x64",
            Self::Aarch64 => "arm64",
        }
    }

    /// Whether a Taarib library of this architecture can be loaded into a
    /// process of that architecture. Injection is architecture-exact: there is
    /// no thunk that lets a 64-bit library live inside a 32-bit process.
    #[must_use]
    pub fn yatawafaq_maa(self, akhar: Self) -> bool {
        self == akhar
    }
}

/// What a game process is running behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum BeeatTawafuq {
    /// The game is native to the operating system it runs on.
    Asli,
    /// A Windows game running under Proton on Linux.
    Proton {
        /// The Proton build name, as Steam reports it.
        isdar: String,
        /// The Wine prefix root — the directory containing `drive_c`.
        beea: PathBuf,
    },
    /// A Windows game running under plain Wine, Lutris, Bottles or Heroic.
    Wine {
        /// The Wine build, when it can be determined.
        isdar: Option<String>,
        /// The Wine prefix root.
        beea: PathBuf,
    },
    /// An Intel game running under Rosetta 2 on Apple Silicon.
    Rosetta,
}

impl BeeatTawafuq {
    /// The Wine prefix, when there is one.
    #[must_use]
    pub fn beea(&self) -> Option<&Path> {
        match self {
            Self::Proton { beea, .. } | Self::Wine { beea, .. } => Some(beea.as_path()),
            Self::Asli | Self::Rosetta => None,
        }
    }

    /// Whether the game runs as a Windows program regardless of the host,
    /// which decides whether the Windows-side framework payload is the one to
    /// install.
    #[must_use]
    pub const fn windows_dakhilan(&self) -> bool {
        matches!(self, Self::Proton { .. } | Self::Wine { .. })
    }
}

/// A running process, as Taarib needs to see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
pub struct Amaliya {
    /// The process identifier.
    pub raqm: u32,
    /// The executable's file name.
    pub ism: String,
    /// The executable's full path, when the system will disclose it.
    pub masar: Option<PathBuf>,
}

/// A sandbox that hands the application its own view of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Sunduq {
    /// A Flatpak sandbox.
    Flatpak,
    /// A confined snap.
    Snap,
    /// A container: Docker, Podman, LXC or `systemd-nspawn`.
    Hawiya,
}

impl Sunduq {
    /// The sandbox's name, for a refusal that has to say where it is running.
    ///
    /// Phrased to drop into "… inside {}": the two brands are proper nouns and
    /// the third is not, so it carries its own article.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Flatpak => "Flatpak",
            Self::Snap => "Snap",
            Self::Hawiya => "a container",
        }
    }

    /// The same, in Arabic.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::Flatpak => "فلاتباك",
            Self::Snap => "سناب",
            Self::Hawiya => "حاوية",
        }
    }
}

/// How much of the host's process table this build can actually read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum RuyatAmaliyat {
    /// The host's own process table, which is what every safety check assumes.
    Kamila,
    /// A private table: the sandbox's own processes and none of the host's.
    Maazula {
        /// The sandbox, which is always named — it is the marker that gave it
        /// away in the first place.
        sunduq: Sunduq,
    },
}

/// Lists running processes whose executable name matches, case-insensitively.
///
/// Used to notice that a game is already running before patching it, and to
/// find the process to inject into.
///
/// An empty result is only meaningful where [`ruyat_amaliyat`] reports
/// [`RuyatAmaliyat::Kamila`]: inside a sandbox this lists the sandbox's own
/// processes, so "nothing matched" is not "nothing is running". A caller that
/// is guarding an operation wants [`halat_tashghil`], which says so.
#[must_use]
pub fn amaliyat_bism(ism: &str) -> Vec<Amaliya> {
    let matlub = ism.to_ascii_lowercase();
    let mut nizam = sysinfo::System::new();
    nizam.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    nizam
        .processes()
        .values()
        .filter_map(|a| {
            let ism_amaliya = a.name().to_string_lossy().to_string();
            if ism_amaliya.to_ascii_lowercase() == matlub {
                Some(Amaliya {
                    raqm: a.pid().as_u32(),
                    ism: ism_amaliya,
                    masar: a.exe().map(Path::to_path_buf),
                })
            } else {
                None
            }
        })
        .collect()
}

/// The sandbox this build is running inside, when it is running inside one.
///
/// Worth answering because a sandbox changes what the process can *see*, not
/// only what it may touch. Flatpak and every container runtime give the
/// application its own PID namespace and its own `/proc`, so a check written as
/// "refuse while the game is running" answers "not running" for every process
/// on the machine and quietly stops guarding anything.
#[must_use]
pub fn fi_sunduq() -> Option<Sunduq> {
    #[cfg(target_os = "linux")]
    {
        sunduq_linux()
    }
    // Windows has no equivalent, and the macOS App Sandbox does not hide the
    // process table from `sysinfo`.
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// The Linux half of [`fi_sunduq`], where the markers actually exist.
#[cfg(target_os = "linux")]
#[expect(
    clippy::disallowed_methods,
    reason = "these are the sandbox runtimes' own markers rather than Taarib configuration, \
              and no configuration file can tell a process it is in a PID namespace"
)]
fn sunduq_linux() -> Option<Sunduq> {
    // Flatpak's documented marker: bind-mounted into every sandbox, and absent
    // outside one. FLATPAK_ID is the same fact by another route, and survives a
    // sandbox that remounts the root.
    if Path::new("/.flatpak-info").exists() || std::env::var_os("FLATPAK_ID").is_some() {
        return Some(Sunduq::Flatpak);
    }
    // Both, because SNAP alone is also exported to hooks that run outside the
    // confinement, where the process table is the host's.
    if std::env::var_os("SNAP").is_some() && std::env::var_os("SNAP_NAME").is_some() {
        return Some(Sunduq::Snap);
    }
    // `container` is what systemd, podman and lxc export into the payload; the
    // two files are what podman and docker leave in the root.
    if std::env::var_os("container").is_some()
        || Path::new("/run/.containerenv").exists()
        || Path::new("/.dockerenv").exists()
    {
        return Some(Sunduq::Hawiya);
    }
    None
}

/// How much of the host's process table this build can read.
///
/// The evidence is the sandbox markers of [`fi_sunduq`], deliberately and not
/// for want of something more direct. Comparing `/proc/self/ns/pid` against the
/// kernel's initial-namespace inode looks like the better measurement and is
/// not: WSL2 puts every distribution in its own PID namespace, so that test
/// calls an ordinary development machine sandboxed and then refuses every
/// guarded operation on it. A hand-rolled `unshare -p` with no marker is
/// therefore missed; a Flatpak, a snap and a container are not, and those are
/// the three ways this product could ever be packaged into one.
#[must_use]
pub fn ruyat_amaliyat() -> RuyatAmaliyat {
    fi_sunduq().map_or(RuyatAmaliyat::Kamila, |sunduq| RuyatAmaliyat::Maazula { sunduq })
}

/// Whether an executable is running — including "that cannot be known here".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HalatTashghil {
    /// At least one process is running it.
    Tashtaghil,
    /// The host's process table was read, and holds no such process.
    LaTashtaghil,
    /// The process table this build can read is not the host's, so the
    /// question has no answer here.
    GhayrMaaruf {
        /// The sandbox that made it unanswerable, so a refusal can say where
        /// it is rather than assert something it cannot know.
        sunduq: Sunduq,
    },
}

impl HalatTashghil {
    /// Whether an operation guarded by this check has to refuse.
    ///
    /// Every caller of this check is a safety refusal, and only one of the
    /// three states clears it: an unanswerable question is not a "no".
    #[must_use]
    pub const fn yamnaa(self) -> bool {
        !matches!(self, Self::LaTashtaghil)
    }

    /// The sandbox that made the question unanswerable, if one did.
    #[must_use]
    pub const fn sunduq(self) -> Option<Sunduq> {
        match self {
            Self::GhayrMaaruf { sunduq } => Some(sunduq),
            Self::Tashtaghil | Self::LaTashtaghil => None,
        }
    }
}

/// Whether any process is currently running that executable, when that can be
/// known.
///
/// A match is trustworthy wherever it is found — the sandbox's own processes
/// are real processes — so it is answered before the visibility question is
/// asked at all.
#[must_use]
pub fn halat_tashghil(ism: &str) -> HalatTashghil {
    if !amaliyat_bism(ism).is_empty() {
        return HalatTashghil::Tashtaghil;
    }
    match ruyat_amaliyat() {
        RuyatAmaliyat::Kamila => HalatTashghil::LaTashtaghil,
        RuyatAmaliyat::Maazula { sunduq } => HalatTashghil::GhayrMaaruf { sunduq },
    }
}

/// Whether this executable has to be treated as running.
///
/// Deliberately not "is it running": that question is [`halat_tashghil`] and it
/// has three answers. This one folds the third into `true`, because a guard
/// that cannot see the process table must refuse rather than wave the operation
/// through — on a Steam Deck under a sandbox, "Steam is not running" would be
/// wrong every single time. Outside a sandbox the answer is unchanged.
#[must_use]
pub fn tashtaghil(ism: &str) -> bool {
    halat_tashghil(ism).yamnaa()
}

/// Whether Taarib is running with administrative or root privileges.
///
/// Taarib never asks for elevation and never needs it for a normal install; it
/// reports the state so that a permission failure can say whether elevation is
/// the reason.
#[must_use]
pub fn salahiyat_mudeer() -> bool {
    #[cfg(unix)]
    {
        // SAFETY: geteuid takes no arguments, touches no memory, and cannot fail.
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Security::{
            GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
        };
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        let mut ramz = HANDLE::default();
        // SAFETY: GetCurrentProcess returns a pseudo-handle that is always valid,
        // and `ramz` is a live, correctly typed out-parameter.
        let maftuh = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut ramz) };
        if maftuh.is_err() {
            return false;
        }
        let mut irtifa = TOKEN_ELEVATION::default();
        let mut hajm = 0u32;
        // SAFETY: the buffer is a live TOKEN_ELEVATION of exactly the size passed,
        // matching the TokenElevation information class.
        let natija = unsafe {
            GetTokenInformation(
                ramz,
                TokenElevation,
                Some((&raw mut irtifa).cast()),
                u32::try_from(size_of::<TOKEN_ELEVATION>()).unwrap_or(0),
                &raw mut hajm,
            )
        };
        natija.is_ok() && irtifa.TokenIsElevated != 0
    }
}

/// Free space, in bytes, on the volume holding `masar`.
///
/// Checked before a download, before a backup, and before an install, because
/// running out of disk halfway through writing a game's files is the one
/// failure that leaves a game unplayable.
///
/// # Errors
///
/// Fails when the volume cannot be queried, which normally means the path does
/// not exist yet or is on a disconnected network share.
pub fn masaha_mutaha(masar: &Path) -> Natija<u64> {
    #[cfg(windows)]
    {
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::HSTRING;

        let nass = HSTRING::from(masar.as_os_str());
        let mut mutah = 0u64;
        // SAFETY: `nass` is a live null-terminated wide string for the lifetime of
        // the call, and `mutah` is a live out-parameter of the required width.
        let natija = unsafe {
            GetDiskFreeSpaceExW(&nass, Some(&raw mut mutah), None, None)
        };
        natija.map(|()| mutah).map_err(|_| {
            Khata::min_tafsir(&KhataManassa::TaadhurQiyasMasaha { masar: masar.to_path_buf() })
        })
    }
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt as _;

        let nass = CString::new(masar.as_os_str().as_bytes()).map_err(|_| {
            Khata::min_tafsir(&KhataManassa::TaadhurQiyasMasaha { masar: masar.to_path_buf() })
        })?;
        // SAFETY: statvfs is a plain-old-data struct of integers, for which an
        // all-zero bit pattern is a valid value; the call overwrites it entirely.
        let mut ihsaa: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `nass` is a valid null-terminated path and `ihsaa` is a live,
        // correctly sized statvfs the call fills in.
        let natija = unsafe { libc::statvfs(nass.as_ptr(), &raw mut ihsaa) };
        if natija == 0 {
            // The two fields are not the same width on every unix: Darwin's
            // `f_bavail` is a 32-bit `fsblkcnt_t` beside a 64-bit `f_frsize`,
            // and 32-bit Linux is the other way round. `u64::from` widens
            // whichever is narrow and is a no-op where both are already 64-bit.
            Ok(u64::from(ihsaa.f_bavail).saturating_mul(u64::from(ihsaa.f_frsize)))
        } else {
            Err(Khata::min_tafsir(&KhataManassa::TaadhurQiyasMasaha {
                masar: masar.to_path_buf(),
            }))
        }
    }
}

/// Reads an executable's architecture from its own header.
///
/// PE, ELF and Mach-O are all read directly here rather than through an object
/// file library, because this answer is needed before any heavier machinery
/// exists — it decides which Taarib library can even be loaded into the game —
/// and because the three headers involved are a few well-specified bytes each.
///
/// # Errors
///
/// Fails when the file cannot be read, is too short to carry a header, or
/// carries a machine type Taarib cannot inject into.
pub fn mimariyat_malaf(masar: &Path) -> Natija<Mimariya> {
    let bayt = qira(masar)?;

    // ELF: magic, then EI_CLASS at offset 4 (1 = 32-bit, 2 = 64-bit), then the
    // machine field at offset 18.
    if bayt.get(..4) == Some(b"\x7fELF") {
        let sinf = bayt.get(4).copied().unwrap_or(0);
        let jihaz = u16le(&bayt, 18).unwrap_or(0);
        return match (sinf, jihaz) {
            (1, _) => Ok(Mimariya::X86),
            (2, 0x00B7) => Ok(Mimariya::Aarch64),
            (2, _) => Ok(Mimariya::X8664),
            _ => Err(Khata::min_tafsir(&KhataManassa::MimariyaMajhula {
                masar: masar.to_path_buf(),
                jihaz: u32::from(jihaz),
            })),
        };
    }

    // Mach-O: the magic itself carries the width, and fat binaries are read as
    // the widest slice they contain since that is the one macOS will run.
    if let Some(sihr) = u32le(&bayt, 0) {
        match sihr {
            0xFEED_FACE | 0xCEFA_EDFE => return Ok(Mimariya::X86),
            0xFEED_FACF | 0xCFFA_EDFE => {
                let naw = u32le(&bayt, 4).unwrap_or(0);
                return Ok(if naw == 0x0100_000C { Mimariya::Aarch64 } else { Mimariya::X8664 });
            }
            // Universal binaries store their slices big-endian regardless of host.
            0xCAFE_BABE | 0xBEBA_FECA => return Ok(Mimariya::hali()),
            _ => {}
        }
    }

    // PE: `MZ`, the PE header offset at 0x3C, then the COFF machine field.
    if bayt.get(..2) == Some(b"MZ") {
        let mawqi = u32le(&bayt, 0x3C).unwrap_or(0) as usize;
        if bayt.get(mawqi..mawqi + 4) == Some(b"PE\0\0") {
            let jihaz = u16le(&bayt, mawqi + 4).unwrap_or(0);
            return match jihaz {
                0x014C => Ok(Mimariya::X86),
                0x8664 => Ok(Mimariya::X8664),
                0xAA64 => Ok(Mimariya::Aarch64),
                _ => Err(Khata::min_tafsir(&KhataManassa::MimariyaMajhula {
                    masar: masar.to_path_buf(),
                    jihaz: u32::from(jihaz),
                })),
            };
        }
    }

    Err(Khata::min_tafsir(&KhataManassa::LaysaTanfidhiyan { masar: masar.to_path_buf() }))
}

fn u16le(bayt: &[u8], mawqi: usize) -> Option<u16> {
    bayt.get(mawqi..mawqi.checked_add(2)?)
        .and_then(|q| <[u8; 2]>::try_from(q).ok())
        .map(u16::from_le_bytes)
}

fn u32le(bayt: &[u8], mawqi: usize) -> Option<u32> {
    bayt.get(mawqi..mawqi.checked_add(4)?)
        .and_then(|q| <[u8; 4]>::try_from(q).ok())
        .map(u32::from_le_bytes)
}

/// Failures of the platform layer.
#[derive(Debug, thiserror::Error)]
pub enum KhataManassa {
    /// A volume's free space could not be queried.
    #[error("cannot measure free space on {masar}")]
    TaadhurQiyasMasaha {
        /// The path whose volume was queried.
        masar: PathBuf,
    },

    /// The file is not a PE, ELF or Mach-O executable at all.
    #[error("not an executable: {masar}")]
    LaysaTanfidhiyan {
        /// The file.
        masar: PathBuf,
    },

    /// The file is an executable for a machine Taarib cannot inject into.
    #[error("unsupported machine {jihaz:#06x} in {masar}")]
    MimariyaMajhula {
        /// The file.
        masar: PathBuf,
        /// The machine value read from its header.
        jihaz: u32,
    },

    /// A process could not be opened.
    #[error("cannot open process {raqm}")]
    TaadhurFathAmaliya {
        /// The process identifier.
        raqm: u32,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },
}

impl Tafsir for KhataManassa {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MANASSA
                + match self {
                    Self::TaadhurQiyasMasaha { .. } => 0,
                    Self::LaysaTanfidhiyan { .. } => 1,
                    Self::MimariyaMajhula { .. } => 2,
                    Self::TaadhurFathAmaliya { .. } => 3,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::TaadhurQiyasMasaha { masar } => format!(
                "تعذّر معرفة المساحة الفارغة على القرص الذي يحتوي {}.",
                masar.display()
            ),
            Self::LaysaTanfidhiyan { masar } => {
                format!("الملف {} ليس ملفًا تنفيذيًا، ولا يمكن فحصه كلعبة.", masar.display())
            }
            Self::MimariyaMajhula { masar, .. } => format!(
                "الملف {} مبني لمعمارية لا يدعمها تعريب، ولا يمكن الحقن فيه.",
                masar.display()
            ),
            Self::TaadhurFathAmaliya { raqm, .. } => {
                format!("تعذّر الوصول إلى العملية رقم {raqm}. قد تكون محمية أو أُغلقت.")
            }
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TaadhurQiyasMasaha { masar } => {
                format!("Cannot read free space on the volume holding {}.", masar.display())
            }
            Self::LaysaTanfidhiyan { masar } => {
                format!("{} is not an executable and cannot be probed as a game.", masar.display())
            }
            Self::MimariyaMajhula { masar, jihaz } => format!(
                "{} is built for machine type {jihaz:#06x}, which Taarib cannot inject into.",
                masar.display()
            ),
            Self::TaadhurFathAmaliya { raqm, .. } => {
                format!("Cannot access process {raqm}. It may be protected or already closed.")
            }
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::TaadhurQiyasMasaha { .. } => Khutwa::AadaMuhawala,
            Self::LaysaTanfidhiyan { .. } | Self::MimariyaMajhula { .. } => {
                Khutwa::IkhtiyarMasar { matlub: MasarMatlub::MalafTanfidhi }
            }
            Self::TaadhurFathAmaliya { sabab, .. } => {
                khutwa_io(sabab, MasarMatlub::MalafTanfidhi)
            }
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TaadhurQiyasMasaha { masar } | Self::LaysaTanfidhiyan { masar } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            }
            Self::MimariyaMajhula { masar, jihaz } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("jihaz".to_owned(), QeemaSiyaq::Raqm(i64::from(*jihaz)));
            }
            Self::TaadhurFathAmaliya { raqm, sabab } => {
                let _ = siyaq.insert("amaliya".to_owned(), QeemaSiyaq::Raqm(i64::from(*raqm)));
                siyaq.extend(siyaq_io(sabab));
            }
        }
        siyaq
    }
}

khata_min!(KhataManassa);

#[cfg(test)]
mod ikhtibarat {
    use super::{
        HalatTashghil, RuyatAmaliyat, Sunduq, fi_sunduq, halat_tashghil, ruyat_amaliyat,
        tashtaghil,
    };

    // A name no process can carry: `/` is the one byte a file name cannot hold.
    const ISM_MUSTAHIL: &str = "la/yumkin/an/yakun/hadha/tanfidhiyan";

    #[test]
    fn ruyat_amaliyat_tuwafiq_al_sunduq() {
        // The two answers are derived from the same evidence and must never
        // disagree: a named sandbox is, by definition, an isolated view.
        if let Some(sunduq) = fi_sunduq() {
            assert_eq!(ruyat_amaliyat(), RuyatAmaliyat::Maazula { sunduq });
        } else {
            assert_eq!(ruyat_amaliyat(), RuyatAmaliyat::Kamila);
        }
    }

    #[test]
    fn al_hala_al_thalitha_tamnaa() {
        assert!(HalatTashghil::Tashtaghil.yamnaa());
        assert!(!HalatTashghil::LaTashtaghil.yamnaa());
        assert!(HalatTashghil::GhayrMaaruf { sunduq: Sunduq::Flatpak }.yamnaa());
        assert!(HalatTashghil::GhayrMaaruf { sunduq: Sunduq::Hawiya }.yamnaa());
    }

    #[test]
    fn sunduq_yuraffiq_nafsahu_bil_hala() {
        let hala = HalatTashghil::GhayrMaaruf { sunduq: Sunduq::Flatpak };
        assert_eq!(hala.sunduq(), Some(Sunduq::Flatpak));
        assert_eq!(HalatTashghil::LaTashtaghil.sunduq(), None);
        assert_eq!(HalatTashghil::Tashtaghil.sunduq(), None);
    }

    #[test]
    fn ism_al_sunduq_ghayr_farigh() {
        for sunduq in [Sunduq::Flatpak, Sunduq::Snap, Sunduq::Hawiya] {
            assert!(!sunduq.ism().is_empty());
            assert!(!sunduq.ism_arabi().is_empty());
        }
    }

    #[test]
    fn ism_mustahil_la_yashtaghil_abadan() {
        // The one answer that is wrong under every environment is "running".
        assert_ne!(halat_tashghil(ISM_MUSTAHIL), HalatTashghil::Tashtaghil);
    }

    #[test]
    fn al_ghilaf_al_amin_yatbaa_al_hala() {
        // `tashtaghil` is the fail-safe fold of `halat_tashghil`, so the two
        // agree by construction — including inside a container, which is where
        // this test itself is likely to run.
        let hala = halat_tashghil(ISM_MUSTAHIL);
        assert_eq!(tashtaghil(ISM_MUSTAHIL), hala.yamnaa());
        match ruyat_amaliyat() {
            RuyatAmaliyat::Kamila => assert_eq!(hala, HalatTashghil::LaTashtaghil),
            RuyatAmaliyat::Maazula { sunduq } => {
                assert_eq!(hala, HalatTashghil::GhayrMaaruf { sunduq });
            }
        }
    }
}
