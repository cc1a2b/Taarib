//! # محوّل تعريب لمحرّك أنريل — the Unreal adapter
//!
//! Unreal already ships a complete text shaping stack: `HarfBuzz` and ICU live
//! inside Slate, and they are correct. The strategy here is therefore not to
//! replace them but to **switch them on, feed them, and correct them**. Text
//! that Slate lays out participates in every Slate effect, material, animation
//! and widget the game already uses; text that Taarib drew over the top would
//! not. Where an engine is right, using it is the better engineering.
//!
//! Built in **Phase 8**, on Phases 3 and 5, and delivered as an injected module
//! plus an additive patch `.pak`.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `tashghil` | forcing full shaping: the Slate setting and the matching console variable, set before the first font measure call and re-asserted when the game overwrites them, plus Slate's bidirectional detection which exists but is not always enabled |
//! | `khatt` (`khatt_unreal` in the roadmap) | runtime font registration — the bundled Arabic font built into a Slate composite font with an Arabic sub-font and the right fallback chain, applied to the styles the patch names |
//! | `qiyas` (`qiyas_unreal` in the roadmap) | hooks on Slate's measurement and layout services to correct flow direction, alignment resolution and wrapping where Slate's own resolution is wrong or the game hard-coded left alignment |
//! | `mawarid` | localization resources: `.locres` v1 through v3 including the city-hash string table form, `.locmeta`, `StringTable` assets, `.pak` v1–v11 with AES-256 where a key is supplied, and UE5 IoStore `.utoc`/`.ucas` |
//! | `alam` | culture registration: add `ar`, make it active at startup, and either surface it in the game's own language menu or bypass that menu, depending on what the game's UI allows |
//! | `jahiziya` | what this build of the adapter *actually* does, capability by capability, and the one verdict a caller may gate on |
//!
//! ## Readiness is answered here, not inferred from this file
//!
//! Everything above says what a module would do to a game. [`jahiziya::jahiziya`]
//! says what happens in this package today, and it is the value anything that
//! gates on the Unreal path must read — a gate that refuses a one-button run for
//! an engine that cannot be patched is acting on it, so a hopeful answer there is
//! a user waiting for a translation that will not arrive. As this is written it
//! answers [`taarib_mustalahat::muharrik::JahiziyatTashghil::Ghaiba`]: the
//! offline half reads and writes real containers, and nothing in this package
//! puts one into a game.
//!
//! ## The one place a font is handed to an engine
//!
//! Decision 5 says Taarib never touches the game's fonts, font assets or asset
//! bundles. `khatt_unreal` registers a font into Slate's font system, and that
//! is not a violation of it: the bytes are **Taarib's own bundled font**, read
//! from the patch, registered as a new composite font. No `.uasset` font object
//! is read, modified, or replaced. Decision 5 is about the game's assets, and
//! this touches none of them.
//!
//! The corollary is that this adapter does not use `lawha`. It has no atlas,
//! because Slate rasterizes. `jisr` is still present and still used — for the
//! preview path, for measurement Taarib needs to make its own decisions from,
//! and for the versions where `qiyas_unreal` has to correct a wrapping result
//! against a width Slate got wrong.
//!
//! ## Containers are additive, always
//!
//! Reinjection writes a new patch `.pak` mounted at a higher priority than the
//! game's own — the standard Unreal modding path, understood by the engine,
//! and reversible by deleting one file. Nothing is rewritten in place. When a
//! container cannot be read — an unknown pak version, a missing AES key, an
//! Oodle-compressed IoStore chunk whose decompressor Taarib has no licence to
//! ship and cannot reach through the game's own export — the adapter says
//! exactly that and the game falls back to tier 3, rather than writing a
//! container it is guessing at.
//!
//! ## In-world text
//!
//! `UTextRenderComponent` and Slate-in-3D widgets go through the same
//! measurement and shaping corrections as the menus, so signage, floating
//! damage numbers and diegetic interfaces are Arabized along with everything
//! else. A game where the menus are Arabic and the world is not is a game that
//! looks half-finished.
//!
//! ## Hard constraints
//!
//! - No `.uasset` font object is read or replaced (Decision 5).
//! - The patch pak is additive and higher priority; original files are never
//!   modified in place.
//! - Full shaping is re-asserted defensively, because games reset console
//!   variables from their own configuration after startup and a single
//!   set-at-launch would silently stop applying.
//! - Adapters make no policy: this module renders what the patch tells it to
//!   render, checks no safety, reads no registry, and makes no network call.
//! - A failure degrades one capability with a named reason reported through the
//!   log, through `barid`, and into the game's own log. The game keeps running.

pub mod alam;
/// The in-process bootstrap. See this crate's `hamula` feature.
#[cfg(feature = "hamula")]
pub mod bidaya;
pub mod isdar;
pub mod jahiziya;
pub mod khata;
pub mod khatt;
pub mod mawarid;
pub mod qiyas;
pub mod slate;
pub mod tashghil;
pub mod wasl;

pub use crate::isdar::{Bina, Naw, Tabaa, afhas};
pub use crate::jahiziya::{HalatQudra, Qudra, hamula_mabniya, jahiziya, naqs, taqreer};
pub use crate::khata::KhataUnreal;

/// The file name of the additive patch container this adapter writes.
///
/// The `_P` suffix and the leading `zzz` are not decoration: Unreal mounts
/// `.pak` files in name order and gives `_P`-suffixed containers a higher mount
/// priority, so a file named this way overrides the game's own content without
/// any of it being modified. Uninstalling is deleting this one file, and that
/// is the entire uninstall procedure.
pub const ISM_HAWIYA: &str = "zzz_taarib_P.pak";

/// Where, relative to the game's root, the patch container is placed.
///
/// Beside the game's own containers rather than in a mod folder, because a mod
/// folder is a convention some games have and most do not, and the containers
/// directory is where the engine already looks.
pub const DALIL_HAWIYA: &str = "Content/Paks";
