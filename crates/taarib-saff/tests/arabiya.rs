//! Engine-level proof that Taarib renders Arabic the way Arabic is written.
//!
//! Every test here asserts one property of the shaped output, and every one of
//! them fails on the implementation this product exists to replace: a renderer
//! that maps each codepoint through `cmap` and draws the results left to right.
//! Where a claim needs a control, the control is in the same test.
//!
//! # Coordinate convention, read out of the code rather than assumed
//!
//! [`taarib_saff::Harf::s`] is the horizontal position of a glyph's origin **in
//! pixels from the layout's left edge**, and [`taarib_saff::Harf::a`] is
//! measured **downward from the layout's top**
//! (`crates/taarib-saff/src/natija.rs:29-36`). Glyphs are stored one line at a
//! time in **visual order, left to right**: `qiyas::mawqi_huruf`
//! (`crates/taarib-saff/src/qiyas.rs:1645-1703`) starts a pen at the line's
//! aligned beginning and walks it forward, pushing each glyph as it goes, after
//! rule L2 has already reordered the runs
//! (`crates/taarib-saff/src/qiyas.rs:1573`).
//!
//! Right-to-left therefore has a precise, checkable meaning here: **as `s`
//! increases, the cluster index decreases**. The first character of the string
//! is drawn at the greatest `s`.
//!
//! A glyph's `GPOS` offsets are not stored on [`taarib_saff::Harf`]; they are
//! already folded into `s` and `a`. They are recovered in these tests by
//! walking the pen exactly the way `mawqi_huruf` walked it — see [`izahat`] —
//! which is a derivation from public output, not a second measurement.
//!
//! # The font
//!
//! IBM Plex Sans Arabic 6.4.2 Regular, the face this repository pins by content
//! hash in `assets/aqfal/qufl_khutut.json` and stages into
//! `apps/studio/src/khutut/`. Nothing here downloads anything.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_saff::{
    Dharra, Harf, Ittijah, KhiyaratTakhtit, MawridKhatt, NamatRasm, NitaqUslub, Rassam, Saff,
    SilsilatKhutut, TakhtitNass, TalabTakhtit, Uslub,
};

/// The size every test lays out at, in pixels.
const HAJM: f32 = 44.0;

/// The byte length `assets/aqfal/qufl_khutut.json` records for the Regular
/// face, checked so a swapped file is caught before any assertion runs.
const TUL_MALAF: usize = 236_708;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Unwraps with the engine's own sentence in the failure message.
#[track_caller]
fn lazim<T, E: core::fmt::Display>(natija: Result<T, E>, ma: &str) -> T {
    match natija {
        Ok(qeema) => qeema,
        Err(khata) => panic!("{ma}: {khata}"),
    }
}

/// Locates the staged Arabic face by walking up from this crate.
fn masar_khatt() -> PathBuf {
    const MURASHAH: &str = "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf";
    let mut dalil: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    while let Some(jidhr) = dalil {
        let masar = jidhr.join(MURASHAH);
        if masar.is_file() {
            return masar;
        }
        dalil = jidhr.parent();
    }
    panic!(
        "{MURASHAH} not found at or above {}",
        env!("CARGO_MANIFEST_DIR")
    );
}

/// Loads the Arabic face through the **validated** constructor.
///
/// [`MawridKhatt::jadeed`], not `jadeed_latini`: every property proved in this
/// file is proved with a font that passed Decision 6's own check, so none of it
/// rests on an escape hatch.
fn khatt() -> Arc<MawridKhatt> {
    let masar = masar_khatt();
    let bayt = lazim(fs::read(&masar), "reading the font");
    assert_eq!(
        bayt.len(),
        TUL_MALAF,
        "{} is not the byte length the hash lock records; the staged font was replaced",
        masar.display()
    );
    Arc::new(lazim(
        MawridKhatt::jadeed(Arc::new(bayt), 0),
        "loading the font",
    ))
}

/// Lays one string out on a single unconstrained line with default options.
fn khattit(nass: &str) -> TakhtitNass {
    let khutut = lazim(SilsilatKhutut::wahid(khatt()), "building the font chain");
    let khiyarat = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();
    lazim(
        saff.khattit(&TalabTakhtit {
            nass,
            khutut: &khutut,
            hajm: HAJM,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &khiyarat,
        }),
        &format!("laying out {nass:?}"),
    )
}

/// Lays one string out with style spans, and without asserting a line count.
fn khattit_bi_nitaqat(nass: &str, nitaqat: &[NitaqUslub]) -> TakhtitNass {
    let khutut = lazim(SilsilatKhutut::wahid(khatt()), "building the font chain");
    let khiyarat = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();
    lazim(
        saff.khattit(&TalabTakhtit {
            nass,
            khutut: &khutut,
            hajm: HAJM,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat,
            khiyarat: &khiyarat,
        }),
        &format!("laying out {nass:?}"),
    )
}

/// Lays one string out over as many lines as its own line separators ask for.
fn khattit_mutaaddid(nass: &str) -> TakhtitNass {
    khattit_bi_nitaqat(nass, &[])
}

/// Asserts that a layout draws no empty box anywhere.
#[track_caller]
fn ahsi_bila_tofu(takhtit: &TakhtitNass, nass: &str) {
    for harf in &takhtit.huruf {
        assert_ne!(
            harf.muarrif, 0,
            "{nass:?} drew .notdef at cluster {} — a tofu box on screen",
            harf.anqud
        );
    }
}

/// Every glyph of a single-line layout, in visual order left to right.
#[track_caller]
fn huruf(takhtit: &TakhtitNass) -> Vec<Harf> {
    assert_eq!(takhtit.sutur.len(), 1, "these tests lay out one line");
    match takhtit.sutur.first() {
        Some(satr) => takhtit.huruf_satr(satr).to_vec(),
        None => Vec::new(),
    }
}

/// The glyph identifiers of a single-line layout, in visual order.
fn muarrifat(nass: &str) -> Vec<u32> {
    huruf(&khattit(nass))
        .iter()
        .map(|harf| harf.muarrif)
        .collect()
}

/// The `GPOS` offsets each glyph carries, recovered by re-walking the pen.
///
/// Returns `(dx, dy)` per glyph in visual order. `dy` is positive upward, which
/// is the font's own convention: [`Harf::a`] is `baseline - dy`.
fn izahat(takhtit: &TakhtitNass) -> Vec<(f32, f32)> {
    let Some(satr) = takhtit.sutur.first() else {
        return Vec::new();
    };
    let mut qalam = satr.bidaya;
    let mut khuruj = Vec::new();
    for harf in takhtit.huruf_satr(satr) {
        khuruj.push((harf.s - qalam, satr.asas - harf.a));
        qalam += harf.taqaddum;
    }
    khuruj
}

/// What a game that never shaped anything draws: one `cmap` lookup per
/// character, in logical order.
///
/// This is the failure this product exists to replace, reproduced here so the
/// tests can assert a difference from it rather than assert a property in the
/// abstract.
fn sadij(nass: &str) -> Vec<u32> {
    let khatt = khatt();
    nass.chars()
        .filter_map(|harf| khatt.muarrif(harf))
        .collect()
}

// ---------------------------------------------------------------------------
// a. Contextual joining
// ---------------------------------------------------------------------------

/// Beh has four contextual shapes and the shaper selects all four.
///
/// `بببب` exercises only three of them — initial, medial, medial, final —
/// because a medial letter run repeats one form; the isolated shape needs a beh
/// with nothing on either side. Both strings are shaped here and the four
/// resulting identifiers are asserted to be four *different* glyphs in the same
/// face.
#[test]
fn contextual_joining_selects_four_distinct_forms_of_beh() {
    let arbaa = muarrifat("بببب");
    assert_eq!(
        arbaa.len(),
        4,
        "four beh must produce four glyphs, got {arbaa:?}"
    );

    // Visual order is left to right and the text is right to left, so the
    // leftmost glyph is the *last* beh.
    let (Some(&nihai), Some(&wasati_a), Some(&wasati_b), Some(&ibtidai)) =
        (arbaa.first(), arbaa.get(1), arbaa.get(2), arbaa.get(3))
    else {
        panic!("four glyphs expected, got {arbaa:?}");
    };

    let munfarid = muarrifat("ب");
    assert_eq!(
        munfarid.len(),
        1,
        "a lone beh must produce one glyph, got {munfarid:?}"
    );
    let Some(&munfarid) = munfarid.first() else {
        panic!("one glyph expected");
    };

    assert_eq!(
        wasati_a, wasati_b,
        "the two interior beh are both medial and must be the same glyph: {arbaa:?}"
    );

    let ashkal: BTreeSet<u32> = [munfarid, ibtidai, wasati_a, nihai].into_iter().collect();
    assert_eq!(
        ashkal.len(),
        4,
        "isolated {munfarid}, initial {ibtidai}, medial {wasati_a} and final {nihai} must be four \
         different glyphs; got {ashkal:?}"
    );

    // The specific failure this test exists to catch: a renderer with no shaper
    // emits the nominal glyph every time.
    assert_ne!(
        ibtidai, munfarid,
        "the first beh took its nominal (isolated) shape — joining did not happen"
    );
    assert_ne!(
        nihai, munfarid,
        "the last beh took its nominal (isolated) shape — joining did not happen"
    );
}

/// The shaped output differs from what an unshaped renderer would draw.
///
/// Stated as a difference rather than as a property, because this is the
/// product's whole claim: the same four characters, the same font, and a
/// different sequence of glyphs.
#[test]
fn shaped_output_differs_from_naive_codepoint_rendering() {
    let ghayr_mashkul = sadij("بببب");
    assert_eq!(
        ghayr_mashkul.len(),
        4,
        "the font maps every beh through cmap"
    );
    let awwal = ghayr_mashkul.first().copied();
    assert!(
        ghayr_mashkul.iter().all(|muarrif| Some(*muarrif) == awwal),
        "an unshaped renderer emits one identifier four times; the harness is wrong if it does \
         not: {ghayr_mashkul:?}"
    );

    let mut mashkul = muarrifat("بببب");
    // Compared as multisets: the naive output is in logical order, the shaped
    // output in visual order, and order is a separate claim tested elsewhere.
    mashkul.sort_unstable();
    let mut sadija = ghayr_mashkul;
    sadija.sort_unstable();
    assert_ne!(
        mashkul, sadija,
        "shaping produced the same glyphs an unshaped renderer would: {mashkul:?}"
    );
}

// ---------------------------------------------------------------------------
// b. Non-joining letters break the run
// ---------------------------------------------------------------------------

/// Alef and dal join only to the right, so the letter after them cannot be
/// medial or final.
///
/// `أدب` is three letters and none of them may join forward: hamza-on-alef does
/// not, dal does not. Every one of the three must therefore take its isolated
/// shape. The control is `بب`, where beh does join forward and the same beh
/// character takes initial and final shapes instead.
#[test]
fn non_joining_letters_break_the_joining_run() {
    let maqtu = muarrifat("أدب");
    assert_eq!(maqtu.len(), 3, "three letters, three glyphs: {maqtu:?}");

    // Visual left to right is logical last to first.
    let (Some(&ba), Some(&dal), Some(&alif)) = (maqtu.first(), maqtu.get(1), maqtu.get(2)) else {
        panic!("three glyphs expected, got {maqtu:?}");
    };

    let munfarid_ba = sadij("ب");
    let munfarid_dal = sadij("د");
    let munfarid_alif = sadij("أ");
    assert_eq!(
        Some(ba),
        munfarid_ba.first().copied(),
        "the beh after a dal must be isolated, not medial or final"
    );
    assert_eq!(
        Some(dal),
        munfarid_dal.first().copied(),
        "the dal after an alef must be isolated"
    );
    assert_eq!(
        Some(alif),
        munfarid_alif.first().copied(),
        "a leading alef must be isolated"
    );

    // The control: the same beh, joined, is neither of those.
    let mawsul = muarrifat("بب");
    assert_eq!(mawsul.len(), 2, "two beh, two glyphs: {mawsul:?}");
    let (Some(&nihai), Some(&ibtidai)) = (mawsul.first(), mawsul.get(1)) else {
        panic!("two glyphs expected, got {mawsul:?}");
    };
    assert_ne!(
        ibtidai, ba,
        "a beh that joins forward must not take the isolated shape"
    );
    assert_ne!(
        nihai, ba,
        "a beh that joins backward must not take the isolated shape"
    );
    assert_ne!(ibtidai, nihai, "initial and final beh are different shapes");
}

// ---------------------------------------------------------------------------
// c. Lam-alef
// ---------------------------------------------------------------------------

/// Lam followed by alef is one glyph, not two.
///
/// Mandatory in Arabic orthography; two separate letterforms side by side is
/// not an alternative spelling, it is wrong. The ligature comes from the font's
/// own `rlig`, never from a substituted presentation-form codepoint.
#[test]
fn lam_alef_ligates_into_one_glyph() {
    const LAM_ALIF: &str = "لا";
    assert_eq!(LAM_ALIF.chars().count(), 2, "the input is two codepoints");

    let takhtit = khattit(LAM_ALIF);
    let mashkul = huruf(&takhtit);
    assert_eq!(
        mashkul.len(),
        1,
        "lam-alef must ligate to one glyph; got {} glyphs: {:?}",
        mashkul.len(),
        mashkul.iter().map(|harf| harf.muarrif).collect::<Vec<_>>()
    );

    let anaqid: BTreeSet<u32> = mashkul.iter().map(|harf| harf.anqud).collect();
    assert!(
        anaqid.len() < LAM_ALIF.chars().count(),
        "the output cluster count {} must be below the input codepoint count {}",
        anaqid.len(),
        LAM_ALIF.chars().count()
    );

    // The ligature is a glyph of its own, not either component reused.
    let mufradat = sadij(LAM_ALIF);
    let Some(&mudmaj) = mashkul.first().map(|harf| &harf.muarrif) else {
        panic!("one glyph expected");
    };
    assert!(
        !mufradat.contains(&mudmaj),
        "the ligature glyph {mudmaj} is one of the nominal glyphs {mufradat:?}; nothing ligated"
    );

    // And an unshaped renderer would have drawn two.
    assert_eq!(
        mufradat.len(),
        2,
        "cmap maps lam and alef to two glyphs: {mufradat:?}"
    );
}

// ---------------------------------------------------------------------------
// d. Right-to-left order
// ---------------------------------------------------------------------------

/// Arabic is laid out right to left: the first character sits at the greatest x.
///
/// Asserted against the convention read out of `natija.rs` and `qiyas.rs` — `s`
/// grows to the right, glyphs are stored in visual order — so right-to-left
/// means the cluster index falls strictly as `s` rises.
#[test]
fn arabic_lays_out_right_to_left() {
    let takhtit = khattit("بببب");
    assert_eq!(
        takhtit.ittijah,
        Ittijah::Yameen,
        "the paragraph must resolve right to left"
    );

    let mashkul = huruf(&takhtit);
    assert!(
        mashkul.len() >= 2,
        "need at least two glyphs to have an order"
    );

    for zawj in mashkul.windows(2) {
        let (Some(sabiq), Some(lahiq)) = (zawj.first(), zawj.get(1)) else {
            continue;
        };
        assert!(
            lahiq.s > sabiq.s,
            "glyphs are stored in visual order, so x must rise: {} then {}",
            sabiq.s,
            lahiq.s
        );
        assert!(
            lahiq.anqud < sabiq.anqud,
            "right-to-left means the cluster index falls as x rises: cluster {} at x {:.3} is \
             followed by cluster {} at x {:.3}",
            sabiq.anqud,
            sabiq.s,
            lahiq.anqud,
            lahiq.s
        );
    }

    // Stated the other way round, because this is the sentence the claim is
    // usually made in: the first character of the string is drawn last.
    let (Some(awwal_basari), Some(akhir_basari)) = (mashkul.first(), mashkul.last()) else {
        panic!("glyphs expected");
    };
    assert_eq!(
        akhir_basari.anqud, 0,
        "the first logical character must be the rightmost glyph"
    );
    assert!(
        akhir_basari.s > awwal_basari.s,
        "the first logical character must sit at the greatest x"
    );
}

/// The control: the same engine lays Latin out left to right.
///
/// Without this, `arabic_lays_out_right_to_left` would also pass on an engine
/// that reversed everything unconditionally.
#[test]
fn latin_control_lays_out_left_to_right() {
    let takhtit = khattit("abcd");
    assert_eq!(
        takhtit.ittijah,
        Ittijah::Yasar,
        "a Latin paragraph must resolve left to right"
    );

    let mashkul = huruf(&takhtit);
    assert_eq!(mashkul.len(), 4, "four letters, four glyphs");
    for zawj in mashkul.windows(2) {
        let (Some(sabiq), Some(lahiq)) = (zawj.first(), zawj.get(1)) else {
            continue;
        };
        assert!(lahiq.s > sabiq.s, "x must rise across the line");
        assert!(
            lahiq.anqud > sabiq.anqud,
            "left-to-right means the cluster index rises with x: {} then {}",
            sabiq.anqud,
            lahiq.anqud
        );
    }
}

// ---------------------------------------------------------------------------
// e. Diacritics
// ---------------------------------------------------------------------------

/// Marks are positioned onto their base, not advanced beside it.
///
/// Every combining mark must carry a zero advance and a non-zero vertical
/// offset. A mark with a real advance is the failure where vocalised Arabic
/// comes out as letters interleaved with floating accents.
#[test]
fn diacritics_are_positioned_not_advanced() {
    const MUSHAKKAL: &str = "مُحَمَّدٌ";

    let takhtit = khattit(MUSHAKKAL);
    let mashkul = huruf(&takhtit);
    let izahat = izahat(&takhtit);
    assert_eq!(mashkul.len(), izahat.len(), "one offset per glyph");

    let alamat: Vec<(usize, &Harf)> = mashkul
        .iter()
        .enumerate()
        .filter(|(_, harf)| harf.alama)
        .collect();
    assert!(
        alamat.len() >= 3,
        "{MUSHAKKAL} carries several marks; the shaper reported {} of them",
        alamat.len()
    );

    for (fahras, alama) in &alamat {
        assert!(
            alama.taqaddum.abs() < f32::EPSILON,
            "mark glyph {} advanced the pen by {}; a mark must not take horizontal room",
            alama.muarrif,
            alama.taqaddum
        );
        let Some(&(_, izaha_a)) = izahat.get(*fahras) else {
            panic!("offset expected for glyph {fahras}");
        };
        assert!(
            izaha_a.abs() > 0.5,
            "mark glyph {} sits {izaha_a} px off the baseline; it was never lifted onto its base",
            alama.muarrif
        );
    }

    // Every mark must share a cluster with a base letter — it is attached to a
    // letter, not standing on its own.
    let qawaid: BTreeSet<u32> = mashkul
        .iter()
        .filter(|harf| !harf.alama)
        .map(|harf| harf.anqud)
        .collect();
    for (_, alama) in &alamat {
        assert!(
            qawaid.contains(&alama.anqud),
            "mark glyph {} is in cluster {} which holds no base letter",
            alama.muarrif,
            alama.anqud
        );
    }
}

/// Diacritics add no width to a line.
///
/// The same word with and without its marks must measure the same, which is the
/// consequence of the zero advance and the reason vocalised text does not
/// overflow a box sized for unvocalised text.
#[test]
fn diacritics_add_no_width() {
    let mashkul = khattit("مُحَمَّدٌ");
    let mujarrad = khattit("محمد");

    assert!(
        mashkul.huruf.len() > mujarrad.huruf.len(),
        "the vocalised form must produce more glyphs: {} vs {}",
        mashkul.huruf.len(),
        mujarrad.huruf.len()
    );
    assert!(
        (mashkul.ard - mujarrad.ard).abs() < 0.01,
        "marks changed the measured width: {} vocalised against {} bare",
        mashkul.ard,
        mujarrad.ard
    );
}

// ---------------------------------------------------------------------------
// f. Bidirectional mixing
// ---------------------------------------------------------------------------

/// A Latin run inside an Arabic line reads left to right while the line reads
/// right to left.
///
/// Game titles, version numbers and key names do this constantly, and it is the
/// case a naive reverse-the-string implementation gets exactly backwards.
#[test]
fn latin_run_stays_left_to_right_inside_an_arabic_line() {
    const NASS: &str = "لعبة Half-Life 2 الشهيرة";
    // Byte offsets of the embedded Latin run, computed rather than counted.
    let bidaya = match NASS.find("Half") {
        Some(mawqi) => u32::try_from(mawqi).unwrap_or(u32::MAX),
        None => panic!("the sample must contain the Latin run"),
    };
    let nihaya = bidaya + 11; // "Half-Life 2"

    let takhtit = khattit(NASS);
    assert_eq!(
        takhtit.ittijah,
        Ittijah::Yameen,
        "the paragraph is Arabic and reads right to left"
    );

    let mashkul = huruf(&takhtit);
    let latini: Vec<&Harf> = mashkul
        .iter()
        .filter(|harf| harf.anqud >= bidaya && harf.anqud < nihaya)
        .collect();
    let arabi: Vec<&Harf> = mashkul
        .iter()
        .filter(|harf| (harf.anqud < bidaya || harf.anqud >= nihaya) && !harf.alama)
        .collect();

    assert!(
        latini.len() >= 10,
        "the Latin run should shape to about eleven glyphs"
    );
    assert!(arabi.len() >= 8, "both Arabic runs should shape");

    // Inside the Latin run: cluster rises with x.
    for zawj in latini.windows(2) {
        let (Some(sabiq), Some(lahiq)) = (zawj.first(), zawj.get(1)) else {
            continue;
        };
        assert!(
            lahiq.anqud > sabiq.anqud,
            "the Latin run must read left to right: cluster {} at x {:.3} then cluster {} at x \
             {:.3}",
            sabiq.anqud,
            sabiq.s,
            lahiq.anqud,
            lahiq.s
        );
    }

    // Outside it: cluster falls with x, in each Arabic run.
    for zawj in arabi.windows(2) {
        let (Some(sabiq), Some(lahiq)) = (zawj.first(), zawj.get(1)) else {
            continue;
        };
        assert!(
            lahiq.anqud < sabiq.anqud,
            "the Arabic runs must read right to left: cluster {} at x {:.3} then cluster {} at x \
             {:.3}",
            sabiq.anqud,
            sabiq.s,
            lahiq.anqud,
            lahiq.s
        );
    }

    // The Latin run is one contiguous block, and it sits between the two Arabic
    // runs rather than at either end.
    let (Some(aqsa_yasar), Some(aqsa_yameen)) = (
        latini.iter().map(|harf| harf.s).reduce(f32::min),
        latini.iter().map(|harf| harf.s).reduce(f32::max),
    ) else {
        panic!("the Latin run produced no glyphs");
    };
    let qabl: Vec<&&Harf> = arabi.iter().filter(|harf| harf.anqud < bidaya).collect();
    let baad: Vec<&&Harf> = arabi.iter().filter(|harf| harf.anqud >= nihaya).collect();
    assert!(
        !qabl.is_empty() && !baad.is_empty(),
        "both Arabic runs must be present"
    );

    for harf in &qabl {
        assert!(
            harf.s > aqsa_yameen,
            "the Arabic that comes first logically must be drawn to the right of the Latin run: \
             cluster {} at x {:.3} against the run's right edge {aqsa_yameen:.3}",
            harf.anqud,
            harf.s
        );
    }
    for harf in &baad {
        assert!(
            harf.s < aqsa_yasar,
            "the Arabic that comes last logically must be drawn to the left of the Latin run: \
             cluster {} at x {:.3} against the run's left edge {aqsa_yasar:.3}",
            harf.anqud,
            harf.s
        );
    }
}

// ---------------------------------------------------------------------------
// g. No tofu
// ---------------------------------------------------------------------------

/// Every sample in this file shapes without producing a single `.notdef`.
///
/// Glyph zero is the empty box a player sees when a font could not draw a
/// character. It is what a failure looks like on screen, so it is asserted
/// across every string the rest of the file uses rather than in one place.
#[test]
fn no_glyph_is_notdef() {
    const AYINAT: &[&str] = &[
        "مرحبًا بالعالم من محرك تعريب",
        "بببب",
        "ب",
        "بب",
        "لا",
        "أدب",
        "مُحَمَّدٌ",
        "محمد",
        "لعبة Half-Life 2 الشهيرة",
        "abcd",
        "الإصدار ٢٫٥ من اللعبة",
    ];

    for nass in AYINAT {
        let takhtit = khattit(nass);
        assert!(
            !takhtit.huruf.is_empty(),
            "{nass:?} produced no glyphs at all"
        );
        for harf in &takhtit.huruf {
            assert_ne!(
                harf.muarrif, 0,
                "{nass:?} produced .notdef at cluster {} — a tofu box on screen",
                harf.anqud
            );
        }
    }
}

/// A line separator is an instruction, and an instruction is not drawn.
///
/// This is the defect the owner photographed on a world-space monitor in The
/// Stalked 3: a page of Arabic paragraphs with an empty rectangle at the end of
/// several sentences. Every one of them was a `\n`. A font has no glyph for a
/// line feed, so `cmap` answers zero, and glyph zero is the box — which then
/// also charged each line the box's own advance, so every line that ended in a
/// newline measured wider than the text on it.
#[test]
fn a_line_separator_draws_no_glyph() {
    const NASS: &str = "هل أنا مراقب؟\nبعد الآن.";
    let takhtit = khattit_mutaaddid(NASS);

    assert_eq!(
        takhtit.sutur.len(),
        2,
        "the newline must end the first line: {:?}",
        takhtit.sutur.len()
    );
    ahsi_bila_tofu(&takhtit, NASS);
    assert_eq!(
        takhtit.taghtiya_naqisa, None,
        "a newline is not a coverage hole and must not be reported as one"
    );

    // Nothing in the layout points at the newline: it produced no glyph, so no
    // glyph names its cluster.
    let mawqi_satr = u32::try_from(
        NASS.find('\n')
            .unwrap_or_else(|| panic!("the sample contains a newline")),
    )
    .unwrap_or(u32::MAX);
    for harf in &takhtit.huruf {
        assert_ne!(
            harf.anqud, mawqi_satr,
            "a glyph was drawn for the line separator at byte {mawqi_satr}"
        );
    }
}

/// A blank line between two paragraphs holds nothing at all.
///
/// The second half of the same photograph: boxes sitting alone on their own
/// lines, where a blank line should be. A paragraph break is two line
/// separators, and the second one had a whole line to itself to put its box on.
#[test]
fn a_blank_line_holds_no_glyph() {
    const NASS: &str = "أو من يقف وراءه.\n\nبعد الآن.";
    let takhtit = khattit_mutaaddid(NASS);

    assert_eq!(
        takhtit.sutur.len(),
        3,
        "two separators make three lines, the middle one blank"
    );
    ahsi_bila_tofu(&takhtit, NASS);

    let Some(wasat) = takhtit.sutur.get(1) else {
        panic!("three lines expected");
    };
    assert!(
        takhtit.huruf_satr(wasat).is_empty(),
        "the blank line between two paragraphs holds {} glyph(s)",
        takhtit.huruf_satr(wasat).len()
    );
}

/// An atom occupies a position in the text and never becomes a glyph.
///
/// `nasq` represents every inline object — a sprite, a `{0}`, a runtime
/// variable — as one U+FFFC carried by a style span that says it is an atom.
/// The span is what keeps the placeholder out of shaping; this asserts the
/// contract from the other end, that the position it reserves is a hole of the
/// declared width and not a letterform.
#[test]
fn an_atom_draws_no_glyph() {
    const NASS: &str = "الذهب: \u{FFFC} قطعة";
    const ARD_DHARRA: f32 = 30.0;
    let Some(mawqi) = NASS.find('\u{FFFC}') else {
        panic!("the sample contains an atom");
    };
    let bidaya = u32::try_from(mawqi).unwrap_or(u32::MAX);

    let takhtit = khattit_bi_nitaqat(
        NASS,
        &[NitaqUslub {
            id: 1,
            bidaya,
            tul: u32::try_from('\u{FFFC}'.len_utf8()).unwrap_or(0),
            uslub: Uslub {
                dharra: Some(Dharra {
                    ard: ARD_DHARRA,
                    irtifa: 0.0,
                    asas: 0.0,
                    marja: 0,
                }),
                ..Uslub::default()
            },
        }],
    );

    ahsi_bila_tofu(&takhtit, NASS);
    assert_eq!(
        takhtit.taghtiya_naqisa, None,
        "an atom is not a character the font failed to cover"
    );
    for harf in &takhtit.huruf {
        assert_ne!(
            harf.anqud, bidaya,
            "a glyph was drawn for the atom at byte {bidaya}"
        );
    }

    // The width it reserved is still in the line, because the game will put its
    // sprite or its substituted value there.
    let bila = khattit_mutaaddid("الذهب:  قطعة");
    assert!(
        takhtit.ard - bila.ard > ARD_DHARRA * 0.5,
        "the atom reserved no room: {:.3} against {:.3} without it",
        takhtit.ard,
        bila.ard
    );
}

/// A character no font in the chain can draw is reported, not boxed.
///
/// The honest answers to a coverage hole are to fall back, to draw nothing, or
/// to say so. Falling back has already been tried and failed by the time a
/// `.notdef` exists — the chain picks the first font that covers each character
/// — so what is left is the other two, and this asserts both: the space is
/// reserved and left empty, and the layout says how many there were and where
/// the first one is.
#[test]
fn an_uncovered_character_is_reported_not_drawn() {
    // A CJK ideograph and an object replacement character with no atom span to
    // carry it. Neither is in an Arabic text face, and the second is exactly
    // what a patch produces when a placeholder's span fails to reach layout.
    for (nass, ghayr) in [("قبل 中 بعد", '\u{4E2D}'), ("قبل \u{FFFC} بعد", '\u{FFFC}')]
    {
        let khatt = khatt();
        assert!(
            !khatt.yughatti(ghayr),
            "this test needs a character the staged face does not cover; it covers U+{:04X}",
            u32::from(ghayr)
        );

        let takhtit = khattit_mutaaddid(nass);
        ahsi_bila_tofu(&takhtit, nass);

        let Some(naqisa) = takhtit.taghtiya_naqisa else {
            panic!(
                "{nass:?} holds U+{:04X}, which no font covers, and the layout reported no missing coverage",
                u32::from(ghayr)
            );
        };
        assert_eq!(
            naqisa.adad, 1,
            "one character is uncoverable in {nass:?}, not {}",
            naqisa.adad
        );
        let mawqi = usize::try_from(naqisa.awwal_anqud).unwrap_or(usize::MAX);
        assert_eq!(
            nass.get(mawqi..).and_then(|baqi| baqi.chars().next()),
            Some(ghayr),
            "the reported offset {mawqi} must name the character the chain could not draw"
        );

        // The room is still reserved: the line is wider than the same sentence
        // with the character taken out.
        let bila = khattit_mutaaddid(&nass.replace(ghayr, ""));
        assert!(
            takhtit.ard > bila.ard,
            "the uncovered character reserved no room: {:.3} against {:.3} without it",
            takhtit.ard,
            bila.ard
        );
    }
}

// ---------------------------------------------------------------------------
// Decisions 3 and 4: the shaper and the rasterizer agree
// ---------------------------------------------------------------------------

/// Every glyph identifier shaping produced resolves to an outline.
///
/// This is the seam where a text stack that uses one library to shape and
/// another to draw comes apart: an identifier from shaper A means a different
/// glyph in rasterizer B, and the page renders as garbage that looks like a
/// font bug. Asserted here because it is cheap to assert and expensive to
/// discover in a game.
#[test]
fn every_shaped_glyph_resolves_to_an_outline() {
    let khatt = khatt();
    let rassam = lazim(Rassam::jadeed(&khatt), "binding the rasterizer");
    let takhtit = khattit("مرحبًا بالعالم من محرك تعريب");

    let mut marsuma = 0_u32;
    for harf in &takhtit.huruf {
        assert!(
            harf.muarrif < rassam.adad_ashkal(),
            "glyph {} is past the {} glyphs this face has",
            harf.muarrif,
            rassam.adad_ashkal()
        );
        let surah = lazim(
            rassam.irsim(harf.muarrif, HAJM, NamatRasm::Taghtiya, 0.0, &[]),
            &format!("rasterizing glyph {}", harf.muarrif),
        );
        if !surah.khali() {
            marsuma += 1;
            assert_eq!(
                surah.bayt.len(),
                (surah.ard as usize) * (surah.irtifa as usize),
                "glyph {} returned a bitmap that is not its own size",
                harf.muarrif
            );
            assert!(
                surah.bayt.iter().any(|taghtiya| *taghtiya > 0),
                "glyph {} rasterized to an entirely blank bitmap of {}x{}",
                harf.muarrif,
                surah.ard,
                surah.irtifa
            );
        }
    }
    assert!(
        marsuma >= 20,
        "only {marsuma} glyphs of the sentence produced ink"
    );
}

// ---------------------------------------------------------------------------
// Decision 6: the validator accepts the font this product ships
// ---------------------------------------------------------------------------

/// The face this repository pins and ships passes its own Arabic validation.
///
/// This is a regression guard on a defect that was live in this tree earlier
/// today. `SIFAT_GSUB_MATLUBA` (`crates/taarib-saff/src/khatt.rs`) used to
/// require an `isol` tag in the font's `GSUB` feature list, and `fahs_arabi`
/// refused any font without one. IBM Plex Sans Arabic **has no `isol`** — the
/// test below reads its feature list out of the raw bytes and proves it — and
/// was therefore rejected by the very product that ships it, while shaping,
/// joining, ligating and positioning marks perfectly the whole time.
///
/// A substitution feature exists to replace the nominal glyph. For an Arabic
/// letter the nominal glyph a `cmap` yields already *is* the isolated form, so
/// a font declares `isol` only when its isolated form differs from that
/// default. Requiring the tag tested an encoding choice, not a capability.
///
/// The assertions here are the two halves of that: the font declares no `isol`,
/// and the validator accepts it anyway.
#[test]
fn shipped_font_passes_arabic_validation_without_declaring_isol() {
    let masar = masar_khatt();
    let bayt = lazim(fs::read(&masar), "reading the font");

    let sifat = sifat_gsub(&bayt);
    assert!(
        !sifat.is_empty(),
        "the font must declare some GSUB features; read {sifat:?}"
    );
    for matlub in ["init", "medi", "fina", "rlig"] {
        assert!(
            sifat.contains(matlub),
            "this font is expected to declare {matlub}; it declares {sifat:?}"
        );
    }
    assert!(
        !sifat.contains("isol"),
        "IBM Plex Sans Arabic was expected to declare no isol; it declares {sifat:?}. If the font \
         was replaced, this regression guard no longer guards anything"
    );

    let mawrid = lazim(
        MawridKhatt::jadeed(Arc::new(bayt), 0),
        "Decision 6 validation rejected the font this product ships",
    );
    assert_eq!(
        mawrid.aila(),
        "IBM Plex Sans Arabic",
        "the face is what it says it is"
    );
    lazim(mawrid.fahs_arabi(), "the Arabic table and coverage check");
    lazim(
        mawrid.fahs_taghtiya("مرحبًا بالعالم لا إله إلا الله"),
        "the coverage check over real Arabic text",
    );
}

/// The `GSUB` feature tags a font declares, read straight out of the file.
///
/// A deliberately independent reading: the point of the test above is to check
/// the engine's verdict against the font's actual contents, and asking the
/// engine what the font contains would make the check circular. This walks the
/// sfnt table directory to `GSUB`, then its `FeatureList`, and collects the
/// four-byte tags. Malformed input yields an empty set rather than a panic.
fn sifat_gsub(bayt: &[u8]) -> BTreeSet<String> {
    /// Reads a big-endian `u16` at a byte offset.
    fn iqra16(bayt: &[u8], mawqi: usize) -> Option<usize> {
        let zawj = bayt.get(mawqi..mawqi.checked_add(2)?)?;
        Some(usize::from(u16::from_be_bytes([
            *zawj.first()?,
            *zawj.get(1)?,
        ])))
    }
    /// Reads a big-endian `u32` at a byte offset.
    fn iqra32(bayt: &[u8], mawqi: usize) -> Option<usize> {
        let arbaa = bayt.get(mawqi..mawqi.checked_add(4)?)?;
        let mut thabit = [0_u8; 4];
        thabit.copy_from_slice(arbaa);
        usize::try_from(u32::from_be_bytes(thabit)).ok()
    }

    let mut sifat = BTreeSet::new();
    let Some(adad_jadawil) = iqra16(bayt, 4) else {
        return sifat;
    };

    let mut gsub: Option<usize> = None;
    for fahras in 0..adad_jadawil {
        let sijil = 12 + fahras * 16;
        if bayt.get(sijil..sijil + 4) == Some(b"GSUB") {
            gsub = iqra32(bayt, sijil + 8);
            break;
        }
    }
    let Some(gsub) = gsub else {
        return sifat;
    };

    // GSUB header: major u16, minor u16, scriptList Offset16, featureList
    // Offset16, lookupList Offset16.
    let Some(izahat_qaima) = iqra16(bayt, gsub + 6) else {
        return sifat;
    };
    let qaima = gsub + izahat_qaima;
    let Some(adad_sifat) = iqra16(bayt, qaima) else {
        return sifat;
    };

    for fahras in 0..adad_sifat {
        let sijil = qaima + 2 + fahras * 6;
        if let Some(wasm) = bayt.get(sijil..sijil + 4) {
            sifat.insert(String::from_utf8_lossy(wasm).into_owned());
        }
    }
    sifat
}
