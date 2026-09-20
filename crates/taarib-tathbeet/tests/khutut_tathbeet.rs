//! The faces a package names, resolved against every font root the machine has.
//!
//! A Unity install places the patch's own faces beside the package, and it
//! finds them by the name and the BLAKE3 the package recorded. There are two
//! stores those faces can have come from — the user's own font directory and
//! the read-only set the build ships — and for a whole phase the installer was
//! handed only the first while every compile path chose from both. A patch
//! built with a bundled face compiled, was published, and then refused to
//! install anywhere, with a message telling the user to update Taarib: which
//! re-shipped the same read-only set to the same directory the installer was
//! not searching.
//!
//! So this file proves the pair together and never separately:
//!
//! 1. a face in the bundled set resolves, and refuses when the installer is
//!    handed only the user's root — the regression itself, stated as an
//!    assertion rather than as prose;
//! 2. a face in the user's own directory still resolves;
//! 3. a face in neither root refuses, and the refusal names the one action that
//!    can produce it — Settings → Fonts, not the updater;
//! 4. a face whose bytes are not the ones the package was shaped against is
//!    refused in either root, under a name that matches exactly. The
//!    fingerprint is the whole point of the lookup: a face resolved by name
//!    alone is a different font, and every precomputed layout in the package
//!    was shaped against this one;
//! 5. and the two failures are worded apart, because "update Taarib" is right
//!    for a face the build owes the user and is a dead end for one it will
//!    never contain.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::path::{Path, PathBuf};

use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_tarqee::irtibat::BasmatKhatt;
use taarib_tathbeet::khata::{KhataTathbeet, MasdarKhatt};
use taarib_tathbeet::masar_tathbeet::{JidhrKhutut, muhtawa_khutut};
use taarib_usus::khata::{Khutwa, QismIdadat, Tafsir as _};
use tempfile::TempDir;

/// The bytes standing in for a face. Not a real font: nothing below parses one,
/// and what is under test is the name-and-fingerprint lookup.
const BAYT_KHATT: &[u8] = b"\0\x01\0\0 the bytes the package was shaped against";

/// The same name carrying different bytes — a face replaced under the name the
/// package recorded.
const BAYT_AKHAR: &[u8] = b"\0\x01\0\0 a different file under the same name";

/// The two roots a studio session has, staged the way the product stages them:
/// the user's own directory, and a read-only set the build ships with a family
/// directory per class.
struct Makhzanan {
    _dalil: TempDir,
    mustakhdim: PathBuf,
    bina: PathBuf,
}

impl Makhzanan {
    fn hayyi() -> Self {
        let dalil = tempfile::tempdir().expect("a temporary directory");
        let mustakhdim = dalil.path().join("bayanat/khutut");
        let bina = dalil.path().join("mawarid/khutut");
        std::fs::create_dir_all(&mustakhdim).expect("the user's font directory");
        std::fs::create_dir_all(bina.join("sans")).expect("the bundled font directory");
        Self {
            _dalil: dalil,
            mustakhdim,
            bina,
        }
    }

    /// Both roots in the order a session reads them, which is the order an
    /// install must search them in.
    fn judhur(&self) -> Vec<JidhrKhutut> {
        vec![
            JidhrKhutut::mustakhdim(&self.mustakhdim),
            JidhrKhutut::bina(&self.bina),
        ]
    }

    /// The user's root alone — the list the installer was given before this
    /// change, kept so the regression can be asserted rather than described.
    fn jidhr_mustakhdim_wahdahu(&self) -> Vec<JidhrKhutut> {
        vec![JidhrKhutut::mustakhdim(&self.mustakhdim)]
    }
}

fn ida(masar: &Path, bayt: &[u8]) {
    std::fs::write(masar, bayt).expect("writing a font into a store");
}

/// The record a package carries for one face: the name it will be placed under
/// and the fingerprint every layout in it was shaped against.
fn sijill(ism: &str, bayt: &[u8]) -> BasmatKhatt {
    BasmatKhatt {
        ism: ism.to_owned(),
        basma: Basma::min_bayt(*blake3::hash(bayt).as_bytes()),
        alam: 0,
    }
}

fn khatt_mafqud(khata: &KhataTathbeet) -> (&str, MasdarKhatt, &[PathBuf], &str) {
    match khata {
        KhataTathbeet::KhattMafqud {
            ism,
            masdar,
            judhur,
            sabab,
        } => (ism, *masdar, judhur, sabab),
        akhar => panic!("expected KhattMafqud, got {akhar:?}"),
    }
}

#[test]
fn khatt_min_hazmat_al_bina_yuthabbat() {
    let makhzanan = Makhzanan::hayyi();
    ida(
        &makhzanan.bina.join("sans/IBMPlexSansArabic-Regular.ttf"),
        BAYT_KHATT,
    );
    let khutut = [sijill("IBMPlexSansArabic-Regular.ttf", BAYT_KHATT)];

    // The regression, first: the list the installer used to be handed cannot
    // resolve this face at all, and the refusal it produces is the one that
    // sent the user to the updater.
    let qabl = muhtawa_khutut(
        AilatMuharrik::Unity,
        &khutut,
        &makhzanan.jidhr_mustakhdim_wahdahu(),
    )
    .expect_err("one root cannot see the set the build ships");
    assert!(
        matches!(qabl, KhataTathbeet::KhattMafqud { .. }),
        "the old single-root call is what refused a patch that was compiled from this very file"
    );

    let muhtawa = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect("both roots resolve a bundled face");
    assert_eq!(muhtawa.len(), 1);
    assert_eq!(
        muhtawa[0].wajha.nisbi(),
        "taarib/khutut/IBMPlexSansArabic-Regular.ttf",
        "the face is placed under taarib/khutut/, where the takeover reads it"
    );
    assert_eq!(
        muhtawa[0].bayt, BAYT_KHATT,
        "the bytes placed are the bytes the fingerprint was taken over"
    );
}

#[test]
fn khatt_al_mustakhdim_yuthabbat() {
    let makhzanan = Makhzanan::hayyi();
    ida(&makhzanan.mustakhdim.join("Kuufi.ttf"), BAYT_KHATT);
    let khutut = [sijill("Kuufi.ttf", BAYT_KHATT)];

    let muhtawa = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect("a face the user imported still resolves");
    assert_eq!(muhtawa.len(), 1);
    assert_eq!(muhtawa[0].bayt, BAYT_KHATT);
}

#[test]
fn khatt_laysa_fi_ayy_jidhr_yurfad_ila_al_idadat() {
    let makhzanan = Makhzanan::hayyi();
    let khutut = [sijill("KhattKhass.ttf", BAYT_KHATT)];

    let khata = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect_err("a face nobody on this machine has cannot be installed");
    let (ism, masdar, judhur, sabab) = khatt_mafqud(&khata);
    assert_eq!(ism, "KhattKhass.ttf");
    assert_eq!(
        masdar,
        MasdarKhatt::Mustakhdim,
        "no root the build ships carries the name, so only the user can supply it"
    );
    assert_eq!(
        judhur,
        [makhzanan.mustakhdim.clone(), makhzanan.bina.clone()],
        "the refusal names every store it searched, not the first one"
    );
    assert!(
        sabab.contains(&makhzanan.bina.display().to_string()),
        "the bundled root has to appear in the detail, or the message repeats the old lie"
    );

    // The action is the one that can actually produce this file. The updater
    // cannot: no Taarib release will ever contain a font a contributor added to
    // their own machine.
    assert_eq!(
        khata.khutwa(),
        Khutwa::FathIdadat {
            qism: QismIdadat::Khutut
        }
    );
    assert!(khata.injilizi().contains("Settings → Fonts"));
    assert!(khata.injilizi().contains("no Taarib update will bring it"));
    assert!(!khata.injilizi().contains("Update Taarib and try again"));
    assert!(khata.arabi().contains("الإعدادات ← الخطوط"));
    assert!(!khata.arabi().contains("حدِّث تعريب ثم أعد المحاولة"));
    assert!(!khata.qabil_lil_iada());
}

#[test]
fn khatt_bina_naqis_yurfad_ila_al_tahdith() {
    let makhzanan = Makhzanan::hayyi();
    // The build ships a file under this name, and it is not the one the patch
    // was shaped against — the build is a version behind the one that compiled
    // the package.
    ida(
        &makhzanan.bina.join("sans/IBMPlexSansArabic-Regular.ttf"),
        BAYT_AKHAR,
    );
    let khutut = [sijill("IBMPlexSansArabic-Regular.ttf", BAYT_KHATT)];

    let khata = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect_err("a name that matches and bytes that do not is not the face");
    let (_, masdar, _, sabab) = khatt_mafqud(&khata);
    assert_eq!(
        masdar,
        MasdarKhatt::Bina,
        "a root the build ships carries the name, so the build owes the file"
    );
    assert!(sabab.contains("not the one the package was shaped against"));
    assert_eq!(khata.khutwa(), Khutwa::TahdithTaarib);
    assert!(khata.injilizi().contains("Update Taarib and try again"));
    assert!(khata.arabi().contains("حدِّث تعريب"));
}

#[test]
fn basma_mukhtalifa_fi_jidhr_al_mustakhdim_turfad() {
    let makhzanan = Makhzanan::hayyi();
    ida(&makhzanan.mustakhdim.join("Kuufi.ttf"), BAYT_AKHAR);
    let khutut = [sijill("Kuufi.ttf", BAYT_KHATT)];

    let khata = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect_err("the fingerprint is not a formality");
    let (_, masdar, _, sabab) = khatt_mafqud(&khata);
    assert_eq!(masdar, MasdarKhatt::Mustakhdim);
    assert!(sabab.contains("not the one the package was shaped against"));
    assert_eq!(
        khata.khutwa(),
        Khutwa::FathIdadat {
            qism: QismIdadat::Khutut
        },
        "the file under that name is the user's, and replacing it is the user's to do"
    );
}

#[test]
fn awwal_jidhr_yuwafiq_al_basma_huwa_al_maqbul() {
    let makhzanan = Makhzanan::hayyi();
    // The same name in both roots, and only the bundled copy is the file the
    // package was shaped against. Stopping at the first *name* would refuse an
    // install that has the right face sitting in the next root.
    ida(&makhzanan.mustakhdim.join("Amiri.ttf"), BAYT_AKHAR);
    ida(&makhzanan.bina.join("sans/Amiri.ttf"), BAYT_KHATT);
    let khutut = [sijill("Amiri.ttf", BAYT_KHATT)];

    let muhtawa = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect("the search continues past a name whose bytes are wrong");
    assert_eq!(muhtawa[0].bayt, BAYT_KHATT);
}

#[test]
fn muharrik_ghayr_unity_la_yatlub_khattan() {
    let makhzanan = Makhzanan::hayyi();
    let khutut = [sijill("LaYujad.ttf", BAYT_KHATT)];
    for aila in [
        AilatMuharrik::Unreal,
        AilatMuharrik::Godot,
        AilatMuharrik::Majhul,
    ] {
        let muhtawa = muhtawa_khutut(aila, &khutut, &makhzanan.judhur())
            .expect("only the Unity takeover draws with the patch's own faces");
        assert!(muhtawa.is_empty());
    }
}

#[test]
fn ism_yahmil_mujalladan_yurfad_qabl_al_bahth() {
    let makhzanan = Makhzanan::hayyi();
    let khutut = [sijill("../../Kuufi.ttf", BAYT_KHATT)];

    let khata = muhtawa_khutut(AilatMuharrik::Unity, &khutut, &makhzanan.judhur())
        .expect_err("a recorded name that is not a bare file name is not a store entry");
    assert!(
        matches!(khata, KhataTathbeet::MasarKharij { .. }),
        "expected MasarKharij, got {khata:?}"
    );
}
