//! # مخزن تعريب — the local store
//!
//! `taarib.db`, and the only code in the product that opens it. Every other
//! crate that persists something — discovered games, engine probe results,
//! installation manifests, patch records, registry cache, translation memory,
//! project state — reaches it through a ledger defined here rather than by
//! writing SQL of its own. One place owns the schema, so one place can migrate
//! it.
//!
//! The database file itself comes from `usus::masarat`; this crate never
//! computes its path. The types it stores come from `taarib-mustalahat`; this
//! crate never redefines a domain concept in order to persist it.
//!
//! The connection pool and the migration runner are established in **Phase 0**
//! so that Phase 4 has somewhere to write the first scan; the ledgers accrete
//! from **Phase 4** onward as each later phase brings a real thing to store.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | [`wasl`] | the connection pool: WAL journal mode, `foreign_keys` on, a busy timeout, `synchronous = NORMAL`, and the per-connection pragmas applied when a connection is made rather than hoped for. Also the online backup through `SQLite`'s own backup API, the integrity check, and compaction |
//! | [`hijra`] | forward-only schema migrations, each one numbered, named, checksummed and applied inside a single transaction, with a refusal when the file on disk declares a version newer than the running build |
//! | [`sijillat`] | the ledgers — one per aggregate: games, launcher identities, builds, engine reports, artwork, scans, patches, contributors, installation manifests, projects, translation memory, and the store's own state — each a typed accessor, none of them a generic query surface |
//! | [`khata`] | what can go wrong, in two sentences and one action, with codes from the `MAKHZAN` block |
//!
//! (`wasl` here is the plain sense — a link, a connection. `saff::wasl` is
//! Arabic letter joining. `sijillat` is the register sense — a ledger.
//! `usus::sijill` is the log. Same words, different subjects, and both senses
//! are ordinary Arabic.)
//!
//! ## Why WAL, and why the pool
//!
//! Studio reads on many threads and writes on few. A library scan streams
//! hundreds of inserts while the interface is drawing the grid from the same
//! tables, and a rollback journal would serialize those into a visible stall on
//! a screen that is not allowed to stall — the writer would hold an exclusive
//! lock over the whole database, the grid's query would wait behind it, and the
//! user would watch their library judder every time a scan ran. WAL lets readers
//! proceed against a consistent snapshot while a writer commits to a separate
//! log. [`wasl`] documents the full trade, including what WAL costs and what
//! happens when the journal mode will not take.
//!
//! `foreign_keys` is enforced, not declared. `SQLite` defaults it off per
//! connection, so it is set on every connection: a patch record whose game row
//! has been deleted is a bug that should fail at the moment it is written, not
//! surface six screens later as an empty detail page.
//!
//! ## Migrations refuse rather than guess
//!
//! [`hijra`] moves forward only. There is no down-migration, because a
//! down-migration that has to discard columns is a data-loss path dressed as a
//! convenience. A file whose schema version is newer than the running build is
//! refused with an explanation — the user is running an older Taarib against a
//! newer database, and the honest answer is to say so and stop, not to open it
//! and drop whatever cannot be read. This is the same rule
//! [`taarib_usus::mukhattat`] applies to versioned files, for the same reason,
//! and it matters more here: the thing silently dropped would be a translation
//! project nobody else has a copy of.
//!
//! A migration whose recorded checksum does not match the one this build carries
//! is refused too. That means a shipped migration was edited, which gives two
//! users different schemas behind the same version number and makes every later
//! migration a coin toss.
//!
//! ## This crate is synchronous, and stays that way
//!
//! `rusqlite` blocks. Studio's backend is async. The rule is therefore explicit
//! and belongs to the caller: **every call into this crate happens on a blocking
//! thread**, through `spawn_blocking` or its Tauri equivalent, and **this crate
//! never spawns one itself**. There is no `tokio` in its manifest and none will
//! be added. A store that owned its own runtime would size a thread pool the
//! application already sized, and would make itself unusable from a command-line
//! tool or a fixture that has no runtime at all.
//!
//! ## Hard constraints
//!
//! - Nothing outside this crate writes SQL against `taarib.db`.
//! - Nothing outside this crate computes the database path.
//! - **No statement text is ever built from a value.** Every statement is a
//!   `&'static str` constant and every value is a bound parameter — including
//!   integers. A scan number interpolated into SQL is the same defect as a game
//!   title interpolated into SQL, and the only reason the first feels safer is
//!   that nobody has tried yet. Where a query needs to vary — four sort orders
//!   for the library grid — the variants are whole constants selected by a
//!   `match`, and optional filters are parameters that neutralise their own
//!   predicate, which also keeps the prepared-statement cache useful.
//! - **A corrupt database is reported, never silently recreated.** The store
//!   does not delete, rename, truncate or rebuild the file under any condition.
//!   Losing a user's translation projects to a "let's start fresh" recovery path
//!   is the worst thing this crate could do, and the situation where it would
//!   trigger is exactly the situation where the data matters most.
//! - A migration runs inside one transaction and leaves the file either fully at
//!   the old version or fully at the new one. An interrupted upgrade is
//!   recoverable by definition.
//! - Uninstalled games are marked absent, never deleted, so a patch record, an
//!   installation manifest and a backup manifest survive a reinstall and the
//!   user gets their game back exactly as it was.
//! - Every write goes through a transaction, through [`Makhzan::bi_muamala`],
//!   which is the only writable handle the crate hands out.
//! - Statements a scan repeats once per game are prepared through the
//!   per-connection cache; statements that run once per screen are not, so a
//!   one-shot query cannot evict a hot one.
//! - Every failure carries the statement's context as structured data, not as a
//!   stringified `rusqlite::Error` handed to the interface.

pub mod hijra;
pub mod khata;
pub mod sijillat;
pub mod wasl;

pub use hijra::{HIJRAT, Hijra, ISDAR_MADUM, TaqreerHijra, isdar_hali, rahhil};
pub use khata::{KhataMakhzan, siyaq_sqlite};
pub use sijillat::{
    AmalMalaf, FahsMukhzan, HalatTathbeet, HasilatMash, IdkhalLuba, MashruMukhzan, MatjarMukhzan,
    MudkhalBayan, MudkhalDhakira, ShardMukhzan, SijillAlaab, SijillBina, SijillDhakira, SijillFahs,
    SijillHalat, SijillMashari, SijillMuharrik, SijillMusahim, SijillRuqaa, SijillSuwar,
    SijillTathbeet, SimaMukhzana, SuratMukhzana, TalabMaktaba, TanbihMukhzan, TarteebMaktaba,
    TathbeetMukhzan, miftah, sima,
};
pub use wasl::{
    ADAD_ITTISALAT, IhsaatMakhzan, IttisalMakhzan, MUHLA_INSHIGHAL, Makhzan, SAAT_JUMAL, alaan,
    khata_jumla,
};
