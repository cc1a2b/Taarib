//! The `.fnt` container, asserted against bytes out of a shipped game.
//!
//! # Where the bytes came from
//!
//! `SYSTEM_ZH_CN` below is `BIO4/Font/system_zh-cn.fnt`, all 448 bytes of it,
//! copied out of *Resident Evil 4* (2005, Ultimate HD Edition) from a Steam
//! library at `F:\SteamLibrary` on 2026-09-05. It is pinned here rather than
//! read from disk so that the round trip is a test on a build machine that has
//! never seen the game — and every test that *can* use the real install still
//! does: [`kull_al_khutut_tadur_bila_taghyeer`] walks all thirty-two files in
//! `BIO4/Font` when it can find them, and reports how many it checked.
//!
//! It is the smallest of the thirty-two and it is the one that matters most: it
//! is the only file whose cell pitch is not 28, and the only one that
//! distinguishes the grid law this crate implements from the simpler law that
//! ignores the trailing off-atlas entries. A crate that got the law wrong would
//! pass every other file and fail this one.
//!
//! # What is asserted
//!
//! * an untouched `.fnt` re-serialises **byte for byte**
//! * the TPL magic is little-endian inside a `.fnt`, and reading it big-endian
//!   is refused
//! * the image is `GX_TF_C4` through a sixteen-entry `GX_TL_RGB5A3` palette,
//!   with both data offsets at the block's own end and no texels behind them
//! * the four bytes there name `18000009.pack`, which exists in the install
//! * the grid law reproduces the height the TPL declares

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "a test that needs the installed game has to be told where it is, and there is no \
              configuration layer inside a test binary to resolve that through"
)]

use std::fs;
use std::path::PathBuf;

use taarib_muhawwil_bio4::hizma::HuwiyatHizma;
use taarib_muhawwil_bio4::khatt::KhattBio4;
use taarib_muhawwil_bio4::tibl::{SighatLawn, SighatSura, TarteebBayt, Tibl};

/// `BIO4/Font/system_zh-cn.fnt`, verbatim.
const SYSTEM_ZH_CN: [u8; 448] = [
    0x20, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x78, 0x56, 0x34, 0x12,
    0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
    0x14, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
    0x44, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x04, 0x08, 0x00, 0x00, 0x00,
    0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x01, 0x11,
    0x01, 0x12, 0x04, 0x10, 0x07, 0x0e, 0x06, 0x11, 0x05, 0x11, 0x05, 0x11,
    0x06, 0x11, 0x06, 0x11, 0x05, 0x11, 0x05, 0x11, 0x05, 0x11, 0x02, 0x14,
    0x01, 0x11, 0x01, 0x13, 0x01, 0x14, 0x00, 0x14, 0x02, 0x14, 0x03, 0x12,
    0x02, 0x11, 0x02, 0x13, 0x02, 0x0a, 0x01, 0x14, 0x01, 0x14, 0x01, 0x13,
    0x00, 0x14, 0x01, 0x11, 0x00, 0x12, 0x02, 0x12, 0x01, 0x11, 0x03, 0x13,
    0x00, 0x14, 0x02, 0x13, 0x02, 0x14, 0x01, 0x14, 0x03, 0x12, 0x05, 0x11,
    0x02, 0x12, 0x02, 0x09, 0x02, 0x14, 0x00, 0x14, 0x01, 0x14, 0x02, 0x13,
    0x00, 0x14, 0x02, 0x14, 0x01, 0x14, 0x01, 0x14, 0x03, 0x12, 0x05, 0x0f,
    0x02, 0x14, 0x03, 0x11, 0x02, 0x13, 0x01, 0x14, 0x04, 0x10, 0x02, 0x12,
    0x03, 0x12, 0x03, 0x13, 0x01, 0x14, 0x00, 0x14, 0x02, 0x14, 0x01, 0x13,
    0x01, 0x12, 0x00, 0x14, 0x01, 0x13, 0x01, 0x11, 0x01, 0x14, 0x01, 0x14,
    0x02, 0x13, 0x02, 0x14, 0x00, 0x12, 0x05, 0x10, 0x09, 0x0e, 0x05, 0x11,
    0x06, 0x11, 0x05, 0x10, 0x06, 0x10, 0x07, 0x10, 0x05, 0x11, 0x01, 0x13,
    0x04, 0x12, 0x06, 0x11, 0x03, 0x11, 0x02, 0x13, 0x02, 0x13, 0x03, 0x0f,
    0x01, 0x14, 0x02, 0x14, 0x01, 0x14, 0x02, 0x14, 0x02, 0x13, 0x00, 0x12,
    0x02, 0x13, 0x01, 0x13, 0x00, 0x14, 0x01, 0x13, 0x01, 0x14, 0x02, 0x13,
    0x01, 0x10, 0x02, 0x11, 0x03, 0x14, 0x01, 0x11, 0x02, 0x12, 0x02, 0x14,
    0x02, 0x13, 0x02, 0x13, 0x01, 0x14, 0x00, 0x14, 0x01, 0x14, 0x02, 0x14,
    0x01, 0x14, 0x00, 0x14, 0x01, 0x14, 0x02, 0x14, 0x02, 0x14, 0x00, 0x14,
    0x01, 0x13, 0x01, 0x13, 0x01, 0x11, 0x02, 0x13, 0x00, 0x14, 0x01, 0x14,
    0x01, 0x13, 0x03, 0x13, 0x02, 0x14, 0x01, 0x14, 0x03, 0x12, 0x01, 0x14,
    0x02, 0x14, 0x00, 0x14, 0x01, 0x14, 0x00, 0x12, 0x06, 0x12, 0x03, 0x11,
    0x02, 0x11, 0x01, 0x11, 0x03, 0x13, 0x06, 0x11, 0x0c, 0x13, 0x00, 0x14,
    0x01, 0x14, 0x01, 0x14, 0x01, 0x08, 0x01, 0x12, 0x03, 0x12, 0x03, 0x12,
    0x01, 0x14, 0x00, 0x14, 0x01, 0x13, 0x01, 0x13, 0x01, 0x13, 0x04, 0x12,
    0x00, 0x20, 0x00, 0x20, 0x00, 0x20, 0x00, 0x20, 0x00, 0x20, 0x00, 0x20,
    0x00, 0x20, 0x00, 0x20,
];

/// Where an installed copy of the game might be, in the order they are tried.
///
/// `TAARIB_RE4` first, so a machine with the game somewhere else — or a
/// directory holding bytes copied out of one — can say where without this file
/// being edited.
///
/// Returning [`None`] makes the tests that need the game pass without checking
/// anything, which is correct on a build machine and is a trap on a machine that
/// has the game and cannot see it. `TAARIB_RE4_ILZAM` closes the trap: set it and
/// a missing install is a failure instead of a silent skip.
const MURASHAHAT: [&str; 3] = [
    "/mnt/f/SteamLibrary/steamapps/common/Resident Evil 4",
    "/mnt/d/Program Files (x86)/Steam/steamapps/common/Resident Evil 4",
    "F:/SteamLibrary/steamapps/common/Resident Evil 4",
];

fn mujallad_luba() -> Option<PathBuf> {
    if let Ok(muallan) = std::env::var("TAARIB_RE4") {
        let masar = PathBuf::from(muallan);
        if masar.is_dir() {
            return Some(masar);
        }
    }
    let wujid = MURASHAHAT.into_iter().map(PathBuf::from).find(|masar| masar.is_dir());
    assert!(
        !(wujid.is_none() && std::env::var_os("TAARIB_RE4_ILZAM").is_some()),
        "TAARIB_RE4_ILZAM is set and no Resident Evil 4 install was found; \
         point TAARIB_RE4 at one, or at a directory holding its BIO4 subtree"
    );
    wujid
}

#[test]
fn khatt_ghayr_muaddal_yadur_bayt_bi_bayt() {
    let khatt = KhattBio4::min_bayt(&SYSTEM_ZH_CN).expect("the pinned .fnt should parse");
    let khruj = khatt.ila_bayt().expect("a parsed .fnt should serialise");
    assert_eq!(
        khruj.as_slice(),
        SYSTEM_ZH_CN.as_slice(),
        "an untouched .fnt must round-trip byte for byte"
    );
}

#[test]
fn tarwis_yutabiq_ma_quri_min_alluba() {
    let khatt = KhattBio4::min_bayt(&SYSTEM_ZH_CN).unwrap();
    assert_eq!(khatt.mawdi_tibl, 0x20);
    assert_eq!(khatt.mawdi_qiyasat, 0x80);
    assert!(khatt.muqaddima.iter().all(|bayt| *bayt == 0), "the preamble is zero");

    khatt.tibl.tahaqquq_khatt().expect("a shipped font is C4 through an RGB5A3 palette");
    let sura = khatt.tibl.sura_wahida().expect("exactly one image");
    assert_eq!(SighatSura::min_raqm(sura.sigha), Some(SighatSura::C4));
    assert_eq!(sura.ard, 1024, "every shipped font is 1024 texels wide");
    assert_eq!(sura.irtifa, 60);
    assert_eq!(sura.mawdi, 0x44, "the data offset is the block's own end");
    assert_eq!(sura.murashah_asghar, 1);
    assert_eq!(sura.murashah_akbar, 1);

    let lawha = khatt.tibl.lawha_wahida().expect("exactly one palette");
    assert_eq!(lawha.adad, 16);
    assert_eq!(SighatLawn::min_raqm(lawha.sigha), Some(SighatLawn::Rgb5a3));
    assert_eq!(lawha.mawdi, 0x44, "the palette offset is the block's own end too");
}

#[test]
fn assihr_saghir_altarteeb_dakhil_alkhatt() {
    let kutla = SYSTEM_ZH_CN.get(0x20..0x80).expect("the TPL block");
    Tibl::min_bayt(kutla, TarteebBayt::Saghir).expect("a .fnt stores the magic little-endian");
    let khata = Tibl::min_bayt(kutla, TarteebBayt::Kabir);
    assert!(
        khata.is_err(),
        "reading a .fnt's magic big-endian must be refused, not silently accepted"
    );
}

#[test]
fn huwiyat_alhizma_tusammi_milaffan_mawjudan() {
    let khatt = KhattBio4::min_bayt(&SYSTEM_ZH_CN).unwrap();
    let hawiya = HuwiyatHizma(khatt.huwiyat_hizma().expect("a trailer with an identifier"));
    assert_eq!(hawiya.0, [0x09, 0x00, 0x00, 0x18]);
    assert_eq!(hawiya.ism(), "18000009.pack");

    let Some(luba) = mujallad_luba() else {
        return;
    };
    let masar = luba.join("BIO4/ImagePack").join(hawiya.ism());
    assert!(masar.is_file(), "{} should exist in the install", masar.display());
}

#[test]
fn ashabaka_tuntij_alirtifa_almuallan() {
    let khatt = KhattBio4::min_bayt(&SYSTEM_ZH_CN).unwrap();
    let shabaka = khatt.shabaka().expect("the grid law should reproduce the declared height");
    assert_eq!(shabaka.hajm_khana(), 20, "system_zh-cn is the one font whose pitch is not 28");
    assert_eq!(shabaka.ard(), 1024);
    assert_eq!(shabaka.aamida(), 51);
    assert_eq!(shabaka.sufuf(), 3);
    assert_eq!(shabaka.irtifa(), 60);
    assert_eq!(khatt.madakhil.len(), 160, "the table has 160 entries");
    assert_eq!(
        khatt.adad_khanat(),
        152,
        "eight of them have a right edge outside a 20-texel cell and are not cells"
    );
}

#[test]
fn milaff_maqsus_marfud() {
    for tul in [0usize, 4, 8, 0x1F, 0x40, 0x7F] {
        let qita = SYSTEM_ZH_CN.get(..tul).expect("a prefix of the pinned file");
        assert!(
            KhattBio4::min_bayt(qita).is_err(),
            "a .fnt truncated to {tul} bytes must be refused"
        );
    }
}

#[test]
fn jadwal_bitul_fardi_marfud() {
    let mut bayt = SYSTEM_ZH_CN.to_vec();
    bayt.push(0);
    assert!(
        KhattBio4::min_bayt(&bayt).is_err(),
        "an odd number of metric bytes cannot be a table of pairs"
    );
}

#[test]
fn kull_al_khutut_tadur_bila_taghyeer() {
    let Some(luba) = mujallad_luba() else {
        // Nothing to do on a machine without the game. The pinned file above is
        // what keeps this suite meaningful there.
        return;
    };
    let dalil = luba.join("BIO4/Font");
    let Ok(madkhalat) = fs::read_dir(&dalil) else {
        panic!("{} could not be listed", dalil.display());
    };

    let mut adad = 0usize;
    for madkhal in madkhalat.flatten() {
        let masar = madkhal.path();
        if masar.extension().and_then(|imtidad| imtidad.to_str()) != Some("fnt") {
            continue;
        }
        let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
        let khatt = KhattBio4::min_bayt(&bayt)
            .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
        let khruj = khatt
            .ila_bayt()
            .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
        assert_eq!(khruj, bayt, "{} did not round-trip byte for byte", masar.display());

        khatt
            .tibl
            .tahaqquq_khatt()
            .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
        let shabaka = khatt
            .shabaka()
            .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
        assert_eq!(shabaka.ard(), 1024, "{}", masar.display());

        let hawiya = HuwiyatHizma(
            khatt
                .huwiyat_hizma()
                .unwrap_or_else(|| panic!("{} has no pack identifier", masar.display())),
        );
        let hizma = luba.join("BIO4/ImagePack").join(hawiya.ism());
        assert!(hizma.is_file(), "{} names {} which is absent", masar.display(), hawiya.ism());
        adad = adad.saturating_add(1);
    }
    assert_eq!(adad, 32, "BIO4/Font holds thirty-two uncompressed .fnt files");
}
