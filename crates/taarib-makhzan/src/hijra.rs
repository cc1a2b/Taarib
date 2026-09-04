//! الهجرة — forward-only schema migration, and the refusals that guard it.
//!
//! A migration is a number, a name, and one block of SQL. The runner applies
//! every migration the database has not yet seen, in order, each inside its own
//! transaction, and records it in the `hijrat` table with a checksum of the
//! exact statement that ran. There is no down-migration anywhere in this crate
//! and there never will be: a reverse step that has to drop a column is a
//! data-loss path wearing the costume of a convenience.
//!
//! ## Two refusals, and why each exists
//!
//! **A database newer than this build is refused, not opened.** This is exactly
//! the rule [`taarib_usus::mukhattat`] applies to versioned JSON files, for
//! exactly the same reason, and it is stated here rather than inherited so that
//! nobody has to go and look: opening a structure a build does not fully
//! understand means reading the fields it recognises, ignoring the ones it does
//! not, and then writing the survivors back — which destroys the rest
//! permanently on the first save. Refusing costs the user one update. Guessing
//! costs them their work. A database is worse than a settings file here, because
//! the thing being silently dropped is a translation project nobody else has a
//! copy of.
//!
//! **A migration whose recorded checksum does not match this build's is
//! refused.** A shipped migration that gets edited after release produces two
//! different schemas behind one version number: users who ran the old form have
//! one shape, users who ran the new form have another, and every later migration
//! is written against whichever one its author happened to have on their
//! machine. The failure is silent, arrives months later, and is unreproducible.
//! Recording the checksum turns it into a refusal at startup with the migration
//! named. The rule that follows from it is absolute: **a migration that has
//! shipped is frozen.** A correction is a new migration, never an edit.
//!
//! The checksum is FNV-1a over the statement bytes. It is a change detector, not
//! a signature: anyone able to rewrite the migration table inside the binary can
//! rewrite the constant beside it. It exists to catch a maintainer editing
//! history, which is a mistake, not an attack.
//!
//! ## Why the version lives in a table and not in `PRAGMA user_version`
//!
//! `PRAGMA` statements cannot take a bound parameter, and this crate does not
//! build SQL out of values — not even integers ([`crate`]'s second hard
//! constraint). A table costs one page and holds far more than a counter could:
//! the number, the name, the checksum, and the moment each step was applied. A
//! maintainer reading the file with `sqlite3` during a support conversation sees
//! the whole upgrade history instead of a bare `3`.

use rusqlite::{Connection, OptionalExtension as _, params};
use taarib_usus::khata::{Khata, Natija};

use crate::khata::KhataMakhzan;

/// The bookkeeping table, created before any migration runs.
///
/// Outside the numbered migrations by necessity: migration 1 has to record
/// itself somewhere, and that somewhere cannot be something migration 1 created.
const JADWAL_HIJRAT: &str = "\
CREATE TABLE IF NOT EXISTS hijrat (
    raqm  INTEGER PRIMARY KEY,
    ism   TEXT NOT NULL,
    basma TEXT NOT NULL,
    waqt  TEXT NOT NULL
) STRICT;";

/// One forward step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hijra {
    /// Its number, which is also the schema version it produces.
    pub raqm: u32,
    /// A short name, recorded so the upgrade history reads as prose.
    pub ism: &'static str,
    /// The statements, applied as one batch inside one transaction.
    pub jumal: &'static str,
}

impl Hijra {
    /// The checksum of the statements this build carries for this step.
    #[must_use]
    pub fn basma(&self) -> String {
        basma_nass(self.jumal)
    }
}

/// Migration 1 — the foundation, and the only step there has ever been.
const AL_ASAS: Hijra = Hijra { raqm: 1, ism: "al-asas", jumal: HIJRA_1 };

/// Every migration this build defines, in ascending order.
///
/// Append only. Editing an entry that has shipped changes its checksum, and
/// every database that already applied it will then refuse to open — which is
/// the point, and is far better than two schemas quietly sharing one number.
pub const HIJRAT: &[Hijra] = &[AL_ASAS];

/// The highest schema version this build understands.
///
/// A database declaring anything above this is refused by [`rahhil`] rather than
/// opened. Written as the last migration's own number rather than derived from
/// the list's length, so that adding a step is one edit in one place with no
/// cast between a length and a version.
pub const ISDAR_MADUM: u32 = AL_ASAS.raqm;

/// What a migration run did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerHijra {
    /// The version the database was at before the run.
    pub min_isdar: u32,
    /// The version it is at now.
    pub ila_isdar: u32,
    /// The steps applied, in the order they ran. Empty when nothing was due,
    /// which is the normal case on every launch after the first.
    pub mutabbaqa: Vec<u32>,
}

impl TaqreerHijra {
    /// Whether the database was actually changed.
    #[must_use]
    pub const fn taghayyara(&self) -> bool {
        !self.mutabbaqa.is_empty()
    }
}

/// The schema version recorded in an open database.
///
/// Zero means the file has never been migrated — either it is new, or it is
/// somebody else's `SQLite` database that happens to sit at Taarib's path.
///
/// # Errors
///
/// Returns [`KhataMakhzan::TaadhurJumla`] when the bookkeeping table cannot be
/// created or read, [`KhataMakhzan::QaidaTalifa`] when `SQLite` reports the file
/// as damaged, and [`KhataMakhzan::QaidaMuqfala`] when another writer holds it.
pub fn isdar_hali(ittisal: &Connection) -> Natija<u32> {
    ittisal
        .execute_batch(JADWAL_HIJRAT)
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("create", "hijrat", 0, q)))?;

    let aqsa: Option<i64> = ittisal
        .query_row("SELECT max(raqm) FROM hijrat", [], |saf| saf.get(0))
        .optional()
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("read version", "hijrat", 0, q)))?
        .flatten();

    Ok(u32::try_from(aqsa.unwrap_or(0)).unwrap_or(u32::MAX))
}

/// Brings a database up to the version this build understands.
///
/// Every step runs inside its own transaction together with the row that records
/// it, so the file is either entirely at the old version or entirely at the new
/// one. A power cut halfway through an upgrade leaves a database that opens.
///
/// Foreign keys are deferred for the duration of each step — not disabled.
/// `SQLite`'s own table-rebuild recipe tells you to turn `foreign_keys` off, but
/// that pragma is a no-op inside a transaction and turning it off around one
/// would leave a window where another connection sees enforcement disabled.
/// `defer_foreign_keys` moves the check to the commit instead, so a migration
/// may create and populate tables in any order and still cannot commit a
/// dangling reference.
///
/// # Errors
///
/// Refuses, without touching the file, when the database declares a version
/// newer than this build ([`KhataMakhzan::IsdarAhdath`]), when it records a step
/// this build does not define ([`KhataMakhzan::HijraMajhula`]), when a recorded
/// step is missing below one that is present ([`KhataMakhzan::HijraNaqisa`]), or
/// when a recorded checksum differs from the one this build carries
/// ([`KhataMakhzan::BasmaHijraMukhtalifa`]). Returns
/// [`KhataMakhzan::HijraFashila`] when a step itself fails, in which case that
/// step was rolled back whole.
pub fn rahhil(ittisal: &mut Connection) -> Natija<TaqreerHijra> {
    ittisal
        .execute_batch(JADWAL_HIJRAT)
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("create", "hijrat", 0, q)))?;

    let mutabbaqa = musajjala(ittisal)?;
    let min_isdar = tahaqquq(&mutabbaqa)?;

    let mut jarat = Vec::new();
    for hijra in HIJRAT.iter().filter(|h| h.raqm > min_isdar) {
        tabbiq(ittisal, hijra)?;
        jarat.push(hijra.raqm);
        tracing::info!(hijra = hijra.raqm, ism = hijra.ism, "schema migration applied");
    }

    Ok(TaqreerHijra { min_isdar, ila_isdar: ISDAR_MADUM, mutabbaqa: jarat })
}

/// Every step the database records, as `(number, checksum)`, in order.
fn musajjala(ittisal: &Connection) -> Natija<Vec<(u32, String)>> {
    let mut jumla = ittisal
        .prepare("SELECT raqm, basma FROM hijrat ORDER BY raqm")
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("prepare", "hijrat", 0, q)))?;

    let sufuf = jumla
        .query_map([], |saf| {
            let raqm: i64 = saf.get(0)?;
            let basma: String = saf.get(1)?;
            Ok((raqm, basma))
        })
        .map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("query", "hijrat", 0, q)))?;

    let mut natija = Vec::new();
    for saf in sufuf {
        let (raqm, basma) =
            saf.map_err(|q| Khata::from(KhataMakhzan::min_rusqlite("read", "hijrat", 0, q)))?;
        natija.push((u32::try_from(raqm).unwrap_or(u32::MAX), basma));
    }
    Ok(natija)
}

/// Checks the recorded history against the one this build carries, and returns
/// the version the database is at.
fn tahaqquq(mutabbaqa: &[(u32, String)]) -> Natija<u32> {
    let Some(aqsa) = mutabbaqa.iter().map(|(raqm, _)| *raqm).max() else {
        return Ok(0);
    };

    if aqsa > ISDAR_MADUM {
        return Err(Khata::from(KhataMakhzan::IsdarAhdath {
            mawjud: aqsa,
            madum: ISDAR_MADUM,
        }));
    }

    for (raqm, basma) in mutabbaqa {
        let Some(hijra) = HIJRAT.iter().find(|h| h.raqm == *raqm) else {
            return Err(Khata::from(KhataMakhzan::HijraMajhula { raqm: *raqm }));
        };
        let mahmula = hijra.basma();
        if mahmula != *basma {
            return Err(Khata::from(KhataMakhzan::BasmaHijraMukhtalifa {
                raqm: *raqm,
                ism: hijra.ism,
                masjala: basma.clone(),
                mahmula,
            }));
        }
    }

    // The applied set must be the contiguous prefix 1..=aqsa. A hole means the
    // schema is a shape no build produced, and running the missing step now
    // would run it against tables written by the steps that came after it.
    for hijra in HIJRAT.iter().filter(|h| h.raqm <= aqsa) {
        if !mutabbaqa.iter().any(|(raqm, _)| *raqm == hijra.raqm) {
            return Err(Khata::from(KhataMakhzan::HijraNaqisa { raqm: hijra.raqm, aqsa }));
        }
    }

    Ok(aqsa)
}

/// Applies one step, together with the row that records it, in one transaction.
fn tabbiq(ittisal: &mut Connection, hijra: &Hijra) -> Natija<()> {
    let muamala = ittisal.transaction().map_err(|q| {
        Khata::from(KhataMakhzan::TaadhurMuamala { marhala: "begun", sabab: q })
    })?;

    muamala.execute_batch("PRAGMA defer_foreign_keys = ON;").map_err(|q| {
        Khata::from(KhataMakhzan::TaadhurDabt { pragma: "defer_foreign_keys", sabab: q })
    })?;

    muamala.execute_batch(hijra.jumal).map_err(|q| {
        Khata::from(KhataMakhzan::HijraFashila { raqm: hijra.raqm, ism: hijra.ism, sabab: q })
    })?;

    let _ = muamala
        .execute(
            "INSERT INTO hijrat (raqm, ism, basma, waqt)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            params![hijra.raqm, hijra.ism, hijra.basma()],
        )
        .map_err(|q| {
            Khata::from(KhataMakhzan::HijraFashila { raqm: hijra.raqm, ism: hijra.ism, sabab: q })
        })?;

    muamala.commit().map_err(|q| {
        Khata::from(KhataMakhzan::TaadhurMuamala { marhala: "committed", sabab: q })
    })
}

/// FNV-1a over the bytes of a migration, rendered as sixteen hexadecimal
/// characters.
///
/// Chosen because it needs no dependency this crate does not already have, is
/// deterministic across platforms and compiler versions, and detects the only
/// thing it is asked to detect: that the text of a shipped migration changed.
fn basma_nass(nass: &str) -> String {
    const ASAS: u64 = 0xcbf2_9ce4_8422_2325;
    const MUDARIB: u64 = 0x0000_0100_0000_01b3;

    let mut basma = ASAS;
    for bayt in nass.as_bytes() {
        basma ^= u64::from(*bayt);
        basma = basma.wrapping_mul(MUDARIB);
    }
    format!("{basma:016x}")
}

/// Migration 1 — the whole schema Phase 4 writes into and every later phase
/// extends.
///
/// Conventions that hold across every table here, each one a decision rather
/// than a habit:
///
/// * **`STRICT` everywhere.** Without it `SQLite`'s type affinity accepts a
///   `LubaId` bound as an integer, stores it as an integer, and hands it back as
///   one months later when the game will not open. `STRICT` turns that into an
///   error at the `INSERT`, next to the code that caused it.
/// * **Timestamps are RFC 3339 text, not integers.** Four bytes per row are
///   worth less than a maintainer being able to run `sqlite3 taarib.db 'SELECT
///   ism, akhir_laab FROM luba'` during a support conversation and read the
///   answer. Text sorts correctly, compares correctly, and needs no timezone
///   convention agreed between the database, the backend and the frontend. Every
///   timestamp the store generates itself comes from `SQLite`'s own clock, so two
///   rows written in one transaction cannot disagree about when it happened.
/// * **Enumerations are their serde names, constrained by `CHECK`.** The text a
///   column holds is the same text the JSON Schema and the TypeScript type use,
///   so the database and the wire never drift; the `CHECK` means a value no
///   variant answers to cannot be written at all.
/// * **Booleans are `INTEGER` with `CHECK (x IN (0, 1))`.** `SQLite` has no
///   boolean type and will happily store `2`.
/// * **Nothing is deleted that a user could want back.** Games go absent,
///   installations go inactive, patches keep their revisions.
const HIJRA_1: &str = r#"
-- ---------------------------------------------------------------------------
-- luba — one row per game, whatever number of launchers know it.
--
-- The identity is Taarib's own LubaId (UUIDv5 over the launcher identifier and
-- the normalized name), not any launcher's id, because the same title bought on
-- Steam and claimed on Epic is one game to the player, one row here, and one
-- target for a patch. The launcher identities live in masdar_luba.
-- ---------------------------------------------------------------------------
CREATE TABLE luba (
    id                TEXT PRIMARY KEY
                      CHECK (length(id) = 36 AND id NOT GLOB '*[^0-9a-f-]*'),

    ism               TEXT NOT NULL,
    -- wahhid_ism(ism): lowercased, letters and digits only. The grid sorts on
    -- this rather than on `ism` because SQLite's NOCASE collation folds ASCII
    -- A-Z and nothing else, so a library containing "Éclat" or an Arabic title
    -- would order differently from one machine to the next. Folding in Rust
    -- with full Unicode lowercasing makes the order identical everywhere.
    ism_muwahhad      TEXT NOT NULL,

    jidhr             TEXT NOT NULL,
    tanfidhi          TEXT,
    hajm              INTEGER NOT NULL DEFAULT 0 CHECK (hajm >= 0),
    akhir_laab        TEXT,
    akhir_tahdith     TEXT,

    -- The build installed right now, as a fingerprint into bina. Deferred,
    -- because a scan inserts the game and its build in one transaction and the
    -- order inside that transaction is not the schema's business.
    basma_haliya      TEXT,

    -- Artwork keys into makhbaa/, content-addressed. ON DELETE SET NULL so that
    -- evicting a cached image leaves the grid drawing its colour placeholder
    -- rather than pointing at a file that is gone.
    ghilaf            TEXT REFERENCES sura (miftah) ON DELETE SET NULL,
    batl              TEXT REFERENCES sura (miftah) ON DELETE SET NULL,
    shiar             TEXT REFERENCES sura (miftah) ON DELETE SET NULL,

    -- The dominant colour of the cover, extracted once at import. Three
    -- constrained integers rather than one '#rrggbb' string: LawnBariz can
    -- render itself as hex but cannot parse itself back, and a text column
    -- would accept '#ff00' without complaint. Denormalized onto the game row on
    -- purpose — the grid draws this placeholder for every visible tile before
    -- any image has decoded, and it is not going to do a join per tile to get
    -- three bytes.
    lawn_ahmar        INTEGER CHECK (lawn_ahmar  BETWEEN 0 AND 255),
    lawn_akhdar       INTEGER CHECK (lawn_akhdar BETWEEN 0 AND 255),
    lawn_azraq        INTEGER CHECK (lawn_azraq  BETWEEN 0 AND 255),

    -- BeeatTawafuq, flattened. A prefix environment must carry its prefix root:
    -- a Proton game whose prefix path was lost is a game the installer would
    -- write a Windows framework into a Linux directory for.
    beea_naw          TEXT NOT NULL DEFAULT 'asli'
                      CHECK (beea_naw IN ('asli', 'proton', 'wine', 'rosetta')),
    beea_isdar        TEXT,
    beea_jidhr        TEXT,

    -- Absent, never deleted. A game uninstalled from disk keeps its patch
    -- records, its projects and its backup manifests, so a reinstall restores
    -- the user's Arabic exactly as it was instead of starting over.
    mawjuda           INTEGER NOT NULL DEFAULT 1 CHECK (mawjuda IN (0, 1)),
    mukhfiya          INTEGER NOT NULL DEFAULT 0 CHECK (mukhfiya IN (0, 1)),
    -- A partially downloaded game is shown but never offered as patchable:
    -- patching it would be patching files the launcher is about to overwrite.
    muktamila         INTEGER NOT NULL DEFAULT 1 CHECK (muktamila IN (0, 1)),
    khiyarat_tashghil TEXT,

    -- The scan generation that last saw this game. The absence sweep is one
    -- statement over this column instead of one round trip per row.
    fahs              INTEGER NOT NULL DEFAULT 0 CHECK (fahs >= 0),
    awwal_ruya        TEXT NOT NULL,
    akhir_ruya        TEXT NOT NULL,

    -- All three channels or none. Half a colour is not a colour, and the grid
    -- would render it as black.
    CHECK ((lawn_ahmar IS NULL) = (lawn_akhdar IS NULL)),
    CHECK ((lawn_akhdar IS NULL) = (lawn_azraq IS NULL)),
    -- A prefix environment has a prefix; a native one does not.
    CHECK ((beea_naw IN ('proton', 'wine')) = (beea_jidhr IS NOT NULL)),

    FOREIGN KEY (id, basma_haliya) REFERENCES bina (luba, basma)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

-- The library grid, sorted by name. Hidden games are never in the default view,
-- so mukhfiya leads: one index serves both the default listing and the
-- "show hidden" one without a second scan.
CREATE INDEX luba_bil_ism ON luba (mukhfiya, ism_muwahhad);

-- The two recency orders the grid offers. NULLs sort last under DESC, which is
-- what "never played" should look like.
CREATE INDEX luba_bil_laab ON luba (mukhfiya, akhir_laab DESC);
CREATE INDEX luba_bil_tahdith ON luba (mukhfiya, akhir_tahdith DESC);

-- The absence sweep after a scan: WHERE fahs < ?. Without this it is a full
-- table scan every time a launcher finishes, on the same connection the grid is
-- reading from.
CREATE INDEX luba_bil_fahs ON luba (fahs);

-- ---------------------------------------------------------------------------
-- masdar_luba — the launcher identities a game is known by.
--
-- One game, many sources. The primary key is (aila, muarrif) rather than a
-- surrogate: it makes it impossible for one Steam app id to be attached to two
-- different games, which would put the same title in the grid twice and make
-- "which patch applies here" ambiguous for both copies.
--
-- Heroic is not a family of its own. MasdarLuba::Heroic wraps the store the
-- game really belongs to, so a patch published against a Steam identifier is
-- found by a Heroic user who owns the same game; `mudir` records that Heroic is
-- how it is installed, and `aila` stays the real store.
-- ---------------------------------------------------------------------------
CREATE TABLE masdar_luba (
    luba    TEXT NOT NULL REFERENCES luba (id) ON DELETE CASCADE,
    aila    TEXT NOT NULL CHECK (aila IN (
                'steam', 'epic', 'gog', 'ea', 'ubisoft',
                'battlenet', 'xbox', 'itch', 'yadawi')),
    muarrif TEXT NOT NULL CHECK (length(muarrif) > 0),
    mudir   TEXT CHECK (mudir IS NULL OR mudir = 'heroic'),

    -- Discovery order, and it is semantic rather than cosmetic: Luba::masdar_asli
    -- reads the first source, and that is the store whose registry shard is
    -- queried for patches. Losing the order changes which catalogue is searched.
    tarteeb INTEGER NOT NULL CHECK (tarteeb >= 0),
    waqt    TEXT NOT NULL,

    PRIMARY KEY (aila, muarrif)
) STRICT, WITHOUT ROWID;

-- One game cannot hold two identities at the same position, so the order is a
-- real sequence and not a set of ties resolved by insertion luck. This index is
-- also how a game's identities are fetched, since it leads with `luba`.
CREATE UNIQUE INDEX masdar_bi_tarteeb ON masdar_luba (luba, tarteeb);

-- "Show me only my Steam games." The primary key can find the launcher but
-- yields muarrif, which would mean a lookup per row to reach the game.
CREATE INDEX masdar_bil_aila ON masdar_luba (aila, luba);

-- ---------------------------------------------------------------------------
-- sima_luba — what a launcher's own metadata says about a game.
--
-- Hints, not conclusions. Steam knows a game is VAC-secured and nothing on disk
-- says so; Phase 16 still refuses on evidence, but a hint here lets the
-- interface warn before the user clicks.
--
-- `aila` is part of the key because provenance is part of the claim: "Steam
-- lists this as VAC-secured" and "Epic lists this as multiplayer" are two
-- different statements about one game, and a refresh of one launcher must be
-- able to replace its own hints without touching the other's.
--
-- `qeema` is the payload for the three hints that carry one and empty for the
-- three that do not — an empty string rather than NULL because it is part of the
-- key, and a key column that is sometimes NULL is a key that does not
-- deduplicate.
-- ---------------------------------------------------------------------------
CREATE TABLE sima_luba (
    luba  TEXT NOT NULL REFERENCES luba (id) ON DELETE CASCADE,
    aila  TEXT NOT NULL,
    naw   TEXT NOT NULL CHECK (naw IN (
              'jamai_mahalli', 'jamai_online', 'himaya_muhtamala',
              'muammana_vac', 'laysat_luba', 'tabaqat_tawafuq')),
    qeema TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (luba, aila, naw, qeema)
) STRICT, WITHOUT ROWID;

-- ---------------------------------------------------------------------------
-- bina — the builds a game has been seen at.
--
-- Keyed by the content fingerprint, not by the launcher's build identifier,
-- because the fingerprint is what a patch binds to. A store that pushes a
-- shader recompile changes the identifier and not the fingerprint, and that is
-- precisely the case MutabaqaBina::Basma exists to keep a patch alive through.
-- When the same content reappears under a new launcher identifier the row is
-- updated rather than duplicated: the previous identifier has no further use
-- once the fingerprint has proved the text-bearing files did not move.
--
-- History is kept rather than overwritten, because Phase 15's update-survival
-- path needs to know what the build changed *from*.
-- ---------------------------------------------------------------------------
CREATE TABLE bina (
    luba          TEXT NOT NULL REFERENCES luba (id) ON DELETE CASCADE,
    -- 64 lowercase hexadecimal characters. An uppercase or truncated
    -- fingerprint would compare unequal to the identical one computed
    -- elsewhere, and a patch would silently stop matching a build it fits.
    basma         TEXT NOT NULL
                  CHECK (length(basma) = 64 AND basma NOT GLOB '*[^0-9a-f]*'),
    manassa       TEXT,
    -- How many files went into the fingerprint, so a fingerprint computed over
    -- a partially downloaded installation is recognisable as one.
    adad_malaffat INTEGER NOT NULL DEFAULT 0 CHECK (adad_malaffat >= 0),
    waqt          TEXT NOT NULL,
    akhir_ruya    TEXT NOT NULL,
    PRIMARY KEY (luba, basma)
) STRICT, WITHOUT ROWID;

-- Registry matching runs the other way round: given a launcher build identifier
-- a patch declares, which of my games is at it?
CREATE INDEX bina_bil_manassa ON bina (manassa) WHERE manassa IS NOT NULL;

-- ---------------------------------------------------------------------------
-- bitaqa_muharrik — the capability report, one per game.
--
-- The fields every downstream dispatch decision reads are columns: which
-- adapter runs, which framework is installed, which tier the interface
-- promises. The report itself is kept whole as JSON in `taqreer`, in exactly
-- the serde form the JSON Schema and the TypeScript type describe, because it
-- is a product artifact consumed as a document — the evidence list goes into a
-- diagnostics bundle verbatim and the limitation sentences are rendered as
-- written. Splitting Daleel and Hadd into child tables would create two tables
-- nothing ever queries by and would let the stored report drift from the
-- schema the frontend validates against.
--
-- isdar_fahs is the probe's own version. A Taarib update that improves
-- detection re-probes every game whose report predates it instead of serving a
-- stale conclusion, and that is one indexed query at startup.
-- ---------------------------------------------------------------------------
CREATE TABLE bitaqa_muharrik (
    luba          TEXT PRIMARY KEY REFERENCES luba (id) ON DELETE CASCADE,
    -- The build the probe examined. A build change invalidates the report by
    -- comparison rather than by a flag somebody has to remember to clear.
    basma_bina    TEXT,

    aila          TEXT NOT NULL CHECK (aila IN (
                      'unity', 'unreal', 'godot', 'rpg_maker_mv', 'rpg_maker_mz',
                      'rpg_maker_vx_ace', 'renpy', 'game_maker', 'electron', 'majhul')),
    isdar_khaam   TEXT,
    isdar_kabir   INTEGER CHECK (isdar_kabir >= 0),
    isdar_sagheer INTEGER CHECK (isdar_sagheer >= 0),
    isdar_tasheeh INTEGER CHECK (isdar_tasheeh >= 0),
    khalfiya      TEXT NOT NULL,
    mimariya      TEXT NOT NULL CHECK (mimariya IN ('x86', 'x8664', 'aarch64')),
    thiqa         INTEGER NOT NULL CHECK (thiqa BETWEEN 0 AND 100),

    tabaqa        TEXT NOT NULL
                  CHECK (tabaqa IN ('kamil', 'rasm_mubashir', 'tarjama_fawqiya')),
    jawda         TEXT NOT NULL
                  CHECK (jawda IN ('mumtaza', 'jayida', 'maqbula', 'mahduda')),
    marfuda       INTEGER NOT NULL DEFAULT 0 CHECK (marfuda IN (0, 1)),

    isdar_fahs    INTEGER NOT NULL CHECK (isdar_fahs >= 0),
    waqt          TEXT NOT NULL,
    taqreer       TEXT NOT NULL,

    -- A version string is either fully parsed or not parsed at all. Two of the
    -- three numbers present would mean somebody read half a version and the
    -- signature database would key on a lie.
    CHECK ((isdar_kabir IS NULL) = (isdar_sagheer IS NULL)),
    CHECK ((isdar_sagheer IS NULL) = (isdar_tasheeh IS NULL)),

    FOREIGN KEY (luba, basma_bina) REFERENCES bina (luba, basma)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

-- "Which games did an older probe examine?" — run once per launch after an
-- update, against every game in the library.
CREATE INDEX bitaqa_bi_isdar_fahs ON bitaqa_muharrik (isdar_fahs);

-- HalatLuba::Marfuda in the grid. Partial, because the refused set is a handful
-- of games out of hundreds and a full index would be almost entirely dead pages.
CREATE INDEX bitaqa_marfuda ON bitaqa_muharrik (luba) WHERE marfuda = 1;

-- ---------------------------------------------------------------------------
-- sura — the artwork cache index.
--
-- The files are content-addressed under makhbaa/; this is what makes eviction
-- possible without walking the directory, and what stops the same cover being
-- fetched once per game that shares it. The dominant colour is not here: it
-- lives on the game row, where the grid reads it without a join.
-- ---------------------------------------------------------------------------
CREATE TABLE sura (
    miftah          TEXT PRIMARY KEY CHECK (length(miftah) > 0),
    naw             TEXT NOT NULL CHECK (naw IN ('ghilaf', 'batl', 'shiar')),
    -- Where it came from: a launcher's own cache file, or the address it was
    -- fetched from. Kept so a broken image can be re-fetched from the same place.
    masdar          TEXT,
    hajm            INTEGER NOT NULL DEFAULT 0 CHECK (hajm >= 0),
    ard             INTEGER CHECK (ard > 0),
    irtifa          INTEGER CHECK (irtifa > 0),
    waqt            TEXT NOT NULL,
    akhir_istikhdam TEXT NOT NULL
) STRICT;

-- Cache eviction is least-recently-used over this column.
CREATE INDEX sura_bil_istikhdam ON sura (akhir_istikhdam);

-- ---------------------------------------------------------------------------
-- fahs / fahs_matjar / tanbih_fahs — what a scan did.
--
-- fahs.raqm is the generation stamped onto every game a scan touches, and the
-- absence sweep is `UPDATE luba SET mawjuda = 0 WHERE fahs < ?`, scoped to the
-- launcher families that actually enumerated. That scoping is the whole point of
-- fahs_matjar.najah: a launcher whose catalogue could not be read must never
-- sweep, because "Steam did not answer" and "the user uninstalled every Steam
-- game" look identical from the sweep's side and one of them empties a library.
-- ---------------------------------------------------------------------------
CREATE TABLE fahs (
    raqm       INTEGER PRIMARY KEY AUTOINCREMENT,
    bidaya     TEXT NOT NULL,
    nihaya     TEXT,
    -- A full rescan across every launcher, or a refresh of one.
    kamil      INTEGER NOT NULL DEFAULT 1 CHECK (kamil IN (0, 1)),
    adad_alaab INTEGER NOT NULL DEFAULT 0 CHECK (adad_alaab >= 0)
) STRICT;

CREATE TABLE fahs_matjar (
    fahs          INTEGER NOT NULL REFERENCES fahs (raqm) ON DELETE CASCADE,
    aila          TEXT NOT NULL,
    -- Whether the launcher is installed at all. Most machines have three of ten.
    mawjud        INTEGER NOT NULL DEFAULT 0 CHECK (mawjud IN (0, 1)),
    -- Whether its catalogue was read end to end. Individual bad entries degrade
    -- into tanbih_fahs and leave this true.
    najah         INTEGER NOT NULL DEFAULT 0 CHECK (najah IN (0, 1)),
    adad_alaab    INTEGER NOT NULL DEFAULT 0 CHECK (adad_alaab >= 0),
    adad_tanbihat INTEGER NOT NULL DEFAULT 0 CHECK (adad_tanbihat >= 0),
    muddat_milli  INTEGER NOT NULL DEFAULT 0 CHECK (muddat_milli >= 0),
    PRIMARY KEY (fahs, aila)
) STRICT, WITHOUT ROWID;

-- One catalogue entry that could not be read, and why. This is the answer to
-- "my game is not showing up", which is otherwise a bug report with nothing in it.
CREATE TABLE tanbih_fahs (
    id    INTEGER PRIMARY KEY AUTOINCREMENT,
    fahs  INTEGER NOT NULL REFERENCES fahs (raqm) ON DELETE CASCADE,
    aila  TEXT NOT NULL,
    mawdi TEXT NOT NULL,
    sabab TEXT NOT NULL
) STRICT;

CREATE INDEX tanbih_bil_fahs ON tanbih_fahs (fahs);

-- The library roots worth watching, so a refresh is reactive rather than polled
-- and the watcher knows what to re-establish after a restart.
CREATE TABLE jidhr_maktaba (
    masar      TEXT PRIMARY KEY,
    aila       TEXT NOT NULL,
    mawjud     INTEGER NOT NULL DEFAULT 1 CHECK (mawjud IN (0, 1)),
    akhir_fahs TEXT
) STRICT;

CREATE INDEX jidhr_bil_aila ON jidhr_maktaba (aila);

-- ---------------------------------------------------------------------------
-- musahim — contributors, as the registry publishes them.
--
-- Identity is a public key fingerprint. There is no account, no password, and
-- nothing to recover; a contributor is whoever can sign with the key their
-- patches were signed with. The counters are derived from the registry's own
-- commit history, so nothing here is a score anyone can grant or inflate.
-- ---------------------------------------------------------------------------
CREATE TABLE musahim (
    id                 TEXT PRIMARY KEY
                       CHECK (length(id) = 64 AND id NOT GLOB '*[^0-9a-f]*'),
    ism                TEXT NOT NULL,
    satr_itiraf        TEXT,
    rabt               TEXT,

    ruqaa_manshura     INTEGER NOT NULL DEFAULT 0 CHECK (ruqaa_manshura >= 0),
    qubila_bila_taadil INTEGER NOT NULL DEFAULT 0 CHECK (qubila_bila_taadil >= 0),
    tulib_taadil       INTEGER NOT NULL DEFAULT 0 CHECK (tulib_taadil >= 0),
    marfuda            INTEGER NOT NULL DEFAULT 0 CHECK (marfuda >= 0),
    masbuba            INTEGER NOT NULL DEFAULT 0 CHECK (masbuba >= 0),
    mutawassit_taqyeem REAL CHECK (mutawassit_taqyeem BETWEEN 0.0 AND 5.0),
    adad_taqyeemat     INTEGER NOT NULL DEFAULT 0 CHECK (adad_taqyeemat >= 0),
    mundhu             TEXT,

    taghtiya_majmu          INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu >= 0),
    taghtiya_mutarjam       INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam >= 0),
    taghtiya_muakkad        INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_muakkad >= 0),
    taghtiya_majmu_takrar   INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_takrar >= 0),
    taghtiya_mutarjam_takrar INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_takrar >= 0),
    taghtiya_majmu_awwal    INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_awwal >= 0),
    taghtiya_mutarjam_awwal INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_awwal >= 0),

    akhir_jalb         TEXT NOT NULL
) STRICT;

-- ---------------------------------------------------------------------------
-- makhbaa_shard — the registry index cache.
--
-- Shard bodies are files under makhbaa/; this is the manifest that decides
-- whether any of them need fetching. A launch where nothing changed costs one
-- conditional request, which is the whole reason the registry is sharded.
-- ---------------------------------------------------------------------------
CREATE TABLE makhbaa_shard (
    miftah        TEXT PRIMARY KEY,
    basma         TEXT NOT NULL
                  CHECK (length(basma) = 64 AND basma NOT GLOB '*[^0-9a-f]*'),
    hajm          INTEGER NOT NULL DEFAULT 0 CHECK (hajm >= 0),
    etag          TEXT,
    akhir_jalb    TEXT NOT NULL,
    akhir_tabdeel TEXT
) STRICT;

-- ---------------------------------------------------------------------------
-- ruqaa — the patch summaries a shard carried.
--
-- (id, murajaa) rather than id alone: a lineage keeps every revision it has
-- published, so a client that installed r3 can be told r4 exists and can still
-- describe the r3 it has. There is no foreign key to musahim on purpose — the
-- index and the contributor records are fetched independently, and a constraint
-- here would turn the arrival order of two unrelated HTTP responses into a
-- correctness requirement.
-- ---------------------------------------------------------------------------
CREATE TABLE ruqaa (
    id             TEXT NOT NULL
                   CHECK (length(id) = 36 AND id NOT GLOB '*[^0-9a-f-]*'),
    murajaa        INTEGER NOT NULL CHECK (murajaa >= 1),

    -- The game it targets, as the registry shards on it.
    aila           TEXT NOT NULL,
    muarrif        TEXT NOT NULL,

    unwan          TEXT NOT NULL,
    musahim        TEXT NOT NULL,
    ism_musahim    TEXT NOT NULL,
    adad_nusus     INTEGER NOT NULL DEFAULT 0 CHECK (adad_nusus >= 0),
    hajm           INTEGER NOT NULL DEFAULT 0 CHECK (hajm >= 0),

    aila_muharrik  TEXT NOT NULL,
    khalfiya       TEXT NOT NULL,
    tabaqa         TEXT NOT NULL
                   CHECK (tabaqa IN ('kamil', 'rasm_mubashir', 'tarjama_fawqiya')),
    tareeqa        TEXT NOT NULL CHECK (tareeqa IN (
                       'bashariya_kamila', 'aaliya_thum_bashariya', 'aaliya_faqat')),
    rukhsa         TEXT NOT NULL,
    halat          TEXT NOT NULL CHECK (halat IN (
                       'musawwada', 'muqaddama', 'qayd_muraja', 'matlub_taadil',
                       'mawafaq_yunshar', 'manshura', 'marfuda', 'mashuba', 'masbuba')),

    taqyeem        REAL CHECK (taqyeem BETWEEN 0.0 AND 5.0),
    adad_taqyeemat INTEGER NOT NULL DEFAULT 0 CHECK (adad_taqyeemat >= 0),

    taghtiya_majmu           INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu >= 0),
    taghtiya_mutarjam        INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam >= 0),
    taghtiya_muakkad         INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_muakkad >= 0),
    taghtiya_majmu_takrar    INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_takrar >= 0),
    taghtiya_mutarjam_takrar INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_takrar >= 0),
    taghtiya_majmu_awwal     INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_awwal >= 0),
    taghtiya_mutarjam_awwal  INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_awwal >= 0),

    waqt_nashr     TEXT NOT NULL,
    basmat_muhtawa TEXT NOT NULL
                   CHECK (length(basmat_muhtawa) = 64
                          AND basmat_muhtawa NOT GLOB '*[^0-9a-f]*'),
    rabt           TEXT NOT NULL,
    rabt_mira      TEXT,
    akhir_jalb     TEXT NOT NULL,

    PRIMARY KEY (id, murajaa)
) STRICT;

-- The badge query: for the games in my library, is there a patch? Joined
-- against masdar_luba on exactly these two columns.
CREATE INDEX ruqaa_bil_luba ON ruqaa (aila, muarrif);

-- The Contributions screen, and the contributor card on a listing.
CREATE INDEX ruqaa_bil_musahim ON ruqaa (musahim);

-- ---------------------------------------------------------------------------
-- ruqaa_bina — the builds a patch declares it fits.
--
-- Two kinds in one table because they answer one question. Given the build the
-- user actually has, MutabaqaBina resolves by looking for the launcher
-- identifier first and the content fingerprint second, and both lookups are the
-- same indexed probe.
-- ---------------------------------------------------------------------------
CREATE TABLE ruqaa_bina (
    ruqaa   TEXT NOT NULL,
    murajaa INTEGER NOT NULL,
    naw     TEXT NOT NULL CHECK (naw IN ('manassa', 'basma')),
    qeema   TEXT NOT NULL CHECK (length(qeema) > 0),
    PRIMARY KEY (ruqaa, murajaa, naw, qeema),
    FOREIGN KEY (ruqaa, murajaa) REFERENCES ruqaa (id, murajaa) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;

-- "Which patches declare this fingerprint?" — one probe, not a scan of every
-- patch the cache holds.
CREATE INDEX ruqaa_bina_bil_qeema ON ruqaa_bina (naw, qeema);

-- ---------------------------------------------------------------------------
-- tathbeet — what has been installed into a game, and what it replaced.
--
-- INTEGER PRIMARY KEY AUTOINCREMENT rather than a plain rowid: without
-- AUTOINCREMENT SQLite reuses the rowid of a deleted row, and a manifest row
-- that names a backup file would then be adopted by a later, unrelated
-- installation that happened to land on the same number. The cost is one extra
-- table; the failure it prevents is a rollback restoring the wrong file into
-- somebody's game.
-- ---------------------------------------------------------------------------
CREATE TABLE tathbeet (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    luba          TEXT NOT NULL REFERENCES luba (id) ON DELETE CASCADE,
    ruqaa         TEXT NOT NULL
                  CHECK (length(ruqaa) = 36 AND ruqaa NOT GLOB '*[^0-9a-f-]*'),
    murajaa       INTEGER NOT NULL CHECK (murajaa >= 1),
    tabaqa        TEXT NOT NULL
                  CHECK (tabaqa IN ('kamil', 'rasm_mubashir', 'tarjama_fawqiya')),
    -- The build it was installed against, so an update is detected by comparing
    -- fingerprints rather than by asking the user whether anything changed.
    basma_bina    TEXT,
    mutabaqa      TEXT NOT NULL
                  CHECK (mutabaqa IN ('tamma', 'basma', 'nitaq', 'ghayr')),
    halat         TEXT NOT NULL
                  CHECK (halat IN ('nashit', 'muzal', 'fashil', 'naqis')),
    -- The backup directory under nusakh/, per installation.
    jidhr_nusakh  TEXT NOT NULL,
    adad_malaffat INTEGER NOT NULL DEFAULT 0 CHECK (adad_malaffat >= 0),
    waqt          TEXT NOT NULL,
    waqt_izala    TEXT,

    -- A removal time exists exactly when the installation was removed. Anything
    -- else means one of the two was written and the other forgotten.
    CHECK ((halat = 'muzal') = (waqt_izala IS NOT NULL)),

    FOREIGN KEY (luba, basma_bina) REFERENCES bina (luba, basma)
        DEFERRABLE INITIALLY DEFERRED
) STRICT;

-- At most one live installation per game. Two patches writing the same game's
-- files is the corruption case, and the database refuses it outright rather
-- than trusting every future caller to check first. Partial, so the history of
-- removed and failed installations is unbounded and unconstrained.
CREATE UNIQUE INDEX tathbeet_nashit ON tathbeet (luba) WHERE halat = 'nashit';

-- The game's detail screen: this installation, then everything before it.
CREATE INDEX tathbeet_bil_luba ON tathbeet (luba, waqt DESC);

-- ---------------------------------------------------------------------------
-- bayan_tathbeet — the manifest of files one installation touched.
--
-- The rollback contract, written as a constraint instead of as a code path
-- somebody has to remember: a file that was replaced must name the backup that
-- holds its original. Without the CHECK, an installer with a bug records the
-- replacement and not the copy, and the failure surfaces only when a user tries
-- to uninstall — at which point the original is already gone.
-- ---------------------------------------------------------------------------
CREATE TABLE bayan_tathbeet (
    tathbeet     INTEGER NOT NULL REFERENCES tathbeet (id) ON DELETE CASCADE,
    -- Relative to the game's root, always. An absolute path here would be a
    -- manifest that stops working the moment the user moves the game.
    masar        TEXT NOT NULL CHECK (length(masar) > 0),
    amal         TEXT NOT NULL
                 CHECK (amal IN ('kutib', 'ustubdil', 'unshia_mujallad')),
    basma_asl    TEXT CHECK (basma_asl IS NULL OR
                     (length(basma_asl) = 64 AND basma_asl NOT GLOB '*[^0-9a-f]*')),
    basma_jadeed TEXT CHECK (basma_jadeed IS NULL OR
                     (length(basma_jadeed) = 64 AND basma_jadeed NOT GLOB '*[^0-9a-f]*')),
    hajm_asl     INTEGER CHECK (hajm_asl IS NULL OR hajm_asl >= 0),
    -- The backup key under nusakh/<jidhr_nusakh>/.
    nuskha       TEXT,

    PRIMARY KEY (tathbeet, masar),
    CHECK ((amal = 'ustubdil') = (nuskha IS NOT NULL))
) STRICT, WITHOUT ROWID;

-- ---------------------------------------------------------------------------
-- mashru — a translation project.
--
-- ON DELETE SET NULL on the game, not CASCADE. A project is the translator's
-- own work and it outlives the game being removed from the library; it keeps
-- the game's name denormalized so it can still say what it is a translation of.
-- ---------------------------------------------------------------------------
CREATE TABLE mashru (
    id            TEXT PRIMARY KEY
                  CHECK (length(id) = 36 AND id NOT GLOB '*[^0-9a-f-]*'),
    luba          TEXT REFERENCES luba (id) ON DELETE SET NULL,
    ism_luba      TEXT NOT NULL,
    unwan         TEXT NOT NULL,
    -- The project directory under mashari/.
    mujallad      TEXT NOT NULL,
    lugha_masdar  TEXT NOT NULL DEFAULT 'en',
    lugha_hadaf   TEXT NOT NULL DEFAULT 'ar',
    halat         TEXT NOT NULL CHECK (halat IN (
                      'musawwada', 'muqaddama', 'qayd_muraja', 'matlub_taadil',
                      'mawafaq_yunshar', 'manshura', 'marfuda', 'mashuba', 'masbuba')),
    -- The lineage it publishes into, once it has one.
    ruqaa         TEXT CHECK (ruqaa IS NULL OR
                      (length(ruqaa) = 36 AND ruqaa NOT GLOB '*[^0-9a-f-]*')),
    murajaa       INTEGER CHECK (murajaa IS NULL OR murajaa >= 1),
    basma_bina    TEXT,
    tareeqa       TEXT CHECK (tareeqa IS NULL OR tareeqa IN (
                      'bashariya_kamila', 'aaliya_thum_bashariya', 'aaliya_faqat')),
    rukhsa        TEXT,
    musahim       TEXT,

    taghtiya_majmu           INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu >= 0),
    taghtiya_mutarjam        INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam >= 0),
    taghtiya_muakkad         INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_muakkad >= 0),
    taghtiya_majmu_takrar    INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_takrar >= 0),
    taghtiya_mutarjam_takrar INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_takrar >= 0),
    taghtiya_majmu_awwal     INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_majmu_awwal >= 0),
    taghtiya_mutarjam_awwal  INTEGER NOT NULL DEFAULT 0 CHECK (taghtiya_mutarjam_awwal >= 0),

    waqt_insha    TEXT NOT NULL,
    akhir_tabdeel TEXT NOT NULL
) STRICT;

CREATE INDEX mashru_bil_luba ON mashru (luba);

-- The workspace opens on "what was I working on?", which is this index.
CREATE INDEX mashru_bil_tabdeel ON mashru (akhir_tabdeel DESC);

-- ---------------------------------------------------------------------------
-- dhakira — translation memory.
--
-- It lives in taarib.db rather than beside the projects because the entire
-- value of a memory is reuse *across* projects: a segment translated for one
-- game is offered in the next, and a memory partitioned per project cannot do
-- that. Lookup is by the normalized source text, indexed, which narrows to the
-- candidates a fuzzy pass then ranks — the ranking is Phase 13's, not the
-- store's.
-- ---------------------------------------------------------------------------
CREATE TABLE dhakira (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    masdar          TEXT NOT NULL,
    -- Lowercased and collapsed in Rust, with full Unicode folding, for the
    -- same reason the library sorts on ism_muwahhad: SQLite's own collations
    -- would fold ASCII and leave everything else to byte order.
    masdar_muwahhad TEXT NOT NULL,
    hadaf           TEXT NOT NULL,
    lugha_masdar    TEXT NOT NULL DEFAULT 'en',
    lugha_hadaf     TEXT NOT NULL DEFAULT 'ar',

    mashru          TEXT REFERENCES mashru (id) ON DELETE SET NULL,
    luba            TEXT REFERENCES luba (id) ON DELETE SET NULL,
    ism_luba        TEXT,
    musahim         TEXT,
    tareeqa         TEXT CHECK (tareeqa IS NULL OR tareeqa IN (
                        'bashariya_kamila', 'aaliya_thum_bashariya', 'aaliya_faqat')),
    halat           TEXT NOT NULL CHECK (halat IN (
                        'lam_tutarjam', 'musawwada', 'lil_muraja',
                        'muakkada', 'mujammada')),
    thiqa           REAL CHECK (thiqa IS NULL OR thiqa BETWEEN 0.0 AND 1.0),
    -- How often this pair has been reused, which is how candidates are ranked.
    marrat          INTEGER NOT NULL DEFAULT 1 CHECK (marrat >= 0),
    waqt            TEXT NOT NULL,
    akhir_istikhdam TEXT
) STRICT;

-- One row per distinct pair. Without this the memory fills with hundreds of
-- copies of the same segment, each with a reuse count of one, and the ranking
-- that decides what a translator is offered becomes noise.
CREATE UNIQUE INDEX dhakira_wahida
    ON dhakira (lugha_hadaf, masdar_muwahhad, hadaf);

-- The lookup itself. The equality on the folded source narrows to the handful
-- of rows that share a segment; the ranking by review status is a CASE
-- expression the query applies over those few, because ordering by the status
-- text would put an unreviewed draft above an approved translation.
CREATE INDEX dhakira_bahth ON dhakira (masdar_muwahhad, lugha_hadaf, marrat DESC);

-- ---------------------------------------------------------------------------
-- halat — state the store owns, keyed by name.
--
-- State, not configuration. Settings live in idadat.json behind usus::idadat,
-- where they can be edited by hand, layered over by the environment, and read
-- before the database exists. What belongs here is what the store produced and
-- the store consumes: when the registry manifest was last checked, which scan
-- generation is current, when the artwork cache was last swept. Mixing the two
-- is how a settings file quietly becomes a database nobody can edit.
-- ---------------------------------------------------------------------------
CREATE TABLE halat (
    miftah TEXT PRIMARY KEY CHECK (length(miftah) > 0),
    qeema  TEXT NOT NULL,
    waqt   TEXT NOT NULL
) STRICT, WITHOUT ROWID;
"#;
