//! Failures of sharing, merging and interchange.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The result type of every fallible workspace operation.
pub type NatijatWarsha<T> = Result<T, KhataWarsha>;

/// Failures of the collaborative workspace.
#[derive(Debug, thiserror::Error)]
pub enum KhataWarsha {
    /// A shared bundle could not be read or written.
    #[error("{masar} could not be read or written while {amal}")]
    KhataMalaf {
        /// The path.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A bundle is not one this build reads.
    #[error("the bundle could not be read: {sabab}")]
    HuzmaTalifa {
        /// Why.
        sabab: String,
    },

    /// A bundle was written by a newer build.
    #[error("the bundle is schema {wujid} and this build reads {madum}")]
    IsdarMajhul {
        /// The version found.
        wujid: u32,
        /// The version this build reads.
        madum: u32,
    },

    /// Two bundles describe different projects.
    #[error("the bundles belong to different projects and were not merged")]
    MashruMukhtalif,

    /// A merge was finalized while conflicts were still unresolved.
    #[error("{adad} conflict(s) are unresolved and the merge was not finalized")]
    NizaatMuallaqa {
        /// How many.
        adad: usize,
    },

    /// A resolution named a conflict the merge does not contain.
    #[error("a resolution names a string that is not in conflict")]
    QararBilaNizaa,

    /// An export failed to round-trip through its own importer.
    #[error("the {sigha} export did not round-trip: {sabab}")]
    TabadulGhayrMutabiq {
        /// The format.
        sigha: &'static str,
        /// What differed.
        sabab: String,
    },

    /// The underlying project, memory or glossary refused.
    #[error("{sabab}")]
    Mawrid {
        /// The underlying failure, already explained.
        sabab: String,
    },
}

impl Tafsir for KhataWarsha {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::WARSHA
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::HuzmaTalifa { .. } => 1,
                    Self::IsdarMajhul { .. } => 2,
                    Self::MashruMukhtalif => 3,
                    Self::NizaatMuallaqa { .. } => 4,
                    Self::QararBilaNizaa => 5,
                    Self::TabadulGhayrMutabiq { .. } => 6,
                    Self::Mawrid { .. } => 7,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::TabadulGhayrMutabiq { .. } => Khutura::Fadih,
            Self::NizaatMuallaqa { .. } => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => "تعذّرت قراءة ملف المشاركة أو الكتابة إليه.".to_owned(),
            Self::HuzmaTalifa { .. } => "ملف المشاركة غير مقروء بهذه النسخة.".to_owned(),
            Self::IsdarMajhul { .. } => {
                "كُتب ملف المشاركة بنسخة أحدث من تعريب؛ حدِّث البرنامج.".to_owned()
            }
            Self::MashruMukhtalif => {
                "الملفّان يخصّان مشروعين مختلفين، ولم يُدمجا.".to_owned()
            }
            Self::NizaatMuallaqa { adad } => {
                format!("بقي {adad} تعارضًا بلا حسم، ولم يُختم الدمج قبل حسمها كلّها.")
            }
            Self::QararBilaNizaa => {
                "أشار قرار حسم إلى عبارة ليست في تعارض.".to_owned()
            }
            Self::TabadulGhayrMutabiq { .. } => {
                "لم يرجع ملف التبادل كما صُدِّر، ولم يُسلَّم.".to_owned()
            }
            Self::Mawrid { sabab } => sabab.clone(),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
            Self::IsdarMajhul { .. } => Khutwa::TahdithTaarib,
            Self::NizaatMuallaqa { .. } => Khutwa::FathNusus,
            Self::TabadulGhayrMutabiq { .. } | Self::QararBilaNizaa => Khutwa::IblaghLilMalik,
            Self::HuzmaTalifa { .. } | Self::MashruMukhtalif | Self::Mawrid { .. } => {
                Khutwa::FathTashkhis
            }
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        if let Self::KhataMalaf { masar, amal, sabab } = self {
            let mut siyaq = siyaq_io(sabab);
            let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            let _ = siyaq.insert("amal".to_owned(), QeemaSiyaq::Nass((*amal).to_owned()));
            return siyaq;
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::KhataMalaf { .. } | Self::MashruMukhtalif | Self::QararBilaNizaa => {}
            Self::HuzmaTalifa { sabab } | Self::Mawrid { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::IsdarMajhul { wujid, madum } => {
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            }
            Self::NizaatMuallaqa { adad } => {
                daa("adad", QeemaSiyaq::Hajm(u64::try_from(*adad).unwrap_or(u64::MAX)));
            }
            Self::TabadulGhayrMutabiq { sigha, sabab } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataWarsha);
