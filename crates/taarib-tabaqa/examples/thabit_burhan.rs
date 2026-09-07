//! برهان الخط الثابت — the fixed-function composite, drawn offscreen.
//!
//! [`talqeem_burhan`] proves the shaping, the atlas and the batch through a
//! software backend transcribed from the **modern** OpenGL fragment stage,
//! blending premultiplied *linear* colour into a linear buffer. That is the
//! right transcription for the four backends it was written for and it is the
//! wrong one for the two this example exists for.
//!
//! `crate::d3d8` and `crate::gl_thabit` draw through pipelines that have no
//! shader and therefore nowhere to put a transfer function. Their colour is
//! converted on the CPU by [`taarib_tabaqa::d3d8::lawn_marmuz`] — divide the
//! alpha out, encode, multiply it back — and the blend that follows happens in
//! the framebuffer's own **encoding**, which is the space the games of that
//! generation blended their own interfaces in. That decision is the single
//! most visible thing about those two backends and the one thing no unit test
//! can show, so this example draws it.
//!
//! ## What this is and is not
//!
//! [`KhattafThabit`] is a **software** [`Khattaf`]. It is not Direct3D 8 and it
//! is not OpenGL, and this example hooks nothing. What it reproduces is exactly
//! the arithmetic those two backends configure, in the order they configure it:
//!
//! * the frame buffer holds **sRGB-encoded bytes**, which is what a Direct3D 8
//!   `X8R8G8B8` backbuffer and a fixed-function OpenGL visual both hold;
//! * every quad's colour goes through `lawn_marmuz`, the real function both
//!   backends call;
//! * the texel and the colour are combined by multiplication, which is
//!   `D3DTOP_MODULATE` and `GL_MODULATE`;
//! * a plate takes the colour alone, which is `D3DTOP_SELECTARG2` and
//!   `glDisable(GL_TEXTURE_2D)`;
//! * the result is composited `ONE, INV_SRC_ALPHA`, which is
//!   `D3DRS_SRCBLEND`/`D3DRS_DESTBLEND` and `glBlendFunc`.
//!
//! Every other link in the chain — the shaper, the runtime atlas, the batch
//! builder, the [`Tabaqa`] frame rules, the disclosure gate, the [`Khattaf`]
//! contract — is the shipping one.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p taarib-tabaqa --example thabit_burhan
//! ```

#![allow(
    clippy::print_stdout,
    reason = "this example exists to report what the fixed-function path produced"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel coordinates are deliberately narrowed to integers, with the ranges checked"
)]
#![allow(
    clippy::indexing_slicing,
    reason = "every index here is into a four-element slice that was taken by `get(..+ 4)` on \
              the line above it"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use taarib_saff::{MawridKhatt, SilsilatKhutut};
use taarib_tabaqa::d3d8::lawn_marmuz;
use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::lawhat_tahakkum::LawhatTahakkum;
use taarib_tabaqa::sidq::{BasmatIfsah, Iqrar};
use taarib_tabaqa::talqeem::{KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
use taarib_tabaqa::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, SighatSath, Tabaqa, WajihatRusum, WasfSath,
};

/// The surface the whole example runs at.
const ARD: u32 = 1280;
/// The surface height.
const IRTIFA: u32 = 720;

/// One translated line and the rectangle a recognizer would have read it from.
struct Satr {
    /// The Arabic, in logical order, as ordinary Unicode.
    arabi: &'static str,
    /// Where the line the translation replaces was.
    sunduq: MustatilBiksel,
}

/// The dialogue this frame carries.
///
/// The rectangles are stated rather than measured, and that is the one thing
/// this example does *less* than `talqeem_burhan`: it does not draw a line of
/// English with the font's own outlines first and then read its ink box. What
/// is being proved here is the colour and the blend, and those do not depend on
/// where the box came from.
const SUTUR: &[Satr] = &[
    Satr {
        arabi: "الطريق الشمالي مغلق حتى ذوبان الثلج.",
        sunduq: MustatilBiksel {
            yasar: 150,
            aala: 470,
            ard: 430,
            irtifa: 22,
        },
    },
    Satr {
        arabi: "اسلك الممر أسفل برج المراقبة القديم.",
        sunduq: MustatilBiksel {
            yasar: 150,
            aala: 512,
            ard: 420,
            irtifa: 22,
        },
    },
    Satr {
        arabi: "سأنتظرك عند الجسر.",
        sunduq: MustatilBiksel {
            yasar: 150,
            aala: 554,
            ard: 230,
            irtifa: 22,
        },
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let (masar_khatt, bayt) = ijid_khatt()?;
    println!("font: {}", masar_khatt.display());

    let khatt = Arc::new(MawridKhatt::jadeed(Arc::new(bayt), 0)?);
    let khutut = SilsilatKhutut::wahid(Arc::clone(&khatt))?;

    // -- the frame the game presented -------------------------------------
    let mut itar = ItarThabit::jadeed(ARD, IRTIFA);
    itar.mashhad();

    // -- the surface, described the way both fixed-function backends do ----
    //
    // `sirgb: false` is not a default. It is the operational statement both
    // backends make: nothing in either pipeline will apply a transfer function
    // to what the overlay writes, so the overlay applies it itself.
    let sath = WasfSath {
        ard: ARD,
        irtifa: IRTIFA,
        sigha: SighatSath::Bgra8,
        sirgb: false,
    };

    let mut lawha_tahakkum = LawhatTahakkum::jadeeda("برهان الخط الثابت");
    lawha_tahakkum.hala_mut().irfa();
    let ansur = lawha_tahakkum.bina(sath);
    println!(
        "control panel: {} elements from LawhatTahakkum::bina",
        ansur.len()
    );

    let mulaqqama: Vec<SatrMulaqqam> = SUTUR
        .iter()
        .map(|satr| SatrMulaqqam {
            nass: satr.arabi.to_owned(),
            mawdi: satr.sunduq,
            thiqa: 94,
        })
        .collect();

    // -- the producer and the overlay -------------------------------------
    let mut mulaqqim = Mulaqqim::jadeed(khutut, KhiyaratTalqeem::iftiradiya())?;
    let lahza = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |mudda| mudda.as_secs());
    let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), lahza, "برهان الخط الثابت")?;

    let musattah = Arc::new(Mutex::new(itar));
    let khattaf = KhattafThabit::jadeed(Arc::clone(&musattah), sath);
    let mut tabaqa = Tabaqa::shaghghil(Box::new(khattaf), iqrar)?;
    for satr in tabaqa.athar() {
        println!("overlay: {satr}");
    }

    let Some(sath_hayy) = tabaqa.sath() else {
        return Err(Box::<dyn Error>::from(
            "the overlay reported no live surface",
        ));
    };
    let dufaa = mulaqqim.qaddim(&mut tabaqa, sath_hayy, &mulaqqama, &ansur, 0)?;

    let ihsaat = mulaqqim.ihsaat();
    println!("\n=== the batch ===");
    println!(
        "quads {} ({} textured, {} solid) from {} lines and {} panel elements",
        dufaa.qitaat.len(),
        dufaa
            .qitaat
            .iter()
            .filter(|qita| qita.khareeta.is_some())
            .count(),
        dufaa
            .qitaat
            .iter()
            .filter(|qita| qita.khareeta.is_none())
            .count(),
        ihsaat.sutur,
        ihsaat.ansur
    );
    println!("glyphs the atlas refused: {}", ihsaat.ashkal_mafquda);

    // -- what the colour conversion actually did --------------------------
    utbua_alwan();

    let masar = masar_kharj();
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid)?;
    }
    musattah.lock().uktub(&masar)?;
    println!("\nwrote {}", masar.display());
    println!(
        "reached: batch produced -> atlas uploaded -> handed to Tabaqa::itar -> composited \
         offscreen through the fixed-function arithmetic both older backends configure. Not \
         drawn through Direct3D 8, not drawn through OpenGL, and not drawn over a running game."
    );
    Ok(())
}

/// Prints what `lawn_marmuz` does to three colours that matter.
fn utbua_alwan() {
    println!("\n=== the conversion the fixed-function backends make on the CPU ===");
    for (ism, khaam) in [
        ("opaque mid grey", [0.5, 0.5, 0.5, 1.0]),
        ("half-covered white", [0.5, 0.5, 0.5, 0.5]),
        ("quarter-covered white", [0.25, 0.25, 0.25, 0.25]),
    ] {
        let marmuz = lawn_marmuz(khaam);
        println!(
            "  {ism:<22} premultiplied linear {khaam:?} -> premultiplied encoded [{:.3}, {:.3}, \
             {:.3}, {:.3}]",
            marmuz[0], marmuz[1], marmuz[2], marmuz[3]
        );
    }
    println!(
        "  the second and third keep their alpha as their colour, which is what an antialiased \
         glyph edge must do; a conversion that skipped the un-premultiply would report the first \
         row's numbers for all three"
    );
}

// ---------------------------------------------------------------------------
// The frame, in the space a fixed-function pipeline actually blends in
// ---------------------------------------------------------------------------

/// An sRGB-encoded RGBA8 frame buffer.
///
/// Encoded, not linear, and that is the entire point of this example. A
/// Direct3D 8 `X8R8G8B8` backbuffer and a fixed-function OpenGL visual both
/// hold values a display reads as sRGB, and neither pipeline has a stage that
/// could linearise them. Every blend below therefore happens on the encoded
/// bytes — which is physically wrong and is exactly what the game's own
/// interface was blended with, and matching the game is the target.
struct ItarThabit {
    ard: u32,
    irtifa: u32,
    /// Four bytes per pixel, row-major from the top, red first.
    biksel: Vec<u8>,
}

impl ItarThabit {
    /// A black frame.
    fn jadeed(ard: u32, irtifa: u32) -> Self {
        let adad = (ard as usize) * (irtifa as usize) * 4;
        Self {
            ard,
            irtifa,
            biksel: vec![0; adad],
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
    fn mashhad(&mut self) {
        for a in 0..self.irtifa {
            let nisba = (a as f32) / (self.irtifa as f32);
            for s in 0..self.ard {
                let ufuqi = (s as f32) / (self.ard as f32);
                let lawn = [
                    ufuqi.mul_add(0.06, nisba.mul_add(0.22, 0.10)),
                    nisba.mul_add(0.26, 0.12),
                    nisba.mul_add(0.32, 0.16),
                ];
                self.biksel_ida(s, a, lawn);
            }
        }
        // The game's own dialogue panel and its rule.
        self.mustatil(110, 440, 700, 160, [0.16, 0.18, 0.24]);
        self.mustatil(110, 440, 700, 3, [0.35, 0.55, 0.78]);
    }

    /// Writes one opaque pixel from already-encoded channels.
    fn biksel_ida(&mut self, s: u32, a: u32, lawn: [f32; 3]) {
        let Some(fahras) = self.fahras(s, a) else {
            return;
        };
        for (qanat, qeema) in lawn.iter().enumerate() {
            if let Some(makan) = self.biksel.get_mut(fahras + qanat) {
                *makan = (qeema.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        if let Some(makan) = self.biksel.get_mut(fahras + 3) {
            *makan = 0xFF;
        }
    }

    /// Fills a rectangle, opaquely.
    fn mustatil(&mut self, yasar: u32, aala: u32, ard: u32, irtifa: u32, lawn: [f32; 3]) {
        for a in aala..aala.saturating_add(irtifa) {
            for s in yasar..yasar.saturating_add(ard) {
                self.biksel_ida(s, a, lawn);
            }
        }
    }

    /// Writes a PNG.
    ///
    /// No conversion on the way out, unlike `talqeem_burhan`: the buffer is
    /// already in the space a PNG stores.
    fn uktub(&self, masar: &Path) -> Result<(), Box<dyn Error>> {
        let mut bayt = Vec::with_capacity((self.ard as usize) * (self.irtifa as usize) * 3);
        for qeema in self.biksel.chunks_exact(4) {
            bayt.extend(qeema.iter().take(3).copied());
        }
        let surah: image::RgbImage = image::ImageBuffer::from_raw(self.ard, self.irtifa, bayt)
            .ok_or_else(|| {
                Box::<dyn Error>::from("the frame's byte count did not match its size")
            })?;
        surah.save(masar)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// A [`Khattaf`] that reproduces the fixed-function arithmetic in software.
struct KhattafThabit {
    itar: Arc<Mutex<ItarThabit>>,
    sath: WasfSath,
    /// The uploaded page: premultiplied RGBA8, its width, its height.
    lawha: Option<(Vec<u8>, u32, u32)>,
}

impl std::fmt::Debug for KhattafThabit {
    fn fmt(&self, mukhraj: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        mukhraj
            .debug_struct("KhattafThabit")
            .field("sath", &self.sath)
            .field(
                "lawha",
                &self.lawha.as_ref().map(|(_, ard, irtifa)| (*ard, *irtifa)),
            )
            .finish_non_exhaustive()
    }
}

impl KhattafThabit {
    /// A backend over a frame buffer.
    const fn jadeed(itar: Arc<Mutex<ItarThabit>>, sath: WasfSath) -> Self {
        Self {
            itar,
            sath,
            lawha: None,
        }
    }

    /// One texel of the uploaded page, nearest-sampled, as zero-to-one floats.
    ///
    /// Nearest rather than linear, and it costs nothing here: both backends ask
    /// for linear filtering, and the batch draws each glyph at the size it was
    /// rasterized at, so every sample lands on a texel centre either way.
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

impl Khattaf for KhattafThabit {
    fn wajiha(&self) -> WajihatRusum {
        // There is no software variant of this enum and adding one would reach
        // into the capability report and the installer. The Direct3D 8 spelling
        // is what this backend's colour handling was transcribed from — the
        // fixed-function OpenGL one is identical — and the example says in its
        // own header and its own output that no device exists here.
        WajihatRusum::Direct3D8
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        Ok(self.sath)
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        self.sath = sath;
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        // The same length check both real backends make, for the same reason:
        // a page whose bytes do not match its declared size would put arbitrary
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
            // The one call this whole example exists to exercise. Both
            // backends make it per quad, on the CPU, because neither pipeline
            // has a fragment stage to make it in.
            let marra = lawn_marmuz(qita.lawn);

            for saf in 0..qita.mawdi.irtifa {
                for amud in 0..qita.mawdi.ard {
                    // `D3DTOP_MODULATE` with `TEXTURE` and `DIFFUSE`, or
                    // `GL_MODULATE`; a plate takes `D3DTOP_SELECTARG2`, which
                    // is the vertex colour alone.
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
                    let mazuj = [
                        asas[0] * marra[0],
                        asas[1] * marra[1],
                        asas[2] * marra[2],
                        asas[3] * marra[3],
                    ];

                    let s = qita.mawdi.yasar.saturating_add(amud);
                    let a = qita.mawdi.aala.saturating_add(saf);
                    let Some(fahras) = itar.fahras(s, a) else {
                        continue;
                    };
                    // `D3DBLEND_ONE, D3DBLEND_INVSRCALPHA`, or
                    // `glBlendFunc(GL_ONE, GL_ONE_MINUS_SRC_ALPHA)` — over the
                    // encoded destination, because that is the only destination
                    // either pipeline has.
                    let baqi = 1.0 - mazuj[3];
                    for (qanat, amam) in mazuj.iter().enumerate() {
                        let Some(makan) = itar.biksel.get_mut(fahras + qanat) else {
                            continue;
                        };
                        let khalfiya = f32::from(*makan) / 255.0;
                        *makan =
                            (khalfiya.mul_add(baqi, *amam).clamp(0.0, 1.0) * 255.0).round() as u8;
                    }
                }
            }
        }
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
                let Some(qita) = itar.biksel.get(fahras..fahras + 4) else {
                    continue;
                };
                // Blue first, which is what `SighatSath::Bgra8` means and what
                // a Direct3D 8 `A8R8G8B8` surface holds in memory.
                kharj.extend_from_slice(&[qita[2], qita[1], qita[0], qita[3]]);
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
fn masar_kharj() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/burhan")
        .join("thabit_burhan.png")
}
