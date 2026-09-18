//! Copying, hashing, the staged manifest, and the catalogue beside it.

use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};
use taarib_tathbeet::bayan_makhzan::{
    BADIYAT_MAKHZAN, BasmatFihris, BayanMukawwinat, FihrisMukawwinat, ISM_MALAF_FIHRIS,
    MUKHATTAT_FIHRIS_MADUM, MalafMudraj, MukawwinMufahras, TAQM_KAMIL, TAQM_NAHIF,
};

use crate::khata::{KhataTajmee, NatijatTajmee};

/// Which components of the matrix a bundle carries.
///
/// The axis is measured, not guessed. The twelve BepInEx components are
/// 475,472,457 bytes of a 519,747,711-byte component tree, and the six IL2CPP
/// ones are 449,711,337 of those — nine tenths of everything the bundle weighs,
/// because BepInEx's IL2CPP host ships a whole .NET 6 runtime beside itself and
/// Unity's Mono host does not. So the line the two sets differ by is exactly
/// that one: a machine with no IL2CPP game downloads none of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Taqm {
    /// Every component of the matrix. What every build before this flag
    /// existed produced, and what the offline bundle still is.
    Kamil,
    /// Everything but the IL2CPP BepInEx builds.
    Nahif,
}

impl Taqm {
    /// The set a `--taqm` argument names.
    pub(crate) fn min_ism(ism: &str) -> Option<Self> {
        match ism {
            TAQM_KAMIL => Some(Self::Kamil),
            TAQM_NAHIF => Some(Self::Nahif),
            _ => None,
        }
    }

    /// The name this set is recorded under in the manifest.
    pub(crate) const fn ism(self) -> &'static str {
        match self {
            Self::Kamil => TAQM_KAMIL,
            Self::Nahif => TAQM_NAHIF,
        }
    }

    /// Whether a bundle staged with this set carries one component's bytes.
    ///
    /// Matched on the segment rather than on a substring: `il2cpp` appears
    /// nowhere else in a component name today, and a rule that would start
    /// matching a component named after it later is a rule that silently drops
    /// a component from the bundle.
    pub(crate) fn yahmil(self, ism: &str) -> bool {
        match self {
            Self::Kamil => true,
            Self::Nahif => !ism
                .rsplit('/')
                .next()
                .is_some_and(|akhir| akhir.starts_with("il2cpp-")),
        }
    }
}

/// One component as the catalogue records it while staging fills it.
#[derive(Debug, Clone)]
struct MukawwinMustaqirr {
    ism: String,
    fi_alhuzma: bool,
    milaffat: Vec<MalafMudraj>,
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

/// The subtree of the distribution directory the component objects land in.
const BADIYAT_TAWZEE: &str = "mukawwinat";

/// The staging area: the resource root, and what has landed in it.
#[derive(Debug)]
pub(crate) struct Mustaqarr {
    jidhr: PathBuf,
    taqm: Taqm,
    tawzee: Option<PathBuf>,
    mukawwin: Option<String>,
    milaffat: Vec<MalafMudraj>,
    fihris: Vec<MukawwinMustaqirr>,
    naqis: Vec<KhataTajmee>,
}

impl Mustaqarr {
    /// Opens a staging area, emptying whatever a previous run left.
    ///
    /// Everything the root holds goes except the names in [`MUBQA`]; the
    /// directory itself is never removed, so nothing tracked inside it and
    /// nothing watching it survives on the tool's good behaviour alone.
    ///
    /// `tawzee` is where the release's content-addressed component objects are
    /// written, when one was asked for. Every component's bytes go there
    /// whether or not the bundle carries them, so one object store serves both
    /// variants and every platform: the three Unity generations of one backend
    /// and architecture are byte-identical trees, and an object named by its
    /// own hash is stored once for all three.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when the root cannot be cleared or created.
    pub(crate) fn iftah(jidhr: &Path, taqm: Taqm, tawzee: Option<&Path>) -> NatijatTajmee<Self> {
        std::fs::create_dir_all(jidhr).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: jidhr.to_path_buf(),
            amal: "creating the staging tree",
            sabab,
        })?;
        farrigh(jidhr)?;
        if let Some(tawzee) = tawzee {
            std::fs::create_dir_all(tawzee.join(BADIYAT_TAWZEE)).map_err(|sabab| {
                KhataTajmee::KhataMalaf {
                    masar: tawzee.to_path_buf(),
                    amal: "creating the distribution object store",
                    sabab,
                }
            })?;
        }
        Ok(Self {
            jidhr: jidhr.to_path_buf(),
            taqm,
            tawzee: tawzee.map(Path::to_path_buf),
            mukawwin: None,
            milaffat: Vec::new(),
            fihris: Vec::new(),
            naqis: Vec::new(),
        })
    }

    /// Names the component every following write belongs to, until the next
    /// call.
    ///
    /// Passed in rather than parsed back out of each destination path: the
    /// component boundary is a fact the matrix knows and a path only implies,
    /// and a parser that got it wrong would put a file in the wrong catalogue
    /// entry with nothing to notice. [`None`] is for what is not a component at
    /// all — the fonts, the notices — which ride in every bundle and are
    /// catalogued nowhere.
    pub(crate) fn fi_mukawwin(&mut self, ism: Option<&str>) {
        self.mukawwin = ism.map(str::to_owned);
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
    /// Three destinations, decided here and nowhere else:
    ///
    /// - the bundle, when this component is one the set carries — or when the
    ///   bytes are not a component's at all;
    /// - the catalogue, for every component's file, carried or not, so that
    ///   both variants ship the same description of the whole release;
    /// - the distribution object store, under the file's own hash, when one was
    ///   asked for.
    ///
    /// A component the bundle does not carry is still read, hashed and
    /// refused-if-absent exactly as a carried one: the slim bundle's catalogue
    /// is only trustworthy because the build machine held the real bytes.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when a destination cannot be written.
    pub(crate) fn uktub(&mut self, bayt: &[u8], wajha: &str) -> NatijatTajmee<()> {
        let basma = hex_min_bayt(&Sha256::digest(bayt));
        let madkhal = MalafMudraj {
            masar: wajha.to_owned(),
            hajm: bayt.len() as u64,
            sha256: basma.clone(),
        };

        match self.mukawwin.clone() {
            None => {
                self.uktub_fi_alhuzma(bayt, wajha)?;
                self.milaffat.push(madkhal);
            },
            Some(ism) => {
                let fi_alhuzma = self.taqm.yahmil(&ism);
                if fi_alhuzma {
                    self.uktub_fi_alhuzma(bayt, wajha)?;
                    self.milaffat.push(madkhal.clone());
                }
                self.sajjil_fi_fihris(&ism, fi_alhuzma, madkhal, wajha);
                self.uktub_kaghrad(bayt, &basma)?;
            },
        }
        Ok(())
    }

    /// Writes one file into the bundle's resource tree.
    fn uktub_fi_alhuzma(&self, bayt: &[u8], wajha: &str) -> NatijatTajmee<()> {
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
        })
    }

    /// Records one file against its component in the catalogue, under a path
    /// relative to the component root rather than to the bundle.
    fn sajjil_fi_fihris(
        &mut self,
        ism: &str,
        fi_alhuzma: bool,
        mut madkhal: MalafMudraj,
        wajha: &str,
    ) {
        let badiya = format!("{BADIYAT_MAKHZAN}{ism}/");
        wajha
            .strip_prefix(&badiya)
            .unwrap_or(wajha)
            .clone_into(&mut madkhal.masar);
        if let Some(mawjud) = self.fihris.iter_mut().find(|mukawwin| mukawwin.ism == ism) {
            mawjud.milaffat.push(madkhal);
            return;
        }
        self.fihris.push(MukawwinMustaqirr {
            ism: ism.to_owned(),
            fi_alhuzma,
            milaffat: vec![madkhal],
        });
    }

    /// Writes one file into the distribution object store under its own hash.
    ///
    /// An object already there is left alone rather than rewritten: the path is
    /// the hash, so a second write of the same name is a second copy of bytes
    /// that are already proven identical. That is what collapses the three
    /// byte-identical Unity generations of each backend into one object.
    fn uktub_kaghrad(&self, bayt: &[u8], basma: &str) -> NatijatTajmee<()> {
        let Some(tawzee) = self.tawzee.as_ref() else {
            return Ok(());
        };
        let Some(bad) = basma.get(..2) else {
            return Ok(());
        };
        let mujallad = tawzee.join(BADIYAT_TAWZEE).join(bad);
        let hadaf = mujallad.join(basma);
        if hadaf.is_file() {
            return Ok(());
        }
        std::fs::create_dir_all(&mujallad).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: mujallad.clone(),
            amal: "creating an object directory",
            sabab,
        })?;
        // Written beside the object and renamed onto it, so an interrupted run
        // never leaves a file whose name asserts a hash its contents do not
        // have — the same rule `qufl::ijlib` follows for a fetched download,
        // and for the same reason: a later run trusts the name.
        let muaqqat = hadaf.with_extension(format!("juzii.{}", std::process::id()));
        std::fs::write(&muaqqat, bayt).map_err(|sabab| KhataTajmee::KhataMalaf {
            masar: muaqqat.clone(),
            amal: "writing a distribution object",
            sabab,
        })?;
        std::fs::rename(&muaqqat, &hadaf).map_err(|sabab| {
            let _ = std::fs::remove_file(&muaqqat);
            KhataTajmee::KhataMalaf {
                masar: hadaf,
                amal: "moving a distribution object into place",
                sabab,
            }
        })
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

    /// Writes the catalogue, then the manifest, and only when nothing is
    /// missing.
    ///
    /// The order is the contract. `docs/tawzee.md` §4 makes the manifest's own
    /// absence the marker for a partial staging, so it is written strictly last
    /// — and the manifest is what vouches for the catalogue, so the catalogue
    /// has to exist and be hashed before the manifest can name it. A catalogue
    /// written after would be one the manifest could not describe, and a
    /// catalogue nothing vouches for is a document a fetch must not trust.
    ///
    /// The names come from `taarib_tathbeet::bayan_makhzan`, which is where the
    /// installer reads both from: written here under one spelling and read
    /// there under another, a rename would leave every install refusing every
    /// component with no way to see why.
    ///
    /// # Errors
    ///
    /// [`KhataTajmee::KhataMalaf`] when either document cannot be written.
    pub(crate) fn akhtim(mut self, isdar: &str, hadaf: &str) -> NatijatTajmee<BayanMukawwinat> {
        let basma = self.uktub_fihris(isdar, hadaf)?;

        self.milaffat
            .sort_by(|awwal, thani| awwal.masar.cmp(&thani.masar));
        let bayan = BayanMukawwinat {
            mukhattat: taarib_tathbeet::bayan_makhzan::MUKHATTAT_MADUM,
            isdar: isdar.to_owned(),
            hadaf: hadaf.to_owned(),
            milaffat: self.milaffat,
            taqm: self.taqm.ism().to_owned(),
            fihris: Some(basma),
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

    /// Writes the catalogue into the bundle, and beside the objects when a
    /// distribution directory was asked for, returning its fingerprint.
    ///
    /// Both variants carry the same description of the release; they differ
    /// only in each entry's `fi_alhuzma` and in which bytes are present. That
    /// is deliberate — it is what lets the slim bundle name a component it does
    /// not hold, state its size, and verify it if it is ever fetched.
    fn uktub_fihris(&mut self, isdar: &str, hadaf: &str) -> NatijatTajmee<BasmatFihris> {
        for mukawwin in &mut self.fihris {
            mukawwin
                .milaffat
                .sort_by(|awwal, thani| awwal.masar.cmp(&thani.masar));
        }
        self.fihris
            .sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));

        let fihris = FihrisMukawwinat {
            mukhattat: MUKHATTAT_FIHRIS_MADUM,
            isdar: isdar.to_owned(),
            hadaf: hadaf.to_owned(),
            mukawwinat: self
                .fihris
                .iter()
                .map(|mukawwin| MukawwinMufahras {
                    ism: mukawwin.ism.clone(),
                    fi_alhuzma: mukawwin.fi_alhuzma,
                    hajm: mukawwin
                        .milaffat
                        .iter()
                        .fold(0_u64, |majmu, malaf| majmu.saturating_add(malaf.hajm)),
                    milaffat: mukawwin.milaffat.clone(),
                })
                .collect(),
        };

        let nass = serde_json::to_vec_pretty(&fihris).unwrap_or_default();
        for jidhr in [Some(self.jidhr.as_path()), self.tawzee.as_deref()]
            .into_iter()
            .flatten()
        {
            let masar = jidhr.join(ISM_MALAF_FIHRIS);
            std::fs::write(&masar, &nass).map_err(|sabab| KhataTajmee::KhataMalaf {
                masar,
                amal: "writing the component catalogue",
                sabab,
            })?;
        }

        Ok(BasmatFihris {
            hajm: nass.len() as u64,
            sha256: hex_min_bayt(&Sha256::digest(&nass)),
        })
    }

    /// What the staging summary prints: the two sets of numbers an operator has
    /// to be able to tell apart.
    #[must_use]
    pub(crate) fn hisab(&self) -> HisabTajmee {
        let mut hisab = HisabTajmee {
            fi_alhuzma: self.milaffat.len(),
            hajm_alhuzma: self
                .milaffat
                .iter()
                .fold(0_u64, |majmu, malaf| majmu.saturating_add(malaf.hajm)),
            mukawwinat: self.fihris.len(),
            kharij: 0,
            hajm_kharij: 0,
        };
        for mukawwin in &self.fihris {
            if mukawwin.fi_alhuzma {
                continue;
            }
            hisab.kharij = hisab.kharij.saturating_add(1);
            hisab.hajm_kharij = mukawwin
                .milaffat
                .iter()
                .fold(hisab.hajm_kharij, |majmu, malaf| {
                    majmu.saturating_add(malaf.hajm)
                });
        }
        hisab
    }
}

/// What one staging run produced, in whole numbers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HisabTajmee {
    /// Files written into the bundle.
    pub(crate) fi_alhuzma: usize,
    /// Their total size in bytes.
    pub(crate) hajm_alhuzma: u64,
    /// Components the catalogue describes.
    pub(crate) mukawwinat: usize,
    /// Components the catalogue describes that this bundle does not carry.
    pub(crate) kharij: usize,
    /// Their total size in bytes — what a machine that needs all of them would
    /// fetch.
    pub(crate) hajm_kharij: u64,
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
