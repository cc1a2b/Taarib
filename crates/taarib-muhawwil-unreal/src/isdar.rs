//! إصدار — which Unreal this is, and what that means for what can be done to it.
//!
//! Two questions, and they are not the same question.
//!
//! **Which version.** Unreal 4.20 and 5.4 are one product line and two very
//! different text stacks. The container formats changed, Slate's flow-direction
//! resolution changed, the culture fallback chain changed, and the console
//! variable that forces full shaping did not exist at all in the earliest
//! versions this adapter meets. A correction applied to the wrong version is
//! worse than no correction.
//!
//! **Which flavour.** A shipping monolithic build has one executable with
//! almost nothing exported; a modular build has `UnrealEditor-Slate.dll` beside
//! it with a great deal exported. That difference decides whether the runtime
//! half of this adapter can reach Slate at all — and it is a property of how the
//! game was packaged, not of its version.
//!
//! ## Where the answer comes from, and why not from the engine
//!
//! From the file system, and in this order:
//!
//! 1. The `.pak` footer's format version, or the `.utoc` header's, which are
//!    written into files that ship with every game and are readable with the
//!    game not running.
//! 2. The build's own directory shape — `Engine/Binaries/`, the `_Data`-less
//!    layout, the presence of per-module DLLs.
//! 3. The executable's version resource, which Unreal stamps for most build
//!    configurations and not all.
//!
//! Notably **not** by asking the engine. The offline half of this adapter runs
//! with the game shut down, and the runtime half needs the version *before* it
//! decides which corrections to install — so a version obtained by calling into
//! the process would arrive after the point it was needed.
//!
//! ## One table, not two
//!
//! The mapping from a container format version to an engine version range was
//! established in Phase 5 by [`taarib_muharrik::dalail::unreal`], which reads
//! exactly these footers to identify a game in the first place. This module
//! calls into it rather than restating it. Two tables saying which Unreal
//! version a pak version 8 with four compression slots implies would eventually
//! disagree, and the one consulted less often would be the wrong one.
//!
//! What this module adds on top is the *flavour*, which detection never needed
//! and the adapter cannot work without.
//!
//! ## Directory names are asked for, not asserted
//!
//! Every directory this module reaches for under a game root goes through
//! [`masar_bila_hala`], which is this crate's one case-folding resolver. Unreal's
//! own convention is `Content/Paks`, but a Windows game under Wine or Proton sees
//! a case-insensitive filesystem, so a depot that ships `content/paks` runs
//! perfectly for the player and is invisible to a literal `Path::join` on Linux.
//! Missing it here does not produce an error: it produces "no containers found",
//! which the caller reads as "this game has nothing to translate".

use std::path::{Path, PathBuf};

use taarib_muharrik::dalail::unreal::{DhaylPak, MadaIsdar, TarwisatUtoc};
use taarib_mustalahat::muharrik::IsdarMuharrik;

use crate::khata::KhataUnreal;

/// How the game was packaged, which decides what the runtime half can reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Naw {
    /// One executable with the whole engine linked in.
    ///
    /// The shipping configuration, and what nearly every released game is.
    /// Almost nothing is exported, so Slate has to be reached through whatever
    /// the primary module does export, and a build that exports nothing usable
    /// degrades to the offline half of the adapter.
    Wahid,

    /// The engine split across per-module dynamic libraries.
    ///
    /// A development or editor build. `Slate`, `SlateCore` and `CoreUObject` are
    /// separate modules with real export tables, so the runtime half has an
    /// ordinary symbol lookup available to it. Rare in the wild and worth
    /// handling, because it is what a translator testing a patch will often
    /// have in front of them.
    Munfasil,

    /// Neither shape could be established.
    Majhul,
}

impl Naw {
    /// Whether Slate is reachable by ordinary symbol lookup in this build.
    #[must_use]
    pub const fn yusaddir(self) -> bool {
        matches!(self, Self::Munfasil)
    }

    /// The name for a log line and the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Wahid => "monolithic",
            Self::Munfasil => "modular",
            Self::Majhul => "unknown",
        }
    }
}

/// How the game's content is packaged, which decides which reader runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tabaa {
    /// Loose files under `Content/`, with no container at all.
    ///
    /// Development builds and a surprising number of small released games.
    /// The easiest case by a wide margin: a `.locres` is a file on disk.
    Sayib,

    /// One or more `.pak` containers.
    Pak,

    /// UE5 IoStore: `.utoc` and `.ucas` beside each other.
    ///
    /// Often alongside `.pak` files rather than instead of them, because UE5
    /// ships both and mounts both. That is what makes the additive patch pak
    /// still work on an IoStore title.
    Iostore,

    /// Nothing recognisable was found.
    Majhul,
}

impl Tabaa {
    /// The name for a log line and the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Sayib => "loose",
            Self::Pak => "pak",
            Self::Iostore => "iostore",
            Self::Majhul => "unknown",
        }
    }
}

/// Everything this adapter needs to know about the build before it acts.
#[derive(Debug, Clone)]
pub struct Bina {
    /// The game's root directory.
    pub jidhr: PathBuf,
    /// The engine version, when one could be established.
    pub isdar: Option<IsdarMuharrik>,
    /// The range the container formats imply, which is always available when a
    /// container was found even if an exact version was not.
    pub mada: Option<MadaIsdar>,
    /// How the engine was linked.
    pub naw: Naw,
    /// How the content is packaged.
    pub tabaa: Tabaa,
    /// Whether any container found is encrypted.
    pub mushaffar: bool,
    /// Where the containers are, in the order they were found.
    pub hawiyat: Vec<PathBuf>,
    /// What each step reported, for the diagnostics bundle.
    pub athar: Vec<String>,
}

impl Bina {
    /// Whether the engine is Unreal 5 or later.
    ///
    /// The line that matters most: UE5 changed the container format, the
    /// default shaping path's console variable name, and the culture fallback
    /// chain. When the version is only known as a range that straddles the
    /// boundary, this answers `false` — because applying a UE5-only correction
    /// to a UE4 game is the failure this is guarding against, and the reverse
    /// merely leaves a correction unapplied.
    #[must_use]
    pub fn khamsa(&self) -> bool {
        if let Some(isdar) = &self.isdar {
            return isdar.kabir >= 5;
        }
        self.mada.is_some_and(|mada| mada.adna.0 >= 5)
    }

    /// Whether the runtime half can expect to find Slate by symbol lookup.
    #[must_use]
    pub const fn yasil_slate(&self) -> bool {
        self.naw.yusaddir()
    }

    /// The version as a sentence, for a log line.
    #[must_use]
    pub fn wasf(&self) -> String {
        let isdar = match (&self.isdar, self.mada) {
            (Some(isdar), _) => isdar.khaam.clone(),
            (None, Some(mada)) => format!(
                "UE {}.{}-{}.{} (from container format)",
                mada.adna.0, mada.adna.1, mada.aqsa.0, mada.aqsa.1
            ),
            (None, None) => "UE version unknown".to_owned(),
        };
        format!(
            "{isdar}, {} build, {} content{}",
            self.naw.ism(),
            self.tabaa.ism(),
            if self.mushaffar { ", encrypted" } else { "" }
        )
    }
}

/// The most containers to look at before concluding.
///
/// Sixteen. A shipped game has one to a handful of `.pak` files and one or two
/// IoStore containers; a game with more than sixteen is a game whose packaging
/// this adapter has not met, and walking all of them to answer a question the
/// first one already answered would be a startup cost paid for nothing.
pub const AQSA_HAWIYAT: usize = 16;

/// Establishes what kind of Unreal build this is.
///
/// Never fails on a directory that exists: an unrecognised build resolves to
/// [`Naw::Majhul`] and [`Tabaa::Majhul`], which the caller handles by running
/// only the parts of the adapter that need neither. Refusing here would deny a
/// translation to a game whose packaging is merely unfamiliar.
///
/// # Errors
///
/// [`KhataUnreal::KhataMalaf`] only when the root itself cannot be listed.
pub fn afhas(jidhr: &Path) -> Result<Bina, KhataUnreal> {
    let mut athar = Vec::new();
    let mut hawiyat = Vec::new();
    let mut mushaffar = false;
    let mut mada: Option<MadaIsdar> = None;
    let mut isdar: Option<IsdarMuharrik> = None;

    let naw = naw_bina(jidhr, &mut athar);
    let dalil_muhtawa = jid_dalil_pak(jidhr, &mut athar);

    let tabaa = match &dalil_muhtawa {
        Some(dalil) => {
            ijma_hawiyat(dalil, &mut hawiyat)?;
            let mut ra_pak = false;
            let mut ra_utoc = false;

            for masar in hawiyat.iter().take(AQSA_HAWIYAT) {
                let imtidad = masar
                    .extension()
                    .and_then(|imtidad| imtidad.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();

                if imtidad == "pak" {
                    if let Some(dhayl) = DhaylPak::iqra(masar) {
                        ra_pak = true;
                        mushaffar = mushaffar || dhayl.yahtaj_miftah();
                        let hadha = dhayl.mada();
                        mada = Some(mada.map_or(hadha, |sabiq| adyaq(sabiq, hadha)));
                        if isdar.is_none() {
                            isdar = hadha.isdar("pak footer");
                        }
                        athar.push(format!(
                            "{}: pak format version {}, {}",
                            ism_malaf(masar),
                            dhayl.nuskha,
                            if dhayl.yahtaj_miftah() {
                                "encrypted"
                            } else {
                                "plain"
                            }
                        ));
                    }
                } else if imtidad == "utoc"
                    && let Some(tarwisa) = TarwisatUtoc::iqra(masar)
                {
                    ra_utoc = true;
                    mushaffar = mushaffar || tarwisa.mushaffara();
                    let hadha = tarwisa.mada();
                    mada = Some(mada.map_or(hadha, |sabiq| adyaq(sabiq, hadha)));
                    if isdar.is_none() {
                        isdar = hadha.isdar("utoc header");
                    }
                    athar.push(format!(
                        "{}: IoStore ToC version {}, {}",
                        ism_malaf(masar),
                        tarwisa.nuskha,
                        if tarwisa.mushaffara() {
                            "encrypted"
                        } else {
                            "plain"
                        }
                    ));
                }
            }

            // IoStore wins when both are present, because a UE5 title ships both
            // and the interesting content is in the newer one. The pak files
            // beside it are usually the small mount-order shims, and reporting
            // the build as pak-packaged would send the extractor to the wrong
            // container.
            match (ra_utoc, ra_pak) {
                (true, _) => Tabaa::Iostore,
                (false, true) => Tabaa::Pak,
                (false, false) => Tabaa::Sayib,
            }
        },
        None => Tabaa::Majhul,
    };

    if mada.is_none() {
        athar.push(
            "no readable container footer was found, so the engine version is only as \
             precise as the build layout allows"
                .to_owned(),
        );
    }

    Ok(Bina {
        jidhr: jidhr.to_path_buf(),
        isdar,
        mada,
        naw,
        tabaa,
        mushaffar,
        hawiyat,
        athar,
    })
}

/// How many entries one directory listing in this module will look at.
///
/// A game root is somebody else's data, and an unbounded read of one is a
/// promise about memory this code cannot keep. Four thousand is the same ceiling
/// every other directory walk in this product uses. Both walks here take it:
/// [`ibn_bila_hala`]'s case-folding fallback and [`jid_dalil_pak`]'s scan for the
/// project directory.
const AQSA_MUTABAQA: usize = 4_096;

/// Resolves one child name against a directory, ignoring letter case.
///
/// The exact spelling is tried first, so a depot that matches Unreal's own
/// convention costs no directory read at all. The listing behind the fallback is
/// bounded by [`AQSA_MUTABAQA`].
fn ibn_bila_hala(jidhr: &Path, ism: &str) -> Option<PathBuf> {
    let mubashir = jidhr.join(ism);
    if mubashir.exists() {
        return Some(mubashir);
    }
    std::fs::read_dir(jidhr)
        .ok()?
        .take(AQSA_MUTABAQA)
        .flatten()
        .find(|madkhal| {
            madkhal
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(ism)
        })
        .map(|madkhal| madkhal.path())
}

/// Resolves a whole relative path against a directory, ignoring letter case.
///
/// [`ibn_bila_hala`] one component at a time, with the rule that makes one
/// resolver serve reading and writing both: **once a component is missing, every
/// component after it is passed through verbatim** rather than searched for. So
/// `Content/Paks` under a depot spelling it `content/` finds the real directory,
/// and the same call under a game whose `Content/` has no `Paks/` yet returns
/// the path to create — inside the game's own `content/`, not beside it.
///
/// That second half is what keeps [`crate::mawarid::pak::KatibPak::uktub_fi_luba`]
/// honest. A patch container written into a freshly created `Content/Paks` next
/// to an existing `content/paks` is a container the engine never mounts, and an
/// install that reports success having changed nothing the player sees.
pub(crate) fn masar_bila_hala(jidhr: &Path, nisbi: &str) -> PathBuf {
    let mut mabni = jidhr.to_path_buf();
    let mut mafqud = false;
    for juz in nisbi
        .split(['/', '\\'])
        .filter(|juz| !juz.is_empty() && *juz != ".")
    {
        if mafqud {
            mabni.push(juz);
            continue;
        }
        if let Some(mawjud) = ibn_bila_hala(&mabni, juz) {
            mabni = mawjud;
        } else {
            mafqud = true;
            mabni.push(juz);
        }
    }
    mabni
}

/// Whether the engine was linked as one binary or as per-module libraries.
///
/// Decided by looking for the module libraries themselves rather than for the
/// executable: a monolithic build and a modular one both have an executable in
/// `Binaries/`, and only the modular one has `*-Slate.*` beside it.
fn naw_bina(jidhr: &Path, athar: &mut Vec<String>) -> Naw {
    const ADILLA: [&str; 4] = ["Slate", "SlateCore", "CoreUObject", "Engine"];
    const IMTIDADAT: [&str; 3] = ["dll", "so", "dylib"];

    let binaries = masar_bila_hala(jidhr, "Engine/Binaries");
    for manassa in ["Win64", "WinGDK", "Linux", "Mac"] {
        let Some(dalil) = ibn_bila_hala(&binaries, manassa) else {
            continue;
        };
        let Ok(madkhalat) = std::fs::read_dir(&dalil) else {
            continue;
        };

        for madkhal in madkhalat.flatten() {
            let ism = madkhal.file_name();
            let Some(ism) = ism.to_str() else { continue };
            let imtidad = Path::new(ism)
                .extension()
                .and_then(|imtidad| imtidad.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ADILLA.iter().any(|dalil| ism.contains(dalil))
                && IMTIDADAT.contains(&imtidad.as_str())
            {
                athar.push(format!("modular build: found {ism} in {manassa}"));
                return Naw::Munfasil;
            }
        }
    }

    // A build with an Engine/Binaries directory and no module libraries in it is
    // a monolithic build; one with no such directory at all is a packaged game
    // whose engine directory was stripped, which is also monolithic.
    if binaries.is_dir() {
        athar.push("monolithic build: Engine/Binaries has no per-module libraries".to_owned());
    } else {
        athar.push("monolithic build: no Engine/Binaries directory".to_owned());
    }
    Naw::Wahid
}

/// Finds the directory holding the game's containers.
///
/// Unreal's layout is `<Root>/<Project>/Content/Paks`, and the project name is
/// not known in advance — so the directory is found by shape rather than by
/// name, which is what makes this work on a game nobody anticipated.
fn jid_dalil_pak(jidhr: &Path, athar: &mut Vec<String>) -> Option<PathBuf> {
    let Ok(madkhalat) = std::fs::read_dir(jidhr) else {
        athar.push("the game root could not be listed".to_owned());
        return None;
    };

    // Bounded like every other walk in these crates, and for the same reason:
    // the game root is somebody else's data. This one returns on the first
    // match, so the entries the bound discards are only ever reached by a root
    // that has no `<Project>/Content` at all — which is exactly the root that
    // would otherwise make a `stat` call per entry across the whole directory.
    for madkhal in madkhalat.take(AQSA_MUTABAQA).flatten() {
        let masar = madkhal.path();
        if !masar.is_dir() {
            continue;
        }
        // Engine/ holds the engine's own content, never the game's text.
        if masar
            .file_name()
            .is_some_and(|ism| ism.eq_ignore_ascii_case("Engine"))
        {
            continue;
        }
        let muhtawa = masar_bila_hala(&masar, "Content");
        if !muhtawa.is_dir() {
            continue;
        }
        let paks = masar_bila_hala(&muhtawa, "Paks");
        if paks.is_dir() {
            athar.push(format!("containers in {}", paks.display()));
            return Some(paks);
        }
        athar.push(format!("loose content in {}", muhtawa.display()));
        return Some(muhtawa);
    }

    athar.push("no <Project>/Content directory was found under the game root".to_owned());
    None
}

/// Collects the container files in one directory, in a stable order.
///
/// Sorted, because a build's flavour must not depend on the order the file
/// system happened to return entries in: two launches reporting two different
/// engine versions for one game would be unreportable.
fn ijma_hawiyat(dalil: &Path, hawiyat: &mut Vec<PathBuf>) -> Result<(), KhataUnreal> {
    let madkhalat = std::fs::read_dir(dalil).map_err(|sabab| KhataUnreal::KhataMalaf {
        masar: dalil.to_path_buf(),
        sabab,
    })?;

    for madkhal in madkhalat.flatten() {
        let masar = madkhal.path();
        let Some(imtidad) = masar.extension().and_then(|imtidad| imtidad.to_str()) else {
            continue;
        };
        let imtidad = imtidad.to_ascii_lowercase();
        if imtidad == "pak" || imtidad == "utoc" {
            hawiyat.push(masar);
        }
    }
    hawiyat.sort();
    Ok(())
}

/// The intersection of two version ranges, or the newer one when they do not
/// overlap.
///
/// A game with several containers written by one engine should narrow to that
/// engine. When two containers genuinely disagree — which happens when a patch
/// pak from a different build sits beside the original — the newer range wins,
/// because the engine that mounts them is at least as new as the newest thing
/// it can read.
fn adyaq(awwal: MadaIsdar, thani: MadaIsdar) -> MadaIsdar {
    let adna = if awwal.adna >= thani.adna {
        awwal.adna
    } else {
        thani.adna
    };
    let aqsa = if awwal.aqsa <= thani.aqsa {
        awwal.aqsa
    } else {
        thani.aqsa
    };
    if adna <= aqsa {
        MadaIsdar { adna, aqsa }
    } else if awwal.adna >= thani.adna {
        awwal
    } else {
        thani
    }
}

/// A path's file name, for a log line, without failing on a path that has none.
fn ism_malaf(masar: &Path) -> String {
    masar
        .file_name()
        .and_then(|ism| ism.to_str())
        .map_or_else(|| masar.display().to_string(), ToOwned::to_owned)
}
