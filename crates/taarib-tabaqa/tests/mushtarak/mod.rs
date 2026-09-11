//! What the proof tests share: a frame a game could have drawn, a software
//! backend that draws over it exactly as `gl.rs`'s fragment stage would, and
//! the staged Arabic face.
//!
//! The backend keeps **two** frames. The game's own frame is what capture reads
//! back — a game redraws its backbuffer every frame, so the overlay never reads
//! its own previous draw — and the composite is what the overlay draws into
//! and what the PNG shows. A single frame would hand the recognizer the
//! overlay's Arabic on the second capture, which no real game does.

#![allow(
    clippy::redundant_pub_crate,
    reason = "this module is private to one test binary, so `pub` would be unreachable — which \
              the workspace denies — and `pub(crate)` is the only visibility that lets the test \
              reach these items; the lint's fix and the deny have no overlap here"
)]
#![allow(
    dead_code,
    reason = "the helpers are shared by the proof tests, and not every test uses every one"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use taarib_saff::rasm::{NamatRasm, Rassam};
use taarib_saff::{MawridKhatt, SilsilatKhutut, TakhtitNass};
use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::wajiha::{Khattaf, LawhatRasm, MustatilBiksel, WajihatRusum, WasfSath};

/// A linear-light RGBA frame buffer.
#[derive(Debug, Clone)]
pub(crate) struct Itar {
    pub(crate) ard: u32,
    pub(crate) irtifa: u32,
    /// Four linear floats per pixel, row-major from the top.
    pub(crate) biksel: Vec<f32>,
}

impl Itar {
    /// A black frame.
    #[must_use]
    pub(crate) fn jadeed(ard: u32, irtifa: u32) -> Self {
        let adad = usize::try_from(ard)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(irtifa).unwrap_or(0))
            .saturating_mul(4);
        Self {
            ard,
            irtifa,
            biksel: vec![0.0; adad],
        }
    }

    /// The index of a pixel's first channel, when it is on the frame.
    #[must_use]
    pub(crate) fn fahras(&self, s: u32, a: u32) -> Option<usize> {
        if s >= self.ard || a >= self.irtifa {
            return None;
        }
        let saf = usize::try_from(a).ok()?;
        let amud = usize::try_from(s).ok()?;
        let ard = usize::try_from(self.ard).ok()?;
        Some(saf.saturating_mul(ard).saturating_add(amud).saturating_mul(4))
    }

    /// One pixel's linear RGB, or black off the frame.
    #[must_use]
    pub(crate) fn lawn(&self, s: u32, a: u32) -> [f32; 3] {
        let Some(fahras) = self.fahras(s, a) else {
            return [0.0; 3];
        };
        self.biksel.get(fahras..fahras.saturating_add(3)).map_or([0.0; 3], |qanawat| {
            [
                qanawat.first().copied().unwrap_or(0.0),
                qanawat.get(1).copied().unwrap_or(0.0),
                qanawat.get(2).copied().unwrap_or(0.0),
            ]
        })
    }

    /// Writes one pixel, opaquely.
    pub(crate) fn uktub(&mut self, s: u32, a: u32, lawn: [f32; 3]) {
        let Some(fahras) = self.fahras(s, a) else {
            return;
        };
        if let Some(qanawat) = self.biksel.get_mut(fahras..fahras.saturating_add(4)) {
            qanawat.copy_from_slice(&[lawn[0], lawn[1], lawn[2], 1.0]);
        }
    }

    /// Blends one straight-alpha colour onto a pixel.
    pub(crate) fn imzij(&mut self, s: u32, a: u32, lawn: [f32; 3], alfa: f32) {
        let Some(fahras) = self.fahras(s, a) else {
            return;
        };
        for (qanat, amam) in lawn.iter().enumerate() {
            if let Some(qeema) = self.biksel.get_mut(fahras.saturating_add(qanat)) {
                *qeema = qeema.mul_add(1.0 - alfa, amam * alfa);
            }
        }
    }

    /// Paints a dark scene with a darker dialogue panel across the lower half.
    ///
    /// No bright strip and no bright art: the frame exists so a recognizer's
    /// segmentation has exactly the text to find, and a fixture whose panel
    /// border read as a line of text would be testing the fixture.
    pub(crate) fn mashhad(&mut self) {
        for a in 0..self.irtifa {
            let nisba = madaa(a) / madaa(self.irtifa.max(1));
            for s in 0..self.ard {
                let ufuqi = madaa(s) / madaa(self.ard.max(1));
                let lawn = [
                    ufuqi.mul_add(0.010, nisba.mul_add(0.040, 0.015)),
                    nisba.mul_add(0.050, 0.020),
                    nisba.mul_add(0.080, 0.035),
                ];
                self.uktub(s, a, lawn);
            }
        }
        self.mustatil(90, 400, 760, 260, [0.050, 0.055, 0.075]);
    }

    /// Fills a rectangle, opaquely.
    pub(crate) fn mustatil(&mut self, yasar: u32, aala: u32, ard: u32, irtifa: u32, lawn: [f32; 3]) {
        for a in aala..aala.saturating_add(irtifa) {
            for s in yasar..yasar.saturating_add(ard) {
                self.uktub(s, a, lawn);
            }
        }
    }

    /// Draws a finished layout with the font's own outlines and returns the
    /// pixel box the ink covered.
    ///
    /// This is the game rendering its own text; it goes nowhere near the
    /// overlay.
    ///
    /// # Errors
    ///
    /// Whatever the rasterizer refuses.
    pub(crate) fn utbua(
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
            let asl_s = i64::from(yasar) + tadwir(harf.s) + i64::from(surah.izaha_s);
            let asl_a = i64::from(aala) + tadwir(harf.a) - i64::from(surah.izaha_a);
            adna_s = adna_s.min(asl_s);
            adna_a = adna_a.min(asl_a);
            aqsa_s = aqsa_s.max(asl_s + i64::from(surah.ard));
            aqsa_a = aqsa_a.max(asl_a + i64::from(surah.irtifa));

            for saf in 0..surah.irtifa {
                for amud in 0..surah.ard {
                    let mawdi = usize::try_from(saf.saturating_mul(surah.ard).saturating_add(amud))
                        .unwrap_or(usize::MAX);
                    let taghtiya = surah.bayt.get(mawdi).copied().unwrap_or(0);
                    if taghtiya == 0 {
                        continue;
                    }
                    let alfa = f32::from(taghtiya) / 255.0;
                    let s = asl_s + i64::from(amud);
                    let a = asl_a + i64::from(saf);
                    let (Ok(s), Ok(a)) = (u32::try_from(s), u32::try_from(a)) else {
                        continue;
                    };
                    self.imzij(s, a, lawn, alfa);
                }
            }
        }

        if adna_s == i64::MAX {
            return Err(Box::<dyn Error>::from("the layout produced no ink"));
        }
        Ok(MustatilBiksel {
            yasar: u32::try_from(adna_s.max(0)).unwrap_or(0),
            aala: u32::try_from(adna_a.max(0)).unwrap_or(0),
            ard: u32::try_from((aqsa_s - adna_s).max(0)).unwrap_or(0),
            irtifa: u32::try_from((aqsa_a - adna_a).max(0)).unwrap_or(0),
        })
    }

    /// The frame as tightly packed sRGB-encoded RGBA8 for a rectangle.
    #[must_use]
    pub(crate) fn rgba8(&self, mintaqa: MustatilBiksel) -> Vec<u8> {
        let mut kharj = Vec::with_capacity(
            usize::try_from(mintaqa.ard)
                .unwrap_or(0)
                .saturating_mul(usize::try_from(mintaqa.irtifa).unwrap_or(0))
                .saturating_mul(4),
        );
        for a in mintaqa.aala..mintaqa.aala.saturating_add(mintaqa.irtifa) {
            for s in mintaqa.yasar..mintaqa.yasar.saturating_add(mintaqa.ard) {
                let Some(fahras) = self.fahras(s, a) else {
                    continue;
                };
                for qanat in 0..4 {
                    let qeema = self
                        .biksel
                        .get(fahras.saturating_add(qanat))
                        .copied()
                        .unwrap_or(0.0);
                    kharj.push(bayt(ila_sirgb(qeema)));
                }
            }
        }
        kharj
    }

    /// Encodes to sRGB and writes a PNG.
    ///
    /// # Errors
    ///
    /// Whatever the encoder or the file system refuses.
    pub(crate) fn ihfaz(&self, masar: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid)?;
        }
        let mut bayt_kull = Vec::with_capacity(
            usize::try_from(self.ard)
                .unwrap_or(0)
                .saturating_mul(usize::try_from(self.irtifa).unwrap_or(0))
                .saturating_mul(3),
        );
        for qeema in self.biksel.chunks_exact(4) {
            for qanat in qeema.iter().take(3) {
                bayt_kull.push(bayt(ila_sirgb(*qanat)));
            }
        }
        let surah: image::RgbImage =
            image::ImageBuffer::from_raw(self.ard, self.irtifa, bayt_kull).ok_or_else(|| {
                Box::<dyn Error>::from("the frame's byte count did not match its size")
            })?;
        surah.save(masar)?;
        Ok(())
    }
}

/// A float pen offset as a whole pixel.
const fn tadwir(qeema: f32) -> i64 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "pen offsets are surface pixels, far inside i64"
    )]
    {
        qeema.round() as i64
    }
}

/// A linear channel encoded to sRGB.
#[must_use]
pub(crate) fn ila_sirgb(khatti: f32) -> f32 {
    let amin = khatti.max(0.0);
    if amin <= 0.003_130_8 {
        amin * 12.92
    } else {
        1.055f32.mul_add(amin.powf(1.0 / 2.4), -0.055)
    }
}

/// A zero-to-one channel as a byte.
fn bayt(qeema: f32) -> u8 {
    let mahdud = (qeema * 255.0).round().clamp(0.0, 255.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, 255] and rounded immediately above"
    )]
    {
        mahdud as u8
    }
}

/// A dimension as a float.
const fn madaa(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// A [`Khattaf`] over two frames: the game's, read by capture, and the
/// composite, drawn by the overlay.
#[derive(Debug)]
pub(crate) struct KhattafBarmaji {
    /// The frame the game drew, which capture reads.
    pub(crate) asl: Arc<Mutex<Itar>>,
    /// The frame the overlay draws into, reset from the game's on every draw.
    pub(crate) murakkab: Arc<Mutex<Itar>>,
    sath: WasfSath,
    lawha: Option<(Vec<u8>, u32, u32)>,
    /// How many batches have been drawn.
    pub(crate) marsuma: u32,
    /// How many captures have been read back.
    pub(crate) multaqata: u32,
}

impl KhattafBarmaji {
    /// A backend over a game frame.
    #[must_use]
    pub(crate) fn jadeed(asl: Arc<Mutex<Itar>>, sath: WasfSath) -> Self {
        let murakkab = Arc::new(Mutex::new(asl.lock().clone()));
        Self {
            asl,
            murakkab,
            sath,
            lawha: None,
            marsuma: 0,
            multaqata: 0,
        }
    }

    /// One texel of the uploaded page, nearest-sampled.
    fn ayina(&self, u: f32, v: f32) -> [f32; 4] {
        let Some((bayt_lawha, ard, irtifa)) = self.lawha.as_ref() else {
            return [0.0; 4];
        };
        if *ard == 0 || *irtifa == 0 {
            return [0.0; 4];
        }
        let s = ihdathiya(u, *ard);
        let a = ihdathiya(v, *irtifa);
        let fahras = usize::try_from(a)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(*ard).unwrap_or(0))
            .saturating_add(usize::try_from(s).unwrap_or(0))
            .saturating_mul(4);
        let Some(qita) = bayt_lawha.get(fahras..fahras.saturating_add(4)) else {
            return [0.0; 4];
        };
        [
            f32::from(qita.first().copied().unwrap_or(0)) / 255.0,
            f32::from(qita.get(1).copied().unwrap_or(0)) / 255.0,
            f32::from(qita.get(2).copied().unwrap_or(0)) / 255.0,
            f32::from(qita.get(3).copied().unwrap_or(0)) / 255.0,
        ]
    }
}

/// A normalized texture coordinate as a texel index inside a page.
fn ihdathiya(nisba: f32, madaa_safha: u32) -> u32 {
    let khaam = (nisba.clamp(0.0, 1.0) * madaa(madaa_safha)).floor();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, page side] immediately above"
    )]
    {
        (khaam as u32).min(madaa_safha.saturating_sub(1))
    }
}

impl Khattaf for KhattafBarmaji {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::OpenGl
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        Ok(self.sath)
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        self.sath = sath;
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt_lawha: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        let matlub = usize::try_from(ard)
            .unwrap_or(0)
            .saturating_mul(usize::try_from(irtifa).unwrap_or(0))
            .saturating_mul(4);
        if bayt_lawha.len() != matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "the atlas declares {ard}×{irtifa} RGBA8, which is {matlub} bytes, and {} \
                     bytes were supplied",
                    bayt_lawha.len()
                ),
            });
        }
        self.lawha = Some((bayt_lawha.to_vec(), ard, irtifa));
        Ok(())
    }

    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        // The game redrew its frame; the overlay draws over that, not over its
        // own previous draw.
        let asl = self.asl.lock().clone();
        let mut itar = self.murakkab.lock();
        *itar = asl;
        for qita in &lawha.qitaat {
            let ard = qita.mawdi.ard.max(1);
            let irtifa = qita.mawdi.irtifa.max(1);
            for saf in 0..qita.mawdi.irtifa {
                for amud in 0..qita.mawdi.ard {
                    let asas = match qita.khareeta {
                        Some(khareeta) => {
                            let u = khareeta.yasar
                                + khareeta.ard * (madaa(amud) + 0.5) / madaa(ard);
                            let v = khareeta.aala
                                + khareeta.irtifa * (madaa(saf) + 0.5) / madaa(irtifa);
                            self.ayina(u, v)
                        },
                        None => [1.0; 4],
                    };
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
                    let baqi = 1.0 - mazuj[3];
                    for (qanat, amam) in mazuj.iter().enumerate() {
                        if let Some(qeema) = itar.biksel.get_mut(fahras.saturating_add(qanat)) {
                            *qeema = qeema.mul_add(baqi, *amam);
                        }
                    }
                }
            }
        }
        self.marsuma = self.marsuma.saturating_add(1);
        Ok(())
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let asl = self.asl.lock();
        if mintaqa.yasar.saturating_add(mintaqa.ard) > asl.ard
            || mintaqa.aala.saturating_add(mintaqa.irtifa) > asl.irtifa
        {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: "capture".to_owned(),
                ard: asl.ard,
                irtifa: asl.irtifa,
            });
        }
        self.multaqata = self.multaqata.saturating_add(1);
        Ok(asl.rgba8(mintaqa))
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        self.lawha = None;
        Ok(())
    }
}

/// The staged Arabic face, found at or above this crate.
///
/// # Errors
///
/// When no face is staged, which names where one was expected.
pub(crate) fn khutut() -> Result<(SilsilatKhutut, Arc<MawridKhatt>, PathBuf), Box<dyn Error>> {
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
                let khatt = Arc::new(MawridKhatt::jadeed(Arc::new(fs::read(&masar)?), 0)?);
                let silsila = SilsilatKhutut::wahid(Arc::clone(&khatt))?;
                return Ok((silsila, khatt, masar));
            }
        }
        jidhr = dalil.parent();
    }
    Err(Box::<dyn Error>::from(
        "no Arabic font found; expected apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf \
         somewhere at or above this crate",
    ))
}
