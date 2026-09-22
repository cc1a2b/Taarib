//! Stage 2 — reading the game's strings, and routing the games where that
//! cannot be done.
//!
//! Two outcomes matter here and only one of them is a failure. A game whose
//! containers are readable produces a table. A game whose containers are not —
//! an IL2CPP release build with the type tree stripped, a title that encrypts
//! its own text — produces an empty table and a refusal report, and the refusal
//! report knows whether runtime capture would recover anything. When it would,
//! this stage says **play the game once with capture enabled, then come back**,
//! and it says it in those words, because that is a path this product supports
//! and not a dead end. The session file that pass produces comes back in
//! through [`crate::talab::TalabTilqai::jalsat_iltiqat`] and is merged here.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use taarib_istikhraj::dammij::{KhiyaratDammij, dammij_iltiqat};
use taarib_istikhraj::iltiqat::{MulahazaMutakarrira, iqra_jalsa};
use taarib_istikhraj::jadwal::JadwalNusus;
use taarib_istikhraj::rafd::TaqreerRafd;
use taarib_mustalahat::muharrik::AilatMuharrik;

use crate::khata::{KhataTilqai, NatijatTilqai, khata_malaf};
use crate::taqaddum::{MarhalaTilqai, Muraqib};
use crate::taqreer::IhsaIstikhraj;

/// How many distinct strings one capture session may contribute.
///
/// A hundred thousand. Generous for any real session — a long RPG draws tens of
/// thousands of distinct strings across a full playthrough — and a bound rather
/// than no bound because the file is written by a process that ran inside
/// somebody's game and this one runs on their desktop.
pub const HADD_JALSA: usize = 100_000;

/// The extracted table and its report, as they are stored between runs.
///
/// One file rather than two because they are one answer: a table without its
/// refusal report cannot say whether it is empty because the game has no text
/// or because nothing could be read, and those are opposite conclusions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JadwalMakhzun {
    /// The table.
    pub jadwal: JadwalNusus,
    /// What was read and what was refused.
    pub rafd: TaqreerRafd,
    /// Whether a capture session contributed to it.
    pub multaqat: bool,
}

impl JadwalMakhzun {
    /// Reads a stored table back.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when the file cannot be read or does not
    /// parse. A resumed run that cannot read its own extraction has to redo it,
    /// and redoing extraction is free; but silently redoing it after a *partial*
    /// read would produce a table that disagrees with the translation journal
    /// keyed against it, so this is an error rather than a fallback.
    pub fn iqra(masar: &Path) -> NatijatTilqai<Self> {
        let bayt = fs::read(masar).map_err(|sabab| khata_malaf(masar, "read", sabab))?;
        serde_json::from_slice(&bayt)
            .map_err(|sabab| khata_malaf(masar, "parsed", std::io::Error::other(sabab)))
    }

    /// Writes it, atomically.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when it cannot be serialized or written.
    pub fn uktub(&self, masar: &Path) -> NatijatTilqai<()> {
        let bayt = serde_json::to_vec(self)
            .map_err(|sabab| khata_malaf(masar, "serialized", std::io::Error::other(sabab)))?;
        taarib_usus::masarat::kitaba_dharra(masar, &bayt).map_err(|sabab| {
            khata_malaf(masar, "written", std::io::Error::other(sabab.to_string()))
        })
    }
}

/// Reads every string this build can read out of the game, and merges a capture
/// session when one is supplied.
///
/// The denominator is [`None`] for the static pass and it is the only place in
/// this crate that is true. `taarib_istikhraj::unity::istakhrij` takes a game
/// root and returns when it is finished; the number of containers it will read
/// is not knowable until it has walked the tree, and the walk *is* the work. A
/// count invented by walking the tree twice would be a different number from
/// the one the extractor uses. The stage reports its real total the moment it
/// has one.
///
/// # Errors
///
/// [`KhataTilqai::JalsaGhayrMaqrua`] when a capture session was named and will
/// not read.
///
/// [`KhataTilqai::YahtajIltiqat`] when nothing was extracted, no capture was
/// supplied, and the refusal report says capture is the remedy.
///
/// [`KhataTilqai::LaNusus`] when nothing was extracted and capture would not
/// help either.
pub fn ijri(
    jidhr: &Path,
    aila: AilatMuharrik,
    jalsa: Option<&Path>,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<(JadwalMakhzun, IhsaIstikhraj)> {
    let (mulahazat, ism_jalsa) = match jalsa {
        Some(masar) => (
            iqra_mulahazat(masar)?,
            masar.file_stem().map_or_else(
                || ISM_JALSA_IFTIRADI.to_owned(),
                |ism| ism.to_string_lossy().into_owned(),
            ),
        ),
        None => (Vec::new(), ISM_JALSA_IFTIRADI.to_owned()),
    };
    ijri_bi_mulahazat(jidhr, aila, &mulahazat, &ism_jalsa, muraqib)
}

/// The name a merged observation's location carries when the session file's own
/// name cannot be read.
const ISM_JALSA_IFTIRADI: &str = "iltiqat";

/// Everything runtime capture has ever observed for one game, beside its runs.
///
/// Extraction rebuilds its table from the game's files on every run, and the
/// captured half of that table came from a session file living in the *game's*
/// directory — which a game update, a store verify, or the next capture
/// overwrites, and which nothing owns. So the first run after a pass carried the
/// `Multaqat` rows and a later one silently did not: the fingerprints differed,
/// the stage re-extracted with no session to merge, and every captured row —
/// with the measured widths and the opening-session membership riding on it —
/// was gone with no way back, because the session had been the only copy.
///
/// Kept here instead, under the game's own runs root, the observations outlive
/// any single run and any single session file, and are re-applied every time
/// extraction runs. This is what makes a capture survive a re-run the way a
/// translation already does.
pub const MALAF_MULAHAZAT: &str = "mulahazat_iltiqat.json";

/// Folds a new session into the game's durable observations and answers all of
/// them.
///
/// Idempotent in the session: re-offering a pass already folded in replaces its
/// records rather than adding to their counts, so a resumed run cannot inflate
/// an occurrence total by repeating work it already did.
///
/// # Errors
///
/// [`KhataTilqai::JalsaGhayrMaqrua`] when the offered session will not read, and
/// a file error when the store itself cannot be read or written.
pub fn ajmaa_mulahazat(
    jidhr_amal: &Path,
    jalsa: Option<&Path>,
) -> NatijatTilqai<Vec<MulahazaMutakarrira>> {
    let masar = jidhr_amal.join(MALAF_MULAHAZAT);
    let mut majmua: BTreeMap<String, MulahazaMutakarrira> = match fs::read(&masar) {
        Ok(bayt) => serde_json::from_slice::<Vec<MulahazaMutakarrira>>(&bayt)
            .map_err(|sabab| KhataTilqai::JalsaGhayrMaqrua {
                masar: masar.clone(),
                sabab: sabab.to_string(),
            })?
            .into_iter()
            .map(|mulahaza| (mulahaza.miftah.clone(), mulahaza))
            .collect(),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(sabab) => return Err(khata_malaf(&masar, "reading the capture store", sabab)),
    };

    let Some(masar_jalsa) = jalsa else {
        return Ok(majmua.into_values().collect());
    };
    for mulahaza in iqra_mulahazat(masar_jalsa)? {
        let _ = majmua.insert(mulahaza.miftah.clone(), mulahaza);
    }
    let mulahazat: Vec<MulahazaMutakarrira> = majmua.into_values().collect();

    // Written before extraction reads it, so a run that dies part way through
    // still leaves the pass recorded: the person played it once and must not be
    // asked to play it again for want of a file this process could have written
    // and did not.
    let bayt = serde_json::to_vec(&mulahazat).map_err(|sabab| KhataTilqai::JalsaGhayrMaqrua {
        masar: masar.clone(),
        sabab: sabab.to_string(),
    })?;
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid)
            .map_err(|sabab| khata_malaf(walid, "creating the capture store", sabab))?;
    }
    fs::write(&masar, &bayt)
        .map_err(|sabab| khata_malaf(&masar, "writing the capture store", sabab))?;
    Ok(mulahazat)
}

/// The fingerprint of a game's accumulated observations.
///
/// Answers the same question [`basmat_jalsa`] answers about one session file —
/// "has extraction already done this work" — over the set extraction actually
/// merges, so folding a new pass in changes it and re-running without one does
/// not.
#[must_use]
pub fn basmat_mulahazat(mulahazat: &[MulahazaMutakarrira]) -> String {
    let mut hasib = blake3::Hasher::new();
    for mulahaza in mulahazat {
        let _ = hasib.update(mulahaza.miftah.as_bytes());
        let _ = hasib.update(&[0]);
        let _ = hasib.update(&mulahaza.marrat.to_le_bytes());
    }
    crate::bina::sittasi(hasib.finalize().as_bytes())
}

/// Reads one capture session file into the observations it accumulated.
///
/// # Errors
///
/// [`KhataTilqai::JalsaGhayrMaqrua`] when the file cannot be opened or parsed.
pub fn iqra_mulahazat(masar: &Path) -> NatijatTilqai<Vec<MulahazaMutakarrira>> {
    let muhammala =
        iqra_jalsa(masar, HADD_JALSA).map_err(|khata| KhataTilqai::JalsaGhayrMaqrua {
            masar: masar.to_path_buf(),
            sabab: khata.injilizi,
        })?;
    Ok(muhammala.sijill.mulahazat().cloned().collect())
}

/// Extraction over observations already in hand, whatever they were read from.
///
/// Split from [`ijri`] so the pipeline can merge everything runtime capture has
/// *ever* recorded for a game rather than whatever single session file happens
/// to be sitting in the game's own directory at the moment it runs.
///
/// # Errors
///
/// The same refusals [`ijri`] raises, minus the session read it no longer does.
pub fn ijri_bi_mulahazat(
    jidhr: &Path,
    aila: AilatMuharrik,
    mulahazat: &[MulahazaMutakarrira],
    ism_jalsa: &str,
    muraqib: &Muraqib<'_>,
) -> NatijatTilqai<(JadwalMakhzun, IhsaIstikhraj)> {
    muraqib.ballagh_bila_majmu(
        MarhalaTilqai::Istikhraj,
        0,
        format!(
            "reading {} containers under {} — the extractor discovers what to read as it walks, \
             so there is no total to count against yet",
            aila.ism(),
            jidhr.display()
        ),
    );

    let (mut jadwal, rafd) = istakhrij(jidhr, aila);
    let bila_mustakhrij = !yuqra(aila);
    let maqrua = tul(rafd.maqrua.len());
    muraqib.ballagh(
        MarhalaTilqai::Istikhraj,
        maqrua,
        maqrua,
        format!(
            "{maqrua} container(s) read, {} refused, {} distinct string(s)",
            rafd.marfuda.len(),
            jadwal.adad()
        ),
    );

    let mut multaqat = false;
    if !mulahazat.is_empty() {
        let taqreer = dammij_iltiqat(
            &mut jadwal,
            mulahazat,
            &KhiyaratDammij::jadeeda(ism_jalsa.to_owned()),
        );
        multaqat = true;
        muraqib.ballagh(
            MarhalaTilqai::Istikhraj,
            maqrua,
            maqrua,
            format!(
                "capture merged: {} observation(s) offered, {} matched a static string, {} kept \
                 on their own",
                taqreer.mulahazat,
                taqreer
                    .mutabaqa_tamma
                    .saturating_add(taqreer.mutabaqa_naqiya)
                    .saturating_add(taqreer.mutabaqa_bi_tabdeel),
                taqreer.multaqat_faqat
            ),
        );
    }

    let ihsa = IhsaIstikhraj {
        adad: tul(jadwal.adad()),
        adad_zahir: tul(jadwal.adad_zahir()),
        maqrua,
        marfuda: tul(rafd.marfuda.len()),
        khasara: tul(rafd.adad_khasara()),
        yanfa_iltiqat: rafd.yanfa_iltiqat(),
        multaqat,
        satrat: rafd.taqreer(),
    };

    if jadwal.khali() {
        // Capture is the remedy in two cases and they are not the same case:
        // the refusal report saying so, and this build having no static reader
        // for the engine at all. The second produces an *empty* report — there
        // is nothing to refuse when nothing was attempted — so asking only
        // `yanfa_iltiqat` would tell a Ren'Py player their game has no text.
        let sabab = if bila_mustakhrij {
            format!(
                "this build reads no container format for {}; every string this game draws has \
                 to be observed at runtime",
                aila.ism()
            )
        } else {
            ihsa.satrat.join("; ")
        };
        return Err(if (ihsa.yanfa_iltiqat || bila_mustakhrij) && !multaqat {
            KhataTilqai::YahtajIltiqat { sabab }
        } else {
            KhataTilqai::LaNusus { sabab }
        });
    }

    // Said out loud when the files gave *something* and the refusal report says
    // capture would reach more.
    //
    // The run used to mention capture only when extraction came back empty, and
    // the common case is the opposite: a Unity release build ships no type tree,
    // so the localization tables read fine and every string living in a
    // component does not. The player then gets a translated menu, an untouched
    // game, a run that reported success, and no reason anywhere. Which is
    // exactly the wrong way round — the more a game hides from a static reader,
    // the more the person needs to be told that playing once with capture on is
    // what reaches the rest.
    if ihsa.yanfa_iltiqat && !multaqat {
        muraqib.ballagh_bila_majmu(
            MarhalaTilqai::Istikhraj,
            maqrua,
            format!(
                "{} container(s) hold text no file reader can open on this game, so part of it \
                 will still be in its original language. Playing once with capture switched on \
                 records what the game actually draws, and running this again with that \
                 recording reaches the rest.",
                ihsa.marfuda
            ),
        );
    }
    muraqib.ikhtim(
        MarhalaTilqai::Istikhraj,
        maqrua,
        format!(
            "{} distinct string(s), {} of them a player sees",
            ihsa.adad, ihsa.adad_zahir
        ),
    );
    Ok((
        JadwalMakhzun {
            jadwal,
            rafd,
            multaqat,
        },
        ihsa,
    ))
}

/// The fingerprint of a capture session file, streamed.
///
/// What a resumed run compares against the one its journal recorded, so that a
/// pass the person played *since* the last run is noticed and a pass that has
/// already been folded in is not extracted a second time. Streamed rather than
/// read whole because a long session is hundreds of megabytes and this is a
/// comparison, not a load.
///
/// # Errors
///
/// [`KhataTilqai::JalsaGhayrMaqrua`] when the file cannot be opened or read —
/// the same refusal the stage itself raises for the same file, brought forward
/// so a resume never answers "nothing changed" about a session it could not
/// read.
pub fn basmat_jalsa(masar: &Path) -> NatijatTilqai<String> {
    let ghayr_maqrua = |sabab: std::io::Error| KhataTilqai::JalsaGhayrMaqrua {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    };
    let mut malaf = fs::File::open(masar).map_err(ghayr_maqrua)?;
    let mut hasib = blake3::Hasher::new();
    std::io::copy(&mut malaf, &mut hasib).map_err(ghayr_maqrua)?;
    Ok(crate::bina::sittasi(hasib.finalize().as_bytes()))
}

/// The extractor for one engine family.
///
/// A family with no extractor is not an error here: it returns an empty table
/// and a report saying nothing was read, which routes through the same emptiness
/// check as a Unity game whose containers were all refused — and produces the
/// same honest answer, which is that this game needs the capture path.
fn istakhrij(jidhr: &Path, aila: AilatMuharrik) -> (JadwalNusus, TaqreerRafd) {
    match aila {
        AilatMuharrik::Unity => taarib_istikhraj::unity::istakhrij(jidhr),
        AilatMuharrik::Unreal => taarib_istikhraj::unreal::istakhrij(jidhr),
        AilatMuharrik::Godot => taarib_istikhraj::godot::istakhrij(jidhr),
        _ => (JadwalNusus::jadeed(), TaqreerRafd::jadeed()),
    }
}

/// Whether this build has a static reader for an engine family at all.
const fn yuqra(aila: AilatMuharrik) -> bool {
    matches!(
        aila,
        AilatMuharrik::Unity | AilatMuharrik::Unreal | AilatMuharrik::Godot
    )
}

/// A count as the report's own width.
fn tul(qeema: usize) -> u64 {
    u64::try_from(qeema).unwrap_or(u64::MAX)
}
