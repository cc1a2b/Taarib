//! إتش — itch.io, read out of the itch app's `butler.db`.
//!
//! The itch app keeps everything it knows in one `SQLite` database written by
//! `butler`, its download and install engine:
//!
//! | platform | path |
//! | --- | --- |
//! | Windows | `%APPDATA%\itch\db\butler.db` |
//! | Linux | `~/.config/itch/db/butler.db` |
//! | macOS | `~/Library/Application Support/itch/db/butler.db` |
//!
//! Three tables carry what discovery needs. `games` is the catalogue metadata —
//! title, classification, cover art. `caves` is the installations: one row per
//! installed copy, naming the game it is a copy of, the folder it went into,
//! and butler's own verdict about which file inside it is the launchable one.
//! `install_locations` maps a cave's location id to a real directory. A cave's
//! install root is `install_locations.path` joined with
//! `caves.install_folder_name`, and neither half is useful alone.
//!
//! ## The database is opened read-only, and that is not a formality
//!
//! `butler.db` belongs to a program that may be running right now. Opening
//! another product's live `SQLite` database read-write is one of the reliable
//! ways to destroy it:
//!
//! - A read-write connection can be handed a **hot journal**. If the itch app
//!   died mid-transaction, the next writer to open the database is obliged to
//!   roll that journal back before doing anything else — so a scanner that
//!   opened read-write would silently rewrite a database it has no business
//!   modifying, using recovery logic whose correctness depends on matching page
//!   sizes and journal modes it never checked.
//! - In WAL mode a read-write connection creates and may **checkpoint** the
//!   `-wal` file, truncating a log the running app is still appending to.
//! - A read-write open takes locks that make the itch app's own writes fail,
//!   which the app surfaces to its user as their library being broken.
//!
//! None of that is hypothetical: it is the normal consequence of two writers on
//! one `SQLite` file, and it is why this adapter opens with
//! `SQLITE_OPEN_READ_ONLY` over a `file:` URI and never anything else.
//!
//! Two modes are tried, in order:
//!
//! 1. `?mode=ro` — an ordinary read-only connection. It takes shared read locks
//!    only, it reads committed data out of the `-wal` file, and it is correct
//!    while the app is running. This is what almost every machine gets.
//! 2. `?immutable=1` — used only when the first fails, which happens when the
//!    database directory is not writable and `SQLite` therefore cannot create the
//!    shared-memory file a WAL reader needs. `immutable=1` promises `SQLite` the
//!    file cannot change, so it takes no locks and reads no `-wal` at all.
//!    That is safe for the itch app but can return data a few writes stale, so
//!    it is recorded as a warning rather than used silently.
//!
//! ## Every query is defensive
//!
//! butler's schema moves. Columns have been added, renamed and dropped across
//! itch app versions, and a scanner written against one version's exact column
//! list breaks on the next. So nothing here names columns in SQL: each table is
//! read with `SELECT *`, every value is fetched **by name** through a lookup
//! that returns nothing when the column is not in that build's schema, and a
//! table that is missing entirely becomes a warning while the other tables are
//! still read. An itch installation that yields games with no artwork is a
//! better outcome than one that yields nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rusqlite::{Connection, OpenFlags};
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
const MUARRIF: &str = "itch";

/// The itch app runs on all three desktop systems.
const MANASSAT: [NizamTashghil; 3] = [
    NizamTashghil::Windows,
    NizamTashghil::Linux,
    NizamTashghil::Mac,
];

/// The database, relative to the app's configuration root.
const MASAR_QAIDA: [&str; 2] = ["db", "butler.db"];

/// The Flatpak application id, for the Linux installations that use it.
const HAWIYAT_FLATPAK: &str = "io.itch.itch";

/// itch.io classifications that are not games.
///
/// itch hosts far more than games, and a library that shows a user's font packs
/// and Twine tools alongside their games is a library they have to filter every
/// time they open it.
const TASNIFAT_GHAYR_LUBA: &[&str] = &[
    "assets",
    "book",
    "comic",
    "other",
    "physical_game",
    "soundtrack",
    "tool",
];

/// itch.io.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarItch;

impl MatjarItch {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Every configuration root worth looking in, most likely first.
    fn judhur_muhtamala(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur = Vec::new();
        match siyaq.nizam {
            NizamTashghil::Windows => {
                judhur.extend(
                    siyaq
                        .bayanat_mutajawwila
                        .as_ref()
                        .map(|bayanat| bayanat.join("itch")),
                );
            },
            NizamTashghil::Mac => {
                judhur.push(
                    siyaq
                        .manzil
                        .join("Library")
                        .join("Application Support")
                        .join("itch"),
                );
            },
            NizamTashghil::Linux => {
                judhur.push(siyaq.manzil.join(".config").join("itch"));
                if siyaq.yashmal_hawiyat {
                    judhur.push(
                        siyaq
                            .manzil
                            .join(".var")
                            .join("app")
                            .join(HAWIYAT_FLATPAK)
                            .join("config")
                            .join("itch"),
                    );
                }
            },
        }
        judhur
    }
}

impl Matjar for MatjarItch {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "إتش"
    }

    fn ism_injilizi(&self) -> &'static str {
        "itch.io"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if let Some(tajawuz) = siyaq.manassat.itch.as_ref() {
            return Some(tajawuz.clone());
        }
        Self::judhur_muhtamala(siyaq)
            .into_iter()
            .find(|jidhr| qaida_fih(jidhr).is_file())
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when the user configured an
    /// itch root that is not there, and [`KhataKashf::TaadhurQiraatFahras`]
    /// when `butler.db` exists but `SQLite` will not open it read-only at all.
    /// A missing table, a missing column, a cave whose folder has been deleted
    /// and a game whose row is gone are each a [`TanbihFahs`], and the games
    /// that did read still arrive.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        if let Some(tajawuz) = siyaq.manassat.itch.as_ref()
            && !tajawuz.is_dir()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: tajawuz.clone(),
            }
            .into());
        }

        let Some(jidhr) = self.mawqi(siyaq) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };
        let qaida = qaida_fih(&jidhr);
        if !qaida.is_file() {
            // The root is here — the user configured it, or a probe found the
            // database a moment ago — and the catalogue is not. Installed and
            // unreadable, never "not installed".
            return Ok(NatijatMatjar::naqisa(
                MUARRIF,
                Some(jidhr),
                qaida.display().to_string(),
                "the itch app's catalogue (db/butler.db) is not under this root, so no itch.io \
                 game could be listed; if the root was configured by hand, point it at the \
                 folder that holds the db directory",
            ));
        }

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, Some(jidhr));

        let (silat, thabita) = iftah_lil_qiraa(&qaida)?;
        if thabita {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                qaida.display().to_string(),
                "butler.db could not be opened for ordinary read-only access, so it was read in \
                 immutable mode instead. That is safe for a running itch app, but a game \
                 installed in the last few seconds may be missing until the next scan.",
            ));
        }

        jama_alaab(&silat, siyaq.nizam, &mut natija);

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        self.mawqi(siyaq)
            .map(|jidhr| jidhr.join(MASAR_QAIDA.first().copied().unwrap_or("db")))
            .filter(|mujallad| mujallad.is_dir())
            .map(|mujallad| vec![mujallad])
            .unwrap_or_default()
    }
}

/// The database file under an itch configuration root.
fn qaida_fih(jidhr: &Path) -> PathBuf {
    MASAR_QAIDA
        .iter()
        .fold(jidhr.to_path_buf(), |masar, juz| masar.join(juz))
}

// ---------------------------------------------------------------------------
// opening butler.db without touching it
// ---------------------------------------------------------------------------

/// How long a read will wait for a lock the running itch app is holding.
///
/// A writer holds the write lock for milliseconds at a time, so half a second
/// is generous; waiting longer would mean a library screen that stalls because
/// somebody happened to be downloading a game while it opened.
const MUHLAT_INTIZAR: std::time::Duration = std::time::Duration::from_millis(500);

/// Opens `butler.db` for reading and nothing else.
///
/// The boolean says whether the fallback immutable mode had to be used, which
/// the caller turns into a warning: immutable reads bypass the write-ahead log,
/// so they can be marginally stale.
///
/// # Errors
///
/// Returns [`KhataKashf::TaadhurQiraatFahras`] when neither mode can open the
/// file, which means the database is not a database, is on a filesystem that
/// refuses the read, or has been deleted between the existence check and here.
fn iftah_lil_qiraa(masar: &Path) -> Natija<(Connection, bool)> {
    // READ_ONLY is the guarantee; URI is what allows the query parameters that
    // select the two modes; NO_MUTEX is correct because the connection never
    // leaves the thread that made it.
    let aalam = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let asas = uri_min_masar(masar);

    if let Ok(sila) = jarrib_fath(&format!("{asas}?mode=ro"), aalam) {
        return Ok((sila, false));
    }
    match jarrib_fath(&format!("{asas}?immutable=1"), aalam) {
        Ok(sila) => Ok((sila, true)),
        Err(sabab) => Err(KhataKashf::TaadhurQiraatFahras {
            matjar: MUARRIF,
            masar: masar.to_path_buf(),
            sabab: std::io::Error::other(sabab.to_string()),
        }
        .into()),
    }
}

/// Opens one URI and proves the connection actually works.
///
/// `SQLite` opens lazily: `open_with_flags` succeeds on a path it has not yet
/// touched, and the real failure — the missing shared-memory file that makes a
/// WAL database unreadable on a read-only directory — surfaces on the first
/// statement. Reading the schema here is what makes the fallback in
/// [`iftah_lil_qiraa`] fire at the right moment instead of one query later.
fn jarrib_fath(uri: &str, aalam: OpenFlags) -> Result<Connection, rusqlite::Error> {
    let sila = Connection::open_with_flags(uri, aalam)?;
    sila.busy_timeout(MUHLAT_INTIZAR)?;
    let _: i64 = sila.query_row("SELECT count(*) FROM sqlite_master", [], |saf| saf.get(0))?;
    Ok(sila)
}

/// Builds a `SQLite` `file:` URI out of a path.
///
/// `SQLite` reads its own URIs, so anything that could be mistaken for a query
/// string or a fragment has to be percent-encoded first — a game library under
/// a folder with a `?` in its name would otherwise silently become a database
/// path plus a parameter list. Separators are normalized to forward slashes,
/// which `SQLite` accepts on Windows, and a Windows path gains the third slash
/// that turns `C:/…` into a valid absolute `file:` URI.
fn uri_min_masar(masar: &Path) -> String {
    use std::fmt::Write as _;

    let kham = masar.to_string_lossy().replace('\\', "/");
    let mut murammaz = String::with_capacity(kham.len().saturating_add(16));
    for harf in kham.chars() {
        if harf.is_ascii_alphanumeric() || matches!(harf, '-' | '.' | '_' | '~' | '/' | ':') {
            murammaz.push(harf);
        } else {
            let mut wahdat = [0u8; 4];
            for bayt in harf.encode_utf8(&mut wahdat).as_bytes() {
                let _ = write!(murammaz, "%{bayt:02X}");
            }
        }
    }
    if murammaz.starts_with('/') {
        format!("file://{murammaz}")
    } else {
        format!("file:///{murammaz}")
    }
}

// ---------------------------------------------------------------------------
// reading rows without assuming a schema
// ---------------------------------------------------------------------------

/// One column value, in whichever of `SQLite`'s storage classes it arrived as.
#[derive(Debug, Clone, PartialEq)]
enum Qeema {
    /// `TEXT`.
    Nass(String),
    /// `INTEGER`.
    Raqm(i64),
    /// `REAL`.
    Ashari(f64),
    /// `NULL`, and also any value that could not be fetched.
    Faragh,
}

impl Qeema {
    fn min(qeema: rusqlite::types::ValueRef<'_>) -> Self {
        match qeema {
            rusqlite::types::ValueRef::Null => Self::Faragh,
            rusqlite::types::ValueRef::Integer(raqm) => Self::Raqm(raqm),
            rusqlite::types::ValueRef::Real(ashari) => Self::Ashari(ashari),
            rusqlite::types::ValueRef::Text(bayt) | rusqlite::types::ValueRef::Blob(bayt) => {
                // butler stores its JSON verdict as a blob in some builds and
                // as text in others; both are UTF-8, and a blob that is not is
                // not a value this adapter reads.
                Self::Nass(String::from_utf8_lossy(bayt).into_owned())
            },
        }
    }
}

/// One row, addressed by column name rather than by position.
#[derive(Debug, Clone, Default)]
struct Saf {
    qiyam: BTreeMap<String, Qeema>,
}

impl Saf {
    /// A text value, or nothing when the column is absent, null or empty.
    fn nass(&self, ism: &str) -> Option<String> {
        match self.qiyam.get(ism) {
            Some(Qeema::Nass(nass)) if !nass.trim().is_empty() => Some(nass.trim().to_owned()),
            Some(Qeema::Raqm(raqm)) => Some(raqm.to_string()),
            _ => None,
        }
    }

    /// An integer value, or nothing when the column is absent or not one.
    ///
    /// A `REAL` is deliberately not accepted: butler stores no identifier or
    /// size as a float, so a float in one of these columns means the schema has
    /// moved somewhere this reader should not follow it, and narrowing a
    /// floating-point value into a game identifier would be inventing data.
    fn raqm(&self, ism: &str) -> Option<i64> {
        match self.qiyam.get(ism) {
            Some(Qeema::Raqm(raqm)) => Some(*raqm),
            Some(Qeema::Nass(nass)) => nass.trim().parse().ok(),
            _ => None,
        }
    }

    /// A time value, normalized to RFC 3339.
    ///
    /// butler has written these as ISO text and as unix integers depending on
    /// its version, so both are accepted; anything that resolves to neither is
    /// dropped rather than passed on, because a timestamp nobody can parse is
    /// worse in a record than an absent one.
    fn waqt(&self, ism: &str) -> Option<String> {
        match self.qiyam.get(ism)? {
            Qeema::Nass(nass) => waqt_min_nass(nass),
            Qeema::Raqm(raqm) => waqt_min_thawani(*raqm),
            _ => None,
        }
    }
}

/// Normalizes a textual timestamp.
fn waqt_min_nass(nass: &str) -> Option<String> {
    let munaqqa = nass.trim();
    if munaqqa.is_empty() {
        return None;
    }
    if let Ok(waqt) = munaqqa.parse::<jiff::Timestamp>() {
        return Some(waqt.to_string());
    }
    // `2019-05-05 12:00:00+00:00` is the shape Go's SQLite driver writes when
    // it is not asked for RFC 3339 explicitly.
    munaqqa
        .replacen(' ', "T", 1)
        .parse::<jiff::Timestamp>()
        .ok()
        .map(|waqt| waqt.to_string())
}

/// Normalizes a numeric timestamp, in seconds or in milliseconds.
fn waqt_min_thawani(raqm: i64) -> Option<String> {
    // Anything past this magnitude cannot be a plausible number of seconds —
    // it is the year 33658 — so it is milliseconds.
    const HADD_MILLI: i64 = 1_000_000_000_000;
    let waqt = if raqm.abs() >= HADD_MILLI {
        jiff::Timestamp::from_millisecond(raqm)
    } else {
        jiff::Timestamp::from_second(raqm)
    };
    waqt.ok().map(|waqt| waqt.to_string())
}

/// Reads a whole table as name-addressed rows.
///
/// The table name is interpolated into the statement, which is safe here and
/// only here: it comes from a `const` in this module and never from a file, a
/// database value or anything the user typed. It is quoted regardless, so a
/// future name containing a keyword still parses.
///
/// # Errors
///
/// Returns the `SQLite` failure unchanged, which the caller turns into a warning
/// naming the table — a table this build of the itch app does not have is the
/// expected reason.
fn asfuf(sila: &Connection, jadwal: &str) -> Result<Vec<Saf>, rusqlite::Error> {
    let mut bayan = sila.prepare(&format!("SELECT * FROM \"{jadwal}\""))?;
    let asmaa: Vec<String> = bayan
        .column_names()
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect();
    let mut nataij = bayan.query([])?;
    let mut khuruj = Vec::new();
    while let Some(saf) = nataij.next()? {
        let mut qiyam = BTreeMap::new();
        for (fahras, ism) in asmaa.iter().enumerate() {
            let qeema = saf.get_ref(fahras).map_or(Qeema::Faragh, Qeema::min);
            let _ = qiyam.insert(ism.clone(), qeema);
        }
        khuruj.push(Saf { qiyam });
    }
    Ok(khuruj)
}

// ---------------------------------------------------------------------------
// caves + games + install_locations → games
// ---------------------------------------------------------------------------

/// Reads the three tables and joins them in memory.
///
/// The join is done here rather than in SQL for one reason: a `JOIN` fails
/// outright when either side is missing a column it names, and this adapter has
/// to survive exactly that. Three independent `SELECT *` reads mean a build of
/// the itch app that dropped `install_locations` still yields every cave whose
/// verdict records its own base path.
fn jama_alaab(sila: &Connection, nizam: NizamTashghil, natija: &mut NatijatMatjar) {
    let mawaqi: BTreeMap<String, PathBuf> = match asfuf(sila, "install_locations") {
        Ok(asfuf) => asfuf
            .iter()
            .filter_map(|saf| Some((saf.nass("id")?, PathBuf::from(saf.nass("path")?))))
            .collect(),
        Err(sabab) => {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                "butler.db: install_locations",
                format!(
                    "cannot read the install locations table ({sabab}); games are still listed \
                     when butler's own verdict records where it put them"
                ),
            ));
            BTreeMap::new()
        },
    };

    let bitaqat: BTreeMap<i64, Saf> = match asfuf(sila, "games") {
        Ok(asfuf) => asfuf
            .into_iter()
            .filter_map(|saf| Some((saf.raqm("id")?, saf)))
            .collect(),
        Err(sabab) => {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                "butler.db: games",
                format!(
                    "cannot read the games table ({sabab}); installed games are still listed, \
                     under their folder names and without cover art"
                ),
            ));
            BTreeMap::new()
        },
    };

    let kuhuf = match asfuf(sila, "caves") {
        Ok(kuhuf) => kuhuf,
        Err(sabab) => {
            natija.tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                "butler.db: caves",
                format!(
                    "cannot read the installations table ({sabab}); no itch.io game can be \
                     located without it, so this launcher contributes nothing to this scan"
                ),
            ));
            return;
        },
    };

    for kahf in &kuhuf {
        match luba_min_kahf(kahf, nizam, &mawaqi, &bitaqat) {
            Ok(luba) => natija.alaab.push(luba),
            Err(tanbih) => natija.tanbihat.push(tanbih),
        }
    }
}

/// Turns one cave into a discovered game.
fn luba_min_kahf(
    kahf: &Saf,
    nizam: NizamTashghil,
    mawaqi: &BTreeMap<String, PathBuf>,
    bitaqat: &BTreeMap<i64, Saf>,
) -> Result<LubaMuktashafa, TanbihFahs> {
    let muarrif_kahf = kahf.nass("id").unwrap_or_else(|| "unnamed cave".to_owned());
    let hukm = hukm_butler(kahf);

    let jidhr = jidhr_kahf(kahf, mawaqi, hukm.as_ref()).ok_or_else(|| {
        TanbihFahs::jadeed(
            MUARRIF,
            format!("butler.db: cave {muarrif_kahf}"),
            "this installation records neither a base path of its own nor an install location \
             Taarib could resolve, so there is nowhere to look for the game",
        )
    })?;

    if !jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            "the itch app has this game installed here and the folder is gone; it was most \
             likely deleted outside the app, which the app itself has not noticed yet",
        ));
    }

    let muarrif_luba = kahf
        .raqm("game_id")
        .or_else(|| kahf.raqm("external_game_id"))
        .ok_or_else(|| {
            TanbihFahs::jadeed(
                MUARRIF,
                format!("butler.db: cave {muarrif_kahf}"),
                "this installation names no game, so it cannot be matched against anything in \
                 the patch registry",
            )
        })?;

    let bitaqa = bitaqat.get(&muarrif_luba);
    let ism = bitaqa
        .and_then(|saf| saf.nass("title"))
        .or_else(|| kahf.nass("install_folder_name"))
        .or_else(|| {
            jidhr
                .file_name()
                .map(|ism| ism.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| format!("itch.io game {muarrif_luba}"));

    let mut simat = Vec::new();
    if let Some(tasnif) = bitaqa.and_then(|saf| saf.nass("classification"))
        && TASNIFAT_GHAYR_LUBA
            .iter()
            .any(|ghayr| ghayr.eq_ignore_ascii_case(&tasnif))
    {
        simat.push(SimatLuba::LaysatLuba(tasnif));
    }
    if let Some(naw) = hukm.as_ref().and_then(|hukm| hukm.naw.as_deref())
        && naw.eq_ignore_ascii_case("windows")
        && nizam != NizamTashghil::Windows
    {
        simat.push(SimatLuba::TabaqatTawafuq(
            "itch records this as a Windows build, so it runs through Wine on this machine"
                .to_owned(),
        ));
    }

    let tanfidhi = hukm
        .as_ref()
        .and_then(|hukm| hukm.murashah.as_deref())
        .and_then(|nisbi| dakhil(&jidhr, &nisbi.replace('\\', std::path::MAIN_SEPARATOR_STR)).ok())
        .filter(|masar| masar.is_file());

    let hajm = kahf
        .raqm("installed_size")
        .and_then(|raqm| u64::try_from(raqm).ok())
        .unwrap_or(0);

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Itch(muarrif_luba),
        hala_matjar: None,
        ism,
        jidhr,
        tanfidhi,
        hajm,
        bina_manassa: kahf
            .nass("build_id")
            .filter(|raqm| raqm != "0")
            .or_else(|| kahf.nass("upload_id")),
        akhir_tahdith: kahf.waqt("installed_at"),
        akhir_laab: kahf.waqt("last_touched_at"),
        beea: BeeatTawafuq::Asli,
        suwar: MasadirSuwar {
            ghilaf: bitaqa
                .and_then(|saf| {
                    saf.nass("cover_url")
                        .or_else(|| saf.nass("still_cover_url"))
                })
                .map(MasdarSura::Rabt),
            batl: None,
            shiar: None,
        },
        khiyarat_tashghil: None,
        // A cave exists only once butler has finished writing the files; a
        // download in progress lives in the `downloads` table instead.
        muktamila: true,
        simat,
    })
}

/// The install root of a cave.
///
/// butler's verdict is preferred because it is butler's own record of where the
/// files actually went, written after the install finished. The location table
/// is the reconstruction for caves whose verdict is absent or unparseable.
fn jidhr_kahf(
    kahf: &Saf,
    mawaqi: &BTreeMap<String, PathBuf>,
    hukm: Option<&HukmButler>,
) -> Option<PathBuf> {
    if let Some(asas) = hukm.and_then(|hukm| hukm.asas.as_ref())
        && asas.is_absolute()
    {
        return Some(asas.clone());
    }
    let mawqi = mawaqi.get(&kahf.nass("install_location_id")?)?;
    let mujallad = kahf.nass("install_folder_name")?;
    dakhil(mawqi, &mujallad).ok()
}

/// What butler concluded about an installed folder.
///
/// `caves.verdict` is a JSON document butler writes after configuring an
/// install. It is read for three things and nothing else: the base path, the
/// most likely launch candidate, and that candidate's platform flavour.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HukmButler {
    /// `basePath` — where butler put the files.
    asas: Option<PathBuf>,
    /// The first launch candidate's `path`, relative to the base.
    murashah: Option<String>,
    /// The first launch candidate's `flavor`: `windows`, `linux`, `macos`,
    /// `html`, `love`, `jar`.
    naw: Option<String>,
}

/// Parses a cave's verdict.
///
/// A verdict that will not parse is not a warning: butler leaves it empty for
/// caves it never configured, and every field it would have supplied has a
/// fallback. Reporting it would fill Diagnostics with noise about installations
/// that are working perfectly.
fn hukm_butler(kahf: &Saf) -> Option<HukmButler> {
    let kham = kahf.nass("verdict")?;
    let qeema: serde_json::Value = serde_json::from_str(&kham).ok()?;
    let murashahun = qeema
        .get("candidates")
        .and_then(serde_json::Value::as_array);
    let awwal = murashahun.and_then(|qaima| qaima.first());
    Some(HukmButler {
        asas: qeema
            .get("basePath")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from),
        murashah: awwal
            .and_then(|murashah| murashah.get("path"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        naw: awwal
            .and_then(|murashah| murashah.get("flavor"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    })
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
    fn jidhr_windows_min_al_bayanat_al_mutajawwila_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let mutajawwila = masrah.path().join("Roaming");
        let jidhr = mutajawwila.join("itch");
        fs::create_dir_all(jidhr.join("db"))?;
        fs::write(qaida_fih(&jidhr), [])?;

        // `%APPDATA%` is untouched — since edition 2024 a test cannot set it —
        // so this can only pass if the candidate came off the context.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_mutajawwila = Some(mutajawwila);
        assert_eq!(MatjarItch::jadeed().mawqi(&siyaq), Some(jidhr));
        Ok(())
    }

    #[test]
    fn bila_bayanat_mutajawwila_la_murashah_ala_windows() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        assert!(MatjarItch::judhur_muhtamala(&siyaq).is_empty());
        Ok(())
    }

    /// A configured root that exists and holds no `db/butler.db`. The user said
    /// the itch app is here and `mawqi` takes them at their word; `ifhas` must
    /// answer "installed, unreadable", not "not installed".
    #[test]
    fn tajawuz_bila_qaida_naqis_la_ghayr_muthabbat() -> NatijatIkhtibar {
        use crate::fahs::HalatFahsMatjar;

        let masrah = tempfile::tempdir()?;
        let tajawuz = masrah.path().join("itch-farigh");
        fs::create_dir_all(&tajawuz)?;
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.manassat.itch = Some(tajawuz.clone());

        let matjar = MatjarItch::jadeed();
        assert_eq!(matjar.mawqi(&siyaq).as_deref(), Some(tajawuz.as_path()));
        let natija = matjar.ifhas(&siyaq)?;
        assert_eq!(natija.hala(), HalatFahsMatjar::Naqisa);
        assert_eq!(natija.jidhr_matjar.as_deref(), Some(tajawuz.as_path()));
        Ok(())
    }
}
