//! الوجود — is this game actually here, and actually launchable, right now?
//!
//! Discovery reads catalogues. A catalogue is a launcher's record of what it
//! *believes* is installed, and the gap between that belief and the filesystem
//! is where most of a library's confusing entries come from: a game the user
//! deleted by hand, a download paused at four percent, a shader pre-cache that
//! made a directory before any content arrived, an install on an external drive
//! that is not plugged in. Every one of those appears in the launcher's own
//! catalogue and none of them can be patched.
//!
//! This module is the gate between the two. Nothing reaches the library grid
//! without passing through it, and — this is the part that matters — nothing is
//! ever *silently* dropped by it.
//!
//! ## Why a failing game is louder than a passing one
//!
//! The obvious design filters the list and shows what survives. It is wrong, and
//! it is wrong in a way that costs the user their trust rather than their time:
//! a player who owns a game, sees it in Steam, and does not see it in Taarib has
//! no way to learn why. They will conclude the product does not support it. They
//! will be wrong, and they will be reasonable.
//!
//! So every rejection produces a [`HukmWujud`] carrying the specific reason, the
//! launcher's own name for it, and the path that failed the check. The interface
//! shows those in a separate collapsed section with a button that opens the
//! launcher at that game. The user never has to wonder.
//!
//! ## Offline is not absent
//!
//! A game on a drive that is not connected is [`SababGhiyab::QursGhayrMuttasil`],
//! not a missing game, and the distinction is structural rather than cosmetic:
//! [`SababGhiyab::yubqa`] is what tells the store layer to keep the record and
//! its patch state instead of deleting them. Deleting the record would lose the
//! user's installed patches, their artwork override, and their play history for
//! a drive that will be back in a minute.
//!
//! The check is deliberately cheap enough to re-run on every drive-arrival
//! notification, so plugging the drive in restores the games without a rescan.
//!
//! ## What "matches what the launcher recorded" means
//!
//! [`ShahidTanfidhi`] holds a size and a modification time, and the comparison
//! is *not* an equality test. A launcher records the size it downloaded; a
//! filesystem reports the size it stored, and on a compressing filesystem those
//! differ. A modification time survives a copy on one platform and does not on
//! another. So the size must match exactly — a downloaded file either arrived or
//! did not — while the timestamp is compared with a tolerance and its
//! disagreement is a *note*, never a rejection. A game rejected because a backup
//! tool touched its mtime would be the gate doing more harm than the problem it
//! exists to catch.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use taarib_mustalahat::ghiyab::{FiatGhiyab, HukmWujud, SababGhiyab, ShahidTanfidhi};
use taarib_mustalahat::luba::MasdarLuba;

/// How far two recorded modification times may drift before it is worth a note.
///
/// Two hours. Large enough to absorb a timezone-naive launcher, a filesystem
/// that stores local time, and a daylight-saving boundary; small enough that a
/// file genuinely replaced by a store update is still noticed. The value is
/// generous on purpose: the timestamp never rejects, so a wide window costs a
/// missed note and a narrow one costs a wall of false notes nobody reads.
pub const TASAMUH_WAQT: Duration = Duration::from_hours(2);

/// The smallest an install directory can be and still plausibly hold a game.
///
/// Sixty-four kibibytes. Below this the directory is a stub: a launcher that
/// created the tree before downloading, a shader cache with no game behind it,
/// or a leftover after a manual delete. Chosen well under any real game and well
/// over an empty directory tree, so it separates the two without needing to
/// enumerate what a game looks like.
pub const ADNA_HAJM_TATHBEET: u64 = 64 * 1024;

/// Everything the gate needs to judge one game, gathered by its adapter.
///
/// Assembled by the launcher that found the game, because only that launcher
/// knows which of its own states mean "downloading" and where its prefixes live.
/// The gate performs the filesystem work and applies the rules; the adapter
/// supplies the facts.
#[derive(Debug, Clone)]
pub struct TalabWujud<'a> {
    /// Which game.
    pub masdar: &'a MasdarLuba,
    /// The install root the launcher recorded.
    pub jidhr: &'a Path,
    /// The primary executable, when the launcher names one.
    pub tanfidhi: Option<&'a Path>,
    /// What the launcher recorded about that executable.
    pub shahid: ShahidTanfidhi,
    /// A state the launcher itself is reporting, which short-circuits every
    /// filesystem check below it.
    ///
    /// A game the launcher says is downloading is downloading; there is nothing
    /// the filesystem can add and a size check on a partial download would
    /// produce a second, less useful reason for the same situation.
    pub hala_matjar: Option<SababGhiyab>,
    /// The compatibility prefix, when the game runs behind one.
    pub beea: Option<&'a Path>,
    /// The Windows path of the executable inside that prefix, when the launcher
    /// records it that way rather than as a host path.
    pub masar_windows: Option<&'a str>,
}

/// Judges one game.
///
/// The order of the checks is the whole design and is not interchangeable:
///
/// 1. **What the launcher says**, first, because a launcher that reports a
///    download in progress has told us everything and any filesystem check after
///    it would produce a worse description of the same fact.
/// 2. **Is the volume reachable**, before touching the path, because an
///    unreachable network mount makes every subsequent check hang for the
///    filesystem's timeout rather than fail.
/// 3. **The directory**, then its size, then the executable, then the prefix.
///    Outermost first, so the reason names the outermost thing that is wrong —
///    "the folder is gone" rather than "the executable is missing", which is
///    true but unhelpful when the whole install is.
///
/// Runs no recursive directory walk. [`hajm_tathbeet`] samples rather than sums,
/// because the gate re-runs on every drive-arrival event and a full recursive
/// size of a two-hundred-game library would take minutes.
#[must_use]
pub fn ihkum(talab: &TalabWujud<'_>) -> HukmWujud {
    if let Some(hala) = talab.hala_matjar.clone() {
        return HukmWujud::ghaiba(hala);
    }

    if let Some(sabab) = ghiyab_al_hajm(talab.jidhr) {
        return HukmWujud::ghaiba(sabab);
    }

    let bayanat = match std::fs::metadata(talab.jidhr) {
        Ok(bayanat) => bayanat,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return HukmWujud::ghaiba(SababGhiyab::MujalladMafqud {
                masar: talab.jidhr.to_path_buf(),
            });
        },
        Err(sabab) => {
            return HukmWujud::ghaiba(SababGhiyab::MujalladMamnu {
                masar: talab.jidhr.to_path_buf(),
                sabab: sabab.to_string(),
            });
        },
    };
    if !bayanat.is_dir() {
        return HukmWujud::ghaiba(SababGhiyab::MujalladMafqud {
            masar: talab.jidhr.to_path_buf(),
        });
    }
    // Readability is proven by listing rather than by the metadata bit. A
    // directory whose ACL denies traversal reports fine from `metadata` and
    // fails at the first `read_dir`, which would otherwise surface much later as
    // an unexplained empty install.
    let hajm_awwali = match hajm_tathbeet(talab.jidhr) {
        Ok(hajm) => hajm,
        Err(sabab) => {
            return HukmWujud::ghaiba(SababGhiyab::MujalladMamnu {
                masar: talab.jidhr.to_path_buf(),
                sabab: sabab.to_string(),
            });
        },
    };
    if hajm_awwali < ADNA_HAJM_TATHBEET {
        return HukmWujud::ghaiba(SababGhiyab::TathbeetNaqis {
            hajm: hajm_awwali,
            adna: ADNA_HAJM_TATHBEET,
        });
    }

    let mut mulahazat = Vec::new();

    let tanfidhi = match hall_tanfidhi(talab, &mut mulahazat) {
        Ok(masar) => masar,
        Err(sabab) => return HukmWujud::ghaiba(sabab),
    };

    let Some(masar_tanfidhi) = tanfidhi else {
        // No executable was named and none had to be. Phase 5 probes the
        // directory for one, and a game admitted here without a known
        // executable is a game the engine detector will identify or decline —
        // which is a better answer than this module guessing at a binary.
        return HukmWujud::hadira(None, Some(hajm_awwali), mulahazat);
    };

    match std::fs::metadata(&masar_tanfidhi) {
        Ok(bayanat) if bayanat.is_file() => {
            let mukhalafa = qarin_shahid(&masar_tanfidhi, &bayanat, talab.shahid, &mut mulahazat);
            if let Some(sabab) = mukhalafa {
                return HukmWujud::ghaiba(sabab);
            }
            HukmWujud::hadira(Some(masar_tanfidhi), Some(hajm_awwali), mulahazat)
        },
        Ok(_) | Err(_) => HukmWujud::ghaiba(SababGhiyab::TanfidhiMafqud {
            masar: masar_tanfidhi,
        }),
    }
}

/// Resolves the executable, through a compatibility prefix when there is one.
///
/// Returns `Ok(None)` when the launcher named no executable, which is not a
/// failure: several launchers do not, and Phase 5 finds one by probing.
fn hall_tanfidhi(
    talab: &TalabWujud<'_>,
    mulahazat: &mut Vec<String>,
) -> Result<Option<PathBuf>, SababGhiyab> {
    let Some(beea) = talab.beea else {
        return Ok(talab.tanfidhi.map(Path::to_path_buf));
    };

    if !beea.is_dir() {
        return Err(SababGhiyab::BeeaMafquda {
            masar: beea.to_path_buf(),
        });
    }

    let Some(masar_windows) = talab.masar_windows else {
        // A prefix with a host-path executable. Legitimate — Lutris records
        // host paths and Proton records Windows ones — so the prefix's presence
        // is all that had to be checked.
        mulahazat.push(format!("runs inside the prefix at {}", beea.display()));
        return Ok(talab.tanfidhi.map(Path::to_path_buf));
    };

    match hall_masar_windows(beea, masar_windows) {
        Some(masar) => {
            mulahazat.push(format!(
                "{masar_windows} resolved to {} inside {}",
                masar.display(),
                beea.display()
            ));
            Ok(Some(masar))
        },
        None => Err(SababGhiyab::TanfidhiKharijBeea {
            beea: beea.to_path_buf(),
            masar_windows: masar_windows.to_owned(),
        }),
    }
}

/// Turns a Windows path into a host path inside a Wine prefix.
///
/// Prefixes map drive letters through symlinks in `dosdevices`, so `C:\Program
/// Files\Game\game.exe` is `<prefix>/dosdevices/c:/Program Files/Game/game.exe`
/// — and `c:` is conventionally a symlink to `../drive_c`, which is why the
/// fallback exists rather than being an alternative worth preferring: a prefix
/// whose `dosdevices` was not built yet still has `drive_c`.
///
/// A UNC path returns [`None`]. Those are real and they are not resolvable to a
/// host path at all, so the honest answer is that the executable is not inside
/// the prefix.
#[must_use]
pub fn hall_masar_windows(beea: &Path, masar_windows: &str) -> Option<PathBuf> {
    let munaqqa = masar_windows.trim();
    if munaqqa.starts_with("\\\\") || munaqqa.starts_with("//") {
        return None;
    }

    let mut ajza = munaqqa.split(['\\', '/']).filter(|juz| !juz.is_empty());
    let awwal = ajza.next()?;
    let harf = awwal.strip_suffix(':')?;
    if harf.len() != 1 {
        return None;
    }
    let harf_saghir = harf.to_ascii_lowercase();

    let baqi: Vec<&str> = ajza.collect();
    let mut murashahat = Vec::with_capacity(2);
    murashahat.push(beea.join("dosdevices").join(format!("{harf_saghir}:")));
    if harf_saghir == "c" {
        murashahat.push(beea.join("drive_c"));
    }

    for jidhr in murashahat {
        if !jidhr.exists() {
            continue;
        }
        let mut masar = jidhr;
        for juz in &baqi {
            masar = masar.join(juz);
        }
        if masar.exists() {
            return Some(masar);
        }
    }
    None
}

/// Compares an executable against what the launcher recorded.
///
/// Size disagreement rejects. Time disagreement notes. The asymmetry is the
/// point and it is argued in this module's header: a downloaded file either
/// arrived whole or did not, while a timestamp is moved by backup tools, file
/// copies, and antivirus software that all leave the bytes alone.
fn qarin_shahid(
    masar: &Path,
    bayanat: &std::fs::Metadata,
    shahid: ShahidTanfidhi,
    mulahazat: &mut Vec<String>,
) -> Option<SababGhiyab> {
    if let Some(musajjal) = shahid.hajm {
        let mawjud = bayanat.len();
        if musajjal != 0 && musajjal != mawjud {
            return Some(SababGhiyab::TanfidhiMukhtalif {
                masar: masar.to_path_buf(),
                musajjal,
                mawjud,
            });
        }
    }

    if let Some(musajjal) = shahid.waqt
        && let Some(mawjud) = waqt_thawani(bayanat)
    {
        let farq = mawjud.abs_diff(musajjal);
        if farq > TASAMUH_WAQT.as_secs() {
            mulahazat.push(format!(
                "the executable's modification time is {farq}s from what the launcher \
                 recorded, which a file copy or a backup tool does without changing a byte"
            ));
        }
    }
    None
}

/// A file's modification time as seconds since the Unix epoch.
///
/// [`None`] for a filesystem that does not record one, and for a time before the
/// epoch — which is what a corrupt timestamp looks like and is not worth
/// modelling as a negative number nothing else in this product can consume.
fn waqt_thawani(bayanat: &std::fs::Metadata) -> Option<u64> {
    bayanat
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|q| q.as_secs())
}

/// Whether the volume holding a path is reachable, and why not when it is not.
///
/// Checked before the path itself. An unreachable SMB mount does not fail fast:
/// it blocks for the mount's own timeout, which on a default Linux configuration
/// is long enough that a library scan appears to have frozen. Probing the volume
/// root first turns that into one timeout for the whole volume instead of one
/// per game on it.
fn ghiyab_al_hajm(jidhr: &Path) -> Option<SababGhiyab> {
    let qurs = jidhr_al_hajm(jidhr)?;
    if qurs.exists() {
        return None;
    }
    if shabakiy(jidhr) {
        Some(SababGhiyab::ShabakaGhayrMutaha {
            masar: jidhr.to_path_buf(),
        })
    } else {
        Some(SababGhiyab::QursGhayrMuttasil {
            qurs: qurs.display().to_string(),
        })
    }
}

/// The volume root of a path, as the platform expresses it.
///
/// `E:\` on Windows. On Unix there is one root and testing it says nothing, so
/// the answer is the mount point's first two components — `/media/games`,
/// `/mnt/library`, `/Volumes/External` — which is where a removable or network
/// mount actually lives on all three Unix platforms this product targets.
#[must_use]
pub fn jidhr_al_hajm(masar: &Path) -> Option<PathBuf> {
    let mut ajza = masar.components();
    let awwal = ajza.next()?;

    #[cfg(windows)]
    {
        use std::path::Component;
        // A prefix component is the drive or the UNC share; either is the thing
        // whose presence decides whether the rest of the path can be reached.
        if matches!(awwal, Component::Prefix(_)) {
            let mut jidhr = PathBuf::from(awwal.as_os_str());
            jidhr.push(std::path::MAIN_SEPARATOR_STR);
            return Some(jidhr);
        }
        None
    }

    #[cfg(not(windows))]
    {
        use std::path::Component;
        if !matches!(awwal, Component::RootDir) {
            return None;
        }
        let mut jidhr = PathBuf::from("/");
        let thani = ajza.next()?;
        let Component::Normal(ism) = thani else {
            return None;
        };
        // Only the conventional mount parents. `/home` and `/usr` are always
        // present, and treating them as volumes would make every ordinary
        // install look like it lived on removable media.
        let mount_parents = ["media", "mnt", "run", "Volumes", "net"];
        let ism_nass = ism.to_str()?;
        if !mount_parents.contains(&ism_nass) {
            return None;
        }
        jidhr.push(ism);
        // `/run/media/<user>/<label>` is what udisks produces, so one more
        // component is taken when the first is `run`.
        if ism_nass == "run" {
            let Some(Component::Normal(thalith)) = ajza.next() else {
                return Some(jidhr);
            };
            jidhr.push(thalith);
        }
        let Some(Component::Normal(rabi)) = ajza.next() else {
            return Some(jidhr);
        };
        jidhr.push(rabi);
        Some(jidhr)
    }
}

/// Whether a path looks like it lives on a network location.
///
/// A UNC path on Windows, and the conventional automount parents on Unix. Used
/// only to choose between two wordings for the same held-not-deleted outcome, so
/// a wrong guess costs a slightly less accurate sentence and nothing else.
fn shabakiy(masar: &Path) -> bool {
    let nass = masar.to_string_lossy();
    nass.starts_with("\\\\") || nass.starts_with("//net/") || nass.starts_with("/net/")
}

/// A cheap, bounded estimate of an install's size.
///
/// **Samples rather than sums.** The gate re-runs whenever a drive arrives and
/// whenever a launcher's state file changes, and a recursive size over a
/// two-hundred-game library is minutes of disk work for a number this function
/// only compares against a 64 KiB floor.
///
/// So it walks the top two levels and stops at [`AQSA_MADAKHIL_ISTITLA`]
/// entries. That is enough to tell an empty stub from a real install, which is
/// the only question being asked. The true size on disk comes from the
/// launcher's own catalogue, which recorded it at download time for free.
///
/// # Errors
///
/// Whatever the filesystem says when the directory cannot be listed, which the
/// caller turns into [`SababGhiyab::MujalladMamnu`].
pub fn hajm_tathbeet(jidhr: &Path) -> std::io::Result<u64> {
    let mut majmu: u64 = 0;
    let mut adad: usize = 0;

    for madkhal in std::fs::read_dir(jidhr)? {
        let madkhal = madkhal?;
        adad = adad.saturating_add(1);
        if adad > AQSA_MADAKHIL_ISTITLA {
            break;
        }
        let Ok(naw) = madkhal.file_type() else {
            continue;
        };
        if naw.is_file() {
            if let Ok(bayanat) = madkhal.metadata() {
                majmu = majmu.saturating_add(bayanat.len());
            }
            if majmu >= ADNA_HAJM_TATHBEET {
                return Ok(majmu);
            }
            continue;
        }
        if !naw.is_dir() {
            continue;
        }
        let Ok(dakhil) = std::fs::read_dir(madkhal.path()) else {
            continue;
        };
        for wahid in dakhil {
            let Ok(wahid) = wahid else { continue };
            adad = adad.saturating_add(1);
            if adad > AQSA_MADAKHIL_ISTITLA {
                return Ok(majmu);
            }
            if let Ok(bayanat) = wahid.metadata()
                && bayanat.is_file()
            {
                majmu = majmu.saturating_add(bayanat.len());
            }
            if majmu >= ADNA_HAJM_TATHBEET {
                return Ok(majmu);
            }
        }
    }
    Ok(majmu)
}

/// How many directory entries the size estimate will look at before stopping.
///
/// Two thousand. A game with fewer files than this in its top two levels is
/// measured exactly; one with more has already passed the floor long before the
/// limit is reached, so stopping early changes no verdict.
pub const AQSA_MADAKHIL_ISTITLA: usize = 2_000;

/// Every verdict from one sweep, keyed by game.
///
/// A map rather than a list because the caller's question is always "what
/// happened to *this* game", and because re-judging a single game on a drive
/// event has to replace one entry rather than rebuild the sweep.
#[derive(Debug, Clone, Default)]
pub struct SijillWujud {
    ahkam: BTreeMap<String, HukmWujud>,
}

impl SijillWujud {
    /// An empty record.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            ahkam: BTreeMap::new(),
        }
    }

    /// Records a verdict.
    pub fn sajjil(&mut self, masdar: &MasdarLuba, hukm: HukmWujud) {
        let _ = self.ahkam.insert(masdar.muarrif(), hukm);
    }

    /// The verdict for one game.
    #[must_use]
    pub fn hukm(&self, masdar: &MasdarLuba) -> Option<&HukmWujud> {
        self.ahkam.get(&masdar.muarrif())
    }

    /// How many games were admitted.
    #[must_use]
    pub fn adad_hadir(&self) -> usize {
        self.ahkam.values().filter(|hukm| hukm.hadir()).count()
    }

    /// Every absent game, grouped by the section it belongs in.
    ///
    /// Sorted within each group by identifier so the collapsed section does not
    /// reorder itself between sweeps — a list that reshuffles while a user reads
    /// it is a list they stop reading.
    #[must_use]
    pub fn ghaiba(&self) -> BTreeMap<FiatGhiyab, Vec<(&str, &SababGhiyab)>> {
        let mut majmuat: BTreeMap<FiatGhiyab, Vec<(&str, &SababGhiyab)>> = BTreeMap::new();
        for (muarrif, hukm) in &self.ahkam {
            if let Some(sabab) = hukm.ghiyab.as_ref() {
                majmuat
                    .entry(sabab.fia())
                    .or_default()
                    .push((muarrif.as_str(), sabab));
            }
        }
        majmuat
    }

    /// Every game whose record must be kept even though it is not showing.
    ///
    /// What the store layer consults before deleting anything. A record that
    /// appears here survives the sweep with its patch state, its artwork
    /// override and its history intact.
    #[must_use]
    pub fn mahfuza(&self) -> Vec<&str> {
        self.ahkam
            .iter()
            .filter(|(_, hukm)| !hukm.hadir() && hukm.yubqa())
            .map(|(muarrif, _)| muarrif.as_str())
            .collect()
    }

    /// Every game the store layer may forget.
    ///
    /// The complement of [`SijillWujud::mahfuza`] among the absent, computed
    /// here rather than by a caller inverting the condition — because inverting
    /// it is where a caller would drop a record that should have been kept.
    #[must_use]
    pub fn qabila_lil_hadhf(&self) -> Vec<&str> {
        self.ahkam
            .iter()
            .filter(|(_, hukm)| !hukm.hadir() && !hukm.yubqa())
            .map(|(muarrif, _)| muarrif.as_str())
            .collect()
    }

    /// A one-line summary for the scan's log.
    #[must_use]
    pub fn taqreer(&self) -> String {
        let ghaiba = self.ghaiba();
        let mut ajza = vec![format!("{} present", self.adad_hadir())];
        for (fia, alaab) in &ghaiba {
            ajza.push(format!(
                "{} {}",
                alaab.len(),
                fia.unwan_injilizi().to_lowercase()
            ));
        }
        ajza.join(", ")
    }
}

/// When this sweep ran, as seconds since the Unix epoch.
///
/// Recorded on the sweep rather than per game so that a stored library can tell
/// how stale its verdicts are without carrying a timestamp on every row.
/// Returns zero if the clock is before the epoch, which is a machine whose
/// battery died rather than a case worth an error path.
#[must_use]
pub fn lahza_al_aan() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |q| q.as_secs())
}
