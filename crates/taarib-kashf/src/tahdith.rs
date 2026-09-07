//! التحديث — keeping the library true without rescanning the world.
//!
//! Two halves. [`qarin`] compares a fresh scan against what is already stored
//! and says what actually changed; [`Muraqib`] watches the launchers' library
//! roots so that a refresh happens because something moved, not because a timer
//! fired.
//!
//! ## A game that disappeared is marked absent, never deleted
//!
//! This is the rule the whole module is shaped around, and it is not a
//! preference. Everything Taarib knows about a game outlives the game's files:
//! the installation manifest recording which files a patch replaced, the
//! original-file backups that make an uninstall exact, the translation project
//! somebody spent three months on, the record that this build was verified.
//! All of it is keyed on the game's identity.
//!
//! Deleting the row when the folder disappears would take all of that with it,
//! and the folder disappears for reasons that have nothing to do with the user
//! being finished with the game: an external drive was unplugged, a network
//! share did not mount before Taarib started, a launcher was mid-update, the
//! user uninstalled it to make room for a weekend and will reinstall it on
//! Monday. Every one of those would be a silent, unrecoverable data loss.
//!
//! So absence is a *state*, expressed by [`TaghyeerLuba::Ghabat`] going out and
//! [`TaghyeerLuba::Aadat`] coming back, and the row stays. The cost of being
//! wrong in this direction is a greyed-out card in the library. The cost of
//! being wrong in the other direction is somebody's work.
//!
//! ## A move is a move
//!
//! A user who drags a Steam library from a full SSD to a new drive has not
//! reinstalled anything: the same bytes are at a new address. Reporting that as
//! a deletion followed by an addition would orphan the installation manifest —
//! the record of exactly which files were modified and what they contained
//! before — leaving an installed patch that Taarib can no longer see, cannot
//! verify, and cannot remove. The user would be left with a modified game and
//! no way back.
//!
//! [`TaghyeerLuba::Naqalat`] therefore carries both roots, so the caller
//! re-roots the manifest instead of discarding it. It is a claim about the
//! *address*, not a promise about the contents: Phase 15 still verifies the
//! recorded hashes before acting on a manifest, so a move that turns out to
//! have been a reinstall is caught there rather than trusted here.
//!
//! ## A build id change is a signal, not a detail
//!
//! [`TaghyeerLuba::BinaJadeed`] carries the old identifier as well as the new
//! one, which is what lets the caller separate "this game was updated" from
//! "this is the first time we have seen a build id for it". Only the first of
//! those means an installed patch may have been overwritten by the launcher,
//! which is the trigger for Phase 15's update-survival path; treating a first
//! sighting as an update would raise that alarm on every newly discovered game.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::{Khata, Natija};
use tokio::sync::Mutex;

use crate::fahs::{HalatFahsMatjar, LubaMuktashafa, NatijatFahs, TanbihFahs};
use crate::khata::KhataKashf;

// ---------------------------------------------------------------------------
// What changed
// ---------------------------------------------------------------------------

/// One thing that happened to one game between two scans.
///
/// A single game can produce more than one of these in one refresh — a game
/// that was both moved by the user and updated by its launcher yields
/// [`Naqalat`](Self::Naqalat) and [`BinaJadeed`](Self::BinaJadeed) together,
/// because a caller that acted on only one of them would get the wrong answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaghyeerLuba {
    /// Not in the store at all. Carries the whole discovered game, because
    /// there is nothing stored to merge it into.
    Jadeeda(LubaMuktashafa),

    /// The same installation, at a new address.
    Naqalat {
        /// Where it used to be.
        qadeem: PathBuf,
        /// Where it is now.
        jadeed: PathBuf,
    },

    /// The launcher reports a different build than the one on record.
    BinaJadeed {
        /// The build that was stored — [`None`] when this is the first build
        /// identifier ever seen for this game, which is not an update.
        qadeem: Option<String>,
        /// The build the launcher reports now.
        jadeed: String,
    },

    /// No longer on disk. The row stays; the game is shown as absent.
    Ghabat,

    /// Present again after being absent.
    Aadat,

    /// Still here, still the same build, but something the launcher says about
    /// it is different — its name, its executable, its size, its launch
    /// options, its artwork, or when it was last played.
    Tahdith,
}

/// Everything that changed between a scan and the store.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FarqFahs {
    /// Each change, paired with the launcher identity it belongs to, in scan
    /// order followed by store order. Deterministic: two runs over the same
    /// inputs produce the same list in the same sequence, which is what makes a
    /// refresh reproducible when somebody reports that it did the wrong thing.
    pub taghyeerat: Vec<(MasdarLuba, TaghyeerLuba)>,
    /// How many stored games produced nothing at all — including the ones this
    /// diff deliberately did not judge, because their launcher was not scanned.
    pub bila_taghyeer: usize,
}

impl FarqFahs {
    /// Whether the scan found the library exactly as it was left.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.taghyeerat.is_empty()
    }

    /// How many changes there are.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.taghyeerat.len()
    }

    /// The games newly discovered by this scan.
    #[must_use]
    pub fn jadeeda(&self) -> Vec<&LubaMuktashafa> {
        self.taghyeerat
            .iter()
            .filter_map(|(_, taghyeer)| match taghyeer {
                TaghyeerLuba::Jadeeda(luba) => Some(luba),
                _ => None,
            })
            .collect()
    }

    /// The games whose build changed under an installed patch — the set Phase
    /// 15's update-survival path acts on.
    ///
    /// A game seen with a build identifier for the first time is not in here.
    #[must_use]
    pub fn muhaddatha(&self) -> Vec<&MasdarLuba> {
        self.taghyeerat
            .iter()
            .filter_map(|(masdar, taghyeer)| match taghyeer {
                TaghyeerLuba::BinaJadeed {
                    qadeem: Some(_), ..
                } => Some(masdar),
                _ => None,
            })
            .collect()
    }
}

/// Compares a scan against the store.
///
/// Games are matched on [`MasdarLuba::muarrif`], which resolves a Heroic
/// installation to the store it really belongs to — so a game installed through
/// Heroic and later installed natively is the same game moving, not a second
/// copy appearing.
///
/// ## The launcher guard
///
/// A launcher whose catalogue was not read end to end this round — not
/// installed, its catalogue directory gone, one of its libraries on a drive
/// that is not mounted — has its games left strictly alone: only a result whose
/// verdict is [`HalatFahsMatjar::Tamma`] is believed about what it did *not*
/// list. Without that guard, starting Taarib before an external drive finished
/// mounting would mark an entire library absent in one pass, and every one of
/// those games would flip back on the next refresh. The user would watch their
/// library empty and refill itself, which is exactly how a tool loses trust.
/// The guard used to test whether a root was found or a game was returned,
/// which a Steam client on one drive with its library on another passed while
/// the library drive was unplugged.
///
/// ## Reading the filesystem
///
/// This consults the filesystem: a launcher catalogue outlives the files it
/// describes, and Steam in particular keeps an `appmanifest` for a game whose
/// directory a user deleted by hand. A game the catalogue lists at a path that
/// does not exist is absent, whatever the catalogue believes. That is one
/// `stat` per game, which on a library of six hundred is unmeasurable next to
/// the parse that produced the scan.
///
/// ## What it cannot tell apart, stated honestly
///
/// A game uninstalled and reinstalled to the *identical* path is
/// indistinguishable, from a scan and a filesystem alone, from a game that was
/// simply updated in place. It surfaces as [`TaghyeerLuba::BinaJadeed`], or as
/// [`TaghyeerLuba::Tahdith`] when the launcher publishes no build identifier —
/// and that is the correct signal regardless, because both cases demand the
/// same response: re-verify the installation manifest against what is on disk
/// before trusting it. A reinstall always moves either the build identifier or
/// the launcher's own update timestamp, so it never passes silently.
#[must_use]
pub fn qarin(mukhtashaf: &NatijatFahs, mukhazzan: &[LubaMuktashafa]) -> FarqFahs {
    // First occurrence wins. A store holding two rows for one identity is
    // already inconsistent, and merging identities is the caller's job, not
    // this function's.
    let mut fahras: HashMap<String, &LubaMuktashafa> = HashMap::with_capacity(mukhazzan.len());
    for luba in mukhazzan {
        let _ = fahras.entry(luba.masdar.muarrif()).or_insert(luba);
    }

    // Which launcher families are in a position to be believed this round:
    // only those whose catalogue was read whole. A found root or a non-empty
    // list is not that — a library on an unplugged drive yields both.
    let mut aailat: HashSet<&'static str> = HashSet::new();
    for natija in &mukhtashaf.matajir {
        if natija.hala() == HalatFahsMatjar::Tamma {
            let _ = aailat.insert(natija.matjar);
        }
    }

    let mut taghyeerat: Vec<(MasdarLuba, TaghyeerLuba)> = Vec::new();
    let mut ruiyat: HashSet<String> = HashSet::new();
    let mut taghayyarat: HashSet<String> = HashSet::new();

    for luba in mukhtashaf.alaab() {
        let miftah = luba.masdar.muarrif();
        // The same identity reported twice in one scan — natively and again
        // through Heroic — is diffed on its first appearance, in scanner order.
        if !ruiyat.insert(miftah.clone()) {
            continue;
        }

        match fahras.get(&miftah) {
            None => {
                taghyeerat.push((luba.masdar.clone(), TaghyeerLuba::Jadeeda(luba.clone())));
            },
            Some(qadeem) => {
                let wahida = qarin_wahida(qadeem, luba);
                if !wahida.is_empty() {
                    let _ = taghayyarat.insert(miftah);
                }
                for taghyeer in wahida {
                    taghyeerat.push((luba.masdar.clone(), taghyeer));
                }
            },
        }
    }

    for luba in mukhazzan {
        let miftah = luba.masdar.muarrif();
        if ruiyat.contains(&miftah) || !aailat.contains(luba.masdar.aila().slug()) {
            continue;
        }
        // Stored, its launcher was read, and the launcher no longer lists it.
        if taghayyarat.insert(miftah) {
            taghyeerat.push((luba.masdar.clone(), TaghyeerLuba::Ghabat));
        }
    }

    let bila_taghyeer = mukhazzan
        .iter()
        .filter(|luba| !taghayyarat.contains(&luba.masdar.muarrif()))
        .count();

    FarqFahs {
        taghyeerat,
        bila_taghyeer,
    }
}

/// Everything that changed about one game the store already knows.
fn qarin_wahida(qadeem: &LubaMuktashafa, jadeed: &LubaMuktashafa) -> Vec<TaghyeerLuba> {
    let mut taghyeerat = Vec::new();

    // The launcher still lists it, but the directory it points at is not there.
    // Nothing else can honestly be said about files that do not exist, so no
    // build change and no metadata change is reported alongside this.
    if !jadeed.jidhr.is_dir() {
        taghyeerat.push(TaghyeerLuba::Ghabat);
        return taghyeerat;
    }

    let intaqal = qadeem.jidhr != jadeed.jidhr;
    if intaqal {
        if qadeem.jidhr.is_dir() || nafs_muhtawa(qadeem, jadeed) {
            taghyeerat.push(TaghyeerLuba::Naqalat {
                qadeem: qadeem.jidhr.clone(),
                jadeed: jadeed.jidhr.clone(),
            });
        } else {
            // The old root is gone and what is at the new one is a different
            // build or a different size: this is not the same installation
            // relocated, it is a fresh one somewhere else.
            taghyeerat.push(TaghyeerLuba::Aadat);
        }
    }

    // Only reportable when the scan actually carries a build identifier. A
    // launcher that reported one last time and reports nothing now has changed
    // what it will say, not what is installed, and raising the update alarm on
    // that would be raising it on a launcher upgrade.
    match jadeed.bina_manassa.as_ref() {
        Some(bina) if qadeem.bina_manassa.as_ref() != Some(bina) => {
            taghyeerat.push(TaghyeerLuba::BinaJadeed {
                qadeem: qadeem.bina_manassa.clone(),
                jadeed: bina.clone(),
            });
        },
        _ => {},
    }

    if taghayyarat_bayanat(qadeem, jadeed) {
        taghyeerat.push(TaghyeerLuba::Tahdith);
    }

    taghyeerat
}

/// Whether two records describe the same installed bytes at two addresses.
///
/// Two launchers' worth of evidence, and both are allowed to be silent. A
/// matching build identifier is decisive; a matching reported size supports it;
/// a zero size means the launcher does not report one and says nothing either
/// way.
///
/// When neither launcher publishes a build identifier and neither publishes a
/// size, this answers *yes* — a move. That default is deliberate. Calling a
/// move a reinstall orphans the installation manifest outright: the patch stays
/// applied, Taarib can no longer see it, and the user cannot remove it. Calling
/// a reinstall a move only re-roots a manifest that Phase 15 re-verifies against
/// the files before it acts on it, so the mistake is caught, cheaply, one layer
/// down.
fn nafs_muhtawa(qadeem: &LubaMuktashafa, jadeed: &LubaMuktashafa) -> bool {
    let bina_mutabiq = match (qadeem.bina_manassa.as_ref(), jadeed.bina_manassa.as_ref()) {
        (Some(awwal), Some(thani)) => awwal == thani,
        (None, None) => true,
        _ => false,
    };
    let hajm_mutabiq = qadeem.hajm == 0 || jadeed.hajm == 0 || qadeem.hajm == jadeed.hajm;

    bina_mutabiq && hajm_mutabiq
}

/// Whether anything the launcher says about a game has moved, other than its
/// root and its build — both of which are reported on their own.
fn taghayyarat_bayanat(qadeem: &LubaMuktashafa, jadeed: &LubaMuktashafa) -> bool {
    qadeem.ism != jadeed.ism
        || qadeem.tanfidhi != jadeed.tanfidhi
        || qadeem.hajm != jadeed.hajm
        || qadeem.akhir_tahdith != jadeed.akhir_tahdith
        || qadeem.akhir_laab != jadeed.akhir_laab
        || qadeem.beea != jadeed.beea
        || qadeem.suwar != jadeed.suwar
        || qadeem.khiyarat_tashghil != jadeed.khiyarat_tashghil
        || qadeem.muktamila != jadeed.muktamila
        || qadeem.simat != jadeed.simat
}

// ---------------------------------------------------------------------------
// Watching
// ---------------------------------------------------------------------------

/// The debounce interval a caller should use unless it has a reason not to.
///
/// **One and a half seconds**, and the number is a compromise between two
/// failures that are both easy to hit.
///
/// A launcher installing or updating a game does not write one file. It writes
/// hundreds of thousands of them, continuously, for as long as the download
/// takes, and every one of them produces an event. Undebounced, each event
/// would trigger a rescan, and a rescan of Steam means parsing every
/// `appmanifest_*.acf` on every library on every drive plus the binary
/// `appinfo.vdf` — hundreds of milliseconds of work at best. The rescans would
/// queue behind each other faster than they complete, and the machine would
/// spend a forty-minute download at full CPU achieving nothing, because the
/// answer only becomes interesting once the writing stops.
///
/// Too long is its own failure, and a more visible one. Somebody installs a
/// game, alt-tabs to Taarib, and it is not there. They press refresh. If the
/// interval were ten seconds they would have pressed it twice before the watch
/// fired, and the watcher would have bought nothing over a button.
///
/// One and a half seconds sits between them. It is far longer than the gap
/// between writes during a sustained download, so the quiet window never opens
/// mid-install and the rescan happens once, at the end. It is short enough to
/// read as immediate: a launcher writes its own manifest *last*, after the file
/// traffic has stopped, so the debounce lands naturally on the write that
/// actually changes the answer, and the library updates while the user is still
/// looking at it.
///
/// The debouncer polls at a quarter of this by default, so its own latency is
/// under 400 ms and the interval is what governs.
pub const TABTEEA_MUQTARAHA: Duration = Duration::from_millis(1500);

/// Floor on the debounce interval.
///
/// Below this the debouncer stops debouncing in any useful sense, and a
/// zero-length interval is rejected outright by the underlying crate, which
/// would turn a bad configuration value into a watcher that fails to start.
const TABTEEA_DUNYA: Duration = Duration::from_millis(250);

/// Ceiling on the debounce interval. Past a minute the watch is indistinguishable
/// from no watch at all.
const TABTEEA_ULYA: Duration = Duration::from_mins(1);

/// The source name warnings from this module carry.
const MASDAR_TANBIH: &str = "muraqib";

/// A debounced watch over the launchers' library roots.
///
/// Owns no control flow. It does not spawn a task of its own beyond the one the
/// debouncer needs, does not call back into anything, and does not decide when
/// a rescan happens — [`istaqbil`](Self::istaqbil) is polled by whatever loop
/// already exists, which keeps the decision about *when* to rescan with the
/// component that knows what else is running.
pub struct Muraqib {
    /// The debouncer, which owns the platform watcher and its thread.
    murattib: Debouncer<RecommendedWatcher, RecommendedCache>,
    /// Drained by `istaqbil`. Behind a lock only so that the struct stays
    /// shareable; the channel itself is the queue.
    mustaqbil: Mutex<Receiver<DebounceEventResult>>,
    /// The roots that are genuinely being watched, which may be fewer than were
    /// asked for.
    judhur: Vec<PathBuf>,
    /// Every root that could not be watched, and why.
    tanbihat: Vec<TanbihFahs>,
    /// The interval actually in use, after clamping.
    tabteea: Duration,
}

impl std::fmt::Debug for Muraqib {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Muraqib")
            .field("judhur", &self.judhur)
            .field("tanbihat", &self.tanbihat)
            .field("tabteea", &self.tabteea)
            .finish_non_exhaustive()
    }
}

impl Muraqib {
    /// Starts watching every root it can.
    ///
    /// Roots are deduplicated and any root nested inside another is dropped,
    /// because a recursive watch already covers it and watching both would
    /// deliver every event twice.
    ///
    /// **A root that cannot be watched is a warning, not a failure.** This is
    /// not politeness — it is the only correct behaviour, because watching
    /// genuinely does not work everywhere. An SMB or NFS share delivers no
    /// change notifications to a client at all. A FUSE filesystem may or may
    /// not, depending on the driver. Linux caps inotify watches per user, and a
    /// recursive watch over a large Steam library can exhaust that cap on a
    /// machine that is already watching something else. In every one of those
    /// cases the product still works: the refresh becomes something the user
    /// asks for rather than something that happens on its own, which is a
    /// smaller product, not a broken one. Ask [`yaamal`](Self::yaamal) whether
    /// anything is being watched, and fall back to on-demand scanning when it
    /// says no.
    ///
    /// `tabteea` is clamped into a quarter of a second to a minute, so a
    /// configuration value of zero cannot produce a watcher that refuses to
    /// start. [`TABTEEA_MUQTARAHA`] explains what to pass and why.
    ///
    /// # Errors
    ///
    /// [`KhataKashf::TaadhurMuraqaba`] only when the debouncer itself cannot be
    /// created — the platform's notification mechanism is unavailable or its
    /// per-process instance limit is exhausted. That is the one case where no
    /// root could ever be watched and the caller should stop trying.
    pub fn jadeed(judhur: Vec<PathBuf>, tabteea: Duration) -> Natija<Self> {
        let tabteea = tabteea.clamp(TABTEEA_DUNYA, TABTEEA_ULYA);
        let (mursil, mustaqbil) = channel::<DebounceEventResult>();

        let mut murattib =
            new_debouncer(tabteea, None, mursil).map_err(|khata| KhataKashf::TaadhurMuraqaba {
                masar: judhur.first().cloned().unwrap_or_default(),
                tafsil: khata.to_string(),
            })?;

        let mut muraqaba: Vec<PathBuf> = Vec::new();
        let mut tanbihat: Vec<TanbihFahs> = Vec::new();

        for jidhr in rattib_judhur(judhur) {
            if !jidhr.is_dir() {
                tanbihat.push(tanbih(&jidhr, "the folder is not there to watch"));
                continue;
            }
            match murattib.watch(jidhr.as_path(), RecursiveMode::Recursive) {
                Ok(()) => muraqaba.push(jidhr),
                Err(khata) => tanbihat.push(tanbih(&jidhr, &khata.to_string())),
            }
        }

        if muraqaba.is_empty() {
            tracing::info!(
                marfuda = tanbihat.len(),
                "no library folder could be watched; refresh is on demand"
            );
        }

        Ok(Self {
            murattib,
            mustaqbil: Mutex::new(mustaqbil),
            judhur: muraqaba,
            tanbihat,
            tabteea,
        })
    }

    /// Takes everything that has accumulated since the last call, without
    /// blocking.
    ///
    /// Returns the watched roots that were touched rather than the individual
    /// files, because that is what a rescan needs and because the collapse is
    /// enormous: a game update that produced four hundred thousand events
    /// becomes one entry naming one library folder. An event under no known
    /// root — which is what a rename *out* of a watched tree looks like on some
    /// backends — falls back to the changed path's parent so that it is not
    /// silently dropped.
    ///
    /// [`None`] means nothing has happened. It also means, harmlessly, that
    /// another caller is draining the queue at this instant: the events stay in
    /// the channel and arrive on the next call, so nothing is lost. Polling
    /// from one loop, which is what this is for, never sees that case.
    ///
    /// Errors reported by the platform watcher after startup — a watched
    /// directory deleted underneath it, an event queue overflow — are logged
    /// and skipped. There is nothing a caller can do about them except rescan,
    /// which is what it is about to do anyway.
    #[must_use]
    pub fn istaqbil(&self) -> Option<Vec<PathBuf>> {
        let mustaqbil = self.mustaqbil.try_lock().ok()?;
        let mut mutaathira: Vec<PathBuf> = Vec::new();

        loop {
            match mustaqbil.try_recv() {
                Ok(Ok(ahdath)) => {
                    for hadath in ahdath {
                        for masar in &hadath.event.paths {
                            let jidhr = self
                                .judhur
                                .iter()
                                .find(|jidhr| masar.starts_with(jidhr))
                                .cloned()
                                .unwrap_or_else(|| {
                                    masar
                                        .parent()
                                        .map_or_else(|| masar.clone(), Path::to_path_buf)
                                });
                            if !mutaathira.contains(&jidhr) {
                                mutaathira.push(jidhr);
                            }
                        }
                    }
                },
                Ok(Err(akhta)) => {
                    for khata in akhta {
                        tracing::warn!(sabab = %khata, "the filesystem watcher reported an error");
                    }
                },
                // Disconnected means the debouncer thread has stopped, which
                // only happens once this watcher is being torn down.
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        (!mutaathira.is_empty()).then_some(mutaathira)
    }

    /// Stops watching and waits for the debouncer's thread to finish.
    ///
    /// Dropping a [`Muraqib`] stops it too; this exists for the caller that
    /// needs the thread to be gone before it does something else — unmounting
    /// the drive it was watching, or replacing this watcher with one over a
    /// different set of roots. It can block for up to a quarter of the debounce
    /// interval, which is the debouncer's own tick.
    pub fn awqif(self) {
        let Self { murattib, .. } = self;
        murattib.stop();
    }

    /// The roots actually being watched.
    #[must_use]
    pub fn judhur(&self) -> &[PathBuf] {
        &self.judhur
    }

    /// Every root that was asked for and could not be watched, ready for the
    /// diagnostics screen.
    #[must_use]
    pub fn tanbihat(&self) -> &[TanbihFahs] {
        &self.tanbihat
    }

    /// Whether anything at all is being watched.
    ///
    /// `false` is the signal to fall back to scanning on demand, and to stop
    /// telling the user the library updates itself.
    #[must_use]
    pub const fn yaamal(&self) -> bool {
        !self.judhur.is_empty()
    }

    /// The debounce interval in force, after clamping.
    #[must_use]
    pub const fn tabteea(&self) -> Duration {
        self.tabteea
    }
}

/// Deduplicates roots and removes any that a recursive watch already covers.
///
/// Sorting lexicographically puts a parent immediately before its children, so
/// one pass keeping only paths that do not begin with something already kept is
/// enough. Watching both `steamapps` and `steamapps/common` would double every
/// event under the second, and on Windows would consume two handles where one
/// does the work.
fn rattib_judhur(mut judhur: Vec<PathBuf>) -> Vec<PathBuf> {
    judhur.sort();
    judhur.dedup();

    let mut mustaqirra: Vec<PathBuf> = Vec::with_capacity(judhur.len());
    for jidhr in judhur {
        if mustaqirra.iter().any(|sabiq| jidhr.starts_with(sabiq)) {
            continue;
        }
        mustaqirra.push(jidhr);
    }
    mustaqirra
}

/// Builds the warning a root that could not be watched produces.
///
/// Carries the English sentence from [`KhataKashf::TaadhurMuraqaba`] rather
/// than a message invented here, so that the diagnostics screen, the log and
/// the error catalogue all say the same thing about the same failure.
fn tanbih(masar: &Path, tafsil: &str) -> TanbihFahs {
    let khata = Khata::min_tafsir(&KhataKashf::TaadhurMuraqaba {
        masar: masar.to_path_buf(),
        tafsil: tafsil.to_owned(),
    });
    tracing::warn!(masar = %masar.display(), sabab = tafsil, "library folder is not being watched");
    TanbihFahs::jadeed(MASDAR_TANBIH, masar.display().to_string(), khata.injilizi)
}
