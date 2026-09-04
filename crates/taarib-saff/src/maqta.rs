//! المقاطع — what the pipeline stages hand to one another.
//!
//! The engine's stages do not pass strings around. They pass *runs*: slices of
//! the logical text that agree on everything shaping needs to be constant over —
//! direction, embedding level, script, language, font, and the parts of a style
//! that change letterforms. A run is the unit HarfRust is called on, the unit
//! reordering moves, and the unit justification reshapes.
//!
//! Splitting text into runs correctly is most of what makes Arabic layout work,
//! and splitting it in the wrong places is most of what makes Arabic layout look
//! broken: a run boundary in the middle of a word severs the join between two
//! letters, and no later stage can put it back.

use core::ops::Range;

use crate::talab::{Dharra, Ittijah, LughaNass};

/// A four-character OpenType script tag.
///
/// Kept as a tag rather than an enum because it is what the shaper is handed,
/// and because a script Taarib has no opinion about still has to survive the
/// journey from character properties to shaping without being flattened into
/// "other".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Kitaba(pub [u8; 4]);

impl Kitaba {
    /// Arabic.
    pub const ARAB: Self = Self(*b"arab");
    /// Latin.
    pub const LATN: Self = Self(*b"latn");
    /// Characters with no script of their own — spaces, most punctuation.
    pub const ZYYY: Self = Self(*b"zyyy");
    /// Inherited: combining marks, which take the script of what they attach to.
    pub const ZINH: Self = Self(*b"zinh");

    /// Whether this script joins cursively, which is what decides whether a run
    /// boundary can be placed inside a word at all.
    #[must_use]
    pub const fn tasil(self) -> bool {
        matches!(&self.0, b"arab" | b"syrc" | b"mand" | b"mong" | b"nkoo" | b"phag" | b"adlm")
    }

    /// Whether this script is written right to left by default.
    #[must_use]
    pub const fn min_alyameen(self) -> bool {
        matches!(&self.0, b"arab" | b"hebr" | b"syrc" | b"thaa" | b"nkoo" | b"adlm" | b"mand")
    }

    /// The tag as text, for logs and diagnostics.
    #[must_use]
    pub fn wasm(self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
}

/// How a shaped glyph joins to its neighbours, and whether it can be stretched.
///
/// This is the information kashida justification is built on. Elongation is not
/// a matter of inserting a tatweel wherever there is a gap: a legitimate
/// elongation point exists only between two glyphs that are actually joined, and
/// only where the preceding letterform is one that classical practice stretches.
/// Both facts come from shaping, not from the characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SifatWasl {
    /// Joined to the glyph before it.
    pub qabl: bool,
    /// Joined to the glyph after it.
    pub baad: bool,
    /// The font offers a stretched form, or the joint after this glyph is one
    /// the script elongates at.
    pub madd: bool,
    /// How good an elongation point this is, from 0 for "never" upward through
    /// the classical priority order.
    pub rutba: u8,
}

/// One glyph as the shaper produced it, before positioning into a line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarfMashkul {
    /// The glyph identifier.
    pub muarrif: u32,
    /// Horizontal advance, in pixels.
    pub taqaddum_s: f32,
    /// Vertical advance, in pixels. Non-zero only in vertical writing, which
    /// this product does not lay out, but carried so the value is never silently
    /// dropped.
    pub taqaddum_a: f32,
    /// Horizontal offset from the pen position.
    pub izaha_s: f32,
    /// Vertical offset from the pen position. This is how a diacritic sits on
    /// its base: `GPOS` mark attachment resolves to an offset here.
    pub izaha_a: f32,
    /// Byte offset into the logical clean text of this glyph's cluster.
    pub anqud: u32,
    /// Whether this glyph is a combining mark.
    pub alama: bool,
    /// Joining and elongation information.
    pub wasl: SifatWasl,
}

/// A slice of logical text that is constant in everything shaping depends on.
#[derive(Debug, Clone, PartialEq)]
pub struct MaqtaMantiqi {
    /// The byte range in the clean text.
    pub nitaq: Range<u32>,
    /// The bidirectional embedding level, which carries both the direction and
    /// the nesting depth reordering needs.
    pub mustawa: u8,
    /// The resolved direction, which is the level's parity.
    pub ittijah: Ittijah,
    /// The style span in force.
    pub uslub: u16,
    /// Index into the font chain of the font that covers this run.
    pub khatt: u8,
    /// The script.
    pub kitaba: Kitaba,
    /// The language, which drives `locl`.
    pub lugha: LughaNass,
    /// When present, the run is an opaque atom and is never shaped.
    pub dharra: Option<Dharra>,
    /// The size this run is laid out at, after any per-span override.
    pub hajm: f32,
}

impl MaqtaMantiqi {
    /// Length in bytes.
    #[must_use]
    pub const fn tul(&self) -> u32 {
        self.nitaq.end.saturating_sub(self.nitaq.start)
    }

    /// Whether this run is empty, which a correct splitter never produces but a
    /// caller may still hand in.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.nitaq.start >= self.nitaq.end
    }
}

/// Anything that carries a bidirectional embedding level.
///
/// Reordering happens twice in the pipeline — once over logical runs while a
/// line is being assembled, once over shaped runs after it is — and rule L2 is
/// subtle enough that two implementations of it would be two chances to get it
/// wrong. In particular the lowest-odd-level derivation has to include the
/// paragraph's own base level, and a version that floors at level 1 instead
/// misorders two adjacent runs that differ only by font. That bug appears on
/// mixed-font text and only sometimes, which is the worst way for a bug to
/// appear. One implementation, over this trait, removes the possibility.
pub trait DhuMustawa {
    /// The run's embedding level.
    fn mustawa(&self) -> u8;
}

impl DhuMustawa for MaqtaMantiqi {
    fn mustawa(&self) -> u8 {
        self.mustawa
    }
}

/// A run after shaping.
#[derive(Debug, Clone, PartialEq)]
pub struct MaqtaMashkul {
    /// What was shaped.
    pub asl: MaqtaMantiqi,
    /// The glyphs, in visual order within the run.
    pub huruf: Vec<HarfMashkul>,
    /// The sum of the horizontal advances, in pixels.
    pub ard: f32,
    /// How far this run rises above the baseline.
    pub suud: f32,
    /// How far it falls below.
    pub hubut: f32,
}

impl DhuMustawa for MaqtaMashkul {
    fn mustawa(&self) -> u8 {
        self.asl.mustawa
    }
}

impl MaqtaMashkul {
    /// An empty shaped run, used for atoms and for zero-length runs.
    #[must_use]
    pub const fn min_asl(asl: MaqtaMantiqi) -> Self {
        Self { asl, huruf: Vec::new(), ard: 0.0, suud: 0.0, hubut: 0.0 }
    }

    /// Recomputes the run's width from its glyphs.
    ///
    /// Called after justification changes advances, because a width that is not
    /// recomputed after reshaping is the reason justified Arabic drifts away
    /// from its margin.
    pub fn qis(&mut self) {
        self.ard = self.huruf.iter().map(|harf| harf.taqaddum_s).sum();
    }

    /// The elongation candidates in this run, as indices into its glyphs, best
    /// first.
    #[must_use]
    pub fn mawadi_kashida(&self) -> Vec<(usize, u8)> {
        let mut mawadi: Vec<(usize, u8)> = self
            .huruf
            .iter()
            .enumerate()
            .filter(|(_, harf)| harf.wasl.madd && harf.wasl.baad && harf.wasl.rutba > 0)
            .map(|(fahras, harf)| (fahras, harf.wasl.rutba))
            .collect();
        mawadi.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        mawadi
    }
}

/// A place the line may be broken, found on the logical text before anything is
/// reordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FursatQat {
    /// The byte offset the break falls before.
    pub mawqi: u32,
    /// Whether the break is mandatory — a newline, a paragraph separator — as
    /// opposed to an opportunity the layout may or may not take.
    pub ilzami: bool,
}
