# Taarib.Unity

The four C# assemblies that run inside a Unity game process. They are the game-side half
of tier 1 — تعريب كامل, full Arabization, text replaced inside the game rather than drawn
over it — for both Unity scripting backends: **Mono**, where the game's managed code is
real IL in `Assembly-CSharp.dll` and reflection works the ordinary way, and **IL2CPP**,
where that code was compiled ahead of time into `GameAssembly.dll` and reaches managed
code only as Il2CppInterop proxies.

Nothing in this tree makes policy. Boundary 3 of section 4.4 of `ROADMAP.md` is the rule:
an adapter renders what a patch tells it to render. It does not decide whether a patch
applies, does not check safety, does not talk to the network, and does not read the
registry. All of that happened in Taarib Studio before the game launched. By the time
this code runs, the only questions left are which text systems exist in this process and
how to put the right glyphs in the right places.

## Table of Contents

- [The projects](#the-projects)
- [Why the takeover lives in Mushtarak](#why-the-takeover-lives-in-mushtarak)
- [Why Jisr is the only way to the native library](#why-jisr-is-the-only-way-to-the-native-library)
- [Decision 5: the game's own fonts are never touched](#decision-5-the-games-own-fonts-are-never-touched)
- [What is on disk at install time](#what-is-on-disk-at-install-time)
- [Which phase fills which project](#which-phase-fills-which-project)
- [How BepInEx is pinned](#how-bepinex-is-pinned)
- [The build settings, and why each one is there](#the-build-settings-and-why-each-one-is-there)
- [Two honest constraints of the target frameworks](#two-honest-constraints-of-the-target-frameworks)

## The projects

| Project | Name | Assembly | Target | References | Ships to |
| --- | --- | --- | --- | --- | --- |
| `Taarib.Unity.Jisr` | جسر — bridge | `Taarib.Unity.Jisr.dll` | `netstandard2.1` | none | both backends |
| `Taarib.Unity.Mushtarak` | مشترك — shared | `Taarib.Unity.Mushtarak.dll` | `netstandard2.1` | Jisr | both backends |
| `Taarib.Unity.Mono` | — | `Taarib.Unity.Mono.dll` | `netstandard2.1` | Jisr, Mushtarak | Mono games |
| `Taarib.Unity.Il2cpp` | — | `Taarib.Unity.Il2cpp.dll` | `net6.0` | Jisr, Mushtarak | IL2CPP games |

`Jisr` is the managed side of the C ABI: one `DllImport` per `taarib_*` entry point, a
`SafeHandle` subclass per opaque handle, the blittable structs laid out against
`crates/taarib-jisr/include/taarib.h`, the `Span<T>` buffer ownership that
`taarib_takhtit` requires, the loader that picks the shared library matching the
process architecture, and the translation of `int32` status codes into `Khata` values
carrying the same stable machine codes Studio reports.

`Mushtarak` is the takeover itself: reading the installed `.ruqaa` container, `lawha`
atlas residency, `rasm` mesh construction, `nasq` markup spans, container reflow
geometry, and the `iltiqat` capture channel.

`Mono` and `Il2cpp` are thin by design. Each is a BepInEx plugin plus the parts that
cannot be shared — HarmonyX patch sets and cached reflection delegates on one side, the
`hall` resolution ladder, the `basmat` signature database and `khatf` native detours on
the other.

## Why the takeover lives in Mushtarak

Two Unity backends means two hooking stories and, for every previous attempt at Arabic in
Unity games, two renderers. That is where those projects die: the Mono build gets a fix,
the IL2CPP build does not, and the two slowly stop producing the same picture from the
same patch. Phase 7's first hard constraint refuses that outcome — *identical rendering
behaviour to Phase 6, and the shared assembly guarantees it structurally.* Structurally is
the operative word. Not "verified by a test suite", which decays. Not "kept in sync by
review", which is a promise about people. The same IL runs in both processes, so a
kashida placed one pixel differently in IL2CPP than in Mono is not a bug class that
exists.

So the split is not by backend. It is by whether the code names a Unity type.

`Mushtarak` names none, and it cannot. Under Mono, a `TMP_Text` is a type in the game's
own `Unity.TextMeshPro.dll`. Under IL2CPP the same component arrives as an
Il2CppInterop-generated proxy in an assembly with a different identity, regenerated per
game from that game's metadata. An assembly with a compile-time reference to either one
would fail to load in the other's process — the type it asked for does not exist there
under that name. This is exactly why the project carries no `PackageReference` at all.

`Mushtarak` therefore owns the **decisions and the data**, and each adapter owns the
**calls**. The atlas is a concrete example. `Mushtarak` decides the format is `R8`, that
there are no mipmaps and no compression, produces the byte array in the exact layout the
upload wants, and knows which submesh takes which material. The adapter performs the
`Texture2D` construction, the raw upload and the `Apply` against whichever Unity binding
its runtime has. Same for meshes: `Mushtarak` writes vertices, UVs, colours and indices
into destination buffers; the adapter is what knows that the destination is a
`TMP_MeshInfo` array here and a `VertexHelper` there.

The consequence is that the interesting work — bidirectional reordering interacting with
rich-text spans, caret positions mapped through cluster indices, a typewriter effect that
reshapes its visible prefix so joining survives every step, an auto-size search over real
measured widths — is written once, reviewed once, and fixed once.

## Why Jisr is the only way to the native library

Boundary 2 of section 4.4: `jisr` is the only way in from outside Rust. There is no
second entry point and no engine-specific extension of it. In this tree that rule has a
mechanical form — **`Taarib.Unity.Jisr` is the only assembly in the solution that
declares a `DllImport` against the Taarib native library** — and four reasons behind it.

That qualifier is load-bearing, not hedging, because a reader who greps the solution for
the attribute finds more than Jisr and deserves to know which hits matter. The counts, as
built: `Taarib.Unity.Jisr` declares 33, of which 28 are the `taarib_*` entry points named
through a single library-name constant and 5 are the loader's own primitives —
`LoadLibrary`/`GetProcAddress` on `kernel32`, `dlopen`/`dlsym` on `libdl.so.2` and on
`/usr/lib/libSystem.B.dylib` — which is how it opens the file it then imports from.
`Taarib.Unity.Il2cpp` declares 16: 7 on `kernel32.dll`, 3 on `libc`
(`mmap`, `mprotect`, `munmap`), 3 `_dyld_*` on `/usr/lib/libSystem.B.dylib`, 2 on
`libSystem.dylib` (`sys_icache_invalidate`, `pthread_jit_write_protect_np`) and 1
`__clear_cache` on `libgcc_s.so.1`. Every one of those 16 is an operating-system
primitive `khatf` needs to make a page writable, publish an instruction rewrite and
invalidate the instruction cache before the CPU runs the old bytes again. None of them
reaches Taarib's ABI, and no glyph, layout, patch string or status code travels through
any of them.

`Taarib.Unity.Mono` and `Taarib.Unity.Mushtarak` declare none at all. The only match in
`Mushtarak` is the comment in `Ruqaa.cs` recording that it may not have one, which is the
rule holding rather than the rule being broken.

**The ABI has a version and a refusal.** `taarib_abi_isdar()` returns a major and a
minor. A caller whose major does not match must refuse to load and say so in the BepInEx
log in a sentence the user can act on. One loader means one place that check happens, and
no path around it. Two loaders means the second one eventually forgets.

**Ownership is stated once.** Every handle — `TaaribContext`, `TaaribKhatt`,
`TaaribLawha`, `TaaribLayout` — is an opaque, generation-tagged pointer with an explicit
create/destroy pair. Which side allocates, which side frees, how long a returned pointer
stays valid, and whether the next call invalidates it are documented per function in the
generated header and in `docs/abi.md`. Encoded once as `SafeHandle` subclasses, those
rules are enforced by the type system instead of remembered. A raw `IntPtr` passed around
by three assemblies is a use-after-free waiting for a slow frame.

**The hot path allocates nothing, and that is only true if there is one hot path.**
`taarib_takhtit` writes positioned glyphs into a caller-owned, reusable buffer; when
the buffer is too small it writes nothing and reports the capacity required, so the caller
grows once and retries. There is deliberately no allocating variant, precisely so no
adapter can reach for one by accident. Concentrating the P/Invoke surface in one assembly
is what makes "no managed allocation on the per-frame path" a property you can confirm by
reading a single project.

**Marshalling mistakes are silent.** A struct whose field order drifts from
`include/taarib.h`, a `bool` marshalled as four bytes instead of one, a UTF-8 string
handed over as UTF-16 — none of these fail loudly. They produce wrong glyphs, or a crash
in the game's own frame that the user will blame on the game. One assembly, laid out
field for field against a generated header, is a surface small enough to audit.

## Decision 5: the game's own fonts are never touched

No game font, no `TMP_FontAsset`, no SDF atlas, no `Font` asset, no `AssetBundle`, and no
`.assets` or `resources.assets` file is read, modified, extended or created anywhere in
this tree. Not to sample metrics, not to borrow a shader, not to add glyphs to an existing
atlas, not to look up a fallback. Taarib rasterizes its own glyphs from its own validated
bundled fonts, packs its own atlas, and builds its own meshes. What it asks of Unity is
only the set of primitives that has existed in every version of the engine and will exist
in the next one: make a texture, upload bytes into it, make a mesh, set vertices and UVs,
make a material with a shader, draw.

The reasoning, restated: **a serialized Unity asset is a snapshot of the exact editor
version that produced it.** Type trees, field ordering, container framing and compression
all move between minor versions. Every tool built on rewriting those assets therefore
inherits a maintenance obligation with no end — each engine bump is a new container format
to reverse, and the maintainers spend their remaining time chasing serialization instead
of improving the thing users care about. Refusing that dependency is what lets this
rendering path work on a Unity version that has not shipped yet, and on an obfuscated
build whose bundles cannot be opened at all.

There is a second reason that matters as much in practice. A patch that rewrites a game's
own assets has permanently altered the installation. Taarib writes into
`BepInEx/plugins/Taarib/` and nowhere else, so uninstalling is deleting a directory and
reverting the framework, verified against the original hashes rather than assumed
(Phase 15). Nothing of the game's is left different.

The one place Taarib reads game assets at all is text extraction (Phase 12). That is
read-only, happens in Studio while the game is not running, is never on the rendering
path, and degrades to runtime string capture through `iltiqat` when a container cannot be
parsed. It is not part of this tree.

## What is on disk at install time

`taarib-tathbeet` (Phase 15) installs BepInEx 6 into the game root with the correct
architecture and backend variant, then puts Taarib's own payload in one directory:

```
<game root>/
  BepInEx/
    core/                            BepInEx itself
    plugins/
      Taarib/
        Taarib.Unity.Jisr.dll
        Taarib.Unity.Mushtarak.dll
        Taarib.Unity.Mono.dll        exactly one of these two, never both
        Taarib.Unity.Il2cpp.dll
        jisr/                        the native library, per architecture
        khutut/                      the fonts the installed patch declares
        basmat/                      the IL2CPP signature database (IL2CPP only)
        <patch>.ruqaa                the installed patch container
```

**The `.pdb` files are built and not shipped.** `DebugType portable` produces one beside
every assembly, and a portable PDB is what turns a stack trace in the game's log into one
with a file and a line. Nothing installs them: `docs/tawzee.md` §3 has no row for them,
and `taarib-tajmee`'s `saf_bepinex` stages `{ism}.dll` and nothing else. Shipping them is
a distribution decision — four more files, roughly 180 KB, in every BepInEx component —
and it belongs to whoever owns that matrix, not to this document. Until it is made, a
crash inside Taarib in somebody's game reports method names without lines.

Three things about that layout are decided here rather than by Phase 15.

**Exactly one plugin assembly is present.** A game is Mono or IL2CPP, never both, and the
two BepInEx distributions are different builds with different hosts. Shipping the Mono
plugin into an IL2CPP install would produce a load failure in the BepInEx log and no
Arabic — so `tathbeet` selects one, following the backend that `taarib-muharrik`
identified in Phase 5.

**`Jisr.dll` and `Mushtarak.dll` sit beside the plugin, not in `BepInEx/core/`.** BepInEx
resolves a plugin's dependencies from the plugin's own directory, and `plugins/Taarib/`
being self-contained is what makes the uninstall a directory deletion.

**Nothing else is copied in.** `BepInEx.Core.dll`, `0Harmony.dll`, `Mono.Cecil.dll` and
Il2CppInterop are already loaded by the host; `UnityEngine.*` is already loaded by the
game. A duplicate beside the plugin is not a safety net — a second `0Harmony` is a second
Harmony instance patching the same methods, which is a class of failure that presents as
"text disappears in one menu". `Directory.Build.targets` enforces this with
`CopyLocalLockFileAssemblies=false`, and the package references that supply those
assemblies are all `PrivateAssets="all"`.

The native library, the fonts and the signature database are payload placement, which
belongs to Phase 15 and Phase 22; the shape above is what this solution's four assemblies
expect to find next to them.

## Which phase fills which project

| Phase | What it writes here |
| --- | --- |
| Phase 0 | these project files, the shared props and targets, this document |
| Phase 3 | `Taarib.Unity.Jisr` — named in Phase 3 as one of the four language wrappers over the ABI, written alongside the ABI it wraps so the header and the C# cannot drift |
| Phase 6 | `Taarib.Unity.Mushtarak` in full, and `Taarib.Unity.Mono` |
| Phase 7 | `Taarib.Unity.Il2cpp`, plus whatever `Mushtarak` needs generalized once a second backend is real |
| Phase 14 | nothing here, but it defines the `.ruqaa` container `Mushtarak` reads |
| Phase 22 | nothing here, but it publishes these assemblies into the per-platform payloads with their hashes in the bundle manifest |

`Mushtarak` is written during Phase 6 rather than before it, deliberately. The right
seam between shared logic and per-backend calls is not knowable in the abstract; it is
knowable after one backend works. Phase 7 then moves whatever turns out to be shared, and
its dependency list says so — Phase 7 depends on Phase 6, not merely on Phase 3.

## How BepInEx is pinned

**The version is exact, in three places, and nowhere else.**
`BepInEx.Unity.Mono` and `BepInEx.Unity.IL2CPP` are both `6.0.0-be.780`, and
`BepInEx.PluginInfoProps` is `2.1.0`. No range, no `6.0.0-*`, no floating suffix.

This matters more for BepInEx 6 than for a typical dependency. There is no stable 6.0.0
release: every usable build is a bleeding-edge snapshot, `be.NNN`, and the API moves
between them. A floating version would mean two machines producing different plugins from
the same commit, and would silently move HarmonyX and Il2CppInterop underneath the
adapters as well, since those arrive transitively. Phase 22 requires reproducible builds
and a bundle manifest listing every artifact's hash; a floating dependency makes that
manifest a description of one machine on one afternoon.

**Why not the newest build.** `be.788` is the head of the bleeding-edge line, and
`be.781` and up declare a dependency on `Samboy063.Cpp2IL.Core 2022.1.0-development.1452`,
a package published on neither nuget.org nor `nuget.bepinex.dev`. Restore silently
resolves a lower version and reports `NU1603`, which `TreatWarningsAsErrors` turns into a
build failure — the correct outcome, since a plugin built against a Cpp2IL the host does
not ship is a load failure moved from restore time to run time. `be.780` is the newest
build that resolves exactly, and the source compiles against it unchanged.

**What is staged is not what is compiled against, and that is a live defect.**
`assets/aqfal/qufl_bepinex.json` pins the redistributables Phase 15 installs: BepInEx
`5.4.23.5` for the Mono components and `6.0.0-pre.2` for the IL2CPP components. Neither is
`6.0.0-be.780`.

*Mono is the total failure.* `Taarib.Unity.Mono.dll` records `BepInEx.Core 6.0.0.0` and
`BepInEx.Unity.Mono 6.0.0.0` in its assembly references. The BepInEx 5.4.23.5 archive's
`BepInEx/core/` holds `BepInEx.dll`, `BepInEx.Preloader.dll`, `BepInEx.Harmony.dll`,
`0Harmony.dll`, Mono.Cecil and MonoMod — and neither of the two assemblies the plugin
names. The chainloader therefore cannot load the assembly at all, and `Awake` is never
entered.

The size of that gap was measured rather than guessed. Compiling
`Taarib.Unity.{Jisr,Mushtarak,Mono}` against the BepInEx 5.4.23.5 assemblies produces
**one** error, on the `using BepInEx.Unity.Mono;` in `Taarib.Unity.Mono/Taarib.cs`:
BepInEx 5 declares `BaseUnityPlugin` in the `BepInEx` root namespace, which that file
already imports, and every other member this adapter uses — `BepInPlugin`, `ConfigEntry`,
`ManualLogSource`, `Logger`, HarmonyX — is present in both lines under the same name. So
the two ways out are both small, and they are genuinely different decisions rather than
one obvious fix:

- point the Mono project at BepInEx 5, matching the lock file and the generation table in
  `taarib-tathbeet::tarkib`, at the cost of the argument above that one BepInEx line
  across both backends is what keeps the two adapters honest; or
- move the lock file's Mono entries to a BepInEx 6 Mono build, keeping one line
  everywhere, at the cost of shipping a bleeding-edge loader to the Unity 5.0–2021.3 Mono
  games the lock file says BepInEx 5 is the right loader for.

Nothing here picks one, because the choice belongs to whoever owns the lock file, and
picking it silently is how a repository ends up with two files that disagree.

*IL2CPP is the same major line and, as far as the compiler can see, compatible.* The
plugin records `BepInEx.Core 6.0.0.0`, `BepInEx.Unity.IL2CPP 6.0.0.0`, `0Harmony 2.10.2.0`
and `Il2CppInterop.Runtime 1.5.3.0`. The staged `6.0.0-pre.2` tree ships the first three at
exactly those versions and `Il2CppInterop.Runtime` at `1.4.6.0`. Compiling
`Taarib.Unity.{Jisr,Mushtarak,Il2cpp}` against the `pre.2` assemblies instead of the NuGet
ones succeeds with no source change, so every member this adapter names exists in `1.4.6`
with a compatible signature. That is a statement about the compile surface and nothing
more: no process has loaded either assembly.

**The feed is pinned too.** `RestoreSources` in `Directory.Build.props` names
`https://api.nuget.org/v3/index.json` and `https://nuget.bepinex.dev/v3/index.json`.
Setting `RestoreSources` replaces the source list a machine's `NuGet.Config` happens to
provide rather than adding to it, so restore does not depend on a developer's global
NuGet configuration — the second feed is not optional, because the BepInEx bleeding-edge
builds and the `UnityEngine.Modules` reference assemblies are not on nuget.org.

**Nothing from a package ships.** Every `PackageReference` in this solution carries
`PrivateAssets="all"`, and `UnityEngine.Modules` additionally carries
`IncludeAssets="compile"`. They exist to compile against. At runtime, BepInEx is already
in the process and Unity is the game's own.

**`UnityEngine.Modules` at `2021.3.0` is a ceiling, not a minimum.** It fixes the Unity
API surface the plugin is permitted to bind statically. The assemblies actually loaded are
whichever version the game shipped with, which may be much older or much newer, so
anything outside that conservative surface is reached through reflection resolved once at
startup and cached as a delegate — never on the render path. That is also why the IL2CPP
project has no `UnityEngine.Modules` reference: under IL2CPP the Unity types come from
Il2CppInterop's generated assemblies, produced from the specific game's metadata, and a
reference assembly would describe types that do not exist in that process.

## The build settings, and why each one is there

Everything below is in `Directory.Build.props` and applies to all four projects.

`LangVersion 12` and `Nullable enable`. A `null` from a game — a `TMP_Text` whose
`fontAsset` was never assigned, a `RectTransform` on a destroyed object — is the ordinary
case here, not the exceptional one. The nullable annotations are the record of which
returns from foreign code have been considered.

`ImplicitUsings disable`. Explicit usings only. This code is read by people debugging a
game process, often from a stack trace and a decompiler rather than from an open solution,
and the same simple type name lives in several of the namespaces in play at once — `Text`,
`Font`, `Material`, `Object`. An explicit using list at the top of a file states which
`Text` this file means. An invisible set of implicit imports does not.

`TreatWarningsAsErrors true`, with `WarningsNotAsErrors` deliberately empty. Nothing is
exempt. The failure modes in this tree are quiet: an unused result, an unreachable branch
after a signature change, a struct field the compiler notices is never assigned. In a
library, a warning is a note; in code injected into someone's game, a warning is the first
symptom.

`GenerateDocumentationFile true`, with `CS1591`, `CS1573` and `CS1712` in `NoWarn`. The
XML file exists for the tooltip a person reads mid-session with a debugger attached.
Requiring a tag on every public member, parameter and type parameter would fail the build
on generated sources that cannot carry any — `MyPluginInfo` from
`BepInEx.PluginInfoProps`, Il2CppInterop's proxies — and would push toward restating in C#
what `include/taarib.h` already documents authoritatively for the ABI. The warnings that
catch documentation which is *wrong* rather than absent stay as errors: `CS1570`
malformed XML, `CS1572` and `CS1734` a `param` tag naming a parameter that does not exist,
`CS1574` and its siblings for an unresolvable `cref`.

`AllowUnsafeBlocks true`. Mesh writing needs it. A frame's worth of glyphs goes from
`jisr`'s output buffer into the arrays a text component already owns through pinned
pointers, because the alternative — copying through managed intermediates — allocates on
the per-frame path, which Phase 6 forbids.

`Deterministic true`, and `ContinuousIntegrationBuild` set only when `CI`,
`GITHUB_ACTIONS` or `TF_BUILD` is set. Phase 22 requires that the bundle manifest's hash
for `Taarib.Unity.Mono.dll` be a fact about a commit rather than about a machine. The
property is conditional because it also normalizes source paths, which is right for a CI
checkout and wrong for a local build, where absolute paths are what make a debugger find
the source.

`DebugType portable`. Portable PDBs are readable by both Unity's Mono runtime and the
CoreCLR runtime BepInEx's IL2CPP host loads. Windows PDBs are readable by neither, which
is the difference between a stack trace in the game's log with file and line and one with
neither.

`Directory.Build.targets` holds the output hygiene, and is a `.targets` rather than a
`.props` on purpose: it is imported after each project has been evaluated, so a project
cannot re-enable any of it by accident. `CopyLocalLockFileAssemblies=false` keeps package
assemblies out of the plugin folder. `GenerateDependencyFile=false` and
`GenerateRuntimeConfigurationFiles=false` suppress a `.deps.json` and a
`.runtimeconfig.json` that no host in this arrangement reads.
`CopyDebugSymbolFilesFromPackages` and `CopyDocumentationFilesFromPackages` are off so
package `.pdb` and `.xml` files do not land next to the plugin.
`SatelliteResourceLanguages=en` stops a dozen localized resource subdirectories from
appearing inside a game install.

## Two honest constraints of the target frameworks

**`netstandard2.1`, not `netstandard2.0`.** `Span<T>`, `ReadOnlySpan<T>`,
`MemoryMarshal` and `ArrayPool<T>` are in the framework itself at 2.1, so the
zero-allocation layout path needs no `System.Memory` package. That is one less assembly
beside the plugin and, more importantly, no chance of a version conflict with a
`System.Memory` the game or another mod has already loaded — a conflict that surfaces as a
`TypeLoadException` from an assembly nobody in this repository referenced. Unity's Mono
runtime from the 2018.1 era onward implements .NET Standard 2.1.

**C# 12 syntax, on frameworks that predate some of it.** A few language features are
compiler features that require attributes the target framework has to declare. `init`
accessors need `IsExternalInit`; `required` members need `RequiredMemberAttribute` and
`SetsRequiredMembersAttribute`. .NET Standard 2.1 carries none of them, so those two
features are unavailable to the three `netstandard2.1` projects unless the project that
wants them declares the attribute itself. Pattern matching, `switch` expressions, target-
typed `new`, file-scoped namespaces, static lambdas and collection expressions over arrays
and spans all work as written.

`scoped` is the one to know about, because the takeover paths need it and it is in the
other category. `Ruqaa.HurufTakhtit` and `Ruqaa.SuturTakhtit` take a layout head by `in`
and return a span over the mapping, not over the argument; without `scoped` on the
parameter the C# 11 ref-safety rules clamp the returned span's lifetime to the caller's
local and refuse every call site with `CS8168`/`CS8347`. `ScopedRefAttribute` and
`RefSafetyRulesAttribute` are not in .NET Standard 2.1 either — but unlike `IsExternalInit`
the compiler emits them into the assembly itself when the framework does not carry them,
so `scoped` needs no declaration of its own and works on all four projects.

`net6.0` for `Taarib.Unity.Il2cpp` is not a choice this repository makes. BepInEx's
IL2CPP host loads a CoreCLR runtime it ships itself — there is no Mono in an IL2CPP game
to load into — and .NET 6 is what that host provides. `CheckEolTargetFramework` is off in
that project alone, because an end-of-support notice about a framework chosen by the host
process is not something any code here can act on.
