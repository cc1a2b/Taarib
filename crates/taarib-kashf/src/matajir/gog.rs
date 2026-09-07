//! متجر غوغ — GOG, from Galaxy's database and from standalone installs.
//!
//! GOG reaches a machine two ways, and a scanner that knows only one of them
//! misses half of everybody's library. Galaxy — the client — keeps an `SQLite`
//! database of what it installed. The standalone installers — the DRM-free
//! `setup_*.exe` files GOG is known for — do not use Galaxy at all: they write
//! a registry key and drop a `goggame-<id>.info` file into the game's own
//! directory. Both are read here, and the two lists are merged rather than
//! concatenated.
//!
//! ```text
//! Windows  %PROGRAMDATA%\GOG.com\Galaxy\storage\galaxy-2.0.db
//!          HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>   (standalone)
//! macOS    ~/Library/Application Support/GOG.com/Galaxy/storage/galaxy-2.0.db
//! ```
//!
//! # Opening another program's live database
//!
//! `galaxy-2.0.db` belongs to Galaxy, which may be running, and which will keep
//! running long after this scan finishes. Opening it read-write — even
//! accidentally, even briefly — is how it gets destroyed: a read-write
//! connection may roll back a hot journal, may checkpoint and truncate the
//! write-ahead log out from under the process that owns it, and may migrate the
//! file format. `rusqlite`'s default flags are `READ_WRITE | CREATE`, so the
//! default is exactly the thing that must never happen, and the flags below are
//! therefore spelled out in full rather than derived from `OpenFlags::default()`.
//!
//! Two read-only modes exist and this adapter tries both, in this order:
//!
//! 1. **`?immutable=1`** with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI`. `SQLite`
//!    takes no locks at all, creates no `-shm` file, reads no `-wal`, and never
//!    touches the directory. A running Galaxy cannot observe that the file was
//!    opened. The cost is honest and bounded: rows committed only to the
//!    write-ahead log are invisible, and a write landing during the read can
//!    make a page look torn — which `SQLite` reports as an error on the query,
//!    and which this adapter turns into a [`TanbihFahs`], not a panic.
//! 2. **`?mode=ro`**, plain read-only, used when the first attempt cannot even
//!    enumerate the schema. `SQLite` now takes proper shared read locks and reads
//!    the write-ahead log, so the snapshot is consistent; it may create a
//!    `-shm` in Galaxy's directory, which is what every read-only `SQLite` client
//!    does and is not a modification of Galaxy's data.
//!
//! Neither mode can write the main database. `SQLITE_OPEN_CREATE` is never
//! passed, so a missing file is an error rather than a new, empty database
//! appearing inside another product's storage directory.
//!
//! # The schema is not ours
//!
//! Everything below about Galaxy's tables is **inferred** from databases in the
//! wild. GOG publishes no schema, guarantees nothing, and has changed it before.
//! So no column is named in a query until the table has been asked what columns
//! it has: every table is opened with `SELECT * … LIMIT 0`, its column names are
//! read back, and the roles this adapter needs — product identifier,
//! installation path, build identifier — are matched against a list of
//! candidate names. A table that is absent, or a table that is present without
//! a usable column, produces one [`TanbihFahs`] naming it and the scan
//! continues with whatever else it could read. A Galaxy update that renames a
//! column costs the user the fields that came from it, never the library.
//!
//! The build identifier and the executable are preferred from
//! `goggame-<id>.info` inside the game's own directory, because that file is
//! written by GOG's own installer, is documented by its own use, and describes
//! the build that is actually on disk rather than a build the catalogue knows
//! about.
//!
//! # Add-ons
//!
//! A `goggame-<id>.info` whose `gameId` differs from its `rootGameId` describes
//! downloadable content installed into the base game's directory. It is not a
//! separate game: it has no executable, and emitting it would put a second card
//! in the library pointing at the same files. Such files are read only to be
//! skipped, and the base game's own `.info` is the one that is used.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Row};
use serde_json::Value;
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
const MUARRIF: &str = "gog";

/// Platforms this adapter can find anything on. Linux is absent on purpose:
/// Galaxy has no Linux client, and GOG games on Linux arrive through Heroic,
/// whose `installed.json` is a different catalogue and a different adapter.
const MANASSAT: [NizamTashghil; 2] = [NizamTashghil::Windows, NizamTashghil::Mac];

/// The database file, under the launcher's `storage` directory.
const ISM_QAIDA: &str = "galaxy-2.0.db";

/// How long a query waits for a lock before giving up. Short on purpose: this
/// is a background scan competing with a program the user is looking at.
const MUHLAT_INTIZAR: Duration = Duration::from_millis(250);

/// Tables that may hold the installed-product list, best first.
const JADAWIL_TATHBEET: [&str; 2] = ["InstalledBaseProducts", "InstalledProducts"];

/// Column names that may carry a product identifier, best first.
const AAMIDA_MUARRIF: [&str; 4] = ["productId", "gameId", "gogId", "id"];

/// Column names that may carry an installation path.
const AAMIDA_MASAR: [&str; 5] =
    ["installationPath", "installPath", "localPath", "location", "path"];

/// Column names that may carry an installation timestamp.
const AAMIDA_TARIKH: [&str; 3] = ["installationDate", "installDate", "date"];

/// Column names that may carry a size on disk.
const AAMIDA_HAJM: [&str; 3] = ["installedSize", "totalSize", "size"];

/// Column names that may carry a build identifier.
const AAMIDA_BINA: [&str; 3] = ["buildId", "buildID", "build"];

/// Image file extensions the artwork pipeline can actually decode. An icon or
/// a format outside this list is left alone rather than handed downstream to
/// fail there.
const IMTIDADAT_SURA: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// How many files are examined inside one product's image cache directory.
const HADD_MALAFFAT_SUWAR: usize = 64;

/// GOG.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarGog;

impl MatjarGog {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarGog {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "غوغ"
    }

    fn ism_injilizi(&self) -> &'static str {
        "GOG"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        if let Some(tajawuz) = siyaq.manassat.gog.as_ref() {
            return Some(tajawuz.clone());
        }
        let jidhr = jidhr_tilqai(siyaq)?;
        // An existence check, not a parse: the database is not opened here.
        (jidhr.is_dir() || malaf_qaida(&jidhr).is_some()).then_some(jidhr)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when a configured launcher
    /// root is not there, and [`KhataKashf::TaadhurQiraatFahras`] when the
    /// database exists and neither read-only mode can read its schema — which
    /// is the only failure that makes the whole Galaxy catalogue unreadable.
    /// Standalone installs are still returned in that case, because they do not
    /// come from the database at all.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let jidhr = match siyaq.manassat.gog.as_ref() {
            Some(tajawuz) if !tajawuz.exists() => {
                return Err(KhataKashf::JidhrMuhaddadMafqud {
                    matjar: MUARRIF,
                    masar: tajawuz.clone(),
                }
                .into());
            },
            Some(tajawuz) => Some(tajawuz.clone()),
            None => jidhr_tilqai(siyaq),
        };

        // Carried as a pair so that the database and the root it was found
        // under cannot drift apart: `alaab_galaxy` resolves install paths
        // against that root, and reaching for the root separately would let a
        // future edit hand it a different one.
        let qaida =
            jidhr.as_ref().and_then(|jidhr| malaf_qaida(jidhr).map(|masar| (jidhr, masar)));
        let mustaqilla = tathbitat_mustaqilla();
        let jidhr_mawjud = jidhr.clone().filter(|masar| masar.is_dir());
        if qaida.is_none() && mustaqilla.is_empty() {
            // Galaxy's folder is here and its database is not: what `mawqi`
            // calls installed, this must not call absent.
            let Some(jidhr_mawjud) = jidhr_mawjud else {
                return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
            };
            let matlub = [jidhr_mawjud.join("storage").join(ISM_QAIDA), jidhr_mawjud.join(ISM_QAIDA)]
                .iter()
                .map(|masar| masar.display().to_string())
                .collect::<Vec<String>>()
                .join(", ");
            return Ok(NatijatMatjar::naqisa(
                MUARRIF,
                Some(jidhr_mawjud),
                matlub,
                "GOG Galaxy is installed here but its database is not, and the registry lists \
                 no standalone GOG install, so no GOG game could be listed; open Galaxy once so \
                 it recreates the database, or correct the configured root",
            ));
        }

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, jidhr_mawjud);
        let mut fahras: BTreeMap<u64, LubaMuktashafa> = BTreeMap::new();
        let mut mawaqi: BTreeSet<String> = BTreeSet::new();

        if let Some((jidhr_qaida, masar_qaida)) = qaida.as_ref() {
            let ittisal = iftah_qaida(masar_qaida).map_err(|tafsil| {
                KhataKashf::TarwisatFahrasTalifa {
                    matjar: MUARRIF,
                    masar: masar_qaida.clone(),
                    tafsil,
                    mawdi: None,
                }
            })?;
            alaab_galaxy(
                &ittisal,
                jidhr_qaida,
                siyaq,
                &mut fahras,
                &mut mawaqi,
                &mut natija.tanbihat,
            );
        }

        for tathbeet in mustaqilla {
            damm_mustaqilla(&tathbeet, siyaq, &mut fahras, &mut mawaqi, &mut natija.tanbihat);
        }

        natija.alaab = fahras.into_values().collect();
        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let jidhr = siyaq.manassat.gog.clone().or_else(|| jidhr_tilqai(siyaq));
        // The storage directory, not the database file: Galaxy writes its
        // journal and its `-wal` alongside, and a write to either is the signal
        // that something was installed. Standalone installs write to the
        // registry, which no filesystem watch can see, so those still need a
        // rescan on demand — which is why the interface always offers one.
        jidhr
            .as_deref()
            .and_then(malaf_qaida)
            .and_then(|masar| masar.parent().map(Path::to_path_buf))
            .filter(|masar| masar.is_dir())
            .into_iter()
            .collect()
    }
}

/// Galaxy's data root, before any user override.
///
/// [`None`] on a Windows context that carries no machine-wide data directory.
/// The macOS layout is under the home directory and so always resolves.
fn jidhr_tilqai(siyaq: &SiyaqFahs) -> Option<PathBuf> {
    match siyaq.nizam {
        NizamTashghil::Windows => {
            Some(siyaq.bayanat_barnamij.as_ref()?.join("GOG.com").join("Galaxy"))
        },
        NizamTashghil::Mac | NizamTashghil::Linux => Some(
            siyaq
                .manzil
                .join("Library")
                .join("Application Support")
                .join("GOG.com")
                .join("Galaxy"),
        ),
    }
}

/// Locates the database under a launcher root, accepting the three shapes a
/// configured override might take.
fn malaf_qaida(jidhr: &Path) -> Option<PathBuf> {
    if jidhr.is_file() {
        return Some(jidhr.to_path_buf());
    }
    let murashahat = [jidhr.join("storage").join(ISM_QAIDA), jidhr.join(ISM_QAIDA)];
    murashahat.into_iter().find(|masar| masar.is_file())
}

// ---------------------------------------------------------------------------
// The database
// ---------------------------------------------------------------------------

/// Opens the database read-only, immutably first.
///
/// # Errors
///
/// Returns a sentence describing both attempts when neither can enumerate the
/// schema, which is what the caller turns into a catalogue-level failure.
fn iftah_qaida(masar: &Path) -> Result<Connection, String> {
    let hudud = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Some(rabt) = rabt_qaida(masar) else {
        return Err("the database path cannot be expressed as a SQLite URI".to_owned());
    };
    match jarrib_fath(&format!("{rabt}?immutable=1"), hudud) {
        Ok(ittisal) => Ok(ittisal),
        Err(thabit) => match jarrib_fath(&format!("{rabt}?mode=ro"), hudud) {
            Ok(ittisal) => Ok(ittisal),
            Err(qari) => Err(format!(
                "immutable read failed ({thabit}), and read-only read failed ({qari})"
            )),
        },
    }
}

/// Opens one connection and proves it can read the schema before returning it.
fn jarrib_fath(rabt: &str, hudud: OpenFlags) -> Result<Connection, String> {
    let ittisal = Connection::open_with_flags(rabt, hudud).map_err(|khata| khata.to_string())?;
    let _ = ittisal.busy_timeout(MUHLAT_INTIZAR);
    {
        let mut bayan = ittisal
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .map_err(|khata| khata.to_string())?;
        let mut sufuf = bayan.query([]).map_err(|khata| khata.to_string())?;
        sufuf.next().map_err(|khata| khata.to_string())?;
    }
    Ok(ittisal)
}

/// Builds the `file:` URI `SQLite` needs in order to accept query parameters.
///
/// Percent-encoding is done here rather than left to `SQLite` because a database
/// under a path containing a space, a `#`, or a `?` would otherwise be opened
/// as a different file, or as a file with a query string nobody wrote.
fn rabt_qaida(masar: &Path) -> Option<String> {
    use std::fmt::Write as _;

    let nass = masar.to_str()?;
    let mut mabni = String::with_capacity(nass.len() + 16);
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

/// The first candidate column name the table actually has.
fn amud_mutah(asmaa: &[String], murashahat: &[&str]) -> Option<String> {
    murashahat
        .iter()
        .find(|matlub| asmaa.iter().any(|mawjud| mawjud.eq_ignore_ascii_case(matlub)))
        .map(|matlub| (*matlub).to_owned())
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
fn qeema_raqm(saff: &Row<'_>, amud: &str) -> Option<u64> {
    match saff.get_ref(amud).ok()? {
        ValueRef::Integer(raqm) => u64::try_from(raqm).ok(),
        ValueRef::Text(bayt) => {
            std::str::from_utf8(bayt).ok().and_then(|nass| nass.trim().parse::<u64>().ok())
        },
        ValueRef::Real(_) | ValueRef::Blob(_) | ValueRef::Null => None,
    }
}

/// One installed product, as the database describes it.
#[derive(Debug, Clone)]
struct TathbeetGalaxy {
    /// The GOG product identifier.
    muarrif: u64,
    /// The installation root.
    jidhr: PathBuf,
    /// The installation timestamp, only when it is already an offset-bearing
    /// RFC 3339 value.
    tarikh: Option<String>,
    /// Size on disk, when the database records one.
    hajm: u64,
    /// The build identifier the database records, which the game's own `.info`
    /// file overrides when it has one.
    bina: Option<String>,
}

/// Reads Galaxy's installed-product list.
fn tathbitat_galaxy(ittisal: &Connection, tanbihat: &mut Vec<TanbihFahs>) -> Vec<TathbeetGalaxy> {
    let mut mawjud = None;
    for jadwal in JADAWIL_TATHBEET {
        if let Some(asmaa) = asmaa_aamida(ittisal, jadwal) {
            mawjud = Some((jadwal, asmaa));
            break;
        }
    }
    let Some((jadwal, asmaa)) = mawjud else {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            JADAWIL_TATHBEET.join(" / "),
            "Galaxy's database has no installed-product table under any name Taarib knows, so \
             only standalone GOG installs were found"
                .to_owned(),
        ));
        return Vec::new();
    };

    let (Some(amud_muarrif), Some(amud_masar)) =
        (amud_mutah(&asmaa, &AAMIDA_MUARRIF), amud_mutah(&asmaa, &AAMIDA_MASAR))
    else {
        tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            jadwal.to_owned(),
            format!(
                "table {jadwal} has no product-identifier or installation-path column; its \
                 columns are: {}",
                asmaa.join(", ")
            ),
        ));
        return Vec::new();
    };
    let amud_tarikh = amud_mutah(&asmaa, &AAMIDA_TARIKH);
    let amud_hajm = amud_mutah(&asmaa, &AAMIDA_HAJM);
    let amud_bina = amud_mutah(&asmaa, &AAMIDA_BINA);

    let mut tathbitat = Vec::new();
    let Ok(mut bayan) = ittisal.prepare(&format!("SELECT * FROM \"{jadwal}\"")) else {
        return tathbitat;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return tathbitat;
    };
    loop {
        match sufuf.next() {
            Ok(Some(saff)) => {
                let (Some(muarrif), Some(masar)) =
                    (qeema_raqm(saff, &amud_muarrif), qeema_nass(saff, &amud_masar))
                else {
                    continue;
                };
                tathbitat.push(TathbeetGalaxy {
                    muarrif,
                    jidhr: PathBuf::from(masar),
                    tarikh: amud_tarikh
                        .as_ref()
                        .and_then(|amud| qeema_nass(saff, amud))
                        .and_then(|qeema| waqt_maqbul(&qeema)),
                    hajm: amud_hajm.as_ref().and_then(|amud| qeema_raqm(saff, amud)).unwrap_or(0),
                    bina: amud_bina.as_ref().and_then(|amud| qeema_nass(saff, amud)),
                });
            },
            Ok(None) => break,
            Err(khata) => {
                tanbihat.push(TanbihFahs::fahras(
                    MUARRIF,
                    jadwal.to_owned(),
                    format!("reading {jadwal} stopped early: {khata}"),
                ));
                break;
            },
        }
    }
    tathbitat
}

/// Titles keyed by product identifier, from `LimitedDetails`.
fn anawin_galaxy(ittisal: &Connection) -> BTreeMap<u64, String> {
    let mut anawin = BTreeMap::new();
    let Some(asmaa) = asmaa_aamida(ittisal, "LimitedDetails") else {
        return anawin;
    };
    let (Some(amud_muarrif), Some(amud_unwan)) =
        (amud_mutah(&asmaa, &["productId", "id"]), amud_mutah(&asmaa, &["title", "name"]))
    else {
        return anawin;
    };
    let Ok(mut bayan) = ittisal.prepare("SELECT * FROM \"LimitedDetails\"") else {
        return anawin;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return anawin;
    };
    while let Ok(Some(saff)) = sufuf.next() {
        if let (Some(muarrif), Some(unwan)) =
            (qeema_raqm(saff, &amud_muarrif), qeema_nass(saff, &amud_unwan))
        {
            let _ = anawin.insert(muarrif, unwan);
        }
    }
    anawin
}

/// What `GamePieces` says about each product: its title and its features.
///
/// `GamePieces` stores one JSON document per product per piece type, keyed by a
/// release key of the form `gog_<productId>`, with the type names living in
/// `GamePieceTypes`. Both the key format and the document shapes are inferred
/// from real databases; every step is guarded, and a shape that does not match
/// yields nothing rather than a wrong answer.
fn qita_galaxy(ittisal: &Connection) -> BTreeMap<u64, (Option<String>, Vec<SimatLuba>)> {
    let mut natija: BTreeMap<u64, (Option<String>, Vec<SimatLuba>)> = BTreeMap::new();

    let anwa = anwa_qita(ittisal);

    let Some(asmaa) = asmaa_aamida(ittisal, "GamePieces") else {
        return natija;
    };
    let (Some(amud_miftah), Some(amud_qeema)) = (
        amud_mutah(&asmaa, &["releaseKey", "gameReleaseKey"]),
        amud_mutah(&asmaa, &["value", "json"]),
    ) else {
        return natija;
    };
    let amud_naw = amud_mutah(&asmaa, &["gamePieceTypeId", "typeId"]);
    let Ok(mut bayan) = ittisal.prepare("SELECT * FROM \"GamePieces\"") else {
        return natija;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return natija;
    };

    while let Ok(Some(saff)) = sufuf.next() {
        let Some(miftah) = qeema_nass(saff, &amud_miftah) else {
            continue;
        };
        let Some(muarrif) = miftah
            .strip_prefix("gog_")
            .and_then(|raqm| raqm.trim().parse::<u64>().ok())
        else {
            continue;
        };
        let naw = amud_naw
            .as_ref()
            .and_then(|amud| qeema_raqm(saff, amud))
            .and_then(|raqm| anwa.get(&raqm).cloned())
            .unwrap_or_default();
        let Some(nass) = qeema_nass(saff, &amud_qeema) else {
            continue;
        };
        let Ok(qeema) = serde_json::from_str::<Value>(&nass) else {
            continue;
        };

        let madkhal = natija.entry(muarrif).or_insert((None, Vec::new()));
        if naw.contains("title")
            && let Some(unwan) = qeema.get("title").and_then(Value::as_str)
            && !unwan.trim().is_empty()
            && madkhal.0.is_none()
        {
            madkhal.0 = Some(unwan.trim().to_owned());
        }
        if naw.contains("features")
            && let Some(mizat) = qeema.get("features").and_then(Value::as_array)
        {
            for miza in mizat {
                let Some(ism) = miza.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(sima) = sima_min_miza(ism)
                    && !madkhal.1.contains(&sima)
                {
                    madkhal.1.push(sima);
                }
            }
        }
    }
    natija
}

/// The `GamePieces` type table: numeric type identifier to lowercased name.
fn anwa_qita(ittisal: &Connection) -> BTreeMap<u64, String> {
    let mut anwa = BTreeMap::new();
    let Some(asmaa) = asmaa_aamida(ittisal, "GamePieceTypes") else {
        return anwa;
    };
    let (Some(amud_muarrif), Some(amud_naw)) =
        (amud_mutah(&asmaa, &["id"]), amud_mutah(&asmaa, &["type", "name"]))
    else {
        return anwa;
    };
    let Ok(mut bayan) = ittisal.prepare("SELECT * FROM \"GamePieceTypes\"") else {
        return anwa;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return anwa;
    };
    while let Ok(Some(saff)) = sufuf.next() {
        if let (Some(muarrif), Some(naw)) =
            (qeema_raqm(saff, &amud_muarrif), qeema_nass(saff, &amud_naw))
        {
            let _ = anwa.insert(muarrif, naw.to_lowercase());
        }
    }
    anwa
}

/// Maps one of GOG's feature names onto what the safety layer wants to know.
fn sima_min_miza(ism: &str) -> Option<SimatLuba> {
    let munkhafid = ism.to_lowercase();
    if munkhafid.contains("local")
        || munkhafid.contains("hotseat")
        || munkhafid.contains("split")
        || munkhafid.contains("lan")
    {
        return Some(SimatLuba::JamaiMahalli);
    }
    if munkhafid.contains("multi")
        || munkhafid.contains("co-op")
        || munkhafid.contains("coop")
        || munkhafid.contains("online")
    {
        return Some(SimatLuba::JamaiOnline);
    }
    None
}

/// Last-played timestamps keyed by product identifier.
fn akhir_laab_galaxy(ittisal: &Connection) -> BTreeMap<u64, String> {
    let mut awqat = BTreeMap::new();
    let Some(asmaa) = asmaa_aamida(ittisal, "LastPlayedDates") else {
        return awqat;
    };
    let (Some(amud_miftah), Some(amud_waqt)) = (
        amud_mutah(&asmaa, &["releaseKey", "gameReleaseKey"]),
        amud_mutah(&asmaa, &["lastPlayedDate", "date", "lastPlayed"]),
    ) else {
        return awqat;
    };
    let Ok(mut bayan) = ittisal.prepare("SELECT * FROM \"LastPlayedDates\"") else {
        return awqat;
    };
    let Ok(mut sufuf) = bayan.query([]) else {
        return awqat;
    };
    while let Ok(Some(saff)) = sufuf.next() {
        let Some(muarrif) = qeema_nass(saff, &amud_miftah)
            .as_deref()
            .and_then(|miftah| miftah.strip_prefix("gog_"))
            .and_then(|raqm| raqm.trim().parse::<u64>().ok())
        else {
            continue;
        };
        if let Some(waqt) = qeema_nass(saff, &amud_waqt).and_then(|qeema| waqt_maqbul(&qeema)) {
            let _ = awqat.insert(muarrif, waqt);
        }
    }
    awqat
}

/// Accepts a timestamp only when it already carries an offset.
///
/// Galaxy's date columns are not documented, and this build has no date
/// formatter of its own to normalize an epoch or a naive local timestamp with.
/// Stamping `Z` onto a value that might be local time would be inventing a
/// timezone, so anything that is not already unambiguous is dropped.
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

/// Turns Galaxy's rows into games, filling every field the database left empty
/// from the game's own `goggame-<id>.info`.
fn alaab_galaxy(
    ittisal: &Connection,
    jidhr_matjar: &Path,
    siyaq: &SiyaqFahs,
    fahras: &mut BTreeMap<u64, LubaMuktashafa>,
    mawaqi: &mut BTreeSet<String>,
    tanbihat: &mut Vec<TanbihFahs>,
) {
    let tathbitat = tathbitat_galaxy(ittisal, tanbihat);
    if tathbitat.is_empty() {
        return;
    }
    let anawin = anawin_galaxy(ittisal);
    let qita = qita_galaxy(ittisal);
    let awqat = akhir_laab_galaxy(ittisal);

    for tathbeet in tathbitat {
        if !tathbeet.jidhr.is_dir() {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                tathbeet.jidhr.display().to_string(),
                "Galaxy still lists this game, but its install directory is gone; reinstall it \
                 or remove it from Galaxy"
                    .to_owned(),
            ));
            continue;
        }

        let bayan = bayan_goggame(&tathbeet.jidhr, Some(tathbeet.muarrif));
        let mut simat: Vec<SimatLuba> =
            qita.get(&tathbeet.muarrif).map(|zawj| zawj.1.clone()).unwrap_or_default();
        simat.dedup();

        let ism = bayan
            .as_ref()
            .and_then(|bayan| bayan.ism.clone())
            .or_else(|| anawin.get(&tathbeet.muarrif).cloned())
            .or_else(|| qita.get(&tathbeet.muarrif).and_then(|zawj| zawj.0.clone()))
            .or_else(|| ism_min_mujallad(&tathbeet.jidhr))
            .unwrap_or_else(|| format!("GOG {}", tathbeet.muarrif));

        let tanfidhi = bayan
            .as_ref()
            .and_then(|bayan| bayan.tanfidhi.as_deref())
            .and_then(|nisbi| masar_dakhili(&tathbeet.jidhr, nisbi))
            .filter(|masar| masar.is_file());

        let _ = mawaqi.insert(muwahhad(&tathbeet.jidhr, siyaq.nizam));
        let _ = fahras.insert(
            tathbeet.muarrif,
            LubaMuktashafa {
                masdar: MasdarLuba::Gog(tathbeet.muarrif),
                hala_matjar: None,
                ism,
                jidhr: tathbeet.jidhr.clone(),
                tanfidhi,
                hajm: tathbeet.hajm,
                bina_manassa: bayan
                    .as_ref()
                    .and_then(|bayan| bayan.bina.clone())
                    .or(tathbeet.bina),
                akhir_tahdith: tathbeet.tarikh,
                akhir_laab: awqat.get(&tathbeet.muarrif).cloned(),
                beea: BeeatTawafuq::Asli,
                suwar: suwar_mahalliya(jidhr_matjar, tathbeet.muarrif),
                khiyarat_tashghil: None,
                // Galaxy removes a product from its installed list while it is
                // downloading and adds it when the install completes, so a row
                // that is present describes a finished install.
                muktamila: true,
                simat,
            },
        );
    }
}

// ---------------------------------------------------------------------------
// goggame-<id>.info
// ---------------------------------------------------------------------------

/// What a `goggame-<id>.info` file says about the install it sits inside.
#[derive(Debug, Clone, Default)]
struct BayanGoggame {
    /// The title, as GOG's installer wrote it.
    ism: Option<String>,
    /// The build that is on disk.
    bina: Option<String>,
    /// The primary executable, relative to the install root.
    tanfidhi: Option<String>,
}

/// Reads the base game's `.info` file out of an install directory.
///
/// Several may be present — one per installed add-on — so the one whose
/// identifier matches the catalogue is preferred, then the one that declares
/// itself its own root, and add-on files are never used as if they were games.
fn bayan_goggame(jidhr: &Path, matlub: Option<u64>) -> Option<BayanGoggame> {
    let mut murashahat: Vec<PathBuf> = std::fs::read_dir(jidhr)
        .ok()?
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            masar
                .file_name()
                .and_then(|ism| ism.to_str())
                .is_some_and(|ism| {
                    let munkhafid = ism.to_ascii_lowercase();
                    munkhafid.starts_with("goggame-")
                        && Path::new(&munkhafid)
                            .extension()
                            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("info"))
                })
        })
        .collect();
    murashahat.sort();

    let mut ihtiyati = None;
    for masar in murashahat {
        let Ok(nass) = qira_nass(&masar) else {
            continue;
        };
        let Ok(qeema) = serde_json::from_str::<Value>(bila_bom(&nass)) else {
            continue;
        };
        let muarrif = nass_haql(&qeema, "gameId").and_then(|raqm| raqm.parse::<u64>().ok());
        let jidhr_luba = nass_haql(&qeema, "rootGameId").and_then(|raqm| raqm.parse::<u64>().ok());
        // An add-on: its own identifier is not its root's. It shares this
        // directory with the game it extends and is never a game itself.
        if let (Some(muarrif), Some(jidhr_luba)) = (muarrif, jidhr_luba)
            && muarrif != jidhr_luba
        {
            continue;
        }

        let bayan = BayanGoggame {
            ism: nass_haql(&qeema, "name"),
            bina: nass_haql(&qeema, "buildId"),
            tanfidhi: tanfidhi_goggame(&qeema),
        };
        if matlub.is_some() && muarrif == matlub {
            return Some(bayan);
        }
        if ihtiyati.is_none() {
            ihtiyati = Some(bayan);
        }
    }
    ihtiyati
}

/// The primary play task's executable, relative to the install root.
fn tanfidhi_goggame(qeema: &Value) -> Option<String> {
    let mahamm = qeema.get("playTasks").and_then(Value::as_array)?;
    let mut ikhtiyar: Option<&Value> = None;
    let mut martaba: u8 = 0;
    for muhimma in mahamm {
        // Only a task that runs a file can be an executable; the rest open a
        // manual or a web page.
        if !nass_haql(muhimma, "type").is_some_and(|naw| naw.eq_ignore_ascii_case("FileTask")) {
            continue;
        }
        let rutba = if muhimma.get("isPrimary").and_then(Value::as_bool).unwrap_or(false) {
            3
        } else if nass_haql(muhimma, "category")
            .is_some_and(|sinf| sinf.eq_ignore_ascii_case("game"))
        {
            2
        } else {
            1
        };
        if rutba > martaba {
            martaba = rutba;
            ikhtiyar = Some(muhimma);
        }
    }
    nass_haql(ikhtiyar?, "path")
}

// ---------------------------------------------------------------------------
// Standalone installs
// ---------------------------------------------------------------------------

/// One standalone install, as the Windows registry describes it.
#[derive(Debug, Clone)]
struct TathbeetMustaqill {
    /// The product identifier, from the key name or from `gameID`.
    muarrif: u64,
    /// The installation root, from `path`.
    jidhr: PathBuf,
    /// The title, from `gameName`.
    ism: Option<String>,
    /// The version string, from `ver`.
    isdar: Option<String>,
    /// The executable, from `exe` or `exeFile`.
    tanfidhi: Option<String>,
}

/// Every standalone GOG install the registry knows about.
///
/// The registry is the only place a standalone installer records anything.
#[cfg(windows)]
fn tathbitat_mustaqilla() -> Vec<TathbeetMustaqill> {
    sijill::alaab_gog()
}

/// Every standalone GOG install the registry knows about.
///
/// Empty on every platform but Windows: there is no registry to read, and the
/// standalone installers are Windows-only anyway.
#[cfg(not(windows))]
const fn tathbitat_mustaqilla() -> Vec<TathbeetMustaqill> {
    Vec::new()
}

/// Adds a standalone install, or completes the Galaxy entry that already covers
/// it.
///
/// The same game is frequently both: installed by the standalone installer and
/// then adopted by Galaxy, or installed by Galaxy while an old registry key
/// survives. Matching on the product identifier and on the install path catches
/// both, and the registry's version string fills the gap Galaxy leaves when it
/// records no build.
fn damm_mustaqilla(
    tathbeet: &TathbeetMustaqill,
    siyaq: &SiyaqFahs,
    fahras: &mut BTreeMap<u64, LubaMuktashafa>,
    mawaqi: &mut BTreeSet<String>,
    tanbihat: &mut Vec<TanbihFahs>,
) {
    if !tathbeet.jidhr.is_dir() {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            tathbeet.jidhr.display().to_string(),
            "a GOG registry entry points at an install directory that is gone".to_owned(),
        ));
        return;
    }
    let muwahhad_masar = muwahhad(&tathbeet.jidhr, siyaq.nizam);

    if let Some(mawjud) = fahras.get_mut(&tathbeet.muarrif) {
        if mawjud.bina_manassa.is_none() {
            mawjud.bina_manassa.clone_from(&tathbeet.isdar);
        }
        if mawjud.tanfidhi.is_none() {
            mawjud.tanfidhi = tathbeet
                .tanfidhi
                .as_deref()
                .and_then(|nisbi| masar_mutlaq_aw_dakhili(&tathbeet.jidhr, nisbi));
        }
        return;
    }
    if mawaqi.contains(&muwahhad_masar) {
        return;
    }

    let bayan = bayan_goggame(&tathbeet.jidhr, Some(tathbeet.muarrif));
    let _ = mawaqi.insert(muwahhad_masar);
    let _ = fahras.insert(
        tathbeet.muarrif,
        LubaMuktashafa {
            masdar: MasdarLuba::Gog(tathbeet.muarrif),
            hala_matjar: None,
            ism: tathbeet
                .ism
                .clone()
                .or_else(|| bayan.as_ref().and_then(|bayan| bayan.ism.clone()))
                .or_else(|| ism_min_mujallad(&tathbeet.jidhr))
                .unwrap_or_else(|| format!("GOG {}", tathbeet.muarrif)),
            jidhr: tathbeet.jidhr.clone(),
            tanfidhi: tathbeet
                .tanfidhi
                .as_deref()
                .and_then(|nisbi| masar_mutlaq_aw_dakhili(&tathbeet.jidhr, nisbi))
                .or_else(|| {
                    bayan
                        .as_ref()
                        .and_then(|bayan| bayan.tanfidhi.as_deref())
                        .and_then(|nisbi| masar_dakhili(&tathbeet.jidhr, nisbi))
                })
                .filter(|masar| masar.is_file()),
            hajm: 0,
            bina_manassa: bayan
                .as_ref()
                .and_then(|bayan| bayan.bina.clone())
                .or_else(|| tathbeet.isdar.clone()),
            akhir_tahdith: None,
            akhir_laab: None,
            beea: BeeatTawafuq::Asli,
            suwar: MasadirSuwar::default(),
            khiyarat_tashghil: None,
            muktamila: true,
            simat: Vec::new(),
        },
    );
}

// ---------------------------------------------------------------------------
// Artwork
// ---------------------------------------------------------------------------

/// Images Galaxy already downloaded for one product.
///
/// Galaxy keeps a web cache beside its database. The directory layout inside it
/// is **not** documented, so nothing is assumed: the product's directory is
/// only used when it is actually there, only files that exist are returned, and
/// a file is only claimed as a cover, a banner or a logo when its own name says
/// so. Nothing is left to a URL pattern — an empty result is handled by `suwar`
/// and a wrong URL is a broken image.
fn suwar_mahalliya(jidhr_matjar: &Path, muarrif: u64) -> MasadirSuwar {
    let mut suwar = MasadirSuwar::default();
    let mujallad = jidhr_matjar.join("webcache").join(muarrif.to_string());
    if !mujallad.is_dir() {
        return suwar;
    }
    for madkhal in walkdir::WalkDir::new(&mujallad)
        .max_depth(3)
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
        if suwar.ghilaf.is_none()
            && (ism.contains("vertical") || ism.contains("cover") || ism.contains("boxart"))
        {
            suwar.ghilaf = Some(MasdarSura::Malaf(masar.to_path_buf()));
        } else if suwar.batl.is_none() && (ism.contains("background") || ism.contains("hero")) {
            suwar.batl = Some(MasdarSura::Malaf(masar.to_path_buf()));
        } else if suwar.shiar.is_none() && ism.contains("logo") {
            suwar.shiar = Some(MasdarSura::Malaf(masar.to_path_buf()));
        }
    }
    suwar
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// A trimmed, non-empty text field.
///
/// The number form is accepted as well, because GOG's installer writes the
/// large identifiers — `buildId` above all — unquoted in some versions and
/// quoted in others, and a build identifier that vanished because of a pair of
/// quotes would silently cost Phase 15 its update-survival check.
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

/// Accepts either the absolute executable path the registry sometimes holds or
/// a name relative to the install root.
fn masar_mutlaq_aw_dakhili(jidhr: &Path, qeema: &str) -> Option<PathBuf> {
    let murashah = Path::new(qeema);
    if murashah.is_absolute() {
        return murashah.is_file().then(|| murashah.to_path_buf());
    }
    masar_dakhili(jidhr, qeema).filter(|masar| masar.is_file())
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

/// Strips a byte order mark, which GOG's installer writes in front of some
/// `.info` files and which no JSON parser accepts.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}

// ---------------------------------------------------------------------------
// The Windows registry
// ---------------------------------------------------------------------------

/// Standalone GOG installs, read from the registry.
///
/// Three views of the same key are tried because a launcher is not consistent
/// about which one it writes: the literal `WOW6432Node` path read without
/// redirection, the 32-bit view of the plain path (which the redirector maps to
/// `WOW6432Node`), and the 64-bit view of the plain path. Passing
/// `KEY_WOW64_32KEY` *and* naming `WOW6432Node` in the path would ask the
/// redirector to redirect an already-redirected path, so the two are never
/// combined.
#[cfg(windows)]
mod sijill {
    use std::path::PathBuf;

    use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ,
        REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
        RegQueryValueExW,
    };
    use windows::core::{PCWSTR, PWSTR};

    use super::TathbeetMustaqill;

    /// The three places the standalone installers register a game.
    const MASARAT: [(&str, REG_SAM_FLAGS); 3] = [
        (r"SOFTWARE\WOW6432Node\GOG.com\Games", KEY_WOW64_64KEY),
        (r"SOFTWARE\GOG.com\Games", KEY_WOW64_32KEY),
        (r"SOFTWARE\GOG.com\Games", KEY_WOW64_64KEY),
    ];

    /// Longest registry string this reader will accept, in bytes. Registry
    /// values can be far larger; a path is not.
    const HADD_QEEMA: u32 = 64 * 1024;

    /// Registry key names are limited to 255 characters by the API itself.
    const HADD_ISM: usize = 512;

    /// A hard stop on enumeration, so a hostile or corrupt hive cannot spin.
    const HADD_MAFATIH: u32 = 4096;

    /// An open key that closes itself.
    struct Miftah(HKEY);

    impl Drop for Miftah {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful RegOpenKeyExW, is not
            // copied anywhere else, and is closed exactly once, here.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    /// A null-terminated wide string, alive for as long as the caller holds it.
    fn wide(nass: &str) -> Vec<u16> {
        nass.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Opens a key under `HKEY_LOCAL_MACHINE`.
    fn fath(masar: &str, ruya: REG_SAM_FLAGS) -> Option<Miftah> {
        let masar_w = wide(masar);
        let mut miftah = HKEY::default();
        // SAFETY: `masar_w` is a live, null-terminated wide string for the whole
        // call, and `miftah` is a live, correctly typed out-parameter.
        let natija = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(masar_w.as_ptr()),
                None,
                KEY_READ | ruya,
                &raw mut miftah,
            )
        };
        (natija == ERROR_SUCCESS).then_some(Miftah(miftah))
    }

    /// Opens a subkey of an open key.
    fn fath_farii(walid: &Miftah, ism: &str, ruya: REG_SAM_FLAGS) -> Option<Miftah> {
        let ism_w = wide(ism);
        let mut miftah = HKEY::default();
        // SAFETY: `ism_w` is a live, null-terminated wide string for the whole
        // call, `walid.0` is an open key, and `miftah` is a live out-parameter.
        let natija = unsafe {
            RegOpenKeyExW(walid.0, PCWSTR(ism_w.as_ptr()), None, KEY_READ | ruya, &raw mut miftah)
        };
        (natija == ERROR_SUCCESS).then_some(Miftah(miftah))
    }

    /// Every immediate subkey name.
    fn mafatih_farya(walid: &Miftah) -> Vec<String> {
        let mut asmaa = Vec::new();
        let mut buffer = vec![0u16; HADD_ISM];
        let mut fahras: u32 = 0;
        while fahras < HADD_MAFATIH {
            let Ok(mut tul) = u32::try_from(buffer.len()) else {
                break;
            };
            // SAFETY: `buffer` holds `tul` u16 slots for the whole call, `tul`
            // is a live out-parameter, and every optional argument the call does
            // not need is passed as None rather than a dangling pointer.
            let natija = unsafe {
                RegEnumKeyExW(
                    walid.0,
                    fahras,
                    Some(PWSTR(buffer.as_mut_ptr())),
                    &raw mut tul,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if natija == ERROR_NO_MORE_ITEMS {
                break;
            }
            if natija == ERROR_MORE_DATA {
                buffer = vec![0u16; buffer.len().saturating_mul(2).min(1 << 16)];
                continue;
            }
            if natija != ERROR_SUCCESS {
                break;
            }
            let adad = usize::try_from(tul).unwrap_or(0).min(buffer.len());
            if let Some(harfiyat) = buffer.get(..adad) {
                let ism = String::from_utf16_lossy(harfiyat);
                let ism = ism.trim_end_matches('\0').trim().to_owned();
                if !ism.is_empty() {
                    asmaa.push(ism);
                }
            }
            fahras = fahras.saturating_add(1);
        }
        asmaa
    }

    /// A string value, or `None` when it is absent, too large, or not a string.
    fn qeema_nass(miftah: &Miftah, ism: &str) -> Option<String> {
        let ism_w = wide(ism);
        let mut naw = REG_VALUE_TYPE::default();
        let mut hajm: u32 = 0;
        // SAFETY: `ism_w` is a live, null-terminated wide string; `naw` and
        // `hajm` are live out-parameters; no data buffer is requested by this
        // first call, which is how the required size is learned.
        let natija = unsafe {
            RegQueryValueExW(
                miftah.0,
                PCWSTR(ism_w.as_ptr()),
                None,
                Some(&raw mut naw),
                None,
                Some(&raw mut hajm),
            )
        };
        if natija != ERROR_SUCCESS && natija != ERROR_MORE_DATA {
            return None;
        }
        if naw != REG_SZ && naw != REG_EXPAND_SZ {
            return None;
        }
        if hajm == 0 || hajm > HADD_QEEMA {
            return None;
        }

        let adad = usize::try_from(hajm).ok()?.div_ceil(2);
        let mut buffer = vec![0u16; adad];
        let mut hajm_mutah = hajm;
        // SAFETY: `buffer` is `adad` u16 slots, which is at least `hajm` bytes,
        // and `hajm_mutah` tells the call exactly that. The pointer is cast to
        // u8 because the API counts bytes; the allocation's alignment is that of
        // u16, which is stricter, so the write is in bounds and aligned.
        let natija = unsafe {
            RegQueryValueExW(
                miftah.0,
                PCWSTR(ism_w.as_ptr()),
                None,
                None,
                Some(buffer.as_mut_ptr().cast::<u8>()),
                Some(&raw mut hajm_mutah),
            )
        };
        if natija != ERROR_SUCCESS {
            return None;
        }
        let adad_harfiyat = usize::try_from(hajm_mutah).ok()?.div_euclid(2).min(buffer.len());
        let harfiyat = buffer.get(..adad_harfiyat)?;
        let tul = harfiyat.iter().position(|harf| *harf == 0).unwrap_or(harfiyat.len());
        let nass = String::from_utf16_lossy(harfiyat.get(..tul)?).trim().to_owned();
        (!nass.is_empty()).then_some(nass)
    }

    /// The first of several value names that is present.
    fn awwal_qeema(miftah: &Miftah, asmaa: &[&str]) -> Option<String> {
        asmaa.iter().find_map(|ism| qeema_nass(miftah, ism))
    }

    /// Every standalone GOG install the registry knows about.
    pub(super) fn alaab_gog() -> Vec<TathbeetMustaqill> {
        let mut tathbitat: Vec<TathbeetMustaqill> = Vec::new();
        let mut maruf: Vec<u64> = Vec::new();

        for (masar, ruya) in MASARAT {
            let Some(walid) = fath(masar, ruya) else {
                continue;
            };
            for ism in mafatih_farya(&walid) {
                let Some(miftah) = fath_farii(&walid, &ism, ruya) else {
                    continue;
                };
                let muarrif = ism
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .or_else(|| {
                        awwal_qeema(&miftah, &["gameID", "gameId", "productID"])
                            .and_then(|qeema| qeema.trim().parse::<u64>().ok())
                    });
                let Some(muarrif) = muarrif else {
                    continue;
                };
                if maruf.contains(&muarrif) {
                    continue;
                }
                let Some(masar_luba) = awwal_qeema(&miftah, &["path", "PATH", "installPath"])
                else {
                    continue;
                };
                maruf.push(muarrif);
                tathbitat.push(TathbeetMustaqill {
                    muarrif,
                    jidhr: PathBuf::from(masar_luba),
                    ism: awwal_qeema(&miftah, &["gameName", "startMenu"]),
                    isdar: awwal_qeema(&miftah, &["ver", "version"]),
                    tanfidhi: awwal_qeema(&miftah, &["exe", "exeFile"]),
                });
            }
        }
        tathbitat
    }
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
    fn jidhr_galaxy_min_bayanat_al_barnamij_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let bayanat = masrah.path().join("ProgramData");
        let jidhr = bayanat.join("GOG.com").join("Galaxy");
        fs::create_dir_all(jidhr.join("storage"))?;

        // Nothing sets `%PROGRAMDATA%` — since edition 2024 it cannot be set
        // from a test — so this passes only if the root really is read off the
        // context.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_barnamij = Some(bayanat);
        assert_eq!(MatjarGog::jadeed().mawqi(&siyaq), Some(jidhr));
        Ok(())
    }

    #[test]
    fn bila_bayanat_barnamij_la_jidhr() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // The old resolver answered `C:\ProgramData\GOG.com\Galaxy` whatever
        // machine asked; a context with no machine-wide data folder now says so.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        assert_eq!(MatjarGog::jadeed().mawqi(&siyaq), None);
        Ok(())
    }

    /// Galaxy's folder is present and its database is not. `mawqi` calls that
    /// installed, so `ifhas` must not call it absent — the shape that let the
    /// absence sweep run over a launcher nobody had read.
    #[test]
    fn jidhr_bila_qaida_naqis_la_ghayr_muthabbat() -> NatijatIkhtibar {
        use crate::fahs::HalatFahsMatjar;

        let masrah = tempfile::tempdir()?;
        let bayanat = masrah.path().join("ProgramData");
        let jidhr = bayanat.join("GOG.com").join("Galaxy");
        fs::create_dir_all(jidhr.join("storage"))?;
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_barnamij = Some(bayanat);

        let matjar = MatjarGog::jadeed();
        assert_eq!(matjar.mawqi(&siyaq).as_deref(), Some(jidhr.as_path()));
        let natija = matjar.ifhas(&siyaq)?;
        assert_ne!(natija.hala(), HalatFahsMatjar::GhayrMuthabbat);
        assert_eq!(natija.jidhr_matjar.as_deref(), Some(jidhr.as_path()));
        // Every host but a Windows machine with standalone GOG installs in its
        // registry reaches the exact case; that one still passes the two above.
        if tathbitat_mustaqilla().is_empty() {
            assert_eq!(natija.hala(), HalatFahsMatjar::Naqisa);
            assert!(natija.tanbihat.iter().any(TanbihFahs::yukhfi_alaab));
        }
        Ok(())
    }

    #[test]
    fn jidhr_mac_yabqa_taht_al_manzil() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr = masrah
            .path()
            .join("Library")
            .join("Application Support")
            .join("GOG.com")
            .join("Galaxy");
        fs::create_dir_all(&jidhr)?;

        // macOS keeps Galaxy under the home directory, so it resolves on a
        // context that has no Windows folders at all.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Mac, masrah.path());
        assert_eq!(MatjarGog::jadeed().mawqi(&siyaq), Some(jidhr));
        Ok(())
    }
}
