//! The proof of safety an installation cannot begin without.

use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::ruqaa::RuqaaId;

use crate::kharijiya::{QitaatTanzeel, TahdheerKhariji};

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

/// Authorisation to install one third-party patch into one game.
///
/// The same shape and the same rules as [`IdhnTathbeet`] — no public
/// constructor, no public fields, no `Deserialize`, not `Clone`, taken by value
/// by the install — and a separate type because the two authorise different
/// things and must not be interchangeable. [`IdhnTathbeet`] vouches for a sealed
/// `.ruqaa` by its content hash against the owner's key; there is no signature
/// anywhere in a third-party patch, so what this vouches for instead is the pin:
/// the exact artifacts, by name, size and sha256, that the gate approved.
///
/// Carrying the pins rather than a single hash is what closes the gap between
/// the gate and the install. The gate reads an entry, decides, and mints this;
/// the install re-checks every staged file against what is in here before it
/// opens a manifest. An entry edited in between authorises nothing, because the
/// install compares against this and not against the entry it was handed.
///
/// [`IdhnTathbeetKhariji::tahdheerat`] is the warnings the person was actually
/// shown, kept so the install can record them beside the manifest rather than
/// re-derive a sentence nobody agreed to.
#[derive(Debug)]
pub struct IdhnTathbeetKhariji {
    luba: LubaId,
    ruqaa: RuqaaId,
    isdar: String,
    qitaa: Vec<QitaatTanzeel>,
    tahdheerat: Vec<TahdheerKhariji>,
    mira: bool,
}

impl IdhnTathbeetKhariji {
    /// Mints the proof. The only constructor, called by
    /// [`crate::fahs_khariji::fahs_khariji`] after every refusal check has
    /// passed.
    pub(crate) const fn jadeed(
        luba: LubaId,
        ruqaa: RuqaaId,
        isdar: String,
        qitaa: Vec<QitaatTanzeel>,
        tahdheerat: Vec<TahdheerKhariji>,
        mira: bool,
    ) -> Self {
        Self {
            luba,
            ruqaa,
            isdar,
            qitaa,
            tahdheerat,
            mira,
        }
    }

    /// The game this authorisation is for.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// The catalogue entry it authorises.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The author's version string for the release it covers.
    #[must_use]
    pub fn isdar(&self) -> &str {
        &self.isdar
    }

    /// The artifacts it approved, each with the pin the install must reproduce.
    #[must_use]
    pub fn qitaa(&self) -> &[QitaatTanzeel] {
        &self.qitaa
    }

    /// The warnings the person was shown and acknowledged.
    #[must_use]
    pub fn tahdheerat(&self) -> &[TahdheerKhariji] {
        &self.tahdheerat
    }

    /// Spends the proof and hands back the warnings it carried.
    ///
    /// What an install ends with. The permit authorises one installation, and a
    /// value that has been turned into its own report is a value no second call
    /// can be given — which is the same "taken by value" discipline the install
    /// entry point already has, carried through to the last thing that reads it.
    #[must_use]
    pub fn ila_tahdheerat(self) -> Vec<TahdheerKhariji> {
        self.tahdheerat
    }

    /// Whether the gate permitted fetching from the registry's mirror.
    ///
    /// False for everything the owner has not explicitly enabled on an entry
    /// whose permission covers it, which is every entry by default.
    #[must_use]
    pub const fn mira_masmuha(&self) -> bool {
        self.mira
    }

    /// Whether this proof authorises installing `ruqaa` into `luba`.
    #[must_use]
    pub fn yushmal(&self, luba: LubaId, ruqaa: RuqaaId) -> bool {
        self.luba == luba && self.ruqaa == ruqaa
    }

    /// The approved pin for one artifact name.
    #[must_use]
    pub fn qitaa_bi_ism(&self, ism: &str) -> Option<&QitaatTanzeel> {
        self.qitaa.iter().find(|qitaa| qitaa.ism == ism)
    }
}
