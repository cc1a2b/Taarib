//! بيان المخزن — the component store's manifest, and what "complete" means.

use std::path::Path;

use taarib_usus::masarat;

use crate::khata::KhataTathbeet;

/// The staging manifest's file name, identical in the bundle and in the store.
///
/// Spelled here and nowhere else. `taarib-tajmee` writes this file, the Studio
/// mirrors it into the component store, and this crate reads it to decide
/// whether an install may proceed. Three crates that cannot see each other, one
/// name: a rename in any single copy would leave the installer finding no
/// manifest and refusing every component, from a one-word edit.
pub const ISM_MALAF_BAYAN: &str = "bayan_mukawwinat.json";

/// The one manifest schema number this build understands (`tawzee.md` §4).
pub const MUKHATTAT_MADUM: u32 = 1;

/// The manifest prefix that maps into the component store.
///
/// An entry `mukawwinat/<baqi>` mirrors to `<store>/<baqi>`. Everything else —
/// fonts, the signature database — rides beside the binary and is verified
/// without being mirrored.
pub const BADIYAT_MAKHZAN: &str = "mukawwinat/";

/// The largest manifest this build will read, in bytes.
///
/// A real one is a few kilobytes; anything near this limit is not a manifest.
const AQSA_HAJM_BAYAN: u64 = 8 * 1024 * 1024;

/// One staged file: a forward-slash path relative to `mawarid/`, its size, and
/// the sha256 the staging tool verified before writing it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MalafMudraj {
    /// The path, forward-slash, relative to `mawarid/`.
    pub masar: String,
    /// The size in bytes.
    pub hajm: u64,
    /// The content hash, 64 lowercase hexadecimal characters.
    pub sha256: String,
}

/// The staging manifest `taarib-tajmee` writes, `tawzee.md` §4 verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BayanMukawwinat {
    /// The manifest schema number; this build understands [`MUKHATTAT_MADUM`].
    pub mukhattat: u32,
    /// The workspace version the bundle was staged from.
    pub isdar: String,
    /// The target triple the bundle was staged for.
    pub hadaf: String,
    /// Every staged file, in the order the staging tool wrote them.
    pub milaffat: Vec<MalafMudraj>,
}

/// Reads the component store's manifest, when it has one.
///
/// [`None`] means the store carries no manifest at all — a development build
/// run from cargo, or a mirror that never settled. That is not an error here;
/// it is the caller's decision what an unmanifested store may be used for.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when the manifest is present and cannot be
/// read, is larger than this build will parse, is not JSON, or declares a
/// schema this build does not understand. A manifest that exists and cannot be
/// trusted is worse than none, so it is refused rather than ignored.
pub fn iqra_bayan(jidhr_makhzan: &Path) -> Result<Option<BayanMukawwinat>, KhataTathbeet> {
    let masar = jidhr_makhzan.join(ISM_MALAF_BAYAN);
    let Ok(wasf) = std::fs::metadata(&masar) else {
        return Ok(None);
    };
    let marfud = |sabab: &str| KhataTathbeet::MukawwinMafqud {
        mukawwin: format!("{ISM_MALAF_BAYAN} ({sabab})"),
        masar: masar.clone(),
    };
    if !wasf.is_file() || wasf.len() > AQSA_HAJM_BAYAN {
        return Err(marfud("not a readable manifest"));
    }
    let bayt = std::fs::read(&masar).map_err(|_| marfud("could not be read"))?;
    let bayan: BayanMukawwinat =
        serde_json::from_slice(&bayt).map_err(|_| marfud("is not the manifest this build reads"))?;
    if bayan.mukhattat != MUKHATTAT_MADUM {
        return Err(marfud("declares a schema this build does not understand"));
    }
    Ok(Some(bayan))
}

/// Proves every file the manifest lists for a component is present at its
/// declared size.
///
/// "The directory holds at least one file" is not completeness, and the
/// difference is the product's worst silent failure: a mirror interrupted after
/// three of a framework's forty files leaves a directory that answers yes to
/// that question, and the install then deploys three files, records a success,
/// and leaves the game in English with nothing logged.
///
/// Sizes rather than hashes. The bytes were hashed on the way into the store,
/// and re-hashing a BepInEx tree on every install would put seconds onto a
/// confirmation screen to re-answer a question already answered. Truncation and
/// replacement both change a file's length, which is what this catches.
///
/// A store with no manifest is accepted. A development build run from cargo has
/// no manifest and must still be able to install; the missing manifest is
/// already reported at startup as its own named problem.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when a listed file is absent or when the
/// manifest lists nothing at all under this component, and
/// [`KhataTathbeet::MukawwinNaqis`] when a file is present at the wrong size.
pub fn kamil_hasab_bayan(jidhr_makhzan: &Path, ism: &str) -> Result<(), KhataTathbeet> {
    let Some(bayan) = iqra_bayan(jidhr_makhzan)? else {
        return Ok(());
    };

    let badiya = format!("{BADIYAT_MAKHZAN}{ism}/");
    let mut adad = 0_usize;
    for malaf in &bayan.milaffat {
        if !malaf.masar.starts_with(&badiya) {
            continue;
        }
        let Some(dhayl) = malaf.masar.strip_prefix(BADIYAT_MAKHZAN) else {
            continue;
        };
        adad = adad.saturating_add(1);
        let masar = masarat::dakhil(jidhr_makhzan, dhayl).map_err(|khata| {
            KhataTathbeet::MasarKharij {
                masar: std::path::PathBuf::from(dhayl),
                jidhr: jidhr_makhzan.to_path_buf(),
                sabab: khata.injilizi,
            }
        })?;
        let Ok(wasf) = std::fs::metadata(&masar) else {
            return Err(KhataTathbeet::MukawwinMafqud {
                mukawwin: format!("{ism}/{dhayl}"),
                masar,
            });
        };
        if !wasf.is_file() || wasf.len() != malaf.hajm {
            return Err(KhataTathbeet::MukawwinNaqis {
                mukawwin: ism.to_owned(),
                masar,
                muallan: malaf.hajm,
                mawjud: wasf.len(),
            });
        }
    }

    // A component the manifest never listed is not a component this build
    // ships, whatever happens to be sitting in a directory with its name.
    if adad == 0 {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr_makhzan.join(ism),
        });
    }
    Ok(())
}
