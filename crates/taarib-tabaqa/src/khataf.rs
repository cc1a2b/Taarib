//! الخطّاف — where the overlay gets onto the presentation path, and how it gets
//! off again.
//!
//! Every unsafe operation this crate performs against a live process is in this
//! module. The four backends draw; none of them installs itself, reads a method
//! table, or changes page protection. Keeping it that way is not tidiness: it
//! means the question "what does the overlay do to somebody's game?" has one
//! file as its answer, and a reviewer can read that file in one sitting.
//!
//! ## Two mechanisms, and the preference between them
//!
//! **A layer** is the right way in, and Vulkan is the only API that offers one.
//! The loader enumerates it, the game's own initialization brings it up, and
//! removing it is deleting a JSON manifest. Nothing writes into the process and
//! nothing resembles a debugger attaching. [`hal_yumkin`] reports it first for
//! that reason.
//!
//! **A vtable write** is how the other three are reached, and it is worth being
//! precise about what that means rather than calling it hooking and moving on.
//! A COM interface's first machine word points at an array of function
//! pointers, shared by every instance of that class in the process. Replacing
//! one entry redirects every call to that method, from every caller, including
//! the game's. The overlay reads the original out, writes its own in, and
//! *calls the original* — so the game's frame still presents, exactly as it
//! would have.
//!
//! This is why the original is captured before the write rather than looked up
//! afterwards: a second hook installed by another overlay between the two would
//! make the "original" this module calls into that other overlay's thunk, and
//! two overlays each calling what they believe is the original is an infinite
//! recursion that ends in a stack overflow inside somebody's game.
//!
//! ## Getting a vtable without a game
//!
//! There is no API for "give me the vtable of the swap chain this process is
//! using". The documented way to find one is to make your own: create a device
//! and a swap chain against a throwaway window, read the pointers out of it, and
//! destroy it. Those pointers are the same ones the game's swap chain uses,
//! because a vtable belongs to the class and not to the instance.
//!
//! The throwaway window is never shown. [`nafidha_muaqqata`] registers a class,
//! creates a hidden window, and destroys both — and it does the destroying in a
//! [`Drop`] impl so that an early return from a failed device creation does not
//! leak a window class into the game's process for the rest of its life.
//!
//! ## Unhooking is not optional
//!
//! [`Khataf::fukk`] restores every entry it changed and verifies the restore by
//! reading it back. A hook that cannot be removed pins this module in the
//! process forever, and the user is told that plainly through
//! [`KhataTabaqa::FakkKhatfFashil`] rather than being left with a "disable"
//! button that quietly does nothing.
//!
//! The verification matters more than it looks. If another overlay hooked the
//! same slot *after* Taarib did, writing Taarib's saved original back would
//! erase that overlay's hook and leave it calling into freed memory. So the
//! restore is conditional: the slot is only written back if it still holds
//! Taarib's thunk, and a slot that holds something else is left alone with the
//! situation reported.

use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::khata::KhataTabaqa;
use crate::wajiha::WajihatRusum;

/// The vtable slot of `IDXGISwapChain::Present`.
///
/// `IUnknown` contributes three (`QueryInterface`, `AddRef`, `Release`),
/// `IDXGIObject` four (`SetPrivateData`, `SetPrivateDataInterface`,
/// `GetPrivateData`, `GetParent`), `IDXGIDeviceSubObject` one (`GetDevice`), and
/// `Present` is the first of `IDXGISwapChain`'s own. Eight.
///
/// Written as a derivation rather than as the number 8, because the number 8
/// is unfalsifiable in review and the derivation is checkable against the
/// headers by anybody who doubts it.
pub const KHANAT_PRESENT: usize = 3 + 4 + 1;

/// The vtable slot of `IDXGISwapChain1::Present1`.
///
/// `IDXGISwapChain` adds nine methods after `Present` — `GetBuffer`,
/// `SetFullscreenState`, `GetFullscreenState`, `GetDesc`, `ResizeBuffers`,
/// `ResizeTarget`, `GetContainingOutput`, `GetFrameStatistics`,
/// `GetLastPresentCount` — and `IDXGISwapChain1` then adds `GetDesc1`,
/// `GetFullscreenDesc`, `GetHwnd`, `GetCoreWindow` before `Present1`.
pub const KHANAT_PRESENT1: usize = KHANAT_PRESENT + 1 + 9 + 4;

/// The vtable slot of `IDXGISwapChain::ResizeBuffers`.
///
/// Hooked alongside `Present` because a resolution change or a fullscreen
/// transition invalidates every overlay resource sized against the backbuffer,
/// and the only moment those resources can be released *before* the swap chain
/// releases its own is inside this call, before it reaches the runtime.
/// Discovering the change on the next `Present` is one frame too late: the
/// runtime has already refused the resize because the overlay still holds a
/// reference to a backbuffer.
///
/// One past `Present`, then `GetBuffer`, `SetFullscreenState`,
/// `GetFullscreenState` and `GetDesc` — thirteen.
pub const KHANAT_RESIZE: usize = KHANAT_PRESENT + 1 + 4;

/// The vtable slot of `ID3D12CommandQueue::ExecuteCommandLists`.
///
/// `IUnknown` three, `ID3D12Object` four (`GetPrivateData`, `SetPrivateData`,
/// `SetPrivateDataInterface`, `SetName`), `ID3D12DeviceChild` one
/// (`GetDevice`), `ID3D12Pageable` none, and `ExecuteCommandLists` is
/// `ID3D12CommandQueue`'s own first method.
///
/// Hooked for one reason, and it is worth stating because it looks like an
/// overreach otherwise: **D3D12 has no way to get the command queue from a swap
/// chain.** `IDXGISwapChain3` does not expose it, and the overlay cannot submit
/// its own command list without one. Capturing the first queue the game submits
/// on is the documented technique and the only one available.
pub const KHANAT_TANFEEDH: usize = 3 + 4 + 1;

/// One replaced vtable entry, and everything needed to put it back.
#[derive(Debug)]
struct KhanaMakhtufa {
    /// The address of the slot itself — the pointer-sized cell inside the table.
    khana: *mut *mut c_void,
    /// What the slot held before the write.
    asli: *mut c_void,
    /// What was written into it.
    badil: *mut c_void,
    /// Which method, for the report.
    ism: &'static str,
}

// SAFETY: `khana` points into a vtable that lives for as long as the module
// owning it is loaded, and `asli`/`badil` are function addresses in loaded
// modules. This type never dereferences any of them and never frees any of
// them; it hands `khana` to `iktub_khana`, which accesses it volatilely and
// checks the value it finds before writing. Moving the record to another thread
// therefore conveys no capability that the vtable does not already give every
// thread in the process, which is why `Send` is sound here and `Sync` is
// deliberately not claimed.
unsafe impl Send for KhanaMakhtufa {}

/// A hidden window and its class, destroyed when this value is dropped.
///
/// Exists purely so that an early return between "created the window" and
/// "created the device" cannot leak a registered window class into the game's
/// process. A leaked class is not fatal, but it is a name in the process's
/// class table forever, and the second attempt to install the overlay would
/// then fail to register it and report something that has nothing to do with
/// the real problem.
#[cfg(windows)]
#[derive(Debug)]
pub struct NafidhaMuaqqata {
    maqbad: windows::Win32::Foundation::HWND,
    sanf: u16,
    wahda: windows::Win32::Foundation::HMODULE,
}

#[cfg(windows)]
impl Drop for NafidhaMuaqqata {
    fn drop(&mut self) {
        use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, UnregisterClassW};
        use windows::core::PCWSTR;

        if !self.maqbad.is_invalid() {
            // SAFETY: `maqbad` was returned by `CreateWindowExW` in
            // `nafidha_muaqqata` and has not been destroyed — this is the only
            // `DestroyWindow` for it, and the field is not reachable afterwards
            // because `self` is being dropped.
            unsafe {
                let _ = DestroyWindow(self.maqbad);
            }
        }
        if self.sanf != 0 {
            // SAFETY: `sanf` is the atom `RegisterClassExW` returned for this
            // module, and the window created from it was destroyed immediately
            // above, which is the precondition `UnregisterClassW` states.
            unsafe {
                // `.into()`: the class was registered against a module handle,
                // and the unregister call names the same module as an
                // `HINSTANCE`. The two are the same value under different
                // names, and the bindings stopped treating them as one type.
                let _ = UnregisterClassW(
                    PCWSTR(self.sanf as usize as *const u16),
                    Some(self.wahda.into()),
                );
            }
        }
    }
}

#[cfg(windows)]
impl NafidhaMuaqqata {
    /// The window handle, for passing to a swap chain description.
    #[must_use]
    pub const fn maqbad(&self) -> windows::Win32::Foundation::HWND {
        self.maqbad
    }
}

/// Creates a hidden window to build a throwaway swap chain against.
///
/// Not shown, not sized, never pumped. It exists because
/// `IDXGIFactory::CreateSwapChain` requires an `HWND` and there is no other way
/// to obtain a swap chain whose vtable can be read.
///
/// # Errors
///
/// [`KhataTabaqa::JadwalGhayrMawjud`] when the class cannot be registered or the
/// window cannot be created — a session with no window station, most often,
/// which is a real deployment and worth naming rather than crashing in.
#[cfg(windows)]
pub fn nafidha_muaqqata() -> Result<NafidhaMuaqqata, KhataTabaqa> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, RegisterClassExW, WINDOW_EX_STYLE, WNDCLASSEXW,
        WS_OVERLAPPED,
    };
    use windows::core::PCWSTR;

    let mut ism: Vec<u16> = "TaaribTabaqaMuaqqata\0".encode_utf16().collect();

    // SAFETY: `GetModuleHandleW(None)` returns the handle of the calling
    // process's own image and cannot fail for the null argument.
    let wahda = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|khata| {
        KhataTabaqa::JadwalGhayrMawjud {
            wajiha: "IDXGISwapChain".to_owned(),
            sabab: format!("the module handle could not be obtained: {khata}"),
        }
    })?;

    // The window procedure has to be a bare `extern "system"` pointer, and the
    // binding for `DefWindowProcW` is a safe-ish Rust wrapper around the real
    // import rather than the import itself, so it cannot be handed over as one.
    // This shim is that pointer: it does nothing but call the default handler,
    // which is the entire behaviour this class wants — the window exists only to
    // give a swap chain something to be created against and never draws.
    unsafe extern "system" fn ijra_iftiradi(
        nafidha: HWND,
        risala: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
    ) -> windows::Win32::Foundation::LRESULT {
        // SAFETY: the arguments are the ones the window manager passed, handed
        // straight back to the handler it would otherwise have reached.
        unsafe { DefWindowProcW(nafidha, risala, wparam, lparam) }
    }

    let sanf_wasf = WNDCLASSEXW {
        cbSize: size_of_u32::<WNDCLASSEXW>(),
        lpfnWndProc: Some(ijra_iftiradi),
        hInstance: wahda.into(),
        lpszClassName: PCWSTR(ism.as_mut_ptr()),
        ..Default::default()
    };

    // SAFETY: `sanf_wasf` is fully initialized above, `lpszClassName` points at
    // `ism`, which is nul-terminated and outlives this call, and `cbSize` is the
    // size of the structure being passed.
    let sanf = unsafe { RegisterClassExW(&raw const sanf_wasf) };
    if sanf == 0 {
        return Err(KhataTabaqa::JadwalGhayrMawjud {
            wajiha: "IDXGISwapChain".to_owned(),
            sabab: "a window class could not be registered in this process".to_owned(),
        });
    }

    // Built before the window, so that a failed `CreateWindowExW` still drops a
    // guard that unregisters the class. Assigning the handle afterwards is what
    // makes the failing path and the succeeding path share one cleanup.
    let mut nafidha =
        NafidhaMuaqqata { maqbad: HWND(std::ptr::null_mut()), sanf, wahda };

    // SAFETY: the class atom was just registered against `wahda`, the style is a
    // plain overlapped window that is never shown, and every pointer argument is
    // null or points at `ism`, which outlives the call.
    let maqbad = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(sanf as usize as *const u16),
            PCWSTR(ism.as_mut_ptr()),
            WS_OVERLAPPED,
            0,
            0,
            8,
            8,
            None,
            None,
            Some(wahda.into()),
            None,
        )
    };

    match maqbad {
        Ok(maqbad) if !maqbad.is_invalid() => {
            nafidha.maqbad = maqbad;
            Ok(nafidha)
        }
        Ok(_) | Err(_) => Err(KhataTabaqa::JadwalGhayrMawjud {
            wajiha: "IDXGISwapChain".to_owned(),
            sabab: "a hidden window could not be created; this process may have no window station"
                .to_owned(),
        }),
    }
}

/// A structure's size as the `u32` the Win32 headers want.
///
/// Every `cbSize` field is a `u32` and every `size_of` is a `usize`. Written
/// once with the reason attached rather than four times with four casts.
#[cfg(windows)]
const fn size_of_u32<T>() -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the structures this is called for are tens of bytes; no Win32 descriptor \
                  approaches u32::MAX and the compiler evaluates this at compile time"
    )]
    {
        size_of::<T>() as u32
    }
}

/// The installed hooks, and the only thing that may remove them.
///
/// Not [`Clone`], not [`Copy`], and it removes its hooks on [`Drop`]. A hook
/// record that could be duplicated would be a hook that two owners each believed
/// they were responsible for restoring, and the second restore would write a
/// saved original over whatever had replaced it in between.
#[derive(Debug)]
pub struct Khataf {
    wajiha: WajihatRusum,
    khanat: Vec<KhanaMakhtufa>,
    athar: Vec<String>,
}

impl Khataf {
    /// An empty hook set for an API.
    #[must_use]
    pub const fn jadeed(wajiha: WajihatRusum) -> Self {
        Self { wajiha, khanat: Vec::new(), athar: Vec::new() }
    }

    /// Which API these hooks are on.
    #[must_use]
    pub const fn wajiha(&self) -> WajihatRusum {
        self.wajiha
    }

    /// How many slots are currently replaced.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.khanat.len()
    }

    /// What has happened to these hooks, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Replaces one vtable entry, capturing what was there.
    ///
    /// `jadwal` is the vtable pointer read out of an interface instance —
    /// the first pointer-sized word of the object. `khana` is the slot index,
    /// one of the `KHANAT_*` constants in this module. `badil` is the
    /// replacement, which must have the exact calling convention and signature
    /// of the method it replaces.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HimayaGhayrQabila`] when the page holding the table cannot
    /// be made writable, which on Windows is usually an anti-tamper product and
    /// is worth saying so, and [`KhataTabaqa::KhatfFashil`] when the slot does
    /// not hold what it was read as — the sign that another hook landed between
    /// the read and the write.
    ///
    /// # Safety
    ///
    /// The caller guarantees that:
    ///
    /// * `jadwal` is a live vtable pointer obtained from an interface instance
    ///   of the class whose slot indices the `KHANAT_*` constants describe, and
    ///   that the table has at least `khana + 1` entries;
    /// * `badil` points at a function whose signature and calling convention
    ///   match the method at `khana` exactly, and which lives for as long as the
    ///   hook is installed — in practice a `'static` function in this module;
    /// * the replacement calls the original it is handed, so the game's own
    ///   frame still presents.
    pub unsafe fn ikhtif(
        &mut self,
        jadwal: *mut *mut c_void,
        khana: usize,
        badil: *mut c_void,
        ism: &'static str,
    ) -> Result<*mut c_void, KhataTabaqa> {
        // SAFETY: the caller guarantees `jadwal` is a vtable with more than
        // `khana` entries, so the offset is in bounds of the same allocation.
        let mawdi = unsafe { jadwal.add(khana) };

        // SAFETY: `mawdi` is in bounds by the line above, is pointer-aligned
        // because a vtable is an array of function pointers, and is initialized
        // because the interface it came from is live. Volatile because another
        // thread's hook may be writing the same slot and the compiler must not
        // assume this read can be reordered with the write below.
        let asli = unsafe { mawdi.read_volatile() };

        iktub_khana(mawdi, badil, asli)?;

        self.athar.push(format!("hooked {ism} at slot {khana}"));
        self.khanat.push(KhanaMakhtufa { khana: mawdi, asli, badil, ism });
        Ok(asli)
    }

    /// Restores every slot this set replaced, in reverse order.
    ///
    /// A slot that no longer holds Taarib's thunk is **left alone**. Another
    /// overlay hooked it afterwards, and writing Taarib's saved original over
    /// that would leave the other overlay calling into a thunk that is about to
    /// be unmapped. The situation is reported rather than fixed, because there
    /// is no correct fix from inside this process: whoever hooked last has to
    /// unhook first, and Taarib cannot make them.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::FakkKhatfFashil`] naming the method, when a slot cannot be
    /// made writable, when the write-back does not read back, or when a slot
    /// holds a third party's hook. All three mean the module cannot be unloaded
    /// and the user is told a restart is what removes it.
    pub fn fukk(&mut self) -> Result<(), KhataTabaqa> {
        let mut aalik: Vec<String> = Vec::new();

        while let Some(sijill) = self.khanat.pop() {
            // SAFETY: `khana` was produced by `ikhtif` from a vtable the caller
            // guaranteed live, is pointer-aligned, and the process has not
            // unloaded the module owning it — which is exactly what the hook
            // being installed prevents.
            let alaan = unsafe { sijill.khana.read_volatile() };

            if !std::ptr::eq(alaan.cast_const(), sijill.badil.cast_const()) {
                aalik.push(format!(
                    "{}: the slot holds neither Taarib's hook nor the original, so another \
                     overlay hooked it after Taarib did and unhooking Taarib now would break it",
                    sijill.ism
                ));
                self.athar.push(format!("{} left in place: hooked by something else", sijill.ism));
                continue;
            }

            match iktub_khana(sijill.khana, sijill.asli, sijill.badil) {
                Ok(()) => self.athar.push(format!("unhooked {}", sijill.ism)),
                Err(khata) => aalik.push(format!("{}: {khata}", sijill.ism)),
            }
        }

        if aalik.is_empty() {
            return Ok(());
        }
        Err(KhataTabaqa::FakkKhatfFashil {
            mawdi: self.wajiha.ism().to_owned(),
            sabab: aalik.join("; "),
        })
    }
}

impl Drop for Khataf {
    fn drop(&mut self) {
        // A hook set going out of scope with hooks still installed would leave
        // thunks pointing into a module that is about to be unloaded, which is
        // a crash at the next frame rather than at the next line. The result is
        // deliberately discarded: `fukk` has already recorded what happened in
        // `athar`, and there is nothing a destructor can usefully do about a
        // failure it cannot report.
        if !self.khanat.is_empty() {
            let _ = self.fukk();
        }
    }
}

/// Writes one pointer into a vtable slot, making the page writable around it.
///
/// `mutawaqqa` is what the slot must still hold for the write to be allowed.
/// Checked *after* protection is changed and *before* the value is written,
/// which is the only ordering that catches a hook landing in the window between
/// the caller's read and this write.
fn iktub_khana(
    khana: *mut *mut c_void,
    qeema: *mut c_void,
    mutawaqqa: *mut c_void,
) -> Result<(), KhataTabaqa> {
    let hirasa = HirasatKitaba::iftah(khana)?;

    // SAFETY: `hirasa` proves the page is writable for at least one pointer at
    // `khana`, and the pointer is aligned because a vtable is an array of
    // function pointers.
    let alaan = unsafe { khana.read_volatile() };
    if !std::ptr::eq(alaan.cast_const(), mutawaqqa.cast_const()) {
        return Err(KhataTabaqa::KhatfFashil {
            mawdi: "a swap chain method".to_owned(),
            sabab: "the table entry changed between being read and being written, which means \
                    something else hooked the same method at the same moment"
                .to_owned(),
        });
    }

    // SAFETY: as above, and volatile so the write is not reordered with the
    // check immediately preceding it or elided as dead.
    unsafe { khana.write_volatile(qeema) };

    // SAFETY: as above. Read back rather than assumed: a page that reports as
    // writable and silently discards writes is what a hypervisor-based
    // anti-tamper product does, and an overlay that believed its own write
    // would call a thunk that was never installed.
    let baad = unsafe { khana.read_volatile() };
    drop(hirasa);

    if std::ptr::eq(baad.cast_const(), qeema.cast_const()) {
        Ok(())
    } else {
        Err(KhataTabaqa::KhatfFashil {
            mawdi: "a swap chain method".to_owned(),
            sabab: "the write did not take effect, which is what a page protected by an \
                    anti-tamper driver looks like"
                .to_owned(),
        })
    }
}

/// Write permission over one pointer, restored when this value is dropped.
///
/// A guard rather than a pair of calls, because every early return between
/// "made it writable" and "put it back" would otherwise leave a page in a
/// game's address space more permissive than the game left it. There are three
/// such returns in [`iktub_khana`] alone.
#[derive(Debug)]
struct HirasatKitaba {
    #[cfg(windows)]
    oinwan: *mut c_void,
    #[cfg(windows)]
    sabiqa: windows::Win32::System::Memory::PAGE_PROTECTION_FLAGS,
    #[cfg(not(windows))]
    bidaya: *mut c_void,
    #[cfg(not(windows))]
    tul: usize,
}

impl HirasatKitaba {
    /// Makes the page holding `khana` writable.
    #[cfg(windows)]
    fn iftah(khana: *mut *mut c_void) -> Result<Self, KhataTabaqa> {
        use windows::Win32::System::Memory::{
            PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS, VirtualProtect,
        };

        let oinwan = khana.cast::<c_void>();
        let mut sabiqa = PAGE_PROTECTION_FLAGS(0);

        // SAFETY: `oinwan` points into a live vtable, `size_of::<*mut c_void>()`
        // is the exact span being written, and `sabiqa` is a valid out-param.
        // `VirtualProtect` rounds to page granularity itself, which is why the
        // guard records the address rather than a page-aligned base.
        let natija = unsafe {
            VirtualProtect(
                oinwan,
                size_of::<*mut c_void>(),
                PAGE_EXECUTE_READWRITE,
                &raw mut sabiqa,
            )
        };
        if natija.is_err() {
            return Err(KhataTabaqa::HimayaGhayrQabila {
                oinwan: oinwan as usize as u64,
                tul: size_of::<*mut c_void>(),
            });
        }
        Ok(Self { oinwan, sabiqa })
    }

    /// Makes the page holding `khana` writable.
    #[cfg(not(windows))]
    fn iftah(khana: *mut *mut c_void) -> Result<Self, KhataTabaqa> {
        let hajm_safha = hajm_safha();
        let khaam = khana as usize;
        let bidaya = khaam & !(hajm_safha.saturating_sub(1));
        // A pointer can straddle a page boundary only on a target where a
        // vtable is under-aligned, which none of these are — but the span is
        // computed rather than assumed, because a one-page `mprotect` over a
        // straddling write silently protects half of it.
        let nihaya = khaam.saturating_add(size_of::<*mut c_void>());
        let tul = nihaya.saturating_sub(bidaya);

        // SAFETY: `bidaya` is page-aligned by construction and `tul` covers the
        // pointer being written. `PROT_READ | PROT_WRITE | PROT_EXEC` is what
        // the page already had for a vtable in a mapped image, plus write.
        let natija = unsafe {
            libc::mprotect(
                bidaya as *mut c_void,
                tul,
                libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
            )
        };
        if natija != 0 {
            return Err(KhataTabaqa::HimayaGhayrQabila {
                oinwan: khaam as u64,
                tul: size_of::<*mut c_void>(),
            });
        }
        Ok(Self { bidaya: bidaya as *mut c_void, tul })
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
        // a failure here leaves a page more permissive than it was rather than
        // less — an audit finding, not a crash.
        unsafe {
            let _ = VirtualProtect(
                self.oinwan,
                size_of::<*mut c_void>(),
                self.sabiqa,
                &raw mut mahjura,
            );
        }
    }

    #[cfg(not(windows))]
    fn drop(&mut self) {
        // SAFETY: the same span this guard made writable. Restored to
        // read-and-execute, which is what a mapped image's text and rodata
        // carry; the write permission this guard added is what is being taken
        // away.
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
    // SAFETY: `sysconf` with a valid name is always safe to call and returns a
    // `c_long`; a negative return means the name is not supported, which for
    // `_SC_PAGESIZE` does not happen on any target this builds for, and the
    // fallback covers it regardless.
    let khaam = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    usize::try_from(khaam).unwrap_or(4096)
}

/// The captured original of a hooked method, shared with the thunk that calls it.
///
/// A thunk is an `extern "system" fn` with no state of its own, so the original
/// it must call has to live somewhere it can reach without a parameter. An
/// atomic rather than a `static mut`: the hook is installed on one thread and
/// called on the game's render thread, and a plain static would be a data race
/// the moment those differ.
#[derive(Debug)]
pub struct AslMahfuz(AtomicPtr<c_void>);

impl AslMahfuz {
    /// An empty slot.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self(AtomicPtr::new(std::ptr::null_mut()))
    }

    /// Records the original, before the hook goes live.
    ///
    /// `Release` ordering, paired with the `Acquire` in [`AslMahfuz::iqra`]: the
    /// thunk must not be able to observe a non-null original before the writes
    /// that produced it are visible, and on a weakly-ordered target that is not
    /// automatic.
    pub fn ihfaz(&self, asl: *mut c_void) {
        self.0.store(asl, Ordering::Release);
    }

    /// The original, or null if the hook is not live.
    ///
    /// A thunk that reads null must call nothing and return a benign value —
    /// there is a window during teardown where the slot has been restored and a
    /// call already in flight is still inside the thunk.
    #[must_use]
    pub fn iqra(&self) -> *mut c_void {
        self.0.load(Ordering::Acquire)
    }

    /// Clears the slot as the hook is removed.
    pub fn imsah(&self) {
        self.0.store(std::ptr::null_mut(), Ordering::Release);
    }
}

/// How many calls are currently inside the overlay's thunks.
///
/// Teardown waits for this to reach zero before the module can be unloaded. The
/// vtable restore stops *new* calls from entering; it does nothing about a call
/// that is already past the entry and about to execute the next instruction in
/// a module that is being unmapped.
#[derive(Debug)]
pub struct AaddadDukhul(AtomicUsize);

impl AaddadDukhul {
    /// A counter at zero.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self(AtomicUsize::new(0))
    }

    /// Marks entry into a thunk.
    pub fn idkhul(&self) {
        let _ = self.0.fetch_add(1, Ordering::Acquire);
    }

    /// Marks exit from a thunk.
    pub fn khuruj(&self) {
        let _ = self.0.fetch_sub(1, Ordering::Release);
    }

    /// How many calls are inside right now.
    #[must_use]
    pub fn qaim(&self) -> usize {
        self.0.load(Ordering::Acquire)
    }

    /// Whether every call has left.
    #[must_use]
    pub fn faragh(&self) -> bool {
        self.qaim() == 0
    }
}

/// A scope guard that counts a thunk's entry and exit.
///
/// Used at the top of every thunk. Written as a guard rather than a pair of
/// calls because a thunk has early returns — a null original, an overlay that
/// is disabled — and each one would otherwise leak a count that teardown then
/// waits on forever.
#[derive(Debug)]
pub struct HirasatDukhul<'a>(&'a AaddadDukhul);

impl<'a> HirasatDukhul<'a> {
    /// Enters, and leaves when dropped.
    #[must_use]
    pub fn udkhul(aaddad: &'a AaddadDukhul) -> Self {
        aaddad.idkhul();
        Self(aaddad)
    }
}

impl Drop for HirasatDukhul<'_> {
    fn drop(&mut self) {
        self.0.khuruj();
    }
}

/// Which graphics APIs are present in this process, and which is preferred.
///
/// Reports rather than decides, and reports what was *seen* rather than what was
/// concluded: a game that links `d3d11.dll` and renders with Vulkan is common
/// enough that the module list alone is not an answer. The caller confirms by
/// finding a live swap chain, and this orders the search so the confirmation
/// starts with the likeliest.
///
/// # Errors
///
/// [`KhataTabaqa::ApiGhayrMawjud`] carrying the whole trail when nothing was
/// found — which for a game whose renderer initializes lazily is a "try again
/// once the picture appears" rather than a refusal, and the error's
/// [`taarib_usus::khata::Khutwa`] says exactly that.
pub fn hal_yumkin() -> Result<Vec<WajihatRusum>, KhataTabaqa> {
    let mut mawjuda = Vec::new();
    let mut athar = Vec::new();

    for wajiha in WajihatRusum::jamee() {
        let mut wujidat = false;
        for maktaba in wajiha.maktabat() {
            if maktaba_muhammala(maktaba) {
                athar.push(format!("{}: {maktaba} is loaded", wajiha.ism()));
                wujidat = true;
            }
        }
        if wujidat {
            mawjuda.push(wajiha);
        } else {
            athar.push(format!("{}: none of its modules are loaded", wajiha.ism()));
        }
    }

    if mawjuda.is_empty() {
        return Err(KhataTabaqa::ApiGhayrMawjud { athar });
    }
    Ok(mawjuda)
}

/// Whether a named module is already loaded in this process.
///
/// Deliberately does **not** load it. `LoadLibrary` on `d3d12.dll` in a process
/// that never used D3D12 would make this function's own answer true, and an
/// overlay that reported a game as D3D12 because it had just loaded D3D12 into
/// it would be reporting on itself.
#[cfg(windows)]
fn maktaba_muhammala(ism: &str) -> bool {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::PCWSTR;

    let mut mawsu: Vec<u16> = ism.encode_utf16().collect();
    mawsu.push(0);
    // SAFETY: `mawsu` is nul-terminated and outlives the call.
    // `GetModuleHandleW` does not load anything; it looks the name up in the
    // process's existing module list and fails if it is not there.
    unsafe { GetModuleHandleW(PCWSTR(mawsu.as_ptr())) }.is_ok()
}

/// Whether a named object is already loaded in this process.
///
/// `RTLD_NOLOAD` is the whole point: it returns a handle only if the object is
/// already mapped, and null otherwise, without loading it.
#[cfg(not(windows))]
fn maktaba_muhammala(ism: &str) -> bool {
    let Ok(mawsu) = std::ffi::CString::new(ism) else {
        return false;
    };
    // SAFETY: `mawsu` is a valid nul-terminated C string that outlives the call.
    // `RTLD_NOLOAD` means this maps nothing; it reports on what is already
    // there. The handle is closed immediately because this function's only
    // output is whether it was non-null.
    let maqbad = unsafe { libc::dlopen(mawsu.as_ptr(), libc::RTLD_LAZY | libc::RTLD_NOLOAD) };
    if maqbad.is_null() {
        return false;
    }
    // SAFETY: `maqbad` was just returned non-null by `dlopen`, and `RTLD_NOLOAD`
    // still increments the reference count, so this drops the reference this
    // call took and not the one the game holds.
    unsafe {
        let _ = libc::dlclose(maqbad);
    }
    true
}

/// The vtable of a COM interface instance.
///
/// The first pointer-sized word of any COM object is its vtable pointer. This is
/// guaranteed by the ABI every Microsoft compiler implements and is what makes
/// vtable hooking possible at all.
///
/// # Safety
///
/// `wajiha` must point at a live COM interface instance. Passing anything else
/// reads a pointer out of whatever is at that address and every subsequent
/// vtable operation is then against arbitrary memory.
#[must_use]
pub const unsafe fn jadwal_min_wajiha(wajiha: *mut c_void) -> *mut *mut c_void {
    // SAFETY: the caller guarantees `wajiha` points at a live COM interface,
    // whose first word is its vtable pointer by the COM ABI. Read as a
    // pointer-to-pointer because that is what the word is.
    unsafe { wajiha.cast::<*mut *mut c_void>().read() }
}
