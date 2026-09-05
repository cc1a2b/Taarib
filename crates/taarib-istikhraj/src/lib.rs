//! # استخراج تعريب — pulling the strings out
//!
//! Every translatable string a game has, from static containers where they can
//! be read and from the running game where they cannot, normalised into one
//! table with enough context attached that a translator can do the job well and
//! a compiler can write the result back to the exact place it came from.
//!
//! This is the one place in the product that reads game assets at all. It is
//! read-only, it is never on the rendering path, and it degrades to runtime
//! capture when a container refuses to be parsed — which is the whole reason
//! Decision 5 can hold everywhere else.
//!
//! Built in **Phase 12**, on Phases 0, 4 and 5, plus the adapters for the
//! capture channel.
//!
//! ## Modules
//!
//! | module | what it reads |
//! | --- | --- |
//! | `unity` | `UnityFS` bundle framing in every compression mode, `SerializedFile` headers across format versions, the type tree, `TextAsset` (49) and `MonoBehaviour` (114) walked through the type tree, `GameObject`/`Transform` for context paths, Unity Localization string tables, I2 Localization `LanguageSource`, plus the IL2CPP `global-metadata.dat` literal heap and the Mono `#US` user-string heap |
//! | `unreal` | `.locres` in every version, `.locmeta`, and `StringTable` assets, from loose files and from `.pak` and IoStore containers, with namespace and key structure preserved exactly |
//! | `godot` | `.translation` resources plain and optimized, `.po` files, and scene and resource text properties from `.scn` and `.tres` inside a `.pck` |
//! | `nusus` | RPG Maker JSON event command lists with their codes kept as context, VX Ace `Marshal` object graphs, Ren'Py `.rpy` parsing and `.rpa` archives, GameMaker `STRG` cross-linked to its code references, and asar contents including JSON, JS literals and HTML text nodes |
//! | `qamus` | Capcom's `DICT` dictionary — the header, the bucket array, the self-relative pointers and the CRC-32 the game hashes its keys with — read, written back byte for byte, and rebuilt |
//! | `iltiqat` | runtime capture over `barid`: every string an adapter drew, with scene, component path, screen rectangle, font size and a screenshot crop |
//! | `jadwal` | the normalized string table that all of the above produce |
//!
//! ## Every reader is written here, on purpose
//!
//! There is no third-party game-asset library anywhere in this crate. Every
//! container reader — `UnityFS`, `SerializedFile`, the type tree, pak, IoStore,
//! PCK, RGSSAD, FORM, asar — is written in this repository. Those libraries
//! exist and some of them are good, but all of them are serialized against
//! exact engine versions, and every one of them breaks on a version bump that a
//! game will ship next month. Owning the readers means a failure is a failure
//! we can diagnose and fix in an afternoon rather than a dependency we wait on.
//!
//! It also means every reader can fail *narrowly*. An object that cannot be
//! walked reports the container, the object and the reason, and the next object
//! is read. A container that cannot be opened at all reports itself and the
//! session continues with runtime capture for the strings it would have held.
//!
//! ## Noisy sources are ranked, not hidden
//!
//! IL2CPP's literal heap and Mono's `#US` heap contain every string constant in
//! the assembly: dialogue, shader names, log messages, format specifiers,
//! reflection keys. They are extraction sources of last resort and they produce
//! a great deal of rubbish alongside real text. They are read, ranked lower,
//! and filtered by heuristics — and Phase 13's review surfaces what the
//! heuristics did rather than silently discarding it, because a filter that
//! quietly drops the one line a translator was looking for is worse than noise.
//!
//! ## What a `jadwal` entry carries
//!
//! A `NassId` derived deterministically from source location plus source text.
//! The source text exactly as stored, markup included. Context: the container,
//! the object path, the field name, the event command code, the speaker where
//! one is identifiable, and the surrounding lines for dialogue. A source
//! location precise enough to write back to. Constraints: the maximum character
//! count where the engine imposes one, and the maximum pixel width measured
//! from the original's own rendered width and the rect runtime capture saw it
//! drawn into. Detected markup and placeholders as `nasq` spans, extracted at
//! import so translation never sees a raw tag. An occurrence count and where it
//! appeared in play. And a duplicate group id, so one translation serves every
//! identical source string in the game.
//!
//! ## Hard constraints
//!
//! - Extraction never writes into the game directory.
//! - Failures are per-container and per-object, never fatal to the session, and
//!   always carry the container name and the reason.
//! - No original game asset extracted here ever reaches a patch. Phase 14's
//!   asset gate proves that mechanically rather than trusting this sentence.
//! - String identity is deterministic: extracting the same build twice produces
//!   the same ids, so a re-extraction after a game update diffs cleanly instead
//!   of looking like a total rewrite.

pub mod dammij;
pub mod godot;
pub mod iltiqat;
pub mod jadwal;
pub mod khata;
pub mod mashru;
pub mod nusus;
pub mod qamus;
pub mod rafd;
pub mod tasnif;
pub mod tawjih;
pub mod unity;
pub mod unreal;

pub use crate::jadwal::{FarqJadwal, JadwalNusus, MawqiNass, MudkhalMustakhraj};
pub use crate::khata::KhataIstikhraj;
pub use crate::qamus::{MudkhalQamus, Qamus, basmat_miftah};
pub use crate::rafd::{SababRafd, TaqreerRafd};
pub use crate::tawjih::{Mustakhrij, istakhrij};
