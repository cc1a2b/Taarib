//! الخط عبر الحدود — fonts loaded from bytes, validated as the native path
//! validates, and chained.
//!
//! There is exactly one way in: [`TaaribKhatt::min_bayt`], taking a
//! `Uint8Array` the caller already holds. No path, no URL, no fetch. The
//! adapter reads the font out of the installed patch and hands the bytes
//! over, which keeps the font inside the patch's integrity seal, keeps this
//! module free of any network or filesystem dependency, and keeps loading
//! order in the one place that can see it — the adapter. The bytes are copied
//! once into WebAssembly linear memory and become the single shared buffer of
//! Decision 4: shaping, metrics and rasterization all read that one copy.
//!
//! Validation is [`MawridKhatt::jadeed`] — the same check, the same order,
//! the same rejection sentences as the native library, because a font that
//! the desktop build rejects and the NW.js build accepts would be a bug
//! report nobody could reproduce.

use std::sync::Arc;

use taarib_saff::khatt::{MawridKhatt, QiyasatKhatt, SilsilatKhutut};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::khata_js;

/// A font's own metrics, scaled to one pixel size.
///
/// Every value comes from the font — nothing here is a fraction of the
/// requested size or a constant. The RPG Maker plugin reads `irtifa_satr` to
/// answer `calcTextHeight` and `suud` to place the baseline inside the
/// engine's line rectangle; the Electron runtime reads the same two to build
/// the `TextMetrics` object `measureText` callers expect.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct TaaribQiyasat {
    /// Distance from the baseline to the top of the alignment box, in pixels.
    #[wasm_bindgen(readonly)]
    pub suud: f32,
    /// Distance from the baseline to the bottom, as a positive number.
    #[wasm_bindgen(readonly)]
    pub hubut: f32,
    /// The extra space the font recommends between lines.
    #[wasm_bindgen(readonly)]
    pub fajwa: f32,
    /// The line height the font recommends: ascent plus descent plus gap.
    #[wasm_bindgen(readonly)]
    pub irtifa_satr: f32,
    /// Height of a capital letter, in pixels.
    #[wasm_bindgen(readonly)]
    pub uluw_kabital: f32,
    /// Height of a lowercase `x` or its Arabic analogue, in pixels.
    #[wasm_bindgen(readonly)]
    pub uluw_saghir: f32,
    /// Design units per em — the scale everything above was derived at.
    #[wasm_bindgen(readonly)]
    pub wahdat: u16,
}

impl TaaribQiyasat {
    /// Wraps the engine's metrics value.
    pub(crate) const fn min_dakhili(qiyasat: QiyasatKhatt) -> Self {
        Self {
            suud: qiyasat.suud,
            hubut: qiyasat.hubut,
            fajwa: qiyasat.fajwa,
            irtifa_satr: qiyasat.irtifa_satr,
            uluw_kabital: qiyasat.uluw_kabital,
            uluw_saghir: qiyasat.uluw_saghir,
            wahdat: qiyasat.wahdat,
        }
    }
}

/// A loaded, validated font: one shared immutable byte buffer and its
/// identity.
///
/// Cheap to clone through [`TaaribKhatt::istinsakh`] — a clone is a reference
/// count on the one buffer, never a second copy of the bytes.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribKhatt {
    pub(crate) khatt: Arc<MawridKhatt>,
}

#[wasm_bindgen]
impl TaaribKhatt {
    /// Loads a font from bytes and validates it for Arabic.
    ///
    /// `bayt` is the complete font file — `ttf`, `otf`, `ttc` or `otc` — as
    /// the adapter read it out of the patch. `fahras` is the face index
    /// inside a collection, and `0` for an ordinary single-face file.
    ///
    /// The check is Decision 6, unchanged from the native path: `cmap`,
    /// `GSUB` and `GPOS` present; `init`, `medi`, `fina`, `isol` and `rlig`
    /// in `GSUB`; `mark` in `GPOS`; and coverage of the representative Arabic
    /// set. A font that fails does not draw ugly Arabic — it draws
    /// twenty-eight disconnected letterforms — so it is refused here, loudly,
    /// at the moment it is chosen.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error naming the real reason — the missing
    /// table, the missing feature, or how many required characters are absent
    /// and which comes first — with the permanent code, the English sentence
    /// and the next action as properties.
    pub fn min_bayt(bayt: Vec<u8>, fahras: u32) -> Result<Self, JsValue> {
        let mawrid = khata_js::min_natija(MawridKhatt::jadeed(Arc::new(bayt), fahras))?;
        Ok(Self {
            khatt: Arc::new(mawrid),
        })
    }

    /// Loads a font from bytes without the Arabic check.
    ///
    /// For the Latin and monospace faces at the end of a fallback chain — the
    /// digits, the identifiers, the embedded English a game is full of. It is
    /// not an escape hatch for an Arabic font that failed validation: nothing
    /// in the product hands a font loaded this way a single Arabic letter.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error when the bytes are not a font or the face
    /// index is past the end of a collection.
    pub fn min_bayt_latini(bayt: Vec<u8>, fahras: u32) -> Result<Self, JsValue> {
        let mawrid = khata_js::min_natija(MawridKhatt::jadeed_latini(Arc::new(bayt), fahras))?;
        Ok(Self {
            khatt: Arc::new(mawrid),
        })
    }

    /// The family name, preferring the typographic family the designer meant
    /// over the four-style legacy family.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn aila(&self) -> String {
        self.khatt.aila().to_owned()
    }

    /// The style name within the family — `Regular`, `Bold`, `Italic`.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn namat(&self) -> String {
        self.khatt.namat().to_owned()
    }

    /// This font's identity as lowercase hexadecimal — the value the atlas
    /// and the layout cache key on. Text rather than a number because it is
    /// sixty-four bits and a JavaScript number is not.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn huwiya(&self) -> String {
        self.khatt.huwiya().to_string()
    }

    /// Whether this font is variable.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn mutaghayyir(&self) -> bool {
        self.khatt.mutaghayyir()
    }

    /// The font's own metrics, scaled to `hajm` pixels.
    #[must_use]
    pub fn qiyasat(&self, hajm: f32) -> TaaribQiyasat {
        TaaribQiyasat::min_dakhili(self.khatt.qiyasat(hajm))
    }

    /// Whether this font covers the first character of `harf`.
    ///
    /// A coverage query, not a shaping shortcut: what gets drawn still comes
    /// out of `GSUB` during shaping. An empty string is covered by nothing.
    #[must_use]
    pub fn yughatti(&self, harf: &str) -> bool {
        harf.chars()
            .next()
            .is_some_and(|awwal| self.khatt.yughatti(awwal))
    }

    /// A second handle to the same font — a reference count, not a second
    /// copy of the bytes.
    ///
    /// Building a [`TaaribSilsila`] consumes the handles it is given, so a
    /// caller that also wants to keep querying a font directly hands the
    /// chain a clone and keeps the original.
    #[must_use]
    pub fn istinsakh(&self) -> Self {
        Self {
            khatt: Arc::clone(&self.khatt),
        }
    }
}

/// An ordered chain of fonts, tried in order per character.
///
/// A fallback *chain*, not a fallback shaper: when the primary font does not
/// cover a character, the next font that does takes the run, and every glyph
/// in the output names the chain index it came from so the atlas fetches the
/// right outline. The chain is what every layout call takes; it is the
/// JavaScript face of the engine's `SilsilatKhutut`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribSilsila {
    pub(crate) silsila: SilsilatKhutut,
}

#[wasm_bindgen]
impl TaaribSilsila {
    /// Builds a chain from fonts, first entry primary.
    ///
    /// The chain takes ownership of the handles it is given — after this
    /// call, the objects in `khutut` are consumed and using them throws.
    /// That is deliberate: the chain is the unit everything downstream works
    /// in, and a caller that also needs a direct handle keeps one via
    /// [`TaaribKhatt::istinsakh`] before building. Metrics remain reachable
    /// through the chain itself, so most callers need nothing else.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error for an empty chain — there would be
    /// nothing to draw with — or for one longer than 256 fonts, which no
    /// glyph could ever name.
    #[wasm_bindgen(constructor)]
    pub fn jadeeda(khutut: Vec<TaaribKhatt>) -> Result<Self, JsValue> {
        let dakhiliya = khutut.into_iter().map(|khatt| khatt.khatt).collect();
        let silsila = khata_js::min_natija(SilsilatKhutut::jadeeda(dakhiliya))?;
        Ok(Self { silsila })
    }

    /// How many fonts the chain holds.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad(&self) -> u32 {
        u32::try_from(self.silsila.adad()).unwrap_or(u32::MAX)
    }

    /// The primary font's metrics at `hajm` pixels — the metrics that set the
    /// line height for every layout made with this chain.
    #[must_use]
    pub fn qiyasat(&self, hajm: f32) -> TaaribQiyasat {
        TaaribQiyasat::min_dakhili(self.silsila.awwal().qiyasat(hajm))
    }

    /// The family name of the font at a chain index, or `undefined` past the
    /// end.
    #[must_use]
    pub fn aila(&self, fahras: u8) -> Option<String> {
        self.silsila
            .khatt(fahras)
            .map(|khatt| khatt.aila().to_owned())
    }
}
