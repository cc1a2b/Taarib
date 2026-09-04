//! تقرير التجاوز — what was measured against its real constraint, what the
//! measurement said, and — kept apart from both answers — what could not be
//! measured at all.
//!
//! Arabic set at the point size English was set at is frequently wider, and a
//! menu whose labels overrun their buttons is precisely what "an amateur
//! translation" looks like in a screenshot. The only way to prevent overflow is
//! to measure it, and the only honest measurement is the one the game will
//! reproduce: the contributor's own Arabic, through `taarib-saff`, at a size the
//! game actually draws at, inside the width extraction recorded for that string.
//!
//! ## Nothing in this module fails a compile
//!
//! [`crate::khata`] is explicit that warnings are not errors: a high overflow
//! count is data, it travels *inside* the package, and it exists so the reviewer
//! sees what the contributor saw. A compiler that refused to build a patch
//! because eleven labels were four pixels wide would be a compiler contributors
//! learn to route around. So there is no `Result` here, no severity that blocks,
//! and no relationship between this report and [`crate::khata::KhataTarqee`].
//!
//! ## Three lists, because two would lie
//!
//! The failure this module is shaped around is not overflow. It is a report that
//! carried one list — the overflowing strings — and was read as though an empty
//! list meant nothing overflows. It did not. It meant the extractor never
//! recorded an available width for those strings, or precomputation produced no
//! layout at that size, and *nothing was ever compared*. A contributor read a
//! clean report and shipped a clipped main menu.
//!
//! So a string is reported as passing only when **both** numbers were actually
//! measured: a real available width recorded by extraction, and a real layout at
//! the size in question. When either is missing the string goes into
//! [`TaqrirTajawuz::ghayr_qabil_lil_tahaqquq`] with the reason it could not be
//! checked, and it is never in [`TaqrirTajawuz::salima`] and never in
//! [`TaqrirTajawuz::tajawuzat`]. Silence about a string must never read as "this
//! string is fine", so the unverifiable count leads every summary sentence this
//! module produces, and [`TaqrirTajawuz::hal_nass`] answers per string with a
//! four-valued verdict in which "never submitted for measurement" is its own
//! outcome rather than an absence.
//!
//! ## Severity is not the ratio
//!
//! A two per cent overrun on a subtitle is invisible: the line wraps, the box
//! scrolls, and nobody in the world can tell. A two per cent overrun on a
//! fixed-width button clips a letter off the end of a word, permanently, on
//! every screen the button appears on. The same number, two entirely different
//! defects. Severity here is therefore a function of the ratio **and** the
//! [`TasnifNass`] the string was classified as, through a per-class sensitivity
//! multiplier applied before the shared threshold ladder:
//!
//! | class | sensitivity | why |
//! | --- | --- | --- |
//! | [`TasnifNass::Qaima`] | [`HASSASIYAT_QAIMA`] | a button is a fixed sprite; the label is clipped at its edge, so overrun deletes letters |
//! | [`TasnifNass::Ism`] | [`HASSASIYAT_ISM`] | names sit in fixed slots in inventories and party lists, where truncation is the *good* case |
//! | [`TasnifNass::Ikhtiyar`] | [`HASSASIYAT_IKHTIYAR`] | a clipped choice is one the player cannot read before committing to it |
//! | [`TasnifNass::Nizam`] | [`HASSASIYAT_NIZAM`] | banners and toasts are fixed panels, usually with a little slack |
//! | [`TasnifNass::Khata`] | [`HASSASIYAT_KHATA`] | the same shape as a system banner, and the one message that must survive intact |
//! | [`TasnifNass::Majhul`] | [`HASSASIYAT_MAJHUL`] | unclassified is treated as sensitive on purpose — see below |
//! | [`TasnifNass::Tafseer`] | [`HASSASIYAT_TAFSEER`] | tooltips size themselves in width but are capped |
//! | [`TasnifNass::Wasf`] | [`HASSASIYAT_WASF`] | body text wraps inside a scrollable box; the ladder applies unmodified |
//! | [`TasnifNass::Hiwar`] | [`HASSASIYAT_HIWAR`] | dialogue wraps and pages; a small overrun genuinely is not seen |
//! | [`TasnifNass::Nusub`] | [`HASSASIYAT_NUSUB`] | credits scroll and clip nothing |
//! | [`TasnifNass::Dakhili`] | [`HASSASIYAT_DAKHILI`] | never drawn to a player, so an overrun here is a classification signal, not a visual defect |
//!
//! The asymmetry at [`TasnifNass::Majhul`] is deliberate. Guessing *low* for a
//! string nobody classified hides a clipped button; guessing high costs a
//! contributor one glance at an entry that turns out to be a debug label.
//!
//! ## An overflow the game already had
//!
//! Some games overflow their own English. A contributor who rewrites perfectly
//! good Arabic to fit a box the original also overran has shortened their text
//! for nothing, and has usually made it worse. So every entry carries
//! [`MasuliyatTajawuz`], and the third state of that enum is the important one:
//! when the original was not laid out at this size, the answer is *unknown*, and
//! it is stated as unknown rather than defaulted to blaming the translation.
//!
//! ## Truncation is not fitting
//!
//! A layout whose overflow policy cut the text reports a width that fits, and
//! reading that as a pass would be the worst lie this module could tell: the
//! text fits because letters were removed. [`TakhtitNass::maqsus`] therefore
//! forces the entry into the overflowing list with severity floored at
//! [`ShiddatTajawuz::Shadid`], whatever the width says.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::nass::{MudkhalNass, NassId, TasnifNass};
use taarib_saff::natija::TakhtitNass;
use taarib_usus::Khutura;

// ---------------------------------------------------------------------------
// Thresholds
// ---------------------------------------------------------------------------

/// The width, in pixels, below which an overrun is not an overrun.
///
/// Half a pixel. Advances accumulate as floating point across a shaped run, so a
/// string that exactly fills its box routinely measures a few hundredths of a
/// pixel over it. Below half a pixel no rasterizer produces different coverage
/// at any integer position, so the box is full and nothing is clipped.
///
/// Applied as a strict `>` against `available + tolerance`, never as an equality
/// test between two measured widths.
pub const TASAMUH_BIKSIL: f32 = 0.5;

/// The effective ratio above which an overrun stops being nothing.
///
/// Two per cent. On an element with the neutral sensitivity of
/// [`HASSASIYAT_WASF`] this is the last glyph of the last word crossing the
/// boundary of a box that wraps — the eye does not catch it, and flagging it
/// would fill a contributor's report with entries they can do nothing useful
/// about. Elements whose sensitivity multiplier is above one reach this ladder
/// rung at a proportionally smaller real overrun, which is how a two per cent
/// button overrun lands two bands higher than a two per cent subtitle overrun.
pub const HADD_TAFIF: f64 = 0.02;

/// The effective ratio at which an overrun becomes visible to a player who is
/// not looking for it.
///
/// Eight per cent. At a typical interface size this is roughly one Arabic
/// letter's advance past the edge — the point at which a reader stops reading a
/// label and starts noticing a label.
pub const HADD_MALHUZ: f64 = 0.08;

/// The effective ratio at which the element stops working.
///
/// Twenty per cent. A fifth of the available width is a whole short word, and no
/// interface element absorbs that: text either escapes its box and collides with
/// its neighbours, or is cut. Entries at or above this band are the ones a
/// contributor is expected to actually rewrite before submitting.
pub const HADD_SHADID: f64 = 0.20;

/// The effective ratio at which the string is unreadable as drawn.
///
/// Forty-five per cent. Close to half the content of the widest line is outside
/// the space it has. Whatever the engine does — clip, overdraw, or squash — the
/// player does not get the sentence.
pub const HADD_QATI: f64 = 0.45;

/// Sensitivity of a menu label, a button, a tab.
///
/// Four. A button is a fixed sprite and the label is clipped at its edge, so
/// every pixel of overrun deletes part of a word rather than moving anything.
/// This is the class the whole per-class design exists for.
pub const HASSASIYAT_QAIMA: f64 = 4.0;

/// Sensitivity of a name.
///
/// Three and a half. Item, skill and character names live in fixed slots inside
/// inventories, party frames and shop rows, where the engine's best behaviour is
/// an ellipsis and its usual behaviour is a hard cut.
pub const HASSASIYAT_ISM: f64 = 3.5;

/// Sensitivity of a choice the player picks between.
///
/// Three. A choice row is clickable and usually fixed, and a choice whose text is
/// clipped is one the player commits to without having read it.
pub const HASSASIYAT_IKHTIYAR: f64 = 3.0;

/// Sensitivity of a system message.
///
/// Two. Save, load and connection banners sit in fixed panels, but those panels
/// are normally authored with slack because the English wording changed twice
/// during development.
pub const HASSASIYAT_NIZAM: f64 = 2.0;

/// Sensitivity of player-visible error text.
///
/// Two. Structurally a system banner, and the one message whose whole value is
/// that the player can read all of it.
pub const HASSASIYAT_KHATA: f64 = 2.0;

/// Sensitivity of a string the extractor could not classify.
///
/// Two. Deliberately not neutral. An unclassified string that is really a button
/// and is scored as body text disappears from the report; an unclassified string
/// that is really a debug label and is scored as a banner costs one glance.
pub const HASSASIYAT_MAJHUL: f64 = 2.0;

/// Sensitivity of hover text and help.
///
/// One and a half. Tooltips generally size themselves in width, but every engine
/// caps that width, and the cap is what the recorded constraint holds.
pub const HASSASIYAT_TAFSEER: f64 = 1.5;

/// Sensitivity of descriptive body text.
///
/// One — the ladder unmodified. Descriptions wrap inside a box that is usually
/// scrollable, which is the neutral case the thresholds were chosen against.
pub const HASSASIYAT_WASF: f64 = 1.0;

/// Sensitivity of dialogue.
///
/// Three quarters. Dialogue wraps, pages, and is frequently drawn into a box
/// with deliberate margin, so a small overrun is genuinely invisible rather than
/// merely tolerable.
pub const HASSASIYAT_HIWAR: f64 = 0.75;

/// Sensitivity of credits, licences and legal text.
///
/// One half. This text scrolls and clips nothing, and it is the last place a
/// contributor's attention should be spent.
pub const HASSASIYAT_NUSUB: f64 = 0.5;

/// Sensitivity of a string that is not shown to a player.
///
/// One quarter, and not zero. Zero would erase these entries, and an internal
/// string that overflows a real recorded width is evidence that the
/// classification was wrong — which is worth one low-severity line, and is the
/// only reason the class is scored at all.
pub const HASSASIYAT_DAKHILI: f64 = 0.25;

/// How much the severity of an overrun is multiplied when the engine refuses to
/// wrap the string.
///
/// One and a half. [`taarib_mustalahat::nass::QuyudNass::satr_wahid`] means the
/// overrun has nowhere to go: it cannot become a second line, so it is clipped
/// or it collides with whatever is drawn beside it.
pub const DAAF_SATR_WAHID: f64 = 1.5;

/// How many characters of a string an entry carries for the report.
///
/// Forty-eight. Enough for a reviewer to recognise the line, short enough that a
/// report over a project of tens of thousands of strings stays a report rather
/// than a second copy of the string table.
pub const TUL_MUQTATAS: usize = 48;

// ---------------------------------------------------------------------------
// Severity
// ---------------------------------------------------------------------------

/// How badly a measured string overruns the room it has.
///
/// Ordered ascending, so [`Ord`] on the enum puts [`ShiddatTajawuz::Qati`] last
/// and the report — which sorts descending — puts it first. The discriminant
/// order is the severity order and nothing else depends on it, so a band added
/// between two existing ones is a one-line change here.
///
/// **No variant fails a compile.** See this module's header: an overflow is
/// data. The mapping to [`Khutura`] exists so the workspace can colour the row,
/// not so anything can refuse to build.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ShiddatTajawuz {
    /// Measured, and inside the width it was given.
    #[default]
    Bila,
    /// Measured over, and at this element's class it will not be seen.
    ///
    /// A real distinction from [`ShiddatTajawuz::Bila`]: "over, and invisible"
    /// is not "not over". A contributor who later reclassifies the string, or a
    /// game update that narrows the widget, turns every one of these into
    /// something else.
    Tafif,
    /// Visible to a player who is not looking for it.
    Malhuz,
    /// The element stops doing its job: text escapes its box or is cut.
    Shadid,
    /// The string, as drawn, cannot be read.
    Qati,
}

impl ShiddatTajawuz {
    /// Every band, weakest first.
    ///
    /// Used to seed the summary so that a band with no entries appears in the
    /// serialized report as an explicit zero. A missing key would read as "not
    /// measured", which is the one thing this module refuses to let a reader
    /// confuse with "measured, none found".
    pub const KULL: [Self; 5] = [Self::Bila, Self::Tafif, Self::Malhuz, Self::Shadid, Self::Qati];

    /// Scores an entry.
    ///
    /// Takes the whole entry rather than a ratio, so Phase 18 can recompute the
    /// band from the serialized fields and get the same answer this compile got
    /// — a severity that only existed at compile time would be a number a
    /// reviewer could not check.
    #[must_use]
    pub fn li_madkhal(madkhal: &MadkhalTajawuz) -> Self {
        let asas = if madkhal.maqsus {
            // The width fits because letters were deleted. Whatever the ratio
            // says, the player is missing part of the sentence.
            Self::Shadid
        } else {
            Self::Bila
        };
        let nisba = madkhal.nisba_muassara();
        let sullam = if nisba.is_nan() || nisba <= 0.0 {
            Self::Bila
        } else if nisba > HADD_QATI {
            Self::Qati
        } else if nisba > HADD_SHADID {
            Self::Shadid
        } else if nisba > HADD_MALHUZ {
            Self::Malhuz
        } else if nisba > HADD_TAFIF {
            Self::Tafif
        } else {
            Self::Bila
        };
        let mut shidda = asas.max(sullam);
        if madkhal.mutajawiz() {
            // An entry that reached the overflowing list crossed its boundary by
            // measurement. The floor says "over, and not visible at this class",
            // which is a different claim from "not over".
            shidda = shidda.max(Self::Tafif);
        }
        if madkhal.tajawuz_sutur() {
            // More lines than the box holds is a clipped paragraph even when
            // every individual line fits the width.
            shidda = shidda.max(Self::Malhuz);
        }
        shidda
    }

    /// How this band is coloured in the workspace and the review console.
    ///
    /// [`Khutura::Fadih`] is deliberately never returned. That level means the
    /// operation failed and left something needing attention, and an overflow
    /// leaves nothing: the package is written, installs, and runs.
    #[must_use]
    pub const fn khutura(self) -> Khutura {
        match self {
            Self::Bila => Khutura::Maluma,
            Self::Tafif | Self::Malhuz => Khutura::Tanbeeh,
            Self::Shadid | Self::Qati => Khutura::Khatar,
        }
    }

    /// Whether a contributor is expected to rewrite the string before
    /// submitting.
    #[must_use]
    pub const fn yastahiqq_iaada(self) -> bool {
        matches!(self, Self::Shadid | Self::Qati)
    }

    /// The band's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Bila => "ضمن المساحة",
            Self::Tafif => "تجاوز طفيف",
            Self::Malhuz => "تجاوز ملحوظ",
            Self::Shadid => "تجاوز شديد",
            Self::Qati => "تجاوز قاطع",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Bila => "within the space",
            Self::Tafif => "slight overrun",
            Self::Malhuz => "noticeable overrun",
            Self::Shadid => "severe overrun",
            Self::Qati => "unreadable as drawn",
        }
    }
}

/// The sensitivity multiplier a class of interface element carries.
///
/// Separated from [`ShiddatTajawuz`] so that the numbers are one table a
/// reviewer can read against the class list, rather than eleven branches buried
/// in a scoring function.
#[must_use]
pub const fn hassasiyat_tasnif(tasnif: TasnifNass) -> f64 {
    match tasnif {
        TasnifNass::Qaima => HASSASIYAT_QAIMA,
        TasnifNass::Ism => HASSASIYAT_ISM,
        TasnifNass::Ikhtiyar => HASSASIYAT_IKHTIYAR,
        TasnifNass::Nizam => HASSASIYAT_NIZAM,
        TasnifNass::Khata => HASSASIYAT_KHATA,
        TasnifNass::Majhul => HASSASIYAT_MAJHUL,
        TasnifNass::Tafseer => HASSASIYAT_TAFSEER,
        TasnifNass::Wasf => HASSASIYAT_WASF,
        TasnifNass::Hiwar => HASSASIYAT_HIWAR,
        TasnifNass::Nusub => HASSASIYAT_NUSUB,
        TasnifNass::Dakhili => HASSASIYAT_DAKHILI,
    }
}

// ---------------------------------------------------------------------------
// Whose overflow it is
// ---------------------------------------------------------------------------

/// Whether an overflow is the translation's or the game's.
///
/// The distinction changes what a contributor should do, which is the only
/// reason it is computed. A translation that overruns a box the original English
/// already overran is not a defect a translator can fix by shortening their
/// Arabic — the widget is too small for its own content, and shortening good
/// Arabic to fit it makes the patch worse, not better.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MasuliyatTajawuz {
    /// The original fitted at this size and the Arabic does not.
    Attarjama,
    /// The original already overran the same width at the same size.
    Alluba,
    /// The original was never laid out at this size, so nobody knows.
    ///
    /// Stated rather than defaulted. Blaming the translation because the source
    /// was not measured would put work on a contributor on the strength of a
    /// measurement nobody made.
    Majhula,
}

impl MasuliyatTajawuz {
    /// Whether this is work the contributor can usefully do.
    #[must_use]
    pub const fn alaa_almusahim(self) -> bool {
        matches!(self, Self::Attarjama)
    }

    /// The sentence beside the entry, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Attarjama => "النص الأصلي كان يسع المساحة، والترجمة لا تسعها.",
            Self::Alluba => "النص الأصلي يتجاوز المساحة نفسها؛ الضيق في اللعبة لا في الترجمة.",
            Self::Majhula => "لم يُخطَّط النص الأصلي عند هذا الحجم، فلا حكم على مصدر التجاوز.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Attarjama => "The original fitted this width and the translation does not.",
            Self::Alluba => {
                "The original overruns the same width; the widget is too small for its own content."
            }
            Self::Majhula => {
                "The original was not laid out at this size, so the source of the overrun is \
                 unknown."
            }
        }
    }
}

// ---------------------------------------------------------------------------
// One measured (string, size) pair
// ---------------------------------------------------------------------------

/// One string, measured at one size, against the width extraction recorded for
/// it.
///
/// Every field is either a measurement or derived from two measurements. There
/// is no field on this type whose value was assumed, and the two that could not
/// always be known — [`MadkhalTajawuz::sutur_masmuha`] and
/// [`MadkhalTajawuz::zaid_asl_miawi`] — are options rather than defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MadkhalTajawuz {
    /// The string's identity, which is what a preview jumps by.
    pub nass: NassId,
    /// The container it was extracted from, relative to the game's root.
    pub hawiya: String,
    /// Where inside that container, precise enough to open.
    pub mawqi: String,
    /// The first [`TUL_MUQTATAS`] characters of the Arabic, so the row is
    /// recognisable without a second lookup into the string table.
    pub muqtatas: String,
    /// What kind of interface element this is, which is half of the severity.
    pub tasnif: TasnifNass,
    /// The size the measurement was requested at.
    pub hajm: f32,
    /// The size the layout actually used.
    ///
    /// Differs from [`MadkhalTajawuz::hajm`] when the layout's overflow policy
    /// shrank the text to fit. A string that "fits" only because it was set two
    /// points smaller than everything around it is a visual defect of a
    /// different kind, and the pair of numbers is what makes it visible.
    pub hajm_fili: f32,
    /// The measured width of the widest line, in the game's pixels.
    pub ard_maqis: f32,
    /// The width extraction recorded as available.
    pub ard_mutah: f32,
    /// How far past the available width the widest line went, in pixels.
    pub zaid_biksil: f32,
    /// The same overrun as a percentage of the available width.
    pub zaid_miawi: f64,
    /// How many lines the layout produced.
    pub sutur_maqisa: u32,
    /// How many lines the recorded constraint allows.
    ///
    /// [`None`] when the constraint recorded no height and the engine did not
    /// declare the string single-line. The line count is then reported and not
    /// judged, because a limit nobody measured is not a limit.
    pub sutur_masmuha: Option<u32>,
    /// Whether the engine refuses to wrap this string.
    pub satr_wahid: bool,
    /// The total height the lines needed.
    pub irtifa_maqis: f32,
    /// The height available, when the constraint recorded one.
    pub irtifa_mutah: Option<f32>,
    /// Whether the layout's overflow policy cut the text.
    pub maqsus: bool,
    /// How many individual lines exceeded the width, as the layout engine
    /// counted them.
    pub sutur_mutajawiza: u32,
    /// The first line that exceeded the width, when any did.
    pub awwal_satr_mutajawiz: Option<u32>,
    /// Whether this overrun is the translation's or the game's.
    pub masuliya: MasuliyatTajawuz,
    /// How far the original overran the same width, as a percentage.
    ///
    /// [`None`] when the original was not laid out at this size — the same
    /// absence [`MasuliyatTajawuz::Majhula`] records, kept as a number so a
    /// reviewer can see that the game overruns by forty per cent and the
    /// translation by forty-one.
    pub zaid_asl_miawi: Option<f64>,
    /// The band this entry sorts into.
    pub shidda: ShiddatTajawuz,
}

impl MadkhalTajawuz {
    /// The overrun as a fraction of the available width.
    ///
    /// Zero when the available width is not a usable positive number, which the
    /// builder refuses to construct an entry from in the first place — the guard
    /// is here so the ratio is total rather than because it is reachable.
    #[must_use]
    pub fn nisba(&self) -> f64 {
        if !self.ard_mutah.is_finite() || self.ard_mutah <= 0.0 {
            return 0.0;
        }
        let zaid = f64::from(self.zaid_biksil.max(0.0));
        zaid / f64::from(self.ard_mutah)
    }

    /// The ratio after the element class and the wrapping rule are applied.
    ///
    /// What the ladder in [`ShiddatTajawuz::li_madkhal`] is compared against, and
    /// what the report's second sort key is.
    #[must_use]
    pub fn nisba_muassara(&self) -> f64 {
        let daaf = if self.satr_wahid { DAAF_SATR_WAHID } else { 1.0 };
        self.nisba() * hassasiyat_tasnif(self.tasnif) * daaf
    }

    /// Whether this measurement is an overflow.
    ///
    /// A strict comparison against the available width plus
    /// [`TASAMUH_BIKSIL`] — never an equality test between two measured widths —
    /// or a layout the overflow policy cut, which did not fit by definition.
    #[must_use]
    pub fn mutajawiz(&self) -> bool {
        self.maqsus || self.ard_maqis > self.ard_mutah + TASAMUH_BIKSIL
    }

    /// Whether the layout produced more lines than the constraint allows.
    ///
    /// Answers `false` when no line limit was measured, because an unmeasured
    /// limit cannot be exceeded.
    #[must_use]
    pub fn tajawuz_sutur(&self) -> bool {
        self.sutur_masmuha.is_some_and(|masmuh| self.sutur_maqisa > masmuh)
    }

    /// Whether the text needed more height than the constraint recorded.
    #[must_use]
    pub fn tajawuz_irtifa(&self) -> bool {
        self.irtifa_mutah.is_some_and(|mutah| self.irtifa_maqis > mutah)
    }

    /// Whether the layout engine set this string smaller than it was asked to.
    #[must_use]
    pub fn sughghir(&self) -> bool {
        self.hajm_fili + TASAMUH_BIKSIL < self.hajm
    }

    /// The row the workspace shows, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let sutur = match self.sutur_masmuha {
            Some(masmuh) => format!("{} سطرًا من {masmuh}", self.sutur_maqisa),
            None => format!("{} سطرًا، دون حدٍّ مقيس", self.sutur_maqisa),
        };
        format!(
            "{}: {:.0} بكسل مقابل {:.0} عند حجم {:.0} ({:+.1}٪)، {sutur}. {}",
            self.shidda.wasf_arabi(),
            self.ard_maqis,
            self.ard_mutah,
            self.hajm,
            self.zaid_miawi,
            self.masuliya.wasf_arabi()
        )
    }

    /// The same row in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let sutur = match self.sutur_masmuha {
            Some(masmuh) => format!("{} line(s) of {masmuh}", self.sutur_maqisa),
            None => format!("{} line(s), against no measured limit", self.sutur_maqisa),
        };
        format!(
            "{}: {:.0}px against {:.0}px at size {:.0} ({:+.1}%), {sutur}. {}",
            self.shidda.wasf_injilizi(),
            self.ard_maqis,
            self.ard_mutah,
            self.hajm,
            self.zaid_miawi,
            self.masuliya.wasf_injilizi()
        )
    }

    /// The total order the report is sorted by.
    ///
    /// Severity first and descending, then the class-adjusted ratio, then the
    /// raw pixel overrun — all three through [`f64::total_cmp`] and
    /// [`f32::total_cmp`], so a `NaN` that survived a degenerate font sorts to
    /// one end instead of silently making the comparison non-transitive and
    /// leaving the sort's output unspecified.
    ///
    /// The last two keys are the class, the identity and the size, which
    /// together are exactly the report's own unit: one entry per (string, size)
    /// pair. Two entries that compare equal under this order are therefore the
    /// same measurement of the same string, which is what makes deriving
    /// [`Eq`] from it honest.
    fn tarteeb(&self, akhar: &Self) -> Ordering {
        akhar
            .shidda
            .cmp(&self.shidda)
            .then_with(|| akhar.nisba_muassara().total_cmp(&self.nisba_muassara()))
            .then_with(|| akhar.zaid_biksil.total_cmp(&self.zaid_biksil))
            .then_with(|| self.tasnif.cmp(&akhar.tasnif))
            .then_with(|| self.nass.cmp(&akhar.nass))
            .then_with(|| self.hajm.total_cmp(&akhar.hajm))
    }
}

impl PartialEq for MadkhalTajawuz {
    fn eq(&self, akhar: &Self) -> bool {
        self.tarteeb(akhar).is_eq()
    }
}

impl Eq for MadkhalTajawuz {}

impl PartialOrd for MadkhalTajawuz {
    fn partial_cmp(&self, akhar: &Self) -> Option<Ordering> {
        Some(self.cmp(akhar))
    }
}

impl Ord for MadkhalTajawuz {
    fn cmp(&self, akhar: &Self) -> Ordering {
        self.tarteeb(akhar)
    }
}

// ---------------------------------------------------------------------------
// What could not be checked
// ---------------------------------------------------------------------------

/// Why a string could not be checked for overflow.
///
/// Six causes, and they are kept apart rather than collapsed into one "not
/// measured" because they need three different people to act: the contributor
/// translates, the capture session records widths, the compiler recomputes
/// layouts. A single cause would send all three to the same wrong place.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababAdamAltahaqquq {
    /// There is no Arabic to measure.
    BilaTarjama,
    /// Extraction never recorded an available width for this string.
    ///
    /// The most common cause on a project built from static extraction alone:
    /// only runtime capture can see the rectangle a string was drawn into.
    BilaArdMutah,
    /// A width was recorded and it is not a width.
    ///
    /// Zero, negative, infinite or `NaN`. Comparing a measured width against
    /// this would produce an answer, and the answer would be meaningless.
    ArdGhayrMujdi {
        /// What the constraint held.
        ard_mutah: f32,
    },
    /// The size the measurement was requested at is not a size.
    HajmGhayrMujdi {
        /// What was asked for.
        hajm: f32,
    },
    /// Precomputation produced no layout for this string at this size.
    BilaTakhtit {
        /// The size that has no layout.
        hajm: f32,
    },
    /// A layout exists at this size and contains no glyphs, for text that is not
    /// empty.
    ///
    /// A font chain with no coverage for the script, or a shaping failure that
    /// returned successfully. Either way there is nothing whose width means
    /// anything.
    TakhtitFarigh {
        /// The size the empty layout came back at.
        hajm: f32,
    },
}

impl SababAdamAltahaqquq {
    /// Whether the contributor is the person who can resolve this.
    #[must_use]
    pub const fn alaa_almusahim(self) -> bool {
        matches!(self, Self::BilaTarjama)
    }

    /// The cause, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "لا ترجمة لقياسها.",
            Self::BilaArdMutah => "لم يسجّل الاستخراج عرضًا متاحًا لهذه العبارة.",
            Self::ArdGhayrMujdi { .. } => "العرض المسجَّل ليس عرضًا صالحًا للمقارنة.",
            Self::HajmGhayrMujdi { .. } => "الحجم المطلوب للقياس ليس حجمًا صالحًا.",
            Self::BilaTakhtit { .. } => "لا تخطيط محسوبًا لهذه العبارة عند هذا الحجم.",
            Self::TakhtitFarigh { .. } => "التخطيط عاد فارغًا لنصٍّ غير فارغ.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "There is no translation to measure.",
            Self::BilaArdMutah => "Extraction recorded no available width for this string.",
            Self::ArdGhayrMujdi { .. } => "The recorded width is not a width worth comparing to.",
            Self::HajmGhayrMujdi { .. } => "The requested measurement size is not a valid size.",
            Self::BilaTakhtit { .. } => "No layout was precomputed for this string at this size.",
            Self::TakhtitFarigh { .. } => "The layout came back empty for text that is not empty.",
        }
    }

    /// What resolves it, in Arabic.
    #[must_use]
    pub const fn ilaj_arabi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "ترجم العبارة ثم أعد البناء.",
            Self::BilaArdMutah | Self::ArdGhayrMujdi { .. } => {
                "شغّل جلسة التقاط تمرّ على الشاشة التي تظهر فيها العبارة لتُسجَّل مساحتها."
            }
            Self::HajmGhayrMujdi { .. } | Self::BilaTakhtit { .. } => {
                "أعد اكتشاف المقاسات ثم أعد حساب التخطيطات."
            }
            Self::TakhtitFarigh { .. } => "تحقّق من تغطية الخطّ للنص العربي في هذا المشروع.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn ilaj_injilizi(self) -> &'static str {
        match self {
            Self::BilaTarjama => "Translate the string and rebuild.",
            Self::BilaArdMutah | Self::ArdGhayrMujdi { .. } => {
                "Run a capture session that reaches the screen this string appears on, so its \
                 rectangle is recorded."
            }
            Self::HajmGhayrMujdi { .. } | Self::BilaTakhtit { .. } => {
                "Re-run size discovery and recompute the layouts."
            }
            Self::TakhtitFarigh { .. } => {
                "Check that the project's font chain covers the Arabic script."
            }
        }
    }

    /// A stable key for grouping causes in the summary.
    ///
    /// Returns the variant name rather than the whole value, so that
    /// [`ArdGhayrMujdi`](SababAdamAltahaqquq::ArdGhayrMujdi) entries carrying
    /// four different bad widths still count as one cause.
    #[must_use]
    pub const fn miftah(self) -> &'static str {
        match self {
            Self::BilaTarjama => "bila_tarjama",
            Self::BilaArdMutah => "bila_ard_mutah",
            Self::ArdGhayrMujdi { .. } => "ard_ghayr_mujdi",
            Self::HajmGhayrMujdi { .. } => "hajm_ghayr_mujdi",
            Self::BilaTakhtit { .. } => "bila_takhtit",
            Self::TakhtitFarigh { .. } => "takhtit_farigh",
        }
    }
}

/// A string that was submitted for measurement and could not be measured.
///
/// Carries the same identifying fields as [`MadkhalTajawuz`] and no numbers,
/// because there are none. An entry here is the report saying "I do not know",
/// which is the answer the whole three-list shape exists to make sayable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MadkhalGhayrMutahaqqaq {
    /// The string's identity.
    pub nass: NassId,
    /// The container it was extracted from.
    pub hawiya: String,
    /// Where inside that container.
    pub mawqi: String,
    /// The first [`TUL_MUQTATAS`] characters of the source text, since there may
    /// be no translation to excerpt.
    pub muqtatas: String,
    /// What kind of interface element it is.
    pub tasnif: TasnifNass,
    /// Why it could not be checked.
    pub sabab: SababAdamAltahaqquq,
}

// ---------------------------------------------------------------------------
// Per-string verdict
// ---------------------------------------------------------------------------

/// What the report knows about one string.
///
/// The four-valued answer that keeps silence from reading as a pass.
/// [`HalatTahaqquq::LamYuqas`] is a distinct outcome from
/// [`HalatTahaqquq::Salim`] and always will be: "I checked it and it fits" and
/// "it was never submitted to me" are the two sentences a single boolean would
/// have merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HalatTahaqquq {
    /// Measured at one or more sizes and over its width at at least one of them.
    Mutajawiz {
        /// At how many sizes.
        adad: u32,
        /// The worst band across those sizes.
        aswa: ShiddatTajawuz,
    },
    /// Submitted at one or more sizes and unmeasurable at at least one of them,
    /// and never over its width where it could be measured.
    GhayrMutahaqqaq {
        /// At how many sizes.
        adad: u32,
    },
    /// Measured at one or more sizes and inside its width at every one.
    Salim {
        /// At how many sizes.
        adad: u32,
    },
    /// Never submitted to this report at any size.
    LamYuqas,
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Per-class counts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct IhsaTasnif {
    /// (string, size) pairs actually measured for this class.
    pub maqis: u32,
    /// Of those, how many overran.
    pub mutajawiz: u32,
    /// Of those, how many fitted.
    pub salim: u32,
    /// Pairs of this class that could not be measured at all.
    pub ghayr_mutahaqqaq: u32,
    /// The worst overrun in this class, as a percentage.
    ///
    /// [`None`] when nothing in the class overran. A zero here would be a
    /// measurement of an empty set, which is not zero.
    pub aswa_nisba_miawiya: Option<f64>,
    /// How many of this class fell into each band.
    pub hasab_alshidda: BTreeMap<ShiddatTajawuz, u32>,
}

/// The report's counts, with the unverifiable total first.
///
/// Field order is not cosmetic: this struct serializes into the package's
/// metadata section and is rendered field by field by the review console, and
/// the number that must not be a footnote is the number of strings nobody
/// checked.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MulakhkhasTajawuz {
    /// (string, size) pairs submitted and not measurable.
    pub ghayr_mutahaqqaq: u32,
    /// Distinct strings with at least one unmeasurable size.
    pub nusus_ghayr_mutahaqqaqa: u32,
    /// Why they could not be measured, by cause key.
    ///
    /// Keyed by [`SababAdamAltahaqquq::miftah`] so that four entries carrying
    /// four different bad widths count as one cause.
    pub hasab_sabab: BTreeMap<String, u32>,
    /// (string, size) pairs actually measured.
    pub maqis: u32,
    /// Distinct strings with at least one real measurement.
    pub nusus_maqisa: u32,
    /// Measured pairs that overran.
    pub mutajawiz: u32,
    /// Measured pairs that fitted.
    pub salim: u32,
    /// How many overruns fell into each band, every band present.
    pub hasab_alshidda: BTreeMap<ShiddatTajawuz, u32>,
    /// Per interface element class.
    pub hasab_altasnif: BTreeMap<TasnifNass, IhsaTasnif>,
    /// Overruns the original English did not have.
    pub bi_masuliyat_altarjama: u32,
    /// Overruns the original English already had.
    pub bi_masuliyat_alluba: u32,
    /// Overruns where the original was never laid out at that size.
    pub majhulat_almasuliya: u32,
    /// Measured pairs the layout policy had to cut.
    pub maqsusa: u32,
    /// Measured pairs the layout policy had to shrink.
    pub musaghghara: u32,
    /// Measured pairs with more lines than their constraint allows.
    pub mutajawizat_alsutur: u32,
    /// The worst overrun in the whole report, as a percentage.
    ///
    /// [`None`] when nothing overran anywhere.
    pub aswa_nisba_miawiya: Option<f64>,
}

impl MulakhkhasTajawuz {
    /// How many entries landed in one band.
    #[must_use]
    pub fn adad_shidda(&self, shidda: ShiddatTajawuz) -> u32 {
        self.hasab_alshidda.get(&shidda).copied().unwrap_or(0)
    }

    /// The counts for one class, when the report saw that class at all.
    #[must_use]
    pub fn ihsa_tasnif(&self, tasnif: TasnifNass) -> Option<&IhsaTasnif> {
        self.hasab_altasnif.get(&tasnif)
    }

    /// How many entries a contributor is expected to rewrite.
    #[must_use]
    pub fn yastahiqq_iaada(&self) -> u32 {
        self.adad_shidda(ShiddatTajawuz::Shadid)
            .saturating_add(self.adad_shidda(ShiddatTajawuz::Qati))
    }

    /// The share of submitted pairs that were actually measured.
    ///
    /// [`None`] when nothing at all was submitted, because a ratio over an empty
    /// submission is not one — and returning `1.0` there is exactly the lie this
    /// module exists to prevent.
    #[must_use]
    pub fn nisbat_altahaqquq(&self) -> Option<f64> {
        let kull = self.maqis.saturating_add(self.ghayr_mutahaqqaq);
        if kull == 0 {
            return None;
        }
        Some(f64::from(self.maqis) / f64::from(kull))
    }
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// The overflow report a package carries.
///
/// Ships inside [`NawQism::Bayan`](taarib_ruqaa::aqsam::NawQism::Bayan) as part
/// of the metadata record, so Phase 18 reads back exactly what the contributor
/// was shown rather than re-deriving it from a different font on a different
/// machine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TaqrirTajawuz {
    /// Everything measured and over its width, worst first.
    pub tajawuzat: Vec<MadkhalTajawuz>,
    /// Everything measured and inside its width.
    ///
    /// Retained rather than counted, so the report can answer "was this string
    /// checked?" per string. [`TaqrirTajawuz::bila_salima`] drops them for the
    /// copy that travels in the package, where the per-string answer is worth
    /// less than the megabytes.
    pub salima: Vec<MadkhalTajawuz>,
    /// Everything submitted and not measurable, with the reason.
    ///
    /// **Never empty because nothing went wrong.** An empty list here means
    /// every string submitted had both a recorded width and a layout, which is a
    /// strong claim about a project and a rare one.
    pub ghayr_qabil_lil_tahaqquq: Vec<MadkhalGhayrMutahaqqaq>,
    /// The counts.
    pub mulakhkhas: MulakhkhasTajawuz,
    /// Whether the passing entries were dropped from this copy.
    ///
    /// Recorded so a reader of [`TaqrirTajawuz::salima`] can tell an empty list
    /// that means "nothing passed" from one that means "the detail was trimmed",
    /// which is the same distinction the unverifiable list draws elsewhere.
    pub salima_muqallama: bool,
}

impl TaqrirTajawuz {
    /// An empty report: nothing measured, nothing submitted.
    #[must_use]
    pub fn farigh() -> Self {
        Self::default()
    }

    /// The worst entry, when anything overran.
    #[must_use]
    pub fn aswa(&self) -> Option<&MadkhalTajawuz> {
        self.tajawuzat.first()
    }

    /// Every overrun of a given class, worst first.
    #[must_use]
    pub fn tajawuzat_tasnif(&self, tasnif: TasnifNass) -> Vec<&MadkhalTajawuz> {
        self.tajawuzat.iter().filter(|madkhal| madkhal.tasnif == tasnif).collect()
    }

    /// Every overrun in a band or worse, worst first.
    #[must_use]
    pub fn tajawuzat_min_shidda(&self, adna: ShiddatTajawuz) -> Vec<&MadkhalTajawuz> {
        self.tajawuzat.iter().filter(|madkhal| madkhal.shidda >= adna).collect()
    }

    /// Every measurement recorded for one string, at any size.
    #[must_use]
    pub fn qiyasat_nass(&self, nass: NassId) -> Vec<&MadkhalTajawuz> {
        self.tajawuzat
            .iter()
            .chain(self.salima.iter())
            .filter(|madkhal| madkhal.nass == nass)
            .collect()
    }

    /// What this report knows about one string.
    ///
    /// Precedence is over, then unverifiable, then passing, then never
    /// submitted — worst-known-first, so a string that fitted at one size and
    /// could not be measured at another never answers [`HalatTahaqquq::Salim`].
    ///
    /// Answers [`HalatTahaqquq::LamYuqas`] for a string not in this report at
    /// all. After [`TaqrirTajawuz::bila_salima`] a string that passed everywhere
    /// answers that too, which is why the trimmed copy records
    /// [`TaqrirTajawuz::salima_muqallama`].
    #[must_use]
    pub fn hal_nass(&self, nass: NassId) -> HalatTahaqquq {
        let mut mutajawiz = 0_u32;
        let mut aswa = ShiddatTajawuz::Bila;
        for madkhal in &self.tajawuzat {
            if madkhal.nass == nass {
                mutajawiz = mutajawiz.saturating_add(1);
                aswa = aswa.max(madkhal.shidda);
            }
        }
        if mutajawiz > 0 {
            return HalatTahaqquq::Mutajawiz { adad: mutajawiz, aswa };
        }

        let ghayr = adad_u32(
            self.ghayr_qabil_lil_tahaqquq.iter().filter(|madkhal| madkhal.nass == nass).count(),
        );
        if ghayr > 0 {
            return HalatTahaqquq::GhayrMutahaqqaq { adad: ghayr };
        }

        let salim = adad_u32(self.salima.iter().filter(|madkhal| madkhal.nass == nass).count());
        if salim > 0 {
            return HalatTahaqquq::Salim { adad: salim };
        }
        HalatTahaqquq::LamYuqas
    }

    /// The same report without the per-string detail of the passing entries.
    ///
    /// The form that goes into the package. A project of forty thousand strings
    /// measured at six sizes produces a quarter of a million passing rows, and
    /// carrying them would make the metadata section larger than the layouts it
    /// describes. The counts survive in [`TaqrirTajawuz::mulakhkhas`] and the
    /// loss is recorded in [`TaqrirTajawuz::salima_muqallama`], so nothing reads
    /// the trimmed list as "nothing passed".
    #[must_use]
    pub fn bila_salima(mut self) -> Self {
        self.salima.clear();
        self.salima.shrink_to_fit();
        self.salima_muqallama = true;
        self
    }

    /// The summary sentence, in Arabic, leading with what was not checked.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let mulakhkhas = &self.mulakhkhas;
        format!(
            "{} قياسًا لم يمكن التحقّق منه (على {} عبارة)، و{} من {} قياسًا متجاوز، منها {} \
             يستحقّ إعادة الصياغة، و{} تجاوزًا موجودًا في النص الأصلي أيضًا.",
            mulakhkhas.ghayr_mutahaqqaq,
            mulakhkhas.nusus_ghayr_mutahaqqaqa,
            mulakhkhas.mutajawiz,
            mulakhkhas.maqis,
            mulakhkhas.yastahiqq_iaada(),
            mulakhkhas.bi_masuliyat_alluba
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let mulakhkhas = &self.mulakhkhas;
        format!(
            "{} measurement(s) could not be verified at all (across {} string(s)); {} of {} \
             measured pair(s) overran, {} of those badly enough to rewrite, and {} were overruns \
             the original already had.",
            mulakhkhas.ghayr_mutahaqqaq,
            mulakhkhas.nusus_ghayr_mutahaqqaqa,
            mulakhkhas.mutajawiz,
            mulakhkhas.maqis,
            mulakhkhas.yastahiqq_iaada(),
            mulakhkhas.bi_masuliyat_alluba
        )
    }
}

// ---------------------------------------------------------------------------
// Submitting a measurement
// ---------------------------------------------------------------------------

/// One measurement handed to the report.
///
/// `takhtit` is an [`Option`] on purpose and it is the module's central
/// invariant in argument form: precomputation that produced no layout at this
/// size hands [`None`], and the string lands in the unverifiable list. There is
/// no way to submit a string with a missing layout and have it counted as
/// passing, because there is no field to lie in.
#[derive(Debug, Clone, Copy)]
pub struct MudkhalQiyas<'a> {
    /// The string, with its recorded constraint and its classification.
    pub madkhal: &'a MudkhalNass,
    /// The size the layout was requested at.
    pub hajm: f32,
    /// The layout of the Arabic, or [`None`] when there is none at this size.
    pub takhtit: Option<&'a TakhtitNass>,
    /// The layout of the original English at the same size and width, when one
    /// was computed.
    ///
    /// [`None`] is the honest common case: a compile that only lays out
    /// translations has nothing to compare against, and the entry then records
    /// [`MasuliyatTajawuz::Majhula`] rather than blaming the translation.
    pub takhtit_asl: Option<&'a TakhtitNass>,
}

/// Accumulates measurements and produces the report.
///
/// Every string that is measured must be submitted through
/// [`BaniTaqrirTajawuz::sajjil`], including the ones that cannot be measured.
/// A caller that filters unmeasurable strings out before calling is a caller
/// producing the single-list report this module was written to replace.
#[derive(Debug, Clone, Default)]
pub struct BaniTaqrirTajawuz {
    tajawuzat: Vec<MadkhalTajawuz>,
    salima: Vec<MadkhalTajawuz>,
    ghayr: Vec<MadkhalGhayrMutahaqqaq>,
    nusus_maqisa: BTreeSet<NassId>,
    nusus_ghayr: BTreeSet<NassId>,
    tasnifat: BTreeSet<TasnifNass>,
}

impl BaniTaqrirTajawuz {
    /// A builder with nothing in it.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// How many pairs have been submitted so far, measurable or not.
    #[must_use]
    pub const fn adad_almuqaddam(&self) -> usize {
        self.tajawuzat.len().saturating_add(self.salima.len()).saturating_add(self.ghayr.len())
    }

    /// Records one (string, size) measurement.
    ///
    /// Decides which of the three lists the pair belongs in, and does so by
    /// asking whether both numbers exist rather than by asking whether the
    /// comparison came out badly.
    pub fn sajjil(&mut self, qiyas: &MudkhalQiyas<'_>) {
        let madkhal = qiyas.madkhal;
        let _ = self.tasnifat.insert(madkhal.tasnif);

        if let Some(sabab) = sabab_adam_altahaqquq(qiyas) {
            self.ghayr.push(MadkhalGhayrMutahaqqaq {
                nass: madkhal.id,
                hawiya: madkhal.siyaq.hawiya.clone(),
                mawqi: madkhal.siyaq.mawqi.clone(),
                muqtatas: muqtatas(madkhal.hadaf.as_deref().unwrap_or(&madkhal.masdar)),
                tasnif: madkhal.tasnif,
                sabab,
            });
            let _ = self.nusus_ghayr.insert(madkhal.id);
            return;
        }

        // Every `else` arm above returned, so both numbers are present. The two
        // `let ... else` below are unreachable by construction and are written
        // as early returns rather than as an unwrap, because the guarantee lives
        // in a different function and a refactor could move it.
        let (Some(ard_mutah), Some(takhtit)) = (madkhal.quyud.aqsa_ard, qiyas.takhtit) else {
            return;
        };

        let sutur_maqisa = adad_u32(takhtit.sutur.len());
        let irtifa_satr = takhtit.sutur.iter().map(|satr| satr.irtifa).fold(0.0_f32, f32::max);
        let sutur_masmuha = if madkhal.quyud.satr_wahid {
            Some(1)
        } else {
            madkhal
                .quyud
                .aqsa_irtifa
                .and_then(|irtifa_mutah| sutur_tasa(irtifa_mutah, irtifa_satr))
        };

        let zaid_biksil = (takhtit.ard - ard_mutah).max(0.0);
        let zaid_miawi = (f64::from(zaid_biksil) / f64::from(ard_mutah)) * 100.0;
        // The layout engine's own count of how many lines went past the width it
        // was given, kept rather than recomputed: it saw every line and this
        // report only sees the widest.
        let (sutur_mutajawiza, awwal_satr_mutajawiz) = takhtit
            .tajawuz
            .map_or((0_u32, None), |taqreer| {
                (taqreer.adad_sutur, Some(taqreer.awwal_satr))
            });

        let zaid_asl_miawi = qiyas.takhtit_asl.map(|asl| {
            let zaid_asl = (asl.ard - ard_mutah).max(0.0);
            (f64::from(zaid_asl) / f64::from(ard_mutah)) * 100.0
        });
        let masuliya = masuliyat_tajawuz(qiyas.takhtit_asl, ard_mutah);

        let mut madkhal_tajawuz = MadkhalTajawuz {
            nass: madkhal.id,
            hawiya: madkhal.siyaq.hawiya.clone(),
            mawqi: madkhal.siyaq.mawqi.clone(),
            muqtatas: muqtatas(madkhal.hadaf.as_deref().unwrap_or(&madkhal.masdar)),
            tasnif: madkhal.tasnif,
            hajm: qiyas.hajm,
            hajm_fili: takhtit.hajm,
            ard_maqis: takhtit.ard,
            ard_mutah,
            zaid_biksil,
            zaid_miawi,
            sutur_maqisa,
            sutur_masmuha,
            satr_wahid: madkhal.quyud.satr_wahid,
            irtifa_maqis: takhtit.irtifa,
            irtifa_mutah: madkhal.quyud.aqsa_irtifa,
            maqsus: takhtit.maqsus,
            sutur_mutajawiza,
            awwal_satr_mutajawiz,
            masuliya,
            zaid_asl_miawi,
            shidda: ShiddatTajawuz::Bila,
        };
        madkhal_tajawuz.shidda = ShiddatTajawuz::li_madkhal(&madkhal_tajawuz);

        let _ = self.nusus_maqisa.insert(madkhal.id);
        if madkhal_tajawuz.mutajawiz() || madkhal_tajawuz.tajawuz_sutur() {
            self.tajawuzat.push(madkhal_tajawuz);
        } else {
            self.salima.push(madkhal_tajawuz);
        }
    }

    /// Records a batch of measurements — typically one string at every
    /// discovered size.
    ///
    /// A convenience over [`BaniTaqrirTajawuz::sajjil`] with exactly the same
    /// contract: a size whose layout is missing is still submitted, with
    /// [`MudkhalQiyas::takhtit`] set to [`None`], and lands in the unverifiable
    /// list rather than being skipped by the caller's loop.
    pub fn sajjil_kull(&mut self, qiyasat: &[MudkhalQiyas<'_>]) {
        for qiyas in qiyasat {
            self.sajjil(qiyas);
        }
    }

    /// Sorts, counts, and produces the report.
    #[must_use]
    pub fn ikhtim(mut self) -> TaqrirTajawuz {
        self.tajawuzat.sort_unstable();
        self.salima.sort_unstable();
        self.ghayr.sort_by(|awwal, thani| {
            awwal
                .tasnif
                .cmp(&thani.tasnif)
                .then_with(|| awwal.sabab.miftah().cmp(thani.sabab.miftah()))
                .then_with(|| awwal.hawiya.cmp(&thani.hawiya))
                .then_with(|| awwal.mawqi.cmp(&thani.mawqi))
                .then_with(|| awwal.nass.cmp(&thani.nass))
        });

        let mut mulakhkhas = MulakhkhasTajawuz {
            ghayr_mutahaqqaq: adad_u32(self.ghayr.len()),
            nusus_ghayr_mutahaqqaqa: adad_u32(self.nusus_ghayr.len()),
            maqis: adad_u32(self.tajawuzat.len().saturating_add(self.salima.len())),
            nusus_maqisa: adad_u32(self.nusus_maqisa.len()),
            mutajawiz: adad_u32(self.tajawuzat.len()),
            salim: adad_u32(self.salima.len()),
            ..MulakhkhasTajawuz::default()
        };

        for shidda in ShiddatTajawuz::KULL {
            let _ = mulakhkhas.hasab_alshidda.insert(shidda, 0);
        }
        for tasnif in &self.tasnifat {
            let mut ihsa = IhsaTasnif::default();
            for shidda in ShiddatTajawuz::KULL {
                let _ = ihsa.hasab_alshidda.insert(shidda, 0);
            }
            let _ = mulakhkhas.hasab_altasnif.insert(*tasnif, ihsa);
        }

        for madkhal in &self.ghayr {
            let miftah = madkhal.sabab.miftah().to_owned();
            let khana = mulakhkhas.hasab_sabab.entry(miftah).or_insert(0);
            *khana = khana.saturating_add(1);
            if let Some(ihsa) = mulakhkhas.hasab_altasnif.get_mut(&madkhal.tasnif) {
                ihsa.ghayr_mutahaqqaq = ihsa.ghayr_mutahaqqaq.saturating_add(1);
            }
        }

        for madkhal in self.tajawuzat.iter().chain(self.salima.iter()) {
            let mutajawiz = madkhal.mutajawiz() || madkhal.tajawuz_sutur();
            if let Some(khana) = mulakhkhas.hasab_alshidda.get_mut(&madkhal.shidda) {
                *khana = khana.saturating_add(1);
            }
            if madkhal.maqsus {
                mulakhkhas.maqsusa = mulakhkhas.maqsusa.saturating_add(1);
            }
            if madkhal.sughghir() {
                mulakhkhas.musaghghara = mulakhkhas.musaghghara.saturating_add(1);
            }
            if madkhal.tajawuz_sutur() {
                mulakhkhas.mutajawizat_alsutur =
                    mulakhkhas.mutajawizat_alsutur.saturating_add(1);
            }
            if mutajawiz {
                match madkhal.masuliya {
                    MasuliyatTajawuz::Attarjama => {
                        mulakhkhas.bi_masuliyat_altarjama =
                            mulakhkhas.bi_masuliyat_altarjama.saturating_add(1);
                    }
                    MasuliyatTajawuz::Alluba => {
                        mulakhkhas.bi_masuliyat_alluba =
                            mulakhkhas.bi_masuliyat_alluba.saturating_add(1);
                    }
                    MasuliyatTajawuz::Majhula => {
                        mulakhkhas.majhulat_almasuliya =
                            mulakhkhas.majhulat_almasuliya.saturating_add(1);
                    }
                }
                mulakhkhas.aswa_nisba_miawiya =
                    Some(aqsa(mulakhkhas.aswa_nisba_miawiya, madkhal.zaid_miawi));
            }

            if let Some(ihsa) = mulakhkhas.hasab_altasnif.get_mut(&madkhal.tasnif) {
                ihsa.maqis = ihsa.maqis.saturating_add(1);
                if let Some(khana) = ihsa.hasab_alshidda.get_mut(&madkhal.shidda) {
                    *khana = khana.saturating_add(1);
                }
                if mutajawiz {
                    ihsa.mutajawiz = ihsa.mutajawiz.saturating_add(1);
                    ihsa.aswa_nisba_miawiya =
                        Some(aqsa(ihsa.aswa_nisba_miawiya, madkhal.zaid_miawi));
                } else {
                    ihsa.salim = ihsa.salim.saturating_add(1);
                }
            }
        }

        TaqrirTajawuz {
            tajawuzat: self.tajawuzat,
            salima: self.salima,
            ghayr_qabil_lil_tahaqquq: self.ghayr,
            mulakhkhas,
            salima_muqallama: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Whether a submitted measurement is unmeasurable, and why.
///
/// The single place the three-list decision is made. Returns [`None`] only when
/// a real available width and a real non-empty layout both exist, which is the
/// definition of "checked" this module holds every other function to.
fn sabab_adam_altahaqquq(qiyas: &MudkhalQiyas<'_>) -> Option<SababAdamAltahaqquq> {
    let madkhal = qiyas.madkhal;
    let hadaf = madkhal.hadaf.as_deref().unwrap_or("");
    if hadaf.trim().is_empty() {
        return Some(SababAdamAltahaqquq::BilaTarjama);
    }
    if !qiyas.hajm.is_finite() || qiyas.hajm <= 0.0 {
        return Some(SababAdamAltahaqquq::HajmGhayrMujdi { hajm: qiyas.hajm });
    }
    let Some(ard_mutah) = madkhal.quyud.aqsa_ard else {
        return Some(SababAdamAltahaqquq::BilaArdMutah);
    };
    if !ard_mutah.is_finite() || ard_mutah <= 0.0 {
        return Some(SababAdamAltahaqquq::ArdGhayrMujdi { ard_mutah });
    }
    let Some(takhtit) = qiyas.takhtit else {
        return Some(SababAdamAltahaqquq::BilaTakhtit { hajm: qiyas.hajm });
    };
    // An empty layout has no width, and a width that is not a number is not a
    // measurement. Treating either as one would put an entry with a `NaN` ratio
    // at whichever end of the sort `total_cmp` happens to place it, and call
    // that a severity.
    if takhtit.khali() || !takhtit.ard.is_finite() {
        return Some(SababAdamAltahaqquq::TakhtitFarigh { hajm: qiyas.hajm });
    }
    None
}

/// Whose overflow it is, from whether the original was laid out at all.
///
/// The original's width is compared against the *same* recorded constraint the
/// translation was compared against, never against a width the source layout
/// carried with it — otherwise a source laid out inside a different box would
/// exonerate a translation it has nothing to do with.
fn masuliyat_tajawuz(takhtit_asl: Option<&TakhtitNass>, ard_mutah: f32) -> MasuliyatTajawuz {
    let Some(asl) = takhtit_asl else {
        return MasuliyatTajawuz::Majhula;
    };
    if !asl.ard.is_finite() {
        return MasuliyatTajawuz::Majhula;
    }
    if asl.maqsus || asl.ard > ard_mutah + TASAMUH_BIKSIL {
        return MasuliyatTajawuz::Alluba;
    }
    MasuliyatTajawuz::Attarjama
}

/// How many lines a measured height holds, given a measured line height.
///
/// Both arguments are measurements. When either is absent or not a usable
/// positive number the answer is [`None`] and the line count is reported without
/// a limit beside it, because a limit derived from an unmeasured height would be
/// a number this module invented.
fn sutur_tasa(irtifa_mutah: f32, irtifa_satr: f32) -> Option<u32> {
    if !irtifa_mutah.is_finite()
        || !irtifa_satr.is_finite()
        || irtifa_mutah <= 0.0
        || irtifa_satr <= 0.0
    {
        return None;
    }
    let adad = (f64::from(irtifa_mutah) / f64::from(irtifa_satr)).floor();
    let mahdud = adad.clamp(1.0, f64::from(u32::MAX));
    // `floor` made the value integral and `clamp` pinned it into 1..=u32::MAX,
    // so the conversion below is exact and cannot wrap or lose a sign.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "floored then clamped into 1.0..=u32::MAX, so the value is exactly representable"
    )]
    let natija = mahdud as u32;
    Some(natija)
}

/// The larger of a running maximum and a new value, with `NaN` never winning.
///
/// [`f64::max`] already returns the non-`NaN` operand, which is the behaviour
/// wanted here: a degenerate measurement must not become the report's headline
/// worst case.
const fn aqsa(jari: Option<f64>, qeema: f64) -> f64 {
    match jari {
        Some(sabiq) => sabiq.max(qeema),
        None => qeema,
    }
}

/// The first [`TUL_MUQTATAS`] characters of a string, with an ellipsis when it
/// was cut.
///
/// Counted in characters rather than bytes, so a cut never lands inside a UTF-8
/// sequence and never inside the middle of an Arabic word's bytes.
fn muqtatas(nass: &str) -> String {
    let mut mukhtasar: String = nass.chars().take(TUL_MUQTATAS).collect();
    if nass.chars().nth(TUL_MUQTATAS).is_some() {
        mukhtasar.push('…');
    }
    mukhtasar
}

/// A count as a `u32`, saturating rather than wrapping.
fn adad_u32(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}
