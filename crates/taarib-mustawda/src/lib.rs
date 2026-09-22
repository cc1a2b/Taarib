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
//! - **The revocation list never travels without its state.** [`sahb`]
//!   refreshes it from the same chain and records every outcome beside the
//!   cache, and the install path takes the list only as a
//!   `taarib_aman::qaimat_sahb::QaimaMuraqaba` — the list beside whether the
//!   registry confirmed it — so an empty answer cannot be read as "nothing is
//!   revoked" when it means "nobody asked".
//!
//! No client fetches the whole catalogue: only the shards covering identifiers
//! the user owns are requested, and an unchanged manifest hits zero network.
//!
//! One more document rides the same chain: [`mujtama`] reads
//! `fahras/tarjamat.json`, the registry's index of Arabic translations other
//! teams published on their own pages. Nothing is installed from it, so no
//! manifest hash vouches for its bytes — but it decides which addresses the
//! product will offer to open, so it carries the owner's signature over its own
//! canonical form and is verified before an entry is read, on the same anchor
//! and with the same refusal grammar as the revocation list. It is capped,
//! validated and cached like a shard besides.

pub mod fahras;
pub mod jalb;
pub mod khata;
pub mod masadir;
pub mod mujtama;
pub mod mutabaqa;
pub mod sabk;
pub mod sahb;
pub mod sumaa;
pub mod tanzeel;
pub mod taqyeem;
pub mod tarteeb;
pub mod tathbeet_bilnaqra;

pub use fahras::{
    ADAD_SHARAIH, BayanMustawda, MuhtawaShareeha, MulakhkhasDhakira, ShareehaMuwaththaqa,
    TajawuzNashr, shareeha,
};
pub use jalb::{
    FahrasMajlub, jalb_bayan, jalb_bayan_maa_masdar, jalb_fahras, jalb_qaimat_sahb,
    jalb_qaimat_sahb_maa_masdar, jalb_sharaih, jalb_shareeha,
};
pub use khata::{KhataMustawda, NatijatMustawda};
pub use masadir::{MasdarMustawda, SilsilatMasadir};
pub use mujtama::{
    AslFahrasMujtama, FahrasMujtama, FahrasMujtamaMajlub, FahrasMukhazzan, KatibFahrasMujtama,
    Tarjama, jalb_fahras_mujtama, tarjamat_li_luba,
};
pub use mutabaqa::{
    IdafatIrtibat, MutabaqatLuba, MutabaqatRuqaa, MutabiqBina, SababGhayrTawafuq, afdal,
    ghayr_mutawafiqa, mutawafiqa,
};
pub use sabk::{
    KhiyaratSabk, MadkhalManshur, Mulghayat, Mustawda, ijri, madkhal_min_huzma, rabt_asl,
};
pub use sahb::{NatijatTajdid, jaddid_qaimat_sahb, jaddid_qaimat_sahb_bi_bayan};
pub use sumaa::{AdadMuraja, HalatSumaa, MulakhkhasSumaa, TaqyeemManshur, ijma};
pub use tanzeel::{MarhalatTanzeel, MukhbirTaqaddum, TalabTanzeel, Taqaddum, nazzif, nazzil};
pub use taqyeem::{
    Balagh, BalaghMuwaqqa, DarajatTaqyeem, SababBalagh, Taqyeem, TaqyeemMuwaqqa, fahs_muaddal,
};
pub use tarteeb::{FiatTaqyeem, KhiyaratTarteeb, MudkhalTarteeb, MuqaranatRuqaa, qarin, rattib};
pub use tathbeet_bilnaqra::{FashalTathbeet, MarhalatTathbeet, TalabNaqra, thabbit_bilnaqra};
