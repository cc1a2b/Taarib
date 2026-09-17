//! Makes the trust anchor a build input cargo can see.
//!
//! `MIRSAT_MALIK` in `src/malik.rs` reads `TAARIB_MIFTAH_ISDAR` through
//! `option_env!`, which cargo does not track: a crate already compiled under
//! one anchor is *not* rebuilt when the variable changes, or appears, or goes
//! away. The symptom is a binary that trusts the development key while every
//! script and log around it says `isdar`, and it refuses every patch the owner
//! signed. It has been hit twice — once by `scripts/isdar.sh` reusing a warm
//! target directory, once by `apps/studio/src-tauri/linux/ibni.sh` reusing a
//! cached staging tree — and both times the workaround was to remember to
//! touch or clean this crate by hand. Remembering is not a mechanism.
//!
//! One line, and the hazard is gone for every caller at once: cargo now
//! rebuilds this crate whenever the anchor differs from the one it last
//! compiled under, including the transition to and from absent.

fn main() {
    println!("cargo:rerun-if-env-changed=TAARIB_MIFTAH_ISDAR");
    println!("cargo:rerun-if-changed=build.rs");
}
