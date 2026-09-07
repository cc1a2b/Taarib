//! أخطاء الكشف — what can go wrong while finding somebody's games.
//!
//! Almost nothing here is fatal, and that is the design. A launcher that is not
//! installed is not an error; a manifest that will not parse is a warning on the
//! result; a game whose directory has been deleted is a warning. The variants
//! below are for the narrow set of failures that make a whole launcher
//! unreadable or that would leave the library in a state a user cannot act on —
//! and every one of them names the file it choked on, because "no games found"
//! with no path in it is a bug report nobody can answer.

use std::collections::BTreeMap;
use std::path::PathBuf;

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam, khutwa_io, siyaq_io,
};
use taarib_usus::khata_min;

/// Failures of discovery.
#[derive(Debug, thiserror::Error)]
pub enum KhataKashf {
    /// A launcher's catalogue file exists but cannot be read at all.
    #[error("cannot read {matjar} catalogue at {masar}")]
    TaadhurQiraatFahras {
        /// Which launcher.
        matjar: &'static str,
        /// The file.
        masar: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A catalogue's own header or top-level structure is not what that format
    /// declares, so nothing inside it can be trusted.
    ///
    /// Narrower than it sounds: a single malformed *entry* is a warning, not
    /// this. This is for a file whose framing is wrong — a binary VDF with a
    /// bad magic number, a protobuf whose length prefix runs past the end.
    #[error("{matjar} catalogue at {masar} is not in the expected format: {tafsil}")]
    TarwisatFahrasTalifa {
        /// Which launcher.
        matjar: &'static str,
        /// The file.
        masar: PathBuf,
        /// What was wrong with the framing.
        tafsil: String,
        /// Where in the file, when the reader knows.
        mawdi: Option<u64>,
    },

    /// A compatibility prefix a game declares does not exist.
    ///
    /// The game is installed and its launcher points at a prefix that is not
    /// there, which usually means the prefix was deleted by hand or the game was
    /// moved between machines. The game cannot be patched until it runs once and
    /// its launcher rebuilds the prefix.
    #[error("compatibility prefix missing: {masar}")]
    BeeaMafquda {
        /// Where the prefix was expected.
        masar: PathBuf,
    },

    /// A path inside a compatibility prefix does not map to anything real.
    #[error("cannot map {masar} through the prefix at {beea}")]
    TaadhurTarjamatMasar {
        /// The path as the game sees it.
        masar: String,
        /// The prefix root.
        beea: PathBuf,
    },

    /// A prefix's registry file could not be read, so DLL overrides and drive
    /// mappings are unknown.
    #[error("cannot read prefix registry {masar}")]
    TaadhurQiraatSijillBeea {
        /// The registry file.
        masar: PathBuf,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// Artwork could not be fetched.
    ///
    /// Never fatal to a scan: the game appears with the placeholder colour and
    /// no image, which is a library that works rather than a library that failed.
    #[error("cannot fetch artwork from {rabt}")]
    TaadhurJalbSura {
        /// The address.
        rabt: String,
        /// What the transport reported.
        tafsil: String,
    },

    /// Artwork was fetched but is not an image this build can decode.
    #[error("artwork from {rabt} is not a decodable image")]
    SuraGhayrSaliha {
        /// The address or path.
        rabt: String,
        /// What the decoder reported.
        tafsil: String,
    },

    /// A manually added path holds nothing that looks like a game.
    #[error("no executable found under {jidhr}")]
    LaYujadTanfidhi {
        /// The directory the user pointed at.
        jidhr: PathBuf,
    },

    /// A filesystem watch could not be established, so refresh falls back to
    /// scanning on demand.
    #[error("cannot watch {masar} for changes")]
    TaadhurMuraqaba {
        /// The directory.
        masar: PathBuf,
        /// What the watcher reported.
        tafsil: String,
    },

    /// A launcher root was configured by the user and is not there.
    ///
    /// Distinguished from "not installed" on purpose: the user told Taarib where
    /// to look, and the answer is that nothing is there. Silently falling back to
    /// automatic detection would hide their mistake.
    #[error("configured {matjar} location does not exist: {masar}")]
    JidhrMuhaddadMafqud {
        /// Which launcher.
        matjar: &'static str,
        /// The configured path.
        masar: PathBuf,
    },
}

impl Tafsir for KhataKashf {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::KASHF
                + match self {
                    Self::TaadhurQiraatFahras { .. } => 0,
                    Self::TarwisatFahrasTalifa { .. } => 1,
                    Self::BeeaMafquda { .. } => 2,
                    Self::TaadhurTarjamatMasar { .. } => 3,
                    Self::TaadhurQiraatSijillBeea { .. } => 4,
                    Self::TaadhurJalbSura { .. } => 5,
                    Self::SuraGhayrSaliha { .. } => 6,
                    Self::LaYujadTanfidhi { .. } => 7,
                    Self::TaadhurMuraqaba { .. } => 8,
                    Self::JidhrMuhaddadMafqud { .. } => 9,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::TaadhurJalbSura { .. }
            | Self::SuraGhayrSaliha { .. }
            | Self::TaadhurMuraqaba { .. } => Khutura::Tanbeeh,
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::TaadhurQiraatFahras { matjar, .. } => format!(
                "تعذّرت قراءة سجلّ الألعاب الخاص بـ{matjar}. قد يكون المتجر قيد التحديث، أو \
                 على قرص غير متصل."
            ),
            Self::TarwisatFahrasTalifa { matjar, .. } => format!(
                "سجلّ ألعاب {matjar} ليس بالصيغة المتوقّعة. قد يكون تالفًا، أو من إصدار لا \
                 يعرفه تعريب بعد."
            ),
            Self::BeeaMafquda { .. } => {
                "بيئة التوافق الخاصة بهذه اللعبة غير موجودة. شغّل اللعبة مرة واحدة ليعيد \
                 المتجر بناءها، ثم أعد الفحص."
                    .to_owned()
            },
            Self::TaadhurTarjamatMasar { masar, .. } => {
                format!("تعذّر تحويل المسار {masar} من داخل بيئة التوافق إلى مسار حقيقي.")
            },
            Self::TaadhurQiraatSijillBeea { .. } => {
                "تعذّرت قراءة سجلّ بيئة التوافق، ولا يمكن معرفة إعداداتها.".to_owned()
            },
            Self::TaadhurJalbSura { .. } => {
                "تعذّر جلب صورة اللعبة. ستظهر اللعبة بلون بديل حتى تتوفّر الصورة.".to_owned()
            },
            Self::SuraGhayrSaliha { .. } => {
                "صورة اللعبة بصيغة لا يمكن قراءتها، وسيُستخدم لون بديل.".to_owned()
            },
            Self::LaYujadTanfidhi { jidhr } => format!(
                "لا يوجد ملف تنفيذي داخل {}. اختر المجلد الذي يحتوي ملف تشغيل اللعبة.",
                jidhr.display()
            ),
            Self::TaadhurMuraqaba { .. } => {
                "تعذّرت مراقبة مجلد الألعاب للتغيّرات؛ سيُحدَّث الفحص عند طلبك فقط.".to_owned()
            },
            Self::JidhrMuhaddadMafqud { matjar, masar } => format!(
                "المسار الذي حدّدته لـ{matjar} غير موجود: {}. صحّحه في الإعدادات أو احذفه \
                 ليبحث تعريب عنه تلقائيًا.",
                masar.display()
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TaadhurQiraatFahras { matjar, .. } => format!(
                "Cannot read the {matjar} game catalogue. The launcher may be updating, or its \
                 library may be on a disconnected drive."
            ),
            Self::TarwisatFahrasTalifa { matjar, tafsil, .. } => format!(
                "The {matjar} catalogue is not in the expected format ({tafsil}). It may be \
                 corrupt, or from a version Taarib does not know yet."
            ),
            Self::BeeaMafquda { .. } => {
                "This game's compatibility prefix does not exist. Run the game once so the \
                 launcher rebuilds it, then rescan."
                    .to_owned()
            },
            Self::TaadhurTarjamatMasar { masar, .. } => {
                format!("Cannot map {masar} from inside the compatibility prefix to a real path.")
            },
            Self::TaadhurQiraatSijillBeea { .. } => {
                "Cannot read the compatibility prefix registry, so its settings are unknown."
                    .to_owned()
            },
            Self::TaadhurJalbSura { .. } => {
                "Cannot fetch this game's artwork. It will show a placeholder colour until the \
                 image is available."
                    .to_owned()
            },
            Self::SuraGhayrSaliha { .. } => {
                "This game's artwork is in a format that cannot be decoded; a placeholder colour \
                 will be used."
                    .to_owned()
            },
            Self::LaYujadTanfidhi { jidhr } => format!(
                "No executable under {}. Choose the folder that contains the game's launcher.",
                jidhr.display()
            ),
            Self::TaadhurMuraqaba { .. } => {
                "Cannot watch the library folder for changes; the scan will refresh only when you \
                 ask it to."
                    .to_owned()
            },
            Self::JidhrMuhaddadMafqud { matjar, masar } => format!(
                "The {matjar} location you set does not exist: {}. Correct it in Settings, or \
                 clear it so Taarib finds the launcher itself.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::TaadhurQiraatFahras { sabab, .. } => {
                khutwa_io(sabab, MasarMatlub::MujalladManassa)
            },
            Self::TarwisatFahrasTalifa { .. } | Self::TaadhurQiraatSijillBeea { .. } => {
                Khutwa::FathTashkhis
            },
            Self::BeeaMafquda { .. } => Khutwa::AadaFahsMaktaba,
            Self::TaadhurTarjamatMasar { .. } => Khutwa::FathTashkhis,
            Self::TaadhurJalbSura { .. } | Self::TaadhurMuraqaba { .. } => Khutwa::AadaMuhawala,
            Self::SuraGhayrSaliha { .. } => Khutwa::LaShay,
            Self::LaYujadTanfidhi { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            Self::JidhrMuhaddadMafqud { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Manassat,
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TaadhurQiraatFahras {
                matjar,
                masar,
                sabab,
            } => {
                let _ = siyaq.insert("matjar".to_owned(), QeemaSiyaq::Nass((*matjar).to_owned()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_io(sabab));
            },
            Self::TarwisatFahrasTalifa {
                matjar,
                masar,
                tafsil,
                mawdi,
            } => {
                let _ = siyaq.insert("matjar".to_owned(), QeemaSiyaq::Nass((*matjar).to_owned()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
                if let Some(mawdi) = mawdi {
                    let _ = siyaq.insert("mawdi".to_owned(), QeemaSiyaq::Hajm(*mawdi));
                }
            },
            Self::BeeaMafquda { masar } => {
                let _ = siyaq.insert("beea".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            },
            Self::TaadhurTarjamatMasar { masar, beea } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
                let _ = siyaq.insert("beea".to_owned(), QeemaSiyaq::Masar(beea.clone()));
            },
            Self::TaadhurQiraatSijillBeea { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                siyaq.extend(siyaq_io(sabab));
            },
            Self::TaadhurJalbSura { rabt, tafsil } | Self::SuraGhayrSaliha { rabt, tafsil } => {
                let _ = siyaq.insert("rabt".to_owned(), QeemaSiyaq::Nass(rabt.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::LaYujadTanfidhi { jidhr } => {
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            },
            Self::TaadhurMuraqaba { masar, tafsil } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::JidhrMuhaddadMafqud { matjar, masar } => {
                let _ = siyaq.insert("matjar".to_owned(), QeemaSiyaq::Nass((*matjar).to_owned()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataKashf);
