//! الذاكرة — caller-owned buffers, the scratch pool behind the hot path, and
//! the only doors through which a caller's pointer ever becomes a reference.
//!
//! Three things live here, and together they are the whole memory story of the
//! bridge.
//!
//! **The scratch pool.** [`MakhzanMuaqqat`] is a rotating pool of finished
//! [`TakhtitNass`] buffers that the entry points lay out into before copying
//! the result to the caller. It is deliberately a pool of *layout results*,
//! not a bump allocator over raw bytes. [`TakhtitNass::amsah`] empties a
//! result while keeping every allocation its vectors have grown, which is
//! exactly the reuse the hot path needs — after the first few frames the
//! buffers have grown to the size the game's text asks for, and a layout call
//! touches the allocator zero times. A bump arena of raw memory would have to
//! rebuild the layout result on top of itself to be usable at all, and two
//! definitions of the same structure are two definitions that drift.
//!
//! **The write path.** [`iktub_takhtit`] copies a finished layout into the
//! caller-owned arrays of a [`TaaribMakhzanTakhtit`], negotiating capacity:
//! too small means the required counts and nothing else, big enough means
//! every glyph, every line, the summary fields and the flags. The conversion
//! from [`Harf`] and [`SatrMansuq`] to [`TaaribHarf`] and [`TaaribSatr`] lives
//! in this file and nowhere else, so the engine's representation and the ABI's
//! frozen one cannot drift apart — a second copy of that mapping would be a
//! second place for a field to be forgotten.
//!
//! **The read fences.** [`iqra_nass`] and [`iqra_shariha`] are the only places
//! in this crate where a pointer that crossed the boundary becomes a Rust
//! reference. Every entry point that accepts caller memory routes it through
//! one of the two, so the null check, the overflow check and the UTF-8 check
//! happen exactly once, in one place, under one written contract. A
//! zero-length input is not an error at either fence: a null pointer with a
//! zero length is an empty slice, which is precisely what a caller with no
//! style spans passes.

use taarib_saff::natija::{Harf, SatrMansuq, TakhtitNass, TaqreerTajawuz};
use taarib_saff::talab::Ittijah;

use crate::anwa::{
    TAARIB_HARF_ALAMA, TAARIB_SATR_AKHIR, TAARIB_SATR_YAMEEN, TAARIB_TAKHTIT_MAKHZAN,
    TAARIB_TAKHTIT_MAQSUS, TAARIB_TAKHTIT_TAJAWUZ, TAARIB_TAKHTIT_YAMEEN, TaaribHarf,
    TaaribMakhzanTakhtit, TaaribSatr, TaaribTaqreerTajawuz,
};
use crate::khata_c::{
    TAARIB_MUASHIR_BATIL, TAARIB_NAJAH, TAARIB_QEEMA_BATILA, TAARIB_SIAT_QASIRA,
    TAARIB_TARMIZ_BATIL, sajjil_ramz,
};

/// The fewest buffers a pool will hold.
///
/// Two, because rotation with one buffer preserves nothing: the whole point of
/// the pool is that the layout an entry point is still reading survives while
/// the next one is produced, and a single slot would be emptied under the
/// reader by the very next acquisition.
const ADNA_MAKHAZIN: usize = 2;

/// The most buffers a pool will hold.
///
/// The count arrives from a foreign process through
/// [`TaaribKhiyaratSiyaq::adad_makhazin`](crate::anwa::TaaribKhiyaratSiyaq),
/// and a mistaken or hostile value must not become a giant allocation inside
/// somebody else's game. No call pattern in this ABI keeps more than a handful
/// of layouts alive at once, so everything past this bound is clamped rather
/// than honoured.
const AQSA_MAKHAZIN: usize = 64;

/// A rotating pool of reusable layout buffers.
///
/// Why a pool rather than one buffer: an entry point that is still reading a
/// layout — copying it into the caller's arrays, comparing it against a bound,
/// deciding an overflow policy — must be able to acquire the next buffer
/// without destroying the one it is reading. With a single buffer the second
/// acquisition would empty the first mid-read, and the defect would surface as
/// data-dependent garbage in a game's vertex buffer rather than as anything
/// attributable. With rotation, a result survives one full trip around the
/// pool: [`MakhzanMuaqqat::takhtit`] hands out the slot under the cursor and
/// [`MakhzanMuaqqat::dawwir`] moves the cursor on, so the previous slot is not
/// touched again until every other slot has been used.
///
/// Why a pool of [`TakhtitNass`] rather than raw memory: emptying a
/// `TakhtitNass` keeps the capacity of both of its vectors, so a buffer that
/// has once held a menu's worth of glyphs lays that menu out forever without
/// allocating. That is the entire mechanism a bump allocator would exist to
/// provide, already shaped as the structure the engine writes and the write
/// path reads — rebuilding it over bytes would mean maintaining the layout
/// result twice.
///
/// The first slot is held apart from the rest so that acquisition is total: a
/// pool can never be empty, and [`MakhzanMuaqqat::takhtit`] can never fail,
/// panic, or index out of bounds.
pub struct MakhzanMuaqqat {
    /// Slot zero, held separately so acquisition needs no index and no unwrap.
    awwal: TakhtitNass,
    /// Slots one onward.
    baqiya: Vec<TakhtitNass>,
    /// The cursor: `0` is [`MakhzanMuaqqat::awwal`], `n` is `baqiya[n - 1]`.
    dawr: usize,
}

impl core::fmt::Debug for MakhzanMuaqqat {
    fn fmt(&self, matbaa: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Sizes, not contents: a pool dump is thousands of glyph positions
        // nobody reads twice.
        matbaa
            .debug_struct("MakhzanMuaqqat")
            .field("adad", &self.adad())
            .field("dawr", &self.dawr)
            .finish_non_exhaustive()
    }
}

impl MakhzanMuaqqat {
    /// Builds a pool of `adad` empty buffers.
    ///
    /// This is where [`TaaribKhiyaratSiyaq::adad_makhazin`](crate::anwa::TaaribKhiyaratSiyaq)
    /// lands. Zero means the caller expressed no preference and gets the
    /// smallest pool that still preserves a previous result, which is two; one
    /// is raised to two for the same reason, because rotation over a single
    /// slot preserves nothing. Anything past sixty-four is clamped: the count
    /// crosses the ABI from a process this library does not trust, and a
    /// mistaken value must cost a few empty structs, not a real allocation.
    ///
    /// The buffers start empty and grow as they are used, so an oversized pool
    /// wastes almost nothing until text actually passes through it.
    #[must_use]
    pub fn jadeed(adad: u32) -> Self {
        let matlub = usize::try_from(adad).unwrap_or(AQSA_MAKHAZIN);
        let adad = matlub.clamp(ADNA_MAKHAZIN, AQSA_MAKHAZIN);
        let mut baqiya = Vec::new();
        baqiya.resize_with(adad.saturating_sub(1), TakhtitNass::default);
        Self { awwal: TakhtitNass::default(), baqiya, dawr: 0 }
    }

    /// The buffer under the cursor, emptied and ready to lay out into.
    ///
    /// Emptying keeps the buffer's allocations, which is the reuse the hot
    /// path is built on. The cursor does not move: calling this twice without
    /// [`MakhzanMuaqqat::dawwir`] in between hands back the same buffer,
    /// re-emptied — which is exactly right for a layout that failed and is
    /// being retried, since nobody kept the half-written attempt.
    ///
    /// The protocol an entry point follows: acquire, fill, copy out, and only
    /// then rotate — so the layout it just produced survives until the pool
    /// has gone all the way around.
    pub fn takhtit(&mut self) -> &mut TakhtitNass {
        if self.dawr > self.baqiya.len() {
            // A cursor past the pool would mean the pool shrank underneath it,
            // which nothing here does; resetting is the total answer.
            self.dawr = 0;
        }
        let fahras = self.dawr.checked_sub(1);
        // The two fields are borrowed up front, disjointly, so that falling
        // back to the first slot is a different borrow rather than a re-borrow
        // the compiler must refuse.
        let Self { awwal, baqiya, .. } = self;
        let mawjud = match fahras {
            Some(raqm) => baqiya.get_mut(raqm),
            None => None,
        };
        let makhzan = match mawjud {
            Some(makhzan) => makhzan,
            None => awwal,
        };
        makhzan.amsah();
        makhzan
    }

    /// Advances the cursor to the next slot, wrapping at the end.
    ///
    /// Called after a produced layout has been handed to the caller, so that
    /// the next acquisition builds into a different buffer and the one just
    /// produced stays intact while anything is still acting on it — the
    /// grow-and-retry step of the capacity negotiation being the case that
    /// matters most, since it happens on every buffer's first frame.
    pub const fn dawwir(&mut self) {
        self.dawr =
            if self.dawr >= self.baqiya.len() { 0 } else { self.dawr.saturating_add(1) };
    }

    /// How many buffers the pool holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.baqiya.len().saturating_add(1)
    }

    /// Releases every buffer's grown memory, keeping the pool's shape.
    ///
    /// The opposite trade from [`TakhtitNass::amsah`]: that keeps allocations
    /// so the next layout is free, this drops them so the process gets its
    /// pages back. It is the pool's share of a context-wide
    /// [`Siyaq::amsah`](crate::hayat::Siyaq::amsah), for a host that has
    /// finished a scene and wants the memory more than the warm buffers.
    pub fn amsah(&mut self) {
        self.awwal = TakhtitNass::default();
        for makhzan in &mut self.baqiya {
            *makhzan = TakhtitNass::default();
        }
    }
}

// ---------------------------------------------------------------------------
// Writing a layout into the caller's arrays
// ---------------------------------------------------------------------------

/// One glyph, engine form to frozen form.
///
/// Field for field, with the one representational change spelled out: the
/// engine's `bool` mark flag becomes the [`TAARIB_HARF_ALAMA`] bit, because a
/// `bool` has no defined ABI representation and a `u8` of named bits does.
const fn harf_jisr(harf: &Harf) -> TaaribHarf {
    TaaribHarf {
        muarrif: harf.muarrif,
        anqud: harf.anqud,
        s: harf.s,
        a: harf.a,
        taqaddum: harf.taqaddum,
        nitaq: harf.nitaq,
        khatt: harf.khatt,
        alam: if harf.alama { TAARIB_HARF_ALAMA } else { 0 },
    }
}

/// One line, engine form to frozen form.
///
/// [`SatrMansuq::huruf`] is a `Range` into the layout's flat glyph vector;
/// [`TaaribSatr`] carries the same information as a first index and a count,
/// because a Rust `Range` has no stable C layout and an index plus a count is
/// how every consumer walks it anyway. The direction and last-line booleans
/// become bits for the same reason the mark flag does.
const fn satr_jisr(satr: &SatrMansuq) -> TaaribSatr {
    let mut alam = 0;
    if satr.akhir {
        alam |= TAARIB_SATR_AKHIR;
    }
    if matches!(satr.ittijah, Ittijah::Yameen) {
        alam |= TAARIB_SATR_YAMEEN;
    }
    TaaribSatr {
        awwal_harf: satr.huruf.start,
        adad_huruf: satr.huruf.end.saturating_sub(satr.huruf.start),
        bidayat_mantiqi: satr.mantiqi.start,
        nihayat_mantiqi: satr.mantiqi.end,
        asas: satr.asas,
        bidaya: satr.bidaya,
        ard: satr.ard,
        irtifa: satr.irtifa,
        suud: satr.suud,
        hubut: satr.hubut,
        dabt: satr.dabt,
        alam,
    }
}

/// The overflow report, engine form to frozen form.
///
/// The one lossy edge is the optional available height: the engine records
/// "none was given" and the frozen structure records zero, exactly as its own
/// documentation says it does.
fn tajawuz_jisr(taqreer: &TaqreerTajawuz) -> TaaribTaqreerTajawuz {
    TaaribTaqreerTajawuz {
        ard: taqreer.ard,
        ard_mutah: taqreer.ard_mutah,
        irtifa: taqreer.irtifa,
        irtifa_mutah: taqreer.irtifa_mutah.unwrap_or(0.0),
        awwal_satr: taqreer.awwal_satr,
        adad_sutur: taqreer.adad_sutur,
    }
}

/// Writes a finished layout into a caller-owned buffer, negotiating capacity.
///
/// This is the single funnel between the engine's layout result and the frozen
/// ABI form — every entry point that produces glyphs for a caller ends here,
/// whether the layout was just shaped or came out of the cache, so the two
/// representations are converted in exactly one place and cannot drift.
///
/// The negotiation: the required glyph and line counts are always written into
/// `adad_huruf` and `adad_sutur`. When either array's declared capacity is
/// smaller than required, **nothing else** is written and the return is
/// [`TAARIB_SIAT_QASIRA`] — the caller grows once, retries, and after the
/// first few frames the buffer never grows again. When both fit, every glyph
/// and every line is written, the summary fields (`ard`, `irtifa`, `hajm`) are
/// set, the flags are set — including [`TAARIB_TAKHTIT_MAKHZAN`] when
/// `min_makhzan` says the layout was served from the cache — and the return is
/// [`TAARIB_NAJAH`].
///
/// The overflow report is written unconditionally on success — zeroed when
/// there was no overflow — so the caller never reads uninitialised memory even
/// if it forgets to test [`TAARIB_TAKHTIT_TAJAWUZ`] first.
///
/// On every failure path the returned code has already been stashed as this
/// thread's last error, so an entry point propagates it as-is.
///
/// # Safety
///
/// The caller guarantees all of the following, and every write below relies on
/// that guarantee being complete:
///
/// - `hadaf` points to a [`TaaribMakhzanTakhtit`] that is valid for reads and
///   writes and is not aliased by anything else for the duration of the call.
///   Null is tolerated and reported as [`TAARIB_MUASHIR_BATIL`] rather than
///   dereferenced.
/// - When `hadaf.huruf` is non-null, it is valid for writes of
///   `hadaf.siaat_huruf` elements of [`TaaribHarf`]; when `hadaf.sutur` is
///   non-null, it is valid for writes of `hadaf.siaat_sutur` elements of
///   [`TaaribSatr`]. A null array with a non-zero required count is reported
///   as [`TAARIB_MUASHIR_BATIL`], never trusted.
/// - The two arrays do not overlap each other or the descriptor itself.
pub unsafe fn iktub_takhtit(
    takhtit: &TakhtitNass,
    hadaf: *mut TaaribMakhzanTakhtit,
    min_makhzan: bool,
) -> i32 {
    if hadaf.is_null() {
        return sajjil_ramz(TAARIB_MUASHIR_BATIL);
    }
    // SAFETY: `hadaf` is non-null, and the caller guarantees it points to a
    // valid, writable, unaliased descriptor for the duration of the call.
    let makhzan = unsafe { &mut *hadaf };

    let adad_huruf = takhtit.huruf.len();
    let adad_sutur = takhtit.sutur.len();
    makhzan.adad_huruf = adad_huruf;
    makhzan.adad_sutur = adad_sutur;

    if makhzan.siaat_huruf < adad_huruf || makhzan.siaat_sutur < adad_sutur {
        return sajjil_ramz(TAARIB_SIAT_QASIRA);
    }
    if (adad_huruf > 0 && makhzan.huruf.is_null())
        || (adad_sutur > 0 && makhzan.sutur.is_null())
    {
        return sajjil_ramz(TAARIB_MUASHIR_BATIL);
    }

    for (fahras, harf) in takhtit.huruf.iter().enumerate() {
        // SAFETY: `huruf` is non-null (checked above) and the caller
        // guarantees it is valid for writes of `siaat_huruf` elements;
        // `fahras < adad_huruf <= siaat_huruf`, so the write is in bounds of
        // one allocation the caller owns.
        unsafe { makhzan.huruf.add(fahras).write(harf_jisr(harf)) };
    }
    for (fahras, satr) in takhtit.sutur.iter().enumerate() {
        // SAFETY: as for the glyphs — `sutur` is non-null and valid for
        // `siaat_sutur` writes, and `fahras < adad_sutur <= siaat_sutur`.
        unsafe { makhzan.sutur.add(fahras).write(satr_jisr(satr)) };
    }

    makhzan.ard = takhtit.ard;
    makhzan.irtifa = takhtit.irtifa;
    makhzan.hajm = takhtit.hajm;

    let mut alam = 0;
    if takhtit.ittijah == Ittijah::Yameen {
        alam |= TAARIB_TAKHTIT_YAMEEN;
    }
    if takhtit.maqsus {
        alam |= TAARIB_TAKHTIT_MAQSUS;
    }
    if min_makhzan {
        alam |= TAARIB_TAKHTIT_MAKHZAN;
    }
    makhzan.tajawuz = match takhtit.tajawuz {
        Some(ref taqreer) => {
            alam |= TAARIB_TAKHTIT_TAJAWUZ;
            tajawuz_jisr(taqreer)
        },
        None => TaaribTaqreerTajawuz::default(),
    };
    makhzan.alam = alam;

    TAARIB_NAJAH
}

// ---------------------------------------------------------------------------
// Reading the caller's memory
// ---------------------------------------------------------------------------

/// Turns a caller's pointer and length into text.
///
/// One of the two places in this crate where an incoming pointer becomes a
/// Rust reference — every entry point that accepts text routes it through
/// here, so the null, overflow and encoding checks run exactly once, in one
/// place. Rejects a null pointer with a non-zero length, a length no
/// allocation could have, and bytes that are not UTF-8. A zero length is an
/// empty string whatever the pointer says, including null, because "no text"
/// is an ordinary input and not a mistake.
///
/// On failure the returned code has already been stashed as this thread's
/// last error, so an entry point returns it directly.
///
/// # Errors
///
/// [`TAARIB_MUASHIR_BATIL`] for a null pointer with a non-zero length,
/// [`TAARIB_QEEMA_BATILA`] for a length larger than any single allocation can
/// be, and [`TAARIB_TARMIZ_BATIL`] for bytes that are not valid UTF-8.
///
/// # Safety
///
/// The caller guarantees, and every later use of the returned reference relies
/// on, all of the following:
///
/// - When `muashir` is non-null, it points to `tul` initialised, readable
///   bytes inside one allocation.
/// - Those bytes are not written by anyone — the caller included — for as
///   long as the returned reference lives.
/// - The lifetime `'a` is chosen by the caller and is unbounded here: the
///   caller must not let the reference outlive the bytes. Inside this library
///   `'a` never escapes the entry point that made the call, which is the whole
///   of how that obligation is met.
pub unsafe fn iqra_nass<'a>(muashir: *const u8, tul: usize) -> Result<&'a str, i32> {
    if tul == 0 {
        return Ok("");
    }
    if muashir.is_null() {
        return Err(sajjil_ramz(TAARIB_MUASHIR_BATIL));
    }
    if isize::try_from(tul).is_err() {
        return Err(sajjil_ramz(TAARIB_QEEMA_BATILA));
    }
    // SAFETY: `muashir` is non-null, the caller guarantees `tul` initialised
    // readable bytes in one allocation that nobody mutates for `'a`, and `tul`
    // was just shown to fit an `isize` as `from_raw_parts` requires.
    let bayt = unsafe { core::slice::from_raw_parts(muashir, tul) };
    match core::str::from_utf8(bayt) {
        Ok(nass) => Ok(nass),
        Err(_) => Err(sajjil_ramz(TAARIB_TARMIZ_BATIL)),
    }
}

/// Turns a caller's pointer and element count into a slice.
///
/// The other of the two fences, used for style spans, feature settings, and
/// every other array a caller hands in. Rejects a null pointer with a non-zero
/// count, a misaligned pointer, and a count whose byte size overflows. A zero
/// count is an empty slice whatever the pointer says, including null — a
/// caller with no style spans passes exactly that, and it is not an error.
///
/// On failure the returned code has already been stashed as this thread's
/// last error, so an entry point returns it directly.
///
/// # Errors
///
/// [`TAARIB_MUASHIR_BATIL`] for a null or misaligned pointer with a non-zero
/// count, and [`TAARIB_QEEMA_BATILA`] for a count whose size in bytes
/// overflows or exceeds what any single allocation can be.
///
/// # Safety
///
/// The caller guarantees, and every later use of the returned slice relies on,
/// all of the following:
///
/// - When `muashir` is non-null, it points to `adad` initialised elements
///   inside one allocation, each of them a valid value of `T`. The `repr(C)`
///   input structures of this ABI are valid at every bit pattern their fields
///   admit, which is what makes this guarantee one a C caller can actually
///   give.
/// - That memory is not written by anyone for as long as the returned slice
///   lives.
/// - The lifetime `'a` is chosen by the caller and is unbounded here: the
///   caller must not let the slice outlive the memory. Inside this library
///   `'a` never escapes the entry point that made the call.
pub unsafe fn iqra_shariha<'a, T>(muashir: *const T, adad: usize) -> Result<&'a [T], i32> {
    if adad == 0 {
        return Ok(&[]);
    }
    if muashir.is_null() || !muashir.addr().is_multiple_of(align_of::<T>()) {
        return Err(sajjil_ramz(TAARIB_MUASHIR_BATIL));
    }
    let Some(bayt) = adad.checked_mul(size_of::<T>()) else {
        return Err(sajjil_ramz(TAARIB_QEEMA_BATILA));
    };
    if isize::try_from(bayt).is_err() {
        return Err(sajjil_ramz(TAARIB_QEEMA_BATILA));
    }
    // SAFETY: `muashir` is non-null and aligned for `T` (checked above), the
    // caller guarantees `adad` initialised, valid, unaliased-for-writes
    // elements in one allocation, and the total byte size was just shown to
    // fit an `isize` as `from_raw_parts` requires.
    Ok(unsafe { core::slice::from_raw_parts(muashir, adad) })
}
