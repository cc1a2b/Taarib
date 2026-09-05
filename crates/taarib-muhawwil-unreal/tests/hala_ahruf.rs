//! Finding — and writing into — a game's container directory when the depot
//! spelled it differently.
//!
//! Unreal's convention is `<Root>/<Project>/Content/Paks`. A Windows game
//! running under Wine or Proton sees a **case-insensitive** filesystem, so a
//! depot that ships `content/paks` runs perfectly for the player and is
//! invisible to a literal `Path::join` on Linux.
//!
//! Both halves of this adapter are affected, and the write half is the worse
//! one. [`afhas`] missing the directory reports a game with no containers, which
//! a caller reads as "nothing to translate". `uktub_fi_luba` missing it does
//! something quieter and more damaging: it *creates* `Content/Paks` beside the
//! game's own `content/paks`, writes the patch container into it, and returns
//! the path — an install that reports success and that the engine never mounts.
//!
//! Each case is checked twice: that a case-differing directory resolves, and
//! that the exact spelling still wins when both exist. The second needs two
//! directories differing only in case to be creatable, which
//! [`bi_hassasiyat_hala`] establishes at run time rather than assuming; where
//! the filesystem cannot hold them the assertion is skipped rather than faked.

#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; refusing to panic here would mean a test that cannot fail"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::fs;
use std::path::{Path, PathBuf};

use taarib_muhawwil_unreal::isdar::{Tabaa, afhas};
use taarib_muhawwil_unreal::mawarid::pak::KatibPak;

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-unreal-hala-{ism}"));
    let _ = fs::remove_dir_all(&masar);
    assert!(fs::create_dir_all(&masar).is_ok(), "{} could not be created", masar.display());
    masar
}

/// Whether this filesystem can hold two entries differing only in case.
fn bi_hassasiyat_hala(dalil: &Path) -> bool {
    let saghir = dalil.join("taarib-hala-probe");
    if fs::write(&saghir, b"x").is_err() {
        return false;
    }
    let kabir = dalil.join("TAARIB-HALA-PROBE");
    let munfasil = !kabir.exists();
    let _ = fs::remove_file(&saghir);
    munfasil
}

/// Places a placeholder container at a project-relative path under a root.
///
/// A placeholder rather than a real pak, deliberately: every step under test
/// here asks only where a file is, and [`afhas`] reports a container whose
/// footer it could not read as exactly that. Building a real container to prove
/// a directory was located would be proving two things at once.
fn dua_hawiya(jidhr: &Path, nisbi: &str) {
    let masar = jidhr.join(nisbi);
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).expect("a fixture directory");
    }
    fs::write(&masar, b"taarib fixture").expect("a fixture container");
}

#[test]
fn dalil_alhawiyat_yuhaddad_rughma_ikhtilaf_alhala() {
    let jidhr = mujallad_ikhtibar("qiraa");
    dua_hawiya(&jidhr, "Riverside/content/paks/Riverside-Windows.pak");

    let bina = afhas(&jidhr).expect("a game root that exists is never a refusal");
    assert_ne!(
        bina.tabaa,
        Tabaa::Majhul,
        "a lower-case content/paks is still the game's container directory: {:?}",
        bina.athar
    );
    assert!(
        bina.hawiyat.iter().any(|masar| masar.ends_with("Riverside-Windows.pak")),
        "the container has to be collected or the adapter reports a game with nothing in it"
    );
}

#[test]
fn dalil_alhawiyat_yaqbal_muhtawa_bila_paks() {
    let jidhr = mujallad_ikhtibar("sayib");
    // A game with loose content and no Paks directory — the second shape
    // `jid_dalil_pak` answers for, and it has to survive the fold too.
    fs::create_dir_all(jidhr.join("Riverside/CONTENT")).expect("the content directory");

    let bina = afhas(&jidhr).expect("the probe");
    assert_ne!(bina.tabaa, Tabaa::Majhul, "loose CONTENT/ is still the game's content: {:?}",
        bina.athar);
}

#[test]
fn dalil_alhawiyat_alhala_almutabiqa_taghlib() {
    let jidhr = mujallad_ikhtibar("tafdil");
    if !bi_hassasiyat_hala(&jidhr) {
        return;
    }
    dua_hawiya(&jidhr, "Riverside/Content/Paks/exact.pak");
    dua_hawiya(&jidhr, "Riverside/content/paks/folded.pak");

    let bina = afhas(&jidhr).expect("the probe");
    assert!(
        bina.hawiyat.iter().any(|masar| masar.ends_with("exact.pak")),
        "Unreal's own spelling is the directory the engine mounts: {:?}",
        bina.hawiyat
    );
    assert!(
        !bina.hawiyat.iter().any(|masar| masar.ends_with("folded.pak")),
        "the folded directory must not be read when the exact one is right there"
    );
}

#[test]
fn hawiyat_alruqaa_taqa_dakhil_dalil_alluba_nafsih() {
    let jidhr = mujallad_ikhtibar("kitaba");
    let mashru = jidhr.join("Riverside");
    fs::create_dir_all(mashru.join("content/paks")).expect("the containers directory");

    let mut katib = KatibPak::jadeed();
    katib
        .daa("Riverside/Content/Localization/Game/ar/Game.locres", b"taarib".to_vec())
        .expect("one entry");

    let masar = katib.uktub_fi_luba(&mashru).expect("the container write");
    assert!(masar.is_file(), "{} was reported written and is not there", masar.display());
    assert!(
        masar.starts_with(mashru.join("content/paks")),
        "{} landed in a directory the engine never mounts, which is an install that reports \
         success and changes nothing",
        masar.display()
    );
    assert!(
        !mashru.join("Content").exists(),
        "a second, capitalised Content/ was created beside the game's own"
    );
}

#[test]
fn hawiyat_alruqaa_tunshi_almasar_haythu_la_yujad() {
    let jidhr = mujallad_ikhtibar("inshaa");
    let mashru = jidhr.join("Riverside");
    // `Content/` exists in the depot's own spelling; `Paks/` does not exist at
    // all. The fold resolves as far as the filesystem goes and then keeps the
    // spelling asked for, so the new directory lands inside the game's tree.
    fs::create_dir_all(mashru.join("CONTENT")).expect("the content directory");

    let mut katib = KatibPak::jadeed();
    katib.daa("Riverside/Content/x.uasset", b"taarib".to_vec()).expect("one entry");

    let masar = katib.uktub_fi_luba(&mashru).expect("the container write");
    assert!(
        masar.starts_with(mashru.join("CONTENT")),
        "{} was created beside the game's content rather than under it",
        masar.display()
    );
    assert!(masar.ends_with("zzz_taarib_P.pak"), "the patch container keeps its own name");
}
