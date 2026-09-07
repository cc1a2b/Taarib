//! العلامات — quality flags: what a translation is measured against before a
//! reviewer's time is spent on it.
//!
//! Every flag in this module is a *measurement or a recorded fact*, never a
//! guess. The distinction is the module's whole discipline, and it exists
//! because of a specific failure mode: a flag list a reviewer has learned to
//! distrust is worse than no flag list at all. The first estimated overflow
//! warning that turns out to fit, the first "untranslated" flag on a character
//! name, the first synthesised confidence score — each one teaches the reviewer
//! to dismiss the column, and from then on the *real* flags ship unread.
//!
//! So the rules are:
//!
//! - **Overflow risk is measured by Phase 1** ([`taarib_saff`]), shaping the
//!   real Arabic in the real font at the real size against the real captured
//!   width. Where the constraint is unknown — no captured width, no font size,
//!   no font chain at the call site — **no flag is produced.** Absent, not
//!   estimated. A character-count heuristic here would be a guess wearing a
//!   measurement's clothes.
//! - **Provider confidence is carried only when the provider reported one.**
//!   Machine-translation APIs that return no score get no
//!   [`AlamJawda::ThiqaMunkhafida`], ever. Deriving a score from length or
//!   perplexity-shaped proxies would put a number the provider never said into
//!   a column labelled with the provider's name.
//! - **Broken placeholders come from [`crate::hima`]'s refusal**, via
//!   [`KhataTarjama::khalal_himaya`], and only from there. The flag marks a
//!   string that *failed* — the broken text was never accepted, the source is
//!   still untranslated, and the flag tells the reviewer why.
//! - **Glossary violations are folded in from the glossary module**
//!   ([`crate::masrad`]), which owns term matching. Re-implementing matching
//!   here would be a second opinion about what a term is, and the two opinions
//!   would disagree on exactly the inflected form that made it matter.
//! - **The machine-only flag is computed from the review record**
//!   ([`SijillMuraja::aali_faqat`]), which is the one place that state cannot
//!   be forged, and never inferred from the text.
//!
//! ## Per-string and cross-string flags are two passes, on purpose
//!
//! [`ihsib_alamat_nass`] computes everything knowable from one entry.
//! [`AlamJawda::TarjamaMutanaqida`] cannot be known from one entry — it is a
//! property of two — so it is computed by [`ihsib_tanaqud`] over the whole
//! table. The two passes own disjoint sets of variants and each replaces only
//! its own: a per-string recompute after an edit must not erase the
//! inconsistency flags the project pass found, and the project pass must not
//! erase the overflow flag a measurement produced. See [`thabbit_alamat`] and
//! [`thabbit_tanaqud`] for the exact split.

use std::collections::{BTreeMap, BTreeSet};

use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId};
use taarib_saff::{KhiyaratTakhtit, Saff, SilsilatKhutut, TalabTakhtit};

use crate::hima::{ALAMAT_IGHLAQ, FATIHA, KHATIMA};
use crate::khata::KhataTarjama;
use crate::muraja_dakhiliya::SijillMuraja;

/// The longest all-uppercase Latin word treated as an acronym.
///
/// Five. `HP`, `MP`, `EXP`, `ATK`, `CRIT`, `SPEED` — game stat abbreviations
/// run short, are kept in Latin by essentially every published Arabic game
/// translation, and flagging them would fire on half the status screen. A
/// six-letter all-caps word is far more likely a shouted untranslated word
/// (`ATTACK!`) than an abbreviation.
pub const AQSA_TUL_IKHTIZAL: usize = 5;

/// The thresholds every judgement in this module is made against.
///
/// Carried as data rather than buried in the functions, because the workshop
/// exposes them in settings: a project translating a stat-heavy RPG legitimately
/// wants a looser Latin-word threshold than a visual novel does. The defaults
/// are the values the doc comment on each field argues for.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AtabatAlamat {
    /// Reported confidence at or below which [`AlamJawda::ThiqaMunkhafida`] is
    /// raised. Zero to one.
    ///
    /// Three quarters. Providers that report scores at all calibrate them
    /// toward the top of the range, so the interesting tail — the strings a
    /// reviewer should see first — sits below 0.75 in practice. Applied only to
    /// scores a provider actually reported; see [`alam_thiqa`].
    pub hadd_thiqa: f32,
    /// How many counted Latin words, in a target that *does* contain Arabic,
    /// raise [`AlamJawda::NassLatiniMutabaqqi`].
    ///
    /// Two. One surviving Latin word inside an Arabic sentence is very often a
    /// deliberate keep — a romanized term, an item code — and flagging every
    /// one would train reviewers to ignore the flag. Two independent leftovers
    /// almost never are deliberate. See [`alam_latini`] for what "counted"
    /// excludes and why the rule errs toward silence.
    pub hadd_latini_makhlut: u32,
    /// The shortest source, in characters, the length-ratio check applies to.
    ///
    /// Ten. Short strings swing wildly and legitimately: `OK` becomes `حسنًا`
    /// at ratio 2.5 and `Yes` becomes `نعم` at 1.0, and neither number means
    /// anything. Below this length the ratio is not evaluated at all — absent,
    /// not estimated, the same discipline as the overflow flag.
    pub adna_tul_nisba: usize,
    /// Below this target-to-source character ratio, the translation is
    /// suspiciously short.
    ///
    /// 0.3: Arabic runs somewhat shorter than English in characters, but a
    /// translation under a third of its source's length has usually dropped a
    /// clause.
    pub adna_nisba: f32,
    /// Above this ratio, suspiciously long — usually a model that explained
    /// instead of translating, or answered twice.
    pub aqsa_nisba: f32,
}

impl Default for AtabatAlamat {
    fn default() -> Self {
        Self {
            hadd_thiqa: 0.75,
            hadd_latini_makhlut: 2,
            adna_tul_nisba: 10,
            adna_nisba: 0.3,
            aqsa_nisba: 3.0,
        }
    }
}

/// A confidence score as a provider reported it — or the fact that it did not.
///
/// Mirrors the two fields the provider result carries (`thiqa`, `maqisa`) so
/// that this module needs nothing else from [`crate::muzawwidun`]. The batch
/// runner builds one of these from each provider result; nothing else
/// constructs them, because nothing else *has* a reported score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThiqaMublagha {
    /// The score, zero to one, as reported.
    pub qeema: f32,
    /// Whether the provider actually measured it.
    ///
    /// `false` means the number beside it is a filler the transport layer had
    /// to put *somewhere* — `DeepL` and the raw MT endpoints return no score —
    /// and [`alam_thiqa`] treats that as "no score", not as "score of that
    /// value". A default of 0.0 read as a real score would flag every string
    /// from a provider that never said anything.
    pub maqisa: bool,
}

/// What the overflow measurement needs: the engine and the fonts.
///
/// Passed in rather than constructed here because [`Saff`] is a mutable shaping
/// cache the caller already owns, and because *having* one is exactly the
/// condition under which the overflow flag may be computed at all. A call site
/// with no fonts loaded — the batch runner, mid-flight — passes [`None`] and
/// gets no overflow flags, which is the confidence discipline doing its job
/// rather than a missing feature.
#[derive(Debug)]
pub struct MudkhalatQiyas<'a> {
    /// The layout engine, with its shaping caches.
    pub saff: &'a mut Saff,
    /// The font chain the game's text is drawn with.
    pub khutut: &'a SilsilatKhutut,
}

/// A protection refusal, as a quality flag on the failed string.
///
/// The one hard flag: [`AlamJawda::NasqMaksur`] means the string is **not
/// translated** — [`crate::hima::istaridd`] refused the reply, the source was
/// left intact, and this flag is the reviewer-visible record of why. It is
/// produced from exactly the failures [`KhataTarjama::khalal_himaya`] names,
/// and from nothing else; a flag computed by re-scanning the text for
/// placeholders would be a second verifier that disagrees with the first.
///
/// `dharrat` are the string's own atoms ([`MudkhalNass::dharrat`]), used when
/// the failure implicates the whole string rather than one token — a span
/// count over the ceiling, a span outside its text — because "which atoms are
/// at risk" is then *all of them*, and an empty list would render as a flag
/// that names nothing.
///
/// Returns [`None`] for any failure that is not a protection refusal: a rate
/// limit, a refused input, an unreachable provider. Those fail the string or
/// the run, but nothing about the *markup* is broken, and saying so would be
/// false.
#[must_use]
pub fn alam_min_khata(khata: &KhataTarjama, dharrat: &[String]) -> Option<AlamJawda> {
    if !khata.khalal_himaya() {
        return None;
    }
    let mafqud = match khata {
        KhataTarjama::RamzMafqud { ramz, .. }
        | KhataTarjama::RamzMukarrar { ramz, .. }
        | KhataTarjama::RamzDakhil { ramz, .. } => vec![ramz.clone()],
        KhataTarjama::RamzTalif { juz, .. } => vec![juz.clone()],
        KhataTarjama::RamzMaqlub { fahras } => {
            vec![format!(
                "{FATIHA}{fahras}{KHATIMA}\u{2026}{FATIHA}{ALAMAT_IGHLAQ}{fahras}{KHATIMA}"
            )]
        },
        KhataTarjama::RumuzKathira { .. } | KhataTarjama::NitaqKharij { .. } => {
            if dharrat.is_empty() {
                vec![khata.to_string()]
            } else {
                dharrat.to_vec()
            }
        },
        // Unreachable while this match and `khalal_himaya` agree. If a new
        // protection failure is added there and not here, the correct behaviour
        // is a flag that quotes the failure rather than a silently dropped one
        // — losing the flag is the bug this arm exists to prevent.
        _ => vec![khata.to_string()],
    };
    Some(AlamJawda::NasqMaksur { mafqud })
}

/// The confidence flag, from a score the provider actually reported.
///
/// [`None`] when `maqisa` is false — the score field then holds a filler, not
/// a measurement, and this module does not launder fillers into flags. Also
/// [`None`] when the reported score clears the threshold, which is most of the
/// time. The value is clamped into `0.0..=1.0` before comparison because a
/// provider quoting percentages instead of fractions has happened, and a flag
/// reading "confidence 8700%" discredits the whole column.
#[must_use]
pub fn alam_thiqa(thiqa: Option<&ThiqaMublagha>, atabat: &AtabatAlamat) -> Option<AlamJawda> {
    let mublagha = thiqa?;
    if !mublagha.maqisa {
        return None;
    }
    let qeema = mublagha.qeema.clamp(0.0, 1.0);
    if qeema <= atabat.hadd_thiqa {
        Some(AlamJawda::ThiqaMunkhafida { qeema })
    } else {
        None
    }
}

/// The empty-target flag.
///
/// Raised when the source has visible content and the target exists but has
/// none — a provider that returned an empty string, or a model whose whole
/// reply was consumed by tokens. An entry with **no** target at all is simply
/// untranslated, which is a workflow state and not a defect, so it produces
/// nothing here.
#[must_use]
pub fn alam_farigh(masdar: &str, hadaf: Option<&str>) -> Option<AlamJawda> {
    let hadaf = hadaf?;
    if !masdar.trim().is_empty() && hadaf.trim().is_empty() {
        Some(AlamJawda::Farigh)
    } else {
        None
    }
}

/// The machine-only flag, from the review record and nowhere else.
///
/// [`SijillMuraja::aali_faqat`] is the single source of truth for "a machine
/// produced this and no human has read it": the record's approved state cannot
/// be forged, so its machine-only state can be believed. When no review record
/// is available at the call site the answer is unknown, and unknown produces
/// **no flag** — the same absent-not-estimated discipline as the overflow
/// check. Guessing "probably machine" from the presence of a provider name
/// would re-flag every string a human already corrected.
#[must_use]
pub const fn alam_aali(muraja: Option<&SijillMuraja>) -> Option<AlamJawda> {
    match muraja {
        Some(sijill) if sijill.aali_faqat() => Some(AlamJawda::AaliyaBilaMuraja),
        _ => None,
    }
}

/// The length-ratio flag.
///
/// Measured in **characters**, never bytes. Arabic is two bytes per letter in
/// UTF-8 and Latin is one, so a byte ratio would report every faithful
/// translation as twice its source and the flag would fire on the whole
/// project. Sources shorter than [`AtabatAlamat::adna_tul_nisba`] are not
/// evaluated at all — see that field for why — and an absent or empty target
/// produces nothing here because [`alam_farigh`] already owns that case.
#[must_use]
pub fn alam_nisba(masdar: &str, hadaf: Option<&str>, atabat: &AtabatAlamat) -> Option<AlamJawda> {
    let hadaf = hadaf?;
    if hadaf.trim().is_empty() {
        return None;
    }
    let tul_masdar = masdar.chars().count();
    if tul_masdar < atabat.adna_tul_nisba {
        return None;
    }
    let nisba = kasr(hadaf.chars().count(), tul_masdar)?;
    if nisba < atabat.adna_nisba || nisba > atabat.aqsa_nisba {
        Some(AlamJawda::NisbaShadha { nisba })
    } else {
        None
    }
}

/// A ratio of two character counts, as `f32`.
///
/// [`None`] when the denominator is zero, which the callers' guards already
/// exclude but which arithmetic must not depend on.
fn kasr(bast: usize, maqam: usize) -> Option<f32> {
    if maqam == 0 {
        return None;
    }
    Some(ila_f32(bast) / ila_f32(maqam))
}

/// A character count as `f32`.
#[expect(
    clippy::cast_precision_loss,
    reason = "counts here are string character counts, far below f32's 2^24 exact-integer \
              range for any string a game ships; and the result feeds a ratio threshold \
              where the 24th significant bit could not change the comparison"
)]
const fn ila_f32(qeema: usize) -> f32 {
    qeema as f32
}

/// How a Latin word in the target is classified before counting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SinfKalima {
    /// All uppercase and short: `HP`, `EXP`. Kept in Latin by convention.
    Ikhtizal,
    /// First letter uppercase, the rest lowercase: `Marcus`, `Steam`. The shape
    /// of a proper noun.
    IsmAlam,
    /// Everything else — the shape of ordinary untranslated prose.
    Aadiya,
}

/// One Latin word found in a text, with its classification.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KalimaLatiniya {
    /// The word as written.
    nass: String,
    /// Its shape class.
    sinf: SinfKalima,
}

/// Whether a character is a Latin letter for the purposes of word counting.
///
/// ASCII letters plus the Latin-1 Supplement and Latin Extended-A/B letters, so
/// `café` and `Škoda` count as one word each rather than splitting at the
/// accented letter and being counted twice.
fn harf_latini(harf: char) -> bool {
    harf.is_ascii_alphabetic()
        || (('\u{00C0}'..='\u{024F}').contains(&harf) && harf.is_alphabetic())
}

/// Whether a character may join two Latin letter runs into one word.
///
/// The apostrophe and the hyphen, so `don't` and `well-worn` are one word, not
/// two — counting them as two would push a single leftover phrase over the
/// threshold and make the count read as larger than what the reviewer sees.
const fn wasil_kalima(harf: char) -> bool {
    matches!(harf, '\'' | '\u{2019}' | '-')
}

/// Whether the text contains any Arabic-script character.
///
/// The Arabic block, its supplement and extensions, and the presentation-form
/// blocks — the last two because text pasted from a legacy source can carry
/// presentation forms before normalization sees it, and "no Arabic at all" must
/// not be concluded about a string that is visibly Arabic on screen.
fn fih_arabi(nass: &str) -> bool {
    nass.chars().any(|harf| {
        matches!(harf,
            '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{08A0}'..='\u{08FF}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
        )
    })
}

/// Every Latin word in a text, in order, with its shape class.
///
/// Single letters are not words: an `x` in `x3`, a hotkey `S`, a variable
/// leftover — none of them is evidence of untranslated prose, and counting
/// them would flag half the interface strings of any game with a crafting
/// menu.
fn kalimat_latiniya(nass: &str) -> Vec<KalimaLatiniya> {
    let mut kalimat = Vec::new();
    let mut hali = String::new();
    let mut ahruf = nass.chars().peekable();

    while let Some(harf) = ahruf.next() {
        if harf_latini(harf) {
            hali.push(harf);
            continue;
        }
        let yasil = wasil_kalima(harf)
            && !hali.is_empty()
            && ahruf.peek().copied().is_some_and(harf_latini);
        if yasil {
            hali.push(harf);
            continue;
        }
        akhtim_kalima(&mut hali, &mut kalimat);
    }
    akhtim_kalima(&mut hali, &mut kalimat);
    kalimat
}

/// Closes the word being built, classifying and keeping it if it qualifies.
fn akhtim_kalima(hali: &mut String, kalimat: &mut Vec<KalimaLatiniya>) {
    if hali.chars().count() < 2 {
        hali.clear();
        return;
    }
    let sinf = sannif_kalima(hali);
    kalimat.push(KalimaLatiniya {
        nass: std::mem::take(hali),
        sinf,
    });
}

/// Classifies a word by its letter-case shape.
fn sannif_kalima(kalima: &str) -> SinfKalima {
    let huruf: Vec<char> = kalima.chars().filter(|harf| harf.is_alphabetic()).collect();
    if !huruf.is_empty()
        && huruf.len() <= AQSA_TUL_IKHTIZAL
        && huruf.iter().all(|harf| harf.is_uppercase())
    {
        return SinfKalima::Ikhtizal;
    }
    let mut baqi = huruf.iter();
    let awwal_kabir = baqi.next().is_some_and(|harf| harf.is_uppercase());
    if awwal_kabir && baqi.all(|harf| harf.is_lowercase()) {
        return SinfKalima::IsmAlam;
    }
    SinfKalima::Aadiya
}

/// The untranslated-remainder flag.
///
/// Latin words surviving inside the target are counted, and the counting is
/// deliberately conservative, because the false positive here is expensive: a
/// proper noun left in Latin is *legitimate and common* in published Arabic
/// game text — `تحدث إلى Marcus` is how professional translations write it —
/// and a flag that fires on every character name teaches reviewers to dismiss
/// the flag, after which the real untranslated clauses ship unread. The rule
/// therefore errs toward silence.
///
/// A word is **exempt** from the count when:
///
/// - it is a short all-uppercase acronym (`HP`, `EXP`; see
///   [`AQSA_TUL_IKHTIZAL`]) — kept in Latin by near-universal convention;
/// - it does not appear in the source at all (case-folded) — a Latin word the
///   translation *introduced* is a deliberate choice, a romanization, not a
///   leftover;
/// - it is `TitleCase` **and the target also contains Arabic** — the shape of a
///   name dropped into an Arabic sentence, which is the legitimate pattern.
///
/// The `TitleCase` exemption is withdrawn when the target contains no Arabic at
/// all, because a target with no Arabic in it is the strongest signal that
/// translation simply did not happen, and `TitleCase` menu labels — `New Game`,
/// `Load Game` — are the classic strings it happens to. There the threshold
/// drops to one counted word. The residual false positive is a target that is
/// a lone brand name in full (`Steam`), which is judged worth one reviewer
/// glance against silently passing every untouched menu.
///
/// In a mixed Arabic/Latin target the threshold is
/// [`AtabatAlamat::hadd_latini_makhlut`] counted words. The cost of the
/// `TitleCase` exemption is that the sentence-initial capitalized word of an
/// untranslated clause is not counted — accepted, because untranslated clauses
/// have more than one word and the rest of them count.
#[must_use]
pub fn alam_latini(masdar: &str, hadaf: Option<&str>, atabat: &AtabatAlamat) -> Option<AlamJawda> {
    let hadaf = hadaf?;
    if hadaf.trim().is_empty() {
        return None;
    }
    let kalimat = kalimat_latiniya(hadaf);
    if kalimat.is_empty() {
        return None;
    }

    let fi_almasdar: BTreeSet<String> = kalimat_latiniya(masdar)
        .into_iter()
        .map(|kalima| kalima.nass.to_lowercase())
        .collect();
    let makhlut = fih_arabi(hadaf);

    let mut adad = 0_u32;
    for kalima in &kalimat {
        if kalima.sinf == SinfKalima::Ikhtizal {
            continue;
        }
        if !fi_almasdar.contains(&kalima.nass.to_lowercase()) {
            continue;
        }
        if makhlut && kalima.sinf == SinfKalima::IsmAlam {
            continue;
        }
        adad = adad.saturating_add(1);
    }

    let hadd = if makhlut {
        atabat.hadd_latini_makhlut
    } else {
        1
    };
    if adad >= hadd.max(1) {
        Some(AlamJawda::NassLatiniMutabaqqi { adad })
    } else {
        None
    }
}

/// The overflow-risk flag, measured or absent — never estimated.
///
/// Calls Phase 1's [`Saff::qis`] on the translation's clean text, in the real
/// font chain at the string's own captured size, and compares the widest
/// resulting line against the width the original text was drawn into. The flag
/// carries the measured pixels, which is what lets a reviewer trust it.
///
/// **Every one of these must be known, or no flag is produced:**
///
/// - [`QuyudNass::aqsa_ard`] — the captured width. Without it there is no
///   constraint to exceed.
/// - [`QuyudNass::hajm_khatt`] — the size the game draws at. Measuring at a
///   guessed size produces a guessed width.
/// - The engine and fonts ([`MudkhalatQiyas`]) at the call site.
/// - A non-empty target to measure.
///
/// This is the phase's confidence discipline and it is not negotiable: an
/// entry whose constraint was never captured gets **no** overflow flag, not a
/// character-count approximation of one. The batch runner, which has no fonts
/// in flight, passes no [`MudkhalatQiyas`] and the workshop re-computes flags
/// once fonts are loaded.
///
/// A measurement that *fails* — a font that cannot shape the run, a width the
/// engine refuses — also produces no flag, logged at debug level: an
/// unmeasurable string is unmeasured, and inventing a verdict for it would be
/// the estimation this module refuses.
///
/// ## The measurement is a floor, and that is the honest direction
///
/// Style spans and placeholder atoms are not reconstructed into the layout
/// request: an atom's rendered width (`{name}` substituted at runtime) is
/// unknowable here. The measured width therefore *under*-counts. That bias is
/// chosen deliberately: a flag that fires is certainly real, and absence of a
/// flag is not a guarantee — which is the only calibration under which a
/// reviewer can act on the flag without re-measuring it themselves.
///
/// [`QuyudNass::aqsa_ard`]: taarib_mustalahat::nass::QuyudNass
/// [`QuyudNass::hajm_khatt`]: taarib_mustalahat::nass::QuyudNass
#[must_use]
pub fn alam_tajawuz(
    mudkhal: &MudkhalNass,
    qiyas: Option<&mut MudkhalatQiyas<'_>>,
) -> Option<AlamJawda> {
    let mudkhalat = qiyas?;
    let hadaf = mudkhal.hadaf.as_deref()?;
    if hadaf.trim().is_empty() {
        return None;
    }
    let mutah = mudkhal.quyud.aqsa_ard?;
    let hajm = mudkhal.quyud.hajm_khatt?;
    if mutah <= 0.0 || hajm <= 0.0 {
        return None;
    }

    // A single-line field is measured unwrapped, because wrapping is exactly
    // what the engine will not do for it; a wrappable one is measured against
    // its own width so that only an unbreakable run — the case wrapping cannot
    // save — reports as wider than the box.
    let ard_mutah = if mudkhal.quyud.satr_wahid {
        None
    } else {
        Some(mutah)
    };
    let khiyarat = KhiyaratTakhtit::default();
    let talab = TalabTakhtit {
        nass: hadaf,
        khutut: mudkhalat.khutut,
        hajm,
        ard_mutah,
        irtifa_mutah: None,
        nitaqat: &[],
        khiyarat: &khiyarat,
    };

    match mudkhalat.saff.qis(&talab) {
        Ok(qiyas_nass) => {
            if qiyas_nass.ard > mutah {
                Some(AlamJawda::KhatarTajawuz {
                    ard: qiyas_nass.ard,
                    mutah,
                    hajm,
                })
            } else {
                None
            }
        },
        Err(khata) => {
            tracing::debug!(
                id = %mudkhal.id,
                sabab = %khata.injilizi,
                "قياس التجاوز تعذّر؛ لا علامة بدل التخمين"
            );
            None
        },
    }
}

/// Every per-string flag for one entry, in one call.
///
/// The single entry point the batch runner and the workshop both use, so the
/// two cannot drift into computing different flag sets for the same string —
/// the drift is not hypothetical: an earlier design computed the ratio check
/// in the runner and the Latin check in the workshop, and a string flagged in
/// one view and clean in the other reads to a contributor as data corruption.
///
/// What goes in, and what each absence means:
///
/// - `muraja` — the review record. [`None`] means the review state is unknown
///   at this call site, and the machine-only flag is then **not** produced
///   (see [`alam_aali`]).
/// - `thiqa` — the provider's score, when a provider was involved *and*
///   reported one. [`None`], or a value whose `maqisa` is false, produces no
///   confidence flag (see [`alam_thiqa`]).
/// - `alamat_masrad` — glossary violations, produced by [`crate::masrad`]'s
///   own matching and folded in here verbatim. This module never re-matches
///   terms; only [`AlamJawda::MustalahMukhtalif`] values are accepted from the
///   slice, so a caller cannot smuggle a hand-built overflow flag past the
///   measurement discipline through this parameter.
/// - `qiyas` — the layout engine and fonts. [`None`] means overflow cannot be
///   measured here, and it is then not flagged (see [`alam_tajawuz`]).
///
/// [`AlamJawda::TarjamaMutanaqida`] is deliberately not computable here: it is
/// a property of two entries, and pretending to compute it from one would
/// require the whole table anyway. It comes from [`ihsib_tanaqud`].
///
/// The result is deduplicated and deterministic for identical inputs, which is
/// what lets the journal and a later recompute be compared line for line.
#[must_use]
pub fn ihsib_alamat_nass(
    mudkhal: &MudkhalNass,
    muraja: Option<&SijillMuraja>,
    thiqa: Option<&ThiqaMublagha>,
    alamat_masrad: &[AlamJawda],
    qiyas: Option<&mut MudkhalatQiyas<'_>>,
    atabat: &AtabatAlamat,
) -> Vec<AlamJawda> {
    let hadaf = mudkhal.hadaf.as_deref();
    let mut alamat: Vec<AlamJawda> = Vec::new();
    let mut daa = |alam: Option<AlamJawda>| {
        if let Some(alam) = alam
            && !alamat.contains(&alam)
        {
            alamat.push(alam);
        }
    };

    daa(alam_farigh(&mudkhal.masdar, hadaf));
    daa(alam_nisba(&mudkhal.masdar, hadaf, atabat));
    daa(alam_latini(&mudkhal.masdar, hadaf, atabat));
    daa(alam_thiqa(thiqa, atabat));
    daa(alam_aali(muraja));
    daa(alam_tajawuz(mudkhal, qiyas));

    for alam in alamat_masrad {
        if matches!(alam, AlamJawda::MustalahMukhtalif { .. }) && !alamat.contains(alam) {
            alamat.push(alam.clone());
        }
    }
    alamat
}

/// Stores freshly computed per-string flags on an entry.
///
/// Replaces every flag the per-string pass owns and **preserves**
/// [`AlamJawda::TarjamaMutanaqida`], which belongs to the project-wide pass. A
/// blanket replacement here was the first implementation and it was wrong in a
/// way a contributor found: editing one string re-ran the per-string pass,
/// which erased the inconsistency flag linking it to its twin, and the twin
/// kept a flag pointing at a string that no longer pointed back.
///
/// [`AlamJawda::NasqMaksur`] is also preserved when the new set carries none,
/// but only while the entry is still untranslated: the flag records a
/// protection *failure*, which only the failing run knows, and a recompute
/// from the entry alone cannot re-derive it. The moment a translation lands
/// (`hadaf` present), a surviving broken-markup flag would be describing a
/// reply that no longer exists, so it is dropped.
pub fn thabbit_alamat(mudkhal: &mut MudkhalNass, jadida: Vec<AlamJawda>) {
    let fih_maksur = jadida
        .iter()
        .any(|alam| matches!(alam, AlamJawda::NasqMaksur { .. }));
    let mut mahfudha: Vec<AlamJawda> = Vec::new();
    for alam in mudkhal.alamat.drain(..) {
        let yubqa = match &alam {
            AlamJawda::TarjamaMutanaqida { .. } => true,
            AlamJawda::NasqMaksur { .. } => !fih_maksur && mudkhal.hadaf.is_none(),
            _ => false,
        };
        if yubqa && !mahfudha.contains(&alam) {
            mahfudha.push(alam);
        }
    }
    for alam in jadida {
        if !mahfudha.contains(&alam) {
            mahfudha.push(alam);
        }
    }
    mudkhal.alamat = mahfudha;
}

/// Computes and stores the per-string flags for one entry.
///
/// The convenience the workshop calls after every edit: [`ihsib_alamat_nass`]
/// then [`thabbit_alamat`], with no way to forget the second half.
pub fn ihsib_wa_thabbit(
    mudkhal: &mut MudkhalNass,
    muraja: Option<&SijillMuraja>,
    thiqa: Option<&ThiqaMublagha>,
    alamat_masrad: &[AlamJawda],
    qiyas: Option<&mut MudkhalatQiyas<'_>>,
    atabat: &AtabatAlamat,
) {
    let jadida = ihsib_alamat_nass(mudkhal, muraja, thiqa, alamat_masrad, qiyas, atabat);
    thabbit_alamat(mudkhal, jadida);
}

/// The cross-string pass: identical sources translated differently.
///
/// Groups the whole table by exact source text and, inside each group,
/// compares the trimmed targets of every entry that has one. Two entries with
/// the same source and different targets each receive an
/// [`AlamJawda::TarjamaMutanaqida`] naming the other, because a player sees
/// `حفظ` on one screen and `احفظ` on the next for the same `Save` and reads
/// the whole patch as careless — it is the single most-reported defect class
/// in community translations, and it is invisible per string.
///
/// What is deliberately **not** an inconsistency:
///
/// - entries with no target, or an all-whitespace one — untranslated is a
///   workflow state, not a disagreement;
/// - targets differing only in leading or trailing whitespace — the containers
///   pad differently and the player cannot see it. Interior differences,
///   including punctuation, *are* differences: `حفظ.` against `حفظ` renders
///   differently on screen.
///
/// The named `akhar` is the lowest [`NassId`] in the group whose trimmed
/// target differs from this entry's, so the output is deterministic across
/// runs and diffs of the project file stay quiet when nothing changed.
///
/// The grouping key is the source text itself rather than
/// [`MudkhalNass::majmua`], because the duplicate group is optional and this
/// check must not silently narrow to the entries something remembered to
/// group.
#[must_use]
pub fn ihsib_tanaqud(madakhil: &[MudkhalNass]) -> BTreeMap<NassId, Vec<AlamJawda>> {
    // Source text -> [(id, trimmed target)], targets present and non-empty.
    let mut majmuat: BTreeMap<&str, Vec<(NassId, &str)>> = BTreeMap::new();
    for mudkhal in madakhil {
        let Some(hadaf) = mudkhal.hadaf.as_deref() else {
            continue;
        };
        let mahdhuf = hadaf.trim();
        if mahdhuf.is_empty() {
            continue;
        }
        majmuat
            .entry(mudkhal.masdar.as_str())
            .or_default()
            .push((mudkhal.id, mahdhuf));
    }

    let mut alamat: BTreeMap<NassId, Vec<AlamJawda>> = BTreeMap::new();
    for aada in majmuat.values_mut() {
        if aada.len() < 2 {
            continue;
        }
        // Sorted by id so "the other string" is the same string tomorrow.
        aada.sort_by_key(|(id, _)| *id);
        for (id, hadaf) in aada.iter() {
            let mukhalif = aada
                .iter()
                .find(|(akhar_id, akhar_hadaf)| akhar_id != id && akhar_hadaf != hadaf);
            if let Some((akhar, _)) = mukhalif {
                alamat
                    .entry(*id)
                    .or_default()
                    .push(AlamJawda::TarjamaMutanaqida { akhar: *akhar });
            }
        }
    }
    alamat
}

/// Computes the cross-string flags and stores them on the table.
///
/// Removes every existing [`AlamJawda::TarjamaMutanaqida`] first and writes
/// the fresh set, touching nothing else — the mirror image of
/// [`thabbit_alamat`]'s split. An entry whose inconsistency was *resolved* by
/// an edit therefore loses the flag on the next pass, which is the behaviour
/// that makes fixing the flag feel like fixing anything: the warning goes away
/// when the cause does.
///
/// Returns how many entries carry an inconsistency after the pass, which is
/// the number the run report and the workshop's status bar both quote.
pub fn thabbit_tanaqud(madakhil: &mut [MudkhalNass]) -> usize {
    let jadida = ihsib_tanaqud(madakhil);
    let mut mualama = 0_usize;
    for mudkhal in madakhil.iter_mut() {
        mudkhal
            .alamat
            .retain(|alam| !matches!(alam, AlamJawda::TarjamaMutanaqida { .. }));
        if let Some(alamat) = jadida.get(&mudkhal.id) {
            let mut zad = false;
            for alam in alamat {
                if !mudkhal.alamat.contains(alam) {
                    mudkhal.alamat.push(alam.clone());
                    zad = true;
                }
            }
            if zad {
                mualama = mualama.saturating_add(1);
            }
        }
    }
    mualama
}

/// A whole table's flags, counted for the status bar and the run report.
///
/// The per-kind fields count *flags*; the last two count *entries*, because
/// the two questions they answer are different — "how many glossary problems
/// are there" wants every occurrence, while "how many strings need attention"
/// and "how many strings are blocked from shipping" want each string once no
/// matter how many things are wrong with it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MulakhkhasAlamat {
    /// Measured overflow risks.
    pub tajawuz: usize,
    /// Untranslated Latin remainders.
    pub latini: usize,
    /// Glossary violations.
    pub mustalah: usize,
    /// Broken or missing placeholders — failed strings.
    pub maksur: usize,
    /// Provider-reported low confidence.
    pub thiqa: usize,
    /// Suspicious length ratios.
    pub nisba: usize,
    /// Empty targets under non-empty sources.
    pub farigh: usize,
    /// Identical sources translated differently.
    pub mutanaqid: usize,
    /// Machine translations no human has read.
    pub aali: usize,
    /// Entries carrying at least one flag of any kind.
    pub muallama: usize,
    /// Entries something blocks from shipping ([`MudkhalNass::yamnaa_alnashr`]).
    pub hajiba: usize,
}

/// Counts every flag currently stored on a table.
///
/// Reads what is stored rather than recomputing, deliberately: the status bar
/// refreshes on every edit, and a summary that re-measured fifty thousand
/// strings to update a number would make the workshop hitch on every
/// keystroke. Recomputation is [`ihsib_mashru`]'s job, on the caller's
/// schedule.
#[must_use]
pub fn lakhkhis_alamat(madakhil: &[MudkhalNass]) -> MulakhkhasAlamat {
    let mut mulakhkhas = MulakhkhasAlamat::default();
    for mudkhal in madakhil {
        for alam in &mudkhal.alamat {
            let adad = match alam {
                AlamJawda::KhatarTajawuz { .. } => &mut mulakhkhas.tajawuz,
                AlamJawda::NassLatiniMutabaqqi { .. } => &mut mulakhkhas.latini,
                AlamJawda::MustalahMukhtalif { .. } => &mut mulakhkhas.mustalah,
                AlamJawda::NasqMaksur { .. } => &mut mulakhkhas.maksur,
                AlamJawda::ThiqaMunkhafida { .. } => &mut mulakhkhas.thiqa,
                AlamJawda::NisbaShadha { .. } => &mut mulakhkhas.nisba,
                AlamJawda::Farigh => &mut mulakhkhas.farigh,
                AlamJawda::TarjamaMutanaqida { .. } => &mut mulakhkhas.mutanaqid,
                AlamJawda::AaliyaBilaMuraja => &mut mulakhkhas.aali,
            };
            *adad = adad.saturating_add(1);
        }
        if !mudkhal.alamat.is_empty() {
            mulakhkhas.muallama = mulakhkhas.muallama.saturating_add(1);
        }
        if mudkhal.yamnaa_alnashr() {
            mulakhkhas.hajiba = mulakhkhas.hajiba.saturating_add(1);
        }
    }
    mulakhkhas
}

/// Recomputes every flag for a whole table: the per-string pass on each entry,
/// then the cross-string pass, then the summary.
///
/// The workshop's "recompute quality flags" action and the last step of a
/// finished batch run both land here, so the two cannot disagree about what a
/// full recompute means.
///
/// The maps carry the facts that live outside the entries, keyed by string:
///
/// - `muraja` — the live review records. Entries with no record produce no
///   machine-only flag, per [`alam_aali`].
/// - `thiqat` — provider-reported scores, extracted from the run journal by
///   [`crate::dufaat::thiqat_min_sijill`]. The score exists nowhere on the
///   entry, so a recompute not handed this map cannot re-judge confidence —
///   and per this module's discipline it then produces **no** confidence
///   flags rather than guessed ones. A caller refreshing flags after a run
///   passes the journal's scores; a caller with no journal passes an empty
///   map and knowingly gets none.
/// - `alamat_masrad` — the glossary module's violations, folded in verbatim.
/// - `qiyas` — the engine and fonts; [`None`] skips overflow measurement, per
///   [`alam_tajawuz`].
///
/// Returns the summary of what the table carries afterwards.
pub fn ihsib_mashru(
    madakhil: &mut [MudkhalNass],
    muraja: Option<&BTreeMap<NassId, SijillMuraja>>,
    thiqat: &BTreeMap<NassId, ThiqaMublagha>,
    alamat_masrad: &BTreeMap<NassId, Vec<AlamJawda>>,
    mut qiyas: Option<&mut MudkhalatQiyas<'_>>,
    atabat: &AtabatAlamat,
) -> MulakhkhasAlamat {
    for mudkhal in madakhil.iter_mut() {
        let sijill = muraja.and_then(|kharita| kharita.get(&mudkhal.id));
        let thiqa = thiqat.get(&mudkhal.id);
        let masrad: &[AlamJawda] = alamat_masrad.get(&mudkhal.id).map_or(&[], Vec::as_slice);
        let jadida =
            ihsib_alamat_nass(mudkhal, sijill, thiqa, masrad, qiyas.as_deref_mut(), atabat);
        thabbit_alamat(mudkhal, jadida);
    }
    let _ = thabbit_tanaqud(madakhil);
    lakhkhis_alamat(madakhil)
}
