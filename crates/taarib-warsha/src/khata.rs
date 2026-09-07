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

    /// A memory share was to be written with warnings nobody acknowledged.
    #[error("{} warning(s) about the shared readings are unacknowledged: {}",
        asma.len(), asma.join("; "))]
    TahdheeratMuallaqa {
        /// The unacknowledged warnings, in their own words.
        asma: Vec<String>,
    },

    /// A sharing permit was spent on an entry set it was not granted for.
    #[error("the sharing permit was granted for a different set of readings")]
    IdhnGhayrMutabiq,

    /// A memory share would have carried nothing.
    #[error("there are no shareable readings for this game and nothing was written")]
    MusharakaFarigha,

    /// A share's signature does not verify against the expected key.
    #[error("the share is not signed by the key it was checked against")]
    TawqeeGhayrSalih,

    /// A rescue was asked of a string file whose every line reads.
    #[error("every row of the string file reads; there is nothing to set aside")]
    LaTalaf,

    /// The preserved copy of a damaged string file did not read back as the
    /// original, so the live table was left untouched.
    #[error(
        "the preserved copy at {} does not match the original byte for byte; the string file \
         was left untouched",
        masar.display()
    )]
    InqadhGhayrMuthbat {
        /// The copy that disagreed.
        masar: PathBuf,
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
                    Self::TahdheeratMuallaqa { .. } => 8,
                    Self::IdhnGhayrMutabiq => 9,
                    Self::MusharakaFarigha => 10,
                    Self::TawqeeGhayrSalih => 11,
                    Self::LaTalaf => 12,
                    Self::InqadhGhayrMuthbat { .. } => 13,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // A permit spent on the wrong payload means something showed a
            // user one thing and was about to write another; that is the
            // consent gate failing, not a user mistake.
            Self::TabadulGhayrMutabiq { .. } | Self::IdhnGhayrMutabiq => Khutura::Fadih,
            Self::NizaatMuallaqa { .. }
            | Self::TahdheeratMuallaqa { .. }
            | Self::MusharakaFarigha
            | Self::LaTalaf => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => "تعذّرت قراءة ملف المشاركة أو الكتابة إليه.".to_owned(),
            Self::HuzmaTalifa { .. } => "ملف المشاركة غير مقروء بهذه النسخة.".to_owned(),
            Self::IsdarMajhul { .. } => {
                "كُتب ملف المشاركة بنسخة أحدث من تعريب؛ حدِّث البرنامج.".to_owned()
            },
            Self::MashruMukhtalif => "الملفّان يخصّان مشروعين مختلفين، ولم يُدمجا.".to_owned(),
            Self::NizaatMuallaqa { adad } => {
                format!("بقي {adad} تعارضًا بلا حسم، ولم يُختم الدمج قبل حسمها كلّها.")
            },
            Self::QararBilaNizaa => "أشار قرار حسم إلى عبارة ليست في تعارض.".to_owned(),
            Self::TabadulGhayrMutabiq { .. } => "لم يرجع ملف التبادل كما صُدِّر، ولم يُسلَّم.".to_owned(),
            Self::TahdheeratMuallaqa { asma } => {
                format!("بقي {} تنبيهًا لم تُقرّ به، ولم تُشارك الذاكرة.", asma.len())
            },
            Self::IdhnGhayrMutabiq => {
                "الإذن مُنح لمجموعة أسطر غير التي كانت ستُكتب، ولم تُشارك.".to_owned()
            },
            Self::MusharakaFarigha => {
                "لا توجد أسطر قابلة للمشاركة لهذه اللعبة، فلم يُكتب ملف.".to_owned()
            },
            Self::TawqeeGhayrSalih => "توقيع ملف الذاكرة لا يطابق المفتاح المتوقّع، ورُفض.".to_owned(),
            Self::LaTalaf => "كل أسطر ملف النصوص تُقرأ؛ لا شيء يُبقى جانبًا ولم يُكتب شيء.".to_owned(),
            Self::InqadhGhayrMuthbat { masar } => format!(
                "النسخة المحفوظة في {} لا تطابق الأصل بايتًا ببايت، فتُرك ملف النصوص كما هو.",
                masar.display()
            ),
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
            Self::NizaatMuallaqa { .. }
            | Self::TahdheeratMuallaqa { .. }
            | Self::MusharakaFarigha => Khutwa::FathNusus,
            Self::TabadulGhayrMutabiq { .. } | Self::QararBilaNizaa | Self::IdhnGhayrMutabiq => {
                Khutwa::IblaghLilMalik
            },
            Self::LaTalaf => Khutwa::LaShay,
            Self::HuzmaTalifa { .. }
            | Self::MashruMukhtalif
            | Self::TawqeeGhayrSalih
            | Self::InqadhGhayrMuthbat { .. }
            | Self::Mawrid { .. } => Khutwa::FathTashkhis,
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
            Self::KhataMalaf { .. }
            | Self::MashruMukhtalif
            | Self::QararBilaNizaa
            | Self::IdhnGhayrMutabiq
            | Self::MusharakaFarigha
            | Self::TawqeeGhayrSalih
            | Self::LaTalaf => {},
            Self::InqadhGhayrMuthbat { masar } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            },
            Self::TahdheeratMuallaqa { asma } => {
                daa("tahdheerat", QeemaSiyaq::Nass(asma.join("; ")));
            },
            Self::HuzmaTalifa { sabab } | Self::Mawrid { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::IsdarMajhul { wujid, madum } => {
                daa("wujid", QeemaSiyaq::Raqm(i64::from(*wujid)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            },
            Self::NizaatMuallaqa { adad } => {
                daa(
                    "adad",
                    QeemaSiyaq::Hajm(u64::try_from(*adad).unwrap_or(u64::MAX)),
                );
            },
            Self::TabadulGhayrMutabiq { sigha, sabab } => {
                daa("sigha", QeemaSiyaq::Nass((*sigha).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataWarsha);
