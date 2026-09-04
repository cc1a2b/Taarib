//! أنريل — Unreal's compiled localization, normalized.
//!
//! Everything here is an adapter over the Phase 8 readers in
//! `taarib_muhawwil_unreal::mawarid`. Not one byte of `.locres`, `.locmeta`,
//! `StringTable`, `.pak` or IoStore framing is parsed in this file; the readers
//! own those formats, they were verified against real games, and a second
//! implementation here would be a second opinion about what a version 3 hash
//! pool means.
//!
//! ## Unreal's own key wins, and it is the reason this engine re-matches well
//!
//! A `.locres` entry is a namespace, a key and a string. The namespace and key
//! are chosen by the developer, written into the game's source, and stable
//! across every build the developer ships — that is what a localization key *is
//! for*. So they go into [`MawqiNass::miftah_muharrik`], which makes identity a
//! function of the engine's key instead of a function of the text, and a patch
//! keyed that way survives the developer rewriting a line.
//!
//! That is the difference between Unreal and, say, RPG Maker here. RPG Maker has
//! no localization system, so identity has to be derived and a rewritten line is
//! a new string. Unreal hands Taarib a key, and a rewritten line under an
//! unchanged key comes back as [`crate::jadwal::FarqJadwal::mughayyara`] — the
//! translator is told the source changed, and keeps their work.
//!
//! `FTextLocalizationManager` merges every loaded `.locres` into one
//! namespace-to-key map at runtime, so a namespace and key pair is unique across
//! the whole game by the engine's own contract. The key therefore needs no
//! container prefix to disambiguate it, and adding one would have broken
//! re-matching the first time a publisher moved a localization target between
//! chunks.
//!
//! ## Which culture is the source
//!
//! A shipped game holds one `.locres` per culture, all of them under the same
//! namespaces and keys. Extracting all of them would put the English, French and
//! German text of every line into the table under one identity, and the fold in
//! [`crate::jadwal::JadwalNusus::adif`] would keep whichever was read first.
//!
//! The `.locmeta` beside them names the native culture, so when there is one
//! this module extracts that culture and records the others in the read report
//! as what they are — translations, not sources. When there is no `.locmeta`,
//! every culture is extracted with the culture folded into the engine key, so
//! nothing is lost and no two of them collide. Guessing "English" instead would
//! be wrong for every game written in Japanese, and this product is not going to
//! be the tool that assumes the source language is English.
//!
//! **A declared native culture does not have to exist on disk.** Unreal's own
//! gather pipeline uses `en-US-POSIX` as the source pseudo-culture and ships no
//! `.locres` under that name, so matching it literally makes every real culture a
//! translation of a source that is not there and the target yields nothing at
//! all. [`wafiq_thaqafat`] resolves the declared name against the cultures the
//! build actually ships before any of them is read.
//!
//! The native culture is also what a writer must not clobber:
//! [`iqra_thaqafat`] hands Phase 14 the exact value so that adding `ar` to a
//! `.locmeta` does not silently change which culture the game falls back to.
//! What that function reports is the value the file **declares**, never the
//! culture this module resolved it to for reading — the two are different facts
//! and a writer needs the first.
//!
//! ## The `StringTable` search is bounded before it reads, and filtered after
//!
//! A `StringTable` is not indexed anywhere, so finding one means reading packages
//! and looking for the payload's shape inside them. Two things keep that honest.
//!
//! Both ceilings are applied from the container's **index**, before a block is
//! decompressed — see [`MizaniyatHizam`]. And a located payload is checked for
//! being text somebody authored before it is believed — see [`jadwal_muqni`],
//! which exists because the structural match alone put hundreds of `SoundCue` and
//! `Blueprint` packages into the read report as string tables.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use taarib_muhawwil_unreal::khata::KhataUnreal;
use taarib_muhawwil_unreal::mawarid::jadwal::JadwalNusus as JadwalUnreal;
use taarib_muhawwil_unreal::mawarid::iostore::HawiyatIoStore;
use taarib_muhawwil_unreal::mawarid::locmeta::MawridLocmeta;
use taarib_muhawwil_unreal::mawarid::locres::{MadkhalLocres, MarjaTarjama, MawridLocres};
use taarib_muhawwil_unreal::mawarid::pak::HawiyatPak;
use taarib_muhawwil_unreal::mawarid::{Mawrid as _, TarmizNass};

use crate::jadwal::{JadwalNusus, MawqiNass};
use crate::rafd::{SababRafd, TaqreerRafd};
use crate::tasnif::{TalabMudkhal, ansha_mudkhal};

/// How deep the walk for loose localization files goes.
///
/// Twenty-four. Unreal projects nest `Content/Localization/<Target>/<Culture>/`
/// under a plugin under a module, and a shipped build adds the platform
/// directory on top; twenty-four clears every layout seen and stops a symlink
/// loop the filesystem failed to break.
pub const UMQ_MASH: usize = 24;

/// How many filesystem entries the walk will look at.
pub const AQSA_MALAFAT: usize = 400_000;

/// The largest package this module will search for a `StringTable` payload.
///
/// Eight mebibytes. A `StringTable` asset is a key-and-string list; the largest
/// one in any shipped game is a few hundred kilobytes. The cap exists so that
/// searching a container's packages does not walk a two hundred megabyte level.
pub const AQSA_HIZMA: u64 = 8 * 1024 * 1024;

/// How many bytes of packages this module will search for `StringTable`
/// payloads, per container.
///
/// A `StringTable` is not indexed anywhere: finding one means reading a package
/// and looking for the payload's structure inside it. There is no cheaper test,
/// so the search is bounded by a budget instead, and a container whose budget
/// runs out says so in the report rather than silently stopping.
pub const AQSA_MASH_HIZAM: u64 = 256 * 1024 * 1024;

/// The per-container budget for searching packages, spent from the index.
///
/// ## Why the size is taken from the index and not from the bytes
///
/// A container's index states every entry's expanded size, so both ceilings —
/// [`AQSA_HIZMA`] per package and [`AQSA_MASH_HIZAM`] per container — can be
/// applied *before* a single block is decompressed. Applying them afterwards
/// means expanding a package in order to throw it away: on Little Nightmares
/// that was fifty mebibytes of the three hundred and six the search decompressed,
/// spent on packages the very next line discarded for being too large.
///
/// ## What "fits" means, exactly
///
/// A package that fits in what remains is charged and searched; one that does
/// not fit **stops** the search. The comparison is strict, so a container whose
/// candidates come to exactly the ceiling is searched to the end and refuses
/// nothing — the ceiling is a limit the container is allowed to reach, and a
/// report saying 268435456 is "above" 268435456 was telling a user their intact
/// game had tripped a limit it had not.
///
/// A package over [`AQSA_HIZMA`] is charged and skipped rather than read. It is
/// charged because the budget bounds the bytes this search *accounts for* in a
/// container, not only the bytes it reads, and leaving the skipped ones free
/// would make the same container cost a different amount depending on how its
/// packages happen to be sized.
#[derive(Debug)]
struct MizaniyatHizam {
    /// How many bytes are still available.
    mutabaqqi: u64,
    /// How many bytes of candidate packages the container holds in total.
    jumla: u64,
}

/// What the budget decided about one candidate package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QararHizma {
    /// Read it and search it for a payload.
    Iqra,
    /// Too large to be a `StringTable`; charged, never expanded.
    Tajawuz,
    /// The budget cannot cover it; the search stops here.
    Tawaqquf,
}

impl MizaniyatHizam {
    /// A budget for a container whose candidate packages come to `jumla` bytes.
    const fn jadeeda(jumla: u64) -> Self {
        Self { mutabaqqi: AQSA_MASH_HIZAM, jumla }
    }

    /// Decides one candidate from its expanded size alone, and charges for it.
    const fn qarrir(&mut self, tul: u64) -> QararHizma {
        if tul > self.mutabaqqi {
            return QararHizma::Tawaqquf;
        }
        self.mutabaqqi = self.mutabaqqi.saturating_sub(tul);
        if tul > AQSA_HIZMA { QararHizma::Tajawuz } else { QararHizma::Iqra }
    }

    /// The refusal a stopped search records.
    ///
    /// `qeema` is what the container actually holds and `saqf` is this build's
    /// ceiling, so the sentence the user reads states a real inequality between
    /// two different numbers.
    fn rafd(&self) -> SababRafd {
        SababRafd::TajawuzHadd {
            hadd: "the per-container budget for searching packages for string tables".to_owned(),
            qeema: self.jumla,
            saqf: AQSA_MASH_HIZAM,
        }
    }
}

/// What a `.locmeta` declares about one localization target.
///
/// Carried out of extraction as its own value rather than folded into the string
/// table, because it is not a string: it is the fact a writer has to preserve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BayanThaqafat {
    /// The container the `.locmeta` was found in, relative to the game's root.
    pub hawiya: String,
    /// The `.locmeta`'s path inside that container, when it was inside one.
    pub asl: Option<String>,
    /// The culture the game's own text is written in.
    ///
    /// **Never overwritten by a patch.** Unreal falls back to this culture for
    /// every key a selected culture does not define, so a patch that rewrote it
    /// to `ar` would make every untranslated string fall back to Arabic that
    /// does not exist, and the game would draw empty labels.
    pub thaqafa_asliya: String,
    /// The localization target's own path, as the `.locmeta` records it.
    pub masar_asli: String,
    /// Every culture the target was compiled for.
    pub thaqafat: Vec<String>,
}

/// Pulls every string out of an Unreal game.
///
/// Returns the table and the report together, always. A game whose every
/// container was encrypted still returns both, and the report is the useful
/// half — see [`crate::rafd`].
#[must_use]
pub fn istakhrij(jidhr: &Path) -> (JadwalNusus, TaqreerRafd) {
    let mut jadwal = JadwalNusus::jadeed();
    let mut taqreer = TaqreerRafd::jadeed();
    let masarat = masarat_lil_mash(jidhr);

    // The loose `.locmeta` files first, because a loose `.locres` beside them
    // needs to know which culture is the source before it is read.
    let mut muallana: BTreeMap<String, String> = BTreeMap::new();
    for masar in &masarat.locmeta {
        let hawiya = nisbi(jidhr, masar);
        match qira_malaf(masar) {
            Ok(bayt) => {
                sajjil_locmeta(&mut taqreer, &mut muallana, &hawiya, None, &hawiya, &bayt);
            }
            Err(khata) => {
                taqreer.sajjil(hawiya, None, sabab_min_khata(&khata, NawMadkhal::Bayanat));
            }
        }
    }

    // The loose `.locres` paths, in the same spelling `sajjil_locres` will derive
    // the target and culture from, so the two agree on what is on disk.
    let asma_res: Vec<String> = masarat.locres.iter().map(|masar| nisbi(jidhr, masar)).collect();
    let asliya = wafiq_thaqafat(
        &muallana,
        &thaqafat_mawjuda(&asma_res),
        "the game's loose localization files",
        &mut taqreer,
    );

    for (masar, hawiya) in masarat.locres.iter().zip(&asma_res) {
        match qira_malaf(masar) {
            Ok(bayt) => {
                sajjil_locres(&mut jadwal, &mut taqreer, hawiya, None, hawiya, &bayt, &asliya);
            }
            Err(khata) => {
                taqreer.sajjil(
                    hawiya.clone(),
                    None,
                    sabab_min_khata(&khata, NawMadkhal::Nass),
                );
            }
        }
    }

    // The loose packages share one budget, because they are one body of files
    // rather than one container each: a game whose assets are cooked loose would
    // otherwise get the per-container budget once per file.
    //
    // The `.uexp` siblings are counted too, because they are charged too. A
    // total that left them out could come to less than the ceiling on a walk
    // that spent more than it, and the refusal would then claim a smaller number
    // is above a larger one — which is the sentence this whole budget was
    // reworked to stop producing.
    let jumla = masarat
        .hizam
        .iter()
        .flat_map(|masar| [masar.clone(), masar.with_extension("uexp")])
        .filter_map(|masar| std::fs::metadata(masar).ok())
        .map(|bayanat| bayanat.len())
        .fold(0_u64, u64::saturating_add);
    let mut mizaniya = MizaniyatHizam::jadeeda(jumla);
    for masar in &masarat.hizam {
        let hawiya = nisbi(jidhr, masar);
        if !sajjil_hizma_ala_qurs(&mut jadwal, &mut taqreer, &hawiya, masar, &mut mizaniya) {
            taqreer.sajjil("the game's loose packages".to_owned(), None, mizaniya.rafd());
            break;
        }
    }

    for masar in &masarat.pak {
        min_hawiyat_pak(&mut jadwal, &mut taqreer, jidhr, masar, &asliya);
    }
    for masar in &masarat.utoc {
        min_hawiyat_iostore(&mut jadwal, &mut taqreer, jidhr, masar, &asliya);
    }

    (jadwal, taqreer)
}

/// Reads every `.locmeta` a game has, loose and inside its containers.
///
/// Separate from [`istakhrij`] because Phase 14 needs the native culture at
/// write time and has no reason to pay for a whole extraction to learn it. It
/// re-opens the containers, which costs an index parse each and nothing more:
/// neither reader loads a member until it is asked for one.
#[must_use]
pub fn iqra_thaqafat(jidhr: &Path) -> Vec<BayanThaqafat> {
    let masarat = masarat_lil_mash(jidhr);
    let mut bayanat = Vec::new();

    for masar in &masarat.locmeta {
        let hawiya = nisbi(jidhr, masar);
        if let Ok(bayt) = qira_malaf(masar)
            && let Ok(mawrid) = MawridLocmeta::min_bayt(&bayt)
        {
            bayanat.push(bayan_thaqafat(&hawiya, None, &mawrid));
        }
    }

    for masar in &masarat.pak {
        let hawiya = nisbi(jidhr, masar);
        let Ok(qari) = HawiyatPak::iftah(masar, None) else { continue };
        let dakhili: Vec<String> = qari.masarat_locmeta().map(str::to_owned).collect();
        for asl in dakhili {
            if let Ok(bayt) = qari.iqra_masar(&asl)
                && let Ok(mawrid) = MawridLocmeta::min_bayt(&bayt)
            {
                bayanat.push(bayan_thaqafat(&hawiya, Some(asl), &mawrid));
            }
        }
    }

    for masar in &masarat.utoc {
        let hawiya = nisbi(jidhr, masar);
        let Ok(qari) = HawiyatIoStore::iftah(masar, None) else { continue };
        let dakhili: Vec<String> =
            qari.masarat_locmeta().map(|(masar_dakhili, _)| masar_dakhili.to_owned()).collect();
        for asl in dakhili {
            if let Ok(bayt) = qari.iqra_masar(&asl)
                && let Ok(mawrid) = MawridLocmeta::min_bayt(&bayt)
            {
                bayanat.push(bayan_thaqafat(&hawiya, Some(asl), &mawrid));
            }
        }
    }

    bayanat
}

/// The files a walk of the game's root turned up, sorted by what they are.
#[derive(Debug, Default)]
struct MasaratUnreal {
    /// Loose `.locres` files.
    locres: Vec<PathBuf>,
    /// Loose `.locmeta` files.
    locmeta: Vec<PathBuf>,
    /// Loose `.uasset` packages, which may or may not hold a `StringTable`.
    hizam: Vec<PathBuf>,
    /// `.pak` containers.
    pak: Vec<PathBuf>,
    /// IoStore tables of contents.
    utoc: Vec<PathBuf>,
}

/// Walks the game's root once and sorts what it finds.
fn masarat_lil_mash(jidhr: &Path) -> MasaratUnreal {
    let mut masarat = MasaratUnreal::default();
    let sayr = walkdir::WalkDir::new(jidhr)
        .max_depth(UMQ_MASH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .take(AQSA_MALAFAT);

    for madkhal in sayr {
        if !madkhal.file_type().is_file() {
            continue;
        }
        let masar = madkhal.into_path();
        let Some(lahiqa) = masar.extension().and_then(std::ffi::OsStr::to_str) else {
            continue;
        };
        match lahiqa.to_ascii_lowercase().as_str() {
            "locres" => masarat.locres.push(masar),
            "locmeta" => masarat.locmeta.push(masar),
            "uasset" => masarat.hizam.push(masar),
            "pak" => masarat.pak.push(masar),
            "utoc" => masarat.utoc.push(masar),
            _ => {}
        }
    }

    // Sorted so that two extractions of the same build read the containers in
    // the same order. Identity does not depend on order, but the read report
    // does, and a report that reshuffled itself between runs would be unusable
    // as a diff.
    masarat.locres.sort();
    masarat.locmeta.sort();
    masarat.hizam.sort();
    masarat.pak.sort();
    masarat.utoc.sort();
    masarat
}

/// Reads every localization resource out of one `.pak`.
fn min_hawiyat_pak(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    jidhr: &Path,
    masar: &Path,
    asliya_amma: &BTreeMap<String, String>,
) {
    let hawiya = nisbi(jidhr, masar);
    let qari = match HawiyatPak::iftah(masar, None) {
        Ok(qari) => qari,
        Err(khata) => {
            taqreer.sajjil(hawiya, None, sabab_min_khata(&khata, NawMadkhal::Nass));
            return;
        }
    };

    let masarat_meta: Vec<String> = qari.masarat_locmeta().map(str::to_owned).collect();
    let masarat_res: Vec<String> = qari.masarat_locres().map(str::to_owned).collect();
    // The expanded size comes from the index, beside the path, so the budget can
    // be spent without expanding anything. `hajm_khaam` is what the entry says it
    // becomes; the reader checks the claim against the bytes it produces, so a
    // lying index costs a refusal and never a wrong allocation.
    let masarat_hizam: Vec<(String, u64)> = qari
        .fahras()
        .madakhil()
        .filter(|(masar_dakhili, _)| hizma_murashaha(masar_dakhili))
        .map(|(masar_dakhili, madkhal)| (masar_dakhili.to_owned(), madkhal.hajm_khaam))
        .collect();

    if masarat_meta.is_empty() && masarat_res.is_empty() && masarat_hizam.is_empty() {
        taqreer.sajjil(hawiya, None, SababRafd::BilaNusus);
        return;
    }

    let mut muallana = asliya_amma.clone();
    for asl in &masarat_meta {
        match qari.iqra_masar(asl) {
            Ok(bayt) => {
                sajjil_locmeta(taqreer, &mut muallana, &hawiya, Some(asl.clone()), asl, &bayt);
            }
            Err(khata) => {
                taqreer.sajjil(
                    hawiya.clone(),
                    Some(asl.clone()),
                    sabab_min_khata(&khata, NawMadkhal::Bayanat),
                );
            }
        }
    }
    let asliya = wafiq_thaqafat(&muallana, &thaqafat_mawjuda(&masarat_res), &hawiya, taqreer);

    for asl in &masarat_res {
        match qari.iqra_masar(asl) {
            Ok(bayt) => {
                sajjil_locres(
                    jadwal,
                    taqreer,
                    &hawiya,
                    Some(asl.clone()),
                    asl,
                    &bayt,
                    &asliya,
                );
            }
            Err(khata) => {
                taqreer.sajjil(
                    hawiya.clone(),
                    Some(asl.clone()),
                    sabab_min_khata(&khata, NawMadkhal::Nass),
                );
            }
        }
    }

    let jumla = masarat_hizam.iter().map(|(_, tul)| *tul).fold(0_u64, u64::saturating_add);
    let mut mizaniya = MizaniyatHizam::jadeeda(jumla);
    for (asl, tul) in &masarat_hizam {
        match mizaniya.qarrir(*tul) {
            QararHizma::Iqra => {}
            QararHizma::Tajawuz => continue,
            QararHizma::Tawaqquf => {
                taqreer.sajjil(hawiya.clone(), None, mizaniya.rafd());
                break;
            }
        }
        let Ok(bayt) = qari.iqra_masar(asl) else { continue };
        sajjil_jadwal(jadwal, taqreer, &hawiya, Some(asl), &bayt);
    }
}

/// Reads every localization resource out of one IoStore container.
fn min_hawiyat_iostore(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    jidhr: &Path,
    masar: &Path,
    asliya_amma: &BTreeMap<String, String>,
) {
    let hawiya = nisbi(jidhr, masar);
    let qari = match HawiyatIoStore::iftah(masar, None) {
        Ok(qari) => qari,
        Err(khata) => {
            taqreer.sajjil(hawiya, None, sabab_min_khata(&khata, NawMadkhal::Nass));
            return;
        }
    };

    let masarat_meta: Vec<String> =
        qari.masarat_locmeta().map(|(dakhili, _)| dakhili.to_owned()).collect();
    let masarat_res: Vec<String> =
        qari.masarat_locres().map(|(dakhili, _)| dakhili.to_owned()).collect();
    // The chunk's length in the container's uncompressed address space, which is
    // the size the reader will produce. Taken from the offset table beside the
    // path, for the reason [`MizaniyatHizam`] gives.
    let mawaqi = qari.jadwal().mawaqi();
    // A path whose chunk index the offset table does not name is dropped rather
    // than charged: the reader would refuse it anyway, and charging a size that
    // was never read would spend a budget on nothing.
    let masarat_hizam: Vec<(String, u64)> = qari
        .masarat()
        .filter(|(dakhili, _)| hizma_murashaha(dakhili))
        .filter_map(|(dakhili, fahras)| {
            let mawqi = usize::try_from(fahras).ok().and_then(|fahras| mawaqi.get(fahras))?;
            Some((dakhili.to_owned(), mawqi.tul))
        })
        .collect();

    if masarat_meta.is_empty() && masarat_res.is_empty() && masarat_hizam.is_empty() {
        // A container with no directory index names nothing, and the reader says
        // so by returning no paths. That is not "holds no text" — it is a
        // container this build cannot search by name — and the two get different
        // reasons because they have different remedies.
        let sabab = if qari.jadwal().dalil().is_none() {
            SababRafd::SighaMajhula {
                wujid: "an IoStore container with no directory index, which cannot be \
                        searched by path"
                    .to_owned(),
            }
        } else {
            SababRafd::BilaNusus
        };
        taqreer.sajjil(hawiya, None, sabab);
        return;
    }

    let mut muallana = asliya_amma.clone();
    for asl in &masarat_meta {
        match qari.iqra_masar(asl) {
            Ok(bayt) => {
                sajjil_locmeta(taqreer, &mut muallana, &hawiya, Some(asl.clone()), asl, &bayt);
            }
            Err(khata) => {
                taqreer.sajjil(
                    hawiya.clone(),
                    Some(asl.clone()),
                    sabab_min_khata(&khata, NawMadkhal::Bayanat),
                );
            }
        }
    }
    let asliya = wafiq_thaqafat(&muallana, &thaqafat_mawjuda(&masarat_res), &hawiya, taqreer);

    for asl in &masarat_res {
        match qari.iqra_masar(asl) {
            Ok(bayt) => {
                sajjil_locres(
                    jadwal,
                    taqreer,
                    &hawiya,
                    Some(asl.clone()),
                    asl,
                    &bayt,
                    &asliya,
                );
            }
            Err(khata) => {
                taqreer.sajjil(
                    hawiya.clone(),
                    Some(asl.clone()),
                    sabab_min_khata(&khata, NawMadkhal::Nass),
                );
            }
        }
    }

    // A cooked IoStore chunk carries the package header and its exports
    // together, so the whole chunk goes to the raw locator rather than being
    // split into a package and a sibling export block the way a loose asset is.
    let jumla = masarat_hizam.iter().map(|(_, tul)| *tul).fold(0_u64, u64::saturating_add);
    let mut mizaniya = MizaniyatHizam::jadeeda(jumla);
    for (asl, tul) in &masarat_hizam {
        match mizaniya.qarrir(*tul) {
            QararHizma::Iqra => {}
            QararHizma::Tajawuz => continue,
            QararHizma::Tawaqquf => {
                taqreer.sajjil(hawiya.clone(), None, mizaniya.rafd());
                break;
            }
        }
        let Ok(bayt) = qari.iqra_masar(asl) else { continue };
        sajjil_jadwal(jadwal, taqreer, &hawiya, Some(asl), &bayt);
    }
}

/// Reads one `.locmeta` and records the native culture it declares.
fn sajjil_locmeta(
    taqreer: &mut TaqreerRafd,
    asliya: &mut BTreeMap<String, String>,
    hawiya: &str,
    asl: Option<String>,
    masar_dakhili: &str,
    bayt: &[u8],
) {
    let mawrid = match MawridLocmeta::min_bayt(bayt) {
        Ok(mawrid) => mawrid,
        Err(khata) => {
            taqreer.sajjil(
                hawiya.to_owned(),
                asl,
                sabab_min_khata(&khata, NawMadkhal::Bayanat),
            );
            return;
        }
    };

    let thaqafa = mawrid.thaqafa_asliya().to_owned();
    if let Some(dalil) = dalil_hadaf(masar_dakhili)
        && !thaqafa.is_empty()
    {
        let _ = asliya.insert(dalil, thaqafa.to_ascii_lowercase());
    }

    // Recorded as a container that was read and contributed no strings, which is
    // exactly what it is: a `.locmeta` holds culture names and a target path, and
    // none of those is text a player reads.
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        0,
        format!(
            "Unreal .locmeta, native culture \"{}\", compiled for [{}] — the native culture \
             is preserved on write and never replaced",
            thaqafa,
            mawrid.asma_thaqafat().collect::<Vec<_>>().join(", ")
        ),
    );
}

/// Which cultures each localization target actually has a `.locres` for.
///
/// Built from the paths, not from the `.locmeta`: a `.locmeta` lists the cultures
/// the target was *compiled for*, and what matters here is which of them the
/// build being read actually ships.
fn thaqafat_mawjuda(masarat: &[String]) -> BTreeMap<String, BTreeSet<String>> {
    let mut mawjuda: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for masar in masarat {
        let (Some(dalil), Some(thaqafa)) = hadaf_wa_thaqafa(masar) else { continue };
        let _ = mawjuda.entry(dalil).or_default().insert(thaqafa);
    }
    mawjuda
}

/// Resolves each target's declared native culture against the cultures on disk.
///
/// ## The problem this exists for
///
/// A `.locmeta`'s native culture is the culture the game's own text is written
/// in, and it does not have to be a culture the build ships a `.locres` for.
/// Unreal's own gather pipeline uses `en-US-POSIX` as the source pseudo-culture,
/// and Little Nightmares Enhanced Edition ships exactly that: `Game.locmeta`
/// declares `en-US-POSIX`, and the fifteen compiled cultures beside it are `ar`,
/// `de`, `en` and so on — none of them named `en-US-POSIX`.
///
/// Matching the declared name literally then makes **every** culture a
/// translation of a source that is not there, so every one of them is skipped and
/// the target yields nothing. That is not a degraded read of the game's text, it
/// is the loss of all of it, caused by the file whose whole job was to say where
/// the text is.
///
/// ## What it does instead
///
/// Three outcomes per target, all of them recorded so a user can see which one
/// applied:
///
/// 1. The declared culture is on disk — it is the source, unchanged.
/// 2. It is not, but exactly one culture on disk shares its language subtag —
///    that one is the source. `en-US-POSIX` resolves to `en`, which is what the
///    engine's own fallback chain does with it.
/// 3. Neither — the target is dropped from the map, which puts it back on the
///    no-`.locmeta` path where every culture is read as a source with the
///    culture folded into the engine key. Nothing is lost and no two cultures
///    collide; the report says the source language could not be identified.
fn wafiq_thaqafat(
    muallana: &BTreeMap<String, String>,
    mawjuda: &BTreeMap<String, BTreeSet<String>>,
    hawiya: &str,
    taqreer: &mut TaqreerRafd,
) -> BTreeMap<String, String> {
    let mut mahlula = BTreeMap::new();
    for (dalil, thaqafa) in muallana {
        let Some(mawjud) = mawjuda.get(dalil) else {
            // No `.locres` for this target in this scope, so there is nothing to
            // resolve against and nothing that could be skipped by keeping it.
            let _ = mahlula.insert(dalil.clone(), thaqafa.clone());
            continue;
        };
        if mawjud.contains(thaqafa) {
            let _ = mahlula.insert(dalil.clone(), thaqafa.clone());
            continue;
        }
        match badil_thaqafa(thaqafa, mawjud) {
            Some(badil) => {
                taqreer.sajjil_qira(
                    hawiya.to_owned(),
                    0,
                    format!(
                        "\"{dalil}\" declares native culture \"{thaqafa}\" and ships no .locres \
                         under that name; \"{badil}\" is the only compiled culture in the same \
                         language and is read as the source"
                    ),
                );
                let _ = mahlula.insert(dalil.clone(), badil);
            }
            None => taqreer.sajjil_qira(
                hawiya.to_owned(),
                0,
                format!(
                    "\"{dalil}\" declares native culture \"{thaqafa}\" and ships no .locres \
                     under that name or in that language; every compiled culture is read as a \
                     source instead, with the culture folded into the engine key so none of \
                     them collides with another"
                ),
            ),
        }
    }
    mahlula
}

/// The compiled culture that stands in for a declared native culture with no
/// file of its own.
///
/// The language subtag has to match and the answer has to be unambiguous. An
/// exact match on the bare language wins outright — `en-US-POSIX` against a
/// target holding both `en` and `en-GB` resolves to `en` — and otherwise a
/// single candidate in that language is taken. Two candidates and no bare
/// language is a choice this module will not make silently: guessing `en-GB`
/// over `en-AU` would put one region's spelling in the table as the source text
/// of the whole game.
fn badil_thaqafa(muallana: &str, mawjuda: &BTreeSet<String>) -> Option<String> {
    let lugha = muallana.split(['-', '_']).next().unwrap_or(muallana);
    if lugha.is_empty() {
        return None;
    }
    if mawjuda.contains(lugha) {
        return Some(lugha.to_owned());
    }
    let mut murashahun = mawjuda
        .iter()
        .filter(|mawjud| mawjud.split(['-', '_']).next() == Some(lugha));
    let awwal = murashahun.next()?;
    if murashahun.next().is_some() {
        return None;
    }
    Some(awwal.clone())
}

/// Turns a read `.locmeta` into the value Phase 14 consumes.
fn bayan_thaqafat(hawiya: &str, asl: Option<String>, mawrid: &MawridLocmeta) -> BayanThaqafat {
    BayanThaqafat {
        hawiya: hawiya.to_owned(),
        asl,
        thaqafa_asliya: mawrid.thaqafa_asliya().to_owned(),
        masar_asli: mawrid.masar_asli().to_owned(),
        thaqafat: mawrid.asma_thaqafat().map(str::to_owned).collect(),
    }
}

/// Reads one `.locres` and folds its entries into the table.
///
/// `masar_dakhili` is the path the culture and the localization target are read
/// out of: the loose path for a file on disk, the container-relative path for a
/// member. Both are `Localization/<Target>/<Culture>/<Target>.locres` in every
/// layout the engine cooks, so one derivation serves both.
fn sajjil_locres(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<String>,
    masar_dakhili: &str,
    bayt: &[u8],
    asliya: &BTreeMap<String, String>,
) {
    let mawrid = match MawridLocres::min_bayt(bayt) {
        Ok(mawrid) => mawrid,
        Err(khata) => {
            taqreer.sajjil(hawiya.to_owned(), asl, sabab_min_khata(&khata, NawMadkhal::Nass));
            return;
        }
    };

    let (dalil, thaqafa) = hadaf_wa_thaqafa(masar_dakhili);
    let asliyat_hadaf = dalil.as_deref().and_then(|dalil| asliya.get(dalil));

    // With a `.locmeta` in hand, only the native culture is a source. Without
    // one, every culture is a source and the culture joins the engine key so
    // that the English and the French of one line do not become one identity.
    let (yustakhraj, bi_thaqafa) = match (asliyat_hadaf, thaqafa.as_deref()) {
        (Some(matlub), Some(mawjud)) => (matlub == mawjud, false),
        (Some(_), None) => (false, false),
        (None, _) => (true, true),
    };

    if !yustakhraj {
        let ism = thaqafa.as_deref().unwrap_or("an unnamed culture");
        let matlub = asliyat_hadaf.map_or("unknown", String::as_str);
        taqreer.sajjil_qira(
            hawiya.to_owned(),
            0,
            format!(
                "Unreal .locres for culture \"{ism}\", which its .locmeta says is a \
                 translation of the native culture \"{matlub}\" and not a source"
            ),
        );
        return;
    }

    let mut adad = 0_usize;
    for (fadaa, madkhal) in mawrid.madakhil() {
        let Some(khaam) = mawrid.nass_madkhal(madkhal) else {
            continue;
        };
        if khaam.trim().is_empty() {
            // An entry whose compiled string is empty has nothing to translate
            // and nothing to write back. Kept out of the table rather than
            // classified, because a row a translator cannot act on is a row that
            // only makes the real ones harder to find.
            continue;
        }

        let ism_fadaa = fadaa.ism().nass().to_owned();
        let ism_miftah = madkhal.miftah().nass().to_owned();
        let mut miftah = String::with_capacity(ism_fadaa.len() + ism_miftah.len() + 24);
        if bi_thaqafa && let Some(ism) = thaqafa.as_deref() {
            miftah.push_str(ism);
            miftah.push('\u{1}');
        }
        miftah.push_str(&ism_fadaa);
        miftah.push('\u{1}');
        miftah.push_str(&ism_miftah);

        let mawqi = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: asl.clone(),
            mawqi: format!("{ism_fadaa}/{ism_miftah}"),
            haql: None,
            miftah_muharrik: Some(miftah),
        };

        let talab = TalabMudkhal::jadeed(mawqi, khaam)
            .bi_nizam_tawtin()
            .bi_tarmiz(tarmiz_madkhal(&mawrid, madkhal));
        jadwal.adif(ansha_mudkhal(talab));
        adad = adad.saturating_add(1);
    }

    if adad == 0 {
        taqreer.sajjil(hawiya.to_owned(), asl, SababRafd::BilaNusus);
        return;
    }

    let ism = thaqafa.as_deref().unwrap_or("an unnamed culture");
    let isdar = mawrid
        .isdar()
        .raqm()
        .map_or_else(|| "legacy".to_owned(), |raqm| raqm.to_string());
    taqreer.sajjil_qira(
        hawiya.to_owned(),
        adad,
        format!("Unreal .locres version {isdar} for culture \"{ism}\", read as the source text"),
    );
}

/// Whether a container-relative path is a package worth searching.
fn hizma_murashaha(masar: &str) -> bool {
    Path::new(masar).extension().is_some_and(|lahiqa| {
        lahiqa.eq_ignore_ascii_case("uasset") || lahiqa.eq_ignore_ascii_case("uexp")
    })
}

/// Searches one loose package, and its sibling export block, for a
/// `StringTable`.
///
/// A cooked package keeps its exports in a `.uexp` beside the `.uasset`, so a
/// failure to find the payload in the package is followed by a look at the
/// sibling rather than treated as an answer. Neither failure is recorded: a
/// package that holds no string table is the ordinary case for essentially every
/// `.uasset` in a game, and reporting one refusal per package would produce a
/// report with a hundred thousand lines in it and no information.
///
/// Returns whether the walk may continue. `false` means the budget could not
/// cover this package, which is the caller's cue to record one refusal for the
/// whole loose set rather than one per file.
fn sajjil_hizma_ala_qurs(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    masar: &Path,
    mizaniya: &mut MizaniyatHizam,
) -> bool {
    // The size decides before the read, exactly as it does inside a container:
    // a package over the ceiling is charged and never opened.
    let Ok(bayanat) = std::fs::metadata(masar) else { return true };
    match mizaniya.qarrir(bayanat.len()) {
        QararHizma::Iqra => {}
        QararHizma::Tajawuz => return true,
        QararHizma::Tawaqquf => return false,
    }
    let Ok(bayt) = qira_malaf(masar) else { return true };
    if sajjil_jadwal(jadwal, taqreer, hawiya, None, &bayt) {
        return true;
    }

    let sahib = masar.with_extension("uexp");
    let Ok(bayanat) = std::fs::metadata(&sahib) else { return true };
    match mizaniya.qarrir(bayanat.len()) {
        QararHizma::Iqra => {}
        QararHizma::Tajawuz => return true,
        QararHizma::Tawaqquf => return false,
    }
    let Ok(bayt) = qira_malaf(&sahib) else { return true };
    let _ = sajjil_jadwal(jadwal, taqreer, hawiya, None, &bayt);
    true
}

/// Folds a `StringTable` payload into the table, if these bytes hold one.
///
/// Returns whether one was found. Tries the package entry point first, which
/// checks the package tag, and falls back to the raw locator for bytes that are
/// an export block rather than a whole package.
fn sajjil_jadwal(
    jadwal: &mut JadwalNusus,
    taqreer: &mut TaqreerRafd,
    hawiya: &str,
    asl: Option<&String>,
    bayt: &[u8],
) -> bool {
    let mawqi = match JadwalUnreal::min_uasset(bayt) {
        Ok(mawqi) => mawqi,
        Err(_) => match JadwalUnreal::jid_fi_kutla(bayt) {
            Ok(mawqi) => mawqi,
            Err(_) => return false,
        },
    };

    let jadwal_unreal = mawqi.jadwal();
    if !jadwal_muqni(jadwal_unreal) {
        return false;
    }
    let muarrif = jadwal_unreal.muarrif().to_owned();
    let mut adad = 0_usize;

    for saf in jadwal_unreal.sufuf() {
        let khaam = saf.asl();
        if !nass_muqni(khaam) || !muarrif_muqni(saf.miftah()) {
            continue;
        }
        let miftah_saf = saf.miftah().to_owned();
        let mawqi_nass = MawqiNass {
            hawiya: hawiya.to_owned(),
            asl: asl.cloned(),
            mawqi: format!("{muarrif}/{miftah_saf}"),
            haql: None,
            // A `StringTable` is looked up by table id and row key through
            // `FStringTableRegistry`, the developer chose both, and they are as
            // stable as a namespace and key. Same rule, same field.
            miftah_muharrik: Some(format!("{muarrif}\u{1}{miftah_saf}")),
        };
        let talab = TalabMudkhal::jadeed(mawqi_nass, khaam)
            .bi_nizam_tawtin()
            .bi_tarmiz(tarmiz_min_naw(saf.nass_asl().tarmiz()));
        jadwal.adif(ansha_mudkhal(talab));
        adad = adad.saturating_add(1);
    }

    if adad > 0 {
        taqreer.sajjil_qira(
            hawiya.to_owned(),
            adad,
            format!("Unreal StringTable \"{muarrif}\""),
        );
    }
    adad > 0
}

/// Whether a located payload is a `StringTable` somebody authored.
///
/// ## Why a structural match is not enough
///
/// The locator finds a position where the payload's shape parses: a
/// length-prefixed id, a row count, that many key/string pairs, a metadata count,
/// and an end inside the buffer. Its own documentation argues that a structure
/// that long is not something arbitrary bytes fall into by accident.
///
/// **On real games it is.** A read-only walk of two shipped Unreal titles put
/// fifty-eight and three hundred and thirty-four "string tables" into the report,
/// out of `SoundCue`, `Blueprint`, `StaticMesh`, `PhysicsAsset` and animation
/// packages — none of which has ever held one. Cooked package data is full of
/// small length-prefixed runs and long stretches of zeroes, and a run of zeroes
/// is a valid empty `FString` followed by a valid zero count; the shape is
/// common, not rare.
///
/// ## The test
///
/// So the shape is necessary and not sufficient, and what separates a real table
/// from a coincidence is that a real one is *text somebody wrote*:
///
/// - the table id is an asset name or a package path, at least
///   [`AQALL_MUARRIF`] characters long, with no control characters and mostly
///   letters and digits — `ST_Dialogue`, `/Game/UI/ST_Menu`;
/// - at least one row has a key of the same shape and a source string that is
///   readable text rather than padding.
///
/// Every junk match on both games fails one of those, and both games' real
/// tables pass. Rows that fail individually are dropped rather than failing the
/// table, because a real table may legitimately hold a blank row.
fn jadwal_muqni(jadwal: &JadwalUnreal) -> bool {
    if !muarrif_muqni(jadwal.muarrif()) {
        return false;
    }
    jadwal
        .sufuf()
        .iter()
        .any(|saf| muarrif_muqni(saf.miftah()) && nass_muqni(saf.asl()))
}

/// The fewest characters a `StringTable` id or row key may have.
///
/// Three. The locator's prefilter accepts a declared length of two units, which
/// is one character and a terminator, and a one-character asset name is not
/// something Unreal's content browser will create — but a single byte followed
/// by a zero is something arbitrary data is full of.
const AQALL_MUARRIF: usize = 3;

/// The share of an identifier that has to be letters or digits.
///
/// Half. `ST_Dialogue` is eleven characters of which ten qualify;
/// `/Game/UI/ST_Menu` is sixteen of which twelve do; a Japanese-authored id is
/// entirely letters, which is why the test is Unicode-aware rather than ASCII.
/// The junk that reached the report was padding around a few stray bytes and
/// came nowhere near half.
const NISBAT_HURUF: usize = 2;

/// Whether a string looks like an identifier a developer chose.
fn muarrif_muqni(ism: &str) -> bool {
    let mut adad = 0_usize;
    let mut huruf = 0_usize;
    for harf in ism.chars() {
        // A control character rules the whole thing out on its own: a table id
        // and a row key are names, and a name with a newline or a NUL in it is
        // not a name that any authoring tool produced.
        if harf.is_control() {
            return false;
        }
        adad = adad.saturating_add(1);
        if harf.is_alphanumeric() {
            huruf = huruf.saturating_add(1);
        }
    }
    adad >= AQALL_MUARRIF && huruf.saturating_mul(NISBAT_HURUF) >= adad
}

/// Whether a row's source string is text rather than padding.
///
/// Looser than [`muarrif_muqni`], because a source string legitimately holds
/// punctuation, digits, spaces and markup — `"{0} of {1}"` is mostly not letters
/// — and legitimately holds a newline. What it may not be is empty, and it may
/// not carry the control characters that only ever come from reading a binary
/// field as if it were a string.
fn nass_muqni(nass: &str) -> bool {
    if nass.trim().is_empty() {
        return false;
    }
    nass.chars().all(|harf| !harf.is_control() || matches!(harf, '\t' | '\n' | '\r'))
}

/// The encoding a `.locres` entry's string was stored in.
///
/// Recorded rather than normalized away, because a writer has to produce the
/// same form: Unreal serializes a positive length as Latin-1 and a negative one
/// as UTF-16LE, and a round trip that changed which one a untouched entry uses
/// would move every byte after it in the file.
fn tarmiz_madkhal(mawrid: &MawridLocres, madkhal: &MadkhalLocres) -> Option<String> {
    let naw = match madkhal.tarjama() {
        MarjaTarjama::Mudmaj(nass) => nass.tarmiz(),
        MarjaTarjama::Fahras(fahras) => {
            let khana = usize::try_from(*fahras).ok()?;
            mawrid.hawd().get(khana)?.nass().tarmiz()
        }
    };
    tarmiz_min_naw(naw)
}

/// The encoding's name, in the spelling the rest of the product uses.
fn tarmiz_min_naw(naw: TarmizNass) -> Option<String> {
    match naw {
        // An empty string has no encoding to preserve: Unreal writes a zero
        // length and no bytes, whatever the string used to be.
        TarmizNass::Khali => None,
        TarmizNass::Ansi => Some("ISO-8859-1".to_owned()),
        TarmizNass::Utf16 => Some("UTF-16LE".to_owned()),
    }
}

/// The localization target directory and the culture directory of a `.locres`.
///
/// `Content/Localization/Game/en/Game.locres` yields the target
/// `content/localization/game` and the culture `en`. Lowercased on both sides,
/// because a Windows-authored project routinely disagrees with itself about the
/// casing of `Content` and a case-sensitive match would then find no metadata
/// for a target whose metadata is sitting right beside it.
fn hadaf_wa_thaqafa(masar: &str) -> (Option<String>, Option<String>) {
    let munkhafid = masar.replace('\\', "/").to_ascii_lowercase();
    let mut ajza: Vec<&str> = munkhafid.split('/').filter(|juz| !juz.is_empty()).collect();
    let _ = ajza.pop();
    let thaqafa = ajza.pop().map(str::to_owned);
    let dalil = if ajza.is_empty() { None } else { Some(ajza.join("/")) };
    (dalil, thaqafa)
}

/// The localization target directory a `.locmeta` sits directly inside.
fn dalil_hadaf(masar: &str) -> Option<String> {
    let munkhafid = masar.replace('\\', "/").to_ascii_lowercase();
    let mut ajza: Vec<&str> = munkhafid.split('/').filter(|juz| !juz.is_empty()).collect();
    let _ = ajza.pop();
    if ajza.is_empty() { None } else { Some(ajza.join("/")) }
}

/// A path relative to the game's root, with forward slashes on every platform.
///
/// The separator is normalized because the container identity ends up inside
/// [`taarib_mustalahat::nass::NassId`], and a project extracted on Windows and
/// re-extracted on Linux has to produce the same identities or every translation
/// in it orphans itself on the second machine.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    let juz = masar.strip_prefix(jidhr).unwrap_or(masar);
    juz.components()
        .map(|qism| qism.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Reads a whole file, naming the path in whatever went wrong.
fn qira_malaf(masar: &Path) -> Result<Vec<u8>, KhataUnreal> {
    std::fs::read(masar)
        .map_err(|sabab| KhataUnreal::KhataMalaf { masar: masar.to_path_buf(), sabab })
}

/// What kind of member a refusal is about.
///
/// The reason a refusal names has to be true of the file, and the remedy the
/// interface offers has to be one that could work for it — and those two things
/// come apart on exactly one axis: whether the member holds text a player reads.
/// A `.locres` this build cannot expand is a container of strings that are drawn
/// on screen, so capture recovers them; a `.locmeta` this build cannot read
/// holds no text at all, so capture recovers nothing and the only thing that
/// could help is a build that reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NawMadkhal {
    /// A member holding text a player reads: a `.locres`, a package, or a whole
    /// container of them.
    Nass,
    /// A member holding no player-visible text: a `.locmeta`.
    Bayanat,
}

/// Turns a reader's refusal into the reason the report shows.
///
/// The mapping is explicit rather than a catch-all, because the reason decides
/// which remedy the interface offers.
///
/// ## What is deliberately *not* called damage
///
/// [`SababRafd::Talif`] renders as "The container is damaged" and its remedy is
/// "Ask the launcher to verify the game's files". That is an accusation about
/// the user's installation, and it is only made where there is evidence for it:
/// a field outside its bounds, a structure running past the end of the file, or
/// a payload that does not match the hash the container itself recorded over it.
///
/// A block this build could not expand, or one that expanded to a size the entry
/// did not declare, is **not** evidence of that. It is equally consistent with a
/// block layout this build gets wrong — which is what Little Nightmares Enhanced
/// Edition turned out to be, twenty-one `.locres` members on a game that plays
/// perfectly. Those map to [`SababRafd::SighaMajhula`], which says this build
/// cannot read it and offers capture, a remedy that does work for text.
fn sabab_min_khata(khata: &KhataUnreal, naw: NawMadkhal) -> SababRafd {
    match khata {
        KhataUnreal::KhataMalaf { sabab, .. } => {
            SababRafd::TaadhurQira { sabab: sabab.to_string() }
        }
        KhataUnreal::PakMushaffar { .. } => SababRafd::Mushaffar {
            wasf: "AES-256, and no key was supplied — Taarib does not go looking for one"
                .to_owned(),
        },
        KhataUnreal::MiftahGhayrSalih { sabab, .. } => {
            SababRafd::Mushaffar { wasf: (*sabab).to_owned() }
        }
        KhataUnreal::IsdarGhayrMadum { ism, wujid, aqsa } => SababRafd::IsdarGhayrMadum {
            sigha: (*ism).to_owned(),
            wujid: wujid.to_string(),
            madum: format!("up to and including {aqsa}"),
        },
        KhataUnreal::HajmMufrit { haql, qeema, saqf } => {
            SababRafd::TajawuzHadd { hadd: (*haql).to_owned(), qeema: *qeema, saqf: *saqf }
        }
        KhataUnreal::SihrGhayrMutabaq { ism, .. } => ghayr_maqru(
            naw,
            ism,
            format!("a {ism} whose header is not the one this build recognises"),
        ),
        KhataUnreal::DaghtMajhul { ism, naw: tareeqa } => ghayr_maqru(
            naw,
            ism,
            format!("a {ism} block compressed with \"{tareeqa}\", which this build cannot expand"),
        ),
        KhataUnreal::FakkFashil { ism, tafsil } => ghayr_maqru(
            naw,
            ism,
            format!(
                "the block layout of a compressed {ism} entry, whose blocks this build's \
                 decompressor could not expand ({tafsil})"
            ),
        ),
        KhataUnreal::HajmGhayrMutabaq { ism, muallan, fili } => ghayr_maqru(
            naw,
            ism,
            format!(
                "the block layout of a compressed {ism} entry, which expanded to {fili} bytes \
                 where the container declares {muallan}"
            ),
        ),
        // The one refusal that is real evidence of a modified or damaged file:
        // the payload does not match the hash the container wrote over it.
        KhataUnreal::BasmaGhayrMutabaqa { madkhal, .. } => SababRafd::Talif {
            sabab: format!(
                "\"{madkhal}\" does not match the hash the container itself recorded over it"
            ),
        },
        akhar => SababRafd::Talif { sabab: akhar.to_string() },
    }
}

/// A member this build could not read, named so that the remedy could work.
///
/// For text, [`SababRafd::SighaMajhula`]: the strings exist and are drawn on
/// screen, so "play once with capture on" recovers them.
///
/// For metadata, [`SababRafd::IsdarGhayrMadum`]: capture cannot recover a native
/// culture, and the only thing that could is a newer build — which is exactly the
/// remedy that variant offers. The fields are chosen so the rendered sentence is
/// true of what happened rather than merely close to it, since the sentence is
/// what a user reads.
fn ghayr_maqru(naw: NawMadkhal, ism: &str, wujid: String) -> SababRafd {
    match naw {
        NawMadkhal::Nass => SababRafd::SighaMajhula { wujid },
        NawMadkhal::Bayanat => SababRafd::IsdarGhayrMadum {
            sigha: format!("the {ism}"),
            wujid: "this container carries".to_owned(),
            madum: format!(
                "the {ism} layout its own reader was written against — nothing a player sees \
                 is missing from the extraction, only the game's own statement of which \
                 language it was written in"
            ),
        },
    }
}
