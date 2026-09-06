//! The anti-cheat gate, asserted against the games actually installed on disk.
//!
//! # Why this test exists as well as the unit tests
//!
//! Every signature in `kashf_himaya` also has a fixture test built from names
//! typed into a temporary directory, and a fixture only ever proves that the
//! matcher matches what its author believed the game ships. This file closes
//! that loop against real installs: it walks the real trees, with the real
//! entry counts and the real capitalisation, and asserts both halves of the
//! answer — that the protected games are named and refused, and that the
//! unprotected ones stay clean and are not blocked by a signature that reaches
//! too far.
//!
//! The second half is the one that is easy to forget and expensive to get
//! wrong. Anti-cheat carries an outright refusal with no override anywhere, so
//! a marker that fires on a game with no anti-cheat permanently blocks a
//! legitimate install and the user has nothing to appeal to.
//!
//! # Where the expectations came from
//!
//! `F:\SteamLibrary\steamapps\common` on 2026-09-06, read only. Three of the
//! seventeen installed games are protected:
//!
//! * `FC 26` ships EA Javelin: `EAAntiCheat.GameServiceLauncher.exe`
//!   (`ProductName` "EA Javelin Anticheat", `InternalName` "skyfall") beside a
//!   52 MB packed `.dll`, `EAAntiCheat.Installer.exe`, `EAAntiCheat.cfg`,
//!   `EAAntiCheat.splash.png`, `EAJavelinInstaller_installscript.vdf`, and an
//!   `__Installer/EAAntiCheat/` directory.
//! * `ELDEN RING` ships Easy Anti-Cheat in both forms.
//! * `Grand Theft Auto V Enhanced` ships `BattlEye/`.
//!
//! The other fourteen ship none, and this file says so by name.
//!
//! A machine without the library skips every case and passes; set
//! `TAARIB_MAKTABA_ILZAM` to turn that skip into a failure.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "a test that needs the installed library has to be told where it is, and there is no \
              configuration layer inside a test binary to resolve that through"
)]

use std::path::{Path, PathBuf};

use taarib_aman::kashf_himaya::{
    DaleelHimaya, HalatMatjar, NawHimaya, ifhas_himaya_bi_matjar, mahmiya,
};

/// Where a Steam library's `common` directory might be, in the order tried.
const MAKTABAT: [&str; 3] = [
    "/mnt/f/SteamLibrary/steamapps/common",
    "F:/SteamLibrary/steamapps/common",
    "/mnt/d/Program Files (x86)/Steam/steamapps/common",
];

/// Where the Steam *client* is, which is the only place `appinfo.vdf` lives.
const JUDHUR_STEAM: [&str; 2] =
    ["/mnt/d/Program Files (x86)/Steam", "D:/Program Files (x86)/Steam"];

/// One installed game, its Steam identifier, and what the gate must say.
struct HalatLuba {
    /// The `installdir` under `steamapps/common`.
    mujallad: &'static str,
    /// Its Steam application id, for the catalogue half.
    appid: u32,
    /// Every anti-cheat the file scan alone must name, in [`NawHimaya`] order.
    mutawaqqa: &'static [NawHimaya],
}

/// The seventeen games installed on the machine this was written against.
const HALAT: &[HalatLuba] = &[
    // --- protected: the gate must refuse all three ---------------------------
    HalatLuba { mujallad: "FC 26", appid: 3_405_690, mutawaqqa: &[NawHimaya::EaJavelin] },
    HalatLuba {
        mujallad: "ELDEN RING",
        appid: 1_245_620,
        mutawaqqa: &[NawHimaya::EasyAntiCheat, NawHimaya::EasyAntiCheatEos],
    },
    HalatLuba {
        mujallad: "Grand Theft Auto V Enhanced",
        appid: 3_240_220,
        mutawaqqa: &[NawHimaya::BattlEye],
    },
    // --- unprotected: none of these may become a false positive --------------
    HalatLuba { mujallad: "AFOP", appid: 2_840_770, mutawaqqa: &[] },
    HalatLuba { mujallad: "Among Us", appid: 945_360, mutawaqqa: &[] },
    HalatLuba { mujallad: "ChainedTogether", appid: 2_567_870, mutawaqqa: &[] },
    HalatLuba { mujallad: "Crash Bandicoot - N Sane Trilogy", appid: 731_490, mutawaqqa: &[] },
    HalatLuba { mujallad: "Crimson Desert", appid: 3_321_460, mutawaqqa: &[] },
    HalatLuba { mujallad: "DARK SOULS REMASTERED", appid: 570_940, mutawaqqa: &[] },
    HalatLuba { mujallad: "Gang Beasts", appid: 285_900, mutawaqqa: &[] },
    HalatLuba { mujallad: "Hollow Knight", appid: 367_520, mutawaqqa: &[] },
    HalatLuba { mujallad: "Little Nightmares", appid: 424_840, mutawaqqa: &[] },
    HalatLuba { mujallad: "Little Nightmares Enhanced Edition", appid: 2_149_010, mutawaqqa: &[] },
    HalatLuba { mujallad: "MECCHA CHAMELEON", appid: 4_704_690, mutawaqqa: &[] },
    HalatLuba { mujallad: "REPO", appid: 3_241_660, mutawaqqa: &[] },
    HalatLuba { mujallad: "Resident Evil 4", appid: 254_700, mutawaqqa: &[] },
    HalatLuba { mujallad: "Tangles", appid: 2_784_980, mutawaqqa: &[] },
];

/// The library root, from the environment first and then the candidates.
///
/// Returning [`None`] makes every case below pass without checking anything,
/// which is right on a build machine and a trap on this one.
/// `TAARIB_MAKTABA_ILZAM` closes the trap.
fn maktaba() -> Option<PathBuf> {
    if let Ok(muallan) = std::env::var("TAARIB_MAKTABA") {
        let masar = PathBuf::from(muallan);
        if masar.is_dir() {
            return Some(masar);
        }
    }
    let wujid = MAKTABAT.into_iter().map(PathBuf::from).find(|masar| masar.is_dir());
    assert!(
        !(wujid.is_none() && std::env::var_os("TAARIB_MAKTABA_ILZAM").is_some()),
        "TAARIB_MAKTABA_ILZAM is set and no Steam library was found; point TAARIB_MAKTABA at a \
         directory holding the game folders"
    );
    wujid
}

/// The Steam client root, when one of the candidates is really there.
fn jidhr_steam() -> Option<PathBuf> {
    JUDHUR_STEAM
        .into_iter()
        .map(PathBuf::from)
        .find(|masar| masar.join("appcache").join("appinfo.vdf").is_file())
}

/// Every case whose directory exists, paired with that directory.
fn hadira(maktaba: &Path) -> Vec<(&'static HalatLuba, PathBuf)> {
    HALAT
        .iter()
        .map(|hala| (hala, maktaba.join(hala.mujallad)))
        .filter(|(_, masar)| masar.is_dir())
        .collect()
}

#[test]
fn fahs_al_malaffat_wahdahu_yusammi_al_mahmiya_wa_yubqi_al_baqiya_nadhifa() {
    let Some(maktaba) = maktaba() else {
        return;
    };
    let mut adad: usize = 0;
    for (hala, masar) in hadira(&maktaba) {
        // No app id and no Steam root: the file scan on its own, so that a
        // failure here is about a signature and never about the catalogue.
        let (ijmaa, matjar) = ifhas_himaya_bi_matjar(&masar, None, None);
        assert_eq!(matjar, HalatMatjar::GhayrMatlub);
        assert_eq!(
            ijmaa.anwa(),
            hala.mutawaqqa,
            "{}: {:#?}",
            hala.mujallad,
            ijmaa.adilla.iter().map(DaleelHimaya::injilizi).collect::<Vec<_>>()
        );
        assert_eq!(mahmiya(&ijmaa), !hala.mutawaqqa.is_empty(), "{}", hala.mujallad);
        assert!(!ijmaa.mabtur, "{} was too large to walk", hala.mujallad);
        adad = adad.saturating_add(1);
    }
    assert!(adad >= 3, "only {adad} of the expected game folders were present");
}

#[test]
fn javelin_fi_fc26_yusamma_wa_yushar_ila_mawdiihi() {
    let Some(maktaba) = maktaba() else {
        return;
    };
    let masar = maktaba.join("FC 26");
    if !masar.is_dir() {
        return;
    }

    let (ijmaa, _) = ifhas_himaya_bi_matjar(&masar, None, None);
    assert!(mahmiya(&ijmaa), "FC 26 ships a kernel-mode anti-cheat and must be refused");
    assert_eq!(ijmaa.anwa(), [NawHimaya::EaJavelin]);

    // The user is shown a path, so every piece of evidence must carry one that
    // is really under the game and really exists.
    for daleel in &ijmaa.adilla {
        let mawdi = daleel.masar.as_deref().unwrap_or_else(|| {
            panic!("a file-scan detection with no path: {}", daleel.injilizi())
        });
        assert!(mawdi.starts_with(&masar), "{mawdi:?} is outside {masar:?}");
        assert!(mawdi.exists(), "{mawdi:?} was named and is not there");
    }

    let ayunn: Vec<&str> = ijmaa.adilla.iter().map(|daleel| daleel.ayn.as_str()).collect();
    for matlub in [
        "EAAntiCheat.GameServiceLauncher.exe",
        "EAAntiCheat.GameServiceLauncher.dll",
        "EAAntiCheat.Installer.exe",
        "EAAntiCheat.cfg",
        "EAJavelinInstaller_installscript.vdf",
    ] {
        assert!(ayunn.iter().any(|ayn| ayn.contains(matlub)), "{matlub} missing from {ayunn:?}");
    }
}

#[test]
fn maa_fahras_al_matjar_tabqa_al_ahkam_kama_hiya() {
    let (Some(maktaba), Some(steam)) = (maktaba(), jidhr_steam()) else {
        return;
    };
    for (hala, masar) in hadira(&maktaba) {
        let (ijmaa, matjar) = ifhas_himaya_bi_matjar(&masar, Some(hala.appid), Some(&steam));
        assert_eq!(matjar, HalatMatjar::Maqru, "{}", hala.mujallad);

        // The catalogue may add to a verdict and may never subtract from one:
        // whatever the files said is still said.
        for naw in hala.mutawaqqa {
            assert!(ijmaa.anwa().contains(naw), "{} lost {naw:?}", hala.mujallad);
        }
        // And it may only add the unnamed kind, which is the only evidence the
        // catalogue can produce that the files cannot.
        for naw in ijmaa.anwa() {
            assert!(
                hala.mutawaqqa.contains(&naw)
                    || matches!(naw, NawHimaya::GhayrMusamma | NawHimaya::Vac),
                "{} gained {naw:?} from the catalogue",
                hala.mujallad
            );
        }
        // A game with no anti-cheat in its files and none in the catalogue is
        // the case the whole product depends on staying installable.
        if hala.mutawaqqa.is_empty() && !ijmaa.anwa().contains(&NawHimaya::GhayrMusamma) {
            assert!(!mahmiya(&ijmaa), "{} became a false positive", hala.mujallad);
        }
    }
}

#[test]
fn ramz_tawafuq_steam_yaltaqit_gta_v() {
    let (Some(maktaba), Some(steam)) = (maktaba(), jidhr_steam()) else {
        return;
    };
    let masar = maktaba.join("Grand Theft Auto V Enhanced");
    if !masar.is_dir() {
        return;
    }

    // Valve's own compatibility verdict for this app is
    // `#SteamDeckVerified_TestResult_UnsupportedAntiCheatConfiguration`, in all
    // three of the sibling test maps. It is a second, independent reason to
    // refuse a game the files already refuse — which is exactly the case where
    // it is safe to prove the reading works.
    let (ijmaa, matjar) = ifhas_himaya_bi_matjar(&masar, Some(3_240_220), Some(&steam));
    assert_eq!(matjar, HalatMatjar::Maqru);
    assert!(ijmaa.anwa().contains(&NawHimaya::BattlEye));
    assert!(
        ijmaa.anwa().contains(&NawHimaya::GhayrMusamma),
        "{:#?}",
        ijmaa.adilla.iter().map(DaleelHimaya::injilizi).collect::<Vec<_>>()
    );
}
