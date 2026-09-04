//! التشكيل — the diacritic guarantee.
//!
//! Every other module in this crate is a stage: text goes in one side and
//! something further along comes out the other. This one is not. Diacritics are
//! a property that has to hold *across* the stages, and the only way to hold it
//! is to state it once and enforce it at each point where a stage could break
//! it.
//!
//! The invariant, plainly:
//!
//! > A mark is a glyph whose advance is zero. It never takes part in line
//! > breaking or justification as if it were a letter. Its position on its base
//! > comes from the font's mark-to-base and mark-to-mark tables — `GPOS`
//! > `mark` and `mkmk` — and from nowhere else. Not from a table of offsets in
//! > this crate, not from a heuristic about where a fatha usually sits, not
//! > from the mark's own metrics.
//!
//! Each clause of that sentence is a real defect somewhere in the wild. A mark
//! with a non-zero advance produces text with a visible gap after every
//! vowelled letter. A mark that counts as a break opportunity lets a line break
//! between a letter and its own kasra. A mark that absorbs kashida elongation
//! makes a justified line stretch the vowels instead of the letters. A mark
//! positioned by a heuristic collides with a shadda the moment two land on one
//! base, which in vocalised Arabic is constantly.
//!
//! ## What each function guarantees
//!
//! - [`huwa_alama`] is the single definition of "is this character a mark",
//!   used by stripping, by segmentation, and by the shaper's cluster handling,
//!   so that the three cannot disagree about what a mark is.
//! - [`tabbiq_siyasa`] applies the patch's [`SiyasatTashkeel`] before shaping,
//!   because a mark that is going to be removed must be removed *before* it
//!   forms a cluster, not hidden afterwards.
//! - [`aadil_alamat`] enforces the zero advance and the width invariant on a
//!   shaped run, and reports the fonts that made it necessary.
//! - [`irtifa_alamat`] tells the line-height calculation how much extra
//!   vertical room a vocalised line actually needs, so that a line does not
//!   clip its own diacritics.
//! - [`tarteeb_alamat`] puts stacked marks into a canonical order so the same
//!   word renders identically no matter which order it was typed in.
//!
//! ## Marks are removed, never hidden
//!
//! [`SiyasatTashkeel::Hadhf`] deletes marks from the text before it reaches
//! `ittijah` or `wasl`. It does not shape them and then skip them at draw
//! time. The difference matters: a mark that reaches the shaper
//! joins its base into a cluster, changes which `ccmp` and `rlig` lookups fire,
//! and in some Naskh fonts selects a different contextual alternate for the
//! base itself. Text shaped with marks and drawn without them is not the same
//! text as text shaped without them, and only the second one is what the
//! translator asked for.

use core::cmp::Ordering;
use std::borrow::Cow;

use icu_properties::props::{CanonicalCombiningClass, GeneralCategory};
use icu_properties::{CodePointMapData, CodePointMapDataBorrowed};
use taarib_usus::khata::Natija;

use crate::khata::KhataSaff;
use crate::maqta::{HarfMashkul, MaqtaMashkul};
use crate::talab::SiyasatTashkeel;

// ---------------------------------------------------------------------------
// Recognising a mark
// ---------------------------------------------------------------------------

/// Whether a character is a combining mark.
///
/// Decided from the character's Unicode properties, not from a hand-written
/// list of codepoints. A list is wrong the day a new Unicode version adds a
/// Quranic annotation sign, and it is wrong quietly: the new mark simply stops
/// being recognised as one, gets an advance, and pushes the rest of the word
/// sideways. The rule here is two properties and nothing else — the character
/// is a non-spacing or enclosing mark, or it carries a non-zero canonical
/// combining class.
///
/// Spacing combining marks (`Mc`) are deliberately **not** included. They have
/// a positive advance by definition, and treating one as zero-advance would
/// collapse a Devanagari matra onto its consonant. No Arabic-script mark is
/// `Mc`, so excluding the category costs this product nothing and protects the
/// mixed text a patch will inevitably carry.
///
/// ## The ranges this has to get right, and why each one matters
///
/// - **U+064B–U+065F, U+0670.** The core تشكيل: the three short vowels, their
///   tanween forms, sukun, shadda, and the superscript alef that spells `هٰذا`
///   and `اللّٰه`. Everything a vocalised text is made of.
/// - **U+0610–U+061A.** The honorifics (`ﷺ`-class marks written above a name)
///   and the small high marks used in Quranic orthography.
/// - **U+06D6–U+06ED.** The Quranic annotation signs: the waqf marks, sajdah,
///   the small high seen, the small waw, yeh and noon. These are the reason a
///   mark-stripping policy has to be a policy and not a default — removing them
///   from a Quranic quotation removes meaning, not decoration.
/// - **U+08A0–U+08FF (Arabic Extended-A) and U+0870–U+089F (Extended-B).**
///   Marks for Quranic orthography and for the African and South Asian
///   languages written in Arabic script.
/// - **U+0654, U+0655, U+0656, U+0657, U+065E.** Hamza above and below and the
///   inverted damma and fatha, which is where Persian and Urdu differ from
///   Arabic in practice.
/// - **U+0300–U+036F.** The general combining diacriticals, which arrive
///   through the Latin runs every mixed patch carries, and which must obey the
///   same zero-advance rule as the Arabic ones or a Latin word with an accent
///   measures wider than it draws.
/// - **U+FE00–U+FE0F.** Variation selectors. They are marks by category and
///   must never advance the pen; a font that gives one a width produces a
///   mysterious gap that no amount of looking at the Arabic will explain.
#[must_use]
pub fn huwa_alama(harf: char) -> bool {
    alama_bi_jadwal(
        CodePointMapData::<GeneralCategory>::new(),
        CodePointMapData::<CanonicalCombiningClass>::new(),
        harf,
    )
}

/// The rule [`huwa_alama`] applies, with the property tables handed in.
///
/// Both tables resolve to a static trie and looking one up costs nothing, but a
/// loop over a whole string looks them up once per character otherwise. This
/// exists so the hot path can hoist them and still ask exactly the same
/// question the public predicate asks — there is one definition of "mark" in
/// this crate and it is this function.
fn alama_bi_jadwal(
    fiat: CodePointMapDataBorrowed<'_, GeneralCategory>,
    tabaqat: CodePointMapDataBorrowed<'_, CanonicalCombiningClass>,
    harf: char,
) -> bool {
    matches!(
        fiat.get(harf),
        GeneralCategory::NonspacingMark | GeneralCategory::EnclosingMark
    ) || tabaqat.get(harf) != CanonicalCombiningClass::NotReordered
}

// ---------------------------------------------------------------------------
// The policy
// ---------------------------------------------------------------------------

/// Applies a patch's diacritic policy to text, before shaping.
///
/// `hiwar` says whether this particular string is dialogue, which is the one
/// thing [`SiyasatTashkeel::IbqaFilHiwar`] keys on: a game may reasonably
/// vocalise its dialogue, where the marks are meaning and there is room for
/// them, and strip its menus, where they cost width and legibility at eleven
/// pixels.
///
/// Returns [`Cow::Borrowed`] whenever nothing was removed — which is the
/// overwhelmingly common case, since most game text carries no marks at all and
/// the two keeping policies never remove anything. This function runs on every
/// string of a patch, on every layout call that misses the cache, and inside a
/// game's frame; allocating a copy of text it did not change would be a cost
/// paid thousands of times a second for nothing.
#[must_use]
pub fn tabbiq_siyasa(nass: &str, siyasa: SiyasatTashkeel, hiwar: bool) -> Cow<'_, str> {
    let yahdhif = match siyasa {
        SiyasatTashkeel::Ibqa => false,
        SiyasatTashkeel::Hadhf => true,
        SiyasatTashkeel::IbqaFilHiwar => !hiwar,
    };
    if !yahdhif {
        return Cow::Borrowed(nass);
    }
    ihdhif_alamat(nass)
}

/// Removes every combining mark from text, borrowing when there are none.
fn ihdhif_alamat(nass: &str) -> Cow<'_, str> {
    let fiat = CodePointMapData::<GeneralCategory>::new();
    let tabaqat = CodePointMapData::<CanonicalCombiningClass>::new();
    let alama = |harf: char| -> bool { alama_bi_jadwal(fiat, tabaqat, harf) };

    // Find the first mark before allocating anything. Scanning twice over text
    // that has no marks is cheaper than allocating once over text that does
    // not need it, and the second scan never happens for the common case.
    let Some(awwal) = nass.char_indices().find(|(_, harf)| alama(*harf)).map(|(mawqi, _)| mawqi)
    else {
        return Cow::Borrowed(nass);
    };

    let mut munaqqa = String::with_capacity(nass.len());
    if let Some(sabiq) = nass.get(..awwal) {
        munaqqa.push_str(sabiq);
    }
    if let Some(baqi) = nass.get(awwal..) {
        munaqqa.extend(baqi.chars().filter(|harf| !alama(*harf)));
    }
    Cow::Owned(munaqqa)
}

// ---------------------------------------------------------------------------
// Enforcing the invariant on a shaped run
// ---------------------------------------------------------------------------

/// What normalising a run's marks had to change, and in which font.
///
/// A font that gives a mark an advance is not merely producing one wrong gap.
/// It is a font whose mark-attachment tables were not built for the marks it
/// claims to cover, and it will also stack a shadda and a kasra on top of each
/// other the first time both land on one letter. That is why the failure is
/// counted and attributed rather than silently repaired: the diagnostics have
/// to be able to name the font, so that a translator staring at collided
/// diacritics is told which font to replace instead of being told nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaqreerAlamat {
    /// Index into the layout's font chain of the font that produced this run.
    pub khatt: u8,
    /// How many glyphs in the run are combining marks.
    pub adad_alamat: u32,
    /// How many of those the font gave a non-zero horizontal advance, which had
    /// to be normalised to zero.
    pub taqaddum_musahhah: u32,
    /// How many were given a non-zero vertical advance, which in horizontal
    /// layout is meaningless and would displace everything after them.
    pub taqaddum_rasi_musahhah: u32,
    /// How many clusters had their stacked marks reordered into canonical
    /// attachment order.
    pub muaad_tarteebuha: u32,
}

impl TaqreerAlamat {
    /// Whether the font behaved: every mark already had a zero advance and
    /// every stack was already in canonical order.
    #[must_use]
    pub const fn salim(&self) -> bool {
        self.taqaddum_musahhah == 0
            && self.taqaddum_rasi_musahhah == 0
            && self.muaad_tarteebuha == 0
    }

    /// Whether this run carries any marks at all.
    #[must_use]
    pub const fn mushakkal(&self) -> bool {
        self.adad_alamat > 0
    }
}

/// Enforces the diacritic invariant on a shaped run.
///
/// Three things are true of the run afterwards, and none of them is true of
/// every font's raw output:
///
/// 1. Every mark's advance, horizontal and vertical, is exactly zero.
/// 2. The run's measured width is the sum of the *bases* alone, so a vocalised
///    word measures the same as the same word unvocalised — which is what makes
///    a line break in the same place whether or not the translator typed the
///    vowels.
/// 3. Every mark's offsets are untouched. They are what `GPOS` produced and
///    this function does not second-guess them. Repairing a bad advance by
///    quietly shifting the offsets that follow it would hide the broken font
///    instead of reporting it, and would move marks the font placed correctly.
///
/// # Errors
///
/// Returns [`KhataSaff::TashkeelFashil`] when a glyph's position is not a
/// finite number. That happens when a variable font's `GPOS` deltas overflow or
/// its anchor table is corrupt, and there is no repair for it: every downstream
/// measurement built on that value would be `NaN`, the line would have no
/// width, and the failure would surface as blank text a hundred lines later.
/// The error names the run and tells the user to choose another font, which is
/// the only thing that actually fixes it.
pub fn aadil_alamat(maqta: &mut MaqtaMashkul) -> Natija<()> {
    let _ = aadil_alamat_mufassal(maqta)?;
    Ok(())
}

/// [`aadil_alamat`], returning what it had to change.
///
/// The reporting form. `aadil_alamat` is the one every stage calls; this one is
/// what the diagnostics and the patch compiler's font report call, because a
/// count of normalised advances per font is the difference between "diacritics
/// look wrong in this patch" and "this font's mark table is incomplete".
///
/// # Errors
///
/// As [`aadil_alamat`].
pub fn aadil_alamat_mufassal(maqta: &mut MaqtaMashkul) -> Natija<TaqreerAlamat> {
    let mut taqreer = TaqreerAlamat { khatt: maqta.asl.khatt, ..TaqreerAlamat::default() };
    let tul = maqta.asl.tul();
    let kitaba = maqta.asl.kitaba.0;

    for harf in &mut maqta.huruf {
        if !harf.izaha_s.is_finite()
            || !harf.izaha_a.is_finite()
            || !harf.taqaddum_s.is_finite()
            || !harf.taqaddum_a.is_finite()
        {
            return Err(KhataSaff::TashkeelFashil { tul, script: kitaba }.into());
        }
        if !harf.alama {
            continue;
        }
        taqreer.adad_alamat = taqreer.adad_alamat.saturating_add(1);
        if harf.taqaddum_s.abs() > f32::EPSILON {
            harf.taqaddum_s = 0.0;
            taqreer.taqaddum_musahhah = taqreer.taqaddum_musahhah.saturating_add(1);
        } else {
            // Normalised even when it is already effectively zero, so that a
            // negative zero or a denormal from a variable font's delta cannot
            // survive into a width sum.
            harf.taqaddum_s = 0.0;
        }
        if harf.taqaddum_a.abs() > f32::EPSILON {
            harf.taqaddum_a = 0.0;
            taqreer.taqaddum_rasi_musahhah = taqreer.taqaddum_rasi_musahhah.saturating_add(1);
        } else {
            harf.taqaddum_a = 0.0;
        }
    }

    taqreer.muaad_tarteebuha = rattib_wa_uddd(&mut maqta.huruf);

    // Recomputed rather than adjusted: a width that is patched by subtraction
    // drifts, and a run's width is the one number every later stage trusts.
    maqta.qis();

    Ok(taqreer)
}

// ---------------------------------------------------------------------------
// Vertical room
// ---------------------------------------------------------------------------

/// How far a run's marks reach beyond its base glyphs, above and below.
///
/// Returns `(above, below)`, both non-negative, in pixels. `above` is how far
/// the highest mark's attachment rises past the highest base attachment;
/// `below` is the mirror of it. A run with no marks returns `(0.0, 0.0)`.
///
/// This is what the line-height calculation needs. A line of vocalised Arabic
/// is genuinely taller than the same line unvocalised — a shadda with a damma
/// stacked on it reaches most of an em above the letter it sits on — and a
/// layout that sizes its line boxes from the font's ascent alone clips the
/// tops of its own diacritics. The caller takes the maximum of the font's own
/// ascent and the base extent plus this, rather than either alone.
///
/// What is measured here is the *attachment displacement* that `mark` and
/// `mkmk` produced, not the mark's own ink height: a shaped run carries the
/// offsets the font resolved, and the ink of each glyph is already folded into
/// the run's own [`MaqtaMashkul::suud`] and [`MaqtaMashkul::hubut`] by the
/// measuring stage. Stacking is captured correctly regardless, because a
/// second mark attached to a first through `mkmk` carries the sum of both
/// displacements in its own offset — which is precisely the number a stacked
/// line needs and precisely the number a per-glyph ink box would miss.
#[must_use]
pub fn irtifa_alamat(maqta: &MaqtaMashkul) -> (f32, f32) {
    let mut asas_ala = 0.0f32;
    let mut asas_asfal = 0.0f32;
    let mut alama_ala = f32::NEG_INFINITY;
    let mut alama_asfal = f32::INFINITY;
    let mut wujidat = false;

    for harf in &maqta.huruf {
        if !harf.izaha_a.is_finite() {
            continue;
        }
        if harf.alama {
            wujidat = true;
            alama_ala = alama_ala.max(harf.izaha_a);
            alama_asfal = alama_asfal.min(harf.izaha_a);
        } else {
            asas_ala = asas_ala.max(harf.izaha_a);
            asas_asfal = asas_asfal.min(harf.izaha_a);
        }
    }

    if !wujidat {
        return (0.0, 0.0);
    }
    let fawq = (alama_ala - asas_ala).max(0.0);
    let taht = (asas_asfal - alama_asfal).max(0.0);
    (fawq, taht)
}

// ---------------------------------------------------------------------------
// Canonical attachment order
// ---------------------------------------------------------------------------

/// Puts stacked marks into canonical attachment order within each cluster.
///
/// Two people typing the same vocalised word can produce two different
/// codepoint sequences — shadda then kasra, or kasra then shadda — and a
/// renderer that draws them in the order it received them produces two
/// different images of one word. Worse, it does so inconsistently: the same
/// string, copied between two source files by two translators, renders
/// differently in the same build. Canonical ordering removes the question.
///
/// The order is by distance from the baseline, nearest first, so a stack is
/// composited from the inside out and the outermost mark is the one drawn on
/// top. Ties break on the horizontal offset and then on the glyph identifier,
/// which makes the order total: the result does not depend on the input order
/// at all, which is the entire point. Unicode's own canonical ordering by
/// combining class has already run, before shaping, on the characters; this is
/// its counterpart on the glyphs, where the offsets rather than the classes are
/// what is known.
///
/// **Only runs of consecutive marks are reordered, and never across a base.**
/// A mark has a zero advance, so permuting marks among themselves moves
/// nothing: every glyph in the run keeps the pen position it was positioned
/// against. Moving a mark past a base would shift it by that base's advance and
/// tear it off the letter it belongs to — which is why this function reorders
/// within mark groups only, and why calling it after [`aadil_alamat`] (which is
/// what guarantees the zero advance) is the correct order.
pub fn tarteeb_alamat(huruf: &mut [HarfMashkul]) {
    let _ = rattib_wa_uddd(huruf);
}

/// [`tarteeb_alamat`], returning how many mark groups were not already in
/// canonical order.
fn rattib_wa_uddd(huruf: &mut [HarfMashkul]) -> u32 {
    let mut muaad: u32 = 0;
    let mut bidaya = 0usize;
    let tul = huruf.len();

    while bidaya < tul {
        let Some(awwal) = huruf.get(bidaya) else { break };
        if !awwal.alama {
            bidaya = bidaya.saturating_add(1);
            continue;
        }
        let anqud = awwal.anqud;
        let mut nihaya = bidaya.saturating_add(1);
        while huruf.get(nihaya).is_some_and(|harf| harf.alama && harf.anqud == anqud) {
            nihaya = nihaya.saturating_add(1);
        }

        if nihaya.saturating_sub(bidaya) > 1
            && let Some(shariha) = huruf.get_mut(bidaya..nihaya)
        {
            if !shariha
                .is_sorted_by(|sabiq, lahiq| rutbat_alama(sabiq, lahiq) != Ordering::Greater)
            {
                muaad = muaad.saturating_add(1);
            }
            shariha.sort_by(rutbat_alama);
        }
        bidaya = nihaya;
    }
    muaad
}

/// The canonical order of two marks on one base.
///
/// `total_cmp` rather than `partial_cmp`: it is a total order over every `f32`
/// including the ones a broken font can produce, so the sort is deterministic
/// even when the offsets are not sensible. A comparator that can answer
/// "neither" is a comparator that can produce a different permutation on a
/// different day.
fn rutbat_alama(sabiq: &HarfMashkul, lahiq: &HarfMashkul) -> Ordering {
    sabiq
        .izaha_a
        .abs()
        .total_cmp(&lahiq.izaha_a.abs())
        .then_with(|| sabiq.izaha_s.abs().total_cmp(&lahiq.izaha_s.abs()))
        .then_with(|| sabiq.muarrif.cmp(&lahiq.muarrif))
}
