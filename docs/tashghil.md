# التشغيل — what runs inside a game, engine by engine

This file is the evidence behind one field. `TaqreerImkaniyat::jahiziya` tells a
user whether the tier their game qualifies for is a tier this build can actually
deliver, and `taarib-muharrik`'s `imkaniyat::jahiziya`
(`crates/taarib-muharrik/src/imkaniyat.rs`) is the table that answers it. A
table of claims with nothing behind it is worse than no table at all, so the
reading that produced each row is written down here, with the file and the
function where each chain stops.

`imkaniyat.rs` is the authority and this file is its working. The tests at the
foot of that file pin the verdicts, one per engine; if this document and that
function ever disagree, the function is right and this document is stale.

**Functions are named here, and line numbers are not.** An earlier version of
this file cited `path:line` for every stop, and within a week most of the
numbers pointed at other code — the two install crates grew by hundreds of
lines, and a tree-wide `cargo fmt` is still pending, which will move every
line in every file. A function name survives both and is what you grep for. The
standard of evidence is unchanged: every "has no caller" below is a grep over
`crates/`, `apps/`, `unity/` and `adapters-script/`, re-run on 2026-09-06.

## What the evidence is, exactly

Every verdict below rests on **reading code**. Games on two of these engines are
installed on the machine this was written on and were walked read-only — R.E.P.O.
and Hollow Knight on Unity, Little Nightmares and its Enhanced Edition on Unreal
— and no engine's in-game half has been observed putting a glyph on a screen.
Where this document says a function has no caller, that is a grep, not an
impression. Where it says a write happens, that is a round trip against a fixture
the project authored — it proves the *write*, not a screen.

There is exactly one result from a real engine, and it is worth stating precisely
because it is the only one: the official Godot **3.6.stable** headless binary
loaded a `.translation` resource this project produced and `tr("Hello")` came back
`مرحبا`. That was driven from a test, not from an installer, and it proves the
delivery format is correct — nothing about whether a player ever reaches it.

## Three outcomes, not one

Until recently "nothing renders" was true everywhere and one sentence covered the
whole table. It no longer is, and the difference matters more to a user than
anything else on this page, because rows in this table can end with a game that is
**worse than it was**.

1. **Nothing reaches the game.** The install may place Taarib's own files beside
   it — a `.ruqaa`, a loader, a component directory — and the game's own data is
   untouched. It runs exactly as it did, in its original language.
2. **The install stops.** A component the deployment step demands is not in the
   store, the install refuses by name and rolls back. Nothing is left behind.
3. **The game changes and cannot draw the change.** Arabic lands in the game's own
   data, the install reports success, and what the player sees is worse than what
   they had: blank boxes, or letters standing apart in the wrong order.

GameMaker and Ren'Py below 7.4 are outcome 3 unconditionally. Ren'Py at 7.4 and
above is the one row the application now reports as **complete** — and the one
row where the difference between a Studio install and the one-button pipeline
decides what the player sees; see its section. Those rows are why
`sadr_jahiziya` says "what makes Arabic appear **readably**" rather than "your game
will not change": for most of this table the game really does not change, and
for those it does.

## Why this is not a limitation

`TaqreerImkaniyat::hudud` is the list of things about a *game* that Arabization
cannot reach: text painted into a picture is pixels, and no release will change
that. What this document describes is a property of *Taarib*, and a release will
change it. The two are separate fields for that reason, and a reader who confuses
them gives a user the wrong answer twice — they stop waiting for the update, and
they blame their game for the product's gap.

## The table

`jahiziya` is the verdict `imkaniyat::jahiziya` returns today. `out` is the
outcome above.

| engine | tier | jahiziya | out | what actually runs | where the chain stops |
| --- | --- | --- | --- | --- | --- |
| Unity / Mono | 1 | `Ghaiba` | 1 | the component is deployed and the chainloader will not load it | `MulhaqTaarib.Awake` — `unity/Taarib.Unity.Mono/Taarib.cs` |
| Unity / IL2CPP | 1 | `Ghaiba` | 1 | the assembly matches its loader; nothing has been observed running | `Jisr`'s `DllImport`s — `unity/Taarib.Unity.Jisr/Jisr.cs` |
| Unreal | 1 | `Ghaiba` | 1 | the loader, and one console variable written into `Engine.ini` | `KatibPak::uktub_fi_luba` — `crates/taarib-muhawwil-unreal/src/mawarid/pak.rs` |
| Godot 4 | 1 | `Ghaiba` | 1 | nothing; the engine is never told to load the extension | `tahyia_mustawa` — `crates/taarib-muhawwil-godot/src/imtidad.rs` |
| Godot 3 | 2 | `Ghaiba` | 1 | the loader, the GDNative binding, and the patch is mapped | `sallim` — `crates/taarib-muhawwil-godot/src/bidaya.rs` |
| RPG Maker MV / MZ | 1 | `Ghaiba` | 2 | `data/*.json` is spliced, then the deployment step refuses when the store lacks the plugin | `asmaa_mukawwin` — `crates/taarib-tathbeet/src/tarkib.rs` |
| RPG Maker VX Ace | 1 | `Ghaiba` | 1 | nothing | `vxace::rakkib` — `crates/taarib-muhawwil-nusus/src/vxace.rs` |
| Ren'Py ≥ 7.4 | 1 | **`Mukammala`** | — | the whole tier: translation, language selection, direction, and the face the Ren'Py component carries, named by the deployment plan | the chain does not stop by reading; what is missing is a screen. Through `taarib-tilqai` no face is registered — see the section |
| Ren'Py < 7.4 | 1 | `Ghaiba` | 3 | the translation installs; the engine blits one character at a time | `saf_mulhaq` — `crates/taarib-tajmee/src/masfufa.rs` |
| GameMaker | 1 | `Ghaiba` | 3 | the string pool is rewritten and no glyph pages are made | `rakkib_gamemaker` — `crates/taarib-muhawwil-nusus/src/tarkeeb.rs` |
| Electron | 1 | `Ghaiba` | 2 | the adapter refuses this package by name when the store lacks the runtime, and stops the install | `rakkib_ghilaf` — `crates/taarib-muhawwil-nusus/src/tarkeeb.rs` |
| Capcom BIO4 | 1 | `Ghaiba` | 1 | nothing; the readers and the font builder exist and no install is routed to them | `tarkeeb::rakkib_luba` has no arm for the family — `crates/taarib-muhawwil-nusus/src/tarkeeb.rs` |
| Frostbite, BlackSpace, Alchemy, Dantelion, RAGE, Snowdrop | 3 | `Ghaiba` | 1 | the engine is named from its own files and sent to the overlay; no reader for its containers, no adapter | `tabaqa_khassa` / `naqs_khassa` — `crates/taarib-muharrik/src/imkaniyat.rs` |
| overlay (tier 3, every unrecognised engine and the six above) | 3 | `Ghaiba` | 1 | the present hook, the game's own device, a complete render pipeline, and a draw batch nothing feeds | `Tabaqa::iltaqit` — `crates/taarib-tabaqa/src/wajiha.rs` |

## The caller that landed, and what it did not change

Four of these rows used to stop at "this function has no caller". They no longer
do. `taarib_muhawwil_nusus::tarkeeb::rakkib_luba` is the dispatcher; it is reached
from `taarib_tathbeet::nusus::raqqi_nusus`, which `tarkib::nashr_bi_khutta` calls
first — before the framework deployment step, deliberately, because RPG Maker's
extraction records carry byte offsets into a `plugins.js` that deployment appends
to. The write runs *inside* the deployment step, under the plan's own tier
decision (`IdhnNusus`), rather than unconditionally beside it as it once did: a
tier-3 game and a game the safety layer refused are never written to.

So all four writers now run:

| writer | called from |
| --- | --- |
| `rpgmaker::rakkib` (`rpgmaker.rs`) | `tarkeeb::rakkib_rpgmaker` |
| `renpy::iktub_idad` (`renpy.rs`) | `tarkeeb::rakkib_renpy` |
| `MuhawwilGameMaker::uktub` (`gamemaker.rs`) | `tarkeeb::rakkib_gamemaker` |
| `electron::rakkib` (`electron.rs`) | `tarkeeb::rakkib_ghilaf` |

VX Ace is the fifth script engine and is refused by name in `rakkib_luba`,
because its archive rewriter takes a Ruby payload the dispatcher does not carry.
That refusal is deliberate and is better than a route that half works.

**None of that moved a verdict except Ren'Py's.** A write is necessary and is not
sufficient: a string an engine has no letter pictures for is drawn blank, and a
string an engine without a shaper draws one character at a time is drawn as
unjoined letters. Both are writes that succeeded and neither is Arabic. What the
caller changed is *which rung* each row now fails at, and for two engines it
changed the outcome from 1 to 3 — which is worse for the user and is why those two
are called out above.

The rest of this file is each row in full.

## Unity, both backends

The C# is not the problem. `unity/` is about 47,000 lines with no
`NotImplementedException`, no `TODO`, and no stubbed method. The Mono adapter
registers four HarmonyX prefixes — `GenerateTextMesh` and `UpdateMaterial` on each
of the screen-space and world-space components
(`unity/Taarib.Unity.Mono/Anzimat/TextMeshPro.cs`) — installs them, and hands the
finished mesh back to the renderer. Only the two `GenerateTextMesh` prefixes are
load-bearing: if neither takes, the whole system is disposed and reports so. The
IL2CPP adapter resolves its targets through a three-rung ladder — Il2CppInterop
metadata, the `il2cpp_*` runtime API, then a byte-signature scan over the
executable — and falls back to hand-written x64 and ARM trampolines
(`unity/Taarib.Unity.Il2cpp/Khatf/Mihmaz.cs`) when HarmonyX cannot take a target.

It compiles now, and the repository builds it. `scripts/isdar.sh` runs
`dotnet build unity/Taarib.Unity.sln -c Release`, and `.github/workflows/isdar.yml`
runs the same on a runner — on `workflow_dispatch`, not on every push. The
outputs under `unity/**/bin/Release/**` are local build products, excluded by
`.gitignore`. `taarib-tajmee --help` names the build as the one command that
assembles the bundle in order.

The staging path that used to be wrong is fixed. `tajmiaat`
(`crates/taarib-tajmee/src/masfufa.rs`) resolves the target-framework directory
per project — `net6.0` for `Taarib.Unity.Il2cpp`, `netstandard2.1` for the other
three — instead of reading all four out of `netstandard2.1`, finding three, and
withholding the manifest from a build that had in fact succeeded.

The two backends now fail at different rungs, and one sentence for both would be
false for whichever it did not describe.

* **Mono asks for a loader generation the bundle does not carry.**
  `Taarib.Unity.Mono.csproj` references `BepInEx.Unity.Mono` **6.0.0-be.780**;
  `assets/aqfal/qufl_bepinex.json` pins **v5.4.23.5** for this backend. Under
  BepInEx 5 the base types live in assemblies the plugin does not name, so the
  chainloader cannot bind it and `Awake` (`Taarib.cs`) is never entered.
* **IL2CPP matches its loader, and its stated gap has been overtaken.**
  `Taarib.Unity.Il2cpp.csproj` references `BepInEx.Unity.IL2CPP` 6.0.0-be.780
  and the lock ships **v6.0.0-pre.2** for this backend — the same generation. The
  rung below it is the native library: all twenty-nine of `Taarib.Unity.Jisr`'s
  `DllImport`s bind the bare name `taarib_jisr` (`unity/Taarib.Unity.Jisr/Jisr.cs`,
  the only file in `Taarib.Unity.*` that declares one), which is matrix rows
  B1–B3 out of the `taarib-jisr` cdylib, and `saf_bepinex` stages it from
  `<target dir>/<triple>/release/`. `scripts/isdar.sh` now builds that library
  for every triple in `hamulat_alalaab`, and the staged tree on the authoring
  machine carries it inside every BepInEx component. So the reason
  `imkaniyat::naqs_unity`'s IL2CPP arm gives — "the native library it calls into
  is not in this package" — is a statement about a tree that has moved. The
  verdict is still `Ghaiba` because nothing has been observed running on this
  backend, not because the library is still missing.
* **World-space `TextMesh` would translate nothing even on a working install.**
  `NizamMujassam.Fahras` (`unity/Taarib.Unity.Mono/Anzimat/NassMujassam.cs`) is
  the scan that finds the labels, it is documented as being called on scene load,
  and there is no `SceneManager.sceneLoaded` handler anywhere in `unity/`.
  Registration succeeds, the log says the system is installed, and it guards zero
  labels.

## Unreal

This is the one adapter that does something to a real game. `taarib_bidaya`
(`crates/taarib-muhawwil-unreal/src/bidaya.rs`) is resolved and called by
`taarib-mudkhal`, resolves `IConsoleManager::Get` across the module names in
`WAHDAT_QAIDA` (`src/wasl.rs`) and `FSlateApplication::Get` (`src/slate.rs`), and
then writes `Slate.DefaultTextShapingMethod=2` and its two companions into the
game's `Engine.ini` under `[SystemSettings]`, atomically, through
`MalafIni::aktub` → `taarib_usus::masarat::kitaba_dharra_nass` (`src/tashghil.rs`).
From the next launch that game's Slate does full HarfBuzz shaping instead of
kerning-only. On a game that is still in English, nothing about that is visible.

Everything that would produce Arabic is declined by name:

* `bidaya.rs` calls `wasl::wasl(None)`. `FaharisAwamir` (`src/wasl.rs`) is an
  assertion that the caller verified this build's vtable slots, and a bootstrap
  inside a shipped game has not. So `muhayya` is false for the life of the
  process and `Tashghil::rutbat_haqn` (`src/tashghil.rs`) returns before touching
  a console variable. The re-assert watchdog declines on the same flag.
* `TasheehQiyas::rakkib` (`src/qiyas.rs`) installs the measurement and
  justification detours. It has no caller, and `AhdafQiyas` is constructed
  nowhere in the workspace. `sajjil_qiyas` (`bidaya.rs`) records the consequence
  in the log: *no detour and no vtable slot was installed, so this process is
  byte-identical to the one this payload entered.*
* `khatt.rs` (font registration) and `alam.rs` (culture activation) have no callers
  at all.
* The `.ruqaa` beside the module is opened only to validate its framing and content
  hash, and is dropped unread.
* `KatibPak::uktub_fi_luba` (`src/mawarid/pak.rs`), which would author the
  additive patch pak, is called only from `tests/hala_ahruf.rs`.
  `taarib-istikhraj/src/unreal.rs` uses the `mawarid` readers and writes nothing.
* The Slate shaping switch itself is behind `--features hamula`
  (`crates/taarib-muhawwil-unreal/Cargo.toml`), which no manifest in the workspace
  turns on by default. Only the release build passes it — `scripts/isdar.sh` and
  `.github/workflows/isdar.yml` — so a payload built by a plain
  `cargo build --release` exports no `taarib_bidaya` at all and writes no
  `Engine.ini`. That is the one Unreal effect this row credits, and it exists only
  in a bundle built the documented way.

## Godot 4

`mulhaqat_muharrik` (`crates/taarib-tathbeet/src/tarkib.rs`) deploys only for
Godot **3**, and nothing anywhere writes a `.gdextension` manifest, so the engine
never resolves `taarib_imtidad` and the library is never loaded.

If it were, it would bind all sixteen GDExtension interface functions by name
(`ASMA_DAWAL`, `crates/taarib-muhawwil-godot/src/imtidad.rs`) and register a
level-initialisation callback, and then stop at `tahyia_mustawa`, because
`KHADIM` is empty: `thabbit_khadim` and `thabbit_turuq` have no callers, so the
`extension_api.json` method hashes that binding needs never arrive. The refusal
names the missing call outright.

The offline route is dead in a different way from before. `BinaHawiya`
(`src/pck/hawiya.rs`) still has no caller, so the additive `taarib.pck` named by
`ISM_HAZMA` (`src/lib.rs`) is never produced. `Tarjama::ila_bayt`
(`src/pck/tarjama.rs`) **does** have callers now, in `tawseel.rs` — but
`TawseelThalith` is reached only through `thabbit_tawseel`, which is the Godot 3
seam below and has no caller either.

## Godot 3

The installer does its half. `godot_thalatha` (`tarkib.rs`) writes `taarib.gdnlib`
and adds the singleton registration to `override.cfg`, so Godot itself loads the
library and calls `taarib_gdnative_init`
(`crates/taarib-muhawwil-godot/src/bidaya.rs`). That binds the GDNative core API
and verifies its type and major version and maps every `*.ruqaa` beside the
module.

It stops in `sallim` (`bidaya.rs`), and the log line there says exactly why: *the
engine is bound and neither half of the Godot 3 path was installed: the patch's
companion called neither `thabbit_tawseel`, which delivers the translated text,
nor `thabbit_istila` with this build's `Font::draw`, `Font::draw_char` and
`Font::get_string_size` addresses, its `VisualServer` binding and the patch's
layout policy.*

Both seams are open. `thabbit_tawseel` and `thabbit_istila` have zero callers, so
`awsil` takes its refusal branch before a `.translation` resource is written, and
`sallim` takes the `Mumtania` branch before a detour is installed. `istila.rs` is
complete — three detours, glyph emission through
`VisualServer::canvas_item_add_texture_rect_region`, and a guard that refuses a
half-supplied configuration — and it refuses to guess an address, which is
correct: a detour installed on the wrong function corrupts a stranger's game.

`sallim` carries a third outcome that nothing can currently reach. When the
delivery went in and the takeover did not, it answers `HalatBidaya::Naqisa` and
says the game's text will be Arabic from the next launch and Godot 3 will draw it
unshaped. That branch is behind `awsil` returning true, which is behind
`thabbit_tawseel`, so today every Godot 3 game takes the `Mumtania` path and the
game is exactly what it was.

This row is where the one real-engine result sits: `TawseelThalith::hayyi`
(`crates/taarib-muhawwil-godot/src/tawseel.rs`) produced a resource that
Godot 3.6.stable headless loaded and answered `tr("Hello")` from. It was driven
from a test. It proves the delivery is correct; it does not put a caller in the
installer.

## RPG Maker MV and MZ

The data splice is real and it runs. `rakkib_rpgmaker` (`tarkeeb.rs`) rediscovers
the project, extracts, checks every translation for the escape codes the engine
substitutes at draw time (`rpgmaker::tahaqquq_hurub`), and calls
`rpgmaker::rakkib`.

**The install does not survive it when the store lacks the plugin.**
`mulhaqat_muharrik` routes this family to `rpg_maker` (`tarkib.rs`)
unconditionally, which asks `asmaa_mukawwin` for the contents of
`mulhaq/rpgmaker/{mv,mz}` and gets `MukawwinMafqud` when the store does not hold
it. Deployment runs *after* the splice, so the splice happens and is then rolled
back with the failed install. That is outcome 2: nothing is left in the game.

Whether the store holds it depends on a step outside cargo. `taarib-tajmee`
stages row I2 from `target/adapters/rpgmaker/taarib.js`. Until 2026-09-05 nothing
in the repository produced that file. `adapters-script/ibni.mjs` now does — `tsc`
for I2, because the plugin has to be ES5 and `esbuild` cannot lower `const`/`let`
to `var`; `esbuild` for I1 — and `scripts/isdar.sh` runs it as one step of the
release build. So `imkaniyat::naqs_rpg_maker` says the module "is not built into
this package", and that is now a statement about a tree that has moved. A bundle
staged by hand without `isdar.sh` still refuses, which is the failure this row
describes; a bundle built the documented way no longer reaches it, and the
verdict has not been re-established against that.

Two further defects sit behind that one and are not what stops it. The
registration is written with `"parameters":{}` (`tarkib.rs`) because
`IdadatMulhaq::barametr` (`rpgmaker.rs`) is only ever called from `rakkib_mulhaq`,
which has no caller — so a built plugin would read its own defaults, log that no
font is configured, and never enter its takeover rung. And the plugin is what
corrects direction, alignment and window mirroring, which Chromium's `fillText`
does not do for the Arabic it otherwise joins correctly.

## RPG Maker VX Ace

Nothing is installed, by design: the Ruby is meant to travel inside the patch
rather than as a component, `hajat_itar` answers `SababLaHaja::DakhilAlRuqaa`
(`tarkib.rs`) and `mulhaqat_muharrik` returns `Ok(())` on the shared arm. The only
thing that would insert `taarib_rgss3.rb` into `Scripts.rvdata2` is `vxace::rakkib`
(`crates/taarib-muhawwil-nusus/src/vxace.rs`), whose Ruby source is
caller-supplied and which has no caller. `tarkeeb::rakkib_luba` refuses this
family by name rather than routing it through an entry point that carries no
payload.

There is a second, independent break behind that one. The script's takeover rung
needs `Taarib/lawha.png` and `Taarib/lawha.tbl` — `vxace.rs` names both and writes
them into the settings file it generates — and nothing in the tree produces
either. Injected but unfed, the adapter raises in
`adapters-script/vxace/taarib_rgss3.rb`, its caller catches it, logs that the
glyph takeover declined, and falls back to its direction-only rung.

## Ren'Py at 7.4 and above — the one `Mukammala`

This is the only row in the table where the application reports the tier as
finished, and it is the one row a reader should be most careful with, because
"finished" here was established by reading three crates, not by launching a game.

Everything installs. `rakkib_renpy` (`tarkeeb.rs`) writes `game/tl/arabic/` —
`renpy::iktub_idad`, `iktub_mustalahat` and `iktub_hiwar` (`renpy.rs`) — and the
generated `.rpy` invokes `taarib_renpy.rakkib`
(`adapters-script/renpy/taarib_renpy/__init__.py`), which selects the language
through `config.language` and sets the direction and alignment via
`sajjil_ittijah`. Every write is additive: Ren'Py compiles anything under
`game/tl/<language>/` by itself with no registration step, so uninstalling is a
delete. The `taarib_renpy` package itself really is deployed, because staging row
I3 is a copy of a directory that is in this repository (`saf_mulhaq`,
`crates/taarib-tajmee/src/masfufa.rs`) and `tarkib::renpy` places it under `game/`.

**The font now arrives too, through a Studio install.** Three pieces, in three
crates, and the chain is worth spelling out because the previous version of this
row said the opposite:

1. `saf_mulhaq` stages `NotoNaskhArabic[wght].ttf` and its `OFL.txt` — the same
   bytes row J1 already fetched and hash-verified for the bundle's own font
   directory — into the Ren'Py component under `taarib/khutut/`
   (`KHATT_RENPY`, `DAKHIL_KHATT_RENPY` in `masfufa.rs`). The staged tree on the
   authoring machine holds both files.
2. `tarkib::khutta` builds the deployment plan and fills
   `KhuttatTarkib::khatt_renpy` from the component store's own listing, choosing
   through `ikhtar_khatt_renpy` over `TARTIB_KHATT_RENPY` — a ranking, so a store
   staged with some other family registers that one instead.
3. The name travels in the permit: `IdhnNusus::min_khutta` carries it into
   `nusus::raqqi_nusus` (`crates/taarib-tathbeet/src/nusus.rs`), which hands it to
   `Mawarid::khatt_renpy`, and `rakkib_renpy` registers that face by name in the
   generated `.rpy`. Because the plan chose the name from the very listing the
   deployment will copy into `game/`, the file the `.rpy` names and the file that
   lands cannot be two different files.

`imkaniyat::jahiziyat_renpy` therefore answers `Mukammala` for a shaping Ren'Py,
and `renpy_yashkul_fahuwa_mukammala` at the foot of `imkaniyat.rs` pins it. The
verdict deliberately does not read the component store — it is a fact about what
this *build* stages, not about what a given machine holds — which produces the
first of three caveats.

* **A bundle staged without the face still reports `Mukammala`.** If `mawarid/`
  was assembled without `--jalb` or before the row existed, `khatt_renpy` is
  `None`, `rakkib_renpy` records that no font was named, and the game is drawn in
  its own font: legible where that font covers Arabic, empty boxes where it does
  not — the situation the previous verdict, `Naqisa`, described. The report
  cannot tell those two machines apart and does not try.
* **The one-button pipeline registers no face.** `taarib-tilqai` deploys no
  component store and builds its permit with `IdhnNusus::min_qarar`
  (`crates/taarib-tilqai/src/tathbeet.rs`), which carries no font name by
  design: a name registered for a file no step will place is a game pointed at a
  font that is not there. Through that path the game's own font still decides
  legibility, and the same `Mukammala` verdict is what gated the run.
* **Nothing has been observed on a screen.** The chain above was established by
  reading `masfufa.rs`, `tarkib.rs`, `nusus.rs` and `tarkeeb.rs`. No Ren'Py game
  is installed on the authoring machine.

One inconsistency inside `imkaniyat.rs` itself is worth naming so nobody
re-derives the old verdict from it: the doc comments on `jahiziyat_renpy` and
`naqs_renpy_khatt` still describe the `Naqisa` state ("the font does not
arrive"), while the arm beneath them returns `Mukammala` and the tests pin it.
The code is right; the two comments are stale and should be rewritten with the
paragraph above.

## Ren'Py below 7.4

Below `dalail::nusus::HADD_TASHKEEL_RENPY` the engine has no HarfBuzz and no
FriBidi, and its text layout is a per-character blit. Everything in the section
above still installs — the translation, the language selection, the direction — so
the install completes and the game does change. It changes to separated letters in
the wrong order, which is not Arabic a person can read.

The adapter's own answer to that is the glyph takeover in
`taarib_renpy._rakkib_istila` (`__init__.py`), which loads `taarib_jisr` through
`jisr.hammil` (`adapters-script/renpy/taarib_renpy/jisr.py`) from
`<game>/taarib/jisr/<arch>/` — the default `dalil_jisr`, with the platform file
names `_ism_maktaba` produces. Nothing stages a library there: `saf_mulhaq`
(`crates/taarib-tajmee/src/masfufa.rs`) copies the Python package and the face,
and the B1–B3 rows go into each BepInEx component instead. So `hammil` raises
`GHAYR_MUHAYYAA` naming both candidate paths, the takeover declines at every
launch, and the configuration rung is all that is left.

`docs/tawzee.md` row B1 used to claim staging placed `taarib_jisr` under
`mulhaq/renpy/taarib/jisr/<mimariya>/`. It does not, and that row has been
corrected; the gap is real and belongs to staging, not to the adapter.

## GameMaker

GameMaker is the one engine with no runtime component at all, and that is correct
rather than a gap: the Arabic goes into `data.win` and the game reads it as its own.
`hajat_itar` says so (`crates/taarib-tathbeet/src/tarkib.rs`) and
`mulhaqat_muharrik` deploys nothing.

The rewriter is about 4,900 lines and complete — the FORM chunk table, the string
pool, texture page append, font glyph tables, `draw_set_halign` bytecode
correction, a round-trip proof and an atomic write — and **it now runs**.
`rakkib_gamemaker` (`tarkeeb.rs`) locates the container, replaces every pool entry
the patch has Arabic for through `MuhawwilGameMaker::badil_nass` (`gamemaker.rs`),
and calls `MuhawwilGameMaker::uktub`, which verifies the whole container in memory
before the guard is handed a byte.

That is why this row is outcome 3 and not outcome 1. A GameMaker font is a **baked
glyph table**: the container ships pictures of letters and an index from character
codes to pictures. There is no font file to swap and no shaping stage to configure,
and the only functions that would add pictures — `sajjil_ashkal`, `nass_manqul`,
`istabdil_khatt`, `alhiq_safha` — are not called by `rakkib_gamemaker`, which calls
`badil_nass` and `uktub` and nothing else. So the pool now holds logical Arabic that
the game's own `FONT` chunk has no picture for, in an engine that neither joins nor
reorders. The write succeeds, the install succeeds, and the text it wrote cannot be
drawn.

A one-button run that ended here would hand somebody a game whose menus had gone
blank. That is the whole reason `jahiziya` answers `Ghaiba` for a row whose write
demonstrably works.

## Electron

`mulhaqat_muharrik` deploys nothing for this family, and the adapter states its own
gap rather than writing around it. `rakkib_ghilaf` (`tarkeeb.rs`) is reached from
the install pipeline and returns `HimlMarfud` when `Mawarid::tashghil_ghilaf` is
`None`, because a translation table injected into somebody's `app.asar` with no
runtime to read it changes nothing on screen. The refusal travels out of
`raqqi_nusus` as `NususMarfuda` (`crates/taarib-tathbeet/src/nusus.rs`) and stops
the install — outcome 2, nothing left behind.

`tashghil_ghilaf` comes from `iqra_tashghil` (`nusus.rs`) reading
`mulhaq/electron/taarib.js` out of the component store, which `taarib-tajmee`
stages from `target/adapters/electron/taarib.js` (`masfufa.rs`). As with RPG
Maker, `adapters-script/ibni.mjs` now compiles it and `scripts/isdar.sh` runs that
compiler, and the staged tree on the authoring machine holds the file — so
`imkaniyat::naqs_electron` states a gap the tree has closed on the production
side. What has not been established is that the compiled runtime does anything in
a real game.

There is a reason to doubt it, and it is a break inside the adapter that survives a
perfect install. Its canvas rung resolves the text engine through
`globalThis.taaribNawat` (`adapters-script/electron/taarib.ts`), and that name is
read once in the whole repository and written nowhere — unlike the RPG Maker
adapter, this one never loads the WebAssembly core. The takeover rung therefore
takes its failure branch, logs that the page world has no Taarib engine, and leaves
canvas text to the game.

## Capcom BIO4

Resident Evil 4's 2005 codebase is recognised (`taarib-mustalahat` names it
`Bio4`, `crates/taarib-muharrik/src/dalail/bio4.rs` detects it) and qualifies for
tier 1: its text sits in dictionary files and its fonts are baked glyph pages, so
Taarib can shape and reorder before writing and generate the pages itself. The
three pieces exist: `taarib-istikhraj`'s `qamus` module reads the game's
dictionaries and rebuilds them, `taarib-muhawwil-bio4` reads the `.fnt` metrics,
the embedded TPL, the cell grid and the `ImagePack` atlas and can build a font into
them, and that crate's `kharita` module carries the code-point-to-cell table read
out of `bio4.exe`.

What is missing is the plumbing between them, and `imkaniyat::naqs_bio4` says so:
`tarkeeb::rakkib_luba` has no arm for the family (a grep for `Bio4` over
`crates/taarib-muhawwil-nusus/src/tarkeeb.rs` returns nothing), and
`taarib-tathbeet`'s `tarkib` groups BIO4 with the engines that get no additive
step in both `hajat_itar` and `mulhaqat_muharrik`. No install touches one of these
games. `naqs_bio4`'s own comment records a third gap inside `taarib-muhawwil-bio4`'s
`naql`; that one was not re-verified here.

## The six named in-house engines

Frostbite, BlackSpace, Alchemy, Dantelion, RAGE and Snowdrop are identified from
their own files (`crates/taarib-muharrik/src/dalail/khassa`) and every one of
them is `qabil_lil_tarqee() == false` in `taarib-mustalahat`: no plugin system, no
scripting runtime, no container format this build can read.
`tabaqa_min_muharrik` sends all six through `tabaqa_khassa` to tier 3, `khazinat_nusus`
names where each keeps its text so the report can say what it cannot open, and
`naqs_khassa` tells the player the engine is known and not yet reachable — which
is a different sentence from "unrecognised", and the reason the six were named at
all. `taarib-istikhraj`'s `tawjih::li_aila` gives each of them an explicit
no-extractor arm rather than a wildcard.

Two lists in the tree have not caught up with them, and neither the compiler nor a
test says so: `KUL_AILAT` in `crates/taarib-tilqai/src/fahs.rs` still holds ten
families, so the one-button gate test walks ten of seventeen, and `MUHARRIKAT` in
`apps/studio/src/maktaba/hifz_khiyarat.ts` holds the same ten, so the library
filter cannot offer BIO4 or any of the six. Both are the "lists nothing will tell
you about" that `CONTRIBUTING.md` warns about, and both should be brought up to
seventeen.

## The overlay, which is tier 3 for every unrecognised engine

The overlay genuinely starts, which makes it the most misleading row here. It has
eight backends now — Direct3D 8, 9, 10, 11 and 12, OpenGL in its fixed-function
and modern profiles, and Vulkan — and `bidaya.rs` probes a game's modules newest
generation first (`d3d12.dll`, `d3d11.dll`, `d3d10.dll`, `d3d9.dll`, `d3d8.dll`,
`opengl32.dll`; `libGL.so.1` on Linux). On the Direct3D and OpenGL paths the
payload hooks the present entry points, takes the first present, constructs a
backend around the game's own device, and builds a complete pipeline — shaders,
input layout, blend and raster state, render target, vertex scratch. Then it sets
`INTAHAT`, and from the second frame onward every present thunk returns
immediately at the guard in `shaghghil_min_itar`. The overlay is alive, correct,
and never called again.

Behind that guard sat three more breaks. **Two are closed**; the third is not, and
it is still sufficient on its own.

* ~~**No draw batch is ever built.**~~ **Closed.** `src/talqeem.rs` is the producer:
  `Mulaqqim::ibni` takes translated lines with their screen rectangles and the
  control panel's own `AnsurLawha`, shapes both through `taarib-saff`, packs every
  glyph into a runtime `taarib_lawha::namu::LawhaHayya`, and returns the
  `LawhatRasm` that `BaniDufa::ikhtim` builds. `Mulaqqim::qaddim` is the whole
  loop — build, upload, `Tabaqa::itar`.
* ~~**No atlas is ever uploaded.**~~ **Closed.** `Mulaqqim::irfa` expands the page
  through `rasm_tabaqa::ila_rgba` and calls `Tabaqa::arfa_lawha`, once, whenever a
  glyph the atlas had not seen turns up.
* **No text ever arrives.** Still true, and now the only break on this path.
  `Tabaqa::iltaqit` (`src/wajiha.rs`) has no caller in `src/` — only an example
  drives it — the capture and OCR modules have no inbound edge from the bootstrap,
  the mapped `.ruqaa` is reachable only through `bi_ruqaa` (`src/bidaya.rs`) which
  nothing calls, and there is no worker thread in the crate. Nothing produces the
  `SatrMulaqqam` values the producer consumes.

The producer is also not yet wired to a frame. `Mulaqqim` appears in `src/lib.rs`
as a re-export and nowhere else in `src/`; the only code that drives it is
`crates/taarib-tabaqa/tests/talqeem.rs` and
`crates/taarib-tabaqa/examples/talqeem_burhan.rs`. So "a present hook calls
`qaddim`" is the design, not the tree.

The producer was verified by rendering rather than by argument: `talqeem_burhan.rs`
runs the real `Tabaqa::shaghghil` → `Mulaqqim::qaddim` → `Tabaqa::itar` →
`Khattaf::irsim` path against a software `Khattaf` and writes a PNG. Doing so found
two defects that no review had caught, both in code that had never been executed: the
atlas page was expanded to white-with-coverage-in-alpha, which against a
premultiplied quad colour drew every glyph as a solid rectangle; and the backing
plate was placed from a pen the glyph path read as the layout's top edge and the
plate path read as a baseline. Both are fixed and both have regression tests in
`crates/taarib-tabaqa/tests/talqeem.rs`.

Nothing above changes what a player sees: with no text source and no present-path
caller, the overlay still puts nothing on screen, and `imkaniyat::jahiziya` still
says so.

Vulkan is a separate entry point and behaves the same way for a different reason. The
layer is live — it negotiates with the loader, follows the dispatch chain and
intercepts `vkQueuePresentKHR` — but the draw callback is never registered, because
`sajjil_munadi` (`src/vulkan.rs`) has no caller. The present handler reads `None`
and forwards the application's present info byte for byte. `KhattafVulkan` is never
constructed anywhere in `src/`.

## One cross-cutting fact

`taarib/<id>.ruqaa` is the only content the installer places inside a game, and no
script-engine adapter reads it: a grep for `ruqaa` or `TRQ1` over `adapters-script/`
returns nothing. The four script adapters read formats of their own —
`taarib.json`, `taarib/idad.json`, `Taarib/idad.txt`, `taarib/hamula.json` — and of
those, `taarib-muhawwil-nusus` writes `game/taarib/idad.json` (`renpy.rs`,
`MALAF_BAYANAT`) and the Electron payload's `hamula.json` (`electron.rs`); the RPG
Maker and VX Ace files are written by no Rust in this tree. Only the Unity adapters
read the patch format, and those are the assemblies whose build is a local step.

That is a narrower fact than it used to be. The four script engines no longer need
the `.ruqaa` at run time for their *text*, because `tarkeeb` writes the Arabic into
their own data before the game ever starts. What the adapters still cannot read is
everything else a patch carries — the settings, the font, the rung.

## Keeping this file true

`docs/bidaya.md` §6 records the same facts for the three native payloads, from the
inside, and the two files must agree. When a seam closes:

1. change the arm in `taarib-muharrik`'s `imkaniyat::jahiziya`, including its
   sentence, which is what a user reads, and its test at the foot of that file;
2. raise `imkaniyat::ISDAR_FAHS` in the same commit, or every library that has
   already been scanned keeps the old verdict forever — it is **7** as this is
   written, and the constant's own doc comment records what each raise was for;
3. update the row here and in `docs/bidaya.md` §6.

**Three arms carry sentences the tree has overtaken**, and a reader should treat
them as the least settled things on this page. On 2026-09-05 the repository gained a
build pipeline it did not have — `adapters-script/ibni.mjs`, `scripts/isdar.sh`, and
`.github/workflows/isdar.yml` — which produces the four Unity assemblies, the
game-side cdylibs for every payload triple, the wasm pair, and both TypeScript
adapters, and a bundle staged with it exists on the authoring machine. That closes
the *production* half of the reason three arms give:

| arm | stated reason | status |
| --- | --- | --- |
| `naqs_unity`, IL2CPP | no per-target `taarib_jisr` is produced | `isdar.sh` builds it per triple; the staged tree carries it |
| `naqs_rpg_maker` | the plugin is not built into this package | `isdar.sh` builds it |
| `naqs_electron` | the renderer runtime is not built | `isdar.sh` builds it; the staged tree carries it |

None of the three verdicts has moved, and none should move on the strength of a
build step alone. `jahiziya` answers whether **a player sees legible Arabic**, and
producing an artifact is not evidence about a screen. What these three now need is
the thing this whole document says it does not have for any row: a game, an install,
and somebody looking at it. Until then the arms are right and their sentences are
stale, which is the least bad of the available states and is why it is written down
here rather than quietly corrected in the code.

The three sentences should be reworded when somebody re-reads those adapters against
the new tree, and `ISDAR_FAHS` raised again in the same commit — it has been raised
for other reasons since this table was first written, without those sentences
moving, so the number that appears here is not the one that will accompany the fix.

A row that says a chain stops somewhere it no longer stops is worse than no row: it
is a claim with a file and a function name attached, which is exactly the shape of
a thing people stop checking.
