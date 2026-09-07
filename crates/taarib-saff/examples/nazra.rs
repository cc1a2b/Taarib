//! نظرة — a look at what the Arabic engine actually produces.
//!
//! Shapes a handful of real Arabic strings through the public pipeline
//! ([`Saff::khattit`]), prints every positioned glyph it gets back, then draws
//! the same glyphs with the engine's own rasterizer ([`Rassam`]) and writes a
//! PNG so a person can look at the result rather than read about it.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p taarib-saff --example nazra
//! ```
//!
//! Nothing here is a mock. The glyph identifiers printed are the identifiers
//! HarfRust produced, the outlines drawn are the outlines skrifa resolved for
//! those same identifiers out of the same byte buffer, and the positions are
//! the positions the layout stage placed them at.

#![allow(
    clippy::print_stdout,
    reason = "this example exists to print what the engine produced"
)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "pixel coordinates are deliberately narrowed to integers, with the ranges checked"
)]

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_saff::{
    Ittijah, KhiyaratTakhtit, MawridKhatt, NamatRasm, Rassam, Saff, SilsilatKhutut, TakhtitNass,
    TalabTakhtit,
};

/// The size the samples are laid out and drawn at, in pixels.
const HAJM: f32 = 44.0;

/// The size the captions on the drawing are laid out at, in pixels.
const HAJM_TAWDEEH: f32 = 17.0;

/// Blank pixels left around the drawing.
const HASHIYA: u32 = 24;

/// Extra vertical room between one sample and the next, in pixels.
const FARQ: u32 = 18;

/// One string to put through the engine, with a label for the printout.
#[derive(Debug, Clone, Copy)]
struct Ayina {
    /// What the sample is meant to show.
    wasf: &'static str,
    /// The text itself, in logical order.
    nass: &'static str,
}

/// The samples, in the order they are printed and drawn.
const AYINAT: &[Ayina] = &[
    Ayina {
        wasf: "a full sentence",
        nass: "مرحبًا بالعالم من محرك تعريب",
    },
    Ayina {
        wasf: "four forms of beh",
        nass: "بببب",
    },
    Ayina {
        wasf: "lam-alef ligature",
        nass: "لا",
    },
    Ayina {
        wasf: "non-joining alef and dal",
        nass: "أدب",
    },
    Ayina {
        wasf: "diacritics stacked on bases",
        nass: "مُحَمَّدٌ",
    },
    Ayina {
        wasf: "Arabic with an embedded Latin run",
        nass: "لعبة Half-Life 2 الشهيرة",
    },
];

fn main() -> Result<(), Box<dyn Error>> {
    let (masar, bayt) = ijid_khatt()?;
    println!("font: {}", masar.display());
    println!("bytes: {}", bayt.len());

    // Decision 6's validation runs first and its verdict is printed, because
    // everything below it is only worth reading if the font got in through the
    // validated door rather than through `jadeed_latini`.
    let bayt = Arc::new(bayt);
    let khatt = match MawridKhatt::jadeed(Arc::clone(&bayt), 0) {
        Ok(khatt) => {
            println!("Decision 6 validation: passed");
            Arc::new(khatt)
        },
        Err(khata) => {
            println!("Decision 6 validation: REJECTED — {khata}");
            println!(
                "  continuing through MawridKhatt::jadeed_latini, which skips the check and \
                 changes nothing about how the font shapes"
            );
            Arc::new(MawridKhatt::jadeed_latini(bayt, 0)?)
        },
    };
    println!("family: {} {}", khatt.aila(), khatt.namat());
    println!("variable: {}", khatt.mutaghayyir());
    println!("covers Latin 'H': {}", khatt.yughatti('H'));
    let qiyasat = khatt.qiyasat(HAJM);
    println!(
        "metrics at {HAJM}px: ascent {:.2}, descent {:.2}, line height {:.2}, units/em {}",
        qiyasat.suud, qiyasat.hubut, qiyasat.irtifa_satr, qiyasat.wahdat
    );

    let rassam = Rassam::jadeed(&khatt)?;
    println!("glyphs in face: {}", rassam.adad_ashkal());

    let khutut = SilsilatKhutut::wahid(Arc::clone(&khatt))?;
    let khiyarat = KhiyaratTakhtit::default();
    let mut saff = Saff::jadeed();

    let mut khattit = |nass: &str, hajm: f32| -> Result<TakhtitNass, Box<dyn Error>> {
        Ok(saff.khattit(&TalabTakhtit {
            nass,
            khutut: &khutut,
            hajm,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: &[],
            khiyarat: &khiyarat,
        })?)
    };

    // The page opens with the comparison the whole product is about: the same
    // string, the same font, drawn twice — once the way a game with no shaper
    // draws it, once the way this engine draws it.
    let muqarana = AYINAT.first().map_or("", |ayina| ayina.nass);
    let mut rusum: Vec<Rasma> = vec![
        Rasma::tawdeeh(&khattit(
            "what a game with no Arabic shaper draws:",
            HAJM_TAWDEEH,
        )?),
        Rasma::sadija(&khatt, &rassam, muqarana)?,
        Rasma::tawdeeh(&khattit(
            "what taarib-saff draws — same font, same string, same bytes:",
            HAJM_TAWDEEH,
        )?),
    ];

    for (raqm, ayina) in AYINAT.iter().enumerate() {
        let takhtit = khattit(ayina.nass, HAJM)?;
        utbua(ayina, &takhtit);
        // The first sample is the comparison above and already has its caption.
        if raqm > 0 {
            rusum.push(Rasma::tawdeeh(&khattit(ayina.wasf, HAJM_TAWDEEH)?));
        }
        rusum.push(Rasma::min_takhtit(&takhtit));
    }

    let masar_surah = irsim_kul(&rassam, &rusum)?;
    println!("\nwrote {}", masar_surah.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// The printout
// ---------------------------------------------------------------------------

/// Prints every glyph of one layout, with the numbers that matter.
///
/// The per-glyph offsets are recovered by walking the pen exactly the way
/// `qiyas::mawqi_huruf` walked it: the pen starts at the line's aligned
/// beginning and advances by each glyph's contribution, so the difference
/// between the pen and the glyph's placed position is precisely the offset
/// `GPOS` gave it. That is a derivation from public output, not a second
/// measurement.
fn utbua(ayina: &Ayina, takhtit: &TakhtitNass) {
    println!("\n=== {} ===", ayina.wasf);
    println!("text (logical): {}", ayina.nass);
    println!(
        "chars: {}  bytes: {}  glyphs: {}  lines: {}",
        ayina.nass.chars().count(),
        ayina.nass.len(),
        takhtit.huruf.len(),
        takhtit.sutur.len()
    );
    println!(
        "paragraph direction: {:?}  width {:.2}px  height {:.2}px",
        takhtit.ittijah, takhtit.ard, takhtit.irtifa
    );

    for (raqm, satr) in takhtit.sutur.iter().enumerate() {
        println!(
            "  line {raqm}: direction {:?}, baseline {:.2}, starts at {:.2}, width {:.2}, \
             logical bytes {}..{}",
            satr.ittijah, satr.asas, satr.bidaya, satr.ard, satr.mantiqi.start, satr.mantiqi.end
        );
        println!(
            "    {:>3}  {:>6}  {:>7}  {:>9}  {:>8}  {:>9}  {:>9}  {:>4}",
            "vis", "glyph", "cluster", "x", "advance", "dx", "dy", "mark"
        );
        let mut qalam = satr.bidaya;
        for (fahras, harf) in takhtit.huruf_satr(satr).iter().enumerate() {
            let izaha_s = harf.s - qalam;
            let izaha_a = satr.asas - harf.a;
            println!(
                "    {fahras:>3}  {:>6}  {:>7}  {:>9.3}  {:>8.3}  {:>9.3}  {:>9.3}  {:>4}",
                harf.muarrif,
                harf.anqud,
                harf.s,
                harf.taqaddum,
                izaha_s,
                izaha_a,
                if harf.alama { "yes" } else { "" }
            );
            qalam += harf.taqaddum;
        }
    }

    let mut ashkal = takhtit.ashkal();
    ashkal.sort_unstable();
    println!("  distinct (font, glyph) pairs: {ashkal:?}");
}

// ---------------------------------------------------------------------------
// Finding the font
// ---------------------------------------------------------------------------

/// The Arabic face this example draws with, and where it was found.
///
/// The staged copy under `apps/studio/src/khutut` is preferred, because that is
/// the file the repository's own hash lock pins. Everything else is a fallback
/// so the example still runs in a tree where staging has not been done.
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
                let bayt = fs::read(&masar)?;
                return Ok((masar, bayt));
            }
        }
        jidhr = dalil.parent();
    }

    Err(Box::<dyn Error>::from(
        "no Arabic font found; expected apps/studio/src/khutut/IBMPlexSansArabic-Regular.ttf \
         somewhere at or above this crate",
    ))
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// A plain RGB canvas, white to begin with.
#[derive(Debug)]
struct Lawh {
    /// Width in pixels.
    ard: u32,
    /// Height in pixels.
    irtifa: u32,
    /// Three bytes per pixel, row-major from the top.
    bayt: Vec<u8>,
}

impl Lawh {
    /// A white canvas.
    fn jadeed(ard: u32, irtifa: u32) -> Self {
        let hajm = (ard as usize) * (irtifa as usize) * 3;
        Self {
            ard,
            irtifa,
            bayt: vec![0xFF; hajm],
        }
    }

    /// Blends one pixel toward a colour by a coverage in `0..=255`.
    fn nuqta(&mut self, s: i64, a: i64, lawn: [u8; 3], taghtiya: u8) {
        if taghtiya == 0
            || s < 0
            || a < 0
            || s >= i64::from(self.ard)
            || a >= i64::from(self.irtifa)
        {
            return;
        }
        let fahras = ((a as usize) * (self.ard as usize) + (s as usize)) * 3;
        let alfa = f32::from(taghtiya) / 255.0;
        for qanat in 0..3 {
            let Some(khana) = self.bayt.get_mut(fahras + qanat) else {
                return;
            };
            let khalf = f32::from(*khana);
            let amam = f32::from(lawn.get(qanat).copied().unwrap_or(0));
            *khana = (khalf.mul_add(1.0 - alfa, amam * alfa))
                .round()
                .clamp(0.0, 255.0) as u8;
        }
    }

    /// Draws a one-pixel horizontal rule, used for the baseline guide.
    fn khatt_ufuqi(&mut self, a: i64, min_s: i64, ila_s: i64, lawn: [u8; 3], taghtiya: u8) {
        for s in min_s..ila_s {
            self.nuqta(s, a, lawn, taghtiya);
        }
    }
}

/// One row of the drawing: glyphs already placed relative to their own origin.
///
/// This is the common shape a laid-out line and a deliberately unshaped line
/// are both reduced to, so the drawing code cannot accidentally give one of
/// them treatment the other did not get.
#[derive(Debug)]
struct Rasma {
    /// The pixel size these glyphs must be rasterized at.
    hajm: f32,
    /// `(glyph id, x from the row's origin, y from its baseline, is a mark)`.
    /// `y` is positive downward, matching [`Harf::a`].
    huruf: Vec<(u32, f32, f32, bool)>,
    /// The row's measured width.
    ard: f32,
    /// How far the row rises above its baseline.
    suud: f32,
    /// How far it falls below.
    hubut: f32,
    /// Right-aligned in the canvas when true, left-aligned when false.
    yameen: bool,
    /// Drawn in grey rather than black — the captions.
    khafeef: bool,
}

impl Rasma {
    /// A row from a finished layout: what the engine produced, unaltered.
    fn min_takhtit(takhtit: &TakhtitNass) -> Self {
        let (suud, hubut) = takhtit
            .sutur
            .first()
            .map_or((0.0, 0.0), |satr| (satr.suud, satr.hubut));
        let mut huruf = Vec::with_capacity(takhtit.huruf.len());
        for satr in &takhtit.sutur {
            for harf in takhtit.huruf_satr(satr) {
                huruf.push((harf.muarrif, harf.s, harf.a - satr.asas, harf.alama));
            }
        }
        Self {
            hajm: takhtit.hajm,
            huruf,
            ard: takhtit.ard,
            suud,
            hubut,
            yameen: takhtit.ittijah == Ittijah::Yameen,
            khafeef: false,
        }
    }

    /// A caption, left-aligned and grey.
    fn tawdeeh(takhtit: &TakhtitNass) -> Self {
        Self {
            yameen: false,
            khafeef: true,
            ..Self::min_takhtit(takhtit)
        }
    }

    /// A row drawn the way a game with no Arabic support draws it.
    ///
    /// One `cmap` lookup per character, each glyph advanced by its own nominal
    /// width, laid down left to right in logical order. No shaper, no
    /// bidirectional reorder, no joining, no ligature, no mark attachment. This
    /// is not a caricature — it is what a text field that calls
    /// `font.getGlyph(codepoint)` in a loop actually produces, and it is drawn
    /// here from the same font file and the same rasterizer as the row above
    /// it so the two are comparable.
    fn sadija(
        khatt: &Arc<MawridKhatt>,
        rassam: &Rassam,
        nass: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let qiyasat = khatt.qiyasat(HAJM);
        let mut huruf = Vec::new();
        let mut qalam = 0.0_f32;
        for harf in nass.chars() {
            let Some(muarrif) = khatt.muarrif(harf) else {
                continue;
            };
            let surah = rassam.irsim(muarrif, HAJM, NamatRasm::Taghtiya, 0.0, &[])?;
            huruf.push((muarrif, qalam, 0.0, false));
            qalam += surah.taqaddum;
        }
        Ok(Self {
            hajm: HAJM,
            huruf,
            ard: qalam,
            suud: qiyasat.suud,
            hubut: qiyasat.hubut,
            yameen: false,
            khafeef: false,
        })
    }

    /// The height of the band this row occupies.
    fn irtifa(&self) -> f32 {
        self.suud + self.hubut
    }
}

/// Draws every row onto one canvas and writes it out as a PNG.
///
/// Ink is black; combining marks are drawn in dark red so that mark placement is
/// visible as placement rather than having to be inferred from a black blob, and
/// each row's baseline is a faint blue rule. Nothing about the geometry is
/// changed by the colouring: every glyph is drawn at the position the layout
/// gave it, from the outline the engine's own rasterizer produced.
fn irsim_kul(rassam: &Rassam, rusum: &[Rasma]) -> Result<PathBuf, Box<dyn Error>> {
    let mut ard: f32 = 0.0;
    let mut irtifa: f32 = 0.0;
    for rasma in rusum {
        ard = ard.max(rasma.ard);
        irtifa += rasma.irtifa() + FARQ as f32;
    }

    let ard_lawh = (ard.ceil() as u32).saturating_add(HASHIYA * 2).max(1);
    let irtifa_lawh = (irtifa.ceil() as u32).saturating_add(HASHIYA * 2).max(1);
    let mut lawh = Lawh::jadeed(ard_lawh, irtifa_lawh);

    let mut alaa = HASHIYA as f32;
    for rasma in rusum {
        let asas = alaa + rasma.suud;
        let izaha_s = if rasma.yameen {
            (ard_lawh as f32) - HASHIYA as f32 - rasma.ard
        } else {
            HASHIYA as f32
        };

        if !rasma.khafeef {
            lawh.khatt_ufuqi(
                asas.round() as i64,
                i64::from(HASHIYA),
                i64::from(ard_lawh.saturating_sub(HASHIYA)),
                [0x99, 0xB4, 0xD0],
                255,
            );
        }

        for (muarrif, s, a, alama) in &rasma.huruf {
            let lawn = if rasma.khafeef {
                [0x70, 0x70, 0x70]
            } else if *alama {
                [0xA0, 0x20, 0x20]
            } else {
                [0x11, 0x11, 0x11]
            };
            irsim_harf(
                rassam,
                &mut lawh,
                *muarrif,
                rasma.hajm,
                s + izaha_s,
                a + asas,
                lawn,
            )?;
        }

        alaa += rasma.irtifa() + FARQ as f32;
    }

    let masar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("target"), |jidhr| jidhr.join("target"))
        .join("nazra.png");
    if let Some(dalil) = masar.parent() {
        fs::create_dir_all(dalil)?;
    }
    fs::write(&masar, ramz_png(&lawh))?;
    Ok(masar)
}

/// Rasterizes one glyph and composites it onto the canvas at its laid-out
/// position.
fn irsim_harf(
    rassam: &Rassam,
    lawh: &mut Lawh,
    muarrif: u32,
    hajm: f32,
    s: f32,
    a: f32,
    lawn: [u8; 3],
) -> Result<(), Box<dyn Error>> {
    // The rasterizer quantizes the fractional part itself; handing it the raw
    // fraction is what keeps the bitmap and the position describing the same
    // subpixel bucket.
    let surah = rassam.irsim(muarrif, hajm, NamatRasm::Taghtiya, s - s.floor(), &[])?;
    if surah.khali() {
        return Ok(());
    }

    let asl_s = s.floor() as i64 + i64::from(surah.izaha_s);
    let asl_a = a.round() as i64 - i64::from(surah.izaha_a);

    for saf in 0..surah.irtifa {
        let Some(bayanat) = surah.satr(saf) else {
            continue;
        };
        for (amud, taghtiya) in bayanat.iter().enumerate() {
            let amud = i64::try_from(amud).unwrap_or(i64::MAX);
            lawh.nuqta(asl_s + amud, asl_a + i64::from(saf), lawn, *taghtiya);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// A PNG, written by hand
// ---------------------------------------------------------------------------
//
// This crate takes no image dependency and this example does not add one: the
// whole point of `taarib-saff` is that it stands alone, and an example that
// pulled in an encoder to prove it would be arguing against itself. A PNG with
// stored (uncompressed) deflate blocks is a few dozen lines and is a perfectly
// ordinary PNG that every viewer reads.

/// Encodes an RGB canvas as an 8-bit truecolour PNG.
fn ramz_png(lawh: &Lawh) -> Vec<u8> {
    let mut khaam = Vec::with_capacity(lawh.bayt.len() + lawh.irtifa as usize);
    let tul_saf = (lawh.ard as usize) * 3;
    for saf in 0..lawh.irtifa as usize {
        khaam.push(0); // filter type 0: none
        if let Some(bayanat) = lawh.bayt.get(saf * tul_saf..(saf + 1) * tul_saf) {
            khaam.extend_from_slice(bayanat);
        }
    }

    let mut png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&lawh.ard.to_be_bytes());
    ihdr.extend_from_slice(&lawh.irtifa.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, truecolour, no interlace
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
    // 0x78 0x01: deflate, 32 KiB window, no preset dictionary, fastest level.
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
            qeema = if qeema & 1 == 0 {
                qeema >> 1
            } else {
                (qeema >> 1) ^ 0xEDB8_8320
            };
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
