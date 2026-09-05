//! The dispatch point, end to end: a real `.ruqaa` in, Arabic in the game out.
//!
//! The round-trip proofs for each engine's *format* live in
//! `taarib-muhawwil-nusus`. What is proved here is the join this crate owns and
//! that nothing in the product had before: a package's string table, keyed on a
//! hash of the source text and carrying no source text at all, resolving against
//! the strings a real game directory actually holds, and the result reaching
//! disk through the installation recorder rather than through a file handle.
//!
//! Ren'Py is the engine under test because its write is pure text, so the first
//! test exercises the whole chain — `MalafRuqaa` → `MutarjimRuqaa` →
//! `rakkib_luba` → `HafizMuthabbit` → `Tathbeet` → the game — with no build
//! artifact standing in for anything.
//!
//! The second test adds the one thing Ren'Py *does* take from the component
//! store: the Arabic face. It is the harder proof, because the ordering is the
//! difficulty. `raqqi_nusus` runs before any deployment — RPG Maker's byte
//! offsets require it — so the settings file names a font that is not on disk
//! yet, and only the deployment a step later puts it there. The test runs the
//! two in that same order and then asks the one question that matters: does the
//! path the generated `.rpy` names resolve to a file?
//!
//! **The Ren'Py game here is authored, not a real one.** No Ren'Py game is
//! installed on the machine this was written on. The tree below is built from
//! the markers `tarkeeb::ayn_hadaf` and the shaping probe actually read, which
//! is a weaker proof than a shipped game and a much stronger one than a mock:
//! the locator rejects it if it is wrong. The component store is likewise this
//! test's own, holding one face and the license that travels with it.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]

use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::{
    AilatMuharrik, JahiziyatTashghil, JawdaMutawaqqaa, KhalfiyaBarmajiya, Muharrik, Tabaqa,
    TaqreerImkaniyat,
};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::katib::Katib;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::{nusus, tarkib};
use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

/// `Hello, traveller.`
const MARHABAN: &str = "مرحبًا أيها المسافر.";

/// `Start Game`
const IBDA: &str = "ابدأ اللعبة";

/// Builds a genuine Ren'Py 8 game tree.
fn ibni_renpy(jidhr: &Path) {
    let iktub = |nisbi: &str, muhtawa: &str| {
        let masar = jidhr.join(nisbi);
        fs::create_dir_all(masar.parent().expect("a parent")).expect("a fixture directory");
        fs::write(&masar, muhtawa).expect("a fixture file");
    };
    iktub("renpy/__init__.py", "version_tuple = (8, 3, 4, vc_version)\n");
    iktub("renpy/text/hbfont.so", "\u{7f}ELF");
    iktub("lib/py3-linux-x86_64/libharfbuzz.so.0", "\u{7f}ELF");
    iktub("lib/py3-linux-x86_64/libfribidi.so.0", "\u{7f}ELF");
    iktub(
        "game/script.rpy",
        concat!(
            "label start:\n",
            "\n",
            "    \"Hello, traveller.\"\n",
            "\n",
            "    menu:\n",
            "        \"Start Game\":\n",
            "            jump chapter_one\n",
            "        \"Quit\":\n",
            "            return\n",
        ),
    );
}

/// Writes a real `.ruqaa` holding two translations and returns its path.
///
/// Built through the container's own writer, so the string table is keyed the
/// way every consumer of the format keys it: the first eight bytes of BLAKE3
/// over the source text, with the source text itself never stored.
fn ibni_ruqaa(masar: &Path) {
    let mut katib = Katib::jadeed();
    let _ = katib.bayan(br#"{"isdar":1}"#);
    let _ = katib.nass("Hello, traveller.", MARHABAN).expect("a string record");
    let _ = katib.nass("Start Game", IBDA).expect("a string record");
    let bayt = katib.ikhtim().expect("a sealed package");
    fs::write(masar, bayt.bayt()).expect("writing the package");
}

fn tarif(jidhr: &Path) -> TarifLuba {
    let masdar = MasdarLuba::Steam(480);
    TarifLuba {
        luba: LubaId::min_masdar(&masdar, "Riverside"),
        masdar,
        ism: "Riverside".to_owned(),
        jidhr: jidhr.to_path_buf(),
        ruqaa: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::jadeeda(1),
        basma_bina: None,
    }
}

#[test]
fn raqq_nusus_yasil_min_alruqaa_ila_almalaf() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba: PathBuf = dalil.path().join("luba");
    let nusakh: PathBuf = dalil.path().join("nusakh");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_renpy(&luba);

    let masar_ruqaa = dalil.path().join("riverside.ruqaa");
    ibni_ruqaa(&masar_ruqaa);
    let ruqaa = MalafRuqaa::iftah(&masar_ruqaa).expect("the package opens and validates");

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");

    // The component store is deliberately absent: three of the four adapters
    // need nothing from it, and Ren'Py is one of them.
    let taqreer = nusus::raqqi_nusus(&luba, &ruqaa, None, &mut tathbeet)
        .expect("the script-engine write")
        .expect("an adapter that applies to this directory");

    assert_eq!(taqreer.aila.ism(), "Ren'Py");
    assert_eq!(taqreer.nusus, 2, "both strings in the package were placed: {taqreer:?}");

    let hiwar = fs::read_to_string(luba.join("game/tl/arabic/taarib_mustalahat.rpy"))
        .expect("the generated interface strings");
    assert!(
        hiwar.contains(&format!("new \"{MARHABAN}\"")),
        "the Arabic came out of the package's string table and into the game:\n{hiwar}"
    );
    assert!(hiwar.contains(&format!("new \"{IBDA}\"")));
    assert!(
        !hiwar.contains("old \"Quit\""),
        "and a string the package does not carry is left alone"
    );

    // Written through the recorder, so the manifest knows about every one of
    // them and an uninstall is a delete rather than a search.
    let bayan = tathbeet.bayan();
    assert!(
        bayan.sijillat().any(|sijill| sijill.masar.ends_with("taarib_mustalahat.rpy")),
        "every file this write produced is in the installation manifest"
    );
    assert!(
        bayan.sijillat().all(|sijill| !sijill.masar.contains("..")),
        "and every recorded path stays inside the game"
    );
}

// ---------------------------------------------------------------------------
// The font, which is the one thing Ren'Py takes from the component store
// ---------------------------------------------------------------------------

/// The face this test's component store ships, inside the Ren'Py component.
///
/// Bracketed on purpose: every Noto face the product bundles is a variable font
/// whose file name states its axis, and that name has to survive both the
/// store walk and the Ren'Py quoting intact.
const WAJH: &str = "NotoNaskhArabic[wght].ttf";

/// Where that face lands relative to `game/`, which is also what the generated
/// settings must name.
const KHATT_FI_ALLUBA: &str = "taarib/khutut/NotoNaskhArabic[wght].ttf";

/// Builds the Ren'Py component of a component store.
///
/// No `bayan_mukawwinat.json`: a store without one is a development build, an
/// arrangement `bayan_makhzan::kamil_hasab_bayan` accepts by name, and what is
/// under test here is the font wiring rather than the mirror's completeness
/// check.
fn ibni_makhzan(jidhr: &Path) {
    let iktub = |nisbi: &str, muhtawa: &[u8]| {
        let masar = jidhr.join("mulhaq/renpy").join(nisbi);
        fs::create_dir_all(masar.parent().expect("a parent")).expect("a store directory");
        fs::write(&masar, muhtawa).expect("a store file");
    };
    iktub("taarib_renpy/__init__.py", b"def rakkib(gamedir):\n    pass\n");
    // The first four bytes of a TrueType file. The installer never parses a
    // face — it copies bytes and names the path — but a fixture that is not
    // even framed like a font would be a fixture pretending to be one.
    iktub(&format!("taarib/khutut/{WAJH}"), b"\x00\x01\x00\x00");
    // The license travels with the face, and must not be mistaken for one.
    iktub("taarib/khutut/OFL.txt", b"Copyright (c) The Noto Project Authors\n");
}

/// A capability report for a Ren'Py game, as `khutta` reads one.
///
/// Written out rather than produced by the probe: this crate does not depend on
/// `taarib-muharrik`, and the three fields the plan actually consults —
/// `marfuda`, `tabaqa` and the engine family — are stated here plainly instead
/// of arriving through a crate boundary that would have to be opened for a test.
fn imkaniyat() -> TaqreerImkaniyat {
    TaqreerImkaniyat {
        muharrik: Muharrik {
            aila: AilatMuharrik::Renpy,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Python,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 100,
            dalail: Vec::new(),
        },
        tabaqa: Tabaqa::Kamil,
        sabab_arabi: "رن‌باي ٨ يشكّل العربية بنفسه".to_owned(),
        sabab_injilizi: "a Ren'Py 8 build shapes Arabic itself".to_owned(),
        jahiziya: JahiziyatTashghil::Naqisa,
        naqs: None,
        anzimat_qabila: Vec::new(),
        jawda: JawdaMutawaqqaa::Jayida,
        hudud: Vec::new(),
        marfuda: false,
        isdar_fahs: 0,
        waqt: "2026-09-05T00:00:00Z".to_owned(),
    }
}

#[test]
fn khatt_renpy_yusajjal_qabl_nashrih_wa_yujad_baadah() {
    let dalil = tempfile::tempdir().expect("a temporary directory");
    let luba: PathBuf = dalil.path().join("luba");
    let nusakh: PathBuf = dalil.path().join("nusakh");
    let makhzan: PathBuf = dalil.path().join("mukawwinat");
    fs::create_dir_all(&luba).expect("the game directory");
    fs::create_dir_all(&nusakh).expect("the backup directory");
    ibni_renpy(&luba);
    ibni_makhzan(&makhzan);

    let masar_ruqaa = dalil.path().join("riverside.ruqaa");
    ibni_ruqaa(&masar_ruqaa);
    let ruqaa = MalafRuqaa::iftah(&masar_ruqaa).expect("the package opens and validates");

    // What the deployment will place, asked before anything is written. This is
    // the value `raqqi_nusus` registers and the value the plan carries, and the
    // rest of this test is the claim that those are the same file.
    let khatt = tarkib::khatt_renpy(&makhzan)
        .expect("the store lists")
        .expect("the component ships a face");
    assert_eq!(khatt, KHATT_FI_ALLUBA, "the name is the path inside `game/`, verbatim");

    let mut tathbeet = Tathbeet::ibda(&nusakh, NawTathbeet::Nass, &tarif(&luba), "dawra")
        .expect("an installation session");

    // Step one, exactly where `masar_tathbeet::thabbit` runs it: the text
    // write, before any deployment.
    let taqreer = nusus::raqqi_nusus(&luba, &ruqaa, Some(&makhzan), &mut tathbeet)
        .expect("the script-engine write")
        .expect("an adapter that applies to this directory");
    assert_eq!(taqreer.aila.ism(), "Ren'Py");
    assert!(
        taqreer.mulahazat.iter().all(|satr| !satr.contains("no Arabic font was supplied")),
        "a store that ships a face must not be reported as one that does not: {:?}",
        taqreer.mulahazat
    );

    let idad = fs::read_to_string(luba.join("game/tl/arabic/taarib_idad.rpy"))
        .expect("the generated settings");
    assert!(
        idad.contains(&format!("_taarib_khatt = \"{KHATT_FI_ALLUBA}\"")),
        "the face is registered under the name the store gave:\n{idad}"
    );
    assert!(
        !idad.contains("[["),
        "a font path is a file name and not displayed text; a doubled bracket names no file"
    );
    // And at this moment it is genuinely not there. That is the whole ordering
    // problem, stated as an assertion rather than as a comment.
    assert!(
        !luba.join("game").join(&khatt).exists(),
        "nothing has been deployed yet, which is why the name had to come from the store"
    );

    // Step two: the deployment, through the same recorder.
    let mukhattat = tarkib::khutta(
        &imkaniyat(),
        &imkaniyat().muharrik,
        NizamTashghil::Linux,
        &BeeatTawafuq::Asli,
        &makhzan,
        &luba,
    )
    .expect("a deployment plan");
    assert_eq!(
        mukhattat.khatt_renpy.as_deref(),
        Some(khatt.as_str()),
        "the plan names the same face the settings were written against"
    );
    assert!(
        mukhattat.mudkhalat.iter().any(|mudkhal| mudkhal.nisbi == format!("game/{khatt}")),
        "and plans to add it: {:?}",
        mukhattat.mudkhalat.iter().map(|mudkhal| &mudkhal.nisbi).collect::<Vec<_>>()
    );
    let munashar = tarkib::nashr_mulhaqat(&mukhattat, &luba, &makhzan, &mut tathbeet)
        .expect("the additive layer deploys");
    assert!(munashar.mutakhatta.is_empty(), "nothing was skipped: {munashar:?}");

    // The question the whole change exists to answer.
    let mutlaq = luba.join("game").join(&khatt);
    assert!(
        mutlaq.is_file(),
        "the path the generated .rpy names must resolve to a file: {}",
        mutlaq.display()
    );
    assert_eq!(
        fs::read(&mutlaq).expect("the deployed face"),
        b"\x00\x01\x00\x00",
        "and to the store's bytes, not to something else that happens to be there"
    );

    // Deployed through the manifest like everything else, so an uninstall takes
    // the face back out again rather than leaving it in somebody's game.
    let bayan = tathbeet.bayan();
    assert!(
        bayan.sijillat().any(|sijill| sijill.masar.ends_with(WAJH)),
        "the face is in the installation manifest"
    );
}
