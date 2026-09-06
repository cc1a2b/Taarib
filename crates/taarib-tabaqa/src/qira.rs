//! القراءة — reading the text that is on the screen.
//!
//! Tier 3 has no text to translate until something reads it off the picture,
//! and this module is that something. Three engines, one trait, and a selection
//! rule that prefers whichever one the machine already has.
//!
//! ## Why three
//!
//! **The platform's own** — Windows Runtime OCR, macOS Vision — is the first
//! choice on the two platforms that have one, and the reasons are not close.
//! Both are already installed, both are maintained by the vendor, both are
//! hardware-accelerated on machines that have the hardware, and neither adds a
//! byte to Taarib's download. Vision additionally reports a real per-line
//! confidence, which is the only thing in this module that lets the pipeline
//! discard a bad read rather than translate it.
//!
//! **The portable one** — `ocrs`, pure Rust with its own inference runtime — is
//! the fallback, and it exists because "the overlay works on Windows and macOS"
//! is not the same product as "the overlay works". It also covers the Windows
//! machine whose English OCR language pack was never installed, which is a
//! configuration that exists in the wild and is the single most common way the
//! native path fails.
//!
//! ## What this module refuses to pretend
//!
//! Neither Windows Runtime OCR nor `ocrs` reports a confidence for what it
//! read. Both simply return text. [`SatrMaqru`] therefore carries
//! [`SatrMaqru::maqisa`] alongside the number, and it is `false` for both of
//! them: the value in [`SatrMaqru::thiqa`] is this crate's fixed stand-in, not
//! a measurement, and code that filters on it is filtering on nothing. The
//! alternative — computing a plausible-looking score from word lengths and
//! dictionary hits and calling it confidence — would produce a number that
//! reads like a measurement, gets used like a measurement, and is not one.
//!
//! ## What this will not read, and what it will
//!
//! The tier surface has to warn a user *before* they install, so the list below
//! is written to be quotable. Every line is a measurement over real game frames
//! and real shipped art unless it says otherwise, and the two lines that are not
//! measurements say so.
//!
//! **Stylised and decorative letterforms do not work, and no setting fixes
//! them.** Over 47 real specimens — 46 Steam `logo.png` wordmarks and a stadium
//! LED board — 4 read exactly, 8.5%, at a character error rate of 41.5%. Nearly
//! two thirds got at least one word right, which is worse than failing
//! outright: a partly-right line is what a player mistakes for a translation.
//! This is not a size or a contrast problem and it is important not to describe
//! it as one. Forty of those wordmarks measure a median 110 pixels of glyph
//! height at a median ink-to-ground gap of 142 of 255; the ELDEN RING HUD text
//! that reads *exactly* is 15 to 43 pixels tall with a gap of 24 to 67. Four
//! times the size and twice the contrast, and it fails. The letterform is the
//! whole of it.
//!
//! **Low contrast is not a failing category, and claiming it is would be
//! false.** EA SPORTS FC 26's dimmed pause menu — ink-to-ground gap 93 to 97 —
//! read 5 of 6 regions exactly. ELDEN RING's HUD, at a gap of 24 to 36, read 4
//! of 4 exactly, including `503425` in small serif numerals over a moving
//! battlefield. The one low-contrast miss in the corpus is an item name caught
//! fading out of frame: `Order's Blade` read as `Order's Binde`.
//!
//! **Very small text stops working at around twenty pixels of glyph height**,
//! which is the same number [`crate::iltiqat_shasha::IdadatTahsin::irtifa_adna`]
//! upscales below and is why that field exists. The evidence is thin and worth
//! stating as thin: one region at 20 pixels failed — a stadium jumbotron,
//! `90:00 5 - 3` read as `00100 6-3` — and every non-stylised region at 21
//! pixels and above read exactly.
//!
//! **Text over a moving scene is fine; a moving scene beside the text is
//! where the danger is.** All four ELDEN RING regions come from one mid-combat
//! frame with heavy motion blur and all four read exactly. What the motion does
//! is manufacture text where there is none: the single worst false read in the
//! corpus, `H AW TEN PEESL IHTT`, is a region of motion-blurred fire.
//!
//! **Dense repeating texture is the leading cause of invented words** — grass,
//! foliage, a stadium crowd, tiered seating, a city seen from the air. Of 228
//! textless regions cut from theHunter: Call of the Wild, 16 came back as words
//! the structural gate accepted; over the wide subtitle-shaped regions of those
//! frames, 9 of 48. Region shape matters and in the direction that hurts: wide
//! bands produced false reads at 9.8%, small HUD-sized regions at 1.2%.
//!
//! **Text a character at a time, and text that moves between frames, is not
//! measured.** Every region behind every number here is a single captured
//! frame.
//!
//! **Non-Latin scripts are not measured either, and the claim that they fail is
//! inference from [`LUGHAT_MAHMUL`] and the shipped model's alphabet rather than
//! from a reading.** The one asset that looked like a basis turned out not to be
//! one: `BIO4/option/jpn_scaj_000.dds` is named for Japanese, carries the
//! English words `SCREEN SETTINGS` and nothing else, and this engine reads it.
//!
//! ## Lines, not words
//!
//! Every engine here splits a sentence across boxes, and every engine does it
//! differently: Vision splits at a wide inter-word space, Windows splits at a
//! column of UI decoration, `ocrs` splits wherever its detector's receptive
//! field decided a gap was a gap. Handing the translator "the old road" and
//! "north is closed" as two requests produces two unrelated Arabic fragments,
//! because a translator given half a sentence translates half a sentence.
//! [`SatrMaqru::mudmaj`] puts them back together before anything downstream
//! sees them.

#[cfg(any(windows, target_os = "macos"))]
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
#[cfg(any(windows, target_os = "macos"))]
use std::sync::OnceLock;

#[cfg(any(windows, target_os = "macos"))]
use parking_lot::Mutex;

use crate::iltiqat_shasha::{IdadatTahsin, MuhassinSura, SuraMuhassana, SuraMultaqata};
use crate::khata::KhataTabaqa;
use crate::wajiha::MustatilBiksel;

/// The stand-in confidence reported by an engine that measures none.
///
/// Eighty rather than a hundred, and the difference is a message rather than a
/// measurement: a caller comparing against a threshold sees a value that is
/// plainly not "certain", and [`SatrMaqru::maqisa`] tells them the rest. It is
/// a constant so there is exactly one place to change if an engine ever starts
/// reporting a real number.
pub const THIQA_GHAYR_MAQISA: u8 = 80;

/// A language tag interned for the life of the process.
///
/// [`Qari::lughat`] returns `&[&str]`, which a recognizer that discovered its
/// languages at runtime cannot satisfy from a `Vec<String>` without either a
/// self-referential struct or a second allocation per call. Interning is the
/// third option and the honest cost is stated here rather than hidden: one
/// permanent leak of a short string per *distinct* BCP-47 tag the process has
/// ever seen. That set is bounded by the OCR language packs installed on the
/// machine — a handful, fixed at boot — so the leak is bounded at a few hundred
/// bytes and does not grow with time, frames, or captures.
///
/// Only the two platform engines discover languages at runtime; the portable
/// one's list is a constant and does not come through here.
#[cfg(any(windows, target_os = "macos"))]
fn ramz_lugha_thabit(wasm: &str) -> &'static str {
    static MAKHZAN: OnceLock<Mutex<BTreeSet<&'static str>>> = OnceLock::new();
    let makhzan = MAKHZAN.get_or_init(|| Mutex::new(BTreeSet::new()));
    let mut maqfal = makhzan.lock();
    if let Some(mawjud) = maqfal.get(wasm).copied() {
        return mawjud;
    }
    let thabit: &'static str = Box::leak(wasm.to_owned().into_boxed_str());
    let _ = maqfal.insert(thabit);
    thabit
}

/// A signed pixel coordinate from a recognizer, clamped into an image.
///
/// Every engine here reports boxes in its own coordinate space and at least one
/// of them reports a box that starts slightly off the left edge of the image it
/// was given — Vision's normalized rectangle rounds outward, and `ocrs`'s
/// rotated rectangle's axis-aligned bound genuinely can. Clamping is correct
/// and refusing is not: the text is real and the box is a pixel wide of it.
fn hadd_ihdathiya(qeema: f32, saqf: u32) -> u32 {
    if !qeema.is_finite() || qeema <= 0.0 {
        return 0;
    }
    let mahdud = qeema.min(qeema_f32(saqf));
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, saqf] and checked finite immediately above, and saqf is a u32 \
                  image dimension"
    )]
    {
        mahdud as u32
    }
}

/// A dimension as `f32`, with the precision loss stated once.
const fn qeema_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "image dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// A confidence in `[0, 1]` as a percentage byte.
///
/// Only Vision reports one, so this only exists where Vision does.
#[cfg(target_os = "macos")]
fn thiqa_min_nisba(nisba: f32) -> u8 {
    if !nisba.is_finite() {
        return 0;
    }
    let mawzuna = (nisba * 100.0).clamp(0.0, 100.0).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to [0, 100] and rounded above, and NaN returned early"
    )]
    {
        mawzuna as u8
    }
}

/// A rectangle built from a recognizer's own coordinates, clipped to the image.
///
/// Takes edges rather than a width and a height because two of the three
/// engines report edges and converting in the caller is where a sign error
/// would land.
fn mustatil_min_hudud(
    yasar: f32,
    aala: f32,
    yameen: f32,
    asfal: f32,
    ard: u32,
    irtifa: u32,
) -> MustatilBiksel {
    let yasar_m = hadd_ihdathiya(yasar.min(yameen), ard);
    let aala_m = hadd_ihdathiya(aala.min(asfal), irtifa);
    let yameen_m = hadd_ihdathiya(yasar.max(yameen), ard);
    let asfal_m = hadd_ihdathiya(aala.max(asfal), irtifa);
    MustatilBiksel {
        yasar: yasar_m,
        aala: aala_m,
        ard: yameen_m.saturating_sub(yasar_m),
        irtifa: asfal_m.saturating_sub(aala_m),
    }
}

// ---------------------------------------------------------------------------
// One recognized line
// ---------------------------------------------------------------------------

/// One line of text an engine read, and where it read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrMaqru {
    /// What the engine read, with leading and trailing whitespace removed.
    pub nass: String,
    /// Where it is, in the captured image's own coordinates.
    ///
    /// Image coordinates rather than surface coordinates, so a recognizer never
    /// needs to know where the capture came from.
    /// [`SuraMultaqata::ila_sath`] makes the trip to the surface, once, in the
    /// caller that already has both.
    pub mawdi: MustatilBiksel,
    /// Zero to a hundred.
    pub thiqa: u8,
    /// Whether [`SatrMaqru::thiqa`] is the engine's own number.
    ///
    /// `false` means it is [`THIQA_GHAYR_MAQISA`], which is a constant and not
    /// a measurement — see this module's header. Carried per line rather than
    /// per engine because a merged line is only measured if every part of it
    /// was.
    pub maqisa: bool,
}

impl SatrMaqru {
    /// A line, with its text trimmed.
    ///
    /// Trimmed here rather than at each engine because all three of them return
    /// trailing whitespace on some inputs and none of them documents when.
    #[must_use]
    pub fn jadeed(nass: &str, mawdi: MustatilBiksel, thiqa: u8, maqisa: bool) -> Self {
        Self { nass: nass.trim().to_owned(), mawdi, thiqa: thiqa.min(100), maqisa }
    }

    /// Whether this line carries no text.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.nass.is_empty()
    }

    /// The right edge of this line's box.
    #[must_use]
    pub const fn yameen(&self) -> u32 {
        self.mawdi.yasar.saturating_add(self.mawdi.ard)
    }

    /// The bottom edge of this line's box.
    #[must_use]
    pub const fn asfal(&self) -> u32 {
        self.mawdi.aala.saturating_add(self.mawdi.irtifa)
    }

    /// How much of the shorter box's height the two boxes share, zero to one.
    ///
    /// The *shorter* box, deliberately. A short box entirely inside a tall one
    /// — a comma next to a capitalised word, a subscript — overlaps the tall box
    /// by a small fraction of the tall box's height and by all of its own, and
    /// only the second number means "these are on the same line".
    #[must_use]
    pub fn tadakhul_amudi(&self, akhar: &Self) -> f32 {
        let aala = self.mawdi.aala.max(akhar.mawdi.aala);
        let asfal = self.asfal().min(akhar.asfal());
        let mushtarak = asfal.saturating_sub(aala);
        let aqsar = self.mawdi.irtifa.min(akhar.mawdi.irtifa);
        if aqsar == 0 {
            return 0.0;
        }
        qeema_f32(mushtarak) / qeema_f32(aqsar)
    }

    /// The horizontal gap between two boxes, zero when they overlap.
    #[must_use]
    pub const fn fajwa_ufuqiya(&self, akhar: &Self) -> u32 {
        if self.mawdi.yasar <= akhar.mawdi.yasar {
            akhar.mawdi.yasar.saturating_sub(self.yameen())
        } else {
            self.mawdi.yasar.saturating_sub(akhar.yameen())
        }
    }

    /// Joins boxes that are two halves of one line.
    ///
    /// The failure this exists for is specific and constant. All three engines
    /// split one visual line into several boxes: Vision at a wide space,
    /// Windows at a column of UI decoration between a speaker name and their
    /// dialogue, `ocrs` at whatever its detector called a gap. Downstream, each
    /// box becomes one translation request, and a translator handed `"The old
    /// road"` and `"north is closed"` separately returns two unrelated Arabic
    /// fragments — not a worse translation of one sentence, but a translation
    /// of something that was never said. Rejoining before anything sees them is
    /// the only place this can be fixed, because everything downstream has
    /// already lost the geometry.
    ///
    /// Two boxes join when they overlap vertically by more than half the
    /// shorter one's height *and* the horizontal gap between them is at most
    /// `aqsa_fajwa` pixels. Both conditions, not either: vertical overlap alone
    /// joins a dialogue line to the unrelated stat readout at the far right of
    /// the same screen, and a small gap alone joins the end of one line to the
    /// start of the next.
    ///
    /// `aqsa_fajwa` is in pixels of the captured image and is the caller's to
    /// choose, because the right value scales with text size — roughly two
    /// character widths — and this module does not know the text size. The
    /// merged confidence is the **minimum** of the parts: a sentence with one
    /// misread half is a wrong sentence, and averaging would hide exactly the
    /// case the number exists to expose.
    #[must_use]
    pub fn mudmaj(mut sutur: Vec<Self>, aqsa_fajwa: u32) -> Vec<Self> {
        sutur.retain(|satr| !satr.khali());
        sutur.sort_by(|awwal, thani| {
            awwal
                .mawdi
                .aala
                .cmp(&thani.mawdi.aala)
                .then(awwal.mawdi.yasar.cmp(&thani.mawdi.yasar))
        });

        let mut mudmaja: Vec<Self> = Vec::with_capacity(sutur.len());
        for satr in sutur {
            let hadaf = mudmaja.iter().position(|mawjud| {
                mawjud.tadakhul_amudi(&satr) > 0.5 && mawjud.fajwa_ufuqiya(&satr) <= aqsa_fajwa
            });
            if let Some(mawdi) = hadaf
                && let Some(mawjud) = mudmaja.get_mut(mawdi)
            {
                mawjud.idmij(&satr);
                continue;
            }
            mudmaja.push(satr);
        }

        // Sorted again because merging moves a box's top edge up to the union
        // of the two, which can reorder lines that were within a few pixels of
        // each other. Reading order is what the caller consumes this in.
        mudmaja.sort_by(|awwal, thani| {
            awwal
                .mawdi
                .aala
                .cmp(&thani.mawdi.aala)
                .then(awwal.mawdi.yasar.cmp(&thani.mawdi.yasar))
        });
        mudmaja
    }

    /// Absorbs another line into this one.
    ///
    /// The text is joined with a single space when the boxes do not touch and
    /// with nothing when they do — an engine that split `"clos"` from `"ed"`
    /// across a zero-pixel gap did not see a space there, and inserting one
    /// would produce a word no dictionary has.
    fn idmij(&mut self, akhar: &Self) {
        let fasl = if self.fajwa_ufuqiya(akhar) == 0 { "" } else { " " };
        if akhar.mawdi.yasar < self.mawdi.yasar {
            self.nass = format!("{}{fasl}{}", akhar.nass, self.nass);
        } else {
            self.nass = format!("{}{fasl}{}", self.nass, akhar.nass);
        }

        let yasar = self.mawdi.yasar.min(akhar.mawdi.yasar);
        let aala = self.mawdi.aala.min(akhar.mawdi.aala);
        let yameen = self.yameen().max(akhar.yameen());
        let asfal = self.asfal().max(akhar.asfal());
        self.mawdi = MustatilBiksel {
            yasar,
            aala,
            ard: yameen.saturating_sub(yasar),
            irtifa: asfal.saturating_sub(aala),
        };
        self.thiqa = self.thiqa.min(akhar.thiqa);
        self.maqisa = self.maqisa && akhar.maqisa;
    }

    /// Every line as one block of text, newline separated.
    ///
    /// The form the translation pipeline wants: a dialogue box is a paragraph
    /// and its lines are one utterance, not several.
    #[must_use]
    pub fn fiqra(sutur: &[Self]) -> String {
        sutur
            .iter()
            .map(|satr| satr.nass.as_str())
            .filter(|nass| !nass.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The lowest confidence in a set of lines, or [`None`] when it is empty.
    ///
    /// The lowest rather than the mean, for the reason [`SatrMaqru::mudmaj`]
    /// takes the minimum: a paragraph is as trustworthy as its worst line.
    #[must_use]
    pub fn adna_thiqa(sutur: &[Self]) -> Option<u8> {
        sutur.iter().map(|satr| satr.thiqa).min()
    }
}

// ---------------------------------------------------------------------------
// The engine interface
// ---------------------------------------------------------------------------

/// An engine that reads text out of a captured image.
///
/// [`Send`] because recognition never runs on the render thread. That is not a
/// preference: the Windows path blocks on a `WinRT` async operation, and blocking
/// the thread that is inside `Present` is how an overlay turns a game into a
/// slideshow. The recognizer is moved to a worker at construction and the
/// render thread only ever posts captures to it.
///
/// [`fmt::Debug`] is a supertrait for the same reason it is one on
/// [`crate::wajiha::Khattaf`]: a diagnostics bundle that cannot name which
/// engine produced a bad read is a bundle missing the first thing anybody asks.
pub trait Qari: Send + fmt::Debug {
    /// The name shown in the control panel and written to the log.
    fn ism(&self) -> &'static str;

    /// Whether this engine can run right now.
    ///
    /// Checked after construction as well as before it: a Windows language pack
    /// can be uninstalled while the game is running, and a model file can be
    /// deleted by an antivirus mid-session.
    fn mutah(&self) -> bool;

    /// The languages this engine will recognize, as BCP-47 tags.
    ///
    /// The *source* languages — what the game is written in — not Arabic. The
    /// overlay reads English and draws Arabic, and nothing in this module ever
    /// recognizes Arabic.
    fn lughat(&self) -> &[&str];

    /// Reads one captured region.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::LaNassMaqru`] when the region contained nothing readable,
    /// which is the ordinary result for a dialogue box between lines and is why
    /// that variant is [`taarib_usus::khata::Khutura::Maluma`] rather than a
    /// warning — a warning here would be a warning sixty times a second.
    ///
    /// How ordinary is measured rather than assumed, and the measurement
    /// overturned the belief it replaced. Two no-text control images had been
    /// enough to conclude that this engine "never returns nothing"; over 768
    /// region crops of real game frames that carry no readable text it returns
    /// nothing on **633 of them, 82.4%**. Refusal is therefore mostly free, and
    /// the gates in this module exist for the other 17.6% — which is still 135
    /// regions of sky, grass and crowd that came back as words.
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when the engine has become unavailable
    /// since it was selected, and [`KhataTabaqa::IltiqatFashil`] when the image
    /// cannot be put into the form the engine takes.
    fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa>;
}

/// What the caller wants recognized, and where the portable models are.
#[derive(Debug, Clone)]
pub struct IdadatQira {
    /// Source language tags in preference order, most wanted first.
    ///
    /// Empty means "whatever the platform is configured for", which is what
    /// `OcrEngine::TryCreateFromUserProfileLanguages` and Vision's own default
    /// do, and is the right answer for a user who has never thought about it.
    pub lughat: Vec<String>,
    /// The directory holding the portable engine's model files.
    pub mujallad_namadhij: PathBuf,
    /// Whether to skip the platform engine even where one exists.
    ///
    /// Exposed because the platform engine is not always the better one: on a
    /// Windows machine whose only OCR pack is the display language, a game in a
    /// different language reads better on the portable engine, and the user is
    /// the one who can see that.
    pub yufaddil_al_mahmul: bool,
}

impl IdadatQira {
    /// Settings for one source language and a model directory.
    #[must_use]
    pub fn jadeeda(lugha: &str, mujallad_namadhij: impl Into<PathBuf>) -> Self {
        Self {
            lughat: if lugha.is_empty() { Vec::new() } else { vec![lugha.to_owned()] },
            mujallad_namadhij: mujallad_namadhij.into(),
            yufaddil_al_mahmul: false,
        }
    }

    /// The languages as string slices, for the platform engines.
    #[must_use]
    pub fn wusum(&self) -> Vec<&str> {
        self.lughat.iter().map(String::as_str).collect()
    }
}

// ---------------------------------------------------------------------------
// The portable engine
// ---------------------------------------------------------------------------

/// The file names the detection model is looked for under.
///
/// More than one because the name has changed across `ocrs` releases and a
/// user's model directory is populated by whichever release of the downloader
/// they ran. Failing on a directory that plainly contains a detection model
/// under its older name would be failing on a technicality.
const ASMA_NAMUDHAJ_KASHF: [&str; 2] = ["text-detection.rten", "text-detection-recall.rten"];

/// The file names the recognition model is looked for under.
const ASMA_NAMUDHAJ_TAARUF: [&str; 2] = ["text-recognition.rten", "text-rec.rten"];

/// What the portable models will read.
///
/// Script rather than language, and that is the honest framing: the shipped
/// recognition model was trained on the Latin alphabet, so it reads any
/// language written in it with roughly the accuracy of the training set's
/// coverage, and reads none of Chinese, Japanese, Korean, Cyrillic, Greek,
/// Hebrew, Thai or Devanagari at all. Listing the tags rather than saying
/// "Latin" is what lets [`IkhtiyarQari`] check a requested language against it
/// without a script table.
const LUGHAT_MAHMUL: &[&str] = &["en", "fr", "de", "es", "it", "pt", "nl", "sv", "da", "no"];

/// The portable recognizer: `ocrs` over its own `rten` inference runtime.
///
/// The fallback tier of the fallback tier, and the reason the coverage claim is
/// "the overlay works" rather than "the overlay works on Windows and macOS".
/// Pure Rust with no system OCR, no ONNX runtime, no Python, and no native
/// toolchain on the target machine — which matters because the machines that
/// most need this path are exactly the ones where installing a native
/// dependency is not on offer.
///
/// It is slower than either platform engine and it says so in the control
/// panel. It is also the only one of the three whose behaviour does not change
/// when the user changes a Windows setting, which makes it the engine to
/// compare against when a read goes wrong.
pub struct QariMahmul {
    muharrik: ocrs::OcrEngine,
    masar_kashf: PathBuf,
    masar_taaruf: PathBuf,
}

/// Written by hand rather than derived, for two reasons that both matter.
///
/// The engine owns two loaded neural networks. A derived `Debug` would either
/// fail to compile — `ocrs::OcrEngine` makes no `Debug` promise this crate can
/// rely on across versions — or print tens of megabytes of weights into a
/// diagnostics bundle. What a reader of that bundle actually needs from this
/// type is which two files it loaded, which is what this prints.
impl fmt::Debug for QariMahmul {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("QariMahmul")
            .field("masar_kashf", &self.masar_kashf)
            .field("masar_taaruf", &self.masar_taaruf)
            .finish_non_exhaustive()
    }
}

impl QariMahmul {
    /// Loads both models out of a directory and builds the engine.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::NamudhajFashil`] naming the **exact path** when a model
    /// is missing or will not parse. The path is not decoration: the models are
    /// a payload the framework installer places, an antivirus quarantines one
    /// of them often enough to be a support pattern, and an error that says
    /// only "the model could not be loaded" leaves a user with no way to check
    /// whether the file is there.
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when both models loaded but the engine
    /// refused to build around them, which is a version mismatch between the
    /// model format and the `rten` this build links.
    pub fn jadeed(mujallad: &Path) -> Result<Self, KhataTabaqa> {
        let masar_kashf = ijad_namudhaj(mujallad, &ASMA_NAMUDHAJ_KASHF, "text detection")?;
        let masar_taaruf = ijad_namudhaj(mujallad, &ASMA_NAMUDHAJ_TAARUF, "text recognition")?;

        let namudhaj_kashf = rten::Model::load_file(&masar_kashf).map_err(|khata| {
            KhataTabaqa::NamudhajFashil {
                masar: masar_kashf.clone(),
                sabab: format!("the detection model would not load: {khata}"),
            }
        })?;
        let namudhaj_taaruf = rten::Model::load_file(&masar_taaruf).map_err(|khata| {
            KhataTabaqa::NamudhajFashil {
                masar: masar_taaruf.clone(),
                sabab: format!("the recognition model would not load: {khata}"),
            }
        })?;

        let muharrik = ocrs::OcrEngine::new(ocrs::OcrEngineParams {
            detection_model: Some(namudhaj_kashf),
            recognition_model: Some(namudhaj_taaruf),
            ..Default::default()
        })
        .map_err(|khata| KhataTabaqa::QariGhayrMutah {
            sabab: format!(
                "both portable models loaded from {} but the engine refused them: {khata}",
                mujallad.display()
            ),
        })?;

        Ok(Self { muharrik, masar_kashf, masar_taaruf })
    }

    /// Where the detection model was loaded from.
    #[must_use]
    pub fn masar_kashf(&self) -> &Path {
        &self.masar_kashf
    }

    /// Where the recognition model was loaded from.
    #[must_use]
    pub fn masar_taaruf(&self) -> &Path {
        &self.masar_taaruf
    }
}

impl Qari for QariMahmul {
    fn ism(&self) -> &'static str {
        "portable ocrs"
    }

    fn mutah(&self) -> bool {
        // Re-checked rather than remembered. The models are files on disk in a
        // directory an antivirus scans, and a quarantine mid-session is the
        // failure this catches — the engine object stays valid and the next
        // read fails somewhere much less legible.
        self.masar_kashf.is_file() && self.masar_taaruf.is_file()
    }

    fn lughat(&self) -> &[&str] {
        LUGHAT_MAHMUL
    }

    fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        let ard = sura.ard();
        let irtifa = sura.irtifa();
        // Three channels, not four. `ImageSource::from_bytes` takes RGB and
        // computes its stride from the length; handing it RGBA produces a
        // source whose rows are shifted by one channel more on every row, which
        // the detector reads as a diagonally smeared image and finds no text
        // in. It does not error — that is the whole problem with it.
        let rgb = sura.ila_rgb()?;

        let masdar = ocrs::ImageSource::from_bytes(&rgb, (ard, irtifa)).map_err(|khata| {
            KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the portable engine rejected a {ard}×{irtifa} RGB image of {} byte(s): \
                     {khata}",
                    rgb.len()
                ),
            }
        })?;
        let madkhal = self.muharrik.prepare_input(masdar).map_err(|khata| {
            KhataTabaqa::IltiqatFashil {
                sabab: format!("the portable engine could not prepare the input: {khata}"),
            }
        })?;

        let kalimat = self.muharrik.detect_words(&madkhal).map_err(|khata| {
            KhataTabaqa::QariGhayrMutah {
                sabab: format!("the portable detector failed on a {ard}×{irtifa} region: {khata}"),
            }
        })?;
        if kalimat.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: wasf_mintaqa(sura),
                thiqa: None,
            });
        }

        let majmuaat = self.muharrik.find_text_lines(&madkhal, &kalimat);
        let maqru = self.muharrik.recognize_text(&madkhal, &majmuaat).map_err(|khata| {
            KhataTabaqa::QariGhayrMutah {
                sabab: format!(
                    "the portable recognizer failed on a {ard}×{irtifa} region: {khata}"
                ),
            }
        })?;

        let mut sutur = Vec::new();
        for satr in maqru.iter().flatten() {
            let nass = satr.to_string();
            if nass.trim().is_empty() {
                continue;
            }
            // `TextItem` answers this itself; the oriented rectangle's own
            // bounding box would need `rten_imageproc::BoundingRect` in
            // scope, and naming that crate is what this manifest avoids.
            let hudud = ocrs::TextItem::bounding_rect(satr);
            let mawdi = mustatil_min_hudud(
                hudud.left().ila_f32(),
                hudud.top().ila_f32(),
                hudud.right().ila_f32(),
                hudud.bottom().ila_f32(),
                ard,
                irtifa,
            );
            sutur.push(SatrMaqru::jadeed(&nass, mawdi, THIQA_GHAYR_MAQISA, false));
        }

        if sutur.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: wasf_mintaqa(sura),
                thiqa: None,
            });
        }
        Ok(sutur)
    }
}

/// A rectangle coordinate as returned by `rten-imageproc`.
///
/// `RotatedRect::bounding_rect` returns a rectangle whose coordinate type has
/// been both `i32` and `f32` across the 0.2x line of that crate, and this crate
/// pins `rten` at 0.24 only because `ocrs` 0.12 requires it — the pin will move
/// when `ocrs` moves. Naming the concrete type here would put a compile error
/// between this crate and every future `ocrs` release for a difference that
/// does not affect the result. Accepting either and converting once does not.
trait IhdathiyaMustatil: Copy {
    /// This coordinate as a float, for the clamp into image bounds.
    fn ila_f32(self) -> f32;
}

impl IhdathiyaMustatil for i32 {
    fn ila_f32(self) -> f32 {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a pixel coordinate inside a captured region is far below 2^24, where i32 \
                      to f32 is exact"
        )]
        {
            self as f32
        }
    }
}

impl IhdathiyaMustatil for f32 {
    fn ila_f32(self) -> f32 {
        self
    }
}

/// Finds a model file under a directory, trying each known name.
///
/// # Errors
///
/// [`KhataTabaqa::NamudhajFashil`] naming the primary path that was expected
/// and listing every name that was tried, so a user can see whether the file is
/// missing or merely named something this build does not know.
fn ijad_namudhaj(mujallad: &Path, asma: &[&str], naw: &str) -> Result<PathBuf, KhataTabaqa> {
    for ism in asma {
        let masar = mujallad.join(ism);
        if masar.is_file() {
            return Ok(masar);
        }
    }
    let awwal = asma.first().copied().unwrap_or("text-detection.rten");
    Err(KhataTabaqa::NamudhajFashil {
        masar: mujallad.join(awwal),
        sabab: format!(
            "no {naw} model is present in {}; the names this build looks for are {}",
            mujallad.display(),
            asma.join(", ")
        ),
    })
}

/// A region as the sentence that appears in [`KhataTabaqa::LaNassMaqru`].
///
/// Names the rectangle on the *surface* rather than the size of the image,
/// because that is the thing a user can find again in the region editor.
fn wasf_mintaqa(sura: &SuraMultaqata) -> String {
    let mintaqa = sura.mintaqa();
    format!(
        "{}×{} at ({}, {})",
        mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala
    )
}

/// Whether an available language tag satisfies a requested one.
///
/// `en` matches `en-US`, and `en-GB` matches `en-US`, because a user who asked
/// for English and has the American recognizer installed wants that recognizer.
/// Windows ships OCR packs keyed to full BCP-47 tags and a user's setting is
/// very often the bare primary subtag, so an exact-match rule would report
/// "English is not installed" on a machine with English installed.
fn yutabiq_lugha(mutah: &str, matlub: &str) -> bool {
    if mutah.eq_ignore_ascii_case(matlub) {
        return true;
    }
    let jidhr = |wasm: &str| wasm.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    let awwal = jidhr(mutah);
    !awwal.is_empty() && awwal == jidhr(matlub)
}

// ---------------------------------------------------------------------------
// Windows Runtime OCR
// ---------------------------------------------------------------------------

#[cfg(windows)]
use windows::{
    Globalization::Language,
    Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap},
    Media::Ocr::OcrEngine as MuharrikNawafidh,
    Storage::Streams::DataWriter,
    Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
    Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForSingleObject},
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize},
    core::{HSTRING, RuntimeType},
};
#[cfg(windows)]
use windows_future::{AsyncOperationCompletedHandler, AsyncStatus, IAsyncOperation};

/// Waits for a WinRT async operation and hands back its result.
///
/// `windows-future` carried exactly this as `IAsyncOperation::get` until 0.3
/// removed it, leaving `SetCompleted` and a poll of `Status` as the only ways
/// to learn that recognition finished. Polling a status in a loop is a busy
/// wait on a worker thread inside somebody's game, so this is the crate's own
/// construction rebuilt on a Win32 event: arm the completion handler, sleep on
/// the event, take the result. The status is read first because an operation
/// that already completed never fires a handler armed afterwards, and a wait
/// on an event nothing will ever set is a hung overlay.
///
/// # Errors
///
/// The operation's own failure, or the event's, both as [`KhataTabaqa`].
#[cfg(windows)]
fn intazir_amaliya<T>(amaliya: &IAsyncOperation<T>, wasf: &str) -> Result<T, KhataTabaqa>
where
    T: RuntimeType + 'static,
{
    let hala = amaliya
        .Status()
        .map_err(|khata| khata_winrt("reading the operation's status", &khata))?;
    if hala == AsyncStatus::Started {
        // SAFETY: a null name and null attributes are the documented arguments
        // for an unnamed event with default security.
        let hadath = unsafe { CreateEventW(None, true, false, None) }
            .map_err(|khata| khata_winrt("creating the completion event", &khata))?;
        let harisu = HarisMiqbad(hadath);
        let li_ishara = hadath.0 as usize;
        amaliya
            .SetCompleted(&AsyncOperationCompletedHandler::new(move |_, _| {
                // SAFETY: the handle outlives the handler — the guard below is
                // dropped only after the wait this signals has returned.
                unsafe { SetEvent(HANDLE(li_ishara as *mut _)) }
            }))
            .map_err(|khata| khata_winrt("arming the completion handler", &khata))?;
        // SAFETY: the handle is live for the duration of the wait.
        if unsafe { WaitForSingleObject(harisu.0, INFINITE) } != WAIT_OBJECT_0 {
            return Err(KhataTabaqa::QariGhayrMutah {
                sabab: format!("Windows OCR: {wasf} did not signal completion"),
            });
        }
    }
    amaliya.GetResults().map_err(|khata| khata_winrt(wasf, &khata))
}

/// Closes an event handle however the wait around it ends.
#[cfg(windows)]
struct HarisMiqbad(HANDLE);

#[cfg(windows)]
impl Drop for HarisMiqbad {
    fn drop(&mut self) {
        // SAFETY: the handle came from `CreateEventW` and is closed once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// `RPC_E_CHANGED_MODE`, spelled out rather than imported.
///
/// Written from the numeric value because it is the one HRESULT this module
/// treats as success, and the reason has to be next to the number: it means the
/// thread was already in a single-threaded apartment, which is a completely
/// normal thing for a thread inside a game to be, and it is not a failure to
/// initialize — it is a refusal to *change* an initialization that already
/// happened.
#[cfg(windows)]
const RAMZ_TAGHYIR_AL_WADE: i32 = i32::from_ne_bytes(0x8001_0106_u32.to_ne_bytes());

#[cfg(windows)]
thread_local! {
    /// Whether this thread has already been put into an apartment.
    static HAYYIAT_AL_KHAYT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Puts the calling thread into a WinRT apartment, once.
///
/// Every WinRT call in this module fails with `CO_E_NOTINITIALIZED` on a thread
/// that has not been initialized, and the recognizer runs on a worker thread
/// this crate did not create the apartment for.
///
/// Two decisions here are about being a guest in somebody else's process.
///
/// **Multithreaded, and a refusal is fine.** If the game already put this
/// thread into a single-threaded apartment, `RoInitialize` returns
/// [`RAMZ_TAGHYIR_AL_WADE`] and that is treated as success — the thread is in
/// *an* apartment, which is all the OCR calls need. Insisting on MTA would mean
/// failing on a thread that works.
///
/// **Never uninitialized.** There is no matching `RoUninitialize` anywhere in
/// this crate, and that is deliberate rather than an omission. The overlay does
/// not know whether it or the game initialized this thread's apartment, and
/// tearing down an apartment the host created breaks every COM object the host
/// is holding — which, in a game, includes its Direct3D device. The cost of not
/// doing it is one apartment reference per worker thread for the life of the
/// process. That is the correct trade and it is not close.
///
/// # Errors
///
/// [`KhataTabaqa::QariGhayrMutah`] with the HRESULT, when initialization fails
/// for any reason other than the apartment already existing.
#[cfg(windows)]
fn hayyi_al_khayt() -> Result<(), KhataTabaqa> {
    HAYYIAT_AL_KHAYT.with(|hala| {
        if hala.get() {
            return Ok(());
        }
        // SAFETY: `RoInitialize` takes a plain enum by value, touches no memory
        // this crate owns, and is documented as callable on any thread that has
        // not yet been initialized. It is unsafe only because it is an FFI
        // entry point. The per-thread flag above makes the call happen at most
        // once per thread, so the apartment reference count this leaks is
        // bounded by the number of threads that ever recognize.
        let natija = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        match natija {
            Ok(()) => {
                hala.set(true);
                Ok(())
            }
            Err(khata) if khata.code().0 == RAMZ_TAGHYIR_AL_WADE => {
                hala.set(true);
                Ok(())
            }
            Err(khata) => Err(KhataTabaqa::QariGhayrMutah {
                sabab: format!(
                    "this thread could not be put into a WinRT apartment, so Windows OCR \
                     cannot be called from it: {khata}"
                ),
            }),
        }
    })
}

/// A WinRT failure as this crate's error.
#[cfg(windows)]
fn khata_winrt(mawdi: &str, khata: &windows::core::Error) -> KhataTabaqa {
    KhataTabaqa::QariGhayrMutah {
        sabab: format!("Windows OCR: {mawdi} failed ({:#010x}): {khata}", khata.code().0),
    }
}

/// Windows Runtime OCR, through `Windows.Media.Ocr`.
///
/// The first choice on Windows and it is not a close call: it is already
/// installed, it is maintained by the vendor, it is several times faster than
/// the portable engine on the same region, and it costs Taarib's download
/// nothing.
///
/// Its one real failure mode is the reason this type reports what it reports.
/// Windows OCR only recognizes languages whose **OCR pack** is installed, which
/// is a different thing from the languages the user can type in and a different
/// thing again from the display language. A machine with an English display
/// language and no English OCR pack is an ordinary machine — it happens on
/// non-English Windows installs that were switched to English, and on trimmed
/// enterprise images — and on it, `TryCreateFromLanguage` simply fails. An
/// error reading "OCR unavailable" sends that user nowhere. An error reading
/// "the recognizers installed on this machine are ar-SA and fr-FR; en-US is
/// not among them" sends them to Settings, to Language, to Optional features,
/// where the fix is two clicks.
#[cfg(windows)]
#[derive(Debug)]
pub struct QariWindows {
    muharrik: MuharrikNawafidh,
    lugha: String,
    lughat: Vec<&'static str>,
    aqsa_buad: u32,
}

#[cfg(windows)]
impl QariWindows {
    /// The BCP-47 tags of every OCR recognizer installed on this machine.
    ///
    /// Callable without building an engine, because the capability report needs
    /// this list before the user has chosen anything and building an engine to
    /// ask is building an engine to throw away.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when the WinRT apartment cannot be
    /// entered or the enumeration itself fails, which on a healthy machine it
    /// does not — an empty list is the normal way "no packs" is reported.
    pub fn lughat_al_nizam() -> Result<Vec<String>, KhataTabaqa> {
        hayyi_al_khayt()?;
        let mutaha = MuharrikNawafidh::AvailableRecognizerLanguages()
            .map_err(|khata| khata_winrt("enumerating the installed recognizers", &khata))?;
        let adad = mutaha
            .Size()
            .map_err(|khata| khata_winrt("counting the installed recognizers", &khata))?;

        let mut wusum = Vec::new();
        for mawdi in 0..adad {
            let lugha = mutaha
                .GetAt(mawdi)
                .map_err(|khata| khata_winrt("reading an installed recognizer", &khata))?;
            let wasm = lugha
                .LanguageTag()
                .map_err(|khata| khata_winrt("reading a recognizer's language tag", &khata))?;
            wusum.push(wasm.to_string());
        }
        Ok(wusum)
    }

    /// Builds an engine for the first requested language that is installed.
    ///
    /// An empty `lughat_matluba` falls through to
    /// `TryCreateFromUserProfileLanguages`, which is what a user who has never
    /// chosen a source language wants: the recognizer for whatever their
    /// Windows is set to.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when no requested language has a
    /// recognizer, and the message names every recognizer that *is* installed —
    /// see this type's header for why that sentence is the whole point of this
    /// engine's error path.
    pub fn jadeed(lughat_matluba: &[&str]) -> Result<Self, KhataTabaqa> {
        hayyi_al_khayt()?;
        let mutaha = Self::lughat_al_nizam()?;

        for matlub in lughat_matluba {
            let Some(mutabiq) = mutaha.iter().find(|wasm| yutabiq_lugha(wasm, matlub)) else {
                continue;
            };
            let lugha = Language::CreateLanguage(&HSTRING::from(mutabiq.as_str()))
                .map_err(|khata| khata_winrt("building a Language object", &khata))?;
            let muharrik = MuharrikNawafidh::TryCreateFromLanguage(&lugha).map_err(|khata| {
                KhataTabaqa::QariGhayrMutah {
                    sabab: format!(
                        "Windows lists {mutabiq} as an installed recognizer but refused to \
                         create an engine for it ({:#010x}): {khata}",
                        khata.code().0
                    ),
                }
            })?;
            return Self::min_muharrik(muharrik, mutabiq.clone(), &mutaha);
        }

        if lughat_matluba.is_empty() {
            let muharrik =
                MuharrikNawafidh::TryCreateFromUserProfileLanguages().map_err(|khata| {
                    KhataTabaqa::QariGhayrMutah {
                        sabab: format!(
                            "no source language was requested and Windows has no OCR \
                             recognizer for any of this profile's languages; the recognizers \
                             installed on this machine are [{}]. Install one under Settings, \
                             Time & language, Language & region, then Optional language \
                             features. ({:#010x}): {khata}",
                            wasf_qaima(&mutaha),
                            khata.code().0
                        ),
                    }
                })?;
            let wasm = muharrik
                .RecognizerLanguage()
                .and_then(|lugha| lugha.LanguageTag())
                .map(|wasm| wasm.to_string())
                .unwrap_or_else(|_| "the user profile's language".to_owned());
            return Self::min_muharrik(muharrik, wasm, &mutaha);
        }

        Err(KhataTabaqa::QariGhayrMutah {
            sabab: format!(
                "the OCR recognizers installed on this machine are [{}]; none of them reads \
                 [{}]. Install the missing language pack under Settings, Time & language, \
                 Language & region, then Optional language features.",
                wasf_qaima(&mutaha),
                lughat_matluba.join(", ")
            ),
        })
    }

    /// Finishes construction once an engine exists.
    fn min_muharrik(
        muharrik: MuharrikNawafidh,
        lugha: String,
        mutaha: &[String],
    ) -> Result<Self, KhataTabaqa> {
        let aqsa_buad = MuharrikNawafidh::MaxImageDimension()
            .map_err(|khata| khata_winrt("reading the maximum image dimension", &khata))?;
        let lughat = mutaha.iter().map(|wasm| ramz_lugha_thabit(wasm)).collect();
        Ok(Self { muharrik, lugha, lughat, aqsa_buad })
    }

    /// The recognizer language this engine was built for.
    #[must_use]
    pub fn lugha(&self) -> &str {
        &self.lugha
    }

    /// The largest image dimension this engine accepts.
    #[must_use]
    pub const fn aqsa_buad(&self) -> u32 {
        self.aqsa_buad
    }

    /// The capture as a BGRA8 `SoftwareBitmap`.
    ///
    /// Two things about this are decisions rather than mechanics.
    ///
    /// **The alpha channel is forced to 255.** `CreateCopyFromBuffer` produces
    /// a BGRA8 bitmap in *premultiplied* alpha mode, and a great many games
    /// leave the backbuffer's alpha at zero because nothing ever reads it — the
    /// swap chain does not. A premultiplied bitmap whose alpha is zero is, by
    /// definition, fully transparent black, and what the recognizer would then
    /// read is a black rectangle. Writing 255 makes premultiplied and straight
    /// alpha identical, so the question of which mode the bitmap is in stops
    /// being able to affect the result.
    ///
    /// **The bytes go through `DataWriter`.** The alternative is
    /// `IBufferByteAccess`, which is a COM interop cast and a raw pointer
    /// write, and it saves one copy of a few hundred kilobytes on a thread that
    /// is not the render thread. One `unsafe` block per capture is not worth a
    /// copy that does not happen inside the frame.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::IltiqatFashil`] when the region is larger than
    /// [`QariWindows::aqsa_buad`], naming both numbers, and
    /// [`KhataTabaqa::QariGhayrMutah`] when a WinRT call refuses.
    fn ila_sura_barmajiya(&self, sura: &SuraMultaqata) -> Result<SoftwareBitmap, KhataTabaqa> {
        if sura.ard() > self.aqsa_buad || sura.irtifa() > self.aqsa_buad {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the region is {}×{} and Windows OCR accepts nothing larger than {} on \
                     either axis",
                    sura.ard(),
                    sura.irtifa(),
                    self.aqsa_buad
                ),
            });
        }
        let (Ok(ard), Ok(irtifa)) =
            (i32::try_from(sura.ard()), i32::try_from(sura.irtifa()))
        else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "the region is {}×{}, which does not fit the signed dimensions WinRT takes",
                    sura.ard(),
                    sura.irtifa()
                ),
            });
        };

        let rgb = sura.ila_rgb()?;
        let sia = rgb.len().checked_div(3).unwrap_or(0).saturating_mul(4);
        let mut bgra = Vec::with_capacity(sia);
        for biksel in rgb.chunks_exact(3) {
            let ahmar = biksel.first().copied().unwrap_or(0);
            let akhdar = biksel.get(1).copied().unwrap_or(0);
            let azraq = biksel.get(2).copied().unwrap_or(0);
            bgra.extend_from_slice(&[azraq, akhdar, ahmar, 255]);
        }

        let katib = DataWriter::new()
            .map_err(|khata| khata_winrt("creating the byte writer", &khata))?;
        katib
            .WriteBytes(&bgra)
            .map_err(|khata| khata_winrt("writing the captured bytes", &khata))?;
        let mukhazzan = katib
            .DetachBuffer()
            .map_err(|khata| khata_winrt("detaching the pixel buffer", &khata))?;

        SoftwareBitmap::CreateCopyFromBuffer(&mukhazzan, BitmapPixelFormat::Bgra8, ard, irtifa)
            .map_err(|khata| khata_winrt("building the bitmap", &khata))
    }
}

#[cfg(windows)]
impl Qari for QariWindows {
    fn ism(&self) -> &'static str {
        "Windows Runtime OCR"
    }

    fn mutah(&self) -> bool {
        // The engine object survives its language pack being uninstalled and
        // then fails at recognition time, so availability is re-derived from
        // the live list rather than from the object's existence.
        Self::lughat_al_nizam()
            .map(|mutaha| mutaha.iter().any(|wasm| yutabiq_lugha(wasm, &self.lugha)))
            .unwrap_or(false)
    }

    fn lughat(&self) -> &[&str] {
        &self.lughat
    }

    fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        hayyi_al_khayt()?;
        let ard = sura.ard();
        let irtifa = sura.irtifa();
        let bitmap = self.ila_sura_barmajiya(sura)?;

        // Blocking on the async operation, on purpose, on a thread that is not
        // the render thread. `RecognizeAsync` completes on a thread pool and
        // the wait below sleeps until it does; calling this from inside
        // `Present` would block the game's render thread on a thread pool item,
        // which is the shape of every overlay deadlock ever reported.
        let amaliya = self
            .muharrik
            .RecognizeAsync(&bitmap)
            .map_err(|khata| khata_winrt("starting recognition", &khata))?;
        let natija = intazir_amaliya(&amaliya, "waiting for recognition")?;

        let sutur_winrt = natija
            .Lines()
            .map_err(|khata| khata_winrt("reading the recognized lines", &khata))?;
        let adad = sutur_winrt
            .Size()
            .map_err(|khata| khata_winrt("counting the recognized lines", &khata))?;

        let mut sutur = Vec::new();
        for mawdi in 0..adad {
            let satr = sutur_winrt
                .GetAt(mawdi)
                .map_err(|khata| khata_winrt("reading a recognized line", &khata))?;
            let nass = satr
                .Text()
                .map_err(|khata| khata_winrt("reading a line's text", &khata))?
                .to_string();
            if nass.trim().is_empty() {
                continue;
            }
            let hudud = hudud_al_satr(&satr, ard, irtifa)?;
            sutur.push(SatrMaqru::jadeed(&nass, hudud, THIQA_GHAYR_MAQISA, false));
        }

        if sutur.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru { mintaqa: wasf_mintaqa(sura), thiqa: None });
        }
        Ok(sutur)
    }
}

/// The union of a line's word rectangles.
///
/// `OcrLine` has no bounding rectangle of its own — only `OcrWord` does — so
/// the line's box is built from its words. A line with no words is possible in
/// principle and is given the whole image, because a box that is too large
/// still places the Arabic over the right region while a box of zero size
/// places it nowhere.
///
/// # Errors
///
/// [`KhataTabaqa::QariGhayrMutah`] when a WinRT call on the word collection
/// refuses.
#[cfg(windows)]
fn hudud_al_satr(
    satr: &windows::Media::Ocr::OcrLine,
    ard: u32,
    irtifa: u32,
) -> Result<MustatilBiksel, KhataTabaqa> {
    let kalimat = satr
        .Words()
        .map_err(|khata| khata_winrt("reading a line's words", &khata))?;
    let adad = kalimat
        .Size()
        .map_err(|khata| khata_winrt("counting a line's words", &khata))?;

    let mut hudud: Option<(f32, f32, f32, f32)> = None;
    for mawdi in 0..adad {
        let kalima = kalimat
            .GetAt(mawdi)
            .map_err(|khata| khata_winrt("reading a word", &khata))?;
        let mustatil = kalima
            .BoundingRect()
            .map_err(|khata| khata_winrt("reading a word's rectangle", &khata))?;
        let yameen = mustatil.X + mustatil.Width;
        let asfal = mustatil.Y + mustatil.Height;
        hudud = Some(match hudud {
            Some((yasar_h, aala_h, yameen_h, asfal_h)) => (
                yasar_h.min(mustatil.X),
                aala_h.min(mustatil.Y),
                yameen_h.max(yameen),
                asfal_h.max(asfal),
            ),
            None => (mustatil.X, mustatil.Y, yameen, asfal),
        });
    }

    Ok(match hudud {
        Some((yasar, aala, yameen, asfal)) => {
            mustatil_min_hudud(yasar, aala, yameen, asfal, ard, irtifa)
        }
        None => MustatilBiksel { yasar: 0, aala: 0, ard, irtifa },
    })
}

/// A list of tags as it appears inside an error message.
///
/// "none" rather than an empty pair of brackets, because a message reading
/// "the recognizers installed on this machine are []" reads as a bug in the
/// message rather than as an answer.
#[cfg(windows)]
fn wasf_qaima(wusum: &[String]) -> String {
    if wusum.is_empty() { "none".to_owned() } else { wusum.join(", ") }
}

// ---------------------------------------------------------------------------
// macOS Vision
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
use objc2::encode::{Encode, Encoding};
#[cfg(target_os = "macos")]
use objc2::rc::{Allocated, Retained};
#[cfg(target_os = "macos")]
use objc2::runtime::AnyObject;
#[cfg(target_os = "macos")]
use objc2::{ClassType, msg_send, sel};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSData, NSDictionary, NSError, NSString};
#[cfg(target_os = "macos")]
use objc2_vision::{VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation};

/// `VNRequestTextRecognitionLevelAccurate`.
///
/// Written as its raw `NSInteger` value because that is what crosses the
/// message send, and accurate rather than fast is not a tuning choice here:
/// the fast path is a per-character classifier meant for real-time barcode-like
/// work, and on 14-pixel game subtitles it reads roughly one word in four
/// wrong. The accurate path is a sequence model and is what makes Vision worth
/// preferring over the portable engine at all.
#[cfg(target_os = "macos")]
const MUSTAWA_DAQEEQ: isize = 0;

/// `CGRect`, declared here rather than imported.
///
/// `CGRect` has moved between `objc2-foundation` and `objc2-core-foundation`
/// across the objc2 line, and this crate needs exactly one method's return
/// type from it. Declaring the layout locally is four lines and does not put a
/// compile error between this crate and a dependency bump for a type whose
/// definition has not changed since 2001.
///
/// The fields are `f64` because `CGFloat` is `double` on every 64-bit Apple
/// target, and Taarib ships no 32-bit macOS or iOS build. On a 32-bit Apple
/// target this would be wrong, which is why the encoding below names `Double`
/// explicitly — a mismatch is caught by `objc2`'s verification rather than
/// read as garbage.
#[cfg(target_os = "macos")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MustatilCG {
    /// Left edge, normalized.
    s: f64,
    /// Bottom edge, normalized — Vision's origin is bottom-left.
    ain: f64,
    /// Width, normalized.
    ard: f64,
    /// Height, normalized.
    irtifa: f64,
}

// SAFETY: the layout declared here is exactly `CGRect`'s: a `CGPoint` of two
// `CGFloat`s followed by a `CGSize` of two `CGFloat`s, with `CGFloat` being
// `double` on all 64-bit Apple targets. `#[repr(C)]` on four consecutive `f64`
// fields produces the identical layout — four eight-byte fields at offsets 0,
// 8, 16 and 24 — because C struct nesting adds no padding when every member is
// eight-byte aligned. The encoding string names the nesting so `objc2` selects
// the correct message-send ABI on x86_64, where a 32-byte struct return is
// passed through a hidden pointer rather than in registers.
#[cfg(target_os = "macos")]
unsafe impl Encode for MustatilCG {
    const ENCODING: Encoding = Encoding::Struct(
        "CGRect",
        &[
            Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]),
            Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]),
        ],
    );
}

/// A `CGFloat` product as an `f32` pixel coordinate.
#[cfg(target_os = "macos")]
fn min_ashari(qeema: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the value is a pixel coordinate inside a captured region, which is far inside \
                  f32's exact-integer range; only sub-pixel precision is lost and the result is \
                  rounded to a whole pixel immediately afterwards"
    )]
    {
        qeema as f32
    }
}

/// macOS Vision, through `VNRecognizeTextRequest`.
///
/// The best of the three on the platform that has it, and the only one that
/// reports a real per-line confidence — which is why [`SatrMaqru::maqisa`] is
/// `true` here and `false` everywhere else. A pipeline running on macOS can
/// discard a bad read before translating it; the same pipeline on Windows
/// cannot, and that is a genuine difference in product quality between the two
/// platforms that this crate declines to paper over.
///
/// The request object is built per call rather than held. `VNRecognizeTextRequest`
/// is not documented as safe to reuse across concurrent handlers, holding one
/// would make this type's [`Send`] claim rest on an undocumented property of
/// somebody else's framework, and constructing one is a `+new` — the cost is
/// nothing next to the recognition it configures.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub struct QariVision {
    matlub: Vec<String>,
    lughat: Vec<&'static str>,
}

#[cfg(target_os = "macos")]
impl QariVision {
    /// Builds a recognizer for the given source languages.
    ///
    /// An empty list leaves Vision on its own default, which is the user's
    /// preferred languages.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when a probe request cannot be created
    /// at all, which means the Vision framework is not present — a stripped
    /// system, or a macOS older than the text recognizer.
    pub fn jadeed(lughat_matluba: &[&str]) -> Result<Self, KhataTabaqa> {
        let matlub: Vec<String> = lughat_matluba.iter().map(|wasm| (*wasm).to_owned()).collect();
        let talab = Self::ibni_talab(&matlub)?;
        let madumah = Self::lughat_al_talab(&talab);
        let asas: Vec<String> = if madumah.is_empty() { matlub.clone() } else { madumah };
        let lughat = asas.iter().map(|wasm| ramz_lugha_thabit(wasm)).collect();
        Ok(Self { matlub, lughat })
    }

    /// A configured `VNRecognizeTextRequest`.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when the class cannot be instantiated.
    fn ibni_talab(matlub: &[String]) -> Result<Retained<VNRecognizeTextRequest>, KhataTabaqa> {
        // SAFETY: `+new` on any Objective-C class that inherits from NSObject
        // is `+alloc` followed by `-init` and returns a `+1` reference, which is
        // exactly what `Retained` takes ownership of. `VNRecognizeTextRequest`
        // inherits from `VNRequest`, which inherits from `NSObject`, and
        // declares no designated initializer that forbids `-init`.
        let talab: Option<Retained<VNRecognizeTextRequest>> =
            unsafe { msg_send![VNRecognizeTextRequest::class(), new] };
        let Some(talab) = talab else {
            return Err(KhataTabaqa::QariGhayrMutah {
                sabab: "the Vision framework's text recognizer could not be created on this \
                        system"
                    .to_owned(),
            });
        };

        // SAFETY: `-setRecognitionLevel:` takes one `NSInteger` by value and
        // returns nothing. `MUSTAWA_DAQEEQ` is the documented raw value of
        // `VNRequestTextRecognitionLevelAccurate` and `isize` is `NSInteger`'s
        // Rust equivalent on every 64-bit Apple target.
        unsafe {
            let _: () = msg_send![&*talab, setRecognitionLevel: MUSTAWA_DAQEEQ];
        }
        // SAFETY: `-setUsesLanguageCorrection:` takes one `BOOL` by value and
        // returns nothing; `objc2` bridges a Rust `bool` to `BOOL` for this
        // argument position. Language correction is on because game dialogue is
        // prose, and the correction model resolves exactly the confusions —
        // `rn` for `m`, `l` for `I` — that a stylised game font produces most.
        unsafe {
            let _: () = msg_send![&*talab, setUsesLanguageCorrection: true];
        }

        if !matlub.is_empty() {
            let wusum: Vec<Retained<NSString>> =
                matlub.iter().map(|wasm| NSString::from_str(wasm)).collect();
            let masfufa: Retained<NSArray<NSString>> = NSArray::from_retained_slice(&wusum);
            // SAFETY: `-setRecognitionLanguages:` takes one object pointer —
            // an `NSArray<NSString *> *` — and returns nothing. The array is
            // owned by `masfufa` for the whole of this call, and the request
            // retains it if it keeps it.
            unsafe {
                let _: () = msg_send![&*talab, setRecognitionLanguages: &*masfufa];
            }
        }

        Ok(talab)
    }

    /// The languages a request says it supports, or an empty list.
    ///
    /// Guarded by `-respondsToSelector:` rather than called directly, and the
    /// reason is the one that governs everything in this crate: this code runs
    /// inside somebody's game. The selector that reports supported languages
    /// was `+supportedRecognitionLanguagesForTextRecognitionLevel:revision:error:`
    /// before macOS 12 and `-supportedRecognitionLanguagesAndReturnError:`
    /// after it, and sending a selector an object does not implement raises an
    /// Objective-C exception, which unwinds through Rust frames and terminates
    /// the process. Asking first costs one message send; not asking costs the
    /// player their session on the one macOS version this build was not tested
    /// against.
    fn lughat_al_talab(talab: &VNRecognizeTextRequest) -> Vec<String> {
        let ramz = sel!(supportedRecognitionLanguagesAndReturnError:);
        // SAFETY: `-respondsToSelector:` is declared on `NSObject` and is
        // implemented by every Objective-C object that reaches this crate. It
        // takes a selector by value and returns `BOOL`.
        let yastajib: bool = unsafe { msg_send![talab, respondsToSelector: ramz] };
        if !yastajib {
            return Vec::new();
        }

        // SAFETY: the selector was just confirmed to exist on this object. It
        // follows the `...AndReturnError:` convention, so `objc2`'s `error: _`
        // form is the correct spelling: it passes an out-pointer to an
        // `NSError *` and turns a nil return into `Err`. The returned array is
        // autoreleased and `Retained` retains it.
        let natija: Result<Retained<NSArray<NSString>>, Retained<NSError>> =
            unsafe { msg_send![talab, supportedRecognitionLanguagesAndReturnError: _] };
        let Ok(masfufa) = natija else {
            return Vec::new();
        };

        // SAFETY: `-count` is `NSArray`'s own method, takes nothing and returns
        // an `NSUInteger`.
        let adad: usize = unsafe { msg_send![&*masfufa, count] };
        let mut wusum = Vec::with_capacity(adad);
        for mawdi in 0..adad {
            // SAFETY: `-objectAtIndex:` is `NSArray`'s own method and the index
            // is strictly below the count read immediately above, so it cannot
            // raise a range exception. Every element of this array is an
            // `NSString` by the method's declared type.
            let wasm: Retained<NSString> =
                unsafe { msg_send![&*masfufa, objectAtIndex: mawdi] };
            wusum.push(wasm.to_string());
        }
        wusum
    }
}

#[cfg(target_os = "macos")]
impl Qari for QariVision {
    fn ism(&self) -> &'static str {
        "macOS Vision"
    }

    fn mutah(&self) -> bool {
        Self::ibni_talab(&self.matlub).is_ok()
    }

    fn lughat(&self) -> &[&str] {
        &self.lughat
    }

    fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        let ard = sura.ard();
        let irtifa = sura.irtifa();
        let talab = Self::ibni_talab(&self.matlub)?;

        // PNG rather than a CGImage. Building a CGImage means a CGColorSpace, a
        // CGDataProvider and a CGBitmapInfo whose byte-order flags are the most
        // commonly mis-set constants in the framework; `-initWithData:` takes
        // any format ImageIO reads and there is one way to get it wrong instead
        // of five. The encode costs a few milliseconds on a region-sized image,
        // off the render thread, and buys a code path with no colour-space
        // question in it at all.
        let png = sura_png(sura)?;
        let bayanat = NSData::with_bytes(&png);
        let khiyarat: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::new();

        // SAFETY: `+alloc` returns an uninitialised `+1` instance, which
        // `Allocated` owns and which the `-initWithData:options:` immediately
        // below consumes exactly once. `-initWithData:options:` is
        // `VNImageRequestHandler`'s documented designated initializer; it takes
        // two object pointers, both of which are owned by live `Retained`
        // values across the call, and returns a `+1` reference.
        let muallij: Retained<VNImageRequestHandler> = unsafe {
            let mukhassas: Allocated<VNImageRequestHandler> =
                msg_send![VNImageRequestHandler::class(), alloc];
            msg_send![mukhassas, initWithData: &*bayanat, options: &*khiyarat]
        };

        let talabat: Retained<NSArray<VNRecognizeTextRequest>> =
            NSArray::from_retained_slice(&[talab.clone()]);
        // SAFETY: `-performRequests:error:` takes an array of requests and an
        // out-pointer to an `NSError *`, which is what `objc2`'s `error: _`
        // form supplies. It runs the request synchronously on this thread; the
        // array and every request in it are owned by live `Retained` values for
        // the whole call.
        let natija: Result<(), Retained<NSError>> =
            unsafe { msg_send![&*muallij, performRequests: &*talabat, error: _] };
        if let Err(khata) = natija {
            return Err(KhataTabaqa::QariGhayrMutah {
                sabab: format!("Vision refused a {ard}×{irtifa} region: {khata}"),
            });
        }

        // SAFETY: `-results` is declared on `VNRequest` and returns a nullable
        // autoreleased array, which is what `Option<Retained<_>>` models. It is
        // read after `-performRequests:error:` returned successfully, which is
        // the only point at which it is documented to be populated.
        let mulahazat: Option<Retained<NSArray<VNRecognizedTextObservation>>> =
            unsafe { msg_send![&*talab, results] };
        let Some(mulahazat) = mulahazat else {
            return Err(KhataTabaqa::LaNassMaqru { mintaqa: wasf_mintaqa(sura), thiqa: None });
        };

        // SAFETY: `NSArray`'s own `-count`, and every `-objectAtIndex:` below
        // is bounded by it.
        let adad: usize = unsafe { msg_send![&*mulahazat, count] };
        let mut sutur = Vec::with_capacity(adad);
        let mut adna_thiqa: Option<u8> = None;

        for mawdi in 0..adad {
            // SAFETY: the index is below the count read above.
            let mulahaza: Retained<VNRecognizedTextObservation> =
                unsafe { msg_send![&*mulahazat, objectAtIndex: mawdi] };

            // SAFETY: `-topCandidates:` is declared on
            // `VNRecognizedTextObservation`, takes an `NSUInteger` by value and
            // returns an autoreleased array that is empty rather than nil when
            // nothing was read — modelled as `Option` anyway, because a nil
            // return would otherwise be a null `Retained`.
            let mursahhat: Option<Retained<NSArray<AnyObject>>> =
                unsafe { msg_send![&*mulahaza, topCandidates: 1_usize] };
            let Some(mursahhat) = mursahhat else {
                continue;
            };
            // SAFETY: `NSArray`'s own `-count`.
            let adad_mursahhat: usize = unsafe { msg_send![&*mursahhat, count] };
            if adad_mursahhat == 0 {
                continue;
            }
            // SAFETY: index zero, and the count above is at least one.
            let murashshah: Retained<AnyObject> =
                unsafe { msg_send![&*mursahhat, objectAtIndex: 0_usize] };

            // SAFETY: `-string` is `VNRecognizedText`'s own property and
            // returns a non-null `NSString *` for any candidate the framework
            // produced — `-topCandidates:` does not emit a candidate without
            // one.
            let nass: Retained<NSString> = unsafe { msg_send![&*murashshah, string] };
            // SAFETY: `-confidence` is `VNRecognizedText`'s own property. It
            // takes nothing and returns a `VNConfidence`, which is a `float`,
            // by value.
            let thiqa_khaam: f32 = unsafe { msg_send![&*murashshah, confidence] };

            // SAFETY: `-boundingBox` is declared on `VNDetectedObject`, which
            // `VNRecognizedTextObservation` inherits from, and returns a
            // `CGRect` by value. `MustatilCG` declares that exact layout and
            // encoding above.
            let sunduq: MustatilCG = unsafe { msg_send![&*mulahaza, boundingBox] };

            let nass = nass.to_string();
            if nass.trim().is_empty() {
                continue;
            }
            let thiqa = thiqa_min_nisba(thiqa_khaam);
            adna_thiqa = Some(adna_thiqa.map_or(thiqa, |sabiqa| sabiqa.min(thiqa)));
            sutur.push(SatrMaqru::jadeed(&nass, min_sunduq(sunduq, ard, irtifa), thiqa, true));
        }

        if sutur.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: wasf_mintaqa(sura),
                thiqa: adna_thiqa,
            });
        }
        Ok(sutur)
    }
}

/// A Vision bounding box as pixels in the captured image.
///
/// Two conversions in one, and both are easy to get silently wrong.
///
/// **Normalized to pixels**: Vision reports every box in the unit square
/// regardless of the image's size, so every coordinate is multiplied by the
/// dimension it belongs to.
///
/// **Bottom-left to top-left**: Vision's origin is the *bottom* left, and every
/// other coordinate system in this crate — the surface, the capture, the
/// overlay's own draw — has its origin at the top left. Skipping the flip
/// produces boxes that are correct in x, correct in size, and mirrored in y,
/// which puts the Arabic for the first line of dialogue over the last one. It
/// looks like a layout bug rather than a coordinate bug, which is why it is
/// worth a sentence here.
#[cfg(target_os = "macos")]
fn min_sunduq(sunduq: MustatilCG, ard: u32, irtifa: u32) -> MustatilBiksel {
    let ard_ashari = f64::from(ard);
    let irtifa_ashari = f64::from(irtifa);
    let yasar = sunduq.s * ard_ashari;
    let yameen = (sunduq.s + sunduq.ard) * ard_ashari;
    let aala = (1.0 - (sunduq.ain + sunduq.irtifa)) * irtifa_ashari;
    let asfal = (1.0 - sunduq.ain) * irtifa_ashari;
    mustatil_min_hudud(
        min_ashari(yasar),
        min_ashari(aala),
        min_ashari(yameen),
        min_ashari(asfal),
        ard,
        irtifa,
    )
}

/// The capture as a PNG.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the encoder refuses, which for a
/// correctly sized RGB buffer it does not — the check exists because the size
/// is computed rather than asserted, and an encoder error here would otherwise
/// surface as a Vision failure with no explanation.
#[cfg(target_os = "macos")]
fn sura_png(sura: &SuraMultaqata) -> Result<Vec<u8>, KhataTabaqa> {
    use image::ImageEncoder;

    let rgb = sura.ila_rgb()?;
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgb, sura.ard(), sura.irtifa(), image::ExtendedColorType::Rgb8)
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!(
                "a {}×{} region could not be encoded for Vision: {khata}",
                sura.ard(),
                sura.irtifa()
            ),
        })?;
    Ok(png)
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// Tries the platform recognizer, recording what happened either way.
///
/// Three definitions, one per platform family, rather than one function full of
/// `cfg!`. The bodies share nothing — the Windows one enumerates language packs
/// and the macOS one probes a framework — and a merged version would be a
/// function where two thirds of the code is compiled out of every build.
#[cfg(windows)]
fn jarrib_al_manassa(wusum: &[&str], athar: &mut Vec<String>) -> Option<Box<dyn Qari>> {
    match QariWindows::jadeed(wusum) {
        Ok(qari) => {
            athar.push(format!(
                "Windows Runtime OCR: chosen. Its {} recognizer is installed, it is already on \
                 this machine, and it is several times faster per region than the portable \
                 engine.",
                qari.lugha()
            ));
            Some(Box::new(qari))
        }
        Err(khata) => {
            athar.push(format!("Windows Runtime OCR: not usable. {khata}"));
            None
        }
    }
}

/// Tries the platform recognizer, recording what happened either way.
#[cfg(target_os = "macos")]
fn jarrib_al_manassa(wusum: &[&str], athar: &mut Vec<String>) -> Option<Box<dyn Qari>> {
    match QariVision::jadeed(wusum) {
        Ok(qari) => {
            athar.push(format!(
                "macOS Vision: chosen. It is part of the system, it reads [{}], and it is the \
                 only engine here that reports a real confidence per line.",
                qari.lughat().join(", ")
            ));
            Some(Box::new(qari))
        }
        Err(khata) => {
            athar.push(format!("macOS Vision: not usable. {khata}"));
            None
        }
    }
}

/// Tries the platform recognizer, recording what happened either way.
#[cfg(not(any(windows, target_os = "macos")))]
fn jarrib_al_manassa(_wusum: &[&str], athar: &mut Vec<String>) -> Option<Box<dyn Qari>> {
    athar.push(
        "platform recognizer: none. This operating system ships no system-wide text \
         recognition service, so the portable engine is not a fallback here — it is the only \
         engine."
            .to_owned(),
    );
    None
}

/// Whether an engine covers at least one of the requested source languages.
///
/// An empty request is satisfied by anything, because it means "the platform
/// default" and the engine has already applied its own.
fn yaqra_al_matlub(qari: &dyn Qari, wusum: &[&str]) -> bool {
    wusum.is_empty()
        || wusum.iter().any(|matlub| {
            qari.lughat().iter().any(|mutah| yutabiq_lugha(mutah, matlub))
        })
}

/// Which recognizer was chosen, and the reasoning that got there.
///
/// The trail is not logging. It is what the control panel shows when a user
/// asks why their text is not being read, and it is written so that every line
/// is an answer rather than a status: "Windows Runtime OCR: not usable. The OCR
/// recognizers installed on this machine are [fr-FR]; none of them reads
/// [en-US]" is a sentence a user can act on. "OCR unavailable" is not.
///
/// The order is platform first, portable second, and the preference is not
/// close on the two platforms that have a native engine: it is already
/// installed, it is maintained by the vendor, it is faster, and it adds nothing
/// to the download. The portable engine wins only when asked for, or when the
/// native one is genuinely unavailable — which, on Windows, most often means a
/// missing language pack rather than a missing feature.
#[derive(Debug)]
pub struct IkhtiyarQari {
    qari: Box<dyn Qari>,
    athar: Vec<String>,
}

impl IkhtiyarQari {
    /// Picks a recognizer.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::QariGhayrMutah`] when nothing is available, carrying the
    /// **whole trail** rather than the last failure. A user whose Windows OCR
    /// pack is missing *and* whose model directory is empty has two problems,
    /// and reporting only the second sends them to fix the wrong one.
    pub fn ikhtar(iadadat: &IdadatQira) -> Result<Self, KhataTabaqa> {
        let mut athar = Vec::new();
        let wusum = iadadat.wusum();
        let wasf_lughat =
            if wusum.is_empty() { "the platform default".to_owned() } else { wusum.join(", ") };
        athar.push(format!("source languages requested: [{wasf_lughat}]"));

        if iadadat.yufaddil_al_mahmul {
            athar.push(
                "platform recognizer: skipped. The user asked for the portable engine, which \
                 is the right choice when the system recognizer only has a pack for a language \
                 the game is not in."
                    .to_owned(),
            );
        } else if let Some(qari) = jarrib_al_manassa(&wusum, &mut athar) {
            if qari.mutah() {
                return Ok(Self { qari, athar });
            }
            athar.push(format!(
                "{}: discarded immediately after construction — it reported itself unavailable, \
                 which happens when a language pack is removed between the enumeration and the \
                 engine being built.",
                qari.ism()
            ));
        }

        match QariMahmul::jadeed(&iadadat.mujallad_namadhij) {
            Ok(qari) => {
                athar.push(format!(
                    "portable ocrs: chosen. Models loaded from {} and {}; it reads the Latin \
                     alphabet and reports no confidence.",
                    qari.masar_kashf().display(),
                    qari.masar_taaruf().display()
                ));
                if !yaqra_al_matlub(&qari, &wusum) {
                    // Not a refusal. The engine will run and will return
                    // something, and what it returns for a script it was never
                    // trained on is confident nonsense — which is the worst
                    // possible output, so the warning is worth a line even
                    // though nothing here can act on it.
                    athar.push(format!(
                        "warning: the portable models were trained on [{}] and none of them \
                         covers [{wasf_lughat}]. Recognition will run and its output should \
                         not be trusted.",
                        qari.lughat().join(", ")
                    ));
                }
                Ok(Self { qari: Box::new(qari), athar })
            }
            Err(khata) => {
                athar.push(format!("portable ocrs: not usable. {khata}"));
                Err(KhataTabaqa::QariGhayrMutah { sabab: athar.join(" | ") })
            }
        }
    }

    /// Wraps a recognizer that was chosen elsewhere.
    ///
    /// For the test harness and for a control panel that lets a user pick an
    /// engine by hand. The trail is supplied by the caller, because a choice
    /// this constructor did not make is a choice it cannot explain.
    #[must_use]
    pub fn min_qari(qari: Box<dyn Qari>, athar: Vec<String>) -> Self {
        Self { qari, athar }
    }

    /// The chosen engine's name.
    #[must_use]
    pub fn ism(&self) -> &'static str {
        self.qari.ism()
    }

    /// The languages the chosen engine reads.
    #[must_use]
    pub fn lughat(&self) -> &[&str] {
        self.qari.lughat()
    }

    /// Whether the chosen engine is still usable.
    #[must_use]
    pub fn mutah(&self) -> bool {
        self.qari.mutah()
    }

    /// The reasoning, one sentence per engine that was tried.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// The chosen engine.
    #[must_use]
    pub fn qari(&self) -> &dyn Qari {
        self.qari.as_ref()
    }

    /// The chosen engine, mutably.
    pub fn qari_mut(&mut self) -> &mut dyn Qari {
        self.qari.as_mut()
    }

    /// Reads a region with the chosen engine.
    ///
    /// # Errors
    ///
    /// As [`Qari::iqra`].
    pub fn iqra(&mut self, sura: &SuraMultaqata) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        self.qari.iqra(sura)
    }

    /// Reads a region and rejoins the fragments the engine split it into.
    ///
    /// The call the pipeline should make. `aqsa_fajwa` of [`None`] derives the
    /// threshold from the lines themselves through [`fajwa_muqtaraha`], which
    /// is right in almost every case: the correct gap scales with the text
    /// size, and the text size is a thing the returned boxes know and the
    /// caller does not.
    ///
    /// # Errors
    ///
    /// As [`Qari::iqra`], and [`KhataTabaqa::LaNassMaqru`] when merging leaves
    /// nothing — which happens when every line the engine returned was
    /// whitespace.
    pub fn iqra_mudmaj(
        &mut self,
        sura: &SuraMultaqata,
        aqsa_fajwa: Option<u32>,
    ) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        let sutur = self.qari.iqra(sura)?;
        let fajwa = aqsa_fajwa.unwrap_or_else(|| fajwa_muqtaraha(&sutur));
        let mudmaja = SatrMaqru::mudmaj(sutur, fajwa);
        if mudmaja.is_empty() {
            return Err(KhataTabaqa::LaNassMaqru {
                mintaqa: wasf_mintaqa(sura),
                thiqa: None,
            });
        }
        Ok(mudmaja)
    }

    /// Preprocesses, reads, merges, and puts the boxes back in the capture's
    /// coordinates.
    ///
    /// The half of [`IkhtiyarQari::iqra_mufattasha`] that does not judge, split
    /// out because it runs twice per region and the two calls must not differ.
    /// A region the engine found nothing in comes back as an empty vector rather
    /// than an error: "nothing was there" is a *result* here, and it is the one
    /// the gate below turns into a refusal.
    fn iqra_muhassana(
        &mut self,
        muhassana: &SuraMuhassana,
    ) -> Result<Vec<SatrMaqru>, KhataTabaqa> {
        let sutur = match self.qari.iqra(muhassana.sura()) {
            Ok(sutur) => sutur,
            Err(KhataTabaqa::LaNassMaqru { .. }) => return Ok(Vec::new()),
            Err(khata) => return Err(khata),
        };
        let fajwa = fajwa_muqtaraha(&sutur);
        Ok(SatrMaqru::mudmaj(sutur, fajwa)
            .into_iter()
            .map(|mut satr| {
                satr.mawdi = muhassana.ila_iltiqat(satr.mawdi);
                satr
            })
            .collect())
    }

    /// Reads one region and decides whether what came back may be translated.
    ///
    /// The whole filter, in one call, and the reason this exists is that until
    /// it did the refusal layer was a set of exposed decisions rather than
    /// something that ran: [`hukm_bunya`] and [`hukm_tawafuq`] were public, and
    /// no caller in the workspace joined preprocessing to recognition to them.
    /// Wiring it here rather than in each backend also puts the two-pass
    /// cascade, the coordinate mapping and the choice of corroborating path in
    /// one place, and all three are things that compile just as well when they
    /// are wrong.
    ///
    /// ## The cascade
    ///
    /// 1. Preprocess with `iadadat` and read. The boxes come back in the
    ///    capture's coordinates, the upscale factor already divided out.
    /// 2. [`hukm_bunya`] — one pass over the string, no extra recognition. If it
    ///    refuses, return; over a measured corpus of 768 regions carrying no
    ///    readable text this ends it for 739 of them.
    /// 3. Otherwise preprocess again with
    ///    [`crate::iltiqat_shasha::IdadatTahsin::yuhawwil_ila_ramadi`]
    ///    **flipped**, read again, and put the two reads to [`hukm_tawafuq`].
    ///
    /// Flipped rather than fixed to grayscale, because a caller who has already
    /// chosen the grayscale chain for their game — a visual novel with flat
    /// dialogue panels, where it wins — must still be corroborated against
    /// something that is not itself, and for them that something is colour.
    ///
    /// The second pass is where the cost is, and it is paid on about a tenth of
    /// regions: it runs only on the ones step 2 accepted, and never on a region
    /// already refused.
    ///
    /// ## What it is worth, measured
    ///
    /// Over 768 real region crops that carry no readable text — sky, walls,
    /// grass, crowd, motion blur, a dimmed pause backdrop, shipped textures —
    /// step 2 alone refused 96.2% and let 29 reads through as text. Adding step
    /// 3 refused all 768 and let none through. Across the 20 regions in that
    /// corpus whose text the engine read exactly right, both configurations
    /// accepted every one.
    ///
    /// Nothing was refused for the wrong reason and nothing was tuned to get
    /// there: the thresholds on [`HududQubul`] are untouched. Zero out of 768 is
    /// a ceiling on that corpus rather than a proof — it bounds the rate below
    /// roughly one region in 250 — and it says nothing about a region type the
    /// corpus does not contain.
    ///
    /// # Errors
    ///
    /// Whatever [`crate::iltiqat_shasha::MuhassinSura::hassin_lil_qari`] and
    /// [`Qari::iqra`] refuse. A region with nothing readable in it is **not** an
    /// error here — it is [`HukmQira::Marfud`], which is the answer the caller
    /// asked for. A failure on the corroborating path is propagated rather than
    /// quietly downgraded to the one-pass verdict: a caller that believes both
    /// gates ran when only one did is exactly the caller this call exists to
    /// prevent.
    pub fn iqra_mufattasha(
        &mut self,
        sura: &SuraMultaqata,
        iadadat: &IdadatTahsin,
        hudud: &HududQubul,
    ) -> Result<QiraMufattasha, KhataTabaqa> {
        let mut muhassin = MuhassinSura::jadeed(*iadadat);
        let muhassana = muhassin.hassin_lil_qari(sura)?;
        let sutur = self.iqra_muhassana(&muhassana)?;

        let hukm = hukm_bunya(&sutur, hudud);
        if !hukm.maqbul() {
            return Ok(QiraMufattasha { sutur, hukm, tawafuq_jara: false });
        }

        let mut iadadat_thani = *iadadat;
        iadadat_thani.yuhawwil_ila_ramadi = !iadadat.yuhawwil_ila_ramadi;
        let mut muhassin_thani = MuhassinSura::jadeed(iadadat_thani);
        let muhassana_thani = muhassin_thani.hassin_lil_qari(sura)?;
        let thani = self.iqra_muhassana(&muhassana_thani)?;

        let hukm = hukm_tawafuq(&sutur, &thani, hudud);
        Ok(QiraMufattasha { sutur, hukm, tawafuq_jara: true })
    }
}

/// One region, read and judged.
///
/// Carries the lines even when they were refused, because the control panel
/// shows a user what their region was read as next to why it was not used, and
/// "this region was read as `AW 7 PR WA` and refused" is the sentence that tells
/// them to move the region. A type that dropped the text on refusal would leave
/// the panel with nothing to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QiraMufattasha {
    /// What the engine read, merged, in the **capture's** coordinates.
    pub sutur: Vec<SatrMaqru>,
    /// Whether it may be translated and drawn, and why not when it may not.
    pub hukm: HukmQira,
    /// Whether the corroborating second pass ran.
    ///
    /// `false` means the structural gate refused first and the expensive pass
    /// was skipped — not that corroboration passed. Reported because a caller
    /// counting recognition passes, or asking why one region cost twice what its
    /// neighbour did, has no other way to know.
    pub tawafuq_jara: bool,
}

impl QiraMufattasha {
    /// The lines as one block of text, or [`None`] when the read was refused.
    ///
    /// The accessor the translation pipeline should use, because it cannot
    /// return the text without the verdict having been consulted.
    #[must_use]
    pub fn nass_maqbul(&self) -> Option<String> {
        self.hukm.maqbul().then(|| SatrMaqru::fiqra(&self.sutur))
    }
}

/// A merge gap derived from the lines' own heights.
///
/// One and a half times the median box height. The median rather than the mean
/// because a single tall box — a line the engine merged with a UI icon — would
/// pull a mean up far enough to join two genuinely separate columns of text,
/// and the whole risk of merging is joining things that were not one sentence.
///
/// One and a half rather than one: an inter-word space in proportional text is
/// roughly a quarter of the line height, a wide typographic gap between a
/// speaker's name and their dialogue is roughly one, and the gap between two
/// separate UI elements is several. One and a half sits in the empty part of
/// that distribution.
///
/// Falls back to twenty pixels for an empty set, which is the width of a space
/// in text at the smallest size this crate bothers to recognize.
#[must_use]
pub fn fajwa_muqtaraha(sutur: &[SatrMaqru]) -> u32 {
    let mut irtifaat: Vec<u32> = sutur
        .iter()
        .map(|satr| satr.mawdi.irtifa)
        .filter(|irtifa| *irtifa > 0)
        .collect();
    if irtifaat.is_empty() {
        return 20;
    }
    irtifaat.sort_unstable();
    let mawdi = irtifaat.len().checked_div(2).unwrap_or(0);
    let wasit = irtifaat.get(mawdi).copied().unwrap_or(20);
    wasit.saturating_mul(3).checked_div(2).unwrap_or(20).max(4)
}

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// A count as `f64`, with the precision loss stated once.
///
/// Every value that reaches this is a character count or a token count over one
/// region's worth of text, which is orders of magnitude below 2^53.
const fn adad_f64(qeema: usize) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "character and token counts over one region are far below 2^53, where the \
                  conversion is exact"
    )]
    {
        qeema as f64
    }
}

/// The punctuation that appears in game text.
///
/// Not "safe ASCII". A list of what a dialogue box, a menu label and a HUD
/// readout genuinely contain, so that everything outside it counts as a
/// character the recognizer invented.
const RUMUZ_MAQBULA: &str = ".,:;'\"-()[]{}/\\%&+*=<>#@$_|~`^…—–°′″×÷";

/// The only single-letter words English has.
///
/// Two of them, which is what makes a line full of stray single letters a
/// measurable signal rather than a guess. A key name — the `F` in "Press F" —
/// is a third case, and it is why this is counted rather than vetoed.
const KALIMAT_HARF: [char; 4] = ['a', 'A', 'i', 'I'];

/// How far a read may be from looking like text before it is refused.
///
/// Every number here was set against a measured corpus rather than chosen —
/// see the harness in the phase-29 measurement notes. They are exposed because
/// the right value differs between a game whose HUD is mostly numbers and one
/// whose dialogue is prose, and because a threshold nobody can move is a
/// threshold that will be wrong for somebody.
///
/// ## They were swept again against 768 blank regions, and left alone
///
/// The four values below decide [`hukm_bunya`] between them, so a sweep over
/// them replays the real verdict rather than a model of it. Over the corpus in
/// [`IkhtiyarQari::iqra_mufattasha`]'s header, raising [`Self::adna_huruf`] from
/// 2 to 4 would have cut what got past the structural gate from 29 regions to
/// 13 — 3.8% to 1.7% — and lowering [`Self::nisbat_tashawwuh`] from 0.34 to
/// 0.25 would have cut it to 11, in both cases without refusing a single one of
/// the 20 regions the engine read exactly right.
///
/// Neither change was made, and the reason is a hole in the corpus rather than
/// a preference. The shortest correct read in it is four characters, so it
/// contains no evidence at all about `OK`, `XI`, `HP`, `No`, `On` or `Map` —
/// the two- and three-character words a game HUD is full of. A sweep that looks
/// free only because the corpus cannot see the cost is not a measurement of the
/// threshold, it is a measurement of the corpus. The gain those two changes
/// offer is in any case already taken, and taken without a threshold move: the
/// corroborating pass refuses all 29.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HududQubul {
    /// The largest fraction of characters that may be ones this crate does not
    /// believe are text.
    pub nisbat_ramz: f64,
    /// The largest fraction of whitespace-separated tokens that may be
    /// malformed in the sense of [`kalima_mushawwaha`].
    pub nisbat_tashawwuh: f64,
    /// The fewest characters a read may carry and still be worth translating.
    ///
    /// One character is never a sentence and is very often a detector firing on
    /// a UI decoration. Two is the floor rather than one because `OK` and `XI`
    /// are real.
    pub adna_huruf: usize,
    /// How many glyphs the recognizer may report it could not identify.
    ///
    /// Zero. The marker is not noise and it is not a threshold judgement: it is
    /// the engine stating that it failed on that glyph, and the letters it put
    /// either side of a failure are not measurements either. One is enough to
    /// refuse the region.
    pub aqsa_majhula: usize,
    /// For [`hukm_tawafuq`]: the largest normalized edit distance between two
    /// independent reads of one region before neither is trusted.
    pub aqsa_khilaf: f64,
}

impl Default for HududQubul {
    fn default() -> Self {
        Self {
            nisbat_ramz: 0.12,
            nisbat_tashawwuh: 0.34,
            adna_huruf: 2,
            aqsa_majhula: 0,
            aqsa_khilaf: 0.25,
        }
    }
}

/// Whether a read is fit to be translated and drawn, and why not when it is not.
///
/// The reason is a sentence rather than a code because it is shown to the
/// player in the control panel next to the region that produced it. "This
/// region was read as `N 4 2 ? i m i? iAW` and refused: 8 of its 12 tokens are
/// malformed" is something a user can act on — they can move the region, or
/// turn the region off. "Low confidence" is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HukmQira {
    /// Translate it.
    Maqbul,
    /// Do not. A confidently wrong subtitle is worse than no subtitle.
    Marfud {
        /// What was wrong, in one sentence.
        sabab: String,
    },
}

impl HukmQira {
    /// Whether the read may be used.
    #[must_use]
    pub const fn maqbul(&self) -> bool {
        matches!(self, Self::Maqbul)
    }

    /// The refusal's reason, or [`None`] when there was none.
    #[must_use]
    pub fn sabab(&self) -> Option<&str> {
        match self {
            Self::Maqbul => None,
            Self::Marfud { sabab } => Some(sabab),
        }
    }
}

/// Whether one whitespace-separated token has a shape ordinary text does not.
///
/// Four clauses, and none of them is a dictionary — a dictionary would refuse
/// proper nouns, invented place names and every game's own vocabulary, which is
/// most of what an overlay has to read. These are shape rules, and each one
/// exists because the measured corpus produced it:
///
/// **A case change inside a word.** `iAW`, `REBSDTo`. Prose has `McCoy` and
/// `iPhone`, so this is counted rather than vetoed: one such token in a line is
/// ordinary, half of them is not.
///
/// **Letters and digits in one token.** `Me2`, `O1O00TC`. Real text does this
/// in weapon names and version numbers, so again it is counted.
///
/// **A stray single character.** `e`, `d`, `B`, `R`, `M` — five of the eleven
/// tokens in one measured read. English has two single-letter words and a game
/// has key names, so `a`, `A`, `i`, `I`, any digit and any lone punctuation
/// mark are exempt and everything else is counted. `Press F to pick up the
/// lantern` puts one such token in seven and stays well under the ceiling;
/// `e d B R M BEUA8G ?R THENE` puts five in eight and does not.
///
/// **All punctuation, more than one character.** `?!`, `»«`. A detector fired
/// on a border or a gradient.
///
/// There is no vowel-frequency clause, and its absence is deliberate: it was
/// written, measured against the corpus, and removed, because the only rule
/// that caught `GUPKNS` also caught `Blacksmith` and `strengths`. A clause with
/// no measured case behind it is a guess wearing a threshold.
#[must_use]
pub fn kalima_mushawwaha(kalima: &str) -> bool {
    let huruf: Vec<char> = kalima.chars().collect();
    if huruf.is_empty() {
        return false;
    }

    if huruf.len() == 1 {
        let wahid = huruf.first().copied().unwrap_or(' ');
        // A lone letter that is not a word and not a key name. Digits and
        // punctuation are ordinary on their own — `3`, `-`, `/` all appear in
        // real HUD text.
        return wahid.is_alphabetic() && !KALIMAT_HARF.contains(&wahid);
    }

    let mut sabiq_saghir = false;
    let mut fiha_harf = false;
    let mut fiha_raqm = false;
    let mut fiha_ghayr = false;
    let mut taghyeer_halat = false;

    for &harf in &huruf {
        if harf.is_alphabetic() {
            fiha_harf = true;
            if harf.is_uppercase() && sabiq_saghir {
                taghyeer_halat = true;
            }
            sabiq_saghir = harf.is_lowercase();
        } else {
            if harf.is_numeric() {
                fiha_raqm = true;
            } else {
                fiha_ghayr = true;
            }
            // A hyphen does not carry case across itself: `well-Known` is
            // ordinary text and a rule that fired on it would refuse dialogue.
            sabiq_saghir = false;
        }
    }

    if !fiha_harf && !fiha_raqm && fiha_ghayr {
        return true;
    }
    taghyeer_halat || (fiha_harf && fiha_raqm)
}

/// How many unread-glyph markers one token carries.
///
/// The portable engine emits `?` for a glyph it could not identify, which puts
/// the same character in two roles. The rule that separates them is positional
/// and it is exact: a `?` followed only by more punctuation is the punctuation
/// mark — `Save your progress?`, `What?!`, `Ready?` — and a `?` with letters
/// after it is the engine saying, in the only way it can, that it did not read
/// that glyph. `FW?REDLANTERN` is the second, and it came from a wordmark whose
/// real text is `THE RED LANTERN`.
fn alamat_majhula(kalima: &str) -> usize {
    let huruf: Vec<char> = kalima.chars().collect();
    let mut adad = 0_usize;
    for (mawdi, &harf) in huruf.iter().enumerate() {
        if harf != '?' {
            continue;
        }
        let baqi_rumuz = huruf
            .get(mawdi.saturating_add(1)..)
            .is_none_or(|baqi| baqi.iter().all(|&b| !b.is_alphanumeric()));
        if !baqi_rumuz {
            adad = adad.saturating_add(1);
        }
    }
    adad
}

/// Whether one character is one this crate believes the screen contained.
fn harf_maqbul(harf: char) -> bool {
    harf.is_alphanumeric() || harf == ' ' || harf == '?' || harf == '!'
        || RUMUZ_MAQBULA.contains(harf)
}

/// The shape statistics a refusal is decided on.
///
/// Returned together because the control panel shows all of them, and because a
/// caller tuning [`HududQubul`] for one game needs to see the numbers the
/// thresholds are being compared against rather than only the verdict.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IhsaBunya {
    /// Fraction of characters outside the set game text uses.
    pub nisbat_ramz: f64,
    /// Fraction of tokens malformed per [`kalima_mushawwaha`].
    pub nisbat_tashawwuh: f64,
    /// How many characters were read in total.
    pub adad_huruf: usize,
    /// How many glyphs the engine reported it could not identify.
    pub alamat_majhula: usize,
}

/// Measures a read's shape without judging it.
#[must_use]
pub fn ihsa_bunya(nass: &str) -> IhsaBunya {
    let kalimat: Vec<&str> = nass.split_whitespace().collect();
    let mut ramz = 0_usize;
    let mut kull = 0_usize;
    let mut majhula = 0_usize;
    for kalima in &kalimat {
        for harf in kalima.chars() {
            kull = kull.saturating_add(1);
            if !harf_maqbul(harf) {
                ramz = ramz.saturating_add(1);
            }
        }
        majhula = majhula.saturating_add(alamat_majhula(kalima));
    }
    let mushawwaha = kalimat.iter().filter(|kalima| kalima_mushawwaha(kalima)).count();

    IhsaBunya {
        nisbat_ramz: if kull == 0 { 0.0 } else { adad_f64(ramz) / adad_f64(kull) },
        nisbat_tashawwuh: if kalimat.is_empty() {
            0.0
        } else {
            adad_f64(mushawwaha) / adad_f64(kalimat.len())
        },
        adad_huruf: kull,
        alamat_majhula: majhula,
    }
}

/// The cheap gate: refuse a read whose shape is not the shape of text.
///
/// One pass over the string the engine already produced, so it costs nothing
/// next to recognition and can run on every region every time. It catches the
/// failure that matters most — a detector firing on a texture, a gradient or a
/// logo, and a recognizer dutifully returning letters for it — and it catches
/// it without a dictionary, a language model, or a confidence the engine does
/// not have.
///
/// It does not catch a *plausible* wrong read. `MARVEL FARERDIANS` for
/// `MARVEL GUARDIANS OF THE GALAXY` passes this gate, and [`hukm_tawafuq`] is
/// what is left for that case.
#[must_use]
pub fn hukm_bunya(sutur: &[SatrMaqru], hudud: &HududQubul) -> HukmQira {
    let nass = SatrMaqru::fiqra(sutur);
    let IhsaBunya { nisbat_ramz, nisbat_tashawwuh, adad_huruf: adad, alamat_majhula } =
        ihsa_bunya(&nass);

    if adad < hudud.adna_huruf {
        return HukmQira::Marfud {
            sabab: format!(
                "the region was read as {adad} character(s), which is below the {} this build \
                 will translate",
                hudud.adna_huruf
            ),
        };
    }
    if alamat_majhula > hudud.aqsa_majhula {
        return HukmQira::Marfud {
            sabab: format!(
                "the recognizer marked {alamat_majhula} glyph(s) as ones it could not identify, \
                 above the {} this build tolerates — it is reporting its own failure and the \
                 letters around them are guesses",
                hudud.aqsa_majhula
            ),
        };
    }
    if nisbat_ramz > hudud.nisbat_ramz {
        return HukmQira::Marfud {
            sabab: format!(
                "{:.0}% of what was read is characters this build does not believe were on the \
                 screen, above the {:.0}% ceiling — the recognizer was guessing at glyphs",
                nisbat_ramz * 100.0,
                hudud.nisbat_ramz * 100.0
            ),
        };
    }
    if nisbat_tashawwuh > hudud.nisbat_tashawwuh {
        return HukmQira::Marfud {
            sabab: format!(
                "{:.0}% of the words read have a shape ordinary text does not — a case change \
                 inside a word, letters mixed with digits, a stray single letter, or nothing \
                 but punctuation — above the {:.0}% ceiling",
                nisbat_tashawwuh * 100.0,
                hudud.nisbat_tashawwuh * 100.0
            ),
        };
    }
    HukmQira::Maqbul
}

/// Levenshtein distance in characters.
///
/// Two rows rather than a full matrix: the strings compared here are one
/// region's worth of text, but this runs per region per pass and the full
/// matrix would be an allocation proportional to the product of two lengths for
/// a number that only needs the previous row.
fn masafat_tahrir(awwal: &str, thani: &str) -> usize {
    let a: Vec<char> = awwal.chars().collect();
    let b: Vec<char> = thani.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut sabiq: Vec<usize> = (0..=b.len()).collect();
    let mut hali = vec![0_usize; b.len().saturating_add(1)];
    for (i, ha) in a.iter().enumerate() {
        if let Some(khana) = hali.first_mut() {
            *khana = i.saturating_add(1);
        }
        for (j, hb) in b.iter().enumerate() {
            let takleefa = usize::from(ha != hb);
            let qutri = sabiq.get(j).copied().unwrap_or(0).saturating_add(takleefa);
            let fawq = sabiq.get(j.saturating_add(1)).copied().unwrap_or(0).saturating_add(1);
            let yasar = hali.get(j).copied().unwrap_or(0).saturating_add(1);
            if let Some(khana) = hali.get_mut(j.saturating_add(1)) {
                *khana = qutri.min(fawq).min(yasar);
            }
        }
        core::mem::swap(&mut sabiq, &mut hali);
    }
    sabiq.last().copied().unwrap_or(0)
}

/// How far apart two reads of one region are, zero to one.
///
/// Normalized by the longer of the two, so a read that is a truncation of the
/// other scores by how much was lost rather than by how long the survivor is.
#[must_use]
pub fn khilaf_qiraatayn(awwal: &[SatrMaqru], thani: &[SatrMaqru]) -> f64 {
    let a = SatrMaqru::fiqra(awwal);
    let b = SatrMaqru::fiqra(thani);
    let tul = a.chars().count().max(b.chars().count());
    if tul == 0 {
        return 0.0;
    }
    adad_f64(masafat_tahrir(&a, &b)) / adad_f64(tul)
}

/// The expensive gate: refuse a region two independent reads disagree about.
///
/// The two reads are the same engine over the same region through two
/// preprocessing paths that must **differ in the pixels they hand the engine**.
/// That is a corroboration test rather than a heuristic about English, and it is
/// the only thing in this crate that catches a *plausible* wrong read: when a
/// region is genuinely legible both paths converge on the same string, and when
/// it is not they diverge, because what each is reading is its own preprocessing
/// artefacts rather than the text.
///
/// ## Which two paths, and why not the obvious two
///
/// The pair to use is
/// [`crate::iltiqat_shasha::MuhassinSura::hassin_lil_qari`] with
/// [`crate::iltiqat_shasha::IdadatTahsin::yuhawwil_ila_ramadi`] **set both
/// ways** — colour against the grayscale chain. [`IkhtiyarQari::iqra_mufattasha`]
/// runs exactly that pair and is the call to make.
///
/// The pair that reads as obvious — the raw capture against `hassin_lil_qari`'s
/// output — is worse than useless, and the reason is measured rather than
/// argued. In its colour shape `hassin_lil_qari` hands the recognizer the
/// capture's own colour, upscaled, and the upscale fires only on text below the
/// height floor; on everything else the two "paths" are the same pixels. Over
/// 837 real region crops the raw read and the default read were **character-for-
/// character identical on 830 of them, 99.2%**, so the gate could not refuse
/// anything it was given. Colour against grayscale returned different text on
/// 378 of the same 837, and over 768 regions that carry no readable text it cut
/// what got past the structural gate from 29 to 0.
///
/// It costs a second recognition pass, which is the most expensive thing this
/// crate does. That cost is bounded by running it second: over those 837
/// regions the structural gate accepted 86, so the second pass runs on about a
/// tenth of the regions a capture loop hands this crate, and never on one that
/// has already been refused.
#[must_use]
pub fn hukm_tawafuq(
    awwal: &[SatrMaqru],
    thani: &[SatrMaqru],
    hudud: &HududQubul,
) -> HukmQira {
    let khilaf = khilaf_qiraatayn(awwal, thani);
    if khilaf > hudud.aqsa_khilaf {
        return HukmQira::Marfud {
            sabab: format!(
                "two preprocessing paths read this region differently — {:.0}% of the \
                 characters disagree, above the {:.0}% ceiling — so neither read is what is on \
                 the screen",
                khilaf * 100.0,
                hudud.aqsa_khilaf * 100.0
            ),
        };
    }
    HukmQira::Maqbul
}

/// What can honestly be said about one region's read.
///
/// The distinction this type exists for is the one the module header opens
/// with: [`ThiqatMintaqa::thiqa`] is [`None`] when the engine reports no
/// confidence, and it is [`None`] rather than [`THIQA_GHAYR_MAQISA`] because a
/// caller that receives a number will compare it against a threshold and a
/// caller that receives [`None`] cannot. The stand-in constant stays on
/// [`SatrMaqru`] for the engines that have to put *something* there; nothing
/// that reports upward should carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThiqatMintaqa {
    /// The engine's own lowest per-line confidence, or [`None`] when it
    /// measures none. Never this crate's stand-in.
    pub thiqa: Option<u8>,
    /// Whether the read may be translated.
    pub hukm: HukmQira,
}

impl ThiqatMintaqa {
    /// Judges a set of lines with the structural gate.
    #[must_use]
    pub fn min_sutur(sutur: &[SatrMaqru], hudud: &HududQubul) -> Self {
        let maqisa = !sutur.is_empty() && sutur.iter().all(|satr| satr.maqisa);
        Self {
            thiqa: if maqisa { SatrMaqru::adna_thiqa(sutur) } else { None },
            hukm: hukm_bunya(sutur, hudud),
        }
    }

    /// The sentence the control panel shows.
    ///
    /// Says "unmeasured" where the engine supplies no confidence, in those
    /// words, rather than printing the stand-in and letting a reader assume the
    /// number came from somewhere.
    #[must_use]
    pub fn wasf(&self) -> String {
        let thiqa = self.thiqa.map_or_else(
            || "confidence: unmeasured — this engine reports none".to_owned(),
            |q| format!("confidence: {q}%, the engine's own"),
        );
        match &self.hukm {
            HukmQira::Maqbul => format!("{thiqa}; accepted"),
            HukmQira::Marfud { sabab } => format!("{thiqa}; refused — {sabab}"),
        }
    }
}
