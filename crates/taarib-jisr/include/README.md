# include/taarib.h

`taarib.h` is the C header for the stable native ABI of `taarib_jisr` — the
shared library (`taarib_jisr.dll` / `libtaarib_jisr.so` /
`libtaarib_jisr.dylib`) that carries the Taarib Arabic text engine into
processes that are not Rust. It declares the 28 exported `taarib_*` functions,
the frozen `Taarib*` structures, the four opaque handle types, and every
`TAARIB_*` status code and flag, each with its ownership contract carried in
the comment above it.

## Generated, and committed on purpose

The header is a build product, not a source file. `build.rs` runs cbindgen
(configured by `../cbindgen.toml`) over this crate's public surface on every
build, compares the result with this file, and rewrites it only on a real
difference. Do not edit it by hand: the next build with the Rust toolchain
destroys hand edits. When the header and the Rust disagree, the Rust is the
truth — fix it there, rebuild, and commit both.

It is committed so that a consumer can bind against the ABI without the Rust
toolchain. Regeneration is deliberately best-effort: a vendored build, a
read-only source tree, or a sandboxed packager builds from this committed copy
as-is, and a failed regeneration is a `cargo:warning`, never a build failure.

## Regenerating

Build the crate with the Rust toolchain present:

```
cargo build -p taarib-jisr
```

`build.rs` reruns cbindgen whenever anything under `src/` or `cbindgen.toml`
changes, and leaves the file's timestamp alone when the surface is unchanged.

## Who binds through this header — and who does not

Uses `taarib.h` directly:

- Nothing in this repository yet. There is no C or C++ source in the tree that
  includes it — the Unreal and Godot adapters are Rust crates that link
  `taarib-jisr` as an `rlib`, and the worked example in `docs/abi.md` is the
  only C that compiles against it.
- Any third party binding against `taarib_jisr` from C or C++.

Does not use it — these mirror the frozen layouts of
`crates/taarib-jisr/src/anwa.rs` in their own language instead, because their
runtimes have no C preprocessor to hand the header to:

- The Unity C# adapter: `DllImport` declarations, with every structure
  mirrored field for field in `unity/Taarib.Unity.Jisr/Anwa.cs`.
- The Ren'Py adapter: a `ctypes` binding in
  `adapters-script/renpy/taarib_renpy/jisr.py`.
- The RPG Maker VX Ace adapter: a `Win32API` binding in
  `adapters-script/vxace/taarib_rgss3.rb`.

For those mirrors, `anwa.rs` is the reference and this header is the neutral
statement of the same contract. The long-form ABI documentation — status
codes, the handle model, the capacity negotiation, threading — is
`docs/abi.md`.
