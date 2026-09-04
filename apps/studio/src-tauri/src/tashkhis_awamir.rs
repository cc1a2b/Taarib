//! التشخيص — the diagnostics surface: the logs, the compatibility report, and the maintainer bundle.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use taarib_makhzan::sijillat::{SijillAlaab, SijillBina, SijillMuharrik};
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::bina::BinaId;
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_usus::idadat::MakhzanIdadat;
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::manassa::NizamTashghil;
use taarib_usus::masarat::{Masarat, insha_mujallad, kitaba_dharra};
use taarib_usus::{ISDAR, khata_min};

use crate::luba_awamir::{huwiya, ijlib_luba};
use crate::warsha_awamir::lahza_alaan;

/// The directory under the data root every diagnostic artifact is written into.
const MUJALLAD_TASHKHIS: &str = "tashkhis";

/// The file-name prefix a compatibility report is written and collected under.
const BADIYAT_TAWAFUQ: &str = "tawafuq-";

/// One log file as the diagnostics screen lists it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MalafSijillHie {
    /// Its name inside the log directory.
    pub ism: String,
    /// Its size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// When it was last written, RFC 3339.
    pub waqt: String,
}

/// The diagnostics view of the logs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SijillatHie {
    /// Every log file, newest first.
    pub malaffat: Vec<MalafSijillHie>,
    /// The last lines of the newest file.
    pub akhir: Vec<String>,
}

/// What building a maintainer bundle produced.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TashkhisHie {
    /// The bundle directory.
    pub masar: String,
    /// How many files it holds.
    pub adad_malaffat: u32,
    /// Their total size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
}

/// The compatibility report as it is written to disk.
#[derive(serde::Serialize)]
struct WatheeqatTawafuq {
    luba: LubaTawafuq,
    taqreer: TaqreerImkaniyat,
    bina: Option<BinaId>,
    isdar: &'static str,
}

/// The game's identity facts inside the compatibility report.
#[derive(serde::Serialize)]
struct LubaTawafuq {
    ism: String,
    jidhr: String,
}

/// What this machine and this build are, inside the maintainer bundle.
#[derive(serde::Serialize)]
struct BayanHuzma {
    isdar: &'static str,
    nizam: String,
    adad_alaab: Option<u64>,
    waqt: String,
}

fn khata_sijillat(sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataTashkhisAmr::SijillatGhayrMaqrua { sabab: sabab.to_string() })
}

fn khata_huzma(sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataTashkhisAmr::HuzmaFashila { sabab: sabab.to_string() })
}

/// A filesystem timestamp as RFC 3339, or the raw seconds when no calendar holds it.
fn waqt_rfc3339(waqt: SystemTime) -> String {
    jiff::Timestamp::try_from(waqt).map_or_else(
        |_| waqt.duration_since(UNIX_EPOCH).map_or(0, |mudda| mudda.as_secs()).to_string(),
        |lahza| lahza.to_string(),
    )
}

/// The last `satr` lines of one file, decoded lossily so a tail the writer tore
/// mid-record still reads.
fn akhir_sutur(masar: &Path, satr: u32) -> Vec<String> {
    // Rotation can delete the file between the listing and this read; no lines is
    // the honest answer, not a diagnostics screen dead on its own logs.
    let Ok(bayt) = fs::read(masar) else {
        return Vec::new();
    };
    let nass = String::from_utf8_lossy(&bayt);
    let sutur: Vec<&str> = nass.lines().collect();
    let bidaya = sutur.len().saturating_sub(usize::try_from(satr).unwrap_or(usize::MAX));
    sutur.into_iter().skip(bidaya).map(ToOwned::to_owned).collect()
}

/// Copies one file into the bundle directory under its own name.
fn unsakh(masar: &Path, mujallad: &Path) -> Natija<()> {
    let Some(ism) = masar.file_name() else {
        return Ok(());
    };
    let _ = fs::copy(masar, mujallad.join(ism)).map_err(khata_huzma)?;
    Ok(())
}

/// The log files newest first, and the last `satr` lines of the newest one.
///
/// # Errors
///
/// [`KhataTashkhisAmr::SijillatGhayrMaqrua`] when the log directory itself cannot be
/// listed. A single file that vanishes or refuses mid-read is skipped, because rotation
/// deletes files while this runs.
#[tauri::command]
#[specta::specta]
pub fn sijillat_akhira(
    satr: u32,
    masarat: tauri::State<'_, Masarat>,
) -> Result<SijillatHie, Khata> {
    let qaima = fs::read_dir(masarat.sijillat()).map_err(khata_sijillat)?;

    let mut murattaba = Vec::new();
    for dakhla in qaima.flatten() {
        let Ok(bayanat) = dakhla.metadata() else { continue };
        if !bayanat.is_file() {
            continue;
        }
        let waqt = bayanat.modified().unwrap_or(UNIX_EPOCH);
        murattaba.push((
            waqt,
            dakhla.path(),
            MalafSijillHie {
                ism: dakhla.file_name().to_string_lossy().into_owned(),
                hajm: bayanat.len(),
                waqt: waqt_rfc3339(waqt),
            },
        ));
    }
    murattaba.sort_by_key(|(waqt, _, _)| Reverse(*waqt));

    let akhir =
        murattaba.first().map_or_else(Vec::new, |(_, masar, _)| akhir_sutur(masar, satr));
    Ok(SijillatHie {
        malaffat: murattaba.into_iter().map(|(_, _, malaf)| malaf).collect(),
        akhir,
    })
}

/// Writes one game's stored facts — identity, capability report, current build, this
/// build's version — as one pretty-printed compatibility report, and answers its path.
///
/// # Errors
///
/// [`KhataTashkhisAmr::TaqreerMafqud`] when no capability report has been stored for the
/// game yet, [`KhataTashkhisAmr::HuzmaFashila`] when the document does not serialize, and
/// whatever the identity parse, the store, or the atomic write raise.
#[tauri::command]
#[specta::specta]
pub fn taqreer_tawafuq(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    qaida: tauri::State<'_, Makhzan>,
) -> Result<String, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&qaida, id)?;
    let (taqreer, bina) = qaida.bil_qira(|ittisal| {
        let taqreer = SijillMuharrik::jadeed(ittisal).wahid(id)?;
        let bina = SijillBina::jadeed(ittisal).haliya(id)?;
        Ok((taqreer, bina))
    })?;
    let Some(taqreer) = taqreer else {
        return Err(Khata::from(KhataTashkhisAmr::TaqreerMafqud { ism: luba.ism }));
    };

    let watheeqa = WatheeqatTawafuq {
        luba: LubaTawafuq {
            ism: luba.ism,
            jidhr: luba.jidhr.to_string_lossy().into_owned(),
        },
        taqreer,
        bina,
        isdar: ISDAR,
    };
    let bayt = serde_json::to_vec_pretty(&watheeqa).map_err(khata_huzma)?;

    let mujallad = masarat.jidhr_bayanat().join(MUJALLAD_TASHKHIS);
    insha_mujallad(&mujallad)?;
    let masar = mujallad.join(format!("{BADIYAT_TAWAFUQ}{id}.json"));
    kitaba_dharra(&masar, &bayt)?;
    Ok(masar.to_string_lossy().into_owned())
}

/// Builds a maintainer bundle: one plain directory holding every log, the settings tree,
/// a statement of this machine and build, and every compatibility report written so far.
///
/// A folder a maintainer can open and read beats an archive nobody can; nothing here is
/// compressed, and nothing from the keychain is anywhere near it.
///
/// # Errors
///
/// [`KhataTashkhisAmr::SijillatGhayrMaqrua`] when the log directory cannot be listed,
/// [`KhataTashkhisAmr::HuzmaFashila`] when a serialization or a report copy fails, and
/// whatever directory creation or the atomic writes raise. A single log file that goes
/// away mid-copy is skipped and logged, never raised.
#[tauri::command]
#[specta::specta]
pub fn huzmat_tashkhis(
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<TashkhisHie, Khata> {
    let jidhr = masarat.jidhr_bayanat().join(MUJALLAD_TASHKHIS);
    let mujallad = jidhr.join(format!("huzma-{}", lahza_alaan()));
    insha_mujallad(&mujallad)?;

    let sijillat = fs::read_dir(masarat.sijillat()).map_err(khata_sijillat)?;
    for dakhla in sijillat.flatten() {
        let masar = dakhla.path();
        // Rotation deletes log files while this loop runs. A bundle short one
        // rotated log still answers the question the user opened it for; a bundle
        // that refused to build answers nothing.
        if masar.is_file()
            && let Err(khata) = unsakh(&masar, &mujallad)
        {
            tracing::warn!(
                masar = %masar.display(),
                sabab = %khata.injilizi,
                "log file skipped while the diagnostics bundle was built"
            );
        }
    }

    // The current tree holds no secrets; credentials live in the platform keychain.
    let idadat = serde_json::to_vec_pretty(&*makhzan.hali()).map_err(khata_huzma)?;
    kitaba_dharra(&mujallad.join("idadat.json"), &idadat)?;

    // The library count comes from a store opened here on purpose: a bundle built
    // because the database is broken must not die on the database.
    let adad_alaab = Makhzan::iftah(&masarat)
        .and_then(|qaida| qaida.bil_qira(|ittisal| SijillAlaab::jadeed(ittisal).adad()))
        .ok();
    let bayan = BayanHuzma {
        isdar: ISDAR,
        nizam: format!("{:?}", NizamTashghil::hali()),
        adad_alaab,
        waqt: waqt_rfc3339(SystemTime::now()),
    };
    let bayan_bayt = serde_json::to_vec_pretty(&bayan).map_err(khata_huzma)?;
    kitaba_dharra(&mujallad.join("bayan.json"), &bayan_bayt)?;

    let taqareer = fs::read_dir(&jidhr).map_err(khata_huzma)?;
    for dakhla in taqareer.flatten() {
        let masar = dakhla.path();
        let ism = dakhla.file_name().to_string_lossy().into_owned();
        if masar.is_file()
            && ism.starts_with(BADIYAT_TAWAFUQ)
            && masar.extension().is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("json"))
        {
            unsakh(&masar, &mujallad)?;
        }
    }

    let mut adad_malaffat = 0_u32;
    let mut hajm = 0_u64;
    for dakhla in fs::read_dir(&mujallad).map_err(khata_huzma)?.flatten() {
        let Ok(bayanat) = dakhla.metadata() else { continue };
        if bayanat.is_file() {
            adad_malaffat = adad_malaffat.saturating_add(1);
            hajm = hajm.saturating_add(bayanat.len());
        }
    }

    Ok(TashkhisHie {
        masar: mujallad.to_string_lossy().into_owned(),
        adad_malaffat,
        hajm,
    })
}

/// Opens the diagnostics directory in the platform file manager.
///
/// # Errors
///
/// [`KhataTashkhisAmr::FathFashil`] when the opener will not start, and whatever creating
/// the directory raises.
#[tauri::command]
#[specta::specta]
pub fn iftah_tashkhis(masarat: tauri::State<'_, Masarat>) -> Result<bool, Khata> {
    let mujallad = masarat.jidhr_bayanat().join(MUJALLAD_TASHKHIS);
    // Created first, so the very first click opens a real folder rather than an
    // operating-system complaint about a missing one.
    insha_mujallad(&mujallad)?;
    let barnamij = match NizamTashghil::hali() {
        NizamTashghil::Windows => "explorer",
        NizamTashghil::Mac => "open",
        NizamTashghil::Linux => "xdg-open",
    };
    std::process::Command::new(barnamij).arg(&mujallad).spawn().map_err(|sabab| {
        Khata::from(KhataTashkhisAmr::FathFashil { sabab: sabab.to_string() })
    })?;
    Ok(true)
}

/// Failures of the diagnostics surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataTashkhisAmr {
    /// The log directory cannot be listed.
    #[error("the log directory does not read: {sabab}")]
    SijillatGhayrMaqrua {
        /// What the filesystem said.
        sabab: String,
    },

    /// No capability report is stored for the game yet.
    #[error("no capability report is stored for {ism}")]
    TaqreerMafqud {
        /// The game's display name.
        ism: String,
    },

    /// A diagnostics artifact could not be assembled.
    #[error("the diagnostics bundle could not be built: {sabab}")]
    HuzmaFashila {
        /// What failed, as the failing layer reported it.
        sabab: String,
    },

    /// The platform file manager would not start.
    #[error("the file manager would not open the diagnostics folder: {sabab}")]
    FathFashil {
        /// What the operating system said.
        sabab: String,
    },
}

impl Tafsir for KhataTashkhisAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::SijillatGhayrMaqrua { .. } => 100,
                    Self::TaqreerMafqud { .. } => 101,
                    Self::HuzmaFashila { .. } => 102,
                    Self::FathFashil { .. } => 103,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Asked for a report that was never produced; nothing was touched.
            Self::TaqreerMafqud { .. } => Khutura::Tanbeeh,
            // The user asked for their logs, a bundle, or a folder and got none.
            Self::SijillatGhayrMaqrua { .. }
            | Self::HuzmaFashila { .. }
            | Self::FathFashil { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::SijillatGhayrMaqrua { sabab } => format!(
                "تعذّرت قراءة مجلد السجلّات: {sabab}. أعد المحاولة، فإن استمر الرفض فافحص \
                 صلاحيات مجلد بيانات تعريب."
            ),
            Self::TaqreerMafqud { ism } => format!(
                "لا تقرير قدرات مخزّنًا للعبة {ism} بعد. افتح شاشة اللعبة أولًا ليفحص تعريب \
                 محرّكها، ثم أعد إنشاء التقرير."
            ),
            Self::HuzmaFashila { sabab } => format!(
                "تعذّر بناء مخرجات التشخيص: {sabab}. أعد المحاولة بعد التأكد من مساحة القرص \
                 وصلاحيات مجلد بيانات تعريب."
            ),
            Self::FathFashil { sabab } => format!(
                "تعذّر فتح مجلد التشخيص في مدير ملفات النظام: {sabab}. افتح شاشة التشخيص \
                 داخل تعريب بدلًا منه."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::SijillatGhayrMaqrua { sabab } => format!(
                "The log directory does not read ({sabab}). Try again; if it keeps refusing, \
                 check the permissions on Taarib's data folder."
            ),
            Self::TaqreerMafqud { ism } => format!(
                "No capability report is stored for {ism} yet. Open the game's screen first \
                 so Taarib probes its engine, then build the report again."
            ),
            Self::HuzmaFashila { sabab } => format!(
                "The diagnostics output could not be built ({sabab}). Try again after checking \
                 the free space and the permissions on Taarib's data folder."
            ),
            Self::FathFashil { sabab } => format!(
                "The system file manager would not open the diagnostics folder ({sabab}). Open \
                 the Diagnostics screen inside Taarib instead."
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::SijillatGhayrMaqrua { .. } | Self::HuzmaFashila { .. } => Khutwa::AadaMuhawala,
            Self::TaqreerMafqud { .. } => Khutwa::AadaFahsMuharrik,
            Self::FathFashil { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::SijillatGhayrMaqrua { sabab }
            | Self::HuzmaFashila { sabab }
            | Self::FathFashil { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::TaqreerMafqud { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataTashkhisAmr);
