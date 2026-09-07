//! الأوامر — the exported surface: every function a non-Rust consumer can call.
//!
//! This file is the whole of boundary 2: C#, C++, Python, Ruby and JavaScript
//! reach the engine through these functions and through nothing else. Every one
//! of them is `extern "C"` and unmangled, returns an `i32` status from
//! [`crate::khata_c`], and writes its results through out-parameters, so a
//! caller in any language can write `if (r < 0)` and be right.
//!
//! Three habits hold across the file, uniformly, because a boundary that is
//! usually safe is not one:
//!
//! - **Every body runs inside the panic shield `ihmi`**, which converts a
//!   panic into [`TAARIB_INHIYAR`] instead of letting it unwind into a game's
//!   C# or C++ frame. One helper rather than thirty hand-written wrappers, so
//!   it cannot be forgotten on the one function where it mattered.
//! - **Every exit keeps the status and the stashed diagnosis in agreement**:
//!   failures pass through [`sajjil`] or [`sajjil_ramz`] at the point of
//!   failure, and every success clears the stash through the shared tail
//!   `khitam`. The `taarib_khata_*` functions are the one deliberate
//!   exception — they are observers of the stash and must not destroy what
//!   they report.
//! - **Every function takes the context lock at most once**, through a single
//!   [`maa_siyaq`] call, and nothing inside the lock re-enters this surface.
//!   The one piece of caller code that ever runs under the lock — the capture
//!   sink — is registered under a written contract that it returns promptly
//!   and never calls back in; every other closure is this library's own.
//!
//! The hot path is [`taarib_takhtit`]. On a cache hit it copies a finished
//! layout into the caller's buffer and touches the allocator zero times; on a
//! miss it lays out into the context's pooled buffer, hands the result to the
//! caller and to the cache, and the next identical request is a hit. There is
//! deliberately no allocating variant of it anywhere in this surface.

use std::cell::RefCell;
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::namu::LawhaHayya;
use taarib_lawha::rasf::{AQSA_SAFAHAT_IFTIRADI, BUD_IFTIRADI, KhiyaratRasf};
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_saff::natija::TakhtitNass;
use taarib_saff::talab::{Dharra, KhiyaratTakhtit, NitaqUslub, SifaIdafiya, TalabTakhtit, Uslub};

use crate::anwa::{
    TAARIB_KHIYAR_HIWAR, TAARIB_KHIYAR_SATR_WAHID, TAARIB_USLUB_DHARRA, TAARIB_USLUB_HAJM,
    TAARIB_USLUB_KHATT, TAARIB_USLUB_LAWN, TAARIB_USLUB_MAAIL, TAARIB_USLUB_WAZN,
    TaaribIhsaatLawha, TaaribKhatt, TaaribKhattKhas, TaaribKhiyaratSiyaq, TaaribLawha,
    TaaribLawhaKhas, TaaribMakhzanTakhtit, TaaribMawdiShakl, TaaribMiftahShakl, TaaribQiyasNass,
    TaaribQiyasatKhatt, TaaribSafha, TaaribSilsila, TaaribSilsilaKhas, TaaribSiyaq, TaaribTalab,
    arqam_min_raqm, dabt_min_raqm, ittijah_min_raqm, lugha_min_raqm, muhadhaha_min_raqm,
    tajawuz_min_raqm, tashkeel_min_raqm,
};
use crate::dhakira::{iktub_takhtit, iqra_nass, iqra_shariha};
use crate::hayat::{
    QanatIltiqat, Siyaq, ihdhif_siyaq, ila_muashir, maa_siyaq, min_muashir, sajjil_siyaq,
};
use crate::khata_c::{
    TAARIB_INHIYAR, TAARIB_MAQBAD_BATIL, TAARIB_MUASHIR_BATIL, TAARIB_NAJAH, TAARIB_QEEMA_BATILA,
    akhir_khutwa, akhir_nass, akhir_ramz, akhir_ramz_nass, iktub_nass, maa_akhir_khata, nazzif,
    sajjil, sajjil_ramz,
};
use crate::khazina::MiftahTakhtit;

// ---------------------------------------------------------------------------
// The ABI version
// ---------------------------------------------------------------------------

/// The ABI's major version.
///
/// Bumped when anything already shipped changes shape or meaning: a structure
/// field moved, a status code repurposed, a contract reworded in a way an old
/// caller would violate. A caller whose compiled-in major differs from the one
/// [`taarib_abi_isdar`] reports must refuse to continue and say so in its own
/// log; continuing past a major mismatch is undefined behaviour by definition,
/// because the two sides no longer agree what the bytes between them mean.
pub const TAARIB_ABI_KABIR: u32 = 1;

/// The ABI's minor version.
///
/// Bumped when something is appended — a new function, a new constant, a new
/// structure. Everything an old caller compiled against keeps working, so a
/// minor mismatch in either direction is not a reason to refuse; it is only a
/// reason not to call what the running library does not have.
pub const TAARIB_ABI_SAGHEER: u32 = 0;

/// The sink [`taarib_iltiqat_shaghghil`] registers for runtime string capture:
/// [`TaaribRaddIltiqat`], made optional so that the null function pointer is
/// representable and refused rather than called.
///
/// The sink is invoked as `radd(mustakhdim, nass, tul)`: the registered opaque
/// pointer first, then the UTF-8 text of a layout request — `tul` bytes, not
/// NUL-terminated, borrowed for the duration of the invocation only, so a sink
/// that wants the text copies it before returning.
///
/// It runs on whichever thread called [`taarib_takhtit`], while that context's
/// lock is held — which is what makes capture free to leave on, and which is
/// why the sink's contract has two halves the host must keep: return promptly,
/// because a frame is waiting, and never call back into this library on any
/// handle, because the lock it would need is the lock it is running under.
///
/// The signature is written out here rather than as `Option<TaaribRaddIltiqat>`
/// for one concrete reason: cbindgen only recognises `Option<T>` as a nullable
/// pointer when `T` is a *literal* pointer or function-pointer type, not when it
/// is a path alias. Written the short way, the generated header gets an opaque
/// `struct Option_TaaribRaddIltiqat` passed by value and no function-pointer
/// type at all — a declaration that compiles and cannot be called, which is the
/// worst kind of ABI defect because the compiler is satisfied and the consumer
/// is stuck. It stays byte-identical to [`TaaribRaddIltiqat`]; keep them so.
pub type TaaribIltiqatFn =
    Option<unsafe extern "C" fn(mustakhdim: *mut c_void, nass: *const u8, tul: usize)>;

/// What the layout cache has been doing.
///
/// Mirrors [`crate::khazina::IhsaatKhazina`] field for field. It is defined
/// here rather than in `anwa.rs` only because it belongs to the cache, and the
/// cache's surface — [`taarib_makhzan_ihsaat`] and this structure — was
/// appended to the ABI as one piece; `anwa.rs` carries the frozen Phase 3 core
/// and appending here keeps the append visible as an append. The layout rules
/// are the same ones `anwa.rs` states: widest fields first, no implicit
/// padding, nothing whose representation the compiler chooses.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaaribIhsaatKhazina {
    /// Layout requests served from the cache without shaping anything.
    pub isabat: u64,
    /// Requests that had to be laid out in full.
    pub ikhfaqat: u64,
    /// Layouts evicted to stay inside the byte budget.
    pub ikhlaat: u64,
    /// What the cached layouts currently cost, in bytes.
    pub bayt: u64,
    /// The byte budget the context was created with. Zero means the cache is
    /// disabled and every request shapes.
    pub mizaniya: u64,
    /// How many layouts the cache holds right now.
    pub madkhalat: u32,
    /// Padding to a multiple of eight. Always written as zero; ignore it.
    pub hashw: u32,
}

// ---------------------------------------------------------------------------
// The boundary discipline, in three private helpers
// ---------------------------------------------------------------------------

/// How many fonts a chain can hold, mirrored from `taarib-saff`, where
/// [`SilsilatKhutut::jadeeda`] refuses anything longer because a positioned
/// glyph names its font in a `u8`. The mirror exists so the cache-key identity
/// buffer can live on the stack: 256 entries is two kilobytes, and a fixed
/// upper bound is what lets the hot path collect font identities without ever
/// touching the allocator.
const AQSA_KHUTUT_SILSILA: usize = 256;

/// Runs one entry point's body, converting a panic into [`TAARIB_INHIYAR`].
///
/// Every exported function routes through this — one helper rather than thirty
/// hand-written wrappers, so the wrapper cannot be missing from exactly the
/// function that needed it. A panic that unwound past the boundary into a
/// game's C# or C++ frame would be a crash the player blames on the game;
/// caught here it becomes a status code, a retrievable diagnosis, and a game
/// that is still running to report the bug.
///
/// `AssertUnwindSafe` is asserted rather than proven, and the assertion is
/// honest: the closures capture raw pointers and touch state behind
/// `parking_lot` locks, which do not poison. What a caller may observe after a
/// caught panic is exactly what the contract already says about every failure
/// — out-parameters are unspecified unless the function documents otherwise,
/// and the only thing guaranteed is the returned status and the stash behind
/// it.
fn ihmi(amal: impl FnOnce() -> i32) -> i32 {
    match catch_unwind(AssertUnwindSafe(amal)) {
        Ok(ramz) => ramz,
        Err(_) => sajjil_ramz(TAARIB_INHIYAR),
    }
}

/// The uniform tail of every entry point: reconciles the returned status with
/// the thread's stash so the two can never disagree.
///
/// Success clears the stash. A failure that was already recorded at its site —
/// [`sajjil`] with the full [`taarib_usus::khata::Khata`], or [`sajjil_ramz`]
/// with a bare code — is left exactly as recorded, because re-stashing the
/// bare code would destroy the sentence. A failure that arrives here
/// unrecorded (a negotiation code out of [`iktub_takhtit`], for instance) is
/// recorded now, so the caller's `taarib_khata_akhir` always matches what this
/// call returned.
fn khitam(ramz: i32) -> i32 {
    if ramz == TAARIB_NAJAH {
        nazzif();
    } else if akhir_ramz() != ramz {
        let _ = sajjil_ramz(ramz);
    }
    ramz
}

/// Runs a string-writing negotiation without letting it destroy the stash it
/// is reporting.
///
/// [`iktub_nass`] records its own failures — a null buffer, a short buffer —
/// which is right everywhere except inside the `taarib_khata_*` observers,
/// where the failure being retrieved is the very thing that recording would
/// overwrite: a caller probing for the required capacity would wipe the error,
/// retry with a grown buffer, and read back an empty string. So the stash is
/// snapshotted first and put back afterwards when the negotiation failed. The
/// negotiation's own code is still *returned*, so the grow-and-retry loop
/// works; it is simply not allowed to become the last error.
fn maa_hifz_alkhata(amal: impl FnOnce() -> i32) -> i32 {
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "`Option::cloned` as a path pins one lifetime, and this callback must be \
                  higher-ranked over the stash borrow; only a closure can be"
    )]
    let mahfuz = maa_akhir_khata(|khata| khata.cloned());
    let ramz_mahfuz = akhir_ramz();
    let natija = amal();
    if natija != TAARIB_NAJAH {
        match mahfuz.as_ref() {
            Some(khata) => {
                let _ = sajjil(khata);
            },
            None => {
                let _ = sajjil_ramz(ramz_mahfuz);
            },
        }
    }
    natija
}

// ---------------------------------------------------------------------------
// Version and diagnostics
// ---------------------------------------------------------------------------

/// Writes the ABI version this library was built as.
///
/// The first call every consumer makes, before creating anything: a caller
/// whose compiled-in [`TAARIB_ABI_KABIR`] differs from what this writes must
/// refuse to load and say so, because past a major mismatch the two sides no
/// longer agree what any structure means. A minor difference is compatible in
/// both directions.
///
/// The caller owns both integers; this function only writes them, and nothing
/// about them is retained past the call.
///
/// # Safety
///
/// `kabir` and `sagheer` must each be either null or valid for an aligned
/// write of one `u32`. Both null is the one degenerate call that has no
/// observable effect beyond its status.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_abi_isdar(kabir: *mut u32, sagheer: *mut u32) -> i32 {
    ihmi(|| {
        if kabir.is_null() && sagheer.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        if !kabir.is_null() {
            // SAFETY: `kabir` is non-null, and the caller guarantees it is
            // valid for an aligned write of one u32.
            unsafe { kabir.write(TAARIB_ABI_KABIR) };
        }
        if !sagheer.is_null() {
            // SAFETY: `sagheer` is non-null, and the caller guarantees it is
            // valid for an aligned write of one u32.
            unsafe { sagheer.write(TAARIB_ABI_SAGHEER) };
        }
        khitam(TAARIB_NAJAH)
    })
}

/// Writes the library's own release version — the crate version, in semantic
/// `major.minor.patch` form — as UTF-8 text with a trailing NUL, for a
/// diagnostics screen or a log line.
///
/// The ABI contract is [`taarib_abi_isdar`]; this string is for humans and
/// nothing may branch on it.
///
/// The caller owns `hadaf` and its lifetime; nothing is allocated and no
/// pointer is retained. When the buffer is too small, `matlub` receives the
/// required capacity in bytes (including the NUL), nothing is written, and the
/// call returns `TAARIB_SIAT_QASIRA` — grow once and retry.
///
/// # Safety
///
/// `hadaf` must be null or valid for writes of `siaa` bytes, and `matlub` must
/// be null or valid for an aligned write of one `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_isdar_nass(hadaf: *mut u8, siaa: usize, matlub: *mut usize) -> i32 {
    ihmi(|| {
        // SAFETY: the caller's guarantees on `hadaf` and `matlub` are exactly
        // the ones `iktub_nass` requires.
        khitam(unsafe { iktub_nass(env!("CARGO_PKG_VERSION"), hadaf, siaa, matlub) })
    })
}

/// The status code of this thread's most recent failure, or zero when the last
/// call on this thread succeeded.
///
/// An observer: it reads the thread-local stash and disturbs nothing, so it
/// can be called any number of times and interleaved freely with the other
/// `taarib_khata_*` functions. The stash is per thread — a failure on the
/// render thread is never reported to a loader thread — and it is replaced by
/// the next completed call on the same thread, so read it before calling
/// anything else.
///
/// # Safety
///
/// None beyond being called at all: the function takes nothing and writes
/// through nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khata_akhir() -> i32 {
    ihmi(akhir_ramz)
}

/// Writes the permanent machine code of this thread's last failure — the
/// `TAARIB-E-2503` form that names one specific defect forever — as UTF-8 with
/// a trailing NUL.
///
/// Writes an empty string when the last failure carried no structured error (a
/// null pointer, a stale handle, a caught panic) or when there was no failure
/// at all.
///
/// The caller owns `hadaf`; on `TAARIB_SIAT_QASIRA` the required capacity is
/// in `matlub` and nothing was written. An observer: the stashed failure
/// survives this call unchanged, including a failed capacity negotiation, so
/// probing with a null-size call and retrying cannot erase what it is probing
/// for.
///
/// # Safety
///
/// `hadaf` must be null or valid for writes of `siaa` bytes, and `matlub` must
/// be null or valid for an aligned write of one `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khata_ramz(hadaf: *mut u8, siaa: usize, matlub: *mut usize) -> i32 {
    ihmi(|| {
        maa_hifz_alkhata(|| {
            let nass = akhir_ramz_nass();
            // SAFETY: the caller's guarantees on `hadaf` and `matlub` are
            // exactly the ones `iktub_nass` requires.
            unsafe { iktub_nass(&nass, hadaf, siaa, matlub) }
        })
    })
}

/// Writes the user-facing sentence of this thread's last failure, as UTF-8
/// with a trailing NUL, in the requested language: `0` Arabic, `1` English,
/// anything unrecognised Arabic.
///
/// Arabic is the primary text of this product, and a caller that passed a wrong
/// number gets the real sentence rather than nothing. Writes an empty string
/// when the last failure carried no structured error or there was none.
///
/// The caller owns `hadaf`; on `TAARIB_SIAT_QASIRA` the required capacity is
/// in `matlub` and nothing was written. An observer: the stashed failure
/// survives this call unchanged, whatever the negotiation returned.
///
/// # Safety
///
/// `hadaf` must be null or valid for writes of `siaa` bytes, and `matlub` must
/// be null or valid for an aligned write of one `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khata_nass(
    lugha: u32,
    hadaf: *mut u8,
    siaa: usize,
    matlub: *mut usize,
) -> i32 {
    ihmi(|| {
        maa_hifz_alkhata(|| {
            let nass = akhir_nass(crate::khata_c::lugha_min_raqm(lugha));
            // SAFETY: the caller's guarantees on `hadaf` and `matlub` are
            // exactly the ones `iktub_nass` requires.
            unsafe { iktub_nass(&nass, hadaf, siaa, matlub) }
        })
    })
}

/// The suggested next action of this thread's last failure, as the stable
/// discriminant documented in `docs/abi.md`: `0` nothing, `1` retry, `5` pick
/// another font, and so on.
///
/// `0` also when there is no stashed failure. A caller that meets a number it
/// does not recognise shows the sentence without a button rather than showing
/// nothing — the space is additive.
///
/// An observer: it disturbs nothing.
///
/// # Safety
///
/// None beyond being called at all: the function takes nothing and writes
/// through nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khata_khutwa() -> i32 {
    ihmi(akhir_khutwa)
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

/// Creates an engine context: the shaping caches, the layout cache, the pooled
/// layout buffers, the handle tables and the capture channel, behind one
/// opaque handle.
///
/// `khiyarat` is read once and may be freed the moment this returns; null
/// means the defaults, and a non-zero value in the reserved `hashw` field is
/// refused with a stashed sentence naming it, so the field stays genuinely
/// reserved. This library allocates the context and owns it until
/// [`taarib_siyaq_ihdham`] destroys it; the handle written to `khuruj` is
/// valid until then and on any thread, one call at a time — calls on the same
/// context serialise on an internal lock, so two threads sharing one context
/// are safe and slow, and a render thread that wants neither shares nothing.
///
/// # Safety
///
/// `khiyarat` must be null or valid for an aligned read of one
/// [`TaaribKhiyaratSiyaq`]; `khuruj` must be valid for an aligned write of one
/// pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_siyaq_insha(
    khiyarat: *const TaaribKhiyaratSiyaq,
    khuruj: *mut TaaribSiyaq,
) -> i32 {
    ihmi(|| {
        if khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        let khiyarat = if khiyarat.is_null() {
            TaaribKhiyaratSiyaq::default()
        } else {
            // SAFETY: `khiyarat` is non-null, and the caller guarantees it is
            // valid for an aligned read of one TaaribKhiyaratSiyaq.
            unsafe { khiyarat.read() }
        };
        // The reserved field is checked inside `Siyaq::jadeed`, which refuses
        // a non-zero value with a sentence naming it — checked, not ignored,
        // because a value there is either garbage or a newer header's field.
        match Siyaq::jadeed(khiyarat) {
            Ok(siyaq) => {
                let maqbad = sajjil_siyaq(siyaq);
                // SAFETY: `khuruj` is non-null, and the caller guarantees it is
                // valid for an aligned write of one pointer.
                unsafe { khuruj.write(maqbad) };
                khitam(TAARIB_NAJAH)
            },
            Err(khata) => sajjil(&khata),
        }
    })
}

/// Destroys a context and everything it owns: every font, chain and atlas
/// handle issued from it, the layout cache, and the pooled buffers.
///
/// The handle, and every handle issued from it, is dead when this returns; the
/// generation tag means a later use is reported as `TAARIB_MAQBAD_BATIL`
/// rather than executed. Any page pointer previously borrowed through
/// [`taarib_lawha_safha`] dangles from this moment. Destroying a context while
/// another thread is inside a call on it waits for that call to finish first,
/// which is the lock doing its job.
///
/// # Safety
///
/// `siyaq` must be a handle from [`taarib_siyaq_insha`], or null (refused, not
/// dereferenced). No other thread may be *about to* use the handle: the caller
/// is responsible for the ordinary teardown ordering of its own threads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_siyaq_ihdham(siyaq: TaaribSiyaq) -> i32 {
    ihmi(|| {
        if siyaq.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        if ihdhif_siyaq(siyaq) {
            khitam(TAARIB_NAJAH)
        } else {
            sajjil_ramz(TAARIB_MAQBAD_BATIL)
        }
    })
}

/// Empties a context's caches — the layout cache, the pooled layout buffers
/// and the prepared shaper state — and returns the memory, without touching a
/// single handle.
///
/// For a scene transition or a memory warning: every font, chain and atlas
/// handle stays valid, and the only cost of having called this is that the
/// next layouts shape again and refill what was dropped.
///
/// # Safety
///
/// `siyaq` must be a live handle from [`taarib_siyaq_insha`], or null
/// (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_siyaq_amsah(siyaq: TaaribSiyaq) -> i32 {
    ihmi(|| {
        if siyaq.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            siyaq.amsah();
            TAARIB_NAJAH
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Loads a font from bytes in the caller's memory and validates it.
///
/// The bytes are **copied**: the caller's buffer is read during this call only
/// and may be freed the moment it returns, and the copy becomes the one shared
/// buffer that shaping, metrics and rasterization all borrow (Decision 4).
/// `fahras` is the face index inside a `ttc`/`otc` collection, `0` for an
/// ordinary file. `fahs_arabi` non-zero runs the full Arabic validation of
/// Decision 6 — required tables, required features, required coverage — and a
/// failure is `TAARIB_KHATT_MARFUD` with a stashed sentence naming the one
/// missing thing; zero skips only the Arabic check, for the Latin and
/// monospace faces at the end of a chain that will never shape an Arabic
/// letter.
///
/// The handle written to `khuruj` belongs to the context and is destroyed by
/// [`taarib_khatt_ihdham`] or with the context. Chains hold their own
/// references, so destroying the handle after building a chain from it is
/// legal and frees nothing until the chain goes too.
///
/// # Safety
///
/// `bayt` must be valid for reads of `tul` bytes; `khuruj` must be valid for
/// an aligned write of one pointer; `siyaq` must be a live handle or null
/// (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khatt_min_dhakira(
    siyaq: TaaribSiyaq,
    bayt: *const u8,
    tul: usize,
    fahras: u32,
    fahs_arabi: u32,
    khuruj: *mut TaaribKhatt,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || bayt.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        // SAFETY: `bayt` is non-null and the caller guarantees `tul` readable
        // bytes behind it for the duration of this call.
        let khaam = match unsafe { iqra_shariha::<u8>(bayt, tul) } {
            Ok(khaam) => khaam,
            Err(ramz) => return sajjil_ramz(ramz),
        };
        // The copy and the validation both happen before the context lock is
        // taken: neither needs the context, and a render thread sharing this
        // context should not stall behind megabytes of font parsing.
        let nuskha = Arc::new(khaam.to_vec());
        let natija = if fahs_arabi == 0 {
            MawridKhatt::jadeed_latini(nuskha, fahras)
        } else {
            MawridKhatt::jadeed(nuskha, fahras)
        };
        let mawrid = match natija {
            Ok(mawrid) => Arc::new(mawrid),
            Err(khata) => return sajjil(&khata),
        };
        match maa_siyaq(siyaq, move |siyaq| {
            let maqbad = siyaq.khutut.sajjil(mawrid);
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one pointer.
            unsafe { khuruj.write(ila_muashir::<TaaribKhattKhas>(maqbad)) };
            TAARIB_NAJAH
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Destroys a font handle.
///
/// Only the handle dies. A chain built from the font holds its own reference,
/// so every chain, every cached layout and every atlas entry that came from
/// this font keeps working; the font's bytes are freed when the last of those
/// references goes. Destroying an already-destroyed handle is reported as
/// `TAARIB_MAQBAD_BATIL`, not executed.
///
/// # Safety
///
/// `siyaq` must be a live handle or null (refused, not dereferenced); `khatt`
/// must be a handle this context issued, or null (refused).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khatt_ihdham(siyaq: TaaribSiyaq, khatt: TaaribKhatt) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khatt.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            match siyaq.khutut.ihdhif(min_muashir::<TaaribKhattKhas>(khatt)) {
                Some(_) => TAARIB_NAJAH,
                None => TAARIB_MAQBAD_BATIL,
            }
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes a font's own metrics scaled to `hajm` pixels: ascent, descent, line
/// gap, recommended line height, cap height, x-height, and the design units
/// they were derived at.
///
/// Every value comes from the font's tables or, where a table is silent, from
/// measured outline bounds — never from a fraction of the size. `hajm` must be
/// finite; beyond that it is taken as given, because a metrics query at an
/// unusual size is a legitimate thing for a workspace to ask. The caller owns
/// `khuruj`; nothing is retained.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one [`TaaribQiyasatKhatt`];
/// `siyaq` and `khatt` must be live handles or null (refused, not
/// dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khatt_qiyasat(
    siyaq: TaaribSiyaq,
    khatt: TaaribKhatt,
    hajm: f32,
    khuruj: *mut TaaribQiyasatKhatt,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khatt.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        if !hajm.is_finite() {
            return sajjil_ramz(TAARIB_QEEMA_BATILA);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(mawrid) = siyaq.khutut.qeema(min_muashir::<TaaribKhattKhas>(khatt)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            let qiyasat = mawrid.qiyasat(hajm);
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one TaaribQiyasatKhatt.
            unsafe {
                khuruj.write(TaaribQiyasatKhatt {
                    suud: qiyasat.suud,
                    hubut: qiyasat.hubut,
                    fajwa: qiyasat.fajwa,
                    irtifa_satr: qiyasat.irtifa_satr,
                    uluw_kabital: qiyasat.uluw_kabital,
                    uluw_saghir: qiyasat.uluw_saghir,
                    wahdat: u32::from(qiyasat.wahdat),
                    hashw: 0,
                });
            }
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes a font's identity: the value derived from its bytes and face index
/// that the layout cache and every atlas key are built on.
///
/// Process-local by design — the hash mixes at pointer width — so it may be
/// compared, mapped and logged inside this process and must never be persisted
/// or sent anywhere. Two handles loaded from identical bytes report the same
/// identity, which is exactly what makes the cache key honest.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one `u64`; `siyaq` and
/// `khatt` must be live handles or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khatt_huwiya(
    siyaq: TaaribSiyaq,
    khatt: TaaribKhatt,
    khuruj: *mut u64,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khatt.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(mawrid) = siyaq.khutut.qeema(min_muashir::<TaaribKhattKhas>(khatt)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one u64.
            unsafe { khuruj.write(mawrid.huwiya().qeema()) };
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes a font's family name — the typographic family where the font
/// declares one — as UTF-8 with a trailing NUL, for the workspace's font
/// picker and for diagnostics.
///
/// An empty string when the font's `name` table carries neither form, which is
/// a broken table and not a reason to refuse the font.
///
/// The caller owns `hadaf`; on `TAARIB_SIAT_QASIRA` the required capacity is
/// in `matlub` and nothing was written — grow once and retry.
///
/// # Safety
///
/// `hadaf` must be null or valid for writes of `siaa` bytes; `matlub` must be
/// null or valid for an aligned write of one `usize`; `siyaq` and `khatt` must
/// be live handles or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_khatt_aila(
    siyaq: TaaribSiyaq,
    khatt: TaaribKhatt,
    hadaf: *mut u8,
    siaa: usize,
    matlub: *mut usize,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khatt.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(mawrid) = siyaq.khutut.qeema(min_muashir::<TaaribKhattKhas>(khatt)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            // SAFETY: the caller's guarantees on `hadaf` and `matlub` are
            // exactly the ones `iktub_nass` requires.
            unsafe { iktub_nass(mawrid.aila(), hadaf, siaa, matlub) }
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Builds a font chain — the ordered list tried per character — from font
/// handles, and writes its handle.
///
/// The chain takes its own reference to every font, so the font handles may be
/// destroyed afterwards and the chain keeps working. `khutut` is read during
/// this call only. An empty chain is refused with a stashed sentence (there
/// would be nothing to draw with), a chain longer than 256 likewise (a
/// positioned glyph names its font in one byte), and a null or stale handle
/// anywhere in the list refuses the whole chain — a chain with a hole in it
/// would surface much later as missing glyphs.
///
/// Destroy with [`taarib_silsila_ihdham`] or with the context.
///
/// # Safety
///
/// `khutut` must be valid for reads of `adad` handles; `khuruj` must be valid
/// for an aligned write of one pointer; `siyaq` must be a live handle or null
/// (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_silsila_insha(
    siyaq: TaaribSiyaq,
    khutut: *const TaaribKhatt,
    adad: usize,
    khuruj: *mut TaaribSilsila,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khuruj.is_null() || (khutut.is_null() && adad != 0) {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        let maqabid: &[TaaribKhatt] = if adad == 0 {
            // A C caller's idiomatic empty array is a null pointer with a zero
            // count; it reaches the engine's own "empty chain" refusal below
            // rather than being mistaken for a bad pointer here.
            &[]
        } else {
            // SAFETY: `khutut` is non-null whenever `adad` is non-zero, and
            // the caller guarantees `adad` readable handles behind it.
            match unsafe { iqra_shariha::<TaaribKhatt>(khutut, adad) } {
                Ok(maqabid) => maqabid,
                Err(ramz) => return sajjil_ramz(ramz),
            }
        };
        match maa_siyaq(siyaq, |siyaq| {
            let mut mawarid: Vec<Arc<MawridKhatt>> = Vec::with_capacity(maqabid.len());
            for maqbad in maqabid {
                if maqbad.is_null() {
                    return TAARIB_MUASHIR_BATIL;
                }
                let Some(mawrid) = siyaq.khutut.qeema(min_muashir::<TaaribKhattKhas>(*maqbad))
                else {
                    return TAARIB_MAQBAD_BATIL;
                };
                mawarid.push(Arc::clone(mawrid));
            }
            match SilsilatKhutut::jadeeda(mawarid) {
                Ok(silsila) => {
                    let maqbad = siyaq.salasil.sajjil(silsila);
                    // SAFETY: `khuruj` is non-null, and the caller guarantees
                    // it is valid for an aligned write of one pointer.
                    unsafe { khuruj.write(ila_muashir::<TaaribSilsilaKhas>(maqbad)) };
                    TAARIB_NAJAH
                },
                Err(khata) => sajjil(&khata),
            }
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) if ramz == TAARIB_MUASHIR_BATIL || ramz == TAARIB_MAQBAD_BATIL => {
                sajjil_ramz(ramz)
            },
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Destroys a chain handle.
///
/// Only the handle dies: the fonts live for as long as anything else
/// references them, cached layouts remain valid data, and cache entries keyed
/// through this chain simply stop being hit unless an identical chain — same
/// fonts, same order — is built again, in which case they hit again, because
/// the key names font identities rather than this handle.
///
/// # Safety
///
/// `siyaq` must be a live handle or null (refused, not dereferenced);
/// `silsila` must be a handle this context issued, or null (refused).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_silsila_ihdham(siyaq: TaaribSiyaq, silsila: TaaribSilsila) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || silsila.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            match siyaq
                .salasil
                .ihdhif(min_muashir::<TaaribSilsilaKhas>(silsila))
            {
                Some(_) => TAARIB_NAJAH,
                None => TAARIB_MAQBAD_BATIL,
            }
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

// ---------------------------------------------------------------------------
// Decoding a request — one place, reused by layout and by measurement
// ---------------------------------------------------------------------------

/// The reusable scratch a request is decoded into.
///
/// Spans and feature overrides arrive as C arrays and leave as the engine's
/// own types, and that conversion needs somewhere to live. A fresh `Vec` per
/// call would put an allocation on the hot path for every styled string, so
/// each thread keeps one of these: the vectors grow to the largest request the
/// thread has ever decoded and are then reused forever, which is the same
/// grow-once shape as the caller's own glyph buffer.
struct MuaddatTalab {
    /// The converted style spans.
    nitaqat: Vec<NitaqUslub>,
    /// The converted decisions, reused so its feature vector's allocation
    /// survives from call to call.
    khiyarat: KhiyaratTakhtit,
}

thread_local! {
    /// Per thread, not per context, because a request is decoded before the
    /// engine is consulted and two threads must be able to decode at once.
    static MUADDAT_TALAB: RefCell<MuaddatTalab> = RefCell::new(MuaddatTalab {
        nitaqat: Vec::new(),
        khiyarat: KhiyaratTakhtit::default(),
    });
}

/// An optional pixel value: zero (of either sign) and anything non-finite mean
/// "not set", which is how the ABI structures express absence without an
/// `Option`.
fn qeema_ghayr_sifriya(qeema: f32) -> Option<f32> {
    (qeema.is_finite() && (qeema.to_bits() & 0x7FFF_FFFF) != 0).then_some(qeema)
}

/// Converts one wire-format style span into the engine's own.
fn nitaq_min_kharij(kharij: &crate::anwa::TaaribNitaqUslub) -> NitaqUslub {
    let alam = kharij.alam;
    NitaqUslub {
        id: kharij.id,
        bidaya: kharij.bidaya,
        tul: kharij.tul,
        uslub: Uslub {
            khatt: (alam & TAARIB_USLUB_KHATT != 0).then_some(kharij.khatt),
            wazn: (alam & TAARIB_USLUB_WAZN != 0).then_some(kharij.wazn),
            maail: alam & TAARIB_USLUB_MAAIL != 0,
            hajm: (alam & TAARIB_USLUB_HAJM != 0).then_some(kharij.hajm),
            lawn: (alam & TAARIB_USLUB_LAWN != 0).then_some(kharij.lawn.to_be_bytes()),
            tabaud: qeema_ghayr_sifriya(kharij.tabaud),
            izaha: qeema_ghayr_sifriya(kharij.izaha),
            dharra: (alam & TAARIB_USLUB_DHARRA != 0).then_some(Dharra {
                ard: kharij.ard_dharra,
                irtifa: kharij.irtifa_dharra,
                asas: kharij.asas_dharra,
                marja: kharij.marja_dharra,
            }),
        },
    }
}

/// Decodes one [`TaaribTalab`] into the engine's [`TalabTakhtit`].
///
/// The single place a wire request becomes an engine request, for layout and
/// measurement alike, so the two can never decode a flag differently. Every
/// discriminant goes through its `anwa` decoder and an unrecognised number
/// takes the documented default rather than failing. On failure the returned
/// `i32` is a ready status code, not yet stashed.
///
/// # Safety
///
/// The caller guarantees what the ABI contract guarantees it: `talab.nass` is
/// readable for `talab.tul_nass` bytes, `talab.nitaqat` for
/// `talab.adad_nitaqat` spans, and `talab.khiyarat.sifat` for
/// `talab.khiyarat.adad_sifat` features, all for the duration of the borrow
/// the returned request carries.
unsafe fn ifrid_talab<'a>(
    talab: &TaaribTalab,
    khutut: &'a SilsilatKhutut,
    muaddat: &'a mut MuaddatTalab,
) -> Result<TalabTakhtit<'a>, i32> {
    let nass: &'a str = if talab.tul_nass == 0 {
        // Empty text is a legal request whatever the pointer holds, and a C
        // caller's idiomatic empty array is a null pointer with a zero count.
        ""
    } else {
        // SAFETY: `tul_nass` is non-zero, and the caller guarantees `nass`
        // points at that many readable bytes for the duration of the call.
        unsafe { iqra_nass(talab.nass, talab.tul_nass) }?
    };
    let nitaqat_kharij: &[crate::anwa::TaaribNitaqUslub] = if talab.adad_nitaqat == 0 {
        &[]
    } else {
        // SAFETY: `adad_nitaqat` is non-zero, and the caller guarantees
        // `nitaqat` points at that many readable spans.
        unsafe { iqra_shariha(talab.nitaqat, talab.adad_nitaqat) }?
    };
    let sifat_kharij: &[crate::anwa::TaaribSifa] = if talab.khiyarat.adad_sifat == 0 {
        &[]
    } else {
        // SAFETY: `adad_sifat` is non-zero, and the caller guarantees `sifat`
        // points at that many readable features.
        unsafe { iqra_shariha(talab.khiyarat.sifat, talab.khiyarat.adad_sifat) }?
    };

    muaddat.nitaqat.clear();
    muaddat
        .nitaqat
        .extend(nitaqat_kharij.iter().map(nitaq_min_kharij));

    let kharij = &talab.khiyarat;
    let khiyarat = &mut muaddat.khiyarat;
    khiyarat.sifat.clear();
    khiyarat
        .sifat
        .extend(sifat_kharij.iter().map(|sifa| SifaIdafiya {
            wasm: sifa.wasm,
            qeema: sifa.qeema,
        }));
    khiyarat.ittijah = ittijah_min_raqm(kharij.ittijah);
    khiyarat.lugha = lugha_min_raqm(kharij.lugha);
    khiyarat.dabt = dabt_min_raqm(kharij.dabt);
    khiyarat.muhadhaha = muhadhaha_min_raqm(kharij.muhadhaha);
    khiyarat.tashkeel = tashkeel_min_raqm(kharij.tashkeel);
    khiyarat.arqam = arqam_min_raqm(kharij.arqam);
    khiyarat.tajawuz = tajawuz_min_raqm(kharij.tajawuz, kharij.hajm_adna);
    khiyarat.irtifa_satr = (kharij.irtifa_satr > 0.0).then_some(kharij.irtifa_satr);
    khiyarat.tabaud_ahruf = kharij.tabaud_ahruf;
    khiyarat.tabaud_kalimat = kharij.tabaud_kalimat;
    khiyarat.hiwar = kharij.alam & TAARIB_KHIYAR_HIWAR != 0;
    khiyarat.satr_wahid = kharij.alam & TAARIB_KHIYAR_SATR_WAHID != 0;

    Ok(TalabTakhtit {
        nass,
        khutut,
        hajm: talab.hajm,
        ard_mutah: (talab.ard_mutah > 0.0).then_some(talab.ard_mutah),
        irtifa_mutah: (talab.irtifa_mutah > 0.0).then_some(talab.irtifa_mutah),
        nitaqat: &muaddat.nitaqat,
        khiyarat: &muaddat.khiyarat,
    })
}

/// Collects the chain's font identities into a caller-supplied stack array and
/// returns how many there are.
///
/// The cache key names fonts by identity, never by handle, so that two chains
/// built from the same bytes hit the same entries. The array is fixed at 256
/// because the chain itself is capped there by construction — which is what
/// lets the hot path gather identities without touching the allocator.
fn huwiyat_silsila(silsila: &SilsilatKhutut, hadaf: &mut [u64; AQSA_KHUTUT_SILSILA]) -> usize {
    let mut adad = 0usize;
    for (khana, khatt) in hadaf.iter_mut().zip(silsila.khutut()) {
        *khana = khatt.huwiya().qeema();
        adad = adad.saturating_add(1);
    }
    adad
}

/// Derives the measurement answer from a finished layout, which is what a
/// cache hit serves instead of running the pipeline again.
fn qiyas_min_takhtit(takhtit: &TakhtitNass) -> TaaribQiyasNass {
    TaaribQiyasNass {
        ard: takhtit.ard,
        irtifa: takhtit.irtifa,
        suud: takhtit.sutur.first().map_or(0.0, |satr| satr.suud),
        hubut: takhtit.sutur.last().map_or(0.0, |satr| satr.hubut),
        adad_sutur: u32::try_from(takhtit.sutur.len()).unwrap_or(u32::MAX),
        hashw: 0,
    }
}

// ---------------------------------------------------------------------------
// Layout — the hot path
// ---------------------------------------------------------------------------

/// Lays text out into the caller's buffer. The hot path: a game calls this for
/// every visible string, every time one changes.
///
/// The caller owns everything: the request and the text it points at are read
/// during this call only, and the glyph and line arrays inside `makhzan` are
/// the caller's, written into and never retained, never freed, never
/// reallocated. When either array is too small, nothing is written, the
/// `adad_*` fields receive the required counts, and the call returns
/// `TAARIB_SIAT_QASIRA` — grow once, retry, and the buffer never grows again.
///
/// Allocation: a request already in the layout cache is copied straight out of
/// it — the `TAARIB_TAKHTIT_MAKHZAN` flag says so — and the call touches the
/// allocator zero times. A request the cache has not seen is shaped into the
/// context's pooled buffer and the finished layout is handed to the cache, so
/// the next identical request is a hit; the shaping itself is where the work
/// and the memory live, and it happens at most once per distinct request.
/// There is deliberately no allocating variant of this function.
///
/// When runtime capture is on, a request that missed the cache is reported to
/// the registered sink before this returns, whatever the buffer negotiation
/// said — new text is new even when the caller's buffer was short. A miss is
/// precisely "text this context has not seen", so the stream self-deduplicates
/// and steady-state frames report nothing. The sink runs under the context's
/// lock and is bound by its registration contract: return promptly, never
/// call back into this library.
///
/// # Safety
///
/// `talab` must be valid for an aligned read of one [`TaaribTalab`], and every
/// pointer inside it — text, spans, features — must be readable at its stated
/// count for the duration of the call. `makhzan` must be valid for an aligned
/// read and write of one [`TaaribMakhzanTakhtit`], and its `huruf` and `sutur`
/// pointers must be writable at their stated capacities. `siyaq` must be a
/// live handle or null (refused, not dereferenced); the chain named by
/// `talab.silsila` must belong to the same context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_takhtit(
    siyaq: TaaribSiyaq,
    talab: *const TaaribTalab,
    makhzan: *mut TaaribMakhzanTakhtit,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || talab.is_null() || makhzan.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        // SAFETY: `talab` is non-null, and the caller guarantees it is valid
        // for an aligned read of one TaaribTalab.
        let talab = unsafe { talab.read() };
        if talab.silsila.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }

        match maa_siyaq(siyaq, |siyaq| {
            let Siyaq {
                saff,
                khazina,
                salasil,
                makhzan: mujammaa,
                iltiqat,
                ..
            } = siyaq;
            let Some(silsila) = salasil.qeema(min_muashir::<TaaribSilsilaKhas>(talab.silsila))
            else {
                return sajjil_ramz(TAARIB_MAQBAD_BATIL);
            };
            MUADDAT_TALAB.with_borrow_mut(|muaddat| {
                // SAFETY: the entry point's own contract is exactly what
                // `ifrid_talab` requires of the request's interior pointers.
                let mursal = match unsafe { ifrid_talab(&talab, silsila, muaddat) } {
                    Ok(mursal) => mursal,
                    Err(ramz) => return sajjil_ramz(ramz),
                };
                let mut huwiyat = [0u64; AQSA_KHUTUT_SILSILA];
                let adad = huwiyat_silsila(silsila, &mut huwiyat);
                let huwiyat = huwiyat.get(..adad).unwrap_or(&[]);

                if khazina.mufaala()
                    && let Some(mahfuz) = khazina.ijlib(MiftahTakhtit::min_talab(&mursal, huwiyat))
                {
                    // SAFETY: `makhzan` is non-null, and the caller guarantees
                    // it and the arrays inside it per the entry contract.
                    return unsafe { iktub_takhtit(mahfuz, makhzan, true) };
                }

                let musawadda = mujammaa.takhtit();
                if let Err(khata) = saff.khattit_fi(&mursal, musawadda) {
                    return sajjil(&khata);
                }
                // SAFETY: `makhzan` is non-null, and the caller guarantees it
                // and the arrays inside it per the entry contract.
                let ramz = unsafe { iktub_takhtit(musawadda, makhzan, false) };
                // The finished layout goes into the cache even when the
                // caller's buffer was short: the work is done, and the retry
                // that follows a TAARIB_SIAT_QASIRA should be a hit.
                if khazina.mufaala() {
                    khazina.daa(
                        MiftahTakhtit::min_talab(&mursal, huwiyat),
                        musawadda.clone(),
                    );
                }
                mujammaa.dawwir();
                // Capture fires on the miss path whatever the negotiation
                // said: a short buffer does not make the text less new, and
                // the retry that follows will hit the cache and never report.
                if let Some(qanat) = iltiqat.as_ref() {
                    qanat.iltaqit(mursal.nass);
                }
                ramz
            })
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Measures text without positioning a single glyph: widest line, total
/// height, first ascent, last descent, line count.
///
/// Runs the same policies and the same pipeline as [`taarib_takhtit`], because
/// a measurement from any other path would disagree with the layout it
/// predicts. A request the layout cache already holds is answered from the
/// cached layout without shaping; a request it does not hold is measured
/// directly, and — since measurement produces no layout — nothing is inserted,
/// so measuring never evicts anything a renderer is relying on.
///
/// The caller owns `talab` and `khuruj`; both are used during this call only.
///
/// # Safety
///
/// `talab` must be valid for an aligned read of one [`TaaribTalab`] with every
/// interior pointer readable at its stated count for the duration of the call;
/// `khuruj` must be valid for an aligned write of one [`TaaribQiyasNass`];
/// `siyaq` must be a live handle or null (refused, not dereferenced); the
/// chain named by `talab.silsila` must belong to the same context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_qiyas(
    siyaq: TaaribSiyaq,
    talab: *const TaaribTalab,
    khuruj: *mut TaaribQiyasNass,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || talab.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        // SAFETY: `talab` is non-null, and the caller guarantees it is valid
        // for an aligned read of one TaaribTalab.
        let talab = unsafe { talab.read() };
        if talab.silsila.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Siyaq {
                saff,
                khazina,
                salasil,
                ..
            } = siyaq;
            let Some(silsila) = salasil.qeema(min_muashir::<TaaribSilsilaKhas>(talab.silsila))
            else {
                return sajjil_ramz(TAARIB_MAQBAD_BATIL);
            };
            MUADDAT_TALAB.with_borrow_mut(|muaddat| {
                // SAFETY: the entry point's own contract is exactly what
                // `ifrid_talab` requires of the request's interior pointers.
                let mursal = match unsafe { ifrid_talab(&talab, silsila, muaddat) } {
                    Ok(mursal) => mursal,
                    Err(ramz) => return sajjil_ramz(ramz),
                };
                let mut huwiyat = [0u64; AQSA_KHUTUT_SILSILA];
                let adad = huwiyat_silsila(silsila, &mut huwiyat);
                let huwiyat = huwiyat.get(..adad).unwrap_or(&[]);

                let qiyas = if khazina.mufaala()
                    && let Some(mahfuz) = khazina.ijlib(MiftahTakhtit::min_talab(&mursal, huwiyat))
                {
                    qiyas_min_takhtit(mahfuz)
                } else {
                    match saff.qis(&mursal) {
                        Ok(qiyas) => TaaribQiyasNass {
                            ard: qiyas.ard,
                            irtifa: qiyas.irtifa,
                            suud: qiyas.suud,
                            hubut: qiyas.hubut,
                            adad_sutur: qiyas.adad_sutur,
                            hashw: 0,
                        },
                        Err(khata) => return sajjil(&khata),
                    }
                };
                // SAFETY: `khuruj` is non-null, and the caller guarantees it
                // is valid for an aligned write of one TaaribQiyasNass.
                unsafe { khuruj.write(qiyas) };
                TAARIB_NAJAH
            })
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes the layout cache's counters: hits, misses, evictions, bytes held,
/// the byte budget, and how many layouts are cached.
///
/// Diagnostics, not control: a cache that never hits is a cache whose key is
/// wrong, and this is how an adapter's overlay or the Diagnostics screen says
/// so with numbers. The caller owns `khuruj`; nothing is retained.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one [`TaaribIhsaatKhazina`];
/// `siyaq` must be a live handle or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_makhzan_ihsaat(
    siyaq: TaaribSiyaq,
    khuruj: *mut TaaribIhsaatKhazina,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let ihsaat = siyaq.khazina.ihsaat();
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one TaaribIhsaatKhazina.
            unsafe {
                khuruj.write(TaaribIhsaatKhazina {
                    isabat: ihsaat.isabat,
                    ikhfaqat: ihsaat.ikhfaqat,
                    ikhlaat: ihsaat.ikhlaat,
                    bayt: ihsaat.bayt,
                    mizaniya: ihsaat.mizaniya,
                    madkhalat: ihsaat.madkhalat,
                    hashw: 0,
                });
            }
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

// ---------------------------------------------------------------------------
// Atlas
// ---------------------------------------------------------------------------

/// Creates a runtime glyph atlas inside a byte budget and writes its handle.
///
/// `aqsa_ard` and `aqsa_irtifa` are the largest page dimensions the atlas may
/// open — zero means the default of 4096 — and `hashw` is the gutter around
/// every glyph in pixels, taken exactly as given, because zero is a meaningful
/// (if unwise) choice. `namat` is `0` for coverage pages, `1` for signed
/// distance fields, anything else coverage. `mizaniya` is the budget in bytes;
/// the page count is derived from it, and a budget too small for one page is
/// refused now, with a sentence, rather than per glyph forever.
///
/// The atlas belongs to the context. Destroy it with [`taarib_lawha_ihdham`]
/// or with the context; the handle is valid until then.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one pointer; `siyaq` must be
/// a live handle or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_insha(
    siyaq: TaaribSiyaq,
    aqsa_ard: u16,
    aqsa_irtifa: u16,
    hashw: u16,
    namat: u32,
    mizaniya: usize,
    khuruj: *mut TaaribLawha,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        let khiyarat = KhiyaratRasf {
            aqsa_ard: if aqsa_ard == 0 {
                BUD_IFTIRADI
            } else {
                aqsa_ard
            },
            aqsa_irtifa: if aqsa_irtifa == 0 {
                BUD_IFTIRADI
            } else {
                aqsa_irtifa
            },
            hashw,
            quwwat_ithnayn: false,
            aqsa_safahat: AQSA_SAFAHAT_IFTIRADI,
        };
        let namat = match namat {
            1 => NamatSafha::Masafa,
            _ => NamatSafha::Taghtiya,
        };
        // Built before the lock: page allocation is the expensive part and
        // needs nothing from the context.
        let lawha = match LawhaHayya::jadeeda(khiyarat, namat, mizaniya) {
            Ok(lawha) => lawha,
            Err(khata) => return sajjil(&khata),
        };
        match maa_siyaq(siyaq, move |siyaq| {
            let maqbad = siyaq.lawhat.sajjil(lawha);
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one pointer.
            unsafe { khuruj.write(ila_muashir::<TaaribLawhaKhas>(maqbad)) };
            TAARIB_NAJAH
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Destroys an atlas and frees its pages.
///
/// Every page pointer previously borrowed from this atlas through
/// [`taarib_lawha_safha`] dangles from this moment; a texture the caller
/// already uploaded is the caller's and is unaffected. Destroying an
/// already-destroyed handle is reported as `TAARIB_MAQBAD_BATIL`, not
/// executed.
///
/// # Safety
///
/// `siyaq` must be a live handle or null (refused, not dereferenced); `lawha`
/// must be a handle this context issued, or null (refused).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_ihdham(siyaq: TaaribSiyaq, lawha: TaaribLawha) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || lawha.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            match siyaq.lawhat.ihdhif(min_muashir::<TaaribLawhaKhas>(lawha)) {
                Some(_) => TAARIB_NAJAH,
                None => TAARIB_MAQBAD_BATIL,
            }
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Looks a glyph up in the atlas, rasterizing and packing it first if it is
/// not there, and writes where it landed and how to draw it.
///
/// The returned position is a value, copied out — the caller may keep it as
/// long as it likes, but the *rectangle it names* is guaranteed only for the
/// current frame: the glyph is pinned until the next
/// [`taarib_lawha_ibda_itar`], and after that the evictor may hand its
/// rectangle to another glyph. An adapter that reuses last frame's positions
/// re-asks (or re-pins by asking) each frame, which is cheap: a present glyph
/// is a hash lookup. The key's font index names a font in `silsila`, which
/// must be the chain the layout that produced the glyph id was made with — the
/// atlas cannot detect a wrong chain, only a missing font.
///
/// Full-and-unevictable — every rectangle belongs to the frame being drawn —
/// is `TAARIB_LAWHA_MUMTALIA` with a sentence naming the pinned count, which
/// means the visible text at one instant is larger than the atlas's budget.
///
/// # Safety
///
/// `miftah` must be valid for an aligned read of one [`TaaribMiftahShakl`];
/// `khuruj` must be valid for an aligned write of one [`TaaribMawdiShakl`];
/// `siyaq`, `lawha` and `silsila` must be live handles or null (refused, not
/// dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_shakl(
    siyaq: TaaribSiyaq,
    lawha: TaaribLawha,
    silsila: TaaribSilsila,
    miftah: *const TaaribMiftahShakl,
    khuruj: *mut TaaribMawdiShakl,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null()
            || lawha.is_null()
            || silsila.is_null()
            || miftah.is_null()
            || khuruj.is_null()
        {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        // SAFETY: `miftah` is non-null, and the caller guarantees it is valid
        // for an aligned read of one TaaribMiftahShakl.
        let miftah = unsafe { miftah.read() };
        match maa_siyaq(siyaq, |siyaq| {
            let Siyaq {
                salasil, lawhat, ..
            } = siyaq;
            let Some(hayya) = lawhat.qeema_mut(min_muashir::<TaaribLawhaKhas>(lawha)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            let Some(khutut) = salasil.qeema(min_muashir::<TaaribSilsilaKhas>(silsila)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            // The wire key carries no rasterization mode: the atlas was built
            // in one mode and every glyph in it shares it, so the mode comes
            // from the atlas rather than trusting the two to agree.
            let namat = hayya
                .safahat()
                .first()
                .map_or(NamatSafha::Taghtiya, |safha| safha.namat);
            let talab = MiftahShakl {
                khatt: miftah.khatt,
                bakat: miftah.bakat,
                hajm_rubi: miftah.hajm_rubi,
                namat,
                muarrif: miftah.muarrif,
            };
            match hayya.shakl_min_silsila(talab, khutut) {
                Ok(mawdi) => {
                    // SAFETY: `khuruj` is non-null, and the caller guarantees
                    // it is valid for an aligned write of one TaaribMawdiShakl.
                    unsafe {
                        khuruj.write(TaaribMawdiShakl {
                            taqaddum: mawdi.taqaddum,
                            s: mawdi.s,
                            a: mawdi.a,
                            ard: mawdi.ard,
                            irtifa: mawdi.irtifa,
                            izaha_s: mawdi.izaha_s,
                            izaha_a: mawdi.izaha_a,
                            safha: mawdi.safha,
                            hashw: 0,
                        });
                    }
                    TAARIB_NAJAH
                },
                Err(khata) => sajjil(&khata),
            }
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) if ramz == TAARIB_MAQBAD_BATIL => sajjil_ramz(ramz),
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes a **borrowed** view of one texture page: a pointer into the atlas's
/// own memory, its length, its dimensions, and what its bytes mean.
///
/// This is the one function in the surface that hands out a pointer this
/// library owns, so its contract is exact. The caller does not free
/// `bayt` and does not keep it: it stays valid only until the next call, on
/// any thread, that reaches this atlas — [`taarib_lawha_shakl`] may rewrite
/// the texels under it, because eviction reuses rectangles, and
/// [`taarib_lawha_ihdham`] and [`taarib_siyaq_ihdham`] free the memory under
/// it. Read it, upload it, and let it go before touching the context again; a
/// caller that wants the bytes longer copies them.
///
/// [`taarib_siyaq_amsah`] is deliberately **not** in that list. It empties the
/// shaping caches, the layout cache and the scratch pool, and leaves the font,
/// chain and atlas tables exactly as they were — see `Siyaq::amsah`. Saying
/// otherwise here would be the conservative direction for this pointer and the
/// wrong direction for the page count, where a caller that expected a reset
/// would read an unchanged number and report a leak that is not there.
///
/// `fahras` past the last page is `TAARIB_QEEMA_BATILA` — ask
/// [`taarib_lawha_adad_safahat`] first, and re-ask after any lookup, because a
/// lookup can open a page.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one [`TaaribSafha`];
/// `siyaq` and `lawha` must be live handles or null (refused, not
/// dereferenced). After this returns, the caller must honour the borrow
/// contract above — the library cannot detect a stale page pointer, only
/// document when it dies.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_safha(
    siyaq: TaaribSiyaq,
    lawha: TaaribLawha,
    fahras: u16,
    khuruj: *mut TaaribSafha,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || lawha.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(hayya) = siyaq.lawhat.qeema(min_muashir::<TaaribLawhaKhas>(lawha)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            let Some(safha) = hayya.safahat().get(usize::from(fahras)) else {
                return TAARIB_QEEMA_BATILA;
            };
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one TaaribSafha. The borrowed
            // `bayt` pointer outliving this lock is the documented contract:
            // it points into the page's own heap buffer, which only the calls
            // named in the contract above free or rewrite.
            unsafe {
                khuruj.write(TaaribSafha {
                    bayt: safha.bayt.as_ptr(),
                    tul: safha.bayt.len(),
                    ard: safha.ard,
                    irtifa: safha.irtifa,
                    namat: u32::from(safha.namat.bayt()),
                });
            }
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes how many pages the atlas currently has open.
///
/// The count only grows within an atlas's life: a lookup can open a page, and
/// nothing closes one short of destroying the atlas or its context.
/// [`taarib_siyaq_amsah`] does not close pages — it clears the shaping and
/// layout caches and leaves the atlas untouched — so an uploader may loop from
/// zero to this after its lookups and upload what changed, without having to
/// reason about the count going backwards.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one `u32`; `siyaq` and
/// `lawha` must be live handles or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_adad_safahat(
    siyaq: TaaribSiyaq,
    lawha: TaaribLawha,
    khuruj: *mut u32,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || lawha.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(hayya) = siyaq.lawhat.qeema(min_muashir::<TaaribLawhaKhas>(lawha)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            let adad = u32::try_from(hayya.safahat().len()).unwrap_or(u32::MAX);
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one u32.
            unsafe { khuruj.write(adad) };
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Begins a frame on the atlas: releases every glyph the previous frame
/// pinned.
///
/// Call once per rendered frame, first thing, from whichever thread renders.
/// Every glyph handed out by [`taarib_lawha_shakl`] is pinned against eviction
/// until this runs again; an adapter that never calls it fills the atlas with
/// unevictable glyphs and then meets `TAARIB_LAWHA_MUMTALIA` — a loud,
/// attributable failure rather than a silently wrong letter.
///
/// # Safety
///
/// `siyaq` and `lawha` must be live handles or null (refused, not
/// dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_ibda_itar(siyaq: TaaribSiyaq, lawha: TaaribLawha) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || lawha.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(hayya) = siyaq
                .lawhat
                .qeema_mut(min_muashir::<TaaribLawhaKhas>(lawha))
            else {
                return TAARIB_MAQBAD_BATIL;
            };
            hayya.ibda_itar();
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Writes the atlas's counters: hits, misses, evictions, growth events, bytes
/// held, the byte budget, mapped glyphs and open pages.
///
/// A miss count that keeps climbing after the first minutes of play means the
/// patch compiler missed strings, and this is the number the Diagnostics
/// screen says it with. The caller owns `khuruj`; nothing is retained.
///
/// # Safety
///
/// `khuruj` must be valid for an aligned write of one [`TaaribIhsaatLawha`];
/// `siyaq` and `lawha` must be live handles or null (refused, not
/// dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_lawha_ihsaat(
    siyaq: TaaribSiyaq,
    lawha: TaaribLawha,
    khuruj: *mut TaaribIhsaatLawha,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() || lawha.is_null() || khuruj.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            let Some(hayya) = siyaq.lawhat.qeema(min_muashir::<TaaribLawhaKhas>(lawha)) else {
                return TAARIB_MAQBAD_BATIL;
            };
            let ihsaat = hayya.ihsaat();
            // SAFETY: `khuruj` is non-null, and the caller guarantees it is
            // valid for an aligned write of one TaaribIhsaatLawha.
            unsafe {
                khuruj.write(TaaribIhsaatLawha {
                    isabat: ihsaat.isabat,
                    ikhfaqat: ihsaat.ikhfaqat,
                    ikhlaat: ihsaat.ikhlaat,
                    ahdath_namu: ihsaat.ahdath_namu,
                    bayt: ihsaat.bayt,
                    mizaniya: ihsaat.mizaniya,
                    ashkal: ihsaat.ashkal,
                    safahat: u32::from(ihsaat.safahat),
                });
            }
            TAARIB_NAJAH
        }) {
            Some(TAARIB_NAJAH) => khitam(TAARIB_NAJAH),
            Some(ramz) => sajjil_ramz(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

// ---------------------------------------------------------------------------
// Runtime string capture (Phase 12)
// ---------------------------------------------------------------------------

/// Turns runtime string capture on: registers the sink that will receive every
/// layout request this context has not seen before.
///
/// From this call until [`taarib_iltiqat_awqif`] or context destruction, a
/// [`taarib_takhtit`] that misses the layout cache reports its text to `radd`,
/// on the laying-out thread, before the layout call returns. Cache misses are
/// precisely "new text", so the stream self-deduplicates and a steady-state
/// frame reports nothing, which is what makes this safe to leave on during
/// play. The text handed to the sink is borrowed from the layout caller and
/// dies when the sink returns; the sink copies what it wants to keep.
///
/// The sink runs while the context's lock is held. That is the registration
/// contract, and both halves of it are the host's to keep: the sink returns
/// promptly, because a frame is waiting on it, and it never calls back into
/// this library on any handle, because the lock it would need is the lock it
/// is running under.
///
/// `mustakhdim` is carried through untouched and never dereferenced by this
/// library; it must simply stay meaningful to the sink until capture stops. A
/// null `radd` is refused — registering nothing is [`taarib_iltiqat_awqif`]'s
/// job. Registering again replaces the previous sink atomically with respect
/// to layout calls.
///
/// # Safety
///
/// `radd` must be callable under the [`TaaribIltiqatFn`] contract, from any
/// thread, and it and `mustakhdim` must remain valid until capture is stopped
/// or the context destroyed; `siyaq` must be a live handle or null (refused,
/// not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_iltiqat_shaghghil(
    siyaq: TaaribSiyaq,
    radd: TaaribIltiqatFn,
    mustakhdim: *mut c_void,
) -> i32 {
    ihmi(|| {
        if siyaq.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        let Some(radd) = radd else {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        };
        match maa_siyaq(siyaq, |siyaq| {
            siyaq.iltiqat = Some(QanatIltiqat::jadeeda(radd, mustakhdim));
            TAARIB_NAJAH
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}

/// Turns runtime string capture off and drops the registered sink.
///
/// Delivery happens under the same context lock this call takes, so when this
/// returns the last report has already been delivered: no sink invocation is
/// in flight, none will follow, and the sink and its `mustakhdim` may be torn
/// down immediately. Stopping capture that was never started succeeds and
/// does nothing — the end state is the same.
///
/// # Safety
///
/// `siyaq` must be a live handle or null (refused, not dereferenced).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_iltiqat_awqif(siyaq: TaaribSiyaq) -> i32 {
    ihmi(|| {
        if siyaq.is_null() {
            return sajjil_ramz(TAARIB_MUASHIR_BATIL);
        }
        match maa_siyaq(siyaq, |siyaq| {
            siyaq.iltiqat = None;
            TAARIB_NAJAH
        }) {
            Some(ramz) => khitam(ramz),
            None => sajjil_ramz(TAARIB_MAQBAD_BATIL),
        }
    })
}
