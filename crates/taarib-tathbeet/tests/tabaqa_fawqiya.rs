//! Tier 3 promises the game is never touched. This is where that is measured.
//!
//! The overlay tier's product surface states it without a hedge — *«the game is
//! not modified at all»* — and until this file existed nothing checked it. The
//! tier's own reason code says the same thing in the source:
//! `SababLaHaja::TabaqaFawqiya` is documented as "the game is not modified at
//! all … and this is where that stays true".
//!
//! It did not stay true. [`khutta`] answers the tier correctly and returns
//! before any per-engine arm, but [`nashr`] then called the framework writer,
//! which re-derived the framework from the **engine alone** — and an
//! unrecognised engine is exactly the engine that gets tier 3. So a tier-3
//! install deployed Taarib's loader and its payloads into a game the plan
//! beside it said needed no framework whatever. Measured on a real game: five
//! files and one directory written into a title whose report had just told the
//! player nothing would be.
//!
//! It was not the only write that escaped the plan. The larger one — the
//! script-engine write, which replaces a game's own shipped text — ran from
//! `masar_tathbeet::thabbit` unconditionally, outside any plan at all;
//! `tests/khutta_mulzima.rs` is the standing guard for that one, and the first
//! test below now also asserts it did not run here.
//!
//! The three tests below are the standing guard:
//!
//! 1. tier 3 writes **nothing** — asserted over the whole tree, files and
//!    directories, not over the return value;
//! 2. the same engine at tier 1 still deploys, so the gate is on the tier and
//!    not on the engine — a fix that simply stopped deploying for `Majhul`
//!    would pass test 1 and break every unrecognised game that *is* patchable;
//! 3. what an uninstall does **not** take back: a file the deployed framework
//!    writes itself, at run time, into a directory Taarib created. That is not
//!    a tier-3 case, and it is the other half of "no game is damaged".
//!
//! The trees here are authored. The shapes are not: they are the ones a real
//! library produced, and the third test's leftovers are the ones a real
//! `BepInEx` install left in a real game after a real uninstall.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::{
    AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik, Tabaqa,
    TaqreerImkaniyat,
};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::katib::Katib;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::nusus::Nashir;
use taarib_tathbeet::tarkib::{
    HalatIdadat, LubaMuhallala, NatijatTarkib, SababLaHaja, nashr,
};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_nass, nazzif_nusakh};
use taarib_usus::khata::Tafsir;
use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

/// The largest fixture body written. The proof is about names and bytes, not
/// volume.
const AQSA_JISM: usize = 512;

/// Writes a file with deterministic, position-dependent content.
///
/// Not zeroes: these tests compare a tree against itself across an operation,
/// and a directory of identical zero-filled files would compare equal even if
/// the operation had swapped two of them.
fn iktub(masar: &Path, hajm: usize) {
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).expect("a fixture directory");
    }
    let tul = hajm.min(AQSA_JISM);
    let ism = masar.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let badhra = ism.bytes().fold(17_u8, |akk, bayt| akk.wrapping_mul(31).wrapping_add(bayt));
    let jism: Vec<u8> =
        (0..tul).map(|mawdi| badhra.wrapping_add(u8::try_from(mawdi % 251).unwrap_or(0))).collect();
    fs::write(masar, jism).expect("a fixture file");
}

/// A game directory shaped like the ones that come back unrecognised: a native
/// executable, its runtime, and a data tree with no engine marker anywhere.
fn ibni_luba(jidhr: &Path) {
    iktub(&jidhr.join("DarkSoulsRemastered.exe"), 9_139_840);
    iktub(&jidhr.join("steam_api64.dll"), 316_568);
    iktub(&jidhr.join("data1.bdt"), 4096);
    iktub(&jidhr.join("data1.bhd"), 4096);
    iktub(&jidhr.join("sound/fdp_main.fsb"), 4096);
    // An empty directory the game shipped. A restore that consumed it would be
    // as wrong as one that left a directory behind, and only a listing that
    // carries directories can tell.
    fs::create_dir_all(jidhr.join("Capture")).expect("the game's own empty directory");
}

/// The component store entry `MukawwinItar::mudkhal` names for a 64-bit Windows
/// game: the loader under `muhammil/`, one payload beside it, one file outside.
fn ibni_makhzan(jidhr: &Path) {
    let mukawwin = jidhr.join("mudkhal/windows/x64");
    iktub(&mukawwin.join("muhammil/version.dll"), 96_000);
    iktub(&mukawwin.join("muhammil/taarib_tabaqa.dll"), 480_000);
    iktub(&mukawwin.join("iqra.txt"), 128);
}

/// Writes a real `.ruqaa` carrying two translations and opens it.
///
/// The deployment step is handed a package now, because the script-engine write
/// happens inside it. This game is on no script engine, so nothing here should
/// place a single string — and a fixture with an empty package would prove that
/// by having nothing to place, which is not the same claim.
fn ibni_ruqaa(masar: &Path) -> MalafRuqaa {
    let mut katib = Katib::jadeed();
    let _ = katib.bayan(br#"{"isdar":1}"#);
    let _ = katib.nass("Bonfire", "شعلة").expect("a string record");
    let _ = katib.nass("Estus Flask", "قارورة الإستوس").expect("a string record");
    let bayt = katib.ikhtim().expect("a sealed package");
    fs::write(masar, bayt.bayt()).expect("writing the package");
    MalafRuqaa::iftah(masar).expect("the package opens and validates")
}

fn tarif(jidhr: &Path) -> TarifLuba {
    let masdar = MasdarLuba::Steam(570_940);
    TarifLuba {
        luba: LubaId::min_masdar(&masdar, "DARK SOULS REMASTERED"),
        masdar,
        ism: "DARK SOULS REMASTERED".to_owned(),
        jidhr: jidhr.to_path_buf(),
        ruqaa: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::jadeeda(1),
        basma_bina: None,
    }
}

/// The game as the installer resolved it: engine unrecognised, 64-bit, Windows.
fn luba_muhallala(jidhr: &Path) -> LubaMuhallala {
    LubaMuhallala {
        jidhr: jidhr.to_path_buf(),
        masar_tanfidhi: jidhr.join("DarkSoulsRemastered.exe"),
        muharrik: muharrik_majhul(),
        beea: BeeatTawafuq::Asli,
        nizam: NizamTashghil::Windows,
        masdar: MasdarLuba::Steam(570_940),
    }
}

const fn muharrik_majhul() -> Muharrik {
    Muharrik {
        aila: AilatMuharrik::Majhul,
        isdar: None,
        khalfiya: KhalfiyaBarmajiya::Majhula,
        itarat: Vec::new(),
        rusum: Vec::new(),
        mimariya: Mimariya::X8664,
        thiqa: 49,
        dalail: Vec::new(),
    }
}

/// A capability report at a chosen tier, as `khutta` reads one.
///
/// Written out rather than produced by the probe, exactly as `raqq_nusus` does
/// it: this crate does not depend on `taarib-muharrik`, and the three fields
/// the plan consults — `marfuda`, `tabaqa` and the engine family — are stated
/// plainly rather than arriving through a crate boundary opened for a test.
/// The values are the ones a real probe returned for a real unrecognised game.
fn imkaniyat(tabaqa: Tabaqa) -> TaqreerImkaniyat {
    TaqreerImkaniyat {
        muharrik: muharrik_majhul(),
        tabaqa,
        sabab_arabi: "لم يتعرّف تعريب على محرّك هذه اللعبة".to_owned(),
        sabab_injilizi: "Taarib did not recognise this game's engine".to_owned(),
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

/// Every entry under `jidhr`: files with their bytes, directories named as
/// themselves.
///
/// Directories are in the listing on purpose. A tier that deploys nothing must
/// also *create* nothing, and a file-only comparison would call an install that
/// left an empty `taarib/` behind a clean one — which is the exact defect a
/// restore was found leaving in a real game.
fn shajara(jidhr: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut jadwal = BTreeMap::new();
    for madkhal in walkdir::WalkDir::new(jidhr).sort_by_file_name() {
        let madkhal = madkhal.expect("walking the game directory");
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr) else { continue };
        if nisbi.as_os_str().is_empty() {
            continue;
        }
        let ism = nisbi.to_string_lossy().replace('\\', "/");
        let qeema = if madkhal.file_type().is_dir() {
            None
        } else {
            Some(fs::read(madkhal.path()).expect("reading a game file"))
        };
        let _ = jadwal.insert(ism, qeema);
    }
    jadwal
}

/// The names in a listing, for a message that says which entry appeared.
fn asmaa(jadwal: &BTreeMap<String, Option<Vec<u8>>>) -> Vec<&str> {
    jadwal.keys().map(String::as_str).collect()
}

// ---------------------------------------------------------------------------
// 1. Tier 3 writes nothing
// ---------------------------------------------------------------------------

#[test]
fn tabaqa_fawqiya_la_taktub_ayya_bayt_fi_alluba() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("luba");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_luba(&luba);
    // The store holds the component this engine would otherwise be given. That
    // is the point: the tier has to be what stops the deployment, not an empty
    // store, or the test proves only that a missing file cannot be copied.
    ibni_makhzan(&makhzan);

    let ruqaa = ibni_ruqaa(&dalil.path().join("dark-souls.ruqaa"));

    let qabl = shajara(&luba);
    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let mut nashir = Nashir::jadeed(&mut tathbeet, &ruqaa, &luba);
    let (itar, mulhaqat) = nashr(
        &luba_muhallala(&luba),
        &HalatIdadat::default(),
        &imkaniyat(Tabaqa::TarjamaFawqiya),
        &makhzan,
        &mut nashir,
    )
    .expect("tier 3 has nothing to deploy and therefore nothing to fail at");
    assert!(
        nashir.nusus().is_none(),
        "and the script-engine write did not run either: at tier 3 the game's own text is \
         never replaced, whatever engine it turns out to be on"
    );

    assert_eq!(
        itar,
        NatijatTarkib::LaHaja(SababLaHaja::TabaqaFawqiya),
        "the framework step must decline, and decline *for the tier* — a decline for any \
         other reason would pass this line while leaving the engines that share the reason \
         unprotected"
    );
    assert!(mulhaqat.mudafa.is_empty(), "no file was added: {:?}", mulhaqat.mudafa);
    assert!(mulhaqat.muaddala.is_empty(), "no file was modified: {:?}", mulhaqat.muaddala);
    assert!(
        mulhaqat.mujalladat.is_empty(),
        "and no directory was created: {:?}",
        mulhaqat.mujalladat
    );

    // The claim itself, measured rather than reported: the tree is what it was.
    let baad = shajara(&luba);
    assert_eq!(
        asmaa(&baad),
        asmaa(&qabl),
        "tier 3 states that the game is not modified at all, so not one entry may appear or \
         disappear"
    );
    assert!(baad == qabl, "and every file that was there still holds exactly its own bytes");
    assert!(
        !luba.join("version.dll").exists(),
        "the loader an unrecognised engine would get at tier 1 must not be here"
    );
    assert!(
        !luba.join("taarib").exists(),
        "and neither must the directory its payloads would land in"
    );

    // Nothing reached the manifest either, which is the same claim from the
    // other side: a manifest with records is a game with Taarib's bytes in it.
    assert_eq!(
        tathbeet.bayan().sijillat().count(),
        0,
        "a tier-3 install records no change because it makes none"
    );
}

// ---------------------------------------------------------------------------
// 2. The gate is the tier, not the engine
// ---------------------------------------------------------------------------

#[test]
fn nafs_almuharrik_bi_tabaqa_kamila_yansur_almuhammil() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("luba");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_luba(&luba);
    ibni_makhzan(&makhzan);

    let ruqaa = ibni_ruqaa(&dalil.path().join("dark-souls.ruqaa"));
    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let mut nashir = Nashir::jadeed(&mut tathbeet, &ruqaa, &luba);
    let (itar, _) = nashr(
        &luba_muhallala(&luba),
        &HalatIdadat::default(),
        // The identical engine, identical store, identical directory. Only the
        // tier differs, and it is the only thing that may decide this.
        &imkaniyat(Tabaqa::Kamil),
        &makhzan,
        &mut nashir,
    )
    .expect("an unrecognised engine at tier 1 gets Taarib's own loader");

    let NatijatTarkib::Nushira(munaffadh) = itar else {
        panic!("tier 1 deploys; a fix that gated on the engine would land here: {itar:?}");
    };
    assert_eq!(munaffadh.mukawwin, "mudkhal/windows/x64");
    assert!(
        luba.join("version.dll").is_file(),
        "the loader landed beside the executable, exactly as before the tier gate existed"
    );
    assert!(luba.join("taarib/iqra.txt").is_file(), "and the payload landed under taarib/");
}

// ---------------------------------------------------------------------------
// 3. What an uninstall cannot take back
// ---------------------------------------------------------------------------

/// A framework's own runtime output, inside a directory Taarib created, is not
/// removed by a default uninstall — and the directory is kept because of it.
///
/// This is not a hypothetical. Installing into two real Unity games, playing
/// each once through Steam and then uninstalling left `BepInEx/LogOutput.log`,
/// `BepInEx/config/BepInEx.cfg` and the `BepInEx/` tree standing in games that
/// had none before — files no manifest could name, because they did not exist
/// when the manifest was written. The recorder only removes what it recorded,
/// which is right; what was missing is anywhere that said so.
///
/// It is still the current behaviour, and deliberately so: nothing in the bytes
/// separates a framework's log from a player's own mod, and the two share the
/// directory. What changed is that the leftovers are now **named** and the
/// manifest line is **not** marked done, so `nazzif_nusakh` will not consume the
/// only record that ties them to this installation. `tests/baqaya_izala.rs`
/// carries that rule in full, including the opt-in sweep that does return the
/// game to byte-identity; the last two assertions here are the corner of it that
/// belongs beside the tier-3 proof.
#[test]
fn istiada_tatruk_ma_katabahu_alitar_bi_nafsihi() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba = dalil.path().join("luba");
    let nusakh = dalil.path().join("nusakh");
    let makhzan = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_luba(&luba);
    ibni_makhzan(&makhzan);

    let ruqaa = ibni_ruqaa(&dalil.path().join("dark-souls.ruqaa"));
    let qabl = shajara(&luba);
    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");
    let (itar, _) = nashr(
        &luba_muhallala(&luba),
        &HalatIdadat::default(),
        &imkaniyat(Tabaqa::Kamil),
        &makhzan,
        &mut Nashir::jadeed(&mut tathbeet, &ruqaa, &luba),
    )
    .expect("the framework deploys");
    assert!(matches!(itar, NatijatTarkib::Nushira(_)));
    drop(tathbeet);

    // The game runs once. The framework Taarib put there writes its own log and
    // its own configuration, into the directory Taarib created, after the
    // manifest was sealed. Nothing about this goes through the recorder,
    // because the framework is not asking the recorder anything.
    iktub(&luba.join("taarib/sijill_tashghil.log"), 748);
    iktub(&luba.join("taarib/idadat/itar.cfg"), 5564);

    let mut radd = RadLaShay;
    let taqreer = istiada_nass(&luba, &nusakh, SiyasatIstiada::Muhafiza, &mut radd)
        .expect("the uninstall completes and reports success");

    assert!(
        taqreer.mujalladat_matruka > 0,
        "the restore leaves the directory standing and counts it: {taqreer:?}"
    );
    assert!(
        taqreer.taqreer().iter().any(|satr| satr.contains("Taarib did not put there")),
        "and says so in its own report: {:?}",
        taqreer.taqreer()
    );

    let baad = shajara(&luba);
    assert_ne!(
        baad, qabl,
        "this is the finding, not a passing case: the game is NOT byte-identical to what it \
         was before the install"
    );
    let zaida: Vec<&str> = asmaa(&baad).into_iter().filter(|ism| !qabl.contains_key(*ism)).collect();
    assert_eq!(
        zaida,
        vec!["taarib", "taarib/idadat", "taarib/idadat/itar.cfg", "taarib/sijill_tashghil.log"],
        "exactly the framework's own runtime output and the directories holding it survive; \
         nothing the manifest recorded does"
    );
    assert!(
        !luba.join("version.dll").exists(),
        "everything the manifest did record was taken back out, which is the half that works"
    );

    // The half that used to fail silently. The leftovers are named rather than
    // counted, and the record of them is refused to `nazzif_nusakh` — because a
    // manifest deleted here is the moment these files stop being attributable to
    // anything and become litter no later run can identify.
    assert_eq!(
        taqreer.baqaya.iter().map(|baqiya| baqiya.mujallad.as_str()).collect::<Vec<_>>(),
        vec!["taarib"],
        "the directory that would not go is named: {:?}",
        taqreer.baqaya
    );
    let khata = nazzif_nusakh(&luba, &nusakh, NawTathbeet::Nass)
        .expect_err("and the record must not be discarded while the residue exists");
    assert!(
        Tafsir::injilizi(&khata).contains("attributable"),
        "for the stated reason: {}",
        Tafsir::injilizi(&khata)
    );
}
