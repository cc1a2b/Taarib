//! الصف عبر الحدود — the engine itself.
//!
//! One [`TaaribSaff`] per adapter, alive for the life of the game. It holds
//! the prepared shaper for each font — the expensive part of shaping, and the
//! part a game would otherwise pay for on every frame — so the second layout
//! of a string over the same chain costs a fraction of the first. It is a
//! mutable cache and deliberately not shared: JavaScript is single-threaded
//! per realm, so the lock the native library needs around a context has no
//! work to do here and does not exist.
//!
//! Three calls. [`TaaribSaff::khattit`] lays clean text out;
//! [`TaaribSaff::qis`] measures without positioning, for `textWidth`,
//! `calcTextHeight` and `measureText`; and [`TaaribSaff::khattit_khaam`]
//! takes text that still carries its markup — RPG Maker escape codes, `BBCode`,
//! Unity tags — lifts the markup into spans, and lays out what is left,
//! because a tag that reaches the bidirectional algorithm corrupts every
//! mixed-direction line it touches.
//!
//! There is no layout cache here, and that is not an omission. The native
//! library hides one behind its ABI because a C caller cannot cheaply hold a
//! finished layout; a JavaScript caller can — the layout *is* an object it
//! holds. An adapter whose message has not changed keeps the
//! [`TaaribTakhtit`] it already has and re-blits from the rows it already
//! read, which is the same shaped-once behaviour the C side's cache provides,
//! without a second copy of the eviction policy to keep correct. Free the
//! layout when the message changes, not per frame.

use taarib_saff::nasq::{self, KhiyaratNasq, LahjatNasq, NassNaqi};
use taarib_saff::natija::TakhtitNass;
use taarib_saff::talab::{KhiyaratTakhtit, TalabTakhtit};
use taarib_saff::{Saff, khatt::SilsilatKhutut};
use taarib_usus::khata::Natija;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::khata_js;
use crate::khatt_js::TaaribSilsila;
use crate::natija_js::{TaaribQiyas, TaaribTakhtit, TaaribTakhtitKhaam};
use crate::talab_js::{TaaribKhiyarat, TaaribTalab};

/// Which markup dialect [`TaaribSaff::khattit_khaam`] should recognize.
///
/// One dialect per call, because the dialects genuinely conflict — GameMaker
/// reads a bare `#` as a line break that would cut `Level #3` in half for
/// every other engine — and an adapter knows exactly which engine it is
/// inside. Any unrecognised number extracts nothing but the
/// dialect-independent escapes, which is the correct behaviour for a plain
/// string table.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lahja {
    /// Unity rich text, as TextMeshPro and the legacy UI parse it.
    Unity = 0,
    /// Unreal's `<Style>text</>` rich text.
    Unreal = 1,
    /// `BBCode`, as Godot's `RichTextLabel` and Ren'Py's `BBCode` mode parse it.
    BbCode = 2,
    /// Ren'Py's own `{tag}` text tags and `[variable]` substitutions.
    RenPy = 3,
    /// RPG Maker escape codes: `\V[n]`, `\C[n]`, `\I[n]`, `\G` and the rest.
    RpgMaker = 4,
    /// GameMaker's `#` line break, and `\#` as the escape for a literal hash.
    GameMaker = 5,
    /// Web-style substitution: `{{name}}` and `${name}`.
    Web = 6,
}

/// The engine: logical-order text in, ordered visual lines of positioned
/// glyphs out.
///
/// Create one, keep it for the life of the game, and hand every layout and
/// every measurement through it so its shaping caches earn their memory.
/// [`TaaribSaff::amsah`] gives the memory back when a set of fonts is
/// finished with — a title returning to its menu after unloading a chapter's
/// chain.
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct TaaribSaff {
    saff: Saff,
}

#[wasm_bindgen]
impl TaaribSaff {
    /// A new engine with empty caches.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn jadeed() -> Self {
        Self {
            saff: Saff::jadeed(),
        }
    }

    /// Lays text out.
    ///
    /// Runs every stage in order — policies, bidirectional analysis, run
    /// splitting, break opportunities, shaping, measurement and reflow,
    /// reordering, justification, positioning — and returns the finished
    /// layout as packed rows. The cluster index on every returned glyph
    /// refers to the text in the request, exactly as the caller passed it.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error carrying whatever any stage reported: an
    /// unusable width or size, a style span that does not fit its text,
    /// markup nested past the bidirectional algorithm's limit, or a font
    /// that cannot shape the run it was given.
    pub fn khattit(&mut self, talab: &TaaribTalab) -> Result<TaaribTakhtit, JsValue> {
        let takhtit = khata_js::min_natija(self.saff.khattit(&TalabTakhtit {
            nass: &talab.nass,
            khutut: &talab.silsila,
            hajm: talab.hajm,
            ard_mutah: talab.ard_mutah,
            irtifa_mutah: talab.irtifa_mutah,
            nitaqat: &talab.nitaqat,
            khiyarat: &talab.khiyarat,
        }))?;
        Ok(TaaribTakhtit { takhtit })
    }

    /// Measures text without positioning glyphs, applying the same policies
    /// and running the same pipeline.
    ///
    /// This is what `Bitmap.measureTextWidth`, `textWidth` and
    /// `measureText` route through: the answer comes from real shaped
    /// advances, so the number an adapter reserves room with is the number
    /// the layout will actually occupy.
    ///
    /// # Errors
    ///
    /// Throws as [`TaaribSaff::khattit`] throws.
    pub fn qis(&mut self, talab: &TaaribTalab) -> Result<TaaribQiyas, JsValue> {
        let qiyas = khata_js::min_natija(self.saff.qis(&TalabTakhtit {
            nass: &talab.nass,
            khutut: &talab.silsila,
            hajm: talab.hajm,
            ard_mutah: talab.ard_mutah,
            irtifa_mutah: talab.irtifa_mutah,
            nitaqat: &talab.nitaqat,
            khiyarat: &talab.khiyarat,
        }))?;
        Ok(TaaribQiyas::min_dakhili(qiyas))
    }

    /// Lays out text that still carries its markup, with default layout
    /// decisions.
    ///
    /// `lahja` is a [`Lahja`] value naming the one dialect this text is
    /// written in. The markup and the placeholders are lifted into spans and
    /// atoms first — see [`TaaribTakhtitKhaam`] — and the clean remainder is
    /// laid out. `ard` is the available width in pixels, or `undefined` for
    /// one unbounded line.
    ///
    /// This is the RPG Maker plugin's whole entry point: `drawTextEx` hands
    /// the raw message here with [`Lahja::RpgMaker`], and the escape codes
    /// come back as atoms rather than glyphs.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error for malformed markup where the dialect
    /// itself would refuse it, plus everything [`TaaribSaff::khattit`]
    /// throws.
    pub fn khattit_khaam(
        &mut self,
        khaam: &str,
        lahja: u32,
        silsila: &TaaribSilsila,
        hajm: f32,
        ard: Option<f32>,
    ) -> Result<TaaribTakhtitKhaam, JsValue> {
        let khiyarat = KhiyaratTakhtit::default();
        let (naqi, takhtit) = khata_js::min_natija(khaam_dakhili(
            &mut self.saff,
            khaam,
            lahja,
            &silsila.silsila,
            hajm,
            ard,
            None,
            &khiyarat,
        ))?;
        Ok(TaaribTakhtitKhaam { naqi, takhtit })
    }

    /// [`TaaribSaff::khattit_khaam`], with the patch's recorded decisions and
    /// an available height.
    ///
    /// The five-argument form uses every default; this form is the one the
    /// adapters actually ship with, because a patch records its digit policy,
    /// its diacritic policy and its justification mode, and a message window
    /// has a height.
    ///
    /// # Errors
    ///
    /// Throws as [`TaaribSaff::khattit_khaam`] throws.
    pub fn khattit_khaam_bi(
        &mut self,
        khaam: &str,
        lahja: u32,
        silsila: &TaaribSilsila,
        hajm: f32,
        ard: Option<f32>,
        irtifa: Option<f32>,
        khiyarat: &TaaribKhiyarat,
    ) -> Result<TaaribTakhtitKhaam, JsValue> {
        let khiyarat = khiyarat.ila_dakhili();
        let (naqi, takhtit) = khata_js::min_natija(khaam_dakhili(
            &mut self.saff,
            khaam,
            lahja,
            &silsila.silsila,
            hajm,
            ard,
            irtifa,
            &khiyarat,
        ))?;
        Ok(TaaribTakhtitKhaam { naqi, takhtit })
    }

    /// Empties the shaping caches, for a caller that has finished with a set
    /// of fonts and wants the memory back.
    pub fn amsah(&mut self) {
        self.saff.amsah();
    }

    /// How many fonts are currently prepared in the shaping cache.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_khutut(&self) -> u32 {
        u32::try_from(self.saff.adad_khutut()).unwrap_or(u32::MAX)
    }
}

/// Extraction plus layout, shared by both raw entry points.
///
/// The relative-size base for `<size=+2>`-style tags is the request's own
/// size, because that is what such a tag is relative to in every dialect that
/// has one. Width and height are sanitized the same way the request setters
/// sanitize them: non-finite or non-positive means unbounded.
fn khaam_dakhili(
    saff: &mut Saff,
    khaam: &str,
    lahja: u32,
    silsila: &SilsilatKhutut,
    hajm: f32,
    ard: Option<f32>,
    irtifa: Option<f32>,
    khiyarat: &KhiyaratTakhtit,
) -> Natija<(NassNaqi, TakhtitNass)> {
    let mut khiyarat_nasq = match lahja_min_raqm(lahja) {
        Some(wahida) => KhiyaratNasq::wahida(wahida),
        None => KhiyaratNasq::bila_lahja(),
    };
    khiyarat_nasq.hajm_asas = Some(hajm);

    let naqi = nasq::istakhrij(khaam, &khiyarat_nasq)?;
    let takhtit = saff.khattit(&TalabTakhtit {
        nass: &naqi.nass,
        khutut: silsila,
        hajm,
        ard_mutah: ard.filter(|qeema| qeema.is_finite() && *qeema > 0.0),
        irtifa_mutah: irtifa.filter(|qeema| qeema.is_finite() && *qeema > 0.0),
        nitaqat: &naqi.nitaqat,
        khiyarat,
    })?;
    Ok((naqi, takhtit))
}

/// Reads the dialect discriminant. Unknown numbers select no dialect at all,
/// which extracts only the dialect-independent escapes — the honest reading
/// of a number this build does not know.
const fn lahja_min_raqm(raqm: u32) -> Option<LahjatNasq> {
    match raqm {
        0 => Some(LahjatNasq::Unity),
        1 => Some(LahjatNasq::Unreal),
        2 => Some(LahjatNasq::BbCode),
        3 => Some(LahjatNasq::RenPy),
        4 => Some(LahjatNasq::RpgMaker),
        5 => Some(LahjatNasq::GameMaker),
        6 => Some(LahjatNasq::Web),
        _ => None,
    }
}
