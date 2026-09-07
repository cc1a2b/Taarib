# TAARIB — ROADMAP

**تعريب** — bringing Arabic to PC games that were never built to accept it.

This document is the contract for the entire build. Every phase is executed against it.
It is written to be sufficient: a reader who has never seen the product should be able to
build it from this file alone. Nothing here is a summary, an aspiration, or a sketch.

**Status:** plan complete, Phase 0 not started.
**Version of this contract:** 1.0.0 — changes to it are changes to the product.

> **How to read this document now.** It is the contract as it was written before Phase 0,
> kept verbatim; the status line above is the status of the plan, not of the tree. What has
> been built since, what has been observed working, and where each in-game path stops are
> recorded in [`README.md`](README.md) under "What works today" and in
> [`docs/tashghil.md`](docs/tashghil.md). Where this document and the tree disagree, the tree
> wins. Two places where they are known to: section 5 names a `taarib-barid` crate that was
> never created and omits six that were (`taarib-mudkhal`, `taarib-tahdith`, `taarib-tajmee`,
> `taarib-tilqai`, `taarib-aql`, `taarib-muhawwil-bio4`); and the coverage matrix in section 3
> predates Capcom's BIO4 codebase and the six in-house engine families — Frostbite, BlackSpace,
> Alchemy, Dantelion, RAGE, Snowdrop — that `taarib-mustalahat` now recognises by name and
> sends to the overlay tier because nothing can read their containers.

---

## Table of Contents

1. [Product Definition and the Problem It Solves](#1-product-definition-and-the-problem-it-solves)
2. [The Eight Non-Negotiable Architectural Decisions](#2-the-eight-non-negotiable-architectural-decisions)
3. [Coverage Matrix](#3-coverage-matrix)
4. [Component Map](#4-component-map)
5. [Repository Layout](#5-repository-layout)
6. [The Twenty-Four Phases](#6-the-twenty-four-phases)
7. [Dependency Graph](#7-dependency-graph)
8. [Interface Design Direction](#8-interface-design-direction)
9. [Operating Rules for the Build](#9-operating-rules-for-the-build)

---

## 1. Product Definition and the Problem It Solves

### 1.1 What Taarib is

Taarib is a cross-platform desktop application that installs Arabic into PC games which
have no Arabic support of any kind, and gives translators the tools to produce those
Arabic versions in the first place.

It has two faces over one codebase:

- **The player face.** Open the application. It has already found every game installed on
  the machine across every launcher. Games with a community Arabic patch are badged. Click
  one, press install, play in Arabic. The player never sees a string table, never sees a
  font, never learns what an engine is.
- **The translator face.** Extract every translatable string out of a game, translate it
  with machine assistance and human review, see exactly how each line will render inside
  the real interface bounds before it ships, compile it into a signed patch, and submit it
  for publication.

One person — the project owner — reviews and publishes everything that reaches the public
registry. There is no committee, no automatic merge, no self-service publishing.

### 1.2 The problem, stated precisely

Arabic is not a font problem. Substituting an Arabic font into a game that has no Arabic
support produces text that is broken in four independent ways at once:

1. **No contextual joining.** Arabic letters change shape according to their neighbours:
   isolated, initial, medial, final. A naive renderer draws twenty-eight disconnected
   isolated forms. The result is not "ugly Arabic" — it is not Arabic. A reader cannot
   parse it.
2. **No ligature formation.** Lam-alef is mandatory, not decorative: ل + ا must become لا.
   Naskh fonts carry hundreds more required and optional ligatures in their substitution
   tables. Without a shaper none of them fire.
3. **No bidirectional layout.** Arabic runs right to left, but numbers run left to right,
   embedded Latin runs left to right, punctuation and brackets take direction from
   context, and brackets must mirror. A game that lays out text left to right and then
   naively reverses the string gets numbers backwards, gets nested Latin backwards, and
   gets mirrored punctuation wrong. This is the single most common failure in existing
   Arabic game patches.
4. **No mark positioning.** Diacritics (تشكيل) are combining marks that must be attached to
   the base glyph through the font's mark-attachment tables, and stacked correctly when
   several land on one base. Drawn naively they float, collide, or vanish.

On top of the script problem sits the engine problem. Games do not expose a text pipeline.
Unity draws text through TextMeshPro or the legacy UI system with its own layout,
generating its own meshes from its own font atlas assets. Unreal runs its own Slate
shaping stack with its own font cache. Godot 3 has no complex-script support at all. RPG
Maker draws to a canvas. Each of these has to be taken over on its own terms.

And under the engine problem sits the distribution problem. Even when a patch exists, it
lives in a forum thread, a Discord message, or a dead file host, with no versioning, no
integrity guarantee, no safety check, and no way to know whether it matches the build the
player has installed. Most Arabic-speaking players never find it.

### 1.3 What existing tools do, and why Taarib is not that

The prevailing approach — used by every widely deployed Arabic game patch today — converts
Arabic text into **Unicode Presentation Forms** (blocks U+FB50–U+FDFF and U+FE70–U+FEFF),
manually reverses the string into visual order, and hands the result to the game as if it
were plain left-to-right text. It works well enough to look like Arabic in a screenshot.

It is also structurally wrong, and Taarib does not do it:

- Presentation forms are a legacy compatibility encoding. They cover a fraction of what a
  real Arabic font's substitution tables express, and nothing at all for Persian, Urdu, or
  the extended Arabic ranges.
- Manual visual reversal is a lossy approximation of the Unicode Bidirectional Algorithm.
  It breaks the moment a number, a Latin word, or a bracket appears — which is constantly,
  in games full of item counts, key bindings, and percentages.
- Copying such text out of the game yields garbage. Search does not work. Screen readers do
  not work.
- Diacritics have no defined behaviour in that pipeline at all.
- It requires replacing the game's own font assets, which are locked to the exact engine
  version the game shipped with — the single largest source of breakage in existing tools.

Taarib rejects the entire approach. It runs a real shaping engine over real font tables,
resolves direction with the real algorithm, and draws its own glyphs from its own atlases
using only the most basic primitives the engine exposes — texture, mesh, material.

### 1.4 Definition of success

- A player with no technical knowledge installs an Arabic patch in one click and the game
  is readable, correctly joined, correctly ordered, correctly justified.
- A translator with no programming knowledge produces a complete, submittable patch inside
  Taarib without touching a file manager or a command line.
- A patch survives a store update that did not change the game's text, because matching
  falls back to content fingerprints rather than build identifiers alone.
- An engine version Taarib has never seen still works, because nothing depends on the
  game's own asset formats for rendering.
- Nothing published carries anything but the contributor's own translated text, layout
  data, and glyphs generated from bundled fonts. No original game asset is ever
  redistributed.

---

## 2. The Eight Non-Negotiable Architectural Decisions

These are settled. They are not revisited during the build. Each is recorded here with the
reasoning that closed it, so that a future reader understands the cost of reopening it.

### Decision 1 — Never produce Unicode Presentation Forms

All contextual forms, all ligatures including lam-alef, and all diacritic placement come
from the font's own OpenType `GSUB` and `GPOS` tables through a real shaping engine. There
is no code path in Taarib that maps a codepoint to a presentation form, and no fallback
that does so when shaping fails. If shaping fails, that is a reported error, not a reason
to degrade.

**Rationale.** Presentation forms are a compatibility encoding with a fixed, incomplete
repertoire. Real fonts express joining through `init`/`medi`/`fina`/`isol`, required
ligatures through `rlig`, contextual alternates through `calt`, mark attachment through
`mark`/`mkmk`, and cursive attachment through `curs`. None of that survives a
presentation-form pipeline. Persian and Urdu, which share the script but not the joining
behaviour or the ligature set, are unrepresentable in it. Committing to real shaping once,
at the foundation, removes an entire permanent class of defects.

**Enforcement.** The rasterizer and every adapter accept glyph identifiers only —
never codepoints — from the layout stage onward. A codepoint cannot reach a draw call
without passing through the shaper, because the draw call has no parameter that accepts one.

### Decision 2 — The shaper is HarfRust

Not rustybuzz. Not a binding to native HarfBuzz. HarfRust is the shaping engine, and the
Arabic, Universal, and Default shapers it provides are the only path from characters to
glyphs in this product.

**Rationale.** HarfRust is the maintained pure-Rust HarfBuzz port under the same
organisation that maintains the font parsing stack it is built on. Choosing it removes the
native build toolchain, the cross-compilation burden for four target triples, and the
unsafe FFI surface a native HarfBuzz binding drags into a library that has to be embedded
inside other people's game processes. Pure Rust also means the shaping engine compiles to
WebAssembly unchanged, which is what makes the RPG Maker and Electron adapters possible
without shipping a second implementation.

### Decision 3 — The rasterizer is skrifa

**Rationale, and this is the whole reason:** skrifa and HarfRust parse fonts through the
same underlying `read-fonts` stack. Glyph identifiers produced by shaping are, by
construction, the same identifiers the rasterizer resolves outlines for. There is no
translation layer between the two, and therefore no possibility of the classic
mismatch bug where the shaper's glyph 4211 is a different glyph from the rasterizer's
glyph 4211 because the two libraries disagreed about face indices, collection offsets,
variation instances, or `CFF2` charstring numbering. That class of bug is silent, is
data-dependent, and reproduces only on specific fonts. Eliminating it structurally is
worth more than any feature another rasterizer offers.

### Decision 4 — One font byte buffer feeds both shaping and rasterization

A loaded font is a single shared, reference-counted, immutable byte buffer with one
identity. Shaping borrows it. Rasterization borrows it. Metrics queries borrow it. The
atlas records that identity in every glyph key. Nothing anywhere loads the same font file
twice into two independent parsers.

**Rationale.** It makes Decision 3's guarantee total rather than probable. It also removes
duplicate memory in a library that will be resident inside a game process where memory is
not ours to waste, and it makes the layout cache key sound: a font is identified by the
identity of its bytes, not by a path or a family name that could resolve differently later.

### Decision 5 — Never touch the game's fonts, font assets, or asset bundles for rendering

Taarib does not read the game's font assets, does not modify them, does not extend their
atlases, and does not use the engine's font asset system. It rasterizes its own glyphs from
its own bundled fonts, packs its own atlases, and builds its own meshes. Engine interaction
is limited to primitives that exist in every version of every engine: create a texture,
upload pixels, create a mesh, set vertices and UVs, create a material with a shader,
issue a draw.

**Rationale.** Asset bundles are serialized against the exact engine version that produced
them — down to type trees, field ordering, and container framing. Tools built on rewriting
those assets break on every engine version bump, and their maintainers spend their lives
chasing formats instead of improving the product. By refusing that dependency, Taarib's
rendering path works on a Unity version that does not exist yet.

The one place Taarib reads game assets at all is **text extraction** (Phase 12), which is
read-only, is never on the rendering path, and degrades to runtime string capture when a
container cannot be parsed.

### Decision 6 — Bundled fonts must carry complete Arabic OpenType tables

Every font shipped with Taarib is verified at build time and again at load time to carry
`GSUB` with the Arabic joining features, `GPOS` with mark attachment, and full coverage of
the Arabic ranges the product declares. User-supplied fonts pass the same validation and
are rejected on failure with a specific, readable reason naming the missing table or
feature.

**Rationale.** HarfRust has no Arabic fallback shaper — nothing synthesises joining when a
font lacks the tables. A font without them does not produce ugly output; it produces
*nothing*, or isolated forms, silently. That failure would surface to a user as "the patch
is broken" with no diagnosis. Validation converts a silent runtime failure into a loud,
specific, actionable one at the moment the font is chosen.

### Decision 7 — The registry is a public Git repository, not a custom server

Patch metadata lives as sharded JSON in a public Git repository. Patch binaries live as
release assets. Distribution is over the raw content endpoint plus a CDN mirror and a
secondary forge mirror.

**Rationale.** Free to operate at any scale the product will ever reach. Nothing to keep
running, nothing to pay for, nothing to lose when interest lapses. Every change is a commit:
the entire history of every patch and every review decision is public and permanent.
Anyone can fork the whole catalogue and keep it alive without asking permission. No
database to breach, no account system to leak, no single point of control over the data.
The client is designed for it — sharding and a hash manifest mean a launch costs one small
request when nothing has changed.

### Decision 8 — Nothing reaches the public source without the owner's signature

There is no automatic publishing path. Not for clean automated checks, not for trusted
contributors, not for the owner's own translations. Every published patch is signed with
the owner's Ed25519 key at the moment of approval, and clients refuse anything unsigned,
mismatched, or revoked.

**Rationale.** The product asks users to let it modify files inside games they paid for.
That trust is the entire product, and it is not divisible. A single reviewed signing
chokepoint is the only structure where the promise "if Taarib installed it, a human
approved it" stays true. The owner's own work goes through the same submission object so
that provenance and signature are uniform across the catalogue with no privileged path.

---

## 3. Coverage Matrix

Everything listed here ships in this build. There is no later.

### 3.1 Operating systems

| OS | Minimum | Architectures | Notes |
| --- | --- | --- | --- |
| Windows | 10 1809 (build 17763) | x86_64, aarch64 | Primary platform. 32-bit game processes supported by a 32-bit build of the injected native library. |
| Linux | glibc 2.31 / any current distro | x86_64, aarch64 | Includes Steam Deck (SteamOS 3, immutable root, patches written to user-writable paths only). |
| macOS | 12 Monterey | x86_64, aarch64 | Native games and games under Rosetta 2. Injection constrained by hardened runtime and SIP; capability probe reports which tier is reachable per game, honestly. |

**Proton and Wine.** On Linux, the application understands prefix structure. It resolves
`compatdata/<appid>/pfx`, maps `Z:\` and drive letter mappings to real paths, reads and
writes `user.reg`/`system.reg` for DLL overrides, installs the correct Windows-side
framework into the Windows-side game directory inside the prefix, and sets launch options
through the launcher's own configuration rather than wrapper scripts where possible.
Heroic, Lutris, Bottles and bare `WINEPREFIX` layouts are all recognised.

**32-bit games.** Both native libraries (`i686` and `x86_64`) ship on Windows and Linux, and
the injector selects by the target process image, not by the host.

### 3.2 Game sources

| Source | Discovery method | Platforms |
| --- | --- | --- |
| Steam | `libraryfolders.vdf` across all drives, then `appmanifest_*.acf` per library; `appinfo.vdf` for categories, launch options and anti-cheat hints; `localconfig.vdf` for user launch options | Win, Linux, macOS |
| Epic Games Store | `.item` manifests in the launcher's manifest directory; `LauncherInstalled.dat` | Win, macOS, Linux via Heroic/Legendary |
| GOG Galaxy | Galaxy's `galaxy-2.0.db` SQLite database; standalone installs via registry keys and `goggame-*.info` files | Win, macOS, Linux via Heroic |
| EA App / Origin | `local.xml` and the EA Desktop install registry; `__Installer/installerdata.xml` per game | Win |
| Ubisoft Connect | Registry `Uplay/Installs` keys and `configuration` manifests | Win |
| Battle.net | `product.db` protobuf and `.build.info` per product | Win, macOS |
| Xbox / Microsoft Store | `AppxManifest.xml` under `WindowsApps`, package family names, `GamingRoot` on secondary drives | Win |
| Itch.io | The itch app's `butler.db` SQLite database and its install locations | Win, Linux, macOS |
| Heroic Games Launcher | `installed.json` for Legendary and GOG, plus Heroic's own configuration for prefixes and wine builds | Linux, Win, macOS |
| Manual | User points at an executable; Taarib probes it exactly as it probes a discovered game | All |

### 3.3 Engines and injection tiers

| Engine | Detection | Tier | Adapter |
| --- | --- | --- | --- |
| Unity (Mono) | `*_Data/Managed/Assembly-CSharp.dll`, `UnityPlayer` module, `globalgamemanagers` version string | 1 — full takeover | BepInEx 6 Mono plugin, C# |
| Unity (IL2CPP) | `GameAssembly.dll`/`.so`, `*_Data/il2cpp_data/Metadata/global-metadata.dat` | 1 — full takeover | BepInEx 6 IL2CPP plugin, C# over Il2CppInterop |
| Unreal Engine 4 | `Engine/Binaries`, `*.pak`, `FEngineVersion` in the executable, `UE4Game` module | 1 — engine-native shaping forced on | Rust injected module + Slate hooks |
| Unreal Engine 5 | As UE4 plus IoStore `.utoc`/`.ucas` and UE5 version signature | 1 | Same adapter, UE5 code path |
| Godot 4 | `.pck` v2 header, `godot` executable metadata, `TextServerAdvanced` present | 1 — engine-native shaping enabled | Rust patcher + GDExtension |
| Godot 3 | `.pck` v1 header, no `TextServer` | 2 — full takeover | Rust injected module, glyphs drawn by Taarib |
| RPG Maker MV / MZ | `www/js/rpg_core.js` / `js/rmmz_core.js`, `package.json` for NW.js | 1 — data patch plus runtime script | Rust patcher + JavaScript runtime over WASM core |
| RPG Maker VX Ace | `Game.rgss3a`, `Game.ini` with `RGSS3` | 1 — archive patch plus Ruby injection | Rust patcher + Ruby script calling the C ABI |
| Ren'Py | `renpy/` directory, `*.rpa`, `lib/py*-*` | 1 — engine translation system plus Python hook | Rust generator + Python runtime module |
| GameMaker Studio | `data.win` / `game.unx` / `game.ios` FORM container | 1 — data patch with generated glyph pages | Rust patcher |
| Electron / web | `resources/app.asar`, `chrome_100_percent.pak`, Chromium modules | 1 — asar repack plus preload script | Rust patcher + TypeScript runtime over WASM core |
| Anything else | Fallback | 3 — overlay | Universal graphics layer, Rust |

**Tier meanings**, used verbatim in the capability report shown to the user:

- **Tier 1 — تعريب كامل (full Arabization).** Text is replaced inside the game. It looks
  native. Menus, dialogue, and interface all carry Arabic.
- **Tier 2 — تعريب بالرسم المباشر (Arabization by direct drawing).** Taarib draws the text
  itself over the engine's own text objects. Visually native in almost every case;
  effects the engine applies to its own text (outlines, gradients, per-character animation)
  are reproduced by Taarib's own renderer rather than the engine's.
- **Tier 3 — طبقة ترجمة (translation overlay).** The game is not modified. Taarib reads the
  screen and shows Arabic over it. Honest about what it is: it is a reading aid, not a
  patch, and the user is told exactly that before they enable it.

---

## 4. Component Map

### 4.1 The naming law

Rule 7 of the build governs every identifier in the product: **naming mirrors the domain.**
Taarib is a product about Arabic typography and Arabic game localization, and its modules
carry the names of the things they actually are. A module that resolves text direction is
`ittijah`, not `bidi_processor`. A module that packs glyphs into a texture is `lawha`, not
`atlas_manager`. There is no `utils`, no `helpers`, no `common`, no `manager`, no `service`,
no `handler`, and no `engine` anywhere in the tree.

The law in three parts:

1. **Crates, modules, files, and screens** take Arabic domain names in Latin transliteration.
2. **Types and functions** take the domain concept as its name — `KashidaOpportunity`,
   `ShapedRun`, `JoiningForm`, `RuqaaManifest`, `BuildFingerprint`. Arabic transliteration
   is used where the concept is specifically Arabic-typographic (`Tashkeel`, `Wasl`,
   `Rasm`, `Kashida`, `Ruqaa`, `Basma`); domain English is used where the concept is not.
3. **User-visible text** is Arabic first, English second, and never transliteration.

The canonical lexicon, used consistently across Rust, C#, TypeScript, Python, Ruby and
JavaScript:

| Term | Arabic | Meaning in this product |
| --- | --- | --- |
| `taarib` | تعريب | Arabization — the product itself |
| `saff` | الصف | Typesetting/composition — the core text engine |
| `nasq` | نسق | Markup and format placeholders |
| `ittijah` | اتجاه | Direction — the bidirectional algorithm |
| `wasl` | وصل | Joining — shaping and contextual forms |
| `taqtee` | تقطيع | Segmentation — line break opportunities |
| `kashida` | كشيدة | Elongation — justification by stretching |
| `tashkeel` | تشكيل | Diacritics and mark positioning |
| `rasm` | رسم | Drawing — rasterization and mesh generation |
| `khatt` | خط | Font — font resources, validation, families |
| `qiyas` | قياس | Measurement — metrics and widths |
| `lawha` | لوحة | Board — the glyph atlas |
| `jisr` | جسر | Bridge — the C ABI boundary |
| `usus` | أسس | Foundations — errors, logging, config, paths |
| `mustalahat` | مصطلحات | Vocabulary — the shared domain types |
| `makhzan` | مخزن | Store — the local SQLite database |
| `hijra` | هجرة | Migration — schema migrations |
| `barid` | بريد | Post — inter-process messaging |
| `kashf` | كشف | Discovery — finding installed games |
| `matjar` / `matajir` | متجر / متاجر | Store(s) — launcher adapters |
| `muharrik` | محرك | Engine — engine identification and capability probe |
| `haqn` | حقن | Injection — process injection and framework install |
| `muhawwil` | محوّل | Adapter — per-engine adapters |
| `nusus` | نصوص | Scripts — the script-engine family |
| `tabaqa` | طبقة | Layer — the universal graphics overlay |
| `istikhraj` | استخراج | Extraction — pulling strings out of games |
| `tarjama` | ترجمة | Translation — the machine translation pipeline |
| `masrad` | مسرد | Glossary |
| `dhakira` | ذاكرة | Memory — translation memory |
| `ruqaa` | رقعة | Patch — the package format (also a script style) |
| `tarqee` | ترقيع | Patching — the patch compiler |
| `tathbeet` | تثبيت | Installation |
| `taraju` | تراجع | Rollback |
| `aman` | أمان | Safety — anti-cheat and integrity refusal |
| `khatm` | ختم | Seal — signing, verification, revocation |
| `mustawda` | مستودع | Repository — the registry client |
| `fahras` | فهرس | Index — registry shards |
| `taqdeem` | تقديم | Submission |
| `muraja` | مراجعة | Review — the owner's console |
| `warsha` | ورشة | Workshop — the collaborative translation workspace |
| `maktaba` | مكتبة | Library — the landing screen |
| `luba` | لعبة | Game — the detail screen |
| `muayana` | معاينة | Preview — the live render preview |
| `musahamat` | مساهمات | Contributions |
| `talabat` | طلبات | Requests — the translation request board |
| `idadat` | إعدادات | Settings |
| `tashkhis` | تشخيص | Diagnostics |
| `bayan` | بيان | Manifest — the record of installed files |
| `basma` | بصمة | Fingerprint — content hashing |
| `nuskha` | نسخة | Copy — original file backups |

### 4.2 The components and how they connect

```
                         ┌──────────────────────────────────────────┐
                         │      Taarib Studio (Tauri desktop)       │
                         │  React + TypeScript  ⇄  Rust backend     │
                         └───────────────┬──────────────────────────┘
                                         │ typed commands & events
   ┌─────────────────────────────────────┼─────────────────────────────────────┐
   │                                     │                                     │
┌──▼────────┐  ┌──────────┐  ┌───────────▼──────┐  ┌──────────┐  ┌─────────────▼──┐
│ kashf     │  │ muharrik │  │ istikhraj        │  │ tarjama  │  │ mustawda       │
│ discovery │→ │ probe    │→ │ extraction       │→ │ pipeline │→ │ registry client│
└───────────┘  └────┬─────┘  └──────────────────┘  └────┬─────┘  └───────┬────────┘
                    │                                    │                │
                    │                          ┌─────────▼──────┐  ┌──────▼───────┐
                    │                          │ tarqee         │  │ taqdeem      │
                    │                          │ patch compiler │→ │ submit/review│
                    │                          └────────┬───────┘  └──────┬───────┘
                    │                                   │                 │
                    │                          ┌────────▼───────┐  ┌──────▼───────┐
                    │                          │ ruqaa (format) │  │ khatm (seal) │
                    │                          └────────┬───────┘  └──────────────┘
                    │                                   │
             ┌──────▼──────┐                   ┌────────▼────────┐
             │ aman        │──refuse/warn────→ │ tathbeet        │
             │ safety      │                   │ install/rollback│
             └─────────────┘                   └────────┬────────┘
                                                        │ writes into the game
   ══════════════════════════════════════════════════════▼══════════════════════════
                              THE GAME PROCESS
   ┌────────────────┐ ┌────────────────┐ ┌──────────┐ ┌────────┐ ┌────────────────┐
   │ Unity plugin   │ │ Unreal module  │ │ Godot    │ │ script │ │ tabaqa overlay │
   │ C# / BepInEx 6 │ │ Rust + Slate   │ │ Rust/GDE │ │ JS·Py·Rb│ │ D3D/GL/VK      │
   └───────┬────────┘ └───────┬────────┘ └────┬─────┘ └───┬────┘ └───────┬────────┘
           │ P/Invoke         │ direct        │ direct    │ ctypes/wasm  │ direct
           └──────────────────┴───────────────┴───────────┴──────────────┘
                                         │
                              ┌──────────▼───────────┐
                              │ jisr — stable C ABI  │
                              └──────────┬───────────┘
                                         │
                              ┌──────────▼───────────┐
                              │ saff — Arabic engine │
                              │ nasq → ittijah →     │
                              │ taqtee → wasl →      │
                              │ qiyas → kashida →    │
                              │ rasm                 │
                              └──────────┬───────────┘
                                         │
                              ┌──────────▼───────────┐
                              │ lawha — glyph atlas  │
                              └──────────────────────┘
```

### 4.3 Technology per component

| Component | Technology | Delivered as |
| --- | --- | --- |
| `saff` — Arabic text engine | Rust 2024 edition, HarfRust, skrifa, read-fonts, unicode-bidi, icu_segmenter, icu_normalizer, icu_properties | `rlib` |
| `lawha` — glyph atlas | Rust, etagere shelf allocator, in-crate exact Euclidean distance transform for SDF | `rlib` |
| `jisr` — native ABI | Rust `cdylib` + `staticlib`, cbindgen-generated header | `taarib_jisr.dll` / `libtaarib_jisr.so` / `libtaarib_jisr.dylib`, i686 and x86_64 and aarch64 |
| WASM core | Rust, wasm-bindgen | `taarib_core.wasm` + TypeScript wrapper package |
| Unity Mono adapter | C# 12, .NET Standard 2.1, BepInEx 6, HarmonyX | `Taarib.Unity.Mono.dll` |
| Unity IL2CPP adapter | C# 12, .NET 6, BepInEx 6 IL2CPP, Il2CppInterop | `Taarib.Unity.Il2cpp.dll` |
| Unreal adapter | Rust with `retour` detours, `windows`/`libc` platform bindings, minimal C++ vtable shim | injected module |
| Godot adapter | Rust; GDExtension C API shim for Godot 4 | injected module + `.gdextension` |
| Script adapters | Rust patchers; TypeScript→JS runtime for RPG Maker MV/MZ and Electron; Python module for Ren'Py; Ruby script for VX Ace | patch payloads |
| `tabaqa` — universal overlay | Rust; `windows` crate for D3D11/D3D12, `ash` for Vulkan, raw GL loader for OpenGL; `ocrs` + platform OCR | injected module |
| Desktop backend | Rust, Tauri 2, tokio, rusqlite (bundled SQLite), reqwest, ed25519-dalek, blake3, zstd | `taarib-studio` binary |
| Desktop frontend | TypeScript 5 strict, React 19, Vite, TanStack Router/Query/Virtual, Zustand, Radix primitives, Tailwind with a private token layer, Motion | bundled webview assets |
| Shared schema | Rust types as source of truth; `specta`/`tauri-specta` for TypeScript command bindings; `schemars` for JSON Schema of registry records | `schemas/*.json`, `apps/studio/src/mustalahat/*.ts` |
| Local data | SQLite in WAL mode with foreign keys enforced, versioned migrations in `hijra` | `taarib.db` |
| Registry | Public Git repository, sharded JSON, release-asset binaries, CDN and forge mirrors | `registry-template/` |

### 4.4 The four boundaries that matter

1. **`saff` knows nothing.** No games, no engines, no launchers, no graphics APIs, no
   filesystem beyond fonts handed to it as bytes, no network, no logging sink of its own.
   It is a library that turns text into positioned glyphs. It is buildable, usable, and
   valuable entirely on its own, and that constraint is what keeps it correct.
2. **`jisr` is the only way in from outside Rust.** C#, C++, Python, Ruby and JavaScript
   reach the engine through one narrow, versioned, ownership-explicit C ABI. There is no
   second entry point and no engine-specific extension of it.
3. **Adapters never make policy.** An adapter renders what a patch tells it to render. It
   does not decide whether a patch applies, does not check safety, does not talk to the
   network, and does not read the registry. All of that happened in Studio before the game
   ever launched.
4. **Studio never renders.** Every pixel of Arabic the user sees in a preview comes from
   `saff` through the same code path the game will use. The frontend does not reimplement
   shaping to draw a preview, because a preview that lies is worse than no preview.

---

## 5. Repository Layout

```
taarib/
├── ROADMAP.md                     this contract
├── README.md
├── LICENSE                        MPL-2.0 for the whole workspace — file-level copyleft
│                                  survives being loaded into a proprietary game process,
│                                  which a strong copyleft would not
├── Cargo.toml                     workspace root, shared lints and profiles
├── rust-toolchain.toml            pinned stable toolchain + rustfmt, clippy, targets
├── rustfmt.toml  clippy.toml  deny.toml  .editorconfig  .gitignore
│
├── crates/
│   ├── taarib-usus/               foundations
│   │   └── src/{lib,khata,sijill,idadat,masarat,mukhattat,manassa}.rs
│   ├── taarib-mustalahat/         shared vocabulary types + JSON Schema derivation
│   │   └── src/{lib,luba,muharrik,bina,ruqaa,nass,taghtiya,musahim}.rs
│   ├── taarib-saff/               PHASE 1 — the Arabic text engine
│   │   └── src/{lib,nasq,ittijah,taqtee,wasl,qiyas,kashida,tashkeel,rasm,khatt,satr,natija}.rs
│   ├── taarib-lawha/              PHASE 2 — glyph atlas pipeline
│   │   └── src/{lib,rasf,misafa,namu,tafrigh,khareeta}.rs
│   ├── taarib-jisr/               PHASE 3 — C ABI shared library
│   │   ├── src/{lib,hayat,dhakira,khazina,khata_c}.rs
│   │   ├── build.rs               cbindgen
│   │   └── include/taarib.h       generated, committed
│   ├── taarib-wasm/               WASM build of saff for JS-side adapters
│   ├── taarib-khatm/              signing, verification, revocation, key custody
│   ├── taarib-makhzan/            SQLite store, hijra migrations, repositories
│   ├── taarib-barid/              IPC: framed msgpack over named pipe / unix socket
│   ├── taarib-kashf/              PHASE 4 — discovery
│   │   └── src/matajir/{steam,epic,gog,ea,ubisoft,battlenet,xbox,itch,heroic,yadawi}.rs
│   ├── taarib-muharrik/           PHASE 5 — engine identification + capability probe
│   ├── taarib-haqn/               injection, framework installation, prefix handling
│   ├── taarib-muhawwil-unreal/    PHASE 8
│   ├── taarib-muhawwil-godot/     PHASE 9
│   ├── taarib-muhawwil-nusus/     PHASE 10 — script engines
│   ├── taarib-tabaqa/             PHASE 11 — universal graphics overlay
│   ├── taarib-istikhraj/          PHASE 12 — extraction
│   ├── taarib-tarjama/            PHASE 13 — translation pipeline
│   ├── taarib-ruqaa/              PHASE 14 — patch container format
│   ├── taarib-tarqee/             PHASE 14 — patch compiler
│   ├── taarib-tathbeet/           PHASE 15 — install, rollback, update survival
│   ├── taarib-aman/               PHASE 16 — safety
│   ├── taarib-mustawda/           PHASE 17 — registry client
│   ├── taarib-taqdeem/            PHASE 18 — submission and review
│   └── taarib-warsha/             PHASE 19 — collaborative workspace
│
├── apps/studio/                   PHASE 20 — Taarib Studio
│   ├── src-tauri/                 crate taarib-studio: commands, events, state, tray
│   ├── src/                       React app
│   │   ├── shashat/               screens: maktaba, luba, warsha, muayana, taqdeem,
│   │   │                          musahamat, muraja, tabaqa, idadat, tashkhis, talabat
│   │   ├── anasir/                components
│   │   ├── nizam/                 design system: tokens, primitives, motion, icons
│   │   ├── mustalahat/            generated TypeScript types and command bindings
│   │   ├── lugha/                 ar.json, en.json, formatting, direction
│   │   └── hayat/                 client state, query keys, keyboard map, command palette
│   ├── index.html  vite.config.ts  tsconfig.json  tailwind.config.ts
│   └── package.json
│
├── unity/                         Taarib.Unity.sln
│   ├── Taarib.Unity.Jisr/         P/Invoke surface, marshalling, safe handles
│   ├── Taarib.Unity.Mushtarak/    shared: patch loading, atlas upload, mesh building,
│   │                              nasq markup, container reflow, capture mode
│   ├── Taarib.Unity.Mono/         PHASE 6 — BepInEx Mono plugin + text-system takeovers
│   └── Taarib.Unity.Il2cpp/       PHASE 7 — BepInEx IL2CPP plugin + signature resolution
│
├── shims/                         minimal C++ only where a vtable or platform API demands
│   ├── d3d12_shim/  vulkan_layer/  il2cpp_shim/  objc_shim/
│
├── adapters-script/               PHASE 10 in-game sides
│   ├── rpgmaker/                  TypeScript → js/plugins/Taarib.js (MV, MZ)
│   ├── electron/                  TypeScript → preload + renderer runtime
│   ├── renpy/                     Python module: taarib_renpy/
│   └── vxace/                     Ruby: taarib_rgss3.rb
│
├── assets/fonts/                  bundled, validated, license files included
│   ├── naskh/     NotoNaskhArabic, Amiri
│   ├── kufi/      NotoKufiArabic, ReemKufi
│   ├── sans/      IBMPlexSansArabic, NotoSansArabic, Cairo, Tajawal
│   ├── latin/     IBMPlexSans, IBMPlexMono
│   └── khutut.json  the font manifest: family, style, weights, validation record
│
├── schemas/                       generated JSON Schema for every on-disk and registry record
├── registry-template/             the public registry repository skeleton
├── packaging/                     PHASE 22 — windows/, linux/, macos/, updater/
└── docs/                          format specifications, ABI reference, adapter notes
```

---

## 6. The Twenty-Four Phases

Each phase states its **scope**, the **components** it produces, its **technology**, its
**hard constraints**, its **dependencies**, and its **definition of done**. Phases run in
order. A phase is done when every line of its definition of done is true of the code on
disk.

---

### Phase 0 — Workspace Foundation

**Scope.** Establish the whole skeleton — Rust workspace, C# solution, frontend tree,
script adapter trees, embedded fonts, shared schemas — and define once, for use everywhere,
the six things that every later phase depends on: the error model, logging and diagnostics,
configuration, the on-disk user data layout, versioned data schemas, and the shared domain
vocabulary.

**Components produced.**

- `Cargo.toml` workspace with all crates declared, a single `[workspace.dependencies]`
  table pinning every third-party version once, shared `[workspace.lints]` denying
  `unwrap_used`, `expect_used` outside constant contexts, `panic`, `unsafe_op_in_unsafe_fn`,
  `missing_docs` on public items, and `clippy::pedantic` with a documented allow list.
  Release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` for the Studio
  binary and `panic = "unwind"` for every injected library, because a panic that unwinds
  across the ABI into a game process must be caught at the boundary rather than aborting
  someone's game.
- **`taarib-usus`** — foundations:
  - `khata` — the error model. One `Khata` enum per domain area, each variant carrying
    structured data, not a formatted string. Every variant has: a stable machine code
    (`TAARIB-E-1042`), an Arabic user-facing sentence, an English user-facing sentence, and
    a suggested next action. A `Khata` renders itself for a log, for a diagnostics bundle,
    and for a user, and those are three different renderings of the same value.
    `thiserror` for definition; no `anyhow` in library crates; a `Natija<T>` alias
    (`Result<T, Khata>`) used throughout.
  - `sijill` — logging and diagnostics. `tracing` with a JSON-lines file layer, a
    human layer, and an in-memory ring buffer the Diagnostics screen reads live. Spans
    carry game identity, patch identity, and phase so a log line is always attributable.
    Rotation by size and age. Redaction of API keys and user paths at the layer, not at
    call sites. The injected libraries log through the same crate to a per-game file and
    over `barid` to Studio when it is listening.
  - `idadat` — configuration. A typed settings tree with defaults, layered
    file → environment → command line, atomic write-and-rename saves, and a change
    notification channel. Secrets never live here; they live in the OS keychain via
    `keyring`.
  - `masarat` — the on-disk layout, resolved once per platform and never assembled by
    string concatenation anywhere else:

    ```
    Windows: %APPDATA%\Taarib\            macOS: ~/Library/Application Support/Taarib/
    Linux:   $XDG_DATA_HOME/taarib/       (config in $XDG_CONFIG_HOME/taarib/)
      taarib.db              local database
      idadat.json            settings
      ruqaa/                 downloaded and imported patches, content-addressed
      nusakh/                original-file backups, per game per install
      mashari/               translation projects (warsha)
      khutut/                user-supplied fonts, validated on import
      sijillat/              logs and diagnostic bundles
      dhakira/               translation memory databases
      makhbaa/               registry cache: shards, manifest, artwork
      sandooq/               sandbox verification workspace (owner only)
    ```

  - `mukhattat` — schema versioning. Every persisted structure carries
    `{ "mukhattat": 3 }`. A registry of migrations per structure, forward-only, with a
    refusal (not a guess) when a file declares a version newer than the running build.
  - `manassa` — platform abstraction: OS, architecture, elevation state, path length
    limits, case sensitivity, executable extension, dynamic library naming, process
    listing, and the Proton/Wine prefix model.
- **`taarib-mustalahat`** — the shared vocabulary. Every type here derives `Serialize`,
  `Deserialize`, `JsonSchema` (schemars) and `Type` (specta), so one definition produces the
  Rust type, the JSON Schema in `schemas/`, and the TypeScript type in
  `apps/studio/src/mustalahat/`. Nothing in the product defines these concepts twice.

  | Type | Meaning |
  | --- | --- |
  | `LubaId` | Taarib's own stable game identity: a UUIDv5 over the tuple of canonical source, source identifier, and normalized name |
  | `MasdarLuba` | Which launcher a game came from, with the launcher's own identifier preserved (`SteamAppId(u32)`, `EpicCatalogItemId(String)`, `GogProductId(u64)`, …) |
  | `Luba` | A discovered game: identity, all source identifiers it is known by, name, install root, executable, size, last played, artwork keys |
  | `Muharrik` | Engine identity: family, version triple, scripting backend, UI framework, graphics APIs, architecture, plus the evidence that produced the identification |
  | `BinaId` | Build identity: launcher build id where one exists, plus `Basma` |
  | `Basma` | Content fingerprint: BLAKE3 over a canonical, ordered selection of text-bearing files with their sizes, so a build that changed only shaders keeps its fingerprint |
  | `RuqaaId` | Patch identity: UUIDv7, stable across revisions of the same patch lineage, with a separate monotonic `RuqaaRevision` |
  | `NassId` | String identity: engine-scoped, deterministic, derived from source location and source text so extraction is reproducible |
  | `HalatTarjama` | Translation status: `LamTutarjam`, `Musawwada`, `LilMuraja`, `Muakkada`, `Mujammada` (untranslated, draft, needs review, approved, locked) |
  | `Taghtiya` | Coverage: strings total, translated, approved, weighted by occurrence count and by whether the string is player-visible in the first hour |
  | `Tabaqa` | Injection tier 1, 2 or 3 with the reason it was chosen |
  | `Musahim` | Contributor: public key fingerprint, display name, credit line, reputation counters |
- **Frontend skeleton** — Vite, React, TypeScript in strict mode with
  `exactOptionalPropertyTypes` and `noUncheckedIndexedAccess`, Tailwind configured with the
  Taarib token layer only (no default palette, no default spacing scale, no default
  radii — the defaults are deleted, not overridden), path aliases, and the generated
  bindings directory wired into the build.
- **C# solution** — `Directory.Build.props` pinning language version, nullable reference
  types enabled, deterministic builds, and the BepInEx package references; project stubs
  with their real assembly names and target frameworks.
- **Fonts** — the bundled set placed under `assets/fonts/` with `khutut.json` recording for
  each: family, style class (naskh/kufi/sans/latin/mono), weight range, whether it is
  variable, the OpenType Arabic features present, the Unicode ranges covered, the license
  file path, and the BLAKE3 of the font bytes.

**Technology.** Rust 2024 edition on a pinned stable toolchain; TypeScript 5 strict; C# 12;
schemars and specta for schema derivation; `tracing` for diagnostics; `directories` for
platform paths.

**Hard constraints.**

- No crate outside `taarib-usus` computes a path. Every path comes from `masarat`.
- No crate defines an error type that stringifies another error. Errors nest as values.
- No type describing a domain concept exists in TypeScript by hand. It is generated.
- Every third-party dependency version appears exactly once, in the workspace table.

**Dependencies.** None.

**Definition of done.** Every crate exists with its real module tree and compiles as an
empty-but-typed skeleton. `schemas/` contains a generated JSON Schema for every vocabulary
type. `apps/studio/src/mustalahat/` contains the generated TypeScript. The font manifest is
complete and every listed font file is present with its license. No `TODO` exists anywhere
in the tree.

---

### Phase 1 — Taarib Core: The Arabic Text Engine

**Scope.** `taarib-saff` — a pure Rust library that turns logical-order text into ordered
lines of positioned glyphs, correctly, for Arabic, Persian, Urdu, mixed Arabic-Latin and
pure Latin. It knows nothing about games, engines, launchers, graphics APIs, or files. It
is the heart of the product and it must stand entirely alone.

**The signature.**

```
Input:  logical-order text
        + a KhattResource (font bytes, shared, validated)
        + pixel size
        + available width (and optional available height)
        + style spans
        + options (direction base, justification mode, features, language)

Output: ordered visual lines of positioned glyphs, each glyph carrying:
        glyph id, x, y, advance, the cluster index back into the logical text,
        and the style span id it belongs to
```

**The pipeline, in exactly this order.** Each stage is a module; each is independently
callable; none skips ahead.

1. **`nasq` — markup and placeholder extraction.** Parse the input for the markup dialects
   the product must survive: Unity rich text (`<b>`, `<i>`, `<size=…>`, `<color=#…>`,
   `<sprite=…>`, `<link=…>`, `<voffset>`, `<cspace>`, `<mspace>`, `<indent>`, `<align>`,
   `<nobr>`, `<noparse>`), Unreal's rich text (`<Style>text</>`), BBCode for Godot and
   Ren'Py, and format placeholders in every dialect games actually use: `{0}`, `{name}`,
   `%s`, `%1$d`, `%d`, `$variable`, `[color]`, `\n`, `\t`. Output is *clean text* plus a
   span table plus an *atomic replacement table* for placeholders. Placeholders become
   single opaque atoms with a measured width; markup never enters directional processing,
   never enters shaping, and never becomes a glyph.
2. **`ittijah` — the Unicode Bidirectional Algorithm, complete.** Paragraph level from the
   first strong character or an explicit base override. Explicit embedding and override
   controls (LRE/RLE/LRO/RLO/PDF) and isolates (LRI/RLI/FSI/PDI) with the full depth stack.
   Weak type resolution: European numbers, Arabic numbers, Arabic-Indic digits, European
   separators and terminators, common separators. Neutral resolution by surrounding strong
   context. Bracket pair resolution over `BD16` with mirroring applied at the glyph level
   through the font's own mirrored characters, never by swapping codepoints blindly.
   Digit shaping policy is explicit and per-patch: keep European digits, map to Arabic-Indic
   (٠-٩), or map to Eastern Arabic-Indic (۰-۹) for Persian and Urdu — a per-locale decision
   that translators control and that is recorded in the patch.
3. **`taqtee` — line break opportunities on the logical text**, by UAX #14 through
   `icu_segmenter`, before any reordering, because break opportunities are a property of the
   logical text and computing them on visual order is a category error. Grapheme cluster
   boundaries and word boundaries are computed here too, because the workspace needs them
   for cursor movement and the game needs them for typewriter effects.
4. **`wasl` — shaping through HarfRust.** One shaping call per direction-and-style run.
   Features enabled by script and language: `init`, `medi`, `fina`, `isol`, `rlig`, `liga`,
   `calt`, `mark`, `mkmk`, `curs`, `ccmp`, and `kern` where present; `locl` driven by the
   patch's declared language so Persian and Urdu get their correct local forms of `kaf`,
   `yeh`, and `heh`. Font variations applied here when the font is variable, once, so that
   metrics and outlines agree. The shaped run carries glyph ids, advances, offsets, cluster
   map, and the joining information `kashida` will need.
5. **`qiyas` — measurement and reflow.** Candidate lines are measured from **real shaped
   advances**, never estimated from character counts, never from an average character
   width, never from a monospace assumption. When a candidate overflows the available
   width the line is broken at the last opportunity `taqtee` offered and the tail is
   reshaped — because breaking a run changes its joining behaviour at the break, and a
   measurement that ignores that is wrong. Overflow with no break opportunity is handled by
   the declared policy: shrink to fit within a floor, ellipsize with the correct Arabic
   ellipsis behaviour, or report overflow and let the caller decide.
6. **`ittijah::tarteeb` — visual reordering per line.** The L2 reordering rule applied to
   the runs of each line after breaking, producing visual order. Runs are reordered; glyphs
   inside a run are already in visual order from the shaper. Line-final whitespace is
   handled per L1.
7. **`kashida` — justification.** Arabic justifies by elongating letters, not only by
   stretching spaces. Elongation points are identified from **shaped joining behaviour**:
   a candidate is a position between two glyphs that are cursively joined and whose
   preceding glyph's joining form permits elongation, ranked by the classical priority
   order (before final `reh`-group letters, after `kaf`/`lam`, before `dal`-group, medial
   connections, and so on down the ladder). Surplus width is distributed across candidates
   by rank with a per-point maximum, the run is **reshaped** with the elongation applied
   through the font's own `tatweel`-aware substitution and positioning rather than by
   inserting U+0640 blindly where the font has a `jstf` table or elongation-aware
   alternates, and the result is re-measured. Space-based justification remains available
   and is the only mode used for Latin runs. Modes: none, spaces only, kashida only,
   kashida then spaces (the default for Arabic).
8. **`tashkeel` — diacritics** are not a separate stage but a guarantee across stages: marks
   are carried as their own glyphs with `mark`/`mkmk` positioning from `GPOS`, they never
   participate in line breaking or justification as if they were letters, their advances are
   zero and never contribute to width, and stacked marks resolve by the font's attachment
   points. A `tashkeel` policy per patch controls whether source diacritics are preserved,
   stripped for interface text where they cost width, or preserved only in dialogue.
9. **`rasm` — rasterization through skrifa.** Antialiased 8-bit coverage bitmaps, and signed
   distance fields, selectable per patch. Hinting off, gamma-correct coverage, subpixel
   positioning quantized to a configurable number of x-positions so the cache stays finite.
   Outlines come from the same font buffer that shaped, resolved by the same glyph id, with
   the same variation coordinates applied.

**Also in this phase.**

- `khatt` — font resources: load from bytes, validate against Decision 6 (required tables,
  required features, declared coverage), report rejection reasons in plain Arabic and
  English, expose family/style/weight/variation axes, handle font collections and named
  instances, and hold the single shared byte buffer of Decision 4. A fallback *chain*
  (not a fallback shaper) resolves characters absent from the primary font to the next font
  in the chain, per run, with the chain recorded in the layout so the atlas builder knows
  which font each glyph came from.
- `qiyas` public queries: string width at a size, line height and baseline metrics, ascent,
  descent, gap, cap height, x-height, and the width of any substring — all from real shaping.
- Script detection over the input (Arabic, Latin, mixed, Persian and Urdu discrimination via
  characteristic characters and declared language) and normalization (NFC by default,
  with Arabic-specific cleanup: tatweel stripping when policy says so, alef-form
  unification for search, and presentation-form *decomposition* to canonical characters when
  a translator pastes text from a legacy source — the one place presentation forms are ever
  read, and they are immediately destroyed).
- A layout result type designed for reuse: it can be serialized into a patch (Phase 14),
  handed across the ABI (Phase 3), or walked to build a mesh (Phase 6) without
  transformation.

**Technology.** Rust 2024. HarfRust for shaping, skrifa and read-fonts for outlines and
metrics, `unicode-bidi` for UBA, `icu_segmenter` for UAX #14 and grapheme/word boundaries,
`icu_properties` for joining types and script, `icu_normalizer` for normalization,
`tinyvec`/`smallvec` for run and glyph buffers to keep the common case allocation-free.

**Hard constraints.**

- No presentation forms produced, ever, by any path (Decision 1).
- No shaper other than HarfRust; no rasterizer other than skrifa (Decisions 2 and 3).
- One font buffer, shared, for shaping and rasterization (Decision 4).
- No `std::fs`, no `std::net`, no global state, no `unsafe` outside the two byte-cast
  helpers that are documented and locally proven.
- The crate compiles to `wasm32-unknown-unknown` unchanged.
- Markup and placeholders never reach `ittijah` or `wasl`.
- Every public function returns `Natija<T>`; nothing panics on malformed input, including
  lone surrogates, unpaired isolates, unterminated markup, and fonts that are lying about
  their own tables.

**Dependencies.** Phase 0.

**Definition of done.** The full pipeline runs end to end for: pure Arabic, Arabic with
embedded Latin, Arabic with European digits, Arabic with Arabic-Indic digits, Persian, Urdu,
text with nested brackets, text with mandatory lam-alef, text with stacked diacritics, text
with Unity rich-text markup and `{0}`-style placeholders, and text requiring kashida
justification into a fixed width. Fonts lacking Arabic tables are rejected with a named
reason. Metrics queries return real shaped widths. Both coverage and SDF rasterization
produce bitmaps. The crate has no dependency on anything outside Phase 0 and the listed
third-party libraries.

---

### Phase 2 — Glyph Atlas Pipeline

**Scope.** `taarib-lawha` — turn a required glyph set into packed textures, offline for
patch compilation and online for text that could not be known ahead of time.

**Components produced.**

- `tafrigh` — glyph set collection. The required set is produced by **shaping the real
  strings** of the patch at the real sizes the game uses, and collecting the glyph ids that
  come out. Never by enumerating Unicode ranges, because a range enumeration both misses
  every ligature and contextual alternate the font would have produced and includes
  thousands of glyphs the game will never draw. Collection is keyed by
  `(font identity, glyph id, pixel size, rasterization mode, subpixel bucket)`.
- `rasf` — packing. Shelf packing via `etagere` with configurable padding, a power-of-two
  or tight-fit page policy, a maximum page dimension respecting the lowest common
  denominator across target GPUs (4096 default, 2048 for the conservative profile), and
  multi-page output when one page cannot hold the set. Deterministic: the same input set
  produces byte-identical pages, so a recompiled patch does not churn its own hash.
- `misafa` — signed distance field generation. Exact Euclidean distance transform
  (Felzenszwalb–Huttenlocher, two-pass, over the high-resolution coverage bitmap) with a
  configurable spread in pixels, encoded to 8-bit with the zero level at 128. SDF pages let
  one atlas serve every size the game asks for, which is what makes auto-sizing text and
  world-space text work; coverage pages are sharper at fixed small sizes and are the default
  for interface text. The choice is per patch, recorded in the patch, and the adapter obeys
  it without asking.
- `khareeta` — the glyph map. For every packed glyph: page index, rectangle in pixels,
  normalized UV rectangle, bearing x and y, advance, the pixel size and mode it was
  rasterized at, and the font identity it came from. Serialized in the patch's fixed-layout
  binary form so that C#, JavaScript and Rust all read it by struct overlay with no parser.
- `namu` — runtime growth. For strings the compiler never saw — player names, dynamically
  composed text, runtime capture — the atlas grows at runtime: allocate into free shelf
  space, add a page when full up to a page budget, and evict by least-recently-used glyph
  *rectangles* (not whole pages) when the budget is reached, with the eviction reflected in
  the glyph map atomically so no frame ever samples a rectangle that has been reassigned.
  Growth events are logged and counted, because a patch whose atlas grows constantly is a
  patch whose compiler missed strings, and the diagnostics say so.

**Technology.** Rust; `etagere` for shelf allocation; in-crate EDT for SDF; `zstd` for page
compression inside the patch container.

**Hard constraints.**

- Glyph identity is `(font identity, glyph id)` and nothing else — never a codepoint.
- Pages are single-channel R8. Colour comes from the material, per style span, at draw time.
- No page exceeds the declared maximum dimension; the packer splits rather than scales.
- Runtime eviction never invalidates a glyph that is referenced by the frame being built.

**Dependencies.** Phase 1.

**Definition of done.** A glyph set collected from real shaped strings packs into
deterministic pages in both modes, with a complete glyph map, and the runtime allocator
grows and evicts correctly under a page budget.

---

### Phase 3 — Native ABI Layer

**Scope.** `taarib-jisr` — wrap the engine in a stable C ABI shared library that C#, C++,
Python, Ruby and JavaScript call into, designed for a game loop that cannot afford
allocation.

**Components produced.**

- **The ABI surface**, versioned and frozen: `taarib_abi_version()` returns a
  major/minor pair; a caller with a mismatched major refuses to load and says so. Every
  function is `extern "C"`, `#[no_mangle]`, returns an `int32` status code, and writes
  results through out-parameters. No Rust type crosses the boundary. No `panic` crosses the
  boundary: every entry point wraps its body in `catch_unwind` and converts a panic into a
  status code plus a logged diagnostic, because a panic unwinding into a game's C# or C++
  frame is a crash the user will blame on the game.
- **Handles.** `TaaribContext`, `TaaribKhatt`, `TaaribLawha`, `TaaribLayout` are opaque
  pointers with explicit create/destroy pairs, generation-tagged so a use-after-free is
  detected and reported rather than executed. Ownership is documented per function in
  `include/taarib.h` and in `docs/abi.md`: who allocates, who frees, how long a returned
  pointer is valid, and whether it is invalidated by the next call.
- **The hot path.** `taarib_layout_into(ctx, request, buffer)` writes positioned glyphs into
  a caller-owned, reusable buffer and allocates nothing. If the buffer is too small it
  returns the required capacity and writes nothing — the caller grows once and retries.
  There is no allocating variant of this function, so no adapter can accidentally use one.
- **`khazina` — the layout cache.** Keyed by the hash of `(text, font identity, pixel size,
  available width, feature set, style span shape, justification mode, language)`. LRU with a
  byte budget, not an entry count, because a cache measured in entries is a cache that will
  eventually hold a thousand paragraphs. Hit and miss counters exposed for diagnostics. The
  cache stores the finished layout, so a menu that redraws every frame shapes once.
- **Streaming and capture.** Entry points for the adapters' runtime string capture
  (Phase 12) and for atlas growth callbacks (Phase 2), both non-blocking, both safe to call
  from a render thread.
- **`include/taarib.h`** — generated by cbindgen at build time, committed to the repository
  so consumers do not need the Rust toolchain, with hand-authored documentation comments on
  every symbol.
- Thin idiomatic wrappers where a language deserves one: `Taarib.Unity.Jisr` (C#, safe
  handles and spans), `taarib_renpy/jisr.py` (ctypes), `taarib_rgss3.rb` (Win32API),
  and the WASM package for JavaScript — which does not go through this ABI at all but
  exposes the same shape, so that adapter code reads the same in every language.

**Technology.** Rust `cdylib` and `staticlib`; cbindgen; `catch_unwind` at every boundary.
Built for `x86_64` and `aarch64` on all three platforms and `i686` on Windows and Linux,
because a 32-bit game process needs a 32-bit library.

**Hard constraints.**

- No allocation in `taarib_layout_into`. This is verifiable by inspection and is enforced by
  routing that path through a bump allocator scoped to the call.
- No global mutable state except the cache, which is behind a lock-free reader path.
- Thread safety stated per function and honoured: a context is `Send`, layout is safe from
  any single thread at a time, and font resources are shared immutably across contexts.
- The header is generated, never hand-edited.

**Dependencies.** Phases 1, 2.

**Definition of done.** The shared library builds for every target triple, exposes the full
surface, generates its header, catches panics at every entry, serves cached layouts, and the
four language wrappers exist and are complete.

---

### Phase 4 — Game Discovery

**Scope.** `taarib-kashf` — find every game installed on the machine, from every source,
on every platform, with artwork, and keep that view fresh.

**Components produced.**

- `matajir::steam` — the primary path, and the most thorough. Locate Steam from the registry
  on Windows, from `~/.steam/steam` and the Flatpak path on Linux, and from
  `~/Library/Application Support/Steam` on macOS. Parse `libraryfolders.vdf` (both the
  legacy and current shapes) to enumerate every library across every drive, then every
  `appmanifest_*.acf` for app id, name, install directory, `buildid`, `LastUpdated`,
  `SizeOnDisk`, and `StateFlags` so that a partially downloaded game is not offered as
  installable. Parse the binary `appinfo.vdf` for launch options, categories (which reveal
  single-player, multiplayer, and anti-cheat associations), and the on-disk `depots` for
  branch identity. Parse `localconfig.vdf` for the user's own launch options so Taarib can
  extend rather than overwrite them. Read `shortcuts.vdf` for non-Steam games added to the
  library — which is how most Steam Deck users run everything else.
- `matajir::epic`, `::gog`, `::ea`, `::ubisoft`, `::battlenet`, `::xbox`, `::itch`,
  `::heroic` — each implementing the same trait, each reading that launcher's real
  catalogue format as listed in the coverage matrix, each degrading to "launcher not
  installed" rather than failing the scan.
- `matajir::yadawi` — manual addition by executable path, which produces a `Luba` identical
  in every respect to a discovered one so that nothing downstream treats it as second class.
- `beea` — the Proton and Wine environment resolver: for a Steam game with a compatibility
  tool, locate `steamapps/compatdata/<appid>/pfx`, read `user.reg` and `system.reg`, build
  the drive-letter map from `dosdevices`, and expose bidirectional translation between
  Windows paths as the game sees them and real paths as the host sees them. The same for
  Heroic, Lutris, Bottles, and a bare `WINEPREFIX`. Identify the Proton build in use,
  because DLL override syntax and the correct injection method depend on it.
- `suwar` — artwork. Read the local caches first: Steam's `librarycache` grid, capsule and
  hero images, GOG's Galaxy image cache, Epic's manifest artwork URLs, Xbox's package logo.
  Fetch what is missing from the launcher's own public artwork endpoints, store
  content-addressed under `makhbaa/`, and extract the dominant colour at import time so the
  interface has it before the image finishes decoding.
- `tahdith` — incremental refresh. A scan is diffed against the database: new games inserted,
  moved games updated, uninstalled games marked absent rather than deleted (so a patch record
  survives a reinstall), and build id changes flagged for the update-survival path in
  Phase 15. Filesystem watches on the library roots make the refresh reactive rather than
  polled, with a debounce, and a full rescan is always available from the interface.

**Technology.** Rust; `keyvalues-parser` shapes for VDF text and a hand-written reader for
binary VDF; `rusqlite` to read GOG and itch databases directly; `quick-xml` for EA and
Ubisoft manifests; `prost` for Battle.net's protobuf `product.db`; `windows` crate for
registry and package enumeration; `reqwest` for artwork.

**Hard constraints.**

- A malformed or unexpected catalogue file degrades that one entry, never the scan.
- No launcher is required to be running, and Taarib never launches one to discover.
- Discovery is read-only. Nothing in this phase writes into a game directory or a launcher's
  configuration.
- Every discovered game gets a `LubaId` that is stable across rescans, reinstalls, and
  moves between drives.

**Dependencies.** Phase 0.

**Definition of done.** A scan on each platform enumerates every installed game from every
listed source with install path, build id, size, and artwork; Proton and Wine prefixes
resolve to real paths in both directions; manual addition works; rescans are incremental and
stable.

---

### Phase 5 — Engine Identification and Capability Probe

**Scope.** `taarib-muharrik` — for every discovered game, determine exactly what it is
built with and exactly what Taarib can do to it, and say so in language a player
understands. This report drives every downstream dispatch decision in the product.

**Components produced.**

- `dalail` — evidence collection, four independent sources, each producing evidence rather
  than a conclusion:
  1. **Directory shape.** `*_Data/`, `Engine/Binaries/`, `renpy/`, `www/js/`, `data.win`,
     `resources/app.asar`, `Game.rgss3a`, `*.pck`, `*.pak`, `*.utoc`.
  2. **Binary signatures.** PE/ELF/Mach-O parsing for imported modules (`UnityPlayer`,
     `GameAssembly`, `mono-2.0-bdwgc`, `d3d11`, `d3d12`, `vulkan-1`, `opengl32`), section
     names, embedded version strings, and the Unreal `FEngineVersion` structure.
  3. **Embedded metadata.** Unity's `globalgamemanagers`/`data.unity3d` version header,
     IL2CPP's `global-metadata.dat` header version, Godot's PCK header magic and version,
     Ren'Py's `renpy/__init__.py` version tuple, NW.js and Electron version from their
     resources, GameMaker's FORM `GEN8` chunk version.
  4. **Asset container headers.** UnityFS signature and compression flags, `.pak` footer
     magic and version, `.utoc` header, `.pck` v1/v2, `.rgss3a` key.
- `tahdid` — resolution. Evidence is weighted and combined into a `Muharrik` with a
  confidence value and the evidence retained. Conflicting evidence is reported, not
  averaged. An unknown engine is a first-class answer, not a failure.
- `imkaniyat` — the capability report, which is a *product artifact*, not a debug dump. For
  each game it states:
  - Engine family and version, scripting backend, UI framework(s) detected, graphics APIs
    present, process architecture, and whether the game runs under a compatibility layer.
  - The **text systems reachable**: TextMeshPro, Unity UI Text, NGUI, FairyGUI, TextMesh,
    Slate, Godot `Label`/`RichTextLabel`/`Control`, RPG Maker `Window_Base`, Ren'Py `Text`,
    GameMaker `draw_text`, DOM text.
  - The **tier** that applies and why.
  - The **expected quality** in plain Arabic: what will be Arabized, what will not, and what
    might look different from the original.
  - The **limitations the user will meet**, named specifically — for example: "النصوص
    المرسومة داخل الصور لن تُترجم" (text baked into images will not be translated), or
    "هذه اللعبة تستخدم نظام نصوص غير معروف؛ سيُستخدم أسلوب الطبقة" (this game uses an
    unrecognised text system; the overlay tier will be used).
- `bitaqa` — the per-game record persisted to the database with the probe's version stamp,
  so that a Taarib update which improves detection re-probes automatically rather than
  serving a stale conclusion.

**Technology.** Rust; `object`/`goblin` for PE, ELF and Mach-O; hand-written readers for the
engine container headers; no execution of the game and no loading of its code.

**Hard constraints.**

- Probing is read-only, bounded in time, and never loads a game module into Taarib's own
  process.
- Every conclusion carries its evidence; the diagnostics bundle includes it verbatim.
- The report is written for a player first. The technical detail is present but secondary.
- An unknown engine resolves to tier 3 with an honest explanation, never to a guess.

**Dependencies.** Phases 0, 4.

**Definition of done.** Every engine in the coverage matrix is identified from a real
installation with version and backend; the capability report renders in Arabic and English
for every tier; unknown engines produce a correct tier-3 report; results persist and
re-probe on probe-version change.

---

### Phase 6 — Unity Mono Adapter

**Scope.** `unity/Taarib.Unity.Mono` — a BepInEx 6 plugin that takes over text rendering for
any text object whose content contains Arabic, in games whose managed code runs on Mono.

**Components produced.**

- `Taarib.Unity.Jisr` — the P/Invoke surface over the C ABI: `DllImport` declarations,
  `SafeHandle` subclasses for every opaque handle, `Span<T>`-based buffer reuse, and a
  strict rule that no managed allocation happens inside a layout call. A single static
  initializer loads the correct architecture's library from the patch directory and refuses
  to continue on ABI major mismatch, with a message in the BepInEx log the user can read.
- `Taarib.Unity.Mushtarak` — everything shared with the IL2CPP adapter: patch loading and
  memory-mapping of the `.ruqaa` container, atlas upload into `Texture2D` with `R8` format
  and no mipmaps, material and shader setup (one shader for coverage, one for SDF, both
  authored in this repository, both compiled to a `.shader` asset created at runtime through
  `Shader.Find` fallbacks and an embedded compiled variant so no asset bundle is required),
  the `nasq` markup bridge, container reflow, and the runtime string capture channel.
- **Text system takeovers**, each a separate patch set, each activated only when its type
  exists in the loaded assemblies:
  - **TextMeshPro** (`TMP_Text`, both `TextMeshPro` and `TextMeshProUGUI`). Intercept
    `GenerateTextMesh` / the `TMP_Text.ForceMeshUpdate` path and the internal
    `TMP_TextInfo` population. When the resolved string contains Arabic, the engine's own
    layout is prevented from running and Taarib supplies the mesh: vertices, UVs, colours,
    and triangle indices built directly from the layout returned by `jisr`, written into the
    `TMP_MeshInfo` arrays the component already owns, with the canvas renderer's material
    swapped to Taarib's atlas material for that submesh. Rich text tags are handled as
    `nasq` spans, so `<color>`, `<b>`, `<i>`, `<size>` and `<sprite>` continue to work,
    including inline sprites which are placed as atomic runs with the sprite asset's own
    metrics.
  - **Unity UI Text** (`UnityEngine.UI.Text`) and `TextGenerator`. Same takeover at the
    `Text.OnPopulateMesh` level via a Harmony prefix that fills the `VertexHelper` from
    Taarib's layout and returns false.
  - **NGUI** (`UILabel`), **FairyGUI** (`TextField`, `GTextField`), and world-space
    `TextMesh` — each with its own mesh-population interception, each falling back cleanly
    to the engine's own rendering when the string has no Arabic.
- **Behavioural fidelity**, which is what separates a takeover that works from one that
  merely renders:
  - **Input fields** (`TMP_InputField`, `InputField`): caret position mapped through the
    layout's cluster indices so the caret lands between graphemes in visual order while the
    underlying string stays in logical order; selection rectangles computed per visual run,
    which means a selection spanning a direction boundary correctly draws as two rectangles;
    text insertion and deletion at logical positions.
  - **Scrolling text and typewriter effects**: `maxVisibleCharacters` and
    `maxVisibleWords` semantics reinterpreted over grapheme clusters in logical order, so an
    Arabic sentence reveals itself letter by letter from the right, with joining preserved at
    every step — which requires reshaping the visible prefix each step, and is done from the
    cache so it costs nothing.
  - **Auto-sizing**: the engine's binary search over font size is replaced by a search over
    Taarib's real measured widths, converging on the same contract (`fontSizeMin`,
    `fontSizeMax`, `enableAutoSizing`) with correct results.
  - **Container reflow**: when translated Arabic needs more room than the original,
    `RectTransform` sizes and anchors are adjusted per the patch's per-string layout hints —
    grow width toward the leading edge, grow height for wrapped text, re-anchor mirrored for
    right-to-left interface direction — and every adjustment is recorded so it can be undone
    when the patch is disabled at runtime.
  - **Right-to-left interface mirroring** for the elements a patch marks as mirrorable:
    anchor flips, layout group reversal, and horizontal alignment inversion.
- `iltiqat` — runtime capture mode: every string that reaches a takeover point is reported
  over `barid` with its component path, scene, screen position, font size, and the rect it
  was drawn into. This is the input to Phase 12's runtime capture and to Phase 14's
  per-size compilation.

**Technology.** C# 12 targeting .NET Standard 2.1, BepInEx 6, HarmonyX for patching, unsafe
spans for mesh writing.

**Hard constraints.**

- No game font, font asset, `TMP_FontAsset`, `SDF Atlas`, or asset bundle is read, modified,
  or created (Decision 5). Taarib's atlas is uploaded as a plain `Texture2D` from raw bytes.
- No managed allocation on the per-frame path. Buffers are pooled and reused.
- A failure to resolve or patch a target degrades that one text system, logs a specific
  reason, and leaves the game running in its original language.
- Reflection over game types is resolved once at startup and cached as delegates; no
  reflection on the render path.

**Dependencies.** Phases 3, 5, 14 (for the container format it reads).

**Definition of done.** Arabic renders correctly, joined, ordered and justified, in
TextMeshPro, Unity UI Text, NGUI, FairyGUI and world-space TextMesh; rich text, input
fields, typewriter effects and auto-sizing all behave; containers reflow; capture mode
streams strings; nothing reads a game asset.

---

### Phase 7 — Unity IL2CPP Adapter

**Scope.** `unity/Taarib.Unity.Il2cpp` — the same takeover for games whose managed code has
been compiled ahead of time into a native binary, where there is no `Assembly-CSharp.dll` to
patch and no managed metadata to reflect over in the usual way.

**Components produced.**

- The IL2CPP plugin built on BepInEx 6's IL2CPP runtime and Il2CppInterop, sharing
  `Taarib.Unity.Mushtarak` and `Taarib.Unity.Jisr` unchanged with the Mono adapter — the
  takeover logic is written once and compiled twice.
- `hall` — the resolution layer, which is the real work of this phase. Finding
  `TMP_Text.GenerateTextMesh` in a stripped native binary is a three-step ladder:
  1. **Generated metadata.** Il2CppInterop's generated assemblies resolve the method
     directly when `global-metadata.dat` is present and its version is supported.
  2. **Metadata-derived pointer scan.** When the metadata is present but the interop layer
     cannot bind the exact overload, resolve through the IL2CPP runtime exports
     (`il2cpp_class_from_name`, `il2cpp_class_get_method_from_name`) directly, which works
     across far more versions than generated bindings do.
  3. **Binary signature scanning.** When metadata is encrypted, stripped, or of an
     unrecognised version — which is common in shipped titles — locate the function by
     pattern. This requires a real signature database.
- `basmat` — the signature database: a versioned, data-driven file keyed by
  `(engine version range, text system version range, architecture, platform)` mapping each
  target method to one or more byte patterns with wildcards, an offset, and a validation
  predicate (for example: the resolved address must be inside the module's text section and
  must be reachable from a known vtable slot). It ships as data in the framework payload and
  is updatable independently of the plugin binary, so a new game version is a data change
  rather than a code change. Every resolution records which rung of the ladder produced it.
- `khatf` — native detour installation for the cases where a managed hook is impossible:
  a trampoline hooking implementation handling relative-jump rewriting, instruction-length
  decoding for the prologue, cross-page allocation within ±2 GB on x86_64, and
  ARM64 branch islands for Apple Silicon and Windows-on-ARM. Calling convention differences
  between IL2CPP's native ABI and the managed thunk are handled explicitly per platform,
  including the hidden `MethodInfo*` trailing argument IL2CPP passes.
- **GC interaction.** Every IL2CPP object reference held across a frame is a pinned or
  tracked handle; Taarib holds no raw `Il2CppObjectBase` pointers across GC boundaries;
  strings crossing into native code are copied into Taarib-owned buffers before any
  allocation could move them.
- **Honest degradation.** When a target cannot be resolved on any rung, the adapter reports
  a specific failure — naming the method, the version, and the rung that failed — through
  the log, through `barid` to Studio, and into the game's own log. The game continues in its
  original language. It does not crash, and it does not half-render.

**Technology.** C# 12 targeting .NET 6, BepInEx 6 IL2CPP, Il2CppInterop; a small C++ shim
(`shims/il2cpp_shim`) only where a raw vtable dispatch or a naked thunk is unavoidable.

**Hard constraints.**

- Identical rendering behaviour to Phase 6 — the shared assembly guarantees it structurally.
- No unresolvable target ever produces a crash, a hang, or a partially patched frame.
- The signature database is data, not code, and is versioned with its own schema.
- Decision 5 holds exactly as in Phase 6.

**Dependencies.** Phases 3, 5, 6.

**Definition of done.** IL2CPP games render Arabic through the same code path as Mono games;
the three-rung resolver works and reports which rung fired; the signature database resolves
targets across several engine versions; unresolvable targets degrade with a named reason.

---

### Phase 8 — Unreal Engine Adapter

**Scope.** `taarib-muhawwil-unreal` — Unreal already ships a complete text shaping stack
(HarfBuzz and ICU inside Slate). The strategy here is not to replace it but to *switch it
on, feed it, and correct it*, because engine-native shaping means Taarib's text participates
in every Slate effect, material, and animation the game already uses.

**Components produced.**

- `tashghil` — forcing full shaping. Unreal defaults to a fast non-shaping text path for
  performance. The adapter sets the shaping method to full shaping globally through the
  Slate settings and the corresponding console variable at startup, before the first font
  measure call, and re-asserts it if the game overwrites it. It also enables the
  bidirectional text detection Slate supports but does not always turn on.
- `khatt_unreal` — runtime font registration. The bundled Arabic font is registered into
  Slate's font system as a composite font with an Arabic sub-font and the correct fallback
  chain, applied to the styles the patch names, without touching a single `.uasset` font
  object. This is the one engine where the font is handed to the engine rather than
  rasterized by Taarib — permitted because it is *Taarib's own bundled font bytes*, loaded
  from the patch, never the game's font asset (Decision 5 is about the game's assets).
- `qiyas_unreal` — hooks on Slate's text measurement and layout services
  (`FSlateFontMeasure`, the text layout marshaller, and the line-breaking iterator) to
  correct flow direction, alignment resolution, and wrapping for right-to-left text in the
  versions where Slate's own resolution is wrong or where the game hard-codes left
  alignment.
- `mawarid` — localization resources. Read and write Unreal's real localization format:
  `.locres` (versions 1 through 3, including the city-hash string table form),
  `.locmeta`, and `StringTable` assets, both as loose files under `Content/Localization/`
  and inside packaged containers. Container support covers `.pak` (versions 1–11, with
  AES-256 decryption when the user supplies the key) and UE5 IoStore (`.utoc`/`.ucas`,
  including the container header, chunk ids, and compression blocks). Reinjection writes a
  new patch `.pak` mounted at a higher priority than the game's own — the standard Unreal
  modding path, fully reversible by deleting one file.
- `alam` — culture registration: add `ar` to the available cultures, set it as the active
  culture at startup, and ensure the game's own language menu either lists it or is bypassed
  by the patch's startup override, depending on what the game's UI allows.
- **In-world text.** `UTextRenderComponent` and Slate-in-3D widgets are covered by the same
  measurement and shaping corrections, so signage, floating damage numbers, and diegetic
  interfaces are Arabized along with the menus.

**Technology.** Rust injected module using `retour` for detours and a minimal C++ shim
(`shims/`) for the two places a `FSlateFontServices` vtable must be dispatched with C++
calling conventions; `aes` for pak decryption; `oodle`-compressed IoStore chunks handled
through the game's own decompressor via its exported hook, never by bundling a decompressor
Taarib has no licence to ship.

**Hard constraints.**

- No `.uasset` font object is read or replaced; fonts come from the patch (Decision 5).
- The patch pak is additive and higher-priority; original files are never modified in place.
- Forcing full shaping must be re-asserted defensively, because some games reset console
  variables from their own configuration after startup.
- If the pak or IoStore container cannot be read — unknown version, missing AES key — the
  adapter says exactly that and falls back to tier 3 rather than writing a broken container.

**Dependencies.** Phases 3, 5.

**Definition of done.** UE4 and UE5 games display Arabic through Slate with correct joining,
direction, alignment and wrapping in both interface and in-world text; localization
resources round-trip through both loose and packaged layouts; the patch installs and
uninstalls as one additive file.

---

### Phase 9 — Godot Adapter

**Scope.** `taarib-muhawwil-godot` — two different engines wearing one name. Godot 4 has a
complete text server with shaping and bidirectional support; Godot 3 has neither.

**Components produced.**

- **Godot 4 path.** Register the bundled Arabic font as a `FontFile` resource created at
  runtime from patch bytes, assign it into the theme's default font and the named theme
  overrides the patch lists, set the text direction and base direction on `Control` nodes,
  enable the advanced text server's Arabic features, and translate through Godot's own
  translation system by loading a generated `Translation` resource for the `ar` locale and
  setting `TranslationServer.set_locale("ar")`. Delivery is a GDExtension library plus a
  patch `.pck` mounted over the game's own via `ProjectSettings.load_resource_pack`, which
  Godot supports natively and which is removed by deleting one file.
- **Godot 3 path.** Godot 3 draws text through `Font::draw_char` with no shaping and no
  bidi, so this is a full takeover: hook the engine's text drawing through the signature
  database, intercept every string, lay it out through `jisr`, and draw the glyphs directly
  from Taarib's atlas using the engine's own low-level canvas item API
  (`VisualServer::canvas_item_add_texture_rect_region`), which exists unchanged across all
  of Godot 3.x. Where hooking the draw call is not viable — a statically linked, stripped,
  custom-built export template — the fallback is **glyph-identifier transport**: generate a
  Godot `BitmapFont`/`DynamicFont` replacement whose glyph table is Taarib's own rasterized
  glyphs mapped onto private-use codepoints, and emit the translated strings as sequences of
  those codepoints in visual order. This is not a presentation-form pipeline: the glyph
  identities come from real HarfRust shaping of the real logical text, and the private-use
  codepoints are an opaque transport for glyph ids, carrying no Unicode meaning and never
  written to any file a user could mistake for text. Decision 1 is intact.
- `pck` — package archive reading and writing for both formats: Godot 3's PCK v1 and
  Godot 4's PCK v2, including embedded-in-executable packages (where the PCK is appended to
  the binary with a trailing offset record), encrypted packages when the key is supplied,
  and the `.translation` resource format (`OptimizedTranslation`'s compressed hash-table
  form as well as plain `Translation`) so translations round-trip through the engine's own
  container rather than through a side file.
- **Script and native projects.** Games exported with GDScript, C#, or a custom module build
  are all handled: GDScript and C# projects take the resource-pack path; a custom native
  build that cannot load an extension takes the hook path with the transport fallback.

**Technology.** Rust; GDExtension C API through a generated binding for the Godot 4 side;
`retour` and the signature database for the Godot 3 side; `zstd`/`deflate` for PCK entries.

**Hard constraints.**

- Godot 4 uses the engine's own shaping (its text server is correct); Taarib does not
  duplicate it there. Godot 3 uses Taarib's shaping entirely.
- The glyph-identifier transport is documented in `docs/`, is never described as Unicode
  text, and is used only where direct drawing is impossible.
- Patch packs are additive and removable; the original `.pck` is never rewritten in place.

**Dependencies.** Phases 3, 5.

**Definition of done.** Godot 4 games render Arabic through the engine's text server with a
Taarib-registered font and a generated translation resource; Godot 3 games render Arabic
drawn by Taarib; both package formats read and write; embedded and encrypted packages are
handled; the transport fallback works where hooking cannot.

---

### Phase 10 — Script Engine Adapters

**Scope.** `taarib-muhawwil-nusus` plus `adapters-script/` — where a game's content is plain
data or interpreted script, patching it directly is more robust than injecting into it. Five
engines, one shared discipline: preserve originals, write patched output separately, stay
fully reversible.

**Components produced.**

- **RPG Maker MV and MZ.** Patch `data/*.json` — `Map*.json` event command lists (codes 101,
  401, 102, 405), `CommonEvents.json`, `Items`, `Weapons`, `Armors`, `Actors`, `Skills`,
  `States`, `Enemies`, `Classes`, `Troops` and `System.json` — writing translations into the
  same structures the engine already reads, so no engine change is needed for the text
  itself. Then inject `js/plugins/Taarib.js` (compiled from TypeScript in
  `adapters-script/rpgmaker/`) and register it in `plugins.js`. The plugin loads
  `taarib_core.wasm`, overrides `Bitmap.prototype.drawText` and the `Window_Base` text
  processing chain (`processCharacter`, `drawTextEx`, `calcTextHeight`, `textWidth`) to lay
  out through the WASM core and blit glyphs from Taarib's atlas onto the bitmap, and mirrors
  window layouts for right-to-left where the patch marks them. Escape codes (`\C[n]`,
  `\I[n]`, `\V[n]`, `\N[n]`, `\P[n]`, `\G`) are `nasq` spans, so colour changes and inline
  icons keep working inside Arabic text.
- **RPG Maker VX Ace.** Read and write `Game.rgss3a` (the RGSSAD v3 archive with its
  key-derived XOR obfuscation), patch `Data/*.rvdata2` (Ruby `Marshal` streams, deflate
  compressed for `Scripts.rvdata2`) to carry translated text, and inject a Ruby script into
  the script list. The injected `taarib_rgss3.rb` binds to the C ABI through RGSS's
  `Win32API`, replaces `Bitmap#draw_text` and the `Window_Base` message pipeline, and draws
  glyphs by blitting from an atlas bitmap loaded from the patch.
- **Ren'Py.** Generate translation scripts in Ren'Py's own format —
  `game/tl/arabic/*.rpy` with `translate arabic <label>:` blocks for dialogue and
  `translate arabic strings:` blocks for interface strings — which is the engine's supported
  localization mechanism and needs no hack for the text. Register the Arabic font by
  emitting style and `gui` overrides plus a `config.font_replacement_map` entry. Then inject
  `taarib_renpy/` (a Python package) which, on Ren'Py versions with built-in HarfBuzz and
  FriBidi support, sets the correct language and text direction properties and gets correct
  shaping from the engine; and on older versions installs a text filter that lays out
  through the C ABI via `ctypes` and returns a custom displayable drawing Taarib's glyphs.
  Version detection chooses the path automatically and records which one was used.
- **GameMaker Studio.** Read and write the `data.win` FORM container: `GEN8` for version,
  `STRG` for the string pool, `FONT` for glyph tables, `TXTR` for texture pages, `CODE` and
  `VARI` for the bytecode string references. Translated strings are written into `STRG` with
  the reference table rebuilt. The Arabic font is delivered as a **generated GameMaker font
  resource** whose glyph table is Taarib's own rasterized glyphs on a Taarib-packed texture
  page appended to `TXTR`, with the glyph-identifier transport encoding described in Phase 9
  used to address them, and text emitted in visual order from real shaping. `draw_text`
  alignment constants are corrected for right-to-left in the patched string data.
- **Electron and web-based games.** Unpack `resources/app.asar`, inject a preload script and
  a renderer runtime (compiled from `adapters-script/electron/`), and repack. The runtime
  sets `dir="rtl"` and the correct `unicode-bidi` and `direction` styles on the document and
  on dynamically created containers, installs `@font-face` for the bundled Arabic font from
  a data URL so no network request is made, and translates string resources in place through
  a `MutationObserver`-driven text node replacement keyed by the patch's string table. For
  canvas-rendered games — where the browser's own shaping never runs — the runtime routes
  `CanvasRenderingContext2D.fillText` and `measureText` through the WASM core and draws
  glyphs from Taarib's atlas.

**Technology.** Rust for every patcher (archive formats, JSON and Marshal rewriting, asar
packing); TypeScript compiled to a single ES5-compatible bundle for RPG Maker MV (NW.js
0.29 era) and a modern bundle for MZ and Electron; Python for Ren'Py; Ruby for VX Ace;
`taarib_core.wasm` shared by both JavaScript adapters.

**Hard constraints.**

- Every original file touched is backed up byte-exact into `nusakh/` before the first write,
  and uninstall restores it (Phase 15 owns the mechanism; this phase produces the manifest
  entries).
- Patched output is written atomically — write beside, fsync, rename — so an interrupted
  patch never leaves a truncated `data.win` or `app.asar`.
- Ruby, Python and JavaScript injected code is complete, readable, and contains no dynamic
  code evaluation of patch content.
- The glyph-identifier transport is used only where the engine cannot be made to shape and
  cannot be hooked, and is documented as a glyph transport, never as text.

**Dependencies.** Phases 1, 2, 3, 5, 14.

**Definition of done.** All five engine families patch, render Arabic correctly, and
uninstall to a byte-identical original; escape codes and inline icons survive; Ren'Py picks
the right path per version; canvas-based web games render through the WASM core.

---

### Phase 11 — Universal Graphics Layer

**Scope.** `taarib-tabaqa` — the fallback that makes the product genuinely universal: for
any game Taarib cannot patch, hook the presentation of frames, read what is on screen, and
draw shaped Arabic over it.

**Components produced.**

- `khataf` — presentation hooks, one implementation per graphics API, each drawing through
  that API's own device so there is no second context and no compositor:
  - **Direct3D 11** — vtable hook on `IDXGISwapChain::Present` and `Present1`, with
    `ResizeBuffers` handled for resolution changes and fullscreen transitions.
  - **Direct3D 12** — vtable hook on `IDXGISwapChain3::Present1` plus command queue capture
    through `ID3D12CommandQueue::ExecuteCommandLists`, with per-frame descriptor heaps and
    fence-synchronised resource lifetimes so the overlay never races the game's frame.
  - **OpenGL** — `wglSwapBuffers` on Windows, `glXSwapBuffers` on Linux, `eglSwapBuffers`
    where EGL is in use, with full state save and restore around the overlay draw because
    OpenGL is a global state machine and a game will not tolerate its state being changed.
  - **Vulkan** — a proper layer (`VK_LAYER_taarib_tabaqa`) with its own manifest, hooking
    `vkQueuePresentKHR` and `vkCreateSwapchainKHR`, allocating from its own device memory,
    and recording into its own command buffers on the presenting queue family.
- `iltiqat_shasha` — frame capture. Because the hook owns the backbuffer, capture reads the
  presented image directly, which works in exclusive fullscreen where desktop capture does
  not. Capture is region-limited, rate-limited, and format-converted on the GPU where
  possible.
- `qira` — on-screen text recognition. Platform-native first — Windows OCR through the
  Windows Runtime APIs, Apple's Vision framework on macOS — because they are fast, accurate,
  and already installed. Cross-platform fallback is the bundled `ocrs` engine with its
  models shipped in the framework payload. Preprocessing: region cropping, contrast
  normalisation, background suppression, and text-block detection so a dialogue box is read
  as a paragraph rather than as scattered words.
- `rasm_tabaqa` — overlay rendering. Every glyph drawn here comes from `saff` through
  `jisr`, from Taarib's own atlas, with a per-API renderer that draws a textured quad batch
  with premultiplied alpha. Presentation modes: over the original text with a solid or
  dimmed backing plate sized to the shaped Arabic, beside it, or in a fixed panel.
- `manatiq` — regions. User-defined regions drawn with a mouse in an in-game editing mode
  and persisted per game; automatic region detection that finds stable text blocks between
  frames; and per-region rules for when to translate (on change, on demand, continuously).
- `sijill_qira` — the reading history panel: every recognised and translated line kept in a
  scrollable panel with timestamps, so a player who missed a line can read it again without
  reloading a save.
- `lawhat_tahakkum` — the in-game control panel: an overlay-rendered interface with keyboard
  shortcuts for toggle, translate-now, region edit, history, opacity, font size, and pause.
  Rendered entirely by Taarib's own renderer; it does not depend on any UI library being
  present in the game.
- **Honest presentation.** Before the overlay is enabled for a game, the interface states
  plainly what tier 3 is: the game is not modified, translation is read from the screen and
  may be imperfect, latency exists, and performance cost is measurable. The user enables it
  knowing that. No marketing language is used about this tier anywhere in the product.

**Technology.** Rust; `windows` crate for D3D11/D3D12/DXGI and Windows OCR; `ash` for
Vulkan; a hand-written GL loader; `objc2` bindings and a small Objective-C shim for Vision
and Metal on macOS; `ocrs` with RTen for the portable OCR path; `dll-syringe`-style injection
on Windows, `LD_PRELOAD` on Linux, `DYLD_INSERT_LIBRARIES` on macOS where the target's
hardened runtime permits it.

**Hard constraints.**

- The overlay never changes game state and never writes into the game's memory outside its
  own hook trampolines.
- Every hook is removable at runtime, and the module unloads cleanly.
- The overlay's own text goes through `saff`. There is no second text path in this product.
- Frame time cost is measured and displayed in the control panel, because a fallback that
  silently halves someone's frame rate is not honest.
- On macOS, where injection into a hardened-runtime process is refused by the system, the
  capability report says so before the user tries, and offers the window-capture path.

**Dependencies.** Phases 1, 2, 3, 5, 13.

**Definition of done.** All four graphics APIs hook, capture, recognise, and render Arabic
overlays on all three platforms within their stated constraints; regions, history, and the
control panel work; the honest presentation is shown before enabling.

---

### Phase 12 — Text Extraction

**Scope.** `taarib-istikhraj` — pull every translatable string out of a game, from static
containers where possible and from the running game where not, and normalise everything
into one string table.

**Components produced.**

- **Unity.** A read-only implementation of Unity's serialization: `UnityFS` bundle framing
  (all compression modes: none, LZMA, LZ4, LZ4HC, and the block/directory info layout),
  `SerializedFile` headers across format versions, the type tree, and object extraction for
  the classes that carry text — `TextAsset` (49), `MonoBehaviour` (114) with its serialized
  fields walked through the type tree so localization tables and dialogue databases are
  read without knowing their C# types, `GameObject`/`Transform` for context paths, and the
  common localization packages (Unity Localization's string tables, I2 Localization's
  `LanguageSource`, and plain CSV/JSON text assets). For IL2CPP games, string literals are
  read from `global-metadata.dat`'s literal heap; for Mono games, from the `#US` user-string
  heap of `Assembly-CSharp.dll` via a minimal CLI metadata reader. Both are noisy sources,
  so both are ranked lower and are filtered by heuristics that Phase 13's review surfaces
  rather than hides.
- **Unreal.** `.locres` (all versions), `.locmeta`, and `StringTable` assets, read from loose
  files and from `.pak` and IoStore containers, with namespace and key structure preserved
  exactly so reinjection round-trips.
- **Godot.** `.translation` resources (plain and optimized/compressed), `.po` files, and
  scene/resource text properties from `.scn`/`.tres` inside `.pck`.
- **Script engines.** RPG Maker JSON event command lists with their command codes preserved
  as context; VX Ace `Marshal` object graphs; Ren'Py `.rpy` script parsing for `say`
  statements, menu choices, and interface strings, plus `.rpa` archive reading; GameMaker
  `STRG` with code-reference cross-linking so a string's use site is known; Electron's asar
  contents including JSON, JS string literals, and HTML text nodes.
- **Runtime capture.** For games whose strings cannot be extracted statically — procedurally
  composed text, strings behind encryption, engines with no readable container — every
  adapter reports each string it draws over `barid`. Studio records it with the scene,
  component path, screen rectangle, font size, and a screenshot crop of where it appeared.
  A capture session is a first-class object: start it, play the game, stop it, and the
  strings are in the project with real context attached.
- `jadwal` — the normalized string table. Every entry, from every source, carries:
  - `NassId` — deterministic identity from source location plus source text.
  - Source text, exactly as stored, including its markup.
  - Context: the container, the object path, the field name, the event command code, the
    speaker where one is identifiable, and the surrounding lines for dialogue.
  - Source location precise enough to write back to.
  - Constraints: maximum characters where the engine imposes one, and maximum pixel width
    measured from the original text's own rendered width and the rect it was drawn into
    when runtime capture saw it.
  - Detected markup and placeholders as `nasq` spans, extracted at import so translation
    never sees raw tags.
  - Occurrence count and where it appeared in play.
  - A duplicate group id, so one translation serves every identical source string.

**Technology.** Rust; hand-written binary readers for every container (no dependency on any
third-party game-asset library, all of which are version-fragile); `serde_json` for RPG
Maker; a Marshal reader for VX Ace; a Ren'Py script parser; `barid` for the capture channel.

**Hard constraints.**

- Extraction is read-only. It never writes into the game directory.
- Extraction failures are per-container and per-object, never fatal to the session, and are
  reported with the container name and the reason.
- No original game asset extracted here is ever included in a patch (Decision 5 and the
  Phase 14 gate that enforces it).
- String identity is deterministic: extracting the same build twice produces the same ids.

**Dependencies.** Phases 0, 4, 5, and the adapters for the capture channel.

**Definition of done.** Every engine in the coverage matrix yields a normalized string table
from a real installation, with context and constraints populated; runtime capture records
strings with screen context; identity is reproducible; duplicates are grouped.

---

### Phase 13 — Translation Pipeline

**Scope.** `taarib-tarjama` — take a string table to a reviewed Arabic translation, with
machine assistance that respects game context and never breaks a placeholder.

**Components produced.**

- `muzawwidun` — providers behind one trait, each with its own model list, pricing table,
  rate limits, and context window: Anthropic (Claude), OpenAI and any OpenAI-compatible
  endpoint, Google Gemini, DeepL, Google and Microsoft translation APIs, and local models
  through an Ollama-compatible endpoint. Credentials live in the OS keychain, never in
  configuration files, never in logs, never in a diagnostics bundle.
- `siyaq` — game-appropriate prompting. A translation request carries far more than the
  string: the game's name and genre, the speaker where known, the surrounding dialogue
  lines, the interface context (a button label is not a sentence), the character or pixel
  constraint so the model knows it must be short, the glossary entries that apply to this
  string, the declared tone (formal Modern Standard Arabic, contemporary, or a specified
  dialect), and the placeholder atoms it must reproduce exactly. Prompts are versioned and
  recorded with each translation so a result can be explained later.
- `hima` — placeholder and markup protection, enforced in three steps: extract every
  placeholder and markup span into opaque atoms before the text leaves for the provider,
  present atoms to the model in a form it will not translate, and validate the returned
  text — every atom present, none duplicated, none reordered in a way that breaks a
  positional format specifier, no stray tag introduced. A translation that fails validation
  is rejected and retried with a corrective instruction, and after a bounded number of
  retries it is flagged for human attention rather than accepted.
- `dhakira` — translation memory: a per-project and cross-project store of source-target
  pairs with fuzzy matching (normalized-token similarity with a configurable threshold),
  scoring, and provenance. Reuse is proposed, never applied silently. Memory is shared
  across games by default because item names and interface verbs repeat endlessly across
  titles, and that is exactly where consistency matters.
- `masrad` — the glossary: canonical Arabic for names, places, items, abilities, and
  systems, with part of speech, gender, plural form, a do-not-translate flag, and notes.
  Enforcement runs over the whole project continuously and reports every string where a
  glossary term's source appears but its canonical target does not.
- `dufaat` — batch processing: concurrency bounded per provider, token-bucket rate limiting,
  cost accumulation against a user-set budget with a hard stop, checkpointed progress so an
  interrupted batch resumes exactly where it stopped, and per-string retry with exponential
  backoff on transient failures.
- `alamat` — quality flags computed after every change, each one actionable and each one
  linking to the strings that caused it: overflow risk (measured through `saff` against the
  string's real pixel constraint, not estimated), untranslated leftovers (Latin text
  remaining inside an Arabic translation), inconsistent terminology against the glossary and
  against the project's own prior choices, broken or missing placeholders, machine
  translation confidence below threshold, and suspicious length ratios in both directions.
- `muraja_dakhiliya` — the review workflow inside a project: per-string status
  (`LamTutarjam` → `Musawwada` → `LilMuraja` → `Muakkada` → `Mujammada`), comments threaded
  per string, and a full change history with author, timestamp, and the previous value, so
  any edit can be traced and reverted.

**Technology.** Rust; `reqwest` with streaming for provider APIs; `governor` for rate
limiting; `keyring` for credentials; `rusqlite` for the memory store with FTS5 for search;
`saff` itself for every width measurement, because an overflow warning computed by any other
means is a lie.

**Hard constraints.**

- No credential ever appears in a log, an error, a crash report, or a diagnostics bundle.
- No translation with a broken placeholder is ever accepted into a project.
- Every machine translation is marked as machine-produced and carries its provider, model,
  and prompt version until a human confirms it.
- Cost is tracked before the request is sent, and the budget stop is enforced client-side.

**Dependencies.** Phases 1, 12.

**Definition of done.** All listed providers work; prompts carry full context; placeholder
protection catches and rejects corruption; memory and glossary propose and enforce; batches
run, resume, and respect budgets; every quality flag computes and links to its strings.

---

### Phase 14 — Patch Compiler

**Scope.** `taarib-ruqaa` (the container format) and `taarib-tarqee` (the compiler) — turn a
reviewed translation into a compact, signed package that a game can consume with zero layout
work at runtime.

**The container format,** specified here because four languages must read it:

```
.ruqaa — little-endian, 16-byte aligned sections, mmap-friendly

  Header (64 bytes)
    magic          "TRQ1"                     4
    format_version u16 = 1                    2
    flags          u16   (sdf | coverage | rtl_mirroring | capture_hints)   2
    section_count  u32                        4
    total_size     u64                        8
    content_hash   [u8; 32]  BLAKE3 of everything after the header           32
    reserved                                  12

  Section table (32 bytes each)
    kind u32  offset u64  length_stored u64  length_raw u64  compression u32
    kinds: 1 BAYAN (metadata, JSON)      2 NUSUS  (string table)
           3 TAKHTIT (precomputed layouts) 4 LAWHA  (atlas pages)
           5 KHAREETA (glyph map)          6 KHATT  (embedded font subset record)
           7 QIYUD  (per-string constraints and reflow hints)
           8 TAWQEE (signature block, never compressed, always last)

  All sections except TAWQEE are zstd-compressed as a unit.
  NUSUS, TAKHTIT, KHAREETA and QIYUD decompress into fixed-layout POD arrays with
  4-byte indices, readable by struct overlay from C#, JavaScript, Rust, Python and Ruby
  with no parser and no allocation.

  TAWQEE: ed25519 signature over content_hash, the signer's public key,
          the key's role (owner | contributor-selfsigned), and the signing timestamp.
```

**Components produced.**

- `tahdid_maqasat` — size discovery: every text size the game actually draws at, learned
  from the capability probe, from runtime capture, and from the adapter's reported font
  sizes, so the atlas is compiled for the sizes that exist rather than for a guessed set.
- `takhtit` — precompilation. Every translated string is run through `saff` at every
  discovered size, inside its real available width, with its real style spans, and the
  resulting positioned glyphs are stored. The game does no shaping, no bidi, no line
  breaking, and no justification at runtime for any string the compiler saw. The runtime
  path exists only for text the compiler could not know.
- `lawha` invocation — the exact glyph set from those layouts is collected and packed
  (Phase 2), in the mode the patch declares.
- `taqrir_tajawuz` — the overflow report. For every string: measured width, available width,
  overflow in pixels and in percent, the size it was measured at, and the interface element
  it belongs to. Sorted by severity. This report ships inside the patch metadata and is
  shown to the reviewer and to the contributor, because a patch that overflows is a patch
  that looks broken in screenshots, and the only way to prevent that is to measure.
- `irtibat` — binding. The patch declares the games it targets across every launcher
  identifier, the build ids it was compiled against, and the `Basma` content fingerprint of
  the original text-bearing files. Installation refuses a mismatch it was not told to
  tolerate (Phase 15 handles re-matching).
- **The asset gate.** Before a package is written, the compiler proves that it contains no
  original game asset: every byte in the package is traceable to translated text, to layout
  computed from that text, to glyphs rasterized from a bundled or user-supplied font whose
  hash is in `khutut.json`, or to metadata authored by the contributor. Any input that
  cannot be traced fails the compile with the offending source named. This is not a warning.
- `mustawrid` — importers, so nobody has to start over: XUnity.AutoTranslator's plain-text
  `original=translated` format (the de facto community format, including its escaping
  rules and section markers), XLIFF 1.2 and 2.0, gettext PO/POT, TMX for memory, CSV with
  configurable columns, and Unity Localization's own CSV export. Every import maps onto the
  extracted string table with matched and unmatched entries reported and resolvable.

**Technology.** Rust; `zstd`; `blake3`; `ed25519-dalek`; `bytemuck` for the POD overlays;
`saff` for every layout and measurement.

**Hard constraints.**

- Layouts are precomputed for every known string and size. A shipped patch that shapes at
  runtime for known text is a compiler bug.
- No original game asset in the package, enforced by the gate, not by convention.
- The format is fixed-layout and readable by struct overlay in every consumer language.
- Compilation is deterministic: the same project produces a byte-identical package.

**Dependencies.** Phases 1, 2, 12, 13.

**Definition of done.** A project compiles to a `.ruqaa` that installs and renders; the
overflow report is complete and accurate; the asset gate blocks a deliberately contaminated
input; all listed import formats round-trip; two compiles of the same project produce
identical bytes.

---

### Phase 15 — Installation, Rollback, and Update Survival

**Scope.** `taarib-tathbeet` — put the right framework, adapter and patch in the right place
on the right platform, record everything, restore perfectly, and survive the day the store
updates the game.

**Components produced.**

- `tarkib` — framework installation, per engine and per platform:
  - **Unity, Windows**: BepInEx 6 into the game root with the correct architecture and
    Unity backend variant, `winhttp.dll` proxy loader, `doorstop_config.ini` written with
    the correct target assembly, and Taarib's plugin, native library, fonts and patch into
    `BepInEx/plugins/Taarib/`.
  - **Unity, Linux native**: BepInEx with `run_bepinex.sh`, `libdoorstop.so`, and the launch
    option rewritten in Steam's `localconfig.vdf` (preserving whatever the user already had
    there) so no wrapper script is needed.
  - **Unity, Proton**: the Windows BepInEx installed *inside the prefix's* game directory,
    with `WINEDLLOVERRIDES=winhttp=n,b` added to the launch options and the prefix's
    registry updated, resolving all paths through the Phase 4 prefix mapper.
  - **Unity, macOS**: BepInEx into the `.app` bundle with `run_bepinex.sh` and the
    `libdoorstop.dylib`, with a clear report when the game's hardened runtime forbids it.
  - **Unreal / Godot 3 / tier 3**: the injected native module plus its loader, per platform.
  - **Godot 4 / script engines**: no framework — the patch is data plus an additive package
    or an injected script.
- `bayan` — the installation manifest: every file added with its hash, every file modified
  with its original content preserved into `nusakh/` (byte-exact, hashed, compressed), every
  launcher setting changed with its previous value, and every registry key written with its
  prior state. The manifest is written *before* the change it describes and committed after,
  so an interrupted install is always recoverable.
- `taraju` — uninstall: restore every modified file from its backup, delete every added file,
  revert every launcher setting and registry value, and verify the result against the
  original hashes. The game returns to its exact original state, and Taarib says so with
  the verification result rather than assuming it.
- `najat_tahdith` — update survival. When a scan finds a new build id for a game with an
  installed patch:
  1. Detect it before the user launches, and say what happened in plain Arabic.
  2. Recompute the `Basma` fingerprint. If the text-bearing files are unchanged, the patch
     still applies exactly and installation is refreshed automatically.
  3. If they changed, offer re-matching: compare the new extraction against the patch's
     string table, migrate every unchanged string, and present exactly what is new,
     changed, or gone — as counts first, then as a list.
  4. If the engine version changed in a way that invalidates the framework, reinstall the
     framework for the new version.
- `tahaqquq` — integrity verification before and after every operation: the game's own files
  hashed against the fingerprint the patch expects, and against the launcher's own manifest
  where one exists, so Taarib knows whether it is patching a modified installation before it
  starts.
- **Launch integration** — writing launch options, wrapper handling for games that relaunch
  themselves through a launcher executable, and `steam://` URL launching that respects the
  user's compatibility tool. Custom launch options the user already had are preserved,
  appended to, and restored on uninstall.

**Technology.** Rust; atomic write-and-rename everywhere; `zstd`-compressed backups;
platform-specific registry and prefix handling from `manassa` and `beea`.

**Hard constraints.**

- Never modify a file without a byte-exact backup and a manifest entry written first.
- Every operation is idempotent and resumable; an interrupted install leaves a recoverable
  state, never a half-patched game.
- Uninstall is verified, not assumed.
- Nothing is written outside the game directory, the prefix, and Taarib's own data
  directories.

**Dependencies.** Phases 4, 5, 14, 16.

**Definition of done.** Install and uninstall are exact on every platform including Proton
prefixes and the Steam Deck; the manifest captures every change; a build update is detected,
explained, and re-matched by fingerprint with unchanged strings migrated; integrity is
verified on both sides of every operation.

---

### Phase 16 — Safety Layer

**Scope.** `taarib-aman` — refuse to hurt the user. This runs before any installation, and
its refusals are absolute.

**Components produced.**

- `kashf_himaya` — anti-cheat detection by file, module, service, driver and manifest
  signature: Easy Anti-Cheat (both the legacy and EOS forms), BattlEye, Denuvo Anti-Cheat,
  Riot Vanguard, nProtect GameGuard, XIGNCODE3, PunkBuster, FACEIT AC, ESEA, Ricochet, and
  Steam's own VAC association read from `appinfo.vdf`. Detection is by evidence, and the
  evidence is shown. When any is found, **installation is refused outright** — not warned
  about, not gated behind an "I understand" — with a plain Arabic explanation that modifying
  files in this game can permanently ban the account, and the name of what was detected.
- `kashf_shabaka` — online and multiplayer detection: Steam categories, known networking
  and service modules, launcher metadata, and the presence of a dedicated server or
  matchmaking client. A multiplayer game without anti-cheat produces an explicit warning
  with an acknowledgement the user must give, every time, per game.
- `tahaqquq_tawqee` — signature verification before installation: the package's signature is
  checked against the owner's public key embedded in the client; a package that is unsigned,
  signed with an unknown key, signed with a revoked key, or whose content hash does not
  match its signature is refused with the specific reason. There is no override, no
  developer switch, and no configuration flag that disables this.
- `sandooq_fak` — isolated unpacking: everything downloaded is unpacked into a quarantine
  directory with path traversal rejected (no `..`, no absolute paths, no symlinks, no
  device names, no case-collision attacks on case-insensitive filesystems), size limits
  enforced against decompression bombs, entry counts bounded, and every extracted path
  validated to stay inside the quarantine before a single byte is written. Nothing is
  trusted by path or by declared content type.
- `iqrar` — the first-run acknowledgement: a clear, non-alarmist Arabic statement that
  Taarib modifies game files, that originals are backed up and restorable, that some games
  forbid modification, and that the user is responsible for their own accounts. Recorded
  once with a timestamp and the product version, and re-shown when the statement changes.
- `qaimat_sahb` — the revocation list: fetched with the registry manifest, cached, and
  checked before every install and on every launch, so a patch found to be harmful stops
  installing immediately and existing installations are flagged with a clean uninstall
  offer.

**Technology.** Rust; `ed25519-dalek` and `blake3` through `taarib-khatm`; platform APIs for
service and driver enumeration.

**Hard constraints.**

- Anti-cheat refusal has no override path anywhere in the product.
- Signature verification cannot be disabled by configuration, by build flag, or by
  developer mode.
- Unpacking is quarantined and validated before any write.
- Every refusal names what was detected and why, in Arabic, with a next step.

**Dependencies.** Phases 4, 5, 14.

**Definition of done.** Every listed anti-cheat is detected and refused with evidence;
multiplayer warnings require per-game acknowledgement; unsigned, tampered and revoked
packages are refused; malicious archives cannot escape quarantine; the acknowledgement is
recorded.

---

### Phase 17 — Taarib Registry: The Community Repository

**Scope.** `taarib-mustawda` and `registry-template/` — the core value of the product. A
player who never translates anything should open Taarib, see that four of their games have
Arabic patches, and install one in a click. Everything in this phase serves that sentence.

**The backend structure** (a public Git repository, Decision 7):

```
taarib-registry/
  bayan.json                 global manifest — ONE small request answers "did anything change?"
    { mukhattat, generated_at, shards: { "steam/00": "<blake3>", ... },
      revocations_hash, contributors_hash, requests_hash,
      mirrors: [ raw, cdn, secondary-forge ], min_client_version }

  fahras/                    the sharded index — no client ever downloads the catalogue
    steam/00.json … ff.json  shard = first byte of blake3(app_id), 256 shards per source
    epic/…  gog/…  xbox/…  itch/…  yadawi/…
      each shard: { games: [ { source_id, luba_id, name, ruqaa: [ RuqaaListing… ] } ] }
      RuqaaListing is deliberately small: id, revision, title, contributor, coverage,
      string_count, size, build_ids, basma list, engine, backend, tier, rating,
      updated_at, content_hash, asset_url, mirror_url

  ruqaa/<ruqaa_id>/<revision>.json   full metadata record, fetched only when a user opens
                                     a game's detail screen — never during the library scan
  musahimun/<key_fingerprint>.json   contributor identity, credit line, reputation counters
  talabat/                           translation requests with demand counts
  sahb.json                          revocation list
  mafatih/malik.pub                  the owner's public key (also embedded in every client)
  sijill_muraja/YYYY-MM.jsonl        the permanent, public audit log of review actions
```

Patch binaries are **release assets**, never committed into Git history, so the repository
stays small enough to clone forever and no binary ever becomes unremovable.

**Components produced.**

- `jalb` — the fetch flow, designed so the library screen never waits on the network:
  1. On launch, Studio already knows the user's games from the local database.
  2. One request fetches `bayan.json`. If every shard hash matches the cache, nothing else
     is fetched and availability badges resolve instantly from cache, offline included.
  3. Otherwise, only the shards matching identifiers the user actually owns are requested,
     batched, in parallel, with HTTP conditional requests and a CDN mirror fallback.
  4. Results are cached with their shard hash and a fetch timestamp.
  5. Badges resolve into the grid as they arrive. The grid renders immediately, always.
- `mutabaqa` — three-tier matching, applied per game, with the tier shown to the user:
  1. **Exact build match** — the patch declares the build id the user has. Marked
     "متوافقة تمامًا" (exactly compatible).
  2. **Fingerprint match** — build ids differ but `Basma` matches, meaning the store updated
     something that is not text. Marked "متوافقة (تحديث لم يغيّر النصوص)". This is the tier
     that keeps patches alive through routine updates, and it is why fingerprints exist.
  3. **Compatibility range** — the patch declares a range that includes this build but
     neither of the above matched. Marked with an explicit warning that some text may be
     missing or misplaced, and the install flow requires acknowledgement.
  No fourth tier. A patch that matches none of these is shown as incompatible with the
  reason, not hidden.
- `tanzeel` — downloading: HTTP range resume across restarts, parallel chunks, mirror
  failover in the order the manifest declares, hash verification streamed during download
  rather than after, and signature verification before a single byte reaches the game.
- `tarteeb` — ranking when a game has several patches: build match quality first, then
  coverage, then rating, then recency. All of them are listed with their differences shown
  side by side — contributor, coverage, method, size, date, tier — because hiding the
  alternatives behind one recommendation is how communities lose trust in a client.
- `taqyeem` — ratings and reports submitted from inside the application, signed by the
  submitting contributor's key, rate-limited per account, and recorded in the registry.
- `sumaa` — contributor reputation, computed from accepted patches, revisions accepted
  without changes requested, revocations against them, and rating aggregate, displayed on
  every listing and on the contributor's own profile.
- **Offline capability, complete:**
  - Import a `.ruqaa` file from disk and install it with no network at all.
  - Export any installed or compiled patch to a file.
  - A local patch library that is a first-class registry source, not a fallback.
  - A bundled offline mirror (a directory or a mounted image with the same structure as the
    repository) and a local network share as configurable registry sources, so a LAN party,
    a school lab, or a country with a bad link to the CDN can run the entire product with
    no internet.
- **Trust, enforced client-side:** every patch signed (Decision 8); unsigned or mismatched
  packages refused outright by Phase 16; the revocation list checked before every install
  and on every launch; the owner's public key embedded in the client binary so a
  compromised registry cannot substitute its own.

**The registry template** ships in this repository: the directory skeleton, the JSON Schema
for every record generated from the Rust types, a `README` explaining the structure to
anyone who forks it, the mirror configuration, and the publishing tooling used by Phase 18.

**Technology.** Rust; `reqwest` with HTTP/2 and conditional requests; `blake3`;
`ed25519-dalek`; `zstd`; `rusqlite` for the cache; the Git forge's raw content endpoint plus
a CDN mirror plus a secondary forge as the fallback chain.

**Hard constraints.**

- The library screen never blocks on the network. Not once, not for artwork, not for badges.
- No client ever fetches the whole catalogue. Sharding is not an optimisation here; it is
  the contract.
- Full patch metadata is fetched lazily, on detail view, never during a scan.
- Every install path — network, local file, mirror, share — goes through the same signature
  and revocation checks.
- The registry is readable and forkable by anyone with `git clone` and no tooling from us.

**Dependencies.** Phases 4, 14, 15, 16.

**Definition of done.** A launch with a warm cache costs one small request; shards fetch
selectively for owned games only; all three match tiers resolve and are labelled; download
resumes across restarts and fails over to mirrors; ranking lists alternatives with their
differences; ratings, reports and reputation work; the product is fully usable with no
network from a local library, a bundled mirror, or a network share.

---

### Phase 18 — Submission and Owner Review

**Scope.** `taarib-taqdeem` — the path from a finished community translation to a published,
signed patch. Review authority belongs to **one person: the project owner.** Everyone else
is a contributor. This is not a permission system with an admin role; it is a product with
one reviewer, and it is built that way deliberately.

#### 18.1 The contributor side

- **Two entry points, one submission object.** Either submit a translation completed in
  Taarib's own workspace, or import one completed elsewhere — XUnity plain text, XLIFF,
  PO, CSV, TMX — which is mapped onto the game's extracted string table with matched and
  unmatched entries reported and resolved *before* the submission can proceed. An import
  that leaves unmatched entries unresolved cannot be submitted.
- **A submission is a persistent, resumable local draft.** It survives closing the
  application, a crash, and a machine restart. It carries: target game identity across every
  launcher, target build ids, original-file fingerprints, the compiled `.ruqaa`, contributor
  identity and credit line, license, title, description, changelog, coverage, string counts,
  the declared translation method (fully human / machine-assisted then human-reviewed /
  machine only), and the complete quality report from Phase 13 and the overflow report from
  Phase 14.
- **The pre-flight gate.** The submit action is inactive until every check passes, and the
  checks are displayed as a checklist the contributor works through, each item linking
  directly to the strings that fail it.

  | Blocking | |
  | --- | --- |
  | Package invalid or unsigned | The compiled package must verify against the contributor's own key |
  | Original game asset detected inside the package | The Phase 14 asset gate, re-run at submission |
  | Broken placeholders or markup against source | Any string whose atoms do not match its source |
  | Any string fails to render through `saff` | Every string is laid out before submission; a failure is a blocker |
  | Missing required metadata | Title, description, license, method, target builds |
  | Coverage below threshold | Below the declared minimum for a publishable patch |
  | Duplicate of the contributor's own patch for that build | Prevents accidental resubmission |

  | Warning — requires explicit acknowledgement | |
  | --- | --- |
  | High overflow count | Shown with the worst offenders listed |
  | Large untranslated remainder | With the count and where they are |
  | Machine-only method | Acknowledged as such and labelled publicly |
  | Terminology inconsistent with the glossary | With every inconsistency listed |

- **The Contributions screen** tracks every submission with live status — `musawwada`
  (draft), `muqaddama` (submitted), `qayd_muraja` (under review), `matlub_taadil` (changes
  requested), `mawafaq_yunshar` (approved and publishing), `manshura` (published),
  `marfuda` (rejected), `mashuba` (withdrawn) — and the complete review conversation with
  every comment anchored to the string it concerns.
- **Contributors see their own submission state and nothing more.** Never the queue, never
  another contributor's submission, never a control that implies they could approve
  anything. There is no hidden route, no URL, and no keyboard shortcut that reveals the
  Review Console in a contributor's session, because the console's routes are not registered
  in that session at all.

#### 18.2 The owner side — the Review Console

Ownership is established by holding the signing key and write access to the registry
repository, resolved at authentication. It is not a flag in a database.

- **Queue view.** Every pending submission. Filterable by game, engine, contributor, method,
  coverage and age. Sortable by wait time, coverage or contributor reputation. Automated
  check results appear as a status column so a failing submission is dispatched in one
  keystroke without opening it. Fully keyboard-driven: navigate, open, act, return.
- **Submission detail — everything in one screen**, because a review that requires
  navigation is a review that gets rushed:
  - Full metadata; contributor identity, reputation, and their previously published patches.
  - The complete automated validation report, check by check, with evidence.
  - The full string table side by side — source against submitted Arabic — with context and
    source location per entry, filterable by status, quality flag and length, and
    free-text searchable across both sides simultaneously.
  - **Live render preview** of any selected string through the real `saff` engine at the
    real size inside the real interface bounds recorded for that string. Not an
    approximation, not a web font preview — the same code path the game will run.
  - The complete overflow report sorted by severity, every entry jumping to its preview.
  - A **diff against any already-published patch** for the same game, so competing or
    successor patches are comparable.
  - A **diff against the contributor's previous revision**, so a second review reads only
    what changed.
- **Sandbox verification.** From inside the console: install the submitted patch into a local
  copy of the game, launch it, and uninstall it — isolated under `sandooq/`, fully
  reversible, with the outcome (installed, launched, rendered, uninstalled clean) recorded
  onto the submission as evidence.
- **Actions.**
  - *Comment*, anchored to a specific string, appearing to the contributor at exactly that
    string in their workspace.
  - *Request changes* — a written summary plus the anchored comments, returning the
    submission for revision. The submission object persists; the contributor revises and
    resubmits the same object rather than creating a new one, so history stays continuous.
  - *Reject* — with a required written reason. An empty rejection is not possible.
  - *Approve and publish.*
- **Publishing** is one atomic operation with an ordered, resumable sequence: sign the
  package with the owner's key; promote the binary from staging to the public release area;
  write the metadata record into the correct shard; recompute the shard hash; update
  `bayan.json`; append to the public audit log; record reviewer identity and approval
  timestamp *inside the patch metadata* so every published patch carries its provenance
  permanently. An interruption at any step resumes from that step; a failure rolls back the
  staging promotion so a half-published patch cannot exist.
- **Also required:** bulk actions across a multi-selection; a permanent audit log of every
  review action, written to the registry and therefore public; revocation that immediately
  removes a published patch from circulation, adds it to `sahb.json`, and notifies every
  user who installed it with a one-click clean uninstall; and in-application notifications
  for state changes, comments, publication, and new queue arrivals.
- **No automatic publishing path exists**, no matter how clean the automated checks come
  back (Decision 8). **No self-approval bypass:** the owner's own translations pass through
  the same submission object and the same publish step, so provenance and signing are
  uniform across the entire catalogue.
- **Optional triage capability.** The owner may grant one other person the ability to sort
  and comment — nothing else. It is a narrow, explicitly granted capability recorded in the
  registry and revocable at any moment; it is not a role, not a hierarchy, and it never
  touches approve or publish, which remain bound to the signing key and therefore to the
  owner alone.

#### 18.3 Translation requests board

Users request that a game be translated. Requests are stored in the registry with a demand
count, deduplicated per user by key, and surfaced on the detail screen of any game that has
no patch — turning the emptiest screen in the product into the place where demand is
recorded. Contributors browse requests sorted by demand, by engine, or by tier feasibility.
Publishing a patch closes the matching request automatically and notifies everyone who asked
for it.

**Technology.** Rust for the submission object, the validation suite, the sandbox runner and
the publishing sequence; `git2` and the forge's API for repository writes and release asset
uploads; `ed25519-dalek` with the owner's key held in the OS keychain and never written to
disk in plaintext; React for both the contributor screens and the console.

**Hard constraints.**

- One reviewer. No role hierarchy, no permission matrix, no admin abstraction.
- The Review Console does not exist in a contributor's session — routes unregistered,
  commands absent from the palette, no dead menu items.
- The submit action is inactive until every blocking check passes locally.
- Every published patch is signed by the owner's key at approval, and carries its reviewer
  and timestamp.
- Publishing is atomic and resumable; a partially published patch is impossible.

**Dependencies.** Phases 13, 14, 15, 16, 17.

**Definition of done.** A translation submits from the workspace or from an import; the
pre-flight gate blocks and explains every listed failure; the console shows the queue and the
full detail screen with live previews, overflow, and both diffs; sandbox verification runs
and records; all four actions work with anchored comments reaching the contributor; publishing
signs, promotes, shards, updates the manifest, logs, and notifies; revocation pulls a patch
and offers uninstall; the requests board records demand and closes on publication.

---

### Phase 19 — Collaborative Translation Workspace

**Scope.** `taarib-warsha` — translation projects that more than one person can work on,
share, merge and reconcile, without a server.

**Components produced.**

- `mashru` — the project file: a portable, versioned, self-describing bundle carrying the
  string table, every translation with its status and history, the glossary, the project's
  translation memory, contributor records, and the compile settings. It is a single file the
  user can send to another person, and it opens with everything intact.
- `tawzi` — assignment and locking: strings assigned per translator individually or in
  ranges; a soft lock claimed while editing so two people do not duplicate work; the lock
  carries the holder and a timestamp and expires, because a stale lock that blocks a project
  is worse than a collision.
- `damj` — merge and reconciliation. Two project files carrying divergent work merge by
  string identity, with three outcomes per string: one side changed (take it), both changed
  identically (take it), or both changed differently (a conflict). Conflicts open in a
  three-pane resolver — source, mine, theirs — with the full history of both sides visible,
  a per-string choice, and bulk resolution by rule (prefer mine, prefer theirs, prefer the
  more recently edited, prefer the reviewed one). Glossary and memory merge by union with
  conflict resolution on differing canonical targets.
- `tarikh` — per-string change history: every edit with author, timestamp, previous value,
  and the reason where one was given, kept in full and browsable, with revert to any point.
- `tabadul` — import and export of the standard interchange formats (XLIFF 1.2 and 2.0,
  PO/POT, TMX, CSV, XUnity plain text) at project level, so a translator can round-trip work
  through whatever tool they already use and bring it back without losing status or history.
- `ihsaat` — contribution statistics per translator: strings translated, reviewed, and
  corrected; words; time span; and per-project share, used for the credit line on the
  published patch and for the contributor's own record.

**Technology.** Rust; SQLite for the project store with the portable bundle as a compressed
export of it; content-addressed string identity from Phase 12 as the merge key.

**Hard constraints.**

- No server. Collaboration is file-based and works by any transport the users already have.
- Merge never loses an edit silently; every discarded value stays in history.
- String identity is the merge key, so renamed containers and reordered files do not create
  false conflicts.

**Dependencies.** Phases 12, 13.

**Definition of done.** Projects export, transfer, and open intact; assignment and locking
work; divergent projects merge with conflicts resolved in the three-pane resolver; history is
complete and revertible; all interchange formats round-trip; statistics compute and feed the
credit line.

---

### Phase 20 — Taarib Studio: The Interface

**Scope.** `apps/studio` — the default and primary experience of the product, built as a
polished consumer application. Arabic by default with complete right-to-left layout and an
English toggle. Dark by default with a light option. Every capability in this roadmap is
reachable from it.

This phase is bound by [Section 8 — Interface Design Direction](#8-interface-design-direction),
which is a hard constraint on the finished result, not a suggestion.

**Screens produced.**

- **`maktaba` — the Library, the landing screen.** Every installed game across every
  launcher as an artwork grid, scanned automatically on first run and refreshed
  incrementally. Each card carries its Arabization status at a glance, in five states:
  رقعة متوفرة (patch available), رقعة مثبّتة (patch installed), لا توجد رقعة بعد (no patch
  yet), المحرك مدعوم (engine supported, unpatched), غير مدعومة (unsupported). Search across
  name and launcher; filters by launcher, engine, tier and status; sorting by name, recency,
  size and playtime; a source selector; a density toggle between grid and table. The grid is
  virtualized and scrolls tens of thousands of rows without stutter. Right-click gives a
  context menu: install, launch, open location, verify, hide, add to a collection.
- **`luba` — Game detail.** Artwork at the top with the game's own dominant colour taken as
  the accent for this screen only. Install path, launcher, detected engine and version,
  scripting backend, graphics API, build id, and the capability report in plain Arabic —
  what will be Arabized, what will not, what the tier means. Below it, every patch available
  for this exact build with contributor, coverage, method, rating, size and date, ranked by
  match quality with the differences between them shown rather than hidden. Actions:
  install, uninstall, verify, launch, export, report. If no patch exists, the screen shows
  the translation request board entry for this game and the button to request or to start
  translating it.
- **`tathbeet` — the installation flow.** Staged, visible, and honest: verifying
  compatibility → checking anti-cheat → downloading → verifying signature → backing up
  originals → installing framework → placing files → confirming. Each stage shows what it is
  doing and what it found. A failure stops at its stage and states in plain Arabic exactly
  what happened and exactly what to do next — never a code, never an apology, never a
  generic retry.
- **`warsha` — the translation workspace.** A multi-pane layout: the string list on one side
  (virtualized, filterable by status and quality flag, searchable across source and target
  simultaneously, multi-selectable with modifier and range keys), the side-by-side editor in
  the centre, and the context panel showing where the string appears in the game — its
  container, its object path, its speaker, its surrounding dialogue, and the screenshot crop
  from runtime capture where one exists. Inline glossary and translation memory suggestions
  appear as you type. Machine translation with one-click acceptance per string and batch
  operations across a selection. Review comments from the owner appear anchored to the exact
  string they concern.
- **`muayana` — Live preview.** The screen that makes the tool trustworthy: selected strings
  rendered exactly as they will appear in the game, through the real `saff` engine, at the
  real pixel size, inside the real interface bounds recorded for that string, with the
  overflow warning list and the measured widths beside them. Toggle between coverage and SDF
  rendering, between fonts, and between sizes. What is shown here is what will ship.
- **`taqdeem` — Submission** and **`musahamat` — Contributions**, exactly as specified in
  Phase 18: the pre-flight checklist with every failure linking to its strings, and the
  contributions list with live status and the full review conversation.
- **`muraja` — the Review Console**, exactly as specified in Phase 18, owner only, with its
  routes and its command palette entries absent from every other session.
- **`tabaqa` — Overlay control.** Configuration and live control of the tier-3 overlay:
  capture regions with a visual editor, keyboard shortcuts, appearance (position, opacity,
  size, backing plate), OCR engine selection, and the reading history.
- **`idadat` — Settings.** Launcher paths, patch storage location, interface language, font
  selection with validation feedback, machine translation providers and credentials
  (stored in the OS keychain, shown as "محفوظ" and never echoed), update behaviour, registry
  sources including offline mirrors and network shares, and diagnostics level.
- **`tashkhis` — Diagnostics.** A live log viewer with filtering by level, span, game and
  time; the compatibility report exporter; and a one-click bundle containing everything a
  maintainer needs — logs, capability reports, installation manifests, versions, and the
  environment — with credentials and user paths redacted, listed explicitly before it is
  written so the user sees what they are about to share.
- **`talabat` — the requests board**, browsable by demand, engine and feasibility.

**Interaction craft, all required and all part of the definition of done:** a command
palette reachable from anywhere that can drive every action in the product; complete
keyboard navigation with visible focus; context menus on game cards and string rows;
multi-select with modifier and range selection in every list; virtualized rendering for the
library grid and the string list; progressive artwork loading with a placeholder derived
from the artwork's own colours; skeleton states shaped like the content they replace;
optimistic updates with quiet reconciliation; inline editing wherever a value is editable;
and undo for every destructive action.

**Technology.** React 19 with TypeScript in strict mode; Vite; TanStack Router for typed
routing, TanStack Query for backend state, TanStack Virtual for virtualization; Zustand for
local UI state; Radix primitives, styled entirely by the Taarib token layer so that nothing
is recognisable as a component library default; Tailwind with the default palette, spacing
and radii scales removed and replaced by Taarib tokens; Motion for spring transitions;
Lucide icons at a single consistent stroke weight; `tauri-specta`-generated command and
event bindings so no request shape is written twice.

**Hard constraints.**

- Every rendered preview of Arabic comes from `saff` through a Tauri command. The frontend
  never shapes text itself, never uses a browser-rendered approximation in a preview, and
  never lies about what will ship.
- No screen blocks on the network. Every list renders from local state first.
- Section 8 is binding, in full, including every explicit prohibition in it.
- No dead controls. Every button in this phase reaches real functionality by Phase 23.

**Dependencies.** Phases 4, 5, 11, 12, 13, 14, 15, 16, 17, 18, 19.

**Definition of done.** Every screen listed exists, is reachable, is populated with real
data from the backend, and honours Section 8; the command palette drives every action; the
library and string list virtualize; previews render through the real engine; the owner
console is absent from contributor sessions.

---

### Phase 21 — Application Localization and Accessibility

**Scope.** `apps/studio/src/lugha` — make the right-to-left interface structurally correct
rather than cosmetically flipped, and make the whole product usable without a mouse and at
any size.

**Components produced.**

- **Structural right-to-left.** Layout is written in CSS logical properties throughout —
  `margin-inline-start`, `padding-inline-end`, `inset-inline`, `border-inline-start` — so
  direction is a document property, not a set of overrides. What must be mirrored explicitly:
  navigation and sidebar position, icon direction for anything directional (back, forward,
  expand, indent, sort), progress direction so a bar fills from the right, table column
  order including headers and alignment, tree indentation, drag handles, slider direction and
  keyboard semantics, scroll position restoration, and animation direction so a panel that
  enters from the leading edge does so on the correct side. What must *not* be mirrored:
  media controls, checkmarks, magnifiers, and any glyph whose meaning is not directional.
- **Complete Arabic and English string sets.** Every user-visible string in `ar.json` and
  `en.json` with no fallback to a key and no English leaking into an Arabic session.
  Pluralization uses Arabic's six forms through the Intl plural rules, not a two-form
  approximation. Number, date, percentage, byte-size and duration formatting go through the
  Arabic locale with the digit system the user selected. Every error, warning, empty state
  and confirmation is written in Arabic first and translated to English second, because the
  Arabic is the primary text and reads better when it is not a translation of a translation.
- **Full keyboard navigation.** Every interactive element is reachable and operable from the
  keyboard, in a logical order that follows the visual order in both directions. Focus is
  always visible and never trapped except in modals, which return focus on close. Every
  list supports arrow navigation, type-ahead, home/end, and range selection with shift.
  Shortcuts are discoverable through the command palette and a shortcuts reference, and are
  remappable.
- **Scalable interface sizing.** The whole interface scales through a root size token from
  compact through comfortable to large, with no layout breaking at any step and no text
  clipped. This is not browser zoom; it is a token the design system respects.
- **High contrast option.** A separate token set meeting a strict contrast floor for text,
  borders, focus rings and status colours, selectable independently of light and dark.
- **Screen reader correctness.** Semantic roles, live regions for progress and status,
  labelled controls, and `lang` and `dir` attributes correct per element so a screen reader
  switches voice at a language boundary inside a sentence.

**Technology.** CSS logical properties; `Intl` for plural, number and date formatting; a
typed translation function generated from the string set so a missing key is a compile
error, not a runtime placeholder.

**Hard constraints.**

- No physical `left`/`right` CSS property anywhere in application styling.
- No untranslated string in either language. A missing key fails the build.
- Keyboard operability is a definition-of-done item on every screen, not a pass at the end.

**Dependencies.** Phase 20.

**Definition of done.** The interface is structurally mirrored in Arabic and correct in
English; both string sets are complete with correct plural and number formatting; every
screen is fully keyboard operable with visible focus; sizing scales cleanly; high contrast
is available; screen reader semantics are correct.

---

### Phase 22 — Packaging and Distribution

**Scope.** `packaging/` — turn the built tree into installable artifacts for three
platforms, with every native library, adapter, plugin assembly and font in the right place,
and a self-update mechanism that works.

**Components produced.**

- **Windows.** An MSI and an NSIS installer produced through Tauri's bundler, plus a
  portable ZIP. Bundled: the Studio binary, the WebView2 bootstrapper handling for machines
  without it, `taarib_jisr.dll` in x86_64 and i686, the injected modules for Unreal, Godot 3
  and the overlay in both architectures, the BepInEx payloads for Mono and IL2CPP, the C#
  plugin assemblies, the script adapters, the fonts, and the signature database. Code signing
  configuration is present and used when a certificate is available; when it is not, the
  installer states plainly that it is unsigned rather than pretending otherwise.
- **Linux.** An AppImage as the primary artifact (works on every distribution including
  SteamOS's immutable root), a Flatpak manifest with the correct permissions declared
  narrowly — home, Steam directories, and network — and `.deb` and `.rpm` packages.
  Bundled: the Studio binary, `libtaarib_jisr.so` in x86_64, i686 and aarch64, the Linux
  injected modules, the Vulkan layer manifest installed to the correct search path, and the
  full Windows-side payload set as well, because a Proton game needs the Windows framework
  installed into its prefix. **Steam Deck** is a first-class target: the AppImage runs in
  Game Mode, the interface is usable at 1280×800 with the compact sizing token, controller
  navigation maps to the keyboard model, and every write goes to a user-writable path.
- **macOS.** A `.app` bundle inside a DMG, universal binary for x86_64 and aarch64,
  hardened runtime with the entitlements the product actually needs and no more, notarization
  configuration, and honest capability reporting for the injection paths that macOS
  forbids.
- **`tahdith_dhati` — self-update.** Tauri's updater against a signed release feed: check on
  a schedule and on demand, show what changed in Arabic, download in the background with
  resume, verify the update signature against the embedded key, apply on restart, and roll
  back if the new version fails to start. Update behaviour is a user setting including a
  fully manual mode, because a tool that modifies game files must never surprise the user
  with a version change mid-session.
- **Reproducible builds** where the toolchain allows: pinned toolchain, pinned dependency
  versions, no build-time network fetches beyond the pinned registry, and a manifest of every
  bundled artifact with its hash so a user can verify what they installed.

**Technology.** Tauri 2 bundler; `cargo-xwin`/`cross` configuration for cross-compilation;
the C# projects published as release assemblies with deterministic builds; a bundling
manifest describing every payload's destination per platform.

**Hard constraints.**

- Every native library ships in every architecture a target game process can be.
- No payload is fetched at first run. Everything needed to patch a game is in the package.
- Updates are signature-verified against a key embedded at build time.
- The Steam Deck path is tested against the real constraints: immutable root, Game Mode,
  controller input, and 1280×800.

**Dependencies.** All prior phases.

**Definition of done.** Installable artifacts exist for all three platforms with every
payload correctly placed; the Steam Deck path works in Game Mode; self-update checks,
downloads, verifies, applies and rolls back; the bundle manifest lists every artifact hash.

---

### Phase 23 — Final Integration

**Scope.** Wire everything into one running application and remove every seam.

**Work performed.**

- **Every screen reaches real functionality.** Every button, menu item, context action and
  command palette entry in Phase 20 is connected to the backend command that performs it.
  Nothing renders sample data. Nothing is disabled with a note.
- **Every adapter is registered and dispatched by the capability probe.** A single dispatch
  table maps `Muharrik` → adapter → framework payload → installation strategy, and the probe
  is the only thing that chooses. There is no engine-specific branch anywhere else in the
  product; adding an engine means adding a row.
- **Every error path surfaces in plain language.** Every `Khata` variant produced anywhere in
  the product has an Arabic sentence, an English sentence, and a next action, and reaches a
  place in the interface where the user will actually see it. A walk of the error enum
  against the interface confirms there is no variant that can only appear in a log.
- **The end-to-end paths are wired and complete:**
  1. Launch → scan → probe → registry badges → detail → install → launch in Arabic.
  2. Launch → game with no patch → extract → translate → preview → compile → submit.
  3. Owner launch → queue → review with previews and diffs → sandbox → approve → sign →
     publish → other clients pick it up.
  4. Offline: import a `.ruqaa` from disk → verify → install → play, with no network at all.
  5. Store update → detected → fingerprint re-match → migrate → reinstall.
  6. Uninstall → restore → verify byte-identical original.
- **No dead code, no disconnected module, no unreachable screen.** Every crate in the
  workspace is reachable from the Studio binary or from an injected payload. Every exported
  function is called. Every module in the tree is used by something.
- **A final pass over the whole codebase**: duplicate logic collapsed, complex functions
  simplified, algorithms with a better shape rewritten, naming checked against the lexicon in
  Section 4.1, and every constraint in this roadmap verified against the code that claims to
  satisfy it.

**Hard constraints.** No placeholder, no stub, no `TODO`, no `unimplemented!`, no commented
"would be implemented later", anywhere in the tree, at the end of this phase.

**Dependencies.** All prior phases.

**Definition of done.** All six end-to-end paths run through real code from the interface;
the dispatch table covers every engine; every error variant is reachable and readable; the
workspace contains no dead code and no unreachable screen; the codebase is ready for the
build phase.

---

## 7. Dependency Graph

```
        ┌──────────────────────────── 0  Workspace Foundation ────────────────────────────┐
        │                                                                                 │
        ▼                                                                                 ▼
   1  saff  (Arabic engine) ──► 2  lawha  (atlas) ──► 3  jisr  (C ABI)              4  kashf  (discovery)
        │                            │                    │                               │
        │                            │                    │                               ▼
        │                            │                    │                        5  muharrik  (probe)
        │                            │                    │                               │
        │                            │        ┌───────────┴───────────┬───────────┬───────┴───────┐
        │                            │        ▼                       ▼           ▼               ▼
        │                            │   6  Unity Mono           8  Unreal    9  Godot     10  Script engines
        │                            │        │                       │           │               │
        │                            │        ▼                       │           │               │
        │                            │   7  Unity IL2CPP              │           │               │
        │                            │        └───────────┬───────────┴───────────┴───────────────┘
        │                            │                    │
        │                            │                    ▼
        │                            │            11  tabaqa  (universal overlay)  ◄── needs 13
        │                            │
        ▼                            ▼
  12  istikhraj  ──►  13  tarjama  ──►  14  tarqee + ruqaa  ──►  15  tathbeet
        ▲                    │                   │                     ▲
        │                    │                   ▼                     │
        │                    │            16  aman  ──────────────────►┘
        │                    │                   │
        │                    ▼                   ▼
        │            19  warsha           17  mustawda  (registry)
        │                    │                   │
        │                    └────────┬──────────┘
        │                             ▼
        │                     18  taqdeem + muraja
        │                             │
        └─────────────────────────────┼──────────────► 20  Taarib Studio
                                      │                      │
                                      │                      ▼
                                      │               21  Localization & Accessibility
                                      │                      │
                                      │                      ▼
                                      └──────────────► 22  Packaging  ──►  23  Final Integration
```

**Reading it:**

- **0 blocks everything.** Nothing is written before the vocabulary, the error model, the
  paths and the schemas exist, because every later phase serializes something.
- **1 → 2 → 3 is the critical path** and the highest-risk sequence in the product. Every
  adapter, the compiler, the preview and the overlay all sit on top of it. It is built
  first and it is built to stand alone.
- **4 → 5 gates every adapter.** No adapter is written before the probe can say which game
  it applies to, because an adapter with no dispatch is untestable in the product.
- **6 → 7 is deliberate.** The Mono adapter establishes the rendering logic in shared code;
  the IL2CPP adapter reuses it and adds only resolution. Writing them in the other order
  would produce two implementations.
- **14 is the hinge of the second half.** The container format must exist before adapters
  can read patches, before installation can place them, before safety can verify them, and
  before the registry can distribute them.
- **16 gates 15.** Safety runs before installation, structurally, not by call order
  convention: `tathbeet` takes a proof-of-safety value that only `aman` can construct.
- **17 → 18 is one system in two halves.** The registry client is written first because the
  submission flow publishes into exactly the structures the client reads.
- **11 depends on 13**, not only on the engine work, because an overlay with no translation
  behind it displays nothing.
- **20 depends on nearly everything**, which is why it is late: the interface is built
  against real backends, never against fixtures.
- **23 touches every phase** and is the only phase permitted to modify earlier code broadly.

---

## 8. Interface Design Direction

This section is a binding constraint on Phase 20 and is judged on the finished result, not
on intent. The interface must look like a professional product from a design-led team. It
must not look machine-generated.

### 8.1 Explicitly forbidden

Not stylistic preferences — prohibitions. None of these appears anywhere in the product:

- Purple, indigo or violet accents.
- Gradients of any kind: gradient text, gradient buttons, gradient backgrounds, gradient
  borders, gradient overlays on artwork.
- Glassmorphism, frosted blur panels, translucent floating cards.
- Uniform heavily-rounded corners applied to everything.
- Emoji anywhere in the interface.
- Three-column icon-heading-paragraph feature grids.
- Large centered hero sections with oversized display text.
- Component library defaults left visually recognisable as defaults.
- Decorative illustrations, mascots, abstract blob shapes.
- Soft diffuse drop shadows applied to everything.
- Centered single-column layouts for screens holding real data.

### 8.2 The direction

The density and restraint of professional desktop tools and game launchers. A serious
application someone opens every day, not a landing page. Information is close together,
aligned, and legible; decoration earns no vertical space. The reference points are a
version-control client, a digital audio workstation's browser, and a well-made game
launcher — not a marketing site.

### 8.3 Colour

A neutral near-black foundation. Elevation is expressed by **subtle luminance shifts plus
hairline borders**, never by shadows on panels. Shadows exist only for true floating
overlays — menus, popovers, dialogs — and are tight and dark rather than soft and diffuse.

```
Dark (default)
  sath-0   #0B0C0D   application background
  sath-1   #121415   sidebar, base surfaces
  sath-2   #17191B   panels, cards, table backgrounds
  sath-3   #1D2022   raised: menus, popovers, dialogs
  sath-4   #24272A   hover on raised surfaces
  hadd-daif  rgba(255,255,255,0.06)   hairline, low emphasis
  hadd       rgba(255,255,255,0.10)   hairline, default
  hadd-qawi  rgba(255,255,255,0.16)   hairline, emphasis / focus outline base
  nass-1   #E8EAEC   primary text
  nass-2   #A8AEB4   secondary text
  nass-3   #6E767D   tertiary and disabled text

Light
  sath-0 #F5F6F7 · sath-1 #FFFFFF · sath-2 #FAFBFB · sath-3 #FFFFFF · sath-4 #F0F2F3
  hadd-daif rgba(0,0,0,0.07) · hadd rgba(0,0,0,0.11) · hadd-qawi rgba(0,0,0,0.18)
  nass-1 #16191C · nass-2 #4E565D · nass-3 #838C93

Accent — one, used sparingly, for primary actions and active state only
  tamyeez        #2A9D8F   Persian green
  tamyeez-qawi   #34B4A3   hover
  tamyeez-khafit rgba(42,157,143,0.16)   active row tint, selected state

Status — used as dots, badges and hairlines, never as panel fills
  najah  #57A773   success        tanbeeh #C99A2E   warning
  khatar #D05353   danger         maalumat #5B8DBE  information
```

**The per-game accent.** On a game's detail screen, the dominant colour extracted from that
game's cover artwork replaces `tamyeez` as the accent for that screen only, after being
normalised: clamped to a legibility-safe lightness and chroma range against `sath-0`, and
rejected back to the default accent when the artwork yields a colour too close to the
status colours or too desaturated to read as intentional. The interface takes on the
character of whatever the user is looking at without ever becoming unreadable.

### 8.4 Typography

One Arabic family with a full weight range, chosen for interface work rather than display:
**IBM Plex Sans Arabic** — precise, technical, legible at small sizes, and unusually well
made at low optical sizes for an Arabic face. Its Latin companion **IBM Plex Sans** and
monospace **IBM Plex Mono** complete the set. Latin and Arabic share metrics closely enough
that a mixed line does not visibly step.

A strict scale of six steps, used without exception:

```
qiyas-1   11 / 16    labels, table metadata, badges, keyboard hints
qiyas-2   12.5 / 18  secondary text, table cells, captions
qiyas-3   14 / 20    body and default interface text
qiyas-4   16 / 24    emphasised body, section headings
qiyas-5   20 / 28    screen titles
qiyas-6   28 / 34    the game title on the detail header — nowhere else

Weights: 400 body · 500 emphasis and headings · 600 screen titles. Nothing heavier.
Line height: 1.35–1.45 for interface text. 1.75 for Arabic reading text, because Arabic
needs the leading — descenders and diacritics collide at Latin line heights.
```

**Monospace is not decorative.** Identifiers, paths, hashes, build ids and app ids are
always IBM Plex Mono, always with Latin digits, and always rendered with an explicit
left-to-right isolation so that the bidirectional algorithm cannot reorder a path or a hash
inside an Arabic sentence. Getting this wrong is the most common way an Arabic technical
interface embarrasses itself, and it is prevented structurally by a single component that
every identifier goes through.

### 8.5 Density and layout

- A **persistent sidebar**, 232px, collapsible to a 56px icon rail. The content area never
  reloads or remounts on navigation; only its inner region changes.
- **Real data tables** where data is tabular — the queue, the string list, the installation
  manifest, the diagnostics log. Rows 32px compact / 36px comfortable, hairline dividers at
  `hadd-daif`, sticky headers, resizable and reorderable columns, right-aligned numerics
  with tabular figures.
- **Multi-pane layouts** in the workspace and the review console: list, editor, context —
  resizable, with sizes persisted per screen.
- **The library grid** uses 2:3 artwork cards at a density the user chooses, with the status
  badge on the card itself and the name below it, never overlaid on the art.
- Spacing scale: 2, 4, 6, 8, 12, 16, 20, 24, 32, 40, 56. Nothing between the steps.
- Radii: 3px controls, 4px panels and cards, 6px dialogs, 0 for table rows and dividers.
  Corners are a detail, not a style.

### 8.6 Motion

Purposeful and fast. Spring-based, short. Elements move between states rather than appearing
and disappearing.

```
hover / press / state    120ms   spring: stiffness 420, damping 38, mass 0.9
panel, pane, disclosure  180ms   same spring family
route and shared element 240ms   shared-layout transition, position and size only
```

No bouncing. No attention-seeking animation. **No animation on data updates** — a table that
re-sorts itself with a flourish is a table nobody can read. `prefers-reduced-motion` is
honoured by making transitions instant, not by making them slower.

### 8.7 Interaction craft — all required

- **Command palette** reachable from anywhere by keyboard, driving every action in the
  product: navigate, install, uninstall, translate, filter, submit, review, open diagnostics.
  It is the fastest path to everything, not a search box for screens.
- **Full keyboard navigation** with visible focus states on every interactive element.
- **Context menus** on game cards and string rows, with the same actions the palette exposes.
- **Multi-select** with modifier and range selection in every list, and batch actions that
  state exactly how many items they will affect.
- **Virtualized rendering** for the library grid and the string list, so tens of thousands of
  rows scroll without stutter.
- **Progressive artwork loading** with a placeholder derived from the artwork's own extracted
  colours — never a grey rectangle, never a spinner over a card.
- **Skeleton states shaped like the content they replace**, with the same row heights and
  column positions, so nothing shifts when data arrives.
- **Optimistic updates with quiet reconciliation**: the interface responds immediately and
  corrects itself without announcement if the backend disagrees.
- **Inline editing** wherever a value is editable — a translation, a glossary term, a
  launcher path, a credit line.
- **Undo for every destructive action**, surfaced as a brief, non-blocking affordance in the
  same region as the action, not as a modal.

### 8.8 Empty and error states

Plain, specific Arabic sentences saying exactly what happened and exactly what to do next.
Never a shrug illustration. Never a generic apology. Never an error code alone.

```
لم يُعثر على أي لعبة مثبتة. تأكد من تشغيل ستيم مرة واحدة على الأقل، أو أضف لعبة يدويًا
من زر «إضافة لعبة».

هذه اللعبة تستخدم نظام حماية «إيزي أنتي تشيت». لن يقوم تعريب بتعديلها، لأن تعديل ملفاتها
قد يؤدي إلى حظر حسابك نهائيًا.

توقّف التنزيل عند 42٪ لانقطاع الاتصال. أعِد المحاولة وسيُكمل التنزيل من حيث توقف.

٧ نصوص تتجاوز عرض الواجهة. افتح تقرير التجاوز لمراجعتها قبل الإرسال.
```

### 8.9 Craft details that must be present

Precise optical alignment — icons optically centred against text rather than
mathematically centred. A single consistent icon stroke weight (1.5px) at two sizes
(16px inline, 20px navigation), with no mixed icon families. Correct number formatting for
the Arabic locale, with the digit system the user chose, and tabular figures in every
column of numbers. Real elevation logic, applied consistently, where a surface's level is a
property of what it is rather than a decision made per component. Hairline dividers at the
correct opacity for their level. Hover states that respond immediately, on the first frame,
with no delay and no transition-in on the pointer entering.

The finished interface should be something a user screenshots and shares because it looks
good — not because it looks like software.

---

## 9. Operating Rules for the Build

These govern the entire build and are restated here because they are part of the contract.

1. **Source code only.** No tests, no test files, no fixtures, no benchmarks, no test
   harnesses anywhere in the tree.
2. **No execution.** No builds, no compilers, no linkers, no package managers, no shell
   commands. Nothing is verified by running it.
3. **No questions, no options.** Every decision is made and carried out.
4. **Continuous execution.** Once the build begins, the phases run in order without pausing
   for approval between them.
5. **No placeholders.** No `TODO`, no `unimplemented!`, no stub bodies, no
   `throw new NotImplementedException()`, no comment saying something would be implemented
   later. Every function has a real body, every screen has real markup, every adapter has
   real logic.
6. **This is the only version.** There is no v2, no future work, no out of scope, no phase
   two. Everything in this document ships in this build.
7. **Naming mirrors the domain**, per the naming law in Section 4.1. No `utils`, no
   `helpers`, no `common`, no `manager`, no `service`, no `handler`.
8. **Stop at Phase 23.** When Phase 23 is complete, the codebase is ready for the build
   phase, and that is the only thing reported.

### Standing engineering rules applied throughout

- **Errors are values with user-facing text.** Every failure that can reach a user carries
  an Arabic sentence, an English sentence, and a next action. No error reaches the interface
  as a code or a stack trace.
- **Nothing panics across a boundary.** Every ABI entry point, every Tauri command, and every
  injected hook catches unwinding and converts it into a reported failure.
- **Every file write that could be interrupted is atomic** — write beside, flush, rename —
  and every modification is preceded by a backup and a manifest entry.
- **No secret is ever written to disk in plaintext, logged, or included in a diagnostics
  bundle.** Credentials live in the OS keychain.
- **No `unsafe` without a written invariant** naming what is guaranteed and by whom.
- **Concurrency is structured**: bounded channels, no detached tasks, cancellation propagated,
  no lock held across an await point, and no shared mutable state on a render path.
- **Every resource has an owner and a drop path.** Injected modules unload cleanly, hooks
  uninstall, atlases free, handles are generation-tagged against use-after-free.
- **The user can always get back to where they started.** Every install has an uninstall that
  restores byte-exact originals and verifies that it did.

---

*ROADMAP.md — Taarib. This document is the contract. The build is executed against it.*










