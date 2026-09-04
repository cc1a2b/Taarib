//! باتل نت — Battle.net, read out of the Agent's own `product.db`.
//!
//! Battle.net keeps one authoritative record of what is installed: a protobuf
//! file written by the Agent process at
//! `%PROGRAMDATA%\Battle.net\Agent\product.db` on Windows and
//! `/Users/Shared/Battle.net/Agent/product.db` on macOS. There is no registry
//! key that lists games, no per-game manifest directory, and no text catalogue.
//! The protobuf is the catalogue.
//!
//! Each installed product's own directory then carries a `.build.info` file,
//! which is a small `|`-separated table naming the branch, the active flag and
//! the installed version. That file is the cross-check: the protobuf says where
//! a product lives, `.build.info` says which branch and version actually sit
//! there, and disagreement between the two is a warning rather than a guess.
//!
//! ## The protobuf, and how honest this reader is about it
//!
//! **Blizzard publishes no `.proto` for `product.db`.** This crate has `prost`
//! for the schemas Taarib itself defines, but no schema and no build-time
//! codegen exist for this file, so it is read here by a hand-written
//! wire-format reader over the four things the protobuf wire format guarantees
//! regardless of schema:
//!
//! | wire type | value | how it is read |
//! | --- | --- | --- |
//! | 0 | varint | base-128, little-endian groups, high bit = continue, capped at ten bytes |
//! | 1 | 64-bit | eight bytes, skipped |
//! | 2 | length-delimited | a varint length, then exactly that many bytes |
//! | 5 | 32-bit | four bytes, skipped |
//!
//! Wire types 3 and 4 (the deprecated start-group/end-group pair) do not occur
//! in this file and are treated as a framing error naming the offset.
//!
//! Field *numbers*, unlike wire types, are schema knowledge, and the ones below
//! are **inferred from the shape of the data rather than taken from a published
//! schema**. They match what the file has looked like across Agent versions,
//! and they are ranked here by how much weight this adapter puts on them:
//!
//! | path | inferred meaning | confidence |
//! | --- | --- | --- |
//! | top-level field 1, wire 2, repeated | one installed-product record | high — it is the only repeated message at the top level and every element parses as a product |
//! | product field 1, wire 2 | the client UID (`wow`, `s2`, `pro`) | high |
//! | product field 2, wire 2 | the product code | high — it is either equal to the UID or its base (`wow_classic` → `wow`) |
//! | product field 3, wire 2 | the settings submessage | high — it is the only nested message that contains a filesystem path |
//! | settings field 1, wire 2 | the install path | medium-high — always the first string in the submessage and always a path that exists |
//! | settings field 2, wire 2 | the play region / selected branch (`us`, `eu`) | low — short, region-shaped, and contradicted by some Agent versions; `.build.info` overrides it whenever `.build.info` parses |
//!
//! Because of that ranking the reader never *requires* a field number to be
//! right. Every string it walks past is kept, and if field 3/field 1 did not
//! yield a directory that exists, the record falls back to the first retained
//! string that resolves to a real directory. A record that yields nothing at
//! all becomes a [`TanbihFahs`] naming the product, and the rest of the file is
//! still read. **An unrecognised structure degrades to a warning; it never
//! fails the scan and never invents a game.**
//!
//! Every read is bounds-checked through `slice::get`. A truncated file — a
//! length prefix that runs past the end, a varint with no terminator — produces
//! a warning naming the byte offset it stopped at, never an out-of-bounds read
//! and never a panic.

use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs, TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "battlenet";

/// The platforms Battle.net exists on. There is no Linux client, and a Linux
/// user's Blizzard games arrive through Lutris or a bare prefix, not through
/// this adapter.
const MANASSAT: [NizamTashghil; 2] = [NizamTashghil::Windows, NizamTashghil::Mac];

/// The client's directory name under Windows' machine-wide data root.
const JIDHR_WINDOWS: &str = "Battle.net";

/// The client's shared directory on macOS.
const JIDHR_MAC: &str = "/Users/Shared/Battle.net";

/// A varint is at most ten base-128 groups: 64 bits divided into seven-bit
/// groups is nine full groups plus one carrying the final bit.
const AQSA_TUL_VARINT: usize = 10;

/// A length-delimited field longer than this is not a field, it is a corrupt
/// length prefix. `product.db` is tens of kilobytes; a single field claiming
/// sixteen megabytes means the framing is already lost.
const AQSA_TUL_HAQL: u64 = 16 * 1024 * 1024;

/// Battle.net product codes whose display name Taarib knows.
///
/// `product.db` records a product **code**, never a title: the file says `pro`,
/// not `Overwatch`. Without this table every Battle.net entry in the library
/// would be a three-letter code, which is not a library anybody recognises.
/// A code that is not listed keeps its code as its name — wrong-looking, but
/// honest, and it still installs.
const ASMAA_MUNTAJAT: &[(&str, &str)] = &[
    ("anbs", "Diablo Immortal"),
    ("d3", "Diablo III"),
    ("d3cn", "Diablo III"),
    ("dst2", "Destiny 2"),
    ("fenris", "Diablo IV"),
    ("fore", "Call of Duty: Vanguard"),
    ("gryphon", "Warcraft Rumble"),
    ("hero", "Heroes of the Storm"),
    ("heroes", "Heroes of the Storm"),
    ("hs", "Hearthstone"),
    ("hsb", "Hearthstone"),
    ("lazr", "Call of Duty: Modern Warfare II"),
    ("odin", "Call of Duty: Modern Warfare"),
    ("osi", "Diablo II: Resurrected"),
    ("pro", "Overwatch"),
    ("rtro", "Blizzard Arcade Collection"),
    ("s1", "StarCraft Remastered"),
    ("s2", "StarCraft II"),
    ("viper", "Call of Duty: Black Ops 4"),
    ("w3", "Warcraft III: Reforged"),
    ("wlby", "Call of Duty: Modern Warfare III"),
    ("wow", "World of Warcraft"),
    ("wow_classic", "World of Warcraft Classic"),
    ("wow_classic_era", "World of Warcraft Classic Era"),
    ("zeus", "Call of Duty: Black Ops Cold War"),
];

/// Product codes that are the client itself rather than a game.
///
/// They appear in `product.db` exactly like games do, with an install path and
/// a branch. Marking them keeps the launcher's own updater out of the library
/// without hiding them from Diagnostics.
const RUMUZ_GHAYR_LUBA: &[&str] = &["agent", "bna", "bts", "battle.net", "bnfw"];

/// Battle.net.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarBattleNet;

impl MatjarBattleNet {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// The Agent directory that holds `product.db`, per platform.
    fn judhur_muhtamala(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        match siyaq.nizam {
            NizamTashghil::Windows => vec![bayanat_barnamij().join(JIDHR_WINDOWS)],
            NizamTashghil::Mac => vec![PathBuf::from(JIDHR_MAC)],
            NizamTashghil::Linux => Vec::new(),
        }
    }
}

impl Matjar for MatjarBattleNet {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "باتل نت"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Battle.net"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        // A configured root is returned whether or not it exists. The user
        // asserted it; `ifhas` is where that assertion is checked and where a
        // wrong path becomes a message they can act on, rather than silently
        // becoming "Battle.net is not installed".
        if let Some(tajawuz) = siyaq.manassat.battlenet.as_ref() {
            return Some(tajawuz.clone());
        }
        Self::judhur_muhtamala(siyaq).into_iter().find(|jidhr| fahras_fih(jidhr).is_file())
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when the user configured a
    /// client root that is not there, [`KhataKashf::TaadhurQiraatFahras`] when
    /// `product.db` exists but cannot be opened, and
    /// [`KhataKashf::TarwisatFahrasTalifa`] when its protobuf framing breaks —
    /// the last one naming the byte offset it broke at. Everything narrower —
    /// one product whose directory is gone, one missing `.build.info`, one
    /// record whose layout is unrecognised — is a [`TanbihFahs`] and costs only
    /// that entry.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        // A user-set root that is not there is their mistake, and saying so is
        // more useful than silently scanning somewhere else.
        if let Some(muhaddad) = siyaq.manassat.battlenet.as_ref()
            && !muhaddad.is_dir()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: muhaddad.clone(),
            }
            .into());
        }

        let Some(jidhr) = self.mawqi(siyaq) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };
        if !fahras_fih(&jidhr).is_file() {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        // Read through `std::fs` rather than the shared helper because the
        // error model wants the raw I/O failure as a value: whether the file
        // was missing, locked by a running Agent, or on a disconnected drive is
        // exactly what the user needs told, and a stringified message loses it.
        let fahras = fahras_fih(&jidhr);
        let bayt = std::fs::read(&fahras).map_err(|sabab| KhataKashf::TaadhurQiraatFahras {
            matjar: MUARRIF,
            masar: fahras.clone(),
            sabab,
        })?;

        let mut tanbihat = Vec::new();
        let sijillat = match sijillat_muntajat(&bayt) {
            Ok(sijillat) => sijillat,
            Err(khata) => {
                return Err(KhataKashf::TarwisatFahrasTalifa {
                    matjar: MUARRIF,
                    masar: fahras,
                    tafsil: khata.tafsil,
                    mawdi: Some(khata.mawdi),
                }
                .into());
            },
        };

        // The file framed correctly but yielded nothing under the field number
        // this reader infers for an installed-product record. That is the
        // signature of an Agent version whose layout has moved, and it is
        // reported as such rather than presented as "no Blizzard games".
        if sijillat.is_empty() {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                fahras.display().to_string(),
                "product.db parsed as protobuf but held no installed-product records where this \
                 reader expects them. Either no Blizzard game is installed, or the Agent's \
                 record layout has changed and Taarib needs updating.",
            ));
        }

        let mut alaab = Vec::new();
        for sijill in sijillat {
            match luba_min_sijill(&sijill, &mut tanbihat) {
                Some(luba) => alaab.push(luba),
                None => tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    sijill.wasf(),
                    "this product record carried no product code, or no install directory that \
                     still exists on disk; the product may have been removed without Battle.net \
                     updating product.db, or the Agent's record layout has changed",
                )),
            }
        }

        Ok(NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: Some(jidhr),
            alaab,
            tanbihat,
            muddat: bidaya.elapsed(),
        })
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        self.mawqi(siyaq)
            .map(|jidhr| jidhr.join("Agent"))
            .filter(|mujallad| mujallad.is_dir())
            .map(|mujallad| vec![mujallad])
            .unwrap_or_default()
    }
}

/// The catalogue file under a client root.
fn fahras_fih(jidhr: &Path) -> PathBuf {
    jidhr.join("Agent").join("product.db")
}

/// Windows' machine-wide application data directory.
///
/// Battle.net installs its Agent under it rather than under the user's profile,
/// because one machine has one Agent regardless of how many people log in. The
/// documented default stands in when the variable is absent, which happens only
/// in stripped service environments.
fn bayanat_barnamij() -> PathBuf {
    std::env::var_os("PROGRAMDATA")
        .filter(|qeema| !qeema.is_empty())
        .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from)
}

// ---------------------------------------------------------------------------
// product.db — the wire-format reader
// ---------------------------------------------------------------------------

/// A framing failure inside `product.db`, with the offset it happened at.
///
/// The offset is the whole point: "product.db is corrupt" is unactionable,
/// "product.db stops making sense at byte 4172" tells the owner whether the
/// file was truncated by a crash or written by an Agent version whose layout
/// this reader does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KhataBrutu {
    /// The byte offset the reader was at.
    mawdi: u64,
    /// What went wrong there.
    tafsil: String,
}

impl KhataBrutu {
    fn jadeed(mawdi: usize, tafsil: impl Into<String>) -> Self {
        Self { mawdi: u64::try_from(mawdi).unwrap_or(u64::MAX), tafsil: tafsil.into() }
    }
}

/// The four wire types this file uses, plus the two group markers it must
/// refuse rather than mis-skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NawHaql {
    /// Wire type 0.
    Varint,
    /// Wire type 1 — eight bytes.
    Thabit64,
    /// Wire type 2 — a varint length then that many bytes.
    Muhaddad,
    /// Wire type 5 — four bytes.
    Thabit32,
}

impl NawHaql {
    fn min_raqm(raqm: u64, mawdi: usize) -> Result<Self, KhataBrutu> {
        match raqm {
            0 => Ok(Self::Varint),
            1 => Ok(Self::Thabit64),
            2 => Ok(Self::Muhaddad),
            5 => Ok(Self::Thabit32),
            3 | 4 => Err(KhataBrutu::jadeed(
                mawdi,
                "protobuf group markers (wire types 3 and 4) are not used by product.db",
            )),
            akhar => Err(KhataBrutu::jadeed(mawdi, format!("unknown protobuf wire type {akhar}"))),
        }
    }
}

/// A bounds-checked cursor over protobuf wire-format bytes.
#[derive(Debug)]
struct QariBrutu<'a> {
    bayt: &'a [u8],
    mawdi: usize,
}

impl<'a> QariBrutu<'a> {
    const fn jadeed(bayt: &'a [u8]) -> Self {
        Self { bayt, mawdi: 0 }
    }

    const fn intaha(&self) -> bool {
        self.mawdi >= self.bayt.len()
    }

    /// Reads one base-128 varint.
    ///
    /// Refuses a run longer than ten groups and refuses a shift past 64 bits,
    /// so a field of `0xFF` bytes cannot spin the loop or wrap the accumulator.
    fn varint(&mut self) -> Result<u64, KhataBrutu> {
        let bidaya = self.mawdi;
        let mut qeema: u64 = 0u64;
        let mut izaha: u32 = 0u32;
        for _ in 0..AQSA_TUL_VARINT {
            let Some(bayt) = self.bayt.get(self.mawdi).copied() else {
                return Err(KhataBrutu::jadeed(bidaya, "file ends in the middle of a varint"));
            };
            self.mawdi = self.mawdi.saturating_add(1);
            let juz = u64::from(bayt & 0x7F);
            let Some(muzah) = juz.checked_shl(izaha) else {
                return Err(KhataBrutu::jadeed(bidaya, "varint is wider than 64 bits"));
            };
            qeema |= muzah;
            if bayt & 0x80 == 0 {
                return Ok(qeema);
            }
            izaha = izaha.saturating_add(7);
        }
        Err(KhataBrutu::jadeed(bidaya, "varint runs past ten bytes without terminating"))
    }

    /// Reads a field tag: the field number and its wire type.
    fn tarwisa(&mut self) -> Result<(u32, NawHaql), KhataBrutu> {
        let bidaya = self.mawdi;
        let tag = self.varint()?;
        let naw = NawHaql::min_raqm(tag & 0x07, bidaya)?;
        let raqm = u32::try_from(tag >> 3)
            .map_err(|_| KhataBrutu::jadeed(bidaya, "protobuf field number exceeds 32 bits"))?;
        if raqm == 0 {
            return Err(KhataBrutu::jadeed(bidaya, "protobuf field number 0 is not valid"));
        }
        Ok((raqm, naw))
    }

    /// Reads a length-delimited field's payload.
    fn muhaddad(&mut self) -> Result<&'a [u8], KhataBrutu> {
        let bidaya = self.mawdi;
        let tul = self.varint()?;
        if tul > AQSA_TUL_HAQL {
            return Err(KhataBrutu::jadeed(
                bidaya,
                format!("length-delimited field claims {tul} bytes, which product.db never has"),
            ));
        }
        let tul_usize = usize::try_from(tul)
            .map_err(|_| KhataBrutu::jadeed(bidaya, "length prefix does not fit this machine"))?;
        let nihaya = self.mawdi.checked_add(tul_usize).ok_or_else(|| {
            KhataBrutu::jadeed(bidaya, "length prefix overflows the end of the file")
        })?;
        let juz = self.bayt.get(self.mawdi..nihaya).ok_or_else(|| {
            KhataBrutu::jadeed(
                bidaya,
                format!("length-delimited field of {tul} bytes runs past the end of the file"),
            )
        })?;
        self.mawdi = nihaya;
        Ok(juz)
    }

    /// Steps over a field whose contents this reader does not need.
    fn takhatti(&mut self, naw: NawHaql) -> Result<(), KhataBrutu> {
        match naw {
            NawHaql::Varint => self.varint().map(|_| ()),
            NawHaql::Thabit64 => self.tajawuz(8),
            NawHaql::Thabit32 => self.tajawuz(4),
            NawHaql::Muhaddad => self.muhaddad().map(|_| ()),
        }
    }

    /// Advances a fixed number of bytes, refusing to step past the end.
    fn tajawuz(&mut self, adad: usize) -> Result<(), KhataBrutu> {
        let nihaya = self
            .mawdi
            .checked_add(adad)
            .ok_or_else(|| KhataBrutu::jadeed(self.mawdi, "fixed-width field overflows"))?;
        if nihaya > self.bayt.len() {
            return Err(KhataBrutu::jadeed(
                self.mawdi,
                format!("fixed-width field of {adad} bytes runs past the end of the file"),
            ));
        }
        self.mawdi = nihaya;
        Ok(())
    }
}

/// One installed-product record, as far as the wire format allows it to be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SijillMuntaj {
    /// Product field 1 — the client UID.
    uid: Option<String>,
    /// Product field 2 — the product code.
    ramz: Option<String>,
    /// Settings field 1 — the install directory.
    masar: Option<PathBuf>,
    /// Settings field 2 — the play region or selected branch, low confidence.
    far: Option<String>,
    /// Every other printable string in the record, kept so that a layout this
    /// reader does not recognise still has somewhere to recover the install
    /// directory from.
    nusus: Vec<String>,
}

impl SijillMuntaj {
    /// The best name this record can be described by in a warning.
    fn wasf(&self) -> String {
        self.ramz
            .clone()
            .or_else(|| self.uid.clone())
            .unwrap_or_else(|| "unnamed product.db record".to_owned())
    }

    /// The product code, falling back to the UID.
    fn ramz_faal(&self) -> Option<&str> {
        self.ramz.as_deref().or(self.uid.as_deref())
    }

    /// The install directory, preferring the inferred field and falling back to
    /// any retained string that names a directory that exists.
    fn jidhr(&self) -> Option<PathBuf> {
        if let Some(masar) = self.masar.as_ref()
            && masar.is_dir()
        {
            return Some(masar.clone());
        }
        self.nusus
            .iter()
            .map(PathBuf::from)
            .find(|murashah| murashah.is_absolute() && murashah.is_dir())
    }
}

/// Walks the top-level message and returns every installed-product record.
fn sijillat_muntajat(bayt: &[u8]) -> Result<Vec<SijillMuntaj>, KhataBrutu> {
    let mut qari = QariBrutu::jadeed(bayt);
    let mut sijillat = Vec::new();
    while !qari.intaha() {
        let (raqm, naw) = qari.tarwisa()?;
        if raqm == 1 && naw == NawHaql::Muhaddad {
            let juz = qari.muhaddad()?;
            sijillat.push(iqra_sijill(juz)?);
        } else {
            qari.takhatti(naw)?;
        }
    }
    Ok(sijillat)
}

/// Reads one product record.
fn iqra_sijill(bayt: &[u8]) -> Result<SijillMuntaj, KhataBrutu> {
    let mut qari = QariBrutu::jadeed(bayt);
    let mut sijill = SijillMuntaj::default();
    while !qari.intaha() {
        let (raqm, naw) = qari.tarwisa()?;
        if naw != NawHaql::Muhaddad {
            qari.takhatti(naw)?;
            continue;
        }
        let juz = qari.muhaddad()?;
        match raqm {
            1 => sijill.uid = nass_min(juz),
            2 => sijill.ramz = nass_min(juz),
            3 => iqra_idadat(juz, &mut sijill)?,
            _ => {
                // Not a field this reader claims to know. If it is printable it
                // is retained, because the fallback path in `jidhr` is what
                // makes an unknown layout degrade instead of vanish.
                if let Some(nass) = nass_min(juz) {
                    sijill.nusus.push(nass);
                } else {
                    ilhaq_nusus_dakhiliya(juz, &mut sijill.nusus);
                }
            },
        }
    }
    Ok(sijill)
}

/// Reads the settings submessage of a product record.
fn iqra_idadat(bayt: &[u8], sijill: &mut SijillMuntaj) -> Result<(), KhataBrutu> {
    let mut qari = QariBrutu::jadeed(bayt);
    while !qari.intaha() {
        let (raqm, naw) = qari.tarwisa()?;
        if naw != NawHaql::Muhaddad {
            qari.takhatti(naw)?;
            continue;
        }
        let juz = qari.muhaddad()?;
        let Some(nass) = nass_min(juz) else {
            continue;
        };
        match raqm {
            1 => sijill.masar = Some(PathBuf::from(&nass)),
            2 => sijill.far = Some(nass),
            _ => sijill.nusus.push(nass),
        }
    }
    Ok(())
}

/// Pulls printable strings out of a nested message this reader has no schema
/// for, one level deep. Failures are silent by design: this is the salvage
/// path, and a submessage that will not parse simply contributes nothing.
fn ilhaq_nusus_dakhiliya(bayt: &[u8], nusus: &mut Vec<String>) {
    let mut qari = QariBrutu::jadeed(bayt);
    while !qari.intaha() {
        let Ok((_, naw)) = qari.tarwisa() else { return };
        if naw == NawHaql::Muhaddad {
            let Ok(juz) = qari.muhaddad() else { return };
            if let Some(nass) = nass_min(juz) {
                nusus.push(nass);
            }
        } else if qari.takhatti(naw).is_err() {
            return;
        }
    }
}

/// Interprets a length-delimited payload as a string, when it plausibly is one.
///
/// A protobuf string field and a submessage field are the same wire type, so
/// this is the only discriminator available without a schema: valid UTF-8 with
/// no control characters and no interior NUL.
fn nass_min(bayt: &[u8]) -> Option<String> {
    if bayt.is_empty() {
        return None;
    }
    let nass = std::str::from_utf8(bayt).ok()?;
    if nass.chars().any(char::is_control) {
        return None;
    }
    Some(nass.to_owned())
}

// ---------------------------------------------------------------------------
// .build.info — the cross-check
// ---------------------------------------------------------------------------

/// What a product's own `.build.info` says about the build sitting on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MaalumatBina {
    /// The branch, as `us`, `eu`, `kr`, `cn`.
    far: Option<String>,
    /// The version string, as `10.2.5.53040`.
    isdar: Option<String>,
    /// The product code this row belongs to.
    muntaj: Option<String>,
    /// The tag column, which carries the platform and locale of the build.
    wusum: Option<String>,
    /// Whether the row is the active install.
    nashit: bool,
}

/// Reads `<install>/.build.info`.
///
/// The format is a `|`-separated table whose header names each column with its
/// type and width — `Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|…` — and
/// whose rows carry one line per branch the product has ever been configured
/// for. Only one row has `Active` set to `1`, and that is the build installed.
/// Columns are addressed by header name rather than by position, because the
/// column order has changed between CASC versions and a positional reader
/// would silently read the CDN path as the version.
fn iqra_maalumat_bina(nass: &str) -> Option<MaalumatBina> {
    let mut sutur = nass.lines().filter(|satr| !satr.trim().is_empty());
    let tarwisa: Vec<String> = sutur
        .next()?
        .split('|')
        .map(|amud| amud.split('!').next().unwrap_or(amud).trim().to_ascii_lowercase())
        .collect();

    let mut awwal: Option<MaalumatBina> = None;
    for satr in sutur {
        let khanat: Vec<&str> = satr.split('|').collect();
        let khana = |ism: &str| -> Option<String> {
            let mawdi = tarwisa.iter().position(|amud| amud == ism)?;
            let qeema = khanat.get(mawdi)?.trim();
            (!qeema.is_empty()).then(|| qeema.to_owned())
        };
        let maalumat = MaalumatBina {
            far: khana("branch"),
            isdar: khana("version"),
            muntaj: khana("product"),
            wusum: khana("tags"),
            nashit: khana("active").as_deref() == Some("1"),
        };
        if maalumat.nashit {
            return Some(maalumat);
        }
        if awwal.is_none() {
            awwal = Some(maalumat);
        }
    }
    awwal
}

// ---------------------------------------------------------------------------
// record → game
// ---------------------------------------------------------------------------

/// Turns one product record into a discovered game, or into nothing when its
/// install directory is not on this machine any more.
fn luba_min_sijill(
    sijill: &SijillMuntaj,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    let jidhr = sijill.jidhr()?;
    let ramz = sijill.ramz_faal()?.to_owned();

    let masar_bina = jidhr.join(".build.info");
    let bina = if let Ok(bayt) = std::fs::read(&masar_bina) {
        let nass = String::from_utf8_lossy(&bayt).into_owned();
        let maalumat = iqra_maalumat_bina(&nass);
        if maalumat.is_none() {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar_bina.display().to_string(),
                "the .build.info table has no readable header row, so the installed branch \
                 and version are unknown; the game is still listed",
            ));
        }
        maalumat
    } else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar_bina.display().to_string(),
            "no .build.info in the install directory, which normally means the product is \
             mid-download or was removed outside Battle.net",
        ));
        None
    };

    // The protobuf's region field is the low-confidence one, so .build.info
    // wins wherever it parsed. Disagreement is recorded rather than resolved
    // silently: it is the signal that the inferred field number has moved.
    let far = bina
        .as_ref()
        .and_then(|maalumat| maalumat.far.clone())
        .or_else(|| sijill.far.clone());
    if let (Some(min_bina), Some(min_brutu)) =
        (bina.as_ref().and_then(|maalumat| maalumat.far.as_ref()), sijill.far.as_ref())
        && min_bina != min_brutu
    {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{ramz} ({})", jidhr.display()),
            format!(
                "product.db reports branch {min_brutu} and .build.info reports {min_bina}; \
                 .build.info was used"
            ),
        ));
    }

    let ism = ASMAA_MUNTAJAT
        .iter()
        .find(|(qeema, _)| *qeema == ramz)
        .map_or_else(|| ramz.clone(), |(_, ism)| (*ism).to_owned());

    let mut simat = Vec::new();
    if RUMUZ_GHAYR_LUBA.contains(&ramz.as_str()) {
        simat.push(SimatLuba::LaysatLuba("Battle.net client component".to_owned()));
    }

    // The tag column carries the platform and locale of the installed build —
    // `Windows x86_64 US?` — which is what tells Phase 15 whether the directory
    // holds a Windows build or a macOS one. It rides along inside the build
    // identifier rather than becoming a trait, because it describes the build,
    // not the game.
    let wusum = bina.as_ref().and_then(|maalumat| maalumat.wusum.clone());
    let bina_manassa =
        match (bina.as_ref().and_then(|maalumat| maalumat.isdar.clone()), far.as_ref(), wusum) {
            (Some(isdar), Some(far), Some(wusum)) => Some(format!("{far}:{isdar}:{wusum}")),
            (Some(isdar), Some(far), None) => Some(format!("{far}:{isdar}")),
            (Some(isdar), None, _) => Some(isdar),
            (None, _, _) => None,
        };

    Some(LubaMuktashafa {
        masdar: MasdarLuba::BattleNet(ramz),
        hala_matjar: None,
        ism,
        jidhr,
        tanfidhi: None,
        hajm: 0,
        bina_manassa,
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: MasadirSuwar::default(),
        khiyarat_tashghil: None,
        muktamila: bina.as_ref().is_some_and(|maalumat| maalumat.nashit),
        simat,
    })
}
