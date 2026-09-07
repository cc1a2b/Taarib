# src/mustalahat — generated bindings

`awamir.ts` in this directory is generated. Do not edit it, do not reformat it,
and do not commit a hand-made correction to it. The other two files are written
by hand: this README, and `arqam.ts`, which holds the decisions the generator
cannot express (its own header says so) and exports one helper, `kasr`.

The generated file **is** committed, which is the one thing here that surprises
people. It is committed so that a fresh clone type-checks before anyone has run
the backend once, and because a build that has never run cannot generate
anything. It is still generated output: the Rust derives are the source of
truth, the file is rewritten on every debug start, and a disagreement between
the two means this file is stale rather than that the Rust is wrong.

## What is here

`awamir.ts` — the command surface and every type it mentions:

- one exported function signature per Tauri command registered in
  `src-tauri/src/main.rs`,
- the TypeScript form of every Rust type those commands accept or return,
  including the shared vocabulary from `taarib-mustalahat` and the error model
  from `taarib-usus` (`Khata`, `Khutwa`, `Khutura`, `Ramz`),
- the event payload types, once the backend emits events.

`arqam.ts` — hand-written, beside the generated file on purpose: the numeric
conventions the bindings need and `specta` cannot derive. Nothing in it
describes a Rust type.

## Where it comes from

`taarib-studio` writes it. The Rust binary builds a `tauri_specta::Builder`
from the `#[tauri::command]` functions it registers, and in a **debug build**
exports that builder to `../src/mustalahat/awamir.ts` before the window opens.
The types themselves come from `specta::Type`, derived across the workspace
behind the `wajiha` feature, which `apps/studio/src-tauri/Cargo.toml` turns on
for both `taarib-usus` and `taarib-mustalahat`.

That means a release build never writes here: it ships whatever was generated
while it was being developed.

## How to regenerate it

Run the application once in development:

```
cd apps/studio
npm run tauri dev
```

The file is rewritten on every debug start, before the frontend is served, so
the TypeScript the interface compiles against is always the command surface the
running backend actually has.

If the file were missing, the frontend would not type-check at all, because
`src/hayat/jisr.ts`, `src/lugha/lugha.ts` and every screen import their request
and response shapes from it. That is why it is committed rather than ignored:
the alternative is a repository that cannot be built until it has been run, and
a first contribution that begins with a wall of unresolved imports.

The consequence to watch for is staleness. A Rust type changed without a debug
start leaves this file describing a shape the backend no longer sends, and
TypeScript will happily believe it. Any change to a `#[tauri::command]`
signature, or to a type behind the `wajiha` feature, is finished only when this
file has been regenerated and the regeneration committed with it.

## Why it is generated rather than written

A request shape written twice is a request shape that will eventually be
written differently in the two places. Renaming a field in Rust, adding a
variant to an enum, or making an optional field required all have to be visible
to the interface as a type error in the screen that reads them — not as a
value that quietly arrives as `undefined` in a webview.

There is no hand-written TypeScript interface anywhere in this application that
describes a Rust type. If one is needed, the Rust type is missing a
`specta::Type` derive, and that is what to fix.
