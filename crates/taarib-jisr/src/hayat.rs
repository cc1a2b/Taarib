//! الحياة — the handle model: generation-tagged handles, the tables behind
//! them, and the context every handle hangs off.
//!
//! ## What a handle actually is
//!
//! The header exposes handles as pointers to never-instantiated types —
//! [`TaaribSiyaq`], `TaaribKhatt`, `TaaribSilsila`, `TaaribLawha` — so that a C
//! caller gets type safety between handle kinds. But the value inside the
//! pointer is not an address and is never dereferenced. It is a
//! generation-tagged index into a table this library owns:
//!
//! ```text
//! 64-bit targets:   [ index : high 32 bits ][ generation : low 32 bits ]
//! 32-bit targets:   [ index : high 16 bits ][ generation : low 16 bits ]
//! ```
//!
//! Generations start at one and never return to zero, so the handle value zero
//! — which is what a null pointer decodes to — is never valid, and null needs
//! no special case anywhere: it fails the generation match exactly like every
//! other stale value. The split narrows on 32-bit targets because the handle
//! must survive a round trip through a 32-bit pointer — `i686` is a shipping
//! target, because 32-bit game processes are — and 65,535 concurrent handles
//! with 65,535 generations per slot is far beyond anything an adapter creates.
//!
//! ## Why the generation lives in the handle and not in the object
//!
//! A pointer-only handle validates itself by looking at the memory it points
//! to, and that is precisely what cannot work. After a destroy, the allocator
//! is free to hand the same address to the next object — and it does, eagerly,
//! because same-size allocations are what free lists exist for. From that
//! moment the stale handle points at a live, correctly-typed, valid-looking
//! object, and every check that inspects the object passes. The result is not
//! a crash but a corrupted frame: text shaped with somebody else's font,
//! glyphs fetched from the wrong atlas, none of it attributable, none of it
//! reproducible. Carrying the generation in the handle moves the check out of
//! the object entirely: destroying a handle advances its slot's generation, so
//! the stale handle names a `(slot, generation)` pair that no longer exists
//! and is **reported** as
//! [`TAARIB_MAQBAD_BATIL`](crate::khata_c::TAARIB_MAQBAD_BATIL) instead of
//! being executed. Vulkan's non-dispatchable handles work this way for the
//! same reason.
//!
//! ## Slots are reused; generations are not
//!
//! A destroyed slot goes back on the free list with its advanced generation,
//! so the table stays small however many create/destroy cycles a session
//! runs. A slot whose generation would wrap, though, is retired instead of
//! reused: wrapping would reissue generation one, at which point the very
//! oldest stale handle to that slot becomes *valid again* — a use-after-free
//! detector that eventually re-arms the bug it exists to catch is worse than
//! useless, because it converts a rare failure into a rarer one. Losing one
//! slot out of four billion per wrapped lifetime is a price; losing the
//! guarantee is not.
//!
//! ## The registry, and the locking contract
//!
//! Contexts live in one process-wide table behind a [`parking_lot::Mutex`],
//! because a C caller has no other place to put them: the only thing that
//! crosses the boundary is an integer, and an integer needs exactly one table
//! that turns it back into an object. This is the one piece of global state in
//! the crate beside the per-context caches, and it is off the per-glyph path:
//! an entry point takes the lock once, and everything inside runs on plain
//! references.
//!
//! The contract, and it is load-bearing:
//!
//! - [`maa_siyaq`] holds the registry lock for the whole duration of its
//!   closure. The mutex is not re-entrant, so a closure that calls back into
//!   the ABI — directly, or through a capture callback that does — deadlocks
//!   the calling thread, which in a game is the render thread.
//! - No entry point may call [`maa_siyaq`] twice in one activation, nested or
//!   sequentially-while-holding; everything an entry point needs from a
//!   context is taken in one closure.
//! - An unwind out of the closure releases the lock — `parking_lot` does not
//!   poison — so a panic caught at the ABI boundary leaves the registry
//!   usable; only the context that was mid-mutation is suspect, and it is the
//!   caller's to destroy.

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::fmt;
use std::sync::Arc;

use parking_lot::Mutex;
use taarib_lawha::namu::LawhaHayya;
use taarib_saff::Saff;
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_usus::khata::{Khata, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;

use crate::anwa::{TaaribKhiyaratSiyaq, TaaribSiyaq};
use crate::dhakira::MakhzanMuaqqat;
use crate::khata_c;
use crate::khazina::Khazina;

// ---------------------------------------------------------------------------
// The handle encoding
// ---------------------------------------------------------------------------

/// How many low bits of a handle carry the generation; the index occupies the
/// same number of high bits, so the two halves split the handle evenly.
///
/// Thirty-two where a pointer is 64 bits wide. Sixteen where it is 32 —
/// `i686` and `wasm32` — because the handle rides inside a pointer-shaped
/// value and must survive the round trip through it undamaged.
#[cfg(target_pointer_width = "64")]
const ARD_JEEL: u32 = 32;

/// How many low bits of a handle carry the generation; see the 64-bit arm.
#[cfg(not(target_pointer_width = "64"))]
const ARD_JEEL: u32 = 16;

/// The mask that extracts a generation from a handle.
const QINA_JEEL: u64 = (1 << ARD_JEEL) - 1;

/// The largest generation a handle can carry. A slot that has used it is
/// retired rather than wrapped back to one.
const AQSA_JEEL: u64 = QINA_JEEL;

/// The most slots one table can address — the index field's range.
const AQSA_KHANAT: u64 = 1 << ARD_JEEL;

/// Packs a slot index and a generation into a handle.
///
/// Total by construction: an index the encoding cannot represent — which the
/// table's own growth bound prevents from ever being live — packs to zero,
/// the never-valid handle, rather than to a value that could collide with a
/// real one.
fn rakkib(fahras: usize, jeel: u64) -> u64 {
    match u64::try_from(fahras) {
        Ok(raqm) if raqm < AQSA_KHANAT => (raqm << ARD_JEEL) | jeel,
        _ => 0,
    }
}

/// Splits a handle into its slot index and generation.
///
/// `None` for anything that cannot name a live entry: a zero generation —
/// which is what a null pointer decodes to — or an index wider than this
/// target ever issues.
fn fakk(maqbad: u64) -> Option<(usize, u64)> {
    let jeel = maqbad & QINA_JEEL;
    if jeel == 0 {
        return None;
    }
    let fahras = usize::try_from(maqbad >> ARD_JEEL).ok()?;
    Some((fahras, jeel))
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// One slot: the generation it is on, and the value if it is occupied.
struct Khana<T> {
    /// The generation a live handle to this slot must carry. Advanced on every
    /// destroy, so every stale handle to this slot mismatches forever.
    jeel: u64,
    /// The occupant, or `None` between registrations.
    qeema: Option<T>,
}

/// A table of generation-tagged handles.
///
/// This is the mechanism behind every opaque handle the ABI issues. A
/// registration takes a slot — a recycled one from the free list, or a fresh
/// one — and returns `(index << ARD_JEEL) | generation` as one `u64`, the
/// index in the high half and the generation in the low, offset so that zero
/// can never occur: generations begin at one. Destroying
/// an entry advances the slot's generation, so any handle issued earlier for
/// that slot now names a pair that does not exist, and lookup reports it
/// invalid instead of handing back whoever moved in afterwards. That is the
/// difference between a bug report naming a stale handle and a corrupted
/// frame in somebody's game — the module documentation walks through why an
/// address-based handle cannot make the same promise.
///
/// The table itself never panics, never indexes unchecked, and never shrinks:
/// slots are recycled through the free list, and a slot whose generation
/// space is exhausted is retired in place rather than reused, keeping the
/// stale-handle guarantee unconditional.
pub struct JadwalMaqabid<T> {
    /// Every slot ever opened, live, free, or retired.
    khanat: Vec<Khana<T>>,
    /// Indices of slots that are free and still have generations to give.
    faragh: Vec<usize>,
}

impl<T> Default for JadwalMaqabid<T> {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl<T> fmt::Debug for JadwalMaqabid<T> {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Counts, not contents: the values are whole engine contexts and
        // atlases, and `T` owes this table no `Debug` of its own.
        matbaa
            .debug_struct("JadwalMaqabid")
            .field("adad", &self.adad())
            .field("khanat", &self.khanat.len())
            .field("faragh", &self.faragh.len())
            .finish()
    }
}

impl<T> JadwalMaqabid<T> {
    /// An empty table.
    ///
    /// `const`, so a table can sit directly in a `static` behind a mutex —
    /// which is exactly where the context registry sits.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            khanat: Vec::new(),
            faragh: Vec::new(),
        }
    }

    /// Registers a value and returns its handle.
    ///
    /// Prefers a recycled slot, whose generation was already advanced when its
    /// previous occupant was destroyed; otherwise opens a fresh slot on
    /// generation one.
    ///
    /// Returns `0` — the never-valid handle — when the table cannot take
    /// another entry, which happens only when every representable slot is live
    /// or retired. The value being registered is dropped in that case, and the
    /// caller reports the refusal rather than inventing a handle.
    #[must_use = "dropping the returned handle leaks the entry"]
    pub fn sajjil(&mut self, qeema: T) -> u64 {
        if let Some(fahras) = self.faragh.pop() {
            let Some(khana) = self.khanat.get_mut(fahras) else {
                // A free-list entry past the table would be a defect in this
                // module; refusing the registration is the non-panicking
                // answer, and zero is the handle that cannot be misread.
                return 0;
            };
            khana.qeema = Some(qeema);
            return rakkib(fahras, khana.jeel);
        }
        let fahras = self.khanat.len();
        if u64::try_from(fahras).unwrap_or(u64::MAX) >= AQSA_KHANAT {
            return 0;
        }
        self.khanat.push(Khana {
            jeel: 1,
            qeema: Some(qeema),
        });
        rakkib(fahras, 1)
    }

    /// The value behind a handle, or `None` for a handle this table never
    /// issued, a handle already destroyed, or the zero a null pointer decodes
    /// to.
    #[must_use]
    pub fn qeema(&self, maqbad: u64) -> Option<&T> {
        let (fahras, jeel) = fakk(maqbad)?;
        let khana = self.khanat.get(fahras)?;
        if khana.jeel == jeel {
            khana.qeema.as_ref()
        } else {
            None
        }
    }

    /// As [`JadwalMaqabid::qeema`], mutably.
    #[must_use]
    pub fn qeema_mut(&mut self, maqbad: u64) -> Option<&mut T> {
        let (fahras, jeel) = fakk(maqbad)?;
        let khana = self.khanat.get_mut(fahras)?;
        if khana.jeel == jeel {
            khana.qeema.as_mut()
        } else {
            None
        }
    }

    /// Destroys a handle, returning its value so the caller decides where the
    /// drop runs — outside a lock, for anything whose teardown is heavy.
    ///
    /// The slot's generation advances first, which is the whole guarantee:
    /// from this call on, the destroyed handle and every copy of it mismatch
    /// and are reported invalid. The slot then returns to the free list —
    /// unless the advance exhausted its generation space, in which case it is
    /// retired in place. Reusing it would wrap the generation back to one and
    /// make the oldest stale handle to that slot valid again, and losing one
    /// slot out of four billion is nothing next to losing the guarantee.
    ///
    /// `None` for a handle that is not live, in which case nothing changes:
    /// destroying twice is reported, not amplified.
    pub fn ihdhif(&mut self, maqbad: u64) -> Option<T> {
        let (fahras, jeel) = fakk(maqbad)?;
        let khana = self.khanat.get_mut(fahras)?;
        if khana.jeel != jeel {
            return None;
        }
        let qeema = khana.qeema.take()?;
        khana.jeel = khana.jeel.saturating_add(1);
        if khana.jeel <= AQSA_JEEL {
            self.faragh.push(fahras);
        }
        Some(qeema)
    }

    /// How many entries are live.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.khanat
            .iter()
            .filter(|khana| khana.qeema.is_some())
            .count()
    }

    /// Destroys every live entry at once, without forgetting the generations.
    ///
    /// Each occupied slot is emptied and its generation advanced, exactly as
    /// if every handle had been destroyed one by one — so handles from before
    /// the sweep stay detectably stale. The one thing this must never do is
    /// reset the slots outright: a table that starts its generations over
    /// hands old handles back their validity, which is the bug this whole
    /// module exists to make impossible.
    pub fn amsah(&mut self) {
        self.faragh.clear();
        for (fahras, khana) in self.khanat.iter_mut().enumerate() {
            if khana.qeema.take().is_some() {
                khana.jeel = khana.jeel.saturating_add(1);
            }
            if khana.jeel <= AQSA_JEEL {
                self.faragh.push(fahras);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The two casts
// ---------------------------------------------------------------------------

/// Wraps a handle integer in the pointer-shaped type the header exposes.
///
/// This function and [`min_muashir`] are the **only** casts between the handle
/// integer and the pointer type, in either direction, anywhere in this
/// library — and the pointer is never dereferenced. That is structural, not
/// disciplinary: the pointer is built with
/// [`without_provenance_mut`](std::ptr::without_provenance_mut), so it carries
/// no provenance and there is no memory it could legally read. Neither
/// function can produce a reference, and nothing downstream can launder one
/// out of what they return.
///
/// A refused registration — handle zero — becomes the null pointer here,
/// which is exactly what a C caller tests for.
#[must_use]
pub fn ila_muashir<T>(maqbad: u64) -> *mut T {
    std::ptr::without_provenance_mut(usize::try_from(maqbad).unwrap_or(0))
}

/// Recovers the handle integer from the pointer-shaped type.
///
/// The other half of the only pointer/integer cast pair in this library; see
/// [`ila_muashir`]. Only the address bits are read — the pointer is not
/// dereferenced and could not be. A null pointer yields zero, the never-valid
/// handle, so null flows through lookup and is reported invalid without a
/// special case.
#[must_use]
pub fn min_muashir<T>(muashir: *mut T) -> u64 {
    muashir.addr() as u64
}

// ---------------------------------------------------------------------------
// The capture channel
// ---------------------------------------------------------------------------

/// The callback a host registers to receive runtime-captured strings.
///
/// `mustakhdim` is the opaque pointer the host registered alongside the
/// callback, handed back untouched. `nass` and `tul` are the captured string
/// as UTF-8 bytes with an explicit length, **not** NUL-terminated, valid only
/// for the duration of the call: a callback that wants the text copies it
/// before returning.
///
/// The registration entry point states, and the host promises, that the pair
/// is callable from any thread, is non-blocking — it runs inside the ABI on
/// what is usually a render thread — and never calls back into this library,
/// because the registry lock is held while it runs.
pub type TaaribRaddIltiqat =
    unsafe extern "C" fn(mustakhdim: *mut c_void, nass: *const u8, tul: usize);

/// Phase 12's runtime string capture channel: the registered callback and the
/// opaque pointer that travels with it.
///
/// Capture exists for the games whose strings cannot be extracted statically:
/// with a session running, every string that reaches a takeover point inside
/// the game is reported through this channel, and the host's transport carries
/// it out to the extraction pipeline.
pub struct QanatIltiqat {
    /// The host's callback.
    radd: TaaribRaddIltiqat,
    /// The host's pointer, carried but never dereferenced.
    mustakhdim: *mut c_void,
}

// SAFETY: `mustakhdim` is an opaque token, not memory this library ever reads
// or writes — it is only ever handed back, unchanged, to the exact callback it
// was registered with, and the registration contract requires that pair to be
// callable from any thread. A raw pointer that is never dereferenced carries
// no thread affinity of its own, so moving the channel between threads — which
// happens because a context is `Send` — cannot violate anything this library
// does; whatever the callback does with the pointer is governed by the
// registrar's own promise.
unsafe impl Send for QanatIltiqat {}

impl fmt::Debug for QanatIltiqat {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Neither pointer is printed: the user pointer is an address inside
        // the host process, and addresses in a diagnostics bundle are layout
        // information the bundle has no business carrying.
        matbaa.debug_struct("QanatIltiqat").finish_non_exhaustive()
    }
}

impl QanatIltiqat {
    /// Binds a callback to its user pointer.
    ///
    /// The registration entry point that calls this is where the host makes
    /// the promises the channel runs on: the callback outlives the
    /// registration, tolerates any thread, returns promptly, and never calls
    /// back into this library.
    #[must_use]
    pub const fn jadeeda(radd: TaaribRaddIltiqat, mustakhdim: *mut c_void) -> Self {
        Self { radd, mustakhdim }
    }

    /// Reports one captured string through the channel.
    ///
    /// The bytes are lent for the duration of the call only; the callback
    /// copies what it wants to keep. This is called with the registry lock
    /// held, which is why the callback's contract forbids it from re-entering
    /// the ABI.
    pub fn iltaqit(&self, nass: &str) {
        // SAFETY: the callback and pointer were registered as a pair through
        // the entry point whose documented contract the host accepted: the
        // callback remains callable for the life of the registration and from
        // any thread, and `mustakhdim` is passed through untouched — this
        // library has never dereferenced it and does not here. `nass` is a
        // live `&str`, so the pointer and length describe initialised UTF-8
        // that outlives the call.
        unsafe { (self.radd)(self.mustakhdim, nass.as_ptr(), nass.len()) };
    }
}

// ---------------------------------------------------------------------------
// The context
// ---------------------------------------------------------------------------

/// An engine context: everything one `TaaribSiyaq` handle owns.
///
/// One context is one caller's whole world — its engine caches, its layout
/// cache, its fonts, chains and atlases, its scratch pool, and its capture
/// channel. The font, chain and atlas tables hang off the context rather than
/// off a second global, so their handles are meaningful only together with
/// the context that issued them, and destroying a context takes everything it
/// issued down with it in one move.
///
/// A context is `Send` — the registry it lives in requires it, and hosts
/// genuinely do create on a loader thread and use on a render thread — but
/// nothing here is `Sync`, and nothing needs to be: all access is serialized
/// through [`maa_siyaq`] and arrives as `&mut`.
pub struct Siyaq {
    /// The engine: shaping caches and the pipeline.
    pub saff: Saff,
    /// The layout cache, budgeted in bytes; disabled when built with a budget
    /// of zero.
    pub khazina: Khazina,
    /// Loaded fonts, by handle. `Arc`, because chains share them and Decision
    /// 4 shares their bytes.
    pub khutut: JadwalMaqabid<Arc<MawridKhatt>>,
    /// Font chains, by handle.
    pub salasil: JadwalMaqabid<SilsilatKhutut>,
    /// Runtime atlases, by handle.
    pub lawhat: JadwalMaqabid<LawhaHayya>,
    /// The rotating pool of layout buffers behind the allocation-free path.
    pub makhzan: MakhzanMuaqqat,
    /// The runtime string capture channel, when a host has registered one.
    pub iltiqat: Option<QanatIltiqat>,
}

impl fmt::Debug for Siyaq {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Counts and states, not contents: a context holds fonts and atlases,
        // and a debug line that dumps them is a log nobody reads.
        matbaa
            .debug_struct("Siyaq")
            .field("saff", &self.saff.adad_khutut())
            .field("khazina", &self.khazina.mufaala())
            .field("khutut", &self.khutut.adad())
            .field("salasil", &self.salasil.adad())
            .field("lawhat", &self.lawhat.adad())
            .field("makhzan", &self.makhzan.adad())
            .field("iltiqat", &self.iltiqat.is_some())
            .finish()
    }
}

impl Siyaq {
    /// Builds a context from the caller's options.
    ///
    /// [`TaaribKhiyaratSiyaq::mizaniyat_makhzan`] is the layout cache's byte
    /// budget, and zero disables the cache entirely — the offline compiler's
    /// configuration, where every string is laid out once and a cache would
    /// only hold memory. [`TaaribKhiyaratSiyaq::adad_makhazin`] sizes the
    /// scratch pool, with zero meaning the default; the pool clamps it to its
    /// own bounds and documents why.
    ///
    /// # Errors
    ///
    /// [`KhataJisr::HaqlMahjuzGhayrSifri`] when the reserved field is not
    /// zero. Reserved means *checked*: a nonzero value there is either garbage
    /// memory or an adapter built against a newer header whose real field this
    /// build would silently ignore, and both deserve a refusal that names
    /// itself over a context that quietly does the wrong thing.
    pub fn jadeed(khiyarat: TaaribKhiyaratSiyaq) -> Natija<Self> {
        if khiyarat.hashw != 0 {
            return Err(KhataJisr::HaqlMahjuzGhayrSifri {
                qeema: khiyarat.hashw,
            }
            .into());
        }
        Ok(Self {
            saff: Saff::jadeed(),
            khazina: Khazina::jadeeda(khiyarat.mizaniyat_makhzan),
            khutut: JadwalMaqabid::jadeed(),
            salasil: JadwalMaqabid::jadeed(),
            lawhat: JadwalMaqabid::jadeed(),
            makhzan: MakhzanMuaqqat::jadeed(khiyarat.adad_makhazin),
            iltiqat: None,
        })
    }

    /// Releases every cache and pooled buffer, keeping every handle valid.
    ///
    /// Empties the shaping caches, the layout cache, and the scratch pool —
    /// the memory that regrows on demand. It deliberately does **not** touch
    /// the font, chain or atlas tables and does not unregister the capture
    /// channel: those are resources the caller created and still holds
    /// handles to, and a "give me the memory back" call that silently turned
    /// every font handle stale would convert a memory request into a broken
    /// frame. Destroying resources is what their destroy entry points are
    /// for.
    pub fn amsah(&mut self) {
        self.saff.amsah();
        self.khazina.amsah();
        self.makhzan.amsah();
    }
}

// ---------------------------------------------------------------------------
// The registry
// ---------------------------------------------------------------------------

/// The process-wide context table.
///
/// Global because a C caller has nowhere else to keep a Rust object: the only
/// thing that crosses the boundary is an integer, and one table turns it back
/// into the context it names. Everything else the ABI touches hangs off a
/// context, so this is the single global beside the caches — and it is locked
/// per entry point, not per glyph.
static SIYAQAT: Mutex<JadwalMaqabid<Siyaq>> = Mutex::new(JadwalMaqabid::jadeed());

/// Registers a context and returns the handle the caller will hold.
///
/// Null — the wrapped zero handle — means the registry refused because its
/// table can take no more entries, which in practice means a host leaking
/// contexts by the thousand. The refusal is stashed as this thread's last
/// error before returning, so the entry point that sees null reports
/// [`akhir_ramz`](crate::khata_c::akhir_ramz)'s code and the caller can
/// retrieve the sentence that names the leak.
#[must_use = "dropping the handle leaks the context"]
pub fn sajjil_siyaq(siyaq: Siyaq) -> TaaribSiyaq {
    let maqbad = SIYAQAT.lock().sajjil(siyaq);
    if maqbad == 0 {
        let khata: Khata = KhataJisr::JadwalMumtali { adad: AQSA_KHANAT }.into();
        let _ = khata_c::sajjil(&khata);
    }
    ila_muashir(maqbad)
}

/// Runs a closure over the context a handle names.
///
/// `None` when the handle is stale, foreign, or null — the caller reports
/// [`TAARIB_MAQBAD_BATIL`](crate::khata_c::TAARIB_MAQBAD_BATIL) — and
/// `Some` of the closure's result otherwise.
///
/// **The locking contract.** The registry lock is held for the entire
/// duration of `amal`, and the mutex is not re-entrant. Therefore: the
/// closure must not call back into the ABI, not even through the capture
/// callback's misbehaviour; no entry point calls `maa_siyaq` twice in one
/// activation; and everything an entry point needs from the context is done
/// inside one closure. Breaking any of these deadlocks the calling thread —
/// inside a game, the render thread — with no diagnostic beyond a frozen
/// process. An unwind out of the closure releases the lock, so a panic caught
/// at the boundary does not wedge the registry.
#[must_use]
pub fn maa_siyaq<T>(maqbad: TaaribSiyaq, amal: impl FnOnce(&mut Siyaq) -> T) -> Option<T> {
    let mut jadwal = SIYAQAT.lock();
    let siyaq = jadwal.qeema_mut(min_muashir(maqbad))?;
    Some(amal(siyaq))
}

/// Destroys the context a handle names, with everything it owns.
///
/// `false` for a handle that was not live — a double destroy is reported, not
/// amplified. The context is lifted out of the table under the lock, but its
/// teardown runs **after** the lock is released: dropping a context frees the
/// layout cache, the pools and every atlas it issued, and holding the
/// registry lock while the allocator walks all of that would stall every
/// other thread's entry points behind a destructor.
#[must_use]
pub fn ihdhif_siyaq(maqbad: TaaribSiyaq) -> bool {
    // The guard is a temporary of this statement, so the lock is released
    // before `siyaq` — and the context's whole teardown — drops at the end of
    // the function.
    let siyaq = SIYAQAT.lock().ihdhif(min_muashir(maqbad));
    siyaq.is_some()
}

/// How many contexts are currently live, for diagnostics.
///
/// A count that climbs for the length of a session is a host leaking
/// contexts, and the diagnostics screen would rather say so than wait for the
/// registry to refuse.
#[must_use]
pub fn adad_siyaqat() -> usize {
    SIYAQAT.lock().adad()
}

// ---------------------------------------------------------------------------
// What can go wrong at the bridge itself
// ---------------------------------------------------------------------------

/// Failures of the bridge's own machinery — not the engine's, not a font's.
///
/// Small on purpose: almost everything that goes wrong at an entry point is
/// either a raw-argument failure that carries no [`Khata`] — a null pointer, a
/// stale handle, a short buffer — or a real engine failure that already
/// arrives as one. What is left is the bridge's own contract being broken,
/// and these variants name those cases.
#[derive(Debug, Clone)]
pub enum KhataJisr {
    /// A reserved field that must be zero is not.
    HaqlMahjuzGhayrSifri {
        /// The value found where zero was required.
        qeema: u32,
    },
    /// The handle table can take no more entries.
    JadwalMumtali {
        /// The table's capacity — how many entries it was refusing at.
        adad: u64,
    },
}

impl fmt::Display for KhataJisr {
    fn fmt(&self, matbaa: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HaqlMahjuzGhayrSifri { qeema } => {
                write!(matbaa, "reserved field not zero: {qeema:#010x}")
            },
            Self::JadwalMumtali { adad } => {
                write!(matbaa, "handle table full at {adad} entries")
            },
        }
    }
}

impl std::error::Error for KhataJisr {}

impl Tafsir for KhataJisr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::JISR
                + match self {
                    Self::HaqlMahjuzGhayrSifri { .. } => 0,
                    Self::JadwalMumtali { .. } => 1,
                },
        )
    }

    fn arabi(&self) -> String {
        match self {
            Self::HaqlMahjuzGhayrSifri { .. } => {
                "حقل محجوز في الخيارات ليس صفرًا؛ يبدو أن المحوِّل مبني على ترويسة أحدث من هذه \
                 المكتبة."
                    .to_owned()
            },
            Self::JadwalMumtali { .. } => {
                "جدول المقابض ممتلئ ولا يمكن إصدار مقبض جديد؛ هذا عادةً تسريب مقابض أُنشئت ولم \
                 تُدمَّر."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::HaqlMahjuzGhayrSifri { .. } => {
                "A reserved field in the options is not zero; the adapter appears to be built \
                 against a newer header than this library."
                    .to_owned()
            },
            Self::JadwalMumtali { .. } => {
                "The handle table is full and cannot issue a new handle; this usually means \
                 handles were created and never destroyed."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // A newer header means a newer adapter, which means this build of
            // Taarib is the old half of the pair.
            Self::HaqlMahjuzGhayrSifri { .. } => Khutwa::TahdithTaarib,
            Self::JadwalMumtali { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::HaqlMahjuzGhayrSifri { qeema } => {
                let _ = siyaq.insert("qeema".to_owned(), QeemaSiyaq::Raqm(i64::from(*qeema)));
            },
            Self::JadwalMumtali { adad } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Hajm(*adad));
            },
        }
        siyaq
    }
}

khata_min!(KhataJisr);
