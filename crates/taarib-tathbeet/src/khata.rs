//! أخطاء التثبيت — what stops an install, an uninstall or a verification, and
//! what the person in front of the screen is supposed to do about it.
//!
//! Every other error module in this workspace answers "what went wrong". This
//! one has to answer three questions, because a restore that fails is the worst
//! failure the product has: **which file**, **why exactly**, and **what to do
//! now**. A message saying "the uninstall failed" is not a message, it is an
//! apology. The user is holding a game they paid for, with Taarib's bytes in it,
//! and they asked to have their game back.
//!
//! ## Three things this module refuses to blur
//!
//! **A locked file is not a permission failure.** They arrive as the same
//! `io::Error` kind on Windows — a sharing violation surfaces as
//! [`std::io::ErrorKind::PermissionDenied`] through several std entry points —
//! and they have opposite remedies. A locked file needs the game closed and one
//! retry. A permission failure needs an elevation or an ownership change, and
//! retrying it forever accomplishes nothing. [`min_khata_io`] separates them by
//! looking at the raw OS code *before* the kind, because the kind is the lossy
//! one.
//!
//! **A missing backup is not a corrupt backup.** A backup that is gone means
//! Taarib has nothing to write; a backup that is present and hashes wrong means
//! Taarib has something to write and knows it is not the original. Both end the
//! same way — the file is not restored — and both point at the same remedy, the
//! launcher's own file verification, which is the one mechanism on the machine
//! that still has an authoritative copy of the game. Saying which of the two
//! happened is what lets a maintainer tell a deleted `nusakh/` directory from a
//! failing disk.
//!
//! **A partial restore is a failure.** There is no variant here that means
//! "most of it worked". [`KhataTathbeet::IstiadaNaqisa`] carries the count that
//! completed *and the list of what did not*, and it is [`Khutura::Fadih`] —
//! the severity reserved for an operation that failed and left something behind
//! that needs attention. Reporting a partial restore as a success is how a user
//! comes to believe their game is clean while three files still hold Taarib's
//! bytes. Its remedy is delegated to the failure underneath it, because the way
//! to finish a partial restore is to fix the thing that stopped it and run
//! again — the restore itself is already resumable.
//!
//! ## The `siyaq` trap
//!
//! Three variants carry both a path and a [`std::io::Error`]. Their context comes
//! from [`siyaq_io`], which returns *its own map*; inserting the path into a
//! freshly built general map and then returning that map throws the I/O half
//! away, and the resulting diagnostics bundle names a file with no error kind
//! and no OS code. Three crates in this workspace shipped that bug. Here those
//! variants — and [`KhataTathbeet::IstiadaNaqisa`], which merges a nested map
//! for the same structural reason — are answered and returned *before* the
//! general map or its closure exists, so there is no map for them to be
//! inserted into wrongly.
//!
//! ## Error codes
//!
//! [`arqam::TATHBEET`] is 6200 and belongs to this crate alone. Restore, the
//! manifest and verification take 6200–6216; the install pipeline's own
//! refusals take 6217–6219 and continue at 6280. 6220–6239 belong to `tarkib`
//! (framework installation), 6240–6259 to `najat_tahdith` (update survival)
//! and 6260–6279 to `itlaq` (launch integration), so that no module ever has
//! to renumber anything that has shipped. A code that has been seen by a user
//! is permanent.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_usus::khata::{
    Khutura, Khutwa, MasarMatlub, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam, khutwa_io,
    siyaq_io,
};
use taarib_usus::khata_min;
use taarib_usus::manassa::Sunduq;

/// The result type of every fallible operation in this crate.
pub type NatijatTathbeet<T> = Result<T, KhataTathbeet>;

/// Windows `ERROR_SHARING_VIOLATION`: another handle has the file open.
///
/// The single most common uninstall failure in the product, because the user's
/// natural order of operations is to close the game's window — which does not
/// always end the process — and then click Uninstall.
const RAMZ_MUSHARAKA: i32 = 32;

/// Windows `ERROR_LOCK_VIOLATION`: a byte range in the file is locked.
const RAMZ_QUFL: i32 = 33;

/// Unix `ETXTBSY`: the file is a running executable.
///
/// The Linux and macOS shape of the same situation. A game's binary cannot be
/// rewritten while it is executing, and the kernel says so with this code rather
/// than with a permission error.
const RAMZ_NASS_MASHGHUL: i32 = 26;

/// Which direction a compression step was going when it failed.
///
/// Kept apart because the remedies have nothing in common. Failing to *compress*
/// a backup is almost always a full volume, and the answer is disk space.
/// Failing to *expand* one means the stored bytes are not a zstd frame any more,
/// and the answer is the launcher's own verification, because Taarib's copy of
/// that original is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IttijahDaght {
    /// Compressing an original on its way into the backup directory.
    Daght,
    /// Expanding an original on its way back out.
    Fakk,
}

impl IttijahDaght {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Daght => "compressing a backup",
            Self::Fakk => "expanding a backup",
        }
    }
}

/// Failures of installation, restore and verification.
#[derive(Debug, thiserror::Error)]
pub enum KhataTathbeet {
    /// A game file, a backup or a manifest could not be read or written.
    ///
    /// The residual case, reached only after [`min_khata_io`] has ruled out a
    /// lock and a permission refusal. `amal` names what was being attempted, so
    /// that "could not be read" in a bug report is "could not read the backup of
    /// `data.win`" rather than a bare path.
    #[error("{masar} could not be read or written while {amal}")]
    KhataMalaf {
        /// The path that failed.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A file is held open by another process — almost always the game itself.
    ///
    /// Separated from a permission failure deliberately. This one is resolved by
    /// the user closing something, it is transient, and retrying is the correct
    /// action. `amaliya` carries the process's name when the caller managed to
    /// find out which one; naming it turns "close whatever is using the file"
    /// into "close Undertale.exe", which is the difference between advice and an
    /// instruction.
    #[error("{masar} is locked by another process and cannot be written")]
    MalafMaqful {
        /// The path that is held.
        masar: PathBuf,
        /// The process holding it, when it is known.
        amaliya: Option<String>,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// The operating system refused the access outright.
    ///
    /// A game installed under `Program Files`, a library directory owned by
    /// another user, a read-only mount, a file whose owner is `root` because a
    /// previous tool ran under `sudo`. Retrying changes nothing; the user has to
    /// grant something.
    #[error("the system refused access to {masar} while {amal}")]
    SalahiyaMarfuda {
        /// The path that was refused.
        masar: PathBuf,
        /// What was being attempted.
        amal: &'static str,
        /// The underlying failure.
        #[source]
        sabab: std::io::Error,
    },

    /// A path names somewhere outside the game's own directory.
    ///
    /// Raised on the manifest's own contents, not only on caller input: a
    /// manifest is a file on the user's disk, and a manifest carrying
    /// `../../../etc/hosts` is either a Taarib bug or a hostile patch. Either
    /// way nothing in it is acted on.
    #[error("{masar} is not inside {jidhr}")]
    MasarKharij {
        /// The path that escaped.
        masar: PathBuf,
        /// The root it was supposed to stay under.
        jidhr: PathBuf,
        /// Which rule it broke.
        sabab: String,
    },

    /// An operation named a path the manifest does not record.
    ///
    /// The refusal that makes "no installation ever touches a file outside the
    /// manifest" enforceable rather than aspirational. A caller that reaches
    /// this has asked to write, delete or restore something Taarib never
    /// preserved, and the answer is always no.
    #[error("{masar} is not recorded in this installation's manifest")]
    SijillMafqud {
        /// The path that is not in the manifest.
        masar: PathBuf,
        /// Which manifest was searched.
        bayan: PathBuf,
    },

    /// The manifest is unreadable, internally inconsistent, or edited by hand.
    ///
    /// Fatal for every operation, including verification. A manifest that
    /// contradicts itself cannot be used to decide which of two files holds
    /// somebody's original, and choosing one anyway is how an uninstall writes
    /// the wrong bytes into a game.
    #[error("the installation manifest at {masar} cannot be trusted")]
    BayanTalif {
        /// The manifest's path.
        masar: PathBuf,
        /// The specific contradiction.
        sabab: String,
    },

    /// A manifest is already present where a new installation wanted to write
    /// one.
    ///
    /// Two installations sharing one backup directory is the state in which
    /// uninstalling the second restores originals belonging to the first. It is
    /// far cheaper to refuse it here than to detect it afterwards, when the
    /// originals have already been mixed.
    #[error("{masar} already holds an installation manifest")]
    BayanMawjud {
        /// The manifest that is already there.
        masar: PathBuf,
        /// What it says it belongs to.
        huwiya: String,
    },

    /// The manifest was written by a build that knows a schema this one does
    /// not.
    ///
    /// Refused rather than migrated. A partial read would drop the fields the
    /// newer build added, rewrite the manifest without them at the next save,
    /// and destroy the information needed to undo a patch that is still
    /// installed. Refusing costs the user an update; guessing costs them the
    /// only record of what was changed in their game.
    #[error("the manifest is schema {mawjud}; this build reads {madum}")]
    IsdarBayanMajhul {
        /// The manifest's path.
        masar: PathBuf,
        /// The version found in the file.
        mawjud: u32,
        /// The version this build writes.
        madum: u32,
    },

    /// A declared size is above what this build will preserve.
    ///
    /// Checked against the size the filesystem reports, before a byte is read,
    /// so a pathological file is a refusal rather than an exhausted machine.
    #[error("{haql} declares {qeema} bytes, above the ceiling of {saqf}")]
    HajmMufrit {
        /// Which field or file.
        haql: &'static str,
        /// What it declared.
        qeema: u64,
        /// The ceiling.
        saqf: u64,
    },

    /// A backup could not be compressed on the way in, or expanded on the way
    /// out.
    #[error("{masar}: {} failed", .ittijah.ism())]
    DaghtFashil {
        /// The backup involved.
        masar: PathBuf,
        /// Which direction.
        ittijah: IttijahDaght,
        /// What the compressor reported.
        tafsil: String,
    },

    /// The manifest names a backup that is not in the backup directory.
    ///
    /// Usually a `nusakh/` directory that was cleaned up by hand or by a disk
    /// utility, occasionally a copy of a game that was moved without its
    /// backups. Taarib has nothing to write back, and the launcher's own file
    /// verification is the only remaining source of the original.
    #[error("the backup of {masar} is missing from {jidhr_nusakh}")]
    NuskhaMafquda {
        /// The game file that cannot be restored.
        masar: PathBuf,
        /// The backup key the manifest named.
        miftah: String,
        /// Where the backups live.
        jidhr_nusakh: PathBuf,
    },

    /// A backup is present and is not the bytes that were recorded for it.
    ///
    /// The backup directory is on the same disk as everything else and is not
    /// magic. Bit rot, a truncated write from a power cut during the install, a
    /// partial sync from a cloud folder, a user who opened a file in
    /// `nusakh/asl/` and saved it. Writing it into the game anyway would replace
    /// a patched file with a corrupt one, which is strictly worse than leaving
    /// the patch in place.
    #[error("the backup of {masar} hashes to {mahsuba}, not the recorded {muallana}")]
    NuskhaTalifa {
        /// The game file that cannot be restored.
        masar: PathBuf,
        /// The backup key the manifest named.
        miftah: String,
        /// The fingerprint recorded at install time.
        muallana: String,
        /// The fingerprint of the backup as it is now.
        mahsuba: String,
    },

    /// The file on disk is not the file the manifest describes.
    ///
    /// The store-update case, and the reason a restore is not simply a copy. The
    /// launcher pushed a new build, rewrote the file, and the bytes now there
    /// belong to a *newer* game than the original Taarib preserved. Writing the
    /// backup over them would silently downgrade a file the store just updated,
    /// which is a far more confusing outcome than a refusal.
    #[error("{masar} has been replaced since Taarib wrote it")]
    MalafMustabdal {
        /// The path that no longer matches.
        masar: PathBuf,
        /// What the manifest recorded for it.
        muallana: String,
        /// What is there now.
        mahsuba: String,
    },

    /// A restore completed and then failed its own verification.
    ///
    /// The original was written back, the file was re-read, and its fingerprint
    /// is not the one recorded before the patch ever touched it. The write
    /// succeeded and the bytes are wrong, which means the disk, the filesystem
    /// or another process is not behaving. Reporting this is not pedantry:
    /// telling a user their game is clean when it is not is the single failure
    /// this crate exists to prevent.
    #[error("{masar} was restored and hashes to {mahsuba}, not the recorded {muallana}")]
    IstiadaGhayrMutabaqa {
        /// The path that was restored.
        masar: PathBuf,
        /// The fingerprint recorded before the patch.
        muallana: String,
        /// The fingerprint of what actually landed.
        mahsuba: String,
    },

    /// A file came back with the right bytes and the wrong mode.
    ///
    /// A restore is not finished when the contents match. A Ren'Py launcher
    /// script that comes back without its execute bit does not start; a file
    /// that comes back read-only when it was not stops the next install; a file
    /// whose read-only flag was dropped stops being protected. The bytes are
    /// only one of the things that were taken away.
    #[error("{masar} was restored but its original permissions could not be put back")]
    SalahiyatGhayrMustaada {
        /// The path.
        masar: PathBuf,
        /// What could not be applied.
        sabab: String,
    },

    /// A launcher setting or registry value could not be put back.
    ///
    /// The user's configuration is theirs. A patch that appends to a launch
    /// option and cannot remove what it appended has left the user editing a
    /// text field to undo something Taarib did.
    #[error("the previous value of {muarrif} could not be restored")]
    IdadGhayrMustaad {
        /// Which setting.
        muarrif: String,
        /// Where it lives.
        mahall: String,
        /// Why it could not be put back.
        sabab: String,
    },

    /// A launch-time setting the deployment recorded could not be applied.
    ///
    /// The install-side mirror of [`Self::IdadGhayrMustaad`], and a refusal for
    /// the same kind of reason. A framework was deployed a moment before this,
    /// and it does not load unless the launcher is told to put it in front of
    /// the game. An install that shrugged here would report success over a game
    /// that runs exactly as it did before, with Taarib's files sitting inside it
    /// unread — which is the one failure a user has no way of noticing.
    #[error("the launch setting {mahall} could not be applied")]
    IdadGhayrMunaffadh {
        /// Where the setting lives, as the record names it.
        mahall: String,
        /// Why it could not be applied.
        sabab: String,
    },

    /// The restore stopped part of the way through.
    ///
    /// Not a success with a caveat. The manifest still records every path that
    /// has *not* been put back, each completed file is already marked, and
    /// running the restore again resumes at the first unfinished line rather
    /// than starting over — so the user's next action is to clear whatever
    /// `sabab` names and press the same button, not to start from scratch.
    ///
    /// `mutabaqqi` is the whole list, not a count, because the one question a
    /// user asks at this point is "what is still modified in my game?" and a
    /// number does not answer it.
    #[error("{luba}: {munjaz} path(s) restored, {} still modified", .mutabaqqi.len())]
    IstiadaNaqisa {
        /// Which installation.
        luba: String,
        /// How many paths were restored and verified before the failure.
        munjaz: usize,
        /// Every path that is still not back to its original state.
        mutabaqqi: Vec<String>,
        /// The failure that stopped the run.
        #[source]
        sabab: Box<Self>,
    },

    // --- the install pipeline (Phase 15) -------------------------------------
    /// The game's own process is running.
    #[error("{amaliya} is running and the game cannot be modified until it exits")]
    LubaTashtaghil {
        /// The process, as the system names it.
        amaliya: String,
        /// The executable it was matched against.
        tanfidhi: PathBuf,
    },

    /// Whether the game's process is running could not be determined.
    ///
    /// Separate from [`Self::LubaTashtaghil`] because they are different
    /// sentences: one says a process was seen, this one says the question was
    /// asked and came back unanswerable. Folding the second into the first
    /// would name a process that was never observed; folding it into success
    /// would install into a game that may be open.
    #[error("cannot see the host's processes from inside {}; whether {} is running is \
             unknown", sunduq.ism(), tanfidhi.display())]
    HalatLubaMajhula {
        /// The sandbox whose private process table hid the answer.
        sunduq: Sunduq,
        /// The executable whose state could not be read.
        tanfidhi: PathBuf,
    },

    /// The package failed verification and nothing was written.
    #[error("{masar} failed verification and was not installed: {sabab}")]
    RuqaaMarfuda {
        /// The package file.
        masar: PathBuf,
        /// Which check refused, in the container's words.
        sabab: String,
    },

    /// The script-engine write could not be attempted, or the adapter refused.
    ///
    /// Distinct from [`KhataTathbeet::KhataMalaf`] because nothing about the
    /// *filesystem* went wrong: the package's string table would not read, or
    /// the game's own data file has drifted from what the patch was built
    /// against, or the container would not round-trip. In every one of those the
    /// game is untouched, which is the fact this variant asserts and the reason
    /// its remedy is to look at the patch rather than at the disk.
    #[error("{masar} could not be patched through its own engine's data: {sabab}")]
    NususMarfuda {
        /// The game root.
        masar: PathBuf,
        /// What refused, in the adapter's own words.
        sabab: String,
    },

    /// The package does not apply to the installed build, or applies only
    /// approximately without the user's acknowledgement.
    #[error("the package does not apply to this build: {hukm}")]
    TawafuqMarfud {
        /// The verdict, as `SababMutabaqa::wasf_injilizi` words it.
        hukm: String,
        /// Whether an explicit acknowledgement would make it installable.
        yumkin_bi_iqrar: bool,
    },

    /// A framework component is not in Taarib's component store.
    #[error("the component {mukawwin} is not in the component store at {masar}")]
    MukawwinMafqud {
        /// The component the per-engine table asked for.
        mukawwin: String,
        /// Where it was expected.
        masar: PathBuf,
    },

    /// A framework component is in the store at the wrong size.
    ///
    /// Distinct from [`KhataTathbeet::MukawwinMafqud`] because the remedy is
    /// the same but the evidence is not: a file that is present and the wrong
    /// length is a mirror that was interrupted mid-write or a store somebody
    /// edited, and naming both sizes is what makes that legible in a report.
    #[error("the component {mukawwin} holds {mawjud} byte(s) at {}, not the {muallan} the \
             manifest declares", masar.display())]
    MukawwinNaqis {
        /// The component the per-engine table asked for.
        mukawwin: String,
        /// The file that is the wrong size.
        masar: PathBuf,
        /// What the manifest declares.
        muallan: u64,
        /// What is on disk.
        mawjud: u64,
    },

    /// The loader slot Taarib's own module is published as is already held by a
    /// file Taarib did not put there.
    ///
    /// A Windows game loads a module by name from beside its own executable
    /// before it looks in `System32`, which is what makes a proxy DLL work at
    /// all — and what makes the slot exclusive. Writing Taarib's `version.dll`
    /// over another mod's does not chain the two: the first mod is gone, its own
    /// files are left behind pointing at a loader that no longer exists, and
    /// nothing anywhere says so. So the install refuses and names the file.
    ///
    /// This is the install-time half of the rule `taarib-haqn` already applies
    /// inside a running process — never unhook someone else, because whoever
    /// took the slot last is the only one who can give it back.
    #[error("{wakeel} beside the game is already another mod's loader ({}, {hajm} byte(s), \
             {}); Taarib will not write over it", masar.display(), huwiya.wasf_injilizi())]
    WakeelMashghul {
        /// The module name Taarib's loader is published as, `version.dll` on
        /// Windows.
        wakeel: String,
        /// The file holding the slot.
        masar: PathBuf,
        /// Which product that file belongs to, as
        /// `wukala::HuwiyatWakeel::wasf_injilizi` renders it — a named mod with
        /// the evidence that named it, an ambiguity between two, or an
        /// unidentified proxy said to be exactly that.
        ///
        /// "Something owns `version.dll`" is a refusal a user can do nothing
        /// with. "`ReShade` owns it, and here is what proved that" is one they can
        /// act on, and the difference is the whole reason this field exists
        /// rather than the size alone standing in for an identity.
        ///
        /// Structured rather than a rendered sentence, because this error is
        /// shown in two languages and a pre-rendered English clause dropped
        /// into the Arabic message would be the one line on that screen that is
        /// not Arabic.
        huwiya: crate::wukala::HuwiyatWakeel,
        /// Its size in bytes, which is what separates a real system module
        /// somebody copied in from a mod loader standing in for one.
        hajm: u64,
        /// The other loader slots in use beside it, so the report names the mod
        /// rather than only the collision.
        jiran: Vec<String>,
    },

    /// The safety authorisation is for a different game or package.
    #[error("the safety authorisation does not cover this game and package")]
    IdhnGhayrMutabiq,

    /// The compatibility prefix a Linux install needs is missing or unusable.
    #[error("the compatibility prefix at {jidhr} cannot be used: {sabab}")]
    BeeaMafquda {
        /// The prefix root the launcher named.
        jidhr: PathBuf,
        /// What is wrong with it.
        sabab: String,
    },

    /// A launcher is running while its configuration must be edited.
    #[error("{manassa} is running and rewrites {malaf} on exit; close it first")]
    MunassaTaamal {
        /// The launcher.
        manassa: String,
        /// The configuration file it owns.
        malaf: PathBuf,
    },

    /// Whether the launcher is running could not be determined.
    ///
    /// The [`Self::MunassaTaamal`] of the third state, and kept apart from it
    /// for the same reason [`Self::HalatLubaMajhula`] is kept apart from
    /// [`Self::LubaTashtaghil`]: "close it and try again" is advice that only
    /// makes sense once something was actually seen running.
    #[error("cannot see the host's processes from inside {}; whether {manassa} is running is \
             unknown, so {} was left alone", sunduq.ism(), malaf.display())]
    HalatManassaMajhula {
        /// The sandbox whose private process table hid the answer.
        sunduq: Sunduq,
        /// The launcher that could not be checked.
        manassa: String,
        /// The configuration file it owns, which was not rewritten.
        malaf: PathBuf,
    },
}

impl KhataTathbeet {
    /// Attaches the name of the process holding a locked file.
    ///
    /// Called by a caller that knows something this module cannot find out on
    /// its own — `taarib-kashf` can enumerate the processes with a handle on the
    /// game directory, and this crate deliberately does not, because a restore
    /// must not depend on a process enumeration succeeding. Anything other than
    /// [`KhataTathbeet::MalafMaqful`] passes through untouched.
    #[must_use]
    pub fn bi_amaliya(self, ism: impl Into<String>) -> Self {
        match self {
            Self::MalafMaqful { masar, sabab, .. } => {
                Self::MalafMaqful { masar, amaliya: Some(ism.into()), sabab }
            }
            akhar => akhar,
        }
    }

    /// The game path this failure is about, when it is about one.
    ///
    /// Used by the sweep in `taraju` to build the list of what remains without
    /// re-deriving it from the error message.
    #[must_use]
    pub fn masar(&self) -> Option<&Path> {
        match self {
            Self::KhataMalaf { masar, .. }
            | Self::MalafMaqful { masar, .. }
            | Self::SalahiyaMarfuda { masar, .. }
            | Self::MasarKharij { masar, .. }
            | Self::SijillMafqud { masar, .. }
            | Self::BayanTalif { masar, .. }
            | Self::BayanMawjud { masar, .. }
            | Self::IsdarBayanMajhul { masar, .. }
            | Self::DaghtFashil { masar, .. }
            | Self::NuskhaMafquda { masar, .. }
            | Self::NuskhaTalifa { masar, .. }
            | Self::MalafMustabdal { masar, .. }
            | Self::IstiadaGhayrMutabaqa { masar, .. }
            | Self::SalahiyatGhayrMustaada { masar, .. }
            | Self::RuqaaMarfuda { masar, .. }
            | Self::NususMarfuda { masar, .. }
            | Self::MukawwinMafqud { masar, .. }
            | Self::MukawwinNaqis { masar, .. }
            | Self::WakeelMashghul { masar, .. } => Some(masar),
            Self::LubaTashtaghil { tanfidhi, .. }
            | Self::HalatLubaMajhula { tanfidhi, .. } => Some(tanfidhi),
            Self::BeeaMafquda { jidhr, .. } => Some(jidhr),
            Self::MunassaTaamal { malaf, .. } | Self::HalatManassaMajhula { malaf, .. } => {
                Some(malaf)
            }
            Self::HajmMufrit { .. }
            | Self::IdadGhayrMustaad { .. }
            | Self::IdadGhayrMunaffadh { .. }
            | Self::TawafuqMarfud { .. }
            | Self::IdhnGhayrMutabiq => None,
            Self::IstiadaNaqisa { sabab, .. } => sabab.masar(),
        }
    }

    /// Whether running the same operation again could plausibly succeed without
    /// the user changing anything on the machine.
    ///
    /// Only a lock qualifies, and only because the thing holding it is usually a
    /// game the user is about to close anyway. Everything else needs an action
    /// first, and a retry loop over it is a spinner that never stops.
    ///
    /// [`Self::HalatLubaMajhula`] and [`Self::HalatManassaMajhula`] fall on the
    /// false side deliberately, and are the one pair where that is not obvious:
    /// they read like a lock, but a sandbox hands this build the same private
    /// process table on every attempt, so the same refusal is the only answer a
    /// retry can ever produce.
    #[must_use]
    pub fn qabil_lil_iada(&self) -> bool {
        match self {
            Self::MalafMaqful { .. }
            | Self::LubaTashtaghil { .. }
            | Self::MunassaTaamal { .. } => true,
            Self::IstiadaNaqisa { sabab, .. } => sabab.qabil_lil_iada(),
            _ => false,
        }
    }
}

impl Tafsir for KhataTathbeet {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::TATHBEET
                + match self {
                    Self::KhataMalaf { .. } => 0,
                    Self::MalafMaqful { .. } => 1,
                    Self::SalahiyaMarfuda { .. } => 2,
                    Self::MasarKharij { .. } => 3,
                    Self::SijillMafqud { .. } => 4,
                    Self::BayanTalif { .. } => 5,
                    Self::BayanMawjud { .. } => 6,
                    Self::IsdarBayanMajhul { .. } => 7,
                    Self::HajmMufrit { .. } => 8,
                    Self::DaghtFashil { .. } => 9,
                    Self::NuskhaMafquda { .. } => 10,
                    Self::NuskhaTalifa { .. } => 11,
                    Self::MalafMustabdal { .. } => 12,
                    Self::IstiadaGhayrMutabaqa { .. } => 13,
                    Self::SalahiyatGhayrMustaada { .. } => 14,
                    Self::IdadGhayrMustaad { .. } => 15,
                    Self::IstiadaNaqisa { .. } => 16,
                    Self::LubaTashtaghil { .. } => 17,
                    Self::RuqaaMarfuda { .. } => 18,
                    Self::TawafuqMarfud { .. } => 19,
                    // Inside the band the header reserves for `tarkib`.
                    Self::MukawwinMafqud { .. } => 20,
                    Self::BeeaMafquda { .. } => 21,
                    Self::MukawwinNaqis { .. } => 22,
                    Self::WakeelMashghul { .. } => 23,
                    // Inside the band reserved for `itlaq`.
                    Self::MunassaTaamal { .. } => 60,
                    Self::HalatManassaMajhula { .. } => 61,
                    Self::IdadGhayrMunaffadh { .. } => 62,
                    // The pipeline's own continuation band.
                    Self::IdhnGhayrMutabiq => 80,
                    Self::NususMarfuda { .. } => 81,
                    Self::HalatLubaMajhula { .. } => 82,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Fadih is the severity for "the operation failed and left
            // something behind that needs attention". These four are exactly
            // that: a game that is neither patched nor restored, a file whose
            // mode is wrong, a setting the user now has to fix by hand.
            Self::IstiadaNaqisa { .. }
            | Self::IstiadaGhayrMutabaqa { .. }
            | Self::SalahiyatGhayrMustaada { .. }
            | Self::IdadGhayrMustaad { .. } => Khutura::Fadih,

            // Everything else refused before it changed anything, which is the
            // whole point of refusing.
            _ => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::KhataMalaf { .. } => {
                "تعذّر فتح ملف من ملفات اللعبة أو النسخ الاحتياطية أو الكتابة إليه.".to_owned()
            }
            Self::MalafMaqful { amaliya, .. } => match amaliya {
                Some(ism) => format!(
                    "الملف مفتوح من قبل «{ism}»، ولا يمكن الكتابة إليه ما دام كذلك. أغلق \
                     البرنامج ثم أعد المحاولة."
                ),
                None => "الملف مفتوح من قبل برنامج آخر — غالبًا اللعبة نفسها. أغلق اللعبة \
                         تمامًا ثم أعد المحاولة."
                    .to_owned(),
            },
            Self::SalahiyaMarfuda { .. } => {
                "رفض النظام الوصول إلى الملف. مجلّد اللعبة يحتاج صلاحية لا يملكها تعريب \
                 حاليًا."
                    .to_owned()
            }
            Self::MasarKharij { .. } => {
                "أحد المسارات المسجّلة يشير خارج مجلّد اللعبة، ولا يكتب تعريب خارجه. لم \
                 يُنفَّذ شيء من هذا البيان."
                    .to_owned()
            }
            Self::SijillMafqud { .. } => {
                "طُلب تعديل ملف لا يسجّله بيان التثبيت. لا يمسّ تعريب ملفًا لم يحفظه أوّلًا."
                    .to_owned()
            }
            Self::BayanTalif { .. } => {
                "بيان التثبيت غير متّسق مع نفسه، فلا يمكن الاعتماد عليه لإرجاع أي ملف."
                    .to_owned()
            }
            Self::BayanMawjud { .. } => {
                "يوجد بيان تثبيت آخر في هذا المجلّد. لن تُخلط نسخ تثبيتين في مكان واحد."
                    .to_owned()
            }
            Self::IsdarBayanMajhul { .. } => {
                "كُتب بيان التثبيت بإصدار أحدث من تعريب. حدِّث البرنامج؛ قراءته بهذا الإصدار \
                 قد تُتلف سجلّ ما عُدِّل في لعبتك."
                    .to_owned()
            }
            Self::HajmMufrit { .. } => {
                "أحد الملفات يعلن حجمًا أكبر مما تحفظه هذه النسخة، ورُفض قبل حجز أي ذاكرة له."
                    .to_owned()
            }
            Self::DaghtFashil { ittijah, .. } => match ittijah {
                IttijahDaght::Daght => {
                    "تعذّر ضغط النسخة الأصلية قبل حفظها؛ غالبًا لا توجد مساحة كافية على \
                     القرص."
                        .to_owned()
                }
                IttijahDaght::Fakk => {
                    "تعذّر فكّ ضغط النسخة الأصلية المحفوظة؛ الملف المحفوظ تالف.".to_owned()
                }
            },
            Self::NuskhaMafquda { .. } => {
                "النسخة الأصلية المسجّلة لهذا الملف غير موجودة. تحقّق من سلامة ملفات اللعبة \
                 من متجرها لاستعادة الملف الأصلي."
                    .to_owned()
            }
            Self::NuskhaTalifa { .. } => {
                "النسخة الأصلية المحفوظة لا تطابق بصمتها المسجّلة، فلن تُكتب فوق ملف اللعبة. \
                 تحقّق من سلامة ملفات اللعبة من متجرها."
                    .to_owned()
            }
            Self::MalafMustabdal { .. } => {
                "تغيّر هذا الملف منذ أن كتبه تعريب — يبدو أن المتجر حدّث اللعبة. إرجاع النسخة \
                 القديمة فوقه سيُرجع اللعبة إلى بناء أقدم، فأُوقف."
                    .to_owned()
            }
            Self::IstiadaGhayrMutabaqa { .. } => {
                "أُعيد الملف الأصلي ولم تطابق بصمته ما سُجِّل له. لم تكتمل الإزالة، ولن يُقال \
                 إنها اكتملت."
                    .to_owned()
            }
            Self::SalahiyatGhayrMustaada { .. } => {
                "أُعيد محتوى الملف ولم تُعَد صلاحياته الأصلية. الملف بمحتوى صحيح وصلاحيات \
                 خاطئة ليس ملفًا مستعادًا."
                    .to_owned()
            }
            Self::IdadGhayrMustaad { .. } => {
                "تعذّر إرجاع أحد إعدادات التشغيل إلى قيمته السابقة. الإعداد إعدادك، ولا يتركه \
                 تعريب معدَّلًا في صمت."
                    .to_owned()
            }
            Self::IdadGhayrMunaffadh { mahall, .. } => format!(
                "تعذّر ضبط إعداد التشغيل ({mahall})، وبدونه لا تُحمَّل ملفات تعريب في اللعبة \
                 أصلًا. أُوقف التثبيت بدل أن يُقال إنه نجح واللعبة تعمل كما كانت."
            ),
            Self::LubaTashtaghil { amaliya, .. } => format!(
                "اللعبة تعمل الآن ({amaliya}). أغلقها تمامًا ثم أعد المحاولة؛ لا يُعدَّل ملف \
                 واللعبة تقرؤه."
            ),
            Self::HalatLubaMajhula { sunduq, .. } => format!(
                "تعريب يعمل داخل {} ولا يرى إلا عمليّاته هو، فتعذّر عليه معرفة هل اللعبة \
                 مفتوحة الآن أم لا. لم يُكتب شيء: تعديل ملفات لعبة مفتوحة يُتلفها، ولعبة لم \
                 تُرَ ليست لعبة رُئيت مغلقة. شغِّل تعريب على الجهاز نفسه لا داخل صندوق عزل \
                 (نسخة AppImage) ليُجرى هذا الفحص.",
                sunduq.ism_arabi()
            ),
            Self::RuqaaMarfuda { .. } => {
                "فشل التحقق من الحزمة، ولم يُكتب منها شيء. لا تُثبَّت حزمة لا تجتاز التحقق."
                    .to_owned()
            }
            Self::NususMarfuda { .. } => {
                "تعذّر ترقيع هذه اللعبة عبر بيانات محرّكها نفسه، ولم يُمسّ منها شيء. اللعبة \
                 كما كانت تمامًا."
                    .to_owned()
            }
            Self::TawafuqMarfud { yumkin_bi_iqrar, .. } => {
                if *yumkin_bi_iqrar {
                    "الحزمة متوافقة تقريبًا مع نسختك، وتثبيتها يحتاج إقرارك بما قد لا يعمل. \
                     لم يُكتب شيء بعد."
                        .to_owned()
                } else {
                    "الحزمة لا تطابق نسخة اللعبة المثبَّتة، ورُفض تثبيتها. لم يُكتب شيء."
                        .to_owned()
                }
            }
            Self::MukawwinMafqud { mukawwin, .. } => format!(
                "أحد مكوّنات الإطار ({mukawwin}) غير موجود في مخزن مكوّنات تعريب. هذه النسخة \
                 ناقصة؛ أعد تثبيت تعريب."
            ),
            Self::MukawwinNaqis { mukawwin, .. } => format!(
                "أحد مكوّنات الإطار ({mukawwin}) في المخزن بحجم غير الحجم المُعلَن. النسخ إلى \
                 المخزن لم يكتمل؛ أعد تثبيت تعريب."
            ),
            Self::WakeelMashghul { wakeel, masar, jiran, huwiya, .. } => {
                let mawdi = masar.display();
                let maa = if jiran.is_empty() {
                    String::new()
                } else {
                    format!(" وبجانبه أيضًا: {}.", jiran.join("، "))
                };
                let man = huwiya.wasf_arabi();
                let bab = huwiya
                    .aila()
                    .and_then(crate::wukala::AilatWakeel::tasalsul_arabi)
                    .map_or_else(String::new, |bab| format!(" {bab}"));
                format!(
                    "يوجد في مجلّد اللعبة ملف باسم {wakeel} ({mawdi})، وهو {man}، والاسم \
                     نفسه الذي يحمّل به تعريب نفسه. تحمّل ويندوز ملفًا واحدًا بهذا الاسم لا \
                     اثنين، والكتابة فوقه تُلغي التعديل الموجود في صمت.{maa} لم يُكتب شيء. \
                     ولا ينتقل تعريب إلى اسم آخر: الاسم الفارغ فارغ لأن اللعبة لا تطلبه، \
                     ووكيل باسم لا يُطلب تثبيتٌ ينجح ولا يفعل شيئًا.{bab} أزل التعديل الآخر \
                     أو غيّر اسم ملفه إن أردت تثبيت تعريب في هذه اللعبة."
                )
            }
            Self::IdhnGhayrMutabiq => {
                "إذن الأمان المقدَّم يخصّ لعبة أو حزمة أخرى، ورُفض التثبيت به.".to_owned()
            }
            Self::BeeaMafquda { .. } => {
                "بيئة التوافق التي تشغَّل اللعبة من خلالها غير موجودة أو غير صالحة. شغِّل \
                 اللعبة مرة واحدة من منصّتها لتُنشأ البيئة، ثم أعد المحاولة."
                    .to_owned()
            }
            Self::MunassaTaamal { manassa, .. } => format!(
                "{manassa} يعمل الآن ويعيد كتابة إعداداته عند إغلاقه، فأي تعديل يُكتب الآن \
                 يُمحى. أغلقه ثم أعد المحاولة."
            ),
            Self::HalatManassaMajhula { sunduq, manassa, .. } => format!(
                "تعريب يعمل داخل {} ولا يرى إلا عمليّاته هو، فتعذّر عليه معرفة هل {manassa} \
                 يعمل الآن أم لا. لم يُمسّ ملف الإعدادات: {manassa} يعيد كتابته من الذاكرة \
                 عند إغلاقه، فأي تعديل يُكتب وهو يعمل يُمحى. شغِّل تعريب على الجهاز نفسه لا \
                 داخل صندوق عزل (نسخة AppImage).",
                sunduq.ism_arabi()
            ),
            Self::IstiadaNaqisa { munjaz, mutabaqqi, sabab, .. } => format!(
                "أُعيد {munjaz} ملفًا وبقي {} لم يُعَد بعد. لم تكتمل الإزالة. {} أصلح السبب ثم \
                 أعد التشغيل: تستأنف الإزالة من حيث توقّفت ولا تبدأ من جديد.",
                mutabaqqi.len(),
                sabab.arabi()
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::KhataMalaf { masar, amal, .. } => {
                format!("{} could not be read or written while {amal}.", masar.display())
            }
            Self::MalafMaqful { masar, amaliya, .. } => match amaliya {
                Some(ism) => format!(
                    "{} is held open by {ism} and cannot be written while it runs. Close it \
                     and try again.",
                    masar.display()
                ),
                None => format!(
                    "{} is held open by another process — usually the game itself. Close the \
                     game completely and try again.",
                    masar.display()
                ),
            },
            Self::SalahiyaMarfuda { masar, amal, .. } => format!(
                "The system refused access to {} while {amal}. This is a permission, not a \
                 lock: retrying will not help until it is granted.",
                masar.display()
            ),
            Self::MasarKharij { masar, jidhr, sabab } => format!(
                "{} is not inside {}: {sabab}. Nothing in this manifest was acted on.",
                masar.display(),
                jidhr.display()
            ),
            Self::SijillMafqud { masar, bayan } => format!(
                "{} is not recorded in {}. Taarib does not touch a file it has not preserved \
                 first, so this was refused rather than performed.",
                masar.display(),
                bayan.display()
            ),
            Self::BayanTalif { masar, sabab } => format!(
                "The installation manifest at {} cannot be trusted: {sabab}",
                masar.display()
            ),
            Self::BayanMawjud { masar, huwiya } => format!(
                "{} already holds a manifest belonging to {huwiya}. Two installations will not \
                 share one backup directory, because uninstalling the second would restore the \
                 first one's originals.",
                masar.display()
            ),
            Self::IsdarBayanMajhul { masar, mawjud, madum } => format!(
                "{} is schema {mawjud} and this build reads {madum}. It was refused rather \
                 than partially read: a partial read would drop the fields it cannot see and \
                 write them away at the next save, destroying the record of a patch that is \
                 still installed.",
                masar.display()
            ),
            Self::HajmMufrit { haql, qeema, saqf } => format!(
                "{haql} declares {qeema} bytes, above the {saqf} this build will preserve. It \
                 was refused before any memory was reserved for it."
            ),
            Self::DaghtFashil { masar, ittijah, tafsil } => {
                format!("{}: {} failed: {tafsil}", masar.display(), ittijah.ism())
            }
            Self::NuskhaMafquda { masar, miftah, jidhr_nusakh } => format!(
                "The backup of {} is missing: {miftah} is not in {}. Taarib has nothing to \
                 write back. Verifying the game's files through its launcher will restore the \
                 original.",
                masar.display(),
                jidhr_nusakh.display()
            ),
            Self::NuskhaTalifa { masar, miftah, muallana, mahsuba } => format!(
                "The backup of {} ({miftah}) hashes to {mahsuba}, not the recorded {muallana}. \
                 It was not written into the game: replacing a patched file with a corrupt one \
                 is worse than leaving the patch in place.",
                masar.display()
            ),
            Self::MalafMustabdal { masar, muallana, mahsuba } => format!(
                "{} hashes to {mahsuba}, not the {muallana} Taarib wrote. The store has \
                 replaced it since the patch was installed, so restoring the older original \
                 over it would downgrade a file the launcher just updated.",
                masar.display()
            ),
            Self::IstiadaGhayrMutabaqa { masar, muallana, mahsuba } => format!(
                "{} was restored and hashes to {mahsuba}, not the recorded {muallana}. The \
                 uninstall did not complete and will not be reported as though it had.",
                masar.display()
            ),
            Self::SalahiyatGhayrMustaada { masar, sabab } => format!(
                "{} was restored byte for byte and its original permissions could not be put \
                 back: {sabab}. A file with the right bytes and the wrong mode is not \
                 restored.",
                masar.display()
            ),
            Self::IdadGhayrMustaad { muarrif, mahall, sabab } => format!(
                "The previous value of {muarrif} in {mahall} could not be restored: {sabab}"
            ),
            Self::IdadGhayrMunaffadh { mahall, sabab } => format!(
                "The launch setting {mahall} could not be applied: {sabab}. Nothing Taarib \
                 deployed would have loaded without it, so the install stopped rather than \
                 report success over a game that runs exactly as it did before."
            ),
            Self::LubaTashtaghil { amaliya, tanfidhi } => format!(
                "{amaliya} is running ({}) and the game cannot be modified until it exits \
                 completely. Close it and try again.",
                tanfidhi.display()
            ),
            Self::HalatLubaMajhula { sunduq, tanfidhi } => format!(
                "Taarib is running inside {} and can only see its own processes, so it cannot \
                 tell whether {} is running. Nothing was written: modifying an open game's \
                 files corrupts them, and a game that could not be seen is not a game that was \
                 seen closed. Run Taarib on the machine itself rather than inside a sandbox — \
                 the AppImage build — so this check can be made.",
                sunduq.ism(),
                tanfidhi.display()
            ),
            Self::RuqaaMarfuda { masar, sabab } => format!(
                "{} failed verification and nothing was written: {sabab}",
                masar.display()
            ),
            Self::NususMarfuda { masar, sabab } => format!(
                "{} could not be patched through its own engine's data and was left exactly \
                 as it was: {sabab}",
                masar.display()
            ),
            Self::TawafuqMarfud { hukm, yumkin_bi_iqrar } => {
                if *yumkin_bi_iqrar {
                    format!(
                        "The package is only approximately compatible ({hukm}) and installing \
                         it needs your explicit acknowledgement of what may not work. Nothing \
                         has been written."
                    )
                } else {
                    format!("The package does not apply to this build ({hukm}) and was refused.")
                }
            }
            Self::MukawwinMafqud { mukawwin, masar } => format!(
                "The framework component {mukawwin} is not in the component store at {}. This \
                 build is incomplete; reinstall Taarib.",
                masar.display()
            ),
            Self::MukawwinNaqis { mukawwin, muallan, mawjud, .. } => format!(
                "The framework component {mukawwin} is in the store at {mawjud} byte(s) where \
                 the manifest declares {muallan}. The copy into the store did not finish; \
                 reinstall Taarib."
            ),
            Self::WakeelMashghul { wakeel, masar, hajm, jiran, huwiya } => {
                let maa = if jiran.is_empty() {
                    String::new()
                } else {
                    format!(" Also in use beside it: {}.", jiran.join(", "))
                };
                let bab = huwiya.aila().and_then(crate::wukala::AilatWakeel::tasalsul).map_or_else(
                    String::new,
                    |bab| format!(" That product does have a way in: {bab}."),
                );
                format!(
                    "The game directory already holds a {wakeel} ({}, {hajm} byte(s)) — {} — \
                     which is the same name Taarib's own loader is published as. Windows loads \
                     one file of that name, not two, so writing over it would remove the mod \
                     that is there without saying so.{maa} Nothing was written. Taarib does not \
                     move to a different name either: a name that is free here is free because \
                     nothing in this game asks for it, and a loader under a name nothing asks \
                     for is an install that succeeds and does nothing.{bab} Remove or rename \
                     that mod's loader if you want Taarib in this game.",
                    masar.display(),
                    huwiya.wasf_injilizi()
                )
            }
            Self::IdhnGhayrMutabiq => {
                "The safety authorisation covers a different game or package, so the install \
                 was refused. This is a defect in the caller, not something you did."
                    .to_owned()
            }
            Self::BeeaMafquda { jidhr, sabab } => format!(
                "The compatibility prefix at {} cannot be used: {sabab}. Run the game once \
                 from its launcher so the prefix is created, then try again.",
                jidhr.display()
            ),
            Self::MunassaTaamal { manassa, malaf } => format!(
                "{manassa} is running and rewrites {} from memory when it exits, so an edit \
                 made now would be silently erased. Close it and try again.",
                malaf.display()
            ),
            Self::HalatManassaMajhula { sunduq, manassa, malaf } => format!(
                "Taarib is running inside {} and can only see its own processes, so it cannot \
                 tell whether {manassa} is running. {} was left exactly as it was: {manassa} \
                 rewrites it from memory when it exits, so an edit made while it runs is \
                 silently erased, and a launcher that could not be seen is not a launcher that \
                 was seen closed. Run Taarib on the machine itself rather than inside a \
                 sandbox — the AppImage build.",
                sunduq.ism(),
                malaf.display()
            ),
            Self::IstiadaNaqisa { luba, munjaz, mutabaqqi, sabab } => format!(
                "{luba}: {munjaz} path(s) restored and verified, {} still modified. The \
                 uninstall did not complete. {} Fix that and run the uninstall again — it \
                 resumes at the first unfinished path rather than starting over. Still \
                 modified: {}",
                mutabaqqi.len(),
                sabab.injilizi(),
                mutabaqqi.join(", ")
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The kind is the lossy signal here too, so the classifier in
            // `min_khata_io` has already stripped the two cases that would map
            // to the wrong action; what reaches this arm is genuinely residual.
            Self::KhataMalaf { sabab, .. } => khutwa_io(sabab, MasarMatlub::MujalladLuba),

            // Transient by nature: something is holding the file, or something
            // is running over it, and the user is the one who can let go. Close
            // it, then press the same button.
            Self::MalafMaqful { .. }
            | Self::LubaTashtaghil { .. }
            | Self::MunassaTaamal { .. } => Khutwa::AadaMuhawala,

            // The opposite of the three above: nothing the user does on this
            // machine changes the answer, because the sandbox will hand this
            // build the same private process table next time. There is no
            // action to offer, so none is offered and the message carries the
            // one remedy there is — a build that is not inside a sandbox.
            Self::HalatLubaMajhula { .. } | Self::HalatManassaMajhula { .. } => Khutwa::LaShay,

            // The same answer as the two above and not the same reason, which
            // is why it is a second arm: there *is* an action here, and it is
            // not one this product may offer as a button — removing somebody
            // else's mod from somebody else's game. The message names the file
            // and says what to do with it, and the decision stays with the
            // person who installed that mod.
            #[expect(
                clippy::match_same_arms,
                reason = "one action, two unrelated reasons for it; merging them would put a \
                          sandbox and a mod collision behind one comment that fits neither"
            )]
            Self::WakeelMashghul { .. } => Khutwa::LaShay,

            Self::SalahiyaMarfuda { .. } | Self::SalahiyatGhayrMustaada { .. } => {
                Khutwa::ManhSalahiya
            }

            // The launcher is the only thing on the machine that still holds an
            // authoritative copy of the game's own files. For a missing backup,
            // a corrupt backup, a store-replaced file or a restore that did not
            // verify, "verify the game's files" is not a consolation — it is the
            // mechanism that actually puts the original back. It also covers a
            // missing compatibility prefix, which the launcher creates by
            // running the game once.
            Self::NuskhaMafquda { .. }
            | Self::NuskhaTalifa { .. }
            | Self::MalafMustabdal { .. }
            | Self::IstiadaGhayrMutabaqa { .. }
            | Self::BeeaMafquda { .. } => Khutwa::TahaqquqSalamatLuba,

            Self::DaghtFashil { ittijah, .. } => match ittijah {
                IttijahDaght::Daght => Khutwa::TahrirMasaha,
                IttijahDaght::Fakk => Khutwa::TahaqquqSalamatLuba,
            },

            Self::IdadGhayrMustaad { .. } | Self::IdadGhayrMunaffadh { .. } => {
                Khutwa::FathIdadat { qism: QismIdadat::Manassat }
            }

            Self::IsdarBayanMajhul { .. }
            | Self::MukawwinMafqud { .. }
            | Self::MukawwinNaqis { .. } => Khutwa::TahdithTaarib,

            // A manifest that names a path outside the game, one that
            // contradicts itself, or a proof for the wrong subject reaching the
            // installer: each is a Taarib defect or a tampered file. None is
            // something the user can fix, and all are things the project needs
            // to see.
            Self::MasarKharij { .. } | Self::BayanTalif { .. } | Self::IdhnGhayrMutabiq => {
                Khutwa::IblaghLilMalik
            }

            // The way to finish a partial restore is to clear whatever stopped
            // it and press the same button again, so the action is the inner
            // failure's action rather than a generic one. Delegating here is
            // what stops a locked file from being reported as "send a
            // diagnostics bundle" once it has been wrapped.
            Self::IstiadaNaqisa { sabab, .. } => sabab.khutwa(),

            // A refused package cannot be repaired locally; the contributor's
            // page is where a corrected one comes from. A near-miss verdict is
            // resolved by re-matching against the installed build.
            Self::RuqaaMarfuda { .. } | Self::NususMarfuda { .. } => Khutwa::IblaghLilMusahim,
            Self::TawafuqMarfud { .. } => Khutwa::IadatMutabaqaBina,

            Self::SijillMafqud { .. } | Self::BayanMawjud { .. } | Self::HajmMufrit { .. } => {
                Khutwa::FathTashkhis
            }
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        // Answered before the general map exists. Three of these carry an
        // `io::Error` whose context comes from `siyaq_io`, which returns its own
        // map; building a general map, inserting the path into it and returning
        // that would silently drop the error kind and the OS code — the two
        // fields that separate a locked file from a full disk in a bug report.
        // The fourth nests another error's map for the same structural reason.
        // Returning here also keeps the closure below from holding a mutable
        // borrow across the merge.
        match self {
            Self::KhataMalaf { masar, amal, sabab }
            | Self::SalahiyaMarfuda { masar, amal, sabab } => {
                let mut siyaq = siyaq_io(sabab);
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("amal".to_owned(), QeemaSiyaq::Nass((*amal).to_owned()));
                return siyaq;
            }
            Self::MalafMaqful { masar, amaliya, sabab } => {
                let mut siyaq = siyaq_io(sabab);
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                if let Some(ism) = amaliya {
                    let _ = siyaq.insert("amaliya".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                }
                return siyaq;
            }
            Self::IstiadaNaqisa { luba, munjaz, mutabaqqi, sabab } => {
                let mut siyaq = BTreeMap::new();
                for (miftah, qeema) in sabab.siyaq() {
                    let _ = siyaq.insert(format!("sabab.{miftah}"), qeema);
                }
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(luba.clone()));
                let _ = siyaq.insert("munjaz".to_owned(), QeemaSiyaq::Hajm(tul_u64(*munjaz)));
                let _ = siyaq
                    .insert("mutabaqqi".to_owned(), QeemaSiyaq::Qaima(mutabaqqi.clone()));
                let _ = siyaq.insert(
                    "sabab.ramz".to_owned(),
                    QeemaSiyaq::Nass(sabab.ramz().to_string()),
                );
                return siyaq;
            }
            _ => {}
        }

        let mut siyaq = BTreeMap::new();
        let mut daa = |miftah: &str, qeema: QeemaSiyaq| {
            let _ = siyaq.insert(miftah.to_owned(), qeema);
        };
        match self {
            // The first four are answered above, before the closure borrowed
            // the map; the last carries no fields to record.
            Self::KhataMalaf { .. }
            | Self::MalafMaqful { .. }
            | Self::SalahiyaMarfuda { .. }
            | Self::IstiadaNaqisa { .. }
            | Self::IdhnGhayrMutabiq => {}
            Self::MasarKharij { masar, jidhr, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("jidhr", QeemaSiyaq::Masar(jidhr.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::SijillMafqud { masar, bayan } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("bayan", QeemaSiyaq::Masar(bayan.clone()));
            }
            Self::BayanTalif { masar, sabab }
            | Self::SalahiyatGhayrMustaada { masar, sabab }
            | Self::RuqaaMarfuda { masar, sabab }
            | Self::NususMarfuda { masar, sabab } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::BayanMawjud { masar, huwiya } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("huwiya", QeemaSiyaq::Nass(huwiya.clone()));
            }
            Self::IsdarBayanMajhul { masar, mawjud, madum } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("mawjud", QeemaSiyaq::Raqm(i64::from(*mawjud)));
                daa("madum", QeemaSiyaq::Raqm(i64::from(*madum)));
            }
            Self::HajmMufrit { haql, qeema, saqf } => {
                daa("haql", QeemaSiyaq::Nass((*haql).to_owned()));
                daa("qeema", QeemaSiyaq::Hajm(*qeema));
                daa("saqf", QeemaSiyaq::Hajm(*saqf));
            }
            Self::DaghtFashil { masar, ittijah, tafsil } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("ittijah", QeemaSiyaq::Nass(ittijah.ism().to_owned()));
                daa("tafsil", QeemaSiyaq::Nass(tafsil.clone()));
            }
            Self::NuskhaMafquda { masar, miftah, jidhr_nusakh } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("miftah", QeemaSiyaq::Nass(miftah.clone()));
                daa("jidhr_nusakh", QeemaSiyaq::Masar(jidhr_nusakh.clone()));
            }
            Self::NuskhaTalifa { masar, miftah, muallana, mahsuba } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("miftah", QeemaSiyaq::Nass(miftah.clone()));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            }
            Self::MalafMustabdal { masar, muallana, mahsuba }
            | Self::IstiadaGhayrMutabaqa { masar, muallana, mahsuba } => {
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("muallana", QeemaSiyaq::Nass(muallana.clone()));
                daa("mahsuba", QeemaSiyaq::Nass(mahsuba.clone()));
            }
            Self::IdadGhayrMustaad { muarrif, mahall, sabab } => {
                daa("muarrif", QeemaSiyaq::Nass(muarrif.clone()));
                daa("mahall", QeemaSiyaq::Nass(mahall.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::IdadGhayrMunaffadh { mahall, sabab } => {
                daa("mahall", QeemaSiyaq::Nass(mahall.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::HalatLubaMajhula { sunduq, tanfidhi } => {
                daa("sunduq", QeemaSiyaq::Nass(sunduq.ism().to_owned()));
                daa("tanfidhi", QeemaSiyaq::Masar(tanfidhi.clone()));
            }
            Self::HalatManassaMajhula { sunduq, manassa, malaf } => {
                daa("sunduq", QeemaSiyaq::Nass(sunduq.ism().to_owned()));
                daa("manassa", QeemaSiyaq::Nass(manassa.clone()));
                daa("malaf", QeemaSiyaq::Masar(malaf.clone()));
            }
            Self::LubaTashtaghil { amaliya, tanfidhi } => {
                daa("amaliya", QeemaSiyaq::Nass(amaliya.clone()));
                daa("tanfidhi", QeemaSiyaq::Masar(tanfidhi.clone()));
            }
            Self::TawafuqMarfud { hukm, yumkin_bi_iqrar } => {
                daa("hukm", QeemaSiyaq::Nass(hukm.clone()));
                daa("yumkin_bi_iqrar", QeemaSiyaq::Nass(yumkin_bi_iqrar.to_string()));
            }
            Self::MukawwinMafqud { mukawwin, masar } => {
                daa("mukawwin", QeemaSiyaq::Nass(mukawwin.clone()));
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
            }
            Self::MukawwinNaqis { mukawwin, masar, muallan, mawjud } => {
                daa("mukawwin", QeemaSiyaq::Nass(mukawwin.clone()));
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("muallan", QeemaSiyaq::Nass(muallan.to_string()));
                daa("mawjud", QeemaSiyaq::Nass(mawjud.to_string()));
            }
            Self::WakeelMashghul { wakeel, masar, hajm, jiran, huwiya } => {
                daa("wakeel", QeemaSiyaq::Nass(wakeel.clone()));
                daa("masar", QeemaSiyaq::Masar(masar.clone()));
                daa("hajm", QeemaSiyaq::Hajm(*hajm));
                daa("jiran", QeemaSiyaq::Qaima(jiran.clone()));
                daa("huwiya", QeemaSiyaq::Nass(huwiya.wasf_injilizi()));
            }
            Self::BeeaMafquda { jidhr, sabab } => {
                daa("jidhr", QeemaSiyaq::Masar(jidhr.clone()));
                daa("sabab", QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::MunassaTaamal { manassa, malaf } => {
                daa("manassa", QeemaSiyaq::Nass(manassa.clone()));
                daa("malaf", QeemaSiyaq::Masar(malaf.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataTathbeet);

/// Classifies an I/O failure on a game file into the variant that names the
/// real cause.
///
/// The whole reason this function exists is that [`std::io::ErrorKind`] is not
/// enough. On Windows a file held open by a running game surfaces as
/// [`std::io::ErrorKind::PermissionDenied`] through several std entry points,
/// which is the same kind a genuinely unwritable `Program Files` directory
/// produces — and the two have opposite remedies. So the raw OS code is checked
/// *first*: `ERROR_SHARING_VIOLATION` and `ERROR_LOCK_VIOLATION` are locks no
/// matter what kind std mapped them to, and on Unix `ETXTBSY` is the same
/// situation for a running executable. Only what survives both checks is treated
/// as a permission refusal, and only what survives that is residual.
///
/// `amal` is a present participle naming the operation — "writing the original
/// back", "reading the backup" — so the message reads as a sentence and a bug
/// report says which step failed rather than only which file.
#[must_use]
pub fn min_khata_io(masar: &Path, amal: &'static str, sabab: std::io::Error) -> KhataTathbeet {
    if maqful(&sabab) {
        return KhataTathbeet::MalafMaqful {
            masar: masar.to_path_buf(),
            amaliya: None,
            sabab,
        };
    }
    if sabab.kind() == std::io::ErrorKind::PermissionDenied
        || sabab.kind() == std::io::ErrorKind::ReadOnlyFilesystem
    {
        return KhataTathbeet::SalahiyaMarfuda { masar: masar.to_path_buf(), amal, sabab };
    }
    KhataTathbeet::KhataMalaf { masar: masar.to_path_buf(), amal, sabab }
}

/// Whether an I/O failure is another process holding the file.
///
/// The raw code is consulted before the kind because the kind is where the
/// information is lost: Windows maps a sharing violation onto
/// `PermissionDenied` in places, and a restore that told the user to grant a
/// permission when the real answer was "close the game" would send them to the
/// wrong dialog every single time.
fn maqful(sabab: &std::io::Error) -> bool {
    if let Some(raqm) = sabab.raw_os_error() {
        let mutabiq = if cfg!(windows) {
            raqm == RAMZ_MUSHARAKA || raqm == RAMZ_QUFL
        } else {
            raqm == RAMZ_NASS_MASHGHUL
        };
        if mutabiq {
            return true;
        }
    }
    // `ExecutableFileBusy` is std's own name for `ETXTBSY` and is the kind a
    // modern Linux or macOS build reports when a game's binary is running. It is
    // checked as well as the raw code rather than instead of it, because the
    // numeric value differs across the BSDs and the mapping is the part std
    // guarantees.
    matches!(
        sabab.kind(),
        std::io::ErrorKind::ExecutableFileBusy
            | std::io::ErrorKind::ResourceBusy
            | std::io::ErrorKind::WouldBlock
    )
}

/// A length as a `u64`, saturating on a platform where `usize` is wider — which
/// is none this product targets, and is still not a reason to write a cast the
/// compiler cannot prove.
#[must_use]
pub fn tul_u64(tul: usize) -> u64 {
    u64::try_from(tul).unwrap_or(u64::MAX)
}
