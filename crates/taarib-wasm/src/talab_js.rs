//! الطلب عبر الحدود — what JavaScript hands the engine.
//!
//! The same three-part request the C ABI takes — text plus chain plus size,
//! style spans over the text, and the recorded decisions — with the same
//! names and the same discriminant numbering, so an adapter author moving
//! between the C# side and this one recognises every field. The difference is
//! idiom, not shape: where C fills a `#[repr(C)]` struct and a flag word,
//! JavaScript assigns properties and calls methods, and unset simply means
//! `undefined` rather than a cleared bit.
//!
//! Discriminants decode exactly as `taarib-jisr::anwa` decodes them, an
//! unrecognised number falling back to the documented default rather than
//! failing — an adapter built against a newer numbering gets the default
//! behaviour, not a refusal. The enums here ([`IttijahAsas`], [`LughaNass`],
//! [`NamatDabt`], [`Muhadhaha`], [`SiyasatTashkeel`], [`SiyasatArqam`],
//! [`SiyasatTajawuz`]) are names for those numbers so no adapter hardcodes
//! them.

use taarib_saff::talab as dakhili;
use taarib_saff::talab::{KhiyaratTakhtit, NitaqUslub, SifaIdafiya, Uslub};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::khatt_js::TaaribSilsila;

/// How the paragraph's base direction is decided. Values for
/// [`TaaribKhiyarat::ittijah`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IttijahAsas {
    /// From the first strong character — the default, and what a game's own
    /// text should get.
    Tilqai = 0,
    /// Forced right to left.
    Yameen = 1,
    /// Forced left to right.
    Yasar = 2,
}

/// The language the text is in. Values for [`TaaribKhiyarat::lugha`].
///
/// Not decoration: Persian and Urdu share the Arabic script but not its
/// letterforms, and the font expresses the difference through `locl` lookups
/// keyed on this.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LughaNass {
    /// Detected from the text itself.
    Tilqai = 0,
    /// Arabic.
    Arabi = 1,
    /// Persian.
    Farisi = 2,
    /// Urdu.
    Urdu = 3,
    /// Latin-script text.
    Latini = 4,
}

/// How surplus width on a justified line is absorbed. Values for
/// [`TaaribKhiyarat::dabt`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamatDabt {
    /// Leave the surplus at the end of the line.
    Bila = 0,
    /// Stretch the spaces — the only mode Latin runs ever use.
    Masafat = 1,
    /// Elongate letters at legitimate points found from shaped joining
    /// behaviour, and reshape.
    Kashida = 2,
    /// Elongate first, then stretch spaces for the remainder — the default
    /// for Arabic, and what hand-set Arabic actually does.
    KashidaThummaMasafat = 3,
}

/// Where a line sits inside the width available to it. Values for
/// [`TaaribKhiyarat::muhadhaha`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Muhadhaha {
    /// Against the leading edge — the right in Arabic, the left in Latin.
    Bidaya = 0,
    /// Against the trailing edge.
    Nihaya = 1,
    /// Centred.
    Wasat = 2,
    /// Filled to the full width by the justification mode, except on a
    /// paragraph's last line.
    Dabt = 3,
}

/// What happens to diacritics. Values for [`TaaribKhiyarat::tashkeel`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiyasatTashkeel {
    /// Keep every mark the source carries.
    Ibqa = 0,
    /// Remove marks before shaping.
    Hadhf = 1,
    /// Keep marks in text marked as dialogue, remove them elsewhere.
    IbqaFilHiwar = 2,
}

/// Which digits the rendered text uses. Values for
/// [`TaaribKhiyarat::arqam`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiyasatArqam {
    /// Leave every digit exactly as the translation wrote it.
    KamaHiya = 0,
    /// Map to European digits.
    Latini = 1,
    /// Map to Arabic-Indic digits.
    Arabi = 2,
    /// Map to Eastern Arabic-Indic digits, for Persian and Urdu.
    Farisi = 3,
}

/// What to do when text does not fit and cannot be broken. Values for
/// [`TaaribKhiyarat::tajawuz`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiyasatTajawuz {
    /// Lay it out anyway and report the overflow — the default, because a
    /// patch that silently shrinks text hides its own defects.
    Ballagh = 0,
    /// Shrink the size until it fits, down to
    /// [`TaaribKhiyarat::hajm_adna`].
    Taqlis = 1,
    /// Truncate and mark the cut with the ellipsis on the correct side.
    Ikhtisar = 2,
}

/// The decisions a caller makes once and records into a patch.
///
/// Construct one, assign the fields the patch recorded, and hand it to
/// [`TaaribTalab::bi_khiyarat`] or to
/// [`crate::saff_js::TaaribSaff::khattit_khaam_bi`]. Every field defaults to
/// the same default the C ABI documents, and an unrecognised discriminant
/// decodes to that default rather than failing.
#[wasm_bindgen]
#[derive(Debug, Clone, Default)]
pub struct TaaribKhiyarat {
    /// How the base direction is decided — a value of [`IttijahAsas`].
    pub ittijah: u32,
    /// The language — a value of [`LughaNass`].
    pub lugha: u32,
    /// How surplus width is absorbed — a value of [`NamatDabt`].
    pub dabt: u32,
    /// Where a line sits in the available width — a value of [`Muhadhaha`].
    pub muhadhaha: u32,
    /// What happens to diacritics — a value of [`SiyasatTashkeel`].
    pub tashkeel: u32,
    /// Which digits the output uses — a value of [`SiyasatArqam`].
    pub arqam: u32,
    /// What happens when text does not fit — a value of [`SiyasatTajawuz`].
    pub tajawuz: u32,
    /// The smallest size shrink-to-fit may use, in pixels. Zero or less
    /// selects the default floor of eight pixels.
    pub hajm_adna: f32,
    /// Line height in pixels; zero or less means the font's own metrics
    /// decide.
    pub irtifa_satr: f32,
    /// Extra spacing between every pair of glyphs, in pixels — applied after
    /// shaping, so it never disturbs joining.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space, in pixels.
    pub tabaud_kalimat: f32,
    /// Whether this text is dialogue, which the keep-diacritics-in-dialogue
    /// policy keys on.
    pub hiwar: bool,
    /// Whether wrapping is forbidden entirely, as in a single-line field.
    pub satr_wahid: bool,
    sifat: Vec<SifaIdafiya>,
}

#[wasm_bindgen]
impl TaaribKhiyarat {
    /// The defaults: automatic direction, detected language, no
    /// justification, leading-edge alignment, diacritics kept, digits left
    /// alone, overflow reported.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Turns an OpenType feature on or off beyond the script's defaults.
    ///
    /// `wasm` is the four-character feature tag, for example `ss01`; a longer
    /// value is truncated and a shorter one padded with spaces, which is what
    /// OpenType itself does with short tags. `qeema` is the feature's value,
    /// zero turning it off.
    pub fn adif_sifa(&mut self, wasm: &str, qeema: u32) {
        let mut ramz = [b' '; 4];
        for (khana, bayt) in ramz.iter_mut().zip(wasm.bytes()) {
            *khana = bayt;
        }
        self.sifat.push(SifaIdafiya { wasm: ramz, qeema });
    }

    /// Removes every feature override added with
    /// [`TaaribKhiyarat::adif_sifa`].
    pub fn amsah_sifat(&mut self) {
        self.sifat.clear();
    }
}

impl TaaribKhiyarat {
    /// Decodes the recorded numbers into the engine's own decisions.
    ///
    /// The numbering — and the fallback of every unknown number to its
    /// documented default — mirrors `taarib-jisr::anwa`'s decoders exactly,
    /// so a patch's recorded values mean the same thing to the native library
    /// and to this module.
    pub(crate) fn ila_dakhili(&self) -> KhiyaratTakhtit {
        KhiyaratTakhtit {
            ittijah: ittijah_min_raqm(self.ittijah),
            lugha: lugha_min_raqm(self.lugha),
            dabt: dabt_min_raqm(self.dabt),
            muhadhaha: muhadhaha_min_raqm(self.muhadhaha),
            tashkeel: tashkeel_min_raqm(self.tashkeel),
            arqam: arqam_min_raqm(self.arqam),
            tajawuz: tajawuz_min_raqm(self.tajawuz, self.hajm_adna),
            irtifa_satr: (self.irtifa_satr > 0.0).then_some(self.irtifa_satr),
            tabaud_ahruf: self.tabaud_ahruf,
            tabaud_kalimat: self.tabaud_kalimat,
            hiwar: self.hiwar,
            satr_wahid: self.satr_wahid,
            sifat: self.sifat.clone(),
        }
    }
}

/// One style span over the text, buildable property by property.
///
/// The idiomatic form of the C ABI's `TaaribNitaqUslub`: where C sets a flag
/// bit and fills the matching field, JavaScript assigns the property or
/// leaves it `undefined`. Only what changes layout is read by the engine;
/// colour is carried through untouched so it survives reordering.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct TaaribNitaq {
    pub(crate) nitaq: NitaqUslub,
}

#[wasm_bindgen]
impl TaaribNitaq {
    /// A span over `tul` bytes starting at byte `bidaya` of the clean text,
    /// identified in the output as `id`.
    ///
    /// Both offsets are byte offsets and must fall on character boundaries —
    /// the request is refused at layout time otherwise, with the reason.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn jadeed(id: u16, bidaya: u32, tul: u32) -> Self {
        Self {
            nitaq: NitaqUslub { id, bidaya, tul, uslub: Uslub::default() },
        }
    }

    /// Which font in the chain this span prefers, or `undefined` for the
    /// chain's own choice.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn khatt(&self) -> Option<u8> {
        self.nitaq.uslub.khatt
    }

    /// Sets the preferred font index.
    #[wasm_bindgen(setter)]
    pub fn set_khatt(&mut self, khatt: Option<u8>) {
        self.nitaq.uslub.khatt = khatt;
    }

    /// A variable-font weight for this span, or `undefined`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn wazn(&self) -> Option<u16> {
        self.nitaq.uslub.wazn
    }

    /// Sets the weight.
    #[wasm_bindgen(setter)]
    pub fn set_wazn(&mut self, wazn: Option<u16>) {
        self.nitaq.uslub.wazn = wazn;
    }

    /// Whether the span is italic or slanted.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn maail(&self) -> bool {
        self.nitaq.uslub.maail
    }

    /// Sets the italic flag.
    #[wasm_bindgen(setter)]
    pub fn set_maail(&mut self, maail: bool) {
        self.nitaq.uslub.maail = maail;
    }

    /// A size override in pixels, or `undefined`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hajm(&self) -> Option<f32> {
        self.nitaq.uslub.hajm
    }

    /// Sets the size override.
    #[wasm_bindgen(setter)]
    pub fn set_hajm(&mut self, hajm: Option<f32>) {
        self.nitaq.uslub.hajm = hajm;
    }

    /// Colour as `0xRRGGBBAA`, or `undefined`. Carried through to the output
    /// untouched — the engine does not draw, but a coloured word must keep
    /// its colour across reordering.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn lawn(&self) -> Option<u32> {
        self.nitaq.uslub.lawn.map(u32::from_be_bytes)
    }

    /// Sets the colour.
    #[wasm_bindgen(setter)]
    pub fn set_lawn(&mut self, lawn: Option<u32>) {
        self.nitaq.uslub.lawn = lawn.map(u32::to_be_bytes);
    }

    /// Extra letter spacing for this span in pixels, or `undefined`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn tabaud(&self) -> Option<f32> {
        self.nitaq.uslub.tabaud
    }

    /// Sets the letter spacing.
    #[wasm_bindgen(setter)]
    pub fn set_tabaud(&mut self, tabaud: Option<f32>) {
        self.nitaq.uslub.tabaud = tabaud;
    }

    /// A vertical offset from the baseline in pixels, or `undefined`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn izaha(&self) -> Option<f32> {
        self.nitaq.uslub.izaha
    }

    /// Sets the vertical offset.
    #[wasm_bindgen(setter)]
    pub fn set_izaha(&mut self, izaha: Option<f32>) {
        self.nitaq.uslub.izaha = izaha;
    }

    /// Makes this span an opaque atom — a format placeholder or an inline
    /// icon — instead of text.
    ///
    /// An atom occupies `ard` by `irtifa` pixels with its bottom edge `asas`
    /// pixels above the baseline, participates in line breaking and
    /// justification as a unit, and is never shaped. `marja` is the caller's
    /// own reference — the RPG Maker plugin puts the icon index from `\I[n]`
    /// here — and comes back untouched on the glyphless span in the output.
    pub fn dharra(&mut self, ard: f32, irtifa: f32, asas: f32, marja: u32) {
        self.nitaq.uslub.dharra = Some(dakhili::Dharra { ard, irtifa, asas, marja });
    }
}

/// One layout request: the text, the chain, the size, the room available,
/// the spans, and the decisions.
///
/// Reusable on purpose — an adapter that lays out every frame keeps one
/// request alive, reassigns [`TaaribTalab::nass`] when the message changes,
/// and hands the same object to the engine each time. The text must be clean
/// logical-order text; raw markup goes through
/// [`crate::saff_js::TaaribSaff::khattit_khaam`] instead, because a tag put
/// through the bidirectional algorithm is the single most common way a text
/// stack corrupts mixed-direction text.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribTalab {
    pub(crate) nass: String,
    pub(crate) silsila: taarib_saff::khatt::SilsilatKhutut,
    pub(crate) hajm: f32,
    pub(crate) ard_mutah: Option<f32>,
    pub(crate) irtifa_mutah: Option<f32>,
    pub(crate) nitaqat: Vec<NitaqUslub>,
    pub(crate) khiyarat: KhiyaratTakhtit,
}

#[wasm_bindgen]
impl TaaribTalab {
    /// A request for `nass` at `hajm` pixels with `silsila`, on one unbounded
    /// line until a width is assigned.
    ///
    /// The chain is shared by reference count, so building many requests over
    /// one chain costs nothing per request.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn jadeed(nass: String, silsila: &TaaribSilsila, hajm: f32) -> Self {
        Self {
            nass,
            silsila: silsila.silsila.clone(),
            hajm,
            ard_mutah: None,
            irtifa_mutah: None,
            nitaqat: Vec::new(),
            khiyarat: KhiyaratTakhtit::default(),
        }
    }

    /// The logical-order text.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn nass(&self) -> String {
        self.nass.clone()
    }

    /// Replaces the text, keeping everything else — the per-frame path for a
    /// window whose message changes.
    #[wasm_bindgen(setter)]
    pub fn set_nass(&mut self, nass: String) {
        self.nass = nass;
    }

    /// The size in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hajm(&self) -> f32 {
        self.hajm
    }

    /// Sets the size in pixels.
    #[wasm_bindgen(setter)]
    pub fn set_hajm(&mut self, hajm: f32) {
        self.hajm = hajm;
    }

    /// The width available in pixels, or `undefined` for one line of
    /// whatever width the text needs.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ard_mutah(&self) -> Option<f32> {
        self.ard_mutah
    }

    /// Sets the available width. `undefined`, a non-finite number and
    /// anything at or below zero all mean unbounded, matching the C ABI's
    /// reading of a non-positive width.
    #[wasm_bindgen(setter)]
    pub fn set_ard_mutah(&mut self, ard: Option<f32>) {
        self.ard_mutah = ard.filter(|qeema| qeema.is_finite() && *qeema > 0.0);
    }

    /// The height available in pixels, or `undefined` for unbounded.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn irtifa_mutah(&self) -> Option<f32> {
        self.irtifa_mutah
    }

    /// Sets the available height, read the same way as the width.
    #[wasm_bindgen(setter)]
    pub fn set_irtifa_mutah(&mut self, irtifa: Option<f32>) {
        self.irtifa_mutah = irtifa.filter(|qeema| qeema.is_finite() && *qeema > 0.0);
    }

    /// Replaces the chain, for a style change that swaps fonts wholesale.
    pub fn bi_silsila(&mut self, silsila: &TaaribSilsila) {
        self.silsila = silsila.silsila.clone();
    }

    /// Records the decisions, decoding them once so every later layout pays
    /// nothing for the translation.
    pub fn bi_khiyarat(&mut self, khiyarat: &TaaribKhiyarat) {
        self.khiyarat = khiyarat.ila_dakhili();
    }

    /// Adds a style span. The span object stays usable — its current state is
    /// copied in.
    pub fn adif_nitaq(&mut self, nitaq: &TaaribNitaq) {
        self.nitaqat.push(nitaq.nitaq);
    }

    /// Removes every style span, for reusing the request over unstyled text.
    pub fn amsah_nitaqat(&mut self) {
        self.nitaqat.clear();
    }

    /// How many style spans the request carries.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_nitaqat(&self) -> u32 {
        u32::try_from(self.nitaqat.len()).unwrap_or(u32::MAX)
    }
}

// ---------------------------------------------------------------------------
// Discriminant decoding — the same numbering, and the same defaults for the
// unknown, as `taarib-jisr::anwa`.
// ---------------------------------------------------------------------------

/// Reads the direction discriminant, defaulting to automatic.
const fn ittijah_min_raqm(raqm: u32) -> dakhili::IttijahAsas {
    match raqm {
        1 => dakhili::IttijahAsas::Yameen,
        2 => dakhili::IttijahAsas::Yasar,
        _ => dakhili::IttijahAsas::Tilqai,
    }
}

/// Reads the language discriminant, defaulting to detection.
const fn lugha_min_raqm(raqm: u32) -> dakhili::LughaNass {
    match raqm {
        1 => dakhili::LughaNass::Arabi,
        2 => dakhili::LughaNass::Farisi,
        3 => dakhili::LughaNass::Urdu,
        4 => dakhili::LughaNass::Latini,
        _ => dakhili::LughaNass::Tilqai,
    }
}

/// Reads the justification discriminant, defaulting to none.
const fn dabt_min_raqm(raqm: u32) -> dakhili::NamatDabt {
    match raqm {
        1 => dakhili::NamatDabt::Masafat,
        2 => dakhili::NamatDabt::Kashida,
        3 => dakhili::NamatDabt::KashidaThummaMasafat,
        _ => dakhili::NamatDabt::Bila,
    }
}

/// Reads the alignment discriminant, defaulting to the leading edge.
const fn muhadhaha_min_raqm(raqm: u32) -> dakhili::Muhadhaha {
    match raqm {
        1 => dakhili::Muhadhaha::Nihaya,
        2 => dakhili::Muhadhaha::Wasat,
        3 => dakhili::Muhadhaha::Dabt,
        _ => dakhili::Muhadhaha::Bidaya,
    }
}

/// Reads the diacritic discriminant, defaulting to keeping them.
const fn tashkeel_min_raqm(raqm: u32) -> dakhili::SiyasatTashkeel {
    match raqm {
        1 => dakhili::SiyasatTashkeel::Hadhf,
        2 => dakhili::SiyasatTashkeel::IbqaFilHiwar,
        _ => dakhili::SiyasatTashkeel::Ibqa,
    }
}

/// Reads the digit discriminant, defaulting to leaving digits alone.
const fn arqam_min_raqm(raqm: u32) -> dakhili::SiyasatArqam {
    match raqm {
        1 => dakhili::SiyasatArqam::Latini,
        2 => dakhili::SiyasatArqam::Arabi,
        3 => dakhili::SiyasatArqam::Farisi,
        _ => dakhili::SiyasatArqam::KamaHiya,
    }
}

/// Reads the overflow discriminant, defaulting to reporting.
///
/// Reporting is the default on purpose: a caller that sends an unrecognised
/// number gets a layout plus an honest measurement of how far past its bounds
/// it went, rather than silently shrunk or truncated text.
fn tajawuz_min_raqm(raqm: u32, hajm_adna: f32) -> dakhili::SiyasatTajawuz {
    match raqm {
        1 => dakhili::SiyasatTajawuz::Taqlis {
            adna: if hajm_adna > 0.0 { hajm_adna } else { 8.0 },
        },
        2 => dakhili::SiyasatTajawuz::Ikhtisar,
        _ => dakhili::SiyasatTajawuz::Ballagh,
    }
}
