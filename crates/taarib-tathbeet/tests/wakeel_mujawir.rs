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
use taarib_mustalahat::muharrik::{
    AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik, Tabaqa,
    TaqreerImkaniyat,
};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::tarkib::{
    HalatIdadat, KhuttatTarkib, LubaMuhallala, NatijatTarkib, khutta, rakkib_itar,
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

/// Writes a fixture module carrying a product's mark the way a real one does.
///
/// The mark goes in as UTF-16, which is not decoration: a Windows version
/// resource stores `CompanyName` and `ProductName` as UTF-16, and that is where
/// the survey actually finds the string that names Ultimate ASI Loader or
/// `re4_tweaks` in a real installation. A fixture that embedded plain ASCII
/// would pass while leaving the encoding that matters unproven.
fn iktub_bi_basma(masar: &Path, basma: &str) {
    iktub(masar, AQSA_JISM);
    let mut jism = fs::read(masar).expect("the fixture just written");
    let utf16: Vec<u8> =
        basma.encode_utf16().flat_map(u16::to_le_bytes).collect();
    // Centred so the mark is nowhere near either end, which is where a reader
    // that only sniffed a prefix or a suffix would find it by accident.
    #[expect(
        clippy::integer_division,
        reason = "a byte offset into a fixture is a whole number of bytes; there is no \
                  precision to lose and a float here would have to be truncated back"
    )]
    let bidaya = AQSA_JISM.saturating_sub(utf16.len()) / 2;
    jism.splice(bidaya..bidaya + utf16.len(), utf16);
    jism.truncate(AQSA_JISM);
    fs::write(masar, jism).expect("the fixture with its mark");
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

/// A capability report for this game at tier 1, as `khutta` reads one.
///
/// Written out rather than produced by the probe: this crate does not depend on
/// `taarib-muharrik`, and the fields the plan consults — `marfuda`, `tabaqa` and
/// the engine — are stated plainly rather than arriving through a crate boundary
/// opened for a test. Tier 1 is what a BIO4 game gets, and it is the tier under
/// which a loader is deployed at all.
fn imkaniyat(muhallala: &LubaMuhallala) -> TaqreerImkaniyat {
    TaqreerImkaniyat {
        muharrik: muhallala.muharrik.clone(),
        tabaqa: Tabaqa::Kamil,
        sabab_arabi: "محرّك بيو٤ لا يملك نظام إضافات".to_owned(),
        sabab_injilizi: "Capcom's BIO4 has no plugin system".to_owned(),
        jahiziya: JahiziyatTashghil::Ghaiba,
        naqs: None,
        anzimat_qabila: Vec::new(),
        jawda: JawdaMutawaqqaa::Maqbula,
        hudud: Vec::new(),
        marfuda: false,
        isdar_fahs: 0,
        waqt: "2026-09-06T00:00:00Z".to_owned(),
    }
}

/// The plan the framework writer executes.
///
/// Built here rather than assumed, because `rakkib_itar` takes a plan now: the
/// component, and the directory its loader lands in, are the plan's answers and
/// no longer the writer's own.
fn khutta_li(muhallala: &LubaMuhallala, makhzan: &Path) -> KhuttatTarkib {
    khutta(&imkaniyat(muhallala), muhallala, makhzan).expect("a deployment plan")
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
    let muhallala = luba_muhallala(&luba);
    let natija = rakkib_itar(
        &khutta_li(&muhallala, &makhzan),
        &muhallala,
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
    // itself under this name among others, so this is not a hypothetical — and
    // it is given the shape a real one has: its own name inside the module, an
    // `.asi` plugin beside it, and the `scripts/` directory it loads from.
    iktub_bi_basma(&luba.join("version.dll"), "Ultimate ASI Loader");
    iktub(&luba.join("Menyoo.asi"), 4_202_496);
    fs::create_dir_all(luba.join("scripts")).expect("the loader's plugin directory");

    let qabl = basmat_shajara(&luba);

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let muhallala = luba_muhallala(&luba);
    let khata = rakkib_itar(
        &khutta_li(&muhallala, &makhzan),
        &muhallala,
        &HalatIdadat::default(),
        &makhzan,
        &mut tathbeet,
    )
    .expect_err("two loaders cannot share one slot, so the install refuses");

    match &khata {
        KhataTathbeet::WakeelMashghul { wakeel, masar, hajm, jiran, huwiya } => {
            assert_eq!(wakeel, "version.dll");
            assert_eq!(masar, &luba.join("version.dll"));
            assert_eq!(*hajm, u64::try_from(AQSA_JISM).expect("a small constant"));
            // The refusal names the product, not merely the collision. This is
            // the difference between a message a user can act on and one they
            // can only stare at.
            assert_eq!(
                huwiya.aila(),
                Some(wukala::AilatWakeel::MuhammilAsi),
                "the mod holding Taarib's slot is named: {huwiya:?}"
            );
            let kayf = huwiya.wasf_injilizi();
            assert!(
                kayf.contains("Ultimate ASI Loader") && kayf.contains("Menyoo.asi"),
                "and the evidence that named it travels with it: {kayf}"
            );
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
    assert!(
        injilizi.contains("an ASI plugin loader"),
        "the reader is told which mod is in the way: {injilizi}"
    );
    // Taarib never fights for a slot and never sneaks into another one, and the
    // message has to say the second part — otherwise "it refused" reads as a
    // limitation rather than as the deliberate refusal it is.
    assert!(
        injilizi.contains("succeeds and does nothing"),
        "the refusal argues why relocating to a free name is not the answer: {injilizi}"
    );
    assert!(
        injilizi.contains("loads every `.asi` beside it"),
        "and names the door this particular product leaves open: {injilizi}"
    );
    let arabi = taarib_usus::khata::Tafsir::arabi(&khata);
    assert!(arabi.contains("لم يُكتب شيء"));
    assert!(arabi.contains("مُحمِّل إضافات ASI"), "the identity is Arabic in Arabic: {arabi}");

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
        let muhallala = luba_muhallala(&luba);
        let natija = rakkib_itar(
            &khutta_li(&muhallala, &makhzan),
            &muhallala,
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
    let natija = rakkib_itar(
        &khutta_li(&muhallala, &makhzan),
        &muhallala,
        &HalatIdadat::default(),
        &makhzan,
        &mut tathbeet,
    );

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

/// Names a directory to run the *survey* over, without installing anything.
///
/// Separate from [`MUTAGHAYYIR_HAQIQI`] because the two prove different things.
/// That one copies a whole game and runs an install and an uninstall over it,
/// which needs an executable. This one only asks "does the identifier name the
/// mods in this directory", which needs nothing but the modules themselves —
/// so it can be pointed at the loader files of a game far too large to copy.
const MUTAGHAYYIR_MASAH: &str = "TAARIB_MUJALLAD_MASAH_HAQIQI";

/// The identifier, run over the real bytes of real mods.
///
/// Every other test in this file authors its fixtures. This one does not: it is
/// pointed at modules that a person actually installed, and it is the only
/// thing here that can catch a mark chosen from a changelog rather than from a
/// binary. Skipped loudly when no directory is offered.
#[test]
fn yusammi_alwukala_alhaqiqiyeen_in_wujidu() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the ban is on configuration being read ad hoc; this is a test asking whether \
                  a directory of real mod binaries was offered to it, and nothing in the \
                  product reads it"
    )]
    let mawjud = std::env::var(MUTAGHAYYIR_MASAH);
    let Ok(jidhr) = mawjud else {
        eprintln!(
            "skipped: set {MUTAGHAYYIR_MASAH} to a directory of real mod loaders to run the \
             identifier over their actual bytes"
        );
        return;
    };
    let jidhr = PathBuf::from(jidhr);
    assert!(jidhr.is_dir(), "{MUTAGHAYYIR_MASAH} must name a directory: {}", jidhr.display());

    let mut wujida = 0_usize;
    for madkhal in fs::read_dir(&jidhr).expect("listing the directory") {
        let mujallad = madkhal.expect("a directory entry").path();
        if !mujallad.is_dir() {
            continue;
        }
        for qaim in wukala::masah(&mujallad).expect("the survey") {
            eprintln!("{}: {}", mujallad.display(), qaim.wasf_injilizi());
            wujida = wujida.saturating_add(1);
            assert!(
                !matches!(qaim.huwiya, wukala::HuwiyatWakeel::Multabis { .. }),
                "a real installation must not come back ambiguous: {qaim:?}"
            );
        }
    }
    assert!(wujida > 0, "the directory offered held no loader slots at all");
}
