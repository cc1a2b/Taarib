//! The swap: replacing the running application without ever leaving it broken.
//!
//! Everything here operates on two things only — the path of the running
//! executable and a download that has already been verified. It never imports
//! [`taarib_usus::masarat`], never touches the data root or the component
//! store, and never opens a game directory. An update that damaged somebody's
//! installed patches while replacing a binary would be an update nobody could
//! justify, and the way to guarantee it cannot is to withhold the paths.

use std::path::{Path, PathBuf};

use crate::jalb::MalafMuhaqqaq;
use crate::khata::{KhataTahdith, NatijatTahdith};

/// The suffix the outgoing version is kept under until a clean launch.
const IMTIDAD_SABIQ: &str = ".sabiq";

/// The suffix the incoming version is staged under before the rename dance.
const IMTIDAD_JADEED: &str = ".jadeed";

/// How this installation replaces itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TareeqatTabdil {
    /// A Windows per-user install: the verified installer is run at exit.
    Nsis,
    /// A single-file `AppImage`: replaced in place by rename.
    AppImage,
    /// A macOS bundle: the whole `.app` directory is replaced by rename.
    HuzmatMac,
    /// A package manager owns this installation and Taarib does not touch it.
    Mudar,
}

/// How the running installation replaces itself, decided from where it runs.
///
/// The order matters: `APPIMAGE` is set by the `AppImage` runtime itself and is
/// the only reliable signal that the file the user launched is the outer image
/// rather than the binary inside its mount. A path under `/usr` means a package
/// manager put it there, whatever the format.
///
/// `$APPIMAGE` is read directly. The environment ban points callers at
/// `taarib_usus::idadat`, but this is not configuration: it is the variable an
/// `AppImage` runtime sets to name the file the user is actually carrying, and
/// nothing in a settings file can answer that.
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "see above: $APPIMAGE is the runtime's own contract, not a setting"
)]
pub fn istadill(masar_tanfidhi: &Path) -> TareeqatTabdil {
    if std::env::var_os("APPIMAGE").is_some() {
        return TareeqatTabdil::AppImage;
    }
    if masar_tanfidhi.ancestors().any(|jidd| {
        jidd.extension().is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("app"))
    }) {
        return TareeqatTabdil::HuzmatMac;
    }
    if cfg!(windows) {
        return TareeqatTabdil::Nsis;
    }
    if masar_tanfidhi.starts_with("/usr") {
        return TareeqatTabdil::Mudar;
    }
    TareeqatTabdil::Mudar
}

/// A swap decided but not yet performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhuttatTabdil {
    /// How the swap is carried out.
    pub tareeqa: TareeqatTabdil,
    /// The verified download.
    pub masdar: PathBuf,
    /// Its verified hash, carried so a caller can record what it installed.
    pub sha256: String,
    /// What the swap replaces: the `AppImage` file, or the `.app` directory.
    pub hadaf: PathBuf,
    /// Where the outgoing version is kept until a clean launch.
    pub sabiq: PathBuf,
}

impl KhuttatTabdil {
    /// The command a Windows caller runs at exit, as program and arguments.
    ///
    /// Silent per NSIS convention, and `/D` is deliberately absent: the
    /// installer keeps the per-user location it already recorded.
    #[must_use]
    pub fn amr_nsis(&self) -> Option<(PathBuf, Vec<String>)> {
        (self.tareeqa == TareeqatTabdil::Nsis)
            .then(|| (self.masdar.clone(), vec!["/S".to_owned()]))
    }
}

/// Decides the swap without performing any of it.
///
/// # Errors
///
/// [`KhataTahdith::KhataMalaf`] when the running executable has no parent, and
/// when the destination and the download are on different filesystems — a
/// rename cannot cross one, and copying over a live application is exactly the
/// window in which an interruption leaves nothing runnable.
pub fn khattit(
    tareeqa: TareeqatTabdil,
    masar_tanfidhi: &Path,
    malaf: &MalafMuhaqqaq,
) -> NatijatTahdith<KhuttatTabdil> {
    if tareeqa == TareeqatTabdil::Mudar {
        return Err(KhataTahdith::KhataMalaf {
            masar: masar_tanfidhi.to_path_buf(),
            amal: "replacing an installation a package manager owns",
            sabab: std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "this copy was installed by the system's package manager; update it from there",
            ),
        });
    }

    let hadaf = match tareeqa {
        TareeqatTabdil::AppImage => masar_appimage(masar_tanfidhi),
        TareeqatTabdil::HuzmatMac => jidhr_huzma(masar_tanfidhi).ok_or_else(|| {
            KhataTahdith::KhataMalaf {
                masar: masar_tanfidhi.to_path_buf(),
                amal: "locating the .app bundle around the running binary",
                sabab: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no .app ancestor, so there is no bundle to replace",
                ),
            }
        })?,
        TareeqatTabdil::Nsis | TareeqatTabdil::Mudar => masar_tanfidhi.to_path_buf(),
    };

    if tareeqa != TareeqatTabdil::Nsis {
        akkid_nafs_alnizam(&malaf.masar, &hadaf)?;
    }

    Ok(KhuttatTabdil {
        tareeqa,
        masdar: malaf.masar.clone(),
        sha256: malaf.sha256.clone(),
        sabiq: bi_imtidad(&hadaf, IMTIDAD_SABIQ),
        hadaf,
    })
}

/// Performs the swap for the two formats Taarib replaces itself.
///
/// Rename only, never copy-over-live: the incoming version is staged beside the
/// destination, both it and its directory are flushed, the outgoing version is
/// renamed aside, and the incoming one takes its place. Every step is atomic on
/// the filesystem, so an interruption leaves either the old version in place or
/// the old version under [`IMTIDAD_SABIQ`] — and [`tahaqqaq_bad_iqla`] restores
/// from the second on the next launch.
///
/// `Nsis` is refused: running an installer over the process that invoked it is
/// the caller's decision and belongs at exit, through
/// [`KhuttatTabdil::amr_nsis`].
///
/// # Errors
///
/// [`KhataTahdith::KhataMalaf`] naming the step that failed. On a failure after
/// the outgoing version was moved aside, it is moved back before returning.
pub fn naffidh(khutta: &KhuttatTabdil) -> NatijatTahdith<()> {
    if khutta.tareeqa == TareeqatTabdil::Nsis || khutta.tareeqa == TareeqatTabdil::Mudar {
        return Err(KhataTahdith::KhataMalaf {
            masar: khutta.hadaf.clone(),
            amal: "swapping a format that is replaced by its own installer",
            sabab: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "run the verified installer at exit instead",
            ),
        });
    }

    let jadeed = bi_imtidad(&khutta.hadaf, IMTIDAD_JADEED);
    let _ = izal(&jadeed);

    match khutta.tareeqa {
        TareeqatTabdil::AppImage => {
            std::fs::rename(&khutta.masdar, &jadeed)
                .map_err(|sabab| khata(&jadeed, "staging the new version beside the old", sabab))?;
            ihfaz_tanfidh(&jadeed)?;
            sinkhrin_malaf(&jadeed)?;
        }
        TareeqatTabdil::HuzmatMac => {
            std::fs::rename(&khutta.masdar, &jadeed)
                .map_err(|sabab| khata(&jadeed, "staging the new bundle beside the old", sabab))?;
        }
        // Refused before this function is reached: an NSIS installer and a
        // package manager both replace the application themselves, and neither
        // is swapped by renaming a file. Returning the refusal again rather
        // than asserting it, because a panic here would land inside an update
        // that has already moved the running version aside.
        TareeqatTabdil::Nsis | TareeqatTabdil::Mudar => {
            return Err(KhataTahdith::KhataMalaf {
                masar: khutta.hadaf.clone(),
                amal: "replacing the application in place",
                sabab: std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "this build is replaced by its installer or package manager, not by a swap",
                ),
            });
        }
    }

    let walid = khutta.hadaf.parent().unwrap_or_else(|| Path::new("."));
    sinkhrin_mujallad(walid)?;

    let _ = izal(&khutta.sabiq);
    std::fs::rename(&khutta.hadaf, &khutta.sabiq)
        .map_err(|sabab| khata(&khutta.hadaf, "moving the running version aside", sabab))?;

    if let Err(sabab) = std::fs::rename(&jadeed, &khutta.hadaf) {
        // The destination is empty and the old version is one rename away.
        let _ = std::fs::rename(&khutta.sabiq, &khutta.hadaf);
        return Err(khata(&khutta.hadaf, "putting the new version in place", sabab));
    }
    sinkhrin_mujallad(walid)?;
    Ok(())
}

/// Settles whatever the last swap left behind. Called on every clean startup.
///
/// A `.sabiq` beside a healthy executable is the previous version after a swap
/// that plainly succeeded — this process is the proof — so it is deleted and
/// its path returned for the log. A `.sabiq` beside a missing or empty
/// destination is an interrupted swap: the previous version is put back and the
/// interruption is reported.
///
/// # Errors
///
/// [`KhataTahdith::KhataMalaf`] naming the restored path when a swap was
/// interrupted, and when the restore itself fails.
pub fn tahaqqaq_bad_iqla(masar_tanfidhi: &Path) -> NatijatTahdith<Option<PathBuf>> {
    let tareeqa = istadill(masar_tanfidhi);
    let hadaf = match tareeqa {
        TareeqatTabdil::AppImage => masar_appimage(masar_tanfidhi),
        TareeqatTabdil::HuzmatMac => match jidhr_huzma(masar_tanfidhi) {
            Some(jidhr) => jidhr,
            None => return Ok(None),
        },
        TareeqatTabdil::Nsis | TareeqatTabdil::Mudar => masar_tanfidhi.to_path_buf(),
    };
    let sabiq = bi_imtidad(&hadaf, IMTIDAD_SABIQ);
    if !sabiq.exists() {
        return Ok(None);
    }

    let salih = salih_lil_itlaq(tareeqa, &hadaf);
    if salih {
        izal(&sabiq).map_err(|sabab| khata(&sabiq, "removing the superseded version", sabab))?;
        return Ok(Some(sabiq));
    }

    let _ = izal(&hadaf);
    std::fs::rename(&sabiq, &hadaf)
        .map_err(|sabab| khata(&hadaf, "restoring the previous version", sabab))?;
    Err(KhataTahdith::KhataMalaf {
        masar: hadaf,
        amal: "completing an update",
        sabab: std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "the update was interrupted and the previous version was restored",
        ),
    })
}

/// The file the user actually launched, which inside an `AppImage` is the outer
/// image rather than the binary within its read-only mount.
#[expect(
    clippy::disallowed_methods,
    reason = "as `istadill`: $APPIMAGE is the runtime's own contract, not a setting"
)]
fn masar_appimage(masar_tanfidhi: &Path) -> PathBuf {
    std::env::var_os("APPIMAGE").map_or_else(|| masar_tanfidhi.to_path_buf(), PathBuf::from)
}

/// Whether what sits at the destination is something that could actually run.
///
/// The question a swap's recovery turns on, and the one place it must not be
/// answered loosely: a positive answer deletes the `.sabiq` copy, which is the
/// only remaining copy of the working application.
///
/// For a single file — an `AppImage`, an `.exe`, a bare binary — a non-zero
/// length is the whole of it. For a macOS bundle it is not: `is_dir()` is true
/// of an empty directory, and a rename interrupted part way through leaves
/// exactly that. So the bundle is judged by the two things macOS itself needs
/// to launch it, `Contents/Info.plist` and an executable under
/// `Contents/MacOS`. A shell with neither is not a previous version worth
/// keeping — it is the interruption.
fn salih_lil_itlaq(tareeqa: TareeqatTabdil, hadaf: &Path) -> bool {
    match tareeqa {
        TareeqatTabdil::HuzmatMac => {
            let muhtawa = hadaf.join("Contents");
            let bayan = muhtawa.join("Info.plist");
            let ghayr_farigh = std::fs::metadata(&bayan)
                .is_ok_and(|wasf| wasf.is_file() && wasf.len() > 0);
            ghayr_farigh && fihi_tanfidhi(&muhtawa.join("MacOS"))
        }
        TareeqatTabdil::AppImage | TareeqatTabdil::Nsis | TareeqatTabdil::Mudar => {
            std::fs::metadata(hadaf).is_ok_and(|wasf| wasf.is_file() && wasf.len() > 0)
        }
    }
}

/// Whether a directory holds at least one non-empty regular file.
fn fihi_tanfidhi(mujallad: &Path) -> bool {
    let Ok(qaima) = std::fs::read_dir(mujallad) else { return false };
    qaima.flatten().any(|madkhal| {
        madkhal.metadata().is_ok_and(|wasf| wasf.is_file() && wasf.len() > 0)
    })
}

/// The `.app` directory around a running macOS binary.
fn jidhr_huzma(masar_tanfidhi: &Path) -> Option<PathBuf> {
    masar_tanfidhi
        .ancestors()
        .find(|jidd| {
            jidd.extension().is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("app"))
        })
        .map(Path::to_path_buf)
}

/// A sibling path carrying one more extension.
fn bi_imtidad(masar: &Path, imtidad: &str) -> PathBuf {
    let mut ism = masar.as_os_str().to_os_string();
    ism.push(imtidad);
    PathBuf::from(ism)
}

/// Refuses a swap whose source and destination are on different filesystems.
fn akkid_nafs_alnizam(masdar: &Path, hadaf: &Path) -> NatijatTahdith<()> {
    let Some(walid_masdar) = masdar.parent() else { return Ok(()) };
    let Some(walid_hadaf) = hadaf.parent() else { return Ok(()) };
    let (Ok(awwal), Ok(thani)) =
        (std::fs::canonicalize(walid_masdar), std::fs::canonicalize(walid_hadaf))
    else {
        return Ok(());
    };
    if nafs_aljihaz(&awwal, &thani) {
        return Ok(());
    }
    Err(KhataTahdith::KhataMalaf {
        masar: hadaf.to_path_buf(),
        amal: "replacing the application from another filesystem",
        sabab: std::io::Error::new(
            std::io::ErrorKind::CrossesDevices,
            "the download and the application are on different filesystems, and only a rename \
             on one filesystem can replace a running application without a window in which \
             nothing is runnable",
        ),
    })
}

/// Whether two directories sit on one filesystem.
#[cfg(unix)]
fn nafs_aljihaz(awwal: &Path, thani: &Path) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    match (std::fs::metadata(awwal), std::fs::metadata(thani)) {
        (Ok(min), Ok(ila)) => min.dev() == ila.dev(),
        _ => true,
    }
}

/// On Windows the volume is the path's prefix, and a rename cannot cross one.
#[cfg(not(unix))]
fn nafs_aljihaz(awwal: &Path, thani: &Path) -> bool {
    use std::path::Component;
    let badiya = |masar: &Path| match masar.components().next() {
        Some(Component::Prefix(bad)) => {
            Some(bad.as_os_str().to_string_lossy().to_lowercase())
        }
        _ => None,
    };
    match (badiya(awwal), badiya(thani)) {
        (Some(min), Some(ila)) => min == ila,
        _ => true,
    }
}

/// Carries the executable bit onto the incoming file.
#[cfg(unix)]
fn ihfaz_tanfidh(masar: &Path) -> NatijatTahdith<()> {
    use std::os::unix::fs::PermissionsExt as _;
    let mut idhn = std::fs::metadata(masar)
        .map_err(|sabab| khata(masar, "reading the new version's permissions", sabab))?
        .permissions();
    idhn.set_mode(idhn.mode() | 0o111);
    std::fs::set_permissions(masar, idhn)
        .map_err(|sabab| khata(masar, "making the new version executable", sabab))
}

/// Nothing to carry: Windows decides executability by extension.
#[cfg(not(unix))]
#[expect(clippy::unnecessary_wraps, reason = "one signature across both platforms")]
fn ihfaz_tanfidh(_masar: &Path) -> NatijatTahdith<()> {
    Ok(())
}

/// Flushes a file to the device.
fn sinkhrin_malaf(masar: &Path) -> NatijatTahdith<()> {
    let malaf = std::fs::File::open(masar)
        .map_err(|sabab| khata(masar, "opening the new version to flush it", sabab))?;
    malaf.sync_all().map_err(|sabab| khata(masar, "flushing the new version", sabab))
}

/// Flushes a directory entry, so a rename survives a power loss.
#[cfg(unix)]
fn sinkhrin_mujallad(masar: &Path) -> NatijatTahdith<()> {
    let mujallad = std::fs::File::open(masar)
        .map_err(|sabab| khata(masar, "opening the directory to flush it", sabab))?;
    mujallad.sync_all().map_err(|sabab| khata(masar, "flushing the directory", sabab))
}

/// Windows offers no directory handle to flush; the rename is already ordered.
#[cfg(not(unix))]
#[expect(clippy::unnecessary_wraps, reason = "one signature across both platforms")]
fn sinkhrin_mujallad(_masar: &Path) -> NatijatTahdith<()> {
    Ok(())
}

/// Removes a file or a directory, whichever is there.
fn izal(masar: &Path) -> std::io::Result<()> {
    match std::fs::metadata(masar) {
        Ok(bayan) if bayan.is_dir() => std::fs::remove_dir_all(masar),
        Ok(_) => std::fs::remove_file(masar),
        Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(khata) => Err(khata),
    }
}

/// One local-file failure, named.
fn khata(masar: &Path, amal: &'static str, sabab: std::io::Error) -> KhataTahdith {
    KhataTahdith::KhataMalaf { masar: masar.to_path_buf(), amal, sabab }
}
