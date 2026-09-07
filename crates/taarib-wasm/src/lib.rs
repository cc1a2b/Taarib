//! # نواة تعريب لويب أسمبلي — the engine, compiled for JavaScript
//!
//! `taarib_core.wasm` plus its generated TypeScript wrapper. This is the same
//! `saff` and the same `lawha`, compiled to `wasm32-unknown-unknown` and
//! exposed through `wasm-bindgen` instead of through C. It exists because two
//! of the engines in the coverage matrix — RPG Maker MV/MZ and Electron — run
//! their text drawing inside a JavaScript runtime that cannot load a native
//! library at all, and shipping a second Arabic implementation in JavaScript
//! for them was never an option: a second shaper is a second set of bugs, a
//! second thing to keep correct, and a guarantee that the two paths drift.
//!
//! Produced in **Phase 3** alongside the C ABI and consumed in **Phase 10** by
//! the script adapters. It is the reason Decision 2 chose a pure-Rust shaper:
//! HarfRust compiles here unchanged, and so the browser path and the native
//! path are the same code producing the same glyphs.
//!
//! ## The two consumers, and what each one needs
//!
//! **The RPG Maker MV/MZ plugin** (`js/plugins/taarib.js`). It overrides
//! `Bitmap.prototype.drawText` and the `Window_Base` text pipeline —
//! `processCharacter`, `drawTextEx`, `calcTextHeight`, `textWidth` — so every
//! string the engine draws is laid out here instead of by the browser. It
//! needs [`saff_js::TaaribSaff::khattit_khaam`], because RPG Maker text still
//! carries its escape codes (`\C[n]`, `\I[n]`, `\V[n]`, `\G`) and those must
//! become `nasq` spans rather than glyphs; it needs
//! [`saff_js::TaaribSaff::qis`] for `textWidth` and `calcTextHeight`, which
//! the engine calls before it ever draws; and it blits every glyph from
//! [`lawha_js::TaaribLawha`]'s pages onto the window's bitmap, never calling
//! the canvas's own text API. The message window redraws every frame, which is
//! why the glyph output crosses as one packed [`js_sys::Float32Array`] and not
//! as objects.
//!
//! **The Electron runtime** (`adapters-script/electron/`). For canvas-rendered
//! web games — where the browser's own shaping never runs, because the game
//! draws text with `CanvasRenderingContext2D.fillText` — the runtime replaces
//! `fillText` and `measureText`: `measureText` becomes
//! [`saff_js::TaaribSaff::qis`], `fillText` becomes
//! [`saff_js::TaaribSaff::khattit`] plus one `drawImage`/`putImageData` blit
//! per glyph from the atlas page. It needs the same measurement, the same
//! layout, and the same atlas; the only difference from the RPG Maker plugin
//! is who owns the destination canvas.
//!
//! Both consumers draw glyph images from the atlas rather than through the
//! browser's text stack, because the browser's stack is exactly what these
//! games bypassed — and because only the atlas draws the glyphs shaping chose,
//! ligatures and contextual forms included, rather than whatever a codepoint
//! maps to.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `khatt_js` | font registration from bytes the caller already holds, validated exactly as the native path validates and rejected with the same reasons; the font chain; metrics |
//! | `talab_js` | the layout request: text, chain, size, the style spans, and the recorded decisions, with the same discriminant numbering the C ABI uses |
//! | `natija_js` | the finished layout as packed typed arrays with named field indices, the measurement result, and the overflow report |
//! | `saff_js` | the engine itself: layout, raw-markup layout, and measurement |
//! | `lawha_js` | the runtime atlas: rasterize-on-miss, per-frame pinning, growth inside a byte budget, and pages handed out ready for `texSubImage2D` or `putImageData` |
//! | `khata_js` | errors surfaced as JavaScript exceptions that carry the permanent code, both sentences and the next action — never a bare string |
//!
//! ## The packed glyph array
//!
//! A dialogue line in RPG Maker is a hundred glyphs, and the message window
//! redraws every frame. A hundred JavaScript objects per frame is a
//! garbage-collection pause the player sees, so glyphs cross the boundary as
//! **one `Float32Array`**, one fixed-stride row per glyph, in visual order.
//! The stride is [`natija_js::HaqlHarf::Adad`] and every field has a named
//! index in [`natija_js::HaqlHarf`], so no adapter ever counts offsets by
//! hand. The row mirrors the C ABI's `TaaribHarf`, field for field:
//!
//! | index | name | meaning |
//! | --- | --- | --- |
//! | 0 | `Muarrif` | glyph identifier, in the font named by `Khatt` |
//! | 1 | `Anqud` | byte offset into the logical text of this glyph's cluster |
//! | 2 | `S` | x of the glyph origin, pixels from the layout's left edge, already in visual order |
//! | 3 | `A` | y of the glyph origin, pixels down from the layout's top edge |
//! | 4 | `Taqaddum` | the advance this glyph contributed; zero for marks |
//! | 5 | `Nitaq` | the style span id this glyph inherited, so colour survives reordering |
//! | 6 | `Khatt` | index into the font chain |
//! | 7 | `Alam` | flags — [`natija_js::AlamHarf::Alama`] marks a combining mark |
//!
//! Lines cross the same way, one [`natija_js::HaqlSatr::Adad`]-float row per
//! line in [`natija_js::HaqlSatr`]'s order, mirroring the C ABI's
//! `TaaribSatr`:
//!
//! | index | name | meaning |
//! | --- | --- | --- |
//! | 0 | `AwwalHarf` | index of the line's first glyph row |
//! | 1 | `AdadHuruf` | how many glyph rows the line has |
//! | 2 | `BidayatMantiqi` | first byte of logical text the line covers |
//! | 3 | `NihayatMantiqi` | one past the last byte |
//! | 4 | `Asas` | the baseline's y, pixels from the layout's top |
//! | 5 | `Bidaya` | where the line box begins horizontally, after alignment |
//! | 6 | `Ard` | the measured width of the line's content |
//! | 7 | `Irtifa` | the line box's height |
//! | 8 | `Suud` | how far the tallest content rises above the baseline |
//! | 9 | `Hubut` | how far the deepest content falls below it |
//! | 10 | `Dabt` | how much width justification added |
//! | 11 | `Alam` | flags — [`natija_js::AlamSatr::Akhir`], [`natija_js::AlamSatr::Yameen`] |
//!
//! Every value in a row is exact in an `f32`: glyph identifiers are 16-bit in
//! OpenType, span and chain indices are 16- and 8-bit, and byte offsets are
//! exact below 2^24 — sixteen megabytes of text, which no game string
//! approaches.
//!
//! ## Errors are real errors
//!
//! Every fallible export throws a JavaScript `Error` whose `message` is the
//! Arabic sentence and whose `name` is `TaaribKhata`, carrying as properties
//! the same four things the C ABI hands back through its retrieval functions:
//! `ramz` (the permanent code, `TAARIB-E-2501`), `injilizi` (the English
//! sentence), `khutwa` (the next-action discriminant, numbered exactly as the
//! C ABI numbers it — [`khata_js::Khutwa`] names the values), and `khutura`
//! (the severity, [`khata_js::Khutura`]). A rejected font therefore reaches
//! the adapter as an exception that says *which table is missing*, in both
//! languages, with the action that fixes it — not as `"failed"`.
//!
//! ## Fonts are loaded from bytes
//!
//! [`khatt_js::TaaribKhatt::min_bayt`] takes a `Uint8Array` and a face index.
//! There is no path variant and no URL variant, deliberately: this module has
//! no filesystem and performs no fetch. The adapter reads the font out of the
//! installed patch — a file the patch installer already placed and whose hash
//! the patch already pinned — and hands the bytes over. A module that fetched
//! for itself would be a module with a network dependency inside somebody's
//! offline game, a second copy of the font outside the patch's integrity
//! seal, and a loading order the adapter cannot see. Bytes in, glyphs out.
//!
//! ## `hayyi()` is explicit
//!
//! [`hayyi`] installs `console_error_panic_hook` so a panic reaches the
//! browser console as a readable Rust message rather than as
//! `unreachable executed`. It is a separate export, called by the adapter
//! once at startup, and **not** run automatically at module load: this module
//! is injected into other people's games, and a host page — a game engine, an
//! overlay, a debugger — may own the panic surface already. A library that
//! installs process-wide hooks as a side effect of being imported is a
//! library fighting its host; one that offers the hook and lets the adapter
//! decide is not. The hook is diagnostic, not protective: a panic still means
//! a bug, and the JavaScript side treats a trapped module as unusable rather
//! than retrying into it.
//!
//! ## Hard constraints
//!
//! - No `std::fs`, no `std::net`, no clock. The module is fed bytes by its
//!   host and returns bytes and numbers; the patch file, the font file and
//!   the atlas are all read by the JavaScript side and handed in.
//! - Glyphs cross into JavaScript as glyph identifiers and positions.
//!   Presentation forms do not exist on this path any more than they do on
//!   the native one (Decision 1).
//! - Every exported class has the `free()` that `wasm-bindgen` generates, and
//!   the adapter calls it when a handle's life ends. WebAssembly linear
//!   memory has no garbage collector reaching into it, and a leaked layout in
//!   a game that runs for six hours is a leak the player pays for. The
//!   short-lived results — a layout, a measurement — are freed after their
//!   frame; the long-lived handles — engine, chain, atlas — live as long as
//!   the game does.
//! - The build target is `wasm32-unknown-unknown` with no WASI, no filesystem
//!   shim, and no thread pool, so the module loads inside NW.js 0.29 for
//!   RPG Maker MV as readily as inside current Chromium for Electron.
//! - This crate does not go through the C ABI — there is nothing to go
//!   through in a browser — but it exposes the same surface shape and the
//!   same names, so adapter code reads the same in TypeScript as it does in
//!   C# or Ruby, which is what keeps five adapter implementations behaving
//!   identically instead of drifting apart one convenience at a time.

#![expect(
    clippy::missing_const_for_fn,
    reason = "`#[wasm_bindgen]` rejects `const fn` outright — \"can only \
              #[wasm_bindgen] non-const functions\" — so every accessor across this \
              crate trips a lint that cannot be satisfied where it fires"
)]

pub mod khata_js;
pub mod khatt_js;
pub mod lawha_js;
pub mod natija_js;
pub mod saff_js;
pub mod talab_js;

use wasm_bindgen::prelude::wasm_bindgen;

pub use crate::khata_js::{Khutura, Khutwa};
pub use crate::khatt_js::{TaaribKhatt, TaaribQiyasat, TaaribSilsila};
pub use crate::lawha_js::{
    NamatLawha, TaaribIhsaat, TaaribKhiyaratLawha, TaaribLawha, TaaribMawdi,
};
pub use crate::natija_js::{
    AlamHarf, AlamSatr, HaqlHarf, HaqlSatr, NawDharra, TaaribQiyas, TaaribTajawuz, TaaribTakhtit,
    TaaribTakhtitKhaam,
};
pub use crate::saff_js::{Lahja, TaaribSaff};
pub use crate::talab_js::{
    IttijahAsas, LughaNass, Muhadhaha, NamatDabt, SiyasatArqam, SiyasatTajawuz, SiyasatTashkeel,
    TaaribKhiyarat, TaaribNitaq, TaaribTalab,
};

/// Prepares the module for use inside a page: installs the panic hook that
/// turns a Rust panic into a readable browser-console message.
///
/// Call it once, from the adapter, at startup. It is idempotent — the hook is
/// installed at most once however many times this is called — and it is
/// deliberately not run at module load: an adapter injected into a game may be
/// loaded in a context where installing a panic hook fights with the host's
/// own, and that is the host's call to make, not this module's.
#[wasm_bindgen]
pub fn hayyi() {
    console_error_panic_hook::set_once();
}

/// The version of the engine this module was built from, as `major.minor.patch`.
///
/// The same number the native library reports through `taarib_abi_isdar`,
/// because they are built from the same workspace at the same version. An
/// adapter logs it once at startup so a bug report says which engine drew the
/// text.
#[wasm_bindgen]
#[must_use]
pub fn isdar() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Which subpixel bucket a horizontal pen position falls in.
///
/// The atlas keys every glyph image on a quantized fractional pen position, so
/// an adapter that draws at `x = 118.3` asks the atlas for the glyph in the
/// bucket `118.3` quantizes to and then blits at the whole-pixel part. This is
/// [`taarib_saff::rasm::bakat_tahazzuz`] re-exported to JavaScript; quantizing
/// here and rasterizing there use the same arithmetic, so the bitmap that
/// comes back and the position that asked for it agree.
#[wasm_bindgen]
#[must_use]
pub fn bakat_tahazzuz(s: f32) -> u8 {
    taarib_saff::rasm::bakat_tahazzuz(s)
}

/// How many subpixel buckets exist.
///
/// [`bakat_tahazzuz`] always returns a value below this. An adapter that
/// rounds pen positions to whole pixels can ignore both and pass bucket zero.
#[wasm_bindgen]
#[must_use]
pub fn mawadi_tahazzuz() -> u8 {
    taarib_saff::rasm::MAWADI_TAHAZZUZ
}
