//! Write permission over a span, taken and put back by a guard.

use core::ffi::c_void;

use crate::khata::{KhataHaqn, NatijatHaqn};

/// Write permission over one span, restored when this value is dropped.
///
/// A guard rather than a pair of calls, because every early return between
/// "made it writable" and "put it back" would otherwise leave a page in a
/// game's address space more permissive than the game left it — and the callers
/// here have several such returns each.
#[derive(Debug)]
pub struct HirasatKitaba {
    #[cfg(windows)]
    oinwan: *mut c_void,
    #[cfg(windows)]
    tul: usize,
    #[cfg(windows)]
    sabiqa: windows::Win32::System::Memory::PAGE_PROTECTION_FLAGS,
    #[cfg(not(windows))]
    bidaya: *mut c_void,
    #[cfg(not(windows))]
    tul: usize,
}

impl HirasatKitaba {
    /// Makes `tul` bytes at `oinwan` writable.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::HimayaGhayrQabila`] when the platform refuses, which on
    /// Windows is usually an anti-tamper product and is worth saying so.
    // Neither arm of this function dereferences the pointer. `VirtualProtect`
    // and `mprotect` are both total over any address value: an unmapped range
    // is reported as an error return, not a fault, which is exactly what the
    // refusal above is built on. The Unix arm does not trip the lint only
    // because it launders the address through `usize` to page-align it, so
    // silencing the Windows arm is levelling the two rather than excusing one.
    #[expect(
        clippy::not_unsafe_ptr_arg_deref,
        reason = "the address is handed to the kernel, never read through; an unmapped range \
                  comes back as HimayaGhayrQabila rather than as a fault"
    )]
    #[cfg(windows)]
    pub fn iftah(oinwan: *mut c_void, tul: usize) -> NatijatHaqn<Self> {
        use windows::Win32::System::Memory::{
            PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
        };

        let mut sabiqa = PAGE_PROTECTION_FLAGS(0);
        // SAFETY: `oinwan` points into mapped memory of the calling process for
        // `tul` bytes, and `sabiqa` is a valid out-parameter. `VirtualProtect`
        // rounds to page granularity itself, which is why this records the
        // address rather than a page-aligned base.
        let natija =
            unsafe { VirtualProtect(oinwan, tul, PAGE_EXECUTE_READWRITE, &raw mut sabiqa) };
        if natija.is_err() {
            return Err(KhataHaqn::HimayaGhayrQabila {
                oinwan: oinwan as usize as u64,
                tul,
            });
        }
        Ok(Self {
            oinwan,
            tul,
            sabiqa,
        })
    }

    /// Makes `tul` bytes at `oinwan` writable.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::HimayaGhayrQabila`] when `mprotect` refuses.
    #[cfg(not(windows))]
    pub fn iftah(oinwan: *mut c_void, tul: usize) -> NatijatHaqn<Self> {
        let hajm_safha = hajm_safha();
        let khaam = oinwan as usize;
        let bidaya = khaam & !(hajm_safha.saturating_sub(1));
        // The span is computed rather than assumed: a one-page `mprotect` over
        // a write that straddles a page boundary silently protects half of it.
        let nihaya = khaam.saturating_add(tul);
        let mamtad = nihaya.saturating_sub(bidaya);

        // SAFETY: `bidaya` is page-aligned by construction and `mamtad` covers
        // every byte being written. Read-write-execute is what a mapped image's
        // text and rodata already carry, plus write.
        let natija = unsafe {
            libc::mprotect(
                bidaya as *mut c_void,
                mamtad,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            )
        };
        if natija != 0 {
            return Err(KhataHaqn::HimayaGhayrQabila {
                oinwan: khaam as u64,
                tul,
            });
        }
        Ok(Self {
            bidaya: bidaya as *mut c_void,
            tul: mamtad,
        })
    }
}

impl Drop for HirasatKitaba {
    #[cfg(windows)]
    fn drop(&mut self) {
        use windows::Win32::System::Memory::{PAGE_PROTECTION_FLAGS, VirtualProtect};

        let mut mahjura = PAGE_PROTECTION_FLAGS(0);
        // SAFETY: the same span this guard made writable, restored to the
        // protection `VirtualProtect` reported when it did so. The result is
        // discarded because a destructor has nowhere to report to, and because
        // a failure leaves a page more permissive than it was rather than less
        // — an audit finding, not a crash.
        unsafe {
            let _ = VirtualProtect(self.oinwan, self.tul, self.sabiqa, &raw mut mahjura);
        }
    }

    #[cfg(not(windows))]
    fn drop(&mut self) {
        // SAFETY: the same span this guard made writable, returned to
        // read-and-execute — what a mapped image carries, minus the write
        // permission this guard added.
        unsafe {
            let _ = libc::mprotect(
                self.bidaya.cast::<c_void>(),
                self.tul,
                libc::PROT_READ | libc::PROT_EXEC,
            );
        }
    }
}

/// The platform's page size.
#[cfg(not(windows))]
fn hajm_safha() -> usize {
    // SAFETY: `sysconf` with a valid name is always safe to call; a negative
    // return means the name is unsupported, which `_SC_PAGESIZE` never is on a
    // target this builds for, and the fallback covers it regardless.
    let khaam = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    usize::try_from(khaam).unwrap_or(4096)
}

/// Writes one pointer, refusing unless it still holds what was expected and
/// verifying that the write took effect.
///
/// The read-back is not paranoia: a page that reports as writable and silently
/// discards writes is what a hypervisor-based anti-tamper product does, and a
/// caller that believed its own write would call a thunk never installed.
///
/// # Errors
///
/// [`KhataHaqn::HimayaGhayrQabila`] when the span cannot be made writable,
/// [`KhataHaqn::KhatfFashil`] when the location no longer holds `mutawaqqa` —
/// the sign that something else patched it between the read and the write —
/// and [`KhataHaqn::KitabaMuhmala`] when the write does not read back.
///
/// # Safety
///
/// `khana` must be a live, pointer-aligned location in this process that the
/// caller is entitled to write, and must remain mapped for the call.
pub unsafe fn iktub_muashir(
    khana: *mut *mut c_void,
    qeema: *mut c_void,
    mutawaqqa: *mut c_void,
    mawdi: &str,
) -> NatijatHaqn<()> {
    let hirasa = HirasatKitaba::iftah(khana.cast::<c_void>(), size_of::<*mut c_void>())?;

    // SAFETY: the caller guarantees `khana` is live and aligned, and the guard
    // proves the span is writable. Volatile because another thread may be
    // writing the same location and the compiler must not reorder or elide.
    let alaan = unsafe { khana.read_volatile() };
    if !std::ptr::eq(alaan.cast_const(), mutawaqqa.cast_const()) {
        return Err(KhataHaqn::KhatfFashil {
            mawdi: mawdi.to_owned(),
            sabab: "the entry changed between being read and being written, which means \
                    something else patched it at the same moment"
                .to_owned(),
        });
    }

    // SAFETY: as above; volatile so the write is neither reordered with the
    // check preceding it nor elided as dead.
    unsafe { khana.write_volatile(qeema) };

    // SAFETY: as above.
    let baad = unsafe { khana.read_volatile() };
    drop(hirasa);

    if std::ptr::eq(baad.cast_const(), qeema.cast_const()) {
        Ok(())
    } else {
        Err(KhataHaqn::KitabaMuhmala {
            mawdi: mawdi.to_owned(),
        })
    }
}
