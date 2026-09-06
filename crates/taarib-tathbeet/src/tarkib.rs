//! Per-engine, per-OS framework deployment: what to place, where, and what makes it load.

use std::fmt;
use std::path::{Component, Path, PathBuf};

use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_mustalahat::muharrik::{
    AilatMuharrik, KhalfiyaBarmajiya, Muharrik, Tabaqa, TaqreerImkaniyat,
};
use taarib_usus::manassa::{BeeatTawafuq, HalatTashghil, Mimariya, NizamTashghil};
use taarib_usus::masarat;
use walkdir::WalkDir;

use crate::bayan::{MahallIdad, Muthabbit, basma_bayt};
use crate::itlaq::{ASMAA_STEAM, ISM_STEAM, halat_manassa, manassa_mughlaqa};
use crate::khata::{KhataTathbeet, NatijatTathbeet, min_khata_io, tul_u64};
use crate::mawdi::{MUJALLAD_TAARIB, WajhatLuba, WajhatNizam};
use crate::nusus::{IdhnNusus, Nashir};
use crate::wukala::{self, WakeelQaim};

/// The largest framework component this build will deploy, in bytes.
pub const AQSA_HAJM_MUKAWWIN: u64 = 536_870_912;

/// The store subdirectory of a split component whose files go beside the game's
/// executable; everything else in such a component goes under the game's own
/// Taarib directory.
pub const MUJALLAD_MUHAMMIL: &str = "muhammil";

/// The Wine load order that makes an overridden module resolve to the copy in
/// the game directory rather than to Wine's own builtin.
const TARTEEB_ASLI: &str = "n,b";

/// The token Steam replaces with the game's own command line.
const RAMZ_AMR: &str = "%command%";

/// The environment variable Wine reads per-process DLL overrides from.
const ISM_TAJAWUZAT: &str = "WINEDLLOVERRIDES";

/// The environment variable a Linux game preloads a shared object from.
const ISM_TAHMIL_LINUX: &str = "LD_PRELOAD";

/// The environment variable a macOS game preloads a dynamic library from.
const ISM_TAHMIL_MAC: &str = "DYLD_INSERT_LIBRARIES";

/// The proxy module BepInEx's Windows loader is published as.
const WAKEEL_BEPINEX: &str = "winhttp";

/// The proxy module Taarib's own Windows loader is published as.
const WAKEEL_MUDKHAL: &str = "version";

/// The base name of BepInEx's Unix preload library, as Taarib stores it.
const ASAS_DOORSTOP: &str = "doorstop";

/// The base name of Taarib's own Unix preload library, as Taarib stores it.
const ASAS_MUDKHAL: &str = "taarib_muhammil";

/// Files whose presence beside a game's executable means some BepInEx is
/// already installed, whoever installed it.
const ALAMAT_BEPINEX: [&str; 3] = ["BepInEx", "doorstop_config.ini", "run_bepinex.sh"];

/// The component store's root for the additive adapters — the script-engine
/// files that are loaded by the engine itself and need no injected loader.
const MUKAWWIN_MULHAQ: &str = "mulhaq";

/// The Ren'Py adapter component, whose whole tree is copied into `game/`.
const MUKAWWIN_RENPY: &str = "mulhaq/renpy";

/// Where the Ren'Py component keeps the Arabic face it ships, *inside* the
/// component.
///
/// A path within the component rather than a store of its own, and that is the
/// point: [`renpy`] copies the component's whole tree into `game/`, so a face
/// staged at `mulhaq/renpy/taarib/khutut/X.ttf` lands at
/// `game/taarib/khutut/X.ttf` with no deployment step of its own, and the name
/// the generated `.rpy` must use is exactly the tail — `taarib/khutut/X.ttf` —
/// because that is what Ren'Py's loader resolves against `game/`.
///
/// `taarib/` already belongs to Taarib inside a Ren'Py game;
/// `taarib_muhawwil_nusus::renpy::MALAF_BAYANAT` is `game/taarib/idad.json`.
const MUJALLAD_KHATT_RENPY: &str = "taarib/khutut/";

/// The file extensions a Ren'Py face may have.
///
/// Ren'Py's font stack reads both. Folded before the comparison, because a
/// component store on a case-insensitive filesystem can hand back `X.TTF` for
/// the entry it was given as `X.ttf`.
const LAWAHIQ_KHATT: [&str; 2] = ["ttf", "otf"];

/// The order a Ren'Py face is chosen in when the component ships more than one,
/// most preferred first, matched against the file name's leading characters.
///
/// A visual novel is not a user interface. Its text is body copy — paragraphs
/// of dialogue read continuously at Ren'Py's default `gui.text_size` of 22 —
/// and the Arabic tradition for body copy is Naskh. So the screen-designed
/// Naskh leads, the book Naskh follows it, and the two sans faces come after
/// both, because a geometric sans set as a novel's dialogue reads as a menu.
///
/// The list ranks; it does not filter. A face the list does not name is still
/// chosen when it is what the component ships, sorted by name after every face
/// the list does name — a bundle that staged one Kufi face into the Ren'Py
/// component meant to ship it, and answering "no font" to that would be this
/// module overruling the build rather than reading it.
const TARTIB_KHATT_RENPY: [&str; 4] =
    ["NotoNaskhArabic", "Amiri", "IBMPlexSansArabic", "NotoSansArabic"];

/// The largest loader registry Taarib will read in order to append its own
/// registration.
const SAQF_HAJM_TASJIL: u64 = 16 * 1024 * 1024;

/// The `$plugins` entry that registers Taarib in an RPG Maker `plugins.js`.
const MADKHAL_PLUGINS: &str =
    "{\"name\":\"taarib\",\"status\":true,\"description\":\"تعريب\",\"parameters\":{}}";

/// The whitespace-free needle that proves Taarib is already registered in a
/// `plugins.js`, matched against a whitespace-stripped copy of the file so
/// that `"name": "taarib"` and `"name":"taarib"` are one registration.
const IBRAT_PLUGINS: &str = "\"name\":\"taarib\"";

/// The resource path `taarib.gdnlib` is registered under in a Godot 3 project.
const MAWRID_GDNLIB: &str = "res://taarib.gdnlib";

/// The Godot 3 project-settings section that lists `GDNative` singletons.
const QISM_GDNATIVE: &str = "[gdnative]";

/// The key inside that section, which holds a bracketed list on one line.
const MIFTAH_SINGLETONS: &str = "singletons=";

/// The Unity generation a BepInEx build is published against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JeelUnity {
    /// Unity 5.0 through 2018.4.
    Qadeem,
    /// Unity 2019.1 through 2021.3.
    Wasat,
    /// Unity 2022.1 and newer, including Unity 6.
    Hadith,
}

impl JeelUnity {
    /// The generation a major version belongs to, or [`None`] below Unity 5.
    #[must_use]
    pub const fn min_kabir(kabir: u16) -> Option<Self> {
        match kabir {
            0..=4 => None,
            5..=2018 => Some(Self::Qadeem),
            2019..=2021 => Some(Self::Wasat),
            _ => Some(Self::Hadith),
        }
    }

    /// The component store's name for this generation.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Qadeem => "qadeem",
            Self::Wasat => "wasat",
            Self::Hadith => "hadith",
        }
    }

    /// The version range this generation covers, for the install report.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Qadeem => "5.0-2018.4",
            Self::Wasat => "2019.1-2021.3",
            Self::Hadith => "2022.1 and newer",
        }
    }
}

/// The Unity scripting backend a BepInEx build is published against.
///
/// Narrower than [`KhalfiyaBarmajiya`], which names every backend of every
/// engine: only these two have a BepInEx build, and a component name assembled
/// from anything else would point at a store entry that does not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KhalfiyatUnity {
    /// Managed assemblies on disk, loaded by Unity's Mono runtime.
    Mono,
    /// Managed code compiled ahead of time into a native binary.
    Il2cpp,
}

impl KhalfiyatUnity {
    /// The backend a general engine reading names, when it names one of these.
    #[must_use]
    pub const fn min_khalfiya(khalfiya: KhalfiyaBarmajiya) -> Option<Self> {
        match khalfiya {
            KhalfiyaBarmajiya::Mono => Some(Self::Mono),
            KhalfiyaBarmajiya::Il2cpp => Some(Self::Il2cpp),
            _ => None,
        }
    }

    /// The component store's name for this backend.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mono => "mono",
            Self::Il2cpp => "il2cpp",
        }
    }

    /// The backend's own spelling, for the install report.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::Mono => "Mono",
            Self::Il2cpp => "IL2CPP",
        }
    }
}

/// How a deployed framework is made to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TahmilMusbaq {
    /// A proxy module the Windows loader takes from the executable's own
    /// directory without being told to.
    WakeelWindows {
        /// The system module the proxy stands in for.
        wahda: String,
    },

    /// The same proxy under Wine, which loads Wine's builtin instead unless the
    /// module is overridden to native.
    TajawuzWine {
        /// The module named in the override.
        wahda: String,
    },

    /// A shared object named in `LD_PRELOAD`.
    LdPreload,

    /// A dynamic library named in `DYLD_INSERT_LIBRARIES`.
    DyldInsert,
}

impl TahmilMusbaq {
    /// The environment variable this mechanism is carried by, when it is
    /// carried by one.
    #[must_use]
    pub const fn mutaghayyir(&self) -> Option<&'static str> {
        match self {
            Self::WakeelWindows { .. } => None,
            Self::TajawuzWine { .. } => Some(ISM_TAJAWUZAT),
            Self::LdPreload => Some(ISM_TAHMIL_LINUX),
            Self::DyldInsert => Some(ISM_TAHMIL_MAC),
        }
    }

    /// A one-line description for the install report.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::WakeelWindows { wahda } => {
                format!("loaded as the {wahda} proxy beside the executable")
            }
            Self::TajawuzWine { wahda } => {
                format!("loaded through a Wine override of {wahda} to native")
            }
            Self::LdPreload => format!("loaded through {ISM_TAHMIL_LINUX}"),
            Self::DyldInsert => format!("loaded through {ISM_TAHMIL_MAC}"),
        }
    }
}

/// How a component's files are spread over the destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TawziMukawwin {
    /// The whole tree lands at the framework's deployment root.
    Wahid,

    /// The tree's `muhammil/` subdirectory lands at the deployment root and
    /// everything else lands under the game's own Taarib directory.
    Munfasil,
}

/// Why an engine needs no framework at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SababLaHaja {
    /// The engine loads the translated data and Taarib's additive package
    /// through its own mechanisms; there is no code to inject.
    BayanatWaMulhaq,

    /// The scripting backend was not identified, and the framework for an
    /// engine is chosen by its backend.
    KhalfiyaMajhula,

    /// The engine version was not read, and the framework for this engine is
    /// chosen by its version.
    IsdarMajhul,

    /// The engine version is outside what any framework in the store covers.
    IsdarKharijAlDam,

    /// The game is not modified at all: tier 3 shows Arabic from Taarib's own
    /// overlay, outside the process, and this is where that stays true.
    TabaqaFawqiya,

    /// The translated content is the patched game file itself — RPG Maker VX
    /// Ace's script archive, a GameMaker `data.win` — which is Phase 14 patch
    /// content and not something this module deploys.
    DakhilAlRuqaa,
}

impl SababLaHaja {
    /// The reason in English, as the install report writes it.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::BayanatWaMulhaq => {
                "this engine loads the translated data and Taarib's additional package on its \
                 own, so no framework is installed into the game"
            }
            Self::KhalfiyaMajhula => {
                "the scripting backend was not identified, and which framework belongs in this \
                 game is decided by it"
            }
            Self::IsdarMajhul => {
                "the engine version was not read, and which framework belongs in this game is \
                 decided by it"
            }
            Self::IsdarKharijAlDam => {
                "the engine version is older than any framework Taarib carries a build for"
            }
            Self::TabaqaFawqiya => {
                "this game is not modified at all; Arabic is shown by Taarib's overlay, which \
                 attaches from outside when the game launches"
            }
            Self::DakhilAlRuqaa => {
                "this engine is translated through its own patched data file, which is part of \
                 the patch itself; there is no framework to install"
            }
        }
    }

    /// The reason in Arabic, as the install report writes it.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::BayanatWaMulhaq => {
                "هذا المحرّك يحمّل البيانات المعرّبة وحزمة تعريب الإضافية بنفسه، فلا يُثبَّت داخل \
                 اللعبة أي إطار."
            }
            Self::KhalfiyaMajhula => {
                "لم تُحدَّد خلفية تشغيل الشيفرة في هذه اللعبة، وعليها يُبنى اختيار الإطار."
            }
            Self::IsdarMajhul => {
                "لم تُقرأ نسخة المحرّك، وعليها يُبنى اختيار الإطار المناسب لهذه اللعبة."
            }
            Self::IsdarKharijAlDam => {
                "نسخة المحرّك أقدم من كل إطار يحمل تعريب بناءً له."
            }
            Self::TabaqaFawqiya => {
                "لا تُعدَّل هذه اللعبة إطلاقًا؛ تُعرض العربية من طبقة تعريب الخارجية عند تشغيلها."
            }
            Self::DakhilAlRuqaa => {
                "تُعرَّب هذه اللعبة عبر ملف بياناتها المرقوع، وهو جزء من الرقعة نفسها؛ لا إطار \
                 يُثبَّت هنا."
            }
        }
    }
}

/// One framework component, as the per-engine table names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MukawwinItar {
    /// The component's path inside the component store, `/`-separated.
    pub ism: String,

    /// A one-line description for the install report.
    pub wasf: String,

    /// How the component's files are spread over the destinations.
    pub tawzi: TawziMukawwin,

    /// How the deployed framework is made to load.
    pub tahmil: TahmilMusbaq,

    /// The loader's own file name, which must be present in the component or
    /// the deployment is refused.
    pub ism_muhammil: String,

    /// Names at the deployment root whose presence means a framework of this
    /// kind is already installed by somebody else.
    pub alamat: Vec<String>,
}

impl MukawwinItar {
    /// The BepInEx build for one Unity backend, generation, target and
    /// architecture.
    #[must_use]
    pub fn bepinex(
        hadaf: NizamTashghil,
        khalfiya: KhalfiyatUnity,
        jeel: JeelUnity,
        mimariya: Mimariya,
        beea: bool,
    ) -> Self {
        let (ism_muhammil, tahmil) = muhammil_windows_aw_unix(
            hadaf,
            beea,
            WAKEEL_BEPINEX,
            ASAS_DOORSTOP,
        );
        let mut alamat: Vec<String> =
            ALAMAT_BEPINEX.iter().map(|ism| (*ism).to_owned()).collect();
        alamat.push(ism_muhammil.clone());
        Self {
            ism: format!(
                "bepinex/{}/{}-{}-{}",
                ism_hadaf(hadaf),
                khalfiya.ism(),
                jeel.ism(),
                mimariya.mujallad()
            ),
            wasf: format!(
                "BepInEx for Unity {} ({}, {})",
                jeel.wasf(),
                khalfiya.wasf(),
                mimariya.mujallad()
            ),
            tawzi: TawziMukawwin::Wahid,
            tahmil,
            ism_muhammil,
            alamat,
        }
    }

    /// Taarib's own loader and injected module, for the engines that have no
    /// third-party framework and for the overlay tier.
    #[must_use]
    pub fn mudkhal(hadaf: NizamTashghil, mimariya: Mimariya, beea: bool) -> Self {
        let (ism_muhammil, tahmil) =
            muhammil_windows_aw_unix(hadaf, beea, WAKEEL_MUDKHAL, ASAS_MUDKHAL);
        Self {
            ism: format!("mudkhal/{}/{}", ism_hadaf(hadaf), mimariya.mujallad()),
            wasf: format!(
                "the Taarib loader and injected module ({}, {})",
                ism_hadaf(hadaf),
                mimariya.mujallad()
            ),
            tawzi: TawziMukawwin::Munfasil,
            tahmil,
            alamat: vec![ism_muhammil.clone()],
            ism_muhammil,
        }
    }
}

/// What the per-engine table says about one identified engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HajatItar {
    /// A framework component must be deployed.
    Matlub(Box<MukawwinItar>),

    /// No framework is deployed, for a named reason.
    LaHaja(SababLaHaja),
}

/// The per-engine, per-OS framework table.
///
/// **The engine's answer, not the product's.** It cannot see the tier, so it
/// says a GameMaker game needs no loader when at tier 2 it does, and it says an
/// unrecognised engine gets Taarib's own loader when at tier 3 nothing at all
/// may be deployed. [`khutta`] is the answer to act on; this is the table it
/// consults. Nothing on the install path calls it directly any more, and a new
/// caller that does is re-introducing the defect [`rakkib_itar`] was fixed for.
#[must_use]
pub fn hajat_itar(
    muharrik: &Muharrik,
    nizam: NizamTashghil,
    beea: &BeeatTawafuq,
) -> HajatItar {
    let hadaf = hadaf_hamula(nizam, beea);
    let fi_beea = beea.windows_dakhilan();
    let mimariya = muharrik.mimariya;

    match muharrik.aila {
        AilatMuharrik::Unity => match KhalfiyatUnity::min_khalfiya(muharrik.khalfiya) {
            Some(khalfiya) => {
                let Some(isdar) = muharrik.isdar.as_ref() else {
                    return HajatItar::LaHaja(SababLaHaja::IsdarMajhul);
                };
                let Some(jeel) = JeelUnity::min_kabir(isdar.kabir) else {
                    return HajatItar::LaHaja(SababLaHaja::IsdarKharijAlDam);
                };
                HajatItar::Matlub(Box::new(MukawwinItar::bepinex(
                    hadaf, khalfiya, jeel, mimariya, fi_beea,
                )))
            }
            None => HajatItar::LaHaja(SababLaHaja::KhalfiyaMajhula),
        },

        AilatMuharrik::Unreal => match muharrik.khalfiya {
            KhalfiyaBarmajiya::UnrealNative => HajatItar::Matlub(Box::new(
                MukawwinItar::mudkhal(hadaf, mimariya, fi_beea),
            )),
            _ => HajatItar::LaHaja(SababLaHaja::KhalfiyaMajhula),
        },

        AilatMuharrik::Godot => match muharrik.isdar.as_ref().map(|isdar| isdar.kabir) {
            None => HajatItar::LaHaja(SababLaHaja::IsdarMajhul),
            Some(0..=2) => HajatItar::LaHaja(SababLaHaja::IsdarKharijAlDam),
            Some(3) => HajatItar::Matlub(Box::new(MukawwinItar::mudkhal(
                hadaf, mimariya, fi_beea,
            ))),
            Some(_) => HajatItar::LaHaja(SababLaHaja::BayanatWaMulhaq),
        },

        AilatMuharrik::RpgMakerMv
        | AilatMuharrik::RpgMakerMz
        | AilatMuharrik::Renpy
        | AilatMuharrik::Electron => HajatItar::LaHaja(SababLaHaja::BayanatWaMulhaq),

        AilatMuharrik::RpgMakerVxAce | AilatMuharrik::GameMaker => {
            HajatItar::LaHaja(SababLaHaja::DakhilAlRuqaa)
        }

        // Capcom's BIO4 has no plugin system, no scripting backend and no
        // third-party framework anywhere — nobody publishes a loader for one
        // 2005 engine — so the module that gets inside is Taarib's own, exactly
        // as for a game whose engine was never identified. Which slot that
        // takes is the question re4_tweaks makes real: `version.dll` here, and
        // `wakeel_qaim` refuses rather than fights for it if another mod already
        // holds it.
        AilatMuharrik::Bio4 | AilatMuharrik::Majhul => {
            HajatItar::Matlub(Box::new(MukawwinItar::mudkhal(hadaf, mimariya, fi_beea)))
        }
    }
}

/// Which platform's payload a game needs.
#[must_use]
const fn hadaf_hamula(nizam: NizamTashghil, beea: &BeeatTawafuq) -> NizamTashghil {
    if beea.windows_dakhilan() { NizamTashghil::Windows } else { nizam }
}

/// The component store's directory name for a payload target.
#[must_use]
const fn ism_hadaf(nizam: NizamTashghil) -> &'static str {
    match nizam {
        NizamTashghil::Windows => "windows",
        NizamTashghil::Linux => "linux",
        NizamTashghil::Mac => "mac",
    }
}

/// The loader file name and load mechanism for a target platform.
fn muhammil_windows_aw_unix(
    hadaf: NizamTashghil,
    beea: bool,
    wakeel: &str,
    asas: &str,
) -> (String, TahmilMusbaq) {
    match hadaf {
        NizamTashghil::Windows => {
            let tahmil = if beea {
                TahmilMusbaq::TajawuzWine { wahda: wakeel.to_owned() }
            } else {
                TahmilMusbaq::WakeelWindows { wahda: wakeel.to_owned() }
            };
            (format!("{wakeel}.dll"), tahmil)
        }
        NizamTashghil::Linux => {
            (NizamTashghil::Linux.ism_maktaba(asas), TahmilMusbaq::LdPreload)
        }
        NizamTashghil::Mac => {
            (NizamTashghil::Mac.ism_maktaba(asas), TahmilMusbaq::DyldInsert)
        }
    }
}

/// Where one deployed file lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MawqiTarkib {
    /// The game's own directory, where the executable sits at its root.
    JidhrLuba,

    /// A path under the game's directory.
    DakhilLuba(WajhatLuba),

    /// The same path expressed inside a compatibility prefix's `drive_c`,
    /// which is where a game installed by a prefix manager actually lives.
    DakhilBeea(WajhatNizam),
}

impl MawqiTarkib {
    /// The absolute path this destination names.
    ///
    /// The game-relative tail is resolved component by component with case
    /// folded, not joined verbatim. The tail is written the way the game's own
    /// files are named — `BepInEx/plugins`, `www/js/plugins`, `game/tl` — which
    /// is Windows spelling, and most games are on a case-sensitive host. Joining
    /// it literally next to a `BepInEx` that the game, or another mod manager,
    /// already created as `bepinex` produces a second directory differing only
    /// in case: the write succeeds, the install records a success, and the game
    /// loads nothing. The prefix arm has always folded case; this is the same
    /// question on the path most games actually take.
    ///
    /// Containment is still checked first, against the unfolded form, so the
    /// resolver can never be the thing that walks out of the game's directory.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] when the game-relative form does not join
    /// onto the root as a contained path.
    pub fn mutlaq(&self, jidhr_luba: &Path) -> Result<PathBuf, KhataTathbeet> {
        match self {
            Self::JidhrLuba => Ok(jidhr_luba.to_path_buf()),
            Self::DakhilLuba(wajha) => {
                masarat::dakhil(jidhr_luba, wajha.nisbi()).map_err(|khata| {
                    KhataTathbeet::MasarKharij {
                        masar: PathBuf::from(wajha.nisbi()),
                        jidhr: jidhr_luba.to_path_buf(),
                        sabab: khata.injilizi,
                    }
                })?;
                Ok(taarib_kashf::beea::hall_bila_hala(jidhr_luba, Path::new(wajha.nisbi())))
            }
            Self::DakhilBeea(wajha) => Ok(wajha.mutlaq()),
        }
    }

    /// Whether this destination is expressed through a compatibility prefix.
    #[must_use]
    pub const fn fi_beea(&self) -> bool {
        matches!(self, Self::DakhilBeea(_))
    }
}

impl fmt::Display for MawqiTarkib {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JidhrLuba => f.write_str("."),
            Self::DakhilLuba(wajha) => f.write_str(wajha.nisbi()),
            Self::DakhilBeea(wajha) => write!(f, "{}", wajha.mutlaq().display()),
        }
    }
}

/// A game as the installer resolved it, with everything the framework table
/// and the destination table read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LubaMuhallala {
    /// The game's root directory.
    pub jidhr: PathBuf,

    /// The game's main executable, which must be inside the root.
    pub masar_tanfidhi: PathBuf,

    /// The identified engine.
    pub muharrik: Muharrik,

    /// What the game runs behind.
    pub beea: BeeatTawafuq,

    /// The system Taarib is installing on.
    pub nizam: NizamTashghil,

    /// The launcher the game was found through, which decides whether a preload
    /// is carried by launch options or by the launch environment.
    pub masdar: MasdarLuba,
}

impl LubaMuhallala {
    /// The directory the executable sits in, relative to the game root, empty
    /// when the executable sits at the root itself.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::MasarKharij`] when the executable is not inside the
    /// game root, or names a component that cannot be written as UTF-8.
    pub fn mujallad_tanfidhi(&self) -> Result<String, KhataTathbeet> {
        let kharij = |sabab: &str| KhataTathbeet::MasarKharij {
            masar: self.masar_tanfidhi.clone(),
            jidhr: self.jidhr.clone(),
            sabab: sabab.to_owned(),
        };
        let Some(walid) = self.masar_tanfidhi.parent() else {
            return Err(kharij("the executable path has no parent directory"));
        };
        let Ok(baqi) = walid.strip_prefix(&self.jidhr) else {
            return Err(kharij(
                "the game's executable is not inside the game directory, and a framework is \
                 deployed beside the executable",
            ));
        };
        nisbi_nass(baqi).ok_or_else(|| {
            kharij("a directory between the game root and its executable is not valid UTF-8")
        })
    }
}

/// What the launcher and the launch environment already hold, so that a
/// recorded previous value is the one that was really there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HalatIdadat {
    /// The launcher's current per-game launch options, [`None`] when unset.
    pub khiyarat_tashghil: Option<String>,

    /// The current `WINEDLLOVERRIDES` for this game, [`None`] when unset.
    pub tajawuzat_dll: Option<String>,

    /// The current preload variable for this platform, [`None`] when unset.
    pub tahmil_musbaq: Option<String>,

    /// The launcher configuration file that would be overwritten from memory by
    /// a running launcher, when the caller knows which file that is.
    pub malaf_idadat_manassa: Option<PathBuf>,
}

/// One setting the deployment recorded, and the value the launcher integration
/// must actually write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdadMunaffadh {
    /// Where the setting lives.
    pub mahall: MahallIdad,

    /// What was there before, [`None`] when nothing was.
    pub qeema_sabiqa: Option<String>,

    /// What must be written for the framework to load.
    pub qeema_maktuba: String,
}

/// One file the deployment placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalafMunashar {
    /// Where it landed.
    pub mawqi: MawqiTarkib,

    /// Its size in bytes.
    pub hajm: u64,

    /// Its fingerprint, computed from the bytes that were written.
    pub basma: Basma,
}

/// A framework this installation deployed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarkibMunaffadh {
    /// The component's path inside the component store.
    pub mukawwin: String,

    /// The component's one-line description.
    pub wasf: String,

    /// The directory the loader landed in.
    pub mawqi_muhammil: MawqiTarkib,

    /// The loader itself, which is what makes the rest of it run.
    pub muhammil: MalafMunashar,

    /// Every file placed, loader included, in destination order.
    pub malaffat: Vec<MalafMunashar>,

    /// How many bytes were placed in total.
    pub hajm: u64,

    /// Every setting recorded so the framework is loaded, and the value the
    /// launcher integration must write.
    pub idadat: Vec<IdadMunaffadh>,

    /// Loader slots beside the game that another mod already held when this
    /// deployment ran, Taarib's own excluded — it refuses rather than sharing.
    ///
    /// Nothing here is Taarib's and nothing here is in the manifest, so an
    /// uninstall cannot reach any of it. It is recorded because the install
    /// report is the record of what the game looked like at the moment it was
    /// modified, and "there was already a mod in it" is part of that.
    pub huqn_mujawir: Vec<WakeelQaim>,
}

/// A framework that was already there when the installation ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TarkibQaim {
    /// The component the table would have deployed.
    pub mukawwin: String,

    /// The path whose presence proved a framework was already installed.
    pub alama: MawqiTarkib,

    /// The settings recorded so Taarib's own payload is loaded by the framework
    /// that is already there.
    pub idadat: Vec<IdadMunaffadh>,
}

/// What the framework step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatijatTarkib {
    /// A framework was deployed by this installation and is in the manifest.
    Nushira(Box<TarkibMunaffadh>),

    /// A framework was already present and was left exactly as it was. Nothing
    /// of it is in the manifest, so uninstalling cannot remove it.
    Mawjud(Box<TarkibQaim>),

    /// No framework is needed at all.
    LaHaja(SababLaHaja),
}

impl NatijatTarkib {
    /// Whether the framework this game needs was already installed by something
    /// other than Taarib.
    #[must_use]
    pub const fn mawjud_musbaqan(&self) -> bool {
        matches!(self, Self::Mawjud(_))
    }

    /// The settings the launcher integration must write, in either case that
    /// produced any.
    #[must_use]
    pub fn idadat(&self) -> &[IdadMunaffadh] {
        match self {
            Self::Nushira(munaffadh) => &munaffadh.idadat,
            Self::Mawjud(qaim) => &qaim.idadat,
            Self::LaHaja(_) => &[],
        }
    }

    /// How many bytes the framework step wrote into the game.
    #[must_use]
    pub fn hajm(&self) -> u64 {
        match self {
            Self::Nushira(munaffadh) => munaffadh.hajm,
            Self::Mawjud(_) | Self::LaHaja(_) => 0,
        }
    }

    /// The step as lines for the install report.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        match self {
            Self::Nushira(munaffadh) => {
                let mut sutur = Vec::with_capacity(munaffadh.malaffat.len().saturating_add(3));
                sutur.push(format!(
                    "framework: {} deployed to {} ({} file(s), {} byte(s))",
                    munaffadh.wasf,
                    munaffadh.mawqi_muhammil,
                    munaffadh.malaffat.len(),
                    munaffadh.hajm
                ));
                sutur.push(format!("  loader: {}", munaffadh.muhammil.mawqi));
                for idad in &munaffadh.idadat {
                    sutur.push(format!(
                        "  setting: {} -> {}",
                        idad.mahall.wasf(),
                        idad.qeema_maktuba
                    ));
                }
                for wakeel in &munaffadh.huqn_mujawir {
                    sutur.push(format!(
                        "  alongside: {} was already there and Taarib did not touch it",
                        wakeel.wasf_injilizi()
                    ));
                }
                sutur
            }
            Self::Mawjud(qaim) => {
                let mut sutur = Vec::with_capacity(qaim.idadat.len().saturating_add(1));
                sutur.push(format!(
                    "framework: {} is already installed at {}; Taarib deployed nothing and \
                     uninstalling will not remove it",
                    qaim.mukawwin, qaim.alama
                ));
                for idad in &qaim.idadat {
                    sutur.push(format!(
                        "  setting: {} -> {}",
                        idad.mahall.wasf(),
                        idad.qeema_maktuba
                    ));
                }
                sutur
            }
            Self::LaHaja(sabab) => {
                vec![format!("framework: none needed — {}", sabab.wasf_injilizi())]
            }
        }
    }
}

/// The compatibility prefix a game runs behind, proven usable.
///
/// # Errors
///
/// [`KhataTathbeet::BeeaMafquda`] when a prefix was reported and its root or its
/// `drive_c` is missing or is not a directory, when a prefix was reported for a
/// game running on Windows, or when Rosetta was reported off macOS. A framework
/// deployed into a prefix that has never been built is a framework nothing will
/// ever load, and it fails silently rather than loudly.
pub fn tahaqquq_beea(
    luba: &LubaMuhallala,
) -> Result<Option<(PathBuf, PathBuf)>, KhataTathbeet> {
    tahaqquq_beea_ajzaa(luba.nizam, &luba.beea, &luba.jidhr)
}

/// The same check, from the three facts it actually needs.
fn tahaqquq_beea_ajzaa(
    nizam: NizamTashghil,
    beea: &BeeatTawafuq,
    jidhr_luba: &Path,
) -> Result<Option<(PathBuf, PathBuf)>, KhataTathbeet> {
    match beea {
        BeeatTawafuq::Asli => Ok(None),

        BeeatTawafuq::Rosetta => {
            if matches!(nizam, NizamTashghil::Mac) {
                Ok(None)
            } else {
                Err(KhataTathbeet::BeeaMafquda {
                    jidhr: jidhr_luba.to_path_buf(),
                    sabab: "Rosetta was reported for a game that is not running on macOS, and \
                            no other system has it"
                        .to_owned(),
                })
            }
        }

        BeeatTawafuq::Proton { beea, .. } | BeeatTawafuq::Wine { beea, .. } => {
            if matches!(nizam, NizamTashghil::Windows) {
                return Err(KhataTathbeet::BeeaMafquda {
                    jidhr: beea.clone(),
                    sabab: "a Wine prefix was reported for a game running on Windows, where \
                            there is no prefix to install into"
                        .to_owned(),
                });
            }
            if !beea.is_dir() {
                return Err(KhataTathbeet::BeeaMafquda {
                    jidhr: beea.clone(),
                    sabab: "the prefix root does not exist or is not a directory".to_owned(),
                });
            }

            let alama = WajhatNizam::fi_beea(beea, "windows")?;
            let qurs = alama.jidhr().to_path_buf();
            if !qurs.is_dir() {
                return Err(KhataTathbeet::BeeaMafquda {
                    jidhr: beea.clone(),
                    sabab: format!(
                        "{} has no drive_c, so it is not a built prefix and a Windows \
                         framework placed against it would never be loaded",
                        beea.display()
                    ),
                });
            }
            Ok(Some((beea.clone(), qurs)))
        }
    }
}

/// The destination table for one game: every framework path, expressed the way
/// the platform the game runs on expresses it.
///
/// Resolved once, by [`khutta`], and carried inside the plan from there on. Two
/// tables built from two different readings of the same game is how a loader
/// came to be written into one directory while the plan beside it named
/// another — see [`KhuttatTarkib::mawadi`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct MawadiTarkib {
    jidhr_luba: PathBuf,
    mujallad_muhammil: String,
    beea: Option<PathBuf>,
    tayl_luba: Option<String>,
}

impl MawadiTarkib {
    /// Resolves the destinations from a game root, the directory the framework's
    /// loader belongs in, and the prefix if there is one.
    ///
    /// The one constructor, called from [`khutta`] and nowhere else. It used to
    /// have a sibling that read the loader directory off the executable, and
    /// having two was the defect: the plan resolved its destinations through one
    /// and the writer through the other.
    fn min_ajzaa(
        jidhr_luba: &Path,
        mujallad_muhammil: String,
        qurs: Option<(&Path, &Path)>,
    ) -> Self {
        let (beea, tayl_luba) = match qurs {
            // Only a game that really lives inside the prefix gets prefix-shaped
            // destinations; a Proton game sits outside `pfx` and keeps its own.
            Some((beea, qurs)) => match jidhr_luba
                .strip_prefix(qurs)
                .ok()
                .and_then(nisbi_nass)
                .filter(|tayl| !tayl.is_empty())
            {
                Some(tayl) => (Some(beea.to_path_buf()), Some(tayl)),
                None => (None, None),
            },
            None => (None, None),
        };
        Self {
            jidhr_luba: jidhr_luba.to_path_buf(),
            mujallad_muhammil,
            beea,
            tayl_luba,
        }
    }

    /// One game-relative path as a destination.
    fn wajha(&self, nisbi: &str) -> Result<MawqiTarkib, KhataTathbeet> {
        if nisbi.is_empty() {
            return Ok(MawqiTarkib::JidhrLuba);
        }
        if let (Some(beea), Some(tayl)) = (self.beea.as_ref(), self.tayl_luba.as_ref()) {
            return Ok(MawqiTarkib::DakhilBeea(WajhatNizam::fi_beea(
                beea,
                &format!("{tayl}/{nisbi}"),
            )?));
        }
        Ok(MawqiTarkib::DakhilLuba(WajhatLuba::jadeed(nisbi)?))
    }

    /// A path beside the game's executable, relative to the game root.
    fn bijanib(&self, dhayl: &str) -> String {
        if self.mujallad_muhammil.is_empty() {
            dhayl.to_owned()
        } else {
            format!("{}/{dhayl}", self.mujallad_muhammil)
        }
    }

    /// A path under the game's own Taarib directory, relative to the game root.
    fn taht_taarib(dhayl: &str) -> String {
        format!("{MUJALLAD_TAARIB}/{dhayl}")
    }

    /// Where one file of a component lands, from its path inside the store.
    fn wajhat_malaf(&self, tawzi: TawziMukawwin, fi_makhzan: &str) -> String {
        match tawzi {
            TawziMukawwin::Wahid => self.bijanib(fi_makhzan),
            TawziMukawwin::Munfasil => fi_makhzan
                .strip_prefix(MUJALLAD_MUHAMMIL)
                .and_then(|baqi| baqi.strip_prefix('/'))
                .map_or_else(|| Self::taht_taarib(fi_makhzan), |baqi| self.bijanib(baqi)),
        }
    }

    /// The deployment root, where the loader lands.
    fn jidhr_muhammil(&self) -> Result<MawqiTarkib, KhataTathbeet> {
        self.wajha(&self.mujallad_muhammil)
    }

    /// The game root this table resolves against.
    fn jidhr_luba(&self) -> &Path {
        &self.jidhr_luba
    }
}

/// Reads one framework component out of Taarib's own component store.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when the component directory is absent, is
/// not a directory, or holds no regular file. [`KhataTathbeet::HajmMufrit`] when
/// the component exceeds [`AQSA_HAJM_MUKAWWIN`].
/// [`KhataTathbeet::MasarKharij`] when the component name does not join onto the
/// store root as a contained path, and the I/O variants when the store cannot be
/// walked or one of its files cannot be read.
pub fn hamil_mukawwin(
    jidhr_makhzan: &Path,
    ism: &str,
) -> Result<Vec<(String, Vec<u8>)>, KhataTathbeet> {
    let jidhr = masarat::dakhil(jidhr_makhzan, ism).map_err(|khata| {
        KhataTathbeet::MasarKharij {
            masar: PathBuf::from(ism),
            jidhr: jidhr_makhzan.to_path_buf(),
            sabab: khata.injilizi,
        }
    })?;
    if !jidhr.is_dir() {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr,
        });
    }
    // Before a byte is read: the store's manifest is the only thing that knows
    // how many files this component should have. Walking the directory answers
    // "is anything here", which an interrupted mirror also answers yes to.
    crate::bayan_makhzan::kamil_hasab_bayan(jidhr_makhzan, ism)?;

    let mut malaffat: Vec<(String, Vec<u8>)> = Vec::new();
    let mut majmu = 0_u64;
    for madkhal in WalkDir::new(&jidhr) {
        let madkhal = madkhal.map_err(|sabab| {
            let masar = sabab.path().map_or_else(|| jidhr.clone(), Path::to_path_buf);
            let sabab = sabab.into_io_error().unwrap_or_else(|| {
                std::io::Error::other("the component store could not be walked")
            });
            min_khata_io(&masar, "reading Taarib's component store", sabab)
        })?;
        // Symbolic links are skipped rather than followed: a component is bytes
        // in the store, not a link to bytes somewhere else on the machine.
        if !madkhal.file_type().is_file() {
            continue;
        }
        let Some(nisbi) = madkhal.path().strip_prefix(&jidhr).ok().and_then(nisbi_nass) else {
            return Err(KhataTathbeet::MukawwinMafqud {
                mukawwin: ism.to_owned(),
                masar: madkhal.path().to_path_buf(),
            });
        };
        if nisbi.is_empty() {
            continue;
        }
        // The declared size is checked before a byte is read, so a pathological
        // component is a refusal naming the number rather than an exhausted
        // machine.
        let muallan = madkhal
            .metadata()
            .map_err(|sabab| {
                let sabab = sabab.into_io_error().unwrap_or_else(|| {
                    std::io::Error::other("the component's size could not be read")
                });
                min_khata_io(madkhal.path(), "sizing a framework component", sabab)
            })?
            .len();
        majmu = majmu.saturating_add(muallan);
        if majmu > AQSA_HAJM_MUKAWWIN {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "framework component",
                qeema: majmu,
                saqf: AQSA_HAJM_MUKAWWIN,
            });
        }
        malaffat.push((nisbi, fs_qira(madkhal.path())?));
    }

    if malaffat.is_empty() {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr,
        });
    }
    malaffat.sort_by(|awwal, thani| awwal.0.cmp(&thani.0));
    Ok(malaffat)
}

/// Reads one file of the component store.
fn fs_qira(masar: &Path) -> Result<Vec<u8>, KhataTathbeet> {
    std::fs::read(masar)
        .map_err(|sabab| min_khata_io(masar, "reading a framework component", sabab))
}

/// A relative path as the manifest and the destination tables spell it.
fn nisbi_nass(masar: &Path) -> Option<String> {
    let mut nateeja = String::new();
    for juz in masar.components() {
        let Component::Normal(ism) = juz else { return None };
        let nass = ism.to_str()?;
        if !nateeja.is_empty() {
            nateeja.push('/');
        }
        nateeja.push_str(nass);
    }
    Some(nateeja)
}

/// Adds one module to a `WINEDLLOVERRIDES` value, keeping every other override
/// the prefix already had.
#[must_use]
pub fn damj_tajawuz(hali: Option<&str>, wahda: &str) -> String {
    let matlub = wahda.to_ascii_lowercase();
    let mut mudkhalat: Vec<String> = Vec::new();
    for mudkhal in hali.unwrap_or_default().split(';') {
        let munaqqa = mudkhal.trim();
        if munaqqa.is_empty() {
            continue;
        }
        let asmaa = munaqqa.split_once('=').map_or(munaqqa, |(asmaa, _)| asmaa);
        let yakhussuna = asmaa.split(',').any(|ism| {
            ism.trim().trim_start_matches('*').to_ascii_lowercase() == matlub
        });
        if !yakhussuna {
            mudkhalat.push(munaqqa.to_owned());
        }
    }
    mudkhalat.push(format!("{wahda}={TARTEEB_ASLI}"));
    mudkhalat.join(";")
}

/// Adds one path to a colon-separated preload variable, leaving what the user
/// already preloads in place and in order.
#[must_use]
pub fn damj_tahmil(hali: Option<&str>, masar: &str) -> String {
    let mut ajza: Vec<&str> = hali
        .unwrap_or_default()
        .split(':')
        .map(str::trim)
        .filter(|juz| !juz.is_empty())
        .collect();
    if ajza.contains(&masar) {
        return ajza.join(":");
    }
    ajza.push(masar);
    ajza.join(":")
}

/// Puts one environment assignment in front of a launcher's launch options.
#[must_use]
pub fn damj_khiyarat(hali: Option<&str>, isnad: &str) -> String {
    let munaqqa = hali.unwrap_or_default().trim();
    if munaqqa.is_empty() {
        return format!("{isnad} {RAMZ_AMR}");
    }
    match munaqqa.split_once(RAMZ_AMR) {
        Some((qabl, baad)) => {
            let qabl = qabl.trim();
            if qabl.is_empty() {
                format!("{isnad} {RAMZ_AMR}{baad}")
            } else {
                format!("{qabl} {isnad} {RAMZ_AMR}{baad}")
            }
        }
        None => format!("{isnad} {RAMZ_AMR} {munaqqa}"),
    }
}

/// Quotes an environment assignment for a launch-options field, which is parsed
/// as a shell command line.
fn isnad_muqtabas(ism: &str, qeema: &str) -> String {
    format!("{ism}=\"{}\"", qeema.replace('"', "\\\""))
}

/// The settings that make a deployed framework load, with the values that were
/// really there before them.
fn idadat_tahmil(
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    tahmil: &TahmilMusbaq,
    masar_muhammil: &Path,
) -> Result<Vec<IdadMunaffadh>, KhataTathbeet> {
    let Some(ism) = tahmil.mutaghayyir() else { return Ok(Vec::new()) };

    let (qeema_sabiqa, qeema) = match tahmil {
        TahmilMusbaq::WakeelWindows { .. } => return Ok(Vec::new()),
        TahmilMusbaq::TajawuzWine { wahda } => (
            halat.tajawuzat_dll.clone(),
            damj_tajawuz(halat.tajawuzat_dll.as_deref(), wahda),
        ),
        TahmilMusbaq::LdPreload | TahmilMusbaq::DyldInsert => {
            let Some(nass) = masar_muhammil.to_str() else {
                return Err(KhataTathbeet::MasarKharij {
                    masar: masar_muhammil.to_path_buf(),
                    jidhr: luba.jidhr.clone(),
                    sabab: format!(
                        "the loader's path is not valid UTF-8 and cannot be named in {ism}"
                    ),
                });
            };
            (
                halat.tahmil_musbaq.clone(),
                damj_tahmil(halat.tahmil_musbaq.as_deref(), nass),
            )
        }
    };

    // Steam carries a per-game environment in its launch options field and
    // nowhere else; every other launcher carries one in its own configuration.
    if matches!(luba.masdar, MasdarLuba::Steam(_)) {
        return Ok(vec![IdadMunaffadh {
            mahall: MahallIdad::KhiyaratTashghil {
                manassa: luba.masdar.ism_injilizi().to_owned(),
                muarrif_luba: luba.masdar.muarrif(),
            },
            qeema_maktuba: damj_khiyarat(
                halat.khiyarat_tashghil.as_deref(),
                &isnad_muqtabas(ism, &qeema),
            ),
            qeema_sabiqa: halat.khiyarat_tashghil.clone(),
        }]);
    }

    Ok(vec![IdadMunaffadh {
        mahall: MahallIdad::MutaghayyirBeea { ism: ism.to_owned() },
        qeema_sabiqa,
        qeema_maktuba: qeema,
    }])
}

/// Refuses when a launch-options setting is about to be recorded and the
/// launcher that owns the file is running — or cannot be seen at all.
///
/// The guard is `itlaq`'s rather than a second copy of it: this writes the same
/// `localconfig.vdf` that `itlaq::KhiyaratSteam` writes, and a check that read
/// a sandbox's empty process list as "Steam is closed" would hand Steam an
/// edited file to overwrite from memory on exit.
fn tahaqquq_manassa(
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    idadat: &[IdadMunaffadh],
) -> Result<(), KhataTathbeet> {
    let yaktub_khiyarat = idadat
        .iter()
        .any(|idad| matches!(idad.mahall, MahallIdad::KhiyaratTashghil { .. }));
    if !yaktub_khiyarat {
        return Ok(());
    }
    // Without the file's own path there is nothing to name and nothing to
    // claim would be overwritten, so the check is the caller's to enable.
    let Some(malaf) = halat.malaf_idadat_manassa.as_ref() else { return Ok(()) };
    manassa_mughlaqa(&ASMAA_STEAM, luba.masdar.ism_injilizi(), malaf)
}

/// Records every setting, in order, before any of them is written by anyone.
fn sajjil_idadat(
    muthabbit: &mut dyn Muthabbit,
    idadat: &[IdadMunaffadh],
) -> Result<(), KhataTathbeet> {
    for idad in idadat {
        muthabbit.sajjil_idad(
            idad.mahall.clone(),
            idad.qeema_sabiqa.clone(),
            Some(idad.qeema_maktuba.clone()),
        )?;
    }
    Ok(())
}

/// One destination of a component, resolved every way it is needed.
#[derive(Debug)]
struct WajhatMalaf {
    nisbi: String,
    mawqi: MawqiTarkib,
    mutlaq: PathBuf,
}

/// The first name at the deployment root that proves a framework of this kind
/// is already installed.
///
/// The component's own loader file name is deliberately **not** one of the
/// names asked about, even though [`MukawwinItar::alamat`] carries it. A file at
/// that path is proof that this framework is installed only when the
/// framework's other markers are beside it — a `BepInEx/` directory next to a
/// `winhttp.dll` is BepInEx; a `winhttp.dll` on its own is some other mod
/// holding the same slot. Answering the second case with "already installed,
/// deploy nothing" produces an install that reports success, writes no loader,
/// and leaves Taarib's payload sitting beside a proxy that will never open it.
/// That case is [`wakeel_qaim`]'s, and it is a refusal.
///
/// Taarib's own component has no other marker at all, so this always answers
/// [`None`] for it and the slot question is the only question.
fn itar_qaim(
    mawadi: &MawadiTarkib,
    mukawwin: &MukawwinItar,
) -> Result<Option<MawqiTarkib>, KhataTathbeet> {
    for alama in &mukawwin.alamat {
        if alama == &mukawwin.ism_muhammil {
            continue;
        }
        let nisbi = mawadi.bijanib(alama);
        let mawqi = mawadi.wajha(&nisbi)?;
        if mawqi.mutlaq(mawadi.jidhr_luba())?.exists() {
            return Ok(Some(mawqi));
        }
    }
    Ok(None)
}

/// What `min_khata_io` is told this crate was doing when a loader-slot survey
/// fails.
const AMAL_MASAH: &str = "surveying the loader slots beside the game's executable";

/// Every loader slot in use in the directory a framework's loader lands in.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] and its siblings when the directory cannot be
/// read. A directory that is not there is not a failure — see
/// [`wukala::masah`].
fn masah_huqn(mujallad: &Path) -> NatijatTathbeet<Vec<WakeelQaim>> {
    wukala::masah(mujallad).map_err(|sabab| min_khata_io(mujallad, AMAL_MASAH, sabab))
}

/// Refuses when the module name Taarib's own loader is published as is already
/// held by a file Taarib did not put there.
///
/// The slot is exclusive. Windows resolves a module name to one file, and the
/// one it resolves to is whichever is beside the executable — so a second
/// `version.dll` written over the first does not chain onto it, it erases it.
/// The mod that was there stops loading, its own configuration and its own
/// files stay in the game directory pointing at nothing, and no message anywhere
/// says what happened. `taarib-haqn` already refuses to restore a hook slot
/// another overlay has taken, for the same reason and with the same conclusion:
/// whoever took the slot last is the only one who can give it back.
///
/// The survey it returns on success is not a by-product. A game with another
/// mod's `dinput8.dll` in it is a game whose behaviour is not the publisher's
/// any more, and the person agreeing to an install is entitled to know that
/// before they agree rather than after.
///
/// # Errors
///
/// [`KhataTathbeet::WakeelMashghul`] when the loader's own path is occupied, and
/// whatever reading the directory raises.
fn wakeel_qaim(
    mawadi: &MawadiTarkib,
    mukawwin: &MukawwinItar,
) -> NatijatTathbeet<Vec<WakeelQaim>> {
    let jidhr = mawadi.jidhr_muhammil()?.mutlaq(mawadi.jidhr_luba())?;
    let qaima = masah_huqn(&jidhr)?;

    let nisbi = mawadi.bijanib(&mukawwin.ism_muhammil);
    let masar = mawadi.wajha(&nisbi)?.mutlaq(mawadi.jidhr_luba())?;
    // `is_file` rather than `exists`: a directory carrying the loader's name is
    // not something any loader can map, and refusing an install over one would
    // be refusing over a collision that cannot happen.
    if !masar.is_file() {
        return Ok(qaima);
    }

    let hajm = std::fs::metadata(&masar).map_or(0, |bayan| bayan.len());
    // The survey has already identified everything it found, so the occupant is
    // looked up rather than re-examined. It is absent from the survey only when
    // the loader is not a proxy slot at all — the Linux and macOS builds, whose
    // loader is a shared object with no system name to stand in for — and an
    // unidentified answer is the right one for a file that was never a proxy.
    let huwiya = wukala::shaghil_slot(&qaima, &mukawwin.ism_muhammil)
        .map_or(wukala::HuwiyatWakeel::Majhul { mahzum: false }, |qaim| qaim.huwiya.clone());
    Err(KhataTathbeet::WakeelMashghul {
        wakeel: mukawwin.ism_muhammil.clone(),
        masar,
        hajm,
        huwiya,
        jiran: qaima
            .iter()
            .filter(|wakeel| !wakeel.ism.eq_ignore_ascii_case(&mukawwin.ism_muhammil))
            .map(WakeelQaim::wasf_injilizi)
            .collect(),
    })
}

/// Installs the framework **the plan decided on**, on this platform.
///
/// It takes the plan rather than the game because the two questions the plan
/// answers — whether a framework is deployed at all, and where its loader lands
/// — are questions the engine alone cannot answer. This function used to ask
/// [`hajat_itar`] instead, which reads the engine and has no tier to consult, so
/// a tier-3 game whose plan said "no framework needed" had `version.dll` and its
/// payloads written into it anyway.
///
/// # Errors
///
/// [`KhataTathbeet::BeeaMafquda`] when the game runs behind a compatibility
/// prefix that has no `drive_c`, because a Windows framework deployed against an
/// unbuilt prefix is never loaded and never says so.
/// [`KhataTathbeet::MukawwinMafqud`] when the component store does not hold the
/// component the table names, or holds it without the loader that makes it run.
/// [`KhataTathbeet::MunassaTaamal`] when a launch-options setting must be
/// recorded and the launcher that rewrites that file on exit is running, and
/// [`KhataTathbeet::HalatManassaMajhula`] when a sandbox leaves this build
/// unable to see whether it is.
/// [`KhataTathbeet::MasarKharij`] when the executable is not inside the game
/// root or a destination does not stay inside it, [`KhataTathbeet::HajmMufrit`]
/// for a component above [`AQSA_HAJM_MUKAWWIN`], and whatever the recorder
/// raises for a file it cannot preserve, add or record.
pub fn rakkib_itar(
    mukhattat: &KhuttatTarkib,
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    jidhr_makhzan: &Path,
    muthabbit: &mut dyn Muthabbit,
) -> Result<NatijatTarkib, KhataTathbeet> {
    match &mukhattat.hajat {
        HajatItar::LaHaja(sabab) => Ok(NatijatTarkib::LaHaja(*sabab)),
        HajatItar::Matlub(mukawwin) => {
            rakkib_mukawwin(luba, halat, jidhr_makhzan, mukawwin, &mukhattat.mawadi, muthabbit)
        }
    }
}

/// Deploys one already-chosen component, into the destinations the plan already
/// resolved.
///
/// Neither half of that sentence is decoration. "Which framework" is the plan's
/// answer because [`hajat_itar`] reads the engine alone while [`khutta`] reads
/// the engine *and* the tier — tier 3 deploys nothing whatever the engine is,
/// and a GameMaker game needs the loader at tier 2 and not at tier 1. "Where"
/// is the plan's answer for the same kind of reason: the loader directory used
/// to be resolved here from the executable and there from the engine, and the
/// launch requirement the plan recorded then named a path this writer had not
/// written to.
fn rakkib_mukawwin(
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    jidhr_makhzan: &Path,
    mukawwin: &MukawwinItar,
    mawadi: &MawadiTarkib,
    muthabbit: &mut dyn Muthabbit,
) -> Result<NatijatTarkib, KhataTathbeet> {
    let mawqi_muhammil = mawadi.jidhr_muhammil()?;
    let jidhr_mutlaq = mawqi_muhammil.mutlaq(mawadi.jidhr_luba())?;
    let idadat = idadat_tahmil(
        luba,
        halat,
        &mukawwin.tahmil,
        &jidhr_mutlaq.join(&mukawwin.ism_muhammil),
    )?;
    tahaqquq_manassa(luba, halat, &idadat)?;

    // Two questions in this order, and the order is the whole point. "Is this
    // same framework already here" is coexistence and deploys nothing; "is
    // Taarib's own loader slot held by something else" is a collision and
    // refuses. Asking them the other way round would answer a foreign proxy
    // with "already installed", which is how an install comes to report success
    // having written a payload that nothing will ever load.
    if let Some(alama) = itar_qaim(mawadi, mukawwin)? {
        sajjil_idadat(muthabbit, &idadat)?;
        return Ok(NatijatTarkib::Mawjud(Box::new(TarkibQaim {
            mukawwin: mukawwin.ism.clone(),
            alama,
            idadat,
        })));
    }
    let huqn = wakeel_qaim(mawadi, mukawwin)?;

    let hamula = hamil_mukawwin(jidhr_makhzan, &mukawwin.ism)?;
    let nisbi_muhammil = mawadi.bijanib(&mukawwin.ism_muhammil);

    let mut wajhat: Vec<WajhatMalaf> = Vec::with_capacity(hamula.len());
    for (fi_makhzan, _) in &hamula {
        let nisbi = mawadi.wajhat_malaf(mukawwin.tawzi, fi_makhzan);
        let mawqi = mawadi.wajha(&nisbi)?;
        let mutlaq = mawqi.mutlaq(mawadi.jidhr_luba())?;
        wajhat.push(WajhatMalaf { nisbi, mawqi, mutlaq });
    }

    // A component whose loader is not in it deploys files that nothing will ever
    // load, and the game runs untouched with no error anywhere.
    if !wajhat.iter().any(|wajha| wajha.nisbi == nisbi_muhammil) {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: format!("{}/{}", mukawwin.ism, mukawwin.ism_muhammil),
            masar: jidhr_makhzan.join(&mukawwin.ism),
        });
    }

    // The whole set is checked before one byte is written: a destination that is
    // already occupied is somebody else's file, and finding the first of them
    // half way through would leave a framework neither installed nor absent.
    if let Some(qaim) = wajhat.iter().find(|wajha| wajha.mutlaq.exists()) {
        sajjil_idadat(muthabbit, &idadat)?;
        return Ok(NatijatTarkib::Mawjud(Box::new(TarkibQaim {
            mukawwin: mukawwin.ism.clone(),
            alama: qaim.mawqi.clone(),
            idadat,
        })));
    }

    if !matches!(mawqi_muhammil, MawqiTarkib::JidhrLuba) {
        muthabbit.ansha_mujallad(&jidhr_mutlaq)?;
    }

    let mut malaffat: Vec<MalafMunashar> = Vec::with_capacity(wajhat.len());
    let mut muhammil: Option<MalafMunashar> = None;
    let mut hajm = 0_u64;
    for (wajha, (_, bayt)) in wajhat.iter().zip(hamula.iter()) {
        muthabbit.ansha(&wajha.mutlaq, bayt)?;
        let munashar = MalafMunashar {
            mawqi: wajha.mawqi.clone(),
            hajm: tul_u64(bayt.len()),
            basma: basma_bayt(bayt),
        };
        hajm = hajm.saturating_add(munashar.hajm);
        if wajha.nisbi == nisbi_muhammil {
            muhammil = Some(munashar.clone());
        }
        malaffat.push(munashar);
    }

    let Some(muhammil) = muhammil else {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: format!("{}/{}", mukawwin.ism, mukawwin.ism_muhammil),
            masar: jidhr_makhzan.join(&mukawwin.ism),
        });
    };

    sajjil_idadat(muthabbit, &idadat)?;

    Ok(NatijatTarkib::Nushira(Box::new(TarkibMunaffadh {
        mukawwin: mukawwin.ism.clone(),
        wasf: format!("{} — {}", mukawwin.wasf, mukawwin.tahmil.wasf()),
        mawqi_muhammil,
        muhammil,
        malaffat,
        hajm,
        idadat,
        huqn_mujawir: huqn,
    })))
}

// The additive layer: what an engine needs besides — or instead of — a
// framework

/// A launch-time change a deployment needs, stated without naming a launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalabItlaq {
    /// The game must be started through a command the launcher runs instead of
    /// the plain executable — `sh ./run_bepinex.sh %command%` and its
    /// relatives.
    KhiyarTashghil {
        /// The command, with `%command%` where the launcher substitutes the
        /// game's own command line.
        qeema: String,
        /// Why the framework does not load without it.
        sabab: &'static str,
    },

    /// The game's process must start with an environment variable set.
    MutaghayyirBeea {
        /// The variable's name.
        ism: &'static str,
        /// The value, with any path in it relative to the game root so that
        /// the requirement survives the game being moved to another drive.
        qeema: String,
        /// Why the framework does not load without it.
        sabab: &'static str,
    },

    /// Taarib's external overlay must attach when the game launches.
    TashgheelLawha {
        /// Why the overlay is the mechanism for this game.
        sabab: &'static str,
    },
}

impl TalabItlaq {
    /// Where this requirement is recorded, once a caller supplies the launcher
    /// the game was found through.
    #[must_use]
    pub fn mahall(&self, masdar: &MasdarLuba) -> Option<MahallIdad> {
        let khiyarat = || MahallIdad::KhiyaratTashghil {
            manassa: masdar.ism_injilizi().to_owned(),
            muarrif_luba: masdar.muarrif(),
        };
        match self {
            Self::KhiyarTashghil { .. } => Some(khiyarat()),
            Self::MutaghayyirBeea { ism, .. } => Some(if matches!(masdar, MasdarLuba::Steam(_)) {
                khiyarat()
            } else {
                MahallIdad::MutaghayyirBeea { ism: (*ism).to_owned() }
            }),
            Self::TashgheelLawha { .. } => None,
        }
    }

    /// One line for the confirmation screen, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::KhiyarTashghil { qeema, sabab } => {
                format!("the game must be launched as `{qeema}` — {sabab}")
            }
            Self::MutaghayyirBeea { ism, qeema, sabab } => {
                format!("the game must start with {ism}={qeema} — {sabab}")
            }
            Self::TashgheelLawha { sabab } => {
                format!("the Taarib overlay attaches at launch — {sabab}")
            }
        }
    }

    /// One line for the confirmation screen, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::KhiyarTashghil { qeema, .. } => format!(
                "يحتاج التشغيل إلى تعديل خيارات الإطلاق لتنفيذ «{qeema}». يكتبه تعريب عند \
                 التثبيت ويعيده كما كان عند الإزالة."
            ),
            Self::MutaghayyirBeea { ism, qeema, .. } => format!(
                "يحتاج التشغيل إلى ضبط المتغيّر {ism} على «{qeema}». يكتبه تعريب عند التثبيت \
                 ويعيده كما كان عند الإزالة."
            ),
            Self::TashgheelLawha { .. } => {
                "تُعرض العربية من طبقة تعريب الخارجية عند تشغيل اللعبة، دون تعديل اللعبة نفسها."
                    .to_owned()
            }
        }
    }
}

/// Whether a planned file is added to the game or replaces one it shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawMudkhal {
    /// A file the game did not have.
    Idafa,
    /// A file the game already has.
    Tadeel,
}

/// Which loader registry a registration edit understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawTasjeel {
    /// RPG Maker's `plugins.js` — the `$plugins` array nwjs reads at boot.
    PluginsJs,
    /// Godot 3's `override.cfg` beside the executable, which the engine reads
    /// before the pack is mounted and which is therefore the one settings
    /// surface reachable without rewriting the pack.
    OverrideCfg,
}

/// Where a planned file's bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasdarMudkhal {
    /// Bytes copied verbatim from one file of a component.
    MinMakhzan {
        /// The component's path inside the store.
        mukawwin: String,
        /// The file's path inside that component.
        fi_makhzan: String,
    },

    /// Text generated by this module, such as `taarib.gdnlib`.
    NassMuwallad {
        /// The complete text.
        nass: String,
        /// What it is, for the confirmation screen.
        wasf: &'static str,
    },

    /// A registration appended into a file the game owns, computed from that
    /// file's bytes *at write time* — never at plan time, because an edit
    /// computed against a stale read overwrites whatever changed in between.
    TasjeelIdafi {
        /// Which registry format the edit understands.
        naw: NawTasjeel,
    },
}

/// One directory the additive layer needs before its files can be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MujalladTarkib {
    /// The destination, resolved the way this game's platform expresses it.
    pub mawqi: MawqiTarkib,
    /// The same destination relative to the game root, for reports.
    pub nisbi: String,
}

/// One planned file: where it goes, whether it adds or replaces, and where its
/// bytes come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MudkhalTarkib {
    /// The destination, resolved the way this game's platform expresses it.
    pub mawqi: MawqiTarkib,
    /// The same destination relative to the game root, for reports.
    pub nisbi: String,
    /// Addition or modification.
    pub naw: NawMudkhal,
    /// The bytes.
    pub masdar: MasdarMudkhal,
}

/// The tier decision one installation writes under.
///
/// Minted only from a capability report the safety layer did not refuse, so
/// *holding one* and *this game may be written to at all* are the same
/// statement. Every write on the install path takes one — from a
/// [`KhuttatTarkib`] where there is a plan, and on its own where there is not
/// — because the alternative is what every instance of this defect was: a
/// writer re-deriving the answer from the game's own directory, which cannot
/// see a tier and cannot see a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QararTabaqa {
    tabaqa: Tabaqa,
}

impl QararTabaqa {
    /// The decision a capability report carries, when it carries one.
    ///
    /// # Errors
    ///
    /// [`KhataTathbeet::IdhnGhayrMutabiq`] when the report says the safety
    /// layer refuses this game. This is the single place that reading is done,
    /// so a caller cannot obtain a decision for a refused game and therefore
    /// cannot write into one.
    pub const fn min_taqreer(taqreer: &TaqreerImkaniyat) -> NatijatTathbeet<Self> {
        if taqreer.marfuda {
            return Err(KhataTathbeet::IdhnGhayrMutabiq);
        }
        Ok(Self { tabaqa: taqreer.tabaqa })
    }

    /// The tier itself, for a report or a confirmation screen.
    #[must_use]
    pub const fn tabaqa(self) -> Tabaqa {
        self.tabaqa
    }

    /// Whether the game's own files may be changed at all.
    ///
    /// False at tier 3 and nowhere else. The overlay tier's product surface
    /// says «the game is not modified at all», and this is the one predicate
    /// that sentence is worth.
    #[must_use]
    pub const fn tughayyar_al_luba(self) -> bool {
        !matches!(self.tabaqa, Tabaqa::TarjamaFawqiya)
    }
}

/// The whole deployment plan for one game, decided before anything is written.
///
/// It carries one private field — the destination table it resolved — and that
/// is what makes it a plan rather than a description: no caller outside this
/// module can build one, so the only way to hold a `KhuttatTarkib` is to have
/// had [`khutta`] decide it, the tier, the safety refusal and the loader
/// directory included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhuttatTarkib {
    /// The engine family the plan was built for.
    pub aila: AilatMuharrik,

    /// The tier the capability report assigned, which decides whether the
    /// game is modified at all.
    pub tabaqa: Tabaqa,

    /// What the framework table says: a component to deploy, or a named
    /// reason none is needed.
    pub hajat: HajatItar,

    /// Where the framework's loader would land, when one is deployed.
    pub jidhr_muhammil: Option<MawqiTarkib>,

    /// Directories the additive layer needs, in creation order.
    pub mujalladat: Vec<MujalladTarkib>,

    /// The additive layer's files, in write order.
    pub mudkhalat: Vec<MudkhalTarkib>,

    /// The Arabic face this plan deploys into a Ren'Py game, named the way the
    /// generated `.rpy` must name it: relative to `game/`.
    ///
    /// [`None`] for every other engine, and for a Ren'Py component that ships
    /// no face — in which case the generated settings register the direction
    /// and the translation and leave the game's own font variables alone.
    ///
    /// It is in the plan because the *text* write needs it before this plan is
    /// executed. `nusus::raqqi_nusus` runs before `nashr`, deliberately — see
    /// [`crate::masar_tathbeet::thabbit`] — so the name has to be knowable from
    /// the store ahead of the deployment that places it. [`khatt_renpy`] is
    /// that answer, asked here as the plan is built and asked again by the text
    /// write, over the same component listing and through the same selector.
    /// The two therefore cannot disagree about which file this is.
    pub khatt_renpy: Option<String>,

    /// Launch-time requirements, for `itlaq` to perform and record. Never
    /// performed by this module.
    pub talabat: Vec<TalabItlaq>,

    /// Loader slots beside the game that a third-party mod already holds, read
    /// before anything is written.
    ///
    /// This is the field a confirmation screen has to show. A game with
    /// `re4_tweaks` or `ReShade` or an ASI loader already in it is not the game the
    /// publisher shipped, and somebody agreeing to "install Arabic into this
    /// game" is agreeing to a different thing than they think if nobody tells
    /// them what else is in there. Taarib does not remove any of it and an
    /// uninstall cannot reach it — see [`TarkibMunaffadh::huqn_mujawir`] — but
    /// the disclosure belongs *before* the install, not in the report after it.
    ///
    /// Empty is the normal answer. When Taarib's own slot is among these the
    /// install refuses outright with [`KhataTathbeet::WakeelMashghul`] rather
    /// than listing it here, so a plan that carries entries is always a plan
    /// that can still go ahead.
    ///
    /// The directory surveyed is the one the framework's loader would land in,
    /// which is beside the game's executable. For a tier-3 game — no framework,
    /// the overlay attaches from outside — it is the game's root, because there
    /// is no loader directory to speak of.
    pub huqn_qaim: Vec<WakeelQaim>,

    /// What is known about the launcher that owns this game's launch options,
    /// when the plan needs a launch-time change and that knowledge is worth
    /// putting in front of the user.
    ///
    /// [`None`] means the launcher was *seen* not running. It never means the
    /// question went unanswered — that is [`MalhuzatManassa`] carrying
    /// [`HalatTashghil::GhayrMaaruf`], because a plan that omits the note
    /// because it could not look reads exactly like a plan with nothing to
    /// warn about.
    pub manassa_taamil: Option<MalhuzatManassa>,

    /// The destination table this plan resolved, handed to the writers rather
    /// than resolved a second time by them.
    ///
    /// Private, and that is deliberate twice over. It is what stops a plan
    /// being fabricated outside this module, and it is what stops the framework
    /// writer answering "which directory does the loader go in" from a
    /// different function than the plan did — the divergence that let a
    /// recorded `LD_PRELOAD` name a path no loader had been written to.
    mawadi: MawadiTarkib,
}

/// What the plan learned about the launcher that owns a game's launch options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalhuzatManassa {
    /// The launcher, as a person reading the plan knows it.
    pub ism: String,
    /// Running, or unseeable. [`HalatTashghil::LaTashtaghil`] never reaches
    /// here: there is nothing to say about a launcher that is closed.
    pub hala: HalatTashghil,
}

impl KhuttatTarkib {
    /// The decision this plan was built under.
    ///
    /// The plan is the long form of it; this is the short form every write that
    /// is not a deployment takes. They cannot disagree, because a plan exists
    /// only where [`khutta`] already refused a report the safety layer refused.
    #[must_use]
    pub const fn qarar(&self) -> QararTabaqa {
        QararTabaqa { tabaqa: self.tabaqa }
    }

    /// Whether the plan writes nothing at all into the game.
    #[must_use]
    pub const fn faragha(&self) -> bool {
        self.mudkhalat.is_empty()
            && self.mujalladat.is_empty()
            && matches!(self.hajat, HajatItar::LaHaja(_))
    }

    /// The named reason nothing is deployed, when nothing is.
    #[must_use]
    pub const fn sabab_faragh(&self) -> Option<SababLaHaja> {
        match &self.hajat {
            HajatItar::LaHaja(sabab) => Some(*sabab),
            HajatItar::Matlub(_) => None,
        }
    }

    /// How many files the additive layer would add.
    #[must_use]
    pub fn adad_idafat(&self) -> usize {
        self.mudkhalat.iter().filter(|m| matches!(m.naw, NawMudkhal::Idafa)).count()
    }

    /// How many files the game already has that the additive layer would
    /// modify, with their originals preserved first.
    #[must_use]
    pub fn adad_tadeelat(&self) -> usize {
        self.mudkhalat.iter().filter(|m| matches!(m.naw, NawMudkhal::Tadeel)).count()
    }

    /// The plan as lines for a confirmation screen.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = Vec::with_capacity(self.mudkhalat.len().saturating_add(4));
        match (&self.hajat, self.jidhr_muhammil.as_ref()) {
            (HajatItar::Matlub(mukawwin), Some(mawqi)) => sutur.push(format!(
                "framework: {} into {mawqi} ({})",
                mukawwin.wasf,
                mukawwin.tahmil.wasf()
            )),
            (HajatItar::Matlub(mukawwin), None) => {
                sutur.push(format!("framework: {}", mukawwin.wasf));
            }
            (HajatItar::LaHaja(sabab), _) => {
                sutur.push(format!("framework: none needed — {}", sabab.wasf_injilizi()));
            }
        }
        for mujallad in &self.mujalladat {
            sutur.push(format!("  directory: {}", mujallad.nisbi));
        }
        for mudkhal in &self.mudkhalat {
            let fil = match mudkhal.naw {
                NawMudkhal::Idafa => "add",
                NawMudkhal::Tadeel => "modify",
            };
            sutur.push(format!("  {fil}: {}", mudkhal.nisbi));
        }
        if let Some(khatt) = self.khatt_renpy.as_deref() {
            sutur.push(format!("  font: {khatt}, registered in the generated Ren'Py settings"));
        }
        for talab in &self.talabat {
            sutur.push(format!("  launch: {}", talab.wasf_injilizi()));
        }
        // Measured on Steam, not reasoned about: a verify compares the tree
        // against the depot manifest, so it restores every file Taarib modified
        // and leaves every file Taarib added — the added ones are not in the
        // manifest to be judged against. The halves come apart rather than the
        // install being undone, which is the state nobody predicts: the loader
        // and the payload are still in place, the translated data is not, and
        // the game launches into its original language with Taarib loaded. Said
        // here because a confirmation screen that lists "modify" lines without
        // it describes the write and hides what routinely reverses it.
        if self.adad_tadeelat() > 0 {
            sutur.push(
                "  note: verifying this game's files through its launcher restores the \
                 originals, so the modifications above are undone while the added files \
                 stay; run the install again after a verify"
                    .to_owned(),
            );
        }
        for wakeel in &self.huqn_qaim {
            sutur.push(format!(
                "  note: this game already has a mod in it — {} — which Taarib leaves exactly \
                 as it is and an uninstall never removes",
                wakeel.wasf_injilizi()
            ));
        }
        if let Some(malhuza) = self.manassa_taamil.as_ref() {
            let ism = &malhuza.ism;
            sutur.push(match malhuza.hala.sunduq() {
                None => format!(
                    "  note: {ism} is running and rewrites its configuration when it exits; \
                     close it before installing"
                ),
                Some(sunduq) => format!(
                    "  note: this build runs inside {} and sees only its own processes, so \
                     whether {ism} is running is unknown; the install will refuse rather \
                     than edit a file {ism} may overwrite from memory",
                    sunduq.ism()
                ),
            });
        }
        sutur
    }
}

/// What [`nashr_mulhaqat`] actually did, path by path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerMulhaqat {
    /// Directories created and recorded.
    pub mujalladat: Vec<String>,

    /// Files added and recorded.
    pub mudafa: Vec<MalafMunashar>,

    /// Game files modified, with their originals preserved first.
    pub muaddala: Vec<MalafMunashar>,

    /// Registrations that were already there — from a previous installation or
    /// from the user's own hand — and were therefore left alone.
    pub mutakhatta: Vec<String>,
}

impl TaqreerMulhaqat {
    /// How many bytes were written into the game.
    #[must_use]
    pub fn hajm(&self) -> u64 {
        self.mudafa
            .iter()
            .chain(self.muaddala.iter())
            .fold(0_u64, |majmu, malaf| majmu.saturating_add(malaf.hajm))
    }
}

/// Builds the whole deployment plan for one game, without writing anything.
///
/// This is the product's answer to "what will be done to this game", and after
/// this change it is also the only answer any writer on the install path is
/// given: [`nashr_bi_khutta`] executes exactly this value, and the script-engine
/// write is authorised by [`KhuttatTarkib::qarar`] rather than by a second
/// reading of the game's directory.
///
/// The game arrives as a whole [`LubaMuhallala`] rather than as its parts, and
/// that is the fix for one of those second readings: the loader directory is
/// resolved here, from the executable's own location, and travels inside the
/// plan. Before it did, the planner assumed the game root while the writer read
/// the executable's directory, and the two disagreed for every game whose
/// executable is not at the root.
///
/// # Errors
///
/// - [`KhataTathbeet::IdhnGhayrMutabiq`] when the capability report says the
///   safety layer refuses this game. Honouring the flag here is what stops the
///   planning stage from being a way around the `aman` gate, and
///   [`QararTabaqa::min_taqreer`] is the single place it is read.
/// - [`KhataTathbeet::BeeaMafquda`] when the game runs behind a compatibility
///   prefix that has never been built, because a Windows framework deployed
///   against one is never loaded and never says so.
/// - [`KhataTathbeet::MukawwinMafqud`] when the component store does not hold
///   a component the table names, or holds it without the loader that makes it
///   run.
/// - [`KhataTathbeet::MasarKharij`] when a destination does not stay inside
///   the game root or the prefix.
/// - [`KhataTathbeet::HajmMufrit`] when a component is above
///   [`AQSA_HAJM_MUKAWWIN`].
/// - [`KhataTathbeet::KhataMalaf`] when the store cannot be walked, when an
///   Unreal game has no packaged `Binaries/<platform>/` to receive the module,
///   or when an RPG Maker game is missing the `plugins.js` its registration
///   needs.
pub fn khutta(
    taqreer: &TaqreerImkaniyat,
    luba: &LubaMuhallala,
    mukawwinat: &Path,
) -> NatijatTathbeet<KhuttatTarkib> {
    let qarar = QararTabaqa::min_taqreer(taqreer)?;
    let muharrik = &luba.muharrik;
    let jidhr_luba = luba.jidhr.as_path();

    let mut mukhattat = KhuttatTarkib {
        aila: muharrik.aila,
        tabaqa: qarar.tabaqa(),
        hajat: HajatItar::LaHaja(SababLaHaja::BayanatWaMulhaq),
        jidhr_muhammil: None,
        mujalladat: Vec::new(),
        mudkhalat: Vec::new(),
        khatt_renpy: None,
        talabat: Vec::new(),
        huqn_qaim: Vec::new(),
        manassa_taamil: None,
        // The tier-3 table, replaced below for every other tier. Its loader
        // directory is the game root, which is the directory tier 3 surveys and
        // the only one it ever names.
        mawadi: MawadiTarkib::min_ajzaa(jidhr_luba, String::new(), None),
    };

    // Tier 3 is engine-independent by definition: whatever the family, the
    // game is not modified. Answering it before the per-engine arms means no
    // arm below can deploy a file into a game the tier says is untouchable.
    if !qarar.tughayyar_al_luba() {
        mukhattat.hajat = HajatItar::LaHaja(SababLaHaja::TabaqaFawqiya);
        mukhattat.talabat.push(TalabItlaq::TashgheelLawha {
            sabab: "no text system inside this game is reachable, so Arabic is drawn over it \
                    from outside",
        });
        // Surveyed even though nothing is deployed into this game. Tier 3 draws
        // over the game from a second process that hooks its presentation, so a
        // graphics proxy already sitting in the directory is exactly the thing
        // the person agreeing has to know about — it is the one already drawing
        // over the same frames.
        mukhattat.huqn_qaim = masah_huqn(jidhr_luba)?;
        mukhattat.manassa_taamil = manassa_qayida();
        return Ok(mukhattat);
    }

    let qurs = tahaqquq_beea(luba)?;
    let mawadi = MawadiTarkib::min_ajzaa(
        jidhr_luba,
        mujallad_muhammil(luba)?,
        qurs.as_ref().map(|(beea, qurs)| (beea.as_path(), qurs.as_path())),
    );

    let jidhr_muhammil = mawadi.jidhr_muhammil()?;
    mukhattat.huqn_qaim = masah_huqn(&jidhr_muhammil.mutlaq(mawadi.jidhr_luba())?)?;

    mukhattat.hajat = hajat_maa_tabaqa(muharrik, qarar.tabaqa(), luba.nizam, &luba.beea);
    if let HajatItar::Matlub(mukawwin) = &mukhattat.hajat {
        tahaqquq_mukawwin(mukawwinat, mukawwin)?;
        if let Some(talab) = talab_tahmil(mukawwin, &mawadi) {
            mukhattat.talabat.push(talab);
        }
        mukhattat.jidhr_muhammil = Some(jidhr_muhammil);
    }

    mulhaqat_muharrik(muharrik, &mawadi, mukawwinat, &mut mukhattat)?;
    if !mukhattat.talabat.is_empty() {
        mukhattat.manassa_taamil = manassa_qayida();
    }
    mukhattat.mawadi = mawadi;
    Ok(mukhattat)
}

/// The framework table, with the two answers the tier overrides.
fn hajat_maa_tabaqa(
    muharrik: &Muharrik,
    tabaqa: Tabaqa,
    nizam: NizamTashghil,
    beea: &BeeatTawafuq,
) -> HajatItar {
    match muharrik.aila {
        AilatMuharrik::GameMaker => {
            if matches!(tabaqa, Tabaqa::RasmMubashir) {
                HajatItar::Matlub(Box::new(MukawwinItar::mudkhal(
                    hadaf_hamula(nizam, beea),
                    muharrik.mimariya,
                    beea.windows_dakhilan(),
                )))
            } else {
                HajatItar::LaHaja(SababLaHaja::DakhilAlRuqaa)
            }
        }
        AilatMuharrik::RpgMakerVxAce => HajatItar::LaHaja(SababLaHaja::DakhilAlRuqaa),
        _ => hajat_itar(muharrik, nizam, beea),
    }
}

/// The directory a framework's loader belongs in, relative to the game root.
///
/// The executable's own directory, because that is the only directory an
/// operating system resolves a proxy module out of and the only one
/// `taarib-mudkhal` looks in for a payload. Unreal is the exception and not a
/// contradiction: a packaged Unreal title is routinely started through a stub at
/// the game root while the module it loads belongs beside the shipping binary in
/// `<Project>/Binaries/<platform>/`, so that directory is located rather than
/// inferred from the stub.
///
/// This is the answer the plan carries and the framework writer uses. It used to
/// be answered twice — here as "the game root, unless Unreal", and again inside
/// the writer as `LubaMuhallala::mujallad_tanfidhi` — and the two disagreed for
/// every game whose executable sits in a subdirectory.
fn mujallad_muhammil(luba: &LubaMuhallala) -> NatijatTathbeet<String> {
    if matches!(luba.muharrik.aila, AilatMuharrik::Unreal) {
        return thunaiyat_unreal(
            hadaf_hamula(luba.nizam, &luba.beea),
            luba.muharrik.mimariya,
            &luba.jidhr,
        );
    }
    luba.mujallad_tanfidhi()
}

/// Locates a packaged Unreal build's `Binaries/<platform>/` directory.
fn thunaiyat_unreal(
    hadaf: NizamTashghil,
    mimariya: Mimariya,
    jidhr_luba: &Path,
) -> NatijatTathbeet<String> {
    let asmaa: &[&str] = match (hadaf, mimariya) {
        (NizamTashghil::Windows, Mimariya::X86) => &["Win32", "Win64"],
        (NizamTashghil::Windows, _) => &["Win64", "WinGDK", "Win32"],
        (NizamTashghil::Linux, Mimariya::Aarch64) => &["LinuxArm64", "Linux"],
        (NizamTashghil::Linux, _) => &["Linux"],
        (NizamTashghil::Mac, _) => &["Mac"],
    };

    let mut murashahun: Vec<(String, bool)> = Vec::new();
    for madkhal in WalkDir::new(jidhr_luba).max_depth(3).sort_by_file_name() {
        let Ok(madkhal) = madkhal else { continue };
        // Case-insensitively, all three times. A Windows game under Wine or
        // Proton sees a case-insensitive filesystem, so a depot shipping
        // `binaries/` runs correctly for the player while an exact comparison
        // here finds nothing — and the injected Unreal module is then given
        // nowhere to go, silently. The exclusion below has the same problem in
        // the opposite direction: an `engine/Binaries` would not be excluded and
        // the engine's own binaries would be offered as a target.
        if !madkhal.file_type().is_dir()
            || !madkhal.file_name().to_string_lossy().eq_ignore_ascii_case("Binaries")
        {
            continue;
        }
        let Ok(nisbi) = madkhal.path().strip_prefix(jidhr_luba) else { continue };
        if nisbi
            .components()
            .any(|juz| juz.as_os_str().to_string_lossy().eq_ignore_ascii_case("Engine"))
        {
            continue;
        }
        let Some(nisbi_binaries) = nisbi_nass(nisbi) else { continue };
        // Listed rather than joined, and the *recorded* name is the directory's
        // own: a path built from the spelling in `asmaa` would not resolve on a
        // depot that spells it `win64`, which is the case this fold exists for.
        let Ok(mudkhalat) = std::fs::read_dir(madkhal.path()) else { continue };
        for far in mudkhalat.take(4_096).flatten() {
            if !far.path().is_dir() {
                continue;
            }
            let ism_haqiqi = far.file_name().to_string_lossy().into_owned();
            if !asmaa.iter().any(|ism| ism_haqiqi.eq_ignore_ascii_case(ism)) {
                continue;
            }
            let fihi_malaf = std::fs::read_dir(far.path()).is_ok_and(|mudkhalat| {
                mudkhalat.filter_map(Result::ok).any(|malaf| malaf.path().is_file())
            });
            murashahun.push((format!("{nisbi_binaries}/{ism_haqiqi}"), fihi_malaf));
        }
    }

    murashahun
        .iter()
        .find(|(_, fihi)| *fihi)
        .or_else(|| murashahun.first())
        .map(|(masar, _)| masar.clone())
        .ok_or_else(|| KhataTathbeet::KhataMalaf {
            masar: jidhr_luba.join("Binaries"),
            amal: "locating a packaged Unreal build's Binaries directory",
            sabab: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no <Project>/Binaries/<platform>/ directory exists under the game root, so \
                 there is nowhere the engine would load an injected module from",
            ),
        })
}

/// Where a component's loader sits *inside the store*.
///
/// Not the same as where it lands. A split component keeps the files that go
/// beside the executable under [`MUJALLAD_MUHAMMIL`] and everything else under
/// the game's own Taarib directory, so the loader's store path carries that
/// prefix while its destination does not. The two spaces are checked at two
/// different stages, and comparing one against the other is how a component
/// passes the plan and fails the write.
#[must_use]
pub fn masar_muhammil_fi_makhzan(mukawwin: &MukawwinItar) -> String {
    match mukawwin.tawzi {
        TawziMukawwin::Wahid => mukawwin.ism_muhammil.clone(),
        TawziMukawwin::Munfasil => {
            format!("{MUJALLAD_MUHAMMIL}/{}", mukawwin.ism_muhammil)
        }
    }
}

/// Proves a component is in the store, with its loader, before anything is
/// promised on a confirmation screen.
fn tahaqquq_mukawwin(
    jidhr_makhzan: &Path,
    mukawwin: &MukawwinItar,
) -> NatijatTathbeet<()> {
    let asmaa = asmaa_mukawwin(jidhr_makhzan, &mukawwin.ism)?;
    let matlub = masar_muhammil_fi_makhzan(mukawwin);
    if asmaa.iter().any(|ism| ism == &matlub) {
        return Ok(());
    }
    // A component without its loader deploys files that nothing will ever
    // load: the game runs untouched, in English, with no error anywhere.
    Err(KhataTathbeet::MukawwinMafqud {
        mukawwin: format!("{}/{matlub}", mukawwin.ism),
        masar: jidhr_makhzan.join(&mukawwin.ism),
    })
}

/// Every file inside one component of the store, as store-relative names.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when the component is absent, is not a
/// directory, or holds no regular file; [`KhataTathbeet::HajmMufrit`] when it
/// is above [`AQSA_HAJM_MUKAWWIN`]; [`KhataTathbeet::MasarKharij`] when the
/// component name does not join onto the store root; and the I/O variants when
/// the store cannot be walked.
pub fn asmaa_mukawwin(jidhr_makhzan: &Path, ism: &str) -> NatijatTathbeet<Vec<String>> {
    let jidhr = masarat::dakhil(jidhr_makhzan, ism).map_err(|khata| {
        KhataTathbeet::MasarKharij {
            masar: PathBuf::from(ism),
            jidhr: jidhr_makhzan.to_path_buf(),
            sabab: khata.injilizi,
        }
    })?;
    if !jidhr.is_dir() {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr,
        });
    }
    // The same gate `hamil_mukawwin` takes, for the same reason: this is what
    // the confirmation screen asks before promising the framework is present.
    crate::bayan_makhzan::kamil_hasab_bayan(jidhr_makhzan, ism)?;

    let mut asmaa: Vec<String> = Vec::new();
    let mut majmu = 0_u64;
    for madkhal in WalkDir::new(&jidhr) {
        let madkhal = madkhal.map_err(|sabab| {
            let masar = sabab.path().map_or_else(|| jidhr.clone(), Path::to_path_buf);
            let sabab = sabab.into_io_error().unwrap_or_else(|| {
                std::io::Error::other("the component store could not be walked")
            });
            min_khata_io(&masar, "listing Taarib's component store", sabab)
        })?;
        // Symbolic links are skipped rather than followed, for the reason
        // `hamil_mukawwin` skips them: a component is bytes in the store, not
        // a link to bytes somewhere else on the machine.
        if !madkhal.file_type().is_file() {
            continue;
        }
        majmu = majmu.saturating_add(
            madkhal.metadata().map(|bayanat| bayanat.len()).unwrap_or_default(),
        );
        if majmu > AQSA_HAJM_MUKAWWIN {
            return Err(KhataTathbeet::HajmMufrit {
                haql: "framework component",
                qeema: majmu,
                saqf: AQSA_HAJM_MUKAWWIN,
            });
        }
        let Some(nisbi) = madkhal.path().strip_prefix(&jidhr).ok().and_then(nisbi_nass) else {
            return Err(KhataTathbeet::MukawwinMafqud {
                mukawwin: ism.to_owned(),
                masar: madkhal.path().to_path_buf(),
            });
        };
        if !nisbi.is_empty() {
            asmaa.push(nisbi);
        }
    }

    if asmaa.is_empty() {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr,
        });
    }
    asmaa.sort();
    Ok(asmaa)
}

/// The launch-time requirement a deployed framework needs, when it needs one.
fn talab_tahmil(mukawwin: &MukawwinItar, mawadi: &MawadiTarkib) -> Option<TalabItlaq> {
    match &mukawwin.tahmil {
        TahmilMusbaq::WakeelWindows { .. } => None,
        TahmilMusbaq::TajawuzWine { wahda } => Some(TalabItlaq::MutaghayyirBeea {
            ism: ISM_TAJAWUZAT,
            qeema: format!("{wahda}={TARTEEB_ASLI}"),
            sabab: "Wine prefers its own builtin module and would load the game with nothing \
                    attached unless this override names the native one",
        }),
        TahmilMusbaq::LdPreload | TahmilMusbaq::DyldInsert => {
            let nisbi = mawadi.bijanib(&mukawwin.ism_muhammil);
            if mukawwin.alamat.iter().any(|alama| alama.as_str() == "run_bepinex.sh") {
                Some(TalabItlaq::KhiyarTashghil {
                    qeema: format!("sh ./{} {RAMZ_AMR}", mawadi.bijanib("run_bepinex.sh")),
                    sabab: "on this platform the loader is preloaded by the run script, which \
                            only runs if the launch command invokes it",
                })
            } else {
                let ism = if matches!(mukawwin.tahmil, TahmilMusbaq::DyldInsert) {
                    ISM_TAHMIL_MAC
                } else {
                    ISM_TAHMIL_LINUX
                };
                Some(TalabItlaq::MutaghayyirBeea {
                    ism,
                    qeema: format!("./{nisbi}"),
                    sabab: "this engine loads no plugins of its own, so the module has to be \
                            put into the process by the dynamic linker",
                })
            }
        }
    }
}

/// The note the plan should carry about the launcher that owns launch options.
///
/// Nothing is said when it was seen closed, and something is said in both other
/// cases: "it is running" and "this build cannot tell" are each a reason the
/// install about to be described will stop, and a plan that mentions neither is
/// a plan that looks ready.
fn manassa_qayida() -> Option<MalhuzatManassa> {
    match halat_manassa(&ASMAA_STEAM) {
        HalatTashghil::LaTashtaghil => None,
        hala => Some(MalhuzatManassa { ism: ISM_STEAM.to_owned(), hala }),
    }
}

/// The additive layer for one engine: the adapter files the engine loads by
/// itself, and the registrations that make it look for them.
fn mulhaqat_muharrik(
    muharrik: &Muharrik,
    mawadi: &MawadiTarkib,
    jidhr_makhzan: &Path,
    mukhattat: &mut KhuttatTarkib,
) -> NatijatTathbeet<()> {
    match muharrik.aila {
        AilatMuharrik::RpgMakerMv => rpg_maker(mawadi, jidhr_makhzan, "www/js", "mv", mukhattat),
        AilatMuharrik::RpgMakerMz => rpg_maker(mawadi, jidhr_makhzan, "js", "mz", mukhattat),
        AilatMuharrik::Renpy => renpy(mawadi, jidhr_makhzan, mukhattat),
        AilatMuharrik::Godot => {
            if muharrik.isdar.as_ref().is_some_and(|isdar| isdar.kabir == 3) {
                godot_thalatha(muharrik, mawadi, mukhattat)
            } else {
                Ok(())
            }
        }
        // Nothing additive: each of these is reached from inside its process by
        // the module the framework step placed, and BIO4 — which loads no
        // plugin of any kind — is the clearest case of it.
        AilatMuharrik::Unity
        | AilatMuharrik::Unreal
        | AilatMuharrik::RpgMakerVxAce
        | AilatMuharrik::GameMaker
        | AilatMuharrik::Electron
        | AilatMuharrik::Bio4
        | AilatMuharrik::Majhul => Ok(()),
    }
}

/// RPG Maker MV and MZ: the plugin, and the registration without which nwjs
/// never loads it.
fn rpg_maker(
    mawadi: &MawadiTarkib,
    jidhr_makhzan: &Path,
    qaida: &str,
    far: &str,
    mukhattat: &mut KhuttatTarkib,
) -> NatijatTathbeet<()> {
    let mukawwin = format!("{MUKAWWIN_MULHAQ}/rpgmaker/{far}");
    let mujallad = format!("{qaida}/plugins");
    mukhattat.mujalladat.push(MujalladTarkib {
        mawqi: mawadi.wajha(&mujallad)?,
        nisbi: mujallad.clone(),
    });

    for fi_makhzan in asmaa_mukawwin(jidhr_makhzan, &mukawwin)? {
        let nisbi = format!("{mujallad}/{fi_makhzan}");
        mukhattat.mudkhalat.push(MudkhalTarkib {
            mawqi: mawadi.wajha(&nisbi)?,
            nisbi,
            naw: NawMudkhal::Idafa,
            masdar: MasdarMudkhal::MinMakhzan {
                mukawwin: mukawwin.clone(),
                fi_makhzan,
            },
        });
    }

    // The registry is the game's own file, and it must be there: an MV or MZ
    // game without `plugins.js` does not have the layout its engine
    // identification promised, and finding that out now — with nothing
    // written — is the entire point of planning.
    let nisbi = format!("{qaida}/plugins.js");
    let mawqi = mawadi.wajha(&nisbi)?;
    let mutlaq = mawqi.mutlaq(mawadi.jidhr_luba())?;
    if !mutlaq.is_file() {
        return Err(KhataTathbeet::KhataMalaf {
            masar: mutlaq,
            amal: "locating the RPG Maker plugin registry",
            sabab: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "plugins.js is missing, so nwjs would load no plugin however many are placed \
                 beside it",
            ),
        });
    }
    mukhattat.mudkhalat.push(MudkhalTarkib {
        mawqi,
        nisbi,
        naw: NawMudkhal::Tadeel,
        masdar: MasdarMudkhal::TasjeelIdafi { naw: NawTasjeel::PluginsJs },
    });
    Ok(())
}

/// Ren'Py: pure addition into `game/`, which the engine compiles and runs at
/// startup with no registration step of any kind.
///
/// The Arabic face travels with the rest of the component and is placed by this
/// same loop; what the arm does about it is *name* it, so that the plan states
/// which of the files it is about to add the generated settings will point every
/// text style at.
fn renpy(
    mawadi: &MawadiTarkib,
    jidhr_makhzan: &Path,
    mukhattat: &mut KhuttatTarkib,
) -> NatijatTathbeet<()> {
    let asmaa = asmaa_mukawwin(jidhr_makhzan, MUKAWWIN_RENPY)?;
    // Chosen from the very names this arm is about to plan, rather than from a
    // second walk of the store: the file the settings name and the file this
    // deploys are then the same file by construction.
    mukhattat.khatt_renpy = ikhtar_khatt_renpy(&asmaa).map(str::to_owned);
    for fi_makhzan in asmaa {
        let nisbi = format!("game/{fi_makhzan}");
        mukhattat.mudkhalat.push(MudkhalTarkib {
            mawqi: mawadi.wajha(&nisbi)?,
            nisbi,
            naw: NawMudkhal::Idafa,
            masdar: MasdarMudkhal::MinMakhzan {
                mukawwin: MUKAWWIN_RENPY.to_owned(),
                fi_makhzan,
            },
        });
    }
    Ok(())
}

/// The Arabic face a Ren'Py install will place, named relative to `game/`.
///
/// The answer [`KhuttatTarkib::khatt_renpy`] carries, asked separately because
/// the *text* write needs it before the plan is executed:
/// `nusus::raqqi_nusus` runs ahead of `nashr` so that RPG Maker's byte offsets
/// into `js/plugins.js` are still the offsets its extraction measured, and at
/// that moment nothing has been deployed. Nothing is invented to bridge that —
/// the store is read, and the name returned is the name [`renpy`] will place,
/// because both go through [`ikhtar_khatt_renpy`] over the same listing.
///
/// [`None`] means no font is registered at all, which is a game rendered with
/// whatever face it already ships: correct Arabic on a build whose GUI font
/// covers Arabic, empty boxes on one whose font does not. The generated
/// settings say so in the install report rather than registering a name.
///
/// # Errors
///
/// Whatever [`asmaa_mukawwin`] raises other than an absent or short component.
/// Those two are [`None`]: a build staged without the Ren'Py adapter, or one
/// whose mirror never settled, is refused by name a moment later when
/// [`khutta`] plans the same component, and a module whose subject is text
/// should not be where that refusal first appears.
pub fn khatt_renpy(jidhr_makhzan: &Path) -> NatijatTathbeet<Option<String>> {
    match asmaa_mukawwin(jidhr_makhzan, MUKAWWIN_RENPY) {
        Ok(asmaa) => Ok(ikhtar_khatt_renpy(&asmaa).map(str::to_owned)),
        Err(KhataTathbeet::MukawwinMafqud { .. } | KhataTathbeet::MukawwinNaqis { .. }) => Ok(None),
        Err(khata) => Err(khata),
    }
}

/// Picks one face out of a Ren'Py component's file listing.
///
/// Candidates are the font files directly under [`MUJALLAD_KHATT_RENPY`] —
/// directly, because a face is a file the component ships for this purpose and
/// a `.ttf` that turned up somewhere else in the tree is not one. They are
/// ranked by [`TARTIB_KHATT_RENPY`] and then by name, so the same store gives
/// the same answer on every machine and in both of the two places that ask.
fn ikhtar_khatt_renpy(asmaa: &[String]) -> Option<&str> {
    let mut mufaddal: Option<(usize, &str)> = None;
    for ism in asmaa {
        let Some(dhayl) = ism.strip_prefix(MUJALLAD_KHATT_RENPY) else { continue };
        if dhayl.contains('/') {
            continue;
        }
        let lahiqa = Path::new(dhayl).extension().and_then(|lahiqa| lahiqa.to_str());
        if !lahiqa.is_some_and(|lahiqa| {
            LAWAHIQ_KHATT.iter().any(|maqbul| lahiqa.eq_ignore_ascii_case(maqbul))
        }) {
            continue;
        }
        // A face the preference list does not name still ranks, after every one
        // it does; `len()` is one past the last named rank.
        let rutba = TARTIB_KHATT_RENPY
            .iter()
            .position(|badiya| dhayl.starts_with(badiya))
            .unwrap_or(TARTIB_KHATT_RENPY.len());
        let afdal = mufaddal.is_none_or(|(hali, ism_hali)| {
            rutba < hali || (rutba == hali && ism.as_str() < ism_hali)
        });
        if afdal {
            mufaddal = Some((rutba, ism));
        }
    }
    mufaddal.map(|(_, ism)| ism)
}

/// Godot 3: the `GDNative` description beside the pack, and the project override
/// that lists it as a singleton.
fn godot_thalatha(
    muharrik: &Muharrik,
    mawadi: &MawadiTarkib,
    mukhattat: &mut KhuttatTarkib,
) -> NatijatTathbeet<()> {
    let HajatItar::Matlub(mukawwin) = &mukhattat.hajat else { return Ok(()) };
    let nass = nass_gdnlib(&mukawwin.ism_muhammil, muharrik.mimariya);

    let gdnlib = mawadi.bijanib("taarib.gdnlib");
    mukhattat.mudkhalat.push(MudkhalTarkib {
        mawqi: mawadi.wajha(&gdnlib)?,
        nisbi: gdnlib,
        naw: NawMudkhal::Idafa,
        masdar: MasdarMudkhal::NassMuwallad {
            nass,
            wasf: "the GDNative library description",
        },
    });

    // A game may or may not ship an `override.cfg`. Which of the two it is
    // decides whether this is an addition or a modification, and it is decided
    // again at write time — another tool may create or remove the file between
    // the confirmation screen and the button.
    let idafi = mawadi
        .bijanib(taarib_mustalahat::muharrik::asmaa_muharrik::TAJAWUZ_GODOT);
    let mawqi = mawadi.wajha(&idafi)?;
    let mawjud = mawqi.mutlaq(mawadi.jidhr_luba())?.is_file();
    mukhattat.mudkhalat.push(MudkhalTarkib {
        mawqi,
        nisbi: idafi,
        naw: if mawjud { NawMudkhal::Tadeel } else { NawMudkhal::Idafa },
        masdar: MasdarMudkhal::TasjeelIdafi { naw: NawTasjeel::OverrideCfg },
    });
    Ok(())
}

/// The base name of the Godot 3 `GDNative` module, as its crate builds it.
const ASAS_GODOT: &str = "taarib_muhawwil_godot";

/// The `taarib.gdnlib` a Godot 3 project loads the module through.
///
/// The entry names the `GDNative` module, never the loader: the loader is a
/// proxy the operating system maps, it exports none of the `taarib_` `GDNative`
/// symbols, and a Godot that opened it would find no `taarib_gdnative_init`
/// and register no singleton. `ism_muhammil`'s extension is read only to
/// decide which platform's key and file naming this entry is for.
fn nass_gdnlib(ism_muhammil: &str, mimariya: Mimariya) -> String {
    let ard = match mimariya {
        Mimariya::X86 => "32",
        Mimariya::X8664 | Mimariya::Aarch64 => "64",
    };
    // Folded, because a component store built on a case-insensitive filesystem
    // can hand back `winhttp.DLL` for the same entry it stored as `winhttp.dll`.
    let imtidad = Path::new(ism_muhammil).extension().and_then(|imt| imt.to_str());
    let (miftah, ism_wahda) = if imtidad.is_some_and(|imt| imt.eq_ignore_ascii_case("dll")) {
        (format!("Windows.{ard}"), NizamTashghil::Windows.ism_maktaba(ASAS_GODOT))
    } else if imtidad.is_some_and(|imt| imt.eq_ignore_ascii_case("dylib")) {
        (format!("OSX.{ard}"), NizamTashghil::Mac.ism_maktaba(ASAS_GODOT))
    } else {
        (format!("X11.{ard}"), NizamTashghil::Linux.ism_maktaba(ASAS_GODOT))
    };
    // A split component puts everything that is not the loader under the
    // game's own Taarib directory, which is where the module lands.
    format!(
        "[general]\n\
         singleton=true\n\
         load_once=true\n\
         symbol_prefix=\"taarib_\"\n\
         reloadable=false\n\
         \n\
         [entry]\n\
         {miftah}=\"res://{MUJALLAD_TAARIB}/{ism_wahda}\"\n\
         \n\
         [dependencies]\n\
         {miftah}=[  ]\n"
    )
}

/// A fresh `override.cfg` for a game that ships none.
fn nass_override_jadeed() -> String {
    format!("{QISM_GDNATIVE}\n{MIFTAH_SINGLETONS}[ \"{MAWRID_GDNLIB}\" ]\n")
}

/// Writes the additive layer of a plan, through the manifest and through
/// nothing else.
///
/// # Errors
///
/// Whatever the recorder raises for a file it cannot preserve, add or record;
/// [`KhataTathbeet::MukawwinMafqud`] when a component the plan named is no
/// longer in the store, or no longer holds the file the plan named;
/// [`KhataTathbeet::HajmMufrit`] for a component above
/// [`AQSA_HAJM_MUKAWWIN`] or a registry above [`SAQF_HAJM_TASJIL`]; and
/// [`KhataTathbeet::KhataMalaf`] when a registry cannot be read, is not UTF-8,
/// or carries no array for the registration to go into — reported rather than
/// guessed at, because guessing at the shape of somebody's game file is how a
/// game stops starting.
pub fn nashr_mulhaqat(
    mukhattat: &KhuttatTarkib,
    jidhr_luba: &Path,
    jidhr_makhzan: &Path,
    muthabbit: &mut dyn Muthabbit,
) -> NatijatTathbeet<TaqreerMulhaqat> {
    let mut taqreer = TaqreerMulhaqat {
        mujalladat: Vec::new(),
        mudafa: Vec::new(),
        muaddala: Vec::new(),
        mutakhatta: Vec::new(),
    };
    if mukhattat.mudkhalat.is_empty() && mukhattat.mujalladat.is_empty() {
        return Ok(taqreer);
    }

    for mujallad in &mukhattat.mujalladat {
        muthabbit.ansha_mujallad(&mujallad.mawqi.mutlaq(jidhr_luba)?)?;
        taqreer.mujalladat.push(mujallad.nisbi.clone());
    }

    // One component is read once however many of its files the plan places,
    // because reading a payload per entry turns a twelve-file adapter into
    // twelve walks of the same directory.
    let mut makhzan: Vec<(String, Vec<(String, Vec<u8>)>)> = Vec::new();

    for mudkhal in &mukhattat.mudkhalat {
        let mutlaq = mudkhal.mawqi.mutlaq(jidhr_luba)?;
        match &mudkhal.masdar {
            MasdarMudkhal::MinMakhzan { mukawwin, fi_makhzan } => {
                let bayt = bayt_min_makhzan(&mut makhzan, jidhr_makhzan, mukawwin, fi_makhzan)?;
                iktub_aw_ansha(muthabbit, &mutlaq, &bayt, mudkhal.naw)?;
                sajjil_munashar(&mut taqreer, mudkhal, &bayt);
            }
            MasdarMudkhal::NassMuwallad { nass, .. } => {
                iktub_aw_ansha(muthabbit, &mutlaq, nass.as_bytes(), mudkhal.naw)?;
                sajjil_munashar(&mut taqreer, mudkhal, nass.as_bytes());
            }
            MasdarMudkhal::TasjeelIdafi { naw } => {
                // Whether the game has this file is decided now rather than
                // trusted from plan time: a launcher update or another tool
                // may have created or removed it since the plan was shown.
                let mawjud = mutlaq.is_file();
                let hali = if mawjud { Some(iqra_tasjil(&mutlaq)?) } else { None };
                let jadeed = match (naw, hali.as_deref()) {
                    (NawTasjeel::PluginsJs, Some(hali)) => damj_plugins_js(&mutlaq, hali)?,
                    (NawTasjeel::PluginsJs, None) => {
                        return Err(KhataTathbeet::KhataMalaf {
                            masar: mutlaq,
                            amal: "registering Taarib in the RPG Maker plugin registry",
                            sabab: std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "plugins.js was there when the plan was built and is not there \
                                 now; nothing was written",
                            ),
                        });
                    }
                    (NawTasjeel::OverrideCfg, Some(hali)) => {
                        damj_override_cfg(&mutlaq, hali)?
                    }
                    (NawTasjeel::OverrideCfg, None) => Some(nass_override_jadeed()),
                };
                match jadeed {
                    Some(nass) => {
                        let naw = if mawjud { NawMudkhal::Tadeel } else { NawMudkhal::Idafa };
                        iktub_aw_ansha(muthabbit, &mutlaq, nass.as_bytes(), naw)?;
                        sajjil_munashar_bi_naw(&mut taqreer, mudkhal, nass.as_bytes(), naw);
                    }
                    None => taqreer.mutakhatta.push(mudkhal.nisbi.clone()),
                }
            }
        }
    }

    Ok(taqreer)
}

/// Dispatches one buffer to the recorder method its kind demands.
fn iktub_aw_ansha(
    muthabbit: &mut dyn Muthabbit,
    mutlaq: &Path,
    bayt: &[u8],
    naw: NawMudkhal,
) -> NatijatTathbeet<()> {
    match naw {
        NawMudkhal::Idafa => muthabbit.ansha(mutlaq, bayt),
        NawMudkhal::Tadeel => muthabbit.iktub(mutlaq, bayt),
    }
}

/// Files one written entry under the report heading its kind belongs to.
fn sajjil_munashar(taqreer: &mut TaqreerMulhaqat, mudkhal: &MudkhalTarkib, bayt: &[u8]) {
    sajjil_munashar_bi_naw(taqreer, mudkhal, bayt, mudkhal.naw);
}

/// The same, for an entry whose kind was decided at write time.
fn sajjil_munashar_bi_naw(
    taqreer: &mut TaqreerMulhaqat,
    mudkhal: &MudkhalTarkib,
    bayt: &[u8],
    naw: NawMudkhal,
) {
    let munashar = MalafMunashar {
        mawqi: mudkhal.mawqi.clone(),
        hajm: tul_u64(bayt.len()),
        basma: basma_bayt(bayt),
    };
    match naw {
        NawMudkhal::Idafa => taqreer.mudafa.push(munashar),
        NawMudkhal::Tadeel => taqreer.muaddala.push(munashar),
    }
}

/// One file's bytes out of a component, reading the component at most once.
fn bayt_min_makhzan(
    makhzan: &mut Vec<(String, Vec<(String, Vec<u8>)>)>,
    jidhr_makhzan: &Path,
    mukawwin: &str,
    fi_makhzan: &str,
) -> NatijatTathbeet<Vec<u8>> {
    if !makhzan.iter().any(|(ism, _)| ism.as_str() == mukawwin) {
        let hamula = hamil_mukawwin(jidhr_makhzan, mukawwin)?;
        makhzan.push((mukawwin.to_owned(), hamula));
    }
    makhzan
        .iter()
        .find(|(ism, _)| ism.as_str() == mukawwin)
        .and_then(|(_, hamula)| hamula.iter().find(|(ism, _)| ism.as_str() == fi_makhzan))
        .map(|(_, bayt)| bayt.clone())
        .ok_or_else(|| KhataTathbeet::MukawwinMafqud {
            mukawwin: format!("{mukawwin}/{fi_makhzan}"),
            masar: jidhr_makhzan.join(mukawwin),
        })
}

/// Reads a loader registry the game owns, within the registry ceiling.
fn iqra_tasjil(masar: &Path) -> NatijatTathbeet<String> {
    let amal = "reading a loader registry in order to register Taarib in it";
    let bayanat = std::fs::metadata(masar)
        .map_err(|sabab| min_khata_io(masar, amal, sabab))?;
    if bayanat.len() > SAQF_HAJM_TASJIL {
        return Err(KhataTathbeet::HajmMufrit {
            haql: "loader registry",
            qeema: bayanat.len(),
            saqf: SAQF_HAJM_TASJIL,
        });
    }
    masarat::qira_nass(masar).map_err(|khata| KhataTathbeet::KhataMalaf {
        masar: masar.to_path_buf(),
        amal,
        sabab: std::io::Error::other(khata.injilizi),
    })
}

/// Adds Taarib's entry to an RPG Maker `plugins.js`, or reports that it is
/// already there.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] when there is no `]` to insert before. That
/// is not a `plugins.js`, and appending an entry to the end of a file that is
/// not an array literal produces a game that does not start.
fn damj_plugins_js(masar: &Path, hali: &str) -> NatijatTathbeet<Option<String>> {
    let bila_faragh: String = hali.chars().filter(|harf| !harf.is_whitespace()).collect();
    if bila_faragh.contains(IBRAT_PLUGINS) {
        return Ok(None);
    }

    let Some(mawqi) = hali.rfind(']') else {
        return Err(KhataTathbeet::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "registering Taarib in the RPG Maker plugin registry",
            sabab: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the file carries no array literal to register a plugin in, so it is not the \
                 plugins.js this engine reads",
            ),
        });
    };
    let Some((qabl, baad)) = hali.split_at_checked(mawqi) else {
        return Err(KhataTathbeet::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "registering Taarib in the RPG Maker plugin registry",
            sabab: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "the array's closing bracket does not fall on a character boundary, so the \
                 file is not the UTF-8 it was read as",
            ),
        });
    };

    let mahdhuf = qabl.trim_end();
    // An empty array takes the entry alone; a populated one takes a comma
    // first, unless the last element already left a trailing one.
    let fasil = if mahdhuf.ends_with('[') || mahdhuf.ends_with(',') { "" } else { "," };
    Ok(Some(format!("{mahdhuf}{fasil}\n{MADKHAL_PLUGINS}\n{baad}")))
}

/// Adds Taarib's `GDNative` singleton to a Godot 3 `override.cfg`, or reports
/// that it is already listed.
///
/// # Errors
///
/// [`KhataTathbeet::KhataMalaf`] when the file has a `singletons=` line whose
/// list this module cannot find the end of. Appending a second `singletons=`
/// line would be worse than refusing: Godot reads the last one and silently
/// drops every singleton the first one named, which is the game's own plugins
/// disappearing because Taarib was installed.
fn damj_override_cfg(masar: &Path, hali: &str) -> NatijatTathbeet<Option<String>> {
    if hali.contains(MAWRID_GDNLIB) {
        return Ok(None);
    }

    let mut sutur: Vec<String> = Vec::new();
    let mut muharrar = false;
    for satr in hali.lines() {
        if muharrar || !satr.trim_start().starts_with(MIFTAH_SINGLETONS) {
            sutur.push(satr.to_owned());
            continue;
        }
        let Some((qabl, baad)) = satr.rfind(']').and_then(|mawqi| satr.split_at_checked(mawqi))
        else {
            return Err(KhataTathbeet::KhataMalaf {
                masar: masar.to_path_buf(),
                amal: "registering Taarib's GDNative singleton in the project override",
                sabab: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "the existing singletons setting has no closing bracket, and adding a \
                     second singletons line would make the engine ignore the game's own",
                ),
            });
        };
        let mahdhuf = qabl.trim_end();
        let fasil = if mahdhuf.ends_with('[') || mahdhuf.ends_with(',') { "" } else { "," };
        sutur.push(format!("{mahdhuf}{fasil} \"{MAWRID_GDNLIB}\" {baad}"));
        muharrar = true;
    }

    if !muharrar {
        if !sutur.is_empty() {
            sutur.push(String::new());
        }
        sutur.push(QISM_GDNATIVE.to_owned());
        sutur.push(format!("{MIFTAH_SINGLETONS}[ \"{MAWRID_GDNLIB}\" ]"));
    }
    let mut nateeja = sutur.join("\n");
    nateeja.push('\n');
    Ok(Some(nateeja))
}

/// Executes one plan: the script-engine write, the framework, and the additive
/// layer, in that order and through the recorder.
///
/// The order is the RPG Maker order and is not negotiable. That engine's
/// extraction records carry byte offsets into `js/plugins.js`, and the additive
/// layer appends Taarib's registration to that same file; splicing against
/// offsets measured before the append would be splicing against a file whose
/// length has moved. `taarib_muhawwil_nusus::rpgmaker::rakkib_mulhaq` documents
/// the same ordering for the same reason.
///
/// Every one of the three writes is authorised by the `mukhattat` argument.
/// That is the whole point of this function existing beside [`nashr`]: the
/// script-engine write used to run outside any plan, unconditionally, from
/// [`crate::masar_tathbeet::thabbit`] — so a tier-3 game, whose report had just
/// told the player it would not be modified at all, had its `data/*.json`, its
/// `game/tl/arabic/*.rpy`, its `data.win` or its `app.asar` rewritten anyway.
///
/// # Errors
///
/// Whatever the script-engine write, the framework writer or [`nashr_mulhaqat`]
/// raise. A missing component or an unbuilt prefix is refused by [`khutta`]
/// before any of the three runs.
pub fn nashr_bi_khutta(
    mukhattat: &KhuttatTarkib,
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    mukawwinat: &Path,
    nashir: &mut Nashir<'_>,
) -> NatijatTathbeet<(NatijatTarkib, TaqreerMulhaqat)> {
    nashir.raqqi(IdhnNusus::min_khutta(mukhattat), Some(mukawwinat))?;
    let itar = rakkib_itar(mukhattat, luba, halat, mukawwinat, nashir.muthabbit())?;
    let mulhaqat =
        nashr_mulhaqat(mukhattat, &luba.jidhr, mukawwinat, nashir.muthabbit())?;
    Ok((itar, mulhaqat))
}

/// Plans and then executes, for a caller that holds a capability report rather
/// than a plan.
///
/// The plan is built **once**, here, before the first byte, and every writer
/// below receives it. Nothing downstream re-derives the tier, the safety
/// refusal or the loader directory from the game's own directory.
///
/// # Errors
///
/// Whatever [`khutta`] and [`nashr_bi_khutta`] raise.
pub fn nashr(
    luba: &LubaMuhallala,
    halat: &HalatIdadat,
    taqreer: &TaqreerImkaniyat,
    mukawwinat: &Path,
    nashir: &mut Nashir<'_>,
) -> NatijatTathbeet<(NatijatTarkib, TaqreerMulhaqat)> {
    let mukhattat = khutta(taqreer, luba, mukawwinat)?;
    nashr_bi_khutta(&mukhattat, luba, halat, mukawwinat, nashir)
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use taarib_mustalahat::muharrik::{AilatMuharrik, Tabaqa};

    use super::{
        HajatItar, KhuttatTarkib, MUJALLAD_KHATT_RENPY, MasdarMudkhal, MawadiTarkib, MawqiTarkib,
        MudkhalTarkib, NawMudkhal, Path, SababLaHaja, WajhatLuba, ikhtar_khatt_renpy,
    };

    /// The component's listing, as `asmaa_mukawwin` hands it over.
    fn asmaa(dhuyul: &[&str]) -> Vec<String> {
        dhuyul.iter().map(|dhayl| (*dhayl).to_owned()).collect()
    }

    /// One planned file of either kind, at a destination inside the game.
    fn mudkhal(nisbi: &str, naw: NawMudkhal) -> MudkhalTarkib {
        MudkhalTarkib {
            mawqi: MawqiTarkib::DakhilLuba(WajhatLuba::jadeed(nisbi).expect("a valid destination")),
            nisbi: nisbi.to_owned(),
            naw,
            masdar: MasdarMudkhal::NassMuwallad { nass: String::new(), wasf: "a test entry" },
        }
    }

    /// A plan carrying exactly the given entries and nothing else to report.
    fn khutta_bi(mudkhalat: Vec<MudkhalTarkib>) -> KhuttatTarkib {
        KhuttatTarkib {
            aila: AilatMuharrik::Unity,
            tabaqa: Tabaqa::Kamil,
            hajat: HajatItar::LaHaja(SababLaHaja::BayanatWaMulhaq),
            jidhr_muhammil: None,
            mujalladat: Vec::new(),
            mudkhalat,
            khatt_renpy: None,
            talabat: Vec::new(),
            huqn_qaim: Vec::new(),
            manassa_taamil: None,
            // These tests read the plan's report, never its destinations, so the
            // table is the trivial one: a game root with the loader at it.
            mawadi: MawadiTarkib::min_ajzaa(Path::new("/luba"), String::new(), None),
        }
    }

    /// The sentence a person has to read before agreeing to a plan that edits
    /// files the store can put back.
    fn fihi_malhuzat_tahaqquq(khutta: &KhuttatTarkib) -> bool {
        khutta.taqreer().iter().any(|satr| satr.contains("verifying this game's files"))
    }

    #[test]
    fn tadeel_yastadi_malhuzat_altahaqquq() {
        // Measured against Steam: a verify restored the modified file byte for
        // byte and left the added ones untouched. A plan that edits a file the
        // store ships has to say so before the edit, not after the verify.
        let khutta = khutta_bi(vec![
            mudkhal("taarib/tarjama.ruqaa", NawMudkhal::Idafa),
            mudkhal("Data/messages.dat", NawMudkhal::Tadeel),
        ]);
        assert!(
            fihi_malhuzat_tahaqquq(&khutta),
            "a plan with a modification must warn that a verify reverses it"
        );
    }

    #[test]
    fn la_malhuza_hina_la_tadeel() {
        // Nothing the store knows about is touched, so a verify has nothing of
        // Taarib's to undo. Saying it anyway would train people past the notice
        // in the one case where it matters.
        let khutta = khutta_bi(vec![
            mudkhal("taarib/tarjama.ruqaa", NawMudkhal::Idafa),
            mudkhal("BepInEx/plugins/taarib.dll", NawMudkhal::Idafa),
        ]);
        assert_eq!(khutta.adad_tadeelat(), 0);
        assert!(
            !fihi_malhuzat_tahaqquq(&khutta),
            "an additive-only plan has nothing a verify would take away"
        );
    }

    #[test]
    fn khutta_farigha_la_tuhadhdhir() {
        assert!(!fihi_malhuzat_tahaqquq(&khutta_bi(Vec::new())));
    }

    #[test]
    fn la_khatt_illa_min_mujallad_alkhutut() {
        // The Python package, its license, and a stray face somewhere else in
        // the tree. None of the three is the component's Arabic face.
        let listing = asmaa(&[
            "taarib_renpy/__init__.py",
            "taarib_renpy/jisr.py",
            "taarib/khutut/OFL.txt",
            "taarib/Amiri-Regular.ttf",
            "taarib/khutut/khass/Amiri-Regular.ttf",
        ]);
        assert_eq!(
            ikhtar_khatt_renpy(&listing),
            None,
            "a face outside the component's font directory, or nested below it, is not the \
             face the component ships"
        );
    }

    #[test]
    fn yukhtaru_alnaskh_qabl_alsans() {
        let listing = asmaa(&[
            "taarib/khutut/Cairo[slnt,wght].ttf",
            "taarib/khutut/IBMPlexSansArabic-Regular.ttf",
            "taarib/khutut/NotoNaskhArabic[wght].ttf",
            "taarib/khutut/OFL.txt",
            "taarib_renpy/__init__.py",
        ]);
        assert_eq!(
            ikhtar_khatt_renpy(&listing),
            Some("taarib/khutut/NotoNaskhArabic[wght].ttf"),
            "a visual novel's text is body copy, and the ranking says Naskh before sans"
        );
    }

    #[test]
    fn wajh_ghayr_musamma_yufaddal_ala_la_shay() {
        // A bundle that staged one Kufi face into the Ren'Py component meant to
        // ship it. Answering "no font" would be this module overruling the build.
        let listing = asmaa(&["taarib/khutut/ReemKufi[wght].ttf", "taarib_renpy/__init__.py"]);
        assert_eq!(
            ikhtar_khatt_renpy(&listing),
            Some("taarib/khutut/ReemKufi[wght].ttf"),
            "the preference list ranks; it does not filter"
        );
    }

    #[test]
    fn alikhtiyar_thabit_bayn_alaalat() {
        // Two faces of equal rank arrive in whatever order the filesystem
        // walked them. The answer must not depend on that.
        let sanad = asmaa(&["taarib/khutut/Zawaya.otf", "taarib/khutut/Alif.ttf"]);
        let maqlub = asmaa(&["taarib/khutut/Alif.ttf", "taarib/khutut/Zawaya.otf"]);
        assert_eq!(ikhtar_khatt_renpy(&sanad), Some("taarib/khutut/Alif.ttf"));
        assert_eq!(ikhtar_khatt_renpy(&sanad), ikhtar_khatt_renpy(&maqlub));
    }

    #[test]
    fn alism_almurjaa_huwa_almasar_dakhil_game() {
        // What `iktub_idad` writes into the `.rpy` is this string verbatim, and
        // what `renpy` deploys is `game/` joined to the same store-relative
        // name. The two are the same value, which is the whole invariant.
        let listing = asmaa(&["taarib/khutut/Amiri-Regular.ttf"]);
        let ikhtiyar = ikhtar_khatt_renpy(&listing).expect("a face");
        assert!(ikhtiyar.starts_with(MUJALLAD_KHATT_RENPY));
        assert_eq!(listing.first().map(String::as_str), Some(ikhtiyar));
    }
}
