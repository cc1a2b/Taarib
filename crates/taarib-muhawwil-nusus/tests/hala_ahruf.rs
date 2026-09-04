//! Locating a game's own directories when the depot spelled them differently.
//!
//! A Windows game running under Wine or Proton sees a **case-insensitive**
//! filesystem. A depot that ships `Game/` where Ren'Py's convention is `game/`,
//! or `Resources/` where Electron's is `resources/`, runs perfectly for the
//! player — and is invisible to a literal `Path::join` on Linux. The failure
//! that produces is not an error message. It is
//! [`tarkeeb::ayn_hadaf`] answering [`None`], the dispatcher reading that as "no
//! adapter applies", and the install **reporting success having translated
//! nothing**.
//!
//! Steam depots are usually consistent. Bottles, Lutris and Heroic installs are
//! much less so, and an itch.io upload repacked on Windows is under no
//! obligation at all.
//!
//! ## What every test here checks, twice
//!
//! Each locator is checked in both directions, because half a fix is a different
//! bug:
//!
//! 1. **A case-differing name resolves.** The locator finds the game.
//! 2. **The exact name still wins when both exist.** Two entries differing only
//!    in case are legal on Linux and impossible on the Windows the game thinks
//!    it is running on. When a depot holds both, the one the engine itself would
//!    open is the exact spelling, so that is the one Taarib must patch —
//!    patching the other would be an install that succeeds against a directory
//!    the game never reads.
//!
//! The second half needs two entries differing only in case to actually exist,
//! which a case-insensitive filesystem cannot provide. [`bi_hassasiyat_hala`]
//! establishes at run time whether this one can, and the assertion is skipped
//! rather than faked where it cannot — a test that quietly passes on macOS
//! without testing anything is worse than one that says what it did not check.

#![allow(
    clippy::panic,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and honouring them here \
              would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_muhawwil_nusus::hifz::Hifz;
use taarib_muhawwil_nusus::tabaqa::Rutba;
use taarib_muhawwil_nusus::tarkeeb::{self, Mawarid};
use taarib_muhawwil_nusus::{electron, gamemaker, renpy, rpgmaker, vxace};

// ---------------------------------------------------------------------------
// Scaffolding
// ---------------------------------------------------------------------------

/// A real game directory, and the backup directory beside it.
struct Saha {
    _dalil: tempfile::TempDir,
    luba: PathBuf,
    nusakh: PathBuf,
}

impl Saha {
    fn jadida() -> Self {
        let dalil = tempfile::tempdir().expect("a temporary directory");
        let luba = dalil.path().join("luba");
        let nusakh = dalil.path().join("nusakh");
        fs::create_dir_all(&luba).expect("the game directory");
        fs::create_dir_all(&nusakh).expect("the backup directory");
        Self { _dalil: dalil, luba, nusakh }
    }

    /// The real preservation session every patcher in this crate writes through.
    fn hifz(&self) -> Hifz {
        Hifz::ibda(&self.luba, &self.nusakh, "hala").expect("a preservation session")
    }

    fn iktub(&self, nisbi: &str, muhtawa: &[u8]) {
        let masar = self.luba.join(nisbi);
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid).expect("a fixture directory");
        }
        fs::write(&masar, muhtawa).expect("a fixture file");
    }

    fn jidhr(&self) -> &Path {
        &self.luba
    }
}

/// Whether this filesystem can hold two entries differing only in case.
///
/// Asked rather than assumed. The suite runs on developer machines and on CI,
/// and a macOS checkout or a Windows one cannot create the fixture the
/// exact-name-wins half of each test needs.
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

/// The Arabic the dialogue fixtures translate into.
const MARHABAN: &str = "مرحبًا أيها المسافر.";

/// A `.pck`-free Electron archive is not needed here: every locator under test
/// asks only whether a path is a file, so a placeholder is the honest fixture —
/// it is exactly what the locator inspects, and nothing more.
const BADEEL: &[u8] = b"taarib fixture";

// ---------------------------------------------------------------------------
// Ren'Py
// ---------------------------------------------------------------------------

/// A Ren'Py game whose script tree is spelled with a capital, as a repack does.
fn ibni_renpy(saha: &Saha, mujallad: &str) {
    saha.iktub(&format!("{mujallad}/script.rpyc"), BADEEL);
    saha.iktub(
        &format!("{mujallad}/script.rpy"),
        b"label start:\n\n    \"Hello, traveller.\"\n",
    );
}

#[test]
fn renpy_yuhaddad_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    ibni_renpy(&saha, "Game");

    let (_, aila) = tarkeeb::ayn_hadaf(saha.jidhr())
        .expect("a game whose script tree is Game/ is still a Ren'Py game");
    assert_eq!(aila.ism(), "Ren'Py");
}

#[test]
fn renpy_yaltaqit_min_mujallad_mukhtalif_alhala() {
    let saha = Saha::jadida();
    ibni_renpy(&saha, "Game");

    let (sijillat, _) = tarkeeb::iltiqat_renpy(saha.jidhr());
    assert!(
        sijillat.iter().any(|sijill| sijill.asl == "Hello, traveller."),
        "the walk has to descend into Game/ or the install translates the empty set"
    );
}

#[test]
fn renpy_yastathni_tarajim_alluba_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    ibni_renpy(&saha, "Game");
    // The game's own French. Everything under `game/tl/` is a translation, and
    // reading one as a source string offers French as English to translate from
    // — or, on a second run, offers Taarib's own Arabic back to itself.
    saha.iktub(
        "Game/tl/french/script.rpy",
        "translate french start_1:\n\n    \"Bonjour, voyageur.\"\n".as_bytes(),
    );

    let (sijillat, _) = tarkeeb::iltiqat_renpy(saha.jidhr());
    assert!(
        !sijillat.iter().any(|sijill| sijill.asl == "Bonjour, voyageur."),
        "Game/tl/ is the game's own translations and is never a source"
    );
}

#[test]
fn renpy_yaktub_dakhil_mujallad_alluba_nafsih() {
    let saha = Saha::jadida();
    ibni_renpy(&saha, "Game");

    let idad =
        renpy::IdadRenPy::jadeed(renpy::Masar::Khadim, Rutba::Idad, 90, "khatt.ttf");
    let mut hifz = saha.hifz();
    let maktub = renpy::iktub_idad(&mut hifz, saha.jidhr(), &idad).expect("the settings write");

    for masar in &maktub {
        assert!(masar.is_file(), "{} was reported written and is not there", masar.display());
        assert!(
            masar.starts_with(saha.jidhr().join("Game")),
            "{} landed beside the game's own tree rather than inside it, which is an install \
             the engine never reads",
            masar.display()
        );
    }
    assert!(
        !saha.jidhr().join("game").exists(),
        "a second, lower-case game/ was created; under Wine that directory shadows the \
         game's own content"
    );
}

#[test]
fn renpy_alhala_almutabiqa_taghlib() {
    let saha = Saha::jadida();
    if !bi_hassasiyat_hala(saha.jidhr()) {
        return;
    }
    // Both spellings, with the statement only in the exact one. The engine opens
    // `game/`, so that is the tree whose text must come back.
    ibni_renpy(&saha, "Game");
    saha.iktub("game/script.rpyc", BADEEL);
    saha.iktub("game/script.rpy", b"label start:\n\n    \"The bridge is out.\"\n");

    let (sijillat, _) = tarkeeb::iltiqat_renpy(saha.jidhr());
    assert!(
        sijillat.iter().any(|sijill| sijill.asl == "The bridge is out."),
        "the exactly-spelled game/ is the one the engine loads and the one to read"
    );
    assert!(
        !sijillat.iter().any(|sijill| sijill.asl == "Hello, traveller."),
        "Game/ must not be walked when game/ is right there"
    );
}

// ---------------------------------------------------------------------------
// RPG Maker MV and MZ
// ---------------------------------------------------------------------------

#[test]
fn rpgmaker_yuhaddad_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    // `WWW/JS/` — every component of MV's deployment layout upper-cased, which
    // is what a Windows repacker produces and what Wine hides from the player.
    saha.iktub("WWW/JS/rpg_core.js", b"Bitmap.prototype.drawText = function () {};\n");
    saha.iktub("WWW/DATA/System.json", br#"{"gameTitle":"Riverside"}"#);

    let bunya = rpgmaker::BunyatMashru::iktashif(saha.jidhr())
        .expect("a project under WWW/ is still an RPG Maker project");
    assert_eq!(bunya.isdar(), rpgmaker::IsdarRpg::Mv);
    assert!(
        bunya.malaf("data/System.json").is_file(),
        "the project-relative path has to resolve onto DATA/ or every read and every write \
         in this adapter is against a file that is not there"
    );
    assert!(bunya.bayanat().is_dir(), "data/ resolves to the directory the game actually has");
}

#[test]
fn rpgmaker_alhala_almutabiqa_taghlib() {
    let saha = Saha::jadida();
    if !bi_hassasiyat_hala(saha.jidhr()) {
        return;
    }
    saha.iktub("www/js/rpg_core.js", b"Bitmap.prototype.drawText = function () {};\n");
    saha.iktub("www/data/System.json", br#"{"gameTitle":"exact"}"#);
    saha.iktub("WWW/JS/rpg_core.js", b"Bitmap.prototype.drawText = function () {};\n");
    saha.iktub("WWW/DATA/System.json", br#"{"gameTitle":"folded"}"#);

    let bunya = rpgmaker::BunyatMashru::iktashif(saha.jidhr()).expect("the project");
    let bayt = fs::read(bunya.malaf("data/System.json")).expect("the data file");
    let nass = String::from_utf8(bayt).expect("the data file is UTF-8");
    assert!(
        nass.contains("exact"),
        "the exactly-spelled www/data/ is what the engine loads, and this read {nass}"
    );
}

/// The other half of the same walk: the **leaf names**.
///
/// `masar_bila_hala` resolves the directories a path asks for, and `istakhrij`
/// then compares the names the directory offered against its own table. A
/// repack that lower-cased `data/` lower-cased the files inside it, and an exact
/// comparison there walks past `commonevents.json` — a file holding every common
/// event's dialogue — while `data/` itself resolves perfectly. That is the
/// silent skip [`rpgmaker::istakhrij`]'s own contract refuses, and it is not
/// reachable through the path fold at all.
#[test]
fn rpgmaker_yastakhrij_min_asmaa_bayanat_mukhtalifat_alhala() {
    let saha = Saha::jadida();
    saha.iktub("www/js/rpg_core.js", b"Bitmap.prototype.drawText = function () {};\n");
    saha.iktub("www/data/System.json", br#"{"gameTitle":"Riverside"}"#);
    saha.iktub(
        "www/data/commonevents.json",
        br#"[null,{"id":1,"list":[{"code":401,"parameters":["Hello, traveller."]}]}]"#,
    );
    saha.iktub("www/data/map001.json", br#"{"displayName":"Riverside","events":[]}"#);
    saha.iktub("www/data/actors.json", br#"[null,{"id":1,"name":"Rowan"}]"#);

    let bunya = rpgmaker::BunyatMashru::iktashif(saha.jidhr()).expect("the project");
    let sijill = rpgmaker::istakhrij(&bunya).expect("the extraction");

    for (malaf, nass) in [
        ("data/commonevents.json", "Hello, traveller."),
        ("data/map001.json", "Riverside"),
        ("data/actors.json", "Rowan"),
    ] {
        assert!(
            sijill
                .madakhil
                .iter()
                .any(|madkhal| madkhal.malaf == malaf && madkhal.naqi == nass),
            "{malaf} contributed nothing: a lower-cased data file is still the game's own text"
        );
    }

    // `MapInfos.json` stays excluded through the fold — the digit test, not the
    // spelling, is what keeps the editor's own map tree out of a translation.
    saha.iktub("www/data/mapinfos.json", br#"[null,{"id":1,"name":"Riverside"}]"#);
    let baad = rpgmaker::istakhrij(&bunya).expect("the second extraction");
    assert!(
        !baad.malaffat.contains("data/mapinfos.json"),
        "the editor's map tree is not player-facing text, whatever case it is spelled in"
    );
}

// ---------------------------------------------------------------------------
// GameMaker
// ---------------------------------------------------------------------------

#[test]
fn gamemaker_yuhaddad_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    // The Linux and Android layouts put the container under `assets/`. This one
    // spells it `Assets/`, and the container itself `Data.win`.
    saha.iktub("Assets/Data.win", BADEEL);

    let masar = gamemaker::MifhasGameMaker::hawiya(saha.jidhr())
        .expect("a container under Assets/ is still a container");
    assert!(masar.ends_with("Data.win"));
    assert!(masar.is_file());
}

#[test]
fn gamemaker_alhala_almutabiqa_taghlib() {
    let saha = Saha::jadida();
    if !bi_hassasiyat_hala(saha.jidhr()) {
        return;
    }
    saha.iktub("data.win", BADEEL);
    saha.iktub("DATA.WIN", BADEEL);

    let masar = gamemaker::MifhasGameMaker::hawiya(saha.jidhr()).expect("the container");
    assert!(
        masar.ends_with("data.win"),
        "the first spelling in HAWIYAT that exists verbatim wins: {}",
        masar.display()
    );
}

// ---------------------------------------------------------------------------
// Electron and NW.js
// ---------------------------------------------------------------------------

#[test]
fn ghilaf_yuhaddad_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    // `Resources/` is Electron's own spelling on macOS and what a repacker
    // reaches for anywhere.
    saha.iktub("Resources/app.asar", BADEEL);

    let masar = electron::hawiya(saha.jidhr())
        .expect("an archive under Resources/ is still an application archive");
    assert!(masar.is_file());
    assert!(masar.ends_with("app.asar"));
}

#[test]
fn ghilaf_alhala_almutabiqa_taghlib() {
    let saha = Saha::jadida();
    if !bi_hassasiyat_hala(saha.jidhr()) {
        return;
    }
    saha.iktub("resources/app.asar", b"exact");
    saha.iktub("Resources/app.asar", b"folded");

    let masar = electron::hawiya(saha.jidhr()).expect("the archive");
    assert_eq!(
        fs::read(&masar).expect("the archive bytes"),
        b"exact",
        "the exactly-spelled resources/ is the one the shell loads"
    );
}

// ---------------------------------------------------------------------------
// RPG Maker VX Ace
// ---------------------------------------------------------------------------

#[test]
fn vxace_yuhaddad_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    saha.iktub("DATA/Scripts.rvdata2", BADEEL);

    let mawqi = vxace::ayn_nusus(saha.jidhr())
        .expect("a script list under DATA/ is still a script list");
    assert!(matches!(mawqi, vxace::MawqiNusus::Malaf(_)));
}

#[test]
fn vxace_almalaf_yaghlib_alhawiya() {
    let saha = Saha::jadida();
    // The engine's own rule, restated against a case-differing depot: RGSS3
    // mounts the archive and then lets a real file shadow a member of the same
    // name, so a project with both runs the loose one.
    saha.iktub("DATA/Scripts.rvdata2", BADEEL);
    saha.iktub("game.rgss3a", BADEEL);

    let mawqi = vxace::ayn_nusus(saha.jidhr()).expect("the script list");
    assert!(
        matches!(mawqi, vxace::MawqiNusus::Malaf(_)),
        "a loose Scripts.rvdata2 shadows the archive, whatever either is spelled"
    );
}

// ---------------------------------------------------------------------------
// The dispatcher, end to end
// ---------------------------------------------------------------------------

#[test]
fn rakkib_luba_yutarjim_rughma_ikhtilaf_alhala() {
    let saha = Saha::jadida();
    saha.iktub("Game/script.rpyc", BADEEL);
    saha.iktub(
        "Game/script.rpy",
        b"label start:\n\n    \"Hello, traveller.\"\n",
    );
    saha.iktub("Renpy/__init__.py", b"version_tuple = (8, 3, 4, vc_version)\n");
    saha.iktub("Renpy/text/hbfont.so", b"\x7fELF");
    saha.iktub("Lib/py3-linux-x86_64/libharfbuzz.so.0", b"\x7fELF");
    saha.iktub("Lib/py3-linux-x86_64/libfribidi.so.0", b"\x7fELF");

    let mut jadwal: BTreeMap<String, String> = BTreeMap::new();
    let _ = jadwal.insert("Hello, traveller.".to_owned(), MARHABAN.to_owned());
    let mawarid = Mawarid::default();
    let mut hifz = saha.hifz();

    let taqreer = tarkeeb::rakkib_luba(saha.jidhr(), &jadwal, &mawarid, &mut hifz)
        .expect("the write")
        .expect("an adapter applies to a game whose directories are capitalised");

    assert!(
        taqreer.nusus > 0,
        "the whole point: an install that finds the game must not report success having \
         translated nothing"
    );
    let maktub = taqreer
        .masarat
        .iter()
        .map(|masar| fs::read_to_string(masar).unwrap_or_default())
        .collect::<String>();
    assert!(maktub.contains(MARHABAN), "the Arabic reached a generated file");
}
