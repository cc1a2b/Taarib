//! لوح بيو٤ — the generated atlas, and the game's own draw loop run offscreen.
//!
//! Builds a `BIO4` font out of real Arabic through the whole pipeline, then draws
//! two things into one PNG:
//!
//! 1. **The atlas**, at one texel per pixel, with the cell grid drawn over it in
//!    grey. Every cell is 28×28, which is the pitch thirty-one of *Resident Evil
//!    4*'s thirty-two shipped fonts use.
//! 2. **Each sample line, drawn the way the game draws it**: walk the transported
//!    cell sequence left to right, blit the sub-rectangle `[yasar, yameen)` of
//!    each cell at the pen, advance the pen by that width. Nothing about Arabic
//!    is known to that loop — it is one image per index, in order — so what
//!    appears is the shaper's output and nothing else.
//!
//! That second half is the point. If the letters in it join, run right to left,
//! ligate `لا`, and carry their marks, then the shaping survived a container
//! that has no shaper, no font file and no notion of direction.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p taarib-muhawwil-bio4 --example lawh_re4 -- /tmp/lawh_re4.png
//! ```

#![allow(
    clippy::print_stdout,
    reason = "this example exists to report what it produced, and where"
)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "an example reports a broken input by stopping, not by threading a Result to main"
)]
#![allow(
    clippy::integer_division,
    reason = "pixel arithmetic over a canvas whose dimensions are chosen here"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_muhawwil_bio4::bina::{KhattMabni, KhiyaratBina, ibni};
use taarib_muhawwil_bio4::shabaka::Shabaka;
use taarib_saff::{KhiyaratTakhtit, MawridKhatt, SilsilatKhutut};

/// The cell pitch, in texels.
const HAJM_KHANA: u32 = 28;

/// The atlas width, in texels.
const ARD_LAWHA: u32 = 1024;

/// The size the Arabic is shaped and drawn at.
const HAJM: f32 = 20.0;

/// The baseline inside a cell, in texels down from its top.
const ASAS: u32 = 21;

/// How much the simulated draw is magnified, so joins are visible.
const TAKBIR: u32 = 3;

/// Blank pixels around everything.
const HASHIYA: u32 = 16;

/// One sample, and what it is there to show.
struct Ayina {
    /// What a reader should look for.
    wasf: &'static str,
    /// The text, in logical order and ordinary Unicode.
    nass: &'static str,
}

/// The samples, chosen so that each of the brief's four questions is answerable
/// from the pixels alone.
const AYINAT: [Ayina; 8] = [
    Ayina { wasf: "joining: one word, four letters, three joins", nass: "بيبب" },
    Ayina { wasf: "isolated / initial / medial / final beh", nass: "ب ببب" },
    Ayina { wasf: "lam-alef: one ligature, not two letters", nass: "لا لأ لإ لآ" },
    Ayina { wasf: "diacritics on their bases", nass: "بَ بُ بِ بّ" },
    Ayina { wasf: "a vocalised word", nass: "مُحَمَّدٌ" },
    Ayina { wasf: "direction: alef first logically, last visually", nass: "ابج" },
    Ayina { wasf: "a real line of interface text", nass: "اضغط الزر للمتابعة" },
    Ayina { wasf: "digits inside Arabic", nass: "الذخيرة ١٢ طلقة" },
];

fn main() -> Result<(), Box<dyn Error>> {
    let masar = std::env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from("lawh_re4.png"), PathBuf::from);

    let khutut = khutut()?;
    let takhtit = KhiyaratTakhtit { satr_wahid: true, ..KhiyaratTakhtit::default() };
    let nusus: Vec<&str> = AYINAT.iter().map(|ayina| ayina.nass).collect();
    let asli = Shabaka::jadeeda(HAJM_KHANA, ARD_LAWHA, 576)?;
    let mabni = ibni(
        &nusus,
        &khutut,
        &takhtit,
        KhiyaratBina { hajm: HAJM, asas: ASAS, hizma: [0x09, 0x00, 0x00, 0x13], asli },
    )?;

    let shabaka = mabni.khatt.shabaka()?;
    println!("cells used      : {}", mabni.taqreer.khanat_mustaamala);
    println!("cells available : {}", mabni.taqreer.khanat_mutaha);
    println!("distinct glyphs : {}", mabni.taqreer.ashkal);
    println!("clamped glyphs  : {}", mabni.taqreer.ashkal_maqsusa);
    println!("worst clamp     : {} texel(s)", mabni.taqreer.aqsa_tajawuz);
    println!("advance lost    : {} texel(s) in total", mabni.taqreer.farq_taqaddum);
    println!("marks dropped   : {}", mabni.taqreer.alamat_mafquda);
    println!("marks contested : {}", mabni.taqreer.alamat_mutanaziaa);
    println!("atlas           : {}x{}", shabaka.ard(), shabaka.irtifa());
    println!();
    println!("the drawn lines, top to bottom:");
    for (fahras, ayina) in AYINAT.iter().enumerate() {
        let tul = mabni.nusus.get(fahras).map_or(0, taarib_muhawwil_bio4::NassManqulBio4::tul);
        println!("  {}. {} — {tul} cell(s)", fahras + 1, ayina.wasf);
    }

    let lawh = irsim(&mabni, shabaka);
    fs::write(&masar, ramz_png(&lawh))?;
    println!("wrote           : {}", masar.display());
    Ok(())
}

/// The staged Arabic face, found by walking up from this crate.
fn khutut() -> Result<SilsilatKhutut, Box<dyn Error>> {
    const MURASHAHAT: &[&str] = &[
        "apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf",
        "target/tajmee/makhbaa/sans/IBMPlexSansArabic-Regular.ttf",
    ];
    let mut jidhr: Option<&Path> = Some(Path::new(env!("CARGO_MANIFEST_DIR")));
    while let Some(dalil) = jidhr {
        for murashah in MURASHAHAT {
            let masar = dalil.join(murashah);
            if masar.is_file() {
                let bayt = fs::read(&masar)?;
                let khatt = MawridKhatt::jadeed(Arc::new(bayt), 0)?;
                return Ok(SilsilatKhutut::wahid(Arc::new(khatt))?);
            }
        }
        jidhr = dalil.parent();
    }
    Err("no Arabic font found at or above this crate".into())
}

// ---------------------------------------------------------------------------
// The canvas
// ---------------------------------------------------------------------------

/// An RGB canvas.
struct Lawh {
    /// Width in pixels.
    ard: u32,
    /// Height in pixels.
    irtifa: u32,
    /// Three bytes per pixel, row-major.
    bayt: Vec<u8>,
}

impl Lawh {
    /// A white canvas.
    fn jadeed(ard: u32, irtifa: u32) -> Self {
        let hajm = (ard as usize) * (irtifa as usize) * 3;
        Self { ard, irtifa, bayt: vec![0xFF; hajm] }
    }

    /// Paints one pixel.
    fn nuqta(&mut self, s: u32, a: u32, lawn: [u8; 3]) {
        if s >= self.ard || a >= self.irtifa {
            return;
        }
        let mawdi = ((a as usize) * (self.ard as usize) + (s as usize)) * 3;
        if let Some(makan) = self.bayt.get_mut(mawdi..mawdi + 3) {
            makan.copy_from_slice(&lawn);
        }
    }

    /// Paints one pixel as `lawn` at `taghtiya` over what is there.
    fn amzij(&mut self, s: u32, a: u32, lawn: [u8; 3], taghtiya: u8) {
        if taghtiya == 0 || s >= self.ard || a >= self.irtifa {
            return;
        }
        let mawdi = ((a as usize) * (self.ard as usize) + (s as usize)) * 3;
        let Some(makan) = self.bayt.get_mut(mawdi..mawdi + 3) else {
            return;
        };
        for (qanat, hadaf) in lawn.iter().zip(makan.iter_mut()) {
            let amam = u32::from(*qanat) * u32::from(taghtiya);
            let khalf = u32::from(*hadaf) * u32::from(255 - taghtiya);
            *hadaf = u8::try_from((amam + khalf) / 255).unwrap_or(*hadaf);
        }
    }
}

/// Draws the atlas and the simulated lines onto one canvas.
fn irsim(mabni: &KhattMabni, shabaka: Shabaka) -> Lawh {
    let ard = ARD_LAWHA + HASHIYA * 2;
    let irtifa_lawha = shabaka.irtifa();
    let irtifa_satr = HAJM_KHANA * TAKBIR + 10;
    let irtifa = HASHIYA * 3
        + irtifa_lawha
        + irtifa_satr * u32::try_from(mabni.nusus.len()).unwrap_or(0);
    let mut lawh = Lawh::jadeed(ard, irtifa);

    // The atlas, one texel per pixel, ink in black.
    for a in 0..irtifa_lawha {
        for s in 0..shabaka.ard() {
            let taghtiya = mabni.sura.texel(s, a).unwrap_or(0);
            lawh.amzij(HASHIYA + s, HASHIYA + a, [0x11, 0x11, 0x11], taghtiya);
        }
    }
    // The cell grid, so a reader can see that every glyph sits inside one cell.
    for satr in 0..=shabaka.sufuf() {
        let a = HASHIYA + satr * HAJM_KHANA;
        for s in 0..shabaka.ard() {
            lawh.nuqta(HASHIYA + s, a.min(HASHIYA + irtifa_lawha), [0xCC, 0xD6, 0xE0]);
        }
    }
    for amud in 0..=shabaka.aamida() {
        let s = HASHIYA + amud * HAJM_KHANA;
        for a in 0..=irtifa_lawha {
            lawh.nuqta(s, HASHIYA + a, [0xCC, 0xD6, 0xE0]);
        }
    }

    // Each line, drawn the way the game draws it.
    let mut asas_a = HASHIYA * 2 + irtifa_lawha;
    for nass in &mabni.nusus {
        // A faint baseline, so vertical placement is checkable too.
        for s in 0..(ard - HASHIYA * 2) {
            lawh.nuqta(HASHIYA + s, asas_a + ASAS * TAKBIR, [0xE4, 0xE9, 0xEF]);
        }

        let mut qalam = HASHIYA;
        for khana in nass.khanat() {
            let Some((cx, cy)) = shabaka.mawdi(*khana) else {
                continue;
            };
            let Some(madkhal) = mabni.khatt.madakhil.get(*khana as usize) else {
                continue;
            };
            if madkhal.khali() {
                // A blank cell still moves the pen by a word space; the container
                // has no other way to say "space", and neither does the game.
                qalam += (HAJM_KHANA / 4) * TAKBIR;
                continue;
            }
            let yasar = u32::from(madkhal.yasar);
            let yameen = u32::from(madkhal.yameen);
            for a in 0..HAJM_KHANA {
                for s in yasar..yameen {
                    let taghtiya = mabni.sura.texel(cx + s, cy + a).unwrap_or(0);
                    if taghtiya == 0 {
                        continue;
                    }
                    for da in 0..TAKBIR {
                        for ds in 0..TAKBIR {
                            lawh.amzij(
                                qalam + (s - yasar) * TAKBIR + ds,
                                asas_a + a * TAKBIR + da,
                                [0x11, 0x11, 0x11],
                                taghtiya,
                            );
                        }
                    }
                }
            }
            qalam += (yameen - yasar) * TAKBIR;
        }
        asas_a += irtifa_satr;
    }
    lawh
}

// ---------------------------------------------------------------------------
// A PNG, written by hand
// ---------------------------------------------------------------------------
//
// This crate takes no image dependency and this example does not add one. A PNG
// with stored (uncompressed) deflate blocks is a few dozen lines and is an
// ordinary PNG that every viewer reads. The same encoder is in
// `taarib-saff/examples/nazra.rs`, for the same reason.

/// Encodes an RGB canvas as an 8-bit truecolour PNG.
fn ramz_png(lawh: &Lawh) -> Vec<u8> {
    let tul_saf = (lawh.ard as usize) * 3;
    let mut khaam = Vec::with_capacity(lawh.bayt.len() + lawh.irtifa as usize);
    for saf in 0..lawh.irtifa as usize {
        khaam.push(0);
        if let Some(bayanat) = lawh.bayt.get(saf * tul_saf..(saf + 1) * tul_saf) {
            khaam.extend_from_slice(bayanat);
        }
    }

    let mut png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&lawh.ard.to_be_bytes());
    ihdr.extend_from_slice(&lawh.irtifa.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    qitaa(&mut png, *b"IHDR", &ihdr);
    qitaa(&mut png, *b"IDAT", &zlib_makhzun(&khaam));
    qitaa(&mut png, *b"IEND", &[]);
    png
}

/// Appends one PNG chunk with its length and CRC.
fn qitaa(makhruj: &mut Vec<u8>, naw: [u8; 4], bayanat: &[u8]) {
    makhruj.extend_from_slice(&(u32::try_from(bayanat.len()).unwrap_or(0)).to_be_bytes());
    makhruj.extend_from_slice(&naw);
    makhruj.extend_from_slice(bayanat);
    let mut madkhal = Vec::with_capacity(4 + bayanat.len());
    madkhal.extend_from_slice(&naw);
    madkhal.extend_from_slice(bayanat);
    makhruj.extend_from_slice(&crc32(&madkhal).to_be_bytes());
}

/// Wraps bytes in a zlib stream of stored deflate blocks.
fn zlib_makhzun(khaam: &[u8]) -> Vec<u8> {
    let mut makhruj: Vec<u8> = vec![0x78, 0x01];
    let mut mawqi = 0usize;
    loop {
        let tul = (khaam.len() - mawqi).min(0xFFFF);
        let akhir = u8::from(mawqi + tul >= khaam.len());
        makhruj.push(akhir);
        let tul16 = u16::try_from(tul).unwrap_or(u16::MAX);
        makhruj.extend_from_slice(&tul16.to_le_bytes());
        makhruj.extend_from_slice(&(!tul16).to_le_bytes());
        if let Some(kutla) = khaam.get(mawqi..mawqi + tul) {
            makhruj.extend_from_slice(kutla);
        }
        mawqi += tul;
        if akhir == 1 {
            break;
        }
    }
    makhruj.extend_from_slice(&adler32(khaam).to_be_bytes());
    makhruj
}

/// The PNG chunk checksum.
fn crc32(bayanat: &[u8]) -> u32 {
    let mut qeema = 0xFFFF_FFFF_u32;
    for bayt in bayanat {
        qeema ^= u32::from(*bayt);
        for _ in 0..8 {
            qeema = if qeema & 1 == 0 { qeema >> 1 } else { (qeema >> 1) ^ 0xEDB8_8320 };
        }
    }
    !qeema
}

/// The zlib stream checksum.
fn adler32(bayanat: &[u8]) -> u32 {
    let mut alif: u32 = 1;
    let mut ba: u32 = 0;
    for bayt in bayanat {
        alif = (alif + u32::from(*bayt)) % 65521;
        ba = (ba + alif) % 65521;
    }
    (ba << 16) | alif
}
