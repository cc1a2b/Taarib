//! الأقسام — the eight section kinds, how each one is stored, and the
//! thirty-two byte entry that describes it.
//!
//! The section table is the only part of a `.ruqaa` that is parsed *before* the
//! content hash is verified, because the hash's own end boundary is the offset
//! of the signature section and there is no way to learn that without reading
//! the table. Everything in this module is therefore written to be safe on
//! bytes nobody has vouched for: every field is a fixed-width integer at a fixed
//! offset, every read is bounds-checked against the slice it comes from, every
//! arithmetic combination is checked, and nothing here allocates or dereferences
//! anything the table points at.
//!
//! ## The entry, byte for byte
//!
//! ```text
//! MadkhalQism — 32 bytes, little-endian
//!
//!   offset  size  field          type  meaning
//!        0     4  naw            u32   section kind, 1..=8
//!        4     8  izaha          u64   absolute byte offset of the stored bytes
//!       12     8  tul_makhzun    u64   stored length, compressed or not
//!       20     8  tul_khaam      u64   length after decompression
//!       28     4  daght          u32   0 = stored as is, 1 = zstd
//! ```
//!
//! Note the offsets. `izaha` sits at 4 and `tul_makhzun` at 12, so neither
//! eight-byte field is eight-byte aligned. That is what the ROADMAP specifies —
//! the five fields sum to exactly thirty-two bytes with no padding anywhere —
//! and it is the reason this structure is read field by field rather than
//! overlaid: an overlay would require either `repr(packed)`, which produces
//! unaligned references, or inserted padding, which would change the layout four
//! other languages are written against. Only the four tables the ROADMAP names
//! as POD arrays are overlaid; the framing is parsed.

use crate::khata::KhataRuqaa;
use crate::tarwisa::{iqra_u32, iqra_u64, tul_u64, uktub_u32, uktub_u64};

/// How many bytes one section table entry occupies.
pub const HAJM_MADKHAL: usize = 32;

/// Byte offset of `naw` within an entry.
pub const IZAHAT_NAW: usize = 0;
/// Byte offset of `izaha` within an entry.
pub const IZAHAT_IZAHA: usize = 4;
/// Byte offset of `tul_makhzun` within an entry.
pub const IZAHAT_TUL_MAKHZUN: usize = 12;
/// Byte offset of `tul_khaam` within an entry.
pub const IZAHAT_TUL_KHAAM: usize = 20;
/// Byte offset of `daght` within an entry.
pub const IZAHAT_DAGHT: usize = 28;

/// The boundary every section starts on.
///
/// Sixteen, so that a section mapped from a page-aligned file is aligned well
/// enough for every POD record the format defines, on every architecture the
/// product targets — including the aarch64 targets where an unaligned load is
/// not merely slower but, through a C# `struct` overlay, undefined.
pub const MUHADHAT_QISM: u64 = 16;

/// What a section holds.
///
/// The numbers are the format's, permanently. A kind that has shipped is never
/// reused for something else, for the same reason an error code is not: four
/// other language bindings switch on these values, and they are not all
/// recompiled at the same time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum NawQism {
    /// Metadata, as a JSON record. What the patch claims about itself: its
    /// identity, its contributor, its licence, its coverage, the builds it
    /// binds to, the fonts it rasterized from, its overflow report, and the
    /// asset gate's attestation.
    Bayan = 1,
    /// The string table: the source text, the Arabic text, and the indices that
    /// join each string to its layouts and its constraints.
    Nusus = 2,
    /// The precomputed layouts: one record per string per size, plus the lines
    /// and the positioned glyphs they name.
    Takhtit = 3,
    /// The atlas pages, as raw single-channel texels with no framing at all.
    /// Where each page begins is declared by [`NawQism::Khareeta`].
    Lawha = 4,
    /// The glyph map: the sorted key-to-rectangle table, and the descriptor for
    /// every page in [`NawQism::Lawha`].
    Khareeta = 5,
    /// The embedded font subset record.
    Khatt = 6,
    /// Per-string constraints and reflow hints.
    Qiyud = 7,
    /// The signature block. Never compressed, always last.
    Tawqee = 8,
}

impl NawQism {
    /// Every kind, in the order the format numbers them.
    pub const KULL: [Self; 8] = [
        Self::Bayan,
        Self::Nusus,
        Self::Takhtit,
        Self::Lawha,
        Self::Khareeta,
        Self::Khatt,
        Self::Qiyud,
        Self::Tawqee,
    ];

    /// The number written into the section table.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        self as u32
    }

    /// Reads a kind from the section table.
    ///
    /// # Errors
    ///
    /// Returns [`KhataRuqaa::NawQismMajhul`] for anything outside 1..=8.
    /// Unlike the discriminants that cross the C ABI, an unrecognised section
    /// kind is a refusal rather than a default: a section this build cannot
    /// name is a section whose bytes it cannot bound the meaning of, and
    /// skipping it silently would let a container carry a payload that no
    /// reader ever accounts for while still hashing and verifying cleanly.
    pub const fn min_raqm(raqm: u32) -> Result<Self, KhataRuqaa> {
        match raqm {
            1 => Ok(Self::Bayan),
            2 => Ok(Self::Nusus),
            3 => Ok(Self::Takhtit),
            4 => Ok(Self::Lawha),
            5 => Ok(Self::Khareeta),
            6 => Ok(Self::Khatt),
            7 => Ok(Self::Qiyud),
            8 => Ok(Self::Tawqee),
            _ => Err(KhataRuqaa::NawQismMajhul { naw: raqm }),
        }
    }

    /// The kind's name, as it appears in a refusal and in a diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Bayan => "BAYAN",
            Self::Nusus => "NUSUS",
            Self::Takhtit => "TAKHTIT",
            Self::Lawha => "LAWHA",
            Self::Khareeta => "KHAREETA",
            Self::Khatt => "KHATT",
            Self::Qiyud => "QIYUD",
            Self::Tawqee => "TAWQEE",
        }
    }

    /// What the kind holds, in a sentence, for the Diagnostics screen.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Bayan => "بيانات الرقعة الوصفية",
            Self::Nusus => "جدول النصوص",
            Self::Takhtit => "التخطيطات المحسوبة مسبقًا",
            Self::Lawha => "صفحات اللوحة",
            Self::Khareeta => "خريطة الأشكال",
            Self::Khatt => "سجل الخط المضمَّن",
            Self::Qiyud => "قيود النصوص وتلميحات إعادة التدفق",
            Self::Tawqee => "كتلة التوقيع",
        }
    }

    /// Whether this section decompresses into fixed-layout POD arrays that a
    /// consumer overlays rather than parses.
    ///
    /// True for exactly the four the ROADMAP names. [`NawQism::Bayan`] is JSON,
    /// [`NawQism::Lawha`] is raw texels, [`NawQism::Khatt`] is an opaque record,
    /// and [`NawQism::Tawqee`] is a single fixed block.
    #[must_use]
    pub const fn jadwal_pod(self) -> bool {
        matches!(
            self,
            Self::Nusus | Self::Takhtit | Self::Khareeta | Self::Qiyud
        )
    }

    /// Whether the format forbids this section from being compressed.
    ///
    /// Only the signature block, and for a reason that is the whole point of
    /// its placement: a verifier must reach it without expanding anything it
    /// has not yet decided to trust.
    #[must_use]
    pub const fn bila_daght(self) -> bool {
        matches!(self, Self::Tawqee)
    }

    /// Whether a container is invalid without this section.
    ///
    /// [`NawQism::Bayan`] because a patch that says nothing about itself cannot
    /// be listed, gated, or reviewed. [`NawQism::Tawqee`] because the block is
    /// written as a fixed-size reservation by the compiler and filled in place
    /// by `taarib-khatm`; if it were optional, sealing a patch would have to
    /// append a section, which would move every offset the content hash covers.
    #[must_use]
    pub const fn ilzami(self) -> bool {
        matches!(self, Self::Bayan | Self::Tawqee)
    }
}

/// How a section's bytes are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u32)]
pub enum NawDaght {
    /// Stored exactly as it will be read. `tul_khaam` equals `tul_makhzun`.
    #[default]
    Bila = 0,
    /// A single zstd frame covering the whole section.
    Zstd = 1,
}

impl NawDaght {
    /// The number written into the section table.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        self as u32
    }

    /// Reads the compression discriminant.
    ///
    /// # Errors
    ///
    /// Returns [`KhataRuqaa::DaghtMajhul`] for anything else. A default here
    /// would mean treating a frame this build cannot decode as though it were
    /// already plain bytes, and handing those bytes to a struct overlay.
    pub const fn min_raqm(raqm: u32, naw: u32) -> Result<Self, KhataRuqaa> {
        match raqm {
            0 => Ok(Self::Bila),
            1 => Ok(Self::Zstd),
            _ => Err(KhataRuqaa::DaghtMajhul { naw, daght: raqm }),
        }
    }
}

/// One section table entry, parsed.
///
/// Holding the kind and the compression as enumerations rather than as raw
/// numbers is deliberate: by the time a value of this type exists, both
/// discriminants have already been checked against the sets the format defines,
/// so no later code has to re-decide what an unrecognised number means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadkhalQism {
    /// What the section holds.
    pub naw: NawQism,
    /// Where the stored bytes begin, from the start of the file.
    pub izaha: u64,
    /// How many bytes are stored, compressed or not.
    pub tul_makhzun: u64,
    /// How many bytes the section becomes once decompressed.
    pub tul_khaam: u64,
    /// How the bytes are stored.
    pub daght: NawDaght,
}

impl MadkhalQism {
    /// Reads one entry out of a thirty-two byte window.
    ///
    /// Validates only what an entry can be judged on in isolation: that the
    /// kind and the compression are values this format defines, and that an
    /// uncompressed section does not claim to expand. Everything that needs the
    /// rest of the container — bounds against `total_size`, alignment, overlap,
    /// ordering, the ceilings — is checked by
    /// [`JadwalAqsam`](crate::tarwisa::JadwalAqsam), which can see all of it.
    ///
    /// # Errors
    ///
    /// Returns [`KhataRuqaa::MalafQaseer`] when the window is short of
    /// [`HAJM_MADKHAL`] bytes, [`KhataRuqaa::NawQismMajhul`] for an unknown
    /// kind, [`KhataRuqaa::DaghtMajhul`] for an unknown compression, and
    /// [`KhataRuqaa::HajmKhaamGhayrMutabaq`] when a section stored without
    /// compression declares two different lengths.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataRuqaa> {
        let qaseer = |haql: &'static str| KhataRuqaa::MalafQaseer {
            haql,
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(HAJM_MADKHAL),
        };
        let naw_raqm = iqra_u32(bayt, IZAHAT_NAW).ok_or_else(|| qaseer("section kind"))?;
        let naw = NawQism::min_raqm(naw_raqm)?;
        let izaha = iqra_u64(bayt, IZAHAT_IZAHA).ok_or_else(|| qaseer("section offset"))?;
        let tul_makhzun =
            iqra_u64(bayt, IZAHAT_TUL_MAKHZUN).ok_or_else(|| qaseer("stored length"))?;
        let tul_khaam =
            iqra_u64(bayt, IZAHAT_TUL_KHAAM).ok_or_else(|| qaseer("uncompressed length"))?;
        let daght_raqm = iqra_u32(bayt, IZAHAT_DAGHT).ok_or_else(|| qaseer("compression"))?;
        let daght = NawDaght::min_raqm(daght_raqm, naw_raqm)?;
        if daght == NawDaght::Bila && tul_khaam != tul_makhzun {
            return Err(KhataRuqaa::HajmKhaamGhayrMutabaq {
                naw: naw_raqm,
                muallan: tul_khaam,
                fili: tul_makhzun,
            });
        }
        Ok(Self {
            naw,
            izaha,
            tul_makhzun,
            tul_khaam,
            daght,
        })
    }

    /// Writes the entry into a thirty-two byte window.
    ///
    /// # Errors
    ///
    /// Returns [`KhataRuqaa::MalafQaseer`] when the window is short.
    pub fn ila_bayt(&self, bayt: &mut [u8]) -> Result<(), KhataRuqaa> {
        if bayt.len() < HAJM_MADKHAL {
            return Err(KhataRuqaa::MalafQaseer {
                haql: "a section table entry",
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(HAJM_MADKHAL),
            });
        }
        uktub_u32(bayt, IZAHAT_NAW, self.naw.raqm());
        uktub_u64(bayt, IZAHAT_IZAHA, self.izaha);
        uktub_u64(bayt, IZAHAT_TUL_MAKHZUN, self.tul_makhzun);
        uktub_u64(bayt, IZAHAT_TUL_KHAAM, self.tul_khaam);
        uktub_u32(bayt, IZAHAT_DAGHT, self.daght.raqm());
        Ok(())
    }

    /// One past the last stored byte, or [`None`] on overflow.
    ///
    /// Returned as an option rather than saturating, because a section table
    /// whose offset and length overflow a `u64` when added is a section table
    /// that was written to make exactly this arithmetic wrap.
    #[must_use]
    pub const fn nihaya(&self) -> Option<u64> {
        self.izaha.checked_add(self.tul_makhzun)
    }

    /// Whether the stored bytes begin on the boundary the format requires.
    #[must_use]
    pub const fn muhadhah(&self) -> bool {
        self.izaha.is_multiple_of(MUHADHAT_QISM)
    }
}
