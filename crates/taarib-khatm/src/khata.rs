//! Failures of hashing, signing, verification and key custody.

use std::collections::BTreeMap;

use taarib_usus::khata::{Khutura, Khutwa, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

/// Failures of the sealing layer.
#[derive(Debug, thiserror::Error)]
pub enum KhataKhatm {
    /// A verifying key was not a canonical Ed25519 point.
    #[error("the key is not a valid Ed25519 public key")]
    MiftahTalif,

    /// A signing key could not be generated, stored or retrieved.
    #[error("a signing key could not be {amal}: {sabab}")]
    KhataMiftah {
        /// What was being attempted.
        amal: &'static str,
        /// What the keychain said.
        sabab: String,
    },

    /// The keychain held no key under the requested name.
    #[error("no signing key is stored under {ism}")]
    MiftahMafqud {
        /// The key name.
        ism: String,
    },

    /// A stored key was the wrong length to be an Ed25519 seed.
    #[error("the stored key material is {tul} bytes, not the 32 an Ed25519 seed is")]
    MaddaTalifa {
        /// The length found.
        tul: usize,
    },
}

impl Tafsir for KhataKhatm {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::KHATM
                + match self {
                    Self::MiftahTalif => 0,
                    Self::KhataMiftah { .. } => 1,
                    Self::MiftahMafqud { .. } => 2,
                    Self::MaddaTalifa { .. } => 3,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        Khutura::Khatar
    }

    fn arabi(&self) -> String {
        match self {
            Self::MiftahTalif => "المفتاح ليس مفتاح Ed25519 عامًّا صالحًا.".to_owned(),
            Self::KhataMiftah { .. } => "تعذّر التعامل مع مفتاح التوقيع في خزنة النظام.".to_owned(),
            Self::MiftahMafqud { .. } => "لا يوجد مفتاح توقيع محفوظ بهذا الاسم.".to_owned(),
            Self::MaddaTalifa { .. } => "مادة المفتاح المحفوظة ليست بطول بذرة Ed25519.".to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        self.to_string()
    }

    fn khutwa(&self) -> Khutwa {
        Khutwa::FathTashkhis
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            Self::MiftahTalif => {},
            Self::KhataMiftah { amal, sabab } => {
                daa("amal", QeemaSiyaq::Nass((*amal).to_owned()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MiftahMafqud { ism } => daa("ism", QeemaSiyaq::Nass(ism.clone())),
            Self::MaddaTalifa { tul } => {
                daa(
                    "tul",
                    QeemaSiyaq::Hajm(u64::try_from(*tul).unwrap_or(u64::MAX)),
                );
            },
        }
        siyaq
    }
}

khata_min!(KhataKhatm);
