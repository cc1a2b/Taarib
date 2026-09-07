//! القارئ — reading a patch, in the order that makes reading it safe.
//!
//! A `.ruqaa` is a file a stranger produced. It arrives over the network, from a
//! registry this client does not control, and it is read inside somebody's game
//! process. Every number in it is attacker-controlled until proven otherwise,
//! and this module is where that proof happens.
//!
//! ## The validation order, and why it is this order
//!
//! 1. **The header's shape** — magic, version, flags, section count, and the
//!    declared total size against the real length ([`crate::tarwisa::Tarwisa`]).
//! 2. **The section table** — every offset and length inside the file, every
//!    section aligned, no two overlapping, no kind twice, the signature block
//!    last ([`crate::tarwisa::JadwalAqsam`]).
//! 3. **The content hash** — BLAKE3 over the bytes between the header and the
//!    signature block, compared against what the header claims.
//! 4. **Only then, anything in the body** — decompression, section preambles,
//!    records, string offsets.
//!
//! One and two before three because hashing requires knowing which bytes to
//! hash, and a section table pointing outside the file cannot be walked at all.
//! Three before four because **parsing attacker-controlled data before knowing
//! it is authentic is what this ordering exists to prevent**: every preamble,
//! record count and string offset in step four is a number that decides how much
//! memory is touched, and a hash is worth nothing if it is checked after the
//! parser has already run on the bytes it was meant to authorise.
//!
//! The hash deliberately stops at the signature block rather than running to the
//! end of the file. That is what lets `taarib-khatm` seal a finished package by
//! overwriting a fixed-size reservation in place, without the hash it is signing
//! changing underneath it.
//!
//! Whether the right key produced that signature is [`crate::tawqee`]'s
//! question, and whether the answer is acceptable is the installer's. This
//! module proves only that the bytes are internally consistent and are the bytes
//! the hash names.
//!
//! ## One implementation over a slice
//!
//! Everything here works on `&[u8]`. Memory-mapping is one way to obtain such a
//! slice and is what a game process does; the compiler and the review console
//! hold the bytes in memory. There is deliberately no second code path for the
//! mapped case — two readers would be two sets of bounds checks, and the one
//! that ran less often would be the one that was wrong.

use bytemuck::Pod;

use crate::aqsam::{MadkhalQism, NawDaght, NawQism};
use crate::jadawil::{
    HAJM_TASDIR, HAJM_TASDIR_KABIR, MarjaNass, SijillHarf, SijillKhatt, SijillMawdiShakl,
    SijillMiftahShakl, SijillNass, SijillNitaq, SijillQayd, SijillSafha, SijillSatr, SijillTakhtit,
    TarwisatKhareeta, TarwisatKhatt, TarwisatLawha, TarwisatNusus, TarwisatQiyud, TarwisatTakhtit,
};
use crate::khata::KhataRuqaa;
use crate::muhadhah::BaytMuhadhah;
use crate::tarwisa::{
    AQSA_QISM_KHAAM, HAJM_TARWISA, JadwalAqsam, Tarwisa, hajm_usize, sittasi, tul_u64,
};
use crate::tawqee::KutlatTawqee;

/// A section's bytes, after any decompression it needed.
///
/// Borrowed when the section was stored uncompressed — the case an adapter
/// wants, because then reading a table is a pointer add into a mapped page and
/// nothing is copied at all.
///
/// The owned case is a [`BaytMuhadhah`] and not a `Vec<u8>` for a reason that is
/// not stylistic: a `Vec<u8>` is one-byte aligned, and a table cast out of one
/// would be refused by `bytemuck` whenever the allocator returned an address
/// that did not happen to suit. [`crate::muhadhah`] explains it at length.
#[derive(Debug)]
pub enum BayanatQism<'a> {
    /// Stored uncompressed; this is a view into the file.
    Muarra(&'a [u8]),
    /// Stored compressed; this is the decompressed copy, on an aligned buffer.
    Mafkuka(BaytMuhadhah),
}

impl BayanatQism<'_> {
    /// The bytes, however they were obtained.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        match self {
            Self::Muarra(bayt) => bayt,
            Self::Mafkuka(bayt) => bayt.bayt(),
        }
    }

    /// Whether these bytes are a view into the file rather than a copy.
    #[must_use]
    pub const fn muarra(&self) -> bool {
        matches!(self, Self::Muarra(_))
    }
}

/// A validated patch.
///
/// Holding one of these is the proof that steps one to three have passed.
/// Nothing in this module hands one out before they have.
#[derive(Debug)]
pub struct Ruqaa<'a> {
    bayt: &'a [u8],
    tarwisa: Tarwisa,
    jadwal: JadwalAqsam,
    tawqee: KutlatTawqee,
}

impl<'a> Ruqaa<'a> {
    /// Validates a patch's framing and returns a reader for it.
    ///
    /// Performs steps one, two and three. The body is not touched: no preamble
    /// is read, no record is cast, and no section is decompressed until a caller
    /// asks for one.
    ///
    /// `bayt` should begin on a sixteen-byte boundary. Framing is validated
    /// either way — every check here is arithmetic on offsets — but a table
    /// stored uncompressed is cast in place, and casting it needs the section's
    /// absolute address to be aligned, not just its offset within the file. A
    /// caller that reads a container into a `Vec<u8>` and passes it here will
    /// pass every check in this function and then be refused by
    /// [`KhataRuqaa::MuhadhahaGhayrSaliha`] at the first table it reads, on some
    /// machines and not others. Both supported ways of obtaining the bytes —
    /// [`MalafRuqaa`], which maps, and [`BaytMuhadhah`], which copies onto an
    /// aligned buffer — satisfy it by construction.
    ///
    /// # Errors
    ///
    /// Whatever the header, the section table or the content hash refuses,
    /// naming the field. A file that is not a `.ruqaa` fails at the magic with
    /// [`KhataRuqaa::SihrGhayrMutabaq`].
    pub fn iftah(bayt: &'a [u8]) -> Result<Self, KhataRuqaa> {
        let tarwisa = Tarwisa::min_bayt(bayt)?;
        let jadwal = JadwalAqsam::min_bayt(bayt, &tarwisa)?;
        tahaqquq_basma(bayt, &tarwisa, &jadwal)?;
        let tawqee = iqra_tawqee(bayt, &jadwal)?;
        Ok(Self {
            bayt,
            tarwisa,
            jadwal,
            tawqee,
        })
    }

    /// The header.
    #[must_use]
    pub const fn tarwisa(&self) -> &Tarwisa {
        &self.tarwisa
    }

    /// The section table.
    #[must_use]
    pub const fn jadwal(&self) -> &JadwalAqsam {
        &self.jadwal
    }

    /// The signature block, parsed but not verified.
    ///
    /// Verifying it needs a key and a verifier, neither of which this crate has;
    /// [`KutlatTawqee::tahaqquq`] takes both. What is guaranteed here is that the
    /// block is well formed and that the hash it commits to is the hash of these
    /// bytes.
    #[must_use]
    pub const fn tawqee(&self) -> &KutlatTawqee {
        &self.tawqee
    }

    /// The content hash the signature commits to.
    #[must_use]
    pub const fn basma(&self) -> &[u8; 32] {
        &self.tarwisa.basma
    }

    /// Whether a section is present.
    #[must_use]
    pub fn yahwi(&self, naw: NawQism) -> bool {
        self.jadwal.qism(naw).is_some()
    }

    /// Every section present, with its stored and uncompressed sizes.
    ///
    /// What Phase 14's asset gate checks against: a package whose sections do
    /// not account for its bytes is a package carrying something nobody
    /// described.
    #[must_use]
    pub fn jard(&self) -> Vec<(NawQism, u64, u64)> {
        self.jadwal
            .madkhalat()
            .map(|madkhal| (madkhal.naw, madkhal.tul_makhzun, madkhal.tul_khaam))
            .collect()
    }

    /// A section's bytes, decompressed if it needed it.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::QismMafqud`] when the section is not in this patch,
    /// [`KhataRuqaa::HajmKhaamMufrit`] when it declares more than the ceiling,
    /// [`KhataRuqaa::FakkFashil`] when decompression fails, and
    /// [`KhataRuqaa::HajmKhaamGhayrMutabaq`] when what came out is not the size
    /// it promised — the check that turns a decompression bomb into a refusal
    /// rather than an exhausted machine.
    pub fn qism(&self, naw: NawQism) -> Result<BayanatQism<'a>, KhataRuqaa> {
        let madkhal = self
            .jadwal
            .qism(naw)
            .ok_or_else(|| KhataRuqaa::QismMafqud { ism: naw.ism() })?;
        let makhzun = shariha(self.bayt, &madkhal)?;
        match madkhal.daght {
            NawDaght::Bila => Ok(BayanatQism::Muarra(makhzun)),
            NawDaght::Zstd => Ok(BayanatQism::Mafkuka(fukk(naw, makhzun, madkhal.tul_khaam)?)),
        }
    }

    /// The metadata section, as raw JSON bytes.
    ///
    /// Returned as bytes rather than parsed: the metadata record's shape belongs
    /// to `taarib-mustalahat` and its schema-version discipline, and a format
    /// crate that also decided what metadata means would be a second owner of
    /// one question.
    ///
    /// # Errors
    ///
    /// As [`Ruqaa::qism`].
    pub fn bayan(&self) -> Result<BayanatQism<'a>, KhataRuqaa> {
        self.qism(NawQism::Bayan)
    }

    /// The metadata section, parsed far enough to know it is JSON.
    ///
    /// Deliberately a [`serde_json::Value`] and not a typed record. What the
    /// fields mean is `taarib-mustalahat`'s question and is governed by its
    /// schema-version discipline; what this crate can answer is whether the
    /// section is JSON at all, which is the difference between a patch with
    /// metadata a newer Taarib understands and a patch whose metadata section is
    /// forty kilobytes of something else.
    ///
    /// # Errors
    ///
    /// As [`Ruqaa::qism`], plus [`KhataRuqaa::BayanTalif`] when the bytes are
    /// not JSON.
    pub fn bayan_json(&self) -> Result<serde_json::Value, KhataRuqaa> {
        let bayanat = self.bayan()?;
        serde_json::from_slice(bayanat.bayt()).map_err(|khata| KhataRuqaa::BayanTalif {
            tafsil: khata.to_string(),
        })
    }
}

/// A patch held open as a memory-mapped file.
///
/// What an adapter inside a game uses. The alternative — reading a hundred
/// megabytes of atlas into the heap at launch — is a hundred megabytes of a
/// game's memory budget spent on pages most sessions never touch, in a process
/// that may be a 32-bit build already short of address space.
///
/// Borrow the patch with [`MalafRuqaa::ruqaa`]; the mapping lives as long as
/// this value does.
#[derive(Debug)]
pub struct MalafRuqaa {
    khareeta: memmap2::Mmap,
}

impl MalafRuqaa {
    /// Opens and maps a patch, then validates its framing.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::KhataMalaf`] naming the path when it cannot be opened or
    /// mapped, and otherwise whatever [`Ruqaa::iftah`] refuses.
    ///
    /// # Safety of the mapping
    ///
    /// A memory map is only as stable as the file behind it: if another process
    /// truncates the file while this value is alive, touching the mapped pages
    /// past the new end is undefined behaviour, and no amount of bounds checking
    /// in this crate can prevent that. Taarib's own installer writes patches to
    /// a temporary path and renames them into place, which on every platform the
    /// product targets leaves an already-open file's contents intact, and it
    /// never writes to a patch that a game may be reading. This function is safe
    /// to call under that discipline and is not safe against an adversary with
    /// write access to the patch directory — who, having write access to the
    /// patch directory, has already won.
    pub fn iftah(masar: &std::path::Path) -> Result<Self, KhataRuqaa> {
        let malaf = std::fs::File::open(masar).map_err(|sabab| KhataRuqaa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })?;
        // SAFETY: see the section above. The mapping is read-only, and the file
        // is one this product wrote and does not rewrite in place.
        let khareeta =
            unsafe { memmap2::Mmap::map(&malaf) }.map_err(|sabab| KhataRuqaa::KhataMalaf {
                masar: masar.to_path_buf(),
                sabab,
            })?;
        // Validated here rather than on demand so that a malformed patch is
        // refused at open, where the caller still has the path to name in the
        // message.
        Ruqaa::iftah(&khareeta)?;
        Ok(Self { khareeta })
    }

    /// The validated patch, borrowed from the mapping.
    ///
    /// # Errors
    ///
    /// None in practice: the same bytes passed [`Ruqaa::iftah`] when the file
    /// was opened. The result is still returned rather than unwrapped, because
    /// "in practice" is doing work in that sentence — the bytes are a mapping of
    /// a file on a disk somebody else can reach, and a crate that answered this
    /// question by aborting the process would be answering it wrongly.
    pub fn ruqaa(&self) -> Result<Ruqaa<'_>, KhataRuqaa> {
        Ruqaa::iftah(&self.khareeta)
    }

    /// The mapped bytes.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.khareeta
    }
}

// ---------------------------------------------------------------------------
// Section readers
// ---------------------------------------------------------------------------

/// The strings, their style spans, and the pool both point into.
#[derive(Debug)]
pub struct JadwalNusus<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatNusus,
    /// One record per translated string, sorted ascending by `miftah`.
    pub sijillat: &'a [SijillNass],
    /// Every style span, sorted ascending by the string it belongs to.
    pub nitaqat: &'a [SijillNitaq],
    /// The UTF-8 pool the string references point into.
    pub hawd: &'a [u8],
}

impl JadwalNusus<'_> {
    /// How many strings the patch carries.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.sijillat.len()
    }

    /// Whether the patch carries no strings, which a font-only patch does not.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.sijillat.is_empty()
    }

    /// Resolves a string reference against the pool.
    ///
    /// Returns [`None`] for a reference pointing outside it, or for bytes that
    /// are not UTF-8.
    #[must_use]
    pub fn nass(&self, marja: MarjaNass) -> Option<&str> {
        let nitaq = marja.nitaq(self.hawd.len())?;
        core::str::from_utf8(self.hawd.get(nitaq)?).ok()
    }

    /// Finds a string by the key of its source text.
    ///
    /// A binary search over the sorted key column. Returns the record's index as
    /// well as the record, because the index is what the layout and constraint
    /// tables are keyed by.
    #[must_use]
    pub fn jid(&self, miftah: u64) -> Option<(u32, &SijillNass)> {
        let fahras = self
            .sijillat
            .binary_search_by(|sijill| sijill.miftah.cmp(&miftah))
            .ok()?;
        let sijill = self.sijillat.get(fahras)?;
        Some((u32::try_from(fahras).ok()?, sijill))
    }

    /// The translated text for a source key, resolved in one call.
    #[must_use]
    pub fn tarjama(&self, miftah: u64) -> Option<&str> {
        let (_, sijill) = self.jid(miftah)?;
        self.nass(sijill.nass)
    }

    /// The style spans belonging to one string, by index.
    #[must_use]
    pub fn nitaqat_li(&self, fahras: u32) -> &[SijillNitaq] {
        nitaqat(self.nitaqat, fahras)
    }
}

/// The key of a source string, as the string table stores it.
///
/// The first eight bytes of BLAKE3 over the source text's UTF-8, read
/// little-endian. Defined here rather than left to each consumer because five
/// languages compute it and a key computed two ways is a patch that half the
/// consumers cannot search.
#[must_use]
pub fn miftah_min_nass(nass: &str) -> u64 {
    let basma = blake3::hash(nass.as_bytes());
    let awwal: [u8; 8] = basma
        .as_bytes()
        .first_chunk::<8>()
        .copied()
        .unwrap_or([0; 8]);
    u64::from_le_bytes(awwal)
}

/// Reads the string section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, including field `tarteeb` when
/// the strings are not sorted by key or the spans are not sorted by string
/// index. Both arrays are searched, and an array that is searched has to be
/// sorted; checking it once at load is cheaper than every lookup silently
/// returning the wrong row.
pub fn nusus(bayt: &[u8]) -> Result<JadwalNusus<'_>, KhataRuqaa> {
    let naw = NawQism::Nusus;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatNusus = iqra_tasdir(naw, bayt, HAJM_TASDIR_KABIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let sijillat: &[SijillNass] = qass_masfufa(
        naw,
        bayt,
        "izahat_nusus",
        tasdir.izahat_nusus,
        tasdir.adad_nusus,
    )?;
    let nitaqat: &[SijillNitaq] = qass_masfufa(
        naw,
        bayt,
        "izahat_nitaqat",
        tasdir.izahat_nitaqat,
        tasdir.adad_nitaqat,
    )?;
    let hawd = qass_bayt(
        naw,
        bayt,
        "izahat_hawd",
        tasdir.izahat_hawd,
        tasdir.tul_hawd,
    )?;

    tahaqquq_tarteeb(naw, sijillat.len(), |i| sijillat.get(i).map(|s| s.miftah))?;
    tahaqquq_tarteeb_ghayr_hasir(naw, nitaqat.len(), |i| nitaqat.get(i).map(|n| n.nass))?;

    Ok(JadwalNusus {
        tasdir,
        sijillat,
        nitaqat,
        hawd,
    })
}

/// The three arrays of the precomputed layout section.
///
/// Heads, glyphs and lines, all cast in place. A head names a run in each of the
/// other two by index; [`takhtit_wahid`] resolves one.
#[derive(Debug)]
pub struct JadwalTakhtit<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatTakhtit,
    /// One head per precomputed layout, sorted by `(nass, hajm_rubi)`.
    pub ruus: &'a [SijillTakhtit],
    /// Every glyph of every layout, in head order.
    pub huruf: &'a [SijillHarf],
    /// Every line of every layout, in head order.
    pub sutur: &'a [SijillSatr],
}

impl JadwalTakhtit<'_> {
    /// How many layouts the section holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.ruus.len()
    }

    /// The layout for one string at one size, if the compiler produced it.
    ///
    /// The exact size is required, not the nearest: drawing a layout measured at
    /// one size into a box the game sizes at another is how text that fitted in
    /// the compiler's measurement overflows on a player's screen.
    #[must_use]
    pub fn jid(&self, nass: u32, hajm_rubi: u16) -> Option<&SijillTakhtit> {
        let matlub = (nass, hajm_rubi);
        let fahras = self
            .ruus
            .binary_search_by(|ras| ras.miftah().cmp(&matlub))
            .ok()?;
        self.ruus.get(fahras)
    }

    /// Every layout the compiler produced for one string, at whatever sizes.
    ///
    /// A contiguous run, because the array is sorted by string first. What an
    /// adapter uses when it is willing to accept a nearby size — deciding which
    /// is its business, not this module's.
    #[must_use]
    pub fn li_nass(&self, nass: u32) -> &[SijillTakhtit] {
        let awwal = self.ruus.partition_point(|ras| ras.nass < nass);
        let akhir = self.ruus.partition_point(|ras| ras.nass <= nass);
        self.ruus.get(awwal..akhir).unwrap_or(&[])
    }

    /// The glyphs and lines of one layout.
    ///
    /// Returns [`None`] when the head's runs point outside the arrays, which is
    /// a corrupt patch rather than a missing layout.
    #[must_use]
    pub fn muhtawa(&self, ras: &SijillTakhtit) -> Option<(&[SijillHarf], &[SijillSatr])> {
        takhtit_wahid(ras, self.huruf, self.sutur)
    }
}

/// Reads the precomputed layout section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, including field `tarteeb` when
/// the heads are not in the sorted order [`JadwalTakhtit::jid`] searches on, and
/// [`KhataRuqaa::MuhadhahaGhayrSaliha`] when one of the arrays does not sit at
/// an address its record type may be read from.
pub fn takhtit(bayt: &[u8]) -> Result<JadwalTakhtit<'_>, KhataRuqaa> {
    let naw = NawQism::Takhtit;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatTakhtit = iqra_tasdir(naw, bayt, HAJM_TASDIR_KABIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let ruus: &[SijillTakhtit] = qass_masfufa(
        naw,
        bayt,
        "izahat_takhtitat",
        tasdir.izahat_takhtitat,
        tasdir.adad_takhtitat,
    )?;
    let huruf: &[SijillHarf] = qass_masfufa(
        naw,
        bayt,
        "izahat_huruf",
        tasdir.izahat_huruf,
        tasdir.adad_huruf,
    )?;
    let sutur: &[SijillSatr] = qass_masfufa(
        naw,
        bayt,
        "izahat_sutur",
        tasdir.izahat_sutur,
        tasdir.adad_sutur,
    )?;

    tahaqquq_tarteeb(naw, ruus.len(), |i| ruus.get(i).map(SijillTakhtit::miftah))?;

    // Every head is checked once, here, so that drawing one is a slice and not
    // a bounds check. An adapter resolving a layout on a frame path should not
    // be re-proving a property of the file it opened.
    let adad_huruf = u32::try_from(huruf.len()).unwrap_or(u32::MAX);
    let adad_sutur = u32::try_from(sutur.len()).unwrap_or(u32::MAX);
    for ras in ruus {
        tahaqquq_nitaq("layout's glyph", ras.awwal_harf, ras.adad_huruf, adad_huruf)?;
        tahaqquq_nitaq("layout's line", ras.awwal_satr, ras.adad_sutur, adad_sutur)?;
    }

    Ok(JadwalTakhtit {
        tasdir,
        ruus,
        huruf,
        sutur,
    })
}

/// The glyph map: keys and positions, parallel.
#[derive(Debug)]
pub struct JadwalKhareeta<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatKhareeta,
    /// The keys, sorted ascending. Searched.
    pub mafatih: &'a [SijillMiftahShakl],
    /// The positions. Position `n` belongs to key `n`.
    pub mawadi: &'a [SijillMawdiShakl],
}

impl JadwalKhareeta<'_> {
    /// How many glyph images the map holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.mafatih.len()
    }

    /// Finds a glyph image.
    ///
    /// A binary search over the packed key column rather than a hash map,
    /// because building a hash map at load time inside somebody's game is
    /// exactly the cost the fixed-layout tables exist to avoid.
    #[must_use]
    pub fn jid(&self, miftah: SijillMiftahShakl) -> Option<&SijillMawdiShakl> {
        let matlub = miftah.raqm();
        let fahras = self
            .mafatih
            .binary_search_by(|mawjud| mawjud.raqm().cmp(&matlub))
            .ok()?;
        self.mawadi.get(fahras)
    }
}

/// Reads the glyph map.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, including field `tarteeb` when
/// the keys are not in the strictly ascending order [`JadwalKhareeta::jid`]
/// searches on. Duplicates are refused as part of that: two records for one
/// font, size, bucket and glyph disagree about where the image is, and there is
/// no way to tell which one the atlas actually holds.
pub fn khareeta(bayt: &[u8]) -> Result<JadwalKhareeta<'_>, KhataRuqaa> {
    let naw = NawQism::Khareeta;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatKhareeta = iqra_tasdir(naw, bayt, HAJM_TASDIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let mafatih: &[SijillMiftahShakl] = qass_masfufa(
        naw,
        bayt,
        "izahat_mafatih",
        tasdir.izahat_mafatih,
        tasdir.adad,
    )?;
    let mawadi: &[SijillMawdiShakl] = qass_masfufa(
        naw,
        bayt,
        "izahat_mawadi",
        tasdir.izahat_mawadi,
        tasdir.adad,
    )?;

    tahaqquq_tarteeb(naw, mafatih.len(), |i| {
        mafatih.get(i).copied().map(SijillMiftahShakl::raqm)
    })?;

    Ok(JadwalKhareeta {
        tasdir,
        mafatih,
        mawadi,
    })
}

/// The constraints and reflow hints.
#[derive(Debug)]
pub struct JadwalQiyud<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatQiyud,
    /// One record per constrained string, sorted ascending by string index.
    pub sijillat: &'a [SijillQayd],
}

impl JadwalQiyud<'_> {
    /// How many constraints the patch records.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.sijillat.len()
    }

    /// The constraint recorded for one string, if there is one.
    #[must_use]
    pub fn jid(&self, nass: u32) -> Option<&SijillQayd> {
        let fahras = self
            .sijillat
            .binary_search_by(|qayd| qayd.nass.cmp(&nass))
            .ok()?;
        self.sijillat.get(fahras)
    }
}

/// Reads the constraints section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, including field `tarteeb` when
/// the records are not sorted by string index.
pub fn qiyud(bayt: &[u8]) -> Result<JadwalQiyud<'_>, KhataRuqaa> {
    let naw = NawQism::Qiyud;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatQiyud = iqra_tasdir(naw, bayt, HAJM_TASDIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let sijillat: &[SijillQayd] = qass_masfufa(naw, bayt, "izaha", tasdir.izaha, tasdir.adad)?;
    tahaqquq_tarteeb(naw, sijillat.len(), |i| {
        sijillat.get(i).map(|qayd| qayd.nass)
    })?;

    Ok(JadwalQiyud { tasdir, sijillat })
}

/// The atlas: its page table, and the section the texels live in.
#[derive(Debug)]
pub struct JadwalLawha<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatLawha,
    /// One descriptor per page.
    pub safahat: &'a [SijillSafha],
    bayt: &'a [u8],
}

impl<'a> JadwalLawha<'a> {
    /// How many pages the atlas holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.safahat.len()
    }

    /// One page's texels: one byte each, row-major from the top, no row padding.
    ///
    /// Returns [`None`] only for an index past the end. Every descriptor's
    /// extent was proven in range when the section was read, so a page that
    /// exists has bytes.
    #[must_use]
    pub fn safha(&self, fahras: usize) -> Option<(&'a SijillSafha, &'a [u8])> {
        let sijill = self.safahat.get(fahras)?;
        let bidaya = usize::try_from(sijill.izaha).ok()?;
        let nihaya = bidaya.checked_add(usize::try_from(sijill.tul).ok()?)?;
        Some((sijill, self.bayt.get(bidaya..nihaya)?))
    }
}

/// Reads the atlas section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, or
/// [`KhataRuqaa::SafhaTalifa`] when a page's texels run past the section or its
/// byte count disagrees with its dimensions.
pub fn lawha(bayt: &[u8]) -> Result<JadwalLawha<'_>, KhataRuqaa> {
    let naw = NawQism::Lawha;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatLawha = iqra_tasdir(naw, bayt, HAJM_TASDIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let safahat: &[SijillSafha] = qass_masfufa(
        naw,
        bayt,
        "izahat_safahat",
        tasdir.izahat_safahat,
        tasdir.adad_safahat,
    )?;

    for (fahras, sijill) in safahat.iter().enumerate() {
        let raqm = u32::try_from(fahras).unwrap_or(u32::MAX);
        let izaha = u64::from(sijill.izaha);
        let nihaya = izaha.saturating_add(u64::from(sijill.tul));
        let talifa = || KhataRuqaa::SafhaTalifa {
            safha: raqm,
            izaha,
            nihaya,
            tul: tul_qism,
        };

        // The byte count must equal the dimensions. A page that disagrees with
        // itself uploads a texture whose rows are shifted against the rectangles
        // the glyph map names, and every glyph then draws a little wrong — which
        // reads as a font bug rather than as a corrupt patch.
        if sijill.tul_madum() != Some(sijill.tul) {
            return Err(talifa());
        }
        if izaha.checked_add(u64::from(sijill.tul)).is_none() || nihaya > tul_qism {
            return Err(talifa());
        }
    }

    Ok(JadwalLawha {
        tasdir,
        safahat,
        bayt,
    })
}

/// The font chain the patch declares.
#[derive(Debug)]
pub struct JadwalKhatt<'a> {
    /// The section's preamble, already checked against the section's length.
    pub tasdir: TarwisatKhatt,
    /// One record per font, in chain order.
    pub sijillat: &'a [SijillKhatt],
    /// The UTF-8 pool the file names point into.
    pub hawd: &'a [u8],
    bayt: &'a [u8],
}

impl JadwalKhatt<'_> {
    /// How many fonts the chain declares.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.sijillat.len()
    }

    /// One font's file name.
    #[must_use]
    pub fn ism(&self, sijill: SijillKhatt) -> Option<&str> {
        let nitaq = sijill.ism.nitaq(self.hawd.len())?;
        core::str::from_utf8(self.hawd.get(nitaq)?).ok()
    }

    /// One font's expected content hash.
    ///
    /// What the adapter checks the file on disk against before shaping with it,
    /// so a font replaced after installation is caught rather than used.
    #[must_use]
    pub fn basma(&self, sijill: SijillKhatt) -> Option<&[u8; 32]> {
        let bidaya = usize::try_from(sijill.izahat_basma).ok()?;
        let nihaya = bidaya.checked_add(32)?;
        self.bayt.get(bidaya..nihaya)?.first_chunk::<32>()
    }
}

/// Reads the font record section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field, or [`KhataRuqaa::FahrasKharij`]
/// when a record's hash offset runs past the section.
pub fn khatt(bayt: &[u8]) -> Result<JadwalKhatt<'_>, KhataRuqaa> {
    let naw = NawQism::Khatt;
    let tul_qism = tul_u64(bayt.len());
    let tasdir: TarwisatKhatt = iqra_tasdir(naw, bayt, HAJM_TASDIR)?;
    tasdir.tahaqquq(tul_qism)?;

    let sijillat: &[SijillKhatt] = qass_masfufa(
        naw,
        bayt,
        "izahat_khutut",
        tasdir.izahat_khutut,
        tasdir.adad_khutut,
    )?;
    let hawd = qass_bayt(
        naw,
        bayt,
        "izahat_hawd",
        tasdir.izahat_hawd,
        tasdir.tul_hawd,
    )?;

    let adad = u32::try_from(sijillat.len()).unwrap_or(u32::MAX);
    for sijill in sijillat {
        let nihaya =
            u64::from(sijill.izahat_basma)
                .checked_add(32)
                .ok_or(KhataRuqaa::FahrasKharij {
                    haql: "font record's content hash",
                    fahras: sijill.izahat_basma,
                    adad,
                })?;
        if nihaya > tul_qism {
            return Err(KhataRuqaa::FahrasKharij {
                haql: "font record's content hash",
                fahras: sijill.izahat_basma,
                adad,
            });
        }
    }

    Ok(JadwalKhatt {
        tasdir,
        sijillat,
        hawd,
        bayt,
    })
}

// ---------------------------------------------------------------------------
// Free helpers a consumer may want without a whole section reader
// ---------------------------------------------------------------------------

/// The glyphs and lines of one precomputed layout.
///
/// Returns [`None`] when the layout's ranges point outside their arrays, which
/// is a corrupt patch rather than a missing layout — the caller raises the error
/// because it knows which string it was drawing.
#[must_use]
pub fn takhtit_wahid<'a>(
    ras: &SijillTakhtit,
    huruf: &'a [SijillHarf],
    sutur: &'a [SijillSatr],
) -> Option<(&'a [SijillHarf], &'a [SijillSatr])> {
    let harf_bidaya = usize::try_from(ras.awwal_harf).ok()?;
    let harf_nihaya = harf_bidaya.checked_add(usize::try_from(ras.adad_huruf).ok()?)?;
    let satr_bidaya = usize::try_from(ras.awwal_satr).ok()?;
    let satr_nihaya = satr_bidaya.checked_add(usize::try_from(ras.adad_sutur).ok()?)?;
    Some((
        huruf.get(harf_bidaya..harf_nihaya)?,
        sutur.get(satr_bidaya..satr_nihaya)?,
    ))
}

/// The style spans belonging to one string.
///
/// Spans are stored sorted by their string index, so one string's spans are a
/// contiguous run and finding them is two partition points rather than a scan.
#[must_use]
pub fn nitaqat(jadwal: &[SijillNitaq], nass: u32) -> &[SijillNitaq] {
    let awwal = jadwal.partition_point(|nitaq| nitaq.nass < nass);
    let akhir = jadwal.partition_point(|nitaq| nitaq.nass <= nass);
    jadwal.get(awwal..akhir).unwrap_or(&[])
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// The bytes of one section, bounds-checked against the whole container.
fn shariha<'a>(bayt: &'a [u8], madkhal: &MadkhalQism) -> Result<&'a [u8], KhataRuqaa> {
    let kharij = |haql: &'static str, qeema: u64| KhataRuqaa::QismKharij {
        naw: madkhal.naw.raqm(),
        haql,
        qeema,
        hadd: tul_u64(bayt.len()),
    };
    let bidaya = hajm_usize(madkhal.izaha).ok_or_else(|| kharij("izaha", madkhal.izaha))?;
    let tul = hajm_usize(madkhal.tul_makhzun).ok_or_else(|| kharij("tul", madkhal.tul_makhzun))?;
    let nihaya = bidaya
        .checked_add(tul)
        .ok_or_else(|| kharij("tul", madkhal.tul_makhzun))?;
    bayt.get(bidaya..nihaya)
        .ok_or_else(|| kharij("izaha", madkhal.izaha))
}

/// A section's preamble, cast in place.
fn iqra_tasdir<T: Pod>(naw: NawQism, bayt: &[u8], hajm: usize) -> Result<T, KhataRuqaa> {
    let khana = bayt.get(..hajm).ok_or_else(|| KhataRuqaa::JadwalTalif {
        naw: naw.raqm(),
        haql: "tasdir",
        qeema: tul_u64(bayt.len()),
        hadd: tul_u64(hajm),
    })?;
    bytemuck::try_from_bytes::<T>(khana)
        .copied()
        .map_err(|_| KhataRuqaa::MuhadhahaGhayrSaliha {
            naw: naw.raqm(),
            haql: "tasdir",
            izaha: 0,
        })
}

/// One array of a section, cast in place.
///
/// The extent was already proven in range by the preamble's own check; what is
/// left is turning it into a slice, and the alignment the cast needs. The bounds
/// are re-derived here anyway rather than carried over, because a slice produced
/// from arithmetic that was checked somewhere else is a slice whose safety
/// depends on two functions staying in agreement.
fn qass_masfufa<'a, T: Pod>(
    naw: NawQism,
    bayt: &'a [u8],
    haql: &'static str,
    izaha: u32,
    adad: u32,
) -> Result<&'a [T], KhataRuqaa> {
    let raqm = naw.raqm();
    let talif = |qeema: u64| KhataRuqaa::JadwalTalif {
        naw: raqm,
        haql,
        qeema,
        hadd: tul_u64(bayt.len()),
    };

    let bidaya = usize::try_from(izaha).map_err(|_| talif(u64::from(izaha)))?;
    let tul = usize::try_from(adad)
        .ok()
        .and_then(|adad| adad.checked_mul(size_of::<T>()))
        .ok_or_else(|| talif(u64::from(adad)))?;
    let nihaya = bidaya
        .checked_add(tul)
        .ok_or_else(|| talif(u64::from(adad)))?;
    let khaam = bayt
        .get(bidaya..nihaya)
        .ok_or_else(|| talif(u64::from(izaha)))?;
    bytemuck::try_cast_slice(khaam).map_err(|_| KhataRuqaa::MuhadhahaGhayrSaliha {
        naw: raqm,
        haql,
        izaha: u64::from(izaha),
    })
}

/// A byte range of a section, bounds-checked.
fn qass_bayt<'a>(
    naw: NawQism,
    bayt: &'a [u8],
    haql: &'static str,
    izaha: u32,
    tul: u32,
) -> Result<&'a [u8], KhataRuqaa> {
    let raqm = naw.raqm();
    let talif = |qeema: u64| KhataRuqaa::JadwalTalif {
        naw: raqm,
        haql,
        qeema,
        hadd: tul_u64(bayt.len()),
    };

    let bidaya = usize::try_from(izaha).map_err(|_| talif(u64::from(izaha)))?;
    let mada = usize::try_from(tul).map_err(|_| talif(u64::from(tul)))?;
    let nihaya = bidaya
        .checked_add(mada)
        .ok_or_else(|| talif(u64::from(tul)))?;
    bayt.get(bidaya..nihaya)
        .ok_or_else(|| talif(u64::from(izaha)))
}

/// Refuses an array that is not strictly ascending on its sort key.
///
/// Strictly, so duplicates are refused too: the arrays this runs over are all
/// searched by a key that is supposed to identify one row, and two rows claiming
/// one key is a container where the answer to a lookup depends on which one the
/// search happened to land on.
fn tahaqquq_tarteeb<K: Ord>(
    naw: NawQism,
    adad: usize,
    miftah: impl Fn(usize) -> Option<K>,
) -> Result<(), KhataRuqaa> {
    let mut sabiq: Option<K> = None;
    for fahras in 0..adad {
        let Some(hali) = miftah(fahras) else { continue };
        if sabiq.as_ref().is_some_and(|qabl| *qabl >= hali) {
            return Err(KhataRuqaa::JadwalTalif {
                naw: naw.raqm(),
                haql: "tarteeb",
                qeema: tul_u64(fahras),
                hadd: tul_u64(adad),
            });
        }
        sabiq = Some(hali);
    }
    Ok(())
}

/// Refuses an array that is not ascending, allowing equal neighbours.
///
/// For the span array, where many rows legitimately share one string index and
/// the ordering exists so that the run can be found by partition point.
fn tahaqquq_tarteeb_ghayr_hasir<K: Ord>(
    naw: NawQism,
    adad: usize,
    miftah: impl Fn(usize) -> Option<K>,
) -> Result<(), KhataRuqaa> {
    let mut sabiq: Option<K> = None;
    for fahras in 0..adad {
        let Some(hali) = miftah(fahras) else { continue };
        if sabiq.as_ref().is_some_and(|qabl| *qabl > hali) {
            return Err(KhataRuqaa::JadwalTalif {
                naw: naw.raqm(),
                haql: "tarteeb",
                qeema: tul_u64(fahras),
                hadd: tul_u64(adad),
            });
        }
        sabiq = Some(hali);
    }
    Ok(())
}

/// Refuses a run that is not entirely inside the array it indexes.
fn tahaqquq_nitaq(haql: &'static str, awwal: u32, adad: u32, kull: u32) -> Result<(), KhataRuqaa> {
    let nihaya = awwal.checked_add(adad).ok_or(KhataRuqaa::FahrasKharij {
        haql,
        fahras: awwal,
        adad: kull,
    })?;
    if nihaya > kull {
        return Err(KhataRuqaa::FahrasKharij {
            haql,
            fahras: nihaya,
            adad: kull,
        });
    }
    Ok(())
}

/// Checks the content hash over the bytes it actually covers.
///
/// From the end of the header to the start of the signature block — not to the
/// end of the file. Hashing to the end would make the hash depend on the
/// signature, which is signed over the hash, and sealing a package would be
/// impossible without recomputing what it committed to.
fn tahaqquq_basma(bayt: &[u8], tarwisa: &Tarwisa, jadwal: &JadwalAqsam) -> Result<(), KhataRuqaa> {
    let nihaya = hajm_usize(jadwal.nihayat_muhtawa()).ok_or_else(|| KhataRuqaa::MalafQaseer {
        haql: "nihayat_muhtawa",
        tul: tul_u64(bayt.len()),
        matlub: jadwal.nihayat_muhtawa(),
    })?;
    let jism = bayt
        .get(HAJM_TARWISA..nihaya)
        .ok_or_else(|| KhataRuqaa::MalafQaseer {
            haql: "muhtawa",
            tul: tul_u64(bayt.len()),
            matlub: jadwal.nihayat_muhtawa(),
        })?;
    let mahsuba = blake3::hash(jism);
    if mahsuba.as_bytes() == &tarwisa.basma {
        Ok(())
    } else {
        Err(KhataRuqaa::BasmaGhayrMutabaqa {
            muallana: sittasi(&tarwisa.basma),
            mahsuba: sittasi(mahsuba.as_bytes()),
        })
    }
}

/// Reads the signature block out of its section.
fn iqra_tawqee(bayt: &[u8], jadwal: &JadwalAqsam) -> Result<KutlatTawqee, KhataRuqaa> {
    let madkhal = jadwal.qism(NawQism::Tawqee).ok_or(KhataRuqaa::QismMafqud {
        ism: NawQism::Tawqee.ism(),
    })?;
    let kutla = shariha(bayt, &madkhal)?;
    KutlatTawqee::min_bayt(kutla)
}

/// Decompresses one section, refusing anything that does not match what it
/// declared.
fn fukk(naw: NawQism, makhzun: &[u8], tul_khaam: u64) -> Result<BaytMuhadhah, KhataRuqaa> {
    let mufrit = KhataRuqaa::HajmKhaamMufrit {
        naw: naw.raqm(),
        muallan: tul_khaam,
        saqf: AQSA_QISM_KHAAM,
    };
    if tul_khaam > AQSA_QISM_KHAAM {
        return Err(mufrit);
    }
    let siaa = hajm_usize(tul_khaam).ok_or(mufrit)?;

    // Decompressing into a buffer of exactly the declared size, rather than into
    // a growing vector, is what bounds the work. A zstd frame is free to claim
    // one size in its header and expand to another; a fixed destination makes
    // the encoder's claim the ceiling instead of a suggestion, and the length
    // check below turns a frame that expands to less than it promised into a
    // refusal rather than a table read over uninitialised zeros.
    let mut mafkuk = BaytMuhadhah::sifr(siaa);
    let fili = zstd::bulk::decompress_to_buffer(makhzun, mafkuk.bayt_mut()).map_err(|khata| {
        KhataRuqaa::FakkFashil {
            naw: naw.raqm(),
            tafsil: khata.to_string(),
        }
    })?;

    if tul_u64(fili) == tul_khaam {
        Ok(mafkuk)
    } else {
        Err(KhataRuqaa::HajmKhaamGhayrMutabaq {
            naw: naw.raqm(),
            muallan: tul_khaam,
            fili: tul_u64(fili),
        })
    }
}
