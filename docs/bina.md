# البناء — building Taarib from source

Everything here has been read out of this tree. Where a step has not been
automated, this file says so rather than describing a script that does not
exist.

Two things now exist that this file was written without: `scripts/isdar.sh`,
which runs the whole release sequence in dependency order, and
`.github/workflows/`, which runs the gates on Linux and Windows and the release
sequence on dispatch. The sections below are still the sequence, and they are
still worth reading — the script's refusals only make sense if you know what
each step produces — but for an actual release build, run the script.

---

## 1. Prerequisites

| what | version | why |
| --- | --- | --- |
| Rust | pinned by `rust-toolchain.toml` — `1.95.0` | `rustup` installs it automatically on first `cargo` invocation in the tree |
| Node.js | recent LTS | the Studio frontend |
| .NET SDK | 8 | the Unity plugins under `unity/` |
| Python | 3 | `scripts/*.py`, standard library only |

The toolchain file also requests the components `rustfmt`, `clippy`, `rust-src`
and `llvm-tools`, and ten targets — the three Windows MSVC triples, three Linux
GNU triples, both Darwin triples, and `wasm32-unknown-unknown`. `rustup` pulls
all of them, so the first invocation is slow and every later one is not.

The pin is `1.95.0` because that is the highest MSRV in the resolved dependency
graph (`sysinfo` requires it). Edition 2024 only needs 1.85, so the dependency
is the binding constraint, not the edition.

### Node package manager

The lockfile in `apps/studio/` is `pnpm-lock.yaml` (lockfile version 9), but
`tauri.conf.json`'s `beforeDevCommand` and `beforeBuildCommand` both say `npm`,
and there is no `packageManager` field in `package.json`. That inconsistency is
real and unresolved in the tree.

In practice: install with `pnpm install` to honour the lockfile, and leave the
`npm run …` hooks alone — `npm run build` will happily run the `build` script
against a `node_modules/` that pnpm populated. If you install with `npm`, you
get a resolution that the lockfile did not describe.

---

## 2. The Rust workspace

Twenty-nine members: the twenty-eight `crates/taarib-*` plus
`apps/studio/src-tauri`. `vendor/retour` is excluded from the workspace and
patched in over crates.io.

```bash
git clone https://github.com/cc1a2b/taarib
cd taarib

cargo build --workspace --release
```

Two things about that build worth knowing before it fails on you:

**`apps/studio/src-tauri/mawarid/` must exist.** `tauri.conf.json` declares
`bundle.resources = ["mawarid"]`, and `tauri-build` checks that the declared
resource path exists on *every* build — including a plain `cargo build` that
will never produce a bundle. The directory is committed containing only a
`README.md` for exactly this reason. Delete it and nothing in the Studio
compiles.

**The lints are strict on purpose.** `cargo clippy --workspace --all-targets`
denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
`unreachable`, `indexing_slicing`, `integer_division`, `float_cmp`,
`undocumented_unsafe_blocks`, `missing_panics_doc`, `missing_errors_doc`,
`allow_attributes_without_reason` and the three lossy-cast lints, on top of
`pedantic` and `nursery` as warnings. `missing_docs` and `unreachable_pub` are
denied at the `rustc` level. `clippy.toml` additionally disallows
`std::env::var`, `std::process::exit`, and `std::sync::{Mutex, RwLock}` — the
last two because the workspace uses `parking_lot`.

Every `#[expect]` needs a `reason`. This is not negotiable and it is not
adjustable per crate.

### Verification

```bash
cargo clippy --workspace --all-targets
cargo test --workspace
cargo deny check
```

`cargo fmt --all -- --check` is deliberately **not** in that list. The tree is
several thousand hunks from what stable rustfmt would produce — most of it
rustfmt wanting to explode compact struct literals, method chains and
`let … else` bodies that are written on one line here — so running it as a gate
would fail on a clean checkout and teach everybody to ignore it. CI carries it
as a non-blocking job for the same reason. It becomes a gate on the day a
tree-wide format lands, and not before.

`deny.toml` is doing real work, not licence bookkeeping. Two copies of
`read-fonts` in the graph is a build failure, and a native HarfBuzz or
`rustybuzz` entering the graph is a build failure — because Decision 3 (the
shaper and the rasterizer read the same font parser) is only true as long as
that stays true. `[graph] targets` lists the same ten triples as the toolchain
file.

### Seeing the text engine work on its own

`taarib-saff` has no dependency on anything else in the workspace and carries a
runnable example:

```bash
cargo run -p taarib-saff --example nazra
```

It shapes Arabic through the real layout pipeline, prints the glyphs it
produced, and writes a PNG.

---

## 3. The desktop application

```bash
cd apps/studio
pnpm install
npm run tauri build
```

`npm run build` — which the Tauri build hook invokes — is
`tsc -p tsconfig.json && tsc -p tsconfig.node.json && vite build`, and it writes
to `apps/studio/dist/`, which is what `frontendDist: "../dist"` points at.

`bundle.targets` is `["deb", "appimage", "nsis", "app", "dmg"]`. The bundler
produces only the formats valid for the host it runs on. `msi` and `rpm` are
deliberately absent: `docs/tawzee.md` §2 requires every shipped format to have
an owner and a tested update path, and those two would have neither.

### Regenerating the generated code

The TypeScript bindings under `apps/studio/src/mustalahat/` are emitted from the
Rust types by the Studio binary in debug builds (`cd apps/studio && npm run
tauri dev`). They are never hand-edited.

The JSON Schemas under `schemas/` come from the same vocabulary crate:

```bash
cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat
```

The `mukhattatat` feature is `required-features` on that binary, so leaving it
off is a cargo error rather than a silent no-op. The binary takes no arguments;
it resolves the workspace root from `CARGO_MANIFEST_DIR` and writes `schemas/`.

---

## 4. The game-side payloads, and the flag you must not forget

This is the single most consequential detail in the whole build.

Three crates compile to a shared library that a game loads into its own process:
`taarib-tabaqa` (the overlay), `taarib-muhawwil-unreal`, and
`taarib-muhawwil-godot`. Each of them exports the bootstrap entry point
`taarib_bidaya`, which `taarib-mudkhal` resolves and calls once the module is
mapped. The contract for that function is [`bidaya.md`](bidaya.md).

That export is behind a cargo feature: **`hamula`**, and it is **off by
default**.

```bash
cargo build --release --target x86_64-pc-windows-msvc \
    -p taarib-tabaqa -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \
    -p taarib-mudkhal -p taarib-jisr --features hamula
```

### Why it is off by default

All three payloads export the *same* symbol name, and `taarib-studio` links all
three as `rlib`s. An always-on export is therefore a duplicate-symbol link
failure on the desktop binary. The feature is enabled by no manifest anywhere;
the payload build passes it explicitly, and only the `cdylib` gets it.

### What forgetting it produces

A perfectly well-formed shared library. It compiles, it links, it installs, the
installer reports success, the game loads it — and then nothing happens, because
`taarib-mudkhal` looks for `taarib_bidaya`, does not find it, and moves on. The
player launches the game and it is in English. Nothing anywhere reports an
error.

That is precisely the failure mode `bidaya.md` was written to close ("the module
loaded and returned" is named there as the exact thing to avoid), and it is why
the staging tool refuses such a build rather than trusting the operator.

### How the refusal works

`crates/taarib-tajmee` stages rows F1, G1 and G2 with the require-bootstrap flag
set. `nasakh::yusaddir_bidaya` scans the compiled image for the literal bytes
`taarib_bidaya`:

```rust
pub(crate) fn yusaddir_bidaya(bayt: &[u8]) -> bool {
    const ISM: &[u8] = b"taarib_bidaya";
    bayt.windows(ISM.len()).any(|nafidha| nafidha == ISM)
}
```

A byte scan rather than a format parse, deliberately: the staging tool runs on
one host and stages payloads for three, so it cannot rely on being able to parse
a PE, an ELF and a Mach-O. An exported symbol's name is a NUL-terminated string
in the image's own string table under all three, and its presence is exactly the
question. A payload built without `hamula` contains the name nowhere.

When it is missing, staging refuses, by name and by path:

```
[F1] target/x86_64-pc-windows-msvc/release/taarib_tabaqa.dll exports no taarib_bidaya; rebuild with --features hamula
```

The manifest is not written, and the exit status is non-zero. The failure lands
on the build machine instead of on a user whose install reported success.

---

## 5. Staging a release tree

`crates/taarib-tajmee` gathers every already-built artifact into
`apps/studio/src-tauri/mawarid/`, hashes each one, and writes
`bayan_mukawwinat.json` last. **It builds nothing**, and the consequence of that
was not theoretical: a bundle assembled by hand shipped with the manifest
absent, and every framework component then refused by name — the product
started, scanned, found games, and could not patch one. `scripts/isdar.sh` runs
the builds in dependency order and then this tool, which is why it exists.
Everything below runs first,
and the tool's whole contribution is that a missing artifact is named here, on a
build machine, rather than discovered by a user whose install silently did
nothing.

```
taarib-tajmee --hadaf <target-triple> [--jidhr <workspace>] [--ahdaf <target dir>]
              [--kharij <out dir>] [--jalb]

    --hadaf    one of the targets in docs/tawzee.md §2 (required)
    --jidhr    workspace root (default: .)
    --ahdaf    cargo target directory (default: <jidhr>/target)
    --kharij   staging root (default: <jidhr>/apps/studio/src-tauri/mawarid)
    --jalb     permit fetching locked artifacts that are not cached yet
```

`--hadaf` accepts `x86_64-pc-windows-msvc`, `x86_64-unknown-linux-gnu`,
`aarch64-apple-darwin` or `x86_64-apple-darwin`; anything else is refused as an
unknown target. `-h`/`--help` is the only short form on the whole tool.

The prerequisites, from the tool's own help text:

```bash
cargo build --release --target <triple>            # the studio and its cdylibs
cargo build --release --target x86_64-pc-windows-msvc \
    -p taarib-tabaqa -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \
    -p taarib-mudkhal -p taarib-jisr --features hamula   # game-side payloads
dotnet build unity/Taarib.Unity.sln -c Release
wasm-pack/wasm-bindgen --target no-modules --out-name taarib_core \
    --out-dir target/wasm-bindgen
esbuild adapters-script/rpgmaker/taarib.ts --bundle \
    --outfile=target/adapters/rpgmaker/taarib.js
esbuild adapters-script/electron/taarib.ts --bundle \
    --outfile=target/adapters/electron/taarib.js
```

Note the second line: the Windows payloads are built on **every** host. A Linux
or macOS client installing into a Wine or Proton game deploys the Windows-side
payloads, so every desktop bundle carries the Windows game-side set.

The tool collects *every* absence and prints them together before exiting
non-zero — an operator who has four things to rebuild learns that once, not four
times. Fetched inputs (the BepInEx redistributables, the bundled fonts) are
pinned in `assets/aqfal/` as `{ rabt, isdar, hajm, sha256 }` and are refused if
unlocked or mismatched; the tool never writes a hash it did not verify.

The full artifact matrix — twenty rows, source and staged destination for each —
is `docs/tawzee.md` §3.

---

## 6. Building the Windows bundle from a Linux checkout

There is no script for this in the tree. What follows is the procedure that
actually produces the Windows `.exe`, including the three things that go wrong
if you improvise.

The shape of the problem: the Rust side cross-compiles cleanly to
`x86_64-pc-windows-msvc`, but the *bundler* has to run on Windows to produce an
NSIS installer, and the frontend's `node_modules/` were installed on the Linux
side of the same checkout.

**Step 1 — build the frontend on the Linux side.** The node modules live where
they were installed, and a Windows `npm run build` against a Linux-populated
`node_modules/` fails on the native binaries inside it. So build `dist/` first,
where the modules work:

```bash
cd apps/studio
pnpm install          # if you have not already
npm run build         # tsc, tsc, vite build -> apps/studio/dist
```

`apps/studio/dist/` is gitignored — it is build output, and it is what
`frontendDist: "../dist"` resolves to.

**Step 2 — install the Tauri CLI on the Windows side.** The bundler runs
natively. The Linux checkout's `node_modules/@tauri-apps/cli` is the Linux
build; it cannot drive an NSIS bundle.

**Step 3 — stop Tauri from rebuilding the frontend.** `tauri.conf.json` sets
`beforeBuildCommand: "npm run build"`. On the Windows side that re-runs the
build you just did on Linux, in the environment where it cannot work. Override
it to nothing.

**Step 4 — pass the override as a *file*, never as inline JSON.** `tauri build`
accepts `-c` / `--config` as either "JSON strings or paths to JSON, JSON5 or TOML
files". Inline JSON is the obvious choice and it is the trap: `cmd.exe` eats the
quoting, and what reaches the CLI is a mangled string that either fails to parse
or — worse — parses into something other than what you wrote. Put the override
in a file and pass the path.

```json
{
  "build": {
    "beforeBuildCommand": ""
  }
}
```

```
tauri build --config <path-to-that-file> --target x86_64-pc-windows-msvc
```

The result lands in `target/x86_64-pc-windows-msvc/release/bundle/nsis/` as
`Taarib_<version>_x64-setup.exe`, per `docs/tawzee/windows.md`. That installer
is per-user (`RequestExecutionLevel user`, `installMode: currentUser`) and
requires no administrator rights.

`taarib-tajmee --hadaf x86_64-pc-windows-msvc --jalb` has to run between steps 1
and 4, so `mawarid/` is populated before the bundler copies it in as a resource.
This is not a release-only step. The bundler used to accept an empty `mawarid/`
without a word, and the installer it produced carried no fonts at all —
`taarib-saff` has no Arabic fallback shaper, so on a user's machine every
layout, preview and patch build failed after download, install and first launch
had each appeared to succeed.

`build.beforeBundleCommand` now runs `src-tauri/tadqiq_mawarid.mjs` between the
compile and the bundler. It verifies every font in `assets/aqfal/qufl_khutut.json`
against its locked size and SHA-256, then verifies every face
`assets/fonts/khutut.json` declares against the load-time rule in
`crates/taarib-saff/src/khatt.rs` — GSUB `init`/`medi`/`fina`/`rlig`, GPOS
`mark`, and the forty characters of `HURUF_MATLUBA`. A bundle that would ship
no font, or an unshapeable one, fails there and names each file. Components are
deliberately not fatal: an absent one is already named at startup and refused
by name at install time.

---

## 7. Signing identities

Two, and they are structurally distinct in `taarib_khatm::malik`.

**`tatwir` — the development identity.** Its public key `MIFTAH_TATWIR` is
committed. Its private half lives in a developer keychain and is unprotected by
design. A build carrying it shows a badge in the Studio footer and will install
dev-signed packages. Every `cargo build` you run produces one of these.

**`isdar` — the release identity.** Its public half is injected at build time
through `TAARIB_MIFTAH_ISDAR` (64 hex characters), and the `isdar` feature on
`taarib-khatm` gates the behaviour. The private half is generated inside the
owner's passphrase-protected OS keychain, on the owner's machine, and is never a
file, a fixture, or an environment variable.

A release build with a malformed injected key, or one equal to the development
key, does not compile. That is the point of the feature existing at all: without
it, a packaging run that forgot to inject `TAARIB_MIFTAH_ISDAR` would produce a
binary that silently trusts the committed development key. A release client
refuses a `MIFTAH_TATWIR` signature by name.

Provisioning the owner key is `cargo run -p taarib-khatm --bin malik`, which
reads a 64-hex-character seed from standard input, requires exactly that, stores
the key, and prints the public half. It takes no arguments.

---

## 8. Asset generation

Neither of these runs as part of a normal build; both are regeneration tools
whose outputs are committed.

**Fonts.** `python3 scripts/ijlib_khutut.py` stages the specific faces the
Studio webview embeds, fetching what is missing and verifying every file against
`assets/aqfal/qufl_khutut.json`, into `apps/studio/src/khutut`. It is standard
library only, by design, because it runs before the node install.

- `--tahaqquq` — verify what is on disk, touch no network, fail if anything is
  off.
- `--sakit` — report only failures.

**Icons.** `python3 scripts/irsim_ayqunat.py` rasterizes every icon Taarib ships
from the two vector marks in `assets/huwiya/` into the bundle PNG/ICO/ICNS set,
the freedesktop hicolor tree, the web favicon and the `.ruqaa` mimetype icons,
then verifies them. It takes no arguments.

---

## 9. What is not automated

Stated plainly, because a build document that implies otherwise wastes people's
afternoons:

- **CI exists but has never run.** `.github/workflows/ci.yml` runs the gates of
  §2 on Ubuntu 24.04 and Windows Server 2025 — `cargo check`, `clippy` and
  `test` on both, plus the two TypeScript projects, `vite build` and
  `cargo deny check` on Linux. It has never executed, because this workspace is
  not yet a git repository and has no remote. Two things about it are worth
  knowing before the first run: the Rust matrix depends on the frontend job and
  downloads its `dist` artifact, because `tauri-codegen` panics outright without
  `apps/studio/dist` and an empty one would embed an application with no
  interface; and `cargo fmt --all --check` is present but `continue-on-error`,
  because the tree is 5 358 hunks from formatted. `.github/workflows/`
  `macos-probe.yml` is dispatch-only and can never be a required check.
  Branch protection cannot be expressed in a file: after publication somebody
  must mark the four real jobs required and must not mark the formatting one.
- **No cross-compilation configuration.** `ROADMAP.md` §22 names `cargo-xwin`
  and `cross` as the intended technology; no such configuration file exists yet.
  Section 6 above is what is done instead.
- ~~**No release script.**~~ `scripts/isdar.sh --hadaf <target-triple> [--jalb]`
  now runs the sequence of sections 4, 5 and 6 in dependency order — `dotnet`
  for the four Unity assemblies, `cargo` for the game-side cdylibs per platform,
  `wasm-bindgen` for the wasm pair, `node adapters-script/ibni.mjs` for the two
  script adapters, then `taarib-tajmee` — and `.github/workflows/isdar.yml` is
  the same sequence on a runner. Run one of those rather than the sections by
  hand. The sections remain, because knowing what each step produces is what
  lets you read the staging tool's refusals.
- **No macOS signing or notarization.** `signingIdentity` is deliberately unset
  in `tauri.conf.json` until an account exists. See `docs/tawzee/macos.md` §3
  for what that means for users, and for the `xattr -dr com.apple.quarantine`
  workaround it documents honestly rather than hiding.
