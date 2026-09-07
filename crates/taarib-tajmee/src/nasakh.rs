//! Copying, hashing, and the staged manifest.

use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest as _, Sha256};

use crate::khata::{KhataTajmee, NatijatTajmee};

/// One staged file, as `bayan_mukawwinat.json` records it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MalafMustaqirr {
    /// Forward-slash path relative to the resource root.
    pub(crate) masar: String,
    /// Its length in bytes.
    pub(crate) hajm: u64,
    /// Its content hash, lowercase hex.
    pub(crate) sha256: String,
}

/// The manifest the studio verifies the bundled tree against.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BayanMukawwinat {
    /// The schema revision.
    pub(crate) mukhattat: u32,
    /// The workspace version this tree was staged from.
    pub(crate) isdar: String,
    /// The target triple this tree belongs to.
    pub(crate) hadaf: String,
    /// Every staged file, sorted by path.
    pub(crate) milaffat: Vec<MalafMustaqirr>,
}

/// Names at the staging root that clearing it must leave alone.
///
/// The default root is `apps/studio/src-tauri/mawarid`, whose `README.md` is a
/// tracked file rather than staging output: `tauri.conf.json` declares
/// `bundle.resources` against that directory and `tauri-build` checks the path
/// exists on every build, including a plain `cargo build` that will never
/// produce a bundle, so the committed file is what keeps the Studio compiling
/// from a clean checkout. Clearing the root wholesale deleted it on every run.
const MUBQA: &[&str] = &["README.md"];

/// The staging area: the resource root, and what has landed in it.
#[derive(Debug)]
pub(crate) struct Mustaqarr {
    jidhr: PathBuf,
    milaffat: Vec<MalafMustaqirr>,
    naqis: Vec<KhataTajmee>,
}

impl Mustaqarr {
    /// Opens a staging area, emptying whatever a previous run left.
    ///
    /// Everything the root holds goes except the names in [`MUBQA`]; the
    /// directory itself is never removed, so nothing tracked inside it and
    /// nothing watching it survives on the tool's good behaviour alone.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the root cannot be cleared or created.
    pub(crate) fn iftah(jidhr: &Path) -> NatijatTajmee<Self> {
        std::fs::create_dir_all(jidhr).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: jidhr.to_path_buf(),
            amal: "creating the staging tree",
            sabab,
        })?;
        farrigh(jidhr)?;
        Ok(Self {
            jidhr: jidhr.to_path_buf(),
            milaffat: Vec::new(),
            naqis: Vec::new(),
        })
    }

    /// Records a refusal and keeps going.
    ///
    /// Staging never stops at the first absence: an operator who has to rebuild
    /// four things learns that once, not four times.
    pub(crate) fn sajjil_naqs(&mut self, khata: KhataTajmee) {
        self.naqis.push(khata);
    }

    /// Every refusal collected so far.
    #[must_use]
    pub(crate) fn naqis(&self) -> &[KhataTajmee] {
        &self.naqis
    }

    /// Copies one file into the tree at `wajha`, hashing it as it goes.
    ///
    /// A source that is absent is recorded as a refusal rather than returned as
    /// an error, so one call site can stage a whole matrix row.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the destination cannot be written.
    pub(crate) fn insakh(
        &mut self,
        saf: &'static str,
        masdar: &Path,
        wajha: &str,
    ) -> NatijatTajmee<bool> {
        self.insakh_bi_shart(saf, masdar, wajha, false)
    }

    /// The same, refusing a payload that carries no `taarib_bidaya`.
    ///
    /// A payload built without the `hamula` feature is a well-formed shared
    /// library that a game loads and that then does nothing — the exact defect
    /// the bootstrap contract exists to close. It is caught here, on the build
    /// machine, rather than by a user whose install reported success.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the destination cannot be written.
    pub(crate) fn insakh_bi_shart(
        &mut self,
        saf: &'static str,
        masdar: &Path,
        wajha: &str,
        bi_bidaya: bool,
    ) -> NatijatTajmee<bool> {
        if !masdar.is_file() {
            self.sajjil_naqs(KhataTajmee::MukawwinGhaib {
                saf,
                masar: masdar.to_path_buf(),
            });
            return Ok(false);
        }
        let bayt = std::fs::read(masdar).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: masdar.to_path_buf(),
            amal: "reading a staged artifact",
            sabab,
        })?;
        if bi_bidaya && !yusaddir_bidaya(&bayt) {
            self.sajjil_naqs(KhataTajmee::BidayaGhaiba {
                saf,
                masar: masdar.to_path_buf(),
            });
            return Ok(false);
        }
        self.uktub(&bayt, wajha)?;
        Ok(true)
    }

    /// Places bytes into the tree at `wajha`, hashing them.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the destination cannot be written.
    pub(crate) fn uktub(&mut self, bayt: &[u8], wajha: &str) -> NatijatTajmee<()> {
        let hadaf = self.jidhr.join(wajha);
        if let Some(walid) = hadaf.parent() {
            std::fs::create_dir_all(walid).map_err(|sabab| KhataTajmee::KhataMalaf {
                masar: walid.to_path_buf(),
                amal: "creating a staging directory",
                sabab,
            })?;
        }
        std::fs::write(&hadaf, bayt).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: hadaf.clone(),
            amal: "writing a staged artifact",
            sabab,
        })?;
        self.milaffat.push(MalafMustaqirr {
            masar: wajha.to_owned(),
            hajm: bayt.len() as u64,
            sha256: hex_min_bayt(&Sha256::digest(bayt)),
        });
        Ok(())
    }

    /// Copies a whole directory into the tree, one file at a time.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the source cannot be walked or the
    /// destination cannot be written.
    pub(crate) fn insakh_mujallad(
        &mut self,
        saf: &'static str,
        masdar: &Path,
        wajha: &str,
    ) -> NatijatTajmee<bool> {
        if !masdar.is_dir() {
            self.sajjil_naqs(KhataTajmee::MukawwinGhaib {
                saf,
                masar: masdar.to_path_buf(),
            });
            return Ok(false);
        }
        let mut wajad = false;
        for madkhal in walk(masdar)? {
            let nisbi = madkhal.strip_prefix(masdar).unwrap_or(&madkhal);
            let dhayl = nisbi.to_string_lossy().replace('\\', "/");
            if self.insakh(saf, &madkhal, &format!("{wajha}/{dhayl}"))? {
                wajad = true;
            }
        }
        Ok(wajad)
    }

    /// Writes the manifest last, and only when nothing is missing.
    ///
    /// The name comes from `taarib_tathbeet::bayan_makhzan`, which is where the
    /// installer reads it from: written here under one spelling and read there
    /// under another, a rename would leave every install refusing every
    /// component with no way to see why.
    ///
    /// Its absence is what marks a partial staging, so it is never written over
    /// an incomplete tree.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the manifest cannot be written.
    pub(crate) fn akhtim(mut self, isdar: &str, hadaf: &str) -> NatijatTajmee<BayanMukawwinat> {
        self.milaffat
            .sort_by(|awwal, thani| awwal.masar.cmp(&thani.masar));
        let bayan = BayanMukawwinat {
            mukhattat: 1,
            isdar: isdar.to_owned(),
            hadaf: hadaf.to_owned(),
            milaffat: self.milaffat,
        };
        let nass = serde_json::to_string_pretty(&bayan).unwrap_or_default();
        let masar = self
            .jidhr
            .join(taarib_tathbeet::bayan_makhzan::ISM_MALAF_BAYAN);
        std::fs::write(&masar, nass).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar,
            amal: "writing the staging manifest",
            sabab,
        })?;
        Ok(bayan)
    }
}

/// Whether a built shared library exports `taarib_bidaya`.
///
/// A byte scan of the image rather than a format parse: the staging tool runs
/// on one host and stages payloads for three, so it cannot rely on being able
/// to parse a PE, an ELF and a Mach-O. An exported symbol's name is a
/// NUL-terminated string in the image's own string table under every one of
/// them, and its presence is exactly the question — a payload built without
/// the `hamula` feature contains the name nowhere.
#[must_use]
pub(crate) fn yusaddir_bidaya(bayt: &[u8]) -> bool {
    const ISM: &[u8] = b"taarib_bidaya";
    bayt.windows(ISM.len()).any(|nafidha| nafidha == ISM)
}

/// Empties a directory in place, keeping the names [`MUBQA`] lists.
///
/// A symlink is unlinked rather than descended into: `DirEntry::file_type` does
/// not follow one, so a link pointing outside the staging root is removed as the
/// single entry it is instead of taking its target's contents with it.
fn farrigh(jidhr: &Path) -> NatijatTajmee<()> {
    let madakhil = std::fs::read_dir(jidhr).map_err(|sabab| KhataTajmee::KhataMalaf {
        masar: jidhr.to_path_buf(),
        amal: "reading the previous staging tree",
        sabab,
    })?;
    for madkhal in madakhil {
        let madkhal = madkhal.map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: jidhr.to_path_buf(),
            amal: "reading the previous staging tree",
            sabab,
        })?;
        let ism = madkhal.file_name();
        if ism.to_str().is_some_and(|ism| MUBQA.contains(&ism)) {
            continue;
        }
        let masar = madkhal.path();
        let naw = madkhal
            .file_type()
            .map_err(|sabab| KhataTajmee::KhataMalaf {
                masar: masar.clone(),
                amal: "reading the previous staging tree",
                sabab,
            })?;
        // `masar` is one direct child of the staging tree, taken from `read_dir`
        // on a path this build tool was given as `--kharij`. It is never derived
        // from a data root: `taarib-tajmee` is a build-machine tool and has no
        // `Masarat` to ask.
        #[expect(
            clippy::disallowed_methods,
            reason = "a direct child of the build tool's own staging tree, which is a CLI argument"
        )]
        let natija = if naw.is_dir() {
            std::fs::remove_dir_all(&masar)
        } else {
            std::fs::remove_file(&masar)
        };
        natija.map_err(|sabab| KhataTajmee::KhataMalaf {
            masar,
            amal: "clearing the previous staging tree",
            sabab,
        })?;
    }
    Ok(())
}

/// Every regular file under a directory, depth first.
fn walk(jidhr: &Path) -> NatijatTajmee<Vec<PathBuf>> {
    let mut khraj = Vec::new();
    let mut tabur = vec![jidhr.to_path_buf()];
    while let Some(hali) = tabur.pop() {
        let madakhil = std::fs::read_dir(&hali).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: hali.clone(),
            amal: "walking a source directory",
            sabab,
        })?;
        for madkhal in madakhil {
            let madkhal = madkhal.map_err(|sabab| KhataTajmee::KhataMalaf {
                masar: hali.clone(),
                amal: "walking a source directory",
                sabab,
            })?;
            let masar = madkhal.path();
            if masar.is_dir() {
                tabur.push(masar);
            } else if masar.is_file() {
                khraj.push(masar);
            }
        }
    }
    khraj.sort();
    Ok(khraj)
}

/// Lowercase hex of a digest.
pub(crate) fn hex_min_bayt(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter()
        .fold(String::with_capacity(bayt.len() * 2), |mut nass, bayta| {
            let _ = write!(nass, "{bayta:02x}");
            nass
        })
}
