//! برهان دايركت٣د ٩ — the Direct3D 9 path drawn, as far as this machine allows.
//!
//! `talqeem_burhan.rs` proved the shared half of tier 3: shaping, the runtime
//! atlas, the batch, the [`Tabaqa`] frame rules and the disclosure gate, drawn
//! offscreen through a software [`Khattaf`] that transcribes `gl.rs`'s fragment
//! stage. Everything it proved is upstream of a backend and none of it is
//! Direct3D 9.
//!
//! This one proves the half that is. It runs the same pipeline and then draws it
//! **twice**:
//!
//! 1. through [`KhattafBarnamaji`], a software backend that transcribes the
//!    *Direct3D 9 fixed-function stage* — [`lawn_d3d`]'s real conversion, the
//!    texture cascade's two operations, and `ONE, INV_SRC_ALPHA` over a
//!    backbuffer holding sRGB-encoded values;
//! 2. through [`KhattafD3D9`] itself against a real `IDirect3DDevice9`, if this
//!    machine has one.
//!
//! The second is attempted rather than assumed, and what happened is printed
//! either way. Under WSL there is no display adapter and `Direct3DCreate9`
//! answers nothing, so the honest ceiling there is the first — **drawn offscreen
//! through the real contract**, which is the rung `talqeem_burhan.rs` reached.
//! Run from Windows against a real adapter it reaches one rung higher: **drawn
//! by the real backend through a real device**, with the state block captured
//! and re-applied around it and the result read back off the GPU. Neither is
//! *drawn over a running game*, and this example never claims to be.
//!
//! ```text
//! cargo run -p taarib-tabaqa --example d3d9_burhan
//! ```

#![allow(
    clippy::print_stdout,
    reason = "this example exists to report what the Direct3D 9 path produced"
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

#[cfg(not(windows))]
fn main() {
    println!(
        "the Direct3D 9 backend is Windows-only, so this proof does not run on this platform. \
         The shared half of the same pipeline is proved by `talqeem_burhan`, which does."
    );
}

#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    windows_only::ishtaghil()
}

#[cfg(windows)]
mod windows_only {
    use std::error::Error;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use parking_lot::Mutex;
    use taarib_saff::{MawridKhatt, SilsilatKhutut};
    use taarib_tabaqa::d3d9::lawn_d3d;
    use taarib_tabaqa::khata::KhataTabaqa;
    use taarib_tabaqa::sidq::{BasmatIfsah, Iqrar, NASS_IFSAH_ARABI};
    use taarib_tabaqa::talqeem::{KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
    use taarib_tabaqa::d3d9::KhattafD3D9;
    use taarib_tabaqa::istitlaa::{HalatSlot, istatli};
    use taarib_tabaqa::khataf::nafidha_muaqqata;
    use taarib_tabaqa::wajiha::{
        Khattaf, LawhatRasm, MustatilBiksel, MustatilNisbi, SighatSath, Tabaqa, WajihatRusum,
        WasfSath,
    };
    use windows::Win32::Graphics::Direct3D9::{
        D3D_SDK_VERSION, D3DADAPTER_DEFAULT, D3DCREATE_FPU_PRESERVE, D3DCREATE_MULTITHREADED,
        D3DCREATE_NOWINDOWCHANGES, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL,
        D3DCLEAR_TARGET, D3DFMT_X8R8G8B8, D3DPRESENT_PARAMETERS, D3DRS_CULLMODE,
        D3DSWAPEFFECT_DISCARD,
        Direct3DCreate9, IDirect3DDevice9,
    };
    use windows::core::BOOL;

    /// The surface the whole proof runs at.
    const ARD: u32 = 960;
    /// The surface height.
    const IRTIFA: u32 = 540;

    /// The lines the recognizer is standing in for, and their Arabic.
    ///
    /// Ordinary Unicode in logical order — no presentation forms. Every joined
    /// letterform in the output picture is the font's own `GSUB` at draw time.
    const HIWAR: &[(&str, u32, u32, u32)] = &[
        ("الطريق الشمالي مغلق حتى ذوبان الثلج.", 120, 380, 420),
        ("اسلك الممر أسفل برج المراقبة القديم.", 120, 416, 420),
        ("سأنتظرك عند الجسر.", 120, 452, 260),
    ];

    /// Runs both halves and reports how far each reached.
    ///
    /// A path given on the command line is surveyed first and nothing else runs.
    /// That is the pre-install half of the same question — see
    /// [`taarib_tabaqa::istitlaa`] — and it is reachable from here so that it can
    /// be pointed at a real game directory without a second binary existing:
    ///
    /// ```text
    /// cargo run -p taarib-tabaqa --example d3d9_burhan -- "F:/.../bio4.exe"
    /// ```
    #[expect(
        clippy::redundant_pub_crate,
        reason = "the lint's own fix does not compile here: this module is private, so widening \
                  to `pub` makes the item unreachable outside it and the workspace denies \
                  `unreachable_pub`. Every narrower visibility is what this lint objects to, so \
                  the two rules have no overlap and the deny is the one that must win"
    )]
    pub(super) fn ishtaghil() -> Result<(), Box<dyn Error>> {
        if let Some(matlub) = std::env::args_os().nth(1) {
            return istatli_luba(Path::new(&matlub));
        }
        let (masar_khatt, bayt) = ijid_khatt()?;
        println!("font: {}", masar_khatt.display());

        let khatt = Arc::new(MawridKhatt::jadeed(Arc::new(bayt), 0)?);
        let khutut = SilsilatKhutut::wahid(Arc::clone(&khatt))?;
        let mut mulaqqim = Mulaqqim::jadeed(khutut, KhiyaratTalqeem::iftiradiya())?;

        let sath = WasfSath {
            ard: ARD,
            irtifa: IRTIFA,
            sigha: SighatSath::Bgra8,
            // What a Direct3D 9 backbuffer is: no `_SRGB` format exists on this
            // API and the backend leaves `D3DRS_SRGBWRITEENABLE` off, so nothing
            // downstream encodes and `lawn_d3d` does it instead.
            sirgb: false,
        };
        let sutur: Vec<SatrMulaqqam> = HIWAR
            .iter()
            .map(|(nass, yasar, aala, ard)| SatrMulaqqam {
                nass: (*nass).to_owned(),
                mawdi: MustatilBiksel { yasar: *yasar, aala: *aala, ard: *ard, irtifa: 24 },
                thiqa: 94,
            })
            .collect();

        println!("\n--- the tier-3 disclosure, shown because the type system requires it ---");
        println!("{NASS_IFSAH_ARABI}");
        println!("--- end of disclosure ---\n");
        let lahza =
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |mudda| mudda.as_secs());

        // -- 1. the fixed-function stage, transcribed ------------------------
        let itar = Arc::new(Mutex::new(Itar::jadeed(ARD, IRTIFA)));
        itar.lock().mashhad();
        let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), lahza, "برهان دايركت٣د ٩")?;
        let mut tabaqa =
            Tabaqa::shaghghil(Box::new(KhattafBarnamaji::jadeed(Arc::clone(&itar), sath)), iqrar)?;
        let Some(hayy) = tabaqa.sath() else {
            return Err(Box::<dyn Error>::from("the overlay reported no live surface"));
        };
        let dufaa = mulaqqim.qaddim(&mut tabaqa, hayy, &sutur, &[], 0)?;
        utbua_dufaa(&dufaa);

        let masar = masar_kharj("d3d9_burhan.png");
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid)?;
        }
        itar.lock().uktub(&masar)?;
        println!("\nwrote {}", masar.display());
        println!(
            "reached: batch produced -> atlas uploaded -> handed to Tabaqa::itar -> drawn \
             offscreen through the Direct3D 9 fixed-function stage transcribed exactly. Not \
             drawn over a running game."
        );

        // -- 2. the real backend, if this machine has an adapter -------------
        println!("\n=== the real device ===");
        match jihaz_haqiqi() {
            Ok(jihaz) => barhin_jihaz(&jihaz, &mut mulaqqim, &sutur)?,
            Err(sabab) => {
                println!("no Direct3D 9 device on this machine: {sabab}");
                println!(
                    "reached: nothing beyond the offscreen rung above. The real backend was not \
                     exercised against a device and this run does not claim it was."
                );
            },
        }
        Ok(())
    }

    /// Surveys one game's files and prints the verdict and every reason.
    ///
    /// Nothing is written and nothing is installed. This is the answer the
    /// installer gets *before* it writes anything, and the reason it exists is
    /// the proxy slot: Taarib takes one name in a game's directory and other
    /// products take names too, so whether that one is free is a fact about the
    /// game rather than about the overlay.
    fn istatli_luba(masar: &Path) -> Result<(), Box<dyn Error>> {
        let taqrir = istatli(masar)?;
        println!("survey: {}", masar.display());
        println!("verdict: {}  ({})", taqrir.hukm(), taqrir.hukm().ism_arabi());
        println!(
            "graphics: {}",
            if taqrir.wajihat.is_empty() {
                "none recognised".to_owned()
            } else {
                taqrir
                    .wajihat
                    .iter()
                    .map(|wajiha| wajiha.ism().to_owned())
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        );
        println!(
            "Taarib's own proxy slot is {}",
            match taqrir.halat_slot() {
                HalatSlot::Hurr => "free".to_owned(),
                HalatSlot::Taarib => "Taarib's own loader (a reinstall)".to_owned(),
                HalatSlot::Mashghul { slot } => format!("TAKEN by {} bytes at {}", slot.hajm, slot.masar.display()),
                HalatSlot::GhayrMaqru { thughra } => format!("UNREAD — {}", thughra.sabab),
            }
        );
        for thughra in &taqrir.thughrat {
            println!("  unread: {} — {}", thughra.ism, thughra.sabab);
        }
        for slot in &taqrir.mashghula {
            println!(
                "  occupied: {} — {} bytes{}",
                slot.ism,
                slot.hajm,
                if slot.taarib { " (Taarib's own)" } else { "" }
            );
        }
        for sabab in &taqrir.asbab {
            println!("  [{}] {}", sabab.hukm, sabab.injilizi);
        }
        println!(
            "installable: {}",
            if taqrir.yumkin() { "yes" } else { "no — see the refusal above" }
        );
        Ok(())
    }

    /// Prints what the batch contained.
    fn utbua_dufaa(dufaa: &LawhatRasm) {
        let masturat = dufaa.qitaat.iter().filter(|qita| qita.khareeta.is_some()).count();
        println!("=== the batch ===");
        println!("quads {}", dufaa.qitaat.len());
        println!("  textured (glyphs, drawn with D3DTOP_MODULATE):   {masturat}");
        println!(
            "  solid (plates, drawn with D3DTOP_SELECTARG1):    {}",
            dufaa.qitaat.len() - masturat
        );
        println!("surface recorded on the batch: {}×{}", dufaa.sath.ard, dufaa.sath.irtifa);
    }

    // -----------------------------------------------------------------------
    // The frame
    // -----------------------------------------------------------------------

    /// An sRGB-encoded RGBA frame buffer.
    ///
    /// Encoded rather than linear, and that is the difference from
    /// `talqeem_burhan.rs`'s frame. A Direct3D 9 backbuffer holds sRGB-encoded
    /// values and the fixed-function pipeline blends in that space; there is no
    /// `_SRGB` format to make the hardware decode and re-encode around the
    /// blend. So the encode happens once, per quad, in [`lawn_d3d`] — and this
    /// buffer is where the result lands, unconverted, exactly as it would in a
    /// game's own backbuffer.
    #[derive(Debug)]
    struct Itar {
        ard: u32,
        irtifa: u32,
        /// Four sRGB-encoded floats per pixel, row-major from the top.
        biksel: Vec<f32>,
    }

    impl Itar {
        /// A black frame.
        fn jadeed(ard: u32, irtifa: u32) -> Self {
            Self { ard, irtifa, biksel: vec![0.0; (ard as usize) * (irtifa as usize) * 4] }
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
                    if let Some(fahras) = self.fahras(s, a) {
                        self.biksel[fahras] = ufuqi.mul_add(0.05, nisba.mul_add(0.22, 0.10));
                        self.biksel[fahras + 1] = nisba.mul_add(0.26, 0.12);
                        self.biksel[fahras + 2] = nisba.mul_add(0.32, 0.18);
                        self.biksel[fahras + 3] = 1.0;
                    }
                }
            }
            self.mustatil(96, 360, 620, 140, [0.20, 0.23, 0.30]);
            self.mustatil(96, 360, 620, 2, [0.44, 0.60, 0.78]);
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

        /// Writes a PNG. The values are already encoded, so nothing converts.
        fn uktub(&self, masar: &Path) -> Result<(), Box<dyn Error>> {
            let mut bayt = Vec::with_capacity((self.ard as usize) * (self.irtifa as usize) * 3);
            for qeema in self.biksel.chunks_exact(4) {
                for qanat in qeema.iter().take(3) {
                    bayt.push((qanat * 255.0).round().clamp(0.0, 255.0) as u8);
                }
            }
            let surah: image::RgbImage =
                image::ImageBuffer::from_raw(self.ard, self.irtifa, bayt).ok_or_else(|| {
                    Box::<dyn Error>::from("the frame's byte count did not match its size")
                })?;
            surah.save(masar)?;
            Ok(())
        }
    }

    // -----------------------------------------------------------------------
    // The Direct3D 9 fixed-function stage, transcribed
    // -----------------------------------------------------------------------

    /// A [`Khattaf`] that runs the Direct3D 9 fixed-function stage in software.
    ///
    /// Not Direct3D, and this example says so in its own header and its own
    /// output. What it is is the same three decisions the real backend makes,
    /// executed by hand:
    ///
    /// * the vertex colour is [`lawn_d3d`]'s output — the *real* function, not a
    ///   reimplementation of it — unpacked back out of the `D3DCOLOR` it packs;
    /// * stage zero is `D3DTOP_MODULATE` of the atlas sample by that colour for
    ///   a glyph, and `D3DTOP_SELECTARG1` of the colour alone for a plate, which
    ///   is what `ibdal_marhala` switches between;
    /// * the blend is `D3DBLEND_ONE, D3DBLEND_INVSRCALPHA` into a backbuffer
    ///   holding encoded values.
    ///
    /// If the picture this writes is right, the numbers the driver is handed are
    /// right, and what is left untested is the driver multiplying them.
    #[derive(Debug)]
    struct KhattafBarnamaji {
        itar: Arc<Mutex<Itar>>,
        sath: WasfSath,
        /// The uploaded page: premultiplied RGBA8, its width, its height.
        lawha: Option<(Vec<u8>, u32, u32)>,
    }

    impl KhattafBarnamaji {
        /// A backend over a frame buffer.
        const fn jadeed(itar: Arc<Mutex<Itar>>, sath: WasfSath) -> Self {
            Self { itar, sath, lawha: None }
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

    /// A `D3DCOLOR` back to four zero-to-one channels, in `R`, `G`, `B`, `A`.
    ///
    /// The inverse of what [`lawn_d3d`] packs. Written here rather than beside
    /// it because the backend never needs it: the driver unpacks the vertex
    /// colour, and this example is standing in for the driver.
    fn min_lawn_d3d(lawn: u32) -> [f32; 4] {
        let qanat = |izaha: u32| ((lawn >> izaha) & 0xFF) as f32 / 255.0;
        [qanat(16), qanat(8), qanat(0), qanat(24)]
    }

    impl Khattaf for KhattafBarnamaji {
        fn wajiha(&self) -> WajihatRusum {
            WajihatRusum::Direct3D9
        }

        fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
            Ok(self.sath)
        }

        fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
            self.sath = sath;
            Ok(())
        }

        fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
            let matlub = (ard as usize).saturating_mul(irtifa as usize).saturating_mul(4);
            if bayt.len() != matlub {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "glyph atlas texture",
                    sabab: format!(
                        "the atlas declares {ard}×{irtifa} RGBA8, which is {matlub} bytes, and \
                         {} were supplied",
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
                // The vertex colour the real backend writes into the stream.
                let diffuse = min_lawn_d3d(lawn_d3d(qita.lawn));
                let ard = qita.mawdi.ard.max(1);
                let irtifa = qita.mawdi.irtifa.max(1);
                for saf in 0..qita.mawdi.irtifa {
                    for amud in 0..qita.mawdi.ard {
                        // Stage zero: MODULATE against the atlas for a glyph,
                        // SELECTARG1 of the diffuse alone for a plate.
                        let natija = match qita.khareeta {
                            Some(khareeta) => {
                                let u = khareeta.yasar
                                    + khareeta.ard * ((amud as f32) + 0.5) / (ard as f32);
                                let v = khareeta.aala
                                    + khareeta.irtifa * ((saf as f32) + 0.5) / (irtifa as f32);
                                let texel = self.ayina(u, v);
                                [
                                    texel[0] * diffuse[0],
                                    texel[1] * diffuse[1],
                                    texel[2] * diffuse[2],
                                    texel[3] * diffuse[3],
                                ]
                            },
                            None => diffuse,
                        };
                        let s = qita.mawdi.yasar.saturating_add(amud);
                        let a = qita.mawdi.aala.saturating_add(saf);
                        let Some(fahras) = itar.fahras(s, a) else {
                            continue;
                        };
                        // D3DBLEND_ONE, D3DBLEND_INVSRCALPHA.
                        let baqi = 1.0 - natija[3];
                        for (qanat, amam) in natija.iter().enumerate() {
                            itar.biksel[fahras + qanat] =
                                itar.biksel[fahras + qanat].mul_add(baqi, *amam);
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
            let mut kharj = Vec::new();
            for a in mintaqa.aala..mintaqa.aala + mintaqa.irtifa {
                for s in mintaqa.yasar..mintaqa.yasar + mintaqa.ard {
                    let Some(fahras) = itar.fahras(s, a) else {
                        continue;
                    };
                    // BGRA, which is what `D3DFMT_A8R8G8B8` is in memory.
                    for qanat in [2_usize, 1, 0, 3] {
                        kharj.push(
                            (itar.biksel[fahras + qanat] * 255.0).round().clamp(0.0, 255.0) as u8,
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

    // -----------------------------------------------------------------------
    // Paths
    // -----------------------------------------------------------------------

    /// The Arabic face this proof draws with, and where it was found.
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

    /// Where a picture goes. Beside the build output, which is already ignored.
    fn masar_kharj(ism: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/burhan").join(ism)
    }

    // -----------------------------------------------------------------------
    // The real device, when this machine has one
    // -----------------------------------------------------------------------

    /// A Direct3D 9 device against a hidden window, or why there is none.
    ///
    /// The same three defensive flags [`crate`]'s bootstrap uses, and the same
    /// reason for the first of them: creating a device without
    /// `D3DCREATE_FPU_PRESERVE` switches the whole process's x87 control word to
    /// single precision. This is an example rather than a game, and it is still
    /// the right flag — an example that demonstrated the wrong habit would be
    /// the place somebody copied it from.
    fn jihaz_haqiqi() -> Result<IDirect3DDevice9, String> {
        let nafidha = nafidha_muaqqata().map_err(|khata| khata.to_string())?;
        // SAFETY: `Direct3DCreate9` takes the SDK version and nothing else, and
        // answers `None` rather than failing when there is no runtime.
        let Some(tisaa) = (unsafe { Direct3DCreate9(D3D_SDK_VERSION) }) else {
            return Err("Direct3DCreate9 produced no interface".to_owned());
        };

        let mut muallimat = D3DPRESENT_PARAMETERS {
            BackBufferWidth: ARD,
            BackBufferHeight: IRTIFA,
            BackBufferFormat: D3DFMT_X8R8G8B8,
            BackBufferCount: 1,
            SwapEffect: D3DSWAPEFFECT_DISCARD,
            hDeviceWindow: nafidha.maqbad(),
            Windowed: BOOL::from(true),
            ..Default::default()
        };
        let mut jihaz: Option<IDirect3DDevice9> = None;
        let aalam = (D3DCREATE_SOFTWARE_VERTEXPROCESSING
            | D3DCREATE_FPU_PRESERVE
            | D3DCREATE_NOWINDOWCHANGES
            | D3DCREATE_MULTITHREADED)
            .cast_unsigned();
        // SAFETY: `tisaa` is the live interface above, the presentation
        // parameters are a fully initialised local naming the hidden window, and
        // the out-parameter addresses a local `None`.
        unsafe {
            tisaa.CreateDevice(
                D3DADAPTER_DEFAULT,
                D3DDEVTYPE_HAL,
                nafidha.maqbad(),
                aalam,
                &raw mut muallimat,
                &raw mut jihaz,
            )
        }
        .map_err(|khata| khata.to_string())?;

        // The hidden window is dropped here and the device keeps rendering into
        // its own backbuffer, which is all this proof reads. A game would keep
        // the window; nothing here presents.
        drop(nafidha);
        jihaz.ok_or_else(|| "Direct3D 9 reported success and produced no device".to_owned())
    }

    /// Drives the real backend against a real device and reads the result back.
    ///
    /// Four things are asserted here that no software transcription can assert:
    /// the device accepts the atlas as a texture, the state block captures and
    /// re-applies, a render state the overlay changes is found where the game
    /// left it afterwards, and the picture the GPU produced can be read off it.
    fn barhin_jihaz(
        jihaz: &IDirect3DDevice9,
        mulaqqim: &mut Mulaqqim,
        sutur: &[SatrMulaqqam],
    ) -> Result<(), Box<dyn Error>> {
        // Something for the overlay to composite over. A game's backbuffer holds
        // its own frame at `Present`; this device has never drawn, so without a
        // clear the picture would be Arabic on black and would say nothing about
        // whether the plate and the glyphs blend correctly over anything.
        //
        // SAFETY: `jihaz` is the live device this proof created, no rectangles
        // are named so the whole target is cleared, and the depth and stencil
        // values are ignored because only the colour buffer is named.
        unsafe {
            jihaz.Clear(0, core::ptr::null(), D3DCLEAR_TARGET.cast_unsigned(), 0xFF33_3A4A, 1.0, 0)?;
        }

        let khattaf = KhattafD3D9::min_jihaz(jihaz)?;
        match khattaf.qudra() {
            Ok(taqrir) => {
                println!("verdict: {}", taqrir.hukm());
                for satr in taqrir.sutur() {
                    println!("  {satr}");
                }
            },
            Err(khata) => println!("the device would not describe itself: {khata}"),
        }

        // A render state the overlay changes, read before and after. `Apply` is
        // what has to put it back, and this is the one assertion that proves it
        // did rather than assuming it.
        let mut qabl = 0_u32;
        // SAFETY: `jihaz` is the live device this proof created, and the
        // out-pointer addresses a live local.
        unsafe { jihaz.GetRenderState(D3DRS_CULLMODE, &raw mut qabl) }?;

        let lahza =
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |mudda| mudda.as_secs());
        let iqrar = Iqrar::baad_ard(BasmatIfsah::hadhihi_al_bina(), lahza, "برهان دايركت٣د ٩")?;
        let mut tabaqa = Tabaqa::shaghghil(Box::new(khattaf), iqrar)?;
        let Some(hayy) = tabaqa.sath() else {
            return Err(Box::<dyn Error>::from("the overlay reported no live surface"));
        };
        println!("surface: {}×{} {}", hayy.ard, hayy.irtifa, hayy.sigha.ism());

        // A fresh feeder, because the one above already spent its atlas against
        // the software backend and `Mulaqqim::irfa` uploads only what changed.
        mulaqqim.amsah();
        let dufaa = mulaqqim.qaddim(&mut tabaqa, hayy, sutur, &[], 0)?;
        println!("drew {} quads through IDirect3DDevice9", dufaa.qitaat.len());

        let mut baad = 0_u32;
        // SAFETY: as the read above.
        unsafe { jihaz.GetRenderState(D3DRS_CULLMODE, &raw mut baad) }?;
        if qabl == baad {
            println!("state block: D3DRS_CULLMODE was {qabl} before and after — restored");
        } else {
            return Err(Box::<dyn Error>::from(format!(
                "the state block did not restore D3DRS_CULLMODE: {qabl} before, {baad} after"
            )));
        }

        let mintaqa =
            MustatilBiksel { yasar: 0, aala: 0, ard: hayy.ard, irtifa: hayy.irtifa };
        let bayt = tabaqa.iltaqit(MustatilNisbi::KAAMIL, "the whole surface")?;
        println!("read back {} bytes from the GPU", bayt.len());

        let masar = masar_kharj("d3d9_burhan_jihaz.png");
        uktub_bgra(&masar, &bayt, mintaqa)?;
        println!("wrote {}", masar.display());
        println!(
            "reached: drawn by KhattafD3D9 through a real IDirect3DDevice9, state captured and \
             re-applied, and the frame read back off the GPU. Still not drawn over a running \
             game."
        );
        Ok(())
    }

    /// Writes a tightly packed BGRA read-back as a PNG.
    fn uktub_bgra(
        masar: &Path,
        bayt: &[u8],
        mintaqa: MustatilBiksel,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(walid) = masar.parent() {
            fs::create_dir_all(walid)?;
        }
        let mut rgb = Vec::with_capacity(bayt.len());
        for biksel in bayt.chunks_exact(4) {
            rgb.extend_from_slice(&[biksel[2], biksel[1], biksel[0]]);
        }
        let surah: image::RgbImage =
            image::ImageBuffer::from_raw(mintaqa.ard, mintaqa.irtifa, rgb).ok_or_else(|| {
                Box::<dyn Error>::from("the read-back byte count did not match the region")
            })?;
        surah.save(masar)?;
        Ok(())
    }
}
