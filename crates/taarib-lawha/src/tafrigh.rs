//! التفريغ — collecting the glyph set an atlas has to hold.
//!
//! **The rule: the set comes from shaping the patch's real strings at the real
//! sizes the game draws them, and from nothing else. Never from enumerating
//! Unicode ranges.**
//!
//! That rule is not a preference and it is not an optimisation. Range
//! enumeration is wrong in both directions at once.
//!
//! It **misses** glyphs. Lam-alef — لا, لآ, لأ, لإ — is mandatory Arabic
//! orthography and has no codepoint to enumerate; it exists only as a `rlig`
//! substitution. So do the hundreds of further required and optional ligatures a
//! Naskh face carries, the `calt` contextual alternates that are the difference
//! between Naskh and something that merely uses Naskh's letters, and the
//! `init`/`medi`/`fina`/`isol` forms that make the script legible at all. An
//! atlas built from a character range contains none of them, and the failure
//! surfaces at runtime, in the game, as a missing letter in the middle of an
//! ordinary word.
//!
//! And it **includes** glyphs. The Arabic block, its supplement, the extended
//! ranges, the presentation forms nobody should be producing anyway — enumerated
//! at four subpixel buckets and three sizes, that is tens of thousands of images
//! for a game whose entire script uses a few hundred. Every one of them is
//! memory taken out of a process that is not ours, and pages that could have
//! held something.
//!
//! Shaping the real strings produces exactly the set that will be asked for and
//! nothing else, because it is the same shaping call that will run in the game.
//!
//! ## What a key is
//!
//! `(font index, glyph id, pixel size, rasterization mode, subpixel bucket)` —
//! [`MiftahShakl`]. Never a codepoint. A codepoint is not a glyph, and an atlas
//! keyed by one could hold a single image per character, which is another way of
//! saying it could not hold Arabic.
//!
//! The subpixel bucket comes from the **real fractional x positions** in the
//! layout, through [`bakat_tahazzuz`]. Not from every bucket the rasterizer
//! defines: a run of interface text laid out from a left margin lands on a
//! handful of fractions, often just one, and collecting the four buckets it
//! never uses would quadruple a coverage atlas for nothing. Not from the
//! integer part either — a glyph at x = 3.25 and the same glyph at x = 91.25 are
//! one image at two whole-pixel offsets.
//!
//! ## Distance fields have no buckets
//!
//! A distance field is placed by the vertex positions of the quad that samples
//! it, and the same texels serve every fraction of a pixel. Four shifted copies
//! of one field are four identical images. [`miftah_qiyasi`] is where that is
//! enforced, and both this collector and the runtime atlas run every key through
//! it, so a key made by one is a key the other finds.

use rustc_hash::FxHashSet;
use taarib_saff::natija::TakhtitNass;
use taarib_saff::rasm::bakat_tahazzuz;

use crate::khareeta::{MiftahShakl, NamatSafha};

/// Puts a key in the one form the whole product agrees on.
///
/// Today that means clearing the subpixel bucket for distance-field keys, for
/// the reason in the module documentation. It is a function rather than a
/// convention because a convention that lives in two modules is a convention
/// that will eventually be followed by one of them: a collector that kept the
/// bucket and a runtime that cleared it would miss every glyph in the compiled
/// atlas and rasterize the whole patch again at runtime — slowly, invisibly, and
/// with the diagnostics screen reporting an atlas that grows forever.
#[must_use]
pub const fn miftah_qiyasi(miftah: MiftahShakl) -> MiftahShakl {
    match miftah.namat {
        NamatSafha::Taghtiya => miftah,
        NamatSafha::Masafa => MiftahShakl { bakat: 0, ..miftah },
    }
}

/// The set of glyphs a patch needs, gathered from finished layouts.
#[derive(Debug, Clone)]
pub struct JamiAshkal {
    namat: NamatSafha,
    ashkal: FxHashSet<MiftahShakl>,
    /// Sizes in quarter-pixels, matching [`MiftahShakl::hajm_rubi`], so the set
    /// of distinct sizes is compared as integers and never as floats.
    ahjam: FxHashSet<u16>,
}

impl JamiAshkal {
    /// An empty collector for one rasterization mode.
    ///
    /// The mode is fixed for the whole collector because it is fixed for the
    /// whole patch: coverage or distance field is recorded in the patch and the
    /// adapter obeys it without asking. A collector that mixed the two would
    /// describe an atlas no patch can declare.
    #[must_use]
    pub fn jadeed(namat: NamatSafha) -> Self {
        Self {
            namat,
            ashkal: FxHashSet::default(),
            ahjam: FxHashSet::default(),
        }
    }

    /// The mode every key in this collector carries.
    #[must_use]
    pub const fn namat(&self) -> NamatSafha {
        self.namat
    }

    /// Adds every glyph a finished layout draws.
    ///
    /// Walks the positioned glyphs rather than
    /// [`TakhtitNass::ashkal`], which returns the distinct `(font, glyph)` pairs
    /// but has already thrown away the fractional positions the subpixel bucket
    /// is derived from. The pairs alone are not a key.
    ///
    /// `hajm` is used only when the layout carries no size of its own. When it
    /// does, the layout's size wins — and that is deliberate: a layout the
    /// overflow policy shrank to fit is *drawn* at the shrunk size, so
    /// collecting the size that was requested would put the wrong image in the
    /// patch and leave the one the game actually draws to be rasterized at
    /// runtime. That failure is invisible in the compiler and shows up later as
    /// an atlas that grows on every menu.
    pub fn idif_takhtit(&mut self, takhtit: &TakhtitNass, hajm: f32) {
        let mustaamal = if takhtit.hajm.is_finite() && takhtit.hajm > 0.0 {
            takhtit.hajm
        } else {
            hajm
        };
        if !mustaamal.is_finite() || mustaamal <= 0.0 {
            return;
        }
        for harf in &takhtit.huruf {
            let bakat = bakat_tahazzuz(harf.s);
            let miftah =
                MiftahShakl::jadeed(harf.khatt, harf.muarrif, mustaamal, self.namat, bakat);
            self.idif_shakl(miftah);
        }
    }

    /// Adds one key directly.
    ///
    /// For the glyphs a layout cannot produce: the `.notdef` box a chain falls
    /// back to, an ellipsis a truncation policy will insert, a glyph a runtime
    /// capture reported. The key's mode is replaced with this collector's, since
    /// the mode belongs to the patch rather than to the caller.
    pub fn idif_shakl(&mut self, miftah: MiftahShakl) {
        let miftah = miftah_qiyasi(MiftahShakl {
            namat: self.namat,
            ..miftah
        });
        let _ = self.ahjam.insert(miftah.hajm_rubi);
        let _ = self.ashkal.insert(miftah);
    }

    /// Every key, sorted and deduplicated.
    ///
    /// The order is [`MiftahShakl`]'s own, which is what makes packing
    /// deterministic: two compiles of one project hand the packer the same
    /// sequence and get byte-identical pages back.
    #[must_use]
    pub fn ashkal(&self) -> Vec<MiftahShakl> {
        let mut ashkal: Vec<MiftahShakl> = self.ashkal.iter().copied().collect();
        ashkal.sort_unstable();
        ashkal
    }

    /// How many distinct keys have been collected.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.ashkal.len()
    }

    /// Whether nothing has been collected.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.ashkal.is_empty()
    }

    /// The distinct pixel sizes seen, ascending.
    ///
    /// The number the atlas is sized by, and the number a reviewer reads first:
    /// a coverage patch with nineteen distinct sizes is a patch whose interface
    /// scales text by a continuous factor somewhere, and it will need distance
    /// fields rather than a larger page budget.
    #[must_use]
    pub fn ahjam(&self) -> Vec<f32> {
        let mut ahjam: Vec<u16> = self.ahjam.iter().copied().collect();
        ahjam.sort_unstable();
        ahjam
            .into_iter()
            .map(|rubi| f32::from(rubi) / 4.0)
            .collect()
    }

    /// The keys belonging to one font of the chain, sorted.
    ///
    /// The compiler reports these per font so a reviewer can see that the Latin
    /// fallback is carrying the digits and the Arabic face is carrying the
    /// prose, rather than the other way round — which is what a chain in the
    /// wrong order looks like from the outside.
    #[must_use]
    pub fn ashkal_khatt(&self, khatt: u8) -> Vec<MiftahShakl> {
        let mut ashkal: Vec<MiftahShakl> = self
            .ashkal
            .iter()
            .copied()
            .filter(|miftah| miftah.khatt == khatt)
            .collect();
        ashkal.sort_unstable();
        ashkal
    }

    /// Empties the collector while keeping its allocations.
    pub fn amsah(&mut self) {
        self.ashkal.clear();
        self.ahjam.clear();
    }
}
