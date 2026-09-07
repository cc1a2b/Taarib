//! الحزمة — `BIO4/ImagePack`, which is where the texels actually are, and the
//! `DDS`/`DXT5` payload the PC build samples.
//!
//! ## How a font finds its texture
//!
//! The four bytes at the TPL's image data offset are an identifier. Written as
//! eight hexadecimal digits **in reverse byte order** they name a file in
//! `BIO4/ImagePack`: `common_zh-cn.fnt` carries `09 00 00 0f` and its texture is
//! `0f000009.pack`. All thirty-two fonts in `BIO4/Font` resolve to a file that
//! exists, and the identifier is repeated inside the pack's own header, which is
//! what makes the mapping checkable rather than merely plausible.
//!
//! ```text
//! +0x00  [u8;4]  identifier, the same four bytes the .fnt carries
//! +0x04  u32     entry count      1 in every font pack
//! +0x08  u32     table offset     0x70 in every font pack
//! +0x70  u32     payload length   file length minus 0x80
//! +0x74  u32     0xFFFF_FFFF
//! +0x78  [u8;4]  identifier again
//! +0x7C  u32     0
//! +0x80          payload — a DDS
//! ```
//!
//! ## The payload is not a TPL
//!
//! It is a `DDS` in `DXT5`, the Ultimate HD Edition's replacement art. For
//! `report_*` and `system_*` it is at or near the dimensions the TPL declares;
//! for `common_*`, `event*` and `stage*` it is exactly twice them; and for the
//! traditional-Chinese `ss_file`, `ss_term` and `sscrn` fonts it is a different
//! shape whose cells no longer match the metrics beside it. So a writer has to
//! decide which of the two containers it is filling, and this module and
//! [`crate::sura`] are those two decisions rather than one with a flag.
//!
//! ## What was measured
//!
//! The coverage lives in the **alpha** channel. `13000009.pack` — the atlas for
//! `report_zh-cn.fnt` — decodes to 1024×448, and thresholding its alpha
//! reproduces the ink span of 548 of the 548 non-blank cells the metrics table
//! describes. The colour channels carry a greyscale of the same shape at lower
//! precision; a page written here reproduces that convention, greyscale in colour
//! and coverage in alpha, so that whichever channel the game's shader reads it
//! finds the glyph.

use crate::khata::{KhataBio4, tul_u64};
use crate::sura::{AQSA_TEXEL, SuraMufakkaka};

/// The four bytes a `DDS` begins with.
pub const DDS_SIHR: [u8; 4] = *b"DDS ";

/// Bytes of a pack header before the payload.
pub const TUL_TARWIS: usize = 0x80;

/// Bytes of a `DDS` header, magic included.
pub const TUL_TARWIS_DDS: usize = 128;

/// Where the pack's entry table sits.
const MAWDI_JADWAL: u32 = 0x70;

/// The largest pack this build will read.
///
/// The largest font pack in the game is 2.9 MB. Sixty-four is past every one of
/// them and past the largest `ImagePackHD` member by a comfortable margin, and it
/// is checked before anything is allocated.
pub const AQSA_HIZMA: u64 = 64 << 20;

/// A pack identifier: the four bytes a `.fnt` and its pack both carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HuwiyatHizma(pub [u8; 4]);

impl HuwiyatHizma {
    /// The file name this identifier resolves to inside `BIO4/ImagePack`.
    ///
    /// Eight lowercase hexadecimal digits in reverse byte order, then `.pack`.
    #[must_use]
    pub fn ism(self) -> String {
        const ARQAM: [char; 16] = [
            '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
        ];
        let Self(bayt) = self;
        let mut ism = String::with_capacity(13);
        for khana in bayt.iter().rev() {
            let aala = usize::from(khana.checked_div(16).unwrap_or(0));
            let adna = usize::from(*khana & 0x0F);
            ism.push(ARQAM.get(aala).copied().unwrap_or('0'));
            ism.push(ARQAM.get(adna).copied().unwrap_or('0'));
        }
        ism.push_str(".pack");
        ism
    }
}

/// A parsed `ImagePack` member.
#[derive(Clone, PartialEq, Eq)]
pub struct Hizma {
    /// The identifier, from the header.
    pub hawiya: HuwiyatHizma,
    /// The payload, which for a font is a `DDS`.
    pub himl: Vec<u8>,
}

impl core::fmt::Debug for Hizma {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        matbaa
            .debug_struct("Hizma")
            .field("hawiya", &self.hawiya.ism())
            .field("himl", &self.himl.len())
            .finish()
    }
}

impl Hizma {
    /// Parses a pack.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] above [`AQSA_HIZMA`], [`KhataBio4::MalafQaseer`]
    /// for a file that ends inside the header or before the payload it declares,
    /// and [`KhataBio4::BunyaGhayrMutawaqqaa`] when the entry count is not one,
    /// when the table is not where every font pack puts it, or when the two
    /// copies of the identifier disagree — which would mean the pack and the font
    /// that names it were matched by filename alone.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataBio4> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_HIZMA {
            return Err(KhataBio4::HajmMufrit {
                haql: ".pack",
                qeema: tul,
                saqf: AQSA_HIZMA,
            });
        }
        let tarwis = bayt
            .get(..TUL_TARWIS)
            .ok_or_else(|| KhataBio4::MalafQaseer {
                haql: ".pack header",
                mawqi: 0,
                tul,
                matlub: tul_u64(TUL_TARWIS),
            })?;

        let hawiya = HuwiyatHizma(quad(tarwis, 0)?);
        let adad = kalima(tarwis, 4)?;
        if adad != 1 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".pack entry count",
                qeema: u64::from(adad),
                sabab: "every font pack in this game holds exactly one payload",
            });
        }
        let jadwal = kalima(tarwis, 8)?;
        if jadwal != MAWDI_JADWAL {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".pack table offset",
                qeema: u64::from(jadwal),
                sabab: "is not the 0x70 every font pack in this game uses",
            });
        }
        let tul_himl = kalima(tarwis, MAWDI_JADWAL)?;
        let thaniya = HuwiyatHizma(quad(tarwis, MAWDI_JADWAL.saturating_add(8))?);
        if thaniya != hawiya {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".pack identifier",
                qeema: u64::from(u32::from_le_bytes(hawiya.0)),
                sabab: "is not repeated identically in the entry table",
            });
        }

        let matlub = u64::from(tul_himl).saturating_add(tul_u64(TUL_TARWIS));
        let himl = bayt.get(TUL_TARWIS..).unwrap_or(&[]);
        if tul_u64(himl.len()) < u64::from(tul_himl) {
            return Err(KhataBio4::MalafQaseer {
                haql: ".pack payload",
                mawqi: tul_u64(TUL_TARWIS),
                tul,
                matlub,
            });
        }
        let hajm = usize::try_from(tul_himl).unwrap_or(usize::MAX);
        Ok(Self {
            hawiya,
            himl: himl.get(..hajm).unwrap_or(himl).to_vec(),
        })
    }

    /// Serialises the pack.
    ///
    /// Reproduces the header every font pack in the game carries, which is what
    /// makes an unmodified round trip byte-identical: the sixteen zero words
    /// between the block header and the entry table are zero in every shipped
    /// pack, and are written as zero here.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] when the payload puts the file above
    /// [`AQSA_HIZMA`].
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataBio4> {
        let tul = tul_u64(self.himl.len()).saturating_add(tul_u64(TUL_TARWIS));
        if tul > AQSA_HIZMA {
            return Err(KhataBio4::HajmMufrit {
                haql: ".pack",
                qeema: tul,
                saqf: AQSA_HIZMA,
            });
        }
        let tul_himl = u32::try_from(self.himl.len()).unwrap_or(u32::MAX);
        let mut kharij = vec![0u8; TUL_TARWIS];
        daa(&mut kharij, 0, &self.hawiya.0);
        daa(&mut kharij, 4, &1u32.to_le_bytes());
        daa(&mut kharij, 8, &MAWDI_JADWAL.to_le_bytes());
        daa(&mut kharij, MAWDI_JADWAL, &tul_himl.to_le_bytes());
        daa(
            &mut kharij,
            MAWDI_JADWAL.saturating_add(4),
            &u32::MAX.to_le_bytes(),
        );
        daa(&mut kharij, MAWDI_JADWAL.saturating_add(8), &self.hawiya.0);
        kharij.extend_from_slice(&self.himl);
        Ok(kharij)
    }
}

// ---------------------------------------------------------------------------
// DDS / DXT5
// ---------------------------------------------------------------------------

/// Decodes the alpha channel of a `DXT5` `DDS` into one byte per texel.
///
/// Alpha and not luminance: that is the channel the shipped atlases carry their
/// coverage in at full precision, and the one the cell-box measurement in
/// [`crate::khatt`] was made against.
///
/// # Errors
///
/// [`KhataBio4::MalafQaseer`] for a file shorter than a `DDS` header,
/// [`KhataBio4::BunyaGhayrMutawaqqaa`] for a header that is not a `DXT5` `DDS`,
/// [`KhataBio4::HajmMufrit`] above [`AQSA_TEXEL`], and
/// [`KhataBio4::HimlGhayrMutabaq`] when the texel blocks run out before the
/// declared dimensions are covered.
pub fn ifkak_dds(bayt: &[u8]) -> Result<SuraMufakkaka, KhataBio4> {
    let tarwis = bayt
        .get(..TUL_TARWIS_DDS)
        .ok_or_else(|| KhataBio4::MalafQaseer {
            haql: "DDS header",
            mawqi: 0,
            tul: tul_u64(bayt.len()),
            matlub: tul_u64(TUL_TARWIS_DDS),
        })?;
    if tarwis.get(..4) != Some(&DDS_SIHR[..]) {
        return Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql: "DDS magic",
            qeema: 0,
            sabab: "is not `DDS `",
        });
    }
    if kalima(tarwis, 4)? != 124 {
        return Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql: "DDS header size",
            qeema: u64::from(kalima(tarwis, 4)?),
            sabab: "is not the 124 the format fixes",
        });
    }
    // Offsets are from the magic, so each is four past the header field's own.
    let irtifa = kalima(tarwis, 12)?;
    let ard = kalima(tarwis, 16)?;
    let rubaee = quad(tarwis, 84)?;
    if rubaee != *b"DXT5" {
        return Err(KhataBio4::BunyaGhayrMutawaqqaa {
            haql: "DDS four-character code",
            qeema: u64::from(u32::from_le_bytes(rubaee)),
            sabab: "is not DXT5, which is the only one this build reads",
        });
    }
    let texel = u64::from(ard).saturating_mul(u64::from(irtifa));
    if texel > AQSA_TEXEL {
        return Err(KhataBio4::HajmMufrit {
            haql: "DDS texels",
            qeema: texel,
            saqf: AQSA_TEXEL,
        });
    }

    let kutal_s = kutal(ard);
    let kutal_a = kutal(irtifa);
    let matlub = u64::from(kutal_s)
        .saturating_mul(u64::from(kutal_a))
        .saturating_mul(16);
    let himl = bayt.get(TUL_TARWIS_DDS..).unwrap_or(&[]);
    if tul_u64(himl.len()) < matlub {
        return Err(KhataBio4::HimlGhayrMutabaq {
            sigha: "DXT5",
            ard,
            irtifa,
            tul: tul_u64(himl.len()),
            matlub,
        });
    }

    let mut sura = SuraMufakkaka::jadeeda(ard, irtifa)?;
    for satr in 0..kutal_a {
        for amud in 0..kutal_s {
            let fahras = u64::from(satr)
                .saturating_mul(u64::from(kutal_s))
                .saturating_add(u64::from(amud))
                .saturating_mul(16);
            let bidaya = usize::try_from(fahras).unwrap_or(usize::MAX);
            let Some(kutla) = himl.get(bidaya..bidaya.saturating_add(16)) else {
                continue;
            };
            ifkak_kutlat_alfa(
                kutla,
                &mut sura,
                amud.saturating_mul(4),
                satr.saturating_mul(4),
            );
        }
    }
    Ok(sura)
}

/// Encodes an image as a `DXT5` `DDS`, greyscale in colour and coverage in
/// alpha.
///
/// # Errors
///
/// [`KhataBio4::HimlGhayrMutabaq`] when the image's buffer does not match its
/// declared dimensions, and [`KhataBio4::HajmMufrit`] when the encoded size
/// overflows.
pub fn irsim_dds(sura: &SuraMufakkaka) -> Result<Vec<u8>, KhataBio4> {
    let matlub = u64::from(sura.ard).saturating_mul(u64::from(sura.irtifa));
    if tul_u64(sura.bayt.len()) != matlub {
        return Err(KhataBio4::HimlGhayrMutabaq {
            sigha: "unpacked",
            ard: sura.ard,
            irtifa: sura.irtifa,
            tul: tul_u64(sura.bayt.len()),
            matlub,
        });
    }
    let kutal_s = kutal(sura.ard);
    let kutal_a = kutal(sura.irtifa);
    let hajm = u64::from(kutal_s)
        .saturating_mul(u64::from(kutal_a))
        .saturating_mul(16);
    if hajm > AQSA_HIZMA {
        return Err(KhataBio4::HajmMufrit {
            haql: "DXT5 payload",
            qeema: hajm,
            saqf: AQSA_HIZMA,
        });
    }

    let saa = TUL_TARWIS_DDS.saturating_add(usize::try_from(hajm).unwrap_or(0));
    let mut kharij = Vec::with_capacity(saa);
    kharij.extend_from_slice(&DDS_SIHR);
    let mut tarwis = [0u8; 124];
    daa(&mut tarwis, 0, &124u32.to_le_bytes());
    // CAPS | HEIGHT | WIDTH | PIXELFORMAT | LINEARSIZE, which is the exact set
    // every shipped font pack declares.
    daa(&mut tarwis, 4, &0x0008_1007u32.to_le_bytes());
    daa(&mut tarwis, 8, &sura.irtifa.to_le_bytes());
    daa(&mut tarwis, 12, &sura.ard.to_le_bytes());
    daa(
        &mut tarwis,
        16,
        &u32::try_from(hajm).unwrap_or(u32::MAX).to_le_bytes(),
    );
    daa(&mut tarwis, 72, &32u32.to_le_bytes());
    daa(&mut tarwis, 76, &4u32.to_le_bytes());
    daa(&mut tarwis, 80, b"DXT5");
    daa(&mut tarwis, 104, &0x1000u32.to_le_bytes());
    kharij.extend_from_slice(&tarwis);

    for satr in 0..kutal_a {
        for amud in 0..kutal_s {
            kharij.extend_from_slice(&irsim_kutla(
                sura,
                amud.saturating_mul(4),
                satr.saturating_mul(4),
            ));
        }
    }
    Ok(kharij)
}

/// How many four-texel blocks cover `madaa`.
fn kutal(madaa: u32) -> u32 {
    madaa.saturating_add(3).checked_div(4).unwrap_or(0)
}

/// The eight alpha values a `BC3` alpha block interpolates.
fn sullam_alfa(a0: u8, a1: u8) -> [u8; 8] {
    let mut sullam = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for khatwa in 1..7u16 {
            let qeema = u16::from(a0)
                .saturating_mul(7u16.saturating_sub(khatwa))
                .saturating_add(u16::from(a1).saturating_mul(khatwa))
                .checked_div(7)
                .unwrap_or(0);
            if let Some(makan) = sullam.get_mut(usize::from(khatwa).saturating_add(1)) {
                *makan = u8::try_from(qeema).unwrap_or(u8::MAX);
            }
        }
    } else {
        for khatwa in 1..5u16 {
            let qeema = u16::from(a0)
                .saturating_mul(5u16.saturating_sub(khatwa))
                .saturating_add(u16::from(a1).saturating_mul(khatwa))
                .checked_div(5)
                .unwrap_or(0);
            if let Some(makan) = sullam.get_mut(usize::from(khatwa).saturating_add(1)) {
                *makan = u8::try_from(qeema).unwrap_or(u8::MAX);
            }
        }
        if let Some(makan) = sullam.get_mut(6) {
            *makan = 0;
        }
        if let Some(makan) = sullam.get_mut(7) {
            *makan = u8::MAX;
        }
    }
    sullam
}

/// Writes one block's alpha into `sura` at `(s, a)`.
fn ifkak_kutlat_alfa(kutla: &[u8], sura: &mut SuraMufakkaka, s: u32, a: u32) {
    let a0 = kutla.first().copied().unwrap_or(0);
    let a1 = kutla.get(1).copied().unwrap_or(0);
    let sullam = sullam_alfa(a0, a1);
    let mut bitat: u64 = 0;
    for (khatwa, bayt) in kutla.get(2..8).unwrap_or(&[]).iter().enumerate() {
        bitat |= u64::from(*bayt) << (khatwa.saturating_mul(8));
    }
    for texel in 0..16u32 {
        let fahras = usize::try_from((bitat >> (texel.saturating_mul(3))) & 0x7).unwrap_or(0);
        let qeema = sullam.get(fahras).copied().unwrap_or(0);
        let amud = texel.checked_rem(4).unwrap_or(0);
        let satr = texel.checked_div(4).unwrap_or(0);
        sura.daa_texel(s.saturating_add(amud), a.saturating_add(satr), qeema);
    }
}

/// Encodes one 4×4 block.
fn irsim_kutla(sura: &SuraMufakkaka, s: u32, a: u32) -> [u8; 16] {
    let mut qeem = [0u8; 16];
    for texel in 0..16usize {
        let amud = u32::try_from(texel.checked_rem(4).unwrap_or(0)).unwrap_or(0);
        let satr = u32::try_from(texel.checked_div(4).unwrap_or(0)).unwrap_or(0);
        if let Some(makan) = qeem.get_mut(texel) {
            *makan = sura
                .texel(s.saturating_add(amud), a.saturating_add(satr))
                .unwrap_or(0);
        }
    }
    let aqsa = qeem.iter().copied().max().unwrap_or(0);
    let adna = qeem.iter().copied().min().unwrap_or(0);

    let mut kutla = [0u8; 16];
    if let Some(makan) = kutla.first_mut() {
        *makan = aqsa;
    }
    if let Some(makan) = kutla.get_mut(1) {
        *makan = adna;
    }
    let sullam = sullam_alfa(aqsa, adna);
    let mut bitat: u64 = 0;
    for (texel, qeema) in qeem.iter().enumerate() {
        let fahras = aqrab(&sullam, *qeema);
        bitat |= u64::from(fahras) << (texel.saturating_mul(3));
    }
    for khatwa in 0..6usize {
        if let Some(makan) = kutla.get_mut(khatwa.saturating_add(2)) {
            *makan = u8::try_from((bitat >> (khatwa.saturating_mul(8))) & 0xFF).unwrap_or(0);
        }
    }

    // The colour half reproduces the shipped convention: a greyscale of the same
    // coverage, so that a shader reading colour rather than alpha still finds the
    // glyph. Both endpoints come from the block's own extremes, which makes a
    // block of one value encode exactly.
    let awwal = ramadi(aqsa);
    let thani = ramadi(adna);
    if let Some(makan) = kutla.get_mut(8..10) {
        makan.copy_from_slice(&awwal.to_le_bytes());
    }
    if let Some(makan) = kutla.get_mut(10..12) {
        makan.copy_from_slice(&thani.to_le_bytes());
    }
    let alwan = [aqsa, adna, thulth(aqsa, adna, 1), thulth(aqsa, adna, 2)];
    let mut fahrasat: u32 = 0;
    for (texel, qeema) in qeem.iter().enumerate() {
        let fahras = u32::from(aqrab(&alwan, *qeema));
        fahrasat |= fahras << (texel.saturating_mul(2));
    }
    if let Some(makan) = kutla.get_mut(12..16) {
        makan.copy_from_slice(&fahrasat.to_le_bytes());
    }
    kutla
}

/// The index of the nearest entry of `sullam` to `qeema`.
fn aqrab(sullam: &[u8], qeema: u8) -> u8 {
    let mut afdal: u8 = 0;
    let mut masafa = u16::MAX;
    for (fahras, mawjud) in sullam.iter().enumerate() {
        let farq = u16::from(mawjud.abs_diff(qeema));
        if farq < masafa {
            masafa = farq;
            afdal = u8::try_from(fahras).unwrap_or(0);
        }
    }
    afdal
}

/// An eight-bit grey as an `RGB565` word.
fn ramadi(qeema: u8) -> u16 {
    let khams = u16::from(qeema)
        .saturating_mul(31)
        .saturating_add(127)
        .checked_div(255);
    let sitt = u16::from(qeema)
        .saturating_mul(63)
        .saturating_add(127)
        .checked_div(255);
    let khams = khams.unwrap_or(0).min(31);
    let sitt = sitt.unwrap_or(0).min(63);
    khams.saturating_mul(0x0800) | sitt.saturating_mul(0x0020) | khams
}

/// One of the two interpolated colours of a four-colour `BC1` block.
fn thulth(a: u8, b: u8, khatwa: u16) -> u8 {
    let qeema = u16::from(a)
        .saturating_mul(3u16.saturating_sub(khatwa))
        .saturating_add(u16::from(b).saturating_mul(khatwa))
        .checked_div(3)
        .unwrap_or(0);
    u8::try_from(qeema).unwrap_or(u8::MAX)
}

// ---------------------------------------------------------------------------
// Byte helpers
// ---------------------------------------------------------------------------

/// A little-endian word at `mawqi`.
fn kalima(bayt: &[u8], mawqi: u32) -> Result<u32, KhataBio4> {
    Ok(u32::from_le_bytes(quad(bayt, mawqi)?))
}

/// Four bytes at `mawqi`.
fn quad(bayt: &[u8], mawqi: u32) -> Result<[u8; 4], KhataBio4> {
    let bidaya = usize::try_from(mawqi).unwrap_or(usize::MAX);
    let nihaya = bidaya.saturating_add(4);
    let qita = bayt
        .get(bidaya..nihaya)
        .ok_or_else(|| KhataBio4::MalafQaseer {
            haql: "header word",
            mawqi: u64::from(mawqi),
            tul: tul_u64(bayt.len()),
            matlub: u64::from(mawqi).saturating_add(4),
        })?;
    let mut kalima = [0u8; 4];
    kalima.copy_from_slice(qita);
    Ok(kalima)
}

/// Writes bytes at `mawqi`, ignoring an offset outside the buffer.
fn daa(hadaf: &mut [u8], mawqi: u32, qeema: &[u8]) {
    let bidaya = usize::try_from(mawqi).unwrap_or(usize::MAX);
    let nihaya = bidaya.saturating_add(qeema.len());
    if let Some(makan) = hadaf.get_mut(bidaya..nihaya) {
        makan.copy_from_slice(qeema);
    }
}
