//! # مستودع تعريب — the registry client
//!
//! A player who never translates anything opens Taarib, sees that four of their
//! games have Arabic patches, and installs one in a click. Everything here
//! serves that sentence.
//!
//! The backend is a public Git repository: sharded JSON for metadata, release
//! assets for binaries, served over a forge's raw-content endpoint with a
//! mirror, a bundled offline copy and a LAN share behind it. Nothing to run,
//! nothing to pay for, and anyone can fork the catalogue with `git clone`.
//!
//! Two guarantees are structural rather than conventional:
//!
//! - **Index content is verified before it is read.** Shard bytes become a
//!   [`fahras::ShareehaMuwaththaqa`] only by hashing against the manifest's
//!   declared entry first; there is no other constructor, so unverified index
//!   content cannot be represented, let alone parsed.
//! - **Every install goes through the safety layer.**
//!   [`tathbeet_bilnaqra::thabbit_bilnaqra`] quarantines, calls
//!   `taarib_aman::fahs`, and moves the permit it mints into
//!   `taarib_tathbeet::thabbit`. A network download, an imported file and a LAN
//!   share take that one path; no source is privileged.
//!
//! No client fetches the whole catalogue: only the shards covering identifiers
//! the user owns are requested, and an unchanged manifest hits zero network.

pub mod fahras;
pub mod jalb;
pub mod khata;
pub mod masadir;
pub mod mutabaqa;
pub mod sumaa;
pub mod tanzeel;
pub mod taqyeem;
pub mod tarteeb;
pub mod tathbeet_bilnaqra;

pub use fahras::{
    ADAD_SHARAIH, BayanMustawda, MuhtawaShareeha, ShareehaMuwaththaqa, TajawuzNashr, shareeha,
};
pub use jalb::{
    FahrasMajlub, jalb_bayan, jalb_fahras, jalb_qaimat_sahb, jalb_shareeha, jalb_sharaih,
};
pub use khata::{KhataMustawda, NatijatMustawda};
pub use masadir::{MasdarMustawda, SilsilatMasadir};
pub use mutabaqa::{
    IdafatIrtibat, MutabaqatLuba, MutabaqatRuqaa, MutabiqBina, SababGhayrTawafuq, afdal,
    ghayr_mutawafiqa, mutawafiqa,
};
pub use sumaa::{AdadMuraja, HalatSumaa, MulakhkhasSumaa, TaqyeemManshur, ijma};
pub use tanzeel::{
    MarhalatTanzeel, MukhbirTaqaddum, TalabTanzeel, Taqaddum, nazzif, nazzil,
};
pub use taqyeem::{
    Balagh, BalaghMuwaqqa, DarajatTaqyeem, SababBalagh, Taqyeem, TaqyeemMuwaqqa, fahs_muaddal,
};
pub use tarteeb::{
    FiatTaqyeem, KhiyaratTarteeb, MudkhalTarteeb, MuqaranatRuqaa, qarin, rattib,
};
pub use tathbeet_bilnaqra::{
    FashalTathbeet, MarhalatTathbeet, TalabNaqra, thabbit_bilnaqra,
};
