//! التوجيه — which extractor runs for which engine, decided once.
//!
//! Phase 5 identifies the engine. This module turns that identification into a
//! call, and it exists so that the turning happens in one place.
//!
//! The alternative — a `match` on [`AilatMuharrik`] wherever extraction is
//! started — is a table that has to be edited every time an engine is added, in
//! a crate that has no other reason to know this one's module names. It would
//! be correct the day it was written and wrong the first time somebody added an
//! engine and missed a copy of it, and *nothing would fail*: the new engine
//! would simply never be extracted, and the user would be told their game has
//! no strings.
//!
//! ## Every engine answers, including the ones that cannot be extracted
//!
//! [`Mustakhrij::li_aila`] returns a value for all eleven families. Ten
//! extract; the unidentified one does not, and it says why in words the user
//! reads. Returning [`None`] for an unsupported engine would push the explanation to
//! the caller, and a caller with no explanation writes "extraction failed",
//! which is both unhelpful and false — nothing failed, the engine is one this
//! build has no reader for and the overlay is the answer.
//!
//! ## Capture is not an engine
//!
//! Runtime capture applies to *every* engine, as a supplement where extraction
//! works and as the whole answer where it does not. It is therefore not a
//! variant here; [`Mustakhrij::yastafeed_min_iltiqat`] says whether it is worth
//! offering, which is true for all eleven and stated rather than assumed.

use std::path::Path;

use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::jadwal::JadwalNusus;
use crate::khata::KhataIstikhraj;
use crate::rafd::TaqreerRafd;

/// Which extractor handles an engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mustakhrij {
    /// Unity: serialized files, bundles, and the localization packages.
    Unity,
    /// Unreal: `.locres`, `.locmeta`, and `StringTable` assets.
    Unreal,
    /// Godot: translation resources and scene text properties.
    Godot,
    /// One of the four script engines, which share an entry point because they
    /// share a crate and differ only in which reader it dispatches to.
    Nusus,
    /// Capcom's `BIO4`: the `DICT` dictionaries under the game's data directory.
    Qamus,
    /// No static extractor exists for this engine.
    LaShay {
        /// Why, in Arabic.
        sabab_arabi: &'static str,
        /// The same in English.
        sabab_injilizi: &'static str,
    },
}

impl Mustakhrij {
    /// The extractor for an engine family.
    ///
    /// Total over [`AilatMuharrik`] on purpose: a new variant added to the
    /// vocabulary makes this function stop compiling, which is the point. A
    /// wildcard arm here would let a new engine silently fall into "no
    /// extractor" and ship that way.
    #[must_use]
    pub const fn li_aila(aila: AilatMuharrik) -> Self {
        match aila {
            AilatMuharrik::Unity => Self::Unity,
            AilatMuharrik::Unreal => Self::Unreal,
            AilatMuharrik::Godot => Self::Godot,
            AilatMuharrik::RpgMakerMv
            | AilatMuharrik::RpgMakerMz
            | AilatMuharrik::RpgMakerVxAce
            | AilatMuharrik::Renpy
            | AilatMuharrik::GameMaker
            | AilatMuharrik::Electron => Self::Nusus,
            AilatMuharrik::Bio4 => Self::Qamus,
            // Six in-house engines this build can name and cannot open. The
            // refusal is deliberately worded differently from the unknown
            // family's below: the engine *was* identified, so telling the user
            // it was not would be false, and what is actually missing is a
            // reader for the containers it keeps its text in.
            AilatMuharrik::Frostbite
            | AilatMuharrik::BlackSpace
            | AilatMuharrik::Alchemy
            | AilatMuharrik::Dantelion
            | AilatMuharrik::Rage
            | AilatMuharrik::Snowdrop => Self::LaShay {
                sabab_arabi: "تعرَّف تعريب على محرّك هذه اللعبة، لكنه محرّك داخلي يحفظ نصوصه \
                              في صيغ لا يقرؤها أي مستخرج في هذا الإصدار. يمكن التقاط النصوص \
                              أثناء اللعب بدلًا من ذلك.",
                sabab_injilizi: "Taarib identified this game's engine, but it is an in-house \
                                 engine that keeps its text in formats no extractor in this \
                                 build reads. Capturing text while you play works instead.",
            },
            AilatMuharrik::Majhul => Self::LaShay {
                sabab_arabi: "لم يُتعرَّف على محرّك هذه اللعبة، ولا يمكن اختيار طريقة استخراج \
                              بدون معرفته. يمكن التقاط النصوص أثناء اللعب بدلًا من ذلك.",
                sabab_injilizi: "This game's engine was not identified, and an extractor cannot \
                                 be chosen without knowing it. Capturing text while you play \
                                 works instead.",
            },
        }
    }

    /// Whether a static extractor exists.
    #[must_use]
    pub const fn yastakhrij(self) -> bool {
        !matches!(self, Self::LaShay { .. })
    }

    /// Whether runtime capture is worth offering.
    ///
    /// Always. Where extraction works, capture supplies the measured pixel
    /// width that nothing static can — and that Phase 14's overflow report
    /// depends on. Where it does not, capture is the whole answer. Written as an
    /// exhaustive match rather than a bare `true`, so that a future extractor
    /// added with capture *not* worth offering has to say so.
    #[must_use]
    pub const fn yastafeed_min_iltiqat(self) -> bool {
        match self {
            Self::Unity
            | Self::Unreal
            | Self::Godot
            | Self::Nusus
            | Self::Qamus
            | Self::LaShay { .. } => true,
        }
    }

    /// The extractor's name, for the log and the refusal report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Unity => "unity",
            Self::Unreal => "unreal",
            Self::Godot => "godot",
            Self::Nusus => "nusus",
            Self::Qamus => "qamus",
            Self::LaShay { .. } => "none",
        }
    }
}

/// Runs the extractor an engine dispatches to.
///
/// The one call site for every extractor in this crate. Everything above it —
/// the workshop, the card's translate entry, the command palette — reaches
/// extraction through here and never names a module.
///
/// # Errors
///
/// [`KhataIstikhraj::JidhrMafqud`] when the game's directory cannot be read, and
/// [`KhataIstikhraj::MuharrikGhayrMadum`] when the engine has no extractor —
/// which is a refusal to start rather than a failure, and carries the sentence
/// the user reads.
///
/// Note what is **not** an error: a run that read four containers and refused
/// two returns `Ok`, with the refusals in the report. See [`crate::rafd`].
pub fn istakhrij(
    jidhr: &Path,
    aila: AilatMuharrik,
) -> Result<(JadwalNusus, TaqreerRafd), KhataIstikhraj> {
    if !jidhr.is_dir() {
        return Err(KhataIstikhraj::JidhrMafqud {
            masar: jidhr.to_path_buf(),
            sabab: std::io::Error::from(std::io::ErrorKind::NotFound),
        });
    }

    match Mustakhrij::li_aila(aila) {
        Mustakhrij::Unity => Ok(crate::unity::istakhrij(jidhr)),
        Mustakhrij::Unreal => Ok(crate::unreal::istakhrij(jidhr)),
        Mustakhrij::Godot => Ok(crate::godot::istakhrij(jidhr)),
        Mustakhrij::Nusus => Ok(crate::nusus::istakhrij(jidhr, aila)),
        Mustakhrij::Qamus => Ok(crate::qamus::istakhrij(jidhr)),
        Mustakhrij::LaShay { .. } => Err(KhataIstikhraj::MuharrikGhayrMadum {
            aila: format!("{aila:?}"),
        }),
    }
}
