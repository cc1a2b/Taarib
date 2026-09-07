//! الحفظ — preservation: the single mechanism through which every patcher in
//! this crate is allowed to touch a game.
//!
//! The other five modules here rewrite files that *are* the game. A `data.win`,
//! an `app.asar`, a `Scripts.rvdata2`, a `data/Map001.json` — none of these is a
//! configuration file that can be regenerated. If Taarib modifies one and cannot
//! put it back, the player's only remedy is to redownload the game, and for a
//! game bought once on a dead storefront there may not be one. So no patcher
//! opens a game file for writing. They all come through here.
//!
//! ## The contract, in the order it must happen
//!
//! **1. Preserve before touching.** [`Hifz::ihfaz`] copies a file's original
//! bytes into the backup directory and records, in the manifest, its relative
//! path, its original size, its original BLAKE3 fingerprint, and — the field
//! that is most often got wrong — whether the file existed at all beforehand.
//! A file Taarib *added* has no original: undoing it means deleting it, not
//! restoring it. Conflating the two is how an uninstall leaves litter behind, or
//! worse, writes an empty "original" over a file the game shipped.
//!
//! **2. The manifest entry is durable before the file is modified.** Not after,
//! and not in the same step. If the order were reversed, a patch interrupted
//! between the write and the record would leave a modified game file with
//! nothing on disk saying it had been modified — no backup key to find, no
//! original fingerprint to compare against, no way for any later run to tell a
//! patched file from a file the game shipped. That state is not recoverable by
//! any amount of cleverness afterwards, which is why the ordering is enforced
//! here rather than left to five callers to remember. The reverse ordering — a
//! manifest entry for a file that was never modified — is harmless: a restore
//! writes the original over an identical original and verifies fine.
//!
//! **3. Every write is atomic.** Bytes go beside the target, are flushed to the
//! device, and are renamed over it, through [`taarib_usus::masarat::kitaba_dharra`].
//! There is no second atomic-write implementation in this crate, deliberately:
//! two of them would be two sets of fsync ordering rules, and the one used less
//! often would be the one that was wrong. A half-written `data.win` is not a
//! damaged patch, it is an unplayable game.
//!
//! **4. Additive delivery is preferred wherever the engine allows it.** Ren'Py
//! reads `game/tl/arabic/*.rpy` that were not there before; RPG Maker loads
//! `js/plugins/taarib.js`; VX Ace takes an appended script entry. Where the
//! payload can be a *new* file, [`Hifz::adif`] records an addition and uninstall
//! is a delete — no original to hold, no fingerprint to reproduce, nothing to
//! get wrong. It also survives the thing that kills most patches: a game update
//! overwrites the files it ships and leaves files it has never heard of alone, so
//! an additive patch frequently still works after an update that would have
//! silently reverted a modification.
//!
//! **5. Restores are verified, never assumed.** [`Hifz::istiada`] writes each
//! original back and then re-hashes what landed on disk. A file whose
//! fingerprint does not match the recorded one is
//! [`KhataNusus::IstiadaGhayrMutabaqa`] and the uninstall stops there — reporting
//! a restore that did not happen is worse than reporting a failure, because the
//! user then believes their game is clean. Restoring is idempotent and each
//! completed file is marked in the manifest before the next one starts, so an
//! uninstall interrupted halfway is resumed rather than restarted.
//!
//! ## The guard makes rule 1 structural
//!
//! [`HarisKitaba`] has no public constructor and no public fields, and it is the
//! only type in this crate with a method that writes into a game directory.
//! [`Hifz`] exposes no `write(path, bytes)`. The only way to obtain a guard is to
//! call [`Hifz::ihfaz`] or [`Hifz::adif`] and have them succeed, which means the
//! backup exists and the manifest entry naming it is already on the device. A
//! patcher that tries to modify a file it has not preserved does not fail a
//! review or trip an assertion — it fails to compile, because the function it
//! would need does not exist. This is the same standard as `naql.rs`: an
//! invariant stated in a comment survives exactly as long as the next person who
//! edits around it.
//!
//! ## What this module does not decide
//!
//! It does not decide *what* to write — that is each engine's adapter — and it
//! does not own the installations ledger, which lives in `taarib-makhzan` and
//! records the same facts in `SQLite` for the library view. The manifest here is
//! the on-disk source of truth that works when the database does not: a user who
//! copies their game and their `nusakh/` directory to another machine can still
//! uninstall.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taarib_mustalahat::bina::Basma;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::khata::{KhataNusus, tul_u64};

/// The file name of the manifest inside an installation's backup directory.
pub const ISM_BAYAN: &str = "bayan.json";

/// The subdirectory of an installation's backup directory holding originals.
pub const ISM_MUJALLAD_ASL: &str = "asl";

/// The largest original this build will preserve, in bytes.
///
/// Two gibibytes. The ceiling exists because preserving a file means holding its
/// bytes in memory long enough to write them beside the backup and rename — the
/// atomic write in [`taarib_usus::masarat`] takes a slice, and a copy that was
/// not atomic would defeat the point of having a backup at all. It is checked
/// against the size the filesystem *declares*, before a single byte is read, so
/// a pathological file is a refusal rather than an exhausted machine.
///
/// No engine this crate patches ships a single text-bearing file near this. A
/// game that does is refused with [`KhataNusus::HajmMufrit`] naming the number,
/// which is a report somebody can act on.
pub const AQSA_HAJM_NUSKHA: u64 = 2_147_483_648;

/// The longest prefix of a file's own name kept in a backup key.
const HADD_ISM_MIFTAH: usize = 40;

/// The BLAKE3 fingerprint of a buffer.
///
/// The same function the manifest, the restore check and the drift check all
/// use. One definition, so a fingerprint recorded at install and a fingerprint
/// computed at uninstall cannot be computed two different ways.
#[must_use]
pub fn basma_bayt(bayt: &[u8]) -> Basma {
    Basma::min_bayt(*blake3::hash(bayt).as_bytes())
}

/// The BLAKE3 fingerprint of a file on disk, without reading it into the heap.
///
/// Mapped rather than read, because verification runs over every file a patch
/// touched and a `data.win` is not something to load in order to hash. The
/// mapping is read-only and lives for the length of this call.
///
/// # Errors
///
/// [`KhataNusus::KhataMalaf`] naming the path when it cannot be opened or
/// mapped, which for a verification pass usually means the game was moved or
/// uninstalled underneath the patch.
pub fn basma_malaf(masar: &Path) -> Result<Basma, KhataNusus> {
    let mut hashib = blake3::Hasher::new();
    hashib
        .update_mmap(masar)
        .map_err(|sabab| KhataNusus::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })?;
    Ok(Basma::min_bayt(*hashib.finalize().as_bytes()))
}

/// What a patch did to one path.
///
/// The distinction between the first two is the whole reversibility contract.
/// Uninstalling a [`NawTaghyeer::Tadeel`] means putting a specific original back
/// and proving it came back; uninstalling a [`NawTaghyeer::Idafa`] means removing
/// a file that was never the game's. Treating an addition as a modification
/// restores a nonexistent original; treating a modification as an addition
/// deletes a file the game needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawTaghyeer {
    /// A file that existed and was rewritten. Undone by restoring the backup.
    Tadeel,

    /// A file Taarib created that was not there before. Undone by deleting it.
    Idafa,

    /// A directory Taarib created to hold additions.
    ///
    /// Recorded so that an additive payload leaves nothing behind. Removed on
    /// uninstall only when it is empty, because a translator who dropped their
    /// own files into `game/tl/arabic/` should not lose them to an uninstall.
    MujalladMudaf,
}

impl NawTaghyeer {
    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Tadeel => "modified",
            Self::Idafa => "added",
            Self::MujalladMudaf => "added directory",
        }
    }

    /// Whether undoing this means deleting rather than restoring.
    #[must_use]
    pub const fn yuhdhaf(self) -> bool {
        matches!(self, Self::Idafa | Self::MujalladMudaf)
    }
}

/// One line of the manifest: everything needed to undo one path.
///
/// Every field that a restore depends on is written before the corresponding
/// file is touched. The optional fields are optional because a directory and an
/// addition genuinely have no original, not because they are sometimes omitted;
/// [`SijillHifz::tahaqquq`] refuses any combination that says otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SijillHifz {
    /// The path relative to the game's root, with `/` separators.
    ///
    /// Never absolute. A manifest full of absolute paths stops working the
    /// moment the user moves the game to another drive, which is the moment they
    /// are most likely to want to uninstall.
    pub masar: String,

    /// What was done to it.
    pub naw: NawTaghyeer,

    /// Whether anything existed at this path before the patch ran.
    ///
    /// Recorded separately from [`SijillHifz::naw`] rather than derived from it,
    /// because this is the fact an uninstall is wrong about when it leaves
    /// litter, and a fact worth storing twice is a fact worth cross-checking.
    pub kan_mawjudan: bool,

    /// The original's size in bytes, when there was an original.
    pub hajm_asli: Option<u64>,

    /// The original's fingerprint, for verifying that a restore reproduced it.
    pub basma_asliya: Option<Basma>,

    /// The backup's file name inside the originals directory.
    pub nuskha: Option<String>,

    /// The fingerprint of what Taarib wrote, filled in after the write.
    ///
    /// Absent until the guard's write succeeds, which is exactly right: a record
    /// with an original and no new fingerprint describes a file that was
    /// preserved and then not modified, and that is a state a restore handles
    /// correctly.
    pub basma_maktuba: Option<Basma>,

    /// Whether this line has already been undone.
    ///
    /// The resumability marker. Written to the manifest after each file is
    /// restored and verified, so an uninstall interrupted by a crash, a full
    /// disk, or a closed laptop resumes at the first line that is still false.
    #[serde(default)]
    pub istiada_tammat: bool,
}

impl SijillHifz {
    /// Checks that this line is internally consistent.
    ///
    /// Run on every line when a manifest is read, before any of it is acted on.
    /// The rules mirror the `CHECK` constraints on `bayan_tathbeet` in the
    /// database, for the same reason they exist there: an installer with a bug
    /// records the modification and forgets the copy, and nobody finds out until
    /// a user tries to uninstall and the original is already gone.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] naming the path and the specific
    /// contradiction — a modification with no backup, a backup with no
    /// fingerprint, an addition claiming an original, or a directory claiming
    /// bytes.
    pub fn tahaqquq(&self) -> Result<(), KhataNusus> {
        let khalal = |sabab: &str| KhataNusus::NuskhaMafquda {
            masar: PathBuf::from(&self.masar),
            sabab: sabab.to_owned(),
        };

        if self.masar.is_empty() {
            return Err(khalal("the manifest holds a line with no path"));
        }

        match self.naw {
            NawTaghyeer::Tadeel => {
                if !self.kan_mawjudan {
                    return Err(khalal(
                        "recorded as a modification of a file that is also recorded as not \
                         having existed; one of the two is wrong and neither can be trusted",
                    ));
                }
                if self.nuskha.is_none() {
                    return Err(khalal(
                        "recorded as a modification with no backup naming its original, so \
                         there is nothing to restore",
                    ));
                }
                if self.basma_asliya.is_none() || self.hajm_asli.is_none() {
                    return Err(khalal(
                        "recorded as a modification with no original fingerprint or size, so \
                         a restore could not be verified even if it succeeded",
                    ));
                }
            },
            NawTaghyeer::Idafa | NawTaghyeer::MujalladMudaf => {
                if self.kan_mawjudan {
                    return Err(khalal(
                        "recorded as something Taarib added, and also as having existed \
                         before the patch; deleting it on uninstall would delete a file the \
                         game shipped",
                    ));
                }
                if self.nuskha.is_some() || self.basma_asliya.is_some() {
                    return Err(khalal(
                        "recorded as something Taarib added, and also as having an original \
                         to restore",
                    ));
                }
            },
        }

        if matches!(self.naw, NawTaghyeer::MujalladMudaf)
            && (self.hajm_asli.is_some() || self.basma_maktuba.is_some())
        {
            return Err(khalal(
                "recorded as a directory and also as having contents",
            ));
        }

        Ok(())
    }

    /// The path this line names, resolved against a game root.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when the recorded path is not a relative
    /// path that stays inside the root — a manifest that has been edited, or one
    /// carrying an entry from a hostile patch.
    pub fn masar_kamil(&self, jidhr_luba: &Path) -> Result<PathBuf, KhataNusus> {
        dakhil_aw_khata(jidhr_luba, &self.masar)
    }
}

/// The manifest: every path one patch touched, and how to put each one back.
///
/// Serialized as JSON with a schema version, through
/// [`taarib_usus::mukhattat`]. The version is read and checked *before any other
/// field is looked at*, so a manifest written by a newer build is refused
/// outright instead of being partially understood. A partial read here would be
/// uniquely bad: the fields a newer build added would be dropped, the file would
/// be rewritten without them on the next save, and the information needed to
/// undo the patch would be gone while the patch was still installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BayanHifz {
    /// The installation this manifest belongs to.
    ///
    /// The same identity used for the backup directory under `nusakh/`, so a
    /// manifest and the originals it names cannot drift apart.
    pub huwiya: String,

    /// Where the game was when the patch was installed.
    ///
    /// Diagnostics only. [`Hifz::istiada`] takes the root as an argument and
    /// never reads it from here, precisely so that a user who moved their game
    /// between installing and uninstalling can still uninstall.
    pub jidhr_luba_asli: PathBuf,

    /// One line per path, keyed by the relative path.
    ///
    /// A map rather than a list, so one path cannot appear twice with two
    /// different originals — the state in which a restore's outcome depends on
    /// which entry it happened to read first.
    pub sijillat: BTreeMap<String, SijillHifz>,
}

impl BayanHifz {
    /// An empty manifest for a new installation.
    #[must_use]
    pub fn jadeed(huwiya: impl Into<String>, jidhr_luba: &Path) -> Self {
        Self {
            huwiya: huwiya.into(),
            jidhr_luba_asli: jidhr_luba.to_path_buf(),
            sijillat: BTreeMap::new(),
        }
    }

    /// How many paths the patch touched.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.sijillat.len()
    }

    /// Whether the patch touched nothing.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.sijillat.is_empty()
    }

    /// The line for one relative path.
    #[must_use]
    pub fn sijill(&self, masar: &str) -> Option<&SijillHifz> {
        self.sijillat.get(masar)
    }

    /// Every line, in ascending path order.
    pub fn sijillat(&self) -> impl Iterator<Item = &SijillHifz> {
        self.sijillat.values()
    }

    /// How many lines are still waiting to be undone.
    #[must_use]
    pub fn mutabaqqi(&self) -> usize {
        self.sijillat
            .values()
            .filter(|sijill| !sijill.istiada_tammat)
            .count()
    }

    /// Whether every line has been undone and verified.
    #[must_use]
    pub fn ustuidat_bilkamil(&self) -> bool {
        self.sijillat.values().all(|sijill| sijill.istiada_tammat)
    }

    /// Checks every line for internal consistency.
    ///
    /// # Errors
    ///
    /// Whatever [`SijillHifz::tahaqquq`] refuses, at the first line that is
    /// inconsistent, plus [`KhataNusus::NuskhaMafquda`] when a line's key does
    /// not match the path inside it — which means the file was edited by hand and
    /// nothing in it can be trusted.
    pub fn tahaqquq(&self) -> Result<(), KhataNusus> {
        for (miftah, sijill) in &self.sijillat {
            if miftah != &sijill.masar {
                return Err(KhataNusus::NuskhaMafquda {
                    masar: PathBuf::from(miftah),
                    sabab: format!(
                        "the manifest files this line under {miftah} and the line itself \
                         names {}; the manifest has been edited outside Taarib",
                        sijill.masar
                    ),
                });
            }
            sijill.tahaqquq()?;
        }
        Ok(())
    }
}

impl DhuMukhattat for BayanHifz {
    const ISM: &'static str = "bayan_hifz";

    /// One. There has never been another shape of this file.
    const ISDAR: u32 = 1;

    fn hijra(min: u32, _qeema: Value) -> Natija<Value> {
        // Reached only for a manifest claiming a version below the first one,
        // which no build of Taarib ever wrote. Refusing is the only honest
        // answer: guessing at the shape of a manifest means guessing at which
        // backup holds which original.
        Err(Khata::from(KhataNusus::NuskhaMafquda {
            masar: PathBuf::from(ISM_BAYAN),
            sabab: format!(
                "the backup manifest claims schema {min}, and this build knows only \
                 {}. Nothing in it can be located, so nothing is restored from it.",
                Self::ISDAR
            ),
        }))
    }
}

/// One change a patcher intends to make, described before anything is written.
///
/// The input to [`khutta`]. A patcher builds these from what it worked out it
/// needs to do, and the plan says what that would cost and whether any of it
/// contradicts what is actually on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalabTaghyeer {
    /// The path relative to the game's root, with `/` separators.
    pub masar: String,
    /// What the patcher intends to do to it.
    pub naw: NawTaghyeer,
}

impl TalabTaghyeer {
    /// A request to modify an existing file.
    #[must_use]
    pub fn tadeel(masar: impl Into<String>) -> Self {
        Self {
            masar: masar.into(),
            naw: NawTaghyeer::Tadeel,
        }
    }

    /// A request to add a file the game does not have.
    #[must_use]
    pub fn idafa(masar: impl Into<String>) -> Self {
        Self {
            masar: masar.into(),
            naw: NawTaghyeer::Idafa,
        }
    }

    /// A request to create a directory for additions.
    #[must_use]
    pub fn mujallad(masar: impl Into<String>) -> Self {
        Self {
            masar: masar.into(),
            naw: NawTaghyeer::MujalladMudaf,
        }
    }
}

/// One line of the dry run: a request, resolved against what is on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhutwatKhutta {
    /// The path relative to the game's root.
    pub masar: String,
    /// What the patcher asked for.
    pub naw: NawTaghyeer,
    /// Whether something is already at that path.
    pub mawjud: bool,
    /// Its current size, when there is something there.
    pub hajm: Option<u64>,
    /// How many bytes preserving it would put into the backup directory.
    pub hajm_nuskha: u64,
    /// What the request and the disk disagree about, when they do.
    ///
    /// Present rather than fatal, because a dry run's job is to tell a person
    /// what would happen. A patcher that goes ahead anyway will be refused by
    /// [`Hifz::adif`] at the moment it matters.
    pub mulahaza: Option<String>,
}

/// The whole dry run: what a patch would back up, add and modify.
///
/// Computed without creating a directory, writing a byte, or opening a file for
/// writing. A caller shows this and gets a decision before anything is
/// committed, which is the difference between "Taarib will modify 214 files in
/// your game, here they are" and an installer that starts and asks later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhuttatHifz {
    /// The game's root, as the plan was computed against.
    pub jidhr_luba: PathBuf,
    /// One line per request, in request order.
    pub khutuwat: Vec<KhutwatKhutta>,
    /// How many existing files would be modified.
    pub adad_tadeel: usize,
    /// How many new files would be added.
    pub adad_idafa: usize,
    /// How many directories would be created.
    pub adad_mujallad: usize,
    /// How many bytes of originals would be copied into the backup directory.
    pub hajm_nusakh: u64,
    /// How many lines disagree with what is on disk.
    pub adad_mulahazat: usize,
}

impl KhuttatHifz {
    /// Whether the plan would modify nothing that already exists.
    ///
    /// The best case, and worth surfacing on its own: a purely additive patch
    /// has no original to hold, cannot be reverted by a game update overwriting
    /// its own files, and uninstalls by deleting what it created.
    #[must_use]
    pub const fn idafi_bilkamil(&self) -> bool {
        self.adad_tadeel == 0
    }

    /// Whether anything in the plan disagrees with what is on disk.
    #[must_use]
    pub const fn fihi_mulahazat(&self) -> bool {
        self.adad_mulahazat > 0
    }

    /// The plan as lines for the log, the console, and the confirmation screen.
    ///
    /// Allocates, and is meant to: it runs once, when a person is about to be
    /// asked whether to modify their game.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.khutuwat.len().saturating_add(2));
        sutur.push(format!(
            "{}: {} modified, {} added, {} new director(ies), {} byte(s) preserved",
            self.jidhr_luba.display(),
            self.adad_tadeel,
            self.adad_idafa,
            self.adad_mujallad,
            self.hajm_nusakh
        ));
        for khutwa in &self.khutuwat {
            let hajm = khutwa
                .hajm
                .map_or_else(|| "-".to_owned(), |qeema| qeema.to_string());
            sutur.push(match &khutwa.mulahaza {
                Some(mulahaza) => {
                    format!(
                        "  {} [{}] {hajm} -- {mulahaza}",
                        khutwa.masar,
                        khutwa.naw.ism()
                    )
                },
                None => format!("  {} [{}] {hajm}", khutwa.masar, khutwa.naw.ism()),
            });
        }
        if self.idafi_bilkamil() {
            sutur.push(
                "  purely additive: uninstalling removes what was added and touches \
                 nothing the game shipped"
                    .to_owned(),
            );
        }
        sutur
    }
}

/// Computes the whole plan without writing anything.
///
/// Opens nothing for writing, creates no directory, and does not touch the
/// backup directory at all. Every line is resolved against the filesystem as it
/// is right now, so a request to add a file that already exists, or to modify one
/// that does not, is reported here rather than discovered halfway through an
/// install.
///
/// # Errors
///
/// [`KhataNusus::NuskhaMafquda`] when a requested path is not a relative path
/// that stays inside the game's root. That is refused rather than reported,
/// because a path escaping the game directory is not a disagreement about the
/// plan — it is a patch trying to write somewhere it has no business writing, and
/// no plan containing one should be shown as though a person could approve it.
pub fn khutta(jidhr_luba: &Path, matlub: &[TalabTaghyeer]) -> Result<KhuttatHifz, KhataNusus> {
    let mut khutta = KhuttatHifz {
        jidhr_luba: jidhr_luba.to_path_buf(),
        khutuwat: Vec::with_capacity(matlub.len()),
        adad_tadeel: 0,
        adad_idafa: 0,
        adad_mujallad: 0,
        hajm_nusakh: 0,
        adad_mulahazat: 0,
    };

    for talab in matlub {
        let kamil = dakhil_aw_khata(jidhr_luba, &talab.masar)?;
        let bayanat = fs::metadata(&kamil).ok();
        let mawjud = bayanat.is_some();
        let hajm = bayanat
            .as_ref()
            .filter(|b| b.is_file())
            .map(fs::Metadata::len);

        let mut mulahaza = None;
        let mut hajm_nuskha = 0;

        match talab.naw {
            NawTaghyeer::Tadeel => {
                khutta.adad_tadeel = khutta.adad_tadeel.saturating_add(1);
                match hajm {
                    Some(qeema) if qeema > AQSA_HAJM_NUSKHA => {
                        mulahaza = Some(format!(
                            "{qeema} bytes, above the {AQSA_HAJM_NUSKHA}-byte ceiling on what \
                             this build will preserve; installing would be refused"
                        ));
                    },
                    Some(qeema) => hajm_nuskha = qeema,
                    None if mawjud => {
                        mulahaza = Some(
                            "a directory is at this path, and a directory cannot be \
                             preserved as a file"
                                .to_owned(),
                        );
                    },
                    None => {
                        mulahaza = Some(
                            "nothing is at this path, so there is no original to preserve; \
                             this would be an addition, not a modification"
                                .to_owned(),
                        );
                    },
                }
            },
            NawTaghyeer::Idafa => {
                khutta.adad_idafa = khutta.adad_idafa.saturating_add(1);
                if mawjud {
                    mulahaza = Some(
                        "something is already at this path, so adding it would overwrite a \
                         file Taarib did not create and uninstalling would delete it"
                            .to_owned(),
                    );
                }
            },
            NawTaghyeer::MujalladMudaf => {
                khutta.adad_mujallad = khutta.adad_mujallad.saturating_add(1);
                if mawjud {
                    mulahaza = Some(
                        "this directory already exists, so it is the game's and will not be \
                         removed on uninstall"
                            .to_owned(),
                    );
                }
            },
        }

        if mulahaza.is_some() {
            khutta.adad_mulahazat = khutta.adad_mulahazat.saturating_add(1);
        }
        khutta.hajm_nusakh = khutta.hajm_nusakh.saturating_add(hajm_nuskha);
        khutta.khutuwat.push(KhutwatKhutta {
            masar: talab.masar.clone(),
            naw: talab.naw,
            mawjud,
            hajm,
            hajm_nuskha,
            mulahaza,
        });
    }

    Ok(khutta)
}

/// A preservation session: one game, one backup directory, one manifest.
///
/// Held by whichever patcher is installing. It owns the manifest in memory and
/// rewrites it to disk atomically at every point where the on-disk record would
/// otherwise fall behind what has been done to the game.
#[derive(Debug)]
pub struct Hifz {
    jidhr_luba: PathBuf,
    jidhr_nusakh: PathBuf,
    mujallad_asl: PathBuf,
    masar_bayan: PathBuf,
    bayan: BayanHifz,
}

impl Hifz {
    /// Opens a new session, creating the backup directory and an empty manifest.
    ///
    /// Refuses when a manifest already exists at that location. Two
    /// installations sharing one backup directory is the state in which
    /// uninstalling the second restores originals belonging to the first, and it
    /// is cheaper to refuse it here than to detect it afterwards.
    ///
    /// The empty manifest is written immediately rather than at the first
    /// preservation, so that a crash during the very first backup still leaves a
    /// directory that identifies itself as Taarib's.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when the backup directory cannot be
    /// created, when a manifest is already there, or when the empty manifest
    /// cannot be written — all of which mean nothing may be modified yet, which
    /// is exactly the state this call is supposed to establish.
    pub fn ibda(
        jidhr_luba: &Path,
        jidhr_nusakh: &Path,
        huwiya: impl Into<String>,
    ) -> Result<Self, KhataNusus> {
        let masar_bayan = jidhr_nusakh.join(ISM_BAYAN);
        if masar_bayan.exists() {
            return Err(KhataNusus::NuskhaMafquda {
                masar: masar_bayan,
                sabab: "a backup manifest is already here, which means another installation \
                        is using this directory; its originals will not be mixed with a \
                        second patch's"
                    .to_owned(),
            });
        }

        let mujallad_asl = jidhr_nusakh.join(ISM_MUJALLAD_ASL);
        insha_aw_khata(&mujallad_asl)?;

        let hifz = Self {
            jidhr_luba: jidhr_luba.to_path_buf(),
            jidhr_nusakh: jidhr_nusakh.to_path_buf(),
            mujallad_asl,
            masar_bayan,
            bayan: BayanHifz::jadeed(huwiya, jidhr_luba),
        };
        hifz.iktub_bayan()?;
        Ok(hifz)
    }

    /// Reopens an existing session from its manifest.
    ///
    /// What an uninstall uses, and what a resumed uninstall uses: the manifest
    /// carries its own progress, so this is also how a half-finished restore is
    /// picked up. The game root is supplied rather than read from the manifest,
    /// so a game moved between installing and uninstalling is still reachable.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when the manifest is absent, unreadable,
    /// written by a newer build, or internally inconsistent. Every one of those
    /// means the patch cannot be reliably undone, and saying so is better than
    /// undoing part of it.
    pub fn istanif(jidhr_luba: &Path, jidhr_nusakh: &Path) -> Result<Self, KhataNusus> {
        let masar_bayan = jidhr_nusakh.join(ISM_BAYAN);
        let bayan: BayanHifz =
            mukhattat::iqra_malaf(&masar_bayan).map_err(|khata| KhataNusus::NuskhaMafquda {
                masar: masar_bayan.clone(),
                sabab: khata.injilizi,
            })?;
        bayan.tahaqquq()?;

        Ok(Self {
            jidhr_luba: jidhr_luba.to_path_buf(),
            jidhr_nusakh: jidhr_nusakh.to_path_buf(),
            mujallad_asl: jidhr_nusakh.join(ISM_MUJALLAD_ASL),
            masar_bayan,
            bayan,
        })
    }

    /// The game's root.
    #[must_use]
    pub fn jidhr_luba(&self) -> &Path {
        &self.jidhr_luba
    }

    /// The backup directory for this installation.
    #[must_use]
    pub fn jidhr_nusakh(&self) -> &Path {
        &self.jidhr_nusakh
    }

    /// The manifest's own path.
    #[must_use]
    pub fn masar_bayan(&self) -> &Path {
        &self.masar_bayan
    }

    /// The manifest as it currently stands.
    #[must_use]
    pub const fn bayan(&self) -> &BayanHifz {
        &self.bayan
    }

    /// Whether a path has already been preserved in this session.
    #[must_use]
    pub fn mahfuz(&self, nisbi: &str) -> bool {
        self.bayan.sijillat.contains_key(&masar_muwahhad(nisbi))
    }

    /// Preserves a file's original and returns the permission to modify it.
    ///
    /// This is the only route to a [`HarisKitaba`], and a [`HarisKitaba`] is the
    /// only thing in this crate that can write into a game directory. By the time
    /// this returns, the original is in the backup directory and the manifest
    /// line naming it has been flushed to the device — in that order, and both
    /// before the caller is able to write anything.
    ///
    /// A path that does not exist yet is recorded as an addition rather than
    /// refused, and undoing it will delete rather than restore. That is the
    /// distinction the whole module turns on.
    ///
    /// Calling this twice for one path is safe and does not copy twice. That is
    /// not an optimization: the second copy would happen *after* the first write,
    /// so it would preserve the patched bytes as though they were the original
    /// and quietly destroy the only copy of the real one.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::HajmMufrit`] when the file declares more than
    /// [`AQSA_HAJM_NUSKHA`], checked before a byte is read.
    /// [`KhataNusus::DawraGhayrMutabaqa`] when the backup that was just written
    /// does not read back as the bytes that went into it.
    /// [`KhataNusus::NuskhaMafquda`] for everything else that stops the original
    /// being preserved — an unreadable file, an unwritable backup directory, a
    /// path that escapes the game's root, or a manifest that cannot be flushed.
    /// In every case nothing has been modified, which is the point.
    pub fn ihfaz(&mut self, nisbi: &str) -> Result<HarisKitaba<'_>, KhataNusus> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        if let Some(mawjud) = self.bayan.sijillat.get(&masar) {
            if matches!(mawjud.naw, NawTaghyeer::MujalladMudaf) {
                return Err(KhataNusus::NuskhaMafquda {
                    masar: mutlaq,
                    sabab: "this path is recorded as a directory Taarib created, and a \
                            directory cannot be rewritten as a file"
                        .to_owned(),
                });
            }
            let naw = mawjud.naw;
            return Ok(HarisKitaba {
                hifz: self,
                masar,
                naw,
            });
        }

        let bayanat = fs::metadata(&mutlaq).ok();
        let Some(bayanat) = bayanat else {
            // Nothing there: this is an addition, and an addition has no
            // original. Recording it as one would mean writing an empty file
            // over the game on uninstall.
            return self.sajjil_wa_ihris(
                masar,
                SijillHifz {
                    masar: String::new(),
                    naw: NawTaghyeer::Idafa,
                    kan_mawjudan: false,
                    hajm_asli: None,
                    basma_asliya: None,
                    nuskha: None,
                    basma_maktuba: None,
                    istiada_tammat: false,
                },
            );
        };

        if !bayanat.is_file() {
            return Err(KhataNusus::NuskhaMafquda {
                masar: mutlaq,
                sabab: "a directory or a special file is at this path, and only a regular \
                        file can be preserved and put back"
                    .to_owned(),
            });
        }

        let hajm = bayanat.len();
        if hajm > AQSA_HAJM_NUSKHA {
            return Err(KhataNusus::HajmMufrit {
                haql: "original file",
                qeema: hajm,
                saqf: AQSA_HAJM_NUSKHA,
            });
        }

        let asli = masarat::qira(&mutlaq).map_err(|khata| KhataNusus::NuskhaMafquda {
            masar: mutlaq.clone(),
            sabab: khata.injilizi,
        })?;
        let basma_asliya = basma_bayt(&asli);

        let miftah = miftah_nuskha(&masar);
        let masar_nuskha = self.mujallad_asl.join(&miftah);
        masarat::kitaba_dharra(&masar_nuskha, &asli).map_err(|khata| {
            KhataNusus::NuskhaMafquda {
                masar: mutlaq.clone(),
                sabab: khata.injilizi,
            }
        })?;

        // Read the copy back before trusting it. A backup that was written to a
        // failing disk, a full volume, or a filesystem that silently truncated it
        // is worth nothing, and the only moment it can be caught cheaply is now,
        // while the real original is still on disk and untouched.
        let murtajaa = masarat::qira(&masar_nuskha).map_err(|khata| KhataNusus::NuskhaMafquda {
            masar: masar_nuskha.clone(),
            sabab: khata.injilizi,
        })?;
        let farq = adad_ikhtilaf(&asli, &murtajaa);
        if farq > 0 {
            return Err(KhataNusus::DawraGhayrMutabaqa {
                sigha: "backup copy",
                adad: farq,
            });
        }

        self.sajjil_wa_ihris(
            masar,
            SijillHifz {
                masar: String::new(),
                naw: NawTaghyeer::Tadeel,
                kan_mawjudan: true,
                hajm_asli: Some(hajm),
                basma_asliya: Some(basma_asliya),
                nuskha: Some(miftah),
                basma_maktuba: None,
                istiada_tammat: false,
            },
        )
    }

    /// Records an addition and returns the permission to write it.
    ///
    /// The preferred route wherever an engine's delivery allows it — a
    /// `game/tl/arabic/*.rpy`, a `js/plugins/taarib.js`, an appended script — for
    /// two reasons. Uninstalling is a delete, so there is no original to hold and
    /// no fingerprint that has to be reproduced. And a game update rewrites the
    /// files the game ships while leaving files it has never heard of alone, so
    /// an additive patch commonly survives an update that would have reverted a
    /// modification without telling anyone.
    ///
    /// Refuses when something is already at the path. That is not pedantry: a
    /// file recorded as an addition is *deleted* on uninstall, so recording an
    /// existing file as one would arm Taarib to delete something it did not
    /// create. A patcher that genuinely means to rewrite an existing file calls
    /// [`Hifz::ihfaz`], which preserves it first.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when something already exists at the path,
    /// when the path escapes the game's root, or when the manifest cannot be
    /// flushed before the caller is allowed to write.
    pub fn adif(&mut self, nisbi: &str) -> Result<HarisKitaba<'_>, KhataNusus> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        if let Some(mawjud) = self.bayan.sijillat.get(&masar) {
            let naw = mawjud.naw;
            if matches!(naw, NawTaghyeer::Idafa) {
                return Ok(HarisKitaba {
                    hifz: self,
                    masar,
                    naw,
                });
            }
            return Err(KhataNusus::NuskhaMafquda {
                masar: mutlaq,
                sabab: format!(
                    "this path is already recorded as {}, and it cannot also be an addition",
                    naw.ism()
                ),
            });
        }

        if mutlaq.exists() {
            return Err(KhataNusus::NuskhaMafquda {
                masar: mutlaq,
                sabab: "something is already at this path, so it is not Taarib's to add. \
                        Uninstalling deletes additions, and deleting a file the game shipped \
                        is not an uninstall. Preserve it instead."
                    .to_owned(),
            });
        }

        self.sajjil_wa_ihris(
            masar,
            SijillHifz {
                masar: String::new(),
                naw: NawTaghyeer::Idafa,
                kan_mawjudan: false,
                hajm_asli: None,
                basma_asliya: None,
                nuskha: None,
                basma_maktuba: None,
                istiada_tammat: false,
            },
        )
    }

    /// Creates a directory for additions and records it so uninstall can remove
    /// it.
    ///
    /// Returns whether the directory was Taarib's to create. A directory that
    /// already existed is the game's and is not recorded, because removing it on
    /// uninstall would remove a directory the game shipped.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when the path escapes the game's root, when
    /// the directory cannot be created, or when the manifest cannot be flushed.
    pub fn sajjil_mujallad(&mut self, nisbi: &str) -> Result<bool, KhataNusus> {
        let masar = masar_muwahhad(nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &masar)?;

        if self.bayan.sijillat.contains_key(&masar) {
            return Ok(false);
        }
        if mutlaq.exists() {
            return Ok(false);
        }

        insha_aw_khata(&mutlaq)?;
        let _ = self.bayan.sijillat.insert(
            masar.clone(),
            SijillHifz {
                masar,
                naw: NawTaghyeer::MujalladMudaf,
                kan_mawjudan: false,
                hajm_asli: None,
                basma_asliya: None,
                nuskha: None,
                basma_maktuba: None,
                istiada_tammat: false,
            },
        );
        self.iktub_bayan()?;
        Ok(true)
    }

    /// Files a manifest line, flushes the manifest, and mints the guard.
    ///
    /// The flush happens here, before the guard exists, which is what makes
    /// "the record is durable before the file is modified" a property of the
    /// type system rather than a rule five callers have to follow.
    fn sajjil_wa_ihris(
        &mut self,
        masar: String,
        mut sijill: SijillHifz,
    ) -> Result<HarisKitaba<'_>, KhataNusus> {
        sijill.masar.clone_from(&masar);
        sijill.tahaqquq()?;
        let naw = sijill.naw;
        let _ = self.bayan.sijillat.insert(masar.clone(), sijill);
        self.iktub_bayan()?;
        Ok(HarisKitaba {
            hifz: self,
            masar,
            naw,
        })
    }

    /// Writes the manifest atomically, through the shared mechanism.
    fn iktub_bayan(&self) -> Result<(), KhataNusus> {
        mukhattat::iktub_malaf(&self.masar_bayan, &self.bayan).map_err(|khata| {
            KhataNusus::NuskhaMafquda {
                masar: self.masar_bayan.clone(),
                sabab: khata.injilizi,
            }
        })
    }
}

/// Permission to write one file of a game, proving its original is preserved.
///
/// There is no public constructor and no public field. The only way to obtain
/// one is [`Hifz::ihfaz`] or [`Hifz::adif`] returning `Ok`, which happens only
/// after the original has been copied into the backup directory and the manifest
/// line naming that copy has been flushed to the device. [`Hifz`] has no method
/// that takes a path and some bytes, so a patcher cannot reach a game file
/// without one of these in its hand.
///
/// That is the entire enforcement mechanism, and it is deliberately a type and
/// not a rule: code that tries to modify an unpreserved file does not fail
/// review, it fails to compile, because the function it would have to call is not
/// there to call.
///
/// The guard is consumed by writing, so one preservation authorizes one write.
/// Dropping it without writing is harmless — the manifest then describes a file
/// that was preserved and left alone, and restoring it writes an identical
/// original back over itself and verifies.
#[derive(Debug)]
#[must_use = "obtaining a guard preserves the original; dropping it without writing \
              leaves a manifest line for a file that was never modified"]
pub struct HarisKitaba<'a> {
    hifz: &'a mut Hifz,
    /// The manifest key, and the **only** identity this guard carries.
    ///
    /// It deliberately does not also hold the absolute path. Carrying both
    /// would be two sources of truth for one fact, and one assignment away from
    /// a guard whose path and whose record disagree — at which point a write
    /// through it lands on a file the manifest describes under a different
    /// name, which *is* a file outside the manifest.
    ///
    /// [`HarisKitaba::masar_kamil`] derives the absolute path from the session's
    /// game root each time instead. There is no field to disagree with the
    /// record, and no parameter through which a caller could supply one.
    masar: String,
    naw: NawTaghyeer,
}

impl HarisKitaba<'_> {
    /// The path this guard authorizes, relative to the game's root.
    #[must_use]
    pub fn masar(&self) -> &str {
        &self.masar
    }

    /// The absolute path this guard authorizes.
    ///
    /// Derived from the manifest key and the session's game root on every call,
    /// which is the same derivation [`HarisKitaba::iktub`] performs.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::NuskhaMafquda`] when the manifest key does not join onto the
    /// game's root as a path inside it: an empty key, one carrying a parent
    /// component, an absolute path or a drive prefix, an embedded NUL, or — on
    /// Windows — a reserved device name, a component ending in a dot or a space,
    /// or an alternate data stream; and when the joined path is longer than the
    /// platform will reliably handle.
    ///
    /// The root is fixed for the session and the key is the one that already
    /// joined successfully when the guard was minted, so a guard held by a
    /// patcher does not reach any of these.
    pub fn masar_kamil(&self) -> Result<PathBuf, KhataNusus> {
        dakhil_aw_khata(&self.hifz.jidhr_luba, &self.masar)
    }

    /// Whether undoing this write will restore an original or delete a file.
    #[must_use]
    pub const fn naw(&self) -> NawTaghyeer {
        self.naw
    }

    /// Writes the file, atomically, and records what was written.
    ///
    /// Beside, fsync, rename, through [`taarib_usus::masarat::kitaba_dharra`] —
    /// so a machine that loses power mid-write comes back with either the whole
    /// old file or the whole new one. A `data.win` or an `app.asar` truncated at
    /// two thirds is not a damaged patch, it is a game that will not start.
    ///
    /// The fingerprint of what was written is added to the manifest afterwards.
    /// That second flush is ordered after the write on purpose: it carries only
    /// drift-detection data, and the line that makes the change reversible was
    /// already durable before this function was reachable.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] naming the path when the atomic write cannot
    /// complete, and [`KhataNusus::NuskhaMafquda`] when the manifest cannot be
    /// updated with what was written — which leaves the file modified and
    /// restorable, since its original and its backup key were recorded before
    /// this call.
    pub fn iktub(self, bayt: &[u8]) -> Result<Basma, KhataNusus> {
        // Derived from the manifest key, here, at the moment of writing. This is
        // the whole "no file outside the manifest" guarantee: the bytes cannot
        // land anywhere except at the path the record names, because that path
        // is computed from the record.
        let mutlaq = dakhil_aw_khata(&self.hifz.jidhr_luba, &self.masar)?;
        masarat::kitaba_dharra(&mutlaq, bayt).map_err(|khata| KhataNusus::KhataMalaf {
            masar: mutlaq.clone(),
            sabab: std::io::Error::other(khata.injilizi),
        })?;

        let basma = basma_bayt(bayt);
        if let Some(sijill) = self.hifz.bayan.sijillat.get_mut(&self.masar) {
            sijill.basma_maktuba = Some(basma);
        }
        self.hifz.iktub_bayan()?;
        Ok(basma)
    }

    /// Writes UTF-8 text, with no byte order mark.
    ///
    /// # Errors
    ///
    /// As [`HarisKitaba::iktub`].
    pub fn iktub_nass(self, nass: &str) -> Result<Basma, KhataNusus> {
        self.iktub(nass.as_bytes())
    }
}

/// The only way any patcher in this crate is permitted to change a game.
///
/// Five modules write into game directories and none of them owns a file
/// handle. They take a `&mut dyn Hafiz` and call one of these four methods,
/// which is what makes "every change is recorded before it happens" a property
/// of the call graph rather than a convention five authors have to remember.
/// [`Hifz`] is the implementation the product uses; the trait exists so that a
/// patcher can be exercised against a recorder that writes nowhere, and so that
/// a reviewer grepping for `fs::File::create` outside this module finds nothing.
///
/// Absolute paths, because every caller already holds one — the game root joined
/// with an engine-specific layout. Converting to the manifest's relative form is
/// this trait's job, and doing it in one place is why a path outside the game
/// root is a refusal instead of five separate opportunities to write into
/// somebody's home directory.
pub trait Hafiz {
    /// Replaces a file that already exists, preserving its original bytes and
    /// its fingerprint first, and writing the replacement atomically.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the original cannot be preserved or the
    /// replacement cannot be written. An implementation must refuse rather than
    /// proceed: the original is the only copy of the game the player owns.
    fn iktub(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus>;

    /// Creates a file the patch adds, recording it so uninstalling removes it.
    ///
    /// # Errors
    ///
    /// As [`Hafiz::iktub`], plus whatever the guard raises when a file it was
    /// asked to create is already there and was not created by this patch.
    fn ansha(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus>;

    /// Creates a directory the patch needs, recording it when it is Taarib's.
    ///
    /// Ren'Py's `game/tl/arabic/` and RPG Maker's `js/plugins/` are directories
    /// a patch may have to make. A directory that already existed is the game's
    /// and is *not* recorded, so an uninstall does not remove it.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the path escapes the game root or the
    /// directory cannot be created.
    fn ansha_mujallad(&mut self, masar: &Path) -> Result<(), KhataNusus>;

    /// Deletes a file **this patch added**, and nothing else.
    ///
    /// The asymmetry with [`Hafiz::ansha`] is the point. An uninstall that could
    /// delete an arbitrary path would be one manifest bug away from deleting a
    /// file the game shipped, so the implementation is required to refuse any
    /// path not recorded as a [`NawTaghyeer::Idafa`]. Deleting a path that was
    /// never added is not an error — uninstalling twice succeeds and the second
    /// run does nothing, which is what an uninstall after a failed install has
    /// to do. The return says which of the two happened.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the path is recorded as a modification
    /// rather than an addition, or when the file cannot be removed.
    fn ihdhif(&mut self, masar: &Path) -> Result<bool, KhataNusus>;

    /// Removes a directory **this patch created**, if it is now empty.
    ///
    /// The pair of [`Hafiz::ansha_mujallad`], and refused on the same terms: a
    /// directory the game shipped is not Taarib's to remove. Emptiness is a
    /// second condition rather than an alternative to the first — a recorded
    /// directory that still holds a translator's own file stays, with its
    /// contents, and the manifest keeps saying Taarib made it so a later
    /// uninstall can try again once the directory is genuinely empty.
    ///
    /// # Errors
    ///
    /// Whatever the guard raises when the path is recorded as something other
    /// than a directory this patch created, or when the removal fails for a
    /// reason other than the directory not being empty.
    fn ihdhif_mujallad(&mut self, masar: &Path) -> Result<bool, KhataNusus>;
}

impl Hafiz for Hifz {
    fn iktub(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let _ = self.ihfaz(&nisbi)?.iktub(bayt)?;
        Ok(())
    }

    fn ansha(&mut self, masar: &Path, bayt: &[u8]) -> Result<(), KhataNusus> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let mifta = masar_muwahhad(&nisbi);
        if let Some((walid, _)) = mifta.rsplit_once('/') {
            self.sajjil_silsilat_mujalladat(walid)?;
        }
        let _ = self.adif(&nisbi)?.iktub(bayt)?;
        Ok(())
    }

    fn ansha_mujallad(&mut self, masar: &Path) -> Result<(), KhataNusus> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        self.sajjil_silsilat_mujalladat(&masar_muwahhad(&nisbi))
    }

    fn ihdhif(&mut self, masar: &Path) -> Result<bool, KhataNusus> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let mifta = masar_muwahhad(&nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &mifta)?;

        let Some(sijill) = self.bayan.sijillat.get_mut(&mifta) else {
            if mutlaq.exists() {
                return Err(KhataNusus::NuskhaMafquda {
                    masar: mutlaq,
                    sabab: "this path is not recorded as something Taarib added, so deleting \
                            it would delete a file the game shipped. Restore through the \
                            manifest instead."
                        .to_owned(),
                });
            }
            return Ok(false);
        };

        if !matches!(sijill.naw, NawTaghyeer::Idafa) {
            return Err(KhataNusus::NuskhaMafquda {
                masar: mutlaq,
                sabab: format!(
                    "this path is recorded as {}, and undoing a modification means writing \
                     its original back, not deleting it",
                    sijill.naw.ism()
                ),
            });
        }

        let kan = mutlaq.exists();
        if kan {
            fs::remove_file(&mutlaq).map_err(|sabab| KhataNusus::KhataMalaf {
                masar: mutlaq.clone(),
                sabab,
            })?;
        }
        sijill.istiada_tammat = true;
        self.iktub_bayan()?;
        Ok(kan)
    }

    fn ihdhif_mujallad(&mut self, masar: &Path) -> Result<bool, KhataNusus> {
        let nisbi = self.nisbi_aw_khata(masar)?;
        let mifta = masar_muwahhad(&nisbi);
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, &mifta)?;

        let Some(sijill) = self.bayan.sijillat.get_mut(&mifta) else {
            return Ok(false);
        };
        if !matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
            return Err(KhataNusus::NuskhaMafquda {
                masar: mutlaq,
                sabab: format!(
                    "this path is recorded as {}, not as a directory Taarib created",
                    sijill.naw.ism()
                ),
            });
        }
        if !mutlaq.is_dir() {
            sijill.istiada_tammat = true;
            self.iktub_bayan()?;
            return Ok(false);
        }
        // A directory that still holds something is left alone, and the record
        // is left standing so a later run can retry. `remove_dir` is used rather
        // than `remove_dir_all` precisely because it cannot take a file with it:
        // the "is it empty?" question is answered by the kernel at the moment of
        // removal, not by a listing this process took a moment earlier and that
        // another process may already have invalidated.
        match fs::remove_dir(&mutlaq) {
            Ok(()) => {
                sijill.istiada_tammat = true;
                self.iktub_bayan()?;
                Ok(true)
            },
            Err(sabab) if sabab.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(false),
            Err(sabab) => Err(KhataNusus::KhataMalaf {
                masar: mutlaq,
                sabab,
            }),
        }
    }
}

impl Hifz {
    /// An absolute path as the manifest names it, or a refusal saying why not.
    ///
    /// Every [`Hafiz`] method starts here, so a path outside the game root, or
    /// one that cannot be written as UTF-8, is rejected once rather than in four
    /// places that could drift apart.
    fn nisbi_aw_khata(&self, masar: &Path) -> Result<String, KhataNusus> {
        nisbi_min(&self.jidhr_luba, masar).ok_or_else(|| KhataNusus::NuskhaMafquda {
            masar: masar.to_path_buf(),
            sabab: format!(
                "this path is not inside the game directory at {}, or cannot be written as \
                 UTF-8, and a patch writes nowhere else",
                self.jidhr_luba.display()
            ),
        })
    }

    /// Records every ancestor of a directory that Taarib has to create, from the
    /// outermost inwards.
    ///
    /// [`Hifz::sajjil_mujallad`] creates the whole chain but records only the
    /// path it was handed, which would leave `game/tl/` behind after an
    /// uninstall that removed `game/tl/arabic/`. Walking outermost-first is what
    /// makes the difference: `sajjil_mujallad` returns `false` for a directory
    /// that was already there, so exactly the ones this patch brought into
    /// existence end up in the manifest, and the ones the game shipped do not.
    fn sajjil_silsilat_mujalladat(&mut self, nisbi: &str) -> Result<(), KhataNusus> {
        let mut mutarakim = String::with_capacity(nisbi.len());
        for juz in nisbi.split('/').filter(|juz| !juz.is_empty()) {
            if !mutarakim.is_empty() {
                mutarakim.push('/');
            }
            mutarakim.push_str(juz);
            let _ = self.sajjil_mujallad(&mutarakim)?;
        }
        Ok(())
    }
}

/// What an uninstall did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TaqreerIstiada {
    /// Originals written back and verified against their recorded fingerprint.
    pub mustaada: usize,
    /// Files Taarib had added, deleted.
    pub mahdhufa: usize,
    /// Directories Taarib had created, removed because they were empty.
    pub mujalladat_muzala: usize,
    /// Directories left in place because they still hold somebody else's files.
    pub mujalladat_matruka: usize,
}

impl TaqreerIstiada {
    /// How many paths were dealt with.
    #[must_use]
    pub const fn majmu(&self) -> usize {
        self.mustaada
            .saturating_add(self.mahdhufa)
            .saturating_add(self.mujalladat_muzala)
            .saturating_add(self.mujalladat_matruka)
    }

    /// The result as lines for the log and the diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "restored {} original(s), deleted {} added file(s), removed {} director(ies)",
            self.mustaada, self.mahdhufa, self.mujalladat_muzala
        )];
        if self.mujalladat_matruka > 0 {
            sutur.push(format!(
                "  {} director(ies) left in place because they still hold files Taarib did \
                 not put there",
                self.mujalladat_matruka
            ));
        }
        sutur
    }
}

/// What verification found at one recorded path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatMalaf {
    /// The file is byte-for-byte what Taarib wrote. The patch is intact here.
    Mutabiq,

    /// The file is byte-for-byte the original.
    ///
    /// The patch is not installed at this path any more, and nothing is
    /// damaged — a game update reinstalling its own file produces exactly this,
    /// and so does a partially completed uninstall.
    Ustuidat,

    /// The file is neither what Taarib wrote nor the original.
    ///
    /// The drift case, and the reason this check exists: a game updated
    /// underneath a patch, or another tool edited the same file. Reinstalling
    /// over it would preserve the *drifted* bytes as though they were the
    /// original, so the caller has to decide, not the installer.
    Munharif {
        /// The fingerprint the manifest recorded for what Taarib wrote.
        muallana: Basma,
        /// The fingerprint of what is there now.
        mahsuba: Basma,
    },

    /// Nothing is at the path any more.
    Mafqud,

    /// The manifest records no fingerprint for what Taarib wrote here.
    ///
    /// The file was preserved and then not modified — a guard obtained and
    /// dropped. There is nothing to compare against and nothing wrong.
    GhayrMuhaqqaq,
}

impl HalatMalaf {
    /// Whether this state means the patch is still in place and unmodified.
    #[must_use]
    pub const fn salim(self) -> bool {
        matches!(self, Self::Mutabiq | Self::GhayrMuhaqqaq)
    }

    /// The name used in reports and log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mutabiq => "intact",
            Self::Ustuidat => "back to the original",
            Self::Munharif { .. } => "drifted",
            Self::Mafqud => "missing",
            Self::GhayrMuhaqqaq => "not modified",
        }
    }
}

/// What verification found across a whole installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerTahaqquq {
    /// One state per recorded path, in ascending path order.
    pub halat: BTreeMap<String, HalatMalaf>,
    /// Files found inside directories Taarib created that the manifest does not
    /// mention.
    ///
    /// Not an error. It is usually a translator's own work, and it is the reason
    /// uninstall removes a created directory only when it is empty.
    pub zaida: Vec<String>,
    /// How many paths drifted.
    pub adad_munharif: usize,
    /// How many paths are gone.
    pub adad_mafqud: usize,
    /// How many paths are back to their original bytes.
    pub adad_mustaad: usize,
}

impl TaqreerTahaqquq {
    /// Whether every recorded path is still what Taarib wrote.
    #[must_use]
    pub const fn salim(&self) -> bool {
        self.adad_munharif == 0 && self.adad_mafqud == 0 && self.adad_mustaad == 0
    }

    /// The result as lines for the log, the bundle and the game's detail screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.halat.len().saturating_add(2));
        sutur.push(format!(
            "{} path(s) checked: {} drifted, {} missing, {} back to the original",
            self.halat.len(),
            self.adad_munharif,
            self.adad_mafqud,
            self.adad_mustaad
        ));
        for (masar, hala) in &self.halat {
            if !hala.salim() {
                sutur.push(format!("  {masar}: {}", hala.ism()));
            }
        }
        for masar in &self.zaida {
            sutur.push(format!("  {masar}: present and not in the manifest"));
        }
        sutur
    }
}

// ---------------------------------------------------------------------------
// Undoing a patch, and proving it was undone
// ---------------------------------------------------------------------------

impl Hifz {
    /// Puts every preserved original back, and deletes everything Taarib added.
    ///
    /// Each restored file is re-hashed against the fingerprint recorded before
    /// it was ever modified, and a file that does not reproduce it stops the
    /// uninstall with [`KhataNusus::IstiadaGhayrMutabaqa`]. A restore that is
    /// assumed rather than verified is worse than a restore that failed loudly:
    /// the user is told their game is clean and it is not.
    ///
    /// Idempotent and resumable. Each line is marked done in the manifest, and
    /// the manifest is flushed, before the next line is started — so a crash, a
    /// full disk or a closed laptop halfway through leaves a manifest that says
    /// exactly which files are already back, and calling this again finishes the
    /// job instead of starting it over. The cost is one small atomic write per
    /// file, which is the right trade for a few hundred files: the alternative is
    /// a single flush at the end, and a single flush at the end is precisely the
    /// design that cannot be resumed.
    ///
    /// Directories Taarib created are removed last, deepest first, and only when
    /// empty. A translator who put their own files into `game/tl/arabic/` keeps
    /// them.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::IstiadaGhayrMutabaqa`] when a restored file does not match
    /// its recorded fingerprint, [`KhataNusus::NuskhaMafquda`] when a backup
    /// named by the manifest is missing or unreadable, and
    /// [`KhataNusus::KhataMalaf`] when a game file cannot be written or deleted.
    /// Whatever had already been restored stays restored and stays marked, so a
    /// later call resumes from the failure.
    pub fn istiada(&mut self) -> Result<TaqreerIstiada, KhataNusus> {
        let mut taqreer = TaqreerIstiada::default();

        let malaffat: Vec<String> = self
            .bayan
            .sijillat
            .iter()
            .filter(|(_, sijill)| {
                !sijill.istiada_tammat && !matches!(sijill.naw, NawTaghyeer::MujalladMudaf)
            })
            .map(|(masar, _)| masar.clone())
            .collect();
        for masar in malaffat {
            self.istiada_wahid(&masar, &mut taqreer)?;
        }

        // Descending path order puts `game/tl/arabic` before `game/tl`, so a
        // nested directory is empty by the time its parent is considered.
        let mut mujalladat: Vec<String> = self
            .bayan
            .sijillat
            .iter()
            .filter(|(_, sijill)| {
                !sijill.istiada_tammat && matches!(sijill.naw, NawTaghyeer::MujalladMudaf)
            })
            .map(|(masar, _)| masar.clone())
            .collect();
        mujalladat.sort_by(|awwal, thani| thani.cmp(awwal));
        for masar in mujalladat {
            self.azil_mujallad(&masar, &mut taqreer)?;
        }

        Ok(taqreer)
    }

    /// Undoes one file and records that it is done before returning.
    fn istiada_wahid(
        &mut self,
        masar: &str,
        taqreer: &mut TaqreerIstiada,
    ) -> Result<(), KhataNusus> {
        let Some(sijill) = self.bayan.sijillat.get(masar) else {
            return Ok(());
        };
        if sijill.istiada_tammat {
            return Ok(());
        }
        let naw = sijill.naw;
        let nuskha = sijill.nuskha.clone();
        let muallana = sijill.basma_asliya;
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, masar)?;

        match naw {
            NawTaghyeer::Tadeel => {
                let (Some(miftah), Some(muallana)) = (nuskha, muallana) else {
                    return Err(KhataNusus::NuskhaMafquda {
                        masar: mutlaq,
                        sabab: "the manifest records this file as modified and names no \
                                backup or no original fingerprint for it"
                            .to_owned(),
                    });
                };
                let masar_nuskha = self.mujallad_asl.join(&miftah);
                let asli =
                    masarat::qira(&masar_nuskha).map_err(|khata| KhataNusus::NuskhaMafquda {
                        masar: masar_nuskha.clone(),
                        sabab: khata.injilizi,
                    })?;
                masarat::kitaba_dharra(&mutlaq, &asli).map_err(|khata| KhataNusus::KhataMalaf {
                    masar: mutlaq.clone(),
                    sabab: std::io::Error::other(khata.injilizi),
                })?;

                // Re-read what actually landed, rather than trusting the write.
                let mahsuba = basma_malaf(&mutlaq)?;
                if mahsuba != muallana {
                    return Err(KhataNusus::IstiadaGhayrMutabaqa {
                        masar: mutlaq,
                        muallana: muallana.to_string(),
                        mahsuba: mahsuba.to_string(),
                    });
                }
                taqreer.mustaada = taqreer.mustaada.saturating_add(1);
            },
            NawTaghyeer::Idafa => {
                masarat::hadhf(&mutlaq).map_err(|khata| KhataNusus::KhataMalaf {
                    masar: mutlaq.clone(),
                    sabab: std::io::Error::other(khata.injilizi),
                })?;
                taqreer.mahdhufa = taqreer.mahdhufa.saturating_add(1);
            },
            NawTaghyeer::MujalladMudaf => return Ok(()),
        }

        if let Some(sijill) = self.bayan.sijillat.get_mut(masar) {
            sijill.istiada_tammat = true;
        }
        self.iktub_bayan()
    }

    /// Removes one created directory, but only if nothing is left in it.
    fn azil_mujallad(
        &mut self,
        masar: &str,
        taqreer: &mut TaqreerIstiada,
    ) -> Result<(), KhataNusus> {
        let Some(sijill) = self.bayan.sijillat.get(masar) else {
            return Ok(());
        };
        if sijill.istiada_tammat || !matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
            return Ok(());
        }
        let mutlaq = dakhil_aw_khata(&self.jidhr_luba, masar)?;

        let khali = fs::read_dir(&mutlaq).is_ok_and(|mut madkhalat| madkhalat.next().is_none());
        if !mutlaq.exists() || (khali && fs::remove_dir(&mutlaq).is_ok()) {
            taqreer.mujalladat_muzala = taqreer.mujalladat_muzala.saturating_add(1);
        } else {
            // Somebody else's files are in here, or the platform will not let go
            // of it. Neither is a failure of the uninstall, and deleting it
            // anyway would destroy work Taarib did not create.
            taqreer.mujalladat_matruka = taqreer.mujalladat_matruka.saturating_add(1);
        }

        if let Some(sijill) = self.bayan.sijillat.get_mut(masar) {
            sijill.istiada_tammat = true;
        }
        self.iktub_bayan()
    }

    /// Re-hashes every recorded path and reports what has changed underneath the
    /// patch.
    ///
    /// This is how a game updated after installation is detected. The store
    /// pushes a build, the launcher rewrites `data.win`, and a patch that was
    /// installed against the old bytes is now sitting on top of new ones — or has
    /// been silently reverted. Comparing fingerprints answers that without asking
    /// the user to remember whether anything updated.
    ///
    /// Reads nothing into the heap: files are hashed through a read-only mapping,
    /// so verifying an installation costs no more memory than verifying one file.
    ///
    /// # Errors
    ///
    /// [`KhataNusus::KhataMalaf`] when a recorded path exists and cannot be
    /// hashed, and [`KhataNusus::NuskhaMafquda`] when a recorded path is not a
    /// path inside the game's root — which means the manifest was edited outside
    /// Taarib and none of it can be acted on.
    pub fn tahaqquq(&self) -> Result<TaqreerTahaqquq, KhataNusus> {
        let mut taqreer = TaqreerTahaqquq {
            halat: BTreeMap::new(),
            zaida: Vec::new(),
            adad_munharif: 0,
            adad_mafqud: 0,
            adad_mustaad: 0,
        };

        for (masar, sijill) in &self.bayan.sijillat {
            let mutlaq = dakhil_aw_khata(&self.jidhr_luba, masar)?;
            let hala = if matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
                if mutlaq.is_dir() {
                    HalatMalaf::Mutabiq
                } else {
                    HalatMalaf::Mafqud
                }
            } else if mutlaq.is_file() {
                let mahsuba = basma_malaf(&mutlaq)?;
                match sijill.basma_maktuba {
                    None => HalatMalaf::GhayrMuhaqqaq,
                    Some(muallana) if muallana == mahsuba => HalatMalaf::Mutabiq,
                    Some(muallana) => {
                        if sijill.basma_asliya == Some(mahsuba) {
                            HalatMalaf::Ustuidat
                        } else {
                            HalatMalaf::Munharif { muallana, mahsuba }
                        }
                    },
                }
            } else {
                HalatMalaf::Mafqud
            };

            match hala {
                HalatMalaf::Munharif { .. } => {
                    taqreer.adad_munharif = taqreer.adad_munharif.saturating_add(1);
                },
                HalatMalaf::Mafqud => {
                    taqreer.adad_mafqud = taqreer.adad_mafqud.saturating_add(1);
                },
                HalatMalaf::Ustuidat => {
                    taqreer.adad_mustaad = taqreer.adad_mustaad.saturating_add(1);
                },
                HalatMalaf::Mutabiq | HalatMalaf::GhayrMuhaqqaq => {},
            }
            let _ = taqreer.halat.insert(masar.clone(), hala);
        }

        for (masar, sijill) in &self.bayan.sijillat {
            if !matches!(sijill.naw, NawTaghyeer::MujalladMudaf) {
                continue;
            }
            let mutlaq = dakhil_aw_khata(&self.jidhr_luba, masar)?;
            for madkhal in walkdir::WalkDir::new(&mutlaq)
                .into_iter()
                .filter_map(Result::ok)
            {
                if !madkhal.file_type().is_file() {
                    continue;
                }
                let Some(nisbi) = nisbi_min(&self.jidhr_luba, madkhal.path()) else {
                    continue;
                };
                if !self.bayan.sijillat.contains_key(&nisbi) {
                    taqreer.zaida.push(nisbi);
                }
            }
        }
        taqreer.zaida.sort();
        taqreer.zaida.dedup();

        Ok(taqreer)
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Normalizes a caller's relative path to the one form the manifest stores.
///
/// Backslashes become forward slashes, leading and trailing separators go, and
/// repeated separators collapse. Without this, `js/plugins/taarib.js` and
/// `js\plugins\taarib.js` are two manifest keys for one file — which means two
/// backups, the second one taken after the first write, holding the patched
/// bytes as though they were the original.
fn masar_muwahhad(nisbi: &str) -> String {
    let mubaddal = nisbi.replace('\\', "/");
    let maqsus = mubaddal.trim_matches('/');
    let mut nateeja = String::with_capacity(maqsus.len());
    let mut sabiq_fasil = false;
    for harf in maqsus.chars() {
        if harf == '/' {
            if sabiq_fasil {
                continue;
            }
            sabiq_fasil = true;
        } else {
            sabiq_fasil = false;
        }
        nateeja.push(harf);
    }
    nateeja
}

/// Joins a relative path onto the game's root through the workspace's only
/// path-safety check, reporting a refusal as a preservation failure.
fn dakhil_aw_khata(jidhr: &Path, nisbi: &str) -> Result<PathBuf, KhataNusus> {
    masarat::dakhil(jidhr, nisbi).map_err(|khata| KhataNusus::NuskhaMafquda {
        masar: jidhr.to_path_buf(),
        sabab: format!(
            "{nisbi} is not a path inside the game's folder: {}",
            khata.injilizi
        ),
    })
}

/// Creates a directory, reporting a failure as a preservation failure.
fn insha_aw_khata(masar: &Path) -> Result<(), KhataNusus> {
    masarat::insha_mujallad(masar).map_err(|khata| KhataNusus::NuskhaMafquda {
        masar: masar.to_path_buf(),
        sabab: khata.injilizi,
    })
}

/// The file name a preserved original is stored under.
///
/// Three parts, each load-bearing. The leading `n` guarantees the name's first
/// dot-separated component is never a Windows device name, so a game file
/// honestly called `con.json` does not turn into a handle on a serial port. The
/// middle is the file's own name with anything outside `[A-Za-z0-9._-]` replaced,
/// so a person browsing `nusakh/` can see what they are looking at. The suffix is
/// eight bytes of BLAKE3 over the *whole relative path*, which is what keeps
/// `data/Map001.json` and `js/Map001.json` in separate backups instead of one
/// overwriting the other.
fn miftah_nuskha(masar: &str) -> String {
    let ism = masar.rsplit('/').next().unwrap_or(masar);
    let mut nazif = String::with_capacity(HADD_ISM_MIFTAH);
    for harf in ism.chars().take(HADD_ISM_MIFTAH) {
        if harf.is_ascii_alphanumeric() || harf == '.' || harf == '-' || harf == '_' {
            nazif.push(harf);
        } else {
            nazif.push('_');
        }
    }
    while nazif.ends_with('.') {
        let _ = nazif.pop();
    }

    let basma = blake3::hash(masar.as_bytes());
    let mut lahiqa = String::with_capacity(16);
    for qeema in basma.as_bytes().iter().take(8) {
        let _ = write!(lahiqa, "{qeema:02x}");
    }
    format!("n{nazif}.{lahiqa}.asl")
}

/// How many bytes two buffers disagree about, counting a length difference as a
/// disagreement for every byte one of them does not have.
fn adad_ikhtilaf(awwal: &[u8], thani: &[u8]) -> u64 {
    let mushtarak = awwal
        .iter()
        .zip(thani.iter())
        .filter(|(a, b)| a != b)
        .count();
    tul_u64(mushtarak).saturating_add(tul_u64(awwal.len().abs_diff(thani.len())))
}

/// An absolute path expressed the way the manifest stores it, or [`None`] when
/// it is not inside the root or cannot be written as UTF-8.
fn nisbi_min(jidhr: &Path, masar: &Path) -> Option<String> {
    let baqi = masar.strip_prefix(jidhr).ok()?;
    let mut nateeja = String::new();
    for juz in baqi.components() {
        let std::path::Component::Normal(ism) = juz else {
            return None;
        };
        let nass = ism.to_str()?;
        if !nateeja.is_empty() {
            nateeja.push('/');
        }
        nateeja.push_str(nass);
    }
    Some(nateeja)
}
