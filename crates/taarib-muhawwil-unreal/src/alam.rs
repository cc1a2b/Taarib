//! العلم — making Arabic a culture the game has, and the culture it starts in.
//!
//! Translated `.locres` under a culture directory the engine never scans is a
//! translation nobody sees. This module does the two things that make the
//! resources `mawarid` installed actually load: it tells Unreal where the
//! patch's localization target lives, and it tells Unreal to start in Arabic.
//!
//! ## The ladder, cheapest first
//!
//! Same shape as [`crate::tashghil`], for the same reason — see that module's
//! header for the argument in full.
//!
//! 1. **Configuration.** `[Internationalization]` in the game's `Engine.ini` is
//!    the documented Unreal route: `Culture=` names the culture the engine
//!    starts in, and `+LocalizationPaths=` adds a directory to the set the
//!    engine scans for localization targets. That second key is what *creates*
//!    the available culture: Unreal derives the culture list from the
//!    subdirectories of the localization targets it knows about, so adding the
//!    patch's target is what makes `ar` exist at all.
//! 2. **Command line.** `-culture=ar` is a documented launch option and outlives
//!    a game that regenerates its ini from a template on every start.
//! 3. **Injection.** [`Musaddir::thabbit_thaqafa`] in the live process, plus a
//!    watchdog assertion, for the game whose own language menu writes its saved
//!    choice back over the startup culture during load.
//!
//! Every rung is verified by reading the active culture back, and
//! [`SijillTashghil`] records the rung whose effect was confirmed rather than
//! the rung that was attempted.
//!
//! ## Which spelling of Arabic to register, and why
//!
//! Unreal resolves a requested culture into a prioritised list by stripping
//! subtags from the right and looks for localization data at each name in turn —
//! [`silsilat_irtida`] is that walk. So:
//!
//! - `ar-SA` resolves to `["ar-SA", "ar"]`. Data placed at `ar-SA` is found by a
//!   player whose system says `ar-SA` and is **invisible** to one whose system
//!   says `ar-EG`, because `ar-EG` never asks for `ar-SA`.
//! - `ar-001` is CLDR's Modern Standard Arabic and resolves to `["ar-001",
//!   "ar"]`. Unreal's ICU data carries it, but a packaged game routinely trims
//!   that data down to the cultures it ships, and a culture whose data was
//!   trimmed resolves to a fallback with nothing behind it.
//! - `ar` is reached from every one of the above.
//!
//! So the patch registers **`ar`** and asks for **`ar`**. It is the only
//! spelling every Arabic-speaking player's system reaches, and it is the one
//! that leaves regional behaviour alone: asking for `ar-SA` would additionally
//! select Saudi number, date and currency formatting for a player in Morocco,
//! and choosing somebody's region for them is a policy decision this adapter is
//! not allowed to make.
//!
//! `ar-SA` and `ar-001` are *accepted* rather than introduced: when the game's
//! own language list already carries one of them, [`wasm_mufaddal`] returns the
//! game's own spelling and nothing adds a second entry beside it. A language
//! menu with two Arabics in it is a bug the player sees.
//!
//! One consequence worth stating rather than discovering: `ar`'s default CLDR
//! numbering system is Arabic-Indic, so numbers **the engine formats itself** —
//! timers, counters, currency — will change digits along with the culture. The
//! patch's own [`taarib_usus::NizamArqam`] governs Taarib's text and does not
//! reach engine-formatted numbers. That is the engine behaving correctly for the
//! culture it was asked for, and it is not this module's to override.
//!
//! ## The three cases the roadmap names
//!
//! - The game's own language menu lists Arabic — [`HalatQaima::Madruja`], and
//!   the best outcome: the player can switch back and forth and the game's own
//!   settings persistence does the work.
//! - The menu does not list it, but the startup culture takes —
//!   [`HalatQaima::Mutajawaza`]. Fine. The menu shows the game's original list
//!   and the game runs in Arabic anyway.
//! - The menu writes its saved choice back over ours during load —
//!   [`HalatQaima::Tuktab`]. Detected as a read-back that disagrees immediately
//!   after a successful set, and handled by contributing [`Alam::tawkeed`] to
//!   the shaping watchdog rather than by starting a second one.

use std::path::{Path, PathBuf};

use crate::khata::KhataUnreal;
use crate::tashghil::{
    MadkhalIni, Musaddir, Rutba, SijillTashghil, Tawkeed, aktub_madakhil, tarajua_madakhil,
};

/// The culture the patch registers and asks for.
pub const WASM_ARABI: &str = "ar";

/// Accepted when the game already spells it this way. Never introduced.
pub const WASM_ARABI_SA: &str = "ar-SA";

/// CLDR's Modern Standard Arabic. Accepted, never introduced.
pub const WASM_ARABI_ALAMI: &str = "ar-001";

/// The ini section Unreal reads its internationalisation settings from.
pub const QISM_TADWEEL: &str = "Internationalization";

/// The key naming the culture the engine starts in.
pub const MIFTAH_THAQAFA: &str = "Culture";

/// The key adding a directory to the set scanned for localization targets.
///
/// Additive — the `+` matters. Writing `LocalizationPaths=` without it would
/// replace the game's own list and the game would lose every language it
/// shipped with.
pub const MIFTAH_MASARAT: &str = "+LocalizationPaths";

/// Where the patch's localization target is mounted.
///
/// Relative to the game directory rather than absolute, because the patch pak
/// mounts its content under the game's own `Content` root at a higher priority
/// and this path then resolves inside it exactly as the game's own targets do.
/// An absolute path written into a game's config is a path that breaks the day
/// the player moves their Steam library to another drive.
pub const MASAR_TADWEEL_RUQAA: &str = "%GAMEDIR%Content/Localization/Taarib";

/// What the game's own language menu does with the culture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalatQaima {
    /// The menu lists Arabic. The player can choose it, and the game remembers.
    Madruja,
    /// The menu does not list it and the startup override carried it anyway.
    Mutajawaza,
    /// The menu writes its own saved choice back over the startup culture.
    Tuktab,
}

impl HalatQaima {
    /// A stable short name for logs and diagnostics bundles.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Madruja => "listed",
            Self::Mutajawaza => "bypassed",
            Self::Tuktab => "overwritten",
        }
    }

    /// Whether this case needs the watchdog to hold the culture.
    #[must_use]
    pub const fn yahtaj_haris(self) -> bool {
        matches!(self, Self::Tuktab)
    }
}

/// What activating the culture produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatijatAlam {
    /// The culture tag that was actually registered — the game's spelling when
    /// it had one, [`WASM_ARABI`] otherwise.
    pub wasm: String,
    /// Which rung fired, and what every rung reported.
    pub sijill: SijillTashghil,
    /// What the game's language menu was observed to do.
    pub hala: HalatQaima,
}

/// Unreal's prioritised culture list for a requested tag.
///
/// The tag itself, then the tag with its last subtag removed, and so on down to
/// the primary language. This is the walk that makes `ar` the right thing to
/// register: every `ar-XX` request passes through it, and no `ar-XX` request
/// passes through any other regional spelling.
///
/// Empty and malformed input produces an empty chain rather than a chain
/// containing an empty name, because an empty culture name in an Unreal config
/// selects the invariant culture and would leave the game in no language at all.
#[must_use]
pub fn silsilat_irtida(wasm: &str) -> Vec<String> {
    let mahdhuf = wasm.trim();
    if mahdhuf.is_empty() {
        return Vec::new();
    }
    let mut silsila = Vec::new();
    let mut hali = mahdhuf;
    loop {
        silsila.push(hali.to_owned());
        match hali.rsplit_once(['-', '_']) {
            Some((asas, _)) if !asas.is_empty() => hali = asas,
            _ => break,
        }
    }
    silsila
}

/// The primary language subtag of a culture name, lowercased.
#[must_use]
pub fn lugha_wasm(wasm: &str) -> String {
    wasm.trim()
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Which spelling of Arabic to use, given what the game already advertises.
///
/// Returns the game's own spelling when its culture list already carries an
/// Arabic one, so that the patch selects the entry the game's language menu
/// already draws instead of adding a second Arabic beside it. Returns
/// [`WASM_ARABI`] otherwise, for the reasons in this module's header.
#[must_use]
pub fn wasm_mufaddal(matah: &[String]) -> String {
    matah
        .iter()
        .find(|wasm| lugha_wasm(wasm) == WASM_ARABI)
        .map_or_else(|| WASM_ARABI.to_owned(), |wasm| wasm.trim().to_owned())
}

/// Whether a culture list already carries an Arabic entry.
#[must_use]
pub fn madruja(matah: &[String]) -> bool {
    matah.iter().any(|wasm| lugha_wasm(wasm) == WASM_ARABI)
}

/// Registering Arabic and making it active, down the ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alam {
    ini: PathBuf,
    wasm: String,
    masarat: Vec<String>,
}

impl Alam {
    /// Builds the registration against one game's `Engine.ini`, asking for
    /// [`WASM_ARABI`] and adding the patch's own localization target.
    #[must_use]
    pub fn jadeed(ini: impl Into<PathBuf>) -> Self {
        Self {
            ini: ini.into(),
            wasm: WASM_ARABI.to_owned(),
            masarat: vec![MASAR_TADWEEL_RUQAA.to_owned()],
        }
    }

    /// Builds the registration using the game's own spelling of Arabic when it
    /// has one — see [`wasm_mufaddal`].
    #[must_use]
    pub fn min_matah(ini: impl Into<PathBuf>, matah: &[String]) -> Self {
        Self::jadeed(ini).bi_wasm(wasm_mufaddal(matah))
    }

    /// Overrides the culture tag.
    #[must_use]
    pub fn bi_wasm(mut self, wasm: impl Into<String>) -> Self {
        self.wasm = wasm.into();
        self
    }

    /// Replaces the localization paths this registration adds.
    #[must_use]
    pub fn bi_masarat(mut self, masarat: Vec<String>) -> Self {
        self.masarat = masarat;
        self
    }

    /// Adds one more localization path — for a patch that ships more than one
    /// target, which a game with separate subtitle and interface targets needs.
    #[must_use]
    pub fn bi_masar_tadweel(mut self, masar: impl Into<String>) -> Self {
        self.masarat.push(masar.into());
        self
    }

    /// The culture tag this registration asks for.
    #[must_use]
    pub fn wasm(&self) -> &str {
        &self.wasm
    }

    /// The ini this registration writes into.
    #[must_use]
    pub fn ini(&self) -> &Path {
        &self.ini
    }

    /// The localization paths this registration adds.
    #[must_use]
    pub fn masarat(&self) -> &[String] {
        &self.masarat
    }

    /// The prioritised chain the engine will walk for this tag.
    #[must_use]
    pub fn silsila(&self) -> Vec<String> {
        silsilat_irtida(&self.wasm)
    }

    /// The ini entries this registration owns.
    ///
    /// The localization paths come first: a `Culture=` naming a culture no
    /// scanned target provides is a culture the engine falls back out of during
    /// start, and the order the keys are written in is the order a maintainer
    /// reads them in.
    #[must_use]
    pub fn madakhil(&self) -> Vec<MadkhalIni> {
        let mut madakhil: Vec<MadkhalIni> = self
            .masarat
            .iter()
            .map(|masar| MadkhalIni::jadeed(MIFTAH_MASARAT, masar.as_str()))
            .collect();
        madakhil.push(MadkhalIni::jadeed(MIFTAH_THAQAFA, self.wasm.as_str()));
        madakhil
    }

    /// The watchdog assertion that holds the culture.
    #[must_use]
    pub fn tawkeed(&self) -> Tawkeed {
        Tawkeed::thaqafa(self.wasm.as_str())
    }

    /// Rung one: write `[Internationalization]`.
    ///
    /// # Errors
    ///
    /// Whatever [`aktub_madakhil`] refuses, unchanged, so the caller can name
    /// the path.
    pub fn rutbat_idadat(&self) -> Result<(), KhataUnreal> {
        aktub_madakhil(&self.ini, QISM_TADWEEL, &self.madakhil())
    }

    /// Rung two: the launch options carrying the same registration.
    ///
    /// `-culture=` is the documented one and is what a game's own launcher
    /// argument field accepts; the `-ini:` overrides are there because a game
    /// that ignores `-culture=` may still honour a config override, and a launch
    /// option costs nothing to offer.
    #[must_use]
    pub fn rutbat_satr(&self) -> Vec<String> {
        if self.wasm.trim().is_empty() {
            return Vec::new();
        }
        let mut khiyarat = vec![
            format!("-culture={}", self.wasm),
            format!(
                "-ini:Engine:[{QISM_TADWEEL}]:{MIFTAH_THAQAFA}={}",
                self.wasm
            ),
        ];
        for masar in &self.masarat {
            khiyarat.push(format!(
                "-ini:Engine:[{QISM_TADWEEL}]:{MIFTAH_MASARAT}={masar}"
            ));
        }
        khiyarat
    }

    /// Rung three: set the culture in the live process.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the binding has no handle on the
    /// engine or does not reach the internationalisation singleton, and
    /// otherwise whatever [`Musaddir::thabbit_thaqafa`] refuses.
    pub fn rutbat_haqn(&self, musaddir: &dyn Musaddir) -> Result<(), KhataUnreal> {
        if !musaddir.muhayya() {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: "the Slate binding reports no handle on this process yet".to_owned(),
            });
        }
        musaddir.thabbit_thaqafa(&self.wasm)
    }

    /// Reads the active culture back and says whether it satisfies this
    /// registration.
    ///
    /// # Errors
    ///
    /// Whatever [`Musaddir::thaqafa_haliya`] refuses.
    pub fn tahaqquq(&self, musaddir: &dyn Musaddir) -> Result<bool, KhataUnreal> {
        self.tawkeed().muhaqqaq(musaddir)
    }

    /// Walks the ladder and reports which rung took and what the menu did.
    ///
    /// `matah` is the game's own advertised culture list, supplied by the caller
    /// because reading it is `mawarid`'s job and deciding what it means is the
    /// patch's. Pass an empty slice when it is not known; the only thing lost is
    /// the ability to distinguish [`HalatQaima::Madruja`] from
    /// [`HalatQaima::Mutajawaza`], and neither changes what is written.
    ///
    /// `musaddir` is [`None`] for the offline installer, which has no process to
    /// look at.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::AlamMarfud`] naming every route that was tried, and only
    /// when no rung took and none is pending. This is a warning in the error
    /// contract, not a failure: the resources are installed either way, and the
    /// player can still reach Arabic through the game's own language menu when
    /// the menu lists it.
    pub fn faal(
        &self,
        musaddir: Option<&dyn Musaddir>,
        matah: &[String],
    ) -> Result<NatijatAlam, KhataUnreal> {
        let mut sijill = SijillTashghil::default();

        let idadat_najahat = match self.rutbat_idadat() {
            Ok(()) => {
                sijill.sajjil(
                    Rutba::Idadat,
                    false,
                    format!(
                        "[{QISM_TADWEEL}] now names {} and scans {} localization path(s); \
                         applies at the next launch",
                        self.wasm,
                        self.masarat.len()
                    ),
                );
                true
            },
            Err(khata) => {
                sijill.sajjil(Rutba::Idadat, false, khata.to_string());
                false
            },
        };

        let khiyarat = self.rutbat_satr();
        sijill.sajjil(
            Rutba::SatrAwamir,
            false,
            format!("{} launch options offered to the installer", khiyarat.len()),
        );

        let mut hala = if madruja(matah) {
            HalatQaima::Madruja
        } else {
            HalatQaima::Mutajawaza
        };

        let Some(musaddir) = musaddir else {
            if idadat_najahat {
                return Ok(NatijatAlam {
                    wasm: self.wasm.clone(),
                    sijill,
                    hala,
                });
            }
            return Err(KhataUnreal::AlamMarfud {
                sabab: sijill.sabab(),
            });
        };

        // Already Arabic? Then nothing goes into the process. The whole point of
        // the ladder is that the injected rung is the one that usually does not
        // have to run.
        match self.tahaqquq(musaddir) {
            Ok(true) => {
                sijill.rutab.clear();
                sijill.sajjil(
                    Rutba::Idadat,
                    true,
                    format!(
                        "the process already reports {}; nothing was injected",
                        self.wasm
                    ),
                );
                return Ok(NatijatAlam {
                    wasm: self.wasm.clone(),
                    sijill,
                    hala,
                });
            },
            Ok(false) => {},
            Err(khata) => {
                tracing::debug!(sabab = %khata, "the active culture could not be read back");
            },
        }

        match self.rutbat_haqn(musaddir) {
            Ok(()) => {
                let muakkada = self.tahaqquq(musaddir).unwrap_or(false);
                if !muakkada {
                    // Set, accepted, and already not active. That is the game's
                    // own settings load writing its saved language back over
                    // ours, which is exactly case three.
                    hala = HalatQaima::Tuktab;
                }
                sijill.sajjil(
                    Rutba::Haqn,
                    muakkada,
                    if muakkada {
                        format!("{} is active and confirmed by read-back", self.wasm)
                    } else {
                        "the culture was set and the game overwrote it; the watchdog will \
                         hold it"
                            .to_owned()
                    },
                );
            },
            Err(khata) => sijill.sajjil(Rutba::Haqn, false, khata.to_string()),
        }

        if sijill.muakkad() || idadat_najahat || hala == HalatQaima::Tuktab {
            Ok(NatijatAlam {
                wasm: self.wasm.clone(),
                sijill,
                hala,
            })
        } else {
            Err(KhataUnreal::AlamMarfud {
                sabab: sijill.sabab(),
            })
        }
    }

    /// Removes everything Taarib wrote into this ini.
    ///
    /// Every Taarib block in the file, not only this module's: one ini carries
    /// the shaping keys and the culture keys, uninstall wants both gone, and a
    /// per-module undo would leave whichever module the caller forgot.
    ///
    /// # Errors
    ///
    /// Whatever [`tarajua_madakhil`] refuses.
    pub fn tarajua(&self) -> Result<(), KhataUnreal> {
        tarajua_madakhil(&self.ini)
    }
}
