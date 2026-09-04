//! # حقن تعريب — getting inside, and hooking once there
//!
//! The three ways Taarib redirects a function inside a game process, factored
//! out of every payload that needs them. The Unreal takeover, the Godot 3
//! takeover, the script-engine loaders and the universal overlay all use these;
//! none of them reimplements a trampoline, a protection dance, or an import
//! walk.
//!
//! Generalised in **Phase 23** from the graphics layer's own hooking, which is
//! the implementation that was proven first and is the reason the refusal
//! discipline below reads the way it does.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `khatf` | trampoline hooking: a detour at an absolute address, and the trampoline that reaches the original |
//! | `jadwal` | virtual table hooking: replace a slot, and put it back only when it is still ours |
//! | `istirad` | import table redirection: repoint what one module calls without touching what it is |
//! | `mawqi` | where this module is, where the game's modules are, and their exported symbols |
//! | `hirasa` | the write guard and the verified pointer write the other three share |
//! | `khata` | every refusal, in band 4200 |
//!
//! ## Loading by override beats loading by force
//!
//! Wherever a game will load Taarib's module on its own — a proxy the engine
//! already imports, a Wine override, a launch option, a documented extension
//! point — that is the path taken. `taarib-tathbeet`'s `tarkib` module writes
//! those at install time, and `taarib-mudkhal` is the module they bring in.
//! This crate is what happens *after* that module is inside: it does not
//! inject, and there is no remote-thread path here at all. A game loaded
//! through its own loader starts in a known state, unloads cleanly, and is
//! undone by deleting one file.
//!
//! ## Hooking is a contract, not a trick
//!
//! Every hook records what it replaced and can put it back. Three rules make
//! that real rather than aspirational:
//!
//! - **Write, then read back.** A page that reports as writable and silently
//!   discards writes is what a hypervisor-based anti-tamper product does, and a
//!   hook that believed its own write would call a thunk that was never
//!   installed. Every write in this crate is verified by reading it back.
//! - **Refuse a moved target.** A location that no longer holds what it held a
//!   moment ago means something else patched it in between. That is a named
//!   refusal, never an overwrite.
//! - **Never unhook someone else.** If a slot no longer holds *this* hook, some
//!   other overlay took it afterwards. Writing the saved original over that
//!   would leave the other overlay calling into a thunk about to be unmapped,
//!   so the situation is reported and the slot is left alone. There is no
//!   correct fix from inside the process: whoever hooked last has to unhook
//!   first, and Taarib cannot make them.
//!
//! ## Hard constraints
//!
//! - Nothing here is a cheat, a bypass, or an anti-tamper defeat. `taarib-aman`
//!   refuses outright to install into any game with anti-cheat present, and
//!   this crate is never reached for such a game. That refusal has no override
//!   path anywhere in the product.
//! - Every hook is removable at runtime and every guard restores the protection
//!   it took. A resource without a drop path does not ship.
//! - Nothing is written into game memory outside a trampoline `retour`
//!   allocated and the pointer slots named by the caller.
//! - Every `unsafe` block carries a written invariant naming what is guaranteed
//!   and by whom, checked at the boundary where it can still be checked.
//! - A failure to hook degrades that one capability with a specific reason and
//!   leaves the game running in its original language. It never crashes the
//!   game and never produces a half-patched frame.

pub mod hirasa;
pub mod istirad;
pub mod jadwal;
pub mod khata;
pub mod khatf;
pub mod mawqi;

pub use hirasa::{HirasatKitaba, iktub_muashir};
pub use istirad::{IstiradMakhtuf, fukk_istirad, ikhtif_istirad};
pub use jadwal::{KhatfJadwal, jadwal_min_wajiha};
pub use khata::{KhataHaqn, NatijatHaqn};
pub use khatf::Masar;
pub use mawqi::{masar_nafsi, mujallad_nafsi, qaidat_wahda, ramz_wahda};
