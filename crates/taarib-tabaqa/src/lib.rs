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
//! | `wajiha` | the shape every backend has: the frame contract, the per-frame budget, the surface description, and the lifecycle rules that are identical on all five APIs |
//! | `khataf` | every unsafe operation this crate performs against a live process: vtable reads and writes, page protection, module presence, and unhooking that verifies itself |
//! | `istitlaa` | what a game's own files say before anything is installed into them: which graphics modules its executable imports, and which proxy slots a third party has already taken — including the one Taarib itself would take, which it refuses to take twice |
//! | `d3d9` | the Direct3D 9 backend: a `D3DSBT_ALL` state block around a fixed-function draw, `Reset` handled so a lost device does not take the game with it, and the sRGB transfer function moved onto the processor because there is no shader to put it in |
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
//! | `tatabbu` | a line of dialogue's identity across frames, and the moment its text stops moving — so a subtitle on screen for two hundred frames is one string and one translation rather than two hundred |
//! | `watira` | the refresh governor: when the per-frame budget is exceeded the overlay gives up how often it reads the screen, never the frames it draws, and says so |
//! | `mutarjim` | the two seams to a translation provider and to the shared translation memory, taken as trait objects so no HTTP stack or database is linked into a game's process |
//! | `qissa` | the session: capture on the frame, recognize and translate off it, draw the most recent completed result |
//!
//! ## Five APIs, five honest implementations
//!
//! **Direct3D 9** — a vtable hook on `IDirect3DDevice9::Present` and `Reset`,
//! plus `PresentEx` and `ResetEx` when the game made a 9Ex device. This is the
//! backend for roughly 2002 to 2012, which is the densest era of story-driven PC
//! games and was until now the one row of the coverage matrix the universal
//! overlay could not reach either. It is not a translation of the D3D11 backend:
//! D3D9 predates the fully programmable pipeline, so the draw is fixed-function
//! texture stages rather than a shader, the save is the API's own state block
//! rather than an enumerated list — because a `D3DCREATE_PUREDEVICE` game
//! answers no `Get` call and an enumerated list would silently read nothing —
//! and the device can be *lost* on an alt-tab, which nothing newer does.
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
//! ## A game the overlay cannot draw over says so before it is installed
//!
//! Every one of the five can fail to attach, and the tier's whole posture is
//! that a failure is a capability report rather than a crash. [`qudra`] is the
//! vocabulary that report is written in, and it has two producers.
//!
//! [`istitlaa`] produces one from a game's files rather than from a process, so
//! "this game imports `d3d9.dll` and something else already owns the proxy slot
//! Taarib would take" is an answer the installer gives before it writes
//! anything, not a line in a log the player finds after the overlay silently did
//! nothing. Each backend produces the other half from the live device — see
//! [`d3d9::KhattafD3D9::qudra`], where whether the game made a losable device
//! and whether its backbuffer can be read at all are first knowable.
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
//! - Taarib never takes a proxy slot another product has taken, and never
//!   restores a function pointer it did not install. The first is [`istitlaa`],
//!   answered before installation; the second is [`khataf::Khataf::fukk`],
//!   which reads a slot back and leaves it alone when it holds somebody else's
//!   hook. Games of the Direct3D 9 era attract third-party proxies, so both
//!   rules are enforced rather than assumed.
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
//! In `khataf`, and — for the five backends — in the calls each makes into its
//! own graphics API. `khataf` owns everything done *to the process*: reading a
//! method table, changing page protection, writing a function pointer,
//! restoring it. The backends own only what is done *through a device the game
//! already created*. Neither category is spread across the other, so "what does
//! the overlay do to somebody's game?" has one file as its answer.

/// The in-process bootstrap. See this crate's `hamula` feature.
#[cfg(feature = "hamula")]
pub mod bidaya;
pub mod iltiqat_shasha;
pub mod istitlaa;
pub mod khata;
pub mod khataf;
pub mod lawhat_tahakkum;
pub mod manatiq;
pub mod mutarjim;
pub mod qira;
pub mod qissa;
pub mod qudra;
pub mod rasm_tabaqa;
pub mod sidq;
pub mod sijill_qira;
pub mod talqeem;
pub mod tatabbu;
pub mod wajiha;
pub mod watira;

/// The provider seam wired onto `taarib-tarjama`.
///
/// See this crate's `tarjama` feature: it is off for the payload, which links
/// no HTTP stack.
#[cfg(feature = "tarjama")]
pub mod wasil_tarjama;

/// The OpenGL backend, on every platform that has one.
pub mod gl;

/// The fixed-function OpenGL backend, on every platform that has one.
///
/// A second backend rather than a branch in the first: a pre-shader context has
/// none of the fifty-five entry points `gl` resolves, and it has a matrix
/// stack, a texture environment and an attribute stack that `gl` has never
/// heard of. [`gl_thabit::ikhtar`] is what decides between them, by asking the
/// context rather than the module list.
pub mod gl_thabit;

/// The Vulkan layer and backend, on every platform that has one.
pub mod vulkan;

/// The Direct3D 9 backend.
#[cfg(windows)]
pub mod d3d9;

/// The Direct3D 11 backend.
#[cfg(windows)]
pub mod d3d11;

/// The Direct3D 12 backend.
#[cfg(windows)]
pub mod d3d12;

/// The Direct3D 8 backend, and the ninety-six-slot method table it verifies
/// before it trusts.
///
/// Not gated to Windows, unlike the three Direct3D backends above it, and the
/// reason is worth stating: nothing binds Direct3D 8, so every interface in
/// that module is declared in it, and the only Windows-specific code left is
/// the pair of functions that open `d3d8.dll` and make a throwaway device. The
/// rest — the method table, the vertex packing, the colour conversion, the
/// state sequencing and the hook bookkeeping — compiles anywhere, which is what
/// lets the hook install, the unhook refusal and the table verification be
/// exercised against a stub table on a machine with no Direct3D at all.
pub mod d3d8;

/// The Direct3D 10 backend, which installs no hook of its own.
///
/// DXGI owns the swap chain on both the tenth and the eleventh generation, so
/// `d3d11`'s `IDXGISwapChain::Present` hook catches a Direct3D 10 game
/// unchanged; only the device the swap chain hands back differs. This module is
/// the backend built from that device and nothing else.
#[cfg(windows)]
pub mod d3d10;

pub use crate::khata::KhataTabaqa;
pub use crate::sidq::{
    BasmatIfsah, ISM_MALAF_IQRAR, Iqrar, NASS_IFSAH_ARABI, NASS_IFSAH_INJILIZI,
};
pub use crate::mutarjim::{
    DhakiraJalsa, DhakiraJalsaMushtaraka, DhakiraTabaqa, MutarjimTabaqa, QaydTabaqa, RaddSatr,
    TalabSatr,
};
pub use crate::qissa::{
    HalatDaf, HalatKhayt, KhaytQissa, KhiyaratQissa, LaqtaTarjama, Munassiq, Qissa,
};
pub use crate::talqeem::{IhsaatTalqeem, KhiyaratTalqeem, Mulaqqim, SatrMulaqqam};
pub use crate::tatabbu::{HalatSatr, MuarrifSatr, Mutatabbi, QiraaMulahaza, SiyasatIstiqrar};
pub use crate::watira::{MunazzimWatira, TaghyeerWatira};
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
