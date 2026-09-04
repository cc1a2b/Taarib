//! الخطّ — handing Slate a font, and the one reason that is allowed.
//!
//! Exposed as `khatt_unreal` by the crate root, which is the name the module
//! table in `lib.rs` uses.
//!
//! ## Read the constraint before the code
//!
//! Decision 5 says Taarib never touches the game's fonts, font assets or asset
//! bundles. This module registers a font into Slate's font system, and `lib.rs`
//! is explicit about why that is not a violation: **the bytes are Taarib's own
//! bundled font, read from the patch**. No `.uasset` font object is read,
//! modified or replaced. Decision 5 is about the game's assets, and this touches
//! none of them.
//!
//! That is a claim, and a claim in a doc comment is worth what the code makes
//! true. So this file is built so the violation is not expressible:
//!
//! - **It imports no filesystem API.** No `std::fs`, no `std::io`, no
//!   `memmap2`. There is no expression in this file that opens, reads,
//!   enumerates, creates, modifies or deletes any file. The check is one `grep`.
//! - **The only font bytes it accepts are the ones passed in**, as
//!   [`FaraiKhatt::bayt`]. There is no constructor that takes a path and there
//!   is no code that could turn a path into bytes.
//! - **The one path it holds is written into an ini as text and never opened**,
//!   and [`masar_masmuh`] refuses `.uasset`, `.uexp`, `.ubulk`, `.umap`,
//!   `.upk`, `.pak`, `.utoc` and `.ucas` before it can even be written. A path
//!   naming a cooked asset cannot get through this module in either direction.
//! - **The ini write goes through [`crate::tashghil::aktub_madakhil`]**, which
//!   takes a section and key/value strings. Nothing here can point it at a font.
//!
//! ## The ladder, and what each rung can actually do
//!
//! 1. **Configuration.** Where a game's engine version reads a fallback font
//!    family out of `[Internationalization]`, that is the better route: it needs
//!    no injection and it survives a game update. The key name is
//!    version-specific — [`MIFTAH_KHATT_IHTIYATI`] is a default the patch may
//!    override — and an ini key Unreal does not recognise is inert, which is
//!    what makes writing an uncertain key safe rather than reckless.
//! 2. **Command line.** The same keys as `-ini:` overrides, for games that
//!    regenerate their ini.
//! 3. **Injection.** [`Musaddir::sajjil_farai`] and
//!    [`Musaddir::atbiq_murakkab`] build the composite font in the live process
//!    and point the patch's named styles at it.
//!
//! Fonts are the one place in this adapter where the ini rung cannot be
//! confirmed in the session that wrote it: a fallback font is bound while the
//! engine starts, so the write applies at the *next* launch. Rung one is
//! therefore recorded honestly as unverified and rung three still runs — rung
//! one is what makes the next launch need less, and rung three is what fixes the
//! menu the player is looking at now. Pretending otherwise would mean recording
//! a rung as fired on the strength of a write nothing read back.
//!
//! ## What a composite font is, and why the ranges are what they are
//!
//! Slate's `FCompositeFont` is a default typeface plus sub-typefaces that claim
//! codepoint ranges and cultures, plus a fallback order for anything nobody
//! claimed. [`KhattMurakkab`] is that shape. The Arabic sub-font claims the
//! Arabic blocks and, deliberately, both presentation-forms blocks: a game whose
//! own strings already carry pre-shaped presentation forms from an older
//! localization pipeline will render them as tofu if the Arabic sub-font does
//! not claim them, and that failure looks exactly like a broken patch.
//!
//! It claims the bidirectional formatting characters and nothing else outside
//! the Arabic blocks. Claiming a whole punctuation block would be easy and would
//! be wrong: every untranslated Latin menu in the game would start drawing its
//! commas and brackets out of the Arabic font, and a patch that visibly changes
//! text it did not translate is a patch users uninstall.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::alam::QISM_TADWEEL;
use crate::khata::{KhataUnreal, tul_u64};
use crate::tashghil::{
    MadkhalIni, Musaddir, Rutba, SijillTashghil, aktub_madakhil, tarajua_madakhil,
};

/// The ini key some engine versions read a per-culture fallback font from.
///
/// A default the patch may override. The exact spelling moved between engine
/// versions, and an unrecognised key under `[Internationalization]` is inert —
/// Unreal keeps it and nothing reads it — so writing the wrong one costs a line
/// in a file and never costs a launch.
pub const MIFTAH_KHATT_IHTIYATI: &str = "FallbackFontFamily";

/// The largest font this module will accept.
///
/// Sixty-four mebibytes is far above any Arabic font and above most fonts that
/// cover CJK as well. The bytes arrive inside a patch a stranger produced, and a
/// length is checked before anything is handed to the engine, because handing
/// Slate a two-gigabyte "font" is a game that stops responding while `FreeType`
/// works through it.
pub const AQSA_HAJM_KHATT: u64 = 64 * 1024 * 1024;

/// File extensions this module refuses to name in a configuration file.
///
/// Cooked Unreal assets and containers, every one of them. The list exists so
/// that "this module cannot be pointed at the game's font asset" is a check and
/// not a promise.
pub const LAWAHIQ_MAMNUA: [&str; 8] =
    ["uasset", "uexp", "ubulk", "umap", "upk", "pak", "utoc", "ucas"];

/// The codepoint ranges the Arabic sub-font claims, inclusive on both ends.
///
/// Each entry is a block, and each block is here for a reason:
///
/// | range | why |
/// | --- | --- |
/// | `0600`–`06FF` | Arabic: letters, harakat, Arabic-Indic digits, tatweel |
/// | `0750`–`077F` | Arabic Supplement: letters for other languages in the script |
/// | `0870`–`089F` | Arabic Extended-B |
/// | `08A0`–`08FF` | Arabic Extended-A, carrying marks modern Qur'anic text uses |
/// | `200C`–`200F` | the zero-width joiners and the two directional marks |
/// | `2066`–`2069` | the isolates: a Latin name inside an Arabic sentence |
/// | `FB50`–`FDFF` | Presentation Forms-A |
/// | `FE70`–`FEFF` | Presentation Forms-B |
pub const NITAQAT_ARABIYA: [(u32, u32); 8] = [
    (0x0600, 0x06FF),
    (0x0750, 0x077F),
    (0x0870, 0x089F),
    (0x08A0, 0x08FF),
    (0x200C, 0x200F),
    (0x2066, 0x2069),
    (0xFB50, 0xFDFF),
    (0xFE70, 0xFEFF),
];

// ---------------------------------------------------------------------------
// Ranges
// ---------------------------------------------------------------------------

/// An inclusive codepoint range a sub-font claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NitaqHuruf {
    /// First codepoint, inclusive.
    pub awwal: u32,
    /// Last codepoint, inclusive.
    pub akhir: u32,
}

impl NitaqHuruf {
    /// Builds a range, ordering the ends so a reversed pair is not a silent
    /// empty range that claims nothing.
    #[must_use]
    pub const fn jadeed(awwal: u32, akhir: u32) -> Self {
        if awwal <= akhir { Self { awwal, akhir } } else { Self { awwal: akhir, akhir: awwal } }
    }

    /// Whether a codepoint falls in this range.
    #[must_use]
    pub const fn yahwi(self, harf: u32) -> bool {
        harf >= self.awwal && harf <= self.akhir
    }

    /// How many codepoints the range covers.
    #[must_use]
    pub const fn ittisa(self) -> u32 {
        self.akhir.saturating_sub(self.awwal).saturating_add(1)
    }

    /// The pair form the [`Musaddir`] seam takes.
    #[must_use]
    pub const fn zawj(self) -> (u32, u32) {
        (self.awwal, self.akhir)
    }

    /// Whether this range names real Unicode scalar values.
    ///
    /// Refuses anything above the Unicode maximum and anything inside the
    /// surrogate range: a font "covering" `D800`–`DFFF` covers codepoints that
    /// cannot appear in any string, and a range table containing one is a table
    /// somebody built out of UTF-16 code units by mistake.
    #[must_use]
    pub const fn salih(self) -> bool {
        self.akhir <= 0x0010_FFFF && !(self.awwal <= 0xDFFF && self.akhir >= 0xD800)
    }
}

/// The Arabic ranges as values.
#[must_use]
pub fn nitaqat_arabiya() -> Vec<NitaqHuruf> {
    NITAQAT_ARABIYA
        .iter()
        .map(|&(awwal, akhir)| NitaqHuruf::jadeed(awwal, akhir))
        .collect()
}

// ---------------------------------------------------------------------------
// Font bytes
// ---------------------------------------------------------------------------

/// The container format a run of font bytes is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SighatKhatt {
    /// A TrueType outline font, `00 01 00 00` or `true`.
    Truetype,
    /// A CFF OpenType font, `OTTO`.
    Opentype,
    /// A TrueType collection, `ttcf`.
    Majmua,
}

impl SighatKhatt {
    /// A stable short name for logs.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Truetype => "truetype",
            Self::Opentype => "opentype",
            Self::Majmua => "collection",
        }
    }
}

/// The format of a run of font bytes, or [`None`] when it is not one Slate's
/// `FreeType` build loads.
///
/// Only the four SFNT magics. A WOFF or WOFF2 falls out as [`None`] and is
/// refused by name in [`FaraiKhatt::tahaqquq`], because Slate reads SFNT and
/// handing it a WOFF produces a font that silently never loads and text that
/// silently keeps drawing in the game's own font — a bug report with no
/// evidence in it.
#[must_use]
pub fn sighat_bayt(bayt: &[u8]) -> Option<SighatKhatt> {
    match bayt.first_chunk::<4>()? {
        // 00 01 00 00, and Apple's "true".
        [0x00, 0x01, 0x00, 0x00] | [0x74, 0x72, 0x75, 0x65] => Some(SighatKhatt::Truetype),
        // "OTTO".
        [0x4F, 0x54, 0x54, 0x4F] => Some(SighatKhatt::Opentype),
        // "ttcf".
        [0x74, 0x74, 0x63, 0x66] => Some(SighatKhatt::Majmua),
        _ => None,
    }
}

/// Whether a path may be named in a configuration file by this module.
///
/// Refuses cooked Unreal assets and containers by extension, and refuses a path
/// that is not valid UTF-8 — which cannot be written into a UTF-8 ini without
/// re-encoding a game's configuration file, and re-encoding somebody's config is
/// not something a font registration gets to do.
#[must_use]
pub fn masar_masmuh(masar: &Path) -> bool {
    if masar.to_str().is_none() {
        return false;
    }
    masar.extension().and_then(|lahiqa| lahiqa.to_str()).is_none_or(|lahiqa| {
        let saghira = lahiqa.to_ascii_lowercase();
        !LAWAHIQ_MAMNUA.contains(&saghira.as_str())
    })
}

// ---------------------------------------------------------------------------
// The composite font
// ---------------------------------------------------------------------------

/// One sub-font of a composite font, with the bytes it draws from.
///
/// The bytes are an [`Arc`] because Slate keeps the buffer alive for the life of
/// the font and the adapter may register the same face under more than one
/// composite; copying a font per registration would spend megabytes of a game's
/// budget on identical buffers.
#[derive(Clone)]
pub struct FaraiKhatt {
    /// The name this sub-font is registered and referred to by.
    pub ism: String,
    /// The patch's own font bytes. The only font bytes this module ever sees.
    pub bayt: Arc<[u8]>,
    /// The codepoint ranges it claims. Empty means "everything not claimed by
    /// another sub-font", which is what a default typeface wants.
    pub nitaqat: Vec<NitaqHuruf>,
    /// The culture tags it claims. Empty means every culture.
    pub thaqafat: Vec<String>,
}

impl fmt::Debug for FaraiKhatt {
    /// Prints the byte count rather than the bytes.
    ///
    /// A derived `Debug` here would put a whole font into any log line or
    /// diagnostics bundle that formatted a composite, which is a megabyte of
    /// hexadecimal where a maintainer wanted one line.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FaraiKhatt")
            .field("ism", &self.ism)
            .field("bayt", &format_args!("{} bytes", self.bayt.len()))
            .field("nitaqat", &self.nitaqat)
            .field("thaqafat", &self.thaqafat)
            .finish()
    }
}

impl FaraiKhatt {
    /// Builds a sub-font from bytes the caller supplies.
    ///
    /// There is deliberately no variant of this that takes a path.
    #[must_use]
    pub fn jadeed(ism: impl Into<String>, bayt: Arc<[u8]>) -> Self {
        Self { ism: ism.into(), bayt, nitaqat: Vec::new(), thaqafat: Vec::new() }
    }

    /// Builds the Arabic sub-font: the Arabic ranges, claimed for every culture.
    ///
    /// Every culture rather than `ar` alone, because Arabic text appears in
    /// games whose active culture is English — a character name, a quoted line,
    /// a location — and a sub-font that claimed only `ar` would leave that text
    /// unshaped for every player who did not switch language.
    #[must_use]
    pub fn arabi(ism: impl Into<String>, bayt: Arc<[u8]>) -> Self {
        Self { ism: ism.into(), bayt, nitaqat: nitaqat_arabiya(), thaqafat: Vec::new() }
    }

    /// Restricts this sub-font to a set of ranges.
    #[must_use]
    pub fn bi_nitaqat(mut self, nitaqat: Vec<NitaqHuruf>) -> Self {
        self.nitaqat = nitaqat;
        self
    }

    /// Restricts this sub-font to a set of culture tags.
    #[must_use]
    pub fn bi_thaqafat(mut self, thaqafat: Vec<String>) -> Self {
        self.thaqafat = thaqafat;
        self
    }

    /// The ranges in the pair form the [`Musaddir`] seam takes.
    #[must_use]
    pub fn azwaj(&self) -> Vec<(u32, u32)> {
        self.nitaqat.iter().map(|nitaq| nitaq.zawj()).collect()
    }

    /// Checks the bytes and the ranges.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhattMarfud`] naming what was wrong: an empty or unnamed
    /// sub-font, bytes that are not an SFNT font, bytes past
    /// [`AQSA_HAJM_KHATT`], or a range that names codepoints no string can
    /// contain.
    pub fn tahaqquq(&self) -> Result<SighatKhatt, KhataUnreal> {
        if self.ism.trim().is_empty() {
            return Err(KhataUnreal::KhattMarfud {
                sabab: "a sub-font with no name cannot be referred to by a fallback chain"
                    .to_owned(),
            });
        }
        let tul = tul_u64(self.bayt.len());
        if tul == 0 {
            return Err(KhataUnreal::KhattMarfud {
                sabab: format!("sub-font \"{}\" carries no bytes", self.ism),
            });
        }
        if tul > AQSA_HAJM_KHATT {
            return Err(KhataUnreal::KhattMarfud {
                sabab: format!(
                    "sub-font \"{}\" is {tul} bytes, above the {AQSA_HAJM_KHATT} ceiling",
                    self.ism
                ),
            });
        }
        for nitaq in &self.nitaqat {
            if !nitaq.salih() {
                return Err(KhataUnreal::KhattMarfud {
                    sabab: format!(
                        "sub-font \"{}\" claims U+{:04X}..U+{:04X}, which is not a range of \
                         Unicode scalar values",
                        self.ism, nitaq.awwal, nitaq.akhir
                    ),
                });
            }
        }
        sighat_bayt(&self.bayt).ok_or_else(|| KhataUnreal::KhattMarfud {
            sabab: format!(
                "sub-font \"{}\" is not a TrueType or OpenType font; Slate's FreeType build \
                 reads SFNT and would load a WOFF as nothing at all",
                self.ism
            ),
        })
    }
}

/// A Slate composite font: a default typeface, sub-typefaces, and a fallback
/// order.
#[derive(Debug, Clone)]
pub struct KhattMurakkab {
    /// The default typeface. Everything no sub-font claims is drawn from this.
    pub asasi: FaraiKhatt,
    /// The sub-typefaces, each claiming ranges and cultures.
    pub farai: Vec<FaraiKhatt>,
    /// The names of the sub-fonts to consult, in order, for a codepoint that no
    /// typeface above provided a glyph for.
    pub irtida: Vec<String>,
}

impl KhattMurakkab {
    /// Builds the ordinary composite: the patch's Latin face as the default, the
    /// patch's Arabic face claiming the Arabic ranges, and the Arabic face as
    /// the first fallback.
    ///
    /// Both runs of bytes are the caller's — the patch's own. The Arabic face is
    /// first in the fallback order because a codepoint that reached the fallback
    /// at all is one the default face did not have, and in an Arabized game the
    /// overwhelmingly likely reason is that it is Arabic.
    ///
    /// # Errors
    ///
    /// Whatever [`KhattMurakkab::tahaqquq`] refuses.
    pub fn qiyasi(asasi: Arc<[u8]>, arabi: Arc<[u8]>) -> Result<Self, KhataUnreal> {
        let asasi = FaraiKhatt::jadeed("taarib-asasi", asasi);
        let arabi = FaraiKhatt::arabi("taarib-arabi", arabi);
        let irtida = vec![arabi.ism.clone()];
        let murakkab = Self { asasi, farai: vec![arabi], irtida };
        murakkab.tahaqquq()?;
        Ok(murakkab)
    }

    /// Builds a composite from parts the caller assembled.
    ///
    /// # Errors
    ///
    /// Whatever [`KhattMurakkab::tahaqquq`] refuses.
    pub fn jadeed(
        asasi: FaraiKhatt,
        farai: Vec<FaraiKhatt>,
        irtida: Vec<String>,
    ) -> Result<Self, KhataUnreal> {
        let murakkab = Self { asasi, farai, irtida };
        murakkab.tahaqquq()?;
        Ok(murakkab)
    }

    /// Every sub-font, the default first.
    pub fn kull(&self) -> impl Iterator<Item = &FaraiKhatt> {
        std::iter::once(&self.asasi).chain(self.farai.iter())
    }

    /// The names in the fallback order.
    #[must_use]
    pub fn asmaa_irtida(&self) -> Vec<&str> {
        self.irtida.iter().map(String::as_str).collect()
    }

    /// Checks every sub-font, the names, and the fallback chain.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhattMarfud`] naming what was wrong: whatever
    /// [`FaraiKhatt::tahaqquq`] refuses, a name used twice — which makes a
    /// fallback entry ambiguous and the glyph that gets drawn a coin toss — or a
    /// fallback entry naming a sub-font that is not in the composite, which
    /// would be a silent hole in coverage rather than an error at draw time.
    pub fn tahaqquq(&self) -> Result<(), KhataUnreal> {
        let mut asmaa: Vec<&str> = Vec::new();
        for farai in self.kull() {
            let _ = farai.tahaqquq()?;
            let ism = farai.ism.as_str();
            if asmaa.iter().any(|mawjud| mawjud.eq_ignore_ascii_case(ism)) {
                return Err(KhataUnreal::KhattMarfud {
                    sabab: format!("two sub-fonts are both named \"{ism}\""),
                });
            }
            asmaa.push(ism);
        }
        for madkhal in &self.irtida {
            if !asmaa.iter().any(|ism| ism.eq_ignore_ascii_case(madkhal)) {
                return Err(KhataUnreal::KhattMarfud {
                    sabab: format!(
                        "the fallback chain names \"{madkhal}\", which is not a sub-font of \
                         this composite"
                    ),
                });
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Registering the patch's font with Slate, down the ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Khatt {
    ini: PathBuf,
    asalib: Vec<String>,
    masar: Option<PathBuf>,
    miftah: String,
}

impl Khatt {
    /// Builds the registration against one game's `Engine.ini`.
    ///
    /// No styles are named. The patch names them: which Slate styles a font
    /// belongs to is content, and an adapter that picked them would be making a
    /// policy decision it is not allowed to make.
    #[must_use]
    pub fn jadeed(ini: impl Into<PathBuf>) -> Self {
        Self {
            ini: ini.into(),
            asalib: Vec::new(),
            masar: None,
            miftah: MIFTAH_KHATT_IHTIYATI.to_owned(),
        }
    }

    /// Names the Slate styles the composite is applied to.
    #[must_use]
    pub fn bi_asalib(mut self, asalib: Vec<String>) -> Self {
        self.asalib = asalib;
        self
    }

    /// Adds one style.
    #[must_use]
    pub fn bi_uslub(mut self, uslub: impl Into<String>) -> Self {
        self.asalib.push(uslub.into());
        self
    }

    /// Supplies the on-disk location of the patch's own font, for the ini rung.
    ///
    /// Written into the ini as text and never opened by this module. Refused by
    /// [`masar_masmuh`] when it names a cooked asset or a container.
    #[must_use]
    pub fn bi_masar_khatt(mut self, masar: impl Into<PathBuf>) -> Self {
        self.masar = Some(masar.into());
        self
    }

    /// Overrides the ini key the fallback font is written under, for an engine
    /// version that spells it differently.
    #[must_use]
    pub fn bi_miftah(mut self, miftah: impl Into<String>) -> Self {
        self.miftah = miftah.into();
        self
    }

    /// The ini this registration writes into.
    #[must_use]
    pub fn ini(&self) -> &Path {
        &self.ini
    }

    /// The styles the patch named.
    #[must_use]
    pub fn asalib(&self) -> &[String] {
        &self.asalib
    }

    /// The ini entries this registration owns.
    ///
    /// # Errors
    ///
    /// [`KhattMarfud`](KhataUnreal::KhattMarfud) when no font path was supplied,
    /// or when the supplied path names a cooked Unreal asset or a container —
    /// the check that makes "this module cannot be pointed at the game's font
    /// asset" true rather than merely intended.
    pub fn madakhil(&self) -> Result<Vec<MadkhalIni>, KhataUnreal> {
        let Some(masar) = self.masar.as_deref() else {
            return Err(KhataUnreal::KhattMarfud {
                sabab: "no font path was supplied, so there is nothing to name in the ini"
                    .to_owned(),
            });
        };
        if !masar_masmuh(masar) {
            return Err(KhataUnreal::KhattMarfud {
                sabab: format!(
                    "{} names a cooked Unreal asset, a container, or a path that is not \
                     UTF-8; Taarib registers only its own font file",
                    masar.display()
                ),
            });
        }
        let Some(nass) = masar.to_str() else {
            return Err(KhataUnreal::KhattMarfud {
                sabab: "the font path is not valid UTF-8".to_owned(),
            });
        };
        Ok(vec![MadkhalIni::jadeed(self.miftah.as_str(), nass)])
    }

    /// Rung one: name the patch's font in `[Internationalization]`.
    ///
    /// # Errors
    ///
    /// Whatever [`Khatt::madakhil`] or [`aktub_madakhil`] refuses.
    pub fn rutbat_idadat(&self) -> Result<(), KhataUnreal> {
        let madakhil = self.madakhil()?;
        aktub_madakhil(&self.ini, QISM_TADWEEL, &madakhil)
    }

    /// Rung two: the same key as a launch option.
    #[must_use]
    pub fn rutbat_satr(&self) -> Vec<String> {
        self.madakhil().map_or_else(
            |_| Vec::new(),
            |madakhil| {
                madakhil
                    .iter()
                    .map(|madkhal| {
                        format!(
                            "-ini:Engine:[{QISM_TADWEEL}]:{}={}",
                            madkhal.miftah, madkhal.qeema
                        )
                    })
                    .collect()
            },
        )
    }

    /// Rung three: build the composite in the live process and point the
    /// patch's styles at it.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the binding has no handle on the
    /// engine or does not reach the font system, and
    /// [`KhataUnreal::KhattMarfud`] when the composite is malformed or the font
    /// system rejected it.
    pub fn rutbat_haqn(
        &self,
        murakkab: &KhattMurakkab,
        musaddir: &dyn Musaddir,
    ) -> Result<(), KhataUnreal> {
        murakkab.tahaqquq()?;
        if !musaddir.muhayya() {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: "the Slate binding reports no handle on this process yet".to_owned(),
            });
        }

        for farai in murakkab.kull() {
            let azwaj = farai.azwaj();
            let thaqafat: Vec<&str> = farai.thaqafat.iter().map(String::as_str).collect();
            musaddir.sajjil_farai(&farai.ism, &farai.bayt, &azwaj, &thaqafat)?;
        }

        let irtida = murakkab.asmaa_irtida();
        for uslub in &self.asalib {
            musaddir.atbiq_murakkab(uslub, &murakkab.asasi.ism, &irtida)?;
        }
        Ok(())
    }

    /// Walks the ladder and reports which rung did what.
    ///
    /// Rung one is written whether or not a process is available, because it is
    /// what makes the *next* launch need no injection; rung three runs when a
    /// process is available, because it is what fixes the frame the player is
    /// looking at. See this module's header for why rung one is never recorded
    /// as confirmed.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhattMarfud`] naming every route that was tried, and only
    /// when no rung did anything. This is a warning in the error contract: a
    /// game that draws Arabic through its own font with full shaping on is a
    /// translated game that looks slightly wrong, which is a very long way from
    /// a game that did not start.
    pub fn sajjil(
        &self,
        murakkab: &KhattMurakkab,
        musaddir: Option<&dyn Musaddir>,
    ) -> Result<SijillTashghil, KhataUnreal> {
        murakkab.tahaqquq()?;
        let mut sijill = SijillTashghil::default();

        let mut shay_najah = false;
        match self.rutbat_idadat() {
            Ok(()) => {
                shay_najah = true;
                sijill.sajjil(
                    Rutba::Idadat,
                    false,
                    format!(
                        "the patch's font is named under [{QISM_TADWEEL}]; it binds at the \
                         next launch and cannot be confirmed from this one"
                    ),
                );
            }
            Err(khata) => sijill.sajjil(Rutba::Idadat, false, khata.to_string()),
        }

        let khiyarat = self.rutbat_satr();
        sijill.sajjil(
            Rutba::SatrAwamir,
            false,
            format!("{} launch options offered to the installer", khiyarat.len()),
        );

        if let Some(musaddir) = musaddir {
            match self.rutbat_haqn(murakkab, musaddir) {
                Ok(()) => {
                    shay_najah = true;
                    sijill.sajjil(
                        Rutba::Haqn,
                        true,
                        format!(
                            "{} sub-font(s) registered and applied to {} style(s)",
                            murakkab.farai.len() + 1,
                            self.asalib.len()
                        ),
                    );
                }
                Err(khata) => sijill.sajjil(Rutba::Haqn, false, khata.to_string()),
            }
        }

        if self.asalib.is_empty() {
            tracing::warn!(
                "the patch named no Slate styles, so the composite font was registered and \
                 applied to nothing"
            );
        }

        if shay_najah {
            Ok(sijill)
        } else {
            Err(KhataUnreal::KhattMarfud { sabab: sijill.sabab() })
        }
    }

    /// Removes everything Taarib wrote into this ini.
    ///
    /// Every Taarib block in the file, not only this module's — see
    /// [`crate::tashghil::Tashghil::tarajua`] for why undo is not per-module.
    ///
    /// # Errors
    ///
    /// Whatever [`tarajua_madakhil`] refuses.
    pub fn tarajua(&self) -> Result<(), KhataUnreal> {
        tarajua_madakhil(&self.ini)
    }
}
