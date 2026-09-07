//! # جسر تعريب — the stable C ABI
//!
//! The only way into the engine from outside Rust. C#, C++, Python, Ruby and
//! JavaScript all reach `saff` and `lawha` through this one narrow, versioned,
//! ownership-explicit surface. There is no second entry point, and no
//! engine-specific extension of it — an adapter that needed a private ABI call
//! would be an adapter making policy, which adapters do not do.
//!
//! Built in **Phase 3**, on top of Phases 1 and 2, and shipped as
//! `taarib_jisr.dll` / `libtaarib_jisr.so` / `libtaarib_jisr.dylib` for `x86_64`
//! and aarch64 on all three platforms, plus i686 on Windows and Linux because a
//! 32-bit game process needs a 32-bit library and the injector selects by the
//! target image rather than by the host.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `hayat` | the opaque handles — context, font, chain, atlas — each generation-tagged so a use-after-free is detected and reported instead of executed |
//! | `dhakira` | caller-owned buffers: the layout write path, capacity negotiation, and the pool of reusable layout buffers |
//! | `khazina` | the layout cache, keyed by the full shape of the request and bounded by a byte budget rather than an entry count |
//! | `khata_c` | the status code space, and the mapping from a `Khata` value into a code plus a retrievable Arabic and English sentence |
//!
//! ## The shape of every entry point
//!
//! Every function is `extern "C"` and unmangled, returns an `int32` status, and
//! writes its results through out-parameters. No Rust type crosses the
//! boundary. `taarib_abi_isdar()` returns a major and a minor, and a caller
//! whose major does not match refuses to load and says why in the host's own
//! log rather than continuing into undefined behaviour.
//!
//! Every entry point wraps its body in `catch_unwind`. A panic that unwound
//! into a game's C# or C++ frame would be a crash the player blames on the
//! game, so a panic here becomes a status code and a logged diagnostic — which
//! is also why the release profile for this workspace uses `panic = "unwind"`
//! rather than `abort`.
//!
//! ## The hot path
//!
//! `taarib_takhtit(siyaq, talab, makhzan)` writes positioned glyphs into a
//! caller-owned, reusable buffer and allocates nothing. When the buffer is too
//! small it returns the required capacity and writes nothing, so the caller
//! grows once and retries. There is deliberately no allocating variant of this
//! function anywhere in the surface: an adapter cannot accidentally choose the
//! convenient one on a render path if the convenient one does not exist.
//!
//! `khazina` sits behind it. The key is the hash of `(text, font identity,
//! pixel size, available width, feature set, style span shape, justification
//! mode, language)`, and the value is the finished layout — so a menu that
//! redraws every frame shapes once, and a typewriter effect that reshapes its
//! visible prefix on every step costs nothing after the first pass. The budget
//! is in bytes because a cache measured in entries is a cache that eventually
//! holds a thousand paragraphs.
//!
//! ## The generated header
//!
//! `include/taarib.h` is produced by cbindgen from a `build.rs` and committed
//! to the repository, so a consumer does not need the Rust toolchain to bind
//! against it. It is generated, never hand-edited, and it carries the ownership
//! contract per function in prose: who allocates, who frees, how long a
//! returned pointer stays valid, and whether the next call invalidates it.
//! `docs/abi.md` carries the same contract in longer form.
//!
//! ## Hard constraints
//!
//! - No allocation on the layout path, enforced structurally rather than by
//!   discipline: the layout is built into a pooled buffer that already owns its
//!   storage, the caller's arrays are the caller's, and there is no entry point
//!   that returns an allocation for somebody to free.
//! - No global mutable state except the context registry, which is a table
//!   behind one lock; the cache lives inside a context rather than beside it,
//!   because a recency update makes every read a write and a cache that had to
//!   be shared would need a lock of its own on the render path.
//! - Thread safety is stated per function and honoured: a context is `Send`,
//!   layout is safe from one thread at a time, and font resources are shared
//!   immutably across contexts.
//! - Streaming entry points — runtime string capture for Phase 12 and atlas
//!   growth callbacks for Phase 2 — are non-blocking and safe to call from a
//!   render thread.
//! - Every `unsafe` block carries a written invariant naming what is guaranteed
//!   and by whom. Being an FFI crate is not an exemption from that; it is the
//!   reason for it.
//!
//! ## Reading this crate
//!
//! Start at [`awamir`]: it is the surface every consumer actually calls, and
//! every function there carries its own ownership contract in prose. [`anwa`] is
//! the frozen type layout those functions read and write, and is the file to
//! check against when a binding in another language disagrees about a field.
//! The other three modules are how the promises in [`awamir`] are kept —
//! [`hayat`] makes a stale handle detectable, [`dhakira`] makes the layout path
//! allocation-free, and [`khazina`] makes a repeated string cost nothing.
//!
//! The Rust API is public as well as the C one. `taarib-wasm` and the Rust-side
//! adapters link this crate directly as an `rlib` and use the same handle model
//! and the same cache without going out through C — because two paths into one
//! engine would be two sets of behaviour to keep identical, and they would not
//! stay identical.

pub mod anwa;
pub mod awamir;
pub mod dhakira;
pub mod hayat;
pub mod khata_c;
pub mod khazina;

pub use crate::anwa::{
    TaaribHarf, TaaribIhsaatLawha, TaaribKhatt, TaaribKhiyarat, TaaribKhiyaratSiyaq, TaaribLawha,
    TaaribMakhzanTakhtit, TaaribMawdiShakl, TaaribMiftahShakl, TaaribNitaqUslub, TaaribQiyasNass,
    TaaribQiyasatKhatt, TaaribSafha, TaaribSatr, TaaribSifa, TaaribSilsila, TaaribSiyaq,
    TaaribTalab, TaaribTaqreerTajawuz,
};
pub use crate::awamir::{
    TAARIB_ABI_KABIR, TAARIB_ABI_SAGHEER, TaaribIhsaatKhazina, TaaribIltiqatFn,
};
pub use crate::hayat::{QanatIltiqat, Siyaq, TaaribRaddIltiqat};
pub use crate::khazina::{IhsaatKhazina, Khazina, MiftahTakhtit};

// No symbol-retention anchor is needed here, and one would be a liability. In a
// `cdylib`, `#[unsafe(no_mangle)]` already makes a symbol part of the library's
// public interface, so the compiler emits it and the linker keeps it even though
// nothing in Rust refers to it — every caller is on the other side of the
// boundary by construction. The `static` of function pointers that this file
// would otherwise carry to "keep the surface alive" would add an `unsafe impl
// Sync`, a const function-pointer cast, and a second thing to keep in step with
// `awamir`, in exchange for a guarantee the crate type already gives.
