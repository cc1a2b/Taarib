//! Failures of drafting, submitting, reviewing and publishing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The result type of every fallible submission operation.
pub type NatijatTaqdeem<T> = Result<T, KhataTaqdeem>;

/// Failures of submission and review.
#[derive(Debug, thiserror::Error)]
pub enum KhataTaqdeem {
    /// A blocking pre-flight check failed, so submit stays inactive.
    #[error("{adad} blocking check(s) failed and the submission was not sent")]
    BawwabaMaghlaqa {
        /// How many.
        adad: usize,
        /// The first few, each naming what to fix.
        amthila: Vec<String>,
    },

    /// A warning that needs acknowledging was not acknowledged.
    #[error("{adad} warning(s) have not been acknowledged")]
    TahdheerBilaIqrar {
        /// How many.
        adad: usize,
    },

    /// The draft is missing something a submission cannot be sent without.
    #[error("the submission cannot be sent without {haql}")]
    BayanNaqis {
        /// Which field.
        haql: &'static str,
    },

    /// The contributor already has a published patch for this game and build.
    #[error("this duplicates an existing patch for the same build")]
    Mukarrar {
        /// The patch it duplicates.
        ruqaa: String,
    },

    /// A draft could not be read or written.
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

    /// Authenticating against the repository host failed or was abandoned.
    #[error("authentication with the repository host did not complete: {sabab}")]
    TawthiqFashil {
        /// Why.
        sabab: String,
    },

    /// The repository host refused an operation.
    #[error("the repository host refused {amal}: {sabab}")]
    MustawdaRafad {
        /// What was being attempted.
        amal: &'static str,
        /// What the host said.
        sabab: String,
    },

    /// A rejection was recorded with no written reason.
    ///
    /// Unreachable through this crate's own types, and checked anyway because a
    /// stored review record is a document on disk.
    #[error("a rejection requires a written reason")]
    RafdBilaSabab,

    /// A publish step failed and the staged binary was rolled back.
    #[error("publishing failed at {marhala} and was rolled back: {sabab}")]
    NashrFashil {
        /// Which step.
        marhala: &'static str,
        /// Why.
        sabab: String,
    },

    /// Sandbox verification could not be run.
    #[error("sandbox verification could not run: {sabab}")]
    SandooqFashil {
        /// Why.
        sabab: String,
    },

    /// The forge transport has no client identifier or no staging endpoint, so
    /// there is nothing to send to.
    ///
    /// Distinct from [`Self::BayanNaqis`] on purpose: that one is the
    /// contributor's draft missing something they can supply, this one is the
    /// registry operator not having provisioned the channel yet. Telling a
    /// contributor to "fix" a field only the operator can fill would send them
    /// looking for a setting that is not theirs.
    #[error("the forge transport is not provisioned: {naqis} is missing")]
    IrsalGhayrMuhayya {
        /// Which settings field is absent, spelled as settings spell it.
        naqis: &'static str,
    },
}

impl Tafsir for KhataTaqdeem {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TAQDEEM
                + match self {
                    Self::BawwabaMaghlaqa { .. } => 0,
                    Self::TahdheerBilaIqrar { .. } => 1,
                    Self::BayanNaqis { .. } => 2,
                    Self::Mukarrar { .. } => 3,
                    Self::KhataMalaf { .. } => 4,
                    Self::TawthiqFashil { .. } => 5,
                    Self::MustawdaRafad { .. } => 6,
                    Self::RafdBilaSabab => 7,
                    Self::NashrFashil { .. } => 8,
                    Self::SandooqFashil { .. } => 9,
                    Self::IrsalGhayrMuhayya { .. } => 10,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::NashrFashil { .. } | Self::RafdBilaSabab => Khutura::Fadih,
            Self::BawwabaMaghlaqa { .. }
            | Self::TahdheerBilaIqrar { .. }
            | Self::IrsalGhayrMuhayya { .. } => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::BawwabaMaghlaqa { adad, .. } => format!(
                "لم يجتز {adad} فحصًا لازمًا، ولم يُرسل التقديم. كلّ فحصٍ يشير إلى العبارات \
                 التي أخفقت فيه."
            ),
            Self::TahdheerBilaIqrar { adad } => {
                format!("{adad} تنبيهًا يحتاج إقرارك قبل الإرسال.")
            },
            Self::BayanNaqis { haql } => format!("لا يمكن الإرسال بدون ({haql})."),
            Self::Mukarrar { .. } => "لديك رقعة منشورة لهذه اللعبة ولهذا البناء نفسه.".to_owned(),
            Self::KhataMalaf { .. } => "تعذّرت قراءة مسوّدة التقديم أو الكتابة إليها.".to_owned(),
            Self::TawthiqFashil { .. } => {
                "لم يكتمل التوثيق مع مستضيف المستودع. أعد المحاولة وأدخل الرمز القصير.".to_owned()
            },
            Self::MustawdaRafad { .. } => "رفض مستضيف المستودع العملية.".to_owned(),
            Self::RafdBilaSabab => "الرفض يحتاج سببًا مكتوبًا.".to_owned(),
            Self::NashrFashil { .. } => {
                "أخفق النشر وأُعيد ما رُفع إلى ما كان عليه؛ لا توجد رقعة نصف منشورة.".to_owned()
            },
            Self::SandooqFashil { .. } => "تعذّر تشغيل التحقّق في البيئة المعزولة.".to_owned(),
            Self::IrsalGhayrMuhayya { naqis } => format!(
                "قناة الرفع إلى السجلّ غير مجهّزة بعد: الحقل {naqis} فارغ في الإعدادات. بقي \
                 تقديمك مسجّلًا محليًا، ويُرفع تلقائيًا متى جهّز مشغّل السجلّ القناة."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            // The only variant whose Display is deliberately shorter than what
            // a person is owed: the sentence has to say whose job the missing
            // field is, or a contributor goes hunting for a setting that was
            // never theirs to fill.
            Self::IrsalGhayrMuhayya { naqis } => format!(
                "The registry upload channel is not provisioned yet: the {naqis} field is \
                 empty in Settings. Your submission stays recorded locally and is sent the \
                 moment the registry operator provisions the channel."
            ),
            _ => self.to_string(),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::BawwabaMaghlaqa { .. } | Self::TahdheerBilaIqrar { .. } => Khutwa::FathNusus,
            Self::BayanNaqis { .. } | Self::Mukarrar { .. } => Khutwa::FathTashkhis,
            Self::TawthiqFashil { .. } | Self::MustawdaRafad { .. } => Khutwa::AadaMuhawala,
            Self::RafdBilaSabab | Self::NashrFashil { .. } | Self::SandooqFashil { .. } => {
                Khutwa::IblaghLilMalik
            },
            Self::IrsalGhayrMuhayya { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Masadir,
            },
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
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
            Self::KhataMalaf { .. } | Self::RafdBilaSabab => {},
            Self::BawwabaMaghlaqa { adad, amthila } => {
                daa(
                    "adad",
                    QeemaSiyaq::Hajm(u64::try_from(*adad).unwrap_or(u64::MAX)),
                );
                daa("amthila", QeemaSiyaq::Qaima(amthila.clone()));
            },
            Self::TahdheerBilaIqrar { adad } => {
                daa(
                    "adad",
                    QeemaSiyaq::Hajm(u64::try_from(*adad).unwrap_or(u64::MAX)),
                );
            },
            Self::BayanNaqis { haql } => daa("haql", QeemaSiyaq::Nass((*haql).to_owned())),
            Self::IrsalGhayrMuhayya { naqis } => {
                daa("naqis", QeemaSiyaq::Nass((*naqis).to_owned()));
            },
            Self::Mukarrar { ruqaa } => daa("ruqaa", QeemaSiyaq::Nass(ruqaa.clone())),
            Self::TawthiqFashil { sabab } | Self::SandooqFashil { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MustawdaRafad { amal, sabab } => {
                daa("amal", QeemaSiyaq::Nass((*amal).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::NashrFashil { marhala, sabab } => {
                daa("marhala", QeemaSiyaq::Nass((*marhala).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataTaqdeem);
