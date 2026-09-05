//! # ورشة تعريب — translating together, without a server
//!
//! Two or more translators working one project, with no infrastructure: a
//! project is a file, shared over whatever transport people already use, and
//! two divergent copies merge by string identity.
//!
//! This phase adds sharing, assignment, merge and conflict resolution on top of
//! Phase 12's project store, Phase 13's review record and Phase 18's comments.
//! It introduces no second store and no second review model.
//!
//! Two guarantees are structural:
//!
//! - **A merge cannot manufacture an approval.** Records move whole, so a
//!   review state only ever travels with the exact text it was granted
//!   against; a third text re-enters review through the public transitions,
//!   and nothing here constructs a [`taarib_mustalahat::muraja::ShahadatMuraja`].
//! - **A conflict ends only by explicit choice.** [`damj::Damj`] holds its
//!   conflicts open, hands out no merged table while any remain, and
//!   [`damj::Damj::itmam`] demands one [`damj::Qarar`] per conflict — there is
//!   no default, no policy and no timestamp that resolves one.
//!
//! [`mushtaraka`] extends the same premise to the one thing this product
//! accumulates by *playing* rather than by working: the overlay's screen
//! readings. A share is a signed, per-game file of observations and nothing
//! else — its wire format has no field for a contributor or a review state,
//! so an import cannot manufacture human provenance however the file is
//! edited — and nothing leaves a machine without an [`mushtaraka::IdhnMusharaka`],
//! which is minted only against the fingerprint of the exact entry set the
//! user was shown.

pub mod damj;
pub mod damj_mawarid;
pub mod ihsaat;
pub mod khata;
pub mod mushtaraka;
pub mod tabadul;
pub mod tarikh;
pub mod tasdir;
pub mod tawzi;

pub use damj::{BitaqatJanib, Damj, Janib, NawNizaa, Nizaa, Qarar, QaydHasm, TaqreerDamj, damj};
pub use damj_mawarid::{DamjMasrad, idmij_dhakira, idmij_masrad};
pub use mushtaraka::{
    HuzmaMuwaththaqa, IdhnMusharaka, IqraratMusharaka, KhiyaratMusharaka,
    MusawwadatMusharaka, QaydMushtarak, TahdheerMusharaka, TaqreerIstirad,
    TarwisatMushtaraka,
};
pub use ihsaat::{IhsaatMashru, IhsaatMusahim, ihsib};
pub use khata::{KhataWarsha, NatijatWarsha};
pub use tabadul::saddir_wa_athbit;
pub use tarikh::{QaydTarikh, TarikhMashru};
pub use tasdir::{AslHuzma, MuhtawaHuzma, istawrid, saddir};
pub use tawzi::{AqfalMashru, IkhtilafTawzi, Qufl, Tawzi};
