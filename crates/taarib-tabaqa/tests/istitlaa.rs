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

use taarib_tabaqa::istitlaa::{AQSA_MALAF, HalatSlot, SLOT_TAARIB, TaqrirIstitlaa, istatli};
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
        + u32::try_from(asmaa.len().saturating_add(1))
            .unwrap_or(0)
            .saturating_mul(20);
    for (fahras, ism) in asmaa.iter().enumerate() {
        let wasf =
            usize::try_from(IZAHAT_QISM + IZAHAT_USTUWANA).unwrap_or(0) + fahras.saturating_mul(20);
        // The original-thunk and first-thunk fields only have to be non-zero,
        // because the terminator is an all-zero descriptor and this walk reads
        // the name and those two fields alone.
        bayt[wasf..wasf + 4].copy_from_slice(&(OINWAN_QISM + 0x0700).to_le_bytes());
        bayt[wasf + 12..wasf + 16].copy_from_slice(&(OINWAN_QISM + izahat_ism).to_le_bytes());
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
            taqrir
                .mustawradat
                .iter()
                .any(|name| name.eq_ignore_ascii_case("d3d9.dll")),
            "{ism} must name the module it was recognised by"
        );
        assert!(
            taqrir.yumkin(),
            "{ism} has a free proxy slot and must be installable"
        );
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
    let masar = uktub(
        &jidhr,
        "luba.exe",
        &banni_pe(false, &["KERNEL32.dll", "USER32.dll"]),
    );
    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    assert!(taqrir.wajihat.is_empty(), "no graphics module was imported");
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Naqisa,
        "an unknown renderer narrows, it does not stop"
    );
    assert!(
        taqrir.yumkin(),
        "an unknown renderer must not stop an installation"
    );
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
    assert!(
        istatli(&masar).is_err(),
        "a non-PE must be refused, not surveyed as empty"
    );
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

    assert!(
        !taqrir.slot_mutah(),
        "{SLOT_TAARIB} is taken and must not be reported as free"
    );
    assert!(
        matches!(taqrir.halat_slot(), HalatSlot::Mashghul { .. }),
        "a readable third-party file is 'taken', not 'unread': {:?}",
        taqrir.halat_slot()
    );
    assert!(taqrir.thughrat.is_empty(), "every slot was readable");
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Mustaheela,
        "a taken {SLOT_TAARIB} must stop the installation outright"
    );
    assert!(
        !taqrir.yumkin(),
        "the overlay must not be offered where its own slot is taken"
    );

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
    assert!(
        !arabi.trim().is_empty(),
        "the refusal must be readable in Arabic too"
    );
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

    assert!(
        taqrir.slot_mutah(),
        "Taarib's own slot is free and must be reported so"
    );
    assert_eq!(taqrir.halat_slot(), HalatSlot::Hurr);
    assert!(
        taqrir.yumkin(),
        "a proxy Taarib does not use must not stop an installation"
    );
    assert_eq!(
        taqrir.ghurabaa().len(),
        1,
        "the third-party proxy must be reported"
    );
    let Some(ghareeb) = taqrir.ghurabaa().first().copied() else {
        panic!("the third-party proxy was not reported");
    };
    assert_eq!(ghareeb.ism, "dinput8.dll");
    assert_eq!(
        ghareeb.hajm, 4096,
        "the size is what lets a user recognise the product"
    );
    assert!(
        !ghareeb.taarib,
        "another product's proxy must not be mistaken for Taarib's"
    );
    assert!(
        taqrir.asbab.iter().any(|sabab| sabab.injilizi.contains(
            "never restores a function \
             pointer it did not install"
        )),
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
    assert!(
        taqrir.slot_mutah(),
        "Taarib's own loader in Taarib's slot is a reinstall"
    );
    assert_eq!(taqrir.halat_slot(), HalatSlot::Taarib);
    assert!(taqrir.yumkin(), "a reinstall must not be refused");
    assert!(
        taqrir.ghurabaa().is_empty(),
        "Taarib's own loader is not a third party"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

// ---------------------------------------------------------------------------
// A slot that could not be read is not a free slot
// ---------------------------------------------------------------------------

/// What every unread-Taarib-slot case must look like, whatever made it unread.
///
/// The verdict is "not determined": not "taken", which would assert a
/// competitor about bytes nothing read, and not "free", which would be the
/// overwrite the survey exists to prevent. The install is refused, the fold
/// says `false`, the four-state answer says *why*, and the sentence says what
/// refused rather than who owns the file.
fn taakkad_slot_ghayr_maqru(taqrir: &TaqrirIstitlaa, matlub: &str) {
    let HalatSlot::GhayrMaqru { thughra } = taqrir.halat_slot() else {
        panic!(
            "an unread {SLOT_TAARIB} must answer as unread, not {:?}",
            taqrir.halat_slot()
        );
    };
    assert!(thughra.ism.eq_ignore_ascii_case(SLOT_TAARIB));
    assert!(
        thughra.sabab.contains(matlub),
        "the gap must say what refused: {}",
        thughra.sabab
    );
    assert!(
        !taqrir.slot_mutah(),
        "the fold of 'unread' is 'do not write'"
    );
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Majhula,
        "unread is neither taken nor free"
    );
    assert!(
        !taqrir.yumkin(),
        "Taarib does not write over a file it could not read"
    );
    assert!(
        taqrir
            .mashghula
            .iter()
            .all(|slot| !slot.ism.eq_ignore_ascii_case(SLOT_TAARIB)),
        "an unread slot must not also be listed as occupied"
    );

    let majhula: Vec<&str> = taqrir
        .asbab
        .iter()
        .filter(|sabab| sabab.hukm == HukmQudra::Majhula)
        .map(|sabab| sabab.injilizi.as_str())
        .collect();
    assert_eq!(
        majhula.len(),
        1,
        "one unread slot, one unanswered question: {majhula:?}"
    );
    let Some(injilizi) = majhula.first() else {
        panic!("the unanswered question carried no sentence");
    };
    assert!(
        injilizi.contains("could not read"),
        "the sentence names the gap: {injilizi}"
    );
    assert!(
        !injilizi.contains("taken by another product"),
        "the sentence must not assert a competitor it did not see: {injilizi}"
    );
    assert!(
        !taqrir
            .asbab
            .iter()
            .any(|sabab| sabab.injilizi.contains("none of the loader names")),
        "the all-clear is a claim about every name and one was not read"
    );
    assert!(
        taqrir
            .sutur()
            .iter()
            .any(|satr| satr.starts_with("unread:")),
        "the bundle must lead with what was not read: {:?}",
        taqrir.sutur()
    );
    let arabi = taqrir
        .asbab
        .iter()
        .find(|sabab| sabab.hukm == HukmQudra::Majhula)
        .map(|sabab| sabab.arabi.clone())
        .unwrap_or_default();
    assert!(
        !arabi.trim().is_empty(),
        "the gap must be readable in Arabic too"
    );
}

/// A `version.dll` past the size this survey will load is unread, not
/// somebody else's.
///
/// This is the case the old code got backwards in words: it refused the write
/// — correctly — with a sentence asserting "taken by another product" about a
/// file that could as easily have been Taarib's own loader. A sparse file
/// produces the size without the bytes on every platform.
#[test]
fn slot_dakhm_ghayr_maqru_laysa_maakhudhan() {
    let jidhr = mujallad("slot-dakhm");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    let dakhm = jidhr.join(SLOT_TAARIB);
    match fs::File::create(&dakhm).and_then(|malaf| malaf.set_len(AQSA_MALAF.saturating_add(1))) {
        Ok(()) => {},
        Err(khata) => panic!("the oversized fixture could not be made: {khata}"),
    }

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    taakkad_slot_ghayr_maqru(&taqrir, "ceiling");
    let HalatSlot::GhayrMaqru { thughra } = taqrir.halat_slot() else {
        panic!("asserted above");
    };
    assert_eq!(
        thughra.hajm,
        Some(AQSA_MALAF.saturating_add(1)),
        "the size was readable even though the bytes were not, and is carried"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// A graphics slot past the cap narrows the verdict and records the gap; it
/// does not stop anything, because nothing in a slot Taarib does not use can.
#[test]
fn slot_rusum_ghayr_maqru_yunqis_wa_yusajjal() {
    let jidhr = mujallad("rusum-dakhm");
    let masar = uktub(
        &jidhr,
        "luba.exe",
        &banni_pe(true, &["d3d11.dll", "dxgi.dll"]),
    );
    match fs::File::create(jidhr.join("dxgi.dll"))
        .and_then(|malaf| malaf.set_len(AQSA_MALAF.saturating_add(1)))
    {
        Ok(()) => {},
        Err(khata) => panic!("the oversized fixture could not be made: {khata}"),
    }

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    assert_eq!(
        taqrir.halat_slot(),
        HalatSlot::Hurr,
        "Taarib's own slot is untouched"
    );
    assert!(
        taqrir.yumkin(),
        "an unread bystander does not stop an installation"
    );
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Naqisa,
        "the worst a graphics slot can be is a wrapper"
    );
    assert_eq!(taqrir.thughrat.len(), 1, "and the gap is on record");
    assert!(
        taqrir
            .thughrat
            .first()
            .is_some_and(|thughra| thughra.ism == "dxgi.dll")
    );
    assert!(
        taqrir.ghurabaa().is_empty(),
        "an unread file is not reported as a known product"
    );
    assert!(
        !taqrir
            .asbab
            .iter()
            .any(|sabab| sabab.injilizi.contains("none of the loader names")),
        "the all-clear must not be said over an unread name"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// A `version.dll` the process is not allowed to read is unread, not free.
///
/// The metadata is readable — the size is known — and the bytes are not, which
/// is what a file an antivirus is holding looks like. Skipped, with the reason
/// printed into the assertion path rather than silently, where the process can
/// read the file regardless: root ignores mode bits.
#[cfg(unix)]
#[test]
fn slot_mamnu_ghayr_maqru_laysa_hurran() {
    use std::os::unix::fs::PermissionsExt as _;

    let jidhr = mujallad("slot-mamnu");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    let mamnu = uktub(&jidhr, SLOT_TAARIB, b"whatever is in here was never read");
    if let Err(khata) = fs::set_permissions(&mamnu, fs::Permissions::from_mode(0o000)) {
        panic!("the fixture's mode could not be changed: {khata}");
    }
    if fs::read(&mamnu).is_ok() {
        // Root reads a mode-000 file without complaint, so this environment
        // cannot produce the condition. Nothing is asserted rather than
        // something false; the sparse-file test above covers the same path.
        let _ = fs::set_permissions(&mamnu, fs::Permissions::from_mode(0o644));
        let _ = fs::remove_dir_all(&jidhr);
        return;
    }

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    taakkad_slot_ghayr_maqru(&taqrir, "could not be read");
    let HalatSlot::GhayrMaqru { thughra } = taqrir.halat_slot() else {
        panic!("asserted above");
    };
    assert!(
        thughra.hajm.is_some(),
        "the metadata was readable and its size is carried"
    );

    let _ = fs::set_permissions(&mamnu, fs::Permissions::from_mode(0o644));
    let _ = fs::remove_dir_all(&jidhr);
}

/// A `version.dll` whose metadata itself cannot be read is unread, not free.
///
/// This is the branch that used to be `continue` — indistinguishable from
/// "not there". A symlink that points at itself makes `metadata` refuse with
/// something other than `NotFound`, which is exactly what a slot behind an ACL
/// the survey cannot traverse produces.
#[cfg(unix)]
#[test]
fn slot_bila_bayanat_ghayr_maqru_laysa_hurran() {
    let jidhr = mujallad("slot-halaqa");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(false, &["d3d9.dll"]));
    if let Err(khata) = std::os::unix::fs::symlink(SLOT_TAARIB, jidhr.join(SLOT_TAARIB)) {
        panic!("the looping symlink could not be made: {khata}");
    }

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    taakkad_slot_ghayr_maqru(&taqrir, "");
    let HalatSlot::GhayrMaqru { thughra } = taqrir.halat_slot() else {
        panic!("asserted above");
    };
    assert_eq!(
        thughra.hajm, None,
        "no metadata, no size — and no invented one"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// A module carrying a product's marks the way a real one does.
///
/// The marks go in as UTF-16, which is where a real one keeps them: a Windows
/// version resource stores `CompanyName` and `ProductName` in UTF-16, and that
/// is the encoding the strings identifying `ReShade` and DXVK actually live in.
fn wahda(basmat: &[&str]) -> Vec<u8> {
    let mut jism = vec![0x00_u8; 256];
    for basma in basmat {
        jism.extend(basma.encode_utf16().flat_map(u16::to_le_bytes));
        jism.push(0);
    }
    jism.extend_from_slice(&[0x90; 256]);
    jism
}

/// A wrapper on the presentation path narrows the verdict and says why.
///
/// This is the collision that is neither a crash nor a refusal. `ReShade` hands
/// the game its own `IDXGISwapChain`, so the method table the overlay reads
/// from a swap chain of its own is not the one the game calls — the hook
/// installs, verifies, and is never invoked. Nothing breaks and nothing is
/// drawn, which is the one failure a user cannot diagnose alone, so it has to
/// be said before an install rather than discovered after one.
#[test]
fn ghilaf_alard_yunqis_wala_yamnaa_wa_yusamma() {
    let jidhr = mujallad("ghilaf-ard");
    let masar = uktub(
        &jidhr,
        "luba.exe",
        &banni_pe(true, &["d3d11.dll", "dxgi.dll"]),
    );
    let _ = uktub(
        &jidhr,
        "dxgi.dll",
        &wahda(&["crosire", "ReShade", "reshade-shaders"]),
    );

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    let Some(slot) = taqrir.ghurabaa().first().copied() else {
        panic!("the presentation hook was not reported");
    };
    assert_eq!(
        slot.muntaj,
        Some("ReShade"),
        "the product is named, not merely the slot"
    );
    assert!(
        taqrir.slot_mutah(),
        "ReShade does not hold the name Taarib uses"
    );
    assert!(
        taqrir.yumkin(),
        "a presentation hook narrows the overlay, it does not stop it"
    );
    assert_eq!(taqrir.hukm(), HukmQudra::Naqisa, "and the verdict says so");
    let injilizi: String = taqrir
        .asbab
        .iter()
        .map(|sabab| sabab.injilizi.clone())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        injilizi.contains("ReShade"),
        "the reason names it: {injilizi}"
    );
    assert!(
        injilizi.contains("never called: no crash, no Arabic"),
        "and states the failure mode exactly: {injilizi}"
    );
    assert!(
        injilizi.contains("does not unhook the other product"),
        "and keeps the unhook rule in view: {injilizi}"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// A translation layer in a graphics slot is not a hook and does not narrow.
///
/// DXVK's `d3d9.dll` *is* Direct3D 9 in that process. `Direct3DCreate9`
/// resolves to it for the overlay exactly as it did for the game, so the method
/// table the overlay reads is the one the game's device uses. One
/// implementation, one table, and the hook composes.
#[test]
fn tabaqat_tarjama_la_tunqis() {
    let jidhr = mujallad("dxvk");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(true, &["d3d9.dll"]));
    let _ = uktub(
        &jidhr,
        "d3d9.dll",
        &wahda(&["DXVK", "DxvkInstance", "zlib/libpng license"]),
    );

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    let Some(slot) = taqrir.ghurabaa().first().copied() else {
        panic!("the translation layer was not reported");
    };
    assert_eq!(slot.muntaj, Some("DXVK"));
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Kamila,
        "a replaced implementation is one method table, not two hooks"
    );
    let _ = fs::remove_dir_all(&jidhr);
}

/// Microsoft's own redistributable is not reported as somebody's mod.
///
/// `xinput1_3.dll` beside a game is the DirectX end-user redistributable far
/// more often than it is Ultimate ASI Loader, and identification is the only
/// thing that separates the two. Unidentified, it stays unmentioned; carrying a
/// loader's marks, it is reported.
#[test]
fn ism_mushtarak_la_yublagh_illa_muaarrafan() {
    let jidhr = mujallad("mushtarak");
    let masar = uktub(&jidhr, "luba.exe", &banni_pe(true, &["d3d11.dll"]));
    let _ = uktub(&jidhr, "xinput1_3.dll", &wahda(&["Microsoft Corporation"]));

    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    assert!(
        taqrir.mashghula.is_empty(),
        "Microsoft's redistributable is not a mod: {:?}",
        taqrir.mashghula
    );

    let _ = uktub(
        &jidhr,
        "xinput1_4.dll",
        &wahda(&["Alexander Blade", "asiloader"]),
    );
    let taqrir = match istatli(&masar) {
        Ok(taqrir) => taqrir,
        Err(khata) => panic!("the executable would not be surveyed: {khata}"),
    };
    let asmaa: Vec<&str> = taqrir
        .mashghula
        .iter()
        .map(|slot| slot.ism.as_str())
        .collect();
    assert_eq!(
        asmaa,
        vec!["xinput1_4.dll"],
        "identified, the same class of name is reported"
    );
    let Some(slot) = taqrir.ghurabaa().first().copied() else {
        panic!("the loader was not reported");
    };
    assert_eq!(slot.muntaj, Some("an ASI plugin loader"));
    assert_eq!(
        taqrir.hukm(),
        HukmQudra::Kamila,
        "a plugin loader is not on the drawing path"
    );
    let _ = fs::remove_dir_all(&jidhr);
}
