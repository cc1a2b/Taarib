//! Trampoline hooking: detour a function at an absolute address, and undo it.

use core::ffi::c_void;

use crate::khata::{KhataHaqn, NatijatHaqn};

/// One installed detour, and the trampoline that reaches the original.
///
/// Not [`Clone`] and not [`Copy`], and it disables itself on [`Drop`]: two
/// owners each restoring the same prologue would write the saved bytes over
/// whatever replaced them in between.
///
/// The prologue analysis — instruction-length decoding, relative-jump
/// rewriting, allocation within reach of the target, branch islands on ARM64 —
/// is `retour`'s. A second implementation of it here would be a second set of
/// bugs in the hardest code in the product, and this crate's contribution is
/// the refusal discipline around it rather than the decoder.
#[derive(Debug)]
pub struct Masar {
    masar: retour::RawDetour,
    ism: &'static str,
}

impl Masar {
    /// Builds a detour from `hadaf` to `badil` and enables it.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::MasarGhayrQabil`] when the target's prologue cannot be
    /// relocated safely, when no trampoline can be allocated in reach of it, or
    /// when the platform refuses the write. A prologue that cannot be moved is
    /// a refusal, never a best-effort write that corrupts an instruction
    /// boundary.
    ///
    /// # Safety
    ///
    /// The caller guarantees that:
    ///
    /// * `hadaf` is the entry point of a live function in this process;
    /// * `badil` has that function's exact signature and calling convention,
    ///   and outlives the detour;
    /// * `badil` calls [`Masar::asl`] so the original behaviour still happens.
    pub unsafe fn jadeed(
        hadaf: *const c_void,
        badil: *const c_void,
        ism: &'static str,
    ) -> NatijatHaqn<Self> {
        // SAFETY: the caller guarantees `hadaf` is a live function entry and
        // `badil` matches its signature; `retour` reads the prologue and
        // allocates its own trampoline.
        let masar = unsafe { retour::RawDetour::new(hadaf.cast::<()>(), badil.cast::<()>()) }
            .map_err(|sabab| KhataHaqn::MasarGhayrQabil {
                oinwan: hadaf as usize as u64,
                sabab: sabab.to_string(),
            })?;

        // SAFETY: the detour was just built for this target and nothing else
        // holds it; enabling writes the jump `retour` prepared.
        unsafe { masar.enable() }.map_err(|sabab| KhataHaqn::MasarGhayrQabil {
            oinwan: hadaf as usize as u64,
            sabab: sabab.to_string(),
        })?;

        Ok(Self { masar, ism })
    }

    /// The trampoline that reaches the original function.
    ///
    /// The replacement transmutes this to the target's signature and calls it.
    #[must_use]
    pub fn asl(&self) -> *const c_void {
        std::ptr::from_ref(self.masar.trampoline()).cast::<c_void>()
    }

    /// Whether the detour is currently installed.
    #[must_use]
    pub fn mumakkan(&self) -> bool {
        self.masar.is_enabled()
    }

    /// What this detour hooks, as the log names it.
    #[must_use]
    pub const fn ism(&self) -> &'static str {
        self.ism
    }

    /// Removes the detour, restoring the original prologue.
    ///
    /// # Errors
    ///
    /// [`KhataHaqn::FakkKhatfFashil`] when the prologue cannot be written back,
    /// which means the module cannot be unloaded and the user is told a restart
    /// is what removes it.
    pub fn fukk(&self) -> NatijatHaqn<()> {
        if !self.masar.is_enabled() {
            return Ok(());
        }
        // SAFETY: this value owns the detour, so nothing else can be disabling
        // it concurrently, and the target is still mapped because the detour
        // being installed is what keeps it so.
        unsafe { self.masar.disable() }.map_err(|sabab| KhataHaqn::FakkKhatfFashil {
            mawdi: self.ism.to_owned(),
            sabab: sabab.to_string(),
        })
    }
}

impl Drop for Masar {
    fn drop(&mut self) {
        // A destructor has nowhere to report to, but leaving a jump into a
        // module about to be unmapped is a crash in somebody's game.
        let _ = self.fukk();
    }
}
