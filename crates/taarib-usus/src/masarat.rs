//! المسارات — the on-disk layout, path safety, and atomic writes.
//!
//! Every path Taarib touches is derived here. No other crate joins a user
//! directory to a name, because every such join is a place where a malicious
//! patch, a hostile archive entry, or a game with a strange install directory
//! could write outside where it is allowed to.
//!
//! The layout, identical in shape on all three platforms:
//!
//! ```text
//! Windows  %APPDATA%\Taarib\            (settings alongside)
//! macOS    ~/Library/Application Support/Taarib/
//! Linux    $XDG_DATA_HOME/taarib/       (settings in $XDG_CONFIG_HOME/taarib/)
//!
//!   taarib.db      local database          mashari/   translation projects
//!   idadat.json    settings                khutut/    user-supplied fonts
//!   ruqaa/         patches, content-addressed          sijillat/  logs and bundles
//!   nusakh/        original-file backups   dhakira/   translation memory
//!   makhbaa/       registry and artwork cache          sandooq/   sandbox (owner)
//!   mafatih/       key material — never committed, never bundled
//! ```
//!
//! The two roots resolve in a fixed order: the `TAARIB_BAYANAT` override, then
//! the portable marker — a file named `taarib.mahmul` beside the executable —
//! then the platform defaults above. Under the marker, both roots live inside
//! one `bayanat` directory next to the executable, and the machine itself
//! keeps nothing.
//!
//! One directory in the layout moves on its own: `ruqaa/`, through
//! [`Masarat::maa_jidhr_ruqaa`], for a patch library that has outgrown a small
//! system drive. Everything else is derived from the two roots and always will
//! be — a second movable directory is a second way for the layout to be half
//! somewhere else.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use crate::khata::{
    Khata, Khutura, Khutwa, MasarMatlub, Natija, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io,
    siyaq_io,
};
use crate::khata_min;

/// Longest path Taarib will construct before refusing.
///
/// Windows enforces 260 characters unless long paths are enabled system-wide,
/// and a game installed deep inside a Steam library on a secondary drive gets
/// close on its own. Refusing at 240 leaves room for the file names Taarib adds
/// on top of whatever the user already has.
const HADD_TUL_MASAR: usize = 240;

/// Device names Windows resolves before it ever looks at the filesystem. An
/// archive entry called `COM1` is not a file; it is a handle to a serial port.
const ASMAA_MAHJOOZA: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The portable marker: a file with this exact name beside the executable.
///
/// Its presence is the user's statement "leave this machine untouched" — run
/// from the drive, keep everything on the drive. The data root moves to
/// `bayanat` beside the executable, and the marker forbids every
/// machine-global write: no keychain entries, no registry, no `~/.config`.
const ALAMAT_MAHMUL: &str = "taarib.mahmul";

/// The resolved locations Taarib reads and writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Masarat {
    bayanat: PathBuf,
    idadat: PathBuf,
    manzil: PathBuf,
    mahmul: bool,
    /// Patch storage, when it has been moved off the data root. `None` is the
    /// ordinary case and means `bayanat/ruqaa`. See [`Masarat::maa_jidhr_ruqaa`].
    ruqaa: Option<PathBuf>,
}

/// The environment variable naming the data root.
///
/// Public because the settings layer scans every `TAARIB_*` variable as a
/// settings path and has to know which names are not settings. Naming them
/// here rather than repeating the strings there is what keeps the two sides
/// from disagreeing: they did, and the disagreement meant that setting the
/// override this module documents made the application refuse to start.
pub const BEEAT_BAYANAT: &str = "TAARIB_BAYANAT";

/// The environment variable naming the settings root. See [`BEEAT_BAYANAT`].
pub const BEEAT_IDADAT: &str = "TAARIB_IDADAT";

/// Every `TAARIB_*` name that is not a settings path.
///
/// The first two are this module's own roots, read before a settings file can
/// be located. The third is stranger and worth stating: the studio's build
/// script emits `cargo:rustc-env=TAARIB_HADAF` so the binary can name the target
/// triple it was compiled for, and **cargo puts a `rustc-env` variable into the
/// runtime environment of `cargo run` and `cargo test` as well as into the
/// compilation**. So every developer running the application the ordinary way
/// had it refuse to start, naming a setting nobody wrote.
///
/// A `TAARIB_*` name added for anything other than a setting belongs on this
/// list. The scan is deliberately strict — an unrecognised name is a typo that
/// silently loses somebody's configuration — and this is the seam that keeps
/// strictness from turning on the product itself.
pub const ASMAA_GHAYR_IDADAT: [&str; 3] = [BEEAT_BAYANAT, BEEAT_IDADAT, "TAARIB_HADAF"];

impl Masarat {
    /// Resolves the layout for this machine.
    ///
    /// In order: the `TAARIB_BAYANAT` and `TAARIB_IDADAT` overrides, which is
    /// how the owner's sandbox works; then the portable marker, a file named
    /// `taarib.mahmul` beside the executable, which is how an installation
    /// carried on a removable drive works; then the platform defaults.
    ///
    /// # Errors
    ///
    /// Fails when the platform has no home directory to derive from, which
    /// happens on service accounts and inside stripped containers.
    pub fn iktashif() -> Natija<Self> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the two root overrides are the one configuration input that must be \
                      readable before any configuration file can be located"
        )]
        let (bayanat_env, idadat_env) = (
            std::env::var_os(BEEAT_BAYANAT).map(PathBuf::from),
            std::env::var_os(BEEAT_IDADAT).map(PathBuf::from),
        );

        let qawaid = directories::BaseDirs::new().ok_or_else(|| {
            Khata::min_tafsir(&KhataMasarat::LaMujalladManzili)
        })?;

        // The override outranks the marker; a layout it redirected is not a portable one.
        let hamil = if bayanat_env.is_some() { None } else { mujallad_mahmul() };

        let bayanat = bayanat_env.unwrap_or_else(|| {
            if let Some(mujallad) = &hamil {
                mujallad.join("bayanat")
            } else if cfg!(target_os = "linux") {
                qawaid.data_dir().join("taarib")
            } else {
                qawaid.data_dir().join("Taarib")
            }
        });

        let idadat = idadat_env.unwrap_or_else(|| {
            if hamil.is_some() {
                // Inside the data root, so one directory beside the executable holds everything.
                bayanat.join("idadat")
            } else if cfg!(target_os = "linux") {
                qawaid.config_dir().join("taarib")
            } else {
                bayanat.clone()
            }
        });

        let manzil = qawaid.home_dir().to_path_buf();
        Ok(Self { bayanat, idadat, manzil, mahmul: hamil.is_some(), ruqaa: None })
    }

    /// Builds a layout rooted anywhere — used by the sandbox and by tests of
    /// the installer against a copied game. Explicit roots are by definition
    /// not the portable marker's doing: `mahmul()` is always `false` here.
    #[must_use]
    pub fn min_judhur(bayanat: impl Into<PathBuf>, idadat: impl Into<PathBuf>) -> Self {
        let bayanat = bayanat.into();
        // A rooted layout's "home" is the directory the roots were placed in.
        // Discovery scans from it, so a sandbox that pointed it at the real home
        // directory would have the sandbox scanning the machine it is isolating
        // itself from.
        let manzil = bayanat.parent().unwrap_or(&bayanat).to_path_buf();
        Self { bayanat, idadat: idadat.into(), manzil, mahmul: false, ruqaa: None }
    }

    /// Re-roots patch storage at the directory the user chose, or puts it back
    /// under the data root when they chose none.
    ///
    /// This is the consumer of [`crate::idadat::IdadatTakhzin::jidhr_ruqaa`],
    /// which is a setting the Studio has always let people edit and which
    /// nothing on this side had ever read. It is applied here rather than in
    /// [`Masarat::iktashif`] because the ordering does not allow anything else:
    /// the settings file is located *through* a layout, so the layout has to
    /// exist before the setting that adjusts it can be read. A caller resolves
    /// the layout, opens the settings over it, and re-roots with the answer —
    /// before [`Masarat::takid`] creates the directories, so the chosen root is
    /// the one that gets made.
    ///
    /// Only `ruqaa/` moves. Backups, projects and the cache stay where they
    /// are: the setting exists for people whose patch library outgrew a small
    /// system drive, and widening it to the whole data root is what
    /// `TAARIB_BAYANAT` is for.
    ///
    /// **Nothing is copied.** Patches already staged under the old root are not
    /// found through the new one, which is the same bargain the environment
    /// override makes and is why the setting is worded as somewhere the user
    /// *moved* storage to.
    #[must_use]
    pub fn maa_jidhr_ruqaa(mut self, jidhr: Option<&Path>) -> Self {
        self.ruqaa = jidhr
            .map(Path::to_path_buf)
            .filter(|masar| !masar.as_os_str().is_empty());
        self
    }

    /// The data root.
    #[must_use]
    pub fn jidhr_bayanat(&self) -> &Path {
        &self.bayanat
    }

    /// The configuration root.
    #[must_use]
    pub fn jidhr_idadat(&self) -> &Path {
        &self.idadat
    }

    /// The user's home directory.
    ///
    /// Resolved once here and handed to whoever needs it, which for discovery
    /// is seventeen launcher adapters. `taarib_kashf::siyaq_fahs` says the same
    /// thing about its own parameter: seventeen adapters must not arrive at
    /// seventeen different answers about where the user's files are, and two
    /// independent `BaseDirs` lookups in one process is how that starts.
    #[must_use]
    pub fn manzil(&self) -> &Path {
        &self.manzil
    }

    /// Whether this layout resolved through the portable marker.
    ///
    /// `true` is the user's statement "leave this machine untouched" made
    /// operative: the roots sit beside the executable, and every
    /// machine-global write — a keychain entry, a registry key, anything
    /// under `~/.config` — is forbidden. Enforcing that rule belongs to the
    /// callers making such writes, because only they can name what they
    /// refused; this module only answers where and whether.
    #[must_use]
    pub const fn mahmul(&self) -> bool {
        self.mahmul
    }

    /// The local database.
    #[must_use]
    pub fn qaida_bayanat(&self) -> PathBuf {
        self.bayanat.join("taarib.db")
    }

    /// The settings file.
    #[must_use]
    pub fn malaf_idadat(&self) -> PathBuf {
        self.idadat.join("idadat.json")
    }

    /// Downloaded and imported patches, content-addressed.
    ///
    /// The one directory in the layout a user can move on their own, through
    /// [`Masarat::maa_jidhr_ruqaa`]; everything else is derived from the roots.
    #[must_use]
    pub fn ruqaa(&self) -> PathBuf {
        self.ruqaa.clone().unwrap_or_else(|| self.bayanat.join("ruqaa"))
    }

    /// Original-file backups, one directory per installation.
    #[must_use]
    pub fn nusakh(&self) -> PathBuf {
        self.bayanat.join("nusakh")
    }

    /// Translation projects.
    #[must_use]
    pub fn mashari(&self) -> PathBuf {
        self.bayanat.join("mashari")
    }

    /// User-supplied fonts, validated on import.
    #[must_use]
    pub fn khutut(&self) -> PathBuf {
        self.bayanat.join("khutut")
    }

    /// Logs and diagnostic bundles.
    #[must_use]
    pub fn sijillat(&self) -> PathBuf {
        self.bayanat.join("sijillat")
    }

    /// Translation memory databases.
    #[must_use]
    pub fn dhakira(&self) -> PathBuf {
        self.bayanat.join("dhakira")
    }

    /// Registry cache: shard files, the global manifest, and artwork.
    #[must_use]
    pub fn makhbaa(&self) -> PathBuf {
        self.bayanat.join("makhbaa")
    }

    /// Sandbox verification workspace — owner only.
    #[must_use]
    pub fn sandooq(&self) -> PathBuf {
        self.bayanat.join("sandooq")
    }

    /// Key material. Never bundled into diagnostics, never committed, never
    /// copied by any export path in the product.
    #[must_use]
    pub fn mafatih(&self) -> PathBuf {
        self.bayanat.join("mafatih")
    }

    /// Plugin frameworks and adapter binaries the installer deploys, one
    /// subdirectory per component per version.
    #[must_use]
    pub fn mukawwinat(&self) -> PathBuf {
        self.bayanat.join("mukawwinat")
    }

    /// The install quarantine: where a downloaded package is isolated and
    /// verified before a byte of it is trusted.
    #[must_use]
    pub fn hajr(&self) -> PathBuf {
        self.bayanat.join("hajr")
    }

    /// The backup directory for one installation.
    ///
    /// # Errors
    ///
    /// Fails when the identity contains anything that cannot be a path segment.
    pub fn nusakh_luba(&self, huwiya: &str) -> Natija<PathBuf> {
        dakhil(&self.nusakh(), huwiya)
    }

    /// The content-addressed location of a patch file, sharded by the first
    /// byte of its hash so no directory ever holds tens of thousands of files.
    ///
    /// # Errors
    ///
    /// Fails when the hash is not lowercase hexadecimal of at least four
    /// characters, which would mean the caller built it from something other
    /// than a real content hash.
    pub fn ruqaa_bi_basma(&self, basma_hex: &str) -> Natija<PathBuf> {
        let salih = basma_hex.len() >= 4
            && basma_hex.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        if !salih {
            return Err(Khata::min_tafsir(&KhataMasarat::BasmaGhayrSaliha {
                basma: basma_hex.to_owned(),
            }));
        }
        let (badiya, baqi) = basma_hex.split_at(2);
        Ok(self.ruqaa().join(badiya).join(format!("{baqi}.ruqaa")))
    }

    /// Creates every directory in the layout.
    ///
    /// Called once at startup. Directories are created with default
    /// permissions; nothing here widens access.
    ///
    /// # Errors
    ///
    /// Fails on the first directory that cannot be created, naming it.
    pub fn takid(&self) -> Natija<()> {
        for masar in [
            self.bayanat.clone(),
            self.idadat.clone(),
            self.ruqaa(),
            self.nusakh(),
            self.mashari(),
            self.khutut(),
            self.sijillat(),
            self.dhakira(),
            self.makhbaa(),
            self.sandooq(),
            self.mafatih(),
        ] {
            insha_mujallad(&masar)?;
        }
        Ok(())
    }

    /// Proves that a directory inside Taarib's own layout may be deleted
    /// recursively, yielding the only value [`hadhf_mujallad`] accepts.
    ///
    /// The target must sit *strictly below* the data root or patch storage, and
    /// must not be — or contain — the data root, the settings root, the user's
    /// home directory, or patch storage. On Windows and macOS the settings root
    /// **is** the data root, so `%APPDATA%\Taarib` is refused twice over.
    ///
    /// # Errors
    ///
    /// [`KhataMasarat::JidhrMahmi`] when the target is one of those roots,
    /// contains one, lies outside the layout entirely, or reaches its place
    /// through a `..` component.
    pub fn hadaf_hadhf(&self, masar: &Path) -> Natija<HadafHadhf> {
        let ruqaa = self.ruqaa();
        ithbat_hadaf(
            &[&self.bayanat, &ruqaa],
            &[&self.bayanat, &self.idadat, &self.manzil, &ruqaa],
            masar,
        )
    }
}

/// A directory that has been proved safe to delete recursively.
///
/// [`hadhf_mujallad`] takes one of these and nothing else, so a recursive
/// deletion cannot be *written* without first passing through a constructor
/// that has already refused every root worth protecting. There are two:
/// [`Masarat::hadaf_hadhf`] for somewhere inside Taarib's own layout, and
/// [`hadaf_hadhf_fi_luba`] for somewhere inside a game installation.
///
/// Neither can yield a value equal to a data root, a settings root, a home
/// directory, patch storage, or a game directory — each requires the target to
/// sit strictly below the root that permits it, and separately refuses any path
/// that *is* such a root or an ancestor of one.
///
/// This is deliberately a property of the type rather than a check repeated at
/// each call site. A data root was destroyed by a deletion whose target was
/// correct at every call site that existed when it was written; the guard has to
/// be the thing a new call site cannot avoid, not the thing it must remember.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HadafHadhf(PathBuf);

impl HadafHadhf {
    /// The proved path.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.0
    }
}

/// Proves that a directory inside a game installation may be deleted
/// recursively.
///
/// The game directory itself is never a valid target: uninstalling Taarib's
/// work from a game removes the files Taarib added, one at a time, and no
/// operation in the product is entitled to remove somebody's game.
///
/// # Errors
///
/// [`KhataMasarat::JidhrMahmi`] when the target is the game directory itself,
/// lies outside it, or reaches its place through a `..` component.
pub fn hadaf_hadhf_fi_luba(jidhr_luba: &Path, masar: &Path) -> Natija<HadafHadhf> {
    ithbat_hadaf(&[jidhr_luba], &[jidhr_luba], masar)
}

/// The shared proof: strictly below one of `judhur`, and neither equal to nor an
/// ancestor of anything in `mahmiyat`.
///
/// The two conditions are separate on purpose. "Strictly below a permitted root"
/// alone would still allow deleting a game directory nested inside the data root
/// — and "not a protected root" alone would allow deleting anything anywhere.
fn ithbat_hadaf(judhur: &[&Path], mahmiyat: &[&Path], masar: &Path) -> Natija<HadafHadhf> {
    let rafd = |jidhr: &Path| -> Natija<HadafHadhf> {
        Err(Khata::min_tafsir(&KhataMasarat::JidhrMahmi {
            masar: masar.to_path_buf(),
            jidhr: jidhr.to_path_buf(),
        }))
    };

    // A `..` anywhere makes `starts_with` meaningless: `<root>/x/..` is the
    // root, and `starts_with` would happily confirm it is below itself.
    if masar.components().any(|juz| matches!(juz, Component::ParentDir)) {
        return rafd(masar);
    }

    for mahmi in mahmiyat {
        // Either the target *is* a protected root, or deleting it would take
        // one with it.
        if masar == *mahmi || mahmi.starts_with(masar) {
            return rafd(mahmi);
        }
    }

    if judhur.iter().any(|jidhr| masar != *jidhr && masar.starts_with(jidhr)) {
        Ok(HadafHadhf(masar.to_path_buf()))
    } else {
        rafd(judhur.first().copied().unwrap_or(masar))
    }
}

/// The directory the portable marker sits in, when this run carries one.
///
/// The marker is looked for beside the file the user actually carries: the
/// canonicalized executable — canonicalized so a launcher symlink does not
/// relocate the data root to wherever the symlink lives — or, inside an
/// `AppImage`, the outer image file named by `$APPIMAGE`.
#[expect(
    clippy::disallowed_methods,
    reason = "this module is the layout layer the rule points other callers towards, and \
              $APPIMAGE names the file the user carries rather than any setting"
)]
fn mujallad_mahmul() -> Option<PathBuf> {
    // In an AppImage, current_exe is the inner binary on the mount; the user carries $APPIMAGE.
    let hamil = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|malaf| malaf.is_file())
        // current_exe failing is not an error; the marker check is simply skipped.
        .or_else(|| std::env::current_exe().ok().and_then(|malaf| fs::canonicalize(malaf).ok()));
    let mujallad = hamil?.parent()?.to_path_buf();
    mujallad.join(ALAMAT_MAHMUL).is_file().then_some(mujallad)
}

/// A path's length in the units its platform limits.
#[cfg(windows)]
fn tul_masar(masar: &Path) -> usize {
    use std::os::windows::ffi::OsStrExt as _;
    masar.as_os_str().encode_wide().count()
}

/// A path's length in the units its platform limits.
#[cfg(not(windows))]
fn tul_masar(masar: &Path) -> usize {
    masar.as_os_str().len()
}

/// Joins a relative path onto a root, refusing anything that could escape it.
///
/// This is the only join permitted for untrusted input — archive entries, patch
/// manifests, launcher catalogues, anything read from a file. It refuses:
/// parent components, absolute paths and drive prefixes, embedded NUL, Windows
/// device names, names ending in a dot or space (Windows silently strips them,
/// so `evil.txt.` and `evil.txt` are the same file), and results longer than
/// the platform will reliably handle.
///
/// # Errors
///
/// Returns [`KhataMasarat::MasarKharij`], [`KhataMasarat::IsmMahjooz`] or
/// [`KhataMasarat::MasarTaweel`] describing exactly which rule was broken.
pub fn dakhil(jidhr: &Path, nisbi: &str) -> Natija<PathBuf> {
    if nisbi.is_empty() || nisbi.contains('\0') {
        return Err(Khata::min_tafsir(&KhataMasarat::MasarKharij {
            jidhr: jidhr.to_path_buf(),
            nisbi: nisbi.to_owned(),
        }));
    }

    // A manifest key written on Windows carries backslashes, and on Unix
    // `Path` sees the whole thing as ONE component — so `Data\Config.ini`
    // becomes a file with a backslash in its name instead of two components.
    // The same conversion is in `tathbeet::mawdi` and `aman::sandooq_fak`;
    // this is the third and last place that needed it.
    let mubaddal = nisbi.replace('\\', "/");
    let murashah = Path::new(&mubaddal);
    let mut mabni = jidhr.to_path_buf();

    for juz in murashah.components() {
        match juz {
            Component::Normal(ism) => {
                let Some(nass) = ism.to_str() else {
                    return Err(Khata::min_tafsir(&KhataMasarat::MasarKharij {
                        jidhr: jidhr.to_path_buf(),
                        nisbi: nisbi.to_owned(),
                    }));
                };
                // Windows rules, applied on Windows. `aux.rpy` is a legal and
                // common Ren'Py file name, and a Linux user told their real
                // file "does not refer to a real file" is being refused for
                // another operating system's reasons.
                if cfg!(windows) {
                    let jidhr_ism =
                        nass.split('.').next().unwrap_or(nass).to_ascii_uppercase();
                    if ASMAA_MAHJOOZA.contains(&jidhr_ism.as_str()) {
                        return Err(Khata::min_tafsir(&KhataMasarat::IsmMahjooz {
                            ism: nass.to_owned(),
                        }));
                    }
                    if nass.ends_with('.') || nass.ends_with(' ') {
                        return Err(Khata::min_tafsir(&KhataMasarat::IsmMahjooz {
                            ism: nass.to_owned(),
                        }));
                    }
                    // An alternate data stream is a second file behind the
                    // first, and nothing here ever means to write one.
                    if nass.contains(':') {
                        return Err(Khata::min_tafsir(&KhataMasarat::IsmMahjooz {
                            ism: nass.to_owned(),
                        }));
                    }
                }
                mabni.push(nass);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Khata::min_tafsir(&KhataMasarat::MasarKharij {
                    jidhr: jidhr.to_path_buf(),
                    nisbi: nisbi.to_owned(),
                }));
            }
        }
    }

    if mabni == jidhr {
        return Err(Khata::min_tafsir(&KhataMasarat::MasarKharij {
            jidhr: jidhr.to_path_buf(),
            nisbi: nisbi.to_owned(),
        }));
    }

    // Measured in the units the platform counts in. `OsStr::len` is bytes, and
    // Arabic is two bytes per UTF-16 unit — so a byte cap refuses an Arabic
    // path at half its real allowance, in a product whose users name folders
    // in Arabic.
    if tul_masar(&mabni) > HADD_TUL_MASAR {
        return Err(Khata::min_tafsir(&KhataMasarat::MasarTaweel {
            masar: mabni,
            hadd: HADD_TUL_MASAR,
        }));
    }

    Ok(mabni)
}

/// Confirms that an existing path really lives inside a root, after symbolic
/// links have been resolved.
///
/// [`dakhil`] is lexical and runs before anything exists; this runs after, and
/// catches the case where a directory inside the root is itself a link pointing
/// somewhere else — the escape a lexical check cannot see.
///
/// # Errors
///
/// Returns [`KhataMasarat::RabtRamzi`] when the resolved path leaves the root,
/// and [`KhataMasarat::TaadhurQira`] when either path cannot be resolved.
pub fn tahaqquq_ihtiwa(jidhr: &Path, masar: &Path) -> Natija<PathBuf> {
    let jidhr_haqiqi = fs::canonicalize(jidhr).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurQira { masar: jidhr.to_path_buf(), sabab: q })
    })?;
    let masar_haqiqi = fs::canonicalize(masar).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurQira { masar: masar.to_path_buf(), sabab: q })
    })?;
    if masar_haqiqi.starts_with(&jidhr_haqiqi) {
        Ok(masar_haqiqi)
    } else {
        Err(Khata::min_tafsir(&KhataMasarat::RabtRamzi {
            masar: masar.to_path_buf(),
            hadaf: masar_haqiqi,
            jidhr: jidhr_haqiqi,
        }))
    }
}

/// Creates a directory and every parent it needs.
///
/// # Errors
///
/// Fails when the directory cannot be created, naming the path and the reason.
pub fn insha_mujallad(masar: &Path) -> Natija<()> {
    fs::create_dir_all(masar).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurInsha { masar: masar.to_path_buf(), sabab: q })
    })
}

/// Writes a file so that it is either entirely the old content or entirely the
/// new one, never a truncated mixture.
///
/// The bytes go to a temporary file in the same directory, are flushed to the
/// device, and are then renamed over the target. An interruption at any point
/// leaves the original intact — which is the difference between a recoverable
/// crash and a destroyed `data.win`.
///
/// # Errors
///
/// Fails when the directory cannot be created, the temporary file cannot be
/// written or synced, or the rename cannot complete.
pub fn kitaba_dharra(masar: &Path, bayt: &[u8]) -> Natija<()> {
    let Some(mujallad) = masar.parent() else {
        return Err(Khata::min_tafsir(&KhataMasarat::MasarKharij {
            jidhr: PathBuf::new(),
            nisbi: masar.display().to_string(),
        }));
    };
    insha_mujallad(mujallad)?;

    let mut muaqqat = tempfile::NamedTempFile::new_in(mujallad).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurKitaba { masar: masar.to_path_buf(), sabab: q })
    })?;
    muaqqat.write_all(bayt).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurKitaba { masar: masar.to_path_buf(), sabab: q })
    })?;
    muaqqat.flush().map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurKitaba { masar: masar.to_path_buf(), sabab: q })
    })?;
    muaqqat.as_file().sync_all().map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurKitaba { masar: masar.to_path_buf(), sabab: q })
    })?;
    muaqqat.persist(masar).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurKitaba {
            masar: masar.to_path_buf(),
            sabab: q.error,
        })
    })?;

    // On POSIX the rename itself is atomic, but the new directory entry is not
    // durable until the directory is flushed. Skipping this is how a machine
    // that lost power comes back with an installation manifest that references
    // a backup the directory forgot about.
    #[cfg(unix)]
    if let Ok(maftuh) = fs::File::open(mujallad) {
        drop(maftuh.sync_all());
    }

    Ok(())
}

/// Atomically writes text, encoded UTF-8 without a byte order mark.
///
/// # Errors
///
/// As [`kitaba_dharra`].
pub fn kitaba_dharra_nass(masar: &Path, nass: &str) -> Natija<()> {
    kitaba_dharra(masar, nass.as_bytes())
}

/// Reads a whole file.
///
/// # Errors
///
/// Fails when the file is missing or unreadable, naming it and suggesting the
/// action that actually fixes that particular I/O failure.
pub fn qira(masar: &Path) -> Natija<Vec<u8>> {
    fs::read(masar).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TaadhurQira { masar: masar.to_path_buf(), sabab: q })
    })
}

/// Reads a whole file as UTF-8 text.
///
/// # Errors
///
/// As [`qira`], plus [`KhataMasarat::TarmizGhayrSalih`] when the bytes are not
/// valid UTF-8 — which for a launcher catalogue usually means the file is in a
/// legacy code page and needs its own reader, not a lossy conversion.
pub fn qira_nass(masar: &Path) -> Natija<String> {
    let bayt = qira(masar)?;
    String::from_utf8(bayt).map_err(|q| {
        Khata::min_tafsir(&KhataMasarat::TarmizGhayrSalih {
            masar: masar.to_path_buf(),
            mawqi: q.utf8_error().valid_up_to(),
        })
    })
}

/// Moves a file, falling back to copy-then-delete across filesystems — which is
/// the normal case when a game lives on one drive and Taarib's data on another.
///
/// # Errors
///
/// Fails when neither the rename nor the copy succeeds.
pub fn naql(min: &Path, ila: &Path) -> Natija<()> {
    if let Some(mujallad) = ila.parent() {
        insha_mujallad(mujallad)?;
    }
    if matches!(fs::rename(min, ila), Ok(())) { Ok(()) } else {
        let _ = fs::copy(min, ila).map_err(|q| {
            Khata::min_tafsir(&KhataMasarat::TaadhurNaql {
                min: min.to_path_buf(),
                ila: ila.to_path_buf(),
                sabab: q,
            })
        })?;
        fs::remove_file(min).map_err(|q| {
            Khata::min_tafsir(&KhataMasarat::TaadhurHadhf {
                masar: min.to_path_buf(),
                sabab: q,
            })
        })
    }
}

/// Deletes a file, treating "already gone" as success.
///
/// # Errors
///
/// Fails when the file exists and cannot be removed.
pub fn hadhf(masar: &Path) -> Natija<()> {
    match fs::remove_file(masar) {
        Ok(()) => Ok(()),
        Err(q) if q.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(q) => Err(Khata::min_tafsir(&KhataMasarat::TaadhurHadhf {
            masar: masar.to_path_buf(),
            sabab: q,
        })),
    }
}

/// Deletes a directory and everything under it, treating "already gone" as
/// success.
///
/// Takes a [`HadafHadhf`] rather than a path, which is the whole point: the
/// only recursive deletion in the product cannot be aimed at a data root, a
/// settings root, a home directory, patch storage or a game directory, because
/// no constructor of its argument will produce one.
///
/// # Errors
///
/// Fails when the directory exists and cannot be removed.
#[expect(
    clippy::disallowed_methods,
    reason = "the one recursive deletion in the product, and the only one whose argument has \
              already been proved not to be a root"
)]
pub fn hadhf_mujallad(hadaf: &HadafHadhf) -> Natija<()> {
    match fs::remove_dir_all(hadaf.masar()) {
        Ok(()) => Ok(()),
        Err(q) if q.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(q) => Err(Khata::min_tafsir(&KhataMasarat::TaadhurHadhf {
            masar: hadaf.masar().to_path_buf(),
            sabab: q,
        })),
    }
}

/// Failures of the on-disk layout.
#[derive(Debug, thiserror::Error)]
pub enum KhataMasarat {
    /// The platform reports no home directory.
    #[error("no home directory")]
    LaMujalladManzili,

    /// A directory could not be created.
    #[error("cannot create {masar}")]
    TaadhurInsha {
        /// The directory.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file could not be written.
    #[error("cannot write {masar}")]
    TaadhurKitaba {
        /// The file.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file could not be read.
    #[error("cannot read {masar}")]
    TaadhurQira {
        /// The file.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file could not be moved.
    #[error("cannot move {min} to {ila}")]
    TaadhurNaql {
        /// Source.
        min: PathBuf,
        /// Destination.
        ila: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file or directory could not be deleted.
    #[error("cannot delete {masar}")]
    TaadhurHadhf {
        /// The path.
        masar: PathBuf,
        /// The underlying I/O failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A relative path tried to leave its root.
    #[error("path escapes its root: {nisbi}")]
    MasarKharij {
        /// The root it was supposed to stay inside.
        jidhr: PathBuf,
        /// The offending relative path.
        nisbi: String,
    },

    /// A path component resolves to a device rather than a file.
    #[error("reserved device name: {ism}")]
    IsmMahjooz {
        /// The offending component.
        ism: String,
    },

    /// The constructed path is longer than the platform handles reliably.
    #[error("path too long: {masar}")]
    MasarTaweel {
        /// The path.
        masar: PathBuf,
        /// The limit it exceeded.
        hadd: usize,
    },

    /// A path inside the root resolves, through links, to somewhere outside it.
    #[error("symlink escapes root: {masar}")]
    RabtRamzi {
        /// The path as given.
        masar: PathBuf,
        /// Where it actually resolves to.
        hadaf: PathBuf,
        /// The root it left.
        jidhr: PathBuf,
    },

    /// A patch hash is not a hash.
    #[error("not a content hash: {basma}")]
    BasmaGhayrSaliha {
        /// The offending value.
        basma: String,
    },

    /// A recursive deletion was aimed at a directory that must never be one.
    #[error("refused to delete {masar}: protected root {jidhr}")]
    JidhrMahmi {
        /// The path the deletion was aimed at.
        masar: PathBuf,
        /// The protected root it is, contains, or fails to sit below.
        jidhr: PathBuf,
    },

    /// A file that must be UTF-8 is not.
    #[error("invalid UTF-8 in {masar} at byte {mawqi}")]
    TarmizGhayrSalih {
        /// The file.
        masar: PathBuf,
        /// The byte offset where decoding failed.
        mawqi: usize,
    },
}

impl Tafsir for KhataMasarat {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MASARAT
                + match self {
                    Self::LaMujalladManzili => 0,
                    Self::TaadhurInsha { .. } => 1,
                    Self::TaadhurKitaba { .. } => 2,
                    Self::TaadhurQira { .. } => 3,
                    Self::MasarKharij { .. } => 4,
                    Self::IsmMahjooz { .. } => 5,
                    Self::MasarTaweel { .. } => 6,
                    Self::TaadhurNaql { .. } => 7,
                    Self::TaadhurHadhf { .. } => 8,
                    Self::RabtRamzi { .. } => 9,
                    Self::BasmaGhayrSaliha { .. } => 10,
                    Self::TarmizGhayrSalih { .. } => 11,
                    Self::JidhrMahmi { .. } => 12,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::MasarKharij { .. }
            | Self::IsmMahjooz { .. }
            | Self::RabtRamzi { .. }
            | Self::JidhrMahmi { .. } => Khutura::Fadih,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::LaMujalladManzili => {
                "تعذّر تحديد مجلد المستخدم على هذا الجهاز، ولا يستطيع تعريب حفظ بياناته.".to_owned()
            }
            Self::TaadhurInsha { masar, .. } => {
                format!("تعذّر إنشاء المجلد: {}", masar.display())
            }
            Self::TaadhurKitaba { masar, .. } => {
                format!("تعذّرت الكتابة إلى الملف: {}", masar.display())
            }
            Self::TaadhurQira { masar, .. } => {
                format!("تعذّرت قراءة الملف: {}", masar.display())
            }
            Self::TaadhurNaql { min, .. } => {
                format!("تعذّر نقل الملف: {}", min.display())
            }
            Self::TaadhurHadhf { masar, .. } => {
                format!("تعذّر حذف: {}", masar.display())
            }
            Self::MasarKharij { nisbi, .. } => format!(
                "رُفض مسار يحاول الخروج من مجلده المسموح: {nisbi}. \
                 هذا يدلّ على ملف تالف أو محتوى غير موثوق."
            ),
            Self::IsmMahjooz { ism } => format!(
                "رُفض اسم محجوز في نظام التشغيل: {ism}. هذا الاسم لا يشير إلى ملف حقيقي."
            ),
            Self::MasarTaweel { hadd, .. } => format!(
                "المسار أطول مما يدعمه النظام ({hadd} حرفًا). انقل اللعبة أو مجلد تعريب إلى \
                 مسار أقصر."
            ),
            Self::RabtRamzi { masar, .. } => format!(
                "المسار {} يشير خارج المجلد المسموح عبر ارتباط رمزي، ولن يُستخدم.",
                masar.display()
            ),
            Self::BasmaGhayrSaliha { .. } => {
                "بصمة الرقعة غير صالحة، ولا يمكن تحديد موضع ملفها.".to_owned()
            }
            Self::TarmizGhayrSalih { masar, .. } => format!(
                "الملف {} ليس بترميز UTF-8، ويحتاج قارئًا خاصًا بترميزه.",
                masar.display()
            ),
            Self::JidhrMahmi { masar, jidhr } => format!(
                "رُفض حذف {} لأنه مجلد محمي أو يحتوي على {}. \
                 لا يحذف تعريب مجلد بياناته ولا مجلد لعبة.",
                masar.display(),
                jidhr.display()
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::LaMujalladManzili => {
                "No home directory on this machine, so Taarib has nowhere to keep its data."
                    .to_owned()
            }
            Self::TaadhurInsha { masar, .. } => {
                format!("Cannot create folder: {}", masar.display())
            }
            Self::TaadhurKitaba { masar, .. } => format!("Cannot write file: {}", masar.display()),
            Self::TaadhurQira { masar, .. } => format!("Cannot read file: {}", masar.display()),
            Self::TaadhurNaql { min, .. } => format!("Cannot move file: {}", min.display()),
            Self::TaadhurHadhf { masar, .. } => format!("Cannot delete: {}", masar.display()),
            Self::MasarKharij { nisbi, .. } => format!(
                "Refused a path that escapes its allowed folder: {nisbi}. This indicates a \
                 corrupt file or untrusted content."
            ),
            Self::IsmMahjooz { ism } => {
                format!("Refused a reserved device name: {ism}. It does not name a real file.")
            }
            Self::MasarTaweel { hadd, .. } => format!(
                "Path is longer than the system supports ({hadd} characters). Move the game or \
                 Taarib's folder somewhere shorter."
            ),
            Self::RabtRamzi { masar, .. } => format!(
                "{} resolves outside its allowed folder through a symbolic link and will not be \
                 used.",
                masar.display()
            ),
            Self::BasmaGhayrSaliha { .. } => {
                "Invalid patch content hash, so its file location cannot be derived.".to_owned()
            }
            Self::TarmizGhayrSalih { masar, .. } => format!(
                "{} is not UTF-8 and needs a reader for its own encoding.",
                masar.display()
            ),
            Self::JidhrMahmi { masar, jidhr } => format!(
                "Refused to delete {} because it is, or contains, the protected folder {}. \
                 Taarib never deletes its own data folder or a game folder.",
                masar.display(),
                jidhr.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::LaMujalladManzili
            | Self::BasmaGhayrSaliha { .. }
            | Self::TarmizGhayrSalih { .. } => Khutwa::FathTashkhis,
            Self::TaadhurInsha { sabab, .. }
            | Self::TaadhurKitaba { sabab, .. }
            | Self::TaadhurNaql { sabab, .. }
            | Self::TaadhurHadhf { sabab, .. } => {
                khutwa_io(sabab, MasarMatlub::MujalladRuqaa)
            }
            Self::TaadhurQira { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::MasarKharij { .. }
            | Self::IsmMahjooz { .. }
            | Self::RabtRamzi { .. }
            | Self::JidhrMahmi { .. } => Khutwa::IblaghLilMalik,
            Self::MasarTaweel { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladRuqaa,
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::LaMujalladManzili => {}
            Self::TaadhurInsha { masar, sabab }
            | Self::TaadhurKitaba { masar, sabab }
            | Self::TaadhurQira { masar, sabab }
            | Self::TaadhurHadhf { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_io(sabab));
            }
            Self::TaadhurNaql { min, ila, sabab } => {
                let _ = siyaq.insert("min".to_owned(), QeemaSiyaq::Masar(min.clone()));
                let _ = siyaq.insert("ila".to_owned(), QeemaSiyaq::Masar(ila.clone()));
                siyaq.extend(siyaq_io(sabab));
            }
            Self::MasarKharij { jidhr, nisbi } => {
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
                let _ = siyaq.insert("nisbi".to_owned(), QeemaSiyaq::Nass(nisbi.clone()));
            }
            Self::IsmMahjooz { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::MasarTaweel { masar, hadd } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("hadd".to_owned(), QeemaSiyaq::Hajm(*hadd as u64));
            }
            Self::RabtRamzi { masar, hadaf, jidhr } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("hadaf".to_owned(), QeemaSiyaq::Masar(hadaf.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            }
            Self::BasmaGhayrSaliha { basma } => {
                let _ = siyaq.insert("basma".to_owned(), QeemaSiyaq::Nass(basma.clone()));
            }
            Self::TarmizGhayrSalih { masar, mawqi } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("mawqi".to_owned(), QeemaSiyaq::Hajm(*mawqi as u64));
            }
            Self::JidhrMahmi { masar, jidhr } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataMasarat);

#[cfg(test)]
mod ikhtibarat {
    use std::path::{Path, PathBuf};

    use super::Masarat;

    fn layout() -> Masarat {
        Masarat::min_judhur(PathBuf::from("/tmp/taarib-test/bayanat"), "/tmp/taarib-test/idadat")
    }

    #[test]
    fn jidhr_ruqaa_yuhawwil_takhzin_alruqaa_wahdah() {
        let asas = layout();
        assert_eq!(asas.ruqaa(), Path::new("/tmp/taarib-test/bayanat/ruqaa"));

        let manqul = layout().maa_jidhr_ruqaa(Some(Path::new("/mnt/store/taarib-ruqaa")));
        assert_eq!(manqul.ruqaa(), Path::new("/mnt/store/taarib-ruqaa"));
        // Only patch storage moves; the setting is not a second data root.
        assert_eq!(manqul.nusakh(), asas.nusakh());
        assert_eq!(manqul.makhbaa(), asas.makhbaa());
        assert_eq!(manqul.qaida_bayanat(), asas.qaida_bayanat());
    }

    #[test]
    fn al_masar_almuaanwan_bilbasma_yatbaa_aljidhr_almanqul() {
        // The content-addressed derivation is the one every install and every
        // download goes through, so a re-rooting it did not follow would be a
        // setting that moves an empty directory and nothing else.
        let manqul = layout().maa_jidhr_ruqaa(Some(Path::new("/mnt/store/taarib-ruqaa")));
        assert_eq!(
            manqul.ruqaa_bi_basma("ab12cd34").as_deref().ok(),
            Some(Path::new("/mnt/store/taarib-ruqaa/ab/12cd34.ruqaa"))
        );
    }

    #[test]
    fn jidhr_ruqaa_farigh_yaud_lil_jidhr_alasli() {
        // The Studio stores "no override" as `null`, but an empty string is one
        // keystroke away in a text field and must not resolve to the filesystem
        // root's own `ruqaa`.
        let asas = layout();
        for la_shay in [None, Some(Path::new(""))] {
            assert_eq!(layout().maa_jidhr_ruqaa(la_shay).ruqaa(), asas.ruqaa());
        }
    }

    // The rest of this module is the guard that replaces a destroyed data root.
    // `hadhf_mujallad` takes a `HadafHadhf` and nothing else, so these tests are
    // exhaustive over what a recursive deletion can ever be aimed at.

    #[test]
    fn jidhr_albayanat_la_yumkin_hallahu_hadafan_lilhadhf() {
        let asas = layout();
        // The data root itself, in every spelling that reaches the same place.
        for muhawala in [
            asas.jidhr_bayanat().to_path_buf(),
            asas.jidhr_bayanat().join("."),
            asas.jidhr_bayanat().join("nusakh").join(".."),
        ] {
            assert!(
                asas.hadaf_hadhf(&muhawala).is_err(),
                "the data root was accepted as a deletion target as {}",
                muhawala.display()
            );
        }
    }

    #[test]
    fn judhur_altakhtit_alukhra_mahmiya_kadhalik() {
        let asas = layout();
        // Settings root, home directory, patch storage, and every ancestor of
        // the data root — deleting any of those takes the data root with it.
        for mahmi in [
            asas.jidhr_idadat().to_path_buf(),
            asas.manzil().to_path_buf(),
            asas.ruqaa(),
            PathBuf::from("/tmp/taarib-test"),
            PathBuf::from("/tmp"),
            PathBuf::from("/"),
        ] {
            assert!(
                asas.hadaf_hadhf(&mahmi).is_err(),
                "{} was accepted as a deletion target",
                mahmi.display()
            );
        }
    }

    #[test]
    fn ma_dakhil_aljidhr_maqbul() {
        // The guard must not refuse the sweep it exists to permit: quarantine is
        // cleared recursively on every launch and is one level below the root.
        let asas = layout();
        for masmuh in [asas.hajr(), asas.nusakh().join("luba-ma"), asas.sandooq().join("musawwada")]
        {
            assert_eq!(
                asas.hadaf_hadhf(&masmuh).map(|hadaf| hadaf.masar().to_path_buf()).ok().as_deref(),
                Some(masmuh.as_path()),
                "{} should be deletable",
                masmuh.display()
            );
        }
    }

    #[test]
    fn jidhr_alluba_la_yumkin_hallahu_hadafan_lilhadhf() {
        // The worse version of the same defect: a cleanup that resolved to a
        // game directory would delete somebody's game.
        let luba = Path::new("/mnt/f/SteamLibrary/steamapps/common/Luba");
        for muhawala in [
            luba.to_path_buf(),
            luba.join("."),
            luba.join("Data").join(".."),
            PathBuf::from("/mnt/f/SteamLibrary/steamapps/common"),
            PathBuf::from("/mnt/f/SteamLibrary"),
            // Outside the game entirely — a sibling game is not this one's to delete.
            PathBuf::from("/mnt/f/SteamLibrary/steamapps/common/Ukhra"),
        ] {
            assert!(
                super::hadaf_hadhf_fi_luba(luba, &muhawala).is_err(),
                "{} was accepted as a deletion target inside a game",
                muhawala.display()
            );
        }

        // What Taarib actually created inside the game still is deletable.
        let mudaf = luba.join("BepInEx").join("plugins").join("Taarib");
        assert_eq!(
            super::hadaf_hadhf_fi_luba(luba, &mudaf).map(|h| h.masar().to_path_buf()).ok(),
            Some(mudaf)
        );
    }

    #[test]
    fn aljidhr_almanqul_lilruqaa_mahmi_fi_makanihi_aljadeed() {
        // Patch storage keeps its protection after it moves: the setting names
        // somewhere the user put their library, not somewhere to be cleared.
        let manqul = layout().maa_jidhr_ruqaa(Some(Path::new("/mnt/store/taarib-ruqaa")));
        assert!(manqul.hadaf_hadhf(Path::new("/mnt/store/taarib-ruqaa")).is_err());
        // A shard below it is still an ordinary target.
        assert!(manqul.hadaf_hadhf(Path::new("/mnt/store/taarib-ruqaa/ab")).is_ok());
        // And the old in-root location is now merely inside the data root.
        assert!(manqul.hadaf_hadhf(&manqul.jidhr_bayanat().join("ruqaa")).is_ok());
    }
}
