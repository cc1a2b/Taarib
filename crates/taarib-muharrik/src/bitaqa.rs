//! البطاقة — the per-game probe record, and the two stamps that stop a stored
//! conclusion from outliving its own truth.
//!
//! A capability report is expensive to produce and almost free to serve, so it
//! is produced once and read from the store for as long as it is trusted. How
//! long that is, exactly, is the whole subject of this module.
//!
//! A stored report is a claim about two things that both move: the code that
//! drew the conclusion, and the game the conclusion is about. When either one
//! moves, the record becomes a statement nothing downstream can check — because
//! nothing downstream looks at the game. Which adapter loads, which framework
//! is installed, which tier the interface promises the user, whether extraction
//! is even offered: every one of those reads the record and never re-examines
//! the install. A stale record is therefore not a stale cache entry. It is a
//! wrong answer with a confident evidence trail attached, and it stays wrong
//! until something here decides otherwise.
//!
//! So a record carries two stamps, and both are tested before it is believed:
//!
//! 1. **The probe's own version** — `imkaniyat::ISDAR_FAHS`, written into every
//!    report as `TaqreerImkaniyat::isdar_fahs`. A record stamped by an older
//!    probe than the one running is not trusted, at all, regardless of what it
//!    says.
//! 2. **A cheap signature over the game's install root**. A record whose
//!    signature does not match the directory as it is today is not trusted
//!    either, because a game that changed may be a game whose engine changed.
//!
//! [`hal_yahtaj_fahs_bil_basma`] applies both and answers with a
//! [`SababFahs`] — the *reason*, not a boolean, because the reason is what the
//! interface shows. [`hal_yahtaj_fahs`] is the same decision with the content
//! test left out, for a caller deciding over records it has already read out of
//! the store without touching a disk.
//!
//! # The version stamp, and the trap underneath it
//!
//! `imkaniyat::ISDAR_FAHS` is a plain counter. It has exactly one rule:
//!
//! > **Anyone who improves detection increments it, in the same commit.**
//!
//! A new detector, a corrected weight, a version string parsed one field
//! further, a text system that is now recognised where it was missed before —
//! every one of those changes what a probe would conclude, and every one of
//! them is worthless to a user who already has a report, unless the counter
//! moves. [`SijillBitaqat::qadeema`] finds every game whose report predates the
//! running probe in one indexed query, and startup re-probes them in the
//! background. That query is the only thing that carries a detection fix from
//! the developer's machine to somebody's library.
//!
//! Forgetting it is a real trap, and it hides well:
//!
//! - **It passes local testing.** A developer's database is fresh or nearly so,
//!   every game is probed by the new code on its first run, and the improvement
//!   is visible immediately — on the one machine where the bug cannot occur.
//! - **It passes review.** The diff that adds a detector looks complete on its
//!   own. Nothing about it points at a constant in another file.
//! - **It fails silently, only for users, and only for the games they already
//!   own.** Taarib keeps serving the conclusion the old code drew, forever. The
//!   bug report that follows says detection is still wrong about code that was
//!   fixed two releases ago, and the maintainer cannot reproduce it, because
//!   the maintainer's copy re-probed.
//!
//! Incrementing it when it was not strictly necessary costs a background
//! re-probe of the library, which is seconds. Not incrementing it when it was
//! necessary costs another release, and the users who never file a report just
//! keep the wrong answer. Increment it.
//!
//! # The directory signature, and exactly what it is worth
//!
//! The second stamp answers a different question: not "has the probe improved"
//! but "is this still the same install". It matters because a game that
//! updated is a game whose *engine version* may have moved, and engine version
//! is not a detail — Unity ships minor bumps routinely, and Phase 7's signature
//! database is keyed on exactly those version ranges. A report that names
//! `2021.3.16f1` for an install that is now `2021.3.29f1` sends the IL2CPP
//! resolver at byte patterns that are not there any more.
//!
//! [`basmat_jidhr`] is deliberately **not** a content hash. It is:
//!
//! - the executable's **size in bytes**,
//! - the executable's **modification time**, to the nanosecond the filesystem
//!   reports,
//! - the **number of entries directly under the install root**, saturating at
//!   [`crate::fahs::AQSA_MADAKHIL`],
//! - and a **recipe version**, so that changing this composition invalidates
//!   every stored signature by construction instead of silently comparing two
//!   different kinds of string.
//!
//! That is one `stat` and one `readdir`. It runs for every game in the library
//! at every launch, and it has to: a real BLAKE3 fingerprint over a sixty
//! gigabyte install is minutes of disk, per game, before the library can be
//! drawn. This signature costs microseconds.
//!
//! **What it catches.** A reinstall. A store update that ships a new player
//! executable — which is every engine version bump, because the engine *is* the
//! executable. A DLC, a mod loader, or a framework directory appearing or
//! disappearing at the top level. An install moved to another drive, which
//! changes the timestamp and costs one unnecessary re-probe, which is the
//! correct way for it to be wrong.
//!
//! **What it misses, stated honestly.**
//!
//! - A replacement executable of **the same size with the same modification
//!   time**. Installers that preserve timestamps, and in-place byte patches
//!   that keep the length and restore the mtime, both produce this. It is
//!   undetectable at this cost by construction.
//! - Any change **below the top level** that leaves the executable alone: a
//!   data-only update that rewrites `resources.assets` and touches nothing
//!   else.
//!
//! The second miss is the interesting one, and it is the reason this trade is
//! acceptable rather than merely cheap. A data-only update is precisely the
//! case where the engine did *not* change — the player binary is the engine,
//! and it is untouched — so the stored capability report is still correct, and
//! re-probing would only reconfirm it. What such an update *does* invalidate is
//! whether an installed patch still fits the game's text, and that question
//! belongs to `Basma`: a real BLAKE3 fingerprint over the text-bearing files,
//! computed by Phase 15 when a patch is installed or checked, not for every
//! game at launch. The two stamps answer two questions and neither pretends to
//! answer the other's.
//!
//! # Where the record actually lives
//!
//! In `taarib-makhzan`, and nowhere else. The `bitaqa_muharrik` table, its
//! columns, its `CHECK` constraints and its `bitaqa_bi_isdar_fahs` index are
//! all defined in that crate's `hijra`, and every statement that touches them
//! is in its `sijillat`. [`SijillBitaqat`] holds a `SijillMuharrik` and calls
//! it. There is no SQL in this file and there is no second spelling of the
//! schema here — a probe that knew the column names would be a probe that
//! breaks when the store migrates.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use taarib_makhzan::SijillMuharrik;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muharrik::{Daleel, Muharrik, Tabaqa, TaqreerImkaniyat};
use taarib_usus::khata::Natija;

use crate::fahs::AQSA_MADAKHIL;

/// The recipe version of the directory signature.
///
/// Part of the signature string itself. Changing what goes into a signature —
/// adding a field, changing the units of one, dropping one — must change this
/// too, and then every stored signature compares unequal to every fresh one and
/// the whole library re-probes once. That is the intended outcome: two strings
/// built by different recipes are not comparable, and quietly comparing them
/// would either miss every change or report one for every game.
pub const ISDAR_BASMA: u32 = 1;

// ---------------------------------------------------------------------------
// السبب — why a game is being examined
// ---------------------------------------------------------------------------

/// Why a probe is about to run, or why one is not needed.
///
/// Not a boolean, because this value is *shown*. The diagnostics screen
/// distinguishes a game that has never been examined from one examined by an
/// older Taarib from one whose files changed, and a user who sees their library
/// re-examining itself after an update is owed the actual reason rather than a
/// spinner. It is also what the log line carries, so that "why did this game
/// re-probe" is answerable from a diagnostics bundle months later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababFahs {
    /// The stored report stands. Nothing runs.
    LaHaja,
    /// No report exists for this game.
    AwwalMarra,
    /// The stored report was written by an older probe than the one running.
    IsdarAqdam {
        /// The probe version that wrote the stored report.
        mukhazzan: u32,
        /// The probe version running now.
        hali: u32,
    },
    /// The install root no longer matches the signature taken with the report.
    TaghyeerMuhtawa,
    /// The user asked for this game to be examined again.
    ///
    /// Never returned by [`hal_yahtaj_fahs`] or
    /// [`hal_yahtaj_fahs_bil_basma`], which decide from stored state only. It
    /// is constructed by whatever handles the re-examine action on the game's
    /// detail screen, so that a deliberate re-probe reads the same in the log
    /// and in the interface as an automatic one.
    TalabMustakhdim,
}

impl SababFahs {
    /// Whether a probe has to run.
    #[must_use]
    pub const fn yahtaj(self) -> bool {
        !matches!(self, Self::LaHaja)
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    ///
    /// Stable in the sense that matters: it is the same text across releases,
    /// so a query over a year of diagnostics counts the same thing throughout.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::LaHaja => "la_haja",
            Self::AwwalMarra => "awwal_marra",
            Self::IsdarAqdam { .. } => "isdar_aqdam",
            Self::TaghyeerMuhtawa => "taghyeer_muhtawa",
            Self::TalabMustakhdim => "talab_mustakhdim",
        }
    }

    /// The sentence the interface shows, in Arabic.
    #[must_use]
    pub fn arabi(self) -> String {
        match self {
            Self::LaHaja => "تقرير هذه اللعبة ما يزال صالحًا؛ لا حاجة لإعادة الفحص.".to_owned(),
            Self::AwwalMarra => "لم تُفحص هذه اللعبة من قبل.".to_owned(),
            Self::IsdarAqdam { mukhazzan, hali } => format!(
                "فُحصت هذه اللعبة بإصدار أقدم من كشف المحرّكات ({mukhazzan})، وهذا الإصدار \
                 ({hali}) يكشف أكثر منه، فستُفحص من جديد."
            ),
            Self::TaghyeerMuhtawa => {
                "تغيّرت ملفات اللعبة منذ آخر فحص — تحديث أو إعادة تثبيت — وقد يكون إصدار \
                 المحرّك تغيّر معها."
                    .to_owned()
            }
            Self::TalabMustakhdim => "طلبتَ إعادة فحص هذه اللعبة.".to_owned(),
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub fn injilizi(self) -> String {
        match self {
            Self::LaHaja => "The stored report is still valid; nothing to re-examine.".to_owned(),
            Self::AwwalMarra => "This game has not been examined before.".to_owned(),
            Self::IsdarAqdam { mukhazzan, hali } => format!(
                "This game was examined by an older engine detector (version {mukhazzan}). \
                 This build (version {hali}) detects more, so it will be examined again."
            ),
            Self::TaghyeerMuhtawa => {
                "The game's files changed since it was last examined — an update or a \
                 reinstall — and its engine version may have changed with them."
                    .to_owned()
            }
            Self::TalabMustakhdim => "You asked for this game to be examined again.".to_owned(),
        }
    }
}

// ---------------------------------------------------------------------------
// البطاقة — the record
// ---------------------------------------------------------------------------

/// One game's probe record: the report, and the two stamps that date it.
///
/// The record is one per game and always the current one. There is no history
/// here, deliberately — a superseded report describes a probe that no longer
/// exists, against an install that may no longer exist, and keeping it would
/// only invite something to read it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitaqatMuharrik {
    /// Which game this is about.
    pub luba: LubaId,
    /// The capability report itself, which also carries the probe version stamp
    /// in `TaqreerImkaniyat::isdar_fahs`.
    pub taqreer: TaqreerImkaniyat,
    /// The install root's signature as [`basmat_jidhr`] computed it at probe
    /// time, or `None` when the root could not be listed at all.
    ///
    /// `None` is not "unchanged" and not "changed": it is "unknown", and
    /// [`Self::taghayyar`] treats it as such rather than re-probing forever.
    pub basmat_jidhr: Option<String>,
    /// When the probe ran, RFC 3339.
    ///
    /// This mirrors `TaqreerImkaniyat::waqt` and is not an independent reading
    /// of the clock. The store writes the report's own timestamp into its
    /// `waqt` column, so a record whose two values disagreed would round-trip
    /// as the report's and quietly lose the other. There is one probe and one
    /// time it ran.
    pub waqt: String,
}

impl BitaqatMuharrik {
    /// Builds a record around a report the probe has just produced.
    ///
    /// Takes the timestamp from the report rather than reading a clock, for the
    /// reason documented on [`Self::waqt`].
    #[must_use]
    pub fn jadeeda(luba: LubaId, taqreer: TaqreerImkaniyat, basmat_jidhr: Option<String>) -> Self {
        let waqt = taqreer.waqt.clone();
        Self { luba, taqreer, basmat_jidhr, waqt }
    }

    /// The probe version that produced this record.
    #[must_use]
    pub const fn isdar_fahs(&self) -> u32 {
        self.taqreer.isdar_fahs
    }

    /// The identified engine.
    #[must_use]
    pub const fn muharrik(&self) -> &Muharrik {
        &self.taqreer.muharrik
    }

    /// The tier this game resolved to.
    #[must_use]
    pub const fn tabaqa(&self) -> Tabaqa {
        self.taqreer.tabaqa
    }

    /// Everything the probe observed, kept verbatim for the diagnostics bundle.
    ///
    /// A detection that cannot be explained cannot be improved, so the evidence
    /// travels with the record instead of being discarded once a conclusion was
    /// drawn from it.
    #[must_use]
    pub fn dalail(&self) -> &[Daleel] {
        &self.taqreer.muharrik.dalail
    }

    /// Whether the install root has changed since this record was written.
    ///
    /// Only a signature on both sides can answer this. When either side is
    /// absent the answer is `false` — *no evidence of change*, which is not the
    /// same claim as "unchanged", and is the only safe reading of it. The
    /// alternative is worse than it looks: a game whose root cannot be listed
    /// would have no stored signature, would therefore look changed at every
    /// launch, and would be re-probed at every launch forever, which is a loop
    /// that costs the user a slow startup and never converges on anything.
    #[must_use]
    pub fn taghayyar(&self, basmat_haliya: Option<&str>) -> bool {
        match (self.basmat_jidhr.as_deref(), basmat_haliya) {
            (Some(mukhazzana), Some(haliya)) => mukhazzana != haliya,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// بصمة الجذر — the cheap install-root signature
// ---------------------------------------------------------------------------

/// A cheap signature over a game's install root.
///
/// One `stat` of the executable and one `readdir` of the root: the executable's
/// size, its modification time, and how many entries sit directly under the
/// root, behind a recipe version. See this module's documentation for the full
/// account of what that catches and what it does not — in short, it catches
/// every change that replaces the player executable, which is every engine
/// version bump, and it does not catch a same-size same-mtime replacement or a
/// data-only update below the top level.
///
/// Returns `None` only when the root cannot be listed at all, which is a game
/// that is missing or unreadable rather than one that changed. A caller must
/// treat that as unknown, not as changed; [`BitaqatMuharrik::taghayyar`] does.
///
/// An absent or unreadable executable is not a failure: the entry count alone
/// still forms a valid signature, marked as such so that it never compares
/// equal to one that had an executable behind it.
#[must_use]
pub fn basmat_jidhr(jidhr: &Path, tanfidhi: Option<&Path>) -> Option<String> {
    let (adad, mushabbaa) = adad_madakhil(jidhr)?;
    let hadd = if mushabbaa { "+" } else { "" };

    Some(match basmat_tanfidhi(tanfidhi) {
        Some((hajm, thawani, nano)) => {
            format!("b{ISDAR_BASMA}:{hajm}:{thawani}.{nano:09}:{adad}{hadd}")
        }
        None => format!("b{ISDAR_BASMA}:-:-:{adad}{hadd}"),
    })
}

/// The executable's size and modification time, when both can be read.
///
/// Every failure collapses to `None` — no executable was named, the file is
/// gone, permission is denied, or the filesystem reports a modification time
/// before the epoch. None of those is worth an error: a signature without the
/// executable is a weaker signature, not an unusable one, and the alternative
/// would be a probe that refuses to record anything because one `stat` failed.
fn basmat_tanfidhi(tanfidhi: Option<&Path>) -> Option<(u64, u64, u32)> {
    let masar = tanfidhi?;
    let bayanat = std::fs::metadata(masar).ok()?;
    let mubaddal = bayanat.modified().ok()?;
    let mudda = mubaddal.duration_since(UNIX_EPOCH).ok()?;
    Some((bayanat.len(), mudda.as_secs(), mudda.subsec_nanos()))
}

/// How many entries sit directly under the root, and whether the count
/// saturated.
///
/// Depth one only: this is a signature, not a walk. Entries that fail to read
/// individually are still counted, because a slot that exists and cannot be
/// read is a slot that exists, and skipping it would make the count flicker
/// with transient permission errors.
///
/// The ceiling is [`crate::fahs::AQSA_MADAKHIL`], shared with the detectors
/// rather than invented again here. Above it the count stops discriminating —
/// a root with more entries than that reports the same number no matter how
/// many are added — which is a documented weakening of the signature for a
/// directory shape no real game install has.
fn adad_madakhil(jidhr: &Path) -> Option<(usize, bool)> {
    let madakhil = std::fs::read_dir(jidhr).ok()?;
    let mut adad: usize = 0;
    for _ in madakhil {
        adad = adad.saturating_add(1);
        if adad >= AQSA_MADAKHIL {
            return Some((adad, true));
        }
    }
    Some((adad, false))
}

// ---------------------------------------------------------------------------
// القرار — is a probe needed, and why
// ---------------------------------------------------------------------------

/// Whether a stored record still stands against the running probe version.
///
/// The version test only. Content is not consulted, because no signature was
/// offered — use [`hal_yahtaj_fahs_bil_basma`] to include it. This exists as
/// its own entry point because it touches no filesystem at all, which is what
/// a caller wants when it is deciding over a list of games it has already read
/// out of the store.
///
/// `isdar_hali` is `imkaniyat::ISDAR_FAHS`. It is passed in rather than read
/// here so that this decision can be replayed against any version — the
/// diagnostics screen shows what an older build would have concluded, and a
/// fixture can drive the whole ladder without pretending to be a release.
#[must_use]
pub fn hal_yahtaj_fahs(mukhazzana: Option<&BitaqatMuharrik>, isdar_hali: u32) -> SababFahs {
    hal_yahtaj_fahs_bil_basma(mukhazzana, isdar_hali, None)
}

/// Whether a stored record still stands, against both the running probe version
/// and the install root as it is now.
///
/// The order of the tests is the order of the answers, and it is not arbitrary.
/// The version test runs first: when a Taarib update improved detection *and*
/// the game updated, both are true, both produce the same work, and the more
/// useful thing to tell the user — and the more useful thing to find in a
/// diagnostics bundle — is that this build examines the game differently than
/// the last one did.
///
/// A record stamped with a version **newer** than the running probe is left
/// alone and reported as [`SababFahs::LaHaja`]. It was written by a build that
/// detects at least as much as this one, so re-probing would replace a better
/// conclusion with a worse one — and, worse, would do so on every launch, since
/// the newer build would then find its own report downgraded and re-probe it
/// back. A user running a beta beside a release must not have their library
/// ground between the two.
#[must_use]
pub fn hal_yahtaj_fahs_bil_basma(
    mukhazzana: Option<&BitaqatMuharrik>,
    isdar_hali: u32,
    basmat_haliya: Option<&str>,
) -> SababFahs {
    let Some(bitaqa) = mukhazzana else {
        return SababFahs::AwwalMarra;
    };

    let mukhazzan = bitaqa.isdar_fahs();
    if mukhazzan < isdar_hali {
        return SababFahs::IsdarAqdam { mukhazzan, hali: isdar_hali };
    }

    if bitaqa.taghayyar(basmat_haliya) {
        return SababFahs::TaghyeerMuhtawa;
    }

    SababFahs::LaHaja
}

// ---------------------------------------------------------------------------
// السجل — the record's storage, over the makhzan ledger
// ---------------------------------------------------------------------------

/// The probe record's ledger, over `taarib-makhzan`'s engine-report ledger.
///
/// Every statement this reaches lives in `taarib-makhzan`. What is added here
/// is the Phase 5 vocabulary — a record is a report *plus* the signature taken
/// with it, and the two are written and read together so that a caller cannot
/// record a conclusion and forget the stamp that dates it.
///
/// Built from a `SijillMuharrik`, which is built from a connection, so the
/// intended shape at a call site is one transaction covering the whole probe:
///
/// ```ignore
/// makhzan.bi_muamala(|muamala| {
///     SijillBitaqat::jadeed(SijillMuharrik::jadeed(muamala)).sajjil(&bitaqa)
/// })
/// ```
///
/// `taarib-makhzan` is synchronous and never spawns a thread of its own, so
/// every call through here happens on a blocking thread — `spawn_blocking` or
/// its Tauri equivalent — exactly as it does for every other ledger.
#[derive(Debug, Clone, Copy)]
pub struct SijillBitaqat<'a> {
    sijill: SijillMuharrik<'a>,
}

impl<'a> SijillBitaqat<'a> {
    /// Binds the record's ledger to the store's engine-report ledger.
    #[must_use]
    pub const fn jadeed(sijill: SijillMuharrik<'a>) -> Self {
        Self { sijill }
    }

    /// The record stored for one game, if there is one.
    ///
    /// Two reads: the report document and the signature that was taken with it.
    /// `None` means no probe has ever examined this game, which is exactly the
    /// state [`SababFahs::AwwalMarra`] describes.
    ///
    /// # Errors
    ///
    /// Fails when a query cannot run, and when the stored report does not parse
    /// back into a [`TaqreerImkaniyat`] — which means the row was written by a
    /// build whose report shape differed. That is refused rather than
    /// half-read: a partially understood capability report would send an
    /// adapter at a game on the strength of the fields that happened to survive.
    pub fn jalb(self, luba: LubaId) -> Natija<Option<BitaqatMuharrik>> {
        let Some(taqreer) = self.sijill.wahid(luba)? else {
            return Ok(None);
        };
        let basmat_jidhr = self.sijill.basmat_jidhr(luba)?;
        let waqt = taqreer.waqt.clone();
        Ok(Some(BitaqatMuharrik { luba, taqreer, basmat_jidhr, waqt }))
    }

    /// Writes a record, replacing whatever was stored for that game.
    ///
    /// The report and the signature are written together. A signature of `None`
    /// clears the stored one rather than leaving the previous probe's behind:
    /// the signature belongs to the report it was taken with, and comparing
    /// tomorrow's directory against a state no current record describes would
    /// announce a change nothing observed.
    ///
    /// The store's `basma_bina` column — the *build* fingerprint the report
    /// applies to — is deliberately left unset here. A Phase 5 probe never
    /// computes a `Basma`; that is a BLAKE3 pass over the text-bearing files
    /// and it belongs to the install path, not to a startup sweep. Recording
    /// the previous probe's build there would be worse than recording nothing,
    /// because it would claim this report was drawn from a build it never saw.
    ///
    /// # Errors
    ///
    /// Fails when the report cannot be serialized, when a value the schema
    /// constrains is out of range, when a statement is rejected, or when
    /// another writer holds the database past the busy timeout.
    pub fn sajjil(self, bitaqa: &BitaqatMuharrik) -> Natija<()> {
        self.sijill.sajjil(bitaqa.luba, &bitaqa.taqreer, None)?;
        self.sijill.sajjil_basmat_jidhr(bitaqa.luba, bitaqa.basmat_jidhr.as_deref())
    }

    /// Every game whose record predates the running probe.
    ///
    /// This is the query the version stamp exists for, and the only path a
    /// detection improvement has into a library that has already been scanned.
    /// One indexed read over `bitaqa_bi_isdar_fahs` — not a walk of every
    /// report, and not a deserialization of every document — so running it at
    /// every launch costs nothing on the overwhelmingly common day when it
    /// returns an empty list.
    ///
    /// The result is a work list, not an answer: startup re-probes those games
    /// in the background and writes fresh records. Until it does, the old
    /// records are still what the interface serves, because a game with no
    /// report at all shows the user less than a game with an outdated one.
    ///
    /// `isdar_hali` is `imkaniyat::ISDAR_FAHS`, and everything in the module
    /// documentation about incrementing it applies here: a detector improved
    /// without moving that constant produces an empty list forever.
    ///
    /// # Errors
    ///
    /// Fails when the query cannot run or a stored identity does not decode.
    pub fn qadeema(self, isdar_hali: u32) -> Natija<Vec<LubaId>> {
        self.sijill.qadima(isdar_hali)
    }

    /// Forgets one game's record entirely — the report and its signature.
    ///
    /// Deleted rather than blanked, so that [`Self::jalb`] answers `None` and
    /// the next decision is [`SababFahs::AwwalMarra`]. "Never examined" is the
    /// honest state to re-probe from; a record emptied field by field would be
    /// a record that still exists and still gets read.
    ///
    /// This is what the detail screen's re-examine action removes before
    /// probing, and what a caller uses when a game's install root has moved far
    /// enough that the stored report is about a directory that is no longer
    /// there.
    ///
    /// # Errors
    ///
    /// Fails when a statement is rejected or the database is held by another
    /// writer past the busy timeout.
    pub fn ihdhif(self, luba: LubaId) -> Natija<()> {
        self.sijill.ihdhif(luba)
    }
}
