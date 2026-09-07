//! The plan is binding: no write into a game's own text without one.
//!
//! `tabaqa_fawqiya.rs` measures what the *deployment* does at tier 3. This file
//! measures the larger of the two writes that used to escape the plan entirely:
//! the script-engine write, which replaces the text a game shipped with.
//!
//! Four of Taarib's engines are patched as data rather than as code — RPG Maker
//! MV and MZ through `data/*.json`, Ren'Py through generated
//! `game/tl/arabic/*.rpy`, GameMaker through the string pool of `data.win`,
//! Electron through a payload inside `app.asar`. That write used to run from
//! `masar_tathbeet::thabbit`, *before* the deployment step and
//! **unconditionally**, re-deriving the engine from the game's own directory.
//! A directory cannot be asked what tier a game is at, and it cannot be asked
//! whether the safety layer refused the game — so neither answer reached it,
//! and both of these happened:
//!
//! 1. a **tier-3** game — whose product surface says «the game is not modified
//!    at all», and whose plan says `SababLaHaja::TabaqaFawqiya` — had its own
//!    shipped dialogue replaced;
//! 2. a game whose capability report carries `marfuda`, meaning the safety
//!    layer refuses it outright, had the same done to it. `khutta` refuses that
//!    report by name; the text write never saw it.
//!
//! Both are measured below over the **whole tree**, byte for byte, because the
//! claim is about the game and not about a return value. The third test is the
//! control that keeps the first two honest: the identical game, the identical
//! package and the identical store at tier 1 *is* patched, so a fix that simply
//! stopped writing would fail here.
//!
//! **The Ren'Py game is authored, not a real one**, and built from the markers
//! `taarib_muhawwil_nusus::tarkeeb::ayn_hadaf` actually reads — the locator
//! rejects it if it is wrong, which is what makes it a fixture rather than a
//! mock. Ren'Py is the engine under test because its write is pure text: every
//! byte the gate does or does not stop is readable in the assertion.

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
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::nusus::{IdhnNusus, Nashir};
use taarib_tathbeet::tarkib::{
    HalatIdadat, KhuttatTarkib, LubaMuhallala, NatijatTarkib, QararTabaqa,
};
use taarib_tathbeet::{nusus, tarkib};
use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

/// `Hello, traveller.`
const MARHABAN: &str = "مرحبًا أيها المسافر.";

/// `Start Game`
const IBDA: &str = "ابدأ اللعبة";

/// The one file the Ren'Py adapter generates that carries the dialogue, and
/// therefore the one file whose absence is the whole proof.
const HIWAR: &str = "game/tl/arabic/taarib_mustalahat.rpy";

/// Builds a genuine Ren'Py 8 game tree.
fn ibni_renpy(jidhr: &Path) {
    let iktub = |nisbi: &str, muhtawa: &str| {
        let masar = jidhr.join(nisbi);
        fs::create_dir_all(masar.parent().expect("a parent")).expect("a fixture directory");
        fs::write(&masar, muhtawa).expect("a fixture file");
    };
    iktub(
        "renpy/__init__.py",
        "version_tuple = (8, 3, 4, vc_version)\n",
    );
    iktub("renpy/text/hbfont.so", "\u{7f}ELF");
    iktub("lib/py3-linux-x86_64/libharfbuzz.so.0", "\u{7f}ELF");
    iktub("lib/py3-linux-x86_64/libfribidi.so.0", "\u{7f}ELF");
    iktub(
        "Riverside.sh",
        "#!/bin/sh\nexec ./lib/py3-linux-x86_64/Riverside \"$@\"\n",
    );
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

/// Builds the Ren'Py component of a component store, face included.
///
/// The store is deliberately complete. The tier has to be what stops the write,
/// not a store with nothing in it to write — a proof over an empty store would
/// only show that a missing file cannot be copied.
fn ibni_makhzan(jidhr: &Path) {
    let iktub = |nisbi: &str, muhtawa: &[u8]| {
        let masar = jidhr.join("mulhaq/renpy").join(nisbi);
        fs::create_dir_all(masar.parent().expect("a parent")).expect("a store directory");
        fs::write(&masar, muhtawa).expect("a store file");
    };
    iktub(
        "taarib_renpy/__init__.py",
        b"def rakkib(gamedir):\n    pass\n",
    );
    // The first four bytes of a TrueType file. The installer never parses a
    // face — it copies bytes and names a path — but a fixture that is not even
    // framed like a font would be a fixture pretending to be one.
    iktub(
        "taarib/khutut/NotoNaskhArabic[wght].ttf",
        b"\x00\x01\x00\x00",
    );
    iktub(
        "taarib/khutut/OFL.txt",
        b"Copyright (c) The Noto Project Authors\n",
    );
}

/// Writes a real `.ruqaa` holding the two translations and opens it.
///
/// Built through the container's own writer, so the string table is keyed the
/// way every consumer of the format keys it: the first eight bytes of BLAKE3
/// over the source text, with the source text itself never stored.
fn ibni_ruqaa(masar: &Path) -> MalafRuqaa {
    let mut katib = Katib::jadeed();
    let _ = katib.bayan(br#"{"isdar":1}"#);
    let _ = katib
        .nass("Hello, traveller.", MARHABAN)
        .expect("a string record");
    let _ = katib.nass("Start Game", IBDA).expect("a string record");
    let bayt = katib.ikhtim().expect("a sealed package");
    fs::write(masar, bayt.bayt()).expect("writing the package");
    MalafRuqaa::iftah(masar).expect("the package opens and validates")
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

const fn muharrik_renpy() -> Muharrik {
    Muharrik {
        aila: AilatMuharrik::Renpy,
        isdar: None,
        khalfiya: KhalfiyaBarmajiya::Python,
        itarat: Vec::new(),
        rusum: Vec::new(),
        mimariya: Mimariya::X8664,
        thiqa: 100,
        dalail: Vec::new(),
    }
}

/// The game as the installer resolved it.
fn luba_muhallala(jidhr: &Path) -> LubaMuhallala {
    LubaMuhallala {
        jidhr: jidhr.to_path_buf(),
        masar_tanfidhi: jidhr.join("Riverside.sh"),
        muharrik: muharrik_renpy(),
        beea: BeeatTawafuq::Asli,
        nizam: NizamTashghil::Linux,
        masdar: MasdarLuba::Steam(480),
    }
}

/// A capability report at a chosen tier and a chosen safety verdict.
///
/// Written out rather than produced by the probe: this crate does not depend on
/// `taarib-muharrik`, and the two fields under test — `tabaqa` and `marfuda` —
/// are stated plainly instead of arriving through a crate boundary that would
/// have to be opened for a test.
fn imkaniyat(tabaqa: Tabaqa, marfuda: bool) -> TaqreerImkaniyat {
    TaqreerImkaniyat {
        muharrik: muharrik_renpy(),
        tabaqa,
        sabab_arabi: "رن‌باي ٨ يشكّل العربية بنفسه".to_owned(),
        sabab_injilizi: "a Ren'Py 8 build shapes Arabic itself".to_owned(),
        jahiziya: JahiziyatTashghil::Naqisa,
        naqs: None,
        anzimat_qabila: Vec::new(),
        jawda: JawdaMutawaqqaa::Jayida,
        hudud: Vec::new(),
        marfuda,
        isdar_fahs: 0,
        waqt: "2026-09-06T00:00:00Z".to_owned(),
    }
}

/// Every entry under `jidhr`: files with their bytes, directories named as
/// themselves.
///
/// Directories are in the listing on purpose. A tier that writes nothing must
/// also *create* nothing, and a file-only comparison would call an install that
/// left an empty `game/tl/arabic/` behind a clean one.
fn shajara(jidhr: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    let mut jadwal = BTreeMap::new();
    for madkhal in walkdir::WalkDir::new(jidhr).sort_by_file_name() {
        let madkhal = madkhal.expect("walking the game directory");
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr) else {
            continue;
        };
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

/// One prepared game: its directory, its backups, its store and its package.
struct Masrah {
    _dalil: tempfile::TempDir,
    luba: PathBuf,
    nusakh: PathBuf,
    makhzan: PathBuf,
    ruqaa: MalafRuqaa,
}

impl Masrah {
    fn ibni() -> Self {
        let dalil = tempfile::tempdir().expect("a temporary directory");
        let luba = dalil.path().join("luba");
        let nusakh = dalil.path().join("nusakh");
        let makhzan = dalil.path().join("mukawwinat");
        fs::create_dir_all(&luba).expect("the game directory");
        fs::create_dir_all(&nusakh).expect("the backup directory");
        ibni_renpy(&luba);
        ibni_makhzan(&makhzan);
        let ruqaa = ibni_ruqaa(&dalil.path().join("riverside.ruqaa"));
        Self {
            _dalil: dalil,
            luba,
            nusakh,
            makhzan,
            ruqaa,
        }
    }

    fn sijill(&self) -> Tathbeet {
        Tathbeet::ibda(&self.nusakh, NawTathbeet::Nass, &tarif(&self.luba), "dawra")
            .expect("an installation session")
    }

    /// The plan for this game at one tier and one safety verdict.
    ///
    /// Built separately from the write because `tarkib::nashr` — the one entry
    /// point that planned and executed together — is gone. The gate under test
    /// is therefore reached exactly the way the product reaches it: a plan
    /// first, and a write that is handed that plan and nothing else.
    fn khutta(&self, tabaqa: Tabaqa, marfuda: bool) -> Result<KhuttatTarkib, KhataTathbeet> {
        tarkib::khutta(
            &imkaniyat(tabaqa, marfuda),
            &luba_muhallala(&self.luba),
            &self.makhzan,
        )
    }
}

// ---------------------------------------------------------------------------
// 1. Tier 3 does not rewrite the game's own text
// ---------------------------------------------------------------------------

#[test]
fn tabaqa_fawqiya_la_tuaid_kitabat_nusus_alluba() {
    let masrah = Masrah::ibni();
    let qabl = shajara(&masrah.luba);

    let mukhattat = masrah
        .khutta(Tabaqa::TarjamaFawqiya, false)
        .expect("tier 3 is a tier, not a refusal, so it plans");

    let mut tathbeet = masrah.sijill();
    {
        let mut nashir = Nashir::jadeed(&mut tathbeet, &masrah.ruqaa, &masrah.luba);
        let (itar, mulhaqat) = tarkib::nashr_bi_khutta(
            &mukhattat,
            &luba_muhallala(&masrah.luba),
            &HalatIdadat::default(),
            &masrah.makhzan,
            &mut nashir,
        )
        .expect("tier 3 has nothing to deploy and therefore nothing to fail at");
        assert!(
            mulhaqat.mudafa.is_empty(),
            "no file was added: {:?}",
            mulhaqat.mudafa
        );
        assert!(
            mulhaqat.muaddala.is_empty(),
            "no file was modified: {:?}",
            mulhaqat.muaddala
        );
        assert!(
            matches!(itar, NatijatTarkib::LaHaja(_)),
            "and no framework was deployed: {itar:?}"
        );
        assert!(
            nashir.nusus().is_none(),
            "the script-engine write did not run: this is the assertion the defect failed"
        );
    }

    // The claim itself, measured rather than reported.
    let baad = shajara(&masrah.luba);
    assert_eq!(
        asmaa(&baad),
        asmaa(&qabl),
        "tier 3 states that the game is not modified at all, so not one entry may appear"
    );
    assert!(
        baad == qabl,
        "and every file that was there still holds exactly its own bytes"
    );
    assert!(
        !masrah.luba.join(HIWAR).exists(),
        "the generated dialogue file is the shape the defect took: five files into a game the \
         report had just said would not be touched"
    );
    assert_eq!(
        fs::read_to_string(masrah.luba.join("game/script.rpy"))
            .expect("the game's own script")
            .matches("Hello, traveller.")
            .count(),
        1,
        "and the game's own script is untouched English"
    );

    // Nothing reached the manifest either, which is the same claim from the
    // other side: a manifest with records is a game with Taarib's bytes in it.
    assert_eq!(
        tathbeet.bayan().sijillat().count(),
        0,
        "a tier-3 install records no change because it makes none"
    );
}

/// The same gate at its own level, without the deployment step around it.
///
/// Worth stating separately: [`nusus::raqqi_nusus`] is a public entry point, and
/// the tier is checked inside it rather than only by the caller that happens to
/// hold the plan.
#[test]
fn raqq_alnusus_yarfud_altabaqa_alfawqiya_bi_nafsih() {
    let masrah = Masrah::ibni();
    let qabl = shajara(&masrah.luba);

    let mut tathbeet = masrah.sijill();
    let idhn = IdhnNusus::min_qarar(
        QararTabaqa::min_taqreer(&imkaniyat(Tabaqa::TarjamaFawqiya, false))
            .expect("tier 3 is a tier, not a refusal"),
    );
    let taqreer = nusus::raqqi_nusus(
        idhn,
        &masrah.luba,
        &masrah.ruqaa,
        Some(&masrah.makhzan),
        &mut tathbeet,
    )
    .expect("a permit that forbids the write is not a failure of the write");

    assert!(taqreer.is_none(), "nothing was patched: {taqreer:?}");
    assert_eq!(shajara(&masrah.luba), qabl, "and nothing on disk moved");
}

// ---------------------------------------------------------------------------
// 2. A game the safety layer refused is not written to at all
// ---------------------------------------------------------------------------

#[test]
fn alluba_almarfuda_amanan_la_yulmas_minha_bayt() {
    let masrah = Masrah::ibni();
    let qabl = shajara(&masrah.luba);

    let mut tathbeet = masrah.sijill();
    {
        let nashir = Nashir::jadeed(&mut tathbeet, &masrah.ruqaa, &masrah.luba);
        // Tier 1 — the tier that patches the most — with the safety layer's
        // refusal beside it. The refusal has to be what stops this, or the gate
        // is only a tier gate wearing a second name.
        //
        // There is no longer any way to attempt the write: the value
        // `nashr_bi_khutta` demands is the plan, and a refused report yields no
        // plan. That is stronger than the refusal this test used to measure,
        // where planning and writing were one call and the refusal happened
        // inside it.
        let khata = masrah
            .khutta(Tabaqa::Kamil, true)
            .expect_err("a report the safety layer refused cannot be planned against");
        assert!(
            matches!(khata, KhataTathbeet::IdhnGhayrMutabiq),
            "and it refuses as an authorisation failure, by name: {khata:?}"
        );
        assert!(
            nashir.nusus().is_none(),
            "with the script-engine write never attempted"
        );
    }

    let baad = shajara(&masrah.luba);
    assert_eq!(
        asmaa(&baad),
        asmaa(&qabl),
        "a refusal that had already rewritten the game's dialogue would not be a refusal"
    );
    assert!(
        baad == qabl,
        "byte for byte, including the game's own script"
    );
    assert_eq!(
        tathbeet.bayan().sijillat().count(),
        0,
        "and the manifest records nothing"
    );
}

/// The refusal at the decision's own level: a refused report mints no permit.
///
/// This is the structural half of the claim. There is no way to reach the write
/// with a refused report, because there is no way to *build* the value the write
/// demands from one.
#[test]
fn altaqreer_almarfud_la_yuntij_qararan() {
    let khata = QararTabaqa::min_taqreer(&imkaniyat(Tabaqa::Kamil, true))
        .expect_err("a refused report yields no decision");
    assert!(
        matches!(khata, KhataTathbeet::IdhnGhayrMutabiq),
        "{khata:?}"
    );
    for tabaqa in [Tabaqa::Kamil, Tabaqa::RasmMubashir, Tabaqa::TarjamaFawqiya] {
        assert!(
            QararTabaqa::min_taqreer(&imkaniyat(tabaqa, false)).is_ok(),
            "and every tier the safety layer allows does yield one: {tabaqa:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. The control: the same game at tier 1 *is* patched
// ---------------------------------------------------------------------------

#[test]
fn nafs_alluba_bi_tabaqa_kamila_tunqal_nususuha() {
    let masrah = Masrah::ibni();
    // The identical game, package and store. Only the tier differs.
    let mukhattat = masrah
        .khutta(Tabaqa::Kamil, false)
        .expect("a deployment plan");
    let mut tathbeet = masrah.sijill();
    {
        let mut nashir = Nashir::jadeed(&mut tathbeet, &masrah.ruqaa, &masrah.luba);
        let _ = tarkib::nashr_bi_khutta(
            &mukhattat,
            &luba_muhallala(&masrah.luba),
            &HalatIdadat::default(),
            &masrah.makhzan,
            &mut nashir,
        )
        .expect("a Ren'Py game at tier 1 is patched");
        let taqreer = nashir
            .nusus()
            .expect("an adapter that applies to this directory");
        assert_eq!(taqreer.aila.ism(), "Ren'Py");
        assert_eq!(
            taqreer.nusus, 2,
            "both strings in the package were placed: {taqreer:?}"
        );
    }

    let hiwar =
        fs::read_to_string(masrah.luba.join(HIWAR)).expect("the generated interface strings");
    assert!(
        hiwar.contains(&format!("new \"{MARHABAN}\"")),
        "the Arabic came out of the package and into the game:\n{hiwar}"
    );
    assert!(hiwar.contains(&format!("new \"{IBDA}\"")));

    // The face too, and by the name the plan chose rather than by a second
    // reading of the store: the settings and the deployed file are one file.
    let idad = fs::read_to_string(masrah.luba.join("game/tl/arabic/taarib_idad.rpy"))
        .expect("the generated settings");
    let khatt = "taarib/khutut/NotoNaskhArabic[wght].ttf";
    assert!(
        idad.contains(&format!("_taarib_khatt = \"{khatt}\"")),
        "the face is registered under the plan's name:\n{idad}"
    );
    assert!(
        masrah.luba.join("game").join(khatt).is_file(),
        "and the deployment placed exactly that file, in the same call"
    );

    // Every byte of it went through the manifest, so an uninstall is a delete
    // rather than a search.
    let bayan = tathbeet.bayan();
    assert!(
        bayan
            .sijillat()
            .any(|sijill| sijill.masar.ends_with("taarib_mustalahat.rpy"))
    );
    assert!(bayan.sijillat().all(|sijill| !sijill.masar.contains("..")));
}
