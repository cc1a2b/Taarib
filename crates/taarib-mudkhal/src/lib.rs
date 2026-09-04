//! # مدخل تعريب — the loader a game loads on its own
//!
//! The one module Taarib asks a game's own loader to bring in, for the engines
//! that have no third-party framework: `version.dll` beside the executable on
//! Windows, an `LD_PRELOAD` object on Linux, a `DYLD_INSERT_LIBRARIES` object
//! on macOS. `taarib-tathbeet`'s `tarkib` module places it and writes whatever
//! makes the platform load it — a proxy name the game already imports, a Wine
//! override, a launch option.
//!
//! Its whole job is to bring the payload modules sitting beside it into the
//! process and then stop mattering. It hooks nothing, allocates nothing in the
//! game's memory, and holds no state past the load. Everything that acts on the
//! game — the Unreal takeover, the Godot 3 takeover, the overlay — is a payload
//! this module opens; none of them is compiled into it.
//!
//! ## Why loading is separated from acting
//!
//! A proxy DLL is the most fragile thing Taarib ships: it sits in somebody's
//! game directory, it is loaded before anything else, and a fault inside it is
//! a game that does not start. So it contains as little as it can. A payload
//! that fails to load leaves this module forwarding `version.dll` exports
//! exactly as the real one would, and the game runs untranslated — which is the
//! degradation the product promises everywhere else.
//!
//! ## Hard constraints
//!
//! - Nothing is loaded from `DllMain`. The Windows loader lock is held there,
//!   and `LoadLibrary` under it deadlocks against any module that takes the
//!   same lock on another thread.
//! - The real `version.dll` is opened by absolute system path, never by name,
//!   because this module *is* `version.dll` to the game's directory and a name
//!   lookup would resolve back into these forwarding stubs.
//! - Nothing here reads settings, touches the data root, or talks to Studio: it
//!   knows only the directory it was loaded from.
//! - A payload that is absent is not an error. The component the installer
//!   placed is the smallest set that game needs.

#[cfg(any(windows, unix))]
mod hamula;

#[cfg(windows)]
mod nawafidh;

#[cfg(unix)]
mod yuniks;
