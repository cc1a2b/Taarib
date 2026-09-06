//! النقل — the glyph-identifier transport, and the argument that it is not a
//! presentation-form pipeline.
//!
//! Read this header before the code. It is the most easily misunderstood thing
//! in the product, and a reader who skims it will conclude that this file does
//! the one thing Taarib exists to refuse.
//!
//! ## Where this sits
//!
//! Godot 3 draws text through `Font::draw_char` with no shaping and no
//! bidirectional reordering. The adapter's Godot 3 strategy is a ladder:
//! `istila` hooks the draw call and lays the text out through `jisr`, and where
//! the hook cannot be installed — a statically linked, stripped, custom-built
//! export template with no symbol to detour and no extension interface to
//! register against — there is one rung left below it, and this is it. When
//! this rung also declines, the adapter reports [`KhataLawha::NaqlMarfud`] and
//! the game keeps its own language. That is the whole ladder; there is nothing
//! under this.
//!
//! It lives beside the atlas rather than inside the Godot adapter because it is
//! not a Godot fact. Any engine whose only glyph-naming mechanism is a character
//! code needs exactly this, and GameMaker's font resources are the second such
//! engine. One adapter cannot depend on another, and this cannot sit in
//! `taarib-jisr` because `jisr` already depends on this crate — so it sits with
//! the atlas whose glyph images it addresses, on the shaping core's own types.
//!
//! ## What it does, in one paragraph
//!
//! The real logical text is shaped, once, by real HarfRust, using the real
//! font's own `GSUB` and `GPOS`. That produces a run of [`Harf`] — glyph
//! identifiers with positions. This module assigns each distinct
//! `(font index, glyph id, size)` in the patch a private-use codepoint, builds a
//! Godot 3 `BitmapFont`-shaped glyph table whose entry for that codepoint is
//! **Taarib's own rasterized image of that exact glyph**, and emits each
//! translated line as the sequence of those codepoints in visual order. The
//! engine then draws the sequence with the only primitive it has — one image per
//! character code, left to right — and what appears on screen is the shaper's
//! output, image for image.
//!
//! ## Decision 1 forbids Unicode presentation forms. This is not that.
//!
//! The objection writes itself: *you are putting non-letters into a string and
//! drawing them as Arabic, which is exactly what a presentation-form pipeline
//! does.* It is not, and the difference is not a matter of degree.
//!
//! **A presentation-form pipeline maps codepoints to codepoints.** It takes
//! U+0628 BEH, decides from a lookup table that this one is medial, and emits
//! U+FE92 ARABIC LETTER BEH MEDIAL FORM. To do that it must *throw the shaping
//! away and re-derive it*, from a table that knows only the letter and its two
//! neighbours. That table cannot know about the font's ligatures, cannot know
//! about its contextual alternates, cannot know about `mark`/`mkmk` attachment,
//! and cannot know that this particular font substitutes a different lam-alef
//! at the end of a word. Every one of those is lost, silently, and the loss is
//! invisible in the output because the output is still a valid Unicode string.
//! It is worse than lossy: the result is **text that lies about what it is**. It
//! sorts wrong, searches wrong, copies wrong, and reads wrong to a screen
//! reader, while looking approximately right to a sighted reader — which is
//! precisely why the practice survived long enough to become a habit.
//!
//! **This maps shaped glyph identifiers to opaque slots.** The shaping already
//! happened, in HarfRust, on the real logical text, with the real font's tables.
//! Nothing re-derives it and nothing approximates it. Every ligature the font
//! formed is one glyph id and gets one slot. Every contextual alternate the font
//! selected is the id the font selected. Every mark the font attached is the
//! mark it attached, at the position `GPOS` gave it. What is transported is the
//! shaper's output; the transport's only job is to name each of those glyphs in
//! a container whose only naming mechanism is a character code.
//!
//! The private-use codepoints carry **no Unicode meaning whatsoever**. They are
//! not characters that happen to look like letters; they are integers in an
//! address space the Unicode standard reserves for exactly this — private
//! agreement between a producer and a consumer, with no interchange semantics.
//! U+F0000 does not mean beh, does not mean anything, and is meaningful only to
//! the one glyph table this module generated in the same build.
//!
//! ## The invariants that keep that true, and where they are structural
//!
//! A claim in a doc comment is worth what the code makes true.
//!
//! 1. **A slot's key cannot contain a codepoint.** [`MiftahKhana`] has three
//!    fields — font index, size, glyph id — and there is no character field to
//!    put one in. Nothing in this file can allocate a slot *for a character*,
//!    because the function that would take one does not exist. This mirrors
//!    [`Harf`] itself, which deliberately carries no codepoint for the
//!    same reason.
//! 2. **There is no inverse into text.** [`TawzeeKhanat::miftah`] maps a slot
//!    back to `(font, size, glyph id)` for diagnostics. There is no function
//!    anywhere in this module from a slot to a character, to a string, or to a
//!    string-table entry, and no place a slot can be written except the glyph
//!    table and the sequence handed to the engine.
//! 3. **A transported sequence never prints as text.** [`NassManqul`] hides its
//!    codepoints behind a manual `Debug` that writes `U+F0000` and never the
//!    characters. A log line, a diagnostics bundle or a `{:?}` in somebody's
//!    debugger therefore cannot produce a run of private-use glyphs that a
//!    reader might copy out and mistake for Arabic.
//! 4. **A slot cannot be an ill-formed scalar.** The only way to obtain one is
//!    [`NitaqKhana::khana`], which goes through `char::from_u32`. A surrogate, a
//!    value above U+10FFFD, or a noncharacter cannot be produced by any path.
//! 5. **The generated resource cannot name a game asset.** The page file names
//!    written into the `.fnt` are refused by [`ism_safha_masmuh`] if they carry
//!    a path separator, a quote, or a control character, so the resource can
//!    only reference files beside itself. And this module opens nothing: it
//!    imports no `std::fs`, holds no file handle, and [`JadwalKhattNaql::aktub`]
//!    takes a sink the caller already opened. There is deliberately no variant
//!    of it that takes a path and opens it — the `&Path` it does take is copied
//!    into an error message and never touched.
//!
//! ## What is lost, said plainly
//!
//! This is a transport, and a transport that pretended to be free would be the
//! dishonest thing. A string that has gone through it is **no longer text**:
//!
//! - It is **not searchable**. A player searching an inventory for a word will
//!   not find it, because the sequence contains no letters.
//! - It is **not selectable or copyable as text**. Copying it out yields
//!   private-use codepoints that mean nothing in any other program.
//! - It is **not readable by a screen reader**, and no accessibility technology
//!   can recover the original from it.
//! - It **cannot be re-shaped**. It is already laid out at one size, for one
//!   font chain, at one line width. Changing any of those needs a new transport.
//! - It **cannot be concatenated** with another transported string and stay
//!   correct, because the join would not have been shaped.
//!
//! Every one of those is a reason this rung is the last one and not the first.
//! The capability report says which path was taken before the user installs, and
//! the documentation names this a transport everywhere it appears.
//!
//! ## Why the supplementary planes, and what breaks there
//!
//! The BMP private-use area, U+E000–U+F8FF, holds **6,400** slots. A patch for a
//! text-heavy game with two font sizes and a full Arabic repertoire —
//! isolated, initial, medial and final forms, the lam-alef ligatures, the
//! digits, the marks, plus whatever Latin the game keeps — will pass that. The
//! supplementary planes hold **131,068**: U+F0000–U+FFFFD in plane 15 and
//! U+100000–U+10FFFD in plane 16, each 65,534 wide. Both stop short of the two
//! noncharacters that end every plane, which is why the ranges end at `FFFD`
//! rather than `FFFF` — a noncharacter in a string is a value that any
//! conforming text stack is entitled to drop or replace, and a glyph table
//! addressed by one would lose two entries on some engines and not others.
//!
//! The risk is real and specific. Godot 3's `CharType` is `wchar_t`, which is
//! **16 bits wide on Windows** and 32 on Linux and macOS. On a Windows Godot 3
//! build, a `String` holds UTF-16-ish code units and the `BitmapFont` character
//! map is keyed by that 16-bit type, so a codepoint above U+FFFF arrives as a
//! surrogate pair and can never match a key — every astral slot would draw
//! nothing at all. Some third-party bitmap-font tooling truncates the same way.
//!
//! So the plane is a **decision the caller makes and the report records**, not
//! an assumption. [`NamatKhana::Mulhaq`] uses the supplementary planes and is
//! the default, because it is the one with room. [`NamatKhana::Asasi`] uses the
//! BMP area for a target whose character type is 16 bits wide, at 6,400 slots —
//! and when a patch needs more than that, it is refused with
//! [`KhataLawha::NaqlMumtali`] naming both numbers. It is never truncated: a
//! truncated glyph table draws the wrong letters, and wrong letters read to a
//! player as a corrupt font rather than as a limit that was reached.
//!
//! ## Determinism
//!
//! Two builds of one patch must produce byte-identical output, or a diff
//! between them says nothing. Slot assignment is therefore a function of the
//! *set* of glyphs alone: the pool is a `BTreeSet`, freezing walks it in
//! ascending `(font index, size, glyph id)` order, and slot *n* is the *n*-th
//! codepoint of the mode's ranges taken in order. Nothing depends on the order
//! strings were registered in, on a hash seed, or on `HashMap` iteration —
//! there is no `HashMap` in this file, and every table that reaches the output
//! is a `BTreeMap` or a `Vec` built by walking one.
//!
//! ## How the shaper's positioning survives a container that has one offset
//!
//! Godot 3's `BitmapFont` gives each character code one texture region, one
//! offset and one advance, plus a kerning table keyed on ordered pairs of codes
//! whose value is subtracted from the first code's advance. That is the entire
//! vocabulary. This module uses all of it:
//!
//! - The **region, offset and advance** come from the atlas entry for that
//!   glyph, so the image and its bearings are the rasterizer's own.
//! - The **per-instance horizontal placement** — every `GPOS` adjustment, every
//!   ligature advance, every mark's negative offset onto its base — is carried
//!   as [`ZawjTaqaddum`] kerning pairs computed from the actual shaped
//!   positions. Consecutive glyphs in the emitted sequence get exactly the gap
//!   the shaper produced.
//! - The **per-instance vertical placement** is the one thing the container
//!   cannot vary: there is one offset per code and no vertical kerning. A mark
//!   that `GPOS` placed at two different heights over two different bases needs
//!   two heights from one slot. This module picks the height the most instances
//!   need, and **counts and reports every instance it could not place**
//!   ([`TaqreerNaql::isti_mutanaziaa`]) rather than discarding the number. A
//!   caller that finds the count unacceptable declines the transport; a caller
//!   that never looked would not have known, which is the failure this report
//!   exists to prevent.
//!
//! ## One line per transport
//!
//! A registered run is one laid-out line. Every base glyph in it must sit on one
//! baseline, and a run whose bases disagree is refused with
//! [`KhataLawha::NaqlMarfud`] naming the glyph. Multi-line text is registered
//! line by line, from the layout's own `SatrMansuq` records. The alternative —
//! accepting a whole paragraph and inferring the line breaks — would mean this
//! module deciding where lines break, which is `saff`'s decision and not an
//! adapter's to make.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use taarib_saff::natija::Harf;

use crate::khareeta::{MawdiShakl, MiftahShakl, NamatSafha};
use crate::khata::KhataLawha;

// ---------------------------------------------------------------------------
// Ceilings
// ---------------------------------------------------------------------------

/// The most glyphs one registered run may carry.
///
/// A run is one laid-out line. Four thousand glyphs is far past any line of any
/// game's interface, and the ceiling exists so that a patch a stranger produced
/// cannot make this module reserve memory proportional to a number it declared.
pub const AQSA_HURUF_NASS: u64 = 4096;

/// The most runs one transport may carry.
///
/// Half a million lines is above every game this product has been pointed at,
/// including the ones whose script is a novel. Past it, refused by size rather
/// than discovered when the machine runs out of memory.
pub const AQSA_NUSUS: u64 = 500_000;

/// The most kerning pairs one generated font may carry.
///
/// Godot 3 builds the kerning map at load time as a red-black tree, and every
/// entry is a node allocated while the player is looking at a loading screen. A
/// quarter of a million is already generous; past it the font is refused rather
/// than shipped as a stall nobody can attribute.
pub const AQSA_AZWAJ: u64 = 262_144;

/// The largest texture page dimension this module will describe.
///
/// Sixteen thousand texels on a side is above what any GL ES 3.0 device
/// accepts, which is the floor Godot 3 targets. A page larger than this is a
/// number a patch declared, not a texture a device would upload.
pub const AQSA_QIYAS_SAFHA: u16 = 16_384;

// ---------------------------------------------------------------------------
// The slot ranges
// ---------------------------------------------------------------------------

/// An inclusive range of private-use codepoints slots are drawn from.
///
/// Constructed only by this module: the three constants below are the only
/// values that exist, so a range naming surrogates or noncharacters cannot be
/// built by a caller and handed back in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NitaqKhana {
    awwal: u32,
    akhir: u32,
}

impl NitaqKhana {
    /// Builds a range. Private on purpose; see the type's documentation.
    const fn jadeed(awwal: u32, akhir: u32) -> Self {
        Self { awwal, akhir }
    }

    /// The first codepoint, inclusive.
    #[must_use]
    pub const fn awwal(self) -> u32 {
        self.awwal
    }

    /// The last codepoint, inclusive.
    #[must_use]
    pub const fn akhir(self) -> u32 {
        self.akhir
    }

    /// How many slots the range provides.
    #[must_use]
    pub const fn ittisa(self) -> u32 {
        self.akhir.saturating_sub(self.awwal).saturating_add(1)
    }

    /// Whether a codepoint falls inside this range.
    #[must_use]
    pub const fn yahwi(self, khana: u32) -> bool {
        khana >= self.awwal && khana <= self.akhir
    }

    /// The slot `fahras` positions into this range.
    ///
    /// Returns [`None`] past the end, and — because it goes through
    /// `char::from_u32` — for anything that is not a Unicode scalar value. That
    /// second check is what makes "a slot is never a surrogate" structural
    /// rather than a property of the constants below.
    #[must_use]
    pub fn khana(self, fahras: u32) -> Option<char> {
        if fahras >= self.ittisa() {
            return None;
        }
        char::from_u32(self.awwal.checked_add(fahras)?)
    }

    /// How far into this range a codepoint sits, or [`None`] if it is outside.
    #[must_use]
    pub const fn fahras(self, khana: u32) -> Option<u32> {
        if self.yahwi(khana) { Some(khana.saturating_sub(self.awwal)) } else { None }
    }
}

/// The Basic Multilingual Plane's private-use area: U+E000–U+F8FF, 6,400 slots.
///
/// The only area addressable on a build whose character type is sixteen bits
/// wide. Small enough that a text-heavy patch will not fit, which is why it is
/// not the default.
pub const NITAQ_ASASI: NitaqKhana = NitaqKhana::jadeed(0x0000_E000, 0x0000_F8FF);

/// Plane 15's private-use area: U+F0000–U+FFFFD, 65,534 slots.
///
/// Stops at `FFFD` rather than `FFFF` because `FFFE` and `FFFF` are
/// noncharacters in every plane, and a conforming text stack may drop or
/// replace one.
pub const NITAQ_MULHAQ_AWWAL: NitaqKhana = NitaqKhana::jadeed(0x000F_0000, 0x000F_FFFD);

/// Plane 16's private-use area: U+100000–U+10FFFD, 65,534 slots.
///
/// The second half of the supplementary pool, filled only after
/// [`NITAQ_MULHAQ_AWWAL`] is exhausted, so a patch that fits in one plane never
/// reaches the other and its assignment does not shift when it grows.
pub const NITAQ_MULHAQ_THANI: NitaqKhana = NitaqKhana::jadeed(0x0010_0000, 0x0010_FFFD);

/// The supplementary ranges, in the order they are filled.
///
/// A `static` and not a `const` so that [`NamatKhana::nitaqat`] hands out a
/// reference to one array with a real `'static` address, rather than depending
/// on constant promotion to give a temporary the lifetime it claims.
static NITAQAT_MULHAQ: [NitaqKhana; 2] = [NITAQ_MULHAQ_AWWAL, NITAQ_MULHAQ_THANI];

/// The Basic Multilingual Plane's range, alone.
static NITAQAT_ASASI: [NitaqKhana; 1] = [NITAQ_ASASI];

/// Which private-use area a transport addresses its glyphs in.
///
/// A decision the caller makes from what it knows about the export template,
/// and one the report records — see this module's header for the sixteen-bit
/// `CharType` this exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum NamatKhana {
    /// The supplementary planes: 131,068 slots, and astral codepoints.
    ///
    /// The default, because it is the one with room for a real patch.
    #[default]
    Mulhaq,
    /// The Basic Multilingual Plane's area: 6,400 slots, no astral codepoints.
    ///
    /// For an export template whose character type is sixteen bits wide, where
    /// an astral slot would arrive as a surrogate pair and match no glyph.
    Asasi,
}

impl NamatKhana {
    /// The ranges this mode draws from, in the order they are filled.
    ///
    /// The order is part of the format: slot zero is the first codepoint of the
    /// first range, and the ranges are consumed in sequence. Reordering them
    /// would renumber every slot in every existing patch.
    ///
    /// Not `const`: it hands back a reference to a `static`, which a constant
    /// may not read.
    #[must_use]
    pub fn nitaqat(self) -> &'static [NitaqKhana] {
        match self {
            Self::Mulhaq => &NITAQAT_MULHAQ,
            Self::Asasi => &NITAQAT_ASASI,
        }
    }

    /// How many slots the mode provides in total.
    #[must_use]
    pub fn siaa(self) -> u32 {
        let mut kull: u32 = 0;
        for nitaq in self.nitaqat() {
            kull = kull.saturating_add(nitaq.ittisa());
        }
        kull
    }

    /// A stable short name for logs and reports.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mulhaq => "supplementary",
            Self::Asasi => "bmp",
        }
    }

    /// The codepoint for a flat slot index, or [`None`] past the pool's end.
    #[must_use]
    pub fn khana(self, fahras: u32) -> Option<char> {
        let mut baqi = fahras;
        for nitaq in self.nitaqat() {
            let ittisa = nitaq.ittisa();
            if baqi < ittisa {
                return nitaq.khana(baqi);
            }
            baqi = baqi.checked_sub(ittisa)?;
        }
        None
    }

    /// The flat slot index for a codepoint, or [`None`] when it is not a slot.
    #[must_use]
    pub fn fahras(self, khana: char) -> Option<u32> {
        let raqm = u32::from(khana);
        let mut asas: u32 = 0;
        for nitaq in self.nitaqat() {
            if let Some(dakhil) = nitaq.fahras(raqm) {
                return asas.checked_add(dakhil);
            }
            asas = asas.checked_add(nitaq.ittisa())?;
        }
        None
    }
}

// ---------------------------------------------------------------------------
// The slot key
// ---------------------------------------------------------------------------

/// What makes one transported glyph distinct from another.
///
/// Three fields, and deliberately not five. The atlas key [`MiftahShakl`] also
/// carries a subpixel bucket, because the rasterizer draws the same glyph
/// differently at different fractional pen positions. A `BitmapFont` has exactly
/// one image per character code and cannot express that, so allocating a slot
/// per bucket would multiply the glyph table by the bucket count for images the
/// container can never tell apart — and would exhaust the pool four times faster
/// for nothing. It also carries the rasterization mode, which is a property of
/// the whole atlas and not of one glyph: an atlas is built in one mode, so every
/// key in one transport would carry the same value. Both are dropped in
/// [`MiftahKhana::min_miftah_shakl`] and there is no field here to put either
/// in.
///
/// There is also no codepoint field, and no constructor that takes a character.
/// That is invariant 1 in this module's header, and it is why this type is
/// declared before anything that allocates.
///
/// The field order is the **allocation order** and is load-bearing: the derived
/// `Ord` compares font index, then size, then glyph id, and freezing the pool
/// walks that order. Reordering these fields would renumber every slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiftahKhana {
    /// Index into the font chain of the font this glyph belongs to.
    pub khatt: u8,
    /// The pixel size in quarter-pixels, as the atlas records it.
    pub hajm_rubi: u16,
    /// The glyph identifier, in the font named by `khatt`.
    pub muarrif: u32,
}

impl MiftahKhana {
    /// Builds a key from its three parts.
    #[must_use]
    pub const fn jadeed(khatt: u8, hajm_rubi: u16, muarrif: u32) -> Self {
        Self { khatt, hajm_rubi, muarrif }
    }

    /// The key for one shaped glyph at one size.
    #[must_use]
    pub const fn min_harf(harf: &Harf, hajm_rubi: u16) -> Self {
        Self { khatt: harf.khatt, hajm_rubi, muarrif: harf.muarrif }
    }

    /// The key for an atlas entry, dropping the subpixel bucket and the
    /// rasterization mode.
    ///
    /// Both are dropped rather than carried: see this type's documentation for
    /// why a `BitmapFont` can express neither.
    #[must_use]
    pub const fn min_miftah_shakl(miftah: MiftahShakl) -> Self {
        Self { khatt: miftah.khatt, hajm_rubi: miftah.hajm_rubi, muarrif: miftah.muarrif }
    }

    /// The atlas key for this slot, at subpixel bucket zero.
    ///
    /// Bucket zero because that is the image drawn at an integral pen position,
    /// which is the only pen position a `BitmapFont` produces. `namat` is asked
    /// for rather than assumed: coverage and a distance field are two different
    /// images of one glyph, and guessing here would build a key the atlas that
    /// was actually compiled does not hold.
    #[must_use]
    pub const fn ila_miftah_shakl(self, namat: NamatSafha) -> MiftahShakl {
        MiftahShakl {
            khatt: self.khatt,
            bakat: 0,
            hajm_rubi: self.hajm_rubi,
            namat,
            muarrif: self.muarrif,
        }
    }
}

impl fmt::Display for MiftahKhana {
    /// `font 0 / 48q / glyph 1093`, which is what an error message wants.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "font {} / {}q / glyph {}", self.khatt, self.hajm_rubi, self.muarrif)
    }
}

// ---------------------------------------------------------------------------
// The pool, and the frozen assignment
// ---------------------------------------------------------------------------

/// The set of distinct glyphs a transport needs, before any slot is assigned.
///
/// A `BTreeSet` and not a `HashSet`, and the reason is not taste: freezing walks
/// this set in order and the walk decides which codepoint each glyph gets. A
/// hashed set would order the walk by a seed, two builds of one patch would
/// disagree about every slot, and a diff between them would be a hundred
/// thousand changed lines describing no change at all.
#[derive(Debug, Clone)]
pub struct HawdKhanat {
    namat: NamatKhana,
    ashkal: BTreeSet<MiftahKhana>,
}

impl HawdKhanat {
    /// An empty pool over one private-use area.
    #[must_use]
    pub const fn jadeed(namat: NamatKhana) -> Self {
        Self { namat, ashkal: BTreeSet::new() }
    }

    /// Which area this pool draws from.
    #[must_use]
    pub const fn namat(&self) -> NamatKhana {
        self.namat
    }

    /// Records that the transport needs one glyph.
    ///
    /// Idempotent: recording the same glyph twice is one slot, which is the
    /// entire point of the pool.
    pub fn sajjil(&mut self, miftah: MiftahKhana) {
        let _ = self.ashkal.insert(miftah);
    }

    /// How many distinct glyphs have been recorded.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.ashkal.len()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.ashkal.is_empty()
    }

    /// How many slots the area provides.
    #[must_use]
    pub fn siaa(&self) -> u32 {
        self.namat.siaa()
    }

    /// How many slots would still be free if the pool were frozen now.
    #[must_use]
    pub fn mutabaqqi(&self) -> u32 {
        let mustakhdam = u32::try_from(self.ashkal.len()).unwrap_or(u32::MAX);
        self.siaa().saturating_sub(mustakhdam)
    }

    /// Every recorded glyph, in allocation order.
    pub fn mafatih(&self) -> impl Iterator<Item = MiftahKhana> + '_ {
        self.ashkal.iter().copied()
    }

    /// Assigns every recorded glyph a codepoint and freezes the result.
    ///
    /// Slot *n* is the *n*-th codepoint of the mode's ranges taken in order, and
    /// *n* is the glyph's position in ascending `(font, size, glyph id)` order.
    /// Nothing about the order strings were registered in reaches this.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMumtali`] naming both numbers when the pool holds more
    /// distinct glyphs than the area addresses. Refused rather than truncated:
    /// a glyph table cut short does not draw fewer letters, it draws the wrong
    /// ones, and wrong letters read to a player as a corrupt font rather than
    /// as a limit that was reached.
    ///
    /// [`KhataLawha::NaqlMarfud`] when a codepoint could not be formed for a
    /// slot inside the declared capacity, which would mean one of this module's
    /// range constants disagrees with its own arithmetic.
    pub fn jammid(&self) -> Result<TawzeeKhanat, KhataLawha> {
        let matlub = tul_u64(self.ashkal.len());
        let mutah = u64::from(self.siaa());
        if matlub > mutah {
            return Err(KhataLawha::NaqlMumtali { matlub, mutah });
        }

        let mut ila_khana: BTreeMap<MiftahKhana, char> = BTreeMap::new();
        let mut min_khana: Vec<MiftahKhana> = Vec::with_capacity(self.ashkal.len());
        for (fahras, miftah) in self.ashkal.iter().enumerate() {
            let raqm = u32::try_from(fahras).map_err(|_| KhataLawha::NaqlMumtali {
                matlub,
                mutah,
            })?;
            let khana = self.namat.khana(raqm).ok_or_else(|| KhataLawha::NaqlMarfud {
                sabab: format!(
                    "slot {raqm} of the {} area has no codepoint, though the area \
                     declares {mutah}",
                    self.namat.ism()
                ),
            })?;
            let _ = ila_khana.insert(*miftah, khana);
            min_khana.push(*miftah);
        }

        Ok(TawzeeKhanat { namat: self.namat, ila_khana, min_khana })
    }
}

/// A frozen slot assignment: every glyph the transport carries, with its
/// codepoint.
///
/// Only obtainable from [`HawdKhanat::jammid`]. There is no public constructor
/// and no public field, so an assignment that was not produced by the
/// deterministic allocator cannot exist — which is what makes "the same patch
/// produces the same assignment" a property of the type rather than a habit of
/// its callers.
#[derive(Debug, Clone)]
pub struct TawzeeKhanat {
    namat: NamatKhana,
    ila_khana: BTreeMap<MiftahKhana, char>,
    min_khana: Vec<MiftahKhana>,
}

impl TawzeeKhanat {
    /// Which private-use area the slots were drawn from.
    #[must_use]
    pub const fn namat(&self) -> NamatKhana {
        self.namat
    }

    /// How many slots were assigned.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.min_khana.len()
    }

    /// Whether the assignment is empty.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.min_khana.is_empty()
    }

    /// How many slots the area provides.
    #[must_use]
    pub fn siaa(&self) -> u32 {
        self.namat.siaa()
    }

    /// How many slots remain unassigned.
    #[must_use]
    pub fn mutabaqqi(&self) -> u32 {
        let mustakhdam = u32::try_from(self.min_khana.len()).unwrap_or(u32::MAX);
        self.siaa().saturating_sub(mustakhdam)
    }

    /// The codepoint one glyph was assigned, if it is in this transport.
    #[must_use]
    pub fn khana(&self, miftah: MiftahKhana) -> Option<char> {
        self.ila_khana.get(&miftah).copied()
    }

    /// The glyph a codepoint addresses.
    ///
    /// Diagnostics only, and it returns a glyph key — a font index, a size and
    /// a glyph identifier. There is deliberately no inverse from a slot to a
    /// character, to a string, or to a string-table entry, anywhere in this
    /// module; see invariant 2 in the module header.
    #[must_use]
    pub fn miftah(&self, khana: char) -> Option<MiftahKhana> {
        let fahras = self.namat.fahras(khana)?;
        self.min_khana.get(usize::try_from(fahras).ok()?).copied()
    }

    /// Every assignment, in slot order, which is ascending codepoint order.
    pub fn tawzee(&self) -> impl Iterator<Item = (MiftahKhana, char)> + '_ {
        self.min_khana.iter().enumerate().filter_map(|(fahras, miftah)| {
            let raqm = u32::try_from(fahras).ok()?;
            Some((*miftah, self.namat.khana(raqm)?))
        })
    }
}

// ---------------------------------------------------------------------------
// The atlas seam
// ---------------------------------------------------------------------------

/// Where this module gets glyph images from.
///
/// Declared here rather than imported, and narrow on purpose. The transport
/// needs three facts — where a glyph's image is, how big the page holding it
/// is, and how many pages there are — and reaching into whatever type actually
/// owns the atlas would couple the last rung of the Godot 3 ladder to a
/// rasterizer's internals for the sake of three accessors.
///
/// [`LawhaJahiza`] is the plain-data implementation for a caller that already
/// has the map in hand, which is the offline compiler's case.
pub trait MasdarLawha {
    /// Where one glyph's image lives, or [`None`] when the atlas does not hold
    /// it.
    fn mawdi(&self, miftah: MiftahKhana) -> Option<MawdiShakl>;

    /// One page's width and height in texels, or [`None`] past the last page.
    fn qiyas_safha(&self, safha: u16) -> Option<(u16, u16)>;

    /// How many pages the atlas has.
    fn adad_safahat(&self) -> u16;
}

/// An atlas handed over as plain data.
///
/// A `BTreeMap` and a `Vec`, both walked in a fixed order, so a transport built
/// from one of these is reproducible from the same inputs.
#[derive(Debug, Clone, Default)]
pub struct LawhaJahiza {
    mawadi: BTreeMap<MiftahKhana, MawdiShakl>,
    safahat: Vec<(u16, u16)>,
}

impl LawhaJahiza {
    /// An empty atlas.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { mawadi: BTreeMap::new(), safahat: Vec::new() }
    }

    /// The bridge from a compiled atlas.
    ///
    /// Pages are declared in index order, so a page index in a [`MawdiShakl`]
    /// still names the same page after the copy — the transport carries the
    /// index rather than looking a page up by anything else, so a reordering
    /// here would silently move every glyph to another sheet.
    ///
    /// ## Why the key narrows, and why that can fail
    ///
    /// A compiled atlas is keyed by [`MiftahShakl`], which carries a subpixel
    /// bucket and a rasterization mode on top of the font, size and glyph id.
    /// [`MiftahKhana`] carries none of that, because the engines at the end of
    /// this ladder describe a glyph with a character code and a rectangle and
    /// have nowhere to put either — see this module's header.
    ///
    /// So two entries of a compiled atlas can narrow onto one transport key,
    /// and when they do they are two different images for what the game will
    /// draw as one glyph. That is refused rather than resolved. Keeping either
    /// one would position a glyph for a subpixel phase the engine will not
    /// reproduce, and the visible result — text that is subtly wrong on some
    /// letters and correct on the rest — is the hardest kind of defect to trace
    /// back to a compile that reported success.
    ///
    /// In practice a patch destined for this ladder is rasterized at bucket
    /// zero in one mode, so the refusal fires on a compiler that stopped doing
    /// that, which is exactly when somebody should hear about it.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMarfud`] when a page a glyph refers to was never
    /// registered with its dimensions, when the pages will not copy across in
    /// order, or when two glyph images narrow onto one transport key.
    ///
    /// Whatever [`LawhaJahiza::dif_safha`] refuses about a page's dimensions.
    pub fn min_lawha(lawha: &crate::Lawha) -> Result<Self, KhataLawha> {
        let mut jahiza = Self::jadeed();
        for fahras in 0..lawha.khareeta.adad_safahat() {
            let raqm = u16::try_from(fahras).map_err(|_| KhataLawha::NaqlMarfud {
                sabab: "the atlas holds more pages than a page index can name".to_owned(),
            })?;
            let Some((ard, irtifa)) = lawha.khareeta.abaad_safha(raqm) else {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "page {raqm} of the atlas has no recorded size, so no texture \
                         coordinate can be derived from it"
                    ),
                });
            };
            let mudraj = jahiza.dif_safha(ard, irtifa)?;
            if mudraj != raqm {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "page {raqm} of the atlas became page {mudraj} of the transport, which \
                         would move every glyph on it to another sheet"
                    ),
                });
            }
        }

        // Sorted, so a narrowing collision is reported against the same pair on
        // every run rather than against whichever of the two a hash map yielded
        // first.
        for (miftah, mawdi) in lawha.khareeta.murattaba() {
            let khana = MiftahKhana::min_miftah_shakl(miftah);
            if jahiza.safahat.get(usize::from(mawdi.safha)).is_none() {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!("{khana} sits on page {}, which the atlas does not have",
                        mawdi.safha),
                });
            }
            if let Some(sabiq) = jahiza.dif_shakl(khana, mawdi) {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "{khana} has two images in the atlas — one at page {} ({},{}) and one at \
                         page {} ({},{}) — which differ only in a subpixel bucket or a \
                         rasterization mode, and the transport can express neither",
                        sabiq.safha, sabiq.s, sabiq.a, mawdi.safha, mawdi.s, mawdi.a
                    ),
                });
            }
        }
        Ok(jahiza)
    }

    /// Declares one page's dimensions and returns its index.
    ///
    /// The only way a [`LawhaJahiza`] acquires a page, and therefore the only
    /// way anything can satisfy [`MasdarLawha::qiyas_safha`] — which
    /// [`Naql::akmil`] calls for every glyph it transports and which
    /// `taarib_muhawwil_nusus`'s GameMaker font rebuilder calls before it emits
    /// a glyph table. [`LawhaJahiza::min_lawha`] is the bridge that calls it
    /// with a compiled atlas's own pages; it stays public because an adapter
    /// assembling a transport from something that is not a [`crate::Lawha`] —
    /// a runtime atlas, a table read back out of a patch — has no other way to
    /// declare a page.
    ///
    /// Each page is declared with its own dimensions, and a caller bridging a
    /// compiled atlas must read them per page rather than assume one size:
    /// [`crate::Lawha::ibni`] crops every finished page to the rows its pack
    /// reached, so the pages of one atlas can genuinely differ.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::HajmMufrit`] when a side is zero or above
    /// [`AQSA_QIYAS_SAFHA`], and [`KhataLawha::NaqlMarfud`] when more pages are
    /// declared than a page index can name.
    pub fn dif_safha(&mut self, ard: u16, irtifa: u16) -> Result<u16, KhataLawha> {
        if ard == 0 || irtifa == 0 || ard > AQSA_QIYAS_SAFHA || irtifa > AQSA_QIYAS_SAFHA {
            return Err(KhataLawha::HajmMufrit {
                haql: "atlas page dimension",
                qeema: u64::from(ard.max(irtifa)),
                saqf: u64::from(AQSA_QIYAS_SAFHA),
            });
        }
        let fahras = u16::try_from(self.safahat.len()).map_err(|_| KhataLawha::NaqlMarfud {
            sabab: "the atlas declares more pages than a page index can name".to_owned(),
        })?;
        self.safahat.push((ard, irtifa));
        Ok(fahras)
    }

    /// Records where one glyph's image lives.
    ///
    /// Replaces a previous entry for the same glyph, and says so in the return
    /// value rather than silently: two positions for one glyph mean two callers
    /// disagree about where the image is, and only one of them can be right.
    pub fn dif_shakl(&mut self, miftah: MiftahKhana, mawdi: MawdiShakl) -> Option<MawdiShakl> {
        self.mawadi.insert(miftah, mawdi)
    }

    /// How many glyph images the atlas holds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mawadi.len()
    }
}

impl MasdarLawha for LawhaJahiza {
    fn mawdi(&self, miftah: MiftahKhana) -> Option<MawdiShakl> {
        self.mawadi.get(&miftah).copied()
    }

    fn qiyas_safha(&self, safha: u16) -> Option<(u16, u16)> {
        self.safahat.get(usize::from(safha)).copied()
    }

    fn adad_safahat(&self) -> u16 {
        u16::try_from(self.safahat.len()).unwrap_or(u16::MAX)
    }
}

// ---------------------------------------------------------------------------
// The generated resource
// ---------------------------------------------------------------------------

/// The vertical metrics the engine needs from a `BitmapFont`.
///
/// Godot 3 stores an ascent and a height and derives the descent from the two,
/// so these three have to agree or the engine's descent is a number nobody
/// chose. All three are whole pixels: the `.fnt` form carries integers, and
/// rounding here once is better than rounding differently in two writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QiyasatNaql {
    suud: i32,
    hubut: i32,
    irtifa: i32,
}

impl QiyasatNaql {
    /// Builds the metrics from a font's own, rounded to whole pixels.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::KhattMarfud`] naming the field when a value is not finite,
    /// when the ascent or the line height is not positive, when the descent is
    /// negative, or when the line height is smaller than ascent plus descent —
    /// which would make every line of the game's text overlap the one above it,
    /// and is the kind of wrong that looks like a rendering bug in the game
    /// rather than a bad number in a patch.
    pub fn jadeed(suud: f32, hubut: f32, irtifa: f32) -> Result<Self, KhataLawha> {
        let marfud = |haql: &str, qeema: f32| KhataLawha::KhattMarfud {
            sabab: format!("the transport's {haql} is {qeema}, which is not a usable metric"),
        };
        let suud = sahih_min_ashri(suud).ok_or_else(|| marfud("ascent", suud))?;
        let hubut = sahih_min_ashri(hubut).ok_or_else(|| marfud("descent", hubut))?;
        let irtifa = sahih_min_ashri(irtifa).ok_or_else(|| marfud("line height", irtifa))?;

        if suud <= 0 {
            return Err(KhataLawha::KhattMarfud {
                sabab: format!("the transport's ascent rounds to {suud}, which is not positive"),
            });
        }
        if hubut < 0 {
            return Err(KhataLawha::KhattMarfud {
                sabab: format!("the transport's descent rounds to {hubut}, which is negative"),
            });
        }
        let matlub = suud.checked_add(hubut).ok_or_else(|| KhataLawha::KhattMarfud {
            sabab: "the transport's ascent and descent do not sum to a usable height".to_owned(),
        })?;
        if irtifa < matlub {
            return Err(KhataLawha::KhattMarfud {
                sabab: format!(
                    "the transport's line height is {irtifa} and its ascent and descent need \
                     {matlub}; every line would overlap the one above it"
                ),
            });
        }
        Ok(Self { suud, hubut, irtifa })
    }

    /// How far the font rises above the baseline, in whole pixels.
    #[must_use]
    pub const fn suud(self) -> i32 {
        self.suud
    }

    /// How far it falls below, as a positive number of whole pixels.
    #[must_use]
    pub const fn hubut(self) -> i32 {
        self.hubut
    }

    /// The line height, in whole pixels.
    #[must_use]
    pub const fn irtifa(self) -> i32 {
        self.irtifa
    }
}

/// One glyph image's rectangle inside its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MustatilShakl {
    /// Left edge in the page, in texels.
    pub s: u16,
    /// Top edge in the page, in texels.
    pub a: u16,
    /// Width in texels.
    pub ard: u16,
    /// Height in texels.
    pub irtifa: u16,
}

/// One row of the generated glyph table.
///
/// The shape Godot 3's `BitmapFont::add_char(character, texture, rect, align,
/// advance)` takes, with the two halves of `align` named separately because
/// they are measured from different things and confusing them puts every glyph
/// in the font one ascent out of place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KhanaShakl {
    /// The private-use codepoint that addresses this glyph.
    ///
    /// An address, not a character. It has no Unicode meaning; see the module
    /// header.
    pub khana: char,
    /// Which texture page the image is on.
    pub safha: u16,
    /// The image's rectangle inside that page.
    pub mustatil: MustatilShakl,
    /// Horizontal offset from the pen, in pixels. Godot's `h_align`.
    pub izaha_s: i16,
    /// Vertical offset **down from the top of the line box**, in pixels.
    ///
    /// Not from the baseline. Godot 3 draws a bitmap glyph at
    /// `pos.y - ascent + v_align`, so this is `ascent` minus the image's top
    /// bearing, plus whatever the shaper's own vertical placement added.
    pub izaha_a: i16,
    /// How far the pen moves after this glyph, in pixels, before kerning.
    pub taqaddum: i16,
}

/// A per-pair advance correction: a kerning entry.
///
/// This is where the shaper's horizontal positioning is carried. A `BitmapFont`
/// gives one advance per character code, which cannot express a ligature's
/// width, a `GPOS` adjustment, or a mark's negative offset onto its base — but
/// it does have a kerning table over ordered pairs, and every pair of glyphs
/// adjacent in a transported sequence gets an entry for exactly the gap the
/// shaper produced.
///
/// ## The sign, which is a trap
///
/// [`ZawjTaqaddum::tashih`] is the **`BMFont` `amount`**: added to the first
/// glyph's advance. Godot 3's `.fnt` importer negates it on the way in
/// (`add_kerning_pair(first, second, -amount)`) and `get_char_size` then
/// subtracts what it stored, so the two negations cancel and `BMFont`'s sense is
/// preserved. A caller going through the `BitmapFont` API directly rather than
/// through a `.fnt` must therefore pass [`ZawjTaqaddum::tashih_godot`], which is
/// the negation. Getting this backwards does not lose the correction — it
/// applies it twice in the wrong direction, and every kerned pair in the game
/// is out by double.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZawjTaqaddum {
    /// The first codepoint of the ordered pair.
    pub awwal: char,
    /// The second.
    pub thani: char,
    /// The `BMFont` `amount`: added to the first glyph's advance.
    pub tashih: i16,
}

impl ZawjTaqaddum {
    /// The value `BitmapFont::add_kerning_pair` wants, which is the negation.
    ///
    /// See this type's documentation for why the two differ.
    ///
    /// Not `const`: the widening conversion goes through `From`, which is not
    /// callable in a constant, and a widening `as` here would be an `as` this
    /// file otherwise has none of.
    #[must_use]
    pub fn tashih_godot(self) -> i32 {
        i32::from(self.tashih).saturating_neg()
    }
}

/// The generated Godot 3 `BitmapFont` glyph table.
///
/// Everything the engine needs and nothing it does not: the vertical metrics,
/// one entry per page, one row per slot in ascending codepoint order, and the
/// kerning pairs that carry the shaper's horizontal positioning.
///
/// The texture pages themselves are not here. This module describes where each
/// glyph sits; producing the image files is the packaging layer's job, and a
/// transport that also wrote textures would be a transport with a second
/// purpose and a filesystem.
#[derive(Debug, Clone)]
pub struct JadwalKhattNaql {
    namat: NamatKhana,
    qiyasat: QiyasatNaql,
    safahat: Vec<(u16, u16)>,
    khanat: Vec<KhanaShakl>,
    azwaj: Vec<ZawjTaqaddum>,
}

impl JadwalKhattNaql {
    /// Which private-use area the slots came from.
    #[must_use]
    pub const fn namat(&self) -> NamatKhana {
        self.namat
    }

    /// The vertical metrics.
    #[must_use]
    pub const fn qiyasat(&self) -> QiyasatNaql {
        self.qiyasat
    }

    /// Each page's width and height in texels, by page index.
    ///
    /// Every page the atlas declared, whether or not this transport draws from
    /// it. Page indices are positional in the resource, so dropping an unused
    /// page would renumber every page after it and give a whole size of the
    /// font somebody else's texture.
    #[must_use]
    pub fn safahat(&self) -> &[(u16, u16)] {
        &self.safahat
    }

    /// The glyph rows, in ascending codepoint order.
    #[must_use]
    pub fn khanat(&self) -> &[KhanaShakl] {
        &self.khanat
    }

    /// The kerning pairs, in ascending `(first, second)` order.
    #[must_use]
    pub fn azwaj(&self) -> &[ZawjTaqaddum] {
        &self.azwaj
    }

    /// The glyph row for one codepoint.
    ///
    /// A binary search over the ascending codepoint column, not a map: the rows
    /// are already sorted because they were built in slot order, and building a
    /// map beside them would be a second copy that can disagree with the first.
    #[must_use]
    pub fn khana(&self, khana: char) -> Option<&KhanaShakl> {
        let fahras = self.khanat.binary_search_by(|saf| saf.khana.cmp(&khana)).ok()?;
        self.khanat.get(fahras)
    }

    /// The `AngelCode` `.fnt` text Godot 3's bitmap-font importer reads.
    ///
    /// One name per page, in page order. The names are written into `file="..."`
    /// and are resolved by the engine relative to the `.fnt` — which is why
    /// [`ism_safha_masmuh`] refuses anything carrying a path separator: a
    /// resource that could name `../../game.pck` would be a resource that can
    /// point the engine at the game's own assets, and this module is not
    /// allowed to do that even by accident.
    ///
    /// `common scaleW`/`scaleH` are the page texture's dimensions, and the
    /// format carries **one** pair for every page — a consumer that derives
    /// texture coordinates from the pixel rectangles below divides all of them
    /// by that single pair. So pages of different sizes cannot be described
    /// here, and a table holding some are refused rather than given the first
    /// page's dimensions and a silent lie about the rest. That was unreachable
    /// while every page in an atlas was opened and shipped at the same maximum;
    /// [`crate::Lawha::ibni`] now crops each finished page to the rows its pack
    /// reached, so a multi-page atlas whose last page is short is a real shape
    /// and this is the check that catches it.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMarfud`] when the number of names does not match the
    /// number of pages — a mismatch would silently give some page the wrong
    /// texture and draw a whole size of the font as somebody else's glyphs —
    /// when the pages are not all one size, or when a name is refused by
    /// [`ism_safha_masmuh`].
    pub fn fnt(&self, asmaa_safahat: &[&str]) -> Result<String, KhataLawha> {
        use std::fmt::Write as _;

        if asmaa_safahat.len() != self.safahat.len() {
            return Err(KhataLawha::NaqlMarfud {
                sabab: format!(
                    "the glyph table has {} page(s) and {} name(s) were supplied",
                    self.safahat.len(),
                    asmaa_safahat.len()
                ),
            });
        }
        for ism in asmaa_safahat {
            if !ism_safha_masmuh(ism) {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "\"{ism}\" is not a bare file name; a transport's page names may not \
                         carry a path separator, a quote or a control character"
                    ),
                });
            }
        }

        let (asas_ard, asas_irtifa) = self.safahat.first().copied().unwrap_or((0, 0));
        for (fahras, (ard, irtifa)) in self.safahat.iter().copied().enumerate() {
            if (ard, irtifa) != (asas_ard, asas_irtifa) {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "page {fahras} is {ard}x{irtifa} and page 0 is \
                         {asas_ard}x{asas_irtifa}; a `.fnt` declares one texture size for \
                         every page, so an atlas whose pages differ cannot be described by \
                         one and would give every page but the first the wrong scale"
                    ),
                });
            }
        }

        let mut nass = String::new();
        nass.push_str(
            "info face=\"taarib-naql\" size=0 bold=0 italic=0 charset=\"\" unicode=1 \
             stretchH=100 smooth=1 aa=1 padding=0,0,0,0 spacing=0,0\n",
        );
        let _ = writeln!(nass, 
            "common lineHeight={} base={} scaleW={asas_ard} scaleH={asas_irtifa} pages={} \
             packed=0",
            self.qiyasat.irtifa(),
            self.qiyasat.suud(),
            self.safahat.len()
        );
        for (fahras, ism) in asmaa_safahat.iter().enumerate() {
            let _ = writeln!(nass, "page id={fahras} file=\"{ism}\"");
        }

        let _ = writeln!(nass, "chars count={}", self.khanat.len());
        for saf in &self.khanat {
            let _ = writeln!(nass, 
                "char id={} x={} y={} width={} height={} xoffset={} yoffset={} xadvance={} \
                 page={} chnl=15",
                u32::from(saf.khana),
                saf.mustatil.s,
                saf.mustatil.a,
                saf.mustatil.ard,
                saf.mustatil.irtifa,
                saf.izaha_s,
                saf.izaha_a,
                saf.taqaddum,
                saf.safha
            );
        }

        let _ = writeln!(nass, "kernings count={}", self.azwaj.len());
        for zawj in &self.azwaj {
            let _ = writeln!(nass, 
                "kerning first={} second={} amount={}",
                u32::from(zawj.awwal),
                u32::from(zawj.thani),
                zawj.tashih
            );
        }
        Ok(nass)
    }

    /// Writes the `.fnt` into a sink the caller already opened.
    ///
    /// `masar` is a label: it is copied into the error message so a failure
    /// names the file the caller was writing, and it is never opened. There is
    /// deliberately no variant of this that takes a path and opens it — that is
    /// invariant 5 in the module header, and it is what keeps "this module
    /// cannot touch a game asset" a property of the code rather than a promise
    /// about its callers.
    ///
    /// # Errors
    ///
    /// Whatever [`JadwalKhattNaql::fnt`] refuses, and
    /// [`KhataLawha::KhataMalaf`] naming `masar` when the sink refuses the
    /// bytes.
    pub fn aktub(
        &self,
        masar: &Path,
        asmaa_safahat: &[&str],
        wijha: &mut dyn std::io::Write,
    ) -> Result<(), KhataLawha> {
        let nass = self.fnt(asmaa_safahat)?;
        wijha
            .write_all(nass.as_bytes())
            .map_err(|sabab| KhataLawha::KhataMalaf { masar: masar.to_path_buf(), sabab })
    }
}

/// Whether a name may be written into the generated resource's `page` line.
///
/// A bare file name and nothing else. Refused: an empty name, `.` and `..`, a
/// name carrying `/` or `\`, a double quote, or any ASCII control character.
/// The engine resolves these relative to the `.fnt`, so a name that could climb
/// out of its directory would be a generated resource pointing at the game's
/// own files.
#[must_use]
pub fn ism_safha_masmuh(ism: &str) -> bool {
    if ism.is_empty() || ism == "." || ism == ".." {
        return false;
    }
    !ism.chars().any(|harf| harf == '/' || harf == '\\' || harf == '"' || harf.is_control())
}

// ---------------------------------------------------------------------------
// A transported line
// ---------------------------------------------------------------------------

/// One line, as the sequence of slots that draws it.
///
/// **This is not text.** It is a sequence of addresses into one generated glyph
/// table, in visual order, and it is meaningful only to the table built in the
/// same call. It is not searchable, not selectable, not copyable, not readable
/// by any assistive technology, and cannot be re-shaped, re-wrapped or
/// concatenated. See the module header, which says so at more length and
/// explains why this rung is the last one.
#[derive(Clone, PartialEq, Eq)]
pub struct NassManqul {
    hawiya: u64,
    satr: u32,
    khanat: String,
    adad_ashkal: u32,
}

impl NassManqul {
    /// The key of the source string this line belongs to.
    #[must_use]
    pub const fn hawiya(&self) -> u64 {
        self.hawiya
    }

    /// Which line of that string this is, counting from zero.
    #[must_use]
    pub const fn satr(&self) -> u32 {
        self.satr
    }

    /// The slot sequence, in visual order.
    ///
    /// Named for what it is. A `&str` because that is what the engine's string
    /// type is filled from, and for no other reason: nothing here is text.
    #[must_use]
    pub fn khanat(&self) -> &str {
        &self.khanat
    }

    /// How many glyphs the line carries.
    #[must_use]
    pub const fn adad_ashkal(&self) -> u32 {
        self.adad_ashkal
    }

    /// Whether the line carries nothing, which a blank line does.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.khanat.is_empty()
    }
}

impl fmt::Debug for NassManqul {
    /// Prints the codepoints, never the characters.
    ///
    /// Invariant 3 in the module header. A derived `Debug` would put a run of
    /// private-use glyphs into any log line, panic message or diagnostics
    /// bundle that formatted one — a run somebody could copy out of a bug
    /// report and mistake for Arabic text, which is the exact confusion this
    /// module's whole argument rests on not causing.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Write as _;

        let mut khanat = String::new();
        for (fahras, harf) in self.khanat.chars().enumerate() {
            if fahras > 0 {
                khanat.push(' ');
            }
            let _ = write!(khanat, "U+{:04X}", u32::from(harf));
        }
        f.debug_struct("NassManqul")
            .field("hawiya", &self.hawiya)
            .field("satr", &self.satr)
            .field("adad_ashkal", &self.adad_ashkal)
            .field("khanat", &format_args!("[{khanat}]"))
            .finish()
    }
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// One line's entry in the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SijillNassManqul {
    /// The key of the source string.
    pub hawiya: u64,
    /// Which line of it, counting from zero.
    pub satr: u32,
    /// How many glyphs the line carries.
    pub adad_ashkal: u32,
}

/// What the transport did, in numbers.
///
/// Not an error and not a log line: a measurement the caller asked for and is
/// expected to act on. A caller that finds the conflict counts unacceptable
/// declines the transport and reports [`KhataLawha::NaqlMarfud`] up the ladder;
/// a caller that never looked would not have known there was anything to look
/// at, which is the failure this type exists to prevent.
#[derive(Debug, Clone, Default)]
pub struct TaqreerNaql {
    /// Which private-use area the slots were drawn from.
    pub namat: NamatKhana,
    /// How many slots that area provides.
    pub siaa: u32,
    /// How many were used.
    pub mustakhdam: u32,
    /// How many remain.
    pub mutabaqqi: u32,
    /// How many texture pages the glyph table references.
    pub adad_safahat: u16,
    /// How many lines were transported.
    pub adad_nusus: u32,
    /// How many glyph instances they carry in total.
    pub adad_ahruf: u64,
    /// How many kerning pairs the glyph table carries.
    pub adad_azwaj: u32,
    /// Slots that one height could not serve.
    ///
    /// A `BitmapFont` has one vertical offset per character code and no
    /// vertical kerning, so a mark that `GPOS` placed at two different heights
    /// over two different bases needs two heights from one slot. This counts
    /// the slots where that happened.
    pub khanat_mutanaziaa: u32,
    /// Glyph instances drawn at a height that was not the one shaping gave
    /// them.
    ///
    /// The number that actually matters: how many marks on screen sit slightly
    /// wrong. Zero on most Latin patches and non-zero on most Arabic ones.
    pub isti_mutanaziaa: u64,
    /// Ordered glyph pairs that one advance correction could not serve.
    pub azwaj_mutanaziaa: u32,
    /// Adjacencies drawn with a correction that was not the one shaping gave
    /// them.
    pub isti_azwaj_mutanaziaa: u64,
    /// One entry per transported line, in registration order.
    pub nusus: Vec<SijillNassManqul>,
}

impl TaqreerNaql {
    /// Whether every glyph was placed exactly where shaping put it.
    #[must_use]
    pub const fn mutabaq(&self) -> bool {
        self.isti_mutanaziaa == 0 && self.isti_azwaj_mutanaziaa == 0
    }

    /// One English sentence for a log line or an installer's summary.
    #[must_use]
    pub fn mulakhkhas(&self) -> String {
        use std::fmt::Write as _;

        let mut nass = format!(
            "the glyph transport used {} of {} {} slot(s), {} remaining, across {} line(s) \
             and {} glyph instance(s) on {} page(s) with {} kerning pair(s)",
            self.mustakhdam,
            self.siaa,
            self.namat.ism(),
            self.mutabaqqi,
            self.adad_nusus,
            self.adad_ahruf,
            self.adad_safahat,
            self.adad_azwaj
        );
        if self.mutabaq() {
            nass.push_str("; every glyph sits where shaping put it");
        } else {
            let _ = write!(nass, 
                "; {} glyph instance(s) across {} slot(s) could not keep their shaped height, \
                 and {} adjacency(ies) across {} pair(s) could not keep their shaped gap",
                self.isti_mutanaziaa,
                self.khanat_mutanaziaa,
                self.isti_azwaj_mutanaziaa,
                self.azwaj_mutanaziaa
            );
        }
        nass
    }
}

/// Everything one completed transport produced.
#[derive(Debug, Clone)]
pub struct NatijatNaql {
    /// The frozen slot assignment.
    pub tawzee: TawzeeKhanat,
    /// The generated glyph table.
    pub khatt: JadwalKhattNaql,
    /// One entry per registered line, in registration order.
    pub nusus: Vec<NassManqul>,
    /// What it did, in numbers.
    pub taqreer: TaqreerNaql,
}

// ---------------------------------------------------------------------------
// The transport
// ---------------------------------------------------------------------------

/// One glyph of a registered line, in the integer domain everything downstream
/// works in.
///
/// The positions are rounded to whole pixels here, once. A `BitmapFont`'s
/// advances and kerning amounts are integers, so the rounding is not optional —
/// doing it once, at the boundary, is what keeps two writers from rounding the
/// same number differently. It also does not accumulate: a pair's correction is
/// the difference of two already-rounded absolute positions, so the corrections
/// along a line telescope back to exactly the line's own width.
#[derive(Debug, Clone, Copy)]
struct RasdHarf {
    miftah: MiftahKhana,
    s: i32,
    a: i32,
}

/// One registered line, before any slot has been assigned.
#[derive(Debug, Clone)]
struct RasdNass {
    hawiya: u64,
    satr: u32,
    asas: i32,
    huruf: Vec<RasdHarf>,
}

/// The glyph-identifier transport: register shaped lines, then complete it.
///
/// Registration only records; nothing is assigned a codepoint until
/// [`Naql::akmil`] runs, because the assignment is a function of the whole set
/// of glyphs and a slot handed out early would depend on when it was asked for.
#[derive(Debug, Clone)]
pub struct Naql {
    qiyasat: QiyasatNaql,
    hawd: HawdKhanat,
    marasid: Vec<RasdNass>,
    adad_ahruf: u64,
}

impl Naql {
    /// A transport over one private-use area, with one set of vertical metrics.
    #[must_use]
    pub const fn jadeed(namat: NamatKhana, qiyasat: QiyasatNaql) -> Self {
        Self {
            qiyasat,
            hawd: HawdKhanat::jadeed(namat),
            marasid: Vec::new(),
            adad_ahruf: 0,
        }
    }

    /// Which private-use area this transport addresses.
    #[must_use]
    pub const fn namat(&self) -> NamatKhana {
        self.hawd.namat()
    }

    /// The vertical metrics the generated font will carry.
    #[must_use]
    pub const fn qiyasat(&self) -> QiyasatNaql {
        self.qiyasat
    }

    /// How many distinct glyphs have been recorded so far.
    #[must_use]
    pub fn adad_ashkal(&self) -> usize {
        self.hawd.adad()
    }

    /// How many lines have been registered.
    #[must_use]
    pub const fn adad_nusus(&self) -> usize {
        self.marasid.len()
    }

    /// How many slots would still be free if the transport were completed now.
    #[must_use]
    pub fn mutabaqqi(&self) -> u32 {
        self.hawd.mutabaqqi()
    }

    /// Records one laid-out line.
    ///
    /// `huruf` is one line's glyphs as `saff` produced them — the run named by a
    /// single `SatrMansuq`. The order they arrive in is not trusted: the run is
    /// put into visual order here, left to right by each cluster's base, with
    /// every combining mark kept immediately after the base it attached to. A
    /// run already in visual order is not disturbed, because the ordering is a
    /// stable sort and a sorted input is a fixed point of one.
    ///
    /// `hajm_rubi` is the pixel size in quarter-pixels, matching the atlas's own
    /// key, and it is part of every slot key: the same glyph at two sizes is two
    /// images and therefore two slots.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::HajmMufrit`] when the run is longer than
    /// [`AQSA_HURUF_NASS`] or this would be the line past [`AQSA_NUSUS`] —
    /// refused by declared size, before any memory is reserved for it.
    ///
    /// [`KhataLawha::NaqlMarfud`] when a glyph's position is not a finite
    /// number this module can round to a pixel, or when the run's base glyphs
    /// do not share one baseline. The second is the multi-line case: a
    /// transported run must be one line, because a `BitmapFont` sequence has no
    /// way to express a line break and inferring one here would mean this
    /// module deciding where lines break, which is `saff`'s decision.
    pub fn sajjil(
        &mut self,
        hawiya: u64,
        satr: u32,
        hajm_rubi: u16,
        huruf: &[Harf],
    ) -> Result<(), KhataLawha> {
        let mawjud = tul_u64(self.marasid.len());
        if mawjud >= AQSA_NUSUS {
            return Err(KhataLawha::HajmMufrit {
                haql: "transported lines",
                qeema: mawjud.saturating_add(1),
                saqf: AQSA_NUSUS,
            });
        }
        let tul = tul_u64(huruf.len());
        if tul > AQSA_HURUF_NASS {
            return Err(KhataLawha::HajmMufrit {
                haql: "glyphs in one transported line",
                qeema: tul,
                saqf: AQSA_HURUF_NASS,
            });
        }

        let tartib = tartib_basari(huruf);
        let mut sujjil: Vec<RasdHarf> = Vec::with_capacity(tartib.len());
        let mut asas: Option<i32> = None;

        for fahras in tartib {
            let Some(harf) = huruf.get(fahras) else { continue };
            let miftah = MiftahKhana::min_harf(harf, hajm_rubi);
            let ghayr_raqm = |haql: &str, qeema: f32| KhataLawha::NaqlMarfud {
                sabab: format!(
                    "glyph {fahras} of line {satr} of string {hawiya:016x} has a {haql} of \
                     {qeema}, which is not a pixel position"
                ),
            };
            let s = sahih_min_ashri(harf.s).ok_or_else(|| ghayr_raqm("horizontal origin", harf.s))?;
            let a = sahih_min_ashri(harf.a).ok_or_else(|| ghayr_raqm("vertical origin", harf.a))?;

            if !harf.alama {
                match asas {
                    None => asas = Some(a),
                    Some(mawdi) if mawdi != a => {
                        return Err(KhataLawha::NaqlMarfud {
                            sabab: format!(
                                "line {satr} of string {hawiya:016x} has base glyphs on two \
                                 baselines, {mawdi} and {a}; a transported run must be one line"
                            ),
                        });
                    }
                    Some(_) => {}
                }
            }

            self.hawd.sajjil(miftah);
            sujjil.push(RasdHarf { miftah, s, a });
        }

        self.adad_ahruf = self.adad_ahruf.saturating_add(tul_u64(sujjil.len()));
        self.marasid.push(RasdNass {
            hawiya,
            satr,
            asas: asas.unwrap_or_else(|| self.qiyasat.suud()),
            huruf: sujjil,
        });
        Ok(())
    }

    /// Assigns the slots, builds the glyph table, and emits every line.
    ///
    /// Runs in one order every time: freeze the pool, resolve every glyph
    /// against the atlas, choose each slot's vertical offset, build the rows in
    /// slot order, emit the sequences, then derive the kerning pairs from those
    /// sequences. Nothing here reads a hash map, and nothing depends on the
    /// order lines were registered in except the order they come back in.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMumtali`] when the patch needs more distinct glyphs
    /// than the private-use area addresses — refused, never truncated.
    ///
    /// [`KhataLawha::NaqlMarfud`] when the atlas does not hold a glyph the
    /// transport needs, when a glyph's image names a page the atlas does not
    /// have or a rectangle that runs off the page it names, or when an offset
    /// or advance does not fit the field the container gives it. Each of those
    /// would otherwise draw a rectangle of somebody else's texture, which reads
    /// as a corrupt font rather than as a missing glyph.
    ///
    /// [`KhataLawha::HajmMufrit`] when the derived kerning table is larger than
    /// [`AQSA_AZWAJ`].
    pub fn akmil(&self, lawha: &dyn MasdarLawha) -> Result<NatijatNaql, KhataLawha> {
        let tawzee = self.hawd.jammid()?;
        let mawadi = self.ijma_mawadi(lawha)?;
        let irtifaat = self.ijma_irtifaat(&mawadi);
        let (khanat, khanat_mutanaziaa, isti_mutanaziaa) =
            self.ibn_khanat(&tawzee, &mawadi, &irtifaat)?;

        let mut jadwal = JadwalKhattNaql {
            namat: tawzee.namat(),
            qiyasat: self.qiyasat,
            safahat: ijma_safahat(lawha),
            khanat,
            azwaj: Vec::new(),
        };

        let nusus = self.abith(&tawzee)?;
        let (azwaj, azwaj_mutanaziaa, isti_azwaj_mutanaziaa) = self.ibn_azwaj(&tawzee, &jadwal)?;
        jadwal.azwaj = azwaj;

        let taqreer = TaqreerNaql {
            namat: tawzee.namat(),
            siaa: tawzee.siaa(),
            mustakhdam: u32::try_from(tawzee.adad()).unwrap_or(u32::MAX),
            mutabaqqi: tawzee.mutabaqqi(),
            adad_safahat: u16::try_from(jadwal.safahat.len()).unwrap_or(u16::MAX),
            adad_nusus: u32::try_from(nusus.len()).unwrap_or(u32::MAX),
            adad_ahruf: self.adad_ahruf,
            adad_azwaj: u32::try_from(jadwal.azwaj.len()).unwrap_or(u32::MAX),
            khanat_mutanaziaa,
            isti_mutanaziaa,
            azwaj_mutanaziaa,
            isti_azwaj_mutanaziaa,
            nusus: self
                .marasid
                .iter()
                .map(|rasd| SijillNassManqul {
                    hawiya: rasd.hawiya,
                    satr: rasd.satr,
                    adad_ashkal: u32::try_from(rasd.huruf.len()).unwrap_or(u32::MAX),
                })
                .collect(),
        };

        if taqreer.mutabaq() {
            tracing::info!(taqreer = %taqreer.mulakhkhas(), "the glyph transport was generated");
        } else {
            tracing::warn!(
                taqreer = %taqreer.mulakhkhas(),
                "the glyph transport was generated with placements a bitmap glyph table \
                 cannot carry"
            );
        }

        Ok(NatijatNaql { tawzee, khatt: jadwal, nusus, taqreer })
    }

    /// Resolves every slot the pool holds against the atlas, once.
    ///
    /// Resolved here rather than at each use so that a missing or malformed
    /// glyph is one refusal naming the glyph, instead of the same refusal
    /// raised once per line that happened to contain it.
    fn ijma_mawadi(
        &self,
        lawha: &dyn MasdarLawha,
    ) -> Result<BTreeMap<MiftahKhana, MawdiShakl>, KhataLawha> {
        let adad_safahat = lawha.adad_safahat();
        let mut mawadi: BTreeMap<MiftahKhana, MawdiShakl> = BTreeMap::new();
        for miftah in self.hawd.mafatih() {
            let mawdi = lawha.mawdi(miftah).ok_or_else(|| KhataLawha::NaqlMarfud {
                sabab: format!("the atlas holds no image for {miftah}"),
            })?;
            if mawdi.safha >= adad_safahat {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "{miftah} names atlas page {} and the atlas has {adad_safahat}",
                        mawdi.safha
                    ),
                });
            }
            let (ard, irtifa) =
                lawha.qiyas_safha(mawdi.safha).ok_or_else(|| KhataLawha::NaqlMarfud {
                    sabab: format!("atlas page {} has no dimensions", mawdi.safha),
                })?;
            let yameen = mawdi.s.checked_add(mawdi.ard);
            let asfal = mawdi.a.checked_add(mawdi.irtifa);
            if yameen.is_none_or(|hadd| hadd > ard) || asfal.is_none_or(|hadd| hadd > irtifa) {
                return Err(KhataLawha::NaqlMarfud {
                    sabab: format!(
                        "{miftah} occupies {}x{} at ({}, {}) on a {ard}x{irtifa} page, which \
                         runs off it",
                        mawdi.ard, mawdi.irtifa, mawdi.s, mawdi.a
                    ),
                });
            }
            let _ = mawadi.insert(miftah, mawdi);
        }
        Ok(mawadi)
    }

    /// Every vertical offset each slot was asked to sit at, with a count.
    ///
    /// A base glyph always yields the same offset, because its origin is the
    /// baseline. A mark yields the baseline offset plus whatever `GPOS` moved
    /// it by, which is why this is a distribution and not a value: the same
    /// fatha over beh and over hah is one glyph at two heights, and a
    /// `BitmapFont` has one offset per code to serve both with.
    fn ijma_irtifaat(
        &self,
        mawadi: &BTreeMap<MiftahKhana, MawdiShakl>,
    ) -> BTreeMap<MiftahKhana, BTreeMap<i32, u32>> {
        let mut ihsaa: BTreeMap<MiftahKhana, BTreeMap<i32, u32>> = BTreeMap::new();
        for rasd in &self.marasid {
            for harf in &rasd.huruf {
                let Some(mawdi) = mawadi.get(&harf.miftah) else { continue };
                let Some(qeema) =
                    izahat_amudiya(self.qiyasat.suud(), mawdi.izaha_a, harf.a, rasd.asas)
                else {
                    continue;
                };
                let adad = ihsaa.entry(harf.miftah).or_default().entry(qeema).or_insert(0_u32);
                *adad = adad.saturating_add(1);
            }
        }
        ihsaa
    }

    /// Builds the glyph rows, in slot order.
    ///
    /// Returns the rows, how many slots one offset could not serve, and how
    /// many glyph instances were placed at an offset that was not theirs. The
    /// second number is the one a caller should act on: it is how many marks on
    /// screen sit slightly wrong, and it is reported rather than swallowed
    /// because a transport that quietly misplaced diacritics would be a
    /// transport lying about the shaping it claims to preserve.
    fn ibn_khanat(
        &self,
        tawzee: &TawzeeKhanat,
        mawadi: &BTreeMap<MiftahKhana, MawdiShakl>,
        irtifaat: &BTreeMap<MiftahKhana, BTreeMap<i32, u32>>,
    ) -> Result<(Vec<KhanaShakl>, u32, u64), KhataLawha> {
        let mut khanat: Vec<KhanaShakl> = Vec::with_capacity(tawzee.adad());
        let mut mutanaziaa: u32 = 0;
        let mut isti: u64 = 0;

        for (miftah, khana) in tawzee.tawzee() {
            let mawdi = mawadi.get(&miftah).ok_or_else(|| KhataLawha::NaqlMarfud {
                sabab: format!("{miftah} was assigned a slot and has no atlas image"),
            })?;
            let ihsaa = irtifaat.get(&miftah);
            let mukhtara = match ihsaa.and_then(qeema_ghaliba) {
                Some((qeema, muwafiq, kull)) => {
                    if ihsaa.is_some_and(|tawzi| tawzi.len() > 1) {
                        mutanaziaa = mutanaziaa.saturating_add(1);
                        isti = isti.saturating_add(u64::from(kull.saturating_sub(muwafiq)));
                    }
                    qeema
                }
                // Unreachable through this module's own flow — every slot came
                // from a run and every run contributed a height — and handled
                // anyway, as the glyph's own bearing, rather than by a branch
                // that would abort a build for a case nobody can produce.
                None => izahat_amudiya(self.qiyasat.suud(), mawdi.izaha_a, 0, 0).ok_or_else(
                    || KhataLawha::NaqlMarfud {
                        sabab: format!("{miftah} has no vertical offset this font can carry"),
                    },
                )?,
            };

            let izaha_a = i16::try_from(mukhtara).map_err(|_| KhataLawha::NaqlMarfud {
                sabab: format!(
                    "{miftah} needs a vertical offset of {mukhtara} pixels, which is outside \
                     what a bitmap glyph table carries"
                ),
            })?;
            let taqaddum_kamil =
                sahih_min_ashri(mawdi.taqaddum).ok_or_else(|| KhataLawha::NaqlMarfud {
                    sabab: format!("{miftah} has an advance of {} pixels", mawdi.taqaddum),
                })?;
            let taqaddum = i16::try_from(taqaddum_kamil).map_err(|_| KhataLawha::NaqlMarfud {
                sabab: format!(
                    "{miftah} has an advance of {taqaddum_kamil} pixels, which is outside what \
                     a bitmap glyph table carries"
                ),
            })?;

            khanat.push(KhanaShakl {
                khana,
                safha: mawdi.safha,
                mustatil: MustatilShakl {
                    s: mawdi.s,
                    a: mawdi.a,
                    ard: mawdi.ard,
                    irtifa: mawdi.irtifa,
                },
                izaha_s: mawdi.izaha_s,
                izaha_a,
                taqaddum,
            });
        }
        Ok((khanat, mutanaziaa, isti))
    }

    /// Emits every registered line as its slot sequence.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMarfud`] when a glyph that was recorded is not in the
    /// frozen assignment, which would mean the pool and the assignment
    /// disagree.
    fn abith(&self, tawzee: &TawzeeKhanat) -> Result<Vec<NassManqul>, KhataLawha> {
        let mut nusus: Vec<NassManqul> = Vec::with_capacity(self.marasid.len());
        for rasd in &self.marasid {
            let mut khanat = String::with_capacity(rasd.huruf.len().saturating_mul(4));
            for harf in &rasd.huruf {
                let khana = tawzee.khana(harf.miftah).ok_or_else(|| KhataLawha::NaqlMarfud {
                    sabab: format!("{} was recorded and then not assigned a slot", harf.miftah),
                })?;
                khanat.push(khana);
            }
            nusus.push(NassManqul {
                hawiya: rasd.hawiya,
                satr: rasd.satr,
                adad_ashkal: u32::try_from(rasd.huruf.len()).unwrap_or(u32::MAX),
                khanat,
            });
        }
        Ok(nusus)
    }

    /// Derives the kerning table from the shaped positions.
    ///
    /// For every pair of glyphs adjacent in an emitted sequence, the correction
    /// is the gap shaping actually left between their origins minus the advance
    /// the glyph table gives the first one. Both terms are already whole pixels,
    /// so the corrections along a line telescope back to exactly the line's own
    /// width and nothing accumulates.
    ///
    /// A pair whose correction is zero gets no entry: it would be a red-black
    /// tree node the engine allocates at load to subtract nothing.
    ///
    /// # Errors
    ///
    /// [`KhataLawha::NaqlMarfud`] when a correction is outside what the field
    /// carries, and [`KhataLawha::HajmMufrit`] when the table is larger than
    /// [`AQSA_AZWAJ`].
    fn ibn_azwaj(
        &self,
        tawzee: &TawzeeKhanat,
        jadwal: &JadwalKhattNaql,
    ) -> Result<(Vec<ZawjTaqaddum>, u32, u64), KhataLawha> {
        let mut ihsaa: BTreeMap<(char, char), BTreeMap<i32, u32>> = BTreeMap::new();
        for rasd in &self.marasid {
            for nafidha in rasd.huruf.windows(2) {
                let (Some(sabiq), Some(lahiq)) = (nafidha.first(), nafidha.get(1)) else {
                    continue;
                };
                let (Some(khana_a), Some(khana_b)) =
                    (tawzee.khana(sabiq.miftah), tawzee.khana(lahiq.miftah))
                else {
                    continue;
                };
                let Some(saf) = jadwal.khana(khana_a) else { continue };
                let Some(fajwa) = lahiq.s.checked_sub(sabiq.s) else { continue };
                let Some(tashih) = fajwa.checked_sub(i32::from(saf.taqaddum)) else { continue };
                let adad =
                    ihsaa.entry((khana_a, khana_b)).or_default().entry(tashih).or_insert(0_u32);
                *adad = adad.saturating_add(1);
            }
        }

        let mut azwaj: Vec<ZawjTaqaddum> = Vec::new();
        let mut mutanaziaa: u32 = 0;
        let mut isti: u64 = 0;
        for ((awwal, thani), tawzi) in &ihsaa {
            let Some((qeema, muwafiq, kull)) = qeema_ghaliba(tawzi) else { continue };
            if tawzi.len() > 1 {
                mutanaziaa = mutanaziaa.saturating_add(1);
                isti = isti.saturating_add(u64::from(kull.saturating_sub(muwafiq)));
            }
            if qeema == 0 {
                continue;
            }
            let tashih = i16::try_from(qeema).map_err(|_| KhataLawha::NaqlMarfud {
                sabab: format!(
                    "the pair U+{:04X}, U+{:04X} needs a {qeema} pixel correction, which is \
                     outside what a kerning entry carries",
                    u32::from(*awwal),
                    u32::from(*thani)
                ),
            })?;
            azwaj.push(ZawjTaqaddum { awwal: *awwal, thani: *thani, tashih });
        }

        let adad = tul_u64(azwaj.len());
        if adad > AQSA_AZWAJ {
            return Err(KhataLawha::HajmMufrit {
                haql: "kerning pairs in the transported font",
                qeema: adad,
                saqf: AQSA_AZWAJ,
            });
        }
        Ok((azwaj, mutanaziaa, isti))
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Every page's dimensions, by index, as the atlas reports them.
///
/// A page the atlas declines to describe stops the walk rather than leaving a
/// hole: page indices are positional in the `.fnt`, and a gap would renumber
/// every page after it and give a whole size of the font somebody else's
/// texture.
fn ijma_safahat(lawha: &dyn MasdarLawha) -> Vec<(u16, u16)> {
    let mut safahat: Vec<(u16, u16)> = Vec::new();
    for fahras in 0..lawha.adad_safahat() {
        let Some(qiyas) = lawha.qiyas_safha(fahras) else { break };
        safahat.push(qiyas);
    }
    safahat
}

/// Where a glyph's image sits, measured down from the top of the line box.
///
/// `suud` minus the image's top bearing puts the base glyph's image where the
/// font says it goes; adding the difference between this instance's origin and
/// the line's baseline carries whatever `GPOS` moved a mark by.
fn izahat_amudiya(suud: i32, izaha_a: i16, a: i32, asas: i32) -> Option<i32> {
    let fawq = suud.checked_sub(i32::from(izaha_a))?;
    fawq.checked_add(a.checked_sub(asas)?)
}

/// Puts one line's glyphs into visual order, keeping every cluster whole.
///
/// A base glyph opens a cluster and every combining mark that follows it joins
/// that cluster, so clusters are contiguous runs of the input and reordering
/// them is reordering the runs. The clusters are then **stably** sorted by their
/// base's horizontal origin, which is what puts a right-to-left line into the
/// left-to-right order a `BitmapFont` draws in, and which leaves a run that was
/// already in visual order exactly as it arrived — a sorted sequence is a fixed
/// point of a stable sort.
///
/// Marks stay immediately after their base rather than being sorted by their own
/// origin, and that is deliberate: an Arabic mark's origin frequently sits to
/// the left of the base it belongs to, so sorting marks independently would
/// scatter them onto the wrong letters.
///
/// A run that begins with a mark — a lone diacritic with no base in this line —
/// opens a cluster of its own rather than being dropped.
fn tartib_basari(huruf: &[Harf]) -> Vec<usize> {
    let mut anaqid: Vec<(usize, usize, f32)> = Vec::new();
    for (fahras, harf) in huruf.iter().enumerate() {
        if harf.alama && !anaqid.is_empty() {
            if let Some(akhir) = anaqid.last_mut() {
                akhir.1 = akhir.1.saturating_add(1);
            }
        } else {
            anaqid.push((fahras, 1, harf.s));
        }
    }

    // `total_cmp` rather than `partial_cmp`: it is a total order over every
    // bit pattern a `f32` can hold, including the NaN a corrupt layout could
    // carry, so the sort is deterministic for every input rather than for the
    // inputs somebody expected. A NaN position is refused later, by name.
    anaqid.sort_by(|awwal, thani| awwal.2.total_cmp(&thani.2));

    let mut tartib: Vec<usize> = Vec::with_capacity(huruf.len());
    for (bidaya, tul, _) in anaqid {
        for izaha in 0..tul {
            let Some(fahras) = bidaya.checked_add(izaha) else { break };
            tartib.push(fahras);
        }
    }
    tartib
}

/// The value the most instances need, and how many agreed.
///
/// Returns `(value, agreeing, total)`. Iterating a `BTreeMap` ascending and
/// keeping only a strictly larger count picks the **smallest** modal value, so
/// a tie resolves the same way on every machine and in every build — a mode
/// chosen by hash order would be a font that differs between two builds of one
/// patch for no reason anybody could see in a diff.
fn qeema_ghaliba(ihsaa: &BTreeMap<i32, u32>) -> Option<(i32, u32, u32)> {
    let mut ghaliba: Option<(i32, u32)> = None;
    let mut kull: u32 = 0;
    for (qeema, adad) in ihsaa {
        kull = kull.saturating_add(*adad);
        if ghaliba.is_none_or(|(_, sabiq)| *adad > sabiq) {
            ghaliba = Some((*qeema, *adad));
        }
    }
    ghaliba.map(|(qeema, adad)| (qeema, adad, kull))
}

/// Rounds a pixel coordinate to a whole number of pixels, exactly and without a
/// cast.
///
/// The workspace denies `cast_possible_truncation`, `cast_possible_wrap` and
/// `cast_sign_loss`, and a float-to-integer `as` is all three at once: it
/// saturates silently, so a coordinate that came out of a corrupt layout as
/// `1e30` would become `i32::MAX` and be drawn rather than refused. Rust also
/// offers no `TryFrom<f32>` for the integers.
///
/// So the conversion is done on the bits. `round` first, which produces an
/// integral value; a normal `f32` is then exactly `(2^23 + mantissa) *
/// 2^(exponent - 150)`, and because the value is integral the right shift
/// discards only zeros and the whole conversion is exact. Anything that will
/// not fit — an infinity, a NaN, a magnitude past what an `i32` holds — comes
/// back as [`None`] and becomes a named refusal at the call site instead of a
/// number nobody chose.
fn sahih_min_ashri(qeema: f32) -> Option<i32> {
    if !qeema.is_finite() {
        return None;
    }
    let mudawwar = qeema.round();
    let bitat = mudawwar.to_bits();
    let salib = (bitat & 0x8000_0000) != 0;
    let uss = (bitat >> 23) & 0xFF;
    let kasr = bitat & 0x007F_FFFF;

    // A biased exponent of zero is a zero or a subnormal, and every subnormal
    // is far below half a pixel, so `round` has already made it zero.
    if uss == 0 {
        return Some(0);
    }

    let asas = u64::from(kasr) | (1_u64 << 23);
    let izaha = i32::try_from(uss).ok()?.checked_sub(150)?;
    let mutlaq: u64 = if izaha >= 0 {
        let khatawat = u32::try_from(izaha).ok()?;
        // The mantissa is below 2^24, so a shift under forty cannot overflow a
        // `u64`; anything at or above it is already far past any pixel
        // coordinate and is refused rather than wrapped.
        if khatawat >= 40 {
            return None;
        }
        asas.checked_shl(khatawat)?
    } else {
        let khatawat = u32::try_from(izaha.checked_neg()?).ok()?;
        if khatawat >= 64 { 0 } else { asas.checked_shr(khatawat)? }
    };

    let madd = i64::try_from(mutlaq).ok()?;
    let mawqi = if salib { madd.checked_neg()? } else { madd };
    i32::try_from(mawqi).ok()
}

/// A length as a `u64`, saturating on a platform where `usize` is wider — which
/// is none this product targets, and is still not a reason to write a cast the
/// compiler cannot prove.
fn tul_u64(tul: usize) -> u64 {
    u64::try_from(tul).unwrap_or(u64::MAX)
}
