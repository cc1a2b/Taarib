//! التوجيه — dispatch: from a Phase 5 identification to a rung, without a match
//! arm per engine anywhere above this crate.
//!
//! Phase 5 answers "which engine is this?" and stops there. [`tabaqa`] answers
//! "which rung does this game land on?" for one engine at a time. Neither
//! answers the question the installer actually asks, which is "here is a
//! directory — what should Taarib do to it?"
//!
//! This module is that join, and it exists so the join happens once. The
//! alternative — the installer holding a `match` over [`AilatMuharrik`] that
//! names five probes and five entry points — is a table that has to be edited
//! every time an engine is added, in a crate that has no other reason to know
//! this crate's module names. That table would be correct on the day it was
//! written and wrong the first time somebody added a sixth engine and forgot it,
//! and *nothing would fail*: the new engine would simply never be probed.
//!
//! ## What "registered" means here
//!
//! [`MAFAHIS`] is the registry, and it is the only one. Every probe in the crate
//! appears in it exactly once, and [`tashkhees`] is the only public way to reach
//! any of them. There is no `pub fn` on the individual probe types that a caller
//! could route around it with — the five `Mifhas` implementations are reachable
//! through the trait, and the trait's methods take a [`SiyaqTabaqa`] that only
//! this module builds from a Phase 5 result. Adding an engine without adding it
//! to [`MAFAHIS`] therefore produces an engine that is never probed, which is
//! visible: [`tashkhees`] returns [`HasilatTashkhees::LaShay`] for a game the
//! detector positively identified, and that combination is a bug report.
//!
//! ## Why more than one probe may match
//!
//! An Electron shell wrapping an RPG Maker MV game is two true statements about
//! one directory, and both probes are right. The wrapper is *not* automatically
//! the answer: patching MV's `data/*.json` through its own loader is more
//! durable than patching the shell around it, because a shell upgrade replaces
//! `app.asar` wholesale and leaves `data/` alone.
//!
//! So [`tashkhees`] returns every verdict it reached rather than one, ordered by
//! [`Hukm::thiqa`], and [`HasilatTashkhees::mukhtar`] states the preference
//! rule in one place: the inner engine wins over the wrapper. A caller that
//! wants the other choice has both verdicts in hand and can say so explicitly,
//! which is different from a dispatcher that silently made the choice for it.
//!
//! ## Refusals are values, not errors
//!
//! A probe that refuses — contradictory evidence, [`KhataNusus::TabaqaMajhula`]
//! — does not stop the others. It is recorded in
//! [`HasilatTashkhees::imtinaat`] and the run continues, because "Ren'Py's
//! evidence is contradictory" is not a reason to stop asking whether this is
//! also an Electron shell. Only a failure to read the directory at all
//! propagates, and that is the same failure for every probe.

use std::cmp::Reverse;
use std::path::Path;

use taarib_muharrik::dalail::nusus::HadafNusus;
use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::electron::MifhasGhilaf;
use crate::gamemaker::MifhasGameMaker;
use crate::khata::KhataNusus;
use crate::renpy::MifhasRenPy;
use crate::rpgmaker::MifhasRpgMaker;
use crate::tabaqa::{Hukm, Mifhas, Rutba, SiyaqTabaqa, hukm};
use crate::vxace::MifhasVxAce;

/// Every probe this crate has, in the order a diagnostics bundle should read.
///
/// Inner engines before the wrapper, matching [`FahisNusus::jamee`]'s order in
/// Phase 5 for exactly the same reason: a person reading the trail wants to know
/// what the game *is* before what it is packaged in.
///
/// Static rather than built per call, and `&'static dyn` rather than an enum,
/// because the only operation anybody performs on this list is iterating it. An
/// enum would buy a match nobody needs and would put the five engine names in a
/// sixth place they have to be kept consistent.
///
/// [`FahisNusus::jamee`]: taarib_muharrik::dalail::nusus::FahisNusus::jamee
pub static MAFAHIS: [&(dyn Mifhas + Sync); 5] = [
    &MifhasRpgMaker,
    &MifhasVxAce,
    &MifhasRenPy,
    &MifhasGameMaker,
    &MifhasGhilaf,
];

/// One probe's outcome, including the ones that declined.
///
/// Modelled with the refusal as a variant rather than as an absent entry so that
/// "Ren'Py was probed and refused" and "Ren'Py was never probed" are different
/// values. Collapsing them is how a diagnostics bundle ends up unable to
/// distinguish an unusual game from a missing registration.
#[derive(Debug, Clone)]
pub enum NatijatMifhas {
    /// The probe applied and reached a verdict.
    Hukm(Hukm),
    /// The probe applied and refused to choose.
    ///
    /// Carries the refusal's own sentence, which names the two readings and
    /// their scores — the thing a person needs in order to say which one is
    /// right for their game.
    Imtina {
        /// Which adapter refused.
        hadaf: HadafNusus,
        /// Why, in the probe's own words.
        sabab: String,
    },
    /// The probe did not apply to this directory.
    LaYantabiq {
        /// Which adapter stood down.
        hadaf: HadafNusus,
    },
}

impl NatijatMifhas {
    /// Which adapter this outcome belongs to.
    #[must_use]
    pub const fn hadaf(&self) -> HadafNusus {
        match self {
            Self::Hukm(hukm) => hukm.hadaf,
            Self::Imtina { hadaf, .. } | Self::LaYantabiq { hadaf } => *hadaf,
        }
    }

    /// The verdict, when there is one.
    #[must_use]
    pub const fn hukm(&self) -> Option<&Hukm> {
        match self {
            Self::Hukm(hukm) => Some(hukm),
            Self::Imtina { .. } | Self::LaYantabiq { .. } => None,
        }
    }

    /// One line for the evidence trail.
    #[must_use]
    pub fn satr(&self) -> String {
        match self {
            Self::Hukm(hukm) => format!(
                "{}: {} (confidence {})",
                hukm.hadaf.ism(),
                hukm.rutba.ism(),
                hukm.thiqa
            ),
            Self::Imtina { hadaf, sabab } => format!("{}: refused — {sabab}", hadaf.ism()),
            Self::LaYantabiq { hadaf } => format!("{}: does not apply", hadaf.ism()),
        }
    }
}

/// Everything the probes concluded about one directory.
#[derive(Debug, Clone)]
pub struct HasilatTashkhees {
    /// Every probe's outcome, in [`MAFAHIS`] order.
    pub natai: Vec<NatijatMifhas>,
    /// The verdicts that were reached, highest confidence first.
    pub ahkam: Vec<Hukm>,
    /// The probes that applied and then refused to choose.
    pub imtinaat: Vec<(HadafNusus, String)>,
}

impl HasilatTashkhees {
    /// Whether nothing in this crate can act on the directory.
    ///
    /// True both when no probe applied and when every probe that applied
    /// refused. The two are different situations and
    /// [`HasilatTashkhees::imtinaat`] tells them apart; this answers the
    /// narrower question the caller has, which is whether to try another adapter
    /// family.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.ahkam.is_empty()
    }

    /// The verdict to act on, and why that one.
    ///
    /// The rule, stated once so no caller reimplements it:
    ///
    /// 1. **An inner engine beats the wrapper.** An Electron shell around an
    ///    RPG Maker MV game is patched as MV, because a shell upgrade replaces
    ///    `app.asar` and leaves `data/` untouched, while the reverse is not true.
    /// 2. **Otherwise the higher confidence wins**, which is the order
    ///    [`HasilatTashkhees::ahkam`] is already in.
    ///
    /// Returns [`None`] when nothing was concluded. It does not fall back to a
    /// refusal-shaped guess: a caller that gets `None` here has a directory this
    /// crate should not touch, and saying so is the whole point of the probe.
    #[must_use]
    pub fn mukhtar(&self) -> Option<&Hukm> {
        self.ahkam
            .iter()
            .find(|hukm| hukm.hadaf != HadafNusus::Ghilaf)
            .or_else(|| self.ahkam.first())
    }

    /// What the chosen rung costs the user, ready to put in front of them.
    ///
    /// Returns the sentence from [`Rutba::wasf`] rather than a rung name,
    /// because "Istila" means nothing to a player and "the text will not be
    /// selectable" means everything. [`None`] when there is nothing to install.
    #[must_use]
    pub fn wasf_lil_mustakhdim(&self) -> Option<&'static str> {
        self.mukhtar().map(|hukm| hukm.rutba.wasf())
    }

    /// Whether the chosen rung takes something away from the player.
    ///
    /// Separate from [`HasilatTashkhees::wasf_lil_mustakhdim`] so an installer
    /// can *require* an acknowledgement rather than merely display a sentence
    /// that scrolls past. A rung that costs nothing needs no ceremony; one that
    /// does should not be installed silently.
    #[must_use]
    pub fn yukallif(&self) -> bool {
        self.mukhtar().is_some_and(|hukm| hukm.rutba.yukallif())
    }

    /// The whole run as lines, for the diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut satr = Vec::with_capacity(self.natai.len().saturating_add(3));
        satr.push(format!("{} probe(s) run", self.natai.len()));
        for natija in &self.natai {
            satr.push(format!("  {}", natija.satr()));
        }
        match self.mukhtar() {
            Some(hukm) => {
                satr.push(format!(
                    "chosen: {} at {} — {}",
                    hukm.hadaf.ism(),
                    hukm.rutba.ism(),
                    hukm.rutba.wasf()
                ));
                if self.ahkam.len() > 1 {
                    satr.push(
                        "more than one adapter applies; the inner engine was preferred over \
                         the wrapper because a shell upgrade replaces the shell and leaves the \
                         game's own data alone"
                            .to_owned(),
                    );
                }
            },
            None => satr.push(
                "nothing in this crate applies to this directory, or every probe that applied \
                 refused to choose"
                    .to_owned(),
            ),
        }
        satr
    }
}

/// Runs every registered probe against one game.
///
/// `aila` is Phase 5's conclusion, carried through rather than re-derived: this
/// module opens no file the probes do not open themselves, and two independent
/// answers to "which engine is this?" would be two chances to disagree.
///
/// Every probe runs. A probe that does not apply costs a [`Mifhas::yantabiq`]
/// call, which is documented as cheap and non-throwing, and running all five
/// unconditionally is what makes the wrapper-plus-inner-engine case visible
/// instead of hidden behind whichever one was checked first.
///
/// # Errors
///
/// [`KhataNusus`] only when a probe cannot read the game's files at all —
/// a broken installation rather than an unusual one. A probe that applies and
/// then refuses to choose is recorded in [`HasilatTashkhees::imtinaat`] and does
/// not stop the run, because one engine's contradictory evidence says nothing
/// about another's.
pub fn tashkhees(
    jidhr: &Path,
    tanfidhi: Option<&Path>,
    aila: AilatMuharrik,
) -> Result<HasilatTashkhees, KhataNusus> {
    let siyaq = SiyaqTabaqa {
        jidhr,
        tanfidhi,
        aila,
    };
    let mut natai = Vec::with_capacity(MAFAHIS.len());
    let mut ahkam = Vec::new();
    let mut imtinaat = Vec::new();

    for mifhas in &MAFAHIS {
        let hadaf = mifhas.hadaf();
        if !mifhas.yantabiq(&siyaq) {
            natai.push(NatijatMifhas::LaYantabiq { hadaf });
            continue;
        }
        let adilla = mifhas.adilla(&siyaq)?;
        match hukm(&siyaq, hadaf, adilla) {
            Ok(natija) => {
                ahkam.push(natija.clone());
                natai.push(NatijatMifhas::Hukm(natija));
            },
            // A refusal to *choose a rung* is data. A failure to *read* is not,
            // and the two are separated here rather than by a caller that would
            // have to know which variants mean which.
            Err(KhataNusus::TabaqaMajhula { sabab }) => {
                imtinaat.push((hadaf, sabab.clone()));
                natai.push(NatijatMifhas::Imtina { hadaf, sabab });
            },
            Err(khata) => return Err(khata),
        }
    }

    ahkam.sort_by_key(|hukm| Reverse(hukm.thiqa));
    Ok(HasilatTashkhees {
        natai,
        ahkam,
        imtinaat,
    })
}

/// The rung a directory lands on, for a caller that wants only the answer.
///
/// The convenience form of [`tashkhees`], and deliberately thin: everything
/// about *why* is discarded, so anything that reports to a user or writes a
/// diagnostics bundle should call [`tashkhees`] instead and keep the evidence.
///
/// # Errors
///
/// As [`tashkhees`], plus [`KhataNusus::TabaqaMajhula`] when no probe reached a
/// verdict — which for this function is a failure, since its whole contract is
/// to produce one.
pub fn rutba(
    jidhr: &Path,
    tanfidhi: Option<&Path>,
    aila: AilatMuharrik,
) -> Result<(HadafNusus, Rutba), KhataNusus> {
    let hasila = tashkhees(jidhr, tanfidhi, aila)?;
    let Some(hukm) = hasila.mukhtar() else {
        return Err(KhataNusus::TabaqaMajhula {
            sabab: format!(
                "no script-engine adapter reached a verdict for {} ({} probe(s) applied and \
                 refused)",
                jidhr.display(),
                hasila.imtinaat.len()
            ),
        });
    };
    Ok((hukm.hadaf, hukm.rutba))
}
