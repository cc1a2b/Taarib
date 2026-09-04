//! Sandbox verification: install, launch, uninstall a submission in isolation.

use std::path::{Path, PathBuf};

use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_tathbeet::taraju::{RadLaShay, SiyasatIstiada, istiada_kul};

use crate::hawiya::SalahiyatMalik;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};

/// What the sandbox run established.
///
/// Never a bare bool: a run that was not attempted, one the safety layer
/// refused, and one that installed and restored cleanly are three different
/// pieces of evidence, and collapsing them would let "not tested" read as
/// "tested and fine".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "natija", rename_all = "snake_case")]
pub enum HasilatSandooq {
    /// The safety layer refused the package; nothing was written.
    RafadaAlaman {
        /// The refusal, in the reviewer's language.
        sabab: String,
    },
    /// Installation failed.
    FashilaTathbeet {
        /// What the installer said.
        sabab: String,
    },
    /// It installed, and the uninstall did not put everything back.
    IstiadaNaqisa {
        /// How many paths were restored.
        mustaada: usize,
        /// What is still modified.
        mutabaqqi: Vec<String>,
    },
    /// It installed and restored cleanly.
    Salima {
        /// How many paths were written.
        maktuba: usize,
        /// How many were restored and verified.
        mustaada: usize,
    },
}

impl HasilatSandooq {
    /// Whether the submission passed sandbox verification.
    #[must_use]
    pub const fn najahat(&self) -> bool {
        matches!(self, Self::Salima { .. })
    }

    /// The line the review console shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::RafadaAlaman { sabab } => format!("safety refused: {sabab}"),
            Self::FashilaTathbeet { sabab } => format!("install failed: {sabab}"),
            Self::IstiadaNaqisa { mustaada, mutabaqqi } => format!(
                "installed, {mustaada} path(s) restored, {} still modified",
                mutabaqqi.len()
            ),
            Self::Salima { maktuba, mustaada } => {
                format!("installed {maktuba} path(s) and restored {mustaada} cleanly")
            }
        }
    }
}

/// Whether the game was launched, and what happened.
///
/// A launch that was not attempted is recorded as such: it is not a launch that
/// succeeded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "halat", rename_all = "snake_case")]
pub enum HalatItlaq {
    /// The reviewer did not launch it.
    LamYujarrab,
    /// It started and was closed by the reviewer.
    Ishtaghala,
    /// It did not start.
    LamYashtaghil {
        /// What was observed.
        sabab: String,
    },
}

/// One sandbox run, recorded onto the submission as evidence.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerSandooq {
    /// The patch verified.
    pub ruqaa: RuqaaId,
    /// Which revision.
    pub murajaa: RuqaaRevision,
    /// The install-and-restore outcome.
    pub hasila: HasilatSandooq,
    /// Whether the game was launched.
    pub itlaq: HalatItlaq,
    /// When the run finished, RFC 3339, supplied by the caller.
    pub waqt: String,
}

/// The isolated workspace one verification runs in.
///
/// Rooted under Taarib's own sandbox directory; the game copy lives inside it,
/// so a run never touches the reviewer's real library.
#[derive(Debug)]
pub struct BeeatSandooq {
    jidhr: PathBuf,
    jidhr_luba: PathBuf,
    jidhr_nusakh: PathBuf,
}

impl BeeatSandooq {
    /// Prepares a sandbox under `jidhr_sandooq` for one submission.
    ///
    /// `jidhr_sandooq` is [`taarib_usus::masarat::Masarat::sandooq`].
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::SandooqFashil`] when the directories cannot be created,
    /// and when `jidhr_sandooq` is not an absolute path — a relative sandbox
    /// root would resolve against whatever directory the process happens to be
    /// in.
    pub fn hayyi(
        _salahiya: &SalahiyatMalik,
        jidhr_sandooq: &Path,
        ruqaa: RuqaaId,
    ) -> NatijatTaqdeem<Self> {
        if !jidhr_sandooq.is_absolute() {
            return Err(KhataTaqdeem::SandooqFashil {
                sabab: "the sandbox root must be an absolute path".to_owned(),
            });
        }
        let jidhr = jidhr_sandooq.join(ruqaa.to_string());
        let jidhr_luba = jidhr.join("luba");
        let jidhr_nusakh = jidhr.join("nusakh");
        for masar in [&jidhr, &jidhr_luba, &jidhr_nusakh] {
            std::fs::create_dir_all(masar).map_err(|sabab| KhataTaqdeem::SandooqFashil {
                sabab: format!("{} could not be created: {sabab}", masar.display()),
            })?;
        }
        Ok(Self { jidhr, jidhr_luba, jidhr_nusakh })
    }

    /// The isolated copy of the game.
    #[must_use]
    pub fn jidhr_luba(&self) -> &Path {
        &self.jidhr_luba
    }

    /// The backup directory this run installs against.
    #[must_use]
    pub fn jidhr_nusakh(&self) -> &Path {
        &self.jidhr_nusakh
    }

    /// Restores the sandboxed game and reports what came back.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::SandooqFashil`] when the restore itself could not run.
    pub fn istaid(&self) -> NatijatTaqdeem<(usize, Vec<String>)> {
        let mut radd = RadLaShay;
        let taqreer =
            istiada_kul(&self.jidhr_luba, &self.jidhr_nusakh, SiyasatIstiada::Sarima, &mut radd);
        let mut mustaada = 0_usize;
        let mut mutabaqqi = Vec::new();
        for wahid in [taqreer.nass, taqreer.sawt].into_iter().flatten() {
            match wahid {
                Ok(natija) => {
                    mustaada = mustaada.saturating_add(natija.mustaada);
                    mutabaqqi.extend(natija.mustabdala.iter().cloned());
                }
                Err(khata) => {
                    use taarib_usus::khata::Tafsir as _;
                    return Err(KhataTaqdeem::SandooqFashil { sabab: khata.injilizi() });
                }
            }
        }
        Ok((mustaada, mutabaqqi))
    }

    /// Removes the sandbox entirely.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::SandooqFashil`] when the tree cannot be removed.
    pub fn imsah(self) -> NatijatTaqdeem<()> {
        std::fs::remove_dir_all(&self.jidhr).map_err(|sabab| KhataTaqdeem::SandooqFashil {
            sabab: format!("{} could not be removed: {sabab}", self.jidhr.display()),
        })
    }
}
