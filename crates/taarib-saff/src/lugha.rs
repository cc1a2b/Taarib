//! اللغة — which script the text is written in, which language it is, and what
//! has to come off it before any of it reaches the pipeline.
//!
//! Everything here runs on the *logical* text, before direction, before
//! segmentation, before shaping. Three questions get answered.
//!
//! **Which script, where.** [`kitabat`] cuts the text into runs of one script
//! each, which is what stops a shaping call from spanning a boundary it cannot
//! shape across. The interesting part is not Arabic or Latin — those are
//! obvious — it is the characters that have no script of their own. Spaces,
//! brackets, most punctuation and the tatweel are `Common`; combining marks are
//! `Inherited`. Left as their own runs they would split `العربية` at every
//! space and sever `ب` from its diacritic, so they are resolved to the script
//! around them.
//!
//! **Which language.** Arabic, Persian and Urdu share a script and do not share
//! its letterforms. [`iktashif_lugha`] discriminates by the characters only one
//! of them uses, and the answer it gives selects `locl` in the shaper — which
//! means it decides what the letters actually look like, not how they are
//! tagged.
//!
//! **What has to be cleaned off.** [`tabee`] is the front door: it destroys
//! presentation forms and normalizes to NFC. [`ihdhif_tashkeel`],
//! [`ihdhif_tatweel`] and [`hawwil_arqam`] are the policy-driven ones, applied
//! only when a patch asks for them, because each of them changes the text
//! rather than repairing it.
//!
//! Every function that returns a [`Cow`] returns [`Cow::Borrowed`] when nothing
//! changed, and the common case — a string that is already clean — allocates
//! nothing. That is not an optimisation for its own sake: these functions run
//! on every string a game draws, several thousand times a second, inside a
//! process whose memory is not ours to waste.

use std::borrow::Cow;
use std::ops::Range;

use icu_normalizer::{ComposingNormalizerBorrowed, DecomposingNormalizerBorrowed};
use icu_properties::props::{GeneralCategory, JoiningType, Script};
use icu_properties::script::ScriptWithExtensions;
use icu_properties::{CodePointMapData, PropertyNamesShort};

use crate::maqta::Kitaba;
use crate::talab::{LughaNass, SiyasatArqam};

/// The elongation character itself.
const TATWEEL: char = '\u{0640}';

/// Scripts whose OpenType tag is not simply the ISO 15924 code lowercased.
///
/// OpenType pads a short script code with spaces where ISO 15924 pads it with
/// letters, and it folds the two kana scripts into one tag. Every other script
/// — including every script this product actually lays out — is the ISO code
/// in lower case, which is the same derivation `HarfBuzz` performs.
const ISTITHNAAT_OPENTYPE: [(Script, [u8; 4]); 6] = [
    (Script::Hiragana, *b"kana"),
    (Script::Katakana, *b"kana"),
    (Script::Lao, *b"lao "),
    (Script::Nko, *b"nko "),
    (Script::Vai, *b"vai "),
    (Script::Yi, *b"yi  "),
];

/// Letters that appear in Urdu and in no other language written in this script.
///
/// The retroflex `tteh`, `ddal` and `rreh`; `noon ghunna`; `heh doachashmee`,
/// which is the aspiration marker Urdu builds half its consonant inventory
/// from; `heh goal` and its hamza form; and `yeh barree` with its hamza form.
/// One of these in a string settles the question — Persian and Arabic use none
/// of them.
const HURUF_URDU: [char; 10] = [
    '\u{0679}', '\u{0688}', '\u{0691}', '\u{06BA}', '\u{06BE}', '\u{06C1}', '\u{06C2}', '\u{06C3}',
    '\u{06D2}', '\u{06D3}',
];

/// Letters and digits that mark text as Persian rather than Arabic.
///
/// `peh`, `tcheh`, `jeh` and `gaf` are the four consonants Persian added to the
/// Arabic inventory; `keheh` and `farsi yeh` are the Persian forms of `kaf` and
/// `yeh`, which sit at their own codepoints precisely because they are drawn
/// differently. Urdu uses all six too, which is why [`HURUF_URDU`] is asked
/// first.
const HURUF_FARISI: [char; 6] = [
    '\u{067E}', '\u{0686}', '\u{0698}', '\u{06A9}', '\u{06AF}', '\u{06CC}',
];

/// The `reh` group: `reh`, `zain`, `rreh`, `jeh`.
///
/// All right-joining, so a letter that connects into one of them is
/// necessarily in a joined form and the letter itself is necessarily final.
const HURUF_REH: [char; 4] = ['\u{0631}', '\u{0632}', '\u{0691}', '\u{0698}'];

/// The `dal` group: `dal`, `thal`, `ddal`. Right-joining like `reh`, with a
/// shorter entry stroke.
const HURUF_DAL: [char; 3] = ['\u{062F}', '\u{0630}', '\u{0688}'];

/// The `waw` group, including the Persian and Uyghur rounded vowels.
const HURUF_WAW: [char; 6] = [
    '\u{0648}', '\u{0624}', '\u{06C6}', '\u{06C7}', '\u{06C8}', '\u{06CB}',
];

/// The `alef` forms, including `alef wasla`.
const HURUF_ALEF: [char; 5] = ['\u{0627}', '\u{0622}', '\u{0623}', '\u{0625}', '\u{0671}'];

/// The `kaf` group, including the Persian `keheh` and `gaf`.
const HURUF_KAF: [char; 6] = [
    '\u{0643}', '\u{06A9}', '\u{06AA}', '\u{06AB}', '\u{06AD}', '\u{06AF}',
];

/// `lam` and the Kurdish `lam with small v`.
const HURUF_LAM: [char; 2] = ['\u{0644}', '\u{06B5}'];

/// `teh marbuta` and its Urdu form.
const HURUF_TEH_MARBUTA: [char; 2] = ['\u{0629}', '\u{06C3}'];

/// The `heh` group: `heh`, `heh doachashmee`, `heh goal`, `ae`.
const HURUF_HEH: [char; 4] = ['\u{0647}', '\u{06BE}', '\u{06C1}', '\u{06D5}'];

/// The `yeh` group, including `alef maqsura`, `farsi yeh` and `yeh barree`.
const HURUF_YEH: [char; 6] = [
    '\u{064A}', '\u{0649}', '\u{0626}', '\u{06CC}', '\u{06D2}', '\u{06D3}',
];

/// How a character joins to its neighbours.
///
/// This is the Unicode `Joining_Type` property, reduced to the four answers
/// that change what Taarib does. It is a property of *characters*, and it is
/// what tells the pipeline whether a cursive connection between two positions
/// can exist at all. Whether one actually formed is a property of the shaped
/// run, and that lives in [`crate::maqta::SifatWasl`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NawWasl {
    /// Dual-joining: connects on both sides, so it takes four contextual
    /// forms — isolated, initial, medial, final. Most Arabic letters.
    Yasil,
    /// Right-joining: connects to what precedes it in logical order and stops
    /// there. `alef`, `dal`, `reh`, `waw` and their groups. A word containing
    /// one of these is drawn as several connected pieces, which is correct and
    /// is not a shaping failure.
    YaqifBaad,
    /// Transparent: a combining mark. It hangs off the letter it attaches to
    /// and takes no part in the joining chain, so two letters on either side of
    /// a mark join to each other as if it were not there.
    Shaffaf,
    /// Non-joining: connects to nothing. Spaces, digits, punctuation, and the
    /// zero-width non-joiner that exists to force a break in the chain.
    La,
}

/// The dominant script of a string.
///
/// Characters with no script of their own do not vote — a sentence of Arabic
/// with four spaces in it is Arabic, not a tie. When nothing in the text has a
/// script, the answer is [`Kitaba::ZYYY`], which is the honest report that the
/// text is digits and punctuation and the shaper should use its default.
///
/// Ties go to the script that appeared first, because a string that is half one
/// script and half another is being read from its beginning.
#[must_use]
pub fn iktashif_kitaba(nass: &str) -> Kitaba {
    let khareeta = CodePointMapData::<Script>::new();
    // Two or three scripts in one string is the realistic maximum, so a linear
    // tally beats a map and costs no allocation in the common case of one.
    let mut tawali: Vec<(Script, u32)> = Vec::new();

    for harf in nass.chars() {
        let kitaba = khareeta.get(harf);
        if !kitaba_haqiqiya(kitaba) {
            continue;
        }
        if let Some(dakhil) = tawali.iter_mut().find(|(sabiqa, _)| *sabiqa == kitaba) {
            dakhil.1 = dakhil.1.saturating_add(1);
        } else {
            tawali.push((kitaba, 1));
        }
    }

    let mut afdal: Option<Script> = None;
    let mut aqsa: u32 = 0;
    for (kitaba, adad) in tawali {
        if adad > aqsa {
            aqsa = adad;
            afdal = Some(kitaba);
        }
    }
    afdal.map_or(Kitaba::ZYYY, kitaba_min_script)
}

/// Cuts the text into runs of one script each, as byte ranges over the input.
///
/// `Common` and `Inherited` characters are resolved to the script around them
/// rather than becoming runs of their own. That resolution is the whole point
/// of this function and it is what makes the two cases that matter come out
/// right:
///
/// - `abc ١٢٣ def` splits into three runs, because Arabic-Indic digits carry
///   `Script=Arabic` and the spaces attach to whichever side they follow.
/// - `العربية (Arabic)` splits into two, because the space and the opening
///   bracket join the Arabic that precedes them and the closing bracket joins
///   the Latin.
///
/// The rule is *the previous script wins*, with a leading run of scriptless
/// characters attaching to the script that follows it — which is what makes a
/// string beginning with a quotation mark shape as one run rather than two.
/// Text with no script at all anywhere comes back as one [`Kitaba::ZYYY`] run.
///
/// The scripts come from `icu_properties`, not from a hand-written table of
/// ranges. A hand-written table is wrong the day it is written and gets more
/// wrong at every Unicode release, and it is wrong in the places nobody tests:
/// the Arabic Extended blocks, the Persian and Urdu additions, the marks.
#[must_use]
pub fn kitabat(nass: &str) -> Vec<(Range<u32>, Kitaba)> {
    let khareeta = CodePointMapData::<Script>::new();
    let mut mukhrajat: Vec<(Range<u32>, Kitaba)> = Vec::new();
    // The run being built: where it starts and what script it is.
    let mut jariya: Option<(usize, Script)> = None;
    // A leading stretch of scriptless characters, held until a real script
    // turns up to claim it.
    let mut muallaq: Option<usize> = None;

    for (mawqi, harf) in nass.char_indices() {
        let kitaba = khareeta.get(harf);
        if !kitaba_haqiqiya(kitaba) {
            if jariya.is_none() && muallaq.is_none() {
                muallaq = Some(mawqi);
            }
            // Inside a run, a scriptless character simply belongs to it: the
            // run's end is fixed by the next real script, not by this one.
            continue;
        }
        let Some((bidaya, sabiqa)) = jariya else {
            // The first real script in the text claims whatever scriptless
            // characters came before it.
            jariya = Some((muallaq.take().unwrap_or(mawqi), kitaba));
            continue;
        };
        if sabiqa != kitaba {
            mukhrajat.push((nitaq(bidaya, mawqi), kitaba_min_script(sabiqa)));
            jariya = Some((mawqi, kitaba));
        }
    }

    let tul = nass.len();
    if let Some((bidaya, kitaba)) = jariya {
        mukhrajat.push((nitaq(bidaya, tul), kitaba_min_script(kitaba)));
    } else if let Some(bidaya) = muallaq {
        mukhrajat.push((nitaq(bidaya, tul), Kitaba::ZYYY));
    }
    mukhrajat
}

/// Which of the languages written in the Arabic script this text is.
///
/// **This decides `locl`, and `locl` decides the letterforms.** Arabic, Persian
/// and Urdu are one script and three orthographies: Persian draws `kaf` without
/// the Arabic head-stroke and `yeh` without its final dots, Urdu draws `heh` in
/// its goal form and `yeh` in its barree form, and a font expresses all of that
/// through `locl` lookups keyed on the OpenType language tag. Shaping Persian
/// as Arabic does not produce a subtle stylistic difference; it produces text a
/// Persian reader will call wrong, and no amount of font choice fixes it
/// because the font is doing exactly what it was told.
///
/// The discrimination is by characteristic characters. Urdu is asked first
/// because its markers — the retroflexes, `noon ghunna`, `heh doachashmee`,
/// `yeh barree` — appear in no other language, while Urdu shares every Persian
/// marker. A string with no marker of either is Arabic. A string with no
/// Arabic script in it at all is [`LughaNass::Latini`] when it has Latin, and
/// [`LughaNass::Tilqai`] otherwise — which carries no OpenType language tag,
/// so the font's default language system is used, which is the correct answer
/// when there is nothing to go on.
///
/// This runs only when the caller left [`LughaNass::Tilqai`] in the request. A
/// declared language always wins, because the person compiling a patch knows
/// what language they translated into and a heuristic does not: an Arabic
/// sentence containing one transliterated name with a `gaf` in it would
/// otherwise be shaped as Persian.
#[must_use]
pub fn iktashif_lugha(nass: &str) -> LughaNass {
    let khareeta = CodePointMapData::<Script>::new();
    let mut arabi = false;
    let mut latini = false;
    let mut farisi = false;

    for harf in nass.chars() {
        if HURUF_URDU.contains(&harf) {
            return LughaNass::Urdu;
        }
        farisi = farisi || HURUF_FARISI.contains(&harf) || raqm_sharqi(harf);
        let kitaba = khareeta.get(harf);
        arabi = arabi || kitaba == Script::Arabic;
        latini = latini || kitaba == Script::Latin;
    }

    if arabi {
        if farisi {
            LughaNass::Farisi
        } else {
            LughaNass::Arabi
        }
    } else if latini {
        LughaNass::Latini
    } else {
        LughaNass::Tilqai
    }
}

/// The front door: canonical characters, canonically composed.
///
/// Two steps, in this order.
///
/// 1. [`hall_ashkal_taqdimiya`] destroys any Unicode Presentation Forms the
///    text arrived with. A translator who pasted a line out of a legacy tool
///    has pasted a compatibility encoding, and everything downstream — search,
///    translation memory, the glossary, shaping itself — needs the canonical
///    characters underneath it.
/// 2. NFC composes the result. This is what turns a pasted `alef` plus
///    `hamza above` into a single `alef with hamza above`, and it is what puts
///    a `shadda` and a `fatha` on one base into the order every font's
///    mark-to-mark tables expect. Text that is not canonically ordered does not
///    fail to shape; it shapes with the marks stacked in the wrong order,
///    which is worse because nothing reports it.
///
/// What this deliberately does **not** do is anything destructive. Tatweel
/// stripping, alef-form unification and digit conversion all change what the
/// text says, and each of them is a per-patch policy with its own function.
/// Unifying alef forms in particular is a *search* transform: it makes `أحمد`
/// and `احمد` match each other, and applying it to text that will be rendered
/// would silently rewrite a translator's orthography.
#[must_use]
pub fn tabee(nass: &str) -> Cow<'_, str> {
    let munaqqa = hall_ashkal_taqdimiya(nass);
    let tarkib = ComposingNormalizerBorrowed::new_nfc();
    match munaqqa {
        Cow::Borrowed(asli) => tarkib.normalize(asli),
        Cow::Owned(mubaddal) => {
            if tarkib.is_normalized(&mubaddal) {
                Cow::Owned(mubaddal)
            } else {
                Cow::Owned(tarkib.normalize(&mubaddal).into_owned())
            }
        },
    }
}

/// Turns Unicode Presentation Forms back into the characters they were made
/// from.
///
/// **This is the one place in the entire product that reads Unicode
/// Presentation Forms, and it exists only to destroy them.** Nothing else in
/// Taarib looks at blocks U+FB50–U+FDFF or U+FE70–U+FEFF, nothing writes into
/// them, and no code path falls back to them when something goes wrong. That is
/// **Decision 1**: contextual forms, lam-alef and every ligature come from the
/// font's own `GSUB` and `GPOS` through a real shaper, and a failure to shape
/// is a reported error rather than a licence to degrade into a compatibility
/// encoding. Presentation forms are the technique every existing Arabic game
/// patch is built on, and refusing them is most of why this one is different.
///
/// They still have to be *read* exactly once, because they arrive: a translator
/// pastes a line out of a tool that produced them, and what they see looks like
/// Arabic. If it were passed through, it would shape as a run of unjoinable
/// isolated characters, it would not match anything in translation memory, it
/// would break the moment a Persian or Urdu letter appeared beside it, and
/// copying it out of the game would yield garbage. So it is decomposed here,
/// immediately, and canonical characters are what leaves.
///
/// The decomposition is `icu_normalizer`'s compatibility decomposition (NFKD)
/// applied per character and restricted to those two blocks, then recomposed
/// with NFC. Restricting it matters: NFKD over the whole string would also flatten
/// superscripts, fullwidth Latin and every other compatibility character in the
/// text, which is a different transform that nobody asked for. Doing it through
/// the normalizer rather than a hand-written table matters too — the table
/// would be seven hundred entries long, it would be wrong in the ligature
/// blocks, and it would need maintaining forever.
#[must_use]
pub fn hall_ashkal_taqdimiya(nass: &str) -> Cow<'_, str> {
    if !nass.chars().any(shakl_taqdimi) {
        return Cow::Borrowed(nass);
    }

    let tahlil = DecomposingNormalizerBorrowed::new_nfkd();
    let tarkib = ComposingNormalizerBorrowed::new_nfc();
    let mut makhraj = String::with_capacity(nass.len());
    for harf in nass.chars() {
        if shakl_taqdimi(harf) {
            makhraj.extend(tahlil.normalize_iter(core::iter::once(harf)));
        } else {
            makhraj.push(harf);
        }
    }
    Cow::Owned(tarkib.normalize(&makhraj).into_owned())
}

/// Removes combining marks.
///
/// This is [`crate::talab::SiyasatTashkeel::Hadhf`] made concrete: diacritics
/// cost vertical room and, at the sizes interface chrome uses, legibility. A
/// game that vocalises its dialogue and strips its menus is making the same
/// choice a publisher makes, and it is a choice — which is why this is a
/// separate function that only runs when a patch asks for it, and never part of
/// [`tabee`].
///
/// Nonspacing marks (`Mn`) and enclosing marks (`Me`) go. Spacing combining
/// marks (`Mc`) stay: they carry a real advance, no Arabic diacritic is one,
/// and removing them from the scripts that do use them would change the width
/// and the meaning of the text rather than just its vocalisation.
#[must_use]
pub fn ihdhif_tashkeel(nass: &str) -> Cow<'_, str> {
    let Some(awwal) = nass
        .char_indices()
        .find(|(_, harf)| alama(*harf))
        .map(|(mawqi, _)| mawqi)
    else {
        return Cow::Borrowed(nass);
    };
    let (raas, dhayl) = nass.split_at_checked(awwal).unwrap_or(("", nass));
    let mut makhraj = String::with_capacity(nass.len());
    makhraj.push_str(raas);
    makhraj.extend(dhayl.chars().filter(|harf| !alama(*harf)));
    Cow::Owned(makhraj)
}

/// Removes the tatweel character.
///
/// A tatweel in the source text is somebody else's justification decision,
/// baked in as a character. It survives into translation memory, it makes two
/// otherwise identical strings compare unequal, and it fights the elongation
/// this engine performs from the font's own tables. Stripping it is how text
/// that came through a presentation-form pipeline gets its shape back.
#[must_use]
pub fn ihdhif_tatweel(nass: &str) -> Cow<'_, str> {
    let Some(awwal) = nass.find(TATWEEL) else {
        return Cow::Borrowed(nass);
    };
    let (raas, dhayl) = nass.split_at_checked(awwal).unwrap_or(("", nass));
    let mut makhraj = String::with_capacity(nass.len());
    makhraj.push_str(raas);
    makhraj.extend(dhayl.chars().filter(|harf| *harf != TATWEEL));
    Cow::Owned(makhraj)
}

/// Rewrites every digit into one numeral system.
///
/// The three systems are three sets of codepoints, not three renderings of one
/// set, and the difference is visible: Arabic-Indic four, five and six
/// (`٤ ٥ ٦`) are drawn differently from Eastern Arabic-Indic four, five and six
/// (`۴ ۵ ۶`). A Persian patch that leaves `٤` in its text shows a Persian
/// reader an Arabic four. That is why this is a per-patch decision recorded in
/// the patch rather than a user setting: an Arabic patch and a Persian patch of
/// the same game legitimately differ, and both are right.
///
/// [`SiyasatArqam::KamaHiya`] leaves the text exactly as the translator wrote
/// it, which is the default and the only correct behaviour for a string that
/// already mixes systems on purpose — a version number beside a quantity.
#[must_use]
pub fn hawwil_arqam(nass: &str, siyasa: SiyasatArqam) -> Cow<'_, str> {
    let Some(asas) = asas_arqam(siyasa) else {
        return Cow::Borrowed(nass);
    };
    let Some(awwal) = nass
        .char_indices()
        .find(|(_, harf)| yataghayyar(*harf, asas))
        .map(|(mawqi, _)| mawqi)
    else {
        return Cow::Borrowed(nass);
    };

    let (raas, dhayl) = nass.split_at_checked(awwal).unwrap_or(("", nass));
    let mut makhraj = String::with_capacity(nass.len());
    makhraj.push_str(raas);
    for harf in dhayl.chars() {
        let mubaddal = qeemat_raqm(harf)
            .and_then(|qeema| char::from_u32(asas.saturating_add(qeema)))
            .unwrap_or(harf);
        makhraj.push(mubaddal);
    }
    Cow::Owned(makhraj)
}

/// How a character joins to its neighbours.
///
/// `Left_Joining` — which exists in Manichaean and Psalter Pahlavi and in no
/// Arabic-script character — is reported as [`NawWasl::La`]. That is the
/// conservative answer rather than the exact one, and conservative is what this
/// property is for: everything downstream uses it to decide whether a cursive
/// joint exists, and the cost of missing a joint is a slightly less elegant
/// line, while the cost of inventing one is a tatweel stretched through a place
/// no scribe would ever stretch.
#[must_use]
pub fn naw_wasl(harf: char) -> NawWasl {
    let naw = CodePointMapData::<JoiningType>::new().get(harf);
    if naw == JoiningType::DualJoining || naw == JoiningType::JoinCausing {
        // Join_Causing is the tatweel itself and the zero-width joiner: both
        // connect on both sides, which is exactly what dual-joining means here.
        NawWasl::Yasil
    } else if naw == JoiningType::RightJoining {
        NawWasl::YaqifBaad
    } else if naw == JoiningType::Transparent {
        NawWasl::Shaffaf
    } else {
        NawWasl::La
    }
}

/// How good an elongation point the joint between two characters is, from `0`
/// for never upward through the classical priority order.
///
/// Arabic justifies by elongating letters, not only by stretching spaces, and
/// where it elongates is not arbitrary. Classical practice stretches particular
/// joints because particular letters are *drawn* to stretch: the flat entry
/// stroke of a `reh`, the horizontal shaft of a `kaf`, the sweep of a `lam`.
/// A renderer that treats every joint as equally stretchable produces the
/// stretched-rubber look that has given kashida justification its bad name —
/// every gap in a line pulled open by the same amount, letters floating away
/// from each other, the word shape destroyed. Ranking is what stops that: a
/// line takes its elongation from the two or three best joints it has, and the
/// rest stay closed.
///
/// The order follows the classical ladder, best first. It is the order the
/// contract states: before a final `reh`-group letter, then after `kaf` or
/// `lam`, then before the `dal` group, then the plain medial connections.
///
/// - **9 — before a final `reh`-group letter.** The longest, flattest entry
///   stroke in the script. It absorbs length without the letter changing
///   shape, which is what a good elongation is.
/// - **8 — after `kaf` or `lam`.** Both are built on a long horizontal that a
///   scribe already varies by hand, so stretching here reads as calligraphy
///   rather than as a gap. This is a different quality of joint from rank 9:
///   there the letter *after* opens up, here the letter *before* extends, and
///   the two are not interchangeable.
/// - **7 — before a final `dal`-group letter.** The same family of joint as
///   `reh` with a shorter run-up, so it takes less length gracefully.
/// - **6 — before a final `waw`-group letter.** Flat entry, shorter still, and
///   the round bowl starts sooner.
/// - **5 — before a final `alef`.** A vertical stroke on a flat approach.
///   Legitimate, and what a line falls back on when it has nothing better.
/// - **4 — before a final `teh marbuta`.** Certainly final, but the closed loop
///   leaves little room before the shape is distorted.
/// - **3 — before `heh` or a `yeh`-group letter.** Common and acceptable, but
///   these are dual-joining, so two characters alone cannot say whether the
///   letter is final, and a medial one is the poorer joint.
/// - **2 — any other real connection.** A plain baseline join. Legal, and last.
/// - **0 — never.**
///
/// Every rank that names a *final* letter can be trusted from two characters
/// alone, because those letters are right-joining: they cannot connect onward,
/// so they are necessarily in final or isolated form. Ranks 3 and 2 cannot,
/// which is why they sit at the bottom — shaping knows more, and
/// [`crate::maqta::SifatWasl::rutba`] is where that knowledge refines this
/// answer against the run that was actually produced.
///
/// Zero is returned when the joint must never be stretched:
///
/// - either side is outside the Arabic script, so there is no cursive joint;
/// - either side is a combining mark, which hangs off a joint rather than
///   forming one — stretching there pulls the mark away from its base;
/// - the preceding character does not join forward, or the following one does
///   not join backward, so the two are not connected in the first place;
/// - the joint is `lam` followed by an `alef`, which is a **mandatory**
///   ligature in Arabic orthography. لا لآ لأ لإ are required forms, and a
///   tatweel between the two letters breaks a ligature the font is obliged to
///   make.
#[must_use]
pub fn rutbat_kashida(qabl: char, baad: char) -> u8 {
    let imtidad = ScriptWithExtensions::new();
    // Script extensions rather than the plain script property: tatweel and the
    // joiners carry `Script=Common` and reach Arabic only through their
    // extensions, and a tatweel already in the text is a legitimate place to
    // add more.
    if !imtidad.has_script(qabl, Script::Arabic) || !imtidad.has_script(baad, Script::Arabic) {
        return 0;
    }

    let (wasl_qabl, wasl_baad) = (naw_wasl(qabl), naw_wasl(baad));
    if wasl_qabl == NawWasl::Shaffaf || wasl_baad == NawWasl::Shaffaf {
        return 0;
    }
    // The letter before must connect forward. `YaqifBaad` stops the chain and
    // `La` never started one, so neither leaves a joint to stretch.
    if wasl_qabl != NawWasl::Yasil || wasl_baad == NawWasl::La {
        return 0;
    }
    if HURUF_LAM.contains(&qabl) && HURUF_ALEF.contains(&baad) {
        return 0;
    }

    // Several rules can apply to one joint — a `kaf` followed by a final `reh`
    // satisfies two — and the joint is as good as the best of them.
    let mut rutba: u8 = 2;
    if HURUF_REH.contains(&baad) {
        rutba = rutba.max(9);
    }
    if HURUF_KAF.contains(&qabl) || HURUF_LAM.contains(&qabl) {
        rutba = rutba.max(8);
    }
    if HURUF_DAL.contains(&baad) {
        rutba = rutba.max(7);
    }
    if HURUF_WAW.contains(&baad) {
        rutba = rutba.max(6);
    }
    if HURUF_ALEF.contains(&baad) {
        rutba = rutba.max(5);
    }
    if HURUF_TEH_MARBUTA.contains(&baad) {
        rutba = rutba.max(4);
    }
    if HURUF_HEH.contains(&baad) || HURUF_YEH.contains(&baad) {
        rutba = rutba.max(3);
    }
    rutba
}

/// Whether a script is one a character can belong to on its own, as opposed to
/// `Common` and `Inherited`, which borrow theirs from their neighbours, and
/// `Unknown`, which has none to borrow.
fn kitaba_haqiqiya(kitaba: Script) -> bool {
    kitaba != Script::Common && kitaba != Script::Inherited && kitaba != Script::Unknown
}

/// Turns an ICU script into the four-byte tag the shaper is handed.
fn kitaba_min_script(kitaba: Script) -> Kitaba {
    for (khass, wasm) in ISTITHNAAT_OPENTYPE {
        if kitaba == khass {
            return Kitaba(wasm);
        }
    }
    let asma = PropertyNamesShort::<Script>::new();
    let Some(ism) = asma.get(kitaba) else {
        return Kitaba::ZYYY;
    };
    // ISO 15924 codes are always four ASCII letters, so this copies exactly
    // four bytes; the space padding only survives for a code that is somehow
    // shorter, which is the same thing OpenType does.
    let mut wasm = *b"    ";
    for (mawqi, bayt) in ism.bytes().take(wasm.len()).enumerate() {
        if let Some(khana) = wasm.get_mut(mawqi) {
            *khana = bayt.to_ascii_lowercase();
        }
    }
    Kitaba(wasm)
}

/// Byte offsets as the crate carries them.
///
/// Every range in this engine is `u32`, so a string beyond four gigabytes could
/// not be laid out whatever this returned. Saturating is the non-panicking
/// answer for a case that cannot arise from any text a game holds.
fn nitaq(bidaya: usize, nihaya: usize) -> Range<u32> {
    let bidaya = u32::try_from(bidaya).unwrap_or(u32::MAX);
    let nihaya = u32::try_from(nihaya).unwrap_or(u32::MAX);
    bidaya..nihaya
}

/// Whether a character sits in one of the two Unicode Presentation Forms
/// blocks. Read in exactly one place, by [`hall_ashkal_taqdimiya`], and only in
/// order to take it apart.
fn shakl_taqdimi(harf: char) -> bool {
    matches!(u32::from(harf), 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Whether a character is a combining mark that occupies no width of its own.
fn alama(harf: char) -> bool {
    // No ASCII character is a combining mark, and most game text is ASCII, so
    // this guard skips the property lookup for nearly every character.
    if harf.is_ascii() {
        return false;
    }
    matches!(
        CodePointMapData::<GeneralCategory>::new().get(harf),
        GeneralCategory::NonspacingMark | GeneralCategory::EnclosingMark
    )
}

/// Whether a character is an Eastern Arabic-Indic digit, which Persian and Urdu
/// use and Arabic does not.
fn raqm_sharqi(harf: char) -> bool {
    matches!(u32::from(harf), 0x06F0..=0x06F9)
}

/// The codepoint of zero in the system a policy selects, or `None` when the
/// policy is to change nothing.
const fn asas_arqam(siyasa: SiyasatArqam) -> Option<u32> {
    match siyasa {
        SiyasatArqam::KamaHiya => None,
        SiyasatArqam::Latini => Some(0x0030),
        SiyasatArqam::Arabi => Some(0x0660),
        SiyasatArqam::Farisi => Some(0x06F0),
    }
}

/// The numeric value of a digit in any of the three systems.
fn qeemat_raqm(harf: char) -> Option<u32> {
    let ramz = u32::from(harf);
    match ramz {
        0x0030..=0x0039 => Some(ramz - 0x0030),
        0x0660..=0x0669 => Some(ramz - 0x0660),
        0x06F0..=0x06F9 => Some(ramz - 0x06F0),
        _ => None,
    }
}

/// Whether converting this character to the system based at `asas` would change
/// it. A digit already in the target system is left exactly where it is, which
/// is what keeps the borrowed path alive for text that is already correct.
fn yataghayyar(harf: char, asas: u32) -> bool {
    qeemat_raqm(harf).is_some_and(|qeema| u32::from(harf) != asas.saturating_add(qeema))
}
