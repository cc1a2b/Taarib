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

    /// The community index is not one this build reads.
    #[error("the community translations index could not be read: {sabab}")]
    FahrasMujtamaTalif {
        /// Why.
        sabab: String,
    },

    /// The community index is larger than any this build will hold.
    #[error("the community translations index is {hajm} bytes, over the {hadd} this build accepts")]
    FahrasMujtamaKabir {
        /// The size served.
        hajm: u64,
        /// The cap.
        hadd: u64,
    },

    /// No source served the community index and no copy is cached.
    #[error("the community translations index could not be fetched and none is cached: {sabab}")]
    FahrasMujtamaGhayrMutah {
        /// What the last attempt reported.
        sabab: String,
    },

    /// The community index carries no owner signature at all.
    ///
    /// Either a revision cast before this scheme existed — the copy a machine
    /// upgrading from 1.0.1 still holds in its cache — or a source serving a
    /// body with the signature stripped off it. The two are the same refusal
    /// because a client cannot tell them apart and must not try: an index
    /// nobody signed decides nothing here, whatever its provenance.
    #[error(
        "the community translations index carries no owner signature, and nothing was read from it"
    )]
    FahrasMujtamaGhayrMuwaqqa,

    /// The community index carries a signature that does not verify.
    #[error(
        "the community translations index signature was refused, and nothing was read from it: \
         {sabab}"
    )]
    FahrasMujtamaTawqeeBatil {
        /// Which check refused it.
        sabab: String,
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
                    Self::FahrasMujtamaTalif { .. } => 13,
                    Self::FahrasMujtamaKabir { .. } => 14,
                    Self::FahrasMujtamaGhayrMutah { .. } => 15,
                    Self::FahrasMujtamaGhayrMuwaqqa => 16,
                    Self::FahrasMujtamaTawqeeBatil { .. } => 17,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Content that did not hash to, or verify against, what the owner
            // vouched for: a corrupted mirror, or a substitution. A community
            // index whose signature fails is judged here and not with the rest
            // of its family — nothing installs from that index, but its
            // addresses are what the browser-open command is allowed to reach,
            // and a body that no longer matches its signature is somebody
            // choosing them.
            Self::BasmaGhayrMutabaqa { .. }
            | Self::TanzeelGhayrMutabiq { .. }
            | Self::TasalsulLilkhalf { .. }
            | Self::FahrasMujtamaTawqeeBatil { .. } => Khutura::Fadih,
            Self::LaMutabaqa => Khutura::Maluma,
            // Nothing installs from the community index, so losing it costs a
            // credit the screen cannot show and nothing the product does. An
            // unsigned one is the ordinary state of a cache written by an older
            // build, which the next refresh replaces.
            Self::FahrasMujtamaTalif { .. }
            | Self::FahrasMujtamaKabir { .. }
            | Self::FahrasMujtamaGhayrMutah { .. }
            | Self::FahrasMujtamaGhayrMuwaqqa => Khutura::Tanbeeh,
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
            Self::FahrasMujtamaTalif { .. } => {
                "تعذّرت قراءة فهرس تعريبات المجتمع كما وصل من المستودع. أعِد المحاولة لاحقًا، وإن \
                 تكرّر ذلك فحدِّث تعريب."
                    .to_owned()
            },
            Self::FahrasMujtamaKabir { .. } => {
                "فهرس تعريبات المجتمع أكبر مما تقبله هذه النسخة من تعريب، ولم يُقرأ.".to_owned()
            },
            Self::FahrasMujtamaGhayrMutah { .. } => {
                "تعذّر جلب فهرس تعريبات المجتمع من أيّ مصدر، ولا نسخة محفوظة منه لديك. أعِد \
                 المحاولة عند توفّر الاتصال."
                    .to_owned()
            },
            Self::FahrasMujtamaGhayrMuwaqqa => {
                "فهرس تعريبات المجتمع لا يحمل توقيع المالك، فلم تُقرأ منه أيّ مشاركة. غالبًا نسخة \
                 قديمة محفوظة من إصدار سابق لتعريب؛ أعِد المحاولة عند توفّر الاتصال ليُجلب الفهرس \
                 الموقَّع ويحلّ محلّها."
                    .to_owned()
            },
            Self::FahrasMujtamaTawqeeBatil { .. } => {
                "توقيع فهرس تعريبات المجتمع لا يطابق مفتاح المالك في هذه النسخة، ولم يُقرأ منه \
                 شيء ولم يُفتح منه رابط. قد يكون الفهرس قد عُدِّل بعد توقيعه أو وقّعه مفتاح آخر."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The community index joins the transient class: a mirror lagging
            // behind the forge, or a source that is down, both resolve by
            // asking again. An index over the cap does not, and sits with the
            // other answer nothing can act on. An unsigned index joins the
            // transient class too: on an upgraded machine it is the cache an
            // older build wrote, and asking again is what replaces it.
            Self::LaMasdar { .. }
            | Self::IstijabaFashila { .. }
            | Self::TanzeelFashil { .. }
            | Self::FahrasMujtamaTalif { .. }
            | Self::FahrasMujtamaGhayrMutah { .. }
            | Self::FahrasMujtamaGhayrMuwaqqa => Khutwa::AadaMuhawala,
            Self::BayanTalif { .. } => Khutwa::TahdithTaarib,
            Self::TasalsulLilkhalf { .. }
            | Self::BasmaGhayrMutabaqa { .. }
            | Self::TanzeelGhayrMutabiq { .. }
            | Self::FahrasMujtamaTawqeeBatil { .. } => Khutwa::IblaghLilMalik,
            Self::ShareehaMajhula { .. }
            | Self::ShareehaTalifa { .. }
            | Self::HajmMufrit { .. } => Khutwa::FathTashkhis,
            // Every local file this crate touches is inside the patch store or
            // an offline share, never a launcher's install; asking for a
            // launcher location was a default nothing here had chosen.
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladRuqaa),
            // Nothing in the catalogue fits this build, which is what the
            // automatic pipeline is for: it translates the game in front of
            // the user rather than waiting for somebody to publish a match.
            Self::LaMutabaqa => Khutwa::FathTilqai,
            // A cap a Taarib release moves, like every other cap here.
            Self::FahrasMujtamaKabir { .. } => Khutwa::TahdithTaarib,
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
            Self::KhataMalaf { .. } | Self::LaMutabaqa | Self::FahrasMujtamaGhayrMuwaqqa => {},
            Self::LaMasdar { sabab }
            | Self::BayanTalif { sabab }
            | Self::FahrasMujtamaTalif { sabab }
            | Self::FahrasMujtamaGhayrMutah { sabab }
            | Self::FahrasMujtamaTawqeeBatil { sabab } => {
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::FahrasMujtamaKabir { hajm, hadd } => {
                daa("hajm", QeemaSiyaq::Hajm(*hajm));
                daa("hadd", QeemaSiyaq::Hajm(*hadd));
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
