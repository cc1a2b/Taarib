//! القياس — measurement, reflow, and the assembly of a finished layout.
//!
//! This is the stage that decides where every line breaks and where every glyph
//! sits. Everything before it produced information; this is where the
//! information becomes a page.
//!
//! ## Every width here is a real shaped width
//!
//! Nothing in this module estimates. A line's width is the sum of the advances
//! HarfRust returned for the glyphs that line actually contains — not a
//! character count times an average, not a monospace assumption, not a cached
//! guess from a similar string. That is not a performance choice, it is the
//! reason the overflow report can be believed: the patch compiler tells a
//! translator that a string is eleven pixels too wide for a button, and the
//! translator rewrites the sentence on the strength of it. A measurement that
//! was estimated would send them chasing a defect that is not there, or hide one
//! that is.
//!
//! Arabic makes the point sharply. The same letter is four different shapes with
//! four different advances depending on its neighbours, a `lam` and an `alef`
//! together are one glyph narrower than either, and a font's `kern` and `curs`
//! move things again. There is no function from characters to width in this
//! script. There is only shaping.
//!
//! ## Breaking a run changes how it joins, so the break reshapes it
//!
//! When a candidate line overflows, the line is broken at the last opportunity
//! `taqtee` offered at or before the overflow point — and then the runs the
//! break touched are **shaped again**. This is not an optimisation that could be
//! skipped for speed.
//!
//! A letter's shape depends on what is next to it. The last letter of the head
//! was medial when the whole word was on one line and is final once the word is
//! cut; the first letter of the tail was medial and is now initial. Those are
//! different glyphs with different advances and different ink. A layout that
//! measured the head using the medial forms it will not draw has measured
//! something that does not exist — and it will be wrong in the direction that
//! matters, because final forms carry the long tails that overflow a box.
//!
//! Reshaping is done against a *view* of the text in which the other side of the
//! break is not adjacent, because handing the shaper the whole string would let
//! it join across the break and hand back exactly the glyphs the break was
//! supposed to change. See [`nisyan_alqat`] for how that view is built without
//! disturbing a single byte offset.
//!
//! ## Measurement and layout are one code path
//!
//! [`qis`] answers the metrics query the workspace shows beside a string as it
//! is typed and the patch compiler runs over every string it compiles. It is the
//! same pipeline as [`rattib_sutur`] — the same validation, the same breaking,
//! the same reshaping, the same justification, the same line metrics — stopping
//! one step short, before glyphs are given positions.
//!
//! Both call [`ibni`], which produces the line structure; `rattib_sutur` then
//! walks it into positioned glyphs and `qis` reads the extents off it. A
//! measurement path that was a *separate implementation* would agree with the
//! layout path on the day it was written and drift from it afterwards, and the
//! first symptom would be a preview that promised a string fits and a game that
//! shows it clipped. Boundary 4 of this product says the preview never lies; one
//! shared implementation is how that is kept true rather than merely intended.

use core::ops::Range;
use std::collections::VecDeque;

use taarib_usus::khata::{Khata, Natija};

use crate::ittijah::{TahleelIttijah, rattib_basariyan};
use crate::khata::KhataSaff;
use crate::maqta::{FursatQat, HarfMashkul, MaqtaMantiqi, MaqtaMashkul};
use crate::natija::{Harf, SatrMansuq, TakhtitNass, TaqreerTajawuz};
use crate::talab::{
    Dharra, Ittijah, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, NitaqUslub, SiyasatTajawuz,
    TalabTakhtit, Uslub,
};
use crate::wasl::{MakhzanTashkeel, shakkil_maqati_bi_asalib};

/// A hundredth of a pixel.
///
/// Line widths are sums of dozens of `f32` advances, so no two of them are ever
/// compared for equality. "It fits" means it is not wider than the room by more
/// than this, and at a hundredth of a pixel nothing that draws can tell.
const DIQQA: f32 = 0.01;

/// The smallest pixel size the rasterizer will produce a usable glyph at.
///
/// Below one pixel an outline has no coverage to speak of and the atlas fills
/// with blank cells. Refusing here turns an invisible line of text into a named
/// failure at the moment the size is chosen.
const HAJM_ADNA: f32 = 1.0;

/// The largest pixel size a single glyph may be rasterized at.
///
/// One glyph at this size is already a four-megapixel coverage bitmap. A request
/// beyond it is a unit mistake — points read as pixels, or a scale factor
/// applied twice — and answering it would exhaust memory inside somebody else's
/// game process rather than in Taarib.
const HAJM_AQSA: f32 = 2048.0;

/// How close the shrink-to-fit search gets before it stops halving.
///
/// A quarter of a pixel is below what any rasterizer resolves at interface
/// sizes, so continuing past it costs a full layout per step and buys a
/// difference nothing can display.
const DIQQA_HAJM: f32 = 0.25;

/// The hard bound on shrink-to-fit iterations.
///
/// The interval halves every step, so this is never the binding constraint —
/// [`DIQQA_HAJM`] is. It exists so that a font whose metrics behave
/// non-monotonically under scaling cannot turn a search into a spin.
const ADAD_TAQLIS: u32 = 16;

/// HORIZONTAL ELLIPSIS, the mark truncation leaves behind.
///
/// One character, not three periods. Three periods are three glyphs the font
/// will kern, they break across a line, and in Arabic they read as a sequence of
/// dots rather than as the elision mark.
const HADHF: char = '\u{2026}';

/// The extents of a piece of text, without positioning a single glyph.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct QiyasNass {
    /// The width of the widest line, in pixels.
    pub ard: f32,
    /// The height of every line box together, in pixels.
    pub irtifa: f32,
    /// How far the first line's tallest content rises above its baseline.
    pub suud: f32,
    /// How far the last line's deepest content falls below its baseline.
    pub hubut: f32,
    /// How many lines the text needed.
    pub adad_sutur: u32,
}

/// The width of one shaped run, in pixels.
///
/// Re-derived from the glyph advances rather than read out of
/// [`MaqtaMashkul::ard`], because that field is a cache and this function is
/// what the cache is checked against. Justification replaces a run's glyphs
/// wholesale, and a caller that trusted a stale `ard` would place the next run
/// on top of this one.
///
/// An atom has no glyphs; its width is the one the caller gave it, and that is
/// what comes back.
///
/// This is the *shaped* width. It does not include the letter spacing or word
/// spacing a request may add, because those are properties of the request rather
/// than of the run, and a caller asking what a run measures is asking about the
/// font.
#[must_use]
pub fn ard_maqta(maqta: &MaqtaMashkul) -> f32 {
    if maqta.asl.dharra.is_some() || maqta.huruf.is_empty() {
        return maqta.ard;
    }
    maqta.huruf.iter().map(|harf| harf.taqaddum_s).sum()
}

/// The width of a sequence of shaped runs, in pixels.
///
/// The sum of [`ard_maqta`] over the runs. Order does not matter: a run's width
/// is the same wherever reordering puts it, which is why a line can be measured
/// before it is reordered and measured again afterwards to the same number.
#[must_use]
pub fn ard_maqati(maqati: &[MaqtaMashkul]) -> f32 {
    maqati.iter().map(ard_maqta).sum()
}

/// The style in force at a byte offset, and the identifier of the span that set
/// it.
///
/// Resolved from the *cluster's* offset against the request's span list, never
/// from [`crate::maqta::MaqtaMantiqi::uslub`]. That field names the span at the
/// run's first byte, and a run deliberately spans several: a colour change does
/// not split a run, because a red word inside a black sentence still joins to
/// its neighbours and splitting the run there would sever the join. Reading the
/// run's field instead would give every glyph in the run the first byte's colour
/// and the red word would come out black — the exact defect that not splitting
/// the run was meant to avoid.
///
/// The merge is the same one the run splitter performs: outermost span first,
/// each span inside it overriding what it set, equal lengths in declaration
/// order so the span written last wins. Any other order would let an outer
/// `<size>` beat an inner one.
fn uslub_ind(nitaqat: &[NitaqUslub], mawqi: u32) -> (u16, Uslub) {
    let mut mughattiya: Vec<(u32, usize)> = Vec::new();
    for (fahras, nitaq) in nitaqat.iter().enumerate() {
        if nitaq.tul > 0 && nitaq.yashmal(mawqi) {
            mughattiya.push((nitaq.tul, fahras));
        }
    }
    mughattiya.sort_unstable_by(|awwal, thani| thani.0.cmp(&awwal.0).then(awwal.1.cmp(&thani.1)));

    let mut uslub = Uslub::default();
    let mut muarrif = 0_u16;
    for (_, fahras) in &mughattiya {
        if let Some(nitaq) = nitaqat.get(*fahras) {
            uslub = dam_uslub(uslub, nitaq.uslub);
            muarrif = nitaq.id;
        }
    }
    (muarrif, uslub)
}

/// Lays one style over another: anything the inner span sets replaces the outer
/// value, anything it leaves unset inherits.
fn dam_uslub(asas: Uslub, fawq: Uslub) -> Uslub {
    Uslub {
        khatt: fawq.khatt.or(asas.khatt),
        wazn: fawq.wazn.or(asas.wazn),
        maail: fawq.maail || asas.maail,
        hajm: fawq.hajm.or(asas.hajm),
        lawn: fawq.lawn.or(asas.lawn),
        tabaud: fawq.tabaud.or(asas.tabaud),
        izaha: fawq.izaha.or(asas.izaha),
        dharra: fawq.dharra.or(asas.dharra),
    }
}

/// The letter spacing in force at a byte offset.
///
/// A span's own value overrides the request's. The common case — no spans at all
/// — skips the resolution entirely, because this runs once per glyph on every
/// measurement.
fn tabaud_harf(nitaqat: &[NitaqUslub], mawqi: u32, khiyarat: &KhiyaratTakhtit) -> f32 {
    if nitaqat.is_empty() {
        return khiyarat.tabaud_ahruf;
    }
    uslub_ind(nitaqat, mawqi)
        .1
        .tabaud
        .unwrap_or(khiyarat.tabaud_ahruf)
}

/// The width one glyph contributes, advance plus whatever spacing applies to it.
///
/// **Letter spacing is applied here, after shaping, and never before it.** That
/// is the whole reason Arabic letter spacing works in this engine and does not
/// work in the implementations that do it at the character level. Inserting
/// spacing characters into the text, or shaping each character separately to
/// place them apart, changes what the shaper sees as adjacent — and adjacency is
/// what selects the initial, medial, final and isolated forms. The letters stop
/// joining. What comes out is not spaced-out Arabic; it is twenty-eight
/// disconnected shapes, which is the first of the four failures this product
/// exists to remove. Adding the spacing to the *advance of an already-shaped
/// glyph* moves the letters apart without the shaper ever knowing, so the forms
/// stay exactly as the font chose them.
///
/// A mark contributes nothing and is never spaced. Its advance is zero by
/// construction, and adding spacing to it would push the diacritic off the
/// letter it is attached to.
///
/// This one function is used by measurement, by breaking, by truncation and by
/// positioning, so the width a line was measured at is the width it is drawn at
/// by construction rather than by agreement between four pieces of arithmetic.
fn ard_harf(nass: &str, harf: &HarfMashkul, talab: &TalabTakhtit<'_>) -> f32 {
    if harf.alama {
        return harf.taqaddum_s;
    }
    let mut ard = harf.taqaddum_s + tabaud_harf(talab.nitaqat, harf.anqud, talab.khiyarat);
    if talab.khiyarat.tabaud_kalimat > 0.0 && masafa_ind(nass, harf.anqud) {
        ard += talab.khiyarat.tabaud_kalimat;
    }
    ard
}

/// The width of a run including the spacing the request adds.
fn ard_maqta_bil_tabaud(nass: &str, maqta: &MaqtaMashkul, talab: &TalabTakhtit<'_>) -> f32 {
    if maqta.asl.dharra.is_some() {
        return maqta.ard;
    }
    maqta
        .huruf
        .iter()
        .map(|harf| ard_harf(nass, harf, talab))
        .sum()
}

/// Whether the cluster at a byte offset is a word space.
fn masafa_ind(nass: &str, mawqi: u32) -> bool {
    usize::try_from(mawqi)
        .ok()
        .and_then(|hadd| nass.get(hadd..))
        .and_then(|dhail| dhail.chars().next())
        .is_some_and(|harf| harf.is_whitespace() && harf != '\u{00A0}' && harf != '\u{202F}')
}

/// The width of a sequence of runs including the request's spacing.
fn ard_maqati_bil_tabaud(nass: &str, maqati: &[MaqtaMashkul], talab: &TalabTakhtit<'_>) -> f32 {
    maqati
        .iter()
        .map(|maqta| ard_maqta_bil_tabaud(nass, maqta, talab))
        .sum()
}

/// The width of the whitespace that trails a line, measured on its **logical**
/// runs.
///
/// A line that ends in a space is not wider than the words in it. Counting the
/// trailing space would push a line that fits exactly into the overflow report,
/// and would let alignment shove the last word in from the margin by the width
/// of a space nobody can see. Rule L1 has already put that space at the end of
/// the line; this is the measurement half of the same decision.
///
/// It has to be measured logically. After rule L1 has reset the trailing space
/// to the paragraph level and rule L2 has reordered the line, a right-to-left
/// line's trailing space is at the *left* end of the visual array and a
/// left-to-right line's is at the right — so "the last glyphs in the array" is
/// the wrong end half the time. Measured once before reordering, the number is
/// correct for both, and it stays correct afterwards because justification never
/// stretches a line's outermost whitespace.
fn ard_dhail(nass: &str, maqati: &[MaqtaMashkul], talab: &TalabTakhtit<'_>) -> f32 {
    let mut dhail = 0.0_f32;
    for maqta in maqati.iter().rev() {
        if maqta.asl.dharra.is_some() {
            break;
        }
        let mut tawaqqaf = false;
        for harf in maqta.huruf.iter().rev() {
            if harf.alama {
                continue;
            }
            if masafa_ind(nass, harf.anqud) {
                dhail += ard_harf(nass, harf, talab);
            } else {
                tawaqqaf = true;
                break;
            }
        }
        if tawaqqaf {
            break;
        }
    }
    dhail
}

/// The width a line is judged by, given the trailing whitespace already measured
/// for it.
fn ard_assatr(nass: &str, maqati: &[MaqtaMashkul], talab: &TalabTakhtit<'_>, dhail: f32) -> f32 {
    (ard_maqati_bil_tabaud(nass, maqati, talab) - dhail).max(0.0)
}

/// Refuses a request that cannot be laid out at all.
///
/// The checks are the ones whose failure would otherwise surface as a nonsense
/// layout rather than as an error: a width no line can fit in, a size the
/// rasterizer cannot produce a glyph at, and a shrink-to-fit floor that is not
/// below the size it is a floor for.
///
/// A height that is present but not positive is refused through
/// [`KhataSaff::ArdGhayrSalih`] carrying the height. A box with no height and a
/// box with no width are the same refusal — there is no room in it — and the
/// alternative was a second variant, which would renumber every stable error
/// code after it. Codes in this product are permanent; a diagnostics bundle from
/// an older build has to keep meaning what it said.
///
/// # Errors
///
/// - [`KhataSaff::ArdGhayrSalih`] when the available width, or the available
///   height, is present and is not a positive finite number of pixels.
/// - [`KhataSaff::HajmGhayrSalih`] when the pixel size is outside the range the
///   rasterizer will produce a usable glyph in, and when a
///   [`SiyasatTajawuz::Taqlis`] floor is not a positive size at or below it.
pub fn tahaqquq(talab: &TalabTakhtit<'_>) -> Natija<()> {
    if !talab.hajm.is_finite() || talab.hajm < HAJM_ADNA || talab.hajm > HAJM_AQSA {
        return Err(Khata::min_tafsir(&KhataSaff::HajmGhayrSalih {
            hajm: talab.hajm,
        }));
    }
    if let Some(ard) = talab.ard_mutah
        && (!ard.is_finite() || ard <= 0.0)
    {
        return Err(Khata::min_tafsir(&KhataSaff::ArdGhayrSalih { ard }));
    }
    if let Some(irtifa) = talab.irtifa_mutah
        && (!irtifa.is_finite() || irtifa <= 0.0)
    {
        return Err(Khata::min_tafsir(&KhataSaff::ArdGhayrSalih { ard: irtifa }));
    }
    if let SiyasatTajawuz::Taqlis { adna } = talab.khiyarat.tajawuz
        && (!adna.is_finite() || adna < HAJM_ADNA || adna > talab.hajm)
    {
        return Err(Khata::min_tafsir(&KhataSaff::HajmGhayrSalih { hajm: adna }));
    }
    Ok(())
}

/// One line, after breaking and reordering but before its glyphs are placed.
///
/// This is the shared shape [`rattib_sutur`] and [`qis`] both work from. It
/// holds the runs rather than glyphs, because until alignment has resolved a
/// starting edge there is no position to give a glyph, and because `qis` never
/// needs one.
#[derive(Debug, Clone, PartialEq)]
struct SatrJari {
    /// The line's runs, in visual order.
    maqati: Vec<MaqtaMashkul>,
    /// The logical byte range the line covers.
    mantiqi: Range<u32>,
    /// The base embedding level of the paragraph this line belongs to.
    ///
    /// Not the layout's. A string holding an Arabic line, a newline and an
    /// English line is two paragraphs, each of which resolved its own base
    /// direction under P2/P3, and this is the level rule L2 reorders this line
    /// at and the level its alignment edge is taken from.
    mustawa: u8,
    /// The line's base direction, which is the parity of [`SatrJari::mustawa`].
    ittijah: Ittijah,
    /// Whether this is the paragraph's last line.
    akhir: bool,
    /// Whether the line was ended by a mandatory break rather than by running
    /// out of width. Justification never stretches such a line: the author
    /// pressed return, and a line that ends where it was told to end is already
    /// the length it is supposed to be.
    ilzami: bool,
    /// The measured width of the content, trailing whitespace excluded.
    ard: f32,
    /// The width of the whitespace trailing the line, measured while the runs
    /// were still in logical order and subtracted from every later measurement
    /// of this line.
    dhail: f32,
    /// How far the tallest content rises above the baseline.
    suud: f32,
    /// How far the deepest content falls below it.
    hubut: f32,
    /// The line box's height.
    irtifa: f32,
    /// The baseline's distance from the top of the line box.
    asas: f32,
    /// Where the line box begins horizontally, after alignment.
    bidaya: f32,
    /// How much width justification added.
    dabt: f32,
}

/// The line structure, which is everything a layout is except glyph positions.
#[derive(Debug, Clone, PartialEq)]
struct BinaSutur {
    /// The lines, in reading order from the top.
    sutur: Vec<SatrJari>,
    /// The width of the widest line.
    ard: f32,
    /// The total height of every line box.
    irtifa: f32,
    /// The paragraph's base direction.
    ittijah: Ittijah,
    /// The size the text was laid out at, which differs from the size requested
    /// when shrink-to-fit settled lower.
    hajm: f32,
    /// Present when the text did not fit.
    tajawuz: Option<TaqreerTajawuz>,
    /// Whether the overflow policy truncated anything.
    maqsus: bool,
    /// The style spans the layout was actually built against.
    ///
    /// Carried rather than borrowed from the request because shrink-to-fit
    /// rebuilds the layout against *scaled* spans, and the positioning pass has
    /// to read the same spans the measuring pass did. Reading the caller's
    /// original spans after the search settled two sizes down would place every
    /// glyph in a size-overridden span at the wrong offset with the wrong
    /// spacing — measured at one size and drawn at another.
    nitaqat: Vec<NitaqUslub>,
    /// The options the layout was actually built against, scaled with the spans
    /// and for the same reason.
    khiyarat: KhiyaratTakhtit,
}

impl BinaSutur {
    /// An empty structure, which is what empty text produces.
    fn farigha(ittijah: Ittijah, hajm: f32, talab: &TalabTakhtit<'_>) -> Self {
        Self {
            sutur: Vec::new(),
            ard: 0.0,
            irtifa: 0.0,
            ittijah,
            hajm,
            tajawuz: None,
            maqsus: false,
            nitaqat: talab.nitaqat.to_vec(),
            khiyarat: talab.khiyarat.clone(),
        }
    }

    /// Whether every line fits the room it was given.
    ///
    /// This is the predicate the shrink-to-fit search converges on, and it is
    /// deliberately answered from the finished line structure rather than from a
    /// prediction: the search asks whether a real layout at a real size fits,
    /// and gets an answer that was measured.
    fn tulaim(&self, talab: &TalabTakhtit<'_>) -> bool {
        if let Some(ard) = talab.ard_mutah
            && self.ard > ard + DIQQA
        {
            return false;
        }
        if let Some(irtifa) = talab.irtifa_mutah
            && self.irtifa > irtifa + DIQQA
        {
            return false;
        }
        true
    }
}

/// A line as breaking left it: its runs in logical order, the text it covers,
/// and why it ended.
#[derive(Debug, Clone, PartialEq)]
struct SatrMaqtu {
    /// The runs, still in logical order — reordering happens afterwards.
    maqati: Vec<MaqtaMashkul>,
    /// The logical byte range covered.
    mantiqi: Range<u32>,
    /// Whether a mandatory break ended it.
    ilzami: bool,
}

/// The view of the text a run is reshaped against after a break.
///
/// The problem this solves is easy to state and easy to get wrong. `wasl` hands
/// the shaper the characters on either side of a run as context, which is
/// exactly right while the run is in the middle of a sentence: it is what keeps
/// a bold word inside a sentence joined to the words around it. But after a
/// break, the two sides of the break are on different lines and must **not** be
/// joined — and if the tail is reshaped against the whole string, the shaper
/// sees the head sitting right in front of it and joins to it, handing back the
/// medial forms the break was supposed to replace.
///
/// So the text before the line's start is blanked to ASCII spaces. A space joins
/// to nothing, so the first letter of the line comes out initial or isolated as
/// it should; and because a space is one byte and it replaces the bytes of the
/// characters it covers one for one, **every byte offset in the string is
/// unchanged**. Cluster indices, run ranges, style spans and break offsets all
/// still mean what they meant. The head of a break needs no such treatment: it
/// is shaped against a prefix slice of this same string, which ends where the
/// line ends and therefore has nothing after it to join to.
///
/// The blanking is monotonic — lines are produced top to bottom, so each break
/// only blanks the bytes the previous one did not — which makes the whole pass
/// one traversal of the text rather than one per line.
fn insa(dhail: &mut String, min: u32, hatta: u32) {
    let (Ok(bidaya), Ok(nihaya)) = (usize::try_from(min), usize::try_from(hatta)) else {
        return;
    };
    let nihaya = nihaya.min(dhail.len());
    if bidaya >= nihaya || !dhail.is_char_boundary(bidaya) || !dhail.is_char_boundary(nihaya) {
        return;
    }
    // Equal length in, equal length out: the replacement is exactly one ASCII
    // space per byte removed, so nothing after it moves.
    dhail.replace_range(bidaya..nihaya, &" ".repeat(nihaya.saturating_sub(bidaya)));
}

/// The smallest mandatory break inside a half-open byte range.
fn fursa_ilzamiya(furas: &[FursatQat], baad: u32, hatta: u32) -> Option<u32> {
    furas
        .iter()
        .find(|fursa| fursa.ilzami && fursa.mawqi > baad && fursa.mawqi <= hatta)
        .map(|fursa| fursa.mawqi)
}

/// The last break opportunity at or before an offset, and after the line's start.
///
/// This is the break the contract names: *the last opportunity at or before the
/// overflow point*. Taking the last one rather than the first keeps the line as
/// full as it legally can be, which is what makes a paragraph look set rather
/// than ragged.
fn fursa_qabl(furas: &[FursatQat], baad: u32, hatta: u32) -> Option<u32> {
    let hadd = furas.partition_point(|fursa| fursa.mawqi <= hatta);
    furas
        .get(..hadd)?
        .iter()
        .rev()
        .find(|fursa| fursa.mawqi > baad)
        .map(|fursa| fursa.mawqi)
}

/// The first break opportunity after an offset.
///
/// Used only when there is no opportunity at all before the overflow — a single
/// word longer than the box — where [`SiyasatTajawuz::Ballagh`] says to lay the
/// line out anyway and report it. The line runs on to the first place it is
/// legal to break rather than being cut through the middle of a word, because a
/// word severed at an arbitrary letter is not a measurement problem the reviewer
/// can act on, it is a rendering defect.
fn fursa_baad(furas: &[FursatQat], baad: u32) -> Option<u32> {
    let hadd = furas.partition_point(|fursa| fursa.mawqi <= baad);
    furas.get(hadd..)?.first().map(|fursa| fursa.mawqi)
}

/// The byte offset of the first cluster in a run that does not fit in the room
/// left on the line.
///
/// Clusters are summed in **logical** order, not in the order the glyphs came
/// back in. In a right-to-left run the shaper emits the last cluster first, so
/// walking the glyph array would accumulate the line from its far end and answer
/// with an offset from the wrong side of the word. A cluster's total advance is
/// the same wherever reordering puts it, which is what makes summing them in
/// text order both legitimate and direction-independent.
///
/// An atom answers with its own start: a placeholder or a sprite is one
/// indivisible box, and the only place it can be broken is before it.
fn hadd_attajawuz(nass: &str, maqta: &MaqtaMashkul, mutah: f32, talab: &TalabTakhtit<'_>) -> u32 {
    if maqta.asl.dharra.is_some() {
        return maqta.asl.nitaq.start;
    }

    // Glyphs of one cluster are contiguous in the array, so one pass groups them.
    let mut anaqid: Vec<(u32, f32)> = Vec::new();
    for harf in &maqta.huruf {
        let ard = ard_harf(nass, harf, talab);
        match anaqid.last_mut() {
            Some((anqud, majmu)) if *anqud == harf.anqud => *majmu += ard,
            _ => anaqid.push((harf.anqud, ard)),
        }
    }
    anaqid.sort_unstable_by_key(|(anqud, _)| *anqud);

    let mut jari = 0.0_f32;
    for (anqud, ard) in &anaqid {
        if jari + *ard > mutah + DIQQA {
            return *anqud;
        }
        jari += *ard;
    }
    maqta.asl.nitaq.end
}

/// Reshapes one run against a given view of the text.
///
/// # Errors
///
/// Whatever [`shakkil_maqati_bi_asalib`] reports. A run that shaped a moment ago
/// and will not shape with one letter fewer is a font that is lying about its
/// own tables, and the failure is passed up rather than papered over with the
/// glyphs from before the break.
fn ashkil_maqta(
    masdar: &str,
    asl: &MaqtaMantiqi,
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<MaqtaMashkul> {
    let mut mashkula = shakkil_maqati_bi_asalib(
        masdar,
        core::slice::from_ref(asl),
        talab.nitaqat,
        talab.khutut,
        talab.khiyarat,
        makhzan,
    )?;
    Ok(mashkula
        .pop()
        .unwrap_or_else(|| MaqtaMashkul::min_asl(asl.clone())))
}

/// Cuts the line at a byte offset and reshapes both sides of the cut.
///
/// Three things happen, in this order and for this reason:
///
/// 1. **The line gives back what belongs to the next one.** Runs that start at
///    or after the cut go back to the head of the queue, and a run straddling
///    the cut is divided into two ranges. An atom is never divided — a
///    placeholder is one box — so a cut that falls inside one moves the whole
///    atom to the next line instead.
/// 2. **The text before the new line start is blanked**, per [`insa`], so that
///    the tail's shaping cannot see the head.
/// 3. **The two runs on either side of the seam are shaped again.** This is the
///    step the contract insists on and the one that is tempting to skip: the
///    head's last letter has just become final where it was medial, and the
///    tail's first letter has become initial. Different glyphs, different
///    advances, different ink. Reusing the pre-break shaping would measure the
///    head with letterforms it will never draw, and would leave `kashida` a
///    joining record describing a joint that the break has just severed —
///    justification would then elongate a connection that is not there.
///
/// The cut is never allowed to leave the line empty. When every run wants to go
/// to the next line, the first one is kept and the line simply overflows, which
/// is a reported defect rather than an infinite loop.
///
/// # Errors
///
/// As [`ashkil_maqta`].
fn iqta(
    dhail: &mut String,
    mansi: &mut u32,
    jari: &mut Vec<MaqtaMashkul>,
    tabur: &mut VecDeque<MaqtaMashkul>,
    mawqi: u32,
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<()> {
    while jari
        .last()
        .is_some_and(|akhir| akhir.asl.nitaq.start >= mawqi)
    {
        if let Some(akhir) = jari.pop() {
            tabur.push_front(akhir);
        }
    }

    let yashtur = jari
        .last()
        .is_some_and(|akhir| akhir.asl.nitaq.start < mawqi && akhir.asl.nitaq.end > mawqi);
    if yashtur && let Some(akhir) = jari.pop() {
        if akhir.asl.dharra.is_some() {
            tabur.push_front(akhir);
        } else {
            let mut ras = akhir.asl.clone();
            ras.nitaq = akhir.asl.nitaq.start..mawqi;
            let mut dhayl = akhir.asl.clone();
            dhayl.nitaq = mawqi..akhir.asl.nitaq.end;
            jari.push(MaqtaMashkul::min_asl(ras));
            tabur.push_front(MaqtaMashkul::min_asl(dhayl));
        }
    }

    if jari.is_empty()
        && let Some(awwal) = tabur.pop_front()
    {
        jari.push(awwal);
    }

    // The seam is where the line really ended, which is the cut unless keeping
    // the line non-empty pushed it further along.
    let Some(hadd) = jari.last().map(|akhir| akhir.asl.nitaq.end) else {
        return Ok(());
    };

    if let Some(akhir) = jari.last().map(|akhir| akhir.asl.clone()) {
        let Ok(nihaya) = usize::try_from(hadd) else {
            return Ok(());
        };
        if let Some(ras) = dhail.get(..nihaya.min(dhail.len())) {
            let mashkul = ashkil_maqta(ras, &akhir, talab, makhzan)?;
            if let Some(hadaf) = jari.last_mut() {
                *hadaf = mashkul;
            }
        }
    }

    insa(dhail, *mansi, hadd);
    *mansi = hadd;

    if let Some(awwal) = tabur.front().map(|awwal| awwal.asl.clone())
        && awwal.nitaq.start == hadd
    {
        let mashkul = ashkil_maqta(dhail.as_str(), &awwal, talab, makhzan)?;
        if let Some(hadaf) = tabur.front_mut() {
            *hadaf = mashkul;
        }
    }

    Ok(())
}

/// Breaks the shaped runs into lines.
///
/// The loop accumulates runs onto a candidate line, measuring as it goes from
/// the runs' real advances. Three things end a line, and they are tested in this
/// order because they do not have equal authority:
///
/// 1. **A mandatory break.** A newline is not an opportunity, it is an
///    instruction, and it ends the line whatever the width says.
/// 2. **Overflow with a break available.** The line is cut at the last
///    opportunity at or before the point where the width ran out.
/// 3. **Overflow with none available.** A word wider than its box. The line runs
///    on to the first legal break after the overflow and is reported. Cutting
///    through the middle of the word instead would sever a join and produce a
///    rendering defect where there was a measurement problem — and a measurement
///    problem is something a translator can fix by rewriting the sentence, which
///    is the entire purpose of the overflow report.
///
/// `satr_wahid` suppresses only the width-driven break. A mandatory break still
/// breaks: the text really does contain a paragraph separator, and running two
/// sentences together on one line to honour a single-line field would lose the
/// boundary between them with nothing to show for it.
///
/// # Errors
///
/// As [`iqta`], for the first run that cannot be reshaped across a break.
fn iqta_assutur(
    nass: &str,
    maqati: Vec<MaqtaMashkul>,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<Vec<SatrMaqtu>> {
    let mutah = if talab.khiyarat.satr_wahid {
        None
    } else {
        talab.ard_mutah
    };

    let mut tabur: VecDeque<MaqtaMashkul> = maqati.into();
    let mut sutur: Vec<SatrMaqtu> = Vec::new();
    // Allocated on the first break and not before, so a string that fits on one
    // line never pays for a copy of itself.
    let mut dhail: Option<String> = None;
    let mut mansi: u32 = 0;

    while !tabur.is_empty() {
        let bidaya = tabur.front().map_or(0, |awwal| awwal.asl.nitaq.start);
        let mut jari: Vec<MaqtaMashkul> = Vec::new();
        let mut ard_jari = 0.0_f32;
        let mut ilzami = false;

        while let Some(maqta) = tabur.pop_front() {
            let nitaq = maqta.asl.nitaq.clone();
            let ard_maqta_hali = ard_maqta_bil_tabaud(nass, &maqta, talab);

            if let Some(qat) = fursa_ilzamiya(furas, nitaq.start.max(bidaya), nitaq.end) {
                jari.push(maqta);
                let masdar = dhail.get_or_insert_with(|| nass.to_owned());
                iqta(
                    masdar, &mut mansi, &mut jari, &mut tabur, qat, talab, makhzan,
                )?;
                ilzami = true;
                break;
            }

            let yatajawaz = mutah.is_some_and(|hadd| ard_jari + ard_maqta_hali > hadd + DIQQA);
            if !yatajawaz {
                ard_jari += ard_maqta_hali;
                jari.push(maqta);
                continue;
            }

            let mutabaqqi = mutah.unwrap_or(0.0) - ard_jari;
            let hadd = hadd_attajawuz(nass, &maqta, mutabaqqi, talab);
            let qat = fursa_qabl(furas, bidaya, hadd).or_else(|| {
                // Nothing legal before the overflow. The next opportunity is
                // taken only when it lies inside this run; when it is further
                // along, the run joins the line and the search continues on the
                // next one rather than cutting the line short of it.
                fursa_baad(furas, hadd).filter(|baad| *baad <= nitaq.end)
            });

            ard_jari += ard_maqta_hali;
            jari.push(maqta);

            if let Some(qat) = qat.filter(|qat| *qat > bidaya) {
                let masdar = dhail.get_or_insert_with(|| nass.to_owned());
                iqta(
                    masdar, &mut mansi, &mut jari, &mut tabur, qat, talab, makhzan,
                )?;
                break;
            }
        }

        if jari.is_empty() {
            break;
        }
        let nihaya = jari.last().map_or(bidaya, |akhir| akhir.asl.nitaq.end);
        sutur.push(SatrMaqtu {
            maqati: jari,
            mantiqi: bidaya..nihaya,
            ilzami,
        });
    }

    Ok(sutur)
}

/// Builds the line structure, applying the declared overflow policy.
///
/// This is the one place the three policies of [`SiyasatTajawuz`] diverge, and
/// they diverge here rather than deeper because each of them is a decision about
/// *the whole layout*, not about a line:
///
/// - [`SiyasatTajawuz::Ballagh`] lays the text out as it is and fills in the
///   overflow report. It is the default because a patch that silently shrinks
///   its own text hides the defect instead of reporting it, and the reviewer
///   never learns that a translation is too long for its button.
/// - [`SiyasatTajawuz::Taqlis`] shrinks until it fits, never below its floor,
///   and records the size it settled on in [`BinaSutur::hajm`].
/// - [`SiyasatTajawuz::Ikhtisar`] cuts the text and marks the cut with an
///   ellipsis on the side the line ends on.
///
/// # Errors
///
/// As [`tahaqquq`] for an unusable request, as [`iqta_assutur`] for a break that
/// cannot be reshaped, and [`KhataSaff::TaadhurAlqat`] when truncation is asked
/// for in a width that cannot even hold the ellipsis.
fn ibni(
    nass: &str,
    maqati: Vec<MaqtaMashkul>,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<BinaSutur> {
    tahaqquq(talab)?;
    if nass.is_empty() || maqati.is_empty() {
        return Ok(BinaSutur::farigha(
            tahleel.ittijah_asas(),
            talab.hajm,
            talab,
        ));
    }

    match talab.khiyarat.tajawuz {
        SiyasatTajawuz::Taqlis { adna } => {
            taqlis(nass, maqati, furas, talab, tahleel, makhzan, adna)
        },
        SiyasatTajawuz::Ballagh | SiyasatTajawuz::Ikhtisar => {
            ibni_asasi(nass, maqati, furas, talab, tahleel, makhzan, talab.hajm)
        },
    }
}

/// Shrinks the text until it fits, by binary search on real measured layouts.
///
/// The search is stated in full because its correctness rests on what it
/// measures rather than on the search itself:
///
/// 1. **The requested size is tried first.** The overwhelmingly common case is
///    that the text fits, and that case costs one layout and no search at all.
/// 2. **The floor is tried next.** If the text does not fit even at the smallest
///    size the caller permits, the floor is the answer: the layout at `adna` is
///    returned with its overflow report filled in. Nothing ever goes below the
///    floor, because a floor exists precisely to say "below this the text is not
///    worth rendering", and a layout that ignored it would produce unreadable
///    text that reported no problem.
/// 3. **Between them, halve.** The invariant is that `adna_hajm` is a size that
///    was *measured* to fit and `aqsa_hajm` is one that was *measured* not to.
///    Each step lays the text out completely at the midpoint — re-splitting into
///    runs, re-shaping every run at that size, re-breaking every line — and asks
///    the finished structure whether it fits. Nothing is scaled, interpolated,
///    or predicted from the previous probe: font metrics are not linear in the
///    size (hinting-free scaling is close, but breaking is not, and one pixel
///    can move a word to another line), so a search over a predicted width would
///    converge confidently on a size that does not actually fit.
/// 4. **Stop at a quarter of a pixel, or at the iteration bound.** The layout
///    that is returned is always one that was measured to fit, never the last
///    midpoint tried, so a font that behaves non-monotonically under scaling
///    cannot make the result worse than the best size the search actually saw.
///
/// # Errors
///
/// As [`ansha_bi_hajm`] for any probe that cannot be shaped.
fn taqlis(
    nass: &str,
    maqati: Vec<MaqtaMashkul>,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
    adna: f32,
) -> Natija<BinaSutur> {
    let kamila = ibni_asasi(nass, maqati, furas, talab, tahleel, makhzan, talab.hajm)?;
    if kamila.tulaim(talab) {
        return Ok(kamila);
    }

    let asghar = ansha_bi_hajm(nass, furas, talab, tahleel, makhzan, adna)?;
    if !asghar.tulaim(talab) {
        return Ok(asghar);
    }

    let mut yulaim = adna;
    let mut la_yulaim = talab.hajm;
    let mut afdal = asghar;

    let mut dawra: u32 = 0;
    while la_yulaim - yulaim > DIQQA_HAJM && dawra < ADAD_TAQLIS {
        let wasat = 0.5 * (yulaim + la_yulaim);
        let mujarrab = ansha_bi_hajm(nass, furas, talab, tahleel, makhzan, wasat)?;
        if mujarrab.tulaim(talab) {
            yulaim = wasat;
            afdal = mujarrab;
        } else {
            la_yulaim = wasat;
        }
        dawra = dawra.saturating_add(1);
    }

    Ok(afdal)
}

/// Lays the whole text out again at a different size.
///
/// Everything scales together. The base size, the per-span size overrides, the
/// letter and word spacing, the line height, and the width, height and baseline
/// offset of every atom are all multiplied by the same ratio, because shrinking
/// to fit means shrinking the *text*, and a heading span or an inline sprite
/// that kept its original size would be the thing still overflowing after the
/// search had shrunk everything around it into illegibility.
///
/// Runs are split again rather than rescaled: a size change moves the font's
/// character coverage decisions not at all, but it does change every advance,
/// and the runs carry their size, so they are rebuilt from the same
/// bidirectional analysis — which does not depend on size and is therefore not
/// recomputed.
///
/// # Errors
///
/// As [`crate::ittijah::qassim`] for a style span that does not fit the text, as
/// [`shakkil_maqati_bi_asalib`] for a run that will not shape, and as
/// [`ibni_asasi`] for the layout itself.
fn ansha_bi_hajm(
    nass: &str,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
    hajm: f32,
) -> Natija<BinaSutur> {
    let nisba = if talab.hajm > DIQQA {
        hajm / talab.hajm
    } else {
        1.0
    };
    let nitaqat = nitaqat_bi_nisba(talab.nitaqat, nisba);
    let khiyarat = khiyarat_bi_nisba(talab.khiyarat, nisba);
    let farii = TalabTakhtit {
        hajm,
        nitaqat: &nitaqat,
        khiyarat: &khiyarat,
        ..*talab
    };

    let lugha = lugha_almatlub(nass, &khiyarat);
    let mantiqiya =
        crate::ittijah::qassim(nass, tahleel, farii.nitaqat, farii.khutut, lugha, hajm)?;
    let mashkula = shakkil_maqati_bi_asalib(
        nass,
        &mantiqiya,
        farii.nitaqat,
        farii.khutut,
        farii.khiyarat,
        makhzan,
    )?;

    ibni_asasi(nass, mashkula, furas, &farii, tahleel, makhzan, hajm)
}

/// The language in force: the declared one, or the one detected from the text.
fn lugha_almatlub(nass: &str, khiyarat: &KhiyaratTakhtit) -> LughaNass {
    match khiyarat.lugha {
        LughaNass::Tilqai => crate::lugha::iktashif_lugha(nass),
        muhaddada => muhaddada,
    }
}

/// The style spans, with every pixel value scaled.
fn nitaqat_bi_nisba(nitaqat: &[NitaqUslub], nisba: f32) -> Vec<NitaqUslub> {
    if !nisba.is_finite() || (nisba - 1.0).abs() <= f32::EPSILON {
        return nitaqat.to_vec();
    }
    nitaqat
        .iter()
        .map(|nitaq| {
            let uslub = Uslub {
                hajm: nitaq.uslub.hajm.map(|qeema| qeema * nisba),
                tabaud: nitaq.uslub.tabaud.map(|qeema| qeema * nisba),
                izaha: nitaq.uslub.izaha.map(|qeema| qeema * nisba),
                dharra: nitaq.uslub.dharra.map(|dharra| Dharra {
                    ard: dharra.ard * nisba,
                    irtifa: dharra.irtifa * nisba,
                    asas: dharra.asas * nisba,
                    marja: dharra.marja,
                }),
                ..nitaq.uslub
            };
            NitaqUslub { uslub, ..*nitaq }
        })
        .collect()
}

/// The options, with every pixel value scaled.
fn khiyarat_bi_nisba(khiyarat: &KhiyaratTakhtit, nisba: f32) -> KhiyaratTakhtit {
    let mut mansuba = khiyarat.clone();
    if !nisba.is_finite() || (nisba - 1.0).abs() <= f32::EPSILON {
        return mansuba;
    }
    mansuba.tabaud_ahruf *= nisba;
    mansuba.tabaud_kalimat *= nisba;
    mansuba.irtifa_satr = mansuba.irtifa_satr.map(|qeema| qeema * nisba);
    mansuba
}

/// Cuts the text down to what fits and marks the cut with an ellipsis.
///
/// Truncation runs on the **logical** lines, before reordering, and that is not
/// a detail. What truncation removes is the *end of the text*, and what it shows
/// is a mark at the *end of the line* — in a right-to-left line the left edge,
/// in a left-to-right line the right. Cutting a reordered line would mean
/// deciding which visual end holds the logical tail, which in mixed-direction
/// text is not one end but both. Cutting logically and letting the reorder place
/// the ellipsis run gets it right by construction: the ellipsis is the last
/// thing in the text, it carries the paragraph's own level, and rule L2 puts the
/// last thing at the paragraph's trailing edge.
///
/// Vertical truncation comes first, because a line that will be dropped for
/// height should not first be trimmed for width. Line boxes are already measured
/// at this point — a line's height does not depend on the order of its runs — so
/// the decision can be made before anything is reordered.
///
/// # Errors
///
/// [`KhataSaff::TaadhurAlqat`] when the available width cannot hold the ellipsis
/// by itself. There is no truncation of the text that fits in that box, so
/// reporting is the only honest answer; returning an empty line would claim the
/// text was rendered.
fn ikhtasir(
    nass: &str,
    sutur: &mut Vec<SatrJari>,
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
    hajm: f32,
) -> Natija<bool> {
    let mut maqsus = false;
    // Tracked apart from `maqsus`, because only a *vertical* cut puts an
    // ellipsis on a line that is not itself too wide. Conflating the two would
    // mark the last line of a block whose second line was trimmed for width,
    // where nothing below it was dropped at all.
    let mut qussa_tulan = false;

    if let Some(mutah) = talab.irtifa_mutah {
        let mut mutarakim = 0.0_f32;
        let mut adad = sutur.len();
        for (fahras, satr) in sutur.iter().enumerate() {
            if mutarakim + satr.irtifa > mutah + DIQQA {
                // At least one line is always kept: a box too short for even one
                // line still has to show something, and an empty result would
                // claim the text was rendered.
                adad = fahras.max(1);
                break;
            }
            mutarakim += satr.irtifa;
        }
        if adad < sutur.len() {
            sutur.truncate(adad);
            maqsus = true;
            qussa_tulan = true;
        }
    }

    let Some(mutah) = talab.ard_mutah else {
        // Nothing to truncate against horizontally. A vertical cut may still
        // have happened, and the ellipsis belongs on the line it happened at.
        if qussa_tulan && let Some(akhir) = sutur.last_mut() {
            alhiq_hadhf(nass, akhir, talab, makhzan, hajm)?;
        }
        return Ok(maqsus);
    };

    // Shaped once, at the layout's own level, and re-levelled per line below.
    let hadhf = maqta_hadhf(nass, talab, tahleel.mustawa_asas(), makhzan, hajm)?;
    let ard_hadhf = ard_maqta_bil_tabaud(nass, &hadhf, talab);
    if ard_hadhf > mutah + DIQQA {
        return Err(Khata::min_tafsir(&KhataSaff::TaadhurAlqat {
            ard: ard_hadhf,
            mutah,
        }));
    }

    let akhir_satr = sutur.len().saturating_sub(1);
    for (fahras, satr) in sutur.iter_mut().enumerate() {
        let yatajawaz = satr.ard > mutah + DIQQA;
        let yahtaj_hadhf = yatajawaz || (qussa_tulan && fahras == akhir_satr);
        if !yahtaj_hadhf {
            continue;
        }
        if yatajawaz {
            let hadd = hadd_iqtitaa(nass, &satr.maqati, mutah - ard_hadhf, talab);
            iqtata_satr(nass, satr, hadd, talab, makhzan)?;
            maqsus = true;
        }
        satr.maqati.push(bi_mustawa(&hadhf, satr.mustawa));
        // The ellipsis is the line's last content, so nothing trails it any
        // more and the line is measured whole.
        satr.dhail = 0.0;
        satr.ard = ard_assatr(nass, &satr.maqati, talab, satr.dhail);
    }

    Ok(maqsus)
}

/// Re-levels an already-shaped ellipsis run onto a line's own paragraph level.
///
/// Only the level changes, and only the level needs to. The ellipsis is one
/// non-joining, script-neutral glyph: it takes no contextual form, ligates with
/// nothing, and looks identical whichever direction it was shaped in. What does
/// depend on direction is where rule L2 puts it, and L2 reads the level — so
/// carrying one shaped run and re-levelling it per line is exact, and costs no
/// shaping call per truncated line.
fn bi_mustawa(hadhf: &MaqtaMashkul, mustawa: u8) -> MaqtaMashkul {
    let mut mansub = hadhf.clone();
    mansub.asl.mustawa = mustawa;
    mansub.asl.ittijah = Ittijah::min_mustawa(mustawa);
    mansub
}

/// Appends the ellipsis to a line that was cut for height rather than width.
///
/// # Errors
///
/// As [`maqta_hadhf`].
fn alhiq_hadhf(
    nass: &str,
    satr: &mut SatrJari,
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
    hajm: f32,
) -> Natija<()> {
    let hadhf = maqta_hadhf(nass, talab, satr.mustawa, makhzan, hajm)?;
    satr.maqati.push(hadhf);
    satr.dhail = 0.0;
    satr.ard = ard_assatr(nass, &satr.maqati, talab, satr.dhail);
    Ok(())
}

/// Shapes the ellipsis as a run of its own.
///
/// It is shaped against a string containing nothing but the ellipsis, which is
/// correct rather than merely convenient: the mark stands outside the text and
/// must not join to it, take a contextual form from it, or acquire a cluster
/// index inside it.
///
/// Its glyphs are then given the cluster index of the text's end. That is what a
/// caret expects: the ellipsis stands for everything from the cut onward, so
/// clicking it lands where the text was cut rather than nowhere.
///
/// `mustawa` is the base level of the paragraph the ellipsis is being added to,
/// not the layout's. That is what makes rule L2 place it against *that line's*
/// trailing edge without this function knowing which edge that is — the left in
/// a right-to-left paragraph, the right in a left-to-right one, and genuinely
/// different edges for two lines of one truncated block when the block holds
/// paragraphs that disagree.
///
/// # Errors
///
/// As [`shakkil_maqati_bi_asalib`], when the font chain has no glyph path for
/// the ellipsis at all.
fn maqta_hadhf(
    nass: &str,
    talab: &TalabTakhtit<'_>,
    mustawa: u8,
    makhzan: &mut MakhzanTashkeel,
    hajm: f32,
) -> Natija<MaqtaMashkul> {
    let mut nass_hadhf = String::with_capacity(HADHF.len_utf8());
    nass_hadhf.push(HADHF);
    let tul = u32::try_from(nass_hadhf.len()).unwrap_or(0);

    let asl = MaqtaMantiqi {
        nitaq: 0..tul,
        mustawa,
        ittijah: Ittijah::min_mustawa(mustawa),
        uslub: 0,
        khatt: talab.khutut.ikhtiyar(HADHF, None),
        kitaba: crate::maqta::Kitaba::ZYYY,
        lugha: lugha_almatlub(nass, talab.khiyarat),
        dharra: None,
        hajm,
    };

    // Shaped against an empty span table, not the caller's. The spans describe
    // the caller's text and this run is not in it; a span that happens to cover
    // byte zero would otherwise put its weight and slant on the ellipsis.
    let mut mashkula = shakkil_maqati_bi_asalib(
        &nass_hadhf,
        core::slice::from_ref(&asl),
        &[],
        talab.khutut,
        talab.khiyarat,
        makhzan,
    )?;
    let mut mashkul = mashkula
        .pop()
        .unwrap_or_else(|| MaqtaMashkul::min_asl(asl.clone()));

    let nihaya = u32::try_from(nass.len()).unwrap_or(u32::MAX);
    for harf in &mut mashkul.huruf {
        harf.anqud = nihaya;
    }
    mashkul.asl.nitaq = nihaya..nihaya;
    Ok(mashkul)
}

/// The byte offset at which a line stops fitting in a given width.
fn hadd_iqtitaa(nass: &str, maqati: &[MaqtaMashkul], mutah: f32, talab: &TalabTakhtit<'_>) -> u32 {
    let mut jari = 0.0_f32;
    for maqta in maqati {
        let ard = ard_maqta_bil_tabaud(nass, maqta, talab);
        if jari + ard > mutah + DIQQA {
            return hadd_attajawuz(nass, maqta, mutah - jari, talab);
        }
        jari += ard;
    }
    maqati.last().map_or(0, |akhir| akhir.asl.nitaq.end)
}

/// Drops everything in a line from a byte offset onward, reshaping the run the
/// cut falls inside.
///
/// The reshape is the same requirement as at a line break and for the same
/// reason: the letter before the cut has just become final, and drawing it as
/// the medial form it was shaped as would leave a connecting stroke running into
/// the ellipsis out of nowhere.
///
/// # Errors
///
/// As [`ashkil_maqta`].
fn iqtata_satr(
    nass: &str,
    satr: &mut SatrJari,
    hadd: u32,
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<()> {
    satr.maqati.retain(|maqta| maqta.asl.nitaq.start < hadd);

    let yashtur = satr
        .maqati
        .last()
        .is_some_and(|akhir| akhir.asl.nitaq.end > hadd && akhir.asl.dharra.is_none());
    if yashtur && let Some(akhir) = satr.maqati.last().map(|akhir| akhir.asl.clone()) {
        let Ok(nihaya) = usize::try_from(hadd) else {
            return Ok(());
        };
        let mut maqsus = akhir.clone();
        maqsus.nitaq = akhir.nitaq.start..hadd;
        if let Some(ras) = nass.get(..nihaya.min(nass.len())) {
            let mashkul = ashkil_maqta(ras, &maqsus, talab, makhzan)?;
            if let Some(hadaf) = satr.maqati.last_mut() {
                *hadaf = mashkul;
            }
        }
    }

    satr.mantiqi = satr.mantiqi.start..hadd.max(satr.mantiqi.start);
    Ok(())
}

/// How far a line rises above and falls below its baseline.
///
/// The maximum over the line's runs and its atoms, never the primary font's
/// constants. A line that fell back to a second font for one word takes that
/// font's ascent if it is taller; a line with an inline sprite in it takes the
/// sprite's height. Using the primary font's numbers instead is why text with a
/// fallback glyph in it collides with the line above in interfaces that were
/// laid out for one font.
///
/// An empty line still needs a height, and takes the first font's metrics at the
/// layout size: a blank line between two paragraphs is a real line and occupies
/// real space.
fn qiyasat_assatr(maqati: &[MaqtaMashkul], talab: &TalabTakhtit<'_>, hajm: f32) -> (f32, f32) {
    let mut suud = 0.0_f32;
    let mut hubut = 0.0_f32;
    for maqta in maqati {
        suud = suud.max(maqta.suud);
        hubut = hubut.max(maqta.hubut);
    }
    if suud <= 0.0 && hubut <= 0.0 {
        let qiyasat = talab.khutut.awwal().qiyasat(hajm);
        return (qiyasat.suud, qiyasat.hubut);
    }
    (suud, hubut)
}

/// The height of a line box.
///
/// The request's own line height wins when it is set, because a game's interface
/// was laid out against a line height and Taarib's job is to fit into it. When
/// it is not set the font decides, and the font that decides is the tallest one
/// on the line rather than the first one in the chain — same reason as
/// [`qiyasat_assatr`].
///
/// The box is never allowed to be shorter than the ink it contains. A caller who
/// asks for a twelve-pixel line height and puts a twenty-pixel sprite in it gets
/// twenty, because the alternative is a sprite drawn through the line above it.
fn irtifa_assatr(
    maqati: &[MaqtaMashkul],
    talab: &TalabTakhtit<'_>,
    hajm: f32,
    suud: f32,
    hubut: f32,
) -> f32 {
    let asas = talab.khiyarat.irtifa_satr.unwrap_or_else(|| {
        let mut aqsa = 0.0_f32;
        for maqta in maqati {
            if let Some(khatt) = talab.khutut.khatt(maqta.asl.khatt) {
                aqsa = aqsa.max(khatt.qiyasat(maqta.asl.hajm).irtifa_satr);
            }
        }
        if aqsa > 0.0 {
            aqsa
        } else {
            talab.khutut.awwal().qiyasat(hajm).irtifa_satr
        }
    });
    asas.max(suud + hubut)
}

/// The justification mode in force on one line.
///
/// Three things switch it off, and each of them is a line that is already the
/// length it is meant to be:
///
/// - the alignment is not [`Muhadhaha::Dabt`], so nothing was asked to fill;
/// - the line is the paragraph's last, which is the rule every typesetting
///   system has and the reason a two-word closing line is not stretched across
///   a column;
/// - the line was ended by a mandatory break. The author pressed return. A line
///   that ends where it was told to end is not short, it is finished.
const fn namat_assatr(satr: &SatrJari, khiyarat: &KhiyaratTakhtit) -> NamatDabt {
    if satr.akhir || satr.ilzami || !matches!(khiyarat.muhadhaha, Muhadhaha::Dabt) {
        return NamatDabt::Bila;
    }
    khiyarat.dabt
}

/// Where a line's content begins horizontally.
///
/// [`Muhadhaha::Bidaya`] is the *leading* edge, which is the right in a
/// right-to-left paragraph and the left in a left-to-right one. That is the
/// whole point of naming it by role rather than by side: a patch declares that
/// its subtitles are leading-aligned once, and the same declaration is correct
/// for the Arabic build and for the English one.
///
/// An overflowing line gets a negative or over-wide start rather than a clamped
/// one. The text really does extend past its box, and moving it back inside
/// would hide the overflow from the eye at the same moment the report is
/// recording it.
fn muhadhaha_assatr(satr: &SatrJari, talab: &TalabTakhtit<'_>) -> f32 {
    let mutah = talab.ard_mutah.unwrap_or(satr.ard);
    let fadl = mutah - satr.ard;
    let min_alyameen = matches!(satr.ittijah, Ittijah::Yameen);

    match talab.khiyarat.muhadhaha {
        Muhadhaha::Bidaya | Muhadhaha::Dabt => {
            if min_alyameen {
                fadl
            } else {
                0.0
            }
        },
        Muhadhaha::Nihaya => {
            if min_alyameen {
                0.0
            } else {
                fadl
            }
        },
        Muhadhaha::Wasat => fadl * 0.5,
    }
}

/// Fills in the overflow report, or reports that there is nothing to report.
///
/// This is not an error and is not treated as one. It is a measurement the
/// caller asked for: the patch compiler turns it into the overflow list, the
/// review console sorts by [`TaqreerTajawuz::nisba`] because forty pixels over a
/// button is catastrophic and forty pixels over a subtitle is invisible, and the
/// workspace shows it beside the string as the translator types.
///
/// A layout with no width limit can still overflow vertically — a fixed-height
/// dialogue box with unlimited line width is a real shape — and reports it with
/// the widest line's own width standing in for the width available, so that the
/// horizontal part of the report reads as zero overflow rather than as a
/// division by nothing.
fn taqreer(
    talab: &TalabTakhtit<'_>,
    sutur: &[SatrJari],
    ard: f32,
    irtifa: f32,
) -> Option<TaqreerTajawuz> {
    let tajawuz_tul = talab
        .irtifa_mutah
        .is_some_and(|mutah| irtifa > mutah + DIQQA);

    let Some(mutah) = talab.ard_mutah else {
        if !tajawuz_tul {
            return None;
        }
        return Some(TaqreerTajawuz {
            ard,
            ard_mutah: ard,
            irtifa,
            irtifa_mutah: talab.irtifa_mutah,
            awwal_satr: 0,
            adad_sutur: 0,
        });
    };

    let mut awwal: Option<u32> = None;
    let mut adad: u32 = 0;
    for (fahras, satr) in sutur.iter().enumerate() {
        if satr.ard > mutah + DIQQA {
            if awwal.is_none() {
                awwal = Some(u32::try_from(fahras).unwrap_or(u32::MAX));
            }
            adad = adad.saturating_add(1);
        }
    }

    if adad == 0 && !tajawuz_tul {
        return None;
    }
    Some(TaqreerTajawuz {
        ard,
        ard_mutah: mutah,
        irtifa,
        irtifa_mutah: talab.irtifa_mutah,
        awwal_satr: awwal.unwrap_or(0),
        adad_sutur: adad,
    })
}

/// Breaks, truncates, reorders, justifies, measures and aligns — everything a
/// layout is except glyph positions.
///
/// The order of the stages is the order the contract lays down and is not
/// interchangeable. Breaking happens on logical runs because break opportunities
/// are a property of logical text. Truncation happens on logical lines because
/// what it removes is the end of the *text*. Reordering happens per line and
/// after breaking, because rule L2 is defined on a line and a line does not
/// exist until breaking has produced one. Justification happens after
/// reordering, on the runs as they will be drawn. Alignment happens last,
/// because it needs the width justification settled on.
///
/// Direction is taken **per paragraph**, not per layout. Every paragraph
/// resolves its own base direction under P2/P3, so a string holding an Arabic
/// line, a newline and an English line is two paragraphs that legitimately
/// disagree — and the level that disagreement produces is what rule L2 reorders
/// each line at, which margin `Muhadhaha::Bidaya` means for it, and which side
/// an `Ikhtisar` ellipsis lands on. A line never straddles that boundary: a
/// paragraph separator is a mandatory break, and a mandatory break ends a line,
/// so the line's first byte answers for all of it.
///
/// [`BinaSutur::ittijah`] stays the whole-text answer from
/// [`TahleelIttijah::mustawa_asas`], because a caller asking which way a block
/// of text runs is asking about the block.
///
/// # Errors
///
/// As [`iqta_assutur`], [`ikhtasir`] and [`crate::kashida::dubt_satr`].
fn ibni_asasi(
    nass: &str,
    maqati: Vec<MaqtaMashkul>,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
    hajm: f32,
) -> Natija<BinaSutur> {
    let khiyarat = talab.khiyarat;
    // The layout's own direction, which is the first paragraph's and is what
    // `TakhtitNass::ittijah` reports. Individual lines do not take it — see
    // below — but the block as a whole does, because that is what a caller
    // asking "which way does this text run" means.
    let mustawa_asas = tahleel.mustawa_asas();
    let ittijah = Ittijah::min_mustawa(mustawa_asas);
    // Most strings have no paragraph separator in them, and for those the
    // per-line lookup can only ever return the same answer. Checking once is
    // cheaper than asking per line, and the check is a vector length.
    let faqara_wahida = tahleel.adad_faqarat() <= 1;

    let maqtua = iqta_assutur(nass, maqati, furas, talab, makhzan)?;
    if maqtua.is_empty() {
        return Ok(BinaSutur::farigha(ittijah, hajm, talab));
    }

    let adad = maqtua.len();
    let mut sutur: Vec<SatrJari> = Vec::with_capacity(adad);
    for (fahras, satr) in maqtua.into_iter().enumerate() {
        let (suud, hubut) = qiyasat_assatr(&satr.maqati, talab, hajm);
        let irtifa = irtifa_assatr(&satr.maqati, talab, hajm, suud, hubut);
        // Measured while the runs are still in logical order, which is the only
        // order in which "the whitespace at the end" means one thing.
        let dhail = ard_dhail(nass, &satr.maqati, talab);
        let ard = ard_assatr(nass, &satr.maqati, talab, dhail);
        // The paragraph the line starts in decides its direction. A line is
        // wholly inside one paragraph by construction — a mandatory break ends
        // a line and a paragraph separator is a mandatory break — so its first
        // byte answers for all of it.
        let mustawa = if faqara_wahida {
            mustawa_asas
        } else {
            tahleel.mustawa_faqara(satr.mantiqi.start)
        };
        sutur.push(SatrJari {
            maqati: satr.maqati,
            mantiqi: satr.mantiqi,
            mustawa,
            ittijah: Ittijah::min_mustawa(mustawa),
            akhir: fahras.saturating_add(1) == adad,
            ilzami: satr.ilzami,
            ard,
            dhail,
            suud,
            hubut,
            irtifa,
            asas: 0.0,
            bidaya: 0.0,
            dabt: 0.0,
        });
    }

    let mut maqsus = false;
    if matches!(khiyarat.tajawuz, SiyasatTajawuz::Ikhtisar) {
        maqsus = ikhtasir(nass, &mut sutur, talab, tahleel, makhzan, hajm)?;
        // Truncation can drop lines, and the line that is now last is the
        // paragraph's last line — which is what stops justification stretching
        // it after the text below it has been cut away.
        let baqiya = sutur.len();
        for (fahras, satr) in sutur.iter_mut().enumerate() {
            satr.akhir = fahras.saturating_add(1) == baqiya;
        }
    }

    let mut asas_jari = 0.0_f32;
    let mut ard_aqsa = 0.0_f32;
    for satr in &mut sutur {
        // Rule L2, once per line, on runs still in the order the text put them.
        // Shared with the pre-shaping path through `DhuMustawa` rather than
        // reimplemented here: one reorder means the runs a measurement was taken
        // over and the runs a glyph is placed from cannot fall into different
        // orders.
        rattib_basariyan(&mut satr.maqati, satr.mustawa);

        let namat = namat_assatr(satr, khiyarat);
        if let Some(mutah) = talab.ard_mutah
            && !matches!(namat, NamatDabt::Bila)
        {
            let fadl = mutah - satr.ard;
            if fadl > DIQQA {
                let dabt = crate::kashida::dubt_satr(
                    nass,
                    &mut satr.maqati,
                    fadl,
                    namat,
                    makhzan,
                    talab.khutut,
                    talab.nitaqat,
                    khiyarat,
                )?;
                satr.dabt = dabt.muwazza;
                // Re-measured from the runs justification actually produced,
                // never from the width it was aiming at.
                satr.ard = ard_assatr(nass, &satr.maqati, talab, satr.dhail);
            }
        }

        let bidaya = muhadhaha_assatr(satr, talab);
        satr.bidaya = bidaya;

        // Half-leading: whatever the line box has over the ink it contains is
        // split evenly above and below, which is what keeps a line of Arabic
        // optically centred in a box sized for Latin rather than sitting on its
        // floor.
        let zaid = (satr.irtifa - (satr.suud + satr.hubut)).max(0.0);
        satr.asas = asas_jari + zaid * 0.5 + satr.suud;
        asas_jari += satr.irtifa;
        ard_aqsa = ard_aqsa.max(satr.ard);
    }

    let tajawuz = taqreer(talab, &sutur, ard_aqsa, asas_jari);
    Ok(BinaSutur {
        sutur,
        ard: ard_aqsa,
        irtifa: asas_jari,
        ittijah,
        hajm,
        tajawuz,
        maqsus,
        nitaqat: talab.nitaqat.to_vec(),
        khiyarat: talab.khiyarat.clone(),
    })
}

/// Walks the line structure into positioned glyphs.
///
/// The pen starts at the line's aligned beginning and advances by each glyph's
/// contribution, which is its shaped advance plus whatever spacing the request
/// adds — [`ard_harf`], the same function every measurement in this module used,
/// so the width the line was measured at is the width the pen travels by
/// construction.
///
/// Each glyph is then displaced by the offsets shaping gave it. `izaha_s` and
/// `izaha_a` are how `GPOS` places a diacritic on its base and how `curs`
/// carries a joined word along a curve; they move the glyph without moving the
/// pen, which is exactly why a mark's advance is zero and its position is still
/// correct. The vertical axis flips: font offsets are positive upward, and
/// [`Harf::a`] is measured downward from the top of the layout, so the offset is
/// subtracted from the baseline rather than added to it.
///
/// An atom draws nothing. It advances the pen by its own width and leaves a hole
/// the caller fills with whatever the placeholder stood for — a sprite, an icon,
/// a value substituted at runtime. Its width was counted in every measurement,
/// so the hole is exactly the size the layout reserved.
fn mawqi_huruf(nass: &str, bina: BinaSutur, talab: &TalabTakhtit<'_>) -> TakhtitNass {
    let BinaSutur {
        sutur,
        ard,
        irtifa,
        ittijah,
        hajm,
        tajawuz,
        maqsus,
        nitaqat,
        khiyarat,
    } = bina;

    // The layout is positioned against the spans and options it was *built*
    // with, which after shrink-to-fit are not the caller's originals.
    let farii = TalabTakhtit {
        hajm,
        nitaqat: &nitaqat,
        khiyarat: &khiyarat,
        ..*talab
    };

    let mut takhtit = TakhtitNass::farigh(ittijah, hajm);
    takhtit.ard = ard;
    takhtit.irtifa = irtifa;
    takhtit.tajawuz = tajawuz;
    takhtit.maqsus = maqsus;
    takhtit.sutur.reserve(sutur.len());

    for satr in sutur {
        let bidaya_huruf = u32::try_from(takhtit.huruf.len()).unwrap_or(u32::MAX);
        let mut qalam = satr.bidaya;

        for maqta in &satr.maqati {
            if let Some(dharra) = maqta.asl.dharra {
                qalam += dharra.ard;
                continue;
            }
            for harf in &maqta.huruf {
                let (muarrif_nitaq, uslub) = uslub_ind(farii.nitaqat, harf.anqud);
                let izaha_asas = uslub.izaha.unwrap_or(0.0);
                let taqaddum = ard_harf(nass, harf, &farii);
                takhtit.huruf.push(Harf {
                    muarrif: harf.muarrif,
                    s: qalam + harf.izaha_s,
                    a: satr.asas - harf.izaha_a - izaha_asas,
                    taqaddum,
                    anqud: harf.anqud,
                    nitaq: muarrif_nitaq,
                    khatt: maqta.asl.khatt,
                    alama: harf.alama,
                });
                qalam += taqaddum;
            }
        }

        let nihayat_huruf = u32::try_from(takhtit.huruf.len()).unwrap_or(u32::MAX);
        takhtit.sutur.push(SatrMansuq {
            huruf: bidaya_huruf..nihayat_huruf,
            asas: satr.asas,
            bidaya: satr.bidaya,
            ard: satr.ard,
            irtifa: satr.irtifa,
            suud: satr.suud,
            hubut: satr.hubut,
            mantiqi: satr.mantiqi,
            ittijah: satr.ittijah,
            akhir: satr.akhir,
            dabt: satr.dabt,
        });
    }

    takhtit
}

/// Turns shaped runs into a finished layout.
///
/// This is the stage's whole job in one call: validate the request, accumulate
/// candidate lines from real shaped advances, break them where `taqtee` allows
/// and reshape across every break, apply the overflow policy, reorder each line
/// by rule L2, justify what should be justified, and place every glyph.
///
/// The inputs are what the earlier stages produced: `nass` is the clean text
/// with markup already lifted into spans, `maqati` are the shaped runs in
/// logical order, `furas` are the break opportunities computed on that logical
/// text, and `tahleel` is the resolved bidirectional analysis. `makhzan` is the
/// shared font store — the same one that shaped `maqati`, so that reshaping
/// across a break does not re-parse a font that is already prepared.
///
/// # Errors
///
/// - [`KhataSaff::ArdGhayrSalih`] and [`KhataSaff::HajmGhayrSalih`] for a
///   request that cannot be laid out at all.
/// - [`KhataSaff::TaadhurAlqat`] when truncation is asked for in a width that
///   cannot hold even the ellipsis.
/// - Whatever shaping reports for a run that will not reshape across a break, a
///   truncation, or an elongation. Failure is never partial: a paragraph with
///   one unshapeable run is a paragraph that cannot be drawn, and returning the
///   lines that worked would leave a hole in the middle of a sentence.
pub fn rattib_sutur(
    nass: &str,
    maqati: Vec<MaqtaMashkul>,
    furas: &[FursatQat],
    talab: &TalabTakhtit<'_>,
    tahleel: &TahleelIttijah,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<TakhtitNass> {
    let bina = ibni(nass, maqati, furas, talab, tahleel, makhzan)?;
    Ok(mawqi_huruf(nass, bina, talab))
}

/// Measures text without positioning a single glyph.
///
/// This is the metrics query the workspace shows beside a string as it is typed,
/// the compiler runs over every string it compiles, and the preview sizes its
/// boxes from. It runs the **same pipeline** as [`rattib_sutur`] — the same
/// validation, the same run splitting, the same shaping, the same accumulation
/// from real advances, the same break search with the same reshaping across
/// breaks, the same overflow policy, the same reordering, the same
/// justification, the same line metrics — and stops one step short, before
/// glyphs are given positions.
///
/// It is written that way deliberately, and the alternative was rejected on
/// purpose. A measurement path that was its own implementation would be faster
/// and would agree with the layout path on the day it was written. It would then
/// drift: a fix to the break search, a change to how trailing whitespace is
/// counted, a new refusal in justification — each one landing in one path and
/// not the other. The first symptom is a preview that tells a translator a
/// string fits and a game that draws it clipped, which is precisely the failure
/// Boundary 4 of this product exists to make impossible. Sharing the
/// implementation is how "the preview never lies" stays true rather than
/// remaining an intention.
///
/// The one thing this call does that `rattib_sutur` does not is run the stages
/// *before* measurement, because it is handed only a request: it resolves the
/// bidirectional analysis, splits the runs and shapes them itself. It prepares
/// its own font store, so a caller measuring many strings in a loop should use
/// [`qis_bi_makhzan`] and hand the same store to every call — parsing a font's
/// layout tables costs far more than measuring a short string with them.
///
/// # Errors
///
/// As [`rattib_sutur`], plus whatever the bidirectional analysis and the run
/// splitter report for text or spans they refuse.
pub fn qis(talab: &TalabTakhtit<'_>) -> Natija<QiyasNass> {
    let mut makhzan = MakhzanTashkeel::jadeed();
    qis_bi_makhzan(talab, &mut makhzan)
}

/// Measures text, reusing a font store across calls.
///
/// Identical to [`qis`] in every respect except that the prepared fonts survive
/// the call. This is the form the compiler and the workspace use, because they
/// measure thousands of strings against a handful of fonts.
///
/// # Errors
///
/// As [`qis`].
pub fn qis_bi_makhzan(
    talab: &TalabTakhtit<'_>,
    makhzan: &mut MakhzanTashkeel,
) -> Natija<QiyasNass> {
    tahaqquq(talab)?;

    let tahleel = TahleelIttijah::jadeed(talab.nass, talab.khiyarat.ittijah)?;
    if talab.nass.is_empty() {
        return Ok(QiyasNass::default());
    }

    let lugha = lugha_almatlub(talab.nass, talab.khiyarat);
    let mantiqiya = crate::ittijah::qassim(
        talab.nass,
        &tahleel,
        talab.nitaqat,
        talab.khutut,
        lugha,
        talab.hajm,
    )?;
    let mashkula = shakkil_maqati_bi_asalib(
        talab.nass,
        &mantiqiya,
        talab.nitaqat,
        talab.khutut,
        talab.khiyarat,
        makhzan,
    )?;
    let furas = crate::taqtee::furas_qat(talab.nass, lugha);

    let bina = ibni(talab.nass, mashkula, &furas, talab, &tahleel, makhzan)?;
    Ok(min_bina(&bina))
}

/// Reads the extents off a finished line structure.
///
/// Ascent is the first line's and descent is the last line's, because those are
/// the two edges of the block: what a caller aligning a paragraph against a
/// baseline needs is how far the text reaches above its first baseline and below
/// its last, not the largest of either taken from somewhere in the middle.
fn min_bina(bina: &BinaSutur) -> QiyasNass {
    QiyasNass {
        ard: bina.ard,
        irtifa: bina.irtifa,
        suud: bina.sutur.first().map_or(0.0, |awwal| awwal.suud),
        hubut: bina.sutur.last().map_or(0.0, |akhir| akhir.hubut),
        adad_sutur: u32::try_from(bina.sutur.len()).unwrap_or(u32::MAX),
    }
}
