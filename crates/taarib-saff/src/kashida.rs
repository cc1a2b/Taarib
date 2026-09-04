//! الكشيدة — justification by elongating letters.
//!
//! Latin justifies by opening its word spaces. Arabic does not, or rather does
//! not only: a hand-set Arabic line takes most of its extra width from
//! **elongating the connecting strokes between letters**, and only what is left
//! over from the spaces. A line justified the Latin way — spaces pulled to twice
//! their width, words drifting apart, letterforms untouched — is the single most
//! recognisable sign that an Arabic page was set by software that did not know
//! what it was setting.
//!
//! Three things about this module are not negotiable, and each of them is a
//! failure this product exists to remove.
//!
//! ## Candidates come from shaping, never from the characters
//!
//! By the time a line reaches justification the text has already been through
//! `wasl`. The characters are no longer the truth about joining: a `lam`
//! followed by an `alef` is two characters and one glyph, a cluster can hold
//! four characters and produce one ligature, and whether a letter is medial or
//! final was decided by lookups this module cannot see. So the elongation points
//! are read out of [`MaqtaMashkul::mawadi_kashida`], which returns the glyph
//! indices the shaper marked joined-and-stretchable, ranked by
//! [`crate::maqta::SifatWasl::rutba`] — the classical priority order that `wasl`
//! resolved against the run it actually produced.
//!
//! Source characters are read here for exactly one purpose: to **veto** a
//! candidate that shaping proposed. A joint whose byte flanks turn out to be a
//! combining mark, a mandatory `lam`-`alef`, or a pair that does not connect
//! after all is refused. Finding places to stretch and refusing one of them are
//! different operations, and only the second is safe to do from the text.
//!
//! ## Elongation is applied through the font, not by widening an advance
//!
//! The tempting implementation is to add the surplus to a glyph's advance and
//! move on. It is wrong, and it is wrong visibly: two Arabic letters that are
//! joined are connected by a *stroke of ink*. Widening the advance moves the
//! second letter away from the first and draws nothing in between, so the
//! finished line shows a white gap in the middle of a word — the letters no
//! longer touch. That is worse than not justifying at all, because an
//! unjustified line is merely short while a broken join is unreadable.
//!
//! What this module does instead is insert U+0640 ARABIC TATWEEL into the run's
//! source text at the candidate position and **reshape the run**, so that the
//! extra length is produced by the font's own substitution and positioning:
//! either the tatweel glyph itself, which the type designer drew as a connecting
//! stroke with entry and exit points that meet its neighbours, or — where the
//! font offers elongation machinery — a genuinely wider alternate letterform.
//!
//! Fonts that carry `jstf`, or elongation-aware alternates in `GSUB` (`jalt`
//! justification alternates, `falt` final-glyph-on-line alternates, `cswh`
//! contextual swashes, `stch` stretching glyph decomposition), are given the
//! chance to use them: those features are turned on for the reshape, and the
//! font substitutes its designed elongated form. A `kaf` whose horizontal shaft
//! really extends is not the same shape as a `kaf` with a bar glued to it, and a
//! Naskh or Nastaliq face that went to the trouble of drawing the first should
//! not be made to render the second. Where the font offers none of that, the
//! plain tatweel path still comes from the font's own outlines and its own
//! joining lookups, which is the whole difference from moving a number.
//!
//! ## Everything touched is re-measured
//!
//! A reshape changes glyph count, glyph identity and every advance in the run.
//! [`MaqtaMashkul::qis`] is called on each affected run afterwards, and the
//! line's real width is recomputed from those advances. A run that is not
//! re-measured is exactly why justified Arabic drifts off its margin: the layout
//! believes the width it predicted rather than the width the font produced, and
//! the error accumulates line after line down the column.
//!
//! ## What is never stretched
//!
//! A line with no candidates. A run that is an atom — a format placeholder or an
//! inline sprite is one indivisible box and has no joints. A joint adjacent to a
//! combining mark, because the mark is attached to the letter and elongating
//! beside it drags it off its base. A position inside a mandatory ligature,
//! `lam`-`alef` above all: لا is a required form, not a spelling choice, and a
//! tatweel between the two letters asks the font to break an obligation. In
//! every one of those cases the surplus is returned unabsorbed. An unjustified
//! line is correct; a wrongly justified one is not.

use read_fonts::TableProvider as _;
use read_fonts::types::Tag;
use taarib_usus::khata::Natija;

use crate::khatt::{MawridKhatt, SilsilatKhutut};
use crate::lugha::{NawWasl, naw_wasl, rutbat_kashida};
use crate::maqta::{MaqtaMantiqi, MaqtaMashkul};
use crate::talab::{Ittijah, KhiyaratTakhtit, NamatDabt, NitaqUslub, SifaIdafiya};
use crate::wasl::{MakhzanTashkeel, shakkil_maqati_bi_asalib};

/// ARABIC TATWEEL — the connecting stroke, three bytes in UTF-8.
const TATWEEL: char = '\u{0640}';

/// How many bytes one tatweel occupies, so an offset shift is arithmetic rather
/// than a re-encode.
const TUL_TATWEEL: u32 = 3;

/// A hundredth of a pixel.
///
/// Widths here are sums of many `f32` advances, so nothing is ever compared for
/// equality; "the surplus is gone" means it is smaller than this, and at a
/// hundredth of a pixel no display and no atlas can tell the difference.
const DIQQA: f32 = 0.01;

/// The most one joint may absorb, as a fraction of the run's em size.
///
/// This is the constant that decides whether kashida looks like calligraphy or
/// like a mistake. Without a cap, the first and best joint on the line takes the
/// entire surplus and the word it sits in is stretched into a rubber band while
/// every other word stays tight. Three quarters of an em is roughly three
/// tatweels: enough that a single good joint carries real weight, short enough
/// that the letter still reads as itself.
const NISBAT_AQSA_LIL_MAWDI: f32 = 0.75;

/// The most tatweels one joint may take, whatever the width cap allows.
///
/// A second cap, on count rather than width, because a font whose tatweel is
/// unusually narrow would otherwise satisfy the width cap with a dozen of them
/// and produce a visible dotted rhythm instead of one stroke.
const ADAD_AQSA_LIL_MAWDI: u32 = 4;

/// How many times the distribution sweeps the ranked candidates.
///
/// One sweep per rank is not enough when the top ranks hit their per-point cap
/// and the surplus has to descend; the sweep repeats until either the surplus is
/// absorbed or a full pass absorbs nothing. The bound is here so that a
/// pathological line cannot spin.
const ADAD_TAWZI: u32 = 8;

/// How many times an overshoot is corrected by removing the weakest insertion.
///
/// The number of tatweels is computed from a measured unit width, but a font
/// with elongation alternates can answer a tatweel with something wider than the
/// probe suggested. When the reshaped line comes back wider than the room it
/// had, the lowest-ranked insertions are dropped one at a time and the line is
/// reshaped again — a few times at most, because each correction costs a
/// shaping call.
const ADAD_TASHIH: u32 = 3;

/// The `GSUB` features that mean "this font was drawn with elongation in mind".
///
/// `jalt` selects a justification alternate, `falt` a line-final alternate,
/// `cswh` a contextual swash, `stch` a stretching decomposition. A font that
/// defines any of them has forms this module should be asking for rather than
/// settling for a bare tatweel.
const WUSUM_MADD: [[u8; 4]; 4] = [*b"jalt", *b"falt", *b"cswh", *b"stch"];

/// The `jstf` table: the font's own account of how it wants to be justified.
const WASM_JSTF: Tag = Tag::new(b"jstf");

/// What a justification pass did to a line.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NatijatDabt {
    /// How much width was actually absorbed, measured after reshaping rather
    /// than predicted before it.
    pub muwazza: f32,
    /// How much surplus is left over. Non-zero is a normal, correct outcome: it
    /// is what a line with no elongation points reports, and what
    /// [`NamatDabt::KashidaThummaMasafat`] hands on to the spaces.
    pub mutabaqqi: f32,
    /// How many distinct points absorbed something — elongation joints, spaces,
    /// or both when the mode uses both.
    pub adad_mawadi: u32,
}

impl NatijatDabt {
    /// The outcome of absorbing nothing, which is what every refusal returns.
    #[must_use]
    pub const fn la_shay(fadl: f32) -> Self {
        Self { muwazza: 0.0, mutabaqqi: fadl, adad_mawadi: 0 }
    }

    /// Whether the whole surplus was absorbed.
    #[must_use]
    pub fn tamm(self) -> bool {
        self.mutabaqqi <= DIQQA
    }
}

/// Absorbs a line's surplus width according to the justification mode.
///
/// `maqati` is one line's runs, already reordered into visual order, and is
/// modified in place: advances change, glyphs are replaced by reshaped ones, and
/// [`MaqtaMashkul::ard`] is brought back into agreement with them before this
/// function returns. `fadl` is the surplus — the available width minus the
/// line's measured width — and a caller that passes a negative or non-finite
/// value gets an immediate refusal rather than a line squeezed backwards.
///
/// `namat` is passed separately from `khiyarat` rather than read out of it,
/// because the mode in force on a *line* is not always the mode declared for the
/// text: `qiyas` passes [`NamatDabt::Bila`] for the last line of a paragraph and
/// for any line ended by a mandatory break, which is what stops a two-word final
/// line being stretched across the whole column.
///
/// The modes:
///
/// - [`NamatDabt::Bila`] returns immediately with the whole surplus unabsorbed.
/// - [`NamatDabt::Masafat`] stretches spaces only. This is what a Latin run
///   gets, and what an Arabic line falls back to when it offers no joints.
/// - [`NamatDabt::Kashida`] elongates only, and leaves whatever it cannot place.
/// - [`NamatDabt::KashidaThummaMasafat`] elongates first and gives the remainder
///   to the spaces. It is the default for Arabic because it is what a scribe
///   does: take the length out of the letters, and let the spaces settle.
///
/// # Errors
///
/// Returns whatever [`shakkil_maqati_bi_asalib`] reports when a run cannot be
/// reshaped after a tatweel is inserted into it — a font that shaped the run a moment ago
/// and cannot shape it with one more connecting character in it is a font that
/// is lying about its own tables, and the line is left in its pre-justification
/// state rather than half-elongated.
pub fn dubt_satr(
    nass: &str,
    maqati: &mut [MaqtaMashkul],
    fadl: f32,
    namat: NamatDabt,
    makhzan: &mut MakhzanTashkeel,
    khutut: &SilsilatKhutut,
    nitaqat: &[NitaqUslub],
    khiyarat: &KhiyaratTakhtit,
) -> Natija<NatijatDabt> {
    if !fadl.is_finite() || fadl <= DIQQA {
        return Ok(NatijatDabt::la_shay(fadl.max(0.0)));
    }

    match namat {
        NamatDabt::Bila => Ok(NatijatDabt::la_shay(fadl)),
        NamatDabt::Masafat => Ok(mudd_bil_masafat(nass, maqati, fadl)),
        NamatDabt::Kashida => {
            mudd_bil_kashida(nass, maqati, fadl, makhzan, khutut, nitaqat, khiyarat)
        }
        NamatDabt::KashidaThummaMasafat => {
            let bil_madd =
                mudd_bil_kashida(nass, maqati, fadl, makhzan, khutut, nitaqat, khiyarat)?;
            if bil_madd.tamm() {
                return Ok(bil_madd);
            }
            // The spaces are measured against what the letters actually took,
            // not against what the distribution hoped they would take. That is
            // the whole reason the elongation pass reports a measured
            // `muwazza`: a residual computed from a prediction would leave the
            // line short or over-full by the difference.
            let bil_masafat = mudd_bil_masafat(nass, maqati, bil_madd.mutabaqqi);
            Ok(NatijatDabt {
                muwazza: bil_madd.muwazza + bil_masafat.muwazza,
                mutabaqqi: bil_masafat.mutabaqqi,
                adad_mawadi: bil_madd.adad_mawadi.saturating_add(bil_masafat.adad_mawadi),
            })
        }
    }
}

/// Distributes surplus width across the line's word spaces.
///
/// Widening a space's advance is legitimate in a way that widening a letter's
/// advance never is: a space has no ink, so the space *is* its advance and there
/// is nothing to disconnect. That is the entire reason this function is allowed
/// to move numbers while the elongation path has to go back through the font.
///
/// Expandable means a space whose job is to separate words. U+0020 and the
/// general-punctuation spaces qualify. No-break space, narrow no-break space and
/// figure space do not: the first two are used precisely to hold two things at a
/// fixed distance, and the third exists to keep columns of figures aligned, so
/// stretching any of them breaks the reason it was typed.
///
/// Whitespace at either visual end of the line is skipped. Rule L1 has already
/// moved the line's trailing space to the end of the line, and stretching it
/// would push the last word inward from a margin it is supposed to sit against.
///
/// The line's runs are re-measured before this returns. A surplus that is
/// negative or not finite is refused rather than applied, and a line with no
/// expandable space reports the whole surplus unabsorbed — this call cannot
/// fail, because there is nothing in it that can.
#[must_use]
pub fn mudd_bil_masafat(nass: &str, maqati: &mut [MaqtaMashkul], fadl: f32) -> NatijatDabt {
    if !fadl.is_finite() || fadl <= DIQQA {
        return NatijatDabt::la_shay(fadl.max(0.0));
    }

    let mawadi = masafat_assatr(nass, maqati);
    if mawadi.is_empty() {
        return NatijatDabt::la_shay(fadl);
    }

    let hissa = fadl / adad_ila_kasr(u32::try_from(mawadi.len()).unwrap_or(u32::MAX));
    if !hissa.is_finite() || hissa <= 0.0 {
        return NatijatDabt::la_shay(fadl);
    }

    let mut muwazza = 0.0_f32;
    let mut adad: u32 = 0;
    for (fahras_maqta, fahras_harf) in &mawadi {
        let Some(maqta) = maqati.get_mut(*fahras_maqta) else { continue };
        let Some(harf) = maqta.huruf.get_mut(*fahras_harf) else { continue };
        harf.taqaddum_s += hissa;
        muwazza += hissa;
        adad = adad.saturating_add(1);
    }

    for (fahras_maqta, _) in &mawadi {
        if let Some(maqta) = maqati.get_mut(*fahras_maqta) {
            maqta.qis();
        }
    }

    NatijatDabt { muwazza, mutabaqqi: (fadl - muwazza).max(0.0), adad_mawadi: adad }
}

/// The line's expandable spaces, as `(run index, glyph index)` pairs.
///
/// Leading and trailing whitespace is trimmed from both visual ends before
/// anything is returned, so a line that is nothing but spaces yields none.
fn masafat_assatr(nass: &str, maqati: &[MaqtaMashkul]) -> Vec<(usize, usize)> {
    let mut kul: Vec<(usize, usize, bool)> = Vec::new();
    for (fahras_maqta, maqta) in maqati.iter().enumerate() {
        if maqta.asl.dharra.is_some() {
            // An atom is one box. It has no spaces inside it and its width is
            // the caller's, not this stage's to alter.
            continue;
        }
        for (fahras_harf, harf) in maqta.huruf.iter().enumerate() {
            if harf.alama || harf.taqaddum_s <= 0.0 {
                continue;
            }
            let masafa = harf_ind(nass, harf.anqud).is_some_and(masafa_tamtadd);
            kul.push((fahras_maqta, fahras_harf, masafa));
        }
    }

    let awwal = kul.iter().position(|(_, _, masafa)| !*masafa);
    let akhir = kul.iter().rposition(|(_, _, masafa)| !*masafa);
    let (Some(awwal), Some(akhir)) = (awwal, akhir) else {
        return Vec::new();
    };

    kul.get(awwal..=akhir)
        .unwrap_or(&[])
        .iter()
        .filter(|(_, _, masafa)| *masafa)
        .map(|(maqta, harf, _)| (*maqta, *harf))
        .collect()
}

/// Whether a character is a space that justification may widen.
const fn masafa_tamtadd(harf: char) -> bool {
    matches!(
        harf,
        '\u{0020}'
            | '\u{1680}'
            | '\u{2000}'..='\u{2006}'
            | '\u{2008}'..='\u{200A}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

/// The character starting at a byte offset, or [`None`] when the offset is past
/// the end or inside a UTF-8 sequence.
fn harf_ind(nass: &str, mawqi: u32) -> Option<char> {
    let mawqi = usize::try_from(mawqi).ok()?;
    nass.get(mawqi..)?.chars().next()
}

/// The character ending at a byte offset — the one immediately before it.
fn harf_qabl(nass: &str, mawqi: u32) -> Option<char> {
    let mawqi = usize::try_from(mawqi).ok()?;
    nass.get(..mawqi)?.chars().next_back()
}

/// Widens a `u32` count into a float without an `as` cast on a value that could
/// lose precision unnoticed.
const fn adad_ila_kasr(adad: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "counts here are elongation points and spaces on a single line, which never \
                  approach the 2^24 boundary where f32 stops being exact for integers"
    )]
    {
        adad as f32
    }
}

/// Floors a non-negative ratio into a count.
fn adad_min_kasr(qeema: f32) -> u32 {
    if !qeema.is_finite() || qeema <= 0.0 {
        return 0;
    }
    let mahdud = qeema.floor().min(f32::from(u16::MAX));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "floored and clamped into 0..=u16::MAX on the line above, so the value is a \
                  non-negative integer that u32 represents exactly"
    )]
    {
        mahdud as u32
    }
}

/// One elongation point, after shaping proposed it and the text failed to veto
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MawdiMadd {
    /// Index of the run the joint belongs to.
    maqta: usize,
    /// Index of the glyph on the left of the joint, kept so that a tie between
    /// two equally ranked points breaks the same way on every run of the
    /// pipeline — a layout that reorders its own candidates would compile a
    /// patch whose hash changes for no reason.
    harf: usize,
    /// Byte offset in the *original* clean text where the tatweel is inserted.
    mawqi: u32,
    /// The classical rank, from [`crate::maqta::SifatWasl::rutba`].
    rutba: u8,
    /// The run's em size, which sets this point's own width cap.
    hajm: f32,
    /// Width assigned to this point by the distribution, before it is converted
    /// into a whole number of tatweels.
    hissa: f32,
    /// How many tatweels are actually inserted here.
    adad: u32,
}

impl MawdiMadd {
    /// The most this point may absorb: a fraction of its run's em, so a line
    /// mixing a heading-sized span with body text caps each of them against its
    /// own size rather than against the paragraph's.
    fn aqsa(&self) -> f32 {
        NISBAT_AQSA_LIL_MAWDI * self.hajm
    }

    /// The room left under this point's cap.
    fn mutabaqqi(&self) -> f32 {
        (self.aqsa() - self.hissa).max(0.0)
    }
}

/// Every elongation point on the line, best rank first.
///
/// The candidates and their ranks come out of [`MaqtaMashkul::mawadi_kashida`],
/// which reads them from the joining information `wasl` wrote against the glyphs
/// it actually produced. Nothing here looks for a place to stretch.
///
/// What the text is consulted for is the veto, and only the veto. A candidate is
/// dropped when:
///
/// - its run is an atom, which is one indivisible box with no joints in it;
/// - its run's script does not join cursively, so there is no stroke to lengthen
///   — a Latin run has spaces to give and nothing else;
/// - the insertion point is not a character boundary, or falls on the run's own
///   edge, where the joint belongs to the seam between two runs and stretching
///   it would work against the context `wasl` shaped each of them with;
/// - [`rutbat_kashida`] refuses the two characters that actually flank the byte
///   offset. That one call carries three of the four refusals the contract
///   names: a combining mark on either flank, a `lam`-`alef` pair whose ligature
///   is mandatory, and a pair that does not connect at all. Checking it against
///   the real byte flanks rather than the cluster's non-mark edges matters,
///   because a cluster's edge character and its flanking *byte* are not the same
///   character when a diacritic sits between them.
fn mawadi_assatr(nass: &str, maqati: &[MaqtaMashkul]) -> Vec<MawdiMadd> {
    let mut mawadi: Vec<MawdiMadd> = Vec::new();

    for (fahras_maqta, maqta) in maqati.iter().enumerate() {
        if maqta.asl.dharra.is_some() || !maqta.asl.kitaba.tasil() {
            continue;
        }
        for (fahras_harf, rutba) in maqta.mawadi_kashida() {
            let Some(harf) = maqta.huruf.get(fahras_harf) else { continue };
            let mawqi = mawqi_alidkhal(maqta, fahras_harf, harf.anqud);
            if mawqi <= maqta.asl.nitaq.start || mawqi >= maqta.asl.nitaq.end {
                continue;
            }
            let Ok(fahras_bayt) = usize::try_from(mawqi) else { continue };
            if !nass.is_char_boundary(fahras_bayt) {
                continue;
            }
            let (Some(qabl), Some(baad)) = (harf_qabl(nass, mawqi), harf_ind(nass, mawqi)) else {
                continue;
            };
            if matches!(naw_wasl(qabl), NawWasl::Shaffaf)
                || matches!(naw_wasl(baad), NawWasl::Shaffaf)
                || rutbat_kashida(qabl, baad) == 0
            {
                continue;
            }
            mawadi.push(MawdiMadd {
                maqta: fahras_maqta,
                harf: fahras_harf,
                mawqi,
                rutba,
                hajm: maqta.asl.hajm,
                hissa: 0.0,
                adad: 0,
            });
        }
    }

    // One cluster can only ever offer one joint — `wasl` marks the joint on the
    // last non-mark glyph of a cluster and nowhere else — but two runs that were
    // handed overlapping ranges by a caller could still collide, and two
    // tatweels at one offset is not an elongation, it is a stutter.
    mawadi.sort_unstable_by_key(|mawdi| mawdi.mawqi);
    mawadi.dedup_by_key(|mawdi| mawdi.mawqi);

    // Best rank first, and inside a rank in text order, so the distribution and
    // the patch it compiles into are both deterministic.
    mawadi.sort_by(|awwal, thani| {
        thani
            .rutba
            .cmp(&awwal.rutba)
            .then(awwal.maqta.cmp(&thani.maqta))
            .then(awwal.harf.cmp(&thani.harf))
    });
    mawadi
}

/// The byte offset a tatweel goes at, for the joint on a glyph's visual right.
///
/// `wasl` records joining in *visual* terms: [`crate::maqta::SifatWasl::baad`]
/// is the join on the glyph's right-hand side, and that is the joint an
/// elongation opens. Turning that back into a byte offset is where direction
/// re-enters, and getting it backwards is not a subtle bug — it stretches the
/// wrong side of every letter in the language.
///
/// In a right-to-left run the glyphs come back from the shaper in visual order,
/// so the array runs against the text: the cluster to a glyph's right is the one
/// *before* it logically, and the joint between them sits at this glyph's own
/// cluster offset. In a left-to-right run the array and the text run together,
/// so the joint is at the end of this glyph's cluster — which is the offset of
/// the next distinct cluster, or the run's end when there is none.
fn mawqi_alidkhal(maqta: &MaqtaMashkul, fahras: usize, anqud: u32) -> u32 {
    match maqta.asl.ittijah {
        Ittijah::Yameen => anqud,
        Ittijah::Yasar => maqta
            .huruf
            .get(fahras.saturating_add(1)..)
            .unwrap_or(&[])
            .iter()
            .map(|harf| harf.anqud)
            .find(|mawqi| *mawqi > anqud)
            .unwrap_or(maqta.asl.nitaq.end),
    }
}

/// Spreads the surplus over the ranked candidates, best rank first, subject to
/// the per-point cap.
///
/// The rule the contract states is "spread across as many high-rank points as
/// the line offers before descending a rank", and that is exactly what this
/// does: the candidates are already sorted by rank, so they are walked in
/// groups of equal rank, and a group is filled evenly — every point in it taking
/// the same share — before the next group is looked at. A point that reaches its
/// cap drops out and its unclaimed share is re-pooled to the points in the same
/// group that still have room, so an even spread stays even instead of quietly
/// losing width to a cap.
///
/// The sweep repeats. The first pass fills the best rank the line has, and only
/// what that rank cannot hold descends; if the whole ladder is walked and
/// surplus is still left while caps still have room, the ladder is walked again.
/// Both loops are bounded, and a pass that absorbs nothing ends the process —
/// there is no arrangement of caps under which spinning would help.
///
/// Returns the surplus that no point could take.
fn wazzi(mawadi: &mut [MawdiMadd], fadl: f32) -> f32 {
    let mut mutabaqqi = fadl;

    for _ in 0..ADAD_TAWZI {
        if mutabaqqi <= DIQQA {
            break;
        }
        let mut taqaddum = false;
        let mut bidaya = 0_usize;

        while bidaya < mawadi.len() {
            let Some(rutba) = mawadi.get(bidaya).map(|mawdi| mawdi.rutba) else { break };
            let mut nihaya = bidaya.saturating_add(1);
            while mawadi.get(nihaya).is_some_and(|mawdi| mawdi.rutba == rutba) {
                nihaya = nihaya.saturating_add(1);
            }

            if mutabaqqi > DIQQA
                && let Some(majmua) = mawadi.get_mut(bidaya..nihaya)
                && wazzi_ala_majmua(majmua, &mut mutabaqqi)
            {
                taqaddum = true;
            }

            bidaya = nihaya;
        }

        if !taqaddum {
            break;
        }
    }

    mutabaqqi.max(0.0)
}

/// Fills one rank group evenly, re-pooling whatever the caps refuse.
///
/// Returns whether anything at all was absorbed, which is what tells the outer
/// sweep that another pass could still help.
fn wazzi_ala_majmua(majmua: &mut [MawdiMadd], mutabaqqi: &mut f32) -> bool {
    let mut absorbed = false;

    // At most one iteration per point: every pass either exhausts the surplus or
    // pushes at least one point onto its cap, and a point that is capped never
    // comes back into the count.
    for _ in 0..majmua.len().max(1) {
        if *mutabaqqi <= DIQQA {
            break;
        }
        let munfatih: Vec<usize> = majmua
            .iter()
            .enumerate()
            .filter(|(_, mawdi)| mawdi.mutabaqqi() > DIQQA)
            .map(|(fahras, _)| fahras)
            .collect();
        if munfatih.is_empty() {
            break;
        }

        let adad = u32::try_from(munfatih.len()).unwrap_or(u32::MAX);
        let hissa = *mutabaqqi / adad_ila_kasr(adad);
        if !hissa.is_finite() || hissa <= 0.0 {
            break;
        }

        let mut akhadha = 0.0_f32;
        for fahras in &munfatih {
            let Some(mawdi) = majmua.get_mut(*fahras) else { continue };
            let qadr = hissa.min(mawdi.mutabaqqi());
            if qadr <= 0.0 {
                continue;
            }
            mawdi.hissa += qadr;
            akhadha += qadr;
        }

        if akhadha <= DIQQA {
            break;
        }
        *mutabaqqi = (*mutabaqqi - akhadha).max(0.0);
        absorbed = true;
    }

    absorbed
}

/// Elongates the line's letters, and reports what that actually bought.
///
/// The order of operations is the whole of the algorithm, and every step of it
/// exists because the step before it cannot be trusted to predict the next.
///
/// 1. **Collect and veto.** [`mawadi_assatr`] returns the joints shaping marked,
///    ranked, with the ones the text refuses already removed.
/// 2. **Probe.** One tatweel is inserted at the best candidate and that run is
///    reshaped, and the width it gained is measured. That measurement is the
///    unit this line is quantized in — because elongation is not continuous. A
///    tatweel is a glyph with an advance the type designer chose, and a font
///    with elongation alternates answers a tatweel with a form of its own
///    width. Neither number can be derived from the size, so it is measured.
///    When the best candidate gains nothing the next few are tried, and when
///    none of them do the line is left alone: a font that does not respond to a
///    tatweel must not be forced to.
/// 3. **Distribute.** [`wazzi`] spreads the surplus by rank under the per-point
///    cap, in real pixels.
/// 4. **Quantize.** Each point's share is turned into a whole number of
///    tatweels, and the sub-tatweel remainder is carried forward to the next
///    point rather than dropped. Dropping it loses up to one tatweel of width
///    per point, which on a line with six points is most of the surplus.
/// 5. **Apply and re-measure.** Every affected run is reshaped once against a
///    text that carries all of the insertions, and re-measured with
///    [`MaqtaMashkul::qis`]. The width reported back is the difference the fonts
///    produced, never the difference the distribution intended.
/// 6. **Correct.** If the fonts produced *more* than the line had room for —
///    which a font with elongation alternates can, since its answer to a tatweel
///    need not match the probe's — the weakest insertions are removed one at a
///    time and the line is reshaped again, a bounded number of times.
///
/// # Errors
///
/// As [`dubt_satr`]: whatever [`shakkil_maqati_bi_asalib`] reports for the first run that
/// will not reshape. The line is restored to its pre-justification glyphs before
/// the error is returned, so a caller that ignores the failure still has a
/// drawable line rather than a half-elongated one.
fn mudd_bil_kashida(
    nass: &str,
    maqati: &mut [MaqtaMashkul],
    fadl: f32,
    makhzan: &mut MakhzanTashkeel,
    khutut: &SilsilatKhutut,
    nitaqat: &[NitaqUslub],
    khiyarat: &KhiyaratTakhtit,
) -> Natija<NatijatDabt> {
    let mut mawadi = mawadi_assatr(nass, maqati);
    if mawadi.is_empty() {
        return Ok(NatijatDabt::la_shay(fadl));
    }

    // The pristine line. Every attempt starts from this rather than from the
    // previous attempt's output, because reshaping a run that already carries
    // tatweels against a text that carries different ones would compound two
    // sets of insertions into a line nobody asked for.
    let asl: Vec<MaqtaMashkul> = maqati.to_vec();
    let ard_asl = ard_assutur(nass, &asl, khiyarat);

    let khiyarat_madd = khiyarat_almadd(khutut, &asl, khiyarat);

    let Some(wahda) = qis_wahdat_almadd(
        nass,
        &asl,
        &mawadi,
        maqati,
        makhzan,
        khutut,
        nitaqat,
        &khiyarat_madd,
        ard_asl,
    )?
    else {
        radd(maqati, &asl);
        return Ok(NatijatDabt::la_shay(fadl));
    };

    let mutabaqqi_muqaddar = wazzi(&mut mawadi, fadl);
    kammim(&mut mawadi, wahda, mutabaqqi_muqaddar);
    mawadi.retain(|mawdi| mawdi.adad > 0);
    if mawadi.is_empty() {
        radd(maqati, &asl);
        return Ok(NatijatDabt::la_shay(fadl));
    }

    let mut afdal: Option<(Vec<MaqtaMashkul>, f32, u32)> = None;
    for _ in 0..=ADAD_TASHIH {
        let idkhalat = idkhalat_min_mawadi(&mawadi);
        if idkhalat.is_empty() {
            break;
        }
        let tabiq = tabiq_alidkhalat(
            nass,
            &asl,
            &idkhalat,
            maqati,
            makhzan,
            khutut,
            nitaqat,
            &khiyarat_madd,
        );
        if let Err(khata) = tabiq {
            radd(maqati, &asl);
            return Err(khata);
        }

        let ziyada = ard_assutur(nass, maqati, khiyarat) - ard_asl;
        let adad = u32::try_from(mawadi.len()).unwrap_or(u32::MAX);
        // A result that fits is taken immediately. A result that overshoots is
        // remembered only if it is the closest seen so far, so that the
        // correction loop can never make the line worse than the attempt it
        // started from.
        if ziyada <= fadl + DIQQA {
            afdal = Some((maqati.to_vec(), ziyada, adad));
            break;
        }
        let ahsan = afdal.as_ref().is_none_or(|(_, sabiq, _)| {
            (ziyada - fadl).abs() < (*sabiq - fadl).abs()
        });
        if ahsan {
            afdal = Some((maqati.to_vec(), ziyada, adad));
        }
        if !anqis_adna(&mut mawadi) {
            break;
        }
    }

    if let Some((natija, ziyada, adad)) = afdal {
        radd(maqati, &natija);
        Ok(NatijatDabt {
            muwazza: ziyada.max(0.0),
            mutabaqqi: (fadl - ziyada).max(0.0),
            adad_mawadi: adad,
        })
    } else {
        radd(maqati, &asl);
        Ok(NatijatDabt::la_shay(fadl))
    }
}

/// Measures what one tatweel is worth on this line, by inserting one and looking.
///
/// Returns [`None`] when no candidate gains anything, which is the honest answer
/// for a font that swallows a tatweel without widening — some display faces do,
/// substituting it away in `ccmp` or mapping it to a zero-advance glyph — and
/// the signal that this line must not be elongated at all.
///
/// Only the first few candidates are probed. A line whose three best joints all
/// refuse to grow is a line whose font does not elongate, and walking the rest
/// would cost a shaping call each to learn the same thing.
fn qis_wahdat_almadd(
    nass: &str,
    asl: &[MaqtaMashkul],
    mawadi: &[MawdiMadd],
    maqati: &mut [MaqtaMashkul],
    makhzan: &mut MakhzanTashkeel,
    khutut: &SilsilatKhutut,
    nitaqat: &[NitaqUslub],
    khiyarat: &KhiyaratTakhtit,
    ard_asl: f32,
) -> Natija<Option<f32>> {
    /// How many joints are tried before the line is declared unelongatable.
    const ADAD_JARB: usize = 3;

    for mawdi in mawadi.iter().take(ADAD_JARB) {
        tabiq_alidkhalat(
            nass,
            asl,
            &[(mawdi.mawqi, 1)],
            maqati,
            makhzan,
            khutut,
            nitaqat,
            khiyarat,
        )?;
        let wahda = ard_assutur(nass, maqati, khiyarat) - ard_asl;
        if wahda > DIQQA {
            radd(maqati, asl);
            return Ok(Some(wahda));
        }
    }

    radd(maqati, asl);
    Ok(None)
}

/// Turns each point's share of pixels into a whole number of tatweels.
///
/// The remainder below one tatweel is carried to the next point instead of being
/// discarded, and `bank` starts holding whatever the distribution could not
/// place at all, so a line whose caps left surplus over still gets the benefit of
/// it wherever a point has room.
fn kammim(mawadi: &mut [MawdiMadd], wahda: f32, mutabaqqi: f32) {
    if wahda <= DIQQA {
        return;
    }
    let mut bank = mutabaqqi.max(0.0);
    for mawdi in mawadi.iter_mut() {
        let matlub = mawdi.hissa + bank;
        let mut adad = adad_min_kasr(matlub / wahda).min(ADAD_AQSA_LIL_MAWDI);
        // A point that was given a share but rounds to nothing still takes one
        // tatweel when its share is most of one, because a line that rounds every
        // point down absorbs nothing and reports a surplus it could have used.
        if adad == 0 && matlub >= wahda * 0.5 {
            adad = 1;
        }
        let akhadha = wahda * adad_ila_kasr(adad);
        bank = (matlub - akhadha).max(0.0);
        mawdi.adad = adad;
    }
}

/// Removes one tatweel from the lowest-ranked point that still has one, and
/// reports whether anything was left to remove.
///
/// The weakest joint gives up its length first, which is the same principle the
/// distribution filled by: what a line can least afford to lose is the elongation
/// at its best joint.
fn anqis_adna(mawadi: &mut Vec<MawdiMadd>) -> bool {
    let Some(fahras) = mawadi.iter().rposition(|mawdi| mawdi.adad > 0) else {
        return false;
    };
    if let Some(mawdi) = mawadi.get_mut(fahras) {
        mawdi.adad = mawdi.adad.saturating_sub(1);
    }
    mawadi.retain(|mawdi| mawdi.adad > 0);
    !mawadi.is_empty()
}

/// The insertions a candidate set implies, in text order.
fn idkhalat_min_mawadi(mawadi: &[MawdiMadd]) -> Vec<(u32, u32)> {
    let mut idkhalat: Vec<(u32, u32)> = mawadi
        .iter()
        .filter(|mawdi| mawdi.adad > 0)
        .map(|mawdi| (mawdi.mawqi, mawdi.adad))
        .collect();
    idkhalat.sort_unstable_by_key(|(mawqi, _)| *mawqi);
    idkhalat
}

/// The sum of the runs' widths, counted the way the layout stage counts them.
///
/// The request's letter and word spacing are included, and they have to be:
/// elongation adds glyphs to a run, every added glyph takes another helping of
/// letter spacing, and a surplus measured without that would be absorbed and
/// then exceeded by exactly the spacing the new glyphs brought with them. The
/// surplus this module is handed was measured *with* spacing, so what it
/// measures back has to be too, or the two are in different units and the line
/// drifts past its margin by a little more on every line that elongates.
///
/// Per-span letter-spacing overrides are not resolved here — this stage is not
/// handed the span table — so a run that carries one is counted with the
/// request's own value. The layout stage re-measures every justified line
/// against the full span table immediately afterwards, which is what makes the
/// final number right regardless.
fn ard_assutur(nass: &str, maqati: &[MaqtaMashkul], khiyarat: &KhiyaratTakhtit) -> f32 {
    let mut ard = 0.0_f32;
    for maqta in maqati {
        ard += maqta.ard;
        if maqta.asl.dharra.is_some() {
            continue;
        }
        for harf in &maqta.huruf {
            if harf.alama {
                continue;
            }
            ard += khiyarat.tabaud_ahruf;
            if khiyarat.tabaud_kalimat > 0.0
                && harf_ind(nass, harf.anqud).is_some_and(masafa_mahsuba)
            {
                ard += khiyarat.tabaud_kalimat;
            }
        }
    }
    ard
}

/// Whether a character counts as a word space for *measurement*.
///
/// Deliberately wider than [`masafa_tamtadd`], which decides what justification
/// may *stretch*. A no-break space is not stretchable — it exists to hold two
/// things at a fixed distance — but it is still a word space and still carries
/// the request's word spacing, so leaving it out of the measurement would put
/// the layout and this module a few pixels apart on every line that contains
/// one.
const fn masafa_mahsuba(harf: char) -> bool {
    harf.is_whitespace() && harf != '\u{00A0}' && harf != '\u{202F}'
}

/// Restores a line's runs from a saved copy.
fn radd(maqati: &mut [MaqtaMashkul], min: &[MaqtaMashkul]) {
    for (hadaf, masdar) in maqati.iter_mut().zip(min.iter()) {
        hadaf.clone_from(masdar);
    }
}

/// The feature set the reshape runs under.
///
/// When any font on the line carries `jstf`, or defines an elongation-aware
/// alternate in `GSUB`, the four elongation features are turned on for the whole
/// reshape. Turning them on globally is safe in a way that is worth stating: a
/// feature tag a font does not define is a no-op in shaping — there is no lookup
/// to run — so a line that mixes an elongating Naskh face with a plain sans gets
/// the Naskh face's designed forms and leaves the sans exactly as it was.
///
/// A caller's own request wins. Somebody who set `jalt` to zero in
/// [`KhiyaratTakhtit::sifat`] turned it off deliberately, most likely because
/// the game's own interface was measured without it, and justification is not
/// entitled to overrule that.
fn khiyarat_almadd(
    khutut: &SilsilatKhutut,
    maqati: &[MaqtaMashkul],
    khiyarat: &KhiyaratTakhtit,
) -> KhiyaratTakhtit {
    let yadum = maqati
        .iter()
        .filter(|maqta| maqta.asl.dharra.is_none())
        .filter_map(|maqta| khutut.khatt(maqta.asl.khatt))
        .any(|khatt| yadum_almadd(khatt));
    if !yadum {
        return khiyarat.clone();
    }

    let mut madd = khiyarat.clone();
    for wasm in WUSUM_MADD {
        if madd.sifat.iter().any(|sifa| sifa.wasm == wasm) {
            continue;
        }
        madd.sifat.push(SifaIdafiya { wasm, qeema: 1 });
    }
    madd
}

/// Whether a font was drawn with elongation in mind.
///
/// `jstf` is the font telling the layout engine, in its own table, how it wants
/// to be justified. The four `GSUB` tags are the same intent expressed as
/// substitutions. Either is enough to prefer the font's forms over a bare
/// tatweel; neither is required, because the tatweel path is correct on its own.
///
/// A font that cannot be parsed answers `false` rather than failing. This is a
/// preference, not a validation: [`crate::khatt::MawridKhatt::fahs_arabi`] has
/// already decided whether the font is usable, and a parse failure here would
/// mean refusing to justify a line over a question that does not affect whether
/// it can be drawn.
fn yadum_almadd(khatt: &MawridKhatt) -> bool {
    let Ok(font) = khatt.khatt() else { return false };
    if font.table_data(WASM_JSTF).is_some() {
        return true;
    }
    let Ok(gsub) = font.gsub() else { return false };
    let Ok(qaima) = gsub.feature_list() else { return false };
    qaima
        .feature_records()
        .iter()
        .any(|sifa| WUSUM_MADD.iter().any(|wasm| sifa.feature_tag() == Tag::new(wasm)))
}

/// Applies a set of insertions to the line and reshapes everything they touch.
///
/// The line is first restored from `asl`, so this is always a fresh application
/// to pristine runs rather than an addition to whatever the last attempt left
/// behind.
///
/// The reshape runs against a synthetic copy of the whole clean text with the
/// tatweels in it, not against the run's slice alone. That matters twice over:
/// `wasl` hands the shaper the characters on either side of a run as context, so
/// a run shaped out of its sentence would join to nothing at its edges; and
/// cluster indices are byte offsets into the whole text, so a run shaped against
/// its own slice would come back with offsets that mean nothing to anyone.
///
/// Because the synthetic text is longer, every offset past the first insertion
/// has moved. The runs are handed forward-mapped ranges, and every glyph that
/// comes back is mapped straight back into the original text's coordinates
/// before it is stored — a glyph that landed on an inserted tatweel maps to the
/// joint the tatweel was inserted into, which is exactly where a caret belongs:
/// the stroke is the connection between two clusters and belongs to neither.
///
/// # Errors
///
/// Whatever [`shakkil_maqati_bi_asalib`] reports. The caller restores the line.
fn tabiq_alidkhalat(
    nass: &str,
    asl: &[MaqtaMashkul],
    idkhalat: &[(u32, u32)],
    maqati: &mut [MaqtaMashkul],
    makhzan: &mut MakhzanTashkeel,
    khutut: &SilsilatKhutut,
    nitaqat: &[NitaqUslub],
    khiyarat: &KhiyaratTakhtit,
) -> Natija<()> {
    radd(maqati, asl);
    if idkhalat.is_empty() {
        return Ok(());
    }

    let nass_mamdud = nass_bil_madd(nass, idkhalat);

    let mut fahaaris: Vec<usize> = Vec::new();
    let mut manqula: Vec<MaqtaMantiqi> = Vec::new();
    for (fahras, maqta) in asl.iter().enumerate() {
        let yamass = idkhalat.iter().any(|(mawqi, _)| {
            *mawqi > maqta.asl.nitaq.start && *mawqi < maqta.asl.nitaq.end
        });
        if !yamass {
            continue;
        }
        let mut manqul = maqta.asl.clone();
        manqul.nitaq = ila_amam(idkhalat, maqta.asl.nitaq.start)
            ..ila_amam(idkhalat, maqta.asl.nitaq.end);
        fahaaris.push(fahras);
        manqula.push(manqul);
    }
    if manqula.is_empty() {
        return Ok(());
    }

    // Shaped with the style table in hand, not without it: a run under a
    // variable-font weight span was shaped with that weight as a variation
    // coordinate, and reshaping it without one would return the bold word to
    // regular the moment the line justified.
    let mashkula = shakkil_maqati_bi_asalib(
        &nass_mamdud,
        &manqula,
        nitaqat,
        khutut,
        khiyarat,
        makhzan,
    )?;

    for (fahras, mut jadeed) in fahaaris.into_iter().zip(mashkula) {
        let (Some(hadaf), Some(asli)) = (maqati.get_mut(fahras), asl.get(fahras)) else {
            continue;
        };
        for harf in &mut jadeed.huruf {
            harf.anqud = ila_khalf(idkhalat, harf.anqud);
        }
        // The run keeps its original identity — its range in the caller's text,
        // its level, its style, its font, its size. Only the glyphs changed.
        jadeed.asl = asli.asl.clone();
        // The re-measure the contract insists on, at the one place the advances
        // have just been replaced wholesale.
        jadeed.qis();
        *hadaf = jadeed;
    }

    Ok(())
}

/// Builds the clean text with the tatweels inserted.
///
/// `idkhalat` must be sorted by offset and free of duplicates, which
/// [`idkhalat_min_mawadi`] guarantees. An entry that is out of order, past the
/// end, or on a byte that is not a character boundary is skipped rather than
/// applied: the text this produces is handed straight to the shaper, and a
/// tatweel spliced into the middle of a UTF-8 sequence would not be a bad
/// elongation, it would not be text.
fn nass_bil_madd(nass: &str, idkhalat: &[(u32, u32)]) -> String {
    let zaid: usize = idkhalat
        .iter()
        .map(|(_, adad)| usize::try_from(*adad).unwrap_or(0).saturating_mul(3))
        .sum();

    let mut mamdud = String::with_capacity(nass.len().saturating_add(zaid));
    let mut sabiq: usize = 0;

    for (mawqi, adad) in idkhalat {
        let Ok(hadd) = usize::try_from(*mawqi) else { continue };
        if hadd < sabiq || hadd > nass.len() || !nass.is_char_boundary(hadd) {
            continue;
        }
        if let Some(juz) = nass.get(sabiq..hadd) {
            mamdud.push_str(juz);
        }
        for _ in 0..*adad {
            mamdud.push(TATWEEL);
        }
        sabiq = hadd;
    }

    if let Some(juz) = nass.get(sabiq..) {
        mamdud.push_str(juz);
    }
    mamdud
}

/// Maps a byte offset in the original text to its place in the elongated one.
///
/// An insertion *at* an offset pushes the character that was there along, so an
/// offset equal to an insertion point lands after that insertion's tatweels —
/// which is what makes a run whose start sits before an insertion keep its
/// start, and a run whose start sits after one move by the right amount.
fn ila_amam(idkhalat: &[(u32, u32)], mawqi: u32) -> u32 {
    let mut zaid: u32 = 0;
    for (hadd, adad) in idkhalat {
        if *hadd > mawqi {
            break;
        }
        zaid = zaid.saturating_add(TUL_TATWEEL.saturating_mul(*adad));
    }
    mawqi.saturating_add(zaid)
}

/// Maps a byte offset in the elongated text back to the original.
///
/// An offset that falls inside an inserted run of tatweels has no original of
/// its own, and answers with the joint it was inserted into.
fn ila_khalf(idkhalat: &[(u32, u32)], mawqi: u32) -> u32 {
    let mut zaid: u32 = 0;
    for (hadd, adad) in idkhalat {
        let hajm = TUL_TATWEEL.saturating_mul(*adad);
        let qaida = hadd.saturating_add(zaid);
        if mawqi < qaida {
            return mawqi.saturating_sub(zaid);
        }
        if mawqi < qaida.saturating_add(hajm) {
            return *hadd;
        }
        zaid = zaid.saturating_add(hajm);
    }
    mawqi.saturating_sub(zaid)
}
