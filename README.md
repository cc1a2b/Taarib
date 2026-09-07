# Taarib

<div align="center">

[![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.95-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/tauri-2-24C8DB?style=flat&logo=tauri)](https://tauri.app)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-lightgrey)](https://github.com/cc1a2b/taarib/releases)
[![Steam Deck](https://img.shields.io/badge/Steam%20Deck-AppImage-1a9fff?style=flat&logo=steamdeck)](docs/tawzee/steamdeck.md)

**Arabic in PC games that were never built to accept it.**

[العربية](README.ar.md)

</div>

## About

Taarib (تعريب — *Arabization*) is a desktop application that installs Arabic into
games with no Arabic support of any kind, and gives translators the tools to
produce those Arabic versions in the first place.

Most people who use it never translate anything. They open it, see every game
installed on their machine across every launcher, notice that some of them have
a community Arabic patch, press one button, and play in Arabic.

The rest — the translators — use the same application to pull every translatable
string out of a game, translate it with machine assistance and human review, see
how each line will render inside the real interface bounds before it ships,
compile it into a signed patch, and submit it for publication.

<!--
  SCREENSHOT: deliberately absent rather than broken.

  A capture of the library screen belongs here and would be the first thing a
  reader looks at. The one that was here is gone, and its replacement has one
  requirement worth stating so it is not reintroduced: the status strip along
  the bottom of the window prints the resolved data root, which on Windows is
  `C:\Users\<name>\AppData\Roaming\Taarib`. A screenshot carries that as pixels,
  where no text search will ever find it. Capture under a profile whose name is
  not yours, or crop the strip.

  Put it at `assets/laqatat/maktaba.png` and restore the embed here and in
  README.ar.md. A broken image is worse than none, which is why there is no
  `<img>` tag below.
-->

### Why this is hard, and why existing tools get it wrong

Arabic is not a font problem. Dropping an Arabic font into a game that has no
Arabic support produces text that is broken in four independent ways at once:

- **No contextual joining.** Arabic letters change shape according to their
  neighbours — isolated, initial, medial, final. A naive renderer draws
  twenty-eight disconnected isolated forms. That is not ugly Arabic; it is not
  Arabic, and a reader cannot parse it.
- **No ligature formation.** Lam-alef is mandatory, not decorative: ل + ا must
  become لا. A Naskh font carries hundreds more required ligatures in its
  substitution tables, and without a shaper none of them fire.
- **No bidirectional layout.** Arabic runs right to left, numbers run left to
  right, embedded Latin runs left to right, punctuation takes direction from
  context, and brackets must mirror. Reversing the string "to make it RTL" gets
  every number backwards — in a game full of item counts and percentages, that is
  constant.
- **No mark positioning.** Diacritics are combining marks that must attach to
  their base glyph through the font's own attachment tables, and stack correctly
  when several land on one base. Drawn naively they float, collide, or vanish.

The prevailing approach in Arabic game patches today converts text into Unicode
**Presentation Forms** (U+FB50–FDFF, U+FE70–FEFF), reverses it manually into
visual order, and hands the result to the game as plain left-to-right text. It
looks like Arabic in a screenshot. It is also a legacy compatibility encoding
covering a fraction of what real font tables express, nothing at all for Persian
or Urdu, undefined for diacritics, unsearchable, uncopyable, and broken the
moment a number appears.

**Taarib never produces a presentation form.** Every contextual form, every
ligature and every diacritic position comes from the font's own OpenType `GSUB`
and `GPOS` tables through a real shaping engine. There is no fallback path around
that, because a fallback would be the wrong answer arriving quietly. If you take
one architectural note away from this repository, take
[`docs/taqdimiya.md`](docs/taqdimiya.md).

### The other half of the problem

Even where a good patch exists, it lives in a forum thread, a Discord message, or
a dead file host — no versioning, no integrity guarantee, no safety check, and no
way to know whether it matches the build you have installed. Most Arabic-speaking
players never find it.

Taarib ships a curated community registry: a public Git repository of signed
patches, sharded so a client fetches only the slice covering games it actually
owns, matched against your installed build in three tiers, verified before
installation, and reviewed by a person before publication.

---

## Table of Contents

- [About](#about)
- [What works today](#what-works-today)
- [How it works](#how-it-works)
- [Coverage](#coverage)
- [Safety](#safety)
- [The registry](#the-registry)
- [Installation](#installation)
- [Quick start](#quick-start)
- [Repository layout](#repository-layout)
- [Building from source](#building-from-source)
- [Command reference](#command-reference)
- [Legal position](#legal-position)
- [Contributing](#contributing)
- [License](#license)
- [Support](#support)

---

## What works today

This is the section to read before any other. Everything below it describes what
Taarib is built to do; this describes what has been observed doing it.

### The static pipeline is proven end to end

Discovery, engine identification, extraction, placeholder-protected
translation, layout, atlas generation, patch compilation and sealing have been
run against **real installed games**. Install, verification and byte-identical
restore have been run against a **synthetic fixture** — deliberately, because
writing into a game nobody consented to touch is not something to do for a
demonstration.

The real-game walk went from a game directory to a signed package. The game was
R.E.P.O., appid 3241660, build 23363152, Unity 2022.3.67f2 on the Mono backend,
1.49 GB on disk:

```
  istakhrij took 917.604692ms
  containers read : 28
  refusals        : 20
  strings counted : 3609 (sum of per-container counts)
  3597 distinct string(s) in the table
  3596 of them a player is believed to see
```

```
  taarib_tarqee::mujammi::ijmaa:
    compiled in 295.522458ms: REPO — Arabic r1 for REPO — 60 translated string(s),
    60 precomputed layout(s), 1 atlas page(s) rasterized from 1 bundled font(s),
    0 container difference(s), and 0 bytes of original game content
```

```
  sealed under the committed development key MIFTAH_TATWIR, whose private half this machine's keychain holds
    the signature verifies under key e4260a5f02029a64b6a41a31a5db53e0af74d6110122d9a352a721a2438b4cea
  wrote .../repo.ruqaa (45472 bytes)
  reopened through MalafRuqaa (memory-mapped) and validated
```

Note `0 bytes of original game content`. That is the provenance gate reporting,
not a claim in a comment — see [Legal position](#legal-position).

Placeholder protection was exercised adversarially. Twenty-two round trips ran
and twenty-two verified, including this one, where a format placeholder and a
rich-text tag share a sentence:

```
    clean    : "Use ￼ to view the map. The map is IMPORTANT for navigation."
    to model : "Use ⟦0⟧ to view the map. The map is ⟦1⟧IMPORTANT⟦/1⟧ for navigation."
    reply    : "استخدم ⟦0⟧ لعرض الخريطة. الخريطة ⟦1⟧مهمة⟦/1⟧ للتنقل."
    restored : "استخدم [map] لعرض الخريطة. الخريطة مهمة للتنقل."
    verdict  : every atom back once, byte-identical, at its span: true
```

and the guard refuses damaged replies rather than shipping them:

```
    token dropped     : refused  -> ⟦0⟧ is missing from the reply
    token invented    : refused  -> ⟦1⟧ is in the reply and was never sent
    token duplicated  : refused  -> ⟦0⟧ came back 2 times; it was sent once
    token reordered   : accepted
```

On the synthetic side, both the product install path and the harder
replace-a-container path ended the same way:

```
  restore ran. report:
    KhayalSaif [text]: restored 1 original(s), deleted 2 added file(s), removed 4 director(ies)
  RESULT: the restored copy is byte-for-byte identical to the pristine copy.
```

### Four caveats on the paragraph above, so it is not read as more than it is

1. **Install and restore were proven on a synthetic fixture, not on a real
   installed game.** The real-game walks stop before writing anything, by
   design, and say so: *"the owner has not consented to the game being touched.
   A byte-identical restore proven on the synthetic fixture is not consent.
   Nothing under the game root was created, modified, renamed or deleted by this
   walk."* Before-and-after snapshots of each real game directory were taken and
   compared; all pairs are byte-identical.
2. **No language model was called.** The translation step in the recorded runs
   used a fixed English-to-Arabic lookup table. What was genuinely exercised is
   the *placeholder protection around* the model call — tokenization, the reply
   guard, and byte-identical restoration of every atom — not the quality of a
   translation.
3. **Overflow was never measured against real widths.** Every overflow report on
   every run reads `not measurable: Extraction recorded no available width for
   this string`. The accompanying `0 overflowed, 0 were shrunk, 0 were
   truncated` is therefore vacuously true and must not be quoted as a quality
   result.
4. **Extraction is not universal.** Against Hollow Knight (appid 367520, Unity
   6000.0.61f1, 5.2 GB) the static path produced **zero strings** from 1,006
   containers read and 1,617 refused. The product names the reason rather than
   guessing: *"This game is built in a way that strips the description of its
   own data layout... a length prefix followed by a string is byte-identical to
   two integers, and guessing would produce text that is not text. The strings
   exist and are drawn on screen; capture reads them there."* The remedy it
   offers is runtime capture, and runtime capture has not been exercised.

### Nothing has ever been observed rendering inside a running game

This is the honest state of the in-game half, and it is the gap between what
Taarib compiles and what a player would see.

No adapter has been deployed into a real game process. No overlay has attached to
a swap chain. **No Arabic produced by this product has been observed on screen
inside a game.** The harnesses that produced the output above say so in their own
headers — *"nothing here observes Arabic rendering, because nothing here renders
anything"*.

That sentence is narrower than it sounds, and the table below is where the
nuance lives. Parts of the in-game half now genuinely work: a Ren'Py game on a
shaping engine comes out Arabized, the Unity assemblies compile and stage, and
Godot 3's delivery was verified against the real engine binary. What has not
happened is anybody launching a game and looking at it. Until that happens,
"works" here means "the code does what its format demands", which is a real
claim and a smaller one.

[`docs/bidaya.md`](docs/bidaya.md) §6 and [`docs/tashghil.md`](docs/tashghil.md)
record where each in-game path stops, function by function.

**One engine is reported by the application as complete today — Ren'Py at 7.4
and above — and that verdict was reached by reading three crates, not by
watching a screen.** Everything else stops at a named point. The table says
where, because "not supported yet" without a location is not something anyone
can act on or verify.

Three different things are meant by "does not work", and they matter differently
to you:

1. **The write never happens.** The patch installs or refuses; the game is
   untouched either way.
2. **The write happens and the install refuses**, because a component this
   build does not ship is missing. Nothing reaches the game.
3. **The write happens, the install succeeds, and the engine cannot draw the
   result.** The game changes *for the worse* — text you could read becomes
   text you cannot. This is the state to watch for, and the application refuses
   the one-button run on it for exactly that reason.

| Engine | Designed tier | State | Where it stops |
| --- | --- | --- | --- |
| Ren'Py 7.4+ | 1 — text replaced in the game | **reported complete; unobserved** | Translation, language selection and text direction all install, and the engine shapes Arabic itself. The Ren'Py component now carries Noto Naskh Arabic, the deployment plan picks that face out of the component store and names it in the generated `.rpy`, and `imkaniyat::jahiziya` answers `Mukammala` — the only engine where it does. Two limits: a bundle staged without the face gets the same verdict and draws the translation in the game's own font (legible on a DejaVu-based GUI, empty boxes on a Latin-only one), and the one-button pipeline registers no face at all. No Ren'Py game has been launched with any of this. |
| Ren'Py before 7.4 | 1 | **3 — visibly worse** | The older per-character path. `taarib_jisr` is loaded from beside the Python package, and the staging step ships the package and the face, never the library. |
| GameMaker Studio | 1 | **3 — visibly worse** | `data.win`'s string pool really is rewritten. The glyph functions are never called, so Arabic lands in a pool whose baked `FONT` table holds no pictures for it. Blank menus, not English ones. |
| RPG Maker MV / MZ | 1 | 2 | The data splice is real and has a caller. The install then refuses when the component store lacks the runtime plugin. `adapters-script/ibni.mjs` now builds that plugin and `scripts/isdar.sh` stages it, so a bundle built the documented way no longer hits the refusal — and no such bundle has been run against a game. |
| Unity (IL2CPP) | 1 | 2 | The C# assembly compiles and stages, and matches the loader that ships. All twenty-nine of its native imports bind `taarib_jisr`; `scripts/isdar.sh` now builds that library per target and the staged tree carries it inside every BepInEx component. Nothing has been observed running. |
| Unity (Mono) | 1 | 2 | The assembly compiles, but references BepInEx `6.0.0-be.780` while the lockfile stages `5.4.23.5`. The chainloader cannot bind it, so its entry point never runs. |
| Electron / web | 1 | 2 | Wired end to end. The adapter refuses by name when the store lacks the renderer runtime rather than writing a translation table into somebody's `app.asar` with no runtime to read it. The runtime is now built by `ibni.mjs` and staged; the adapter's own canvas rung still looks for a `globalThis.taaribNawat` that nothing sets, so even a perfect install leaves canvas text to the game. |
| RPG Maker VX Ace | 1 | 1 | The install routine has no caller. |
| Unreal 4 / 5 | 1 | 1 | The container writer has no caller outside its own tests, the install step writes nothing for Unreal, and the engine's shaping switch exists only behind a build feature no manifest enables. |
| Godot 4 | 1 | 1 | No extension descriptor is written, so the module is never loaded. Four functions behind it have no callers. |
| Godot 3 | 2 — Taarib draws the glyphs itself | 1 | Both install functions have zero callers, so delivery and takeover always take the refusal branch. |
| Capcom BIO4 | 1 | 1 | Resident Evil 4's engine is recognised and its dictionaries, font containers and code-point table are all readable and rebuildable. Nothing routes an install to any of it: the script-engine dispatcher has no arm for the family. |
| Frostbite, BlackSpace, Alchemy, Dantelion, RAGE, Snowdrop | 3 | 1 | Named from their own files and then refused: no reader for their containers and no adapter exist, so the report says which engine it is and offers only the overlay. |
| Overlay — Direct3D 8 through 12, OpenGL, Vulkan | 3 — reading aid over the game | 1 | The draw batch and the glyph atlas are both closed now. Nothing captures text from the game, so no batch is ever produced. |

**How this was established, exactly.** Every verdict in that table comes from
reading the code and following callers, and the application computes it from the
same place rather than from a list written by hand. **Games on two of these
engines — Unity and Unreal — are installed on the machine this was written on
and were walked read-only; no game on any other engine is, and no engine's
in-game half has been observed putting a glyph on a screen.** The round trips
that exist prove writes, against containers built from the format
specifications, and are labelled as such in the tests that drive them.

The one result involving a real engine is Godot 3: the official 3.6.stable
headless binary loaded a patched `.translation` and answered `tr("Hello")` with
`مرحبا`, `tr("New Game")` with `لعبة جديدة`. That was driven from a test rather
than from an installer, which is why the table still reads 1 for Godot 3 — the
format work is right and nothing calls it.

[`docs/tashghil.md`](docs/tashghil.md) names the exact function where each chain
ends. The application states this per game before you install anything.

The read-only halves of several of these *are* exercised. The Unreal path was
walked over two real games: Little Nightmares (UE4, pak format 3, 9.6 GB) gave
up 21,137 distinct strings in 3.4 seconds with every one of its eighteen
`.locres` files round-tripping byte-identically, and Little Nightmares Enhanced
Edition (pak format 11, Zlib) gave up 556. Neither run went near the game's
process.

---

## How it works

Taarib is one text engine with a lot of ways into a game.

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

**`saff`** is the heart: a pure Rust library that turns logical-order text into
ordered lines of positioned glyphs. It knows nothing about games, engines,
launchers, or files, and it is buildable and useful entirely on its own. The
pipeline runs in a fixed order — extract markup and placeholders into style
spans, resolve direction with the full Unicode Bidirectional Algorithm, find line
break opportunities on the *logical* text, shape each run with HarfRust, measure
candidate lines from **real shaped advances**, reorder into visual order, justify
with Arabic kashida elongation identified from real joining behaviour, and
rasterize through skrifa.

Shaping is [HarfRust](https://github.com/harfbuzz/harfrust). Rasterization is
[skrifa](https://github.com/googlefonts/fontations). That pairing is deliberate:
both parse fonts through the same `read-fonts` stack, so a glyph identifier
produced by the shaper cannot possibly denote a different glyph in the
rasterizer. `deny.toml` enforces it — two copies of `read-fonts` in the
dependency graph is a build failure, and a native HarfBuzz or rustybuzz entering
the graph is a build failure.

**Taarib never touches the game's fonts.** Not the font assets, not the engine's
font asset system, not asset bundles. Those are serialized against the exact
engine version that produced them, and rewriting them is why existing
localization tools break on every engine version bump. Taarib rasterizes its own
glyphs, packs its own atlases, builds its own meshes, and asks the engine for
nothing but a texture, a mesh, and a material — primitives that exist in every
version of every engine. The rendering path works on a Unity version that does
not exist yet.

Everything a shipped patch needs is precomputed: layouts for every string at
every size the game draws at, and an atlas containing exactly the glyphs real
shaping produced from the real strings — never a guessed Unicode range, which
would both miss every ligature and include thousands of glyphs nothing draws.

The full architecture, crate by crate, is [`docs/mimar.md`](docs/mimar.md).

---

## Coverage

**Operating systems.** Windows 10 1809+ and Linux, including the Steam Deck, on
x86-64, ARM64, and 32-bit x86 for game processes that need it. **macOS is not
supported** — the bundle targets are configured and the workspace does not build
for Apple; see the platform table below for what that actually costs. Games
running under Proton or Wine are understood as such: Taarib resolves the prefix,
maps `Z:\` and the drive letters to real paths, and installs the Windows-side
framework into the Windows-side game directory. The launch option a Proton game
needs (`WINEDLLOVERRIDES`) is written into Steam's own per-account
configuration rather than through a wrapper script — the writer, the record of
the previous value and the restore on uninstall live in
`crates/taarib-tathbeet/src/itlaq.rs`, and as this is written the install and
uninstall paths call both halves. That wiring landed days ago, is still being
worked on, and has not been exercised against a real Steam account, so the Steam
Deck notes below still tell you how to set the option by hand.

**Launchers.** Steam, Epic Games Store, GOG Galaxy, EA app, Ubisoft Connect,
Battle.net, Xbox and Microsoft Store, itch.io, Amazon Games, Rockstar, Riot,
Heroic, Legendary, Lutris, Bottles and Playnite — each read from its own real
catalogue format — plus manual addition of any game by executable path.

**Engines.** The design tiers are below. What each one has actually been
observed doing is in [What works today](#what-works-today), and the two tables
should be read together.

| Engine | Tier | How |
| --- | --- | --- |
| Unity (Mono) | 1 | BepInEx 6 plugin; mesh generation intercepted for TextMeshPro, UI Text, NGUI, FairyGUI, world-space text |
| Unity (IL2CPP) | 1 | Same takeover over Il2CppInterop, with a three-rung resolver ending in binary signature scanning |
| Unreal 4 / 5 | 1 | Full text shaping forced on, Arabic font registered into Slate, measurement and layout corrected, `.locres` reinjected via an additive pak |
| Godot 4 | 1 | Font registered, direction set, translated through Godot's own translation resources |
| Godot 3 | 2 | Full takeover — Taarib lays out and draws the glyphs itself |
| RPG Maker MV / MZ | 1 | Data patched, runtime plugin shapes through a WebAssembly build of the core |
| RPG Maker VX Ace | 1 | Archive patched, Ruby script bound to the C ABI |
| Ren'Py | 1 | Translation scripts in the engine's own format plus a Python runtime hook |
| GameMaker Studio | 1 | `data.win` patched, glyph pages generated by Taarib |
| Electron / web | 1 | `app.asar` repacked with a preload runtime; canvas text routed through the WebAssembly core |
| Capcom BIO4 (Resident Evil 4, 2005) | 1 | Dictionary files rewritten and the baked glyph pages regenerated, with the text shaped before it is written because the engine has no shaper of its own. The readers and the font builder exist; nothing routes an install to them yet |
| Frostbite, Pearl Abyss BlackSpace, Vicarious Visions Alchemy, FromSoftware Dantelion, Rockstar RAGE, Ubisoft Snowdrop | 3 | Recognised by name from the game's own files — a version resource, an archive magic, an assertion string — and then refused: no reader for their containers and no adapter exist, so the report names the engine and offers only the overlay. Naming them is what lets it say "not reachable yet" instead of "unknown" |
| Anything unrecognised | 3 | Overlay: presentation hooked on Direct3D 8, 9, 10, 11 or 12, OpenGL (fixed-function or modern) or Vulkan, screen read, Arabic composited over it |

Recognised is not the same as supported, and the line between the two is one
function: `AilatMuharrik::qabil_lil_tarqee` in
`crates/taarib-mustalahat/src/muharrik.rs`. Ten families answer yes, the six
in-house engines answer no, and so does the unrecognised family. When one of the
six gains a reader and an adapter it moves across that line, and the table above
moves with it.

The three tiers are named honestly everywhere they appear in the interface —
**تعريب كامل** (text replaced inside the game), **تعريب بالرسم المباشر** (Taarib
draws the text itself), **طبقة ترجمة** (the game is not modified; this is a
reading aid, and it says so before you enable it).

---

## Safety

Taarib asks you to let it modify files inside games you paid for. Everything
below is non-negotiable and has no override switch anywhere in the product:

- **Anti-cheat is a refusal, not a warning.** Easy Anti-Cheat (standalone and
  the Epic Online Services variant), BattlEye, EA Javelin, Denuvo Anti-Cheat,
  Riot Vanguard, nProtect GameGuard, XIGNCODE3, Tencent's Anti-Cheat Expert,
  NetEase's NEAC Protect, Nexon Game Security, miHoYo Protect, PunkBuster,
  FACEIT, ESEA, Activision Ricochet and a VAC association are all detected by
  evidence — a file, a loaded module, a running service, a kernel driver, or
  Steam's own catalogue — and installation is refused outright with the name of
  what was found. Steam's catalogue can also declare an anti-cheat without
  naming it, and that is refused too. The closed set is the `NawHimaya` enum in
  `crates/taarib-aman/src/kashf_himaya.rs`; this sentence is a copy of it, and
  the file wins when they differ.
- **Multiplayer games warn explicitly**, per game, every time.
- **Every patch is signed**, and the client refuses anything unsigned, mismatched
  or revoked. Signature verification cannot be disabled by configuration, by
  build flag, or by developer mode.
- **Everything downloaded is unpacked in quarantine** with path traversal,
  device names, symlink escapes, case-collision attacks and decompression bombs
  all rejected before a byte is written.
- **Every modified file is backed up byte-exact before it is touched**, with a
  manifest written before the change it describes. Uninstall restores the game to
  its exact original state and verifies that it did rather than assuming it.
- **No original game asset is ever redistributed.** The patch compiler proves it
  by construction rather than by scanning — see
  [Legal position](#legal-position).

---

## The registry

The community catalogue is a **public Git repository**, not a server. Free to
operate, permanent, publicly auditable, forkable by anyone with `git clone`, with
no database to breach and no account system to leak. The official root is
`https://github.com/cc1a2b/taarib-registry`, with a jsDelivr mirror.

- A tiny global manifest carries a hash per shard, so a launch with a warm cache
  costs **one small request** when nothing has changed — and works fully offline
  when it has not.
- The index is sharded by application identifier. No client ever downloads the
  whole catalogue; it requests only the shards covering games it owns.
- Patch binaries are release assets, never committed into history.
- Matching runs in three tiers: exact build identifier, then **content
  fingerprint** — which is what keeps a patch alive through a store update that
  changed shaders and not text — then a declared compatibility range with an
  explicit warning.
- Several patches for one game are all listed, ranked, with their differences
  shown rather than hidden behind a single recommendation.
- Offline is a first-class path: import and install from a local file, a bundled
  mirror, or a network share, with the same signature and revocation checks.

**Publication is reviewed by one person.** A contributor submits from inside
Taarib and it goes nowhere public — it lands in the project owner's review queue,
where it is read string by string, previewed through the real text engine at real
size inside real interface bounds, optionally installed into a sandboxed copy of
the game, and then returned for changes, rejected with a written reason, or
approved and signed. There is no automatic publishing path, no matter how clean
the automated checks come back, and no self-approval bypass: the owner's own
translations go through the same submission object so provenance and signing stay
uniform across the whole catalogue.

That authority is possession of the signing key, not a row in a permissions
table. Every review entry point takes a token that can only be minted by proving
possession of the owner's key, so the review console is not merely hidden from a
contributor's session — the argument it requires cannot be produced there.

---

## Installation

`bundle.targets` in `apps/studio/src-tauri/tauri.conf.json` is
`["deb", "appimage", "nsis", "app", "dmg"]`, and that list is the complete set of
formats this project ships. `msi` and `rpm` were removed deliberately:
[`docs/tawzee.md`](docs/tawzee.md) §2 requires every shipped format to have an
owner and a tested update path, and those two would have neither.

| Platform | Artifact | State |
| --- | --- | --- |
| Windows 10 1809+ | `Taarib_<version>_x64-setup.exe` — NSIS, per-user, no administrator rights | **Built, installed, run and uninstalled on a real Windows 11 machine on 2026-09-04** — a 10.6 MB installer whose component store was still empty, installed to `%LOCALAPPDATA%`, no admin rights, no registry trace left behind. Every DLL it imports is in-box; nothing from a developer install. It found Steam on `D:` and a library on `F:` from the registry alone. The record is [`docs/tawzee/tahaqquq.md`](docs/tawzee/tahaqquq.md); the specification is [`docs/tawzee/windows.md`](docs/tawzee/windows.md). An installer built the next day with the component store staged is about 66 MB and has not been through that audit. |
| Linux x86-64 | `Taarib_<version>_amd64.AppImage` and `taarib-studio_<version>_amd64.deb` | **Built in an Ubuntu 22.04 container and started under xvfb on stock 22.04 and 24.04.** The glibc floor is **2.35**; an earlier build demanded 2.42, which no released SteamOS has ever carried. Specified in [`docs/tawzee/linux.md`](docs/tawzee/linux.md). |
| Steam Deck | the same AppImage as Linux | The 2.35 floor clears every SteamOS release — 3.5 ships 2.37, 3.8.1x ships 2.41. Proton prefixes are handled throughout, per library, so an SD-card game finds its prefix on the SD card. Audited against SteamOS in [`docs/tawzee/steamdeck.md`](docs/tawzee/steamdeck.md). No Deck has physically run it. |
| macOS 12+ | none | **Not supported, and not claimed.** The bundle targets are configured, but the workspace does not build for Apple: when this was measured, 7 of the workspace's then 29 members compiled for `aarch64-apple-darwin` and 22 did not (the workspace has 31 members now and has not been re-measured). After a genuine portability fix in the foundation crate, none of the 22 failures were in Taarib's own code — they were third-party C build scripts failing on `cc: unrecognized command-line option '-arch'`, which proves the toolchain wall rather than portability. The real cost is unknown and only a Mac can measure it. See [`docs/tawzee/macos.md`](docs/tawzee/macos.md). |

Nothing is fetched on first run. What a bundle holds depends on how it was
staged, and the two bundles that exist so far differ exactly there. The
installer audited in `docs/tawzee/tahaqquq.md` shipped an **empty component
store**: fonts and the signature database in; native libraries, BepInEx payloads
and adapters out. It started, scanned the library, translated and previewed
correctly, and refused every framework component by name — the staging tool
enumerates what is missing rather than shipping a fraction of it. Since then
`scripts/isdar.sh` builds every component in dependency order and
`taarib-tajmee` stages it, and a Windows-target tree staged that way on
2026-09-05 holds the BepInEx components with the Unity assemblies and the C-ABI
core, the loader and the three native payloads for both Windows architectures,
the RPG Maker and Electron runtimes, and the Ren'Py package with its face. No
bundle built from a staged tree has been installed and exercised against a game
yet.

### Platform notes that will actually come up

**Windows.** The installer is per-user (`RequestExecutionLevel user`,
`installMode: currentUser`) and needs no administrator rights. WebView2 is
installed through Microsoft's evergreen bootstrapper if it is absent. The
uninstaller offers the library-wide restore — every game with Taarib content
installed is offered a full removal — *before* it removes the application.

**Linux.** The AppImage carries WebKitGTK; the host must supply FUSE, a Secret
Service for the keyring, and a working graphics stack. On a host without
`libfuse2`:

```bash
./Taarib_1.0.0_amd64.AppImage --appimage-extract-and-run
```

The `.deb` depends on `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `shared-mime-info`,
`desktop-file-utils` and `hicolor-icon-theme`, and recommends
`gnome-keyring | libsecret-1-0`.

**Steam Deck.** Use the Linux AppImage, and run it with extract-and-run — that is
the documented path, not a workaround:

```bash
./Taarib_<version>_amd64.AppImage --appimage-extract-and-run
```

Stock SteamOS has no `org.freedesktop.secrets` provider, which the Steam Deck
document covers honestly rather than assuming a keyring exists. For a Proton
game, the launch option Taarib needs is set in Steam, in the game's Properties,
under Launch Options:

```
WINEDLLOVERRIDES="winhttp=n,b" %command%
```

**macOS.** Because there is no signing identity yet, Gatekeeper will refuse the
first launch. The honest workaround, documented rather than hidden:

```bash
xattr -dr com.apple.quarantine /Applications/Taarib.app
```

Two entitlements are granted — network client and keychain access — and the
document lists what is deliberately *not* granted, including
`com.apple.security.cs.allow-dyld-environment-variables`, with the reasoning.

**Portable mode**, on any platform: a file named `taarib.mahmul` beside the
executable puts the data root at `<exe dir>/bayanat` and forbids every
machine-global write — no keychain, no registry, no `~/.config`.

---

## Quick start

**As a player.** Open Taarib. It scans your machine and shows every installed
game as an artwork grid, badged with its Arabization status. Open one that shows
رقعة متوفرة, read the capability report, press تثبيت, and watch the staged
progress: compatibility, anti-cheat, download, signature, backup, framework,
files, done. Launch the game from the same screen.

**As a translator.** Open a game with no patch. Taarib probes its engine, reports
what it can reach, and extracts its strings — statically from the engine's own
containers, or by recording every string the game actually draws while you play
it. Translate in the workspace with glossary and translation-memory suggestions
and machine translation you accept per string or in batches. Watch the live
preview: every line rendered through the real engine, at real size, inside real
bounds, with measured widths and an overflow list. Compile, work through the
pre-flight checklist until it goes green, and submit.

Read [What works today](#what-works-today) before expecting the last step of the
player flow to end in Arabic on screen. The translator flow above is the designed
flow; of it, the read-only half — engine probe, static extraction, placeholder
protection, preview and compile — is what has been exercised, and runtime
capture has not.

---

## Repository layout

```
taarib/
  ROADMAP.md          the build contract: 24 phases, every constraint, every decision
  docs/               architecture, the ABI, the packaging contract, per-platform notes
  crates/             30 Rust crates (the list that counts is Cargo.toml's [workspace] members)
    taarib-usus/          foundations: errors, diagnostics, config, paths, platform
    taarib-mustalahat/    the shared vocabulary, source of TypeScript and JSON Schema
    taarib-saff/          the Arabic text engine
    taarib-lawha/         the glyph atlas
    taarib-jisr/          the stable C ABI
    taarib-wasm/          the same engine, compiled for JavaScript
    taarib-kashf/         game discovery       taarib-muharrik/  engine probe
    taarib-aql/           one game held whole: every crate's answer about a game, composed once
    taarib-istikhraj/     text extraction      taarib-tarjama/   translation pipeline
    taarib-ruqaa/         patch format         taarib-tarqee/    patch compiler
    taarib-tathbeet/      install and rollback taarib-aman/      safety
    taarib-mustawda/      registry client      taarib-taqdeem/   submission and review
    taarib-haqn/          injection and hooks  taarib-mudkhal/   the in-game loader
    taarib-muhawwil-*/    Unreal, Godot, Capcom BIO4 and script-engine adapters
    taarib-tabaqa/        the universal overlay
    taarib-tilqai/        the one-button pipeline: probe, extract, translate, build, install
    taarib-khatm/         signing              taarib-makhzan/   the local store
    taarib-warsha/        collaborative workspace
    taarib-tahdith/       the application's own updates
    taarib-tajmee/        the build-machine staging tool
  apps/studio/        Taarib Studio: Tauri backend and React frontend
  unity/              the C# plugins for Unity Mono and IL2CPP
  adapters-script/    the in-game JavaScript, Python and Ruby sides
  assets/fonts/       the bundled fonts and their validation manifest
  assets/aqfal/       version locks for every fetched build input
  schemas/            JSON Schema generated from the Rust vocabulary
  scripts/            font staging and icon rasterization
  vendor/retour/      the detour library, vendored and patched in
```

Module names mirror the domain rather than abstracting it: the module that
resolves text direction is `ittijah`, the one that packs glyphs is `lawha`, the
one that finds games is `kashf`. There is no `utils`, no `helpers`, no `common`,
no `manager`, and no `service` anywhere in the tree. The full lexicon is in
[`ROADMAP.md`](ROADMAP.md) section 4.1.

---

## Building from source

The full procedure — including the Windows bundle, which has real gotchas — is
[`docs/bina.md`](docs/bina.md). The short version:

```bash
git clone https://github.com/cc1a2b/taarib
cd taarib

# The whole Rust workspace. rust-toolchain.toml pins 1.95.0 and installs it.
cargo build --workspace --release

# The desktop application, with the frontend.
cd apps/studio && pnpm install && npm run tauri build

# The Unity game-side plugins.
dotnet build unity/Taarib.Unity.sln -c Release

# Regenerate the JSON Schemas from the Rust vocabulary.
cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat

# A release tree: every game-side artifact built in order, then staged.
scripts/isdar.sh --hadaf x86_64-pc-windows-msvc --jalb
```

The lockfile is `pnpm-lock.yaml`, but the Tauri build hooks say `npm`. Install
with pnpm, build with the npm scripts — `docs/bina.md` §1 explains why that
inconsistency exists and what happens if you resolve it the other way.

Two things a green build on Linux does not prove. Three crates —
`taarib-mudkhal`, `taarib-tabaqa` and `taarib-haqn` — keep their real content
behind `#[cfg(windows)]` and compile on Linux with none of it; check them on a
Windows toolchain. And if your `~/.cargo/config.toml` redirects `target-dir`,
every `target/` path in the documentation means that directory instead, and
`taarib-tajmee` has to be told with `--ahdaf`.

### The one flag you must not forget

Three crates compile to a shared library a game loads into its own process:
`taarib-tabaqa`, `taarib-muhawwil-unreal` and `taarib-muhawwil-godot`. Each
exports the bootstrap entry point `taarib_bidaya`, and **that export is behind
the cargo feature `hamula`, which is off by default.**

```bash
cargo build --release --target x86_64-pc-windows-msvc \
    -p taarib-jisr -p taarib-mudkhal -p taarib-tabaqa \
    -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \
    --features taarib-tabaqa/hamula,taarib-muhawwil-unreal/hamula,taarib-muhawwil-godot/hamula
```

The feature is spelled package-qualified because only the three payload crates
declare it; `taarib-jisr` and `taarib-mudkhal` have no `hamula`, and a bare
`--features hamula` on a command that also selects them is rejected by cargo.
It is off by default because all three payloads export the same symbol and
`taarib-studio` links all three as `rlib`s — an always-on export is a
duplicate-symbol link failure on the desktop binary.

Forgetting it does not fail. It produces a perfectly well-formed shared library
that compiles, links, installs, reports success, and is loaded by the game — at
which point the loader looks for `taarib_bidaya`, does not find it, and moves on.
The player launches the game and it is in English, with no error anywhere.

That is why the staging tool refuses such a build rather than trusting the
operator. `taarib-tajmee` byte-scans each payload image for the symbol name and
stops with:

```
[F1] .../taarib_tabaqa.dll exports no taarib_bidaya; rebuild with --features hamula
```

The manifest is not written and the exit status is non-zero, so the failure
lands on the build machine instead of on a user whose install reported success.

---

## Command reference

Taarib is a desktop application, not a CLI. `taarib-studio` takes two things on
its command line and nothing else: `--istiada`, which runs the library-wide
restore before the application is removed (the uninstaller invokes it), and the
path of a `.ruqaa` file, which the operating system passes when the registered
file type is opened. The rest of the command-line surface is the developer and
build-machine tooling.

```
# Build every game-side artifact in dependency order, then stage the tree below.
scripts/isdar.sh --hadaf <target-triple> [--jalb]

# Stage a release resource tree. Builds nothing; refuses on any missing artifact.
taarib-tajmee --hadaf <target-triple> [--jidhr <workspace>] [--ahdaf <target dir>]
              [--kharij <out dir>] [--jalb]

    --hadaf    x86_64-pc-windows-msvc | x86_64-unknown-linux-gnu
               | aarch64-apple-darwin | x86_64-apple-darwin   (required)
    --jidhr    workspace root                    (default: .)
    --ahdaf    cargo target directory            (default: <jidhr>/target)
    --kharij   staging root                      (default: apps/studio/src-tauri/mawarid)
    --jalb     permit fetching locked artifacts not yet cached
    -h, --help

# Regenerate schemas/*.json from the Rust vocabulary. Takes no arguments.
cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat

# Import the owner signing seed on this machine. Reads 64 hex characters on stdin,
# stores the key in the keychain, prints the public half.
cargo run -p taarib-khatm --bin malik

# Mint the release signing key. The private half goes only to the OS keychain,
# behind its passphrase; the public anchor is written to <file>, never overwritten.
cargo run -p taarib-khatm --bin isdar -- wallid --mirsa <file> [--ism <account>]
cargo run -p taarib-khatm --bin isdar -- mirsa [--ism <account>]      # print the anchor
cargo run -p taarib-khatm --bin isdar -- tahaqquq --mirsa <file>     # key still derives it?

# Stage the webview fonts against assets/aqfal/qufl_khutut.json.
python3 scripts/ijlib_khutut.py [--tahaqquq] [--sakit]

    --tahaqquq  verify what is on disk, touch no network, fail if anything is off
    --sakit     report only failures

# Rasterize every shipped icon from assets/huwiya/. Takes no arguments.
python3 scripts/irsim_ayqunat.py

# Run the text engine on its own: shapes Arabic, prints glyphs, writes a PNG.
cargo run -p taarib-saff --example nazra
```

---

## Legal position

Stated once, plainly, because a studio's legal team should not have to hunt for
it or read between lines.

**Taarib distributes patches. It never distributes game content.** A published
patch contains translated text written by a contributor, layout data computed
from that text, glyph images rasterized from fonts Taarib bundles under their own
open licences, and metadata. Nothing else.

That is enforced by construction, not by a scan. `crates/taarib-tarqee/src/bawwaba.rs`
is the gate, and its own header explains why it is not a check: a function that
inspects a finished package can only answer "these bytes do not look like a game
asset", because by the time it runs the provenance is gone. So provenance is
carried instead. A token meaning *Taarib generated these bytes* has no public
constructor; the only thing that mints one is the rasterizer, which will only
accept a font from Taarib's own font directory. To put bytes into a package you
must hold that token, and there is no path from a game directory to one. Adding
such a path would mean adding a constructor in that file — a visible change in
review.

Where an engine keeps text inside a container that also holds art, audio and
code, a package never carries the rewritten container. It carries the
*instructions to produce one* — offsets and replacement text — and the installer
applies them to the user's own copy. The original never travels. The compiler
reports this per build; the R.E.P.O. run quoted above ends
`0 container difference(s), and 0 bytes of original game content`.

**Taarib requires the user to own the game.** It operates on games already
installed on the user's machine, discovered through the launcher that installed
them. It downloads no game, contains no game, and cannot install a patch for a
game that is not present.

**Taarib refuses games carrying anti-cheat.** Every anti-cheat named in
[Safety](#safety) above — the closed set in
`crates/taarib-aman/src/kashf_himaya.rs` — is detected by evidence and
installation is refused outright, naming what was found. This is a refusal, not
a warning, and there is no override switch — not in configuration, not behind a
build flag, not in developer mode. Multiplayer games without anti-cheat still
warn explicitly, per game, every time.

**There is a takedown path and it is honoured.** If you hold rights in a game and
want a patch removed from the registry, open an issue at
[github.com/cc1a2b/taarib](https://github.com/cc1a2b/taarib/issues) identifying
the patch, or write to the maintainer through the address on the
[cc1a2b](https://github.com/cc1a2b) GitHub profile. You do not need to prove
anything beyond a plausible claim of right to get a first response.

Revocation is a first-class operation in the product rather than a promise about
process. A revoked patch is recorded with a machine-readable reason —
`IntihakRukhsa`, "it carries content its licence does not permit", is one of the
four — and a reviewer's written statement, which is required and cannot be
blank. The revocation enters a signed list that every client checks. Anyone who
already installed the patch is told on next launch, in Arabic and English, why it
was pulled, and is offered a clean uninstall that restores the game to its
original bytes.

The whole catalogue is a public Git repository, so a removal is a commit anyone
can audit — including the person who asked for it.

**Licensing.** Taarib itself is MPL-2.0. Each bundled font keeps its own SIL Open
Font License, redistributed alongside it; `assets/NOTICES.md` carries the full
third-party notices that ship in every bundle. Community translations are the
work of their contributors.

If something here is not enough — a specific game, a specific patch, a broader
concern — the fastest route is an issue. It will be read.

---

## Contributing

Two kinds of contribution, with different routes. Both are covered in full in
[`CONTRIBUTING.md`](CONTRIBUTING.md).

**Translations go through the application, not through pull requests.** Taarib
validates a submission locally against a checklist and the submit action stays
inactive until it passes: the game does not already ship official Arabic, the
compiler's hard checks, the asset gate's certificate, coverage against the
publishable floor (60% of strings overall, 85% of what a player meets in the
first hour), no duplicate of your own published patch for the same build, and no
undecided import mappings. Four further conditions warn rather than block —
twenty or more strings overrunning their space, a quarter or more of the game
still untranslated, machine translation declared with no human review, and
terminology that disagrees with the glossary — and each must be acknowledged
before the gate will mint a submission.

**Code contributions are pull requests.** Before opening one:

- Read the relevant phase in `ROADMAP.md`. It is a specification, not a summary,
  and a change that contradicts one of the eight architectural decisions in
  section 2 will be declined regardless of how well it is written.
- Match the naming law in section 4.1. Domain names, not abstractions.
- `cargo check --workspace --all-targets`, `cargo clippy --workspace
  --all-targets`, `cargo test --workspace` and `cargo deny check` must all pass.
  `cargo fmt --check` is deliberately not a gate — the tree is several thousand
  hunks from what rustfmt wants and the reformat is pending, so do not run
  `cargo fmt` over files your change does not touch. The lint configuration is
  strict on purpose: `unwrap`, `panic`, `todo`, `unimplemented`, indexing, and
  undocumented `unsafe` are denied workspace-wide, and every `#[expect]` needs a
  reason.
- Commits follow the conventional format.

Everyone who takes part is expected to behave according to
[`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Security issues have their own route:
[`SECURITY.md`](SECURITY.md).

---

## License

Mozilla Public License 2.0. See [LICENSE](LICENSE).

MPL rather than a stronger copyleft for a concrete reason: parts of this codebase
are loaded into other people's game processes, and file-level copyleft is the
licence that stays coherent when that happens. Every bundled font keeps its own
SIL Open Font License, included beside it.

```
Copyright (c) 2025-2026 Hussain Alsharman
Licensed under the Mozilla Public License 2.0 — file-level copyleft,
chosen because this code runs inside processes it does not own.
```

---

## Support

If Taarib is useful to you: star the repository, follow the project, and tell
someone who plays games in a language they do not read.

<div align="center">

Built by [cc1a2b](https://github.com/cc1a2b)

</div>
