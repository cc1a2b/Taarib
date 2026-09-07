//! Naming `override.cfg` when the game directory already spells it otherwise.
//!
//! This adapter has exactly one place where a name is joined onto a game's own
//! directory: [`MalafTajawuz::fi_mujallad`], which names the project-settings
//! override beside the executable. Everything else is handed a path that was
//! already resolved — the package locator finds a `.pck` by extension across a
//! tree walk rather than by joining a name, and the generated translation goes
//! into the player's own writable directory.
//!
//! The one join still matters, and for a reason that is about *reading* rather
//! than writing. [`MalafTajawuz::hala`] is what stops rung one from overwriting
//! somebody else's file. A game directory carrying an `Override.cfg` written by
//! a launcher or a previous tool holds a file this module must leave alone — but
//! a literal `override.cfg` join answers [`HalatTajawuz::Ghaib`] against it,
//! rung one writes its own beside it, and under Wine, where an exact match wins
//! a case-insensitive lookup, the engine reads Taarib's and silently loses every
//! setting the other file carried.
//!
//! Checked in both directions: a case-differing file is seen, and the exact
//! spelling still wins when both exist. The second needs two entries differing
//! only in case to be creatable, which [`bi_hassasiyat_hala`] establishes at run
//! time rather than assuming.

#![allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and refusing to panic here \
              would mean a test that cannot fail"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory; the product's own recursive deletes go through `HadafHadhf`"
)]

use std::fs;
use std::path::{Path, PathBuf};

use taarib_muhawwil_godot::khadim_nusus::{HalatTajawuz, MALAF_TAJAWUZ, MalafTajawuz};

/// A directory of this test's own, removed and recreated so a rerun starts
/// clean.
fn mujallad_ikhtibar(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-godot-hala-{ism}"));
    let _ = fs::remove_dir_all(&masar);
    assert!(
        fs::create_dir_all(&masar).is_ok(),
        "{} could not be created",
        masar.display()
    );
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

/// The upper-cased spelling a Windows launcher or a repacker produces.
fn ism_mukhtalif() -> String {
    let mut harf = MALAF_TAJAWUZ.chars();
    match harf.next() {
        Some(awwal) => format!("{}{}", awwal.to_uppercase(), harf.as_str()),
        None => MALAF_TAJAWUZ.to_owned(),
    }
}

#[test]
fn tajawuz_ghareeb_yura_rughma_ikhtilaf_alhala() {
    let jidhr = mujallad_ikhtibar("ghareeb");
    // Somebody else's settings, under a spelling Godot itself would open.
    let ghareeb = jidhr.join(ism_mukhtalif());
    fs::write(
        &ghareeb,
        b"[locale]\ntranslations=PoolStringArray( \"res://en.translation\" )\n",
    )
    .expect("the foreign override");

    let tajawuz = MalafTajawuz::fi_mujallad(&jidhr);
    assert_eq!(
        tajawuz.masar(),
        ghareeb,
        "the file already on disk is the one this module has to reason about"
    );
    let Ok(hala) = tajawuz.hala() else {
        panic!("the state could not be read")
    };
    assert_eq!(
        hala,
        HalatTajawuz::Ghareeb,
        "a file somebody else wrote is not touched, whatever it is spelled"
    );
    assert!(!hala.qabil_lil_kitaba(), "rung one must not write over it");
}

#[test]
fn la_yujad_fa_yuhfaz_alism_almatlub() {
    let jidhr = mujallad_ikhtibar("ghaib");

    let tajawuz = MalafTajawuz::fi_mujallad(&jidhr);
    assert_eq!(
        tajawuz.masar(),
        jidhr.join(MALAF_TAJAWUZ),
        "with nothing on disk the spelling asked for is kept, which is the lower-case name \
         Godot documents"
    );
    let Ok(hala) = tajawuz.hala() else {
        panic!("the state could not be read")
    };
    assert_eq!(hala, HalatTajawuz::Ghaib);
    assert!(
        hala.qabil_lil_kitaba(),
        "an empty directory is rung one's ordinary case"
    );
}

#[test]
fn alhala_almutabiqa_taghlib() {
    let jidhr = mujallad_ikhtibar("tafdil");
    if !bi_hassasiyat_hala(&jidhr) {
        return;
    }
    fs::write(jidhr.join(MALAF_TAJAWUZ), b"exact").expect("the exact spelling");
    fs::write(jidhr.join(ism_mukhtalif()), b"folded").expect("the other spelling");

    let tajawuz = MalafTajawuz::fi_mujallad(&jidhr);
    assert_eq!(
        tajawuz.masar(),
        jidhr.join(MALAF_TAJAWUZ),
        "the exactly-spelled file is the one Godot opens and the one to act on"
    );
}
