//! The Linux and macOS preload library: a constructor that brings the payload in.

use core::ffi::{c_char, c_int, c_void};
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt as _;
use std::path::PathBuf;

use crate::hamula;

/// `dladdr`'s answer, of which only the module path is read.
#[repr(C)]
struct MaalumatWahda {
    masar: *const c_char,
    qaida: *mut c_void,
    ism_ramz: *const c_char,
    unwan_ramz: *mut c_void,
}

unsafe extern "C" {
    fn dladdr(unwan: *const c_void, maalumat: *mut MaalumatWahda) -> c_int;
}

/// The directory this preload library itself was loaded from.
///
/// `dladdr` on an address inside this module rather than anything from the
/// environment: `LD_PRELOAD` may name a relative path, and the process's
/// working directory at constructor time is the launcher's, not the game's.
fn jidhr_nafsi() -> Option<PathBuf> {
    let mut maalumat = MaalumatWahda {
        masar: core::ptr::null(),
        qaida: core::ptr::null_mut(),
        ism_ramz: core::ptr::null(),
        unwan_ramz: core::ptr::null_mut(),
    };
    // SAFETY: the address is a function in this module and `maalumat` is a live
    // out-parameter of exactly the layout `dladdr` writes.
    let natija = unsafe { dladdr(jidhr_nafsi as *const c_void, &raw mut maalumat) };
    if natija == 0 || maalumat.masar.is_null() {
        return None;
    }
    // SAFETY: on success `dladdr` leaves a NUL-terminated path owned by the
    // dynamic loader, valid for the lifetime of the process.
    let masar = unsafe { CStr::from_ptr(maalumat.masar) };
    let masar = PathBuf::from(std::ffi::OsStr::from_bytes(masar.to_bytes()));
    masar.parent().map(PathBuf::from)
}

/// Loads every payload present beside this library and calls its entry.
fn hammil() {
    let Some(jidhr) = jidhr_nafsi() else {
        return;
    };

    for masar in hamula::hamulat_mawjuda(&jidhr) {
        let mut bayt = masar.as_os_str().as_bytes().to_vec();
        bayt.push(0);
        // SAFETY: `bayt` is a NUL-terminated path that outlives the call.
        // RTLD_NOW so a payload with an unresolved symbol fails here, named,
        // rather than at an arbitrary later frame inside the game.
        let wahda = unsafe { libc::dlopen(bayt.as_ptr().cast::<c_char>(), libc::RTLD_NOW) };
        if wahda.is_null() {
            hamula::sajjil(&jidhr, &format!("taarib: تعذّر تحميل {}", masar.display()));
            continue;
        }
        // SAFETY: `ISM_BIDAYA` is a NUL-terminated ASCII literal and `wahda` is
        // a handle `dlopen` just returned.
        let bidaya =
            unsafe { libc::dlsym(wahda, hamula::ISM_BIDAYA.as_ptr().cast::<c_char>()) };
        if !bidaya.is_null() {
            // SAFETY: every payload declares this entry as a no-argument
            // `extern "C"` function; a module exporting the name with another
            // shape is a build error in that payload, not here.
            let bidaya: extern "C" fn() = unsafe { core::mem::transmute(bidaya) };
            bidaya();
        }
        hamula::sajjil(&jidhr, &format!("taarib: حُمِّل {}", masar.display()));
    }
}

/// The constructor the dynamic loader runs when this library is preloaded.
///
/// Placed in the platform's init-array so it runs before the game's `main`,
/// which is the whole point of a preload: an adapter that installed itself
/// after the engine had already built its text pipeline would be too late.
#[used]
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
#[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
static MUNSHI: extern "C" fn() = bidayat_wahda;

extern "C" fn bidayat_wahda() {
    hammil();
}
