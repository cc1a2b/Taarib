//! الفحص — what a probe is given, what a detector returns, and why none of them
//! is allowed to conclude anything on its own.
//!
//! A detector answers a narrow question about a game directory and reports what
//! it *saw*. It does not decide what engine the game is, does not rank itself
//! against other detectors, and does not get a veto. [`crate::tahdid`] combines
//! every detector's observations into one [`Muharrik`], and it is the only place
//! in the crate where an opinion is formed.
//!
//! That separation exists because engine detection is a field of near-misses.
//! A Godot game shipped inside an Electron wrapper is both. A Unity game with a
//! custom launcher executable looks like nothing until you find `UnityPlayer`
//! two directories down. An RPG Maker MZ project exported for desktop is
//! JavaScript inside NW.js inside a Windows binary, and all three are true.
//! Detectors that concluded would fight; detectors that observe accumulate.
//!
//! ## Read-only, bounded, and never executed
//!
//! Probing opens files and reads bytes. It does not load a game's code into this
//! process, does not run the game, does not write into the game's directory, and
//! does not follow symbolic links out of it. Every read is bounded: a detector
//! that walks a directory has a depth and an entry limit, and a detector that
//! reads a header reads a header rather than a file.
//!
//! The reason is not tidiness. This code runs across every game on somebody's
//! machine, including the one with a hundred thousand files, the one on a
//! network share, and the one whose installer left a symlink pointing at `/`.

use std::cmp::Reverse;
use std::path::{Path, PathBuf};
use std::time::Duration;

use taarib_mustalahat::muharrik::{
    AilatMuharrik, Daleel, IsdarMuharrik, ItarNusus, KhalfiyaBarmajiya, NawDaleel, WajihaRusum,
};
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, Mimariya, NizamTashghil};

/// How deep a detector may walk a game directory.
///
/// Three levels finds `Game_Data/Managed/Assembly-CSharp.dll` and
/// `Engine/Binaries/Win64/`, which is what the shapes actually require. Deeper
/// costs real time on a game with a hundred thousand asset files and finds
/// nothing a shallower walk missed, because engines put their markers near the
/// top by construction — the executable has to find them too.
pub const AQSA_UMQ: usize = 3;

/// How many directory entries a single detector may look at.
///
/// A ceiling rather than a target. It exists for the game whose developer
/// shipped every source asset next to the binary, where an unbounded walk turns
/// a two-second library scan into a two-minute one.
pub const AQSA_MADAKHIL: usize = 20_000;

/// How many bytes of a file a detector reads when it is looking for a header or
/// a version string.
///
/// Engine markers live in the first pages: a PE header, a `UnityFS` signature, a
/// PCK magic, a `GEN8` chunk. Reading more is reading a game's assets, which is
/// both slow and none of this crate's business.
pub const HAJM_TARWISA: usize = 64 * 1024;

/// What a probe is handed.
///
/// Deliberately small: a path, an executable, and the platform facts. A detector
/// that needed more than this would be a detector reaching for something it
/// should have been given.
#[derive(Debug, Clone)]
pub struct SiyaqFahs<'a> {
    /// The game's installation root.
    pub jidhr: &'a Path,
    /// The executable, when discovery found one. Many launchers do not name it,
    /// and several detectors find it themselves from the directory shape.
    pub tanfidhi: Option<&'a Path>,
    /// The game's own name, used only to break ties between two executables
    /// that are equally plausible.
    pub ism: &'a str,
    /// The operating system this probe is running on.
    pub nizam: NizamTashghil,
    /// The compatibility layer the game runs behind. A Windows game under Proton
    /// is probed as a Windows game — its files are Windows files — and the
    /// prefix only matters to Phase 15, which has to install into it.
    pub beea: &'a BeeatTawafuq,
}

impl SiyaqFahs<'_> {
    /// Joins a relative path onto the game root without leaving it.
    ///
    /// Every detector uses this rather than `Path::join`, so that a game
    /// directory containing a symbolic link to somewhere else cannot make a
    /// probe read outside the game.
    ///
    /// # Errors
    ///
    /// Fails when the relative path escapes the root or names something that
    /// cannot be a path component.
    pub fn dakhil(&self, nisbi: &str) -> Natija<PathBuf> {
        taarib_usus::masarat::dakhil(self.jidhr, nisbi)
    }

    /// Whether a path exists under the root, case-insensitively where the
    /// platform's filesystem is.
    ///
    /// Engine markers are spelled inconsistently across versions and exports —
    /// `steamapps` and `SteamApps`, `data.unity3d` and `Data.unity3d` — and a
    /// case-sensitive check finds a game on Windows and misses the same game on
    /// Linux.
    #[must_use]
    pub fn yujad(&self, nisbi: &str) -> bool {
        self.dakhil(nisbi).is_ok_and(|masar| masar.exists())
    }
}

/// What one detector observed.
///
/// Every field is optional or empty by default, because a detector reports only
/// what it is competent to see. The Unity detector has nothing to say about
/// graphics APIs; the binary detector has nothing to say about Ren'Py's version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HasilatFahs {
    /// The engine family, when this detector saw enough to name one.
    pub aila: Option<AilatMuharrik>,
    /// The version, when it was readable.
    pub isdar: Option<IsdarMuharrik>,
    /// How the game's code runs.
    pub khalfiya: Option<KhalfiyaBarmajiya>,
    /// Text systems this detector found evidence of.
    pub itarat: Vec<ItarNusus>,
    /// Graphics APIs this detector found evidence of.
    pub rusum: Vec<WajihaRusum>,
    /// The architecture of the game's main executable, when this detector read
    /// one.
    pub mimariya: Option<Mimariya>,
    /// The executable this detector believes is the game's, when it found one
    /// discovery did not name.
    pub tanfidhi: Option<PathBuf>,
    /// Everything that was actually observed. This is the part that survives
    /// into the report and into diagnostics, and the part a maintainer reads
    /// when an identification is wrong.
    pub dalail: Vec<Daleel>,
}

impl HasilatFahs {
    /// An observation of nothing. The honest answer from a detector whose engine
    /// is not the one installed, and by far the most common result: nine of the
    /// ten detectors return this for any given game.
    #[must_use]
    pub fn la_shay() -> Self {
        Self::default()
    }

    /// Records one observation.
    ///
    /// `wazn` is how much this observation is worth, 0 to 100. The scale is not
    /// arbitrary and is documented on [`crate::tahdid`]: roughly, a file only
    /// one engine ever ships is 90 and up, a file several engines share is 40 to
    /// 60, and a name that merely suggests something is below 30.
    pub fn sajjil(
        &mut self,
        naw: NawDaleel,
        wasf: impl Into<String>,
        mawqi: Option<String>,
        wazn: u8,
    ) {
        self.dalail.push(Daleel { naw, wasf: wasf.into(), mawqi, wazn: wazn.min(100) });
    }

    /// Records an observation and names the engine family it points at.
    pub fn sajjil_aila(
        &mut self,
        aila: AilatMuharrik,
        naw: NawDaleel,
        wasf: impl Into<String>,
        mawqi: Option<String>,
        wazn: u8,
    ) {
        self.aila = Some(aila);
        self.sajjil(naw, wasf, mawqi, wazn);
    }

    /// Adds a text system, without duplicating one already seen.
    pub fn daa_itar(&mut self, itar: ItarNusus) {
        if !self.itarat.contains(&itar) {
            self.itarat.push(itar);
        }
    }

    /// Adds a graphics API, without duplicating one already seen.
    pub fn daa_rusum(&mut self, wajiha: WajihaRusum) {
        if !self.rusum.contains(&wajiha) && wajiha != WajihaRusum::Majhula {
            self.rusum.push(wajiha);
        }
    }

    /// Whether this detector saw anything worth combining.
    #[must_use]
    pub const fn wajad(&self) -> bool {
        !self.dalail.is_empty()
    }

    /// The strongest single observation's weight, which is what
    /// [`crate::tahdid`] uses to rank a detector's claim against another's.
    #[must_use]
    pub fn aqwa(&self) -> u8 {
        self.dalail.iter().map(|daleel| daleel.wazn).max().unwrap_or(0)
    }
}

/// One detector.
///
/// Implementations are cheap to construct, hold no state between probes, and are
/// safe to run in any order — [`crate::tahdid`] runs all of them and combines
/// what they return, so a detector that behaved differently depending on whether
/// another had already run would make the result depend on iteration order.
pub trait Fahis: Send + Sync {
    /// A stable name, used in diagnostics and in the evidence trail.
    fn ism(&self) -> &'static str;

    /// Looks at the game and reports what it saw.
    ///
    /// # Errors
    ///
    /// Only for a failure that makes the game directory unreadable as a whole —
    /// it does not exist, or permission is denied at its root. A file that will
    /// not parse, a header with an unknown version, a directory that cannot be
    /// listed: all of those are the absence of evidence, and the correct return
    /// is [`HasilatFahs::la_shay`] rather than an error. A probe that failed
    /// because one of ten detectors met one unreadable file would leave the user
    /// with no capability report at all.
    fn ifhas(&self, siyaq: &SiyaqFahs<'_>) -> Natija<HasilatFahs>;
}

/// Everything every detector saw, before any of it is resolved.
#[derive(Debug, Clone, Default)]
pub struct JamiHasilat {
    /// One entry per detector that observed anything, in the order they ran.
    pub hasilat: Vec<(&'static str, HasilatFahs)>,
    /// How long the whole probe took, for the diagnostics screen.
    pub muddat: Duration,
}

impl JamiHasilat {
    /// Every observation, flattened, strongest first.
    ///
    /// Sorted so that a reader of the diagnostics bundle sees the evidence that
    /// decided the answer before the evidence that merely agreed with it.
    #[must_use]
    pub fn dalail(&self) -> Vec<&Daleel> {
        let mut dalail: Vec<&Daleel> =
            self.hasilat.iter().flat_map(|(_, hasila)| hasila.dalail.iter()).collect();
        dalail.sort_by_key(|daleel| Reverse(daleel.wazn));
        dalail
    }

    /// Every engine family any detector named, with the strongest weight behind
    /// each.
    ///
    /// More than one entry here is the interesting case: it is a game that looks
    /// like two engines, which is either a wrapper (Electron around a web build)
    /// or a misidentification, and [`crate::tahdid`] has to decide which.
    #[must_use]
    pub fn ailat(&self) -> Vec<(AilatMuharrik, u8)> {
        let mut ailat: Vec<(AilatMuharrik, u8)> = Vec::new();
        for (_, hasila) in &self.hasilat {
            let Some(aila) = hasila.aila else { continue };
            let quwwa = hasila.aqwa();
            match ailat.iter_mut().find(|(mawjuda, _)| *mawjuda == aila) {
                Some((_, sabiqa)) => *sabiqa = (*sabiqa).max(quwwa),
                None => ailat.push((aila, quwwa)),
            }
        }
        ailat.sort_by_key(|(_, quwwa)| Reverse(*quwwa));
        ailat
    }

    /// Every text system any detector found.
    #[must_use]
    pub fn itarat(&self) -> Vec<ItarNusus> {
        let mut itarat: Vec<ItarNusus> = Vec::new();
        for (_, hasila) in &self.hasilat {
            for itar in &hasila.itarat {
                if !itarat.contains(itar) {
                    itarat.push(*itar);
                }
            }
        }
        itarat
    }

    /// Every graphics API any detector found.
    #[must_use]
    pub fn rusum(&self) -> Vec<WajihaRusum> {
        let mut rusum: Vec<WajihaRusum> = Vec::new();
        for (_, hasila) in &self.hasilat {
            for wajiha in &hasila.rusum {
                if !rusum.contains(wajiha) {
                    rusum.push(*wajiha);
                }
            }
        }
        rusum
    }
}
