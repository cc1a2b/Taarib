//! The proof of safety an installation cannot begin without.

use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;

/// Authorisation to modify one game with one package.
///
/// No public constructor, no public fields, no `Deserialize`, not `Clone`. The
/// only value the install pipeline accepts, taken by value. Phase 16's
/// detection pipeline is the sole minting site and does not exist yet, so the
/// install path compiles and cannot run — the correct state for a product
/// whose anti-cheat detection is unwritten.
#[derive(Debug)]
pub struct IdhnTathbeet {
    luba: LubaId,
    basmat_ruqaa: Basma,
}

impl IdhnTathbeet {
    /// Mints the proof. The only constructor, called by [`crate::fahs`] after
    /// every refusal check has passed.
    pub(crate) const fn jadeed(luba: LubaId, basmat_ruqaa: Basma) -> Self {
        Self { luba, basmat_ruqaa }
    }

    /// The game this authorisation is for.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// The content hash of the package it authorises.
    #[must_use]
    pub const fn basmat_ruqaa(&self) -> Basma {
        self.basmat_ruqaa
    }

    /// Whether this proof authorises installing `basmat_ruqaa` into `luba`.
    #[must_use]
    pub fn yushmal(&self, luba: LubaId, basmat_ruqaa: Basma) -> bool {
        self.luba == luba && self.basmat_ruqaa == basmat_ruqaa
    }
}
