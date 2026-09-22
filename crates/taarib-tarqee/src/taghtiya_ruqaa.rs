//! تغطية الرقعة — how much of a game this compile actually covers, measured
//! four different ways, with the provenance of every number attached to it.
//!
//! [`Taghtiya`] already exists and already knows how to be a coverage
//! measurement: by string, by occurrence, by opening session, folded across
//! containers. **Nothing here redefines it.** This module's job is to fill one
//! honestly out of a real project's strings, to add the two things a compile
//! knows that the shared type deliberately does not — a weighting the project's
//! own classification and capture produced, and the enumerated reasons a
//! contributor is not ready to publish — and to refuse to state any figure
//! nobody measured.
//!
//! ## The failure this is shaped around
//!
//! A patch was published at "97% weighted coverage". The weighting was computed
//! entirely from class guesses, because no capture session had ever run on that
//! machine, so every string in the project was weighted as though its class were
//! the only thing known about it. The strings the game actually draws most were
//! in the untranslated three per cent. The number was arithmetically correct and
//! told the contributor nothing true.
//!
//! So a weighted figure here always travels with its provenance:
//! [`TaghtiyaMawzuna::bi_mulahaza`] against [`TaghtiyaMawzuna::bi_tasnif`], and
//! the same split by weight rather than by count, because ten heavily observed
//! strings and ten thousand guessed ones are not a fifty-fifty split of
//! anything. [`TaghtiyaMawzuna::mawthuqa`] answers the one question a caller
//! actually has — may I show this number without a caveat — and it answers
//! `false` for the patch above.
//!
//! ## A frequency is never invented
//!
//! Where runtime capture recorded how often a string was drawn, the weight uses
//! it. Where it did not, the weight is the class weight alone, and the string is
//! counted in [`TaghtiyaMawzuna::bi_tasnif`]. There is no default frequency, no
//! "assume once", and no interpolation from similar strings: a made-up
//! denominator is worse than a smaller honest one, because the smaller honest
//! one is visibly smaller.
//!
//! ## The empty denominator that reads as a hundred per cent
//!
//! [`Taghtiya::nisba_awwal`] returns `1.0` when `majmu_awwal` is zero, and that
//! is correct for the type: a container with no strings is fully covered, and
//! folding containers with [`Taghtiya::damm`] depends on it. It is *not*
//! correct as a verdict. A project on which no capture session has ever run has
//! `majmu_awwal == 0`, therefore a first-hour ratio of `1.0`, therefore clears
//! the opening-session half of [`Taghtiya::qabila_lil_nashr`] without a single
//! string having been observed.
//!
//! This module uses [`Taghtiya::qabila_lil_nashr`] — it does not reimplement the
//! floor — and then refuses to call a project publishable when the first-hour
//! figure rests on nothing, naming
//! [`SababAdamAlnashr::BilaJalsatAwwal`] as its own cause. "Measured a hundred
//! per cent" and "measured nothing" must not be the same answer.
//!
//! ## "Not ready" is not a reason
//!
//! Every cause in [`SababAdamAlnashr`] carries counts, so the interface renders
//! *312 strings untranslated, 89 awaiting review, 47 more needed to reach the
//! sixty per cent floor* rather than a verdict a contributor cannot act on.
//! [`TaqrirTaghtiya::asbab`] is computed whether or not the project is
//! publishable, because a publishable project with two hundred untranslated
//! strings still wants that sentence; [`TaqrirTaghtiya::asbab_hasima`] narrows
//! it to the ones that actually block.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::nass::{MudkhalNass, NassId, TasnifNass};
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_usus::Khutura;

// ---------------------------------------------------------------------------
// Class weights
// ---------------------------------------------------------------------------

/// Weight of a menu label, a button, a tab.
///
/// Five, the highest in the table. A main-menu button is on screen in every
/// session a player ever starts, usually before anything else is, and an
/// untranslated one is the first thing a screenshot shows.
pub const WAZN_QAIMA: f64 = 5.0;

/// Weight of a choice the player picks between.
///
/// Four. Every choice is read, deliberately, before the player commits — there
/// is no skimming a decision.
pub const WAZN_IKHTIYAR: f64 = 4.0;

/// Weight of an item, skill, character or place name.
///
/// Three. Names recur across inventories, party frames, shops and dialogue, so
/// one untranslated name is visible in a dozen places.
pub const WAZN_ISM: f64 = 3.0;

/// Weight of a system message.
///
/// Three. Save, load, autosave and connection messages appear in every session
/// regardless of how far the player gets.
pub const WAZN_NIZAM: f64 = 3.0;

/// Weight of hover text and help.
///
/// Two. Read on demand, but read at exactly the moment a player is confused,
/// which is when a language they cannot read costs the most.
pub const WAZN_TAFSEER: f64 = 2.0;

/// Weight of dialogue.
///
/// Two. The bulk of most projects by count, and read once each. Weighting it
/// with names and menus would let sheer volume drown out the interface — which
/// is precisely the aggregate this module exists to break apart.
pub const WAZN_HIWAR: f64 = 2.0;

/// Weight of player-visible error text.
///
/// One and a half. Rare, and the one message whose whole purpose is to be
/// understood when it appears.
pub const WAZN_KHATA: f64 = 1.5;

/// Weight of descriptive body text.
///
/// One — the neutral weight the rest of the table is scaled against.
pub const WAZN_WASF: f64 = 1.0;

/// Weight of a string the extractor could not classify.
///
/// One, the neutral weight. Deliberately not the highest and not the lowest:
/// scoring an unknown high would let a project full of unclassified asset paths
/// claim a weighted coverage it has not earned, and scoring it zero would delete
/// real interface strings from the denominator.
pub const WAZN_MAJHUL: f64 = 1.0;

/// Weight of credits, licences and legal text.
///
/// A quarter. Seen once, scrolled past, and never the reason a player installs
/// a translation.
pub const WAZN_NUSUB: f64 = 0.25;

/// Weight of a string that is not shown to a player.
///
/// Zero, and this is an **exclusion rather than a score**: a string with zero
/// weight is in neither the weighted numerator nor the weighted denominator, and
/// is counted in [`TaghtiyaMawzuna::mustabaad`]. Counting internal strings as
/// covered would let a project of asset paths report high weighted coverage;
/// counting them as uncovered would punish a contributor for not translating
/// node names. Neither is true, so neither is computed.
pub const WAZN_DAKHILI: f64 = 0.0;

/// What an observation is worth before its frequency is counted at all.
///
/// One. A string the capture session saw exactly once still starts from its full
/// class weight, so observing a string never makes it worth *less* than an
/// identical string nobody observed. A recorded frequency of zero — the session
/// ran and this string never appeared — therefore leaves the class weight
/// untouched rather than erasing the string, because "not drawn in this session"
/// is not "not drawn in this game".
pub const WAZN_MULAHAZA_ASAS: f64 = 1.0;

/// The share of weighted coverage that must come from observation before the
/// weighted figure is shown without a caveat.
///
/// A half, tested with a strict `>`. Below it, most of the number is the class
/// table talking to itself, and the class table is a set of eleven constants
/// chosen by a person who has never played this particular game. Above it, most
/// of the number came from a session that actually ran.
pub const HADD_WITHUQ_ALMULAHAZA: f64 = 0.50;

/// The by-string floor [`Taghtiya::qabila_lil_nashr`] enforces.
///
/// **A mirror, never the source of truth.** The verdict in this module is always
/// taken from [`Taghtiya::qabila_lil_nashr`]; this constant exists only so that
/// [`SababAdamAlnashr::TaghtiyaDunAlhadd`] can say *how far short* and *how many
/// more strings*, which a boolean cannot. If the two ever disagree the shared
/// type wins and this constant is the bug.
pub const HADD_TAGHTIYA_LILNASHR: f64 = 0.60;

/// The opening-session floor [`Taghtiya::qabila_lil_nashr`] enforces.
///
/// The same mirror, for the same reason, with the same precedence.
pub const HADD_AWWAL_LILNASHR: f64 = 0.85;

/// Every review state, in the order the shared enum declares them.
///
/// Kept here so [`TaqrirTaghtiya::hasab_hala`] can be seeded with an explicit
/// zero for every state. A missing key in a serialized histogram reads as "not
/// measured", and the whole point of this module is that it never says that by
/// accident.
pub const HALAT_MURAJA: [HalatMuraja; 6] = [
    HalatMuraja::LamTutarjam,
    HalatMuraja::TarjamaAaliya,
    HalatMuraja::Musawwada,
    HalatMuraja::LilMuraja,
    HalatMuraja::Muakkada,
    HalatMuraja::Marfuda,
];

/// The weight one class of interface element carries.
///
/// One table, readable against the class list in one glance, rather than eleven
/// branches inside the accumulation loop.
#[must_use]
pub const fn wazn_tasnif(tasnif: TasnifNass) -> f64 {
    match tasnif {
        TasnifNass::Qaima => WAZN_QAIMA,
        TasnifNass::Ikhtiyar => WAZN_IKHTIYAR,
        TasnifNass::Ism => WAZN_ISM,
        TasnifNass::Nizam => WAZN_NIZAM,
        TasnifNass::Tafseer => WAZN_TAFSEER,
        TasnifNass::Hiwar => WAZN_HIWAR,
        TasnifNass::Khata => WAZN_KHATA,
        TasnifNass::Wasf => WAZN_WASF,
        TasnifNass::Majhul => WAZN_MAJHUL,
        TasnifNass::Nusub => WAZN_NUSUB,
        TasnifNass::Dakhili => WAZN_DAKHILI,
    }
}

// ---------------------------------------------------------------------------
// Observed frequency
// ---------------------------------------------------------------------------

/// How often runtime capture actually saw each string drawn.
///
/// A map with a deliberately narrow interface. [`MulahazatTakrar::takrar`]
/// returns an [`Option`] and never a default, because the difference between
/// *observed zero times* and *never observed* is the difference between the two
/// halves of [`MasdarWazn`], and a map that answered `0` for both would erase it
/// in the one place it matters.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MulahazatTakrar {
    marrat: BTreeMap<NassId, u64>,
}

impl MulahazatTakrar {
    /// An empty record: no session has contributed anything.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Records that a string was drawn a number of times.
    ///
    /// Adds to whatever a previous session recorded, so several capture sessions
    /// over the same project accumulate rather than overwrite — which is what a
    /// contributor who plays the opening twice expects, and what makes the
    /// frequency worth weighting by at all.
    pub fn sajjil(&mut self, nass: NassId, marrat: u64) {
        let khana = self.marrat.entry(nass).or_insert(0);
        *khana = khana.saturating_add(marrat);
    }

    /// How often a string was drawn, or [`None`] when no session ever saw it.
    #[must_use]
    pub fn takrar(&self, nass: NassId) -> Option<u64> {
        self.marrat.get(&nass).copied()
    }

    /// How many distinct strings any session observed.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.marrat.len()
    }

    /// Whether no session has contributed anything at all.
    #[must_use]
    pub fn khaliya(&self) -> bool {
        self.marrat.is_empty()
    }

    /// Every observation, ascending by identity.
    pub fn mulahazat(&self) -> impl Iterator<Item = (NassId, u64)> + '_ {
        self.marrat.iter().map(|(nass, marrat)| (*nass, *marrat))
    }
}

/// The strings a capture session saw in the opening stretch of play.
///
/// Present-and-empty is a different state from absent, and both are kept: a
/// session that ran and named nothing is a broken session
/// ([`SababAdamAlnashr::JalsatAwwalFarigha`]), and no session at all is an
/// unmeasured project ([`SababAdamAlnashr::BilaJalsatAwwal`]). Collapsing the
/// two would send a contributor to fix the wrong thing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MajmuatAwwal {
    nusus: BTreeSet<NassId>,
}

impl MajmuatAwwal {
    /// A session record with nothing in it yet.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Records that the opening session drew this string.
    pub fn adif(&mut self, nass: NassId) {
        let _ = self.nusus.insert(nass);
    }

    /// Whether the opening session drew this string.
    #[must_use]
    pub fn tahtawi(&self, nass: NassId) -> bool {
        self.nusus.contains(&nass)
    }

    /// How many distinct strings the opening session drew.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.nusus.len()
    }

    /// Whether the session recorded nothing.
    #[must_use]
    pub fn khaliya(&self) -> bool {
        self.nusus.is_empty()
    }

    /// The opening session's strings, read from the rows' own provenance.
    ///
    /// A row runtime capture contributed to is a row a session drew, so the set
    /// is derived from the same table the coverage is counted over and cannot
    /// drift out of step with it. [`None`] when no row carries capture at all,
    /// because that is [`SababAdamAlnashr::BilaJalsatAwwal`] — the state an
    /// empty denominator cannot be told apart from on its own, and the reason
    /// every caller passed [`None`] here before this existed: the set had no
    /// producer, so no project could ever clear the opening floor.
    #[must_use]
    pub fn min_madakhil(nusus: &[MudkhalNass]) -> Option<Self> {
        let mut majmua = Self::jadeeda();
        for madkhal in nusus {
            if madkhal.masdar_istikhraj.multaqat() {
                majmua.adif(madkhal.id);
            }
        }
        (!majmua.khaliya()).then_some(majmua)
    }
}

/// Where one string's weight came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MasdarWazn {
    /// Runtime capture recorded how often this string is drawn, and the weight
    /// used it.
    Mulahaza,
    /// No session ever saw this string, so the weight is its class weight and
    /// nothing else. **No frequency was assumed.**
    Tasnif,
}

impl MasdarWazn {
    /// The label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Mulahaza => "بترجيح ملاحظ",
            Self::Tasnif => "بترجيح التصنيف وحده",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Mulahaza => "weighted by observation",
            Self::Tasnif => "weighted by class alone",
        }
    }
}

// ---------------------------------------------------------------------------
// Weighted coverage
// ---------------------------------------------------------------------------

/// Coverage weighted by what each string is worth, with its provenance.
///
/// Every ratio on this type is an [`Option`]. A weighted coverage over a
/// denominator of zero is not a hundred per cent and is not zero — it is a
/// question nobody asked, and the type says so instead of picking one.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct TaghtiyaMawzuna {
    /// Total weight of every string that carries any.
    pub wazn_kulli: f64,
    /// Weight covered by a translation of any kind.
    pub wazn_mutarjam: f64,
    /// Weight covered by a translation a human reviewed and approved.
    pub wazn_muakkad: f64,
    /// Strings whose weight used an observed frequency.
    pub bi_mulahaza: u32,
    /// Strings weighted by their class alone, with no frequency observed.
    pub bi_tasnif: u32,
    /// How much of the total weight came from observed strings.
    pub wazn_bi_mulahaza: f64,
    /// How much came from class weights alone.
    ///
    /// Reported beside the count because the two split differently: ten heavily
    /// observed strings can outweigh ten thousand guessed ones, and a caller
    /// reading only the counts would draw the opposite conclusion.
    pub wazn_bi_tasnif: f64,
    /// Strings excluded from the weighting entirely.
    ///
    /// The zero-weight classes — see [`WAZN_DAKHILI`]. Counted rather than
    /// dropped, so the difference between the weighted and by-string
    /// denominators is always explainable.
    pub mustabaad: u32,
}

impl TaghtiyaMawzuna {
    /// Weighted coverage, or [`None`] when nothing carried weight.
    #[must_use]
    pub fn nisba(&self) -> Option<f64> {
        self.qismat(self.wazn_mutarjam)
    }

    /// Weighted coverage counting only reviewed and approved strings.
    #[must_use]
    pub fn nisba_muakkada(&self) -> Option<f64> {
        self.qismat(self.wazn_muakkad)
    }

    /// The share of the total weight that came from observation.
    ///
    /// The number that decides whether the weighted figure means anything.
    #[must_use]
    pub fn hissat_almulahaza(&self) -> Option<f64> {
        let kull = self.wazn_bi_mulahaza + self.wazn_bi_tasnif;
        if kull.is_nan() || kull <= 0.0 {
            return None;
        }
        Some(self.wazn_bi_mulahaza / kull)
    }

    /// Whether the weighted figure may be shown without a caveat.
    ///
    /// Requires more than [`HADD_WITHUQ_ALMULAHAZA`] of the weight to have come
    /// from a session that actually ran. Answers `false` when no weight exists
    /// at all, because an unmeasured project is not a trustworthy one.
    #[must_use]
    pub fn mawthuqa(&self) -> bool {
        self.hissat_almulahaza()
            .is_some_and(|hissa| hissa > HADD_WITHUQ_ALMULAHAZA)
    }

    /// Folds another weighting into this one.
    ///
    /// Mirrors [`Taghtiya::damm`] so a project assembled from several containers
    /// folds both measurements the same way.
    pub fn damm(&mut self, akhar: &Self) {
        self.wazn_kulli += akhar.wazn_kulli;
        self.wazn_mutarjam += akhar.wazn_mutarjam;
        self.wazn_muakkad += akhar.wazn_muakkad;
        self.bi_mulahaza = self.bi_mulahaza.saturating_add(akhar.bi_mulahaza);
        self.bi_tasnif = self.bi_tasnif.saturating_add(akhar.bi_tasnif);
        self.wazn_bi_mulahaza += akhar.wazn_bi_mulahaza;
        self.wazn_bi_tasnif += akhar.wazn_bi_tasnif;
        self.mustabaad = self.mustabaad.saturating_add(akhar.mustabaad);
    }

    /// The sentence the workspace shows, in Arabic, with the caveat attached
    /// when it is owed.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let (Some(nisba), Some(hissa)) = (self.nisba(), self.hissat_almulahaza()) else {
            return "لا تغطية مرجَّحة: لم يحمل أي نصٍّ ظاهرٍ للاعب وزنًا في هذا المشروع.".to_owned();
        };
        format!(
            "{:.0}٪ تغطية مرجَّحة، {:.0}٪ منها من ملاحظة فعلية ({} عبارة ملاحَظة مقابل {} \
             بالتصنيف وحده).",
            nisba * 100.0,
            hissa * 100.0,
            self.bi_mulahaza,
            self.bi_tasnif
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let (Some(nisba), Some(hissa)) = (self.nisba(), self.hissat_almulahaza()) else {
            return "No weighted coverage: no player-visible string in this project carried any \
                    weight."
                .to_owned();
        };
        format!(
            "{:.0}% weighted coverage, {:.0}% of it from real observation ({} observed \
             string(s) against {} weighted by class alone).",
            nisba * 100.0,
            hissa * 100.0,
            self.bi_mulahaza,
            self.bi_tasnif
        )
    }

    /// A ratio against the total weight, refusing an empty denominator.
    fn qismat(&self, juz: f64) -> Option<f64> {
        if self.wazn_kulli.is_nan() || self.wazn_kulli <= 0.0 {
            return None;
        }
        Some(juz / self.wazn_kulli)
    }
}

// ---------------------------------------------------------------------------
// Per section
// ---------------------------------------------------------------------------

/// Coverage of one class of interface element.
///
/// The reason the report can say *dialogue is 100%, UI is 41%* instead of one
/// aggregate that is true of nothing. A project whose dialogue is finished and
/// whose menus are not looks identical to one whose menus are finished and whose
/// dialogue is not, from a single percentage — and they are not remotely the
/// same patch to install.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QismTaghtiya {
    /// Which class.
    pub tasnif: TasnifNass,
    /// The shared measurement, filled for this class alone.
    pub taghtiya: Taghtiya,
    /// The weighting for this class alone.
    pub mawzuna: TaghtiyaMawzuna,
}

impl QismTaghtiya {
    /// The section's line in the report, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        format!(
            "{}: {:.0}٪ ({} من {})",
            self.tasnif.wasf_arabi(),
            self.taghtiya.nisba() * 100.0,
            self.taghtiya.mutarjam,
            self.taghtiya.majmu
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        format!(
            "{}: {:.0}% ({} of {})",
            self.tasnif.wasf_injilizi(),
            self.taghtiya.nisba() * 100.0,
            self.taghtiya.mutarjam,
            self.taghtiya.majmu
        )
    }
}

// ---------------------------------------------------------------------------
// Why a project is not publishable
// ---------------------------------------------------------------------------

/// One reason, with counts, that a compile is or is not ready to publish.
///
/// Every variant names numbers. "Not ready" is a verdict a contributor cannot
/// act on; "312 strings untranslated, 47 more needed to reach the floor" is a
/// morning's work with an end in sight.
///
/// Not every variant blocks — see [`SababAdamAlnashr::hasim`]. The untranslated
/// remainder and the review queue are reported for a publishable project too,
/// because a contributor sitting on two hundred untranslated strings wants that
/// sentence whether or not the floor is cleared.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababAdamAlnashr {
    /// The project holds no strings at all.
    BilaNusus,
    /// Strings with no translation.
    NususGhayrMutarjama {
        /// How many.
        adad: u32,
        /// How many more would have to be translated to clear the by-string
        /// floor, or zero when the floor is already cleared.
        matlub_lilhadd: u32,
    },
    /// Translations no human has approved yet.
    BiIntizarAlmuraja {
        /// How many in total.
        adad: u32,
        /// Of those, machine output no human has read.
        aali_faqat: u32,
        /// Of those, a human's own unfinished draft.
        musawwada: u32,
        /// Of those, explicitly flagged for a second pair of eyes.
        matlub_muraja: u32,
    },
    /// Translations a reviewer rejected and nobody has replaced.
    ///
    /// The text is kept — see [`HalatMuraja::Marfuda`] — so these count as
    /// translated by [`HalatMuraja::mutarjam`] while being exactly the strings a
    /// human said were wrong.
    MarfudaBaqiya {
        /// How many.
        adad: u32,
    },
    /// By-string coverage is below the publishable floor.
    TaghtiyaDunAlhadd {
        /// Where it stands, as a percentage.
        miawi: f64,
        /// The floor, as a percentage.
        hadd_miawi: f64,
        /// How many more strings would clear it.
        naqis: u32,
    },
    /// Opening-session coverage is below the publishable floor.
    AwwalSaaDunAlhadd {
        /// Where it stands, as a percentage.
        miawi: f64,
        /// The floor, as a percentage.
        hadd_miawi: f64,
        /// How many more of the observed opening strings would clear it.
        naqis: u32,
    },
    /// No capture session has ever recorded what the opening of the game shows.
    ///
    /// Blocking, and the whole reason this enum exists. Without it the empty
    /// denominator in [`Taghtiya::nisba_awwal`] reads as a perfect first hour and
    /// the project sails through a floor nothing was measured against.
    BilaJalsatAwwal,
    /// A capture session was recorded and it names no strings.
    ///
    /// A different defect from having no session: the session ran, the hooks
    /// were in place, and nothing came back. Blocking for the same reason and
    /// pointing at a different fix.
    JalsatAwwalFarigha,
    /// Strings carrying a quality flag that forbids shipping.
    ///
    /// Computed from [`MudkhalNass::yamnaa_alnashr`], which is the same
    /// judgement the workspace shows beside the string — a lost placeholder or
    /// an empty translation of a non-empty source.
    AlamatMania {
        /// How many.
        adad: u32,
    },
    /// Nothing a player can see carried any weight.
    ///
    /// Every string in the project is in a zero-weight class. A patch whose
    /// entire weighted denominator is zero changes nothing on screen, and
    /// publishing it puts a listing in the catalogue that no player benefits
    /// from.
    BilaWaznZahir {
        /// How many strings were excluded for carrying no weight.
        mustabaad: u32,
    },
}

impl SababAdamAlnashr {
    /// Whether this cause alone stops the project from being publishable.
    ///
    /// The untranslated remainder and the review queue do not: the floor in
    /// [`Taghtiya::qabila_lil_nashr`] is deliberately low, and a patch that
    /// translates the menus and the first chapter is worth publishing. What
    /// blocks is an empty project, a floor not cleared, a first-hour figure that
    /// rests on nothing, a string that cannot ship, and a project with no
    /// player-visible content.
    #[must_use]
    pub const fn hasim(self) -> bool {
        matches!(
            self,
            Self::BilaNusus
                | Self::TaghtiyaDunAlhadd { .. }
                | Self::AwwalSaaDunAlhadd { .. }
                | Self::BilaJalsatAwwal
                | Self::JalsatAwwalFarigha
                | Self::AlamatMania { .. }
                | Self::BilaWaznZahir { .. }
        )
    }

    /// How the interface colours the row.
    #[must_use]
    pub const fn khutura(self) -> Khutura {
        match self {
            Self::AlamatMania { .. } => Khutura::Fadih,
            Self::BilaNusus
            | Self::TaghtiyaDunAlhadd { .. }
            | Self::AwwalSaaDunAlhadd { .. }
            | Self::BilaJalsatAwwal
            | Self::JalsatAwwalFarigha
            | Self::BilaWaznZahir { .. } => Khutura::Khatar,
            Self::NususGhayrMutarjama { .. }
            | Self::BiIntizarAlmuraja { .. }
            | Self::MarfudaBaqiya { .. } => Khutura::Tanbeeh,
        }
    }

    /// A stable key, for grouping and for a caller that renders its own copy.
    #[must_use]
    pub const fn miftah(self) -> &'static str {
        match self {
            Self::BilaNusus => "bila_nusus",
            Self::NususGhayrMutarjama { .. } => "nusus_ghayr_mutarjama",
            Self::BiIntizarAlmuraja { .. } => "bi_intizar_almuraja",
            Self::MarfudaBaqiya { .. } => "marfuda_baqiya",
            Self::TaghtiyaDunAlhadd { .. } => "taghtiya_dun_alhadd",
            Self::AwwalSaaDunAlhadd { .. } => "awwal_saa_dun_alhadd",
            Self::BilaJalsatAwwal => "bila_jalsat_awwal",
            Self::JalsatAwwalFarigha => "jalsat_awwal_farigha",
            Self::AlamatMania { .. } => "alamat_mania",
            Self::BilaWaznZahir { .. } => "bila_wazn_zahir",
        }
    }

    /// The sentence, with its counts, in Arabic.
    #[must_use]
    pub fn wasf_arabi(self) -> String {
        match self {
            Self::BilaNusus => "لا نصوص في المشروع.".to_owned(),
            Self::NususGhayrMutarjama {
                adad,
                matlub_lilhadd,
            } => {
                if matlub_lilhadd > 0 {
                    format!("{adad} عبارة دون ترجمة، ويلزم {matlub_lilhadd} منها لبلوغ الحدّ.")
                } else {
                    format!("{adad} عبارة دون ترجمة، والحدّ مبلوغ.")
                }
            },
            Self::BiIntizarAlmuraja {
                adad,
                aali_faqat,
                musawwada,
                matlub_muraja,
            } => format!(
                "{adad} عبارة تنتظر المراجعة: {aali_faqat} آلية لم يقرأها إنسان، \
                 و{musawwada} مسوّدة، و{matlub_muraja} مطلوبة للمراجعة."
            ),
            Self::MarfudaBaqiya { adad } => {
                format!("{adad} ترجمة مرفوضة لم يُستبدل نصّها بعد.")
            },
            Self::TaghtiyaDunAlhadd {
                miawi,
                hadd_miawi,
                naqis,
            } => format!(
                "التغطية {miawi:.0}٪ دون حدّ النشر {hadd_miawi:.0}٪؛ يلزم {naqis} عبارة أخرى."
            ),
            Self::AwwalSaaDunAlhadd {
                miawi,
                hadd_miawi,
                naqis,
            } => format!(
                "تغطية أول ساعة {miawi:.0}٪ دون حدّ النشر {hadd_miawi:.0}٪؛ يلزم {naqis} عبارة \
                 أخرى مما رُصد في الافتتاح."
            ),
            Self::BilaJalsatAwwal => {
                "لم تُسجَّل جلسة التقاط لافتتاح اللعبة، فلا قياس لتغطية أول ساعة أصلًا.".to_owned()
            },
            Self::JalsatAwwalFarigha => {
                "سُجّلت جلسة افتتاح ولم تُسمِّ أي عبارة، فالقياس فارغ لا كامل.".to_owned()
            },
            Self::AlamatMania { adad } => {
                format!("{adad} عبارة تحمل علامة جودة تمنع النشر.")
            },
            Self::BilaWaznZahir { mustabaad } => {
                format!("لا نصّ ظاهر للاعب في المشروع: {mustabaad} عبارة كلّها من تصنيفات بلا وزن.")
            },
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(self) -> String {
        match self {
            Self::BilaNusus => "The project holds no strings.".to_owned(),
            Self::NususGhayrMutarjama {
                adad,
                matlub_lilhadd,
            } => {
                if matlub_lilhadd > 0 {
                    format!(
                        "{adad} string(s) untranslated; {matlub_lilhadd} of them are needed to \
                         reach the floor."
                    )
                } else {
                    format!("{adad} string(s) untranslated; the floor is already cleared.")
                }
            },
            Self::BiIntizarAlmuraja {
                adad,
                aali_faqat,
                musawwada,
                matlub_muraja,
            } => format!(
                "{adad} string(s) awaiting review: {aali_faqat} machine-translated and unread, \
                 {musawwada} draft(s), {matlub_muraja} explicitly flagged."
            ),
            Self::MarfudaBaqiya { adad } => {
                format!("{adad} rejected translation(s) have not been replaced.")
            },
            Self::TaghtiyaDunAlhadd {
                miawi,
                hadd_miawi,
                naqis,
            } => format!(
                "Coverage is {miawi:.0}%, below the {hadd_miawi:.0}% publishing floor; {naqis} \
                 more string(s) would clear it."
            ),
            Self::AwwalSaaDunAlhadd {
                miawi,
                hadd_miawi,
                naqis,
            } => format!(
                "First-hour coverage is {miawi:.0}%, below the {hadd_miawi:.0}% floor; {naqis} \
                 more of the observed opening strings would clear it."
            ),
            Self::BilaJalsatAwwal => {
                "No capture session recorded the opening of the game, so first-hour coverage was \
                 never measured at all."
                    .to_owned()
            },
            Self::JalsatAwwalFarigha => {
                "A capture session was recorded and named no strings, so the first-hour figure is \
                 empty rather than complete."
                    .to_owned()
            },
            Self::AlamatMania { adad } => {
                format!("{adad} string(s) carry a quality flag that forbids shipping.")
            },
            Self::BilaWaznZahir { mustabaad } => format!(
                "Nothing in the project is player-visible: all {mustabaad} string(s) fall in \
                 zero-weight classes."
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

/// Everything this compile knows about its own coverage.
///
/// Serialized into the package's metadata section beside the overflow report, so
/// Phase 18 reads the contributor's own figures rather than recomputing them
/// from a project file it does not have.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag records whether a distinct measurement was taken at all; folding them \
              into one state would recreate the ambiguity between unmeasured and measured-zero \
              that this whole module exists to remove"
)]
pub struct TaqrirTaghtiya {
    /// The whole project, one entry per extracted string.
    pub kulli: Taghtiya,
    /// The same project with duplicates collapsed.
    ///
    /// One entry per duplicate group, because one translation serves every
    /// identical source — see [`MudkhalNass::majmua`]. The pair of numbers is
    /// what tells a contributor whether "eight thousand strings left" is eight
    /// thousand decisions or four hundred decisions copied twenty times.
    pub mujammaa: Taghtiya,
    /// Per interface element class, **worst covered first**.
    ///
    /// Ordered so the first row of the table is the one that needs work, which
    /// is the whole reason the aggregate was broken apart.
    pub aqsam: Vec<QismTaghtiya>,
    /// The weighted measurement, with its provenance.
    pub mawzuna: TaghtiyaMawzuna,
    /// How many strings sit in each review state, every state present.
    pub hasab_hala: BTreeMap<HalatMuraja, u32>,
    /// Whether an opening-session record was supplied at all.
    pub jalsat_awwal_masjjala: bool,
    /// Whether any runtime frequency was supplied at all.
    pub mulahazat_takrar_masjjala: bool,
    /// The strings whose weight used an observed frequency.
    ///
    /// Carried so a reader of the package can tell which half of
    /// [`TaghtiyaMawzuna`] any individual string is in without the capture
    /// session that produced it. [`TaqrirTaghtiya::bila_tafsil_alwazn`] drops it
    /// for a project large enough that the identities cost more than the answer.
    pub nusus_bi_mulahaza: BTreeSet<NassId>,
    /// Whether that detail was dropped from this copy.
    ///
    /// Recorded so an empty [`TaqrirTaghtiya::nusus_bi_mulahaza`] that means
    /// "trimmed" is never read as one that means "nothing was observed".
    pub tafsil_alwazn_muqallam: bool,
    /// Every reason, blocking or not, with counts.
    pub asbab: Vec<SababAdamAlnashr>,
    /// The verdict.
    ///
    /// Taken from [`Taghtiya::qabila_lil_nashr`] and then narrowed by the
    /// blocking causes. It is never widened: no cause here can make a project
    /// publishable that the shared floor refused.
    pub qabila_lil_nashr: bool,
}

impl TaqrirTaghtiya {
    /// Only the causes that actually block publication.
    #[must_use]
    pub fn asbab_hasima(&self) -> Vec<SababAdamAlnashr> {
        self.asbab
            .iter()
            .copied()
            .filter(|sabab| sabab.hasim())
            .collect()
    }

    /// One class's coverage, when the project holds any string of that class.
    #[must_use]
    pub fn qism(&self, tasnif: TasnifNass) -> Option<&QismTaghtiya> {
        self.aqsam.iter().find(|qism| qism.tasnif == tasnif)
    }

    /// The worst-covered class, which is the first row of the table.
    #[must_use]
    pub fn aswa_qism(&self) -> Option<&QismTaghtiya> {
        self.aqsam.first()
    }

    /// How many strings are in one review state.
    #[must_use]
    pub fn adad_hala(&self, hala: HalatMuraja) -> u32 {
        self.hasab_hala.get(&hala).copied().unwrap_or(0)
    }

    /// How many translations exist that no human has approved.
    #[must_use]
    pub fn bi_intizar_almuraja(&self) -> u32 {
        self.adad_hala(HalatMuraja::TarjamaAaliya)
            .saturating_add(self.adad_hala(HalatMuraja::Musawwada))
            .saturating_add(self.adad_hala(HalatMuraja::LilMuraja))
    }

    /// The sections folded back into one measurement.
    ///
    /// Uses [`Taghtiya::damm`], which is what the shared type provides for
    /// exactly this. The result equals [`TaqrirTaghtiya::kulli`] for a report
    /// this module produced, and a caller that has merged reports from several
    /// containers can check that it still does.
    #[must_use]
    pub fn majmu_alaqsam(&self) -> Taghtiya {
        let mut majmu = Taghtiya::default();
        for qism in &self.aqsam {
            majmu.damm(&qism.taghtiya);
        }
        majmu
    }

    /// The same report without the per-string weight provenance.
    #[must_use]
    pub fn bila_tafsil_alwazn(mut self) -> Self {
        self.nusus_bi_mulahaza.clear();
        self.tafsil_alwazn_muqallam = true;
        self
    }

    /// Where one string's weight came from, when the detail survives.
    ///
    /// [`None`] after [`TaqrirTaghtiya::bila_tafsil_alwazn`], because answering
    /// [`MasdarWazn::Tasnif`] for a string whose record was trimmed would be a
    /// guess dressed as a measurement.
    #[must_use]
    pub fn masdar_wazn(&self, nass: NassId) -> Option<MasdarWazn> {
        if self.tafsil_alwazn_muqallam {
            return None;
        }
        Some(if self.nusus_bi_mulahaza.contains(&nass) {
            MasdarWazn::Mulahaza
        } else {
            MasdarWazn::Tasnif
        })
    }

    /// The summary sentence, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let aswa = self
            .aswa_qism()
            .map_or_else(|| "لا أقسام".to_owned(), QismTaghtiya::wasf_arabi);
        format!(
            "{} {} أضعف الأقسام: {aswa}.",
            self.kulli.wasf_arabi(),
            self.mawzuna.wasf_arabi()
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let aswa = self
            .aswa_qism()
            .map_or_else(|| "no sections".to_owned(), QismTaghtiya::wasf_injilizi);
        format!(
            "{} {} Weakest section: {aswa}.",
            self.kulli.wasf_injilizi(),
            self.mawzuna.wasf_injilizi()
        )
    }
}

// ---------------------------------------------------------------------------
// Measuring a project
// ---------------------------------------------------------------------------

/// The yes/no facts one string contributes to a measurement.
///
/// Carried together rather than as loose arguments: every one of them is a
/// `bool`, so a call site that transposed two would still compile and would
/// silently report confirmed strings as opening-session ones.
#[derive(Debug, Clone, Copy, Default)]
struct AlamMadkhal {
    /// Has text, and a review record that admits to having it.
    mutarjam: bool,
    /// A human review confirmed it.
    muakkad: bool,
    /// A capture session saw it during the game's opening.
    fi_alawwal: bool,
}

/// What one duplicate group has accumulated so far.
///
/// A group shares a translation, so it is covered when *any* member is — see
/// [`MudkhalNass::majmua`]. Counting a group as covered only when every member
/// is would count the same decision as several, which is the double count the
/// collapsed measurement exists to remove.
#[derive(Debug, Clone, Copy, Default)]
struct HasilMajmua {
    alam: AlamMadkhal,
    takrar: u64,
}

/// Measures a project's coverage.
///
/// `mulahazat` is [`None`] when no runtime capture recorded any frequency, and
/// `awwal` is [`None`] when no capture session recorded the opening of the game.
/// Both absences are carried into the report as facts rather than smoothed into
/// defaults: the first turns every weight into a class weight and shows in
/// [`TaghtiyaMawzuna::mawthuqa`], the second blocks publication through
/// [`SababAdamAlnashr::BilaJalsatAwwal`].
#[must_use]
pub fn ihsib_taghtiya(
    nusus: &[MudkhalNass],
    mulahazat: Option<&MulahazatTakrar>,
    awwal: Option<&MajmuatAwwal>,
) -> TaqrirTaghtiya {
    let mut kulli = Taghtiya::default();
    let mut mawzuna = TaghtiyaMawzuna::default();
    let mut aqsam: BTreeMap<TasnifNass, QismTaghtiya> = BTreeMap::new();
    let mut majmuat: BTreeMap<NassId, HasilMajmua> = BTreeMap::new();
    let mut hasab_hala: BTreeMap<HalatMuraja, u32> = BTreeMap::new();
    let mut nusus_bi_mulahaza: BTreeSet<NassId> = BTreeSet::new();
    let mut alamat_mania = 0_u32;

    for hala in HALAT_MURAJA {
        let _ = hasab_hala.insert(hala, 0);
    }

    for madkhal in nusus {
        let hala = madkhal.muraja.hala();
        if let Some(khana) = hasab_hala.get_mut(&hala) {
            *khana = khana.saturating_add(1);
        }
        if madkhal.yamnaa_alnashr() {
            alamat_mania = alamat_mania.saturating_add(1);
        }

        // A translation is text *and* a review record that admits to having one.
        // A `hadaf` of `Some("")` is not a translation, and a record still in
        // `LamTutarjam` carrying text is a project file somebody hand-edited.
        let hadaf = madkhal.hadaf.as_deref().unwrap_or("");
        let alam = AlamMadkhal {
            mutarjam: !hadaf.trim().is_empty() && hala.mutarjam(),
            muakkad: hala == HalatMuraja::Muakkada,
            fi_alawwal: awwal.is_some_and(|majmua| majmua.tahtawi(madkhal.id)),
        };

        // A string that is in the table occurred at least once, whatever the
        // recorded count says. Zero here is a count extraction failed to fill,
        // not evidence that the string is never drawn.
        let takrar = u64::from(madkhal.takrar.max(1));

        damm_madkhal(&mut kulli, alam, takrar);

        let qism = aqsam.entry(madkhal.tasnif).or_insert_with(|| QismTaghtiya {
            tasnif: madkhal.tasnif,
            taghtiya: Taghtiya::default(),
            mawzuna: TaghtiyaMawzuna::default(),
        });
        damm_madkhal(&mut qism.taghtiya, alam, takrar);

        let hasil = majmuat
            .entry(madkhal.majmua.unwrap_or(madkhal.id))
            .or_default();
        hasil.alam.mutarjam |= alam.mutarjam;
        hasil.alam.muakkad |= alam.muakkad;
        hasil.alam.fi_alawwal |= alam.fi_alawwal;
        hasil.takrar = hasil.takrar.saturating_add(takrar);

        let wazn_asas = wazn_tasnif(madkhal.tasnif);
        if wazn_asas.is_nan() || wazn_asas <= 0.0 {
            mawzuna.mustabaad = mawzuna.mustabaad.saturating_add(1);
            qism.mawzuna.mustabaad = qism.mawzuna.mustabaad.saturating_add(1);
            continue;
        }

        let marrat = mulahazat.and_then(|sijill| sijill.takrar(madkhal.id));
        let wazn = wazn_asas * muamil_takrar(marrat);
        let masdar = if marrat.is_some() {
            MasdarWazn::Mulahaza
        } else {
            MasdarWazn::Tasnif
        };
        if masdar == MasdarWazn::Mulahaza {
            let _ = nusus_bi_mulahaza.insert(madkhal.id);
        }
        damm_wazn(&mut mawzuna, wazn, masdar, alam);
        damm_wazn(&mut qism.mawzuna, wazn, masdar, alam);
    }

    // One group, one decision. A group whose opening-session member carries no
    // text of its own is still covered, because the translation on any member
    // serves all of them — which is exactly the double count the collapsed view
    // exists to remove.
    let mut mujammaa = Taghtiya::default();
    for hasil in majmuat.values() {
        damm_madkhal(&mut mujammaa, hasil.alam, hasil.takrar);
    }

    let mut murattaba: Vec<QismTaghtiya> = aqsam.into_values().collect();
    // Worst covered first, through `total_cmp` rather than a partial comparison:
    // a section whose ratio came back `NaN` sorts to one end deterministically
    // instead of leaving the whole table's order unspecified.
    murattaba.sort_by(|awwal_qism, thani| {
        awwal_qism
            .taghtiya
            .nisba()
            .total_cmp(&thani.taghtiya.nisba())
            .then_with(|| thani.taghtiya.majmu.cmp(&awwal_qism.taghtiya.majmu))
            .then_with(|| awwal_qism.tasnif.cmp(&thani.tasnif))
    });

    let jalsat_awwal_masjjala = awwal.is_some_and(|majmua| !majmua.khaliya());
    let mulahazat_takrar_masjjala = mulahazat.is_some_and(|sijill| !sijill.khaliya());
    let asbab = ihsib_asbab(&kulli, &mawzuna, &hasab_hala, alamat_mania, awwal);
    // The floor itself is the shared type's answer, never recomputed here. The
    // causes only narrow it: no cause in this module can make a project
    // publishable that `qabila_lil_nashr` refused.
    let qabila_lil_nashr =
        kulli.qabila_lil_nashr() && !asbab.iter().copied().any(SababAdamAlnashr::hasim);

    TaqrirTaghtiya {
        kulli,
        mujammaa,
        aqsam: murattaba,
        mawzuna,
        hasab_hala,
        jalsat_awwal_masjjala,
        mulahazat_takrar_masjjala,
        nusus_bi_mulahaza,
        tafsil_alwazn_muqallam: false,
        asbab,
        qabila_lil_nashr,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Folds one string into a [`Taghtiya`].
///
/// The single place the shared type's seven counters are incremented, so the
/// project total, every section and the duplicate-collapsed view are all filled
/// by identical arithmetic rather than by three loops that drifted apart.
const fn damm_madkhal(taghtiya: &mut Taghtiya, alam: AlamMadkhal, takrar: u64) {
    taghtiya.majmu = taghtiya.majmu.saturating_add(1);
    taghtiya.majmu_takrar = taghtiya.majmu_takrar.saturating_add(takrar);
    if alam.mutarjam {
        taghtiya.mutarjam = taghtiya.mutarjam.saturating_add(1);
        taghtiya.mutarjam_takrar = taghtiya.mutarjam_takrar.saturating_add(takrar);
    }
    if alam.muakkad {
        taghtiya.muakkad = taghtiya.muakkad.saturating_add(1);
    }
    if alam.fi_alawwal {
        taghtiya.majmu_awwal = taghtiya.majmu_awwal.saturating_add(1);
        if alam.mutarjam {
            taghtiya.mutarjam_awwal = taghtiya.mutarjam_awwal.saturating_add(1);
        }
    }
}

/// Folds one string's weight into a weighted measurement.
///
/// `alam.fi_alawwal` has no weighted counterpart: the opening session is
/// counted, never weighted.
fn damm_wazn(mawzuna: &mut TaghtiyaMawzuna, wazn: f64, masdar: MasdarWazn, alam: AlamMadkhal) {
    mawzuna.wazn_kulli += wazn;
    if alam.mutarjam {
        mawzuna.wazn_mutarjam += wazn;
    }
    if alam.muakkad {
        mawzuna.wazn_muakkad += wazn;
    }
    match masdar {
        MasdarWazn::Mulahaza => {
            mawzuna.bi_mulahaza = mawzuna.bi_mulahaza.saturating_add(1);
            mawzuna.wazn_bi_mulahaza += wazn;
        },
        MasdarWazn::Tasnif => {
            mawzuna.bi_tasnif = mawzuna.bi_tasnif.saturating_add(1);
            mawzuna.wazn_bi_tasnif += wazn;
        },
    }
}

/// The multiplier an observed frequency contributes.
///
/// [`None`] — nothing was observed — returns exactly `1.0`, so the string is
/// worth its class weight and not a fraction more. **No frequency is assumed
/// for it**, which is the difference between this and every "estimated
/// occurrences" heuristic.
///
/// An observation contributes `WAZN_MULAHAZA_ASAS + ln(1 + marrat)`. The
/// logarithm is the point: the difference between a string drawn once and one
/// drawn a hundred times is real and should move the number, and the difference
/// between a hundred and ten thousand is not — the player has read both, and a
/// linear weight would let one chatty status line outweigh an entire menu tree.
fn muamil_takrar(marrat: Option<u64>) -> f64 {
    let Some(adad) = marrat else {
        return 1.0;
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a draw count above 2^53 is not reachable in a capture session, and the value \
                  feeds a logarithm whose result is a display weight"
    )]
    let qeema = adad as f64;
    WAZN_MULAHAZA_ASAS + qeema.ln_1p()
}

/// Builds the enumerated causes.
///
/// Computes every cause the project exhibits, blocking or not, so the interface
/// can render the untranslated remainder and the review queue for a publishable
/// project too. Whether a cause blocks is [`SababAdamAlnashr::hasim`]'s answer,
/// asked once, at the call site that forms the verdict.
fn ihsib_asbab(
    kulli: &Taghtiya,
    mawzuna: &TaghtiyaMawzuna,
    hasab_hala: &BTreeMap<HalatMuraja, u32>,
    alamat_mania: u32,
    awwal: Option<&MajmuatAwwal>,
) -> Vec<SababAdamAlnashr> {
    let mut asbab = Vec::new();

    if kulli.majmu == 0 {
        asbab.push(SababAdamAlnashr::BilaNusus);
        return asbab;
    }

    let ghayr_mutarjam = kulli.mutabaqqi();
    if ghayr_mutarjam > 0 {
        asbab.push(SababAdamAlnashr::NususGhayrMutarjama {
            adad: ghayr_mutarjam,
            matlub_lilhadd: naqis_lilhadd(kulli.majmu, kulli.mutarjam, HADD_TAGHTIYA_LILNASHR),
        });
    }

    let adad_hala = |hala: HalatMuraja| hasab_hala.get(&hala).copied().unwrap_or(0);
    let aali_faqat = adad_hala(HalatMuraja::TarjamaAaliya);
    let musawwada = adad_hala(HalatMuraja::Musawwada);
    let matlub_muraja = adad_hala(HalatMuraja::LilMuraja);
    let muntazir = aali_faqat
        .saturating_add(musawwada)
        .saturating_add(matlub_muraja);
    if muntazir > 0 {
        asbab.push(SababAdamAlnashr::BiIntizarAlmuraja {
            adad: muntazir,
            aali_faqat,
            musawwada,
            matlub_muraja,
        });
    }

    let marfuda = adad_hala(HalatMuraja::Marfuda);
    if marfuda > 0 {
        asbab.push(SababAdamAlnashr::MarfudaBaqiya { adad: marfuda });
    }

    if f64::from(kulli.nisba()) < HADD_TAGHTIYA_LILNASHR {
        asbab.push(SababAdamAlnashr::TaghtiyaDunAlhadd {
            miawi: f64::from(kulli.nisba()) * 100.0,
            hadd_miawi: HADD_TAGHTIYA_LILNASHR * 100.0,
            naqis: naqis_lilhadd(kulli.majmu, kulli.mutarjam, HADD_TAGHTIYA_LILNASHR),
        });
    }

    // The opening-session figure is judged only where it was measured. An absent
    // or empty session produces its own cause instead of a ratio, because
    // `nisba_awwal` answers 1.0 over an empty denominator and that answer must
    // never reach a verdict.
    match awwal {
        None => asbab.push(SababAdamAlnashr::BilaJalsatAwwal),
        Some(majmua) if majmua.khaliya() => {
            asbab.push(SababAdamAlnashr::JalsatAwwalFarigha);
        },
        Some(_) => {
            if f64::from(kulli.nisba_awwal()) < HADD_AWWAL_LILNASHR {
                asbab.push(SababAdamAlnashr::AwwalSaaDunAlhadd {
                    miawi: f64::from(kulli.nisba_awwal()) * 100.0,
                    hadd_miawi: HADD_AWWAL_LILNASHR * 100.0,
                    naqis: naqis_lilhadd(
                        kulli.majmu_awwal,
                        kulli.mutarjam_awwal,
                        HADD_AWWAL_LILNASHR,
                    ),
                });
            }
        },
    }

    if alamat_mania > 0 {
        asbab.push(SababAdamAlnashr::AlamatMania { adad: alamat_mania });
    }

    if mawzuna.wazn_kulli.is_nan() || mawzuna.wazn_kulli <= 0.0 {
        asbab.push(SababAdamAlnashr::BilaWaznZahir {
            mustabaad: mawzuna.mustabaad,
        });
    }

    asbab
}

/// How many more covered strings would clear a floor.
///
/// Computed in `f64` throughout — a ratio of two counts is never taken by
/// integer division, which would floor the requirement and report a project as
/// one string closer to the floor than it is.
fn naqis_lilhadd(majmu: u32, mughatta: u32, hadd: f64) -> u32 {
    let matlub = (f64::from(majmu) * hadd)
        .ceil()
        .clamp(0.0, f64::from(u32::MAX));
    // `ceil` made the value integral and `clamp` pinned it into 0..=u32::MAX,
    // so the conversion below is exact and can neither wrap nor lose a sign.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ceiled then clamped into 0.0..=u32::MAX, so the value is exactly representable"
    )]
    let matlub = matlub as u32;
    matlub.saturating_sub(mughatta)
}
