//! البدء — starting an Arabization from the card, and what the user is told
//! before any work begins.
//!
//! A user looking at a game with no patch must be able to start translating it
//! without leaving the card. This module is the state that entry point can be
//! in, and the honest verdict it shows first.
//!
//! ## The verdict comes before the work, always
//!
//! Choosing "translate this game" does not start an extraction. It runs engine
//! identification and the capability probe, and shows what came back: which
//! tier applies, what the result will actually look like, what will not work,
//! and roughly how many strings are involved. Only then is there a button that
//! begins.
//!
//! Ordering it the other way — extract first, explain after — would be faster
//! to build and would produce a product that spends four minutes of somebody's
//! time before telling them their game is tier 3 and the text will not be
//! selectable. The probe is seconds; the extraction is minutes. Spending the
//! seconds first is not caution, it is arithmetic.
//!
//! ## The entry says what is actually true
//!
//! There are five different situations behind one menu item, and a product that
//! labelled all five "Translate this game" would be lying in four of them:
//!
//! | situation | the entry says |
//! | --- | --- |
//! | nothing exists | begin |
//! | a draft project exists | continue, and show its coverage |
//! | a published patch exists | start a new translation, and offer to seed from it |
//! | strings cannot be extracted statically | play once with capture running |
//! | the safety layer refuses the game | nothing — the entry is absent, with the reason shown |
//!
//! [`HalatBadaa`] is that distinction, and it is an enum rather than a set of
//! booleans so that a caller cannot render "continue" and "seed from existing"
//! at once, which is a state that does not exist.
//!
//! ## Demand is shown, not acted on
//!
//! When other people have asked for this game on the request board, the count
//! appears here. It is information — somebody deciding whether to spend a
//! weekend on a translation deserves to know eleven people are waiting — and it
//! is never a prompt, a nag, or a sorted-to-the-top suggestion.

use serde::{Deserialize, Serialize};

use crate::muharrik::{Tabaqa, TaqreerImkaniyat};
use crate::ruqaa::{RuqaaId, RukhsaRuqaa};
use crate::taghtiya::Taghtiya;

/// Which of the five situations this game's entry point is in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum HalatBadaa {
    /// Nothing exists for this game and extraction will work.
    Ibda,
    /// A project is already open for it.
    ///
    /// The entry says *continue*, not *begin*, and shows the coverage — because
    /// a user who started this two weeks ago and forgot needs to be reminded
    /// rather than offered a fresh start that would silently shadow their work.
    Istikmal {
        /// How much of the game is already translated.
        taghtiya: Taghtiya,
        /// When it was last worked on, RFC 3339.
        akhir_amal: String,
    },
    /// A published patch exists and the user wants their own anyway.
    ///
    /// A legitimate thing to want: the published one may be machine-translated,
    /// or may be for a dialect the user does not write. The entry says *start a
    /// new translation* rather than pretending nothing is there.
    BadaaJadeed {
        /// The patch that already exists.
        ruqaa: RuqaaId,
        /// Its title.
        unwan: String,
        /// Its licence, which decides whether seeding is offered.
        rukhsa: RukhsaRuqaa,
        /// Whether the licence permits starting from it.
        ///
        /// Computed here rather than left to the interface, so that "may I use
        /// this as a starting point" has exactly one answer in the product.
        yasmah_bil_bidhra: bool,
    },
    /// The strings cannot be extracted from the files and must be captured while
    /// the game runs.
    IltiqatWaqtTashghil {
        /// Why static extraction will not work, in Arabic.
        sabab_arabi: String,
        /// The same in English.
        sabab_injilizi: String,
    },
    /// The safety layer refuses this game.
    ///
    /// The entry is not shown at all, and the reason is. A disabled menu item
    /// with no explanation is worse than no menu item: it invites the user to
    /// keep looking for the way to enable it.
    Marfud {
        /// Why, in Arabic.
        sabab_arabi: String,
        /// The same in English.
        sabab_injilizi: String,
    },
}

impl HalatBadaa {
    /// Whether the entry appears in the card's context menu at all.
    #[must_use]
    pub const fn zahir(&self) -> bool {
        !matches!(self, Self::Marfud { .. })
    }

    /// The label the entry carries, in Arabic.
    ///
    /// Five situations, five labels. Written here rather than in the interface
    /// because the card's menu, the detail screen and the command palette all
    /// show this entry, and three call sites picking a label independently is
    /// three chances for them to disagree about what the button does.
    #[must_use]
    pub const fn unwan_arabi(&self) -> &'static str {
        match self {
            Self::Ibda => "ابدأ تعريب هذه اللعبة",
            Self::Istikmal { .. } => "أكمل تعريب هذه اللعبة",
            Self::BadaaJadeed { .. } => "ابدأ تعريبًا جديدًا",
            Self::IltiqatWaqtTashghil { .. } => "التقط النصوص أثناء اللعب",
            Self::Marfud { .. } => "",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn unwan_injilizi(&self) -> &'static str {
        match self {
            Self::Ibda => "Translate this game",
            Self::Istikmal { .. } => "Continue translating",
            Self::BadaaJadeed { .. } => "Start a new translation",
            Self::IltiqatWaqtTashghil { .. } => "Capture text while playing",
            Self::Marfud { .. } => "",
        }
    }

    /// Whether choosing this entry opens an existing project rather than making
    /// one.
    #[must_use]
    pub const fn yaftah_mawjud(&self) -> bool {
        matches!(self, Self::Istikmal { .. })
    }
}

/// The honest verdict, shown before any extraction starts.
///
/// Assembled from the capability probe rather than restating it: every field
/// here that could have come from [`TaqreerImkaniyat`] does, so a probe that
/// improves improves this too and there is no second opinion to keep in sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct HukmBadaa {
    /// The capability report, in full.
    pub taqreer: TaqreerImkaniyat,
    /// Which situation the entry point is in.
    pub hala: HalatBadaa,
    /// Roughly how many strings are involved.
    ///
    /// [`None`] when the tier is one whose strings are not countable in
    /// advance — a tier-3 overlay reads text off the screen and there is no set
    /// to count. Showing a fabricated number there would be worse than showing
    /// none, because a user would plan around it.
    pub adad_nusus_taqribi: Option<u32>,
    /// How many people have asked for this game on the request board.
    ///
    /// Zero when nobody has. Information, never a prompt.
    pub adad_talabat: u32,
}

impl HukmBadaa {
    /// The sentence the user reads before deciding, in Arabic.
    ///
    /// Assembled from the tier's own description, the probe's reason, and the
    /// string count. Deliberately not a summary the interface composes: three
    /// screens showing this verdict must show the same words, and a verdict
    /// that reads differently in two places is a verdict the user stops
    /// trusting.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let mut ajza = vec![self.taqreer.sabab_arabi.clone()];

        if let Some(adad) = self.adad_nusus_taqribi {
            ajza.push(format!("تقديريًا {adad} نصًّا في هذه اللعبة."));
        }

        // Every limitation, named. Not summarised as "some limitations apply",
        // which is the phrasing that lets a product avoid saying the thing the
        // user needed to hear.
        if !self.taqreer.hudud.is_empty() {
            ajza.push("ما لن يعمل:".to_owned());
        }

        if self.adad_talabat > 0 {
            ajza.push(format!("طلب {} شخصًا تعريب هذه اللعبة.", self.adad_talabat));
        }

        ajza.join(" ")
    }

    /// Whether this game's result will cost the user something they had before.
    ///
    /// True for the tier that transports glyphs, where the text stops being
    /// searchable, selectable and copyable. Surfaced as its own question rather
    /// than left inside the description, so the interface can *require* an
    /// acknowledgement instead of printing a sentence that scrolls past.
    #[must_use]
    pub const fn yukallif(&self) -> bool {
        matches!(self.taqreer.tabaqa, Tabaqa::TarjamaFawqiya)
    }

    /// Whether there is anything to start at all.
    #[must_use]
    pub const fn qabil_lil_badaa(&self) -> bool {
        !self.taqreer.marfuda && self.hala.zahir()
    }
}

/// Whether a licence permits starting a new translation from an existing one.
///
/// Delegates to [`RukhsaRuqaa::yasmah_bil_ishtiqaq`] rather than deciding
/// anything, and exists so that this module's readers find the question
/// answered where they look for it. The distinction that matters is one level
/// down: permission to *mirror* a patch and permission to *build on* it are
/// different grants, and the three permissive licences answering both the same
/// way is what makes them easy to confuse.
///
/// When it returns false the entry still offers a fresh start, with the
/// existing patch named but not used — rather than quietly omitting the option
/// and leaving the user to wonder why this game behaves differently from the
/// last one.
#[must_use]
pub const fn yasmah_bil_bidhra(rukhsa: &RukhsaRuqaa) -> bool {
    rukhsa.yasmah_bil_ishtiqaq()
}
