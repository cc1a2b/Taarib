//! Read → translate → write → read back, on a real file of each engine's format.
//!
//! Every test here builds a genuine artifact of the format under test — a real
//! `FORM` container with a real `GEN8` and a real `STRG` pool, a real asar with
//! its pickle framing and its directory JSON, a real RPG Maker MV project, a
//! real Ren'Py game tree — patches it through the same entry point the installer
//! calls, and then **reads the result back with this crate's own reader** and
//! compares the Arabic byte for byte.
//!
//! Reading back through the reader rather than grepping the output is the whole
//! point: a container that holds the right bytes at the wrong offset is a
//! container the game will not load, and only the reader can tell the two apart.
//!
//! ## The fixtures are authored, and that is stated rather than blurred
//!
//! No game of any of these four engines is installed on the machine this was
//! written on. Every fixture below is therefore constructed here, from the
//! format definitions the readers in this crate implement — which is a weaker
//! proof than a shipped game and a much stronger one than a mock, because the
//! bytes are produced independently of the code under test and are rejected by
//! it if they are wrong. `MuhawwilGameMaker::ukhruj` re-verifies every untouched
//! region and `HawiyatAsar::uktub` re-parses the buffer it is about to write, so
//! a fixture that were not a real container would fail these tests rather than
//! pass them.
//!
//! ## Arabic goes in as Arabic
//!
//! Every string written here is ordinary Unicode Arabic in logical order.
//! [`la_ashkal_taqdimiya`] checks every byte of every output for the Arabic
//! Presentation Forms blocks, which this product never produces anywhere.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking and asserts on values it has just \
              constructed; the lints are written for library code, and honouring them here \
              would mean a test that cannot fail"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_muhawwil_nusus::hifz::Hifz;
use taarib_muhawwil_nusus::tarkeeb::{self, Mawarid, TaqreerTarkeeb};
use taarib_muhawwil_nusus::{electron, gamemaker, rpgmaker};

// ---------------------------------------------------------------------------
// The Arabic under test
// ---------------------------------------------------------------------------

/// `Hello, traveller.` — ordinary Unicode, logical order, no presentation forms.
const MARHABAN: &str = "مرحبًا أيها المسافر.";

/// `The bridge is out.`
const JISR: &str = "الجسر مقطوع.";

/// `Start Game`
const IBDA: &str = "ابدأ اللعبة";

/// `Riverside`
const DIFAF: &str = "ضفاف النهر";

/// `Quit`
const KHURUJ: &str = "خروج";

/// `Beware!`
const IHDHAR: &str = "احذر!";

// ---------------------------------------------------------------------------
// Shared scaffolding
// ---------------------------------------------------------------------------

/// A game directory and the backup directory beside it, both real.
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
        Hifz::ibda(&self.luba, &self.nusakh, "dawra").expect("a preservation session")
    }

    fn iktub(&self, nisbi: &str, muhtawa: &[u8]) {
        let masar = self.luba.join(nisbi);
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid).expect("a fixture directory");
        }
        fs::write(&masar, muhtawa).expect("a fixture file");
    }

    fn iqra(&self, nisbi: &str) -> Vec<u8> {
        fs::read(self.luba.join(nisbi)).expect("reading a patched file back")
    }

    fn iqra_nass(&self, nisbi: &str) -> String {
        String::from_utf8(self.iqra(nisbi)).expect("a patched file that is still UTF-8")
    }
}

/// Runs the dispatcher exactly as the installer does.
fn rakkib(saha: &Saha, jadwal: &BTreeMap<String, String>, mawarid: &Mawarid<'_>) -> TaqreerTarkeeb {
    let mut hifz = saha.hifz();
    tarkeeb::rakkib_luba(&saha.luba, jadwal, mawarid, &mut hifz)
        .expect("the script-engine write")
        .expect("an adapter that applies to this directory")
}

/// Every Arabic Presentation Form codepoint, which this product never emits.
///
/// The two blocks are U+FB50..=U+FDFF and U+FE70..=U+FEFF. A shaped glyph is a
/// glyph identifier and never a codepoint; anything in these ranges reaching a
/// file would mean some path decided to "shape" by substituting characters,
/// which is the failure mode this whole product exists to replace.
fn shakl_taqdimi(nass: &str) -> Option<char> {
    nass.chars().find(|harf| {
        matches!(u32::from(*harf), 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
    })
}

fn la_ashkal(nass: &str, ayn: &str) {
    assert!(
        shakl_taqdimi(nass).is_none(),
        "{ayn} carries the Arabic Presentation Form {:?}, which nothing in this product \
         produces",
        shakl_taqdimi(nass)
    );
}

// ---------------------------------------------------------------------------
// Ren'Py
// ---------------------------------------------------------------------------

/// A Ren'Py 8 game tree: the engine marker, the shaping module, the shared
/// libraries, a script and a translation the game already ships.
fn ibni_renpy(saha: &Saha) {
    saha.iktub(
        "renpy/__init__.py",
        b"# Ren'Py\nversion_tuple = (8, 3, 4, vc_version)\nversion = \"8.3.4\"\n",
    );
    // The HarfBuzz text module, which is the one conclusive thing that can be
    // observed about a Ren'Py build.
    saha.iktub("renpy/text/hbfont.so", b"\x7fELF");
    saha.iktub("renpy/text/ftfont.so", b"\x7fELF");
    saha.iktub("lib/py3-linux-x86_64/libharfbuzz.so.0", b"\x7fELF");
    saha.iktub("lib/py3-linux-x86_64/libfribidi.so.0", b"\x7fELF");

    saha.iktub(
        "game/script.rpy",
        concat!(
            "label start:\n",
            "\n",
            "    \"Hello, traveller.\"\n",
            "\n",
            "    e \"The bridge is out.\"\n",
            "\n",
            "    menu:\n",
            "        \"Start Game\":\n",
            "            jump chapter_one\n",
            "        \"Quit\":\n",
            "            return\n",
        )
        .as_bytes(),
    );
    // A translation the game itself ships. Ren'Py's statement identifiers are a
    // hash of the statement's regenerated source and only the engine can
    // compute one, so they are recovered from here and never invented.
    saha.iktub(
        "game/tl/french/script.rpy",
        concat!(
            "translate french start_8f3a2b1c:\n",
            "\n",
            "    # \"Hello, traveller.\"\n",
            "    \"Bonjour, voyageur.\"\n",
            "\n",
        )
        .as_bytes(),
    );
}

#[test]
fn renpy_yaktub_arabiyan_wa_yuidu_qiraatah() {
    let saha = Saha::jadida();
    ibni_renpy(&saha);

    let mut jadwal = BTreeMap::new();
    let _ = jadwal.insert("Hello, traveller.".to_owned(), MARHABAN.to_owned());
    let _ = jadwal.insert("The bridge is out.".to_owned(), JISR.to_owned());
    let _ = jadwal.insert("Start Game".to_owned(), IBDA.to_owned());
    // "Quit" is deliberately absent: an untranslated string must be left out of
    // the generated file rather than written back in its own language.

    let taqreer = rakkib(&saha, &jadwal, &Mawarid::default());
    assert_eq!(taqreer.aila.ism(), "Ren'Py");
    assert_eq!(
        taqreer.rutba,
        Some(taarib_muhawwil_nusus::Rutba::Idad),
        "a Ren'Py 8 that ships hbfont shapes Arabic itself"
    );

    let mustalahat = saha.iqra_nass("game/tl/arabic/taarib_mustalahat.rpy");
    let hiwar = saha.iqra_nass("game/tl/arabic/taarib_hiwar.rpy");
    let idad = saha.iqra_nass("game/tl/arabic/taarib_idad.rpy");
    let bayanat = saha.iqra_nass("game/taarib/idad.json");

    // The identifier the game's own French translation carried, reused exactly.
    assert!(
        hiwar.contains("translate arabic start_8f3a2b1c:"),
        "the recovered statement identifier is what the dialogue block is keyed on:\n{hiwar}"
    );
    assert!(
        hiwar.contains(&format!("\"{MARHABAN}\"")),
        "the Arabic dialogue survives the write byte for byte:\n{hiwar}"
    );
    assert!(
        !hiwar.contains("\"Hello, traveller.\"\n\n    \"Hello"),
        "the source is a comment, never the translation"
    );

    assert!(
        mustalahat.contains(&format!("new \"{IBDA}\"")),
        "the menu choice is delivered through the strings mechanism:\n{mustalahat}"
    );
    assert!(
        mustalahat.contains(&format!("old \"{}\"", "Start Game")),
        "matched on the source text, which is how Ren'Py's strings block works"
    );
    assert!(
        !mustalahat.contains("old \"Quit\""),
        "an untranslated string must not be emitted at all: an identity rule shadows the \
         string instead of translating it\n{mustalahat}"
    );

    // No font was supplied, so no font is registered — and the direction half,
    // which is what is wrong even on an engine that joins every letter, is.
    assert!(!idad.contains("sajjil_khatt"), "no font supplied means no font registered:\n{idad}");
    assert!(idad.contains("sajjil_ittijah()"), "the direction is always set:\n{idad}");
    assert!(idad.contains("text_align 1.0"), "and so is the alignment:\n{idad}");
    assert!(bayanat.contains("\"lugha\": \"arabic\""), "the settings name the language");

    for (ayn, nass) in [("taarib_hiwar.rpy", &hiwar), ("taarib_mustalahat.rpy", &mustalahat)] {
        la_ashkal(nass, ayn);
    }

    // Read the generated file back through this crate's own Ren'Py scanner: the
    // Arabic has to be a real say statement inside a real translate block, not
    // merely a substring that happens to be present.
    let mudawwar = taarib_muhawwil_nusus::renpy::iltiqat_min_rpy(&hiwar, "taarib_hiwar.rpy");
    assert!(
        mudawwar.iter().any(|sijill| sijill.asl == MARHABAN),
        "the Arabic parses back out as a say statement:\n{hiwar}"
    );

    // And running the extraction over the patched game again must not offer any
    // of it back as a source string. Everything under `game/tl/` is a
    // translation — the game's own French and now Taarib's Arabic — and a pass
    // that read it would key the next patch on this one's output.
    let (thaniya, _) = tarkeeb::iltiqat_renpy(&saha.luba);
    assert!(
        thaniya.iter().all(|sijill| sijill.asl != MARHABAN && sijill.asl != IBDA),
        "a second pass over a patched game must not re-ingest its own Arabic"
    );
    assert!(
        thaniya.iter().all(|sijill| sijill.asl != "Bonjour, voyageur."),
        "nor the game's own French"
    );
    assert!(
        thaniya.iter().any(|sijill| sijill.asl == "Hello, traveller."),
        "and the game's own English is still what it offers"
    );
}

// ---------------------------------------------------------------------------
// RPG Maker MV
// ---------------------------------------------------------------------------

/// A deployed RPG Maker MV project: the core scripts, the plugin registry and
/// the two data files whose event commands carry the dialogue.
fn ibni_rpgmaker(saha: &Saha) {
    saha.iktub(
        "js/rpg_core.js",
        b"Bitmap.prototype.drawText = function(text, x, y, maxWidth, lineHeight, align) {\n\
          this._context.fillText(text, x, y, maxWidth);\n};\n",
    );
    saha.iktub(
        "js/rpg_windows.js",
        b"Window_Base.prototype.processNormalCharacter = function(textState) {\n\
          var c = textState.text[textState.index++];\n\
          this.contents.drawText(c, textState.x, textState.y, 100, textState.height);\n};\n",
    );
    saha.iktub("js/plugins.js", b"// Generated by RPG Maker.\nvar $plugins =\n[\n];\n");
    saha.iktub(
        "data/Map001.json",
        concat!(
            r#"{"displayName":"Riverside","width":17,"height":13,"events":[null,"#,
            r#"{"id":1,"name":"EV001","x":8,"y":6,"pages":[{"list":["#,
            "\n",
            r#"{"code":101,"indent":0,"parameters":["Actor1",0,0,2]},"#,
            "\n",
            r#"{"code":401,"indent":0,"parameters":["Hello, traveller."]},"#,
            "\n",
            r#"{"code":401,"indent":0,"parameters":["The bridge is out."]},"#,
            "\n",
            r#"{"code":401,"indent":0,"parameters":["\\V[1] gold remains."]},"#,
            "\n",
            r#"{"code":401,"indent":0,"parameters":["\\C[2]Beware!"]},"#,
            "\n",
            r#"{"code":0,"indent":0,"parameters":[]}]}]}]}"#,
        )
        .as_bytes(),
    );
    saha.iktub(
        "data/System.json",
        br#"{"gameTitle":"Riverside","currencyUnit":"G","locale":"en_US",
"advanced":{"mainFontFilename":"mplus-1m-regular.ttf","fontSize":28},
"terms":{"basic":["Level"],"commands":["Fight","Escape"],"params":["Max HP"],
"messages":{"actionFailure":"There was no effect on %1!"}}}"#,
    );
}

#[test]
fn rpgmaker_yaktub_arabiyan_fi_bayanatih() {
    let saha = Saha::jadida();
    ibni_rpgmaker(&saha);

    let mut jadwal = BTreeMap::new();
    let _ = jadwal.insert("Hello, traveller.".to_owned(), MARHABAN.to_owned());
    let _ = jadwal.insert("The bridge is out.".to_owned(), JISR.to_owned());
    let _ = jadwal.insert("Riverside".to_owned(), DIFAF.to_owned());
    // The clean form of a line that carries a variable substitution. Writing
    // this would drop the `\V[1]` the engine substitutes at draw time, so the
    // dispatcher must leave the line in English and say so.
    let _ = jadwal.insert(" gold remains.".to_owned(), " قطعة ذهبية متبقية.".to_owned());
    // A colour change is a typographic decision, not information: dropping it
    // is allowed, so this one is written.
    let _ = jadwal.insert("Beware!".to_owned(), IHDHAR.to_owned());

    let taqreer = rakkib(&saha, &jadwal, &Mawarid::default());
    assert_eq!(taqreer.aila.ism(), "RPG Maker MV");
    assert!(taqreer.nusus >= 3, "at least the three plain lines were written: {taqreer:?}");
    assert!(
        taqreer.mulahazat.iter().any(|satr| satr.contains("escape code")),
        "the skipped line is reported, not silently dropped: {:?}",
        taqreer.mulahazat
    );

    let khaam = saha.iqra_nass("data/Map001.json");
    la_ashkal(&khaam, "data/Map001.json");
    let qeema: serde_json::Value =
        serde_json::from_str(&khaam).expect("the patched map is still valid JSON");

    let qaima = &qeema["events"][1]["pages"][0]["list"];
    assert_eq!(qaima[1]["parameters"][0], serde_json::json!(MARHABAN));
    assert_eq!(qaima[2]["parameters"][0], serde_json::json!(JISR));
    assert_eq!(
        qaima[3]["parameters"][0],
        serde_json::json!("\\V[1] gold remains."),
        "a translation that lost a variable substitution is refused, and the line stays"
    );
    assert_eq!(
        qaima[4]["parameters"][0],
        serde_json::json!(IHDHAR),
        "a colour change is typography and may be dropped by the translator"
    );
    assert_eq!(qeema["displayName"], serde_json::json!(DIFAF));
    assert_eq!(qeema["width"], serde_json::json!(17), "untouched fields are untouched");

    // Read back through this crate's own extractor: the map must now offer the
    // Arabic where it offered the English, at a byte range that still resolves.
    let bunya = rpgmaker::BunyatMashru::iktashif(&saha.luba).expect("the project");
    let sijill = rpgmaker::istakhrij(&bunya).expect("a second extraction pass");
    let mudawwar: Vec<&str> =
        sijill.madakhil.iter().map(|madkhal| madkhal.khaam.as_str()).collect();
    assert!(mudawwar.contains(&MARHABAN), "the Arabic is what a re-read now finds");
    assert!(mudawwar.contains(&JISR));
    assert!(!mudawwar.contains(&"Hello, traveller."), "and the English is gone");

    // The original is preserved byte-exact, which is what makes it reversible.
    let asl = fs::read_dir(saha.nusakh.join("asl")).expect("the originals directory");
    assert!(asl.count() > 0, "the original data file was preserved before it was rewritten");
}

// ---------------------------------------------------------------------------
// GameMaker
// ---------------------------------------------------------------------------

/// Builds a genuine `FORM` container holding a `GEN8`, a `FONT` and a `STRG`.
///
/// Every offset in this format is an **absolute file offset**, which is why the
/// chunks are sized before any of them is filled in: the pool's pointer array
/// and the font's two name pointers all have to name bytes whose position is
/// only known once the layout is fixed.
///
/// `STRG` is last, which is what a real container does and what lets an appended
/// record grow the chunk without moving anything behind it. `FONT` is present
/// because a GameMaker game's text comes out of a baked glyph table, and that
/// chunk is the one observation the tier probe treats as decisive.
fn ibni_data_win(nusus: &[&str]) -> Vec<u8> {
    const HAJM_GEN8: usize = 108;
    // 4 count + 4 pointer + 40 fixed fields + 4 glyph count + 8 pointer array
    // + 2 glyph records of 16 bytes each.
    const HAJM_FONT: usize = 4 + 4 + 40 + 4 + 8 + 32;
    const ADAD_ASHKAL: u32 = 2;

    let bidayat_gen8 = 8 + 8; // FORM header, then GEN8's own name and length.
    let bidayat_font = bidayat_gen8 + HAJM_GEN8 + 8;
    let bidayat_strg = bidayat_font + HAJM_FONT + 8;

    // The pool, with every pointer resolved against its final file offset.
    let mut mu_ashirat: Vec<u32> = Vec::with_capacity(nusus.len());
    let mut ajsad: Vec<u8> = Vec::new();
    let ras_ajsad = bidayat_strg + 4 + nusus.len() * 4;
    for nass in nusus {
        let mawqi_sijil = ras_ajsad + ajsad.len();
        ajsad.extend_from_slice(&u32::try_from(nass.len()).unwrap().to_le_bytes());
        ajsad.extend_from_slice(nass.as_bytes());
        ajsad.push(0);
        mu_ashirat.push(u32::try_from(mawqi_sijil + 4).unwrap());
    }
    let mut strg: Vec<u8> = Vec::new();
    strg.extend_from_slice(&u32::try_from(nusus.len()).unwrap().to_le_bytes());
    for mu_ashir in &mu_ashirat {
        strg.extend_from_slice(&mu_ashir.to_le_bytes());
    }
    strg.extend_from_slice(&ajsad);

    // GEN8. Only the fields the reader names are meaningful; the rest is the
    // zero padding a real chunk has between them.
    let mut gen8 = vec![0_u8; HAJM_GEN8];
    gen8[0] = 0; // the debugger is not disabled
    gen8[1] = 17; // bytecode version 17 — GameMaker 2022 and newer
    {
        let mut daa = |izaha: usize, qeema: u32| {
            gen8[izaha..izaha + 4].copy_from_slice(&qeema.to_le_bytes());
        };
        daa(4, mu_ashirat[0]); // the project filename
        daa(8, mu_ashirat[0]); // the configuration name
        daa(20, 0x0001_3579); // the game id
        daa(40, mu_ashirat[0]); // the game's own name
        daa(44, 2); // runtime major
        daa(48, 3); // minor
        daa(52, 1); // release
        daa(56, 642); // build
        daa(60, 1280); // window width
        daa(64, 720); // window height
        daa(68, 0); // info flags
    }

    // FONT: one entry, whose glyph table sits at the fixed offset the reader
    // searches from, and whose first glyph pointer lands exactly past the
    // pointer array — which is the invariant `jadwal_ashkal` verifies.
    let mawqi_madkhal = u32::try_from(bidayat_font + 8).unwrap();
    let bidayat_ashkal = mawqi_madkhal + 40 + 4 + ADAD_ASHKAL * 4;
    let mut font: Vec<u8> = Vec::new();
    font.extend_from_slice(&1_u32.to_le_bytes()); // one font
    font.extend_from_slice(&mawqi_madkhal.to_le_bytes());
    let ism_khatt = mu_ashirat[nusus.len() - 2]; // "fnt_dialog"
    let ism_ard = mu_ashirat[nusus.len() - 1]; // "Arial"
    font.extend_from_slice(&ism_khatt.to_le_bytes());
    font.extend_from_slice(&ism_ard.to_le_bytes());
    font.extend_from_slice(&12_u32.to_le_bytes()); // point size
    font.extend_from_slice(&0_u32.to_le_bytes()); // not bold
    font.extend_from_slice(&0_u32.to_le_bytes()); // not italic
    font.extend_from_slice(&32_u16.to_le_bytes()); // range start
    font.push(0); // charset
    font.push(1); // antialiasing
    font.extend_from_slice(&127_u32.to_le_bytes()); // range end
    font.extend_from_slice(&0_u32.to_le_bytes()); // no TPAG item
    font.extend_from_slice(&1.0_f32.to_le_bytes()); // scale x
    font.extend_from_slice(&1.0_f32.to_le_bytes()); // scale y
    font.extend_from_slice(&ADAD_ASHKAL.to_le_bytes());
    for fahras in 0..ADAD_ASHKAL {
        font.extend_from_slice(&(bidayat_ashkal + fahras * 16).to_le_bytes());
    }
    for (harf, s) in [(b'A', 0_u16), (b'B', 8_u16)] {
        font.extend_from_slice(&u16::from(harf).to_le_bytes()); // character code
        font.extend_from_slice(&s.to_le_bytes()); // x on the page
        font.extend_from_slice(&0_u16.to_le_bytes()); // y
        font.extend_from_slice(&8_u16.to_le_bytes()); // width
        font.extend_from_slice(&12_u16.to_le_bytes()); // height
        font.extend_from_slice(&0_i16.to_le_bytes()); // offset
        font.extend_from_slice(&8_i16.to_le_bytes()); // advance
        font.extend_from_slice(&0_u16.to_le_bytes()); // no kerning pairs
    }
    assert_eq!(font.len(), HAJM_FONT, "the FONT chunk's size is what the layout assumed");

    let mut hamula: Vec<u8> = Vec::new();
    for (ism, juz) in [(b"GEN8", &gen8), (b"FONT", &font), (b"STRG", &strg)] {
        hamula.extend_from_slice(ism);
        hamula.extend_from_slice(&u32::try_from(juz.len()).unwrap().to_le_bytes());
        hamula.extend_from_slice(juz);
    }

    let mut hawiya: Vec<u8> = Vec::new();
    hawiya.extend_from_slice(b"FORM");
    hawiya.extend_from_slice(&u32::try_from(hamula.len()).unwrap().to_le_bytes());
    hawiya.extend_from_slice(&hamula);
    hawiya
}

#[test]
fn gamemaker_yaktub_arabiyan_fi_hawd_nususih() {
    let saha = Saha::jadida();
    let nusus =
        ["Riverside", "Hello, traveller.", "Start Game", "spr_hero", "fnt_dialog", "Arial"];
    let khaam = ibni_data_win(&nusus);
    saha.iktub("data.win", &khaam);

    // The fixture is a container this crate's own reader accepts, or it is not a
    // fixture. Proving that before patching it is what makes the round trip
    // mean something.
    let qabl = gamemaker::HawiyatGameMaker::min_bayt(khaam, Path::new("data.win"))
        .expect("the fixture is a readable FORM container");
    assert_eq!(qabl.hawd().adad(), nusus.len());
    assert_eq!(qabl.ism_luba(), Some("Riverside"));
    assert_eq!(qabl.khutut().len(), 1, "the FONT chunk parses as one font");
    assert_eq!(qabl.khutut()[0].ism, "fnt_dialog");
    assert_eq!(qabl.khutut()[0].ashkal.len(), 2, "and its baked glyph table as two glyphs");

    let mut jadwal = BTreeMap::new();
    let _ = jadwal.insert("Riverside".to_owned(), DIFAF.to_owned());
    let _ = jadwal.insert("Hello, traveller.".to_owned(), MARHABAN.to_owned());
    let _ = jadwal.insert("Start Game".to_owned(), IBDA.to_owned());
    // `spr_hero`, `fnt_dialog` and `Arial` are resource names, not text, and the
    // patch carries no translation for any of them: they must come back exactly
    // as they went in.

    let taqreer = rakkib(&saha, &jadwal, &Mawarid::default());
    assert_eq!(
        taqreer.rutba,
        Some(taarib_muhawwil_nusus::Rutba::Istila),
        "a baked glyph table indexed by character code cannot join Arabic"
    );
    assert_eq!(taqreer.nusus, 3);
    assert_eq!(taqreer.matruka, 3, "the three resource names had no translation");

    // Read the patched container back with the same reader.
    let baad_khaam = saha.iqra("data.win");
    let baad = gamemaker::HawiyatGameMaker::min_bayt(baad_khaam, Path::new("data.win"))
        .expect("the patched container still parses");
    assert_eq!(baad.hawd().adad(), nusus.len(), "the pool's order and count never change");

    let madakhil = baad.hawd().madakhil();
    assert_eq!(madakhil[0].nass, DIFAF, "byte for byte, in logical order");
    assert_eq!(madakhil[1].nass, MARHABAN);
    assert_eq!(madakhil[2].nass, IBDA);
    assert_eq!(madakhil[3].nass, "spr_hero", "an untranslated entry is untouched");
    for madkhal in madakhil {
        la_ashkal(&madkhal.nass, "the GameMaker string pool");
    }

    // Every reference into the pool was repointed: the game's own name is read
    // through GEN8's pointer, and it now resolves to the Arabic.
    assert_eq!(baad.ism_luba(), Some(DIFAF), "GEN8's name pointer was repointed, not left");
    assert_eq!(baad.gen8().muarrif, 0x0001_3579, "and nothing else in GEN8 moved");

    // The font resource is still described by the same chunk, still names the
    // same pool entry, and still carries the same glyph table.
    assert_eq!(baad.khutut().len(), 1);
    assert_eq!(baad.khutut()[0].ism, "fnt_dialog", "a pointer into the pool that did not move");
    assert_eq!(baad.khutut()[0].ashkal.len(), 2);
    assert_eq!(baad.khutut()[0].ashkal[0].harf, u16::from(b'A'));
}

// ---------------------------------------------------------------------------
// Electron
// ---------------------------------------------------------------------------

/// Builds a genuine `asar`: the sixteen bytes of pickle framing, the directory
/// JSON padded to four, then the packed bodies in directory order.
fn ibni_asar(madakhil: &[(&str, &[u8])]) -> Vec<u8> {
    let mut ajsad: Vec<u8> = Vec::new();
    let mut malaffat = serde_json::Map::new();
    for (masar, muhtawa) in madakhil {
        let mut uqda = serde_json::Map::new();
        let _ = uqda.insert("size".to_owned(), serde_json::json!(muhtawa.len()));
        let _ = uqda.insert("offset".to_owned(), serde_json::json!(ajsad.len().to_string()));
        let _ = malaffat.insert((*masar).to_owned(), serde_json::Value::Object(uqda));
        ajsad.extend_from_slice(muhtawa);
    }
    let shajara = serde_json::json!({ "files": malaffat });
    let json = serde_json::to_vec(&shajara).unwrap();

    let tarwisa = electron::TarwisatAsar::li_tul(u64::try_from(json.len()).unwrap())
        .expect("the framing for this directory");
    let mut hawiya = Vec::from(tarwisa.ila_bayt().expect("the framing bytes"));
    hawiya.extend_from_slice(&json);
    hawiya.resize(usize::try_from(tarwisa.bidayat_muhtawa).unwrap(), 0);
    hawiya.extend_from_slice(&ajsad);
    hawiya
}

/// The compiled renderer runtime the Electron adapter embeds.
///
/// The shipped one is `adapters-script/electron/taarib.ts` after `esbuild` has
/// run over it, and it is read from the component store at install time. This
/// stands in for it here so the round trip can be proved on a machine with no
/// JavaScript toolchain; it is the transport that is under test, not the
/// runtime's own behaviour.
const TASHGHIL: &str = "'use strict';\n\
                        // stand-in for the compiled renderer runtime\n\
                        module.exports = { rakkib: function () {} };\n";

#[test]
fn ghilaf_yaktub_arabiyan_dakhil_asar() {
    let saha = Saha::jadida();
    let index = b"'use strict';\n\
        const root = document.querySelector('#app');\n\
        root.appendChild(document.createTextNode(labels.start));\n\
        root.textContent = labels.quit;\n";
    let nusus_json = br#"{"start":"Start Game","quit":"Quit","hint":"Hello, traveller."}"#;
    let package = br#"{"name":"riverside","version":"1.0.0","main":"index.js"}"#;
    let khaam = ibni_asar(&[
        ("package.json", package),
        ("index.js", index),
        ("strings.json", nusus_json),
    ]);
    saha.iktub("resources/app.asar", &khaam);

    let qabl = electron::HawiyatAsar::min_masar(&saha.luba.join("resources/app.asar"))
        .expect("the fixture is a readable asar");
    assert_eq!(qabl.madakhil().len(), 3);
    assert_eq!(qabl.muhtawa("index.js"), Some(&index[..]));

    let mut jadwal = BTreeMap::new();
    let _ = jadwal.insert("Start Game".to_owned(), IBDA.to_owned());
    let _ = jadwal.insert("Quit".to_owned(), KHURUJ.to_owned());
    let _ = jadwal.insert("Hello, traveller.".to_owned(), MARHABAN.to_owned());

    let mawarid = Mawarid { tashghil_ghilaf: Some(TASHGHIL), khatt_renpy: None };
    let taqreer = rakkib(&saha, &jadwal, &mawarid);
    assert_eq!(taqreer.aila.ism(), "Electron");
    assert!(taqreer.nusus >= 3, "every string the sweep found was translated: {taqreer:?}");

    // Read the repacked archive back with the same reader.
    let baad = electron::HawiyatAsar::min_masar(&saha.luba.join("resources/app.asar"))
        .expect("the repacked archive still parses");

    let hamula = baad.muhtawa("taarib/hamula.json").expect("the payload is inside the archive");
    let nass = std::str::from_utf8(hamula).expect("the payload is UTF-8");
    la_ashkal(nass, "taarib/hamula.json");
    let qeema: serde_json::Value = serde_json::from_str(nass).expect("the payload is JSON");
    assert_eq!(qeema["jadwal"]["Start Game"], serde_json::json!(IBDA));
    assert_eq!(qeema["jadwal"]["Quit"], serde_json::json!(KHURUJ));
    assert_eq!(qeema["jadwal"]["Hello, traveller."], serde_json::json!(MARHABAN));
    assert_eq!(qeema["ittijah"], serde_json::json!("rtl"));

    assert_eq!(
        baad.muhtawa("taarib/taarib.js").map(<[u8]>::to_vec),
        Some(TASHGHIL.as_bytes().to_vec()),
        "the runtime that reads the payload travels with it"
    );

    // The application's own files are byte-identical, and its entry point is
    // recorded so an uninstall can put it back.
    assert_eq!(baad.muhtawa("index.js"), Some(&index[..]), "the application's own script");
    assert_eq!(baad.muhtawa("strings.json"), Some(&nusus_json[..]));
    let huzma: serde_json::Value =
        serde_json::from_slice(baad.muhtawa("package.json").expect("package.json")).unwrap();
    assert_eq!(huzma["main"], serde_json::json!("taarib/tamhid.js"));
    assert_eq!(huzma["taaribAsl"], serde_json::json!("index.js"));
    assert_eq!(huzma["name"], serde_json::json!("riverside"));
}

#[test]
fn ghilaf_yarfud_bila_tashghil() {
    let saha = Saha::jadida();
    let khaam = ibni_asar(&[
        ("package.json", br#"{"name":"riverside","main":"index.js"}"#),
        ("index.js", b"document.createTextNode('Start Game');\n"),
    ]);
    saha.iktub("resources/app.asar", &khaam);

    let mut jadwal = BTreeMap::new();
    let _ = jadwal.insert("Start Game".to_owned(), IBDA.to_owned());

    let mut hifz = saha.hifz();
    let natija = tarkeeb::rakkib_luba(&saha.luba, &jadwal, &Mawarid::default(), &mut hifz);
    let khata = natija.expect_err("a payload with no runtime to read it must be refused");
    assert!(
        khata.to_string().contains("asar runtime"),
        "the refusal names the missing component: {khata}"
    );
    // And nothing was written: the archive is byte-identical to the fixture.
    assert_eq!(saha.iqra("resources/app.asar"), khaam);
}

// ---------------------------------------------------------------------------
// The rule that holds across all four
// ---------------------------------------------------------------------------

#[test]
fn la_ashkal_taqdimiya() {
    // Every constant this file writes is checked at its source as well as in
    // every output above, so a fixture that introduced a presentation form
    // would fail here before it reached any format.
    for nass in [MARHABAN, JISR, IBDA, DIFAF, KHURUJ, IHDHAR] {
        la_ashkal(nass, "a test constant");
    }
}

#[test]
fn ayn_hadaf_yufaddil_almuharrik_aldakhili() {
    // An Electron shell around an RPG Maker MV game is two true statements about
    // one directory. The inner engine wins, because a shell upgrade replaces
    // app.asar wholesale and leaves data/ alone.
    let saha = Saha::jadida();
    saha.iktub("resources/app/js/rpg_core.js", b"Bitmap.prototype.drawText = function () {};\n");
    saha.iktub("resources/app.asar", &ibni_asar(&[("package.json", b"{\"main\":\"i.js\"}")]));

    let (_, aila) = tarkeeb::ayn_hadaf(&saha.luba).expect("an adapter applies");
    assert_eq!(aila.ism(), "RPG Maker MV", "the inner engine beats the wrapper");

    // And a game on none of the four is not this crate's to touch.
    let akhar = Saha::jadida();
    akhar.iktub("Riverside_Data/globalgamemanagers", b"\x00\x00\x00\x00");
    assert!(tarkeeb::ayn_hadaf(&akhar.luba).is_none());
}
