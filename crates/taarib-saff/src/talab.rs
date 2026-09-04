//! الطلب — everything the caller hands to the engine.
//!
//! A layout request is text, a font chain, a size, the room available, the style
//! spans over the text, and a set of decisions the caller has already made about
//! how Arabic should behave. Those decisions are values here rather than
//! arguments spread across a call, because they are recorded verbatim into a
//! compiled patch: the digits a patch uses, whether it keeps diacritics, and how
//! it justifies are properties of that patch forever, not of whatever build of
//! Taarib happens to render it.

use crate::khatt::SilsilatKhutut;

/// A resolved direction. There is no third value: every run is one or the other
/// by the time anything is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum Ittijah {
    /// Right to left.
    Yameen,
    /// Left to right.
    Yasar,
}

impl Default for Ittijah {
    /// Right to left. A cache slot that has not been laid out yet is described
    /// in the direction this product exists to produce; Arabic is the source
    /// language here, not a translation target.
    fn default() -> Self {
        Self::Yameen
    }
}

impl Ittijah {
    /// The embedding level parity this direction corresponds to.
    #[must_use]
    pub const fn mustawa_fardi(self) -> bool {
        matches!(self, Self::Yameen)
    }

    /// The direction of an embedding level.
    #[must_use]
    pub const fn min_mustawa(mustawa: u8) -> Self {
        if mustawa.is_multiple_of(2) { Self::Yasar } else { Self::Yameen }
    }
}

/// How the paragraph's base direction is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum IttijahAsas {
    /// From the first strong character, per rule P2/P3 — which is what a game's
    /// own text should get, because a line that is entirely Latin inside an
    /// Arabic patch should still read left to right.
    #[default]
    Tilqai,
    /// Forced right to left.
    Yameen,
    /// Forced left to right.
    Yasar,
}

/// The language the text is in, which changes real shaping decisions.
///
/// This is not decoration. Persian and Urdu share the Arabic script but not its
/// letterforms: `kaf`, `yeh` and `heh` take different shapes, and the font
/// expresses that through `locl` lookups keyed on language. Shaping Persian as
/// Arabic produces text a Persian reader will call wrong, in a way that no
/// amount of font choice fixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum LughaNass {
    /// Detected from the text itself.
    #[default]
    Tilqai,
    /// Arabic.
    Arabi,
    /// Persian.
    Farisi,
    /// Urdu.
    Urdu,
    /// Latin-script text.
    Latini,
}

impl LughaNass {
    /// The OpenType language system tag, which is what `locl` is keyed on.
    #[must_use]
    pub const fn wasm_opentype(self) -> Option<&'static str> {
        match self {
            Self::Arabi => Some("ARA"),
            Self::Farisi => Some("FAR"),
            Self::Urdu => Some("URD"),
            Self::Latini => Some("ENG"),
            Self::Tilqai => None,
        }
    }

    /// The BCP 47 tag, used for segmentation tailoring and for the interface.
    #[must_use]
    pub const fn wasm(self) -> &'static str {
        match self {
            Self::Arabi | Self::Tilqai => "ar",
            Self::Farisi => "fa",
            Self::Urdu => "ur",
            Self::Latini => "en",
        }
    }

    /// Whether this language is written in the Arabic script.
    #[must_use]
    pub const fn arabiyat_alrasm(self) -> bool {
        matches!(self, Self::Arabi | Self::Farisi | Self::Urdu)
    }
}

/// How surplus width on a line is absorbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum NamatDabt {
    /// Leave the surplus at the end of the line.
    #[default]
    Bila,
    /// Stretch the spaces, which is the only mode Latin runs ever use.
    Masafat,
    /// Elongate letters at legitimate points found from shaped joining
    /// behaviour, and reshape.
    Kashida,
    /// Elongate first, then stretch spaces for whatever remains. The default for
    /// Arabic, and what hand-set Arabic actually does.
    KashidaThummaMasafat,
}

/// Where a line sits inside the width available to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum Muhadhaha {
    /// Against the leading edge — the right in Arabic, the left in Latin.
    #[default]
    Bidaya,
    /// Against the trailing edge.
    Nihaya,
    /// Centred.
    Wasat,
    /// Filled to the full width by the justification mode, except on the last
    /// line of a paragraph.
    Dabt,
}

/// What happens to diacritics.
///
/// Diacritics cost vertical room and, in dense interface text at small sizes,
/// legibility. They are also meaning: a game that vocalises its dialogue and
/// strips its menus is making the same choice a publisher makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum SiyasatTashkeel {
    /// Keep every mark the source carries.
    #[default]
    Ibqa,
    /// Remove marks before shaping.
    Hadhf,
    /// Keep marks in text the caller marked as dialogue, remove them elsewhere.
    IbqaFilHiwar,
}

/// Which digits the rendered text uses.
///
/// Independent of the interface setting: a patch decides this for the game it
/// patches, and a Persian patch and an Arabic patch of the same game will
/// legitimately differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum SiyasatArqam {
    /// Leave every digit exactly as the translation wrote it.
    #[default]
    KamaHiya,
    /// Map to European digits.
    Latini,
    /// Map to Arabic-Indic digits.
    Arabi,
    /// Map to Eastern Arabic-Indic digits.
    Farisi,
}

/// What to do when text does not fit and cannot be broken.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
#[derive(Default)]
pub enum SiyasatTajawuz {
    /// Lay it out anyway and report the overflow, so the compiler can put it in
    /// the overflow report and the reviewer can see it. The default, because a
    /// patch that silently shrinks text hides its own defects.
    #[default]
    Ballagh,
    /// Shrink the size until it fits, down to a floor.
    Taqlis {
        /// The smallest size allowed, in pixels.
        adna: f32,
    },
    /// Truncate and mark the cut with an ellipsis on the correct side.
    Ikhtisar,
}


/// An OpenType feature the caller wants on or off beyond the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct SifaIdafiya {
    /// The four-character feature tag, for example `ss01`.
    pub wasm: [u8; 4],
    /// The feature's value. Zero turns it off.
    pub qeema: u32,
}

/// An opaque run that takes part in layout but is not shaped.
///
/// A format placeholder and an inline sprite are the same thing to the layout
/// engine: a box of known size that occupies a position, participates in line
/// breaking and justification, and must survive into the output as a unit.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct Dharra {
    /// How wide it is at the request's size, in pixels.
    pub ard: f32,
    /// How tall it is, used for line height when it exceeds the font's own.
    pub irtifa: f32,
    /// How far above the baseline its bottom edge sits.
    pub asas: f32,
    /// The caller's own reference to it, carried through untouched.
    pub marja: u32,
}

/// What a style span does to the text under it.
///
/// Only the fields that change *layout* are read by the engine. Colour is
/// carried through to the output untouched, because the engine does not draw —
/// but it must survive the round trip, or a coloured word loses its colour the
/// moment the text is reordered.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct Uslub {
    /// Which font in the chain to prefer for this span.
    pub khatt: Option<u8>,
    /// A variable-font weight for this span.
    pub wazn: Option<u16>,
    /// Whether the span is italic or slanted.
    pub maail: bool,
    /// A size override, in pixels.
    pub hajm: Option<f32>,
    /// Colour, carried through untouched.
    pub lawn: Option<[u8; 4]>,
    /// Extra letter spacing for this span, in pixels.
    pub tabaud: Option<f32>,
    /// A vertical offset from the baseline, in pixels.
    pub izaha: Option<f32>,
    /// When present, the span is an opaque atom rather than text.
    pub dharra: Option<Dharra>,
}

impl Uslub {
    /// Whether this style forces a run boundary against another.
    ///
    /// Two adjacent spans can share a shaping run only when nothing that
    /// changes shaping differs between them. Colour does not: a red word inside
    /// a black sentence still joins to its neighbours, and splitting the run
    /// there would break the join.
    #[must_use]
    pub fn yaqta(&self, akhar: &Self) -> bool {
        self.khatt != akhar.khatt
            || self.wazn != akhar.wazn
            || self.maail != akhar.maail
            || self.hajm != akhar.hajm
            || self.tabaud != akhar.tabaud
            || self.dharra.is_some()
            || akhar.dharra.is_some()
    }
}

/// A style span over the clean text.
///
/// `bidaya` and `tul` are byte offsets into the clean text, and both must fall
/// on character boundaries. Spans of different kinds may nest freely; spans that
/// set the same property may not overlap, because there would be no answer to
/// which one applies.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct NitaqUslub {
    /// Identifies this span in the output, so a glyph can be traced back to the
    /// style it came from.
    pub id: u16,
    /// Byte offset of the span's first byte.
    pub bidaya: u32,
    /// Length in bytes.
    pub tul: u32,
    /// What it does.
    pub uslub: Uslub,
}

impl NitaqUslub {
    /// One past the last byte.
    #[must_use]
    pub const fn nihaya(&self) -> u32 {
        self.bidaya.saturating_add(self.tul)
    }

    /// Whether a byte offset falls inside this span.
    #[must_use]
    pub const fn yashmal(&self, mawqi: u32) -> bool {
        mawqi >= self.bidaya && mawqi < self.nihaya()
    }
}

/// The decisions a caller makes once and records into a patch.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct KhiyaratTakhtit {
    /// How the base direction is decided.
    pub ittijah: IttijahAsas,
    /// The language, which drives `locl` and segmentation tailoring.
    pub lugha: LughaNass,
    /// How surplus width is absorbed.
    pub dabt: NamatDabt,
    /// Where a line sits in the width available.
    pub muhadhaha: Muhadhaha,
    /// What happens to diacritics.
    pub tashkeel: SiyasatTashkeel,
    /// Which digits the output uses.
    pub arqam: SiyasatArqam,
    /// What happens when text does not fit.
    pub tajawuz: SiyasatTajawuz,
    /// Line height in pixels. When absent, the font's own metrics decide.
    pub irtifa_satr: Option<f32>,
    /// Extra spacing between every pair of glyphs, in pixels. Applied after
    /// shaping, so it never disturbs joining.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space, in pixels.
    pub tabaud_kalimat: f32,
    /// Whether this text is dialogue, which is what
    /// [`SiyasatTashkeel::IbqaFilHiwar`] keys on.
    pub hiwar: bool,
    /// Whether the caller forbids wrapping entirely, as a single-line field
    /// does.
    pub satr_wahid: bool,
    /// Feature overrides beyond the defaults for the script.
    pub sifat: Vec<SifaIdafiya>,
}

/// One layout request.
#[derive(Debug, Clone, Copy)]
pub struct TalabTakhtit<'a> {
    /// Logical-order text, already clean: markup and placeholders have been
    /// lifted into spans by [`crate::nasq`] before this point. Passing raw
    /// markup here would put a tag through the bidirectional algorithm, which
    /// is the single most common way a text stack corrupts mixed-direction
    /// text.
    pub nass: &'a str,
    /// The fonts to shape and draw with, tried in order per character.
    pub khutut: &'a SilsilatKhutut,
    /// The size in pixels.
    pub hajm: f32,
    /// The width available, in pixels. `None` means the text is laid out on one
    /// line of whatever width it turns out to need.
    pub ard_mutah: Option<f32>,
    /// The height available, in pixels, used to detect vertical overflow.
    pub irtifa_mutah: Option<f32>,
    /// Style spans over the clean text.
    pub nitaqat: &'a [NitaqUslub],
    /// The decisions.
    pub khiyarat: &'a KhiyaratTakhtit,
}
