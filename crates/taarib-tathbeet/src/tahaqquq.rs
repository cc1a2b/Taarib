//! التحقّق — is the patch still there, and if not, who took it away.
//!
//! An installation is a claim about a set of files, and the claim expires
//! without notice. A store pushes an update overnight, the launcher rewrites the
//! files it ships, and the patch the user installed last month is either sitting
//! on top of bytes it was never matched against or has been silently reverted. No
//! event fires. Nothing asks. The next thing that happens is the user launching a
//! game they believe is translated.
//!
//! This module answers the question that prevents that, and it insists on
//! separating three states that produce the same coarse symptom — "the file is
//! not what the manifest says" — and have three different correct responses.
//!
//! | state | what happened | what to offer |
//! | --- | --- | --- |
//! | [`NatijatTahaqquq::Salim`] | every recorded path is still exactly what Taarib wrote | nothing |
//! | [`NatijatTahaqquq::MustabdalMinAlmatjar`] | the launcher updated the game and overwrote patched files | reinstall the patch |
//! | [`NatijatTahaqquq::TaghyeerMustakhdim`] | the user, or another tool, changed a file Taarib had patched | restore, and warn that reinstalling would lose their change |
//! | [`NatijatTahaqquq::GhayrMuakkad`] | files drifted and the evidence does not decide between the two | say so, offer both, guess at neither |
//! | [`NatijatTahaqquq::Naqis`] | recorded paths are gone from disk entirely | repair or remove |
//!
//! The middle two matter because the wrong answer destroys something. Offering
//! "reinstall" for a file the user edited overwrites their edit and preserves it
//! as though it were the original — the patch's *own* backup then holds their
//! modified file, and the real original is gone for good. Offering "restore" for
//! a store update writes a stale original over a file the launcher just updated,
//! downgrading it. So the fourth row exists: when the evidence does not decide,
//! this module says it does not decide, rather than picking the more likely one
//! and being wrong for a minority of users in a way they cannot undo.
//!
//! ## How the two are told apart
//!
//! Neither is directly observable — Taarib does not know the store's new build's
//! fingerprints and cannot ask. What it has is the shape of the change, and the
//! two shapes are genuinely different:
//!
//! - **A store update rewrites many files in one pass.** Their modification times
//!   land within seconds or minutes of each other, and all of them are after the
//!   installation. So the drifted set is clustered in time and the cluster has
//!   more than one member.
//! - **A person edits one file.** One modification time, unrelated to any other,
//!   with every sibling file still exactly as Taarib wrote it.
//!
//! A third signal corroborates but does not decide: a store update rewrites the
//! files the game ships and leaves files it has never heard of alone, so the
//! patch's *additions* usually survive it intact. This is treated as
//! corroboration and not as proof, because some launchers' repair modes do delete
//! unknown files, and a rule that depended on it would misclassify those.
//!
//! When the clustering is ambiguous — two files a day apart, or one drifted file
//! whose siblings were never fingerprinted — the answer is
//! [`NatijatTahaqquq::GhayrMuakkad`].
//!
//! ## The sweep has to be cheap enough to run at startup
//!
//! Verifying every installed patch on launch means, in the naive form, hashing
//! several gigabytes per game. So the sweep compares **size and modification
//! time first, and hashes only when those disagree** with what was recorded
//! immediately after the write.
//!
//! **Why that is sound here.** The manifest does not record the size of the
//! buffer Taarib wrote; it records the size and mtime that the filesystem
//! reported *after* the rename ([`crate::bayan::SijillTaghyeer::hajm_maktub`] and
//! `waqt_maktub`). Every ordinary writer — a launcher's updater, an installer, a
//! text editor, a patch tool, a file copy — goes through the normal write path
//! and updates the modification time. A file whose size and mtime are both still
//! the pair observed at write time was therefore not rewritten by anything that
//! behaves like software. The cost of being wrong is bounded too: a false
//! "unchanged" here delays a discovery to the next full check, and every
//! operation that *acts* on the answer runs the full check first.
//!
//! **Where this ordering would not be sound**, and is deliberately not used:
//!
//! - **As a tamper check.** Modification times are writable. `utimensat` and
//!   `SetFileTime` are ordinary calls, and anything trying to hide a change
//!   restores the timestamp and pads to the same length. This is a change
//!   detector for software that is not trying to hide, which is all software.
//!   Signature verification lives in `taarib-khatm` and does not consult a clock.
//! - **Before destroying something.** [`tahaqquq_kamil`] hashes unconditionally
//!   and is what runs before an uninstall or a reinstall, because those decisions
//!   overwrite files and a wrong answer is unrecoverable.
//! - **On filesystems with coarse timestamps.** FAT32 stores modification times
//!   in two-second units, so a same-size rewrite inside one tick is invisible to
//!   this comparison. Games on USB sticks and SD cards are real, which is another
//!   reason the fast path is only ever the fast path.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use taarib_mustalahat::bina::Basma;
use taarib_usus::khata::Khutwa;

use crate::bayan::{
    BayanTathbeet, NawTaghyeer, NawTathbeet, SijillTaghyeer, Tathbeet, WaqtNizam, basma_bayt,
    basma_malaf, dakhil_aw_khata, nisbi_min, thawani_min_rfc3339,
};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io};
use crate::taraju::MawqiTathbeet;

/// How far apart two modification times may be and still count as one pass.
///
/// Fifteen minutes. A store update writes its files as fast as the disk allows,
/// but a large game's update spans minutes, and a machine that slept in the
/// middle of one spans more. Wider than a plausible burst and far narrower than
/// the gap between two unrelated edits by a person.
pub const NAFIDHAT_TAHDITH: i64 = 900;

/// How long after the installation a change has to land before it counts as a
/// change at all.
///
/// Sixty seconds. The installation itself writes files, and the manifest's own
/// timestamp is taken when the session opens rather than when the last file is
/// written, so a large patch legitimately finishes writing after the time it
/// records. Without this margin the patch's own last files would be classified as
/// having changed after their own installation.
pub const HADD_TAJADDUD: i64 = 60;

/// How many drifted files a cluster needs before it looks like one pass.
///
/// Two. One file is a person; two files written within a quarter of an hour of
/// each other, both after the installation, is a process working through a list.
pub const ADNA_MAJMUAA: usize = 2;

/// What verification found at one recorded path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatMalaf {
    /// The file is byte-for-byte what Taarib wrote. The patch is intact here.
    Mutabiq,

    /// The manifest records no fingerprint for what Taarib wrote here.
    ///
    /// The file was preserved and then not modified — a guard obtained and
    /// dropped. There is nothing to compare against and nothing wrong.
    GhayrMuhaqqaq,

    /// The file is byte-for-byte the original.
    ///
    /// The patch is not installed at this path any more, and nothing is damaged.
    /// A store reinstalling the exact file it shipped produces this, and so does
    /// a partially completed uninstall. Distinguished from
    /// [`HalatMalaf::Munharif`] because there is no ambiguity about what to do:
    /// the original is already there, and reinstalling would simply reapply the
    /// patch.
    Ustuidat,

    /// The file is neither what Taarib wrote nor the original.
    ///
    /// The only genuinely ambiguous state, and the reason this module has an
    /// attribution step rather than a per-file verdict. The modification time is
    /// carried because it is the evidence the attribution runs on.
    Munharif {
        /// The fingerprint the manifest recorded for what Taarib wrote.
        muallana: Basma,
        /// The fingerprint of what is there now.
        mahsuba: Basma,
        /// When the file was last modified, when the filesystem reported it.
        waqt: Option<WaqtNizam>,
    },

    /// Nothing is at the path any more.
    Mafqud,

    /// A directory Taarib created is still there.
    MujalladMawjud,

    /// A directory Taarib created is gone.
    MujalladMafqud,
}

impl HalatMalaf {
    /// Whether this state means the patch is still in place and undamaged.
    #[must_use]
    pub const fn salim(self) -> bool {
        matches!(
            self,
            Self::Mutabiq | Self::GhayrMuhaqqaq | Self::MujalladMawjud
        )
    }

    /// Whether this state means the file changed underneath the patch.
    #[must_use]
    pub const fn munharif(self) -> bool {
        matches!(self, Self::Munharif { .. })
    }

    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mutabiq => "intact",
            Self::GhayrMuhaqqaq => "not modified",
            Self::Ustuidat => "back to the original",
            Self::Munharif { .. } => "changed",
            Self::Mafqud => "missing",
            Self::MujalladMawjud => "directory present",
            Self::MujalladMafqud => "directory missing",
        }
    }
}

/// What the drifted files, taken together, appear to have been done by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SababInhiraf {
    /// Nothing drifted.
    LaShay,

    /// Several files changed together after the installation: a store update.
    Matjar {
        /// How many files fell in the largest cluster.
        adad: usize,
        /// The earliest modification time in that cluster, Unix seconds.
        bidaya: i64,
    },

    /// One file changed on its own, with its siblings untouched: a person.
    Mustakhdim {
        /// How many files changed this way.
        adad: usize,
    },

    /// Files changed and the evidence does not decide which.
    ///
    /// Reported rather than resolved. Both remedies destroy something when they
    /// are applied to the wrong cause, and a coin flip between them is not a
    /// diagnosis.
    GhayrMuakkad {
        /// How many files changed.
        adad: usize,
        /// Why the evidence did not decide.
        sabab: &'static str,
    },
}

impl SababInhiraf {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::LaShay => "nothing changed",
            Self::Matjar { .. } => "the store updated the game",
            Self::Mustakhdim { .. } => "someone changed a patched file",
            Self::GhayrMuakkad { .. } => "changed, cause not established",
        }
    }
}

/// The verdict for one installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NatijatTahaqquq {
    /// Every recorded path is still what Taarib wrote.
    Salim,

    /// The launcher updated the game over the patch. Reinstalling reapplies it.
    MustabdalMinAlmatjar,

    /// Someone changed a file the patch had written.
    ///
    /// Reinstalling would overwrite that change *and preserve it as the
    /// original*, which is the worst available outcome: the user's edit becomes
    /// the backup, and the file the game shipped is gone. So the offer is a
    /// restore, with the warning attached.
    TaghyeerMustakhdim,

    /// Files changed and the cause was not established.
    GhayrMuakkad,

    /// Recorded paths are missing from disk.
    ///
    /// The game was moved, partially deleted, or repaired by a launcher that
    /// removes unknown files. Nothing here can be fixed by reinstalling over
    /// whatever is left.
    Naqis,
}

impl NatijatTahaqquq {
    /// Whether nothing needs doing.
    #[must_use]
    pub const fn salim(self) -> bool {
        matches!(self, Self::Salim)
    }

    /// The action to offer the user.
    ///
    /// A value rather than advice text, so an interface turns it into one button
    /// and cannot present an outcome with nothing to do about it.
    #[must_use]
    pub const fn khutwa(self) -> Khutwa {
        match self {
            Self::Salim => Khutwa::LaShay,
            // The patch's own files are gone from the game; putting them back is
            // exactly a re-match against the new build.
            Self::MustabdalMinAlmatjar => Khutwa::IadatMutabaqaBina,
            // Restore, not reinstall. Reinstalling here preserves the user's
            // edited file as though it were the original.
            Self::TaghyeerMustakhdim => Khutwa::IlghaTathbeet,
            // Both remedies are on the table and neither is safe to pick
            // automatically, so the user is shown the evidence.
            Self::GhayrMuakkad => Khutwa::FathTashkhis,
            Self::Naqis => Khutwa::TahaqquqSalamatLuba,
        }
    }

    /// The sentence an Arabic-speaking user reads.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::Salim => "الترقيع سليم وكل ملفاته كما كُتبت.",
            Self::MustabdalMinAlmatjar => {
                "حدّث المتجر اللعبة وأعاد كتابة ملفات كان تعريب قد رقّعها. أعد المطابقة مع \
                 البناء الجديد لإرجاع الترجمة."
            },
            Self::TaghyeerMustakhdim => {
                "تغيّر ملف كان تعريب قد رقّعه، ولا يبدو أن المتجر هو من غيّره. إعادة التثبيت \
                 فوقه ستطمس تغييرك وتحفظه على أنه الأصل؛ الإزالة والإرجاع أسلم."
            },
            Self::GhayrMuakkad => {
                "تغيّرت ملفات مرقّعة ولم يتّضح سبب التغيير. لن يخمّن تعريب: كل من الحلّين \
                 يُتلف شيئًا إن كان التشخيص خاطئًا."
            },
            Self::Naqis => {
                "ملفات مسجّلة في بيان التثبيت لم تعد موجودة على القرص. تحقّق من سلامة ملفات \
                 اللعبة من متجرها."
            },
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::Salim => "The patch is intact; every file is still what Taarib wrote.",
            Self::MustabdalMinAlmatjar => {
                "The store updated this game and rewrote files Taarib had patched. Re-match \
                 the patch against the new build to bring the translation back."
            },
            Self::TaghyeerMustakhdim => {
                "A file Taarib had patched has changed, and it does not look like the store \
                 did it. Reinstalling over it would overwrite that change and preserve it as \
                 though it were the original file — uninstall and restore instead."
            },
            Self::GhayrMuakkad => {
                "Patched files have changed and the cause was not established. Taarib will \
                 not guess: each remedy destroys something if the diagnosis is wrong."
            },
            Self::Naqis => {
                "Paths recorded in the installation manifest are no longer on disk. Verify \
                 the game's files through its launcher."
            },
        }
    }
}

/// What verification found across one installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerTahaqquq {
    /// Which installation was checked.
    pub naw: NawTathbeet,

    /// The game.
    pub luba: String,

    /// When it was installed, RFC 3339, as the manifest recorded it.
    pub waqt_tathbeet: String,

    /// One state per recorded path, in ascending path order.
    pub halat: BTreeMap<String, HalatMalaf>,

    /// Files found inside directories Taarib created that the manifest does not
    /// mention.
    ///
    /// Not an error. It is usually a translator's own work, and it is the reason
    /// an uninstall removes a created directory only when it is empty.
    pub zaida: Vec<String>,

    /// How many paths changed.
    pub adad_munharif: usize,

    /// How many paths are gone.
    pub adad_mafqud: usize,

    /// How many paths are back to their original bytes.
    pub adad_mustaad: usize,

    /// How many paths had to be hashed, out of the whole set.
    ///
    /// The measure of whether the fast path is earning its keep. A startup sweep
    /// over an intact installation should report zero here, and one that reports
    /// the full count on every run means the recorded sizes and times are not
    /// surviving something on the user's machine — a synchronising folder, an
    /// antivirus that rewrites files, a filesystem with coarse timestamps.
    pub adad_mahsub: usize,

    /// What the changed files, taken together, appear to have been done by.
    pub sabab: SababInhiraf,
}

impl TaqreerTahaqquq {
    /// The verdict.
    #[must_use]
    pub const fn natija(&self) -> NatijatTahaqquq {
        if self.adad_mafqud > 0 {
            return NatijatTahaqquq::Naqis;
        }
        match self.sabab {
            SababInhiraf::LaShay => NatijatTahaqquq::Salim,
            SababInhiraf::Matjar { .. } => NatijatTahaqquq::MustabdalMinAlmatjar,
            SababInhiraf::Mustakhdim { .. } => NatijatTahaqquq::TaghyeerMustakhdim,
            SababInhiraf::GhayrMuakkad { .. } => NatijatTahaqquq::GhayrMuakkad,
        }
    }

    /// Whether every recorded path is still what Taarib wrote.
    #[must_use]
    pub const fn salim(&self) -> bool {
        self.adad_munharif == 0 && self.adad_mafqud == 0 && self.adad_mustaad == 0
    }

    /// Every path that is not in the state the manifest describes.
    #[must_use]
    pub fn masarat_munharifa(&self) -> Vec<&str> {
        self.halat
            .iter()
            .filter(|(_, hala)| !hala.salim())
            .map(|(masar, _)| masar.as_str())
            .collect()
    }

    /// The result as lines for the log, the bundle and the game's detail screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.halat.len().saturating_add(4));
        sutur.push(format!(
            "{} [{}] installed {}: {} path(s) checked, {} hashed, {} changed, {} missing, {} \
             back to the original",
            self.luba,
            self.naw.ism(),
            self.waqt_tathbeet,
            self.halat.len(),
            self.adad_mahsub,
            self.adad_munharif,
            self.adad_mafqud,
            self.adad_mustaad
        ));
        sutur.push(format!("  verdict: {}", self.sabab.ism()));
        if let SababInhiraf::GhayrMuakkad { sabab, .. } = self.sabab {
            sutur.push(format!("  undecided because {sabab}"));
        }
        for (masar, hala) in &self.halat {
            if !hala.salim() {
                sutur.push(format!("  {masar}: {}", hala.ism()));
            }
        }
        for masar in &self.zaida {
            sutur.push(format!("  {masar}: present and not in the manifest"));
        }
        sutur.push(format!("  {}", self.natija().injilizi()));
        sutur
    }
}

/// One game's place in a library-wide verification.
#[derive(Debug)]
pub struct NatijatFahsLuba {
    /// The game.
    pub ism: String,
    /// Where it is.
    pub jidhr_luba: PathBuf,
    /// The text installation's report, or [`None`] when there is none.
    pub nass: Option<NatijatTathbeet<TaqreerTahaqquq>>,
    /// The voice installation's report, or [`None`] when there is none.
    pub sawt: Option<NatijatTathbeet<TaqreerTahaqquq>>,
}

impl NatijatFahsLuba {
    /// Whether every installation that is present is intact.
    #[must_use]
    pub fn salim(&self) -> bool {
        let salim = |natija: &Option<NatijatTathbeet<TaqreerTahaqquq>>| {
            natija
                .as_ref()
                .is_none_or(|n| n.as_ref().is_ok_and(TaqreerTahaqquq::salim))
        };
        salim(&self.nass) && salim(&self.sawt)
    }

    /// Whether anything is installed at all.
    #[must_use]
    pub const fn wujidat(&self) -> bool {
        self.nass.is_some() || self.sawt.is_some()
    }

    /// Every report that was produced, text before voice.
    #[must_use]
    pub fn taqareer(&self) -> Vec<&TaqreerTahaqquq> {
        let mut taqareer = Vec::new();
        if let Some(Ok(taqreer)) = &self.nass {
            taqareer.push(taqreer);
        }
        if let Some(Ok(taqreer)) = &self.sawt {
            taqareer.push(taqreer);
        }
        taqareer
    }
}

/// The whole library sweep.
#[derive(Debug)]
pub struct TaqreerFahs {
    /// One entry per game, in the order they were given.
    pub alaab: Vec<NatijatFahsLuba>,
}

impl TaqreerFahs {
    /// How many games are fully intact.
    #[must_use]
    pub fn adad_salim(&self) -> usize {
        self.alaab.iter().filter(|luba| luba.salim()).count()
    }

    /// How many games need something done.
    #[must_use]
    pub fn adad_yahtaj(&self) -> usize {
        self.alaab
            .iter()
            .filter(|luba| luba.wujidat() && !luba.salim())
            .count()
    }

    /// Every game that needs something, with the action to offer for it.
    ///
    /// The list an interface turns into a notification. Only games with a
    /// readable manifest and a real verdict appear; a game whose manifest could
    /// not be read is a separate problem, reported by
    /// [`TaqreerFahs::taqreer`] rather than folded in here as though it were an
    /// out-of-date patch.
    #[must_use]
    pub fn yahtaj(&self) -> Vec<(&str, NatijatTahaqquq)> {
        let mut qaima = Vec::new();
        for luba in &self.alaab {
            for taqreer in luba.taqareer() {
                let natija = taqreer.natija();
                if !natija.salim() {
                    qaima.push((luba.ism.as_str(), natija));
                }
            }
        }
        qaima
    }

    /// The whole sweep as lines, failures named rather than counted.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "{} game(s) verified: {} intact, {} need attention",
            self.alaab.len(),
            self.adad_salim(),
            self.adad_yahtaj()
        )];
        for luba in &self.alaab {
            for (naw, natija) in [
                (NawTathbeet::Nass, &luba.nass),
                (NawTathbeet::Sawt, &luba.sawt),
            ] {
                match natija {
                    None => {},
                    Some(Ok(taqreer)) => sutur.extend(taqreer.taqreer()),
                    Some(Err(khata)) => sutur.push(format!(
                        "{} [{}] could not be verified: {}",
                        luba.ism,
                        naw.ism(),
                        taarib_usus::khata::Tafsir::injilizi(khata)
                    )),
                }
            }
        }
        sutur
    }
}

// ---------------------------------------------------------------------------
// Verifying
// ---------------------------------------------------------------------------

/// Verifies one installation, hashing only what the cheap comparison flags.
///
/// The startup form. Every recorded path is compared by size and modification
/// time against what was observed immediately after the write, and only the paths
/// where that disagrees are hashed. See this module's header for exactly why that
/// ordering is sound for change detection and why it is never used before an
/// operation that overwrites something.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] or [`KhataTathbeet::IsdarBayanMajhul`] when the
/// manifest cannot be read or trusted, [`KhataTathbeet::MasarKharij`] when it
/// names a path outside the game, and the I/O variants when a path exists and
/// cannot be hashed — which during a sweep usually means the game is running and
/// holds its own files open.
pub fn tahaqquq_luba(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
) -> NatijatTathbeet<TaqreerTahaqquq> {
    ifhas(jidhr_luba, jidhr_nusakh, naw, false)
}

/// Verifies one installation, hashing every recorded path unconditionally.
///
/// What runs before an uninstall, before a reinstall, and whenever the answer is
/// about to be acted on rather than displayed. The size-and-time comparison is a
/// change detector for well-behaved software; a decision that overwrites a user's
/// game is not allowed to rest on one.
///
/// # Errors
///
/// As [`tahaqquq_luba`].
pub fn tahaqquq_kamil(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
) -> NatijatTathbeet<TaqreerTahaqquq> {
    ifhas(jidhr_luba, jidhr_nusakh, naw, true)
}

/// Verifies every game it was given, continuing past a failure.
///
/// A sweep that stopped at the first unreadable manifest would tell a user with
/// forty patched games nothing about thirty-nine of them. Every game is
/// attempted, every outcome is kept whole, and a game whose manifest cannot be
/// read is reported as that rather than as an out-of-date patch.
///
/// `daqiq` selects [`tahaqquq_kamil`] over [`tahaqquq_luba`] for every game; the
/// startup sweep passes `false` and the "verify everything" button passes `true`.
#[must_use]
pub fn tahaqquq_al_maktaba(mawaqi: &[MawqiTathbeet], daqiq: bool) -> TaqreerFahs {
    let mut alaab = Vec::with_capacity(mawaqi.len());
    for mawqi in mawaqi {
        let ifhas_wahid = |naw: NawTathbeet| {
            Tathbeet::mawjud(&mawqi.jidhr_nusakh, naw)
                .then(|| ifhas(&mawqi.jidhr_luba, &mawqi.jidhr_nusakh, naw, daqiq))
        };
        alaab.push(NatijatFahsLuba {
            ism: mawqi.ism.clone(),
            jidhr_luba: mawqi.jidhr_luba.clone(),
            nass: ifhas_wahid(NawTathbeet::Nass),
            sawt: ifhas_wahid(NawTathbeet::Sawt),
        });
    }
    TaqreerFahs { alaab }
}

/// The one body behind both verification entry points.
fn ifhas(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
    daqiq: bool,
) -> NatijatTathbeet<TaqreerTahaqquq> {
    let tathbeet = Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw)?;
    let bayan = tathbeet.bayan();

    let mut taqreer = TaqreerTahaqquq {
        naw,
        luba: bayan.ism_luba.clone(),
        waqt_tathbeet: bayan.waqt.clone(),
        halat: BTreeMap::new(),
        zaida: Vec::new(),
        adad_munharif: 0,
        adad_mafqud: 0,
        adad_mustaad: 0,
        adad_mahsub: 0,
        sabab: SababInhiraf::LaShay,
    };

    for (masar, sijill) in &bayan.sijillat {
        let mutlaq = dakhil_aw_khata(jidhr_luba, masar)?;
        let hala = hala_masar(sijill, &mutlaq, daqiq, &mut taqreer.adad_mahsub)?;

        match hala {
            HalatMalaf::Munharif { .. } => {
                taqreer.adad_munharif = taqreer.adad_munharif.saturating_add(1);
            },
            HalatMalaf::Mafqud | HalatMalaf::MujalladMafqud => {
                taqreer.adad_mafqud = taqreer.adad_mafqud.saturating_add(1);
            },
            HalatMalaf::Ustuidat => {
                taqreer.adad_mustaad = taqreer.adad_mustaad.saturating_add(1);
            },
            HalatMalaf::Mutabiq | HalatMalaf::GhayrMuhaqqaq | HalatMalaf::MujalladMawjud => {},
        }
        let _ = taqreer.halat.insert(masar.clone(), hala);
    }

    taqreer.zaida = ihsa_zaida(bayan, jidhr_luba);
    taqreer.sabab = nasib(bayan, &taqreer.halat);
    Ok(taqreer)
}

/// Decides one path's state, hashing only when it has to.
///
/// `adad_mahsub` is incremented whenever a hash was actually computed, so the
/// report can say how much work the fast path saved — and, more usefully, so a
/// machine where the fast path never hits shows up as such instead of just being
/// slow.
fn hala_masar(
    sijill: &SijillTaghyeer,
    mutlaq: &Path,
    daqiq: bool,
    adad_mahsub: &mut usize,
) -> NatijatTathbeet<HalatMalaf> {
    if matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
        return Ok(if mutlaq.is_dir() {
            HalatMalaf::MujalladMawjud
        } else {
            HalatMalaf::MujalladMafqud
        });
    }

    let Ok(bayanat) = fs::metadata(mutlaq) else {
        return Ok(HalatMalaf::Mafqud);
    };
    if !bayanat.is_file() {
        return Ok(HalatMalaf::Mafqud);
    }

    let Some(muallana) = sijill.basma_maktuba else {
        // Preserved through a guard that was dropped without writing. There is
        // nothing recorded to compare against, and nothing wrong.
        return Ok(HalatMalaf::GhayrMuhaqqaq);
    };

    if !daqiq && sijill.yutabiq_bila_basma(&bayanat) {
        return Ok(HalatMalaf::Mutabiq);
    }

    *adad_mahsub = adad_mahsub.saturating_add(1);
    let mahsuba = basma_malaf(mutlaq)?;
    if mahsuba == muallana {
        return Ok(HalatMalaf::Mutabiq);
    }
    if sijill.basma_asliya == Some(mahsuba) {
        return Ok(HalatMalaf::Ustuidat);
    }
    Ok(HalatMalaf::Munharif {
        muallana,
        mahsuba,
        waqt: bayanat.modified().ok().and_then(WaqtNizam::min_nizam),
    })
}

/// Lists files inside directories Taarib created that the manifest does not
/// mention.
///
/// Never an error and never counted against the patch. A translator who drops
/// their own work into `game/tl/arabic/` should find it there afterwards, and
/// this is the list that stops an uninstall from removing the directory around
/// it.
fn ihsa_zaida(bayan: &BayanTathbeet, jidhr_luba: &Path) -> Vec<String> {
    let mut zaida = Vec::new();
    for (masar, sijill) in &bayan.sijillat {
        if !matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
            continue;
        }
        let Ok(mutlaq) = dakhil_aw_khata(jidhr_luba, masar) else {
            continue;
        };
        for madkhal in walkdir::WalkDir::new(&mutlaq)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !madkhal.file_type().is_file() {
                continue;
            }
            let Some(nisbi) = nisbi_min(jidhr_luba, madkhal.path()) else {
                continue;
            };
            if !bayan.sijillat.contains_key(&nisbi) {
                zaida.push(nisbi);
            }
        }
    }
    zaida.sort();
    zaida.dedup();
    zaida
}

// ---------------------------------------------------------------------------
// Attribution
// ---------------------------------------------------------------------------

/// Decides what the changed files, taken together, were changed by.
///
/// Runs on the whole set rather than per file, because the discriminating signal
/// *is* the shape of the set: one file is a person, a cluster is a process. A
/// per-file verdict cannot see the difference and would have to guess on every
/// one of them independently.
fn nasib(bayan: &BayanTathbeet, halat: &BTreeMap<String, HalatMalaf>) -> SababInhiraf {
    let mut awqat: Vec<i64> = Vec::new();
    let mut bila_waqt = 0_usize;
    let mut adad = 0_usize;

    for hala in halat.values() {
        let HalatMalaf::Munharif { waqt, .. } = hala else {
            continue;
        };
        adad = adad.saturating_add(1);
        match waqt {
            Some(waqt) => awqat.push(waqt.thawani),
            None => bila_waqt = bila_waqt.saturating_add(1),
        }
    }

    if adad == 0 {
        return SababInhiraf::LaShay;
    }

    let Some(waqt_tathbeet) = thawani_min_rfc3339(&bayan.waqt) else {
        return SababInhiraf::GhayrMuakkad {
            adad,
            sabab: "the manifest's own installation timestamp is not a form this build \
                    parses, so there is no reference point to measure a change against",
        };
    };
    if bila_waqt > 0 {
        return SababInhiraf::GhayrMuakkad {
            adad,
            sabab: "the filesystem reported no modification time for at least one changed \
                    file, and the shape of the change cannot be read without one",
        };
    }

    // Anything at or before the installation was not changed by the
    // installation's aftermath at all — it was already like that, or the clock
    // moved. Either way it is not evidence about a store update.
    let hadd = waqt_tathbeet.saturating_add(HADD_TAJADDUD);
    let mut bad_al_tathbeet: Vec<i64> = awqat.iter().copied().filter(|waqt| *waqt > hadd).collect();
    if bad_al_tathbeet.len() < adad {
        return SababInhiraf::GhayrMuakkad {
            adad,
            sabab: "at least one changed file is stamped as older than the installation \
                    itself, which means a clock moved or the file's date was rewritten",
        };
    }

    bad_al_tathbeet.sort_unstable();
    let (akbar, bidaya) = akbar_majmuaa(&bad_al_tathbeet);

    // A store update rewrites the files a game ships and leaves files it has
    // never heard of alone, so surviving additions corroborate the store
    // reading. It is corroboration and not proof: some launchers' repair modes
    // do delete unknown files, and a rule that required this would misread
    // those.
    let idafat_salima = halat
        .iter()
        .filter(|(masar, _)| {
            bayan
                .sijillat
                .get(*masar)
                .is_some_and(|s| matches!(s.naw, NawTaghyeer::Idafa))
        })
        .all(|(_, hala)| hala.salim());

    if akbar >= ADNA_MAJMUAA {
        if idafat_salima {
            return SababInhiraf::Matjar {
                adad: akbar,
                bidaya,
            };
        }
        return SababInhiraf::GhayrMuakkad {
            adad,
            sabab: "several files changed together, which looks like a store update, but \
                    files Taarib added were also changed or removed, which a store update \
                    does not normally do",
        };
    }

    if adad == 1 && idafat_salima {
        return SababInhiraf::Mustakhdim { adad };
    }

    SababInhiraf::GhayrMuakkad {
        adad,
        sabab: "the changed files are spread too far apart in time to be one pass and are \
                too many to be a single edit",
    }
}

/// The size and start of the largest cluster of timestamps within
/// [`NAFIDHAT_TAHDITH`] of each other.
///
/// A single forward pass over a sorted list: extend the current window while the
/// next value is within the window of the window's *start*, otherwise begin a new
/// one. Measuring against the start rather than the previous element is
/// deliberate — chaining from neighbour to neighbour would let a long slow trickle
/// of unrelated edits, each fourteen minutes after the last, accumulate into one
/// enormous "cluster" and read as a store update.
fn akbar_majmuaa(awqat: &[i64]) -> (usize, i64) {
    let mut akbar = 0_usize;
    let mut bidaya_akbar = 0_i64;
    let mut awwal = 0_usize;

    for (akhir, waqt) in awqat.iter().enumerate() {
        while let Some(bidaya) = awqat.get(awwal) {
            if waqt.saturating_sub(*bidaya) <= NAFIDHAT_TAHDITH {
                break;
            }
            awwal = awwal.saturating_add(1);
        }
        let tul = akhir.saturating_sub(awwal).saturating_add(1);
        if tul > akbar {
            akbar = tul;
            bidaya_akbar = awqat.get(awwal).copied().unwrap_or(*waqt);
        }
    }

    (akbar, bidaya_akbar)
}

// ---------------------------------------------------------------------------
// Reading a manifest that may not be there
// ---------------------------------------------------------------------------

/// Which installations a game actually has.
///
/// Answered by looking for the two manifest files and nothing else — no manifest
/// is parsed, no game file is touched. What a library view calls for every game
/// on every render.
#[must_use]
pub fn anwa_mutahabbata(jidhr_nusakh: &Path) -> Vec<NawTathbeet> {
    NawTathbeet::KULL
        .into_iter()
        .filter(|naw| Tathbeet::mawjud(jidhr_nusakh, *naw))
        .collect()
}

/// How much disk both installations' backups occupy for one game.
///
/// The number a "free up space" screen shows. Read from the manifests' recorded
/// compressed sizes rather than by walking the backup directory, so it costs two
/// small reads instead of a directory traversal per game.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] when a manifest that exists cannot be read. A
/// manifest that is simply absent contributes zero and is not an error.
pub fn hajm_nusakh_luba(jidhr_nusakh: &Path) -> NatijatTathbeet<u64> {
    let mut majmu = 0_u64;
    for naw in NawTathbeet::KULL {
        if !Tathbeet::mawjud(jidhr_nusakh, naw) {
            continue;
        }
        let masar_bayan = jidhr_nusakh.join(naw.ism_bayan());
        let bayan: BayanTathbeet =
            taarib_usus::mukhattat::iqra_malaf(&masar_bayan).map_err(|khata| {
                KhataTathbeet::BayanTalif {
                    masar: masar_bayan.clone(),
                    sabab: khata.injilizi,
                }
            })?;
        majmu = majmu.saturating_add(bayan.hajm_nusakh());
    }
    Ok(majmu)
}

/// Confirms that a backup directory holds everything its manifests promise.
///
/// Checks that every backup key a manifest names is present and expands to the
/// recorded fingerprint, without touching the game at all. This is the check that
/// answers "could I still uninstall this if I wanted to?", and it is worth
/// running on its own schedule: a `nusakh/` directory emptied by a disk cleaner
/// is discovered here, while the patch is still installed and the originals could
/// still be re-obtained from the launcher — rather than during an uninstall, when
/// the answer is that they cannot.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] when a manifest cannot be read. A missing or
/// corrupt *backup* is returned in the list rather than as an error, because the
/// point of the call is to enumerate them all rather than stop at the first.
pub fn tahaqquq_nusakh(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    naw: NawTathbeet,
) -> NatijatTathbeet<Vec<KhataTathbeet>> {
    let tathbeet = Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw)?;
    let mut akhta = Vec::new();

    for (masar, sijill) in &tathbeet.bayan().sijillat {
        let (Some(miftah), Some(muallana)) = (sijill.nuskha.as_deref(), sijill.basma_asliya) else {
            continue;
        };
        let mutlaq = match dakhil_aw_khata(jidhr_luba, masar) {
            Ok(mutlaq) => mutlaq,
            Err(khata) => {
                akhta.push(khata);
                continue;
            },
        };
        match tathbeet.iqra_nuskha(miftah, &mutlaq) {
            Err(khata) => akhta.push(khata),
            Ok(asli) => {
                let mahsuba = basma_bayt(&asli);
                if mahsuba != muallana {
                    akhta.push(KhataTathbeet::NuskhaTalifa {
                        masar: mutlaq,
                        miftah: miftah.to_owned(),
                        muallana: muallana.to_string(),
                        mahsuba: mahsuba.to_string(),
                    });
                }
            },
        }
    }

    Ok(akhta)
}

/// Turns an I/O failure met while verifying into this crate's vocabulary.
///
/// Exposed because the sweep is the one operation a caller may want to run
/// against a directory it discovered itself rather than one Taarib recorded, and
/// a caller doing that needs the same classification — a game holding its own
/// files open must be reported as a lock, not as a mysterious read failure.
#[must_use]
pub fn khata_fahs(masar: &Path, sabab: std::io::Error) -> KhataTathbeet {
    min_khata_io(masar, "verifying an installed patch", sabab)
}
