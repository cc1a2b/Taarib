//! النسق — markup and format placeholders, lifted out of the text before
//! anything else in the pipeline is allowed to look at it.
//!
//! A game does not store the string it draws. It stores
//! `<color=#ff0000>{0}</color> أصاب <b>%s</b>` and expects its own renderer to
//! turn that into a red number, a name in bold, and Arabic in between. This
//! module turns that raw string into three things: **clean text** with no markup
//! left in it at all, a **span table** over the clean text saying what applies
//! where, and an **atom table** naming every format placeholder exactly as it
//! appeared.
//!
//! ## Why this runs first, before `ittijah`
//!
//! Put `<color=#ff0000>` through the Unicode Bidirectional Algorithm and the
//! algorithm does exactly what it is defined to do: `<` and `>` are Other
//! Neutrals, `=` is a Common Separator, `#` is a European Terminator, and `f`
//! and `0` are a Left-to-Right letter and a European Number. The tag is not a tag
//! to the algorithm — it is nine characters of mixed directional class sitting
//! in the middle of a right-to-left paragraph. Rule L2 then reverses the runs
//! it lands in, and the tag comes out of the other side as `>0000ff#=roloc<`,
//! or split across two positions, or with its digits reordered relative to its
//! `#`. The string the game gets back is no longer valid markup, and every
//! mixed-direction string it touches is corrupted in a way that looks, from the
//! outside, like the Arabic itself is broken.
//!
//! The same argument applies to shaping, to line breaking, and to measurement:
//! a tag has no letterforms, occupies no width, and offers no break
//! opportunity. Markup never enters directional processing. It is removed here,
//! once, and restored by [`aid_binaa`] after the text has been translated.
//!
//! ## Why an atom is exactly one U+FFFC
//!
//! A format placeholder is not text and it is not nothing. `{0}` will become
//! `147` or `3` at runtime — a width the layout cannot know, a direction it
//! must not guess, and a unit that must never be broken across a line. So every
//! atom is represented in the clean text by exactly one U+FFFC OBJECT
//! REPLACEMENT CHARACTER, for three reasons, all of them load-bearing:
//!
//! - **One position.** The bidirectional algorithm and the line breaker both
//!   reason over positions. An atom that spanned several characters could be
//!   reordered internally or broken through the middle; an atom that spanned
//!   none would have no place in the reordered sequence to come back to.
//! - **A neutral directional class.** U+FFFC is `Other Neutral`, so an atom
//!   takes the direction of whatever surrounds it rather than imposing one.
//!   `{0}` between two Arabic words stays inside the Arabic run; the same `{0}`
//!   between two Latin words stays inside the Latin run. Substituting a digit
//!   or a letter here would inject a strong or numeric type and shift the
//!   surrounding text.
//! - **Never mistaken for a letter.** U+FFFC has no glyph in any real font, is
//!   not a letter in any script, and carries no joining behaviour, so it cannot
//!   accidentally join to an Arabic neighbour, cannot absorb a kashida, and
//!   cannot be shaped into something visible if a later stage forgets to skip
//!   it.
//!
//! ## The round trip is the point
//!
//! [`aid_binaa`] reconstructs the original string **byte for byte** from the
//! clean text plus the extraction record, for every input [`istakhrij`]
//! accepts. That property is what the translation pipeline is built on: a
//! translator, and a machine translation provider, see only clean text, so
//! neither can corrupt a tag it never saw; and after translation the markup is
//! put back around the new text mechanically. Without an exact round trip,
//! restoring markup would be a guess, and a guess that is wrong once in a
//! thousand strings is a patch that ships with a visible `<color=#ff0000>` in
//! it.
//!
//! ## Leniency
//!
//! Every engine here ships a lenient parser, and this module matches each one
//! rather than being stricter than the thing it is imitating. Unity leaves an
//! unknown tag in the text as literal characters; so does this. Godot prints an
//! unrecognized `[tag]` verbatim; so does this. A patch that silently ate a tag
//! the game meant to *display* would be worse than one that showed a tag the
//! game meant to interpret, because the first is invisible until a player
//! reports it. Where a dialect is lenient, [`istakhrij`] is lenient and returns
//! `Ok`. Where the caller needs the stricter reading — the extraction and
//! review path in particular, which exists to find defects rather than to
//! survive them — [`KhiyaratNasq::sarim`] turns each leniency into a named
//! [`SababNasq`] with the byte offset that caused it.

use core::ops::Range;

use smallvec::SmallVec;
use taarib_usus::khata::Natija;

use crate::khata::{KhataSaff, SababNasq};
use crate::talab::{Dharra, NitaqUslub, Uslub};

/// The character an atom occupies in the clean text: U+FFFC OBJECT REPLACEMENT
/// CHARACTER, one position, directionally neutral, absent from every font.
pub const BADEEL_DHARRA: char = '\u{FFFC}';

/// The same character as a string, which is what the clean text is built from.
const NASS_BADEEL: &str = "\u{FFFC}";

/// How far past a `<` or `[` the scanner will look for the closing bracket
/// before deciding the character was punctuation rather than the start of a
/// tag. TextMeshPro imposes a comparable limit on its own tag buffer; without
/// one, a single stray `<` in a long paragraph costs a scan of the whole
/// paragraph.
const HADD_WASM: usize = 256;

/// Default limit on how many tags may be open at once.
///
/// Not quite a nesting limit, because in these dialects a tag need not be
/// closed at all: `<color=a>x<color=b>y` leaves two tags open with nothing
/// nested, and TextMeshPro's own style stacks behave the same way. Sixty-four
/// is far above anything hand-written or engine-generated and far below the
/// point where a hostile string could make this scanner do interesting amounts
/// of work.
const UMQ_IFTIRADI: u8 = 64;

/// How deeply braces may nest inside a placeholder's format field.
///
/// Unity Localization's Smart Strings put brace-delimited arguments inside the
/// format field of a placeholder: `{players:list:{}|,   |   and   }` is a list
/// formatter whose item template and separators are themselves braced. That is
/// one level. A list nested inside another list's item template is two, and
/// nothing in Unity's own documentation, or in either game read for this
/// module, goes past that — so four is double the deepest construct anyone
/// writes.
///
/// The limit is not there to bound work: [`HADD_WASM`] already stops the scan
/// after two hundred and fifty-six bytes whatever the braces do. It is there
/// because depth is what separates a format specifier from prose. Text that
/// opens brace after brace is not a formatter's input, and reading it as one
/// would swallow a long run of real words into a single opaque atom, hiding
/// them from the translator with no diagnostic anywhere.
const UMQ_TANSIQ: u8 = 4;

/// What an atom stands for.
///
/// The four kinds are the four things a game substitutes into a string at
/// runtime, and they are treated differently downstream: a `Mawdi` must survive
/// translation in the same relative order, a `Sura` needs a measured width from
/// the engine's own sprite metrics, a `Mutaghayyir` may expand to text of any
/// direction, and an `Amr` is never drawn — though a few of them, `<space=8>`
/// and a tab among them, still occupy width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum NawDharra {
    /// A format placeholder: `{0}`, `{name}`, `%s`, `%1$d`.
    Mawdi,
    /// An inline image: `<sprite=3>`, `[img]icon.png[/img]`, `\I[12]`.
    Sura,
    /// A runtime variable: `$gold`, `${player}`, `\V[7]`, Ren'Py's `[points]`.
    Mutaghayyir,
    /// A command the engine acts on rather than draws: `\n`, `\C[2]`, `<br>`,
    /// GameMaker's `#`.
    Amr,
}

impl NawDharra {
    /// Whether this kind of atom puts marks on the screen.
    ///
    /// A command does not. It may still take up room — a tab and `<space=8>`
    /// both do — but nothing is ever drawn in that room, which is what a
    /// renderer walking the atom table needs to know before it asks the engine
    /// for an image it will not get.
    #[must_use]
    pub const fn marii(self) -> bool {
        !matches!(self, Self::Amr)
    }
}

/// One markup dialect the scanner is allowed to recognize.
///
/// Dialects are opt-in per string because they genuinely conflict. GameMaker
/// reads a bare `#` as a line break, which would silently cut `Level #3` in
/// half for every other engine. Ren'Py reads `[gold]` as a variable, which is
/// `BBCode`'s opening bold tag. Enabling only what the game actually parses is
/// the difference between extracting its markup and inventing some.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum LahjatNasq {
    /// Unity rich text, as TextMeshPro and the legacy UI parse it.
    Unity,
    /// Unreal's `<Style>text</>` rich text.
    Unreal,
    /// Unreal's `%0`-style glyph slots: a percent sign and a slot number the
    /// game replaces with a button icon or a key name.
    ///
    /// Separate from [`Self::Unreal`], and excluded from the default set, for
    /// the same reason [`Self::GameMaker`] is: the syntax collides with
    /// ordinary text. Turkish, Persian and several other locales write a
    /// percentage with the sign in front — `%50` is fifty percent — so reading
    /// `%` plus digits as a placeholder everywhere would lift half the numbers
    /// out of those languages' strings and then refuse every translation that
    /// did not carry the invented atom back.
    ///
    /// C-style conversions are unaffected: `%0d`, `%1$s` and `%.3f` are tried
    /// as printf first and only what printf refuses is offered to this rule.
    UnrealRumuz,
    /// `BBCode`, as Godot's `RichTextLabel` and Ren'Py's `BBCode` mode parse it.
    BbCode,
    /// Ren'Py's own `{tag}` text tags and `[variable]` substitutions.
    RenPy,
    /// RPG Maker escape codes: `\V[n]`, `\C[n]`, `\I[n]`, `\G` and the rest.
    RpgMaker,
    /// GameMaker's `#` line break, and `\#` as the escape for a literal hash.
    GameMaker,
    /// Web-style substitution: `{{name}}` and `${name}`.
    Web,
}

/// What the scanner is allowed to recognize, and how it behaves at the edges.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct KhiyaratNasq {
    /// Which dialects to parse. An empty list still extracts nothing but the
    /// dialect-independent escapes, which is the correct behaviour for a plain
    /// string table.
    pub lahjat: Vec<LahjatNasq>,
    /// The width an atom occupies when the caller has no better number, in
    /// pixels at the request's own size.
    ///
    /// Zero means "unknown", and is the honest default: `nasq` cannot measure a
    /// sprite it has never seen or a variable that has not been substituted
    /// yet. A caller that knows — an adapter holding the engine's own sprite
    /// metrics, or a compiler that has already substituted representative
    /// values — passes the real width, and the layout is right. Commands are
    /// always zero regardless, because they draw nothing.
    pub ard_dharra_iftiradi: f32,
    /// How many tags may be open at once before the text is refused. Zero
    /// selects the default of sixty-four.
    pub aqsa_umq: u8,
    /// The size, in pixels, that a relative size tag is relative to.
    ///
    /// `<size=+2>` and `<size=120%>` cannot be resolved without it, and
    /// [`Uslub::hajm`] is absolute pixels by definition. When this is `None`
    /// the span is still produced and still round-trips, but carries no size
    /// override, because inventing one would be worse than declining to.
    pub hajm_asas: Option<f32>,
    /// Whether to report what the engines themselves tolerate.
    ///
    /// `false` — the default — matches each dialect's own parser exactly: an
    /// unclosed tag runs to the end of the string, a stray closing tag is
    /// dropped, an unreadable attribute value leaves the whole tag as literal
    /// text, and none of it is an error, because none of it is an error to the
    /// game either. `true` turns each of those into
    /// [`KhataSaff::NasqTalif`] with the offset that caused it, which is what
    /// the extraction and review path wants: there, malformed markup in a
    /// source string is a defect worth naming rather than a condition to
    /// survive.
    pub sarim: bool,
}

impl Default for KhiyaratNasq {
    /// Every dialect except the two whose syntax collides with ordinary text:
    /// GameMaker, where a bare `#` is punctuation everywhere else, and
    /// [`LahjatNasq::UnrealRumuz`], where `%50` is a percentage in several
    /// languages. Both are opt-in per the reasons given on [`LahjatNasq`];
    /// enabling either by default would damage far more strings than it would
    /// help.
    fn default() -> Self {
        Self {
            lahjat: vec![
                LahjatNasq::Unity,
                LahjatNasq::Unreal,
                LahjatNasq::BbCode,
                LahjatNasq::RenPy,
                LahjatNasq::RpgMaker,
                LahjatNasq::Web,
            ],
            ard_dharra_iftiradi: 0.0,
            aqsa_umq: UMQ_IFTIRADI,
            hajm_asas: None,
            sarim: false,
        }
    }
}

impl KhiyaratNasq {
    /// Options that recognize exactly one dialect.
    #[must_use]
    pub fn wahida(lahja: LahjatNasq) -> Self {
        Self {
            lahjat: vec![lahja],
            ..Self::default()
        }
    }

    /// Options that recognize nothing but the dialect-independent escapes.
    #[must_use]
    pub fn bila_lahja() -> Self {
        Self {
            lahjat: Vec::new(),
            ..Self::default()
        }
    }

    /// Whether a dialect is enabled.
    #[must_use]
    pub fn tashmal(&self, lahja: LahjatNasq) -> bool {
        self.lahjat.contains(&lahja)
    }

    /// The nesting limit in force.
    #[must_use]
    pub const fn umq(&self) -> u8 {
        if self.aqsa_umq == 0 {
            UMQ_IFTIRADI
        } else {
            self.aqsa_umq
        }
    }
}

/// One atom, as it appeared and as the layout will treat it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct DharraMustakhraja {
    /// The span that carries it. That span covers exactly the one
    /// [`BADEEL_DHARRA`] this atom became, and holds the [`Dharra`] the layout
    /// reads its width from.
    pub nitaq: u16,
    /// The placeholder exactly as it appeared in the raw text, including its
    /// delimiters. This is the string a translation is checked against: an
    /// atom that is missing, duplicated, or altered is a rejected translation.
    pub khaam: String,
    /// What it stands for.
    pub naw: NawDharra,
    /// The positional index the dialect wrote, when it wrote one.
    ///
    /// Kept exactly as written and never normalized between dialects: `{0}` is
    /// zero-based and `%1$s` is one-based, and rewriting either to match the
    /// other would make a reordering check compare two different numbering
    /// schemes and report a defect that is not there. `%s` and `{name}` have no
    /// written index and get `None`.
    pub tarteeb: Option<u32>,
}

/// One piece of raw text that was removed, and where it was removed from.
///
/// This is the whole of the reconstruction record. `mawqi` is a byte offset
/// into the clean text, `khaam` is the exact bytes that were taken out, and
/// `badeel` is how many bytes of clean text now stand in their place — zero for
/// a tag that left nothing behind, three for an atom that became a
/// [`BADEEL_DHARRA`], one for an escape like `{{` that became a literal `{`.
/// Those three numbers are sufficient to put the original back byte for byte,
/// and nothing else in the record is needed to do it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct AtharNasq {
    /// Byte offset into the clean text where this text was removed from.
    pub mawqi: u32,
    /// The exact bytes that were removed.
    pub khaam: String,
    /// How many bytes of clean text stand in their place.
    pub badeel: u32,
}

/// A rule a span draws over the text it covers.
///
/// Kept beside the span table rather than inside [`Uslub`] because an [`Uslub`]
/// is what the *layout engine* reads, and neither of these changes layout: a
/// rule under or through a run adds no advance, forces no shaping boundary and
/// moves no baseline. They still have to leave this module, because the
/// vocabulary a translator and a patch writer work in names them — an
/// underlined word that came back from translation with no underline is a
/// visible regression, and one that nothing downstream can detect if the fact
/// stops here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub enum Zakhrafa {
    /// Underline: `<u>`, `<underline>`, `[u]`, `{u}`.
    TahtKhat,
    /// Strikethrough: `<s>`, `<strikethrough>`, `[s]`, `{s}`.
    Shatb,
}

/// One decorated span: which span, and what it draws.
///
/// A side table keyed by [`NitaqUslub::id`] rather than a field on the span,
/// because decoration is rare — most spans carry none — and because the span
/// type belongs to the layout engine, which has nothing to do with a rule drawn
/// over a glyph run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct ZakhrafatNitaq {
    /// The identifier of the span this applies to, as in
    /// [`NassNaqi::nitaqat`].
    pub nitaq: u16,
    /// What it draws.
    pub naw: Zakhrafa,
}

/// Clean text, the spans over it, the atoms in it, and enough to undo all of it.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "hifz", derive(serde::Serialize, serde::Deserialize))]
pub struct NassNaqi {
    /// The clean text: no tags, no escapes, one [`BADEEL_DHARRA`] per atom.
    /// This is what goes to `ittijah`, to a translator, and to a machine
    /// translation provider, and it is the only form of the string any of them
    /// ever sees.
    pub nass: String,
    /// Style spans over the clean text, sorted by start offset, and by
    /// decreasing length within one start so that an enclosing span always
    /// precedes the spans nested inside it.
    pub nitaqat: Vec<NitaqUslub>,
    /// Every atom, in the order it appeared.
    pub dharrat: Vec<DharraMustakhraja>,
    /// The rules drawn over spans, in the order the tags that ask for them were
    /// opened. Empty for the great majority of strings.
    pub zakhrafat: Vec<ZakhrafatNitaq>,
    /// What was removed and where, in the order it was removed. Consumed only
    /// by [`aid_binaa`].
    pub aathar: Vec<AtharNasq>,
    /// The length of the raw input in bytes, kept so extraction can report how
    /// much of a string was markup without holding the raw string alive.
    pub asl_tul: u32,
}

impl NassNaqi {
    /// Clean text with nothing extracted from it — the fast-path result, and
    /// the correct answer for a string with no markup in it.
    #[must_use]
    pub fn kama_hiya(khaam: &str) -> Self {
        Self {
            nass: khaam.to_owned(),
            nitaqat: Vec::new(),
            dharrat: Vec::new(),
            zakhrafat: Vec::new(),
            aathar: Vec::new(),
            asl_tul: q32(khaam.len()),
        }
    }

    /// Whether anything at all was extracted.
    #[must_use]
    pub const fn naqi_asasan(&self) -> bool {
        self.aathar.is_empty()
    }

    /// How many bytes of the original were markup rather than text.
    #[must_use]
    pub fn tul_nasq(&self) -> u32 {
        self.asl_tul.saturating_sub(q32(self.nass.len()))
    }

    /// The span carrying a given atom.
    #[must_use]
    pub fn nitaq_dharra(&self, dharra: &DharraMustakhraja) -> Option<&NitaqUslub> {
        self.nitaqat.iter().find(|nitaq| nitaq.id == dharra.nitaq)
    }

    /// The rule a span draws, if it draws one.
    ///
    /// A linear scan, because [`NassNaqi::zakhrafat`] is empty for nearly every
    /// string and holds one or two entries for the rest; an index would cost
    /// more to build than every lookup it would ever serve.
    #[must_use]
    pub fn zakhrafat_nitaq(&self, id: u16) -> Option<Zakhrafa> {
        self.zakhrafat
            .iter()
            .find(|zakhrafa| zakhrafa.nitaq == id)
            .map(|zakhrafa| zakhrafa.naw)
    }
}

/// Narrows a byte offset to the `u32` the span types use.
///
/// Offsets are bounded by the length of one game string. The saturation is
/// unreachable for any real input and is present only so that no path here can
/// panic on a hostile one, which is the crate-wide rule.
fn q32(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// Widens a span offset back to an index. Lossless on every target this crate
/// builds for, saturating rather than panicking on any that it is not.
fn qusize(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}

/// Extracts markup and placeholders, producing clean text plus the tables over
/// it.
///
/// The common case — a short string with no markup at all — is a single scan
/// for the handful of bytes that could begin a construct, followed by one copy
/// of the string. Nothing else is allocated on that path.
///
/// # Errors
///
/// Returns [`KhataSaff::NasqTalif`] with the byte offset in the *raw* input
/// and the rule that was broken:
///
/// - [`SababNasq::UmqZaid`] when nesting passes [`KhiyaratNasq::aqsa_umq`], or
///   when a single string produces more spans than a span identifier can hold.
///   Always reported, in every mode, because it is a bound on this scanner's
///   own state rather than a behaviour of any dialect.
/// - [`SababNasq::MawdiTalif`] when a format placeholder commits to being one
///   and then does not finish: `{gold` with no closing brace, `${name` at the
///   end of the string, `\V` with no `[n]` after it. Always reported, because
///   every one of those makes the game's own formatter throw.
/// - [`SababNasq::WasmMaftuh`], [`SababNasq::WasmMughlaqZaid`] and
///   [`SababNasq::QeemaTalifa`] only when [`KhiyaratNasq::sarim`] is set. Those
///   three are the leniencies the engines themselves have, and reporting them
///   by default would refuse strings the game renders without complaint.
pub fn istakhrij(khaam: &str, khiyarat: &KhiyaratNasq) -> Natija<NassNaqi> {
    // The fast path. Every construct in every dialect begins with one of these
    // bytes, so a string containing none of them cannot contain markup, and no
    // amount of scanning it further will discover any.
    let qina = qina_lahjat(khiyarat);
    let fih_nasq = khaam
        .bytes()
        .any(|bayt| bayt < 0x80 && qina & (1_u128 << u32::from(bayt)) != 0);
    if !fih_nasq {
        return Ok(NassNaqi::kama_hiya(khaam));
    }

    let mut massah = Massah::jadeed(khaam, khiyarat, qina);
    massah.imsah()?;
    massah.ikhtim()
}

/// The set of bytes that can begin a construct, as a bitmask over ASCII.
///
/// A mask rather than a byte-by-byte match because this test runs over every
/// byte of every string the product ever lays out, and it is the only work done
/// at all for the strings that have no markup in them — which is most of them.
fn qina_lahjat(khiyarat: &KhiyaratNasq) -> u128 {
    // Dialect-independent: `{{`, `}}`, `%%` and `\\` are escapes wherever they
    // appear, and a string that carries them carries them for a formatter that
    // is going to unescape them.
    let mut qina = alam(b'{') | alam(b'}') | alam(b'%') | alam(b'\\') | alam(b'$');
    for lahja in &khiyarat.lahjat {
        qina |= match lahja {
            LahjatNasq::Unity | LahjatNasq::Unreal => alam(b'<'),
            // `%` is already dialect-independent, because `%%` is an escape
            // wherever it appears. Named anyway so this arm says what the
            // dialect triggers on rather than leaving a reader to infer it.
            LahjatNasq::UnrealRumuz => alam(b'%'),
            LahjatNasq::BbCode => alam(b'['),
            LahjatNasq::RenPy => alam(b'[') | alam(b'{'),
            LahjatNasq::RpgMaker => alam(b'\\'),
            LahjatNasq::GameMaker => alam(b'#') | alam(b'\\'),
            LahjatNasq::Web => alam(b'{') | alam(b'$'),
        };
    }
    qina
}

/// One bit of the trigger mask.
const fn alam(bayt: u8) -> u128 {
    1_u128 << bayt
}

/// Which bracket syntax opened a tag.
///
/// Carried on the open-tag stack so that `[b]` cannot be closed by `</b>` and
/// `<i>` cannot be closed by `{/i}`. Three engines' markup can appear in one
/// string — a Ren'Py script with `BBCode` enabled is the normal case — and
/// matching a close against the wrong syntax would silently move a style span
/// to cover the wrong words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lisan {
    /// `<tag>` … `</tag>` or `</>`.
    Zawi,
    /// `[tag]` … `[/tag]`.
    Murabba,
    /// `{tag}` … `{/tag}`.
    Muqawwas,
}

/// A tag that has been opened and not yet closed.
struct QaidMaftuh {
    /// The span identifier reserved for it, meaningful only when `yuntij`.
    id: u16,
    /// The tag name, as a byte range into the raw input, so matching a closing
    /// tag costs no allocation.
    ism: Range<usize>,
    /// Byte offset into the clean text where the span's content begins.
    bidaya: u32,
    /// What the span does.
    uslub: Uslub,
    /// Byte offset of the opening tag in the raw input, for error reporting.
    mawqi: u32,
    /// Which bracket syntax opened it.
    lisan: Lisan,
    /// Whether closing it should emit a span at all. `<gradient>` is the one
    /// tag that does not; see `wasm_unity` for why.
    yuntij: bool,
}

/// The scanner's whole state.
struct Massah<'a> {
    khaam: &'a str,
    bayt: &'a [u8],
    khiyarat: &'a KhiyaratNasq,
    /// Cursor into the raw input.
    i: usize,
    /// Start of the literal run not yet copied into the clean text.
    nusikh: usize,
    nass: String,
    nitaqat: Vec<NitaqUslub>,
    dharrat: Vec<DharraMustakhraja>,
    zakhrafat: Vec<ZakhrafatNitaq>,
    aathar: Vec<AtharNasq>,
    kudus: SmallVec<[QaidMaftuh; 8]>,
    /// The next span identifier to hand out.
    talii: u16,
    /// The trigger mask, computed once.
    qina: u128,
}

/// Builds the malformed-markup failure with the offset that caused it.
fn khata(mawqi: usize, sabab: SababNasq) -> KhataSaff {
    KhataSaff::NasqTalif {
        mawqi: q32(mawqi),
        sabab,
    }
}

impl<'a> Massah<'a> {
    fn jadeed(khaam: &'a str, khiyarat: &'a KhiyaratNasq, qina: u128) -> Self {
        Self {
            khaam,
            bayt: khaam.as_bytes(),
            khiyarat,
            i: 0,
            nusikh: 0,
            // The clean text is never longer than the raw text, so one
            // allocation of the raw length is the last one this scan makes for
            // it.
            nass: String::with_capacity(khaam.len()),
            nitaqat: Vec::new(),
            dharrat: Vec::new(),
            zakhrafat: Vec::new(),
            aathar: Vec::new(),
            kudus: SmallVec::new(),
            talii: 0,
            qina,
        }
    }

    /// Copies the pending literal run up to `hatta` into the clean text.
    ///
    /// `hatta` is always the first byte of a recognized construct, and
    /// `nusikh` is always one past the last byte of the previous one. Both are
    /// therefore ASCII positions and never split a UTF-8 sequence.
    fn iltahim(&mut self, hatta: usize) {
        if hatta <= self.nusikh {
            return;
        }
        let khaam = self.khaam;
        if let Some(juz) = khaam.get(self.nusikh..hatta) {
            self.nass.push_str(juz);
        }
        self.nusikh = hatta;
    }

    /// Removes `khaam[bida..niha)` from the text and records what replaced it.
    ///
    /// `badeel` is the clean text that stands in its place: empty for a tag,
    /// [`NASS_BADEEL`] for an atom, a single character for an escape. This one
    /// method is the entire extraction primitive, and every dialect below is
    /// written in terms of it, which is what keeps the round trip exact — there
    /// is no second way to remove text from this scanner.
    fn athar(&mut self, bida: usize, niha: usize, badeel: &str) {
        self.iltahim(bida);
        let khaam = self.khaam;
        if let Some(juz) = khaam.get(bida..niha) {
            self.aathar.push(AtharNasq {
                mawqi: q32(self.nass.len()),
                khaam: juz.to_owned(),
                badeel: q32(badeel.len()),
            });
        }
        self.nass.push_str(badeel);
        self.nusikh = niha;
        self.i = niha;
    }

    /// Reserves the next span identifier.
    fn hawiya(&mut self, mawqi: usize) -> Natija<u16> {
        let id = self.talii;
        // Sixty-five thousand spans in one string is the same class of
        // pathology as sixty-five thousand levels of nesting: the input is not
        // markup, and there is no rendering of it to salvage.
        self.talii = self
            .talii
            .checked_add(1)
            .ok_or_else(|| khata(mawqi, SababNasq::UmqZaid))?;
        Ok(id)
    }

    /// Turns `khaam[bida..niha)` into a single atom.
    fn dharra(
        &mut self,
        bida: usize,
        niha: usize,
        naw: NawDharra,
        tarteeb: Option<u32>,
        ard: f32,
    ) -> Natija<()> {
        let id = self.hawiya(bida)?;
        let marja = q32(self.dharrat.len());
        let khaam = self.khaam;
        let nass_khaam = khaam.get(bida..niha).unwrap_or_default().to_owned();

        self.iltahim(bida);
        let bidaya = q32(self.nass.len());
        self.athar(bida, niha, NASS_BADEEL);

        self.nitaqat.push(NitaqUslub {
            id,
            bidaya,
            tul: q32(NASS_BADEEL.len()),
            // The width is decided per construct, not per kind: `\n` and
            // `\C[2]` draw nothing and get zero, `<space=8>` states its own
            // width, and a sprite or a variable takes the caller's default
            // because only the caller can know what it will expand to.
            uslub: Uslub {
                dharra: Some(Dharra {
                    ard,
                    irtifa: 0.0,
                    asas: 0.0,
                    marja,
                }),
                ..Uslub::default()
            },
        });
        self.dharrat.push(DharraMustakhraja {
            nitaq: id,
            khaam: nass_khaam,
            naw,
            tarteeb,
        });
        Ok(())
    }

    /// Pushes an opened tag onto the stack and removes it from the text.
    ///
    /// Answers the span identifier reserved for the tag, which is meaningful
    /// only when `yuntij`. Callers that need to record something else about the
    /// span — a decoration, so far — key it on that identifier; callers that do
    /// not simply drop it.
    fn iftah(
        &mut self,
        bida: usize,
        niha: usize,
        ism: Range<usize>,
        uslub: Uslub,
        lisan: Lisan,
        yuntij: bool,
    ) -> Natija<u16> {
        if self.kudus.len() >= usize::from(self.khiyarat.umq()) {
            return Err(khata(bida, SababNasq::UmqZaid).into());
        }
        let id = if yuntij { self.hawiya(bida)? } else { 0 };
        self.athar(bida, niha, "");
        self.kudus.push(QaidMaftuh {
            id,
            ism,
            bidaya: q32(self.nass.len()),
            uslub,
            mawqi: q32(bida),
            lisan,
            yuntij,
        });
        Ok(id)
    }

    /// Opens a tag and records the rule it draws, when it draws one.
    ///
    /// The three bracket dialects spell decoration identically — `<u>`, `[u]`
    /// and `{u}` all underline — so the decision is made once here rather than
    /// three times in three tag tables that would drift apart.
    fn iftah_bi_zakhrafa(
        &mut self,
        bida: usize,
        niha: usize,
        ism: Range<usize>,
        uslub: Uslub,
        lisan: Lisan,
    ) -> Natija<()> {
        let naw = self.khaam.get(ism.clone()).and_then(zakhrafa_min_ism);
        let id = self.iftah(bida, niha, ism, uslub, lisan, true)?;
        if let Some(naw) = naw {
            self.zakhrafat.push(ZakhrafatNitaq { nitaq: id, naw });
        }
        Ok(())
    }

    /// Closes the innermost matching open tag and emits its span.
    ///
    /// `ism` is `None` for the closing shorthands — Unity's and Unreal's `</>` —
    /// which close whatever is innermost in their own syntax.
    ///
    /// The matched entry is *removed* from the stack rather than the stack
    /// being unwound to it, which is what Unity's own style stack does and what
    /// makes `<b><i></b></i>` behave the way the engine behaves: bold ends,
    /// italic stays open. The two spans then genuinely overlap, and that is
    /// correct — they set different properties, so there is no position where
    /// the answer is ambiguous, and refusing the string would refuse markup
    /// that renders.
    fn aghliq(
        &mut self,
        bida: usize,
        niha: usize,
        ism: Option<Range<usize>>,
        lisan: Lisan,
    ) -> Natija<()> {
        let khaam = self.khaam;
        let matlub = ism.and_then(|nitaq| khaam.get(nitaq));
        let mawdi = self.kudus.iter().rposition(|qaid| {
            qaid.lisan == lisan
                && matlub.is_none_or(|matlub| {
                    khaam
                        .get(qaid.ism.clone())
                        .is_some_and(|mawjud| mawjud.eq_ignore_ascii_case(matlub))
                })
        });

        let Some(mawdi) = mawdi else {
            // Lenient: every one of these engines drops a closing tag that
            // matches nothing. The tag still leaves the visible text, so it is
            // still recorded, and the round trip still restores it exactly.
            if self.khiyarat.sarim {
                return Err(khata(bida, SababNasq::WasmMughlaqZaid).into());
            }
            self.athar(bida, niha, "");
            return Ok(());
        };

        self.iltahim(bida);
        let nihaya = q32(self.nass.len());
        self.athar(bida, niha, "");

        let qaid = self.kudus.remove(mawdi);
        if qaid.yuntij {
            self.nitaqat.push(NitaqUslub {
                id: qaid.id,
                bidaya: qaid.bidaya,
                tul: nihaya.saturating_sub(qaid.bidaya),
                uslub: qaid.uslub,
            });
        }
        Ok(())
    }

    /// The main dispatch: walk the raw bytes, and at every byte that could
    /// begin a construct, ask the dialect that owns it.
    ///
    /// A handler returns `true` when it consumed a construct and moved the
    /// cursor itself, and `false` when what it found was not markup after all —
    /// in which case the byte stays in the text as an ordinary character, which
    /// is the leniency rule stated in the module documentation.
    fn imsah(&mut self) -> Natija<()> {
        while let Some(&bayt) = self.bayt.get(self.i) {
            if bayt >= 0x80 || self.qina & (1_u128 << u32::from(bayt)) == 0 {
                self.i = self.i.saturating_add(1);
                continue;
            }
            let iltuqita = match bayt {
                b'<' => self.zawi()?,
                b'[' => self.murabba()?,
                b'{' => self.muqawwas()?,
                b'}' => self.mughlaq_muqawwas(),
                b'%' => self.mieawi()?,
                b'$' => self.dulari()?,
                b'\\' => self.mailil()?,
                b'#' => self.shabaka()?,
                _ => false,
            };
            if !iltuqita {
                self.i = self.i.saturating_add(1);
            }
        }
        self.iltahim(self.khaam.len());
        Ok(())
    }

    /// Finishes: closes whatever is still open, orders the spans, and hands
    /// back the result.
    fn ikhtim(mut self) -> Natija<NassNaqi> {
        if let Some(awwal) = self.kudus.first()
            && self.khiyarat.sarim
        {
            return Err(khata(qusize(awwal.mawqi), SababNasq::WasmMaftuh).into());
        }

        // Lenient: an unclosed tag runs to the end of the string, which is what
        // TextMeshPro, Godot and Ren'Py all do. The stack is in open order, so
        // draining it in place gives outer spans before inner ones.
        let nihaya = q32(self.nass.len());
        for qaid in self.kudus.drain(..) {
            if qaid.yuntij {
                self.nitaqat.push(NitaqUslub {
                    id: qaid.id,
                    bidaya: qaid.bidaya,
                    tul: nihaya.saturating_sub(qaid.bidaya),
                    uslub: qaid.uslub,
                });
            }
        }

        // Sorted by start, and by decreasing length within a start, so a
        // consumer applying spans in order always applies the enclosing style
        // before the style nested inside it. A stable sort keeps the original
        // open order for spans that agree on both.
        self.nitaqat.sort_by(|awwal, thani| {
            awwal
                .bidaya
                .cmp(&thani.bidaya)
                .then_with(|| thani.tul.cmp(&awwal.tul))
        });

        Ok(NassNaqi {
            nass: self.nass,
            nitaqat: self.nitaqat,
            dharrat: self.dharrat,
            zakhrafat: self.zakhrafat,
            aathar: self.aathar,
            asl_tul: q32(self.khaam.len()),
        })
    }
}

/// The extent of a bracketed tag, as byte ranges into the raw input.
struct HududWasm {
    /// One past the closing bracket.
    niha: usize,
    /// The tag name.
    ism: Range<usize>,
    /// Everything between the name and the closing bracket: `=value`, or a run
    /// of `attribute=value` pairs, or nothing.
    himl: Range<usize>,
    /// The tag began with a slash.
    mughlaq: bool,
    /// The tag was the bare closing shorthand — `</>` in Unity and Unreal.
    mukhtasar: bool,
}

/// Whether a byte can appear in a tag name.
///
/// The dot is here for Unreal, whose style names are dotted paths such as
/// `RichText.Bold`, and the hyphen for Unity's `line-height`.
const fn bayt_ism(bayt: u8) -> bool {
    bayt.is_ascii_alphanumeric() || bayt == b'-' || bayt == b'_' || bayt == b'.'
}

/// Lowercases an ASCII tag name into a caller-owned buffer.
///
/// Tag names are matched case-insensitively by every engine here, and this
/// avoids an allocation per tag on a path that runs over every string the
/// product lays out. A name longer than the buffer is not a name any of these
/// dialects defines, so it is rejected rather than truncated.
fn asfar<'b>(ism: &str, mukhazzan: &'b mut [u8]) -> Option<&'b str> {
    let bayt = ism.as_bytes();
    if bayt.is_empty() || bayt.len() > mukhazzan.len() || !ism.is_ascii() {
        return None;
    }
    for (hadaf, masdar) in mukhazzan.iter_mut().zip(bayt) {
        *hadaf = masdar.to_ascii_lowercase();
    }
    mukhazzan
        .get(..bayt.len())
        .and_then(|juz| core::str::from_utf8(juz).ok())
}

/// The rule a tag name draws, for the four names that draw one.
///
/// Case-insensitive, like every tag name in every dialect here, and shared by
/// all three bracket syntaxes because all three spell decoration the same way.
/// A name this does not know draws nothing, which is the answer for the great
/// majority of tags.
fn zakhrafa_min_ism(ism: &str) -> Option<Zakhrafa> {
    let mut mukhazzan = [0_u8; 16];
    match asfar(ism, &mut mukhazzan)? {
        "u" | "underline" => Some(Zakhrafa::TahtKhat),
        "s" | "strikethrough" => Some(Zakhrafa::Shatb),
        _ => None,
    }
}

/// Finds the closing tag for `ism` at or after `min`, case-insensitively, in
/// whichever bracket pair the dialect uses.
fn mawdi_ighlaq(khaam: &str, min: usize, ism: &str, fath: u8, ghalq: u8) -> Option<(usize, usize)> {
    let bayt = khaam.as_bytes();
    let mut i = min;
    while i < bayt.len() {
        if bayt.get(i) == Some(&fath) && bayt.get(i.saturating_add(1)) == Some(&b'/') {
            let baad = i.saturating_add(2);
            let baad_ism = baad.saturating_add(ism.len());
            if khaam
                .get(baad..baad_ism)
                .is_some_and(|juz| juz.eq_ignore_ascii_case(ism))
                && bayt.get(baad_ism) == Some(&ghalq)
            {
                return Some((i, baad_ism.saturating_add(1)));
            }
        }
        i = i.saturating_add(1);
    }
    None
}

impl Massah<'_> {
    /// Measures a bracketed tag beginning at `bida`, or decides there is none.
    ///
    /// Returns `None` — meaning the bracket is ordinary punctuation — when the
    /// name is empty, when the closing bracket does not arrive within
    /// [`HADD_WASM`] bytes, or when another opening bracket or a line break
    /// arrives first. That last rule is what stops `a < b and c > d` from being
    /// read as a tag spanning half a sentence.
    fn hudud(&self, bida: usize, fath: u8, ghalq: u8) -> Option<HududWasm> {
        let bayt = self.bayt;
        let mut i = bida.saturating_add(1);
        let mughlaq = bayt.get(i) == Some(&b'/');
        if mughlaq {
            i = i.saturating_add(1);
            if bayt.get(i) == Some(&ghalq) {
                return Some(HududWasm {
                    niha: i.saturating_add(1),
                    ism: i..i,
                    himl: i..i,
                    mughlaq: true,
                    mukhtasar: true,
                });
            }
        }

        let bidayat_ism = i;
        while bayt.get(i).is_some_and(|b| bayt_ism(*b)) {
            i = i.saturating_add(1);
        }
        if i == bidayat_ism {
            return None;
        }
        let ism = bidayat_ism..i;

        let mut k = i;
        let hadd = bida.saturating_add(HADD_WASM).min(bayt.len());
        while k < hadd {
            match bayt.get(k) {
                Some(&b) if b == ghalq => {
                    return Some(HududWasm {
                        niha: k.saturating_add(1),
                        ism,
                        himl: i..k,
                        mughlaq,
                        mukhtasar: false,
                    });
                },
                Some(&b) if b == fath || b == b'\n' || b == b'\r' => return None,
                Some(_) => k = k.saturating_add(1),
                None => return None,
            }
        }
        None
    }

    /// Dispatches an angle bracket to Unity, then to Unreal, then to nothing.
    ///
    /// The order matters when both dialects are enabled: Unity's tags are a
    /// fixed, known vocabulary and Unreal's style names are arbitrary
    /// identifiers, so a name Unity knows must be read as Unity's, and only a
    /// name Unity does not know can be an Unreal style. A tag neither dialect
    /// claims stays in the text as literal characters — Unity's own behaviour,
    /// and the safer failure: a tag the game meant to display and this module
    /// silently ate would not be noticed until a player saw the gap.
    fn zawi(&mut self) -> Natija<bool> {
        let bida = self.i;
        let Some(hudud) = self.hudud(bida, b'<', b'>') else {
            return Ok(false);
        };

        if hudud.mukhtasar {
            if self.khiyarat.tashmal(LahjatNasq::Unity) || self.khiyarat.tashmal(LahjatNasq::Unreal)
            {
                self.aghliq(bida, hudud.niha, None, Lisan::Zawi)?;
                return Ok(true);
            }
            return Ok(false);
        }

        let khaam = self.khaam;
        let ism = khaam.get(hudud.ism.clone()).unwrap_or_default();
        let mut mukhazzan = [0_u8; 24];
        let asfar_ism = asfar(ism, &mut mukhazzan);

        if self.khiyarat.tashmal(LahjatNasq::Unity)
            && let Some(asfar_ism) = asfar_ism
            && wasm_unity_maruf(asfar_ism)
        {
            return self.unity(&hudud, asfar_ism);
        }
        if self.khiyarat.tashmal(LahjatNasq::Unreal) && !ism.is_empty() {
            return self.unreal(&hudud);
        }
        Ok(false)
    }

    /// Unity rich text, as TextMeshPro and the legacy UI parse it.
    ///
    /// Returns `false` when the tag name is one Unity knows but the value is
    /// not readable as what that tag requires. Unity's own parser does the same
    /// thing in that case — it abandons the tag and emits the characters — and
    /// matching it is the difference between rendering what the game renders
    /// and rendering something else.
    fn unity(&mut self, hudud: &HududWasm, ism: &str) -> Natija<bool> {
        let bida = self.i;
        let niha = hudud.niha;

        if hudud.mughlaq {
            self.aghliq(bida, niha, Some(hudud.ism.clone()), Lisan::Zawi)?;
            return Ok(true);
        }

        let khaam = self.khaam;
        let himl = khaam.get(hudud.himl.clone()).unwrap_or_default();
        let qeema = qeema_himl(himl);
        let asas = self.khiyarat.hajm_asas;
        let iftiradi = self.khiyarat.ard_dharra_iftiradi;

        // Atoms. Each of these is a self-contained instruction with no closing
        // tag, so it becomes one atom rather than opening a span that would
        // never be closed.
        match ism {
            // `<br>` is a hard line break, `<pos>` an absolute horizontal
            // position, `<page>` a page break. All three are instructions to
            // the engine's own layout with no width of their own, and all three
            // stay atoms rather than becoming the characters they stand for:
            // writing a real U+000A into the clean text in place of `<br>`
            // would make the reconstructed string differ from the original. A
            // caller that wants the break asks the atom table for it.
            "br" | "pos" | "page" => {
                self.dharra(bida, niha, NawDharra::Amr, None, 0.0)?;
                return Ok(true);
            },
            "space" => {
                let ard = qeema
                    .and_then(hall_qadr)
                    .and_then(|qadr| qadr.bikselat(asas))
                    .unwrap_or(0.0);
                self.dharra(bida, niha, NawDharra::Amr, None, ard)?;
                return Ok(true);
            },
            "sprite" => {
                // Every spelling Unity accepts — `<sprite=3>`,
                // `<sprite index=3>`, `<sprite name="star">`,
                // `<sprite="Sheet" index=3>` — is the same thing to layout: an
                // inline image of a width only the engine's sprite asset knows.
                self.dharra(bida, niha, NawDharra::Sura, None, iftiradi)?;
                return Ok(true);
            },
            "noparse" => return self.noparse(bida, niha),
            _ => {},
        }

        // Paired tags. `uslub` carries only what changes layout; everything
        // else survives on the span's identity and in the reconstruction
        // record, which is what the adapter reads to reapply it.
        let uslub = match ism {
            // Bold is a weight, and a weight is what a variable font and a
            // font chain both understand. There is no separate bold flag to
            // set, and there should not be: `<b>` on a family that has a real
            // semibold should reach that face, not a synthesised one.
            "b" => Uslub {
                wazn: Some(700),
                ..Uslub::default()
            },
            "i" => Uslub {
                maail: true,
                ..Uslub::default()
            },
            "size" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    hajm: qadr.bikselat(asas),
                    ..Uslub::default()
                }
            },
            "color" | "mark" => {
                let Some(lawn) = qeema.and_then(hall_lawn) else {
                    return self.la_qeema(bida);
                };
                // `<mark>` is a highlight behind the text rather than the text
                // colour, so it opens a span that does not carry the colour
                // forward: a renderer that read it from `lawn` would paint the
                // glyphs the highlight colour instead of painting behind them.
                if ism == "mark" {
                    Uslub::default()
                } else {
                    Uslub {
                        lawn: Some(lawn),
                        ..Uslub::default()
                    }
                }
            },
            "voffset" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    izaha: qadr.bikselat(asas),
                    ..Uslub::default()
                }
            },
            "cspace" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    tabaud: qadr.bikselat(asas),
                    ..Uslub::default()
                }
            },
            "align" => {
                let salih = qeema.is_some_and(|qeema| {
                    let mut buf = [0_u8; 16];
                    asfar(qeema, &mut buf).is_some_and(|qeema| {
                        matches!(qeema, "left" | "right" | "center" | "justified" | "flush")
                    })
                });
                if !salih {
                    return self.la_qeema(bida);
                }
                Uslub::default()
            },
            "gradient" => {
                // Parsed so that it is removed from the text and restored by
                // the round trip, and then dropped: a gradient is a colour
                // effect the engine applies across a run's own glyphs, not a
                // layout property. There is nothing in a layout result for it
                // to change, and carrying it as a style span would invite a
                // renderer to treat it as a flat colour, which is worse than
                // not carrying it.
                self.iftah(
                    bida,
                    niha,
                    hudud.ism.clone(),
                    Uslub::default(),
                    Lisan::Zawi,
                    false,
                )?;
                return Ok(true);
            },
            // Everything else opens a span with no layout effect of its own:
            // interaction (`link`), casing, the named font and style sheet, and
            // the paragraph-level geometry Unity applies outside the run.
            // `font` in particular resolves a *family name*, and turning a
            // family name into an index into a font chain is `khatt`'s work —
            // this module must not know that a font chain exists.
            //
            // `u` and `s` are here too, because a rule drawn over a run is not
            // a layout property either. What they are is recorded beside the
            // span rather than in it; see [`Zakhrafa`].
            _ => Uslub::default(),
        };

        self.iftah_bi_zakhrafa(bida, niha, hudud.ism.clone(), uslub, Lisan::Zawi)?;
        Ok(true)
    }

    /// A known tag whose value could not be read as the kind it must be.
    ///
    /// Lenient by default because Unity's parser is: it abandons the tag and
    /// leaves the characters in the text, which is why `<size=large>` shows up
    /// on screen in shipped games rather than being silently dropped.
    fn la_qeema(&self, bida: usize) -> Natija<bool> {
        if self.khiyarat.sarim {
            return Err(khata(bida, SababNasq::QeemaTalifa).into());
        }
        Ok(false)
    }

    /// `<noparse>` … `</noparse>`: a region the engine is told not to
    /// interpret, and neither does this. Everything between the two tags is
    /// copied into the clean text verbatim, including any `<`, `[`, `{` or `%`
    /// it contains.
    fn noparse(&mut self, bida: usize, niha: usize) -> Natija<bool> {
        let khaam = self.khaam;
        let id = self.hawiya(bida)?;
        self.athar(bida, niha, "");
        let bidaya = q32(self.nass.len());

        if let Some((qat_bida, qat_niha)) = mawdi_ighlaq(khaam, niha, "noparse", b'<', b'>') {
            self.iltahim(qat_bida);
            let tul = q32(self.nass.len()).saturating_sub(bidaya);
            self.athar(qat_bida, qat_niha, "");
            self.nitaqat.push(NitaqUslub {
                id,
                bidaya,
                tul,
                uslub: Uslub::default(),
            });
        } else {
            if self.khiyarat.sarim {
                return Err(khata(bida, SababNasq::WasmMaftuh).into());
            }
            self.iltahim(khaam.len());
            self.i = khaam.len();
            let tul = q32(self.nass.len()).saturating_sub(bidaya);
            self.nitaqat.push(NitaqUslub {
                id,
                bidaya,
                tul,
                uslub: Uslub::default(),
            });
        }
        Ok(true)
    }
}

/// The Unity rich-text vocabulary.
///
/// A name outside this set is not a Unity tag, and by Unity's own rule it stays
/// in the text as literal characters. The set is deliberately the full one
/// TextMeshPro implements rather than a useful subset: a tag missing from here
/// would be rendered visibly by Taarib and invisibly by the game, and that
/// difference is exactly the kind of defect a patch ships and nobody notices
/// until it is on screen.
fn wasm_unity_maruf(ism: &str) -> bool {
    matches!(
        ism,
        "b" | "i"
            | "u"
            | "s"
            | "sup"
            | "sub"
            | "size"
            | "color"
            | "alpha"
            | "mark"
            | "sprite"
            | "link"
            | "voffset"
            | "cspace"
            | "mspace"
            | "indent"
            | "align"
            | "nobr"
            | "noparse"
            | "pos"
            | "space"
            | "font"
            | "style"
            | "width"
            | "line-height"
            | "margin"
            | "rotate"
            | "gradient"
            | "br"
            | "page"
            | "underline"
            | "strikethrough"
            | "uppercase"
            | "lowercase"
            | "smallcaps"
            | "allcaps"
    )
}

/// The `=value` form of a tag payload, with any surrounding quotes removed.
fn qeema_himl(himl: &str) -> Option<&str> {
    let baqi = himl.strip_prefix('=')?.trim();
    Some(nazi_iqtibas(baqi))
}

/// Strips one matched pair of quotes.
fn nazi_iqtibas(qeema: &str) -> &str {
    qeema
        .strip_prefix('"')
        .and_then(|juz| juz.strip_suffix('"'))
        .or_else(|| {
            qeema
                .strip_prefix('\'')
                .and_then(|juz| juz.strip_suffix('\''))
        })
        .unwrap_or(qeema)
}

/// A length as the markup dialects write them, before it is resolved.
///
/// Kept unresolved because three of the four forms mean nothing without the
/// size the text is being laid out at, and this module does not have it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Qadr {
    /// An absolute number of pixels: `12`, `12.5`, `40px`.
    Mutlaq(f32),
    /// A fraction of the base size: `120%`, `1.5em`.
    Nisbi(f32),
    /// An offset from the base size: `+2`, `-3`.
    Izafi(f32),
}

impl Qadr {
    /// Resolves against a base size, or gives up when there is none.
    ///
    /// Giving up is deliberate. A relative size that cannot be resolved leaves
    /// the span in place with no size override, so the tag still round-trips
    /// and the adapter still sees it; inventing a number would put text on
    /// screen at a size the game never asked for, and the preview would be a
    /// lie about what ships.
    fn bikselat(self, asas: Option<f32>) -> Option<f32> {
        match self {
            Self::Mutlaq(qeema) => Some(qeema),
            Self::Nisbi(nisba) => asas.map(|asas| asas * nisba),
            Self::Izafi(izaha) => asas.map(|asas| asas + izaha),
        }
    }
}

/// Strips a two-letter unit suffix, case-insensitively.
fn bila_wahda<'b>(qeema: &'b str, wahda: &str) -> Option<&'b str> {
    let hadd = qeema.len().checked_sub(wahda.len())?;
    let (raqm, lahiqa) = qeema.split_at_checked(hadd)?;
    lahiqa.eq_ignore_ascii_case(wahda).then_some(raqm)
}

/// Reads a length in any of the forms Unity, Godot and Ren'Py accept.
fn hall_qadr(qeema: &str) -> Option<Qadr> {
    let qeema = qeema.trim();
    if qeema.is_empty() {
        return None;
    }
    if let Some(raqm) = qeema.strip_suffix('%') {
        let mahlul: f32 = raqm.trim().parse().ok()?;
        return mahlul.is_finite().then_some(Qadr::Nisbi(mahlul / 100.0));
    }
    if let Some(raqm) = bila_wahda(qeema, "em") {
        let mahlul: f32 = raqm.trim().parse().ok()?;
        return mahlul.is_finite().then_some(Qadr::Nisbi(mahlul));
    }
    let raqm = bila_wahda(qeema, "px").unwrap_or(qeema);
    // The sign is read before parsing, not after: `+2` and `2` parse to the
    // same number but mean different things, and only the written sign says
    // which.
    let izafi = raqm.starts_with('+') || raqm.starts_with('-');
    let mahlul: f32 = raqm.trim().parse().ok()?;
    if !mahlul.is_finite() {
        return None;
    }
    Some(if izafi {
        Qadr::Izafi(mahlul)
    } else {
        Qadr::Mutlaq(mahlul)
    })
}

/// Reads a colour in any of the forms these dialects accept: three, four, six
/// or eight hexadecimal digits after a `#`, or one of the names the engines
/// define.
fn hall_lawn(qeema: &str) -> Option<[u8; 4]> {
    let qeema = nazi_iqtibas(qeema.trim());
    if let Some(sitteen) = qeema.strip_prefix('#') {
        return lawn_sitta_ashar(sitteen);
    }
    let mut mukhazzan = [0_u8; 16];
    let ism = asfar(qeema, &mut mukhazzan)?;
    let (ahmar, akhdar, azraq) = match ism {
        "black" => (0x00, 0x00, 0x00),
        "white" => (0xFF, 0xFF, 0xFF),
        "red" => (0xFF, 0x00, 0x00),
        "green" | "lime" => (0x00, 0xFF, 0x00),
        "blue" => (0x00, 0x00, 0xFF),
        "yellow" => (0xFF, 0xFF, 0x00),
        "cyan" | "aqua" => (0x00, 0xFF, 0xFF),
        "magenta" | "fuchsia" => (0xFF, 0x00, 0xFF),
        "orange" => (0xFF, 0xA5, 0x00),
        "purple" => (0x80, 0x00, 0x80),
        "brown" => (0xA5, 0x2A, 0x2A),
        "lightblue" => (0xAD, 0xD8, 0xE6),
        "darkblue" => (0x00, 0x00, 0x8B),
        "grey" | "gray" => (0x80, 0x80, 0x80),
        "maroon" => (0x80, 0x00, 0x00),
        "navy" => (0x00, 0x00, 0x80),
        "olive" => (0x80, 0x80, 0x00),
        "pink" => (0xFF, 0xC0, 0xCB),
        "silver" => (0xC0, 0xC0, 0xC0),
        "teal" => (0x00, 0x80, 0x80),
        _ => return None,
    };
    Some([ahmar, akhdar, azraq, 0xFF])
}

/// Reads `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`.
fn lawn_sitta_ashar(sitteen: &str) -> Option<[u8; 4]> {
    let bayt = sitteen.as_bytes();
    if !bayt.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let khana = |fahras: usize| -> Option<u8> {
        bayt.get(fahras)
            .map(|b| char::from(*b))
            .and_then(|h| h.to_digit(16))
            .and_then(|d| u8::try_from(d).ok())
    };
    let mut lawn = [0xFF_u8; 4];
    match bayt.len() {
        // Short form: each digit stands for a doubled byte, so `f` is `ff`.
        tul @ (3 | 4) => {
            for (fahras, hadaf) in lawn.iter_mut().enumerate().take(tul) {
                *hadaf = khana(fahras)?.saturating_mul(17);
            }
        },
        tul @ (6 | 8) => {
            let azwaj = if tul == 8 { 4 } else { 3 };
            for (fahras, hadaf) in lawn.iter_mut().enumerate().take(azwaj) {
                let ala = khana(fahras.saturating_mul(2))?;
                let adna = khana(fahras.saturating_mul(2).saturating_add(1))?;
                *hadaf = ala.saturating_mul(16).saturating_add(adna);
            }
        },
        _ => return None,
    }
    Some(lawn)
}

impl Massah<'_> {
    /// Unreal rich text: `<Style>text</>` and `<Style attribute="value">text</>`.
    ///
    /// The style name is carried on the span's identity and in the
    /// reconstruction record, not decoded into an [`Uslub`]. Unreal resolves a
    /// style name against the widget's own style set — a table of fonts,
    /// sizes, colours and outlines that lives in the game's own asset data —
    /// and this crate has no access to that table and, by the boundary Phase 1
    /// draws around it, must never acquire one. The adapter that does have it
    /// reads the raw tag back out and applies the real style.
    fn unreal(&mut self, hudud: &HududWasm) -> Natija<bool> {
        let bida = self.i;
        if hudud.mughlaq {
            self.aghliq(bida, hudud.niha, Some(hudud.ism.clone()), Lisan::Zawi)?;
            return Ok(true);
        }
        self.iftah(
            bida,
            hudud.niha,
            hudud.ism.clone(),
            Uslub::default(),
            Lisan::Zawi,
            true,
        )?;
        Ok(true)
    }

    /// A square bracket: `BBCode` for Godot and Ren'Py, or a Ren'Py substitution.
    ///
    /// `BBCode` is tried first when both are enabled, because `[b]` is bold in
    /// every `BBCode` document ever written and a variable named `b` in almost
    /// none. A bracket that is neither stays in the text, which is what Godot
    /// does with an unrecognized tag.
    fn murabba(&mut self) -> Natija<bool> {
        let bida = self.i;
        let khaam = self.khaam;
        let bbcode = self.khiyarat.tashmal(LahjatNasq::BbCode);
        let renpy = self.khiyarat.tashmal(LahjatNasq::RenPy);

        // Ren'Py doubles the bracket to mean a literal one. The clean text gets
        // the single character it stands for, so it is measured and shaped as
        // the one glyph the player sees.
        if renpy && self.bayt.get(bida.saturating_add(1)) == Some(&b'[') {
            self.athar(bida, bida.saturating_add(2), "[");
            return Ok(true);
        }

        if bbcode && let Some(hudud) = self.hudud(bida, b'[', b']') {
            let ism = khaam.get(hudud.ism.clone()).unwrap_or_default();
            let mut mukhazzan = [0_u8; 24];
            if let Some(asfar_ism) = asfar(ism, &mut mukhazzan)
                && wasm_bbcode_maruf(asfar_ism)
            {
                return self.bbcode(&hudud, asfar_ism);
            }
        }

        if renpy && let Some(niha) = self.hudud_istibdal(bida) {
            let iftiradi = self.khiyarat.ard_dharra_iftiradi;
            self.dharra(bida, niha, NawDharra::Mutaghayyir, None, iftiradi)?;
            return Ok(true);
        }

        Ok(false)
    }

    /// `BBCode`, as Godot's `RichTextLabel` parses it.
    fn bbcode(&mut self, hudud: &HududWasm, ism: &str) -> Natija<bool> {
        let bida = self.i;
        let niha = hudud.niha;

        if hudud.mughlaq {
            self.aghliq(bida, niha, Some(hudud.ism.clone()), Lisan::Murabba)?;
            return Ok(true);
        }

        let khaam = self.khaam;
        let himl = khaam.get(hudud.himl.clone()).unwrap_or_default();
        let qeema = qeema_himl(himl);

        match ism {
            // The bracket escapes. Like Ren'Py's `[[`, these become the single
            // character they stand for rather than an atom, because the player
            // sees a bracket and the line breaker and the shaper should too.
            "lb" => {
                self.athar(bida, niha, "[");
                return Ok(true);
            },
            "rb" => {
                self.athar(bida, niha, "]");
                return Ok(true);
            },
            "br" => {
                self.dharra(bida, niha, NawDharra::Amr, None, 0.0)?;
                return Ok(true);
            },
            // `[img]path[/img]` wraps a *path*, not text. The whole
            // construct — both tags and everything between them — is one atom,
            // because the path must never be translated, never be shaped, and
            // never be broken across a line.
            "img" => {
                let (qat_niha, bila_ighlaq) = mawdi_ighlaq(khaam, niha, "img", b'[', b']')
                    .map_or((niha, true), |(_, ila)| (ila, false));
                if bila_ighlaq && self.khiyarat.sarim {
                    return Err(khata(bida, SababNasq::WasmMaftuh).into());
                }
                self.dharra(
                    bida,
                    qat_niha,
                    NawDharra::Sura,
                    None,
                    self.khiyarat.ard_dharra_iftiradi,
                )?;
                return Ok(true);
            },
            _ => {},
        }

        let uslub = match ism {
            "b" => Uslub {
                wazn: Some(700),
                ..Uslub::default()
            },
            "i" => Uslub {
                maail: true,
                ..Uslub::default()
            },
            "color" | "fgcolor" => {
                let Some(lawn) = qeema.and_then(hall_lawn) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    lawn: Some(lawn),
                    ..Uslub::default()
                }
            },
            "size" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    hajm: qadr.bikselat(self.khiyarat.hajm_asas),
                    ..Uslub::default()
                }
            },
            // Alignment, indentation, decoration, the named font, the
            // background colour and Godot's animated effects all leave layout
            // within the run untouched, so they open a span with no style of
            // their own and are reapplied from the reconstruction record.
            _ => Uslub::default(),
        };

        self.iftah_bi_zakhrafa(bida, niha, hudud.ism.clone(), uslub, Lisan::Murabba)?;
        Ok(true)
    }

    /// The extent of a Ren'Py substitution — `[points]`, `[obj.name]`,
    /// `[amount!q]`, `[cost:>6]` — or `None` when the brackets hold something
    /// else.
    fn hudud_istibdal(&self, bida: usize) -> Option<usize> {
        let bayt = self.bayt;
        let mut i = bida.saturating_add(1);
        // A substitution names a Python expression, and every one of those
        // begins with a letter or an underscore. Requiring that is what keeps
        // `[3]` and `[ ]` out.
        if !bayt
            .get(i)
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        {
            return None;
        }
        let hadd = bida.saturating_add(HADD_WASM).min(bayt.len());
        while i < hadd {
            match bayt.get(i) {
                Some(&b']') => return Some(i.saturating_add(1)),
                Some(&(b'[' | b'\n' | b'\r')) | None => return None,
                Some(_) => i = i.saturating_add(1),
            }
        }
        None
    }
}

/// The `BBCode` vocabulary Godot's `RichTextLabel` recognizes, plus the two
/// bracket escapes.
///
/// As with Unity, the set is the engine's rather than a convenient subset: a
/// tag missing from here would show on screen in Taarib's rendering and be
/// consumed in the game's, and that divergence is invisible until someone looks
/// at the finished patch.
fn wasm_bbcode_maruf(ism: &str) -> bool {
    matches!(
        ism,
        "b" | "i"
            | "u"
            | "s"
            | "color"
            | "bgcolor"
            | "fgcolor"
            | "font"
            | "font_size"
            | "size"
            | "img"
            | "url"
            | "center"
            | "right"
            | "left"
            | "fill"
            | "indent"
            | "code"
            | "p"
            | "br"
            | "lb"
            | "rb"
            | "outline_size"
            | "outline_color"
            | "table"
            | "cell"
            | "ol"
            | "ul"
            | "hint"
            | "lang"
            | "wave"
            | "tornado"
            | "shake"
            | "fade"
            | "rainbow"
            | "pulse"
    )
}

/// One past the last ASCII digit at or after `i`.
fn nihayat_arqam(bayt: &[u8], mut i: usize) -> usize {
    while bayt.get(i).is_some_and(u8::is_ascii_digit) {
        i = i.saturating_add(1);
    }
    i
}

/// Whether a byte can begin a placeholder name.
///
/// ASCII only, and that is the load-bearing part. Arabic prose brackets whole
/// phrases in braces — a Quranic quotation is written `{...}` by convention —
/// and treating `{إنا}` as a format placeholder would delete the quotation from
/// the visible text. A placeholder name in every dialect here is ASCII, so
/// requiring ASCII costs nothing and removes that whole class of damage.
const fn bidayat_ism_mawdi(bayt: u8) -> bool {
    bayt.is_ascii_alphanumeric() || bayt == b'_'
}

/// Whether a byte can continue a placeholder name.
const fn dakhil_ism_mawdi(bayt: u8) -> bool {
    bayt.is_ascii_alphanumeric() || bayt == b'_' || bayt == b'.' || bayt == b'[' || bayt == b']'
}

impl Massah<'_> {
    /// An opening brace: an escape, a Ren'Py text tag, a web substitution, or a
    /// `{0}`-style format placeholder.
    fn muqawwas(&mut self) -> Natija<bool> {
        let bida = self.i;
        let bayt = self.bayt;

        if bayt.get(bida.saturating_add(1)) == Some(&b'{') {
            // `{{name}}` is a web template substitution; `{{` anywhere else is
            // the escape that .NET, Python and Ren'Py all use for a literal
            // brace, and becomes the one character it stands for.
            if self.khiyarat.tashmal(LahjatNasq::Web)
                && let Some(niha) = self.hudud_mustache(bida)
            {
                self.dharra(
                    bida,
                    niha,
                    NawDharra::Mawdi,
                    None,
                    self.khiyarat.ard_dharra_iftiradi,
                )?;
                return Ok(true);
            }
            self.athar(bida, bida.saturating_add(2), "{");
            return Ok(true);
        }

        if self.khiyarat.tashmal(LahjatNasq::RenPy)
            && let Some(hudud) = self.hudud(bida, b'{', b'}')
        {
            let ism = self.khaam.get(hudud.ism.clone()).unwrap_or_default();
            let mut mukhazzan = [0_u8; 24];
            if let Some(asfar_ism) = asfar(ism, &mut mukhazzan)
                && wasm_renpy_maruf(asfar_ism)
            {
                // A known Ren'Py tag name wins over a format placeholder, which
                // is the same precedence Ren'Py itself applies: `{b}` is bold
                // and `{gold}` is an interpolation, and the difference is
                // whether the name is in the tag table.
                return self.renpy(&hudud, asfar_ism);
            }
        }

        self.mawdi_muqawwas(bida)
    }

    /// A closing brace, which is only ever markup as half of the `}}` escape.
    fn mughlaq_muqawwas(&mut self) -> bool {
        let bida = self.i;
        if self.bayt.get(bida.saturating_add(1)) == Some(&b'}') {
            self.athar(bida, bida.saturating_add(2), "}");
            return true;
        }
        false
    }

    /// `{0}`, `{0:N2}`, `{0,-8:X}`, `{name}`, `{obj.field}`.
    ///
    /// # Errors
    ///
    /// [`SababNasq::MawdiTalif`] when the brace committed to a placeholder — a
    /// well-formed name followed by a well-formed specifier — and then the
    /// string ended before the closing brace. Every formatter that reads this
    /// syntax throws on exactly that input, so it is a defect in the source
    /// string rather than a leniency to match.
    fn mawdi_muqawwas(&mut self, bida: usize) -> Natija<bool> {
        let bayt = self.bayt;
        let mut i = bida.saturating_add(1);
        if !bayt.get(i).copied().is_some_and(bidayat_ism_mawdi) {
            return Ok(false);
        }
        let bidayat_ism = i;
        while bayt.get(i).copied().is_some_and(dakhil_ism_mawdi) {
            i = i.saturating_add(1);
        }
        let ism = self.khaam.get(bidayat_ism..i).unwrap_or_default();

        // The alignment field: `,` then an optionally signed width.
        if bayt.get(i) == Some(&b',') {
            i = i.saturating_add(1);
            if matches!(bayt.get(i), Some(&(b'+' | b'-'))) {
                i = i.saturating_add(1);
            }
            let baad = nihayat_arqam(bayt, i);
            if baad == i {
                return Ok(false);
            }
            i = baad;
        }

        // The format field: `:` then anything up to the closing brace, with
        // braces inside it allowed to nest.
        //
        // They nest because Unity Localization's Smart Strings put brace
        // arguments in there — `{players:list:{}|,   |   and   }` is R.E.P.O.'s
        // own player list, an item template of `{}` and two literal separators.
        // Refusing at the inner `{` produced no span at all for that string, so
        // the whole construct went to the translator as ordinary text with
        // nothing to stop it being rewritten. The depth ceiling is
        // [`UMQ_TANSIQ`], which is what keeps prose from being read as a
        // formatter.
        if bayt.get(i) == Some(&b':') {
            i = i.saturating_add(1);
            let hadd = bida.saturating_add(HADD_WASM).min(bayt.len());
            let mut umq: u8 = 0;
            loop {
                match bayt.get(i) {
                    // Only a brace that closes nothing nested ends the field.
                    Some(&b'}') if umq == 0 => break,
                    // A format specifier longer than the scan limit is not one,
                    // and neither is an unbalanced brace that would otherwise
                    // run the scan to the end of the string.
                    Some(_) if i >= hadd => return Ok(false),
                    Some(&b'}') => umq = umq.saturating_sub(1),
                    Some(&b'{') => {
                        umq = umq.saturating_add(1);
                        if umq > UMQ_TANSIQ {
                            return Ok(false);
                        }
                    },
                    Some(&(b'\n' | b'\r')) => return Ok(false),
                    Some(_) => {},
                    // Committed and never finished: the formatter throws here.
                    None => return Err(khata(bida, SababNasq::MawdiTalif).into()),
                }
                i = i.saturating_add(1);
            }
        }

        match bayt.get(i) {
            Some(&b'}') => {},
            // A brace followed by a word and then something that is not part of
            // the syntax was never a placeholder: `{Hello world}` is a sentence
            // in braces, and it stays one.
            Some(_) => return Ok(false),
            None => return Err(khata(bida, SababNasq::MawdiTalif).into()),
        }
        let niha = i.saturating_add(1);
        let tarteeb = ism.parse::<u32>().ok();
        self.dharra(
            bida,
            niha,
            NawDharra::Mawdi,
            tarteeb,
            self.khiyarat.ard_dharra_iftiradi,
        )?;
        Ok(true)
    }

    /// The extent of a `{{name}}` web substitution.
    fn hudud_mustache(&self, bida: usize) -> Option<usize> {
        let bayt = self.bayt;
        let mut i = bida.saturating_add(2);
        // Handlebars section and comment sigils, which name a variable in the
        // same way an ordinary substitution does.
        if matches!(bayt.get(i), Some(&(b'#' | b'/' | b'^' | b'&' | b'!'))) {
            i = i.saturating_add(1);
        }
        if !bayt.get(i).copied().is_some_and(bidayat_ism_mawdi) {
            return None;
        }
        while bayt.get(i).copied().is_some_and(dakhil_ism_mawdi) {
            i = i.saturating_add(1);
        }
        (bayt.get(i) == Some(&b'}') && bayt.get(i.saturating_add(1)) == Some(&b'}'))
            .then(|| i.saturating_add(2))
    }

    /// Ren'Py's own `{tag}` text tags.
    fn renpy(&mut self, hudud: &HududWasm, ism: &str) -> Natija<bool> {
        let bida = self.i;
        let niha = hudud.niha;

        if hudud.mughlaq {
            self.aghliq(bida, niha, Some(hudud.ism.clone()), Lisan::Muqawwas)?;
            return Ok(true);
        }

        let khaam = self.khaam;
        let himl = khaam.get(hudud.himl.clone()).unwrap_or_default();
        let qeema = qeema_himl(himl);

        // The self-closing tags: waits, pauses, the typing-speed controls, and
        // the two that insert measurable space. None of them has a closing
        // form, so each becomes an atom rather than opening a span that would
        // run to the end of the line.
        match ism {
            "w" | "p" | "nw" | "fast" | "done" | "clear" => {
                self.dharra(bida, niha, NawDharra::Amr, None, 0.0)?;
                return Ok(true);
            },
            "space" | "vspace" => {
                let ard = qeema
                    .and_then(hall_qadr)
                    .and_then(|qadr| qadr.bikselat(self.khiyarat.hajm_asas))
                    .unwrap_or(0.0);
                // A vertical space occupies no horizontal room, so only
                // `{space=N}` contributes width.
                let ard = if ism == "space" { ard } else { 0.0 };
                self.dharra(bida, niha, NawDharra::Amr, None, ard)?;
                return Ok(true);
            },
            "image" => {
                self.dharra(
                    bida,
                    niha,
                    NawDharra::Sura,
                    None,
                    self.khiyarat.ard_dharra_iftiradi,
                )?;
                return Ok(true);
            },
            _ => {},
        }

        let uslub = match ism {
            "b" => Uslub {
                wazn: Some(700),
                ..Uslub::default()
            },
            "i" => Uslub {
                maail: true,
                ..Uslub::default()
            },
            "color" => {
                let Some(lawn) = qeema.and_then(hall_lawn) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    lawn: Some(lawn),
                    ..Uslub::default()
                }
            },
            "size" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    hajm: qadr.bikselat(self.khiyarat.hajm_asas),
                    ..Uslub::default()
                }
            },
            "k" => {
                let Some(qadr) = qeema.and_then(hall_qadr) else {
                    return self.la_qeema(bida);
                };
                Uslub {
                    tabaud: qadr.bikselat(self.khiyarat.hajm_asas),
                    ..Uslub::default()
                }
            },
            _ => Uslub::default(),
        };

        self.iftah_bi_zakhrafa(bida, niha, hudud.ism.clone(), uslub, Lisan::Muqawwas)?;
        Ok(true)
    }

    /// A percent sign: the `%%` escape, a printf conversion, or — only when
    /// [`LahjatNasq::UnrealRumuz`] is enabled — an Unreal glyph slot.
    fn mieawi(&mut self) -> Natija<bool> {
        let bida = self.i;
        let bayt = self.bayt;
        if bayt.get(bida.saturating_add(1)) == Some(&b'%') {
            self.athar(bida, bida.saturating_add(2), "%");
            return Ok(true);
        }

        let mut i = bida.saturating_add(1);
        let mut tarteeb = None;

        // A leading run of digits is a positional argument only when a `$`
        // follows it; otherwise it is the field width and belongs below.
        let baad_arqam = nihayat_arqam(bayt, i);
        if baad_arqam > i && bayt.get(baad_arqam) == Some(&b'$') {
            tarteeb = self
                .khaam
                .get(i..baad_arqam)
                .and_then(|juz| juz.parse::<u32>().ok());
            i = baad_arqam.saturating_add(1);
        }

        // Flags, minus the space flag that C defines. `50% off` would otherwise
        // read as a percent sign, a space flag and an octal conversion — a
        // valid printf spec and a catastrophic misreading of a sale banner.
        // No game format string has ever needed `% d`.
        while matches!(bayt.get(i), Some(&(b'-' | b'+' | b'#' | b'0'))) {
            i = i.saturating_add(1);
        }

        if bayt.get(i) == Some(&b'*') {
            i = i.saturating_add(1);
        } else {
            i = nihayat_arqam(bayt, i);
        }

        if bayt.get(i) == Some(&b'.') {
            i = i.saturating_add(1);
            if bayt.get(i) == Some(&b'*') {
                i = i.saturating_add(1);
            } else {
                i = nihayat_arqam(bayt, i);
            }
        }

        // Length modifiers, longest first so `ll` is not read as `l` followed
        // by a conversion that is not there.
        for lahiqa in [
            b"hh".as_slice(),
            b"ll".as_slice(),
            b"h".as_slice(),
            b"l".as_slice(),
            b"L".as_slice(),
            b"q".as_slice(),
            b"j".as_slice(),
            b"z".as_slice(),
            b"t".as_slice(),
        ] {
            let baad = i.saturating_add(lahiqa.len());
            if bayt.get(i..baad) == Some(lahiqa) {
                i = baad;
                break;
            }
        }

        let tahweel = matches!(
            bayt.get(i),
            Some(
                &(b'd'
                    | b'i'
                    | b'u'
                    | b'o'
                    | b'x'
                    | b'X'
                    | b'e'
                    | b'E'
                    | b'f'
                    | b'F'
                    | b'g'
                    | b'G'
                    | b'a'
                    | b'A'
                    | b'c'
                    | b's'
                    | b'p'
                    | b'n'
                    | b'@')
            )
        );
        if !tahweel {
            // Not a C conversion. It may still be a glyph slot, and printf is
            // asked first precisely so that `%0d`, `%1$s` and `%.3f` are read
            // as the conversions they are rather than as slots zero and one
            // followed by loose letters.
            return self.ramz_mieawi(bida);
        }

        let niha = i.saturating_add(1);
        self.dharra(
            bida,
            niha,
            NawDharra::Mawdi,
            tarteeb,
            self.khiyarat.ard_dharra_iftiradi,
        )?;
        Ok(true)
    }

    /// Unreal's `%0`-style glyph slot: a percent sign and a run of digits the
    /// game replaces with a button icon or the name of a key.
    ///
    /// Little Nightmares writes every one of its input prompts this way —
    /// `Press %0 to Start`, `Hold %0 and press %1 to climb` — and a translation
    /// that dropped or reordered one of those would put a prompt on screen
    /// naming the wrong button, or no button at all.
    ///
    /// Recorded as [`NawDharra::Mawdi`] with the written number as its index,
    /// because that is what it is: a positional argument, numbered from zero,
    /// in the same family as `{0}` and `%1$s`. Not [`NawDharra::Sura`] — a slot
    /// resolves to a key *name* on a keyboard and to an icon on a pad, so
    /// calling it an image would be right only half the time, and would refuse
    /// precomputed layout for every string that carries one.
    fn ramz_mieawi(&mut self, bida: usize) -> Natija<bool> {
        if !self.khiyarat.tashmal(LahjatNasq::UnrealRumuz) {
            return Ok(false);
        }
        let arqam = bida.saturating_add(1);
        let niha = nihayat_arqam(self.bayt, arqam);
        // No digits at all is a bare percent sign — `% of frame:` and `Exc Time
        // (%)` are both real strings out of this game — and digits followed by
        // `$` are printf's own positional syntax, which the caller has already
        // tried and which must not be halved into a slot and a stray `$`.
        if niha == arqam || self.bayt.get(niha) == Some(&b'$') {
            return Ok(false);
        }
        let tarteeb = self
            .khaam
            .get(arqam..niha)
            .and_then(|juz| juz.parse::<u32>().ok());
        self.dharra(
            bida,
            niha,
            NawDharra::Mawdi,
            tarteeb,
            self.khiyarat.ard_dharra_iftiradi,
        )?;
        Ok(true)
    }

    /// `$variable` and `${variable}`.
    ///
    /// # Errors
    ///
    /// [`SababNasq::MawdiTalif`] when `${` is opened and never closed.
    fn dulari(&mut self) -> Natija<bool> {
        let bida = self.i;
        let bayt = self.bayt;
        let mut i = bida.saturating_add(1);

        if bayt.get(i) == Some(&b'{') {
            i = i.saturating_add(1);
            if !bayt.get(i).copied().is_some_and(bidayat_ism_mawdi) {
                return Ok(false);
            }
            let hadd = bida.saturating_add(HADD_WASM).min(bayt.len());
            while i < hadd {
                match bayt.get(i) {
                    Some(&b'}') => {
                        let niha = i.saturating_add(1);
                        self.dharra(
                            bida,
                            niha,
                            NawDharra::Mutaghayyir,
                            None,
                            self.khiyarat.ard_dharra_iftiradi,
                        )?;
                        return Ok(true);
                    },
                    Some(&(b'\n' | b'\r' | b'{')) => return Ok(false),
                    Some(_) => i = i.saturating_add(1),
                    // Committed and never finished: the substitution throws.
                    None => return Err(khata(bida, SababNasq::MawdiTalif).into()),
                }
            }
            // A name longer than the scan limit is not a name.
            return Ok(false);
        }

        // A bare `$` takes a name, never a number: `$5.00` is a price and
        // `$gold` is a variable, and the difference is the first character.
        if !bayt
            .get(i)
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        {
            return Ok(false);
        }
        while bayt
            .get(i)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            i = i.saturating_add(1);
        }
        self.dharra(
            bida,
            i,
            NawDharra::Mutaghayyir,
            None,
            self.khiyarat.ard_dharra_iftiradi,
        )?;
        Ok(true)
    }

    /// A backslash: the `\\` escape, the C escapes, GameMaker's `\#`, and the
    /// RPG Maker escape codes.
    ///
    /// # Errors
    ///
    /// [`SababNasq::MawdiTalif`] when an RPG Maker code that requires an index
    /// does not have one — `\V` with no `[7]` after it. RPG Maker's own
    /// interpreter reads the following characters as the index and produces
    /// nonsense; there is no reading of that string which is correct.
    fn mailil(&mut self) -> Natija<bool> {
        let bida = self.i;
        let bayt = self.bayt;
        let Some(&harf) = bayt.get(bida.saturating_add(1)) else {
            return Ok(false);
        };
        let iftiradi = self.khiyarat.ard_dharra_iftiradi;

        if harf == b'\\' {
            // A doubled backslash stands for one printable backslash, so it
            // becomes that character rather than an atom: the player sees a
            // glyph, and the shaper and the line breaker should see one too.
            self.athar(bida, bida.saturating_add(2), "\\");
            return Ok(true);
        }

        if harf == b'#' && self.khiyarat.tashmal(LahjatNasq::GameMaker) {
            self.athar(bida, bida.saturating_add(2), "#");
            return Ok(true);
        }

        if self.khiyarat.tashmal(LahjatNasq::RpgMaker) {
            let musarrah = matches!(
                harf,
                b'V' | b'v' | b'N' | b'n' | b'P' | b'p' | b'C' | b'c' | b'I' | b'i'
            );
            if musarrah && bayt.get(bida.saturating_add(2)) == Some(&b'[') {
                let arqam = bida.saturating_add(3);
                let baad = nihayat_arqam(bayt, arqam);
                if baad == arqam || bayt.get(baad) != Some(&b']') {
                    return Err(khata(bida, SababNasq::MawdiTalif).into());
                }
                // The number inside the brackets identifies a variable, an
                // actor or a palette entry. It is not an argument position, so
                // it is deliberately not reported as one: comparing `\V[3]`
                // against `\V[4]` is a comparison of the raw text, and calling
                // three a positional index would make a reordering check
                // compare it against `{3}`.
                let (naw, ard) = match harf {
                    b'I' | b'i' => (NawDharra::Sura, iftiradi),
                    b'C' | b'c' => (NawDharra::Amr, 0.0),
                    _ => (NawDharra::Mutaghayyir, iftiradi),
                };
                self.dharra(bida, baad.saturating_add(1), naw, None, ard)?;
                return Ok(true);
            }
            // A bracketed code with no bracket. The lowercase C escapes below
            // are the one exception, because `\n` is a newline in RPG Maker
            // too and only `\N` takes an actor index.
            if musarrah && !matches!(harf, b'n' | b'r' | b't') {
                return Err(khata(bida, SababNasq::MawdiTalif).into());
            }
            let mufrad = match harf {
                b'G' | b'g' => Some((NawDharra::Mutaghayyir, iftiradi)),
                b'.' | b'|' | b'!' | b'>' | b'<' | b'^' => Some((NawDharra::Amr, 0.0)),
                _ => None,
            };
            if let Some((naw, ard)) = mufrad {
                self.dharra(bida, bida.saturating_add(2), naw, None, ard)?;
                return Ok(true);
            }
        }

        // The C escapes, which every dialect here inherits from its host
        // language. They stay atoms rather than becoming real control
        // characters: the two-character sequence is what the game stores, the
        // engine is what interprets it, and writing a real U+000A into the
        // clean text would make the reconstruction differ from the original in
        // the one place this module promises it will not.
        let (naw, ard) = match harf {
            b'n' | b'r' => (NawDharra::Amr, 0.0),
            b't' => (NawDharra::Amr, iftiradi),
            _ => return Ok(false),
        };
        self.dharra(bida, bida.saturating_add(2), naw, None, ard)?;
        Ok(true)
    }

    /// GameMaker's bare `#`, which its legacy `draw_text` reads as a line
    /// break.
    ///
    /// Only ever reached when the GameMaker dialect is enabled, because for
    /// every other engine a hash is ordinary punctuation and `Level #3` must
    /// survive intact.
    fn shabaka(&mut self) -> Natija<bool> {
        let bida = self.i;
        self.dharra(bida, bida.saturating_add(1), NawDharra::Amr, None, 0.0)?;
        Ok(true)
    }
}

/// Ren'Py's text tag vocabulary.
fn wasm_renpy_maruf(ism: &str) -> bool {
    matches!(
        ism,
        "b" | "i"
            | "u"
            | "s"
            | "plain"
            | "alt"
            | "noalt"
            | "color"
            | "outlinecolor"
            | "size"
            | "font"
            | "k"
            | "alpha"
            | "cps"
            | "a"
            | "art"
            | "rt"
            | "rb"
            | "vert"
            | "horiz"
            | "w"
            | "p"
            | "nw"
            | "fast"
            | "done"
            | "clear"
            | "space"
            | "vspace"
            | "image"
    )
}

/// Rebuilds the original raw string from clean text plus the extraction record.
///
/// Byte for byte, for every input [`istakhrij`] accepted. This is the half of
/// the contract the translation pipeline is built on: markup and placeholders
/// are lifted out before a translator or a machine translation provider ever
/// sees the string, so neither can damage a tag it was never shown, and they
/// are put back afterwards mechanically rather than by pattern-matching the
/// translated text. A reconstruction that were merely usually right would put a
/// visible `<color=#ff0000>` on somebody's screen a few thousand strings into a
/// patch, which is precisely the class of defect this design exists to make
/// impossible.
///
/// The walk is linear because the record is: traces are produced in the order
/// they were removed and their offsets are non-decreasing, so the clean text is
/// consumed once, from left to right, with each trace splicing its raw bytes
/// back in and skipping whatever stood in for them.
#[must_use]
pub fn aid_binaa(naqi: &NassNaqi) -> String {
    let mut khaam = String::with_capacity(qusize(naqi.asl_tul));
    let mut sabiq = 0_usize;
    for athar in &naqi.aathar {
        let mawqi = qusize(athar.mawqi);
        if let Some(bayn) = naqi.nass.get(sabiq..mawqi) {
            khaam.push_str(bayn);
        }
        khaam.push_str(&athar.khaam);
        sabiq = mawqi.saturating_add(qusize(athar.badeel));
    }
    if let Some(baqi) = naqi.nass.get(sabiq..) {
        khaam.push_str(baqi);
    }
    khaam
}

/// Just the atoms, in the order they appear, exactly as they were written.
///
/// This is the list a translation is validated against: every atom present,
/// none duplicated, none altered. Nothing else about the extraction is needed
/// for that check, and asking for nothing else keeps the check cheap enough to
/// run on every string in a project on every edit.
///
/// A string whose markup does not parse yields an empty list rather than a
/// failure, because a caller in this position is validating, not extracting,
/// and it has already learned about the malformed markup from [`istakhrij`] on
/// the string's own extraction pass.
#[must_use]
pub fn dharrat_nass(khaam: &str, khiyarat: &KhiyaratNasq) -> Vec<String> {
    istakhrij(khaam, khiyarat).map_or_else(
        |_| Vec::new(),
        |naqi| {
            naqi.dharrat
                .into_iter()
                .map(|dharra| dharra.khaam)
                .collect()
        },
    )
}

/// Confirms that every span really lands inside the clean text and on
/// character boundaries.
///
/// [`istakhrij`] produces spans that satisfy this by construction — every
/// offset it records is the length of the clean text at a moment when the
/// clean text was valid UTF-8. The check exists for the other direction: spans
/// that arrived from a patch file, from an adapter, or from a translation whose
/// markup was reapplied by something other than [`aid_binaa`]. A span boundary
/// inside a UTF-8 sequence would split a character, and therefore split a
/// cluster, and therefore put a diacritic in one style and its base letter in
/// another.
///
/// # Errors
///
/// [`KhataSaff::NitaqKharij`] when a span reaches past the end of the clean
/// text, and [`KhataSaff::HaddNitaqTalif`] when either of its boundaries falls
/// inside a character.
pub fn tahaqquq(naqi: &NassNaqi) -> Natija<()> {
    let tul = q32(naqi.nass.len());
    for nitaq in &naqi.nitaqat {
        let nihaya = nitaq.nihaya();
        if nitaq.bidaya > tul || nihaya > tul {
            return Err(KhataSaff::NitaqKharij {
                id: nitaq.id,
                bidaya: nitaq.bidaya,
                nihaya,
                tul,
            }
            .into());
        }
        for mawqi in [nitaq.bidaya, nihaya] {
            if !naqi.nass.is_char_boundary(qusize(mawqi)) {
                return Err(KhataSaff::HaddNitaqTalif {
                    id: nitaq.id,
                    mawqi,
                }
                .into());
            }
        }
    }
    Ok(())
}
