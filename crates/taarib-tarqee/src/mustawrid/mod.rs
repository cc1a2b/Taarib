//! المستورد — taking somebody else's translation file and putting it onto this
//! project's string table without losing a line and without inventing one.
//!
//! Six interchange formats arrive here. A contributor who has already translated
//! a game with `XUnity.AutoTranslator`, or has a `.po` from a fan patch, or was
//! handed an XLIFF by an agency, should not retype any of it — and should not
//! have to trust that the import understood what it read.
//!
//! ## An imported translation is an unreviewed translation
//!
//! This is the rule the whole module is shaped around. Text arriving from
//! another tool enters at [`HalatMuraja::TarjamaAaliya`] or
//! [`HalatMuraja::Musawwada`] and **there is no path from an import to
//! [`HalatMuraja::Muakkada`]**.
//!
//! That is enforced structurally rather than by discipline. Everything this
//! module can say about a string's state is a [`HalatWarid`], which has three
//! variants and none of them maps to approval or rejection. A file cannot
//! express approval because there is no value in the type that means it. The
//! same reasoning is why [`taarib_mustalahat::muraja::ShahadatMuraja`] has no
//! public constructor and no `Deserialize`: an approval is a human's attestation
//! that they read the sentence, and a `.po` header claiming `X-Reviewed: yes` is
//! a line of text somebody typed.
//!
//! XLIFF's `state="final"` and `approved="yes"`, TMX's `creationid`, and a PO
//! file with no `fuzzy` flags all mean *another tool considered this done*. They
//! are mapped onto the draft side and the mapping is written down in
//! [`xliff`]'s header, because another tool's idea of finished is not this
//! project's review.
//!
//! ## Nothing is dropped, and the accounting proves it
//!
//! [`NatijatIstirad`] carries the number of entries the parser produced, and
//! every one of them is recorded into exactly one of four buckets by
//! [`NatijatIstirad::sajjil`] — the single recording method.
//! [`NatijatIstirad::farq_muhasaba`] returns the difference between the parsed
//! count and the recorded count, so a caller can ask whether anything went
//! missing and get a number rather than a debug assertion that a release build
//! compiles away.
//!
//! ## Matching, and why a guess is a proposal
//!
//! Three strategies run in order: the file's own key, the source text
//! byte-for-byte, then the source text normalized for whitespace and case. All
//! three are exact — they either identify a row or they do not.
//!
//! Similarity matching runs only when it is asked for, and its output is never
//! applied. The metric is **normalized Levenshtein similarity over Unicode
//! scalar values**: `1 − distance / max(len)`, with unit-cost insert, delete and
//! substitute, computed on the normalized forms of both strings. The default
//! threshold is [`HADD_TASHABUH_MABDAI`] and it cannot be lowered below
//! [`ADNA_HADD_TASHABUH`]. Strings shorter than [`ADNA_TUL_LITTAKHMEEN`]
//! characters are not offered at all, because at four characters a single edit
//! is a quarter of the string and every short label in a game resembles every
//! other one.
//!
//! A wrong guess puts the wrong sentence in front of a player, in a language the
//! contributor may be the only person on the project who reads. So a similarity
//! match arrives as an [`IqtirahIstirad`] with its candidates and their scores,
//! and a human picks.
//!
//! ## Format is read from content, never from the extension
//!
//! A `.txt` from `XUnity.AutoTranslator` and a `.txt` that is a TMX are
//! different files, and a `.csv` that is a Unity Localization export is not the
//! same thing as a `.csv` with three columns. [`shakhkhis`] looks at what is in
//! the file. When it cannot tell, it returns [`None`] and
//! [`wasf_ma_wujid`] describes what was actually there, so the refusal names
//! something rather than shrugging.

pub mod nusus_basita;
pub mod po_tmx;
pub mod xliff;

use std::collections::BTreeMap;
use std::path::Path;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_mustalahat::ruqaa::TareeqaTarjama;

use crate::khata::KhataTarqee;

/// The similarity a proposal must reach before it is offered at all.
///
/// Ninety per cent of the longer string. At that level the two texts differ by
/// a punctuation mark, a trailing space or one short word — the shapes a game
/// update actually produces. Below it the candidates stop being near-misses and
/// start being other sentences that happen to share a prefix.
pub const HADD_TASHABUH_MABDAI: f64 = 0.90;

/// The floor a caller's threshold is clamped to.
///
/// A configurable threshold with no floor is a field somebody sets to `0.3` to
/// make an import "work", after which every unmatched line acquires a plausible
/// candidate and the proposal list stops being reviewable.
pub const ADNA_HADD_TASHABUH: f64 = 0.75;

/// The shortest source text a similarity proposal is offered for.
///
/// Four characters. At three, one substitution is a third of the string, and
/// `Yes`/`Yep`/`Yen` are mutual near-misses in a menu that contains all three.
pub const ADNA_TUL_LITTAKHMEEN: usize = 4;

/// How many similarity candidates one proposal carries.
///
/// Three. A proposal is something a person reads; a list of forty candidates is
/// a list nobody reads, which is the same as applying the first one.
pub const AQSA_MURASHSHAHAT: usize = 3;

/// How much of a file the sniffer looks at.
///
/// A quarter of a mebibyte. Every format this module reads declares itself in
/// its first few lines, and a TMX with two hundred thousand units should not be
/// walked twice to find out it is a TMX.
pub const NAFIDHAT_TASHKHIS: usize = 262_144;

// ---------------------------------------------------------------------------
// The formats
// ---------------------------------------------------------------------------

/// An interchange format this build reads.
///
/// Seven variants for the six formats the crate documentation lists: XLIFF 1.2
/// and XLIFF 2.0 share a name and a file extension and are otherwise unrelated
/// grammars, with different element names, a different nesting model and
/// different state vocabularies. Folding them into one variant would mean one
/// parser branching internally on a version it had already had to detect.
///
/// **Every variant is parsed.** [`iqra_bi_sigha`] matches on this enum with no
/// wildcard arm, so a variant added without a parser does not compile. A format
/// this module recognised but could not read would be a format that reports a
/// successful import of nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SighatIstirad {
    /// `XUnity.AutoTranslator`'s translation cache: `original=translation` per
    /// line, with regex rules mixed in.
    XUnityAutoTranslator,
    /// XLIFF 1.2 — `<trans-unit>` with `<source>` and `<target>`.
    Xliff12,
    /// XLIFF 2.0 — `<unit>` and `<segment>`, a different grammar entirely.
    Xliff20,
    /// gettext PO or POT.
    GettextPo,
    /// TMX 1.4 translation memory.
    Tmx,
    /// A delimited table with columns the caller names.
    Csv,
    /// Unity Localization's own CSV export, one column per locale.
    UnityLocalizationCsv,
}

impl SighatIstirad {
    /// The format's name, as its own specification spells it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::XUnityAutoTranslator => "XUnity.AutoTranslator",
            Self::Xliff12 => "XLIFF 1.2",
            Self::Xliff20 => "XLIFF 2.0",
            Self::GettextPo => "gettext PO",
            Self::Tmx => "TMX 1.4",
            Self::Csv => "CSV",
            Self::UnityLocalizationCsv => "Unity Localization CSV",
        }
    }

    /// The sentence the import screen shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::XUnityAutoTranslator => "ذاكرة ترجمة XUnity.AutoTranslator",
            Self::Xliff12 => "ملف XLIFF بالإصدار 1.2",
            Self::Xliff20 => "ملف XLIFF بالإصدار 2.0",
            Self::GettextPo => "ملف gettext من نوع PO أو POT",
            Self::Tmx => "ذاكرة ترجمة TMX",
            Self::Csv => "جدول مفصول بفواصل بأعمدة يحدّدها المستورِد",
            Self::UnityLocalizationCsv => "تصدير Unity Localization بعمود لكل لغة",
        }
    }

    /// The extensions this format is usually written with.
    ///
    /// **Informational only.** Nothing in this module decides anything from an
    /// extension; [`shakhkhis`] reads content. This list exists so a file
    /// picker can offer a sensible filter and so a refusal can say what the
    /// name suggested against what the bytes said.
    #[must_use]
    pub const fn imtidadat(self) -> &'static [&'static str] {
        match self {
            Self::XUnityAutoTranslator => &["txt"],
            Self::Xliff12 | Self::Xliff20 => &["xlf", "xliff"],
            Self::GettextPo => &["po", "pot"],
            Self::Tmx => &["tmx"],
            Self::Csv | Self::UnityLocalizationCsv => &["csv", "tsv"],
        }
    }

    /// The most a translation from this format may claim on arrival.
    ///
    /// `XUnity.AutoTranslator` writes a machine-translation cache — that is what
    /// the tool is — so its content is machine output whoever ran it. Every
    /// other format can legitimately carry a person's own work, so the ceiling
    /// there is a draft, and the individual entry can still be lowered to
    /// machine by the file's own state vocabulary.
    ///
    /// Nothing here can return an approved state. See [`HalatWarid`].
    #[must_use]
    pub const fn saqf_hala(self) -> HalatWarid {
        match self {
            Self::XUnityAutoTranslator => HalatWarid::Aaliya,
            Self::Xliff12
            | Self::Xliff20
            | Self::GettextPo
            | Self::Tmx
            | Self::Csv
            | Self::UnityLocalizationCsv => HalatWarid::Musawwada,
        }
    }

    /// Whether this format carries its own review vocabulary.
    ///
    /// Read by the import screen to decide whether to show a state column at
    /// all: a CSV has no notion of state, and a column of identical values is
    /// noise.
    #[must_use]
    pub const fn laha_halat(self) -> bool {
        matches!(self, Self::Xliff12 | Self::Xliff20 | Self::GettextPo)
    }
}

/// The most an import may say about where a translation stands.
///
/// **Three variants, and the two that are missing are the point.** There is no
/// `Muakkada` and no `Marfuda` here, so no parser in this module — present or
/// future — has a value it could produce that would mean a human approved or
/// rejected the string. A file cannot mint an approval because the type an
/// importer speaks in has no word for one.
///
/// [`HalatWarid::ila_muraja`] is the only bridge to the project's own state, and
/// it is total: every variant maps, and none of them maps to approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalatWarid {
    /// A machine produced it and nobody has read it.
    Aaliya,
    /// Somebody's unfinished work — the default for a file a person authored.
    Musawwada,
    /// The file itself said this string wants a second pair of eyes.
    LilMuraja,
}

impl HalatWarid {
    /// The project state this arrives as.
    #[must_use]
    pub const fn ila_muraja(self) -> HalatMuraja {
        match self {
            Self::Aaliya => HalatMuraja::TarjamaAaliya,
            Self::Musawwada => HalatMuraja::Musawwada,
            Self::LilMuraja => HalatMuraja::LilMuraja,
        }
    }

    /// The weaker of two claims.
    ///
    /// Used to hold an entry's own state under its format's ceiling: an XLIFF
    /// unit marked `mt-suggestion` inside a file a person authored is still
    /// machine output, and a CSV row cannot climb above draft by having a
    /// column that says so.
    #[must_use]
    pub const fn adna(self, akhar: Self) -> Self {
        match (self, akhar) {
            (Self::Aaliya, _) | (_, Self::Aaliya) => Self::Aaliya,
            (Self::LilMuraja, _) | (_, Self::LilMuraja) => Self::LilMuraja,
            (Self::Musawwada, Self::Musawwada) => Self::Musawwada,
        }
    }

    /// The label the import preview shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Aaliya => "ترجمة آلية",
            Self::Musawwada => "مسوّدة",
            Self::LilMuraja => "للمراجعة",
        }
    }
}

// ---------------------------------------------------------------------------
// Plural forms
// ---------------------------------------------------------------------------

/// A CLDR plural category, as Arabic uses them.
///
/// Six, and the count is not negotiable. A PO file written for English declares
/// `nplurals=2`, where index 1 means *other*; in the Arabic rule index 1 means
/// *one* and *other* is index 5. Reading one file's indices with the other
/// file's rule puts the plural sentence in the singular slot, so
/// [`po_tmx`] refuses a plural entry whose declared form count is not six
/// rather than mapping across.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiatJama {
    /// صفر — zero.
    Sifr,
    /// واحد — one.
    Wahid,
    /// اثنان — two, which Arabic has and most rules do not.
    Ithnan,
    /// قليل — three to ten.
    Qalil,
    /// كثير — eleven to ninety-nine.
    Kathir,
    /// غير ذلك — everything else, including one hundred and a fraction.
    Ghayr,
}

impl FiatJama {
    /// The six categories in the order gettext indexes them for Arabic.
    pub const TARTEEB: [Self; 6] = [
        Self::Sifr,
        Self::Wahid,
        Self::Ithnan,
        Self::Qalil,
        Self::Kathir,
        Self::Ghayr,
    ];

    /// The category a `msgstr[N]` index denotes.
    ///
    /// [`None`] when the index is outside the six, which is a malformed entry
    /// rather than a seventh category.
    #[must_use]
    pub fn min_fahras(fahras: usize) -> Option<Self> {
        Self::TARTEEB.get(fahras).copied()
    }

    /// The CLDR name, which is what a glossary and a report use.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Sifr => "zero",
            Self::Wahid => "one",
            Self::Ithnan => "two",
            Self::Qalil => "few",
            Self::Kathir => "many",
            Self::Ghayr => "other",
        }
    }
}

/// One form of a plural set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuratJama {
    /// Which category.
    pub fia: FiatJama,
    /// The Arabic for it, which may legitimately be empty in a partly finished
    /// file.
    pub nass: String,
}

// ---------------------------------------------------------------------------
// One incoming entry
// ---------------------------------------------------------------------------

/// What kind of thing an incoming entry is.
///
/// Three, and the second exists because `XUnity.AutoTranslator` mixes literal
/// pairs and regular expressions in one file with one delimiter. A rule read as
/// a literal would put `^Chapter (\d+)$` into the game as the source text of a
/// string nobody can find.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum NawWarid {
    /// A literal source-to-target pair.
    Nass,
    /// A regular-expression rule, recorded exactly as written and **never
    /// compiled here**.
    ///
    /// This crate owns no regex engine, and it must not: a pattern evaluated by
    /// a different engine than the one the game's translator plugin uses is a
    /// pattern that matches different text. So the rule is carried verbatim,
    /// declined from the string mapping with [`SababRafd::QaidaNamatiya`], and
    /// listed for the contributor to reproduce in the runtime rules where such
    /// a thing belongs.
    QaidaNamatiya {
        /// The pattern, unquoted but otherwise untouched.
        namat: String,
        /// The replacement, with `$1`-style back-references left alone.
        badil: String,
        /// Whether this was the splitter form (`sr:`) rather than the plain
        /// regex form (`r:`).
        mujazzi: bool,
    },
    /// A plural set, with every form the file declared.
    Jama {
        /// The `msgid_plural`, which is the source's own plural form.
        asl_jama: String,
        /// The forms, in category order, as far as the file supplied them.
        suwar: Vec<SuratJama>,
    },
}

/// One entry exactly as the file gave it, before anything is matched.
///
/// Format-neutral on purpose: every parser in this module produces this, and the
/// matcher and the reporter know nothing about XLIFF or PO. A per-format match
/// path would be five places for the overwrite rule to be got wrong in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MudkhalWarid {
    /// The file's own key: a `trans-unit` id, a `tuid`, a CSV key column, the
    /// `msgctxt` and `msgid` joined. [`None`] when the format has no key.
    pub miftah: Option<String>,
    /// The source text, as the file wrote it. Placeholders in the source game's
    /// own syntax — `{0}`, `%s`, `<color=#fff>` — are left exactly as they are.
    pub masdar: String,
    /// The Arabic, when the file carried one.
    pub hadaf: Option<String>,
    /// The disambiguating context: PO's `msgctxt`, XLIFF's group path, a CSV
    /// comment column used as one.
    ///
    /// Kept separate from [`MudkhalWarid::miftah`] because two identical
    /// `msgid`s under different `msgctxt` are two different strings, and a
    /// parser that concatenated them into one key could not tell a caller which
    /// half disagreed.
    pub siyaq: Option<String>,
    /// Translator-facing notes the file carried.
    pub mulahazat: Vec<String>,
    /// Source references, `file:line` in PO's `#:` lines and `<context>` in
    /// XLIFF.
    pub marja: Vec<String>,
    /// The file's own state string, verbatim, so a report can quote it.
    pub hala_khaam: Option<String>,
    /// What that state maps to here.
    pub hala: HalatWarid,
    /// Whether the file itself marked this entry as unconfirmed — a PO `fuzzy`,
    /// an XLIFF `state-qualifier="fuzzy-match"`.
    ///
    /// Such an entry is never applied. It becomes an [`IqtirahIstirad`].
    pub muallam: bool,
    /// The standalone inline placeholders found in the source.
    pub dharrat_masdar: Vec<String>,
    /// The same for the target.
    pub dharrat_hadaf: Vec<String>,
    /// What kind of entry this is.
    pub naw: NawWarid,
    /// The line it started on, one-based. Zero when the format has no line
    /// structure worth naming.
    pub satr: usize,
}

impl MudkhalWarid {
    /// A plain pair with everything else empty.
    ///
    /// The shape every parser starts from, so that a field added later defaults
    /// consistently instead of being forgotten in four constructors.
    #[must_use]
    pub const fn jadeed(masdar: String, hadaf: Option<String>, satr: usize) -> Self {
        Self {
            miftah: None,
            masdar,
            hadaf,
            siyaq: None,
            mulahazat: Vec::new(),
            marja: Vec::new(),
            hala_khaam: None,
            hala: HalatWarid::Musawwada,
            muallam: false,
            dharrat_masdar: Vec::new(),
            dharrat_hadaf: Vec::new(),
            naw: NawWarid::Nass,
            satr,
        }
    }

    /// The atoms the source had and the target does not.
    ///
    /// Only the standalone kinds — a `<ph/>`, an `<x/>`, an `<ec/>`. A paired
    /// style tag that went missing is a formatting loss; a missing standalone
    /// placeholder is the player's own name gone out of the sentence, which is
    /// the failure `taarib_tarjama::hima` exists for.
    #[must_use]
    pub fn dharrat_mafquda(&self) -> Vec<String> {
        self.dharrat_masdar
            .iter()
            .filter(|dharra| !self.dharrat_hadaf.contains(dharra))
            .cloned()
            .collect()
    }

    /// The short label a report row shows for this entry.
    #[must_use]
    pub fn wasf(&self) -> String {
        let raas = self.miftah.as_deref().unwrap_or(&self.masdar);
        let mukhtasar: String = raas.chars().take(64).collect();
        if self.satr == 0 {
            mukhtasar
        } else {
            format!("{}: {mukhtasar}", self.satr)
        }
    }
}

/// Everything one parse produced.
///
/// The parsers stop here. Matching is a separate pass in [`tabiq`] over this
/// value, so a format's grammar and the project's matching rules are never
/// mixed in one function.
#[derive(Debug, Clone, Serialize)]
pub struct MilaffWarid {
    /// Which grammar produced it.
    pub sigha: SighatIstirad,
    /// The entries that are candidates for matching.
    pub madakhil: Vec<MudkhalWarid>,
    /// Entries the parser read and deliberately declined before matching — an
    /// empty target, a regex rule, a PO header.
    pub marfuda: Vec<MudkhalMarfud>,
    /// Things worth saying out loud that are not failures: which Arabic variant
    /// was taken when a file carried several, a plural expression that is not
    /// the canonical Arabic one.
    pub tanbihat: Vec<String>,
    /// The source language the file declared.
    pub lugha_masdar: Option<String>,
    /// The target language the file declared.
    pub lugha_hadaf: Option<String>,
    /// How many lines the file had, for the report.
    pub asturr: usize,
}

impl MilaffWarid {
    /// An empty result for a format, ready to be filled.
    #[must_use]
    pub const fn jadeed(sigha: SighatIstirad, asturr: usize) -> Self {
        Self {
            sigha,
            madakhil: Vec::new(),
            marfuda: Vec::new(),
            tanbihat: Vec::new(),
            lugha_masdar: None,
            lugha_hadaf: None,
            asturr,
        }
    }

    /// Every entry the parser produced, matched or declined.
    ///
    /// The number [`NatijatIstirad`] balances its accounting against.
    #[must_use]
    pub const fn adad_kulli(&self) -> usize {
        self.madakhil.len().saturating_add(self.marfuda.len())
    }

    /// Whether the file held nothing at all this module could use.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.adad_kulli() == 0
    }
}

// ---------------------------------------------------------------------------
// What happened to each entry
// ---------------------------------------------------------------------------

/// Why the parser read an entry and then declined it.
///
/// Distinct from failing to match. These entries were understood completely;
/// the module is refusing to turn them into a translation, and every variant
/// names a reason a person can act on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "sabab", rename_all = "snake_case")]
pub enum SababRafd {
    /// The entry has a source and no target at all — a POT template, an
    /// untranslated `trans-unit`.
    BilaHadaf,
    /// The target is present and empty.
    HadafFarigh,
    /// The target is byte-identical to the source, which is a passthrough
    /// rather than a translation into Arabic.
    HadafKaAlmasdar,
    /// A regular-expression rule. See [`NawWarid::QaidaNamatiya`].
    QaidaNamatiya,
    /// The file's metadata pseudo-entry: PO's `msgid ""` header, a TMX
    /// `<header>`.
    Taalim,
    /// A gettext entry commented out with `#~`.
    Mahjura,
    /// The unit says it must not be translated: XLIFF `translate="no"`.
    MamnuMinAttarjama,
    /// The translation lost a standalone placeholder the source had.
    ///
    /// Refused rather than imported with a warning, for the reason
    /// `taarib_tarjama::hima`'s header gives: a `%d` that came back as
    /// nothing reads fine, ships, and takes down the game's own formatter in
    /// front of a player.
    DharratMafquda {
        /// Which ones went missing.
        mafqud: Vec<String>,
    },
    /// A plural set. The project row holds one target and the file holds six,
    /// and choosing which one silently is the bug this refuses to commit.
    ///
    /// The set is not discarded — it travels in the entry and is offered as a
    /// proposal.
    MajmuatJama {
        /// How many forms the file supplied.
        suwar: usize,
    },
    /// A plural entry whose file declares a form count that is not Arabic's
    /// six, so its indices cannot be mapped.
    SuwarJamaGhayrArabiya {
        /// What the `Plural-Forms` header declared.
        adad: usize,
    },
    /// A plural entry in a file with no `Plural-Forms` header at all.
    SighatJamaMajhula,
    /// The row is in a locale that is not Arabic.
    LughaGhayrArabiya {
        /// The code found.
        ramz: String,
    },
    /// The unit carried no Arabic segment.
    LughaGhayrMawjuda,
    /// A record shorter than the column the caller named.
    AmudMafqud {
        /// The column asked for, zero-based.
        fahras: usize,
        /// How many columns the record had.
        mawjud: usize,
    },
    /// A line the format's own grammar does not define.
    ///
    /// Recorded as a declined entry rather than skipped, because a line the
    /// parser could not place is exactly the line a contributor needs to see:
    /// silently ignoring it is how half a translation file goes missing and the
    /// import still reports success.
    SatrGhayrMafhum {
        /// The line, truncated.
        juz: String,
    },
    /// The entry matched, every row it matched already holds something, and
    /// overwrite proposals were switched off.
    ///
    /// Recorded rather than dropped: an entry that matched and did nothing is
    /// exactly the outcome a contributor would otherwise never hear about.
    SufufMashghula {
        /// How many occupied rows it matched.
        adad: usize,
    },
}

impl SababRafd {
    /// The sentence the import report shows, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::BilaHadaf => "المُدخل بلا ترجمة أصلًا.".to_owned(),
            Self::HadafFarigh => "الترجمة فارغة.".to_owned(),
            Self::HadafKaAlmasdar => "الترجمة مطابقة للنص الأصلي حرفًا بحرف.".to_owned(),
            Self::QaidaNamatiya => "قاعدة تعبير نمطي، لا نصّ ثابت. لا تُطبَّق كنصّ حرفي.".to_owned(),
            Self::Taalim => "سطر بيانات الملف، لا عبارة قابلة للترجمة.".to_owned(),
            Self::Mahjura => "مُدخل مهجور في ملف gettext.".to_owned(),
            Self::MamnuMinAttarjama => "الملف يمنع ترجمة هذه الوحدة.".to_owned(),
            Self::DharratMafquda { mafqud } => {
                format!("فقدت الترجمة عناصر محفوظة: {}.", mafqud.join("، "))
            },
            Self::MajmuatJama { suwar } => {
                format!("مجموعة جمع بـ{suwar} صيغة، والصفّ يحمل ترجمة واحدة.")
            },
            Self::SuwarJamaGhayrArabiya { adad } => {
                format!("الملف يعلن {adad} صيغة جمع، والعربية ستّ. لا تُنقل الفهارس.")
            },
            Self::SighatJamaMajhula => {
                "لا ترويسة Plural-Forms في الملف، فلا معنى لفهارس الجمع.".to_owned()
            },
            Self::LughaGhayrArabiya { ramz } => format!("العمود بلغة ({ramz}) لا بالعربية."),
            Self::LughaGhayrMawjuda => "لا يحمل المُدخل مقطعًا عربيًّا.".to_owned(),
            Self::AmudMafqud { fahras, mawjud } => {
                format!("طُلب العمود رقم {fahras} والسجلّ يحمل {mawjud} عمودًا.")
            },
            Self::SatrGhayrMafhum { juz } => {
                format!("سطر لا تعرفه قواعد الصيغة: «{juz}».")
            },
            Self::SufufMashghula { adad } => {
                format!("تطابق المُدخل مع {adad} صفًّا، وكلّها يحمل ترجمة، ولم تُطلب اقتراحات الاستبدال.")
            },
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::BilaHadaf => "the entry carries no translation".to_owned(),
            Self::HadafFarigh => "the translation is empty".to_owned(),
            Self::HadafKaAlmasdar => "the translation is identical to the source".to_owned(),
            Self::QaidaNamatiya => {
                "a regular-expression rule, not a literal string; applying it as text would be \
                 wrong"
                    .to_owned()
            },
            Self::Taalim => "the file's own metadata entry, not a translatable string".to_owned(),
            Self::Mahjura => "an obsolete gettext entry".to_owned(),
            Self::MamnuMinAttarjama => "the file marks this unit as not translatable".to_owned(),
            Self::DharratMafquda { mafqud } => {
                format!(
                    "the translation lost these placeholders: {}",
                    mafqud.join(", ")
                )
            },
            Self::MajmuatJama { suwar } => format!(
                "a plural set with {suwar} form(s); the project row holds one target and \
                 choosing a form silently would be a guess"
            ),
            Self::SuwarJamaGhayrArabiya { adad } => format!(
                "the file declares {adad} plural form(s) and Arabic has six, so the indices \
                 cannot be carried across"
            ),
            Self::SighatJamaMajhula => {
                "the file has no Plural-Forms header, so its plural indices mean nothing here"
                    .to_owned()
            },
            Self::LughaGhayrArabiya { ramz } => format!("the column is {ramz}, not Arabic"),
            Self::LughaGhayrMawjuda => "the unit carries no Arabic segment".to_owned(),
            Self::AmudMafqud { fahras, mawjud } => {
                format!("column {fahras} was asked for and the record has {mawjud}")
            },
            Self::SatrGhayrMafhum { juz } => {
                format!("a line the format's grammar does not define: {juz:?}")
            },
            Self::SufufMashghula { adad } => format!(
                "the entry matched {adad} row(s), all of which already hold a translation, and \
                 overwrite proposals were switched off"
            ),
        }
    }
}

/// An entry the parser understood and declined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MudkhalMarfud {
    /// The entry, kept whole so the workshop can still show it.
    pub warid: MudkhalWarid,
    /// Why.
    pub sabab: SababRafd,
}

/// Why an entry found no row in the string table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "sabab", rename_all = "snake_case")]
pub enum SababAdamTatabuq {
    /// The file's key names nothing in this project.
    MiftahMajhul {
        /// The key that was looked up.
        miftah: String,
    },
    /// No row carries this source text, byte-for-byte or normalized.
    MasdarMajhul {
        /// The first sixty-four characters of it.
        masdar: String,
    },
    /// The key names several rows and the file gives nothing to choose between
    /// them.
    ///
    /// Refused rather than resolved by taking the first: two rows sharing a key
    /// mean the key is not the identity this project uses, and picking one puts
    /// the translation on a coin flip.
    MiftahMultabis {
        /// The key.
        miftah: String,
        /// How many rows answered to it.
        adad: usize,
    },
    /// Similarity matching ran and the best candidate did not reach the
    /// threshold.
    TashabuhDunAlhadd {
        /// What the best candidate scored.
        afdal: f64,
        /// What it had to reach.
        hadd: f64,
    },
    /// The source text is too short for a similarity proposal to mean anything.
    ///
    /// See [`ADNA_TUL_LITTAKHMEEN`].
    AqsarMinAlhadd {
        /// How many characters it had.
        tul: usize,
    },
    /// Similarity matching was not asked for, so nothing was guessed.
    TakhmeenGhayrMasmuh,
}

impl SababAdamTatabuq {
    /// The sentence the unmatched list shows, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::MiftahMajhul { miftah } => format!("المفتاح ({miftah}) ليس في جدول المشروع."),
            Self::MasdarMajhul { masdar } => {
                format!("لا صفّ يحمل هذا النص الأصلي: «{masdar}».")
            },
            Self::MiftahMultabis { miftah, adad } => {
                format!("المفتاح ({miftah}) يشير إلى {adad} صفًّا، ولا مُرجِّح بينها.")
            },
            Self::TashabuhDunAlhadd { afdal, hadd } => format!(
                "أقرب مرشّح تشابهه {:.0}٪ وحدّ الاقتراح {:.0}٪.",
                afdal * 100.0,
                hadd * 100.0
            ),
            Self::AqsarMinAlhadd { tul } => format!(
                "النص الأصلي {tul} محرفًا، وهو أقصر من أن يُقترح له شبيه ({ADNA_TUL_LITTAKHMEEN})."
            ),
            Self::TakhmeenGhayrMasmuh => "لم يُطلب التطابق التقريبي، فلم يُخمَّن شيء.".to_owned(),
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::MiftahMajhul { miftah } => {
                format!("the key {miftah} names nothing in this project")
            },
            Self::MasdarMajhul { masdar } => {
                format!("no row carries the source text \"{masdar}\"")
            },
            Self::MiftahMultabis { miftah, adad } => {
                format!("the key {miftah} answers to {adad} rows and nothing chooses between them")
            },
            Self::TashabuhDunAlhadd { afdal, hadd } => format!(
                "the closest candidate scored {:.0}% against a threshold of {:.0}%",
                afdal * 100.0,
                hadd * 100.0
            ),
            Self::AqsarMinAlhadd { tul } => format!(
                "the source is {tul} character(s), shorter than the {ADNA_TUL_LITTAKHMEEN} a \
                 similarity proposal is offered for"
            ),
            Self::TakhmeenGhayrMasmuh => {
                "similarity matching was not enabled, so nothing was guessed".to_owned()
            },
        }
    }
}

/// An entry that matched nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MudkhalGhayrMutatabiq {
    /// The entry.
    pub warid: MudkhalWarid,
    /// Why it matched nothing.
    pub sabab: SababAdamTatabuq,
}

/// Why a row that matched was not written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababTaajil {
    /// The row already carries a translation.
    TarjamaMawjuda,
    /// The row is frozen. See [`taarib_mustalahat::muraja::SijillMuraja::mujammad`].
    Mujammad,
    /// A human has already read this row's translation, whatever it says.
    QaraahaInsan,
}

impl SababTaajil {
    /// The sentence the proposal shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::TarjamaMawjuda => "الصفّ يحمل ترجمة بالفعل.",
            Self::Mujammad => "الصفّ مجمَّد ولا تكتب فيه العمليات الجُملية.",
            Self::QaraahaInsan => "قرأ إنسان ترجمة هذا الصفّ.",
        }
    }
}

/// One row an import would have written over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaffIstibdal {
    /// Which row.
    pub id: NassId,
    /// What it says now.
    pub qadeem: Option<String>,
    /// Where it stands now.
    pub hala_qadeema: HalatMuraja,
    /// Why it was not written to.
    pub sabab: SababTaajil,
}

/// One candidate a similarity proposal offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MurashshahTakhmeen {
    /// The row.
    pub id: NassId,
    /// Its source text, so the person choosing sees both sides.
    pub masdar_jadwal: String,
    /// The similarity, zero to one, by the metric this module's header states.
    pub darajat: f64,
}

/// Something an import proposes and will not do on its own.
///
/// Kept as one enum rather than three lists because a proposal is a proposal:
/// the review screen shows them together, and an entry belongs to exactly one
/// of these no matter which of the three reasons produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum IqtirahIstirad {
    /// The entry matched, and every row it matched already holds something.
    Istibdal {
        /// The entry.
        warid: MudkhalWarid,
        /// How it matched.
        tareeqa: TareeqaTatabuq,
        /// The rows, and what each one currently says.
        sufuf: Vec<SaffIstibdal>,
    },
    /// The entry matched nothing exactly, and similarity found candidates.
    TatabuqTaqreebi {
        /// The entry.
        warid: MudkhalWarid,
        /// The candidates, best first, at most [`AQSA_MURASHSHAHAT`].
        murashshahat: Vec<MurashshahTakhmeen>,
    },
    /// The entry matched cleanly and the **file itself** said it is not
    /// confirmed: a PO `fuzzy` flag, an XLIFF `fuzzy-match` qualifier, a plural
    /// set that has to be collapsed to one target.
    ///
    /// Not folded into the other two, because the uncertainty is the source
    /// tool's rather than this module's, and a contributor treats those
    /// differently: a `fuzzy` PO entry is usually right and wants a glance,
    /// while a similarity candidate is a guess about identity.
    MuallamMinAlmasdar {
        /// The entry.
        warid: MudkhalWarid,
        /// The rows it would go to.
        hadafat: Vec<NassId>,
        /// What the file said, verbatim.
        wasm: String,
    },
}

impl IqtirahIstirad {
    /// The entry behind the proposal.
    #[must_use]
    pub const fn warid(&self) -> &MudkhalWarid {
        match self {
            Self::Istibdal { warid, .. }
            | Self::TatabuqTaqreebi { warid, .. }
            | Self::MuallamMinAlmasdar { warid, .. } => warid,
        }
    }

    /// Whether this proposal would write over an existing translation.
    #[must_use]
    pub const fn yastabdil(&self) -> bool {
        matches!(self, Self::Istibdal { .. })
    }

    /// Whether this proposal is a guess about identity rather than about
    /// quality.
    #[must_use]
    pub const fn takhmeen(&self) -> bool {
        matches!(self, Self::TatabuqTaqreebi { .. })
    }
}

/// How an entry found its row.
///
/// Recorded on every match, because "matched by key" and "matched by lowercased
/// source text" carry different confidence and a reviewer looking at a bad
/// import needs to know which one produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TareeqaTatabuq {
    /// The file's key named the row.
    Miftah,
    /// The source texts are byte-identical.
    MasdarHarfi,
    /// The source texts agree once whitespace is collapsed and case folded.
    MasdarMuwahhad,
}

impl TareeqaTatabuq {
    /// The label the match column shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Miftah => "بالمفتاح",
            Self::MasdarHarfi => "بالنص الأصلي حرفيًّا",
            Self::MasdarMuwahhad => "بالنص الأصلي بعد التوحيد",
        }
    }
}

/// An entry that matched and will be applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TatabuqIstirad {
    /// The entry.
    pub warid: MudkhalWarid,
    /// How it matched.
    pub tareeqa: TareeqaTatabuq,
    /// The rows that have nothing yet and will receive this translation.
    pub jahiza: Vec<NassId>,
    /// Rows this same entry also matched that already hold something.
    ///
    /// Carried inside the match rather than split into a second entry, so that
    /// one input line stays one accounted outcome. They are proposals here
    /// exactly as they are in [`IqtirahIstirad::Istibdal`].
    pub mujjala: Vec<SaffIstibdal>,
}

/// What became of one incoming entry.
///
/// The single value [`NatijatIstirad::sajjil`] takes. Every entry produces one
/// of these and it is recorded once, which is what makes the accounting hold by
/// construction rather than by everybody remembering to increment a counter.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum HasilatMudkhal {
    /// Matched and applicable.
    Mutatabiq(TatabuqIstirad),
    /// Matched or nearly matched, and offered rather than applied.
    Muqtarah(IqtirahIstirad),
    /// Matched nothing.
    GhayrMutatabiq(MudkhalGhayrMutatabiq),
    /// Read and declined.
    Marfud(MudkhalMarfud),
}

// ---------------------------------------------------------------------------
// The result
// ---------------------------------------------------------------------------

/// Everything one import did, and everything it did not do.
///
/// **Serializable, deliberately not deserializable.** A result that could be
/// read back from a file would arrive with its accounting already summed and no
/// parse behind it, which is precisely the shape
/// [`NatijatIstirad::farq_muhasaba`] exists to detect. It travels into the
/// diagnostics bundle and comes back from nowhere.
#[derive(Debug, Clone, Serialize)]
pub struct NatijatIstirad {
    sigha: SighatIstirad,
    madakhil_maqrua: usize,
    asturr: usize,
    mutatabiqa: Vec<TatabuqIstirad>,
    iqtirahat: Vec<IqtirahIstirad>,
    ghayr_mutatabiqa: Vec<MudkhalGhayrMutatabiq>,
    marfuda: Vec<MudkhalMarfud>,
    tanbihat: Vec<String>,
}

impl NatijatIstirad {
    /// A result for a parse that produced `madakhil_maqrua` entries out of
    /// `asturr` lines.
    ///
    /// The parsed count is fixed here and never changes. Everything after this
    /// is a call to [`NatijatIstirad::sajjil`], so the two numbers the
    /// accounting compares come from two independent places — which is the only
    /// arrangement in which comparing them says anything.
    #[must_use]
    pub const fn jadeed(sigha: SighatIstirad, madakhil_maqrua: usize, asturr: usize) -> Self {
        Self {
            sigha,
            madakhil_maqrua,
            asturr,
            mutatabiqa: Vec::new(),
            iqtirahat: Vec::new(),
            ghayr_mutatabiqa: Vec::new(),
            marfuda: Vec::new(),
            tanbihat: Vec::new(),
        }
    }

    /// Records what became of one entry.
    ///
    /// **The only way anything enters this value.** One call, one entry, one
    /// bucket. There is no method that pushes into a bucket directly, so an
    /// outcome cannot be recorded twice and cannot be recorded nowhere.
    pub fn sajjil(&mut self, hasila: HasilatMudkhal) {
        match hasila {
            HasilatMudkhal::Mutatabiq(tatabuq) => self.mutatabiqa.push(tatabuq),
            HasilatMudkhal::Muqtarah(iqtirah) => self.iqtirahat.push(iqtirah),
            HasilatMudkhal::GhayrMutatabiq(ghayr) => self.ghayr_mutatabiqa.push(ghayr),
            HasilatMudkhal::Marfud(marfud) => self.marfuda.push(marfud),
        }
    }

    /// Adds a remark that is not about any one entry.
    pub fn nabbih(&mut self, tanbih: String) {
        self.tanbihat.push(tanbih);
    }

    /// Which format this came from.
    #[must_use]
    pub const fn sigha(&self) -> SighatIstirad {
        self.sigha
    }

    /// How many entries the parser produced.
    #[must_use]
    pub const fn madakhil_maqrua(&self) -> usize {
        self.madakhil_maqrua
    }

    /// How many lines the file had.
    #[must_use]
    pub const fn asturr(&self) -> usize {
        self.asturr
    }

    /// How many outcomes were recorded, across every bucket.
    #[must_use]
    pub const fn adad_musajjal(&self) -> usize {
        self.mutatabiqa
            .len()
            .saturating_add(self.iqtirahat.len())
            .saturating_add(self.ghayr_mutatabiqa.len())
            .saturating_add(self.marfuda.len())
    }

    /// The difference between what was parsed and what was accounted for.
    ///
    /// Zero means every entry the parser produced landed in exactly one bucket.
    /// A positive number means entries went missing between the parse and the
    /// report — the failure mode this whole result type is shaped to make
    /// visible. A negative number means something was recorded that the parser
    /// did not produce.
    ///
    /// A method rather than a `debug_assert!`, because the interesting case is
    /// a release build importing a forty-thousand-line file on a contributor's
    /// machine, and an assertion that is compiled out there is an assertion
    /// that never runs where it matters. A caller shows this number.
    #[must_use]
    pub fn farq_muhasaba(&self) -> i64 {
        let maqru = i64::try_from(self.madakhil_maqrua).unwrap_or(i64::MAX);
        let musajjal = i64::try_from(self.adad_musajjal()).unwrap_or(i64::MAX);
        maqru.saturating_sub(musajjal)
    }

    /// Whether the accounting balances.
    #[must_use]
    pub fn muhasaba_mutawazina(&self) -> bool {
        self.farq_muhasaba() == 0
    }

    /// Entries that matched and will be applied.
    #[must_use]
    pub fn mutatabiqa(&self) -> &[TatabuqIstirad] {
        &self.mutatabiqa
    }

    /// Everything the import proposes and will not do on its own.
    #[must_use]
    pub fn iqtirahat(&self) -> &[IqtirahIstirad] {
        &self.iqtirahat
    }

    /// Only the proposals that would write over an existing translation.
    pub fn iqtirahat_istibdal(&self) -> impl Iterator<Item = &IqtirahIstirad> {
        self.iqtirahat.iter().filter(|iqtirah| iqtirah.yastabdil())
    }

    /// Only the proposals that are a guess about which row an entry belongs to.
    pub fn iqtirahat_takhmeen(&self) -> impl Iterator<Item = &IqtirahIstirad> {
        self.iqtirahat.iter().filter(|iqtirah| iqtirah.takhmeen())
    }

    /// Entries that matched nothing, each with the reason it did not.
    #[must_use]
    pub fn ghayr_mutatabiqa(&self) -> &[MudkhalGhayrMutatabiq] {
        &self.ghayr_mutatabiqa
    }

    /// Entries the parser read and deliberately declined.
    #[must_use]
    pub fn marfuda(&self) -> &[MudkhalMarfud] {
        &self.marfuda
    }

    /// Remarks that belong to the file rather than to any one entry.
    #[must_use]
    pub fn tanbihat(&self) -> &[String] {
        &self.tanbihat
    }

    /// Every regular-expression rule the file carried.
    ///
    /// Derived from the declined list rather than stored twice, so a rule cannot
    /// appear in one place and not the other. `XUnity.AutoTranslator` files
    /// routinely carry these and a contributor needs them listed to reproduce
    /// them in the runtime rules, where a pattern belongs.
    pub fn qawaid_namatiya(&self) -> impl Iterator<Item = &MudkhalWarid> {
        self.marfuda
            .iter()
            .filter(|marfud| matches!(marfud.sabab, SababRafd::QaidaNamatiya))
            .map(|marfud| &marfud.warid)
    }

    /// How many project rows would actually be written.
    #[must_use]
    pub fn sufuf_qabila_lilkitaba(&self) -> usize {
        self.mutatabiqa.iter().fold(0_usize, |majmu, tatabuq| {
            majmu.saturating_add(tatabuq.jahiza.len())
        })
    }

    /// The paragraph the import preview shows, in Arabic.
    #[must_use]
    pub fn mulakhkhas_arabi(&self) -> String {
        let mut jumal = vec![format!(
            "قُرئ {} مُدخلًا من {} سطرًا بصيغة {}: تطابق {} مُدخلًا فتكتب في {} صفًّا، \
             و{} اقتراحًا لا يُطبَّق تلقائيًّا، و{} مُدخلًا بلا تطابق، و{} مُدخلًا مرفوضًا.",
            self.madakhil_maqrua,
            self.asturr,
            self.sigha.ism(),
            self.mutatabiqa.len(),
            self.sufuf_qabila_lilkitaba(),
            self.iqtirahat.len(),
            self.ghayr_mutatabiqa.len(),
            self.marfuda.len()
        )];
        let farq = self.farq_muhasaba();
        if farq != 0 {
            jumal.push(format!(
                "تحذير: الفارق بين ما قُرئ وما سُجّل {farq}. سقط شيء بين التحليل والتقرير."
            ));
        }
        jumal.push("كل ما استُورد يدخل مسوّدةً أو ترجمةً آلية. لا يصل الاستيراد إلى الاعتماد.".to_owned());
        jumal.join(" ")
    }

    /// The same, in English.
    #[must_use]
    pub fn mulakhkhas_injilizi(&self) -> String {
        let mut jumal = vec![format!(
            "{} entries read from {} lines of {}: {} matched and would write {} row(s), {} \
             proposed and not applied, {} unmatched, {} declined.",
            self.madakhil_maqrua,
            self.asturr,
            self.sigha.ism(),
            self.mutatabiqa.len(),
            self.sufuf_qabila_lilkitaba(),
            self.iqtirahat.len(),
            self.ghayr_mutatabiqa.len(),
            self.marfuda.len()
        )];
        let farq = self.farq_muhasaba();
        if farq != 0 {
            jumal.push(format!(
                "Accounting discrepancy of {farq}: something was lost between the parse and the \
                 report."
            ));
        }
        jumal.push(
            "Everything imported arrives as a draft or as machine output. An import never reaches \
             approved."
                .to_owned(),
        );
        jumal.join(" ")
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// An encoding the caller states rather than one this module infers.
///
/// Three Unicode forms and nothing else. A legacy single-byte or multi-byte
/// encoding is refused by name — see [`nusus_basita`]'s header for why guessing
/// one produces text that looks like a translation and is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TarmizMuhaddad {
    /// UTF-8, with or without a byte-order mark.
    Utf8,
    /// UTF-16 little-endian.
    Utf16Saghir,
    /// UTF-16 big-endian.
    Utf16Kabir,
}

impl TarmizMuhaddad {
    /// The name the encoding registry gives it, for a report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Saghir => "UTF-16LE",
            Self::Utf16Kabir => "UTF-16BE",
        }
    }
}

/// How a header row is decided for the delimited formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KashfTarwisa {
    /// Decide from the first record's contents.
    #[default]
    Talqai,
    /// The first record is a header.
    Mawjuda,
    /// There is no header; the first record is data.
    Ghaiba,
}

/// Which column of a delimited file holds what.
///
/// Indices rather than names, because a fan-made CSV routinely has no header
/// at all, and a header that exists is routinely in the contributor's own
/// language. When a header *is* present and an index is left unset, the column
/// is resolved by name from the header — never the other way round, so an
/// explicit index always wins over a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmidatCsv {
    /// The delimiter, one ASCII byte.
    pub mahdid: u8,
    /// Whether the first record is a header.
    pub tarwisa: KashfTarwisa,
    /// The column holding the engine's own key, when there is one.
    pub miftah: Option<usize>,
    /// The column holding the source text.
    pub masdar: Option<usize>,
    /// The column holding the Arabic.
    pub hadaf: Option<usize>,
    /// The column holding a translator note.
    pub mulahaza: Option<usize>,
}

impl Default for AmidatCsv {
    /// Comma-delimited, header detected, key in column zero, source in one,
    /// target in two.
    ///
    /// The shape almost every hand-made translation sheet has. Every field is
    /// overridable, because "almost every" is not "every" and an importer that
    /// insists on its own layout is an importer nobody uses twice.
    fn default() -> Self {
        Self {
            mahdid: b',',
            tarwisa: KashfTarwisa::Talqai,
            miftah: Some(0),
            masdar: Some(1),
            hadaf: Some(2),
            mulahaza: None,
        }
    }
}

/// What a caller may change about an import.
#[derive(Debug, Clone, PartialEq)]
pub struct IstiradKhiyarat {
    /// An encoding stated rather than detected. [`None`] means detect from a
    /// byte-order mark and otherwise require UTF-8.
    pub tarmiz: Option<TarmizMuhaddad>,
    /// Whether to compute proposals for rows that already hold a translation.
    ///
    /// Turning this off does not make the import overwrite them — it makes the
    /// import stop mentioning them. There is no setting that applies one.
    pub iqtirah_istibdal: bool,
    /// Whether similarity matching may run at all.
    ///
    /// Off by default. An import that silently acquires near-miss matches is an
    /// import whose failures are invisible.
    pub samah_bittakhmeen: bool,
    /// The similarity a candidate must reach, clamped to
    /// [`ADNA_HADD_TASHABUH`].
    pub hadd_tashabuh: f64,
    /// The delimited-format column layout.
    pub amida: AmidatCsv,
    /// The Arabic variant to prefer when a file carries several, as BCP-47.
    pub ramz_lugha: String,
    /// Who is running the import, when the caller knows.
    ///
    /// Present, the import lands at [`HalatMuraja::Musawwada`] with this person
    /// recorded as the author of the transition. Absent, it lands at
    /// [`HalatMuraja::TarjamaAaliya`], which claims less: an unattributed
    /// translation is one no identified human has vouched for.
    pub musahim: Option<MusahimId>,
    /// The moment, in seconds since the Unix epoch, supplied by the caller.
    ///
    /// Never read from a clock here, for the reason every other module in this
    /// workspace gives: a stored history has to be reproducible.
    pub lahza: u64,
    /// The same moment as RFC 3339, for [`MudkhalNass::akhir_tabdeel`].
    pub waqt: Option<String>,
}

impl Default for IstiradKhiyarat {
    fn default() -> Self {
        Self {
            tarmiz: None,
            iqtirah_istibdal: true,
            samah_bittakhmeen: false,
            hadd_tashabuh: HADD_TASHABUH_MABDAI,
            amida: AmidatCsv::default(),
            ramz_lugha: "ar".to_owned(),
            musahim: None,
            lahza: 0,
            waqt: None,
        }
    }
}

impl IstiradKhiyarat {
    /// The threshold that will actually be used.
    ///
    /// Clamped into `[ADNA_HADD_TASHABUH, 1.0]`. `f64::max` and `f64::min`
    /// return the other operand for a NaN, so a threshold that arrived as NaN
    /// comes out as the floor rather than as a comparison that answers `false`
    /// to everything and quietly disables the feature.
    #[must_use]
    pub const fn hadd_faal(&self) -> f64 {
        self.hadd_tashabuh.max(ADNA_HADD_TASHABUH).min(1.0)
    }

    /// The state an entry lands at when the file says nothing more specific.
    ///
    /// With nobody named, nothing is claimed: the entry arrives as machine
    /// output, which records no author and asserts that no human has read it.
    #[must_use]
    pub const fn hala_asasiya(&self, sigha: SighatIstirad) -> HalatWarid {
        if self.musahim.is_some() {
            sigha.saqf_hala()
        } else {
            HalatWarid::Aaliya
        }
    }
}

// ---------------------------------------------------------------------------
// Sniffing
// ---------------------------------------------------------------------------

/// The format a file is in, read from what is inside it.
///
/// **Never from the extension.** `_AutoGeneratedTranslations.txt` and a TMX
/// somebody saved as `.txt` are both `.txt`; a Unity Localization export and a
/// three-column spreadsheet are both `.csv`. The name of a file is a claim its
/// author made and the bytes are the fact.
///
/// [`None`] when nothing here recognises it, including the cases where the file
/// is *nearly* something known — an `<xliff version="2.1">`, a TMX declaring
/// version 3. Those return [`None`] rather than the closest match, because
/// reading a 2.1 document with a 2.0 parser produces a partial import that
/// looks complete, which is the failure
/// [`KhataTarqee::SighatHuzmaGhayrMaduma`] exists to prevent one level up.
#[must_use]
pub fn shakhkhis(nass: &str) -> Option<SighatIstirad> {
    let nafidha = nafidhat_tashkhis(nass);

    if let Some((ism, sifat)) = unsur_jidhr(nafidha) {
        return match ism.as_str() {
            "xliff" => match sifa_khaam(&sifat, "version").as_deref() {
                Some("1.2") => Some(SighatIstirad::Xliff12),
                Some("2.0") => Some(SighatIstirad::Xliff20),
                // A root with no version at all is malformed but recoverable:
                // the two grammars name their units differently, and that
                // difference is a fact about the document rather than a guess.
                None if nafidha.contains("<trans-unit") => Some(SighatIstirad::Xliff12),
                None if nafidha.contains("<unit ") || nafidha.contains("<segment") => {
                    Some(SighatIstirad::Xliff20)
                },
                _ => None,
            },
            "tmx" => match sifa_khaam(&sifat, "version") {
                // 1.1 through 1.4 share the `<tu>`/`<tuv>`/`<seg>` shape this
                // reads. Anything else is refused rather than attempted.
                Some(nuskha) if nuskha.starts_with("1.") => Some(SighatIstirad::Tmx),
                None => Some(SighatIstirad::Tmx),
                Some(_) => None,
            },
            _ => None,
        };
    }

    if yushbih_po(nafidha) {
        return Some(SighatIstirad::GettextPo);
    }
    if yushbih_wahdat_tawteen(nafidha) {
        return Some(SighatIstirad::UnityLocalizationCsv);
    }
    // The regex prefixes are unique to `XUnity.AutoTranslator` and settle the
    // question before the structural CSV test, which a file of `r:"..."=...`
    // lines containing commas would otherwise pass.
    if yahmil_qawaid_xunity(nafidha) {
        return Some(SighatIstirad::XUnityAutoTranslator);
    }
    if yushbih_jadwal(nafidha) {
        return Some(SighatIstirad::Csv);
    }
    if yushbih_xunity(nafidha) {
        return Some(SighatIstirad::XUnityAutoTranslator);
    }
    None
}

/// What the file looked like, for a refusal that names something.
///
/// Read by [`KhataTarqee::SighatIstiradMajhula`]'s `wujid` field. A refusal
/// saying "unrecognised" tells a contributor nothing; one saying `an <xliff>
/// root declaring version="2.1"` tells them exactly what to do next.
#[must_use]
pub fn wasf_ma_wujid(nass: &str) -> String {
    let nafidha = nafidhat_tashkhis(nass);
    if nafidha.trim().is_empty() {
        return "an empty file".to_owned();
    }
    if let Some((ism, sifat)) = unsur_jidhr(nafidha) {
        return match sifa_khaam(&sifat, "version") {
            Some(nuskha) => {
                format!("XML whose root element is <{ism}>, declaring version=\"{nuskha}\"")
            },
            None => format!("XML whose root element is <{ism}>, declaring no version"),
        };
    }
    let awwal: String = nafidha
        .lines()
        .find(|satr| !satr.trim().is_empty())
        .unwrap_or_default()
        .chars()
        .take(80)
        .collect();
    format!("text whose first non-empty line is {awwal:?}")
}

/// The format, or a refusal naming what was found.
///
/// # Errors
///
/// [`KhataTarqee::SighatIstiradMajhula`], carrying [`wasf_ma_wujid`]'s
/// description of the file.
pub fn shakhkhis_aw_irfud(masar: &Path, nass: &str) -> Result<SighatIstirad, KhataTarqee> {
    shakhkhis(nass).ok_or_else(|| KhataTarqee::SighatIstiradMajhula {
        masar: masar.to_path_buf(),
        wujid: wasf_ma_wujid(nass),
    })
}

/// The leading slice the sniffer reads, cut on a character boundary.
fn nafidhat_tashkhis(nass: &str) -> &str {
    if nass.len() <= NAFIDHAT_TASHKHIS {
        return nass;
    }
    let mut hadd = NAFIDHAT_TASHKHIS;
    while hadd > 0 && !nass.is_char_boundary(hadd) {
        hadd = hadd.saturating_sub(1);
    }
    nass.get(..hadd).unwrap_or(nass)
}

/// The first real element in a document, with its raw attribute text.
///
/// Skips the XML declaration, comments, processing instructions and the
/// document type declaration, which is why it can be run before the parser that
/// refuses the last of those.
fn unsur_jidhr(nass: &str) -> Option<(String, String)> {
    let mut baqi = nass;
    loop {
        let bidaya = baqi.find('<')?;
        let baad = baqi.get(bidaya.saturating_add(1)..)?;
        let awwal = baad.chars().next()?;
        if awwal == '?' || awwal == '!' || awwal == '/' {
            let taqaddum = bidaya.saturating_add(1);
            baqi = baqi.get(taqaddum..)?;
            continue;
        }
        if !awwal.is_alphabetic() && awwal != '_' {
            let taqaddum = bidaya.saturating_add(1);
            baqi = baqi.get(taqaddum..)?;
            continue;
        }
        let tul_ism = baad
            .find(|harf: char| harf.is_whitespace() || harf == '>' || harf == '/')
            .unwrap_or(baad.len());
        let ism = baad.get(..tul_ism).unwrap_or_default();
        let baqi_unsur = baad.get(tul_ism..).unwrap_or_default();
        let tul_sifat = baqi_unsur.find('>').unwrap_or(baqi_unsur.len());
        let sifat = baqi_unsur.get(..tul_sifat).unwrap_or_default();
        // A namespace prefix is not part of the name that identifies a format.
        let mahalli = ism.rsplit(':').next().unwrap_or(ism);
        return Some((mahalli.to_ascii_lowercase(), sifat.to_owned()));
    }
}

/// One attribute out of a raw attribute run, without an XML parser.
///
/// Used only by the sniffer, on text the real parser has not seen yet. Entity
/// references are not expanded here and do not need to be: a version number and
/// a locale code do not contain them, and anything that did would simply fail
/// to match and fall through to a refusal.
fn sifa_khaam(sifat: &str, matlub: &str) -> Option<String> {
    let mut baqi = sifat;
    while let Some(mawqi) = baqi.find('=') {
        let qabl = baqi.get(..mawqi).unwrap_or_default();
        let ism = qabl.split_whitespace().next_back().unwrap_or_default();
        let baad = baqi
            .get(mawqi.saturating_add(1)..)
            .unwrap_or_default()
            .trim_start();
        let iqtibas = baad.chars().next().unwrap_or(' ');
        let (qeema, taqaddum) = if iqtibas == '"' || iqtibas == '\'' {
            let jasad = baad.get(1..).unwrap_or_default();
            let tul = jasad.find(iqtibas).unwrap_or(jasad.len());
            (jasad.get(..tul).unwrap_or_default(), tul.saturating_add(2))
        } else {
            let tul = baad.find(char::is_whitespace).unwrap_or(baad.len());
            (baad.get(..tul).unwrap_or_default(), tul)
        };
        if ism
            .rsplit(':')
            .next()
            .unwrap_or(ism)
            .eq_ignore_ascii_case(matlub)
        {
            return Some(qeema.to_owned());
        }
        let mustahlak = baqi
            .len()
            .saturating_sub(baad.len())
            .saturating_add(taqaddum);
        baqi = baqi.get(mustahlak..).unwrap_or_default();
        if baqi.is_empty() {
            break;
        }
    }
    None
}

/// Whether the text reads as a gettext catalogue.
fn yushbih_po(nass: &str) -> bool {
    let mut ma_id = 0_usize;
    let mut ma_str = 0_usize;
    let mut gharib = 0_usize;
    for satr in nass.lines().take(400) {
        let mahdhub = satr.trim_start();
        if mahdhub.is_empty() || mahdhub.starts_with('#') || mahdhub.starts_with('"') {
            continue;
        }
        if mahdhub.starts_with("msgid") || mahdhub.starts_with("msgctxt") {
            ma_id = ma_id.saturating_add(1);
        } else if mahdhub.starts_with("msgstr") {
            ma_str = ma_str.saturating_add(1);
        } else {
            gharib = gharib.saturating_add(1);
        }
    }
    ma_id > 0 && ma_str > 0 && gharib == 0
}

/// Whether the first record looks like Unity Localization's own export header.
fn yushbih_wahdat_tawteen(nass: &str) -> bool {
    let Some(tarwisa) = nass.lines().find(|satr| !satr.trim().is_empty()) else {
        return false;
    };
    for mahdid in [',', '\t', ';'] {
        let huqul = huqul_tashkhis(tarwisa, mahdid);
        let Some(awwal) = huqul.first() else { continue };
        if !awwal.trim().eq_ignore_ascii_case("key") || huqul.len() < 2 {
            continue;
        }
        let laha_amida = huqul.iter().skip(1).any(|haql| {
            haql.trim().eq_ignore_ascii_case("shared comments")
                || haql.trim().eq_ignore_ascii_case("id")
                || ramz_lugha_min_unwan(haql).is_some()
        });
        if laha_amida {
            return true;
        }
    }
    false
}

/// Whether the text carries `XUnity.AutoTranslator`'s regex rule prefixes.
fn yahmil_qawaid_xunity(nass: &str) -> bool {
    nass.lines()
        .take(400)
        .any(|satr| satr.starts_with("r:\"") || satr.starts_with("sr:\""))
}

/// Whether the text is a run of `original=translation` lines.
fn yushbih_xunity(nass: &str) -> bool {
    let mut bihi = 0_usize;
    let mut bidunih = 0_usize;
    for satr in nass.lines().take(400) {
        let mahdhub = satr.trim_end_matches(['\r', '\n']);
        if mahdhub.trim().is_empty() || mahdhub.starts_with("//") {
            continue;
        }
        if mawqi_fasil_xunity(mahdhub).is_some() {
            bihi = bihi.saturating_add(1);
        } else {
            bidunih = bidunih.saturating_add(1);
        }
    }
    bihi > 0 && bihi >= bidunih.saturating_mul(2)
}

/// Whether the text is a delimited table of consistent width.
fn yushbih_jadwal(nass: &str) -> bool {
    for mahdid in [',', '\t', ';'] {
        let mut mutawaqqa = 0_usize;
        let mut muttafiqa = 0_usize;
        let mut kulli = 0_usize;
        for satr in nass.lines().take(50) {
            if satr.trim().is_empty() {
                continue;
            }
            let adad = huqul_tashkhis(satr, mahdid).len();
            kulli = kulli.saturating_add(1);
            if mutawaqqa == 0 {
                mutawaqqa = adad;
            }
            if adad == mutawaqqa {
                muttafiqa = muttafiqa.saturating_add(1);
            }
        }
        if mutawaqqa >= 2 && kulli > 0 && muttafiqa.saturating_mul(5) >= kulli.saturating_mul(3) {
            return true;
        }
    }
    false
}

/// A record split for sniffing only, honouring quotes but not embedded
/// newlines.
///
/// The real parse goes through the `csv` crate, which handles the whole
/// grammar. This exists because deciding *which* parser to run cannot itself
/// require having chosen one.
fn huqul_tashkhis(satr: &str, mahdid: char) -> Vec<String> {
    let mut huqul = Vec::new();
    let mut hali = String::new();
    let mut dakhil = false;
    let mut ahruf = satr.chars().peekable();
    while let Some(harf) = ahruf.next() {
        if dakhil {
            if harf == '"' {
                if ahruf.peek() == Some(&'"') {
                    let _ = ahruf.next();
                    hali.push('"');
                } else {
                    dakhil = false;
                }
            } else {
                hali.push(harf);
            }
        } else if harf == '"' && hali.trim().is_empty() {
            hali.clear();
            dakhil = true;
        } else if harf == mahdid {
            huqul.push(std::mem::take(&mut hali));
        } else if harf != '\r' {
            hali.push(harf);
        }
    }
    huqul.push(hali);
    huqul
}

/// The offset of the `=` that separates an `XUnity.AutoTranslator` pair.
///
/// The **first unescaped** one. A source string containing `=` escapes it as
/// `\=`, and taking the last separator instead of the first would move the
/// split whenever a translation contained one.
#[must_use]
pub fn mawqi_fasil_xunity(satr: &str) -> Option<usize> {
    let mut mailat = false;
    for (izaha, harf) in satr.char_indices() {
        if mailat {
            mailat = false;
            continue;
        }
        match harf {
            '\\' => mailat = true,
            '=' => return Some(izaha),
            _ => {},
        }
    }
    None
}

/// The BCP-47 code inside a Unity Localization column heading.
///
/// Unity writes a locale column as the locale's name followed by its code in
/// parentheses — `Arabic (ar)`, `Arabic(ar-SA)` — and older exports write the
/// bare code. Both shapes are read; anything else returns [`None`] and the
/// column is treated as data rather than as a locale.
#[must_use]
pub fn ramz_lugha_min_unwan(unwan: &str) -> Option<String> {
    let mahdhub = unwan.trim();
    if let Some(fath) = mahdhub.rfind('(') {
        let jasad = mahdhub.get(fath.saturating_add(1)..)?;
        let ighlaq = jasad.find(')')?;
        let ramz = jasad.get(..ighlaq)?.trim();
        if ramz_bcp47(ramz) {
            return Some(ramz.to_owned());
        }
        return None;
    }
    if ramz_bcp47(mahdhub) {
        Some(mahdhub.to_owned())
    } else {
        None
    }
}

/// Whether a string is shaped like a BCP-47 tag this module would act on.
///
/// Deliberately narrow: two or three letters, optionally followed by
/// subtags of letters or digits separated by `-` or `_`. Enough to recognise
/// `ar`, `ar-SA`, `zh-Hans-CN` and `pt_BR`, and not enough to mistake a column
/// headed `Notes` for a locale.
fn ramz_bcp47(ramz: &str) -> bool {
    let mut ajza = ramz.split(['-', '_']);
    let Some(awwal) = ajza.next() else {
        return false;
    };
    if !(2..=3).contains(&awwal.chars().count()) || !awwal.chars().all(|h| h.is_ascii_alphabetic())
    {
        return false;
    }
    ajza.all(|juz| {
        !juz.is_empty()
            && juz.chars().count() <= 8
            && juz.chars().all(|harf| harf.is_ascii_alphanumeric())
    })
}

/// Whether a language tag denotes Arabic.
///
/// One implementation for all six formats, so "what counts as Arabic" is one
/// opinion rather than five. Matches on the primary subtag only: `ar`, `ar-SA`,
/// `ar_EG`, `ara` (ISO 639-2/T) and `arb` (ISO 639-3, Standard Arabic) are all
/// Arabic, and the region is a preference rather than a qualification —
/// refusing `ar-EG` because a project asked for `ar-SA` would refuse a file
/// that is entirely usable.
#[must_use]
pub fn lugha_arabiya(ramz: &str) -> bool {
    let awwal = ramz.split(['-', '_']).next().unwrap_or_default();
    awwal.eq_ignore_ascii_case("ar")
        || awwal.eq_ignore_ascii_case("ara")
        || awwal.eq_ignore_ascii_case("arb")
}

/// How well an Arabic tag answers a preference, lower being better.
///
/// Zero for the exact tag asked for, one for a bare `ar`, two for any other
/// Arabic variant, [`None`] for something that is not Arabic at all. Used to
/// choose between several Arabic segments in one TMX unit or several Arabic
/// columns in one export — and the loser is always named in the report rather
/// than dropped in silence.
#[must_use]
pub fn martabat_arabiya(ramz: &str, mufaddal: &str) -> Option<u8> {
    if !lugha_arabiya(ramz) {
        return None;
    }
    let musawwa = ramz.replace('_', "-");
    if musawwa.eq_ignore_ascii_case(&mufaddal.replace('_', "-")) {
        return Some(0);
    }
    if musawwa.eq_ignore_ascii_case("ar") {
        return Some(1);
    }
    Some(2)
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// Parses a decoded file with the parser for its format.
///
/// The match below has **no wildcard arm**. Adding a variant to
/// [`SighatIstirad`] without writing its parser fails to compile, which is the
/// only way to keep the promise that a format this build recognises is a format
/// this build reads. A `_ => Err(unsupported)` arm here would turn that promise
/// into a comment.
///
/// # Errors
///
/// Whatever the format's own parser refuses: [`KhataTarqee::IstiradFashil`] for
/// a malformed document, an external DTD, an encoding declaration that
/// contradicts the bytes, or a Unity export with no Arabic column.
pub fn iqra_bi_sigha(
    sigha: SighatIstirad,
    masar: &Path,
    nass: &str,
    khiyarat: &IstiradKhiyarat,
) -> Result<MilaffWarid, KhataTarqee> {
    match sigha {
        SighatIstirad::XUnityAutoTranslator => nusus_basita::iqra_xunity(masar, nass, khiyarat),
        SighatIstirad::Csv => nusus_basita::iqra_jadwal(masar, nass, khiyarat),
        SighatIstirad::UnityLocalizationCsv => {
            nusus_basita::iqra_wahdat_tawteen(masar, nass, khiyarat)
        },
        SighatIstirad::Xliff12 => xliff::iqra_nuskha_ula(masar, nass, khiyarat),
        SighatIstirad::Xliff20 => xliff::iqra_nuskha_thaniya(masar, nass, khiyarat),
        SighatIstirad::GettextPo => po_tmx::iqra_po(masar, nass, khiyarat),
        SighatIstirad::Tmx => po_tmx::iqra_tmx(masar, nass, khiyarat),
    }
}

/// Reads a file from disk and parses it, without matching anything.
///
/// What the import preview runs: a contributor sees what the file contains
/// before a project is involved.
///
/// # Errors
///
/// [`KhataTarqee::KhataMalaf`] when the file cannot be read,
/// [`KhataTarqee::SighatIstiradMajhula`] when nothing recognises it, and
/// [`KhataTarqee::IstiradFashil`] when the encoding cannot be established or
/// the format's own parser refuses the document.
pub fn iqra(masar: &Path, khiyarat: &IstiradKhiyarat) -> Result<MilaffWarid, KhataTarqee> {
    let bayt = std::fs::read(masar).map_err(|sabab| KhataTarqee::KhataMalaf {
        masar: masar.to_path_buf(),
        sabab,
    })?;
    let nass = nusus_basita::fak_tarmiz(masar, &bayt, khiyarat)?;
    let sigha = shakhkhis_aw_irfud(masar, &nass)?;
    tracing::debug!(masar = %masar.display(), sigha = sigha.ism(), "import format identified");
    iqra_bi_sigha(sigha, masar, &nass, khiyarat)
}

/// Reads a file and matches it against a project's string table.
///
/// The whole import in one call. Nothing is written: the result says what would
/// happen, and [`tabbiq`] is what makes it happen.
///
/// # Errors
///
/// As [`iqra`].
pub fn istawrid(
    masar: &Path,
    jadwal: &[MudkhalNass],
    khiyarat: &IstiradKhiyarat,
) -> Result<NatijatIstirad, KhataTarqee> {
    let milaff = iqra(masar, khiyarat)?;
    Ok(tabiq(&milaff, jadwal, khiyarat))
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// The lookup structure an import matches against.
///
/// Built once per table and reusable across several files, which matters when a
/// contributor imports a directory of eleven `.po` files against a table of
/// forty thousand rows.
#[derive(Debug)]
pub struct FahrasJadwal<'a> {
    jadwal: &'a [MudkhalNass],
    mawqi: BTreeMap<NassId, usize>,
    bilmiftah: BTreeMap<String, Vec<NassId>>,
    bilmasdar: BTreeMap<String, Vec<NassId>>,
    bilmuwahhad: BTreeMap<String, Vec<NassId>>,
}

impl<'a> FahrasJadwal<'a> {
    /// Indexes a table by identity, by every key spelling a row answers to, and
    /// by its source text in both raw and normalized form.
    #[must_use]
    pub fn jadeed(jadwal: &'a [MudkhalNass]) -> Self {
        let mut fahras = Self {
            jadwal,
            mawqi: BTreeMap::new(),
            bilmiftah: BTreeMap::new(),
            bilmasdar: BTreeMap::new(),
            bilmuwahhad: BTreeMap::new(),
        };
        for (mawdi, saff) in jadwal.iter().enumerate() {
            let _ = fahras.mawqi.insert(saff.id, mawdi);
            for miftah in mafatih_saff(saff) {
                let qaima = fahras.bilmiftah.entry(miftah).or_default();
                if !qaima.contains(&saff.id) {
                    qaima.push(saff.id);
                }
            }
            let bilmasdar = fahras.bilmasdar.entry(saff.masdar.clone()).or_default();
            if !bilmasdar.contains(&saff.id) {
                bilmasdar.push(saff.id);
            }
            let muwahhad = wahhid(&saff.masdar);
            if !muwahhad.is_empty() {
                let bilmuwahhad = fahras.bilmuwahhad.entry(muwahhad).or_default();
                if !bilmuwahhad.contains(&saff.id) {
                    bilmuwahhad.push(saff.id);
                }
            }
        }
        fahras
    }

    /// The rows in the table, in the order they were given.
    #[must_use]
    pub const fn jadwal(&self) -> &'a [MudkhalNass] {
        self.jadwal
    }

    /// One row by identity.
    #[must_use]
    pub fn saff(&self, id: NassId) -> Option<&'a MudkhalNass> {
        self.mawqi
            .get(&id)
            .and_then(|mawdi| self.jadwal.get(*mawdi))
    }

    /// The rows an entry identifies, and how.
    ///
    /// Key first, then source text, then normalized source text. A key that
    /// answers to more than one row identifies nothing and falls through to the
    /// text strategies, because two rows sharing a key means the key is not
    /// this project's identity and choosing between them would be a coin flip.
    fn iltamis(&self, warid: &MudkhalWarid, naqi: Option<&str>) -> IltimasNatija {
        let mut iltibas = None;
        for miftah in mafatih_warid(warid) {
            if let Some(qaima) = self.bilmiftah.get(&miftah) {
                match qaima.len() {
                    1 => {
                        return IltimasNatija::Wujid(TareeqaTatabuq::Miftah, qaima.clone());
                    },
                    0 => {},
                    adad => iltibas = Some((miftah, adad)),
                }
            }
        }

        for shakl in [Some(warid.masdar.as_str()), naqi] {
            let Some(shakl) = shakl else { continue };
            if let Some(qaima) = self.bilmasdar.get(shakl)
                && !qaima.is_empty()
            {
                return IltimasNatija::Wujid(TareeqaTatabuq::MasdarHarfi, qaima.clone());
            }
        }

        let muwahhad = wahhid(naqi.unwrap_or(&warid.masdar));
        if !muwahhad.is_empty()
            && let Some(qaima) = self.bilmuwahhad.get(&muwahhad)
            && !qaima.is_empty()
        {
            return IltimasNatija::Wujid(TareeqaTatabuq::MasdarMuwahhad, qaima.clone());
        }

        match iltibas {
            Some((miftah, adad)) => IltimasNatija::Multabis { miftah, adad },
            None => IltimasNatija::Mafqud,
        }
    }

    /// The best similarity candidates for a source text.
    ///
    /// Scans every row. Returns at most [`AQSA_MURASHSHAHAT`], best first, and
    /// only those at or above `hadd`. Ties are broken by identity so that
    /// running the same import twice produces the same proposal list — a
    /// proposal that reorders itself between runs is one a reviewer cannot
    /// approve in bulk.
    fn murashshahat(&self, masdar: &str, hadd: f64) -> Vec<MurashshahTakhmeen> {
        let hadaf: Vec<char> = wahhid(masdar).chars().collect();
        if hadaf.len() < ADNA_TUL_LITTAKHMEEN {
            return Vec::new();
        }
        let mut wujida: Vec<MurashshahTakhmeen> = self
            .jadwal
            .par_iter()
            .filter_map(|saff| {
                let muqabil: Vec<char> = wahhid(&saff.masdar).chars().collect();
                let darajat = tashabuh(&hadaf, &muqabil, hadd)?;
                Some(MurashshahTakhmeen {
                    id: saff.id,
                    masdar_jadwal: saff.masdar.clone(),
                    darajat,
                })
            })
            .collect();
        wujida.sort_by(|awwal, thani| {
            thani
                .darajat
                .partial_cmp(&awwal.darajat)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| awwal.id.cmp(&thani.id))
        });
        wujida.truncate(AQSA_MURASHSHAHAT);
        wujida
    }

    /// The best similarity score any row reached, whether or not it qualified.
    ///
    /// Reported on a failure so an unmatched entry can say *how close* it came,
    /// which is what tells a contributor whether to lower the threshold or fix
    /// the file.
    fn afdal_tashabuh(&self, masdar: &str) -> f64 {
        let hadaf: Vec<char> = wahhid(masdar).chars().collect();
        if hadaf.is_empty() {
            return 0.0;
        }
        self.jadwal
            .par_iter()
            .map(|saff| {
                let muqabil: Vec<char> = wahhid(&saff.masdar).chars().collect();
                tashabuh(&hadaf, &muqabil, 0.0).unwrap_or(0.0)
            })
            .fold(|| 0.0_f64, f64::max)
            .reduce(|| 0.0_f64, f64::max)
    }
}

/// What a lookup concluded.
#[derive(Debug)]
enum IltimasNatija {
    /// Rows were identified.
    Wujid(TareeqaTatabuq, Vec<NassId>),
    /// A key matched several rows and nothing else matched at all.
    Multabis {
        /// The key.
        miftah: String,
        /// How many rows answered to it.
        adad: usize,
    },
    /// Nothing matched.
    Mafqud,
}

/// Matches a parsed file against a table.
///
/// Every entry in `milaff` produces exactly one outcome, and every entry the
/// parser already declined is carried through unchanged. Nothing is written —
/// see [`tabbiq`].
#[must_use]
pub fn tabiq(
    milaff: &MilaffWarid,
    jadwal: &[MudkhalNass],
    khiyarat: &IstiradKhiyarat,
) -> NatijatIstirad {
    let fahras = FahrasJadwal::jadeed(jadwal);
    let mut natija = NatijatIstirad::jadeed(milaff.sigha, milaff.adad_kulli(), milaff.asturr);
    for tanbih in &milaff.tanbihat {
        natija.nabbih(tanbih.clone());
    }
    for marfud in &milaff.marfuda {
        natija.sajjil(HasilatMudkhal::Marfud(marfud.clone()));
    }

    // The clean form of every incoming source, computed once and in parallel.
    // The table stores clean text with markup lifted into spans, so an XLIFF
    // source carrying `<ph id="1"/>` has to be put through the same lift before
    // it can be compared against a row — otherwise every string with a
    // placeholder in it fails to match and the import looks like it found
    // nothing.
    let nuqiy: Vec<Option<String>> = milaff
        .madakhil
        .par_iter()
        .map(|warid| naqqi(&warid.masdar))
        .collect();

    for (mawdi, warid) in milaff.madakhil.iter().enumerate() {
        let naqi = nuqiy.get(mawdi).and_then(Option::as_deref);
        natija.sajjil(hasilat_wahid(&fahras, warid, naqi, khiyarat));
    }
    natija
}

/// What becomes of one entry.
///
/// Split out so that the bucket rules are in one function rather than inline in
/// a loop, and so that every path through them ends in a `HasilatMudkhal` the
/// caller then records exactly once.
fn hasilat_wahid(
    fahras: &FahrasJadwal<'_>,
    warid: &MudkhalWarid,
    naqi: Option<&str>,
    khiyarat: &IstiradKhiyarat,
) -> HasilatMudkhal {
    let (tareeqa, sufuf) = match fahras.iltamis(warid, naqi) {
        IltimasNatija::Wujid(tareeqa, sufuf) => (tareeqa, sufuf),
        IltimasNatija::Multabis { miftah, adad } => {
            return HasilatMudkhal::GhayrMutatabiq(MudkhalGhayrMutatabiq {
                warid: warid.clone(),
                sabab: SababAdamTatabuq::MiftahMultabis { miftah, adad },
            });
        },
        IltimasNatija::Mafqud => return bila_tatabuq(fahras, warid, khiyarat),
    };

    let mut jahiza = Vec::new();
    let mut mujjala = Vec::new();
    for id in &sufuf {
        let Some(saff) = fahras.saff(*id) else {
            continue;
        };
        match sabab_taajil(saff) {
            Some(sabab) => mujjala.push(SaffIstibdal {
                id: *id,
                qadeem: saff.hadaf.clone(),
                hala_qadeema: saff.muraja.hala(),
                sabab,
            }),
            None => jahiza.push(*id),
        }
    }

    // The file said this entry is not confirmed, or it is a plural set that
    // cannot be collapsed to one target without choosing a form. Either way it
    // is offered, never applied — including for rows that are empty.
    if let Some(wasm) = wasm_iqtirah(warid) {
        return HasilatMudkhal::Muqtarah(IqtirahIstirad::MuallamMinAlmasdar {
            warid: warid.clone(),
            hadafat: sufuf,
            wasm,
        });
    }

    if !jahiza.is_empty() {
        return HasilatMudkhal::Mutatabiq(TatabuqIstirad {
            warid: warid.clone(),
            tareeqa,
            jahiza,
            mujjala,
        });
    }

    if khiyarat.iqtirah_istibdal && !mujjala.is_empty() {
        return HasilatMudkhal::Muqtarah(IqtirahIstirad::Istibdal {
            warid: warid.clone(),
            tareeqa,
            sufuf: mujjala,
        });
    }
    HasilatMudkhal::Marfud(MudkhalMarfud {
        warid: warid.clone(),
        sabab: SababRafd::SufufMashghula {
            adad: mujjala.len(),
        },
    })
}

/// The outcome for an entry that matched no row exactly.
fn bila_tatabuq(
    fahras: &FahrasJadwal<'_>,
    warid: &MudkhalWarid,
    khiyarat: &IstiradKhiyarat,
) -> HasilatMudkhal {
    let sabab_asasi = || match warid.miftah.as_deref() {
        Some(miftah) => SababAdamTatabuq::MiftahMajhul {
            miftah: miftah.to_owned(),
        },
        None => SababAdamTatabuq::MasdarMajhul {
            masdar: warid.masdar.chars().take(64).collect(),
        },
    };

    if !khiyarat.samah_bittakhmeen {
        return HasilatMudkhal::GhayrMutatabiq(MudkhalGhayrMutatabiq {
            warid: warid.clone(),
            sabab: sabab_asasi(),
        });
    }

    let tul = wahhid(&warid.masdar).chars().count();
    if tul < ADNA_TUL_LITTAKHMEEN {
        return HasilatMudkhal::GhayrMutatabiq(MudkhalGhayrMutatabiq {
            warid: warid.clone(),
            sabab: SababAdamTatabuq::AqsarMinAlhadd { tul },
        });
    }

    let hadd = khiyarat.hadd_faal();
    let murashshahat = fahras.murashshahat(&warid.masdar, hadd);
    if murashshahat.is_empty() {
        return HasilatMudkhal::GhayrMutatabiq(MudkhalGhayrMutatabiq {
            warid: warid.clone(),
            sabab: SababAdamTatabuq::TashabuhDunAlhadd {
                afdal: fahras.afdal_tashabuh(&warid.masdar),
                hadd,
            },
        });
    }
    HasilatMudkhal::Muqtarah(IqtirahIstirad::TatabuqTaqreebi {
        warid: warid.clone(),
        murashshahat,
    })
}

/// Why a row that matched would not be written to, or [`None`] when it is free.
const fn sabab_taajil(saff: &MudkhalNass) -> Option<SababTaajil> {
    if saff.muraja.mujammad() {
        return Some(SababTaajil::Mujammad);
    }
    if saff.hadaf.is_some() {
        return Some(if saff.muraja.hala().qaraaha_insan() {
            SababTaajil::QaraahaInsan
        } else {
            SababTaajil::TarjamaMawjuda
        });
    }
    if saff.muraja.hala().qaraaha_insan() {
        return Some(SababTaajil::QaraahaInsan);
    }
    None
}

/// The reason this entry must be proposed rather than applied, if there is one.
fn wasm_iqtirah(warid: &MudkhalWarid) -> Option<String> {
    if let NawWarid::Jama { suwar, .. } = &warid.naw {
        return Some(format!(
            "a plural set of {} form(s); the project row holds one target",
            suwar.len()
        ));
    }
    if warid.muallam {
        return Some(
            warid
                .hala_khaam
                .clone()
                .unwrap_or_else(|| "marked unconfirmed by the source file".to_owned()),
        );
    }
    None
}

/// Every key spelling a table row answers to.
fn mafatih_saff(saff: &MudkhalNass) -> Vec<String> {
    let mut mafatih = Vec::with_capacity(5);
    mafatih.push(saff.id.to_string());
    mafatih.push(saff.id.uuid().as_simple().to_string());
    let mawqi = saff.siyaq.mawqi.as_str();
    if !mawqi.is_empty() {
        mafatih.push(mawqi.to_owned());
        if !saff.siyaq.hawiya.is_empty() {
            mafatih.push(format!("{}/{mawqi}", saff.siyaq.hawiya));
        }
        // `mawqi_kamil` joins asset, location and field with U+0001. The last
        // segment is what a hand-written key column usually holds.
        if let Some(akhir) = mawqi.rsplit(['\u{1}', '/']).next()
            && akhir != mawqi
            && !akhir.is_empty()
        {
            mafatih.push(akhir.to_owned());
        }
    }
    mafatih
}

/// Every key spelling an incoming entry offers.
///
/// The context-qualified form comes first, because a `msgctxt` exists precisely
/// to distinguish two entries that would otherwise be the same key, and trying
/// the bare form first would let the second of them match the first's row.
fn mafatih_warid(warid: &MudkhalWarid) -> Vec<String> {
    let mut mafatih = Vec::with_capacity(4);
    let Some(miftah) = warid.miftah.as_deref() else {
        return mafatih;
    };
    if let Some(siyaq) = warid.siyaq.as_deref() {
        mafatih.push(format!("{siyaq}\u{1}{miftah}"));
        mafatih.push(format!("{siyaq}/{miftah}"));
    }
    mafatih.push(miftah.to_owned());
    if let Some(akhir) = miftah.rsplit(['\u{1}', '/']).next()
        && akhir != miftah
        && !akhir.is_empty()
    {
        mafatih.push(akhir.to_owned());
    }
    mafatih
}

// ---------------------------------------------------------------------------
// Applying
// ---------------------------------------------------------------------------

/// What an application actually did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TaqreerTatbeeq {
    /// How many matched entries were used.
    pub madakhil: usize,
    /// How many project rows received a translation.
    pub sufuf_maktuba: usize,
    /// How many identities the result named that this table does not hold.
    ///
    /// Non-zero means the result was produced against a different table than
    /// the one being written, which is a caller mistake worth surfacing rather
    /// than a row to skip quietly.
    pub sufuf_mafquda: usize,
    /// How many translations were stored with their markup unlifted because the
    /// markup parser refused them.
    pub bila_nasq: usize,
}

impl TaqreerTatbeeq {
    /// The sentence the import screen shows afterwards.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} entry(ies) applied to {} row(s); {} identity(ies) were not in this table and {} \
             translation(s) kept their markup unlifted",
            self.madakhil, self.sufuf_maktuba, self.sufuf_mafquda, self.bila_nasq
        )
    }
}

/// Writes the matched entries of a result onto a table.
///
/// **Only [`NatijatIstirad::mutatabiqa`], and only their
/// [`TatabuqIstirad::jahiza`] rows.** Proposals are not applied here and there
/// is no argument that makes them be: a caller that wants an overwrite resolves
/// it into a matched entry first, through a review action that records who
/// decided.
///
/// Every row written gets its review record moved through
/// [`taarib_mustalahat::muraja::SijillMuraja`]'s own transitions, so the change
/// lands in the string's history with an author and a moment. The state is
/// whatever [`HalatWarid::ila_muraja`] produced, which cannot be approval.
///
/// The translation's markup is lifted into spans the same way extraction lifts
/// a source string's, through the one markup parser this workspace has. A
/// translation whose markup will not parse is stored verbatim with no spans and
/// counted in [`TaqreerTatbeeq::bila_nasq`], because dropping the text would
/// lose the contributor's work over a formatting tag.
pub fn tabbiq(
    natija: &NatijatIstirad,
    jadwal: &mut [MudkhalNass],
    khiyarat: &IstiradKhiyarat,
) -> TaqreerTatbeeq {
    let mawqi: BTreeMap<NassId, usize> = jadwal
        .iter()
        .enumerate()
        .map(|(mawdi, saff)| (saff.id, mawdi))
        .collect();
    let mut taqreer = TaqreerTatbeeq::default();
    let muzawwid = format!("istirad:{}", natija.sigha().ism());

    for tatabuq in natija.mutatabiqa() {
        let Some(hadaf) = tatabuq.warid.hadaf.as_deref() else {
            continue;
        };
        taqreer.madakhil = taqreer.madakhil.saturating_add(1);
        let (naqi, nasq) = match taarib_istikhraj::jadwal::irfa_nasq(hadaf) {
            Ok((naqi, nasq)) => (naqi, nasq),
            Err(khata) => {
                tracing::warn!(sabab = %khata, "imported translation kept its markup unlifted");
                taqreer.bila_nasq = taqreer.bila_nasq.saturating_add(1);
                (hadaf.to_owned(), Vec::new())
            },
        };

        for id in &tatabuq.jahiza {
            // The index and the row are looked up in two steps rather than
            // chained through a closure: a closure would have to capture the
            // table mutably, and the reference it returned would not outlive
            // the call.
            let Some(mawdi) = mawqi.get(id).copied() else {
                taqreer.sufuf_mafquda = taqreer.sufuf_mafquda.saturating_add(1);
                continue;
            };
            let Some(saff) = jadwal.get_mut(mawdi) else {
                taqreer.sufuf_mafquda = taqreer.sufuf_mafquda.saturating_add(1);
                continue;
            };
            saff.hadaf = Some(naqi.clone());
            saff.nasq_hadaf.clone_from(&nasq);
            saff.muzawwid = Some(muzawwid.clone());
            saff.akhir_tabdeel.clone_from(&khiyarat.waqt);
            sajjil_hala(saff, tatabuq.warid.hala, khiyarat);
            taqreer.sufuf_maktuba = taqreer.sufuf_maktuba.saturating_add(1);
        }
    }
    taqreer
}

/// Moves one row's review record to the state an import may claim.
///
/// The draft transition needs an author, so an import with nobody named lands
/// at the machine state instead — which records no author, and claims that no
/// human has read the text. That is true of an unattributed import and it is
/// the weaker of the two claims, which is the right way to be wrong.
fn sajjil_hala(saff: &mut MudkhalNass, hala: HalatWarid, khiyarat: &IstiradKhiyarat) {
    match (hala, khiyarat.musahim.clone()) {
        (HalatWarid::Musawwada, Some(musahim)) => {
            saff.muharrir = Some(musahim.clone());
            saff.tareeqa = None;
            saff.muraja.sajjil_musawwada(musahim, khiyarat.lahza);
        },
        (HalatWarid::LilMuraja, musahim) => {
            saff.muharrir.clone_from(&musahim);
            saff.tareeqa = None;
            saff.muraja.tlub_muraja(
                musahim,
                khiyarat.lahza,
                Some("imported from an interchange file".to_owned()),
            );
        },
        (HalatWarid::Aaliya | HalatWarid::Musawwada, _) => {
            saff.muharrir = None;
            saff.tareeqa = Some(TareeqaTarjama::AaliyaFaqat);
            saff.muraja.sajjil_aali(khiyarat.lahza);
        },
    }
}

// ---------------------------------------------------------------------------
// Text helpers
// ---------------------------------------------------------------------------

/// The clean form of a source string, when lifting its markup changed it.
///
/// [`None`] when the markup parser refused the string or when the lift was a
/// no-op, so the caller does a second lookup only when there is a second form
/// to look up.
fn naqqi(khaam: &str) -> Option<String> {
    taarib_istikhraj::jadwal::irfa_nasq(khaam)
        .ok()
        .map(|(naqi, _)| naqi)
        .filter(|naqi| naqi != khaam)
}

/// The comparison form of a string.
///
/// Whitespace runs collapse to one space, leading and trailing whitespace goes,
/// case folds down, and the invisible formatting characters that a spreadsheet
/// export scatters through Arabic text — the zero-width marks, the bidi
/// overrides, a stray byte-order mark mid-file — are removed.
///
/// **No Unicode normalization form is applied.** Doing it properly needs a
/// normalizer, this crate has none, and a half-normalization that folded some
/// sequences and not others would make matching depend on which of two
/// identical-looking strings was written first. What this does is stated
/// exactly, so a match by this route can be explained.
#[must_use]
pub fn wahhid(nass: &str) -> String {
    let mut natija = String::with_capacity(nass.len());
    let mut faragh = false;
    for harf in nass.chars() {
        if harf.is_whitespace() {
            faragh = true;
            continue;
        }
        if matches!(
            harf,
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'
        ) {
            continue;
        }
        if faragh && !natija.is_empty() {
            natija.push(' ');
        }
        faragh = false;
        for saghir in harf.to_lowercase() {
            natija.push(saghir);
        }
    }
    natija
}

/// The largest edit distance that can still reach `hadd` over `aqsa`
/// characters.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is the product of a u32 length and a factor clamped into [0, 1], so it \
              is non-negative and never exceeds that length; truncating it to usize can neither \
              wrap nor lose a sign"
)]
fn saqf_masafa(aqsa: u32, hadd: f64) -> usize {
    let masmuh = f64::from(aqsa) * (1.0 - hadd.clamp(0.0, 1.0));
    masmuh.floor() as usize
}

/// The similarity of two normalized strings, or [`None`] below `hadd`.
///
/// `1 − distance / max(len)` over Unicode scalar values, with the distance
/// being Levenshtein's: unit cost for an insertion, a deletion or a
/// substitution. Two strings of very different length are rejected on their
/// lengths alone before any table is built, which is what makes scanning a
/// forty-thousand-row project affordable.
fn tashabuh(awwal: &[char], thani: &[char], hadd: f64) -> Option<f64> {
    let aqsa = awwal.len().max(thani.len());
    if aqsa == 0 {
        return (hadd <= 1.0).then_some(1.0);
    }
    let aqsa_raqm = u32::try_from(aqsa).unwrap_or(u32::MAX);
    let saqf = saqf_masafa(aqsa_raqm, hadd);
    if aqsa.saturating_sub(awwal.len().min(thani.len())) > saqf {
        return None;
    }
    let masafa = masafat_tahrir(awwal, thani, saqf)?;
    let masafa_raqm = u32::try_from(masafa).unwrap_or(u32::MAX);
    let darajat = 1.0 - f64::from(masafa_raqm) / f64::from(aqsa_raqm);
    (darajat >= hadd).then_some(darajat)
}

/// Levenshtein distance, abandoned as soon as it cannot come in under `saqf`.
///
/// Two rows of the matrix held in one vector, walked left to right. Every
/// access goes through [`slice::get`] and [`slice::get_mut`]: the indices are
/// provably in range, and writing them as subscripts would be four places where
/// a future edit could put one out of range and turn a similarity search into a
/// crash on somebody's forty-thousand-string project.
fn masafat_tahrir(awwal: &[char], thani: &[char], saqf: usize) -> Option<usize> {
    let mut saf: Vec<usize> = (0..=thani.len()).collect();
    for (safa, harf_a) in awwal.iter().enumerate() {
        let mut qutri = saf.first().copied().unwrap_or(0);
        let hali = safa.saturating_add(1);
        if let Some(khana) = saf.first_mut() {
            *khana = hali;
        }
        let mut adna = hali;
        for (amud, harf_b) in thani.iter().enumerate() {
            let talia = amud.saturating_add(1);
            let ala = saf.get(talia).copied().unwrap_or(usize::MAX);
            let yasar = saf.get(amud).copied().unwrap_or(usize::MAX);
            let badal = qutri.saturating_add(usize::from(harf_a != harf_b));
            let qeema = badal
                .min(ala.saturating_add(1))
                .min(yasar.saturating_add(1));
            if let Some(khana) = saf.get_mut(talia) {
                *khana = qeema;
            }
            qutri = ala;
            adna = adna.min(qeema);
        }
        if adna > saqf {
            return None;
        }
    }
    saf.last().copied().filter(|masafa| *masafa <= saqf)
}

/// Byte offset to line number, for the two XML parsers.
///
/// `quick-xml` reports a byte position and not a line, and every entry in a
/// report names the line it came from. Built once per document and answered by
/// a binary search, because doing it by counting newlines per entry is
/// quadratic on exactly the large files where it matters.
#[derive(Debug)]
pub(crate) struct FahrasAstur {
    nihayat: Vec<usize>,
}

impl FahrasAstur {
    /// Indexes every newline in a document.
    pub(crate) fn jadeed(nass: &str) -> Self {
        Self {
            nihayat: nass.match_indices('\n').map(|(izaha, _)| izaha).collect(),
        }
    }

    /// The one-based line a byte offset falls on.
    pub(crate) fn satr(&self, bayt: usize) -> usize {
        self.nihayat
            .partition_point(|nihaya| *nihaya < bayt)
            .saturating_add(1)
    }

    /// How many lines the document has.
    pub(crate) const fn adad(&self) -> usize {
        self.nihayat.len().saturating_add(1)
    }
}
