//! Import table redirection: repoint what a module calls, without touching what it is.

use core::ffi::c_void;

use crate::khata::NatijatHaqn;

/// One redirected import, and what to put back.
#[derive(Debug)]
pub struct IstiradMakhtuf {
    khana: *mut *mut c_void,
    asli: *mut c_void,
    badil: *mut c_void,
    ism: String,
}

impl IstiradMakhtuf {
    /// The original the replacement must call.
    #[must_use]
    pub const fn asl(&self) -> *mut c_void {
        self.asli
    }

    /// Which import this is, as the log names it.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
    }
}

/// Redirects one named import of a loaded module.
///
/// Patching the import address table rather than the function itself: the
/// callee's own bytes are never touched, so an anti-tamper product that hashes
/// its own code sees nothing changed, and a module that resolves the same
/// function by any other route still reaches the real one. What changes is only
/// what *this* module calls.
///
/// # Errors
///
/// [`crate::KhataHaqn::RaasGhayrMafhum`] when the module's headers are not a
/// shape this reader understands, [`crate::KhataHaqn::IstiradMafqud`] when the
/// module imports no such symbol from that library,
/// [`crate::KhataHaqn::HimayaGhayrQabila`] when the table cannot be made
/// writable, and [`crate::KhataHaqn::KitabaMuhmala`] when the write does not
/// take effect.
///
/// # Safety
///
/// `qaida` must be the base address of a module mapped in this process, and
/// `badil` must have the imported function's exact signature and calling
/// convention and outlive the redirection.
#[cfg(windows)]
pub unsafe fn ikhtif_istirad(
    qaida: *mut c_void,
    wahda: &str,
    ism: &str,
    badil: *mut c_void,
) -> NatijatHaqn<IstiradMakhtuf> {
    use crate::hirasa::iktub_muashir;
    use crate::khata::KhataHaqn;
    use windows::Win32::System::Diagnostics::Debug::{
        IMAGE_DIRECTORY_ENTRY_IMPORT, IMAGE_NT_HEADERS64,
    };
    use windows::Win32::System::SystemServices::{
        IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_IMPORT_BY_NAME, IMAGE_IMPORT_DESCRIPTOR,
        IMAGE_NT_SIGNATURE, IMAGE_ORDINAL_FLAG64,
    };
    // The thunk is the one import-table type the bindings file under
    // `WindowsProgramming` rather than beside the descriptor it belongs to.
    use windows::Win32::System::WindowsProgramming::IMAGE_THUNK_DATA64;

    let raas_ghayr = |sabab: &str| KhataHaqn::RaasGhayrMafhum {
        qaida: qaida as usize as u64,
        sabab: sabab.to_owned(),
    };

    // SAFETY: the caller guarantees `qaida` is a mapped module base, and every
    // PE image begins with a DOS header.
    let dos = unsafe { &*qaida.cast::<IMAGE_DOS_HEADER>() };
    if dos.e_magic != IMAGE_DOS_SIGNATURE {
        return Err(raas_ghayr("the module does not begin with a DOS signature"));
    }

    // SAFETY: `e_lfanew` is the header's own offset to the NT headers, inside
    // the same mapped image.
    let nt = unsafe { qaida.byte_offset(dos.e_lfanew as isize).cast::<IMAGE_NT_HEADERS64>() };
    // SAFETY: as above.
    let nt = unsafe { &*nt };
    if nt.Signature != IMAGE_NT_SIGNATURE {
        return Err(raas_ghayr("the NT headers carry no PE signature"));
    }

    let dalil = nt.OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT.0 as usize];
    if dalil.VirtualAddress == 0 || dalil.Size == 0 {
        return Err(raas_ghayr("the module has no import directory"));
    }

    // SAFETY: the import directory's RVA is inside the mapped image by the
    // header's own account, and a loaded module is mapped at its full size.
    let mut wasf = unsafe {
        qaida.byte_offset(dalil.VirtualAddress as isize).cast::<IMAGE_IMPORT_DESCRIPTOR>()
    };

    loop {
        // SAFETY: the descriptor array is inside the mapped image and is
        // terminated by an all-zero entry, which the check below reads.
        let hali = unsafe { *wasf };
        if hali.Name == 0 {
            break;
        }

        // SAFETY: `Name` is an RVA to a NUL-terminated ASCII library name
        // inside the same image.
        let ism_wahda = unsafe {
            core::ffi::CStr::from_ptr(qaida.byte_offset(hali.Name as isize).cast::<i8>())
        };
        let ism_wahda = ism_wahda.to_string_lossy();

        if ism_wahda.eq_ignore_ascii_case(wahda) {
            // The original-first-thunk array holds the names; the first-thunk
            // array holds the addresses the loader wrote. Walking both in step
            // is what lets a name be matched and its address be replaced.
            // SAFETY: both RVAs are inside the mapped image.
            let asma = unsafe {
                let rva = hali.Anonymous.OriginalFirstThunk;
                let rva = if rva == 0 { hali.FirstThunk } else { rva };
                qaida.byte_offset(rva as isize).cast::<IMAGE_THUNK_DATA64>()
            };
            // SAFETY: as above.
            let anawin = unsafe {
                qaida.byte_offset(hali.FirstThunk as isize).cast::<IMAGE_THUNK_DATA64>()
            };

            let mut fihris: isize = 0;
            loop {
                // SAFETY: both arrays are inside the mapped image and are
                // terminated by an all-zero entry.
                let ism_thunk = unsafe { *asma.offset(fihris) };
                // SAFETY: as above.
                let unwan_thunk = unsafe { anawin.offset(fihris) };
                // SAFETY: as above.
                if unsafe { ism_thunk.u1.AddressOfData } == 0 {
                    break;
                }
                // An ordinal import carries no name to match against.
                // SAFETY: as above.
                let khaam = unsafe { ism_thunk.u1.Ordinal };
                if khaam & IMAGE_ORDINAL_FLAG64 == 0 {
                    // SAFETY: for a named import, `AddressOfData` is an RVA to
                    // an IMAGE_IMPORT_BY_NAME inside the same image.
                    let bism = unsafe {
                        qaida
                            .byte_offset(ism_thunk.u1.AddressOfData as isize)
                            .cast::<IMAGE_IMPORT_BY_NAME>()
                    };
                    // SAFETY: `Name` is the first byte of a NUL-terminated
                    // ASCII string inside the image.
                    let ism_daala = unsafe {
                        core::ffi::CStr::from_ptr(std::ptr::addr_of!((*bism).Name).cast::<i8>())
                    };
                    if ism_daala.to_bytes() == ism.as_bytes() {
                        let khana = unwan_thunk.cast::<*mut c_void>();
                        // SAFETY: the thunk is inside the mapped image and is
                        // pointer-aligned.
                        let asli = unsafe { khana.read_volatile() };
                        let kamil = format!("{wahda}!{ism}");
                        // SAFETY: as above.
                        unsafe { iktub_muashir(khana, badil, asli, &kamil) }?;
                        return Ok(IstiradMakhtuf { khana, asli, badil, ism: kamil });
                    }
                }
                fihris = fihris.saturating_add(1);
            }
        }

        // SAFETY: descriptors are a contiguous array inside the image.
        wasf = unsafe { wasf.offset(1) };
    }

    Err(KhataHaqn::IstiradMafqud {
        wahda: wahda.to_owned(),
        ism: ism.to_owned(),
        qaida: qaida as usize as u64,
    })
}

/// Import redirection is a PE concept; ELF and Mach-O interposition is done by
/// the dynamic loader before Taarib's module exists.
///
/// # Errors
///
/// Always [`crate::KhataHaqn::GhayrMutahaHuna`]. On Linux and macOS the same
/// effect is `LD_PRELOAD`/`DYLD_INSERT_LIBRARIES`, which `taarib-tathbeet`
/// writes at install time — there is nothing to patch at runtime.
///
/// # Safety
///
/// Nothing is dereferenced; the signature matches the Windows one so callers
/// need no `cfg`.
#[cfg(not(windows))]
pub const unsafe fn ikhtif_istirad(
    _qaida: *mut c_void,
    _wahda: &str,
    _ism: &str,
    _badil: *mut c_void,
) -> NatijatHaqn<IstiradMakhtuf> {
    Err(crate::khata::KhataHaqn::GhayrMutahaHuna { amal: "import table redirection" })
}

/// Puts a redirected import back, unless something else took the slot.
///
/// # Errors
///
/// [`crate::KhataHaqn::FakkKhatfFashil`] when the slot holds a third party's
/// pointer — restoring then would break whatever hooked it afterwards — or when
/// the write-back fails.
pub fn fukk_istirad(makhtuf: &IstiradMakhtuf) -> NatijatHaqn<()> {
    // SAFETY: `khana` came from `ikhtif_istirad` on a module still mapped —
    // which the redirection itself keeps true — and is pointer-aligned.
    let alaan = unsafe { makhtuf.khana.read_volatile() };
    if !std::ptr::eq(alaan.cast_const(), makhtuf.badil.cast_const()) {
        return Err(crate::khata::KhataHaqn::FakkKhatfFashil {
            mawdi: makhtuf.ism.clone(),
            sabab: "the import slot holds neither this hook nor the original, so something else \
                    redirected it afterwards and restoring now would break it"
                .to_owned(),
        });
    }
    // SAFETY: as above.
    unsafe {
        crate::hirasa::iktub_muashir(makhtuf.khana, makhtuf.asli, makhtuf.badil, &makhtuf.ism)
    }
}
