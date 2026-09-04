//! Every refusal the hooking layer can produce, in band 4200.

use std::collections::BTreeMap;

use taarib_usus::khata::{Khutura, Khutwa, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

/// The hooking layer's result type.
pub type NatijatHaqn<T> = Result<T, KhataHaqn>;

/// Why a hook could not be installed or removed.
#[derive(Debug, thiserror::Error)]
pub enum KhataHaqn {
    /// A page could not be made writable for the span being patched.
    #[error("the {tul}-byte span at 0x{oinwan:x} could not be made writable")]
    HimayaGhayrQabila {
        /// Where the write was to land.
        oinwan: u64,
        /// How many bytes it covers.
        tul: usize,
    },

    /// The location held something other than what was read a moment earlier.
    #[error("{mawdi} changed between being read and being written: {sabab}")]
    KhatfFashil {
        /// What was being hooked, named for the log.
        mawdi: String,
        /// Which check refused.
        sabab: String,
    },

    /// A write reported success and did not take effect.
    #[error("the write at {mawdi} did not take effect")]
    KitabaMuhmala {
        /// What was being hooked.
        mawdi: String,
    },

    /// Removing a hook failed, so the module cannot be unloaded.
    #[error("{mawdi} could not be unhooked: {sabab}")]
    FakkKhatfFashil {
        /// What was being unhooked.
        mawdi: String,
        /// Every reason, joined.
        sabab: String,
    },

    /// The trampoline builder refused the target's prologue.
    #[error("a detour could not be built at 0x{oinwan:x}: {sabab}")]
    MasarGhayrQabil {
        /// The function that was to be detoured.
        oinwan: u64,
        /// What the builder reported.
        sabab: String,
    },

    /// A named import was not found in the module's import table.
    #[error("{wahda}!{ism} is not imported by the module at 0x{qaida:x}")]
    IstiradMafqud {
        /// The library the import comes from.
        wahda: String,
        /// The imported symbol.
        ism: String,
        /// The importing module's base address.
        qaida: u64,
    },

    /// The module's headers are not a shape this reader understands.
    #[error("the module at 0x{qaida:x} could not be read: {sabab}")]
    RaasGhayrMafhum {
        /// The module's base address.
        qaida: u64,
        /// Which structural check refused.
        sabab: String,
    },

    /// An operation that only one platform has.
    #[error("{amal} is not available on this platform")]
    GhayrMutahaHuna {
        /// What was attempted.
        amal: &'static str,
    },
}

impl Tafsir for KhataHaqn {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::HAQN
                + match self {
                    Self::HimayaGhayrQabila { .. } => 0,
                    Self::KhatfFashil { .. } => 1,
                    Self::KitabaMuhmala { .. } => 2,
                    Self::FakkKhatfFashil { .. } => 3,
                    Self::MasarGhayrQabil { .. } => 4,
                    Self::IstiradMafqud { .. } => 5,
                    Self::RaasGhayrMafhum { .. } => 6,
                    Self::GhayrMutahaHuna { .. } => 7,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The game is untouched; the capability declined.
            Self::HimayaGhayrQabila { .. }
            | Self::KhatfFashil { .. }
            | Self::KitabaMuhmala { .. }
            | Self::MasarGhayrQabil { .. }
            | Self::IstiradMafqud { .. }
            | Self::RaasGhayrMafhum { .. }
            | Self::GhayrMutahaHuna { .. } => Khutura::Tanbeeh,
            // Something is still installed in a process that wanted to let go.
            Self::FakkKhatfFashil { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::HimayaGhayrQabila { .. } => "تعذّر جعل موضع في ذاكرة اللعبة قابلًا للكتابة، \
                 وغالبًا سببه برنامج حماية يمنع التعديل. تعمل اللعبة كما هي بلا تعريب."
                .to_owned(),
            Self::KhatfFashil { .. } => "تغيّر موضع الاعتراض في اللحظة نفسها، ما يعني أن برنامجًا \
                 آخر اعترضه في الوقت ذاته. لم يُعدَّل شيء، واللعبة تعمل كما هي."
                .to_owned(),
            Self::KitabaMuhmala { .. } => "قُبلت الكتابة ولم تأخذ مفعولها، وهذا ما يفعله برنامج \
                 حماية يعمل تحت النظام. تعمل اللعبة كما هي بلا تعريب."
                .to_owned(),
            Self::FakkKhatfFashil { .. } => "تعذّر رفع اعتراض تعريب من اللعبة، فلا يمكن إخراجه \
                 من العملية الآن. أغلق اللعبة وشغّلها من جديد."
                .to_owned(),
            Self::MasarGhayrQabil { .. } => "تعذّر بناء معبر آمن إلى دالة في اللعبة، فبدايتها لا \
                 تُنقل بأمان. تُترك هذه الدالة كما هي وتعمل اللعبة بلا تعريب لهذا الجزء."
                .to_owned(),
            Self::IstiradMafqud { wahda, ism, .. } => format!(
                "لا تستورد الوحدة الدالة {ism} من {wahda}، فلا موضع لاعتراضها. لم يُعدَّل شيء."
            ),
            Self::RaasGhayrMafhum { .. } => "ترويسة وحدة اللعبة ليست بالشكل الذي يقرؤه تعريب، \
                 فلا يمكن تحديد جدول استيرادها. لم يُعدَّل شيء."
                .to_owned(),
            Self::GhayrMutahaHuna { .. } => "هذه العملية غير متاحة على هذا النظام؛ يستخدم تعريب \
                 طريقة أخرى عليه."
                .to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // Nothing the user does changes an anti-tamper driver's mind, and
            // nothing was written, so the honest next step is the log.
            Self::FakkKhatfFashil { .. } => Khutwa::FathTashkhis,
            _ => Khutwa::LaShay,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::HimayaGhayrQabila { oinwan, tul } => {
                // Addresses are unsigned and the context field is signed; the
                // hex rendering in the sentence is the authority either way.
                let _ = siyaq
                    .insert("oinwan".to_owned(), QeemaSiyaq::Nass(format!("0x{oinwan:x}")));
                let _ = siyaq
                    .insert("tul".to_owned(), QeemaSiyaq::Hajm(u64::try_from(*tul).unwrap_or(0)));
            }
            Self::KhatfFashil { mawdi, .. }
            | Self::KitabaMuhmala { mawdi }
            | Self::FakkKhatfFashil { mawdi, .. } => {
                let _ = siyaq.insert("mawdi".to_owned(), QeemaSiyaq::Nass(mawdi.clone()));
            }
            Self::MasarGhayrQabil { oinwan, .. } => {
                let _ = siyaq
                    .insert("oinwan".to_owned(), QeemaSiyaq::Nass(format!("0x{oinwan:x}")));
            }
            Self::IstiradMafqud { wahda, ism, qaida } => {
                let _ = siyaq.insert("wahda".to_owned(), QeemaSiyaq::Nass(wahda.clone()));
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq
                    .insert("qaida".to_owned(), QeemaSiyaq::Nass(format!("0x{qaida:x}")));
            }
            Self::RaasGhayrMafhum { qaida, .. } => {
                let _ = siyaq
                    .insert("qaida".to_owned(), QeemaSiyaq::Nass(format!("0x{qaida:x}")));
            }
            Self::GhayrMutahaHuna { amal } => {
                let _ =
                    siyaq.insert("amal".to_owned(), QeemaSiyaq::Nass((*amal).to_owned()));
            }
        }
        siyaq
    }
}

khata_min!(KhataHaqn);
