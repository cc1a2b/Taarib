//! # تحديث تعريب — the application's own updates
//!
//! Checking a signed channel, fetching what it offers, and replacing this
//! installation without ever leaving it broken.
//!
//! The trust rule is the whole design: the channel manifest is verified by the
//! **same anchor that verifies patches**, `taarib_khatm::MIRSAT_MALIK`. A
//! release client therefore cannot be updated by anything the owner did not
//! sign, and a channel signed by the published development key is refused by
//! name rather than treated as merely unknown — the update path is the one
//! place where accepting a wrong signature would replace the program that does
//! all the other checking.
//!
//! Built in **Phase 22** on `taarib-khatm` and the transport `taarib-mustawda`
//! established.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `bayan` | the channel manifest: its shape, its detached-signature verification, and choosing the entry for this target, channel and version |
//! | `jalb` | fetching one entry: the free-space refusal, range resume, the declared size as a hard ceiling, and streamed hashing |
//! | `tabdil` | the swap, per format, and the startup check that finishes or undoes an interrupted one |
//! | `khata` | every refusal, in band 8100 |
//!
//! ## What an update never touches
//!
//! The data root, the component store, and every game with Taarib content
//! installed. `tabdil` is handed the executable's path and a verified download
//! and nothing else, so the guarantee is structural rather than remembered.

pub mod bayan;
pub mod jalb;
pub mod khata;
pub mod tabdil;

pub use bayan::{BayanTahdith, MadkhalTahdith, ihlil, intiqa};
pub use jalb::{MalafMuhaqqaq, ijlib};
pub use khata::{KhataTahdith, NatijatTahdith};
pub use tabdil::{KhuttatTabdil, TareeqatTabdil, istadill, khattit, naffidh, tahaqqaq_bad_iqla};
