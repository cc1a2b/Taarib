//! # تثبيت تعريب — install, restore, and survive the update
//!
//! Records every change to a game before making it, restores exactly on
//! request against the recorded hashes, and handles the store updating the
//! game underneath a patch.
//!
//! Two structural guarantees hold the crate together, both enforced by types
//! rather than by call order:
//!
//! - **Safety before writing.** [`masar_tathbeet::thabbit`] takes an
//!   [`taarib_aman::IdhnTathbeet`] by value, and only Phase 16's detection
//!   pipeline can mint one. There is no install path that skips it.
//! - **The manifest is durable before the first byte.** [`bayan::Tathbeet`] is
//!   the only writer, it is obtained only from [`bayan::Tathbeet::ibda`], and
//!   `ibda` flushes the manifest before returning. A path outside the game
//!   root is unrepresentable: package content becomes a [`mawdi::WajhatLuba`]
//!   or it is refused.
//! - **A patch Taarib did not build goes through the same machinery.**
//!   [`khariji::thabbit_khariji`] fetches somebody else's artifacts, proves both
//!   halves of each pin in a staging area outside the game, and then writes
//!   everything — including the files the author's own instructions say to
//!   delete — through [`bayan::Tathbeet`]. So the uninstall that puts the game
//!   back byte for byte is [`taraju`]'s, unchanged, and there is no second
//!   restore path to be worse than it.
//! - **The plan is an input, never a recomputation.** [`tarkib::khutta`] is the
//!   one place the tier, the safety refusal, the loader directory and the loader
//!   slot are decided, and every write on the install path receives its answer:
//!   [`tarkib::nashr_bi_khutta`] executes a [`tarkib::KhuttatTarkib`], and the
//!   script-engine write takes a [`nusus::IdhnNusus`] carrying a
//!   [`tarkib::QararTabaqa`] that only a report the safety layer did not refuse
//!   can mint. A plan cannot be built outside `tarkib`, so a writer cannot
//!   disagree with one, and there is no entry point that writes without one.

pub mod bayan;
pub mod bayan_makhzan;
pub mod itlaq;
pub mod jalb_khariji;
pub mod khariji;
pub mod khata;
pub mod masar_tathbeet;
pub mod mawdi;
pub mod najat_tahdith;
pub mod nusus;
pub mod tahaqquq;
pub mod taraju;
pub mod tarkib;
pub mod tasadum;
pub mod wukala;

pub use bayan::{
    AwqatMalaf, BayanTathbeet, HarisTathbeet, MahallIdad, Muthabbit, NawTaghyeer, NawTathbeet,
    SalahiyatMalaf, SijillIdad, SijillTaghyeer, TarifLuba, Tathbeet, WaqtNizam,
};
pub use itlaq::{
    IdadBeea, IsnadItlaq, KhiyaratLutris, KhiyaratMughallif, KhiyaratSteam, MUTAGHAYYIR_TAJAWUZ,
    RadItlaq, TAJAWUZ_TAARIB, app_talab_steam, badiyat_amr_bidun_tahmeel, badiyat_amr_maa_tahmeel,
    bidun_tajawuz, dam_tajawuz, fihi_ramz_amr, isnadat_talab, khiyarat_bidun_tahmeel,
    khiyarat_maa_amr, khiyarat_maa_isnad, khiyarat_maa_mutaghayyir, khiyarat_maa_tahmeel,
    naffidh_talabat_steam, tajawuz_maa, talabat_steam, yabda_bi_beea,
};
pub use jalb_khariji::{
    AQSA_HAJM_QITAA, NaqilKhariji, QitaaMuhaqqaqa, ijlib_qitaa, nazzif_marhala,
};
pub use khariji::{
    MUJALLAD_KHARIJI, NatijatTathbeetKhariji, TalabTathbeetKhariji, azil_khariji,
    jidhr_nusakh_khariji, kharijiyat_mathbita, thabbit_khariji,
};
pub use khata::{IttijahDaght, KhataTathbeet, MasdarKhatt, NatijatTathbeet};
pub use masar_tathbeet::{
    JidhrKhutut, NatijatTathbeetKamil, NawJidhrKhutut, QararTawafuq, TalabTathbeet, WadaMuhtawa,
    la_tashtaghil, muhtawa_khutut, thabbit,
};
pub use mawdi::{MUJALLAD_TAARIB, NawWajhatNizam, WajhatLuba, WajhatNizam};
pub use najat_tahdith::{
    DaleelTaghayyur, DaleelTatbaq, IhsaHijra, JadwalNusus, MasdarBina, MasirRuqaa,
    SababGhayrMahsum, TalabNajat, TaqdeerBina, TaqreerNajat, fahs_najat,
};
pub use nusus::{
    HafizMuthabbit, IdhnNusus, MutarjimRuqaa, Nashir, makhzan_mukawwinat, raqqi_nusus,
};
pub use tahaqquq::{
    HalatMalaf, NatijatFahsLuba, NatijatTahaqquq, SababInhiraf, TaqreerFahs, TaqreerTahaqquq,
    anwa_mutahabbata, hajm_nusakh_luba, tahaqquq_al_maktaba, tahaqquq_kamil, tahaqquq_luba,
    tahaqquq_nusakh,
};
pub use taraju::{
    BaqiyaMujallad, KhuttatIstiada, MawqiTathbeet, NatijatLuba, RadIdad, RadLaShay, SiyasatIstiada,
    TaqreerIstiada, TaqreerKul, TaqreerMaktaba, ihsa_al_maktaba, istiada_al_maktaba, istiada_kul,
    istiada_nass, istiada_sawt, nazzif_nusakh,
};
pub use tarkib::{
    HajatItar, HalatIdadat, HalatSlot, KhuttatTarkib, LubaMuhallala, MalhuzatManassa, MukawwinItar,
    NatijatTarkib, QararTabaqa, SababLaHaja, SlotMuhammil, TalabItlaq, TaqreerMulhaqat, hajat_itar,
    khutta, nashr_bi_khutta, nashr_mulhaqat, rakkib_itar,
};
pub use tasadum::{
    ALAMAT_KHARIJIYA, ALAMAT_MUSHTARAKA, ALAMAT_TAARIB, AtharTasadum, JihatTasadum, athar_khariji,
    athar_taarib, la_yatasadam_maa_khariji, la_yatasadam_maa_taarib, masah_tasadum,
};
// Renamed on the way out: `masah` is unambiguous inside `wukala` and much less
// so beside `khutta` and `nashr` at the crate root.
pub use wukala::{WUKALA_NIZAM, WakeelQaim, masah as masah_wukala, wakeel_nizam};
