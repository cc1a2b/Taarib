//! Installing beside a mod that is already in the game, and leaving it alone.
//!
//! The situation this proves is a real one on the machine it was written on:
//! `Resident Evil 4`'s `Bin32/` holds **`re4_tweaks`**, which is two DLL-proxy
//! hooks at once — `dinput8.dll` with its `dinput8.ini` and a live
//! `dinput8.log`, and a second proxy at `winmm.dll` — plus the mod's own
//! `re4_tweaks/` directory. Taarib's loader is published as `version.dll`, so
//! the slot it wants is free and the two can share the process.
//!
//! **The game directory below is authored, not the real one.** The layout, the
//! file names and the sizes are copied from that installation; the bytes are
//! this test's own, because a test must not depend on a Steam library being
//! mounted and must never write into one. The real directory was surveyed
//! separately, read-only, and `wukala`'s own tests carry the same layout.
//!
//! Three questions, in the order the product asks them:
//!
//! 1. does the survey see a mod that holds no slot Taarib wants, and let the
//!    install proceed while naming it;
//! 2. does the install **refuse by name** when the slot Taarib's own loader
//!    needs is already held;
//! 3. after an install and an uninstall, is every file the other mod owns
//!    byte-for-byte what it was.
//!
//! The third is the one that matters most. Taarib asks people to let it modify
//! a game they paid for, and a game that already has somebody else's mod in it
//! is a game where "uninstall removes only what Taarib installed" has to be a
//! measurement rather than a claim.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    clippy::print_stderr,
    reason = "a test reports failure by panicking and reports a skipped proof on stderr; the \
              lints are written for library code, and honouring them here would mean a test \
              that cannot fail and a skip nobody can see"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Muharrik};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::tarkib::{
    HalatIdadat, LubaMuhallala, NatijatTarkib, rakkib_itar,
};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_nass};
use taarib_tathbeet::wukala;
use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

/// The mod's files in `Bin32/`, with the sizes the real installation has.
///
/// Sizes are capped when the fixture is written — an 11 MB file proves nothing
/// here that 64 bytes does not — but the names are exact, because the names are
/// what the survey reads.
const RE4_TWEAKS: [(&str, usize); 5] = [
    ("dinput8.dll", 11_760_128),
    ("dinput8.ini", 22_044),
    ("dinput8.log", 42_726),
    ("winmm.dll", 80_384),
    ("re4_tweaks/trainer.ini", 5_956),
];

/// The game's own files beside them.
const LUBA_NAFSUHA: [(&str, usize); 3] =
    [("bio4.exe", 9_139_840), ("steam_api.dll", 106_408), ("steam_appid.txt", 6)];

/// The largest fixture body written, so the test stays fast and the proof stays
/// about names and bytes rather than about volume.
const AQSA_JISM: usize = 512;

/// Writes a file with deterministic, position-dependent content.
///
/// Not zeroes: the uninstall proof compares before and after, and a directory
/// full of identical zero-filled files would pass that comparison even if the
/// uninstall had swapped two of them.
fn iktub(masar: &Path, hajm: usize) {
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).expect("a fixture directory");
    }
    let tul = hajm.min(AQSA_JISM);
    let ism = masar.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let badhra = ism.bytes().fold(17_u8, |akk, bayt| akk.wrapping_mul(31).wrapping_add(bayt));
    let jism: Vec<u8> = (0..tul)
        .map(|mawdi| badhra.wrapping_add(u8::try_from(mawdi % 251).unwrap_or(0)))
        .collect();
    fs::write(masar, jism).expect("a fixture file");
}

/// Builds `Bin32/` as it is on the machine this was written against.
fn ibni_bin32(jidhr: &Path) {
    for (nisbi, hajm) in RE4_TWEAKS.into_iter().chain(LUBA_NAFSUHA) {
        iktub(&jidhr.join(nisbi), hajm);
    }
    // The mod's own subdirectories, which an uninstall must not walk into.
    fs::create_dir_all(jidhr.join("re4_tweaks/sideload/op")).expect("the mod's tree");
    iktub(&jidhr.join("re4_tweaks/sideload/info.txt"), 404);
}

/// Builds the component store entry `MukawwinItar::mudkhal` names for a 32-bit
/// Windows game: the loader in `muhammil/`, and one payload beside it.
///
/// `TawziMukawwin::Munfasil` is what puts them where they have to be —
/// everything under `muhammil/` lands beside the executable, which is the only
/// place `taarib-mudkhal` looks for a payload, and everything else lands under
/// the game's own `taarib/` directory.
fn ibni_makhzan(jidhr: &Path) {
    let mukawwin = jidhr.join("mudkhal/windows/x86");
    iktub(&mukawwin.join("muhammil/version.dll"), 96_000);
    iktub(&mukawwin.join("muhammil/taarib_tabaqa.dll"), 480_000);
    iktub(&mukawwin.join("iqra.txt"), 128);
}

fn tarif(jidhr: &Path) -> TarifLuba {
    let masdar = MasdarLuba::Steam(254_700);
    TarifLuba {
        luba: LubaId::min_masdar(&masdar, "Resident Evil 4"),
        masdar,
        ism: "Resident Evil 4".to_owned(),
        jidhr: jidhr.to_path_buf(),
        ruqaa: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::jadeeda(1),
        basma_bina: None,
    }
}

/// The game as the installer resolved it: BIO4, 32-bit, Windows, no prefix.
///
/// The game root is `Bin32/` itself, because that is where the executable sits
/// and the framework's loader goes beside the executable.
fn luba_muhallala(jidhr: &Path) -> LubaMuhallala {
    LubaMuhallala {
        jidhr: jidhr.to_path_buf(),
        masar_tanfidhi: jidhr.join("bio4.exe"),
        muharrik: Muharrik {
            aila: AilatMuharrik::Bio4,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Majhula,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X86,
            thiqa: 96,
            dalail: Vec::new(),
        },
        beea: BeeatTawafuq::Asli,
        nizam: NizamTashghil::Windows,
        masdar: MasdarLuba::Steam(254_700),
    }
}

/// Every file under `jidhr`, keyed by its path relative to it, with its bytes.
fn basmat_shajara(jidhr: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut jadwal = BTreeMap::new();
    for madkhal in walkdir::WalkDir::new(jidhr).sort_by_file_name() {
        let madkhal = madkhal.expect("walking the game directory");
        if !madkhal.file_type().is_file() {
            continue;
        }
        let nisbi = madkhal
            .path()
            .strip_prefix(jidhr)
            .expect("every entry is under the root")
            .to_string_lossy()
            .replace('\\', "/");
        let bayt = fs::read(madkhal.path()).expect("reading a game file");
        let _ = jadwal.insert(nisbi, bayt);
    }
    jadwal
}

/// The subset of a tree that belongs to the other mod.
fn milk_almod(jadwal: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    jadwal
        .iter()
        .filter(|(nisbi, _)| {
            nisbi.starts_with("re4_tweaks/")
                || nisbi.starts_with("dinput8.")
                || nisbi.as_str() == "winmm.dll"
        })
        .map(|(nisbi, bayt)| (nisbi.clone(), bayt.clone()))
        .collect()
}

#[test]
fn taarib_yathbut_bijanib_re4_tweaks_wa_yusammih() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("Bin32");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_bin32(&luba);
    ibni_makhzan(&makhzan);

    // What the survey sees on its own, before any install decision rests on it.
    let masah = wukala::masah(&luba).expect("the survey reads the game directory");
    let asmaa: Vec<&str> = masah.iter().map(|wakeel| wakeel.ism.as_str()).collect();
    assert_eq!(
        asmaa,
        vec!["dinput8.dll", "winmm.dll"],
        "re4_tweaks holds two loader slots; the game's own steam_api.dll holds none"
    );

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let natija = rakkib_itar(
        &luba_muhallala(&luba),
        &HalatIdadat::default(),
        &makhzan,
        &mut tathbeet,
    )
    .expect("version.dll is free, so the install proceeds");

    let NatijatTarkib::Nushira(munaffadh) = natija else {
        panic!("a free slot deploys the loader; got {natija:?}");
    };
    assert_eq!(munaffadh.mukawwin, "mudkhal/windows/x86");
    assert!(
        luba.join("version.dll").is_file(),
        "the loader landed beside the executable, in the slot nothing else held"
    );
    assert!(
        luba.join("taarib/iqra.txt").is_file(),
        "and the component's non-loader files landed under the game's own Taarib directory"
    );

    // The disclosure. An install into a game that already has a mod in it is a
    // different thing to agree to, and this is where the report says so.
    let mujawir: Vec<&str> =
        munaffadh.huqn_mujawir.iter().map(|wakeel| wakeel.ism.as_str()).collect();
    assert_eq!(mujawir, vec!["dinput8.dll", "winmm.dll"]);
    let dinput = munaffadh.huqn_mujawir.first().expect("the first neighbour");
    assert_eq!(
        dinput.rifaq,
        vec!["dinput8.ini".to_owned(), "dinput8.log".to_owned()],
        "the proxy's settings and its log are named after the slot, and the report says so"
    );
    let sutur = NatijatTarkib::Nushira(munaffadh).taqreer().join("\n");
    assert!(sutur.contains("dinput8.dll"), "the install report names it:\n{sutur}");
    assert!(sutur.contains("did not touch it"), "and says what Taarib did about it");
}

#[test]
fn wakeel_mashghul_yarfud_bil_ism_wala_yaktub_shayan() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("Bin32");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_bin32(&luba);
    ibni_makhzan(&makhzan);
    // A third mod, in the one slot Taarib needs. Ultimate ASI Loader publishes
    // itself under this name among others, so this is not a hypothetical.
    iktub(&luba.join("version.dll"), 80_384);

    let qabl = basmat_shajara(&luba);

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let khata = rakkib_itar(
        &luba_muhallala(&luba),
        &HalatIdadat::default(),
        &makhzan,
        &mut tathbeet,
    )
    .expect_err("two loaders cannot share one slot, so the install refuses");

    match &khata {
        KhataTathbeet::WakeelMashghul { wakeel, masar, hajm, jiran } => {
            assert_eq!(wakeel, "version.dll");
            assert_eq!(masar, &luba.join("version.dll"));
            assert_eq!(*hajm, u64::try_from(AQSA_JISM).expect("a small constant"));
            // The refusal names the rest of what is in the game, because "your
            // slot is taken" without "and here is the mod that took it" is a
            // message the reader cannot act on.
            assert!(
                jiran.iter().any(|satr| satr.contains("dinput8.dll")),
                "the refusal names the other loaders beside it: {jiran:?}"
            );
        }
        akhar => panic!("expected WakeelMashghul, got {akhar:?}"),
    }

    let injilizi = taarib_usus::khata::Tafsir::injilizi(&khata);
    assert!(injilizi.contains("version.dll"));
    assert!(injilizi.contains("Nothing was written."));
    assert!(taarib_usus::khata::Tafsir::arabi(&khata).contains("لم يُكتب شيء"));

    assert_eq!(
        basmat_shajara(&luba),
        qabl,
        "a refusal that had already written something would not be a refusal"
    );
}

#[test]
fn ilgha_altathbeet_la_yamiss_milk_almod_alakhar() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("Bin32");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_bin32(&luba);
    ibni_makhzan(&makhzan);

    let qabl = basmat_shajara(&luba);
    let milk_qabl = milk_almod(&qabl);
    assert_eq!(milk_qabl.len(), 6, "the other mod owns six files: {:?}", milk_qabl.keys());

    {
        let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
            .expect("an installation session");
        let natija = rakkib_itar(
            &luba_muhallala(&luba),
            &HalatIdadat::default(),
            &makhzan,
            &mut tathbeet,
        )
        .expect("the install");
        assert!(matches!(natija, NatijatTarkib::Nushira(_)));
    }

    assert!(luba.join("version.dll").is_file(), "Taarib's loader is in the game");

    let taqreer = istiada_nass(&luba, &nusakh, SiyasatIstiada::Muhafiza, &mut RadLaShay)
        .expect("the uninstall completes");
    assert!(
        taqreer.nazif(),
        "an uninstall that left something behind proves nothing: {taqreer:?}"
    );

    // Taarib's own work is gone.
    assert!(!luba.join("version.dll").exists(), "the loader came out");
    assert!(!luba.join("taarib").exists(), "and so did the directory it created");

    // And the other mod is byte-for-byte what it was — not merely present, and
    // not merely the right length.
    let baad = basmat_shajara(&luba);
    assert_eq!(
        milk_almod(&baad),
        milk_qabl,
        "every file re4_tweaks owns is exactly the bytes it was before the install"
    );
    // Nothing else moved either: the game's own files, and the mod's, together.
    assert_eq!(baad, qabl, "the directory is exactly what it was before Taarib touched it");

    // The mod's directories are still directories, not casualties of the
    // deepest-first empty-directory sweep.
    assert!(luba.join("re4_tweaks/sideload/op").is_dir());
}

/// Names a **copy** of a real game directory to run the whole thing over.
///
/// The three tests above are hermetic and are what CI runs. This one is how the
/// same claims were checked against the actual bytes of an actual modded game
/// on the machine this was written on, without any of that reaching a
/// repository or a build server. It copies the directory again before touching
/// it, so even the copy the variable names is left alone.
const MUTAGHAYYIR_HAQIQI: &str = "TAARIB_MUJALLAD_LUBA_HAQIQI";

/// Copies a directory tree, files and directories, preserving nothing else.
fn insakh(min: &Path, ila: &Path) {
    fs::create_dir_all(ila).expect("the destination directory");
    for madkhal in walkdir::WalkDir::new(min).sort_by_file_name() {
        let madkhal = madkhal.expect("walking the source tree");
        let nisbi = madkhal.path().strip_prefix(min).expect("every entry is under the root");
        if nisbi.as_os_str().is_empty() {
            continue;
        }
        let hadaf = ila.join(nisbi);
        if madkhal.file_type().is_dir() {
            fs::create_dir_all(&hadaf).expect("a copied directory");
        } else if madkhal.file_type().is_file() {
            if let Some(walid) = hadaf.parent() {
                fs::create_dir_all(walid).expect("a copied file's directory");
            }
            let _ = fs::copy(madkhal.path(), &hadaf).expect("copying a file");
        }
    }
}

#[test]
fn ala_mujallad_haqiqi_in_wujid() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the ban is on configuration being read ad hoc; this is a test asking whether \
                  a real game directory was offered to it, and nothing in the product reads it"
    )]
    let mawjud = std::env::var(MUTAGHAYYIR_HAQIQI);
    let Ok(masdar) = mawjud else {
        // Not a silent pass: a proof that did not run has to say so, or a
        // reader takes the green tick for evidence it is not.
        eprintln!(
            "skipped: set {MUTAGHAYYIR_HAQIQI} to a copy of a real game directory to run the \
             survey, the install and the uninstall over it"
        );
        return;
    };
    let masdar = PathBuf::from(masdar);
    assert!(masdar.is_dir(), "{MUTAGHAYYIR_HAQIQI} must name a directory: {}", masdar.display());

    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("luba");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    insakh(&masdar, &luba);
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_makhzan(&makhzan);

    let qabl = basmat_shajara(&luba);
    let masah = wukala::masah(&luba).expect("the survey");
    let asmaa: Vec<&str> = masah.iter().map(|wakeel| wakeel.ism.as_str()).collect();
    eprintln!("loader slots in use: {asmaa:?}");

    let tanfidhi = fs::read_dir(&luba)
        .expect("listing the game directory")
        .filter_map(Result::ok)
        .map(|madkhal| madkhal.file_name().to_string_lossy().into_owned())
        .find(|ism| ism.to_ascii_lowercase().ends_with(".exe"))
        .expect("a real game directory has an executable in it");

    let mut muhallala = luba_muhallala(&luba);
    muhallala.masar_tanfidhi = luba.join(&tanfidhi);

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let natija =
        rakkib_itar(&muhallala, &HalatIdadat::default(), &makhzan, &mut tathbeet);

    match natija {
        Ok(NatijatTarkib::Nushira(munaffadh)) => {
            eprintln!("installed: {}", NatijatTarkib::Nushira(munaffadh).taqreer().join("\n"));
            drop(tathbeet);
            let taqreer = istiada_nass(&luba, &nusakh, SiyasatIstiada::Muhafiza, &mut RadLaShay)
                .expect("the uninstall completes");
            assert!(taqreer.nazif(), "{taqreer:?}");
            assert_eq!(
                basmat_shajara(&luba),
                qabl,
                "every byte of the real game directory, third-party mod included, is what it \
                 was before the install"
            );
        }
        Ok(akhar) => panic!("expected a deployment, got {akhar:?}"),
        Err(khata) => {
            // A refusal is also a correct outcome for a real directory — it is
            // the outcome when Taarib's own slot is taken — but it must have
            // changed nothing.
            assert!(matches!(khata, KhataTathbeet::WakeelMashghul { .. }), "{khata:?}");
            assert_eq!(basmat_shajara(&luba), qabl, "a refusal writes nothing");
        }
    }
}
