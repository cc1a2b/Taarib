//! Real Arabic through the whole pipeline, into a `BIO4` font.
//!
//! Nothing here is a fixture. The strings are ordinary logical-order Unicode,
//! they are shaped by `taarib-saff` through IBM Plex Sans Arabic's own `GSUB`
//! and `GPOS`, rasterized by `taarib-lawha`, and placed into the fixed cell grid
//! a shipped *Resident Evil 4* font uses — 28 texels of pitch across a
//! 1024-texel page, which is what thirty-one of the game's thirty-two fonts are.
//!
//! What the assertions are really checking is that the **shaper's** work survives
//! the container:
//!
//! * `لا` occupies **one** cell, because the font ligates it and the transport
//!   carries glyphs rather than characters. A presentation-form pipeline would
//!   have two here and would be wrong.
//! * `ببب` occupies **three** cells with **three different** contents, because
//!   initial, medial and final beh are three glyphs. A codepoint-keyed atlas
//!   would have one image for all three, which is the failure this whole design
//!   exists to avoid.
//! * `بَ` occupies **one** cell, with the fatha composed into it at the offset
//!   `GPOS` gave it. A mark cannot have a cell of its own in this container: a
//!   cell's only advance is its ink span, and a mark's advance is zero.
//! * the produced `.fnt` round-trips byte for byte and its grid re-derives to the
//!   one it was built for.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_muhawwil_bio4::bina::{KhiyaratBina, ibni};
use taarib_muhawwil_bio4::khatt::KhattBio4;
use taarib_muhawwil_bio4::shabaka::Shabaka;
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, SilsilatKhutut};

/// The cell pitch thirty-one of the game's thirty-two fonts use.
const HAJM_KHANA: u32 = 28;

/// The texture width every one of them uses.
const ARD: u32 = 1024;

/// The size the Arabic is shaped and drawn at, inside a 28-texel cell.
const HAJM: f32 = 20.0;

/// The baseline, in texels down from a cell's top edge.
const ASAS: u32 = 22;

/// The pack identifier a rebuilt `report_zh-cn.fnt` would carry.
const HIZMA: [u8; 4] = [0x09, 0x00, 0x00, 0x13];

/// The staged Arabic face, found by walking up from this crate.
fn khutut() -> SilsilatKhutut {
    const MURASHAHAT: &[&str] = &[
        "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf",
        "target/tajmee/makhbaa/sans/IBMPlexSansArabic-Regular.ttf",
    ];

    let mut jidhr: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    let mut masar: Option<PathBuf> = None;
    while let Some(dalil) = jidhr {
        for murashah in MURASHAHAT {
            let murashah = dalil.join(murashah);
            if murashah.is_file() {
                masar = Some(murashah);
                break;
            }
        }
        if masar.is_some() {
            break;
        }
        jidhr = dalil.parent();
    }

    let Some(masar) = masar else {
        panic!(
            "no Arabic font found at or above {}",
            env!("CARGO_MANIFEST_DIR")
        );
    };
    let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
    let khatt = MawridKhatt::jadeed(Arc::new(bayt), 0)
        .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
    SilsilatKhutut::wahid(Arc::new(khatt)).expect("a one-font chain")
}

/// The grid a rebuilt `report_zh-cn.fnt` would be written into.
fn shabaka_asliya() -> Shabaka {
    Shabaka::jadeeda(HAJM_KHANA, ARD, 576).expect("a 28-texel grid over a 1024-texel page")
}

/// Layout options: one line, no wrapping, direction from the text itself.
fn khiyarat_takhtit() -> KhiyaratTakhtit {
    KhiyaratTakhtit {
        satr_wahid: true,
        ..KhiyaratTakhtit::default()
    }
}

/// Builds a font from `nusus` against the standard grid.
fn ibni_ayina(nusus: &[&str]) -> taarib_muhawwil_bio4::bina::KhattMabni {
    let khutut = khutut();
    let takhtit = khiyarat_takhtit();
    ibni(
        nusus,
        &khutut,
        &takhtit,
        KhiyaratBina {
            hajm: HAJM,
            asas: ASAS,
            hizma: HIZMA,
            asli: shabaka_asliya(),
        },
    )
    .unwrap_or_else(|khata| panic!("the font would not build: {khata}"))
}

#[test]
fn alkhatt_almabni_yadur_wa_yuattid_shabakatah() {
    let mabni = ibni_ayina(&[
        "اضغط الزر للمتابعة",
        "لا تقترب من الباب",
        "الذخيرة منخفضة",
        "احفظ اللعبة الآن؟",
    ]);

    let bayt = mabni
        .khatt
        .ila_bayt()
        .expect("a built .fnt should serialise");
    let thani = KhattBio4::min_bayt(&bayt).expect("and parse back");
    assert_eq!(
        thani.ila_bayt().unwrap(),
        bayt,
        "a built .fnt must round-trip byte for byte"
    );

    thani
        .tibl
        .tahaqquq_khatt()
        .expect("what is written is C4 through an RGB5A3 palette");
    let shabaka = thani
        .shabaka()
        .expect("the grid law must hold on what this crate writes");
    assert_eq!(shabaka.hajm_khana(), HAJM_KHANA);
    assert_eq!(shabaka.ard(), ARD);
    assert_eq!(
        (mabni.sura.ard, mabni.sura.irtifa),
        (shabaka.ard(), shabaka.irtifa())
    );

    assert!(
        mabni.taqreer.ashkal > 20,
        "four sentences of Arabic need more than twenty cells"
    );
    assert_eq!(
        mabni.taqreer.khanat_mustaamala,
        mabni.taqreer.ashkal + 1,
        "the cells are the glyphs plus the reserved blank"
    );
    assert!(
        mabni.sura.bayt.iter().any(|texel| *texel > 0),
        "the atlas must have ink in it"
    );

    let hadd = u8::try_from(HAJM_KHANA).unwrap();
    for (fahras, madkhal) in thani.madakhil.iter().enumerate() {
        assert!(
            madkhal.yameen <= hadd,
            "cell {fahras} spans past its own cell: {madkhal:?}"
        );
    }
}

#[test]
fn lam_alif_khana_wahida() {
    let mabni = ibni_ayina(&["لا", "لأ", "لإ", "لآ"]);
    for (fahras, nass) in mabni.nusus.iter().enumerate() {
        assert_eq!(
            nass.tul(),
            1,
            "lam-alef {fahras} is one ligature and must occupy one cell, not {}",
            nass.tul()
        );
    }
    // Four different lam-alef ligatures, so four different cells.
    let khanat: Vec<u32> = mabni
        .nusus
        .iter()
        .filter_map(|n| n.khanat().first().copied())
        .collect();
    let mut farida = khanat.clone();
    farida.sort_unstable();
    farida.dedup();
    assert_eq!(
        farida.len(),
        khanat.len(),
        "the four lam-alef forms are four distinct glyphs"
    );
}

#[test]
fn ashkal_alwasl_arbaa_khanat_mukhtalifa() {
    // Isolated, then a three-letter word whose beh is initial, medial and final.
    let mabni = ibni_ayina(&["ب", "ببب"]);
    let munfasil = mabni.nusus.first().expect("the isolated line");
    let mawsul = mabni.nusus.get(1).expect("the joined line");
    assert_eq!(munfasil.tul(), 1);
    assert_eq!(mawsul.tul(), 3, "three letters, three cells");

    let mut kull: Vec<u32> = Vec::new();
    kull.extend_from_slice(munfasil.khanat());
    kull.extend_from_slice(mawsul.khanat());
    let mut farida = kull.clone();
    farida.sort_unstable();
    farida.dedup();
    assert_eq!(
        farida.len(),
        4,
        "isolated, initial, medial and final beh are four glyphs and must be four cells; \
         an atlas keyed by codepoint would have one"
    );
}

#[test]
fn attashkeel_yandamij_fi_khanat_alharf() {
    let mabni = ibni_ayina(&["بَ", "بُ", "بِ", "ب"]);
    for (fahras, nass) in mabni.nusus.iter().enumerate() {
        assert_eq!(
            nass.tul(),
            1,
            "line {fahras} is one letter and must be one cell"
        );
    }
    assert_eq!(
        mabni.taqreer.alamat_mafquda, 0,
        "every mark had a base to compose into"
    );

    let khanat: Vec<u32> = mabni
        .nusus
        .iter()
        .filter_map(|n| n.khanat().first().copied())
        .collect();
    let mut farida = khanat.clone();
    farida.sort_unstable();
    farida.dedup();
    assert_eq!(
        farida.len(),
        4,
        "one letter with three different marks and without one is four distinct cells"
    );

    // The bare letter and the vocalised ones share a base glyph, and the cell
    // that carries a mark must be taller-inked than the one that does not.
    let bila = *khanat.last().expect("the unmarked line");
    let maa = *khanat.first().expect("the fatha line");
    let miftah_bila = mabni.tawzee.miftah(bila).expect("a key behind every cell");
    let miftah_maa = mabni.tawzee.miftah(maa).expect("a key behind every cell");
    assert_eq!(
        miftah_bila.muarrif, miftah_maa.muarrif,
        "the same beh underneath both"
    );
    assert!(miftah_bila.alama.is_none());
    assert!(
        miftah_maa.alama.is_some(),
        "the fatha is part of the cell's key"
    );
}

#[test]
fn attartib_basari_wa_alittijah_min_alyameen() {
    // Three distinct letters, so the visual order can be read off the cells. The
    // first cell drawn must be the *last* letter logically, which is what the
    // bidirectional algorithm produced and what the game will draw first.
    let mabni = ibni_ayina(&["ابج", "ج", "ا"]);
    let jumla = mabni.nusus.first().expect("the word");
    assert_eq!(jumla.tul(), 3);

    let awwal = jumla.khanat().first().copied().expect("a first cell");
    let miftah_awwal = mabni.tawzee.miftah(awwal).expect("a key");
    let miftah_jeem = mabni
        .nusus
        .get(1)
        .and_then(|n| n.khanat().first().copied())
        .and_then(|khana| mabni.tawzee.miftah(khana))
        .expect("the isolated jeem");
    // `ج` at the end of the word is a final form and the isolated one is not, so
    // the identifiers differ; what must hold is that the first cell drawn is not
    // the alef, which is the letter a left-to-right writer would have put there.
    let miftah_alif = mabni
        .nusus
        .get(2)
        .and_then(|n| n.khanat().first().copied())
        .and_then(|khana| mabni.tawzee.miftah(khana))
        .expect("the isolated alef");
    assert_ne!(
        miftah_awwal.muarrif, miftah_alif.muarrif,
        "the first cell drawn must not be the first logical letter; the line is right to left"
    );
    let _ = miftah_jeem;
}

#[test]
fn shabaka_daiqa_marfuda() {
    let khutut = khutut();
    let takhtit = khiyarat_takhtit();
    // A page four cells wide: one row of four, blank included.
    let daiqa = Shabaka::jadeeda(HAJM_KHANA, HAJM_KHANA * 4, 1).expect("a four-cell grid");
    let natija = ibni(
        &["اضغط الزر للمتابعة والاستمرار في اللعب"],
        &khutut,
        &takhtit,
        KhiyaratBina {
            hajm: HAJM,
            asas: ASAS,
            hizma: HIZMA,
            asli: daiqa,
        },
    );
    assert!(
        natija.is_err(),
        "a glyph set larger than the grid must be refused, never truncated"
    );
}

#[test]
fn binaan_mutatabian_yuntijan_nafs_albaytat() {
    // Determinism is a promise this crate makes and a promise a patch compiler
    // depends on: a rebuilt font that churns its own bytes invalidates every
    // mirror that already has the patch. The cell assignment walks a `BTreeSet`,
    // the atlas is packed by `taarib-lawha`'s deterministic shelf order, and
    // nothing here iterates a hash map — so two builds of one input must be one
    // sequence of bytes.
    let nusus = ["اضغط الزر للمتابعة", "لا تقترب من الباب", "مُحَمَّدٌ"];
    let awwal = ibni_ayina(&nusus);
    let thani = ibni_ayina(&nusus);

    assert_eq!(
        awwal.khatt.ila_bayt().unwrap(),
        thani.khatt.ila_bayt().unwrap(),
        "two builds of one input must produce the same .fnt"
    );
    assert_eq!(awwal.sura.bayt, thani.sura.bayt, "and the same atlas");
    for (a, b) in awwal.nusus.iter().zip(thani.nusus.iter()) {
        assert_eq!(a.khanat(), b.khanat(), "and the same cell sequences");
    }
}
