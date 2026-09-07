//! النتيجة عبر الحدود — the finished layout, packed for a render loop.
//!
//! A dialogue line in RPG Maker is a hundred glyphs and the message window
//! redraws every frame. Crossing that as a hundred JavaScript objects per
//! frame is a hundred allocations the collector must eventually stop the
//! world for, and the stall lands in the middle of somebody's game. So the
//! layout crosses as **two typed arrays** — one `Float32Array` of fixed-width
//! glyph rows, one of fixed-width line rows — built in one pass and copied
//! across the boundary once.
//!
//! The row layouts are frozen and named. [`HaqlHarf`] and [`HaqlSatr`] are
//! the field indices, their `Adad` members are the strides, and the full
//! tables live in the crate documentation; an adapter writes
//! `sufuf[fahras * HaqlHarf.Adad + HaqlHarf.S]` and never counts offsets by
//! hand. The field order is the C ABI's `TaaribHarf` and `TaaribSatr`, field
//! for field, so the C# adapter and the TypeScript adapter read the same
//! shape.
//!
//! Every value is exact in an `f32`: glyph identifiers are 16-bit in
//! OpenType, span and chain indices are 16- and 8-bit, and byte offsets stay
//! exact below 2^24 — sixteen megabytes of text, far past any game string.
//! The arrays are copies, not views into WebAssembly memory: a view would be
//! silently detached the next time the module's memory grew, which is a
//! corruption the adapter could neither see nor reproduce, and one copy per
//! layout is the price of that never happening.

use js_sys::{Float32Array, Uint32Array};
use taarib_saff::nasq::{self, NassNaqi};
use taarib_saff::natija::{TakhtitNass, TaqreerTajawuz};
use taarib_saff::qiyas::QiyasNass;
use taarib_saff::talab::Ittijah;
use wasm_bindgen::prelude::wasm_bindgen;

/// Field indices into one glyph row of [`TaaribTakhtit::huruf`].
///
/// `Adad` is the stride: row `n` begins at `n * HaqlHarf.Adad`.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaqlHarf {
    /// The glyph identifier, in the font named by `Khatt`.
    Muarrif = 0,
    /// Byte offset into the logical text of this glyph's cluster.
    Anqud = 1,
    /// Horizontal position of the glyph's origin, in pixels from the
    /// layout's left edge — already in visual order.
    S = 2,
    /// Vertical position of the origin, in pixels down from the layout's
    /// top.
    A = 3,
    /// The advance this glyph contributed. Zero for marks.
    Taqaddum = 4,
    /// The style span this glyph inherited, so colour survives reordering.
    Nitaq = 5,
    /// Index into the font chain of the font this glyph belongs to.
    Khatt = 6,
    /// Flags — see [`AlamHarf`].
    Alam = 7,
    /// The stride: how many floats one glyph row occupies.
    Adad = 8,
}

/// Flags in a glyph row's [`HaqlHarf::Alam`] field.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlamHarf {
    /// This glyph is a combining mark, positioned onto a base rather than
    /// advancing the pen. It contributes nothing to width and nothing to a
    /// selection rectangle.
    Alama = 1,
}

/// Field indices into one line row of [`TaaribTakhtit::sutur`].
///
/// `Adad` is the stride: row `n` begins at `n * HaqlSatr.Adad`.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaqlSatr {
    /// Index of this line's first glyph row in the glyph array.
    AwwalHarf = 0,
    /// How many glyph rows this line has.
    AdadHuruf = 1,
    /// First byte of the logical text this line covers.
    BidayatMantiqi = 2,
    /// One past the last byte.
    NihayatMantiqi = 3,
    /// The baseline's vertical position, in pixels from the layout's top.
    Asas = 4,
    /// Where the line box begins horizontally, after alignment.
    Bidaya = 5,
    /// The measured width of the line's content.
    Ard = 6,
    /// The line box's height.
    Irtifa = 7,
    /// How far the tallest content rises above the baseline.
    Suud = 8,
    /// How far the deepest content falls below it.
    Hubut = 9,
    /// How much width justification added to this line.
    Dabt = 10,
    /// Flags — see [`AlamSatr`].
    Alam = 11,
    /// The stride: how many floats one line row occupies.
    Adad = 12,
}

/// Flags in a line row's [`HaqlSatr::Alam`] field.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlamSatr {
    /// The last line of its paragraph, which is what stops justification
    /// stretching a short final line across the full width.
    Akhir = 1,
    /// This line's base direction is right to left.
    Yameen = 2,
}

/// What an atom in a raw-markup layout stands for. Values returned by
/// [`TaaribTakhtitKhaam::dharra_naw`].
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawDharra {
    /// A format placeholder: `{0}`, `{name}`, `%s`, `%1$d`.
    Mawdi = 0,
    /// An inline image: `<sprite=3>`, `\I[12]`.
    Sura = 1,
    /// A runtime variable: `$gold`, `\V[7]`.
    Mutaghayyir = 2,
    /// A command the engine acts on rather than draws: `\n`, `\C[2]`. It may
    /// occupy width but nothing is ever drawn in it.
    Amr = 3,
}

/// What overflowed, by how much, and where.
///
/// Not an error — a measurement. The adapter decides what to do with it: log
/// it, shrink its window, or hand it to the diagnostics the patch compiler
/// reads.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct TaaribTajawuz {
    taqreer: TaqreerTajawuz,
}

#[wasm_bindgen]
impl TaaribTajawuz {
    /// The widest line's measured width, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ard(&self) -> f32 {
        self.taqreer.ard
    }

    /// The width that was available.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ard_mutah(&self) -> f32 {
        self.taqreer.ard_mutah
    }

    /// The total height the text needed.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn irtifa(&self) -> f32 {
        self.taqreer.irtifa
    }

    /// The height that was available, or `undefined` when none was given.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn irtifa_mutah(&self) -> Option<f32> {
        self.taqreer.irtifa_mutah
    }

    /// The first line that exceeded the width.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn awwal_satr(&self) -> u32 {
        self.taqreer.awwal_satr
    }

    /// How many lines exceeded it.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_sutur(&self) -> u32 {
        self.taqreer.adad_sutur
    }

    /// How far past the available width the worst line went, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn zaid(&self) -> f32 {
        self.taqreer.zaid()
    }

    /// The overflow as a fraction of the available width — what a severity
    /// sort orders by, because forty pixels over a button is catastrophic and
    /// forty pixels over a subtitle is invisible.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn nisba(&self) -> f32 {
        self.taqreer.nisba()
    }

    /// Whether the text also ran past the height it was given.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn tajawuz_irtifa(&self) -> bool {
        self.taqreer.tajawuz_irtifa()
    }
}

/// Text measured without positioning a single glyph — the same pipeline, the
/// same policies, none of the output.
///
/// What `textWidth` and `measureText` return through. Every value comes from
/// real shaped advances, never from a character count or an average width.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct TaaribQiyas {
    /// The width of the widest line, in pixels.
    #[wasm_bindgen(readonly)]
    pub ard: f32,
    /// The height of every line box together, in pixels.
    #[wasm_bindgen(readonly)]
    pub irtifa: f32,
    /// How far the first line's tallest content rises above its baseline.
    #[wasm_bindgen(readonly)]
    pub suud: f32,
    /// How far the last line's deepest content falls below its baseline.
    #[wasm_bindgen(readonly)]
    pub hubut: f32,
    /// How many lines the text needed.
    #[wasm_bindgen(readonly)]
    pub adad_sutur: u32,
}

impl TaaribQiyas {
    /// Wraps the engine's measurement.
    pub(crate) const fn min_dakhili(qiyas: QiyasNass) -> Self {
        Self {
            ard: qiyas.ard,
            irtifa: qiyas.irtifa,
            suud: qiyas.suud,
            hubut: qiyas.hubut,
            adad_sutur: qiyas.adad_sutur,
        }
    }
}

/// A finished layout: ordered visual lines of positioned glyphs.
///
/// The glyph and line data cross as packed typed arrays — see the crate
/// documentation for the row tables — and everything else is a scalar
/// getter. Free it when its frame is done; the typed arrays already handed
/// out stay valid, because they are copies owned by JavaScript.
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribTakhtit {
    pub(crate) takhtit: TakhtitNass,
}

#[wasm_bindgen]
impl TaaribTakhtit {
    /// How many glyphs were laid out.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_huruf(&self) -> u32 {
        adad(self.takhtit.huruf.len())
    }

    /// How many lines they occupy.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_sutur(&self) -> u32 {
        adad(self.takhtit.sutur.len())
    }

    /// The glyph rows: one <code>[HaqlHarf].Adad</code>-float row per glyph, in
    /// visual order, grouped by line. A fresh copy per read — read it once
    /// per layout and index into it.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn huruf(&self) -> Float32Array {
        sufuf_huruf(&self.takhtit)
    }

    /// The line rows: one <code>[HaqlSatr].Adad</code>-float row per line, in reading
    /// order from the top. A fresh copy per read.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn sutur(&self) -> Float32Array {
        sufuf_sutur(&self.takhtit)
    }

    /// The width of the widest line, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ard(&self) -> f32 {
        self.takhtit.ard
    }

    /// The total height of every line box, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn irtifa(&self) -> f32 {
        self.takhtit.irtifa
    }

    /// The size the text was finally laid out at — differs from the size
    /// requested when the overflow policy shrank it to fit, and is the size
    /// to ask the atlas for.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hajm(&self) -> f32 {
        self.takhtit.hajm
    }

    /// Whether the paragraph's resolved base direction is right to left.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn yameen(&self) -> bool {
        self.takhtit.ittijah == Ittijah::Yameen
    }

    /// Whether the overflow policy truncated the text.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn maqsus(&self) -> bool {
        self.takhtit.maqsus
    }

    /// Present when the text did not fit, `undefined` when it did.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn tajawuz(&self) -> Option<TaaribTajawuz> {
        self.takhtit
            .tajawuz
            .map(|taqreer| TaaribTajawuz { taqreer })
    }

    /// Every distinct `(font index, glyph identifier)` this layout draws, as
    /// a `Uint32Array` of pairs — element `2n` is the chain index, element
    /// `2n + 1` the glyph identifier.
    ///
    /// This is how an adapter warms the atlas before its first frame: from
    /// what shaping actually produced, never from a guessed character range.
    #[must_use]
    pub fn ashkal(&self) -> Uint32Array {
        ashkal_dakhili(&self.takhtit)
    }
}

/// A layout made from text that still carried its markup, plus what the
/// markup became.
///
/// Everything [`TaaribTakhtit`] exposes is exposed here identically — the
/// same packed rows, the same scalars — plus the clean text the cluster
/// indices refer to and the atom table the escape codes were lifted into.
/// The RPG Maker plugin walks the atoms to reinsert `\I[n]` icons and apply
/// `\C[n]` colours: each atom names the span that carries it, and the glyph
/// rows name their span in [`HaqlHarf::Nitaq`].
#[wasm_bindgen]
#[derive(Debug)]
pub struct TaaribTakhtitKhaam {
    pub(crate) naqi: NassNaqi,
    pub(crate) takhtit: TakhtitNass,
}

#[wasm_bindgen]
impl TaaribTakhtitKhaam {
    /// The clean text — markup and placeholders removed, one object
    /// replacement character per atom. This is what every [`HaqlHarf::Anqud`]
    /// byte offset refers to. A fresh copy per read.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn nass(&self) -> String {
        self.naqi.nass.clone()
    }

    /// How many glyphs were laid out.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_huruf(&self) -> u32 {
        adad(self.takhtit.huruf.len())
    }

    /// How many lines they occupy.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_sutur(&self) -> u32 {
        adad(self.takhtit.sutur.len())
    }

    /// The glyph rows, exactly as [`TaaribTakhtit::huruf`].
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn huruf(&self) -> Float32Array {
        sufuf_huruf(&self.takhtit)
    }

    /// The line rows, exactly as [`TaaribTakhtit::sutur`].
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn sutur(&self) -> Float32Array {
        sufuf_sutur(&self.takhtit)
    }

    /// The width of the widest line, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn ard(&self) -> f32 {
        self.takhtit.ard
    }

    /// The total height of every line box, in pixels.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn irtifa(&self) -> f32 {
        self.takhtit.irtifa
    }

    /// The size the text was finally laid out at.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn hajm(&self) -> f32 {
        self.takhtit.hajm
    }

    /// Whether the paragraph's resolved base direction is right to left.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn yameen(&self) -> bool {
        self.takhtit.ittijah == Ittijah::Yameen
    }

    /// Whether the overflow policy truncated the text.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn maqsus(&self) -> bool {
        self.takhtit.maqsus
    }

    /// Present when the text did not fit, `undefined` when it did.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn tajawuz(&self) -> Option<TaaribTajawuz> {
        self.takhtit
            .tajawuz
            .map(|taqreer| TaaribTajawuz { taqreer })
    }

    /// Every distinct `(font index, glyph identifier)` this layout draws, as
    /// [`TaaribTakhtit::ashkal`] returns it.
    #[must_use]
    pub fn ashkal(&self) -> Uint32Array {
        ashkal_dakhili(&self.takhtit)
    }

    /// How many atoms the markup was lifted into.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn adad_dharrat(&self) -> u32 {
        adad(self.naqi.dharrat.len())
    }

    /// The atom's raw source exactly as it appeared — `\I[12]`, `{0}` — or
    /// `undefined` past the end. This is the string an adapter matches its
    /// own escape-code handling against.
    #[must_use]
    pub fn dharra_khaam(&self, fahras: u32) -> Option<String> {
        self.naqi
            .dharrat
            .get(qusize(fahras))
            .map(|dharra| dharra.khaam.clone())
    }

    /// The identifier of the span that carries the atom, or `undefined` past
    /// the end. Glyph rows whose [`HaqlHarf::Nitaq`] equals this belong to
    /// the atom.
    #[must_use]
    pub fn dharra_nitaq(&self, fahras: u32) -> Option<u16> {
        self.naqi
            .dharrat
            .get(qusize(fahras))
            .map(|dharra| dharra.nitaq)
    }

    /// What the atom stands for, as a [`NawDharra`] value, or `undefined`
    /// past the end.
    #[must_use]
    pub fn dharra_naw(&self, fahras: u32) -> Option<u32> {
        self.naqi
            .dharrat
            .get(qusize(fahras))
            .map(|dharra| match dharra.naw {
                nasq::NawDharra::Mawdi => 0,
                nasq::NawDharra::Sura => 1,
                nasq::NawDharra::Mutaghayyir => 2,
                nasq::NawDharra::Amr => 3,
            })
    }

    /// The positional index the dialect wrote — the `3` of `\I[3]`, the `0`
    /// of `{0}` — or `undefined` when it wrote none or `fahras` is past the
    /// end. Kept exactly as written, never renumbered between dialects.
    #[must_use]
    pub fn dharra_tarteeb(&self, fahras: u32) -> Option<u32> {
        self.naqi
            .dharrat
            .get(qusize(fahras))
            .and_then(|dharra| dharra.tarteeb)
    }
}

// ---------------------------------------------------------------------------
// Packing
// ---------------------------------------------------------------------------

/// The glyph rows, packed.
///
/// One pass, one allocation on the Rust side, one copy into a JS-owned
/// array. The row layout is [`HaqlHarf`] and must never be reordered — the
/// indices are published to every adapter.
#[expect(
    clippy::cast_precision_loss,
    reason = "glyph identifiers are 16-bit in OpenType and cluster byte offsets are exact in an \
              f32 below 2^24 — sixteen megabytes of text, far past any game string; the row \
              format documents both bounds"
)]
pub(crate) fn sufuf_huruf(takhtit: &TakhtitNass) -> Float32Array {
    let mut mabni: Vec<f32> = Vec::with_capacity(
        takhtit
            .huruf
            .len()
            .saturating_mul(qusize(HaqlHarf::Adad as u32)),
    );
    for harf in &takhtit.huruf {
        let alam: u32 = if harf.alama {
            AlamHarf::Alama as u32
        } else {
            0
        };
        mabni.push(harf.muarrif as f32);
        mabni.push(harf.anqud as f32);
        mabni.push(harf.s);
        mabni.push(harf.a);
        mabni.push(harf.taqaddum);
        mabni.push(f32::from(harf.nitaq));
        mabni.push(f32::from(harf.khatt));
        mabni.push(alam as f32);
    }
    Float32Array::from(mabni.as_slice())
}

/// The line rows, packed. The row layout is [`HaqlSatr`] and must never be
/// reordered.
#[expect(
    clippy::cast_precision_loss,
    reason = "glyph indices and logical byte offsets are exact in an f32 below 2^24, and no \
              single layout approaches sixteen million glyphs or sixteen megabytes of text"
)]
pub(crate) fn sufuf_sutur(takhtit: &TakhtitNass) -> Float32Array {
    let mut mabni: Vec<f32> = Vec::with_capacity(
        takhtit
            .sutur
            .len()
            .saturating_mul(qusize(HaqlSatr::Adad as u32)),
    );
    for satr in &takhtit.sutur {
        let mut alam: u32 = 0;
        if satr.akhir {
            alam |= AlamSatr::Akhir as u32;
        }
        if satr.ittijah == Ittijah::Yameen {
            alam |= AlamSatr::Yameen as u32;
        }
        mabni.push(satr.huruf.start as f32);
        mabni.push((satr.huruf.end.saturating_sub(satr.huruf.start)) as f32);
        mabni.push(satr.mantiqi.start as f32);
        mabni.push(satr.mantiqi.end as f32);
        mabni.push(satr.asas);
        mabni.push(satr.bidaya);
        mabni.push(satr.ard);
        mabni.push(satr.irtifa);
        mabni.push(satr.suud);
        mabni.push(satr.hubut);
        mabni.push(satr.dabt);
        mabni.push(alam as f32);
    }
    Float32Array::from(mabni.as_slice())
}

/// The distinct glyph set, as flat `(font index, glyph identifier)` pairs.
fn ashkal_dakhili(takhtit: &TakhtitNass) -> Uint32Array {
    let ashkal = takhtit.ashkal();
    let mut mabni: Vec<u32> = Vec::with_capacity(ashkal.len().saturating_mul(2));
    for (khatt, muarrif) in ashkal {
        mabni.push(u32::from(khatt));
        mabni.push(muarrif);
    }
    Uint32Array::from(mabni.as_slice())
}

/// A count, saturated into the `u32` the getters return.
fn adad(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// A `u32` index as the `usize` a slice wants, saturated — which turns an
/// impossible index into a miss rather than a wrap.
fn qusize(qeema: u32) -> usize {
    usize::try_from(qeema).unwrap_or(usize::MAX)
}
