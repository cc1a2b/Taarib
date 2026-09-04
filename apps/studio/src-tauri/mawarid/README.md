# mawarid — the staged resource tree

This directory is the staging target, not source. `crates/taarib-tajmee` fills
it before a release bundle is built:

    cargo run -p taarib-tajmee -- --hadaf <target-triple> --jalb

`--jalb` is what permits the network, and a first run needs it. Neither the
BepInEx archives nor the twenty-five font faces are in this repository; they are
pinned by sha256 in `assets/aqfal/` and fetched into `target/tajmee/makhbaa`.
Without the flag a clean checkout stages neither and refuses both by name.

**That command alone is not enough, and it will tell you so.** `taarib-tajmee`
builds nothing: ten Rust cdylibs, four Unity assemblies, the `wasm-bindgen`
pair and the two compiled script adapters have to exist before it runs, and on
a clean checkout none of them do — the tool exits non-zero naming every one.
`scripts/isdar.sh --hadaf <target-triple> --jalb` runs those builds in
dependency order and then this one; `.github/workflows/isdar.yml` is the same
sequence on a runner. Run either of those rather than this command by hand,
unless you already know what you rebuilt.

It is committed empty because `tauri.conf.json` declares `bundle.resources`
against it, and `tauri-build` checks that the path exists on every build —
including a `cargo build` that will never produce a bundle. Without the
directory, no development build of the Studio compiles at all.

**Staging preserves this file.** It did not always: `Mustaqarr::iftah` used to
clear the staging root with `remove_dir_all`, and this README lives inside that
root, so every staging run deleted a tracked file and left the next person to
clone with a directory and no explanation of what it is for. `iftah` now creates
the root and clears its *contents*, keeping `README.md`; symlinks are unlinked
rather than descended. Nothing needs restoring afterwards.

## Empty is valid for a build, and fatal for a bundle

The two cases are not the same, and conflating them shipped an installer whose
`mawarid/` held only this file.

**Components may be absent.** `zamin_mukawwinat` reports a build with no
`bayan_mukawwinat.json` as one named problem at startup, after which the Studio
continues and every install refuses each absent component by name rather than
deploying a fraction of one.

**Fonts may not.** `taarib-saff` has no Arabic fallback shaper: with no font
under `mawarid/khutut/`, and none imported by the user, every layout, preview
and patch build fails with `KhututNaqisa` — after download, install and first
launch have all appeared to succeed. Nothing in the bundler noticed.

So `build.beforeBundleCommand` runs `../src-tauri/tadqiq_mawarid.mjs` between
the compile and the bundler. It checks every font `assets/aqfal/qufl_khutut.json`
locks against its recorded size and SHA-256, then checks every face
`assets/fonts/khutut.json` declares for the tables, the OpenType features and
the character coverage `crates/taarib-saff/src/khatt.rs` demands at load time —
GSUB `init`/`medi`/`fina`/`rlig`, GPOS `mark`, and the forty characters of
`HURUF_MATLUBA`. A build that would ship an unshapeable font, or no font, stops
there and names each file.

`isol` is not on that list and must not be added; the reason is in both
`tadqiq_mawarid.mjs` and `khatt.rs`.
