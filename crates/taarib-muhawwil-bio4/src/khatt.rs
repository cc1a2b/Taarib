//! الخطّ — the `.fnt` container `BIO4/Font` ships, read and written whole.
//!
//! ## The file
//!
//! ```text
//! +0x00  u32   offset of the TPL block          0x20 in all thirty-two files
//! +0x04  u32   offset of the metrics table      0x80 in all thirty-two files
//! +0x08        padding to the TPL block         zero in all thirty-two files
//! +0x20        the TPL block                    see `crate::tibl`
//! +0x80        the metrics table                two bytes per cell, to the end
//! ```
//!
//! There is **no metrics header**. Entry *i* describes cell *i* directly, and
//! entry zero is the blank cell every font starts with — encoded as
//! `(pitch, 0)`, an empty span whose left edge sits at the cell pitch. Reading a
//! header there and shifting the table by one is the mistake this module was
//! written after making: it produces metrics that are plausible, off by one cell
//! everywhere, and correct nowhere.
//!
//! ## What an entry is
//!
//! `(yasar, yameen)`: the left and right edges of the glyph's ink inside its
//! cell, in cell-local texels, right exclusive. Not a bearing and an advance —
//! that is what the two bytes look like until they are laid over a real texture.
//!
//! `BIO4/ImagePack/13000009.pack` is the atlas for `report_zh-cn.fnt`, and it is
//! one of the few the Ultimate HD Edition did not re-author: it decodes to a
//! 1024×448 `DXT5` surface at exactly the dimensions the TPL declares. Measuring
//! the ink span of every non-blank cell in it and comparing against the metrics
//! table gives **548 agreements out of 548**, with the 28 remaining cells blank
//! and their entries the off-atlas `(0, 32)` run. Under the "bearing and advance"
//! reading the same measurement agrees on 242 of 547 — which is close enough to
//! look like a rounding problem and is in fact a different quantity.
//!
//! ## Round trip
//!
//! [`KhattBio4::ila_bayt`] reproduces its input byte for byte, including the
//! padding between the two offsets and the `ImagePack` identifier that follows
//! the TPL headers. That is asserted against all thirty-two shipped files in
//! `tests/khatt_haqiqi.rs`, and it is the precondition for trusting anything this
//! crate writes: a writer that cannot reproduce a file it did not change has no
//! standing to produce one it did.

use crate::khata::{KhataBio4, tul_u64};
use crate::shabaka::{AQSA_KHANAT, Shabaka, adad_khanat};
use crate::tibl::{SighatLawn, SighatSura, TarteebBayt, TarwisLawha, TarwisSura, Tibl, WasfTibl};

/// The largest `.fnt` this build will read.
///
/// The largest shipped one is 2,256 bytes. A megabyte is three orders of
/// magnitude past it and is checked before anything is allocated.
pub const AQSA_MALAF: u64 = 1 << 20;

/// Where the TPL block starts in every shipped `.fnt`, and where this module
/// puts it in one it writes.
pub const MAWDI_TIBL_QIYASI: u32 = 0x20;

/// Where the metrics table starts in every shipped `.fnt`.
pub const MAWDI_QIYASAT_QIYASI: u32 = 0x80;

/// One cell's horizontal ink span, in cell-local texels.
///
/// Right exclusive. A blank cell is any entry whose right edge is at or before
/// its left edge, which is how every shipped font encodes cell zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MadkhalKhana {
    /// The first inked column of the cell.
    pub yasar: u8,
    /// One past the last inked column.
    pub yameen: u8,
}

impl MadkhalKhana {
    /// An entry from its two edges.
    #[must_use]
    pub const fn jadeed(yasar: u8, yameen: u8) -> Self {
        Self { yasar, yameen }
    }

    /// Whether the cell has no ink.
    #[must_use]
    pub const fn khali(self) -> bool {
        self.yameen <= self.yasar
    }

    /// How many texels wide the ink is.
    #[must_use]
    pub const fn ard(self) -> u8 {
        self.yameen.saturating_sub(self.yasar)
    }

    /// The pair as the grid law reads it.
    #[must_use]
    pub fn zawj(self) -> (u32, u32) {
        (u32::from(self.yasar), u32::from(self.yameen))
    }
}

/// A parsed `.fnt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhattBio4 {
    /// Where the TPL block starts.
    pub mawdi_tibl: u32,
    /// Where the metrics table starts.
    pub mawdi_qiyasat: u32,
    /// The bytes between the two offsets and the TPL block, kept verbatim.
    ///
    /// Zero in every shipped file. Kept rather than assumed so that a round trip
    /// is a round trip and not an assertion that the game's authors never used
    /// them.
    pub muqaddima: Vec<u8>,
    /// The embedded TPL block.
    pub tibl: Tibl,
    /// The metrics table: one entry per cell, in cell order.
    pub madakhil: Vec<MadkhalKhana>,
}

impl KhattBio4 {
    /// Parses a `.fnt`.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::HajmMufrit`] for a file or a metrics table above this build's
    /// ceiling, [`KhataBio4::MalafQaseer`] for a file that ends inside a field,
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the two offsets do not describe a
    /// file — out of order, inside the header, or with an odd number of metric
    /// bytes after them — and whatever [`Tibl::min_bayt`] refuses.
    pub fn min_bayt(bayt: &[u8]) -> Result<Self, KhataBio4> {
        let tul = tul_u64(bayt.len());
        if tul > AQSA_MALAF {
            return Err(KhataBio4::HajmMufrit {
                haql: ".fnt",
                qeema: tul,
                saqf: AQSA_MALAF,
            });
        }
        let mawdi_tibl = iqra_kalima(bayt, 0, ".fnt TPL offset")?;
        let mawdi_qiyasat = iqra_kalima(bayt, 4, ".fnt metrics offset")?;

        if mawdi_tibl < 8 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".fnt TPL offset",
                qeema: u64::from(mawdi_tibl),
                sabab: "would overlap the two offsets that name it",
            });
        }
        if mawdi_qiyasat <= mawdi_tibl {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".fnt metrics offset",
                qeema: u64::from(mawdi_qiyasat),
                sabab: "does not follow the TPL block",
            });
        }
        if u64::from(mawdi_qiyasat) > tul {
            return Err(KhataBio4::MalafQaseer {
                haql: ".fnt metrics table",
                mawqi: u64::from(mawdi_qiyasat),
                tul,
                matlub: u64::from(mawdi_qiyasat),
            });
        }

        let bidaya_tibl = usize::try_from(mawdi_tibl).unwrap_or(usize::MAX);
        let bidaya_qiyasat = usize::try_from(mawdi_qiyasat).unwrap_or(usize::MAX);

        let muqaddima = bayt.get(8..bidaya_tibl).unwrap_or(&[]).to_vec();
        let kutla =
            bayt.get(bidaya_tibl..bidaya_qiyasat)
                .ok_or_else(|| KhataBio4::MalafQaseer {
                    haql: ".fnt TPL block",
                    mawqi: u64::from(mawdi_tibl),
                    tul,
                    matlub: u64::from(mawdi_qiyasat),
                })?;
        let tibl = Tibl::min_bayt(kutla, TarteebBayt::Saghir)?;

        let khaam = bayt.get(bidaya_qiyasat..).unwrap_or(&[]);
        if khaam.len() % 2 != 0 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".fnt metrics table length",
                qeema: tul_u64(khaam.len()),
                sabab: "is odd, and every entry is two bytes",
            });
        }
        let adad = tul_u64(khaam.len()).checked_div(2).unwrap_or(0);
        if adad > u64::from(AQSA_KHANAT) {
            return Err(KhataBio4::HajmMufrit {
                haql: ".fnt metrics entry count",
                qeema: adad,
                saqf: u64::from(AQSA_KHANAT),
            });
        }
        let madakhil = khaam
            .chunks_exact(2)
            .filter_map(|zawj| match zawj {
                [yasar, yameen] => Some(MadkhalKhana::jadeed(*yasar, *yameen)),
                _ => None,
            })
            .collect();

        Ok(Self {
            mawdi_tibl,
            mawdi_qiyasat,
            muqaddima,
            tibl,
            madakhil,
        })
    }

    /// Serialises the `.fnt` back to bytes.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the TPL block does not fit the
    /// space between the two offsets — which can only happen after a caller has
    /// changed one of them — and whatever [`Tibl::ila_bayt`] refuses.
    pub fn ila_bayt(&self) -> Result<Vec<u8>, KhataBio4> {
        let kutla = self.tibl.ila_bayt()?;
        let masaha = u64::from(self.mawdi_qiyasat).saturating_sub(u64::from(self.mawdi_tibl));
        if tul_u64(kutla.len()) > masaha {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: ".fnt TPL block length",
                qeema: tul_u64(kutla.len()),
                sabab: "does not fit between the TPL offset and the metrics offset",
            });
        }

        let hajm = usize::try_from(self.mawdi_qiyasat).unwrap_or(usize::MAX);
        let qiyasat = self.madakhil.len().saturating_mul(2);
        let mut kharij = Vec::with_capacity(hajm.saturating_add(qiyasat));
        kharij.extend_from_slice(&self.mawdi_tibl.to_le_bytes());
        kharij.extend_from_slice(&self.mawdi_qiyasat.to_le_bytes());
        kharij.extend_from_slice(&self.muqaddima);
        kharij.resize(usize::try_from(self.mawdi_tibl).unwrap_or(usize::MAX), 0);
        kharij.extend_from_slice(&kutla);
        kharij.resize(hajm, 0);
        for madkhal in &self.madakhil {
            kharij.push(madkhal.yasar);
            kharij.push(madkhal.yameen);
        }
        Ok(kharij)
    }

    /// The cell pitch this font's table declares.
    ///
    /// Entry zero's left edge. [`None`] for an empty table, which
    /// [`KhattBio4::min_bayt`] permits — a zero-length metrics table is a
    /// structurally valid file — and every consumer here refuses.
    #[must_use]
    pub fn hajm_khana(&self) -> Option<u32> {
        self.madakhil
            .first()
            .map(|madkhal| u32::from(madkhal.yasar))
    }

    /// The grid this font describes, checked against the TPL's own dimensions.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the block does not hold exactly
    /// one image, and whatever [`Shabaka::min_madakhil`] refuses — including the
    /// height check that is the whole point of deriving the grid rather than
    /// trusting it.
    pub fn shabaka(&self) -> Result<Shabaka, KhataBio4> {
        let sura = self
            .tibl
            .sura_wahida()
            .ok_or_else(|| KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "TPL image count",
                qeema: tul_u64(self.tibl.suwar.len()),
                sabab: "a BIO4 font declares exactly one image",
            })?;
        let zawj: Vec<(u32, u32)> = self.madakhil.iter().map(|m| m.zawj()).collect();
        Shabaka::min_madakhil(&zawj, u32::from(sura.ard), u32::from(sura.irtifa))
    }

    /// How many entries of the table are cells of this font's atlas.
    #[must_use]
    pub fn adad_khanat(&self) -> usize {
        let Some(hajm_khana) = self.hajm_khana() else {
            return 0;
        };
        let zawj: Vec<(u32, u32)> = self.madakhil.iter().map(|m| m.zawj()).collect();
        adad_khanat(&zawj, hajm_khana)
    }

    /// The `ImagePack` identifier the TPL's data offset points at.
    ///
    /// The four bytes that follow the TPL headers. [`None`] when the trailer is
    /// shorter than that, which no shipped file is.
    #[must_use]
    pub fn huwiyat_hizma(&self) -> Option<[u8; 4]> {
        let qita = self.tibl.dhayl.get(..4)?;
        let mut hawiya = [0u8; 4];
        hawiya.copy_from_slice(qita);
        Some(hawiya)
    }

    /// Builds a `.fnt` in the layout every shipped one uses.
    ///
    /// The TPL is written with the offsets, filters, wrap modes and palette shape
    /// the shipped fonts carry, its dimensions taken from `shabaka`, and its data
    /// offset pointing at the block's own end where the `ImagePack` identifier
    /// goes — because that is what the game reads there, and a writer that put
    /// texels there instead would produce a file whose header says one thing and
    /// whose loader does another.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the metrics table does not
    /// describe `shabaka` — the pitch in entry zero and the cell count both have
    /// to agree with it, and a caller that got either wrong would be writing a
    /// font whose texture is sampled at the wrong stride.
    pub fn jadeed(
        shabaka: Shabaka,
        madakhil: Vec<MadkhalKhana>,
        hizma: [u8; 4],
    ) -> Result<Self, KhataBio4> {
        let zawj: Vec<(u32, u32)> = madakhil.iter().map(|m| m.zawj()).collect();
        let mahsuba = Shabaka::min_madakhil(&zawj, shabaka.ard(), shabaka.irtifa())?;
        if mahsuba != shabaka {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "metrics table",
                qeema: tul_u64(madakhil.len()),
                sabab: "does not describe the grid it was built for",
            });
        }

        let ard = u16::try_from(shabaka.ard()).unwrap_or(u16::MAX);
        let irtifa = u16::try_from(shabaka.irtifa()).unwrap_or(u16::MAX);
        let tibl = Tibl {
            tarteeb: TarteebBayt::Saghir,
            mawdi_jadwal: 0x0C,
            wasf: vec![WasfTibl {
                mawdi_sura: 0x20,
                mawdi_lawha: 0x14,
            }],
            lawhat: vec![(
                0x14,
                TarwisLawha {
                    adad: 16,
                    mufakkak: 0,
                    hashw: 0,
                    sigha: SighatLawn::Rgb5a3.raqm(),
                    mawdi: 0x44,
                },
            )],
            suwar: vec![(
                0x20,
                TarwisSura {
                    irtifa,
                    ard,
                    sigha: SighatSura::C4.raqm(),
                    mawdi: 0x44,
                    laff_s: 0,
                    laff_a: 0,
                    murashah_asghar: 1,
                    murashah_akbar: 1,
                    inhiyaz_mustawa: 0,
                    hafat_mustawa: 0,
                    adna_mustawa: 0,
                    aqsa_mustawa: 0,
                    mufakkak: 0,
                },
            )],
            dhayl: hizma.to_vec(),
            mawdi_dhayl: 0x44,
        };

        Ok(Self {
            mawdi_tibl: MAWDI_TIBL_QIYASI,
            mawdi_qiyasat: MAWDI_QIYASAT_QIYASI,
            muqaddima: vec![0u8; 0x18],
            tibl,
            madakhil,
        })
    }
}

/// A little-endian word at `mawqi`.
fn iqra_kalima(bayt: &[u8], mawqi: u32, haql: &'static str) -> Result<u32, KhataBio4> {
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
    Ok(u32::from_le_bytes(kalima))
}
