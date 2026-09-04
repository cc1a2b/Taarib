//! The dispatch point, end to end: a real `.ruqaa` in, Arabic in the game out.
//!
//! The round-trip proofs for each engine's *format* live in
//! `taarib-muhawwil-nusus`. What is proved here is the join this crate owns and
//! that nothing in the product had before: a package's string table, keyed on a
//! hash of the source text and carrying no source text at all, resolving against
//! the strings a real game directory actually holds, and the result reaching
//! disk through the installation recorder rather than through a file handle.
//!
//! Ren'Py is the engine under test because its write is pure text and needs
//! nothing from the component store, so this exercises the whole chain —
//! `MalafRuqaa` → `MutarjimRuqaa` → `rakkib_luba` → `HafizMuthabbit` →
//! `Tathbeet` → the game — with no build artifact standing in for anything.

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
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::katib::Katib;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba, Tathbeet};
use taarib_tathbeet::nusus;

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
