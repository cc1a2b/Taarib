//! The Windows proxy: forward every `version.dll` export, and load the payload once.

use core::ffi::c_void;
use std::os::windows::ffi::{OsStrExt as _, OsStringExt as _};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use windows::Win32::Foundation::{CloseHandle, FARPROC, HMODULE, MAX_PATH};
use windows::Win32::System::LibraryLoader::{
    DisableThreadLibraryCalls, GetModuleFileNameW, GetProcAddress, LoadLibraryW,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{CreateThread, THREAD_CREATION_FLAGS};
use windows::core::{PCSTR, PCWSTR};

use crate::hamula;

/// `DLL_PROCESS_ATTACH`, the only reason code this loader acts on.
const SABAB_IRTIBAT: u32 = 1;

/// This module's own handle, stashed at attach so the payload thread can find
/// the directory it was loaded from.
static MIQBAD_NAFSI: AtomicUsize = AtomicUsize::new(0);

/// The real `version.dll`, resolved from the system directory on first use.
static MIQBAD_ASLI: AtomicUsize = AtomicUsize::new(0);

/// The path a module handle was loaded from.
fn masar_wahda(miqbad: HMODULE) -> Option<PathBuf> {
    let mut hajiz = [0u16; MAX_PATH as usize];
    // SAFETY: `hajiz` is a live buffer of exactly the length passed.
    let tul = unsafe { GetModuleFileNameW(Some(miqbad), &mut hajiz) } as usize;
    // Zero is the API's failure return and a length equal to the buffer means
    // the name was truncated; neither is a path. `get` alone would accept both,
    // because `get(..0)` is an empty slice and `get(..len)` is the whole buffer
    // — so the guard stays and `get` is only how the bound is expressed.
    // `indexing_slicing` is denied workspace-wide and has no flow analysis, and
    // this crate is `#[cfg(windows)]`, so a Linux clippy run never reaches here.
    if tul == 0 || tul >= hajiz.len() {
        return None;
    }
    let ism = hajiz.get(..tul)?;
    Some(PathBuf::from(std::ffi::OsString::from_wide(ism)))
}

/// The real `version.dll`, loaded by absolute path.
///
/// By absolute path and never by name: this module *is* `version.dll` as far as
/// the game's directory is concerned, so a name lookup would resolve back here
/// and every forwarded call would recurse until the stack ended.
fn wahda_asliya() -> HMODULE {
    let mahfuz = MIQBAD_ASLI.load(Ordering::Acquire);
    if mahfuz != 0 {
        return HMODULE(mahfuz as *mut c_void);
    }

    let mut hajiz = [0u16; MAX_PATH as usize];
    // SAFETY: `hajiz` is a live buffer of exactly the length passed.
    let tul = unsafe { GetSystemDirectoryW(Some(&mut hajiz)) } as usize;
    // Same two conditions as `masar_wahda`, for the same reason.
    if tul == 0 || tul >= hajiz.len() {
        return HMODULE::default();
    }
    let Some(mujallad) = hajiz.get(..tul) else {
        return HMODULE::default();
    };
    let mut masar: Vec<u16> = mujallad.to_vec();
    masar.extend("\\version.dll\0".encode_utf16());

    // SAFETY: `masar` is a NUL-terminated wide string that outlives the call.
    let Ok(miqbad) = (unsafe { LoadLibraryW(PCWSTR(masar.as_ptr())) }) else {
        return HMODULE::default();
    };
    MIQBAD_ASLI.store(miqbad.0 as usize, Ordering::Release);
    miqbad
}

/// One export of the real `version.dll`, resolved once and cached.
fn hall(marji: &AtomicUsize, ism: &[u8]) -> FARPROC {
    let mahfuz = marji.load(Ordering::Acquire);
    if mahfuz != 0 {
        type Khaam = unsafe extern "system" fn() -> isize;
        // SAFETY: the stored value came from `GetProcAddress` in this process
        // and the module it belongs to is never freed.
        return Some(unsafe { core::mem::transmute::<usize, Khaam>(mahfuz) });
    }
    let wahda = wahda_asliya();
    if wahda.is_invalid() {
        return None;
    }
    // SAFETY: `ism` is a NUL-terminated ASCII literal.
    let daala = unsafe { GetProcAddress(wahda, PCSTR(ism.as_ptr())) };
    if let Some(mawjuda) = daala {
        marji.store(mawjuda as usize, Ordering::Release);
    }
    daala
}

/// Declares one forwarding stub per export, resolved lazily and called through.
///
/// A stub whose target cannot be resolved returns the failure value the real
/// function documents for that case rather than calling a null pointer.
macro_rules! wakeel {
    ($($ism:ident ( $($muaamil:ident : $naw:ty),* $(,)? ) -> $radd:ty = $ikhfaq:expr;)*) => {
        $(
            // `unreachable_pub` is right about Rust and wrong about this crate.
            // Nothing in the module tree reaches these, because their caller is
            // the Windows loader: a game imports `version.dll`, this cdylib is
            // what answers, and each stub is one of the exports the real
            // library publishes. Narrowing them to `pub(crate)` would satisfy
            // the lint by making the proxy export nothing — a `version.dll`
            // that resolves no import is a game that does not start.
            #[expect(unreachable_pub, reason = "an export of this cdylib, reached by the loader")]
            #[unsafe(no_mangle)]
            pub unsafe extern "system" fn $ism($($muaamil: $naw),*) -> $radd {
                type Daala = unsafe extern "system" fn($($naw),*) -> $radd;
                static MARJI: AtomicUsize = AtomicUsize::new(0);
                let Some(hadaf) = hall(&MARJI, concat!(stringify!($ism), "\0").as_bytes())
                else {
                    return $ikhfaq;
                };
                // SAFETY: the resolved export carries exactly this signature,
                // which is the documented one for the function of this name.
                let hadaf: Daala = unsafe { core::mem::transmute(hadaf) };
                // SAFETY: every argument is passed through untouched to the
                // function the caller believed it was calling.
                unsafe { hadaf($($muaamil),*) }
            }
        )*
    };
}

wakeel! {
    GetFileVersionInfoA(ism: *const u8, miqbad: u32, tul: u32, bayanat: *mut c_void) -> i32 = 0;
    GetFileVersionInfoW(ism: *const u16, miqbad: u32, tul: u32, bayanat: *mut c_void) -> i32 = 0;
    GetFileVersionInfoExA(
        alam: u32, ism: *const u8, miqbad: u32, tul: u32, bayanat: *mut c_void,
    ) -> i32 = 0;
    GetFileVersionInfoExW(
        alam: u32, ism: *const u16, miqbad: u32, tul: u32, bayanat: *mut c_void,
    ) -> i32 = 0;
    GetFileVersionInfoSizeA(ism: *const u8, miqbad: *mut u32) -> u32 = 0;
    GetFileVersionInfoSizeW(ism: *const u16, miqbad: *mut u32) -> u32 = 0;
    GetFileVersionInfoSizeExA(alam: u32, ism: *const u8, miqbad: *mut u32) -> u32 = 0;
    GetFileVersionInfoSizeExW(alam: u32, ism: *const u16, miqbad: *mut u32) -> u32 = 0;
    VerQueryValueA(
        kutla: *const c_void, kutla_farya: *const u8, hajiz: *mut *mut c_void, tul: *mut u32,
    ) -> i32 = 0;
    VerQueryValueW(
        kutla: *const c_void, kutla_farya: *const u16, hajiz: *mut *mut c_void, tul: *mut u32,
    ) -> i32 = 0;
    VerFindFileA(
        alam: u32, ism: *const u8, majallad_windows: *const u8, majallad_tatbiq: *const u8,
        majallad_hali: *mut u8, tul_hali: *mut u32, majallad_hadaf: *mut u8, tul_hadaf: *mut u32,
    ) -> u32 = 0;
    VerFindFileW(
        alam: u32, ism: *const u16, majallad_windows: *const u16, majallad_tatbiq: *const u16,
        majallad_hali: *mut u16, tul_hali: *mut u32, majallad_hadaf: *mut u16,
        tul_hadaf: *mut u32,
    ) -> u32 = 0;
    VerInstallFileA(
        alam: u32, ism_masdar: *const u8, ism_hadaf: *const u8, majallad_masdar: *const u8,
        majallad_hadaf: *const u8, majallad_hali: *const u8, malaf_muaqqat: *mut u8,
        tul_muaqqat: *mut u32,
    ) -> u32 = 0;
    VerInstallFileW(
        alam: u32, ism_masdar: *const u16, ism_hadaf: *const u16, majallad_masdar: *const u16,
        majallad_hadaf: *const u16, majallad_hali: *const u16, malaf_muaqqat: *mut u16,
        tul_muaqqat: *mut u32,
    ) -> u32 = 0;
    VerLanguageNameA(lugha: u32, hajiz: *mut u8, tul: u32) -> u32 = 0;
    VerLanguageNameW(lugha: u32, hajiz: *mut u16, tul: u32) -> u32 = 0;
}

/// Loads the payload modules beside this one. Runs on its own thread.
///
/// # Safety
///
/// Takes the shape `CreateThread` requires; the argument is unused.
unsafe extern "system" fn khayt_hamula(_muaamil: *mut c_void) -> u32 {
    let miqbad = HMODULE(MIQBAD_NAFSI.load(Ordering::Acquire) as *mut c_void);
    let Some(jidhr) = masar_wahda(miqbad).and_then(|masar| masar.parent().map(PathBuf::from))
    else {
        return 0;
    };

    for masar in hamula::hamulat_mawjuda(&jidhr) {
        let mut wasi: Vec<u16> = masar.as_os_str().encode_wide().collect();
        wasi.push(0);
        // SAFETY: `wasi` is a NUL-terminated wide path that outlives the call.
        let Ok(wahda) = (unsafe { LoadLibraryW(PCWSTR(wasi.as_ptr())) }) else {
            hamula::sajjil(&jidhr, &format!("taarib: تعذّر تحميل {}", masar.display()));
            continue;
        };
        // SAFETY: `ISM_BIDAYA` is a NUL-terminated ASCII literal.
        if let Some(bidaya) = unsafe { GetProcAddress(wahda, PCSTR(hamula::ISM_BIDAYA.as_ptr())) } {
            // SAFETY: every payload declares this entry as a no-argument
            // `extern "system"` function; a module exporting the name with
            // another shape is a build error in that payload, not here.
            let bidaya: extern "system" fn() = unsafe { core::mem::transmute(bidaya) };
            bidaya();
        }
        hamula::sajjil(&jidhr, &format!("taarib: حُمِّل {}", masar.display()));
    }
    0
}

/// The module entry point.
///
/// Nothing is loaded here. `LoadLibrary` inside `DllMain` runs under the
/// loader lock and deadlocks against any module that takes it on another
/// thread, so attach only records the handle and starts the thread that does
/// the real work once the lock is released.
// The loader calls this by name before anything in the module tree exists; see
// the note on the export stubs above.
#[expect(unreachable_pub, reason = "the loader's entry point into this cdylib")]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(miqbad: HMODULE, sabab: u32, _mahfuz: *mut c_void) -> i32 {
    if sabab == SABAB_IRTIBAT {
        MIQBAD_NAFSI.store(miqbad.0 as usize, Ordering::Release);
        // Thread attach and detach notifications cost a lock on every thread a
        // game creates, and this module has nothing to do on either.
        // SAFETY: `miqbad` is this module's handle, valid for the call.
        let _ = unsafe { DisableThreadLibraryCalls(miqbad) };
        // SAFETY: the thread function is a `'static` item and the argument is
        // null; the handle is dropped because nothing here joins the thread.
        if let Ok(khayt) = unsafe {
            CreateThread(
                None,
                0,
                Some(khayt_hamula),
                None,
                THREAD_CREATION_FLAGS(0),
                None,
            )
        } {
            // SAFETY: a handle returned by CreateThread, closed exactly once.
            let _ = unsafe { CloseHandle(khayt) };
        }
    }
    1
}
