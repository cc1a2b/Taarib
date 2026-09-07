//! المسافة — signed distance fields, at the page level.
//!
//! There is **one** definition in Taarib of what a signed distance field is, and
//! it is not in this file. [`taarib_saff::rasm::masafa_min_taghtiya`] is that
//! definition — an exact Euclidean distance transform, Felzenszwalb–Huttenlocher,
//! two passes, run over the inside and the outside separately and combined, with
//! the zero level at 128 and the spread given in pixels. This module calls it. It
//! does not reimplement it, does not approximate it with a chamfer mask, and does
//! not carry a second copy of it that could drift. A patch compiled by the atlas
//! and a glyph rasterized at runtime by the engine cannot disagree about where
//! the edge of a letter is, because they are the same code.
//!
//! What belongs here is everything *around* that transform: choosing a spread
//! from the range of sizes a patch draws at, rasterizing the source coverage
//! finer than the cell it will be stored in, and resampling down.
//!
//! ## The gamma question, and why it decides the API
//!
//! [`taarib_saff::rasm::Rassam::irsim`] gamma-encodes its output for
//! [`NamatRasm::Taghtiya`] and does not for [`NamatRasm::Masafa`]. That is
//! correct — a coverage page is blended in non-linear sRGB space by every engine
//! Taarib draws into, so the stored byte belongs in that space — but it means
//! there is no way to ask that function for a *linear* coverage bitmap.
//!
//! And linear coverage is exactly what the distance transform requires: it reads
//! the 128 level as the half-covered pixel and therefore as the contour. Feeding
//! it an sRGB-encoded page would put the contour at roughly 22 % coverage instead
//! of 50 %, which dilates every letter by a fraction of a pixel — heavier stems,
//! filled-in counters in `sad` and `ain`, joins that thicken where a Naskh
//! hairline meets its neighbour. It looks like a font choice rather than a bug,
//! which is what makes it dangerous.
//!
//! So this module never asks for [`NamatRasm::Taghtiya`] and never transforms
//! anything itself. It asks for [`NamatRasm::Masafa`], which is the mode whose
//! documented contract is that linear coverage — before any transfer function —
//! is what reaches [`taarib_saff::rasm::masafa_min_taghtiya`]. The field comes
//! back already transformed and already encoded, and this module's remaining job
//! is purely geometric.
//!
//! ## Supersampling, and why averaging the field is legitimate
//!
//! The source is rasterized at `daqqat_masdar` times the target cell, and the
//! resulting field is area-averaged down. Averaging a *distance field* is sound
//! in a way that averaging coverage is not: the field is very nearly linear in
//! the neighbourhood of the contour, so the mean of a block of samples is a good
//! estimate of the field at the block's centre. Averaging coverage, by contrast,
//! is a box filter — it blurs the edge, and a blurred edge fed to a distance
//! transform yields a contour that has moved.
//!
//! The encoding survives the resample untouched, and this is the reason the
//! source spread is scaled by the same factor as the source size. The field
//! stores `0.5 + 0.5 · d / intishar`. At `k` times the resolution a target
//! distance `d` measures `k · d` source pixels, and the source spread is
//! `k · intishar`, so the ratio — and therefore the stored byte — is identical.
//! No rescaling pass is needed after the average, and none is done: a second
//! quantization to eight bits would cost precision for nothing.
//!
//! ## The tradeoff nobody writes down
//!
//! Three numbers interact and none of them can be chosen alone.
//!
//! **Cell size** (`hajm_khalyia`) is what the atlas stores and what the whole
//! size range is served from. It sets the texel spacing of the field.
//!
//! **Spread** (`intishar`) is how far, in cell pixels, the field carries usable
//! information either side of the outline, and it is what the eight bits are
//! divided across: one encoded level is `2 · intishar / 255` cell pixels. A large
//! spread buys room for outlines and glows and spends precision; a small spread
//! is precise and saturates immediately, leaving a shader nothing to build an
//! outline out of.
//!
//! **Draw size** is what the game asks for. Drawn at `hajm`, the field is
//! magnified by `hajm / hajm_khalyia`. Magnification is what makes one page
//! serve every size — and it is also the limit.
//!
//! The limit is corner rounding, and it is a property of a single-channel field
//! rather than of any parameter. A distance field sampled bilinearly reconstructs
//! a straight edge exactly and a convex corner as an arc, because the true
//! distance function near a corner is not linear and two texels cannot express
//! it. The arc's radius is about one texel of the cell. At a magnification of 2
//! it is invisible. At 4 the sharp terminals of a Kufi `alef` are perceptibly
//! soft. At 8 every corner in the face is rounded and the type looks like a
//! different, softer design — which is precisely the size at which someone
//! reports that the Arabic "looks blurry" and no amount of spread tuning fixes
//! it, because spread is not what is wrong. A cell of
//! [`HAJM_KHALYIA_IFTIRADI`] therefore serves comfortably to about 128 px and
//! honestly to about 256; past that the answer is a larger cell, or coverage
//! pages at the sizes that matter.

use taarib_saff::rasm::{NamatRasm, Rassam, SurahHarf};
use taarib_usus::khata::Natija;

use crate::khata::KhataLawha;

/// The default cell a distance field is stored in, in pixels.
///
/// Thirty-two is where an Arabic face still resolves its own hairlines — a
/// Naskh stem is around two texels at this cell, enough for the transform to
/// place a contour inside it — while a page holds several thousand glyphs.
pub const HAJM_KHALYIA_IFTIRADI: u16 = 32;

/// The default spread, in cell pixels.
pub const INTISHAR_IFTIRADI: f32 = 4.0;

/// The default source resolution, as a multiple of the cell.
pub const DAQQA_IFTIRADIYA: f32 = 4.0;

/// The smallest spread [`intishar_munasib`] will return.
///
/// Below two cell pixels there is not enough gradient for a shader to build an
/// outline or a shadow out of, and both are things games do to text constantly.
pub const INTISHAR_ADNA: f32 = 2.0;

/// The largest spread [`intishar_munasib`] will return.
///
/// Past eight cell pixels, one encoded level is more than a sixteenth of a
/// pixel, and the field around a thin stroke is compressed into so few distinct
/// values that the stroke's own centre stops being distinguishable from its
/// edge.
pub const INTISHAR_AQSA: f32 = 8.0;

/// The largest source resolution multiplier.
const DAQQA_QUSWA: f32 = 16.0;

/// The largest pixel size `taarib-saff` will rasterize at.
///
/// Mirrored here because the cell and the multiplier have to be checked against
/// it *before* they are multiplied together, so that the failure names the
/// multiplier rather than a size the caller never wrote down.
const HAJM_RASM_AQSA: f32 = 1024.0;

/// How much gradient, in screen pixels, a shader needs at the smallest size.
///
/// One pixel is the minimum for an antialiased edge; the half above it is margin
/// for a caller whose "smallest size" turns out to be optimistic.
const TASAMUH_HAFFA: f32 = 1.5;

/// How much of a screen pixel one encoded level may cover at the largest size.
const KHATWA_QUSWA: f32 = 0.5;

/// Tolerance for treating a resolution multiplier as exactly one.
const TASAMUH_KASR: f32 = 1.0e-3;

/// How a distance field is generated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhiyaratMisafa {
    /// The spread, in *cell* pixels — the units of `hajm_khalyia`, not of the
    /// size the game draws at.
    pub intishar: f32,
    /// How many times finer than the cell the source coverage is rasterized.
    ///
    /// One means no supersampling. Four is the default: it costs sixteen times
    /// the rasterization work for a field whose contour is placed to a quarter
    /// of a cell texel, and rasterization is not what a patch compile is
    /// bounded by.
    pub daqqat_masdar: f32,
    /// The cell the field is stored in, in pixels.
    pub hajm_khalyia: u16,
}

impl Default for KhiyaratMisafa {
    fn default() -> Self {
        Self {
            intishar: INTISHAR_IFTIRADI,
            daqqat_masdar: DAQQA_IFTIRADIYA,
            hajm_khalyia: HAJM_KHALYIA_IFTIRADI,
        }
    }
}

impl KhiyaratMisafa {
    /// The options for a patch that draws between two sizes, with the spread
    /// derived from that range by [`intishar_munasib`].
    #[must_use]
    pub fn li_mada(adna_hajm: f32, aqsa_hajm: f32) -> Self {
        Self {
            intishar: intishar_munasib(adna_hajm, aqsa_hajm),
            ..Self::default()
        }
    }

    /// The pixel size the source coverage is rasterized at.
    #[must_use]
    pub fn hajm_masdar(self) -> f32 {
        f32::from(self.hajm_khalyia) * self.daqqat_masdar
    }

    /// The spread, in source pixels, that keeps the encoded field identical
    /// after the resample.
    #[must_use]
    pub fn intishar_masdar(self) -> f32 {
        self.intishar * self.daqqat_masdar
    }
}

/// The spread, in cell pixels, for a patch drawing between two sizes.
///
/// Two forces pull against each other and this function sits between them.
///
/// The **floor** comes from the smallest size. A field stored in a cell of
/// [`HAJM_KHALYIA_IFTIRADI`] pixels and drawn at `adna_hajm` is *minified* by
/// `adna_hajm / cell`; the `intishar` cell pixels of gradient it carries shrink
/// to `intishar · adna_hajm / cell` screen pixels, and a shader needs at least
/// one of those to antialias an edge rather than stair-step it. That gives
/// `intishar ≥ cell / adna_hajm`, taken here with a half-pixel of margin.
///
/// The **ceiling** comes from the largest size, and it is quantization. One
/// encoded level is `2 · intishar / 255` cell pixels; magnified by
/// `aqsa_hajm / cell` it is `2 · intishar · aqsa_hajm / (255 · cell)` screen
/// pixels, and once a single level of the field moves the edge by more than half
/// a screen pixel the antialiasing ramp becomes visibly stepped along near-flat
/// contours — the long horizontal of a `kaf`, the flat top of a `tah`. That
/// gives `intishar ≤ 63.75 · cell / aqsa_hajm`.
///
/// In practice the floor is what decides. The ceiling only falls below it once
/// the top of the range passes roughly sixteen times the cell — world-space
/// text, a title scaled across a cutscene — and that is exactly the case it
/// exists for. The smaller of the two is taken, and then the result is clamped
/// into the range [`INTISHAR_ADNA`] to [`INTISHAR_AQSA`], inclusive. When the two genuinely cross —
/// a patch drawing one font at 6 px and at 600 px — no single field serves both
/// well, and the clamp's hard floor is what decides: it protects the small size,
/// because an edge that aliases at 6 px is a defect on every line of the game
/// while a stepped ramp at 600 px is one letter of one title.
///
/// Non-finite or non-positive inputs, and a pair given the wrong way round, are
/// all repaired rather than refused: this function returns a number for a
/// configuration screen to show, and a configuration screen with a hole in it is
/// worse than one showing the default.
#[must_use]
pub fn intishar_munasib(adna_hajm: f32, aqsa_hajm: f32) -> f32 {
    let khalyia = f32::from(HAJM_KHALYIA_IFTIRADI);
    let awwal = hajm_salih(adna_hajm);
    let thani = hajm_salih(aqsa_hajm);
    let adna = awwal.min(thani);
    let aqsa = awwal.max(thani);

    let lil_saghir = (khalyia / adna) * TASAMUH_HAFFA;
    let lil_kabir = KHATWA_QUSWA * 255.0 * khalyia / (2.0 * aqsa);

    lil_saghir
        .min(lil_kabir)
        .clamp(INTISHAR_ADNA, INTISHAR_AQSA)
}

/// Generates one glyph's distance field, at the cell the options declare.
///
/// The returned [`SurahHarf`] is in **cell** units: its dimensions are cell
/// texels, and its bearings and advance are cell pixels. An adapter drawing at
/// `hajm` scales all three by `hajm / hajm_khalyia`. That is the whole point of
/// a distance-field atlas — one image, every size — and it is why the key that
/// stores it carries the cell size rather than the draw size.
///
/// The subpixel bucket plays no part. A field is placed by the vertex positions
/// of the quad that samples it, at whatever fraction of a pixel the layout
/// produced, and the same texels serve every fraction; rasterizing four shifted
/// copies would put four identical images in the atlas.
/// [`crate::tafrigh::miftah_qiyasi`] is what normalises the key accordingly.
///
/// # Errors
///
/// [`KhataLawha::DaqqaGhayrSaliha`] when the cell is zero, when the multiplier
/// is not at least one and finite, or when the two multiply past what the
/// rasterizer will draw.
///
/// Whatever [`taarib_saff::rasm::Rassam::irsim`] reports otherwise, unchanged:
/// a glyph identifier absent from the font, an outline the parser refuses, a
/// spread that is not a positive number of pixels. Those errors already name the
/// glyph and the font; the caller that also holds a size and a chain index —
/// [`crate::namu::LawhaHayya`] — is the one that wraps them in
/// [`KhataLawha::RasmFashil`], because it is the first layer that knows the
/// whole key.
pub fn masafa_shakl(rassam: &Rassam, muarrif: u32, khiyarat: KhiyaratMisafa) -> Natija<SurahHarf> {
    tahaqquq(khiyarat)?;

    let namat_hadaf = NamatRasm::Masafa {
        intishar: khiyarat.intishar,
    };
    // `Masafa`, never `Taghtiya`: it is the mode that routes *linear* coverage
    // into the distance transform. Asking for coverage here and transforming it
    // afterwards would feed the transform an sRGB-encoded bitmap and dilate
    // every letter in the patch.
    let namat_masdar = NamatRasm::Masafa {
        intishar: khiyarat.intishar_masdar(),
    };
    let surah = rassam.irsim(muarrif, khiyarat.hajm_masdar(), namat_masdar, 0.0, &[])?;

    let daqqa = khiyarat.daqqat_masdar;
    if surah.khali() {
        // A space still has an advance, and that advance came back in *source*
        // pixels. Returning it unscaled would put every space in the patch
        // `daqqat_masdar` times too wide — an error that is invisible in the
        // atlas and obvious in the line.
        return Ok(SurahHarf::farigha(surah.taqaddum / daqqa, namat_hadaf));
    }
    if (daqqa - 1.0).abs() <= TASAMUH_KASR {
        return Ok(SurahHarf {
            namat: namat_hadaf,
            ..surah
        });
    }

    let ard = ila_bud(f32::from(ila_bud_u32(surah.ard)) / daqqa);
    let irtifa = ila_bud(f32::from(ila_bud_u32(surah.irtifa)) / daqqa);
    let bayt = saghghir(&surah.bayt, surah.ard, surah.irtifa, ard, irtifa);

    Ok(SurahHarf {
        ard: u32::from(ard),
        irtifa: u32::from(irtifa),
        // Bearings are whole source pixels and become whole cell pixels, so the
        // placement carries up to half a cell pixel of rounding. That is
        // deliberate and it is why the cell and the multiplier should divide:
        // an image lives at integer texel offsets in a page, and a bearing that
        // kept a fraction would have nowhere to spend it.
        izaha_s: ila_izaha(ila_kasr_i32(surah.izaha_s) / daqqa),
        izaha_a: ila_izaha(ila_kasr_i32(surah.izaha_a) / daqqa),
        taqaddum: surah.taqaddum / daqqa,
        bayt,
        namat: namat_hadaf,
    })
}

/// Refuses a cell and a multiplier that cannot produce a field.
fn tahaqquq(khiyarat: KhiyaratMisafa) -> Natija<()> {
    let daqqa = khiyarat.daqqat_masdar;
    let khalyia = khiyarat.hajm_khalyia;
    let salih = khalyia > 0
        && daqqa.is_finite()
        && (1.0..=DAQQA_QUSWA).contains(&daqqa)
        && f32::from(khalyia) * daqqa <= HAJM_RASM_AQSA;
    if salih {
        Ok(())
    } else {
        Err(KhataLawha::DaqqaGhayrSaliha {
            daqqa,
            hajm_khalyia: khalyia,
        }
        .into())
    }
}

/// Area-averages a distance field down to a smaller grid.
///
/// Every target texel is the mean of the source texels its footprint covers,
/// weighted by how much of each it covers, so a non-integer ratio loses nothing
/// to a nearest-neighbour step. The encoded values need no rescaling: the source
/// spread was scaled by the same factor as the source size, which leaves the
/// stored ratio unchanged.
fn saghghir(masdar: &[u8], ard_m: u32, irtifa_m: u32, ard_h: u16, irtifa_h: u16) -> Vec<u8> {
    let ard_m_h = ila_hajm(ard_m);
    let irtifa_m_h = ila_hajm(irtifa_m);
    let ard_h_h = usize::from(ard_h);
    let irtifa_h_h = usize::from(irtifa_h);
    let hajm = ard_h_h.saturating_mul(irtifa_h_h);
    let hajm_masdar = ard_m_h.saturating_mul(irtifa_m_h);
    if hajm == 0 || hajm_masdar == 0 || masdar.len() != hajm_masdar {
        return vec![0u8; hajm];
    }

    let nisbat_s = ila_kasr_hajm(ard_m_h) / ila_kasr_hajm(ard_h_h);
    let nisbat_a = ila_kasr_hajm(irtifa_m_h) / ila_kasr_hajm(irtifa_h_h);

    let mut natij = vec![0u8; hajm];
    for satr in 0..irtifa_h_h {
        let a0 = ila_kasr_hajm(satr) * nisbat_a;
        let a1 = (a0 + nisbat_a).min(ila_kasr_hajm(irtifa_m_h));
        let satr_awwal = ila_hajm_kasr(a0.floor());
        let satr_akhir = ila_hajm_kasr(a1.ceil()).min(irtifa_m_h);

        for amud in 0..ard_h_h {
            let s0 = ila_kasr_hajm(amud) * nisbat_s;
            let s1 = (s0 + nisbat_s).min(ila_kasr_hajm(ard_m_h));
            let amud_awwal = ila_hajm_kasr(s0.floor());
            let amud_akhir = ila_hajm_kasr(s1.ceil()).min(ard_m_h);

            let mut jam = 0.0f32;
            let mut wazn_kulli = 0.0f32;
            for satr_m in satr_awwal..satr_akhir {
                let wazn_a = tadakhul(a0, a1, satr_m);
                if wazn_a <= 0.0 {
                    continue;
                }
                let bidaya = satr_m.saturating_mul(ard_m_h);
                for amud_m in amud_awwal..amud_akhir {
                    let wazn_s = tadakhul(s0, s1, amud_m);
                    if wazn_s <= 0.0 {
                        continue;
                    }
                    let wazn = wazn_a * wazn_s;
                    let qeema = masdar
                        .get(bidaya.saturating_add(amud_m))
                        .copied()
                        .unwrap_or(0);
                    jam += wazn * f32::from(qeema);
                    wazn_kulli += wazn;
                }
            }
            let makan = satr.saturating_mul(ard_h_h).saturating_add(amud);
            if wazn_kulli > 0.0
                && let Some(khana) = natij.get_mut(makan)
            {
                *khana = ila_bayt(jam / wazn_kulli);
            }
        }
    }
    natij
}

/// How much of source texel `fahras` the interval `min..ila` covers.
fn tadakhul(min: f32, ila: f32, fahras: usize) -> f32 {
    let hadd_adna = ila_kasr_hajm(fahras);
    let hadd_aqsa = hadd_adna + 1.0;
    (ila.min(hadd_aqsa) - min.max(hadd_adna)).max(0.0)
}

// ---------------------------------------------------------------------------
// Numeric conversions
// ---------------------------------------------------------------------------

/// Repairs a size into something the spread formula can divide by.
fn hajm_salih(hajm: f32) -> f32 {
    if hajm.is_finite() && hajm > 0.0 {
        hajm
    } else {
        f32::from(HAJM_KHALYIA_IFTIRADI)
    }
}

/// A resampled dimension, at least one texel.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped into 1.0..=65535.0 before the conversion, so the number \
              converted is a positive whole number inside u16's range"
)]
fn ila_bud(qeema: f32) -> u16 {
    if !qeema.is_finite() {
        return 1;
    }
    (qeema.round().clamp(1.0, f32::from(u16::MAX))) as u16
}

/// A resampled bearing.
///
/// Clamped into `i16` even though [`SurahHarf`] carries an `i32`, because the
/// atlas stores a placed glyph's bearings in an `i16`
/// ([`crate::khareeta::MawdiShakl`]) and a value that fits here and not there
/// would be a glyph whose image is in the page and whose position is not.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is clamped into i16's range on the line it is converted, so the \
              conversion is exact for every input that reaches it"
)]
fn ila_izaha(qeema: f32) -> i32 {
    if !qeema.is_finite() {
        return 0;
    }
    i32::from(
        qeema
            .round()
            .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16,
    )
}

/// The `u16` form of a bitmap dimension, for the `f32::from` that follows it.
fn ila_bud_u32(qeema: u32) -> u16 {
    u16::try_from(qeema).unwrap_or(u16::MAX)
}

/// The `f32` form of a bearing.
fn ila_kasr_i32(qeema: i32) -> f32 {
    f32::from(i16::try_from(qeema).unwrap_or(if qeema < 0 { i16::MIN } else { i16::MAX }))
}

/// The `usize` form of a bitmap dimension.
fn ila_hajm(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(0)
}

/// The `usize` form of a floating index, with anything negative treated as zero.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is an already-floored or already-ceiled coordinate clamped into \
              0.0..=65535.0, so the conversion is exact and non-negative"
)]
fn ila_hajm_kasr(qeema: f32) -> usize {
    if !qeema.is_finite() {
        return 0;
    }
    usize::from(qeema.clamp(0.0, f32::from(u16::MAX)) as u16)
}

/// The `f32` form of a pixel index or count.
#[expect(
    clippy::cast_precision_loss,
    reason = "indices are bounded by the source raster, at most 1024 px on a side, far inside \
              f32's exact integer range"
)]
const fn ila_kasr_hajm(qeema: usize) -> f32 {
    qeema as f32
}

/// Rounds an averaged field value back to the byte a page stores.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the argument is clamped into 0.0..=255.0 and a half added before the cast, so the \
              value converted is a non-negative whole number inside u8's range"
)]
fn ila_bayt(qeema: f32) -> u8 {
    if !qeema.is_finite() {
        return 0;
    }
    (qeema.clamp(0.0, 255.0) + 0.5).min(255.0) as u8
}
