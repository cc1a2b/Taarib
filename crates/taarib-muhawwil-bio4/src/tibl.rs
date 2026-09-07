//! التبل — the GameCube TPL container, as `BIO4` embeds it.
//!
//! ## What was read out of the shipped bytes
//!
//! Every claim below was checked against the thirty-two `.fnt` files in
//! `BIO4/Font` of *Resident Evil 4* (2005, Ultimate HD Edition) and against the
//! 616 standalone `.tpl` files elsewhere in the same install. Nothing here is
//! from a specification; the specification agrees, which is why the field names
//! are Nintendo's.
//!
//! The block begins with `0x1234_5678`. Inside a `.fnt` that word is stored
//! **little-endian** — the bytes are `78 56 34 12` — and every standalone `.tpl`
//! in the same game stores the same constant as the byte sequence
//! `12 34 56 78`. They are not the same four bytes, and a reader that accepted
//! either would accept a file whose remaining fields it is about to misread. So
//! [`Tibl::min_bayt`] takes the byte order it is to read in, the `.fnt` path
//! passes [`TarteebBayt::Saghir`], and a mismatch is refused by name.
//!
//! Every other field is little-endian in both, which is the porting artefact it
//! looks like: Capcom's PC build byte-swapped the header fields and left the
//! magic as a literal byte array in one writer and as a `u32` in the other.
//!
//! ## The layout, and why offsets are kept rather than assumed
//!
//! ```text
//! +0x00  u32  magic            0x1234_5678
//! +0x04  u32  descriptor count
//! +0x08  u32  descriptor table offset
//!        ...
//! descriptor (8 bytes, `descriptor count` of them):
//!        u32  image header offset
//!        u32  palette header offset, or 0 for none
//! palette header (0x10 bytes):
//!        u16  entries      u8 unpacked   u8 pad
//!        u32  format       u32 data offset
//! image header (0x24 bytes):
//!        u16  height       u16 width
//!        u32  format       u32 data offset
//!        u32  wrap s       u32 wrap t
//!        u32  min filter   u32 mag filter
//!        f32  lod bias
//!        u8   edge lod     u8 min lod     u8 max lod     u8 unpacked
//! ```
//!
//! In a `.fnt` the descriptor sits at `+0x0C`, its palette header at `+0x14` and
//! its image header at `+0x20`; in a standalone `.tpl` the image header sits at
//! `+0x14` and there is no palette. Both are reproduced by writing each piece
//! back at the offset it was read from and zero-filling the gaps, which is what
//! [`Tibl::ila_bayt`] does — so a round trip is byte-identical for both layouts
//! without this module having to decide which one it is looking at.
//!
//! ## There is no texel data in any of these files
//!
//! The image header's data offset in every shipped `.fnt` is `0x44`, which is the
//! first byte **past** the image header — the block's own end. The 616
//! standalone `.tpl` files do the same thing at `0x38`, and every one of them is
//! 64, 116 or 168 bytes long. Not one carries a texel.
//!
//! The texels live in `BIO4/ImagePack/<id>.pack`, and the id is the four bytes at
//! the data offset, written as eight hexadecimal digits in **reverse byte order**
//! — `09 00 00 0f` names `0f000009.pack`. All thirty-two fonts resolve to a file
//! that exists. See [`crate::hizma`].

use crate::khata::{KhataBio4, tul_u64};

/// The word every TPL block starts with.
pub const SIHR: u32 = 0x1234_5678;

/// The most descriptors this build will read out of one block.
///
/// The largest standalone `.tpl` in *Resident Evil 4* is 168 bytes and holds
/// three. Sixty-four is far past anything the format is used for and low enough
/// that a corrupt count is refused rather than allocated for.
pub const AQSA_WASF: u32 = 64;

/// The largest TPL block this build will read.
///
/// A megabyte. The blocks that exist are tens of bytes; the ceiling is here so a
/// declared offset cannot make this module reserve memory proportional to a
/// number a file claimed.
pub const AQSA_KUTLA: u64 = 1 << 20;

/// Bytes of one descriptor.
const TUL_WASF: u32 = 8;

/// Bytes of one palette header.
const TUL_TARWIS_LAWHA: u32 = 0x10;

/// Bytes of one image header.
const TUL_TARWIS_SURA: u32 = 0x24;

/// Bytes of the block header.
const TUL_TARWIS_KUTLA: u32 = 0x0C;

/// Which end of a word this container puts first.
///
/// Exists because *one game* stores the same constant both ways in two file
/// types that are otherwise the same format. See the module header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TarteebBayt {
    /// Least significant byte first: the `.fnt` path.
    Saghir,
    /// Most significant byte first: the standalone `.tpl` path.
    Kabir,
}

impl TarteebBayt {
    /// Reads a `u32` in this order.
    #[must_use]
    const fn kalima(self, bayt: [u8; 4]) -> u32 {
        match self {
            Self::Saghir => u32::from_le_bytes(bayt),
            Self::Kabir => u32::from_be_bytes(bayt),
        }
    }

    /// Writes a `u32` in this order.
    #[must_use]
    const fn bayt_kalima(self, qeema: u32) -> [u8; 4] {
        match self {
            Self::Saghir => qeema.to_le_bytes(),
            Self::Kabir => qeema.to_be_bytes(),
        }
    }
}

/// A GameCube texel format, by its `GX_TF_*` enumerant.
///
/// Named rather than numbered so that a refusal can say `CMPR` instead of `14`.
/// Only [`SighatSura::C4`] is encoded and decoded by this build, because it is
/// the only one any `BIO4/Font` file declares; the rest exist so a reader can
/// report what it found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SighatSura {
    /// Four-bit intensity.
    I4,
    /// Eight-bit intensity.
    I8,
    /// Four-bit intensity with four-bit alpha.
    Ia4,
    /// Eight-bit intensity with eight-bit alpha.
    Ia8,
    /// Five-six-five colour.
    Rgb565,
    /// Five-bit colour or four-bit colour with three-bit alpha, per texel.
    Rgb5a3,
    /// Thirty-two-bit colour.
    Rgba32,
    /// Four-bit palette indices. **The one `BIO4/Font` uses.**
    C4,
    /// Eight-bit palette indices.
    C8,
    /// Fourteen-bit palette indices in sixteen-bit texels.
    C14x2,
    /// Block-compressed colour, the GameCube's S3TC variant.
    Cmpr,
}

impl SighatSura {
    /// The format for an enumerant, or [`None`] when it is not one.
    #[must_use]
    pub const fn min_raqm(raqm: u32) -> Option<Self> {
        match raqm {
            0 => Some(Self::I4),
            1 => Some(Self::I8),
            2 => Some(Self::Ia4),
            3 => Some(Self::Ia8),
            4 => Some(Self::Rgb565),
            5 => Some(Self::Rgb5a3),
            6 => Some(Self::Rgba32),
            8 => Some(Self::C4),
            9 => Some(Self::C8),
            10 => Some(Self::C14x2),
            14 => Some(Self::Cmpr),
            _ => None,
        }
    }

    /// The enumerant.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        match self {
            Self::I4 => 0,
            Self::I8 => 1,
            Self::Ia4 => 2,
            Self::Ia8 => 3,
            Self::Rgb565 => 4,
            Self::Rgb5a3 => 5,
            Self::Rgba32 => 6,
            Self::C4 => 8,
            Self::C8 => 9,
            Self::C14x2 => 10,
            Self::Cmpr => 14,
        }
    }

    /// The `GX_TF_*` name, for a message a reader has to act on.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::I4 => "GX_TF_I4",
            Self::I8 => "GX_TF_I8",
            Self::Ia4 => "GX_TF_IA4",
            Self::Ia8 => "GX_TF_IA8",
            Self::Rgb565 => "GX_TF_RGB565",
            Self::Rgb5a3 => "GX_TF_RGB5A3",
            Self::Rgba32 => "GX_TF_RGBA32",
            Self::C4 => "GX_TF_C4",
            Self::C8 => "GX_TF_C8",
            Self::C14x2 => "GX_TF_C14X2",
            Self::Cmpr => "GX_TF_CMPR",
        }
    }

    /// The name for an enumerant, `"unknown"` when there is none.
    #[must_use]
    pub const fn ism_raqm(raqm: u32) -> &'static str {
        match Self::min_raqm(raqm) {
            Some(sigha) => sigha.ism(),
            None => "unknown",
        }
    }

    /// The texel block this format is tiled in, as `(width, height)`.
    ///
    /// GameCube textures are stored block by block, never scanline by scanline,
    /// and the block differs per format. A decoder that used one block size for
    /// all of them would produce an image that is a shuffled version of the real
    /// one — recognisable, which is what makes the mistake survive review.
    #[must_use]
    #[expect(
        clippy::match_same_arms,
        reason = "CMPR's 8x8 block is a compressed block and C4's is a texel block; merging the \
                  two arms would say they are one thing, and a future format change to either \
                  would then silently move the other"
    )]
    pub const fn kutla(self) -> (u32, u32) {
        match self {
            Self::I4 | Self::C4 => (8, 8),
            Self::I8 | Self::Ia4 | Self::C8 => (8, 4),
            Self::Ia8 | Self::Rgb565 | Self::Rgb5a3 | Self::Rgba32 | Self::C14x2 => (4, 4),
            Self::Cmpr => (8, 8),
        }
    }

    /// Bits per texel.
    #[must_use]
    pub const fn bitat(self) -> u32 {
        match self {
            Self::I4 | Self::C4 | Self::Cmpr => 4,
            Self::I8 | Self::Ia4 | Self::C8 => 8,
            Self::Ia8 | Self::Rgb565 | Self::Rgb5a3 | Self::C14x2 => 16,
            Self::Rgba32 => 32,
        }
    }

    /// How many bytes a `ard` by `irtifa` image of this format occupies, with
    /// both dimensions rounded up to whole blocks.
    ///
    /// [`None`] on overflow, which a caller reaching it should treat as a refusal
    /// rather than as a size.
    #[must_use]
    pub fn hajm(self, ard: u32, irtifa: u32) -> Option<u64> {
        let (kutla_ard, kutla_irtifa) = self.kutla();
        let sufuf_s = u64::from(kutal(ard, kutla_ard)?);
        let sufuf_a = u64::from(kutal(irtifa, kutla_irtifa)?);
        let texel = u64::from(kutla_ard)
            .checked_mul(u64::from(kutla_irtifa))?
            .checked_mul(u64::from(self.bitat()))?;
        let bayt_kutla = texel.checked_div(8)?;
        sufuf_s.checked_mul(sufuf_a)?.checked_mul(bayt_kutla)
    }
}

/// How many whole blocks of `qiyas` cover `madaa`.
const fn kutal(madaa: u32, qiyas: u32) -> Option<u32> {
    if qiyas == 0 {
        return None;
    }
    match madaa.checked_add(qiyas - 1) {
        Some(mawsu) => mawsu.checked_div(qiyas),
        None => None,
    }
}

/// A GameCube palette format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SighatLawn {
    /// Eight-bit intensity with eight-bit alpha.
    Ia8,
    /// Five-six-five colour, always opaque.
    Rgb565,
    /// Five-bit colour or four-bit colour with three-bit alpha, per entry.
    /// **The one `BIO4/Font` uses.**
    Rgb5a3,
}

impl SighatLawn {
    /// The format for an enumerant, or [`None`] when it is not one.
    #[must_use]
    pub const fn min_raqm(raqm: u32) -> Option<Self> {
        match raqm {
            0 => Some(Self::Ia8),
            1 => Some(Self::Rgb565),
            2 => Some(Self::Rgb5a3),
            _ => None,
        }
    }

    /// The enumerant.
    #[must_use]
    pub const fn raqm(self) -> u32 {
        match self {
            Self::Ia8 => 0,
            Self::Rgb565 => 1,
            Self::Rgb5a3 => 2,
        }
    }

    /// The `GX_TL_*` name.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Ia8 => "GX_TL_IA8",
            Self::Rgb565 => "GX_TL_RGB565",
            Self::Rgb5a3 => "GX_TL_RGB5A3",
        }
    }

    /// The name for an enumerant, `"unknown"` when there is none.
    #[must_use]
    pub const fn ism_raqm(raqm: u32) -> &'static str {
        match Self::min_raqm(raqm) {
            Some(sigha) => sigha.ism(),
            None => "unknown",
        }
    }
}

/// One entry of the descriptor table: where an image's two headers are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WasfTibl {
    /// Block-relative offset of the image header.
    pub mawdi_sura: u32,
    /// Block-relative offset of the palette header, or zero for none.
    pub mawdi_lawha: u32,
}

/// A palette header, field for field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TarwisLawha {
    /// How many entries the palette holds.
    pub adad: u16,
    /// The `unpacked` byte. Zero in every shipped file, and kept because a
    /// round trip that dropped it would not be one.
    pub mufakkak: u8,
    /// The byte after it. Zero in every shipped file, kept for the same reason.
    pub hashw: u8,
    /// The raw palette-format enumerant. Raw rather than [`SighatLawn`] so a
    /// header carrying a value this build does not know still round-trips.
    pub sigha: u32,
    /// Block-relative offset of the palette entries.
    pub mawdi: u32,
}

/// An image header, field for field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TarwisSura {
    /// Height in texels.
    pub irtifa: u16,
    /// Width in texels.
    pub ard: u16,
    /// The raw texel-format enumerant.
    pub sigha: u32,
    /// Block-relative offset of the texels.
    pub mawdi: u32,
    /// `GX` wrap mode on s.
    pub laff_s: u32,
    /// `GX` wrap mode on t.
    pub laff_a: u32,
    /// `GX` minification filter.
    pub murashah_asghar: u32,
    /// `GX` magnification filter.
    pub murashah_akbar: u32,
    /// Level-of-detail bias, as the raw bit pattern.
    ///
    /// Stored as bits and not as an `f32` so that the type can derive `Eq` and
    /// `Hash`, and so that a round trip reproduces a signalling `NaN` or a
    /// negative zero exactly as it was found rather than as whatever an
    /// arithmetic identity turned it into.
    pub inhiyaz_mustawa: u32,
    /// Whether edge level-of-detail is enabled.
    pub hafat_mustawa: u8,
    /// Smallest level of detail.
    pub adna_mustawa: u8,
    /// Largest level of detail.
    pub aqsa_mustawa: u8,
    /// The `unpacked` byte.
    pub mufakkak: u8,
}

impl TarwisSura {
    /// The level-of-detail bias as the number it encodes.
    #[must_use]
    pub const fn inhiyaz(&self) -> f32 {
        f32::from_bits(self.inhiyaz_mustawa)
    }
}

/// A parsed TPL block: its headers, its offsets, and whatever follows them.
///
/// The offsets are kept as they were read, not recomputed, so that
/// [`Tibl::ila_bayt`] can put every piece back where it was and reproduce the
/// input byte for byte. A caller that changes a dimension changes a header
/// field; a caller that changes the *layout* is doing something this format has
/// never been observed to do, and would have to say so by setting the offsets
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tibl {
    /// Which byte order the magic was stored in, and the order it is written
    /// back in.
    pub tarteeb: TarteebBayt,
    /// Block-relative offset of the descriptor table.
    pub mawdi_jadwal: u32,
    /// The descriptor table.
    pub wasf: Vec<WasfTibl>,
    /// Palette headers, by the offset each was read from, ascending.
    pub lawhat: Vec<(u32, TarwisLawha)>,
    /// Image headers, by the offset each was read from, ascending.
    pub suwar: Vec<(u32, TarwisSura)>,
    /// Everything from [`Tibl::mawdi_dhayl`] to the end of the block.
    ///
    /// In every shipped `.fnt` this is the four-byte `ImagePack` identifier
    /// followed by zeros — **not** texels. See the module header.
    pub dhayl: Vec<u8>,
    /// Where the trailer starts, block-relative.
    pub mawdi_dhayl: u32,
}

impl Tibl {
    /// Parses one block out of `bayt`, which must begin at the block's own start.
    ///
    /// `tarteeb` says which way round the magic is; everything else is read
    /// little-endian, which is what both shipped layouts use.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::SihrGhayrMutabaq`] when the magic is not [`SIHR`] in the
    /// order asked for; [`KhataBio4::MalafQaseer`] when a header runs past the
    /// end; [`KhataBio4::HajmMufrit`] when the block or the descriptor count is
    /// above this build's ceiling; and [`KhataBio4::BunyaGhayrMutawaqqaa`] when
    /// an offset is not word-aligned, when the descriptor table overlaps the
    /// block header, or when two headers claim the same bytes with different
    /// contents.
    pub fn min_bayt(bayt: &[u8], tarteeb: TarteebBayt) -> Result<Self, KhataBio4> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_KUTLA {
            return Err(KhataBio4::HajmMufrit {
                haql: "TPL block",
                qeema: tul,
                saqf: AQSA_KUTLA,
            });
        }

        let sihr = tarteeb.kalima(iqra4(bayt, 0, "TPL magic")?);
        if sihr != SIHR {
            return Err(KhataBio4::SihrGhayrMutabaq { wujid: sihr });
        }

        let adad = u32::from_le_bytes(iqra4(bayt, 4, "TPL descriptor count")?);
        if adad > AQSA_WASF {
            return Err(KhataBio4::HajmMufrit {
                haql: "TPL descriptor count",
                qeema: u64::from(adad),
                saqf: u64::from(AQSA_WASF),
            });
        }
        let mawdi_jadwal = u32::from_le_bytes(iqra4(bayt, 8, "TPL descriptor table offset")?);
        if mawdi_jadwal < TUL_TARWIS_KUTLA {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "TPL descriptor table offset",
                qeema: u64::from(mawdi_jadwal),
                sabab: "would overlap the block header",
            });
        }
        hatta_muhadhah(mawdi_jadwal, "TPL descriptor table offset")?;

        let mut wasf = Vec::with_capacity(adad.min(AQSA_WASF).try_into().unwrap_or(0));
        for fahras in 0..adad {
            let asas = mawdi_jadwal
                .checked_add(fahras.saturating_mul(TUL_WASF))
                .ok_or_else(|| KhataBio4::BunyaGhayrMutawaqqaa {
                    haql: "TPL descriptor offset",
                    qeema: u64::from(fahras),
                    sabab: "runs past the address space of a block",
                })?;
            wasf.push(WasfTibl {
                mawdi_sura: u32::from_le_bytes(iqra4_min(bayt, asas, 0, "TPL image offset")?),
                mawdi_lawha: u32::from_le_bytes(iqra4_min(bayt, asas, 4, "TPL palette offset")?),
            });
        }

        let mut lawhat: Vec<(u32, TarwisLawha)> = Vec::new();
        let mut suwar: Vec<(u32, TarwisSura)> = Vec::new();
        for entry in &wasf {
            if entry.mawdi_lawha != 0 {
                let tarwis = iqra_tarwis_lawha(bayt, entry.mawdi_lawha)?;
                daa_farid(
                    &mut lawhat,
                    entry.mawdi_lawha,
                    tarwis,
                    "TPL palette header offset",
                )?;
            }
            let tarwis = iqra_tarwis_sura(bayt, entry.mawdi_sura)?;
            daa_farid(
                &mut suwar,
                entry.mawdi_sura,
                tarwis,
                "TPL image header offset",
            )?;
        }

        // The trailer starts wherever the last header ends. Taking the maximum
        // over the headers rather than the smallest declared data offset is
        // deliberate: in a `.fnt` the image and the palette both point at the
        // block's own end, so a reader that trusted a data offset would decide
        // the trailer starts before the header it just parsed.
        let mut mawdi_dhayl = mawdi_jadwal.saturating_add(adad.saturating_mul(TUL_WASF));
        for (mawdi, _) in &lawhat {
            mawdi_dhayl = mawdi_dhayl.max(mawdi.saturating_add(TUL_TARWIS_LAWHA));
        }
        for (mawdi, _) in &suwar {
            mawdi_dhayl = mawdi_dhayl.max(mawdi.saturating_add(TUL_TARWIS_SURA));
        }

        let bidaya = usize::try_from(mawdi_dhayl)
            .unwrap_or(usize::MAX)
            .min(bayt.len());
        let dhayl = bayt.get(bidaya..).unwrap_or(&[]).to_vec();

        lawhat.sort_unstable_by_key(|(mawdi, _)| *mawdi);
        suwar.sort_unstable_by_key(|(mawdi, _)| *mawdi);

        Ok(Self {
            tarteeb,
            mawdi_jadwal,
            wasf,
            lawhat,
            suwar,
            dhayl,
            mawdi_dhayl,
        })
    }

    /// Serialises the block back to bytes.
    ///
    /// Each piece is written at the offset it carries and the gaps are zero, so a
    /// block that was parsed from bytes whose gaps were zero — which is every
    /// shipped one — comes back identical. Nothing is recomputed here: a caller
    /// that changed a dimension has already changed the field, and a serialiser
    /// that also derived offsets would be a second opinion about a layout the
    /// file already stated.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] when the offsets place the block past
    /// [`AQSA_KUTLA`], and [`KhataBio4::BunyaGhayrMutawaqqaa`] when two pieces
    /// would overlap.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataBio4> {
        let adad = u32::try_from(self.wasf.len()).unwrap_or(u32::MAX);
        let mut tul = u64::from(self.mawdi_dhayl).saturating_add(tul_u64(self.dhayl.len()));
        tul = tul.max(u64::from(TUL_TARWIS_KUTLA));
        if tul > AQSA_KUTLA {
            return Err(KhataBio4::HajmMufrit {
                haql: "TPL block",
                qeema: tul,
                saqf: AQSA_KUTLA,
            });
        }
        let hajm = usize::try_from(tul).unwrap_or(usize::MAX);
        let mut kharij = vec![0u8; hajm];

        iktub(&mut kharij, 0, &self.tarteeb.bayt_kalima(SIHR))?;
        iktub(&mut kharij, 4, &adad.to_le_bytes())?;
        iktub(&mut kharij, 8, &self.mawdi_jadwal.to_le_bytes())?;

        for (fahras, entry) in self.wasf.iter().enumerate() {
            let khatwa = u32::try_from(fahras)
                .unwrap_or(u32::MAX)
                .saturating_mul(TUL_WASF);
            let asas = self.mawdi_jadwal.saturating_add(khatwa);
            iktub(&mut kharij, asas, &entry.mawdi_sura.to_le_bytes())?;
            iktub(
                &mut kharij,
                asas.saturating_add(4),
                &entry.mawdi_lawha.to_le_bytes(),
            )?;
        }

        for (mawdi, tarwis) in &self.lawhat {
            iktub(&mut kharij, *mawdi, &tarwis.adad.to_le_bytes())?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(2),
                &[tarwis.mufakkak, tarwis.hashw],
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(4),
                &tarwis.sigha.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(8),
                &tarwis.mawdi.to_le_bytes(),
            )?;
        }

        for (mawdi, tarwis) in &self.suwar {
            iktub(&mut kharij, *mawdi, &tarwis.irtifa.to_le_bytes())?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(2),
                &tarwis.ard.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(4),
                &tarwis.sigha.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(8),
                &tarwis.mawdi.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(12),
                &tarwis.laff_s.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(16),
                &tarwis.laff_a.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(20),
                &tarwis.murashah_asghar.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(24),
                &tarwis.murashah_akbar.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(28),
                &tarwis.inhiyaz_mustawa.to_le_bytes(),
            )?;
            iktub(
                &mut kharij,
                mawdi.saturating_add(32),
                &[
                    tarwis.hafat_mustawa,
                    tarwis.adna_mustawa,
                    tarwis.aqsa_mustawa,
                    tarwis.mufakkak,
                ],
            )?;
        }

        iktub(&mut kharij, self.mawdi_dhayl, &self.dhayl)?;
        Ok(kharij)
    }

    /// The block's serialised length.
    #[must_use]
    pub fn tul(&self) -> u64 {
        u64::from(self.mawdi_dhayl).saturating_add(tul_u64(self.dhayl.len()))
    }

    /// The single image header, for the overwhelmingly common one-image block.
    ///
    /// [`None`] when the block holds anything but exactly one image, which is a
    /// shape no `BIO4/Font` file has and every caller here refuses rather than
    /// guesses at.
    #[must_use]
    pub fn sura_wahida(&self) -> Option<&TarwisSura> {
        match self.suwar.as_slice() {
            [(_, tarwis)] => Some(tarwis),
            _ => None,
        }
    }

    /// The single image header, mutably.
    pub fn sura_wahida_mut(&mut self) -> Option<&mut TarwisSura> {
        match self.suwar.as_mut_slice() {
            [(_, tarwis)] => Some(tarwis),
            _ => None,
        }
    }

    /// The single palette header, or [`None`] when there is not exactly one.
    #[must_use]
    pub fn lawha_wahida(&self) -> Option<&TarwisLawha> {
        match self.lawhat.as_slice() {
            [(_, tarwis)] => Some(tarwis),
            _ => None,
        }
    }

    /// Refuses a block that is not shaped like a `BIO4` font's.
    ///
    /// One image in [`SighatSura::C4`], one palette in [`SighatLawn::Rgb5a3`]
    /// with sixteen entries — which is what the four bits of a C4 index address,
    /// and a palette that disagreed with the index width would be a file whose
    /// two halves were produced by different tools.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] for the wrong number of images or
    /// palettes and for a palette whose entry count does not match the index
    /// width, and [`KhataBio4::SighatSuraGhayrMaduma`] or
    /// [`KhataBio4::SighatLawnGhayrMaduma`] naming a format this build does not
    /// handle.
    pub fn tahaqquq_khatt(&self) -> Result<(), KhataBio4> {
        let Some(sura) = self.sura_wahida() else {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "TPL image count",
                qeema: tul_u64(self.suwar.len()),
                sabab: "a BIO4 font declares exactly one image",
            });
        };
        if SighatSura::min_raqm(sura.sigha) != Some(SighatSura::C4) {
            return Err(KhataBio4::SighatSuraGhayrMaduma {
                raqm: sura.sigha,
                ism: SighatSura::ism_raqm(sura.sigha),
            });
        }
        let Some(lawha) = self.lawha_wahida() else {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "TPL palette count",
                qeema: tul_u64(self.lawhat.len()),
                sabab: "a C4 image is addressed through exactly one palette",
            });
        };
        if SighatLawn::min_raqm(lawha.sigha) != Some(SighatLawn::Rgb5a3) {
            return Err(KhataBio4::SighatLawnGhayrMaduma {
                raqm: lawha.sigha,
                ism: SighatLawn::ism_raqm(lawha.sigha),
            });
        }
        if lawha.adad != 16 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "TPL palette entry count",
                qeema: u64::from(lawha.adad),
                sabab: "a four-bit index addresses sixteen entries and no other number",
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reading and writing helpers
// ---------------------------------------------------------------------------

/// Four bytes at `mawqi`, or a refusal naming the field that wanted them.
fn iqra4(bayt: &[u8], mawqi: u32, haql: &'static str) -> Result<[u8; 4], KhataBio4> {
    let bidaya = usize::try_from(mawqi).unwrap_or(usize::MAX);
    let nihaya = bidaya.saturating_add(4);
    let qita = bayt
        .get(bidaya..nihaya)
        .ok_or_else(|| KhataBio4::MalafQaseer {
            haql,
            mawqi: u64::from(mawqi),
            tul: tul_u64(bayt.len()),
            matlub: u64::from(mawqi).saturating_add(4),
        })?;
    let mut kalima = [0u8; 4];
    kalima.copy_from_slice(qita);
    Ok(kalima)
}

/// Four bytes at `asas + izaha`, refusing an offset that overflows.
fn iqra4_min(bayt: &[u8], asas: u32, izaha: u32, haql: &'static str) -> Result<[u8; 4], KhataBio4> {
    let mawqi = asas
        .checked_add(izaha)
        .ok_or_else(|| KhataBio4::BunyaGhayrMutawaqqaa {
            haql,
            qeema: u64::from(asas),
            sabab: "runs past the address space of a block",
        })?;
    iqra4(bayt, mawqi, haql)
}

/// Two bytes at `mawqi`.
fn iqra2(bayt: &[u8], mawqi: u32, haql: &'static str) -> Result<u16, KhataBio4> {
    let bidaya = usize::try_from(mawqi).unwrap_or(usize::MAX);
    let nihaya = bidaya.saturating_add(2);
    let qita = bayt
        .get(bidaya..nihaya)
        .ok_or_else(|| KhataBio4::MalafQaseer {
            haql,
            mawqi: u64::from(mawqi),
            tul: tul_u64(bayt.len()),
            matlub: u64::from(mawqi).saturating_add(2),
        })?;
    let mut kalima = [0u8; 2];
    kalima.copy_from_slice(qita);
    Ok(u16::from_le_bytes(kalima))
}

/// One byte at `mawqi`.
fn iqra1(bayt: &[u8], mawqi: u32, haql: &'static str) -> Result<u8, KhataBio4> {
    let fahras = usize::try_from(mawqi).unwrap_or(usize::MAX);
    bayt.get(fahras)
        .copied()
        .ok_or_else(|| KhataBio4::MalafQaseer {
            haql,
            mawqi: u64::from(mawqi),
            tul: tul_u64(bayt.len()),
            matlub: u64::from(mawqi).saturating_add(1),
        })
}

/// Reads a palette header at `mawdi`.
fn iqra_tarwis_lawha(bayt: &[u8], mawdi: u32) -> Result<TarwisLawha, KhataBio4> {
    hatta_muhadhah(mawdi, "TPL palette header offset")?;
    Ok(TarwisLawha {
        adad: iqra2(bayt, mawdi, "TPL palette entry count")?,
        mufakkak: iqra1(bayt, mawdi.saturating_add(2), "TPL palette unpacked")?,
        hashw: iqra1(bayt, mawdi.saturating_add(3), "TPL palette pad")?,
        sigha: u32::from_le_bytes(iqra4_min(bayt, mawdi, 4, "TPL palette format")?),
        mawdi: u32::from_le_bytes(iqra4_min(bayt, mawdi, 8, "TPL palette data offset")?),
    })
}

/// Reads an image header at `mawdi`.
fn iqra_tarwis_sura(bayt: &[u8], mawdi: u32) -> Result<TarwisSura, KhataBio4> {
    hatta_muhadhah(mawdi, "TPL image header offset")?;
    Ok(TarwisSura {
        irtifa: iqra2(bayt, mawdi, "TPL image height")?,
        ard: iqra2(bayt, mawdi.saturating_add(2), "TPL image width")?,
        sigha: u32::from_le_bytes(iqra4_min(bayt, mawdi, 4, "TPL image format")?),
        mawdi: u32::from_le_bytes(iqra4_min(bayt, mawdi, 8, "TPL image data offset")?),
        laff_s: u32::from_le_bytes(iqra4_min(bayt, mawdi, 12, "TPL wrap s")?),
        laff_a: u32::from_le_bytes(iqra4_min(bayt, mawdi, 16, "TPL wrap t")?),
        murashah_asghar: u32::from_le_bytes(iqra4_min(bayt, mawdi, 20, "TPL min filter")?),
        murashah_akbar: u32::from_le_bytes(iqra4_min(bayt, mawdi, 24, "TPL mag filter")?),
        inhiyaz_mustawa: u32::from_le_bytes(iqra4_min(bayt, mawdi, 28, "TPL lod bias")?),
        hafat_mustawa: iqra1(bayt, mawdi.saturating_add(32), "TPL edge lod")?,
        adna_mustawa: iqra1(bayt, mawdi.saturating_add(33), "TPL min lod")?,
        aqsa_mustawa: iqra1(bayt, mawdi.saturating_add(34), "TPL max lod")?,
        mufakkak: iqra1(bayt, mawdi.saturating_add(35), "TPL unpacked")?,
    })
}

/// Refuses an offset that is not word-aligned.
///
/// Every offset in every shipped block is a multiple of four. An unaligned one
/// would be a file whose header table is not a header table, and reading it
/// would produce plausible numbers out of the wrong bytes.
fn hatta_muhadhah(mawdi: u32, haql: &'static str) -> Result<(), KhataBio4> {
    if mawdi.is_multiple_of(4) {
        Ok(())
    } else {
        Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql,
            qeema: u64::from(mawdi),
            sabab: "is not a multiple of four, and every offset in this format is",
        })
    }
}

/// Records a header at an offset, refusing a second, different one at the same
/// place.
///
/// Two descriptors sharing one palette is legal and common; two descriptors
/// disagreeing about what is at one offset is a file this module cannot
/// serialise back, because it would have to write both.
fn daa_farid<T: PartialEq>(
    majmua: &mut Vec<(u32, T)>,
    mawdi: u32,
    qeema: T,
    haql: &'static str,
) -> Result<(), KhataBio4> {
    if let Some((_, mawjud)) = majmua.iter().find(|(makan, _)| *makan == mawdi) {
        if *mawjud == qeema {
            return Ok(());
        }
        return Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql,
            qeema: u64::from(mawdi),
            sabab: "is claimed twice with two different headers",
        });
    }
    majmua.push((mawdi, qeema));
    Ok(())
}

/// Writes `qeema` at `mawdi`, refusing an offset outside the buffer.
fn iktub(hadaf: &mut [u8], mawdi: u32, qeema: &[u8]) -> Result<(), KhataBio4> {
    let bidaya = usize::try_from(mawdi).unwrap_or(usize::MAX);
    let nihaya = bidaya.saturating_add(qeema.len());
    let makan = hadaf
        .get_mut(bidaya..nihaya)
        .ok_or_else(|| KhataBio4::BunyaGhayrMutawaqqaa {
            haql: "TPL field offset",
            qeema: u64::from(mawdi),
            sabab: "falls outside the block the offsets describe",
        })?;
    makan.copy_from_slice(qeema);
    Ok(())
}
