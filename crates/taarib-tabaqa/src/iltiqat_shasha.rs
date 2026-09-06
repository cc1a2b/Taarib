//! الالتقاط — turning a rectangle of backbuffer bytes into something a
//! recognizer can actually read.
//!
//! There is no graphics API in this module. [`crate::wajiha::Khattaf::iltaqit`]
//! has already done the hard part — a staging copy, a map, a row-pitch
//! unpacking — and what arrives here is a tightly packed rectangle in whatever
//! format the game happens to present. Everything after that point is identical
//! on all four APIs, so it is written once, here, where it can be reasoned
//! about without a device in scope.
//!
//! ## Why preprocessing is here and not in `qira`
//!
//! The roadmap puts region cropping, contrast normalisation, background
//! suppression and text-block detection under the recognizer. They are here
//! instead, and the divergence is worth naming rather than leaving for
//! somebody to notice: every one of those steps operates on a captured
//! rectangle and none of them knows or cares which of the three engines will
//! read it. Putting them behind [`crate::qira::Qari`] would mean either
//! three copies — one per engine — or a preprocessing pass the trait
//! performs before dispatching, which is this module with a different name
//! and an engine in scope that has no reason to be.
//!
//! The boundary that results is a clean one: this module turns bytes into a
//! readable image, and `qira` turns a readable image into text.
//!
//! ## The three things that go wrong
//!
//! **The format is not what the recognizer wants.** Every OCR engine in
//! [`crate::qira`] takes RGB or grayscale. Games present BGRA, RGBA, ten-bit
//! RGB, and half-float RGBA. [`SuraMultaqata::ila_rgb`] is the one conversion,
//! and the half-float case is the one that is not a channel shuffle: an HDR
//! frame carries values well above 1.0, and clamping them produces a white
//! rectangle where the dialogue box was. That is not a hypothetical — it is
//! what an HDR game looks like to a capture path that treats 1.0 as white, and
//! the recognizer reads exactly nothing from a white rectangle.
//!
//! **The contrast is not what the recognizer was trained on.** Game text is
//! frequently light on dark, is frequently drawn over a moving background, and
//! is frequently anti-aliased into a drop shadow. [`MuhassinSura`] is five
//! separate steps rather than one `preprocess()` because the control panel
//! shows which ones ran: when a user reports that a region reads nothing, the
//! useful answer is "the inversion did not trigger", not "preprocessing
//! happened".
//!
//! **The text is not one string.** A dialogue box holds a paragraph, and a
//! recognizer that is handed the whole rectangle returns a scatter of word
//! boxes. [`KutalNass`] finds the lines first, by projection profile, so the
//! translator receives sentences instead of fragments — translating a fragment
//! and translating a sentence are different tasks and only one of them produces
//! Arabic a player can read.
//!
//! ## And the thing that goes wrong quietly
//!
//! A static screen still presents sixty frames a second. Without
//! [`MuqayyidMuadal`] the overlay recognizes the same unchanged pause menu
//! sixty times a second, burns a core doing it, and the frame budget in
//! [`crate::wajiha::MeezaniyatItar`] is spent on producing the same string
//! repeatedly. The rate limit is a wall clock minimum, and the perceptual hash
//! is what makes the wall clock minimum rarely matter: an unchanged region is
//! recognized once.

use std::sync::OnceLock;

use crate::khata::{KhataTabaqa, hajm_usize, tul_u64};
use crate::wajiha::{MustatilBiksel, SighatSath};

/// The largest rectangle this build will convert, in pixels.
///
/// A little over 33 megapixels, which covers 8K with room to spare. It exists
/// because [`SuraMultaqata::jadeeda`] is reachable from a swap chain whose
/// reported dimensions came from a driver, and a bad dimension turns into an
/// allocation before it turns into an error anywhere else.
const SAQF_BIKSELAT: u64 = 33_600_000;

// ---------------------------------------------------------------------------
// Numeric conversions, written once
// ---------------------------------------------------------------------------

/// A pixel count as `f32`, with the precision loss stated rather than repeated.
///
/// `u32` to `f32` is exact below 2^24. Every value that reaches this is a
/// surface dimension or a histogram bin count over one region, both of which
/// are orders of magnitude below that.
const fn qeema_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions and per-region counts are below 2^24, where the \
                  conversion is exact"
    )]
    {
        qeema as f32
    }
}

/// A length as `f64`, for the mean and variance arithmetic.
///
/// `usize` to `f64` is exact below 2^53. A region large enough to lose
/// precision here would be forty thousand times the ceiling in
/// [`SAQF_BIKSELAT`].
const fn adad_f64(qeema: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "region pixel counts are capped at SAQF_BIKSELAT, far below 2^53"
    )]
    {
        qeema as f64
    }
}

/// An accumulated sum as `f64`.
///
/// The integral image accumulates in `u64` precisely so the sum cannot wrap;
/// converting it back for the division is where the precision question lands.
/// The largest possible accumulator is 255² × [`SAQF_BIKSELAT`], about 2^41,
/// which `f64` represents exactly.
const fn hajm_f64(qeema: u64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the largest integral-image accumulator is about 2^41, exact in f64"
    )]
    {
        qeema as f64
    }
}

/// A float back to a byte, clamped and rounded.
///
/// Every path that produces a channel value ends here, which is why the clamp
/// is inside the conversion rather than at each call site: a tone-mapped value
/// that came out slightly above 1.0 through floating-point rounding must not
/// wrap to zero.
const fn bayt_min_f32(qeema: f32) -> u8 {
    if qeema.is_nan() {
        return 0;
    }
    let mahdud = qeema.clamp(0.0, 255.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, 255] and rounded on the line above, and NaN returned early"
    )]
    {
        mahdud as u8
    }
}

/// A length as `u32`, saturating rather than wrapping.
fn tul_u32(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// The index a percentile lands on in a sequence of `adad` items.
///
/// Clamped into the sequence rather than allowed to reach `adad`, which is what
/// a percentile of 1.0 would otherwise produce and what would then be an
/// out-of-range partition point.
fn mawdi_miawi(adad: usize, nisba: f64) -> usize {
    if adad == 0 {
        return 0;
    }
    let khaam = adad_f64(adad) * nisba.clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "nisba is clamped into [0, 1] and adad is a real length, so the product is a \
                  non-negative value no larger than adad"
    )]
    let mawdi = khaam as usize;
    mawdi.min(adad.saturating_sub(1))
}

// ---------------------------------------------------------------------------
// Transfer functions
// ---------------------------------------------------------------------------

/// One sRGB-encoded channel value, zero to one, as a linear value.
///
/// The real piecewise curve, not `powf(2.2)`. The difference is largest in the
/// bottom sixteenth of the range, which is exactly where dark game UI lives,
/// and a 2.2 approximation there is off by enough to move a Sauvola threshold
/// across a stroke edge.
fn min_sirgb(qeema: f32) -> f32 {
    if qeema <= 0.040_449_936 { qeema / 12.92 } else { ((qeema + 0.055) / 1.055).powf(2.4) }
}

/// A linear channel value, zero to one, encoded to sRGB.
///
/// The inverse of [`min_sirgb`], with the same objection to a gamma
/// approximation. Values outside the range are clamped by the caller's
/// conversion to a byte rather than here, so that a tone mapper can hand over a
/// value of 1.0000001 without this function pretending it is out of domain.
fn ila_sirgb(qeema: f32) -> f32 {
    if qeema <= 0.003_130_8 { 12.92 * qeema } else { 1.055f32.mul_add(qeema.powf(1.0 / 2.4), -0.055) }
}

/// The 256 sRGB byte values as linear floats, built once.
///
/// [`min_sirgb`] is a `powf` per call and grayscale conversion calls it three
/// times per pixel. On a 1920×200 dialogue-box region that is 1.15 million
/// `powf` calls per capture, which is measurable on the thread that has a
/// two-millisecond budget. The table is 1 KiB.
fn jadwal_khatti() -> &'static [f32; 256] {
    static JADWAL: OnceLock<[f32; 256]> = OnceLock::new();
    JADWAL.get_or_init(|| {
        let mut jadwal = [0.0_f32; 256];
        for (mawdi, khana) in jadwal.iter_mut().enumerate() {
            let bayt = u8::try_from(mawdi).unwrap_or(u8::MAX);
            *khana = min_sirgb(f32::from(bayt) / 255.0);
        }
        jadwal
    })
}

/// One sRGB byte as a linear float, through the table.
fn khatti_min_bayt(qeema: u8) -> f32 {
    jadwal_khatti().get(usize::from(qeema)).copied().unwrap_or(0.0)
}

/// An IEEE 754 binary16 value as an `f32`, exactly.
///
/// Written out rather than taken from a crate, and written without a single
/// floating-point operation, because every shortcut version of this conversion
/// gets one of three cases wrong: subnormals become zero, infinities become a
/// finite number near 131008, or a NaN payload turns into an infinity. An HDR
/// backbuffer contains all three. A sky sample is frequently above the
/// half-float normal range in one channel, a fully occluded pixel is
/// frequently subnormal, and a shader that divided by zero somewhere leaves a
/// NaN that must stay a NaN so the tone mapper's percentile ignores it rather
/// than being dragged to infinity by it.
///
/// The mapping is exact in both directions: every `f32` produced here converts
/// back to the same `u16`.
fn min_nisf(khaam: u16) -> f32 {
    let ishara = u32::from(khaam & 0x8000) << 16;
    let uss = u32::from((khaam >> 10) & 0x001f);
    let kasr = u32::from(khaam & 0x03ff);

    if uss == 0 {
        if kasr == 0 {
            // Signed zero, and the sign is kept: a negative zero in a channel
            // is a legitimate scRGB value and flattening it to +0 would be a
            // conversion that does not round-trip.
            return f32::from_bits(ishara);
        }
        // Subnormal half: the value is `kasr × 2^-24`. Normalising it means
        // finding the highest set bit, which becomes the implicit one.
        let aala = 31_u32.saturating_sub(kasr.leading_zeros());
        let uss_ahadi = aala.saturating_add(103);
        let kasr_ahadi = (kasr << 23_u32.saturating_sub(aala)) & 0x007f_ffff;
        return f32::from_bits(ishara | (uss_ahadi << 23) | kasr_ahadi);
    }

    if uss == 0x1f {
        // Infinity when the mantissa is empty, NaN otherwise. Shifting the
        // mantissa by 13 puts half's quiet bit — bit 9 — onto f32's quiet bit
        // — bit 22 — so a quiet NaN stays quiet and a signalling one stays
        // signalling.
        return f32::from_bits(ishara | 0x7f80_0000 | (kasr << 13));
    }

    // Normal: rebias the exponent from 15 to 127 and widen the mantissa.
    f32::from_bits(ishara | (uss.saturating_add(112) << 23) | (kasr << 13))
}

/// Rec.709 relative luminance of a **linear** RGB triple.
///
/// Linear is not a detail. Applying these coefficients to sRGB-encoded values
/// produces a number that is not luminance and is wrong in a direction that
/// matters: it overweights the dark end, so pale text on a mid-grey panel comes
/// out with less separation than it has.
fn idaa(ahmar: f32, akhdar: f32, azraq: f32) -> f32 {
    0.072_2f32.mul_add(azraq, 0.212_6f32.mul_add(ahmar, 0.715_2 * akhdar))
}

// ---------------------------------------------------------------------------
// The captured rectangle
// ---------------------------------------------------------------------------

/// A rectangle of the presented frame, as the backend handed it over.
///
/// Owns its bytes. The alternative — a borrow of the backend's staging buffer —
/// was tried and abandoned: recognition happens on another thread so the
/// overlay's draw is not blocked by it, and a borrow of a mapped D3D resource
/// cannot outlive the frame it was mapped in.
#[derive(Debug, Clone)]
pub struct SuraMultaqata {
    bayt: Vec<u8>,
    ard: u32,
    irtifa: u32,
    sigha: SighatSath,
    mintaqa: MustatilBiksel,
}

impl SuraMultaqata {
    /// Wraps captured bytes, checking that they are the size they claim to be.
    ///
    /// The check is not defensive noise. A backend that unpacked row pitch
    /// incorrectly produces a buffer that is a plausible size and reads as a
    /// sheared image — every row shifted a few pixels further right than the
    /// last — which the recognizer reports as "no text" rather than as an
    /// error. Comparing the length against the geometry catches the whole
    /// family of pitch mistakes at the boundary instead of at the symptom.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IltiqatFashil`] when a dimension is zero, when the
    /// rectangle disagrees with the stated width and height, or when the byte
    /// count is not exactly what the format and geometry require — naming both
    /// the length that was given and the length that was needed, because a
    /// report that says only "wrong size" leaves the reader to work out which
    /// of the two numbers is the surprising one.
    ///
    /// [`KhataTabaqa::HajmMufrit`] when the rectangle is larger than this build
    /// converts, refused before the allocation rather than after it.
    pub fn jadeeda(
        bayt: Vec<u8>,
        ard: u32,
        irtifa: u32,
        sigha: SighatSath,
        mintaqa: MustatilBiksel,
    ) -> Result<Self, KhataTabaqa> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the captured rectangle is {ard}×{irtifa}, which has no pixels to read"
                ),
            });
        }
        if mintaqa.ard != ard || mintaqa.irtifa != irtifa {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the capture is {ard}×{irtifa} but the region it came from is {}×{}",
                    mintaqa.ard, mintaqa.irtifa
                ),
            });
        }

        let bikselat = u64::from(ard).saturating_mul(u64::from(irtifa));
        if bikselat > SAQF_BIKSELAT {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "captured pixels",
                qeema: bikselat,
                saqf: SAQF_BIKSELAT,
            });
        }

        let matlub = bikselat.saturating_mul(u64::from(sigha.bayt_lil_biksel()));
        let mawjud = tul_u64(bayt.len());
        if mawjud != matlub {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the capture carries {mawjud} byte(s) but {ard}×{irtifa} in {} needs \
                     exactly {matlub}",
                    sigha.ism()
                ),
            });
        }

        Ok(Self { bayt, ard, irtifa, sigha, mintaqa })
    }

    /// The captured bytes, in the surface's own format.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }

    /// Width in pixels.
    #[must_use]
    pub const fn ard(&self) -> u32 {
        self.ard
    }

    /// Height in pixels.
    #[must_use]
    pub const fn irtifa(&self) -> u32 {
        self.irtifa
    }

    /// The format the bytes are in.
    #[must_use]
    pub const fn sigha(&self) -> SighatSath {
        self.sigha
    }

    /// Where on the surface this came from.
    ///
    /// Carried so that a rectangle the recognizer reports inside this image can
    /// be moved back onto the surface without the caller having to remember
    /// which region it asked for. See [`SuraMultaqata::ila_sath`].
    #[must_use]
    pub const fn mintaqa(&self) -> MustatilBiksel {
        self.mintaqa
    }

    /// A rectangle in this image's coordinates, moved onto the surface.
    ///
    /// The overlay draws Arabic where the original text was, so every box the
    /// recognizer returns has to make this trip. Doing it here rather than at
    /// each recognizer means the three engines in [`crate::qira`] all report in
    /// image coordinates and none of them needs to know where the capture came
    /// from.
    #[must_use]
    pub const fn ila_sath(&self, mahalli: MustatilBiksel) -> MustatilBiksel {
        MustatilBiksel {
            yasar: self.mintaqa.yasar.saturating_add(mahalli.yasar),
            aala: self.mintaqa.aala.saturating_add(mahalli.aala),
            ard: mahalli.ard,
            irtifa: mahalli.irtifa,
        }
    }

    /// The capture as tightly packed RGB8, three bytes per pixel.
    ///
    /// Three channels rather than four because that is what every recognizer in
    /// [`crate::qira`] takes: `ocrs`'s `ImageSource::from_bytes` expects RGB and
    /// silently misreads RGBA as a shifted RGB, which produces a red-tinted
    /// image the detector finds no text in.
    ///
    /// The four formats divide into two jobs. Three of them are a rearrangement
    /// — swap two channels, drop an alpha, unpack ten bits into eight — and
    /// carry values that are already sRGB-encoded, so nothing about their
    /// brightness changes here.
    ///
    /// [`SighatSath::Rgba16f`] is the other job. Its values are linear and
    /// unbounded: an HDR game routinely presents a specular highlight at 8.0
    /// and a sky at 20.0 while the dialogue box's white text sits near 1.0.
    /// Clamping that to `[0, 1]` and encoding gives a rectangle in which the
    /// text and the panel behind it and the sky beyond it are all 255 — one
    /// flat white area with no edges, from which the recognizer reads nothing
    /// and reports nothing, which is indistinguishable from a region with no
    /// text in it. So the range is *compressed* rather than cut: extended
    /// Reinhard with the white point taken from the region's own 99th
    /// percentile luminance, which keeps the text's contrast against its panel
    /// while pulling the highlights down into range. Taking the white point
    /// from the region rather than from a constant matters because the whole
    /// point of a per-region capture is that the dialogue box is dark and the
    /// sky is not; a fixed white point tuned for the sky flattens the box.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HajmMufrit`] when the RGB result would not fit this
    /// target's address space, which is checked rather than allowed to abort
    /// inside an allocator on the game's render thread.
    pub fn ila_rgb(&self) -> Result<Vec<u8>, KhataTabaqa> {
        let tul = u64::from(self.ard)
            .saturating_mul(u64::from(self.irtifa))
            .saturating_mul(3);
        let Some(sia) = hajm_usize(tul) else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "RGB conversion buffer",
                qeema: tul,
                saqf: tul_u64(usize::MAX),
            });
        };
        let mut mukhraj = Vec::with_capacity(sia);

        match self.sigha {
            SighatSath::Bgra8 => {
                for biksel in self.bayt.chunks_exact(4) {
                    let azraq = biksel.first().copied().unwrap_or(0);
                    let akhdar = biksel.get(1).copied().unwrap_or(0);
                    let ahmar = biksel.get(2).copied().unwrap_or(0);
                    mukhraj.extend_from_slice(&[ahmar, akhdar, azraq]);
                }
            }
            SighatSath::Rgba8 => {
                for biksel in self.bayt.chunks_exact(4) {
                    let ahmar = biksel.first().copied().unwrap_or(0);
                    let akhdar = biksel.get(1).copied().unwrap_or(0);
                    let azraq = biksel.get(2).copied().unwrap_or(0);
                    mukhraj.extend_from_slice(&[ahmar, akhdar, azraq]);
                }
            }
            SighatSath::Rgb10a2 => {
                for biksel in self.bayt.chunks_exact(4) {
                    let hazma = u32::from_le_bytes([
                        biksel.first().copied().unwrap_or(0),
                        biksel.get(1).copied().unwrap_or(0),
                        biksel.get(2).copied().unwrap_or(0),
                        biksel.get(3).copied().unwrap_or(0),
                    ]);
                    mukhraj.extend_from_slice(&[
                        ashra_ila_thamania(hazma & 0x3ff),
                        ashra_ila_thamania((hazma >> 10) & 0x3ff),
                        ashra_ila_thamania((hazma >> 20) & 0x3ff),
                    ]);
                }
            }
            SighatSath::Rgba16f => {
                let khatti = self.ila_khatti_hdr();
                let bayda = nuqtat_bayda(&khatti);
                for biksel in khatti.chunks_exact(3) {
                    let ahmar = biksel.first().copied().unwrap_or(0.0);
                    let akhdar = biksel.get(1).copied().unwrap_or(0.0);
                    let azraq = biksel.get(2).copied().unwrap_or(0.0);
                    mukhraj.extend_from_slice(&[
                        bayt_min_f32(ila_sirgb(daght_mada(ahmar, bayda)) * 255.0),
                        bayt_min_f32(ila_sirgb(daght_mada(akhdar, bayda)) * 255.0),
                        bayt_min_f32(ila_sirgb(daght_mada(azraq, bayda)) * 255.0),
                    ]);
                }
            }
        }

        Ok(mukhraj)
    }

    /// The half-float capture decoded to linear `f32` RGB, alpha discarded.
    ///
    /// Split out because the tone mapper needs two passes over the same values
    /// — one to find the white point and one to apply it — and decoding twice
    /// would be decoding four half-floats per pixel twice.
    fn ila_khatti_hdr(&self) -> Vec<f32> {
        let sia = self.bayt.len().saturating_mul(3).checked_div(8).unwrap_or(0);
        let mut khatti = Vec::with_capacity(sia);
        for biksel in self.bayt.chunks_exact(8) {
            let qanat = |mawdi: usize| -> f32 {
                let adna = biksel.get(mawdi).copied().unwrap_or(0);
                let aala = biksel.get(mawdi.saturating_add(1)).copied().unwrap_or(0);
                min_nisf(u16::from_le_bytes([adna, aala]))
            };
            khatti.push(qanat(0));
            khatti.push(qanat(2));
            khatti.push(qanat(4));
        }
        khatti
    }
}

/// A ten-bit channel value as an eight-bit one.
///
/// `(q × 255 + 511) / 1023`, which is round-to-nearest rather than the truncating
/// shift-by-two that most conversions use. The shift is off by up to one part
/// in 255 at every value, which is invisible in a picture and is not invisible
/// in a histogram stretch that then multiplies the error up.
fn ashra_ila_thamania(qeema: u32) -> u8 {
    let mawzun = qeema.saturating_mul(255).saturating_add(511);
    let natija = mawzun.checked_div(1023).unwrap_or(0);
    u8::try_from(natija).unwrap_or(u8::MAX)
}

/// The 99th percentile of a linear RGB buffer's luminance.
///
/// The percentile, not the maximum. One blown-out pixel — a sun, a muzzle
/// flash, a bloom sample that came back at 3000 — would define the white point
/// as 3000 and map everything else, including all the text, into the bottom
/// thousandth of the range. Taking the value that 99% of the region is below
/// leaves that pixel to clip, which is correct: it is one pixel and it is not
/// text.
///
/// Non-finite samples are dropped rather than sorted. A NaN compares as neither
/// smaller nor larger than anything, and leaving it in the buffer being
/// partitioned makes the result depend on where in the buffer it happened to
/// be.
fn nuqtat_bayda(khatti: &[f32]) -> f32 {
    let mut idaat: Vec<f32> = khatti
        .chunks_exact(3)
        .filter_map(|biksel| {
            let ahmar = biksel.first().copied().unwrap_or(0.0);
            let akhdar = biksel.get(1).copied().unwrap_or(0.0);
            let azraq = biksel.get(2).copied().unwrap_or(0.0);
            let qeema = idaa(ahmar, akhdar, azraq);
            if qeema.is_finite() { Some(qeema.max(0.0)) } else { None }
        })
        .collect();

    if idaat.is_empty() {
        // Everything in the region was NaN or infinite, which is a game that
        // presented a broken frame. One is the sRGB white point and produces a
        // plain clamp, which is the correct behaviour when there is no usable
        // information to derive anything better from.
        return 1.0;
    }

    let mawdi = mawdi_miawi(idaat.len(), 0.99);
    let (_, qeema, _) = idaat.select_nth_unstable_by(mawdi, f32::total_cmp);
    // Below the sRGB white point the compression would darken a frame that was
    // already in range, so the white point never drops under one.
    qeema.max(1.0)
}

/// Extended Reinhard: `x · (1 + x/w²) / (1 + x)`.
///
/// Chosen over the plain `x/(1+x)` for one property that matters to a
/// recognizer rather than to a viewer: at `x = w` the result is exactly 1.0, so
/// the white point maps to white and everything below it keeps a monotone,
/// smooth ramp with no flat region. Filmic curves look better and have a toe
/// that crushes the low end, which is where dark UI panels live and where the
/// separation between a panel and the text on it has to survive.
fn daght_mada(qeema: f32, bayda: f32) -> f32 {
    if !qeema.is_finite() {
        // An infinity is a highlight that overflowed the half-float range, and
        // it is white. A NaN is a shader bug and is black, which is at least
        // consistent between frames.
        return if qeema.is_nan() { 0.0 } else { 1.0 };
    }
    let musba = qeema.max(0.0);
    let bayda_murabbaa = (bayda * bayda).max(f32::MIN_POSITIVE);
    (musba * (1.0 + musba / bayda_murabbaa) / (1.0 + musba)).clamp(0.0, 1.0)
}

/// A dimension as an index, saturating rather than wrapping.
///
/// `u32` fits `usize` on every target this product builds for; the fallible
/// form is used anyway because the `as` would be a cast, and a cast in the
/// middle of index arithmetic is the one place a silent wrap turns into a wrong
/// pixel rather than into a compile error.
fn mawdi_usize(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}

// ---------------------------------------------------------------------------
// Single-channel images
// ---------------------------------------------------------------------------

/// A one-byte-per-pixel image, row-major and tightly packed.
///
/// Every preprocessing step in [`MuhassinSura`] takes one of these and returns
/// one, so the steps compose in any order the control panel offers and none of
/// them has to carry a stride.
#[derive(Debug, Clone)]
pub struct SuratRamadiya {
    bayt: Vec<u8>,
    ard: u32,
    irtifa: u32,
}

impl SuratRamadiya {
    /// Wraps single-channel bytes, checking the geometry.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IltiqatFashil`] when a dimension is zero or when the byte
    /// count is not exactly `ard × irtifa`, naming both numbers.
    pub fn jadeeda(bayt: Vec<u8>, ard: u32, irtifa: u32) -> Result<Self, KhataTabaqa> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!("a {ard}×{irtifa} grayscale image has no pixels"),
            });
        }
        let matlub = u64::from(ard).saturating_mul(u64::from(irtifa));
        let mawjud = tul_u64(bayt.len());
        if mawjud != matlub {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the grayscale buffer carries {mawjud} byte(s) but {ard}×{irtifa} needs \
                     exactly {matlub}"
                ),
            });
        }
        Ok(Self { bayt, ard, irtifa })
    }

    /// The pixels.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }

    /// Width in pixels.
    #[must_use]
    pub const fn ard(&self) -> u32 {
        self.ard
    }

    /// Height in pixels.
    #[must_use]
    pub const fn irtifa(&self) -> u32 {
        self.irtifa
    }

    /// The rows, each exactly [`SuratRamadiya::ard`] bytes long.
    ///
    /// `chunks_exact` rather than an index loop, and it is the reason no step
    /// in this module computes `y * ard + x` in an inner loop: the compiler
    /// bounds-checks a chunk once instead of once per pixel, and there is no
    /// arithmetic left that could produce a row-shifted image.
    pub fn sufuf(&self) -> impl Iterator<Item = &[u8]> {
        self.bayt.chunks_exact(mawdi_usize(self.ard).max(1))
    }

    /// One pixel, or zero when the coordinates are outside the image.
    ///
    /// Zero rather than a wrap, because every caller of this is sampling a
    /// neighbourhood at an edge and "outside is black" is the behaviour those
    /// callers want. The alternative — clamping to the edge pixel — is
    /// implemented separately in [`MuhassinSura::kabbir`], where it is correct
    /// and here it would not be.
    #[must_use]
    pub fn qeema(&self, x: u32, y: u32) -> u8 {
        if x >= self.ard || y >= self.irtifa {
            return 0;
        }
        let mawdi = mawdi_usize(y).saturating_mul(mawdi_usize(self.ard)).saturating_add(
            mawdi_usize(x),
        );
        self.bayt.get(mawdi).copied().unwrap_or(0)
    }

    /// The mean pixel value, zero to 255.
    #[must_use]
    pub fn mutawassit(&self) -> f64 {
        if self.bayt.is_empty() {
            return 0.0;
        }
        let majmu: u64 = self.bayt.iter().map(|&q| u64::from(q)).sum();
        hajm_f64(majmu) / adad_f64(self.bayt.len())
    }

    /// This image as a capture a recognizer will accept.
    ///
    /// The gap this closes is not a convenience. [`MuhassinSura::hassin`]
    /// produces one of these and every engine behind [`crate::qira::Qari`]
    /// takes a [`SuraMultaqata`], so without a conversion the two halves of the
    /// pipeline cannot be joined: a caller that preprocesses a region has no
    /// way to hand the result to a reader, and a caller that reads a region is
    /// reading the raw backbuffer with none of the preprocessing applied. The
    /// second is what happens by default, silently, because both calls compile.
    ///
    /// The grey value goes into all three colour channels rather than into one.
    /// `ImageSource::from_bytes` derives its stride from the buffer length, so
    /// a single-channel buffer of this geometry is also a *valid* RGB buffer of
    /// a third the width — it does not error, it reads as a sheared image with
    /// no text in it.
    ///
    /// The rectangle the result reports is its own frame, at the origin, not a
    /// position on the surface: preprocessing may have upscaled, so this
    /// image's coordinates are no longer the surface's. [`SuraMuhassana`] is
    /// what carries the trip back, and it is what
    /// [`MuhassinSura::hassin_lil_qari`] returns for exactly that reason.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HajmMufrit`] when four bytes per pixel would not fit this
    /// target's address space, refused before the allocation.
    pub fn ila_multaqata(&self) -> Result<SuraMultaqata, KhataTabaqa> {
        let tul = tul_u64(self.bayt.len()).saturating_mul(4);
        let Some(sia) = hajm_usize(tul) else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "grayscale-to-RGBA buffer",
                qeema: tul,
                saqf: tul_u64(usize::MAX),
            });
        };
        let mut bayt = Vec::with_capacity(sia);
        for &qeema in &self.bayt {
            bayt.extend_from_slice(&[qeema, qeema, qeema, 255]);
        }
        SuraMultaqata::jadeeda(
            bayt,
            self.ard,
            self.irtifa,
            SighatSath::Rgba8,
            MustatilBiksel { yasar: 0, aala: 0, ard: self.ard, irtifa: self.irtifa },
        )
    }
}

/// A preprocessed region, ready to read, and the way back to the surface.
///
/// Preprocessing changes an image's size — [`MuhassinSura::kabbir`] upscales a
/// short region by an integer factor — so a rectangle a recognizer reports
/// inside the preprocessed image is not a rectangle on the game's surface. Two
/// numbers are needed to make the trip and neither of them survives in a bare
/// [`SuraMultaqata`]: the factor, and the region the capture came from. Both
/// are here, so the overlay draws Arabic where the English was rather than at
/// two or three times the offset.
#[derive(Debug, Clone)]
pub struct SuraMuhassana {
    sura: SuraMultaqata,
    mintaqa: MustatilBiksel,
    mudaaf: u32,
}

impl SuraMuhassana {
    /// The preprocessed image, in the form a recognizer takes.
    #[must_use]
    pub const fn sura(&self) -> &SuraMultaqata {
        &self.sura
    }

    /// Where the original capture sat on the surface.
    #[must_use]
    pub const fn mintaqa(&self) -> MustatilBiksel {
        self.mintaqa
    }

    /// The integer factor preprocessing upscaled by; one when it did not.
    #[must_use]
    pub const fn mudaaf(&self) -> u32 {
        self.mudaaf
    }

    /// A rectangle in the preprocessed image, moved back onto the surface.
    ///
    /// Divides by the upscale factor before offsetting, in that order. The
    /// other order gives a box that is correct only when the region starts at
    /// the origin, which is true of exactly one region on any screen and is why
    /// the mistake survives a first test.
    #[must_use]
    pub fn ila_sath(&self, mahalli: MustatilBiksel) -> MustatilBiksel {
        let musaghghar = self.ila_iltiqat(mahalli);
        MustatilBiksel {
            yasar: self.mintaqa.yasar.saturating_add(musaghghar.yasar),
            aala: self.mintaqa.aala.saturating_add(musaghghar.aala),
            ..musaghghar
        }
    }

    /// A rectangle in the preprocessed image, moved back into the **capture's**
    /// coordinates — the factor divided out, the region's origin not added.
    ///
    /// Two destinations exist and they are one offset apart, which is exactly
    /// the kind of difference that compiles either way.
    /// [`SuraMuhassana::ila_sath`] is for drawing, because the overlay draws on
    /// the surface. This one is for [`crate::qira::SatrMaqru::mawdi`], which
    /// that field's own documentation defines as the captured image's
    /// coordinates so that a recognizer never has to know where the capture came
    /// from. A recognizer handed the preprocessed image reports boxes in a third
    /// space — the upscaled one — and without this the boxes are silently two or
    /// three times too large and too far right.
    #[must_use]
    pub fn ila_iltiqat(&self, mahalli: MustatilBiksel) -> MustatilBiksel {
        let mudaaf = if self.mudaaf == 0 { 1 } else { self.mudaaf };
        let asghar = |qeema: u32| -> u32 { qeema.checked_div(mudaaf).unwrap_or(qeema) };
        MustatilBiksel {
            yasar: asghar(mahalli.yasar),
            aala: asghar(mahalli.aala),
            ard: asghar(mahalli.ard),
            irtifa: asghar(mahalli.irtifa),
        }
    }
}

// ---------------------------------------------------------------------------
// Preprocessing
// ---------------------------------------------------------------------------

/// The knobs on [`MuhassinSura`], all of them exposed in the control panel.
///
/// Exposed rather than tuned and hidden because the right values genuinely
/// differ between a visual novel with 40px text on a flat panel and a strategy
/// game with 11px labels over terrain, and a user who can see that inversion
/// fired when it should not have can turn it off for that one game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdadatTahsin {
    /// The low percentile the histogram stretch anchors black to.
    pub nisba_dunya: f64,
    /// The high percentile it anchors white to.
    ///
    /// Not the maximum, for the same reason [`nuqtat_bayda`] is not the
    /// maximum: a single specular pixel in the corner of a region would
    /// otherwise define white and compress all the text into the bottom of the
    /// range.
    pub nisba_ulya: f64,
    /// How many pixels deep the border ring the inversion test samples is.
    pub sumk_itar: u32,
    /// How much brighter than the border the interior must be before the image
    /// is inverted, in the zero-to-255 scale.
    ///
    /// Without a margin a flat region flips on sensor-level noise, and a region
    /// that flips on some frames and not others produces two different
    /// recognitions of the same screen.
    pub hamish_aks: f64,
    /// Half the side of the Sauvola window, in pixels.
    pub nisf_nafidha: u32,
    /// Sauvola's `k`. Larger removes more background and thins strokes.
    pub muamil_sauvola: f64,
    /// Sauvola's `R`, the dynamic range of the standard deviation.
    pub mada_sauvola: f64,
    /// Whether background suppression runs at all.
    ///
    /// Off by default, and that default is a judgement worth stating: Sauvola
    /// was the right answer for the OCR engines of fifteen years ago, and both
    /// engines this crate ships to are neural and were trained on grayscale
    /// photographs. Binarizing before them throws away the anti-aliasing they
    /// use to resolve a stroke. It stays available because it is the thing that
    /// rescues text over a busy, moving background, which is the one case where
    /// the neural detectors do fail.
    pub yubaddil_thunai: bool,
    /// The height below which the **text** is upscaled, in pixels.
    ///
    /// The text's height, measured by [`KutalNass`], not the region's. Those
    /// are different numbers, and comparing against the second one made this
    /// setting inert: over a 360-case corpus with 11-pixel text inside regions
    /// 27 to 29 pixels tall, the upscale it governs fired zero times.
    pub irtifa_adna: u32,
    /// The largest integer upscale factor.
    pub aqsa_takbir: u32,
    /// Whether the recognizer is handed grayscale rather than colour.
    ///
    /// Off, and that default is the one decision in this struct that was made
    /// from a measurement rather than from an argument. Over a 360-case corpus
    /// — six conditions, three faces, twenty lines, composited over text-free
    /// crops of real game frames — collapsing to grayscale before the portable
    /// recognizer moved exact-match accuracy like this:
    ///
    /// | condition | colour | grayscale chain |
    /// | --- | --- | --- |
    /// | clean | 91.7% | 93.3% |
    /// | outlined | 73.3% | 78.3% |
    /// | shadowed | 91.7% | 91.7% |
    /// | low-contrast | 90.0% | 93.3% |
    /// | over-texture | 63.3% | 51.7% |
    /// | small | 86.7% | 83.3% |
    /// | all | 82.8% | 81.9% |
    ///
    /// The grayscale chain wins on flat panels and loses badly on text over
    /// game art — eleven and a half points, and character error rate more than
    /// doubles, from 4.07% to 9.46%. Text over game art is the case tier 3
    /// exists for: the tiers that can patch a game are used when the game can
    /// be patched, and what is left for the overlay is arbitrary art behind
    /// arbitrary text. So colour is the default and the chain is an option a
    /// user can turn on for a game whose UI is flat panels.
    ///
    /// Turning it on also turns on the contrast stretch and the inversion,
    /// which have no meaning on a colour buffer. The measurement image is built
    /// either way — the upscale factor and the change-detection hash both come
    /// from it — and this only decides which of the two reaches the reader.
    ///
    /// It has a second job that is not a quality knob, and a caller who removes
    /// this field for being a mere preference would break it:
    /// [`crate::qira::IkhtiyarQari::iqra_mufattasha`] reads each region twice
    /// with this flag set **both** ways, and [`crate::qira::hukm_tawafuq`]
    /// refuses the region when the two reads disagree. Colour and the grayscale
    /// chain are the only two paths in this crate that hand the recognizer
    /// genuinely different pixels — over 837 real region crops they returned
    /// different text on 379 — and that difference is what makes corroboration
    /// able to refuse anything at all.
    pub yuhawwil_ila_ramadi: bool,
}

impl Default for IdadatTahsin {
    fn default() -> Self {
        Self {
            nisba_dunya: 0.02,
            nisba_ulya: 0.98,
            sumk_itar: 2,
            hamish_aks: 6.0,
            nisf_nafidha: 12,
            muamil_sauvola: 0.34,
            mada_sauvola: 128.0,
            yubaddil_thunai: false,
            // Twenty pixels is where recognition accuracy falls off a cliff on
            // both engines. No margin is added on top: this is compared against
            // the text's own measured height now, so a margin would only make
            // the step fire on text that does not need it.
            irtifa_adna: 20,
            aqsa_takbir: 3,
            yuhawwil_ila_ramadi: false,
        }
    }
}

/// The five preprocessing steps, and a record of which of them did anything.
///
/// Each step is a method rather than a stage inside one `preprocess()` for a
/// reason that is about support rather than about design: when a user reports
/// that one region reads nothing, the first useful question is which steps ran
/// and what they decided, and [`MuhassinSura::athar`] answers it in the control
/// panel without a debug build.
#[derive(Debug, Clone)]
pub struct MuhassinSura {
    iadadat: IdadatTahsin,
    athar: Vec<String>,
}

impl MuhassinSura {
    /// A preprocessor with the given settings.
    #[must_use]
    pub const fn jadeed(iadadat: IdadatTahsin) -> Self {
        Self { iadadat, athar: Vec::new() }
    }

    /// The settings.
    #[must_use]
    pub const fn iadadat(&self) -> &IdadatTahsin {
        &self.iadadat
    }

    /// Changes the settings.
    pub const fn ayyin(&mut self, iadadat: IdadatTahsin) {
        self.iadadat = iadadat;
    }

    /// What each step decided, most recent last.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Clears the trail, which the caller does once per capture.
    pub fn imsah_athar(&mut self) {
        self.athar.clear();
    }

    /// Rec.709 luminance of linearised RGB, re-encoded to sRGB for storage.
    ///
    /// Two decisions are packed into one line of arithmetic and both are worth
    /// naming.
    ///
    /// **Linearise first.** `(r + g + b) / 3` on sRGB bytes — or worse, the
    /// same average with the Rec.709 weights applied to encoded values — is the
    /// standard grayscale conversion and it is wrong. sRGB encoding is roughly
    /// a 1/2.2 power, so encoded values are compressed at the top and stretched
    /// at the bottom; weighting them as if they were linear underweights green
    /// where green carries most of the luminance. The visible consequence is
    /// specific: pale text on a saturated blue or red panel — a health warning,
    /// a faction colour — loses most of its separation from the panel and the
    /// binarizer then puts the stroke and the background in the same class. The
    /// text vanishes.
    ///
    /// **Re-encode after.** The result is stored sRGB-encoded rather than
    /// linear because every step after this one — the percentile stretch, the
    /// Sauvola threshold, the projection profile — is a *perceptual* judgement
    /// about what a reader would call dark, and sRGB is the encoding in which
    /// equal steps are roughly equal perceptual steps. A linear grayscale image
    /// fed to Sauvola produces thresholds biased hard toward the shadows.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IltiqatFashil`] when `rgb` is not exactly
    /// `ard × irtifa × 3` bytes, which means a conversion upstream produced a
    /// buffer that does not match its own geometry.
    pub fn ila_ramadi(
        &mut self,
        rgb: &[u8],
        ard: u32,
        irtifa: u32,
    ) -> Result<SuratRamadiya, KhataTabaqa> {
        let matlub = u64::from(ard).saturating_mul(u64::from(irtifa)).saturating_mul(3);
        let mawjud = tul_u64(rgb.len());
        if mawjud != matlub {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the RGB buffer carries {mawjud} byte(s) but {ard}×{irtifa} RGB needs \
                     exactly {matlub}"
                ),
            });
        }

        let mut ramadi = Vec::with_capacity(rgb.len().checked_div(3).unwrap_or(0));
        for biksel in rgb.chunks_exact(3) {
            let ahmar = khatti_min_bayt(biksel.first().copied().unwrap_or(0));
            let akhdar = khatti_min_bayt(biksel.get(1).copied().unwrap_or(0));
            let azraq = khatti_min_bayt(biksel.get(2).copied().unwrap_or(0));
            ramadi.push(bayt_min_f32(ila_sirgb(idaa(ahmar, akhdar, azraq)) * 255.0));
        }

        self.athar.push(format!(
            "grayscale: Rec.709 luminance on linearised channels, {ard}×{irtifa}"
        ));
        SuratRamadiya::jadeeda(ramadi, ard, irtifa)
    }

    /// Stretches contrast between two percentiles of the histogram.
    ///
    /// Percentiles rather than the minimum and maximum. A region very often
    /// contains one pixel at 0 and one at 255 — a hard panel border, a cursor,
    /// a compression artefact — and anchoring to those two means the stretch
    /// does nothing at all, on exactly the regions that most need it. Anchoring
    /// to the 2nd and 98th throws away four percent of the pixels and gains the
    /// whole range for the ninety-six percent that are the panel and the text.
    ///
    /// Returns whether the image was changed. A region that is already using
    /// its full range is left alone rather than run through an identity map,
    /// and the control panel says so.
    pub fn sawi_tabayun(&mut self, surah: &mut SuratRamadiya) -> bool {
        let mut tawzee = [0_u64; 256];
        for &qeema in &surah.bayt {
            if let Some(khana) = tawzee.get_mut(usize::from(qeema)) {
                *khana = khana.saturating_add(1);
            }
        }

        let adad = surah.bayt.len();
        let hadaf_dunya = tul_u64(mawdi_miawi(adad, self.iadadat.nisba_dunya));
        let hadaf_ulya = tul_u64(mawdi_miawi(adad, self.iadadat.nisba_ulya));

        let mut jari = 0_u64;
        let mut qaa = 0_u8;
        let mut saqf = 255_u8;
        let mut wujid_qaa = false;
        for (bin, &adad_bin) in tawzee.iter().enumerate() {
            jari = jari.saturating_add(adad_bin);
            let qeema = u8::try_from(bin).unwrap_or(u8::MAX);
            if !wujid_qaa && jari > hadaf_dunya {
                qaa = qeema;
                wujid_qaa = true;
            }
            if jari > hadaf_ulya {
                saqf = qeema;
                break;
            }
        }

        if saqf <= qaa {
            self.athar.push(format!(
                "contrast: left alone, the {}th and {}th percentiles are both {qaa}, which is a \
                 flat region",
                (self.iadadat.nisba_dunya * 100.0).round(),
                (self.iadadat.nisba_ulya * 100.0).round()
            ));
            return false;
        }
        if qaa == 0 && saqf == 255 {
            self.athar.push("contrast: left alone, the region already uses the full range"
                .to_owned());
            return false;
        }

        let asfal = f32::from(qaa);
        let mada = f32::from(saqf) - asfal;
        for khana in &mut surah.bayt {
            let mumaddad = (f32::from(*khana) - asfal) * 255.0 / mada;
            *khana = bayt_min_f32(mumaddad);
        }
        self.athar.push(format!("contrast: stretched {qaa}..={saqf} onto 0..=255"));
        true
    }

    /// Inverts the image when the text is light on a dark background.
    ///
    /// Every OCR engine worth shipping was trained on scanned documents, which
    /// means dark strokes on a light page. Game UI is the opposite roughly half
    /// the time. Both engines in [`crate::qira`] degrade on inverted text —
    /// they do not fail outright, which is worse, because the failure mode is a
    /// plausible wrong word rather than an empty result.
    ///
    /// The decision compares the mean of a border ring against the mean of the
    /// interior, and the reasoning is that a region a user drew around a
    /// dialogue box has background at its edges and text in its middle. If the
    /// middle is brighter than the edge, the strokes are the bright thing and
    /// the image is inverted. A margin keeps a uniform region from flipping on
    /// noise and, more importantly, from flipping on some frames and not others
    /// — a region that alternates produces two different recognitions of one
    /// unchanged screen and defeats the hash gate in [`MuqayyidMuadal`]
    /// completely.
    ///
    /// Returns whether the image was inverted.
    pub fn aksi_idha_lazim(&mut self, surah: &mut SuratRamadiya) -> bool {
        let Some((itar, dakhil)) = self.mutawassitat_al_hlqa(surah) else {
            // No interior: the region is at most twice the ring thick in one
            // axis, which is a very thin strip. The whole thing is background
            // and foreground together, so the only signal left is whether it is
            // dark overall.
            let kull = surah.mutawassit();
            let yaqlib = kull < 110.0;
            self.athar.push(format!(
                "inversion: {}, the region is too thin for a border ring and its mean is \
                 {kull:.1}",
                if yaqlib { "applied" } else { "not applied" }
            ));
            if yaqlib {
                for khana in &mut surah.bayt {
                    *khana = 255_u8.saturating_sub(*khana);
                }
            }
            return yaqlib;
        };

        if dakhil > itar + self.iadadat.hamish_aks {
            for khana in &mut surah.bayt {
                *khana = 255_u8.saturating_sub(*khana);
            }
            self.athar.push(format!(
                "inversion: applied, the interior mean {dakhil:.1} is above the border mean \
                 {itar:.1} by more than the {:.1} margin",
                self.iadadat.hamish_aks
            ));
            return true;
        }

        self.athar.push(format!(
            "inversion: not applied, the interior mean {dakhil:.1} is not above the border mean \
             {itar:.1} by the {:.1} margin",
            self.iadadat.hamish_aks
        ));
        false
    }

    /// The mean of the border ring and the mean of what it encloses.
    ///
    /// [`None`] when the ring swallows the whole image, which happens on a
    /// region only a few pixels tall.
    fn mutawassitat_al_hlqa(&self, surah: &SuratRamadiya) -> Option<(f64, f64)> {
        let sumk = self.iadadat.sumk_itar.max(1);
        if surah.ard <= sumk.saturating_mul(2) || surah.irtifa <= sumk.saturating_mul(2) {
            return None;
        }

        let mut majmu_itar = 0_u64;
        let mut adad_itar = 0_usize;
        let mut majmu_dakhil = 0_u64;
        let mut adad_dakhil = 0_usize;
        let asfal = surah.irtifa.saturating_sub(sumk);
        let yameen = surah.ard.saturating_sub(sumk);

        for (y, saf) in surah.sufuf().enumerate() {
            let saf_raqm = tul_u32(y);
            let saf_itar = saf_raqm < sumk || saf_raqm >= asfal;
            for (x, &qeema) in saf.iter().enumerate() {
                let amud = tul_u32(x);
                if saf_itar || amud < sumk || amud >= yameen {
                    majmu_itar = majmu_itar.saturating_add(u64::from(qeema));
                    adad_itar = adad_itar.saturating_add(1);
                } else {
                    majmu_dakhil = majmu_dakhil.saturating_add(u64::from(qeema));
                    adad_dakhil = adad_dakhil.saturating_add(1);
                }
            }
        }

        if adad_itar == 0 || adad_dakhil == 0 {
            return None;
        }
        Some((
            hajm_f64(majmu_itar) / adad_f64(adad_itar),
            hajm_f64(majmu_dakhil) / adad_f64(adad_dakhil),
        ))
    }

    /// Separates strokes from background with a per-pixel adaptive threshold.
    ///
    /// Sauvola rather than Otsu, and Sauvola rather than a fixed threshold,
    /// because game text has no single correct threshold: one dialogue box can
    /// have a gradient panel behind it, a portrait at one end and a name plate
    /// at the other, and any global cut either eats the strokes on the dark
    /// half or keeps the panel on the light half. Sauvola's rule —
    /// `T = m·(1 + k·(s/R − 1))` over a local window — lowers the threshold
    /// where the neighbourhood is flat, which is what a background is, and
    /// raises it where the neighbourhood has variance, which is what a stroke
    /// edge is.
    ///
    /// The naïve implementation re-sums a `(2r+1)²` window at every pixel. At
    /// the default radius that is 625 additions per pixel, or 78 million for a
    /// 1920×65 subtitle strip, which does not fit in the frame budget and does
    /// not fit in a background thread's share of a core either. The integral
    /// image formulation replaces it with four lookups per pixel after one
    /// linear pass, so the cost stops depending on the window size entirely and
    /// the radius becomes a free parameter the control panel can expose.
    ///
    /// The accumulators are `u64` and that is not caution for its own sake: the
    /// sum-of-squares image over a 33-megapixel region reaches 255² × 33.6M,
    /// which is about 2.2 × 10¹², eight times past what `u32` holds. A `u32`
    /// integral image wraps somewhere in the lower right of a large capture and
    /// produces a variance that is negative before the `max(0.0)` catches it —
    /// a threshold of `m·(1 − k)` across the entire bottom of the image, which
    /// reads as a solid black band.
    #[must_use]
    pub fn akhmid_khalfiya(&mut self, surah: &SuratRamadiya) -> SuratRamadiya {
        let tarakum = SuraTarakumiya::ibni(surah);
        let nisf = self.iadadat.nisf_nafidha.max(1);
        let mada = if self.iadadat.mada_sauvola.abs() < f64::EPSILON {
            128.0
        } else {
            self.iadadat.mada_sauvola
        };
        let mut mukhraj = Vec::with_capacity(surah.bayt.len());

        for (y, saf) in surah.sufuf().enumerate() {
            let saf_raqm = tul_u32(y);
            let aala = saf_raqm.saturating_sub(nisf);
            let asfal = saf_raqm.saturating_add(nisf).saturating_add(1).min(surah.irtifa);
            for (x, &qeema) in saf.iter().enumerate() {
                let amud = tul_u32(x);
                let yasar = amud.saturating_sub(nisf);
                let yameen = amud.saturating_add(nisf).saturating_add(1).min(surah.ard);
                let (majmu, murabbaat, adad) = tarakum.nafidha(yasar, aala, yameen, asfal);
                if adad == 0 {
                    mukhraj.push(255);
                    continue;
                }
                let adad_ashari = hajm_f64(adad);
                let mutawassit = hajm_f64(majmu) / adad_ashari;
                let tabayun = mutawassit.mul_add(-mutawassit, hajm_f64(murabbaat) / adad_ashari);
                let inhiraf = tabayun.max(0.0).sqrt();
                let atabah =
                    mutawassit * self.iadadat.muamil_sauvola.mul_add(inhiraf / mada - 1.0, 1.0);
                mukhraj.push(if f64::from(qeema) < atabah { 0 } else { 255 });
            }
        }

        self.athar.push(format!(
            "background suppression: Sauvola over a {}×{} window, k={}, R={mada}",
            nisf.saturating_mul(2).saturating_add(1),
            nisf.saturating_mul(2).saturating_add(1),
            self.iadadat.muamil_sauvola
        ));
        SuratRamadiya { bayt: mukhraj, ard: surah.ard, irtifa: surah.irtifa }
    }

    /// The height of the text in a region, not the height of the region.
    ///
    /// The median of the line bands [`KutalNass`] finds, or [`None`] when it
    /// finds none. Measured rather than assumed, and the difference between
    /// those two is the whole reason this function exists.
    ///
    /// [`MuhassinSura::kabbir`] used to compare the *region's* height against
    /// [`IdadatTahsin::irtifa_adna`], with a comment conceding that the region's
    /// height is not the text's and a floor set higher "with a margin" to
    /// compensate. Measurement showed what that costs: over a 360-case corpus,
    /// with 11-pixel text inside regions 27 to 29 pixels tall, the upscale
    /// *never fired once* — the step meant to rescue small text was dead on
    /// exactly the corpus it exists for, and the results with it and without it
    /// were byte-identical. A margin cannot fix a quantity that is measuring the
    /// wrong thing.
    ///
    /// The median rather than the mean, for the reason every median in this
    /// crate is one: a single band that merged two lines, or caught a UI rule
    /// under the text, would drag a mean far enough to suppress the upscale.
    fn irtifa_nass(surah: &SuratRamadiya) -> Option<u32> {
        let kutal = KutalNass::iktashif(surah, &IdadatKutal::default());
        let mut irtifaat: Vec<u32> =
            kutal.sutur().iter().map(|satr| satr.itar.irtifa).filter(|q| *q > 0).collect();
        if irtifaat.is_empty() {
            return None;
        }
        irtifaat.sort_unstable();
        irtifaat.get(irtifaat.len().checked_div(2).unwrap_or(0)).copied()
    }

    /// The integer factor this region should be upscaled by.
    ///
    /// One when the text is already tall enough, otherwise the smallest integer
    /// that reaches the stated minimum, capped. Integer rather than arbitrary
    /// because a non-integer scale puts stroke edges at fractional positions
    /// that differ from row to row, and the resulting ragged stems cost more
    /// accuracy than the extra pixels buy.
    fn muamil_takbir(&self, irtifa: u32) -> u32 {
        if irtifa == 0 || irtifa >= self.iadadat.irtifa_adna {
            return 1;
        }
        let saqf = self.iadadat.aqsa_takbir.max(1);
        let matlub = self
            .iadadat
            .irtifa_adna
            .saturating_add(irtifa.saturating_sub(1))
            .checked_div(irtifa)
            .unwrap_or(1);
        matlub.clamp(2, saqf.max(2))
    }

    /// Upscales a short region so the recognizers have strokes to work with.
    ///
    /// Both engines this crate ships to lose accuracy sharply below roughly
    /// twenty pixels of text height, and neither says so — they return
    /// confident wrong words rather than nothing, which is the failure that
    /// costs a player the most because it produces fluent Arabic saying
    /// something the game did not say. A 12px subtitle upscaled 2× is not more
    /// information, and that is the point: it is the *same* information
    /// presented at the scale the detector's receptive field was trained for.
    ///
    /// Bilinear rather than nearest. Nearest at 2× turns every anti-aliased
    /// stroke edge into a stair, and a stair is a high-frequency feature the
    /// detector did not see in training; bilinear preserves the ramp that was
    /// already there.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HajmMufrit`] when the upscaled result would exceed this
    /// build's pixel ceiling, refused before the allocation.
    pub fn kabbir(&mut self, surah: &SuratRamadiya) -> Result<SuratRamadiya, KhataTabaqa> {
        // The text's height, and the region's only when no text was found —
        // at which point there is nothing to upscale for and the fallback is
        // the conservative one.
        let (irtifa_qiyas, masdar) = Self::irtifa_nass(surah)
            .map_or((surah.irtifa, "the region"), |q| (q, "the text"));
        let mudaaf = self.muamil_takbir(irtifa_qiyas);
        if mudaaf <= 1 {
            self.athar.push(format!(
                "upscale: none, {masdar} is {irtifa_qiyas}px tall and the floor is {}px",
                self.iadadat.irtifa_adna
            ));
            return Ok(surah.clone());
        }

        let ard_jadeed = surah.ard.saturating_mul(mudaaf);
        let irtifa_jadeed = surah.irtifa.saturating_mul(mudaaf);
        let bikselat = u64::from(ard_jadeed).saturating_mul(u64::from(irtifa_jadeed));
        if bikselat > SAQF_BIKSELAT {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "upscaled region pixels",
                qeema: bikselat,
                saqf: SAQF_BIKSELAT,
            });
        }

        let Some(sia) = hajm_usize(bikselat) else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "upscaled region pixels",
                qeema: bikselat,
                saqf: tul_u64(usize::MAX),
            });
        };
        let mut mukhraj = Vec::with_capacity(sia);
        let miqyas = qeema_f32(mudaaf);

        for y in 0..irtifa_jadeed {
            // The half-pixel offsets put output sample centres at the centres
            // of the source pixels they cover. Without them the whole image
            // shifts up and left by `(mudaaf - 1) / (2·mudaaf)` of a source
            // pixel, which at 3× is a third of a pixel of skew applied to text
            // that is being upscaled precisely because it has no pixels to
            // spare.
            let masdar_y = (qeema_f32(y) + 0.5) / miqyas - 0.5;
            let qaa_y = masdar_y.floor();
            let nisbat_y = (masdar_y - qaa_y).clamp(0.0, 1.0);
            let y0 = mawdi_mahdud(qaa_y, surah.irtifa);
            let y1 = mawdi_mahdud(qaa_y + 1.0, surah.irtifa);
            for x in 0..ard_jadeed {
                let masdar_x = (qeema_f32(x) + 0.5) / miqyas - 0.5;
                let qaa_x = masdar_x.floor();
                let nisbat_x = (masdar_x - qaa_x).clamp(0.0, 1.0);
                let x0 = mawdi_mahdud(qaa_x, surah.ard);
                let x1 = mawdi_mahdud(qaa_x + 1.0, surah.ard);

                let aala_yasar = f32::from(surah.qeema(x0, y0));
                let aala_yameen = f32::from(surah.qeema(x1, y0));
                let asfal_yasar = f32::from(surah.qeema(x0, y1));
                let asfal_yameen = f32::from(surah.qeema(x1, y1));
                let aala = (aala_yameen - aala_yasar).mul_add(nisbat_x, aala_yasar);
                let asfal = (asfal_yameen - asfal_yasar).mul_add(nisbat_x, asfal_yasar);
                mukhraj.push(bayt_min_f32((asfal - aala).mul_add(nisbat_y, aala)));
            }
        }

        self.athar.push(format!(
            "upscale: {mudaaf}× bilinear, {}×{} to {ard_jadeed}×{irtifa_jadeed}, because \
             {masdar} is {irtifa_qiyas}px and the floor is {}px",
            surah.ard, surah.irtifa, self.iadadat.irtifa_adna
        ));
        SuratRamadiya::jadeeda(mukhraj, ard_jadeed, irtifa_jadeed)
    }

    /// Runs every step, in the order the recognizers want them.
    ///
    /// Grayscale, then contrast, then inversion, then background suppression if
    /// it is enabled, then upscale. Two orderings in that list are decisions
    /// rather than defaults.
    ///
    /// **Inversion after contrast**, because the border-ring test compares two
    /// means and a region whose values all sit between 90 and 120 gives the
    /// test almost nothing to compare. Stretching first spreads those thirty
    /// levels across the full range and the same comparison becomes decisive.
    ///
    /// **Upscale last.** Running Sauvola on an already-upscaled image would
    /// need the window radius scaled with it or the threshold would follow the
    /// interpolated ramps instead of the strokes; and interpolating a binarized
    /// image is a feature rather than a cost, because the grey it produces at
    /// every stroke edge is the anti-aliasing the binarizer had just thrown
    /// away.
    ///
    /// # Errors
    ///
    /// Whatever [`SuraMultaqata::ila_rgb`], [`MuhassinSura::ila_ramadi`] and
    /// [`MuhassinSura::kabbir`] refuse.
    pub fn hassin(&mut self, sura: &SuraMultaqata) -> Result<SuratRamadiya, KhataTabaqa> {
        self.imsah_athar();
        let rgb = sura.ila_rgb()?;
        let mut ramadi = self.ila_ramadi(&rgb, sura.ard(), sura.irtifa())?;
        let _ = self.sawi_tabayun(&mut ramadi);
        let _ = self.aksi_idha_lazim(&mut ramadi);
        if self.iadadat.yubaddil_thunai {
            ramadi = self.akhmid_khalfiya(&ramadi);
        } else {
            self.athar.push(
                "background suppression: off, both recognizers read grayscale better than they \
                 read a binarized image"
                    .to_owned(),
            );
        }
        self.kabbir(&ramadi)
    }

    /// The region as the recognizer should receive it.
    ///
    /// This is the call the capture loop makes, and [`MuhassinSura::hassin`] is
    /// the one to make when the grayscale itself is what is wanted — the
    /// change-detection hash, the text-block profile, a diagnostic image.
    /// Splitting them this way is the point: `hassin` alone produces something
    /// no recognizer can be handed, and every caller that only ever called
    /// `hassin` was preprocessing into a value it then had to throw away.
    ///
    /// Two shapes, chosen by [`IdadatTahsin::yuhawwil_ila_ramadi`], and the
    /// table on that field is the argument for which is the default.
    ///
    /// **Colour, the default.** The grayscale is still built and still stretched
    /// and inverted, because [`KutalNass`]'s projection profile needs dark ink
    /// on a light ground to find a line and the line is what gives the text's
    /// height. That image is then measured from and dropped, and what the
    /// recognizer receives is the capture's own colour, upscaled by the factor
    /// the measurement asked for.
    ///
    /// **Grayscale.** The full chain of [`MuhassinSura::hassin`], converted
    /// back. Better on flat panels, eleven and a half points worse on text over
    /// game art.
    ///
    /// In both shapes the upscale factor rides along in [`SuraMuhassana`],
    /// because a box the recognizer reports is in the upscaled image's
    /// coordinates and the overlay draws in the surface's.
    ///
    /// # Errors
    ///
    /// Whatever [`SuraMultaqata::ila_rgb`], [`MuhassinSura::hassin`],
    /// [`MuhassinSura::ila_ramadi`] and [`SuratRamadiya::ila_multaqata`] refuse,
    /// plus [`KhataTabaqa::HajmMufrit`] when the upscaled colour buffer would
    /// exceed this build's pixel ceiling.
    pub fn hassin_lil_qari(
        &mut self,
        sura: &SuraMultaqata,
    ) -> Result<SuraMuhassana, KhataTabaqa> {
        if self.iadadat.yuhawwil_ila_ramadi {
            let irtifa_asli = sura.irtifa().max(1);
            let ramadi = self.hassin(sura)?;
            let mudaaf = ramadi.irtifa().checked_div(irtifa_asli).unwrap_or(1).max(1);
            return Ok(SuraMuhassana {
                sura: ramadi.ila_multaqata()?,
                mintaqa: sura.mintaqa(),
                mudaaf,
            });
        }

        self.imsah_athar();
        let rgb = sura.ila_rgb()?;
        // The grayscale is still built, and still stretched and inverted,
        // because the projection profile that measures the text's height needs
        // dark ink on a light ground to find a line at all. It is measured from
        // and then dropped; what the recognizer receives is the colour.
        let mut qiyas = self.ila_ramadi(&rgb, sura.ard(), sura.irtifa())?;
        let _ = self.sawi_tabayun(&mut qiyas);
        let _ = self.aksi_idha_lazim(&mut qiyas);
        let (irtifa_qiyas, masdar) = Self::irtifa_nass(&qiyas)
            .map_or_else(|| (qiyas.irtifa(), "the region"), |q| (q, "the text"));
        let mudaaf = self.muamil_takbir(irtifa_qiyas);

        let (mukabbar, ard, irtifa) =
            Self::kabbir_rgb(&rgb, sura.ard(), sura.irtifa(), mudaaf)?;
        self.athar.push(format!(
            "to the recognizer: colour, {ard}×{irtifa}, upscaled {mudaaf}× because {masdar} \
             is {irtifa_qiyas}px and the floor is {}px. The grayscale was built to measure \
             that and discarded: over the measured corpus, collapsing colour before this \
             engine cost 11.6 points of exact-match on text over game art.",
            self.iadadat.irtifa_adna
        ));

        let sia = mukabbar.len().saturating_mul(4).checked_div(3).unwrap_or(0);
        let mut bayt = Vec::with_capacity(sia);
        for biksel in mukabbar.chunks_exact(3) {
            bayt.extend_from_slice(&[
                biksel.first().copied().unwrap_or(0),
                biksel.get(1).copied().unwrap_or(0),
                biksel.get(2).copied().unwrap_or(0),
                255,
            ]);
        }
        Ok(SuraMuhassana {
            sura: SuraMultaqata::jadeeda(
                bayt,
                ard,
                irtifa,
                SighatSath::Rgba8,
                MustatilBiksel { yasar: 0, aala: 0, ard, irtifa },
            )?,
            mintaqa: sura.mintaqa(),
            mudaaf,
        })
    }

    /// Upscales a tightly packed RGB8 buffer by an integer factor, bilinearly.
    ///
    /// The same half-pixel convention [`MuhassinSura::kabbir`] uses, and for the
    /// same reason: without it the image shifts up and left by a fraction of a
    /// source pixel, applied to text that is being upscaled precisely because it
    /// has no pixels to spare. Written separately rather than by running the
    /// grayscale version three times because three passes over one buffer is
    /// three times the cache traffic on a thread that shares a machine with a
    /// game.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HajmMufrit`] when the result would exceed this build's
    /// pixel ceiling or this target's address space.
    fn kabbir_rgb(
        rgb: &[u8],
        ard: u32,
        irtifa: u32,
        mudaaf: u32,
    ) -> Result<(Vec<u8>, u32, u32), KhataTabaqa> {
        if mudaaf <= 1 {
            return Ok((rgb.to_vec(), ard, irtifa));
        }
        let ard_jadeed = ard.saturating_mul(mudaaf);
        let irtifa_jadeed = irtifa.saturating_mul(mudaaf);
        let bikselat = u64::from(ard_jadeed).saturating_mul(u64::from(irtifa_jadeed));
        if bikselat > SAQF_BIKSELAT {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "upscaled colour region pixels",
                qeema: bikselat,
                saqf: SAQF_BIKSELAT,
            });
        }
        let Some(sia) = hajm_usize(bikselat.saturating_mul(3)) else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "upscaled colour region pixels",
                qeema: bikselat,
                saqf: tul_u64(usize::MAX),
            });
        };

        let mut mukhraj = Vec::with_capacity(sia);
        let miqyas = qeema_f32(mudaaf);
        let ain = |x: u32, y: u32, qanat: usize| -> f32 {
            let mawdi = mawdi_usize(y)
                .saturating_mul(mawdi_usize(ard))
                .saturating_add(mawdi_usize(x))
                .saturating_mul(3)
                .saturating_add(qanat);
            f32::from(rgb.get(mawdi).copied().unwrap_or(0))
        };

        for y in 0..irtifa_jadeed {
            let masdar_y = (qeema_f32(y) + 0.5) / miqyas - 0.5;
            let qaa_y = masdar_y.floor();
            let nisbat_y = (masdar_y - qaa_y).clamp(0.0, 1.0);
            let y0 = mawdi_mahdud(qaa_y, irtifa);
            let y1 = mawdi_mahdud(qaa_y + 1.0, irtifa);
            for x in 0..ard_jadeed {
                let masdar_x = (qeema_f32(x) + 0.5) / miqyas - 0.5;
                let qaa_x = masdar_x.floor();
                let nisbat_x = (masdar_x - qaa_x).clamp(0.0, 1.0);
                let x0 = mawdi_mahdud(qaa_x, ard);
                let x1 = mawdi_mahdud(qaa_x + 1.0, ard);
                for qanat in 0..3 {
                    let aala = (ain(x1, y0, qanat) - ain(x0, y0, qanat))
                        .mul_add(nisbat_x, ain(x0, y0, qanat));
                    let asfal = (ain(x1, y1, qanat) - ain(x0, y1, qanat))
                        .mul_add(nisbat_x, ain(x0, y1, qanat));
                    mukhraj.push(bayt_min_f32((asfal - aala).mul_add(nisbat_y, aala)));
                }
            }
        }
        Ok((mukhraj, ard_jadeed, irtifa_jadeed))
    }
}

impl Default for MuhassinSura {
    fn default() -> Self {
        Self::jadeed(IdadatTahsin::default())
    }
}

/// A source coordinate clamped into an image, as an index.
///
/// Clamping to the edge rather than returning nothing, because this is used by
/// bilinear sampling where the half-pixel offset legitimately produces −0.5 on
/// the first output column and `ard − 0.5` on the last. Both should sample the
/// edge pixel, which is what edge clamping means and what treating them as
/// out-of-bounds would get wrong.
fn mawdi_mahdud(qeema: f32, hadd: u32) -> u32 {
    if hadd == 0 || qeema <= 0.0 {
        return 0;
    }
    let saqf = qeema_f32(hadd.saturating_sub(1));
    let mahdud = qeema.min(saqf);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, hadd - 1] immediately above, and hadd is a u32 image dimension"
    )]
    {
        mahdud as u32
    }
}

/// Summed-area tables of a grayscale image and of its squares.
///
/// Both are `(ard + 1) × (irtifa + 1)` with a zero first row and column, which
/// is what removes the special case at the top and left edges: a window that
/// starts at column zero reads the zero column rather than being handled
/// separately, and the four-lookup rectangle sum is one expression for every
/// pixel in the image.
#[derive(Debug)]
struct SuraTarakumiya {
    majmu: Vec<u64>,
    murabbaat: Vec<u64>,
    ard: usize,
}

impl SuraTarakumiya {
    /// Builds both tables in one pass.
    fn ibni(surah: &SuratRamadiya) -> Self {
        let ard = mawdi_usize(surah.ard).saturating_add(1);
        let irtifa = mawdi_usize(surah.irtifa).saturating_add(1);
        let sia = ard.saturating_mul(irtifa);
        let mut majmu = vec![0_u64; sia];
        let mut murabbaat = vec![0_u64; sia];

        for (y, saf) in surah.sufuf().enumerate() {
            let sabiq = y.saturating_mul(ard);
            let hali = sabiq.saturating_add(ard);
            let mut jari_majmu = 0_u64;
            let mut jari_murabba = 0_u64;
            for (x, &qeema) in saf.iter().enumerate() {
                let wahid = u64::from(qeema);
                jari_majmu = jari_majmu.saturating_add(wahid);
                jari_murabba = jari_murabba.saturating_add(wahid.saturating_mul(wahid));
                let mawdi = x.saturating_add(1);
                let fawq_majmu =
                    majmu.get(sabiq.saturating_add(mawdi)).copied().unwrap_or(0);
                let fawq_murabba =
                    murabbaat.get(sabiq.saturating_add(mawdi)).copied().unwrap_or(0);
                if let Some(khana) = majmu.get_mut(hali.saturating_add(mawdi)) {
                    *khana = fawq_majmu.saturating_add(jari_majmu);
                }
                if let Some(khana) = murabbaat.get_mut(hali.saturating_add(mawdi)) {
                    *khana = fawq_murabba.saturating_add(jari_murabba);
                }
            }
        }

        Self { majmu, murabbaat, ard }
    }

    /// The sum, the sum of squares and the pixel count of a half-open window.
    ///
    /// `[yasar, yameen) × [aala, asfal)`. Half-open so a window clamped against
    /// the right or bottom edge is expressed by moving one bound, with no
    /// off-by-one left for the caller to get wrong twice.
    fn nafidha(&self, yasar: u32, aala: u32, yameen: u32, asfal: u32) -> (u64, u64, u64) {
        if yameen <= yasar || asfal <= aala {
            return (0, 0, 0);
        }
        let x1 = mawdi_usize(yasar);
        let y1 = mawdi_usize(aala);
        let x2 = mawdi_usize(yameen);
        let y2 = mawdi_usize(asfal);

        let qeema = |jadwal: &[u64], x: usize, y: usize| -> u64 {
            jadwal.get(y.saturating_mul(self.ard).saturating_add(x)).copied().unwrap_or(0)
        };
        let hisab = |jadwal: &[u64]| -> u64 {
            qeema(jadwal, x2, y2)
                .saturating_add(qeema(jadwal, x1, y1))
                .saturating_sub(qeema(jadwal, x2, y1))
                .saturating_sub(qeema(jadwal, x1, y2))
        };

        let adad = u64::from(yameen.saturating_sub(yasar))
            .saturating_mul(u64::from(asfal.saturating_sub(aala)));
        (hisab(&self.majmu), hisab(&self.murabbaat), adad)
    }
}

// ---------------------------------------------------------------------------
// Text-block detection
// ---------------------------------------------------------------------------

/// The knobs on [`KutalNass`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdadatKutal {
    /// The value below which a pixel counts as ink, or [`None`] to derive it.
    ///
    /// [`None`] runs Otsu over the region's own histogram, which is the right
    /// default: a fixed 128 is correct for a binarized image and wrong for the
    /// grayscale one this crate prefers to hand the recognizers, and the same
    /// region can be either depending on whether the user turned background
    /// suppression on.
    pub atabat_hibr: Option<u8>,
    /// The fraction of a row that must be ink before the row is part of a line.
    ///
    /// A fraction rather than a count so the same value works on a 400-pixel
    /// subtitle strip and a 1920-pixel one. The floor is one pixel, so a very
    /// narrow region does not end up with a threshold of zero and call every
    /// row a text row.
    pub nisbat_hibr_lil_satr: f64,
    /// How many blank rows may sit inside one line without splitting it.
    ///
    /// Not zero, and the reason is descenders: a row through the gap between
    /// the bowl of a lowercase `e` and the tail of a `g` on the next word can
    /// genuinely have no ink in it, and splitting there produces two half-lines
    /// that translate to nonsense.
    pub fajwat_sutur: u32,
    /// The shortest run of rows that is still a line, in pixels.
    pub adna_irtifa_satr: u32,
    /// How many blank columns still join two column runs into one word group.
    pub fajwat_kalimat: u32,
    /// The narrowest column run that is still a word, in pixels.
    pub adna_ard_kalima: u32,
    /// Pixels of padding added around every reported box.
    ///
    /// Recognizers want a margin. A box drawn exactly on the ink clips the
    /// anti-aliased edge of the first and last stroke, and both engines read
    /// a clipped first letter as a different letter often enough to matter.
    pub hashiya: u32,
}

impl Default for IdadatKutal {
    fn default() -> Self {
        Self {
            atabat_hibr: None,
            nisbat_hibr_lil_satr: 0.004,
            fajwat_sutur: 1,
            adna_irtifa_satr: 5,
            fajwat_kalimat: 6,
            adna_ard_kalima: 3,
            hashiya: 2,
        }
    }
}

/// One detected line of text and the word groups inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrKutla {
    /// The line's bounding box, in the grayscale image's own coordinates.
    pub itar: MustatilBiksel,
    /// The word groups, left to right.
    ///
    /// Left to right in *image* order, which is not reading order for the
    /// original text and is not reading order for the Arabic either. Nothing in
    /// this module knows the script; the boxes are geometry and
    /// [`crate::qira`] pairs them with the strings the recognizer returned.
    pub kalimat: Vec<MustatilBiksel>,
}

/// Where the text is in a captured region, found by projection profile.
///
/// The reason this exists rather than handing the whole rectangle to the
/// recognizer: a dialogue box is a paragraph. Both engines, given a rectangle,
/// return a set of word boxes with no grouping, and a translator fed
/// `["The", "old", "road", "north", "is", "closed"]` as six requests produces
/// six unrelated Arabic words. Fed one sentence it produces one sentence. The
/// projection profile is what turns the first into the second, and it is a
/// projection profile rather than a connected-components pass because it costs
/// two linear scans and connected components costs a union-find over every
/// pixel — on a thread that is already competing with a game for a core.
#[derive(Debug, Clone)]
pub struct KutalNass {
    sutur: Vec<SatrKutla>,
    atabah: u8,
    athar: Vec<String>,
}

impl KutalNass {
    /// Finds the lines and the word groups in a preprocessed grayscale image.
    ///
    /// Expects dark ink on a light background, which is what
    /// [`MuhassinSura::hassin`] produces — including the inversion, so a
    /// light-on-dark region has already been flipped by the time it arrives.
    /// Running this on an un-inverted region finds the *background* as ink and
    /// reports one line covering everything.
    #[must_use]
    pub fn iktashif(surah: &SuratRamadiya, iadadat: &IdadatKutal) -> Self {
        let mut athar = Vec::new();
        let atabah = if let Some(qeema) = iadadat.atabat_hibr {
            athar.push(format!("ink threshold: {qeema}, set by hand"));
            qeema
        } else {
            let qeema = atabat_otsu(surah);
            athar.push(format!("ink threshold: {qeema}, from Otsu over the region"));
            qeema
        };

        let hibr_lil_saf = ihsa_sufuf(surah, atabah);
        let adna = hadd_hibr(surah.ard, iadadat.nisbat_hibr_lil_satr);
        let nitaqat = nitaqat_min_ihsa(&hibr_lil_saf, adna, iadadat.fajwat_sutur);
        athar.push(format!(
            "line profile: {} band(s) of rows carrying at least {adna} ink pixel(s)",
            nitaqat.len()
        ));

        let mut sutur = Vec::new();
        for (aala, asfal) in nitaqat {
            let irtifa = asfal.saturating_sub(aala);
            if irtifa < iadadat.adna_irtifa_satr {
                continue;
            }
            let hibr_lil_amud = ihsa_aamida(surah, atabah, aala, asfal);
            let majmuaat = nitaqat_min_ihsa(&hibr_lil_amud, 1, iadadat.fajwat_kalimat);

            // Collected as edges first and widened once at the end. Widening
            // each word and then taking the bounding box of the widened words
            // would apply the margin twice to the line, which pulls in the
            // descenders of the line above.
            let mut hudud: Vec<(u32, u32)> = Vec::new();
            for (yasar, yameen) in majmuaat {
                if yameen.saturating_sub(yasar) >= iadadat.adna_ard_kalima {
                    hudud.push((yasar, yameen));
                }
            }
            let (Some(&(awwal, _)), Some(&(_, akhir))) = (hudud.first(), hudud.last()) else {
                continue;
            };

            let kalimat = hudud
                .iter()
                .map(|&(yasar, yameen)| {
                    wassi(
                        MustatilBiksel {
                            yasar,
                            aala,
                            ard: yameen.saturating_sub(yasar),
                            irtifa,
                        },
                        iadadat.hashiya,
                        surah.ard,
                        surah.irtifa,
                    )
                })
                .collect();
            let itar = wassi(
                MustatilBiksel {
                    yasar: awwal,
                    aala,
                    ard: akhir.saturating_sub(awwal),
                    irtifa,
                },
                iadadat.hashiya,
                surah.ard,
                surah.irtifa,
            );
            sutur.push(SatrKutla { itar, kalimat });
        }

        athar.push(format!(
            "blocks: {} line(s), {} word group(s)",
            sutur.len(),
            sutur.iter().map(|satr| satr.kalimat.len()).sum::<usize>()
        ));
        Self { sutur, atabah, athar }
    }

    /// The detected lines, top to bottom.
    #[must_use]
    pub fn sutur(&self) -> &[SatrKutla] {
        &self.sutur
    }

    /// The ink threshold that was used, derived or given.
    #[must_use]
    pub const fn atabah(&self) -> u8 {
        self.atabah
    }

    /// What detection decided, for the control panel.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Whether anything was found.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.sutur.is_empty()
    }

    /// The box enclosing every line — the paragraph.
    ///
    /// [`None`] for a region with no text, which is the ordinary case for a
    /// dialogue box between lines and is why [`KhataTabaqa::LaNassMaqru`] is
    /// [`taarib_usus::khata::Khutura::Maluma`] rather than a warning.
    #[must_use]
    pub fn fiqra(&self) -> Option<MustatilBiksel> {
        let awwal = self.sutur.first()?;
        let mut yasar = awwal.itar.yasar;
        let mut aala = awwal.itar.aala;
        let mut yameen = awwal.itar.yasar.saturating_add(awwal.itar.ard);
        let mut asfal = awwal.itar.aala.saturating_add(awwal.itar.irtifa);
        for satr in self.sutur.iter().skip(1) {
            yasar = yasar.min(satr.itar.yasar);
            aala = aala.min(satr.itar.aala);
            yameen = yameen.max(satr.itar.yasar.saturating_add(satr.itar.ard));
            asfal = asfal.max(satr.itar.aala.saturating_add(satr.itar.irtifa));
        }
        Some(MustatilBiksel {
            yasar,
            aala,
            ard: yameen.saturating_sub(yasar),
            irtifa: asfal.saturating_sub(aala),
        })
    }
}

/// Otsu's threshold over a grayscale image.
///
/// Maximises between-class variance across the 256 bins, which is the standard
/// derivation and is exactly right for the bimodal histogram a UI panel with
/// text on it produces. It is a poor choice for a photograph and this is never
/// run on one.
///
/// Returns 128 for a degenerate histogram — a region of one value — because
/// there is no meaningful split and 128 at least makes the ink counter report
/// consistently rather than reporting every pixel as ink on one frame and none
/// on the next.
fn atabat_otsu(surah: &SuratRamadiya) -> u8 {
    let mut tawzee = [0_u64; 256];
    for &qeema in surah.bayt() {
        if let Some(khana) = tawzee.get_mut(usize::from(qeema)) {
            *khana = khana.saturating_add(1);
        }
    }

    let kull = tul_u64(surah.bayt().len());
    if kull == 0 {
        return 128;
    }
    let kull_ashari = hajm_f64(kull);
    let majmu_kulli: f64 = tawzee
        .iter()
        .enumerate()
        .map(|(bin, &adad)| adad_f64(bin) * hajm_f64(adad))
        .sum();

    let mut majmu_khalfiya = 0.0_f64;
    let mut wazn_khalfiya = 0.0_f64;
    let mut afdal_tabayun = -1.0_f64;
    let mut afdal_atabah = 128_u8;

    for (bin, &adad) in tawzee.iter().enumerate() {
        wazn_khalfiya += hajm_f64(adad);
        if wazn_khalfiya <= 0.0 {
            continue;
        }
        let wazn_amamiya = kull_ashari - wazn_khalfiya;
        if wazn_amamiya <= 0.0 {
            break;
        }
        majmu_khalfiya += adad_f64(bin) * hajm_f64(adad);
        let mutawassit_khalfiya = majmu_khalfiya / wazn_khalfiya;
        let mutawassit_amamiya = (majmu_kulli - majmu_khalfiya) / wazn_amamiya;
        let farq = mutawassit_khalfiya - mutawassit_amamiya;
        let tabayun = wazn_khalfiya * wazn_amamiya * farq * farq;
        if tabayun > afdal_tabayun {
            afdal_tabayun = tabayun;
            afdal_atabah = u8::try_from(bin).unwrap_or(128);
        }
    }

    afdal_atabah
}

/// How many ink pixels a row must carry to be part of a line.
///
/// At least one, always. A fraction of a narrow region rounds to zero, and a
/// threshold of zero makes every row a text row and the whole region one line.
fn hadd_hibr(ard: u32, nisba: f64) -> u32 {
    let khaam = f64::from(ard) * nisba.clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "nisba is clamped into [0, 1] and ard is a u32 image width, so the product is \
                  a non-negative value no larger than ard"
    )]
    let hadd = khaam as u32;
    hadd.max(1)
}

/// The ink count of every row.
fn ihsa_sufuf(surah: &SuratRamadiya, atabah: u8) -> Vec<u32> {
    surah
        .sufuf()
        .map(|saf| tul_u32(saf.iter().filter(|&&qeema| qeema < atabah).count()))
        .collect()
}

/// The ink count of every column, over one band of rows.
fn ihsa_aamida(surah: &SuratRamadiya, atabah: u8, aala: u32, asfal: u32) -> Vec<u32> {
    let mut ihsa = vec![0_u32; mawdi_usize(surah.ard())];
    for (y, saf) in surah.sufuf().enumerate() {
        let saf_raqm = tul_u32(y);
        if saf_raqm < aala || saf_raqm >= asfal {
            continue;
        }
        for (x, &qeema) in saf.iter().enumerate() {
            if qeema < atabah
                && let Some(khana) = ihsa.get_mut(x) {
                    *khana = khana.saturating_add(1);
                }
        }
    }
    ihsa
}

/// Runs of positions whose count reaches `adna`, joined across gaps of at most
/// `fajwa`.
///
/// Returned half-open as `[bidaya, nihaya)`. One function for both axes,
/// because the row profile and the column profile are the same problem — the
/// only difference is which direction the counts were summed in, and writing it
/// twice would be writing the gap-joining logic twice.
fn nitaqat_min_ihsa(ihsa: &[u32], adna: u32, fajwa: u32) -> Vec<(u32, u32)> {
    let mut nitaqat: Vec<(u32, u32)> = Vec::new();
    let mut bidaya: Option<u32> = None;
    let mut akhir_maleeh = 0_u32;

    for (mawdi, &adad) in ihsa.iter().enumerate() {
        let raqm = tul_u32(mawdi);
        if adad >= adna {
            if bidaya.is_none() {
                bidaya = Some(raqm);
            }
            akhir_maleeh = raqm;
            continue;
        }
        if let Some(bidayat_nitaq) = bidaya
            && raqm.saturating_sub(akhir_maleeh) > fajwa {
                nitaqat.push((bidayat_nitaq, akhir_maleeh.saturating_add(1)));
                bidaya = None;
            }
    }

    if let Some(bidayat_nitaq) = bidaya {
        nitaqat.push((bidayat_nitaq, akhir_maleeh.saturating_add(1)));
    }
    nitaqat
}

/// A box grown by a margin and clipped back inside the image.
fn wassi(mustatil: MustatilBiksel, hashiya: u32, ard: u32, irtifa: u32) -> MustatilBiksel {
    let yasar = mustatil.yasar.saturating_sub(hashiya);
    let aala = mustatil.aala.saturating_sub(hashiya);
    let yameen = mustatil
        .yasar
        .saturating_add(mustatil.ard)
        .saturating_add(hashiya)
        .min(ard);
    let asfal = mustatil
        .aala
        .saturating_add(mustatil.irtifa)
        .saturating_add(hashiya)
        .min(irtifa);
    MustatilBiksel {
        yasar,
        aala,
        ard: yameen.saturating_sub(yasar),
        irtifa: asfal.saturating_sub(aala),
    }
}

// ---------------------------------------------------------------------------
// Rate limiting
// ---------------------------------------------------------------------------

/// The 8×8 average hash of a grayscale image, as 64 bits.
///
/// Average-hash rather than a difference hash or a DCT hash, and the choice is
/// about the failure mode rather than about accuracy. What this has to detect
/// is "the dialogue box now says something else", and what it has to *ignore*
/// is "the background behind the semi-transparent panel moved". A DCT hash is
/// more discriminating, which here means it fires on the moving background and
/// re-runs recognition on an unchanged sentence — the exact thing the gate
/// exists to prevent. Averaging 8×8 cells over the whole region blurs a moving
/// background into a stable mean while a changed line of text moves several
/// cells across the threshold at once.
///
/// Box-averaged rather than point-sampled. Point sampling an 8×8 grid of a
/// 1920×200 region reads 64 pixels, and whether those 64 pixels land on strokes
/// is close to arbitrary; two different sentences frequently sample identical.
#[must_use]
pub fn basmat_mutawassit(surah: &SuratRamadiya) -> u64 {
    let mut khalaya = [0.0_f64; 64];

    for (fahras, khana) in khalaya.iter_mut().enumerate() {
        let sutur_fahras = tul_u32(fahras);
        let saf = sutur_fahras.checked_div(8).unwrap_or(0);
        let amud = sutur_fahras.checked_rem(8).unwrap_or(0);
        let (yasar, yameen) = shariha(amud, surah.ard());
        let (aala, asfal) = shariha(saf, surah.irtifa());
        if yameen <= yasar || asfal <= aala {
            continue;
        }

        let mut majmu = 0_u64;
        let mut adad = 0_u64;
        for y in aala..asfal {
            for x in yasar..yameen {
                majmu = majmu.saturating_add(u64::from(surah.qeema(x, y)));
                adad = adad.saturating_add(1);
            }
        }
        if adad > 0 {
            *khana = hajm_f64(majmu) / hajm_f64(adad);
        }
    }

    let mutawassit = khalaya.iter().sum::<f64>() / 64.0;
    khalaya.iter().enumerate().fold(0_u64, |basma, (fahras, &qeema)| {
        if qeema > mutawassit { basma | (1_u64 << (fahras & 63)) } else { basma }
    })
}

/// One of eight equal slices of a dimension, as a half-open range.
///
/// The last slice absorbs the remainder rather than the first, so a 1921-pixel
/// region does not shift every cell boundary by one and change the hash of an
/// otherwise identical frame after a resize by a single pixel.
fn shariha(fahras: u32, madaa: u32) -> (u32, u32) {
    let bidaya = u64::from(fahras)
        .saturating_mul(u64::from(madaa))
        .checked_div(8)
        .unwrap_or(0);
    let nihaya = u64::from(fahras.saturating_add(1))
        .saturating_mul(u64::from(madaa))
        .checked_div(8)
        .unwrap_or(0);
    (
        u32::try_from(bidaya).unwrap_or(madaa),
        u32::try_from(nihaya).unwrap_or(madaa).min(madaa),
    )
}

/// The Hamming distance between two average hashes.
#[must_use]
pub const fn masafat_basmatayn(awwal: u64, thani: u64) -> u32 {
    (awwal ^ thani).count_ones()
}

/// The gate between "a frame was presented" and "recognition runs".
///
/// Without it the overlay recognizes a paused game sixty times a second. That
/// is not a small waste: recognition is the most expensive thing this crate
/// does by two orders of magnitude, it runs off the render thread but on the
/// same machine, and a player whose frame rate drops because the pause menu is
/// being read repeatedly will — correctly — conclude the overlay is the
/// problem.
///
/// Two independent gates, in this order, because they cost different amounts:
///
/// 1. **The clock.** A minimum interval, compared against the caller's own
///    microsecond counter. Costs nothing and rejects 96% of frames at the
///    default quarter-second.
/// 2. **The picture.** An 8×8 average hash of the preprocessed grayscale,
///    compared against the last one that was recognized. Costs one pass over
///    the region and rejects everything that survived the clock but did not
///    change — which on a dialogue box is almost all of it, because a line of
///    text is on screen for seconds.
///
/// The clock runs first for the obvious reason: the hash is the cheaper of the
/// two only in comparison to recognition.
#[derive(Debug, Clone)]
pub struct MuqayyidMuadal {
    fasil_adna_mikro: u64,
    aqsa_masafa: u32,
    akhir_lahza: Option<u64>,
    akhir_basma: Option<u64>,
    marrat: u64,
    tark_zaman: u64,
    tark_tashabuh: u64,
}

impl MuqayyidMuadal {
    /// Four times a second.
    ///
    /// Chosen against how fast a game reveals text rather than as a round
    /// number: a typewriter effect at its fastest adds a word every 60 to 80
    /// milliseconds, and recognizing mid-reveal produces a truncated sentence
    /// that then has to be discarded when the full one arrives. A quarter of a
    /// second is slow enough that most reveals have finished and fast enough
    /// that a player does not notice waiting.
    pub const FASIL_IFTIRADI_MIKRO: u64 = 250_000;

    /// Three bits of the sixty-four.
    ///
    /// Zero would re-recognize on any change at all, including the one cell
    /// that flickers because a semi-transparent panel has a particle effect
    /// behind it. Three is roughly "one cell of the sixty-four moved across the
    /// mean", which no real text change stays under: a changed line moves eight
    /// to twenty.
    pub const MASAFA_IFTIRADIYA: u32 = 3;

    /// A limiter with an explicit interval and Hamming threshold.
    #[must_use]
    pub const fn jadeed(fasil_adna_mikro: u64, aqsa_masafa: u32) -> Self {
        Self {
            fasil_adna_mikro,
            aqsa_masafa,
            akhir_lahza: None,
            akhir_basma: None,
            marrat: 0,
            tark_zaman: 0,
            tark_tashabuh: 0,
        }
    }

    /// The minimum interval, in microseconds.
    #[must_use]
    pub const fn fasil_mikro(&self) -> u64 {
        self.fasil_adna_mikro
    }

    /// Changes the minimum interval.
    pub const fn ihdud(&mut self, fasil_adna_mikro: u64) {
        self.fasil_adna_mikro = fasil_adna_mikro;
    }

    /// Changes the Hamming threshold.
    pub const fn ayyin_masafa(&mut self, aqsa_masafa: u32) {
        self.aqsa_masafa = aqsa_masafa;
    }

    /// Whether enough time has passed since the last pass.
    ///
    /// `lahza_mikro` is the caller's microsecond counter. This crate reads no
    /// clock — it runs inside a game's process and the caller is already
    /// timing frames for [`crate::wajiha::MeezaniyatItar`], so a second clock
    /// read here would be a syscall for a number that is already in a register.
    ///
    /// A counter that went *backwards* is treated as a fresh start rather than
    /// as an enormous elapsed time. That is not paranoia about monotonic
    /// clocks: a caller that passes a wall clock will see one on a daylight
    /// saving change or an NTP step, and the alternative interpretation —
    /// `saturating_sub` yielding zero, so the gate stays closed — would stall
    /// recognition until the clock caught back up.
    ///
    /// Records the moment when it returns true, so this is not a pure query and
    /// calling it twice for one frame skips a pass.
    pub const fn hal_yalzam(&mut self, lahza_mikro: u64) -> bool {
        if let Some(sabiqa) = self.akhir_lahza
            && lahza_mikro >= sabiqa
                && lahza_mikro.saturating_sub(sabiqa) < self.fasil_adna_mikro
            {
                self.tark_zaman = self.tark_zaman.saturating_add(1);
                return false;
            }
        self.akhir_lahza = Some(lahza_mikro);
        true
    }

    /// Whether this picture differs enough from the last recognized one.
    ///
    /// Records the hash when it returns true, and deliberately does not when it
    /// returns false: comparing against the last *recognized* frame rather than
    /// the last *seen* one is what stops a slow fade from creeping past the
    /// threshold one bit at a time and never triggering.
    pub const fn hal_taghayyarat(&mut self, basma: u64) -> bool {
        if let Some(sabiqa) = self.akhir_basma
            && masafat_basmatayn(sabiqa, basma) <= self.aqsa_masafa {
                self.tark_tashabuh = self.tark_tashabuh.saturating_add(1);
                return false;
            }
        self.akhir_basma = Some(basma);
        self.marrat = self.marrat.saturating_add(1);
        true
    }

    /// Both gates, in order, against a preprocessed region.
    ///
    /// This is the call the capture loop makes. The hash is only computed when
    /// the clock has already let the frame through, which is what keeps the
    /// per-frame cost of the limiter at a comparison.
    pub fn hal_yalzam_lil_sura(&mut self, lahza_mikro: u64, surah: &SuratRamadiya) -> bool {
        if !self.hal_yalzam(lahza_mikro) {
            return false;
        }
        let basma = basmat_mutawassit(surah);
        self.hal_taghayyarat(basma)
    }

    /// Forgets what was last recognized, so the next frame passes both gates.
    ///
    /// Called when the region moves, when the surface changes, and when the
    /// user presses "translate now" — that last one is why this exists at all.
    /// A user who asks for a re-read and is told nothing changed has been given
    /// a correct answer to a question they did not ask.
    pub const fn ansa(&mut self) {
        self.akhir_lahza = None;
        self.akhir_basma = None;
    }

    /// How many times recognition was allowed through.
    #[must_use]
    pub const fn marrat(&self) -> u64 {
        self.marrat
    }

    /// How many frames the clock rejected.
    #[must_use]
    pub const fn tark_zaman(&self) -> u64 {
        self.tark_zaman
    }

    /// How many frames the hash rejected.
    #[must_use]
    pub const fn tark_tashabuh(&self) -> u64 {
        self.tark_tashabuh
    }

    /// The sentence the control panel displays.
    ///
    /// Both rejection counts are shown separately, because they mean different
    /// things to somebody debugging a region: a high clock count is normal, and
    /// a high similarity count next to a low pass count means the region is
    /// being captured but its contents are not changing — which is usually a
    /// region drawn around the wrong part of the screen.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} recognition pass(es); {} frame(s) held by the {} µs minimum interval, {} held \
             as unchanged within {} bit(s)",
            self.marrat,
            self.tark_zaman,
            self.fasil_adna_mikro,
            self.tark_tashabuh,
            self.aqsa_masafa
        )
    }
}

impl Default for MuqayyidMuadal {
    fn default() -> Self {
        Self::jadeed(Self::FASIL_IFTIRADI_MIKRO, Self::MASAFA_IFTIRADIYA)
    }
}
