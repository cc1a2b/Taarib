//! الشبكة — the fixed cell grid a `BIO4` font's texture is, and the law that
//! ties it to the height the TPL declares.
//!
//! ## The law
//!
//! ```text
//! pitch    = metrics[0].yasar                       (28 texels; 20 for `system`)
//! columns  = floor(texture width / pitch)           (36 at the 1024 every font uses)
//! cells    = metrics length, less the trailing run whose right edge exceeds the pitch
//! rows     = ceil(cells / columns)
//! height   = rows * pitch
//! cell i   at (x, y) = ((i mod columns) * pitch, (i div columns) * pitch)
//! ```
//!
//! ## It is also the law the executable runs
//!
//! The rule above was derived from the files. It has since been read out of the
//! game: the quad emitter at `0x0071_7660` in `bio4.exe` computes
//! `columns = width / cell width`, then `x = (cell % columns) * cell width` and
//! `y = (cell / columns) * cell height`, taking the width from the `u16` at offset
//! 2 of the embedded TPL image header. Two details the files could not have shown:
//! the pitch is **not** read from the metrics table but passed in when the font is
//! loaded — `0x1C`, `0x14` or `0x40` depending on the family — and the horizontal
//! and vertical pitches are separate fields that happen to be equal in every
//! shipped load. `metrics[0].yasar` agrees with the loaded pitch in all
//! thirty-two, which is why taking it from there is sound and why it is checked
//! rather than trusted. See [`crate::kharita`].
//!
//! Every one of the thirty-two `.fnt` files in `BIO4/Font` satisfies it exactly,
//! including `system_zh-cn.fnt`, which is the only one that distinguishes it from
//! the simpler rule that ignores the trailing run: `system` has 160 entries at a
//! 20-texel pitch, and 160 cells would need four rows where its TPL declares
//! three. Eight of its entries have a right edge of 32 in a 20-texel cell, which
//! is not a cell in this atlas at all — those are the button-icon slots the
//! game's text references as `^917539^`-style tags and draws from somewhere else.
//! Strip them and the height is exact.
//!
//! ## Where the pitch comes from
//!
//! Entry zero of every shipped metrics table is `(pitch, 0)` — a span whose left
//! edge is at the pitch and whose right edge is at zero, which is the natural way
//! to encode "this cell is blank" and, read the other way, is the only place the
//! pitch is written down. Both readings coincide in all thirty-two files, and
//! [`Shabaka::min_madakhil`] takes it from there and then **checks** it: a pitch
//! that does not divide the declared width into whole columns, or does not
//! reproduce the declared height, is refused rather than used. That check is what
//! turns a coincidence into something a writer may rely on.

use crate::khata::KhataBio4;

/// The widest texture this build will describe a grid over.
///
/// Every shipped `BIO4` font is 1024 texels wide and the Ultimate HD Edition's
/// replacement art goes to 2048. Eight thousand is far past both, and low enough
/// that a corrupt width is refused before anything is allocated for it.
pub const AQSA_ARD: u32 = 8192;

/// The most cells this build will place.
///
/// The largest shipped font holds 1,064. Sixty-five thousand is three orders of
/// magnitude past it.
pub const AQSA_KHANAT: u32 = 65_536;

/// A fixed cell grid: pitch, columns, rows, and the texture they imply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Shabaka {
    hajm_khana: u32,
    ard: u32,
    aamida: u32,
    sufuf: u32,
    khanat: u32,
}

impl Shabaka {
    /// Builds a grid that holds `khanat` cells of `hajm_khana` texels in a
    /// texture `ard` wide.
    ///
    /// The height is derived, never given: a caller that supplied one would be
    /// supplying a number the other three already determine, and the two would
    /// eventually disagree.
    ///
    /// The width is deliberately **not** required to be a whole number of cells.
    /// Every shipped font is 1024 texels wide at a 28-texel pitch, which leaves
    /// sixteen texels unused at the right of every row; a constructor that
    /// insisted on a clean division would refuse the format it exists to read.
    ///
    /// # Errors
    ///
    /// [`KhataBio4::BunyaGhayrMutawaqqaa`] when the pitch is zero or does not fit
    /// the width even once, and [`KhataBio4::HajmMufrit`] when the width, the
    /// cell count or the derived height is above this build's ceiling.
    pub fn jadeeda(hajm_khana: u32, ard: u32, khanat: u32) -> Result<Self, KhataBio4> {
        if ard == 0 || ard > AQSA_ARD {
            return Err(KhataBio4::HajmMufrit {
                haql: "grid texture width",
                qeema: u64::from(ard),
                saqf: u64::from(AQSA_ARD),
            });
        }
        if khanat > AQSA_KHANAT {
            return Err(KhataBio4::HajmMufrit {
                haql: "grid cell count",
                qeema: u64::from(khanat),
                saqf: u64::from(AQSA_KHANAT),
            });
        }
        if hajm_khana == 0 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "grid cell pitch",
                qeema: 0,
                sabab: "a cell of no texels addresses nothing",
            });
        }
        let aamida = ard.checked_div(hajm_khana).unwrap_or(0);
        if aamida == 0 {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "grid cell pitch",
                qeema: u64::from(hajm_khana),
                sabab: "is wider than the whole texture",
            });
        }
        let sufuf = khanat
            .checked_add(aamida.saturating_sub(1))
            .and_then(|mawsu| mawsu.checked_div(aamida))
            .unwrap_or(0)
            .max(1);
        let irtifa = u64::from(sufuf).saturating_mul(u64::from(hajm_khana));
        if irtifa > u64::from(AQSA_ARD) {
            return Err(KhataBio4::HajmMufrit {
                haql: "grid texture height",
                qeema: irtifa,
                saqf: u64::from(AQSA_ARD),
            });
        }
        Ok(Self {
            hajm_khana,
            ard,
            aamida,
            sufuf,
            khanat,
        })
    }

    /// Derives the grid a metrics table describes, and checks it against the
    /// dimensions the TPL declares.
    ///
    /// `madakhil` is the whole table, trailing off-atlas entries included;
    /// stripping them is this function's job and is part of the law.
    ///
    /// # Errors
    ///
    /// Whatever [`Shabaka::jadeeda`] refuses, plus
    /// [`KhataBio4::ShabakaGhayrMutasiqa`] when the derived height is not the one
    /// the TPL carries — which means the table and the texture were not produced
    /// from one another and nothing written from this grid would land where the
    /// game samples.
    pub fn min_madakhil(madakhil: &[(u32, u32)], ard: u32, irtifa: u32) -> Result<Self, KhataBio4> {
        let Some(&(hajm_khana, _)) = madakhil.first() else {
            return Err(KhataBio4::BunyaGhayrMutawaqqaa {
                haql: "metrics table length",
                qeema: 0,
                sabab: "an empty table carries neither a cell pitch nor a cell",
            });
        };
        let khanat = u32::try_from(adad_khanat(madakhil, hajm_khana)).unwrap_or(u32::MAX);
        let shabaka = Self::jadeeda(hajm_khana, ard, khanat)?;
        if shabaka.irtifa() != irtifa {
            return Err(KhataBio4::ShabakaGhayrMutasiqa {
                hajm_khana,
                aamida: shabaka.aamida,
                khanat,
                irtifa_muallan: irtifa,
                irtifa_mahsub: shabaka.irtifa(),
            });
        }
        Ok(shabaka)
    }

    /// The cell pitch, in texels, on both axes.
    #[must_use]
    pub const fn hajm_khana(self) -> u32 {
        self.hajm_khana
    }

    /// The texture's width, in texels.
    #[must_use]
    pub const fn ard(self) -> u32 {
        self.ard
    }

    /// The texture's height, in texels.
    #[must_use]
    pub const fn irtifa(self) -> u32 {
        self.sufuf.saturating_mul(self.hajm_khana)
    }

    /// Cells per row.
    #[must_use]
    pub const fn aamida(self) -> u32 {
        self.aamida
    }

    /// Rows of cells.
    #[must_use]
    pub const fn sufuf(self) -> u32 {
        self.sufuf
    }

    /// How many cells the metrics table asked for.
    #[must_use]
    pub const fn khanat(self) -> u32 {
        self.khanat
    }

    /// How many cells the grid actually provides, which is the last row rounded
    /// out.
    ///
    /// The difference between this and [`Shabaka::khanat`] is texture the game
    /// can address and nothing describes. A writer may use those cells; a reader
    /// must not assume they are blank, because the original art may have put
    /// something there.
    #[must_use]
    pub const fn siaa(self) -> u32 {
        self.sufuf.saturating_mul(self.aamida)
    }

    /// The top-left texel of one cell, or [`None`] past the grid.
    #[must_use]
    pub fn mawdi(self, fahras: u32) -> Option<(u32, u32)> {
        if fahras >= self.siaa() {
            return None;
        }
        let amud = fahras.checked_rem(self.aamida)?;
        let satr = fahras.checked_div(self.aamida)?;
        Some((
            amud.saturating_mul(self.hajm_khana),
            satr.saturating_mul(self.hajm_khana),
        ))
    }
}

/// How many entries of a metrics table are cells of the atlas.
///
/// The table, less the trailing run whose right edge lies outside a cell. Those
/// entries describe glyphs the game draws from another texture — the button
/// icons its text references as `^`-delimited tag code points — and counting them
/// as cells makes the texture one row too tall, which is the single defect this
/// function exists to prevent. In all thirty-two shipped fonts the out-of-cell
/// entries are exactly a trailing run; an interior one would mean a table this
/// module does not understand, and it is left in the count rather than skipped so
/// that the height check refuses the file instead of silently reinterpreting it.
#[must_use]
pub fn adad_khanat(madakhil: &[(u32, u32)], hajm_khana: u32) -> usize {
    let mut adad = madakhil.len();
    while let Some(&(_, yameen)) = adad.checked_sub(1).and_then(|akhir| madakhil.get(akhir)) {
        if yameen <= hajm_khana {
            break;
        }
        adad = adad.saturating_sub(1);
    }
    adad
}
