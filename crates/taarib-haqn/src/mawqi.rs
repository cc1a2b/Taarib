//! Where this module is, and where the modules around it are.

use core::ffi::c_void;
use std::path::PathBuf;

/// The directory the calling module was loaded from.
///
/// Resolved from an address inside the module rather than from the process's
/// arguments or its working directory: a payload is loaded into somebody else's
/// game, whose working directory is the launcher's and whose executable is not
/// Taarib's. The only thing a payload knows for certain is where its own bytes
/// came from.
///
/// [`None`] when the platform cannot answer, which a caller treats as "there is
/// nothing beside me" rather than as a failure to report.
#[must_use]
pub fn mujallad_nafsi() -> Option<PathBuf> {
    masar_nafsi().and_then(|masar| masar.parent().map(PathBuf::from))
}

/// The full path of the calling module.
#[must_use]
#[cfg(windows)]
pub fn masar_nafsi() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt as _;
    use windows::Win32::Foundation::{HMODULE, MAX_PATH};
    use windows::Win32::System::LibraryLoader::{
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        GetModuleFileNameW, GetModuleHandleExW,
    };
    use windows::core::PCWSTR;

    let mut miqbad = HMODULE::default();
    // SAFETY: the address is a function in this module, and the flags ask for
    // the module containing it without taking a reference — this call must not
    // change the module's lifetime, only report it.
    let natija = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR((masar_nafsi as *const ()).cast::<u16>()),
            &raw mut miqbad,
        )
    };
    if natija.is_err() {
        return None;
    }

    let mut hajiz = [0u16; MAX_PATH as usize];
    // SAFETY: `hajiz` is a live buffer of exactly the length passed.
    let tul = unsafe { GetModuleFileNameW(Some(miqbad), &mut hajiz) } as usize;
    // Zero is the API's failure return and a length equal to the buffer means
    // the name was truncated; neither is a path. The guard answers both, and
    // `get` is only how the bound is expressed — `indexing_slicing` is denied
    // workspace-wide and has no flow analysis to see that the guard already
    // holds.
    if tul == 0 || tul >= hajiz.len() {
        return None;
    }
    Some(PathBuf::from(std::ffi::OsString::from_wide(
        hajiz.get(..tul)?,
    )))
}

/// The full path of the calling module.
#[must_use]
#[cfg(not(windows))]
pub fn masar_nafsi() -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt as _;

    #[repr(C)]
    struct MaalumatWahda {
        masar: *const core::ffi::c_char,
        qaida: *mut c_void,
        ism_ramz: *const core::ffi::c_char,
        unwan_ramz: *mut c_void,
    }
    unsafe extern "C" {
        fn dladdr(unwan: *const c_void, maalumat: *mut MaalumatWahda) -> core::ffi::c_int;
    }

    let mut maalumat = MaalumatWahda {
        masar: core::ptr::null(),
        qaida: core::ptr::null_mut(),
        ism_ramz: core::ptr::null(),
        unwan_ramz: core::ptr::null_mut(),
    };
    // SAFETY: the address is a function in this module and `maalumat` is a live
    // out-parameter of exactly the layout `dladdr` writes.
    let natija = unsafe { dladdr(masar_nafsi as *const c_void, &raw mut maalumat) };
    if natija == 0 || maalumat.masar.is_null() {
        return None;
    }
    // SAFETY: on success `dladdr` leaves a NUL-terminated path owned by the
    // dynamic loader and valid for the life of the process.
    let masar = unsafe { std::ffi::CStr::from_ptr(maalumat.masar) };
    Some(PathBuf::from(std::ffi::OsStr::from_bytes(masar.to_bytes())))
}

/// The base address of a loaded module, by name, or [`None`] when the process
/// has not loaded it.
///
/// The name is matched as the platform's loader spells it: `UnityPlayer.dll` on
/// Windows, `libGL.so.1` on Linux. A module that is not loaded is not an error
/// — it is the ordinary answer for "is this game a Unity game", and every
/// caller here treats it as a fact rather than a failure.
#[must_use]
#[cfg(windows)]
pub fn qaidat_wahda(ism: &str) -> Option<*mut c_void> {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::PCWSTR;

    let mut wasi: Vec<u16> = ism.encode_utf16().collect();
    wasi.push(0);
    // SAFETY: `wasi` is a NUL-terminated wide string that outlives the call.
    // `GetModuleHandleW` takes no reference, so the handle must not be freed.
    let miqbad = unsafe { GetModuleHandleW(PCWSTR(wasi.as_ptr())) }.ok()?;
    if miqbad.is_invalid() {
        None
    } else {
        Some(miqbad.0.cast::<c_void>())
    }
}

/// The base address of a loaded module, by name.
#[must_use]
#[cfg(not(windows))]
pub fn qaidat_wahda(ism: &str) -> Option<*mut c_void> {
    let mut bayt: Vec<u8> = ism.as_bytes().to_vec();
    bayt.push(0);
    // SAFETY: `bayt` is a NUL-terminated name that outlives the call.
    // `RTLD_NOLOAD` asks only whether the module is already loaded, so this
    // never brings one in as a side effect of asking.
    let miqbad = unsafe {
        libc::dlopen(
            bayt.as_ptr().cast::<core::ffi::c_char>(),
            libc::RTLD_NOW | libc::RTLD_NOLOAD,
        )
    };
    if miqbad.is_null() {
        return None;
    }
    // The handle is closed immediately: `RTLD_NOLOAD` still takes a reference,
    // and a payload that leaked one per probe would pin the game's modules.
    // SAFETY: `miqbad` came from `dlopen` on the line above.
    let _ = unsafe { libc::dlclose(miqbad) };
    Some(miqbad.cast::<c_void>())
}

/// The address of an exported symbol in a loaded module.
///
/// # Safety
///
/// `qaida` must be the base address of a module currently mapped in this
/// process, as [`qaidat_wahda`] returns.
#[must_use]
#[cfg(windows)]
pub unsafe fn ramz_wahda(qaida: *mut c_void, ism: &str) -> Option<*mut c_void> {
    use windows::Win32::Foundation::HMODULE;
    use windows::Win32::System::LibraryLoader::GetProcAddress;
    use windows::core::PCSTR;

    let mut bayt: Vec<u8> = ism.as_bytes().to_vec();
    bayt.push(0);
    // SAFETY: the caller guarantees `qaida` is a mapped module base, and `bayt`
    // is a NUL-terminated name that outlives the call.
    let daala = unsafe { GetProcAddress(HMODULE(qaida), PCSTR(bayt.as_ptr())) }?;
    Some(daala as *mut c_void)
}

/// The address of an exported symbol in a loaded module.
///
/// # Safety
///
/// `qaida` must be the base address of a module currently mapped in this
/// process.
#[must_use]
#[cfg(not(windows))]
pub unsafe fn ramz_wahda(qaida: *mut c_void, ism: &str) -> Option<*mut c_void> {
    let mut bayt: Vec<u8> = ism.as_bytes().to_vec();
    bayt.push(0);
    // SAFETY: the caller guarantees `qaida` is a live handle from `dlopen`, and
    // `bayt` is a NUL-terminated name that outlives the call.
    let unwan = unsafe { libc::dlsym(qaida, bayt.as_ptr().cast::<core::ffi::c_char>()) };
    if unwan.is_null() {
        None
    } else {
        Some(unwan.cast::<c_void>())
    }
}
