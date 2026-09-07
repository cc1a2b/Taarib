//! اللوحة عبر الحدود — the runtime atlas, inside a JavaScript game.
//!
//! The patch compiler saw every string it was given; it did not see the
//! player's name or the number that reached five digits for the first time.
//! [`TaaribLawha`] is the atlas that rasterizes what it is asked for, packs
//! it, and — inside a byte budget — forgets what nobody has drawn recently.
//! It is `taarib-lawha`'s `LawhaHayya`, unchanged: the same shelf packing,
//! the same rectangle eviction, the same per-frame pinning that makes
//! eviction safe.
//!
//! The contract that pinning imposes crosses the boundary intact: **call
//! [`TaaribLawha::ibda_itar`] once per frame.** Every position
//! [`TaaribLawha::shakl`] returns is pinned until the next `ibda_itar`, so a
//! rectangle referenced by the frame being drawn can never be reassigned
//! underneath it; an adapter that never begins a frame will instead fill the
//! atlas with unevictable glyphs and get a loud, specific error — which is a
//! defect somebody can fix, where one wrong letter on screen for one frame is
//! not.
//!
//! Pages cross as plain `Uint8Array` copies of single-channel R8 texels, one
//! byte per texel, row-major, no row padding — exactly what
//! `putImageData` (expanded to RGBA by the adapter) or
//! `texSubImage2D(..., LUMINANCE, UNSIGNED_BYTE, ...)` wants. A copy rather
//! than a view into WebAssembly memory, deliberately: the module's memory
//! can grow on any allocation, growth detaches every outstanding view, and a
//! detached texture upload is a corruption no adapter could reproduce. The
//! adapter re-reads a page only when the atlas has changed, so the copy is
//! paid per growth event, not per frame.

use js_sys::Uint8Array;
use taarib_lawha::khareeta::MawdiShakl;
use taarib_lawha::namu::IhsaatNamu;
use taarib_lawha::{KhiyaratMisafa, KhiyaratRasf, LawhaHayya, MiftahShakl, NamatSafha};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::khata_js;
use crate::khatt_js::TaaribSilsila;

/// The default byte budget: thirty-two mebibytes, which is eight pages of
/// the conservative 2048-square profile.
///
/// Far past what a patched RPG Maker interface needs, and low enough that a
/// runaway glyph set is reported rather than quietly taking a game's memory.
const MIZANIYA_IFTIRADIYA: usize = 32 * 1024 * 1024;

/// How atlas pages are rasterized. Values for
/// [`TaaribKhiyaratLawha::namat`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamatLawha {
    /// Eight-bit antialiased coverage — sharper at fixed small sizes, the
    /// default for interface text.
    Taghtiya = 0,
    /// A signed distance field — one page serves every size, which is what
    /// makes auto-sizing text work.
    Masafa = 1,
}

/// How the atlas is sized, padded, budgeted and rasterized.
///
/// The defaults are the conservative profile — 2048-square pages, rounded to
/// a power of two, one-pixel gutters, coverage rasterization, a
/// thirty-two-mebibyte budget — because this module's oldest host is NW.js
/// 0.29 under RPG Maker MV, whose GL path still wants power-of-two textures
/// and modest dimensions. A patch that declares otherwise assigns what it
/// declared; the adapter obeys the patch without asking.
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct TaaribKhiyaratLawha {
    rasf: KhiyaratRasf,
    namat: NamatSafha,
    misafa: KhiyaratMisafa,
    mizaniyat_bayt: usize,
}

impl Default for TaaribKhiyaratLawha {
    fn default() -> Self {
        Self {
            rasf: KhiyaratRasf::muhafiz(),
            namat: NamatSafha::Taghtiya,
            misafa: KhiyaratMisafa::default(),
            mizaniyat_bayt: MIZANIYA_IFTIRADIYA,
        }
    }
}

#[wasm_bindgen]
impl TaaribKhiyaratLawha {
    /// The conservative defaults described on the type.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// The widest page the atlas may create, in texels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn aqsa_ard(&self) -> u16 {
        self.rasf.aqsa_ard
    }

    /// Sets the widest page. Never exceeded — the packer opens another page
    /// instead of scaling a glyph down.
    #[wasm_bindgen(setter)]
    pub fn set_aqsa_ard(&mut self, aqsa_ard: u16) {
        self.rasf.aqsa_ard = aqsa_ard;
    }

    /// The tallest page the atlas may create, in texels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn aqsa_irtifa(&self) -> u16 {
        self.rasf.aqsa_irtifa
    }

    /// Sets the tallest page.
    #[wasm_bindgen(setter)]
    pub fn set_aqsa_irtifa(&mut self, aqsa_irtifa: u16) {
        self.rasf.aqsa_irtifa = aqsa_irtifa;
    }

    /// The gutter around every glyph, in pixels, on all four sides — what a
    /// bilinear tap at a rectangle's edge is stopped from reaching a
    /// neighbour by.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hashw(&self) -> u16 {
        self.rasf.hashw
    }

    /// Sets the gutter.
    #[wasm_bindgen(setter)]
    pub fn set_hashw(&mut self, hashw: u16) {
        self.rasf.hashw = hashw;
    }

    /// Whether page dimensions are reduced to a power of two, for the GL
    /// paths that still require it.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn quwwat_ithnayn(&self) -> bool {
        self.rasf.quwwat_ithnayn
    }

    /// Sets the power-of-two rule.
    #[wasm_bindgen(setter)]
    pub fn set_quwwat_ithnayn(&mut self, quwwat_ithnayn: bool) {
        self.rasf.quwwat_ithnayn = quwwat_ithnayn;
    }

    /// How many pages the atlas may open. The byte budget still rules: the
    /// page count actually used is the smaller of this and what the budget
    /// pays for.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn aqsa_safahat(&self) -> u16 {
        self.rasf.aqsa_safahat
    }

    /// Sets the page limit.
    #[wasm_bindgen(setter)]
    pub fn set_aqsa_safahat(&mut self, aqsa_safahat: u16) {
        self.rasf.aqsa_safahat = aqsa_safahat;
    }

    /// How glyphs are rasterized — a [`NamatLawha`] value.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn namat(&self) -> u32 {
        u32::from(self.namat.bayt())
    }

    /// Sets the rasterization mode. The patch records this choice; the
    /// adapter assigns what the patch declared, because an atlas built in a
    /// different mode from the one the compiler measured makes every
    /// overflow report in the patch a lie. Unrecognised numbers select
    /// coverage.
    #[wasm_bindgen(setter)]
    pub fn set_namat(&mut self, namat: u32) {
        self.namat = NamatSafha::min_bayt(u8::try_from(namat).unwrap_or(u8::MAX));
    }

    /// The distance-field spread, in cell pixels. Ignored entirely for a
    /// coverage atlas.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn intishar(&self) -> f32 {
        self.misafa.intishar
    }

    /// Sets the distance-field spread.
    #[wasm_bindgen(setter)]
    pub fn set_intishar(&mut self, intishar: f32) {
        self.misafa.intishar = intishar;
    }

    /// The byte budget the atlas lives inside.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn mizaniyat_bayt(&self) -> u32 {
        u32::try_from(self.mizaniyat_bayt).unwrap_or(u32::MAX)
    }

    /// Sets the byte budget. In bytes — not pages and not entries — because
    /// the number a game's memory plan is written in is bytes. Must pay for
    /// at least one page, or building the atlas is refused with the reason.
    #[wasm_bindgen(setter)]
    pub fn set_mizaniyat_bayt(&mut self, mizaniyat_bayt: u32) {
        self.mizaniyat_bayt = usize::try_from(mizaniyat_bayt).unwrap_or(usize::MAX);
    }
}

/// Where one glyph lives in the atlas, and how to draw it.
///
/// The blit an adapter performs is: destination x = pen x + `izaha_s`,
/// destination y = baseline y − `izaha_a`, source rectangle
/// (`s`, `a`, `ard`, `irtifa`) of page `safha`, then advance the pen by
/// `taqaddum`. A glyph with no image — a space, a joiner — has zero `ard`
/// and `irtifa` and still carries its advance, so the adapter draws nothing
/// and advances anyway, with no special case.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct TaaribMawdi {
    /// The glyph's advance at this size, in pixels — carried here so an
    /// adapter drawing from the atlas alone never reaches back into the
    /// font.
    #[wasm_bindgen(readonly)]
    pub taqaddum: f32,
    /// Which page.
    #[wasm_bindgen(readonly)]
    pub safha: u16,
    /// Left edge in the page, in texels.
    #[wasm_bindgen(readonly)]
    pub s: u16,
    /// Top edge in the page, in texels.
    #[wasm_bindgen(readonly)]
    pub a: u16,
    /// Width in texels. Zero for a glyph with no image.
    #[wasm_bindgen(readonly)]
    pub ard: u16,
    /// Height in texels. Zero for a glyph with no image.
    #[wasm_bindgen(readonly)]
    pub irtifa: u16,
    /// Left bearing: how far right of the pen position the image starts.
    #[wasm_bindgen(readonly)]
    pub izaha_s: i16,
    /// Top bearing: how far above the baseline the image's top edge sits.
    #[wasm_bindgen(readonly)]
    pub izaha_a: i16,
}

#[wasm_bindgen]
impl TaaribMawdi {
    /// Whether this glyph has no image at all — the correct and common
    /// answer for a space.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn khali(&self) -> bool {
        self.ard == 0 || self.irtifa == 0
    }
}

impl TaaribMawdi {
    /// Wraps the atlas's own position value.
    pub(crate) const fn min_dakhili(mawdi: MawdiShakl) -> Self {
        Self {
            taqaddum: mawdi.taqaddum,
            safha: mawdi.safha,
            s: mawdi.s,
            a: mawdi.a,
            ard: mawdi.ard,
            irtifa: mawdi.irtifa,
            izaha_s: mawdi.izaha_s,
            izaha_a: mawdi.izaha_a,
        }
    }
}

/// What the atlas has been doing — the counters the Diagnostics screen
/// reads.
///
/// A miss count that keeps climbing after the first minutes of play means
/// the patch compiler missed strings, and an eviction count that is high
/// relative to the miss count means the budget is too small for the working
/// set. The large counters cross as `f64`, exact to 2^53 — past any count a
/// running game reaches.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct TaaribIhsaat {
    /// Glyphs served from the atlas without rasterizing anything.
    #[wasm_bindgen(readonly)]
    pub isabat: f64,
    /// Glyphs that had to be rasterized and packed.
    #[wasm_bindgen(readonly)]
    pub ikhfaqat: f64,
    /// Rectangles reclaimed to make room.
    #[wasm_bindgen(readonly)]
    pub ikhlaat: f64,
    /// Pages opened after the first.
    #[wasm_bindgen(readonly)]
    pub ahdath_namu: f64,
    /// What the pages currently cost, in bytes.
    #[wasm_bindgen(readonly)]
    pub bayt: f64,
    /// The byte budget this atlas was built with.
    #[wasm_bindgen(readonly)]
    pub mizaniya: f64,
    /// How many glyphs are mapped.
    #[wasm_bindgen(readonly)]
    pub ashkal: u32,
    /// How many glyphs the frame being drawn has referenced.
    #[wasm_bindgen(readonly)]
    pub mathbut: u32,
    /// How many pages are open.
    #[wasm_bindgen(readonly)]
    pub safahat: u16,
    /// The share of requests served without rasterizing — the one number
    /// worth putting on a screen. A healthy patch settles above 0.99 within
    /// seconds of a menu opening.
    #[wasm_bindgen(readonly)]
    pub nisbat_isaba: f32,
}

impl TaaribIhsaat {
    /// Wraps a counters snapshot.
    #[expect(
        clippy::cast_precision_loss,
        reason = "these counters are diagnostics; one large enough to lose precision in an f64 \
                  has long since made its exact value irrelevant"
    )]
    pub(crate) fn min_dakhili(ihsaat: IhsaatNamu) -> Self {
        Self {
            isabat: ihsaat.isabat as f64,
            ikhfaqat: ihsaat.ikhfaqat as f64,
            ikhlaat: ihsaat.ikhlaat as f64,
            ahdath_namu: ihsaat.ahdath_namu as f64,
            bayt: ihsaat.bayt as f64,
            mizaniya: ihsaat.mizaniya as f64,
            ashkal: ihsaat.ashkal,
            mathbut: ihsaat.mathbut,
            safahat: ihsaat.safahat,
            nisbat_isaba: ihsaat.nisbat_isaba(),
        }
    }
}

/// The runtime atlas: rasterize on miss, pack, pin per frame, evict inside a
/// byte budget.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribLawha {
    lawha: LawhaHayya,
    namat: NamatSafha,
}

#[wasm_bindgen]
impl TaaribLawha {
    /// Builds an atlas inside the options' byte budget, opening the first
    /// page immediately.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error for page dimensions the packer cannot
    /// use, or a budget that will not pay for a single page — an atlas with
    /// no page cannot hold a glyph, and finding that out here is better than
    /// finding it out per glyph, forever.
    #[wasm_bindgen(constructor)]
    pub fn jadeeda(khiyarat: &TaaribKhiyaratLawha) -> Result<Self, JsValue> {
        let lawha = khata_js::min_natija(LawhaHayya::jadeeda_bi_misafa(
            khiyarat.rasf,
            khiyarat.namat,
            khiyarat.mizaniyat_bayt,
            khiyarat.misafa,
        ))?;
        Ok(Self {
            lawha,
            namat: khiyarat.namat,
        })
    }

    /// Begins a frame: releases every pin the last frame took.
    ///
    /// Call once per frame, before the first [`TaaribLawha::shakl`] of the
    /// frame. Skipping it does not corrupt anything — it fills the atlas
    /// with unevictable glyphs until requests fail loudly, which is the
    /// diagnosable version of the failure.
    pub fn ibda_itar(&mut self) {
        self.lawha.ibda_itar();
    }

    /// Where a glyph is, rasterizing and packing it if it is not there yet —
    /// and pinning it for the rest of the frame.
    ///
    /// `khatt` and `muarrif` name the glyph as the layout rows name it:
    /// the chain index from [`crate::natija_js::HaqlHarf::Khatt`] and the
    /// identifier from [`crate::natija_js::HaqlHarf::Muarrif`], resolved
    /// against the same chain the layout was made with. `hajm` is the
    /// layout's final size and `bakat` the subpixel bucket from
    /// [`crate::bakat_tahazzuz`] — zero for an adapter that draws on whole
    /// pixels.
    ///
    /// On a hit this is a map lookup; only a miss rasterizes. When a miss
    /// grows or changes a page, re-upload the affected page —
    /// [`TaaribLawha::adad_safahat`] and the counters say when.
    ///
    /// # Errors
    ///
    /// Throws a `TaaribKhata` error when the key names a font past the end
    /// of the chain — an atlas and a chain from different patches — when the
    /// rasterizer cannot draw the glyph, when the glyph is larger than a
    /// page, or when the atlas is full and everything in it is pinned by the
    /// frame being drawn.
    pub fn shakl(
        &mut self,
        silsila: &TaaribSilsila,
        khatt: u8,
        muarrif: u32,
        hajm: f32,
        bakat: u8,
    ) -> Result<TaaribMawdi, JsValue> {
        let miftah = MiftahShakl::jadeed(khatt, muarrif, hajm, self.namat, bakat);
        let mawdi = khata_js::min_natija(self.lawha.shakl_min_silsila(miftah, &silsila.silsila))?;
        Ok(TaaribMawdi::min_dakhili(mawdi))
    }

    /// Pins a glyph for the rest of the frame without asking for it.
    ///
    /// [`TaaribLawha::shakl`] already pins everything it returns; this is
    /// for an adapter re-emitting a draw list it built on an earlier frame
    /// from positions it cached. Pinning a glyph the atlas does not hold is
    /// allowed and costs nothing.
    pub fn ithbit(&mut self, khatt: u8, muarrif: u32, hajm: f32, bakat: u8) {
        self.lawha
            .ithbit(MiftahShakl::jadeed(khatt, muarrif, hajm, self.namat, bakat));
    }

    /// Releases one pin early, for a caller that finished with a glyph
    /// mid-frame and would rather let a large batch behind it succeed.
    pub fn atliq(&mut self, khatt: u8, muarrif: u32, hajm: f32, bakat: u8) {
        self.lawha
            .atliq(MiftahShakl::jadeed(khatt, muarrif, hajm, self.namat, bakat));
    }

    /// One page's texels, as a fresh copy — one byte per texel, row-major
    /// from the top, no row padding — or `undefined` past the last page.
    ///
    /// Upload it and drop it; re-read a page only when the atlas has
    /// changed, which the counters report.
    #[must_use]
    pub fn safha(&self, fahras: u16) -> Option<Uint8Array> {
        self.lawha
            .safahat()
            .get(usize::from(fahras))
            .map(|safha| Uint8Array::from(safha.bayt.as_slice()))
    }

    /// A page's width in texels, or `undefined` past the last page.
    #[must_use]
    pub fn ard_safha(&self, fahras: u16) -> Option<u16> {
        self.lawha
            .safahat()
            .get(usize::from(fahras))
            .map(|safha| safha.ard)
    }

    /// A page's height in texels, or `undefined` past the last page.
    #[must_use]
    pub fn irtifa_safha(&self, fahras: u16) -> Option<u16> {
        self.lawha
            .safahat()
            .get(usize::from(fahras))
            .map(|safha| safha.irtifa)
    }

    /// How many pages are open.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_safahat(&self) -> u32 {
        u32::try_from(self.lawha.safahat().len()).unwrap_or(u32::MAX)
    }

    /// How this atlas's pages are rasterized — a [`NamatLawha`] value.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn namat(&self) -> u32 {
        u32::from(self.namat.bayt())
    }

    /// The counters, as a snapshot.
    #[must_use]
    pub fn ihsaat(&self) -> TaaribIhsaat {
        TaaribIhsaat::min_dakhili(self.lawha.ihsaat())
    }

    /// Empties the atlas, keeping one page and every lifetime counter — for
    /// a level transition, where the working set is about to be replaced
    /// wholesale. Every page the adapter uploaded is stale after this;
    /// re-upload before the next draw.
    pub fn amsah(&mut self) {
        self.lawha.amsah();
    }
}
