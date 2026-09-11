//! The loop, end to end: a frame with English on it goes in, and Arabic glyph
//! pixels come out where the English was.
//!
//! What is real here: the software [`Khattaf`] draws with the same premultiplied
//! blend the GPU backends configure; capture reads the game's own pixels back
//! through the real [`Tabaqa`]; the read goes through the real preprocessing,
//! upscale and both refusal gates; the tracker, the settle policy and the
//! session are the shipped ones; the cache is a real file under the test's
//! scratch directory; the Arabic is shaped by `taarib-saff` from the staged
//! IBM Plex Arabic face into the real runtime atlas; and the batch is drawn
//! through [`Tabaqa::itar`] with its budget rules.
//!
//! What is not real, stated plainly: the **recognizer** segments the captured
//! pixels into line boxes and attaches the fixture's own strings to them in
//! reading order, so the boxes are genuinely found in the picture but the
//! words are not read out of it; and the **translator** is a dictionary, so
//! the cache-miss path is exercised without a model. Nothing about OCR accuracy
//! or translation quality is claimed by this file.
//!
//! The composited frame is written to `/mnt/e/Taarib/halaqa_burhan.png`.

#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

mod mushtarak;

use std::cell::Cell;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_saff::rasm::Rassam;
use taarib_saff::{KhiyaratTakhtit, Saff, TakhtitNass, TalabTakhtit};
use taarib_tabaqa::halaqa::{Halaqa, KhiyaratHalaqa, MakunatHalaqa, dhakira_murakkaba};
use taarib_tabaqa::iltiqat_shasha::SuraMultaqata;
use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::khazina::{DhakiraMalaf, masar_khazina};
use taarib_tabaqa::lawhat_tahakkum::Ikhtisarat;
use taarib_tabaqa::manatiq::MajmuatManatiq;
use taarib_tabaqa::mutarjim::{MutarjimTabaqa, RaddSatr, TalabSatr};
use taarib_tabaqa::mutarjim_mahalli::HalatMutarjim;
use taarib_tabaqa::qira::{IkhtiyarQari, Qari, SatrMaqru};
use taarib_tabaqa::qissa::{KhiyaratQissa, Qissa};
use taarib_tabaqa::sidq::{BasmatIfsah, Iqrar};
use taarib_tabaqa::wajiha::{MustatilBiksel, SighatSath, Tabaqa, WasfSath};

use crate::mushtarak::{Itar, KhattafBarmaji};

/// The surface the whole proof runs at.
const ARD: u32 = 1280;
/// Its height.
const IRTIFA: u32 = 720;

/// The size the game's own English is drawn at, in pixels.
const HAJM_LUBA: f32 = 24.0;

/// Where the menu's first line sits, and the pitch between lines.
const YASAR_QAIMA: u32 = 140;
const AALA_QAIMA: u32 = 440;
const KHUTWAT_QAIMA: u32 = 44;

/// The English the game draws, in reading order, and its Arabic.
const QAIMA: &[(&str, &str)] = &[
    ("New Game", "لعبة جديدة"),
    ("Continue", "متابعة"),
    ("Options", "الخيارات"),
    ("Quit to Desktop", "الخروج إلى سطح المكتب"),
];

/// The colour the game draws its English in: pale blue, so a pixel of it can
/// never be mistaken for a pixel of the overlay's near-white Arabic.
const LAWN_INJILIZI: [f32; 3] = [0.40, 0.60, 0.95];

/// Where the picture goes.
const MASAR_SURA: &str = "/mnt/e/Taarib/halaqa_burhan.png";

/// What every test here returns.
type Natija = Result<(), Box<dyn Error>>;

// ---------------------------------------------------------------------------
// The two stand-ins
// ---------------------------------------------------------------------------

/// A recognizer that finds the lines in the picture and names them from the
/// fixture.
///
/// The boxes are segmented from whatever pixels the preprocessing handed over
/// — colour or the grayscale chain, upscaled or not — by contrast against the
/// image's median, which is what makes both corroborating passes find the same
/// four rectangles. The strings are the fixture's, attached top to bottom.
#[derive(Debug)]
struct QariMustatilat {
    asma: Vec<&'static str>,
    qiraat: Arc<AtomicU64>,
}

impl Qari for QariMustatilat {
    fn ism(&self) -> &'static str {
        "fixture rectangles"
    }

    fn mutah(&self) -> bool {
        true
    }

    fn lughat(&self) -> &[&str] {
        &["en"]
    }

    fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        let _ = self.qiraat.fetch_add(1, Ordering::Relaxed);
        let rgb = sura.ila_rgb()?;
        let ard = usize::try_from(sura.ard()).unwrap_or(1).max(1);
        let ramadi: Vec<u8> = rgb
            .chunks_exact(3)
            .map(|biksel| {
                let majmu: u32 = biksel.iter().map(|qanat| u32::from(*qanat)).sum();
                u8::try_from(majmu.checked_div(3).unwrap_or(0)).unwrap_or(255)
            })
            .collect();
        let mut murattaba = ramadi.clone();
        murattaba.sort_unstable();
        let wasit = murattaba
            .get(murattaba.len().checked_div(2).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let hibr = |qeema: u8| i32::from(qeema).abs_diff(i32::from(wasit)) > 90;

        // Row profile, then runs of inked rows, then a column extent per run.
        let mut sufuf: Vec<bool> = Vec::new();
        for saf in ramadi.chunks_exact(ard) {
            sufuf.push(saf.iter().filter(|qeema| hibr(**qeema)).count() >= 2);
        }
        let mut sutur: Vec<MustatilBiksel> = Vec::new();
        let mut bidaya: Option<usize> = None;
        let mut faragh = 0_usize;
        for (fahras, mahbur) in sufuf.iter().copied().chain(std::iter::once(false)).enumerate() {
            if mahbur {
                if bidaya.is_none() {
                    bidaya = Some(fahras);
                }
                faragh = 0;
                continue;
            }
            let Some(awwal) = bidaya else {
                continue;
            };
            faragh = faragh.saturating_add(1);
            if faragh <= 6 && fahras < sufuf.len() {
                continue;
            }
            let akhir = fahras.saturating_sub(faragh);
            if akhir.saturating_sub(awwal) >= 4 {
                let mut yasar = usize::MAX;
                let mut yameen = 0_usize;
                for saf in awwal..akhir {
                    let Some(satr) = ramadi.get(saf.saturating_mul(ard)..saf.saturating_add(1).saturating_mul(ard))
                    else {
                        continue;
                    };
                    for (amud, qeema) in satr.iter().enumerate() {
                        if hibr(*qeema) {
                            yasar = yasar.min(amud);
                            yameen = yameen.max(amud.saturating_add(1));
                        }
                    }
                }
                if yameen > yasar {
                    sutur.push(MustatilBiksel {
                        yasar: u32::try_from(yasar).unwrap_or(0),
                        aala: u32::try_from(awwal).unwrap_or(0),
                        ard: u32::try_from(yameen.saturating_sub(yasar)).unwrap_or(0),
                        irtifa: u32::try_from(akhir.saturating_sub(awwal)).unwrap_or(0),
                    });
                }
            }
            bidaya = None;
            faragh = 0;
        }
        if sutur.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: "fixture".to_owned(),
                thiqa: None,
            });
        }
        Ok(sutur
            .into_iter()
            .zip(self.asma.iter())
            .map(|(mawdi, ism)| SatrMaqru::jadeed(ism, mawdi, 90, true))
            .collect())
    }
}

/// A dictionary standing in for a provider, counting what it was asked.
#[derive(Debug, Clone)]
struct MutarjimQamus {
    qamus: HashMap<&'static str, &'static str>,
    talabat: Arc<AtomicU64>,
}

impl MutarjimQamus {
    fn jadeed() -> Self {
        Self {
            qamus: QAIMA.iter().copied().collect(),
            talabat: Arc::new(AtomicU64::new(0)),
        }
    }

    fn adad(&self) -> u64 {
        self.talabat.load(Ordering::Relaxed)
    }
}

impl MutarjimTabaqa for MutarjimQamus {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait's signature is `&str`, and an impl cannot narrow it"
    )]
    fn ism(&self) -> &str {
        "dictionary"
    }

    fn mutah(&self) -> bool {
        true
    }

    fn tarjim(&self, talab: &TalabSatr<'_>) -> Result<RaddSatr, KhataTabaqa> {
        let _ = self.talabat.fetch_add(1, Ordering::Relaxed);
        self.qamus
            .get(talab.asl)
            .map(|arabi| RaddSatr::aaliya((*arabi).to_owned()))
            .ok_or_else(|| KhataTabaqa::MawridFashil {
                mawrid: "dictionary",
                sabab: format!("no entry for \"{}\"", talab.asl),
            })
    }
}

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/// The game's frame: the scene, and the menu drawn with the font's outlines.
struct Mashhad {
    asl: Arc<Mutex<Itar>>,
    takhtitat: Vec<TakhtitNass>,
    rassam: Rassam,
    sanadiq: Vec<MustatilBiksel>,
}

impl Mashhad {
    /// Draws the menu at a vertical offset and records where the ink landed.
    fn irsim(&mut self, izaha: u32) -> Natija {
        let mut itar = self.asl.lock();
        itar.mashhad();
        self.sanadiq.clear();
        for (fahras, takhtit) in self.takhtitat.iter().enumerate() {
            let aala = AALA_QAIMA
                .saturating_add(u32::try_from(fahras).unwrap_or(0).saturating_mul(KHUTWAT_QAIMA))
                .saturating_add(izaha);
            let sunduq = itar.utbua(&self.rassam, takhtit, YASAR_QAIMA, aala, LAWN_INJILIZI)?;
            self.sanadiq.push(sunduq);
        }
        Ok(())
    }
}

/// The scratch directory this test writes under.
fn mujallad_khidsh(ism: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("halaqa-{ism}-{}", std::process::id()))
}

/// The surface every frame is built against.
const fn sath() -> WasfSath {
    WasfSath {
        ard: ARD,
        irtifa: IRTIFA,
        sigha: SighatSath::Rgba8,
        sirgb: true,
    }
}

/// The fixture's identity. Not a real game.
fn luba() -> LubaId {
    LubaId::min_masdar(
        &MasdarLuba::Yadawi("taarib-halaqa-fixture".to_owned()),
        ISM_LUBA,
    )
}

const ISM_LUBA: &str = "Synthetic Menu (a test fixture, not a game)";

/// One overlay session over the shared game frame, with a fresh translator
/// counter and the cache at the given path.
struct Jalsa {
    halaqa: Halaqa,
    murakkab: Arc<Mutex<Itar>>,
    mutarjim: MutarjimQamus,
    khazina: Arc<DhakiraMalaf>,
    qiraat: Arc<AtomicU64>,
    lahza: Cell<u64>,
}

impl Jalsa {
    fn ibda(mashhad: &Mashhad, masar_khazina: &Path) -> Result<Self, Box<dyn Error>> {
        let (khutut, _, _) = mushtarak::khutut()?;
        let khattaf = KhattafBarmaji::jadeed(Arc::clone(&mashhad.asl), sath());
        let murakkab = Arc::clone(&khattaf.murakkab);
        let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), 0, ISM_LUBA)?;
        let tabaqa = Tabaqa::shaghghil(Box::new(khattaf), iqrar)?;

        let khazina = Arc::new(DhakiraMalaf::iftah(masar_khazina.to_path_buf(), "en")?);
        let mutarjim = MutarjimQamus::jadeed();
        let qissa = Qissa::jadeeda(
            luba(),
            ISM_LUBA,
            KhiyaratQissa {
                yasjil: false,
                ..KhiyaratQissa::iftiradiya()
            },
        )
        .bi_dhakira(dhakira_murakkaba(None, Arc::clone(&khazina)))
        .bi_mutarjim(Box::new(mutarjim.clone()))
        .bi_qari("fixture rectangles");
        let qiraat = Arc::new(AtomicU64::new(0));
        let qari = IkhtiyarQari::min_qari(
            Box::new(QariMustatilat {
                asma: QAIMA.iter().map(|(injilizi, _)| *injilizi).collect(),
                qiraat: Arc::clone(&qiraat),
            }),
            vec!["chosen by the test".to_owned()],
        );
        let makunat = MakunatHalaqa {
            khutut,
            qari: Ok(qari),
            qissa,
            hala_mutarjim: HalatMutarjim::Mutah {
                ism: "dictionary".to_owned(),
                namudhaj: "fixture".to_owned(),
                asas: "in-process".to_owned(),
            },
            khazina: Some(Arc::clone(&khazina)),
            sijill: None,
            manatiq: MajmuatManatiq::jadeeda(luba(), ISM_LUBA),
            ikhtisarat: Ikhtisarat::iftiradiya(),
            ism_luba: ISM_LUBA.to_owned(),
            athar: Vec::new(),
        };
        let khiyarat = KhiyaratHalaqa {
            fasil_shasha_milli: 250,
            ..KhiyaratHalaqa::iftiradiya()
        };
        let halaqa = Halaqa::ibda(tabaqa, makunat, khiyarat)?;
        Ok(Self {
            halaqa,
            murakkab,
            mutarjim,
            khazina,
            qiraat,
            lahza: Cell::new(0),
        })
    }

    /// Runs frames fifty logical milliseconds apart until the loop draws at
    /// least `adad` lines, or gives up.
    fn shaghghil_hatta(&mut self, adad: usize, aqsa_itarat: u32) -> Natija {
        for _ in 0..aqsa_itarat {
            let lahza = &self.lahza;
            let saa = move || lahza.get();
            self.halaqa.itar(&saa, &[])?;
            self.lahza.set(self.lahza.get().saturating_add(50_000));
            if self.halaqa.sutur_marsuma().len() >= adad {
                // One more frame, so the composite holds what the snapshot
                // just delivered.
                self.halaqa.itar(&saa, &[])?;
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(3));
        }
        Err(Box::<dyn Error>::from(format!(
            "the loop never drew {adad} line(s) within {aqsa_itarat} frames: {}",
            self.halaqa.khulasa()
        )))
    }

    /// Runs a fixed number of frames.
    fn shaghghil(&mut self, itarat: u32) -> Natija {
        for _ in 0..itarat {
            let lahza = &self.lahza;
            let saa = move || lahza.get();
            self.halaqa.itar(&saa, &[])?;
            self.lahza.set(self.lahza.get().saturating_add(50_000));
            std::thread::sleep(Duration::from_millis(3));
        }
        Ok(())
    }
}

/// Whether a linear pixel is the overlay's near-white Arabic ink.
fn arabi(lawn: [f32; 3]) -> bool {
    lawn.iter().all(|qanat| *qanat > 0.55)
        && (lawn[0] - lawn[1]).abs() < 0.08
        && (lawn[1] - lawn[2]).abs() < 0.08
}

/// Whether a linear pixel is the game's pale-blue English ink.
fn injilizi(lawn: [f32; 3]) -> bool {
    lawn[2] > 0.55 && lawn[0] < 0.55 && lawn[2] - lawn[0] > 0.25
}

/// How many pixels inside a box satisfy a predicate.
fn add(itar: &Itar, sunduq: MustatilBiksel, shart: impl Fn([f32; 3]) -> bool) -> u32 {
    let mut adad = 0_u32;
    for a in sunduq.aala..sunduq.aala.saturating_add(sunduq.irtifa) {
        for s in sunduq.yasar..sunduq.yasar.saturating_add(sunduq.ard) {
            if shart(itar.lawn(s, a)) {
                adad = adad.saturating_add(1);
            }
        }
    }
    adad
}

/// The game's frame with the menu drawn on it.
fn mashhad() -> Result<Mashhad, Box<dyn Error>> {
    let (khutut, khatt, _) = mushtarak::khutut()?;
    let rassam = Rassam::jadeed(&khatt)?;
    let mut saff = Saff::jadeed();
    let khiyarat = KhiyaratTakhtit::default();
    let mut takhtitat = Vec::new();
    for (injilizi, _) in QAIMA {
        takhtitat.push(saff.khattit(&TalabTakhtit {
            nass: injilizi,
            khutut: &khutut,
            hajm: HAJM_LUBA,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &khiyarat,
        })?);
    }
    let mut mashhad = Mashhad {
        asl: Arc::new(Mutex::new(Itar::jadeed(ARD, IRTIFA))),
        takhtitat,
        rassam,
        sanadiq: Vec::new(),
    };
    mashhad.irsim(0)?;
    Ok(mashhad)
}

// ---------------------------------------------------------------------------
// The proof
// ---------------------------------------------------------------------------

/// Capture, recognize through both gates, miss the cache, translate, write the
/// cache, shape, draw — and the Arabic lands on the English.
///
/// Then a second session over the same cache file draws the same four lines
/// without asking the translator once, and follows the menu when it moves.
#[test]
fn arabi_yursam_fawq_al_injilizi_wa_al_khazina_tujib_thaniyan() -> Natija {
    let mut mashhad = mashhad()?;
    let khidsh = mujallad_khidsh("burhan");
    let masar = masar_khazina(&khidsh, "en");
    let _ = fs::remove_file(&masar);

    // -- session one: every line is a cache miss ----------------------------
    let mut jalsa = Jalsa::ibda(&mashhad, &masar)?;
    for sunduq in &mashhad.sanadiq {
        let qabl = mashhad.asl.lock().clone();
        assert_eq!(
            add(&qabl, *sunduq, arabi),
            0,
            "the game's own frame must hold no Arabic ink before the overlay draws"
        );
        assert!(
            add(&qabl, *sunduq, injilizi) > 20,
            "the game's own frame must hold English ink in {sunduq:?}"
        );
    }
    jalsa.shaghghil_hatta(QAIMA.len(), 400)?;

    let marsuma = jalsa.halaqa.sutur_marsuma().to_vec();
    assert_eq!(
        marsuma.len(),
        QAIMA.len(),
        "four lines drawn, got {}: {}",
        marsuma.len(),
        jalsa.halaqa.khulasa()
    );
    let murakkab = jalsa.murakkab.lock().clone();
    murakkab.ihfaz(Path::new(MASAR_SURA))?;
    for (sunduq, (injilizi_nass, arabi_nass)) in mashhad.sanadiq.iter().zip(QAIMA) {
        let arabi_baad = add(&murakkab, *sunduq, arabi);
        let injilizi_qabl = add(&mashhad.asl.lock(), *sunduq, injilizi);
        let injilizi_baad = add(&murakkab, *sunduq, injilizi);
        assert!(
            arabi_baad >= 40,
            "\"{arabi_nass}\" left only {arabi_baad} Arabic ink pixel(s) inside the box \
             \"{injilizi_nass}\" occupied ({sunduq:?}); see {MASAR_SURA}"
        );
        assert!(
            injilizi_baad.saturating_mul(20) <= injilizi_qabl,
            "the plate under \"{arabi_nass}\" left {injilizi_baad} of {injilizi_qabl} English \
             pixel(s) showing; the original must be covered"
        );
        let mawjud = marsuma
            .iter()
            .any(|satr| satr.nass == *arabi_nass && yatadakhal(satr.mawdi, *sunduq));
        assert!(
            mawjud,
            "\"{arabi_nass}\" was not drawn over {sunduq:?}; drawn: {marsuma:?}"
        );
    }
    assert_eq!(
        jalsa.mutarjim.adad(),
        QAIMA.len() as u64,
        "four distinct strings cost {} provider request(s)",
        jalsa.mutarjim.adad()
    );
    let ihsaat = jalsa.khazina.ihsaat();
    assert_eq!(ihsaat.kutibat, QAIMA.len() as u64, "{}", jalsa.khazina.wasf());
    assert!(
        jalsa.qiraat.load(Ordering::Relaxed) >= 2,
        "the corroborating read runs the recognizer twice per accepted capture"
    );
    let athar = jalsa.halaqa.khudh_athar();
    assert!(
        athar.iter().any(|satr| satr.starts_with("first Arabic drawn")),
        "the loop's trace must record the first frame with Arabic on it: {athar:?}"
    );
    let sutur_malaf = fs::read_to_string(&masar)?;
    assert_eq!(
        sutur_malaf.lines().count(),
        QAIMA.len(),
        "the cache file holds one line per pair"
    );
    jalsa.halaqa.aghliq()?;
    drop(jalsa);

    // -- session two: the file answers everything ---------------------------
    let mut thaniya = Jalsa::ibda(&mashhad, &masar)?;
    assert_eq!(
        thaniya.khazina.adad(),
        QAIMA.len(),
        "a fresh open reads the four pairs back off the disk"
    );
    thaniya.shaghghil_hatta(QAIMA.len(), 400)?;
    assert_eq!(
        thaniya.mutarjim.adad(),
        0,
        "a second session must not ask the provider for a string the file holds"
    );
    assert_eq!(
        thaniya.khazina.ihsaat().isabat,
        QAIMA.len() as u64,
        "{}",
        thaniya.khazina.wasf()
    );
    // The panel's sentences are refreshed twice a second, not every frame;
    // half a second of frames is what it takes for the hits to reach it.
    thaniya.shaghghil(30)?;
    let bina = thaniya.halaqa.lawha().tarjama().cloned();
    let bina = bina.ok_or("the panel must have been told about the cache")?;
    assert!(
        bina.dhakira.contains("100"),
        "the panel's cache sentence must carry the hit rate: {}",
        bina.dhakira
    );

    // -- the menu jitters by three pixels: the same lines, no new requests --
    let aala_qabl: Vec<u32> = thaniya
        .halaqa
        .sutur_marsuma()
        .iter()
        .map(|satr| satr.mawdi.aala)
        .collect();
    mashhad.irsim(3)?;
    thaniya.shaghghil(40)?;
    let aala_baad: Vec<u32> = thaniya
        .halaqa
        .sutur_marsuma()
        .iter()
        .map(|satr| satr.mawdi.aala)
        .collect();
    assert_eq!(aala_baad.len(), QAIMA.len(), "{}", thaniya.halaqa.khulasa());
    for (qabl, baad) in aala_qabl.iter().zip(&aala_baad) {
        assert!(
            baad.abs_diff(*qabl) <= 4,
            "a three-pixel jitter moved a drawn line from {qabl} to {baad}"
        );
    }
    assert_eq!(thaniya.mutarjim.adad(), 0);

    // -- the menu moves by sixty: new identities, still no requests ---------
    mashhad.irsim(60)?;
    let mut tabiat = false;
    for _ in 0..12 {
        thaniya.shaghghil(10)?;
        let marsuma = thaniya.halaqa.sutur_marsuma();
        if marsuma.len() == QAIMA.len()
            && marsuma
                .iter()
                .all(|satr| satr.mawdi.aala >= AALA_QAIMA.saturating_add(50))
        {
            tabiat = true;
            break;
        }
    }
    assert!(
        tabiat,
        "the Arabic did not follow the menu sixty pixels down: {}",
        thaniya.halaqa.khulasa()
    );
    assert_eq!(
        thaniya.mutarjim.adad(),
        0,
        "re-identified lines are answered from the cache, not the provider"
    );
    thaniya.halaqa.aghliq()?;
    Ok(())
}

/// Whether two boxes share any vertical extent.
const fn yatadakhal(awwal: MustatilBiksel, thani: MustatilBiksel) -> bool {
    let asfal_awwal = awwal.aala.saturating_add(awwal.irtifa);
    let asfal_thani = thani.aala.saturating_add(thani.irtifa);
    awwal.aala < asfal_thani && thani.aala < asfal_awwal
}

/// A stopped recognizer reaches the panel and the caption, and the loop keeps
/// drawing the panel rather than nothing.
#[test]
fn qari_mayyit_yasil_ila_al_lawha() -> Natija {
    #[derive(Debug)]
    struct QariMayyit;
    impl Qari for QariMayyit {
        fn ism(&self) -> &'static str {
            "dead"
        }
        fn mutah(&self) -> bool {
            false
        }
        fn lughat(&self) -> &[&str] {
            &["en"]
        }
        fn iqra(&mut self, _: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
            Err(KhataTabaqa::QariGhayrMutah {
                sabab: "the language pack was removed".to_owned(),
            })
        }
    }

    let mashhad = mashhad()?;
    let (khutut, _, _) = mushtarak::khutut()?;
    let khattaf = KhattafBarmaji::jadeed(Arc::clone(&mashhad.asl), sath());
    let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), 0, ISM_LUBA)?;
    let tabaqa = Tabaqa::shaghghil(Box::new(khattaf), iqrar)?;
    let qissa = Qissa::jadeeda(luba(), ISM_LUBA, KhiyaratQissa::iftiradiya());
    let makunat = MakunatHalaqa {
        khutut,
        qari: Ok(IkhtiyarQari::min_qari(Box::new(QariMayyit), Vec::new())),
        qissa,
        hala_mutarjim: HalatMutarjim::Ghaib(taarib_usus::idadat::HalatMuzawwidin::Faragh),
        khazina: None,
        sijill: None,
        manatiq: MajmuatManatiq::jadeeda(luba(), ISM_LUBA),
        ikhtisarat: Ikhtisarat::iftiradiya(),
        ism_luba: ISM_LUBA.to_owned(),
        athar: Vec::new(),
    };
    let mut halaqa = Halaqa::ibda(
        tabaqa,
        makunat,
        KhiyaratHalaqa {
            fasil_shasha_milli: 250,
            ..KhiyaratHalaqa::iftiradiya()
        },
    )?;
    let lahza = Cell::new(0_u64);
    let mut mutawaqqifa = false;
    for _ in 0..200 {
        let saa = || lahza.get();
        halaqa.itar(&saa, &[])?;
        lahza.set(lahza.get().saturating_add(50_000));
        if halaqa
            .lawha()
            .khayt()
            .is_some_and(taarib_tabaqa::qissa::HalatKhayt::mutawaqqifa)
        {
            mutawaqqifa = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(3));
    }
    assert!(
        mutawaqqifa,
        "the panel never heard that recognition stopped: {}",
        halaqa.khulasa()
    );
    let ihsaat = *halaqa.ihsaat();
    assert!(ihsaat.iltiqatat >= 1, "at least one capture was read back");
    assert!(
        ihsaat.itarat_bi_arabi == 0,
        "nothing can have been drawn from a dead recognizer"
    );
    let athar = halaqa.khudh_athar();
    assert!(
        athar.iter().any(|satr| satr.contains("recognition stopped")),
        "the trace names the stop: {athar:?}"
    );
    halaqa.aghliq()?;
    Ok(())
}
