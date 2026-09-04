# adapters-script

The in-game sides of the script-engine adapters: the code that actually runs
inside RPG Maker, Ren'Py, VX Ace and Electron games after a patch is
installed. The Rust patchers that put these files into a game live in
`crates/taarib-muhawwil-nusus`; what lives here is what executes afterwards,
in whatever interpreter the game happens to ship.

## What lives where, and which phase builds it

| Directory | Language and target | Built in |
| --- | --- | --- |
| `renpy/taarib_renpy/jisr.py` | Python 3.9 (Ren'Py 8's bundled CPython), `ctypes` over the native ABI | Phase 3 |
| `renpy/` (the rest of the package) | The Ren'Py runtime module: version detection, the text filter, the custom displayable | Phase 10 |
| `vxace/taarib_rgss3.rb` | Ruby 1.9.2 (RGSS3), `Win32API` over the native ABI, 32-bit Windows only | Phase 3 |
| `vxace/` (the injected window code) | `Bitmap#draw_text` and `Window_Base` replacements that draw through the binding | Phase 10 |
| `rpgmaker/` | TypeScript compiled to `js/plugins/taarib.js` for MV and MZ, over the WASM core | Phase 10 |
| `electron/` | TypeScript preload and renderer runtime for Electron games, over the WASM core | Phase 10 |

Phase 3 builds the two native-ABI bindings because they are part of the ABI's
own definition of done: the C surface is not finished until something that is
not Rust has bound every function of it. Phase 10 builds the adapters proper
on top of them. The two JavaScript trees have no binding file here because
they do not use the native library at all — they load `taarib_core.wasm`
(built in Phase 3 alongside the ABI), which exposes the same shape through
`taarib-wasm`'s wrapper.

## The one rule of this tree

**All four language sides call the same engine. None of them implements any
of it.**

No file in this tree shapes Arabic, reorders a bidirectional line, chooses a
kashida, positions a diacritic, or maps a codepoint to a glyph. The Python
and Ruby sides cross the C ABI documented in `docs/abi.md`; the two
JavaScript sides cross into the same Rust code compiled to WebAssembly. What
this tree contains is transport and integration: marshalling structures
across an FFI boundary, hooking the engine's draw calls, and blitting glyphs
the engine handed back.

The reasoning is the same one that closed Decisions 2 through 4 of the
roadmap: a second Arabic shaping implementation is a second set of bugs.
Shaping correctness lives in the interaction of contextual joining, mandatory
ligatures, bidirectional reordering and mark attachment — failures there are
silent, data-dependent, and reproduce only on specific fonts and specific
sentences. One implementation means one place those bugs can exist, one place
they get fixed, and the guarantee that the preview in Taarib Studio, the text
in a Unity game and the text in a VX Ace game are the same pixels from the
same code. A convenience reimplementation in JavaScript or Ruby — however
small, however "just for this one case" — would fork that guarantee, and the
fork would drift.

The practical consequence for anyone working here: if a task seems to need
text logic on this side of the boundary, the task is wrong or the ABI is
missing something. Extend `taarib-jisr` (behind its versioning rules), never
the adapter.

## Constraints worth knowing before editing

- Each side targets the interpreter the game ships, not the one on your
  machine: Python 3.9 for Ren'Py 8, Ruby 1.9.2 for RGSS3, ES5-compatible
  output for RPG Maker MV's NW.js. The binding files document their exact
  constraints in their own headers; the Ruby file in particular explains the
  `Win32API` marshalling it is built on, and why.
- Injected code contains no dynamic evaluation of patch content, is complete
  and readable as shipped, and never reads the game's own font assets
  (Decision 5).
- Everything an adapter renders was decided before the game launched.
  Adapters do not check safety, do not touch the network, and do not read
  the registry — they render what the patch says (boundary 3 of the
  component map).
