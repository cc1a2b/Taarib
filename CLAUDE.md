# Taarib · تعريب

<!-- Project instructions. The behavior contract (finish flows, verify once at the end,
     deep engineering, minimal comments, agents only for big work) lives in ~/.claude/CLAUDE.md
     and is not repeated here. This file is the project-specific facts. -->

## Stack

Rust workspace (~30 crates, edition 2024, toolchain pinned 1.95.0) + TypeScript/React 19/Vite/Tauri 2 desktop app (apps/studio) + C# BepInEx Unity plugin + WASM build. Local SQLite (taarib.db). Shaper HarfRust, rasterizer skrifa.

## Commands

- workspace check: `cargo check --workspace --all-targets`
- clippy: `cargo clippy --workspace --all-targets`
- test: `cargo test --workspace`
- fmt: `cargo fmt --all --check`
- deny: `cargo deny check`
- studio dev: `cd apps/studio && npm install && npm run dev`
- studio build: `cd apps/studio && npm run build` then `npm run tauri build`
- studio typecheck: `cd apps/studio && pnpm exec tsc -p tsconfig.json`

## Domain and layout — crate names are Arabic and are the vocabulary; never translate them

- Taarib brings correct Arabic rendering (contextual joining, ligatures, diacritics, bidi) into PC games with no native Arabic support, plus a signed community patch registry.
- Foundations: `usus` (errors, schemas), `mustalahat` (shared concepts: luba=game, muharrik=engine, bina=build).
- Text engine: `saff` (logical→visual layout, the core contract), `lawha` (glyph atlas), `ruqaa` (the .ruqaa signed patch container), `tarqee` (patch compiler).
- Discovery/dispatch: `kashf` (find installed games), `muharrik` (identify engine + capability report), `aql` (one game held whole).
- Injection: `haqn` (hooking primitives), `mudkhal` (self-loaded loader: version.dll / LD_PRELOAD / DYLD_INSERT_LIBRARIES), `tabaqa` (universal overlay, tier 3 fallback).
- Adapters: `muhawwil-unreal`, `muhawwil-godot`, `muhawwil-bio4`, `muhawwil-nusus` (script engines).
- Trust: `khatm` (Ed25519 sign/verify, key custody — the ONLY crate that holds a private key or builds a signature), `aman` (pre-install safety gate, produces the IdhnTathbeet an install requires), `tathbeet` (install/restore).
- Content: `istikhraj` (string extraction), `tarjama` (game-aware MT), `warsha` (serverless collaborative translation), `taqdeem` (submission/review/publish), `mustawda` (registry client), `makhzan` (the only opener of taarib.db).
- Cross-cutting: `jisr` (the single versioned C ABI — the only non-Rust entry point), `wasm`, `tajmee`/`tajmi` (staging), `tahdith` (self-update), `saff`/`lawha` also run inside the game via WASM/C.

## Architecture facts Claude cannot infer

- No Unicode Presentation Forms, ever — shaping is HarfRust only. HarfRust + skrifa share font parsing so glyph IDs cannot mismatch.
- The text engine is self-contained: it never borrows the game's fonts or asset bundles.
- Every published patch requires the owner's cryptographic signature; community work flows through review, never auto-publish. No second signing path may be added.
- `jisr` is the sole entry from C#/C++/Python/Ruby/JS — do not add an engine-specific ABI extension.
- The graphics overlay (`tabaqa`), not per-engine plugins, is the universal coverage path.
- Only `common`/host-testable crates run on the host; anything touching a game needs a running title on a display (some behaviors remain verified-by-inference only).

## Gotchas

- Arabic-focused repo: Arabic in functional content only (crate names, sample text) — no decorative slogans per the workspace README policy.
- Toolchain is pinned in rust-toolchain.toml (1.95.0); CI checks the pin — do not bump casually.
- Money/keys/signatures are security-critical (khatm, aman) — never weaken verification to make a flow pass.
