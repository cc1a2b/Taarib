//! Stages, progress reports, and where a report goes.
//!
//! Shaped after `taarib_mustawda::tanzeel`: a `Copy`-cheap event, a
//! three-variant reporter the caller picks, and a synchronous `ballagh` that
//! swallows a receiver which has gone away. The one deliberate difference is
//! [`Taqaddum::majmu`], which is an [`Option`] here — because two of the seven
//! stages genuinely cannot know their denominator before they run, and a
//! fabricated one is worse than an honest absence.

use std::fmt;

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;

/// The seven stages of an automatic run, plus the terminal one.
///
/// The numbering is the product's own and is stable: a log line naming stage 4
/// means layout in this build and in every later one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarhalaTilqai {
    /// Probing the engine and resolving the support tier.
    Fahs,
    /// Reading strings out of the game's containers, and the refusal report.
    Istikhraj,
    /// Machine translation, through the placeholder guard.
    Tarjama,
    /// Font validation, size discovery, coverage and the overflow report.
    Takhtit,
    /// Laying out every string at every size, building the atlas, and writing
    /// the container.
    Tarqee,
    /// Sealing the container with the contributor's key.
    Khatm,
    /// The safety gate, then the transactional install.
    Tathbeet,
    /// Nothing left to do.
    Tamma,
}

impl MarhalaTilqai {
    /// Every stage in the order they run.
    pub const KUL: [Self; 8] = [
        Self::Fahs,
        Self::Istikhraj,
        Self::Tarjama,
        Self::Takhtit,
        Self::Tarqee,
        Self::Khatm,
        Self::Tathbeet,
        Self::Tamma,
    ];

    /// The stage's number, one through seven; eight is the terminal stage.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Fahs => 1,
            Self::Istikhraj => 2,
            Self::Tarjama => 3,
            Self::Takhtit => 4,
            Self::Tarqee => 5,
            Self::Khatm => 6,
            Self::Tathbeet => 7,
            Self::Tamma => 8,
        }
    }

    /// The stable identifier used in the journal and in structured context.
    #[must_use]
    pub const fn ramz(self) -> &'static str {
        match self {
            Self::Fahs => "fahs",
            Self::Istikhraj => "istikhraj",
            Self::Tarjama => "tarjama",
            Self::Takhtit => "takhtit",
            Self::Tarqee => "tarqee",
            Self::Khatm => "khatm",
            Self::Tathbeet => "tathbeet",
            Self::Tamma => "tamma",
        }
    }

    /// The label the interface shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Fahs => "فحص المحرّك",
            Self::Istikhraj => "استخراج النصوص",
            Self::Tarjama => "الترجمة الآلية",
            Self::Takhtit => "الخطوط والمقاسات",
            Self::Tarqee => "تجميع الرقعة",
            Self::Khatm => "ختم الرقعة",
            Self::Tathbeet => "التثبيت",
            Self::Tamma => "اكتمل",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Fahs => "Probing the engine",
            Self::Istikhraj => "Extracting strings",
            Self::Tarjama => "Machine translation",
            Self::Takhtit => "Fonts and sizes",
            Self::Tarqee => "Compiling the patch",
            Self::Khatm => "Signing the patch",
            Self::Tathbeet => "Installing",
            Self::Tamma => "Done",
        }
    }

    /// Whether a cancellation at this stage still leaves the game untouched
    /// without any work being undone.
    ///
    /// True for everything before [`Self::Tathbeet`], because every one of
    /// those stages writes only inside the run directory. False for the
    /// install, which is still *reversible* — the manifest is what makes it so
    /// — but no longer free: reversing it is a second pass over the game's
    /// files rather than deleting a scratch folder.
    #[must_use]
    pub const fn ilgha_majaniya(self) -> bool {
        !matches!(self, Self::Tathbeet)
    }
}

impl fmt::Display for MarhalaTilqai {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.wasf_injilizi(), self.raqm())
    }
}

/// One progress report.
///
/// `munjaz` and `majmu` are in whatever unit the stage counts in — strings,
/// containers, requests, files — and [`Taqaddum::amal`] says which, so a
/// caller never has to infer the unit from the stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taqaddum {
    /// Which stage.
    pub marhala: MarhalaTilqai,
    /// Units accounted for so far.
    pub munjaz: u64,
    /// Units this stage will account for in total, when that is knowable.
    ///
    /// [`None`] is not "unknown yet, ask again": it is this stage's honest
    /// statement that no denominator exists to report. Only extraction and the
    /// compile's inner layout pass are ever `None`, and both say so in `amal`.
    pub majmu: Option<u64>,
    /// What is happening, in English, in one clause.
    pub amal: String,
    /// Settled spend so far, in nano-dollars, including previous runs.
    pub munfaq: u64,
    /// The ceiling this run honours, when one is set.
    pub saqf: Option<u64>,
}

impl Taqaddum {
    /// How far through this stage is, as whole percent, when a denominator
    /// exists.
    ///
    /// A stage with a zero denominator reports one hundred: it has nothing to
    /// do and is therefore finished, which is the answer a progress bar needs.
    #[must_use]
    pub fn nisba(&self) -> Option<u8> {
        let majmu = self.majmu?;
        if majmu == 0 {
            return Some(100);
        }
        let mia = self
            .munjaz
            .saturating_mul(100)
            .min(majmu.saturating_mul(100));
        #[expect(
            clippy::integer_division,
            reason = "whole percent is the unit a progress bar wants, and truncating is the \
                      right direction: ninety-nine point nine percent must not read as done"
        )]
        let nisba = mia / majmu;
        u8::try_from(nisba).ok()
    }

    /// Units still to come, when a denominator exists.
    #[must_use]
    pub fn mutabaqqi(&self) -> Option<u64> {
        self.majmu.map(|majmu| majmu.saturating_sub(self.munjaz))
    }

    /// How much of the ceiling is left, when one is set.
    #[must_use]
    pub const fn baqi_min_saqf(&self) -> Option<u64> {
        match self.saqf {
            Some(saqf) => Some(saqf.saturating_sub(self.munfaq)),
            None => None,
        }
    }
}

/// Where progress goes.
///
/// The caller picks. This crate has no opinion about Tauri, a terminal, a test
/// harness or nothing at all, and cannot be made to acquire one.
#[derive(Default)]
pub enum MukhbirTaqaddum {
    /// Nowhere.
    #[default]
    Samit,
    /// Onto an unbounded channel, which a slow reader cannot stall.
    Qanat(UnboundedSender<Taqaddum>),
    /// Into a callback, called on the task driving the run.
    Nida(Box<dyn Fn(Taqaddum) + Send + Sync>),
}

impl MukhbirTaqaddum {
    /// Wraps a callback.
    #[must_use]
    pub fn min_nida<F: Fn(Taqaddum) + Send + Sync + 'static>(nida: F) -> Self {
        Self::Nida(Box::new(nida))
    }

    /// Delivers one report, ignoring a receiver that has gone away.
    ///
    /// Synchronous and infallible, and called from inside async code, so a
    /// callback that blocks stalls the run.
    pub fn ballagh(&self, taqaddum: Taqaddum) {
        match self {
            Self::Samit => {},
            Self::Qanat(mursil) => {
                let _ = mursil.send(taqaddum);
            },
            Self::Nida(nida) => nida(taqaddum),
        }
    }
}

impl fmt::Debug for MukhbirTaqaddum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let naw = match self {
            Self::Samit => "Samit",
            Self::Qanat(_) => "Qanat",
            Self::Nida(_) => "Nida",
        };
        f.debug_struct("MukhbirTaqaddum")
            .field("naw", &naw)
            .finish()
    }
}

/// The reporter, plus the run-wide numbers every report has to carry.
///
/// Carrying the spend and the ceiling here rather than passing them to every
/// call site is what stops a stage from reporting progress without cost: there
/// is no way to build a [`Taqaddum`] through this type that omits them.
#[derive(Debug)]
pub struct Muraqib<'a> {
    /// Where the reports go.
    mukhbir: &'a MukhbirTaqaddum,
    /// The ceiling, when one is set.
    saqf: Option<u64>,
    /// Settled spend, updated by the translation stage as replies land.
    munfaq: u64,
}

impl<'a> Muraqib<'a> {
    /// Binds a reporter to a run's ceiling.
    #[must_use]
    pub const fn jadeed(mukhbir: &'a MukhbirTaqaddum, saqf: Option<u64>) -> Self {
        Self {
            mukhbir,
            saqf,
            munfaq: 0,
        }
    }

    /// Records what the run has spent so far, so later reports carry it.
    pub const fn sajjil_infaq(&mut self, munfaq: u64) {
        self.munfaq = munfaq;
    }

    /// What the run has spent so far.
    #[must_use]
    pub const fn munfaq(&self) -> u64 {
        self.munfaq
    }

    /// The ceiling, when one is set.
    #[must_use]
    pub const fn saqf(&self) -> Option<u64> {
        self.saqf
    }

    /// One report with a knowable denominator.
    pub fn ballagh(
        &self,
        marhala: MarhalaTilqai,
        munjaz: u64,
        majmu: u64,
        amal: impl Into<String>,
    ) {
        self.arsil(marhala, munjaz, Some(majmu), amal);
    }

    /// One report for a stage whose denominator does not exist.
    ///
    /// Used by exactly two callers, each of which says so in `amal`, because
    /// this is the shape that becomes an indeterminate spinner and it must
    /// never be reached for a stage that could have counted.
    pub fn ballagh_bila_majmu(&self, marhala: MarhalaTilqai, munjaz: u64, amal: impl Into<String>) {
        self.arsil(marhala, munjaz, None, amal);
    }

    /// A stage that finished, reported at its own full count.
    pub fn ikhtim(&self, marhala: MarhalaTilqai, majmu: u64, amal: impl Into<String>) {
        self.arsil(marhala, majmu, Some(majmu), amal);
    }

    fn arsil(
        &self,
        marhala: MarhalaTilqai,
        munjaz: u64,
        majmu: Option<u64>,
        amal: impl Into<String>,
    ) {
        self.mukhbir.ballagh(Taqaddum {
            marhala,
            munjaz,
            majmu,
            amal: amal.into(),
            munfaq: self.munfaq,
            saqf: self.saqf,
        });
    }
}
