//! رموز الأخطاء — the status code space, and how a rich failure survives a
//! boundary that can only carry an integer.
//!
//! Every entry point returns an `int32`. Zero is success and every failure is
//! negative, so a caller in any language can write `if (r < 0)` and be right.
//!
//! An integer is not enough to act on, though, and the whole error model of this
//! product exists so that a failure arrives carrying a permanent code, a
//! sentence in Arabic, a sentence in English and one concrete next action. So
//! the full [`Khata`] is stashed in thread-local storage on its way out, and the
//! caller retrieves whichever parts it needs with
//! [`taarib_khata_ramz`](crate::taarib_khata_ramz),
//! [`taarib_khata_nass`](crate::taarib_khata_nass) and
//! [`taarib_khata_khutwa`](crate::taarib_khata_khutwa).
//!
//! Thread-local, not global, for a reason that matters inside a game: the render
//! thread and a background loader can both be in this library at once, and a
//! shared last-error slot would let one thread's failure be reported as the
//! other's — which is worse than no diagnostic at all, because it sends whoever
//! reads it looking in the wrong place.

use std::cell::RefCell;

use taarib_usus::Lugha;
use taarib_usus::khata::{Khata, Khutwa};

/// The operation succeeded.
pub const TAARIB_NAJAH: i32 = 0;

/// Something failed that no more specific code describes. The stashed [`Khata`]
/// still names it exactly; this is the code, not the diagnosis.
pub const TAARIB_KHATA_AAM: i32 = -1;

/// A required pointer argument was null.
pub const TAARIB_MUASHIR_BATIL: i32 = -2;

/// A handle was not one this library issued, or was destroyed already.
///
/// This is the use-after-free that generation tagging exists to catch. It is
/// reported rather than executed, which is the difference between a bug report
/// naming a stale handle and a crash dump in someone's game.
pub const TAARIB_MAQBAD_BATIL: i32 = -3;

/// The caller's buffer is too small. The required capacity has been written to
/// the out-parameter and nothing else was written.
///
/// Not an error in the usual sense — it is the negotiation the allocation-free
/// hot path is built on. A caller grows once and retries, and from then on the
/// buffer is big enough forever.
pub const TAARIB_SIAT_QASIRA: i32 = -4;

/// The caller was built against a different major version of this ABI.
pub const TAARIB_ISDAR_GHAYR_MUTAWAFIQ: i32 = -5;

/// A string argument was not valid UTF-8.
pub const TAARIB_TARMIZ_BATIL: i32 = -6;

/// A font was rejected, or could not be read.
///
/// The stashed error names the missing table or feature, because "this font has
/// no GSUB table and cannot join Arabic letters" is a sentence a user can act on
/// and "font error" is not.
pub const TAARIB_KHATT_MARFUD: i32 = -7;

/// Shaping produced nothing for text that is not empty.
pub const TAARIB_TASHKEEL_FASHIL: i32 = -8;

/// An allocation failed. Reported rather than aborted: this library runs inside
/// somebody else's process and does not get to decide that the process ends.
pub const TAARIB_DHAKIRA: i32 = -9;

/// An argument was structurally fine but its value is not usable — a negative
/// width, a size of zero, a style span that points outside its text.
pub const TAARIB_QEEMA_BATILA: i32 = -10;

/// A panic was caught at the boundary and converted into a status.
///
/// A panic unwinding into a game's C# or C++ frame would be a crash the player
/// blames on the game. Every entry point catches, so this code means the library
/// has a bug worth reporting — and the game is still running to report it.
pub const TAARIB_INHIYAR: i32 = -11;

/// The atlas is full and nothing in it may be evicted, because everything in it
/// is referenced by the frame currently being drawn.
pub const TAARIB_LAWHA_MUMTALIA: i32 = -12;

/// The operation is not available in this build — a feature compiled out, or a
/// platform that does not offer what was asked for.
pub const TAARIB_GHAYR_MADUM: i32 = -13;

/// The library has not been initialised, or was shut down.
pub const TAARIB_GHAYR_MUHAYYAA: i32 = -14;

thread_local! {
    static AKHIR_KHATA: RefCell<Option<Khata>> = const { RefCell::new(None) };
    static AKHIR_RAMZ: RefCell<i32> = const { RefCell::new(TAARIB_NAJAH) };
}

/// Stashes a failure for this thread and returns the status code that describes
/// it.
///
/// Called at every entry point that fails, so that the caller's `int32` and the
/// retrievable sentence can never disagree about what went wrong.
#[must_use]
pub fn sajjil(khata: &Khata) -> i32 {
    let ramz = ramz_min_khata(khata);
    let mahfuz = khata.clone();
    AKHIR_KHATA.with_borrow_mut(|khana| *khana = Some(mahfuz));
    AKHIR_RAMZ.with_borrow_mut(|khana| *khana = ramz);
    ramz
}

/// Stashes a failure that has no [`Khata`] behind it — a null pointer, an
/// invalid handle, a caught panic — so that the retrieval functions still have
/// something honest to return.
#[must_use]
pub fn sajjil_ramz(ramz: i32) -> i32 {
    AKHIR_KHATA.with_borrow_mut(|khana| *khana = None);
    AKHIR_RAMZ.with_borrow_mut(|khana| *khana = ramz);
    ramz
}

/// Clears this thread's stashed failure, called on every successful entry.
pub fn nazzif() {
    AKHIR_KHATA.with_borrow_mut(|khana| *khana = None);
    AKHIR_RAMZ.with_borrow_mut(|khana| *khana = TAARIB_NAJAH);
}

/// The status code of this thread's last failure.
#[must_use]
pub fn akhir_ramz() -> i32 {
    AKHIR_RAMZ.with_borrow(|khana| *khana)
}

/// Reads this thread's stashed failure, if there is one.
pub fn maa_akhir_khata<T>(amal: impl FnOnce(Option<&Khata>) -> T) -> T {
    AKHIR_KHATA.with_borrow(|khana| amal(khana.as_ref()))
}

/// The permanent code of this thread's last failure, as text, or an empty string
/// when the failure carried no [`Khata`].
#[must_use]
pub fn akhir_ramz_nass() -> String {
    maa_akhir_khata(|khata| khata.map_or_else(String::new, |q| q.ramz.to_string()))
}

/// The sentence for this thread's last failure, in the requested language.
#[must_use]
pub fn akhir_nass(lugha: Lugha) -> String {
    maa_akhir_khata(|khata| {
        khata.map_or_else(String::new, |q| match lugha {
            Lugha::Arabi => q.arabi.clone(),
            Lugha::Injilizi => q.injilizi.clone(),
        })
    })
}

/// The next action for this thread's last failure, as the discriminant the ABI
/// exposes.
#[must_use]
pub fn akhir_khutwa() -> i32 {
    maa_akhir_khata(|khata| khata.map_or(0, |q| raqm_khutwa(&q.khutwa)))
}

/// Maps a [`Khata`] onto the status code that best describes it.
///
/// The mapping is by the error's own permanent code block, not by string
/// matching or by guesswork: `TAARIB-E-2500..2599` is the font block, so every
/// failure in it is a font rejection, whatever its individual variant says.
/// A block gaining a new variant therefore gets the right status automatically.
const fn ramz_min_khata(khata: &Khata) -> i32 {
    use taarib_usus::khata::arqam;

    match khata.ramz.kutla() {
        arqam::KHATT => TAARIB_KHATT_MARFUD,
        arqam::LAWHA => TAARIB_LAWHA_MUMTALIA,
        arqam::SAFF => {
            // Within the layout block the distinction that matters to a caller
            // is whether the text could not be shaped at all — which means a
            // different font is the answer — or whether an argument was wrong,
            // which means the caller is.
            if khata.ramz.raqm() == arqam::SAFF + 7 {
                TAARIB_TASHKEEL_FASHIL
            } else {
                TAARIB_QEEMA_BATILA
            }
        },
        _ => TAARIB_KHATA_AAM,
    }
}

/// The discriminant an ABI caller sees for a next action.
///
/// Stable and additive: a new action takes the next free number, and a caller
/// that does not recognise a number shows the sentence without a button rather
/// than showing nothing.
const fn raqm_khutwa(khutwa: &Khutwa) -> i32 {
    match khutwa {
        Khutwa::LaShay => 0,
        Khutwa::AadaMuhawala => 1,
        Khutwa::AadaFahsMaktaba => 2,
        Khutwa::AadaFahsMuharrik => 3,
        Khutwa::IkhtiyarMasar { .. } => 4,
        Khutwa::IkhtiyarKhattAakhar => 5,
        Khutwa::FathIdadat { .. } => 6,
        Khutwa::FathTashkhis => 7,
        Khutwa::FathTaqreerTajawuz => 8,
        Khutwa::FathNusus => 9,
        Khutwa::TahdithTaarib => 10,
        Khutwa::IadatTarkibIttar => 11,
        Khutwa::IlghaTathbeet => 12,
        Khutwa::IadatMutabaqaBina => 13,
        Khutwa::TahaqquqSalamatLuba => 14,
        Khutwa::IblaghLilMusahim => 15,
        Khutwa::IblaghLilMalik => 16,
        Khutwa::TahrirMasaha => 17,
        Khutwa::ManhSalahiya => 18,
        Khutwa::FathTabaqa => 19,
        Khutwa::FathTilqai => 20,
    }
}

/// Writes a string into a caller-owned byte buffer, negotiating capacity.
///
/// The one string-returning convention in the whole ABI, used by every function
/// that hands text back. It never allocates on the caller's behalf and never
/// returns a pointer the caller has to free — both of which are ownership
/// questions, and every ownership question in an ABI is a leak or a double free
/// waiting for the one caller who reads the documentation differently.
///
/// Writes a trailing NUL when there is room for one, so C callers can treat the
/// buffer as a string without consulting the length.
///
/// # Safety
///
/// `hadaf` must either be null or point to at least `siaa` writable bytes, and
/// `matlub` must either be null or point to a writable `usize`.
pub unsafe fn iktub_nass(nass: &str, hadaf: *mut u8, siaa: usize, matlub: *mut usize) -> i32 {
    let tul = nass.len();
    if !matlub.is_null() {
        // SAFETY: the caller guarantees `matlub` is writable when non-null.
        unsafe { matlub.write(tul.saturating_add(1)) };
    }
    if hadaf.is_null() {
        return sajjil_ramz(TAARIB_MUASHIR_BATIL);
    }
    if siaa <= tul {
        return sajjil_ramz(TAARIB_SIAT_QASIRA);
    }
    // SAFETY: `hadaf` is non-null with at least `siaa` writable bytes, and
    // `siaa` is strictly greater than `tul`, so `tul + 1` bytes fit.
    unsafe {
        std::ptr::copy_nonoverlapping(nass.as_ptr(), hadaf, tul);
        hadaf.add(tul).write(0);
    }
    TAARIB_NAJAH
}

/// The language discriminant an ABI caller passes.
///
/// # Errors
///
/// Anything unrecognised resolves to Arabic, because Arabic is the primary text
/// of this product and a caller that passed a wrong number gets the real
/// sentence rather than nothing.
#[must_use]
pub const fn lugha_min_raqm(raqm: u32) -> Lugha {
    match raqm {
        1 => Lugha::Injilizi,
        _ => Lugha::Arabi,
    }
}
