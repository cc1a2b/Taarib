//! المراجعة — where a translation stands, who moved it there, and the one
//! transition that cannot be forged.
//!
//! Modelled here once and completely, because three phases read it: Phase 18's
//! review console drives it, Phase 19's collaboration reconciles two people's
//! versions of it, and Phase 14 refuses to compile a patch out of anything that
//! has not reached the right state.
//!
//! ## A machine translation never becomes approved
//!
//! Not by a code path that forgets to check, not by a bulk action, not by an
//! import, and not by a future edit. The rule is in the type:
//! [`HalatMuraja::Muakkada`] is reachable only through
//! [`SijillMuraja::iaatimad`], which requires a [`ShahadatMuraja`] — and a
//! [`ShahadatMuraja`] has no public constructor. The single way to obtain one is
//! [`ShahadatMuraja::baad_muraja`], which takes the reviewer's identity and the
//! moment they reviewed.
//!
//! This is the same discipline as Phase 11's tier-3 disclosure and Phase 10's
//! write guard: a rule a future edit can break by accident is not a rule. Here
//! the specific accident is a "approve all machine translations" bulk action
//! written by somebody who did not know the rule existed — and with this shape,
//! that action does not compile.
//!
//! ### Why it matters more than it looks
//!
//! An approved string is one a contributor is telling other people is correct.
//! It ships in a published patch under their name. A machine translation that
//! reached approved without a human reading it puts their name on text they
//! never saw, in a language a reviewer of the patch may not read, in front of
//! however many people install it.
//!
//! ## The history is the record, not a log line
//!
//! Every transition is kept: what it moved from, what to, who, when, and why
//! where a reason was given. Phase 19 reconciles two contributors' histories by
//! merging them, so the history has to be data rather than prose.

use crate::musahim::MusahimId;
use serde::{Deserialize, Serialize};

/// Where a translation stands.
///
/// Six states. [`HalatMuraja::TarjamaAaliya`] and [`HalatMuraja::Musawwada`]
/// look similar and are deliberately separate: one is text no human has read
/// and the other is a human's own draft, and every downstream decision — what
/// to show a reviewer first, what a bulk operation may touch, what Phase 14 will
/// compile — treats them differently.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HalatMuraja {
    /// No translation yet.
    #[default]
    LamTutarjam,
    /// A machine produced it and no human has read it.
    TarjamaAaliya,
    /// A human wrote or edited it and has not marked it finished.
    Musawwada,
    /// Flagged as wanting a second pair of eyes.
    LilMuraja,
    /// A human read it and accepted it.
    ///
    /// Reachable only through [`SijillMuraja::iaatimad`]. See this module's
    /// header.
    Muakkada,
    /// A human read it and rejected it.
    ///
    /// The translation is kept rather than cleared, because the next translator
    /// wants to see what was wrong with it, and because a rejection with no
    /// artefact is indistinguishable from never having tried.
    Marfuda,
}

impl HalatMuraja {
    /// Whether this state counts as translated for coverage.
    #[must_use]
    pub const fn mutarjam(self) -> bool {
        !matches!(self, Self::LamTutarjam)
    }

    /// Whether a human has read this translation.
    ///
    /// The signal Phase 14 consults before compiling a patch that claims human
    /// review, and the one the review console sorts by.
    #[must_use]
    pub const fn qaraaha_insan(self) -> bool {
        matches!(self, Self::Musawwada | Self::Muakkada | Self::Marfuda)
    }

    /// Whether this string may ship.
    #[must_use]
    pub const fn qabila_lil_nashr(self) -> bool {
        matches!(self, Self::Muakkada)
    }

    /// The label the console shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::LamTutarjam => "لم تُترجم",
            Self::TarjamaAaliya => "ترجمة آلية",
            Self::Musawwada => "مسوّدة",
            Self::LilMuraja => "للمراجعة",
            Self::Muakkada => "مؤكَّدة",
            Self::Marfuda => "مرفوضة",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::LamTutarjam => "untranslated",
            Self::TarjamaAaliya => "machine-translated",
            Self::Musawwada => "draft",
            Self::LilMuraja => "needs review",
            Self::Muakkada => "approved",
            Self::Marfuda => "rejected",
        }
    }
}

/// Proof that a human read a translation.
///
/// **No public constructor.** The only way to obtain one is
/// [`ShahadatMuraja::baad_muraja`], which a review action calls with the
/// reviewer's identity and the moment of the review. A caller that wants to
/// approve a machine translation without a human reading it has nothing it can
/// put in the argument position.
///
/// Deliberately not [`Copy`] and not [`Clone`]: one review attests to one
/// string. A copyable attestation would let a single human review approve a
/// thousand machine translations, which is precisely the bulk action this type
/// exists to make impossible.
///
/// Deliberately **not [`Deserialize`]** either, and this is the subtler hole of
/// the two. A `Deserialize` impl *is* a public constructor: anything that can
/// name the type could mint an attestation out of two JSON fields, and the whole
/// guarantee would come down to nobody thinking of it. The attestation is
/// transient by design — it is consumed by the transition it authorises, and
/// what persists is the [`IntiqalMuraja`] in the history, which records the
/// reviewer and the moment without being a token anyone can forge later.
///
/// [`Deserialize`]: serde::Deserialize
#[derive(Debug)]
pub struct ShahadatMuraja {
    murajii: MusahimId,
    lahza: u64,
}

impl ShahadatMuraja {
    /// Records that a human read this translation.
    ///
    /// `lahza` is seconds since the Unix epoch, supplied by the caller rather
    /// than read here — the same discipline every other module in this product
    /// follows, so that a stored history is reproducible and a test does not
    /// depend on a clock.
    #[must_use]
    pub const fn baad_muraja(murajii: MusahimId, lahza: u64) -> Self {
        Self { murajii, lahza }
    }

    /// Who reviewed.
    #[must_use]
    pub const fn murajii(&self) -> &MusahimId {
        &self.murajii
    }

    /// When, in seconds since the Unix epoch.
    #[must_use]
    pub const fn lahza(&self) -> u64 {
        self.lahza
    }
}

/// One transition, kept forever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct IntiqalMuraja {
    /// Where it was.
    pub min: HalatMuraja,
    /// Where it went.
    pub ila: HalatMuraja,
    /// Who moved it, or [`None`] when a machine did.
    ///
    /// The absence is the record: a transition with no author is one no person
    /// made, which is exactly what distinguishes a machine translation landing
    /// in [`HalatMuraja::TarjamaAaliya`] from a human drafting one.
    pub musahim: Option<MusahimId>,
    /// When, in seconds since the Unix epoch.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub lahza: u64,
    /// Why, where a reason was given.
    pub sabab: Option<String>,
}

/// A string's review state and the whole history behind it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct SijillMuraja {
    hala: HalatMuraja,
    tareekh: Vec<IntiqalMuraja>,
    mujammad: bool,
}

impl SijillMuraja {
    /// A fresh record, untranslated and with nothing behind it.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            hala: HalatMuraja::LamTutarjam,
            tareekh: Vec::new(),
            mujammad: false,
        }
    }

    /// Where the string stands.
    #[must_use]
    pub const fn hala(&self) -> HalatMuraja {
        self.hala
    }

    /// Every transition, oldest first.
    #[must_use]
    pub fn tareekh(&self) -> &[IntiqalMuraja] {
        &self.tareekh
    }

    /// Records that a machine translated this string.
    ///
    /// Lands in [`HalatMuraja::TarjamaAaliya`] and records no author, because
    /// no person made this transition.
    ///
    /// **This method cannot reach [`HalatMuraja::Muakkada`]** and that is not an
    /// oversight — it is the enforcement. The approval transition is a different
    /// method taking a different argument that a machine cannot construct.
    pub fn sajjil_aali(&mut self, lahza: u64) {
        self.intaqil(HalatMuraja::TarjamaAaliya, None, lahza, None);
    }

    /// Records a human's own draft or edit.
    pub fn sajjil_musawwada(&mut self, musahim: MusahimId, lahza: u64) {
        self.intaqil(HalatMuraja::Musawwada, Some(musahim), lahza, None);
    }

    /// Flags the string as wanting review.
    pub fn tlub_muraja(&mut self, musahim: Option<MusahimId>, lahza: u64, sabab: Option<String>) {
        self.intaqil(HalatMuraja::LilMuraja, musahim, lahza, sabab);
    }

    /// Approves the string.
    ///
    /// **The only path to [`HalatMuraja::Muakkada`] in this product.** It
    /// consumes a [`ShahadatMuraja`], which only a human review can construct,
    /// so there is no sequence of calls that approves a machine translation
    /// nobody read.
    ///
    /// The attestation is consumed rather than borrowed: one review approves one
    /// string, and a borrowed attestation could be handed to a loop.
    pub fn iaatimad(&mut self, shahada: ShahadatMuraja, sabab: Option<String>) {
        let ShahadatMuraja { murajii, lahza } = shahada;
        self.intaqil(HalatMuraja::Muakkada, Some(murajii), lahza, sabab);
    }

    /// Rejects the string.
    ///
    /// Also requires an attestation: a rejection is a human judgement exactly as
    /// an approval is, and one that could be made without a reviewer would let a
    /// failed automated check bury a good translation.
    pub fn irfud(&mut self, shahada: ShahadatMuraja, sabab: Option<String>) {
        let ShahadatMuraja { murajii, lahza } = shahada;
        self.intaqil(HalatMuraja::Marfuda, Some(murajii), lahza, sabab);
    }

    /// Returns the string to untranslated, discarding its translation.
    pub fn imsah(&mut self, musahim: Option<MusahimId>, lahza: u64) {
        self.intaqil(HalatMuraja::LamTutarjam, musahim, lahza, None);
    }

    /// Whether bulk operations and machine translation must keep out.
    ///
    /// A **flag rather than a seventh state**, and the distinction is the
    /// design. Freezing answers "may anything change this?", which is
    /// orthogonal to "where does this stand?" — a contributor freezes an
    /// approved string so a re-run cannot overwrite it, and freezes a draft
    /// they are mid-way through for the same reason. Modelling it as a state
    /// would have forced a choice between recording that it is frozen and
    /// recording that it was approved, and the pair is exactly what a reviewer
    /// needs to see.
    #[must_use]
    pub const fn mujammad(&self) -> bool {
        self.mujammad
    }

    /// Freezes or unfreezes the string.
    ///
    /// Records nothing in the history: a freeze is not a transition of the
    /// translation's own state, and putting it in the timeline would bury the
    /// transitions that are.
    pub const fn jammid(&mut self, mujammad: bool) {
        self.mujammad = mujammad;
    }

    /// Whether a bulk run may write to this string.
    ///
    /// The single question the batch layer asks, so that "frozen" and "already
    /// has a human's translation" are one check rather than two that a caller
    /// could get half-right.
    #[must_use]
    pub const fn qabil_lil_kitaba_aliyan(&self) -> bool {
        !self.mujammad && !self.hala.qaraaha_insan()
    }

    /// Whether a machine produced the current translation and no human has
    /// touched it since.
    ///
    /// What the [`AlamJawda`] machine-only flag is computed from, and what the
    /// review console filters by to find the work that still needs a person.
    ///
    /// [`AlamJawda`]: crate::nass::AlamJawda
    #[must_use]
    pub const fn aali_faqat(&self) -> bool {
        matches!(self.hala, HalatMuraja::TarjamaAaliya)
    }

    /// Applies a transition and records it.
    ///
    /// Private, and the single place `hala` is ever assigned. Every public
    /// method above funnels through it, so a transition that reached the state
    /// without reaching the history is not expressible.
    fn intaqil(
        &mut self,
        ila: HalatMuraja,
        musahim: Option<MusahimId>,
        lahza: u64,
        sabab: Option<String>,
    ) {
        if self.hala == ila && sabab.is_none() {
            // A no-op transition is not history. Recording it would fill a
            // reviewer's timeline with entries that say nothing happened.
            return;
        }
        self.tareekh.push(IntiqalMuraja {
            min: self.hala,
            ila,
            musahim,
            lahza,
            sabab,
        });
        self.hala = ila;
    }
}
