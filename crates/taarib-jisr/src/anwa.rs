//! الأنواع — every type that crosses the boundary.
//!
//! These layouts are frozen. A field may be appended to a structure only behind
//! a major version bump, a field may never be reordered or resized, and no
//! structure here contains a Rust type, a `bool`, an enum, or anything else
//! whose representation the compiler is free to choose. Every structure that
//! carries data is `#[repr(C)]` and `Copy`, and every field is a fixed-width
//! integer, a `f32`, a raw pointer, or a `usize`.
//!
//! The four types behind the handles are the deliberate exception: they carry
//! no data, are never instantiated, and are pointed at but never dereferenced.
//! They are left without `#[repr(C)]` on purpose, because that is what makes
//! cbindgen emit them as opaque forward declarations. Marked `#[repr(C)]` they
//! would instead be defined — as a struct whose only member is a zero-length
//! array, which is a GNU extension, is ill-formed ISO C, and is rejected
//! outright by MSVC. The header has to compile for the C++ shims and for any
//! third party binding directly, so the opaque form is the correct one.
//!
//! Fields are ordered widest-first so that no structure carries implicit
//! padding. Padding is not merely wasteful across an ABI — it is uninitialised
//! bytes that a caller in another language may or may not zero, and a structure
//! whose padding differs between two compilers is a structure two sides disagree
//! about.
//!
//! Enumerations cross as `u32` rather than as C enums, because a C enum's width
//! is implementation-defined and a value outside the declared set is undefined
//! behaviour in C++ — and an adapter compiled against a newer header will
//! eventually send exactly such a value. Every decoder here treats an
//! unrecognised number as the documented default instead.

use taarib_saff::talab::{
    IttijahAsas, LughaNass, Muhadhaha, NamatDabt, SiyasatArqam, SiyasatTajawuz, SiyasatTashkeel,
};

// ---------------------------------------------------------------------------
// Handles
// ---------------------------------------------------------------------------

/// An engine context: the caches, the buffers, and the capture channel.
///
/// Opaque and never dereferenced. The value is a generation-tagged index into
/// this library's own table, carried in a pointer-shaped type so that C callers
/// get type safety between handle kinds and so that a stale handle is detected
/// by lookup rather than by touching freed memory. Vulkan's non-dispatchable
/// handles work the same way and for the same reason.
pub type TaaribSiyaq = *mut TaaribSiyaqKhas;

/// The never-instantiated type behind [`TaaribSiyaq`].
#[expect(
    clippy::trailing_empty_array,
    reason = "the absent `repr` is what makes cbindgen emit an opaque forward declaration; with \
              one the header would define a zero-length array member, which MSVC rejects"
)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribSiyaqKhas {
    _khas: [u8; 0],
}

/// A loaded, validated font.
pub type TaaribKhatt = *mut TaaribKhattKhas;

/// The never-instantiated type behind [`TaaribKhatt`].
#[expect(
    clippy::trailing_empty_array,
    reason = "the absent `repr` is what makes cbindgen emit an opaque forward declaration; with \
              one the header would define a zero-length array member, which MSVC rejects"
)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribKhattKhas {
    _khas: [u8; 0],
}

/// An ordered chain of fonts, tried in order per character.
pub type TaaribSilsila = *mut TaaribSilsilaKhas;

/// The never-instantiated type behind [`TaaribSilsila`].
#[expect(
    clippy::trailing_empty_array,
    reason = "the absent `repr` is what makes cbindgen emit an opaque forward declaration; with \
              one the header would define a zero-length array member, which MSVC rejects"
)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribSilsilaKhas {
    _khas: [u8; 0],
}

/// A glyph atlas that grows and evicts at runtime.
pub type TaaribLawha = *mut TaaribLawhaKhas;

/// The never-instantiated type behind [`TaaribLawha`].
#[expect(
    clippy::trailing_empty_array,
    reason = "the absent `repr` is what makes cbindgen emit an opaque forward declaration; with \
              one the header would define a zero-length array member, which MSVC rejects"
)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribLawhaKhas {
    _khas: [u8; 0],
}

// ---------------------------------------------------------------------------
// Flags
// ---------------------------------------------------------------------------

/// [`TaaribHarf::alam`]: this glyph is a combining mark, positioned onto a base
/// rather than advancing the pen.
pub const TAARIB_HARF_ALAMA: u8 = 1 << 0;

/// [`TaaribSatr::alam`]: this is the last line of its paragraph, which is what
/// stops justification stretching it across the full width.
pub const TAARIB_SATR_AKHIR: u32 = 1 << 0;

/// [`TaaribSatr::alam`]: this line's base direction is right to left.
pub const TAARIB_SATR_YAMEEN: u32 = 1 << 1;

/// [`TaaribKhiyarat::alam`]: this text is dialogue, which is what the
/// keep-diacritics-in-dialogue policy keys on.
pub const TAARIB_KHIYAR_HIWAR: u32 = 1 << 0;

/// [`TaaribKhiyarat::alam`]: the caller forbids wrapping entirely, as a
/// single-line input field does.
pub const TAARIB_KHIYAR_SATR_WAHID: u32 = 1 << 1;

/// [`TaaribMakhzanTakhtit::alam`]: the layout's overall direction is right to
/// left.
pub const TAARIB_TAKHTIT_YAMEEN: u32 = 1 << 0;

/// [`TaaribMakhzanTakhtit::alam`]: the overflow policy truncated the text.
pub const TAARIB_TAKHTIT_MAQSUS: u32 = 1 << 1;

/// [`TaaribMakhzanTakhtit::alam`]: the text did not fit, and
/// [`TaaribMakhzanTakhtit::tajawuz`] describes by how much.
pub const TAARIB_TAKHTIT_TAJAWUZ: u32 = 1 << 2;

/// [`TaaribMakhzanTakhtit::alam`]: this layout came from the cache, so nothing
/// was shaped.
///
/// Exposed because an adapter's own diagnostics want the hit rate, and because a
/// cache that never hits is a cache whose key is wrong.
pub const TAARIB_TAKHTIT_MAKHZAN: u32 = 1 << 3;

/// [`TaaribNitaqUslub::alam`]: this span is italic or slanted.
pub const TAARIB_USLUB_MAAIL: u32 = 1 << 0;

/// [`TaaribNitaqUslub::alam`]: this span is an opaque atom — a format
/// placeholder or an inline sprite — and is never shaped.
pub const TAARIB_USLUB_DHARRA: u32 = 1 << 1;

/// [`TaaribNitaqUslub::alam`]: this span sets a font index.
pub const TAARIB_USLUB_KHATT: u32 = 1 << 2;

/// [`TaaribNitaqUslub::alam`]: this span sets a weight.
pub const TAARIB_USLUB_WAZN: u32 = 1 << 3;

/// [`TaaribNitaqUslub::alam`]: this span sets a size.
pub const TAARIB_USLUB_HAJM: u32 = 1 << 4;

/// [`TaaribNitaqUslub::alam`]: this span sets a colour.
pub const TAARIB_USLUB_LAWN: u32 = 1 << 5;

// ---------------------------------------------------------------------------
// Layout output
// ---------------------------------------------------------------------------

/// One positioned glyph, ready to become two triangles.
///
/// Twenty-four bytes, four-byte aligned, no padding. This is the structure a
/// game process walks thousands of times per frame, so its size is a decision
/// rather than an accident.
///
/// Note what is absent: a codepoint. Nothing downstream of shaping is given the
/// character a glyph came from, because a field that carried it would eventually
/// be drawn from — which is the presentation-form pipeline this product exists
/// to make impossible.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaaribHarf {
    /// The glyph identifier, in the font named by [`TaaribHarf::khatt`].
    pub muarrif: u32,
    /// Byte offset into the caller's text of the cluster this glyph belongs to.
    pub anqud: u32,
    /// Horizontal position of the glyph's origin, in pixels from the layout's
    /// left edge. Already in visual order.
    pub s: f32,
    /// Vertical position of the origin, in pixels down from the layout's top.
    pub a: f32,
    /// The advance this glyph contributed. Zero for marks.
    pub taqaddum: f32,
    /// The style span this glyph inherited, so colour survives reordering.
    pub nitaq: u16,
    /// Index into the font chain of the font this glyph belongs to.
    pub khatt: u8,
    /// [`TAARIB_HARF_ALAMA`].
    pub alam: u8,
}

/// One laid-out line.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaaribSatr {
    /// Index of this line's first glyph in the glyph buffer.
    pub awwal_harf: u32,
    /// How many glyphs this line has.
    pub adad_huruf: u32,
    /// First byte of the logical text this line covers.
    pub bidayat_mantiqi: u32,
    /// One past the last byte.
    pub nihayat_mantiqi: u32,
    /// The baseline's vertical position, in pixels from the layout's top.
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
    /// [`TAARIB_SATR_AKHIR`], [`TAARIB_SATR_YAMEEN`].
    pub alam: u32,
}

/// What overflowed, by how much.
///
/// Not an error: a measurement the caller asked for. The patch compiler turns it
/// into the overflow report, the review console sorts by it, and the workspace
/// shows it beside the string as it is typed.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TaaribTaqreerTajawuz {
    /// The widest line's measured width, in pixels.
    pub ard: f32,
    /// The width that was available.
    pub ard_mutah: f32,
    /// The total height the text needed.
    pub irtifa: f32,
    /// The height that was available, or zero when none was given.
    pub irtifa_mutah: f32,
    /// The first line that exceeded the width.
    pub awwal_satr: u32,
    /// How many lines exceeded it.
    pub adad_sutur: u32,
}

/// The caller-owned buffer the layout is written into.
///
/// The caller owns both arrays and their lifetime; this library never allocates
/// them, never frees them, and never keeps a pointer to them past the call. The
/// `siaat_*` fields are capacity in elements and are read; the `adad_*` fields
/// are how many were written and are written.
///
/// When either array is too small, nothing is written, `adad_*` receives the
/// required count for both arrays, and the call returns
/// [`TAARIB_SIAT_QASIRA`](crate::khata_c::TAARIB_SIAT_QASIRA). A caller grows
/// once and retries, and after the first few frames the buffer never grows
/// again — which is what makes the hot path allocation-free in practice and not
/// merely in principle.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribMakhzanTakhtit {
    /// The caller's glyph array.
    pub huruf: *mut TaaribHarf,
    /// Its capacity, in elements.
    pub siaat_huruf: usize,
    /// How many glyphs were written, or how many are required.
    pub adad_huruf: usize,
    /// The caller's line array.
    pub sutur: *mut TaaribSatr,
    /// Its capacity, in elements.
    pub siaat_sutur: usize,
    /// How many lines were written, or how many are required.
    pub adad_sutur: usize,
    /// The width of the widest line.
    pub ard: f32,
    /// The total height of every line box.
    pub irtifa: f32,
    /// The size the text was finally laid out at, which differs from the size
    /// requested when the overflow policy shrank it to fit.
    pub hajm: f32,
    /// [`TAARIB_TAKHTIT_YAMEEN`], [`TAARIB_TAKHTIT_MAQSUS`],
    /// [`TAARIB_TAKHTIT_TAJAWUZ`], [`TAARIB_TAKHTIT_MAKHZAN`].
    pub alam: u32,
    /// Valid only when [`TAARIB_TAKHTIT_TAJAWUZ`] is set.
    pub tajawuz: TaaribTaqreerTajawuz,
}

/// Text measured without positioning a single glyph.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TaaribQiyasNass {
    /// The width of the widest line.
    pub ard: f32,
    /// The total height.
    pub irtifa: f32,
    /// The first line's ascent.
    pub suud: f32,
    /// The last line's descent.
    pub hubut: f32,
    /// How many lines the text needed.
    pub adad_sutur: u32,
    /// Padding to a multiple of eight. Always written as zero; ignore it.
    pub hashw: u32,
}

// ---------------------------------------------------------------------------
// Layout input
// ---------------------------------------------------------------------------

/// A style span over the caller's text.
///
/// Only what changes layout is read. Colour is carried through untouched,
/// because the engine does not draw — but it must survive reordering, or a
/// coloured word inside a sentence loses its colour the moment the line is put
/// into visual order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaaribNitaqUslub {
    /// Byte offset of the span's first byte in the caller's text.
    pub bidaya: u32,
    /// Length in bytes.
    pub tul: u32,
    /// Colour as `0xRRGGBBAA`, read only when [`TAARIB_USLUB_LAWN`] is set.
    pub lawn: u32,
    /// [`TAARIB_USLUB_MAAIL`] and the rest of the `TAARIB_USLUB_*` flags.
    pub alam: u32,
    /// A size override in pixels, read only when [`TAARIB_USLUB_HAJM`] is set.
    pub hajm: f32,
    /// Extra letter spacing for this span, in pixels.
    pub tabaud: f32,
    /// A vertical offset from the baseline, in pixels.
    pub izaha: f32,
    /// An atom's width in pixels, read only when [`TAARIB_USLUB_DHARRA`] is set.
    pub ard_dharra: f32,
    /// An atom's height in pixels.
    pub irtifa_dharra: f32,
    /// How far above the baseline an atom's bottom edge sits.
    pub asas_dharra: f32,
    /// The caller's own reference to an atom, carried through untouched.
    pub marja_dharra: u32,
    /// A variable-font weight, read only when [`TAARIB_USLUB_WAZN`] is set.
    pub wazn: u16,
    /// Identifies this span in the output.
    pub id: u16,
    /// The font index this span prefers, read only when [`TAARIB_USLUB_KHATT`]
    /// is set.
    pub khatt: u8,
    /// Padding to a multiple of four. Always written as zero; ignore it.
    pub hashw: [u8; 3],
}

/// An OpenType feature the caller wants on or off beyond the defaults.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaaribSifa {
    /// The four-character tag, for example `ss01`, in writing order.
    pub wasm: [u8; 4],
    /// The value. Zero turns the feature off.
    pub qeema: u32,
}

/// The decisions a caller makes once and records into a patch.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribKhiyarat {
    /// Extra feature settings.
    pub sifat: *const TaaribSifa,
    /// How many.
    pub adad_sifat: usize,
    /// How the base direction is decided: 0 automatic, 1 right to left,
    /// 2 left to right.
    pub ittijah: u32,
    /// The language: 0 detect, 1 Arabic, 2 Persian, 3 Urdu, 4 Latin.
    pub lugha: u32,
    /// How surplus width is absorbed: 0 none, 1 spaces, 2 kashida,
    /// 3 kashida then spaces.
    pub dabt: u32,
    /// Where a line sits: 0 leading edge, 1 trailing edge, 2 centre, 3 filled.
    pub muhadhaha: u32,
    /// Diacritics: 0 keep, 1 strip, 2 keep in dialogue only.
    pub tashkeel: u32,
    /// Digits: 0 leave, 1 European, 2 Arabic-Indic, 3 Eastern Arabic-Indic.
    pub arqam: u32,
    /// Overflow: 0 report, 1 shrink to fit, 2 truncate.
    pub tajawuz: u32,
    /// The smallest size shrink-to-fit may use, in pixels.
    pub hajm_adna: f32,
    /// Line height in pixels; zero or less means the font's own metrics decide.
    pub irtifa_satr: f32,
    /// Extra spacing between every pair of glyphs, applied after shaping so it
    /// never disturbs joining.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space.
    pub tabaud_kalimat: f32,
    /// [`TAARIB_KHIYAR_HIWAR`], [`TAARIB_KHIYAR_SATR_WAHID`].
    pub alam: u32,
}

/// One layout request.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribTalab {
    /// UTF-8 text in logical order, with markup and placeholders already lifted
    /// into spans. Not required to be NUL-terminated.
    pub nass: *const u8,
    /// Its length in bytes.
    pub tul_nass: usize,
    /// The style spans.
    pub nitaqat: *const TaaribNitaqUslub,
    /// How many.
    pub adad_nitaqat: usize,
    /// The fonts to shape and draw with.
    pub silsila: TaaribSilsila,
    /// The size in pixels.
    pub hajm: f32,
    /// The width available in pixels; zero or less means one line of whatever
    /// width the text needs.
    pub ard_mutah: f32,
    /// The height available in pixels; zero or less means unbounded.
    pub irtifa_mutah: f32,
    /// The decisions.
    pub khiyarat: TaaribKhiyarat,
}

/// How a context is built.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaaribKhiyaratSiyaq {
    /// The layout cache's budget in bytes. Zero disables the cache entirely,
    /// which is what an offline compiler wants and what a game never does.
    pub mizaniyat_makhzan: usize,
    /// How many glyph buffers to keep pooled for the allocation-free path.
    pub adad_makhazin: u32,
    /// Reserved, must be zero.
    pub hashw: u32,
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// A font's metrics, scaled to a pixel size.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TaaribQiyasatKhatt {
    /// How far the font rises above the baseline.
    pub suud: f32,
    /// How far it falls below, as a positive number.
    pub hubut: f32,
    /// The gap the font asks for between lines.
    pub fajwa: f32,
    /// The line height the font recommends.
    pub irtifa_satr: f32,
    /// Cap height.
    pub uluw_kabital: f32,
    /// x-height.
    pub uluw_saghir: f32,
    /// Units per em, as declared by the font.
    pub wahdat: u32,
    /// Padding to a multiple of eight. Always written as zero; ignore it.
    pub hashw: u32,
}

// ---------------------------------------------------------------------------
// Atlas
// ---------------------------------------------------------------------------

/// Everything that makes one rasterized image of a glyph distinct.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaaribMiftahShakl {
    /// The glyph identifier.
    pub muarrif: u32,
    /// The pixel size in quarter-pixels.
    pub hajm_rubi: u16,
    /// Index into the font chain.
    pub khatt: u8,
    /// The subpixel bucket.
    pub bakat: u8,
}

/// Where a glyph lives in the atlas, and how to draw it.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TaaribMawdiShakl {
    /// The glyph's advance at this size.
    pub taqaddum: f32,
    /// Left edge in the page, in pixels.
    pub s: u16,
    /// Top edge in the page, in pixels.
    pub a: u16,
    /// Width in pixels.
    pub ard: u16,
    /// Height in pixels.
    pub irtifa: u16,
    /// Left bearing: how far right of the pen the image starts.
    pub izaha_s: i16,
    /// Top bearing: how far above the baseline the image's top edge sits.
    pub izaha_a: i16,
    /// Which page.
    pub safha: u16,
    /// Padding to a multiple of four. Always written as zero; ignore it.
    pub hashw: u16,
}

/// One texture page, borrowed.
///
/// `bayt` points into the atlas and stays valid until the next call that can
/// change the atlas — any call to the glyph lookup, or destroying the atlas.
/// The caller uploads it and does not keep it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaaribSafha {
    /// The texels, one byte each, row-major from the top, no row padding.
    pub bayt: *const u8,
    /// How many bytes, which is width times height.
    pub tul: usize,
    /// Width in texels.
    pub ard: u16,
    /// Height in texels.
    pub irtifa: u16,
    /// 0 for coverage, 1 for a signed distance field.
    pub namat: u32,
}

/// What the atlas has been doing.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaaribIhsaatLawha {
    /// Glyphs served without rasterizing anything.
    pub isabat: u64,
    /// Glyphs that had to be rasterized and packed. A number that keeps climbing
    /// after the first minutes of play means the patch compiler missed strings.
    pub ikhfaqat: u64,
    /// Rectangles reclaimed to make room.
    pub ikhlaat: u64,
    /// Pages opened after the first.
    pub ahdath_namu: u64,
    /// What the pages currently cost, in bytes.
    pub bayt: u64,
    /// The byte budget.
    pub mizaniya: u64,
    /// How many glyphs are mapped.
    pub ashkal: u32,
    /// How many pages are open.
    pub safahat: u32,
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

/// Reads the direction discriminant, defaulting to automatic.
#[must_use]
pub const fn ittijah_min_raqm(raqm: u32) -> IttijahAsas {
    match raqm {
        1 => IttijahAsas::Yameen,
        2 => IttijahAsas::Yasar,
        _ => IttijahAsas::Tilqai,
    }
}

/// Reads the language discriminant, defaulting to detection.
#[must_use]
pub const fn lugha_min_raqm(raqm: u32) -> LughaNass {
    match raqm {
        1 => LughaNass::Arabi,
        2 => LughaNass::Farisi,
        3 => LughaNass::Urdu,
        4 => LughaNass::Latini,
        _ => LughaNass::Tilqai,
    }
}

/// Reads the justification discriminant, defaulting to none.
#[must_use]
pub const fn dabt_min_raqm(raqm: u32) -> NamatDabt {
    match raqm {
        1 => NamatDabt::Masafat,
        2 => NamatDabt::Kashida,
        3 => NamatDabt::KashidaThummaMasafat,
        _ => NamatDabt::Bila,
    }
}

/// Reads the alignment discriminant, defaulting to the leading edge.
#[must_use]
pub const fn muhadhaha_min_raqm(raqm: u32) -> Muhadhaha {
    match raqm {
        1 => Muhadhaha::Nihaya,
        2 => Muhadhaha::Wasat,
        3 => Muhadhaha::Dabt,
        _ => Muhadhaha::Bidaya,
    }
}

/// Reads the diacritic discriminant, defaulting to keeping them.
#[must_use]
pub const fn tashkeel_min_raqm(raqm: u32) -> SiyasatTashkeel {
    match raqm {
        1 => SiyasatTashkeel::Hadhf,
        2 => SiyasatTashkeel::IbqaFilHiwar,
        _ => SiyasatTashkeel::Ibqa,
    }
}

/// Reads the digit discriminant, defaulting to leaving digits alone.
#[must_use]
pub const fn arqam_min_raqm(raqm: u32) -> SiyasatArqam {
    match raqm {
        1 => SiyasatArqam::Latini,
        2 => SiyasatArqam::Arabi,
        3 => SiyasatArqam::Farisi,
        _ => SiyasatArqam::KamaHiya,
    }
}

/// Reads the overflow discriminant, defaulting to reporting.
///
/// Reporting is the default on purpose: a caller that sends an unrecognised
/// number gets a layout plus an honest measurement of how far past its bounds it
/// went, rather than silently shrunk or truncated text.
#[must_use]
pub fn tajawuz_min_raqm(raqm: u32, hajm_adna: f32) -> SiyasatTajawuz {
    match raqm {
        1 => SiyasatTajawuz::Taqlis {
            adna: if hajm_adna > 0.0 { hajm_adna } else { 8.0 },
        },
        2 => SiyasatTajawuz::Ikhtisar,
        _ => SiyasatTajawuz::Ballagh,
    }
}
