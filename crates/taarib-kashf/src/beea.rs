//! البيئة — Proton and Wine prefixes: the drive map, the registry, and the
//! translation between the filesystem the game believes in and the one that
//! exists.
//!
//! ## The thing that makes this hard
//!
//! **The game sees a filesystem that does not exist.** A Windows game running
//! under Proton is told it lives on `C:\`, that its user profile is
//! `C:\users\steamuser`, and that the whole host is reachable through `Z:\`.
//! None of those are real. They are a fiction the prefix maintains: `C:\` is a
//! directory called `drive_c` somewhere under `compatdata`, `Z:\` is whatever
//! the `dosdevices/z:` symlink happens to point at, and a drive letter the user
//! added by hand points wherever they aimed it.
//!
//! Every path in the game's own configuration, in its manifests, in its
//! registry entries, and in the arguments it is launched with is a path in that
//! fiction. Every path Taarib writes to is a real one. This module is the only
//! place the two meet.
//!
//! ## Why getting it wrong is worse than failing
//!
//! A wrong mapping does not fail loudly. `C:\Program Files\Game\BepInEx` maps to
//! *some* directory; if that directory is the wrong one, the write succeeds, the
//! files land, the manifest records them, and the installer reports success. The
//! game then launches, looks in the directory it actually uses, finds nothing,
//! and runs in English. The user reports that Taarib "did nothing", and there is
//! no error anywhere to explain it.
//!
//! That failure mode is why this module refuses to assume:
//!
//! - The drive map is **read from `dosdevices`**, never assumed. `Z:` is `/` by
//!   convention, not by guarantee, and a prefix where it differs is a prefix
//!   where every translated path is silently wrong.
//! - Path translation resolves **each component case-insensitively against what
//!   is actually on disk**, because Windows paths are case-insensitive and the
//!   host filesystem is not. See [`MaalumatBeea::ila_mudif`] for why the obvious
//!   shortcuts fail.
//! - A prefix is only a prefix when it has both a `drive_c` directory and a
//!   `user.reg`. A `compatdata/<appid>` directory is created before the prefix
//!   is populated, and treating an empty one as usable installs a patch into
//!   nothing.
//! - The Wine build is reported as `None` when it cannot be determined, never
//!   guessed.
//!
//! ## What lives here
//!
//! | item | what it answers |
//! | --- | --- |
//! | [`hal_beea`] | open a prefix: drive map, Wine build |
//! | [`MaalumatBeea::ila_mudif`] / [`MaalumatBeea::min_mudif`] | `C:\x` ⇄ `/real/x` |
//! | [`MaalumatBeea::tajawuzat_dll`] | `[Software\Wine\DllOverrides]`, which is where Phase 15 makes BepInEx's proxy DLL load |
//! | [`iqra_sijill`] / [`SijillBeea`] | Wine's `.reg` text format, parsed properly |
//! | [`beea_steam`] | `steamapps/compatdata/<appid>/pfx`, verified rather than assumed |
//! | [`iktashif_beeat`] | prefixes outside Steam: bare Wine, Lutris, Bottles, Heroic, and the Flatpak layout of each |
//! | [`naw_beea`] | Proton or plain Wine, by Proton's own markers |
//!
//! Nothing here writes. Discovery is read-only; Phase 15 owns every write, and
//! it writes through the translations this module produces.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use taarib_usus::khata::{Khata, Natija};
use taarib_usus::manassa::BeeatTawafuq;

use crate::khata::KhataKashf;

/// The directory inside a prefix that `C:\` conventionally resolves to.
///
/// Conventionally, not necessarily: the drive map is still read from
/// `dosdevices`. This name is used to *recognise* a prefix and as the last
/// resort when `dosdevices` is unreadable.
const ISM_QURS_C: &str = "drive_c";

/// The directory holding the drive-letter symlinks.
const ISM_AJHIZA: &str = "dosdevices";

/// `HKEY_CURRENT_USER` on disk.
const ISM_SIJILL_MUSTAKHDIM: &str = "user.reg";

/// `HKEY_LOCAL_MACHINE` on disk.
const ISM_SIJILL_NIZAM: &str = "system.reg";

/// The key Wine reads DLL load order from, and the key Phase 15 writes into so
/// that BepInEx's `winhttp.dll` proxy is loaded from the game directory instead
/// of from Wine's own builtin.
const MIFTAH_TAJAWUZAT: &str = r"Software\Wine\DllOverrides";

/// Where a prefix manager may have recorded which Wine build built the prefix.
const MIFTAH_WINE: &str = r"Software\Wine";

/// How many path components a translated path may carry.
///
/// A real Windows path never comes close. A path that does is either corrupt or
/// hostile, and walking it would mean an unbounded number of directory reads.
const HADD_AJZA_MASAR: usize = 128;

/// How many entries a single directory listing will consider.
///
/// Bounds the cost of case-insensitive resolution and of prefix discovery
/// against a directory somebody filled with a million files.
const HADD_MUDKHALAT: usize = 4096;

/// How many bytes are read from a small marker file — `version`, `config_info`,
/// `bottle.yml`. These are a few lines each; anything larger is not one of them.
const HADD_ALAMA: u64 = 64 * 1024;

/// Values of `HKCU\Software\Wine\Version` that are the **Windows** version
/// override winecfg writes, not a Wine build.
///
/// This trap is worth naming: the value under `[Software\Wine]` called
/// `Version` is what Wine reports to the application as the Windows version, so
/// reading it as "the Wine version" produces `win10` as a Wine build. Anything
/// in this list is discarded rather than reported.
const ISDARAT_WINDOWS: [&str; 20] = [
    "win20", "win30", "win31", "win95", "win98", "winme", "nt351", "nt40", "win2000", "win2k",
    "winxp", "win2k3", "vista", "win2k8", "win7", "win2k8r2", "win8", "win81", "win10", "win11",
];

// ---------------------------------------------------------------------------
// The prefix
// ---------------------------------------------------------------------------

/// An opened compatibility prefix.
///
/// Cheap to build and cheap to keep: it holds the drive map and the Wine build,
/// and reads the registry on demand rather than at open time, because most
/// callers want a path translated and never look at a registry key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaalumatBeea {
    /// The prefix root — the directory holding `drive_c`, `dosdevices`,
    /// `system.reg` and `user.reg`.
    pub jidhr: PathBuf,
    /// The drive map as `dosdevices` declares it, lowercase letter to real
    /// host path, sorted by letter so the order never depends on the order the
    /// filesystem happened to list the links in.
    pub aqrass: Vec<(char, PathBuf)>,
    /// The Wine or Proton build this prefix was made by, when something on disk
    /// names it. `None` means it could not be determined — never a guess.
    pub isdar_wine: Option<String>,
}

/// Opens a prefix.
///
/// Reads the drive map out of `dosdevices` and determines the Wine build. Does
/// not read the registry; [`MaalumatBeea::tajawuzat_dll`] and [`iqra_sijill`]
/// do that when something asks for it.
///
/// # Errors
///
/// Returns [`KhataKashf::BeeaMafquda`] when `jidhr` is not a populated prefix —
/// it does not exist, has no `drive_c` directory, or has no `user.reg`. The
/// last case is the one that matters in practice: Steam creates
/// `compatdata/<appid>` as soon as a game is configured for a compatibility
/// tool, and it stays empty until the game is first run. An empty prefix is not
/// a prefix, and reporting it as one would put a patch somewhere the game never
/// looks.
pub fn hal_beea(jidhr: &Path) -> Natija<MaalumatBeea> {
    if !hiya_beea(jidhr) {
        return Err(Khata::min_tafsir(&KhataKashf::BeeaMafquda { masar: jidhr.to_path_buf() }));
    }
    let jidhr = jidhr.to_path_buf();
    let aqrass = khareetat_aqrass(&jidhr);
    let isdar_wine = isdar_beea(&jidhr);
    Ok(MaalumatBeea { jidhr, aqrass, isdar_wine })
}

/// Whether a directory is a populated Wine prefix.
///
/// The test is `drive_c` as a directory **and** `user.reg` as a file. Both are
/// required, and for the reason spelled out on [`hal_beea`]: the directory
/// alone proves only that something intended to create a prefix there.
#[must_use]
pub fn hiya_beea(jidhr: &Path) -> bool {
    jidhr.join(ISM_QURS_C).is_dir() && jidhr.join(ISM_SIJILL_MUSTAKHDIM).is_file()
}

impl MaalumatBeea {
    /// The host path a drive letter resolves to, case-insensitively.
    #[must_use]
    pub fn qurs(&self, harf: char) -> Option<&Path> {
        let matlub = harf.to_ascii_lowercase();
        self.aqrass.iter().find(|(h, _)| *h == matlub).map(|(_, masar)| masar.as_path())
    }

    /// The real directory `C:\` resolves to.
    ///
    /// Falls back to `<prefix>/drive_c` when `dosdevices` declares no `c:` —
    /// which only happens on a prefix whose links have been removed, and where
    /// the conventional layout is the only evidence left.
    #[must_use]
    pub fn drive_c(&self) -> PathBuf {
        self.qurs('c').map_or_else(|| self.jidhr.join(ISM_QURS_C), Path::to_path_buf)
    }

    /// The prefix's `user.reg` — `HKEY_CURRENT_USER`.
    #[must_use]
    pub fn masar_sijill_mustakhdim(&self) -> PathBuf {
        self.jidhr.join(ISM_SIJILL_MUSTAKHDIM)
    }

    /// The prefix's `system.reg` — `HKEY_LOCAL_MACHINE`.
    #[must_use]
    pub fn masar_sijill_nizam(&self) -> PathBuf {
        self.jidhr.join(ISM_SIJILL_NIZAM)
    }

    /// What this prefix is, classified as [`naw_beea`] classifies it.
    #[must_use]
    pub fn naw(&self) -> BeeatTawafuq {
        naw_beea(&self.jidhr)
    }
}

// ---------------------------------------------------------------------------
// The registry, as a value
// ---------------------------------------------------------------------------

/// A parsed Wine registry file.
///
/// Wine keeps the registry as text: `user.reg` is `HKEY_CURRENT_USER`,
/// `system.reg` is `HKEY_LOCAL_MACHINE`, and `userdef.reg` is the default user
/// hive. All three share one format, documented on [`iqra_sijill`].
///
/// Keys are held in a normalized form — unescaped, lowercased, with leading and
/// trailing separators stripped — so a lookup written the way the file spells it
/// (`Software\\Wine\\DllOverrides`) and a lookup written the way a person spells
/// it (`Software\Wine\DllOverrides`) both find the same key, as they do in the
/// real registry, which is case-insensitive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SijillBeea {
    masar: PathBuf,
    mafatih: BTreeMap<String, BTreeMap<String, String>>,
    mimariya: Option<String>,
}

impl SijillBeea {
    /// The file this was read from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The architecture the file declares in its `#arch=` directive — `win32`
    /// or `win64`.
    ///
    /// This is the prefix's own word for whether it is a 32-bit or 64-bit
    /// prefix, and it decides which BepInEx build belongs inside it.
    #[must_use]
    pub fn mimariya(&self) -> Option<&str> {
        self.mimariya.as_deref()
    }

    /// One key's values, by its path relative to the file's own hive.
    ///
    /// The path is matched case-insensitively and accepts either escaping —
    /// `Software\Wine` and `Software\\Wine` are the same key.
    #[must_use]
    pub fn miftah(&self, masar: &str) -> Option<&BTreeMap<String, String>> {
        self.mafatih.get(&tabi_miftah(masar))
    }

    /// One value, by key path and value name.
    ///
    /// The default value of a key — written `@=` in the file — has the empty
    /// name.
    #[must_use]
    pub fn qeema(&self, masar: &str, ism: &str) -> Option<&str> {
        let miftah = self.miftah(masar)?;
        let matlub = ism.to_lowercase();
        miftah
            .iter()
            .find(|(mawjud, _)| mawjud.to_lowercase() == matlub)
            .map(|(_, qeema)| qeema.as_str())
    }

    /// Every key in the file, in normalized form.
    pub fn mafatih(&self) -> impl Iterator<Item = &str> {
        self.mafatih.keys().map(String::as_str)
    }

    /// How many keys the file carried.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mafatih.len()
    }
}

// ---------------------------------------------------------------------------
// The drive map, read rather than assumed
// ---------------------------------------------------------------------------

/// Builds the drive-letter map by resolving the symlinks in `dosdevices`.
///
/// A prefix's `dosdevices` directory is the drive table. Wine creates `c:`
/// pointing at `../drive_c` and `z:` pointing at `/`, and everything after that
/// is the user, the launcher, or `winecfg`: a second library drive, a mapped
/// network share, a `d:` aimed at a mounted disc image. Lutris and Bottles both
/// add letters of their own, and Steam adds nothing but does not stop the user.
///
/// So the table is read. Hardcoding `z:` to `/` would be correct on almost every
/// machine and silently wrong on the ones where it is not — and "silently wrong"
/// here means every host path derived from a `Z:\` path lands somewhere the game
/// cannot see.
///
/// Entries that are not drive letters are skipped. `dosdevices` also holds the
/// device links Wine uses for volume identity — `c::`, `d::` pointing at
/// `/dev/sr0` — and those are devices, not drives, so the name must be exactly a
/// letter and a colon.
fn khareetat_aqrass(jidhr: &Path) -> Vec<(char, PathBuf)> {
    let mut khareeta: BTreeMap<char, PathBuf> = BTreeMap::new();
    let mujallad = jidhr.join(ISM_AJHIZA);

    if let Ok(qaima) = fs::read_dir(&mujallad) {
        for madkhal in qaima.take(HADD_MUDKHALAT).flatten() {
            let ism = madkhal.file_name();
            let Some(harf) = ism.to_str().and_then(harf_qurs) else {
                continue;
            };
            if let Some(hadaf) = hall_rabt(&madkhal.path(), &mujallad) {
                let _ = khareeta.insert(harf, hadaf);
            }
        }
    }

    // A prefix whose links were lost — copied through a filesystem that does not
    // carry symlinks, or restored from an archive that dropped them — still has
    // the one drive its own layout proves. Recording that and nothing else is
    // honest: `C:\` translates, and any other letter reports as unmappable
    // rather than being guessed at.
    if let Entry::Vacant(makan) = khareeta.entry('c') {
        let qurs_c = jidhr.join(ISM_QURS_C);
        if qurs_c.is_dir() {
            let _ = makan.insert(fs::canonicalize(&qurs_c).unwrap_or(qurs_c));
        }
    }

    khareeta.into_iter().collect()
}

/// The drive letter an entry in `dosdevices` names, if it names one.
///
/// Exactly one ASCII letter followed by exactly one colon. `c::` is a device
/// link, `com1` is a serial port, and neither is a drive.
fn harf_qurs(ism: &str) -> Option<char> {
    let mut huruf = ism.chars();
    let awwal = huruf.next()?;
    if !awwal.is_ascii_alphabetic() || huruf.next() != Some(':') || huruf.next().is_some() {
        return None;
    }
    Some(awwal.to_ascii_lowercase())
}

/// Resolves one `dosdevices` entry to a real host path.
///
/// The link target is usually relative — `c:` is `../drive_c` — and relative
/// targets resolve against the `dosdevices` directory itself, not against the
/// process's working directory. Getting that wrong maps `C:\` to
/// `<cwd>/../drive_c`, which on a developer's machine frequently exists.
///
/// The result is canonicalized so that later prefix comparisons in
/// [`MaalumatBeea::min_mudif`] compare like with like; when canonicalization
/// fails — a link pointing at a drive that is not mounted right now — the
/// lexically normalized path is kept, because an unmounted drive is still a
/// declared drive.
fn hall_rabt(rabt: &Path, mujallad: &Path) -> Option<PathBuf> {
    match fs::read_link(rabt) {
        Ok(hadaf) => {
            let mutlaq = if hadaf.is_absolute() { hadaf } else { mujallad.join(hadaf) };
            Some(fs::canonicalize(&mutlaq).unwrap_or_else(|_| sawi(&mutlaq)))
        }
        // Not a link at all: a prefix restored onto a filesystem with no symlink
        // support keeps `c:` as a real directory, and it is still the drive.
        Err(_) if rabt.is_dir() => {
            Some(fs::canonicalize(rabt).unwrap_or_else(|_| sawi(rabt)))
        }
        Err(_) => None,
    }
}

/// Removes `.` and `..` from a path lexically, without touching the filesystem.
///
/// Used only where canonicalization is impossible — an unmounted target, a path
/// that does not exist yet. It is not a substitute for canonicalization and does
/// not resolve symlinks; it exists so that an unresolvable path is at least
/// comparable.
fn sawi(masar: &Path) -> PathBuf {
    let mut mabni = PathBuf::new();
    for juz in masar.components() {
        match juz {
            Component::ParentDir => {
                let _ = mabni.pop();
            }
            Component::CurDir => {}
            akhar => mabni.push(akhar.as_os_str()),
        }
    }
    mabni
}

// ---------------------------------------------------------------------------
// What kind of prefix this is
// ---------------------------------------------------------------------------

/// Classifies a prefix as Proton or as plain Wine.
///
/// Proton is not a flavour of Wine prefix — it has a layout of its own, and it
/// leaves its own marks. Steam gives each app a `compatdata/<appid>` directory
/// and Proton builds the prefix in a subdirectory called `pfx`, writing beside
/// it:
///
/// - `version` — the Proton build, as `<unix timestamp> <build name>`, copied
///   from the Proton distribution's own `version` file,
/// - `config_info` — the runtime configuration, whose first line is the path to
///   the Proton distribution in use,
/// - `tracked_files` — everything Proton installed into the prefix, which is how
///   it knows what to redo after an upgrade,
/// - `pfx.lock` — held while the prefix is being built or upgraded.
///
/// A prefix is Proton when it is named `pfx` **and** at least one of those
/// markers is beside it. Both halves matter: the name alone would misclassify
/// any prefix a user happened to call `pfx`, and the markers alone would
/// misclassify a plain Wine prefix that a script left a `version` file next to.
///
/// Everything else that is a prefix is plain Wine — Lutris, Bottles, Heroic, a
/// bare `WINEPREFIX`. A Heroic prefix running a Proton-GE build classifies as
/// Wine, because Heroic writes none of Proton's markers into it, and reporting
/// Proton on the strength of a directory name would be a guess.
///
/// A path that is not a prefix at all classifies as [`BeeatTawafuq::Asli`]:
/// nothing here runs behind a compatibility layer.
#[must_use]
pub fn naw_beea(jidhr: &Path) -> BeeatTawafuq {
    if !hiya_beea(jidhr) {
        return BeeatTawafuq::Asli;
    }
    isdar_proton(jidhr).map_or_else(
        || BeeatTawafuq::Wine { isdar: isdar_beea(jidhr), beea: jidhr.to_path_buf() },
        |isdar| BeeatTawafuq::Proton { isdar, beea: jidhr.to_path_buf() },
    )
}

/// The Proton build behind a prefix, or `None` when the prefix is not Proton's.
fn isdar_proton(jidhr: &Path) -> Option<String> {
    if jidhr.file_name() != Some(OsStr::new("pfx")) {
        return None;
    }
    let walid = jidhr.parent()?;
    let malaf_idad = walid.join("config_info");
    let alama = malaf_idad.is_file()
        || walid.join("tracked_files").is_file()
        || walid.join("version").is_file()
        || walid.join("pfx.lock").exists();
    if !alama {
        return None;
    }

    // `version` reads `1699999999 proton-9.0-4`; the build name is the last
    // field. Some builds write the name alone, which the same rule handles.
    if let Some(satr) = satr_awwal(&walid.join("version"))
        && let Some(ism) = satr.split_whitespace().last()
        && !ism.is_empty()
    {
        return Some(ism.to_owned());
    }

    // Failing that, `config_info`'s first line is the path to the distribution
    // in use, and Steam installs those as `steamapps/common/<build>`.
    if let Some(satr) = satr_awwal(&malaf_idad)
        && let Some(ism) = ism_tawzi_proton(&satr)
    {
        return Some(ism);
    }

    // The markers say Proton and nothing says which. The name is the honest
    // answer; the build is genuinely unknown.
    Some("Proton".to_owned())
}

/// Pulls a Proton build name out of a distribution path.
///
/// `/home/u/.steam/steam/steamapps/common/Proton 9.0 (Beta)/files/` yields
/// `Proton 9.0 (Beta)` — the component after `common`.
fn ism_tawzi_proton(masar: &str) -> Option<String> {
    let mut sabiq: Option<OsString> = None;
    let mut natija = None;
    for juz in Path::new(masar.trim()).components() {
        if let Component::Normal(ism) = juz {
            if sabiq.as_deref() == Some(OsStr::new("common")) {
                natija = ism.to_str().map(str::to_owned);
            }
            sabiq = Some(ism.to_os_string());
        }
    }
    natija
}

/// The Wine or Proton build that made this prefix, when something on disk names
/// it.
///
/// Four sources, in the order they are asked, which is the order of how
/// specifically each one answers the question:
///
/// 1. **Proton's `version` beside the prefix.** For a Proton prefix this is the
///    true and complete answer, and it is one small file.
/// 2. **`bottle.yml`, a `.update-timestamp` neighbour in the prefix root.**
///    Bottles keeps its bottle configuration inside the prefix and records the
///    runner there, so a Bottles prefix names its own Wine build exactly.
/// 3. **`.wine-version` or `version` in the prefix root**, written by some
///    managers and by hand.
/// 4. **`[Software\Wine]` in `system.reg`, then `user.reg`.** Last, and for two
///    reasons. It is the most expensive — a Proton `system.reg` runs to
///    megabytes and this runs once per discovered game — and it is the least
///    reliable: the value called `Version` under that key is normally the
///    **Windows** version winecfg writes, not a Wine build. Anything matching
///    [`ISDARAT_WINDOWS`] is discarded rather than reported, because answering
///    `win10` to "which Wine built this" would look like an answer and be
///    nonsense.
///
/// When none of them answer, the result is `None`. Wine itself does not record
/// its own version inside a prefix, so a bare `WINEPREFIX` built by the
/// distribution package genuinely cannot be identified from the prefix alone,
/// and saying so is better than reading the Windows version and calling it Wine.
fn isdar_beea(jidhr: &Path) -> Option<String> {
    if let Some(isdar) = isdar_proton(jidhr) {
        return Some(isdar);
    }

    if let Some(runner) = satr_bi_bidaya(&jidhr.join("bottle.yml"), "Runner:") {
        return Some(runner);
    }

    for ism_malaf in [".wine-version", "version"] {
        if let Some(satr) = satr_awwal(&jidhr.join(ism_malaf))
            && let Some(ism) = satr.split_whitespace().last()
            && !ism.is_empty()
        {
            return Some(ism.to_owned());
        }
    }

    for ism_malaf in [ISM_SIJILL_NIZAM, ISM_SIJILL_MUSTAKHDIM] {
        let masar = jidhr.join(ism_malaf);
        if !masar.is_file() {
            continue;
        }
        let Ok(sijill) = iqra_sijill(&masar) else {
            continue;
        };
        for ism_qeema in ["WineVersion", "Build", "Version"] {
            let Some(qeema) = sijill.qeema(MIFTAH_WINE, ism_qeema) else {
                continue;
            };
            let munaqqa = qeema.trim();
            if !munaqqa.is_empty()
                && !ISDARAT_WINDOWS.contains(&munaqqa.to_ascii_lowercase().as_str())
            {
                return Some(munaqqa.to_owned());
            }
        }
    }

    None
}

/// The first non-empty line of a small file, trimmed.
fn satr_awwal(masar: &Path) -> Option<String> {
    let nass = iqra_alama(masar)?;
    nass.lines().map(str::trim).find(|satr| !satr.is_empty()).map(str::to_owned)
}

/// The remainder of the first line of a small file that begins with `bidaya`,
/// trimmed and unquoted.
///
/// Enough for the one-key-per-line configuration files prefix managers write;
/// deliberately not a YAML parser, because reading one key out of `bottle.yml`
/// does not justify a YAML dependency in the discovery crate.
fn satr_bi_bidaya(masar: &Path, bidaya: &str) -> Option<String> {
    let nass = iqra_alama(masar)?;
    let matlub = bidaya.to_ascii_lowercase();
    for satr in nass.lines() {
        let munaqqa = satr.trim();
        if munaqqa.to_ascii_lowercase().starts_with(&matlub) {
            let baqi = munaqqa.get(bidaya.len()..)?.trim().trim_matches(['"', '\'']).trim();
            if !baqi.is_empty() {
                return Some(baqi.to_owned());
            }
        }
    }
    None
}

/// Reads at most [`HADD_ALAMA`] bytes of a marker file as text.
///
/// Bounded on purpose: these are files a launcher writes and a user can replace,
/// and a version probe must not become a way to make Taarib read a gigabyte.
fn iqra_alama(masar: &Path) -> Option<String> {
    let malaf = fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(HADD_ALAMA).read_to_end(&mut bayt).ok()?;
    Some(nass_min_bayt(&bayt))
}

// ---------------------------------------------------------------------------
// Translation, both directions
// ---------------------------------------------------------------------------

impl MaalumatBeea {
    /// Translates a Windows path as the game sees it into a real host path.
    ///
    /// `C:\Program Files\Game\game.exe` becomes something like
    /// `/home/deck/.steam/steam/steamapps/compatdata/440/pfx/drive_c/Program
    /// Files/Game/game.exe` — but only after each component has been resolved
    /// against what is actually on disk.
    ///
    /// ## Why the obvious approaches fail
    ///
    /// Windows paths are case-insensitive and backslash-separated. The
    /// filesystem underneath a prefix is case-sensitive and forward-slash
    /// separated. Two shortcuts suggest themselves and both are wrong:
    ///
    /// - **Lowercase the whole path and join.** `c:\program files` becomes
    ///   `drive_c/program files`, which does not exist, because what is on disk
    ///   is `Program Files`. Wine created it with that spelling and every file
    ///   in it is under that spelling.
    /// - **Keep the game's own spelling verbatim and join.** This works right up
    ///   until the game's configuration disagrees with the directory, which it
    ///   routinely does: an installer writes `C:\GAME\Data`, a launcher writes
    ///   `C:\Game\data`, a save file records `c:\game\Data`, and the directory
    ///   on disk is `Game/Data`. All four are the same path to the game and only
    ///   one of them opens on Linux.
    ///
    /// So each component is matched case-insensitively against the real
    /// directory listing, which is what Wine's own path resolution does inside
    /// `ntdll`. A component that matches verbatim is taken immediately without a
    /// listing; only a component that does not match costs a `read_dir`.
    ///
    /// ## Paths that do not exist yet
    ///
    /// Resolution stops being able to match at the first component that is not
    /// there, and from that point the remaining components are appended as
    /// written. This is deliberate and is what makes the function usable for
    /// installation: `C:\Program Files\Game\BepInEx\plugins\Taarib` must resolve
    /// to a real destination before any of `BepInEx`, `plugins` or `Taarib`
    /// exists, and the part that *does* exist — the game directory — still has
    /// to be spelled the way the disk spells it.
    ///
    /// ## What it costs
    ///
    /// One `read_dir` per component that does not match verbatim, bounded by
    /// `HADD_MUDKHALAT` entries. Nothing is cached between calls, because a
    /// cache would have to be invalidated by writes this crate does not see.
    /// A caller translating many paths under one directory should translate the
    /// directory once and join beneath the result.
    ///
    /// ## Accepted forms
    ///
    /// A drive letter and colon, then any mixture of `\` and `/` separators.
    /// `\\?\C:\…` and the NT object form `\??\C:\…` are accepted and stripped —
    /// both appear in registry values and in launcher configuration. `.` and
    /// `..` are resolved lexically.
    ///
    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurTarjamatMasar`] when the path names no drive
    /// letter, names a drive letter this prefix has no mapping for, is a UNC
    /// path (`\\server\share`, which is not a drive and has no host equivalent),
    /// contains a NUL, walks above the drive root with `..`, or carries more
    /// than `HADD_AJZA_MASAR` components.
    pub fn ila_mudif(&self, masar_windows: &str) -> Natija<PathBuf> {
        let munaqqa = masar_windows.trim();
        if munaqqa.is_empty() || munaqqa.contains('\0') {
            return Err(self.khata_tarjama(masar_windows));
        }

        let jism = munaqqa
            .strip_prefix(r"\\?\")
            .or_else(|| munaqqa.strip_prefix(r"\??\"))
            .unwrap_or(munaqqa);

        // A UNC path names a share, not a drive. Wine reaches those through its
        // own network redirector and there is no host directory to point at.
        if jism.starts_with(r"\\") || jism.starts_with("//") {
            return Err(self.khata_tarjama(masar_windows));
        }

        let mut huruf = jism.chars();
        let harf = huruf.next().unwrap_or('\0');
        if !harf.is_ascii_alphabetic() || huruf.next() != Some(':') {
            return Err(self.khata_tarjama(masar_windows));
        }
        let Some(jidhr_qurs) = self.qurs(harf) else {
            return Err(self.khata_tarjama(masar_windows));
        };
        // The letter and the colon are both ASCII, so byte 2 is a boundary.
        let baqi = jism.get(2..).unwrap_or("");

        let mut mabni = jidhr_qurs.to_path_buf();
        let mut umq = 0usize;
        let mut mafqud = false;

        for juz in baqi.split(['\\', '/']).filter(|juz| !juz.is_empty() && *juz != ".") {
            if juz == ".." {
                if umq == 0 {
                    // Walking above the drive root would leave the prefix
                    // entirely. Wine clamps this; refusing is safer, because a
                    // path that tries it came from somewhere that is confused
                    // about where it is.
                    return Err(self.khata_tarjama(masar_windows));
                }
                let _ = mabni.pop();
                umq = umq.saturating_sub(1);
                continue;
            }
            if umq >= HADD_AJZA_MASAR {
                return Err(self.khata_tarjama(masar_windows));
            }
            umq = umq.saturating_add(1);

            if mafqud {
                mabni.push(juz);
                continue;
            }
            if let Some(ism) = mutabaqa_bila_hala(&mabni, juz) {
                mabni.push(ism);
            } else {
                mafqud = true;
                mabni.push(juz);
            }
        }

        Ok(mabni)
    }

    /// Translates a real host path into the Windows path the game would see.
    ///
    /// The inverse of [`MaalumatBeea::ila_mudif`], and the direction Phase 15
    /// needs when it has to tell a game where it put something: a
    /// `doorstop_config.ini` inside a prefix must name the target assembly as
    /// `C:\…`, not as `/home/…`, because the process reading it is a Windows
    /// process.
    ///
    /// The drive is chosen by **longest match**, which is not a refinement but a
    /// requirement. `Z:` maps to `/` on a stock prefix and therefore prefixes
    /// every path on the machine, including everything under `drive_c`.
    /// Matching the first drive that fits would report the game's own files as
    /// living on `Z:`, which is a path the game can technically open and which
    /// no game's configuration ever uses. Matching the longest gives `C:`, which
    /// is what the game actually calls it.
    ///
    /// Comparison is done on canonicalized paths where the filesystem allows it,
    /// so a path reached through a symlink and the drive root it lives under
    /// still match.
    ///
    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurTarjamatMasar`] when no drive in this
    /// prefix contains the path — which is the normal answer for a path outside
    /// the prefix on a machine where `Z:` was removed — or when a component of
    /// the path is not valid UTF-8 and therefore cannot be written into a
    /// Windows path at all.
    pub fn min_mudif(&self, masar: &Path) -> Natija<String> {
        let haqiqi = fs::canonicalize(masar).unwrap_or_else(|_| sawi(masar));

        let mut afdal: Option<(char, usize, PathBuf)> = None;
        for (harf, jidhr) in &self.aqrass {
            let jidhr_haqiqi = fs::canonicalize(jidhr).unwrap_or_else(|_| sawi(jidhr));
            let Ok(baqi) = haqiqi.strip_prefix(&jidhr_haqiqi) else {
                continue;
            };
            // Component count, not string length: `/mnt/games` must not appear
            // to contain `/mnt/games-old`, and `strip_prefix` on a `Path` is
            // already component-wise for exactly that reason.
            let tul = jidhr_haqiqi.components().count();
            let abaad = afdal.as_ref().is_none_or(|(_, sabiq, _)| tul > *sabiq);
            if abaad {
                afdal = Some((*harf, tul, baqi.to_path_buf()));
            }
        }

        let Some((harf, _, baqi)) = afdal else {
            return Err(self.khata_tarjama(&masar.display().to_string()));
        };

        let mut natija = String::new();
        natija.push(harf.to_ascii_uppercase());
        natija.push_str(":\\");
        let mut awwal = true;
        for juz in baqi.components() {
            if let Component::Normal(ism) = juz {
                let Some(nass) = ism.to_str() else {
                    // A file name that is not UTF-8 has no representation in a
                    // Windows path string. Lossy conversion would produce a
                    // path that looks right and opens nothing.
                    return Err(self.khata_tarjama(&masar.display().to_string()));
                };
                if !awwal {
                    natija.push('\\');
                }
                natija.push_str(nass);
                awwal = false;
            }
        }
        Ok(natija)
    }

    /// The failure both translation directions report, naming the path and the
    /// prefix it could not be mapped through.
    fn khata_tarjama(&self, masar: &str) -> Khata {
        Khata::min_tafsir(&KhataKashf::TaadhurTarjamatMasar {
            masar: masar.to_owned(),
            beea: self.jidhr.clone(),
        })
    }
}

/// Finds the real name of one path component, ignoring case.
///
/// The verbatim spelling is tried first with a single metadata call, because it
/// is right most of the time and a directory listing is not free.
/// `symlink_metadata` rather than `metadata`: a broken symlink is still an entry
/// that exists under that name, and resolving it to the wrong case because its
/// target is missing would be a different bug.
///
/// When two entries differ only in case — legal on Linux, impossible on the
/// Windows the game thinks it is running on — the first in sort order wins, so
/// that the answer is the same on every run rather than depending on the order
/// the filesystem returned.
#[must_use]
pub fn mutabaqa_bila_hala(mujallad: &Path, ism: &str) -> Option<OsString> {
    if mujallad.join(ism).symlink_metadata().is_ok() {
        return Some(OsString::from(ism));
    }

    let matlub = ism.to_lowercase();
    let mut mutabiq: Option<OsString> = None;
    for madkhal in fs::read_dir(mujallad).ok()?.take(HADD_MUDKHALAT).flatten() {
        let mawjud = madkhal.file_name();
        if mawjud.to_string_lossy().to_lowercase() != matlub {
            continue;
        }
        let afdal = match &mutabiq {
            Some(sabiq) => mawjud < *sabiq,
            None => true,
        };
        if afdal {
            mutabiq = Some(mawjud);
        }
    }
    mutabiq
}

/// Resolves a relative path under a host directory, folding case per component.
///
/// The prefix is a Windows filesystem as far as the game is concerned, and
/// Windows does not distinguish `AppData` from `appdata`. The host underneath
/// usually does. Joining a Windows-shaped tail onto a host root verbatim
/// therefore creates a *second* directory beside the one the game uses,
/// differing only in case — the write succeeds, the installer reports success,
/// and the game never sees the file.
///
/// Each component is matched against what is on disk; the first component that
/// does not exist ends the matching, and everything from there is taken
/// literally, because a path being created is a path whose tail is supposed to
/// be absent.
#[must_use]
pub fn hall_bila_hala(jidhr: &Path, nisbi: &Path) -> PathBuf {
    let mut mabni = jidhr.to_path_buf();
    let mut mafqud = false;
    for juz in nisbi.components() {
        let Component::Normal(ism) = juz else {
            // A relative tail that is validated before it reaches here holds
            // nothing else; anything that did is passed through untouched
            // rather than silently reinterpreted.
            mabni.push(juz.as_os_str());
            continue;
        };
        if mafqud {
            mabni.push(ism);
            continue;
        }
        if let Some(mawjud) = ism.to_str().and_then(|ism| mutabaqa_bila_hala(&mabni, ism)) {
            mabni.push(mawjud);
        } else {
            mafqud = true;
            mabni.push(ism);
        }
    }
    mabni
}

// ---------------------------------------------------------------------------
// Wine's registry text format
// ---------------------------------------------------------------------------

/// Reads and parses a Wine registry file.
///
/// ## The format
///
/// Wine's `.reg` files are a text serialization of one registry hive, close to
/// the `regedit` export format but not identical to it. The grammar implemented
/// here:
///
/// ```text
/// WINE REGISTRY Version 2            header line, ignored
/// ;; comment                         a line beginning with ';'
/// #arch=win64                        directive: the prefix's architecture
///
/// [Software\\Wine\\DllOverrides] 1761234567
/// #time=1db2c3d4e5f6a70                the key's modification time
/// #class="AppUserModelId"              the key's class, when it has one
/// #link                                the key is a symbolic link
/// @="default value"                    the key's unnamed default value
/// "winhttp"="native,builtin"           REG_SZ
/// "Count"=dword:0000000a               REG_DWORD, always eight hex digits
/// "Path"=str(2):"%SystemRoot%\\system32"   REG_EXPAND_SZ, as text
/// "List"=str(7):"one\0two\0"           REG_MULTI_SZ, as text
/// "Blob"=hex:01,02,03,\                REG_BINARY, continued on the next line
///   04,05
/// "Wide"=hex(2):25,00,53,00,00,00      a typed value in raw UTF-16LE bytes
/// ```
///
/// Section headers carry the key path with `\` escaped as `\\`, followed by a
/// decimal modification timestamp. Value names and string values are escaped the
/// same way, plus `\"`, `\n`, `\r`, `\t`, `\0` and `\xHHHH` for an arbitrary
/// UTF-16 code unit. A `hex` value continues onto the next line whenever the
/// line ends in a backslash, and a long binary value routinely spans dozens.
///
/// ## How values are normalized
///
/// Every value is presented as a `String`, because every consumer in Taarib
/// wants text and a typed registry value model would be a type nobody reads:
///
/// - strings, `str(N):` and the typed `hex(1)`, `hex(2)`, `hex(7)` forms are
///   decoded to text, with the UTF-16 forms decoded from their raw bytes and
///   `REG_MULTI_SZ` elements separated by newlines,
/// - `dword:` and `hex(4):` become the decimal number,
/// - `hex:` and every other typed binary become lowercase hexadecimal with no
///   separators.
///
/// ## Robustness
///
/// A line that does not parse is skipped rather than failing the file. Prefix
/// registries are edited by installers, by `winetricks`, by launchers and by
/// hand, and a single malformed line in a file with forty thousand of them must
/// not cost the caller the DLL overrides it came for. A UTF-16 byte order mark
/// is honoured, and bytes that are not valid UTF-8 are replaced rather than
/// rejected — old prefixes carry values written in a legacy code page, and one
/// of them must not hide the whole registry.
///
/// # Errors
///
/// Returns [`KhataKashf::TaadhurQiraatSijillBeea`] when the file cannot be
/// opened or read. Parsing itself does not fail.
pub fn iqra_sijill(masar: &Path) -> Natija<SijillBeea> {
    let bayt = fs::read(masar).map_err(|sabab| {
        Khata::min_tafsir(&KhataKashf::TaadhurQiraatSijillBeea {
            masar: masar.to_path_buf(),
            sabab,
        })
    })?;
    Ok(hallil_sijill(masar, &nass_min_bayt(&bayt)))
}

/// Parses the text of a registry file.
fn hallil_sijill(masar: &Path, nass: &str) -> SijillBeea {
    let mut mafatih: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut mimariya = None;
    let mut hali: Option<(String, BTreeMap<String, String>)> = None;
    let mut sutur = nass.lines();

    while let Some(satr_kham) = sutur.next() {
        let satr = satr_kham.trim();
        if satr.is_empty() || satr.starts_with(';') {
            continue;
        }

        if let Some(tawjih) = satr.strip_prefix('#') {
            // `#time=`, `#class=` and `#link` describe the key that precedes
            // them, not a value, and Taarib needs none of the three. `#arch=` is
            // the one directive that carries an answer somebody asks for.
            if let Some(qeema) = tawjih.strip_prefix("arch=") {
                mimariya = Some(qeema.trim().to_owned());
            }
            continue;
        }

        if satr.starts_with('[') {
            if let Some((ism, qeem)) = hali.take() {
                adkhil_miftah(&mut mafatih, &ism, qeem);
            }
            hali = ism_miftah(satr).map(|ism| (ism, BTreeMap::new()));
            continue;
        }

        // Anything left that is not a value line is the format's own header, or
        // damage. Both are skipped.
        let Some((ism, kham)) = qassim_qeema(satr) else {
            continue;
        };

        let mut kamil = kham.trim().to_owned();
        // Only binary values continue across lines, and they announce
        // themselves. Testing the trailing backslash alone would swallow the
        // line after any string value ending in an escaped separator.
        while kamil.starts_with("hex") && kamil.ends_with('\\') {
            let Some(taali) = sutur.next() else {
                break;
            };
            let _ = kamil.pop();
            kamil.push_str(taali.trim());
        }

        if let Some((_, qeem)) = hali.as_mut() {
            let _ = qeem.insert(ism, hallil_qeema(&kamil));
        }
    }

    if let Some((ism, qeem)) = hali.take() {
        adkhil_miftah(&mut mafatih, &ism, qeem);
    }

    SijillBeea { masar: masar.to_path_buf(), mafatih, mimariya }
}

/// Files a key's values, merging when the same key appears twice — which it does
/// in a hive that has been rewritten in place.
fn adkhil_miftah(
    mafatih: &mut BTreeMap<String, BTreeMap<String, String>>,
    ism: &str,
    qeem: BTreeMap<String, String>,
) {
    mafatih.entry(tabi_miftah(ism)).or_default().extend(qeem);
}

/// Normalizes a key path for lookup: unescaped, lowercase, no leading or
/// trailing separator.
///
/// Accepts a path written either way, because the file spells it
/// `Software\\Wine` and a caller spells it `Software\Wine`, and the registry
/// itself considers key names case-insensitive.
fn tabi_miftah(ism: &str) -> String {
    ism.trim().replace("\\\\", "\\").trim_matches('\\').to_lowercase()
}

/// Reads the key path out of a section header.
///
/// The closing bracket is the first one not preceded by a backslash: a key whose
/// own name contains `]` escapes it, and stopping at the wrong bracket would
/// truncate the key silently.
fn ism_miftah(satr: &str) -> Option<String> {
    let dakhil = satr.strip_prefix('[')?;
    let mut harab = false;
    let mut nihaya = None;
    for (mawqi, harf) in dakhil.char_indices() {
        if harab {
            harab = false;
            continue;
        }
        match harf {
            '\\' => harab = true,
            ']' => {
                nihaya = Some(mawqi);
                break;
            }
            _ => {}
        }
    }
    Some(fukk_harab(dakhil.get(..nihaya?)?))
}

/// Splits a value line into its name and its raw value.
///
/// `@=…` is the key's default value and has the empty name, which is what it has
/// in the registry too.
fn qassim_qeema(satr: &str) -> Option<(String, &str)> {
    if let Some(baqi) = satr.strip_prefix("@=") {
        return Some((String::new(), baqi));
    }
    let dakhil = satr.strip_prefix('"')?;
    let mut harab = false;
    for (mawqi, harf) in dakhil.char_indices() {
        if harab {
            harab = false;
            continue;
        }
        match harf {
            '\\' => harab = true,
            '"' => {
                let ism = fukk_harab(dakhil.get(..mawqi)?);
                // The quote is one byte, so the next boundary is `mawqi + 1`.
                let baqi = dakhil.get(mawqi.saturating_add(1)..)?.strip_prefix('=')?;
                return Some((ism, baqi));
            }
            _ => {}
        }
    }
    None
}

/// Turns one raw value into the text form described on [`iqra_sijill`].
fn hallil_qeema(kham: &str) -> String {
    let kham = kham.trim();

    if let Some(dakhil) = kham.strip_prefix('"') {
        return fukk_nass_muqtabas(dakhil);
    }

    if let Some(raqm) = kham.strip_prefix("dword:") {
        let munaqqa = raqm.trim();
        return u32::from_str_radix(munaqqa, 16)
            .map_or_else(|_| munaqqa.to_owned(), |qeema| qeema.to_string());
    }

    // Wine's own extension: a typed string written as text rather than as raw
    // UTF-16 bytes. `str(2)` is REG_EXPAND_SZ, `str(7)` is REG_MULTI_SZ.
    if let Some(baqi) = kham.strip_prefix("str(")
        && let Some((_, nass)) = baqi.split_once("):")
    {
        let munaqqa = nass.trim();
        return munaqqa.strip_prefix('"').map_or_else(|| munaqqa.to_owned(), fukk_nass_muqtabas);
    }

    if let Some(baqi) = kham.strip_prefix("hex(")
        && let Some((naw, bayanat)) = baqi.split_once("):")
    {
        let bayt = bayt_min_hex(bayanat);
        return match u32::from_str_radix(naw.trim(), 16) {
            // REG_SZ, REG_EXPAND_SZ, REG_MULTI_SZ: raw UTF-16LE.
            Ok(1 | 2 | 7) => nass_min_utf16(&bayt),
            // REG_DWORD and its big-endian twin.
            Ok(4) => raqm_min_bayt(&bayt, false),
            Ok(5) => raqm_min_bayt(&bayt, true),
            _ => hex_nass(&bayt),
        };
    }

    if let Some(bayanat) = kham.strip_prefix("hex:") {
        return hex_nass(&bayt_min_hex(bayanat));
    }

    kham.to_owned()
}

/// Unescapes a quoted string, stopping at the closing quote.
fn fukk_nass_muqtabas(dakhil: &str) -> String {
    let mut harab = false;
    for (mawqi, harf) in dakhil.char_indices() {
        if harab {
            harab = false;
            continue;
        }
        match harf {
            '\\' => harab = true,
            '"' => return dakhil.get(..mawqi).map_or_else(String::new, fukk_harab),
            _ => {}
        }
    }
    fukk_harab(dakhil)
}

/// Applies the escape rules of the format.
///
/// `\\`, `\"`, `\n`, `\r`, `\t`, `\a`, `\b`, `\f`, `\v`, `\0`, and `\xHHHH` for
/// up to four hexadecimal digits. An unknown escape yields the escaped character
/// itself, which is what Wine's own reader does.
fn fukk_harab(kham: &str) -> String {
    let mut natija = String::with_capacity(kham.len());
    let mut huruf = kham.chars().peekable();
    while let Some(harf) = huruf.next() {
        if harf != '\\' {
            natija.push(harf);
            continue;
        }
        match huruf.next() {
            Some('n') => natija.push('\n'),
            Some('r') => natija.push('\r'),
            Some('t') => natija.push('\t'),
            Some('a') => natija.push('\u{7}'),
            Some('b') => natija.push('\u{8}'),
            Some('f') => natija.push('\u{c}'),
            Some('v') => natija.push('\u{b}'),
            Some('0') => natija.push('\0'),
            Some('x') => {
                let mut qeema = 0u32;
                let mut adad = 0usize;
                while adad < 4 {
                    let Some(raqam) = huruf.peek().and_then(|harf| harf.to_digit(16)) else {
                        break;
                    };
                    qeema = qeema.saturating_mul(16).saturating_add(raqam);
                    let _ = huruf.next();
                    adad = adad.saturating_add(1);
                }
                if adad == 0 {
                    natija.push('x');
                } else {
                    natija.push(char::from_u32(qeema).unwrap_or('\u{fffd}'));
                }
            }
            Some(akhar) => natija.push(akhar),
            None => natija.push('\\'),
        }
    }
    natija
}

/// Decodes the comma-separated hexadecimal byte list of a binary value,
/// tolerating the trailing backslash of a continued line.
fn bayt_min_hex(bayanat: &str) -> Vec<u8> {
    bayanat
        .split(',')
        .filter_map(|juz| {
            let munaqqa = juz.trim().trim_end_matches('\\').trim();
            if munaqqa.is_empty() { None } else { u8::from_str_radix(munaqqa, 16).ok() }
        })
        .collect()
}

/// Renders bytes as lowercase hexadecimal with no separators.
fn hex_nass(bayt: &[u8]) -> String {
    let mut natija = String::with_capacity(bayt.len().saturating_mul(2));
    for wahda in bayt {
        let _ = write!(natija, "{wahda:02x}");
    }
    natija
}

/// Decodes a four-byte registry number.
fn raqm_min_bayt(bayt: &[u8], kabir: bool) -> String {
    match bayt.get(..4).and_then(|juz| <[u8; 4]>::try_from(juz).ok()) {
        Some(arbaa) => {
            let qeema = if kabir { u32::from_be_bytes(arbaa) } else { u32::from_le_bytes(arbaa) };
            qeema.to_string()
        }
        None => hex_nass(bayt),
    }
}

/// Decodes raw UTF-16LE registry bytes.
///
/// A `REG_MULTI_SZ` is a run of NUL-terminated strings ending in an empty one;
/// the trailing NULs are dropped and the separators become newlines, so the
/// value reads as the list it is.
fn nass_min_utf16(bayt: &[u8]) -> String {
    let wahdat: Vec<u16> = bayt
        .chunks_exact(2)
        .filter_map(|juz| <[u8; 2]>::try_from(juz).ok())
        .map(u16::from_le_bytes)
        .collect();
    String::from_utf16_lossy(&wahdat).trim_end_matches('\0').replace('\0', "\n")
}

/// Decodes file bytes to text, honouring a byte order mark.
///
/// Wine writes UTF-8 without a mark, but a `.reg` produced on Windows and
/// dropped into a prefix by an installer is UTF-16LE with one, and reading that
/// as UTF-8 yields a file that appears to contain no keys at all.
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
        }
        Some([0xFE, 0xFF]) => {
            let wahdat: Vec<u16> = bayt
                .get(2..)
                .unwrap_or_default()
                .chunks_exact(2)
                .filter_map(|juz| <[u8; 2]>::try_from(juz).ok())
                .map(u16::from_be_bytes)
                .collect();
            String::from_utf16_lossy(&wahdat)
        }
        _ => String::from_utf8_lossy(bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bayt))
            .into_owned(),
    }
}

// ---------------------------------------------------------------------------
// DLL overrides
// ---------------------------------------------------------------------------

impl MaalumatBeea {
    /// The prefix's DLL load-order overrides, as `(module, order)` pairs.
    ///
    /// This is the key Phase 15 has to get right. BepInEx loads into a Windows
    /// game through a proxy DLL — `winhttp.dll` next to the executable — and
    /// Wine will only load that copy instead of its own builtin when the module
    /// is overridden to `native,builtin`. Without the override the proxy is
    /// never loaded, the plugin never initializes, and the game runs untouched
    /// with no error anywhere.
    ///
    /// Both hives are read and merged with the precedence Wine itself uses:
    /// `HKEY_LOCAL_MACHINE` from `system.reg` first, then
    /// `HKEY_CURRENT_USER` from `user.reg` on top of it, because a per-user
    /// override wins. Module names are lowercased, which is how Wine matches
    /// them and what keeps `WinHTTP` and `winhttp` from arriving as two
    /// different overrides. The `*` prefix that forces an override ahead of the
    /// application's own list is preserved.
    ///
    /// The order string is Wine's own: a comma-separated preference list drawn
    /// from `native`, `builtin` and the older aliases `n` and `b`, and an empty
    /// value meaning the module is disabled entirely.
    ///
    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurQiraatSijillBeea`] when `user.reg` cannot be
    /// read — without it the prefix's overrides are unknown, and Phase 15 must
    /// not write a new override on top of a state it could not see. A missing
    /// `system.reg` is not an error: the machine hive contributes defaults, and
    /// a prefix mid-build may not have written it yet.
    pub fn tajawuzat_dll(&self) -> Natija<Vec<(String, String)>> {
        let mut mudmaj: BTreeMap<String, String> = BTreeMap::new();

        let nizam = self.masar_sijill_nizam();
        if nizam.is_file() {
            let sijill = iqra_sijill(&nizam)?;
            idmaj_tajawuzat(&sijill, &mut mudmaj);
        }

        let sijill = iqra_sijill(&self.masar_sijill_mustakhdim())?;
        idmaj_tajawuzat(&sijill, &mut mudmaj);

        Ok(mudmaj.into_iter().collect())
    }
}

/// Folds one hive's `DllOverrides` key into the merged map.
fn idmaj_tajawuzat(sijill: &SijillBeea, mudmaj: &mut BTreeMap<String, String>) {
    let Some(miftah) = sijill.miftah(MIFTAH_TAJAWUZAT) else {
        return;
    };
    for (ism, qeema) in miftah {
        let _ = mudmaj.insert(ism.to_lowercase(), qeema.clone());
    }
}

// ---------------------------------------------------------------------------
// Finding prefixes
// ---------------------------------------------------------------------------

/// Resolves the compatibility prefix Steam keeps for one application.
///
/// Steam puts it at `steamapps/compatdata/<appid>/pfx`, relative to the library
/// folder the game is installed in — not relative to the Steam root, which is
/// why `jidhr_steam` is whichever root the caller is holding. A library root, a
/// Steam root and a `steamapps` directory are all accepted, and the older
/// `SteamApps` capitalization is checked as well, because a Linux install that
/// predates the rename still has it and the filesystem still cares.
///
/// The result is verified rather than assumed. Steam creates
/// `compatdata/<appid>` the moment a game is set to run under a compatibility
/// tool, and it stays an empty directory — or one holding only `pfx.lock` — until
/// the game is first launched and Proton populates it. Returning that directory
/// because it exists is how a patch gets installed into nothing: every write
/// succeeds, the manifest records them, and the game never sees any of it. So
/// the answer is only returned when it is a real prefix by [`hiya_beea`] — a
/// `drive_c` and a `user.reg`.
///
/// Returns `None` when there is no populated prefix for that application, which
/// is the correct and expected answer for a game that has never been run.
#[must_use]
pub fn beea_steam(jidhr_steam: &Path, app: u32) -> Option<PathBuf> {
    let raqm = app.to_string();
    for asas in [
        jidhr_steam.join("steamapps"),
        jidhr_steam.join("SteamApps"),
        jidhr_steam.to_path_buf(),
    ] {
        let tawafuq = asas.join("compatdata").join(&raqm);
        // `pfx` is where Proton builds it; the bare directory covers the
        // layouts where a compatibility tool used it as the prefix directly.
        for murashah in [tawafuq.join("pfx"), tawafuq] {
            if hiya_beea(&murashah) {
                return Some(fs::canonicalize(&murashah).unwrap_or(murashah));
            }
        }
    }
    None
}

/// Finds Wine prefixes that Steam did not create.
///
/// Steam's own prefixes are [`beea_steam`]'s job, resolved per application from
/// the library the game lives in; this covers everything else, and on a Linux
/// machine that is most of what a person actually has. Every location is
/// checked and only what exists and is a real prefix comes back.
///
/// | launcher | native | Flatpak |
/// | --- | --- | --- |
/// | bare Wine | `$WINEPREFIX`, `~/.wine`, `~/.local/share/wineprefixes/*` | `~/.var/app/org.winehq.Wine/data/wineprefixes/*` |
/// | Lutris | `~/Games/*`, `~/.local/share/lutris/runners/wine/*` | `~/.var/app/net.lutris.Lutris/data/Games/*`, `~/.var/app/net.lutris.Lutris/data/lutris/runners/wine/*` |
/// | Bottles | `~/.local/share/bottles/bottles/<name>` | `~/.var/app/com.usebottles.bottles/data/bottles/bottles/<name>` |
/// | Heroic | `~/Games/Heroic/Prefixes/**`, `~/.config/heroic/Prefixes/**`, `~/.config/Prefixes/**` | `~/.var/app/com.heroicgameslauncher.hgl/{config/heroic,data/heroic,Games/Heroic}/Prefixes/**` |
/// | `PlayOnLinux` | `~/.PlayOnLinux/wineprefix/*` | — |
///
/// Each listed parent is scanned two levels deep, and at every level both the
/// directory itself and a `pfx` inside it are tested. Two levels is what the
/// real layouts need: Heroic nests prefixes under a profile directory
/// (`Prefixes/default/<game>`), and Lutris puts a bare prefix directly under
/// `~/Games/<slug>`. Deeper than that and a scan of `~/Games` would start
/// walking somebody's entire game library.
///
/// `$WINEPREFIX` is read from the environment because that is the only place it
/// exists — it is Wine's own interface, not Taarib configuration, and a user who
/// exported it has told the system where their prefix is.
///
/// Results are canonicalized and deduplicated, so the same prefix reached
/// through two paths — `~/.wine` and a `wineprefixes` entry symlinked to it —
/// appears once. Order is stable across runs.
#[must_use]
pub fn iktashif_beeat(manzil: &Path) -> Vec<PathBuf> {
    let mut natija: Vec<PathBuf> = Vec::new();
    let mut ruit: BTreeSet<PathBuf> = BTreeSet::new();

    let mut mubashira: Vec<PathBuf> = Vec::new();
    if let Some(muhaddad) = std::env::var_os("WINEPREFIX") {
        let masar = PathBuf::from(muhaddad);
        if !masar.as_os_str().is_empty() {
            mubashira.push(masar);
        }
    }
    mubashira.push(manzil.join(".wine"));
    for murashah in &mubashira {
        daf_beea(&mut natija, &mut ruit, murashah);
    }

    let flatpak = manzil.join(".var").join("app");
    let lutris = flatpak.join("net.lutris.Lutris").join("data");
    let heroic = flatpak.join("com.heroicgameslauncher.hgl");

    let walidun = [
        // Bare Wine and winetricks' convention for extra prefixes.
        manzil.join(".local").join("share").join("wineprefixes"),
        flatpak.join("org.winehq.Wine").join("data").join("wineprefixes"),
        manzil.join(".PlayOnLinux").join("wineprefix"),
        // Lutris: prefixes default under ~/Games, and the runner tree is
        // scanned too because a user who pointed Lutris there gets prefixes
        // there.
        manzil.join("Games"),
        manzil.join(".local").join("share").join("lutris").join("runners").join("wine"),
        lutris.join("Games"),
        lutris.join("lutris").join("runners").join("wine"),
        // Bottles: one directory per bottle, each of which is itself a prefix
        // with the bottle configuration sitting inside it.
        manzil.join(".local").join("share").join("bottles").join("bottles"),
        flatpak.join("com.usebottles.bottles").join("data").join("bottles").join("bottles"),
        // Heroic: a Prefixes tree, in any of the several places Heroic has kept
        // it, plus the Flatpak layout of each.
        manzil.join("Games").join("Heroic").join("Prefixes"),
        manzil.join(".config").join("heroic").join("Prefixes"),
        manzil.join(".config").join("Prefixes"),
        heroic.join("config").join("heroic").join("Prefixes"),
        heroic.join("config").join("Prefixes"),
        heroic.join("data").join("heroic").join("Prefixes"),
        heroic.join("Games").join("Heroic").join("Prefixes"),
    ];

    for walid in &walidun {
        ijma_beeat(walid, 2, &mut natija, &mut ruit);
    }

    natija
}

/// Scans one directory for prefixes, `umq` levels deep.
fn ijma_beeat(walid: &Path, umq: usize, natija: &mut Vec<PathBuf>, ruit: &mut BTreeSet<PathBuf>) {
    if umq == 0 || !walid.is_dir() {
        return;
    }

    let Ok(qaima) = fs::read_dir(walid) else {
        return;
    };
    let mut abna: Vec<PathBuf> = qaima
        .take(HADD_MUDKHALAT)
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| masar.is_dir())
        .collect();
    abna.sort();

    for ibn in abna {
        if hiya_beea(&ibn) {
            daf_beea(natija, ruit, &ibn);
            continue;
        }
        let dakhili = ibn.join("pfx");
        if hiya_beea(&dakhili) {
            daf_beea(natija, ruit, &dakhili);
            continue;
        }
        ijma_beeat(&ibn, umq.saturating_sub(1), natija, ruit);
    }
}

/// Records a candidate, once, if it really is a prefix.
fn daf_beea(natija: &mut Vec<PathBuf>, ruit: &mut BTreeSet<PathBuf>, murashah: &Path) {
    if !hiya_beea(murashah) {
        return;
    }
    let mutlaq = fs::canonicalize(murashah).unwrap_or_else(|_| sawi(murashah));
    if ruit.insert(mutlaq.clone()) {
        natija.push(mutlaq);
    }
}

// ---------------------------------------------------------------------------
// The user profile inside the prefix
// ---------------------------------------------------------------------------

impl MaalumatBeea {
    /// The prefix's user profile directory — what the game calls
    /// `C:\users\<name>`.
    ///
    /// Needed wherever a game keeps something under the profile rather than
    /// beside its executable: `AppData`, `Documents\My Games`, `Saved Games`.
    /// Those are the paths a configuration file names as `%USERPROFILE%\…`, and
    /// they are inside the prefix, not in the real home directory — writing to
    /// the real one is a change the game will never see.
    ///
    /// Resolution, in order:
    ///
    /// 1. **`steamuser`.** Proton names the profile `steamuser` unconditionally,
    ///    whoever is logged in, so on a Steam Deck this is always the answer.
    /// 2. **The login name.** Wine names it after the user who created the
    ///    prefix, which is normally the user running Taarib.
    /// 3. **The only remaining profile**, once the system profiles Windows
    ///    always has — `Public`, `Default`, `All Users` — are excluded. When
    ///    several remain the first in sort order is taken, deterministically.
    ///
    /// Both profile layouts are searched: `drive_c/users`, and the
    /// `drive_c/windows/profiles` of a prefix built against a much older Wine.
    ///
    /// `None` when the prefix has no profile directory at all, which means it
    /// was created but never booted.
    #[must_use]
    pub fn mujallad_mustakhdim(&self) -> Option<PathBuf> {
        let qurs_c = self.drive_c();
        [qurs_c.join("users"), qurs_c.join("windows").join("profiles")]
            .iter()
            .find_map(|asas| mustakhdim_fi(asas))
    }
}

/// Picks the user profile out of one profiles directory.
fn mustakhdim_fi(asas: &Path) -> Option<PathBuf> {
    if !asas.is_dir() {
        return None;
    }

    let steamuser = asas.join("steamuser");
    if steamuser.is_dir() {
        return Some(steamuser);
    }

    // Wine's own profile is named after the login. `var_os`, not `var`: a
    // login name is not required to be UTF-8, and this is the operating
    // system's answer rather than Taarib configuration.
    let ism_hali = std::env::var_os("USER")
        .or_else(|| std::env::var_os("USERNAME"))
        .and_then(|qeema| qeema.into_string().ok())
        .map(|qeema| qeema.to_lowercase());

    let asma_nizam = ["public", "all users", "default", "default user", "defaultuser0"];
    let mut murashahun: Vec<PathBuf> = Vec::new();

    for madkhal in fs::read_dir(asas).ok()?.take(HADD_MUDKHALAT).flatten() {
        let masar = madkhal.path();
        if !masar.is_dir() {
            continue;
        }
        let ism = madkhal.file_name().to_string_lossy().to_lowercase();
        if asma_nizam.contains(&ism.as_str()) {
            continue;
        }
        if ism_hali.as_deref() == Some(ism.as_str()) {
            return Some(masar);
        }
        murashahun.push(masar);
    }

    murashahun.sort();
    murashahun.into_iter().next()
}
