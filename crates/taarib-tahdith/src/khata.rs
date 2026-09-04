//! Failures of checking, downloading and verifying an application update.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The result type of every fallible self-update operation.
pub type NatijatTahdith<T> = Result<T, KhataTahdith>;

/// Failures of the self-updater.
#[derive(Debug, thiserror::Error)]
pub enum KhataTahdith {
    /// The update channel could not be reached, or broke off mid-answer.
    #[error("the update channel could not be reached: {sabab}")]
    QanatGhayrMutaha {
        /// What the attempt reported.
        sabab: String,
    },

    /// The channel manifest is not one this build reads.
    #[error("the update manifest could not be read: {sabab}")]
    BayanTalif {
        /// Why.
        sabab: String,
    },

    /// The manifest's detached signature does not verify against the anchor.
    #[error("the update manifest's signature does not verify against the owner's embedded key")]
    TawqeeGhayrSalih,

    /// The manifest is signed by the committed development key, and this is a
    /// release build that names and refuses that key specifically.
    #[error(
        "the update channel is signed by the published development key, which a release build \
         refuses by name"
    )]
    TawqeeTatwir,

    /// The manifest lists no entry for this target on this channel.
    #[error("the update manifest lists no entry for {hadaf} on channel {qanat}")]
    LaMadkhal {
        /// The target triple this build runs on.
        hadaf: String,
        /// The channel that was asked.
        qanat: String,
    },

    /// The offered entry's minimum version is above the running version.
    #[error("the update requires at least version {adna} and this build is {hali}")]
    AdnaIsdarFawq {
        /// The floor the entry declares.
        adna: String,
        /// The version running now.
        hali: String,
    },

    /// A downloaded update does not hash to what the manifest declared.
    #[error("{rabt} hashes to {mahsuba}, not the declared {muallana}")]
    TanzeelGhayrMutabiq {
        /// Where it came from.
        rabt: String,
        /// The hash the manifest declared.
        muallana: String,
        /// The hash the bytes produced.
        mahsuba: String,
    },

    /// A download exceeded the size the manifest declared.
    #[error("the download exceeded its declared {muallan} bytes")]
    HajmMufrit {
        /// The declared size.
        muallan: u64,
    },

    /// A download was interrupted and what is on disk cannot be resumed.
    #[error("the download from {rabt} was interrupted and cannot be resumed: {sabab}")]
    IstinafMutaadhdhir {
        /// Where it came from.
        rabt: String,
        /// Why resuming is impossible.
        sabab: String,
    },

    /// The destination's filesystem cannot hold the download and its headroom.
    #[error(
        "{masar} needs {} MB free and has only {} MB; free disk space and retry",
        mijabayt_matluba(*.matlub),
        mijabayt_mutaha(*.mutah)
    )]
    MisahaGhayrKafiya {
        /// Bytes still required, headroom included.
        matlub: u64,
        /// Bytes the destination's filesystem reports free.
        mutah: u64,
        /// The destination.
        masar: PathBuf,
    },

    /// A local file could not be read or written.
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
}

impl Tafsir for KhataTahdith {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TAHDITH
                + match self {
                    Self::QanatGhayrMutaha { .. } => 0,
                    Self::BayanTalif { .. } => 1,
                    Self::TawqeeGhayrSalih => 2,
                    Self::TawqeeTatwir => 3,
                    Self::LaMadkhal { .. } => 4,
                    Self::AdnaIsdarFawq { .. } => 5,
                    Self::TanzeelGhayrMutabiq { .. } => 6,
                    Self::HajmMufrit { .. } => 7,
                    Self::IstinafMutaadhdhir { .. } => 8,
                    Self::MisahaGhayrKafiya { .. } => 9,
                    Self::KhataMalaf { .. } => 10,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // A channel the anchor did not sign, or content that did not hash
            // to what the signed manifest declared: a substitution attempt.
            Self::TawqeeGhayrSalih | Self::TawqeeTatwir | Self::TanzeelGhayrMutabiq { .. } => {
                Khutura::Fadih
            }
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::QanatGhayrMutaha { .. } => {
                "تعذّر الوصول إلى قناة التحديث. تحقّق من اتصالك ثم أعد المحاولة؛ نسختك الحالية تبقى تعمل كما هي."
                    .to_owned()
            }
            Self::BayanTalif { .. } => {
                "بيان التحديث غير مقروء بهذه النسخة. أعد المحاولة لاحقًا، وإن استمرّ الخطأ فنزّل تعريب من صفحة الإصدارات الرسمية."
                    .to_owned()
            }
            Self::TawqeeGhayrSalih => {
                "توقيع قناة التحديث لا يطابق مفتاح المالك المضمّن، ورُفض التحديث كاملًا. أبلغ المالك ولا تثبّت شيئًا من مصدر آخر."
                    .to_owned()
            }
            Self::TawqeeTatwir => {
                "قناة التحديث موقّعة بمفتاح التطوير المعلن، وهذه نسخة إصدار ترفضه بالاسم. أبلغ المالك؛ لا يُحدَّث عميل الإصدار إلا بما وقّعه المالك."
                    .to_owned()
            }
            Self::LaMadkhal { .. } => {
                "لا يذكر بيان التحديث إصدارًا لمنصّتك على هذه القناة. إن احتجت الأحدث فثبّته يدويًا من صفحة الإصدارات الرسمية."
                    .to_owned()
            }
            Self::AdnaIsdarFawq { .. } => {
                "نسختك الحالية أقدم من الحدّ الأدنى الذي يقبله هذا التحديث. ثبّت الإصدار الأحدث يدويًا من صفحة الإصدارات الرسمية."
                    .to_owned()
            }
            Self::TanzeelGhayrMutabiq { .. } => {
                "بصمة حزمة التحديث المنزّلة لا تطابق المعلنة في البيان الموقّع، فحُذفت ولم تُثبَّت. أبلغ المالك."
                    .to_owned()
            }
            Self::HajmMufrit { .. } => {
                "تجاوز تنزيل التحديث الحجم المعلن له، فأُوقف وحُذف. أعد المحاولة لاحقًا.".to_owned()
            }
            Self::IstinafMutaadhdhir { .. } => {
                "انقطع تنزيل التحديث وتعذّر استئنافه من حيث توقّف. أعد المحاولة ليبدأ التنزيل من جديد."
                    .to_owned()
            }
            Self::MisahaGhayrKafiya { matlub, mutah, .. } => format!(
                "لا تكفي المساحة الخالية على القرص لتنزيل التحديث: يحتاج {} ميغابايت والمتاح {} ميغابايت فقط. أخلِ مساحة ثم أعد المحاولة.",
                mijabayt_matluba(*matlub),
                mijabayt_mutaha(*mutah)
            ),
            Self::KhataMalaf { .. } => {
                "تعذّرت قراءة ملف محلّي أو الكتابة إليه أثناء التحديث.".to_owned()
            }
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::QanatGhayrMutaha { .. } | Self::IstinafMutaadhdhir { .. } => {
                Khutwa::AadaMuhawala
            }
            Self::BayanTalif { .. } | Self::HajmMufrit { .. } => Khutwa::FathTashkhis,
            Self::TawqeeGhayrSalih | Self::TawqeeTatwir | Self::TanzeelGhayrMutabiq { .. } => {
                Khutwa::IblaghLilMalik
            }
            Self::LaMadkhal { .. } | Self::AdnaIsdarFawq { .. } => Khutwa::LaShay,
            Self::MisahaGhayrKafiya { .. } => Khutwa::TahrirMasaha,
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladRuqaa),
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
            Self::KhataMalaf { .. } | Self::TawqeeGhayrSalih | Self::TawqeeTatwir => {}
            Self::QanatGhayrMutaha { sabab } | Self::BayanTalif { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::LaMadkhal { hadaf, qanat } => {
                daa("hadaf", QeemaSiyaq::Nass(hadaf.clone()));
                daa("qanat", QeemaSiyaq::Nass(qanat.clone()));
            }
            Self::AdnaIsdarFawq { adna, hali } => {
                daa("adna", QeemaSiyaq::Nass(adna.clone()));
                daa("hali", QeemaSiyaq::Nass(hali.clone()));
            }
            Self::TanzeelGhayrMutabiq { rabt, muallana, mahsuba } => {
                daa("rabt", QeemaSiyaq::Nass(rabt.clone()));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            }
            Self::HajmMufrit { muallan } => daa("muallan", QeemaSiyaq::Hajm(*muallan)),
            Self::IstinafMutaadhdhir { rabt, sabab } => {
                daa("rabt", QeemaSiyaq::Nass(rabt.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::MisahaGhayrKafiya { matlub, mutah, masar } => {
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
                daa("mutah", QeemaSiyaq::Hajm(*mutah));
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            }
        }
        siyaq
    }
}

/// Whole megabytes rounded up, so a small requirement never reads as zero.
const fn mijabayt_matluba(bayt: u64) -> u64 {
    bayt.saturating_add((1 << 20) - 1) >> 20
}

/// Whole megabytes rounded down, so free space is never overstated.
const fn mijabayt_mutaha(bayt: u64) -> u64 {
    bayt >> 20
}

khata_min!(KhataTahdith);
