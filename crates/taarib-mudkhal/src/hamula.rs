//! The payload set, resolved beside this module, and the record of what happened.
#![expect(
    clippy::redundant_pub_crate,
    reason = "the items kept `pub(crate)` here are reached from the platform \
              modules but from nothing outside this crate; the bare `pub` this \
              lint asks for in a private module trips `unreachable_pub`, which \
              the workspace denies"
)]

use std::path::{Path, PathBuf};

/// The payload modules this loader brings in, in load order.
///
/// Base names without the platform's prefix or extension. The order is the
/// order they are attempted in: the overlay last, because it is the fallback
/// tier and an engine adapter that took the process has no need of it.
const HAMULAT: [&str; 3] =
    ["taarib_muhawwil_unreal", "taarib_muhawwil_godot", "taarib_tabaqa"];

/// The entry point every payload exports, called once after it loads.
///
/// A payload without this symbol is loaded and left alone: its own
/// initialisation ran when the platform loader mapped it, and the absence of
/// the symbol is recorded rather than treated as a failure.
pub(crate) const ISM_BIDAYA: &[u8] = b"taarib_bidaya\0";

/// The log this loader appends to, beside the module itself.
const ISM_SIJILL: &str = "mudkhal.sijill";

/// The cap on that log, past which it is truncated before the next append.
///
/// A game may be launched hundreds of times with this module in place, and a
/// loader that grew a log without bound inside somebody's game directory would
/// be a loader that eventually fills their disk.
const AQSA_SIJILL: u64 = 262_144;

/// The platform file name for a payload base name.
#[must_use]
fn ism_wahda(asas: &str) -> String {
    if cfg!(windows) {
        format!("{asas}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib{asas}.dylib")
    } else {
        format!("lib{asas}.so")
    }
}

/// The payload paths that exist beside this module, in [`HAMULAT`] order.
///
/// An absent payload is not an error: a Godot game's component carries no
/// Unreal adapter, and the component the installer placed is deliberately the
/// smallest set that game needs.
#[must_use]
pub(crate) fn hamulat_mawjuda(jidhr: &Path) -> Vec<PathBuf> {
    HAMULAT
        .iter()
        .map(|asas| jidhr.join(ism_wahda(asas)))
        .filter(|masar| masar.is_file())
        .collect()
}

/// Appends one line to the loader's log, truncating it first when it is full.
///
/// Every failure here is swallowed: this runs inside somebody's game, and a
/// loader that could not write its own log has no business interrupting a
/// launch to say so.
pub(crate) fn sajjil(jidhr: &Path, satr: &str) {
    use std::io::Write as _;

    let masar = jidhr.join(ISM_SIJILL);
    if std::fs::metadata(&masar).is_ok_and(|bayan| bayan.len() > AQSA_SIJILL) {
        let _ = std::fs::remove_file(&masar);
    }
    let Ok(mut malaf) = std::fs::OpenOptions::new().create(true).append(true).open(&masar) else {
        return;
    };
    let _ = writeln!(malaf, "{satr}");
}
