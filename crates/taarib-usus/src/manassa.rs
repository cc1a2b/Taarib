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
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        clippy::missing_const_for_fn,
        reason = "constant only on the platforms whose answer is a constant; the Linux body reads \
                  the sandbox markers, and one signature serves both"
    )
)]
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
///
/// **No caller in the product today, and kept anyway.** Both guards that ask the
/// question — `taarib_tathbeet`'s install-path check and its launch check — need
/// the third answer *by name*, because they tell the user the difference between
/// "close the game" and "Taarib cannot see whether the game is running". This is
/// what a guard that does not need that distinction should reach for, and it
/// exists so that the easiest correct thing to write is not
/// `halat_tashghil(ism) == HalatTashghil::Tashtaghil`, which is the same
/// question with the sandbox answered the unsafe way.
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
            //
            // `allow` rather than `expect`, because on the targets where the
            // conversion is not a no-op there is nothing to fulfil and an
            // `expect` would warn there instead.
            #[allow(
                clippy::useless_conversion,
                reason = "a no-op only on the targets where both fields are already 64-bit; \
                          see the comment above"
            )]
            Ok(u64::from(ihsaa.f_bavail).saturating_mul(u64::from(ihsaa.f_frsize)))
        } else {
            Err(Khata::min_tafsir(&KhataManassa::TaadhurQiyasMasaha {
                masar: masar.to_path_buf(),
            }))
        }
    }
}

/// Where a path lives, decided before the path itself is touched.
///
/// Four answers, and the fourth is the one an `Option<PathBuf>` folds into the
/// third: "no volume could be named for this path" and "this path is on the
/// system's own disk" are different facts, and only the second licenses the
/// caller to read a missing directory as a deleted one. A game on a second SSD
/// mounted at `/games` whose mount lost the race with the desktop session is
/// intact and unreachable, and telling its owner to reinstall it is the mistake
/// this type exists to make impossible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HalatHajm {
    /// A volume the platform names, whose root answered just now.
    Muttasil {
        /// The root, as the platform spells it: `E:\`, `\\server\share\`,
        /// `/media/games`, `/mnt/library`, `/Volumes/External`.
        jidhr: PathBuf,
        /// Whether it is a network location, for the wording and nothing else.
        shabaki: bool,
    },
    /// A volume the platform names, whose root did not answer — a drive letter
    /// with nothing behind it, a declared mount that is not mounted, a share
    /// that timed out.
    GhayrMuttasil {
        /// The root that did not answer.
        jidhr: PathBuf,
        /// Whether it is a network location.
        shabaki: bool,
    },
    /// The path lives on the filesystem this process runs from, which is
    /// reachable by construction. No volume question is owed, and an absent
    /// directory under it is genuinely absent.
    JidhrAlNizam,
    /// No volume could be named or probed for the path, and this is why.
    ///
    /// Not "fine". The caller has been told nothing about reachability, and a
    /// missing directory under such a path is not evidence of deletion.
    Majhul {
        /// What could not be read or decided.
        sabab: String,
    },
}

impl HalatHajm {
    /// Whether a missing directory under this path means the directory is
    /// gone.
    ///
    /// Only the two answers that *established* reachability clear it. An
    /// unreachable volume and an unanswered question both say "do not decide
    /// from absence", which is the same rule [`HalatTashghil::yamnaa`] applies
    /// to a process table that could not be read.
    #[must_use]
    pub const fn yasmah_bil_hukm(&self) -> bool {
        matches!(self, Self::Muttasil { .. } | Self::JidhrAlNizam)
    }

    /// The volume root, where one was named.
    #[must_use]
    pub fn jidhr(&self) -> Option<&Path> {
        match self {
            Self::Muttasil { jidhr, .. } | Self::GhayrMuttasil { jidhr, .. } => Some(jidhr),
            Self::JidhrAlNizam | Self::Majhul { .. } => None,
        }
    }
}

/// One mount, as the mount table or its declaration names it.
#[cfg(unix)]
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "constructed only from the Linux mount table; macOS reaches the same decision \
                  with an empty one, and the type is what keeps the two decisions one function"
    )
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tarkeeb {
    /// Where it is, or would be, mounted.
    masar: PathBuf,
    /// Whether its filesystem type is a network one.
    shabaki: bool,
}

/// Which volume holds a path, and whether that volume answers right now.
///
/// On Windows the answer is the drive or share prefix, probed once. On Linux the
/// kernel's own mount table decides: the deepest mount covering the path is the
/// volume, and a path the table places on the root filesystem is then checked
/// against `/etc/fstab` — a mount point declared there and not in the table is
/// a volume that is *not connected*, which is exactly what a LUKS volume not yet
/// unlocked or a mount unit that lost the race with the session looks like. The
/// conventional removable-media parents (`/media`, `/mnt`, `/run/media`,
/// `/Volumes`, `/net`) are honoured on every Unix, which is how macOS, with no
/// mount table this build reads, still names an external disk.
///
/// The network flag chooses between two wordings for one held-not-deleted
/// outcome, so a wrong guess there costs a less accurate sentence and nothing
/// else. Every other distinction here costs a user's patch state if it is
/// wrong, which is why the fourth answer exists rather than being folded away.
#[must_use]
pub fn halat_hajm(masar: &Path) -> HalatHajm {
    #[cfg(windows)]
    {
        halat_hajm_windows(masar)
    }
    #[cfg(unix)]
    {
        halat_hajm_unix(masar)
    }
}

/// The Windows half: the prefix is the volume.
#[cfg(windows)]
fn halat_hajm_windows(masar: &Path) -> HalatHajm {
    use std::path::{Component, Prefix};

    let Some(Component::Prefix(badia)) = masar.components().next() else {
        return HalatHajm::Majhul {
            sabab: format!(
                "{} carries no drive letter or share prefix, so no volume can be named for it",
                masar.display()
            ),
        };
    };
    let shabaki = match badia.kind() {
        Prefix::Disk(_) | Prefix::VerbatimDisk(_) => false,
        Prefix::UNC(..) | Prefix::VerbatimUNC(..) => true,
        Prefix::Verbatim(_) | Prefix::DeviceNS(_) => {
            return HalatHajm::Majhul {
                sabab: format!(
                    "{} names a device namespace rather than a volume, so its reachability \
                     is not a question this build can ask",
                    masar.display()
                ),
            };
        }
    };
    let mut jidhr = PathBuf::from(badia.as_os_str());
    jidhr.push(std::path::MAIN_SEPARATOR_STR);
    ijhas_jidhr(jidhr, shabaki, |jidhr| std::fs::metadata(jidhr).map(|_| ()))
}

/// The Unix half: the mount table where there is one, the conventions
/// everywhere.
#[cfg(unix)]
fn halat_hajm_unix(masar: &Path) -> HalatHajm {
    use std::path::Component;

    if !matches!(masar.components().next(), Some(Component::RootDir)) {
        return HalatHajm::Majhul {
            sabab: format!(
                "{} is not an absolute path, so no volume can be named for it",
                masar.display()
            ),
        };
    }

    #[cfg(target_os = "linux")]
    {
        let jadwal = match jadwal_al_tarkeeb() {
            Ok(jadwal) => jadwal,
            Err(sabab) => {
                return HalatHajm::Majhul {
                    sabab: format!("the mount table {MASAR_MOUNTINFO} could not be read: {sabab}"),
                };
            }
        };
        ihkum_hajm(masar, &jadwal, ilanat_fstab, |jidhr| std::fs::metadata(jidhr).map(|_| ()))
    }
    #[cfg(target_os = "macos")]
    {
        // Everything outside `/Volumes` is the boot volume or a firmlink onto
        // its data half, both of which are mounted for as long as the system
        // is up — so the conventions are the whole answer here.
        ihkum_hajm(masar, &[], || Ok(Vec::new()), |jidhr| std::fs::metadata(jidhr).map(|_| ()))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        HalatHajm::Majhul {
            sabab: format!(
                "this build reads no mount table on this platform, so the volume holding {} \
                 cannot be named",
                masar.display()
            ),
        }
    }
}

/// The decision, over an already-read mount table, a lazily-read declaration
/// list, and an injected probe — so every branch can be driven by a test
/// without a mount of its own.
///
/// The order is the design. The kernel's table is consulted first because it
/// is the truth about what *is* mounted; the conventions second, because a
/// removable disk mounted by `udisks` is in the table when present and its
/// directory is gone when not, so a conventional root that is neither mounted
/// nor present is absent; the declarations last, because they are the only
/// thing that turns "on the root filesystem as far as the kernel knows" into
/// "on a volume that is not here". A conventional root that is present but
/// neither mounted nor declared is an unmounted volume and a plain directory
/// wearing the same face, and the fourth answer is the only honest one there.
#[cfg(unix)]
fn ihkum_hajm(
    masar: &Path,
    jadwal: &[Tarkeeb],
    ilanat: impl FnOnce() -> std::io::Result<Vec<Tarkeeb>>,
    ijhas: impl Fn(&Path) -> std::io::Result<()>,
) -> HalatHajm {
    if let Some(tarkeeb) = atwal_tarkeeb(jadwal, masar)
        && tarkeeb.masar != Path::new("/")
    {
        return ijhas_jidhr(tarkeeb.masar.clone(), tarkeeb.shabaki, ijhas);
    }

    if let Some(jidhr) = jidhr_taqlidi(masar) {
        let shabaki = shabaki_taqlidi(masar);
        return match ijhas(&jidhr) {
            Err(sabab) if ghaib(&sabab) => HalatHajm::GhayrMuttasil { jidhr, shabaki },
            Err(sabab) => HalatHajm::Majhul {
                sabab: format!(
                    "the volume root {} answered neither present nor absent: {sabab}",
                    jidhr.display()
                ),
            },
            // The directory is there and nothing is mounted on it. A declaration
            // covering the path — at this root or deeper — settles it as a
            // volume that is not here; without one, nothing can.
            Ok(()) => match ilanat() {
                Ok(ilanat) => match atwal_tarkeeb(&ilanat, masar) {
                    Some(ilan) if ilan.masar != Path::new("/") => HalatHajm::GhayrMuttasil {
                        jidhr: ilan.masar.clone(),
                        shabaki: ilan.shabaki || shabaki,
                    },
                    _ => HalatHajm::Majhul {
                        sabab: format!(
                            "{} exists, nothing is mounted on it and nothing declares it, so an \
                             unmounted volume and a plain directory look the same from here",
                            jidhr.display()
                        ),
                    },
                },
                Err(sabab) => HalatHajm::Majhul {
                    sabab: format!(
                        "{} exists and nothing is mounted on it, and the declarations that \
                         would say whether it is a volume could not be read: {sabab}",
                        jidhr.display()
                    ),
                },
            },
        };
    }

    match ilanat() {
        Ok(ilanat) => match atwal_tarkeeb(&ilanat, masar) {
            Some(ilan) if ilan.masar != Path::new("/") => {
                HalatHajm::GhayrMuttasil { jidhr: ilan.masar.clone(), shabaki: ilan.shabaki }
            }
            _ => HalatHajm::JidhrAlNizam,
        },
        Err(sabab) => HalatHajm::Majhul {
            sabab: format!(
                "the declarations that would say whether {} is on a volume of its own could \
                 not be read: {sabab}",
                masar.display()
            ),
        },
    }
}

/// Probes a named root once and reads the three answers off the result.
fn ijhas_jidhr(
    jidhr: PathBuf,
    shabaki: bool,
    ijhas: impl Fn(&Path) -> std::io::Result<()>,
) -> HalatHajm {
    match ijhas(&jidhr) {
        Ok(()) => HalatHajm::Muttasil { jidhr, shabaki },
        Err(sabab) if ghaib(&sabab) => HalatHajm::GhayrMuttasil { jidhr, shabaki },
        Err(sabab) => HalatHajm::Majhul {
            sabab: format!(
                "the volume root {} answered neither present nor absent: {sabab}",
                jidhr.display()
            ),
        },
    }
}

/// Whether a probe failure means "not there" rather than "could not ask".
///
/// A drive letter with nothing behind it and an unmounted directory are
/// [`std::io::ErrorKind::NotFound`]; a share that is down answers with one of
/// the network kinds or a timeout. A permission refusal is neither — something
/// answered — and stays a gap.
fn ghaib(sabab: &std::io::Error) -> bool {
    matches!(
        sabab.kind(),
        std::io::ErrorKind::NotFound
            | std::io::ErrorKind::HostUnreachable
            | std::io::ErrorKind::NetworkUnreachable
            | std::io::ErrorKind::NetworkDown
            | std::io::ErrorKind::StaleNetworkFileHandle
            | std::io::ErrorKind::TimedOut
    )
}

/// The deepest mount covering a path, or [`None`] when no entry covers it.
///
/// Component-wise, so `/mnt/wslg` does not cover `/mnt/wslgames`. Equal depths
/// resolve to the later entry, which is the one the kernel lists as shadowing
/// the earlier.
#[cfg(unix)]
fn atwal_tarkeeb<'a>(jadwal: &'a [Tarkeeb], masar: &Path) -> Option<&'a Tarkeeb> {
    jadwal
        .iter()
        .filter(|tarkeeb| masar.starts_with(&tarkeeb.masar))
        .max_by_key(|tarkeeb| tarkeeb.masar.components().count())
}

/// The conventional volume root under a removable-media parent, if the path is
/// under one.
///
/// `/media/<label>`, `/mnt/<name>`, `/Volumes/<name>`, `/net/<host>`, and the
/// `udisks` shape `/run/media/<user>/<label>`. `/home` and `/usr` are always
/// present and treating them as volumes would make every ordinary install look
/// like it lived on removable media.
#[cfg(unix)]
fn jidhr_taqlidi(masar: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut ajza = masar.components();
    if !matches!(ajza.next(), Some(Component::RootDir)) {
        return None;
    }
    let Some(Component::Normal(walid)) = ajza.next() else {
        return None;
    };
    let walid_nass = walid.to_str()?;
    if !["media", "mnt", "run", "Volumes", "net"].contains(&walid_nass) {
        return None;
    }
    let mut jidhr = PathBuf::from("/");
    jidhr.push(walid);
    if walid_nass == "run" {
        let Some(Component::Normal(thani)) = ajza.next() else { return None };
        if thani != "media" {
            return None;
        }
        jidhr.push(thani);
        let Some(Component::Normal(mustakhdim)) = ajza.next() else { return None };
        jidhr.push(mustakhdim);
    }
    let Some(Component::Normal(ism)) = ajza.next() else { return None };
    jidhr.push(ism);
    Some(jidhr)
}

/// Whether a conventional root looks like a network location.
#[cfg(unix)]
fn shabaki_taqlidi(masar: &Path) -> bool {
    masar.starts_with("/net")
}

/// Filesystem types that put a path on the far side of a network.
#[cfg(target_os = "linux")]
const ANWA_SHABAKIYA: &[&str] = &[
    "nfs",
    "nfs4",
    "cifs",
    "smb3",
    "smbfs",
    "afs",
    "ceph",
    "glusterfs",
    "sshfs",
    "fuse.sshfs",
    "davfs",
    "fuse.davfs2",
    "fuse.rclone",
    "ncpfs",
];

/// The kernel's mount table.
#[cfg(target_os = "linux")]
const MASAR_MOUNTINFO: &str = "/proc/self/mountinfo";

/// The declared mounts.
#[cfg(target_os = "linux")]
const MASAR_FSTAB: &str = "/etc/fstab";

/// The kernel's mount table, read from `/proc`.
///
/// # Errors
///
/// Whatever the read refuses. Inside a sandbox that hides `/proc` this is the
/// gap that makes every answer [`HalatHajm::Majhul`], which is right: a build
/// that cannot see the mount table cannot say what is mounted.
#[cfg(target_os = "linux")]
fn jadwal_al_tarkeeb() -> std::io::Result<Vec<Tarkeeb>> {
    // Lossy on purpose: one mount point that is not UTF-8 must not turn every
    // path on the machine into a gap, and the four bytes the kernel escapes
    // are the only ones a lookup here depends on.
    let bayt = std::fs::read(MASAR_MOUNTINFO)?;
    Ok(ifham_mountinfo(&String::from_utf8_lossy(&bayt)))
}

/// The declared mounts, read from `/etc/fstab`.
///
/// An absent file is an empty declaration list, not a gap: a system with no
/// `fstab` has declared nothing, and that was read successfully.
///
/// # Errors
///
/// Any refusal other than absence.
#[cfg(target_os = "linux")]
fn ilanat_fstab() -> std::io::Result<Vec<Tarkeeb>> {
    match std::fs::read(MASAR_FSTAB) {
        Ok(bayt) => Ok(ifham_fstab(&String::from_utf8_lossy(&bayt))),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(sabab) => Err(sabab),
    }
}

/// `mountinfo`, one mount per line.
///
/// Field five is the mount point and the fields after the `-` separator begin
/// with the filesystem type; both are what this needs and nothing else is read.
/// A line that does not have that shape is skipped rather than failing the
/// table — the kernel does not write malformed lines, and a reader that
/// refused the whole table over one it did not understand would turn every
/// path on the machine into a gap.
#[cfg(target_os = "linux")]
fn ifham_mountinfo(nass: &str) -> Vec<Tarkeeb> {
    nass.lines()
        .filter_map(|satr| {
            let mut huqul = satr.split(' ');
            let masar = huqul.nth(4)?;
            let naw = huqul.skip_while(|haql| *haql != "-").nth(1).unwrap_or("");
            Some(Tarkeeb {
                masar: PathBuf::from(fukk_tahreeb(masar)),
                shabaki: ANWA_SHABAKIYA.contains(&naw),
            })
        })
        .collect()
}

/// `fstab`, one declaration per line.
///
/// Comments and blank lines are skipped, and so is anything whose mount point
/// is not an absolute path — swap, and the `none` some tools write.
#[cfg(target_os = "linux")]
fn ifham_fstab(nass: &str) -> Vec<Tarkeeb> {
    nass.lines()
        .filter_map(|satr| {
            let satr = satr.split('#').next().unwrap_or("").trim();
            if satr.is_empty() {
                return None;
            }
            let mut huqul = satr.split_whitespace();
            let _ = huqul.next()?;
            let masar = fukk_tahreeb(huqul.next()?);
            if !masar.starts_with('/') {
                return None;
            }
            let naw = huqul.next().unwrap_or("");
            Some(Tarkeeb { masar: PathBuf::from(masar), shabaki: ANWA_SHABAKIYA.contains(&naw) })
        })
        .collect()
}

/// Undoes the octal escapes both tables use for the four characters a mount
/// point cannot carry literally: `\040` space, `\011` tab, `\012` newline,
/// `\134` backslash. A backslash not followed by three octal digits is kept as
/// it was, digits included.
#[cfg(target_os = "linux")]
fn fukk_tahreeb(nass: &str) -> String {
    let mut natija = String::with_capacity(nass.len());
    let mut ahruf = nass.chars().peekable();
    while let Some(harf) = ahruf.next() {
        if harf != '\\' {
            natija.push(harf);
            continue;
        }
        let mut arqam = String::new();
        while arqam.len() < 3 {
            match ahruf.peek() {
                Some(raqm) if raqm.is_digit(8) => {
                    arqam.push(*raqm);
                    let _ = ahruf.next();
                }
                _ => break,
            }
        }
        let mafkuk = if arqam.len() == 3 {
            u32::from_str_radix(&arqam, 8).ok().and_then(char::from_u32)
        } else {
            None
        };
        if let Some(mafkuk) = mafkuk {
            natija.push(mafkuk);
        } else {
            natija.push('\\');
            natija.push_str(&arqam);
        }
    }
    natija
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

    // -----------------------------------------------------------------------
    // Which volume holds a path
    // -----------------------------------------------------------------------

    use std::path::{Path, PathBuf};

    use super::{HalatHajm, halat_hajm};

    /// Only the two answers that established reachability let a missing
    /// directory be read as a deleted one. Re-merging the fourth answer into
    /// the third is the defect this type was split to remove, and it fails here.
    #[test]
    fn al_hukm_min_al_ghiyab_yahtaj_ittisalan_muthbatan() {
        let jidhr = PathBuf::from("/games");
        assert!(HalatHajm::Muttasil { jidhr: jidhr.clone(), shabaki: false }.yasmah_bil_hukm());
        assert!(HalatHajm::JidhrAlNizam.yasmah_bil_hukm());
        assert!(!HalatHajm::GhayrMuttasil { jidhr, shabaki: false }.yasmah_bil_hukm());
        assert!(!HalatHajm::Majhul { sabab: "no table".to_owned() }.yasmah_bil_hukm());
    }

    /// A path that is reachable by construction is never called unreachable
    /// and never called unknown.
    #[test]
    fn masar_muttasil_bil_bina_la_yuqal_anhu_ghayr_dhalik() {
        let hala = halat_hajm(&std::env::temp_dir());
        assert!(
            hala.yasmah_bil_hukm(),
            "the temporary directory is on a mounted volume and the answer was {hala:?}"
        );
    }

    /// A relative path names no volume, and the answer says so rather than
    /// pretending the system disk was meant.
    #[test]
    fn masar_nisbi_majhul() {
        assert!(matches!(
            halat_hajm(Path::new("alaab/luba")),
            HalatHajm::Majhul { .. }
        ));
    }

    #[cfg(unix)]
    mod hajm_unix {
        use std::cell::Cell;
        use std::io::{Error, ErrorKind};
        use std::path::{Path, PathBuf};

        use super::super::{HalatHajm, Tarkeeb, atwal_tarkeeb, ihkum_hajm, jidhr_taqlidi};

        fn tarkeeb(masar: &str) -> Tarkeeb {
            Tarkeeb { masar: PathBuf::from(masar), shabaki: false }
        }

        fn jidhr_faqat() -> Vec<Tarkeeb> {
            vec![tarkeeb("/")]
        }

        // The probes and the declaration lists a test injects, in the
        // signatures the decision takes. The two that always succeed are still
        // fallible in type, because that is the type the decision is written
        // against.
        #[expect(
            clippy::unnecessary_wraps,
            reason = "a probe that always answers 'present', in the fallible signature the \
                      decision takes; narrowing it would not fit the parameter"
        )]
        fn mawjud(_: &Path) -> std::io::Result<()> {
            Ok(())
        }

        fn ghaib(_: &Path) -> std::io::Result<()> {
            Err(Error::from(ErrorKind::NotFound))
        }

        fn mamnu(_: &Path) -> std::io::Result<()> {
            Err(Error::from(ErrorKind::PermissionDenied))
        }

        #[expect(
            clippy::unnecessary_wraps,
            reason = "a declaration list that always reads, in the fallible signature the \
                      decision takes; see `mawjud`"
        )]
        fn bila_ilanat() -> std::io::Result<Vec<Tarkeeb>> {
            Ok(Vec::new())
        }

        #[test]
        fn atwal_tarkeeb_yuqaddim_al_aamaq_wa_yuqarin_bil_ajza() {
            let jadwal = vec![
                tarkeeb("/"),
                tarkeeb("/mnt/wslg"),
                tarkeeb("/mnt/wslg/distro"),
                tarkeeb("/tmp/.X11-unix"),
            ];
            let aamaq = atwal_tarkeeb(&jadwal, Path::new("/mnt/wslg/distro/etc/fstab"));
            assert_eq!(aamaq.map(|t| t.masar.as_path()), Some(Path::new("/mnt/wslg/distro")));
            // A string prefix is not a path prefix: `/mnt/wslg` must not cover
            // `/mnt/wslgames`, and `/tmp/.X11-unix` must not cover `/tmp/x`.
            let jidhr = atwal_tarkeeb(&jadwal, Path::new("/mnt/wslgames/x"));
            assert_eq!(jidhr.map(|t| t.masar.as_path()), Some(Path::new("/")));
            let jidhr = atwal_tarkeeb(&jadwal, Path::new("/tmp/x"));
            assert_eq!(jidhr.map(|t| t.masar.as_path()), Some(Path::new("/")));
            assert!(atwal_tarkeeb(&[], Path::new("/anything")).is_none());
        }

        #[test]
        fn al_judhur_al_taqlidiya() {
            let hal = |masar: &str| jidhr_taqlidi(Path::new(masar));
            assert_eq!(hal("/media/games/Steam/x"), Some(PathBuf::from("/media/games")));
            assert_eq!(hal("/mnt/library/x"), Some(PathBuf::from("/mnt/library")));
            assert_eq!(hal("/Volumes/External/x"), Some(PathBuf::from("/Volumes/External")));
            assert_eq!(
                hal("/run/media/hassan/Games/x"),
                Some(PathBuf::from("/run/media/hassan/Games"))
            );
            // `/run/user/...` is not media, and a bare parent names no volume.
            assert_eq!(hal("/run/user/1000/x"), None);
            assert_eq!(hal("/mnt"), None);
            assert_eq!(hal("/home/hassan/Games/x"), None);
            assert_eq!(hal("/usr/share/x"), None);
        }

        #[test]
        fn mujallad_murakkab_yuhkam_min_jadwal_al_nawa() {
            let jadwal = vec![tarkeeb("/"), tarkeeb("/games")];
            let masar = Path::new("/games/Steam/steamapps/common/ELDEN RING");
            assert_eq!(
                ihkum_hajm(masar, &jadwal, bila_ilanat, mawjud),
                HalatHajm::Muttasil { jidhr: PathBuf::from("/games"), shabaki: false }
            );
            // Mounted a moment ago and gone now: the table is stale, the probe
            // is not.
            assert_eq!(
                ihkum_hajm(masar, &jadwal, bila_ilanat, ghaib),
                HalatHajm::GhayrMuttasil { jidhr: PathBuf::from("/games"), shabaki: false }
            );
            // Something answered, and it was not "absent".
            assert!(matches!(
                ihkum_hajm(masar, &jadwal, bila_ilanat, mamnu),
                HalatHajm::Majhul { .. }
            ));
        }

        #[test]
        fn mujallad_muallan_ghayr_murakkab_huwa_hajm_ghayr_muttasil() {
            // The F20 shape exactly: `/games` in fstab, not in the mount table,
            // and the game directory under it absent. The old answer was "the
            // game was deleted"; the honest one is "the volume is not here".
            let ilanat = || Ok(vec![tarkeeb("/games"), tarkeeb("/")]);
            let masar = Path::new("/games/Steam/steamapps/common/ELDEN RING");
            let musta = Cell::new(0_u32);
            let ijhas = |_: &Path| {
                musta.set(musta.get() + 1);
                Err(Error::from(ErrorKind::NotFound))
            };
            assert_eq!(
                ihkum_hajm(masar, &jidhr_faqat(), ilanat, ijhas),
                HalatHajm::GhayrMuttasil { jidhr: PathBuf::from("/games"), shabaki: false }
            );
            assert_eq!(musta.get(), 0, "a declared, unmounted volume is not probed");
        }

        #[test]
        fn masar_ala_jidhr_al_nizam_bila_ilan() {
            let masar = Path::new("/home/hassan/Games/x");
            assert_eq!(
                ihkum_hajm(masar, &jidhr_faqat(), bila_ilanat, mamnu),
                HalatHajm::JidhrAlNizam
            );
        }

        #[test]
        fn al_ilanat_ghayr_al_maqrua_tamnaa_al_hukm() {
            // fstab unreadable and the path on the root filesystem: a declared
            // volume cannot be ruled out, so the answer is not "system disk".
            let ilanat = || Err(Error::from(ErrorKind::PermissionDenied));
            let masar = Path::new("/home/hassan/Games/x");
            assert!(matches!(
                ihkum_hajm(masar, &jidhr_faqat(), ilanat, mawjud),
                HalatHajm::Majhul { .. }
            ));
        }

        #[test]
        fn jidhr_taqlidi_ghaib_huwa_hajm_ghayr_muttasil() {
            // `udisks` removes the mount-point directory on unmount, so an
            // absent conventional root is an absent volume.
            let masar = Path::new("/run/media/hassan/Games/Steam/x");
            assert_eq!(
                ihkum_hajm(masar, &jidhr_faqat(), bila_ilanat, ghaib),
                HalatHajm::GhayrMuttasil {
                    jidhr: PathBuf::from("/run/media/hassan/Games"),
                    shabaki: false
                }
            );
        }

        #[test]
        fn jidhr_taqlidi_mawjud_ghayr_murakkab() {
            let masar = Path::new("/mnt/library/Steam/x");
            // Declared: an unmounted volume.
            let muallan = || Ok(vec![tarkeeb("/mnt/library")]);
            assert_eq!(
                ihkum_hajm(masar, &jidhr_faqat(), muallan, mawjud),
                HalatHajm::GhayrMuttasil { jidhr: PathBuf::from("/mnt/library"), shabaki: false }
            );
            // Declared deeper than the conventional root: still the declared one.
            let aamaq = || Ok(vec![tarkeeb("/mnt/library/Steam")]);
            assert_eq!(
                ihkum_hajm(masar, &jidhr_faqat(), aamaq, mawjud),
                HalatHajm::GhayrMuttasil {
                    jidhr: PathBuf::from("/mnt/library/Steam"),
                    shabaki: false
                }
            );
            // Not declared: a plain directory and an unmounted volume look the
            // same, and neither "deleted" nor "not connected" may be asserted.
            assert!(matches!(
                ihkum_hajm(masar, &jidhr_faqat(), bila_ilanat, mawjud),
                HalatHajm::Majhul { .. }
            ));
        }

        #[test]
        fn al_ilan_al_shabaki_yahmil_wasfahu() {
            let ilanat = || Ok(vec![Tarkeeb { masar: PathBuf::from("/games"), shabaki: true }]);
            assert_eq!(
                ihkum_hajm(Path::new("/games/x"), &jidhr_faqat(), ilanat, ghaib),
                HalatHajm::GhayrMuttasil { jidhr: PathBuf::from("/games"), shabaki: true }
            );
        }
    }

    #[cfg(target_os = "linux")]
    mod hajm_linux {
        use std::path::Path;

        use super::super::{fukk_tahreeb, ifham_fstab, ifham_mountinfo};

        #[test]
        fn fakk_al_tahreeb() {
            assert_eq!(fukk_tahreeb(r"/media/user/My\040Games"), "/media/user/My Games");
            assert_eq!(fukk_tahreeb(r"C:\134"), r"C:\");
            assert_eq!(fukk_tahreeb(r"a\011b\012c"), "a\tb\nc");
            // Not an escape: kept as written, digits and all.
            assert_eq!(fukk_tahreeb(r"\04x"), r"\04x");
            assert_eq!(fukk_tahreeb(r"end\"), r"end\");
            assert_eq!(fukk_tahreeb("/plain/path"), "/plain/path");
        }

        #[test]
        fn qiraat_mountinfo() {
            // Real lines from a WSL2 machine, plus one NFS mount.
            let nass = "82 67 8:48 / / rw,relatime - ext4 /dev/sdd rw,discard\n\
                        78 82 0:34 / /mnt/wsl rw,relatime shared:1 - tmpfs none rw\n\
                        136 82 0:74 / /mnt/e rw,noatime - 9p E:\\134 rw,aname=drvfs\n\
                        140 82 0:80 / /media/hassan/My\\040Games rw - ext4 /dev/sdb1 rw\n\
                        141 82 0:81 / /net/nas rw,relatime - nfs4 nas:/games rw\n\
                        malformed line\n";
            let jadwal = ifham_mountinfo(nass);
            let masarat: Vec<&Path> = jadwal.iter().map(|t| t.masar.as_path()).collect();
            assert_eq!(
                masarat,
                vec![
                    Path::new("/"),
                    Path::new("/mnt/wsl"),
                    Path::new("/mnt/e"),
                    Path::new("/media/hassan/My Games"),
                    Path::new("/net/nas"),
                ]
            );
            let shabaki: Vec<bool> = jadwal.iter().map(|t| t.shabaki).collect();
            assert_eq!(shabaki, vec![false, false, false, false, true]);
        }

        #[test]
        fn qiraat_fstab() {
            let nass = "# /etc/fstab\n\
                        \n\
                        UUID=1234 /  ext4 errors=remount-ro 0 1\n\
                        UUID=5678 /games ext4 defaults,noauto 0 2  # second SSD\n\
                        /dev/mapper/vault /home/hassan/Games ext4 noauto 0 0\n\
                        nas:/export /net/nas nfs4 defaults 0 0\n\
                        /swapfile none swap sw 0 0\n\
                        tmpfs /tmp tmpfs defaults 0 0\n";
            let ilanat = ifham_fstab(nass);
            let masarat: Vec<&Path> = ilanat.iter().map(|t| t.masar.as_path()).collect();
            assert_eq!(
                masarat,
                vec![
                    Path::new("/"),
                    Path::new("/games"),
                    Path::new("/home/hassan/Games"),
                    Path::new("/net/nas"),
                    Path::new("/tmp"),
                ]
            );
            let shabaki: Vec<bool> = ilanat.iter().map(|t| t.shabaki).collect();
            assert_eq!(shabaki, vec![false, false, false, true, false]);
        }

        #[test]
        fn al_jihaz_al_hali_yaqra_jadwalahu() {
            // `/proc/self/mountinfo` is readable here, and the one entry every
            // machine has is the root filesystem.
            let jadwal = super::super::jadwal_al_tarkeeb();
            assert!(
                jadwal
                    .as_ref()
                    .is_ok_and(|jadwal| jadwal.iter().any(|t| t.masar == Path::new("/"))),
                "the mount table must be readable and name the root: {jadwal:?}"
            );
        }
    }
}
