//! البداية — the first launch: every path resolved, every directory created, no way to crash.
//!
//! [`Masarat::takid`] fails on the first directory it cannot create. That is the right shape
//! for the owner's sandbox, which must not proceed half-built, and the wrong one for a user's
//! first run: a machine with one unwritable corner — an antivirus hold on one folder, a
//! read-only remnant left by a backup tool — is still a machine the window, the library and
//! the diagnostics screen can open on, and the diagnostics screen is the one place the
//! failure can ever be explained.
//!
//! So [`jahhiz`] collects instead of raising. It walks every directory the product will later
//! assume exists — the data root, the settings root, and the eleven named subdirectories —
//! creates each with `create_dir_all`, and turns each failure into one [`TahdheerBidaya`]
//! while the loop continues to the next. A writability probe on the data root then closes the
//! gap creation cannot see: a directory that already exists may still refuse writes, and one
//! written-and-removed file turns the scattered failures that would otherwise follow into a
//! single named warning at startup.
//!
//! [`sajjil`] writes the warnings to the log once tracing is alive to receive them. The
//! caller decides what, if anything, degrades; nothing here does more than create, probe and
//! report.

// `main` declares this module private, so clippy reads every `pub(crate)` below as reachable
// only from inside it and asks for plain `pub`. Writing `pub` makes `unreachable_pub`, which
// the workspace denies, fire on the same item instead; the two rules only reconcile where the
// module is declared.
#![expect(
    clippy::redundant_pub_crate,
    reason = "`pub` here trips the workspace's denied `unreachable_pub` on a private module"
)]

use std::fs;
use std::path::PathBuf;

use taarib_usus::masarat::Masarat;

/// The probe file written and removed under the data root to prove it accepts writes.
const MALAF_FAHS: &str = ".taarib_fahs";

/// The warning name for a data root that exists but refuses writes. The caller keys its
/// degradation decisions on this value, so it never changes casually.
const JIDHR_LA_YUKTAB: &str = "jidhr_la_yuktab";

/// One startup warning: a location that could not be prepared, named for the machine, pathed
/// for the user, and carrying the operating system's own words for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TahdheerBidaya {
    /// The machine-readable name: the [`Masarat`] accessor whose directory could not be
    /// created, or [`JIDHR_LA_YUKTAB`] when the data root exists but refuses writes.
    pub(crate) ramz: &'static str,
    /// The path that failed.
    pub(crate) masar: PathBuf,
    /// The cause, exactly as the operating system reported it.
    pub(crate) sabab: String,
}

/// Creates every directory the product will later assume exists, and probes the data root
/// for writability.
///
/// One [`TahdheerBidaya`] per location that could not be prepared, in the order the layout
/// names them; an empty vector is a machine where every location is ready. The probe appends
/// a warning with `ramz` [`JIDHR_LA_YUKTAB`] when the data root itself refuses writes — the
/// one signal the caller keys real degradation on, because everything under the root
/// inherits its refusal.
///
/// # Errors
///
/// Never returns an error and never panics. Each failure becomes one warning in the returned
/// list and the loop continues, because a first run with one unwritable corner must still
/// reach the window with the library and the diagnostics screen alive.
#[must_use]
pub(crate) fn jahhiz(masarat: &Masarat) -> Vec<TahdheerBidaya> {
    // Every `Masarat` accessor that names a directory, plus the two roots. Deliberately
    // absent: `manzil()` — the user's home is found, never made — and the file accessors
    // `qaida_bayanat()`/`malaf_idadat()`, which their owners create inside these roots.
    let mujalladat: [(&'static str, PathBuf); 13] = [
        ("jidhr_bayanat", masarat.jidhr_bayanat().to_path_buf()),
        ("jidhr_idadat", masarat.jidhr_idadat().to_path_buf()),
        ("ruqaa", masarat.ruqaa()),
        ("nusakh", masarat.nusakh()),
        ("mashari", masarat.mashari()),
        ("khutut", masarat.khutut()),
        ("sijillat", masarat.sijillat()),
        ("dhakira", masarat.dhakira()),
        ("makhbaa", masarat.makhbaa()),
        ("sandooq", masarat.sandooq()),
        ("mafatih", masarat.mafatih()),
        ("mukawwinat", masarat.mukawwinat()),
        ("hajr", masarat.hajr()),
    ];

    let mut tahdheerat = Vec::new();
    for (ramz, masar) in mujalladat {
        if let Err(sabab) = fs::create_dir_all(&masar) {
            tahdheerat.push(TahdheerBidaya { ramz, masar, sabab: sabab.to_string() });
        }
    }

    let fahs = masarat.jidhr_bayanat().join(MALAF_FAHS);
    match fs::write(&fahs, b"taarib") {
        Ok(()) => {
            // Writability is proven. A probe that cannot then be removed is left for the
            // next launch to overwrite rather than escalated: escalating would report an
            // unwritable root about a root that just took a write.
            let _ = fs::remove_file(&fahs);
        },
        Err(sabab) => {
            tahdheerat.push(TahdheerBidaya {
                ramz: JIDHR_LA_YUKTAB,
                masar: masarat.jidhr_bayanat().to_path_buf(),
                sabab: sabab.to_string(),
            });
        },
    }

    tahdheerat
}

/// Prepares a patch root the user redirected, after their settings have been read.
///
/// [`jahhiz`] cannot do this: it runs before the settings file is opened, because it is what
/// creates the directory that file lives in. So the redirected root — `takhzin.jidhr_ruqaa`,
/// a setting the Studio has always offered and nothing on this side ever honoured — is the
/// one location that becomes known too late for the sweep above.
///
/// Returns the same warning shape for the same reason: a redirect at an unwritable path must
/// degrade and be named, not abort a launch that is otherwise fine.
#[must_use]
pub(crate) fn jahhiz_ruqaa(masarat: &Masarat) -> Option<TahdheerBidaya> {
    let masar = masarat.ruqaa();
    fs::create_dir_all(&masar).err().map(|sabab| TahdheerBidaya {
        ramz: "ruqaa",
        masar,
        sabab: sabab.to_string(),
    })
}

/// Writes each startup warning to the log, exactly once, as an Arabic sentence carrying the
/// path, with `ramz`, `masar` and `sabab` attached as machine-readable fields. An empty
/// slice logs nothing.
///
/// Called after `sijill::hayyi` has installed the subscriber, never before: an event emitted
/// into a process with no subscriber is dropped silently, and these warnings exist precisely
/// so that nothing about a degraded first run is silent.
///
/// # Errors
///
/// Cannot fail: emitting a tracing event has no failure path at the call site.
pub(crate) fn sajjil(tahdheerat: &[TahdheerBidaya]) {
    for tahdheer in tahdheerat {
        tracing::warn!(
            ramz = tahdheer.ramz,
            masar = %tahdheer.masar.display(),
            sabab = %tahdheer.sabab,
            "تحذير عند بدء التشغيل: تعذّر تجهيز المسار {}",
            tahdheer.masar.display()
        );
    }
}

// فحص: epic gog ea ubisoft battlenet itch amazon rockstar riot: absent -> ghayr_mutah. saleem
// فحص: steam.rs:342 hall_jidhr absent -> Ok(None) -> empty; Err only for a set override. saleem
// فحص: xbox.rs:489 locked WindowsApps degrades to an explanatory tanbih, never an Err. saleem
// فحص: heroic legendary lutris bottles playnite: absent -> ghayr_mutah, empty scan. saleem
// فحص: yadawi mahmul: no launcher; clean machine -> empty, bad settings folder -> tanbih. saleem
// فحص: kashf lib.rs:198 ifhas wraps adapter Err as ghayr_mutah + warn; the scan never dies. saleem
