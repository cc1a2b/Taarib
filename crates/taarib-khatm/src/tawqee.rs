//! Ed25519 signing and strict verification over the patch signature message.

use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use taarib_ruqaa::tawqee::MudaqqiqTawqee;

use crate::khata::KhataKhatm;

/// Verifies patch signatures with Ed25519.
///
/// `verify_strict` rejects small-order and non-canonical keys, which a
/// permissive check would accept from a hostile signature block.
#[derive(Debug, Clone, Copy, Default)]
pub struct MudaqqiqEd25519;

impl MudaqqiqTawqee for MudaqqiqEd25519 {
    fn tahaqquq(&self, miftah: &[u8; 32], risala: &[u8], tawqee: &[u8; 64]) -> bool {
        let Ok(mafateeh) = VerifyingKey::from_bytes(miftah) else {
            return false;
        };
        let tawqee = Signature::from_bytes(tawqee);
        mafateeh.verify_strict(risala, &tawqee).is_ok()
    }
}

/// An Ed25519 verifying key.
#[derive(Debug, Clone)]
pub struct MiftahAam {
    mafateeh: VerifyingKey,
}

impl MiftahAam {
    /// Wraps 32 raw key bytes.
    ///
    /// # Errors
    ///
    /// [`KhataKhatm::MiftahTalif`] when the bytes are not a canonical Ed25519
    /// point.
    pub fn min_bayt(bayt: &[u8; 32]) -> Result<Self, KhataKhatm> {
        VerifyingKey::from_bytes(bayt)
            .map(|mafateeh| Self { mafateeh })
            .map_err(|_| KhataKhatm::MiftahTalif)
    }

    /// The raw 32-byte encoding.
    #[must_use]
    pub fn bayt(&self) -> [u8; 32] {
        self.mafateeh.to_bytes()
    }

    /// Whether `tawqee` is a valid signature over `risala` under this key.
    #[must_use]
    pub fn tahaqquq(&self, risala: &[u8], tawqee: &[u8; 64]) -> bool {
        let tawqee = Signature::from_bytes(tawqee);
        self.mafateeh.verify_strict(risala, &tawqee).is_ok()
    }
}

/// An Ed25519 signing key.
///
/// Zeroized on drop by `ed25519-dalek`'s own `SigningKey`.
#[derive(Debug)]
pub struct MiftahKhass {
    tawqee: SigningKey,
}

impl MiftahKhass {
    /// Wraps 32 raw seed bytes.
    #[must_use]
    pub fn min_bayt(bayt: &[u8; 32]) -> Self {
        Self { tawqee: SigningKey::from_bytes(bayt) }
    }

    /// The 32-byte seed.
    #[must_use]
    pub fn bayt(&self) -> [u8; 32] {
        self.tawqee.to_bytes()
    }

    /// The matching verifying key.
    #[must_use]
    pub fn aam(&self) -> MiftahAam {
        MiftahAam { mafateeh: self.tawqee.verifying_key() }
    }

    /// Signs a message.
    #[must_use]
    pub fn waqqi(&self, risala: &[u8]) -> [u8; 64] {
        use ed25519_dalek::Signer as _;
        self.tawqee.sign(risala).to_bytes()
    }
}
