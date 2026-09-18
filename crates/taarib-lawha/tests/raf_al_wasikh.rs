//! رفع الوسخ — an uploader that sends only what changed still sends everything
//! a glyph can be sampled from.
//!
//! The adapter used to re-upload every atlas page in full on every frame, which
//! made the question this file asks unaskable: whatever the dirty tracking said,
//! the whole page went up anyway. Once that stops, what reaches the GPU is
//! exactly the rectangles somebody decided to send, and the rest of the texture
//! is whatever was in it before — so the rule for choosing those rectangles
//! becomes load-bearing, and it is not the obvious one.
//!
//! ## The rule, and why the glyph's own rectangle is not enough
//!
//! A bilinear tap at the edge of a glyph's rectangle reads one texel outside it.
//! [`taarib_lawha::rasf`] reserves exactly that much around every glyph and
//! keeps it at zero — and it keeps it at zero *on the CPU side*. Two things put
//! zeros there and neither of them is visible to an uploader that watches glyph
//! rectangles:
//!
//!   - a page is zero-filled when it is opened, so a gutter has never held
//!     anything;
//!   - `Rasif::harrir` zeroes a whole allocation when a glyph is evicted, and a
//!     smaller glyph packed into that slot afterwards leaves the tail of the old
//!     letter inside the *new* glyph's gutter — on the CPU it is zeros, on the
//!     GPU it is still the old letter.
//!
//! So the rectangle that has to go up is the glyph's own grown by the gutter.
//! Both tests below drive a real runtime atlas through a budget small enough
//! that eviction and reuse run constantly, model the GPU's copy as a byte array
//! that only ever receives what an uploader sent, and compare. The first asserts
//! the grown rectangle is sufficient. The second asserts the glyph's own
//! rectangle is not, because a rule that is merely believed to be necessary gets
//! narrowed by the next person who reads it.
//!
//! The model is deliberately tighter than the adapter: `Lawha.Iltaqit` unions a
//! page's rectangles into one bounding box and uploads that, which is a superset
//! of the per-glyph rectangles modelled here. A rule sufficient per glyph is
//! sufficient for their union.
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
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking, and the lints are written for library code"
)]
#![expect(
    clippy::print_stdout,
    reason = "the byte accounting is the measurement this file exists to produce, and a number \
              nobody can read is a number nobody checks"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_lawha::khareeta::{MawdiShakl, MiftahShakl, NamatSafha};
use taarib_lawha::rasf::{KhiyaratRasf, Safha};
use taarib_lawha::{JamiAshkal, LawhaHayya};
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, Saff, SilsilatKhutut, TalabTakhtit};

/// A subtitle's worth of Arabic: joined forms, a lam-alef ligature, marks, a
/// deep-descending final ya, and the digits a line of dialogue usually carries.
const JUMLA: &str = "لا تقترب من الباب، فالحارس ينظر إليك الآن — ٣ دقائق فقط";

/// A second line, so the working set changes between frames and the evictor has
/// something to prefer.
const JUMLA_THANIYA: &str = "المفتاح تحت السجّادة، لكنّ الباب الخلفي مقفل منذ أمسٍ بعيد";

/// The layout size, in pixels.
const HAJM: f32 = 28.0;

/// The rungs the C# ladder quantizes an on-screen size to.
const SALALIM: [f32; 10] = [8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 96.0, 128.0, 192.0];

/// The gutter the packer leaves around every glyph, and the plugin's own value.
const HASHW: u16 = 1;

/// Pages small enough that a subtitle at the upper rungs needs several of them.
const BUD: u16 = 256;

/// Two pages of budget, so the evictor runs on nearly every frame.
const MIZANIYA: usize = 2 * 256 * 256;

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

/// A one-font chain over the staged face.
fn silsila() -> SilsilatKhutut {
    let masar = masar_khatt();
    let bayt = fs::read(&masar).unwrap_or_else(|_| panic!("{} unreadable", masar.display()));
    let mawrid = MawridKhatt::jadeed(Arc::new(bayt), 0)
        .unwrap_or_else(|khata| panic!("{}: {khata}", masar.display()));
    SilsilatKhutut::wahid(Arc::new(mawrid)).expect("a one-font chain")
}

/// The glyph keys one sentence asks the atlas for, at one rasterization size.
fn mafatih(khutut: &SilsilatKhutut, nass: &str, hajm_lawha: f32) -> Vec<MiftahShakl> {
    let khiyarat = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();
    let takhtit = saff
        .khattit(&TalabTakhtit {
            nass,
            khutut,
            hajm: HAJM,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &khiyarat,
        })
        .unwrap_or_else(|khata| panic!("laying {nass:?} out: {khata}"));

    let mut jami = JamiAshkal::jadeed(NamatSafha::Taghtiya);
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

/// A rectangle of texels inside one page, as the uploader names it.
#[derive(Debug, Clone, Copy)]
struct Mustatil {
    s: usize,
    a: usize,
    ard: usize,
    irtifa: usize,
}

/// The rectangle a glyph occupies, optionally grown by the gutter and clipped
/// to the page.
///
/// `hashw` of zero is the narrow rule the second test exists to refuse; one is
/// what the adapter sends.
fn mustatil(mawdi: &MawdiShakl, safha: &Safha, hashw: usize) -> Mustatil {
    let s = usize::from(mawdi.s).saturating_sub(hashw);
    let a = usize::from(mawdi.a).saturating_sub(hashw);
    let yameen =
        (usize::from(mawdi.s) + usize::from(mawdi.ard) + hashw).min(usize::from(safha.ard));
    let asfal =
        (usize::from(mawdi.a) + usize::from(mawdi.irtifa) + hashw).min(usize::from(safha.irtifa));
    Mustatil {
        s,
        a,
        ard: yameen.saturating_sub(s),
        irtifa: asfal.saturating_sub(a),
    }
}

/// The GPU's copy of every page, which only ever receives what was uploaded.
#[derive(Debug, Default)]
struct Mirat {
    safahat: Vec<Vec<u8>>,
    /// How many whole pages were uploaded, which only a new page costs.
    kamila: u64,
    /// How many sub-rectangles were uploaded.
    juziya: u64,
    /// How many texels crossed in total, whole pages included.
    texelat: u64,
}

impl Mirat {
    /// Copies any page the atlas opened since the last frame, in full.
    ///
    /// A texture the adapter is about to create has undefined contents, so the
    /// whole page goes up — the one case where a full upload is correct rather
    /// than wasteful.
    fn zamin(&mut self, safahat: &[Safha]) {
        while self.safahat.len() < safahat.len() {
            let fahras = self.safahat.len();
            let masdar = &safahat[fahras];
            self.safahat.push(masdar.bayt.clone());
            self.kamila += 1;
            self.texelat += u64::try_from(masdar.bayt.len()).unwrap_or(0);
        }
    }

    /// Copies one rectangle of one page.
    fn irfa(&mut self, safahat: &[Safha], fahras: usize, mustatil: Mustatil) {
        let masdar = &safahat[fahras];
        let hadaf = &mut self.safahat[fahras];
        let khatwa = usize::from(masdar.ard);
        for satr in 0..mustatil.irtifa {
            let bidaya = (mustatil.a + satr) * khatwa + mustatil.s;
            let nihaya = bidaya + mustatil.ard;
            hadaf[bidaya..nihaya].copy_from_slice(&masdar.bayt[bidaya..nihaya]);
        }
        self.juziya += 1;
        self.texelat += u64::try_from(mustatil.ard * mustatil.irtifa).unwrap_or(0);
    }

    /// The first texel inside a rectangle where this copy and the atlas differ.
    fn farq(&self, safahat: &[Safha], fahras: usize, mustatil: Mustatil) -> Option<(usize, usize)> {
        let masdar = &safahat[fahras];
        let khatwa = usize::from(masdar.ard);
        for satr in 0..mustatil.irtifa {
            for amud in 0..mustatil.ard {
                let izaha = (mustatil.a + satr) * khatwa + mustatil.s + amud;
                if self.safahat[fahras][izaha] != masdar.bayt[izaha] {
                    return Some((mustatil.s + amud, mustatil.a + satr));
                }
            }
        }
        None
    }
}

/// Drives a runtime atlas through every rung twice, uploading with one rule, and
/// reports the first glyph whose sampled neighbourhood the GPU disagrees about.
///
/// `hashw_rafa` is the gutter the *uploader* adds. The atlas always packs with
/// [`HASHW`]; the question is only what the uploader sends.
fn jarrib(hashw_rafa: usize) -> (Mirat, Option<String>) {
    let khutut = silsila();
    let khiyarat = KhiyaratRasf {
        aqsa_ard: BUD,
        aqsa_irtifa: BUD,
        hashw: HASHW,
        ..KhiyaratRasf::default()
    };
    let mut hayya = LawhaHayya::jadeeda(khiyarat, NamatSafha::Taghtiya, MIZANIYA)
        .expect("a runtime atlas inside a two-page budget");
    let mut mirat = Mirat::default();
    let mut maruf: Vec<MiftahShakl> = Vec::new();

    // Twice through the ladder, because the interesting state is the second
    // pass: by then every rectangle in the atlas has been evicted and reused at
    // least once, which is the condition the gutter rule exists for.
    for dawra in 0..2 {
        for hajm in SALALIM {
            let nass = if dawra == 0 { JUMLA } else { JUMLA_THANIYA };
            let ashkal = mafatih(&khutut, nass, hajm);

            hayya.ibda_itar();
            let mut hadatha: Vec<(MiftahShakl, MawdiShakl)> = Vec::with_capacity(ashkal.len());
            for miftah in &ashkal {
                // A budget this small genuinely runs out of unpinned rectangles,
                // and refusing is the documented answer. A refusal uploads
                // nothing and is not a defect.
                if let Ok(mawdi) = hayya.shakl_min_silsila(*miftah, &khutut)
                    && mawdi.ard != 0
                    && mawdi.irtifa != 0
                {
                    hadatha.push((*miftah, mawdi));
                    if !maruf.contains(miftah) {
                        maruf.push(*miftah);
                    }
                }
            }

            // Residency is finished; now the frame's upload, exactly as the
            // adapter does it — new pages whole, everything else by rectangle.
            mirat.zamin(hayya.safahat());
            for (_, mawdi) in &hadatha {
                let fahras = usize::from(mawdi.safha);
                let mustatil = mustatil(mawdi, &hayya.safahat()[fahras], hashw_rafa);
                mirat.irfa(hayya.safahat(), fahras, mustatil);
            }

            // Every glyph the atlas still maps — not only this frame's — must be
            // sampleable from the GPU's copy. A glyph that was resident three
            // frames ago and is still mapped is still being drawn by whatever
            // text object asked for it.
            for miftah in &maruf {
                let Some(mawdi) = hayya.khareeta().mawdi(*miftah) else {
                    continue;
                };
                if mawdi.ard == 0 || mawdi.irtifa == 0 {
                    continue;
                }
                let fahras = usize::from(mawdi.safha);
                let safha = &hayya.safahat()[fahras];
                // Checked over the neighbourhood a bilinear tap can reach, which
                // is the glyph grown by the packer's gutter — never by the
                // uploader's, whatever the uploader chose to send.
                let manzur = mustatil(mawdi, safha, usize::from(HASHW));
                if let Some((s, a)) = mirat.farq(hayya.safahat(), fahras, manzur) {
                    let izaha = a * usize::from(safha.ard) + s;
                    let sabab = format!(
                        "at {hajm}px, pass {dawra}: glyph {} sits at ({}, {}) {}x{} on page \
                         {fahras}, and texel ({s}, {a}) of what a bilinear tap at its edge reads \
                         is {} on the GPU against {} in the atlas",
                        miftah.muarrif,
                        mawdi.s,
                        mawdi.a,
                        mawdi.ard,
                        mawdi.irtifa,
                        mirat.safahat[fahras][izaha],
                        safha.bayt[izaha],
                    );
                    return (mirat, Some(sabab));
                }
            }
        }
    }
    (mirat, None)
}

// ---------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------

/// Uploading each glyph's rectangle grown by the gutter keeps every glyph
/// sampleable, through any amount of eviction and reuse.
///
/// This is the contract `Lawha.Sajjil` now implements, stated as an assertion
/// over a real atlas rather than as an argument in a comment.
#[test]
fn ikhtibar_al_rafa_bil_hashw_tabqa_kull_shakl_saliha() {
    let (mirat, khata) = jarrib(usize::from(HASHW));
    assert!(
        khata.is_none(),
        "uploading the glyph and its gutter still left a glyph unsampleable — {}",
        khata.unwrap_or_default()
    );
    assert!(
        mirat.juziya > 0,
        "nothing was uploaded by rectangle, so the rule was never exercised"
    );

    // What the old rule cost for the same run: every open page, in full, on
    // every frame. This drive is the worst case the change can be measured on —
    // twenty frames, each one a different sentence at a different size against a
    // two-page budget, so the entire working set is evicted and re-rasterized
    // between frames — and the saving is still better than half. The case the
    // defect was actually about is the opposite one, a scene nobody is changing,
    // and that is `ikhtibar_itar_bila_ashkal_la_yughayyir_shayan` below.
    let itarat = 2 * u64::try_from(SALALIM.len()).unwrap_or(0);
    let safahat = u64::try_from(mirat.safahat.len()).unwrap_or(0);
    let kamil = itarat * safahat * u64::from(BUD) * u64::from(BUD);
    println!(
        "{itarat} frame(s) over {safahat} page(s): {} texel(s) uploaded in {} whole page(s) and \
         {} rectangle(s), against {kamil} for a full re-upload every frame",
        mirat.texelat, mirat.kamila, mirat.juziya
    );
    assert!(
        mirat.texelat * 2 < kamil,
        "the incremental uploader moved {} texel(s) where a full re-upload every frame moves \
         {kamil}; even with the whole working set churning every frame it has to cost less than \
         half of that, or the rectangles are not being tracked",
        mirat.texelat
    );
    assert_eq!(
        mirat.kamila, safahat,
        "a page was uploaded whole more than once, so something other than opening it forced a \
         full upload"
    );
}

/// Uploading each glyph's own rectangle and nothing more does not.
///
/// The gutter is the whole of the difference between the two tests, and this one
/// is what stops it being optimised away by a reader who sees a rectangle being
/// grown for no reason they can find. The failure it catches is a sliver of one
/// letter along the edge of another, at some sizes, on some pages, in motion.
#[test]
fn ikhtibar_al_rafa_bila_hashw_tatruk_shaklan_ghayr_salih() {
    let (_, khata) = jarrib(0);
    assert!(
        khata.is_some(),
        "uploading only each glyph's own texels left every glyph sampleable, which means this \
         atlas never evicted and reused a rectangle and the case proves nothing"
    );
}

/// A frame that rasterized nothing uploads nothing.
///
/// The defect this file was written for was the opposite: beginning a frame
/// marked every page dirty, so a scene that had been static for ten minutes
/// re-uploaded the whole atlas sixty times a second. Stated here in the terms
/// this crate can state it — the pages themselves do not change — because the
/// adapter's half is checked in `Taarib.Unity.Fahs`.
#[test]
fn ikhtibar_itar_bila_ashkal_la_yughayyir_shayan() {
    let khutut = silsila();
    let khiyarat = KhiyaratRasf {
        aqsa_ard: BUD,
        aqsa_irtifa: BUD,
        hashw: HASHW,
        ..KhiyaratRasf::default()
    };
    let mut hayya = LawhaHayya::jadeeda(khiyarat, NamatSafha::Taghtiya, MIZANIYA)
        .expect("a runtime atlas inside a two-page budget");

    let ashkal = mafatih(&khutut, JUMLA, 32.0);
    hayya.ibda_itar();
    for miftah in &ashkal {
        let _ = hayya.shakl_min_silsila(*miftah, &khutut);
    }
    let qabl: Vec<Vec<u8>> = hayya.safahat().iter().map(|s| s.bayt.clone()).collect();

    // Nine more frames of the same text, which is what a static menu is.
    for _ in 0..9 {
        hayya.ibda_itar();
        for miftah in &ashkal {
            let _ = hayya.shakl_min_silsila(*miftah, &khutut);
        }
    }

    let baad: Vec<Vec<u8>> = hayya.safahat().iter().map(|s| s.bayt.clone()).collect();
    assert_eq!(
        qabl.len(),
        baad.len(),
        "redrawing the same text opened a page"
    );
    for (fahras, (awwal, thani)) in qabl.iter().zip(baad.iter()).enumerate() {
        assert!(
            awwal == thani,
            "page {fahras} changed across nine frames that drew the same text, so an uploader \
             that sends what changed would have had something to send"
        );
    }
}
