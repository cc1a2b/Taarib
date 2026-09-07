//! الجداول — the fixed-layout records five languages read by struct overlay.
//!
//! Everything in this module is a `#[repr(C)]` plain-old-data record with a
//! frozen layout. C#, JavaScript, Python and Ruby all read these tables by
//! casting a byte range to an array of structs — no parser, no allocation, no
//! per-entry work — and that is the entire reason the container is shaped this
//! way rather than as JSON or a serialization format.
//!
//! The cost of that choice is that these layouts can never change. A field may
//! be appended to a record only behind a major container version, a field is
//! never reordered or resized, and every record is documented **with its byte
//! offsets** so that a binding author in another language has something exact to
//! check their `StructLayout` or `_fields_` against. `crates/taarib-jisr/anwa.rs`
//! applies the same discipline to the ABI, for the same reason and with the same
//! consequence when it is broken.
//!
//! ## Four records here are the ABI's records, byte for byte
//!
//! [`SijillSatr`], [`SijillHarf`], [`SijillMiftahShakl`] and [`SijillMawdiShakl`]
//! are `TaaribSatr`, `TaaribHarf`, `TaaribMiftahShakl` and `TaaribMawdiShakl`
//! with different names and identical bytes. That is not duplication that
//! escaped review; it is the single most useful property of this format.
//!
//! An adapter draws two kinds of text. Text the compiler laid out in advance
//! comes out of this container. Text the compiler never saw — a player's name, a
//! composed sentence, a number substituted into a placeholder — comes out of
//! `taarib_takhtit` at run time. If those two produced different structures, the
//! mesh builder would need two input paths, and the one exercised less often
//! would be the one that was wrong. Because they are the same bytes in the same
//! order, there is one path.
//!
//! The two definitions are kept in step by the byte tables below and by the
//! size assertions beside them, not by a shared crate: `taarib-ruqaa` does not
//! depend on `taarib-jisr`, because the container format has to be readable by a
//! tool that links no C ABI at all.
//!
//! ## Why an offset pool rather than inline strings
//!
//! Every string in the container lives in one byte pool, and records point into
//! it with an offset and a length. Inline strings would make records
//! variable-length, and a variable-length record cannot be indexed — reading
//! string number nine thousand would mean walking nine thousand records instead
//! of multiplying by a stride. A game's message system asks for one string by
//! index, at a time when a frame is waiting.
//!
//! Offsets are validated against the real pool length before any of them is
//! used, every time, in [`crate::qari`]. A record in a file a stranger produced
//! is not evidence about the file.
//!
//! ## Why offsets inside a section are 32 bits
//!
//! A section may not exceed [`crate::tarwisa::AQSA_QISM_KHAAM`], which is 512
//! mebibytes, so every offset inside one fits a `u32` with three bits to spare.
//! Halving the width of every offset field halves a large part of the container,
//! and the tables are the part a game maps into its own address space.
//!
//! ## Alignment
//!
//! Each table declares the alignment its records require, and the section table
//! guarantees every section starts on a sixteen-byte boundary. Sixteen is more
//! than any record here needs; it is what makes the guarantee uniform, so a
//! reader never has to ask which table it is about to cast.

use bytemuck::{Pod, Zeroable};

use crate::aqsam::NawQism;
use crate::khata::KhataRuqaa;

/// The alignment every section start is guaranteed to satisfy.
pub const MUHADHAT_JADWAL: usize = 16;

/// How many bytes the preamble of a single-array section occupies.
pub const HAJM_TASDIR: usize = 16;

/// The same size as a `u32`, for offset arithmetic in the section's own width.
pub const HAJM_TASDIR_U32: u32 = 16;

/// How many bytes the preamble of a multi-array section occupies.
pub const HAJM_TASDIR_KABIR: usize = 32;

/// The same size as a `u32`.
pub const HAJM_TASDIR_KABIR_U32: u32 = 32;

const _: () = assert!(HAJM_TASDIR_U32 as usize == HAJM_TASDIR);
const _: () = assert!(HAJM_TASDIR_KABIR_U32 as usize == HAJM_TASDIR_KABIR);

// ---------------------------------------------------------------------------
// Section preambles
// ---------------------------------------------------------------------------

/// The string section's preamble.
///
/// ```text
/// TarwisatNusus — 32 bytes, little-endian, 8-byte aligned
///
///   offset  size  field           type   meaning
///        0     4  adad_nusus      u32    how many string records
///        4     4  adad_nitaqat    u32    how many style spans, over every string
///        8     4  izahat_nusus    u32    byte offset of the string array, from the section start
///       12     4  izahat_nitaqat  u32    byte offset of the span array
///       16     4  izahat_hawd     u32    byte offset of the UTF-8 pool
///       20     4  tul_hawd        u32    how many bytes the pool holds
///       24     8  mahjuz          u64    reserved, written as zero
/// ```
///
/// Spans live in this section rather than their own because a span is
/// meaningless without the string it covers: its `bidaya` and `tul` are byte
/// offsets into that string's translated text, and a container that could carry
/// one without the other would be a container that could carry spans pointing
/// into nothing. The array is sorted by string index, so one string's spans are
/// a contiguous run found by two partition points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatNusus {
    /// How many string records.
    pub adad_nusus: u32,
    /// How many span records, over every string.
    pub adad_nitaqat: u32,
    /// Byte offset of the string array, from the start of the section.
    pub izahat_nusus: u32,
    /// Byte offset of the span array.
    pub izahat_nitaqat: u32,
    /// Byte offset of the UTF-8 pool.
    pub izahat_hawd: u32,
    /// How many bytes the pool holds.
    pub tul_hawd: u32,
    /// Reserved. Written as zero; ignore it.
    pub mahjuz: u64,
}

/// The layout section's preamble.
///
/// ```text
/// TarwisatTakhtit — 32 bytes, little-endian, 8-byte aligned
///
///   offset  size  field             type   meaning
///        0     4  adad_takhtitat    u32    how many precomputed layouts
///        4     4  izahat_takhtitat  u32    byte offset of the layout array
///        8     4  adad_huruf        u32    how many glyphs, summed over every layout
///       12     4  izahat_huruf      u32    byte offset of the glyph array
///       16     4  adad_sutur        u32    how many lines, summed over every layout
///       20     4  izahat_sutur      u32    byte offset of the line array
///       24     8  mahjuz            u64    reserved, written as zero
/// ```
///
/// Three arrays rather than one because a layout is a head that names a run of
/// glyphs and a run of lines. Storing the runs inside the head would make the
/// head variable-length and therefore unindexable, which is the one thing the
/// whole format is arranged to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatTakhtit {
    /// How many precomputed layouts.
    pub adad_takhtitat: u32,
    /// Byte offset of the layout array, from the start of the section.
    pub izahat_takhtitat: u32,
    /// How many glyphs, summed over every layout.
    pub adad_huruf: u32,
    /// Byte offset of the glyph array.
    pub izahat_huruf: u32,
    /// How many lines, summed over every layout.
    pub adad_sutur: u32,
    /// Byte offset of the line array.
    pub izahat_sutur: u32,
    /// Reserved. Written as zero; ignore it.
    pub mahjuz: u64,
}

/// The glyph map's preamble.
///
/// ```text
/// TarwisatKhareeta — 16 bytes, little-endian, 4-byte aligned
///
///   offset  size  field           type   meaning
///        0     4  adad            u32    how many glyph images
///        4     4  izahat_mafatih  u32    byte offset of the key array
///        8     4  izahat_mawadi   u32    byte offset of the position array
///       12     4  mahjuz          u32    reserved, written as zero
/// ```
///
/// Two parallel arrays rather than one array of pairs, and this is the one place
/// in the format where a layout decision is about cache lines. A lookup binary
/// searches the keys and touches the position only once, at the end. Keys packed
/// eight bytes apart put seven of them in a cache line; interleaved with their
/// twenty-byte positions there would be two. On a glyph map of fifty thousand
/// images that is the difference between a search touching four lines and one
/// touching sixteen, on a path a game runs while a frame is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatKhareeta {
    /// How many glyph images.
    pub adad: u32,
    /// Byte offset of the key array, from the start of the section.
    pub izahat_mafatih: u32,
    /// Byte offset of the position array.
    pub izahat_mawadi: u32,
    /// Reserved. Written as zero; ignore it.
    pub mahjuz: u32,
}

/// The constraints section's preamble.
///
/// ```text
/// TarwisatQiyud — 16 bytes, little-endian, 8-byte aligned
///
///   offset  size  field   type   meaning
///        0     4  adad    u32    how many constraint records
///        4     4  izaha   u32    byte offset of the record array
///        8     8  mahjuz  u64    reserved, written as zero
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatQiyud {
    /// How many constraint records.
    pub adad: u32,
    /// Byte offset of the record array, from the start of the section.
    pub izaha: u32,
    /// Reserved. Written as zero; ignore it.
    pub mahjuz: u64,
}

/// The atlas section's preamble.
///
/// ```text
/// TarwisatLawha — 16 bytes, little-endian, 8-byte aligned
///
///   offset  size  field           type   meaning
///        0     4  adad_safahat    u32    how many pages
///        4     4  izahat_safahat  u32    byte offset of the page table
///        8     8  mahjuz          u64    reserved, written as zero
/// ```
///
/// The page table describes where each page's texels sit; the texels themselves
/// follow it in the same section. Keeping them in one section rather than two
/// means one mapping and one bounds check covers both, and a page descriptor
/// pointing outside its own section is caught by the same arithmetic that
/// bounds every other offset in the container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatLawha {
    /// How many atlas pages.
    pub adad_safahat: u32,
    /// Byte offset of the page table, from the start of the section.
    pub izahat_safahat: u32,
    /// Reserved. Written as zero; ignore it.
    pub mahjuz: u64,
}

/// The font record section's preamble.
///
/// ```text
/// TarwisatKhatt — 16 bytes, little-endian, 4-byte aligned
///
///   offset  size  field          type   meaning
///        0     4  adad_khutut    u32    how many fonts the chain declares
///        4     4  izahat_khutut  u32    byte offset of the font array
///        8     4  izahat_hawd    u32    byte offset of the UTF-8 pool of file names
///       12     4  tul_hawd       u32    how many bytes the pool holds
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct TarwisatKhatt {
    /// How many fonts the chain declares.
    pub adad_khutut: u32,
    /// Byte offset of the font array, from the start of the section.
    pub izahat_khutut: u32,
    /// Byte offset of the UTF-8 pool holding file names.
    pub izahat_hawd: u32,
    /// How many bytes the pool holds.
    pub tul_hawd: u32,
}

// ---------------------------------------------------------------------------
// Records
// ---------------------------------------------------------------------------

/// A reference into a section's byte pool.
///
/// ```text
/// MarjaNass — 8 bytes, 4-byte aligned
///
///   offset  size  field   type   meaning
///        0     4  izaha   u32    byte offset into the pool
///        4     4  tul     u32    length in bytes, UTF-8, not NUL-terminated
/// ```
///
/// Not NUL-terminated on purpose. A length-prefixed slice is what every consumer
/// wants — `ReadOnlySpan<byte>`, a `Uint8Array` view, a `memoryview` — and a
/// terminator would both cost a byte per string and let a string containing a
/// NUL truncate itself silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct MarjaNass {
    /// Byte offset into the section's pool.
    pub izaha: u32,
    /// Length in bytes.
    pub tul: u32,
}

impl MarjaNass {
    /// Whether this reference names nothing, which is how an absent optional
    /// string is written.
    #[must_use]
    pub const fn khali(self) -> bool {
        self.tul == 0
    }

    /// The byte range this reference names, checked against the pool's real
    /// length.
    ///
    /// Returns [`None`] rather than an error because the caller is walking a
    /// table and knows which record it is on; the error it raises names that
    /// record, which this function cannot.
    #[must_use]
    pub fn nitaq(self, tul_hawd: usize) -> Option<core::ops::Range<usize>> {
        let bidaya = usize::try_from(self.izaha).ok()?;
        let tul = usize::try_from(self.tul).ok()?;
        let nihaya = bidaya.checked_add(tul)?;
        (nihaya <= tul_hawd).then_some(bidaya..nihaya)
    }
}

/// One translated string.
///
/// ```text
/// SijillNass — 16 bytes, 8-byte aligned
///
///   offset  size  field    type       meaning
///        0     8  miftah   u64        BLAKE3 of the source text, first 8 bytes, little-endian
///        8     8  nass     MarjaNass  the Arabic text, into the section's pool
/// ```
///
/// **The array is sorted ascending by `miftah`.** That ordering is part of the
/// format and not an implementation detail: every consumer looks a string up by
/// hashing the source text the game handed it and binary-searching this array,
/// and a container whose keys were unsorted would not be searchable by any of
/// the five languages that read it.
///
/// The source text itself is not stored. Sixty-four bits of BLAKE3 over a
/// hundred thousand strings gives a collision probability around one in twenty
/// billion, and carrying every source string would roughly double the pool for a
/// check no consumer performs at run time. The review console works from the
/// project, which has the sources; the container is what ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillNass {
    /// The first eight bytes of BLAKE3 over the source text's UTF-8, read
    /// little-endian. The array is sorted ascending by this.
    pub miftah: u64,
    /// The Arabic text.
    pub nass: MarjaNass,
}

/// One style span over a string's clean text.
///
/// ```text
/// SijillNitaq — 24 bytes, 4-byte aligned
///
///   offset  size  field   type   meaning
///        0     4  nass    u32    index of the string this span belongs to
///        4     4  bidaya  u32    byte offset of the span's first byte in the translation
///        8     4  tul     u32    length in bytes
///       12     4  lawn    u32    colour as 0xRRGGBBAA
///       16     4  hajm    f32    size override in pixels, or zero
///       20     2  id      u16    the span's identity, as glyphs report it
///       22     2  alam    u16    flags
/// ```
///
/// **The array is sorted ascending by `nass`**, so one string's spans are a
/// contiguous run.
///
/// `id` is what [`SijillHarf::nitaq`] carries, and it is why colour survives the
/// bidirectional algorithm: a glyph moved to the other end of the line still
/// names the span it came from, so the mesh builder colours it correctly without
/// knowing anything about where it started.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillNitaq {
    /// Index of the string this span belongs to. The array is sorted by this.
    pub nass: u32,
    /// Byte offset of the span's first byte, within the translated text.
    pub bidaya: u32,
    /// Length in bytes.
    pub tul: u32,
    /// Colour as `0xRRGGBBAA`. Meaningless unless [`ALAM_NITAQ_LAWN`] is set.
    pub lawn: u32,
    /// A size override in pixels, or zero for none.
    pub hajm: f32,
    /// The span's identity, which is what a glyph's `nitaq` field names.
    pub id: u16,
    /// [`ALAM_NITAQ_MAAIL`] and the rest.
    pub alam: u16,
}

/// [`SijillNitaq::alam`]: this span is italic or slanted.
pub const ALAM_NITAQ_MAAIL: u16 = 1 << 0;

/// [`SijillNitaq::alam`]: this span is an opaque atom and is never shaped.
///
/// A format placeholder, a `noparse` run, an inline sprite. The bytes pass
/// through untouched and in their original order, because a `{0}` whose braces
/// were reordered is a crash in the game's own substitution code.
pub const ALAM_NITAQ_DHARRA: u16 = 1 << 1;

/// [`SijillNitaq::alam`]: `lawn` carries a colour. Without this flag the field
/// is zero and means nothing, which is not the same as a transparent black.
pub const ALAM_NITAQ_LAWN: u16 = 1 << 2;

/// [`SijillNitaq::alam`]: this span is bold or heavier than the run around it.
pub const ALAM_NITAQ_ASWAD: u16 = 1 << 3;

/// [`SijillNitaq::alam`]: this span is an inline sprite; `id` names the sprite.
pub const ALAM_NITAQ_SURA: u16 = 1 << 4;

/// The head of one precomputed layout.
///
/// ```text
/// SijillTakhtit — 32 bytes, 4-byte aligned
///
///   offset  size  field       type   meaning
///        0     4  nass        u32    index into the string table of the string this laid out
///        4     4  awwal_harf  u32    index of the first glyph in the section's glyph array
///        8     4  adad_huruf  u32    how many glyphs
///       12     4  awwal_satr  u32    index of the first line in the section's line array
///       16     4  adad_sutur  u32    how many lines
///       20     4  ard         f32    width of the widest line, in pixels
///       24     4  irtifa      f32    total height of every line box, in pixels
///       28     2  hajm_rubi   u16    the size it was laid out at, in quarter pixels
///       30     2  alam        u16    flags
/// ```
///
/// **The array is sorted ascending by `nass`, then by `hajm_rubi`.** One string
/// may therefore carry several layouts, which it has to: a game draws the same
/// label at one size in a menu and another in a tooltip, and a format that
/// allowed one layout per string would have to pick one of them and let the
/// other be laid out at run time — visibly disagreeing with the text beside it
/// about justification and diacritics.
///
/// `hajm_rubi` is quarter pixels and not a `f32`, deliberately: it is the same
/// unit and the same quantization as [`SijillMiftahShakl::hajm_rubi`], so a
/// layout and the glyph images it refers to can never disagree about what size
/// means. Two floats that ought to be equal and are not is a class of bug this
/// format simply does not have.
///
/// Note also what `hajm_rubi` is *not*: it is not the size that was requested.
/// When the overflow policy shrank the text to fit, this is what it settled on.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillTakhtit {
    /// Index into the string table of the string this laid out.
    pub nass: u32,
    /// Index of the first glyph.
    pub awwal_harf: u32,
    /// How many glyphs.
    pub adad_huruf: u32,
    /// Index of the first line.
    pub awwal_satr: u32,
    /// How many lines.
    pub adad_sutur: u32,
    /// The width of the widest line, in pixels.
    pub ard: f32,
    /// The total height of every line box, in pixels.
    pub irtifa: f32,
    /// The size the text was finally laid out at, in quarter pixels.
    pub hajm_rubi: u16,
    /// [`ALAM_TAKHTIT_YAMEEN`] and the rest.
    pub alam: u16,
}

impl SijillTakhtit {
    /// The size this was laid out at, in pixels.
    #[must_use]
    pub fn hajm(&self) -> f32 {
        f32::from(self.hajm_rubi) / 4.0
    }

    /// The sort key the layout array is ordered by, and searched on.
    #[must_use]
    pub const fn miftah(&self) -> (u32, u16) {
        (self.nass, self.hajm_rubi)
    }
}

/// [`SijillTakhtit::alam`]: the layout's overall direction is right to left.
pub const ALAM_TAKHTIT_YAMEEN: u16 = 1 << 0;

/// [`SijillTakhtit::alam`]: the overflow policy truncated this text.
pub const ALAM_TAKHTIT_MAQSUS: u16 = 1 << 1;

/// [`SijillTakhtit::alam`]: the text did not fit the bounds it was measured
/// against, and shipped anyway.
///
/// The overflow report names it; the flag is here so an adapter can log which
/// strings were known to be tight.
pub const ALAM_TAKHTIT_TAJAWUZ: u16 = 1 << 2;

/// [`SijillTakhtit::alam`]: the overflow policy shrank this text, so
/// `hajm_rubi` is smaller than the size the game asked for.
pub const ALAM_TAKHTIT_MUSAGHGHAR: u16 = 1 << 3;

/// [`SijillTakhtit::alam`]: this layout was computed with **no width
/// constraint**, because none was ever measured for the string.
///
/// The four flags above all describe a layout that had bounds. This one says
/// there were none: `ard` is the width the text wanted rather than a width it
/// was fitted into, and line breaking never ran, so the record is one line
/// however narrow the panel it lands in turns out to be.
///
/// Defined here rather than in the compiler that sets it, even though nothing
/// in this crate reads it. Every bit of this `u16` is one namespace, and a
/// second crate assigning positions in it is a collision waiting for whoever
/// adds bit four here next — a defect that would produce two flags with one
/// meaning and no compile error anywhere.
pub const ALAM_TAKHTIT_BILA_QAYD_ARD: u16 = 1 << 4;

/// [`SijillTakhtit::alam`]: this layout contains format placeholders, measured
/// at the width of their own raw text.
///
/// An adapter that substitutes a value into a placeholder **must lay the string
/// out again**: every position after the atom was computed from `%s`, not from
/// the eleven characters the game is about to put there.
pub const ALAM_TAKHTIT_DHARRAT: u16 = 1 << 5;

/// One laid-out line.
///
/// ```text
/// SijillSatr — 48 bytes, 4-byte aligned
///
///   offset  size  field             type   meaning
///        0     4  awwal_harf        u32    index of this line's first glyph
///        4     4  adad_huruf        u32    how many glyphs this line has
///        8     4  bidayat_mantiqi   u32    first byte of the logical text this line covers
///       12     4  nihayat_mantiqi   u32    one past the last byte
///       16     4  asas              f32    baseline, pixels down from the layout's top
///       20     4  bidaya            f32    where the line box begins horizontally
///       24     4  ard               f32    measured width of the line's content
///       28     4  irtifa            f32    the line box's height
///       32     4  suud              f32    ascent above the baseline
///       36     4  hubut             f32    descent below it
///       40     4  dabt              f32    how much width justification added
///       44     4  alam              u32    flags
/// ```
///
/// Byte-identical to `TaaribSatr` in the ABI. See this module's header for why
/// that matters more than it looks like it should.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillSatr {
    /// Index of this line's first glyph.
    pub awwal_harf: u32,
    /// How many glyphs it has.
    pub adad_huruf: u32,
    /// First byte of the logical text this line covers.
    pub bidayat_mantiqi: u32,
    /// One past the last byte.
    pub nihayat_mantiqi: u32,
    /// Baseline, in pixels down from the layout's top.
    pub asas: f32,
    /// Where the line box begins horizontally, after alignment.
    pub bidaya: f32,
    /// The measured width of the line's content.
    pub ard: f32,
    /// The line box's height.
    pub irtifa: f32,
    /// How far the tallest content rises above the baseline.
    pub suud: f32,
    /// How far the deepest content falls below it.
    pub hubut: f32,
    /// How much width justification added to this line.
    pub dabt: f32,
    /// [`ALAM_SATR_AKHIR`] and [`ALAM_SATR_YAMEEN`].
    pub alam: u32,
}

/// [`SijillSatr::alam`]: this is the last line of the layout.
pub const ALAM_SATR_AKHIR: u32 = 1 << 0;

/// [`SijillSatr::alam`]: this line's direction is right to left.
pub const ALAM_SATR_YAMEEN: u32 = 1 << 1;

/// One positioned glyph.
///
/// ```text
/// SijillHarf — 24 bytes, 4-byte aligned
///
///   offset  size  field      type   meaning
///        0     4  muarrif    u32    glyph id, in the font at `khatt`
///        4     4  anqud      u32    byte offset into the text of the cluster this came from
///        8     4  s          f32    x, pixels from the layout's left edge
///       12     4  a          f32    y, pixels down from the layout's top
///       16     4  taqaddum   f32    the advance this glyph contributed
///       20     2  nitaq      u16    the style span it inherited
///       22     1  khatt      u8     index into the patch's font chain
///       23     1  alam       u8     flags
/// ```
///
/// Byte-identical to `TaaribHarf` in the ABI.
///
/// Note what is absent, here as there: a codepoint. Nothing downstream of
/// shaping is given the character a glyph came from, because a field carrying it
/// would eventually be drawn from — which is the presentation-form pipeline this
/// product exists to make impossible.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillHarf {
    /// The glyph identifier.
    pub muarrif: u32,
    /// Byte offset of the cluster this glyph belongs to.
    pub anqud: u32,
    /// Horizontal position of the glyph's origin.
    pub s: f32,
    /// Vertical position of the origin.
    pub a: f32,
    /// The advance this glyph contributed. Zero for marks.
    pub taqaddum: f32,
    /// The style span this glyph inherited, so colour survives reordering.
    pub nitaq: u16,
    /// Index into the patch's font chain.
    pub khatt: u8,
    /// [`ALAM_HARF_ALAMA`].
    pub alam: u8,
}

/// [`SijillHarf::alam`]: a combining mark, positioned onto a base rather than
/// advancing the pen.
pub const ALAM_HARF_ALAMA: u8 = 1 << 0;

/// Everything that makes one rasterized image of a glyph distinct.
///
/// ```text
/// SijillMiftahShakl — 8 bytes, 4-byte aligned
///
///   offset  size  field      type   meaning
///        0     4  muarrif    u32    the glyph identifier
///        4     2  hajm_rubi  u16    the pixel size, in quarter pixels
///        6     1  khatt      u8     index into the font chain
///        7     1  bakat      u8     the subpixel bucket
/// ```
///
/// Byte-identical to `TaaribMiftahShakl` in the ABI. **The key array is sorted
/// ascending by these bytes read as a little-endian `u64`**, which — because the
/// fields are laid out in this order — is the same as sorting by `muarrif`, then
/// `hajm_rubi`, then `khatt`, then `bakat`. A consumer may therefore search it
/// as an array of `u64` without unpacking anything, and several do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillMiftahShakl {
    /// The glyph identifier.
    pub muarrif: u32,
    /// The pixel size, in quarter pixels.
    pub hajm_rubi: u16,
    /// Index into the font chain.
    pub khatt: u8,
    /// The subpixel bucket.
    pub bakat: u8,
}

impl SijillMiftahShakl {
    /// The key as the single number the array is ordered by.
    ///
    /// Little-endian, so the least significant byte is `muarrif`'s first byte
    /// and the most significant is `bakat` — which makes the numeric order the
    /// field order stated above.
    #[must_use]
    pub fn raqm(self) -> u64 {
        u64::from(self.muarrif)
            | (u64::from(self.hajm_rubi) << 32)
            | (u64::from(self.khatt) << 48)
            | (u64::from(self.bakat) << 56)
    }

    /// The pixel size this image was rasterized at.
    #[must_use]
    pub fn hajm(self) -> f32 {
        f32::from(self.hajm_rubi) / 4.0
    }
}

/// Where a glyph image lives in the atlas, and how to draw it.
///
/// ```text
/// SijillMawdiShakl — 20 bytes, 4-byte aligned
///
///   offset  size  field      type   meaning
///        0     4  taqaddum   f32    the glyph's advance at this size
///        4     2  s          u16    left edge in the page, pixels
///        6     2  a          u16    top edge in the page, pixels
///        8     2  ard        u16    width in pixels
///       10     2  irtifa     u16    height in pixels
///       12     2  izaha_s    i16    left bearing
///       14     2  izaha_a    i16    top bearing
///       16     2  safha      u16    which atlas page
///       18     2  hashw      u16    padding, written as zero
/// ```
///
/// Byte-identical to `TaaribMawdiShakl` in the ABI. Parallel to the key array:
/// position `n` belongs to key `n`.
///
/// Both bearings are stored rather than one being derived. Deriving the top
/// bearing from the size and the font's ascent is exact only for a glyph whose
/// ink reaches the ascent line, which is most of them and not all of them — a
/// `hamza` sits well below it and a `lam` carrying a `shadda` may exceed it. Two
/// bytes per glyph is the cheaper mistake.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillMawdiShakl {
    /// The glyph's advance at this size.
    pub taqaddum: f32,
    /// Left edge in the page.
    pub s: u16,
    /// Top edge in the page.
    pub a: u16,
    /// Width in pixels.
    pub ard: u16,
    /// Height in pixels.
    pub irtifa: u16,
    /// Left bearing: how far right of the pen the image starts.
    pub izaha_s: i16,
    /// Top bearing: how far above the baseline the image's top edge sits.
    pub izaha_a: i16,
    /// Which atlas page.
    pub safha: u16,
    /// Padding to a multiple of four. Written as zero; ignore it.
    pub hashw: u16,
}

impl SijillMawdiShakl {
    /// Whether this glyph has no image, which is the correct answer for a space.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ard == 0 || self.irtifa == 0
    }
}

/// Everything the compiler measured about the space one string is drawn into,
/// and what the patch permits an adapter to change about it.
///
/// ```text
/// SijillQayd — 48 bytes, 4-byte aligned
///
///   offset  size  field            type   meaning
///        0     4  nass             u32    index into the string table
///        4     4  ard_mutah        f32    width available in pixels; zero or less is unbounded
///        8     4  irtifa_mutah     f32    height available; zero or less is unbounded
///       12     4  hajm             f32    the size the game draws the original at
///       16     4  hajm_adna        f32    the smallest size auto-sizing may use
///       20     4  irtifa_satr      f32    line height; zero or less lets the font decide
///       24     4  tabaud_ahruf     f32    extra spacing between glyphs
///       28     4  tabaud_kalimat   f32    extra spacing added to every space
///       32     4  alam             u32    permissions, per the ALAM_QAYD_* constants
///       36     1  muhadhaha        u8     where the line sits
///       37     1  ittijah          u8     how base direction is decided
///       38     1  dabt             u8     how surplus width is absorbed
///       39     1  tashkeel         u8     the diacritics policy
///       40     1  arqam            u8     the digits policy
///       41     1  tajawuz          u8     the overflow policy
///       42     1  lugha            u8     the declared language
///       43     1  hashw0           u8     padding, written as zero
///       44     4  hashw1           u32    padding, written as zero
/// ```
///
/// **The array is sorted ascending by `nass`.**
///
/// This is the record that makes runtime text behave like precompiled text. When
/// a string reaches a takeover point the compiler never saw — a player name, a
/// composed sentence, a number substituted into a placeholder — the adapter
/// still knows the slot it is going into, and turns this row straight into the
/// layout options for it. Without it, unknown text would be laid out with
/// defaults and would visibly disagree with the text beside it about alignment,
/// justification and diacritics.
///
/// The seven policy discriminants are stored as bytes and are **widened, not
/// validated**, by every consumer. Each of the ABI's policy enumerations
/// documents an unrecognised number as resolving to its own default, so a
/// container from a newer compiler that names a policy this build has never
/// heard of lays the text out with the default instead of refusing to draw it.
/// Text in the wrong justification mode is a flaw; no text is a broken patch.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillQayd {
    /// Index into the string table of the string this constrains.
    pub nass: u32,
    /// The width available in pixels; zero or less means unbounded.
    pub ard_mutah: f32,
    /// The height available in pixels; zero or less means unbounded.
    pub irtifa_mutah: f32,
    /// The size the game draws the original at, in pixels.
    pub hajm: f32,
    /// The smallest size auto-sizing may use, in pixels.
    pub hajm_adna: f32,
    /// Line height in pixels; zero or less lets the font's metrics decide.
    pub irtifa_satr: f32,
    /// Extra spacing between glyphs, in pixels.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space, in pixels.
    pub tabaud_kalimat: f32,
    /// [`ALAM_QAYD_MIRAH`] and the rest.
    pub alam: u32,
    /// Where the line sits: 0 leading edge, 1 trailing edge, 2 centre, 3 filled.
    pub muhadhaha: u8,
    /// Base direction: 0 automatic, 1 right to left, 2 left to right.
    pub ittijah: u8,
    /// Surplus width: 0 none, 1 spaces, 2 kashida, 3 kashida then spaces.
    pub dabt: u8,
    /// Diacritics: 0 keep, 1 strip, 2 keep in dialogue only.
    pub tashkeel: u8,
    /// Digits: 0 leave, 1 European, 2 Arabic-Indic, 3 Eastern Arabic-Indic.
    pub arqam: u8,
    /// Overflow: 0 report, 1 shrink to fit, 2 truncate.
    pub tajawuz: u8,
    /// Language: 0 detect, 1 Arabic, 2 Persian, 3 Urdu, 4 Latin.
    pub lugha: u8,
    /// Padding to a multiple of four. Written as zero; ignore it.
    pub hashw0: u8,
    /// Padding to a multiple of eight. Written as zero; ignore it.
    pub hashw1: u32,
}

/// [`SijillQayd::alam`]: this element may be mirrored for a right-to-left
/// interface, so an adapter may flip its anchors and reverse its layout group.
pub const ALAM_QAYD_MIRAH: u32 = 1 << 0;

/// [`SijillQayd::alam`]: the container may be grown horizontally to fit.
pub const ALAM_QAYD_NAMU_ARD: u32 = 1 << 1;

/// [`SijillQayd::alam`]: the container may be grown vertically to fit.
pub const ALAM_QAYD_NAMU_IRTIFA: u32 = 1 << 2;

/// [`SijillQayd::alam`]: the element's anchoring may be re-pinned when it is
/// mirrored or grown.
pub const ALAM_QAYD_IADAT_TATHBIT: u32 = 1 << 3;

/// [`SijillQayd::alam`]: the engine refuses to wrap this string.
pub const ALAM_QAYD_SATR_WAHID: u32 = 1 << 4;

/// [`SijillQayd::alam`]: this string is dialogue, which is what the
/// keep-diacritics-in-dialogue policy keys on.
pub const ALAM_QAYD_HIWAR: u32 = 1 << 5;

/// [`SijillQayd::alam`]: auto-sizing is on for this element, so `hajm_adna`
/// means something.
pub const ALAM_QAYD_HAJM_TILQAI: u32 = 1 << 6;

/// Where one atlas page's texels sit inside the atlas section.
///
/// ```text
/// SijillSafha — 16 bytes, 4-byte aligned
///
///   offset  size  field    type   meaning
///        0     4  izaha    u32    byte offset of the texels, from the section start
///        4     4  tul      u32    how many texel bytes
///        8     2  ard      u16    width in texels
///       10     2  irtifa   u16    height in texels
///       12     4  hashw    u32    padding, written as zero
/// ```
///
/// One byte per texel, row-major from the top, with no row padding — the same
/// buffer shape `taarib_lawha_safha` hands back for a page rasterized at run
/// time, so an adapter has one upload path rather than two.
///
/// `tul` must equal `ard` times `irtifa`, and [`crate::qari`] checks it, because
/// a page whose byte count disagrees with its dimensions uploads a texture whose
/// rows are offset from the rectangles the glyph map names — and every glyph
/// then draws slightly wrong, in a way that looks like a font bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillSafha {
    /// Byte offset of the page's texels, from the start of the section.
    pub izaha: u32,
    /// How many texel bytes.
    pub tul: u32,
    /// Width in texels.
    pub ard: u16,
    /// Height in texels.
    pub irtifa: u16,
    /// Padding to a multiple of eight. Written as zero; ignore it.
    pub hashw: u32,
}

impl SijillSafha {
    /// How many bytes the dimensions imply, or [`None`] on overflow.
    #[must_use]
    pub fn tul_madum(self) -> Option<u32> {
        u32::from(self.ard).checked_mul(u32::from(self.irtifa))
    }
}

/// Which font the patch declares at one position of its chain.
///
/// ```text
/// SijillKhatt — 16 bytes, 4-byte aligned
///
///   offset  size  field         type       meaning
///        0     8  ism           MarjaNass  the file name, into the section's pool
///        8     4  izahat_basma  u32        byte offset of this font's 32-byte BLAKE3 hash
///       12     2  fahras        u16        the position this font takes in the chain
///       14     2  alam          u16        flags
/// ```
///
/// A record, not a font. **The container carries no font bytes at all.** The
/// files live beside the plugin, put there by the installer; what this section
/// provides is the name to look for, the position in the chain that
/// [`SijillHarf::khatt`] indexes into, and the hash to check the file against —
/// so a font replaced on disk after installation is caught rather than shaped
/// with.
///
/// This is Decision 5 in the format: no game asset and no game font is ever
/// inside a `.ruqaa`, and there is no field here that could carry one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct SijillKhatt {
    /// The font file's name, into the section's pool.
    pub ism: MarjaNass,
    /// Byte offset, from the start of the section, of this font's 32-byte
    /// BLAKE3 content hash.
    pub izahat_basma: u32,
    /// The position this font takes in the patch's font chain.
    pub fahras: u16,
    /// [`ALAM_KHATT_ASASI`] and the rest.
    pub alam: u16,
}

/// [`SijillKhatt::alam`]: the primary font. Exactly one font in a chain carries
/// this, and it is the one a glyph is looked for in first.
pub const ALAM_KHATT_ASASI: u16 = 1 << 0;

/// [`SijillKhatt::alam`]: a fallback, consulted in chain order when the primary
/// has no glyph for a character.
pub const ALAM_KHATT_IHTIYATI: u16 = 1 << 1;

/// [`SijillKhatt::alam`]: this font was verified to carry complete Arabic
/// OpenType tables — `init`, `medi`, `fina`, `rlig`, `mark` and `mkmk` — when
/// the patch was compiled.
///
/// Decision 6 refuses to ship a font without them, and this bit records that
/// the check ran rather than being assumed.
pub const ALAM_KHATT_JADAWIL_KAMILA: u16 = 1 << 2;

// ---------------------------------------------------------------------------
// Layout assertions
// ---------------------------------------------------------------------------

const _: () = assert!(size_of::<TarwisatNusus>() == 32);
const _: () = assert!(size_of::<TarwisatTakhtit>() == 32);
const _: () = assert!(size_of::<TarwisatKhareeta>() == HAJM_TASDIR);
const _: () = assert!(size_of::<TarwisatQiyud>() == HAJM_TASDIR);
const _: () = assert!(size_of::<TarwisatLawha>() == HAJM_TASDIR);
const _: () = assert!(size_of::<TarwisatKhatt>() == HAJM_TASDIR);
const _: () = assert!(size_of::<TarwisatNusus>() == HAJM_TASDIR_KABIR);
const _: () = assert!(size_of::<TarwisatTakhtit>() == HAJM_TASDIR_KABIR);

const _: () = assert!(size_of::<MarjaNass>() == 8);
const _: () = assert!(size_of::<SijillNass>() == 16);
const _: () = assert!(size_of::<SijillNitaq>() == 24);
const _: () = assert!(size_of::<SijillTakhtit>() == 32);
const _: () = assert!(size_of::<SijillSatr>() == 48);
const _: () = assert!(size_of::<SijillHarf>() == 24);
const _: () = assert!(size_of::<SijillMiftahShakl>() == 8);
const _: () = assert!(size_of::<SijillMawdiShakl>() == 20);
const _: () = assert!(size_of::<SijillQayd>() == 48);
const _: () = assert!(size_of::<SijillSafha>() == 16);
const _: () = assert!(size_of::<SijillKhatt>() == 16);

// Every record must be readable off a sixteen-byte boundary, which is what the
// section table guarantees. None of these may exceed it.
const _: () = assert!(align_of::<SijillNass>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillNitaq>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillTakhtit>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillSatr>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillHarf>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillMiftahShakl>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillMawdiShakl>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillQayd>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillSafha>() <= MUHADHAT_JADWAL);
const _: () = assert!(align_of::<SijillKhatt>() <= MUHADHAT_JADWAL);

// ---------------------------------------------------------------------------
// Preamble validation
// ---------------------------------------------------------------------------

/// One array's extent inside a section, as a preamble declares it.
///
/// Every preamble check reduces to the same three questions — does the array
/// start after the preamble, does its length overflow, and does it end inside
/// the section — so they are asked in one place rather than six.
#[derive(Debug, Clone, Copy)]
pub struct MadaMasfufa {
    /// Which field named the offset, for the refusal.
    pub haql_izaha: &'static str,
    /// Which field named the count.
    pub haql_adad: &'static str,
    /// The declared offset from the section's start.
    pub izaha: u32,
    /// The declared record count.
    pub adad: u32,
    /// This build's size for one record.
    pub khatwa: usize,
}

/// Checks every array of a section against the section's real length.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
pub fn tahaqquq_mada(
    naw: NawQism,
    tasdir: u32,
    tul_qism: u64,
    madayat: &[MadaMasfufa],
) -> Result<(), KhataRuqaa> {
    let raqm = naw.raqm();
    let talif = |haql: &'static str, qeema: u64, hadd: u64| KhataRuqaa::JadwalTalif {
        naw: raqm,
        haql,
        qeema,
        hadd,
    };
    for mada in madayat {
        if mada.izaha < tasdir {
            return Err(talif(
                mada.haql_izaha,
                u64::from(mada.izaha),
                u64::from(tasdir),
            ));
        }
        let khatwa = u64::try_from(mada.khatwa).unwrap_or(u64::MAX);
        let nihaya = u64::from(mada.adad)
            .checked_mul(khatwa)
            .and_then(|tul| u64::from(mada.izaha).checked_add(tul))
            .ok_or_else(|| talif(mada.haql_adad, u64::from(mada.adad), tul_qism))?;
        if nihaya > tul_qism {
            return Err(talif(mada.haql_adad, nihaya, tul_qism));
        }
    }
    Ok(())
}

/// Checks a byte range — a string pool, a run of texels — against the section.
///
/// # Errors
///
/// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
pub fn tahaqquq_hawd(
    naw: NawQism,
    tasdir: u32,
    tul_qism: u64,
    haql_izaha: &'static str,
    haql_tul: &'static str,
    izaha: u32,
    tul: u32,
) -> Result<(), KhataRuqaa> {
    let raqm = naw.raqm();
    let talif = |haql: &'static str, qeema: u64, hadd: u64| KhataRuqaa::JadwalTalif {
        naw: raqm,
        haql,
        qeema,
        hadd,
    };
    // An empty pool is written at the first byte past the records rather than at
    // zero. Writing it at zero would place it inside the preamble, and a reader
    // cannot tell that apart from an offset that was never filled in.
    if izaha < tasdir {
        return Err(talif(haql_izaha, u64::from(izaha), u64::from(tasdir)));
    }
    let nihaya = u64::from(izaha)
        .checked_add(u64::from(tul))
        .ok_or_else(|| talif(haql_tul, u64::from(tul), tul_qism))?;
    if nihaya > tul_qism {
        return Err(talif(haql_tul, nihaya, tul_qism));
    }
    Ok(())
}

impl TarwisatNusus {
    /// Checks both arrays and the pool against the section they live in.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(&self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Nusus,
            HAJM_TASDIR_KABIR_U32,
            tul_qism,
            &[
                MadaMasfufa {
                    haql_izaha: "izahat_nusus",
                    haql_adad: "adad_nusus",
                    izaha: self.izahat_nusus,
                    adad: self.adad_nusus,
                    khatwa: size_of::<SijillNass>(),
                },
                MadaMasfufa {
                    haql_izaha: "izahat_nitaqat",
                    haql_adad: "adad_nitaqat",
                    izaha: self.izahat_nitaqat,
                    adad: self.adad_nitaqat,
                    khatwa: size_of::<SijillNitaq>(),
                },
            ],
        )?;
        tahaqquq_hawd(
            NawQism::Nusus,
            HAJM_TASDIR_KABIR_U32,
            tul_qism,
            "izahat_hawd",
            "tul_hawd",
            self.izahat_hawd,
            self.tul_hawd,
        )
    }
}

impl TarwisatTakhtit {
    /// Checks all three arrays against the section they live in.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(&self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Takhtit,
            HAJM_TASDIR_KABIR_U32,
            tul_qism,
            &[
                MadaMasfufa {
                    haql_izaha: "izahat_takhtitat",
                    haql_adad: "adad_takhtitat",
                    izaha: self.izahat_takhtitat,
                    adad: self.adad_takhtitat,
                    khatwa: size_of::<SijillTakhtit>(),
                },
                MadaMasfufa {
                    haql_izaha: "izahat_huruf",
                    haql_adad: "adad_huruf",
                    izaha: self.izahat_huruf,
                    adad: self.adad_huruf,
                    khatwa: size_of::<SijillHarf>(),
                },
                MadaMasfufa {
                    haql_izaha: "izahat_sutur",
                    haql_adad: "adad_sutur",
                    izaha: self.izahat_sutur,
                    adad: self.adad_sutur,
                    khatwa: size_of::<SijillSatr>(),
                },
            ],
        )
    }
}

impl TarwisatKhareeta {
    /// Checks both parallel arrays against the section they live in.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Khareeta,
            HAJM_TASDIR_U32,
            tul_qism,
            &[
                MadaMasfufa {
                    haql_izaha: "izahat_mafatih",
                    haql_adad: "adad",
                    izaha: self.izahat_mafatih,
                    adad: self.adad,
                    khatwa: size_of::<SijillMiftahShakl>(),
                },
                MadaMasfufa {
                    haql_izaha: "izahat_mawadi",
                    haql_adad: "adad",
                    izaha: self.izahat_mawadi,
                    adad: self.adad,
                    khatwa: size_of::<SijillMawdiShakl>(),
                },
            ],
        )
    }
}

impl TarwisatQiyud {
    /// Checks the record array against the section it lives in.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Qiyud,
            HAJM_TASDIR_U32,
            tul_qism,
            &[MadaMasfufa {
                haql_izaha: "izaha",
                haql_adad: "adad",
                izaha: self.izaha,
                adad: self.adad,
                khatwa: size_of::<SijillQayd>(),
            }],
        )
    }
}

impl TarwisatLawha {
    /// Checks the page table against the section it lives in.
    ///
    /// Does not check the pages' texels: those are described by the page table
    /// itself, and validating them means reading records this function has only
    /// just proved are in range. [`crate::qari`] does it, in that order.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Lawha,
            HAJM_TASDIR_U32,
            tul_qism,
            &[MadaMasfufa {
                haql_izaha: "izahat_safahat",
                haql_adad: "adad_safahat",
                izaha: self.izahat_safahat,
                adad: self.adad_safahat,
                khatwa: size_of::<SijillSafha>(),
            }],
        )
    }
}

impl TarwisatKhatt {
    /// Checks the font array and the name pool against the section.
    ///
    /// # Errors
    ///
    /// [`KhataRuqaa::JadwalTalif`] naming the field that failed.
    pub fn tahaqquq(self, tul_qism: u64) -> Result<(), KhataRuqaa> {
        tahaqquq_mada(
            NawQism::Khatt,
            HAJM_TASDIR_U32,
            tul_qism,
            &[MadaMasfufa {
                haql_izaha: "izahat_khutut",
                haql_adad: "adad_khutut",
                izaha: self.izahat_khutut,
                adad: self.adad_khutut,
                khatwa: size_of::<SijillKhatt>(),
            }],
        )?;
        tahaqquq_hawd(
            NawQism::Khatt,
            HAJM_TASDIR_U32,
            tul_qism,
            "izahat_hawd",
            "tul_hawd",
            self.izahat_hawd,
            self.tul_hawd,
        )
    }
}
