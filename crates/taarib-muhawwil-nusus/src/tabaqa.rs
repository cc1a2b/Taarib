//! طبقة — which rung applies, and why. The first thing this phase decides.
//!
//! The tempting assumption about these five engines is that none of them can
//! shape Arabic, because none of them was designed to. That assumption is
//! wrong for at least two of them and probably three, and acting on it would be
//! the worst kind of bug: a takeover that *works*, produces plausible text, and
//! is a regression against what the engine was already doing correctly.
//!
//! An Electron game draws through a browser engine. Chromium shapes Arabic with
//! `HarfBuzz` and applies the full bidirectional algorithm, and it does it better
//! than Taarib will, because it has been doing it for fifteen years against
//! every font on the web. A Ren'Py 8 game ships `HarfBuzz` and `FriBidi` in its own
//! `librenpy` and uses them. An RPG Maker MZ game draws through a canvas
//! rasterizer that is *also* the browser's, so `fillText` on an Arabic string
//! already joins. Taking any of those over would mean replacing a correct
//! implementation with a newer one, losing text selection, losing search,
//! losing the accessibility tree, and gaining nothing.
//!
//! So this module runs before any of the five adapters and answers one
//! question per game, from the actual shipped files rather than from the engine's
//! reputation.
//!
//! ## The three rungs
//!
//! **[`Rutba::Idad`] — configuration only.** The engine shapes correctly. Supply
//! the font, set the direction, install the translation through the engine's own
//! localization mechanism, and stop. Text stays real text: selectable,
//! searchable, copyable, and legible to a screen reader. This is the best
//! outcome and it is available more often than one would expect.
//!
//! **[`Rutba::Tasheeh`] — corrected shaping.** The engine shapes but resolves
//! direction, wrapping or alignment wrongly. Correct those specific behaviours
//! and leave the shaping alone. The most common real case, because "supports
//! Arabic" and "supports right-to-left layout" are different features and
//! engines frequently ship the first without the second.
//!
//! **[`Rutba::Istila`] — full takeover.** The engine cannot shape. Lay out
//! through `saff`, transport shaped glyph identifiers, draw directly. This is
//! the rung that costs the user something real, and [`Rutba::yukallif`] says so
//! before anything is installed.
//!
//! ## Why the verdict is evidence and not a lookup table
//!
//! A version number is not the answer. Ren'Py 7.4 shapes and Ren'Py 7.3 does
//! not, but a 7.4 built without the optional libraries does not either; an
//! Electron game whose text is drawn onto a `<canvas>` gets no shaping from the
//! browser at all, because `fillText` on a canvas bypasses the layout engine
//! that would have shaped it. So [`Hukm`] carries the evidence that produced it,
//! every piece of it reaches the diagnostics bundle, and a verdict nobody can
//! explain is a verdict this module refuses to issue —
//! [`KhataNusus::TabaqaMajhula`] rather than a guess.
//!
//! Guessing has an asymmetric cost and that asymmetry is why the refusal
//! exists. Guessing *low* — assuming shaping when there is none — ships unjoined
//! letters, which is the exact failure this product was built to prevent.
//! Guessing *high* — taking over an engine that was already correct — ships text
//! that looks right and has quietly stopped being text.

use std::fmt;
use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::nusus::HadafNusus;
use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::khata::KhataNusus;

/// Which rung of the ladder a game lands on.
///
/// Ordered by how much of the engine Taarib replaces, least first. The numbers
/// are stable because they reach the capability report, the diagnostics bundle
/// and the registry's per-game records, where a value written by an older build
/// has to keep meaning what it meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rutba {
    /// The engine shapes correctly. Font, direction, translation, stop.
    Idad = 1,

    /// The engine shapes but lays out right-to-left text wrongly.
    Tasheeh = 2,

    /// The engine cannot shape. Taarib lays out and draws.
    Istila = 3,
}

impl Rutba {
    /// The name for a log line and the capability report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Idad => "configuration",
            Self::Tasheeh => "corrected shaping",
            Self::Istila => "full takeover",
        }
    }

    /// Whether this rung costs the user a capability they had before.
    ///
    /// Only the takeover does, and only because a transported glyph run is no
    /// longer text to anything that inspects it. Every caller that installs is
    /// required to surface [`Rutba::wasf`] when this is true.
    #[must_use]
    pub const fn yukallif(self) -> bool {
        matches!(self, Self::Istila)
    }

    /// What a player should be told about this rung before they install.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Idad => {
                "This game's engine shapes Arabic correctly on its own. Taarib supplies the \
                 font and the translation and changes nothing else, so the text behaves \
                 exactly like the game's own — selectable, searchable and copyable."
            },
            Self::Tasheeh => {
                "This game's engine shapes Arabic but lays it out left to right. Taarib \
                 corrects the direction, wrapping and alignment and leaves the shaping to the \
                 engine, so the text stays real text."
            },
            Self::Istila => {
                "This game's engine cannot shape Arabic, so Taarib draws the text itself. It \
                 will look correct and it will NOT be selectable, searchable or copyable \
                 inside the game."
            },
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Idad => {
                "محرّك هذه اللعبة يشكّل العربية تشكيلًا صحيحًا بنفسه. يزوّده تعريب بالخطّ \
                 والترجمة ولا يغيّر شيئًا آخر، فيبقى النصّ نصًّا حقيقيًا: يُحدَّد ويُبحث فيه \
                 ويُنسخ."
            },
            Self::Tasheeh => {
                "محرّك هذه اللعبة يشكّل العربية لكنه يخطئ في اتجاهها. يصحّح تعريب الاتجاه \
                 والالتفاف والمحاذاة ويترك التشكيل للمحرّك، فيبقى النصّ نصًّا حقيقيًا."
            },
            Self::Istila => {
                "محرّك هذه اللعبة لا يستطيع تشكيل العربية، فيرسم تعريب النصّ بنفسه. سيظهر \
                 صحيحًا، ولن يكون قابلًا للتحديد أو البحث أو النسخ داخل اللعبة."
            },
        }
    }
}

impl fmt::Display for Rutba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ism())
    }
}

/// One thing the probe observed, and what it implies.
///
/// Kept rather than collapsed into a boolean, because the verdict has to be
/// explainable. "Rung three" tells a translator nothing; "Ren'Py 7.2, and
/// `libharfbuzz` is not among the shipped shared libraries" tells them why, and
/// tells the owner what changed when a later version of the same game lands on
/// rung one.
#[derive(Debug, Clone)]
pub struct Dalil {
    /// What was looked at, as a path or a short name.
    pub masdar: String,
    /// What was found there.
    pub wujid: String,
    /// Which way it points, or [`None`] when it is context rather than evidence.
    pub yushir: Option<Rutba>,
    /// How much this observation is worth, 0..=100.
    ///
    /// A shipped shaping library found by name is near-conclusive; a version
    /// number alone is suggestive, because a build can omit an optional
    /// dependency. The weights are what make a contradiction visible instead of
    /// letting the last check win.
    pub thiqa: u8,
}

impl Dalil {
    /// Records an observation.
    #[must_use]
    pub fn jadeed(
        masdar: impl Into<String>,
        wujid: impl Into<String>,
        yushir: Option<Rutba>,
        thiqa: u8,
    ) -> Self {
        Self {
            masdar: masdar.into(),
            wujid: wujid.into(),
            yushir,
            thiqa: if thiqa > 100 { 100 } else { thiqa },
        }
    }

    /// Records context that points nowhere on its own.
    #[must_use]
    pub fn siyaq(masdar: impl Into<String>, wujid: impl Into<String>) -> Self {
        Self::jadeed(masdar, wujid, None, 0)
    }
}

/// The probe's verdict for one game.
#[derive(Debug, Clone)]
pub struct Hukm {
    /// The game's root.
    pub jidhr: PathBuf,
    /// Which engine family this is.
    pub aila: AilatMuharrik,
    /// Which of this crate's five adapters applies.
    pub hadaf: HadafNusus,
    /// The rung.
    pub rutba: Rutba,
    /// How sure the probe is, 0..=100.
    pub thiqa: u8,
    /// Everything it looked at.
    pub adilla: Vec<Dalil>,
}

impl Hukm {
    /// Whether the verdict is strong enough to act on without asking.
    ///
    /// Sixty, which is the point at which one near-conclusive observation or two
    /// agreeing suggestive ones have been made. Below it the caller shows the
    /// evidence and lets a person decide, because an adapter that acts on a
    /// coin-flip is an adapter that will take over a correct engine one time in
    /// two.
    #[must_use]
    pub const fn qatii(&self) -> bool {
        self.thiqa >= 60
    }

    /// The verdict as lines for the log and the diagnostics bundle.
    ///
    /// Allocates, and is meant to: it runs once per game, when a capability
    /// report is built or a bundle is written.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.adilla.len().saturating_add(2));
        sutur.push(format!(
            "{}: rung {} ({}), confidence {}",
            self.hadaf.ism(),
            self.rutba as u8,
            self.rutba.ism(),
            self.thiqa
        ));
        for dalil in &self.adilla {
            sutur.push(match dalil.yushir {
                Some(rutba) => format!(
                    "  {} = {} -> rung {} (weight {})",
                    dalil.masdar, dalil.wujid, rutba as u8, dalil.thiqa
                ),
                None => format!("  {} = {}", dalil.masdar, dalil.wujid),
            });
        }
        sutur.push(format!("  {}", self.rutba.wasf()));
        sutur
    }

    /// Whether any observation contradicted the verdict.
    ///
    /// Reported rather than hidden. Contradictory evidence usually means a game
    /// was built unusually — a Ren'Py with its optional libraries stripped, an
    /// Electron game that draws to a canvas — and those are exactly the games
    /// where a translator needs to know why the adapter chose what it chose.
    #[must_use]
    pub fn tanaqud(&self) -> bool {
        self.adilla
            .iter()
            .any(|dalil| dalil.yushir.is_some_and(|rutba| rutba != self.rutba) && dalil.thiqa > 0)
    }
}

/// What a probe needs in order to look.
///
/// Supplied rather than discovered so this module opens no file it was not
/// pointed at, and so a caller that has already read the game's layout — which
/// Phase 5's detector has — does not pay to walk it twice.
#[derive(Debug, Clone, Copy)]
pub struct SiyaqTabaqa<'a> {
    /// The game's root directory.
    pub jidhr: &'a Path,
    /// The game's executable, when one was identified.
    pub tanfidhi: Option<&'a Path>,
    /// The engine family Phase 5 concluded.
    pub aila: AilatMuharrik,
}

/// One engine's probe.
///
/// A trait rather than a match, because each engine's evidence is entirely
/// different — a shared library's presence, a JSON field, a chunk's contents —
/// and a single function that knew all five would be a function nobody could
/// change safely.
pub trait Mifhas {
    /// Which adapter this probe speaks for.
    fn hadaf(&self) -> HadafNusus;

    /// Whether this probe applies to the game in front of it.
    ///
    /// Cheap, and must not throw. Called for all five probes on every game.
    fn yantabiq(&self, siyaq: &SiyaqTabaqa<'_>) -> bool;

    /// Gathers evidence.
    ///
    /// # Errors
    ///
    /// Only when the game's files cannot be read at all. An engine whose
    /// shaping support is simply unclear returns evidence saying so and lets
    /// [`hukm`] refuse, rather than erroring — the difference matters, because
    /// one is a broken installation and the other is an unusual build.
    fn adilla(&self, siyaq: &SiyaqTabaqa<'_>) -> Result<Vec<Dalil>, KhataNusus>;
}

/// Weighs evidence into a verdict.
///
/// The rule is deliberately simple and deliberately conservative: sum the
/// weights per rung, take the highest, and refuse when nothing reaches the
/// floor. It is not a scoring system anybody should tune — a probe that needs
/// clever arithmetic to reach the right answer is a probe that is not looking at
/// the right evidence.
///
/// # Errors
///
/// [`KhataNusus::TabaqaMajhula`] when no observation pointed anywhere, or when
/// the best two rungs are within [`FARQ_HASIM`] of each other. A tie is not
/// broken by preference: the two outcomes are "replace a correct engine" and
/// "ship unjoined Arabic", and choosing between them by tiebreak would be
/// choosing arbitrarily between two bad results.
pub fn hukm(
    siyaq: &SiyaqTabaqa<'_>,
    hadaf: HadafNusus,
    adilla: Vec<Dalil>,
) -> Result<Hukm, KhataNusus> {
    let mut mizan = [0u32; 4];
    for dalil in &adilla {
        if let Some(rutba) = dalil.yushir {
            let khana = rutba as usize;
            if let Some(majmu) = mizan.get_mut(khana) {
                *majmu = majmu.saturating_add(u32::from(dalil.thiqa));
            }
        }
    }

    let mut awwal = (Rutba::Idad, 0u32);
    let mut thani = 0u32;
    for (khana, majmu) in mizan.iter().enumerate() {
        let Some(rutba) = rutba_min_khana(khana) else {
            continue;
        };
        if *majmu > awwal.1 {
            thani = awwal.1;
            awwal = (rutba, *majmu);
        } else if *majmu > thani {
            thani = *majmu;
        }
    }

    if awwal.1 == 0 {
        return Err(KhataNusus::TabaqaMajhula {
            sabab: format!(
                "nothing observed about {} pointed at any rung ({} observation(s) made)",
                hadaf.ism(),
                adilla.len()
            ),
        });
    }
    if awwal.1.saturating_sub(thani) < FARQ_HASIM && thani > 0 {
        return Err(KhataNusus::TabaqaMajhula {
            sabab: format!(
                "the evidence about {} is contradictory: the two leading readings score {} \
                 and {}, which is too close to act on. Choosing would mean picking \
                 arbitrarily between taking over an engine that may already be correct and \
                 shipping Arabic that may not join.",
                hadaf.ism(),
                awwal.1,
                thani
            ),
        });
    }

    let thiqa = u8::try_from(awwal.1.min(100)).unwrap_or(100);
    Ok(Hukm {
        jidhr: siyaq.jidhr.to_path_buf(),
        aila: siyaq.aila,
        hadaf,
        rutba: awwal.0,
        thiqa,
        adilla,
    })
}

/// How far the leading rung must be ahead of the runner-up to be acted on.
///
/// Twenty-five. Below that the two readings are close enough that whichever one
/// wins is an accident of which files happened to be present, and the refusal is
/// the correct output. Deliberately not tunable per engine: a threshold that
/// varies per caller is a threshold somebody lowers to make a game work.
pub const FARQ_HASIM: u32 = 25;

/// The rung a weight slot belongs to.
const fn rutba_min_khana(khana: usize) -> Option<Rutba> {
    match khana {
        1 => Some(Rutba::Idad),
        2 => Some(Rutba::Tasheeh),
        3 => Some(Rutba::Istila),
        _ => None,
    }
}
