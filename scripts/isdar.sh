#!/usr/bin/env bash
# إصدار تعريب — build one complete, installable bundle from a clean checkout.
#
# `crates/taarib-tajmee` stages the component tree and builds nothing. Everything
# it stages is built by a different tool — cargo for five cdylibs per game
# platform, `dotnet` for four Unity assemblies, `wasm-bindgen` for the wasm pair,
# `tsc`/`esbuild` for the two script adapters — and until this file existed there
# was no one place that ran them. The consequence was not theoretical: a bundle
# built by hand shipped with `bayan_mukawwinat.json` absent, which is the marker
# for a partial staging, and every framework component then refused by name. The
# product started, scanned, found games, and could not patch one.
#
#   scripts/isdar.sh --hadaf x86_64-pc-windows-msvc [--jalb]
#
# Stages, in dependency order:
#
#   unity   rows C1–C4   dotnet build      the BepInEx-side assemblies
#   hamula  rows B,E,F,G cargo build       the game-side cdylibs, per platform
#   wasm    row  H1      cargo + wasm-bindgen
#   mulhaq  rows I1,I2   node adapters-script/ibni.mjs
#   tajmee  rows D1,J1,K1,M1,N1            stage, verify, write the manifest
#   wajiha  row  A2      the frontend
#   huzma   rows A1,A3   tauri build       the installer
#
# Any stage may be skipped with TAARIB_TAKHATTI=unity,wasm — which is safe by
# construction, because `tajmee` refuses to write a manifest over a tree that is
# missing anything, whatever the reason it is missing.
#
# The order of `hamula`, `tajmee` and `huzma` is not a preference. Three payload
# crates are built twice from one target directory: once as `hamula` cdylibs for
# a game, and once as plain rlibs for the studio, which links all three. The two
# builds differ only in a feature, so they share `…/release/taarib_tabaqa.dll`
# and the later one overwrites it. Staging between them is what makes the bytes
# in the bundle the ones that export `taarib_bidaya`.

set -o errexit
set -o nounset
set -o pipefail

JIDHR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

# ---------------------------------------------------------------------------
# Options
# ---------------------------------------------------------------------------

hadaf=""
jalb=0
# The staging tool reads cargo's outputs from here. It stays relative so that a
# Windows cargo invoked from WSL and a WSL cargo agree on one directory.
ahdaf="target"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --hadaf) hadaf="${2-}"; shift 2 ;;
    --ahdaf) ahdaf="${2-}"; shift 2 ;;
    --jalb) jalb=1; shift ;;
    -h|--help) sed -n '2,27p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) printf 'isdar: unknown argument %s\n' "$1" >&2; exit 2 ;;
  esac
done

if [ -z "$hadaf" ]; then
  printf 'isdar: --hadaf is required; see --help\n' >&2
  exit 2
fi

# The cargo that builds the host-side binary and the wasm core.
QARGO="${TAARIB_CARGO:-cargo}"
# The cargo that builds the game-side payloads. Separate because the two are not
# always the same program: on a WSL host there is no MSVC linker, so the
# `*-pc-windows-msvc` payloads are built by the Windows cargo across /mnt while
# the studio and the wasm core are built by the Linux one.
QARGO_HAMULA="${TAARIB_CARGO_HAMULA:-$QARGO}"
TAKHATTI="${TAARIB_TAKHATTI:-}"

cd "$JIDHR"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

marhala() { printf '\n\033[1m== %s ==\033[0m\n' "$1"; }
tuhmal() { case ",${TAKHATTI}," in *",$1,"*) return 0 ;; *) return 1 ;; esac; }

lazim() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'isdar: %s is not on PATH; %s\n' "$1" "$2" >&2
    exit 1
  fi
}

# The game-side payload platforms this bundle carries, mirroring
# `hadaf::hamulat_alalaab`: windows always, because a Linux or macOS client
# installing into a Wine or Proton game deploys the windows payloads, plus the
# host's own platform when it is not windows.
#
# TAARIB_HAMULA_MUTHALLATHAT overrides the list, and every release that is not
# a Windows one has to use it: the `*-pc-windows-msvc` payloads need `link.exe`,
# which exists on no Linux or macOS runner, so those two builds take the Windows
# halves from a Windows job's artifact and build only their own. Whatever is
# neither built nor supplied is caught by `tajmee`, by name, before a manifest
# exists — so an override that is wrong fails the release rather than shipping
# a bundle with a hole in it.
muthallathat_hamula() {
  if [ -n "${TAARIB_HAMULA_MUTHALLATHAT:-}" ]; then
    printf '%s\n' "${TAARIB_HAMULA_MUTHALLATHAT//,/$'\n'}"
    return
  fi
  printf '%s\n' x86_64-pc-windows-msvc i686-pc-windows-msvc
  case "$hadaf" in
    *-pc-windows-msvc) ;;
    *) printf '%s\n' "$hadaf" ;;
  esac
}

# ---------------------------------------------------------------------------
# Row H1's two halves have to agree exactly
# ---------------------------------------------------------------------------

# The `wasm-bindgen` CLI and the `wasm-bindgen` crate are one program split in
# two, and they refuse each other's output across versions — so the version is
# read out of Cargo.lock rather than written here, where it would be a second
# number to keep in step.
isdar_wasm_bindgen() {
  awk '
    /^name = "wasm-bindgen"$/ { wajad = 1; next }
    wajad && /^version = / { gsub(/[",]/, "", $3); print $3; exit }
  ' Cargo.lock
}

# ---------------------------------------------------------------------------
# Stages
# ---------------------------------------------------------------------------

if ! tuhmal unity; then
  marhala "unity — rows C1-C4"
  lazim dotnet "the four BepInEx-side assemblies are built with it"
  dotnet build unity/Taarib.Unity.sln -c Release --nologo
fi

if ! tuhmal hamula; then
  marhala "hamula — rows B1-B3, E1-E3, F1, G1, G2"
  # `--features` is package-qualified because `-p` selects five packages and
  # only three of them have a `hamula` feature: `taarib-jisr` and
  # `taarib-mudkhal` export their entry points unconditionally, and a bare
  # `--features hamula` would be rejected for naming a feature they lack.
  #
  # The feature itself is not optional. `tajmee` refuses to stage an F/G payload
  # that exports no `taarib_bidaya`, because such a payload is a well-formed
  # library a game loads and that then does nothing.
  while read -r muthallath; do
    printf -- '-- %s\n' "$muthallath"
    "$QARGO_HAMULA" build --release --target "$muthallath" \
      -p taarib-jisr -p taarib-mudkhal -p taarib-tabaqa \
      -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \
      --features taarib-tabaqa/hamula,taarib-muhawwil-unreal/hamula,taarib-muhawwil-godot/hamula \
      --target-dir "$ahdaf"
  done < <(muthallathat_hamula)
fi

if ! tuhmal wasm; then
  marhala "wasm — row H1"
  matlub="$(isdar_wasm_bindgen)"
  if [ -z "$matlub" ]; then
    printf 'isdar: no wasm-bindgen version in Cargo.lock\n' >&2
    exit 1
  fi
  lazim wasm-bindgen "install it with: cargo install wasm-bindgen-cli --version ${matlub} --locked"
  hali="$(wasm-bindgen --version | awk '{ print $2 }')"
  if [ "$hali" != "$matlub" ]; then
    printf 'isdar: wasm-bindgen %s is installed but Cargo.lock pins the crate at %s.\n' \
      "$hali" "$matlub" >&2
    printf '       The CLI and the crate are one program in two halves and refuse\n' >&2
    printf '       each other across versions. Install the matching one with:\n' >&2
    printf '           cargo install wasm-bindgen-cli --version %s --locked\n' "$matlub" >&2
    exit 1
  fi
  # Its own target directory, not "$ahdaf": on a WSL host "$ahdaf" is written by
  # the Windows cargo, and a wasm artifact built by the Linux one has no
  # business landing in the same tree with the same fingerprint files.
  "$QARGO" build --release --target wasm32-unknown-unknown -p taarib-wasm \
    --target-dir target/wasm-cargo
  # `--target no-modules` and `--out-name taarib_core` are both load-bearing:
  # the RPG Maker adapter reads the global `wasm_bindgen` that no-modules
  # defines, and both adapters load exactly these two file names.
  wasm-bindgen --target no-modules --out-name taarib_core \
    --out-dir target/wasm-bindgen \
    target/wasm-cargo/wasm32-unknown-unknown/release/taarib_wasm.wasm
fi

if ! tuhmal mulhaq; then
  marhala "mulhaq — rows I1, I2"
  lazim node "the two script adapters are compiled with it"
  lazim pnpm "adapters-script pins its compilers in a lockfile"
  ( cd adapters-script && pnpm install --frozen-lockfile )
  # Always <jidhr>/target: `masfufa.rs` resolves rows H1, I1 and I2 relative to
  # the workspace root rather than to --ahdaf, because none of the three is a
  # cargo output and the cargo target directory is not theirs to live in.
  node adapters-script/ibni.mjs --ahdaf "$JIDHR/target"
fi

if ! tuhmal tajmee; then
  marhala "tajmee — rows D1, J1, K1, M1, N1, and the manifest"
  lazim curl "the pinned BepInEx and font downloads are fetched with it"
  wusata=(--hadaf "$hadaf" --jidhr "$JIDHR" --ahdaf "$ahdaf")
  if [ "$jalb" -eq 1 ]; then wusata+=(--jalb); fi
  # Through `cargo run` rather than a path into the target directory: this
  # workspace's target directory can be redirected by a user-level cargo
  # config, and cargo knows where it put the binary.
  #
  # `--ahdaf` stays as given, which is where the payload builds above wrote.
  # Rows H1, I1 and I2 are read from <jidhr>/target whatever this says, because
  # none of the three is a cargo output.
  "$QARGO" run --release -q -p taarib-tajmee -- "${wusata[@]}"
fi

if ! tuhmal wajiha; then
  marhala "wajiha — row A2"
  lazim pnpm "the studio frontend is built with it"
  ( cd apps/studio && pnpm install --frozen-lockfile && pnpm build )
fi

if ! tuhmal huzma; then
  marhala "huzma — rows A1, A3, and the installer"
  lazim cargo-tauri "install it with: cargo install tauri-cli --version ^2 --locked"
  # `beforeBuildCommand` is emptied because the `wajiha` stage above already
  # built the frontend. Letting tauri run it again is not merely wasteful: it
  # rewrites `apps/studio/dist` *while* `tauri::generate_context!` is walking
  # it, and the compile then fails with "failed to read asset at
  # …/dist/assets/<hashed>.ttf … os error 3" naming a file that exists. One
  # frontend build, finished before the compile starts, is the fix.
  #
  # Through a file rather than `--config '{"…"}'`: a JSON argument survives a
  # POSIX shell fine, but on Windows the callee re-parses the command line and
  # the quotes do not survive it. A path has no quotes in it.
  #
  # `beforeBundleCommand` is deliberately left alone: `tadqiq_mawarid.mjs` is the
  # gate that refuses to turn an unstaged `mawarid/` into an installer.
  mkdir -p "$ahdaf"
  printf '{"build":{"beforeBuildCommand":""}}' > "$ahdaf/isdar-tajawuz.json"
  tajawuz="$(cd "$ahdaf" && pwd)/isdar-tajawuz.json"
  ( cd apps/studio && cargo-tauri build --target "$hadaf" --config "$tajawuz" )
fi

marhala "تمّ"
printf 'isdar: %s\n' "$hadaf"
printf '  manifest: apps/studio/src-tauri/mawarid/bayan_mukawwinat.json\n'
printf '  bundle:   %s/release/bundle/\n' "$ahdaf"
