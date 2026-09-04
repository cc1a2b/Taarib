<!--
Delete the sections that do not apply. An empty heading left in place is noise; a heading
deleted because it genuinely does not apply is a decision, and both are fine. What is not fine
is leaving a checkbox ticked that was not actually done — a green box nobody checked is worse
than no box, because it stops the next person looking.
-->

## What this changes

<!-- One paragraph. What behaviour is different after this than before it. -->

## Why

<!--
The reasoning, not the restatement. If this fixes a reported issue, link it. If it contradicts
something written down in ROADMAP.md, docs/, or a module header, say so here and argue it —
that conversation is welcome, and discovering it in review is not.

ROADMAP.md section 2 holds eight settled architectural decisions. A change that contradicts one
of them will be declined regardless of how well it is written. Decision 1 — never produce
Unicode presentation forms — is the one newcomers reopen most often, and docs/taqdimiya.md
exists so the argument does not have to be had from scratch each time.
-->

## Gates

Run these locally before opening. CI runs the same commands on the same toolchain, so a green
run here is a green run there.

- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] From `apps/studio`: `npx tsc -p tsconfig.json`, then `npx tsc -p tsconfig.node.json`,
      then `npx vite build`

`cargo fmt --all --check` is **not** a gate and is expected to fail — the tree currently has
about 5,400 formatting hunks across 300 files, and the reformat is pending. Do not reformat
files this change did not otherwise touch; a diff where the real change is buried in whitespace
is a diff nobody can review.

`--all-targets` on the clippy line is not decoration. It is what makes clippy see test code,
and the workspace's denied lints apply there too — see CONTRIBUTING.md for the `#![allow]`
header test files carry.

## Platforms

<!--
Three crates carry their real substance behind `#[cfg(windows)]`: taarib-mudkhal (the
version.dll proxy), taarib-tabaqa (the D3D11/D3D12 presentation hooks) and taarib-haqn (process
injection). All three compile on Linux while compiling none of that code, so a Linux-only green
proves nothing about them. If you touched any of the three, say what you ran on Windows.
-->

- [ ] I did not touch `taarib-mudkhal`, `taarib-tabaqa` or `taarib-haqn`
- [ ] I touched one of them and ran `cargo check --workspace` on Windows — result below

## Documentation

- [ ] Module headers that explain behaviour this change alters have been updated
- [ ] New public items carry doc comments, with `# Errors` on anything returning `Result` and
      `# Safety` on anything `unsafe`
- [ ] No doc comment now describes code that no longer exists

<!--
If this changes what a *user* is told, the user-facing sentences are part of the change too,
and they exist in Arabic and English. A row with one of them empty is a screen with a blank
on it.
-->

## If this touches engine detection or capability

- [ ] `taarib_muharrik::ISDAR_FAHS` is raised **in this same commit**

<!--
Non-negotiable, and the trap it guards is specific: a change that would produce a different
report for a game whose files did not change, without the version bump, passes your local
testing (your database is fresh), passes review (the diff looks complete), and then fails
silently and only for users who already own the game — they keep a stale conclusion forever.

Adding an engine always counts. So does a new detector, a corrected weight, a changed tier
rule, or a reworded limitation. A change that cannot alter any output does not.
-->

- [ ] `docs/tashghil.md` has been updated, with the function name and the `path:line` where the
      chain stops, established by grep and not by impression
- [ ] `docs/bidaya.md` §6 agrees with it

## If this touches the shared vocabulary or the command surface

- [ ] `schemas/*.json` regenerated with
      `cargo run -p taarib-mustalahat --features mukhattatat --bin mukhattatat` — never edited
      by hand
- [ ] The generated TypeScript in `apps/studio/src/mustalahat/awamir.ts` is current
- [ ] Any hand-written TypeScript type that mirrors a Rust one still matches it

<!--
On that last box: there is a live mismatch to be aware of rather than to copy. The generated
Rust enum is `JahiziyatTashghil`; the hand-written TypeScript union in
apps/studio/src/maktaba/jahiziya.ts is spelled `JahiziyaTashghil`, one `t` short. Both compile,
neither is wrong at runtime — the wire format is the snake_case string — and grepping for one
name silently misses the other. If you add a hand-written mirror of a generated type, spell it
identically.
-->

## Registry records

- [ ] This pull request does not hand-edit a registry record

<!--
Translations are submitted through the application, which seals the package, builds the record,
forks the registry, pushes a branch and opens the pull request. A hand-edited record is a record
nobody verified, and will be closed with a pointer to CONTRIBUTING.md.
-->

---

One logical change per pull request. Conventional commit format. If it touches a phase in
ROADMAP.md, name the phase.
