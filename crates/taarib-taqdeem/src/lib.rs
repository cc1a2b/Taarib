//! # تقديم تعريب — submission, review, and publication
//!
//! The path from a finished community translation to a published, signed patch.
//!
//! Review authority belongs to one person: the project owner. Everyone else is
//! a contributor. There is no role table, no permission matrix and no admin
//! abstraction, because authority here is possession of the signing key rather
//! than data that could be granted or misconfigured.
//!
//! Three guarantees are structural, each a token no other code can construct:
//!
//! - **The console is absent from a contributor's session.** Every queue,
//!   comment and review-action entry point takes a [`hawiya::SalahiyatMalik`],
//!   minted only by proving possession of the owner's key. A contributor cannot
//!   call them because the argument cannot be produced.
//! - **Submit is inactive until the checks pass.** [`irsal::irsal`] takes a
//!   [`bawwaba::IjtiyazTaqdeem`] by value, minted only when every blocking
//!   check passed and every warning was acknowledged; it owns the
//!   `IjtiyazFuhus` proving Phase 14's hard checks ran on this package.
//! - **Nothing publishes unsigned.** [`nashr::waqqi`] is the only source of
//!   [`nashr::HuzmaMakhtuma`], and it requires both the owner authority and the
//!   owner's private key.
//!
//! The owner's own translations pass through the same submission object and the
//! same publish step; no route through this crate skips either.

pub mod bawwaba;
pub mod hawiya;
pub mod irsal;
pub mod khata;
pub mod muraja;
pub mod musawwada;
pub mod nashr;
pub mod sahb;
pub mod sandooq;
pub mod taaliq;
pub mod tabur;
pub mod talabat;

pub use bawwaba::{BandFahs, IjtiyazTaqdeem, Iqrarat, MudkhalatBawwaba, QaimatFahs, ifhas, ijri};
pub use hawiya::{HawiyatMusahim, Jalsa, SalahiyatMalik};
pub use irsal::{IdadatIrsal, MarhalatIrsal, NatijatIrsal, TalabIrsal, irsal};
pub use khata::{KhataTaqdeem, NatijatTaqdeem};
pub use muraja::{
    IjraMuraja, MarjiMuraja, QararIaatimad, QaydMuraja, SababRafd, SijillMuraja, iaatimad,
    urfud, utlub_taadil,
};
pub use musawwada::{
    HalatTaqdeem, Musawwada, MusawwadaMutaadhira, QaydMusawwada, SijillMusawwadat,
};
pub use nashr::{HuzmaMakhtuma, ItimadManshur, MarhalatNashr, TaqaddumNashr, waqqi};
pub use sahb::{IshaarSahb, QaydSahb, SababSahb, ishab};
pub use sandooq::{BeeatSandooq, HalatItlaq, HasilatSandooq, TaqreerSandooq};
pub use taaliq::{Taaliq, TaaliqatMusawwada};
pub use tabur::{HalatFuhus, MudkhalTabur, MurashshihTabur, TarteebTabur, tabur};
pub use talabat::{IghlaqTalab, LawhatTalabat, TalabTarjama, TalabatLuba};
