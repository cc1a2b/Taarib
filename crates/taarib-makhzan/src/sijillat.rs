//! السجلات — the ledgers, one per aggregate.
//!
//! (`sijillat` here is the register sense of the word — a ledger, a book of
//! record. It is not `usus::sijill`, which is the log.)
//!
//! Each ledger is a plain struct over a borrowed connection with a named method
//! per operation the product actually performs. None of them is a generic query
//! surface: there is no `execute(sql)` anywhere in this crate, nothing takes a
//! `WHERE` clause as an argument, and no statement text is ever built at
//! runtime. Every statement in this module is a `&'static str` constant with
//! bound parameters, including for integers — a scan identifier interpolated
//! into SQL is the same defect as a game title interpolated into SQL, and the
//! only reason the first one feels safer is that nobody has tried yet.
//!
//! ## Constructed from a connection, used inside a transaction
//!
//! A ledger is built from `&Connection`. `rusqlite::Transaction` dereferences to
//! `Connection`, so the intended call shape is:
//!
//! ```ignore
//! makhzan.bi_muamala(|muamala| {
//!     SijillAlaab::jadeed(muamala).sajjil(&idkhal)?;
//!     SijillBina::jadeed(muamala).sajjil(idkhal.luba.id, &bina)?;
//!     Ok(())
//! })
//! ```
//!
//! and both ledgers write inside the same transaction because both were handed
//! the same one. That is a convention at the level of types — but not only a
//! convention in practice: the constraint tying `luba.basma_haliya` to `bina` is
//! `DEFERRABLE INITIALLY DEFERRED`, and outside an explicit transaction every
//! statement is its own implicit one, so the deferral has nothing to defer to
//! and a game inserted with a current build outside a transaction fails on the
//! spot. The main write path of the whole crate cannot be done wrongly and stay
//! quiet about it.
//!
//! The pairing above is the *shape*, not a claim that a library scan writes
//! both. [`SijillBina::sajjil`] wants a [`BinaId`], whose fingerprint is
//! `bina.basma` — `NOT NULL`, and half the primary key. Computing one means
//! hashing a named selection of the game's containers, and that selection lives
//! inside a patch package or a translation project; discovery has no such list
//! for a game nobody has a patch for. So a build row is written where a recipe
//! is in hand — the install, manual or automatic — and a scan writes the game
//! alone.
//!
//! ## Prepared statements are cached where a scan repeats them
//!
//! [`SijillAlaab::sajjil`], [`SijillBina::sajjil`], [`SijillDhakira::sajjil`] and
//! the other per-row writers use `prepare_cached`, because a scan of a
//! four-hundred-game library runs each of them four hundred times against one
//! connection and re-parsing identical SQL that many times is work nobody asked
//! for. One-shot queries — the library listing, a lookup by identity — use
//! `prepare`, because caching a statement that runs once per screen only takes a
//! slot away from one that runs once per row.
//!
//! ## Decoding refuses rather than guesses
//!
//! A discriminator column that holds text no variant answers to produces
//! [`crate::khata::KhataMakhzan::SafTalif`] naming the table, the column and the
//! value. It never falls back to a default. A launcher family that decoded to
//! "Steam" because the real value was unreadable would send a patch lookup to
//! the wrong registry shard and report, confidently, that no Arabic patch exists
//! for a game that has one.
//!
//! Identities and enumerations cross the boundary in their serde form — the same
//! text the JSON Schema in `schemas/` and the TypeScript in Studio use — so a
//! value read out of `sqlite3` by hand is the value the frontend would receive.
//! There is no second spelling of a domain value anywhere in the product.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension as _, Row, params};
use serde::Serialize;
use serde::de::DeserializeOwned;
use taarib_mustalahat::bina::{Basma, BinaId, MutabaqaBina};
use taarib_mustalahat::luba::{LawnBariz, Luba, LubaId, MasdarLuba, SuwarLuba};
use taarib_mustalahat::muharrik::{Tabaqa, TaqreerImkaniyat};
use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::musahim::{Musahim, MusahimId, Sumaa};
use taarib_mustalahat::ruqaa::{
    HalatRuqaa, MulakhkhasRuqaa, RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama,
};
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::manassa::BeeatTawafuq;

use crate::khata::KhataMakhzan;
use crate::wasl::{alaan, khata_jumla};

// ---------------------------------------------------------------------------
// Storage vocabulary owned by this crate
// ---------------------------------------------------------------------------

/// The discriminators the `sima_luba.naw` column accepts.
///
/// These name the launcher metadata hints `taarib-kashf` collects. They live
/// here, not there, because the column and its `CHECK` constraint live here: a
/// writer that spelled one of them differently would be rejected by the database
/// at the insert, and a writer that uses these constants cannot spell one
/// differently at all.
pub mod sima {
    /// The launcher lists local multiplayer.
    pub const JAMAI_MAHALLI: &str = "jamai_mahalli";
    /// The launcher lists online multiplayer.
    pub const JAMAI_ONLINE: &str = "jamai_online";
    /// The launcher associates an anti-cheat service; the payload names it.
    pub const HIMAYA_MUHTAMALA: &str = "himaya_muhtamala";
    /// Steam reports the title as VAC-secured.
    pub const MUAMMANA_VAC: &str = "muammana_vac";
    /// The entry is a tool, runtime, soundtrack or demo; the payload says which.
    pub const LAYSAT_LUBA: &str = "laysat_luba";
    /// The launcher records a compatibility layer; the payload names it.
    pub const TABAQAT_TAWAFUQ: &str = "tabaqat_tawafuq";
}

/// One launcher metadata hint, as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimaMukhzana {
    /// Which launcher said it, by registry shard family.
    pub aila: String,
    /// One of the constants in [`sima`].
    pub naw: String,
    /// The payload, empty for the hints that carry none.
    pub qeema: String,
}

impl SimaMukhzana {
    /// Builds a hint.
    #[must_use]
    pub fn jadeeda(aila: &str, naw: &str, qeema: &str) -> Self {
        Self {
            aila: aila.to_owned(),
            naw: naw.to_owned(),
            qeema: qeema.to_owned(),
        }
    }
}

/// Everything one discovered game contributes to the store.
///
/// [`Luba`] carries what the whole product speaks about; the rest of these
/// fields are facts a launcher's catalogue gives at discovery time and nothing
/// downstream re-derives, so the store keeps them rather than losing them
/// between the scan and the install.
#[derive(Debug, Clone)]
pub struct IdkhalLuba<'a> {
    /// The game itself.
    pub luba: &'a Luba,
    /// Whether the launcher reports it as fully downloaded. A partial download
    /// is shown in the library and never offered as patchable.
    pub muktamila: bool,
    /// The user's own launch options, kept so an installer extends them instead
    /// of overwriting them.
    pub khiyarat_tashghil: Option<&'a str>,
    /// What the launcher's metadata says about the title.
    pub simat: &'a [SimaMukhzana],
    /// The scan generation that found it.
    pub fahs: u64,
}

/// How the library grid is ordered.
///
/// A closed set. The clause each one produces is a literal chosen by a `match`,
/// never a string assembled from an argument, so the statement text stays
/// constant and the prepared-statement cache stays useful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TarteebMaktaba {
    /// By name, using the Unicode-folded key rather than a `SQLite` collation.
    #[default]
    Ism,
    /// Most recently played first; never played sorts last.
    AkhirLaab,
    /// Most recently updated by its launcher first.
    AkhirTahdith,
    /// Largest on disk first.
    Hajm,
}

/// What the library grid is asking for.
#[derive(Debug, Clone, Copy, Default)]
pub struct TalabMaktaba<'a> {
    /// The order.
    pub tarteeb: TarteebMaktaba,
    /// Show hidden games instead of visible ones.
    pub mukhfiya: bool,
    /// Restrict to games still present on disk. Off by default: an absent game
    /// stays in the library, greyed, because its patches and projects are still
    /// there and the user may reinstall it tomorrow.
    pub mawjuda_faqat: bool,
    /// Restrict to one launcher family, by its registry shard name.
    pub aila: Option<&'a str>,
    /// At most this many rows.
    pub hadd: u32,
    /// Skip this many rows.
    pub izaha: u32,
}

// ---------------------------------------------------------------------------
// Shared encoding and decoding
// ---------------------------------------------------------------------------

/// Decodes a value from the text a column holds, through the same serde form the
/// JSON Schema and the TypeScript bindings use.
///
/// Serves both identities (which are transparent newtypes over a UUID) and unit
/// enumerations (which serialize as their `snake_case` name), so there is one
/// decoding path for every discriminator in the schema instead of a hand-written
/// `match` per enum that could drift from the derive that produced the wire
/// form.
fn min_ramz<T: DeserializeOwned>(
    jadwal: &'static str,
    amud: &'static str,
    nass: &str,
) -> Natija<T> {
    serde_json::from_value(serde_json::Value::String(nass.to_owned())).map_err(|_| {
        Khata::from(KhataMakhzan::SafTalif {
            jadwal,
            amud,
            qeema: nass.to_owned(),
        })
    })
}

/// Encodes a unit enumeration into the text its column holds.
fn ila_ramz<T: Serialize>(jadwal: &'static str, amud: &'static str, qeema: &T) -> Natija<String> {
    match serde_json::to_value(qeema) {
        Ok(serde_json::Value::String(nass)) => Ok(nass),
        _ => Err(Khata::from(KhataMakhzan::SafTalif {
            jadwal,
            amud,
            qeema: "a value that does not serialize to a name".to_owned(),
        })),
    }
}

/// Renders a path for storage, refusing one that is not valid Unicode.
fn masar_nass<'a>(amud: &'static str, masar: &'a Path) -> Natija<&'a str> {
    masar.to_str().ok_or_else(|| {
        Khata::from(KhataMakhzan::MasarGhayrNassi {
            amud,
            masar: masar.to_path_buf(),
        })
    })
}

/// Splits a launcher identity into the three columns `masdar_luba` holds.
///
/// Heroic is unwrapped: `aila` is always the store the game really belongs to,
/// so a patch published against a Steam identifier is found by a Heroic user who
/// owns the same game, and `mudir` records that Heroic is merely how it was
/// installed.
fn fakkik_masdar(masdar: &MasdarLuba) -> (&'static str, String, Option<&'static str>) {
    let asl = masdar.asl();
    let kamil = asl.muarrif();
    let muarrif = kamil
        .split_once(':')
        .map_or_else(|| kamil.clone(), |(_, dhayl)| dhayl.to_owned());
    let mudir = if matches!(masdar, MasdarLuba::Heroic(_)) {
        Some("heroic")
    } else {
        None
    };
    (asl.aila().slug(), muarrif, mudir)
}

/// Rebuilds a launcher identity from its three columns.
fn ijma_masdar(aila: &str, muarrif: &str, mudir: Option<&str>) -> Natija<MasdarLuba> {
    let talif = || {
        Khata::from(KhataMakhzan::SafTalif {
            jadwal: "masdar_luba",
            amud: "muarrif",
            qeema: format!("{aila}:{muarrif}"),
        })
    };

    let asl = match aila {
        "steam" => MasdarLuba::Steam(muarrif.parse::<u32>().map_err(|_| talif())?),
        "epic" => MasdarLuba::Epic(muarrif.to_owned()),
        "gog" => MasdarLuba::Gog(muarrif.parse::<u64>().map_err(|_| talif())?),
        "ea" => MasdarLuba::Ea(muarrif.to_owned()),
        "ubisoft" => MasdarLuba::Ubisoft(muarrif.parse::<u32>().map_err(|_| talif())?),
        "battlenet" => MasdarLuba::BattleNet(muarrif.to_owned()),
        "xbox" => MasdarLuba::Xbox(muarrif.to_owned()),
        "itch" => MasdarLuba::Itch(muarrif.parse::<i64>().map_err(|_| talif())?),
        "yadawi" => MasdarLuba::Yadawi(muarrif.to_owned()),
        _ => {
            return Err(Khata::from(KhataMakhzan::SafTalif {
                jadwal: "masdar_luba",
                amud: "aila",
                qeema: aila.to_owned(),
            }));
        },
    };

    Ok(if mudir == Some("heroic") {
        MasdarLuba::Heroic(Box::new(asl))
    } else {
        asl
    })
}

/// Splits a compatibility environment into the three columns `luba` holds.
fn fakkik_beea(beea: &BeeatTawafuq) -> Natija<(&'static str, Option<String>, Option<String>)> {
    Ok(match beea {
        BeeatTawafuq::Asli => ("asli", None, None),
        BeeatTawafuq::Rosetta => ("rosetta", None, None),
        BeeatTawafuq::Proton { isdar, beea } => (
            "proton",
            Some(isdar.clone()),
            Some(masar_nass("beea_jidhr", beea)?.to_owned()),
        ),
        BeeatTawafuq::Wine { isdar, beea } => (
            "wine",
            isdar.clone(),
            Some(masar_nass("beea_jidhr", beea)?.to_owned()),
        ),
    })
}

/// Rebuilds a compatibility environment from its three columns.
///
/// A prefix environment with no prefix is refused rather than filled in with a
/// guess: an installer handed an invented prefix root writes a Windows framework
/// into a directory that is not the game's.
fn ijma_beea(naw: &str, isdar: Option<String>, jidhr: Option<String>) -> Natija<BeeatTawafuq> {
    let mafqud = |amud: &'static str| {
        Khata::from(KhataMakhzan::SafTalif {
            jadwal: "luba",
            amud,
            qeema: "NULL".to_owned(),
        })
    };

    match naw {
        "asli" => Ok(BeeatTawafuq::Asli),
        "rosetta" => Ok(BeeatTawafuq::Rosetta),
        "proton" => Ok(BeeatTawafuq::Proton {
            isdar: isdar.ok_or_else(|| mafqud("beea_isdar"))?,
            beea: PathBuf::from(jidhr.ok_or_else(|| mafqud("beea_jidhr"))?),
        }),
        "wine" => Ok(BeeatTawafuq::Wine {
            isdar,
            beea: PathBuf::from(jidhr.ok_or_else(|| mafqud("beea_jidhr"))?),
        }),
        _ => Err(Khata::from(KhataMakhzan::SafTalif {
            jadwal: "luba",
            amud: "beea_naw",
            qeema: naw.to_owned(),
        })),
    }
}

/// Reads a colour from three adjacent columns, treating an absent channel as an
/// absent colour.
fn lawn_min_saf(saf: &Row<'_>, min: usize) -> rusqlite::Result<Option<LawnBariz>> {
    let ahmar: Option<i64> = saf.get(min)?;
    let akhdar: Option<i64> = saf.get(min + 1)?;
    let azraq: Option<i64> = saf.get(min + 2)?;
    Ok(match (ahmar, akhdar, azraq) {
        (Some(a), Some(kh), Some(z)) => Some(LawnBariz {
            ahmar: u8::try_from(a).unwrap_or(0),
            akhdar: u8::try_from(kh).unwrap_or(0),
            azraq: u8::try_from(z).unwrap_or(0),
        }),
        _ => None,
    })
}

/// Reads the seven coverage counters starting at a column offset.
fn taghtiya_min_saf(saf: &Row<'_>, min: usize) -> rusqlite::Result<Taghtiya> {
    let raqm32 = |q: i64| u32::try_from(q).unwrap_or(0);
    let raqm64 = |q: i64| u64::try_from(q).unwrap_or(0);
    Ok(Taghtiya {
        majmu: raqm32(saf.get(min)?),
        mutarjam: raqm32(saf.get(min + 1)?),
        muakkad: raqm32(saf.get(min + 2)?),
        majmu_takrar: raqm64(saf.get(min + 3)?),
        mutarjam_takrar: raqm64(saf.get(min + 4)?),
        majmu_awwal: raqm32(saf.get(min + 5)?),
        mutarjam_awwal: raqm32(saf.get(min + 6)?),
    })
}

/// Reads a fingerprint from a column, refusing anything that is not one.
fn basma_min_nass(jadwal: &'static str, amud: &'static str, nass: &str) -> Natija<Basma> {
    nass.parse::<Basma>().map_err(|_| {
        Khata::from(KhataMakhzan::SafTalif {
            jadwal,
            amud,
            qeema: nass.to_owned(),
        })
    })
}

// ---------------------------------------------------------------------------
// الألعاب — games and their launcher identities
// ---------------------------------------------------------------------------

/// The columns a game listing reads, and the join that resolves its current
/// build.
///
/// One `LEFT JOIN` rather than a second query per game: the build is one row and
/// the grid wants it for every tile, and N+1 queries across four hundred games
/// is four hundred round trips to draw one screen.
macro_rules! jumlat_maktaba {
    ($tarteeb:literal) => {
        concat!(
            "SELECT l.id, l.ism, l.jidhr, l.tanfidhi, l.hajm, l.akhir_laab, l.akhir_tahdith,
                    l.basma_haliya, l.ghilaf, l.batl, l.shiar,
                    l.lawn_ahmar, l.lawn_akhdar, l.lawn_azraq,
                    l.beea_naw, l.beea_isdar, l.beea_jidhr, l.mawjuda, l.mukhfiya,
                    b.manassa, b.adad_malaffat, b.waqt
             FROM luba l
             LEFT JOIN bina b ON b.luba = l.id AND b.basma = l.basma_haliya
             WHERE l.mukhfiya = ?1
               AND (?2 = 0 OR l.mawjuda = 1)
               AND (?3 IS NULL OR EXISTS (
                     SELECT 1 FROM masdar_luba m WHERE m.luba = l.id AND m.aila = ?3))
             ORDER BY ",
            $tarteeb,
            " LIMIT ?4 OFFSET ?5"
        )
    };
}

/// The four orderings, each a whole constant statement.
///
/// Assembled by `concat!` at compile time, never by `format!` at run time. Every
/// filter is a bound parameter that neutralises its own predicate — `?3 IS NULL`
/// switches the launcher filter off — so clicking through filters reuses four
/// prepared statements instead of producing a new statement text per
/// combination.
const MAKTABA_BIL_ISM: &str = jumlat_maktaba!("l.ism_muwahhad ASC");
const MAKTABA_BIL_LAAB: &str = jumlat_maktaba!("l.akhir_laab DESC, l.ism_muwahhad ASC");
const MAKTABA_BIL_TAHDITH: &str = jumlat_maktaba!("l.akhir_tahdith DESC, l.ism_muwahhad ASC");
const MAKTABA_BIL_HAJM: &str = jumlat_maktaba!("l.hajm DESC, l.ism_muwahhad ASC");

/// One `luba` row, before any domain type has been reconstructed from it.
///
/// The rusqlite closure fills this and nothing else. Domain decoding happens
/// outside the closure so that a value the schema does not recognise produces a
/// [`KhataMakhzan::SafTalif`] naming the column, rather than being flattened
/// into `rusqlite::Error::FromSqlConversionFailure` with nothing useful in it.
#[derive(Debug, Clone)]
struct KhaamLuba {
    id: String,
    ism: String,
    jidhr: String,
    tanfidhi: Option<String>,
    hajm: i64,
    akhir_laab: Option<String>,
    akhir_tahdith: Option<String>,
    basma_haliya: Option<String>,
    ghilaf: Option<String>,
    batl: Option<String>,
    shiar: Option<String>,
    lawn: Option<LawnBariz>,
    beea_naw: String,
    beea_isdar: Option<String>,
    beea_jidhr: Option<String>,
    mawjuda: bool,
    mukhfiya: bool,
    bina_manassa: Option<String>,
    bina_adad: Option<i64>,
    bina_waqt: Option<String>,
}

impl KhaamLuba {
    fn min_saf(saf: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: saf.get(0)?,
            ism: saf.get(1)?,
            jidhr: saf.get(2)?,
            tanfidhi: saf.get(3)?,
            hajm: saf.get(4)?,
            akhir_laab: saf.get(5)?,
            akhir_tahdith: saf.get(6)?,
            basma_haliya: saf.get(7)?,
            ghilaf: saf.get(8)?,
            batl: saf.get(9)?,
            shiar: saf.get(10)?,
            lawn: lawn_min_saf(saf, 11)?,
            beea_naw: saf.get(14)?,
            beea_isdar: saf.get(15)?,
            beea_jidhr: saf.get(16)?,
            mawjuda: saf.get::<_, i64>(17)? != 0,
            mukhfiya: saf.get::<_, i64>(18)? != 0,
            bina_manassa: saf.get(19)?,
            bina_adad: saf.get(20)?,
            bina_waqt: saf.get(21)?,
        })
    }

    fn ila_luba(self, masadir: Vec<MasdarLuba>) -> Natija<Luba> {
        let id: LubaId = min_ramz("luba", "id", &self.id)?;

        let bina = match (self.basma_haliya, self.bina_waqt) {
            (Some(basma), Some(waqt)) => Some(BinaId {
                manassa: self.bina_manassa,
                basma: basma_min_nass("bina", "basma", &basma)?,
                adad_malaffat: u32::try_from(self.bina_adad.unwrap_or(0)).unwrap_or(0),
                waqt,
            }),
            // A current fingerprint with no matching build row cannot happen
            // while the deferred foreign key is enforced, so the pair is simply
            // absent together.
            _ => None,
        };

        Ok(Luba {
            id,
            masadir,
            ism: self.ism,
            jidhr: PathBuf::from(self.jidhr),
            tanfidhi: self.tanfidhi.map(PathBuf::from),
            hajm: u64::try_from(self.hajm).unwrap_or(0),
            akhir_laab: self.akhir_laab,
            akhir_tahdith: self.akhir_tahdith,
            bina,
            suwar: SuwarLuba {
                ghilaf: self.ghilaf,
                batl: self.batl,
                shiar: self.shiar,
                lawn: self.lawn,
            },
            beea: ijma_beea(&self.beea_naw, self.beea_isdar, self.beea_jidhr)?,
            mawjuda: self.mawjuda,
            mukhfiya: self.mukhfiya,
        })
    }
}

/// The games ledger: `luba`, `masdar_luba` and `sima_luba`.
#[derive(Debug, Clone, Copy)]
pub struct SijillAlaab<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillAlaab<'a> {
    /// Binds the ledger to a connection — a pooled one for reading, a
    /// transaction for writing.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Writes a discovered game, its launcher identities and its launcher hints,
    /// and stamps it with the scan generation that found it.
    ///
    /// What an upsert deliberately does **not** overwrite:
    ///
    /// * `mukhfiya` — the user hid this game, and a rescan is not permission to
    ///   unhide it.
    /// * `awwal_ruya` — when it was first seen is a fact about the past.
    /// * `tanfidhi`, `akhir_laab`, `khiyarat_tashghil` — coalesced, so a
    ///   launcher that reports nothing for a field does not erase what another
    ///   launcher reported. A game owned on Steam and Epic where Epic knows no
    ///   last-played date must not lose Steam's.
    /// * the artwork columns — they belong to [`Self::sajjil_suwar`], which runs
    ///   after the grid is already on screen.
    ///
    /// # Errors
    ///
    /// Fails when a path is not valid Unicode, when a statement is rejected, or
    /// when the database is held by another writer past the busy timeout.
    pub fn sajjil(self, idkhal: &IdkhalLuba<'_>) -> Natija<()> {
        let luba = idkhal.luba;
        let (beea_naw, beea_isdar, beea_jidhr) = fakkik_beea(&luba.beea)?;
        let waqt = alaan(self.ittisal)?;
        let fahs = i64::try_from(idkhal.fahs).unwrap_or(i64::MAX);

        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO luba (
                     id, ism, ism_muwahhad, jidhr, tanfidhi, hajm,
                     akhir_laab, akhir_tahdith,
                     beea_naw, beea_isdar, beea_jidhr,
                     mawjuda, mukhfiya, muktamila, khiyarat_tashghil,
                     fahs, awwal_ruya, akhir_ruya)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                         1, ?12, ?13, ?14, ?15, ?16, ?16)
                 ON CONFLICT (id) DO UPDATE SET
                     ism               = excluded.ism,
                     ism_muwahhad      = excluded.ism_muwahhad,
                     jidhr             = excluded.jidhr,
                     tanfidhi          = coalesce(excluded.tanfidhi, luba.tanfidhi),
                     hajm              = excluded.hajm,
                     akhir_laab        = coalesce(excluded.akhir_laab, luba.akhir_laab),
                     akhir_tahdith     = coalesce(excluded.akhir_tahdith, luba.akhir_tahdith),
                     beea_naw          = excluded.beea_naw,
                     beea_isdar        = excluded.beea_isdar,
                     beea_jidhr        = excluded.beea_jidhr,
                     mawjuda           = 1,
                     muktamila         = excluded.muktamila,
                     khiyarat_tashghil = coalesce(excluded.khiyarat_tashghil,
                                                  luba.khiyarat_tashghil),
                     fahs              = excluded.fahs,
                     akhir_ruya        = excluded.akhir_ruya",
            )
            .map_err(|q| khata_jumla("prepare upsert", "luba", q))?;

        let _ = jumla
            .execute(params![
                luba.id.to_string(),
                luba.ism,
                taarib_mustalahat::wahhid_ism(&luba.ism),
                masar_nass("jidhr", &luba.jidhr)?,
                luba.tanfidhi
                    .as_deref()
                    .map(|q| masar_nass("tanfidhi", q))
                    .transpose()?,
                i64::try_from(luba.hajm).unwrap_or(i64::MAX),
                luba.akhir_laab,
                luba.akhir_tahdith,
                beea_naw,
                beea_isdar,
                beea_jidhr,
                i64::from(luba.mukhfiya),
                i64::from(idkhal.muktamila),
                idkhal.khiyarat_tashghil,
                fahs,
                waqt,
            ])
            .map_err(|q| khata_jumla("upsert", "luba", q))?;

        self.sajjil_masadir(luba.id, &luba.masadir, &waqt)?;
        self.sajjil_simat(luba.id, idkhal.simat)?;
        Ok(())
    }

    /// Writes the launcher identities of one game.
    ///
    /// A new identity is appended at the end of the discovery order; an identity
    /// already known keeps the position it was first seen at, because
    /// `Luba::masdar_asli` reads the first one and that decides which registry
    /// shard is searched for patches. An identity that has moved to a different
    /// game — which happens when a launcher renames a title and the derived
    /// identity changes with it — moves rather than duplicating, and takes a
    /// fresh position at the end of its new game's order.
    fn sajjil_masadir(self, luba: LubaId, masadir: &[MasdarLuba], waqt: &str) -> Natija<()> {
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO masdar_luba (luba, aila, muarrif, mudir, tarteeb, waqt)
                 VALUES (?1, ?2, ?3, ?4,
                         (SELECT coalesce(max(m.tarteeb) + 1, 0)
                            FROM masdar_luba m WHERE m.luba = ?1),
                         ?5)
                 ON CONFLICT (aila, muarrif) DO UPDATE SET
                     luba    = excluded.luba,
                     mudir   = excluded.mudir,
                     tarteeb = CASE
                                 WHEN masdar_luba.luba = excluded.luba
                                 THEN masdar_luba.tarteeb
                                 ELSE (SELECT coalesce(max(m.tarteeb) + 1, 0)
                                         FROM masdar_luba m WHERE m.luba = excluded.luba)
                               END",
            )
            .map_err(|q| khata_jumla("prepare upsert", "masdar_luba", q))?;

        for masdar in masadir {
            let (aila, muarrif, mudir) = fakkik_masdar(masdar);
            let _ = jumla
                .execute(params![luba.to_string(), aila, muarrif, mudir, waqt])
                .map_err(|q| khata_jumla("upsert", "masdar_luba", q))?;
        }
        Ok(())
    }

    /// Adds launcher hints without disturbing hints from other launchers.
    ///
    /// Hints accumulate rather than replace, because a per-launcher refresh only
    /// knows about its own. Clearing one launcher's hints before rewriting them
    /// is [`Self::imsah_simat`].
    fn sajjil_simat(self, luba: LubaId, simat: &[SimaMukhzana]) -> Natija<()> {
        if simat.is_empty() {
            return Ok(());
        }
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO sima_luba (luba, aila, naw, qeema)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (luba, aila, naw, qeema) DO NOTHING",
            )
            .map_err(|q| khata_jumla("prepare insert", "sima_luba", q))?;

        for sima in simat {
            let _ = jumla
                .execute(params![luba.to_string(), sima.aila, sima.naw, sima.qeema])
                .map_err(|q| khata_jumla("insert", "sima_luba", q))?;
        }
        Ok(())
    }

    /// Clears one launcher's hints for one game, before that launcher rewrites
    /// them.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected or the database is locked.
    pub fn imsah_simat(self, luba: LubaId, aila: &str) -> Natija<u64> {
        let adad = self
            .ittisal
            .execute(
                "DELETE FROM sima_luba WHERE luba = ?1 AND aila = ?2",
                params![luba.to_string(), aila],
            )
            .map_err(|q| khata_jumla("delete", "sima_luba", q))?;
        Ok(u64::try_from(adad).unwrap_or(0))
    }

    /// Every hint recorded for a game.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn simat(self, luba: LubaId) -> Natija<Vec<SimaMukhzana>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT aila, naw, qeema FROM sima_luba
                 WHERE luba = ?1 ORDER BY aila, naw, qeema",
            )
            .map_err(|q| khata_jumla("prepare", "sima_luba", q))?;

        let sufuf = jumla
            .query_map(params![luba.to_string()], |saf| {
                Ok(SimaMukhzana {
                    aila: saf.get(0)?,
                    naw: saf.get(1)?,
                    qeema: saf.get(2)?,
                })
            })
            .map_err(|q| khata_jumla("query", "sima_luba", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(saf.map_err(|q| khata_jumla("read", "sima_luba", q))?);
        }
        Ok(natija)
    }

    /// The launcher identities of one game, in discovery order.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run, or when a stored identity does not
    /// decode — a launcher family the schema does not define, or a numeric
    /// identifier that is not numeric.
    pub fn masadir(self, luba: LubaId) -> Natija<Vec<MasdarLuba>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT aila, muarrif, mudir FROM masdar_luba
                 WHERE luba = ?1 ORDER BY tarteeb",
            )
            .map_err(|q| khata_jumla("prepare", "masdar_luba", q))?;

        let sufuf = jumla
            .query_map(params![luba.to_string()], |saf| {
                Ok((
                    saf.get::<_, String>(0)?,
                    saf.get::<_, String>(1)?,
                    saf.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|q| khata_jumla("query", "masdar_luba", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let (aila, muarrif, mudir) = saf.map_err(|q| khata_jumla("read", "masdar_luba", q))?;
            natija.push(ijma_masdar(&aila, &muarrif, mudir.as_deref())?);
        }
        Ok(natija)
    }

    /// Which game a launcher identity belongs to, if any.
    ///
    /// This is how a scan decides whether it has met a game before, and it is
    /// one indexed probe of the primary key rather than a walk of the library.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or the stored identity does not decode.
    pub fn bil_masdar(self, masdar: &MasdarLuba) -> Natija<Option<LubaId>> {
        let (aila, muarrif, _) = fakkik_masdar(masdar);
        let nass: Option<String> = self
            .ittisal
            .query_row(
                "SELECT luba FROM masdar_luba WHERE aila = ?1 AND muarrif = ?2",
                params![aila, muarrif],
                |saf| saf.get(0),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "masdar_luba", q))?;

        nass.map(|q| min_ramz("masdar_luba", "luba", &q))
            .transpose()
    }

    /// One game, with its identities and its current build.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn wahida(self, id: LubaId) -> Natija<Option<Luba>> {
        let khaam: Option<KhaamLuba> = self
            .ittisal
            .query_row(
                "SELECT l.id, l.ism, l.jidhr, l.tanfidhi, l.hajm, l.akhir_laab, l.akhir_tahdith,
                        l.basma_haliya, l.ghilaf, l.batl, l.shiar,
                        l.lawn_ahmar, l.lawn_akhdar, l.lawn_azraq,
                        l.beea_naw, l.beea_isdar, l.beea_jidhr, l.mawjuda, l.mukhfiya,
                        b.manassa, b.adad_malaffat, b.waqt
                 FROM luba l
                 LEFT JOIN bina b ON b.luba = l.id AND b.basma = l.basma_haliya
                 WHERE l.id = ?1",
                params![id.to_string()],
                KhaamLuba::min_saf,
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "luba", q))?;

        match khaam {
            Some(khaam) => Ok(Some(khaam.ila_luba(self.masadir(id)?)?)),
            None => Ok(None),
        }
    }

    /// The library listing.
    ///
    /// Two statements for the whole screen: one for the games, one for every
    /// identity of the games returned, stitched in memory. The alternative — a
    /// query per game to fetch its launchers — is four hundred round trips to
    /// draw one grid, and it is the difference between a library that appears
    /// and a library that fills in.
    ///
    /// # Errors
    ///
    /// Fails when a query cannot run or a stored value does not decode.
    pub fn qaima(self, talab: &TalabMaktaba<'_>) -> Natija<Vec<Luba>> {
        let nass = match talab.tarteeb {
            TarteebMaktaba::Ism => MAKTABA_BIL_ISM,
            TarteebMaktaba::AkhirLaab => MAKTABA_BIL_LAAB,
            TarteebMaktaba::AkhirTahdith => MAKTABA_BIL_TAHDITH,
            TarteebMaktaba::Hajm => MAKTABA_BIL_HAJM,
        };

        let hadd = if talab.hadd == 0 {
            -1_i64
        } else {
            i64::from(talab.hadd)
        };

        let mut jumla = self
            .ittisal
            .prepare(nass)
            .map_err(|q| khata_jumla("prepare list", "luba", q))?;

        let sufuf = jumla
            .query_map(
                params![
                    i64::from(talab.mukhfiya),
                    i64::from(talab.mawjuda_faqat),
                    talab.aila,
                    hadd,
                    i64::from(talab.izaha),
                ],
                KhaamLuba::min_saf,
            )
            .map_err(|q| khata_jumla("list", "luba", q))?;

        let mut khaam = Vec::new();
        for saf in sufuf {
            khaam.push(saf.map_err(|q| khata_jumla("read", "luba", q))?);
        }

        let masadir = self.masadir_lil_maktaba(talab)?;

        let mut natija = Vec::with_capacity(khaam.len());
        for wahid in khaam {
            let id: LubaId = min_ramz("luba", "id", &wahid.id)?;
            let li_hadhihi = masadir.get(&id).cloned().unwrap_or_default();
            natija.push(wahid.ila_luba(li_hadhihi)?);
        }
        Ok(natija)
    }

    /// Every launcher identity in the library, grouped by game, in discovery
    /// order.
    ///
    /// Deliberately unfiltered by the listing's own limit: the identities table
    /// is small — a handful of rows per game — and one full ordered read of it
    /// is cheaper than parameterising a second statement to match whichever page
    /// of the grid is on screen.
    fn masadir_lil_maktaba(
        self,
        talab: &TalabMaktaba<'_>,
    ) -> Natija<BTreeMap<LubaId, Vec<MasdarLuba>>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT m.luba, m.aila, m.muarrif, m.mudir
                 FROM masdar_luba m
                 JOIN luba l ON l.id = m.luba
                 WHERE l.mukhfiya = ?1
                 ORDER BY m.luba, m.tarteeb",
            )
            .map_err(|q| khata_jumla("prepare", "masdar_luba", q))?;

        let sufuf = jumla
            .query_map(params![i64::from(talab.mukhfiya)], |saf| {
                Ok((
                    saf.get::<_, String>(0)?,
                    saf.get::<_, String>(1)?,
                    saf.get::<_, String>(2)?,
                    saf.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|q| khata_jumla("query", "masdar_luba", q))?;

        let mut natija: BTreeMap<LubaId, Vec<MasdarLuba>> = BTreeMap::new();
        for saf in sufuf {
            let (luba, aila, muarrif, mudir) =
                saf.map_err(|q| khata_jumla("read", "masdar_luba", q))?;
            let id: LubaId = min_ramz("masdar_luba", "luba", &luba)?;
            natija
                .entry(id)
                .or_default()
                .push(ijma_masdar(&aila, &muarrif, mudir.as_deref())?);
        }
        Ok(natija)
    }

    /// How many games the library holds, hidden ones included.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn adad(self) -> Natija<u64> {
        let adad: i64 = self
            .ittisal
            .query_row("SELECT count(*) FROM luba", [], |saf| saf.get(0))
            .map_err(|q| khata_jumla("count", "luba", q))?;
        Ok(u64::try_from(adad).unwrap_or(0))
    }

    /// Hides or unhides a game.
    ///
    /// # Errors
    ///
    /// Fails when the update cannot run.
    pub fn ghayyir_ikhfa(self, id: LubaId, mukhfiya: bool) -> Natija<()> {
        let _ = self
            .ittisal
            .execute(
                "UPDATE luba SET mukhfiya = ?2 WHERE id = ?1",
                params![id.to_string(), i64::from(mukhfiya)],
            )
            .map_err(|q| khata_jumla("update", "luba", q))?;
        Ok(())
    }

    /// Records the artwork keys and the colour taken from the cover.
    ///
    /// Separate from [`Self::sajjil`] because artwork arrives after the grid,
    /// never before it: the scan writes the game, the grid draws it, and the
    /// artwork stage fills these columns as each image resolves.
    ///
    /// # Errors
    ///
    /// Fails when the update cannot run, and when an artwork key names a cache
    /// entry that does not exist — the foreign key catches a game pointed at an
    /// image nothing ever stored.
    pub fn sajjil_suwar(self, id: LubaId, suwar: &SuwarLuba) -> Natija<()> {
        let (ahmar, akhdar, azraq) = suwar.lawn.map_or((None, None, None), |q| {
            (
                Some(i64::from(q.ahmar)),
                Some(i64::from(q.akhdar)),
                Some(i64::from(q.azraq)),
            )
        });

        let _ = self
            .ittisal
            .execute(
                "UPDATE luba SET
                     ghilaf      = ?2,
                     batl        = ?3,
                     shiar       = ?4,
                     lawn_ahmar  = ?5,
                     lawn_akhdar = ?6,
                     lawn_azraq  = ?7
                 WHERE id = ?1",
                params![
                    id.to_string(),
                    suwar.ghilaf,
                    suwar.batl,
                    suwar.shiar,
                    ahmar,
                    akhdar,
                    azraq,
                ],
            )
            .map_err(|q| khata_jumla("update artwork", "luba", q))?;
        Ok(())
    }

    /// Marks as absent every game of one launcher family that the current scan
    /// did not see — and only when that scan recorded the family's catalogue as
    /// read end to end.
    ///
    /// Absent, not deleted. The ROADMAP requires that a game removed from disk
    /// keeps its patches, its projects and its backups, so that reinstalling it
    /// tomorrow restores the user's Arabic exactly as it was instead of starting
    /// from nothing.
    ///
    /// The guard is in the statement, not left to the caller. "Steam did not
    /// answer" and "the user uninstalled every Steam game" look identical from
    /// this side, and acting on the second when it was the first empties
    /// somebody's library. `fahs_matjar.najah`, written by
    /// [`SijillFahs::sajjil_natijat_matjar`], records which is which, and both
    /// the lookup and the `UPDATE` read it: a scan that never recorded the
    /// launcher, or recorded it as not read whole, gets
    /// [`HasilatMash::Rufidat`] and an untouched table. The rule used to live
    /// only in a comment addressed to the caller, and the caller — which built
    /// its list from whether a launcher's folder existed — did not follow it.
    ///
    /// A game installed through a manager, recorded in `masdar_luba.mudir`, is
    /// swept only when that manager was read whole too: its store's own adapter
    /// never listed it, so the store's silence says nothing about it.
    ///
    /// # Errors
    ///
    /// Fails when the lookup or the update cannot run.
    pub fn allim_ghayr_mawjud(self, fahs: u64, aila: &str) -> Natija<HasilatMash> {
        if !SijillFahs::jadeed(self.ittisal).najah_matjar(fahs, aila)? {
            return Ok(HasilatMash::Rufidat);
        }
        let adad = self
            .ittisal
            .execute(
                "UPDATE luba SET mawjuda = 0
                 WHERE fahs < ?1
                   AND mawjuda = 1
                   AND EXISTS (SELECT 1 FROM fahs_matjar f
                                WHERE f.fahs = ?1 AND f.aila = ?2 AND f.najah = 1)
                   AND EXISTS (SELECT 1 FROM masdar_luba m
                                WHERE m.luba = luba.id
                                  AND m.aila = ?2
                                  AND (m.mudir IS NULL
                                       OR EXISTS (SELECT 1 FROM fahs_matjar w
                                                   WHERE w.fahs = ?1
                                                     AND w.aila = m.mudir
                                                     AND w.najah = 1)))",
                params![i64::try_from(fahs).unwrap_or(i64::MAX), aila],
            )
            .map_err(|q| khata_jumla("sweep", "luba", q))?;
        Ok(HasilatMash::Jarat {
            adad: u64::try_from(adad).unwrap_or(0),
        })
    }
}

/// What an absence sweep did, as a value beside the count.
///
/// Two outcomes that a bare count collapsed: zero rows swept because every
/// stored game was seen again, and zero rows swept because the launcher was
/// never read this scan. The first is a library in order; the second is a
/// launcher the interface must not describe as searched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HasilatMash {
    /// The launcher's catalogue was read end to end in this scan, and this many
    /// games it no longer lists were marked absent.
    Jarat {
        /// Rows marked absent.
        adad: u64,
    },
    /// The scan recorded no complete read of this launcher's catalogue, so
    /// nothing was touched.
    Rufidat,
}

impl HasilatMash {
    /// Rows marked absent; zero for a refused sweep.
    #[must_use]
    pub const fn adad(self) -> u64 {
        match self {
            Self::Jarat { adad } => adad,
            Self::Rufidat => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// البناء — builds
// ---------------------------------------------------------------------------

/// The builds ledger: `bina`, and the pointer on `luba` that names the current
/// one.
#[derive(Debug, Clone, Copy)]
pub struct SijillBina<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillBina<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Records a build and makes it the game's current one.
    ///
    /// The row is keyed by the fingerprint. A store that pushed an update which
    /// changed no text-bearing file produces the same fingerprint under a new
    /// launcher build identifier, and that updates the identifier in place
    /// rather than adding a row — which is exactly the evidence
    /// [`MutabaqaBina::Basma`] is built on, and the reason a patch survives a
    /// routine store update instead of dying with it.
    ///
    /// # Errors
    ///
    /// Fails when the fingerprint is not 64 lowercase hexadecimal characters
    /// (the `CHECK` refuses it), when the game does not exist, or when the
    /// database is locked.
    pub fn sajjil(self, luba: LubaId, bina: &BinaId) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO bina (luba, basma, manassa, adad_malaffat, waqt, akhir_ruya)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (luba, basma) DO UPDATE SET
                     manassa       = coalesce(excluded.manassa, bina.manassa),
                     adad_malaffat = excluded.adad_malaffat,
                     akhir_ruya    = excluded.akhir_ruya",
            )
            .map_err(|q| khata_jumla("prepare upsert", "bina", q))?;

        let _ = jumla
            .execute(params![
                luba.to_string(),
                bina.basma.to_string(),
                bina.manassa,
                i64::from(bina.adad_malaffat),
                bina.waqt,
                waqt,
            ])
            .map_err(|q| khata_jumla("upsert", "bina", q))?;

        self.ijal_haliya(luba, &bina.basma)
    }

    /// Points a game at one of its recorded builds.
    ///
    /// # Errors
    ///
    /// Fails when the update cannot run, and — at commit — when the fingerprint
    /// names no build of that game, because the constraint between the two is
    /// deferred to the commit rather than skipped.
    pub fn ijal_haliya(self, luba: LubaId, basma: &Basma) -> Natija<()> {
        let _ = self
            .ittisal
            .execute(
                "UPDATE luba SET basma_haliya = ?2 WHERE id = ?1",
                params![luba.to_string(), basma.to_string()],
            )
            .map_err(|q| khata_jumla("update current build", "luba", q))?;
        Ok(())
    }

    /// The build a game is currently at.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or the stored fingerprint does not decode.
    pub fn haliya(self, luba: LubaId) -> Natija<Option<BinaId>> {
        let khaam: Option<(String, Option<String>, i64, String)> = self
            .ittisal
            .query_row(
                "SELECT b.basma, b.manassa, b.adad_malaffat, b.waqt
                 FROM luba l JOIN bina b ON b.luba = l.id AND b.basma = l.basma_haliya
                 WHERE l.id = ?1",
                params![luba.to_string()],
                |saf| Ok((saf.get(0)?, saf.get(1)?, saf.get(2)?, saf.get(3)?)),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup current build", "bina", q))?;

        khaam
            .map(|(basma, manassa, adad, waqt)| {
                Ok(BinaId {
                    manassa,
                    basma: basma_min_nass("bina", "basma", &basma)?,
                    adad_malaffat: u32::try_from(adad).unwrap_or(0),
                    waqt,
                })
            })
            .transpose()
    }

    /// Every build of a game, most recently seen first.
    ///
    /// Kept rather than overwritten because Phase 15 needs to know what a build
    /// changed *from* in order to decide whether a patch still applies.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored fingerprint does not decode.
    pub fn tarikh(self, luba: LubaId) -> Natija<Vec<BinaId>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT basma, manassa, adad_malaffat, waqt FROM bina
                 WHERE luba = ?1 ORDER BY akhir_ruya DESC",
            )
            .map_err(|q| khata_jumla("prepare", "bina", q))?;

        let sufuf = jumla
            .query_map(params![luba.to_string()], |saf| {
                Ok((
                    saf.get::<_, String>(0)?,
                    saf.get::<_, Option<String>>(1)?,
                    saf.get::<_, i64>(2)?,
                    saf.get::<_, String>(3)?,
                ))
            })
            .map_err(|q| khata_jumla("query", "bina", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let (basma, manassa, adad, waqt) = saf.map_err(|q| khata_jumla("read", "bina", q))?;
            natija.push(BinaId {
                manassa,
                basma: basma_min_nass("bina", "basma", &basma)?,
                adad_malaffat: u32::try_from(adad).unwrap_or(0),
                waqt,
            });
        }
        Ok(natija)
    }
}

// ---------------------------------------------------------------------------
// المحرّك — engine capability reports
// ---------------------------------------------------------------------------

/// The engine reports ledger: `bitaqa_muharrik`.
#[derive(Debug, Clone, Copy)]
pub struct SijillMuharrik<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillMuharrik<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Writes the capability report for a game, replacing whatever was there.
    ///
    /// One report per game, always the current one. History is not kept: a
    /// superseded report describes a probe that no longer exists and a build
    /// that may no longer be installed, and keeping it would invite something to
    /// read it.
    ///
    /// # Errors
    ///
    /// Fails when the report cannot be serialized, when a value the schema
    /// constrains is out of range, or when the statement is rejected.
    pub fn sajjil(
        self,
        luba: LubaId,
        taqreer: &TaqreerImkaniyat,
        basma_bina: Option<&Basma>,
    ) -> Natija<()> {
        let muharrik = &taqreer.muharrik;
        let jism = serde_json::to_string(taqreer).map_err(|_| {
            Khata::from(KhataMakhzan::SafTalif {
                jadwal: "bitaqa_muharrik",
                amud: "taqreer",
                qeema: "a report that does not serialize".to_owned(),
            })
        })?;

        let (kabir, sagheer, tasheeh, khaam) =
            muharrik
                .isdar
                .as_ref()
                .map_or((None, None, None, None), |q| {
                    (
                        Some(i64::from(q.kabir)),
                        Some(i64::from(q.sagheer)),
                        Some(i64::from(q.tasheeh)),
                        Some(q.khaam.clone()),
                    )
                });

        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO bitaqa_muharrik (
                     luba, basma_bina, aila, isdar_khaam,
                     isdar_kabir, isdar_sagheer, isdar_tasheeh,
                     khalfiya, mimariya, thiqa, tabaqa, jawda, marfuda,
                     isdar_fahs, waqt, taqreer)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                 ON CONFLICT (luba) DO UPDATE SET
                     basma_bina    = excluded.basma_bina,
                     aila          = excluded.aila,
                     isdar_khaam   = excluded.isdar_khaam,
                     isdar_kabir   = excluded.isdar_kabir,
                     isdar_sagheer = excluded.isdar_sagheer,
                     isdar_tasheeh = excluded.isdar_tasheeh,
                     khalfiya      = excluded.khalfiya,
                     mimariya      = excluded.mimariya,
                     thiqa         = excluded.thiqa,
                     tabaqa        = excluded.tabaqa,
                     jawda         = excluded.jawda,
                     marfuda       = excluded.marfuda,
                     isdar_fahs    = excluded.isdar_fahs,
                     waqt          = excluded.waqt,
                     taqreer       = excluded.taqreer",
            )
            .map_err(|q| khata_jumla("prepare upsert", "bitaqa_muharrik", q))?;

        let _ = jumla
            .execute(params![
                luba.to_string(),
                basma_bina.map(ToString::to_string),
                ila_ramz("bitaqa_muharrik", "aila", &muharrik.aila)?,
                khaam,
                kabir,
                sagheer,
                tasheeh,
                ila_ramz("bitaqa_muharrik", "khalfiya", &muharrik.khalfiya)?,
                ila_ramz("bitaqa_muharrik", "mimariya", &muharrik.mimariya)?,
                i64::from(muharrik.thiqa),
                ila_ramz("bitaqa_muharrik", "tabaqa", &taqreer.tabaqa)?,
                ila_ramz("bitaqa_muharrik", "jawda", &taqreer.jawda)?,
                i64::from(taqreer.marfuda),
                i64::from(taqreer.isdar_fahs),
                taqreer.waqt,
                jism,
            ])
            .map_err(|q| khata_jumla("upsert", "bitaqa_muharrik", q))?;
        Ok(())
    }

    /// The capability report for a game.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run, or when the stored document does not
    /// parse back into a report — which means the row was written by a build
    /// whose report shape differed, and is refused rather than half-read.
    pub fn wahid(self, luba: LubaId) -> Natija<Option<TaqreerImkaniyat>> {
        let jism: Option<String> = self
            .ittisal
            .query_row(
                "SELECT taqreer FROM bitaqa_muharrik WHERE luba = ?1",
                params![luba.to_string()],
                |saf| saf.get(0),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "bitaqa_muharrik", q))?;

        jism.map(|nass| {
            serde_json::from_str::<TaqreerImkaniyat>(&nass).map_err(|q| {
                Khata::from(KhataMakhzan::SafTalif {
                    jadwal: "bitaqa_muharrik",
                    amud: "taqreer",
                    qeema: q.to_string(),
                })
            })
        })
        .transpose()
    }

    /// The tier recorded for a game, without parsing the whole report.
    ///
    /// The library grid needs the tier per tile to badge it and needs nothing
    /// else from the report, so it reads the column instead of deserializing a
    /// document per row.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or the stored tier does not decode.
    pub fn tabaqa(self, luba: LubaId) -> Natija<Option<Tabaqa>> {
        let nass: Option<String> = self
            .ittisal
            .query_row(
                "SELECT tabaqa FROM bitaqa_muharrik WHERE luba = ?1",
                params![luba.to_string()],
                |saf| saf.get(0),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup tier", "bitaqa_muharrik", q))?;

        nass.map(|q| min_ramz("bitaqa_muharrik", "tabaqa", &q))
            .transpose()
    }

    /// The games an older probe examined, which this build should re-examine.
    ///
    /// This is why the probe stamps its own version into every report: an update
    /// that improves detection re-probes the library instead of serving a
    /// conclusion it no longer agrees with. One indexed query at startup, not a
    /// walk of every report.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored identity does not decode.
    pub fn qadima(self, isdar_fahs: u32) -> Natija<Vec<LubaId>> {
        self.huwiyat(
            "SELECT luba FROM bitaqa_muharrik WHERE isdar_fahs < ?1",
            params![i64::from(isdar_fahs)],
        )
    }

    /// The probe version stamped on every stored report, keyed by game.
    ///
    /// One query for the whole library, for the sweep that decides which games
    /// to re-examine: a game with no report and a game an older probe examined
    /// are the same decision, and [`Self::qadima`] answers only the second.
    /// Answering the first with [`Self::wahid`] per game would parse a whole
    /// document per row to learn whether the row exists.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run, or a stored identity or version does
    /// not decode.
    pub fn isdarat(self) -> Natija<BTreeMap<LubaId, u32>> {
        let mut jumla = self
            .ittisal
            .prepare("SELECT luba, isdar_fahs FROM bitaqa_muharrik")
            .map_err(|q| khata_jumla("prepare versions", "bitaqa_muharrik", q))?;

        let sufuf = jumla
            .query_map(params![], |saf| {
                Ok((saf.get::<_, String>(0)?, saf.get::<_, i64>(1)?))
            })
            .map_err(|q| khata_jumla("query versions", "bitaqa_muharrik", q))?;

        let mut isdarat = BTreeMap::new();
        for saf in sufuf {
            let (luba, isdar) =
                saf.map_err(|q| khata_jumla("read versions", "bitaqa_muharrik", q))?;
            let isdar = u32::try_from(isdar).map_err(|_| {
                Khata::from(KhataMakhzan::SafTalif {
                    jadwal: "bitaqa_muharrik",
                    amud: "isdar_fahs",
                    qeema: isdar.to_string(),
                })
            })?;
            let _ = isdarat.insert(min_ramz("bitaqa_muharrik", "luba", &luba)?, isdar);
        }
        Ok(isdarat)
    }

    /// Every game the safety layer refuses outright.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored identity does not decode.
    pub fn marfuda(self) -> Natija<Vec<LubaId>> {
        self.huwiyat(
            "SELECT luba FROM bitaqa_muharrik WHERE marfuda = 1",
            params![],
        )
    }

    /// The cheap install-root signature the probe took with the current report.
    ///
    /// Kept in `halat` under [`miftah::basmat_jidhr`] rather than in a column of
    /// `bitaqa_muharrik`, and not in `basma_bina`, which is a different thing
    /// entirely: that column is the *build* fingerprint, it is foreign-keyed to
    /// `bina`, and `bina` is what a patch binds to. The engine probe runs for
    /// every game at launch and cannot hash a sixty-gigabyte install to decide
    /// whether to re-run itself, so it records something far cheaper — the
    /// executable's size and modification time and how many entries the install
    /// root holds. Putting that in `basma_bina` would either be refused by the
    /// foreign key or, if a row were invented to satisfy it, would make a patch
    /// match a build it does not fit.
    ///
    /// The value is opaque to this crate. Its only operation is equality, its
    /// composition belongs to `taarib-muharrik`, and nothing here parses it.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn basmat_jidhr(self, luba: LubaId) -> Natija<Option<String>> {
        SijillHalat::jadeed(self.ittisal).iqra(&miftah::basmat_jidhr(luba))
    }

    /// Records the signature that goes with the current report, or clears it
    /// when the probe could not take one.
    ///
    /// Cleared rather than left behind, for the same reason [`Self::sajjil`]
    /// keeps one report per game: a signature taken with a superseded report
    /// describes a directory state no current record describes, and a later
    /// comparison against it would announce a change nothing observed.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected.
    pub fn sajjil_basmat_jidhr(self, luba: LubaId, basma: Option<&str>) -> Natija<()> {
        let halat = SijillHalat::jadeed(self.ittisal);
        let miftah_luba = miftah::basmat_jidhr(luba);
        match basma {
            Some(qeema) => halat.iktub(&miftah_luba, qeema),
            None => halat.ihdhif(&miftah_luba),
        }
    }

    /// Forgets one game's report and the signature taken with it.
    ///
    /// Deleted rather than blanked, so that [`Self::wahid`] answers `None` and
    /// the probe's next decision is "never examined" — which is the honest
    /// state to re-probe from. A report emptied field by field would be a report
    /// that still exists and still gets read.
    ///
    /// # Errors
    ///
    /// Fails when a statement is rejected or the database is held by another
    /// writer past the busy timeout.
    pub fn ihdhif(self, luba: LubaId) -> Natija<()> {
        let _ = self
            .ittisal
            .execute(
                "DELETE FROM bitaqa_muharrik WHERE luba = ?1",
                params![luba.to_string()],
            )
            .map_err(|q| khata_jumla("delete", "bitaqa_muharrik", q))?;
        self.sajjil_basmat_jidhr(luba, None)
    }

    /// Runs a statement that yields one identity column.
    fn huwiyat(self, nass: &'static str, muamalat: &[&dyn rusqlite::ToSql]) -> Natija<Vec<LubaId>> {
        let mut jumla = self
            .ittisal
            .prepare(nass)
            .map_err(|q| khata_jumla("prepare", "bitaqa_muharrik", q))?;

        let sufuf = jumla
            .query_map(muamalat, |saf| saf.get::<_, String>(0))
            .map_err(|q| khata_jumla("query", "bitaqa_muharrik", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let nass = saf.map_err(|q| khata_jumla("read", "bitaqa_muharrik", q))?;
            natija.push(min_ramz("bitaqa_muharrik", "luba", &nass)?);
        }
        Ok(natija)
    }
}

// ---------------------------------------------------------------------------
// الصور — the artwork cache index
// ---------------------------------------------------------------------------

/// One cached artwork file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuratMukhzana {
    /// The content address, which is also the file name under `makhbaa/`.
    pub miftah: String,
    /// Which of the three artwork slots it fills: `ghilaf`, `batl`, `shiar`.
    pub naw: String,
    /// The launcher cache file or address it came from, so a damaged copy can be
    /// fetched again from where it was found.
    pub masdar: Option<String>,
    /// Size in bytes.
    pub hajm: u64,
    /// Pixel width, when it is known.
    pub ard: Option<u32>,
    /// Pixel height, when it is known.
    pub irtifa: Option<u32>,
}

/// The artwork ledger: `sura`.
#[derive(Debug, Clone, Copy)]
pub struct SijillSuwar<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillSuwar<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Records a cached image, or refreshes what is known about one.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected — including when `naw` is not one of
    /// the three artwork slots the `CHECK` allows.
    pub fn sajjil(self, sura: &SuratMukhzana) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO sura (miftah, naw, masdar, hajm, ard, irtifa, waqt, akhir_istikhdam)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                 ON CONFLICT (miftah) DO UPDATE SET
                     naw             = excluded.naw,
                     masdar          = coalesce(excluded.masdar, sura.masdar),
                     hajm            = excluded.hajm,
                     ard             = coalesce(excluded.ard, sura.ard),
                     irtifa          = coalesce(excluded.irtifa, sura.irtifa),
                     akhir_istikhdam = excluded.akhir_istikhdam",
            )
            .map_err(|q| khata_jumla("prepare upsert", "sura", q))?;

        let _ = jumla
            .execute(params![
                sura.miftah,
                sura.naw,
                sura.masdar,
                i64::try_from(sura.hajm).unwrap_or(i64::MAX),
                sura.ard.map(i64::from),
                sura.irtifa.map(i64::from),
                waqt,
            ])
            .map_err(|q| khata_jumla("upsert", "sura", q))?;
        Ok(())
    }

    /// Marks an image as used now, which is what keeps it out of the eviction
    /// list.
    ///
    /// # Errors
    ///
    /// Fails when the update cannot run.
    pub fn lamas(self, miftah: &str) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "UPDATE sura SET akhir_istikhdam = ?2 WHERE miftah = ?1",
                params![miftah, waqt],
            )
            .map_err(|q| khata_jumla("touch", "sura", q))?;
        Ok(())
    }

    /// The least recently used images, oldest first, for eviction.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn aqdam(self, hadd: u32) -> Natija<Vec<String>> {
        let mut jumla = self
            .ittisal
            .prepare("SELECT miftah FROM sura ORDER BY akhir_istikhdam ASC LIMIT ?1")
            .map_err(|q| khata_jumla("prepare", "sura", q))?;

        let sufuf = jumla
            .query_map(params![i64::from(hadd)], |saf| saf.get::<_, String>(0))
            .map_err(|q| khata_jumla("query", "sura", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(saf.map_err(|q| khata_jumla("read", "sura", q))?);
        }
        Ok(natija)
    }

    /// Forgets a cached image.
    ///
    /// Any game pointing at it has its artwork key set to NULL by the foreign
    /// key, so the grid falls back to the colour placeholder rather than
    /// requesting a file that is no longer there.
    ///
    /// # Errors
    ///
    /// Fails when the delete cannot run.
    pub fn ihdhif(self, miftah: &str) -> Natija<()> {
        let _ = self
            .ittisal
            .execute("DELETE FROM sura WHERE miftah = ?1", params![miftah])
            .map_err(|q| khata_jumla("delete", "sura", q))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// الفحص — scan runs, their outcomes, and their warnings
// ---------------------------------------------------------------------------

/// What one launcher's part of a scan produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatjarMukhzan {
    /// The launcher, by registry shard family.
    pub aila: String,
    /// Whether the launcher was found installed at all.
    pub mawjud: bool,
    /// Whether its catalogue was read end to end. **This is what decides whether
    /// an absence sweep may run for this family.**
    pub najah: bool,
    /// How many games it produced.
    pub adad_alaab: u32,
    /// How many entries degraded into warnings.
    pub adad_tanbihat: u32,
    /// How long it took, in milliseconds. A launcher that takes seconds is a
    /// launcher whose catalogue is on a slow or disconnected drive, and that is
    /// worth seeing in Diagnostics.
    pub muddat_milli: u64,
}

/// One catalogue entry a scan could not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TanbihMukhzan {
    /// The launcher whose catalogue it came from.
    pub aila: String,
    /// The file or entry, as specifically as it can be named.
    pub mawdi: String,
    /// What was wrong with it, in a sentence.
    pub sabab: String,
}

/// A scan run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FahsMukhzan {
    /// The generation number, which is what games are stamped with.
    pub raqm: u64,
    /// When it started, RFC 3339.
    pub bidaya: String,
    /// When it finished, or `None` while it is still running.
    pub nihaya: Option<String>,
    /// Whether it covered every launcher or refreshed one.
    pub kamil: bool,
    /// How many games it saw.
    pub adad_alaab: u32,
}

/// The scan ledger: `fahs`, `fahs_matjar`, `tanbih_fahs`, `jidhr_maktaba`.
#[derive(Debug, Clone, Copy)]
pub struct SijillFahs<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillFahs<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Opens a scan and returns its generation number.
    ///
    /// The number is what every game the scan touches is stamped with, and what
    /// the absence sweep compares against afterwards. It comes from
    /// `AUTOINCREMENT`, so it never goes backwards even after rows are pruned —
    /// a reused generation would make a sweep treat games from an older scan as
    /// current.
    ///
    /// # Errors
    ///
    /// Fails when the insert cannot run.
    pub fn ibda(self, kamil: bool) -> Natija<u64> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "INSERT INTO fahs (bidaya, kamil) VALUES (?1, ?2)",
                params![waqt, i64::from(kamil)],
            )
            .map_err(|q| khata_jumla("insert", "fahs", q))?;
        Ok(u64::try_from(self.ittisal.last_insert_rowid()).unwrap_or(0))
    }

    /// Closes a scan.
    ///
    /// # Errors
    ///
    /// Fails when the update cannot run.
    pub fn anhi(self, fahs: u64, adad_alaab: u32) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "UPDATE fahs SET nihaya = ?2, adad_alaab = ?3 WHERE raqm = ?1",
                params![
                    i64::try_from(fahs).unwrap_or(i64::MAX),
                    waqt,
                    i64::from(adad_alaab)
                ],
            )
            .map_err(|q| khata_jumla("update", "fahs", q))?;
        Ok(())
    }

    /// Records what one launcher contributed to a scan.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected.
    pub fn sajjil_matjar(self, fahs: u64, matjar: &MatjarMukhzan) -> Natija<()> {
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO fahs_matjar
                     (fahs, aila, mawjud, najah, adad_alaab, adad_tanbihat, muddat_milli)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (fahs, aila) DO UPDATE SET
                     mawjud        = excluded.mawjud,
                     najah         = excluded.najah,
                     adad_alaab    = excluded.adad_alaab,
                     adad_tanbihat = excluded.adad_tanbihat,
                     muddat_milli  = excluded.muddat_milli",
            )
            .map_err(|q| khata_jumla("prepare upsert", "fahs_matjar", q))?;

        let _ = jumla
            .execute(params![
                i64::try_from(fahs).unwrap_or(i64::MAX),
                matjar.aila,
                i64::from(matjar.mawjud),
                i64::from(matjar.najah),
                i64::from(matjar.adad_alaab),
                i64::from(matjar.adad_tanbihat),
                i64::try_from(matjar.muddat_milli).unwrap_or(i64::MAX),
            ])
            .map_err(|q| khata_jumla("upsert", "fahs_matjar", q))?;
        Ok(())
    }

    /// Records one launcher's part of a scan together with every warning it
    /// produced, in one call, so that neither is written without the other: a
    /// launcher row with no warnings behind it would say "read whole" about a
    /// catalogue the warnings say was not, and warnings with no row would be
    /// sentences nobody can attribute.
    ///
    /// # Errors
    ///
    /// Fails when a statement is rejected.
    pub fn sajjil_natijat_matjar(
        self,
        fahs: u64,
        matjar: &MatjarMukhzan,
        tanbihat: &[TanbihMukhzan],
    ) -> Natija<()> {
        self.sajjil_matjar(fahs, matjar)?;
        for tanbih in tanbihat {
            self.sajjil_tanbih(fahs, tanbih)?;
        }
        Ok(())
    }

    /// Whether a launcher's catalogue was read end to end in a given scan.
    ///
    /// The question [`SijillAlaab::allim_ghayr_mawjud`] asks before it sweeps,
    /// and the one its `UPDATE` repeats: sweeping on the strength of a catalogue
    /// that failed to open marks a user's whole library absent. `false` for a
    /// launcher the scan never recorded at all.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn najah_matjar(self, fahs: u64, aila: &str) -> Natija<bool> {
        let najah: Option<i64> = self
            .ittisal
            .query_row(
                "SELECT najah FROM fahs_matjar WHERE fahs = ?1 AND aila = ?2",
                params![i64::try_from(fahs).unwrap_or(i64::MAX), aila],
                |saf| saf.get(0),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "fahs_matjar", q))?;
        Ok(najah.unwrap_or(0) != 0)
    }

    /// Records one entry that could not be read.
    ///
    /// # Errors
    ///
    /// Fails when the insert cannot run.
    pub fn sajjil_tanbih(self, fahs: u64, tanbih: &TanbihMukhzan) -> Natija<()> {
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO tanbih_fahs (fahs, aila, mawdi, sabab) VALUES (?1, ?2, ?3, ?4)",
            )
            .map_err(|q| khata_jumla("prepare insert", "tanbih_fahs", q))?;

        let _ = jumla
            .execute(params![
                i64::try_from(fahs).unwrap_or(i64::MAX),
                tanbih.aila,
                tanbih.mawdi,
                tanbih.sabab
            ])
            .map_err(|q| khata_jumla("insert", "tanbih_fahs", q))?;
        Ok(())
    }

    /// What every launcher contributed to a scan, by family name.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn matajir(self, fahs: u64) -> Natija<Vec<MatjarMukhzan>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT aila, mawjud, najah, adad_alaab, adad_tanbihat, muddat_milli
                 FROM fahs_matjar WHERE fahs = ?1 ORDER BY aila",
            )
            .map_err(|q| khata_jumla("prepare", "fahs_matjar", q))?;

        let sufuf = jumla
            .query_map(params![i64::try_from(fahs).unwrap_or(i64::MAX)], |saf| {
                Ok(MatjarMukhzan {
                    aila: saf.get(0)?,
                    mawjud: saf.get::<_, i64>(1)? != 0,
                    najah: saf.get::<_, i64>(2)? != 0,
                    adad_alaab: u32::try_from(saf.get::<_, i64>(3)?).unwrap_or(u32::MAX),
                    adad_tanbihat: u32::try_from(saf.get::<_, i64>(4)?).unwrap_or(u32::MAX),
                    muddat_milli: u64::try_from(saf.get::<_, i64>(5)?).unwrap_or(0),
                })
            })
            .map_err(|q| khata_jumla("query", "fahs_matjar", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(saf.map_err(|q| khata_jumla("read", "fahs_matjar", q))?);
        }
        Ok(natija)
    }

    /// Every warning a scan produced.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn tanbihat(self, fahs: u64) -> Natija<Vec<TanbihMukhzan>> {
        let mut jumla = self
            .ittisal
            .prepare("SELECT aila, mawdi, sabab FROM tanbih_fahs WHERE fahs = ?1 ORDER BY id")
            .map_err(|q| khata_jumla("prepare", "tanbih_fahs", q))?;

        let sufuf = jumla
            .query_map(params![i64::try_from(fahs).unwrap_or(i64::MAX)], |saf| {
                Ok(TanbihMukhzan {
                    aila: saf.get(0)?,
                    mawdi: saf.get(1)?,
                    sabab: saf.get(2)?,
                })
            })
            .map_err(|q| khata_jumla("query", "tanbih_fahs", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(saf.map_err(|q| khata_jumla("read", "tanbih_fahs", q))?);
        }
        Ok(natija)
    }

    /// The most recent scan, running or finished.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn akhir(self) -> Natija<Option<FahsMukhzan>> {
        self.ittisal
            .query_row(
                "SELECT raqm, bidaya, nihaya, kamil, adad_alaab
                 FROM fahs ORDER BY raqm DESC LIMIT 1",
                [],
                |saf| {
                    Ok(FahsMukhzan {
                        raqm: u64::try_from(saf.get::<_, i64>(0)?).unwrap_or(0),
                        bidaya: saf.get(1)?,
                        nihaya: saf.get(2)?,
                        kamil: saf.get::<_, i64>(3)? != 0,
                        adad_alaab: u32::try_from(saf.get::<_, i64>(4)?).unwrap_or(0),
                    })
                },
            )
            .optional()
            .map_err(|q| khata_jumla("lookup latest", "fahs", q))
    }

    /// Records a library root worth watching for changes.
    ///
    /// # Errors
    ///
    /// Fails when the path is not valid Unicode, or the statement is rejected.
    pub fn sajjil_jidhr(self, masar: &Path, aila: &str) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "INSERT INTO jidhr_maktaba (masar, aila, mawjud, akhir_fahs)
                 VALUES (?1, ?2, 1, ?3)
                 ON CONFLICT (masar) DO UPDATE SET
                     aila       = excluded.aila,
                     mawjud     = 1,
                     akhir_fahs = excluded.akhir_fahs",
                params![masar_nass("masar", masar)?, aila, waqt],
            )
            .map_err(|q| khata_jumla("upsert", "jidhr_maktaba", q))?;
        Ok(())
    }

    /// Every library root the watcher should re-establish after a restart.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn judhur(self) -> Natija<Vec<(PathBuf, String)>> {
        let mut jumla = self
            .ittisal
            .prepare("SELECT masar, aila FROM jidhr_maktaba WHERE mawjud = 1 ORDER BY masar")
            .map_err(|q| khata_jumla("prepare", "jidhr_maktaba", q))?;

        let sufuf = jumla
            .query_map([], |saf| {
                Ok((
                    PathBuf::from(saf.get::<_, String>(0)?),
                    saf.get::<_, String>(1)?,
                ))
            })
            .map_err(|q| khata_jumla("query", "jidhr_maktaba", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(saf.map_err(|q| khata_jumla("read", "jidhr_maktaba", q))?);
        }
        Ok(natija)
    }
}

// ---------------------------------------------------------------------------
// الرقع — the registry cache
// ---------------------------------------------------------------------------

/// One cached registry shard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShardMukhzan {
    /// The shard's key, as the registry names it.
    pub miftah: String,
    /// The hash of the shard body, which is what decides whether the copy on
    /// disk is current.
    pub basma: String,
    /// Size in bytes.
    pub hajm: u64,
    /// The entity tag the server sent, so the next check costs a conditional
    /// request and no body.
    pub etag: Option<String>,
    /// When the shard was last modified upstream, RFC 3339.
    pub akhir_tabdeel: Option<String>,
}

/// Rebuilds a licence from the short identifier stored for it.
///
/// The identifier round-trips exactly: the four named licences map back to
/// themselves and anything else is the licence the contributor named, which is
/// what [`RukhsaRuqaa::Ukhra`] is for.
fn ijma_rukhsa(muarrif: &str) -> RukhsaRuqaa {
    match muarrif {
        "CC0-1.0" => RukhsaRuqaa::Cc0,
        "CC-BY-4.0" => RukhsaRuqaa::CcBy,
        "CC-BY-SA-4.0" => RukhsaRuqaa::CcBySa,
        "all-rights-reserved" => RukhsaRuqaa::MilkiyaKhassa,
        akhar => RukhsaRuqaa::Ukhra {
            ism: akhar.to_owned(),
        },
    }
}

/// The patch ledger: `ruqaa`, `ruqaa_bina` and `makhbaa_shard`.
#[derive(Debug, Clone, Copy)]
pub struct SijillRuqaa<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillRuqaa<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Caches one patch summary and the builds it declares it fits.
    ///
    /// `aila` and `muarrif` name the game the patch targets, as the registry
    /// shards on it, so the library grid's badge query is a join on exactly the
    /// two columns `masdar_luba` is keyed by.
    ///
    /// # Errors
    ///
    /// Fails when a value the schema constrains is out of range, or when a
    /// statement is rejected.
    pub fn sajjil(self, mulakhkhas: &MulakhkhasRuqaa, aila: &str, muarrif: &str) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let taghtiya = &mulakhkhas.taghtiya;

        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO ruqaa (
                     id, murajaa, aila, muarrif, unwan, musahim, ism_musahim,
                     adad_nusus, hajm, aila_muharrik, khalfiya, tabaqa, tareeqa,
                     rukhsa, halat, taqyeem, adad_taqyeemat,
                     taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                     taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal, taghtiya_mutarjam_awwal,
                     waqt_nashr, basmat_muhtawa, rabt, rabt_mira, akhir_jalb)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                         ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)
                 ON CONFLICT (id, murajaa) DO UPDATE SET
                     aila                     = excluded.aila,
                     muarrif                  = excluded.muarrif,
                     unwan                    = excluded.unwan,
                     musahim                  = excluded.musahim,
                     ism_musahim              = excluded.ism_musahim,
                     adad_nusus               = excluded.adad_nusus,
                     hajm                     = excluded.hajm,
                     aila_muharrik            = excluded.aila_muharrik,
                     khalfiya                 = excluded.khalfiya,
                     tabaqa                   = excluded.tabaqa,
                     tareeqa                  = excluded.tareeqa,
                     rukhsa                   = excluded.rukhsa,
                     halat                    = excluded.halat,
                     taqyeem                  = excluded.taqyeem,
                     adad_taqyeemat           = excluded.adad_taqyeemat,
                     taghtiya_majmu           = excluded.taghtiya_majmu,
                     taghtiya_mutarjam        = excluded.taghtiya_mutarjam,
                     taghtiya_muakkad         = excluded.taghtiya_muakkad,
                     taghtiya_majmu_takrar    = excluded.taghtiya_majmu_takrar,
                     taghtiya_mutarjam_takrar = excluded.taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal     = excluded.taghtiya_majmu_awwal,
                     taghtiya_mutarjam_awwal  = excluded.taghtiya_mutarjam_awwal,
                     waqt_nashr               = excluded.waqt_nashr,
                     basmat_muhtawa           = excluded.basmat_muhtawa,
                     rabt                     = excluded.rabt,
                     rabt_mira                = excluded.rabt_mira,
                     akhir_jalb               = excluded.akhir_jalb",
            )
            .map_err(|q| khata_jumla("prepare upsert", "ruqaa", q))?;

        let _ = jumla
            .execute(params![
                mulakhkhas.id.to_string(),
                i64::from(mulakhkhas.murajaa.qeema()),
                aila,
                muarrif,
                mulakhkhas.unwan,
                mulakhkhas.musahim.nass(),
                mulakhkhas.ism_musahim,
                i64::from(mulakhkhas.adad_nusus),
                i64::try_from(mulakhkhas.hajm).unwrap_or(i64::MAX),
                ila_ramz("ruqaa", "aila_muharrik", &mulakhkhas.aila)?,
                ila_ramz("ruqaa", "khalfiya", &mulakhkhas.khalfiya)?,
                ila_ramz("ruqaa", "tabaqa", &mulakhkhas.tabaqa)?,
                ila_ramz("ruqaa", "tareeqa", &mulakhkhas.tareeqa)?,
                mulakhkhas.rukhsa.muarrif(),
                // A summary that reached a shard is published by definition —
                // the registry index carries nothing else — so the column is
                // written from the constant rather than from a field the
                // summary does not have.
                ila_ramz("ruqaa", "halat", &HalatRuqaa::Manshura)?,
                mulakhkhas.taqyeem.map(f64::from),
                i64::from(mulakhkhas.adad_taqyeemat),
                i64::from(taghtiya.majmu),
                i64::from(taghtiya.mutarjam),
                i64::from(taghtiya.muakkad),
                i64::try_from(taghtiya.majmu_takrar).unwrap_or(i64::MAX),
                i64::try_from(taghtiya.mutarjam_takrar).unwrap_or(i64::MAX),
                i64::from(taghtiya.majmu_awwal),
                i64::from(taghtiya.mutarjam_awwal),
                mulakhkhas.waqt_nashr,
                mulakhkhas.basmat_muhtawa.to_string(),
                mulakhkhas.rabt,
                mulakhkhas.rabt_mira,
                waqt,
            ])
            .map_err(|q| khata_jumla("upsert", "ruqaa", q))?;

        self.sajjil_bina(mulakhkhas)
    }

    /// Records the launcher build identifiers and content fingerprints a patch
    /// declares.
    fn sajjil_bina(self, mulakhkhas: &MulakhkhasRuqaa) -> Natija<()> {
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO ruqaa_bina (ruqaa, murajaa, naw, qeema)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (ruqaa, murajaa, naw, qeema) DO NOTHING",
            )
            .map_err(|q| khata_jumla("prepare insert", "ruqaa_bina", q))?;

        let id = mulakhkhas.id.to_string();
        let murajaa = i64::from(mulakhkhas.murajaa.qeema());

        for manassa in &mulakhkhas.bina_manassa {
            let _ = jumla
                .execute(params![id, murajaa, "manassa", manassa])
                .map_err(|q| khata_jumla("insert", "ruqaa_bina", q))?;
        }
        for basma in &mulakhkhas.basmat {
            let _ = jumla
                .execute(params![id, murajaa, "basma", basma.to_string()])
                .map_err(|q| khata_jumla("insert", "ruqaa_bina", q))?;
        }
        Ok(())
    }

    /// Every cached patch that targets one game, newest revision first.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn li_luba(self, aila: &str, muarrif: &str) -> Natija<Vec<MulakhkhasRuqaa>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT id, murajaa, aila, muarrif, unwan, musahim, ism_musahim,
                        adad_nusus, hajm, aila_muharrik, khalfiya, tabaqa, tareeqa,
                        rukhsa, halat, taqyeem, adad_taqyeemat,
                        taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                        taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                        taghtiya_majmu_awwal, taghtiya_mutarjam_awwal,
                        waqt_nashr, basmat_muhtawa, rabt, rabt_mira
                 FROM ruqaa
                 WHERE aila = ?1 AND muarrif = ?2
                 ORDER BY waqt_nashr DESC, murajaa DESC",
            )
            .map_err(|q| khata_jumla("prepare", "ruqaa", q))?;

        let sufuf = jumla
            .query_map(params![aila, muarrif], KhaamRuqaa::min_saf)
            .map_err(|q| khata_jumla("query", "ruqaa", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let khaam = saf.map_err(|q| khata_jumla("read", "ruqaa", q))?;
            let bina = self.bina_ruqaa(&khaam.id, khaam.murajaa)?;
            natija.push(khaam.ila_mulakhkhas(bina.0, bina.1)?);
        }
        Ok(natija)
    }

    /// The patch revisions that declare a given build, by either kind of
    /// evidence.
    ///
    /// One indexed probe. This is the lookup behind [`MutabaqaBina`]: the
    /// launcher identifier is tried first and the content fingerprint second,
    /// and a hit on the second is what keeps a patch alive through a store
    /// update that changed no text.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored identity does not decode.
    pub fn bi_bina(self, naw: &str, qeema: &str) -> Natija<Vec<(RuqaaId, RuqaaRevision)>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT ruqaa, murajaa FROM ruqaa_bina
                 WHERE naw = ?1 AND qeema = ?2 ORDER BY murajaa DESC",
            )
            .map_err(|q| khata_jumla("prepare", "ruqaa_bina", q))?;

        let sufuf = jumla
            .query_map(params![naw, qeema], |saf| {
                Ok((saf.get::<_, String>(0)?, saf.get::<_, i64>(1)?))
            })
            .map_err(|q| khata_jumla("query", "ruqaa_bina", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let (id, murajaa) = saf.map_err(|q| khata_jumla("read", "ruqaa_bina", q))?;
            natija.push((
                min_ramz("ruqaa_bina", "ruqaa", &id)?,
                RuqaaRevision::jadeeda(u32::try_from(murajaa).unwrap_or(1)),
            ));
        }
        Ok(natija)
    }

    /// The build evidence one patch revision declares, split into launcher
    /// identifiers and content fingerprints.
    fn bina_ruqaa(self, id: &str, murajaa: i64) -> Natija<(Vec<String>, Vec<Basma>)> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT naw, qeema FROM ruqaa_bina
                 WHERE ruqaa = ?1 AND murajaa = ?2 ORDER BY naw, qeema",
            )
            .map_err(|q| khata_jumla("prepare", "ruqaa_bina", q))?;

        let sufuf = jumla
            .query_map(params![id, murajaa], |saf| {
                Ok((saf.get::<_, String>(0)?, saf.get::<_, String>(1)?))
            })
            .map_err(|q| khata_jumla("query", "ruqaa_bina", q))?;

        let mut manassat = Vec::new();
        let mut basmat = Vec::new();
        for saf in sufuf {
            let (naw, qeema) = saf.map_err(|q| khata_jumla("read", "ruqaa_bina", q))?;
            if naw == "basma" {
                basmat.push(basma_min_nass("ruqaa_bina", "qeema", &qeema)?);
            } else {
                manassat.push(qeema);
            }
        }
        Ok((manassat, basmat))
    }

    /// Records a fetched shard, so the next launch can ask whether it changed
    /// instead of downloading it again.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected.
    pub fn sajjil_shard(self, shard: &ShardMukhzan) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "INSERT INTO makhbaa_shard (miftah, basma, hajm, etag, akhir_jalb, akhir_tabdeel)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (miftah) DO UPDATE SET
                     basma         = excluded.basma,
                     hajm          = excluded.hajm,
                     etag          = excluded.etag,
                     akhir_jalb    = excluded.akhir_jalb,
                     akhir_tabdeel = excluded.akhir_tabdeel",
                params![
                    shard.miftah,
                    shard.basma,
                    i64::try_from(shard.hajm).unwrap_or(i64::MAX),
                    shard.etag,
                    waqt,
                    shard.akhir_tabdeel,
                ],
            )
            .map_err(|q| khata_jumla("upsert", "makhbaa_shard", q))?;
        Ok(())
    }

    /// What is known about a cached shard.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn shard(self, miftah: &str) -> Natija<Option<ShardMukhzan>> {
        self.ittisal
            .query_row(
                "SELECT miftah, basma, hajm, etag, akhir_tabdeel
                 FROM makhbaa_shard WHERE miftah = ?1",
                params![miftah],
                |saf| {
                    Ok(ShardMukhzan {
                        miftah: saf.get(0)?,
                        basma: saf.get(1)?,
                        hajm: u64::try_from(saf.get::<_, i64>(2)?).unwrap_or(0),
                        etag: saf.get(3)?,
                        akhir_tabdeel: saf.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "makhbaa_shard", q))
    }
}

/// One `ruqaa` row, before the domain types are rebuilt from it.
#[derive(Debug, Clone)]
struct KhaamRuqaa {
    id: String,
    murajaa: i64,
    unwan: String,
    musahim: String,
    ism_musahim: String,
    adad_nusus: i64,
    hajm: i64,
    aila_muharrik: String,
    khalfiya: String,
    tabaqa: String,
    tareeqa: String,
    rukhsa: String,
    taqyeem: Option<f64>,
    adad_taqyeemat: i64,
    taghtiya: Taghtiya,
    waqt_nashr: String,
    basmat_muhtawa: String,
    rabt: String,
    rabt_mira: Option<String>,
}

impl KhaamRuqaa {
    fn min_saf(saf: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: saf.get(0)?,
            murajaa: saf.get(1)?,
            unwan: saf.get(4)?,
            musahim: saf.get(5)?,
            ism_musahim: saf.get(6)?,
            adad_nusus: saf.get(7)?,
            hajm: saf.get(8)?,
            aila_muharrik: saf.get(9)?,
            khalfiya: saf.get(10)?,
            tabaqa: saf.get(11)?,
            tareeqa: saf.get(12)?,
            rukhsa: saf.get(13)?,
            taqyeem: saf.get(15)?,
            adad_taqyeemat: saf.get(16)?,
            taghtiya: taghtiya_min_saf(saf, 17)?,
            waqt_nashr: saf.get(24)?,
            basmat_muhtawa: saf.get(25)?,
            rabt: saf.get(26)?,
            rabt_mira: saf.get(27)?,
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "a rating is 0.0 to 5.0 by CHECK constraint, which f32 represents exactly \
                  enough for a star display"
    )]
    fn ila_mulakhkhas(
        self,
        bina_manassa: Vec<String>,
        basmat: Vec<Basma>,
    ) -> Natija<MulakhkhasRuqaa> {
        Ok(MulakhkhasRuqaa {
            id: min_ramz("ruqaa", "id", &self.id)?,
            murajaa: RuqaaRevision::jadeeda(u32::try_from(self.murajaa).unwrap_or(1)),
            unwan: self.unwan,
            musahim: MusahimId::jadeed(self.musahim.clone()).map_err(|_| {
                Khata::from(KhataMakhzan::SafTalif {
                    jadwal: "ruqaa",
                    amud: "musahim",
                    qeema: self.musahim,
                })
            })?,
            ism_musahim: self.ism_musahim,
            taghtiya: self.taghtiya,
            adad_nusus: u32::try_from(self.adad_nusus).unwrap_or(0),
            hajm: u64::try_from(self.hajm).unwrap_or(0),
            bina_manassa,
            basmat,
            aila: min_ramz("ruqaa", "aila_muharrik", &self.aila_muharrik)?,
            khalfiya: min_ramz("ruqaa", "khalfiya", &self.khalfiya)?,
            tabaqa: min_ramz("ruqaa", "tabaqa", &self.tabaqa)?,
            tareeqa: min_ramz("ruqaa", "tareeqa", &self.tareeqa)?,
            rukhsa: ijma_rukhsa(&self.rukhsa),
            // The local mirror of the catalogue has no column for it: this row
            // is what a cached listing renders from, and the credit is read off
            // the served catalogue entry rather than kept in two places that
            // can disagree about who wrote something.
            masdar_khariji: None,
            taqyeem: self.taqyeem.map(|q| q as f32),
            adad_taqyeemat: u32::try_from(self.adad_taqyeemat).unwrap_or(0),
            waqt_nashr: self.waqt_nashr,
            basmat_muhtawa: basma_min_nass("ruqaa", "basmat_muhtawa", &self.basmat_muhtawa)?,
            rabt: self.rabt,
            rabt_mira: self.rabt_mira,
        })
    }
}

// ---------------------------------------------------------------------------
// المساهمون — contributors
// ---------------------------------------------------------------------------

/// The contributors ledger: `musahim`.
#[derive(Debug, Clone, Copy)]
pub struct SijillMusahim<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillMusahim<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Caches a contributor record as the registry published it.
    ///
    /// # Errors
    ///
    /// Fails when the identity is not a key fingerprint (the `CHECK` refuses
    /// it), or when the statement is rejected.
    pub fn sajjil(self, musahim: &Musahim) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let sumaa = &musahim.sumaa;
        let taghtiya = &musahim.majmu_taghtiya;

        let _ = self
            .ittisal
            .execute(
                "INSERT INTO musahim (
                     id, ism, satr_itiraf, rabt,
                     ruqaa_manshura, qubila_bila_taadil, tulib_taadil, marfuda, masbuba,
                     mutawassit_taqyeem, adad_taqyeemat, mundhu,
                     taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                     taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal, taghtiya_mutarjam_awwal, akhir_jalb)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                         ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
                 ON CONFLICT (id) DO UPDATE SET
                     ism                      = excluded.ism,
                     satr_itiraf              = excluded.satr_itiraf,
                     rabt                     = excluded.rabt,
                     ruqaa_manshura           = excluded.ruqaa_manshura,
                     qubila_bila_taadil       = excluded.qubila_bila_taadil,
                     tulib_taadil             = excluded.tulib_taadil,
                     marfuda                  = excluded.marfuda,
                     masbuba                  = excluded.masbuba,
                     mutawassit_taqyeem       = excluded.mutawassit_taqyeem,
                     adad_taqyeemat           = excluded.adad_taqyeemat,
                     mundhu                   = excluded.mundhu,
                     taghtiya_majmu           = excluded.taghtiya_majmu,
                     taghtiya_mutarjam        = excluded.taghtiya_mutarjam,
                     taghtiya_muakkad         = excluded.taghtiya_muakkad,
                     taghtiya_majmu_takrar    = excluded.taghtiya_majmu_takrar,
                     taghtiya_mutarjam_takrar = excluded.taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal     = excluded.taghtiya_majmu_awwal,
                     taghtiya_mutarjam_awwal  = excluded.taghtiya_mutarjam_awwal,
                     akhir_jalb               = excluded.akhir_jalb",
                params![
                    musahim.id.nass(),
                    musahim.ism,
                    musahim.satr_itiraf,
                    musahim.rabt,
                    i64::from(sumaa.ruqaa_manshura),
                    i64::from(sumaa.qubila_bila_taadil),
                    i64::from(sumaa.tulib_taadil),
                    i64::from(sumaa.marfuda),
                    i64::from(sumaa.masbuba),
                    sumaa.mutawassit_taqyeem.map(f64::from),
                    i64::from(sumaa.adad_taqyeemat),
                    musahim.mundhu,
                    i64::from(taghtiya.majmu),
                    i64::from(taghtiya.mutarjam),
                    i64::from(taghtiya.muakkad),
                    i64::try_from(taghtiya.majmu_takrar).unwrap_or(i64::MAX),
                    i64::try_from(taghtiya.mutarjam_takrar).unwrap_or(i64::MAX),
                    i64::from(taghtiya.majmu_awwal),
                    i64::from(taghtiya.mutarjam_awwal),
                    waqt,
                ],
            )
            .map_err(|q| khata_jumla("upsert", "musahim", q))?;
        Ok(())
    }

    /// One contributor.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run, or when the stored identity is not a key
    /// fingerprint.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "an average rating is 0.0 to 5.0 by CHECK constraint"
    )]
    pub fn wahid(self, id: &MusahimId) -> Natija<Option<Musahim>> {
        let khaam = self
            .ittisal
            .query_row(
                "SELECT id, ism, satr_itiraf, rabt,
                        ruqaa_manshura, qubila_bila_taadil, tulib_taadil, marfuda, masbuba,
                        mutawassit_taqyeem, adad_taqyeemat, mundhu,
                        taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                        taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                        taghtiya_majmu_awwal, taghtiya_mutarjam_awwal
                 FROM musahim WHERE id = ?1",
                params![id.nass()],
                |saf| {
                    let raqm32 = |q: i64| u32::try_from(q).unwrap_or(0);
                    Ok((
                        saf.get::<_, String>(0)?,
                        saf.get::<_, String>(1)?,
                        saf.get::<_, Option<String>>(2)?,
                        saf.get::<_, Option<String>>(3)?,
                        Sumaa {
                            ruqaa_manshura: raqm32(saf.get(4)?),
                            qubila_bila_taadil: raqm32(saf.get(5)?),
                            tulib_taadil: raqm32(saf.get(6)?),
                            marfuda: raqm32(saf.get(7)?),
                            masbuba: raqm32(saf.get(8)?),
                            mutawassit_taqyeem: saf.get::<_, Option<f64>>(9)?.map(|q| q as f32),
                            adad_taqyeemat: raqm32(saf.get(10)?),
                        },
                        saf.get::<_, Option<String>>(11)?,
                        taghtiya_min_saf(saf, 12)?,
                    ))
                },
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "musahim", q))?;

        let Some((nass, ism, satr_itiraf, rabt, sumaa, mundhu, majmu_taghtiya)) = khaam else {
            return Ok(None);
        };

        let huwiya = MusahimId::jadeed(nass.clone()).map_err(|_| {
            Khata::from(KhataMakhzan::SafTalif {
                jadwal: "musahim",
                amud: "id",
                qeema: nass,
            })
        })?;

        Ok(Some(Musahim {
            id: huwiya,
            ism,
            satr_itiraf,
            rabt,
            sumaa,
            mundhu,
            majmu_taghtiya,
        }))
    }
}

// ---------------------------------------------------------------------------
// التثبيت — installations and their manifests
// ---------------------------------------------------------------------------

/// Where an installation stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatTathbeet {
    /// Installed and in force. At most one per game, enforced by a partial
    /// unique index rather than by every caller remembering to check.
    Nashit,
    /// Uninstalled, with the originals restored.
    Muzal,
    /// The install failed before it wrote anything that needs undoing.
    Fashil,
    /// The install was interrupted partway. Files were written and the manifest
    /// is incomplete, so the game needs repairing before anything else touches
    /// it — which is why this is a state and not an absence of one.
    Naqis,
}

impl HalatTathbeet {
    /// The text the column holds.
    #[must_use]
    pub const fn ramz(self) -> &'static str {
        match self {
            Self::Nashit => "nashit",
            Self::Muzal => "muzal",
            Self::Fashil => "fashil",
            Self::Naqis => "naqis",
        }
    }

    /// Reads the state back from the column.
    fn min_ramz(nass: &str) -> Natija<Self> {
        match nass {
            "nashit" => Ok(Self::Nashit),
            "muzal" => Ok(Self::Muzal),
            "fashil" => Ok(Self::Fashil),
            "naqis" => Ok(Self::Naqis),
            _ => Err(Khata::from(KhataMakhzan::SafTalif {
                jadwal: "tathbeet",
                amud: "halat",
                qeema: nass.to_owned(),
            })),
        }
    }
}

/// What one installation did to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmalMalaf {
    /// A file Taarib created that was not there before. Removing it undoes it.
    Kutib,
    /// A file that existed and was replaced. Undoing it requires the backup,
    /// which the schema refuses to let this variant be written without.
    Ustubdil,
    /// A directory Taarib created.
    UnshiaMujallad,
}

impl AmalMalaf {
    /// The text the column holds.
    #[must_use]
    pub const fn ramz(self) -> &'static str {
        match self {
            Self::Kutib => "kutib",
            Self::Ustubdil => "ustubdil",
            Self::UnshiaMujallad => "unshia_mujallad",
        }
    }

    /// Reads the action back from the column.
    fn min_ramz(nass: &str) -> Natija<Self> {
        match nass {
            "kutib" => Ok(Self::Kutib),
            "ustubdil" => Ok(Self::Ustubdil),
            "unshia_mujallad" => Ok(Self::UnshiaMujallad),
            _ => Err(Khata::from(KhataMakhzan::SafTalif {
                jadwal: "bayan_tathbeet",
                amud: "amal",
                qeema: nass.to_owned(),
            })),
        }
    }
}

/// One line of an installation manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MudkhalBayan {
    /// The path, relative to the game's root. Absolute paths are refused by
    /// design: a manifest full of them stops working the moment the user moves
    /// the game to another drive.
    pub masar: PathBuf,
    /// What was done to it.
    pub amal: AmalMalaf,
    /// The fingerprint of the original, for verifying a restore.
    pub basma_asl: Option<Basma>,
    /// The fingerprint of what Taarib wrote, for detecting later tampering.
    pub basma_jadeed: Option<Basma>,
    /// The original's size in bytes.
    pub hajm_asl: Option<u64>,
    /// The backup key under `nusakh/`. Required for a replacement and forbidden
    /// otherwise, by `CHECK`.
    pub nuskha: Option<String>,
}

/// An installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TathbeetMukhzan {
    /// The local row identifier.
    pub id: i64,
    /// The game it was installed into.
    pub luba: LubaId,
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision installed.
    pub murajaa: RuqaaRevision,
    /// The tier it installed at.
    pub tabaqa: Tabaqa,
    /// The build it was installed against, so a later build change is detected
    /// by comparing fingerprints rather than by asking the user.
    pub basma_bina: Option<Basma>,
    /// How well the patch matched that build at install time.
    pub mutabaqa: MutabaqaBina,
    /// Where it stands.
    pub halat: HalatTathbeet,
    /// The backup directory under `nusakh/`.
    pub jidhr_nusakh: String,
    /// How many files the manifest holds.
    pub adad_malaffat: u32,
    /// When it was installed, RFC 3339.
    pub waqt: String,
    /// When it was removed, RFC 3339, if it was.
    pub waqt_izala: Option<String>,
}

/// The installations ledger: `tathbeet` and `bayan_tathbeet`.
#[derive(Debug, Clone, Copy)]
pub struct SijillTathbeet<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillTathbeet<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Opens an installation in the [`HalatTathbeet::Naqis`] state and returns
    /// its identifier.
    ///
    /// Deliberately not opened as active. An installation becomes active only
    /// once its files are written and its manifest is complete
    /// ([`Self::nashhit`]); a crash between the two leaves a row that says, in
    /// the database, exactly what it is — a half-applied patch that needs
    /// repairing — instead of a row claiming an install that never finished.
    ///
    /// # Errors
    ///
    /// Fails when the game does not exist, when the identifiers are malformed,
    /// or when the statement is rejected.
    pub fn ibda(
        self,
        luba: LubaId,
        ruqaa: RuqaaId,
        murajaa: RuqaaRevision,
        tabaqa: Tabaqa,
        basma_bina: Option<&Basma>,
        mutabaqa: MutabaqaBina,
        jidhr_nusakh: &str,
    ) -> Natija<i64> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "INSERT INTO tathbeet (
                     luba, ruqaa, murajaa, tabaqa, basma_bina, mutabaqa,
                     halat, jidhr_nusakh, adad_malaffat, waqt)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'naqis', ?7, 0, ?8)",
                params![
                    luba.to_string(),
                    ruqaa.to_string(),
                    i64::from(murajaa.qeema()),
                    ila_ramz("tathbeet", "tabaqa", &tabaqa)?,
                    basma_bina.map(ToString::to_string),
                    ila_ramz("tathbeet", "mutabaqa", &mutabaqa)?,
                    jidhr_nusakh,
                    waqt,
                ],
            )
            .map_err(|q| khata_jumla("insert", "tathbeet", q))?;
        Ok(self.ittisal.last_insert_rowid())
    }

    /// Records one file the installation touched.
    ///
    /// # Errors
    ///
    /// Fails when the path is not valid Unicode, when a replacement is recorded
    /// with no backup — the `CHECK` refuses it, which is the rollback guarantee
    /// enforced by the schema rather than by the installer remembering — or when
    /// the statement is rejected.
    pub fn sajjil_malaf(self, tathbeet: i64, mudkhal: &MudkhalBayan) -> Natija<()> {
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO bayan_tathbeet
                     (tathbeet, masar, amal, basma_asl, basma_jadeed, hajm_asl, nuskha)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT (tathbeet, masar) DO UPDATE SET
                     amal         = excluded.amal,
                     basma_asl    = excluded.basma_asl,
                     basma_jadeed = excluded.basma_jadeed,
                     hajm_asl     = excluded.hajm_asl,
                     nuskha       = excluded.nuskha",
            )
            .map_err(|q| khata_jumla("prepare insert", "bayan_tathbeet", q))?;

        let _ = jumla
            .execute(params![
                tathbeet,
                masar_nass("masar", &mudkhal.masar)?,
                mudkhal.amal.ramz(),
                mudkhal.basma_asl.map(|q| q.to_string()),
                mudkhal.basma_jadeed.map(|q| q.to_string()),
                mudkhal
                    .hajm_asl
                    .map(|q| i64::try_from(q).unwrap_or(i64::MAX)),
                mudkhal.nuskha,
            ])
            .map_err(|q| khata_jumla("insert", "bayan_tathbeet", q))?;
        Ok(())
    }

    /// Marks an installation as complete and in force.
    ///
    /// Fails if the game already has an active installation, because the partial
    /// unique index refuses a second one. Two patches writing the same game's
    /// files is the corruption case this product cannot allow, and the database
    /// is a better place to refuse it than a check somebody might forget.
    ///
    /// # Errors
    ///
    /// Fails when another installation is already active for the same game, or
    /// when the update is rejected.
    pub fn nashhit(self, tathbeet: i64, adad_malaffat: u32) -> Natija<()> {
        let _ = self
            .ittisal
            .execute(
                "UPDATE tathbeet SET halat = 'nashit', adad_malaffat = ?2 WHERE id = ?1",
                params![tathbeet, i64::from(adad_malaffat)],
            )
            .map_err(|q| khata_jumla("activate", "tathbeet", q))?;
        Ok(())
    }

    /// Marks an installation as failed.
    ///
    /// # Errors
    ///
    /// Fails when the update is rejected.
    pub fn afshil(self, tathbeet: i64) -> Natija<()> {
        let _ = self
            .ittisal
            .execute(
                "UPDATE tathbeet SET halat = 'fashil' WHERE id = ?1",
                params![tathbeet],
            )
            .map_err(|q| khata_jumla("fail", "tathbeet", q))?;
        Ok(())
    }

    /// Marks an installation as removed and stamps when.
    ///
    /// The manifest is kept. A user who reinstalls the game wants the same patch
    /// back, and the record of what was replaced is what makes that possible.
    ///
    /// # Errors
    ///
    /// Fails when the update is rejected — including when the removal time and
    /// the state would disagree, which the `CHECK` refuses.
    pub fn azil(self, tathbeet: i64) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "UPDATE tathbeet SET halat = 'muzal', waqt_izala = ?2 WHERE id = ?1",
                params![tathbeet, waqt],
            )
            .map_err(|q| khata_jumla("uninstall", "tathbeet", q))?;
        Ok(())
    }

    /// The installation currently in force on a game, if any.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn nashit(self, luba: LubaId) -> Natija<Option<TathbeetMukhzan>> {
        self.wahid(
            "SELECT id, luba, ruqaa, murajaa, tabaqa, basma_bina, mutabaqa, halat,
                    jidhr_nusakh, adad_malaffat, waqt, waqt_izala
             FROM tathbeet WHERE luba = ?1 AND halat = 'nashit'",
            params![luba.to_string()],
        )
    }

    /// Every installation a game has ever had, newest first.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn tarikh(self, luba: LubaId) -> Natija<Vec<TathbeetMukhzan>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT id, luba, ruqaa, murajaa, tabaqa, basma_bina, mutabaqa, halat,
                        jidhr_nusakh, adad_malaffat, waqt, waqt_izala
                 FROM tathbeet WHERE luba = ?1 ORDER BY waqt DESC",
            )
            .map_err(|q| khata_jumla("prepare", "tathbeet", q))?;

        let sufuf = jumla
            .query_map(params![luba.to_string()], KhaamTathbeet::min_saf)
            .map_err(|q| khata_jumla("query", "tathbeet", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(
                saf.map_err(|q| khata_jumla("read", "tathbeet", q))?
                    .ila_tathbeet()?,
            );
        }
        Ok(natija)
    }

    /// The manifest of one installation, in the order the files were recorded.
    ///
    /// A rollback walks this backwards: the last thing written is the first
    /// thing undone.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn bayan(self, tathbeet: i64) -> Natija<Vec<MudkhalBayan>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT masar, amal, basma_asl, basma_jadeed, hajm_asl, nuskha
                 FROM bayan_tathbeet WHERE tathbeet = ?1 ORDER BY masar",
            )
            .map_err(|q| khata_jumla("prepare", "bayan_tathbeet", q))?;

        let sufuf = jumla
            .query_map(params![tathbeet], |saf| {
                Ok((
                    saf.get::<_, String>(0)?,
                    saf.get::<_, String>(1)?,
                    saf.get::<_, Option<String>>(2)?,
                    saf.get::<_, Option<String>>(3)?,
                    saf.get::<_, Option<i64>>(4)?,
                    saf.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|q| khata_jumla("query", "bayan_tathbeet", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            let (masar, amal, asl, jadeed, hajm, nuskha) =
                saf.map_err(|q| khata_jumla("read", "bayan_tathbeet", q))?;
            natija.push(MudkhalBayan {
                masar: PathBuf::from(masar),
                amal: AmalMalaf::min_ramz(&amal)?,
                basma_asl: asl
                    .map(|q| basma_min_nass("bayan_tathbeet", "basma_asl", &q))
                    .transpose()?,
                basma_jadeed: jadeed
                    .map(|q| basma_min_nass("bayan_tathbeet", "basma_jadeed", &q))
                    .transpose()?,
                hajm_asl: hajm.map(|q| u64::try_from(q).unwrap_or(0)),
                nuskha,
            });
        }
        Ok(natija)
    }

    /// Runs a statement expected to yield at most one installation.
    fn wahid(
        self,
        nass: &'static str,
        muamalat: &[&dyn rusqlite::ToSql],
    ) -> Natija<Option<TathbeetMukhzan>> {
        let khaam = self
            .ittisal
            .query_row(nass, muamalat, KhaamTathbeet::min_saf)
            .optional()
            .map_err(|q| khata_jumla("lookup", "tathbeet", q))?;
        khaam.map(KhaamTathbeet::ila_tathbeet).transpose()
    }
}

/// One `tathbeet` row before its domain types are rebuilt.
#[derive(Debug, Clone)]
struct KhaamTathbeet {
    id: i64,
    luba: String,
    ruqaa: String,
    murajaa: i64,
    tabaqa: String,
    basma_bina: Option<String>,
    mutabaqa: String,
    halat: String,
    jidhr_nusakh: String,
    adad_malaffat: i64,
    waqt: String,
    waqt_izala: Option<String>,
}

impl KhaamTathbeet {
    fn min_saf(saf: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: saf.get(0)?,
            luba: saf.get(1)?,
            ruqaa: saf.get(2)?,
            murajaa: saf.get(3)?,
            tabaqa: saf.get(4)?,
            basma_bina: saf.get(5)?,
            mutabaqa: saf.get(6)?,
            halat: saf.get(7)?,
            jidhr_nusakh: saf.get(8)?,
            adad_malaffat: saf.get(9)?,
            waqt: saf.get(10)?,
            waqt_izala: saf.get(11)?,
        })
    }

    fn ila_tathbeet(self) -> Natija<TathbeetMukhzan> {
        Ok(TathbeetMukhzan {
            id: self.id,
            luba: min_ramz("tathbeet", "luba", &self.luba)?,
            ruqaa: min_ramz("tathbeet", "ruqaa", &self.ruqaa)?,
            murajaa: RuqaaRevision::jadeeda(u32::try_from(self.murajaa).unwrap_or(1)),
            tabaqa: min_ramz("tathbeet", "tabaqa", &self.tabaqa)?,
            basma_bina: self
                .basma_bina
                .map(|q| basma_min_nass("tathbeet", "basma_bina", &q))
                .transpose()?,
            mutabaqa: min_ramz("tathbeet", "mutabaqa", &self.mutabaqa)?,
            halat: HalatTathbeet::min_ramz(&self.halat)?,
            jidhr_nusakh: self.jidhr_nusakh,
            adad_malaffat: u32::try_from(self.adad_malaffat).unwrap_or(0),
            waqt: self.waqt,
            waqt_izala: self.waqt_izala,
        })
    }
}

// ---------------------------------------------------------------------------
// المشاريع — translation projects
// ---------------------------------------------------------------------------

/// A translation project, as the store holds it.
///
/// The project's working files live under `mashari/`; this row is the index over
/// them — what the workspace lists, sorts and reopens without reading a
/// directory of project folders on every launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MashruMukhzan {
    /// The project's identity, a UUID in text form. Minted by the workspace: the
    /// store records identities, it does not invent them.
    pub id: String,
    /// The game being translated, when it is still in the library.
    pub luba: Option<LubaId>,
    /// The game's name, kept denormalized so a project can still say what it is
    /// a translation of after the game is gone.
    pub ism_luba: String,
    /// The title the translator gave it.
    pub unwan: String,
    /// The project directory under `mashari/`.
    pub mujallad: PathBuf,
    /// Source language tag.
    pub lugha_masdar: String,
    /// Target language tag.
    pub lugha_hadaf: String,
    /// Where it stands in the submission workflow.
    pub halat: HalatRuqaa,
    /// The patch lineage it publishes into, once it has one.
    pub ruqaa: Option<RuqaaId>,
    /// The revision it is aimed at.
    pub murajaa: Option<RuqaaRevision>,
    /// The build it was extracted from.
    pub basma_bina: Option<Basma>,
    /// How the translation is being produced.
    pub tareeqa: Option<TareeqaTarjama>,
    /// The licence the translator publishes under.
    pub rukhsa: Option<RukhsaRuqaa>,
    /// The contributor who owns it.
    pub musahim: Option<MusahimId>,
    /// How much of the game it covers.
    pub taghtiya: Taghtiya,
    /// When it was created, RFC 3339.
    pub waqt_insha: String,
    /// When it was last touched, RFC 3339. The workspace opens on this order.
    pub akhir_tabdeel: String,
}

/// The projects ledger: `mashru`.
#[derive(Debug, Clone, Copy)]
pub struct SijillMashari<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillMashari<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Writes a project, creating it or bringing it up to date.
    ///
    /// # Errors
    ///
    /// Fails when the project directory is not valid Unicode, when a constrained
    /// value is out of range, or when the statement is rejected.
    pub fn sajjil(self, mashru: &MashruMukhzan) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let taghtiya = &mashru.taghtiya;

        let _ = self
            .ittisal
            .execute(
                "INSERT INTO mashru (
                     id, luba, ism_luba, unwan, mujallad, lugha_masdar, lugha_hadaf,
                     halat, ruqaa, murajaa, basma_bina, tareeqa, rukhsa, musahim,
                     taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                     taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal, taghtiya_mutarjam_awwal,
                     waqt_insha, akhir_tabdeel)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                         ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)
                 ON CONFLICT (id) DO UPDATE SET
                     luba                     = excluded.luba,
                     ism_luba                 = excluded.ism_luba,
                     unwan                    = excluded.unwan,
                     mujallad                 = excluded.mujallad,
                     lugha_masdar             = excluded.lugha_masdar,
                     lugha_hadaf              = excluded.lugha_hadaf,
                     halat                    = excluded.halat,
                     ruqaa                    = excluded.ruqaa,
                     murajaa                  = excluded.murajaa,
                     basma_bina               = excluded.basma_bina,
                     tareeqa                  = excluded.tareeqa,
                     rukhsa                   = excluded.rukhsa,
                     musahim                  = excluded.musahim,
                     taghtiya_majmu           = excluded.taghtiya_majmu,
                     taghtiya_mutarjam        = excluded.taghtiya_mutarjam,
                     taghtiya_muakkad         = excluded.taghtiya_muakkad,
                     taghtiya_majmu_takrar    = excluded.taghtiya_majmu_takrar,
                     taghtiya_mutarjam_takrar = excluded.taghtiya_mutarjam_takrar,
                     taghtiya_majmu_awwal     = excluded.taghtiya_majmu_awwal,
                     taghtiya_mutarjam_awwal  = excluded.taghtiya_mutarjam_awwal,
                     akhir_tabdeel            = excluded.akhir_tabdeel",
                params![
                    mashru.id,
                    mashru.luba.map(|q| q.to_string()),
                    mashru.ism_luba,
                    mashru.unwan,
                    masar_nass("mujallad", &mashru.mujallad)?,
                    mashru.lugha_masdar,
                    mashru.lugha_hadaf,
                    ila_ramz("mashru", "halat", &mashru.halat)?,
                    mashru.ruqaa.map(|q| q.to_string()),
                    mashru.murajaa.map(|q| i64::from(q.qeema())),
                    mashru.basma_bina.map(|q| q.to_string()),
                    mashru
                        .tareeqa
                        .map(|q| ila_ramz("mashru", "tareeqa", &q))
                        .transpose()?,
                    mashru.rukhsa.as_ref().map(|q| q.muarrif().to_owned()),
                    mashru.musahim.as_ref().map(MusahimId::nass),
                    i64::from(taghtiya.majmu),
                    i64::from(taghtiya.mutarjam),
                    i64::from(taghtiya.muakkad),
                    i64::try_from(taghtiya.majmu_takrar).unwrap_or(i64::MAX),
                    i64::try_from(taghtiya.mutarjam_takrar).unwrap_or(i64::MAX),
                    i64::from(taghtiya.majmu_awwal),
                    i64::from(taghtiya.mutarjam_awwal),
                    mashru.waqt_insha,
                    waqt,
                ],
            )
            .map_err(|q| khata_jumla("upsert", "mashru", q))?;
        Ok(())
    }

    /// One project.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn wahid(self, id: &str) -> Natija<Option<MashruMukhzan>> {
        let khaam = self
            .ittisal
            .query_row(JUMLAT_MASHRU_WAHID, params![id], KhaamMashru::min_saf)
            .optional()
            .map_err(|q| khata_jumla("lookup", "mashru", q))?;
        khaam.map(KhaamMashru::ila_mashru).transpose()
    }

    /// Every project, most recently touched first.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn qaima(self) -> Natija<Vec<MashruMukhzan>> {
        let mut jumla = self
            .ittisal
            .prepare(JUMLAT_MASHRU_QAIMA)
            .map_err(|q| khata_jumla("prepare", "mashru", q))?;

        let sufuf = jumla
            .query_map([], KhaamMashru::min_saf)
            .map_err(|q| khata_jumla("query", "mashru", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(
                saf.map_err(|q| khata_jumla("read", "mashru", q))?
                    .ila_mashru()?,
            );
        }
        Ok(natija)
    }
}

/// The project columns, in the order [`KhaamMashru::min_saf`] reads them.
macro_rules! jumlat_mashru {
    ($dhayl:literal) => {
        concat!(
            "SELECT id, luba, ism_luba, unwan, mujallad, lugha_masdar, lugha_hadaf,
                    halat, ruqaa, murajaa, basma_bina, tareeqa, rukhsa, musahim,
                    taghtiya_majmu, taghtiya_mutarjam, taghtiya_muakkad,
                    taghtiya_majmu_takrar, taghtiya_mutarjam_takrar,
                    taghtiya_majmu_awwal, taghtiya_mutarjam_awwal,
                    waqt_insha, akhir_tabdeel
             FROM mashru ",
            $dhayl
        )
    };
}

const JUMLAT_MASHRU_WAHID: &str = jumlat_mashru!("WHERE id = ?1");
const JUMLAT_MASHRU_QAIMA: &str = jumlat_mashru!("ORDER BY akhir_tabdeel DESC");

/// One `mashru` row before its domain types are rebuilt.
#[derive(Debug, Clone)]
struct KhaamMashru {
    id: String,
    luba: Option<String>,
    ism_luba: String,
    unwan: String,
    mujallad: String,
    lugha_masdar: String,
    lugha_hadaf: String,
    halat: String,
    ruqaa: Option<String>,
    murajaa: Option<i64>,
    basma_bina: Option<String>,
    tareeqa: Option<String>,
    rukhsa: Option<String>,
    musahim: Option<String>,
    taghtiya: Taghtiya,
    waqt_insha: String,
    akhir_tabdeel: String,
}

impl KhaamMashru {
    fn min_saf(saf: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: saf.get(0)?,
            luba: saf.get(1)?,
            ism_luba: saf.get(2)?,
            unwan: saf.get(3)?,
            mujallad: saf.get(4)?,
            lugha_masdar: saf.get(5)?,
            lugha_hadaf: saf.get(6)?,
            halat: saf.get(7)?,
            ruqaa: saf.get(8)?,
            murajaa: saf.get(9)?,
            basma_bina: saf.get(10)?,
            tareeqa: saf.get(11)?,
            rukhsa: saf.get(12)?,
            musahim: saf.get(13)?,
            taghtiya: taghtiya_min_saf(saf, 14)?,
            waqt_insha: saf.get(21)?,
            akhir_tabdeel: saf.get(22)?,
        })
    }

    fn ila_mashru(self) -> Natija<MashruMukhzan> {
        let musahim = self
            .musahim
            .map(|q| {
                MusahimId::jadeed(q.clone()).map_err(|_| {
                    Khata::from(KhataMakhzan::SafTalif {
                        jadwal: "mashru",
                        amud: "musahim",
                        qeema: q,
                    })
                })
            })
            .transpose()?;

        Ok(MashruMukhzan {
            id: self.id,
            luba: self
                .luba
                .map(|q| min_ramz("mashru", "luba", &q))
                .transpose()?,
            ism_luba: self.ism_luba,
            unwan: self.unwan,
            mujallad: PathBuf::from(self.mujallad),
            lugha_masdar: self.lugha_masdar,
            lugha_hadaf: self.lugha_hadaf,
            halat: min_ramz("mashru", "halat", &self.halat)?,
            ruqaa: self
                .ruqaa
                .map(|q| min_ramz("mashru", "ruqaa", &q))
                .transpose()?,
            murajaa: self
                .murajaa
                .map(|q| RuqaaRevision::jadeeda(u32::try_from(q).unwrap_or(1))),
            basma_bina: self
                .basma_bina
                .map(|q| basma_min_nass("mashru", "basma_bina", &q))
                .transpose()?,
            tareeqa: self
                .tareeqa
                .map(|q| min_ramz("mashru", "tareeqa", &q))
                .transpose()?,
            rukhsa: self.rukhsa.as_deref().map(ijma_rukhsa),
            musahim,
            taghtiya: self.taghtiya,
            waqt_insha: self.waqt_insha,
            akhir_tabdeel: self.akhir_tabdeel,
        })
    }
}

// ---------------------------------------------------------------------------
// الذاكرة — translation memory
// ---------------------------------------------------------------------------

/// One translation memory segment.
#[derive(Debug, Clone, PartialEq)]
pub struct MudkhalDhakira {
    /// The source text, as the translator saw it.
    pub masdar: String,
    /// The lookup key: the source, folded in Rust with full Unicode lowercasing.
    /// Folding here rather than through a `SQLite` collation is the same decision
    /// the library sort makes, for the same reason — `NOCASE` folds ASCII and
    /// leaves everything else to byte order, which for Arabic means no folding
    /// at all.
    pub masdar_muwahhad: String,
    /// The Arabic.
    pub hadaf: String,
    /// Source language tag.
    pub lugha_masdar: String,
    /// Target language tag.
    pub lugha_hadaf: String,
    /// The project it came from, if it is still there.
    pub mashru: Option<String>,
    /// The game it came from, if it is still in the library.
    pub luba: Option<LubaId>,
    /// The game's name, kept so a segment can name its origin after either is
    /// gone.
    pub ism_luba: Option<String>,
    /// Who produced it.
    pub musahim: Option<MusahimId>,
    /// How it was produced.
    pub tareeqa: Option<TareeqaTarjama>,
    /// Its review status, which is how candidates are preferred.
    pub halat: HalatMuraja,
    /// Machine-translation confidence, 0.0 to 1.0.
    pub thiqa: Option<f32>,
    /// How many times this pair has been reused.
    pub marrat: u32,
    /// When it entered the memory, RFC 3339.
    pub waqt: String,
    /// When it was last offered, RFC 3339.
    pub akhir_istikhdam: Option<String>,
}

/// The translation memory ledger: `dhakira`.
#[derive(Debug, Clone, Copy)]
pub struct SijillDhakira<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillDhakira<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Adds a segment, or counts another use of one already known.
    ///
    /// The unique index on `(target language, folded source, target)` is what
    /// makes the reuse counter mean anything: without it a memory fills with
    /// hundreds of copies of one segment, each used once, and ranking candidates
    /// by reuse ranks noise.
    ///
    /// # Errors
    ///
    /// Fails when a constrained value is out of range or the statement is
    /// rejected.
    pub fn sajjil(self, mudkhal: &MudkhalDhakira) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let mut jumla = self
            .ittisal
            .prepare_cached(
                "INSERT INTO dhakira (
                     masdar, masdar_muwahhad, hadaf, lugha_masdar, lugha_hadaf,
                     mashru, luba, ism_luba, musahim, tareeqa, halat, thiqa,
                     marrat, waqt, akhir_istikhdam)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?13)
                 ON CONFLICT (lugha_hadaf, masdar_muwahhad, hadaf) DO UPDATE SET
                     masdar          = excluded.masdar,
                     mashru          = coalesce(excluded.mashru, dhakira.mashru),
                     luba            = coalesce(excluded.luba, dhakira.luba),
                     ism_luba        = coalesce(excluded.ism_luba, dhakira.ism_luba),
                     musahim         = coalesce(excluded.musahim, dhakira.musahim),
                     tareeqa         = coalesce(excluded.tareeqa, dhakira.tareeqa),
                     halat           = excluded.halat,
                     thiqa           = coalesce(excluded.thiqa, dhakira.thiqa),
                     marrat          = dhakira.marrat + 1,
                     akhir_istikhdam = excluded.akhir_istikhdam",
            )
            .map_err(|q| khata_jumla("prepare upsert", "dhakira", q))?;

        let _ = jumla
            .execute(params![
                mudkhal.masdar,
                mudkhal.masdar_muwahhad,
                mudkhal.hadaf,
                mudkhal.lugha_masdar,
                mudkhal.lugha_hadaf,
                mudkhal.mashru,
                mudkhal.luba.map(|q| q.to_string()),
                mudkhal.ism_luba,
                mudkhal.musahim.as_ref().map(MusahimId::nass),
                mudkhal
                    .tareeqa
                    .map(|q| ila_ramz("dhakira", "tareeqa", &q))
                    .transpose()?,
                ila_ramz("dhakira", "halat", &mudkhal.halat)?,
                mudkhal.thiqa.map(f64::from),
                waqt,
            ])
            .map_err(|q| khata_jumla("upsert", "dhakira", q))?;
        Ok(())
    }

    /// Candidates for a source segment, best reviewed and most reused first.
    ///
    /// Exact matching only. Ranking near matches is Phase 13's work over the
    /// candidates this narrows to; the store's job is to make that set small
    /// with an index rather than to hand the pipeline the whole memory.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored value does not decode.
    pub fn bahth(
        self,
        masdar_muwahhad: &str,
        lugha_hadaf: &str,
        hadd: u32,
    ) -> Natija<Vec<MudkhalDhakira>> {
        let mut jumla = self
            .ittisal
            .prepare(
                "SELECT masdar, masdar_muwahhad, hadaf, lugha_masdar, lugha_hadaf,
                        mashru, luba, ism_luba, musahim, tareeqa, halat, thiqa,
                        marrat, waqt, akhir_istikhdam
                 FROM dhakira
                 WHERE masdar_muwahhad = ?1 AND lugha_hadaf = ?2
                 -- Ranked by review status, not by the text of the status
                 -- column: alphabetical order would put an unreviewed draft
                 -- above an approved translation, which is the opposite of
                 -- what a translator is asking for.
                 ORDER BY CASE halat
                            WHEN 'mujammada'   THEN 0
                            WHEN 'muakkada'    THEN 1
                            WHEN 'lil_muraja'  THEN 2
                            WHEN 'musawwada'   THEN 3
                            ELSE 4
                          END,
                          marrat DESC
                 LIMIT ?3",
            )
            .map_err(|q| khata_jumla("prepare", "dhakira", q))?;

        let sufuf = jumla
            .query_map(
                params![masdar_muwahhad, lugha_hadaf, i64::from(hadd.max(1))],
                KhaamDhakira::min_saf,
            )
            .map_err(|q| khata_jumla("query", "dhakira", q))?;

        let mut natija = Vec::new();
        for saf in sufuf {
            natija.push(
                saf.map_err(|q| khata_jumla("read", "dhakira", q))?
                    .ila_mudkhal()?,
            );
        }
        Ok(natija)
    }

    /// How many segments the memory holds.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn adad(self) -> Natija<u64> {
        let adad: i64 = self
            .ittisal
            .query_row("SELECT count(*) FROM dhakira", [], |saf| saf.get(0))
            .map_err(|q| khata_jumla("count", "dhakira", q))?;
        Ok(u64::try_from(adad).unwrap_or(0))
    }
}

/// One `dhakira` row before its domain types are rebuilt.
#[derive(Debug, Clone)]
struct KhaamDhakira {
    masdar: String,
    masdar_muwahhad: String,
    hadaf: String,
    lugha_masdar: String,
    lugha_hadaf: String,
    mashru: Option<String>,
    luba: Option<String>,
    ism_luba: Option<String>,
    musahim: Option<String>,
    tareeqa: Option<String>,
    halat: String,
    thiqa: Option<f64>,
    marrat: i64,
    waqt: String,
    akhir_istikhdam: Option<String>,
}

impl KhaamDhakira {
    fn min_saf(saf: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            masdar: saf.get(0)?,
            masdar_muwahhad: saf.get(1)?,
            hadaf: saf.get(2)?,
            lugha_masdar: saf.get(3)?,
            lugha_hadaf: saf.get(4)?,
            mashru: saf.get(5)?,
            luba: saf.get(6)?,
            ism_luba: saf.get(7)?,
            musahim: saf.get(8)?,
            tareeqa: saf.get(9)?,
            halat: saf.get(10)?,
            thiqa: saf.get(11)?,
            marrat: saf.get(12)?,
            waqt: saf.get(13)?,
            akhir_istikhdam: saf.get(14)?,
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "confidence is 0.0 to 1.0 by CHECK constraint and is a display value"
    )]
    fn ila_mudkhal(self) -> Natija<MudkhalDhakira> {
        let musahim = self
            .musahim
            .map(|q| {
                MusahimId::jadeed(q.clone()).map_err(|_| {
                    Khata::from(KhataMakhzan::SafTalif {
                        jadwal: "dhakira",
                        amud: "musahim",
                        qeema: q,
                    })
                })
            })
            .transpose()?;

        Ok(MudkhalDhakira {
            masdar: self.masdar,
            masdar_muwahhad: self.masdar_muwahhad,
            hadaf: self.hadaf,
            lugha_masdar: self.lugha_masdar,
            lugha_hadaf: self.lugha_hadaf,
            mashru: self.mashru,
            luba: self
                .luba
                .map(|q| min_ramz("dhakira", "luba", &q))
                .transpose()?,
            ism_luba: self.ism_luba,
            musahim,
            tareeqa: self
                .tareeqa
                .map(|q| min_ramz("dhakira", "tareeqa", &q))
                .transpose()?,
            halat: min_ramz("dhakira", "halat", &self.halat)?,
            thiqa: self.thiqa.map(|q| q as f32),
            marrat: u32::try_from(self.marrat).unwrap_or(0),
            waqt: self.waqt,
            akhir_istikhdam: self.akhir_istikhdam,
        })
    }
}

// ---------------------------------------------------------------------------
// الحالة — state the store owns
// ---------------------------------------------------------------------------

/// Keys the store itself uses, so a caller never spells one twice.
pub mod miftah {
    /// The generation of the most recent completed scan.
    pub const AKHIR_FAHS: &str = "akhir_fahs";
    /// When the registry manifest was last checked, RFC 3339.
    pub const AKHIR_MUSTAWDA: &str = "akhir_mustawda";
    /// The hash of the registry manifest as last seen.
    pub const BASMAT_MUSTAWDA: &str = "basmat_mustawda";
    /// When the artwork cache was last swept, RFC 3339.
    pub const AKHIR_KANS_SUWAR: &str = "akhir_kans_suwar";
    /// The probe version that last examined the whole library.
    pub const ISDAR_FAHS_MUHARRIK: &str = "isdar_fahs_muharrik";

    /// The prefix of the per-game key holding the engine probe's cheap
    /// install-root signature. [`basmat_jidhr`] builds the whole key.
    pub const BASMAT_JIDHR_MUHARRIK: &str = "basmat_jidhr_muharrik:";

    /// The key one game's install-root signature is stored under.
    ///
    /// A key assembled from a value, which is not the same thing as a statement
    /// assembled from a value: the SQL underneath is the same constant for every
    /// game and the key reaches it as a bound parameter, exactly like a game
    /// title does.
    #[must_use]
    pub fn basmat_jidhr(luba: super::LubaId) -> String {
        format!("{BASMAT_JIDHR_MUHARRIK}{luba}")
    }
}

/// The state ledger: `halat`.
///
/// State, not configuration. Settings live in `idadat.json` behind
/// `usus::idadat`, where a user can edit them by hand, the environment can layer
/// over them, and they can be read before the database is even open. What lives
/// here is what the store produced and the store consumes.
#[derive(Debug, Clone, Copy)]
pub struct SijillHalat<'a> {
    ittisal: &'a Connection,
}

impl<'a> SijillHalat<'a> {
    /// Binds the ledger to a connection.
    #[must_use]
    pub const fn jadeed(ittisal: &'a Connection) -> Self {
        Self { ittisal }
    }

    /// Reads a value.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn iqra(self, miftah: &str) -> Natija<Option<String>> {
        self.ittisal
            .query_row(
                "SELECT qeema FROM halat WHERE miftah = ?1",
                params![miftah],
                |saf| saf.get(0),
            )
            .optional()
            .map_err(|q| khata_jumla("lookup", "halat", q))
    }

    /// Writes a value, stamping when.
    ///
    /// # Errors
    ///
    /// Fails when the statement is rejected.
    pub fn iktub(self, miftah: &str, qeema: &str) -> Natija<()> {
        let waqt = alaan(self.ittisal)?;
        let _ = self
            .ittisal
            .execute(
                "INSERT INTO halat (miftah, qeema, waqt) VALUES (?1, ?2, ?3)
                 ON CONFLICT (miftah) DO UPDATE SET qeema = excluded.qeema, waqt = excluded.waqt",
                params![miftah, qeema, waqt],
            )
            .map_err(|q| khata_jumla("upsert", "halat", q))?;
        Ok(())
    }

    /// Reads a value as an unsigned number, treating an absent or unreadable
    /// value as zero.
    ///
    /// Absent is genuinely zero here — no scan has run, no sweep has happened —
    /// so this is a default rather than a guess about damaged data.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run.
    pub fn iqra_raqm(self, miftah: &str) -> Natija<u64> {
        Ok(self
            .iqra(miftah)?
            .and_then(|q| q.parse::<u64>().ok())
            .unwrap_or(0))
    }

    /// Writes an unsigned number.
    ///
    /// # Errors
    ///
    /// As [`Self::iktub`].
    pub fn iktub_raqm(self, miftah: &str, qeema: u64) -> Natija<()> {
        self.iktub(miftah, &qeema.to_string())
    }

    /// Forgets a value.
    ///
    /// # Errors
    ///
    /// Fails when the delete cannot run.
    pub fn ihdhif(self, miftah: &str) -> Natija<()> {
        let _ = self
            .ittisal
            .execute("DELETE FROM halat WHERE miftah = ?1", params![miftah])
            .map_err(|q| khata_jumla("delete", "halat", q))?;
        Ok(())
    }
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use taarib_mustalahat::muharrik::{
        AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik,
    };
    use taarib_usus::manassa::Mimariya;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// A store failure as a test failure, carrying the sentence a person would
    /// read rather than a debug dump of the whole value.
    fn bila_khata<T>(natija: Natija<T>) -> Result<T, Box<dyn Error>> {
        natija.map_err(|khata| khata.injilizi.into())
    }

    /// The whole schema, in memory, through the same migration runner the
    /// product uses — so a sweep tested here runs against the real tables and
    /// the real constraints, not a hand-copied subset of them.
    fn qaida() -> Result<Connection, Box<dyn Error>> {
        let mut ittisal = Connection::open_in_memory()?;
        ittisal.execute_batch("PRAGMA foreign_keys = ON;")?;
        let _ = bila_khata(crate::hijra::rahhil(&mut ittisal))?;
        Ok(ittisal)
    }

    /// A game known by one launcher identity and nothing else.
    fn luba(masdar: MasdarLuba, ism: &str) -> Luba {
        Luba {
            id: LubaId::min_masdar(&masdar, ism),
            masadir: vec![masdar],
            ism: ism.to_owned(),
            jidhr: PathBuf::from("/alaab").join(ism),
            tanfidhi: None,
            hajm: 0,
            akhir_laab: None,
            akhir_tahdith: None,
            bina: None,
            suwar: SuwarLuba::default(),
            beea: BeeatTawafuq::Asli,
            mawjuda: true,
            mukhfiya: false,
        }
    }

    /// Stamps a game with a scan generation, as the scan does for what it saw.
    fn sajjil(ittisal: &Connection, luba: &Luba, fahs: u64) -> NatijatIkhtibar {
        bila_khata(SijillAlaab::jadeed(ittisal).sajjil(&IdkhalLuba {
            luba,
            muktamila: true,
            khiyarat_tashghil: None,
            simat: &[],
            fahs,
        }))
    }

    /// Whether the store still says a game is on disk.
    fn mawjuda(ittisal: &Connection, id: LubaId) -> Result<bool, Box<dyn Error>> {
        let luba = bila_khata(SijillAlaab::jadeed(ittisal).wahida(id))?;
        Ok(luba
            .ok_or("the game's row is gone, and absent is not deleted")?
            .mawjuda)
    }

    /// An installed launcher's record for a scan, with or without a complete
    /// read.
    fn matjar(aila: &str, najah: bool) -> MatjarMukhzan {
        MatjarMukhzan {
            aila: aila.to_owned(),
            mawjud: true,
            najah,
            adad_alaab: 0,
            adad_tanbihat: u32::from(!najah),
            muddat_milli: 1,
        }
    }

    /// A launcher whose folder is there and whose catalogue was not read — the
    /// Epic-without-`Manifests` machine — must leave every stored game of that
    /// family exactly as it was, and its reason must be readable afterwards.
    #[test]
    fn al_mash_yurfad_li_matjar_lam_yuqra_fahrasuhu() -> NatijatIkhtibar {
        let ittisal = qaida()?;
        let awwal = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let luba = luba(MasdarLuba::Epic("abc".to_owned()), "Alan Wake 2");
        sajjil(&ittisal, &luba, awwal)?;

        let thani = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let tanbih = TanbihMukhzan {
            aila: "epic".to_owned(),
            mawdi: "C:\\ProgramData\\Epic".to_owned(),
            sabab: "no manifest directory under this Epic root".to_owned(),
        };
        bila_khata(SijillFahs::jadeed(&ittisal).sajjil_natijat_matjar(
            thani,
            &matjar("epic", false),
            std::slice::from_ref(&tanbih),
        ))?;

        let hasila = bila_khata(SijillAlaab::jadeed(&ittisal).allim_ghayr_mawjud(thani, "epic"))?;
        assert_eq!(hasila, HasilatMash::Rufidat);
        assert!(
            mawjuda(&ittisal, luba.id)?,
            "a game nobody looked for is not absent"
        );

        // The warning that explains the refusal is on record beside the scan.
        let mukhzana = bila_khata(SijillFahs::jadeed(&ittisal).tanbihat(thani))?;
        assert_eq!(mukhzana, vec![tanbih]);
        let matajir = bila_khata(SijillFahs::jadeed(&ittisal).matajir(thani))?;
        assert_eq!(matajir, vec![matjar("epic", false)]);
        Ok(())
    }

    /// A launcher the scan never recorded at all is the same refusal: the old
    /// caller built its sweep list from folder existence and never wrote this
    /// table, so an unrecorded launcher is exactly the case that emptied
    /// libraries.
    #[test]
    fn al_mash_yurfad_li_matjar_lam_yusajjal() -> NatijatIkhtibar {
        let ittisal = qaida()?;
        let awwal = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let luba = luba(MasdarLuba::Steam(220), "Half-Life 2");
        sajjil(&ittisal, &luba, awwal)?;

        let thani = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let hasila = bila_khata(SijillAlaab::jadeed(&ittisal).allim_ghayr_mawjud(thani, "steam"))?;
        assert_eq!(hasila, HasilatMash::Rufidat);
        assert!(mawjuda(&ittisal, luba.id)?);
        Ok(())
    }

    /// The sweep still does its job for a launcher that was read whole: a game
    /// it no longer lists is marked absent, and a game it listed again is not.
    #[test]
    fn al_mash_yajri_li_matjar_quria_kamilan() -> NatijatIkhtibar {
        let ittisal = qaida()?;
        let awwal = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let dhahaba = luba(MasdarLuba::Steam(220), "Half-Life 2");
        let baqiya = luba(MasdarLuba::Steam(400), "Portal");
        sajjil(&ittisal, &dhahaba, awwal)?;
        sajjil(&ittisal, &baqiya, awwal)?;

        let thani = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        sajjil(&ittisal, &baqiya, thani)?;
        bila_khata(SijillFahs::jadeed(&ittisal).sajjil_natijat_matjar(
            thani,
            &matjar("steam", true),
            &[],
        ))?;

        let hasila = bila_khata(SijillAlaab::jadeed(&ittisal).allim_ghayr_mawjud(thani, "steam"))?;
        assert_eq!(hasila, HasilatMash::Jarat { adad: 1 });
        assert!(!mawjuda(&ittisal, dhahaba.id)?);
        assert!(mawjuda(&ittisal, baqiya.id)?);
        Ok(())
    }

    /// A game installed through Heroic shards under Epic, but Epic's own
    /// catalogue never listed it — so Epic being read whole says nothing about
    /// it. It is swept only once Heroic was read whole too.
    #[test]
    fn al_mash_la_yamass_luba_mudirha_lam_yuqra() -> NatijatIkhtibar {
        let ittisal = qaida()?;
        let awwal = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let luba = luba(
            MasdarLuba::Heroic(Box::new(MasdarLuba::Epic("h1".to_owned()))),
            "Hades",
        );
        sajjil(&ittisal, &luba, awwal)?;

        let thani = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        bila_khata(SijillFahs::jadeed(&ittisal).sajjil_natijat_matjar(
            thani,
            &matjar("epic", true),
            &[],
        ))?;
        let hasila = bila_khata(SijillAlaab::jadeed(&ittisal).allim_ghayr_mawjud(thani, "epic"))?;
        assert_eq!(hasila, HasilatMash::Jarat { adad: 0 });
        assert!(
            mawjuda(&ittisal, luba.id)?,
            "Heroic was not read, so its game stays"
        );

        bila_khata(SijillFahs::jadeed(&ittisal).sajjil_natijat_matjar(
            thani,
            &matjar("heroic", true),
            &[],
        ))?;
        let hasila = bila_khata(SijillAlaab::jadeed(&ittisal).allim_ghayr_mawjud(thani, "epic"))?;
        assert_eq!(hasila, HasilatMash::Jarat { adad: 1 });
        assert!(!mawjuda(&ittisal, luba.id)?);
        Ok(())
    }

    /// A report that satisfies every constraint the ledger checks, stamped
    /// with the probe version the caller wants.
    fn taqreer(isdar_fahs: u32) -> TaqreerImkaniyat {
        TaqreerImkaniyat {
            muharrik: Muharrik {
                aila: AilatMuharrik::Majhul,
                isdar: None,
                khalfiya: KhalfiyaBarmajiya::Majhula,
                itarat: Vec::new(),
                rusum: Vec::new(),
                mimariya: Mimariya::X8664,
                thiqa: 20,
                dalail: Vec::new(),
            },
            tabaqa: Tabaqa::TarjamaFawqiya,
            sabab_arabi: String::new(),
            sabab_injilizi: String::new(),
            jahiziya: JahiziyatTashghil::Ghaiba,
            naqs: None,
            anzimat_qabila: Vec::new(),
            jawda: JawdaMutawaqqaa::Mahduda,
            hudud: Vec::new(),
            marfuda: false,
            isdar_fahs,
            waqt: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    /// The version map answers for the whole library at once: a game with no
    /// report is absent from it, and every stored report is keyed under its
    /// game with the probe version that wrote it — which is what lets one read
    /// decide both "never probed" and "probed by an older build".
    #[test]
    fn isdarat_altaqarir_tuqra_marra_wahida() -> NatijatIkhtibar {
        let ittisal = qaida()?;
        let fahs = bila_khata(SijillFahs::jadeed(&ittisal).ibda(true))?;
        let qadima = luba(MasdarLuba::Steam(1), "Qadima");
        let haditha = luba(MasdarLuba::Steam(2), "Haditha");
        let bila_taqreer = luba(MasdarLuba::Steam(3), "Bila Taqreer");
        for wahida in [&qadima, &haditha, &bila_taqreer] {
            sajjil(&ittisal, wahida, fahs)?;
        }
        bila_khata(SijillMuharrik::jadeed(&ittisal).sajjil(qadima.id, &taqreer(3), None))?;
        bila_khata(SijillMuharrik::jadeed(&ittisal).sajjil(haditha.id, &taqreer(9), None))?;

        let isdarat = bila_khata(SijillMuharrik::jadeed(&ittisal).isdarat())?;
        assert_eq!(isdarat.len(), 2);
        assert_eq!(isdarat.get(&qadima.id), Some(&3));
        assert_eq!(isdarat.get(&haditha.id), Some(&9));
        assert!(!isdarat.contains_key(&bila_taqreer.id));

        let qadima_faqat = bila_khata(SijillMuharrik::jadeed(&ittisal).qadima(9))?;
        assert_eq!(
            qadima_faqat,
            vec![qadima.id],
            "the two reads agree on which report an older probe wrote"
        );
        Ok(())
    }
}
