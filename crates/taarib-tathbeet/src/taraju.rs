//! التراجع — uninstall: put every original back, prove each one came back, and
//! never claim more than that.
//!
//! This module is the promise the rest of the product is allowed to make. Taarib
//! asks people to let it modify files inside games they paid for, sometimes games
//! bought once on a storefront that no longer sells them, and the only thing that
//! makes that a reasonable request is that they can always get back to exactly
//! where they started — and be *told so from a verification rather than from an
//! assumption*.
//!
//! ## Text and voice are never coupled
//!
//! [`istiada_nass`] and [`istiada_sawt`] are separate operations over separate
//! manifests in separate directories. Removing the Arabic text leaves the Arabic
//! dubbing exactly as it was, and removing the dubbing leaves the text exactly as
//! it was. This is not a convention: the manifest a restore opens is named by the
//! kind it was asked for ([`crate::bayan::NawTathbeet::ism_bayan`]), so the code
//! path removing text has no expression in it that names the voice manifest, and
//! the loaded manifest is then re-checked against the kind that was requested so
//! a file copied or renamed by hand cannot cross them either.
//!
//! [`istiada_kul`] removes both, and does so by running the two independently and
//! collecting *both* outcomes. It does not stop at the first failure, because a
//! text patch that will not come off is not a reason to leave a voice patch
//! installed that would have come off cleanly.
//!
//! ## A restore is not finished when the bytes match
//!
//! Every restored file goes through the same five steps in the same order, and
//! the order is not interchangeable:
//!
//! 1. the backup is expanded and hashed against what was recorded, *before* a
//!    byte of the game is touched — a corrupt backup must be discovered while the
//!    patched file is still intact, not after it has been overwritten;
//! 2. the original is written atomically over the game file;
//! 3. what actually landed on disk is re-hashed and compared;
//! 4. the recorded timestamps are applied;
//! 5. the recorded permissions are applied.
//!
//! Content first, because writing sets the modification time — stamping before
//! writing would immediately overwrite the stamp. Timestamps before permissions,
//! because setting a file's times needs a writable handle on Windows, and a file
//! whose recorded read-only attribute has already gone back on cannot be opened
//! for writing; the reverse order is safe because neither `chmod` nor setting the
//! read-only attribute disturbs a file's modification or access time. A file
//! restored in any other order comes back stamped with the moment of the
//! uninstall, which makes a launcher's own integrity check believe the entire
//! game was rewritten and start a multi-gigabyte re-download.
//!
//! And a file restored with the right bytes and the wrong mode is not restored.
//! A Linux game's player binary without its execute bit does not start. That is
//! why step 4 raising [`KhataTathbeet::SalahiyatGhayrMustaada`] fails the run
//! instead of being logged and shrugged at.
//!
//! ## Resumable, and honest about being incomplete
//!
//! Each record is marked done in the manifest, and the manifest is flushed,
//! before the next record is started. One small atomic write per file is the
//! right trade for a few hundred files: the alternative is a single flush at the
//! end, and a single flush at the end is precisely the design that cannot be
//! resumed. A crash, a full disk or a closed laptop halfway through leaves a
//! manifest that says exactly which paths are already back, and running the
//! uninstall again finishes the job instead of starting it over.
//!
//! When a run cannot finish, it returns [`KhataTathbeet::IstiadaNaqisa`] carrying
//! how many paths completed and **the list of every path that has not**. There is
//! no success value in this module that means "most of it worked". A user told
//! their game is clean while three files still hold Taarib's bytes has been
//! misled about the one thing this crate exists to be truthful about.
//!
//! ## What the store already undid
//!
//! A game that was updated after being patched has files the store rewrote with
//! its own newer versions. Those files no longer contain Taarib's bytes and no
//! longer match the original Taarib preserved either. Writing the old backup over
//! them would silently downgrade files the launcher had just updated.
//!
//! Under the default [`SiyasatIstiada::Muhafiza`] such a file is **left exactly as
//! it is and reported by name**. That is a complete removal of Taarib's change —
//! there is nothing of Taarib's left in the file to remove — and it is the
//! outcome a user wants. [`SiyasatIstiada::Sarima`] refuses instead, with
//! [`KhataTathbeet::MalafMustabdal`] naming the file, for a caller that needs an
//! exact revert and would rather be stopped than surprised.
//!
//! There is deliberately no third policy that forces the old original back over a
//! store's newer file. A user who wants the pre-update build back gets it from
//! the launcher, which has the real thing, not from a backup Taarib took of a
//! build that is no longer the one they own.
//!
//! ## Additions come off, whatever is in them now
//!
//! A file recorded as an addition exists only because Taarib created it, and
//! removing it is the uninstall. It is deleted even if its contents have changed
//! since — because the alternative is leaving a modified copy of Taarib's own
//! file behind after the manifest that describes it has been consumed, which no
//! later run would ever clean up. A translator who edited an added file is warned
//! by [`crate::tahaqquq`] *before* the uninstall, which is the point at which the
//! warning is still useful.
//!
//! Directories Taarib created come off last, deepest first, and only when empty
//! and only when recorded. Somebody else's files in `BepInEx/plugins/` are
//! somebody else's.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::bina::Basma;
use taarib_usus::khata::Tafsir as _;
use taarib_usus::masarat;

use crate::bayan::{
    BayanTathbeet, NawTaghyeer, NawTathbeet, SalahiyatMalaf, SijillIdad, SijillTaghyeer,
    Tathbeet, basma_bayt, basma_malaf, dakhil_aw_khata,
};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io};

/// How a restore treats a file that is neither Taarib's work nor the original.
///
/// The only meaningful choice this module offers a caller, and it exists because
/// the two reasonable answers serve different people. Everything else about a
/// restore is fixed, because everything else has one correct answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum SiyasatIstiada {
    /// Leave a replaced file alone and report it. The default.
    ///
    /// Taarib's bytes are already gone from such a file; there is nothing left
    /// for an uninstall to remove, and writing an older original over it would
    /// undo the store's update rather than Taarib's patch. The uninstall
    /// completes and the report names every file this happened to.
    #[default]
    Muhafiza,

    /// Refuse, naming the file.
    ///
    /// For a caller that needs the game returned to exactly its pre-patch state
    /// and would rather be stopped than told afterwards that four files were
    /// left as the store had rewritten them.
    Sarima,
}

impl SiyasatIstiada {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Muhafiza => "leave replaced files alone",
            Self::Sarima => "refuse on a replaced file",
        }
    }
}

/// The way a setting Taarib changed is put back.
///
/// The counterpart of [`crate::bayan::Muthabbit`] for things that are not files,
/// and it is a trait for the same reason: this crate must not guess. Restoring a
/// launch option means editing Steam's `localconfig.vdf`, restoring a registry
/// value means opening a hive, restoring a Wine prefix key means writing
/// `user.reg` in the encoding Wine expects. Each of those belongs to the
/// component that owns that format, and a version of this module that
/// half-implemented three of them would be three new ways to corrupt a
/// launcher's configuration while uninstalling a patch.
///
/// So `taraju` drives the restore and the caller supplies the writer. What
/// `taraju` guarantees is the part it can: the previous value was recorded and
/// flushed before the setting was ever changed, the record survives an
/// interrupted uninstall, and a setting that could not be put back is a failure
/// of the whole run rather than a line in a log.
pub trait RadIdad {
    /// Puts one setting back to the value recorded in `sijill`.
    ///
    /// [`SijillIdad::yuhdhaf`] says which of the two operations this is: a
    /// setting with no previous value must be *removed*, not written as an empty
    /// string. An implementation that cannot tell the difference will leave a
    /// registry value the game never had, and several launchers treat an
    /// environment variable that is set-and-empty as a different thing from one
    /// that is unset.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::IdadGhayrMustaad`] naming the setting, where it lives and
    /// why it could not be put back. Anything else the implementation raises is
    /// passed through unchanged.
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet>;
}

/// A restorer for installations that changed no setting, which refuses if asked.
///
/// The overwhelmingly common case: a text patch writes files and touches nothing
/// else, so its manifest has an empty settings map and this is never called. It
/// refuses rather than silently succeeding, because a no-op that reports success
/// for a setting it did not restore is exactly the "partial restore reported as
/// complete" this crate refuses to produce anywhere else. The refusal names the
/// component that should have been passed instead.
#[derive(Debug, Clone, Copy, Default)]
pub struct RadLaShay;

impl RadIdad for RadLaShay {
    fn rudd(&mut self, sijill: &SijillIdad) -> Result<(), KhataTathbeet> {
        Err(KhataTathbeet::IdadGhayrMustaad {
            muarrif: sijill.muarrif.clone(),
            mahall: sijill.mahall.wasf(),
            sabab: format!(
                "this uninstall was given no way to write settings back, and this \
                 installation changed one. Pass a restorer from {} instead.",
                sijill.mahall.masul()
            ),
        })
    }
}

/// What one install's restore did.
///
/// Every field counts something that finished. There is no "partly done" field,
/// because a partly done restore is [`KhataTathbeet::IstiadaNaqisa`] and never
/// reaches this type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerIstiada {
    /// Which installation was removed.
    pub naw: NawTathbeet,

    /// The game, for a report a person reads.
    pub luba: String,

    /// Originals written back and verified against their recorded fingerprint.
    pub mustaada: usize,

    /// Paths that were already back to their original bytes when the run
    /// reached them.
    ///
    /// A store that reinstalled the file it ships produces this, and so does a
    /// previous run of this same uninstall that completed the write and was
    /// interrupted before it could mark the line. Counted apart from
    /// [`TaqreerIstiada::mustaada`] because "restored 214 files" and "restored
    /// 200 files, 14 were already fine" are different reports.
    pub kanat_asliya: usize,

    /// Files Taarib had added, deleted.
    pub mahdhufa: usize,

    /// Directories Taarib had created, removed because they were empty.
    pub mujalladat_muzala: usize,

    /// Directories left in place because they still hold somebody else's files.
    pub mujalladat_matruka: usize,

    /// Settings put back to their previous values.
    pub idadat_mustaada: usize,

    /// Paths the store replaced with its own newer versions, left as they are.
    ///
    /// Named rather than counted, because "four files were left alone" is not
    /// something a user can check and "these four files were left alone" is.
    pub mustabdala: Vec<String>,
}

impl TaqreerIstiada {
    /// An empty report for one installation.
    #[must_use]
    pub fn jadeed(naw: NawTathbeet, luba: impl Into<String>) -> Self {
        Self {
            naw,
            luba: luba.into(),
            mustaada: 0,
            kanat_asliya: 0,
            mahdhufa: 0,
            mujalladat_muzala: 0,
            mujalladat_matruka: 0,
            idadat_mustaada: 0,
            mustabdala: Vec::new(),
        }
    }

    /// How many records were dealt with in total.
    #[must_use]
    pub const fn majmu(&self) -> usize {
        self.mustaada
            .saturating_add(self.kanat_asliya)
            .saturating_add(self.mahdhufa)
            .saturating_add(self.mujalladat_muzala)
            .saturating_add(self.mujalladat_matruka)
            .saturating_add(self.idadat_mustaada)
    }

    /// Whether the game is now exactly as the store shipped it, with nothing
    /// left over.
    ///
    /// False when a directory had to be left in place or a file had already been
    /// replaced by the store — both of which are successful outcomes and neither
    /// of which is "the game is byte-for-byte what it was before the patch".
    /// Distinguishing them is what lets a caller say "removed" for one and
    /// "removed, with notes" for the other instead of overstating both.
    #[must_use]
    pub const fn nazif(&self) -> bool {
        self.mujalladat_matruka == 0 && self.mustabdala.is_empty()
    }

    /// The result as lines for the log, the bundle and the game's detail screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "{} [{}]: restored {} original(s), deleted {} added file(s), removed {} \
             director(ies), restored {} setting(s)",
            self.luba,
            self.naw.ism(),
            self.mustaada,
            self.mahdhufa,
            self.mujalladat_muzala,
            self.idadat_mustaada
        )];
        if self.kanat_asliya > 0 {
            sutur.push(format!(
                "  {} path(s) were already back to their original bytes and were left alone",
                self.kanat_asliya
            ));
        }
        if self.mujalladat_matruka > 0 {
            sutur.push(format!(
                "  {} director(ies) left in place because they still hold files Taarib did \
                 not put there",
                self.mujalladat_matruka
            ));
        }
        for masar in &self.mustabdala {
            sutur.push(format!(
                "  {masar}: replaced by the store since Taarib wrote it, left as it is"
            ));
        }
        sutur
    }
}

/// Both installations' outcomes, gathered independently.
///
/// Returned by [`istiada_kul`] rather than a `Result`, because a `Result` would
/// force one of the two to be discarded. The text patch failing and the voice
/// patch coming off cleanly is a real and common state, and the user is entitled
/// to be told both halves of it.
#[derive(Debug)]
pub struct TaqreerKul {
    /// The text installation's outcome, or [`None`] when there was none.
    pub nass: Option<NatijatTathbeet<TaqreerIstiada>>,

    /// The voice installation's outcome, or [`None`] when there was none.
    pub sawt: Option<NatijatTathbeet<TaqreerIstiada>>,
}

impl TaqreerKul {
    /// Whether every installation that was present came off.
    #[must_use]
    pub fn najahat(&self) -> bool {
        let salim = |natija: &Option<NatijatTathbeet<TaqreerIstiada>>| {
            natija.as_ref().is_none_or(Result::is_ok)
        };
        salim(&self.nass) && salim(&self.sawt)
    }

    /// Whether anything at all was installed.
    #[must_use]
    pub const fn wujidat(&self) -> bool {
        self.nass.is_some() || self.sawt.is_some()
    }

    /// Every failure, in kind order.
    ///
    /// Both, not the first: reporting only the text failure would hide that the
    /// voice patch also refused to come off, and the user would fix one problem
    /// and meet the other.
    #[must_use]
    pub fn akhta(&self) -> Vec<&KhataTathbeet> {
        let mut akhta = Vec::new();
        if let Some(Err(khata)) = &self.nass {
            akhta.push(khata);
        }
        if let Some(Err(khata)) = &self.sawt {
            akhta.push(khata);
        }
        akhta
    }

    /// The lines a caller shows for this game.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::new();
        for (naw, natija) in
            [(NawTathbeet::Nass, &self.nass), (NawTathbeet::Sawt, &self.sawt)]
        {
            match natija {
                None => sutur.push(format!("  no {} installation", naw.ism())),
                Some(Ok(taqreer)) => sutur.extend(taqreer.taqreer()),
                Some(Err(khata)) => {
                    sutur.push(format!("  {} FAILED: {}", naw.ism(), khata.injilizi()));
                }
            }
        }
        sutur
    }
}

/// Where one game's installation lives, for the library-wide sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MawqiTathbeet {
    /// The game's display name, for the report.
    pub ism: String,
    /// The game's install directory as it is *now*, not as the manifest recorded
    /// it — a game moved between installing and uninstalling is still removable.
    pub jidhr_luba: PathBuf,
    /// The game's directory under `nusakh/`, holding both kinds' manifests.
    pub jidhr_nusakh: PathBuf,
}

/// One game's place in a library-wide restore.
#[derive(Debug)]
pub struct NatijatLuba {
    /// The game.
    pub ism: String,
    /// Where it is.
    pub jidhr_luba: PathBuf,
    /// What happened to each of its installations.
    pub kul: TaqreerKul,
}

/// The whole library sweep.
///
/// Every game is attempted and every outcome is kept. The sweep never aborts on
/// a failure, because the failure modes here are per-game — one locked
/// executable, one deleted backup directory — and stopping the sweep at the first
/// one would leave a user who asked to remove Taarib from their library with most
/// of it still installed and no report saying which.
#[derive(Debug)]
pub struct TaqreerMaktaba {
    /// One entry per game, in the order they were given.
    pub alaab: Vec<NatijatLuba>,
}

impl TaqreerMaktaba {
    /// How many games came off completely.
    #[must_use]
    pub fn adad_najah(&self) -> usize {
        self.alaab.iter().filter(|luba| luba.kul.najahat()).count()
    }

    /// How many games had at least one installation refuse to come off.
    #[must_use]
    pub fn adad_fashal(&self) -> usize {
        self.alaab.iter().filter(|luba| !luba.kul.najahat()).count()
    }

    /// How many games had nothing installed.
    #[must_use]
    pub fn adad_faragh(&self) -> usize {
        self.alaab.iter().filter(|luba| !luba.kul.wujidat()).count()
    }

    /// Whether every game came off.
    #[must_use]
    pub fn najahat(&self) -> bool {
        self.alaab.iter().all(|luba| luba.kul.najahat())
    }

    /// The whole sweep as lines, failures included and named.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "{} game(s) swept: {} clean, {} failed, {} had nothing installed",
            self.alaab.len(),
            self.adad_najah(),
            self.adad_fashal(),
            self.adad_faragh()
        )];
        for luba in &self.alaab {
            sutur.push(format!("{} ({})", luba.ism, luba.jidhr_luba.display()));
            sutur.extend(luba.kul.taqreer());
        }
        sutur
    }
}

// ---------------------------------------------------------------------------
// The three operations
// ---------------------------------------------------------------------------

/// Removes the **text** installation and nothing else.
///
/// The voice installation, its manifest, its backup directory and every file it
/// touched are not read, not opened, and not named anywhere on this path.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] when the text manifest is missing, unreadable,
/// inconsistent, or is the voice manifest under the text manifest's name, and
/// [`KhataTathbeet::IstiadaNaqisa`] when the restore starts and cannot finish —
/// carrying what completed and every path that did not, so a second run resumes
/// rather than restarts.
pub fn istiada_nass(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
) -> NatijatTathbeet<TaqreerIstiada> {
    istiada_naw(jidhr_luba, jidhr_nusakh, NawTathbeet::Nass, siyasa, radd)
}

/// Removes the **voice** installation and nothing else.
///
/// The mirror of [`istiada_nass`], and deliberately a separate function rather
/// than the same function with a flag: two entry points cannot be given the wrong
/// argument, and a caller reading `istiada_sawt` at the call site can see which
/// one it is without following a variable.
///
/// # Errors
///
/// As [`istiada_nass`], for the voice manifest.
pub fn istiada_sawt(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
) -> NatijatTathbeet<TaqreerIstiada> {
    istiada_naw(jidhr_luba, jidhr_nusakh, NawTathbeet::Sawt, siyasa, radd)
}

/// Removes both installations, independently.
///
/// Runs text, then voice, and keeps both outcomes whatever either of them did.
/// A failure in one is never allowed to skip the other: they are separate
/// installations of separate content with separate backups, and a locked
/// `data.win` has no bearing on whether a directory of dubbed audio can be
/// deleted.
///
/// An installation that is not present is [`None`] rather than an error, so
/// calling this on a game with only subtitles installed succeeds and says so.
#[must_use]
pub fn istiada_kul(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
) -> TaqreerKul {
    let nass = Tathbeet::mawjud(jidhr_nusakh, NawTathbeet::Nass)
        .then(|| istiada_nass(jidhr_luba, jidhr_nusakh, siyasa, radd));
    let sawt = Tathbeet::mawjud(jidhr_nusakh, NawTathbeet::Sawt)
        .then(|| istiada_sawt(jidhr_luba, jidhr_nusakh, siyasa, radd));
    TaqreerKul { nass, sawt }
}

/// Removes Taarib from every game it was given, reporting each one.
///
/// Continues past a failure rather than aborting the sweep. The failures here are
/// per-game and independent — one game's executable is running, one game's backup
/// directory was deleted by a disk cleaner, one game was moved to a drive that is
/// not mounted — and a sweep that stopped at the first would leave a user who
/// asked to remove Taarib from their library with most of it still installed and
/// nothing telling them which parts.
///
/// Every failure is preserved in the returned report, in full, with its code, its
/// path and its remedy. Nothing is collapsed into a count.
#[must_use]
pub fn istiada_al_maktaba(
    mawaqi: &[MawqiTathbeet],
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
) -> TaqreerMaktaba {
    let mut alaab = Vec::with_capacity(mawaqi.len());
    for mawqi in mawaqi {
        let kul = istiada_kul(&mawqi.jidhr_luba, &mawqi.jidhr_nusakh, siyasa, radd);
        alaab.push(NatijatLuba {
            ism: mawqi.ism.clone(),
            jidhr_luba: mawqi.jidhr_luba.clone(),
            kul,
        });
    }
    TaqreerMaktaba { alaab }
}

/// The one body behind [`istiada_nass`] and [`istiada_sawt`].
///
/// Takes the kind as a value and never derives it from anything on disk, so the
/// manifest that gets opened is decided entirely by which of the two public
/// entry points was called.
fn istiada_naw(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
) -> NatijatTathbeet<TaqreerIstiada> {
    let mut tathbeet = Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw)?;
    let ism_luba = tathbeet.bayan().ism_luba.clone();
    let mut taqreer = TaqreerIstiada::jadeed(naw, ism_luba.clone());

    // Bound before the match rather than matched on directly, so the borrow of
    // `taqreer` the call takes is unambiguously over before the arms read it.
    let natija = nafidh(&mut tathbeet, siyasa, radd, &mut taqreer);
    match natija {
        Ok(()) => Ok(taqreer),
        Err(sabab) => Err(KhataTathbeet::IstiadaNaqisa {
            luba: format!("{ism_luba} [{}]", naw.ism()),
            munjaz: taqreer.majmu(),
            mutabaqqi: tathbeet.bayan().qaimat_mutabaqqi(),
            sabab: Box::new(sabab),
        }),
    }
}

/// Walks the manifest in the one order that is safe, stopping at the first
/// refusal.
///
/// Files, then settings, then directories. Files first because a directory
/// cannot be empty until the files inside it are gone. Settings before
/// directories only because a setting failing is the failure a user is most
/// likely to be able to fix immediately, and stopping there leaves the fewest
/// half-done things behind. Directories last, deepest first, so a nested
/// directory is empty by the time its parent is considered.
fn nafidh(
    tathbeet: &mut Tathbeet,
    siyasa: SiyasatIstiada,
    radd: &mut dyn RadIdad,
    taqreer: &mut TaqreerIstiada,
) -> Result<(), KhataTathbeet> {
    let malaffat: Vec<String> = tathbeet
        .bayan()
        .sijillat
        .iter()
        .filter(|(_, sijill)| {
            !sijill.istiada_tammat && !matches!(sijill.naw, NawTaghyeer::MujalladMudaf)
        })
        .map(|(masar, _)| masar.clone())
        .collect();
    for masar in malaffat {
        istiada_wahid(tathbeet, &masar, siyasa, taqreer)?;
    }

    let idadat: Vec<String> = tathbeet
        .bayan()
        .idadat
        .iter()
        .filter(|(_, idad)| !idad.istiada_tammat)
        .map(|(muarrif, _)| muarrif.clone())
        .collect();
    for muarrif in idadat {
        rudd_idad(tathbeet, &muarrif, radd, taqreer)?;
    }

    // Descending path order puts `BepInEx/plugins` before `BepInEx`, so the
    // child is gone by the time the parent is asked whether it is empty.
    let mut mujalladat: Vec<String> = tathbeet
        .bayan()
        .sijillat
        .iter()
        .filter(|(_, sijill)| {
            !sijill.istiada_tammat && matches!(sijill.naw, NawTaghyeer::MujalladMudaf)
        })
        .map(|(masar, _)| masar.clone())
        .collect();
    mujalladat.sort_by(|awwal, thani| thani.cmp(awwal));
    for masar in mujalladat {
        azil_mujallad(tathbeet, &masar, taqreer)?;
    }

    Ok(())
}

/// What the file at a recorded path currently is.
///
/// Computed before anything is written, because every one of the four answers
/// leads somewhere different and three of them are not "write the backup over
/// it".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HalatQablIstiada {
    /// Still the bytes Taarib wrote. The ordinary case.
    Maktub,
    /// Already the original. Nothing to do.
    Asli,
    /// Nothing is there.
    Mafqud,
    /// Something else entirely — the store replaced it, or another tool did.
    ///
    /// Carries the fingerprint that was computed to reach this conclusion, so
    /// the caller that reports it does not hash a multi-gigabyte asset archive a
    /// second time to say what it already knows.
    Mustabdal(Basma),
}

/// Undoes one file and records that it is done before returning.
fn istiada_wahid(
    tathbeet: &mut Tathbeet,
    masar: &str,
    siyasa: SiyasatIstiada,
    taqreer: &mut TaqreerIstiada,
) -> Result<(), KhataTathbeet> {
    let Some(sijill) = tathbeet.bayan().sijill(masar).cloned() else { return Ok(()) };
    if sijill.istiada_tammat {
        return Ok(());
    }
    let mutlaq = dakhil_aw_khata(tathbeet.jidhr_luba(), masar)?;

    match sijill.naw {
        NawTaghyeer::Tadeel => {
            istiada_muaddal(tathbeet, &sijill, &mutlaq, siyasa, taqreer)?;
        }
        NawTaghyeer::Idafa => {
            // Deleted whatever its contents are now. See this module's header:
            // an addition exists only because Taarib created it, and leaving a
            // modified copy of Taarib's own file behind after the manifest is
            // consumed is litter no later run can clean up.
            if mutlaq.exists() {
                let _ = SalahiyatMalaf::ataih_kitaba(&mutlaq)?;
                fs::remove_file(&mutlaq).map_err(|sabab| {
                    min_khata_io(&mutlaq, "deleting a file Taarib added", sabab)
                })?;
            }
            taqreer.mahdhufa = taqreer.mahdhufa.saturating_add(1);
        }
        NawTaghyeer::MujalladMudaf => return Ok(()),
    }

    tathbeet.allim_tammat(masar)
}

/// Puts one preserved original back, and proves it came back.
fn istiada_muaddal(
    tathbeet: &Tathbeet,
    sijill: &SijillTaghyeer,
    mutlaq: &Path,
    siyasa: SiyasatIstiada,
    taqreer: &mut TaqreerIstiada,
) -> Result<(), KhataTathbeet> {
    let (Some(miftah), Some(muallana)) = (sijill.nuskha.as_deref(), sijill.basma_asliya) else {
        return Err(KhataTathbeet::BayanTalif {
            masar: tathbeet.masar_bayan().to_path_buf(),
            sabab: format!(
                "{} is recorded as modified and names no backup or no original fingerprint, \
                 so there is nothing to put back and nothing to check it against",
                sijill.masar
            ),
        });
    };

    // The backup is expanded and checked first, while the patched file is still
    // intact. Discovering a corrupt backup after overwriting the game file would
    // leave the user with neither version.
    let asli = tathbeet.iqra_nuskha(miftah, mutlaq)?;
    let basma_nuskha = basma_bayt(&asli);
    if basma_nuskha != muallana {
        return Err(KhataTathbeet::NuskhaTalifa {
            masar: mutlaq.to_path_buf(),
            miftah: miftah.to_owned(),
            muallana: muallana.to_string(),
            mahsuba: basma_nuskha.to_string(),
        });
    }

    match hala_qabl(sijill, mutlaq, muallana)? {
        HalatQablIstiada::Asli => {
            // Already back. Its permissions and times are whatever put it back
            // gave it, and overwriting them would be a change rather than a
            // restore.
            taqreer.kanat_asliya = taqreer.kanat_asliya.saturating_add(1);
            return Ok(());
        }
        HalatQablIstiada::Mustabdal(mahsuba) => {
            if matches!(siyasa, SiyasatIstiada::Sarima) {
                return Err(KhataTathbeet::MalafMustabdal {
                    masar: mutlaq.to_path_buf(),
                    muallana: sijill
                        .basma_maktuba
                        .map_or_else(|| muallana.to_string(), |b| b.to_string()),
                    mahsuba: mahsuba.to_string(),
                });
            }
            taqreer.mustabdala.push(sijill.masar.clone());
            return Ok(());
        }
        HalatQablIstiada::Maktub | HalatQablIstiada::Mafqud => {}
    }

    // A read-only target is cleared before the rename, not after: Windows
    // refuses to replace one. The attribute that goes back on is the recorded
    // one, applied below, so this cannot leak into the restored state.
    let _ = SalahiyatMalaf::ataih_kitaba(mutlaq)?;

    // The destination is re-derived with links resolved at the moment of the
    // write, exactly as the install's own write does. A restore is a write
    // into the game directory driven by a stored document, and it gets no
    // weaker gate than the install that stored it.
    let mutlaq_haqiqi = crate::bayan::mutlaq_lil_kitaba(tathbeet.jidhr_luba(), &sijill.masar)?;
    masarat::kitaba_dharra(&mutlaq_haqiqi, &asli).map_err(|khata| KhataTathbeet::KhataMalaf {
        masar: mutlaq.to_path_buf(),
        amal: "writing the original back",
        sabab: std::io::Error::other(khata.injilizi),
    })?;

    // Re-read what actually landed, rather than trusting the write — through
    // the same resolved path the write went to, so the verification and the
    // write cannot be talking about two different files.
    let mahsuba = basma_malaf(&mutlaq_haqiqi)?;
    if mahsuba != muallana {
        return Err(KhataTathbeet::IstiadaGhayrMutabaqa {
            masar: mutlaq.to_path_buf(),
            muallana: muallana.to_string(),
            mahsuba: mahsuba.to_string(),
        });
    }

    // Content, then times, then permissions. See this module's header for why
    // the order is fixed and why the last two are not the other way round.
    if let Some(awqat) = sijill.awqat_asliya {
        awqat.tatbeeq(&mutlaq_haqiqi)?;
    }
    if let Some(salahiyat) = sijill.salahiyat_asliya {
        salahiyat.tatbeeq(&mutlaq_haqiqi)?;
    }

    taqreer.mustaada = taqreer.mustaada.saturating_add(1);
    Ok(())
}

/// Decides what is at a path without hashing it unless it has to.
///
/// The size-and-time comparison answers "is this still exactly what Taarib
/// wrote?" for the overwhelming majority of files, and only the ones where it
/// disagrees are hashed. On a patch touching several hundred files that is the
/// difference between an uninstall that starts immediately and one that reads
/// every byte of the game twice.
fn hala_qabl(
    sijill: &SijillTaghyeer,
    mutlaq: &Path,
    muallana: Basma,
) -> Result<HalatQablIstiada, KhataTathbeet> {
    let Ok(bayanat) = fs::metadata(mutlaq) else {
        return Ok(HalatQablIstiada::Mafqud);
    };
    if !bayanat.is_file() {
        // A directory or a device node where a regular file was recorded. It is
        // not Taarib's and it is not the original, and writing over it would
        // mean deciding what to do with whatever is inside it.
        return Ok(HalatQablIstiada::Mustabdal(muallana));
    }
    if sijill.yutabiq_bila_basma(&bayanat) {
        return Ok(HalatQablIstiada::Maktub);
    }

    let mahsuba = basma_malaf(mutlaq)?;
    if mahsuba == muallana {
        return Ok(HalatQablIstiada::Asli);
    }
    match sijill.basma_maktuba {
        // Preserved and never written through the guard. Writing the identical
        // original back over it is harmless and verifies, so it takes the
        // ordinary path rather than being reported as a replacement.
        None => Ok(HalatQablIstiada::Maktub),
        Some(maktuba) if maktuba == mahsuba => Ok(HalatQablIstiada::Maktub),
        Some(_) => Ok(HalatQablIstiada::Mustabdal(mahsuba)),
    }
}

/// Puts one setting back, through the caller's writer, and marks it done.
fn rudd_idad(
    tathbeet: &mut Tathbeet,
    muarrif: &str,
    radd: &mut dyn RadIdad,
    taqreer: &mut TaqreerIstiada,
) -> Result<(), KhataTathbeet> {
    let Some(idad) = tathbeet.bayan().idadat.get(muarrif).cloned() else { return Ok(()) };
    if idad.istiada_tammat {
        return Ok(());
    }
    radd.rudd(&idad)?;
    taqreer.idadat_mustaada = taqreer.idadat_mustaada.saturating_add(1);
    tathbeet.allim_idad_tammat(muarrif)
}

/// Removes one created directory, but only if nothing is left in it.
fn azil_mujallad(
    tathbeet: &mut Tathbeet,
    masar: &str,
    taqreer: &mut TaqreerIstiada,
) -> Result<(), KhataTathbeet> {
    let Some(naw) = tathbeet.bayan().sijill(masar).map(|sijill| sijill.naw) else {
        return Ok(());
    };
    if !matches!(naw, NawTaghyeer::MujalladMudaf) {
        return Ok(());
    }
    let mutlaq = dakhil_aw_khata(tathbeet.jidhr_luba(), masar)?;

    if !mutlaq.exists() {
        taqreer.mujalladat_muzala = taqreer.mujalladat_muzala.saturating_add(1);
        return tathbeet.allim_tammat(masar);
    }

    // `remove_dir` rather than `remove_dir_all`, precisely because it cannot
    // take a file with it: the "is it empty?" question is answered by the kernel
    // at the moment of removal, not by a listing this process took a moment
    // earlier and that another process may already have invalidated.
    match fs::remove_dir(&mutlaq) {
        Ok(()) => {
            taqreer.mujalladat_muzala = taqreer.mujalladat_muzala.saturating_add(1);
        }
        Err(sabab)
            if matches!(
                sabab.kind(),
                std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
            ) =>
        {
            // Somebody else's files are in here. Not a failure of the
            // uninstall, and deleting it anyway would destroy work Taarib did
            // not create.
            taqreer.mujalladat_matruka = taqreer.mujalladat_matruka.saturating_add(1);
        }
        Err(sabab) => {
            return Err(min_khata_io(&mutlaq, "removing a directory Taarib created", sabab));
        }
    }

    tathbeet.allim_tammat(masar)
}

// ---------------------------------------------------------------------------
// Reading a manifest without removing anything
// ---------------------------------------------------------------------------

/// What an uninstall *would* do, computed without writing a byte.
///
/// A confirmation screen's input. It opens the manifest, resolves every record
/// against what is on disk right now, and says how many files would be restored,
/// how many deleted, how many the store has already replaced, and how much space
/// the backups occupy — before a person is asked whether to proceed.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] or [`KhataTathbeet::IsdarBayanMajhul`] when the
/// manifest cannot be trusted, and [`KhataTathbeet::MasarKharij`] when it names a
/// path outside the game. Nothing else: a record whose file is missing or has
/// been replaced is part of the answer, not an error.
pub fn khutta(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
) -> NatijatTathbeet<KhuttatIstiada> {
    let tathbeet = Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw)?;
    let bayan = tathbeet.bayan();
    let mut khutta = KhuttatIstiada {
        naw,
        luba: bayan.ism_luba.clone(),
        jidhr_luba: jidhr_luba.to_path_buf(),
        waqt_tathbeet: bayan.waqt.clone(),
        li_istiada: 0,
        li_hadhf: 0,
        mujalladat: 0,
        mustabdala: Vec::new(),
        mafquda: Vec::new(),
        hajm_nusakh: bayan.hajm_nusakh(),
        mutabaqqi: bayan.mutabaqqi(),
    };

    for (masar, sijill) in &bayan.sijillat {
        if sijill.istiada_tammat {
            continue;
        }
        let mutlaq = dakhil_aw_khata(jidhr_luba, masar)?;
        match sijill.naw {
            NawTaghyeer::MujalladMudaf => {
                khutta.mujalladat = khutta.mujalladat.saturating_add(1);
            }
            NawTaghyeer::Idafa => {
                khutta.li_hadhf = khutta.li_hadhf.saturating_add(1);
                if !mutlaq.exists() {
                    khutta.mafquda.push(masar.clone());
                }
            }
            NawTaghyeer::Tadeel => {
                khutta.li_istiada = khutta.li_istiada.saturating_add(1);
                let Some(muallana) = sijill.basma_asliya else { continue };
                match hala_qabl(sijill, &mutlaq, muallana)? {
                    HalatQablIstiada::Mustabdal(_) => khutta.mustabdala.push(masar.clone()),
                    HalatQablIstiada::Mafqud => khutta.mafquda.push(masar.clone()),
                    HalatQablIstiada::Maktub | HalatQablIstiada::Asli => {}
                }
            }
        }
    }

    Ok(khutta)
}

/// The dry run: what removing one installation would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhuttatIstiada {
    /// Which installation.
    pub naw: NawTathbeet,
    /// The game.
    pub luba: String,
    /// Where it is now.
    pub jidhr_luba: PathBuf,
    /// When it was installed, RFC 3339.
    pub waqt_tathbeet: String,
    /// How many originals would be written back.
    pub li_istiada: usize,
    /// How many added files would be deleted.
    pub li_hadhf: usize,
    /// How many created directories would be considered for removal.
    pub mujalladat: usize,
    /// Paths the store has already replaced, which would be left alone.
    pub mustabdala: Vec<String>,
    /// Recorded paths that are not on disk at all.
    pub mafquda: Vec<String>,
    /// How many bytes the backups occupy, which uninstalling frees.
    pub hajm_nusakh: u64,
    /// How many records are still outstanding, including from an earlier run
    /// that did not finish.
    pub mutabaqqi: usize,
}

impl KhuttatIstiada {
    /// Whether removing this would leave the game byte-for-byte as it shipped.
    #[must_use]
    pub const fn nazif(&self) -> bool {
        self.mustabdala.is_empty() && self.mafquda.is_empty()
    }

    /// The plan as lines for the confirmation screen and the log.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![
            format!(
                "{} [{}] installed {}: {} file(s) to restore, {} to delete, {} \
                 director(ies), {} byte(s) of backups to free",
                self.luba,
                self.naw.ism(),
                self.waqt_tathbeet,
                self.li_istiada,
                self.li_hadhf,
                self.mujalladat,
                self.hajm_nusakh
            ),
        ];
        for masar in &self.mustabdala {
            sutur.push(format!(
                "  {masar}: replaced by the store since installation; it would be left alone"
            ));
        }
        for masar in &self.mafquda {
            sutur.push(format!("  {masar}: recorded and not on disk"));
        }
        if self.mutabaqqi > self.li_istiada.saturating_add(self.li_hadhf) {
            sutur.push(format!(
                "  {} record(s) outstanding in total, including settings",
                self.mutabaqqi
            ));
        }
        sutur
    }
}

/// Deletes one installation's manifest and its preserved originals, once there
/// is provably nothing left to restore.
///
/// The step that reclaims the disk an uninstall stopped needing, and the one that
/// must never run a moment early. It re-reads the manifest and refuses unless
/// **every** record — every file, every directory and every setting — is marked
/// restored. That check is not a formality: the backup directory is the only copy
/// of the user's original files, and deleting it while a single record is still
/// outstanding converts a resumable uninstall into a game that cannot be repaired
/// by anything except the launcher.
///
/// Returns how many bytes were freed, taken from the manifest's own recorded
/// compressed sizes before anything is removed.
///
/// Deliberately not called by [`istiada_nass`] or [`istiada_sawt`]. An uninstall
/// that also destroys its own evidence leaves nothing to diagnose when the user
/// says afterwards that their game is not right, and a few hundred megabytes is
/// a cheap price for being able to answer them. The caller decides when the
/// answer stops being needed.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] when the manifest cannot be read or still has
/// outstanding records — naming how many — and the I/O variants when the
/// directory cannot be removed.
pub fn nazzif_nusakh(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
) -> NatijatTathbeet<u64> {
    let tathbeet = Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw)?;
    let bayan = tathbeet.bayan();

    let mutabaqqi = bayan.mutabaqqi();
    if mutabaqqi > 0 {
        return Err(KhataTathbeet::BayanTalif {
            masar: tathbeet.masar_bayan().to_path_buf(),
            sabab: format!(
                "{mutabaqqi} record(s) are still not restored, so the preserved originals \
                 are still the only copy of them. Finish the uninstall first: {}",
                bayan.qaimat_mutabaqqi().join(", ")
            ),
        });
    }

    let hajm = bayan.hajm_nusakh();
    let mujallad = tathbeet.mujallad_asl().to_path_buf();
    let masar_bayan = tathbeet.masar_bayan().to_path_buf();
    drop(tathbeet);

    // The originals go first and the manifest second. The reverse order would
    // leave, on an interrupted run, a directory of unlabelled backups that no
    // manifest names and that nothing will ever be able to restore or identify.
    // This way an interruption leaves a manifest whose backups are gone, which
    // reads correctly as "nothing left to restore" — which is true.
    if mujallad.exists() {
        // `<backup root>/<kind>/asl`, two components below a backup root that
        // was itself built by `masarat::dakhil`, which refuses a result equal to
        // its root. The data root is three levels up and unreachable from here.
        #[expect(
            clippy::disallowed_methods,
            reason = "two components below a validated per-game backup root, never a root itself"
        )]
        fs::remove_dir_all(&mujallad).map_err(|sabab| {
            min_khata_io(&mujallad, "removing the preserved originals", sabab)
        })?;
    }
    fs::remove_file(&masar_bayan)
        .map_err(|sabab| min_khata_io(&masar_bayan, "removing the manifest", sabab))?;

    Ok(hajm)
}

/// Every game under a `nusakh/` root that still has an installation recorded.
///
/// The input a library sweep needs when the database is unavailable — a user who
/// copied their games and their `nusakh/` directory to a new machine has no
/// `SQLite` ledger and still has to be able to uninstall. The game roots come from
/// each manifest's own record and are returned for the caller to confirm or
/// replace, never used to write anything: a manifest's recorded root is a
/// diagnostic, and a game that has moved is found by the caller, not guessed at
/// here.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] when the backup root cannot be listed. An
/// individual directory that holds no readable manifest is skipped rather than
/// failing the sweep, because one abandoned directory must not stop a user
/// uninstalling everything else.
pub fn ihsa_al_maktaba(jidhr_nusakh: &Path) -> NatijatTathbeet<Vec<MawqiTathbeet>> {
    let madakhil = fs::read_dir(jidhr_nusakh)
        .map_err(|sabab| min_khata_io(jidhr_nusakh, "listing the backup directory", sabab))?;

    let mut mawaqi: BTreeMap<PathBuf, MawqiTathbeet> = BTreeMap::new();
    for madkhal in madakhil.flatten() {
        let mujallad = madkhal.path();
        if !mujallad.is_dir() {
            continue;
        }
        for naw in NawTathbeet::KULL {
            if !Tathbeet::mawjud(&mujallad, naw) {
                continue;
            }
            let masar_bayan = mujallad.join(naw.ism_bayan());
            let Ok(bayan) =
                taarib_usus::mukhattat::iqra_malaf::<BayanTathbeet>(&masar_bayan)
            else {
                continue;
            };
            let _ = mawaqi.entry(mujallad.clone()).or_insert_with(|| MawqiTathbeet {
                ism: bayan.ism_luba.clone(),
                jidhr_luba: bayan.jidhr_luba_asli.clone(),
                jidhr_nusakh: mujallad.clone(),
            });
        }
    }

    Ok(mawaqi.into_values().collect())
}
