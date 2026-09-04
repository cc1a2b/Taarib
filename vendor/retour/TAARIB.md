# Why this crate is vendored

`retour` 0.3.1 as published does not compile for **any** target on a current
Rust compiler, and neither does any of its forks (`detour`, `detour2`,
`detour3`) or its own `0.4.0-alpha` line — they all carry the same four lines.

`src/macros.rs` emits `Function` trait impls for `extern "stdcall"`,
`extern "fastcall"`, `extern "thiscall"` and `extern "win64"`
unconditionally. The first three exist only on 32-bit x86 and the fourth only
on x86_64, and Rust turned "unsupported ABI" from a future-compatibility
warning into a hard error (`E0570`). The result is that the crate fails to
build for `x86_64` and `aarch64` on the first three lines and for `i686` on the
fourth, before any of Taarib's own code is reached.

## The change

Four `#[cfg(target_arch = ...)]` attributes in `src/macros.rs`, marked
`// TAARIB:` — three `"x86"` and one `"x86_64"`. Nothing else is modified. The
impls are gated rather than deleted because both a 32-bit and a 64-bit game
process are in Taarib's coverage matrix and on each architecture its own ABIs
are correct.

The `win64` gate arrived later than the other three, when a release build first
had to produce the `i686-pc-windows-msvc` payloads of rows B1/E1/F1/G1/G2 of
docs/tawzee.md §3: nothing before that had built this crate for a 32-bit
target, so the mirror-image half of the same defect had never been reached.

## Why vendored rather than replaced

No maintained alternative covers Taarib's platforms under an acceptable
licence: `neohook` is Windows-only, `sighook` is LGPL-2.1-only, and `hook_king`
is GPL-3.0. Writing a second instruction-length decoder was the other option,
and a hand-rolled prologue relocator is the last thing this product should own
two of.

`retour` is BSD-2-Clause; redistribution with the notice retained is permitted,
and `LICENSE` is unmodified beside this file. When upstream gates these ABIs,
this directory is deleted and the `[patch.crates-io]` entry in the workspace
manifest goes with it.
