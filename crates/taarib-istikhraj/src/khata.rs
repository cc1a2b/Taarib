//! أخطاء الاستخراج — what stops an extraction, as opposed to what it refuses.
//!
//! The distinction is the whole module and it is easy to get backwards.
//!
//! **A refusal is not an error.** An extraction that read four containers and
//! skipped two succeeded: it produced a table and a report naming what it
//! skipped and why. That lives in [`crate::rafd`] and never reaches this type.
//! An IL2CPP build with no type tree is the *normal* case for a large class of
//! games, and modelling it as an error would make the common path the failure
//! path.
//!
//! **An error is an extraction that could not run.** The game directory is
//! gone, the project cannot be written, the engine is one this crate has no
//! extractor for at all. Those stop the operation rather than narrowing it.
//!
//! Getting this backwards produces one of two bad products: one that reports
//! success for a game it read nothing from, or one that reports failure for a
//! game it extracted eight thousand strings from. The type boundary is what
//! keeps both from happening.
//!
//! ## Error codes
//!
//! This crate owns [`arqam::ISTIKHRAJ`], which is 5000, in its entirety.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures that stop an extraction.
#[derive(Debug, thiserror::Error)]
pub enum KhataIstikhraj {
    /// The game's directory is not there or cannot be read.
    #[error("{masar} could not be read")]
    JidhrMafqud {
        /// Where the game was expected.
        masar: PathBuf,
        /// What the filesystem said.
        sabab: std::io::Error,
    },

    /// This crate has no extractor for the identified engine.
    ///
    /// Distinct from every refusal in [`crate::rafd`]: those are containers
    /// within an engine this crate does handle. This is the engine itself being
    /// outside the set, which means there is nothing to run and the answer is
    /// the tier-3 overlay rather than a capture session.
    #[error("no extractor for {aila}")]
    MuharrikGhayrMadum {
        /// The engine family, as Phase 5 identified it.
        aila: String,
    },

    /// The engine could not be identified at all.
    ///
    /// Extraction is dispatched per engine, so an unidentified game has no
    /// dispatch target. Worth its own variant rather than folding into the one
    /// above, because the remedy differs: an unknown engine is a diagnostics
    /// report, and an unsupported known engine is a feature request.
    #[error("the engine at {masar} could not be identified")]
    MuharrikMajhul {
        /// The game's root.
        masar: PathBuf,
    },

    /// A container declared a size larger than this build will allocate.
    ///
    /// Checked against the number the container *declares*, before a byte is
    /// read, so a damaged or hostile file is a refusal rather than an exhausted
    /// machine.
    #[error("{haql} declares {qeema}, above this build's ceiling of {saqf}")]
    HajmMufrit {
        /// Which field.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// Markup in a string could not be parsed.
    ///
    /// Raised by [`crate::jadwal::irfa_nasq`] and — importantly — handled by its
    /// callers rather than propagated: a string whose markup will not parse is
    /// still extracted, with its raw text as its clean text and no spans.
    /// Refusing to extract it would lose it entirely, and a string with
    /// unparsed markup is worth more to a translator than no string at all.
    #[error("markup in {nass} could not be parsed: {sabab}")]
    NasqTalif {
        /// The first 64 characters of the string, so it can be found.
        nass: String,
        /// What the parser said.
        sabab: String,
    },

    /// The project could not be written.
    #[error("the project at {masar} could not be written")]
    TaadhurKitabatMashru {
        /// Where.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// A stored project is in a shape this build does not know.
    ///
    /// Refused rather than migrated. A project is a contributor's work and a
    /// migration that guessed wrong would corrupt it silently.
    #[error("{masar} is a project schema {wujid}, and this build knows {madum}")]
    MukhattatGhayrMafhum {
        /// The project file.
        masar: PathBuf,
        /// The schema found.
        wujid: u32,
        /// The schema this build reads.
        madum: u32,
    },

    /// A capture session's file could not be read or written.
    #[error("the capture session at {masar} could not be used: {sabab}")]
    KhataIltiqat {
        /// The session file.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },
}

impl Tafsir for KhataIstikhraj {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::ISTIKHRAJ
                + match self {
                    Self::JidhrMafqud { .. } => 0,
                    Self::MuharrikGhayrMadum { .. } => 1,
                    Self::MuharrikMajhul { .. } => 2,
                    Self::HajmMufrit { .. } => 3,
                    Self::NasqTalif { .. } => 4,
                    Self::TaadhurKitabatMashru { .. } => 5,
                    Self::MukhattatGhayrMafhum { .. } => 6,
                    Self::KhataIltiqat { .. } => 7,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The string is still extracted; this is a note on it.
            Self::NasqTalif { .. } => Khutura::Tanbeeh,
            // A contributor's work is at stake.
            Self::TaadhurKitabatMashru { .. } | Self::MukhattatGhayrMafhum { .. } => {
                Khutura::Fadih
            }
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::JidhrMafqud { .. } => {
                "تعذّر الوصول إلى مجلّد اللعبة. قد تكون نُقلت أو حُذفت.".to_owned()
            }
            Self::MuharrikGhayrMadum { aila } => format!(
                "لا يستطيع تعريب استخراج النصوص من محرّك ({aila}) بعد. تبقى الطبقة العامة \
                 متاحة لهذه اللعبة."
            ),
            Self::MuharrikMajhul { .. } => {
                "تعذّر التعرّف على محرّك هذه اللعبة، ولا يمكن اختيار طريقة استخراج بدونه."
                    .to_owned()
            }
            Self::HajmMufrit { .. } => {
                "تعلن إحدى حاويات اللعبة حجمًا أكبر مما تسمح به هذه النسخة، ورُفضت قبل حجز أي \
                 ذاكرة."
                    .to_owned()
            }
            Self::NasqTalif { .. } => {
                "تعذّر تحليل الوسوم داخل أحد النصوص، واستُخرج النص كما هو دون فصل وسومه."
                    .to_owned()
            }
            Self::TaadhurKitabatMashru { .. } => {
                "تعذّرت كتابة مشروع الترجمة على القرص، ولم يُحفظ شيء.".to_owned()
            }
            Self::MukhattatGhayrMafhum { .. } => {
                "مشروع الترجمة محفوظ بصيغة لا تعرفها هذه النسخة. لم يُفتح ولم يُعدَّل، حفاظًا \
                 على عمل صاحبه."
                    .to_owned()
            }
            Self::KhataIltiqat { .. } => {
                "تعذّر استخدام ملف جلسة الالتقاط.".to_owned()
            }
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::JidhrMafqud { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::MuharrikGhayrMadum { .. } | Self::MuharrikMajhul { .. } => Khutwa::FathTashkhis,
            Self::HajmMufrit { .. } => Khutwa::IblaghLilMusahim,
            // The extraction continues; there is nothing for the user to do.
            Self::NasqTalif { .. } => Khutwa::LaShay,
            Self::TaadhurKitabatMashru { .. } => Khutwa::TahrirMasaha,
            Self::MukhattatGhayrMafhum { .. } => Khutwa::TahdithTaarib,
            Self::KhataIltiqat { .. } => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Handled before the general map exists. `siyaq_io` returns a different
        // map, and inserting the path into the one built below would insert it
        // into something that is then discarded — a bug three earlier crates in
        // this workspace shipped before it was found.
        if let Self::JidhrMafqud { masar, sabab } = self {
            let mut siyaq = siyaq_io(sabab);
            let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            return siyaq;
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::MuharrikGhayrMadum { aila } => daa("aila", QeemaSiyaq::Nass(aila.clone())),
            Self::MuharrikMajhul { masar } => daa("masar", QeemaSiyaq::Masar(masar.clone())),
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            }
            Self::NasqTalif { nass, sabab } => {
                daa("nass", QeemaSiyaq::Nass(nass.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::TaadhurKitabatMashru { masar, sabab }
            | Self::KhataIltiqat { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::MukhattatGhayrMafhum { masar, wujid, madum } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            }
            // Handled above.
            Self::JidhrMafqud { .. } => {}
        }
        siyaq
    }
}

khata_min!(KhataIstikhraj);

/// A length as a `u64`, without a cast that can wrap.
#[must_use]
pub fn tul_u64(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}

/// A `u64` back to a length, or [`None`] when it does not fit this target.
#[must_use]
pub fn hajm_usize(qeema: u64) -> Option<usize> {
    usize::try_from(qeema).ok()
}
