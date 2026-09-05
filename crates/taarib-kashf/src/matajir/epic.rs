//! متجر إيبك — the Epic Games Store, read from its `.item` manifests.
//!
//! Epic keeps one JSON file per installed application in a machine-wide
//! manifest directory, and a second, much thinner list —
//! `LauncherInstalled.dat` — that the launcher and the Unreal Engine installer
//! both maintain. The `.item` files are the catalogue; `LauncherInstalled.dat`
//! is a cross-check that occasionally knows about an install the manifest
//! directory has lost, which happens after a launcher reinstall that kept the
//! games but not the manifests.
//!
//! ```text
//! Windows  %PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests\*.item
//!          %PROGRAMDATA%\Epic\UnrealEngineLauncher\LauncherInstalled.dat
//! macOS    /Users/Shared/Epic/EpicGamesLauncher/Data/Manifests/*.item
//!          /Users/Shared/Epic/UnrealEngineLauncher/LauncherInstalled.dat
//! ```
//!
//! Linux is not listed and is not pretended to be supported: the Epic launcher
//! does not run there natively, and the games Linux users own through Epic are
//! installed by Heroic or Legendary, whose own catalogue is a different file in
//! a different shape and therefore a different adapter.
//!
//! ## What a `.item` file says
//!
//! Every field this adapter reads is one Epic writes itself:
//!
//! | field | used for |
//! | --- | --- |
//! | `AppName` | the launcher's own key for the application, and the link a DLC entry uses to name its base game |
//! | `CatalogItemId` | the identity Taarib keeps, because [`MasdarLuba::Epic`] is the catalogue item |
//! | `DisplayName` | the name, verbatim |
//! | `InstallLocation` | the installation root |
//! | `InstallSize` | size on disk |
//! | `LaunchExecutable` | the executable, relative to the installation root |
//! | `AppVersionString` | the build identifier |
//! | `MainGameAppName` | present on every entry; **different** from `AppName` only on add-ons |
//! | `bIsIncompleteInstall` | a download that has not finished |
//! | `AppCategories` | `games`, `applications`, `engines` — how a tool is told apart from a game |
//! | `ProcessNames`, `BackgroundProcessNames`, `PrereqName`, `PrereqPath` | where an anti-cheat service names itself |
//!
//! ## Add-ons are not games
//!
//! An entry whose `MainGameAppName` differs from its `AppName` is downloadable
//! content, not a title: Epic gives it a manifest of its own because it is
//! separately installable, but it has no executable of its own and its
//! `InstallLocation` is the base game's directory. Emitting it would produce a
//! second card in the library pointing at the same folder, a second [`LubaId`]
//! for one game, and a second patch target for one set of files — and Phase 15
//! would then be able to install two patches over each other. So add-ons are
//! read, counted, and dropped. The one case that is reported is an add-on whose
//! base game has no manifest at all, because then a directory really is
//! installed and really is missing from the library, and the user deserves the
//! reason.
//!
//! [`LubaId`]: taarib_mustalahat::luba::LubaId
//!
//! ## Artwork
//!
//! Left empty, deliberately. A `.item` manifest carries no image reference of
//! any kind, and the launcher's only local image store is the Chromium web
//! cache under `EpicGamesLauncher/Saved/webcache*`, which is keyed by a hash of
//! the request rather than by application, and cannot be attributed to a game
//! without replaying the launcher's own catalogue calls. An empty
//! [`MasadirSuwar`] is handled by `suwar`; a guessed URL would be a broken
//! image in the library grid, so none is guessed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::{dakhil, qira_nass};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs, TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "epic";

/// Platforms this adapter can find anything on.
const MANASSAT: [NizamTashghil; 2] = [NizamTashghil::Windows, NizamTashghil::Mac];

/// Where the launcher keeps machine-wide data on macOS. Not under the user's
/// home directory: Epic installs for every account on the machine at once.
const JIDHR_MAC: &str = "/Users/Shared/Epic";

/// Anti-cheat services, matched case-insensitively against the process and
/// prerequisite names Epic records. These are hints for Phase 16, never
/// conclusions: the safety layer decides on evidence on disk.
const ATHAR_HIMAYA: [(&str, &str); 8] = [
    ("easyanticheat", "EasyAntiCheat"),
    ("eac_launcher", "EasyAntiCheat"),
    ("start_protected_game", "EasyAntiCheat"),
    ("battleye", "BattlEye"),
    ("beservice", "BattlEye"),
    ("xigncode", "XIGNCODE3"),
    ("vanguard", "Riot Vanguard"),
    ("punkbuster", "PunkBuster"),
];

/// The Epic Games Store.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarEpic;

impl MatjarEpic {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarEpic {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "إيبك"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Epic Games"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        // A configured root is returned whether or not it exists. The user
        // asserted it; `ifhas` is where the assertion is checked and where a
        // wrong path becomes a message they can act on, rather than silently
        // becoming "Epic is not installed".
        if let Some(tajawuz) = siyaq.manassat.epic.as_ref() {
            return Some(tajawuz.clone());
        }
        let jidhr = jidhr_tilqai(siyaq)?;
        jidhr.is_dir().then_some(jidhr)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when the user configured a
    /// launcher root that is not there, and
    /// [`KhataKashf::TaadhurQiraatFahras`] when the manifest directory exists
    /// but cannot be listed. Every narrower failure — one unreadable manifest,
    /// one game whose directory was deleted — is a [`TanbihFahs`] and costs
    /// only that entry.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let jidhr = match siyaq.manassat.epic.as_ref() {
            Some(tajawuz) if !tajawuz.exists() => {
                return Err(KhataKashf::JidhrMuhaddadMafqud {
                    matjar: MUARRIF,
                    masar: tajawuz.clone(),
                }
                .into());
            },
            Some(tajawuz) => tajawuz.clone(),
            // No data root to look in is the same answer as an empty one: the
            // launcher is not here.
            None => match jidhr_tilqai(siyaq) {
                Some(jidhr) => jidhr,
                None => return Ok(NatijatMatjar::ghayr_mutah(MUARRIF)),
            },
        };

        let Some(mujallad) = mujallad_bayanat(&jidhr) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: Some(jidhr.clone()),
            ..NatijatMatjar::default()
        };

        let malaffat = malaffat_bayan(&mujallad, MUARRIF)?;
        let mut asmaa: BTreeSet<String> = BTreeSet::new();
        let mut idafat: BTreeMap<String, String> = BTreeMap::new();
        let mut mawaqi: BTreeSet<String> = BTreeSet::new();

        for masar in malaffat {
            let nass = match qira_nass(&masar) {
                Ok(nass) => nass,
                Err(khata) => {
                    natija.tanbihat.push(TanbihFahs::jadeed(
                        MUARRIF,
                        masar.display().to_string(),
                        khata.injilizi,
                    ));
                    continue;
                },
            };
            let qeema: Value = match serde_json::from_str(bila_bom(&nass)) {
                Ok(qeema) => qeema,
                Err(khata) => {
                    natija.tanbihat.push(TanbihFahs::jadeed(
                        MUARRIF,
                        masar.display().to_string(),
                        format!("manifest is not valid JSON: {khata}"),
                    ));
                    continue;
                },
            };

            let Some(ism_tatbeeq) = nass_haql(&qeema, "AppName") else {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    masar.display().to_string(),
                    "manifest has no AppName, so it names no application".to_owned(),
                ));
                continue;
            };
            let _ = asmaa.insert(ism_tatbeeq.to_owned());

            // An add-on: same install directory as its base game, no executable
            // of its own, and no business being a second library entry.
            if let Some(asl) = nass_haql(&qeema, "MainGameAppName")
                && asl != ism_tatbeeq
            {
                let _ = idafat.insert(ism_tatbeeq.to_owned(), asl.to_owned());
                continue;
            }

            match luba_min_bayan(&qeema, ism_tatbeeq, &masar) {
                Ok(luba) => {
                    let _ = mawaqi.insert(muwahhad(&luba.jidhr, siyaq.nizam));
                    if nass_haql(&qeema, "CatalogItemId").is_none() {
                        natija.tanbihat.push(TanbihFahs::jadeed(
                            MUARRIF,
                            masar.display().to_string(),
                            format!(
                                "manifest has no CatalogItemId; identity was derived from \
                                 AppName {ism_tatbeeq} instead, and will not match a patch \
                                 published against the catalogue item"
                            ),
                        ));
                    }
                    natija.alaab.push(luba);
                },
                Err(tanbih) => natija.tanbihat.push(tanbih),
            }
        }

        for (idafa, asl) in &idafat {
            if !asmaa.contains(asl) {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    idafa.clone(),
                    format!(
                        "this entry is add-on content for {asl}, which has no manifest of its \
                         own, so the game it belongs to cannot be shown"
                    ),
                ));
            }
        }

        idaf_min_qaimat_altathbeet(&jidhr, siyaq, &asmaa, &mut mawaqi, &mut natija);

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let Some(jidhr) = siyaq.manassat.epic.clone().or_else(|| jidhr_tilqai(siyaq)) else {
            return Vec::new();
        };
        let mut judhur = Vec::new();
        if let Some(mujallad) = mujallad_bayanat(&jidhr) {
            judhur.push(mujallad);
        }
        if let Some(malaf) = malaf_qaimat_altathbeet(&jidhr)
            && let Some(mujallad) = malaf.parent()
        {
            judhur.push(mujallad.to_path_buf());
        }
        judhur
    }
}

/// The launcher's machine-wide data root, before any user override.
///
/// [`None`] only on a Windows context that names no `%PROGRAMDATA%`, which is a
/// machine with nowhere for the launcher's data to be. The macOS path is a
/// fixed absolute one and is returned on Linux too, where it is what a copied
/// or prefix-mounted layout looks like.
fn jidhr_tilqai(siyaq: &SiyaqFahs) -> Option<PathBuf> {
    match siyaq.nizam {
        NizamTashghil::Windows => {
            siyaq.bayanat_barnamij.as_ref().map(|bayanat| bayanat.join("Epic"))
        },
        NizamTashghil::Mac | NizamTashghil::Linux => Some(PathBuf::from(JIDHR_MAC)),
    }
}

/// Resolves the manifest directory under a launcher root.
///
/// Four shapes are accepted, because "the Epic root" means different things to
/// different people: the manifest directory itself, a `Data` parent, the
/// launcher directory, and the Epic data root.
fn mujallad_bayanat(jidhr: &Path) -> Option<PathBuf> {
    let murashahat = [
        jidhr.to_path_buf(),
        jidhr.join("Manifests"),
        jidhr.join("Data").join("Manifests"),
        jidhr.join("EpicGamesLauncher").join("Data").join("Manifests"),
    ];
    murashahat.into_iter().find(|masar| {
        masar.is_dir() && (masar.ends_with("Manifests") || fih_bayan(masar))
    })
}

/// Whether a directory holds at least one `.item` file, which is what makes it
/// a manifest directory rather than a directory that happens to be named one.
fn fih_bayan(mujallad: &Path) -> bool {
    let Ok(qaima) = std::fs::read_dir(mujallad) else {
        return false;
    };
    qaima.flatten().any(|madkhal| imtidad_hua(&madkhal.path(), "item"))
}

/// The cross-check list, when it is where the launcher puts it.
fn malaf_qaimat_altathbeet(jidhr: &Path) -> Option<PathBuf> {
    let murashahat = [
        jidhr.join("UnrealEngineLauncher").join("LauncherInstalled.dat"),
        jidhr.join("LauncherInstalled.dat"),
        jidhr
            .parent()
            .map_or_else(|| jidhr.to_path_buf(), Path::to_path_buf)
            .join("UnrealEngineLauncher")
            .join("LauncherInstalled.dat"),
    ];
    murashahat.into_iter().find(|masar| masar.is_file())
}

/// Every `.item` file in the manifest directory, in a deterministic order.
///
/// # Errors
///
/// Fails only when the directory itself cannot be listed, which is the one
/// condition that makes the whole catalogue unreadable.
fn malaffat_bayan(mujallad: &Path, matjar: &'static str) -> Natija<Vec<PathBuf>> {
    let qaima = std::fs::read_dir(mujallad).map_err(|sabab| KhataKashf::TaadhurQiraatFahras {
        matjar,
        masar: mujallad.to_path_buf(),
        sabab,
    })?;
    let mut malaffat: Vec<PathBuf> =
        qaima
            .flatten()
            .map(|madkhal| madkhal.path())
            .filter(|masar| imtidad_hua(masar, "item"))
            .collect();
    malaffat.sort();
    Ok(malaffat)
}

/// Builds one game out of one manifest.
fn luba_min_bayan(
    qeema: &Value,
    ism_tatbeeq: &str,
    masar_bayan: &Path,
) -> Result<LubaMuktashafa, TanbihFahs> {
    let muarrif_catalog =
        nass_haql(qeema, "CatalogItemId").map_or_else(|| ism_tatbeeq.to_owned(), str::to_owned);

    let Some(mawqi) = nass_haql(qeema, "InstallLocation") else {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            masar_bayan.display().to_string(),
            "manifest has no InstallLocation, so there is nothing on disk to point at".to_owned(),
        ));
    };
    let jidhr = PathBuf::from(mawqi);
    if !jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            "the launcher still lists this game, but its install directory is gone; reinstall it \
             or let the launcher repair it"
                .to_owned(),
        ));
    }

    let ism = nass_haql(qeema, "DisplayName")
        .map(str::to_owned)
        .or_else(|| ism_min_mujallad(&jidhr))
        .unwrap_or_else(|| ism_tatbeeq.to_owned());

    let tanfidhi = nass_haql(qeema, "LaunchExecutable")
        .and_then(|nisbi| masar_dakhili(&jidhr, nisbi))
        .filter(|masar| masar.is_file());

    let mut simat = Vec::new();
    let asnaf = qaimat_nusus(qeema, "AppCategories");
    if !asnaf.is_empty() && !asnaf.iter().any(|sinf| sinf.eq_ignore_ascii_case("games")) {
        simat.push(SimatLuba::LaysatLuba(asnaf.join(", ")));
    }
    for himaya in himayat(qeema) {
        simat.push(SimatLuba::HimayaMuhtamala(himaya));
    }

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Epic(muarrif_catalog),
        hala_matjar: None,
        ism,
        jidhr,
        tanfidhi,
        hajm: raqm_haql(qeema, "InstallSize").unwrap_or(0),
        bina_manassa: nass_haql(qeema, "AppVersionString").map(str::to_owned),
        // Epic records no update or play timestamp in a `.item` file. The
        // file's own modification time is not that timestamp — it is rewritten
        // by a verify pass as well — so nothing is claimed here.
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        suwar: MasadirSuwar::default(),
        khiyarat_tashghil: None,
        muktamila: !sawab_haql(qeema, "bIsIncompleteInstall").unwrap_or(false),
        simat,
    })
}

/// Reads `LauncherInstalled.dat` and adds what the manifest directory missed.
///
/// Entries already covered by a manifest — by application name or by install
/// directory — are skipped, which is what keeps this from doubling the library.
fn idaf_min_qaimat_altathbeet(
    jidhr: &Path,
    siyaq: &SiyaqFahs,
    asmaa: &BTreeSet<String>,
    mawaqi: &mut BTreeSet<String>,
    natija: &mut NatijatMatjar,
) {
    let Some(masar) = malaf_qaimat_altathbeet(jidhr) else {
        return;
    };
    let nass = match qira_nass(&masar) {
        Ok(nass) => nass,
        Err(khata) => {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                khata.injilizi,
            ));
            return;
        },
    };
    let qeema: Value = match serde_json::from_str(bila_bom(&nass)) {
        Ok(qeema) => qeema,
        Err(khata) => {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                format!("cross-check list is not valid JSON: {khata}"),
            ));
            return;
        },
    };
    let Some(qaima) = qeema.get("InstallationList").and_then(Value::as_array) else {
        return;
    };

    for madkhal in qaima {
        let Some(mawqi) = nass_haql(madkhal, "InstallLocation") else {
            continue;
        };
        let masar_luba = PathBuf::from(mawqi);
        if !masar_luba.is_dir() {
            continue;
        }
        let muwahhad_masar = muwahhad(&masar_luba, siyaq.nizam);
        if mawaqi.contains(&muwahhad_masar) {
            continue;
        }
        let ism_tatbeeq = nass_haql(madkhal, "AppName").unwrap_or_default();
        if !ism_tatbeeq.is_empty() && asmaa.contains(ism_tatbeeq) {
            continue;
        }

        let muarrif_catalog = nass_haql(madkhal, "ItemId")
            .or_else(|| nass_haql(madkhal, "AppName"))
            .or_else(|| nass_haql(madkhal, "ArtifactId"));
        let Some(muarrif_catalog) = muarrif_catalog else {
            continue;
        };

        let mut simat = Vec::new();
        let athar = nass_haql(madkhal, "ArtifactId").unwrap_or(ism_tatbeeq);
        if athar.starts_with("UE_") || ism_tatbeeq.starts_with("UE_") {
            simat.push(SimatLuba::LaysatLuba("Unreal Engine".to_owned()));
        }

        let _ = mawaqi.insert(muwahhad_masar);
        natija.alaab.push(LubaMuktashafa {
            masdar: MasdarLuba::Epic(muarrif_catalog.to_owned()),
            hala_matjar: None,
            ism: ism_min_mujallad(&masar_luba)
                .or_else(|| nass_haql(madkhal, "ArtifactId").map(str::to_owned))
                .unwrap_or_else(|| ism_tatbeeq.to_owned()),
            jidhr: masar_luba,
            tanfidhi: None,
            hajm: 0,
            bina_manassa: nass_haql(madkhal, "AppVersion").map(str::to_owned),
            akhir_tahdith: None,
            akhir_laab: None,
            beea: BeeatTawafuq::Asli,
            suwar: MasadirSuwar::default(),
            khiyarat_tashghil: None,
            // The cross-check list carries no completion flag. An entry only
            // reaches it once the launcher has finished writing the install, so
            // treating it as complete is the launcher's own claim, not a guess
            // about the bytes on disk.
            muktamila: true,
            simat,
        });
    }
}

/// Anti-cheat services named anywhere in the manifest's process fields.
fn himayat(qeema: &Value) -> Vec<String> {
    let mut nusus = qaimat_nusus(qeema, "ProcessNames");
    nusus.extend(qaimat_nusus(qeema, "BackgroundProcessNames"));
    nusus.extend(nass_haql(qeema, "PrereqName").map(str::to_owned));
    nusus.extend(nass_haql(qeema, "PrereqPath").map(str::to_owned));
    nusus.extend(nass_haql(qeema, "LaunchExecutable").map(str::to_owned));

    let mut wujida: BTreeSet<String> = BTreeSet::new();
    for nass in nusus {
        let munkhafid = nass.to_ascii_lowercase();
        for (athar, ism) in ATHAR_HIMAYA {
            if munkhafid.contains(athar) {
                let _ = wujida.insert(ism.to_owned());
            }
        }
    }
    wujida.into_iter().collect()
}

/// A trimmed, non-empty string field.
fn nass_haql<'a>(qeema: &'a Value, miftah: &str) -> Option<&'a str> {
    qeema
        .get(miftah)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|nass| !nass.is_empty())
}

/// A number field, accepting the string form some launcher versions write.
fn raqm_haql(qeema: &Value, miftah: &str) -> Option<u64> {
    match qeema.get(miftah) {
        Some(Value::Number(raqm)) => raqm.as_u64(),
        Some(Value::String(nass)) => nass.trim().parse::<u64>().ok(),
        _ => None,
    }
}

/// A boolean field, accepting the number and string forms.
fn sawab_haql(qeema: &Value, miftah: &str) -> Option<bool> {
    match qeema.get(miftah) {
        Some(Value::Bool(sawab)) => Some(*sawab),
        Some(Value::Number(raqm)) => raqm.as_u64().map(|qeema| qeema != 0),
        Some(Value::String(nass)) => match nass.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// A string array field, empty when the field is absent or is not an array.
fn qaimat_nusus(qeema: &Value, miftah: &str) -> Vec<String> {
    qeema.get(miftah).and_then(Value::as_array).map_or_else(Vec::new, |qaima| {
        qaima
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|nass| !nass.is_empty())
            .map(str::to_owned)
            .collect()
    })
}

/// Joins a launcher-supplied relative path onto an install root through the
/// one join in the product that refuses to leave its root.
fn masar_dakhili(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munazzam = nisbi.replace('\\', "/");
    let munazzam = munazzam.trim_start_matches('/');
    dakhil(jidhr, munazzam).ok()
}

/// An install directory's own name, which is what Epic names most of them.
fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().into_owned())
        .map(|ism| ism.trim().to_owned())
        .filter(|ism| !ism.is_empty())
}

/// Whether a path carries an extension, compared without regard to case.
fn imtidad_hua(masar: &Path, imtidad: &str) -> bool {
    masar
        .extension()
        .and_then(|qeema| qeema.to_str())
        .is_some_and(|qeema| qeema.eq_ignore_ascii_case(imtidad))
}

/// A path in the one form two paths can be compared in on this platform.
fn muwahhad(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy().replace('\\', "/");
    let nass = nass.trim_end_matches('/').to_owned();
    if nizam.hassas_lil_ahruf() { nass } else { nass.to_lowercase() }
}

/// Strips a byte order mark, which several launcher versions write in front of
/// otherwise ordinary UTF-8 JSON and which no JSON parser accepts.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    #[test]
    fn jidhr_windows_min_bayanat_al_barnamij_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let bayanat = masrah.path().join("ProgramData");
        fs::create_dir_all(bayanat.join("Epic").join("EpicGamesLauncher").join("Data"))?;

        // No variable is set anywhere in this test. The launcher root is found
        // because the context named the machine's data folder, which is what
        // makes a Windows-only path testable from a Linux host at all.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_barnamij = Some(bayanat.clone());
        assert_eq!(jidhr_tilqai(&siyaq), Some(bayanat.join("Epic")));
        assert_eq!(MatjarEpic::jadeed().mawqi(&siyaq), Some(bayanat.join("Epic")));
        Ok(())
    }

    #[test]
    fn bila_bayanat_barnamij_la_yuqrar_annahu_ghayr_muthabbat() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());

        assert_eq!(jidhr_tilqai(&siyaq), None);
        assert_eq!(MatjarEpic::jadeed().mawqi(&siyaq), None);

        // An absent root is "not installed", never a failure: nine other
        // launchers' games are still coming.
        let natija = MatjarEpic::jadeed().ifhas(&siyaq)?;
        assert!(natija.jidhr_matjar.is_none());
        assert!(natija.alaab.is_empty());
        assert!(MatjarEpic::jadeed().judhur_muraqaba(&siyaq).is_empty());
        Ok(())
    }

    #[test]
    fn jidhr_mac_thabit_la_yamurru_bi_al_beea() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Mac, masrah.path());

        // macOS keeps the launcher's data at one absolute path that no variable
        // relocates, so it is unaffected by the Windows field being absent.
        assert_eq!(jidhr_tilqai(&siyaq), Some(PathBuf::from(JIDHR_MAC)));
        Ok(())
    }
}
