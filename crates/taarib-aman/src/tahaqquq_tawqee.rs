//! Signature verification against the compiled trust anchor, with no override.

use taarib_khatm::{HawiyatThiqa, MIFTAH_TATWIR, MirsatThiqa, MudaqqiqEd25519};
use taarib_mustalahat::bina::Basma;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_ruqaa::tawqee::DawrMiftah;

/// Why a package's signature was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SababTawqee {
    /// The package carries no signature.
    GhayrMuwaqqaa,
    /// The package was signed by a key that is not the owner's.
    MiftahMajhul {
        /// The key that signed it, hex.
        miftah: String,
    },
    /// The package was signed by the committed development key, and this is a
    /// release build that names and refuses that key specifically.
    TawqeeTatwir,
    /// The package was self-signed by a contributor and not yet countersigned
    /// by the owner.
    MusahimFaqat,
    /// The signature does not verify over the package's content hash.
    TawqeeGhayrSalih,
    /// The package's framing or content hash is itself invalid.
    HuzmaTalifa {
        /// What the container reader said.
        sabab: String,
    },
}

impl SababTawqee {
    /// The sentence shown to the user, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::GhayrMuwaqqaa => "الحزمة غير موقّعة، ولا يُثبّت تعريب حزمة غير موقّعة.".to_owned(),
            Self::MiftahMajhul { .. } => {
                "وقّعت الحزمة بمفتاح ليس مفتاح المالك المضمّن في البرنامج.".to_owned()
            }
            Self::TawqeeTatwir => {
                "الحزمة موقّعة بمفتاح التطوير المعلن، وهذه نسخة إصدار ترفضه بالاسم.".to_owned()
            }
            Self::MusahimFaqat => {
                "الحزمة موقّعة من المساهم ولم يعتمدها المالك بعد.".to_owned()
            }
            Self::TawqeeGhayrSalih => {
                "توقيع الحزمة لا يطابق محتواها؛ رُبّما عُدّلت بعد توقيعها.".to_owned()
            }
            Self::HuzmaTalifa { .. } => "بنية الحزمة أو بصمتها غير سليمة.".to_owned(),
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::GhayrMuwaqqaa => "the package is unsigned".to_owned(),
            Self::MiftahMajhul { miftah } => {
                format!("the package was signed by {miftah}, not the owner's embedded key")
            }
            Self::TawqeeTatwir => {
                "the package is signed by the published development key, which a release build \
                 refuses by name"
                    .to_owned()
            }
            Self::MusahimFaqat => {
                "the package is contributor-self-signed and not yet owner-approved".to_owned()
            }
            Self::TawqeeGhayrSalih => {
                "the signature does not verify over the package's content hash".to_owned()
            }
            Self::HuzmaTalifa { sabab } => format!("the package framing is invalid: {sabab}"),
        }
    }
}

/// Verifies a package against the compiled trust anchor.
///
/// There is no parameter that disables this and no path that skips it: a caller
/// holding a [`MalafRuqaa`] either gets `Ok(basma)` — the verified content hash,
/// which the caller binds an authorisation to — or the specific reason.
///
/// `mirsa` is the anchor compiled into the client. A package must be signed by
/// exactly its key; a contributor self-signature is
/// [`SababTawqee::MusahimFaqat`], an unsigned block is
/// [`SababTawqee::GhayrMuwaqqaa`], the development key under a release anchor
/// is [`SababTawqee::TawqeeTatwir`] by name, and any other key is
/// [`SababTawqee::MiftahMajhul`].
///
/// # Errors
///
/// [`SababTawqee`] naming which check refused. The content hash is validated by
/// [`taarib_ruqaa::qari::Ruqaa::iftah`] before the signature is examined, so a
/// tampered container is rejected before its signature is trusted.
pub fn tahaqquq(malaf: &MalafRuqaa, mirsa: &MirsatThiqa) -> Result<Basma, SababTawqee> {
    let ruqaa = malaf.ruqaa().map_err(|khata| SababTawqee::HuzmaTalifa {
        sabab: khata.to_string(),
    })?;
    let kutla = ruqaa.tawqee();
    if !kutla.muwaqqaa() {
        return Err(SababTawqee::GhayrMuwaqqaa);
    }
    if kutla.miftah != mirsa.miftah {
        // Only a release build may name this refusal, because the refusal's own
        // sentence asserts that it is one. A development build reaches here too
        // whenever the anchor is run-local rather than `MIRSAT_MALIK` — the
        // Phase 26 one-button install anchors to the key that just signed the
        // package — and answering it with a release build's wording would state
        // something untrue about the build the user is holding. It falls
        // through to the ordinary unknown-key arms instead.
        if kutla.miftah == MIFTAH_TATWIR && matches!(mirsa.hawiya, HawiyatThiqa::Isdar) {
            return Err(SababTawqee::TawqeeTatwir);
        }
        return match kutla.dawr {
            DawrMiftah::Musahim => Err(SababTawqee::MusahimFaqat),
            DawrMiftah::Malik => Err(SababTawqee::MiftahMajhul {
                miftah: hex::encode(kutla.miftah),
            }),
        };
    }

    let basma = ruqaa.tarwisa().basma;
    kutla
        .tahaqquq(&basma, &MudaqqiqEd25519)
        .map_err(|_| SababTawqee::TawqeeGhayrSalih)?;
    Ok(Basma::min_bayt(basma))
}
