//! برهان التلقيم — the overlay drawing Arabic over a frame, end to end.
//!
//! Runs the whole tier-3 path that Phase 28 Stage 5 existed to close:
//!
//! 1. a frame is rendered *by something that is not Taarib* — English text drawn
//!    straight into a pixel buffer with the font's own outlines, which is what a
//!    game presents;
//! 2. the rectangles that text occupies become [`SatrMaqru`] values, exactly as a
//!    recognizer would report them;
//! 3. Arabic translations are attached to those rectangles as [`SatrMulaqqam`];
//! 4. [`LawhatTahakkum::bina`] produces the real control panel's elements;
//! 5. [`Mulaqqim`] shapes all of it through `taarib-saff`, packs every glyph into
//!    a runtime [`taarib_lawha::namu::LawhaHayya`], and emits one
//!    [`taarib_tabaqa::LawhatRasm`];
//! 6. [`Tabaqa::shaghghil`] starts an overlay over a backend — with a real
//!    [`Iqrar`], because there is no other way to start one — and
//!    [`Mulaqqim::qaddim`] uploads the atlas and calls [`Tabaqa::itar`];
//! 7. the backend draws the batch and the frame is written out as a PNG.
//!
//! ## What the backend is, stated plainly
//!
//! [`KhattafBarmaji`] is a **software** [`Khattaf`]. It is not Direct3D, OpenGL
//! or Vulkan, and this example does not hook a game. Every other link in the
//! chain above is the shipping one — the same shaper, the same atlas, the same
//! batch builder, the same [`Tabaqa`] frame rules, the same disclosure gate, the
//! same [`Khattaf`] contract and the same premultiplied blend the four real
//! backends configure. What is replaced is the call into a graphics driver, and
//! only that.
//!
//! [`KhattafBarmaji::irsim`] reproduces `gl.rs`'s fragment stage line for line:
//! `asas = nasij ? texel : vec4(1.0)`, `mazuj = asas * lawn`, then `ONE,
//! INV_SRC_ALPHA` over the destination. If the picture it writes is right, the
//! geometry and the colour the GPU backends are handed are right.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p taarib-tabaqa --example talqeem_burhan
//! ```

#![allow(
    clippy::print_stdout,
    reason = "this example exists to report what the overlay produced"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel coordinates are deliberately narrowed to integers, with the ranges checked"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "every index here is bounded by a loop over the buffer's own dimensions"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use taarib_saff::rasm::{NamatRasm, Rassam};
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, Saff, SilsilatKhutut, TakhtitNass, TalabTakhtit};
use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::lawhat_tahakkum::LawhatTahakkum;
use taarib_tabaqa::qira::SatrMaqru;
use taarib_tabaqa::sidq::{BasmatIfsah, Iqrar, NASS_IFSAH_ARABI};
use taarib_tabaqa::talqeem::{KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
use taarib_tabaqa::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, SighatSath, Tabaqa, WajihatRusum, WasfSath,
};

/// The surface the whole example runs at.
const ARD: u32 = 1280;
/// The surface height.
const IRTIFA: u32 = 720;

/// The size the game's own English is drawn at, in pixels.
const HAJM_LUBA: f32 = 21.0;

/// One line of the game's dialogue and the Arabic that came back for it.
#[derive(Debug, Clone, Copy)]
struct Hiwar {
    /// What the game drew.
    injilizi: &'static str,
    /// What the translation pipeline returned for it.
    arabi: &'static str,
    /// The left edge of the line the game drew it on.
    yasar: u32,
    /// The top edge.
    aala: u32,
}

/// The dialogue this frame contains.
///
/// The Arabic is ordinary Unicode in logical order — no presentation forms, and
/// the example checks that before it shapes anything. Every joined letterform in
/// the output picture is the font's own `GSUB` doing its work at draw time.
const HIWAR: &[Hiwar] = &[
    Hiwar {
        injilizi: "The north road is closed until the thaw.",
        arabi: "الطريق الشمالي مغلق حتى ذوبان الثلج.",
        yasar: 150,
        aala: 470,
    },
    Hiwar {
        injilizi: "Take the pass below the old watchtower.",
        arabi: "اسلك الممر أسفل برج المراقبة القديم.",
        yasar: 150,
        aala: 512,
    },
    Hiwar {
        injilizi: "I will wait for you at the bridge.",
        arabi: "سأنتظرك عند الجسر.",
        yasar: 150,
        aala: 554,
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let (masar_khatt, bayt) = ijid_khatt()?;
    println!("font: {}", masar_khatt.display());

    let khatt = Arc::new(MawridKhatt::jadeed(Arc::new(bayt), 0)?);
    let khutut = SilsilatKhutut::wahid(Arc::clone(&khatt))?;
    let rassam = Rassam::jadeed(&khatt)?;

    // -- the input is ordinary Unicode ------------------------------------
    // Checked rather than asserted in prose. A presentation form anywhere in
    // the source text would mean the joining in the output picture came from
    // the string instead of from the font, which would make the whole proof
    // worthless.
    for satr in HIWAR {
        if let Some(harf) = satr.arabi.chars().find(|harf| huwa_shakl_ard(*harf)) {
            return Err(Box::<dyn Error>::from(format!(
                "the source text carries the Arabic presentation form U+{:04X}; this project \
                 never produces one and must never consume one",
                u32::from(harf)
            )));
        }
    }
    println!(
        "source text: no Arabic Presentation Forms — checked, all {} lines",
        HIWAR.len()
    );

    // -- 1. a frame the game rendered -------------------------------------
    let mut itar = Itar::jadeed(ARD, IRTIFA);
    itar.mashhad();
    let mut saff = Saff::jadeed();
    let khiyarat_luba = KhiyaratTakhtit::default();

    let mut maqru: Vec<SatrMaqru> = Vec::new();
    for satr in HIWAR {
        let takhtit = saff.khattit(&TalabTakhtit {
            nass: satr.injilizi,
            khutut: &khutut,
            hajm: HAJM_LUBA,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &khiyarat_luba,
        })?;
        // The game draws its own text. Nothing about this goes through the
        // overlay: it is here so there is something underneath for the Arabic
        // to cover, and so the recognized rectangle is a rectangle real ink
        // actually occupies.
        let sunduq = itar.utbua(&rassam, &takhtit, satr.yasar, satr.aala, [0.85, 0.87, 0.90])?;
        maqru.push(SatrMaqru::jadeed(satr.injilizi, sunduq, 94, true));
    }
    println!(
        "\nthe game's frame: {ARD}×{IRTIFA}, {} lines of its own text drawn",
        maqru.len()
    );
    for (satr, khaam) in maqru.iter().zip(HIWAR) {
        println!(
            "  read {:>3},{:>3} {:>3}×{:>2} conf {:>3}  {:?}",
            satr.mawdi.yasar,
            satr.mawdi.aala,
            satr.mawdi.ard,
            satr.mawdi.irtifa,
            satr.thiqa,
            khaam.injilizi
        );
    }

    // -- 2. what the producer is fed --------------------------------------
    // The recognizer's boxes are already in surface coordinates here, because
    // this capture is the whole surface. `SatrMulaqqam::min_maqru` takes the
    // converted rectangle separately precisely so that a caller whose capture
    // was a sub-region cannot forget to convert.
    let sutur: Vec<SatrMulaqqam> = maqru
        .iter()
        .zip(HIWAR)
        .map(|(satr, khaam)| SatrMulaqqam::min_maqru(satr, satr.mawdi, khaam.arabi))
        .collect();

    // -- 3. the real control panel ----------------------------------------
    let mut lawha_tahakkum = LawhatTahakkum::jadeeda("برهان التلقيم");
    lawha_tahakkum.hala_mut().irfa();
    let sath = WasfSath {
        ard: ARD,
        irtifa: IRTIFA,
        sigha: SighatSath::Rgba8,
        sirgb: true,
    };
    let ansur = lawha_tahakkum.bina(sath);
    println!(
        "control panel: {} elements from LawhatTahakkum::bina",
        ansur.len()
    );

    // -- 4. the producer ---------------------------------------------------
    let mut mulaqqim = Mulaqqim::jadeed(khutut, KhiyaratTalqeem::iftiradiya())?;

    // -- 5. the overlay, behind the disclosure it cannot start without -----
    println!("\n--- the tier-3 disclosure, shown because the type system requires it ---");
    println!("{NASS_IFSAH_ARABI}");
    println!("--- end of disclosure ---\n");
    let lahza = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |mudda| mudda.as_secs());
    let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), lahza, "برهان التلقيم")?;

    let musattah = Arc::new(Mutex::new(itar));
    let khattaf = KhattafBarmaji::jadeed(Arc::clone(&musattah), sath);
    let mut tabaqa = Tabaqa::shaghghil(Box::new(khattaf), iqrar)?;
    for satr in tabaqa.athar() {
        println!("overlay: {satr}");
    }

    // -- 6. produce a batch and draw it -----------------------------------
    let Some(sath_hayy) = tabaqa.sath() else {
        return Err(Box::<dyn Error>::from(
            "the overlay reported no live surface",
        ));
    };
    let dufaa = mulaqqim.qaddim(&mut tabaqa, sath_hayy, &sutur, &ansur, 0)?;

    utbua_dufaa(&dufaa, &mulaqqim);

    // -- 7. write the proof -----------------------------------------------
    let masar = masar_kharj();
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid)?;
    }
    musattah.lock().uktub(&masar)?;
    println!("\nwrote {}", masar.display());
    println!(
        "reached: batch produced -> atlas uploaded -> handed to Tabaqa::itar -> drawn offscreen \
         through a software Khattaf. Not drawn over a running game."
    );
    Ok(())
}

/// Prints what the batch and the atlas actually contained.
fn utbua_dufaa(dufaa: &LawhatRasm, mulaqqim: &Mulaqqim) {
    let ihsaat = mulaqqim.ihsaat();
    println!("\n=== the batch ===");
    println!(
        "quads {}  (from {} translated lines and {} panel elements)",
        dufaa.qitaat.len(),
        ihsaat.sutur,
        ihsaat.ansur
    );
    let masturat = dufaa
        .qitaat
        .iter()
        .filter(|qita| qita.khareeta.is_some())
        .count();
    println!("  textured (glyphs): {masturat}");
    println!(
        "  solid (plates and panel): {}",
        dufaa.qitaat.len() - masturat
    );
    println!(
        "surface recorded on the batch: {}×{}",
        dufaa.sath.ard, dufaa.sath.irtifa
    );

    println!("\n=== the runtime atlas ===");
    let namu = ihsaat.lawha;
    println!(
        "glyphs {}  pages {}  bytes {}  hit rate {:.3}  evictions {}",
        namu.ashkal,
        namu.safahat,
        namu.bayt,
        namu.nisbat_isaba(),
        namu.ikhlaat
    );
    println!("uploads to the backend: {}", ihsaat.marfuaat);
    if ihsaat.ashkal_mafquda > 0 {
        println!(
            "glyphs the atlas refused: {} (last: {})",
            ihsaat.ashkal_mafquda,
            ihsaat.akhir_radd.as_deref().unwrap_or("—")
        );
    } else {
        println!("glyphs the atlas refused: 0");
    }
}

/// Whether a character is in one of the two Arabic Presentation Forms blocks.
///
/// U+FB50..=U+FDFF and U+FE70..=U+FEFF. Nothing in this project produces one and
/// nothing may consume one; this is the check that says so about the input to
/// this example rather than the claim.
const fn huwa_shakl_ard(harf: char) -> bool {
    let raqm = harf as u32;
    (raqm >= 0xFB50 && raqm <= 0xFDFF) || (raqm >= 0xFE70 && raqm <= 0xFEFF)
}

// ---------------------------------------------------------------------------
// The frame
// ---------------------------------------------------------------------------

/// A linear-light RGBA frame buffer.
///
/// Linear because that is what the surface this example declares is: `sirgb` is
/// true, which in the real backends means the driver encodes on write and the
/// shader emits linear. The encode happens once here, when the PNG is written,
/// so every blend in between is the blend a GPU would do.
#[derive(Debug)]
struct Itar {
    ard: u32,
    irtifa: u32,
    /// Four linear floats per pixel, row-major from the top.
    biksel: Vec<f32>,
}

impl Itar {
    /// A black frame.
    fn jadeed(ard: u32, irtifa: u32) -> Self {
        let adad = (ard as usize) * (irtifa as usize) * 4;
        Self {
            ard,
            irtifa,
            biksel: vec![0.0; adad],
        }
    }

    /// The index of a pixel's first channel, when it is on the frame.
    const fn fahras(&self, s: u32, a: u32) -> Option<usize> {
        if s >= self.ard || a >= self.irtifa {
            return None;
        }
        Some(((a as usize) * (self.ard as usize) + (s as usize)) * 4)
    }

    /// Paints something for the overlay to sit on top of.
    ///
    /// A vertical gradient with a lighter dialogue panel across the lower third.
    /// It is not a game, and it is not pretending to be one — it is a background
    /// with enough tonal range that a translucent plate over it is visibly a
    /// translucent plate.
    fn mashhad(&mut self) {
        for a in 0..self.irtifa {
            let nisba = (a as f32) / (self.irtifa as f32);
            for s in 0..self.ard {
                let ufuqi = (s as f32) / (self.ard as f32);
                let lawn = [
                    ufuqi.mul_add(0.012, nisba.mul_add(0.055, 0.020)),
                    nisba.mul_add(0.070, 0.028),
                    nisba.mul_add(0.105, 0.045),
                ];
                if let Some(fahras) = self.fahras(s, a) {
                    self.biksel[fahras] = lawn[0];
                    self.biksel[fahras + 1] = lawn[1];
                    self.biksel[fahras + 2] = lawn[2];
                    self.biksel[fahras + 3] = 1.0;
                }
            }
        }
        // The game's own dialogue panel.
        self.mustatil(110, 440, 700, 160, [0.055, 0.065, 0.090]);
        self.mustatil(110, 440, 700, 3, [0.20, 0.34, 0.52]);
    }

    /// Fills a rectangle, opaquely.
    fn mustatil(&mut self, yasar: u32, aala: u32, ard: u32, irtifa: u32, lawn: [f32; 3]) {
        for a in aala..aala.saturating_add(irtifa) {
            for s in yasar..yasar.saturating_add(ard) {
                if let Some(fahras) = self.fahras(s, a) {
                    self.biksel[fahras] = lawn[0];
                    self.biksel[fahras + 1] = lawn[1];
                    self.biksel[fahras + 2] = lawn[2];
                    self.biksel[fahras + 3] = 1.0;
                }
            }
        }
    }

    /// Draws a finished layout with the font's own outlines, and returns the
    /// pixel box the ink actually covered.
    ///
    /// This is *the game rendering its own text*. It goes nowhere near the
    /// overlay, the atlas or the batch — it exists so the recognized rectangle
    /// handed to the producer is a rectangle that real ink occupies rather than
    /// a number chosen to make the picture look right.
    fn utbua(
        &mut self,
        rassam: &Rassam,
        takhtit: &TakhtitNass,
        yasar: u32,
        aala: u32,
        lawn: [f32; 3],
    ) -> Result<MustatilBiksel, Box<dyn Error>> {
        let mut adna_s = i64::MAX;
        let mut adna_a = i64::MAX;
        let mut aqsa_s = i64::MIN;
        let mut aqsa_a = i64::MIN;

        for harf in &takhtit.huruf {
            let surah = rassam.irsim(harf.muarrif, takhtit.hajm, NamatRasm::Taghtiya, 0.0, &[])?;
            if surah.ard == 0 || surah.irtifa == 0 {
                continue;
            }
            let asl_s = i64::from(yasar) + (harf.s.round() as i64) + i64::from(surah.izaha_s);
            let asl_a = i64::from(aala) + (harf.a.round() as i64) - i64::from(surah.izaha_a);
            adna_s = adna_s.min(asl_s);
            adna_a = adna_a.min(asl_a);
            aqsa_s = aqsa_s.max(asl_s + i64::from(surah.ard));
            aqsa_a = aqsa_a.max(asl_a + i64::from(surah.irtifa));

            for saf in 0..surah.irtifa {
                for amud in 0..surah.ard {
                    let taghtiya = surah.bayt[(saf * surah.ard + amud) as usize];
                    if taghtiya == 0 {
                        continue;
                    }
                    let alfa = f32::from(taghtiya) / 255.0;
                    let s = asl_s + i64::from(amud);
                    let a = asl_a + i64::from(saf);
                    if s < 0 || a < 0 {
                        continue;
                    }
                    let Some(fahras) = self.fahras(s as u32, a as u32) else {
                        continue;
                    };
                    for (qanat, amam) in lawn.iter().enumerate() {
                        self.biksel[fahras + qanat] =
                            self.biksel[fahras + qanat].mul_add(1.0 - alfa, amam * alfa);
                    }
                }
            }
        }

        if adna_s == i64::MAX {
            return Ok(MustatilBiksel {
                yasar,
                aala,
                ard: 0,
                irtifa: 0,
            });
        }
        Ok(MustatilBiksel {
            yasar: adna_s.max(0) as u32,
            aala: adna_a.max(0) as u32,
            ard: (aqsa_s - adna_s).max(0) as u32,
            irtifa: (aqsa_a - adna_a).max(0) as u32,
        })
    }

    /// Encodes to sRGB and writes a PNG.
    fn uktub(&self, masar: &Path) -> Result<(), Box<dyn Error>> {
        let mut bayt = Vec::with_capacity((self.ard as usize) * (self.irtifa as usize) * 3);
        for qeema in self.biksel.chunks_exact(4) {
            for qanat in qeema.iter().take(3) {
                bayt.push((ila_sirgb(*qanat) * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        let surah: image::RgbImage = image::ImageBuffer::from_raw(self.ard, self.irtifa, bayt)
            .ok_or_else(|| {
                Box::<dyn Error>::from("the frame's byte count did not match its size")
            })?;
        surah.save(masar)?;
        Ok(())
    }
}

/// One linear channel encoded to sRGB.
///
/// The real piecewise transfer function, matching `rasm_tabaqa`'s decode and the
/// GL fragment stage's `ila_sirgb`. A 2.2 power here would make the picture
/// disagree with what a GPU writes in exactly the near-black range that
/// antialiased glyph edges live in.
fn ila_sirgb(khatti: f32) -> f32 {
    let amin = khatti.max(0.0);
    if amin <= 0.003_130_8 {
        amin * 12.92
    } else {
        1.055f32.mul_add(amin.powf(1.0 / 2.4), -0.055)
    }
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// A [`Khattaf`] that draws onto a pixel buffer instead of through a device.
///
/// It implements the trait's contract in full — describe the surface, build
/// resources against it, take an atlas, draw a batch, capture a region, release
/// — and its `irsim` is a transcription of `gl.rs`'s fragment stage. What it is
/// not is a graphics API, and nothing in this example claims otherwise.
#[derive(Debug)]
struct KhattafBarmaji {
    itar: Arc<Mutex<Itar>>,
    sath: WasfSath,
    /// The uploaded page: premultiplied RGBA8, its width, its height.
    lawha: Option<(Vec<u8>, u32, u32)>,
    /// How many batches have been drawn, so the example can say so.
    marsuma: u32,
}

impl KhattafBarmaji {
    /// A backend over a frame buffer.
    const fn jadeed(itar: Arc<Mutex<Itar>>, sath: WasfSath) -> Self {
        Self {
            itar,
            sath,
            lawha: None,
            marsuma: 0,
        }
    }

    /// One texel of the uploaded page, nearest-sampled.
    fn ayina(&self, u: f32, v: f32) -> [f32; 4] {
        let Some((bayt, ard, irtifa)) = self.lawha.as_ref() else {
            return [0.0; 4];
        };
        if *ard == 0 || *irtifa == 0 {
            return [0.0; 4];
        }
        let s = ((u * (*ard as f32)) as u32).min(ard.saturating_sub(1));
        let a = ((v * (*irtifa as f32)) as u32).min(irtifa.saturating_sub(1));
        let fahras = ((a as usize) * (*ard as usize) + (s as usize)) * 4;
        let Some(qita) = bayt.get(fahras..fahras + 4) else {
            return [0.0; 4];
        };
        [
            f32::from(qita[0]) / 255.0,
            f32::from(qita[1]) / 255.0,
            f32::from(qita[2]) / 255.0,
            f32::from(qita[3]) / 255.0,
        ]
    }
}

impl Khattaf for KhattafBarmaji {
    fn wajiha(&self) -> WajihatRusum {
        // There is no software variant of this enum and adding one would reach
        // into the capability report and the installer. `OpenGl` is what this
        // backend's blend and colour handling were transcribed from, and the
        // example says in its own header and its own output that no GL context
        // exists here.
        WajihatRusum::OpenGl
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        Ok(self.sath)
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        self.sath = sath;
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        // The same length check the GL backend makes, for the same reason: a
        // page whose bytes do not match its declared size would put arbitrary
        // memory on screen.
        let matlub = (ard as usize)
            .saturating_mul(irtifa as usize)
            .saturating_mul(4);
        if bayt.len() != matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "the atlas declares {ard}×{irtifa} RGBA8, which is {matlub} bytes, and {} \
                     bytes were supplied",
                    bayt.len()
                ),
            });
        }
        self.lawha = Some((bayt.to_vec(), ard, irtifa));
        Ok(())
    }

    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        let mut itar = self.itar.lock();
        for qita in &lawha.qitaat {
            let ard = qita.mawdi.ard.max(1);
            let irtifa = qita.mawdi.irtifa.max(1);
            for saf in 0..qita.mawdi.irtifa {
                for amud in 0..qita.mawdi.ard {
                    // `asas = mix(vec4(1.0), texture(lawha, uv), nasij)`.
                    let asas = match qita.khareeta {
                        Some(khareeta) => {
                            let u = khareeta.yasar
                                + khareeta.ard * ((amud as f32) + 0.5) / (ard as f32);
                            let v = khareeta.aala
                                + khareeta.irtifa * ((saf as f32) + 0.5) / (irtifa as f32);
                            self.ayina(u, v)
                        },
                        None => [1.0; 4],
                    };
                    // `mazuj = asas * lawn_marra`, both premultiplied.
                    let mazuj = [
                        asas[0] * qita.lawn[0],
                        asas[1] * qita.lawn[1],
                        asas[2] * qita.lawn[2],
                        asas[3] * qita.lawn[3],
                    ];
                    let s = qita.mawdi.yasar.saturating_add(amud);
                    let a = qita.mawdi.aala.saturating_add(saf);
                    let Some(fahras) = itar.fahras(s, a) else {
                        continue;
                    };
                    // `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)`.
                    let baqi = 1.0 - mazuj[3];
                    for (qanat, amam) in mazuj.iter().enumerate() {
                        itar.biksel[fahras + qanat] =
                            itar.biksel[fahras + qanat].mul_add(baqi, *amam);
                    }
                }
            }
        }
        self.marsuma = self.marsuma.saturating_add(1);
        Ok(())
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let itar = self.itar.lock();
        if mintaqa.yasar.saturating_add(mintaqa.ard) > itar.ard
            || mintaqa.aala.saturating_add(mintaqa.irtifa) > itar.irtifa
        {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: "capture".to_owned(),
                ard: itar.ard,
                irtifa: itar.irtifa,
            });
        }
        let mut kharj = Vec::with_capacity((mintaqa.ard as usize) * (mintaqa.irtifa as usize) * 4);
        for a in mintaqa.aala..mintaqa.aala + mintaqa.irtifa {
            for s in mintaqa.yasar..mintaqa.yasar + mintaqa.ard {
                let Some(fahras) = itar.fahras(s, a) else {
                    continue;
                };
                for qanat in 0..4 {
                    kharj.push(
                        (ila_sirgb(itar.biksel[fahras + qanat]) * 255.0)
                            .round()
                            .clamp(0.0, 255.0) as u8,
                    );
                }
            }
        }
        Ok(kharj)
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        self.lawha = None;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

/// The Arabic face this example draws with, and where it was found.
fn ijid_khatt() -> Result<(PathBuf, Vec<u8>), Box<dyn Error>> {
    const MURASHAHAT: &[&str] = &[
        "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf",
        "target/tajmee/makhbaa/sans/IBMPlexSansArabic-Regular.ttf",
        "assets/fonts/IBMPlexSansArabic-Regular.ttf",
    ];

    let mut jidhr: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    while let Some(dalil) = jidhr {
        for murashah in MURASHAHAT {
            let masar = dalil.join(murashah);
            if masar.is_file() {
                return Ok((masar.clone(), fs::read(&masar)?));
            }
        }
        jidhr = dalil.parent();
    }

    Err(Box::<dyn Error>::from(
        "no Arabic font found; expected apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf \
         somewhere at or above this crate",
    ))
}

/// Where the picture goes.
///
/// Beside the build output, which is already ignored. Not configurable through
/// the environment: `clippy.toml` disallows every way of reading it, because
/// configuration in this product is resolved by `taarib_usus::idadat` and not
/// read ad hoc — and an example is not a good enough reason to be the one place
/// that does.
fn masar_kharj() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/burhan")
        .join("talqeem_burhan.png")
}
