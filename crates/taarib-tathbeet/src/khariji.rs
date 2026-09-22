//! تثبيت الرقعة الخارجية — installing a patch Taarib did not build, and taking
//! it back out again exactly.
//!
//! Most finished Arabic game translations are for engines Taarib has no adapter
//! for, and they install by replacing files the game ships. Taarib cannot
//! compile one. What it can do is install one **better than its own installer
//! can**, and that is the entire value proposition of this module: a user picks
//! Taarib over the author's own `.exe` because Taarib can undo it, and if the
//! uninstall does not put the game back byte for byte then there is no reason
//! for any of this to exist.
//!
//! ## The sequence, and why it is in this order
//!
//! 1. **The permit is checked against the entry.** [`IdhnTathbeetKhariji`]
//!    carries what the gate approved — the game, the lineage, the release and
//!    every artifact by name, size and digest. The entry handed to the install
//!    is a document that could have been edited since. Anything they disagree
//!    about is [`KhataTathbeet::IdhnKharijiGhayrMutabiq`], before a byte moves.
//! 2. **The game is not running**, through the same guard the signed install
//!    uses.
//! 3. **Nobody else holds this game**, in both directions — see
//!    [`crate::tasadum`].
//! 4. **Fetch and verify, into staging.** [`crate::jalb_khariji`] downloads each
//!    artifact into a directory that is not inside a game and reproduces both
//!    halves of its pin there. Nothing has touched the game yet, and a mismatch
//!    at this point costs the user nothing at all.
//! 5. **The manifest is opened.** [`Tathbeet::ibda`] creates the backup
//!    directory and flushes an empty manifest. **This is the rollback point.**
//!    Before it, there is nothing to undo; after it, every single change is
//!    recorded on the device *before* it is made, so [`azil_khariji`] can undo
//!    any prefix of the work — including a run that died halfway.
//! 6. **The author's remove-list is honoured, backed up first.** RTEA's own
//!    instructions say to delete `lml/`, `version.dll`, `ScriptHookRDR2.dll`,
//!    `vfs.asi` and a dozen more before installing, because they conflict with
//!    what is about to be written. Taarib deletes them — the author is right
//!    about the conflict — through [`Tathbeet::ihfaz_wa_ihdhif`], which
//!    compresses the original into the backup directory and flushes the manifest
//!    line naming it *before* the file goes. That is the one thing the author's
//!    installer cannot do, and it is the reason this module is worth writing.
//! 7. **The archives are unpacked through the recorder**, entry by entry, each
//!    one checked against the game root and against the layout the entry
//!    declares. Nothing is written by anything but [`crate::bayan::Muthabbit`],
//!    so nothing lands outside the manifest.
//!
//! ## Where the backups live
//!
//! Under `<the game's backup directory>/khariji/<lineage>/`, which is beside
//! rather than inside the `nass/` and `sawt/` directories Taarib's own installs
//! use. Three things fall out of that and all three are wanted: a third-party
//! install and a Taarib install never share a manifest; two third-party entries
//! never share one either; and an uninstall of one walks its own manifest and
//! has no expression in it that names the other's files.
//!
//! ## What is deleted and what is emptied
//!
//! A remove-list path that names a directory has its **files** removed and the
//! directory itself left standing. A directory has no bytes to preserve, so
//! removing one is a change nothing can put back; and emptying it is the whole
//! of what the author's instruction is for, since a loader reads files out of
//! `lml/` and an empty `lml/` loads nothing. A symbolic link is left alone for
//! the same reason: a link is not bytes, and Taarib removes only what it can
//! give back.

use std::fs::File;
use std::io::{BufReader, Read as _};
use std::path::{Path, PathBuf};

use taarib_aman::IdhnTathbeetKhariji;
use taarib_kashf::beea::hall_bila_hala;
use taarib_mustalahat::khariji::{RuqaaKharijiya, TahdheerKhariji, TakhtitKhariji};
use taarib_mustalahat::ruqaa::RuqaaId;
use walkdir::WalkDir;

use crate::bayan::{Muthabbit as _, NawTathbeet, TarifLuba, Tathbeet, nisbi_min};
use crate::jalb_khariji::{NaqilKhariji, QitaaMuhaqqaqa, ijlib_qitaa, nazzif_marhala};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io, tul_u64};
use crate::masar_tathbeet::la_tashtaghil;
use crate::mawdi::WajhatLuba;
use crate::taraju::{RadLaShay, SiyasatIstiada, TaqreerIstiada, istiada_nass};
use crate::tasadum::{la_yatasadam_maa_khariji, la_yatasadam_maa_taarib};

/// The subdirectory of a game's backup directory third-party installs live in.
pub const MUJALLAD_KHARIJI: &str = "khariji";

/// The largest single archive entry this build will unpack, in bytes.
///
/// One gibibyte. RTEA's largest is a few megabytes; the ceiling exists so an
/// entry that declares an absurd expanded size is a refusal naming the number
/// rather than an exhausted machine, and it is checked against the size the
/// archive *declares* before a byte is read.
pub const AQSA_HAJM_MADKHAL: u64 = 1024 * 1024 * 1024;

/// The largest total one entry's archives may expand to, in bytes.
///
/// Eight gibibytes, which is comfortably above a full voice pack and well below
/// what a decompression bomb wants. Accumulated across every artifact of one
/// install, not per archive, because the bomb splits trivially otherwise.
pub const AQSA_HAJM_KULLI: u64 = 8 * 1024 * 1024 * 1024;

/// The most entries one install will unpack.
///
/// RTEA is 2,223 language databases plus a handful of others; a quarter of a
/// million is far past anything real and short of what an index-bomb needs.
pub const AQSA_ADAD_MADAKHIL: usize = 250_000;

/// The backup directory one third-party entry occupies for one game.
///
/// The full lineage rather than its short form: two entries whose identities
/// agree in the first eight characters would otherwise share a manifest, and
/// sharing a manifest is how one uninstall restores the other's originals.
#[must_use]
pub fn jidhr_nusakh_khariji(jidhr_nusakh: &Path, ruqaa: RuqaaId) -> PathBuf {
    jidhr_nusakh
        .join(MUJALLAD_KHARIJI)
        .join(ruqaa.uuid().as_simple().to_string())
}

/// Every third-party install recorded under one game's backup directory.
///
/// The directories, in a stable order, each holding a manifest. Read from the
/// filesystem rather than from the database, for the reason the manifest exists
/// at all: a user who copied their game and their `nusakh/` directory to another
/// machine must still be able to see what is installed and take it off.
#[must_use]
pub fn kharijiyat_mathbita(jidhr_nusakh: &Path) -> Vec<PathBuf> {
    let Ok(madakhil) = std::fs::read_dir(jidhr_nusakh.join(MUJALLAD_KHARIJI)) else {
        return Vec::new();
    };
    let mut mathbita: Vec<PathBuf> = madakhil
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| Tathbeet::mawjud(masar, NawTathbeet::Nass))
        .collect();
    mathbita.sort();
    mathbita
}

/// What the caller supplies to install one third-party entry.
pub struct TalabTathbeetKhariji<'a> {
    /// The game, its launcher and its root.
    ///
    /// [`TarifLuba::ruqaa`] must be the entry's own lineage: it is what the
    /// manifest records, and a manifest recording a different lineage is a
    /// manifest that cannot be matched back to what installed it.
    pub luba: &'a TarifLuba,
    /// The catalogue entry being installed.
    pub ruqaa: &'a RuqaaKharijiya,
    /// The game executable's name, to refuse installing while it runs.
    pub tanfidhi: &'a str,
    /// The game's backup directory, shared with Taarib's own installs.
    pub jidhr_nusakh: &'a Path,
    /// Where artifacts are fetched and verified before anything is written.
    ///
    /// Anywhere but inside the game. This call carves out its own subdirectory
    /// under it and removes what it put there before returning.
    pub jidhr_tajmee: &'a Path,
}

impl std::fmt::Debug for TalabTathbeetKhariji<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabTathbeetKhariji")
            .field("luba", &self.luba.ism)
            .field("ruqaa", &self.ruqaa.id)
            .field("isdar", &self.ruqaa.isdar)
            .finish()
    }
}

/// What one third-party installation did.
#[derive(Debug)]
pub struct NatijatTathbeetKhariji {
    /// Where this entry's manifest and backups are, for an uninstall.
    pub jidhr_nusakh: PathBuf,
    /// How many paths the archives wrote.
    pub adad_maktub: usize,
    /// How many files the author's remove-list took out, each backed up first.
    pub adad_mahdhuf: usize,
    /// The warnings the person acknowledged, recorded as shown.
    pub tahdheerat: Vec<TahdheerKhariji>,
}

/// Installs one third-party entry, recording every change before making it.
///
/// `idhn` is taken by value, as the signed path's authorisation is: the permit
/// is spent on one install and cannot be reused for a second.
///
/// # Errors
///
/// [`KhataTathbeet::IdhnKharijiGhayrMutabiq`] when the permit and the entry
/// disagree; [`KhataTathbeet::LubaTashtaghil`] or
/// [`KhataTathbeet::HalatLubaMajhula`] when the game is running or that cannot
/// be established; [`KhataTathbeet::TasadumRuqaa`] when Taarib's own patch or
/// somebody else's is already in the game; whatever [`ijlib_qitaa`] refuses when
/// an artifact does not reproduce its pin — in every one of those cases nothing
/// in the game has been touched; and afterwards whatever the manifest, the
/// backups or the archives raise.
///
/// A failure **after** the manifest is opened leaves the backup directory
/// complete: every change made up to that point is recorded with its original
/// beside it, and [`azil_khariji`] over the same game root, backup directory and
/// lineage puts the game back. That is the rollback, and it is deliberately the
/// caller's call rather than something taken automatically: a restore that fails
/// in turn has its own report to make, and burying it inside the install's
/// failure would lose it.
pub fn thabbit_khariji(
    talab: &TalabTathbeetKhariji<'_>,
    idhn: IdhnTathbeetKhariji,
    naqil: &mut dyn NaqilKhariji,
) -> NatijatTathbeet<NatijatTathbeetKhariji> {
    tabiq_idhn(talab, &idhn)?;
    la_tashtaghil(talab.tanfidhi)?;
    la_yatasadam_maa_taarib(&talab.luba.jidhr, talab.jidhr_nusakh)?;
    la_yatasadam_maa_khariji(&talab.luba.jidhr, talab.jidhr_nusakh)?;

    let jidhr_marhala = talab
        .jidhr_tajmee
        .join(talab.ruqaa.id.uuid().as_simple().to_string());
    // Everything up to `naffidh` can fail without the game having changed; the
    // manifest it opens is the rollback point. The staging directory is cleaned
    // on every one of those paths, because a half-fetched run that left a
    // verified artifact behind is a file the next run would otherwise find under
    // its final name and could be tempted to trust.
    let natija = ijlib_qitaa(naqil, &idhn, &talab.ruqaa.mira, &jidhr_marhala)
        .and_then(|qitaa| naffidh(talab, idhn, &qitaa));
    nazzif_marhala(&jidhr_marhala);
    natija
}

/// Removes one third-party entry and puts the game back.
///
/// A thin name over [`istiada_nass`] pointed at this entry's own backup
/// directory, and thin on purpose: the restore machinery that verifies every
/// byte, reapplies every timestamp and every permission, resumes an interrupted
/// run and refuses to report a partial removal as a success is the same
/// machinery a Taarib patch comes off with. There is no second restore path
/// here to be worse than that one.
///
/// It cannot touch Taarib's own files, or another entry's, and that is
/// structural rather than careful: the manifest it walks is the one inside
/// [`jidhr_nusakh_khariji`], and a restore names no path that is not in the
/// manifest it opened.
///
/// [`RadLaShay`] is the settings writer, because a third-party install changes
/// no launcher setting — it writes files into a game directory and nothing else.
///
/// # Errors
///
/// [`KhataTathbeet::BayanTalif`] when this entry has no manifest under the
/// game's backup directory, or it cannot be read, and
/// [`KhataTathbeet::IstiadaNaqisa`] when the restore starts and cannot finish,
/// carrying what completed and every path that did not.
pub fn azil_khariji(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    ruqaa: RuqaaId,
    siyasa: SiyasatIstiada,
) -> NatijatTathbeet<TaqreerIstiada> {
    let jidhr = jidhr_nusakh_khariji(jidhr_nusakh, ruqaa);
    istiada_nass(jidhr_luba, &jidhr, siyasa, &mut RadLaShay)
}

/// Refuses every way the permit and the entry can disagree.
///
/// The permit is the authority throughout. It was minted by the gate over the
/// entry as the gate read it, and the entry reaching the installer is a
/// separate value that something could have replaced in between — with a
/// different lineage, a different release, or the same artifact names pinned to
/// different bytes. Comparing them here is what makes "the gate approved this"
/// mean the same thing at both ends.
fn tabiq_idhn(talab: &TalabTathbeetKhariji<'_>, idhn: &IdhnTathbeetKhariji) -> NatijatTathbeet<()> {
    let marfud = |sabab: String| KhataTathbeet::IdhnKharijiGhayrMutabiq { sabab };

    if !idhn.yushmal(talab.luba.luba, talab.ruqaa.id) {
        return Err(marfud(format!(
            "it authorises {} in {:?} and the install is {} in {:?}",
            idhn.ruqaa(),
            idhn.luba(),
            talab.ruqaa.id,
            talab.luba.luba
        )));
    }
    if talab.luba.ruqaa != talab.ruqaa.id {
        return Err(marfud(format!(
            "the manifest would record lineage {} for an install of {}",
            talab.luba.ruqaa, talab.ruqaa.id
        )));
    }
    if idhn.isdar() != talab.ruqaa.isdar {
        return Err(marfud(format!(
            "it approved release {:?} and the entry now says {:?}",
            idhn.isdar(),
            talab.ruqaa.isdar
        )));
    }
    if idhn.qitaa().is_empty() {
        return Err(marfud(
            "it approved no artifacts at all, so there is nothing to install".to_owned(),
        ));
    }
    for approved in idhn.qitaa() {
        let Some(hali) = talab
            .ruqaa
            .qitaa
            .iter()
            .find(|qitaa| qitaa.ism == approved.ism)
        else {
            return Err(marfud(format!(
                "it approved {:?} and the entry no longer carries an artifact of that name",
                approved.ism
            )));
        };
        if !approved.yutabiq(hali) {
            return Err(marfud(format!(
                "it approved {:?} at {} byte(s) / {} and the entry now pins {} byte(s) / {}",
                approved.ism, approved.hajm, approved.sha256, hali.hajm, hali.sha256
            )));
        }
    }
    Ok(())
}

/// Everything from the rollback point on.
fn naffidh(
    talab: &TalabTathbeetKhariji<'_>,
    idhn: IdhnTathbeetKhariji,
    qitaa: &[QitaaMuhaqqaqa],
) -> NatijatTathbeet<NatijatTathbeetKhariji> {
    let jidhr_nusakh = jidhr_nusakh_khariji(talab.jidhr_nusakh, talab.ruqaa.id);
    let huwiya = talab.ruqaa.id.uuid().as_simple().to_string();
    let mut tathbeet = Tathbeet::ibda(&jidhr_nusakh, NawTathbeet::Nass, talab.luba, huwiya)?;

    // The author's remove-list first, exactly as the author's own instructions
    // order it: the files it names conflict with what is about to be written,
    // and a loader left in place beside its replacement is the state the
    // instruction exists to avoid.
    let adad_mahdhuf = ikhli_qaimat_alhadhf(&mut tathbeet, &talab.ruqaa.takhtit)?;

    let mut mizan = Mizan::default();
    let mut adad_maktub = 0_usize;
    for wahid in qitaa {
        adad_maktub = adad_maktub.saturating_add(fakk_arshif(
            &mut tathbeet,
            wahid,
            &talab.ruqaa.takhtit,
            &mut mizan,
        )?);
    }

    // No [`crate::tahaqquq`] sweep closes this install, and that is deliberate
    // rather than an omission. The sweep counts a recorded path that is not on
    // disk as missing, which is exactly the state the author's remove-list
    // leaves behind on purpose — so running it here would report every file
    // Taarib correctly removed as a fault, on every install, forever. What the
    // sweep would have proved is already proved a stronger way: every write
    // went through a guard that re-read what landed and recorded its
    // fingerprint into the manifest before this function could return.
    Ok(NatijatTathbeetKhariji {
        jidhr_nusakh,
        adad_maktub,
        adad_mahdhuf,
        tahdheerat: idhn.ila_tahdheerat(),
    })
}

/// Backs up and removes everything the author's instructions say conflicts.
///
/// Returns how many files were taken out. A path the user's game never had is
/// not an error and is not recorded: a manifest line for a file that never
/// existed is a line whose "restore" writes a file into a game that did not have
/// it.
fn ikhli_qaimat_alhadhf(
    tathbeet: &mut Tathbeet,
    takhtit: &TakhtitKhariji,
) -> NatijatTathbeet<usize> {
    let jidhr_luba = tathbeet.jidhr_luba().to_path_buf();
    // Every path is validated before the first one is removed. A malformed entry
    // halfway down an otherwise good list would otherwise leave a game with some
    // of its loader gone and the patch not written — recoverable through the
    // manifest, and still a state worth never reaching.
    let masarat: Vec<WajhatLuba> = takhtit
        .yahdhif
        .iter()
        .map(|khaam| WajhatLuba::jadeed(khaam))
        .collect::<NatijatTathbeet<_>>()?;

    let mut adad = 0_usize;
    for wajha in &masarat {
        let mutlaq = hall_bila_hala(&jidhr_luba, Path::new(wajha.nisbi()));
        let Ok(bayanat) = std::fs::symlink_metadata(&mutlaq) else {
            continue;
        };
        if bayanat.is_symlink() {
            // A link is not bytes. Preserving one would mean preserving whatever
            // it points at — possibly outside the game — and restoring it would
            // put a real file where a link used to be.
            continue;
        }

        if bayanat.is_dir() {
            for malaf in malaffat_tahta(&mutlaq) {
                adad =
                    adad.saturating_add(usize::from(ihdhif_wahid(tathbeet, &jidhr_luba, &malaf)?));
            }
        } else {
            adad = adad.saturating_add(usize::from(ihdhif_wahid(tathbeet, &jidhr_luba, &mutlaq)?));
        }
    }
    Ok(adad)
}

/// Every regular file under one directory, deepest first, links not followed.
fn malaffat_tahta(jidhr: &Path) -> Vec<PathBuf> {
    WalkDir::new(jidhr)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|madkhal| madkhal.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .collect()
}

/// Preserves one file and removes it, returning whether anything was there.
fn ihdhif_wahid(
    tathbeet: &mut Tathbeet,
    jidhr_luba: &Path,
    mutlaq: &Path,
) -> NatijatTathbeet<bool> {
    let Some(nisbi) = nisbi_min(jidhr_luba, mutlaq) else {
        return Err(KhataTathbeet::MasarKharij {
            masar: mutlaq.to_path_buf(),
            jidhr: jidhr_luba.to_path_buf(),
            sabab: "a path on this patch's remove-list is not inside the game directory".to_owned(),
        });
    };
    tathbeet.ihfaz_wa_ihdhif(&nisbi)
}

/// What one install is allowed to expand in total.
#[derive(Debug, Default)]
struct Mizan {
    hajm: u64,
    adad: usize,
}

impl Mizan {
    /// Accounts for one entry, refusing once the install passes its ceilings.
    fn hisab(&mut self, hajm: u64) -> NatijatTathbeet<()> {
        self.adad = self.adad.saturating_add(1);
        if self.adad > AQSA_ADAD_MADAKHIL {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "archive entries in one third-party install",
                qeema: tul_u64(self.adad),
                saqf: tul_u64(AQSA_ADAD_MADAKHIL),
            });
        }
        self.hajm = self.hajm.saturating_add(hajm);
        if self.hajm > AQSA_HAJM_KULLI {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "the total one third-party install expands to",
                qeema: self.hajm,
                saqf: AQSA_HAJM_KULLI,
            });
        }
        Ok(())
    }
}

/// Unpacks one verified archive through the recorder, entry by entry.
///
/// Every write goes through [`Muthabbit`](crate::bayan::Muthabbit), so every
/// file is either preserved before being overwritten or recorded as an addition
/// to be deleted, and the directories the install brings into existence are
/// recorded with it. There is no path in this function that opens a file in a
/// game directory itself.
fn fakk_arshif(
    tathbeet: &mut Tathbeet,
    qitaa: &QitaaMuhaqqaqa,
    takhtit: &TakhtitKhariji,
    mizan: &mut Mizan,
) -> NatijatTathbeet<usize> {
    let malaf = File::open(qitaa.masar())
        .map_err(|sabab| min_khata_io(qitaa.masar(), "opening a staged archive", sabab))?;
    let mut arshif = zip::ZipArchive::new(BufReader::new(malaf)).map_err(|sabab| {
        KhataTathbeet::ArshifTalif {
            masar: qitaa.masar().to_path_buf(),
            sabab: sabab.to_string(),
        }
    })?;

    let jidhr_luba = tathbeet.jidhr_luba().to_path_buf();
    let mut adad = 0_usize;
    for fihris in 0..arshif.len() {
        let mut madkhal = arshif
            .by_index(fihris)
            .map_err(|sabab| KhataTathbeet::ArshifTalif {
                masar: qitaa.masar().to_path_buf(),
                sabab: sabab.to_string(),
            })?;

        let khaam = madkhal.name().to_owned();
        let hukm = HukmAlqari::min_madkhal(&madkhal);
        let mujallad = madkhal.is_dir();
        let hajm = madkhal.size();

        let wajha = madkhal_aamin(qitaa.ism(), &khaam, hukm, &jidhr_luba, takhtit)?;
        mizan.hisab(hajm)?;

        let mutlaq = jidhr_luba.join(wajha.nisbi());
        if mujallad {
            tathbeet.ansha_mujallad(&mutlaq)?;
            continue;
        }

        if hajm > AQSA_HAJM_MADKHAL {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "an archive entry in a third-party patch",
                qeema: hajm,
                saqf: AQSA_HAJM_MADKHAL,
            });
        }
        let bayt = iqra_madkhal(&mut madkhal, qitaa, &khaam, hajm)?;
        drop(madkhal);

        // Which of the two recording routes this is comes from the manifest
        // first and the filesystem second. A path this run already preserved —
        // because the remove-list took it out a moment ago — is a modification
        // whose original is already held, and treating it as an addition would
        // arm the uninstall to delete a file the game shipped.
        if tathbeet.mahfuz(wajha.nisbi()) || mutlaq.exists() {
            tathbeet.iktub(&mutlaq, &bayt)?;
        } else {
            tathbeet.ansha(&mutlaq, &bayt)?;
        }
        adad = adad.saturating_add(1);
    }
    Ok(adad)
}

/// Reads one archive entry, refusing a body longer than it declared.
///
/// The declared size is a ceiling rather than an allocation: a zip header is
/// written by whoever packed the archive, so it says how much to *refuse* past,
/// not how much to trust.
fn iqra_madkhal<R: std::io::Read>(
    madkhal: &mut R,
    qitaa: &QitaaMuhaqqaqa,
    khaam: &str,
    hajm: u64,
) -> NatijatTathbeet<Vec<u8>> {
    let saqf = hajm.min(AQSA_HAJM_MADKHAL);
    let mut bayt = Vec::new();
    let maqru = madkhal
        .take(saqf.saturating_add(1))
        .read_to_end(&mut bayt)
        .map_err(|sabab| min_khata_io(qitaa.masar(), "expanding an archive entry", sabab))?;
    if tul_u64(maqru) > saqf {
        return Err(KhataTathbeet::ArshifTalif {
            masar: qitaa.masar().to_path_buf(),
            sabab: format!(
                "the entry {khaam:?} expands past the {hajm} byte(s) its own header declares"
            ),
        });
    }
    Ok(bayt)
}

/// What the archive reader itself says about one entry, before its name is read.
///
/// Three states rather than two booleans, because the three are ordered: a link
/// is refused whatever its name says, and a name the reader will not vouch for
/// is refused before this build's own parser is asked about it. Two independent
/// flags would let a caller ask them in the other order, which is the order that
/// reads a link's name and decides it looks fine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HukmAlqari {
    /// A name the reader vouches for as staying inside a directory.
    Mughlaq,
    /// A name the reader will not vouch for.
    Maftuh,
    /// A symbolic link, whatever its name says.
    RabtRamzi,
}

impl HukmAlqari {
    /// Reads the verdict off one archive entry.
    fn min_madkhal<R: std::io::Read>(madkhal: &zip::read::ZipFile<'_, R>) -> Self {
        if madkhal.is_symlink() {
            Self::RabtRamzi
        } else if madkhal.enclosed_name().is_some() {
            Self::Mughlaq
        } else {
            Self::Maftuh
        }
    }
}

/// Validates one archive entry name against the game root and the declared
/// layout.
///
/// Four independent refusals, in the order that costs least:
///
/// * a **symbolic link**, which is a file whose content is a path and whose
///   extraction is a write to wherever that path leads;
/// * a name the archive reader itself will not vouch for, which is its own
///   `..`-and-absolute check and is kept as a second opinion rather than as the
///   only one;
/// * a Windows-shaped escape — `C:\x`, `C:x`, `\\server\share\x` — checked
///   **textually**, because a Unix build's path parser reads `C:\x` as an
///   ordinary file name and would let it through to a Windows machine's
///   extractor;
/// * and [`WajhatLuba::jadeed`], the same validator every destination inside a
///   game goes through, which settles `..`, absolute paths, NUL bytes, reserved
///   device names and components ending in a dot or a space.
///
/// What survives all four is then checked against
/// [`TakhtitKhariji::yasmah`]: inside the game root is not enough, because the
/// entry declares which paths this patch writes and an archive that writes
/// somewhere else is an archive nobody reviewed.
fn madkhal_aamin(
    ism_qitaa: &str,
    khaam: &str,
    hukm: HukmAlqari,
    jidhr_luba: &Path,
    takhtit: &TakhtitKhariji,
) -> NatijatTathbeet<WajhatLuba> {
    let marfud = |sabab: &str| KhataTathbeet::MadkhalKharijAlTakhtit {
        ism: ism_qitaa.to_owned(),
        madkhal: khaam.to_owned(),
        jidhr: jidhr_luba.to_path_buf(),
        sabab: sabab.to_owned(),
    };

    match hukm {
        HukmAlqari::RabtRamzi => {
            return Err(marfud(
                "it is a symbolic link, and a link's content is a path that decides at \
                 extraction time where the write lands",
            ));
        },
        HukmAlqari::Maftuh => {
            return Err(marfud(
                "the archive reader will not vouch for the name as one that stays inside a \
                 directory",
            ));
        },
        HukmAlqari::Mughlaq => {},
    }
    if let Some(sabab) = shakl_windows(khaam) {
        return Err(marfud(sabab));
    }

    let wajha = WajhatLuba::jadeed(khaam)
        .map_err(|_| marfud("it is not a relative path that stays inside the game directory"))?;
    if !takhtit.yasmah(wajha.nisbi()) {
        return Err(marfud(
            "it is inside the game and outside every path this entry declares the patch writes",
        ));
    }
    Ok(wajha)
}

/// The Windows-shaped escapes a Unix path parser does not see.
///
/// Returns the reason when the raw name is one of them. Checked on every
/// platform rather than behind `cfg(windows)`: the archive is the same file
/// wherever it is unpacked, and a check that only fires on the machine the
/// attack targets is a check that never fires on the machine that reviews it.
fn shakl_windows(khaam: &str) -> Option<&'static str> {
    let mubaddal = khaam.replace('\\', "/");
    if mubaddal.starts_with("//") {
        return Some("it is a UNC path, which names a host rather than a place in the game");
    }
    if mubaddal.starts_with('/') {
        return Some("it is an absolute path");
    }
    let mut huruf = mubaddal.chars();
    let (Some(awwal), Some(thani)) = (huruf.next(), huruf.next()) else {
        return None;
    };
    if awwal.is_ascii_alphabetic() && thani == ':' {
        return Some(
            "it carries a drive letter, which on Windows resolves against that drive's own \
             current directory rather than against the game",
        );
    }
    None
}

#[cfg(test)]
mod ikhtibarat {
    use std::path::Path;

    use taarib_mustalahat::khariji::TakhtitKhariji;

    use super::{HukmAlqari, madkhal_aamin, shakl_windows};
    use crate::khata::KhataTathbeet;

    fn takhtit() -> TakhtitKhariji {
        TakhtitKhariji {
            yaktub: vec!["lml".to_owned(), "dinput8.dll".to_owned()],
            yahdhif: Vec::new(),
        }
    }

    fn marfud(khaam: &str, hukm: HukmAlqari) -> Option<String> {
        madkhal_aamin("update.zip", khaam, hukm, Path::new("/luba"), &takhtit())
            .err()
            .map(|khata| match khata {
                KhataTathbeet::MadkhalKharijAlTakhtit { sabab, .. } => sabab,
                akhar => format!("unexpected: {akhar:?}"),
            })
    }

    #[test]
    fn al_madkhal_al_maqbul_yamurr() {
        assert!(
            madkhal_aamin(
                "update.zip",
                "lml/RTEA/Subtitles/texts/a.yldb",
                HukmAlqari::Mughlaq,
                Path::new("/luba"),
                &takhtit(),
            )
            .is_ok()
        );
    }

    #[test]
    fn al_khuruj_min_jidhr_al_luba_marfud() {
        assert!(marfud("../evil.dll", HukmAlqari::Mughlaq).is_some());
        assert!(marfud("/etc/shadow", HukmAlqari::Mughlaq).is_some());
        assert!(marfud("lml/../../evil.dll", HukmAlqari::Mughlaq).is_some());
        // The reader's own verdict is honoured even when the text looks fine.
        assert!(marfud("lml/a.yldb", HukmAlqari::Maftuh).is_some());
        // And a link is refused before anything else is asked about it.
        assert!(
            marfud("lml/a.yldb", HukmAlqari::RabtRamzi)
                .is_some_and(|sabab| sabab.contains("symbolic link"))
        );
    }

    /// The two shapes a Unix path parser reads as ordinary file names.
    #[test]
    fn shakl_windows_yuktashaf_ala_kull_manassa() {
        assert!(shakl_windows("C:\\Windows\\System32\\evil.dll").is_some());
        assert!(shakl_windows("C:evil.dll").is_some());
        assert!(shakl_windows("\\\\server\\share\\evil.dll").is_some());
        assert!(shakl_windows("\\Windows\\evil.dll").is_some());
        assert!(shakl_windows("lml/RTEA/a.yldb").is_none());
        // A colon that is not a drive letter is left to the path validator.
        assert!(shakl_windows("lml/a:b.yldb").is_none());
    }

    /// Inside the game is not enough: the entry says what it writes.
    #[test]
    fn dakhil_al_luba_wa_kharij_al_takhtit_marfud() {
        assert!(
            marfud("redteamassets/x.bin", HukmAlqari::Mughlaq)
                .is_some_and(|sabab| sabab.contains("declares the patch writes"))
        );
        assert!(marfud("lmlx/a.yldb", HukmAlqari::Mughlaq).is_some());
        assert!(marfud("dinput8.dll.bak", HukmAlqari::Mughlaq).is_some());
    }
}
