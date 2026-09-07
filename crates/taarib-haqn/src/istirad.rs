//! Import table redirection: repoint what a module calls, without touching what it is.
//!
//! ## The image is the process's own width, and so are its structures
//!
//! A PE image is either PE32 or PE32+, and the two disagree about the layout of
//! the optional header — which is where the import directory's address is read
//! from — and about the width of a thunk. A module mapped in *this* process is
//! always the process's own width: a 32-bit process loads only PE32 modules and
//! a 64-bit one only PE32+. So the choice is `target_pointer_width`, decided at
//! compile time, and there is no runtime branch to get wrong.
//!
//! This matters more than it sounds. Reading a PE32 image through PE32+
//! structures does not fail: the signature still matches, the directory is read
//! from the wrong offset, and the walk then writes a pointer at an address
//! computed from a garbage RVA — inside somebody's game. The first 32-bit game
//! this crate met is `bio4.exe`, which is why the aliases below exist.

use core::ffi::c_void;

use crate::khata::NatijatHaqn;

/// The NT headers of a module of this process's width.
#[cfg(all(windows, target_pointer_width = "32"))]
use windows::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS32 as RaasNt;
/// The NT headers of a module of this process's width.
#[cfg(all(windows, target_pointer_width = "64"))]
use windows::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64 as RaasNt;

/// The bit that marks an import as by-ordinal rather than by-name.
#[cfg(all(windows, target_pointer_width = "32"))]
use windows::Win32::System::SystemServices::IMAGE_ORDINAL_FLAG32 as ALAM_TARTEEBI;
/// The bit that marks an import as by-ordinal rather than by-name, at this
/// process's thunk width.
#[cfg(all(windows, target_pointer_width = "64"))]
use windows::Win32::System::SystemServices::IMAGE_ORDINAL_FLAG64 as ALAM_TARTEEBI;

/// One entry of an import thunk array, at this process's width.
#[cfg(all(windows, target_pointer_width = "32"))]
use windows::Win32::System::WindowsProgramming::IMAGE_THUNK_DATA32 as Thunk;
/// One entry of an import thunk array, at this process's width.
///
/// The thunk is the one import-table type the bindings file under
/// `WindowsProgramming` rather than beside the descriptor it belongs to.
#[cfg(all(windows, target_pointer_width = "64"))]
use windows::Win32::System::WindowsProgramming::IMAGE_THUNK_DATA64 as Thunk;

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
    use windows::Win32::System::Diagnostics::Debug::IMAGE_DIRECTORY_ENTRY_IMPORT;
    use windows::Win32::System::SystemServices::{
        IMAGE_DOS_HEADER, IMAGE_DOS_SIGNATURE, IMAGE_IMPORT_BY_NAME, IMAGE_IMPORT_DESCRIPTOR,
        IMAGE_NT_SIGNATURE,
    };

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

    // `e_lfanew` is `i32` and is documented non-negative; a negative one would
    // point before the image, which is a header this reader refuses rather than
    // an offset it applies.
    let Ok(izahat_nt) = usize::try_from(dos.e_lfanew) else {
        return Err(raas_ghayr(
            "the DOS header's offset to the NT headers is negative",
        ));
    };
    // SAFETY: `e_lfanew` is the header's own offset to the NT headers, inside
    // the same mapped image.
    let nt = unsafe { &*qaida.byte_add(izahat_nt).cast::<RaasNt>() };
    if nt.Signature != IMAGE_NT_SIGNATURE {
        return Err(raas_ghayr("the NT headers carry no PE signature"));
    }

    let Some(dalil) = nt
        .OptionalHeader
        .DataDirectory
        .get(IMAGE_DIRECTORY_ENTRY_IMPORT.0 as usize)
        .copied()
    else {
        return Err(raas_ghayr(
            "the optional header has no import entry in its data directory",
        ));
    };
    if dalil.VirtualAddress == 0 || dalil.Size == 0 {
        return Err(raas_ghayr("the module has no import directory"));
    }

    // SAFETY: the import directory's RVA is inside the mapped image by the
    // header's own account, and a loaded module is mapped at its full size.
    let mut wasf =
        unsafe { min_rva(qaida, dalil.VirtualAddress) }?.cast::<IMAGE_IMPORT_DESCRIPTOR>();

    loop {
        // SAFETY: the descriptor array is inside the mapped image and is
        // terminated by an all-zero entry, which the check below reads.
        let hali = unsafe { *wasf };
        if hali.Name == 0 {
            break;
        }

        // SAFETY: `Name` is an RVA to a NUL-terminated ASCII library name
        // inside the same image.
        let ism_wahda = unsafe { core::ffi::CStr::from_ptr(min_rva(qaida, hali.Name)?.cast()) };
        let ism_wahda = ism_wahda.to_string_lossy();

        if ism_wahda.eq_ignore_ascii_case(wahda) {
            // The original-first-thunk array holds the names; the first-thunk
            // array holds the addresses the loader wrote. Walking both in step
            // is what lets a name be matched and its address be replaced. A
            // bound module has no original-first-thunk, and then the addresses
            // are the only array there is.
            // SAFETY: both RVAs are inside the mapped image.
            let rva_asma = unsafe { hali.Anonymous.OriginalFirstThunk };
            let rva_asma = if rva_asma == 0 {
                hali.FirstThunk
            } else {
                rva_asma
            };
            // SAFETY: as above.
            let asma = unsafe { min_rva(qaida, rva_asma) }?.cast::<Thunk>();
            // SAFETY: as above.
            let anawin = unsafe { min_rva(qaida, hali.FirstThunk) }?.cast::<Thunk>();

            let mut fihris: usize = 0;
            loop {
                // SAFETY: both arrays are inside the mapped image and are
                // terminated by an all-zero entry.
                let ism_thunk = unsafe { *asma.add(fihris) };
                // SAFETY: as above.
                let unwan_thunk = unsafe { anawin.add(fihris) };
                // SAFETY: as above.
                if unsafe { ism_thunk.u1.AddressOfData } == 0 {
                    break;
                }
                // An ordinal import carries no name to match against.
                // SAFETY: as above.
                let khaam = unsafe { ism_thunk.u1.Ordinal };
                if khaam & ALAM_TARTEEBI == 0 {
                    // SAFETY: for a named import, `AddressOfData` is an RVA to
                    // an IMAGE_IMPORT_BY_NAME inside the same image.
                    let rva_ism = unsafe { ism_thunk.u1.AddressOfData };
                    // SAFETY: as above.
                    let bism = unsafe { min_rva(qaida, rva_ism) }?.cast::<IMAGE_IMPORT_BY_NAME>();
                    // SAFETY: `Name` is the first byte of a NUL-terminated
                    // ASCII string inside the image.
                    let ism_daala =
                        unsafe { core::ffi::CStr::from_ptr((&raw const (*bism).Name).cast()) };
                    if ism_daala.to_bytes() == ism.as_bytes() {
                        let khana = unwan_thunk.cast::<*mut c_void>();
                        // SAFETY: the thunk is inside the mapped image and is
                        // pointer-aligned.
                        let asli = unsafe { khana.read_volatile() };
                        let kamil = format!("{wahda}!{ism}");
                        // SAFETY: as above.
                        unsafe { iktub_muashir(khana, badil, asli, &kamil) }?;
                        return Ok(IstiradMakhtuf {
                            khana,
                            asli,
                            badil,
                            ism: kamil,
                        });
                    }
                }
                fihris = fihris.saturating_add(1);
            }
        }

        // SAFETY: descriptors are a contiguous array inside the image.
        wasf = unsafe { wasf.add(1) };
    }

    Err(KhataHaqn::IstiradMafqud {
        wahda: wahda.to_owned(),
        ism: ism.to_owned(),
        qaida: qaida as usize as u64,
    })
}

/// One relative virtual address turned into an address inside the mapped image.
///
/// The conversion is checked rather than cast. An RVA is what the image says
/// about itself, and on a 32-bit build a thunk's `AddressOfData` is a `u32`
/// that a corrupt or hostile header can set to anything; `as isize` would turn
/// the top half of that range into a *negative* offset and walk backwards out
/// of the image. `try_from` makes that a named refusal instead.
///
/// # Errors
///
/// [`crate::KhataHaqn::RaasGhayrMafhum`] when the address does not fit this
/// target's pointer width.
///
/// # Safety
///
/// `qaida` must be the base of a module mapped in this process, and the RVA must
/// be one the module's own headers named — the result is only in bounds because
/// the image is mapped at its full virtual size.
#[cfg(windows)]
unsafe fn min_rva<T: Into<u64>>(qaida: *mut c_void, rva: T) -> NatijatHaqn<*mut c_void> {
    let rva: u64 = rva.into();
    let Ok(izaha) = usize::try_from(rva) else {
        return Err(crate::khata::KhataHaqn::RaasGhayrMafhum {
            qaida: qaida as usize as u64,
            sabab: format!("a relative address of {rva} does not fit this process's pointers"),
        });
    };
    // SAFETY: the caller guarantees `qaida` is a mapped module base and `izaha`
    // an offset the module's own headers named, so the sum is inside the image.
    Ok(unsafe { qaida.byte_add(izaha) })
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
    Err(crate::khata::KhataHaqn::GhayrMutahaHuna {
        amal: "import table redirection",
    })
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

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use core::ffi::c_void;

    use super::ikhtif_istirad;
    use crate::khata::KhataHaqn;

    /// A symbol name nothing exports and therefore nothing imports.
    #[cfg(windows)]
    const RAMZ_MUSTAHIL: &str = "TaaribLaYujadHadhaAlRamzAbadan";

    /// Walking a real mapped module's import table to the end, and writing
    /// nothing.
    ///
    /// The assertion looks weak — "the symbol was not found" — and is not. To
    /// reach that answer the reader has to accept the DOS header, find the NT
    /// headers, read the import directory out of the **optional header at this
    /// target's width**, walk every descriptor, match a library name, and walk
    /// that library's whole thunk array to its terminator. A PE32 image read
    /// through PE32+ structures does not fail any of the early checks: it reads
    /// the directory from the wrong offset and produces `RaasGhayrMafhum`, a
    /// wild pointer, or a fault. So this test is the width check, and it is why
    /// it is worth running on both `x86_64-pc-windows-msvc` and
    /// `i686-pc-windows-msvc`.
    ///
    /// `kernel32.dll` is the module under test because every Windows process
    /// has it mapped and it genuinely imports from `ntdll.dll`, so the inner
    /// walk runs rather than being skipped for want of a matching library.
    #[cfg(windows)]
    #[test]
    fn al_mashy_ala_jadwal_istirad_haqiqi_yasil_ila_nihayatih() {
        let Some(qaida) = crate::mawqi::qaidat_wahda("kernel32.dll") else {
            panic!("every Windows process has kernel32 mapped");
        };
        // SAFETY: `qaida` is the base of a module this process has mapped, and
        // the walk returns before any write because no import carries this
        // name — `badil` is never installed anywhere.
        let natija = unsafe {
            ikhtif_istirad(
                qaida,
                "ntdll.dll",
                RAMZ_MUSTAHIL,
                core::ptr::null_mut::<c_void>(),
            )
        };
        match natija {
            Err(KhataHaqn::IstiradMafqud { wahda, ism, .. }) => {
                assert_eq!(wahda, "ntdll.dll");
                assert_eq!(ism, RAMZ_MUSTAHIL);
            },
            Err(akhar) => panic!(
                "the reader did not reach the end of a real import table, which is what a \
                 wrong-width parse looks like: {akhar:?}"
            ),
            Ok(_) => panic!("no module imports {RAMZ_MUSTAHIL}"),
        }
    }

    /// A library the module does not import at all, which must be the same
    /// refusal rather than a header complaint.
    #[cfg(windows)]
    #[test]
    fn wahda_ghayr_mustawrada_hiya_nafs_alrafd() {
        let Some(qaida) = crate::mawqi::qaidat_wahda("kernel32.dll") else {
            panic!("every Windows process has kernel32 mapped");
        };
        // SAFETY: as above; the walk writes nothing on this path.
        let natija = unsafe {
            ikhtif_istirad(
                qaida,
                "taarib-la-tujad-hadhihi-al-wahda.dll",
                "AnySymbol",
                core::ptr::null_mut::<c_void>(),
            )
        };
        assert!(
            matches!(natija, Err(KhataHaqn::IstiradMafqud { .. })),
            "a library that is not imported is a missing import, not an unreadable header"
        );
    }

    /// Off Windows there is no import table to patch, and the refusal says so
    /// rather than pretending to have looked.
    #[cfg(not(windows))]
    #[test]
    fn kharij_windows_yurfad_bil_ism() {
        // SAFETY: the non-Windows arm dereferences nothing; the signature
        // matches the Windows one so callers need no `cfg`.
        let natija = unsafe {
            ikhtif_istirad(
                core::ptr::null_mut::<c_void>(),
                "libc.so.6",
                "getenv",
                core::ptr::null_mut::<c_void>(),
            )
        };
        match natija {
            Err(KhataHaqn::GhayrMutahaHuna { amal }) => {
                assert_eq!(amal, "import table redirection");
            },
            akhar => panic!("expected GhayrMutahaHuna, got {akhar:?}"),
        }
    }
}
