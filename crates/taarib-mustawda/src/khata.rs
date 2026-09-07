//! Failures of fetching, verifying, downloading and installing from a registry.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// The result type of every fallible registry operation.
pub type NatijatMustawda<T> = Result<T, KhataMustawda>;

/// Failures of the registry client.
#[derive(Debug, thiserror::Error)]
pub enum KhataMustawda {
    /// No configured source could be reached.
    #[error("no registry source could be reached: {sabab}")]
    LaMasdar {
        /// What the last attempt reported.
        sabab: String,
    },

    /// A source answered, but not with success.
    #[error("{rabt} answered {ramz}")]
    IstijabaFashila {
        /// The URL.
        rabt: String,
        /// The HTTP status.
        ramz: u16,
    },

    /// The global manifest is not one this build reads.
    #[error("the registry manifest could not be read: {sabab}")]
    BayanTalif {
        /// Why.
        sabab: String,
    },

    /// A manifest older than the cached one was offered.
    #[error("the manifest is revision {wujid} and the cached one is {mukhazzan}")]
    TasalsulLilkhalf {
        /// The revision offered.
        wujid: u64,
        /// The revision already held.
        mukhazzan: u64,
    },

    /// The manifest declares no hash for a shard that was fetched.
    #[error("the manifest declares no shard {raqm}")]
    ShareehaMajhula {
        /// The shard index.
        raqm: u16,
    },

    /// A shard's bytes do not hash to what the manifest declared.
    #[error("shard {raqm} hashes to {mahsuba}, not the declared {muallana}")]
    BasmaGhayrMutabaqa {
        /// The shard index.
        raqm: u16,
        /// The hash the manifest declared.
        muallana: String,
        /// The hash the bytes produced.
        mahsuba: String,
    },

    /// A verified shard's bytes do not parse.
    #[error("shard {raqm} verified and did not parse: {sabab}")]
    ShareehaTalifa {
        /// The shard index.
        raqm: u16,
        /// Why.
        sabab: String,
    },

    /// A downloaded package does not hash to what its listing declared.
    #[error("{rabt} hashes to {mahsuba}, not the declared {muallana}")]
    TanzeelGhayrMutabiq {
        /// Where it came from.
        rabt: String,
        /// The hash the listing declared.
        muallana: String,
        /// The hash the bytes produced.
        mahsuba: String,
    },

    /// A download could not be completed.
    #[error("the download from {rabt} failed: {sabab}")]
    TanzeelFashil {
        /// Where it came from.
        rabt: String,
        /// Why.
        sabab: String,
    },

    /// A download exceeded the size its listing declared.
    #[error("the download exceeded its declared {muallan} bytes")]
    HajmMufrit {
        /// The declared size.
        muallan: u64,
    },

    /// A local file or cache entry could not be read or written.
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

    /// No patch in the catalogue applies to the installed build.
    #[error("no patch matches this build")]
    LaMutabaqa,

    /// The destination's filesystem cannot hold the download and its headroom.
    #[error(
        "{masar} needs {} MB free and has only {} MB; clear space or move patch storage",
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
}

impl Tafsir for KhataMustawda {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::MUSTAWDA
                + match self {
                    Self::LaMasdar { .. } => 0,
                    Self::IstijabaFashila { .. } => 1,
                    Self::BayanTalif { .. } => 2,
                    Self::TasalsulLilkhalf { .. } => 3,
                    Self::ShareehaMajhula { .. } => 4,
                    Self::BasmaGhayrMutabaqa { .. } => 5,
                    Self::ShareehaTalifa { .. } => 6,
                    Self::TanzeelGhayrMutabiq { .. } => 7,
                    Self::TanzeelFashil { .. } => 8,
                    Self::HajmMufrit { .. } => 9,
                    Self::KhataMalaf { .. } => 10,
                    Self::LaMutabaqa => 11,
                    Self::MisahaGhayrKafiya { .. } => 12,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Content that did not hash to what the manifest or the listing
            // declared: a corrupted mirror, or a substitution.
            Self::BasmaGhayrMutabaqa { .. }
            | Self::TanzeelGhayrMutabiq { .. }
            | Self::TasalsulLilkhalf { .. } => Khutura::Fadih,
            Self::LaMutabaqa => Khutura::Maluma,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::LaMasdar { .. } => {
                "تعذّر الوصول إلى أيّ مصدر للمستودع. تعمل تعريب دون اتصال بما هو محفوظ لديك."
                    .to_owned()
            },
            Self::IstijabaFashila { .. } => "ردّ المستودع بما لا يمكن قراءته.".to_owned(),
            Self::BayanTalif { .. } => "بيان المستودع غير مقروء بهذه النسخة؛ حدِّث تعريب.".to_owned(),
            Self::TasalsulLilkhalf { .. } => {
                "عُرض بيان أقدم من المحفوظ لديك، ورُفض: قد يكون إرجاعًا مقصودًا لإخفاء إبطال.".to_owned()
            },
            Self::ShareehaMajhula { .. } => "وصلت شريحة فهرس لا يذكرها البيان، ولم تُقرأ.".to_owned(),
            Self::BasmaGhayrMutabaqa { .. } => {
                "بصمة شريحة الفهرس لا تطابق ما يعلنه البيان، ولم يُقرأ منها شيء.".to_owned()
            },
            Self::ShareehaTalifa { .. } => "تعذّرت قراءة شريحة فهرس بعد التحقق منها.".to_owned(),
            Self::TanzeelGhayrMutabiq { .. } => {
                "بصمة الحزمة المنزّلة لا تطابق المعلنة، ولم تُثبَّت.".to_owned()
            },
            Self::TanzeelFashil { .. } => {
                "تعذّر إكمال التنزيل. يمكن استئنافه من حيث توقّف.".to_owned()
            },
            Self::HajmMufrit { .. } => "تجاوز التنزيل الحجم المعلن له، وأُوقف.".to_owned(),
            Self::KhataMalaf { .. } => "تعذّرت قراءة ملف محلّي أو الكتابة إليه.".to_owned(),
            Self::LaMutabaqa => "لا توجد رقعة تطابق نسخة لعبتك الحالية.".to_owned(),
            Self::MisahaGhayrKafiya { matlub, mutah, .. } => format!(
                "لا تكفي المساحة الخالية على القرص: يحتاج التنزيل {} ميغابايت والمتاح {} ميغابايت فقط. أخلِ مساحة أو انقل مخزن الرقع من الإعدادات.",
                mijabayt_matluba(*matlub),
                mijabayt_mutaha(*mutah)
            ),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::LaMasdar { .. } | Self::IstijabaFashila { .. } | Self::TanzeelFashil { .. } => {
                Khutwa::AadaMuhawala
            },
            Self::BayanTalif { .. } => Khutwa::TahdithTaarib,
            Self::TasalsulLilkhalf { .. }
            | Self::BasmaGhayrMutabaqa { .. }
            | Self::TanzeelGhayrMutabiq { .. } => Khutwa::IblaghLilMalik,
            Self::ShareehaMajhula { .. }
            | Self::ShareehaTalifa { .. }
            | Self::HajmMufrit { .. } => Khutwa::FathTashkhis,
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladManassa),
            Self::LaMutabaqa => Khutwa::LaShay,
            Self::MisahaGhayrKafiya { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Takhzin,
            },
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
            Self::KhataMalaf { .. } | Self::LaMutabaqa => {},
            Self::LaMasdar { sabab } | Self::BayanTalif { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()))
            },
            Self::IstijabaFashila { rabt, ramz } => {
                daa("rabt", QeemaSiyaq::Nass(rabt.clone()));
                daa("ramz", QeemaSiyaq::Raqm(i64::from(*ramz)));
            },
            Self::TasalsulLilkhalf { wujid, mukhazzan } => {
                daa("wujid", QeemaSiyaq::Hajm(*wujid));
                daa("mukhazzan", QeemaSiyaq::Hajm(*mukhazzan));
            },
            Self::ShareehaMajhula { raqm } => daa("raqm", QeemaSiyaq::Raqm(i64::from(*raqm))),
            Self::BasmaGhayrMutabaqa {
                raqm,
                muallana,
                mahsuba,
            } => {
                daa("raqm", QeemaSiyaq::Raqm(i64::from(*raqm)));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            },
            Self::ShareehaTalifa { raqm, sabab } => {
                daa("raqm", QeemaSiyaq::Raqm(i64::from(*raqm)));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::TanzeelGhayrMutabiq {
                rabt,
                muallana,
                mahsuba,
            } => {
                daa("rabt", QeemaSiyaq::Nass(rabt.clone()));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            },
            Self::TanzeelFashil { rabt, sabab } => {
                daa("rabt", QeemaSiyaq::Nass(rabt.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::HajmMufrit { muallan } => daa("muallan", QeemaSiyaq::Hajm(*muallan)),
            Self::MisahaGhayrKafiya {
                matlub,
                mutah,
                masar,
            } => {
                daa("matlub", QeemaSiyaq::Hajm(*matlub));
                daa("mutah", QeemaSiyaq::Hajm(*mutah));
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            },
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

khata_min!(KhataMustawda);
