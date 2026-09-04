//! # ختم تعريب — the sealing layer
//!
//! Every cryptographic operation in the product happens here: Ed25519 signing
//! and strict verification of a patch's signature block, and private-key
//! custody in the OS keychain. No other crate constructs a signature or holds a
//! private key, because a second implementation is a second place the trust
//! model can be wrong.
//!
//! [`MudaqqiqEd25519`] is what closes the verification seam
//! `taarib-tathbeet`'s installer and `taarib-aman`'s pre-flight check leave
//! open: both take a `&dyn taarib_ruqaa::tawqee::MudaqqiqTawqee`, and this is
//! the real one. Verification uses `verify_strict`, which rejects the
//! small-order and non-canonical keys a permissive check would accept from a
//! hostile block.
//!
//! A private key is never written to disk in plaintext, never logged, and never
//! held longer than the operation that needs it: [`mafatih`] stores it in the
//! platform keychain and hands back only the public half.

pub mod khata;
pub mod mafatih;
pub mod malik;
pub mod tawqee;

pub use khata::KhataKhatm;
pub use mafatih::{hat, imsah, khzin, wallid};
pub use malik::{
    HawiyatThiqa, ISM_MIFTAH_MALIK, MIFTAH_TATWIR, MIRSAT_MALIK, MirsatThiqa, hat_malik,
    huwa_malik, khzin_malik,
};
pub use tawqee::{MiftahAam, MiftahKhass, MudaqqiqEd25519};
