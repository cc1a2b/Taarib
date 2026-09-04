//! # طبقة تعريب — the universal overlay
//!
//! Tier 3. For any game Taarib cannot patch — an unknown engine, a custom
//! renderer, a container nobody can read — hook the presentation of frames,
//! read what is on the screen, and draw shaped Arabic over it. This is the
//! fallback that makes the coverage matrix's last row say "anything else"
//! instead of "unsupported".
//!
//! It is also the tier the product is most careful about describing. The game
//! is not modified, the translation is read from the screen and may be
//! imperfect, latency exists, and the performance cost is measurable. The
//! interface says all of that in plain Arabic before the user enables it, and
//! no marketing language is used about this tier anywhere in the product.
//!
//! Built in **Phase 11**, on Phases 1, 2, 3, 5 and 13 — 13 because an overlay
//! with no translation pipeline behind it displays nothing.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `khata` | every refusal this crate can produce, and which of the three responses it maps to: skip the frame, disable the overlay, or refuse to hook |
//! | `sidq` | the tier-3 disclosure as a value the overlay cannot start without, rather than a dialog a caller is trusted to have shown |
//! | `wajiha` | the shape every backend has: the frame contract, the per-frame budget, the surface description, and the lifecycle rules that are identical on all four APIs |
//! | `khataf` | every unsafe operation this crate performs against a live process: vtable reads and writes, page protection, module presence, and unhooking that verifies itself |
//! | `d3d11` | the Direct3D 11 backend: full pipeline state save and restore around each draw, staging read-back for capture |
//! | `d3d12` | the Direct3D 12 backend: per-frame allocators, fence-synchronised resource lifetimes, the command queue captured because a swap chain cannot yield one |
//! | `gl` | the OpenGL backend and its hand-written 3.3-core loader, with the complete global-state save and restore that OpenGL alone demands |
//! | `vulkan` | the Vulkan layer: the dispatch chain, the manifest, semaphore rewiring around `vkQueuePresentKHR`, and per-image command buffers |
//! | `iltiqat_shasha` | frame capture and preprocessing: format conversion including HDR tone mapping, contrast normalisation, adaptive thresholding, upscaling, and the change-detection gate that keeps recognition off a static screen |
//! | `qira` | on-screen text recognition: Windows Runtime, macOS Vision, and the bundled portable engine, behind one trait that reports which was chosen and why |
//! | `rasm_tabaqa` | overlay rendering: shaped glyphs to premultiplied quads, and the single statement of the glyph placement convention |
//! | `talqeem` | the feeder: translated lines and panel elements in, shaped through `saff`, packed into a runtime atlas, out as the batch a backend draws |
//! | `manatiq` | regions — drawn with a mouse in an in-game editing mode, persisted per game in normalized coordinates, plus automatic detection of stable text blocks and per-region rules for when to translate |
//! | `sijill_qira` | the reading history panel, so a player who missed a line can read it again without reloading a save |
//! | `lawhat_tahakkum` | the in-game control panel: toggle, translate now, edit regions, history, opacity, font size, pause — rendered by Taarib's own renderer, depending on no UI library being present in the game |
//!
//! ## Four APIs, four honest implementations
//!
//! **Direct3D 11** — a vtable hook on `IDXGISwapChain::Present` and `Present1`,
//! with `ResizeBuffers` handled so resolution changes and fullscreen
//! transitions do not leave the overlay drawing into a dead surface.
//!
//! **Direct3D 12** — a vtable hook on `IDXGISwapChain3::Present1` plus command
//! queue capture through `ID3D12CommandQueue::ExecuteCommandLists`, with
//! per-frame descriptor heaps and fence-synchronised resource lifetimes, so the
//! overlay never races the frame the game is still building.
//!
//! **OpenGL** — `wglSwapBuffers`, `glXSwapBuffers`, or `eglSwapBuffers`, with
//! full state save and restore around the draw. OpenGL is a global state
//! machine and a game will not tolerate its state being changed underneath it;
//! anything the overlay binds, it unbinds.
//!
//! **Vulkan** — a proper layer, `VK_LAYER_taarib_tabaqa`, with its own
//! manifest, hooking `vkQueuePresentKHR` and `vkCreateSwapchainKHR`, allocating
//! from its own device memory and recording into its own command buffers on the
//! presenting queue family. A layer rather than a detour, because Vulkan has a
//! documented extension point and using it is both more correct and more
//! removable.
//!
//! ## There is no second text path
//!
//! Every glyph the overlay draws comes from `saff` through `jisr`, out of
//! Taarib's own atlas. The control panel's own labels included. A tier-3
//! renderer that reimplemented text would be the one place in the product where
//! Arabic could be wrong in a way nothing else catches, so it does not exist.
//!
//! ## Hard constraints
//!
//! - The overlay never changes game state and never writes into the game's
//!   memory outside its own hook trampolines.
//! - Every hook is removable at runtime and the module unloads cleanly.
//! - Frame time cost is measured and displayed in the control panel. A fallback
//!   that silently halves someone's frame rate is not honest.
//! - Capture is region-limited and rate-limited, and format conversion happens
//!   on the GPU where the API allows it.
//! - On macOS, where injection into a hardened-runtime process is refused by
//!   the system, the capability report says so before the user tries and offers
//!   the window-capture path instead.
//! - The honest description of tier 3 is shown before the overlay is enabled,
//!   every time it is enabled for a new game. This one is enforced by the type
//!   system rather than by discipline: [`wajiha::Tabaqa::shaghghil`] takes a
//!   [`sidq::Iqrar`] by value, [`sidq::Iqrar`] has no constructor but the one
//!   that requires the fingerprint of the text that was displayed, and there is
//!   no other way to start an overlay. Skipping the disclosure means deleting a
//!   parameter from a public signature, which is visible in a review.
//!
//! ## Where the unsafe lives
//!
//! In `khataf`, and — for the four backends — in the calls each makes into its
//! own graphics API. `khataf` owns everything done *to the process*: reading a
//! method table, changing page protection, writing a function pointer,
//! restoring it. The backends own only what is done *through a device the game
//! already created*. Neither category is spread across the other, so "what does
//! the overlay do to somebody's game?" has one file as its answer.

/// The in-process bootstrap. See this crate's `hamula` feature.
#[cfg(feature = "hamula")]
pub mod bidaya;
pub mod iltiqat_shasha;
pub mod khata;
pub mod khataf;
pub mod lawhat_tahakkum;
pub mod manatiq;
pub mod qira;
pub mod rasm_tabaqa;
pub mod sidq;
pub mod sijill_qira;
pub mod talqeem;
pub mod wajiha;

/// The OpenGL backend, on every platform that has one.
pub mod gl;

/// The Vulkan layer and backend, on every platform that has one.
pub mod vulkan;

/// The Direct3D 11 backend.
#[cfg(windows)]
pub mod d3d11;

/// The Direct3D 12 backend.
#[cfg(windows)]
pub mod d3d12;

pub use crate::khata::KhataTabaqa;
pub use crate::sidq::{
    BasmatIfsah, ISM_MALAF_IQRAR, Iqrar, NASS_IFSAH_ARABI, NASS_IFSAH_INJILIZI,
};
pub use crate::talqeem::{IhsaatTalqeem, KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
pub use crate::wajiha::{
    HalatTabaqa, Khattaf, LawhatRasm, MeezaniyatItar, MustatilBiksel, MustatilNisbi, QitaRasm,
    SighatSath, Tabaqa, WajihatRusum, WasfSath,
};

/// The directory, under the platform's data directory, that holds this tier's
/// per-game state.
///
/// Regions and reading history, one subdirectory per game. Separate from the
/// patch storage the other tiers use because nothing here patches anything:
/// deleting this directory loses a player's regions and their history and
/// touches no game.
pub const DALIL_TABAQA: &str = "tabaqa";

// The region set and the reading history are named by the modules that read
// and write them, and re-exported here rather than spelled again. They were
// spelled again, and the second copy had already drifted: this constant said
// `sijill.jsonl` while `sijill_qira` has always written `sijill_qira.jsonl`, so
// a caller reaching for the crate-root name would have looked for a file
// nothing writes and found no reading history at all.
pub use crate::manatiq::ISM_MALAF as MALAF_MANATIQ;
pub use crate::sijill_qira::ISM_MALAF as MALAF_SIJILL;

/// The Vulkan layer's name, as the loader enumerates it.
///
/// Declared here rather than only in `vulkan` because the installer writes the
/// manifest that carries it and the uninstaller deletes it, and a name that
/// lived in one module would be a name spelled twice.
pub const ISM_TABAQAT_VULKAN: &str = "VK_LAYER_taarib_tabaqa";
