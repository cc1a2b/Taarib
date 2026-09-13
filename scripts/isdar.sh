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
# Set TAARIB_MIFTAH_ISDAR to the release public key for a release bundle;
# without it the bundle trusts the development key committed in the tree and
# says so at the end.
#
# Stages, in dependency order:
#
#   unity   rows C1–C4   dotnet build      the BepInEx-side assemblies
#   hamula  rows B,E,F,G cargo build       the game-side cdylibs, per platform
#   wasm    row  H1      cargo + wasm-bindgen
#   mulhaq  rows I1,I2   node adapters-script/ibni.mjs
#   tajmee  rows D1,J1,K1,M1,N1            stage, verify, write the manifest
#   badhra  row  L1      sabk --badhra     the revocation seed, release only
#   wajiha  row  A2      the frontend
#   huzma   rows A1,A3   tauri build       the installer
#
# TAARIB_TASALSUL sets the seed's sequence number; it defaults to 1.
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
    -h|--help) sed -n '2,41p' "${BASH_SOURCE[0]}"; exit 0 ;;
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
# The bundler. Overridable for the same reason: a WSL host bundling for Windows
# runs `cargo-tauri.exe`, a name `command -v cargo-tauri` does not resolve.
TAURI="${TAARIB_TAURI:-cargo-tauri}"
# The SDK that builds rows C1-C4, overridable for that reason once more: a WSL
# host has no .NET of its own and drives the Windows SDK across /mnt, where the
# bare name `dotnet` resolves to nothing at all.
DOTNET="${TAARIB_DOTNET:-dotnet}"
TAKHATTI="${TAARIB_TAKHATTI:-}"

# The trust anchor this bundle is built against: the release *public* key, as 64
# hex characters. `taarib_khatm::MIRSAT_MALIK` reads it through `option_env!`, so
# exporting it is what decides whether the client trusts the release identity or
# the development key committed in the tree. The private half is minted by
# `taarib-khatm --bin isdar wallid` inside the owner's keychain and never leaves
# it — nothing here reads, writes or needs it.
#
# Checked here rather than only at compile time so that a typo fails in a second
# instead of after the unity and payload stages, and because a build that quietly
# falls back to the development anchor produces an installer that reports
# `tatwir` in its provenance and refuses every patch the owner actually signed.
MIRSA="${TAARIB_MIFTAH_ISDAR:-}"
if [ -n "$MIRSA" ]; then
  if ! printf '%s' "$MIRSA" | grep -Eq '^[0-9a-fA-F]{64}$'; then
    printf 'isdar: TAARIB_MIFTAH_ISDAR is not 64 hex characters.\n' >&2
    printf '       It is the release public key printed by:\n' >&2
    printf '           cargo run -p taarib-khatm --bin isdar -- wallid\n' >&2
    exit 2
  fi
  export TAARIB_MIFTAH_ISDAR="$MIRSA"
  SIMAT_ISDAR=(--features taarib-khatm/isdar)
else
  SIMAT_ISDAR=()
fi

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
  lazim "$DOTNET" "the four BepInEx-side assemblies are built with it"
  "$DOTNET" build unity/Taarib.Unity.sln -c Release --nologo
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
  #
  # The triple list is read on descriptor 3, not on stdin. A build in the body
  # inherits descriptor 0, and a `cargo.exe` driven from WSL drains it: the loop
  # then sees end-of-file after the first triple and every later one is skipped
  # in silence. That is worse than it sounds, because the skip is invisible —
  # a `target/` that already held yesterday's `i686` payloads staged them again
  # and the bundle looked complete. `tajmee` names the absence on a clean tree,
  # so the release fails rather than ships, but it fails for the wrong reason.
  while read -r muthallath <&3; do
    printf -- '-- %s\n' "$muthallath"
    "$QARGO_HAMULA" build --release --target "$muthallath" \
      -p taarib-jisr -p taarib-mudkhal -p taarib-tabaqa \
      -p taarib-muhawwil-unreal -p taarib-muhawwil-godot \
      --features taarib-tabaqa/hamula,taarib-muhawwil-unreal/hamula,taarib-muhawwil-godot/hamula \
      --target-dir "$ahdaf"
  done 3< <(muthallathat_hamula)
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

if [ -n "$MIRSA" ] && ! tuhmal badhra; then
  marhala "badhra — row L1, the revocation seed"
  # The seed is compiled into the studio through `include_bytes!`, and it is
  # verified at startup against whatever anchor the build carries. The committed
  # one is signed by the development key, so a release build anchored elsewhere
  # rejects it and the safety layer stops before a game is scanned — the failure
  # says "the signature does not verify against the owner key" and names nothing
  # about keys, which is why this ran into it the hard way once already.
  #
  # Written before `huzma` because the studio compiles it in, and the sequence
  # tracks the manifest's: a seed older than the served list is replaced by it
  # on first fetch, which is the direction that has to hold.
  "$QARGO" run --release -q -p taarib-mustawda --bin sabk \
    ${SIMAT_ISDAR[@]+"${SIMAT_ISDAR[@]}"} \
    -- --badhra assets/qaimat_sahb.json --tasalsul "${TAARIB_TASALSUL:-1}"
fi

if ! tuhmal wajiha; then
  marhala "wajiha — row A2"
  lazim pnpm "the studio frontend is built with it"
  ( cd apps/studio && pnpm install --frozen-lockfile && pnpm build )
fi

if ! tuhmal huzma; then
  marhala "huzma — rows A1, A3, and the installer"
  lazim "$TAURI" "install it with: cargo install tauri-cli --version ^2 --locked"

  # What the bundler ships is whatever `mawarid/` holds when it walks it, which
  # is not necessarily what this run staged: `TAARIB_TAKHATTI=tajmee` skips the
  # staging outright, and a tree an earlier run left for another triple carries
  # that triple's payloads under names this one will never look for.
  # `tadqiq_mawarid.mjs` catches neither — it gates the fonts, and the fonts are
  # the one part of the tree that is byte-identical on every target.
  bayan="apps/studio/src-tauri/mawarid/bayan_mukawwinat.json"
  if [ ! -f "$bayan" ]; then
    printf 'isdar: %s is absent.\n' "$bayan" >&2
    printf '       Its absence is the marker for a partial staging, and an\n' >&2
    printf '       installer built over one refuses every component by name.\n' >&2
    printf '       Run the tajmee stage before huzma.\n' >&2
    exit 1
  fi
  # `awk` on the first `"hadaf"` line rather than a JSON parser: the manifest is
  # pretty-printed, so the value is on the key's own line, and the only other
  # keys in the file are `mukhattat`, `isdar`, `masar`, `hajm` and `sha256`.
  mustaqirr="$(awk -F'"' '/^[[:space:]]*"hadaf"[[:space:]]*:/ { print $4; exit }' "$bayan")"
  if [ "$mustaqirr" != "$hadaf" ]; then
    printf 'isdar: mawarid/ is staged for %s, and this bundle is %s.\n' \
      "$mustaqirr" "$hadaf" >&2
    printf '       Re-run the tajmee stage for %s.\n' "$hadaf" >&2
    exit 1
  fi

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
  # Written at a fixed place under `target/` rather than under `--ahdaf`, and
  # named to the bundler *relatively*, because those are the two things that
  # hold on every host: a bundler invoked across the WSL boundary cannot open
  # `/mnt/e/...`, and `--ahdaf` may be somewhere else entirely. This file is not
  # a cargo output, so it lives beside the other two things `masfufa.rs`
  # resolves from the workspace root — `target/adapters` and
  # `target/wasm-bindgen` — for the same reason.
  #
  # `beforeBundleCommand` is deliberately left alone: `tadqiq_mawarid.mjs` is the
  # gate that refuses to turn an unstaged `mawarid/` into an installer.
  mkdir -p target
  printf '{"build":{"beforeBuildCommand":""}}' > target/isdar-tajawuz.json
  # `taarib-khatm/isdar` is passed only when an anchor was injected, and it is
  # not decoration: the feature's own `const` assertion fails the compile if the
  # variable went missing between here and the crate, which is the one failure
  # mode that would otherwise ship silently.
  ( cd apps/studio \
      && "$TAURI" build --target "$hadaf" --config ../../target/isdar-tajawuz.json \
        ${SIMAT_ISDAR[@]+"${SIMAT_ISDAR[@]}"} )
fi

marhala "تمّ"
printf 'isdar: %s\n' "$hadaf"
if [ -n "$MIRSA" ]; then
  printf '  anchor:   release — %s\n' "$MIRSA"
else
  printf '  anchor:   development — this bundle trusts the key committed in the\n'
  printf '            tree and refuses a patch signed by the owner. Set\n'
  printf '            TAARIB_MIFTAH_ISDAR to build a release bundle.\n'
fi
printf '  manifest: apps/studio/src-tauri/mawarid/bayan_mukawwinat.json\n'
# `--target` is passed to the bundler, so the artifacts land one directory
# deeper than an untargeted build would put them.
printf '  bundle:   %s/%s/release/bundle/\n' "$ahdaf" "$hadaf"
