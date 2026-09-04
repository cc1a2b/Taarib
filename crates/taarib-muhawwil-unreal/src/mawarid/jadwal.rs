//! الجدول — `StringTable` assets, and exactly how much of a `.uasset` is read.
//!
//! A `UStringTable` is the one place a shipped Unreal game keeps *source* text
//! in an asset rather than in a compiled resource. A designer authors a table of
//! key/string pairs, widgets reference a row by table id and key, and the
//! localization pipeline gathers those rows into the `.manifest` that eventually
//! becomes a `.locres`. So a game's translatable text is split: the translations
//! live in [`super::locres`], and the English a translator was working from
//! lives here.
//!
//! Taarib reads these for two reasons. Extraction needs the source strings, with
//! their table id and key, because that pair is what a `.locres` entry's
//! namespace and key are derived from — a string extracted without them cannot
//! be reinjected. And a small number of games ship a `StringTable` with no
//! `.locres` at all, using the table as the display text directly; those are the
//! games where replacing a source string is the only route there is.
//!
//! ## What is read, precisely
//!
//! **This is not a `.uasset` parser and does not become one.** What is
//! implemented is the payload `FStringTable::Serialize` writes, which is a
//! self-describing, length-prefixed run of bytes sitting inside one export:
//!
//! ```text
//! JadwalNusus — the FStringTable payload, little-endian
//!
//!   offset  size  field            type      meaning
//!        0   var  muarrif          FString   the table id, which is its namespace
//!        -     4  adad_sufuf       i32       how many key/source rows follow
//!   repeat adad_sufuf times:
//!        -   var  miftah           FString   the row's key
//!        -   var  asl              FString   the row's source string
//!        -     4  adad_bayanat     i32       how many keys carry metadata
//!   repeat adad_bayanat times:
//!        -   var  miftah           FString   which key the metadata belongs to
//!        -     4  adad_qeyam       i32       how many metadata values it carries
//!     repeat adad_qeyam times:
//!        -     8  muarrif_qeema    FName     a name-map reference, kept verbatim
//!        -   var  qeema            FString   the metadata value
//! ```
//!
//! Everything else in the package is untouched and unread: the file summary, the
//! name map, the import and export maps, soft object paths, gatherable text
//! data, the thumbnail table, asset registry data, bulk data, and every other
//! export. None of it is parsed, so none of it can be parsed wrongly.
//!
//! ## What is refused, and why each refusal is the honest answer
//!
//! * **A metadata identifier is not resolved to a name.** It is an `FName` — an
//!   eight-byte pair of an index into the package's name map and a number — and
//!   resolving it would mean parsing the summary and the name map, which is the
//!   general parser this module refuses to be. The eight bytes are carried
//!   verbatim in [`MarjaIsm`], so the payload round-trips exactly and a caller
//!   that does have a name map can resolve them itself.
//! * **An edit that changes the payload's byte length cannot be spliced back
//!   into a package.** A package's summary records its total header size and its
//!   bulk-data start offset, and every export records its own serial offset and
//!   size; moving a payload by one byte invalidates all of them. Rewriting them
//!   is a `.uasset` writer. [`MawqiJadwal::ila_hizma`] therefore refuses a
//!   different length, by name, rather than producing a package that mounts and
//!   then fails to load — and the supported route for a longer translation is
//!   the one the engine already has, which is a `.locres` entry that overrides
//!   the source string at run time.
//! * **A package this build cannot find a payload in is refused, not guessed
//!   at.** See the locator below.
//!
//! ## Finding the payload without parsing the package
//!
//! In a cooked game the export payloads usually live in a `.uexp` beside the
//! `.uasset`, and in an uncooked or single-file package they live in the
//! `.uasset` itself. Rather than parse either container,
//! [`JadwalNusus::jid_fi_kutla`] scans the bytes for a position where the whole
//! structure above parses consistently: a plausible table id, a row count whose
//! rows all read, a metadata count whose rows all read, and an end inside the
//! buffer. A structure that long is not something arbitrary bytes fall into by
//! accident — the shortest accepted match already commits about twenty
//! interdependent length fields — and a candidate that parses is accepted only
//! if it has at least one row.
//!
//! The scan is bounded twice: it starts every candidate on a four-byte boundary,
//! because Unreal's archives write `int32` fields aligned and an export begins at
//! one; and it gives up after [`AQSA_MUHAWALAT`] full parse attempts, so a large
//! package of hostile bytes costs a bounded amount of work rather than a
//! quadratic amount.
//!
//! [`JadwalNusus::min_uasset`] is the same scan with the package tag checked
//! first, so pointing it at something that is not an Unreal package fails with a
//! magic refusal rather than with "no string table here".

use std::path::{Path, PathBuf};

use super::{
    Katib, Mawrid, NassMukhazzan, Qari, adad_musir, iqra_malaf, sammi_masar, tahaqquq_adad,
    uktub_malaf,
};
use crate::khata::{KhataUnreal, tul_u64};

/// The four bytes an Unreal package begins with: `0x9E2A83C1`, little-endian.
pub const SIHR_HIZMA: [u8; 4] = [0xC1, 0x83, 0x2A, 0x9E];

/// The format's name in every refusal this module raises.
const ISM: &str = "StringTable";

/// The largest number of key/source rows one table may declare.
///
/// A million. The largest authored `StringTable` seen in a shipped game holds a
/// few thousand rows; a table with a million would be a table nobody authored.
pub const AQSA_SUFUF: u64 = 1 << 20;

/// The largest number of keys that may carry metadata.
pub const AQSA_BAYANAT: u64 = 1 << 20;

/// The largest number of metadata values one key may carry.
pub const AQSA_QEYAM: u64 = 1 << 12;

/// How many candidate offsets the locator will fully parse before giving up.
///
/// The prefilter rejects almost everything for the cost of one `i32` read, so
/// this counts only the positions that looked like a table id. Four thousand of
/// those in one package is already a package that is not what it claims to be,
/// and the cap is what keeps the scan linear instead of quadratic.
pub const AQSA_MUHAWALAT: u32 = 4096;

/// The longest table id the prefilter will consider, in units.
///
/// Table ids are asset names and package paths — `ST_Dialogue`, or
/// `/Game/UI/ST_Menu`. Two hundred and fifty-six units is well past the longest
/// real one and short enough to reject nearly every random `i32`.
const AQSA_MUARRIF: u32 = 256;

/// The fewest bytes one key/source row can occupy: two empty `FString`s.
const AQALL_SAFF: u64 = 8;

/// The fewest bytes one metadata row can occupy: an empty key and a count.
const AQALL_BAYANAT: u64 = 8;

/// The fewest bytes one metadata value can occupy: an `FName` and an empty
/// `FString`.
const AQALL_QEEMA: u64 = 12;

/// A metadata identifier, kept as the eight bytes the package stored.
///
/// An `FName` is an index into the package's name map plus a number, and the
/// name map is not read here. Carrying the bytes rather than a resolved string
/// is what lets the payload round-trip exactly while this module stays out of
/// the business of parsing packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MarjaIsm([u8; 8]);

impl MarjaIsm {
    /// The eight bytes, as stored.
    #[must_use]
    pub const fn khaam(self) -> [u8; 8] {
        self.0
    }

    /// The name-map index, for a caller that has the name map.
    #[must_use]
    pub fn fahras(self) -> u32 {
        self.0.first_chunk::<4>().map_or(0, |khana| u32::from_le_bytes(*khana))
    }

    /// The instance number Unreal appends to a repeated name.
    #[must_use]
    pub fn raqm(self) -> u32 {
        self.0.last_chunk::<4>().map_or(0, |khana| u32::from_le_bytes(*khana))
    }
}

/// One row: a key and the source string under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SijillJadwal {
    miftah: NassMukhazzan,
    asl: NassMukhazzan,
}

impl SijillJadwal {
    /// A new row.
    #[must_use]
    pub fn jadeed(miftah: &str, asl: &str) -> Self {
        Self { miftah: NassMukhazzan::jadeed(miftah), asl: NassMukhazzan::jadeed(asl) }
    }

    /// The row's key, which with the table id identifies the string.
    #[must_use]
    pub fn miftah(&self) -> &str {
        self.miftah.nass()
    }

    /// The source string.
    #[must_use]
    pub fn asl(&self) -> &str {
        self.asl.nass()
    }

    /// The source string, with the encoding it was stored in.
    #[must_use]
    pub const fn nass_asl(&self) -> &NassMukhazzan {
        &self.asl
    }
}

/// The metadata one key carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SijillBayanat {
    miftah: NassMukhazzan,
    qeyam: Vec<(MarjaIsm, NassMukhazzan)>,
}

impl SijillBayanat {
    /// Which key this metadata belongs to.
    #[must_use]
    pub fn miftah(&self) -> &str {
        self.miftah.nass()
    }

    /// The values, each with the unresolved identifier it was filed under.
    #[must_use]
    pub fn qeyam(&self) -> &[(MarjaIsm, NassMukhazzan)] {
        &self.qeyam
    }
}

/// One `StringTable` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JadwalNusus {
    muarrif: NassMukhazzan,
    sufuf: Vec<SijillJadwal>,
    bayanat: Vec<SijillBayanat>,
}

impl Mawrid for JadwalNusus {
    const ISM: &'static str = ISM;

    fn min_bayt(bayt: &[u8]) -> Result<Self, KhataUnreal> {
        let mut qari = Qari::jadeed(ISM, bayt);
        let jadwal = min_qari(&mut qari)?;
        // Exact consumption, because this entry point is for a payload somebody
        // has already isolated. A caller with a whole package uses the locator,
        // which reports where the payload ends instead of demanding it be the
        // end of the buffer.
        if qari.mawqi() != bayt.len() {
            return Err(KhataUnreal::MawridTalif {
                ism: ISM,
                haql: "the payload's length, which must be exactly the bytes given",
                qeema: tul_u64(qari.mawqi()),
                hadd: tul_u64(bayt.len()),
            });
        }
        Ok(jadwal)
    }

    fn ila_bayt(&self) -> Result<Vec<u8>, KhataUnreal> {
        let mut katib = Katib::bi_siaa(self.siaa_mutawaqqaa());
        katib.uktub_nass(ISM, "the table id", &self.muarrif)?;
        katib.uktub_i32(adad_khana("adad_sufuf", self.sufuf.len(), AQSA_SUFUF)?);
        for saff in &self.sufuf {
            katib.uktub_nass(ISM, "a row's key", &saff.miftah)?;
            katib.uktub_nass(ISM, "a row's source string", &saff.asl)?;
        }
        katib.uktub_i32(adad_khana("adad_bayanat", self.bayanat.len(), AQSA_BAYANAT)?);
        for sijill in &self.bayanat {
            katib.uktub_nass(ISM, "a metadata row's key", &sijill.miftah)?;
            katib.uktub_i32(adad_khana("adad_qeyam", sijill.qeyam.len(), AQSA_QEYAM)?);
            for (muarrif, qeema) in &sijill.qeyam {
                katib.uktub_bayt(&muarrif.khaam());
                katib.uktub_nass(ISM, "a metadata value", qeema)?;
            }
        }
        Ok(katib.ila_vec())
    }
}

impl JadwalNusus {
    /// An empty table with a given id.
    #[must_use]
    pub fn jadeed(muarrif: &str) -> Self {
        Self {
            muarrif: NassMukhazzan::jadeed(muarrif),
            sufuf: Vec::new(),
            bayanat: Vec::new(),
        }
    }

    /// The table id, which is the namespace its rows resolve under.
    #[must_use]
    pub fn muarrif(&self) -> &str {
        self.muarrif.nass()
    }

    /// The rows, in the order the asset lists them.
    #[must_use]
    pub fn sufuf(&self) -> &[SijillJadwal] {
        &self.sufuf
    }

    /// The metadata rows, in the order the asset lists them.
    #[must_use]
    pub fn bayanat(&self) -> &[SijillBayanat] {
        &self.bayanat
    }

    /// The source string for a key.
    #[must_use]
    pub fn asl(&self, miftah: &str) -> Option<&str> {
        self.sufuf.iter().find(|saff| saff.miftah() == miftah).map(SijillJadwal::asl)
    }

    /// Replaces one row's source string, returning whether the row was there.
    ///
    /// Nothing is created. A key the asset does not have is a key no widget
    /// references, so inventing it would add bytes the game never reads — and
    /// would change the payload's length, which is the one thing that stops it
    /// being spliced back.
    pub fn istabdil(&mut self, miftah: &str, asl: &str) -> bool {
        let Some(saff) = self.sufuf.iter_mut().find(|saff| saff.miftah.nass() == miftah) else {
            return false;
        };
        saff.asl.ghayyir(asl);
        true
    }

    /// Adds a row, replacing the source string if the key is already there.
    ///
    /// For a table Taarib authors. Adding to a table read out of a package makes
    /// a payload that no longer fits the space the package reserved for it —
    /// which [`MawqiJadwal::ila_hizma`] then refuses, by design.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::HajmMufrit`] when the table is already at [`AQSA_SUFUF`].
    pub fn adif(&mut self, miftah: &str, asl: &str) -> Result<(), KhataUnreal> {
        if self.istabdil(miftah, asl) {
            return Ok(());
        }
        if tul_u64(self.sufuf.len()) >= AQSA_SUFUF {
            return Err(KhataUnreal::HajmMufrit {
                haql: "adad_sufuf",
                qeema: tul_u64(self.sufuf.len()).saturating_add(1),
                saqf: AQSA_SUFUF,
            });
        }
        self.sufuf.push(SijillJadwal::jadeed(miftah, asl));
        Ok(())
    }

    /// Finds the payload inside any buffer of Unreal export bytes.
    ///
    /// For a cooked game that is the `.uexp`; for an uncooked or single-file
    /// package it is the `.uasset` itself. No package structure is parsed — see
    /// this module's header for how the scan is bounded and why a structural
    /// match is trustworthy.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] naming the string table payload when no
    /// position in the buffer parses as one, which includes the ordinary case of
    /// a package that simply holds no string table.
    pub fn jid_fi_kutla(bayt: &[u8]) -> Result<MawqiJadwal, KhataUnreal> {
        let mut muhawalat = 0u32;
        let mut izaha = 0usize;
        while izaha < bayt.len() {
            if muhawalat >= AQSA_MUHAWALAT {
                break;
            }
            if murashshah(bayt, izaha) {
                muhawalat = muhawalat.saturating_add(1);
                if let Some(mawqi) = jarrib(bayt, izaha) {
                    return Ok(mawqi);
                }
            }
            izaha = izaha.saturating_add(4);
        }
        Err(KhataUnreal::MawridTalif {
            ism: ISM,
            haql: "the string table payload, which no position in these bytes parses as",
            qeema: tul_u64(bayt.len()),
            hadd: tul_u64(bayt.len()),
        })
    }

    /// Finds the payload inside an Unreal package, checking the package tag
    /// first.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SihrGhayrMutabaq`] when the first four bytes are not the
    /// package tag, and otherwise whatever [`JadwalNusus::jid_fi_kutla`]
    /// refuses. A cooked package whose exports were split into a `.uexp` will
    /// fail here with the payload refusal, and the caller should pass the
    /// `.uexp`'s bytes to [`JadwalNusus::jid_fi_kutla`] instead.
    pub fn min_uasset(bayt: &[u8]) -> Result<MawqiJadwal, KhataUnreal> {
        if bayt.get(..SIHR_HIZMA.len()) != Some(SIHR_HIZMA.as_slice()) {
            return Err(KhataUnreal::SihrGhayrMutabaq { malaf: PathBuf::new(), ism: ISM });
        }
        Self::jid_fi_kutla(bayt)
    }

    /// Reads a package from a path and finds the payload in it.
    ///
    /// **The convenience function.** Everything else works over `&[u8]`, because
    /// a `.uasset` is found inside a `.pak` or an IoStore chunk far more often
    /// than it is found loose.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming the path, [`KhataUnreal::HajmMufrit`]
    /// when the file is above [`super::AQSA_MALAF`], and otherwise whatever
    /// [`JadwalNusus::min_uasset`] refuses.
    pub fn min_malaf(masar: &Path) -> Result<MawqiJadwal, KhataUnreal> {
        let bayt = iqra_malaf(masar)?;
        Self::min_uasset(&bayt).map_err(|khata| sammi_masar(khata, masar))
    }

    /// A rough size for the output buffer.
    fn siaa_mutawaqqaa(&self) -> usize {
        self.sufuf
            .len()
            .saturating_mul(64)
            .saturating_add(self.bayanat.len().saturating_mul(32))
            .max(64)
    }
}

/// A payload together with where it sat in the package it came from.
///
/// The offset and length are what make reinjection possible at all: they are the
/// exact span [`MawqiJadwal::ila_hizma`] replaces, and the length is what an
/// edited payload has to still be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawqiJadwal {
    izaha: usize,
    tul: usize,
    jadwal: JadwalNusus,
}

impl MawqiJadwal {
    /// Where the payload starts in the package.
    #[must_use]
    pub const fn izaha(&self) -> usize {
        self.izaha
    }

    /// How many bytes it occupied.
    #[must_use]
    pub const fn tul(&self) -> usize {
        self.tul
    }

    /// The table.
    #[must_use]
    pub const fn jadwal(&self) -> &JadwalNusus {
        &self.jadwal
    }

    /// The table, to edit.
    pub const fn jadwal_mut(&mut self) -> &mut JadwalNusus {
        &mut self.jadwal
    }

    /// Splices the table back into the package it came from.
    ///
    /// `bayt` must be the same package these bytes were located in. The result
    /// is that package with the payload's span replaced, and with no other byte
    /// touched — so calling this straight after locating, with no edit, returns
    /// the input unchanged.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::MawridTalif`] when the edited payload is not exactly as
    /// long as the original — the refusal this module's header explains, and the
    /// point at which a caller should reinject through a `.locres` instead — or
    /// when the recorded span is not inside `bayt`, which means this value was
    /// paired with a different package. Also whatever [`Mawrid::ila_bayt`]
    /// refuses.
    pub fn ila_hizma(&self, bayt: &[u8]) -> Result<Vec<u8>, KhataUnreal> {
        let jadeed = self.jadwal.ila_bayt()?;
        if jadeed.len() != self.tul {
            return Err(KhataUnreal::MawridTalif {
                ism: ISM,
                haql: "the edited payload's length, which the package's own offsets fix",
                qeema: tul_u64(jadeed.len()),
                hadd: tul_u64(self.tul),
            });
        }
        let nihaya = self.izaha.checked_add(self.tul).ok_or_else(|| KhataUnreal::MawridTalif {
            ism: ISM,
            haql: "the payload's recorded span",
            qeema: tul_u64(self.izaha),
            hadd: tul_u64(bayt.len()),
        })?;
        let qabl = bayt.get(..self.izaha).ok_or_else(|| KhataUnreal::MawridTalif {
            ism: ISM,
            haql: "the payload's recorded span",
            qeema: tul_u64(self.izaha),
            hadd: tul_u64(bayt.len()),
        })?;
        let baad = bayt.get(nihaya..).ok_or_else(|| KhataUnreal::MawridTalif {
            ism: ISM,
            haql: "the payload's recorded span",
            qeema: tul_u64(nihaya),
            hadd: tul_u64(bayt.len()),
        })?;
        let mut natija = Vec::with_capacity(bayt.len());
        natija.extend_from_slice(qabl);
        natija.extend_from_slice(&jadeed);
        natija.extend_from_slice(baad);
        Ok(natija)
    }

    /// Splices the table back into a package on disk and writes it.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhataMalaf`] naming a path, and otherwise whatever
    /// [`MawqiJadwal::ila_hizma`] refuses.
    pub fn ila_malaf(&self, masdar: &Path, hadaf: &Path) -> Result<(), KhataUnreal> {
        let bayt = iqra_malaf(masdar)?;
        uktub_malaf(hadaf, &self.ila_hizma(&bayt)?)
    }
}

// ---------------------------------------------------------------------------
// Reading, and the locator
// ---------------------------------------------------------------------------

/// One payload from a cursor, leaving it just past the end of the table.
fn min_qari(qari: &mut Qari<'_>) -> Result<JadwalNusus, KhataUnreal> {
    let muarrif = qari.iqra_nass("the table id")?;

    let muallan = qari.iqra_i32("adad_sufuf")?;
    let adad = tahaqquq_adad(
        ISM,
        "adad_sufuf",
        adad_musir(ISM, "adad_sufuf", muallan)?,
        AQSA_SUFUF,
        AQALL_SAFF,
        qari.baqi(),
    )?;
    let mut sufuf = Vec::with_capacity(adad);
    for _ in 0..adad {
        let miftah = qari.iqra_nass("a row's key")?;
        let asl = qari.iqra_nass("a row's source string")?;
        sufuf.push(SijillJadwal { miftah, asl });
    }

    let muallan = qari.iqra_i32("adad_bayanat")?;
    let adad = tahaqquq_adad(
        ISM,
        "adad_bayanat",
        adad_musir(ISM, "adad_bayanat", muallan)?,
        AQSA_BAYANAT,
        AQALL_BAYANAT,
        qari.baqi(),
    )?;
    let mut bayanat = Vec::with_capacity(adad);
    for _ in 0..adad {
        let miftah = qari.iqra_nass("a metadata row's key")?;
        let muallan = qari.iqra_i32("adad_qeyam")?;
        let adad_qeyam = tahaqquq_adad(
            ISM,
            "adad_qeyam",
            adad_musir(ISM, "adad_qeyam", muallan)?,
            AQSA_QEYAM,
            AQALL_QEEMA,
            qari.baqi(),
        )?;
        let mut qeyam = Vec::with_capacity(adad_qeyam);
        for _ in 0..adad_qeyam {
            let muarrif_qeema: [u8; 8] = qari.iqra_masfufa("a metadata identifier")?;
            let qeema = qari.iqra_nass("a metadata value")?;
            qeyam.push((MarjaIsm(muarrif_qeema), qeema));
        }
        bayanat.push(SijillBayanat { miftah, qeyam });
    }

    Ok(JadwalNusus { muarrif, sufuf, bayanat })
}

/// The cheap test a candidate offset has to pass before it is parsed.
///
/// One `i32` read and a look at the terminator. Everything a full parse would
/// have to allocate for is behind this.
fn murashshah(bayt: &[u8], izaha: usize) -> bool {
    let Some(khana) = bayt.get(izaha..).and_then(<[u8]>::first_chunk::<4>) else {
        return false;
    };
    let tul = i32::from_le_bytes(*khana);
    // `unsigned_abs` rather than `abs`, because `i32::MIN` has no absolute value
    // and this runs on every four-byte window of a file somebody else wrote.
    // A table id is never empty, so the length is at least two units: one
    // character and the terminator.
    if !(2..=AQSA_MUARRIF).contains(&tul.unsigned_abs()) {
        return false;
    }
    let bidaya = izaha.saturating_add(4);
    let wahdat = usize::try_from(tul.unsigned_abs()).unwrap_or(0);
    if tul > 0 {
        let akhir = bidaya.saturating_add(wahdat).saturating_sub(1);
        return bayt.get(akhir) == Some(&0);
    }
    let akhir = bidaya.saturating_add(wahdat.saturating_mul(2)).saturating_sub(2);
    bayt.get(akhir..akhir.saturating_add(2))
        .is_some_and(|zawj| zawj.iter().all(|wahda| *wahda == 0))
}

/// One full parse attempt at a candidate offset.
///
/// A candidate is accepted only when the whole structure reads and the table has
/// at least one row. A zero-row table is legal in the format and is exactly what
/// a run of zero bytes looks like, so accepting one would make every zeroed
/// region in a package a false match.
fn jarrib(bayt: &[u8], izaha: usize) -> Option<MawqiJadwal> {
    let dhayl = bayt.get(izaha..)?;
    let mut qari = Qari::jadeed(ISM, dhayl);
    let jadwal = min_qari(&mut qari).ok()?;
    if jadwal.sufuf.is_empty() {
        return None;
    }
    Some(MawqiJadwal { izaha, tul: qari.mawqi(), jadwal })
}

/// A count as the `i32` the format writes, refused when it does not fit.
fn adad_khana(haql: &'static str, adad: usize, saqf: u64) -> Result<i32, KhataUnreal> {
    i32::try_from(adad)
        .map_err(|_| KhataUnreal::HajmMufrit { haql, qeema: tul_u64(adad), saqf })
}
