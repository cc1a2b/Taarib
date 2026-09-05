# البداية — the payload bootstrap contract

Phase 23 Part B. `taarib-mudkhal` loads every payload beside it and calls
`taarib_bidaya` on the ones that export it. This file is what that function is,
and the three payloads implement exactly it.

Before it, every native path installed successfully, reported success, and ran the
game in English with nothing saying so. That is the defect this contract closes:
"the module loaded and returned" is the exact failure mode it exists to prevent.
It closed it — §6 records where each path now stops, by name, in a log. Closing it
did not make any of them draw.

## 1. The symbol

```rust
#[unsafe(no_mangle)]
pub extern "C" fn taarib_bidaya()
```

No arguments, no return value, `extern "C"` on Unix and `extern "system"` on
Windows — the loader transmutes to `extern "system" fn()` on Windows and
`extern "C" fn()` elsewhere, and on every target in the matrix those are the
same convention. It is called exactly once, from the loader's own thread, after
the module is mapped and before the game has finished starting.

**It must never panic and must never unwind across the boundary.** Every
implementation wraps its body so that a failure becomes a logged refusal and a
return, not an abort inside somebody's game. Use
`std::panic::catch_unwind(AssertUnwindSafe(...))` at the top level.

## 2. The order, and what each step means

1. **Where am I.** `taarib_haqn::mawqi::mujallad_nafsi()` — the directory this
   payload was loaded from, which for a split component is the game's own
   `taarib/` directory. Never `current_exe`, never the working directory:
   both belong to the game or its launcher.
2. **What am I inside.** `taarib_haqn::mawqi::qaidat_wahda(<module>)` for the
   engine module this payload exists to take over. Absent means "this is not
   that kind of game" — a clean, logged decline, not an error.
3. **The patch.** `<own dir>/*.ruqaa`, which is where
   `taarib_tathbeet::masar_tathbeet` writes it. Read through
   `taarib_ruqaa::qari`. No patch means nothing to do: log and return.
4. **The disclosure**, tier 3 only. `taarib_tabaqa::sidq::Iqrar::baad_ard`
   takes the fingerprint of the text that was displayed, so a caller that wants
   to fabricate consent has to display the disclosure to obtain the fingerprint
   — at which point it is not fabricated. The bootstrap **must not route around
   this**: it reads the persisted acknowledgement the studio wrote, checks it
   with `sidq::mahfuz_salih`, and declines when it is absent or stale.
5. **Resolve what the adapter needs**, through `taarib-haqn` — `Masar` for a
   detour at an address, `KhatfJadwal` for a vtable slot, `ikhtif_istirad` for
   an import. Nothing else writes to game memory.
6. **Hand over** to the adapter's own initialisation, and return.

## 3. Refusal discipline

Every step that can decline does so by name, in one line, to
`<own dir>/taarib.sijill`. A payload inside a game has no console, no window and
no channel to Studio, so the log file is the only surface it has, and it is
capped so a game launched a thousand times does not fill a disk.

The states are distinct and must not be collapsed:

| state | meaning | what happens |
| --- | --- | --- |
| declined | this is not that kind of game, or no patch is installed | log one line, return, game untouched |
| refused | something is wrong that the user could fix | log the reason, return, game untouched |
| failed | a hook or a read failed unexpectedly | log the reason, undo whatever was installed, return |
| started | the adapter has control | log it, return |

A half-initialised payload is worse than one that declines: anything installed
before a failure is removed before returning, which the `Drop` on `Masar` and
`KhatfJadwal` already guarantees as long as the bootstrap holds them in a value
that is dropped on the failure path.

## 4. The three implementations

| payload | module to find | what it hands over to |
| --- | --- | --- |
| `taarib-tabaqa` | the graphics API in use: `d3d11.dll`, `d3d12.dll`, `opengl32.dll`/`libGL.so.1` | `wajiha::Tabaqa::shaghghil(Box<dyn Khattaf>, Iqrar)` with the backend for whichever API answered |
| `taarib-muhawwil-unreal` | the game's own executable module (Unreal links the engine in) | `tashghil` / `wasl`'s initialisation |
| `taarib-muhawwil-godot` | Godot 3 loads the module through `taarib.gdnlib`, so this payload's own GDNative entry is the bootstrap | `istila`'s takeover |

Vulkan is **not** in this table: the Vulkan loader calls
`vkNegotiateLoaderLayerInterfaceVersion` in `taarib-tabaqa` directly, so that path
needs no bootstrap of its own and the bootstrap must not try to initialise Vulkan a
second way. That is a statement about *entry*, and only about entry — the layer is
live and it draws nothing. See §6.

Unity Mono and Unity IL2CPP are **not** in this table either: BepInEx loads the
C# plugin and calls its `Load()`, which is that path's bootstrap and already
exists. The same is true of the script engines, whose adapters are registered
by the game's own script runtime. `taarib_bidaya` is for the three native
payloads `taarib-mudkhal` opens, and for nothing else.

## 5. File ownership

| file | owner |
| --- | --- |
| `crates/taarib-tabaqa/src/bidaya.rs` | agent B1 |
| `crates/taarib-muhawwil-unreal/src/bidaya.rs` | agent B2 |
| `crates/taarib-muhawwil-godot/src/bidaya.rs` | agent B3 |
| each crate's `lib.rs` (`pub mod bidaya;`), each `Cargo.toml` | convergence |
| `crates/taarib-haqn/**` | convergence (Part A, complete) |


## 6. What the bootstraps reach, and where each stops

Recorded after implementation and re-read against the tree on 2026-09-05. Every
line here is a fact about this tree, not a plan. `docs/tashghil.md` is the
engine-by-engine version of the same reading, with the line numbers; where the two
disagree, `tashghil.md` and `imkaniyat::jahiziya` are the ones that were checked
most recently.

| path | reaches | stops at |
| --- | --- | --- |
| Vulkan overlay | `vkQueuePresentKHR`, through the loader's own layer chain | the draw callback, which is never registered: `sajjil_munadi` (`crates/taarib-tabaqa/src/vulkan.rs:786`) has no caller, so the present handler reads `None` at `vulkan.rs:1518` and forwards the application's present info byte for byte. `KhattafVulkan` is never constructed |
| D3D11 / D3D12 / OpenGL overlay | `Tabaqa::shaghghil` from the first-present hook, through `taarib-haqn`'s vtable hooker, with a complete pipeline built around the game's own device | first the persisted disclosure (`bidaya.rs:366`), which declines by name rather than constructing an `Iqrar` any other way. Past it, `INTAHAT` is set on the successful start (`bidaya.rs:790`) and every later present returns at the guard in `shaghghil_min_itar` (`:764`) — and nothing would be drawn anyway, because `Tabaqa::iltaqit` (`src/wajiha.rs:891`) has no caller and no worker produces lines |
| Unreal | `Tashghil::shaghghil` — the adapter's own initialisation, with the engine's `Engine.ini` written for the next launch | `qiyas::AhdafQiyas` and `wasl::FaharisAwamir` are *supplied*, not discovered: `slate.rs` refuses a byte-pattern database and `wasl.rs` refuses a guessed vtable slot. Nothing in the product supplies them, so `rutbat_haqn` returns at `src/tashghil.rs:916` and `TasheehQiyas::rakkib` (`src/qiyas.rs:978`) is never called. The `Engine.ini` write is behind `--features hamula`, which only `scripts/isdar.sh` turns on |
| Godot 3 | `taarib_gdnative_init` / `_singleton` bind the core API and open the patch | **both** halves, not one. `thabbit_tawseel` (`crates/taarib-muhawwil-godot/src/bidaya.rs:805`) delivers the translated text and `thabbit_istila` (`:780`) delivers three `Font::` addresses and a `VisualServer` slot that `istila.rs` refuses to discover. Neither has a caller, so `awsil` refuses at `:1095` and `sallim` takes its `Mumtania` branch at `:1197`. The patch companion that would call either does not exist |
| Godot 3 via preload | — | declines by name, having verified its own `taarib_gdnative_init` export is really there. Correct: a Godot game is taken over through the gdnlib |
| Unity Mono / IL2CPP | BepInEx calls the C# plugin's `Load()` | not in this contract — that bootstrap already existed. Mono stops earlier than its own bootstrap: the plugin asks for BepInEx 6 and the bundle pins 5.4.23.5 for that backend, so the chainloader never reaches `Awake` |
| script engines | the game's own script runtime registers the adapter | not in this contract. Their *text* no longer depends on it: `taarib_muhawwil_nusus::tarkeeb::rakkib_luba` writes the Arabic into the game's own data at install time, from `taarib_tathbeet::nusus::raqqi_nusus` |

**The honest summary.** Every payload has an entry point, every entry point reaches
a named outcome, and no path "loads and returns" in silence. Not one of them draws
a glyph. Three stop at a seam that needs per-build verified addresses or a
configuration call the product does not make — which the adapter crates deliberately
refuse to guess — and the overlay additionally has no text source at all. That is a
capability gap with a named place to be filled, not a lie about what runs.

The one claim this section used to make that was simply wrong is worth naming, since
it was repeated elsewhere: **the Vulkan path has never drawn anything**. The layer is
live and correctly registered, which is a real and unusual achievement and is not the
same thing. §4 above says the loader calls
`vkNegotiateLoaderLayerInterfaceVersion` directly, and that remains true; "which is
why that one path has worked all along" was reading a live layer as a working
overlay.
