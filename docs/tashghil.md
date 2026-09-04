# التشغيل — what runs inside a game, engine by engine

This file is the evidence behind one field. `TaqreerImkaniyat::jahiziya` tells a
user whether the tier their game qualifies for is a tier this build can actually
deliver, and `taarib-muharrik`'s `imkaniyat::jahiziya`
(`crates/taarib-muharrik/src/imkaniyat.rs:491`) is the table that answers it. A
table of claims with nothing behind it is worse than no table at all, so the
reading that produced each row is written down here, with the file and the
function where each chain stops.

`imkaniyat.rs` is the authority and this file is its working. Nineteen tests at
the foot of that file pin the verdicts; if this document and that function ever
disagree, the function is right and this document is stale.

## What the evidence is, exactly

Every verdict below rests on **reading code**. No game of any of these engines is
installed on this machine, and no engine's in-game half has been observed putting
a glyph on a screen. Where this document says a function has no caller, that is a
grep over `crates/`, `apps/`, `unity/` and `adapters-script/`, not an impression.
Where it says a write happens, that is a round trip against a fixture the project
authored — it proves the *write*, not a screen.

There is exactly one result from a real engine, and it is worth stating precisely
because it is the only one: the official Godot **3.6.stable** headless binary
loaded a `.translation` resource this project produced and `tr("Hello")` came back
`مرحبا`. That was driven from a test, not from an installer, and it proves the
delivery format is correct — nothing about whether a player ever reaches it.

## Three outcomes, not one

Until recently "nothing renders" was true everywhere and one sentence covered the
whole table. It no longer is, and the difference matters more to a user than
anything else on this page, because two of these rows end with a game that is
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
above is outcome 3 *conditionally*, and that condition is a fact about the game
rather than about Taarib — see its section. Those three rows are why
`sadr_jahiziya` (`imkaniyat.rs:554`) says "what makes Arabic appear **readably**"
rather than "your game will not change": for most of this table the game really
does not change, and for those three it does.

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
| Unity / Mono | 1 | `Ghaiba` | 1 | the component is deployed and the chainloader will not load it | `MulhaqTaarib.Awake` — `unity/Taarib.Unity.Mono/Taarib.cs:275` |
| Unity / IL2CPP | 1 | `Ghaiba` | 1 | the assembly matches its loader and has no native library under it | `Jisr`'s `DllImport`s — `unity/Taarib.Unity.Jisr/Jisr.cs:64` |
| Unreal | 1 | `Ghaiba` | 1 | the loader, and one console variable written into `Engine.ini` | `KatibPak::uktub_fi_luba` — `crates/taarib-muhawwil-unreal/src/mawarid/pak.rs:2989` |
| Godot 4 | 1 | `Ghaiba` | 1 | nothing; the engine is never told to load the extension | `tahyia_mustawa` — `crates/taarib-muhawwil-godot/src/imtidad.rs:2054` |
| Godot 3 | 2 | `Ghaiba` | 1 | the loader, the GDNative binding, and the patch is mapped | `sallim` — `crates/taarib-muhawwil-godot/src/bidaya.rs:1151` |
| RPG Maker MV / MZ | 1 | `Ghaiba` | 2 | `data/*.json` is spliced, then the deployment step refuses | `asmaa_mukawwin` — `crates/taarib-tathbeet/src/tarkib.rs:1966` |
| RPG Maker VX Ace | 1 | `Ghaiba` | 1 | nothing | `vxace::rakkib` — `crates/taarib-muhawwil-nusus/src/vxace.rs:2729` |
| Ren'Py ≥ 7.4 | 1 | **`Naqisa`** | 3\* | the whole tier except an Arabic font | `Mawarid::khatt_renpy` — `crates/taarib-tathbeet/src/nusus.rs:253` |
| Ren'Py < 7.4 | 1 | `Ghaiba` | 3 | the translation installs; the engine blits one character at a time | `saf_mulhaq` — `crates/taarib-tajmee/src/masfufa.rs:206` |
| GameMaker | 1 | `Ghaiba` | 3 | the string pool is rewritten and no glyph pages are made | `rakkib_gamemaker` — `crates/taarib-muhawwil-nusus/src/tarkeeb.rs:713` |
| Electron | 1 | `Ghaiba` | 2 | the adapter refuses this package by name and stops the install | `rakkib_ghilaf` — `crates/taarib-muhawwil-nusus/src/tarkeeb.rs:782` |
| overlay (tier 3, every unrecognised engine) | 3 | `Ghaiba` | 1 | the present hook, the game's own device, a complete render pipeline, and a draw batch nothing feeds | `Tabaqa::iltaqit` — `crates/taarib-tabaqa/src/wajiha.rs:891` |

\* Ren'Py ≥ 7.4 is outcome 3 only when the game's own font has no Arabic letters.
When it has them, the translation is legible and the row is a real, if partial,
success. Nothing in this build knows which of the two a given game is.

## The caller that landed, and what it did not change

Four of these rows used to stop at "this function has no caller". They no longer
do. `taarib_muhawwil_nusus::tarkeeb` is the dispatcher
(`crates/taarib-muhawwil-nusus/src/tarkeeb.rs:341`, `rakkib_luba`); it is reached
from `taarib_tathbeet::nusus::raqqi_nusus` (`crates/taarib-tathbeet/src/nusus.rs:214`),
which `masar_tathbeet::thabbit` calls at `crates/taarib-tathbeet/src/masar_tathbeet.rs:135`
— before the deployment step, deliberately, because RPG Maker's extraction records
carry byte offsets into a `plugins.js` that deployment appends to.

So all four writers now run:

| writer | called from |
| --- | --- |
| `rpgmaker::rakkib` (`rpgmaker.rs:2258`) | `tarkeeb.rs:457` |
| `renpy::iktub_idad` (`renpy.rs:3093`) | `tarkeeb.rs:546` |
| `MuhawwilGameMaker::uktub` (`gamemaker.rs:3542`) | `tarkeeb.rs:751` |
| `electron::rakkib` (`electron.rs:1976`) | `tarkeeb.rs:822` |

VX Ace is the fifth script engine and is refused by name at `tarkeeb.rs:359`,
because its archive rewriter takes a Ruby payload the dispatcher does not carry.
That refusal is deliberate and is better than a route that half works.

**None of that moved a verdict.** A write is necessary and is not sufficient: a
string an engine has no letter pictures for is drawn blank, and a string an engine
without a shaper draws one character at a time is drawn as unjoined letters. Both
are writes that succeeded and neither is Arabic. What the caller changed is *which
rung* each row now fails at, and for two engines it changed the outcome from 1 to 3
— which is worse for the user and is why those two are called out above.

The rest of this file is each row in full.

## Unity, both backends

The C# is not the problem. `unity/` is about 48,000 lines with no
`NotImplementedException`, no `TODO`, and no stubbed method. The Mono adapter
takes four HarmonyX prefixes over `TextMeshProUGUI.GenerateTextMesh` and its
siblings (`unity/Taarib.Unity.Mono/Anzimat/TextMeshPro.cs:1911`, `:1914`),
installs them at `:2585`, and hands the finished mesh back to the renderer at
`:2474`. The IL2CPP adapter resolves its targets through a three-rung ladder —
Il2CppInterop metadata, the `il2cpp_*` runtime API, then a byte-signature scan
over the executable — and falls back to hand-written x64 and ARM trampolines
(`unity/Taarib.Unity.Il2cpp/Khatf/Mihmaz.cs:167`) when HarmonyX cannot take a
target.

It compiles now. `unity/**/bin/Release/**` holds `Taarib.Unity.Jisr.dll`,
`Taarib.Unity.Mushtarak.dll`, `Taarib.Unity.Mono.dll` and
`Taarib.Unity.Il2cpp.dll`, built 2026-09-05. That is a **local build output and
not repository content** — `.gitignore:48` excludes `unity/**/bin/` — and nothing
in the repository produces it: the only occurrences of `dotnet build` are the two
READMEs, `docs/bina.md:242`, and the instruction block `taarib-tajmee` prints,
which opens `This tool builds nothing. Run these first:`
(`crates/taarib-tajmee/src/main.rs:153`). There is no CI step; `.github/workflows/`
holds `ci.yml` and `macos-probe.yml`, and neither invokes `dotnet`.

The staging path that used to be wrong is fixed. `tajmiaat`
(`crates/taarib-tajmee/src/masfufa.rs:231`) now resolves the target-framework
directory per project — `net6.0` for `Taarib.Unity.Il2cpp`
(`unity/Taarib.Unity.Il2cpp/Taarib.Unity.Il2cpp.csproj:36`), `netstandard2.1` for
the other three — instead of reading all four out of `netstandard2.1`, finding
three, and withholding the manifest from a build that had in fact succeeded.

The two backends now fail at different rungs, and one sentence for both would be
false for whichever it did not describe.

* **Mono asks for a loader generation the bundle does not carry.**
  `Taarib.Unity.Mono.csproj:41` references `BepInEx.Unity.Mono` **6.0.0-be.780**;
  `assets/aqfal/qufl_bepinex.json` pins **v5.4.23.5** for this backend. Under
  BepInEx 5 the base types live in assemblies the plugin does not name, so the
  chainloader cannot bind it and `Awake` (`Taarib.cs:275`) is never entered.
* **IL2CPP matches its loader and has nothing underneath it.**
  `Taarib.Unity.Il2cpp.csproj:70` references `BepInEx.Unity.IL2CPP` 6.0.0-be.780
  and the lock ships **v6.0.0-pre.2** for this backend — the same generation. What
  is absent is a rung lower: all twenty-nine of `Taarib.Unity.Jisr`'s `DllImport`s
  bind the bare name `taarib_jisr` (`unity/Taarib.Unity.Jisr/Jisr.cs:64` onward,
  the only file in `Taarib.Unity.*` that declares one), which is matrix rows B1–B3
  out of the `taarib-jisr` cdylib. `saf_bepinex` stages it from
  `<target dir>/<triple>/release/` (`masfufa.rs:102`–`:109`), and no per-target
  build of that cdylib is produced by anything in the repository — the cross-target
  `cargo build --release --target …` line is in the same "run these first" block.
  A managed assembly with no native library beneath it loads and then fails at its
  first call.
* **World-space `TextMesh` would translate nothing even on a working install.**
  `NizamMujassam.Fahras`
  (`unity/Taarib.Unity.Mono/Anzimat/NassMujassam.cs:281`) is the scan that finds
  the labels, it is documented as being called on scene load, and there is no
  `SceneManager.sceneLoaded` handler anywhere in `unity/`. Registration succeeds,
  the log says the system is installed, and it guards zero labels.

## Unreal

This is the one adapter that does something to a real game. `taarib_bidaya`
(`crates/taarib-muhawwil-unreal/src/bidaya.rs:286` on Windows, `:296` elsewhere) is
resolved and called by `taarib-mudkhal`, resolves `IConsoleManager::Get` across the
module names in `WAHDAT_QAIDA` (`src/wasl.rs:51`) and `FSlateApplication::Get`
(`src/slate.rs:671`), and then writes `Slate.DefaultTextShapingMethod=2` and its two
companions into the game's `Engine.ini` under `[SystemSettings]`, atomically,
through `MalafIni::aktub` → `taarib_usus::masarat::kitaba_dharra_nass`
(`src/tashghil.rs:647`, `:662`). From the next launch that game's Slate does full
HarfBuzz shaping instead of kerning-only. On a game that is still in English,
nothing about that is visible.

Everything that would produce Arabic is declined by name:

* `bidaya.rs:418` calls `wasl::wasl(None)`. `FaharisAwamir` (`src/wasl.rs:85`) is an
  assertion that the caller verified this build's vtable slots, and a bootstrap
  inside a shipped game has not. So `muhayya` (`src/wasl.rs:314`) is false for the
  life of the process and `Tashghil::rutbat_haqn` returns at `src/tashghil.rs:916`
  before touching a console variable. The re-assert watchdog declines on the same
  flag at `bidaya.rs:818`.
* `TasheehQiyas::rakkib` (`src/qiyas.rs:978`) installs the measurement and
  justification detours. It has no caller, and `AhdafQiyas` (`src/qiyas.rs:249`) is
  constructed nowhere in the workspace. `sajjil_qiyas` (`bidaya.rs:715`) records the
  consequence in the log at `:730`: *no detour and no vtable slot was installed, so
  this process is byte-identical to the one this payload entered.*
* `khatt.rs` (font registration) and `alam.rs` (culture activation) have no callers
  at all.
* The `.ruqaa` beside the module is opened only to validate its framing and content
  hash, and is dropped unread at `bidaya.rs:412`.
* `KatibPak::uktub_fi_luba` (`src/mawarid/pak.rs:2989`), which would author the
  additive patch pak, is called only from `tests/hala_ahruf.rs:132` and `:158`.
  `taarib-istikhraj/src/unreal.rs` uses the `mawarid` readers and writes nothing.
* The Slate shaping switch itself is behind `--features hamula`, which no manifest
  in the workspace enables.

## Godot 4

`mulhaqat_muharrik` (`crates/taarib-tathbeet/src/tarkib.rs:2071`) deploys only for
Godot **3** (`:2081`–`:2087`), and nothing anywhere writes a `.gdextension`
manifest, so the engine never resolves `taarib_imtidad` and the library is never
loaded.

If it were, it would bind all sixteen GDExtension interface functions by name
(`ASMA_DAWAL`, `crates/taarib-muhawwil-godot/src/imtidad.rs:501`) and register a
level-initialisation callback (`:2025`), and then stop at `tahyia_mustawa`
(`:2054`), because `KHADIM` is empty: `thabbit_khadim` (`:1864`) and `thabbit_turuq`
(`:1851`) have no callers, so the `extension_api.json` method hashes that binding
needs never arrive. The refusal at `:2073` names the missing call outright.

The offline route is dead in a different way from before. `BinaHawiya`
(`src/pck/hawiya.rs:1299`) still has no caller, so the additive `taarib.pck` named
by `ISM_HAZMA` (`src/lib.rs:109`) is never produced. `Tarjama::ila_bayt`
(`src/pck/tarjama.rs:1828`, `:2262`) **does** have callers now — `tawseel.rs:650`
and `:652` — but `TawseelThalith` is reached only through `thabbit_tawseel`, which
is the Godot 3 seam below and has no caller either.

## Godot 3

The installer does its half. `godot_thalatha` (`tarkib.rs:2178`) writes
`taarib.gdnlib` (`:2186`) and adds the singleton registration to `override.cfg`
(`:2197`–`:2210`), so Godot itself loads the library and calls
`taarib_gdnative_init` (`crates/taarib-muhawwil-godot/src/bidaya.rs:1362`). That
binds the GDNative core API and verifies its type and major version (`:445`) and
maps every `*.ruqaa` beside the module (`:1040`).

It stops in `sallim` (`bidaya.rs:1151`), and the log line at `:1191` says exactly
why: *the engine is bound and neither half of the Godot 3 path was installed: the
patch's companion called neither `thabbit_tawseel`, which delivers the translated
text, nor `thabbit_istila` with this build's `Font::draw`, `Font::draw_char` and
`Font::get_string_size` addresses, its `VisualServer` binding and the patch's
layout policy.*

Both seams are open. `thabbit_tawseel` (`:805`) and `thabbit_istila` (`:780`) have
zero callers, so `awsil` (`:1089`) takes its refusal branch at `:1095` before a
`.translation` resource is written, and `sallim` takes the `Mumtania` branch at
`:1191` before a detour is installed. `istila.rs` is complete — three detours,
glyph emission through `VisualServer::canvas_item_add_texture_rect_region`, and a
guard at `istila.rs:2570` that refuses a half-supplied configuration — and it
refuses to guess an address, which is correct: a detour installed on the wrong
function corrupts a stranger's game.

`sallim` carries a third outcome that nothing can currently reach. When the
delivery went in and the takeover did not, it answers `HalatBidaya::Naqisa` at
`:1188` and says the game's text will be Arabic from the next launch and Godot 3
will draw it unshaped. That branch is behind `awsil` returning true, which is
behind `thabbit_tawseel`, so today every Godot 3 game takes the `Mumtania` path
and the game is exactly what it was.

This row is where the one real-engine result sits: `TawseelThalith::hayyi`
(`crates/taarib-muhawwil-godot/src/tawseel.rs:805`) produced a resource that
Godot 3.6.stable headless loaded and answered `tr("Hello")` from. It was driven
from a test. It proves the delivery is correct; it does not put a caller in the
installer.

## RPG Maker MV and MZ

The data splice is real and it runs. `rakkib_rpgmaker` (`tarkeeb.rs:418`) rediscovers
the project, extracts, checks every translation for the escape codes the engine
substitutes at draw time (`rpgmaker::tahaqquq_hurub`, `rpgmaker.rs:2226`), and calls
`rpgmaker::rakkib` (`rpgmaker.rs:2258`) at `tarkeeb.rs:457`.

**The install does not survive it.** `mulhaqat_muharrik` routes this family to
`rpg_maker` (`tarkib.rs:2099`) unconditionally, which asks `asmaa_mukawwin` for the
contents of `mulhaq/rpgmaker/{mv,mz}` and raises `MukawwinMafqud` (`tarkib.rs:1003`)
when the component store does not hold it. The deployment step runs *after* the
splice (`masar_tathbeet.rs:135` then `:141`), so the splice happens and is then
rolled back with the failed install. That is outcome 2: nothing is left in the game.

Whether the store holds it depends on a step outside cargo. `taarib-tajmee` stages
row I2 from `target/adapters/rpgmaker/taarib.js` (`masfufa.rs:180`, `:183`). Until
2026-09-05 nothing in the repository produced that file. `adapters-script/ibni.mjs`
now does — it drives `tsc` for I2 and `esbuild` for I1, and `package.json` exposes it
as `pnpm bina` — and no CI workflow invokes it, so a bundle staged without that step
still has no RPG Maker support. `imkaniyat::jahiziya` has not moved for this family
and its arm's stated reason (`imkaniyat.rs:739`) is the one now in motion; see
"Keeping this file true".

Two further defects sit behind that one and are not what stops it. The registration
is written with `"parameters":{}` (`tarkib.rs:70`) because `IdadatMulhaq::barametr`
(`rpgmaker.rs:2374`) is only ever called from `rakkib_mulhaq` (`rpgmaker.rs:2432`),
which has no caller — so a built plugin would read its own defaults, log that no
font is configured, and never enter its takeover rung. And the plugin is what
corrects direction, alignment and window mirroring, which Chromium's `fillText`
does not do for the Arabic it otherwise joins correctly.

## RPG Maker VX Ace

Nothing is installed, by design: the Ruby is meant to travel inside the patch rather
than as a component, `hajat_itar` answers `SababLaHaja::DakhilAlRuqaa`
(`tarkib.rs:468`) and `mulhaqat_muharrik` returns `Ok(())` on the shared arm at
`tarkib.rs:2093`. The only thing that would insert `taarib_rgss3.rb` into
`Scripts.rvdata2` is `vxace::rakkib`
(`crates/taarib-muhawwil-nusus/src/vxace.rs:2729`), whose Ruby source is
caller-supplied and which has no caller. `tarkeeb::rakkib_luba` refuses this family
by name (`tarkeeb.rs:359`) rather than routing it through an entry point that carries
no payload.

There is a second, independent break behind that one. The script's takeover rung
needs `Taarib/lawha.png` and `Taarib/lawha.tbl` — `vxace.rs:2585` and `:2588` name
both and `:2640` writes them into the settings file it generates — and nothing in the
tree produces either. Injected but unfed, the adapter raises at
`adapters-script/vxace/taarib_rgss3.rb:1703`, its caller catches it, logs that the
glyph takeover declined, and falls back to its direction-only rung.

## Ren'Py at 7.4 and above — the one `Naqisa`

This is the only row in the table where most of the tier arrives, and it is the one
row a reader should be careful with.

Everything but the font installs. `rakkib_renpy` (`tarkeeb.rs:510`) writes
`game/tl/arabic/` — `renpy::iktub_idad` (`renpy.rs:3093`) from `tarkeeb.rs:546`,
`iktub_mustalahat` (`:3197`) and `iktub_hiwar` (`:3276`) from `:548` and `:550` — and
the generated `.rpy` invokes `taarib_renpy.rakkib`
(`adapters-script/renpy/taarib_renpy/__init__.py:257`), which selects the language
through `config.language` and sets the direction and alignment via `sajjil_ittijah`
(`__init__.py:488`). Every write is additive: Ren'Py compiles anything under
`game/tl/<language>/` by itself with no registration step, so uninstalling is a
delete. The `taarib_renpy` package itself really is deployed, because staging row I3
is a copy of a directory that is in this repository
(`masfufa.rs:206`–`:211`) and `tarkib::renpy` (`tarkib.rs:2155`) places it under
`game/` (`:2162`).

The font does not arrive. `raqqi_nusus` passes `Mawarid::khatt_renpy` as `None`
(`crates/taarib-tathbeet/src/nusus.rs:253`), because the component store's Ren'Py
deployment step copies `mulhaq/renpy/**` and no font with it, so there is no name
this call could honestly hand over. `rakkib_renpy` records that in the install report
(`tarkeeb.rs:528`–`:536`) and the generated file registers no face at all — which is
the correct choice, because Ren'Py assigns whatever name it is given to every `gui`
font variable, and an empty one is a game with **no** font rather than a game with
its own.

So the game is drawn in the font it shipped with. If that face carries Arabic, the
translation appears as it should and this row is a partial success. If it does not,
the player sees empty boxes where the text was — a game visibly worse than the one
they installed over. Nothing in this build can tell which of the two a given game is,
which is exactly why the verdict is `Naqisa` and the sentence
(`imkaniyat.rs:788`) states both outcomes instead of averaging them into a promise.

## Ren'Py below 7.4

Below `dalail::nusus::HADD_TASHKEEL_RENPY` the engine has no HarfBuzz and no
FriBidi, and its text layout is a per-character blit. Everything in the section
above still installs — the translation, the language selection, the direction — so
the install completes and the game does change. It changes to separated letters in
the wrong order, which is not Arabic a person can read.

The adapter's own answer to that is the glyph takeover in
`taarib_renpy._rakkib_istila` (`__init__.py:588`), which loads `taarib_jisr` through
`jisr.hammil` (`adapters-script/renpy/taarib_renpy/jisr.py:954`) from
`<game>/taarib/jisr/<arch>/` — the default `dalil_jisr` at `__init__.py:193`, joined
at `__init__.py:622`, with the platform file names `_ism_maktaba` produces
(`jisr.py:710`). Nothing stages a library there: `saf_mulhaq`
(`crates/taarib-tajmee/src/masfufa.rs:172`) copies the Python package alone
(`:206`–`:211`), and the B1–B3 rows go into each BepInEx component instead
(`:102`–`:109`). So `hammil` raises `GHAYR_MUHAYYAA` naming both candidate paths, the
takeover declines at every launch, and the configuration rung is all that is left.

`docs/tawzee.md` row B1 used to claim staging placed `taarib_jisr` under
`mulhaq/renpy/taarib/jisr/<mimariya>/`. It does not, and that row has been corrected;
the gap is real and belongs to staging, not to the adapter.

## GameMaker

GameMaker is the one engine with no runtime component at all, and that is correct
rather than a gap: the Arabic goes into `data.win` and the game reads it as its own.
`hajat_itar` says so (`crates/taarib-tathbeet/src/tarkib.rs:468`) and
`mulhaqat_muharrik` deploys nothing (`:2093`).

The rewriter is 4,900 lines and complete — the FORM chunk table, the string pool,
texture page append, font glyph tables, `draw_set_halign` bytecode correction, a
round-trip proof and an atomic write — and **it now runs**. `rakkib_gamemaker`
(`tarkeeb.rs:713`) locates the container, replaces every pool entry the patch has
Arabic for through `MuhawwilGameMaker::badil_nass` (`gamemaker.rs:2742`), and calls
`MuhawwilGameMaker::uktub` (`gamemaker.rs:3542`) at `tarkeeb.rs:751`, which verifies
the whole container in memory before the guard is handed a byte.

That is why this row is outcome 3 and not outcome 1. A GameMaker font is a **baked
glyph table**: the container ships pictures of letters and an index from character
codes to pictures. There is no font file to swap and no shaping stage to configure,
and the only functions that would add pictures — `sajjil_ashkal`
(`gamemaker.rs:4045`), `nass_manqul` (`:4071`), `istabdil_khatt` (`:2973`),
`alhiq_safha` (`:2906`) — are not called by `rakkib_gamemaker`, which calls
`badil_nass` and `uktub` and nothing else. So the pool now holds logical Arabic that
the game's own `FONT` chunk has no picture for, in an engine that neither joins nor
reorders. The write succeeds, the install succeeds, and the text it wrote cannot be
drawn.

A one-button run that ended here would hand somebody a game whose menus had gone
blank. That is the whole reason `jahiziya` answers `Ghaiba` for a row whose write
demonstrably works.

## Electron

`mulhaqat_muharrik` deploys nothing for this family (`tarkib.rs:2093`), and the
adapter states its own gap rather than writing around it. `rakkib_ghilaf`
(`tarkeeb.rs:771`) is reached from the install pipeline and returns `HimlMarfud` at
`tarkeeb.rs:783` when `Mawarid::tashghil_ghilaf` is `None`, because a translation
table injected into somebody's `app.asar` with no runtime to read it changes nothing
on screen. The refusal travels out of `raqqi_nusus` as `NususMarfuda`
(`crates/taarib-tathbeet/src/nusus.rs:262`) and stops the install — outcome 2,
nothing left behind.

`tashghil_ghilaf` comes from `iqra_tashghil` (`nusus.rs:280`) reading
`mulhaq/electron/taarib.js` out of the component store, which `taarib-tajmee` stages
from `target/adapters/electron/taarib.js` (`masfufa.rs:195`, `:196`). As with RPG
Maker, `adapters-script/ibni.mjs` now compiles it and no CI step runs that compiler.

There is a third break inside the adapter itself, and it survives a perfect install.
Its canvas rung resolves the text engine through `globalThis.taaribNawat`
(`adapters-script/electron/taarib.ts:1326`), and that name is read once in the whole
repository and written nowhere — unlike the RPG Maker adapter, this one never loads
the WebAssembly core. The takeover rung therefore takes the failure branch at
`taarib.ts:1364`, logs that the page world has no Taarib engine, and leaves canvas
text to the game.

## The overlay, which is tier 3 for every unrecognised engine

The overlay genuinely starts, which makes it the most misleading row here. On D3D11,
D3D12 and OpenGL the payload hooks `Present`, `Present1`, `ResizeBuffers`,
`ExecuteCommandLists` and the GL swap entry points
(`crates/taarib-tabaqa/src/bidaya.rs:499`, `:463`), takes the first present,
constructs a backend around the game's own device, and builds a complete pipeline —
shaders, input layout, blend and raster state, render target, vertex scratch. Then it
sets `INTAHAT` (`bidaya.rs:790`), and from the second frame onward every present thunk
returns immediately at the guard in `shaghghil_min_itar` (`bidaya.rs:763`, guard at
`:764`). The overlay is alive, correct, and never called again.

Behind that guard sat three more breaks. **Two are closed**; the third is not, and it
is still sufficient on its own.

* ~~**No draw batch is ever built.**~~ **Closed.** `src/talqeem.rs` is the producer:
  `Mulaqqim::ibni` (`:329`) takes translated lines with their screen rectangles and
  the control panel's own `AnsurLawha`, shapes both through `taarib-saff`, packs every
  glyph into a runtime `taarib_lawha::namu::LawhaHayya`, and returns the `LawhatRasm`
  that `BaniDufa::ikhtim` builds. `Mulaqqim::qaddim` (`:405`) is the whole loop —
  build, upload, `Tabaqa::itar`.
* ~~**No atlas is ever uploaded.**~~ **Closed.** `Mulaqqim::irfa` (`:374`) expands the
  page through `rasm_tabaqa::ila_rgba` and calls `Tabaqa::arfa_lawha`, once, whenever
  a glyph the atlas had not seen turns up.
* **No text ever arrives.** Still true, and now the only break on this path.
  `Tabaqa::iltaqit` (`src/wajiha.rs:891`) has no caller, the capture and OCR modules
  have no inbound edge from the bootstrap, the mapped `.ruqaa` is reachable only
  through `bi_ruqaa` (`src/bidaya.rs:1330`) which nothing calls, and there is no worker
  thread in the crate. Nothing produces the `SatrMulaqqam` values the producer
  consumes.

The producer is also not yet wired to a frame. `Mulaqqim` appears in `src/lib.rs:130`
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
`sajjil_munadi` (`src/vulkan.rs:786`) has no caller. The present handler reads `None`
at `vulkan.rs:1518` and forwards the application's present info byte for byte.
`KhattafVulkan` is never constructed at all.

## One cross-cutting fact

`taarib/<id>.ruqaa` is the only content the installer places inside a game, and no
script-engine adapter reads it: a grep for `ruqaa` or `TRQ1` over `adapters-script/`
returns nothing. The four script adapters read formats of their own —
`taarib.json`, `taarib/idad.json`, `Taarib/idad.txt`, `taarib/hamula.json` — and no
Rust in this tree writes any of them. Only the Unity adapters read the patch format,
and those are the assemblies whose build is a local step.

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
   already been scanned keeps the old verdict forever — it is **3** today, raised
   from 2 when Ren'Py stopped answering `Ghaiba` unconditionally;
3. update the row here and in `docs/bidaya.md` §6.

Two rows are in motion as this is written, and a reader should treat them as the
least settled things on the page: `adapters-script/ibni.mjs` landed on 2026-09-05 and
compiles the Electron and RPG Maker adapters, which is the component both of those
rows refuse for, and the Unity assemblies were built locally the same day. Neither is
reached by CI and neither has moved a `jahiziya` arm. When they do, the arms at
`imkaniyat.rs:739` and `:898` are the ones to change, and `ISDAR_FAHS` goes to 4.

A row that says a chain stops somewhere it no longer stops is worse than no row: it
is a claim with a file and a line number attached, which is exactly the shape of a
thing people stop checking.
