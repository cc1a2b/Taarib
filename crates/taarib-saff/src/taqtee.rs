//! التقطيع — where a line may break, where a cluster begins, and where a word
//! ends, all computed on the logical text.
//!
//! Every boundary in this module is found before anything is reordered, and that
//! ordering is not a preference. A break opportunity is a property of the
//! *logical* sequence of characters: UAX #14 answers "may a line end between
//! these two characters" by looking at the pair in the order the author wrote
//! them. Run that question over visually reordered text and it is being asked
//! about a string that does not exist in that order — the character before a
//! given position in visual order may be from a different word, a different
//! direction, or the far end of the sentence. The answer would be well-formed
//! and meaningless.
//!
//! ## A break changes joining, which is why the next stage reshapes
//!
//! Breaking a line is not cutting a ribbon. In Arabic the last letter before the
//! break loses the neighbour it was joined to, so it changes form: a medial `ـهـ`
//! becomes a final `ـه`, and its width changes with it. The same happens to the
//! first letter after the break, which becomes initial instead of medial. A
//! measurement taken from the unbroken run and then split at an offset is
//! therefore wrong on both sides of every break — narrower or wider than the two
//! real lines, by an amount that grows with the number of breaks.
//!
//! That is why the measurement stage that follows this one, `qiyas`, reshapes the
//! head and the tail after it takes a break rather than reusing the run it
//! already shaped — and why it measures from real shaped advances rather than
//! from anything derived before the break existed. This module's job is only
//! to say where the breaks are allowed to be; the cost of taking one is paid
//! afterwards, honestly.
//!
//! ## Clusters are what a caret moves by
//!
//! [`hudud_anaqid`] gives the grapheme cluster boundaries, and a grapheme cluster
//! is the unit a person perceives as one character: a base letter with its
//! diacritics, a Hangul syllable, an emoji with its modifiers. A caret that moves
//! by bytes lands inside a UTF-8 sequence; a caret that moves by `char` lands
//! between a letter and its own tashkeel, deleting a fatha and leaving the letter
//! it belonged to. Both are the same bug seen from two heights.
//!
//! A typewriter effect reveals text by these same boundaries, and in Arabic that
//! costs more than it looks. Revealing one cluster at a time means the visible
//! prefix changes shape at every step: the letter that was final becomes medial
//! the moment the next letter appears, so the prefix has to be *reshaped* at each
//! frame rather than drawn one glyph longer. Nothing about that is optional — a
//! prefix drawn by appending glyphs shows disconnected letterforms that snap
//! together as the animation runs. It is affordable only because the layout cache
//! keys on the text and returns a finished layout for a prefix it has already
//! seen, so the whole animation shapes each prefix once and then replays it.

use icu_locale_core::{LanguageIdentifier, langid};
use icu_segmenter::options::{LineBreakOptions, LineBreakStrictness, WordBreakInvariantOptions};
use icu_segmenter::{GraphemeClusterSegmenter, LineSegmenter, WordSegmenter};

use crate::maqta::FursatQat;
use crate::talab::LughaNass;

/// Byte offsets are `u32` everywhere in this engine because they cross the C ABI
/// and are stored in patches. A string long enough to truncate cannot reach here
/// — the ABI refuses one — and saturating is the only answer that does not panic.
fn ila32(mawqi: usize) -> u32 {
    u32::try_from(mawqi).unwrap_or(u32::MAX)
}

/// Every place the text may be broken, in logical order.
///
/// UAX #14 through `icu_segmenter`, tailored by the declared language. The
/// segmenter reports a boundary at offset zero and one at the end of the text;
/// the first is dropped, because "you may break before the first character" is
/// not an opportunity anyone can take, and the last is kept, because a paragraph
/// that ends in a newline really does have one more line after it.
///
/// A boundary is marked [`FursatQat::ilzami`] when the character before it forces
/// a break rather than merely permitting one: LF, CR, VT, FF, NEL, LINE
/// SEPARATOR and PARAGRAPH SEPARATOR. A CR immediately followed by an LF is one
/// break after the LF, never two, which is why a Windows line ending does not
/// produce a blank line in the middle of a patched dialogue box.
///
/// An empty string has no opportunities and allocates nothing to say so.
#[must_use]
pub fn furas_qat(nass: &str, lugha: LughaNass) -> Vec<FursatQat> {
    if nass.is_empty() {
        return Vec::new();
    }

    let muarrif = muarrif_lugha(lugha);
    let muqattie = LineSegmenter::new_auto(khiyarat_qat(&muarrif));

    let mut furas: Vec<FursatQat> = Vec::new();
    for hadd in muqattie.segment_str(nass) {
        if hadd == 0 {
            continue;
        }
        furas.push(FursatQat {
            mawqi: ila32(hadd),
            ilzami: ilzami_qabl(nass, hadd),
        });
    }
    furas
}

/// The grapheme cluster boundaries of the text, in byte offsets, including the
/// offsets at both ends.
///
/// This is what a caret steps over, what a selection snaps to, and what a
/// typewriter effect reveals by — see the module documentation for why revealing
/// Arabic by clusters means reshaping the visible prefix at every step.
///
/// An empty string has no clusters and therefore no boundaries, and allocates
/// nothing to say so. A caret in an empty field has exactly one position, zero,
/// and needs no table to find it.
#[must_use]
pub fn hudud_anaqid(nass: &str) -> Vec<u32> {
    if nass.is_empty() {
        return Vec::new();
    }
    GraphemeClusterSegmenter::new()
        .segment_str(nass)
        .map(ila32)
        .collect()
}

/// The word boundaries of the text, in byte offsets, including the offsets at
/// both ends.
///
/// UAX #29, which is what double-click selection, word-wise caret movement and
/// the workspace's word counts all agree on. Locale-independent options are used
/// deliberately: word breaking is tailored per locale only for the scripts that
/// need a dictionary or a model to find word edges at all — Thai, Lao, Khmer,
/// Japanese, Chinese — and none of them is a script this engine's language set
/// declares. Arabic, Persian, Urdu and Latin all break words at the same places.
///
/// An empty string allocates nothing.
#[must_use]
pub fn hudud_kalimat(nass: &str) -> Vec<u32> {
    if nass.is_empty() {
        return Vec::new();
    }
    WordSegmenter::new_auto(WordBreakInvariantOptions::default())
        .segment_str(nass)
        .map(ila32)
        .collect()
}

/// The grapheme cluster boundary before a byte offset — where the caret goes when
/// it moves back one position.
///
/// An offset that falls inside a cluster snaps to the start of that cluster
/// rather than stepping over it, which is what makes a caret recover from a
/// position some other code computed by bytes. Offset zero, and any offset in an
/// empty string, answer zero.
#[must_use]
pub fn anqud_sabiq(nass: &str, mawqi: u32) -> u32 {
    if nass.is_empty() || mawqi == 0 {
        return 0;
    }
    let hadd = mawqi.min(ila32(nass.len()));
    let mut sabiq = 0u32;
    for hudud in GraphemeClusterSegmenter::new().segment_str(nass) {
        let hudud = ila32(hudud);
        if hudud >= hadd {
            break;
        }
        sabiq = hudud;
    }
    sabiq
}

/// The grapheme cluster boundary after a byte offset — where the caret goes when
/// it moves forward one position.
///
/// An offset inside a cluster moves to the end of that cluster, so a caret never
/// stops between a letter and its own diacritic. An offset at or past the end of
/// the text answers with the end of the text.
#[must_use]
pub fn anqud_tali(nass: &str, mawqi: u32) -> u32 {
    let tul = ila32(nass.len());
    if nass.is_empty() || mawqi >= tul {
        return tul;
    }
    for hudud in GraphemeClusterSegmenter::new().segment_str(nass) {
        let hudud = ila32(hudud);
        if hudud > mawqi {
            return hudud;
        }
    }
    tul
}

/// The breaks the text itself demands, in byte offsets, with no opportunities
/// among them.
///
/// Computed from the characters rather than from the segmenter, because these are
/// the breaks a caller needs before it has decided anything about width: a
/// single-line field still has to know that the string it was handed contains a
/// newline, and a paragraph-level caller splits on these before it ever asks
/// where a line could wrap.
///
/// The offset reported is the one *after* the break character, so the break
/// character belongs to the line it ends. A CR LF pair is one break, after the
/// LF.
///
/// An empty string allocates nothing.
#[must_use]
pub fn qat_idtirari(nass: &str) -> Vec<u32> {
    if nass.is_empty() {
        return Vec::new();
    }

    let mut mawadi: Vec<u32> = Vec::new();
    let mut huruf = nass.char_indices().peekable();
    while let Some((izaha, harf)) = huruf.next() {
        let mut nihaya = izaha + harf.len_utf8();
        match harf {
            '\u{000D}' => {
                if huruf.peek().is_some_and(|(_, tali)| *tali == '\u{000A}') {
                    let _ = huruf.next();
                    nihaya += 1;
                }
            },
            '\u{000A}' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => {},
            _ => continue,
        }
        mawadi.push(ila32(nihaya));
    }
    mawadi
}

/// The line-breaking options this engine uses, carrying the content language so
/// the tailored rules apply.
///
/// Strictness is set to normal rather than left at the crate's default. The
/// locale only opens the additional break opportunities it defines when
/// strictness is normal or loose, so a stricter setting would take the language
/// tag and then ignore what it is for. Normal is also what a browser does with
/// `line-break: auto`, which is the behaviour translators are calibrated against
/// when they judge whether a string will fit.
///
/// The options are built by assignment rather than as a literal because
/// `LineBreakOptions` is non-exhaustive: no crate but its own can name all of its
/// fields at once.
const fn khiyarat_qat(muarrif: &LanguageIdentifier) -> LineBreakOptions<'_> {
    let mut khiyarat = LineBreakOptions::default();
    khiyarat.strictness = Some(LineBreakStrictness::Normal);
    khiyarat.content_locale = Some(muarrif);
    khiyarat
}

/// The language tag segmentation is tailored by.
///
/// Built from a literal at compile time rather than parsed, so there is no
/// failure path for a tag this crate wrote itself. Text whose language was never
/// declared is tailored as Arabic, because that is what this product exists to
/// lay out and because the Arabic tailoring differs from the untailored rules
/// only in ways that are correct for Latin text too.
const fn muarrif_lugha(lugha: LughaNass) -> LanguageIdentifier {
    match lugha {
        LughaNass::Farisi => langid!("fa"),
        LughaNass::Urdu => langid!("ur"),
        LughaNass::Latini => langid!("en"),
        LughaNass::Arabi | LughaNass::Tilqai => langid!("ar"),
    }
}

/// Whether the character ending at a byte offset forces the break there.
fn ilzami_qabl(nass: &str, hadd: usize) -> bool {
    let Some(sabiq) = nass.get(..hadd).and_then(|qabl| qabl.chars().next_back()) else {
        return false;
    };
    match sabiq {
        '\u{000A}' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}' => true,
        // A carriage return forces a break only when no line feed follows it. The
        // pair is one break, and the segmenter does not offer a boundary between
        // the two halves of it, but a caller that computed this offset some other
        // way still gets the right answer here.
        '\u{000D}' => !nass
            .get(hadd..)
            .is_some_and(|baad| baad.starts_with('\u{000A}')),
        _ => false,
    }
}
