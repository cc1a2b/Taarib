//! الرسم — turning a glyph identifier into pixels, through skrifa.
//!
//! This is the last stage of the pipeline and the only one that produces
//! anything a GPU can sample. It takes a glyph identifier — never a codepoint,
//! never a character, never a presentation form — and returns either an 8-bit
//! antialiased coverage bitmap or a signed distance field, together with the
//! bearings that place it against the pen position the layout computed.
//!
//! ## Why skrifa, and why that is the entire point
//!
//! Decision 3 chose skrifa as the rasterizer for one reason, and it is not
//! speed or features: skrifa and HarfRust parse fonts through the same
//! `read-fonts` stack, resolved once by Cargo for the whole dependency graph.
//! A glyph identifier that came out of shaping is, by construction, the same
//! identifier skrifa resolves an outline for. There is no translation layer
//! between the two and therefore no possibility of the classic mismatch where
//! the shaper's glyph 4211 and the rasterizer's glyph 4211 are different
//! glyphs because the two libraries disagreed about face indices, collection
//! offsets, variation instances or `CFF2` charstring numbering. That bug is
//! silent, is data-dependent, and reproduces only on particular fonts;
//! eliminating it structurally is worth more than any feature another
//! rasterizer could offer.
//!
//! Decision 4 makes that guarantee total rather than probable. [`Rassam`]
//! holds the same reference-counted [`MawridKhatt`] that shaping borrowed, and
//! every outline it draws comes out of that one immutable byte buffer, at the
//! same variation coordinates, resolved by the same identifier. Nothing here
//! opens a font, reads a file, or parses bytes a second time. If the identity
//! could drift, it would drift here — so it is not possible to drift here.
//!
//! ## The rasterizer
//!
//! Coverage is computed by a signed-area accumulation scanline rasterizer:
//! each edge of the flattened outline deposits, per pixel it crosses, the
//! exact signed area it contributes and the vertical cover it carries into the
//! rest of the scanline; integrating those deltas along a row yields the
//! winding value at every pixel, and the coverage is its magnitude clamped to
//! one. Nothing is supersampled and nothing is snapped to an integer grid
//! before filling.
//!
//! Both approximations are tempting and both are fatal here. Arabic strokes
//! are thin, and in Naskh they are also diagonal and continuously varying in
//! width: a hairline that a supersampler resolves as a ragged dotted line, or
//! that integer snapping thickens on one side of a curve and erases on the
//! other, is not a slightly worse letter — at interface sizes it is an
//! unreadable one. Exact area coverage is the only method that keeps a
//! quarter-pixel stroke visible as a quarter-pixel stroke.
//!
//! Quadratic and cubic segments are flattened adaptively against a tolerance
//! expressed in *pixels*, not font units, so a glyph drawn at 12 px is not
//! subdivided as finely as the same glyph at 120 px, and neither is
//! under-subdivided.
//!
//! ## Coverage is stored gamma-encoded; distance fields are not
//!
//! The rasterizer's natural output is a *linear* area fraction. Every engine
//! Taarib draws into blends in non-linear sRGB space, and blending a linear
//! coverage value there thins dark-on-light text and fattens light-on-dark
//! text — the classic reason a correct rasterizer still produces text that
//! looks wrong. So coverage pages are encoded through the sRGB transfer
//! function on the way out, which puts the stored byte in the same space as
//! the colours it will be mixed with.
//!
//! Distance fields are not coverage and are never gamma-encoded: they encode a
//! distance, and [`masafa_min_taghtiya`] expects the *linear* coverage bitmap
//! whose 128 level is the half-covered contour. [`Rassam::irsim`] feeds it
//! exactly that, before any transfer function is applied.
//!
//! ## One definition of a Taarib SDF
//!
//! [`masafa_min_taghtiya`] is public, and `taarib-lawha` (Phase 2, the atlas)
//! calls this function for its own atlas pages rather than implementing a
//! second distance transform. There is therefore exactly one definition in the
//! product of what a Taarib signed distance field is: an exact Euclidean
//! distance transform — Felzenszwalb–Huttenlocher, two passes, run over the
//! inside and the outside separately and combined — with the zero level at 128
//! and the spread given in pixels. A patch compiled by one version of the
//! atlas and a glyph rasterized at runtime by this module cannot disagree
//! about where the edge of a letter is, because they are the same code.
//!
//! ## What this module never does
//!
//! Hinting is off, always. Hinting exists to snap stems to a pixel grid at
//! small sizes for scripts built out of vertical stems; applied to Arabic it
//! destroys the very connections that make the script readable, and its
//! benefit is nil for a product that also ships distance-field atlases scaled
//! to arbitrary sizes. There is no parameter to turn it on.

use std::sync::Arc;

use skrifa::MetadataProvider as _;
use skrifa::instance::{Location, LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, GlyphId, Tag};
use taarib_usus::khata::Natija;

use crate::khata::{KhataKhatt, KhataSaff};
use crate::khatt::MawridKhatt;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// How many horizontal subpixel positions a glyph is rasterized at.
///
/// Positioning a glyph at its true fractional pen position is what keeps the
/// spacing of a line even; rounding every glyph to a whole pixel makes word
/// shapes visibly lumpy, and in cursive Arabic it also breaks joins, because
/// two letters that the font positioned to touch stop touching when they round
/// in opposite directions.
///
/// But a glyph drawn at an arbitrary fraction is a *different bitmap* for every
/// fraction. Without quantization the glyph cache key contains a float, every
/// pen position produces a cache miss, and the atlas grows without bound until
/// it exhausts its page budget on a thousand copies of the same letter. Four
/// positions is the point where the residual error — an eighth of a pixel at
/// worst — is below what antialiasing already blurs, while the cache stays
/// finite and small: four entries per `(font, glyph, size, mode)`, forever.
pub const MAWADI_TAHAZZUZ: u8 = 4;

/// Smallest pixel size that is rasterized rather than refused.
const HAJM_ADNA: f32 = 1.0;

/// Largest pixel size that is rasterized rather than refused.
///
/// Well past any interface text; a caller asking for more has computed a size
/// from something that is not a size.
const HAJM_AQSA: f32 = 1024.0;

/// Largest edge, in pixels, either dimension of a glyph bitmap may reach.
///
/// A single glyph larger than this is not a glyph; it is a bad size or a
/// corrupt outline, and allocating for it would be the memory failure rather
/// than the report of one.
const ABAD_AQSA: u32 = 8192;

/// Flattening tolerance, in pixels, for curve subdivision.
///
/// A tenth of a pixel: below what the coverage rasterizer can express, so
/// subdividing further changes no output byte, and above the point where thin
/// Naskh curves start to show their chords.
const TASAMUH_TASTIH: f32 = 0.1;

/// Ceiling on how many line segments one curve is flattened into.
///
/// Reached only by outlines whose control points are absurd — a corrupt
/// charstring, a variation delta that overflowed. Bounding the work turns a
/// hostile font into a slightly wrong glyph rather than a hang.
const ADAD_TAQSIM_AQSA: usize = 512;

/// The stand-in for "no seed here" in the distance transform.
///
/// A real infinity would make the lower-envelope intersections `NaN` when two
/// parabolas are both infinite. This value is far beyond any distance a
/// bitmap of at most [`ABAD_AQSA`] pixels can produce — the largest genuine
/// squared distance is under `1.4e8` — while staying small enough that adding
/// a genuine squared distance to it still changes the `f32`, which is what
/// keeps the envelope well ordered.
const LA_NIHAYA: f32 = 1.0e10;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// What a rasterized glyph's single channel means.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum NamatRasm {
    /// 8-bit antialiased coverage, sRGB-encoded.
    ///
    /// Sharper than a distance field at a fixed small size, which is why it is
    /// the default for interface text, and why a patch that only ever draws at
    /// the sizes its compiler saw should choose it.
    #[default]
    Taghtiya,
    /// A signed distance field: 128 is the outline, higher is inside.
    ///
    /// One page serves every size the game asks for, which is what makes
    /// auto-sizing text and world-space text work at all.
    Masafa {
        /// How far, in pixels, the field extends either side of the outline
        /// before it saturates. The shader's antialiasing width and any
        /// outline or glow effect are both bounded by this.
        intishar: f32,
    },
}

/// One rasterized glyph: its pixels and where they go.
///
/// The buffer is exactly `ard * irtifa` bytes with no row padding and no
/// alignment slack. Padding between glyphs is the atlas's business, because
/// only the atlas knows how much bleed its sampler needs, and a rasterizer
/// that guesses forces every consumer to know the guess.
#[derive(Debug, Clone, PartialEq)]
pub struct SurahHarf {
    /// Width in pixels.
    pub ard: u32,
    /// Height in pixels.
    pub irtifa: u32,
    /// Left bearing: where the bitmap's left edge sits relative to the pen
    /// position, in whole pixels, with the subpixel offset already folded in.
    pub izaha_s: i32,
    /// Top bearing: how far the bitmap's top edge sits above the baseline, in
    /// whole pixels. Positive is up.
    pub izaha_a: i32,
    /// The glyph's horizontal advance at this size and these variation
    /// coordinates, in pixels.
    pub taqaddum: f32,
    /// The pixels, row-major from the top, one byte per pixel.
    pub bayt: Vec<u8>,
    /// What those bytes mean.
    pub namat: NamatRasm,
}

impl SurahHarf {
    /// A glyph with no pixels — a space, a zero-width joiner, a mark the font
    /// draws nothing for.
    ///
    /// This is a normal result, not a failure. Roughly a fifth of the glyphs in
    /// a line of Arabic prose have no outline, and a rasterizer that reported
    /// an error for each of them would drown the diagnostics that matter.
    #[must_use]
    pub const fn farigha(taqaddum: f32, namat: NamatRasm) -> Self {
        Self {
            ard: 0,
            irtifa: 0,
            izaha_s: 0,
            izaha_a: 0,
            taqaddum,
            bayt: Vec::new(),
            namat,
        }
    }

    /// Whether this glyph has no pixels.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ard == 0 || self.irtifa == 0
    }

    /// One row of pixels, from the top.
    #[must_use]
    pub fn satr(&self, a: u32) -> Option<&[u8]> {
        if a >= self.irtifa {
            return None;
        }
        let ard = ila_hajm_u32(self.ard);
        let bidaya = ila_hajm_u32(a).checked_mul(ard)?;
        self.bayt.get(bidaya..bidaya.checked_add(ard)?)
    }
}

/// One variation axis pinned to a value, in the axis's own user coordinates.
///
/// The same values shaping was given. A variable font shaped at one weight and
/// drawn at another produces glyphs whose advances do not match the advances
/// the line was measured with, and the line drifts off its margin — which is
/// exactly the failure Decision 4 exists to make impossible, so these are
/// carried explicitly rather than defaulted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MihwarQeema {
    /// The four-character axis tag, for example `wght`.
    pub wasm: [u8; 4],
    /// The position on that axis, in user coordinates.
    pub qeema: f32,
}

/// Draws glyphs from one font resource.
///
/// Cheap to clone: it holds the shared font buffer by reference count and owns
/// nothing else. Cheap to keep: it caches no bitmaps, because caching is the
/// atlas's job and a second cache underneath it would only mean two eviction
/// policies fighting over the same memory.
#[derive(Clone)]
pub struct Rassam {
    khatt: Arc<MawridKhatt>,
    adad_ashkal: u32,
}

impl core::fmt::Debug for Rassam {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        matbaa
            .debug_struct("Rassam")
            .field("adad_ashkal", &self.adad_ashkal)
            .finish_non_exhaustive()
    }
}

impl Rassam {
    /// Binds a rasterizer to a font resource.
    ///
    /// The resource is shared, not copied: this is the same buffer shaping
    /// borrows, held by the same reference count, and the outlines drawn
    /// through it are the outlines of the glyph identifiers shaping produced.
    ///
    /// # Errors
    ///
    /// Returns [`KhataKhatt::JadwalMafqud`] when the font carries no outline
    /// source at all — no `glyf`, no `CFF`, no `CFF2`, no `VARC`. Such a font
    /// may still shape, which is what makes the failure worth catching here:
    /// it would otherwise surface as every glyph in the patch coming out blank.
    /// Propagates whatever [`MawridKhatt::khatt`] reports when the bytes
    /// cannot be parsed.
    pub fn jadeed(khatt: &Arc<MawridKhatt>) -> Natija<Self> {
        let marja = khatt.khatt()?;
        if marja.outline_glyphs().format().is_none() {
            return Err(KhataKhatt::JadwalMafqud { jadwal: "glyf" }.into());
        }
        let adad_ashkal =
            marja.glyph_metrics(Size::unscaled(), LocationRef::default()).glyph_count();
        Ok(Self { khatt: Arc::clone(khatt), adad_ashkal })
    }

    /// The font resource this rasterizer draws from.
    ///
    /// Exposed so the atlas can key a glyph by `(font identity, glyph id)`
    /// without being handed a second reference to the same font from somewhere
    /// else and having to trust that they match.
    #[must_use]
    pub const fn khatt(&self) -> &Arc<MawridKhatt> {
        &self.khatt
    }

    /// How many glyphs the font has.
    #[must_use]
    pub const fn adad_ashkal(&self) -> u32 {
        self.adad_ashkal
    }

    /// Rasterizes one glyph.
    ///
    /// `tahazzuz` is the fractional part of the pen position the layout
    /// produced. It is quantized here, by [`bakat_tahazzuz`], rather than
    /// trusted as given — so the bitmap this returns and the cache key the
    /// caller computed can never describe different subpixel positions.
    ///
    /// A glyph with no outline returns [`SurahHarf::farigha`] carrying its real
    /// advance, not an error.
    ///
    /// # Errors
    ///
    /// - [`KhataSaff::HajmGhayrSalih`] when `hajm` is not finite or falls
    ///   outside the rasterizable range, and when a distance-field spread is
    ///   not a positive finite number of pixels.
    /// - [`KhataKhatt::MihwarMajhul`] when an axis is set that this font does
    ///   not have. Silently ignoring it would draw a glyph at coordinates that
    ///   are not the coordinates the run was measured at.
    /// - [`KhataKhatt::TahleelFashil`] when the glyph identifier is not in this
    ///   font, or when skrifa refuses the outline. The first of those is the
    ///   Decision 3 alarm: an identifier the shaper produced that the
    ///   rasterizer cannot resolve means the two disagree about the font, which
    ///   is the one thing sharing `read-fonts` is supposed to prevent.
    /// - [`KhataSaff::ArdGhayrSalih`] when the outline's own bounding box
    ///   exceeds [`ABAD_AQSA`] in either dimension.
    pub fn irsim(
        &self,
        muarrif: u32,
        hajm: f32,
        namat: NamatRasm,
        tahazzuz: f32,
        mawadi: &[MihwarQeema],
    ) -> Natija<SurahHarf> {
        tahaqquq_hajm(hajm)?;
        if let NamatRasm::Masafa { intishar } = namat {
            tahaqquq_intishar(intishar)?;
        }

        let izaha = izahat_tahazzuz(tahazzuz);
        let (hudud, hafat, taqaddum) = self.istakhrij(muarrif, hajm, izaha, mawadi)?;

        let Some(hudud) = hudud else {
            return Ok(SurahHarf::farigha(taqaddum, namat));
        };

        let taghtiya = irsim_taghtiya(&hafat, hudud);
        let bayt = match namat {
            NamatRasm::Taghtiya => {
                taghtiya.iter().map(|qeema| ila_bayt(tarmiz_srgb(*qeema))).collect()
            }
            NamatRasm::Masafa { intishar } => {
                // The distance transform is fed the *linear* coverage, through
                // the same public function the atlas calls, so that a glyph
                // rasterized here and a page transformed there cannot disagree
                // about where the edge of a letter is.
                let khattiya: Vec<u8> = taghtiya.iter().map(|qeema| ila_bayt(*qeema)).collect();
                masafa_min_taghtiya(&khattiya, hudud.ard, hudud.irtifa, intishar)?
            }
        };

        Ok(SurahHarf {
            ard: hudud.ard,
            irtifa: hudud.irtifa,
            izaha_s: hudud.izaha_s,
            izaha_a: hudud.izaha_a,
            taqaddum,
            bayt,
            namat,
        })
    }

    /// The pixel bounding box of a glyph at a size, without rasterizing it.
    ///
    /// Returns `(left bearing, top bearing, width, height)` — the same four
    /// numbers [`SurahHarf`] carries, measured the same way, so the atlas can
    /// plan a page's allocation before it commits to filling it. An outline
    /// with no contours returns `(0, 0, 0, 0)`.
    ///
    /// The box is measured at the font's default variation coordinates and at
    /// subpixel position zero, which is what a packer wants: the width of the
    /// widest bucket, not of one particular one.
    ///
    /// # Errors
    ///
    /// As [`Rassam::irsim`], less the mode-specific failures.
    pub fn hudud(&self, muarrif: u32, hajm: f32) -> Natija<(i32, i32, u32, u32)> {
        tahaqquq_hajm(hajm)?;
        let (hudud, _, _) = self.istakhrij(muarrif, hajm, 0.0, &[])?;
        Ok(hudud.map_or((0, 0, 0, 0), |h| (h.izaha_s, h.izaha_a, h.ard, h.irtifa)))
    }

    /// Draws the outline, flattens it, and measures it.
    ///
    /// The one place in this module that touches skrifa, so that every path
    /// into an outline goes through the same size, the same location, and the
    /// same identifier.
    fn istakhrij(
        &self,
        muarrif: u32,
        hajm: f32,
        izaha: f32,
        mawadi: &[MihwarQeema],
    ) -> Natija<(Option<Hudud>, Vec<Hafa>, f32)> {
        let marja = self.khatt.khatt()?;
        let mawqi = self.mawqi(&marja, mawadi)?;

        let hawiya = GlyphId::new(muarrif);
        let ashkal = marja.outline_glyphs();
        let Some(shakl) = ashkal.get(hawiya) else {
            return Err(KhataKhatt::TahleelFashil {
                tafsil: format!(
                    "glyph {muarrif} has no outline in a font of {} glyphs; the shaper and the \
                     rasterizer are not looking at the same face",
                    self.adad_ashkal
                ),
            }
            .into());
        };

        let mut qalam = QalamTastih::jadeed(izaha);
        let qiyas = shakl.draw(DrawSettings::unhinted(Size::new(hajm), &mawqi), &mut qalam);
        let qiyas = qiyas.map_err(|khata| KhataKhatt::TahleelFashil {
            tafsil: format!("glyph {muarrif} could not be drawn: {khata}"),
        })?;
        qalam.aghliq();

        let taqaddum = marja
            .glyph_metrics(Size::new(hajm), &mawqi)
            .advance_width(hawiya)
            .or(qiyas.advance_width)
            .unwrap_or(0.0);

        let hudud = qalam.hudud()?;
        Ok((hudud, qalam.hafat, taqaddum))
    }

    /// Converts user-space axis values into the normalized coordinates skrifa
    /// and HarfRust both position in.
    ///
    /// An axis the font does not have is a refusal rather than a shrug: it
    /// means the caller's idea of the font and the font itself have diverged,
    /// and the glyph that would come back is not the glyph the run was
    /// measured with.
    fn mawqi(&self, marja: &FontRef<'_>, mawadi: &[MihwarQeema]) -> Natija<Location> {
        if mawadi.is_empty() {
            return Ok(Location::default());
        }
        let mahawir = self.khatt.mahawir();
        for matlub in mawadi {
            if !mahawir.iter().any(|mihwar| mihwar.wasm == matlub.wasm) {
                return Err(KhataKhatt::MihwarMajhul {
                    mihwar: String::from_utf8_lossy(&matlub.wasm).into_owned(),
                }
                .into());
            }
        }
        Ok(marja
            .axes()
            .location(mawadi.iter().map(|m| (Tag::from_be_bytes(m.wasm), m.qeema))))
    }
}

// ---------------------------------------------------------------------------
// Subpixel quantization
// ---------------------------------------------------------------------------

/// Which of the [`MAWADI_TAHAZZUZ`] horizontal buckets a pen position falls in.
///
/// Only the fractional part matters: a glyph at x = 3.25 and the same glyph at
/// x = 91.25 are the same bitmap placed at different whole-pixel offsets, and
/// the cache must see them as one entry or it is not a cache.
///
/// Non-finite input returns bucket zero rather than failing, because a caller
/// that produced a `NaN` pen position has a problem the rasterizer cannot fix
/// and a glyph drawn at the pixel grid is a better report of it than an error
/// buried three layers down.
#[must_use]
pub fn bakat_tahazzuz(s: f32) -> u8 {
    if !s.is_finite() || MAWADI_TAHAZZUZ == 0 {
        return 0;
    }
    let kasr = s - s.floor();
    let mut baka: u8 = 0;
    // Compared rather than multiplied-and-cast: four comparisons cost nothing
    // and there is no float-to-integer conversion to reason about.
    while baka + 1 < MAWADI_TAHAZZUZ
        && kasr >= f32::from(baka + 1) / f32::from(MAWADI_TAHAZZUZ)
    {
        baka += 1;
    }
    baka
}

/// The horizontal shift, in pixels, that a quantized pen position implies.
fn izahat_tahazzuz(s: f32) -> f32 {
    if MAWADI_TAHAZZUZ == 0 {
        return 0.0;
    }
    f32::from(bakat_tahazzuz(s)) / f32::from(MAWADI_TAHAZZUZ)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Refuses a pixel size that cannot produce a sane bitmap.
fn tahaqquq_hajm(hajm: f32) -> Natija<()> {
    if hajm.is_finite() && (HAJM_ADNA..=HAJM_AQSA).contains(&hajm) {
        Ok(())
    } else {
        Err(KhataSaff::HajmGhayrSalih { hajm }.into())
    }
}

/// Refuses a distance-field spread that is not a positive number of pixels.
///
/// Reported as a size failure because that is what it is: a spread is a
/// pixel-space extent, and a zero or negative one asks for a field with no
/// gradient, which every shader that samples it would read as a hard edge.
fn tahaqquq_intishar(intishar: f32) -> Natija<()> {
    if intishar.is_finite() && intishar > 0.0 && intishar <= HAJM_AQSA {
        Ok(())
    } else {
        Err(KhataSaff::HajmGhayrSalih { hajm: intishar }.into())
    }
}

// ---------------------------------------------------------------------------
// Outline flattening
// ---------------------------------------------------------------------------

/// A point in device pixels, y up, origin at the pen position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Nuqta {
    s: f32,
    a: f32,
}

/// One straight segment of a flattened contour.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hafa {
    min: Nuqta,
    ila: Nuqta,
}

/// A glyph's pixel box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Hudud {
    izaha_s: i32,
    izaha_a: i32,
    ard: u32,
    irtifa: u32,
}

/// The pen skrifa draws into: it flattens curves and remembers edges.
///
/// The subpixel shift is applied here, at the input, so that the bounding box
/// measured from these points is the box of the glyph *as it will be drawn*
/// rather than the box of some canonical position it is then nudged away from.
struct QalamTastih {
    hafat: Vec<Hafa>,
    bidaya: Nuqta,
    jari: Nuqta,
    maftuh: bool,
    izaha: f32,
    adna_s: f32,
    adna_a: f32,
    aqsa_s: f32,
    aqsa_a: f32,
}

impl QalamTastih {
    fn jadeed(izaha: f32) -> Self {
        Self {
            hafat: Vec::new(),
            bidaya: Nuqta::default(),
            jari: Nuqta::default(),
            maftuh: false,
            izaha,
            adna_s: f32::INFINITY,
            adna_a: f32::INFINITY,
            aqsa_s: f32::NEG_INFINITY,
            aqsa_a: f32::NEG_INFINITY,
        }
    }

    fn hawwil(&self, s: f32, a: f32) -> Nuqta {
        Nuqta { s: s + self.izaha, a }
    }

    const fn wassi(&mut self, nuqta: Nuqta) {
        if !nuqta.s.is_finite() || !nuqta.a.is_finite() {
            return;
        }
        self.adna_s = self.adna_s.min(nuqta.s);
        self.adna_a = self.adna_a.min(nuqta.a);
        self.aqsa_s = self.aqsa_s.max(nuqta.s);
        self.aqsa_a = self.aqsa_a.max(nuqta.a);
    }

    fn ila(&mut self, nuqta: Nuqta) {
        self.hafat.push(Hafa { min: self.jari, ila: nuqta });
        self.jari = nuqta;
        self.wassi(nuqta);
    }

    /// Closes the current contour if one is open.
    ///
    /// Called on every `move_to`, on every `close`, and once more after
    /// drawing finishes, because a font is entitled to end a charstring
    /// without an explicit close and an unclosed contour rasterizes as a
    /// half-filled smear.
    fn aghliq(&mut self) {
        if self.maftuh {
            let bidaya = self.bidaya;
            self.hafat.push(Hafa { min: self.jari, ila: bidaya });
            self.jari = bidaya;
            self.maftuh = false;
        }
    }

    /// The measured box, or `None` when nothing was drawn.
    fn hudud(&self) -> Natija<Option<Hudud>> {
        if self.hafat.is_empty() || !self.adna_s.is_finite() || !self.adna_a.is_finite() {
            return Ok(None);
        }
        let s0 = ila_sahih(self.adna_s.floor());
        let a0 = ila_sahih(self.adna_a.floor());
        let s1 = ila_sahih(self.aqsa_s.ceil());
        let a1 = ila_sahih(self.aqsa_a.ceil());

        let ard = u32::try_from(s1.saturating_sub(s0)).unwrap_or(0);
        let irtifa = u32::try_from(a1.saturating_sub(a0)).unwrap_or(0);
        if ard == 0 || irtifa == 0 {
            // A contour with no area — a degenerate outline, or a glyph whose
            // whole extent falls inside one pixel boundary. It draws nothing.
            return Ok(None);
        }
        if ard > ABAD_AQSA || irtifa > ABAD_AQSA {
            return Err(KhataSaff::ArdGhayrSalih { ard: ila_kasr(ard.max(irtifa)) }.into());
        }
        Ok(Some(Hudud { izaha_s: s0, izaha_a: a1, ard, irtifa }))
    }
}

impl OutlinePen for QalamTastih {
    fn move_to(&mut self, s: f32, a: f32) {
        self.aghliq();
        let nuqta = self.hawwil(s, a);
        self.bidaya = nuqta;
        self.jari = nuqta;
        self.maftuh = true;
        self.wassi(nuqta);
    }

    fn line_to(&mut self, s: f32, a: f32) {
        let nuqta = self.hawwil(s, a);
        self.ila(nuqta);
    }

    fn quad_to(&mut self, ds: f32, da: f32, s: f32, a: f32) {
        let dhabt = self.hawwil(ds, da);
        let nihaya = self.hawwil(s, a);
        let bidaya = self.jari;
        let adad = taqsim_tarbii(bidaya, dhabt, nihaya);
        let adad_f = ila_kasr_hajm(adad);
        for khatwa in 1..=adad {
            let t = ila_kasr_hajm(khatwa) / adad_f;
            self.ila(qeemat_tarbii(bidaya, dhabt, nihaya, t));
        }
    }

    fn curve_to(&mut self, ds0: f32, da0: f32, ds1: f32, da1: f32, s: f32, a: f32) {
        let dhabt0 = self.hawwil(ds0, da0);
        let dhabt1 = self.hawwil(ds1, da1);
        let nihaya = self.hawwil(s, a);
        let bidaya = self.jari;
        let adad = taqsim_thulathi(bidaya, dhabt0, dhabt1, nihaya);
        let adad_f = ila_kasr_hajm(adad);
        for khatwa in 1..=adad {
            let t = ila_kasr_hajm(khatwa) / adad_f;
            self.ila(qeemat_thulathi(bidaya, dhabt0, dhabt1, nihaya, t));
        }
    }

    fn close(&mut self) {
        self.aghliq();
    }
}

/// Length of the second difference of a quadratic or cubic, which bounds its
/// second derivative and therefore its distance from any chord.
fn tul_farq(min: Nuqta, wasat: Nuqta, ila: Nuqta) -> f32 {
    let s = 2.0f32.mul_add(-wasat.s, min.s) + ila.s;
    let a = 2.0f32.mul_add(-wasat.a, min.a) + ila.a;
    s.hypot(a)
}

/// Turns an error bound into a segment count.
fn adad_min_khata(hadd: f32) -> usize {
    if !hadd.is_finite() || hadd <= 0.0 {
        return 1;
    }
    let adad = ila_sahih(hadd.sqrt().ceil());
    ila_hajm(adad).clamp(1, ADAD_TAQSIM_AQSA)
}

/// Segments a quadratic needs to stay inside [`TASAMUH_TASTIH`].
///
/// A quadratic's second derivative is the constant `2·(p₀ − 2c + p₂)`, and
/// flattening into `n` equal-parameter chords leaves an error of at most
/// `|B″| / 8n²`. Solving for `n` gives `√(|p₀ − 2c + p₂| / 4ε)`.
fn taqsim_tarbii(min: Nuqta, dhabt: Nuqta, ila: Nuqta) -> usize {
    adad_min_khata(tul_farq(min, dhabt, ila) / (4.0 * TASAMUH_TASTIH))
}

/// Segments a cubic needs to stay inside [`TASAMUH_TASTIH`].
///
/// A cubic's second derivative is bounded by `6·max(|p₀ − 2c₀ + c₁|,
/// |c₀ − 2c₁ + p₃|)`, so the same `|B″| / 8n²` bound gives
/// `n = √(3·M / 4ε)`.
fn taqsim_thulathi(min: Nuqta, dhabt0: Nuqta, dhabt1: Nuqta, ila: Nuqta) -> usize {
    let akbar = tul_farq(min, dhabt0, dhabt1).max(tul_farq(dhabt0, dhabt1, ila));
    adad_min_khata(3.0 * akbar / (4.0 * TASAMUH_TASTIH))
}

/// A quadratic Bézier at parameter `t`.
fn qeemat_tarbii(min: Nuqta, dhabt: Nuqta, ila: Nuqta, t: f32) -> Nuqta {
    let u = 1.0 - t;
    let a0 = u * u;
    let a1 = 2.0 * u * t;
    let a2 = t * t;
    Nuqta {
        s: a2.mul_add(ila.s, a0 * min.s + a1 * dhabt.s),
        a: a2.mul_add(ila.a, a0 * min.a + a1 * dhabt.a),
    }
}

/// A cubic Bézier at parameter `t`.
fn qeemat_thulathi(min: Nuqta, dhabt0: Nuqta, dhabt1: Nuqta, ila: Nuqta, t: f32) -> Nuqta {
    let u = 1.0 - t;
    let a0 = u * u * u;
    let a1 = 3.0 * u * u * t;
    let a2 = 3.0 * u * t * t;
    let a3 = t * t * t;
    Nuqta {
        s: a3.mul_add(ila.s, a0 * min.s + a1 * dhabt0.s + a2 * dhabt1.s),
        a: a3.mul_add(ila.a, a0 * min.a + a1 * dhabt0.a + a2 * dhabt1.a),
    }
}

// ---------------------------------------------------------------------------
// The scanline rasterizer
// ---------------------------------------------------------------------------

/// Rasterizes flattened edges into linear coverage in `0.0..=1.0`.
///
/// The accumulation buffer carries two guard columns per row. Every edge
/// deposits a closing delta at the column past its rightmost pixel, and an edge
/// that runs along the right edge of the box — which is exactly what a vertical
/// stem whose right side lands on a pixel boundary does — puts that delta one
/// column further still. Without the guards those deltas land on the first
/// pixels of the *next* row: a faint vertical line down the left of the glyph,
/// and a last column that cancels itself to nothing. The guards are working
/// memory, not output: the returned buffer is exactly `ard * irtifa` values.
fn irsim_taghtiya(hafat: &[Hafa], hudud: Hudud) -> Vec<f32> {
    let ard = ila_hajm_u32(hudud.ard);
    let irtifa = ila_hajm_u32(hudud.irtifa);
    let hajm = ard.saturating_mul(irtifa);
    if hajm == 0 {
        return Vec::new();
    }
    let khatwa = ard.saturating_add(2);

    let mut mutarakim = vec![0.0f32; khatwa.saturating_mul(irtifa)];

    // The box's own origin, in the same y-up space the pen recorded.
    let asl_s = ila_kasr_sahih(hudud.izaha_s);
    let asl_a = ila_kasr_sahih(hudud.izaha_a.saturating_sub(ila_sahih_u32(hudud.irtifa)));
    let irtifa_f = ila_kasr(hudud.irtifa);

    for hafa in hafat {
        // Translate into the bitmap and flip y so row zero is the top. The
        // flip reverses winding, which is why coverage is taken as the
        // magnitude of the accumulated value: a rasterizer that trusted the
        // sign would fill TrueType and PostScript outlines with opposite
        // polarity.
        let min = Nuqta {
            s: hafa.min.s - asl_s,
            a: irtifa_f - (hafa.min.a - asl_a),
        };
        let ila = Nuqta {
            s: hafa.ila.s - asl_s,
            a: irtifa_f - (hafa.ila.a - asl_a),
        };
        khutt(&mut mutarakim, khatwa, ard, irtifa, min, ila);
    }

    let mut taghtiya = vec![0.0f32; hajm];
    for satr in 0..irtifa {
        let bidaya = satr.saturating_mul(khatwa);
        let hadaf = satr.saturating_mul(ard);
        let mut jam = 0.0f32;
        for amud in 0..ard {
            jam += mutarakim.get(bidaya.saturating_add(amud)).copied().unwrap_or(0.0);
            if let Some(khana) = taghtiya.get_mut(hadaf.saturating_add(amud)) {
                *khana = jam.abs().min(1.0);
            }
        }
    }
    taghtiya
}

/// Deposits one edge's signed area and cover into the accumulation buffer.
///
/// For each scanline the edge crosses, the exact trapezoid the edge cuts out of
/// that row is distributed across the columns it spans: the first and last
/// columns receive the partial areas of the triangles at the ends, and every
/// column between them receives an equal share of the middle. The signed
/// vertical extent travels with it, so integrating the row left to right
/// reproduces the winding value at every pixel — which is the whole trick, and
/// the reason no sample grid appears anywhere in this function.
fn khutt(
    mutarakim: &mut [f32],
    khatwa: usize,
    ard: usize,
    irtifa: usize,
    min: Nuqta,
    ila: Nuqta,
) {
    if ard == 0 || irtifa == 0 {
        return;
    }
    if !min.s.is_finite() || !min.a.is_finite() || !ila.s.is_finite() || !ila.a.is_finite() {
        return;
    }
    // A horizontal edge contributes no cover: it changes no winding number.
    if (min.a - ila.a).abs() < f32::EPSILON {
        return;
    }

    let (ittijah, aala, adna) =
        if min.a < ila.a { (1.0f32, min, ila) } else { (-1.0f32, ila, min) };

    let dsda = (adna.s - aala.s) / (adna.a - aala.a);
    let irtifa_f = ila_kasr_hajm(irtifa);
    let ard_f = ila_kasr_hajm(ard);

    let a_bidaya = aala.a.max(0.0);
    let a_nihaya = adna.a.min(irtifa_f);
    if a_nihaya <= a_bidaya {
        return;
    }

    let mut satr = ila_hajm(ila_sahih(a_bidaya.floor())).min(irtifa.saturating_sub(1));
    let satr_akhir = ila_hajm(ila_sahih(a_nihaya.ceil())).min(irtifa);
    let mut s = (a_bidaya - aala.a).mul_add(dsda, aala.s);

    while satr < satr_akhir {
        let satr_f = ila_kasr_hajm(satr);
        let aala_satr = satr_f.max(a_bidaya);
        let adna_satr = (satr_f + 1.0).min(a_nihaya);
        let da = adna_satr - aala_satr;
        if da <= 0.0 {
            satr = satr.saturating_add(1);
            continue;
        }
        let s_talin = dsda.mul_add(da, s);
        // The signed vertical cover this row receives from the edge.
        let ghita = da * ittijah;

        let s_awwal = s.clamp(0.0, ard_f);
        let s_thani = s_talin.clamp(0.0, ard_f);
        let (s0, s1) =
            if s_awwal < s_thani { (s_awwal, s_thani) } else { (s_thani, s_awwal) };

        let s0_ard = s0.floor();
        let s1_saqf = s1.ceil();
        // `s0` and `s1` are clamped into `0..=ard`, so `i0` is exactly
        // `floor(s0)` and both `i0 + 1` and `i1` stay inside this row's two
        // guard columns. Nothing here can reach the next row.
        let i0 = ila_hajm(ila_sahih(s0_ard)).min(ard);
        let i1 = ila_hajm(ila_sahih(s1_saqf)).clamp(i0, ard);
        let bidaya = satr.saturating_mul(khatwa);

        if i1 <= i0 + 1 {
            // The edge stays inside one column: the trapezoid is a rectangle
            // split between that column and its right neighbour by the mean x.
            let wasat = 0.5f32.mul_add(s0 + s1, -s0_ard).clamp(0.0, 1.0);
            zid(mutarakim, bidaya.saturating_add(i0), ghita * (1.0 - wasat));
            zid(mutarakim, bidaya.saturating_add(i0 + 1), ghita * wasat);
        } else {
            let maqlub = (s1 - s0).recip();
            let s0_kasr = s0 - s0_ard;
            let hissa_ula = 0.5 * maqlub * (1.0 - s0_kasr) * (1.0 - s0_kasr);
            let s1_kasr = s1 - s1_saqf + 1.0;
            let hissa_akhira = 0.5 * maqlub * s1_kasr * s1_kasr;
            zid(mutarakim, bidaya.saturating_add(i0), ghita * hissa_ula);
            if i1 == i0 + 2 {
                zid(
                    mutarakim,
                    bidaya.saturating_add(i0 + 1),
                    ghita * (1.0 - hissa_ula - hissa_akhira),
                );
            } else {
                let hissa_thaniya = maqlub * (1.5 - s0_kasr);
                zid(
                    mutarakim,
                    bidaya.saturating_add(i0 + 1),
                    ghita * (hissa_thaniya - hissa_ula),
                );
                for amud in (i0 + 2)..i1.saturating_sub(1) {
                    zid(mutarakim, bidaya.saturating_add(amud), ghita * maqlub);
                }
                let mutarakima = hissa_thaniya + ila_kasr_hajm(i1 - i0 - 3) * maqlub;
                zid(
                    mutarakim,
                    bidaya.saturating_add(i1.saturating_sub(1)),
                    ghita * (1.0 - mutarakima - hissa_akhira),
                );
            }
            zid(mutarakim, bidaya.saturating_add(i1), ghita * hissa_akhira);
        }

        s = s_talin;
        satr = satr.saturating_add(1);
    }
}

/// Adds to an accumulation cell, ignoring an index the clamps should already
/// have made impossible.
fn zid(mutarakim: &mut [f32], fahras: usize, qeema: f32) {
    if let Some(khana) = mutarakim.get_mut(fahras) {
        *khana += qeema;
    }
}

// ---------------------------------------------------------------------------
// Gamma
// ---------------------------------------------------------------------------

/// The sRGB transfer function, as IEC 61966-2-1 defines it.
///
/// The piecewise form, not a `^(1/2.2)` approximation: the linear segment near
/// zero is what keeps the faintest coverage — the tip of a diagonal Naskh
/// stroke, the thin end of a kashida — from being crushed to nothing, which is
/// precisely where a power curve is worst and precisely where Arabic needs it
/// most.
fn tarmiz_srgb(khatti: f32) -> f32 {
    let qeema = if khatti.is_finite() { khatti.clamp(0.0, 1.0) } else { 0.0 };
    if qeema <= 0.003_130_8 {
        qeema * 12.92
    } else {
        1.055f32.mul_add(qeema.powf(1.0 / 2.4), -0.055)
    }
}

// ---------------------------------------------------------------------------
// Signed distance fields
// ---------------------------------------------------------------------------

/// Turns a linear coverage bitmap into a signed distance field.
///
/// The transform is exact: Felzenszwalb–Huttenlocher, the two-pass separable
/// algorithm that computes the true Euclidean distance to the nearest seed by
/// finding the lower envelope of one parabola per sample. It is run twice —
/// once seeded on the inside and once on the outside — and the two results are
/// combined into a signed field. Neither pass is a chamfer, a 3×4 mask, a
/// dead-reckoning sweep, or any of the other approximations that are cheaper
/// and produce fields whose contours wobble diagonally. A wobble of a third of
/// a pixel is invisible in a coverage bitmap and is a visibly wavy letter edge
/// once a shader magnifies the field to sixty pixels.
///
/// **The input must be *linear* coverage**, where 128 is the half-covered
/// pixel and therefore the outline itself. [`Rassam::irsim`] hands this
/// function exactly that, before any transfer function is applied;
/// `taarib-lawha` must do the same for the pages it transforms. Feeding it an
/// sRGB-encoded page would place the contour at roughly 22 % coverage instead
/// of 50 %, dilating every letter by a fraction of a pixel.
///
/// `taarib-lawha` (Phase 2) calls this function rather than implementing a
/// second distance transform, so there is exactly one definition in the product
/// of what a Taarib SDF is.
///
/// The output is `ard * irtifa` bytes: 128 is the outline, values above it are
/// inside the glyph, values below are outside, and the field saturates at
/// `intishar` pixels either side.
///
/// # Errors
///
/// - [`KhataSaff::ArdGhayrSalih`] when `ard * irtifa` is not the length of
///   `taghtiya`, because then the declared width does not describe the buffer
///   it was given and every row of the transform would be misaligned.
/// - [`KhataSaff::HajmGhayrSalih`] when `intishar` is not a positive finite
///   number of pixels.
pub fn masafa_min_taghtiya(
    taghtiya: &[u8],
    ard: u32,
    irtifa: u32,
    intishar: f32,
) -> Natija<Vec<u8>> {
    tahaqquq_intishar(intishar)?;

    let ard_h = ila_hajm_u32(ard);
    let irtifa_h = ila_hajm_u32(irtifa);
    let hajm = ard_h
        .checked_mul(irtifa_h)
        .filter(|hajm| *hajm == taghtiya.len())
        .ok_or_else(|| KhataSaff::ArdGhayrSalih { ard: ila_kasr(ard) })?;

    if hajm == 0 {
        return Ok(Vec::new());
    }

    // Seeds. `bidhar_dakhil` is seeded on the outside, so its transform gives
    // every pixel its distance to the nearest outside pixel — which is the
    // depth of an inside pixel. `bidhar_kharij` is the mirror of it.
    let mut bidhar_dakhil = vec![0.0f32; hajm];
    let mut bidhar_kharij = vec![0.0f32; hajm];
    for (fahras, qeema) in taghtiya.iter().enumerate() {
        let dakhil = *qeema >= 128;
        if let Some(khana) = bidhar_dakhil.get_mut(fahras) {
            *khana = if dakhil { LA_NIHAYA } else { 0.0 };
        }
        if let Some(khana) = bidhar_kharij.get_mut(fahras) {
            *khana = if dakhil { 0.0 } else { LA_NIHAYA };
        }
    }

    let masafat_dakhil = tahweel_thunai(bidhar_dakhil, ard_h, irtifa_h);
    let masafat_kharij = tahweel_thunai(bidhar_kharij, ard_h, irtifa_h);

    let mut natij = vec![0u8; hajm];
    for (fahras, qeema) in taghtiya.iter().enumerate() {
        // Distances are measured between pixel centres, so the nearest pixel
        // across the boundary is a whole pixel away while the boundary itself
        // is half of one. Subtracting the half puts the zero level where the
        // contour actually is, and gives the two pixels straddling an edge
        // the symmetric values ±0.5 they should have.
        let masafa = if *qeema >= 128 {
            masafat_dakhil.get(fahras).copied().unwrap_or(LA_NIHAYA).max(0.0).sqrt() - 0.5
        } else {
            0.5 - masafat_kharij.get(fahras).copied().unwrap_or(LA_NIHAYA).max(0.0).sqrt()
        };
        let mansub = 0.5f32.mul_add(masafa / intishar, 0.5);
        if let Some(khana) = natij.get_mut(fahras) {
            *khana = ila_bayt(mansub);
        }
    }
    Ok(natij)
}

/// The two-dimensional exact distance transform, in place over its seeds.
///
/// Separable because the squared Euclidean distance is: transforming every
/// column and then every row of the result gives the same answer as searching
/// the whole plane, which is what makes an exact transform linear in the number
/// of pixels rather than quadratic.
fn tahweel_thunai(mut bidhar: Vec<f32>, ard: usize, irtifa: usize) -> Vec<f32> {
    if ard == 0 || irtifa == 0 {
        return bidhar;
    }
    let aqsa = ard.max(irtifa);
    let mut f = vec![0.0f32; aqsa];
    let mut d = vec![0.0f32; aqsa];
    let mut v = vec![0usize; aqsa];
    let mut z = vec![0.0f32; aqsa.saturating_add(1)];

    for amud in 0..ard {
        for satr in 0..irtifa {
            let qeema = bidhar
                .get(satr.saturating_mul(ard).saturating_add(amud))
                .copied()
                .unwrap_or(LA_NIHAYA);
            if let Some(khana) = f.get_mut(satr) {
                *khana = qeema;
            }
        }
        tahweel_uhadi(irtifa, &f, &mut d, &mut v, &mut z);
        for satr in 0..irtifa {
            let qeema = d.get(satr).copied().unwrap_or(LA_NIHAYA);
            if let Some(khana) =
                bidhar.get_mut(satr.saturating_mul(ard).saturating_add(amud))
            {
                *khana = qeema;
            }
        }
    }

    for satr in 0..irtifa {
        let bidaya = satr.saturating_mul(ard);
        for amud in 0..ard {
            let qeema =
                bidhar.get(bidaya.saturating_add(amud)).copied().unwrap_or(LA_NIHAYA);
            if let Some(khana) = f.get_mut(amud) {
                *khana = qeema;
            }
        }
        tahweel_uhadi(ard, &f, &mut d, &mut v, &mut z);
        for amud in 0..ard {
            let qeema = d.get(amud).copied().unwrap_or(LA_NIHAYA);
            if let Some(khana) = bidhar.get_mut(bidaya.saturating_add(amud)) {
                *khana = qeema;
            }
        }
    }

    bidhar
}

/// The one-dimensional lower-envelope transform.
///
/// Each sample `q` contributes the parabola `(x − q)² + f(q)`. The first loop
/// walks left to right maintaining the vertices `v` of the lower envelope and
/// the boundaries `z` between them, discarding any vertex the new parabola
/// hides; the second loop reads the envelope off at every sample. Both are
/// linear, and the result is the exact minimum — the property a chamfer mask
/// gives up.
#[expect(
    clippy::many_single_char_names,
    reason = "n, f, d, v, z, k and q are Felzenszwalb and Huttenlocher's own names for the \
              sample count, the sampled function, the transform, the envelope's vertices, \
              its boundaries, the current vertex and the current sample; the comment above \
              reads against the paper and renaming them would break that correspondence"
)]
#[expect(
    clippy::while_float,
    reason = "the envelope walk advances while a boundary lies left of the sample, which is \
              an ordering comparison the algorithm specifies, not an equality test on floats"
)]
fn tahweel_uhadi(n: usize, f: &[f32], d: &mut [f32], v: &mut [usize], z: &mut [f32]) {
    if n == 0 {
        return;
    }
    let mut k = 0usize;
    if let Some(khana) = v.get_mut(0) {
        *khana = 0;
    }
    if let Some(khana) = z.get_mut(0) {
        *khana = f32::NEG_INFINITY;
    }
    if let Some(khana) = z.get_mut(1) {
        *khana = f32::INFINITY;
    }

    for q in 1..n {
        let fq = f.get(q).copied().unwrap_or(LA_NIHAYA);
        let qf = ila_kasr_hajm(q);
        let mut taqatu;
        loop {
            let vk = v.get(k).copied().unwrap_or(0);
            let vf = ila_kasr_hajm(vk);
            let fv = f.get(vk).copied().unwrap_or(LA_NIHAYA);
            // `qf` is strictly greater than `vf`: every vertex on the envelope
            // is an index already visited, so the denominator is never zero.
            taqatu = (qf.mul_add(qf, fq) - (fv + vf * vf)) / (2.0 * (qf - vf));
            if k == 0 || taqatu > z.get(k).copied().unwrap_or(f32::NEG_INFINITY) {
                break;
            }
            k -= 1;
        }
        k = k.saturating_add(1);
        if let Some(khana) = v.get_mut(k) {
            *khana = q;
        }
        if let Some(khana) = z.get_mut(k) {
            *khana = taqatu;
        }
        if let Some(khana) = z.get_mut(k.saturating_add(1)) {
            *khana = f32::INFINITY;
        }
    }

    k = 0;
    for q in 0..n {
        let qf = ila_kasr_hajm(q);
        while z.get(k.saturating_add(1)).copied().unwrap_or(f32::INFINITY) < qf {
            k = k.saturating_add(1);
        }
        let vk = v.get(k).copied().unwrap_or(0);
        let vf = ila_kasr_hajm(vk);
        let fv = f.get(vk).copied().unwrap_or(LA_NIHAYA);
        let farq = qf - vf;
        if let Some(khana) = d.get_mut(q) {
            *khana = farq * farq + fv;
        }
    }
}

// ---------------------------------------------------------------------------
// Numeric conversions
// ---------------------------------------------------------------------------

/// Truncates a floating value toward zero into an `i32`.
///
/// Callers pass an already-floored or already-ceiled value; this only performs
/// the conversion.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the value is clamped into a range far inside i32 on the line above, so the \
              conversion is exact for every input that reaches the cast"
)]
const fn ila_sahih(qeema: f32) -> i32 {
    if !qeema.is_finite() {
        return 0;
    }
    qeema.clamp(-1.0e9, 1.0e9) as i32
}

/// The `i32` form of a bitmap dimension.
fn ila_sahih_u32(qeema: u32) -> i32 {
    i32::try_from(qeema).unwrap_or(i32::MAX)
}

/// The `usize` form of a signed index, with anything negative treated as zero.
fn ila_hajm(qeema: i32) -> usize {
    usize::try_from(qeema).unwrap_or(0)
}

/// The `usize` form of a bitmap dimension.
fn ila_hajm_u32(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(0)
}

/// The `f32` form of a bitmap dimension.
#[expect(
    clippy::cast_precision_loss,
    reason = "dimensions are bounded by ABAD_AQSA (8192), far inside f32's exact integer range"
)]
const fn ila_kasr(qeema: u32) -> f32 {
    qeema as f32
}

/// The `f32` form of a pixel index or count.
#[expect(
    clippy::cast_precision_loss,
    reason = "indices are bounded by ABAD_AQSA (8192), far inside f32's exact integer range"
)]
const fn ila_kasr_hajm(qeema: usize) -> f32 {
    qeema as f32
}

/// The `f32` form of a bearing.
#[expect(
    clippy::cast_precision_loss,
    reason = "bearings are bounded by the pixel size ceiling, far inside f32's exact integer \
              range"
)]
const fn ila_kasr_sahih(qeema: i32) -> f32 {
    qeema as f32
}

/// Rounds a value in `0.0..=1.0` to the byte a single-channel page stores.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the argument is clamped into 0.0..=255.0 and a half added before the cast, so \
              the value converted is a non-negative whole number inside u8's range"
)]
fn ila_bayt(qeema: f32) -> u8 {
    if !qeema.is_finite() {
        return 0;
    }
    ((qeema * 255.0).clamp(0.0, 255.0) + 0.5).min(255.0) as u8
}
