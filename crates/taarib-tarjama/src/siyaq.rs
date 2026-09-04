//! السياق — what a translation request is told, and what it is deliberately not.
//!
//! A game string handed to a model bare — "Attack", no speaker, no screen, no
//! register — comes back as whichever Arabic the model reached first, and the
//! failure is not random: menu labels come back as verb sentences, dialogue
//! comes back as newspaper prose, and the same item name comes back three ways
//! in three scenes. Every builder in this module exists because one of those
//! happened to a real string table.
//!
//! ## Register per classification, not per request
//!
//! Phase 12 derives [`TasnifNass`] from where a string was found, never from
//! its content, precisely because content cannot distinguish `Attack` the menu
//! label from `Attack` the dialogue line. This module is where that
//! distinction pays: each variant carries its own written instruction —
//! natural spoken Arabic for [`TasnifNass::Hiwar`], concise Modern Standard
//! for [`TasnifNass::Qaima`], a consistent noun phrase for [`TasnifNass::Ism`]
//! — and the instruction is written out in Arabic, because an instruction
//! *about* Arabic register described in English is a translation of itself and
//! loses exactly the nuance it is trying to demand.
//!
//! ## The tokens are enumerated, never described
//!
//! The instruction does not say "preserve any placeholders". It lists them:
//! "reproduce ⟦0⟧ and ⟦1⟧ exactly, 2 tokens in, 2 tokens out", taken from
//! [`NassMahmi::rumuz`]. A model told which tokens exist and how many loses
//! them measurably less often than one told to preserve a category — the
//! category invites judgement about what counts as a placeholder, and the
//! model's judgement is the thing [`crate::hima`] exists to remove. The count
//! is stated as well as the list because a model that dropped one token can
//! still count.
//!
//! ## Absent constraints are absent, not estimated
//!
//! Where the engine imposes a character limit, the instruction states it as a
//! hard limit. Where runtime capture measured a pixel width, the instruction
//! states the width in pixels and says the result will be measured — pixels
//! are the real unit and converting them to "roughly N characters" here would
//! be this crate inventing a number [`taarib_saff`] exists to measure. And
//! where nothing is known, the instruction says **nothing about length at
//! all**. An invented "keep it short" makes a translator of dialogue clip
//! sentences that had all the room in the world; the absence of a constraint
//! is information and it is transmitted by absence.
//!
//! ## The reply is a schema, not prose
//!
//! [`mukhattat_radd`] and [`mukhattat_sarim`] define the reply as data: a JSON
//! object with the translation and, only where the provider reports one, a
//! confidence. Two variants exist because two dialects of JSON Schema are in
//! play — `OpenAI`'s strict mode *requires* `additionalProperties: false` and
//! every property listed in `required`, while Gemini's `OpenAPI` subset rejects
//! `additionalProperties` outright — and papering over that with one schema
//! would silently disable strictness on the provider that has it.
//!
//! A reply that does not parse against the schema is
//! [`KhataTarjama::RaddGhayrMufassal`]. Nothing in this crate extracts a
//! translation from a conversational reply with a pattern: a model that wrote
//! "Here is your translation: …" has not answered in the shape it was asked
//! for, and scraping the shape out of the prose is how a polite preamble ships
//! inside a game's dialogue box.
//!
//! ## What a request never carries
//!
//! [`SiyaqTalab`] has no field for the string's own source text, and
//! [`SiyaqTalab::min_mudkhal`] does not copy it. The only copy of the string a
//! provider ever sees is the tokenised text inside [`NassMahmi`] — see
//! [`crate::hima`] for why that is structural. The neighbouring dialogue lines
//! in [`SiyaqNass::jiwar`] are context in the ordinary sense and travel as
//! themselves; the string being translated does not.

use serde::Deserialize;
use serde_json::{Value, json};

use taarib_mustalahat::nass::{MudkhalNass, QuyudNass, SiyaqNass, TasnifNass};

use crate::hima::NassMahmi;
use crate::khata::KhataTarjama;

/// The version of the prompt this module builds.
///
/// Recorded against every machine translation so a result can still be
/// explained after the prompt changes: "why is this string worded like a menu"
/// has a different answer under a version that carried no register
/// instruction. Bumped on any change to the instructions this module emits,
/// not on refactors that leave the emitted text identical.
pub const ISDAR_TAWJIH: &str = "tawjih-13.1";

/// The JSON field carrying the translation in every structured reply.
pub const HAQL_TARJAMA: &str = "tarjama";

/// The JSON field carrying the model's confidence, where one was requested.
pub const HAQL_THIQA: &str = "thiqa";

/// How many neighbouring dialogue lines the prompt will carry, per side.
///
/// Eight, which covers a scene's exchange without turning every request into a
/// transcript. The cap exists because context is billed like everything else,
/// and a caller that loaded a whole chapter into [`SiyaqNass::jiwar`] would
/// pay for the chapter on every line of it.
pub const AQSA_JIWAR: usize = 8;

/// How many translation-memory matches the prompt will carry.
///
/// Three. The nearest match is the useful one; the second and third are there
/// so the model can see a *pattern* of prior choices. A tenth match at 61%
/// similarity is noise with a token cost.
pub const AQSA_DHAKIRA: usize = 3;

/// How many glossary terms the prompt will carry.
///
/// Bounded so a pathological string that happens to contain half the glossary
/// does not build a prompt that is mostly glossary. Sixty-four terms in one
/// string is not a sentence being translated, it is the glossary being read
/// back.
pub const AQSA_MUSTALAHAT: usize = 64;

/// Below this classification confidence, the register instruction hedges.
///
/// Sixty out of a hundred. Phase 12 carries the extractor's confidence beside
/// the classification precisely so that downstream layers can treat "I know"
/// and "I am guessing" differently, and this is the threshold at which this
/// module does: under it, the instruction still names the likely register but
/// tells the model the classification is uncertain and to keep the
/// conservative habits of [`TasnifNass::Majhul`] where the two conflict. A
/// hedged prompt for a right guess costs a little polish; a confident prompt
/// for a wrong guess turns a line of dialogue into a button label.
pub const AQALL_THIQAT_TASNIF: u8 = 60;

// ---------------------------------------------------------------------------
// What travels with a request
// ---------------------------------------------------------------------------

/// One glossary term that appears in the string being translated.
///
/// Carried as data rather than pre-rendered into prose so the same term list
/// can be rendered into the prompt here and checked against the reply by the
/// quality-flag layer, without the two ever disagreeing about what was
/// required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MustalahMulzim {
    /// The term as it appears in the source text.
    pub asl: String,
    /// The canonical Arabic the project has decided on.
    pub arabi: String,
    /// Whether the term must not be translated at all — a proper noun, a brand,
    /// a stylised system name the game renders in Latin script on purpose.
    pub thabit: bool,
    /// A translator's note about usage, where the glossary carries one.
    pub mulahaza: Option<String>,
}

/// One translation-memory match near the string being translated.
///
/// The similarity is an integer percentage rather than a float because it is
/// compared, sorted and displayed, and every one of those is cleaner over
/// `u8` than over an `f32` this workspace's lints would then forbid comparing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TatabuqDhakira {
    /// The remembered source text.
    pub masdar: String,
    /// The Arabic it was translated to.
    pub tarjama: String,
    /// Similarity to the current string, zero to one hundred.
    pub nisba: u8,
}

/// Everything a prompt carries about one string.
///
/// **Deliberately without the string's own source text.** The text travels
/// only inside [`NassMahmi`], already tokenised — see this module's header and
/// [`crate::hima`]. A `SiyaqTalab` on its own cannot leak the string it
/// describes because it does not hold it.
#[derive(Debug, Clone)]
pub struct SiyaqTalab {
    /// What kind of string this is, from Phase 12's location-derived
    /// classification.
    pub tasnif: TasnifNass,
    /// Where the string came from: container, location, field, speaker, and
    /// the neighbouring lines where they are sequential dialogue.
    pub siyaq: SiyaqNass,
    /// How much room the string has, where that was measured. Every field in
    /// here is optional and an absent one stays absent in the prompt.
    pub quyud: QuyudNass,
    /// The glossary terms present in this string, canonical Arabic included.
    pub mustalahat: Vec<MustalahMulzim>,
    /// The translation memory's nearest matches, best first.
    pub dhakira: Vec<TatabuqDhakira>,
    /// The game's title, where the project knows it, so the model translates
    /// "the sword in this game" rather than "a sword in general".
    pub ism_luba: Option<String>,
    /// How sure Phase 12's extractor is of the classification, zero to one
    /// hundred, copied from [`MudkhalNass::thiqat_tasnif`].
    ///
    /// Carried because "menu label, and I am certain" and "menu label, and I
    /// am guessing" are prompted differently: below
    /// [`AQALL_THIQAT_TASNIF`] the instruction admits the classification is a
    /// guess and pulls the register toward the conservative one, instead of
    /// confidently demanding menu Arabic for what might be a line of dialogue.
    pub thiqat_tasnif: u8,
}

impl SiyaqTalab {
    /// Builds the context for a string from its Phase 12 entry.
    ///
    /// Copies the classification, the source location and the constraints.
    /// **Does not copy [`MudkhalNass::masdar`]** — the source text reaches a
    /// provider only through [`crate::hima::ihmi`], and this constructor not
    /// taking it is part of that guarantee rather than an omission.
    ///
    /// Glossary terms and memory matches start empty; the layers that own the
    /// glossary and the memory fill them in, bounded by [`AQSA_MUSTALAHAT`]
    /// and [`AQSA_DHAKIRA`].
    #[must_use]
    pub fn min_mudkhal(mudkhal: &MudkhalNass) -> Self {
        Self {
            tasnif: mudkhal.tasnif,
            siyaq: mudkhal.siyaq.clone(),
            quyud: mudkhal.quyud.clone(),
            mustalahat: Vec::new(),
            dhakira: Vec::new(),
            ism_luba: None,
            thiqat_tasnif: mudkhal.thiqat_tasnif,
        }
    }

    /// The neighbouring lines the prompt will actually carry.
    ///
    /// Bounded by [`AQSA_JIWAR`] from the end, because when the capture layer
    /// stored more than the cap the *nearest* lines are the ones that
    /// disambiguate a pronoun or a reply, and the nearest are last.
    #[must_use]
    pub fn jiwar_mahdud(&self) -> &[SatrJiwar] {
        let tul = self.siyaq.jiwar.len();
        let bidaya = tul.saturating_sub(AQSA_JIWAR);
        self.siyaq.jiwar.get(bidaya..).unwrap_or_default()
    }
}

/// A neighbouring line as stored by Phase 12.
///
/// An alias rather than a wrapper: [`SiyaqNass::jiwar`] already owns the
/// representation and inventing a second type for the same string would give
/// two modules something to disagree about.
pub type SatrJiwar = String;

// ---------------------------------------------------------------------------
// Register per classification
// ---------------------------------------------------------------------------

/// The register instruction for one classification, written in Arabic.
///
/// In Arabic because the instruction is *about* Arabic: "natural spoken
/// register" in English names a target the model must translate back into a
/// concept, while «فصحى مبسّطة قريبة من الكلام» names it directly. One
/// instruction per [`TasnifNass`] variant, exhaustively, so a variant added to
/// Phase 12 fails to compile here instead of silently getting dialogue
/// treatment.
#[must_use]
pub const fn tawjih_tasnif(tasnif: TasnifNass) -> &'static str {
    match tasnif {
        TasnifNass::Hiwar => {
            "هذا سطر حوار منطوق. ترجمه بعربية فصيحة مبسّطة قريبة من إيقاع الكلام، لا بلغة الصحف ولا بعامية محلية: جمل قصيرة، تراكيب يقولها متحدث فعلًا، وانفعال يطابق انفعال الأصل. راعِ جنس المتكلم والمخاطب في تصريف الأفعال والضمائر إن دلّ السياق عليهما، وحافظ على شخصية المتكلم كما تظهر في السطور المجاورة — من يتكلم بجفاء يبقى جافيًا ومن يمزح يبقى مازحًا."
        }
        TasnifNass::Ikhtiyar => {
            "هذا خيار يظهر للاعب ليختاره من بين خيارات. ترجمه بصيغة موجزة مباشرة يقرؤها اللاعب بلمحة، وبتركيب نحوي متوازٍ مع بقية الخيارات إن ظهرت في الجوار: إن كانت أفعالًا فأفعال، وإن كانت جملًا اسمية فجمل اسمية. لا تحوّل الخيار إلى جملة تفسيرية."
        }
        TasnifNass::Ism => {
            "هذا اسم — اسم عنصر أو مهارة أو شخصية أو مكان. ترجمه تركيبًا اسميًا ثابتًا لا جملة فعلية، بلا نقطة في آخره، وبصيغة تصلح للتكرار في كل موضع يظهر فيه: في قائمة، في وصف، في حوار. إن ورد الاسم في المسرد فالمسرد هو الحجة ولا اجتهاد معه."
        }
        TasnifNass::Wasf => {
            "هذا نص وصفي — وصف عنصر أو مهارة أو مكان. ترجمه بعربية فصيحة سليمة بجمل تامة، تنقل المعلومة والنبرة معًا، من غير حشو تفسيري لم يرد في الأصل ومن غير اختصار يسقط معلومة وردت فيه."
        }
        TasnifNass::Qaima => {
            "هذا عنصر واجهة — زر أو تبويب أو بند قائمة. ترجمه بفصحى موجزة على عرف الواجهات العربية: المصدر لا فعل الأمر حيث يستقيم («حفظ» لا «احفظ»)، كلمة أو كلمتان حيث أمكن، بلا نقطة في الآخر، وبثبات تام — البند نفسه يُترجم الترجمة نفسها في كل شاشة."
        }
        TasnifNass::Tafseer => {
            "هذا نص مساعدة أو تلميح يظهر عند التمرير أو الوقوف على عنصر. ترجمه جملة قصيرة تامة تشرح وظيفة العنصر مباشرة، وخاطب اللاعب بصيغة المخاطب حيث يرشده النص إلى فعل شيء."
        }
        TasnifNass::Nizam => {
            "هذه رسالة نظام — حُفظت اللعبة، انقطع الاتصال، اكتمل التنزيل. ترجمها بصيغة إخبارية موجزة محايدة كما تصاغ رسائل الأنظمة العربية («تم الحفظ»، «انقطع الاتصال بالخادم»)، من غير خطاب شخصي ومن غير زخرفة."
        }
        TasnifNass::Khata => {
            "هذه رسالة خطأ يراها اللاعب. ترجمها بعبارة مباشرة تقول ما الذي حدث وما الذي يفعله اللاعب حياله إن ذكره الأصل، من غير لوم ومن غير تهويل، وبمصطلحات تقنية متسقة مع بقية الرسائل."
        }
        TasnifNass::Nusub => {
            "هذا نص ثابت — أسماء فريق العمل أو تراخيص أو نص قانوني. ترجمه بفصحى رسمية دقيقة، وأبقِ أسماء الأعلام والمؤسسات والتراخيص بحروفها اللاتينية كما وردت إلا ما نصّ المسرد على تعريبه. في النص القانوني الدقة قبل السلاسة."
        }
        TasnifNass::Dakhili => {
            "صُنّف هذا النص داخليًا — معرّف أو مسار أو مفتاح لا يُعرض للاعب عادة — وقد اختار مترجم بشري إرساله رغم ذلك. ترجم الكلمات المقروءة وحدها إن وُجدت، وأبقِ كل ما يشبه المعرّف التقني أو المسار أو المفتاح البرمجي كما هو حرفًا بحرف، فتغييره قد يعطّل اللعبة لا نصّها."
        }
        TasnifNass::Majhul => {
            "لم يتبيّن نوع هذا النص من موضعه. ترجمه بفصحى محايدة ملتزمة ببنية الأصل التزامًا وثيقًا، من غير افتراض أنه حوار أو زر أو وصف: لا تضف نبرة ولا تحذف شيئًا، فالترجمة المحافظة هي الوحيدة الصالحة لكل الاحتمالات."
        }
    }
}

// ---------------------------------------------------------------------------
// The system instruction
// ---------------------------------------------------------------------------

/// Builds the system instruction for one request.
///
/// The instruction is assembled from parts in a fixed order — role, register,
/// token contract, constraint, glossary, output contract — because providers
/// cache system prompts by prefix and a stable prefix is money: the role and
/// register sections are identical for every string of the same
/// classification, so they come first and the per-string material comes last.
///
/// `bi_thiqa` is whether the reply schema will carry a confidence field, which
/// changes the output contract's wording. It is a parameter rather than read
/// from a capability here because this module does not know providers exist;
/// the provider passes what its capability declares.
#[must_use]
pub fn tawjih_nizam(talab: &SiyaqTalab, mahmi: &NassMahmi, bi_thiqa: bool) -> String {
    let mut nass = String::with_capacity(2048);

    // The role, and the one sentence of product truth that shapes everything:
    // this is a game, and the string is a game string.
    nass.push_str(
        "أنت مترجم ألعاب محترف تنقل نصوص لعبة من لغتها الأصلية إلى العربية. \
         النص التالي سطر واحد من جدول نصوص اللعبة، وليس مقالًا ولا وثيقة.",
    );
    if let Some(ism) = &talab.ism_luba {
        nass.push_str("\nاللعبة: ");
        nass.push_str(ism);
        nass.push('.');
    }
    nass.push_str("\n\n");

    nass.push_str(tawjih_tasnif(talab.tasnif));
    nass.push('\n');
    if talab.thiqat_tasnif < AQALL_THIQAT_TASNIF
        && !matches!(talab.tasnif, TasnifNass::Majhul)
    {
        nass.push_str(
            "تنبيه: تصنيف هذا النص غير مؤكد، فهو استنتاج من موضعه لا حقيقة قاطعة. إن بدا لك من \
             النص نفسه أنه من نوع آخر فالتزم الحذر: حافظ على بنية الأصل ولا تفرض عليه قالب \
             التصنيف فرضًا.\n",
        );
    }
    nass.push('\n');

    idraj_aqd_rumuz(&mut nass, mahmi);
    idraj_qayd_hajm(&mut nass, &talab.quyud);
    idraj_mustalahat(&mut nass, &talab.mustalahat);
    idraj_aqd_ikhraj(&mut nass, bi_thiqa);

    nass
}

/// Writes the token contract into the instruction.
///
/// The tokens are enumerated one by one from [`NassMahmi::rumuz`], with their
/// count, and the contract is stated in both directions: reproduce these, add
/// none. "Reproduce ⟦0⟧ and ⟦1⟧ exactly" loses tokens far less often than
/// "preserve placeholders" — the enumeration removes the model's judgement
/// about what counts as one, and the count gives it a check it can perform on
/// its own output. Reordering is explicitly permitted because Arabic grammar
/// reorders arguments, and a model afraid to move a token produces English
/// word order with Arabic words.
fn idraj_aqd_rumuz(nass: &mut String, mahmi: &NassMahmi) {
    use std::fmt::Write as _;

    let rumuz = mahmi.rumuz();
    if rumuz.is_empty() {
        return;
    }
    nass.push_str("يحتوي النص على رموز محفوظة معتمة ليست جزءًا من اللغة: ");
    for (fahras, ramz) in rumuz.iter().enumerate() {
        if fahras > 0 {
            nass.push_str("، ");
        }
        nass.push_str(ramz);
    }
    let adad = rumuz.len();
    let _ = write!(
        nass,
        "\nأعد كتابة هذه الرموز في الترجمة كما هي حرفًا بحرف — {adad} في الأصل و{adad} في \
         الترجمة، لا أكثر ولا أقل، ولا تخترع رمزًا جديدًا. يجوز لك تغيير موضعها في الجملة بما \
         تقتضيه العربية، فالمهم وجودها لا مكانها. النص الواقع بين رمز فتح ورمز إغلاق مثل \
         ⟦١⟧…⟦/١⟧ يُترجم ويبقى بين الرمزين.\n\n"
    );
}

/// Writes the size constraint into the instruction — or writes nothing.
///
/// Three honest cases and no fourth. A known character limit is a hard limit
/// and is said so. A measured pixel width is stated in pixels — its real unit
/// — with the fact that the result will be measured; converting it to a
/// character count here would be inventing the number `taarib-saff` exists to
/// measure. And when nothing is known, **nothing is said**: an invented "keep
/// it short" clips dialogue that had room, and the absence of a constraint is
/// transmitted by absence.
fn idraj_qayd_hajm(nass: &mut String, quyud: &QuyudNass) {
    use std::fmt::Write as _;

    if let Some(aqsa) = quyud.aqsa_ahruf {
        let _ = write!(
            nass,
            "قيد صارم: لا يجوز أن تتجاوز الترجمة {aqsa} حرفًا بأي حال، فالمحرك يقتطع ما زاد. \
             الرموز المحفوظة تُحسب ضمن العدد.\n\n"
        );
        return;
    }
    if let Some(ard) = quyud.aqsa_ard {
        let _ = write!(
            nass,
            "يُعرض هذا النص في مساحة ثابتة عرضها {ard:.0} بكسل وسيُقاس عرض الترجمة فعليًا \
             بعد إنتاجها. قدّم أوجز صياغة صحيحة تفي بالمعنى من غير إخلال به.\n\n"
        );
        return;
    }
    if quyud.satr_wahid {
        nass.push_str(
            "يُعرض هذا النص في سطر واحد لا يلتف. لا تُدخل فواصل أسطر في الترجمة.\n\n",
        );
    }
}

/// Writes the binding glossary terms into the instruction.
///
/// The glossary is stated as authority, not suggestion: «المسرد حجة». Terms
/// flagged do-not-translate are listed separately with the stronger wording,
/// because "translate X as Y" and "do not translate X" are different contracts
/// and a model given them in one list treats both as preferences.
fn idraj_mustalahat(nass: &mut String, mustalahat: &[MustalahMulzim]) {
    if mustalahat.is_empty() {
        return;
    }
    let mahduda = mustalahat.get(..mustalahat.len().min(AQSA_MUSTALAHAT)).unwrap_or_default();

    let (thabita, mutarjama): (Vec<_>, Vec<_>) =
        mahduda.iter().partition(|mustalah| mustalah.thabit);

    if !mutarjama.is_empty() {
        nass.push_str("مصطلحات المشروع — المسرد حجة ولا اجتهاد معه:\n");
        for mustalah in &mutarjama {
            nass.push_str("- «");
            nass.push_str(&mustalah.asl);
            nass.push_str("» تُترجم دائمًا «");
            nass.push_str(&mustalah.arabi);
            nass.push('»');
            if let Some(mulahaza) = &mustalah.mulahaza {
                nass.push_str(" — ");
                nass.push_str(mulahaza);
            }
            nass.push('\n');
        }
        nass.push('\n');
    }
    if !thabita.is_empty() {
        nass.push_str("أسماء لا تُترجم وتبقى بحروفها كما وردت:\n");
        for mustalah in &thabita {
            nass.push_str("- ");
            nass.push_str(&mustalah.asl);
            if let Some(mulahaza) = &mustalah.mulahaza {
                nass.push_str(" — ");
                nass.push_str(mulahaza);
            }
            nass.push('\n');
        }
        nass.push('\n');
    }
}

/// Writes the output contract: JSON, these fields, nothing else.
///
/// The word "JSON" appears explicitly because one of the providers this crate
/// targets (any OpenAI-compatible endpoint running in `json_object` mode)
/// refuses requests whose messages never mention JSON, and because a model
/// told the shape in prose *and* held to it by a schema fails less often than
/// one held by either alone.
fn idraj_aqd_ikhraj(nass: &mut String, bi_thiqa: bool) {
    use std::fmt::Write as _;

    let _ = write!(
        nass,
        "أجب بكائن JSON واحد لا شيء غيره: لا شرح قبله ولا بعده ولا علامات تنسيق حوله. \
         الحقل «{HAQL_TARJAMA}» يحمل الترجمة العربية وحدها."
    );
    if bi_thiqa {
        let _ = write!(
            nass,
            " والحقل «{HAQL_THIQA}» يحمل تقديرك لثقتك في صحة هذه الترجمة عددًا بين 0 و1، \
             وإن لم تستطع تقديرها فاجعله null ولا تخترع عددًا."
        );
    }
    nass.push('\n');
}

// ---------------------------------------------------------------------------
// The user message
// ---------------------------------------------------------------------------

/// Builds the user message: the context that varies per string, then the text.
///
/// Ordered speaker → neighbours → memory → text, with the text last and
/// clearly fenced, because a model answers what it read most recently and the
/// thing to answer is the text. The neighbouring lines are labelled as context
/// and explicitly excluded from translation — the single most common failure
/// of context-carrying prompts is the model translating the context too, and
/// the exclusion sentence is what prevents it.
#[must_use]
pub fn risalat_mustakhdim(talab: &SiyaqTalab, mahmi: &NassMahmi) -> String {
    use std::fmt::Write as _;

    let mut nass = String::with_capacity(1024);

    if let Some(mutakallim) = &talab.siyaq.mutakallim {
        nass.push_str("المتكلم: ");
        nass.push_str(mutakallim);
        nass.push('\n');
    }
    if let Some(mashhad) = &talab.siyaq.mashhad {
        nass.push_str("المشهد: ");
        nass.push_str(mashhad);
        nass.push('\n');
    }

    let jiwar = talab.jiwar_mahdud();
    if !jiwar.is_empty() {
        nass.push_str(
            "\nالسطور المجاورة في الحوار، للسياق فقط — لا تترجمها ولا تضمّنها في الجواب:\n",
        );
        for satr in jiwar {
            nass.push_str("| ");
            nass.push_str(satr);
            nass.push('\n');
        }
    }

    let dhakira = talab.dhakira.get(..talab.dhakira.len().min(AQSA_DHAKIRA)).unwrap_or_default();
    if !dhakira.is_empty() {
        nass.push_str(
            "\nترجمات سابقة معتمدة لنصوص مشابهة في هذا المشروع، فالتزم أسلوبها ومصطلحاتها:\n",
        );
        for tatabuq in dhakira {
            let _ = writeln!(
                nass,
                "- ({}٪) «{}» ← «{}»",
                tatabuq.nisba, tatabuq.masdar, tatabuq.tarjama
            );
        }
    }

    nass.push_str("\nالنص المطلوب ترجمته، وهو ما بين السطرين وحده:\n");
    nass.push_str("<<<\n");
    nass.push_str(mahmi.matn());
    nass.push_str("\n>>>");
    nass
}

// ---------------------------------------------------------------------------
// The reply, as a schema and as data
// ---------------------------------------------------------------------------

/// The reply schema in its portable form.
///
/// A JSON object with [`HAQL_TARJAMA`] required and, when `bi_thiqa`,
/// [`HAQL_THIQA`] optional and bounded to `[0, 1]`. This is the variant sent
/// to Gemini's `responseSchema`, whose `OpenAPI` subset rejects
/// `additionalProperties` — the field `OpenAI`'s strict mode requires — which is
/// the whole reason [`mukhattat_sarim`] exists beside this.
#[must_use]
pub fn mukhattat_radd(bi_thiqa: bool) -> Value {
    let mut khawass = serde_json::Map::new();
    let _ = khawass.insert(
        HAQL_TARJAMA.to_owned(),
        json!({
            "type": "string",
            "description": "الترجمة العربية وحدها، بكل رمز محفوظ ورد في الأصل"
        }),
    );
    if bi_thiqa {
        let _ = khawass.insert(
            HAQL_THIQA.to_owned(),
            json!({
                "type": "number",
                "minimum": 0,
                "maximum": 1,
                "description": "ثقة النموذج في صحة الترجمة، من 0 إلى 1"
            }),
        );
    }
    json!({
        "type": "object",
        "properties": Value::Object(khawass),
        "required": [HAQL_TARJAMA]
    })
}

/// The reply schema in its strict form.
///
/// `OpenAI`'s `strict: true` structured output demands `additionalProperties:
/// false` and **every** listed property in `required` — optionality is
/// expressed as a union with `null`, not by omission. Anthropic's tool input
/// schema accepts this form as ordinary JSON Schema, so both use it. Gemini
/// cannot: its `OpenAPI` subset rejects `additionalProperties`, which is why the
/// portable [`mukhattat_radd`] exists. One schema for all three would mean
/// disabling strictness on the two providers that offer it.
#[must_use]
pub fn mukhattat_sarim(bi_thiqa: bool) -> Value {
    let mut khawass = serde_json::Map::new();
    let _ = khawass.insert(
        HAQL_TARJAMA.to_owned(),
        json!({
            "type": "string",
            "description": "الترجمة العربية وحدها، بكل رمز محفوظ ورد في الأصل"
        }),
    );
    let mut matluba = vec![Value::from(HAQL_TARJAMA)];
    if bi_thiqa {
        let _ = khawass.insert(
            HAQL_THIQA.to_owned(),
            json!({
                "type": ["number", "null"],
                "description": "ثقة النموذج في صحة الترجمة من 0 إلى 1، أو null إن تعذّر تقديرها"
            }),
        );
        matluba.push(Value::from(HAQL_THIQA));
    }
    json!({
        "type": "object",
        "properties": Value::Object(khawass),
        "required": Value::Array(matluba),
        "additionalProperties": false
    })
}

/// A structured reply, parsed as data.
///
/// The only way a provider's words become a translation. Constructed by
/// [`hallil_radd`] and [`hallil_qeema`] and by nothing else, so there is no
/// path on which a conversational reply is scraped into one with a pattern.
#[derive(Debug, Clone, Deserialize)]
pub struct RaddMufassal {
    /// The translation, exactly as the model wrote it. Its tokens have not
    /// been verified yet — that is [`crate::hima::istaridd`]'s job and no one
    /// else's.
    #[serde(rename = "tarjama")]
    pub tarjama: String,
    /// The confidence field as it arrived, kept in the wire's own width.
    /// Read through [`RaddMufassal::thiqa`], which is where the bounds live.
    #[serde(rename = "thiqa", default)]
    thiqa_khaam: Option<f64>,
}

impl RaddMufassal {
    /// The model's self-reported confidence, when it reported one.
    ///
    /// [`None`] when the field was absent, `null`, or not a finite number — a
    /// model that wrote `NaN` has not reported a confidence, it has reported a
    /// malfunction, and passing it through would poison every threshold
    /// comparison downstream. A finite value is clamped into `[0, 1]` because
    /// a model that wrote `1.3` meant "very sure", and rejecting the whole
    /// translation over an out-of-range sidecar number would fail good work
    /// for bad bookkeeping.
    #[must_use]
    pub fn thiqa(&self) -> Option<f32> {
        let khaam = self.thiqa_khaam?;
        if !khaam.is_finite() {
            return None;
        }
        let mahsur = khaam.clamp(0.0, 1.0);
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the value is clamped into [0.0, 1.0], where every f64 maps into f32's \
                      range; only precision beyond f32 is lost, and a confidence has no \
                      meaningful precision at that depth"
        )]
        {
            Some(mahsur as f32)
        }
    }
}

/// Parses a reply that arrived as a JSON string.
///
/// The path for providers that return their structured output as text —
/// OpenAI-compatible `message.content`, Gemini's `parts[].text`. The text
/// must *be* the object; nothing is searched for inside prose.
///
/// # Errors
///
/// [`KhataTarjama::RaddGhayrMufassal`] when the text is not a JSON object
/// carrying [`HAQL_TARJAMA`] as a string, or when the translation is empty —
/// an empty translation of a non-empty source is not a translation, and
/// accepting it would ship [`taarib_mustalahat::nass::AlamJawda::Farigh`]
/// with extra steps.
pub fn hallil_radd(nass: &str, muzawwid: &str) -> Result<RaddMufassal, KhataTarjama> {
    let qeema: Value = serde_json::from_str(nass.trim())
        .map_err(|_| ghayr_mufassal(muzawwid, nass))?;
    hallil_qeema(&qeema, muzawwid)
}

/// Parses a reply that arrived already as JSON.
///
/// The path for providers that deliver the object as data — Anthropic's
/// `tool_use.input`. Shared with [`hallil_radd`] so the two paths cannot
/// drift in what they accept.
///
/// # Errors
///
/// [`KhataTarjama::RaddGhayrMufassal`] on the same conditions as
/// [`hallil_radd`]: not an object, no [`HAQL_TARJAMA`] string, or an empty
/// translation.
pub fn hallil_qeema(qeema: &Value, muzawwid: &str) -> Result<RaddMufassal, KhataTarjama> {
    let radd: RaddMufassal = serde_json::from_value(qeema.clone())
        .map_err(|_| ghayr_mufassal(muzawwid, &qeema.to_string()))?;
    if radd.tarjama.trim().is_empty() {
        return Err(ghayr_mufassal(muzawwid, &qeema.to_string()));
    }
    Ok(radd)
}

/// The unparseable-reply failure, with the reply bounded for the report.
///
/// Bounded to the first 64 characters for the same reason
/// [`crate::hima`] bounds its own reports: a model that answered with four
/// thousand words of apology should not put four thousand words into a log.
fn ghayr_mufassal(muzawwid: &str, radd: &str) -> KhataTarjama {
    KhataTarjama::RaddGhayrMufassal {
        muzawwid: muzawwid.to_owned(),
        radd: radd.chars().take(64).collect(),
    }
}

// ---------------------------------------------------------------------------
// Context for engines that take context but not instructions
// ---------------------------------------------------------------------------

/// The context in one plain line, for engines with a context slot and no
/// system instruction.
///
/// `DeepL`'s `context` parameter takes free text that informs the translation
/// without being translated; the dedicated translation APIs have no register
/// instruction to send, so the register work above cannot reach them. What
/// *can* reach them is the situation: who is speaking, what was said around
/// this line, what kind of string it is. That is what this renders — compact,
/// in the source-side language, because a context an engine cannot read is
/// tokens spent on nothing.
///
/// [`None`] when there is nothing to say, so a caller can omit the parameter
/// entirely rather than sending an empty field an API may reject.
#[must_use]
pub fn siyaq_khatti(talab: &SiyaqTalab) -> Option<String> {
    let mut ajza: Vec<String> = Vec::new();

    ajza.push(format!(
        "Game UI string of kind: {}.",
        talab.tasnif.wasf_injilizi()
    ));
    if let Some(mutakallim) = &talab.siyaq.mutakallim {
        ajza.push(format!("Spoken by {mutakallim}."));
    }
    let jiwar = talab.jiwar_mahdud();
    if !jiwar.is_empty() {
        ajza.push(format!(
            "Surrounding dialogue: {}",
            jiwar.join(" / ")
        ));
    }

    // The classification line alone is worth sending — "this is a menu label"
    // is exactly the fact a bare MT engine cannot know — so the only empty
    // case is the one where even that is unknown.
    if matches!(talab.tasnif, TasnifNass::Majhul) && ajza.len() == 1 {
        return None;
    }
    Some(ajza.join(" "))
}

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

/// What was asked, recorded so the answer can be explained later.
///
/// The product promise this serves is in the crate header: every machine
/// translation carries its provider, model and prompt version until a human
/// confirms it. The provider and model come from the provider layer; the
/// prompt's identity comes from here, because only this module knows which
/// version of which instruction a request was built with. A translation that
/// reads like a menu label is explained in one look at this record — or it is
/// unexplainable a year later, which is the failure this type exists against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtharTawjih {
    /// The prompt version, [`ISDAR_TAWJIH`] at the time the request was built.
    pub isdar: &'static str,
    /// The classification whose register instruction was used.
    pub tasnif: TasnifNass,
    /// How many protected tokens the instruction enumerated.
    pub adad_rumuz: usize,
    /// Whether the reply schema requested a confidence.
    pub bi_thiqa: bool,
    /// Whether the instruction stated a hard character limit.
    pub bi_hadd_ahruf: bool,
}

impl AtharTawjih {
    /// Records the shape of one request at the moment it is built.
    ///
    /// Taken from the same inputs the builders read, at the same time, so the
    /// record cannot describe a different prompt than the one sent.
    #[must_use]
    pub fn min_talab(talab: &SiyaqTalab, mahmi: &NassMahmi, bi_thiqa: bool) -> Self {
        Self {
            isdar: ISDAR_TAWJIH,
            tasnif: talab.tasnif,
            adad_rumuz: mahmi.adad_rumuz(),
            bi_thiqa,
            bi_hadd_ahruf: talab.quyud.aqsa_ahruf.is_some(),
        }
    }
}

impl std::fmt::Display for AtharTawjih {
    fn fmt(&self, mukhraj: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            mukhraj,
            "{}/{}/rumuz:{}/thiqa:{}/hadd:{}",
            self.isdar,
            self.tasnif.wasf_injilizi(),
            self.adad_rumuz,
            self.bi_thiqa,
            self.bi_hadd_ahruf
        )
    }
}
