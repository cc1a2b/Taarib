//! الصورة — the texels the TPL header describes: `GX_TF_C4` indices through a
//! sixteen-entry `GX_TL_RGB5A3` palette.
//!
//! ## What is verified here and what is not
//!
//! The *header* is verified: every `.fnt` in `BIO4/Font` declares
//! `GX_TF_C4` with a sixteen-entry `GX_TL_RGB5A3` palette, and
//! [`crate::tibl::Tibl::tahaqquq_khatt`] refuses anything else by name.
//!
//! The *encoding* is not verified against a shipped payload, because **no
//! shipped file carries one** — see [`crate::tibl`]. The tiling and the two
//! sixteen-bit forms below are the documented GameCube ones and this module round
//! trips against itself; what it cannot do is prove that the byte order of a
//! palette entry inside a `.fnt` is the console's, because there is no `.fnt`
//! with a palette to compare against. Entries are written **big-endian**, which
//! is what a GameCube reads, and this paragraph is here so a reader who finds
//! otherwise knows exactly which claim to overturn.
//!
//! The path the PC build actually samples is [`crate::hizma`], and that one *is*
//! verified against shipped bytes.
//!
//! ## Tiling
//!
//! `GX_TF_C4` stores texels in 8×8 blocks of 32 bytes, blocks left to right then
//! top to bottom, rows within a block top to bottom, and two texels per byte with
//! the **left** texel in the high nibble. A decoder that walked scanlines instead
//! would produce a shuffled image that is still recognisably a font, which is
//! exactly why the block size is written down here rather than assumed.
//!
//! ## Why sixteen entries buy eight opacities
//!
//! `RGB5A3` spends its top bit on the choice of form: set means five-bit opaque
//! colour, clear means three-bit alpha over four-bit colour. A ramp from
//! transparent to opaque therefore has to live in the transparent form, where
//! alpha is three bits — eight levels. The palette still has sixteen entries
//! because a four-bit index addresses sixteen and the header says sixteen; the
//! ramp simply lands on each opacity twice. Nothing is lost that the format could
//! have carried.

use crate::khata::{KhataBio4, tul_u64};
use crate::tibl::SighatSura;

/// How many entries a `GX_TF_C4` palette has.
pub const ADAD_ALWAN: usize = 16;

/// Bytes one `GX_TL_RGB5A3` palette occupies.
pub const TUL_LAWHAT: usize = ADAD_ALWAN * 2;

/// The largest unpacked image this build will hold.
///
/// Sixteen million texels — a 4096×4096 page — which is four times the largest
/// atlas the Ultimate HD Edition ships.
pub const AQSA_TEXEL: u64 = 4096 * 4096;

/// A sixteen-entry `GX_TL_RGB5A3` palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LawhatAlwan {
    madakhil: [u16; ADAD_ALWAN],
}

impl LawhatAlwan {
    /// The palette Taarib writes: white at sixteen steps of the eight opacities
    /// `RGB5A3` can express.
    ///
    /// Index zero is fully transparent, index fifteen fully opaque. Colour comes
    /// from nothing here and everything from the game's own material, which is
    /// the same rule `taarib_lawha` states for its own pages: an atlas carries
    /// coverage, never colour.
    #[must_use]
    pub fn taarib() -> Self {
        let mut madakhil = [0u16; ADAD_ALWAN];
        for (fahras, khana) in madakhil.iter_mut().enumerate() {
            let daraja = u16::try_from(fahras).unwrap_or(0);
            // Three bits of alpha over four bits of each channel, white.
            let alfa = daraja
                .saturating_mul(7)
                .saturating_add(7)
                .checked_div(15)
                .unwrap_or(0);
            *khana = alfa.min(7).saturating_mul(0x1000) | 0x0FFF;
        }
        Self { madakhil }
    }

    /// A palette from its sixteen big-endian entries.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HimlGhayrMutabaq`] when the slice is not exactly
    /// [`TUL_LAWHAT`] bytes.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataBio4> {
        if bayt.len() != TUL_LAWHAT {
            return Err(KhataBio4::HimlGhayrMutabaq {
                sigha: "GX_TL_RGB5A3",
                ard: 16,
                irtifa: 1,
                tul: tul_u64(bayt.len()),
                matlub: tul_u64(TUL_LAWHAT),
            });
        }
        let mut madakhil = [0u16; ADAD_ALWAN];
        for (khana, zawj) in madakhil.iter_mut().zip(bayt.chunks_exact(2)) {
            if let [aala, adna] = zawj {
                *khana = u16::from_be_bytes([*aala, *adna]);
            }
        }
        Ok(Self { madakhil })
    }

    /// The palette's bytes, big-endian per entry.
    #[must_use]
    pub fn ila_bayt(&self) -> [u8; TUL_LAWHAT] {
        let mut bayt = [0u8; TUL_LAWHAT];
        for (khana, makan) in self.madakhil.iter().zip(bayt.chunks_exact_mut(2)) {
            makan.copy_from_slice(&khana.to_be_bytes());
        }
        bayt
    }

    /// One entry, raw.
    #[must_use]
    pub fn madkhal(&self, fahras: u8) -> Option<u16> {
        self.madakhil.get(usize::from(fahras)).copied()
    }

    /// The opacity one entry carries, expanded to eight bits.
    ///
    /// The opaque form has no alpha field, so it reports 255.
    #[must_use]
    pub fn alfa(&self, fahras: u8) -> u8 {
        let Some(madkhal) = self.madkhal(fahras) else {
            return 0;
        };
        if madkhal & 0x8000 != 0 {
            return u8::MAX;
        }
        let thulath = madkhal.checked_div(0x1000).unwrap_or(0) & 0x7;
        // Three bits spread across eight: seven becomes 255 and zero becomes
        // zero, so a full-alpha entry is genuinely opaque.
        thulath
            .saturating_mul(255)
            .checked_div(7)
            .and_then(|qeema| u8::try_from(qeema).ok())
            .unwrap_or(0)
    }

    /// The index whose opacity is nearest to `taghtiya`.
    ///
    /// A linear map onto the sixteen indices, which lands on the eight opacities
    /// the format has. Written as a search over the palette's own alphas rather
    /// than as arithmetic on the index, so that a caller who supplies a different
    /// palette gets the right answer for *that* palette.
    #[must_use]
    pub fn fahras_taghtiya(&self, taghtiya: u8) -> u8 {
        let mut afdal: u8 = 0;
        let mut masafa = u16::MAX;
        for fahras in 0..ADAD_ALWAN {
            let raqm = u8::try_from(fahras).unwrap_or(0);
            let farq = u16::from(self.alfa(raqm).abs_diff(taghtiya));
            if farq < masafa {
                masafa = farq;
                afdal = raqm;
            }
        }
        afdal
    }
}

impl Default for LawhatAlwan {
    fn default() -> Self {
        Self::taarib()
    }
}

/// An image with one byte per texel, row-major from the top.
///
/// The same shape as `taarib_lawha::rasf::Safha`, and deliberately a separate
/// type: this one carries opacity for a palette or a `DXT5` alpha block, and that
/// page carries coverage or a distance field. Converting between them is one
/// documented step in [`crate::bina`] rather than a `From` that would let a
/// distance field reach a texture as if it were ink.
#[derive(Clone, PartialEq, Eq)]
pub struct SuraMufakkaka {
    /// Width in texels.
    pub ard: u32,
    /// Height in texels.
    pub irtifa: u32,
    /// The texels.
    pub bayt: Vec<u8>,
}

impl core::fmt::Debug for SuraMufakkaka {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // A derived one would put megabytes of texels in a log line.
        matbaa
            .debug_struct("SuraMufakkaka")
            .field("ard", &self.ard)
            .field("irtifa", &self.irtifa)
            .field("bayt", &self.bayt.len())
            .finish()
    }
}

impl SuraMufakkaka {
    /// A transparent image.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] when the two dimensions multiply past
    /// [`AQSA_TEXEL`].
    pub fn jadeeda(ard: u32, irtifa: u32) -> Result<Self, KhataBio4> {
        let texel = u64::from(ard).saturating_mul(u64::from(irtifa));
        if texel > AQSA_TEXEL {
            return Err(KhataBio4::HajmMufrit {
                haql: "unpacked image texels",
                qeema: texel,
                saqf: AQSA_TEXEL,
            });
        }
        let hajm = usize::try_from(texel).unwrap_or(usize::MAX);
        Ok(Self {
            ard,
            irtifa,
            bayt: vec![0u8; hajm],
        })
    }

    /// One texel, or [`None`] outside the image.
    #[must_use]
    pub fn texel(&self, s: u32, a: u32) -> Option<u8> {
        if s >= self.ard || a >= self.irtifa {
            return None;
        }
        let fahras = u64::from(a)
            .checked_mul(u64::from(self.ard))?
            .checked_add(u64::from(s))?;
        self.bayt.get(usize::try_from(fahras).ok()?).copied()
    }

    /// Sets one texel. A position outside the image is ignored.
    pub fn daa_texel(&mut self, s: u32, a: u32, qeema: u8) {
        if s >= self.ard || a >= self.irtifa {
            return;
        }
        let mawdi = u64::from(a)
            .checked_mul(u64::from(self.ard))
            .and_then(|saf| saf.checked_add(u64::from(s)));
        let Some(fahras) = mawdi else {
            return;
        };
        if let Some(makan) = usize::try_from(fahras)
            .ok()
            .and_then(|i| self.bayt.get_mut(i))
        {
            *makan = qeema;
        }
    }

    /// Copies a rectangle of `masdar` into this image at `(s, a)`.
    ///
    /// Out-of-bounds texels on either side are skipped rather than refused: the
    /// caller that places glyphs into cells has already checked that a glyph fits
    /// its cell, and a second refusal here would only turn a bug into a different
    /// bug.
    pub fn ulsuq(&mut self, s: u32, a: u32, masdar: &[u8], ard: u32, irtifa: u32) {
        for satr in 0..irtifa {
            for amud in 0..ard {
                let Some(fahras) = u64::from(satr)
                    .checked_mul(u64::from(ard))
                    .and_then(|saf| saf.checked_add(u64::from(amud)))
                    .and_then(|i| usize::try_from(i).ok())
                else {
                    continue;
                };
                let Some(qeema) = masdar.get(fahras).copied() else {
                    continue;
                };
                self.daa_texel(s.saturating_add(amud), a.saturating_add(satr), qeema);
            }
        }
    }
}

/// Packs an image into `GX_TF_C4` texels through `lawhat`.
///
/// # Errors
///
/// [`KhataBio4::HimlGhayrMutabaq`] when the image's buffer does not match its
/// declared dimensions, and [`KhataBio4::HajmMufrit`] when the packed size
/// overflows.
pub fn irsim_c4(sura: &SuraMufakkaka, lawhat: &LawhatAlwan) -> Result<Vec<u8>, KhataBio4> {
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
    let hajm = SighatSura::C4
        .hajm(sura.ard, sura.irtifa)
        .ok_or(KhataBio4::HajmMufrit {
            haql: "GX_TF_C4 payload",
            qeema: matlub,
            saqf: AQSA_TEXEL,
        })?;
    let mut kharij = vec![0u8; usize::try_from(hajm).unwrap_or(usize::MAX)];

    let (kutla_ard, kutla_irtifa) = SighatSura::C4.kutla();
    let mut mawdi: usize = 0;
    let mut asas_a = 0u32;
    while asas_a < sura.irtifa {
        let mut asas_s = 0u32;
        while asas_s < sura.ard {
            for satr in 0..kutla_irtifa {
                let mut amud = 0u32;
                while amud < kutla_ard {
                    let s = asas_s.saturating_add(amud);
                    let a = asas_a.saturating_add(satr);
                    let yasar = ila_fahras(sura, lawhat, s, a);
                    let yameen = ila_fahras(sura, lawhat, s.saturating_add(1), a);
                    if let Some(makan) = kharij.get_mut(mawdi) {
                        *makan = yasar.saturating_mul(16) | (yameen & 0x0F);
                    }
                    mawdi = mawdi.saturating_add(1);
                    amud = amud.saturating_add(2);
                }
            }
            asas_s = asas_s.saturating_add(kutla_ard);
        }
        asas_a = asas_a.saturating_add(kutla_irtifa);
    }
    Ok(kharij)
}

/// Unpacks `GX_TF_C4` texels back into one byte per texel.
///
/// # Errors
///
/// [`KhataBio4::HimlGhayrMutabaq`] when the payload is not the length the
/// dimensions require, and [`KhataBio4::HajmMufrit`] when the image is above
/// [`AQSA_TEXEL`].
pub fn ifkak_c4(
    bayt: &[u8],
    ard: u32,
    irtifa: u32,
    lawhat: &LawhatAlwan,
) -> Result<SuraMufakkaka, KhataBio4> {
    let matlub = SighatSura::C4
        .hajm(ard, irtifa)
        .ok_or_else(|| KhataBio4::HajmMufrit {
            haql: "GX_TF_C4 payload",
            qeema: u64::from(ard).saturating_mul(u64::from(irtifa)),
            saqf: AQSA_TEXEL,
        })?;
    if tul_u64(bayt.len()) != matlub {
        return Err(KhataBio4::HimlGhayrMutabaq {
            sigha: "GX_TF_C4",
            ard,
            irtifa,
            tul: tul_u64(bayt.len()),
            matlub,
        });
    }
    let mut sura = SuraMufakkaka::jadeeda(ard, irtifa)?;

    let (kutla_ard, kutla_irtifa) = SighatSura::C4.kutla();
    let mut mawdi: usize = 0;
    let mut asas_a = 0u32;
    while asas_a < irtifa {
        let mut asas_s = 0u32;
        while asas_s < ard {
            for satr in 0..kutla_irtifa {
                let mut amud = 0u32;
                while amud < kutla_ard {
                    let zawj = bayt.get(mawdi).copied().unwrap_or(0);
                    mawdi = mawdi.saturating_add(1);
                    let yasar = zawj.checked_div(16).unwrap_or(0);
                    let yameen = zawj & 0x0F;
                    sura.daa_texel(
                        asas_s.saturating_add(amud),
                        asas_a.saturating_add(satr),
                        lawhat.alfa(yasar),
                    );
                    sura.daa_texel(
                        asas_s.saturating_add(amud).saturating_add(1),
                        asas_a.saturating_add(satr),
                        lawhat.alfa(yameen),
                    );
                    amud = amud.saturating_add(2);
                }
            }
            asas_s = asas_s.saturating_add(kutla_ard);
        }
        asas_a = asas_a.saturating_add(kutla_irtifa);
    }
    Ok(sura)
}

/// The palette index for one texel, transparent outside the image.
///
/// A block that runs past the right or bottom edge is padding the format
/// requires and the game never samples; filling it with index zero rather than
/// with the nearest texel keeps two encodings of the same image identical.
fn ila_fahras(sura: &SuraMufakkaka, lawhat: &LawhatAlwan, s: u32, a: u32) -> u8 {
    sura.texel(s, a)
        .map_or(0, |taghtiya| lawhat.fahras_taghtiya(taghtiya))
}
