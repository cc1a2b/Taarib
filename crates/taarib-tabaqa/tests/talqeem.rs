//! What the feeder must produce, asserted against the batch it actually builds.
//!
//! Every test here failed on the code as it stood before Phase 28 Stage 5, and
//! each one names a defect that was invisible in review and obvious the first
//! time anything drew:
//!
//! * [`lawha_madruba_musbaqan`] — the atlas page was expanded to white with the
//!   coverage in alpha, which multiplied against a premultiplied quad colour
//!   leaves every colour channel at full strength across the glyph's whole
//!   bitmap. Each letter drew as a solid rectangle.
//! * [`lawh_yughatti_almustatil`] — the plate was placed from a pen the glyph
//!   path treated as the layout's top and the plate path treated as a baseline,
//!   and it was sized from the layout's width while the content was
//!   right-aligned inside a wider box. It landed above and to the left of the
//!   text it was meant to sit behind.
//! * [`nass_arabi_yalzam_alyameen`] — the whole point of the tier. Arabic
//!   replacing a line of English has to start at the right of the rectangle the
//!   English occupied.
//!
//! # The font
//!
//! IBM Plex Sans Arabic Regular, staged into `apps/studio/src/khutut/`. Nothing
//! here downloads anything.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_saff::{MawridKhatt, SilsilatKhutut};
use taarib_tabaqa::rasm_tabaqa::ila_rgba;
use taarib_tabaqa::talqeem::{KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
use taarib_tabaqa::wajiha::{LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WasfSath};

/// The surface every test builds against.
const SATH: WasfSath =
    WasfSath { ard: 1280, irtifa: 720, sigha: SighatSath::Rgba8, sirgb: true };

/// A line of English a recognizer might have read, and where.
const SUNDUQ: MustatilBiksel = MustatilBiksel { yasar: 200, aala: 480, ard: 420, irtifa: 22 };

/// The Arabic that replaces it, in logical order and ordinary Unicode.
const ARABI: &str = "الطريق الشمالي مغلق حتى ذوبان الثلج.";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The staged Arabic face.
fn khutut() -> SilsilatKhutut {
    const MURASHAHAT: &[&str] = &[
        "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf",
        "target/tajmee/makhbaa/sans/IBMPlexSansArabic-Regular.ttf",
        "assets/fonts/IBMPlexSansArabic-Regular.ttf",
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
        panic!("no Arabic font found at or above {}", env!("CARGO_MANIFEST_DIR"));
    };
    let Ok(bayt) = fs::read(&masar) else {
        panic!("{} could not be read", masar.display());
    };
    let Ok(khatt) = MawridKhatt::jadeed(Arc::new(bayt), 0) else {
        panic!("{} did not pass Decision 6 validation", masar.display());
    };
    match SilsilatKhutut::wahid(Arc::new(khatt)) {
        Ok(silsila) => silsila,
        Err(khata) => panic!("the font chain would not build: {khata}"),
    }
}

/// One frame's batch for a single translated line.
fn dufaa_satr() -> LawhatRasm {
    let mut mulaqqim = match Mulaqqim::jadeed(khutut(), KhiyaratTalqeem::iftiradiya()) {
        Ok(mulaqqim) => mulaqqim,
        Err(khata) => panic!("the feeder would not build: {khata}"),
    };
    let sutur =
        vec![SatrMulaqqam { nass: ARABI.to_owned(), mawdi: SUNDUQ, thiqa: 94 }];
    match mulaqqim.ibni(SATH, &sutur, &[]) {
        Ok(dufaa) => {
            let ihsaat = mulaqqim.ihsaat();
            assert_eq!(
                ihsaat.ashkal_mafquda, 0,
                "the atlas refused {} glyph(s); last reason: {}",
                ihsaat.ashkal_mafquda,
                ihsaat.akhir_radd.as_deref().unwrap_or("—")
            );
            dufaa
        },
        Err(khata) => panic!("the batch would not build: {khata}"),
    }
}

/// The quads that sample the atlas.
fn ashkal(dufaa: &LawhatRasm) -> Vec<&QitaRasm> {
    dufaa.qitaat.iter().filter(|qita| qita.khareeta.is_some()).collect()
}

/// The quads that do not.
fn alwah(dufaa: &LawhatRasm) -> Vec<&QitaRasm> {
    dufaa.qitaat.iter().filter(|qita| qita.khareeta.is_none()).collect()
}

// ---------------------------------------------------------------------------
// The atlas reaches the backend premultiplied
// ---------------------------------------------------------------------------

/// Every channel of an expanded texel carries the coverage, not just alpha.
///
/// The backends compute `texel * quad_colour` with a premultiplied quad colour
/// and blend `ONE, INV_SRC_ALPHA`. A texel of `(1, 1, 1, c)` leaves the colour
/// channels unweighted, so a half-covered pixel at the edge of a letter is drawn
/// at full colour with half alpha — which over a whole bitmap is a filled
/// rectangle rather than a glyph.
#[test]
fn lawha_madruba_musbaqan() {
    let taghtiya: [u8; 4] = [0, 64, 128, 255];
    let rgba = match ila_rgba(&taghtiya, 4, 1) {
        Ok(rgba) => rgba,
        Err(khata) => panic!("the page would not expand: {khata}"),
    };
    assert_eq!(rgba.len(), taghtiya.len() * 4, "one texel must become four bytes");
    for (fahras, mutawaqqa) in taghtiya.iter().enumerate() {
        let Some(texel) = rgba.get(fahras * 4..fahras * 4 + 4) else {
            panic!("texel {fahras} is missing from the expansion");
        };
        assert_eq!(
            texel,
            [*mutawaqqa; 4],
            "texel {fahras} of coverage {mutawaqqa} expanded to {texel:?}; a premultiplied page \
             carries the coverage in all four channels"
        );
    }
}

// ---------------------------------------------------------------------------
// The plate sits behind the text and over what it replaces
// ---------------------------------------------------------------------------

/// The plate covers the recognized rectangle **and** every glyph drawn into it.
///
/// Both halves matter and they failed for different reasons. Covering the
/// rectangle is what stops the original English showing beside a shorter
/// translation. Covering the glyphs is what says the plate and the glyphs were
/// placed from the same pen with the same meaning — the disagreement that put
/// one an ascent above the other.
#[test]
fn lawh_yughatti_almustatil() {
    let dufaa = dufaa_satr();
    let alwah = alwah(&dufaa);
    assert_eq!(alwah.len(), 1, "one translated line emits exactly one plate, got {}", alwah.len());
    let Some(lawh) = alwah.first() else {
        panic!("the batch carried no plate");
    };
    let hudud = |qita: &QitaRasm| {
        (
            qita.mawdi.yasar,
            qita.mawdi.aala,
            qita.mawdi.yasar + qita.mawdi.ard,
            qita.mawdi.aala + qita.mawdi.irtifa,
        )
    };
    let (l_yasar, l_aala, l_yameen, l_asfal) = hudud(lawh);

    assert!(
        l_yasar <= SUNDUQ.yasar
            && l_aala <= SUNDUQ.aala
            && l_yameen >= SUNDUQ.yasar + SUNDUQ.ard
            && l_asfal >= SUNDUQ.aala + SUNDUQ.irtifa,
        "the plate {:?} does not cover the rectangle it replaces {SUNDUQ:?}",
        lawh.mawdi
    );

    let ashkal = ashkal(&dufaa);
    assert!(!ashkal.is_empty(), "the line produced no glyph quads at all");
    for shakl in &ashkal {
        let (yasar, aala, yameen, asfal) = hudud(shakl);
        assert!(
            yasar >= l_yasar && aala >= l_aala && yameen <= l_yameen && asfal <= l_asfal,
            "glyph {:?} falls outside its own plate {:?}",
            shakl.mawdi,
            lawh.mawdi
        );
    }

    // The plate is emitted first: submission order is the layering and there is
    // no depth buffer, so a plate after its glyphs is a plate over them.
    let Some(awwal) = dufaa.qitaat.first() else {
        panic!("the batch was empty");
    };
    assert!(awwal.khareeta.is_none(), "the first quad must be the plate, not a glyph");
}

// ---------------------------------------------------------------------------
// Arabic runs right to left inside the rectangle it replaces
// ---------------------------------------------------------------------------

/// The line hugs the right edge of its box, and there is room left over on the
/// left.
///
/// This is the property the whole tier exists for. Arabic replacing English in
/// the English's own rectangle begins where Arabic begins — at the right — and a
/// translation shorter than what it replaces leaves its slack on the left. A
/// renderer that laid the text out left to right passes every other test in this
/// file and fails this one.
#[test]
fn nass_arabi_yalzam_alyameen() {
    let dufaa = dufaa_satr();
    let ashkal = ashkal(&dufaa);
    assert!(!ashkal.is_empty(), "the line produced no glyph quads at all");

    let aqsa = ashkal.iter().map(|qita| qita.mawdi.yasar + qita.mawdi.ard).max().unwrap_or(0);
    let adna = ashkal.iter().map(|qita| qita.mawdi.yasar).min().unwrap_or(0);
    let yameen_sunduq = SUNDUQ.yasar + SUNDUQ.ard;

    // Within a few pixels of the right edge: the last glyph's own right side
    // bearing is real and is not slack.
    assert!(
        aqsa + 8 >= yameen_sunduq && aqsa <= yameen_sunduq + 8,
        "the text's right edge is {aqsa}, and the rectangle's is {yameen_sunduq}; Arabic must \
         start at the right of the box it replaces"
    );
    assert!(
        adna > SUNDUQ.yasar,
        "the text spans the whole box from {adna}; this translation is shorter than the line it \
         replaces, so the slack belongs on the left"
    );
}
