//! الوصل — the connection to `taarib.db`, and the guarantees every connection
//! carries before it is handed out.
//!
//! (`wasl` here is the plain sense of the word — a link, a connection. It is not
//! `saff::wasl`, which is Arabic letter joining. Same root, different subject.)
//!
//! One pool, opened once, shared by every crate that persists anything. The path
//! comes from `usus::masarat` and is never assembled here; the pragmas are
//! applied when a connection is created and are therefore true for its whole
//! life, rather than being assumed and discovered to be false at the first
//! constraint that should have fired.
//!
//! ## Why WAL matters here in particular
//!
//! Taarib's read and write patterns overlap constantly, and on the one screen
//! that is not allowed to stutter. The library grid reads `luba`, `masdar_luba`
//! and `ruqaa` continuously while the user scrolls, and a discovery scan streams
//! hundreds of inserts into those same tables at the same time. Under `SQLite`'s
//! default rollback journal a writer takes an exclusive lock over the whole
//! database for the duration of its transaction: every reader blocks, the grid's
//! query waits, the frame that needed it is missed, and the user sees a scan
//! make their library judder. WAL inverts that — the writer appends to a
//! separate log while readers keep serving a consistent snapshot from the main
//! file, so a scan and a scroll do not see each other at all.
//!
//! WAL costs one thing worth naming: the database becomes three files
//! (`taarib.db`, `-wal`, `-shm`) and it does not work over a network filesystem,
//! because it needs shared memory the network layer cannot provide. If the
//! journal mode does not take — which is exactly what happens when a user's
//! profile is on an SMB share — the store says so in the log and carries on in
//! the mode `SQLite` fell back to, because a Taarib that refuses to open is worse
//! for that user than a Taarib that is occasionally slow.
//!
//! ## `synchronous = NORMAL`, and why that is the right trade here
//!
//! In WAL mode `NORMAL` means the write-ahead log is not fsynced on every
//! commit; it is synced at checkpoints. The exposure is precise and worth
//! stating exactly: **a power loss or a kernel panic can lose the most recent
//! transactions. It cannot corrupt the database.** `FULL` would buy back those
//! last few transactions at the cost of an fsync per commit, and a scan that
//! inserts four hundred games would pay four hundred device syncs — seconds of
//! wall time on a spinning disk, on the very first screen the user ever sees.
//!
//! That trade is only defensible because of what is actually in these tables. A
//! lost transaction here costs a rescan: the games come back, the probe re-runs,
//! the registry re-caches. Nothing in this database is the only copy of anything
//! — patch files live under `ruqaa/`, backups under `nusakh/`, projects under
//! `mashari/`, all written through `masarat::kitaba_dharra`, which does fsync,
//! because those *are* the only copy. The database indexes them; it does not
//! hold them.
//!
//! ## Every write goes through a transaction
//!
//! Not a convention — [`Makhzan::bi_muamala`] is the only write path this crate
//! exposes, and the ledgers in [`crate::sijillat`] take a
//! [`rusqlite::Transaction`] rather than a connection, so there is no signature
//! anywhere that lets a caller write outside one. Two reasons:
//!
//! * A game and its launcher identities, its build and its artwork are one fact.
//!   Half of it committed is a game in the grid that belongs to no store and
//!   cannot be matched to a patch.
//! * `luba` and `bina` reference each other, and the constraint between them is
//!   `DEFERRABLE INITIALLY DEFERRED` — it is checked at commit. Outside a
//!   transaction there is no commit to check at, so every insert would have to
//!   be ordered by hand and would fail the moment somebody reordered them.
//!
//! Write transactions are `IMMEDIATE`. A deferred transaction that reads first
//! and writes later has to upgrade its lock, and `SQLite` does **not** invoke the
//! busy handler for that upgrade — it returns `SQLITE_BUSY` at once, so the busy
//! timeout configured below would not apply and the write would simply fail
//! under exactly the contention it exists to survive. Taking the write lock up
//! front makes the timeout do its job.
//!
//! ## `rusqlite` is synchronous. This crate never makes it otherwise.
//!
//! Every function here blocks the thread it is called on, and there is no
//! `tokio` dependency in this crate to hide that with. Studio's backend is
//! async, so **every call into this crate happens on a blocking thread** —
//! `tauri::async_runtime::spawn_blocking` or `tokio::task::spawn_blocking`,
//! chosen by the caller, never by the store. A store that spawned its own
//! threads would own a runtime that the application already owns, would size a
//! thread pool the application already sized, and would make a library that a
//! command-line tool or a test harness could not use without dragging an async
//! runtime in behind it.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, Transaction, TransactionBehavior};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::{self, Masarat};

use crate::hijra;
use crate::khata::KhataMakhzan;

/// How long a statement waits for another writer before giving up.
///
/// Ten seconds sounds enormous for a local file and is not: an install writing a
/// large manifest, a scan committing four hundred games, and a registry refresh
/// can all land in the same second on a machine whose disk is busy doing
/// something else entirely. The alternative to waiting is failing an operation
/// the user asked for, in order to save nine seconds they were not watching.
pub const MUHLA_INSHIGHAL: Duration = Duration::from_secs(10);

/// The same timeout in whole seconds, for the sentences errors carry.
const MUHLA_THAWAN: u64 = MUHLA_INSHIGHAL.as_secs();

/// How many connections the pool will open.
///
/// Sized for readers. WAL permits exactly one writer at a time no matter how
/// many connections exist, so raising this would not widen the write path; what
/// it buys is the grid, the detail screen, the diagnostics buffer and a
/// background refresh each holding a connection without queueing behind one
/// another.
pub const ADAD_ITTISALAT: u32 = 8;

/// Prepared statements kept per connection.
///
/// A discovery scan runs the same handful of statements once per discovered
/// game — upsert the game, upsert each of its launcher identities, record the
/// build, stamp the scan generation. Re-preparing those for every row of a
/// four-hundred-game library is four hundred parses of identical SQL. The cache
/// is per connection, so this is the working set of one thread, not of the
/// application.
pub const SAAT_JUMAL: usize = 64;

/// The per-connection configuration, applied at creation.
///
/// A pooled connection keeps its pragmas for its whole life, so creation is the
/// only moment that matters; re-running these on every checkout would add three
/// round trips to every query for no change in behaviour. `busy_timeout` is set
/// separately through `rusqlite`'s typed API so that the duration lives in one
/// constant rather than in a constant and a number inside a string.
const TAHYIA: &str = "\
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = NORMAL;
PRAGMA temp_store = MEMORY;
PRAGMA cache_size = -8192;";

/// A pooled connection, checked out for the duration of one operation.
pub type IttisalMakhzan = r2d2::PooledConnection<SqliteConnectionManager>;

/// The local store: one WAL-mode `SQLite` database, one pool, one schema.
#[derive(Clone)]
pub struct Makhzan {
    birka: r2d2::Pool<SqliteConnectionManager>,
    masar: PathBuf,
}

impl fmt::Debug for Makhzan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let halat = self.birka.state();
        f.debug_struct("Makhzan")
            .field("masar", &self.masar)
            .field("ittisalat", &halat.connections)
            .field("khamila", &halat.idle_connections)
            .finish()
    }
}

impl Makhzan {
    /// Opens the store at the location `usus::masarat` resolved for this
    /// machine, creating it if it is not there and migrating it if it is behind.
    ///
    /// # Errors
    ///
    /// Fails when the data directory cannot be created, when the file cannot be
    /// opened or configured, or when the schema cannot be brought forward — in
    /// particular when the file was written by a newer build of Taarib, which is
    /// a refusal and not a repair. See [`crate::hijra::rahhil`].
    pub fn iftah(masarat: &Masarat) -> Natija<Self> {
        Self::min_masar(&masarat.qaida_bayanat())
    }

    /// Opens the store at an explicit path.
    ///
    /// For the owner's sandbox and for portable installations, both of which
    /// have a real data root that is simply not the machine's default one. The
    /// path still comes from a [`Masarat`] in every normal caller; this exists
    /// so that the sandbox does not have to reimplement opening.
    ///
    /// # Errors
    ///
    /// As [`Self::iftah`].
    pub fn min_masar(masar: &Path) -> Natija<Self> {
        if let Some(mujallad) = masar.parent() {
            masarat::insha_mujallad(mujallad)?;
        }

        let mudir = SqliteConnectionManager::file(masar).with_init(|ittisal| {
            ittisal.busy_timeout(MUHLA_INSHIGHAL)?;
            ittisal.execute_batch(TAHYIA)?;
            ittisal.set_prepared_statement_cache_capacity(SAAT_JUMAL);
            Ok(())
        });

        let birka = r2d2::Pool::builder()
            .max_size(ADAD_ITTISALAT)
            .min_idle(Some(1))
            .connection_timeout(MUHLA_INSHIGHAL)
            .build(mudir)
            .map_err(|q| {
                Khata::from(KhataMakhzan::TaadhurBirka {
                    muhla_thawan: MUHLA_THAWAN,
                    sabab: q,
                })
                .ma("masar", masar)
            })?;

        let makhzan = Self {
            birka,
            masar: masar.to_path_buf(),
        };

        {
            let mut ittisal = makhzan.ittisal()?;
            makhzan.tahaqquq_namat_sijill(&ittisal);
            let taqreer = hijra::rahhil(&mut ittisal)?;
            if taqreer.taghayyara() {
                tracing::info!(
                    min = taqreer.min_isdar,
                    ila = taqreer.ila_isdar,
                    "database schema brought forward"
                );
            }
        }

        Ok(makhzan)
    }

    /// The database file this store was opened from.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// Checks a connection out of the pool for read work.
    ///
    /// Blocks the calling thread. Callers inside Studio are already on a
    /// blocking thread; see this module's documentation for why that is the
    /// caller's job and not this crate's.
    ///
    /// # Errors
    ///
    /// Returns [`KhataMakhzan::TaadhurBirka`] when every connection is in use
    /// for longer than [`MUHLA_INSHIGHAL`], which on a machine that is not
    /// deadlocked means the pool is undersized rather than that anything failed.
    pub fn ittisal(&self) -> Natija<IttisalMakhzan> {
        self.birka.get().map_err(|q| {
            Khata::from(KhataMakhzan::TaadhurBirka {
                muhla_thawan: MUHLA_THAWAN,
                sabab: q,
            })
            .ma("masar", self.masar.clone())
        })
    }

    /// Runs a unit of write work inside one `IMMEDIATE` transaction.
    ///
    /// The only write path this crate offers. The closure receives the
    /// transaction; every ledger in [`crate::sijillat`] takes one, so a caller
    /// composes as many ledger calls as the operation needs and they all commit
    /// or none of them do. Returning `Err` from the closure rolls the whole
    /// thing back, including the parts that had already succeeded.
    ///
    /// # Errors
    ///
    /// Returns [`KhataMakhzan::TaadhurMuamala`] when the transaction cannot be
    /// started or committed, whatever the closure returned when it fails, and
    /// [`KhataMakhzan::QaidaMuqfala`] when another writer held the database past
    /// the busy timeout.
    pub fn bi_muamala<T, F>(&self, amal: F) -> Natija<T>
    where
        F: FnOnce(&Transaction<'_>) -> Natija<T>,
    {
        let mut ittisal = self.ittisal()?;
        let muamala = ittisal
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|q| {
                Khata::from(KhataMakhzan::min_rusqlite(
                    "begin",
                    "muamala",
                    MUHLA_THAWAN,
                    q,
                ))
            })?;

        let natija = amal(&muamala)?;

        muamala.commit().map_err(|q| {
            Khata::from(KhataMakhzan::min_rusqlite(
                "commit",
                "muamala",
                MUHLA_THAWAN,
                q,
            ))
        })?;

        Ok(natija)
    }

    /// Runs a unit of read work on a pooled connection.
    ///
    /// Reads do not need a transaction to be consistent under WAL: a statement
    /// sees a snapshot. This exists so that read code reads the same as write
    /// code at the call site.
    ///
    /// # Errors
    ///
    /// As [`Self::ittisal`], plus whatever the closure returns.
    pub fn bil_qira<T, F>(&self, amal: F) -> Natija<T>
    where
        F: FnOnce(&Connection) -> Natija<T>,
    {
        let ittisal = self.ittisal()?;
        amal(&ittisal)
    }

    /// Runs `SQLite`'s own integrity check over every page.
    ///
    /// Reports what it finds and repairs nothing. This is deliberate and it is
    /// the crate's third hard constraint: a store that reacted to damage by
    /// recreating the file would, in the one situation where a user most needs
    /// their data, delete it. What a damaged database needs is a backup and a
    /// person, and both of those are reachable from the error this returns.
    ///
    /// # Errors
    ///
    /// Returns [`KhataMakhzan::QaidaTalifa`] carrying every line the check
    /// reported, and [`KhataMakhzan::TaadhurJumla`] when the check itself could
    /// not run.
    pub fn tahaqquq_salama(&self) -> Natija<()> {
        let ittisal = self.ittisal()?;
        let mut jumla = ittisal.prepare("PRAGMA integrity_check").map_err(|q| {
            Khata::from(KhataMakhzan::min_rusqlite(
                "prepare",
                "integrity_check",
                MUHLA_THAWAN,
                q,
            ))
        })?;

        let sufuf = jumla
            .query_map([], |saf| saf.get::<_, String>(0))
            .map_err(|q| {
                Khata::from(KhataMakhzan::min_rusqlite(
                    "query",
                    "integrity_check",
                    MUHLA_THAWAN,
                    q,
                ))
            })?;

        let mut satr_khata = Vec::new();
        for saf in sufuf {
            let nass = saf.map_err(|q| {
                Khata::from(KhataMakhzan::min_rusqlite(
                    "read",
                    "integrity_check",
                    MUHLA_THAWAN,
                    q,
                ))
            })?;
            if !nass.eq_ignore_ascii_case("ok") {
                satr_khata.push(nass);
            }
        }

        if satr_khata.is_empty() {
            Ok(())
        } else {
            Err(Khata::from(KhataMakhzan::QaidaTalifa {
                tafsil: satr_khata.join("; "),
            })
            .ma("masar", self.masar.clone())
            .ma("tafasil", satr_khata))
        }
    }

    /// Copies the live database to another file through `SQLite`'s own backup API.
    ///
    /// The backup API copies page by page while other connections keep reading
    /// and writing, and it restarts if a write invalidates what it has copied so
    /// far. Copying the file with the filesystem instead would produce a torn
    /// image whenever a write landed mid-copy, and in WAL mode it would also
    /// miss everything still sitting in the `-wal` file.
    ///
    /// # Errors
    ///
    /// Fails when the destination directory cannot be created or the copy cannot
    /// complete.
    pub fn insakh(&self, ila: &Path) -> Natija<()> {
        if let Some(mujallad) = ila.parent() {
            masarat::insha_mujallad(mujallad)?;
        }
        let ittisal = self.ittisal()?;
        ittisal.backup(rusqlite::MAIN_DB, ila, None).map_err(|q| {
            Khata::from(KhataMakhzan::min_rusqlite(
                "backup",
                "main",
                MUHLA_THAWAN,
                q,
            ))
            .ma("ila", ila)
        })
    }

    /// Checkpoints the write-ahead log and reclaims free pages.
    ///
    /// Worth running after a large deletion — a purged registry cache, a
    /// discarded project — and worth never running on a schedule: `VACUUM`
    /// rewrites the entire file, which on a database holding a translation
    /// memory is real work for no benefit if nothing was freed.
    ///
    /// # Errors
    ///
    /// Fails when the checkpoint or the vacuum cannot run, which is normally
    /// because another connection is mid-transaction.
    pub fn idghat(&self) -> Natija<()> {
        let ittisal = self.ittisal()?;
        ittisal
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|q| {
                Khata::from(KhataMakhzan::min_rusqlite(
                    "checkpoint",
                    "main",
                    MUHLA_THAWAN,
                    q,
                ))
            })?;
        ittisal.execute_batch("VACUUM;").map_err(|q| {
            Khata::from(KhataMakhzan::min_rusqlite(
                "vacuum",
                "main",
                MUHLA_THAWAN,
                q,
            ))
        })
    }

    /// What the store currently costs, for the Diagnostics screen.
    ///
    /// # Errors
    ///
    /// Fails when the page counters cannot be read.
    pub fn ihsaat(&self) -> Natija<IhsaatMakhzan> {
        let ittisal = self.ittisal()?;
        let safahat = raqm_pragma(&ittisal, "PRAGMA page_count")?;
        let hajm_safha = raqm_pragma(&ittisal, "PRAGMA page_size")?;
        let hurra = raqm_pragma(&ittisal, "PRAGMA freelist_count")?;
        let halat = self.birka.state();

        Ok(IhsaatMakhzan {
            safahat,
            hajm_safha,
            safahat_hurra: hurra,
            bayt: safahat.saturating_mul(hajm_safha),
            ittisalat: halat.connections,
            khamila: halat.idle_connections,
        })
    }

    /// Warns when the journal mode is not the one this store was designed
    /// around, without refusing to run in it.
    fn tahaqquq_namat_sijill(&self, ittisal: &Connection) {
        let namat: Result<String, rusqlite::Error> =
            ittisal.query_row("PRAGMA journal_mode", [], |saf| saf.get(0));
        match namat {
            Ok(qeema) if qeema.eq_ignore_ascii_case("wal") => {},
            Ok(qeema) => tracing::warn!(
                namat = %qeema,
                masar = %self.masar.display(),
                "journal mode is not WAL; readers will block behind writers. This normally means \
                 the data directory is on a network filesystem, where WAL's shared memory is not \
                 available."
            ),
            Err(q) => tracing::warn!(sabab = %q, "could not read the journal mode"),
        }
    }
}

/// What the store costs on disk and in connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IhsaatMakhzan {
    /// Pages allocated to the database.
    pub safahat: u64,
    /// Bytes per page.
    pub hajm_safha: u64,
    /// Pages on the free list — space the file holds and is not using, which is
    /// what [`Makhzan::idghat`] reclaims.
    pub safahat_hurra: u64,
    /// The main file's size in bytes. The `-wal` file is not counted: it is
    /// transient by design and its size says more about checkpoint timing than
    /// about how much data there is.
    pub bayt: u64,
    /// Connections the pool currently holds.
    pub ittisalat: u32,
    /// How many of those are idle.
    pub khamila: u32,
}

/// Reads a single-integer pragma.
fn raqm_pragma(ittisal: &Connection, jumla: &'static str) -> Natija<u64> {
    let qeema: i64 = ittisal
        .query_row(jumla, [], |saf| saf.get(0))
        .map_err(|q| {
            Khata::from(KhataMakhzan::min_rusqlite(
                "read",
                "pragma",
                MUHLA_THAWAN,
                q,
            ))
            .ma("pragma", jumla)
        })?;
    Ok(u64::try_from(qeema).unwrap_or(0))
}

/// The current instant, RFC 3339, from `SQLite`'s own clock.
///
/// Every timestamp the store generates comes from here rather than from the
/// host's clock through a date library, for two reasons. It keeps one clock, so
/// two rows written by one transaction cannot disagree about when the
/// transaction happened. And it keeps this crate's dependency list to what its
/// manifest declares — the store does not need a calendar, it needs the string
/// the columns are documented to hold.
///
/// # Errors
///
/// Fails only when the connection cannot execute a statement at all.
pub fn alaan(ittisal: &Connection) -> Natija<String> {
    ittisal
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |saf| {
            saf.get(0)
        })
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("read", "clock", MUHLA_THAWAN, q)))
}

/// Turns a `rusqlite` failure into a store error with this crate's timeout
/// already attached.
///
/// The ledgers call this on every statement, so the operation name and the table
/// are recorded once at the call site and never assembled into a sentence.
#[must_use]
pub fn khata_jumla(amaliya: &'static str, jadwal: &'static str, sabab: rusqlite::Error) -> Khata {
    Khata::from(KhataMakhzan::min_rusqlite(
        amaliya,
        jadwal,
        MUHLA_THAWAN,
        sabab,
    ))
}
