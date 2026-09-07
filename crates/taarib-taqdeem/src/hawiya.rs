//! Who is who: the owner authority token, and the contributor identity.

use taarib_khatm::{MiftahAam, MiftahKhass};
use taarib_mustalahat::musahim::MusahimId;

/// The challenge the owner's key signs to prove possession.
const TAHADDI: &[u8] = b"taarib-taqdeem/malik/v1\0";

/// Proof that this session holds the owner's signing key.
///
/// No public constructor, no public fields, no `Deserialize`, not `Clone`. The
/// only way to obtain one is [`SalahiyatMalik::bi_miftah`], which requires the
/// owner's private key and verifies its public half against the key compiled
/// into the client. Every review-console and publishing entry point takes one
/// by reference, so a contributor session cannot call them — not because a
/// check refuses, but because the argument cannot be produced.
///
/// Authority is therefore possession of a key, never a flag, a role or a row.
/// There is no grant, no delegation and no second tier: none is written.
#[derive(Debug)]
pub struct SalahiyatMalik {
    miftah_aam: [u8; 32],
}

impl SalahiyatMalik {
    /// Proves ownership by signing a fixed challenge with the owner's key.
    ///
    /// `miftah_malik` is the owner's public key compiled into the client.
    /// Returns [`None`] when `khass` is any other key.
    #[must_use]
    pub fn bi_miftah(khass: &MiftahKhass, miftah_malik: &[u8; 32]) -> Option<Self> {
        let aam = khass.aam();
        if &aam.bayt() != miftah_malik {
            return None;
        }
        // The signature is checked as well as the key equality: a key whose
        // public half matches but which cannot sign is not a usable owner key.
        let tawqee = khass.waqqi(TAHADDI);
        aam.tahaqquq(TAHADDI, &tawqee).then_some(Self {
            miftah_aam: *miftah_malik,
        })
    }

    /// The owner's public key, for recording provenance.
    #[must_use]
    pub const fn miftah_aam(&self) -> &[u8; 32] {
        &self.miftah_aam
    }
}

/// Who a session is acting as.
///
/// Two states and no third. A contributor session carries no field that could
/// become an owner one, and the owner variant carries the proof rather than a
/// boolean, so "is the owner" and "can prove it" are the same question.
#[derive(Debug)]
pub enum Jalsa {
    /// A contributor, identified by their published identity.
    Musahim {
        /// Their identity.
        musahim: MusahimId,
        /// Their signing key's public half, for signing submissions.
        miftah: [u8; 32],
    },
    /// The owner, holding the review authority.
    Malik {
        /// Their identity.
        musahim: MusahimId,
        /// The proof.
        salahiya: SalahiyatMalik,
    },
}

impl Jalsa {
    /// The identity acting in this session.
    #[must_use]
    pub const fn musahim(&self) -> &MusahimId {
        match self {
            Self::Musahim { musahim, .. } | Self::Malik { musahim, .. } => musahim,
        }
    }

    /// The owner authority, when this session has it.
    ///
    /// A contributor session answers [`None`], and every console entry point
    /// needs the value rather than the answer, so there is nothing to bypass.
    #[must_use]
    pub const fn salahiya(&self) -> Option<&SalahiyatMalik> {
        match self {
            Self::Malik { salahiya, .. } => Some(salahiya),
            Self::Musahim { .. } => None,
        }
    }
}

/// A contributor's credit line and identity, as a submission records it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HawiyatMusahim {
    /// Their identity.
    pub musahim: MusahimId,
    /// The display name shown on every listing.
    pub ism: String,
    /// The credit line they want carried with the patch.
    pub itimad: Option<String>,
    /// Their signing key's public half.
    pub miftah: [u8; 32],
}

impl HawiyatMusahim {
    /// Whether a signature over `risala` was made by this contributor's key.
    #[must_use]
    pub fn waqqaa(&self, risala: &[u8], tawqee: &[u8; 64]) -> bool {
        MiftahAam::min_bayt(&self.miftah).is_ok_and(|aam| aam.tahaqquq(risala, tawqee))
    }
}
