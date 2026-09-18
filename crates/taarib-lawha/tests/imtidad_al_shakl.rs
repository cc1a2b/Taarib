//! امتداد الشكل — the rectangle the atlas hands back holds the whole glyph.
//!
//! The defect these tests exist to catch is a glyph drawn with its lower band
//! missing: a horizontal cut through every letter of a line, word shapes still
//! recognisable, the text no longer readable. Every mechanism that can produce
//! it lives between the rasterizer and the glyph map, and each one is checked
//! here against the real Arabic face rather than against a fabricated bitmap.
//!
//!   - **The rectangle is shorter than the bitmap.** The packer would then hold
//!     only the top rows and the map would still report the full height, so the
//!     quad's lower band samples texels no glyph was written into.
//!   - **The page was cropped through a shelf.** [`taarib_lawha::Lawha::ibni`]
//!     finishes a page at the rows the pack reached; a crop measured from the
//!     wrong number cuts the bottom off every glyph on the last shelf.
//!   - **A rectangle moved under a glyph that is still mapped.** The runtime
//!     atlas evicts by rectangle and reuses the space; a glyph whose map entry
//!     survived its eviction would draw from whatever was written there next.
//!
//! The assertion in every case is the same and it is the strongest one
//! available: the texels inside a glyph's mapped rectangle must equal, byte for
//! byte, the bitmap the rasterizer produces for that key. Anything less — a
//! bounds check, an ink probe on the last row — passes for a glyph whose bottom
//! row happens to be blank, which is most of them.
//!
//! # The font
//!
//! IBM Plex Sans Arabic Regular, staged into `apps/studio/src/khutut/`. Nothing
//! here downloads anything and no page is fabricated: every texel measured was
//! rasterized by the same code a compile and a game both run.

#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_lawha::khareeta::{MawdiShakl, MiftahShakl, NamatSafha};
use taarib_lawha::misafa::KhiyaratMisafa;
use taarib_lawha::rasf::Safha;
use taarib_lawha::{JamiAshkal, KhiyaratRasf, Lawha, LawhaHayya};
use taarib_saff::rasm::{MAWADI_TAHAZZUZ, NamatRasm, Rassam, SurahHarf};
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, Saff, SilsilatKhutut, TakhtitNass, TalabTakhtit};

/// A subtitle's worth of Arabic: joined forms, a lam-alef ligature, marks, a
/// deep-descending final ya, and the digits a line of dialogue usually carries.
const JUMLA: &str = "لا تقترب من الباب، فالحارس ينظر إليك الآن — ٣ دقائق فقط";

/// The layout size, in pixels. A subtitle's size, not a heading's.
const HAJM: f32 = 28.0;

/// The rungs [`Nasij.HajmLawhaMulaim`] quantizes an on-screen size to. The C#
/// ladder and this list are the same ten numbers; a runtime atlas now meets
/// several of them in one session, which is what made eviction and page growth
/// ordinary rather than rare.
const SALALIM: [f32; 10] = [8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 192.0];

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// The staged Arabic face, found by walking up from this crate.
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

/// The font resource, shared by the chain and the rasterizer.
fn khatt() -> Arc<MawridKhatt> {
    let masar = masar_khatt();
    let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
    Arc::new(
        MawridKhatt::jadeed(Arc::new(bayt), 0)
            .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display())),
    )
}

/// A one-font chain over the staged face.
fn silsila() -> SilsilatKhutut {
    SilsilatKhutut::wahid(khatt()).expect("a one-font chain")
}

/// Shapes the sentence on one unconstrained line at a size.
fn khattit(khutut: &SilsilatKhutut, hajm: f32) -> TakhtitNass {
    let khiyarat = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();
    saff.khattit(&TalabTakhtit {
        nass: JUMLA,
        khutut,
        hajm,
        ard_mutah: None,
        irtifa_mutah: None,
        nitaqat: &[],
        khiyarat: &khiyarat,
    })
    .unwrap_or_else(|khata| panic!("laying {JUMLA:?} out at {hajm}px: {khata}"))
}

/// The glyph keys one shaped line asks the atlas for, at one rasterization
/// size.
///
/// Collected through [`JamiAshkal`] rather than by walking the layout by hand,
/// because the collector is what a compile uses and the subpixel bucket has to
/// come from the same place in both.
fn mafatih(khutut: &SilsilatKhutut, hajm_takhtit: f32, hajm_lawha: f32) -> Vec<MiftahShakl> {
    let takhtit = khattit(khutut, hajm_takhtit);
    let mut jami = JamiAshkal::jadeed(NamatSafha::Taghtiya);
    // The atlas is keyed by the rasterization size, which the ladder separates
    // from the layout size; the buckets still come from the layout's own
    // fractional pen positions.
    for harf in &takhtit.huruf {
        jami.idif_shakl(MiftahShakl::jadeed(
            harf.khatt,
            harf.muarrif,
            hajm_lawha,
            NamatSafha::Taghtiya,
            taarib_saff::rasm::bakat_tahazzuz(harf.s),
        ));
    }
    jami.ashkal()
}

/// The bitmap the rasterizer produces for one key, which is what the page must
/// hold at that key's rectangle.
fn surah(rassam: &Rassam, miftah: MiftahShakl) -> SurahHarf {
    let tahazzuz =
        f32::from(miftah.bakat.min(MAWADI_TAHAZZUZ.saturating_sub(1))) / f32::from(MAWADI_TAHAZZUZ);
    rassam
        .irsim(
            miftah.muarrif,
            miftah.hajm(),
            NamatRasm::Taghtiya,
            tahazzuz,
            &[],
        )
        .unwrap_or_else(|khata| panic!("rasterizing glyph {}: {khata}", miftah.muarrif))
}

/// Compares one glyph's mapped rectangle against the bitmap it was made from,
/// row by row, and reports the first row that disagrees.
///
/// The row index is in the report because "the bitmap does not match" and "the
/// bitmap matches down to row nine and is zero below it" are different defects,
/// and only the second one is the cut this file is named for.
#[track_caller]
fn tahaqquq(
    safahat: &[Safha],
    miftah: MiftahShakl,
    mawdi: &MawdiShakl,
    surah: &SurahHarf,
    ayn: &str,
) {
    if surah.khali() {
        assert!(
            mawdi.khali(),
            "{ayn}: glyph {} at {}px draws nothing, and the map gives it a {}x{} rectangle",
            miftah.muarrif,
            miftah.hajm(),
            mawdi.ard,
            mawdi.irtifa
        );
        return;
    }

    assert_eq!(
        (u32::from(mawdi.ard), u32::from(mawdi.irtifa)),
        (surah.ard, surah.irtifa),
        "{ayn}: glyph {} at {}px rasterizes to {}x{} and the map reports {}x{}",
        miftah.muarrif,
        miftah.hajm(),
        surah.ard,
        surah.irtifa,
        mawdi.ard,
        mawdi.irtifa
    );

    let Some(safha) = safahat.get(usize::from(mawdi.safha)) else {
        panic!(
            "{ayn}: glyph {} names page {} and the atlas has {}",
            miftah.muarrif,
            mawdi.safha,
            safahat.len()
        );
    };
    assert!(
        u32::from(mawdi.a) + u32::from(mawdi.irtifa) <= u32::from(safha.irtifa)
            && u32::from(mawdi.s) + u32::from(mawdi.ard) <= u32::from(safha.ard),
        "{ayn}: glyph {} sits at {}+{} by {}+{} in a page of {}x{} — the rectangle runs off \
         the page, so its lower band is rows the texture does not have",
        miftah.muarrif,
        mawdi.s,
        mawdi.ard,
        mawdi.a,
        mawdi.irtifa,
        safha.ard,
        safha.irtifa
    );

    let khatwa = usize::from(safha.ard);
    for satr in 0..u32::from(mawdi.irtifa) {
        let mutawaqqa = surah.satr(satr).unwrap_or_else(|| {
            panic!(
                "{ayn}: the bitmap for glyph {} has no row {satr} of {}",
                miftah.muarrif, surah.irtifa
            )
        });
        let bidaya = (usize::from(mawdi.a) + usize::try_from(satr).unwrap_or(0)) * khatwa
            + usize::from(mawdi.s);
        let fili = safha
            .bayt
            .get(bidaya..bidaya + usize::from(mawdi.ard))
            .unwrap_or_else(|| {
                panic!(
                    "{ayn}: row {satr} of glyph {} falls outside a page of {} byte(s)",
                    miftah.muarrif,
                    safha.bayt.len()
                )
            });
        assert_eq!(
            fili,
            mutawaqqa,
            "{ayn}: glyph {} at {}px, page {}, rectangle {}x{} at ({}, {}) — row {satr} of {} \
             differs from the bitmap it was drawn from{}",
            miftah.muarrif,
            miftah.hajm(),
            mawdi.safha,
            mawdi.ard,
            mawdi.irtifa,
            mawdi.s,
            mawdi.a,
            mawdi.irtifa,
            if fili.iter().all(|b| *b == 0) {
                "; the page holds zeros there, which is the bottom band of the letter missing"
            } else {
                ""
            }
        );
    }
}

// ---------------------------------------------------------------------------
// The compiled atlas
// ---------------------------------------------------------------------------

/// Every glyph of a compiled page holds the bitmap it was made from, in full.
///
/// The crop that finishes a page is the mechanism under test: it is measured
/// from the allocator's own bottom-most row, and a crop measured from anything
/// else would take the last shelf's lower rows with it.
#[test]
fn ikhtibar_al_lawha_al_mabniya_tahfaz_kull_satr() {
    let khutut = silsila();
    let rassam = Rassam::jadeed(&khatt()).expect("a rasterizer over the staged face");

    for hajm in SALALIM {
        let ashkal = mafatih(&khutut, HAJM, hajm);
        assert!(
            !ashkal.is_empty(),
            "the sentence shapes to glyphs at {hajm}px"
        );

        let mabniya = Lawha::ibni(
            &ashkal,
            &khutut,
            KhiyaratRasf::default(),
            KhiyaratMisafa::default(),
            NamatSafha::Taghtiya,
        )
        .unwrap_or_else(|khata| panic!("packing the sentence at {hajm}px: {khata}"));

        for miftah in &ashkal {
            let Some(mawdi) = mabniya.khareeta.mawdi(*miftah) else {
                panic!(
                    "glyph {} at {hajm}px was packed and is not mapped",
                    miftah.muarrif
                )
            };
            tahaqquq(
                &mabniya.safahat,
                *miftah,
                mawdi,
                &surah(&rassam, *miftah),
                &format!("compiled at {hajm}px"),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The runtime atlas
// ---------------------------------------------------------------------------

/// Every glyph the runtime atlas hands out inside one frame still holds its own
/// bitmap at the end of that frame.
///
/// This is the pinning contract stated as an assertion. The atlas is given a
/// budget far below the working set and asked for the whole sentence at every
/// rung of the ladder, so eviction runs constantly — which is what the ladder
/// made ordinary, because a size that used to be one is now as many as the
/// camera and the canvas scaler between them produce.
#[test]
fn ikhtibar_al_lawha_al_hayya_la_tunqus_shaklan_mathbutan() {
    let khutut = silsila();
    let rassam = Rassam::jadeed(&khatt()).expect("a rasterizer over the staged face");

    let khiyarat = KhiyaratRasf {
        // Small pages, so the sentence at the upper rungs needs several of them
        // and the packer has to grow inside a frame.
        aqsa_ard: 256,
        aqsa_irtifa: 256,
        ..KhiyaratRasf::default()
    };
    // Four pages of 256 square: enough to draw a subtitle, far too little to
    // hold ten sizes of one, so the evictor runs on nearly every frame.
    let mut hayya = LawhaHayya::jadeeda(khiyarat, NamatSafha::Taghtiya, 4 * 256 * 256)
        .expect("a runtime atlas inside a four-page budget");

    for hajm in SALALIM {
        let ashkal = mafatih(&khutut, HAJM, hajm);

        hayya.ibda_itar();
        let mut mawaqi: Vec<(MiftahShakl, MawdiShakl)> = Vec::with_capacity(ashkal.len());
        for miftah in &ashkal {
            // A budget this small genuinely runs out of unpinned rectangles,
            // and refusing is the documented answer. What must never happen is
            // a glyph handed out and then moved, so a refusal is passed over
            // and everything that did come back is checked below.
            if let Ok(mawdi) = hayya.shakl_min_silsila(*miftah, &khutut) {
                mawaqi.push((*miftah, mawdi));
            }
        }
        assert!(
            !mawaqi.is_empty(),
            "the atlas handed out nothing at all at {hajm}px"
        );

        // The whole frame is now resident. Nothing handed out above may have
        // moved, been overwritten, or lost a row.
        for (miftah, mawdi) in &mawaqi {
            tahaqquq(
                hayya.safahat(),
                *miftah,
                mawdi,
                &surah(&rassam, *miftah),
                &format!("runtime at {hajm}px"),
            );
        }

        // And the map must still agree with what it handed out.
        for (miftah, mawdi) in &mawaqi {
            let Some(hali) = hayya.khareeta().mawdi(*miftah) else {
                panic!(
                    "glyph {} at {hajm}px was handed out this frame and is no longer mapped",
                    miftah.muarrif
                )
            };
            assert_eq!(
                (hali.safha, hali.s, hali.a, hali.ard, hali.irtifa),
                (mawdi.safha, mawdi.s, mawdi.a, mawdi.ard, mawdi.irtifa),
                "glyph {} moved inside the frame that was drawing it",
                miftah.muarrif
            );
        }
    }
}

/// A page the runtime atlas reports is the page its rectangles are measured
/// against.
///
/// The mesh builder divides every rectangle by the page dimensions the map
/// reports. A page that grew, or was opened at one size and recorded at
/// another, would put every texture coordinate on a different scale from the
/// rectangle it came from — and the visible half of that error is a glyph whose
/// lower rows sample past its own bitmap.
#[test]
fn ikhtibar_abaad_al_safahat_al_hayya_tutabiq_al_bayt() {
    let khutut = silsila();
    let khiyarat = KhiyaratRasf {
        aqsa_ard: 256,
        aqsa_irtifa: 256,
        ..KhiyaratRasf::default()
    };
    let mut hayya = LawhaHayya::jadeeda(khiyarat, NamatSafha::Taghtiya, 4 * 256 * 256)
        .expect("a runtime atlas inside a four-page budget");

    for hajm in SALALIM {
        hayya.ibda_itar();
        for miftah in mafatih(&khutut, HAJM, hajm) {
            let _ = hayya.shakl_min_silsila(miftah, &khutut);
        }

        for (fahras, safha) in hayya.safahat().iter().enumerate() {
            let raqm = u16::try_from(fahras).unwrap_or(u16::MAX);
            assert_eq!(
                safha.bayt.len(),
                usize::from(safha.ard) * usize::from(safha.irtifa),
                "page {fahras} is {}x{} and holds {} byte(s)",
                safha.ard,
                safha.irtifa,
                safha.bayt.len()
            );
            assert_eq!(
                hayya.khareeta().abaad_safha(raqm),
                Some((safha.ard, safha.irtifa)),
                "page {fahras} measures {}x{} and the map records {:?}",
                safha.ard,
                safha.irtifa,
                hayya.khareeta().abaad_safha(raqm)
            );
        }
    }
}
