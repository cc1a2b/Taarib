//! لوتريس — Lutris, whose catalogue is a database and whose games mostly belong
//! to somebody else's store.
//!
//! Lutris is the oldest and the least uniform of the Linux game managers. It
//! runs Windows games under Wine, Linux games natively, console titles through
//! four dozen emulators, and store titles through installer scripts written by
//! whoever felt like writing one. Nothing about an entry is implied by anything
//! else about it, so this adapter reads every fact separately and refuses to
//! infer the ones that are absent.
//!
//! ## What is read
//!
//! | file | what it carries |
//! | --- | --- |
//! | `$XDG_DATA_HOME/lutris/pga.db` | the whole catalogue: one row per game in the `games` table |
//! | `$XDG_CONFIG_HOME/lutris/games/<configpath>.yml` | that game's executable, its Wine prefix, its Wine build, its launch arguments |
//! | `$XDG_DATA_HOME/lutris/coverart/<slug>.jpg` | the vertical cover Lutris already downloaded |
//! | `$XDG_DATA_HOME/lutris/banners/<slug>.jpg` | the wide banner |
//! | `$XDG_DATA_HOME/icons/hicolor/128x128/apps/lutris_<slug>.png` | the icon |
//!
//! The Flatpak build keeps all of it under `~/.var/app/net.lutris.Lutris/`, and
//! the data root, the configuration root and the cache root are resolved
//! **together** rather than independently: a native `pga.db` read against a
//! Flatpak `games/*.yml` would hand every game a prefix belonging to a different
//! installation, and every one of those paths would resolve, and every patch
//! would land somewhere the game never looks.
//!
//! ## `pga.db` belongs to a program that may be running
//!
//! It is opened `SQLITE_OPEN_READ_ONLY` over a `file:` URI and never any other
//! way. A read-write connection to another product's live database can be handed
//! a hot journal to roll back, can checkpoint and truncate a `-wal` the running
//! application is still appending to, and takes locks that make Lutris's own
//! writes fail in front of its user. None of that is hypothetical; it is the
//! ordinary consequence of two writers on one `SQLite` file.
//!
//! A **busy** database is not a failure. Lutris holds its write lock for
//! milliseconds at a time, so the read waits briefly and then, if the lock is
//! still held or the directory is not writable enough for a WAL reader's
//! shared-memory file, falls back to `immutable=1` — which takes no locks and
//! reads no `-wal`, at the cost of being a few writes stale. That staleness is
//! recorded as a [`TanbihFahs`] rather than hidden, because "the game I added
//! ten seconds ago is missing" deserves an explanation on the diagnostics screen.
//!
//! ## The identity mapping is the point of this adapter
//!
//! A Lutris row's `service` column names the store the entry came from and
//! `service_id` is that store's own identifier for it. Where both are present
//! this adapter builds the store's real source and puts it inside
//! [`MasdarLutris::asl`], so that [`MasdarLuba::aila`] and
//! [`MasdarLuba::muarrif`] answer `gog` and `gog:1207658691` — byte-identical to
//! what the GOG adapter would derive on Windows. That is what makes a patch
//! published by a translator against `gog:1207658691` reachable by a Linux user
//! who installed the same game through a Lutris script, with neither of them
//! knowing the other's launcher exists.
//!
//! | `service` | wrapped source | `service_id` is |
//! | --- | --- | --- |
//! | `steam`, `steamwindows` | [`MasdarLuba::Steam`] | a decimal app id (`u32`) |
//! | `gog` | [`MasdarLuba::Gog`] | a decimal product id (`u64`) |
//! | `egs` | [`MasdarLuba::Epic`] | the catalogue item name, a string |
//! | `origin`, `ea`, `ea_app` | [`MasdarLuba::Ea`] | EA's content id, a string |
//! | `ubisoft`, `uplay` | [`MasdarLuba::Ubisoft`] | a decimal install id (`u32`) |
//! | `battlenet`, `battle.net`, `blizzard` | [`MasdarLuba::BattleNet`] | a product code, a string |
//! | `amazon` | [`MasdarLuba::Amazon`] | the product ASIN |
//! | `itchio`, `itch.io`, `itch` | [`MasdarLuba::Itch`] | a decimal game id (`i64`) |
//! | `humblebundle`, `humble` | none | a Humble machine name, which the vocabulary has no variant for |
//! | `lutris`, `flathub`, `xdg`, empty | none | not a store at all |
//!
//! The `None` rows matter as much as the others. A hand-written install script,
//! a disc image, an emulated cartridge and a Humble download have **no upstream
//! identity**, and inventing one would key a patch against a store identifier
//! the game does not have — a patch that would then be offered to the wrong
//! people and never to the right ones. Those entries keep only their Lutris
//! slug, [`MasdarLuba::aila`] answers `lutris`, and they shard under Lutris
//! because there is no other family they could belong to.
//!
//! The slug is kept in **both** cases. [`MasdarLutris`] carries `silaa`
//! unconditionally, because a Lutris game that wraps a store is still launched
//! through `lutris lutris:rungameid/<slug>` — losing the slug would mean keeping
//! an identity that can be matched to a patch and cannot be started.
//!
//! ## Configuration files are YAML, and there is no YAML crate here
//!
//! Lutris writes one small file per game. Adding a YAML dependency to the
//! discovery crate to read four keys out of it would be a poor trade, so this
//! module carries [`hallil_yaml`] — a deliberately tiny, deliberately strict
//! reader for the exact subset both Lutris and Bottles emit, documented in full
//! on that function. It is `pub(crate)` and [`crate::matajir::bottles`] imports
//! it rather than growing a second one; the two launchers are both Python
//! programs calling `yaml.dump`, so they write the same shape and one reader is
//! correct for both.
//!
//! The reader **refuses** every construct outside its subset instead of guessing
//! at it. A refused value yields nothing to every accessor and is recorded with
//! its line number, so a game missing its prefix because Lutris started writing
//! an anchor produces a warning naming the construct and the line rather than a
//! silently absent prefix.
//!
//! ## What Lutris does not know
//!
//! Install size, build identifier and download completeness are all absent from
//! `pga.db`, and this adapter reports them absent rather than substituting
//! plausible values. `hajm` is zero, `bina_manassa` is `None`, and `muktamila`
//! is true for every row — because `installed = 0` rows are skipped entirely,
//! and Lutris has no partial-install state to distinguish. Content fingerprints
//! exist precisely for launchers like this one.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};
use taarib_mustalahat::luba::{MasdarLuba, MasdarLutris};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier. It names the *adapter*; the games it produces shard
/// under whichever store their `service` column named, and only the ones with no
/// store behind them shard under `lutris`.
const MUARRIF: &str = "lutris";

/// Lutris is a Linux program. There is no Windows build and no macOS build, and
/// listing either would mean running a scan that can only ever find nothing.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Linux];

/// The Flatpak application id.
const HAWIYAT_FLATPAK: &str = "net.lutris.Lutris";

/// The directory Lutris uses under each of the three XDG roots.
const MUJALLAD_LUTRIS: &str = "lutris";

/// The catalogue. "Personal game archive", which is what Lutris has called it
/// since long before it had a database at all.
const ISM_QAIDA: &str = "pga.db";

/// The one table this adapter reads.
const JADWAL_ALAAB: &str = "games";

/// Where the per-game configuration files live, under the configuration root.
const MUJALLAD_IDADAT: &str = "games";

/// How long a read waits for a lock Lutris is holding.
///
/// Lutris holds its write lock for milliseconds, so half a second is generous.
/// Waiting longer would mean a library screen that stalls because the user
/// happened to be installing a game while it opened.
const MUHLAT_INTIZAR: Duration = Duration::from_millis(500);

/// The largest configuration file this adapter will read into memory.
///
/// A Lutris game config is a few hundred bytes and a Bottles `bottle.yml` with a
/// hundred registered programs is a few tens of kilobytes. Four mebibytes is
/// three orders of magnitude above either and far below the point where a
/// corrupt or hostile file could matter.
pub(crate) const HADD_WATHIQA: u64 = 4 * 1024 * 1024;

/// Lutris runners that produce a Windows program running under plain Wine.
///
/// `winesteam` is the retired runner that ran the Windows Steam client inside a
/// prefix; installations made by it still exist in old databases.
const MUSHAGHGHILAT_WINE: [&str; 2] = ["wine", "winesteam"];

/// Lutris runners that produce a Windows program running under Proton.
const MUSHAGHGHILAT_PROTON: [&str; 2] = ["proton", "protonge"];

/// Lutris runners that run something native to Linux.
///
/// `steam` is native in the sense that matters here: Lutris hands the launch to
/// the Steam client, and it is Steam — not Lutris — that decides whether Proton
/// is involved. Reporting a prefix Lutris does not own would be inventing one,
/// and the Steam adapter finds the same game with the truth attached.
const MUSHAGHGHILAT_ASLIYA: [&str; 6] = ["linux", "steam", "flatpak", "web", "browser", "xdg"];

/// Lutris.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarLutris;

impl MatjarLutris {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarLutris {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "لوتريس"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Lutris"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        JudhurLutris::awwal(siyaq).map(|judhur| judhur.bayanat)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurQiraatFahras`] when `pga.db` exists and
    /// neither an ordinary read-only nor an immutable connection can be
    /// established — the file has been deleted between the existence check and
    /// here, or the filesystem refuses the read — and
    /// [`KhataKashf::TarwisatFahrasTalifa`] when the file opens but carries no
    /// `games` table, which means it is either corrupt or not a Lutris database
    /// at all. Both make the entire catalogue unreadable, which is the only
    /// thing this trait fails a scan for.
    ///
    /// Everything narrower degrades one entry: a row whose install directory has
    /// been deleted, a `service_id` that will not parse into the type its store
    /// uses, a configuration file that is missing or that uses a YAML construct
    /// the reader refuses. Each is a [`TanbihFahs`] on the result and the other
    /// games still arrive.
    ///
    /// There is no user override for Lutris in `IdadatManassat`, so
    /// [`KhataKashf::JidhrMuhaddadMafqud`] cannot arise here; when one is added,
    /// this is where it belongs.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let Some(judhur) = JudhurLutris::awwal(siyaq) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, Some(judhur.bayanat.clone()));

        let qaida = judhur.qaida();
        let (ittisal, thabita) = iftah_lil_qiraa(&qaida)?;
        if thabita {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                qaida.display().to_string(),
                "pga.db could not be opened for ordinary read-only access, so it was read in \
                 immutable mode instead. That is safe for a running Lutris, but a game added in \
                 the last few seconds may be missing until the next scan.",
            ));
        }

        let sufuf = match asfuf_alaab(&ittisal) {
            Ok(sufuf) => sufuf,
            Err(sabab) => {
                return Err(KhataKashf::TarwisatFahrasTalifa {
                    matjar: MUARRIF,
                    masar: qaida,
                    tafsil: format!("the games table could not be read ({sabab})"),
                    mawdi: None,
                }
                .into());
            },
        };

        // Resolved once and passed down: every game that does not override the
        // Wine build inherits this one, and re-reading `runners/wine.yml` per
        // game would be one file open per title for a value that cannot change
        // during a scan.
        let iftiradi = isdar_wine_iftiradi(&judhur);
        for saff in &sufuf {
            if let Some(luba) =
                luba_min_saff(saff, &judhur, iftiradi.as_deref(), &mut natija.tanbihat)
            {
                natija.alaab.push(luba);
            }
        }

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let Some(judhur) = JudhurLutris::awwal(siyaq) else {
            return Vec::new();
        };
        // The data root is where `pga.db` and its `-wal` are rewritten on every
        // install, uninstall and play; the configuration directory is where a
        // prefix change lands without the database being touched at all. Both
        // are needed: watching only the first misses a user pointing a game at a
        // different prefix, and watching only the second misses every install.
        [judhur.bayanat.clone(), judhur.idadat.join(MUJALLAD_IDADAT)]
            .into_iter()
            .filter(|masar| masar.is_dir())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// where Lutris keeps its three roots
// ---------------------------------------------------------------------------

/// One consistent Lutris installation: its data, configuration and cache roots.
///
/// Held together rather than resolved one at a time because mixing them is a
/// silent, plausible failure. A machine can carry both the distribution package
/// and the Flatpak, each with its own catalogue and its own prefixes; reading
/// the native `pga.db` and then looking for `configpath` under the Flatpak
/// configuration root would find nothing for every game, and reading them the
/// other way round would find files that exist and describe different prefixes.
#[derive(Debug, Clone, PartialEq, Eq)]
struct JudhurLutris {
    /// `$XDG_DATA_HOME/lutris`, holding `pga.db` and the downloaded artwork.
    bayanat: PathBuf,
    /// `$XDG_DATA_HOME` itself, because the icon theme Lutris installs into is
    /// a sibling of its own data directory rather than a child of it.
    bayanat_am: PathBuf,
    /// `$XDG_CONFIG_HOME/lutris`, holding `games/<configpath>.yml`.
    idadat: PathBuf,
    /// `$XDG_CACHE_HOME/lutris`, where older releases kept the artwork that
    /// current ones keep under the data root.
    makhbaa: PathBuf,
}

impl JudhurLutris {
    /// The catalogue file under this installation's data root.
    fn qaida(&self) -> PathBuf {
        self.bayanat.join(ISM_QAIDA)
    }

    /// The first candidate installation that actually has a catalogue.
    fn awwal(siyaq: &SiyaqFahs) -> Option<Self> {
        Self::muhtamala(siyaq).into_iter().find(|judhur| judhur.qaida().is_file())
    }

    /// Every installation layout worth looking at, most likely first.
    ///
    /// The native layout is first because a user who has both almost always
    /// installed the distribution package first and the Flatpak to test
    /// something. The Flatpak layout is only considered when the scan is allowed
    /// to look inside containers.
    fn muhtamala(siyaq: &SiyaqFahs) -> Vec<Self> {
        let mut judhur: Vec<Self> = Vec::new();

        // The user's override names the *data* root — the directory holding
        // `pga.db`. The other three are derived from it as siblings rather than
        // taken from the conventional locations, because a user who moved their
        // Lutris installation moved all four together; pairing an overridden
        // data root with a conventional config root is what produces a scan
        // that finds games and none of their executables.
        if let Some(bayanat) = siyaq.manassat.lutris.as_ref() {
            let walid = bayanat.parent().unwrap_or(bayanat).to_path_buf();
            judhur.push(Self {
                bayanat: bayanat.clone(),
                bayanat_am: walid.clone(),
                idadat: walid.join(MUJALLAD_LUTRIS),
                makhbaa: walid.join(MUJALLAD_LUTRIS),
            });
        }

        // All three from the context: the XDG base directories are ambient, and
        // the context has already applied the specification's absolute-only
        // rule and its home-relative defaults to each of them.
        judhur.push(Self {
            bayanat: siyaq.khazina_bayanat.join(MUJALLAD_LUTRIS),
            bayanat_am: siyaq.khazina_bayanat.clone(),
            idadat: siyaq.khazina_idadat.join(MUJALLAD_LUTRIS),
            makhbaa: siyaq.khazina_makhbaa.join(MUJALLAD_LUTRIS),
        });

        if siyaq.yashmal_hawiyat {
            let hawiya = siyaq.manzil.join(".var").join("app").join(HAWIYAT_FLATPAK);
            judhur.push(Self {
                bayanat: hawiya.join("data").join(MUJALLAD_LUTRIS),
                bayanat_am: hawiya.join("data"),
                idadat: hawiya.join("config").join(MUJALLAD_LUTRIS),
                makhbaa: hawiya.join("cache").join(MUJALLAD_LUTRIS),
            });
        }

        judhur
    }
}

// ---------------------------------------------------------------------------
// opening pga.db without touching it
// ---------------------------------------------------------------------------

/// Opens `pga.db` for reading and nothing else.
///
/// The boolean says whether the fallback immutable mode had to be used, which
/// the caller turns into a warning: immutable reads bypass the write-ahead log,
/// so they can be a few writes stale.
///
/// Ordinary read-only is tried first because it is correct while Lutris is
/// running — it takes shared read locks and reads committed data out of the
/// `-wal`. `immutable=1` is the fallback rather than the default because it
/// promises `SQLite` the file cannot change, which is a promise this process is
/// not in a position to make and which costs the `-wal` entirely.
///
/// # Errors
///
/// Returns [`KhataKashf::TaadhurQiraatFahras`] when neither mode can open the
/// file, naming the path so a permissions problem is actionable rather than
/// mysterious.
fn iftah_lil_qiraa(masar: &Path) -> Natija<(Connection, bool)> {
    // READ_ONLY is the guarantee; URI is what allows the query parameters that
    // select the two modes; NO_MUTEX is correct because the connection never
    // leaves the thread that made it.
    let hudud = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let asas = uri_min_masar(masar);

    if let Ok(ittisal) = jarrib_fath(&format!("{asas}?mode=ro"), hudud) {
        return Ok((ittisal, false));
    }
    match jarrib_fath(&format!("{asas}?immutable=1"), hudud) {
        Ok(ittisal) => Ok((ittisal, true)),
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
fn jarrib_fath(uri: &str, hudud: OpenFlags) -> Result<Connection, rusqlite::Error> {
    let ittisal = Connection::open_with_flags(uri, hudud)?;
    ittisal.busy_timeout(MUHLAT_INTIZAR)?;
    let _: i64 = ittisal.query_row("SELECT count(*) FROM sqlite_master", [], |saff| saff.get(0))?;
    Ok(ittisal)
}

/// Builds a `SQLite` `file:` URI out of a path.
///
/// `SQLite` reads its own URIs, so anything that could be mistaken for a query
/// string or a fragment has to be percent-encoded first: a home directory with a
/// `?` in its name would otherwise silently become a database path plus a
/// parameter list, and the parameters would be whatever the directory name
/// happened to spell.
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
enum QeemaAmud {
    /// `TEXT`, and any `BLOB` that decodes as text.
    Nass(String),
    /// `INTEGER`.
    Raqm(i64),
    /// `REAL`.
    Ashari(f64),
    /// `NULL`, and any value that could not be fetched.
    Faragh,
}

impl QeemaAmud {
    /// Converts one fetched value.
    fn min(qeema: rusqlite::types::ValueRef<'_>) -> Self {
        match qeema {
            rusqlite::types::ValueRef::Null => Self::Faragh,
            rusqlite::types::ValueRef::Integer(raqm) => Self::Raqm(raqm),
            rusqlite::types::ValueRef::Real(ashari) => Self::Ashari(ashari),
            rusqlite::types::ValueRef::Text(bayt) | rusqlite::types::ValueRef::Blob(bayt) => {
                Self::Nass(String::from_utf8_lossy(bayt).into_owned())
            },
        }
    }
}

/// One row of `games`, addressed by column name rather than by position.
///
/// Lutris's schema moves: `service` and `service_id` did not exist before the
/// integrated store clients, `sortname` and `discord_id` arrived later still,
/// and columns have been dropped as well as added. A reader that named columns
/// in its `SELECT` would break on the first release that removed one, and a
/// reader that addressed them by index would silently read the wrong column on
/// the first release that inserted one. Neither failure is acceptable for the
/// primary discovery path on Linux, so every value is fetched by name and a
/// column this build of Lutris does not have simply yields nothing.
#[derive(Debug, Clone, Default)]
struct SaffLutris {
    /// The row, keyed by lowercased column name.
    qiyam: BTreeMap<String, QeemaAmud>,
}

impl SaffLutris {
    /// A text value, trimmed, or nothing when the column is absent, null or
    /// empty. An integer is rendered as its decimal text, because `service_id`
    /// is declared `TEXT` and stored as an integer by some Lutris releases.
    fn nass(&self, ism: &str) -> Option<String> {
        match self.qiyam.get(ism) {
            Some(QeemaAmud::Nass(nass)) if !nass.trim().is_empty() => {
                Some(nass.trim().to_owned())
            },
            Some(QeemaAmud::Raqm(raqm)) => Some(raqm.to_string()),
            _ => None,
        }
    }

    /// An integer value, or nothing when the column is absent or holds something
    /// that is not one.
    ///
    /// A `REAL` is deliberately refused: nothing this adapter reads as a number
    /// — an identifier, a unix timestamp, an installed flag — is ever stored as
    /// a float, so a float here means the schema has moved somewhere this reader
    /// should not follow it, and narrowing one into a game identifier would be
    /// inventing data.
    fn raqm(&self, ism: &str) -> Option<i64> {
        match self.qiyam.get(ism) {
            Some(QeemaAmud::Raqm(raqm)) => Some(*raqm),
            Some(QeemaAmud::Nass(nass)) => nass.trim().parse().ok(),
            _ => None,
        }
    }
}

/// Reads every row of `games` as name-addressed values.
///
/// The table name is interpolated into the statement, which is safe here and
/// only here: it is a `const` in this module and never a value read from a file,
/// a database or anything a user typed. It is quoted regardless, so a future
/// name that collides with a keyword still parses.
///
/// # Errors
///
/// Returns the `SQLite` failure unchanged. The caller turns it into a fatal
/// [`KhataKashf::TarwisatFahrasTalifa`], because a `pga.db` with no readable
/// `games` table is not a Lutris catalogue and there is nothing else in it to
/// fall back to.
fn asfuf_alaab(ittisal: &Connection) -> Result<Vec<SaffLutris>, rusqlite::Error> {
    let mut bayan = ittisal.prepare(&format!("SELECT * FROM \"{JADWAL_ALAAB}\""))?;
    let asmaa: Vec<String> =
        bayan.column_names().into_iter().map(str::to_ascii_lowercase).collect();
    let mut nataij = bayan.query([])?;
    let mut khuruj = Vec::new();
    while let Some(saff) = nataij.next()? {
        let mut qiyam = BTreeMap::new();
        for (fahras, ism) in asmaa.iter().enumerate() {
            let qeema = saff.get_ref(fahras).map_or(QeemaAmud::Faragh, QeemaAmud::min);
            let _ = qiyam.insert(ism.clone(), qeema);
        }
        khuruj.push(SaffLutris { qiyam });
    }
    Ok(khuruj)
}

// ---------------------------------------------------------------------------
// which store an entry came from
// ---------------------------------------------------------------------------

/// Builds the wrapped store source for one row, or `None` when there is not one.
///
/// This is the function the whole adapter exists to get right, and it is written
/// to fail closed. A `service` this reader does not recognise, a `service_id`
/// that is absent, and a `service_id` that will not parse into the type its
/// store's identifiers actually are all produce `None` — never a guess. The
/// entry is still discovered and still patchable; it simply shards under
/// `lutris` and matches only patches published against its Lutris slug.
///
/// A recognised service whose identifier will not parse is worth a warning,
/// because it is the one case where a patch the user could have had is out of
/// reach and the reason is not visible anywhere else. An unrecognised service is
/// not worth one: Lutris adds store integrations faster than this list is
/// updated, and a warning per game for every new one would be noise on every
/// scan.
fn masdar_matjar(
    khidma: Option<&str>,
    muarrif_khidma: Option<&str>,
    ism: &str,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<MasdarLuba> {
    let khidma = khidma?.trim().to_ascii_lowercase();
    if khidma.is_empty() {
        return None;
    }

    let maruf = matches!(
        khidma.as_str(),
        "steam"
            | "steamwindows"
            | "gog"
            | "egs"
            | "origin"
            | "ea"
            | "ea_app"
            | "ubisoft"
            | "uplay"
            | "battlenet"
            | "battle.net"
            | "blizzard"
            | "amazon"
            | "itchio"
            | "itch.io"
            | "itch"
    );

    let Some(muarrif) = muarrif_khidma.map(str::trim).filter(|qeema| !qeema.is_empty()) else {
        if maruf {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                format!("{MUARRIF}: {ism}"),
                format!(
                    "Lutris records this game as a {khidma} title and stores no identifier for \
                     it, so it can only be matched against patches published for Lutris itself. \
                     Re-syncing the {khidma} account inside Lutris restores the identifier."
                ),
            ));
        }
        return None;
    };

    let mafqud = |naw: &str, tanbihat: &mut Vec<TanbihFahs>| -> Option<MasdarLuba> {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{khidma}: {muarrif}"),
            format!(
                "this Lutris entry claims to be a {khidma} title, and its identifier is not {naw} \
                 the way {khidma} writes them, so Taarib cannot derive the identity the patch \
                 registry shards {khidma} titles by. The game is still listed under its Lutris \
                 slug."
            ),
        ));
        None
    };

    match khidma.as_str() {
        // Steam app ids are unsigned 32-bit and Lutris stores them as text in
        // `service_id` and as an integer in the legacy `steamid` column.
        "steam" | "steamwindows" => muarrif
            .parse::<u32>()
            .ok()
            .map(MasdarLuba::Steam)
            .or_else(|| mafqud("a number", tanbihat)),
        // GOG product ids exceed 32 bits for titles published after 2018.
        "gog" => muarrif
            .parse::<u64>()
            .ok()
            .map(MasdarLuba::Gog)
            .or_else(|| mafqud("a number", tanbihat)),
        // Epic's catalogue item name is an opaque string and is never numeric.
        "egs" => Some(MasdarLuba::Epic(muarrif.to_owned())),
        "origin" | "ea" | "ea_app" => Some(MasdarLuba::Ea(muarrif.to_owned())),
        "ubisoft" | "uplay" => muarrif
            .parse::<u32>()
            .ok()
            .map(MasdarLuba::Ubisoft)
            .or_else(|| mafqud("a number", tanbihat)),
        "battlenet" | "battle.net" | "blizzard" => {
            Some(MasdarLuba::BattleNet(muarrif.to_owned()))
        },
        "amazon" => Some(MasdarLuba::Amazon(muarrif.to_owned())),
        // itch.io game ids are signed in the vocabulary because butler's own
        // schema declares them signed, and this adapter does not widen or
        // narrow a store's own type.
        "itchio" | "itch.io" | "itch" => muarrif
            .parse::<i64>()
            .ok()
            .map(MasdarLuba::Itch)
            .or_else(|| mafqud("a number", tanbihat)),
        // Humble, Flathub, XDG desktop entries and Lutris's own community
        // installer service. The first has no vocabulary variant and the rest
        // are not stores; all four keep the slug and nothing else.
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// what the runner column means
// ---------------------------------------------------------------------------

/// What kind of thing a Lutris runner produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NawMushaghghil {
    /// A Windows program inside a Wine prefix.
    Wine,
    /// A Windows program inside a Proton prefix.
    Proton,
    /// A program native to Linux.
    Asli,
    /// A console title inside an emulator, which is itself a native program.
    Muhakah,
}

/// Classifies a Lutris runner.
///
/// The default is deliberately **emulation**, not native. Lutris ships roughly
/// four dozen runners and all but a handful of them are emulators —
/// `libretro`, `dosbox`, `scummvm`, `dolphin`, `pcsx2`, `rpcs3`, `duckstation`,
/// `mupen64plus`, `flycast`, `melonds`, `vice`, `mednafen`, and so on down a
/// list that grows with every release. A runner this build has never heard of is
/// therefore far more likely to be a new emulator than a new way of running a
/// Windows program, and classifying it as native would claim a fact — that the
/// title is a Linux binary — which nothing on disk supports.
///
/// An absent or empty runner is native, because that is what a row written by a
/// Lutris old enough not to have the column describes.
fn naw_mushaghghil(mushaghghil: Option<&str>) -> NawMushaghghil {
    let Some(ism) = mushaghghil.map(str::trim).filter(|ism| !ism.is_empty()) else {
        return NawMushaghghil::Asli;
    };
    let saghir = ism.to_ascii_lowercase();
    if MUSHAGHGHILAT_WINE.contains(&saghir.as_str()) {
        NawMushaghghil::Wine
    } else if MUSHAGHGHILAT_PROTON.contains(&saghir.as_str()) {
        NawMushaghghil::Proton
    } else if MUSHAGHGHILAT_ASLIYA.contains(&saghir.as_str()) {
        NawMushaghghil::Asli
    } else {
        NawMushaghghil::Muhakah
    }
}

/// Resolves the compatibility layer one Lutris game runs behind.
///
/// A Wine or Proton runner without a prefix is the case worth spelling out. It
/// happens: Lutris writes the game's row when the installer starts and the
/// prefix when it finishes, so a scan during an install sees one without the
/// other, and a user who deleted a prefix by hand leaves the row behind. There
/// is no `BeeatTawafuq` variant for "a compatibility layer whose prefix is not
/// known", so the honest encoding is the one Heroic's adapter already uses:
/// report [`BeeatTawafuq::Asli`], attach a
/// [`SimatLuba::TabaqatTawafuq`] recording that a layer is involved and is
/// unidentified, and raise a warning that names the remedy. Reporting a prefix
/// path that does not exist would be worse than reporting none: every path
/// derived from it would resolve, and every file written through it would land
/// somewhere the game never looks.
fn beea_luba(
    naw: NawMushaghghil,
    idad: &IdadLutris,
    isdar_iftiradi: Option<&str>,
    ism: &str,
    simat: &mut Vec<SimatLuba>,
    tanbihat: &mut Vec<TanbihFahs>,
) -> BeeatTawafuq {
    match naw {
        // An emulator is a native Linux program, so an emulated title runs
        // through no compatibility layer at all. That the machine underneath is
        // emulated is recorded as a trait by the caller, where it belongs.
        NawMushaghghil::Asli | NawMushaghghil::Muhakah => BeeatTawafuq::Asli,
        NawMushaghghil::Wine | NawMushaghghil::Proton => {
            let Some(beea) = idad.beea.clone().filter(|masar| masar.is_dir()) else {
                simat.push(SimatLuba::TabaqatTawafuq(
                    "Wine, prefix not recorded by Lutris".to_owned(),
                ));
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    ism.to_owned(),
                    "Lutris runs this game through Wine and records no usable prefix for it, so \
                     Taarib cannot tell where the game's own C: drive is. Finish the Lutris \
                     installation, or launch the game once from Lutris so the prefix is created, \
                     then rescan.",
                ));
                return BeeatTawafuq::Asli;
            };

            // `hal_beea` is where prefix knowledge lives — the `dosdevices`
            // drive map, the registry, the Wine build. A failure here is not
            // fatal: the prefix path Lutris recorded is still the right answer
            // for Phase 15, and what is lost is only the detected build.
            let maktashaf = match crate::beea::hal_beea(&beea) {
                Ok(maalumat) => maalumat.isdar_wine,
                Err(khata) => {
                    tanbihat.push(TanbihFahs::jadeed(
                        MUARRIF,
                        beea.display().to_string(),
                        format!("{ism}: {}", khata.injilizi),
                    ));
                    None
                },
            };

            let isdar = idad
                .isdar_wine
                .clone()
                .or_else(|| isdar_iftiradi.map(str::to_owned))
                .or(maktashaf);

            if naw == NawMushaghghil::Proton {
                BeeatTawafuq::Proton { isdar: isdar.unwrap_or_else(|| "Proton".to_owned()), beea }
            } else {
                BeeatTawafuq::Wine { isdar, beea }
            }
        },
    }
}

// ---------------------------------------------------------------------------
// one game's YAML configuration
// ---------------------------------------------------------------------------

/// One Lutris game configuration, reduced to what decides how it runs.
///
/// Lutris writes far more than this — every runner option, every system
/// override, the whole installer script's residue — and none of the rest changes
/// where the game's files are or which prefix they live in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct IdadLutris {
    /// `game: exe:` — the program Lutris launches. Absolute on a Linux runner,
    /// relative to the install directory or a `C:\` path on a Wine runner.
    tanfidhi: Option<String>,
    /// `game: prefix:` — the directory holding `drive_c`.
    beea: Option<PathBuf>,
    /// `game: working_dir:` — where Lutris starts the program, which is the
    /// install root for every installer script that sets it.
    mujallad_amal: Option<PathBuf>,
    /// `game: main_file:` — the ROM or disc image an emulator runner loads. The
    /// emulated equivalent of `exe`, and the only path such an entry has.
    malaf_asasi: Option<String>,
    /// `game: args:` — the arguments the user or the installer script set, so
    /// Phase 15 extends them rather than replacing them.
    khiyarat: Option<String>,
    /// `wine: version:` — the Wine build this one game overrides the default
    /// with.
    isdar_wine: Option<String>,
}

impl IdadLutris {
    /// Reads one `games/<configpath>.yml`.
    ///
    /// A configuration that is absent is ordinary and silent: Lutris only writes
    /// one when a game has something to configure, and a Steam or Flatpak entry
    /// frequently has nothing. A configuration that is present and uses a YAML
    /// construct the reader refuses is **not** silent, because that is the case
    /// where a prefix exists and this adapter could not see it.
    fn iqra(masar: &Path, ism: &str, tanbihat: &mut Vec<TanbihFahs>) -> Self {
        let Some(wathiqa) = iqra_yaml(masar) else {
            return Self::default();
        };

        for masar_miftah in [
            ["game", "exe"].as_slice(),
            ["game", "prefix"].as_slice(),
            ["game", "working_dir"].as_slice(),
            ["game", "main_file"].as_slice(),
            ["game", "args"].as_slice(),
            ["wine", "version"].as_slice(),
        ] {
            if let Some(rafd) = wathiqa.marfud(masar_miftah) {
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    masar.display().to_string(),
                    format!(
                        "{ism}: the key {} is written as {} on line {}, which this reader refuses \
                         rather than guessing at. That setting is being ignored.",
                        masar_miftah.join(": "),
                        rafd.bina,
                        rafd.satr
                    ),
                ));
            }
        }

        Self {
            tanfidhi: wathiqa.nass(&["game", "exe"]),
            beea: wathiqa.nass(&["game", "prefix"]).map(PathBuf::from),
            mujallad_amal: wathiqa.nass(&["game", "working_dir"]).map(PathBuf::from),
            malaf_asasi: wathiqa.nass(&["game", "main_file"]),
            khiyarat: wathiqa.nass(&["game", "args"]),
            isdar_wine: wathiqa.nass(&["wine", "version"]),
        }
    }
}

/// The Wine build Lutris uses for every game that does not override it.
///
/// Lutris keeps per-runner defaults in `runners/wine.yml`, and a game whose own
/// configuration says nothing about Wine runs on whatever is in there. Reading
/// it once per scan rather than per game is why it is resolved here and passed
/// down.
fn isdar_wine_iftiradi(judhur: &JudhurLutris) -> Option<String> {
    let masar = judhur.idadat.join("runners").join("wine.yml");
    iqra_yaml(&masar)?.nass(&["wine", "version"])
}

// ---------------------------------------------------------------------------
// artwork Lutris already downloaded
// ---------------------------------------------------------------------------

/// Finds the artwork Lutris has already fetched for one slug.
///
/// Local files, never URLs. Lutris downloads all three images at install time
/// and names them after the slug, so there is nothing to fetch and nothing to
/// guess — and a local file costs no network, works offline, and is exactly the
/// image the user is already looking at inside Lutris.
///
/// Both the current layout and the retired one are probed. Lutris moved artwork
/// out of `$XDG_CACHE_HOME/lutris` and into `$XDG_DATA_HOME/lutris` when it
/// stopped treating covers as disposable, and a library installed before that
/// move still has the files in the old place; checking both costs four
/// `is_file` calls and finding nothing would cost the user their grid.
fn suwar_lutris(judhur: &JudhurLutris, silaa: &str) -> MasadirSuwar {
    let min_mujallad = |jidhr: &Path, mujallad: &str, imtidad: &str| -> Option<PathBuf> {
        let nisbi = format!("{mujallad}/{silaa}.{imtidad}");
        let masar = dakhil(jidhr, &nisbi).ok()?;
        masar.is_file().then_some(masar)
    };

    let awwal = |mujallad: &str| -> Option<MasdarSura> {
        for jidhr in [judhur.bayanat.as_path(), judhur.makhbaa.as_path()] {
            for imtidad in ["jpg", "png", "jpeg"] {
                if let Some(masar) = min_mujallad(jidhr, mujallad, imtidad) {
                    return Some(MasdarSura::Malaf(masar));
                }
            }
        }
        None
    };

    // The icon is installed into the user's own icon theme rather than into
    // Lutris's data directory, because Lutris registers a desktop entry for
    // every game and a desktop entry's icon has to be somewhere the desktop
    // looks. That is why this path is resolved from the XDG data root and not
    // from the Lutris directory under it.
    let shiar = ["128x128", "256x256", "64x64"].into_iter().find_map(|hajm| {
        let nisbi = format!("icons/hicolor/{hajm}/apps/lutris_{silaa}.png");
        let masar = dakhil(&judhur.bayanat_am, &nisbi).ok()?;
        masar.is_file().then_some(MasdarSura::Malaf(masar))
    });

    MasadirSuwar { ghilaf: awwal("coverart"), batl: awwal("banners"), shiar }
}

// ---------------------------------------------------------------------------
// one row becomes one game
// ---------------------------------------------------------------------------

/// Turns one `games` row into a discovered game, or into nothing plus a reason.
///
/// `installed = 0` rows leave silently and without a warning. They are not
/// broken games: Lutris keeps a row for every title it has ever seen in a synced
/// store library, so a user with a large Steam account has hundreds of them and
/// warning about each would bury every real diagnostic under a wall of entries
/// that are working exactly as intended.
fn luba_min_saff(
    saff: &SaffLutris,
    judhur: &JudhurLutris,
    isdar_iftiradi: Option<&str>,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    if saff.raqm("installed").unwrap_or(0) == 0 {
        return None;
    }

    let silaa = if let Some(silaa) = saff.nass("slug").or_else(|| saff.nass("installer_slug")) {
        silaa
    } else {
        let Some(raqm) = saff.raqm("id") else {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                judhur.qaida().display().to_string(),
                "a row in the games table has neither a slug nor a row id, so there is no \
                 stable way to name it and no way to launch it through Lutris",
            ));
            return None;
        };
        // `lutris:rungameid/<id>` is Lutris's own launch URI, so the row id
        // is a real identity rather than a placeholder — it is what the
        // desktop entry Lutris writes for the game already uses.
        format!("id/{raqm}")
    };

    let ism = saff.nass("name").unwrap_or_else(|| silaa.clone());

    let idad = match saff.nass("configpath") {
        Some(masar_idad) => {
            let masar = dakhil(
                &judhur.idadat.join(MUJALLAD_IDADAT),
                &format!("{masar_idad}.yml"),
            );
            if let Ok(masar) = masar {
                IdadLutris::iqra(&masar, &ism, tanbihat)
            } else {
                // `configpath` is a file name Lutris derives from the slug, and
                // a value that escapes its own directory means the database has
                // been edited by something that is not Lutris.
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    format!("{MUARRIF}: {ism}"),
                    format!(
                        "the configuration path recorded for this game ({masar_idad}) does not \
                         stay inside Lutris's own configuration directory, so it was not read. \
                         Its prefix and executable are unknown."
                    ),
                ));
                IdadLutris::default()
            }
        },
        None => IdadLutris::default(),
    };

    let jidhr = saff
        .nass("directory")
        .map(PathBuf::from)
        .or_else(|| idad.mujallad_amal.clone())
        .or_else(|| walid_mutlaq(idad.tanfidhi.as_deref()))
        .or_else(|| walid_mutlaq(idad.malaf_asasi.as_deref()));

    let Some(jidhr) = jidhr else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{MUARRIF}: {ism}"),
            "Lutris lists this game as installed and records no directory, no working directory \
             and no absolute executable, so there is nothing on disk to probe",
        ));
        return None;
    };

    if !jidhr.is_dir() {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "Lutris lists {ism} as installed here and the folder is gone; it was deleted \
                 outside Lutris, or lives on a drive that is not mounted"
            ),
        ));
        return None;
    }

    let mushaghghil = saff.nass("runner");
    let naw = naw_mushaghghil(mushaghghil.as_deref());
    let mut simat = Vec::new();
    let beea = beea_luba(naw, &idad, isdar_iftiradi, &ism, &mut simat, tanbihat);

    match &beea {
        BeeatTawafuq::Proton { isdar, .. } => {
            simat.push(SimatLuba::TabaqatTawafuq(format!("Proton, {isdar}")));
        },
        BeeatTawafuq::Wine { isdar, .. } => {
            simat.push(SimatLuba::TabaqatTawafuq(
                isdar.clone().unwrap_or_else(|| "Wine".to_owned()),
            ));
        },
        BeeatTawafuq::Asli | BeeatTawafuq::Rosetta => {},
    }

    if naw == NawMushaghghil::Muhakah {
        // The emulator itself is a native Linux program, so the compatibility
        // environment is `Asli` and this trait is the only place the fact that a
        // machine is being emulated survives. It is the one the vocabulary
        // provides for exactly this, and it carries the consequence: the entry
        // exists so the library is complete, not because it can be translated.
        // A ROM is a single opaque image whose text is inside a cartridge dump
        // the emulator never presents separately, and there is no path by which
        // this product modifies one.
        //
        // Both the runner and the image's extension go in the string. The
        // vocabulary asks for the extension family so the interface can name the
        // console; Lutris names the console more precisely than the extension
        // does — `.iso` is four different machines — so the runner leads and the
        // extension follows it when there is one.
        let ism_muhaki = mushaghghil.unwrap_or_else(|| "emulator".to_owned());
        let imtidad = idad
            .malaf_asasi
            .as_deref()
            .and_then(|malaf| Path::new(malaf).extension())
            .map(|imtidad| imtidad.to_string_lossy().to_ascii_lowercase());
        simat.push(SimatLuba::MuhakatRum(match imtidad {
            Some(imtidad) => format!("{ism_muhaki} (.{imtidad})"),
            None => ism_muhaki,
        }));
    }

    let asl = masdar_matjar(
        saff.nass("service").as_deref(),
        saff.nass("service_id").as_deref(),
        &ism,
        tanbihat,
    );

    Some(LubaMuktashafa {
        masdar: MasdarLuba::Lutris(Box::new(MasdarLutris { silaa: silaa.clone(), asl })),
        hala_matjar: None,
        ism,
        tanfidhi: tanfidhi_luba(&jidhr, &idad, saff, &beea),
        jidhr,
        // Lutris records no install size anywhere in `pga.db`. Walking the
        // install tree to compute one would turn a scan of two hundred games
        // into hundreds of thousands of `stat` calls, and discovery is on the
        // path the first screen waits for.
        hajm: 0,
        // Lutris has no build identifier for anything, including the store
        // titles it manages: its installer scripts record what they installed,
        // not which build it was. This is exactly the launcher content
        // fingerprints exist for.
        bina_manassa: None,
        akhir_tahdith: saff
            .nass("updated")
            .and_then(|nass| waqt_mahalli(&nass))
            .or_else(|| saff.raqm("installed_at").and_then(waqt_min_thawani)),
        akhir_laab: saff.raqm("lastplayed").and_then(waqt_min_thawani),
        beea,
        suwar: suwar_lutris(judhur, &silaa),
        khiyarat_tashghil: idad.khiyarat.clone(),
        // Every row that reaches here has `installed = 1`, and Lutris has no
        // partial-install state to distinguish: an interrupted Lutris install
        // leaves the row at zero and is skipped above.
        muktamila: true,
        simat,
    })
}

/// The parent directory of a path, but only when the path is absolute.
///
/// A relative executable is relative to the install directory, so using its
/// parent to *find* the install directory would be circular.
fn walid_mutlaq(masar: Option<&str>) -> Option<PathBuf> {
    let munaqqa = masar?.trim();
    if munaqqa.is_empty() {
        return None;
    }
    let murashah = Path::new(munaqqa);
    if !murashah.is_absolute() {
        return None;
    }
    murashah.parent().map(Path::to_path_buf).filter(|walid| walid.is_dir())
}

/// Resolves the program Lutris launches for one game.
///
/// Three candidates in order of how specifically each answers the question: the
/// configuration's `exe`, which is what Lutris actually runs; the `executable`
/// column, which older releases wrote instead; and the emulator runner's
/// `main_file`, which is the ROM or disc image and is the only path such an
/// entry has at all.
///
/// An absolute path outside the install directory is accepted, and that is
/// deliberate rather than an oversight. Lutris genuinely launches programs that
/// live elsewhere — a system-wide emulator binary, a Flatpak wrapper, a shell
/// script in the user's own `bin` — and rejecting those would leave the entries
/// that need an executable most with none. What is *not* accepted is a relative
/// path that escapes the install directory: that goes through [`dakhil`], so a
/// tampered `pga.db` cannot aim Taarib's probe at a file outside the game.
fn tanfidhi_luba(
    jidhr: &Path,
    idad: &IdadLutris,
    saff: &SaffLutris,
    beea: &BeeatTawafuq,
) -> Option<PathBuf> {
    let murashahat =
        [idad.tanfidhi.clone(), saff.nass("executable"), idad.malaf_asasi.clone()];
    murashahat.into_iter().flatten().find_map(|kham| hall_tanfidhi(jidhr, &kham, beea))
}

/// Resolves one executable candidate against the install root and the prefix.
fn hall_tanfidhi(jidhr: &Path, kham: &str, beea: &BeeatTawafuq) -> Option<PathBuf> {
    let munaqqa = kham.trim();
    if munaqqa.is_empty() {
        return None;
    }

    // A `C:\` path is a path in the prefix's fiction, and `wujud` owns the one
    // translation of that fiction into a real path. This adapter calls it and
    // never writes a second one.
    if huwa_masar_windows(munaqqa) {
        let jidhr_beea = beea.beea()?;
        let masar = crate::wujud::hall_masar_windows(jidhr_beea, munaqqa)?;
        return masar.is_file().then_some(masar);
    }

    let murashah = Path::new(munaqqa);
    if murashah.is_absolute() {
        return murashah.is_file().then(|| murashah.to_path_buf());
    }

    let nisbi = munaqqa.replace('\\', std::path::MAIN_SEPARATOR_STR);
    let kamil = dakhil(jidhr, &nisbi).ok()?;
    kamil.is_file().then_some(kamil)
}

/// Whether a string begins with a Windows drive letter.
///
/// Exactly one ASCII letter and a colon. A single-character check is enough
/// because the alternative forms — UNC paths and the `\??\` object namespace —
/// are handled by the translator itself, which refuses the ones that have no
/// host equivalent.
fn huwa_masar_windows(masar: &str) -> bool {
    let mut huruf = masar.chars();
    huruf.next().is_some_and(|harf| harf.is_ascii_alphabetic()) && huruf.next() == Some(':')
}

// ---------------------------------------------------------------------------
// timestamps, shared with the Bottles adapter
// ---------------------------------------------------------------------------

/// Normalizes a unix-seconds timestamp to RFC 3339.
///
/// Lutris writes `installed_at` and `lastplayed` as integer seconds. A value
/// that is not a representable instant — a corrupt row, a column that has been
/// repurposed — is dropped rather than clamped, because a timestamp nobody can
/// parse is worse in a record than an absent one.
fn waqt_min_thawani(raqm: i64) -> Option<String> {
    jiff::Timestamp::from_second(raqm).ok().map(|waqt| waqt.to_string())
}

/// Normalizes a textual timestamp to RFC 3339, and shared with
/// [`crate::matajir::bottles`] because both launchers are Python programs that
/// write times the same two ways.
///
/// Two shapes are accepted, in this order:
///
/// 1. **Anything that already carries an offset** — `2024-03-01T12:00:00Z`,
///    `2024-03-01 12:00:00+03:00`. Parsed as an instant and returned as one.
/// 2. **A naive local time** — `2024-03-01 12:00:00.123456`, which is what
///    Python's `datetime.now()` renders and what both Lutris and Bottles store.
///    Resolved through the **system time zone**, not through UTC. Treating a
///    naive timestamp as UTC is the tempting shortcut and it is wrong by however
///    many hours the user is from Greenwich: the value was written by a program
///    running on this machine, so this machine's zone is what it meant.
///
/// A value matching neither is dropped. Nothing downstream can use a timestamp
/// it cannot order.
pub(crate) fn waqt_mahalli(nass: &str) -> Option<String> {
    let munaqqa = nass.trim();
    if munaqqa.is_empty() {
        return None;
    }

    let bi_t = munaqqa.replacen(' ', "T", 1);
    if let Ok(waqt) = bi_t.parse::<jiff::Timestamp>() {
        return Some(waqt.to_string());
    }

    let madani = bi_t.parse::<jiff::civil::DateTime>().ok()?;
    let mantiqa = jiff::tz::TimeZone::system();
    madani.to_zoned(mantiqa).ok().map(|mahalli| mahalli.timestamp().to_string())
}

// ---------------------------------------------------------------------------
// The configuration reader, shared with the Bottles adapter
//
// Everything below this line is about one problem: Lutris and Bottles both
// write their configuration as YAML, this workspace has no YAML crate, and
// adding one to the discovery crate to read six keys would be a bad trade — a
// full YAML implementation is tens of thousands of lines of parsing surface
// applied to files this product never writes and only ever reads six values out
// of.
//
// The answer is not "a loose reader that mostly works". A loose reader is how a
// game silently loses its prefix: it returns *something* for a construct it does
// not understand, that something is wrong, and the wrongness surfaces three
// phases later as a patch installed into a directory the game does not use. So
// the reader below accepts a small, exactly stated subset and **refuses**
// everything else by name, recording what it refused and where.
// ---------------------------------------------------------------------------

/// How many lines of a configuration file are considered.
///
/// A Lutris game config is a dozen lines and the largest `bottle.yml` seen in
/// the wild is a few thousand. Forty thousand is far past either and bounds the
/// cost of a file somebody filled with newlines.
const HADD_SUTUR_YAML: usize = 40_000;

/// How deep a block mapping may nest.
///
/// Lutris nests two levels (`game: exe:`) and Bottles three
/// (`External_Programs: <key>: executable:`). Eight is generous and stops a
/// crafted file from driving this reader's recursion into the native stack.
const HADD_UMQ_YAML: usize = 8;

/// How many mapping entries one document may contribute.
///
/// Bounds the memory a single configuration file can make this process reserve.
const HADD_UQAD_YAML: usize = 20_000;

/// How many refusals are kept.
///
/// The list exists to explain a missing value, and one refusal explains it. A
/// file that is entirely outside the subset would otherwise carry one record per
/// line into a warning nobody can read.
const HADD_MARFUDAT_YAML: usize = 64;

/// One construct the reader refused rather than guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RafdYaml {
    /// What it was, named the way the YAML specification names it, and phrased
    /// to be readable at the end of a sentence in a diagnostics message.
    pub(crate) bina: &'static str,
    /// The one-based line it was on, so the user can open the file and look.
    pub(crate) satr: usize,
}

/// One value in a parsed configuration document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QeemaYaml {
    /// A scalar, unescaped.
    Nass(String),
    /// A nested block mapping.
    Khareeta(BTreeMap<String, Self>),
    /// An explicit null — `key:` with nothing under it, `~`, or `null`.
    Faragh,
    /// A construct outside the accepted subset. Yields nothing to every
    /// accessor, so a caller can never mistake a refusal for a value; the record
    /// is what lets that caller say *why* the value is missing.
    Marfuda(RafdYaml),
}

impl QeemaYaml {
    /// The scalar, or nothing for a null, a mapping, or a refusal.
    fn nass(&self) -> Option<String> {
        match self {
            Self::Nass(nass) if !nass.is_empty() => Some(nass.clone()),
            _ => None,
        }
    }

    /// One scalar field of a nested mapping, by key.
    ///
    /// This is what the Bottles adapter walks `External_Programs` with: each
    /// entry there is a mapping of `executable`, `path`, `name`, `folder` and
    /// `arguments`, and it is reached as a value rather than by an absolute
    /// path because the entry's own key is a value the file chose.
    pub(crate) fn nass_haql(&self, miftah: &str) -> Option<String> {
        match self {
            Self::Khareeta(khareeta) => ibn_khareeta(khareeta, miftah)?.nass(),
            _ => None,
        }
    }
}

/// A parsed configuration document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WathiqatYaml {
    /// The root mapping.
    judhur: BTreeMap<String, QeemaYaml>,
    /// Every construct refused while reading, in the order they were met.
    marfudat: Vec<RafdYaml>,
}

impl WathiqatYaml {
    /// The node at a key path, or nothing when any step of the path is absent.
    fn uqda(&self, masar: &[&str]) -> Option<&QeemaYaml> {
        let (awwal, baqi) = masar.split_first()?;
        let mut hali = ibn_khareeta(&self.judhur, awwal)?;
        for juz in baqi {
            let QeemaYaml::Khareeta(khareeta) = hali else {
                return None;
            };
            hali = ibn_khareeta(khareeta, juz)?;
        }
        Some(hali)
    }

    /// The scalar at a key path.
    pub(crate) fn nass(&self, masar: &[&str]) -> Option<String> {
        self.uqda(masar)?.nass()
    }

    /// The mapping at a key path.
    pub(crate) fn khareeta(&self, masar: &[&str]) -> Option<&BTreeMap<String, QeemaYaml>> {
        match self.uqda(masar)? {
            QeemaYaml::Khareeta(khareeta) => Some(khareeta),
            _ => None,
        }
    }

    /// The refusal at a key path, when that specific key is the one that was
    /// refused.
    ///
    /// This is the accessor that turns a silent absence into a message. A caller
    /// asks for `game: prefix:`, gets nothing, and asks here whether the reason
    /// was that the file used a construct outside the subset — which is the one
    /// case where the value exists in the file and Taarib cannot see it.
    pub(crate) fn marfud(&self, masar: &[&str]) -> Option<&RafdYaml> {
        match self.uqda(masar)? {
            QeemaYaml::Marfuda(rafd) => Some(rafd),
            _ => None,
        }
    }

    /// Everything refused anywhere in the document, including constructs that
    /// belonged to no key — a document that is a sequence, a tab in an
    /// indentation, a second document after `---`.
    pub(crate) fn marfudat(&self) -> &[RafdYaml] {
        &self.marfudat
    }
}

/// Looks one key up in a mapping, exactly first and case-insensitively second.
///
/// Both writers are Python programs whose key spellings come from source
/// literals, and both have changed the case of a key across releases — Bottles
/// alone has shipped `External_Programs`, and Lutris's runner sections are
/// lowercase while its top-level keys have not always been. An exact hit is
/// taken immediately and costs one tree lookup; the fallback costs a scan of a
/// mapping that has at most a few dozen entries, and it is what keeps a rename
/// from silently emptying a game's configuration.
fn ibn_khareeta<'a>(
    khareeta: &'a BTreeMap<String, QeemaYaml>,
    miftah: &str,
) -> Option<&'a QeemaYaml> {
    if let Some(qeema) = khareeta.get(miftah) {
        return Some(qeema);
    }
    let matlub = miftah.to_ascii_lowercase();
    khareeta
        .iter()
        .find(|(mawjud, _)| mawjud.to_ascii_lowercase() == matlub)
        .map(|(_, qeema)| qeema)
}

/// Reads and parses a configuration file, bounded at [`HADD_WATHIQA`] bytes.
///
/// Returns nothing for a file that is absent and for one that cannot be opened,
/// and the two are deliberately not distinguished: every caller treats them
/// identically — this game has no configuration — and every caller already names
/// the path it tried in whatever it reports.
///
/// A UTF-8 byte order mark is stripped. Bytes that are not valid UTF-8 are
/// replaced rather than rejected, because a bottle whose name was typed in a
/// legacy code page must not cost the user every program inside it.
pub(crate) fn iqra_yaml(masar: &Path) -> Option<WathiqatYaml> {
    let malaf = std::fs::File::open(masar).ok()?;
    let mut bayt = Vec::new();
    let _ = malaf.take(HADD_WATHIQA).read_to_end(&mut bayt).ok()?;
    let bila_alama = bayt.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bayt.as_slice());
    Some(hallil_yaml(&String::from_utf8_lossy(bila_alama)))
}

/// Parses the block-mapping subset of YAML that Lutris and Bottles write.
///
/// ## What is accepted
///
/// | construct | example |
/// | --- | --- |
/// | a block mapping at the document root | `Runner: sys-wine-9.0` |
/// | a nested block mapping opened by a bare key | `game:` then a more-indented `exe: /x/y` |
/// | a plain scalar | `version: lutris-fshack-7.2` |
/// | a single-quoted scalar, with `''` for a literal quote | `args: '--fullscreen'` |
/// | a double-quoted scalar, with `\\ \" \n \r \t \0 \a \b \f \v \e \/ \xHH \uHHHH \UHHHHHHHH` | `name: "Baldur\u0027s Gate"` |
/// | an explicit null: an empty value, `~`, `null`, `Null`, `NULL` | `save_path:` |
/// | a quoted key | `'C:\Games': …` |
/// | a whole-line comment, and a trailing comment introduced by a space and `#` | `exe: game.exe # the launcher` |
/// | a leading `---` document marker, and a trailing `...` | |
/// | CRLF line endings, and a UTF-8 byte order mark | |
///
/// Keys are kept verbatim and looked up exactly first, then
/// case-insensitively. A key that appears twice keeps the **last** value, which
/// is what `PyYAML` does and therefore what both writers assume.
///
/// ## What is refused, by name
///
/// Each of these produces a [`QeemaYaml::Marfuda`] carrying the construct and
/// the line, and every accessor returns nothing for it. Nothing is guessed, and
/// nothing is silently dropped.
///
/// | construct | example | why it is refused rather than approximated |
/// | --- | --- | --- |
/// | an anchor | `beea: &pfx /home/u/pfx` | the value is defined here and used elsewhere; reading it here and not there would make two keys disagree |
/// | an alias | `beea: *pfx` | resolving it means implementing anchors, and returning the literal `*pfx` would be a path that does not exist |
/// | a merge key | `<<: *defaults` | the same, with the additional problem that it changes sibling keys |
/// | a tag | `beea: !!str /home/u/pfx` | a tag changes what the scalar *is*, and ignoring it is how `no` becomes a path |
/// | a multi-line scalar | `script: \|` then indented lines | folding and chomping have four variants and none of the six values this reader wants is ever one |
/// | a flow collection | `wine: {version: ge}` or `urls: [a, b]` | a nested flow parser is most of a YAML implementation, and neither writer puts a value this reader needs in one |
/// | a block sequence | `Installed_Dependencies:` then `- vcredist2019` | there is no list among the six values, and the sequence is consumed as a unit so one refusal is recorded rather than one per item |
/// | an explicit key | `? long key` then `: value` | |
/// | a tab in the indentation | | YAML forbids it, and treating it as one space or as eight changes the tree |
/// | inconsistent indentation | a child indented further than its siblings | the two readings differ, and picking one would be a guess |
/// | nesting past [`HADD_UMQ_YAML`] | | |
/// | a second document | a `---` after content, or a `...` | a configuration file with two documents is a file something else wrote |
/// | an unterminated quoted scalar | `name: "Game` | |
/// | a line that is not a mapping entry | `just some text` | |
///
/// ## What is deliberately not implemented at all
///
/// Type inference. Every scalar comes out as text, exactly as written. This
/// reader never decides that `9.0` is a float or that `no` is a boolean, both of
/// which YAML 1.1 does and both of which have corrupted real configuration for
/// real people. A caller that wants a number parses the text itself and can say
/// so when it fails.
pub(crate) fn hallil_yaml(nass: &str) -> WathiqatYaml {
    let mut muhallil = MuhallilYaml::jadeed(nass);
    // The shallowest line in the document is the root level. Taking the *first*
    // line's indentation instead would be right for both writers and wrong for a
    // file whose first entry happens to be indented — the mapping would then
    // stop at the first line that dedents to column zero, silently losing every
    // key after it. The minimum has no such cliff: a document that really is
    // consistently indented reads identically, and an inconsistent one produces
    // a visible refusal instead of a truncation.
    let izaha = muhallil.sutur.iter().map(|satr| satr.izaha).min().unwrap_or(0);
    let judhur = muhallil.khareeta(izaha, 0);
    WathiqatYaml { judhur, marfudat: muhallil.marfudat }
}

/// One line, after comments, blanks and line endings have been dealt with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SatrYaml<'a> {
    /// The one-based line number in the original file.
    raqm: usize,
    /// How many spaces it is indented by.
    izaha: usize,
    /// The line with its indentation and trailing whitespace removed.
    nass: &'a str,
}

/// The reader's state: the prepared lines, a cursor, and what it refused.
#[derive(Debug)]
struct MuhallilYaml<'a> {
    /// Every line worth looking at, in order.
    sutur: Vec<SatrYaml<'a>>,
    /// How far through them the reader is.
    mawqi: usize,
    /// Everything refused so far.
    marfudat: Vec<RafdYaml>,
    /// How many mapping entries have been built, against [`HADD_UQAD_YAML`].
    adad: usize,
}

impl<'a> MuhallilYaml<'a> {
    /// Prepares a document: drops blanks and comment lines, refuses tabs in
    /// indentation, and stops at a second document.
    fn jadeed(nass: &'a str) -> Self {
        let mut sutur = Vec::new();
        let mut marfudat = Vec::new();
        let mut bada = false;

        for (fahras, kham) in nass.lines().take(HADD_SUTUR_YAML).enumerate() {
            let raqm = fahras.saturating_add(1);
            let bila_masafat = kham.trim_start_matches(' ');

            if bila_masafat.starts_with('\t') {
                sajjil(&mut marfudat, RafdYaml { bina: "a tab in the indentation", satr: raqm });
                continue;
            }

            let izaha = kham.len().saturating_sub(bila_masafat.len());
            let jism = bila_masafat.trim_end();
            if jism.is_empty() || jism.starts_with('#') {
                continue;
            }

            // `...` ends a document, and a `---` after content starts a second
            // one. Either way there is nothing further this reader will read: a
            // configuration file with two documents in it was written by
            // something other than Lutris or Bottles, and merging them would
            // invent a document neither of them wrote.
            if jism == "..." {
                break;
            }
            if jism == "---" || jism.starts_with("--- ") {
                if bada {
                    sajjil(&mut marfudat, RafdYaml { bina: "a second document", satr: raqm });
                    break;
                }
                continue;
            }

            bada = true;
            sutur.push(SatrYaml { raqm, izaha, nass: jism });
        }

        Self { sutur, mawqi: 0, marfudat, adad: 0 }
    }

    /// The line the cursor is on.
    fn hali(&self) -> Option<SatrYaml<'a>> {
        self.sutur.get(self.mawqi).copied()
    }

    /// Records a refusal, up to the cap.
    fn sajjil(&mut self, rafd: RafdYaml) {
        sajjil(&mut self.marfudat, rafd);
    }

    /// Consumes every line more indented than `izaha`.
    fn takhatta(&mut self, izaha: usize) {
        while let Some(satr) = self.hali() {
            if satr.izaha <= izaha {
                break;
            }
            self.mawqi = self.mawqi.saturating_add(1);
        }
    }

    /// Consumes a block sequence belonging to a key indented at `izaha`.
    ///
    /// Both indentations are consumed, because YAML allows a sequence to be
    /// written either under its key or level with it, and both writers use the
    /// level form: `Installed_Dependencies:` at column zero with `- vcredist`
    /// also at column zero is what `yaml.dump` produces by default.
    fn takhatta_qaima(&mut self, izaha: usize) {
        while let Some(satr) = self.hali() {
            let tabi = satr.izaha > izaha || (satr.izaha == izaha && satr.nass.starts_with('-'));
            if !tabi {
                break;
            }
            self.mawqi = self.mawqi.saturating_add(1);
        }
    }

    /// Reads a block mapping whose entries are indented by exactly `izaha`.
    fn khareeta(&mut self, izaha: usize, umq: usize) -> BTreeMap<String, QeemaYaml> {
        let mut natija: BTreeMap<String, QeemaYaml> = BTreeMap::new();

        while let Some(satr) = self.hali() {
            if satr.izaha < izaha {
                break;
            }
            if satr.izaha > izaha {
                self.sajjil(RafdYaml { bina: "an inconsistently indented block", satr: satr.raqm });
                self.takhatta(izaha);
                continue;
            }

            // A sequence where a mapping entry was expected: the document, or
            // this level of it, is a list. Consumed as one unit so that a
            // twenty-item list produces one refusal rather than twenty.
            if satr.nass.starts_with('-') {
                self.sajjil(RafdYaml { bina: "a block sequence", satr: satr.raqm });
                self.takhatta_qaima(izaha);
                continue;
            }

            self.mawqi = self.mawqi.saturating_add(1);

            if satr.nass.starts_with('?') {
                self.sajjil(RafdYaml { bina: "an explicit key", satr: satr.raqm });
                self.takhatta(izaha);
                continue;
            }

            let Some((miftah, baqi)) = qassim_madkhal(satr.nass) else {
                self.sajjil(RafdYaml {
                    bina: "a line that is not a mapping entry",
                    satr: satr.raqm,
                });
                continue;
            };

            if miftah == "<<" {
                self.sajjil(RafdYaml { bina: "a merge key", satr: satr.raqm });
                self.takhatta(izaha);
                continue;
            }

            if self.adad >= HADD_UQAD_YAML {
                self.sajjil(RafdYaml {
                    bina: "more entries than this reader keeps",
                    satr: satr.raqm,
                });
                break;
            }
            self.adad = self.adad.saturating_add(1);

            let qeema = self.qeema(baqi, izaha, umq, satr.raqm);
            // Last wins, which is what PyYAML does with a duplicate key and
            // therefore what both writers assume when they emit one.
            let _ = natija.insert(miftah, qeema);
        }

        natija
    }

    /// Reads the value of one mapping entry.
    fn qeema(&mut self, kham: &str, izaha: usize, umq: usize, raqm: usize) -> QeemaYaml {
        let munaqqa = kham.trim();
        if munaqqa.is_empty() {
            return self.qeema_mutadakhkhila(izaha, umq);
        }

        let marfuda = |bina: &'static str| RafdYaml { bina, satr: raqm };

        match munaqqa.chars().next() {
            Some('&') => {
                let rafd = marfuda("an anchor");
                self.sajjil(rafd);
                QeemaYaml::Marfuda(rafd)
            },
            Some('*') => {
                let rafd = marfuda("an alias");
                self.sajjil(rafd);
                QeemaYaml::Marfuda(rafd)
            },
            Some('!') => {
                let rafd = marfuda("a tag");
                self.sajjil(rafd);
                QeemaYaml::Marfuda(rafd)
            },
            Some('|' | '>') => {
                // The body of a block scalar is every following more-indented
                // line, and leaving it would make each of those lines look like
                // a mapping entry of the enclosing block.
                let rafd = marfuda("a multi-line scalar");
                self.sajjil(rafd);
                self.takhatta(izaha);
                QeemaYaml::Marfuda(rafd)
            },
            Some('{' | '[') => {
                let rafd = marfuda("a flow collection");
                self.sajjil(rafd);
                QeemaYaml::Marfuda(rafd)
            },
            Some('\'') => {
                if let Some((qeema, _)) = iqtabis_mufrad(munaqqa) {
                    nass_aw_faragh(qeema)
                } else {
                    let rafd = marfuda("an unterminated quoted scalar");
                    self.sajjil(rafd);
                    QeemaYaml::Marfuda(rafd)
                }
            },
            Some('"') => {
                if let Some((qeema, _)) = iqtabis_muzdawij(munaqqa) {
                    nass_aw_faragh(qeema)
                } else {
                    let rafd = marfuda("an unterminated quoted scalar");
                    self.sajjil(rafd);
                    QeemaYaml::Marfuda(rafd)
                }
            },
            _ => qeema_sada(munaqqa),
        }
    }

    /// Reads whatever follows a bare `key:`.
    fn qeema_mutadakhkhila(&mut self, izaha: usize, umq: usize) -> QeemaYaml {
        let Some(talii) = self.hali() else {
            return QeemaYaml::Faragh;
        };

        // A sequence written level with its own key belongs to that key, so it
        // is consumed and refused here rather than being met again by the
        // enclosing mapping and refused a second time.
        if talii.nass.starts_with('-') && talii.izaha >= izaha {
            let rafd = RafdYaml { bina: "a block sequence", satr: talii.raqm };
            self.sajjil(rafd);
            self.takhatta_qaima(izaha);
            return QeemaYaml::Marfuda(rafd);
        }

        if talii.izaha <= izaha {
            return QeemaYaml::Faragh;
        }

        if umq >= HADD_UMQ_YAML {
            let rafd = RafdYaml {
                bina: "a block nested deeper than this reader follows",
                satr: talii.raqm,
            };
            self.sajjil(rafd);
            self.takhatta(izaha);
            return QeemaYaml::Marfuda(rafd);
        }

        QeemaYaml::Khareeta(self.khareeta(talii.izaha, umq.saturating_add(1)))
    }
}

/// Appends a refusal while honouring [`HADD_MARFUDAT_YAML`].
fn sajjil(marfudat: &mut Vec<RafdYaml>, rafd: RafdYaml) {
    if marfudat.len() < HADD_MARFUDAT_YAML {
        marfudat.push(rafd);
    }
}

/// A scalar that turned out to be empty is a null, not an empty string.
fn nass_aw_faragh(qeema: String) -> QeemaYaml {
    if qeema.is_empty() { QeemaYaml::Faragh } else { QeemaYaml::Nass(qeema) }
}

/// Reads a plain scalar: trailing comment removed, null spellings recognised.
///
/// The trailing-comment rule is YAML's own and it is not optional: a `#` only
/// starts a comment when whitespace precedes it. Cutting at every `#` would
/// truncate `path: C:\Games\Hash#1\game.exe`, and never cutting would leave the
/// comment inside the value — and `PyYAML`, which wrote the file, applies exactly
/// this rule.
fn qeema_sada(kham: &str) -> QeemaYaml {
    let munaqqa = qass_taliq(kham).trim();
    if munaqqa.is_empty() || matches!(munaqqa, "~" | "null" | "Null" | "NULL") {
        QeemaYaml::Faragh
    } else {
        QeemaYaml::Nass(munaqqa.to_owned())
    }
}

/// Cuts a plain scalar at a trailing comment.
fn qass_taliq(kham: &str) -> &str {
    let mut baada_masafa = false;
    for (mawqi, harf) in kham.char_indices() {
        if harf == '#' && baada_masafa {
            return kham.get(..mawqi).unwrap_or(kham);
        }
        baada_masafa = harf == ' ' || harf == '\t';
    }
    kham
}

/// Splits one line into its key and the text of its value.
///
/// A plain key ends at the first `:` that is followed by a space or by the end
/// of the line — which is YAML's rule and the reason `path: C:\Games\x` reads
/// correctly: the colon after the drive letter is followed by a backslash, so it
/// is part of the value rather than a second separator. A quoted key is read as
/// a quoted scalar and the `:` must follow it.
///
/// Returns nothing for `key:value` with no space, which YAML considers a plain
/// scalar rather than a mapping entry, and which the caller records as a line
/// that is not a mapping entry.
fn qassim_madkhal(jism: &str) -> Option<(String, &str)> {
    if jism.starts_with('\'') || jism.starts_with('"') {
        let (miftah, baad) = if jism.starts_with('\'') {
            iqtabis_mufrad(jism)?
        } else {
            iqtabis_muzdawij(jism)?
        };
        let baqi = jism.get(baad..)?.trim_start();
        return Some((miftah, baqi.strip_prefix(':')?));
    }

    for (mawqi, harf) in jism.char_indices() {
        if harf != ':' {
            continue;
        }
        // The colon is one byte, so the next byte is a character boundary.
        let baqi = jism.get(mawqi.saturating_add(1)..)?;
        if !baqi.is_empty() && !baqi.starts_with(' ') {
            continue;
        }
        let miftah = jism.get(..mawqi)?.trim();
        if miftah.is_empty() {
            return None;
        }
        return Some((miftah.to_owned(), baqi));
    }
    None
}

/// Reads a single-quoted scalar, returning it and the byte just past its closing
/// quote.
///
/// The only escape single quotes have is `''` for a literal quote, which is why
/// Lutris and Bottles both prefer them for Windows paths: `'C:\Program
/// Files\Game'` needs no backslash doubling and means exactly what it says.
fn iqtabis_mufrad(kham: &str) -> Option<(String, usize)> {
    let mut huruf = kham.char_indices().peekable();
    if !matches!(huruf.next(), Some((_, '\''))) {
        return None;
    }
    let mut natija = String::new();
    while let Some((mawqi, harf)) = huruf.next() {
        if harf != '\'' {
            natija.push(harf);
            continue;
        }
        if matches!(huruf.peek(), Some((_, '\''))) {
            let _ = huruf.next();
            natija.push('\'');
            continue;
        }
        return Some((natija, mawqi.saturating_add(1)));
    }
    None
}

/// Reads a double-quoted scalar, returning it and the byte just past its closing
/// quote.
///
/// An escape this reader does not know yields the escaped character itself,
/// which is what every YAML implementation does and what keeps a Windows path
/// written with single backslashes inside double quotes — which `PyYAML` never
/// emits but a hand-edited file routinely contains — from losing its separators.
fn iqtabis_muzdawij(kham: &str) -> Option<(String, usize)> {
    let mut huruf = kham.char_indices();
    if !matches!(huruf.next(), Some((_, '"'))) {
        return None;
    }
    let mut natija = String::new();
    while let Some((mawqi, harf)) = huruf.next() {
        match harf {
            '"' => return Some((natija, mawqi.saturating_add(1))),
            '\\' => {
                let (_, taali) = huruf.next()?;
                match taali {
                    'n' => natija.push('\n'),
                    'r' => natija.push('\r'),
                    't' => natija.push('\t'),
                    '0' => natija.push('\0'),
                    'a' => natija.push('\u{7}'),
                    'b' => natija.push('\u{8}'),
                    'f' => natija.push('\u{c}'),
                    'v' => natija.push('\u{b}'),
                    'e' => natija.push('\u{1b}'),
                    'x' => natija.push(harf_min_hex(&mut huruf, 2)?),
                    'u' => natija.push(harf_min_hex(&mut huruf, 4)?),
                    'U' => natija.push(harf_min_hex(&mut huruf, 8)?),
                    akhar => natija.push(akhar),
                }
            },
            _ => natija.push(harf),
        }
    }
    None
}

/// Reads a fixed number of hexadecimal digits as one character.
///
/// A surrogate or an out-of-range value yields nothing, which refuses the whole
/// scalar rather than substituting a replacement character — a path with a
/// replacement character in it is a path that does not exist, and reporting it
/// as the game's executable would send every later phase somewhere real files
/// are not.
fn harf_min_hex(huruf: &mut std::str::CharIndices<'_>, adad: usize) -> Option<char> {
    let mut qeema: u32 = 0;
    for _ in 0..adad {
        let (_, harf) = huruf.next()?;
        let raqam = harf.to_digit(16)?;
        qeema = qeema.checked_mul(16)?.checked_add(raqam)?;
    }
    char::from_u32(qeema)
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
    fn judhur_lutris_min_khazain_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let bayanat = masrah.path().join("khazinat-bayanat");
        let idadat = masrah.path().join("khazinat-idadat");
        let makhbaa = masrah.path().join("khazinat-makhbaa");
        fs::create_dir_all(bayanat.join(MUJALLAD_LUTRIS))?;
        fs::write(bayanat.join(MUJALLAD_LUTRIS).join(ISM_QAIDA), [])?;

        // Three separate directories, none of them derivable from the home: a
        // resolver that still read `$XDG_DATA_HOME` inline could not be pointed
        // at any of them, because since edition 2024 setting one from a test is
        // an `unsafe` mutation of the whole process.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, masrah.path());
        siyaq.khazina_bayanat = bayanat.clone();
        siyaq.khazina_idadat = idadat.clone();
        siyaq.khazina_makhbaa = makhbaa.clone();

        assert_eq!(
            MatjarLutris::jadeed().mawqi(&siyaq),
            Some(bayanat.join(MUJALLAD_LUTRIS)),
            "the data root must come from the context"
        );

        // All four, not just the one holding the catalogue: a scan that took the
        // data root from the context and the config root from `~/.config` would
        // find games and none of their executables.
        let muhtamala = JudhurLutris::muhtamala(&siyaq);
        let awwal = muhtamala.first().ok_or("no candidate installation was built")?;
        assert_eq!(awwal.bayanat_am, bayanat);
        assert_eq!(awwal.idadat, idadat.join(MUJALLAD_LUTRIS));
        assert_eq!(awwal.makhbaa, makhbaa.join(MUJALLAD_LUTRIS));
        Ok(())
    }
}
