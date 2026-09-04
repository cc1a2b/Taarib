//! متجر أمازون — Amazon Games, whose whole catalogue is a `SQLite` database the
//! client keeps open while it runs.
//!
//! ```text
//! %LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameInstallInfo.sqlite
//! %LOCALAPPDATA%\Amazon Games\Data\Games\Sql\GameProductInfo.sqlite
//! <install root>\fuel.json
//! ```
//!
//! Windows only. Amazon ships no macOS or Linux client, and the Prime Gaming
//! titles a Linux user has are installed by Heroic's `nile` backend, which is a
//! different catalogue in a different place and is deliberately not this
//! adapter's problem.
//!
//! # Opening a database the launcher is holding
//!
//! `GameInstallInfo.sqlite` belongs to the Amazon Games client, which may be
//! running and which will keep running long after this scan ends. `rusqlite`'s
//! default flags are `READ_WRITE | CREATE`, so the flags below are spelled out
//! in full rather than derived from `OpenFlags::default()`: a read-write
//! connection to somebody else's live database may roll back a hot journal,
//! checkpoint and truncate a write-ahead log out from under its owner, or
//! migrate the file format. None of those are things a *scan* is allowed to do.
//!
//! Two read-only modes are tried, in this order:
//!
//! 1. **`?mode=ro`** — proper read-only. `SQLite` takes shared read locks and
//!    reads the write-ahead log, so the snapshot is consistent and a game
//!    installed thirty seconds ago is visible.
//! 2. **`?immutable=1`** — no locks at all, no `-shm`, no `-wal`, no write to
//!    the containing directory. Used when the first mode cannot even read the
//!    schema, which is what happens when the client holds an exclusive lock or
//!    the directory is not writable enough for `SQLite` to create the shared
//!    memory file a WAL database needs. The cost is stated rather than hidden:
//!    rows that live only in the write-ahead log are invisible, so a very
//!    recent install can be missing until the next scan. That is reported as a
//!    [`TanbihFahs`], not swallowed.
//!
//! Neither mode can write. `SQLITE_OPEN_CREATE` is never passed, so a missing
//! file is an error rather than a brand new empty database appearing inside
//! another product's storage directory. A database that is merely *busy* is a
//! warning and an empty game list, never a hard failure — the user's other nine
//! launchers still have to arrive.
//!
//! # The schema is not ours
//!
//! Amazon publishes no schema and guarantees nothing. Everything below is
//! inferred from databases in the wild, so no column is named in a query until
//! the table has been asked what columns it has: `DbSet` is opened with
//! `SELECT * … LIMIT 0`, its column names are read back, and each role this
//! adapter needs is matched case-insensitively against a list of candidate
//! names. A column that is not there costs the field that came from it and
//! nothing else. A client update that renames `SizeOnDisk` costs the size, not
//! the library.
//!
//! Exactly one column is load-bearing: `Id`, the product ASIN, which is the
//! identity [`MasdarLuba::Amazon`] carries and the key the registry shards on.
//! A row without it cannot be published against and is reported as a warning
//! rather than given an invented identifier.
//!
//! # `Installed = 0` is not a game
//!
//! `DbSet` holds the user's whole Prime Gaming entitlement, not their installs:
//! every title they have ever claimed has a row, and `Installed` is what
//! separates the two. Emitting the zero rows would fill the library with
//! hundreds of cards for games that are not on the machine, each of them
//! unpatchable, each of them indistinguishable from a game whose directory was
//! deleted. They are skipped here and produce no warning, because "you own a
//! game you have not installed" is not a fault to report.
//!
//! # `fuel.json`
//!
//! The client writes `fuel.json` into the root of every finished install; it is
//! how the client itself knows what to run. `Main.Command` is the executable,
//! relative to the install root, and it is the only source for it — `DbSet`
//! records a directory and never a binary.
//!
//! Its absence is therefore information. A row the database calls installed,
//! whose directory exists, and which has no `fuel.json` is an install the
//! client has not finished writing. Such a game is still discovered and still
//! shown, and [`LubaMuktashafa::muktamila`] is false for it, so nothing offers
//! to patch files the client is about to replace.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Row};
use serde_json::Value;
use taarib_mustalahat::ghiyab::SababGhiyab;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::{dakhil, qira_nass};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "amazon";

/// Platforms this adapter can find anything on. Amazon has no macOS or Linux
/// client at all, so there is nothing to look for anywhere else.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The client's directory under the user's local application data.
const MUJALLAD_MATJAR: &str = "Amazon Games";

/// Where the install catalogue lives under the client's root.
const MASAR_QAIDA: [&str; 4] = ["Data", "Games", "Sql", "GameInstallInfo.sqlite"];

/// The product catalogue, beside the install catalogue, carrying titles and
/// artwork addresses.
const ISM_QAIDA_MUNTAJ: &str = "GameProductInfo.sqlite";

/// The one table both databases use. Amazon's client is written against an ORM
/// that names every table this, in both files.
const JADWAL: &str = "DbSet";

/// The file the client writes into a finished install, naming what to run.
const ISM_WAQUD: &str = "fuel.json";

/// How long a query waits for a lock before giving up.
///
/// Half a second. The client holds its write lock for milliseconds at a time,
/// so this absorbs an install landing mid-scan; waiting longer would mean a
/// library screen that stalls because somebody happened to be downloading.
const MUHLAT_INTIZAR: Duration = Duration::from_millis(500);

/// Column names that may carry the product ASIN, best first.
const AAMIDA_MUARRIF: [&str; 4] = ["Id", "ProductAsin", "ProductIdStr", "Asin"];

/// Column names that may carry the installation directory.
const AAMIDA_MASAR: [&str; 3] = ["InstallDirectory", "InstallPath", "InstallLocation"];

/// Column names that may carry the title.
const AAMIDA_UNWAN: [&str; 3] = ["ProductTitle", "Title", "ProductName"];

/// Column names that may carry the installed flag.
const AAMIDA_MANSUB: [&str; 2] = ["Installed", "IsInstalled"];

/// Column names that may carry the installed version.
const AAMIDA_ISDAR: [&str; 3] = ["InstallVersion", "Version", "ProductVersion"];

/// Column names that may carry the size on disk.
const AAMIDA_HAJM: [&str; 3] = ["SizeOnDisk", "InstallSize", "Size"];

/// Column names that may carry an installation or update timestamp.
const AAMIDA_TARIKH: [&str; 4] =
    ["InstallDate", "InstallDateUtc", "LastUpdateUtc", "UpdatedAt"];

/// Column names that may carry a last-played timestamp.
const AAMIDA_LAAB: [&str; 3] = ["LastPlayDate", "LastPlayedUtc", "LastPlayed"];

/// Column names that may carry a transfer state.
///
/// Not every client version has one, which is why it is probed rather than
/// selected. See [`hala_min_qeema`] for the values that are recognised and for
/// why an unrecognised value is dropped instead of being guessed at.
const AAMIDA_HALA: [&str; 4] = ["InstallState", "DownloadState", "State", "Status"];

/// Column names that may carry download progress, as a percentage or as a
/// fraction of bytes.
const AAMIDA_TAQADDUM: [&str; 3] = ["PercentComplete", "DownloadProgress", "Progress"];

/// Column names in the product catalogue that may carry an icon address.
const AAMIDA_AYQUNA: [&str; 3] = ["ProductIconUrl", "IconUrl", "ProductIcon"];

/// Column names in the product catalogue that may carry a logo address.
const AAMIDA_SHIAR: [&str; 3] = ["ProductLogoUrl", "LogoUrl", "ProductLogo"];

/// Column names in the product catalogue that may carry a wide background.
const AAMIDA_KHALFIYA: [&str; 4] =
    ["BackgroundUrl1", "BackgroundUrl2", "ProductBackgroundUrl", "HeroUrl"];

/// How many rows either catalogue is read for.
///
/// A Prime Gaming entitlement runs to a few hundred titles. Eight thousand is
/// far past any real account and bounds the work a corrupt or hostile database
/// can ask this process to do.
const HADD_SUFUF: usize = 8192;

/// Image file extensions the artwork pipeline can decode.
const IMTIDADAT_SURA: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// How many files are examined inside the client's image cache.
const HADD_MALAFFAT_SUWAR: usize = 512;

/// Amazon Games.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarAmazon;

impl MatjarAmazon {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarAmazon {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "أمازون"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Amazon Games"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        let jidhr = jidhr_tilqai(siyaq);
        // An existence check, not a parse: no database is opened here, because
        // this runs before every scan to decide whether to scan at all.
        (jidhr.is_dir() || malaf_qaida(&jidhr).is_some()).then_some(jidhr)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurQiraatFahras`] only when the install
    /// catalogue exists and neither read-only mode can read its schema, which
    /// is the single failure that makes the whole Amazon catalogue unreadable.
    /// A database that is present and merely locked by the running client is a
    /// [`TanbihFahs`] and an empty game list, because a launcher that is busy
    /// must not cost the user the nine other launchers in the same scan.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let jidhr = jidhr_tilqai(siyaq);
        let Some(masar_qaida) = malaf_qaida(&jidhr) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: jidhr.is_dir().then(|| jidhr.clone()),
            ..NatijatMatjar::default()
        };

        let (ittisal, thabita) = match iftah_qaida(&masar_qaida) {
            Ok(maftuh) => maftuh,
            Err(tafsil) => {
                return Err(KhataKashf::TaadhurQiraatFahras {
                    matjar: MUARRIF,
                    masar: masar_qaida,
                    sabab: std::io::Error::other(tafsil),
                }
                .into());
            },
        };
        if thabita {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar_qaida.display().to_string(),
                "the Amazon Games client is holding its catalogue, so it was read in immutable \
                 mode; a game installed in the last few seconds may be missing until the next \
                 scan",
            ));
        }

        let sufuf = sufuf_tathbeet(&ittisal, &mut natija.tanbihat);
        let muntajat =
            malaf_qaida_muntaj(&jidhr).map(|masar| bayanat_muntajat(&masar)).unwrap_or_default();

        // Read once, here, rather than by a caller remembering to call
        // `halat_matjar` after every scan. A state that has to be fetched
        // separately is a state that is eventually not fetched, and the
        // existence gate would then report "the folder is nearly empty" about a
        // download the client is actively running.
        let halat = halat_matjar(siyaq);

        let mut mawaqi: BTreeSet<String> = BTreeSet::new();
        for saff in sufuf {
            // Owned but never installed. Not a fault, not a warning, not a game.
            if !saff.mansub {
                continue;
            }
            let hala = halat.get(&saff.muarrif).cloned();
            match luba_min_saff(&saff, &jidhr, muntajat.get(&saff.muarrif), hala) {
                Ok(luba) => {
                    // Two rows pointing at one directory is what a reinstall
                    // into a renamed ASIN leaves behind. Keeping both would put
                    // two cards in the library over one set of files.
                    if mawaqi.insert(muwahhad(&luba.jidhr, siyaq.nizam)) {
                        natija.alaab.push(luba);
                    }
                },
                Err(tanbih) => natija.tanbihat.push(tanbih),
            }
        }

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        // The `Sql` directory rather than the file: SQLite writes its journal
        // and its `-wal` beside the database, and a write to either is the
        // signal that something was installed or removed.
        malaf_qaida(&jidhr_tilqai(siyaq))
            .and_then(|masar| masar.parent().map(Path::to_path_buf))
            .filter(|masar| masar.is_dir())
            .into_iter()
            .collect()
    }
}

/// What the Amazon client's own catalogue says it is doing to each game, keyed
/// by ASIN.
///
/// Fills [`TalabWujud::hala_matjar`](crate::wujud::TalabWujud::hala_matjar) so
/// that the existence gate short-circuits: a game the client says is
/// downloading is downloading, and a size check against a partial download
/// would produce a second, worse description of the same fact.
///
/// [`Matjar::ifhas`] calls this itself and puts the answer on each game's
/// [`LubaMuktashafa::hala_matjar`], so nothing outside this module has to
/// remember to. It stays public because the interface re-reads it while a
/// download is running — a card that says "downloading (4%)" has to become
/// "downloading (37%)" without a rescan, and a rescan is the one thing that
/// must not happen sixty times a minute.
///
/// The map is empty — legitimately, and on most machines — when the client
/// version installed has no transfer-state column at all. Amazon publishes no
/// schema, several client versions carry no such column, and inventing a state
/// from the `Installed` flag alone would report every finished install as
/// something it is not. An empty map means the gate does its own filesystem
/// checks, which is the correct outcome and not a degraded one.
#[must_use]
pub fn halat_matjar(siyaq: &SiyaqFahs) -> BTreeMap<String, SababGhiyab> {
    let mut halat = BTreeMap::new();
    if !MANASSAT.contains(&siyaq.nizam) {
        return halat;
    }
    let Some(masar_qaida) = malaf_qaida(&jidhr_tilqai(siyaq)) else {
        return halat;
    };
    let Ok((ittisal, _)) = iftah_qaida(&masar_qaida) else {
        return halat;
    };
    // This read has nowhere to attach a warning — the caller wanted states, not
    // a scan result — so anything that degraded goes to the log rather than
    // being dropped. A state map that is empty because the table could not be
    // read is worth being able to find afterwards.
    let mut tanbihat = Vec::new();
    let sufuf = sufuf_tathbeet(&ittisal, &mut tanbihat);
    for tanbih in &tanbihat {
        tracing::debug!(
            matjar = MUARRIF,
            mawdi = %tanbih.mawdi,
            sabab = %tanbih.sabab,
            "the Amazon state read degraded"
        );
    }
    for saff in sufuf {
        if !saff.mansub {
            continue;
        }
        if let Some(hala) = saff.hala {
            let _ = halat.insert(saff.muarrif, hala);
        }
    }
    halat
}

/// The client's data root, under the user's local application data.
fn jidhr_tilqai(siyaq: &SiyaqFahs) -> PathBuf {
    // The user's override wins outright. Folded in here rather than checked at
    // each of the four call sites, because a resolver that four callers have to
    // remember to wrap is a resolver three of them eventually will not.
    siyaq
        .manassat
        .amazon
        .clone()
        .unwrap_or_else(|| bayanat_mahalliya(&siyaq.manzil).join(MUJALLAD_MATJAR))
}

/// Windows' per-user local application data directory.
///
/// The environment is consulted first because a roaming or domain profile can
/// move it; the layout under the home directory is the fallback for the
/// ordinary case where the variable is not set, which is how this resolves at
/// all on the non-Windows builds that still compile this file.
fn bayanat_mahalliya(manzil: &Path) -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .filter(|qeema| !qeema.is_empty())
        .map_or_else(|| manzil.join("AppData").join("Local"), PathBuf::from)
}

/// Locates the install catalogue under the client root.
fn malaf_qaida(jidhr: &Path) -> Option<PathBuf> {
    let masar = MASAR_QAIDA.iter().fold(jidhr.to_path_buf(), |mabni, juz| mabni.join(juz));
    masar.is_file().then_some(masar)
}

/// Locates the product catalogue, which sits beside the install catalogue.
fn malaf_qaida_muntaj(jidhr: &Path) -> Option<PathBuf> {
    let masar = malaf_qaida(jidhr)?.with_file_name(ISM_QAIDA_MUNTAJ);
    masar.is_file().then_some(masar)
}

// ---------------------------------------------------------------------------
// Opening the catalogue without touching it
// ---------------------------------------------------------------------------

/// Opens a catalogue read-only.
///
/// The boolean says whether the immutable fallback had to be used, which the
/// caller turns into a warning: an immutable read bypasses the write-ahead log
/// and can therefore be slightly stale.
///
/// # Errors
///
/// Returns a sentence describing both attempts when neither can enumerate the
/// schema, which is what the caller turns into a catalogue-level failure.
fn iftah_qaida(masar: &Path) -> Result<(Connection, bool), String> {
    let hudud = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Some(rabt) = rabt_qaida(masar) else {
        return Err("the catalogue path cannot be expressed as a SQLite URI".to_owned());
    };
    match jarrib_fath(&format!("{rabt}?mode=ro"), hudud) {
        Ok(ittisal) => Ok((ittisal, false)),
        Err(qari) => match jarrib_fath(&format!("{rabt}?immutable=1"), hudud) {
            Ok(ittisal) => Ok((ittisal, true)),
            Err(thabit) => Err(format!(
                "read-only open failed ({qari}), and immutable open failed ({thabit})"
            )),
        },
    }
}

/// Opens one connection and proves it can read the schema before returning it.
///
/// `SQLite` opens lazily: `open_with_flags` succeeds against a path it has not
/// touched, and the real failure — the shared-memory file a WAL database needs
/// and cannot create on a directory this process may not write — only surfaces
/// on the first statement. Reading the schema here is what makes the fallback
/// in [`iftah_qaida`] fire at the right moment rather than one query later.
fn jarrib_fath(rabt: &str, hudud: OpenFlags) -> Result<Connection, String> {
    let ittisal = Connection::open_with_flags(rabt, hudud).map_err(|khata| khata.to_string())?;
    let _ = ittisal.busy_timeout(MUHLAT_INTIZAR);
    let adad: Result<i64, rusqlite::Error> =
        ittisal.query_row("SELECT count(*) FROM sqlite_master", [], |saff| saff.get(0));
    adad.map_err(|khata| khata.to_string())?;
    Ok(ittisal)
}

/// Builds the `file:` URI `SQLite` needs in order to accept query parameters.
///
/// Percent-encoding is done here rather than left to `SQLite` because a
/// database
/// under a path containing a space, a `#`, or a `?` would otherwise be opened
/// as a different file, or as a file carrying a query string nobody wrote.
fn rabt_qaida(masar: &Path) -> Option<String> {
    use std::fmt::Write as _;

    let nass = masar.to_str()?;
    let mut mabni = String::with_capacity(nass.len().saturating_add(16));
    for harf in nass.chars() {
        match harf {
            '\\' | '/' => mabni.push('/'),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' | ':' => mabni.push(harf),
            _ => {
                let mut buffer = [0u8; 4];
                for bayt in harf.encode_utf8(&mut buffer).as_bytes() {
                    let _ = write!(mabni, "%{bayt:02X}");
                }
            },
        }
    }
    Some(format!("file:///{}", mabni.trim_start_matches('/')))
}

/// The column names of a table, or `None` when the table is not there.
fn asmaa_aamida(ittisal: &Connection, jadwal: &str) -> Option<Vec<String>> {
    let bayan = ittisal.prepare(&format!("SELECT * FROM \"{jadwal}\" LIMIT 0")).ok()?;
    Some(bayan.column_names().into_iter().map(str::to_owned).collect())
}

/// The first candidate column name the table actually has, in the table's own
/// spelling, so the lookup that follows is exact.
fn amud_mutah(asmaa: &[String], murashahat: &[&str]) -> Option<String> {
    murashahat
        .iter()
        .find_map(|matlub| asmaa.iter().find(|mawjud| mawjud.eq_ignore_ascii_case(matlub)))
        .cloned()
}

/// Text out of a column, accepting the integer form some columns take.
fn qeema_nass(saff: &Row<'_>, amud: &str) -> Option<String> {
    match saff.get_ref(amud).ok()? {
        ValueRef::Text(bayt) => {
            let nass = String::from_utf8_lossy(bayt).trim().to_owned();
            (!nass.is_empty()).then_some(nass)
        },
        ValueRef::Integer(raqm) => Some(raqm.to_string()),
        ValueRef::Real(_) | ValueRef::Blob(_) | ValueRef::Null => None,
    }
}

/// A non-negative number out of a column, accepting the text form.
///
/// `SizeOnDisk` is an integer in some client versions and a decimal string in
/// others, and a size that vanished because of a pair of quotes would show a
/// hundred-gigabyte install as zero bytes in the library.
fn qeema_raqm(saff: &Row<'_>, amud: &str) -> Option<u64> {
    match saff.get_ref(amud).ok()? {
        ValueRef::Integer(raqm) => u64::try_from(raqm).ok(),
        ValueRef::Text(bayt) => {
            std::str::from_utf8(bayt).ok().and_then(|nass| nass.trim().parse::<u64>().ok())
        },
        ValueRef::Real(_) | ValueRef::Blob(_) | ValueRef::Null => None,
    }
}

/// A boolean out of a column, in every shape `SQLite` lets one be stored in.
///
/// `Installed` decides whether a row becomes a card in somebody's library, so
/// it is read defensively: an integer, the strings `SQLite`'s own drivers write,
/// and nothing else. A value that is none of those yields `None`, and the
/// caller treats an unreadable flag as not installed — the direction that
/// shows too few games rather than a library full of titles that are not there.
fn qeema_sawab(saff: &Row<'_>, amud: &str) -> Option<bool> {
    match saff.get_ref(amud).ok()? {
        ValueRef::Integer(raqm) => Some(raqm != 0),
        ValueRef::Text(bayt) => {
            let nass = String::from_utf8_lossy(bayt).trim().to_ascii_lowercase();
            match nass.as_str() {
                "1" | "true" | "yes" | "installed" => Some(true),
                "0" | "false" | "no" => Some(false),
                _ => None,
            }
        },
        ValueRef::Real(_) | ValueRef::Blob(_) | ValueRef::Null => None,
    }
}

// ---------------------------------------------------------------------------
// The install catalogue
// ---------------------------------------------------------------------------

/// One row of `DbSet` in `GameInstallInfo.sqlite`.
#[derive(Debug, Clone)]
struct SaffTathbeet {
    /// The product ASIN, which is the identity and is never invented.
    muarrif: String,
    /// The installation root the client recorded.
    jidhr: PathBuf,
    /// The title, when the row carries one.
    unwan: Option<String>,
    /// Whether the client considers this title installed.
    mansub: bool,
    /// The installed version string.
    isdar: Option<String>,
    /// Size on disk, zero when the row does not record it.
    hajm: u64,
    /// An install or update timestamp, only when it already carries an offset.
    tarikh: Option<String>,
    /// A last-played timestamp, under the same rule.
    laab: Option<String>,
    /// What the client says it is doing to this title right now.
    hala: Option<SababGhiyab>,
}

/// Reads `DbSet` out of the install catalogue.
fn sufuf_tathbeet(ittisal: &Connection, tanbihat: &mut Vec<TanbihFahs>) -> Vec<SaffTathbeet> {
    let Some(asmaa) = asmaa_aamida(ittisal, JADWAL) else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            JADWAL.to_owned(),
            "the Amazon Games catalogue has no DbSet table, so this client version stores its \
             installs somewhere Taarib does not know yet"
                .to_owned(),
        ));
        return Vec::new();
    };

    let (Some(amud_muarrif), Some(amud_masar)) =
        (amud_mutah(&asmaa, &AAMIDA_MUARRIF), amud_mutah(&asmaa, &AAMIDA_MASAR))
    else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            JADWAL.to_owned(),
            format!(
                "DbSet has no product-identifier or install-directory column; its columns are: {}",
                asmaa.join(", ")
            ),
        ));
        return Vec::new();
    };
    let amud_unwan = amud_mutah(&asmaa, &AAMIDA_UNWAN);
    let amud_mansub = amud_mutah(&asmaa, &AAMIDA_MANSUB);
    let amud_isdar = amud_mutah(&asmaa, &AAMIDA_ISDAR);
    let amud_hajm = amud_mutah(&asmaa, &AAMIDA_HAJM);
    let amud_tarikh = amud_mutah(&asmaa, &AAMIDA_TARIKH);
    let amud_laab = amud_mutah(&asmaa, &AAMIDA_LAAB);
    let amud_hala = amud_mutah(&asmaa, &AAMIDA_HALA);
    let amud_taqaddum = amud_mutah(&asmaa, &AAMIDA_TAQADDUM);

    if amud_mansub.is_none() {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            JADWAL.to_owned(),
            "DbSet has no installed flag, so every entitlement in the account was treated as \
             installed and the ones that are not there will be reported as missing folders"
                .to_owned(),
        ));
    }

    let mut sufuf = Vec::new();
    let Ok(mut bayan) = ittisal.prepare(&format!("SELECT * FROM \"{JADWAL}\"")) else {
        return sufuf;
    };
    let Ok(mut saffat) = bayan.query([]) else {
        return sufuf;
    };

    while sufuf.len() < HADD_SUFUF {
        match saffat.next() {
            Ok(Some(saff)) => {
                let Some(muarrif) = qeema_nass(saff, &amud_muarrif) else {
                    continue;
                };
                let Some(masar) = qeema_nass(saff, &amud_masar) else {
                    // A row with an identity and no directory is an entitlement
                    // the client has not placed anywhere. Not a fault.
                    continue;
                };
                let taqaddum =
                    amud_taqaddum.as_ref().and_then(|amud| qeema_raqm(saff, amud)).and_then(nisba);
                sufuf.push(SaffTathbeet {
                    muarrif,
                    jidhr: PathBuf::from(masar),
                    unwan: amud_unwan.as_ref().and_then(|amud| qeema_nass(saff, amud)),
                    mansub: amud_mansub
                        .as_ref()
                        .is_none_or(|amud| qeema_sawab(saff, amud).unwrap_or(false)),
                    isdar: amud_isdar.as_ref().and_then(|amud| qeema_nass(saff, amud)),
                    hajm: amud_hajm.as_ref().and_then(|amud| qeema_raqm(saff, amud)).unwrap_or(0),
                    tarikh: amud_tarikh
                        .as_ref()
                        .and_then(|amud| qeema_nass(saff, amud))
                        .and_then(|qeema| waqt_maqbul(&qeema)),
                    laab: amud_laab
                        .as_ref()
                        .and_then(|amud| qeema_nass(saff, amud))
                        .and_then(|qeema| waqt_maqbul(&qeema)),
                    hala: amud_hala
                        .as_ref()
                        .and_then(|amud| qeema_nass(saff, amud))
                        .and_then(|qeema| hala_min_qeema(&qeema, taqaddum)),
                });
            },
            Ok(None) => break,
            Err(khata) => {
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    JADWAL.to_owned(),
                    format!("reading DbSet stopped early: {khata}"),
                ));
                break;
            },
        }
    }
    sufuf
}

/// Clamps a progress figure to a percentage, accepting the two units the
/// column is written in.
///
/// Some client versions store a whole percentage and some store a fraction
/// hundredths of one — `9550` for 95.5 per cent. Anything above one hundred is
/// read as the hundredths form; anything still above one hundred after that is
/// discarded rather than reported, because a progress bar that reads two
/// hundred per cent is worse than a progress bar with no number in it.
fn nisba(kham: u64) -> Option<u8> {
    let mabdai = if kham > 100 { kham.div_euclid(100) } else { kham };
    u8::try_from(mabdai).ok().filter(|qeema| *qeema <= 100)
}

/// Maps a transfer-state value onto the vocabulary the interface speaks.
///
/// Amazon's state column is not documented and its values differ between client
/// versions, so the match is on substrings of the lowercased value rather than
/// on an enumeration nobody published. Only the three situations that change
/// what the product may do to a game are recognised:
///
/// | value contains | verdict |
/// | --- | --- |
/// | `download`, `install`, `fetch` | downloading |
/// | `update`, `patch`, `repair` | updating |
/// | `pause`, `suspend`, `stop`, `queue` | a stopped transfer |
/// | anything else | nothing |
///
/// "Anything else" is the important row. A value this table does not know —
/// `Complete`, `Idle`, a numeric code, a state a future client invents — yields
/// `None`, and the game is treated as an ordinary install whose files the
/// existence gate checks for itself. Guessing here would either hide a finished
/// game behind an invented download or offer to patch one that is mid-transfer.
fn hala_min_qeema(qeema: &str, taqaddum: Option<u8>) -> Option<SababGhiyab> {
    let munkhafid = qeema.to_ascii_lowercase();
    if munkhafid.contains("pause")
        || munkhafid.contains("suspend")
        || munkhafid.contains("stop")
        || munkhafid.contains("queue")
    {
        return Some(SababGhiyab::TanzilMutawaqqif { nisba: taqaddum });
    }
    if munkhafid.contains("update")
        || munkhafid.contains("patch")
        || munkhafid.contains("repair")
    {
        return Some(SababGhiyab::QaydTahdith);
    }
    if munkhafid.contains("download")
        || munkhafid.contains("install")
        || munkhafid.contains("fetch")
    {
        return Some(SababGhiyab::QaydTanzil { nisba: taqaddum });
    }
    None
}

// ---------------------------------------------------------------------------
// The product catalogue
// ---------------------------------------------------------------------------

/// What `GameProductInfo.sqlite` adds to a row of the install catalogue.
#[derive(Debug, Clone, Default)]
struct BayanMuntaj {
    /// The title as the storefront spells it, which is frequently better
    /// punctuated than the one the install row carries.
    unwan: Option<String>,
    /// The square product icon.
    ayquna: Option<String>,
    /// The transparent product logo.
    shiar: Option<String>,
    /// A wide background image.
    khalfiya: Option<String>,
}

/// Reads the product catalogue, keyed by ASIN.
///
/// Every failure here is silent and total: a missing file, a missing table, a
/// renamed column, a locked database. None of them can cost the user a game,
/// because everything this catalogue carries is decoration — a nicer title and
/// three image addresses. Reporting a warning for each would fill Diagnostics
/// with noise about artwork.
fn bayanat_muntajat(masar: &Path) -> BTreeMap<String, BayanMuntaj> {
    let mut muntajat: BTreeMap<String, BayanMuntaj> = BTreeMap::new();
    let Ok((ittisal, _)) = iftah_qaida(masar) else {
        return muntajat;
    };
    let Some(asmaa) = asmaa_aamida(&ittisal, JADWAL) else {
        return muntajat;
    };
    let Some(amud_muarrif) = amud_mutah(&asmaa, &AAMIDA_MUARRIF) else {
        return muntajat;
    };
    let amud_unwan = amud_mutah(&asmaa, &AAMIDA_UNWAN);
    let amud_ayquna = amud_mutah(&asmaa, &AAMIDA_AYQUNA);
    let amud_shiar = amud_mutah(&asmaa, &AAMIDA_SHIAR);
    let amud_khalfiya = amud_mutah(&asmaa, &AAMIDA_KHALFIYA);

    let Ok(mut bayan) = ittisal.prepare(&format!("SELECT * FROM \"{JADWAL}\"")) else {
        return muntajat;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return muntajat;
    };
    while muntajat.len() < HADD_SUFUF {
        let Ok(Some(saff)) = sufuf.next() else {
            break;
        };
        let Some(muarrif) = qeema_nass(saff, &amud_muarrif) else {
            continue;
        };
        let _ = muntajat.insert(
            muarrif,
            BayanMuntaj {
                unwan: amud_unwan.as_ref().and_then(|amud| qeema_nass(saff, amud)),
                ayquna: amud_ayquna
                    .as_ref()
                    .and_then(|amud| qeema_nass(saff, amud))
                    .filter(|qeema| rabt(qeema)),
                shiar: amud_shiar
                    .as_ref()
                    .and_then(|amud| qeema_nass(saff, amud))
                    .filter(|qeema| rabt(qeema)),
                khalfiya: amud_khalfiya
                    .as_ref()
                    .and_then(|amud| qeema_nass(saff, amud))
                    .filter(|qeema| rabt(qeema)),
            },
        );
    }
    muntajat
}

/// Whether a catalogue value is an address the artwork pipeline may fetch.
///
/// Only `https`. The client stores plain `http` addresses in older rows and a
/// few local paths, and fetching either would mean a scan that downloads an
/// image over an unauthenticated connection or reads a file the catalogue named
/// with no bound on where it points.
fn rabt(qeema: &str) -> bool {
    qeema.starts_with("https://")
}

// ---------------------------------------------------------------------------
// fuel.json
// ---------------------------------------------------------------------------

/// The executable a `fuel.json` names, relative to the install root.
///
/// The file's shape is `{"Main": {"Command": "Game.exe", "Args": [...]}}`, with
/// a `SchemaVersion` beside it and a `PostInstall` list Taarib has no business
/// reading. `Main.Command` is the only field taken: `Args` are the publisher's
/// launch arguments, not the user's, and putting them in
/// [`LubaMuktashafa::khiyarat_tashghil`] — which exists so an installer extends
/// what the *user* set rather than overwriting it — would make the installer
/// append the publisher's arguments to themselves on every run.
fn tanfidhi_waqud(jidhr: &Path) -> Option<String> {
    let masar = jidhr.join(ISM_WAQUD);
    let nass = qira_nass(&masar).ok()?;
    let qeema: Value = serde_json::from_str(bila_bom(&nass)).ok()?;
    let amr = qeema.get("Main").and_then(|main| nass_haql(main, "Command"));
    amr.filter(|qeema| !qeema.is_empty())
}

/// Whether the client finished writing this install.
///
/// `fuel.json` is written at the end of an install, so its absence inside a
/// directory the catalogue calls installed means the transfer did not finish.
fn tamma_al_tathbeet(jidhr: &Path) -> bool {
    jidhr.join(ISM_WAQUD).is_file()
}

// ---------------------------------------------------------------------------
// Artwork
// ---------------------------------------------------------------------------

/// Where a game's three artwork pieces can be found.
///
/// Local files first, because the client already downloaded them and they cost
/// nothing; the product catalogue's addresses fill whatever the cache did not
/// have.
///
/// The icon is used as the cover. Amazon publishes no vertical box art at all —
/// its storefront is built around a square icon and a wide background — so the
/// alternative to a square cover in the library grid is an empty cell. The
/// interface letterboxes it, which is visibly imperfect and visibly a game.
fn suwar_amazon(jidhr_matjar: &Path, muarrif: &str, muntaj: Option<&BayanMuntaj>) -> MasadirSuwar {
    let mut suwar = suwar_mahalliya(jidhr_matjar, muarrif);
    let Some(muntaj) = muntaj else {
        return suwar;
    };
    if suwar.ghilaf.is_none() {
        suwar.ghilaf = muntaj.ayquna.clone().map(MasdarSura::Rabt);
    }
    if suwar.batl.is_none() {
        suwar.batl = muntaj.khalfiya.clone().map(MasdarSura::Rabt);
    }
    if suwar.shiar.is_none() {
        suwar.shiar = muntaj.shiar.clone().map(MasdarSura::Rabt);
    }
    suwar
}

/// Images the client already downloaded for one product.
///
/// The layout of the client's image cache is **not** documented, so nothing is
/// assumed: the cache directory is only used when it is there, only files that
/// exist are returned, and a file is only claimed for a product when its own
/// name contains that product's ASIN. An ASIN is ten characters of uppercase
/// alphanumerics, which is specific enough that a match is not a coincidence.
/// When the cache holds nothing, the result is empty and the addresses from the
/// product catalogue are used instead.
fn suwar_mahalliya(jidhr_matjar: &Path, muarrif: &str) -> MasadirSuwar {
    let mut suwar = MasadirSuwar::default();
    let mujallad = jidhr_matjar.join("Data").join("Cache");
    if !mujallad.is_dir() {
        return suwar;
    }
    let matlub = muarrif.to_ascii_lowercase();
    for madkhal in walkdir::WalkDir::new(&mujallad)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|madkhal| madkhal.file_type().is_file())
        .take(HADD_MALAFFAT_SUWAR)
    {
        let masar = madkhal.path();
        let sura_maqbula = masar
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| {
                IMTIDADAT_SURA.iter().any(|maqbul| imtidad.eq_ignore_ascii_case(maqbul))
            });
        if !sura_maqbula {
            continue;
        }
        let Some(ism) = masar.file_name().and_then(|ism| ism.to_str()).map(str::to_lowercase)
        else {
            continue;
        };
        if !ism.contains(&matlub) {
            continue;
        }
        if suwar.shiar.is_none() && ism.contains("logo") {
            suwar.shiar = Some(MasdarSura::Malaf(masar.to_path_buf()));
        } else if suwar.batl.is_none()
            && (ism.contains("background") || ism.contains("hero") || ism.contains("wide"))
        {
            suwar.batl = Some(MasdarSura::Malaf(masar.to_path_buf()));
        } else if suwar.ghilaf.is_none() {
            suwar.ghilaf = Some(MasdarSura::Malaf(masar.to_path_buf()));
        }
    }
    suwar
}

// ---------------------------------------------------------------------------
// One row becomes one game
// ---------------------------------------------------------------------------

/// Turns one installed row into a game, or into the warning that explains why
/// it is not in the library.
fn luba_min_saff(
    saff: &SaffTathbeet,
    jidhr_matjar: &Path,
    muntaj: Option<&BayanMuntaj>,
    hala: Option<SababGhiyab>,
) -> Result<LubaMuktashafa, TanbihFahs> {
    if saff.muarrif.is_empty() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            saff.jidhr.display().to_string(),
            "this install has no ASIN in the Amazon catalogue, so there is no identity a patch \
             could be published against"
                .to_owned(),
        ));
    }
    if !saff.jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            saff.jidhr.display().to_string(),
            "Amazon Games still lists this game as installed, but its folder is gone; reinstall \
             it or remove it from the client"
                .to_owned(),
        ));
    }

    let tanfidhi = tanfidhi_waqud(&saff.jidhr)
        .as_deref()
        .and_then(|nisbi| masar_dakhili(&saff.jidhr, nisbi))
        .filter(|masar| masar.is_file());

    let tamm = tamma_al_tathbeet(&saff.jidhr);
    let jariya = saff.hala.as_ref().is_some_and(SababGhiyab::jariya);
    let mutawaqqif = saff.hala.is_some() && !jariya;

    let ism = muntaj
        .and_then(|muntaj| muntaj.unwan.clone())
        .or_else(|| saff.unwan.clone())
        .or_else(|| ism_min_mujallad(&saff.jidhr))
        .unwrap_or_else(|| format!("Amazon {}", saff.muarrif));

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Amazon(saff.muarrif.clone()),
        hala_matjar: hala,
        ism,
        jidhr: saff.jidhr.clone(),
        tanfidhi,
        hajm: saff.hajm,
        bina_manassa: saff.isdar.clone(),
        akhir_tahdith: saff.tarikh.clone(),
        akhir_laab: saff.laab.clone(),
        // Amazon installs Windows games onto Windows and nothing else; a Prime
        // Gaming title running under Proton got there through Heroic, whose own
        // adapter records the prefix.
        beea: BeeatTawafuq::Asli,
        suwar: suwar_amazon(jidhr_matjar, &saff.muarrif, muntaj),
        khiyarat_tashghil: None,
        // Complete means all three: the client says installed, the client is
        // not moving bytes right now, and `fuel.json` is on disk. Patching a
        // game that fails any of them would be patching files the client is
        // about to overwrite.
        muktamila: tamm && !jariya && !mutawaqqif,
        simat: simat_luba(&saff.jidhr),
    })
}

/// What Amazon's own metadata says about a game that the safety layer needs.
///
/// The catalogue records nothing about multiplayer or anti-cheat — it is an
/// install table, not a storefront — so the only hint available is evidence in
/// the install directory itself. The two anti-cheat products that ship inside
/// Prime Gaming titles both leave a directory with their own name at the
/// install root, and naming one here lets the interface warn before the user
/// clicks. Phase 16 still decides on its own evidence; this only makes the
/// refusal faster.
fn simat_luba(jidhr: &Path) -> Vec<SimatLuba> {
    let mut simat = Vec::new();
    for (athar, ism) in [
        ("EasyAntiCheat", "EasyAntiCheat"),
        ("EasyAntiCheat_EOS", "EasyAntiCheat"),
        ("BattlEye", "BattlEye"),
    ] {
        if jidhr.join(athar).is_dir() {
            let sima = SimatLuba::HimayaMuhtamala(ism.to_owned());
            if !simat.contains(&sima) {
                simat.push(sima);
            }
        }
    }
    simat
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// A trimmed, non-empty text field, accepting the number form.
fn nass_haql(qeema: &Value, miftah: &str) -> Option<String> {
    match qeema.get(miftah) {
        Some(Value::String(nass)) => {
            let munazzam = nass.trim();
            (!munazzam.is_empty()).then(|| munazzam.to_owned())
        },
        Some(Value::Number(raqm)) => Some(raqm.to_string()),
        _ => None,
    }
}

/// Joins a relative path onto an install root through the one join that refuses
/// to leave its root.
fn masar_dakhili(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munazzam = nisbi.replace('\\', "/");
    let munazzam = munazzam.trim_start_matches('/');
    dakhil(jidhr, munazzam).ok()
}

/// An install directory's own name.
fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().trim().to_owned())
        .filter(|ism| !ism.is_empty())
}

/// A path in the one form two paths can be compared in on this platform.
fn muwahhad(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy().replace('\\', "/");
    let nass = nass.trim_end_matches('/').to_owned();
    if nizam.hassas_lil_ahruf() { nass } else { nass.to_lowercase() }
}

/// Accepts a timestamp only when it already carries an offset.
///
/// Amazon's date columns are not documented and appear both as UTC strings and
/// as local ones. Stamping `Z` onto a value that might be local time would be
/// inventing a timezone and would show a game as last played tomorrow, so
/// anything not already unambiguous is dropped.
fn waqt_maqbul(qeema: &str) -> Option<String> {
    let nass = qeema.trim();
    let yabdu_tarikh = nass.len() >= 20
        && nass.get(4..5) == Some("-")
        && nass.get(7..8) == Some("-")
        && (nass.get(10..11) == Some("T") || nass.get(10..11) == Some(" "));
    let lahu_mintaqa = nass.ends_with('Z')
        || nass.ends_with('z')
        || nass.rfind(['+', '-']).is_some_and(|mawqi| mawqi > 10);
    (yabdu_tarikh && lahu_mintaqa).then(|| nass.replace(' ', "T"))
}

/// Strips a byte order mark, which no JSON parser accepts and which the
/// client's installer writes in front of some `fuel.json` files.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}
