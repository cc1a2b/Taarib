//! Capcom's `DICT`, checked against the bytes Capcom shipped.
//!
//! Nothing in `tests/` is authored data. Every number below was read out of the
//! eight dictionaries under `BIO4/text/` in a retail install of Resident Evil 4
//! (Steam app 254700, the 2014 Ultimate HD Edition), on 2026-09-05, from a
//! library at `F:\SteamLibrary` — and every test that needs the bytes reads
//! those files rather than a fixture, because a dictionary invented to match
//! this reader would prove only that the reader matches itself.
//!
//! The suite is in two halves for one reason: shipping a commercial game's text
//! inside this repository is not this project's to do.
//!
//! * The **pinned** half runs anywhere. Its constants came out of the shipped
//!   files; the constants are the test, so a build machine that has never seen
//!   the game still checks the part that write-back depends on — that
//!   [`basmat_miftah`] reproduces the hashes the game stores.
//! * The **install** half needs the game. It reads all eight files, round-trips
//!   each one, rebuilds six of them, and corrupts one on purpose. It looks for
//!   the directory in `TAARIB_QAMUS_RE4` and then at [`MASAR_MUTAWAQQA`]; with
//!   neither present it says so on stderr and returns, rather than failing a
//!   machine that simply does not own the game.
//!
//! # What the pinned key names are
//!
//! A `.dct` stores a CRC of a key name and never the name. These twenty-three
//! were recovered by hashing the ASCII strings in `bio4.exe` with the seed the
//! dictionary itself carries and keeping the ones that landed on an occupied
//! bucket — 249 of the 335 came back that way, and the ones below are the short
//! ones, chosen so a reader can find them. Each is written beside the English
//! string in the bucket it lands on, which is the check that they are key names
//! and not collisions: `text_f12` landing on `"F12"` two hundred and forty-nine
//! times over is not an accident anybody has to argue about.
//!
//! # The three things a future change must not break
//!
//! * The seed comes from the **file**, not from a constant here. The shipped
//!   lookup loads the header's `+8` word and passes it to the hash; a reader
//!   that hardcoded `0x55d5_7d9f` would silently compute wrong hashes for any
//!   dictionary built with another one.
//! * A pointer is resolved as `&field + value + 1`. Drop the bias and every
//!   bucket resolves to the terminator of the string in front of the one it
//!   wanted, which reads as a table of empty strings rather than as an error.
//! * The terminator is **not** hashed. Adding it changes all 335 hashes.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking and asserts on values it has just read; the \
              lints are written for library code, and refusing to panic here would mean a test \
              that cannot fail"
)]
#![allow(
    clippy::print_stderr,
    reason = "the install-gated half says out loud that it did not run, which is the whole \
              difference between a skipped test and one that passed without checking anything"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "the game's location is a property of the machine running the suite and not of the \
              product, so it comes from the environment rather than from taarib_usus::idadat, \
              which resolves the product's own configuration"
)]

use std::path::{Path, PathBuf};

use taarib_istikhraj::qamus::{
    AQSA_MADAKHIL, ISDAR_MADUM, Qamus, TUL_TARWISA, basmat_miftah, istakhrij,
};
use taarib_istikhraj::rafd::SababRafd;

/// Where the eight dictionaries live in a default Steam install on this machine.
const MASAR_MUTAWAQQA: &str =
    "/mnt/f/SteamLibrary/steamapps/common/Resident Evil 4/BIO4/text";

/// The environment variable that overrides it.
const MUTAGHAYYIR: &str = "TAARIB_QAMUS_RE4";

/// The seed every one of the eight files carries at offset 8.
const BIDHRA: u32 = 0x55d5_7d9f;

/// How many buckets each of the eight declares.
const ADAD_KHANAT: usize = 403;

/// How many of those carry a key.
const ADAD_MASHGHUL: usize = 335;

/// Key names recovered from `bio4.exe`, and the bucket hash each one produces.
///
/// The comment on each line is the English string in the bucket it lands on.
const MIFATIH: [(&str, u32); 23] = [
    ("text_0", 0xb167_ab3a),                    // "0"
    ("text_f1", 0x7b82_44f1),                   // "F1"
    ("text_esc", 0xde73_e3f2),                  // "ESCAPE"
    ("text_end", 0xbf7b_1a4d),                  // "END"
    ("text_back", 0x0b78_17a4),                 // "Back"
    ("text_action", 0x33c9_56e3),               // "Action"
    ("text_enter", 0x0212_5bcb),                // "ENTER"
    ("text_delete", 0x4e17_a6f6),               // "DELETE"
    ("text_disable", 0xba31_e6ac),              // "OFF"
    ("text_enable", 0x87a5_1243),               // "ON"
    ("text_capslock", 0x14e7_f92b),             // "CAPS LOCK"
    ("text_backspace", 0x1e2f_8074),            // "BACKSPACE"
    ("text_cancel_loading", 0x6750_e135),       // "Cancel loading?…"
    ("text_aiming_mode", 0x71f3_dbbc),          // "AIMING MODE"
    ("text_button_config", 0xd26e_b38e),        // "BUTTON CONFIGURATION"
    ("text_exit_tooltip", 0x6974_9a87),         // "Exit current menu."
    ("text_continue_point", 0xd9da_b0c8),       // "RETRY FROM A CHECKPOINT"
    ("text_antyaliasing", 0x7227_e80f),         // "ANTI-ALIASING"
    ("text_brightness_adjust", 0x1744_3020),    // "BRIGHTNESS ADJUST"
    ("text_controller_setup", 0x7baa_10c7),     // "CONTROLLER SETUP"
    ("text_confirm_reset_default", 0xd8a0_2af3), // "Are you sure?"
    ("text_camera_lr", 0x9c1a_c6c2),            // "Camera Left/Right"
    ("text_camera_ud", 0xf3ce_da8b),            // "Camera Up/Down"
];

/// The eight files, and what each one holds.
///
/// `pointers` is how many buckets carry a non-null pointer and `not_utf8` how
/// many occupied buckets hold bytes that are not UTF-8. The two Chinese files
/// differ in both, and that difference is the evidence that they were written by
/// a different tool — see the `qamus` module header.
const MALAFFAT: [(&str, usize, usize, usize, usize); 8] = [
    // name, bytes, pointers, not utf-8, strings this build extracts
    ("CHINESE_S_WIN32.dct", 9_632, 403, 7, 0),
    ("CHINESE_T_WIN32.dct", 9_568, 403, 30, 0),
    ("ENGLISH_WIN32.dct", 10_824, 335, 0, 331),
    ("FRENCH_WIN32.dct", 12_171, 335, 0, 331),
    ("GERMAN_WIN32.dct", 11_890, 335, 0, 331),
    ("ITALIAN_WIN32.dct", 11_464, 335, 0, 331),
    ("JAPANESE_WIN32.dct", 12_503, 335, 0, 330),
    ("SPANISH_WIN32.dct", 11_635, 335, 0, 331),
];

/// `text_enable` in each of the eight, which is one key resolving in eight files
/// whose text has nothing in common.
const TASHGHIL: [(&str, &str); 8] = [
    ("CHINESE_S_WIN32.dct", "启用"),
    ("CHINESE_T_WIN32.dct", "啟用"),
    ("ENGLISH_WIN32.dct", "ON"),
    ("FRENCH_WIN32.dct", "ACTIVER"),
    ("GERMAN_WIN32.dct", "EIN"),
    ("ITALIAN_WIN32.dct", "SÌ"),
    ("JAPANESE_WIN32.dct", "有効化"),
    ("SPANISH_WIN32.dct", "SÍ"),
];

/// The six Capcom's own localization tool wrote, which rebuild byte for byte.
const ASLIYA: [&str; 6] = [
    "ENGLISH_WIN32.dct",
    "FRENCH_WIN32.dct",
    "GERMAN_WIN32.dct",
    "ITALIAN_WIN32.dct",
    "JAPANESE_WIN32.dct",
    "SPANISH_WIN32.dct",
];

/// The two that do not, because another tool produced them.
const MUAADA: [&str; 2] = ["CHINESE_S_WIN32.dct", "CHINESE_T_WIN32.dct"];

// ---------------------------------------------------------------------------
// Pinned: runs on a machine that has never seen the game.
// ---------------------------------------------------------------------------

#[test]
fn basmat_al_miftah_tuidu_ma_yakhzunuhu_al_malaf() {
    for (miftah, mutawaqqa) in MIFATIH {
        let hasil = basmat_miftah(miftah, BIDHRA);
        assert_eq!(
            hasil, mutawaqqa,
            "key {miftah:?}: a shipped .dct stores {mutawaqqa:#010x} and this build computes \
             {hasil:#010x}"
        );
    }
}

#[test]
fn basmat_al_miftah_al_farigh_hiya_al_bidhra() {
    // The hash of nothing is the seed unchanged, which is why the shipped
    // lookup refuses a hash equal to the seed instead of testing the key for
    // emptiness: it never sees the key by then.
    assert_eq!(basmat_miftah("", BIDHRA), BIDHRA);
    assert_eq!(basmat_miftah("", 0), 0);
}

#[test]
fn halat_al_ahruf_juz_min_al_miftah() {
    assert_ne!(basmat_miftah("text_f12", BIDHRA), basmat_miftah("TEXT_F12", BIDHRA));
    assert_ne!(basmat_miftah("text_f12", BIDHRA), basmat_miftah("Text_F12", BIDHRA));
}

#[test]
fn al_khatima_laysat_juzan_min_al_miftah() {
    // Feeding the terminator moves every hash, so a build that started doing it
    // would produce a dictionary the game finds nothing in.
    assert_ne!(basmat_miftah("text_f12\0", BIDHRA), basmat_miftah("text_f12", BIDHRA));
}

#[test]
fn al_bidhra_min_al_malaf_la_min_thabit() {
    // Two seeds, two hashes for one key. A reader that hardcoded the seed would
    // agree with the shipped files and disagree with anything else built the
    // same way.
    assert_ne!(basmat_miftah("text_f12", BIDHRA), basmat_miftah("text_f12", 0));
}

#[test]
fn al_bina_yadau_al_jadwal_fi_makanihi() {
    let madakhil: Vec<(u32, Option<&[u8]>)> =
        MIFATIH.iter().map(|(_, basma)| (*basma, Some(b"x".as_slice()))).collect();
    let qamus = Qamus::ibni(BIDHRA, &madakhil).expect("a 23-bucket dictionary fits every ceiling");
    let bayt = qamus.uktub();

    // `0x0c + 7 + 1 == 0x14`. The header's own pointer is the smallest instance
    // of the format's one-byte bias, and the shipped files all store 7 here.
    assert_eq!(&bayt[0..4], b"DICT");
    assert_eq!(u32::from_le_bytes(bayt[4..8].try_into().unwrap()), ISDAR_MADUM);
    assert_eq!(u32::from_le_bytes(bayt[8..12].try_into().unwrap()), BIDHRA);
    assert_eq!(u32::from_le_bytes(bayt[12..16].try_into().unwrap()), 7);
    assert_eq!(u32::from_le_bytes(bayt[16..20].try_into().unwrap()), 23);
    assert_eq!(TUL_TARWISA, 20);
}

#[test]
fn dawrat_bina_wa_kitaba_wa_qira() {
    let nusus: Vec<&[u8]> = vec![
        b"".as_slice(),
        b"F12",
        b"Cancel loading?\n\n^983047^Yes\n^983047^No^983048^",
        "有効化".as_bytes(),
        b"ON",
    ];
    let madakhil: Vec<(u32, Option<&[u8]>)> = MIFATIH
        .iter()
        .take(nusus.len())
        .zip(nusus.iter())
        .map(|((_, basma), nass)| (*basma, Some(*nass)))
        .collect();

    let qamus = Qamus::ibni(BIDHRA, &madakhil).expect("five buckets fit every ceiling");
    let maktub = qamus.uktub();
    let mustarja = Qamus::iqra(&maktub).expect("what ibni built, iqra reads");
    assert_eq!(mustarja, qamus);
    assert_eq!(mustarja.uktub(), maktub);
    for (fahras, nass) in nusus.iter().enumerate() {
        assert_eq!(mustarja.bayt(mustarja.madakhil()[fahras]), Some(*nass));
    }
    // The blob opens with the sentinel the shipped writer opens it with.
    assert_eq!(mustarja.kutla().first(), Some(&0));
}

#[test]
fn al_muashir_al_farigh_yabqa_farighan() {
    let madakhil: Vec<(u32, Option<&[u8]>)> =
        vec![(0x1111_1111, Some(b"a".as_slice())), (0, None), (0x2222_2222, Some(b"b".as_slice()))];
    let qamus = Qamus::ibni(BIDHRA, &madakhil).expect("three buckets fit every ceiling");
    assert_eq!(qamus.madakhil()[1].izaha(), 0);
    assert_eq!(qamus.madakhil()[1].mawqi(), None);
    assert_eq!(qamus.bayt(qamus.madakhil()[1]), None);
    assert!(!qamus.madakhil()[1].mashghul());
    let mustarja = Qamus::iqra(&qamus.uktub()).expect("a null pointer survives a round trip");
    assert_eq!(mustarja, qamus);
}

#[test]
fn al_nass_bi_khatima_dakhiliya_marfud() {
    let madakhil: Vec<(u32, Option<&[u8]>)> = vec![(0x1111_1111, Some(b"a\0b".as_slice()))];
    let khata = Qamus::ibni(BIDHRA, &madakhil).expect_err("an interior NUL cannot be pointed at");
    assert!(matches!(khata, SababRafd::TajawuzHadd { .. }), "{khata:?}");
}

#[test]
fn adad_khanat_mufrit_marfud_qabl_al_hajz() {
    let mut bayt = Qamus::ibni(BIDHRA, &[(0x1111_1111, Some(b"a".as_slice()))])
        .expect("one bucket fits")
        .uktub();
    bayt[16..20].copy_from_slice(&AQSA_MADAKHIL.saturating_add(1).to_le_bytes());
    let khata = Qamus::iqra(&bayt).expect_err("a count above the ceiling is refused");
    assert!(matches!(khata, SababRafd::TajawuzHadd { .. }), "{khata:?}");
}

// ---------------------------------------------------------------------------
// Install: needs the game.
// ---------------------------------------------------------------------------

/// The directory holding the eight dictionaries, or [`None`].
fn mujallad() -> Option<PathBuf> {
    if let Some(min_al_bia) = std::env::var_os(MUTAGHAYYIR) {
        let masar = PathBuf::from(min_al_bia);
        assert!(
            masar.is_dir(),
            "{MUTAGHAYYIR} names {}, which is not a directory",
            masar.display()
        );
        return Some(masar);
    }
    let masar = PathBuf::from(MASAR_MUTAWAQQA);
    masar.is_dir().then_some(masar)
}

/// Says why an install-gated test checked nothing.
fn ghaib(ism: &str) {
    eprintln!(
        "{ism}: skipped — Resident Evil 4 is not at {MASAR_MUTAWAQQA} and {MUTAGHAYYIR} is unset"
    );
}

/// The bytes of one dictionary.
fn bayt_malaf(mujallad: &Path, ism: &str) -> Vec<u8> {
    let masar = mujallad.join(ism);
    std::fs::read(&masar).unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()))
}

#[test]
fn kull_khana_tuqra_bil_bunya() {
    let Some(mujallad) = mujallad() else {
        return ghaib("kull_khana_tuqra_bil_bunya");
    };
    for (ism, hajm, muashirat, ghayr_salih, _) in MALAFFAT {
        let bayt = bayt_malaf(&mujallad, ism);
        assert_eq!(bayt.len(), hajm, "{ism}");
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));

        assert_eq!(qamus.adad(), ADAD_KHANAT, "{ism}");
        assert_eq!(qamus.bidhra(), BIDHRA, "{ism}");
        assert_eq!(qamus.isdar(), ISDAR_MADUM, "{ism}");

        let mashghul = qamus.madakhil().iter().filter(|m| m.mashghul()).count();
        assert_eq!(mashghul, ADAD_MASHGHUL, "{ism}");

        let bi_muashir = qamus.madakhil().iter().filter(|m| m.mawqi().is_some()).count();
        assert_eq!(bi_muashir, muashirat, "{ism}");

        // Every pointer resolves, and every resolved string is terminated. The
        // reader would have refused the file otherwise, so this is the statement
        // that it refused nothing quietly.
        for madkhal in qamus.madakhil() {
            assert_eq!(
                qamus.bayt(*madkhal).is_some(),
                madkhal.mawqi().is_some(),
                "{ism}: bucket {:#010x}",
                madkhal.basma()
            );
        }

        let ghayr = qamus
            .madakhil()
            .iter()
            .filter(|m| m.mashghul())
            .filter(|m| qamus.bayt(**m).is_some() && qamus.nass(**m).is_none())
            .count();
        assert_eq!(ghayr, ghayr_salih, "{ism}");
    }
}

#[test]
fn dawrat_kitaba_mutabiqa_bil_bayt() {
    let Some(mujallad) = mujallad() else {
        return ghaib("dawrat_kitaba_mutabiqa_bil_bayt");
    };
    for (ism, ..) in MALAFFAT {
        let bayt = bayt_malaf(&mujallad, ism);
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        let maktub = qamus.uktub();
        assert_eq!(maktub.len(), bayt.len(), "{ism}: length");
        assert!(maktub == bayt, "{ism}: an untouched round trip changed bytes");
    }
}

#[test]
fn ibni_yuidu_bina_malaffat_capcom_al_sitta() {
    let Some(mujallad) = mujallad() else {
        return ghaib("ibni_yuidu_bina_malaffat_capcom_al_sitta");
    };
    for ism in ASLIYA {
        let bayt = bayt_malaf(&mujallad, ism);
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        let madakhil: Vec<(u32, Option<&[u8]>)> =
            qamus.madakhil().iter().map(|m| (m.basma(), qamus.bayt(*m))).collect();
        let mabni = Qamus::ibni(qamus.bidhra(), &madakhil)
            .unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        assert!(
            mabni.uktub() == bayt,
            "{ism}: rebuilding from keys and strings did not reproduce the shipped file"
        );
    }
}

#[test]
fn ibni_la_yuidu_bina_al_malaffayn_al_sinniyayn() {
    let Some(mujallad) = mujallad() else {
        return ghaib("ibni_la_yuidu_bina_al_malaffayn_al_sinniyayn");
    };
    for ism in MUAADA {
        let bayt = bayt_malaf(&mujallad, ism);
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        let madakhil: Vec<(u32, Option<&[u8]>)> =
            qamus.madakhil().iter().map(|m| (m.basma(), qamus.bayt(*m))).collect();
        let mabni = Qamus::ibni(qamus.bidhra(), &madakhil)
            .unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        // Pinned as a difference rather than left to be discovered: these two
        // open the blob without the sentinel byte and close it with 26 and 16
        // bytes of padding, so a rebuild is a dictionary the game reads and not
        // the file that was read. Round-tripping them is `uktub`'s job, and the
        // line below is the assertion that it does it to the byte.
        assert!(
            mabni.uktub() != bayt,
            "{ism}: this file now rebuilds exactly, so the module header's account of how it \
             differs from Capcom's six is out of date"
        );
        assert!(qamus.uktub() == bayt, "{ism}: an untouched round trip changed bytes");
    }
}

#[test]
fn tawzee_al_khanat_yutabiq_jadwal_tajzia() {
    let Some(mujallad) = mujallad() else {
        return ghaib("tawzee_al_khanat_yutabiq_jadwal_tajzia");
    };
    for (ism, ..) in MALAFFAT {
        let bayt = bayt_malaf(&mujallad, ism);
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        assert!(
            qamus.muttasiq(),
            "{ism}: an occupied bucket is not at hash % {ADAD_KHANAT} nor at the next free slot \
             after it, which would mean the count field is not the modulus"
        );
    }
}

#[test]
fn al_bahth_yujib_kama_tujib_al_luba() {
    let Some(mujallad) = mujallad() else {
        return ghaib("al_bahth_yujib_kama_tujib_al_luba");
    };
    for (ism, mutawaqqa) in TASHGHIL {
        let bayt = bayt_malaf(&mujallad, ism);
        let qamus = Qamus::iqra(&bayt).unwrap_or_else(|sabab| panic!("{ism}: {sabab:?}"));
        assert_eq!(qamus.abhath("text_enable"), Some(mutawaqqa.as_bytes()), "{ism}");
        assert_eq!(qamus.abhath("text_f12"), Some(b"F12".as_slice()), "{ism}");
        // Both of the shipped lookup's refusals.
        assert_eq!(qamus.abhath(""), None, "{ism}: the empty key hashes to the seed");
        assert_eq!(qamus.abhath("text_f12 "), None, "{ism}: a key that is not in the file");
    }
}

#[test]
fn istikhraj_al_mujallad_yaqra_sitta_wa_yarfud_ithnayn() {
    let Some(mujallad) = mujallad() else {
        return ghaib("istikhraj_al_mujallad_yaqra_sitta_wa_yarfud_ithnayn");
    };
    let (jadwal, taqreer) = istakhrij(&mujallad);

    assert_eq!(taqreer.maqrua.len(), 6, "six of the eight are readable");
    assert_eq!(taqreer.marfuda.len(), 2, "the two Chinese ones are refused");
    for madkhal in &taqreer.marfuda {
        assert!(
            matches!(madkhal.sabab, SababRafd::HadUlBina { .. }),
            "{}: {:?}",
            madkhal.hawiya,
            madkhal.sabab
        );
        assert!(
            madkhal.hawiya.starts_with("CHINESE_"),
            "{} was refused and should not have been",
            madkhal.hawiya
        );
    }

    let mut mutawaqqa = 0_usize;
    for (ism, _, _, ghayr_salih, adad) in MALAFFAT {
        if ghayr_salih > 0 {
            continue;
        }
        let qura = taqreer
            .maqrua
            .iter()
            .find(|q| q.hawiya == ism)
            .unwrap_or_else(|| panic!("{ism} is missing from the read report"));
        assert_eq!(qura.adad, adad, "{ism}");
        mutawaqqa = mutawaqqa.saturating_add(adad);
    }
    assert_eq!(taqreer.adad_nusus(), mutawaqqa);

    // One identity per key per language file: the container is part of the
    // identity, so eight translations of one key are eight rows and not one.
    assert_eq!(jadwal.adad(), mutawaqqa);

    let bi_miftah_muharrik = jadwal.madakhil().filter(|m| m.mawqi.min_almuharrik()).count();
    assert_eq!(
        bi_miftah_muharrik,
        jadwal.adad(),
        "every row's identity comes from the game's own key"
    );
}

#[test]
fn malaf_talif_marfud_bil_ism() {
    let Some(mujallad) = mujallad() else {
        return ghaib("malaf_talif_marfud_bil_ism");
    };
    let asl = bayt_malaf(&mujallad, "ENGLISH_WIN32.dct");
    assert!(Qamus::iqra(&asl).is_ok(), "the untouched file reads");

    // The signature.
    let mut bayt = asl.clone();
    bayt[3] = b'X';
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::SighaMajhula { .. })),
        "a broken signature: {:?}",
        Qamus::iqra(&bayt)
    );

    // The version the shipped loader compares for exact equality.
    let mut bayt = asl.clone();
    bayt[4..8].copy_from_slice(&0x1001_u32.to_le_bytes());
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::IsdarGhayrMadum { .. })),
        "a version the game itself refuses: {:?}",
        Qamus::iqra(&bayt)
    );

    // A count that makes the bucket array run past the end of the file. Half a
    // table is the damage that would otherwise read as a shorter dictionary.
    let mut bayt = asl.clone();
    bayt[16..20].copy_from_slice(&5_000_u32.to_le_bytes());
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::Talif { .. })),
        "a truncated bucket array: {:?}",
        Qamus::iqra(&bayt)
    );

    // A negative count, which the loader's signed compare would read as empty.
    let mut bayt = asl.clone();
    bayt[16..20].copy_from_slice(&(-1_i32).to_le_bytes());
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::Talif { .. })),
        "a negative bucket count: {:?}",
        Qamus::iqra(&bayt)
    );

    // A null pointer to the bucket array with buckets declared.
    let mut bayt = asl.clone();
    bayt[12..16].copy_from_slice(&0_u32.to_le_bytes());
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::Talif { .. })),
        "a null bucket array with 403 buckets declared: {:?}",
        Qamus::iqra(&bayt)
    );

    // One bucket aimed past the end of the file.
    let mut bayt = asl.clone();
    bayt[24..28].copy_from_slice(&0x00ff_ffff_u32.to_le_bytes());
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::Talif { .. })),
        "a bucket pointing outside the blob: {:?}",
        Qamus::iqra(&bayt)
    );

    // The last terminator removed, which is the damage the renderer would walk
    // off the end of the mapping on.
    let mut bayt = asl.clone();
    let akhir = bayt.len().saturating_sub(1);
    bayt[akhir] = b'X';
    assert!(
        matches!(Qamus::iqra(&bayt), Err(SababRafd::Talif { .. })),
        "an unterminated last string: {:?}",
        Qamus::iqra(&bayt)
    );

    // A file cut inside its own bucket array: the header still declares 403.
    let maqtu = TUL_TARWISA.saturating_add(800);
    assert!(
        matches!(Qamus::iqra(&asl[..maqtu]), Err(SababRafd::Talif { .. })),
        "a truncated file: {:?}",
        Qamus::iqra(&asl[..maqtu])
    );
}
