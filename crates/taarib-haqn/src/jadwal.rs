//! Virtual table hooking: replace a slot, and put it back only if it is still ours.

use core::ffi::c_void;

use crate::hirasa::iktub_muashir;
use crate::khata::{KhataHaqn, NatijatHaqn};

/// One replaced slot, and everything needed to undo it.
#[derive(Debug)]
struct KhanaMakhtufa {
    khana: *mut *mut c_void,
    asli: *mut c_void,
    badil: *mut c_void,
    ism: &'static str,
}

/// The installed vtable hooks, and the only thing that may remove them.
///
/// Not [`Clone`] and not [`Copy`], and it removes its hooks on [`Drop`]. A hook
/// record that could be duplicated would be a hook two owners each believed
/// they were responsible for restoring, and the second restore would write a
/// saved original over whatever had replaced it in between.
#[derive(Debug)]
pub struct KhatfJadwal {
    ism: String,
    khanat: Vec<KhanaMakhtufa>,
    athar: Vec<String>,
}

impl KhatfJadwal {
    /// An empty hook set, named for the log and the diagnostics bundle.
    #[must_use]
    pub const fn jadeed(ism: String) -> Self {
        Self { ism, khanat: Vec::new(), athar: Vec::new() }
    }

    /// What this set hooks, as the log names it.
    #[must_use]
    pub fn ism(&self) -> &str {
        &self.ism
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

    /// Replaces one vtable entry, answering what was there.
    ///
    /// `jadwal` is the vtable pointer read out of an object — the first
    /// pointer-sized word of the instance. `khana` is the slot index. `badil`
    /// is the replacement, which must have the exact calling convention and
    /// signature of the method it replaces.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::HimayaGhayrQabila`] when the page holding the table cannot
    /// be made writable, [`KhataHaqn::KhatfFashil`] when the slot does not hold
    /// what it was read as, and [`KhataHaqn::KitabaMuhmala`] when the write
    /// does not take.
    ///
    /// # Safety
    ///
    /// The caller guarantees that:
    ///
    /// * `jadwal` is a live vtable pointer taken from an instance of the class
    ///   whose slot numbering `khana` uses, and the table has at least
    ///   `khana + 1` entries;
    /// * `badil` points at a function whose signature and calling convention
    ///   match the method at `khana` exactly, and which outlives the hook — in
    ///   practice a `'static` function;
    /// * the replacement calls the original it is handed, so the game's own
    ///   behaviour still happens.
    pub unsafe fn ikhtif(
        &mut self,
        jadwal: *mut *mut c_void,
        khana: usize,
        badil: *mut c_void,
        ism: &'static str,
    ) -> NatijatHaqn<*mut c_void> {
        // SAFETY: the caller guarantees the table has more than `khana`
        // entries, so the offset stays inside the same allocation.
        let mawdi = unsafe { jadwal.add(khana) };

        // SAFETY: in bounds by the line above, pointer-aligned because a vtable
        // is an array of function pointers, and initialized because the
        // instance it came from is live.
        let asli = unsafe { mawdi.read_volatile() };

        // SAFETY: `mawdi` is live, aligned, and inside this process.
        unsafe { iktub_muashir(mawdi, badil, asli, ism) }?;

        self.athar.push(format!("hooked {ism} at slot {khana}"));
        self.khanat.push(KhanaMakhtufa { khana: mawdi, asli, badil, ism });
        Ok(asli)
    }

    /// Restores every slot this set replaced, in reverse order.
    ///
    /// A slot that no longer holds this hook's own thunk is **left alone**.
    /// Something else hooked it afterwards, and writing the saved original over
    /// that would leave the other hook calling into a thunk about to be
    /// unmapped. The situation is reported rather than fixed, because there is
    /// no correct fix from inside this process: whoever hooked last has to
    /// unhook first, and Taarib cannot make them.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::FakkKhatfFashil`] naming every slot that could not be
    /// restored and why. It means the module cannot be unloaded, and the user
    /// is told a restart is what removes it.
    pub fn fukk(&mut self) -> NatijatHaqn<()> {
        let mut aalik: Vec<String> = Vec::new();

        while let Some(sijill) = self.khanat.pop() {
            // SAFETY: `khana` was produced by `ikhtif` from a table the caller
            // guaranteed live, is pointer-aligned, and the module owning it is
            // still mapped — which is exactly what the hook prevents unloading.
            let alaan = unsafe { sijill.khana.read_volatile() };

            if !std::ptr::eq(alaan.cast_const(), sijill.badil.cast_const()) {
                aalik.push(format!(
                    "{}: the slot holds neither this hook nor the original, so something else \
                     hooked it afterwards and restoring now would break it",
                    sijill.ism
                ));
                self.athar
                    .push(format!("{} left in place: hooked by something else", sijill.ism));
                continue;
            }

            // SAFETY: as above.
            match unsafe { iktub_muashir(sijill.khana, sijill.asli, sijill.badil, sijill.ism) } {
                Ok(()) => self.athar.push(format!("unhooked {}", sijill.ism)),
                Err(khata) => aalik.push(format!("{}: {khata}", sijill.ism)),
            }
        }

        if aalik.is_empty() {
            return Ok(());
        }
        Err(KhataHaqn::FakkKhatfFashil { mawdi: self.ism.clone(), sabab: aalik.join("; ") })
    }
}

impl Drop for KhatfJadwal {
    fn drop(&mut self) {
        // A destructor has nowhere to report to, but leaving a thunk installed
        // in a module about to be unmapped is a crash in somebody's game, so
        // the attempt is always made.
        let _ = self.fukk();
    }
}

/// The vtable pointer of a C++ object: the first pointer-sized word.
///
/// # Safety
///
/// `wajiha` must point at a live object of a class with virtual methods.
#[must_use]
pub unsafe fn jadwal_min_wajiha(wajiha: *mut c_void) -> *mut *mut c_void {
    // SAFETY: the caller guarantees `wajiha` is a live polymorphic object, and
    // the first word of such an object is its vtable pointer under every ABI
    // this product targets.
    unsafe { *wajiha.cast::<*mut *mut c_void>() }
}
