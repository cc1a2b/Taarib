//! Failures of the safety layer — the checks not running, never their verdicts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The result type of every fallible safety operation.
pub type NatijatAman<T> = Result<T, KhataAman>;

/// A safety check could not be performed. A check that *ran* and refused is a
/// refusal value, not one of these.
#[derive(Debug, thiserror::Error)]
pub enum KhataAman {
    /// A file or directory could not be read while scanning.
    #[error("{masar} could not be read while {amal}")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A downloaded archive could not be extracted into quarantine.
    #[error("the package could not be safely unpacked: {sabab}")]
    FakFashil {
        /// What went wrong.
        sabab: String,
    },

    /// An archive entry named a path that escapes the quarantine root.
    #[error("{madkhal} escapes the quarantine directory and was refused")]
    MadkhalKharij {
        /// The offending entry name.
        madkhal: String,
    },

    /// The revocation list could not be fetched, read or parsed.
    #[error("the revocation list could not be {amal}: {sabab}")]
    QaimatSahbFashila {
        /// What was being attempted.
        amal: &'static str,
        /// Why.
        sabab: String,
    },

    /// The acknowledgement record could not be read or written.
    #[error("the acknowledgement record at {masar} could not be {amal}")]
    KhataIqrar {
        /// The record path.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },
}

impl KhataAman {
    /// The path this failure concerns, when it has one.
    #[must_use]
    pub fn masar(&self) -> Option<&Path> {
        match self {
            Self::KhataMalaf { masar, .. } | Self::KhataIqrar { masar, .. } => Some(masar),
            Self::FakFashil { .. }
            | Self::MadkhalKharij { .. }
            | Self::QaimatSahbFashila { .. } => None,
        }
    }
}

impl Tafsir for KhataAman {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::AMAN
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::FakFashil { .. } => 1,
                    Self::MadkhalKharij { .. } => 2,
                    Self::QaimatSahbFashila { .. } => 3,
                    Self::KhataIqrar { .. } => 4,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        Khutura::Khatar
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => "تعذّرت قراءة أحد ملفات اللعبة أثناء الفحص.".to_owned(),
            Self::FakFashil { .. } => {
                "تعذّر فكّ ضغط الحزمة في الحجر الآمن، ولم يُفحص منها شيء.".to_owned()
            }
            Self::MadkhalKharij { .. } => {
                "يحتوي أرشيف الحزمة على مسار يخرج عن مجلّد الحجر، ورُفض.".to_owned()
            }
            Self::QaimatSahbFashila { .. } => {
                "تعذّر الحصول على قائمة الإبطال أو قراءتها، ولا يُثبَّت شيء دون فحصها.".to_owned()
            }
            Self::KhataIqrar { .. } => "تعذّرت قراءة سجلّ الإقرار أو الكتابة إليه.".to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),
            Self::KhataIqrar { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
            Self::FakFashil { .. } => Khutwa::FathTashkhis,
            Self::MadkhalKharij { .. } => Khutwa::IblaghLilMalik,
            Self::QaimatSahbFashila { .. } => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        match self {
            Self::KhataMalaf { masar, amal, sabab }
            | Self::KhataIqrar { masar, amal, sabab } => {
                let mut siyaq = siyaq_io(sabab);
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("amal".to_owned(), QeemaSiyaq::Nass((*amal).to_owned()));
                return siyaq;
            }
            _ => {}
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::KhataMalaf { .. } | Self::KhataIqrar { .. } => {}
            Self::FakFashil { sabab } => daa("sabab", QeemaSiyaq::Nass(sabab.clone())),
            Self::MadkhalKharij { madkhal } => daa("madkhal", QeemaSiyaq::Nass(madkhal.clone())),
            Self::QaimatSahbFashila { amal, sabab } => {
                daa("amal", QeemaSiyaq::Nass((*amal).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataAman);
