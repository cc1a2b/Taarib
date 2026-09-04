//! الاتجاه — the Unicode Bidirectional Algorithm, run splitting, and visual
//! reordering.
//!
//! This is the third of the four ways Arabic breaks in software that was never
//! built to accept it. Arabic runs right to left, but the numbers inside it run
//! left to right, an embedded Latin word runs left to right, punctuation and
//! brackets take their direction from whatever surrounds them, and brackets have
//! to mirror. The prevailing answer in existing Arabic game patches — lay the
//! text out left to right and then reverse the string — gets every one of those
//! wrong, and it is the single most visible defect in the patches that exist
//! today. Reversing harder does not fix it. `"لديك 12 من 30"` is not the reverse
//! of anything: the digits keep their own order inside a line that does not.
//!
//! So the real algorithm runs, in full, and this module drives it and exposes
//! exactly three things to the rest of the pipeline:
//!
//! 1. [`TahleelIttijah`] — the resolved embedding levels of the text, per byte,
//!    with the paragraph level and the level runs inside any range.
//! 2. [`qassim`] — the split of the logical text into the runs the shaper is
//!    called on, one call per run.
//! 3. [`rattib_basariyan`] — rule L2, applied to a line's runs after breaking.
//!
//! ## What forces a run boundary, and what must not
//!
//! A boundary is forced when the embedding level changes, when the script
//! changes, when the font chosen for the character changes, or when a style
//! property that actually changes letterforms changes ([`Uslub::yaqta`]).
//!
//! A colour change does **not** force one, and that is not an optimisation. A
//! run boundary is a separate shaping call, and two Arabic letters shaped in two
//! separate calls do not join: the first is shaped as if nothing followed it and
//! the second as if nothing preceded it, so both come out in isolated or final
//! form and the cursive connection between them is gone. A single red word
//! inside a black sentence would visibly disconnect from the words on either
//! side of it. Colour survives instead through the cluster index every glyph
//! carries: the stage that builds [`crate::natija::Harf`] resolves the style span
//! from the glyph's cluster offset, so one shaped run can legitimately span
//! several colour spans. [`crate::maqta::MaqtaMantiqi::uslub`] therefore names
//! the span in force at the run's *first* byte, not the only span it covers.
//!
//! ## Reordering moves runs, never glyphs
//!
//! [`rattib_basariyan`] reverses sequences of *runs*. It never touches the glyphs
//! inside a run, because the shaper already emitted them in visual order — for a
//! right-to-left run HarfRust returns the last letter first. Reversing inside a
//! run reverses text that was already reversed, and the result is a word whose
//! letters read backwards while still being correctly joined, which is the
//! hardest kind of bug to see in a screenshot and the easiest to ship.
//!
//! ## Mirroring is a character lookup, not a transform
//!
//! [`atn`] returns the mirrored *character* for a bracket. Mirroring is applied
//! by looking that character up in the font and drawing its glyph — never by
//! flipping, rotating or otherwise transforming the original glyph. A font's
//! `(` and `)` are two independently drawn shapes whose curves, side bearings and
//! ink extents differ; a mirrored `(` drawn by negating x is a different shape
//! from the `)` the type designer drew, and it will not match the rest of the
//! text it sits in.

use core::ops::Range;
use std::borrow::Cow;

use icu_properties::CodePointMapData;
use icu_properties::props::BidiMirroringGlyph;
use smallvec::SmallVec;
use taarib_usus::khata::Natija;
use unicode_bidi::level::Level;
use unicode_bidi::{BidiClass, BidiInfo};

use crate::khata::KhataSaff;
use crate::khatt::SilsilatKhutut;
use crate::lugha::kitabat;
use crate::maqta::{DhuMustawa, Kitaba, MaqtaMantiqi};
use crate::talab::{Ittijah, IttijahAsas, LughaNass, NitaqUslub, SiyasatArqam, Uslub};

/// Byte offsets are `u32` everywhere in this engine because they cross the C ABI
/// and are stored in patches. A string long enough to truncate cannot reach here
/// — the ABI refuses one — and saturating is the only answer that does not panic.
fn ila32(mawqi: usize) -> u32 {
    u32::try_from(mawqi).unwrap_or(u32::MAX)
}

/// The resolved directional structure of one piece of text.
///
/// Holds the embedding level and the original bidirectional class of every byte,
/// plus the paragraphs the text was divided into and the level each of them
/// resolved to. It deliberately does not hold the text: the levels are what every
/// later stage asks for, and keeping the analysis independent of the string's
/// lifetime is what lets a caller cache it beside the string rather than inside
/// a borrow of it.
#[derive(Debug, Clone)]
pub struct TahleelIttijah {
    /// Embedding level per byte. Multi-byte characters repeat their level, which
    /// is what makes a byte-offset lookup exact rather than approximate.
    mustawayat: Vec<Level>,
    /// Original bidirectional class per byte, before the W, N and I rules
    /// rewrote anything. L1 is defined in terms of these originals, and so is
    /// the digit question in [`hawwil_arqam_bil_siyaq`].
    asnaf: Vec<BidiClass>,
    /// Each paragraph's byte range and the level it resolved to. A game string
    /// with a newline in it is genuinely two paragraphs, and the second one may
    /// resolve to the opposite direction from the first.
    faqarat: Vec<(Range<u32>, u8)>,
    /// The first paragraph's level, which is the layout's base direction.
    asas: u8,
    /// Length of the analysed text in bytes.
    tul: u32,
}

impl TahleelIttijah {
    /// Resolves the text.
    ///
    /// This drives `unicode-bidi`'s full path, not its fast one. The crate offers
    /// `ParagraphBidiInfo` for text known to be a single paragraph and
    /// [`BidiInfo`] for text that is not; game strings carry `\n` constantly —
    /// item descriptions, tooltips, subtitle cues — and each paragraph in one
    /// resolves its own base direction under P2/P3, so the multi-paragraph path
    /// is the only correct one here. `BidiInfo::new` performs the whole
    /// algorithm: P2/P3 for the paragraph level, X1–X8 for the embedding and
    /// override controls and the isolates with their depth stack, W1–W7 for the
    /// weak types including European and Arabic numbers with their separators and
    /// terminators, N0 for bracket pairs over BD16, N1/N2 for the remaining
    /// neutrals, and I1/I2 for the implicit levels.
    ///
    /// The data source is the crate's own hardcoded table rather than
    /// `icu_properties`. `unicode-bidi` can take its bidirectional classes from
    /// ICU through `new_with_data_source`, which would keep one copy of the
    /// property data in the binary, but that adapter lives behind the
    /// `unicode_bidi` feature of `icu_properties` and this workspace does not
    /// enable it. If it is ever enabled, this is the one call that changes.
    ///
    /// # Errors
    ///
    /// Returns [`KhataSaff::UmqIttijahTajawuz`] when the text nests embeddings
    /// and isolates deeper than the algorithm's maximum explicit depth. The
    /// algorithm's own answer to that is to ignore the controls that overflow,
    /// which silently produces a layout nobody asked for; text that nests 125
    /// deep is machine-generated or hostile, and refusing it is more honest than
    /// laying out a clamped approximation of it.
    pub fn jadeed(nass: &str, asas: IttijahAsas) -> Natija<Self> {
        let matlub = match asas {
            IttijahAsas::Tilqai => None,
            IttijahAsas::Yameen => Some(Level::rtl()),
            IttijahAsas::Yasar => Some(Level::ltr()),
        };

        let mut tahleel = BidiInfo::new(nass, matlub);

        // The depth check runs per paragraph, against that paragraph's own
        // resolved level, because X1 restarts the directional status stack at
        // every paragraph and a base level of 1 rather than 0 shifts every level
        // above it by one.
        for faqara in &tahleel.paragraphs {
            if let Some(juz) = nass.get(faqara.range.clone()) {
                tahaqquq_umq(juz, faqara.level.number())?;
            }
        }

        let faqarat: Vec<(Range<u32>, u8)> = tahleel
            .paragraphs
            .iter()
            .map(|faqara| {
                (ila32(faqara.range.start)..ila32(faqara.range.end), faqara.level.number())
            })
            .collect();
        let asas = faqarat.first().map_or_else(
            || matlub.map_or(0, |mustawa| mustawa.number()),
            |(_, mustawa)| *mustawa,
        );

        // Moved out field by field rather than cloned: these two vectors are the
        // whole cost of the analysis, and the layout cache will hold them for the
        // lifetime of the string they describe.
        let mustawayat = core::mem::take(&mut tahleel.levels);
        let asnaf = core::mem::take(&mut tahleel.original_classes);

        Ok(Self { mustawayat, asnaf, faqarat, asas, tul: ila32(nass.len()) })
    }

    /// The base direction of the text as a whole.
    #[must_use]
    pub const fn ittijah_asas(&self) -> Ittijah {
        Ittijah::min_mustawa(self.asas)
    }

    /// The base embedding level of the text as a whole.
    #[must_use]
    pub const fn mustawa_asas(&self) -> u8 {
        self.asas
    }

    /// The resolved embedding level at a byte offset.
    ///
    /// An offset past the end of the text answers with the base level, because a
    /// caller asking about the position after the last character is asking where
    /// a caret goes, and the caret goes where the paragraph starts.
    #[must_use]
    pub fn mustawa(&self, mawqi: u32) -> u8 {
        self.mustawayat.get(mawqi as usize).map_or(self.asas, Level::number)
    }

    /// The level runs inside a byte range, in logical order, with rule L1
    /// applied to the range's own end.
    ///
    /// L1 resets segment separators, paragraph separators, and any run of
    /// whitespace or isolate formatting characters that precedes one of those or
    /// ends the range, back to the paragraph level. That is what puts the trailing
    /// space of a right-to-left line on the left, where the line ends, instead of
    /// stranding it on the right where it would push the first word away from the
    /// margin. Because the rule is defined on a *line*, the range passed here is
    /// expected to be one; passing the whole text applies L1 only at its end,
    /// which is exactly right for text that is never broken.
    ///
    /// L1 is deliberately computed from the original bidirectional classes rather
    /// than the resolved ones, as the rule itself specifies.
    #[must_use]
    pub fn maqati(&self, nitaq: Range<u32>) -> Vec<(Range<u32>, u8)> {
        let bidaya = nitaq.start.min(self.tul);
        let nihaya = nitaq.end.min(self.tul);
        if bidaya >= nihaya {
            return Vec::new();
        }

        let mut maqati: Vec<(Range<u32>, u8)> = Vec::new();
        // Walking backwards is what makes L1 a single pass: "trailing" is only
        // knowable from the end, and every reset in the rule is a run of
        // whitespace seen from the right.
        let mut dhail = true;
        let mut nihayat_maqta = nihaya;
        let mut jari: Option<u8> = None;
        let mut mawqi = nihaya;

        while mawqi > bidaya {
            mawqi -= 1;
            let sanf = self.sanf(mawqi);
            let mustawa = match sanf {
                BidiClass::S | BidiClass::B => {
                    dhail = true;
                    self.mustawa_faqara(mawqi)
                }
                BidiClass::WS
                | BidiClass::LRI
                | BidiClass::RLI
                | BidiClass::FSI
                | BidiClass::PDI
                    if dhail =>
                {
                    self.mustawa_faqara(mawqi)
                }
                _ => {
                    dhail = false;
                    self.mustawa(mawqi)
                }
            };

            match jari {
                Some(sabiq) if sabiq == mustawa => {}
                Some(sabiq) => {
                    maqati.push((mawqi + 1..nihayat_maqta, sabiq));
                    nihayat_maqta = mawqi + 1;
                    jari = Some(mustawa);
                }
                None => jari = Some(mustawa),
            }
        }

        if let Some(sabiq) = jari {
            maqati.push((bidaya..nihayat_maqta, sabiq));
        }
        maqati.reverse();
        maqati
    }

    /// The base embedding level of the paragraph a byte offset falls in.
    ///
    /// Every paragraph resolves its own base direction under P2/P3, from its own
    /// first strong character. A single game string that holds an Arabic line, a
    /// newline, and an English line is two paragraphs that legitimately disagree,
    /// and this is the level each line's alignment and justification edge must
    /// follow: the Arabic line sits against the right margin and stretches from
    /// the right, the English line sits against the left and stretches from the
    /// left. Laying both out from the level of the first one puts the second
    /// against the wrong edge — visibly, on every mixed string in the patch.
    ///
    /// [`Self::mustawa_asas`] is the whole-text answer and stays that: it is the
    /// layout's overall direction, which is what [`crate::natija::TakhtitNass`]
    /// reports in its own `ittijah` field and what a caller means when it asks
    /// which way this block of text runs. Use this one per line, that one per
    /// layout.
    ///
    /// An offset at or past the end of the text answers with the last paragraph,
    /// because that is where a caret sitting after the final character lives.
    #[must_use]
    pub fn mustawa_faqara(&self, mawqi: u32) -> u8 {
        self.faqara_ind(mawqi).map_or(self.asas, |(_, mustawa)| *mustawa)
    }

    /// The base direction of the paragraph a byte offset falls in.
    ///
    /// The parity of [`Self::mustawa_faqara`], which is the same thing said the
    /// way an alignment decision wants to hear it.
    #[must_use]
    pub fn ittijah_faqara(&self, mawqi: u32) -> Ittijah {
        Ittijah::min_mustawa(self.mustawa_faqara(mawqi))
    }

    /// The byte range of the paragraph a byte offset falls in.
    ///
    /// Text that resolved into no paragraphs at all — which only empty text does
    /// — answers with the whole text, so a caller never has to special-case it.
    #[must_use]
    pub fn faqara(&self, mawqi: u32) -> Range<u32> {
        self.faqara_ind(mawqi).map_or(0..self.tul, |(nitaq, _)| nitaq.clone())
    }

    /// How many paragraphs the text resolved into.
    ///
    /// One for any string without a paragraph separator in it, which is most of
    /// them; a caller that sees one can take the whole-text direction and skip
    /// the per-line question entirely.
    #[must_use]
    pub const fn adad_faqarat(&self) -> usize {
        self.faqarat.len()
    }

    /// The original bidirectional class at a byte offset.
    fn sanf(&self, mawqi: u32) -> BidiClass {
        self.asnaf.get(mawqi as usize).copied().unwrap_or(BidiClass::ON)
    }

    /// The paragraph a byte offset falls in, or the last one when the offset is
    /// at or past the end of the text. Every paragraph query goes through here so
    /// that all of them agree about that edge.
    fn faqara_ind(&self, mawqi: u32) -> Option<&(Range<u32>, u8)> {
        self.faqarat
            .iter()
            .find(|(nitaq, _)| mawqi >= nitaq.start && mawqi < nitaq.end)
            .or_else(|| self.faqarat.last())
    }
}

/// Verifies that one paragraph's explicit nesting stays inside the algorithm's
/// maximum depth, rather than letting the controls beyond it be quietly dropped.
///
/// This is X1 through X8 reduced to the one question the pipeline needs an answer
/// to. The status stack carries only the level and whether the entry was pushed
/// by an isolate, because the override status changes what characters resolve to
/// and not how deep they nest.
fn tahaqquq_umq(nass: &str, asas: u8) -> Natija<()> {
    // Every directional formatting character lives in U+202A–U+202E or
    // U+2066–U+2069, and every one of those encodes as three bytes beginning
    // 0xE2. Text without a single 0xE2 byte cannot nest at all, which is the
    // overwhelmingly common case and is worth one linear byte scan to skip.
    if !nass.as_bytes().contains(&0xE2) {
        return Ok(());
    }

    let aqsa = Level::max_explicit_depth();
    let mut kawm: SmallVec<[(u8, bool); 16]> = SmallVec::new();
    kawm.push((asas, false));
    let mut azl_salih: u32 = 0;

    for (mawqi, harf) in nass.char_indices() {
        let sanf = unicode_bidi::bidi_class(harf);
        match sanf {
            BidiClass::RLE
            | BidiClass::LRE
            | BidiClass::RLO
            | BidiClass::LRO
            | BidiClass::RLI
            | BidiClass::LRI
            | BidiClass::FSI => {
                let yameen = match sanf {
                    BidiClass::RLE | BidiClass::RLO | BidiClass::RLI => true,
                    // An FSI takes the direction of the first strong character
                    // inside its own scope, so its depth cost is not knowable
                    // without looking, and guessing here would either refuse
                    // valid text or accept text that overflows.
                    BidiClass::FSI => yameen_dakhil_azl(nass, mawqi + harf.len_utf8()),
                    _ => false,
                };
                let hali = u16::from(kawm.last().map_or(asas, |(mustawa, _)| *mustawa));
                // The least odd level greater than the current one, or the least
                // even one, exactly as X2–X5 define them.
                let jadeed = if yameen { (hali + 1) | 1 } else { (hali + 2) & !1 };
                if jadeed > u16::from(aqsa) {
                    return Err(KhataSaff::UmqIttijahTajawuz { aqsa }.into());
                }
                let azl = matches!(sanf, BidiClass::RLI | BidiClass::LRI | BidiClass::FSI);
                if azl {
                    azl_salih += 1;
                }
                kawm.push((u8::try_from(jadeed).unwrap_or(aqsa), azl));
            }
            BidiClass::PDF
                // A PDF terminates an embedding or an override, and is ignored
                // when the innermost entry is an isolate — an isolate is closed
                // only by its PDI.
                if kawm.len() > 1 && kawm.last().is_some_and(|(_, azl)| !*azl) => {
                    let _ = kawm.pop();
                }
            BidiClass::PDI
                if azl_salih > 0 => {
                    while kawm.len() > 1 && kawm.last().is_some_and(|(_, azl)| !*azl) {
                        let _ = kawm.pop();
                    }
                    if kawm.len() > 1 {
                        let _ = kawm.pop();
                    }
                    azl_salih -= 1;
                }
            _ => {}
        }
    }

    Ok(())
}

/// P2/P3 applied inside an isolate's scope: the direction of the first strong
/// character between an FSI and its matching PDI, skipping any nested isolate
/// whole, and left to right when there is none.
fn yameen_dakhil_azl(nass: &str, min: usize) -> bool {
    let Some(baqi) = nass.get(min..) else {
        return false;
    };
    let mut umq: u32 = 0;
    for harf in baqi.chars() {
        match unicode_bidi::bidi_class(harf) {
            BidiClass::LRI | BidiClass::RLI | BidiClass::FSI => umq += 1,
            BidiClass::PDI => {
                if umq == 0 {
                    return false;
                }
                umq -= 1;
            }
            BidiClass::B => return false,
            BidiClass::L if umq == 0 => return false,
            BidiClass::R | BidiClass::AL if umq == 0 => return true,
            _ => {}
        }
    }
    false
}

/// Splits logical text into the runs the shaper is called on.
///
/// A run is a slice of the text over which everything shaping depends on is
/// constant: the embedding level, the script, the font the characters resolve to,
/// and the parts of a style that change letterforms. One run is one call into
/// HarfRust.
///
/// A boundary is forced when any of those changes, and by nothing else. In
/// particular a colour change does not force one — see the module documentation
/// for why severing a run at a colour boundary disconnects Arabic letters that
/// were joined. An atom (a format placeholder or an inline sprite, carried in
/// [`Uslub::dharra`]) always forms one run of its own and is never shaped.
///
/// Two smaller rules matter as much as the four above:
///
/// - A combining mark, a joining control such as ZWJ or ZWNJ, and a character
///   whose script is inherited never *start* a run. If the font chain would put a
///   mark in a different font from its base letter, the mark stays with the base
///   anyway: a mark shaped in its own run has no base for `GPOS` to attach it to,
///   and a floating tashkeel mark is a worse failure than a mark drawn from the
///   second font in the chain.
/// - Explicit formatting characters stay inside the runs they fall in rather than
///   being deleted. X9 calls for removing them; deleting bytes here would shift
///   every cluster index away from the string the caller handed in, and those
///   indices are what a caret, a selection and the review console all navigate
///   by. They are default-ignorable, HarfRust drops them, and the net effect is
///   the same removal with the offsets left honest.
///
/// # Errors
///
/// Returns [`KhataSaff::HajmGhayrSalih`] when the size is not a positive finite
/// number, [`KhataSaff::NitaqKharij`] when a style span points past the end of
/// the text, [`KhataSaff::HaddNitaqTalif`] when a span boundary falls inside a
/// UTF-8 sequence, and [`KhataSaff::NitaqMutadakhil`] when two spans that set the
/// same property overlap without one containing the other.
pub fn qassim(
    nass: &str,
    tahleel: &TahleelIttijah,
    nitaqat: &[NitaqUslub],
    khutut: &SilsilatKhutut,
    lugha: LughaNass,
    hajm: f32,
) -> Natija<Vec<MaqtaMantiqi>> {
    if !hajm.is_finite() || hajm <= 0.0 {
        return Err(KhataSaff::HajmGhayrSalih { hajm }.into());
    }
    let tul = ila32(nass.len());
    tahaqquq_nitaqat(nass, nitaqat, tul)?;
    if nass.is_empty() {
        return Ok(Vec::new());
    }

    let kitabat_nass = kitabat(nass);
    let hudud = hudud_uslub(nitaqat, tul);

    let mut maqati: Vec<MaqtaMantiqi> = Vec::new();
    let mut fahras_kitaba: usize = 0;
    let mut uslub_sabiq: Option<Uslub> = None;

    for zawj in hudud.windows(2) {
        let (Some(&bidaya), Some(&nihaya)) = (zawj.first(), zawj.get(1)) else {
            continue;
        };
        if bidaya >= nihaya {
            continue;
        }
        let Some(juz) = nass.get(bidaya as usize..nihaya as usize) else {
            continue;
        };
        let (muarrif, uslub) = uslub_fassal(nitaqat, bidaya..nihaya);

        if let Some(dharra) = uslub.dharra {
            let mustawa = tahleel.mustawa(bidaya);
            maqati.push(MaqtaMantiqi {
                nitaq: bidaya..nihaya,
                mustawa,
                ittijah: Ittijah::min_mustawa(mustawa),
                uslub: muarrif,
                khatt: uslub.khatt.unwrap_or(0),
                kitaba: Kitaba::ZYYY,
                lugha,
                dharra: Some(dharra),
                hajm: uslub.hajm.unwrap_or(hajm),
            });
            uslub_sabiq = Some(uslub);
            continue;
        }

        let yaqta_uslub = uslub_sabiq.is_none_or(|sabiq| sabiq.yaqta(&uslub));
        let mut awwal = true;

        for (izaha, harf) in juz.char_indices() {
            let mawqi = bidaya + ila32(izaha);
            let nihayat_harf = mawqi + ila32(harf.len_utf8());
            let mustawa = tahleel.mustawa(mawqi);
            let kitaba = kitaba_ind(&kitabat_nass, &mut fahras_kitaba, mawqi);
            let sanf = unicode_bidi::bidi_class(harf);
            let yulazim = mulazim(sanf, kitaba);
            let khatt = khutut.ikhtiyar(harf, uslub.khatt);

            let yamtadd = !(awwal && yaqta_uslub)
                && maqati.last().is_some_and(|akhir| {
                    akhir.dharra.is_none()
                        && akhir.nitaq.end == mawqi
                        && akhir.mustawa == mustawa
                        && (yulazim || (akhir.kitaba == kitaba && akhir.khatt == khatt))
                });

            if yamtadd {
                if let Some(akhir) = maqati.last_mut() {
                    akhir.nitaq.end = nihayat_harf;
                }
            } else {
                maqati.push(MaqtaMantiqi {
                    nitaq: mawqi..nihayat_harf,
                    mustawa,
                    ittijah: Ittijah::min_mustawa(mustawa),
                    uslub: muarrif,
                    khatt,
                    kitaba,
                    lugha,
                    dharra: None,
                    hajm: uslub.hajm.unwrap_or(hajm),
                });
            }
            awwal = false;
        }

        uslub_sabiq = Some(uslub);
    }

    Ok(maqati)
}

/// Reorders **one line's** runs, **in logical order**, into visual order, by
/// rule L2.
///
/// Both halves of that sentence are load-bearing. Applied to runs that span more
/// than one line, it reorders across a line break that visual order does not
/// cross. Applied twice, it does not undo itself — it produces an order that
/// looks plausible and is wrong. Call it once, per line, on runs that are still
/// in the order the text put them.
///
/// Generic over [`DhuMustawa`] so that the logical runs assembled before
/// shaping and the shaped runs positioned after it go through the same
/// implementation rather than two that can drift apart.
///
/// From the highest level present down to the lowest odd level, every contiguous
/// sequence of runs at or above that level is reversed. Intermediate levels that
/// no run actually carries are included, because L2 is defined over levels and
/// not over the levels that happen to exist: skipping one leaves an embedded
/// left-to-right phrase inside right-to-left text in the wrong place.
///
/// The glyphs inside a run are not touched. They are already in visual order —
/// the shaper emitted a right-to-left run last letter first — and reversing them
/// again produces a word that is correctly joined and reads backwards.
///
/// `mustawa_asas` participates in the lowest-level search because L1 has already
/// reset the line's trailing whitespace to it, so it is a level the line really
/// carries. Including it can only add whole-line reversals in pairs, which cancel,
/// so the result is identical to the textbook formulation over the runs alone.
///
/// This is the same rule `unicode_bidi::BidiInfo::reorder_visual` implements for
/// characters. It is done here instead of there because reordering runs in place
/// allocates nothing, and this runs once per line inside a game's frame.
pub fn rattib_basariyan<T: DhuMustawa>(maqati: &mut [T], mustawa_asas: u8) {
    if maqati.len() < 2 {
        return;
    }

    let mut aqsa = mustawa_asas;
    let mut adna = mustawa_asas;
    for maqta in &*maqati {
        aqsa = aqsa.max(maqta.mustawa());
        adna = adna.min(maqta.mustawa());
    }
    // The lowest odd level at or above the lowest level present.
    let adna_fardi = adna | 1;

    let mut mustawa = aqsa;
    // `adna_fardi` is odd and therefore at least 1, so the loop stops before
    // `mustawa` could underflow past zero.
    while mustawa >= adna_fardi {
        let mut fahras = 0usize;
        while fahras < maqati.len() {
            if maqati.get(fahras).is_some_and(|maqta| maqta.mustawa() >= mustawa) {
                let mut nihaya = fahras + 1;
                while maqati.get(nihaya).is_some_and(|maqta| maqta.mustawa() >= mustawa) {
                    nihaya += 1;
                }
                if let Some(shariha) = maqati.get_mut(fahras..nihaya) {
                    shariha.reverse();
                }
                fahras = nihaya;
            } else {
                fahras += 1;
            }
        }
        mustawa -= 1;
    }
}

/// The mirrored form of a bracket, per the `Bidi_Mirroring_Glyph` property.
///
/// Returns [`None`] for every character that has no mirrored form, which is
/// almost all of them. The answer is a *character*: the caller looks it up in the
/// font and draws the glyph the type designer drew for it. Nothing in this engine
/// mirrors a glyph by transforming it, because a mirrored outline is not the same
/// shape as the mirrored character's own outline — the curves, the side bearings
/// and the ink extents all differ, and the difference is visible next to real
/// text at real sizes.
#[must_use]
pub fn atn(harf: char) -> Option<char> {
    CodePointMapData::<BidiMirroringGlyph>::new().get(harf).mirroring_glyph
}

/// Applies a digit policy to the digits that belong to the text, leaving the ones
/// that belong to an identifier alone.
///
/// The case this exists for: `"اضغط F5 للحفظ"`. Under a policy that maps digits to
/// Arabic-Indic, converting that `5` produces `F٥` — a key that does not exist on
/// any keyboard, printed as an instruction to press it. The same string's other
/// numbers, as in `"لديك 5 أسهم"`, must convert, because those are quantities in
/// Arabic text and an Arabic patch that leaves them in European digits looks
/// half-finished.
///
/// The two are told apart by the bidirectional resolution the analysis already
/// did. Rule W7 changes a European number to L when the nearest preceding strong
/// character is L, and W2 changes it to AN when that character is AL: a digit
/// preceded by Latin is part of a left-to-right token — a key name, a file name,
/// a version, an identifier — and a digit preceded by Arabic, or by nothing at
/// all in a right-to-left paragraph, is part of the sentence. That is the
/// distinction applied here, computed from the original classes over the same
/// backward search the rules define.
///
/// The mapping never touches a strong character, so the answer to that question
/// is the same before and after it. Re-analysing the converted text — which the
/// caller must do before splitting runs, because European and Arabic numbers do
/// not resolve identically — cannot reclassify a digit this function already
/// decided about. A borrowed return means nothing changed and the existing
/// analysis is still exact.
#[must_use]
pub fn hawwil_arqam_bil_siyaq<'n>(
    nass: &'n str,
    siyasa: SiyasatArqam,
    tahleel: &TahleelIttijah,
) -> Cow<'n, str> {
    if matches!(siyasa, SiyasatArqam::KamaHiya) || nass.is_empty() {
        return Cow::Borrowed(nass);
    }

    let mut makhraj: Option<String> = None;
    // The last strong direction seen, reset at every paragraph and at every
    // isolate boundary, which is what bounds the backward search W2 and W7
    // describe.
    let mut qawi: Option<bool> = None;
    let mut faqara_haliya = u32::MAX;

    for (izaha, harf) in nass.char_indices() {
        let mawqi = ila32(izaha);
        let bidayat = tahleel.faqara(mawqi).start;
        if bidayat != faqara_haliya {
            faqara_haliya = bidayat;
            qawi = None;
        }

        let sanf = tahleel.sanf(mawqi);
        match sanf {
            BidiClass::L => qawi = Some(false),
            BidiClass::R | BidiClass::AL => qawi = Some(true),
            BidiClass::LRI | BidiClass::RLI | BidiClass::FSI | BidiClass::PDI | BidiClass::B => {
                qawi = None;
            }
            _ => {}
        }

        let badeel = qeemat_raqm(harf)
            .filter(|_| qawi.unwrap_or_else(|| tahleel.mustawa_faqara(mawqi) % 2 == 1))
            .and_then(|qeema| raqm_min(qeema, siyasa))
            .filter(|jadeed| *jadeed != harf);

        // Nothing is allocated until the first digit that actually changes: up to
        // that point the text is still exactly the caller's own, and returning it
        // borrowed is what tells the caller its analysis is still valid.
        if let Some(jadeed) = badeel {
            let mabni = makhraj.get_or_insert_with(|| {
                let mut mabni = String::with_capacity(nass.len() + 8);
                if let Some(sabiq) = nass.get(..izaha) {
                    mabni.push_str(sabiq);
                }
                mabni
            });
            mabni.push(jadeed);
        } else if let Some(mabni) = makhraj.as_mut() {
            mabni.push(harf);
        }
    }

    makhraj.map_or(Cow::Borrowed(nass), Cow::Owned)
}

/// The value of a digit in any of the three digit sets this product maps between.
fn qeemat_raqm(harf: char) -> Option<u32> {
    let ramz = u32::from(harf);
    match harf {
        '0'..='9' => Some(ramz - 0x0030),
        '\u{0660}'..='\u{0669}' => Some(ramz - 0x0660),
        '\u{06F0}'..='\u{06F9}' => Some(ramz - 0x06F0),
        _ => None,
    }
}

/// The digit of a given value in the set a policy names.
const fn raqm_min(qeema: u32, siyasa: SiyasatArqam) -> Option<char> {
    let asas = match siyasa {
        SiyasatArqam::KamaHiya => return None,
        SiyasatArqam::Latini => 0x0030,
        SiyasatArqam::Arabi => 0x0660,
        SiyasatArqam::Farisi => 0x06F0,
    };
    char::from_u32(asas + qeema)
}

/// Whether a character must stay in the run of the character before it.
fn mulazim(sanf: BidiClass, kitaba: Kitaba) -> bool {
    matches!(sanf, BidiClass::NSM | BidiClass::BN) || kitaba == Kitaba::ZINH
}

/// The script covering a byte offset, walking the script runs in step with the
/// text rather than searching them per character.
fn kitaba_ind(kitabat: &[(Range<u32>, Kitaba)], fahras: &mut usize, mawqi: u32) -> Kitaba {
    while kitabat.get(*fahras).is_some_and(|(nitaq, _)| nitaq.end <= mawqi) {
        *fahras += 1;
    }
    kitabat
        .get(*fahras)
        .filter(|(nitaq, _)| mawqi >= nitaq.start && mawqi < nitaq.end)
        .map_or(Kitaba::ZYYY, |(_, kitaba)| *kitaba)
}

/// Every offset at which the effective style can change, plus the two ends of the
/// text, sorted and deduplicated. Between two consecutive entries the set of
/// spans in force cannot change, so the effective style is computed once per
/// interval instead of once per character.
fn hudud_uslub(nitaqat: &[NitaqUslub], tul: u32) -> Vec<u32> {
    let mut hudud: Vec<u32> = Vec::with_capacity(nitaqat.len() * 2 + 2);
    hudud.push(0);
    hudud.push(tul);
    for nitaq in nitaqat {
        if nitaq.tul == 0 {
            continue;
        }
        if nitaq.bidaya < tul {
            hudud.push(nitaq.bidaya);
        }
        if nitaq.nihaya() < tul {
            hudud.push(nitaq.nihaya());
        }
    }
    hudud.sort_unstable();
    hudud.dedup();
    hudud
}

/// The style in force over an interval, and the span it is attributed to.
///
/// Spans nest, and an inner span overrides an outer one property by property, so
/// they are merged from the outermost inward. The interval is attributed to the
/// innermost span covering it, which is the one a caller means when it asks which
/// span a glyph came from. Text no span covers is attributed to span zero.
fn uslub_fassal(nitaqat: &[NitaqUslub], nitaq: Range<u32>) -> (u16, Uslub) {
    let mut mughattiya: SmallVec<[(u32, usize); 8]> = SmallVec::new();
    for (fahras, nitaq_uslub) in nitaqat.iter().enumerate() {
        if nitaq_uslub.tul > 0
            && nitaq_uslub.bidaya <= nitaq.start
            && nitaq_uslub.nihaya() >= nitaq.end
        {
            mughattiya.push((nitaq_uslub.tul, fahras));
        }
    }
    // Longest first: the outermost span is merged first and every span inside it
    // overrides what it set. Equal lengths keep declaration order, so the span
    // written last wins, which is how every markup dialect this product parses
    // behaves.
    mughattiya.sort_unstable_by(|awwal, thani| thani.0.cmp(&awwal.0).then(awwal.1.cmp(&thani.1)));

    let mut uslub = Uslub::default();
    let mut muarrif = 0u16;
    for (_, fahras) in &mughattiya {
        if let Some(nitaq_uslub) = nitaqat.get(*fahras) {
            uslub = dam_uslub(uslub, nitaq_uslub.uslub);
            muarrif = nitaq_uslub.id;
        }
    }
    (muarrif, uslub)
}

/// Lays one style over another. Anything the inner span sets replaces the outer
/// value; anything it leaves unset inherits.
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

/// Checks the style spans against the text before anything is built from them.
fn tahaqquq_nitaqat(nass: &str, nitaqat: &[NitaqUslub], tul: u32) -> Natija<()> {
    for nitaq in nitaqat {
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
        for hadd in [nitaq.bidaya, nihaya] {
            if !nass.is_char_boundary(hadd as usize) {
                return Err(KhataSaff::HaddNitaqTalif { id: nitaq.id, mawqi: hadd }.into());
            }
        }
    }

    for (fahras, awwal) in nitaqat.iter().enumerate() {
        for thani in nitaqat.iter().skip(fahras + 1) {
            if tadakhul_juzii(awwal, thani) && yashtarik(&awwal.uslub, &thani.uslub) {
                return Err(KhataSaff::NitaqMutadakhil { awwal: awwal.id, thani: thani.id }.into());
            }
        }
    }

    Ok(())
}

/// Whether two spans cross: they share text, and each has text the other does
/// not. Nesting is not crossing — an inner span overriding an outer one is how
/// markup works — but two spans that each begin inside the other have no answer
/// to which of them applies where they meet.
const fn tadakhul_juzii(awwal: &NitaqUslub, thani: &NitaqUslub) -> bool {
    let (a1, a2) = (awwal.bidaya, awwal.nihaya());
    let (t1, t2) = (thani.bidaya, thani.nihaya());
    let yataqata = a1 < t2 && t1 < a2;
    let yahtawi = (a1 <= t1 && t2 <= a2) || (t1 <= a1 && a2 <= t2);
    yataqata && !yahtawi
}

/// Whether two styles set at least one of the same properties.
const fn yashtarik(awwal: &Uslub, thani: &Uslub) -> bool {
    (awwal.khatt.is_some() && thani.khatt.is_some())
        || (awwal.wazn.is_some() && thani.wazn.is_some())
        || (awwal.maail && thani.maail)
        || (awwal.hajm.is_some() && thani.hajm.is_some())
        || (awwal.lawn.is_some() && thani.lawn.is_some())
        || (awwal.tabaud.is_some() && thani.tabaud.is_some())
        || (awwal.izaha.is_some() && thani.izaha.is_some())
        || (awwal.dharra.is_some() && thani.dharra.is_some())
}
