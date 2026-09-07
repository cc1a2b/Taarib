# المعمار — how Taarib is put together

A map of every crate under `crates/`, the four boundaries that keep them honest,
and the path a single string takes from a game's own data files to Arabic drawn
on a screen.

The authority for all of this is [`ROADMAP.md`](../ROADMAP.md) — sections 2
(the eight settled decisions), 4 (the component map and the naming law) and 5
(the repository layout). This document is the readable version; where they
disagree, the roadmap wins.

---

## 1. One text engine, many ways into a game

```
  Taarib Studio (Tauri + React)
        discovery -> engine probe -> extraction -> translation -> compile -> install
                                                                   |
  =================================================================v==========
                              THE GAME PROCESS
   Unity plugin  .  Unreal module  .  Godot  .  script adapters  .  overlay
        +----------------------+-----------------------------------+
                     jisr - one stable C ABI
                               |
                     saff - the Arabic text engine
       markup -> bidi -> line breaking -> shaping -> measurement -> kashida -> raster
                               |
                     lawha - the glyph atlas
```

Everything above the double line runs on the user's desktop, in Studio, before
the game is ever launched. Everything below it runs inside a process Taarib does
not own. The narrow point between them is deliberate: it is one C ABI, and it is
the only way in from outside Rust.

## 2. The four boundaries

These are from `ROADMAP.md` section 4.4, and they are the reason the tree does
not turn into a pile of mutually aware modules.

**`saff` knows nothing.** No games, no engines, no launchers, no graphics APIs,
no filesystem beyond fonts handed to it as bytes, no network, no logging sink of
its own, no global state. It turns logical-order text into ordered visual lines
of positioned glyphs, and that is the entire contract. Two things follow, and
both are the point: it compiles to `wasm32-unknown-unknown` unchanged (which is
what makes the RPG Maker and Electron adapters possible without a second
implementation of Arabic shaping in JavaScript), and it is testable and valuable
entirely on its own. A text engine that can reach for a file or a log when it is
confused will eventually do so instead of returning a real answer.

**`jisr` is the only way in from outside Rust.** C#, C++, Python, Ruby and
JavaScript reach the engine through one narrow, versioned, ownership-explicit C
ABI. There is no second entry point and no engine-specific extension of it. The
ABI contract is documented in full in [`abi.md`](abi.md).

**Adapters never make policy.** An adapter renders what a patch tells it to
render. It does not decide whether a patch applies, does not check safety, does
not talk to the network, and does not read the registry. All of that happened in
Studio before the game launched. An adapter that could decide things would be a
second place where safety is enforced, and the second place is always the one
that is wrong.

**Studio never renders.** Every pixel of Arabic in a preview comes from `saff`
through the same code path the game will use. The frontend does not reimplement
shaping to draw a preview, because a preview that lies is worse than no preview.

## 3. The crates

Thirty under `crates/` as this is written, plus the Studio's own `taarib-studio`
under `apps/studio/src-tauri`, which makes thirty-one workspace members. The
authoritative list is `[workspace] members` in the root `Cargo.toml`; if a table
below and that list ever disagree, the manifest is right and this page is stale.
Names mirror the domain rather than abstracting it — there is no `utils`, no
`helpers`, no `common`, no `manager` and no `service` anywhere in the tree. The
full lexicon is `ROADMAP.md` section 4.1.

### Foundations

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-usus` | أسس | errors, diagnostics, configuration, paths, platform facts |
| `taarib-mustalahat` | مصطلحات | the shared vocabulary; source of the TypeScript bindings and the JSON Schemas |
| `taarib-makhzan` | مخزن | the local SQLite store and its versioned migrations |

### The text engine

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-saff` | صف | the Arabic text engine: markup, bidi, breaking, shaping, measurement, kashida, rasterization |
| `taarib-lawha` | لوحة | the glyph atlas: collection, packing, distance fields, and the Godot 3 / GameMaker glyph transport |
| `taarib-jisr` | جسر | the stable C ABI, as `cdylib` and `staticlib`, with a generated header |
| `taarib-wasm` | — | the same engine compiled for JavaScript, via wasm-bindgen |

### Finding and understanding a game

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-kashf` | كشف | discovery across every launcher's own catalogue format, plus Proton and Wine prefixes |
| `taarib-muharrik` | محرك | engine identification and the capability probe: what the game is, and what can be done to it |
| `taarib-istikhraj` | استخراج | text extraction, static from the engine's containers and dynamic from runtime capture |
| `taarib-aql` | عقل | one game held whole: every producer's answer about a game composed into a single vantage point, so that two screens cannot disagree about the same game |

### Producing a patch

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-tarjama` | ترجمة | the translation pipeline: machine translation, glossary, translation memory, context |
| `taarib-warsha` | ورشة | the collaborative workspace — translating together without a server |
| `taarib-ruqaa` | رقعة | the `.ruqaa` patch container: reader, writer, tables, manifest |
| `taarib-tarqee` | ترقيع | the patch compiler, including the provenance gate |

### Getting it onto a machine

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-aman` | أمان | the safety layer: anti-cheat detection, quarantine unpacking, signature verification, consent |
| `taarib-khatm` | ختم | signing, verification, revocation, and the two signing identities |
| `taarib-mustawda` | مستودع | the registry client: shards, matching, download, ranking |
| `taarib-taqdeem` | تقديم | submission, the owner's review console, publication |
| `taarib-tathbeet` | تثبيت | install, byte-exact backup, rollback, and surviving a game update |
| `taarib-tahdith` | تحديث | the application's own update channel |
| `taarib-tajmee` | تجميع | the build-machine staging tool (a binary, no library) |
| `taarib-tilqai` | تلقائي | the one-button pipeline: probe, extract, translate, build, install, with resume and cancel — it drives stages that already exist and adds no capability of its own |

### Inside the game

| crate | Arabic | what it is |
| --- | --- | --- |
| `taarib-haqn` | حقن | getting inside a process, and hooking once there |
| `taarib-mudkhal` | مدخل | the loader a game loads on its own, which opens the payloads beside it |
| `taarib-muhawwil-unreal` | محوّل | the Unreal adapter |
| `taarib-muhawwil-godot` | محوّل | the Godot adapter |
| `taarib-muhawwil-nusus` | محوّل نصوص | the script-engine patchers: RPG Maker, Ren'Py, VX Ace, GameMaker, Electron |
| `taarib-muhawwil-bio4` | محوّل بيو٤ | the Capcom BIO4 adapter (Resident Evil 4's 2005 codebase): the `.fnt` font container, the fixed-cell glyph grid and the cell transport for an engine with no shaper. Readers and builders exist; nothing routes an install to them yet — see `tashghil.md` |
| `taarib-tabaqa` | طبقة | the universal overlay: Direct3D 8, 9, 10, 11 and 12, OpenGL in both its fixed-function and modern profiles, and Vulkan. It attaches and can draw; nothing feeds it text yet — see `tashghil.md` |

The Unity side is C#, under `unity/` — four projects built by
`dotnet build unity/Taarib.Unity.sln`. The in-game JavaScript, Python and Ruby
sides are under `adapters-script/`.

## 4. The two pipelines

### Producing a patch

1. **`kashf`** reads each launcher's own catalogue — Steam's VDF, GOG's
   database, the Epic manifests, and so on — plus Proton and Wine prefixes, and
   produces a list of installed games with real paths.
2. **`muharrik`** identifies the engine and probes what can be done to it,
   producing a capability report. The report is what the user is shown before
   anything is installed, and it names the tier honestly.
3. **`istikhraj`** pulls translatable strings out. Statically from the engine's
   own containers where they can be parsed; by recording every string the game
   actually draws where they cannot. This is the one place Taarib reads game
   assets at all, it is read-only, and it is never on the rendering path.
4. **`tarjama`** runs the translation pipeline — machine translation with
   placeholder protection, glossary, translation memory, per-string context —
   and `warsha` lets several people work on it.
5. **`tarqee`** compiles. It lays every string out through `saff` at every size
   the game draws at, collects the resulting glyph set through `lawha`,
   rasterizes an atlas from Taarib's own fonts, and writes a `.ruqaa`.
6. **`taqdeem`** submits it. **`khatm`** signs it, once, at approval.

### Installing a patch

1. **`mustawda`** matches your installed build against the registry index in
   three tiers: exact build identifier, then content fingerprint, then a
   declared compatibility range with an explicit warning.
2. **`aman`** refuses on anti-cheat evidence, unpacks the download in
   quarantine, and verifies the signature. None of these has an override.
3. **`tathbeet`** backs up every file byte-exact *before* touching it, writing
   the manifest before the change it describes, then installs the framework and
   the patch.
4. At launch, **`mudkhal`** is loaded by the game and opens the payloads beside
   it, calling `taarib_bidaya` on the ones that export it. That contract is
   [`bidaya.md`](bidaya.md), which also records exactly where each path
   currently stops.
5. The adapter draws through **`jisr`** into **`saff`** and **`lawha`**.

Uninstall runs `tathbeet` in reverse from the manifest, and verifies the restore
rather than assuming it.

## 5. Why the compiler precomputes everything

A shipped patch carries finished layouts for every string at every size the game
draws at, and an atlas containing exactly the glyphs real shaping produced from
the real strings.

Two reasons. The first is cost: a game that shapes text every frame pays for it
every frame, inside a process whose budget is not ours to spend. The second is
correctness: the atlas is built from the same shaping call the game will run, so
the set is exactly the set that will be asked for. An atlas built from a guessed
Unicode range would miss every ligature (they have no codepoints to enumerate)
and include tens of thousands of images nothing draws. That argument is written
out in `crates/taarib-lawha/src/tafrigh.rs` and summarised in
[`taqdimiya.md`](taqdimiya.md).

## 6. Two library choices that are load-bearing

**HarfRust for shaping, skrifa for rasterization** — chosen together, not
separately. Both parse fonts through the same `read-fonts` stack, so a glyph
identifier produced by the shaper is by construction the same glyph the
rasterizer resolves an outline for. That removes an entire class of silent,
data-dependent bug where the shaper's glyph 4211 and the rasterizer's glyph 4211
are different glyphs because two libraries disagreed about face indices,
collection offsets, variation instances or `CFF2` charstring numbering.

`deny.toml` enforces the pairing rather than trusting it: two copies of
`read-fonts` in the dependency graph is a build failure, and a native HarfBuzz
or rustybuzz entering the graph is a build failure.

**One font byte buffer.** A loaded font is a single shared, reference-counted,
immutable buffer with one identity. Shaping borrows it, rasterization borrows
it, metrics borrow it, and the atlas records that identity in every glyph key.
Nothing loads the same font twice into two parsers. This makes the guarantee
above total rather than probable, and it makes the layout cache key sound — a
font is identified by the identity of its bytes, not by a path or a family name
that could resolve differently later.

## 7. Taarib never touches the game's fonts

Not the font assets, not the engine's font asset system, not asset bundles.

Asset bundles are serialized against the exact engine version that produced them
— down to type trees, field ordering and container framing. Tools built on
rewriting them break on every engine version bump, and their maintainers spend
their lives chasing formats instead of improving the product.

So Taarib rasterizes its own glyphs, packs its own atlases, builds its own
meshes, and asks the engine for nothing but a texture, a mesh and a material —
primitives that exist in every version of every engine. The rendering path works
on a Unity version that does not exist yet.

## 8. Nothing untraceable enters a package

`crates/taarib-tarqee/src/bawwaba.rs` calls itself the product's legal
foundation, and it is deliberately **not** a check.

A function that walks a finished package looking for game bytes can only answer
"these bytes do not look like a game asset". It cannot answer "these bytes did
not come from a game", because by the time it runs the provenance is gone — a
texture read out of `resources.assets` and a texture Taarib rasterized are both a
width, a height and a run of bytes.

So provenance is carried rather than inferred. `IthbatTawlid` is a token meaning
*Taarib generated these bytes*. It has no public constructor. The only thing
that mints one is `rassim`, which takes a glyph set and a chain of Taarib's own
bundled fonts and produces an atlas from them. To get bytes into a package a
caller must hold that token; to hold one it must have called `rassim`; to call
`rassim` it needs a font that can only be named inside Taarib's own font
directory. There is no path from a game directory to any of those, and adding
one would mean adding a constructor in that file — a visible change in review,
rather than a `Vec<u8>` quietly acquiring a new caller somewhere else.

Where an engine keeps text inside a container that also holds art, audio and
code, a package never carries the rewritten container. It carries `FarqHawiya` —
the instructions to produce one — and the installer applies those to the user's
own copy. The original never travels.

There is no entropy heuristic, no signature table, no "does this look like a
PNG" test anywhere in that module. Every one of those would be an inference
about bytes whose origin is already known by construction, and adding one would
suggest the construction is not trusted.

## 9. Determinism

The same project compiled twice produces a byte-identical `.ruqaa`. Nothing in
the manifest is a timestamp, a path, a machine name, a username, or ordered by a
hash map. Slot assignment in the glyph transport is a function of the *set* of
glyphs alone — a `BTreeSet`, walked in ascending order — so it does not depend
on the order strings were registered in or on a hash seed.

This is not tidiness. A build time in the container would make two compiles of
one unchanged project hash differently, which makes a rebuild indistinguishable
from tampering, invalidates every mirror, and costs the owner a re-signature for
nothing.

## 10. Where to read next

- [`abi.md`](abi.md) — the `jisr` C ABI in full: types, ownership, versioning,
  error model.
- [`bidaya.md`](bidaya.md) — the payload bootstrap contract, and the honest
  record of what each in-game path currently reaches and where it stops.
- [`tawzee.md`](tawzee.md) — the packaging and distribution contract: signing
  identities, targets, and the artifact matrix the staging tool enforces.
- [`taqdimiya.md`](taqdimiya.md) — why Taarib never produces a presentation
  form.
- [`bina.md`](bina.md) — building all of it from source.
