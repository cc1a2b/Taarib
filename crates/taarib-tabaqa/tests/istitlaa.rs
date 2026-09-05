//! The two refusals the overlay owes a user before it is installed, asserted.
//!
//! Both of these are about the Direct3D 9 era specifically, and both were
//! written because the alternative failure is invisible.
//!
//! * [`yaqra_mustawradat_al_luba`] — a game that imports `d3d9.dll` is reported
//!   as a Direct3D 9 game from its own import table, without running it. A walk
//!   that got the PE32 versus PE32+ data-directory offset wrong would report
//!   *no* graphics API on half the executables in existence and look like a
//!   game with a run-time renderer, so both widths are built and both are read.
//! * [`yarfud_slot_maakhudh`] — Taarib does not take a proxy slot another
//!   product has taken. This is the rule Resident Evil 4 makes concrete on the
//!   machine this was written on: its `Bin32` directory already carries an
//!   eleven-megabyte `dinput8.dll` belonging to `re4_tweaks`, with a live log
//!   beside it. That one is not Taarib's slot and is left alone; a third party
//!   in `version.dll` **is**, and the verdict for it is
//!   [`HukmQudra::Mustaheela`].
//!
//! Nothing here reads a real game. The executables are synthesized byte by byte
//! in this file, which is the only way to assert that a *malformed* one is
//! refused rather than silently reported as importing nothing.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "every index here is into a buffer this file just built at a size it chose"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "scratch teardown under `std::env::temp_dir()`, never a data root or a game \
              directory: every path deleted here was created by `mujallad` a few lines earlier \
              under a name carrying this process's own id"
)]

use std::fs;
use std::path::{Path, PathBuf};

use taarib_tabaqa::istitlaa::{SLOT_TAARIB, istatli};
use taarib_tabaqa::qudra::HukmQudra;
use taarib_tabaqa::wajiha::WajihatRusum;

/// Where the section this file builds is mapped, in the loaded image.
const OINWAN_QISM: u32 = 0x1000;

/// Where the same bytes start in the file.
const IZAHAT_QISM: u32 = 0x0400;

/// Where the import descriptors sit inside the section.
const IZAHAT_USTUWANA: u32 = 0x0100;

// ---------------------------------------------------------------------------
// A Portable Executable, built here so a malformed one can be built too
// ---------------------------------------------------------------------------

/// Builds a PE that imports the named modules and nothing else.
///
/// `mumtadd` chooses PE32+ over PE32, which moves the data directories sixteen
/// bytes further into the optional header. That single number is the one this
/// walk can get wrong in a way that still parses, so both are produced and both
/// are asserted against.
fn banni_pe(mumtadd: bool, asmaa: &[&str]) -> Vec<u8> {
    let hajm_ikhtiyari: u16 = if mumtadd { 240 } else { 224 };
    let bidayat_pe: usize = 0x80;
    let bidayat_ikhtiyari = bidayat_pe + 24;
    let izahat_adilla = if mumtadd { 112 } else { 96 };

    let mut bayt = vec![0_u8; 0x1000];
    bayt[0] = b'M';
    bayt[1] = b'Z';
    bayt[0x3C..0x40].copy_from_slice(&u32::try_from(bidayat_pe).unwrap_or(0).to_le_bytes());

    bayt[bidayat_pe..bidayat_pe + 4].copy_from_slice(b"PE\0\0");
    // Machine, then the section count, then eight bytes this walk ignores, then
    // the optional header's size and the characteristics.
    bayt[bidayat_pe + 4..bidayat_pe + 6].copy_from_slice(&0x8664_u16.to_le_bytes());
    bayt[bidayat_pe + 6..bidayat_pe + 8].copy_from_slice(&1_u16.to_le_bytes());
    bayt[bidayat_pe + 20..bidayat_pe + 22].copy_from_slice(&hajm_ikhtiyari.to_le_bytes());

    let sihr: u16 = if mumtadd { 0x020B } else { 0x010B };
    bayt[bidayat_ikhtiyari..bidayat_ikhtiyari + 2].copy_from_slice(&sihr.to_le_bytes());

    // Data directory one is the import table.
    let bidayat_ustuwana = bidayat_ikhtiyari + izahat_adilla + 8;
    bayt[bidayat_ustuwana..bidayat_ustuwana + 4]
        .copy_from_slice(&(OINWAN_QISM + IZAHAT_USTUWANA).to_le_bytes());

    // One section, forty bytes, immediately after the optional header.
    let bidayat_aqsam = bidayat_ikhtiyari + usize::from(hajm_ikhtiyari);
    bayt[bidayat_aqsam..bidayat_aqsam + 8].copy_from_slice(b".rdata\0\0");
    bayt[bidayat_aqsam + 8..bidayat_aqsam + 12].copy_from_slice(&0x0800_u32.to_le_bytes());
    bayt[bidayat_aqsam + 12..bidayat_aqsam + 16].copy_from_slice(&OINWAN_QISM.to_le_bytes());
    bayt[bidayat_aqsam + 16..bidayat_aqsam + 20].copy_from_slice(&0x0800_u32.to_le_bytes());
    bayt[bidayat_aqsam + 20..bidayat_aqsam + 24].copy_from_slice(&IZAHAT_QISM.to_le_bytes());

    // The names go after the descriptors, which are twenty bytes each plus one
    // all-zero terminator.
    let mut izahat_ism = IZAHAT_USTUWANA
        + u32::try_from(asmaa.len().saturating_add(1)).unwrap_or(0).saturating_mul(20);
    for (fahras, ism) in asmaa.iter().enumerate() {
        let wasf = usize::try_from(IZAHAT_QISM + IZAHAT_USTUWANA).unwrap_or(0)
            + fahras.saturating_mul(20);
        // The original-thunk and first-thunk fields only have to be non-zero,
        // because the terminator is an all-zero descriptor and this walk reads
        // the name and those two fields alone.
        bayt[wasf..wasf + 4].copy_from_slice(&(OINWAN_QISM + 0x0700).to_le_bytes());
        bayt[wasf + 12..wasf + 16]
            .copy_from_slice(&(OINWAN_QISM + izahat_ism).to_le_bytes());
        bayt[wasf + 16..wasf + 20].copy_from_slice(&(OINWAN_QISM + 0x0700).to_le_bytes());

        let mawdi = usize::try_from(IZAHAT_QISM + izahat_ism).unwrap_or(0);
        bayt[mawdi..mawdi + ism.len()].copy_from_slice(ism.as_bytes());
        izahat_ism += u32::try_from(ism.len().saturating_add(1)).unwrap_or(0);
    }
    bayt
}

/// A scratch directory of this test's own.
#[expect(
    clippy::expect_used,
    reason = "a test that cannot create its own scratch directory has nothing to assert; \
              failing loudly here is the report"
)]
fn mujallad(ism: &str) -> PathBuf {
    let masar = std::env::temp_dir().join(format!("taarib-istitlaa-{ism}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&masar);
    fs::create_dir_all(&masar).expect("the scratch directory could not be created");
    masar
}

/// Writes a file and returns where it went.
#[expect(
    clippy::expect_used,
    reason = "as `mujallad`: a fixture that cannot be written is a test that cannot run"
)]
fn uktub(mujallad: &Path, ism: &str, bayt: &[u8]) -> PathBuf {
    let masar = mujallad.join(ism);
    fs::write(&masar, bayt).expect("the fixture could not be written");
    masar
}

// ---------------------------------------------------------------------------
// The import table
// ---------------------------------------------------------------------------

/// A game that links `d3d9.dll` is reported as a Direct3D 9 game.
///
/// Asserted for both PE widths, because the data-directory offset differs
/// between them by exactly sixteen bytes and a walk that used one number for
/// both would find the import directory on half of all executables and nothing
/// on the other half — reported as "no graphics API", which is indistinguishable
/// from a game whose renderer loads at run time.
#[test]
fn yaqra_mustawradat_al_luba() {
    let jidhr = mujallad("mustawradat");
    for (ism, mumtadd) in [("luba32.exe", false), ("luba64.exe", true)] {
        let masar = uktub(
            &jidhr,
            ism,
            &banni_pe(mumtadd, &["KERNEL32.dll", "d3d9.dll", "d3dx9_43.dll"]),
        );
        let taqrir = match istatli(&masar) {
            Ok(taqrir) => taqrir,
            Err(khata) => panic!("{ism} would not be surveyed: {khata}"),
        };

        assert!(
            taqrir.tisaa(),
            "{ism} imports d3d9.dll and was reported as {:?}",
            taqrir.wajihat
        );
        assert_eq!(
            taqrir.wajihat,
            vec![WajihatRusum::Direct3D9],
            "{ism} imports one graphics module and must be reported as one API"
        );
        assert!(
            taqrir.mustawradat.iter().any(|name| name.eq_ignore_ascii_case("d3d9.dll")),
            "{ism} must name the module it was recognised by"
        );
        assert!(taqrir.yumkin(), "{ism} has a free proxy slot and must be installable");
    }
    let _ = fs::remove_dir_all(&jidhr);
}

/// A game that links none of the graphics modules is reported as unknown, not
/// as a refusal.
///
/// The distinction is the whole point of the verdict being a value: a game whose
/// renderer is loaded at run time imports nothing either, so this narrows the
/// report without stopping an installation.
#[test]
fn luba_bila_rusum_tunqas_wala_turfad() {
    let jidhr = mujallad("bila-rusum");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["KERNEL32.dll", "USER32.dll"]));
    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    assert!(taqrir.wajihat.is_empty(), "no graphics module was imported");
    assert_eq!(taqrir.hukm(), HukmQudra::Naqisa, "an unknown renderer narrows, it does not stop");
    assert!(taqrir.yumkin(), "an unknown renderer must not stop an installation");
    let _ = fs::remove_dir_all(&jidhr);
}

/// A file that is not a Portable Executable is refused rather than reported
/// empty.
///
/// A report that said "no graphics API" about a file it could not parse would be
/// a report that looked like an answer.
#[test]
fn malaf_ghayr_pe_yurfad() {
    let jidhr = mujallad("ghayr-pe");
    let masar = uktub(&jidhr, "luba.exe", b"this is not an executable at all");
    assert!(istatli(&masar).is_err(), "a non-PE must be refused, not surveyed as empty");
    let _ = fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// The proxy slot
// ---------------------------------------------------------------------------

/// Taarib refuses to take a proxy slot another product has taken.
///
/// The verdict is [`HukmQudra::Mustaheela`] and the reason names the file, in
/// both languages, so a user can recognise what they already installed. There is
/// no override: overwriting the file would delete the other product, and there
/// is no second copy of the name to write beside it.
#[test]
fn yarfud_slot_maakhudh() {
    let jidhr = mujallad("slot-maakhudh");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    // Not Taarib's loader: it carries none of Taarib's own marker bytes.
    let _ = uktub(&jidhr, SLOT_TAARIB, b"another product's forwarding proxy");

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };

    assert!(!taqrir.slot_mutah(), "{SLOT_TAARIB} is taken and must not be reported as free");
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Mustaheela,
        "a taken {SLOT_TAARIB} must stop the installation outright"
    );
    assert!(!taqrir.yumkin(), "the overlay must not be offered where its own slot is taken");

    let mustaheela = taqrir
        .asbab
        .iter()
        .find(|sabab| sabab.hukm == HukmQudra::Mustaheela)
        .map(|sabab| (sabab.arabi.clone(), sabab.injilizi.clone()));
    let Some((arabi, injilizi)) = mustaheela else {
        panic!("the refusal carried no reason");
    };
    assert!(
        injilizi.contains(SLOT_TAARIB) && injilizi.contains("another product"),
        "the English reason must name the slot and say who has it: {injilizi}"
    );
    assert!(!arabi.trim().is_empty(), "the refusal must be readable in Arabic too");
    let _ = fs::remove_dir_all(&jidhr);
}

/// A third party in a slot Taarib does not use is reported and does not stop
/// anything.
///
/// This is Resident Evil 4's shape exactly: an eleven-megabyte `dinput8.dll`
/// that belongs to `re4_tweaks`. Taarib takes `version.dll` and never that one, so
/// the correct behaviour is to say a third-party hook will be live in the
/// process — which is why unhooking verifies itself — and install anyway.
#[test]
fn wakeel_ghareeb_yudhkar_wala_yamnaa() {
    let jidhr = mujallad("wakeel-ghareeb");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    let _ = uktub(&jidhr, "dinput8.dll", &vec![0x90_u8; 4096]);

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };

    assert!(taqrir.slot_mutah(), "Taarib's own slot is free and must be reported so");
    assert!(taqrir.yumkin(), "a proxy Taarib does not use must not stop an installation");
    assert_eq!(taqrir.ghurabaa().len(), 1, "the third-party proxy must be reported");
    let Some(ghareeb) = taqrir.ghurabaa().first().copied() else {
        panic!("the third-party proxy was not reported");
    };
    assert_eq!(ghareeb.ism, "dinput8.dll");
    assert_eq!(ghareeb.hajm, 4096, "the size is what lets a user recognise the product");
    assert!(!ghareeb.taarib, "another product's proxy must not be mistaken for Taarib's");
    assert!(
        taqrir.asbab.iter().any(|sabab| sabab.injilizi.contains("never restores a function \
             pointer it did not install")),
        "the report must state the unhook rule the third-party hook makes load-bearing"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// Taarib's own loader in Taarib's own slot is a reinstall, not a conflict.
#[test]
fn slot_taarib_nafsuh_laysa_taarudan() {
    let jidhr = mujallad("slot-taarib");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    // The marker `taarib-mudkhal` carries: the name it writes its own log under.
    let mut mudkhal = vec![0x00_u8; 512];
    mudkhal.extend_from_slice(b"mudkhal.sijill\0");
    mudkhal.extend_from_slice(&[0x00; 512]);
    let _ = uktub(&jidhr, SLOT_TAARIB, &mudkhal);

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    assert!(taqrir.slot_mutah(), "Taarib's own loader in Taarib's slot is a reinstall");
    assert!(taqrir.yumkin(), "a reinstall must not be refused");
    assert!(taqrir.ghurabaa().is_empty(), "Taarib's own loader is not a third party");
    let _ = fs::remove_dir_all(&jidhr);
}
