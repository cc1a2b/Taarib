//! # محوّل تعريب لمحرّك غودو — the Godot adapter
//!
//! Two different engines wearing one name. Godot 4 has `TextServerAdvanced`: a
//! complete text server with real shaping and bidirectional support, and it is
//! correct. Godot 3 draws text through `Font::draw_char` with no shaping and no
//! bidi at all. So this crate holds two strategies that share only a package
//! format, and the probe decides which one applies before either loads.
//!
//! Built in **Phase 9**, on Phases 3 and 5.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `khadim_nusus` | the Godot 4 path: a `FontFile` created at runtime from patch bytes, theme default and named overrides, `Control` text and base direction, the advanced text server's Arabic features, and a generated `Translation` resource loaded for the `ar` locale |
//! | `istila` | the Godot 3 takeover: hook the engine's text drawing, intercept every string, lay it out through `jisr`, and draw glyphs from Taarib's atlas through `VisualServer::canvas_item_add_texture_rect_region` |
//! | `tawseel` | the Godot 3 delivery: the generated `.translation` in Godot 3's own resource format, the `override.cfg` in Godot 3's own setting namespace and list syntax, and the ladder that puts them where the engine loads them without anything being injected |
//! | `naql` | the glyph-identifier transport, for statically linked, stripped, custom-built export templates where the draw call cannot be hooked at all |
//! | `imtidad` | the `GDExtension` entry point: the one exported C function Godot 4 calls, the 4.0-against-4.1 interface ABI split, and the interface functions this adapter binds |
//! | `isdar` | which Godot this is, and therefore which of the two strategies runs — the switch the whole crate turns on |
//! | `khata` | every refusal this adapter can produce, each naming what failed and whether it ends the ladder or descends it |
//! | `pck` | package archives: PCK v1 and v2, embedded-in-executable packages with their trailing offset record, encrypted packages when a key is supplied, and the `.translation` resource format in both its plain and `OptimizedTranslation` hash-table forms |
//!
//! ## Godot 4 is not duplicated
//!
//! Where the engine's text server is right, Taarib uses it. `khadim_nusus`
//! registers a font, sets a locale, sets a direction, and stops. The delivery
//! is a `GDExtension` library plus a patch `.pck` mounted over the game's own
//! through `ProjectSettings.load_resource_pack`, which Godot supports natively
//! and which is removed by deleting one file. Nothing is hooked, nothing is
//! detoured, and nothing about the game's own rendering changes.
//!
//! ## The transport fallback, and why it is not a presentation-form pipeline
//!
//! `naql` is the last resort for a Godot 3 export template that cannot be
//! extended and cannot be hooked. It generates a `BitmapFont`/`DynamicFont`
//! replacement whose glyph table holds Taarib's own rasterized glyphs mapped
//! onto private-use codepoints, and emits translated strings as sequences of
//! those codepoints in visual order.
//!
//! That is a glyph transport, not text, and Decision 1 is intact through it.
//! The glyph identities come from real HarfRust shaping of the real logical
//! text — every ligature, every contextual alternate, every mark position that
//! the font's own `GSUB` and `GPOS` produced. The private-use codepoints carry
//! no Unicode meaning whatsoever; they are an addressing scheme for glyph ids
//! in a container that has no other way to name them. They are never written to
//! any file a user could mistake for text, never round-tripped back into the
//! string table, and the mechanism is documented in `docs/` as a transport
//! everywhere it is named. A presentation-form pipeline maps *codepoints* to
//! *codepoints* and destroys the shaping; this maps shaped *glyph ids* to
//! opaque slots and preserves it exactly.
//!
//! ## Godot 3 is two halves and needs both
//!
//! `istila` shapes **the strings the game draws**. It does not translate them,
//! and it cannot: a hook on `Font::draw` sees the text the game already decided
//! to draw, and if that text is English then `istila::yahtaj` finds no Arabic in
//! it and every interception forwards to the engine untouched. So a takeover
//! installed on an untranslated game is correct, complete and invisible.
//!
//! `tawseel` is the other half. It writes the patch's messages as a Godot 3
//! `.translation` and names it in the project override Godot reads at startup,
//! so the strings the game draws become Arabic — at which point `istila` has
//! something to shape. Neither half is useful alone, and
//! [`bidaya::HalatBidaya::Naqisa`] exists so that having one of them is reported
//! as what it is rather than as success or failure.
//!
//! ## Hard constraints
//!
//! - Godot 4 uses the engine's shaping; Taarib does not duplicate it there.
//!   Godot 3 uses Taarib's shaping entirely. There is no in-between mode.
//! - Patch packs are additive and removable. The original `.pck` is never
//!   rewritten in place, and an embedded package inside an executable is never
//!   modified — the patch pack is mounted alongside it.
//! - The transport is used only where direct drawing is impossible, and the
//!   capability report says which path was taken before the user installs.
//! - Adapters make no policy. This module renders what the patch declares.
//! - Every hook installed by `istila` is removable, and the module unloads
//!   leaving the process byte-identical.

/// The in-process bootstrap. See this crate's `hamula` feature.
#[cfg(feature = "hamula")]
pub mod bidaya;
pub mod imtidad;
pub mod isdar;
pub mod istila;
pub mod khadim_nusus;
pub mod khata;
pub mod pck;
pub mod tawseel;

/// The glyph-identifier transport, which lives in `taarib-lawha`.
///
/// It was written here, for Godot 3, and it is not a Godot fact: GameMaker
/// needs the same capability and one adapter crate cannot depend on another. It
/// therefore moved to `taarib-lawha`, beside the atlas whose images it
/// addresses, and is re-exported at its original path so that every
/// `crate::naql::...` in this crate keeps resolving.
pub use taarib_lawha::naql;

pub use crate::isdar::{Bina, Masar, afhas};
pub use crate::khata::KhataGodot;

/// The file name of the additive patch package this adapter writes.
///
/// Godot mounts a pack loaded through `ProjectSettings.load_resource_pack` over
/// its own content, so a file named this way overrides the game's resources
/// without any of them being modified. Uninstalling is deleting this one file.
pub const ISM_HAZMA: &str = "taarib.pck";

/// The locale this adapter registers and activates.
///
/// Plain `ar`, not `ar-SA`. Godot's locale fallback resolves `ar-EG` and
/// `ar-MA` to `ar`, so data filed under `ar` reaches every Arabic-speaking
/// player, while data filed under `ar-SA` is invisible to most of them. It also
/// avoids choosing one country's regional formatting on a player's behalf,
/// which is a policy decision an adapter is not allowed to make.
pub const WASM_THAQAFA: &str = "ar";
