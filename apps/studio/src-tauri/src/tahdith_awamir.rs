//! التحديث — the update surface: check the signed channel, fetch, and swap.

use std::path::PathBuf;

use taarib_tahdith::bayan::{BayanTahdith, MadkhalTahdith};
use taarib_tahdith::{ihlil, intiqa, jalb, tabdil};
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat::Masarat;
use taarib_usus::ISDAR;

use std::sync::Arc;

/// The channel manifest's name under the official registry root.
const ISM_BAYAN: &str = "tahdith.json";

/// Its detached signature, beside it.
const ISM_TAWQEE: &str = "tahdith.json.tawqee";

/// What the interface draws when an update is offered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TahdithHie {
    /// The version being offered.
    pub isdar: String,
    /// Its size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// Which channel it came from.
    pub qanat: String,
    /// Whether this installation can replace itself, or a package manager owns
    /// it and the user updates from there.
    pub qabil_lil_tabdil: bool,
}

/// The target triple this build runs on, as the channel manifest names it.
const fn hadaf_hali() -> &'static str {
    // A single source rather than `std::env::consts` assembled at runtime: the
    // manifest's keys are Rust target triples, and this build knows its own.
    env!("TAARIB_HADAF")
}

/// Fetches and verifies the channel manifest.
async fn ijlib_bayan(idadat: &Idadat) -> Natija<BayanTahdith> {
    let jidhr = idadat.masadir.rasmi.trim_end_matches('/');
    let mabni = reqwest::Client::new();

    let matn = mabni
        .get(format!("{jidhr}/{ISM_BAYAN}"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|khata| khata_qanat(&khata))?
        .bytes()
        .await
        .map_err(|khata| khata_qanat(&khata))?;

    let tawqee = mabni
        .get(format!("{jidhr}/{ISM_TAWQEE}"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|khata| khata_qanat(&khata))?
        .text()
        .await
        .map_err(|khata| khata_qanat(&khata))?;

    // The same anchor that verifies a patch verifies the channel: a release
    // client cannot be updated by anything the owner did not sign.
    ihlil(&matn, tawqee.trim(), &taarib_khatm::MIRSAT_MALIK).map_err(|khata| Khata::min_tafsir(&khata))
}

/// One transport failure, as the update crate names it.
fn khata_qanat(khata: &reqwest::Error) -> Khata {
    Khata::min_tafsir(&taarib_tahdith::KhataTahdith::QanatGhayrMutaha {
        sabab: khata.to_string(),
    })
}

/// Whether a newer version is offered for this build on its channel.
///
/// Answers `None` when this build is current, which is the ordinary case and
/// not a failure.
///
/// # Errors
///
/// Whatever the channel, its signature, or the version comparison refuses —
/// including a channel signed by the development key under a release build,
/// which is refused by name.
#[tauri::command]
#[specta::specta]
pub async fn tahaqquq_tahdith(
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<Option<TahdithHie>, Khata> {
    let idadat = Arc::unwrap_or_clone(makhzan.hali());
    let bayan = ijlib_bayan(&idadat).await?;
    let qanat = qanat_nassiya(&idadat);

    let Some(madkhal) = intiqa(&bayan, hadaf_hali(), &qanat, ISDAR).map_err(|khata| Khata::min_tafsir(&khata))?
    else {
        return Ok(None);
    };

    let tareeqa = tabdil::istadill(&masar_tanfidhi()?);
    Ok(Some(TahdithHie {
        isdar: bayan.isdar.clone(),
        hajm: madkhal.hajm,
        qanat: madkhal.qanat.clone(),
        qabil_lil_tabdil: tareeqa != tabdil::TareeqatTabdil::Mudar,
    }))
}

/// Downloads the offered update, verifies it, and stages the swap.
///
/// The swap itself happens on the next launch for the formats that replace a
/// running file, so this answers the path the user can see rather than
/// restarting anything behind their back.
///
/// # Errors
///
/// Whatever the channel, the download, the hash check, the free-space check or
/// the swap refuses. A package-manager installation is refused by name.
#[tauri::command]
#[specta::specta]
pub async fn nazzil_tahdith(
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
    masarat: tauri::State<'_, Masarat>,
) -> Result<String, Khata> {
    let idadat = Arc::unwrap_or_clone(makhzan.hali());
    let bayan = ijlib_bayan(&idadat).await?;
    let qanat = qanat_nassiya(&idadat);

    let madkhal: &MadkhalTahdith = intiqa(&bayan, hadaf_hali(), &qanat, ISDAR)
        .map_err(|khata| Khata::min_tafsir(&khata))?
        .ok_or_else(|| {
            Khata::min_tafsir(&taarib_tahdith::KhataTahdith::LaMadkhal {
                hadaf: hadaf_hali().to_owned(),
                qanat: qanat.clone(),
            })
        })?;

    let sandooq = masarat.sandooq();
    taarib_usus::masarat::insha_mujallad(&sandooq)?;
    let malaf = jalb::ijlib(madkhal, &sandooq).await.map_err(|khata| Khata::min_tafsir(&khata))?;

    let tanfidhi = masar_tanfidhi()?;
    let tareeqa = tabdil::istadill(&tanfidhi);
    let khutta = tabdil::khattit(tareeqa, &tanfidhi, &malaf).map_err(|khata| Khata::min_tafsir(&khata))?;

    if khutta.tareeqa == tabdil::TareeqatTabdil::Nsis {
        // The installer replaces a running executable, so it is run at exit
        // rather than from under the process it is replacing.
        return Ok(khutta.masdar.to_string_lossy().into_owned());
    }
    tabdil::naffidh(&khutta).map_err(|khata| Khata::min_tafsir(&khata))?;
    Ok(khutta.hadaf.to_string_lossy().into_owned())
}

/// The channel the settings name, as the manifest spells it.
fn qanat_nassiya(idadat: &Idadat) -> String {
    match idadat.tahdith.qanat {
        taarib_usus::idadat::QanatTahdith::Mustaqirr => "mustaqirr".to_owned(),
        taarib_usus::idadat::QanatTahdith::Tajribi => "tajribi".to_owned(),
    }
}

/// This process's own executable.
fn masar_tanfidhi() -> Natija<PathBuf> {
    std::env::current_exe().map_err(|sabab| {
        Khata::min_tafsir(&taarib_tahdith::KhataTahdith::KhataMalaf {
            masar: PathBuf::from("."),
            amal: "locating the running executable",
            sabab,
        })
    })
}
