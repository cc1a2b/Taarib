//! The pinned downloads: reading a lock, and refusing anything it does not vouch for.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest as _, Sha256};

use crate::khata::{KhataTajmee, NatijatTajmee};
use crate::nasakh::{Mustaqarr, hex_min_bayt};

/// One pinned artifact.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MadkhalQufl {
    /// What this entry is, e.g. `bepinex-mono-windows` or a font file's path.
    pub(crate) muarrif: String,
    /// Where it is fetched from.
    pub(crate) rabt: String,
    /// The upstream version or tag it was pinned at.
    #[expect(
        dead_code,
        reason = "carried so that an entry with no pinned version is refused when the \
                  lock is parsed; staging itself verifies by hash, never by version"
    )]
    pub(crate) isdar: String,
    /// Where inside the staged tree it lands, relative to the row's root.
    #[serde(default)]
    pub(crate) wajha: Option<String>,
    /// Its length in bytes, when known.
    #[serde(default)]
    #[expect(
        dead_code,
        reason = "the pinning tool records it and an operator reads it; the hash is \
                  what staging checks, and it already covers the length"
    )]
    pub(crate) hajm: Option<u64>,
    /// Its verified content hash, absent when the fetch could not be verified.
    #[serde(default)]
    pub(crate) sha256: Option<String>,
    /// Whether this entry is a zip to unpack rather than a file to place.
    #[serde(default)]
    pub(crate) huzma: bool,
    /// Set when the entry could not be resolved and must be refused.
    #[serde(default)]
    pub(crate) ghaib: bool,
}

/// A whole lock file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Qufl {
    /// The schema revision.
    #[expect(
        dead_code,
        reason = "required of a well-formed lock, so a file without it is refused when \
                  it is parsed; no revision but 1 exists yet to branch on"
    )]
    pub(crate) mukhattat: u32,
    /// Every pinned artifact.
    pub(crate) madakhil: Vec<MadkhalQufl>,
    /// The directory fetched copies are cached in, relative to the workspace.
    #[serde(default = "makhbaa_iftiradi")]
    pub(crate) makhbaa: String,
}

fn makhbaa_iftiradi() -> String {
    "target/tajmee/makhbaa".to_owned()
}

impl Qufl {
    /// Reads a lock file.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when it cannot be read or does not parse.
    pub(crate) fn iqra(masar: &Path) -> NatijatTajmee<Self> {
        let bayt = std::fs::read(masar).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "reading a lock file",
            sabab,
        })?;
        serde_json::from_slice(&bayt).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "parsing a lock file",
            sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, sabab.to_string()),
        })
    }

    /// The cached bytes of one entry, verified against its lock.
    ///
    /// Never fetches without `jalb`, and never accepts an entry the lock does
    /// not vouch for: an unhashed or absent-marked entry is a refusal, because
    /// a staging tool that trusted an unverified download would be the one
    /// place the whole signing story could be walked around.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] on local read failures.
    fn hat(
        &self,
        saf: &'static str,
        madkhal: &MadkhalQufl,
        jalb: bool,
        jidhr: &Path,
    ) -> NatijatTajmee<Result<Vec<u8>, KhataTajmee>> {
        if madkhal.ghaib || madkhal.sha256.is_none() {
            return Ok(Err(KhataTajmee::QuflNaqis {
                saf,
                muarrif: madkhal.muarrif.clone(),
            }));
        }
        let muqfal = madkhal.sha256.clone().unwrap_or_default();
        let makhbaa = jidhr.join(&self.makhbaa).join(&muqfal);

        if !makhbaa.is_file() {
            if !jalb {
                return Ok(Err(KhataTajmee::MukawwinGhaib {
                    saf,
                    masar: makhbaa,
                }));
            }
            // A fetch verifies before it renames onto the cache path, so its
            // bytes arrive already checked and re-reading them here would only
            // read back what was written a moment ago.
            return Ok(ijlib(saf, madkhal, &muqfal, &makhbaa));
        }

        let bayt = std::fs::read(&makhbaa).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: makhbaa.clone(),
            amal: "reading a cached download",
            sabab,
        })?;
        let mahsuba = hex_min_bayt(&Sha256::digest(&bayt));
        if !mahsuba.eq_ignore_ascii_case(&muqfal) {
            return Ok(Err(KhataTajmee::QuflGhayrMutabiq {
                saf,
                muarrif: madkhal.muarrif.clone(),
                muqfal,
                mahsuba,
            }));
        }
        Ok(Ok(bayt))
    }

    /// Stages one named entry into the tree at `wajha`.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] on local read or write failures.
    pub(crate) fn ifragh(
        &self,
        saf: &'static str,
        muarrif: &str,
        wajha: &str,
        jidhr: &Path,
        jalb: bool,
        mustaqarr: &mut Mustaqarr,
    ) -> NatijatTajmee<()> {
        let Some(madkhal) = self
            .madakhil
            .iter()
            .find(|madkhal| madkhal.muarrif == muarrif)
        else {
            mustaqarr.sajjil_naqs(KhataTajmee::QuflNaqis {
                saf,
                muarrif: muarrif.to_owned(),
            });
            return Ok(());
        };
        self.ifragh_madkhal(saf, madkhal, wajha, jidhr, jalb, mustaqarr)
    }

    /// Stages every entry of a lock under one root.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] on local read or write failures.
    pub(crate) fn ifragh_kul(
        &self,
        saf: &'static str,
        jidhr_wajha: &str,
        jidhr: &Path,
        jalb: bool,
        mustaqarr: &mut Mustaqarr,
    ) -> NatijatTajmee<()> {
        for madkhal in &self.madakhil {
            let dhayl = madkhal
                .wajha
                .clone()
                .unwrap_or_else(|| madkhal.muarrif.clone());
            self.ifragh_madkhal(
                saf,
                madkhal,
                &format!("{jidhr_wajha}/{dhayl}"),
                jidhr,
                jalb,
                mustaqarr,
            )?;
        }
        Ok(())
    }

    fn ifragh_madkhal(
        &self,
        saf: &'static str,
        madkhal: &MadkhalQufl,
        wajha: &str,
        jidhr: &Path,
        jalb: bool,
        mustaqarr: &mut Mustaqarr,
    ) -> NatijatTajmee<()> {
        let bayt = match self.hat(saf, madkhal, jalb, jidhr)? {
            Ok(bayt) => bayt,
            Err(khata) => {
                mustaqarr.sajjil_naqs(khata);
                return Ok(());
            },
        };
        if madkhal.huzma {
            ifragh_huzma(&bayt, wajha, mustaqarr)
        } else {
            mustaqarr.uktub(&bayt, wajha)
        }
    }
}

/// Unpacks a zip into the tree, refusing any entry that escapes it.
fn ifragh_huzma(bayt: &[u8], wajha: &str, mustaqarr: &mut Mustaqarr) -> NatijatTajmee<()> {
    let mut arshif = zip::ZipArchive::new(std::io::Cursor::new(bayt)).map_err(|sabab| {
        KhataTajmee::KhataMalaf {
            masar: PathBuf::from(wajha),
            amal: "opening a locked archive",
            sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, sabab.to_string()),
        }
    })?;
    for fihris in 0..arshif.len() {
        let mut madkhal = arshif
            .by_index(fihris)
            .map_err(|sabab| KhataTajmee::KhataMalaf {
                masar: PathBuf::from(wajha),
                amal: "reading a locked archive entry",
                sabab: std::io::Error::new(std::io::ErrorKind::InvalidData, sabab.to_string()),
            })?;
        if madkhal.is_dir() {
            continue;
        }
        // An archive entry naming `..` or an absolute path would write outside
        // the staging tree; a zip is untrusted input like any other.
        let Some(nisbi) = madkhal.enclosed_name() else {
            continue;
        };
        let dhayl = nisbi.to_string_lossy().replace('\\', "/");
        let mut muhtawa = Vec::new();
        std::io::copy(&mut madkhal, &mut muhtawa).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: PathBuf::from(&dhayl),
            amal: "unpacking a locked archive entry",
            sabab,
        })?;
        mustaqarr.uktub(&muhtawa, &format!("{wajha}/{dhayl}"))?;
    }
    Ok(())
}

/// Fetches one locked URL into the cache and returns its verified bytes.
///
/// `curl` rather than an HTTP client dependency: staging runs on a build
/// machine where curl is already a prerequisite, and a tool that only ever
/// fetches pinned, hash-verified bytes does not need a TLS stack of its own.
///
/// The transfer lands *beside* the content-addressed entry and is renamed onto
/// it only once the bytes hash to `muqfal`. `curl --output` pointed straight at
/// that path creates and truncates the file before the first byte arrives, so a
/// connection dropped mid-transfer left a partial file whose name asserts a hash
/// its contents do not have. The next run found `is_file()`, skipped the fetch,
/// and reported [`KhataTajmee::QuflGhayrMutabiq`] — which on a pinned download
/// reads as a supply-chain alarm rather than as yesterday's bad network.
///
/// The temporary name carries the process id so two staging runs fetching the
/// same entry cannot write to one another's partial file; both then rename onto
/// the same final path, which is harmless because the path is the hash.
fn ijlib(
    saf: &'static str,
    madkhal: &MadkhalQufl,
    muqfal: &str,
    hadaf: &Path,
) -> Result<Vec<u8>, KhataTajmee> {
    if let Some(walid) = hadaf.parent() {
        std::fs::create_dir_all(walid).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: walid.to_path_buf(),
            amal: "creating the download cache",
            sabab,
        })?;
    }
    let muaqqat = hadaf.with_extension(format!("juzii.{}", std::process::id()));

    let khraj = std::process::Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(&muaqqat)
        .arg(&madkhal.rabt)
        .output()
        .map_err(|_| KhataTajmee::AdaaGhaiba { adaa: "curl" })?;
    if !khraj.status.success() {
        let _ = std::fs::remove_file(&muaqqat);
        return Err(KhataTajmee::AdaaFashila {
            adaa: "curl",
            amal: format!("fetching {}", madkhal.rabt),
            sabab: String::from_utf8_lossy(&khraj.stderr).trim().to_owned(),
        });
    }

    let bayt = match std::fs::read(&muaqqat) {
        Ok(bayt) => bayt,
        Err(sabab) => {
            let _ = std::fs::remove_file(&muaqqat);
            return Err(KhataTajmee::KhataMalaf {
                masar: muaqqat,
                amal: "reading a fetched download",
                sabab,
            });
        },
    };
    let mahsuba = hex_min_bayt(&Sha256::digest(&bayt));
    if !mahsuba.eq_ignore_ascii_case(muqfal) {
        let _ = std::fs::remove_file(&muaqqat);
        return Err(KhataTajmee::QuflGhayrMutabiq {
            saf,
            muarrif: madkhal.muarrif.clone(),
            muqfal: muqfal.to_owned(),
            mahsuba,
        });
    }

    std::fs::rename(&muaqqat, hadaf).map_err(|sabab| KhataTajmee::KhataMalaf {
        masar: muaqqat,
        amal: "moving a verified download into the cache",
        sabab,
    })?;
    Ok(bayt)
}
