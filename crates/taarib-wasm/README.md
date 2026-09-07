# taarib-wasm

**نواة تعريب لويب أسمبلي — the Taarib text engine, compiled for JavaScript.**

This crate produces the `taarib_core.js` / `taarib_core_bg.wasm` pair: the
same `taarib-saff` Arabic engine and the same `taarib-lawha` glyph atlas that
live inside the native library, compiled to `wasm32-unknown-unknown` and
exposed through `wasm-bindgen` instead of through C. It exists because two
engine families in Taarib's coverage matrix — RPG Maker MV/MZ and Electron —
run their text drawing inside a JavaScript runtime that cannot load a native
library at all.

## One engine, not a reimplementation

This is a build of the engine, not a port of it. The shaping, the
bidirectional algorithm, the line breaking, the kashida justification, the
font validation and the atlas packing are the exact Rust code the desktop
library runs — `taarib-saff` and `taarib-lawha` compile to
`wasm32-unknown-unknown` unchanged, which is the reason Decision 2 chose a
pure-Rust shaper in the first place.

The alternative — an Arabic shaping implementation written in JavaScript for
the browser-hosted engines — was never on the table, and the reasoning is
worth stating because it is the load-bearing wall of this crate: a second
implementation is a second set of bugs, a second thing to keep correct, and a
permanent guarantee that the two paths drift. A lam-alef that ligates on
desktop and does not inside NW.js would be a bug report nobody could
reproduce, filed against whichever half of the product the reporter happened
to be looking at. With one engine there is no second half. The browser path
and the native path are the same code producing the same glyphs, and a layout
measured by the patch compiler is the layout the game draws.

What this crate adds on top is only the boundary: classes instead of opaque
pointers, thrown errors instead of status codes, typed arrays instead of
caller-owned buffers — the same surface *shape* as the C ABI, in JavaScript's
idiom, so an adapter author moving between the C# side and the TypeScript
side recognises every name.

## Building

The shipping build is the one `scripts/isdar.sh` runs, and both of its flags
are load-bearing:

```sh
cargo build --release --target wasm32-unknown-unknown -p taarib-wasm
wasm-bindgen --target no-modules --out-name taarib_core \
    --out-dir target/wasm-bindgen target/wasm32-unknown-unknown/release/taarib_wasm.wasm
```

`--target no-modules` because the RPG Maker plugin reads the global
`wasm_bindgen` that only that target defines — the games run from `file://`
in NW.js, where an ES module import is not an option — and
`--out-name taarib_core` because the adapter loads exactly these two names
and cannot be parameterised. The `wasm-bindgen` CLI version must equal the
`wasm-bindgen` crate in `Cargo.lock`; the script checks and refuses otherwise.
(A `wasm-pack build --target web` run produces a `pkg/` of `taarib_wasm.*`
ES-module files; it is a fine way to poke at the API in a browser and it is not
what ships.)

The build target is `wasm32-unknown-unknown` — no WASI, no filesystem shim,
no thread pool — so the module loads inside NW.js 0.29 (RPG Maker MV) as
readily as inside current Chromium (RPG Maker MZ, Electron).

`wasm-bindgen` writes `target/wasm-bindgen/`:

| file | what it is |
| --- | --- |
| `taarib_core_bg.wasm` | the engine itself — shaper, bidi, line breaker, justifier, rasterizer, atlas |
| `taarib_core.js` | the no-modules glue: defines the global `wasm_bindgen`, loads the module, wraps every export in the classes below |
| `taarib_core.d.ts` | TypeScript declarations for the whole surface, generated from the Rust signatures and doc comments |
| `taarib_core_bg.wasm.d.ts` | declarations for the raw wasm exports; adapters never touch these directly |

`taarib-tajmee` stages the pair (matrix row H1) under each JavaScript
adapter's own `taarib/` subdirectory — `mukawwinat/mulhaq/rpgmaker/{mv,mz}/taarib/`
and `mukawwinat/mulhaq/electron/taarib/` — and the adapters load them as two
separate files from beside themselves: the RPG Maker plugin injects the glue
through a `<script>` element and fetches the `.wasm` over `XMLHttpRequest`,
which is how those games load everything from `file://`. Nothing is inlined
into the plugin, and nothing in this crate fetches anything — see below.

## Consumers

**The RPG Maker MV/MZ plugin** (`adapters-script/rpgmaker/`, injected as
`js/plugins/taarib.js`). It overrides `Bitmap.prototype.drawText` and the
`Window_Base` text pipeline — `processCharacter`, `drawTextEx`,
`calcTextHeight`, `textWidth` — so every string the engine draws is laid out
here. Message text still carries its escape codes (`\C[n]`, `\I[n]`,
`\V[n]`, `\G`), so the plugin calls `khattit_khaam` with `Lahja.RpgMaker`
and gets the codes back as atoms and spans rather than glyphs; measurement
questions route through `qis`; and every glyph is blitted from the atlas
onto the window's bitmap.

**The Electron runtime** (`adapters-script/electron/`). Its default rung
leaves the browser's text APIs alone on purpose: Chromium shapes Arabic in the
DOM *and* in `fillText`, so the runtime installs the font, sets the direction
and replaces the strings, and the text stays real text — selectable,
searchable, legible to a screen reader. The takeover in `rakkibLawha` is for
the one case the browser cannot help: a game that draws each character as its
own sprite, blits a bitmap font, or bundles its own layout library. On that
rung, and only when the patch recorded it, `measureText` becomes `qis` and
`fillText` becomes `khattit` plus one blit per glyph from the atlas page.

The RPG Maker plugin always draws glyph *images* from the atlas, because
`Bitmap.drawText` is the engine's own rasterizer and the only way to put the
glyphs shaping chose — contextual forms, lam-alef, the `rlig` and `calt`
alternates — onto its bitmaps. The Electron runtime does so only on its
takeover rung.

## Using it

```js
// The shipping glue is a no-modules build: taarib_core.js defines a global
// `wasm_bindgen` and the module's exports hang off it once it has loaded.
await wasm_bindgen("./taarib/taarib_core_bg.wasm");
const {
  hayyi, TaaribKhatt, TaaribSilsila, TaaribSaff, TaaribTalab,
  TaaribLawha, TaaribKhiyaratLawha, HaqlHarf, HaqlSatr, Lahja,
} = wasm_bindgen;

hayyi();                                   // panic hook — explicit, once

// Fonts come from bytes the adapter already read out of the patch.
const khatt   = TaaribKhatt.min_bayt(new Uint8Array(fontBytes), 0);
const silsila = new TaaribSilsila([khatt]);

const saff  = new TaaribSaff();
const lawha = new TaaribLawha(new TaaribKhiyaratLawha());

// Per frame:
lawha.ibda_itar();
const takhtit = saff.khattit_khaam("\\C[2]مرحبًا بالعالم", Lahja.RpgMaker,
                                   silsila, 24, 320);
const huruf = takhtit.huruf;               // one Float32Array, read once
for (let n = 0; n < takhtit.adad_huruf; n += 1) {
  const saf     = n * HaqlHarf.Adad;
  const muarrif = huruf[saf + HaqlHarf.Muarrif];
  const khattFi = huruf[saf + HaqlHarf.Khatt];
  const mawdi   = lawha.shakl(silsila, khattFi, muarrif, takhtit.hajm, 0);
  // blit page rect (mawdi.s, mawdi.a, mawdi.ard, mawdi.irtifa) at
  // (huruf[saf + HaqlHarf.S] + mawdi.izaha_s, baseline - mawdi.izaha_a)
}
takhtit.free();                            // linear memory has no GC
```

`hayyi()` is explicit rather than automatic because this module is injected
into other people's games, and the host may own the panic surface already;
`free()` is real because WebAssembly linear memory has no garbage collector
reaching into it — short-lived results are freed after their frame, the
engine, chain and atlas live as long as the game.

## The packed rows

Glyphs cross the boundary as **one `Float32Array`**, one fixed-stride row
per glyph in visual order, because a hundred objects per redrawn frame is a
garbage-collection pause the player sees. `HaqlHarf` and `HaqlSatr` export
the indices; `HaqlHarf.Adad` and `HaqlSatr.Adad` are the strides. The order
mirrors the C ABI's `TaaribHarf` and `TaaribSatr`, field for field.

Glyph row (`takhtit.huruf`, stride `HaqlHarf.Adad` = 8):

| index | name | meaning |
| --- | --- | --- |
| 0 | `Muarrif` | glyph identifier, in the font named by `Khatt` |
| 1 | `Anqud` | byte offset into the logical text of this glyph's cluster |
| 2 | `S` | x of the glyph origin, pixels from the layout's left edge |
| 3 | `A` | y of the glyph origin, pixels down from the layout's top |
| 4 | `Taqaddum` | the advance this glyph contributed; zero for marks |
| 5 | `Nitaq` | style span id, so colour survives reordering |
| 6 | `Khatt` | index into the font chain |
| 7 | `Alam` | flags: `AlamHarf.Alama` marks a combining mark |

Line row (`takhtit.sutur`, stride `HaqlSatr.Adad` = 12):

| index | name | meaning |
| --- | --- | --- |
| 0 | `AwwalHarf` | index of the line's first glyph row |
| 1 | `AdadHuruf` | how many glyph rows the line has |
| 2 | `BidayatMantiqi` | first byte of logical text the line covers |
| 3 | `NihayatMantiqi` | one past the last byte |
| 4 | `Asas` | baseline y, pixels from the layout's top |
| 5 | `Bidaya` | where the line box begins horizontally, after alignment |
| 6 | `Ard` | measured width of the line's content |
| 7 | `Irtifa` | the line box's height |
| 8 | `Suud` | rise of the tallest content above the baseline |
| 9 | `Hubut` | fall of the deepest content below it |
| 10 | `Dabt` | width justification added to this line |
| 11 | `Alam` | flags: `AlamSatr.Akhir`, `AlamSatr.Yameen` |

Every value is exact in an `f32`: glyph identifiers are 16-bit in OpenType,
span and chain indices are 16- and 8-bit, and byte offsets stay exact below
2^24 — sixteen megabytes of text, which no game string approaches. The
arrays are copies, not views into wasm memory: a view is silently detached
the next time the module's memory grows, and that is a corruption no adapter
could reproduce.

## Errors

Every fallible export throws a JavaScript `Error` whose `name` is
`TaaribKhata` and whose `message` is the Arabic sentence. It carries the same
parts the C ABI hands back — `ramz` (the permanent code, `TAARIB-E-2501`),
`injilizi` (the English sentence) and `khutwa` (the next-action discriminant,
numbered exactly as the native library numbers it; the `Khutwa` enum names the
values) — plus two the C surface has no accessor for: `khutura` (the severity)
and `sijill` (the nested cause chain). A font
without a `GSUB` table is rejected at `TaaribKhatt.min_bayt` with the
sentence that says so, in both languages, with the action that fixes it —
not with `"failed"`.

## Limits worth knowing

- No fetch, no paths, no filesystem. Fonts and pages are bytes in, bytes
  out; the adapter owns all I/O. This keeps the font inside the patch's
  integrity seal and the module loadable in an offline game.
- One `TaaribSaff` per adapter. It is a mutable shaping cache; JavaScript is
  single-threaded per realm, so it carries no lock.
- The atlas requires `ibda_itar()` once per frame. Positions handed out
  during a frame are pinned until the next one begins — that is what makes
  eviction safe — and an adapter that never begins a frame fills the atlas
  with unevictable glyphs and gets a loud, specific error instead of one
  wrong letter on screen.
- Building a `TaaribSilsila` consumes the `TaaribKhatt` handles it is
  given; keep a `istinsakh()` clone first if a font is also needed
  directly.

## License

MPL-2.0, as the whole workspace.
