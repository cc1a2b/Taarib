//! # أمان تعريب — the safety layer
//!
//! Runs before any installation and produces the only value `taarib-tathbeet`
//! accepts as authorisation to write into a game. The check cannot be skipped
//! by a caller that forgot it, because there is nothing to forget: an install
//! takes an [`IdhnTathbeet`], and [`fahs::fahs`] is the sole function that
//! mints one.
//!
//! Three guarantees are structural, not conventional:
//!
//! - **Anti-cheat is an outright refusal with no override.** No setting, build
//!   flag, developer mode or environment variable disables it, because none is
//!   written. The refusal names the evidence and where it was found — and a
//!   check that could not run is a refusal too, never an absence of evidence,
//!   because VAC is declared in Steam's catalogue and leaves nothing at all in
//!   a game folder for a second opinion to find.
//! - **Signature verification cannot be disabled.** [`tahaqquq_tawqee`] checks
//!   a package against the owner's embedded key and refuses an unsigned,
//!   unknown-key, contributor-only or tampered package by its specific reason.
//! - **Nothing is trusted by declared type.** [`sandooq_fak`] validates every
//!   quarantined path before a byte is written; traversal is unrepresentable,
//!   not filtered.

pub mod fahs;
pub mod idhn;
pub mod iqrar;
pub mod kashf_himaya;
pub mod kashf_shabaka;
pub mod khata;
pub mod qaimat_sahb;
pub mod sandooq_fak;
pub mod tahaqquq_tawqee;

pub use fahs::{NatijatFahs, Rafd, TalabFahs, fahs};
pub use idhn::IdhnTathbeet;
pub use khata::{KhataAman, NatijatAman};
