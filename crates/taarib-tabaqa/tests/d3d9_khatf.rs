//! The Direct3D 9 hook, proved against a method table this file builds.
//!
//! No device, no driver and no game. What is asserted is everything about the
//! hook that does not need one, and each of the four is a real defect that has
//! shipped in other overlays:
//!
//! * [`khanat_tutabiq_al_jadwal`] — the slot numbers are the ones the interface
//!   really has. They are checked against `windows`' own generated vtable
//!   layout with [`core::mem::offset_of`], not against a hand count, because a
//!   hand count is exactly what is wrong when an overlay writes into
//!   `GetSwapChain` believing it is `Present`.
//! * [`yakhtif_wa_yafukk`] — installing replaces the slot with Taarib's thunk
//!   and returns the original, and removing puts the original back.
//! * [`yarfud_istiaadat_khana_ghayrih`] — **the refusal.** A slot that no longer
//!   holds Taarib's thunk is left alone and reported, because writing Taarib's
//!   saved original over another product's hook would leave that product calling
//!   into a thunk that is about to be unmapped. This is the rule Resident Evil
//!   4's `Bin32` makes concrete: `re4_tweaks` is already hooking in that process.
//! * [`al_isqat_yafukk`] — a hook set dropped without being removed removes
//!   itself, so an early return cannot leave a thunk pointing into a module that
//!   is about to unload.
//!
//! Windows only, and not because of Direct3D. [`taarib_tabaqa::khataf`] changes
//! page protection to write a table entry and restores what it found; on Windows
//! that round-trips exactly, and on Linux the restore sets read-and-execute,
//! which is right for a mapped image's vtable and would make the heap page this
//! file allocates unwritable for everything else on it.

#![cfg(windows)]
#![allow(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code, and \
              refusing to panic here would mean a test that cannot fail"
)]

use core::ffi::c_void;
use core::mem::offset_of;

use taarib_tabaqa::khata::KhataTabaqa;
use taarib_tabaqa::khataf::{
    KHANAT_ISTIAADA, KHANAT_ISTIAADA_MUMTADDA, KHANAT_TAQDEEM9, KHANAT_TAQDEEM_MUMTADD, Khataf,
};
use taarib_tabaqa::wajiha::WajihatRusum;
use windows::Win32::Graphics::Direct3D9::{IDirect3DDevice9Ex_Vtbl, IDirect3DDevice9_Vtbl};

/// How many entries the stub table has.
///
/// `IDirect3DDevice9Ex`'s whole table, so the extended slots are in bounds too.
const ADAD_KHANAT: usize = 160;

// The three stand-ins below each return a different number, and that is
// load-bearing rather than decorative. Written as empty bodies they compile to
// identical code, and both the MSVC linker and LLVM fold identical functions
// onto one address — so `thunk_wahmi` and `thunk_ghareeb` became the same
// pointer, the refusal test saw Taarib's own thunk still in the slot, and it
// passed for the wrong reason. Distinct bodies are what make the three
// addresses distinct.

/// Stands in for the real method at whichever slot is being hooked.
///
/// Never called through. It exists to be a distinguishable non-null address,
/// which is what the assertions compare against.
extern "system" fn asl_wahmi() -> usize {
    0x0A5C_0000
}

/// Stands in for Taarib's thunk.
extern "system" fn thunk_wahmi() -> usize {
    0x7AA5_1B00
}

/// Stands in for another product's hook, installed after Taarib's.
extern "system" fn thunk_ghareeb() -> usize {
    0x6417_E200
}

/// The three stand-ins really are three addresses.
///
/// Asserted rather than assumed, because the whole of the refusal test rests on
/// it and a linker that folded two of them would make that test pass while
/// proving nothing.
#[test]
fn al_thalatha_anawin_mukhtalifa() {
    let anawin = [
        asl_wahmi as *const () as usize,
        thunk_wahmi as *const () as usize,
        thunk_ghareeb as *const () as usize,
    ];
    assert_ne!(anawin[0], anawin[1], "the original and Taarib's thunk folded onto one address");
    assert_ne!(anawin[1], anawin[2], "Taarib's thunk and the third party's folded onto one");
    assert_ne!(anawin[0], anawin[2], "the original and the third party's thunk folded onto one");
}

/// A method table of [`ADAD_KHANAT`] entries, every one of them [`asl_wahmi`].
///
/// A `Vec` on the heap rather than a mapped page: the hook makes the page
/// writable and restores the protection it found, which for a heap page is
/// read-write both before and after.
fn jadwal_wahmi() -> Vec<*mut c_void> {
    vec![asl_wahmi as *const () as *mut c_void; ADAD_KHANAT]
}

/// The slot index a vtable field's byte offset corresponds to.
fn khana_min_izaha(izaha: usize) -> usize {
    #[expect(
        clippy::integer_division,
        reason = "a vtable is an array of function pointers, so a field's byte offset is exactly \
                  its index times the pointer size and the division is exact by construction"
    )]
    {
        izaha / size_of::<*const c_void>()
    }
}

/// The slot numbers are the ones the interface really has.
///
/// Checked against the layout `windows` generates from the Windows metadata
/// rather than against the derivation in the constants' own documentation. The
/// derivation is what a reviewer reads; this is what the process executes, and
/// a hooking overlay that gets one wrong writes a thunk into some other method
/// and corrupts a game in a way whose stack trace names nothing.
#[test]
fn khanat_tutabiq_al_jadwal() {
    assert_eq!(
        khana_min_izaha(offset_of!(IDirect3DDevice9_Vtbl, Present)),
        KHANAT_TAQDEEM9,
        "IDirect3DDevice9::Present is not at the slot the hook writes"
    );
    assert_eq!(
        khana_min_izaha(offset_of!(IDirect3DDevice9_Vtbl, Reset)),
        KHANAT_ISTIAADA,
        "IDirect3DDevice9::Reset is not at the slot the hook writes"
    );
    assert_eq!(
        khana_min_izaha(offset_of!(IDirect3DDevice9Ex_Vtbl, PresentEx)),
        KHANAT_TAQDEEM_MUMTADD,
        "IDirect3DDevice9Ex::PresentEx is not at the slot the hook writes"
    );
    assert_eq!(
        khana_min_izaha(offset_of!(IDirect3DDevice9Ex_Vtbl, ResetEx)),
        KHANAT_ISTIAADA_MUMTADDA,
        "IDirect3DDevice9Ex::ResetEx is not at the slot the hook writes"
    );
    // The extended table is the whole of the plain one plus its own methods, so
    // the stub has to be at least as long as the largest slot the hook touches.
    assert!(
        KHANAT_ISTIAADA_MUMTADDA < ADAD_KHANAT,
        "the stub table is shorter than the slots this test writes into"
    );
}

/// Installing replaces the slot and removing puts the original back.
#[test]
fn yakhtif_wa_yafukk() {
    let mut jadwal = jadwal_wahmi();
    let asas = jadwal.as_mut_ptr();
    let mut khataf = Khataf::jadeed(WajihatRusum::Direct3D9);

    // SAFETY: `asas` addresses a live array of `ADAD_KHANAT` pointers, which is
    // longer than both slots written below; `thunk_wahmi` is a `'static`
    // function in this file. Nothing calls through the table, so the signature
    // requirement the real hook carries is not in play here.
    let asl = unsafe {
        khataf.ikhtif(
            asas,
            KHANAT_TAQDEEM9,
            thunk_wahmi as *const () as *mut c_void,
            "IDirect3DDevice9::Present",
        )
    };
    let Ok(asl) = asl else {
        panic!("the Present slot could not be hooked");
    };
    assert!(
        core::ptr::eq(asl.cast_const(), (asl_wahmi as *const ()).cast::<c_void>()),
        "the hook must answer with what the slot held before it wrote"
    );
    assert_eq!(
        jadwal.get(KHANAT_TAQDEEM9).copied(),
        Some(thunk_wahmi as *const () as *mut c_void),
        "the slot must hold Taarib's thunk after the hook is installed"
    );

    // SAFETY: as above, for the Reset slot.
    let thani = unsafe {
        khataf.ikhtif(
            asas,
            KHANAT_ISTIAADA,
            thunk_wahmi as *const () as *mut c_void,
            "IDirect3DDevice9::Reset",
        )
    };
    assert!(thani.is_ok(), "the Reset slot could not be hooked");
    assert_eq!(khataf.adad(), 2, "two slots were replaced");

    match khataf.fukk() {
        Ok(()) => {},
        Err(khata) => panic!("the hooks would not come out: {khata}"),
    }
    assert_eq!(khataf.adad(), 0, "no slot may remain recorded after a clean removal");
    for khana in [KHANAT_TAQDEEM9, KHANAT_ISTIAADA] {
        assert_eq!(
            jadwal.get(khana).copied(),
            Some(asl_wahmi as *const () as *mut c_void),
            "slot {khana} was not restored to what the game had there"
        );
    }
}

/// A slot another product hooked after Taarib is left alone, and said so.
///
/// This is the rule the brief calls out and it has teeth here rather than in a
/// comment. Writing Taarib's saved original back would erase the other
/// product's hook and leave it calling into a thunk inside a module that is
/// about to be unmapped — a crash in somebody else's code with Taarib's name
/// nowhere in it. There is no correct fix from inside the process: whoever
/// hooked last has to unhook first, and Taarib cannot make them. So the slot is
/// left exactly as it is and the situation is reported.
#[test]
fn yarfud_istiaadat_khana_ghayrih() {
    let mut jadwal = jadwal_wahmi();
    let asas = jadwal.as_mut_ptr();
    let mut khataf = Khataf::jadeed(WajihatRusum::Direct3D9);

    // SAFETY: as `yakhtif_wa_yafukk`.
    let hukm = unsafe {
        khataf.ikhtif(
            asas,
            KHANAT_TAQDEEM9,
            thunk_wahmi as *const () as *mut c_void,
            "IDirect3DDevice9::Present",
        )
    };
    assert!(hukm.is_ok(), "the Present slot could not be hooked");

    // Another product hooks the same slot afterwards. This is exactly what
    // `re4_tweaks` does in Resident Evil 4's process, from a `dinput8.dll` proxy
    // that is already there before Taarib is.
    let Some(khana) = jadwal.get_mut(KHANAT_TAQDEEM9) else {
        panic!("the stub table is shorter than the slot being written");
    };
    *khana = thunk_ghareeb as *const () as *mut c_void;

    let natija = khataf.fukk();
    let Err(khata) = natija else {
        panic!("unhooking a slot another product had taken must not succeed");
    };
    assert!(
        matches!(khata, KhataTabaqa::FakkKhatfFashil { .. }),
        "the refusal must be reported as a failed removal, not absorbed: {khata}"
    );

    assert_eq!(
        jadwal.get(KHANAT_TAQDEEM9).copied(),
        Some(thunk_ghareeb as *const () as *mut c_void),
        "the other product's hook must still be in the slot; Taarib does not restore a pointer \
         it did not install"
    );

    let athar = khataf.athar().join("; ");
    assert!(
        athar.contains("left in place"),
        "the diagnostics must record that the slot was left alone: {athar}"
    );
}

/// A hook set that goes out of scope removes its own hooks.
///
/// Every early return between installing the first slot and publishing the set
/// would otherwise leave a thunk in a table pointing into a module that is about
/// to be unloaded — a crash on the next frame rather than on the next line.
#[test]
fn al_isqat_yafukk() {
    let mut jadwal = jadwal_wahmi();
    let asas = jadwal.as_mut_ptr();
    {
        let mut khataf = Khataf::jadeed(WajihatRusum::Direct3D9);
        // SAFETY: as `yakhtif_wa_yafukk`.
        let hukm = unsafe {
            khataf.ikhtif(
                asas,
                KHANAT_ISTIAADA,
                thunk_wahmi as *const () as *mut c_void,
                "IDirect3DDevice9::Reset",
            )
        };
        assert!(hukm.is_ok(), "the Reset slot could not be hooked");
        assert_eq!(
            jadwal.get(KHANAT_ISTIAADA).copied(),
            Some(thunk_wahmi as *const () as *mut c_void),
            "the slot must hold the thunk while the set is alive"
        );
    }
    assert_eq!(
        jadwal.get(KHANAT_ISTIAADA).copied(),
        Some(asl_wahmi as *const () as *mut c_void),
        "dropping the hook set must restore the slot"
    );
}
