//! # محوّل تعريب لمحرّكات النصوص — the script-engine patchers
//!
//! Five engines whose content is plain data or interpreted script. For all of
//! them, patching the data directly is more robust than injecting into the
//! process: the game loads Taarib's translations through its own loader, on
//! every version, with no hook to break when the engine updates. One shared
//! discipline across all five — preserve the originals byte-exact, write the
//! patched output separately and atomically, stay fully reversible.
//!
//! This crate is the **Rust side**: the archive readers, the format rewriters,
//! and the payload generators. The in-game side lives in `adapters-script/` —
//! TypeScript for RPG Maker MV/MZ and Electron, Python for Ren'Py, Ruby for
//! VX Ace — and reaches the engine through `taarib-wasm` or through the C ABI,
//! never through this crate.
//!
//! Built in **Phase 10**, on Phases 1, 2, 3, 5 and the container format from
//! Phase 14.
//!
//! ## Modules
//!
//! | module | what it patches |
//! | --- | --- |
//! | `tabaqa` | the shaping tier probe: which of three rungs a game lands on, the evidence that produced the verdict, and the refusal when the evidence contradicts itself |
//! | `tawjih` | the registry of all five probes and the join from a Phase 5 identification to a rung, including the rule that prefers the inner engine over the wrapper when both apply |
//! | `tarkeeb` | the single entry point an installer calls: identify the adapter from the directory, establish the rung, read the game's own strings, look each one up, and write the translations back through the adapter that owns the format |
//! | `hifz` | the reversibility contract every patcher goes through: originals preserved before the first write, a manifest written first, atomic writes, and a restore verified against a recorded fingerprint |
//! | `khata` | every refusal this crate can produce, and whether it stops a patcher or merely picks a different rung |
//! | `rpgmaker` | MV and MZ: `data/*.json` event command lists (codes 101, 401, 102, 405), `CommonEvents`, `Items`, `Weapons`, `Armors`, `Actors`, `Skills`, `States`, `Enemies`, `Classes`, `Troops`, `System.json`, plus injecting `js/plugins/taarib.js` and registering it in `plugins.js` |
//! | `vxace` | the `Game.rgss3a` RGSSAD v3 archive with its key-derived obfuscation, the `Data/*.rvdata2` Ruby `Marshal` streams, deflate-compressed `Scripts.rvdata2`, and the injected Ruby script entry |
//! | `renpy` | generated `game/tl/arabic/*.rpy` — `translate arabic <label>:` blocks for dialogue and `translate arabic strings:` for interface text — plus style and `gui` overrides, a `config.font_replacement_map` entry, `.rpa` archive reading, and version detection choosing between the engine's own `HarfBuzz` path and the `ctypes` layout path |
//! | `gamemaker` | the `data.win` FORM container: `GEN8` for version, `STRG` for the string pool with its reference table rebuilt, `FONT` for glyph tables, `TXTR` for texture pages, and `CODE`/`VARI` for the bytecode string references |
//! | `electron` | `resources/app.asar` unpacked, a preload script and renderer runtime injected, and repacked |
//!
//! ## Why `saff` and `lawha` are here and `jisr` is not
//!
//! Two of these engines need Taarib to rasterize at *patch time*, not at run
//! time. GameMaker gets a generated font resource whose glyph table is Taarib's
//! own rasterized glyphs on a Taarib-packed texture page appended to `TXTR`,
//! addressed through the glyph-identifier transport, with the text emitted in
//! visual order from real shaping. VX Ace gets an atlas bitmap loaded from the
//! patch and blitted by the Ruby side. Both need shaping and packing while the
//! compiler runs, which is `saff` and `lawha` directly.
//!
//! The runtime sides need `jisr` or the WASM core, and they link it themselves
//! from their own languages. This crate does not depend on the ABI because it
//! never crosses it.
//!
//! ## Escape codes are markup, not text
//!
//! RPG Maker's `\C[n]`, `\I[n]`, `\V[n]`, `\N[n]`, `\P[n]` and `\G` are `nasq`
//! spans, so colour changes and inline icons keep working inside Arabic text
//! instead of being shaped as literal backslashes and brackets. The same
//! applies to Ren'Py's text tags and to the HTML the Electron runtime walks.
//! Markup never reaches the bidi algorithm and never becomes a glyph — the same
//! rule that holds inside `saff` holds at every boundary that feeds it.
//!
//! ## Hard constraints
//!
//! - Every original file touched is backed up byte-exact into `nusakh/` before
//!   the first write, with a manifest entry flushed first. `hifz` owns the
//!   mechanism and it is not optional: no patcher in this crate holds a file
//!   handle to a game directory. All five take a `&mut dyn Hafiz` and call one
//!   of its four operations, so "the change was recorded before it happened" is
//!   a property of the call graph rather than a rule five authors remember. A
//!   reviewer's check is a one-line grep — `fs::File::create` outside `hifz`
//!   returns the `.rpa` builder and nothing else, and that one refuses to
//!   overwrite anything, so it has no original to lose.
//! - Patched output is written atomically — beside, fsync, rename — through the
//!   single implementation in `taarib_usus::masarat::kitaba_dharra`, so an
//!   interrupted patch never leaves a truncated `data.win` or `app.asar`. Those
//!   two files *are* the game; a half-written one is an unplayable game. There
//!   is deliberately one such implementation and not six: the one used least
//!   often would be the one whose fsync ordering was wrong.
//! - The injected Ruby, Python and JavaScript is complete, readable, and
//!   contains no dynamic evaluation of patch content. A patch is data, and data
//!   never becomes code.
//! - The glyph-identifier transport is used only where the engine cannot be
//!   made to shape and cannot be hooked, and is documented as a glyph
//!   transport, never as text (Decision 1 holds).
//! - Uninstalling restores a byte-identical original, and the restore is
//!   verified against the recorded hash rather than assumed.

pub mod electron;
pub mod gamemaker;
pub mod hifz;
pub mod khata;
pub mod renpy;
pub mod rpgmaker;
pub mod tabaqa;
pub mod tarkeeb;
pub mod tawjih;
pub mod vxace;

pub use crate::hifz::{Hafiz, HarisKitaba, Hifz, NawTaghyeer};
pub use crate::khata::KhataNusus;
pub use crate::tabaqa::{Hukm, Mifhas, Rutba, SiyaqTabaqa};
pub use crate::tarkeeb::{Mawarid, Mutarjim, TaqreerTarkeeb, ayn_hadaf, rakkib_luba};
pub use crate::tawjih::{HasilatTashkhees, tashkhees};

/// The directory, under the game's root, that holds every preserved original.
///
/// One place rather than a backup beside each file, so that uninstalling is a
/// directory walk rather than a search, and so that a user who deletes it by
/// hand has broken one obvious thing rather than silently lost the ability to
/// undo a patch they still have installed.
pub const DALIL_NUSAKH: &str = "nusakh";

/// The manifest file inside [`DALIL_NUSAKH`].
///
/// Written before the first modification and updated before each subsequent one.
/// A modified file with no manifest entry is unrecoverable, which is why the
/// ordering is a contract rather than a convenience.
pub const MALAF_BAYAN: &str = "bayan.json";
