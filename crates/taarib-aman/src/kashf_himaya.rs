//! كشف الحماية — anti-cheat detection by hard evidence: file, module, service, driver and store signature.
//!
//! # What a partial match means here
//!
//! [`Thiqa`] grades the *marker*, not the verdict. The gate above this module
//! refuses on any evidence at all — [`mahmiya`] is `!adilla.is_empty()` — so a
//! [`Thiqa::Rajiha`] marker refuses exactly as hard as a [`Thiqa::Muakkada`]
//! one, and there is no override for either. The grade is what the user is
//! shown so that a refusal can be argued with; it is not a threshold, and no
//! caller may treat it as one.
//!
//! That makes the bar for adding a marker the same as the bar for refusing an
//! install: a name is only written into [`ALAMAT`] when meeting it inside one
//! game's own directory is, on its own, reason enough to refuse. A name too
//! short or too widely shared to carry that is not softened into a weaker tier —
//! it is left out, because a false positive here blocks a legitimate install
//! with no way around it and a false negative permits an account-permanent ban.
//! `Rajiha` marks the handful whose *name* is shared with something outside
//! games — `NGService.exe`, `faceit` — and which are decisive only because this
//! scan is rooted at a single game folder and never leaves it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use object::read::Object as _;
use serde::{Deserialize, Serialize};
use taarib_kashf::matajir::vdf::QeemaVdf;
use taarib_usus::manassa;

use crate::matjar::QiraatMatjar;

/// A known anti-cheat, as a closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawHimaya {
    /// Easy Anti-Cheat, the standalone form.
    EasyAntiCheat,
    /// Easy Anti-Cheat delivered through Epic Online Services.
    EasyAntiCheatEos,
    /// `BattlEye`.
    BattlEye,
    /// EA Javelin, the kernel-mode anti-cheat EA ships with its own titles.
    EaJavelin,
    /// Denuvo Anti-Cheat, distinct from the Denuvo anti-tamper DRM.
    Denuvo,
    /// Riot Vanguard.
    Vanguard,
    /// nProtect `GameGuard`.
    GameGuard,
    /// XIGNCODE3.
    Xigncode3,
    /// Tencent's Anti-Cheat Expert, the `ACE` of Delta Force and Wuthering Waves.
    AntiCheatExpert,
    /// `NetEase`'s `NEAC` Protect.
    NeacProtect,
    /// Nexon Game Security, which the games it protects call `NGS` or `BlackCipher`.
    NexonGameSecurity,
    /// `HoYoverse`/miHoYo's own kernel protection.
    MihoyoProtect,
    /// `PunkBuster`.
    PunkBuster,
    /// FACEIT Anti-Cheat.
    FaceitAc,
    /// ESEA's client anti-cheat.
    Esea,
    /// Activision Ricochet.
    Ricochet,
    /// Valve Anti-Cheat.
    Vac,
    /// An anti-cheat the store declares without naming which one it is.
    ///
    /// Steam publishes the result of its own compatibility testing per app, and
    /// two of the verdicts it can record are "this game's anti-cheat is not
    /// configured for our runtime" and "this game uses an anti-cheat we do not
    /// support". Both are Valve asserting that a client-side anti-cheat exists;
    /// neither says whose. That is less than the other kinds carry and it is
    /// still a store telling us an account is exposed, so it is recorded as its
    /// own kind rather than guessed into one of the named ones.
    GhayrMusamma,
}

impl NawHimaya {
    /// The Arabic display name.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::EasyAntiCheat => "إيزي أنتي-تشيت",
            Self::EasyAntiCheatEos => "إيزي أنتي-تشيت (عبر خدمات إيبك أونلاين)",
            Self::BattlEye => "باتل آي",
            Self::EaJavelin => "جافلين لمكافحة الغش من EA",
            Self::Denuvo => "دينوفو لمكافحة الغش",
            Self::Vanguard => "ريوت فانغارد",
            Self::GameGuard => "جيم غارد من nProtect",
            Self::Xigncode3 => "زين كود ٣",
            Self::AntiCheatExpert => "خبير مكافحة الغش (ACE) من تنسنت",
            Self::NeacProtect => "نياك بروتكت من نت إيز",
            Self::NexonGameSecurity => "حماية ألعاب نكسون (NGS)",
            Self::MihoyoProtect => "حماية ميهويو",
            Self::PunkBuster => "بانك باستر",
            Self::FaceitAc => "مضاد الغش من فيسإت",
            Self::Esea => "مضاد الغش من ESEA",
            Self::Ricochet => "ريكوشيه من أكتيفجن",
            Self::Vac => "مكافحة الغش من فالف (VAC)",
            Self::GhayrMusamma => "نظام مكافحة غش يُعلنه المتجر دون تسميته",
        }
    }

    /// The English display name.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::EasyAntiCheat => "Easy Anti-Cheat",
            Self::EasyAntiCheatEos => "Easy Anti-Cheat (EOS)",
            Self::BattlEye => "BattlEye",
            Self::EaJavelin => "EA Javelin Anticheat",
            Self::Denuvo => "Denuvo Anti-Cheat",
            Self::Vanguard => "Riot Vanguard",
            Self::GameGuard => "nProtect GameGuard",
            Self::Xigncode3 => "XIGNCODE3",
            Self::AntiCheatExpert => "Anti-Cheat Expert (Tencent ACE)",
            Self::NeacProtect => "NEAC Protect (NetEase)",
            Self::NexonGameSecurity => "Nexon Game Security",
            Self::MihoyoProtect => "miHoYo Protect",
            Self::PunkBuster => "PunkBuster",
            Self::FaceitAc => "FACEIT Anti-Cheat",
            Self::Esea => "ESEA Anti-Cheat",
            Self::Ricochet => "Activision Ricochet",
            Self::Vac => "Valve Anti-Cheat (VAC)",
            Self::GhayrMusamma => "an anti-cheat the store declares but does not name",
        }
    }
}

/// The kind of evidence a detection rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawDaleel {
    /// A file or directory present on disk under the game.
    MalafMawjud,
    /// A module a game binary loads, read from its import table.
    WahdaMuhammala,
    /// A service or process running now, from inside the game's own files.
    KhidmaTashtaghil,
    /// A kernel-mode driver file.
    MushaghghilNawat,
    /// A signature in the store's own catalogue.
    TawqeeMatjar,
}

impl NawDaleel {
    /// The Arabic name of this evidence kind.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::MalafMawjud => "ملف على القرص",
            Self::WahdaMuhammala => "وحدة محمَّلة في ملف اللعبة",
            Self::KhidmaTashtaghil => "خدمة قيد التشغيل",
            Self::MushaghghilNawat => "مشغِّل نواة",
            Self::TawqeeMatjar => "توقيع في فهرس المتجر",
        }
    }

    /// The English name of this evidence kind.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::MalafMawjud => "a file on disk",
            Self::WahdaMuhammala => "a module the game binary loads",
            Self::KhidmaTashtaghil => "a running service",
            Self::MushaghghilNawat => "a kernel driver",
            Self::TawqeeMatjar => "a store-catalogue signature",
        }
    }
}

/// How decisive a single marker is on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thiqa {
    /// The marker belongs to this anti-cheat and to nothing else.
    Muakkada,
    /// The marker is strong but short or shared enough to name as less than certain.
    Rajiha,
}

impl Thiqa {
    /// The Arabic word for this confidence.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::Muakkada => "مؤكَّد",
            Self::Rajiha => "راجِح",
        }
    }

    /// The English word for this confidence.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::Muakkada => "certain",
            Self::Rajiha => "strong",
        }
    }
}

/// One piece of hard evidence that a specific anti-cheat is present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaleelHimaya {
    /// Which anti-cheat this names.
    pub naw: NawHimaya,
    /// The kind of evidence it rests on.
    pub sinf: NawDaleel,
    /// Where it was found: a relative path, module name, process name, or catalogue key.
    pub ayn: String,
    /// The absolute path behind the evidence, when there is one.
    pub masar: Option<PathBuf>,
    /// How decisive the marker is.
    pub thiqa: Thiqa,
}

impl DaleelHimaya {
    /// The evidence named in one Arabic line.
    #[must_use]
    pub fn arabi(&self) -> String {
        format!("{}: {} — {}", self.naw.arabi(), self.sinf.arabi(), self.ayn)
    }

    /// The evidence named in one English line.
    #[must_use]
    pub fn injilizi(&self) -> String {
        format!(
            "{}: {} — {}",
            self.naw.injilizi(),
            self.sinf.injilizi(),
            self.ayn
        )
    }
}

/// A place the scan could not read, so absence there is not proof of absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThughraFahs {
    /// The path that could not be read or was refused.
    pub masar: PathBuf,
    /// The reason, as a stable short label.
    pub sabab: String,
}

/// Everything one scan found, and everything it could not reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IjmaaHimaya {
    /// The game root that was scanned.
    pub jidhr: PathBuf,
    /// Every distinct piece of evidence, in the order it was found.
    pub adilla: Vec<DaleelHimaya>,
    /// Places the scan could not read, so a thin report is told from a clean one.
    pub thughrat: Vec<ThughraFahs>,
    /// The filesystem walk stopped at a bound before it finished.
    pub mabtur: bool,
}

impl IjmaaHimaya {
    /// The distinct anti-cheats named by the evidence, in a stable order.
    #[must_use]
    pub fn anwa(&self) -> Vec<NawHimaya> {
        let mut ruit: BTreeSet<NawHimaya> = BTreeSet::new();
        for daleel in &self.adilla {
            let _ = ruit.insert(daleel.naw);
        }
        ruit.into_iter().collect()
    }
}

/// Whether any evidence at all was found — the one question installation asks.
#[must_use]
pub const fn mahmiya(ijmaa: &IjmaaHimaya) -> bool {
    !ijmaa.adilla.is_empty()
}

/// What became of the store-catalogue half of a scan.
///
/// Kept apart from [`IjmaaHimaya`] because it is not evidence; it is whether
/// the question was put at all. VAC ships nothing whatever into a game folder —
/// it is declared in Steam's catalogue and nowhere else — so a scan that could
/// not read the catalogue returns an evidence list byte-identical to the one a
/// genuinely clean game returns. Everything downstream that mints an
/// authorisation has to be able to tell those two apart, and
/// [`ThughraFahs`] alone cannot: a gap is a place, not a claim about which
/// check it cost.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalatMatjar {
    /// No catalogue applies. The game carries no Steam identity, so there is
    /// nothing about it for Steam to have declared, and its absence proves
    /// nothing either way. Most games in most libraries.
    GhayrMatlub,
    /// The catalogue was read, and whatever it said is in the evidence list.
    Maqru,
    /// The game is a Steam game and no Steam installation was named, so the
    /// catalogue could not even be looked for.
    JidhrMajhul,
    /// The catalogue was named and could not be read.
    Mutaadhdhir {
        /// The `appcache/appinfo.vdf` that was tried.
        masar: PathBuf,
        /// Why, as the same short label the gap list carries.
        sabab: String,
    },
}

impl HalatMatjar {
    /// Whether the catalogue was owed and never read, so the VAC question went
    /// unanswered rather than answered "no".
    #[must_use]
    pub const fn lam_yuqra(&self) -> bool {
        matches!(self, Self::JidhrMajhul | Self::Mutaadhdhir { .. })
    }
}

/// Deepest level the walk descends; a filesystem root is refused, not walked whole.
const AQSA_UMQ: usize = 8;

/// Entry ceiling for one walk; past it the scan stops and marks the report partial.
const AQSA_MADAKHIL: usize = 200_000;

/// How many executables are opened for import inspection, bounding the reads.
const AQSA_TANFIDHIYAT: usize = 8;

/// Largest executable read whole for imports; a larger one is left to disk markers.
const HADD_QIRA: u64 = 64 * 1024 * 1024;

/// Cap on recorded evidence, so one folder cannot produce an unbounded report.
const AQSA_ADILLA: usize = 256;

/// The Steam store category number that means Valve Anti-Cheat is enabled.
const FIAT_VAC: u32 = 8;

/// File extensions whose import table is worth reading.
const IMTIDADAT_MUSTAWRAD: [&str; 4] = ["exe", "dll", "so", "dylib"];

/// Anti-cheat keywords as the store catalogue spells them.
///
/// `eaanticheat` is here because a Javelin-protected app names the anti-cheat
/// launcher as its `config/launch` executable rather than the game — Steam's own
/// catalogue on this machine spells it `EAAntiCheat.GameServiceLauncher.exe`
/// across every one of Battlefield 6's seventeen launch entries — so the
/// catalogue names Javelin for a game that is not even installed yet.
const KALIMAT_MATJAR: [(&str, NawHimaya); 7] = [
    ("start_protected_game", NawHimaya::EasyAntiCheat),
    ("easyanticheat_eos", NawHimaya::EasyAntiCheatEos),
    ("easyanticheat", NawHimaya::EasyAntiCheat),
    ("easy anti-cheat", NawHimaya::EasyAntiCheat),
    ("battleye", NawHimaya::BattlEye),
    ("eaanticheat", NawHimaya::EaJavelin),
    ("eajavelin", NawHimaya::EaJavelin),
];

/// The three sibling maps Steam writes its own compatibility test results into.
const FUHUS_TAWAFUQ: [&str; 3] = ["tests", "steam_machine_tests", "steamos_tests"];

/// The fragment both of Valve's anti-cheat test-result tokens carry, folded.
///
/// The two are `#SteamDeckVerified_TestResult_UnsupportedAntiCheatConfiguration`
/// and `#SteamDeckVerified_TestResult_UnsupportedAntiCheat_Other`, each with a
/// `#SteamMachine_` and a `#SteamOS_` twin. Every one of them contains this.
const RAMZ_HIMAYA_TAWAFUQ: &str = "unsupportedanticheat";

/// Where a marker is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MahalAlama {
    /// A file or directory name that contains the marker.
    MalafJuzi,
    /// A file or directory name equal to the marker.
    MalafKamil,
    /// A `.sys` driver whose stem contains the marker.
    Mushaghghil,
    /// A module named in a game binary's import table.
    Wahda,
    /// A running process whose name is the marker.
    Khidma,
}

/// One marker: a needle, where it is looked for, and how much it is worth.
#[derive(Debug, Clone, Copy)]
struct Alama {
    naw: NawHimaya,
    ibra: &'static str,
    /// A name that also carries this token is not this marker.
    illa: Option<&'static str>,
    mahal: MahalAlama,
    thiqa: Thiqa,
}

/// Every marker this build recognises.
const ALAMAT: &[Alama] = &[
    // EOSSDK alone is Epic Online Services, not anti-cheat, so it is never a marker.
    Alama {
        naw: NawHimaya::EasyAntiCheatEos,
        ibra: "easyanticheat_eos",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheatEos,
        ibra: "easyanticheat_eos",
        illa: None,
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheatEos,
        ibra: "easyanticheat_eos",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheatEos,
        ibra: "easyanticheat_eos",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheat,
        ibra: "easyanticheat",
        illa: Some("eos"),
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheat,
        ibra: "easyanticheat",
        illa: Some("eos"),
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheat,
        ibra: "easyanticheat",
        illa: Some("eos"),
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheat,
        ibra: "easyanticheat",
        illa: Some("eos"),
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EasyAntiCheat,
        ibra: "start_protected_game",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "battleye",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "beclient",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "beclient",
        illa: None,
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "beservice",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "beservice",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "beservice_x64",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::BattlEye,
        ibra: "bedaisy",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    // `eaanticheat` is not a substring of `easyanticheat` — the second has no
    // `eaa` — so these two families never collide and neither needs an `illa`.
    // The token covers everything EA drops in a protected game's root:
    // `EAAntiCheat.GameServiceLauncher.exe`/`.dll` and their `_b` rollback
    // copies, `EAAntiCheat.Installer.exe`, `EAAntiCheat.cfg` (a signed,
    // resource-only PE, not text), `EAAntiCheat.splash.png`, and the
    // `__Installer/EAAntiCheat/` directory.
    Alama {
        naw: NawHimaya::EaJavelin,
        ibra: "eaanticheat",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::EaJavelin,
        ibra: "eaanticheat",
        illa: None,
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Muakkada,
    },
    // The launcher is the process the store starts *instead of* the game, so it
    // runs from the game's own directory and the containment test can attribute
    // it. `EAAntiCheat.GameService.exe`, the persistent service, installs to
    // `Program Files\EA\AC` and is deliberately not claimed here: it is outside
    // every game root and would be attributed to whichever game was scanned.
    Alama {
        naw: NawHimaya::EaJavelin,
        ibra: "eaanticheat.gameservicelauncher",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    // Steam installs Javelin through a per-game install script the game ships.
    Alama {
        naw: NawHimaya::EaJavelin,
        ibra: "eajavelin",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    // A bare denuvo name is DRM, not anti-cheat; only a driver or -anti-cheat name counts.
    Alama {
        naw: NawHimaya::Denuvo,
        ibra: "denuvo-anti-cheat",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Denuvo,
        ibra: "denuvoanticheat",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Denuvo,
        ibra: "denuvo-anti-cheat",
        illa: None,
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Denuvo,
        ibra: "denuvo",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Vanguard,
        ibra: "vgk",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Vanguard,
        ibra: "riot vanguard",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Vanguard,
        ibra: "vgc",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Vanguard,
        ibra: "vgtray",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::GameGuard,
        ibra: "gameguard",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::GameGuard,
        ibra: "npggnt",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::GameGuard,
        ibra: "gamemon",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::GameGuard,
        ibra: "gamemon",
        illa: None,
        mahal: MahalAlama::Wahda,
        thiqa: Thiqa::Rajiha,
    },
    Alama {
        naw: NawHimaya::Xigncode3,
        ibra: "xigncode",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Xigncode3,
        ibra: "x3.xem",
        illa: None,
        mahal: MahalAlama::MalafKamil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Xigncode3,
        ibra: "xhunter1",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    // ACE puts an `AntiCheatExpert` directory inside the game's own
    // `Binaries/Win64`, holding its setup binary, and installs `ACE-BASE.sys`
    // and `ACE-GAME.sys` as drivers — the base driver being the one with a
    // published privilege-escalation advisory, which is how the name is
    // independently attested rather than only observed.
    Alama {
        naw: NawHimaya::AntiCheatExpert,
        ibra: "anticheatexpert",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::AntiCheatExpert,
        ibra: "ace-base",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    // A crashed load leaves `ace-game-0.sys` behind, so the stem is matched
    // by containment rather than equality.
    Alama {
        naw: NawHimaya::AntiCheatExpert,
        ibra: "ace-game",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::NeacProtect,
        ibra: "neacsafe",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::NeacProtect,
        ibra: "neacsafe",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::NeacProtect,
        ibra: "neacclient",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::NeacProtect,
        ibra: "neacinterface",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::NexonGameSecurity,
        ibra: "blackcipher",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    // `NGService.exe` is a name an antivirus vendor also uses, so on its own it
    // is only strong — but this scan is rooted at one game's own directory, and
    // nothing else puts that name there. The service test is bounded the same
    // way: a process outside the game root is never attributed to the game.
    Alama {
        naw: NawHimaya::NexonGameSecurity,
        ibra: "ngservice",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Rajiha,
    },
    Alama {
        naw: NawHimaya::NexonGameSecurity,
        ibra: "ngservice",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Rajiha,
    },
    // Both of these sit beside the game executable, and the `.sys` one ships
    // with mixed case (`mhyprot3.Sys`), which the folded comparison absorbs.
    Alama {
        naw: NawHimaya::MihoyoProtect,
        ibra: "mhyprot",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::MihoyoProtect,
        ibra: "mhyprot",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::MihoyoProtect,
        ibra: "hoyokprotect",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::MihoyoProtect,
        ibra: "hoyokprotect",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::PunkBuster,
        ibra: "pnkbstr",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::PunkBuster,
        ibra: "pbsvc",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::PunkBuster,
        ibra: "pnkbstra",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::PunkBuster,
        ibra: "pnkbstrb",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::FaceitAc,
        ibra: "faceit",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Rajiha,
    },
    Alama {
        naw: NawHimaya::FaceitAc,
        ibra: "faceit",
        illa: None,
        mahal: MahalAlama::Mushaghghil,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::FaceitAc,
        ibra: "faceitservice",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Esea,
        ibra: "eseaclient",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Esea,
        ibra: "eseaclient",
        illa: None,
        mahal: MahalAlama::Khidma,
        thiqa: Thiqa::Muakkada,
    },
    Alama {
        naw: NawHimaya::Esea,
        ibra: "esea",
        illa: None,
        mahal: MahalAlama::MalafKamil,
        thiqa: Thiqa::Rajiha,
    },
    // Ricochet ships no stable public filename; only a literal `ricochet` name is claimed.
    Alama {
        naw: NawHimaya::Ricochet,
        ibra: "ricochet",
        illa: None,
        mahal: MahalAlama::MalafJuzi,
        thiqa: Thiqa::Rajiha,
    },
];

/// Scans one game for anti-cheat evidence.
///
/// The store catalogue is read too when both `masdar_luba_appid` and
/// `jidhr_steam` are given.
///
/// Never fails: a place it cannot read becomes a [`ThughraFahs`] in the report.
///
/// This is the reading surface — a diagnostics screen, a game card, a warning
/// banner — where a thin report is a thing to *show*. Anything that decides
/// whether to write into a game must call [`ifhas_himaya_bi_matjar`] instead
/// and act on the [`HalatMatjar`] it also returns, because the evidence list
/// alone cannot say whether the VAC check ran.
#[must_use]
pub fn ifhas_himaya(
    jidhr_luba: &Path,
    masdar_luba_appid: Option<u32>,
    jidhr_steam: Option<&Path>,
) -> IjmaaHimaya {
    ifhas_himaya_bi_matjar(jidhr_luba, masdar_luba_appid, jidhr_steam).0
}

/// The same scan, and what became of its store-catalogue half.
///
/// One pass, not two: a caller that wanted both answers by calling twice would
/// read `appinfo.vdf` twice, and that file is measured in megabytes on a mature
/// account.
///
/// Never fails, for the same reason [`ifhas_himaya`] does not — the catalogue's
/// fate is a value here rather than an error, so that a caller cannot receive
/// it and discard it by writing `.ok()`.
#[must_use]
pub fn ifhas_himaya_bi_matjar(
    jidhr_luba: &Path,
    masdar_luba_appid: Option<u32>,
    jidhr_steam: Option<&Path>,
) -> (IjmaaHimaya, HalatMatjar) {
    ifhas_himaya_bi_qiraa(
        jidhr_luba,
        &QiraatMatjar::iqra(masdar_luba_appid, jidhr_steam),
    )
}

/// The same scan again, against a catalogue reading the caller already has.
///
/// [`crate::fahs::fahs`] runs this scan *and* [`crate::kashf_shabaka`]'s on the
/// same game before it will authorise anything, and both want the same
/// `appinfo.vdf`. This entry point is how that file gets read once instead of
/// twice: [`QiraatMatjar::iqra`] opens it, and each probe is handed the half it
/// is about.
#[must_use]
pub fn ifhas_himaya_bi_qiraa(
    jidhr_luba: &Path,
    matjar: &QiraatMatjar,
) -> (IjmaaHimaya, HalatMatjar) {
    let mut musajjil = Musajjil::default();

    imsah_luba(jidhr_luba, &mut musajjil);
    ifhas_khidmat(jidhr_luba, &mut musajjil);

    for daleel in &matjar.himaya {
        musajjil.sajjil(daleel.clone());
    }
    // Still a gap on the report as well: the surface that only displays this
    // scan must keep showing the same fact the install path now refuses over.
    if let HalatMatjar::Mutaadhdhir { masar, sabab } = &matjar.hala {
        musajjil.thughra(masar.clone(), sabab.clone());
    }

    let ijmaa = IjmaaHimaya {
        jidhr: jidhr_luba.to_path_buf(),
        adilla: musajjil.adilla,
        thughrat: musajjil.thughrat,
        mabtur: musajjil.mabtur,
    };
    (ijmaa, matjar.hala.clone())
}

/// Accumulates evidence, dropping duplicates, capping the total, and recording gaps.
#[derive(Debug, Default)]
struct Musajjil {
    adilla: Vec<DaleelHimaya>,
    ruyat: BTreeSet<String>,
    thughrat: Vec<ThughraFahs>,
    mabtur: bool,
}

impl Musajjil {
    /// Records one detection, unless it is a duplicate or the cap is reached.
    fn sajjil(&mut self, daleel: DaleelHimaya) {
        if self.adilla.len() >= AQSA_ADILLA {
            self.mabtur = true;
            return;
        }
        let ayn = daleel.ayn.to_ascii_lowercase();
        let miftah = format!("{:?}|{:?}|{ayn}", daleel.naw, daleel.sinf);
        if self.ruyat.insert(miftah) {
            self.adilla.push(daleel);
        }
    }

    /// Records a place the scan could not read.
    fn thughra(&mut self, masar: PathBuf, sabab: impl Into<String>) {
        self.thughrat.push(ThughraFahs {
            masar,
            sabab: sabab.into(),
        });
    }
}

/// One directory entry reduced to what the marker tests ask about.
#[derive(Debug)]
struct MadkhalMafhus<'a> {
    /// The name, folded to ASCII lowercase.
    ism: &'a str,
    /// The lowercased stem, for the driver test.
    jidhr_ism: &'a str,
    /// The lowercased extension, empty when there is none.
    imtidad: &'a str,
    /// Whether it is a directory.
    mujallad: bool,
    /// The absolute path.
    masar: &'a Path,
    /// The path as the report names it: relative to the game root.
    nisbi: String,
}

/// Walks the game folder, bounded, and records the on-disk markers it meets.
fn imsah_luba(jidhr: &Path, musajjil: &mut Musajjil) {
    if jidhr.parent().is_none() {
        musajjil.thughra(
            jidhr.to_path_buf(),
            "refused: scanning a filesystem root would walk an entire drive",
        );
        return;
    }
    if !jidhr.is_dir() {
        musajjil.thughra(jidhr.to_path_buf(), "not a directory");
        return;
    }

    let mut adad: usize = 0;
    let mut tanfidhiyat: Vec<PathBuf> = Vec::new();

    let mashi = walkdir::WalkDir::new(jidhr)
        .min_depth(1)
        .max_depth(AQSA_UMQ)
        .follow_links(false)
        .sort_by_file_name();

    for natija in mashi {
        let madkhal = match natija {
            Ok(madkhal) => madkhal,
            Err(khata) => {
                let masar = khata
                    .path()
                    .map_or_else(|| jidhr.to_path_buf(), Path::to_path_buf);
                let sabab = khata
                    .io_error()
                    .map_or_else(|| "walk error".to_owned(), |io| format!("{:?}", io.kind()));
                musajjil.thughra(masar, sabab);
                continue;
            },
        };

        adad = adad.saturating_add(1);
        if adad > AQSA_MADAKHIL {
            musajjil.mabtur = true;
            break;
        }

        let naw = madkhal.file_type();
        // A symlink is never followed to decide presence; its target is outside our control.
        if naw.is_symlink() {
            continue;
        }

        let ism = madkhal.file_name().to_string_lossy().to_ascii_lowercase();
        let masar = madkhal.path();
        let (jidhr_ism, imtidad) = qism_ism(&ism);
        let mafhus = MadkhalMafhus {
            ism: &ism,
            jidhr_ism: &jidhr_ism,
            imtidad: &imtidad,
            mujallad: naw.is_dir(),
            masar,
            nisbi: nisbi(jidhr, masar),
        };
        fahs_madkhal(&mafhus, musajjil);

        if naw.is_file()
            && IMTIDADAT_MUSTAWRAD.contains(&imtidad.as_str())
            && tanfidhiyat.len() < AQSA_TANFIDHIYAT
            && madkhal.metadata().map_or(u64::MAX, |bayanat| bayanat.len()) <= HADD_QIRA
        {
            tanfidhiyat.push(masar.to_path_buf());
        }
    }

    for masar in tanfidhiyat {
        ifhas_mustawradat(jidhr, &masar, musajjil);
    }
}

/// Tests one directory entry against every file, driver and directory marker.
fn fahs_madkhal(mafhus: &MadkhalMafhus<'_>, musajjil: &mut Musajjil) {
    for alama in ALAMAT {
        let sinf = match alama.mahal {
            MahalAlama::MalafJuzi | MahalAlama::MalafKamil => NawDaleel::MalafMawjud,
            MahalAlama::Mushaghghil => NawDaleel::MushaghghilNawat,
            MahalAlama::Wahda | MahalAlama::Khidma => continue,
        };
        let mutabiq = match alama.mahal {
            MahalAlama::MalafJuzi => yahwi(mafhus.ism, alama),
            MahalAlama::MalafKamil => mafhus.ism == alama.ibra && !mustathna(mafhus.ism, alama),
            MahalAlama::Mushaghghil => {
                !mafhus.mujallad && mafhus.imtidad == "sys" && yahwi(mafhus.jidhr_ism, alama)
            },
            MahalAlama::Wahda | MahalAlama::Khidma => false,
        };
        if mutabiq {
            musajjil.sajjil(DaleelHimaya {
                naw: alama.naw,
                sinf,
                ayn: mafhus.nisbi.clone(),
                masar: Some(mafhus.masar.to_path_buf()),
                thiqa: alama.thiqa,
            });
        }
    }
}

/// Reads one binary's import table and records the anti-cheat modules it loads.
fn ifhas_mustawradat(jidhr: &Path, masar: &Path, musajjil: &mut Musajjil) {
    let bayt = match std::fs::read(masar) {
        Ok(bayt) => bayt,
        Err(khata) => {
            musajjil.thughra(masar.to_path_buf(), format!("{:?}", khata.kind()));
            return;
        },
    };
    let Ok(kaen) = object::read::File::parse(bayt.as_slice()) else {
        return;
    };
    let Ok(mustawradat) = kaen.imports() else {
        return;
    };

    let ism = nisbi(jidhr, masar);
    for mustawrad in mustawradat {
        // The reader yields a Result per entry: a malformed table is a fact
        // about the file, and one bad entry never fails the whole scan.
        let Ok(mustawrad) = mustawrad else { continue };
        let maktaba = String::from_utf8_lossy(mustawrad.library()).to_ascii_lowercase();
        if maktaba.is_empty() {
            continue;
        }
        for alama in ALAMAT
            .iter()
            .filter(|alama| matches!(alama.mahal, MahalAlama::Wahda))
        {
            if yahwi(&maktaba, alama) {
                musajjil.sajjil(DaleelHimaya {
                    naw: alama.naw,
                    sinf: NawDaleel::WahdaMuhammala,
                    ayn: format!("{ism} → {maktaba}"),
                    masar: Some(masar.to_path_buf()),
                    thiqa: alama.thiqa,
                });
            }
        }
    }
}

/// Records an anti-cheat service only when it runs from inside the game.
fn ifhas_khidmat(jidhr: &Path, musajjil: &mut Musajjil) {
    // A filesystem root would make every running service look contained; refuse it.
    if jidhr.parent().is_none() {
        return;
    }
    let jidhr_kanuni = std::fs::canonicalize(jidhr).ok();
    let mut ruit: BTreeSet<String> = BTreeSet::new();

    for alama in ALAMAT
        .iter()
        .filter(|alama| matches!(alama.mahal, MahalAlama::Khidma))
    {
        for ism in [format!("{}.exe", alama.ibra), alama.ibra.to_owned()] {
            if !ruit.insert(ism.clone()) {
                continue;
            }
            for amaliya in manassa::amaliyat_bism(&ism) {
                let Some(masar) = amaliya.masar.as_deref() else {
                    continue;
                };
                // A service outside the game's own files is not attributed to this game.
                if !dakhil(jidhr_kanuni.as_deref(), masar) {
                    continue;
                }
                musajjil.sajjil(DaleelHimaya {
                    naw: alama.naw,
                    sinf: NawDaleel::KhidmaTashtaghil,
                    ayn: format!("{} (pid {})", amaliya.ism, amaliya.raqm),
                    masar: Some(masar.to_path_buf()),
                    thiqa: alama.thiqa,
                });
            }
        }
    }
}

/// Projects one app's `appinfo.vdf` tree into the store signatures it carries.
pub(crate) fn adillat_appinfo(bayanat: &QeemaVdf, masar: &Path) -> Vec<DaleelHimaya> {
    let mut adilla: Vec<DaleelHimaya> = Vec::new();

    let fiat = bayanat
        .kain_bi_masar(&["appinfo", "common", "category"])
        .unwrap_or(&[]);
    for (miftah, qeema) in fiat {
        // A cleared category is written as zero, not removed, so the value is checked too.
        if qeema.raqm().unwrap_or(1) == 0 {
            continue;
        }
        let raqm = miftah
            .rsplit('_')
            .next()
            .and_then(|raqm| raqm.parse::<u32>().ok());
        if raqm == Some(FIAT_VAC) {
            adilla.push(tawqee(
                NawHimaya::Vac,
                format!("appinfo.vdf: common/category/{miftah}"),
                masar,
                Thiqa::Muakkada,
            ));
        }
    }

    for (miftah, qeema) in bayanat.kain_bi_masar(&["appinfo", "config"]).unwrap_or(&[]) {
        let saghir = miftah.to_ascii_lowercase();
        // `vacmodulefilename` and the `VACMacModuleInfo` map: the second folds without `vacmodule`.
        if saghir.contains("vacmodule") || saghir.contains("vacmacmodule") {
            let dhayl = qeema
                .nass()
                .map(|nass| format!(" = {nass}"))
                .unwrap_or_default();
            adilla.push(tawqee(
                NawHimaya::Vac,
                format!("appinfo.vdf: config/{miftah}{dhayl}"),
                masar,
                Thiqa::Muakkada,
            ));
        }
    }

    for (_, madkhal) in bayanat
        .kain_bi_masar(&["appinfo", "config", "launch"])
        .unwrap_or(&[])
    {
        if let Some(tanfidhi) = madkhal.nass_bi_masar(&["executable"])
            && let Some((naw, kalima)) = matjar_himaya(tanfidhi)
        {
            adilla.push(tawqee(
                naw,
                format!("appinfo.vdf: config/launch executable names {kalima}"),
                masar,
                Thiqa::Rajiha,
            ));
        }
    }

    for (_, ittifaq) in bayanat
        .kain_bi_masar(&["appinfo", "common", "eulas"])
        .unwrap_or(&[])
    {
        for miftah in ["name", "id"] {
            if let Some(nass) = ittifaq.nass_bi_masar(&[miftah])
                && let Some((naw, kalima)) = matjar_himaya(nass)
            {
                adilla.push(tawqee(
                    naw,
                    format!("appinfo.vdf: common/eulas/{miftah} names {kalima}"),
                    masar,
                    Thiqa::Rajiha,
                ));
            }
        }
    }

    adilla.extend(adillat_tawafuq(bayanat, masar));
    adilla
}

/// The unnamed-anti-cheat evidence Valve's own compatibility testing records.
///
/// Steam stores each app's test verdicts under `common/steam_deck_compatibility`
/// as three sibling maps of `{ display, token }`, and two of the tokens it can
/// write say, in Valve's words, that the game's anti-cheat is unsupported or
/// misconfigured for its runtime. This is the only marker in the build that
/// names no product, and it is [`Thiqa::Rajiha`] for exactly that reason: it
/// establishes that a client-side anti-cheat exists, not which one.
///
/// It is worth reading because it answers for games whose anti-cheat this build
/// has no file signature for at all, and because it answers before the game is
/// installed. On the catalogue this was written against — 688 apps — fourteen
/// carry a token, and every one of the fourteen is a game with a real,
/// account-banning anti-cheat: `BattlEye` (GTA V, Rainbow Six Siege, PUBG,
/// Destiny 2), Easy Anti-Cheat (Vermintide 2, The First Descendant), EA's
/// (Apex, Battlefield 1/V/2042), Ricochet (Call of Duty). No false positive.
fn adillat_tawafuq(bayanat: &QeemaVdf, masar: &Path) -> Vec<DaleelHimaya> {
    let mut adilla: Vec<DaleelHimaya> = Vec::new();
    for fahs in FUHUS_TAWAFUQ {
        let masar_fahs = ["appinfo", "common", "steam_deck_compatibility", fahs];
        for (miftah, natija) in bayanat.kain_bi_masar(&masar_fahs).unwrap_or(&[]) {
            let Some(ramz) = natija.nass_bi_masar(&["token"]) else {
                continue;
            };
            if ramz.to_ascii_lowercase().contains(RAMZ_HIMAYA_TAWAFUQ) {
                adilla.push(tawqee(
                    NawHimaya::GhayrMusamma,
                    format!(
                        "appinfo.vdf: common/steam_deck_compatibility/{fahs}/{miftah}/token \
                         = {ramz}"
                    ),
                    masar,
                    Thiqa::Rajiha,
                ));
            }
        }
    }
    adilla
}

/// Builds one store-signature detection.
fn tawqee(naw: NawHimaya, ayn: String, masar: &Path, thiqa: Thiqa) -> DaleelHimaya {
    let masar = Some(masar.to_path_buf());
    DaleelHimaya {
        naw,
        sinf: NawDaleel::TawqeeMatjar,
        ayn,
        masar,
        thiqa,
    }
}

/// The anti-cheat a catalogue string names, and the keyword that named it.
fn matjar_himaya(nass: &str) -> Option<(NawHimaya, &'static str)> {
    let saghir = nass.to_ascii_lowercase();
    KALIMAT_MATJAR
        .into_iter()
        .find(|(kalima, _)| saghir.contains(*kalima))
        .map(|(kalima, naw)| (naw, kalima))
}

/// Whether a name carries a marker and is not one the marker excludes.
fn yahwi(nass: &str, alama: &Alama) -> bool {
    nass.contains(alama.ibra) && !mustathna(nass, alama)
}

/// Whether a name carries the marker's exclusion token.
fn mustathna(nass: &str, alama: &Alama) -> bool {
    alama.illa.is_some_and(|illa| nass.contains(illa))
}

/// A name split into a lowercased stem and extension.
fn qism_ism(ism: &str) -> (String, String) {
    match ism.rsplit_once('.') {
        Some((jidhr, imtidad)) if !jidhr.is_empty() => (jidhr.to_owned(), imtidad.to_owned()),
        _ => (ism.to_owned(), String::new()),
    }
}

/// A path as the report names it: relative to the game root when it is under it.
fn nisbi(jidhr: &Path, masar: &Path) -> String {
    masar
        .strip_prefix(jidhr)
        .unwrap_or(masar)
        .display()
        .to_string()
}

/// Whether `masar` resolves under `jidhr`, following links; with no resolved
/// root, nothing is claimed.
fn dakhil(jidhr: Option<&Path>, masar: &Path) -> bool {
    let Some(jidhr) = jidhr else {
        return false;
    };
    match std::fs::canonicalize(masar) {
        Ok(kanuni) => kanuni.starts_with(jidhr),
        Err(_) => masar.starts_with(jidhr),
    }
}

#[cfg(test)]
mod ikhtibarat {
    use std::collections::BTreeMap;
    use std::error::Error;
    use std::fs;

    use taarib_kashf::matajir::vdf;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// Builds a game folder holding exactly these relative paths, and scans it.
    ///
    /// The directory a path implies is created too, which is how the folder
    /// markers — `__Installer/EAAntiCheat`, `Binaries/Win64/AntiCheatExpert` —
    /// come into existence without being listed twice.
    fn ifhas_asma(asma: &[&str]) -> Result<(tempfile::TempDir, IjmaaHimaya), Box<dyn Error>> {
        let masrah = tempfile::tempdir()?;
        let jidhr = masrah.path().join("luba");
        fs::create_dir_all(&jidhr)?;
        for ism in asma {
            let masar = jidhr.join(ism);
            if let Some(walid) = masar.parent() {
                fs::create_dir_all(walid)?;
            }
            fs::write(&masar, b"")?;
        }
        let ijmaa = ifhas_himaya(&jidhr, None, None);
        Ok((masrah, ijmaa))
    }

    /// The kinds a set of file names produces, in the report's own order.
    fn anwa_asma(asma: &[&str]) -> Result<Vec<NawHimaya>, Box<dyn Error>> {
        let (_masrah, ijmaa) = ifhas_asma(asma)?;
        Ok(ijmaa.anwa())
    }

    /// Reads a text VDF fixture into the shape `adillat_appinfo` is handed.
    fn shajarat_appinfo(nass: &str) -> Result<QeemaVdf, Box<dyn Error>> {
        vdf::iqra_nassi(nass).map_err(Into::into)
    }

    /// `EA SPORTS FC 26`'s root, as it is on the machine this was written on.
    ///
    /// Every name here was read off `F:\SteamLibrary\steamapps\common\FC 26`.
    /// The `_b` pair are EA's own rollback copies, and `EAAntiCheat.cfg` is a
    /// signed resource-only PE rather than text — neither changes the answer,
    /// and both are here so that the fixture is the real directory and not a
    /// tidied version of it.
    const JIDHR_FC26: [&str; 9] = [
        "EAAntiCheat.cfg",
        "EAAntiCheat.GameServiceLauncher.dll",
        "EAAntiCheat.GameServiceLauncher.dll_b",
        "EAAntiCheat.GameServiceLauncher.exe",
        "EAAntiCheat.Installer.exe",
        "EAAntiCheat.splash.png",
        "EAJavelinInstaller_installscript.vdf",
        "__Installer/EAAntiCheat/EAAntiCheat.Installer.exe",
        "FC26.exe",
    ];

    #[test]
    fn jidhr_fc26_yusammi_javelin_wa_yurfad() -> NatijatIkhtibar {
        let (_masrah, ijmaa) = ifhas_asma(&JIDHR_FC26)?;
        assert!(
            mahmiya(&ijmaa),
            "a Javelin-protected root must not read as clean"
        );
        assert_eq!(ijmaa.anwa(), vec![NawHimaya::EaJavelin]);
        assert!(
            ijmaa.thughrat.is_empty(),
            "nothing in the fixture was out of reach"
        );

        // The refusal has to be able to point at something, so the evidence
        // names the launcher itself rather than only the folder it sits in.
        let ayunn: Vec<&str> = ijmaa
            .adilla
            .iter()
            .map(|daleel| daleel.ayn.as_str())
            .collect();
        assert!(
            ayunn
                .iter()
                .any(|ayn| ayn.contains("EAAntiCheat.GameServiceLauncher.exe")),
            "{ayunn:?}"
        );
        assert!(
            ayunn.iter().any(|ayn| ayn.contains("EAJavelinInstaller")),
            "{ayunn:?}"
        );
        Ok(())
    }

    #[test]
    fn javelin_wa_easyanticheat_la_yatadakhalan() -> NatijatIkhtibar {
        // `eaanticheat` is not a substring of `easyanticheat`, and this is the
        // test that says so: neither family may claim the other's game.
        assert_eq!(anwa_asma(&["EAAntiCheat.cfg"])?, vec![NawHimaya::EaJavelin]);
        assert_eq!(
            anwa_asma(&["EasyAntiCheat/EasyAntiCheat_x64.dll"])?,
            vec![NawHimaya::EasyAntiCheat]
        );
        assert_eq!(
            anwa_asma(&["EasyAntiCheat_EOS/easyanticheat_eos_setup.exe"])?,
            vec![NawHimaya::EasyAntiCheatEos]
        );
        Ok(())
    }

    #[test]
    fn ace_yuraf_bi_mujallad_al_luba_wa_bi_mushaghghil() -> NatijatIkhtibar {
        let (_masrah, ijmaa) = ifhas_asma(&[
            "Client/Binaries/Win64/AntiCheatExpert/ACE-Setup64.exe",
            "Client/Binaries/Win64/ACE-BASE.sys",
            "Client/Binaries/Win64/ace-game-0.sys",
        ])?;
        assert_eq!(ijmaa.anwa(), vec![NawHimaya::AntiCheatExpert]);
        let asnaf: BTreeSet<NawDaleel> = ijmaa.adilla.iter().map(|daleel| daleel.sinf).collect();
        assert!(
            asnaf.contains(&NawDaleel::MushaghghilNawat),
            "a kernel driver must be reported as one: {asnaf:?}"
        );
        Ok(())
    }

    #[test]
    fn neac_yuraf_bi_thalathat_asma() -> NatijatIkhtibar {
        assert_eq!(
            anwa_asma(&["NeacSafe64.sys"])?,
            vec![NawHimaya::NeacProtect]
        );
        assert_eq!(
            anwa_asma(&["NeacClient.exe"])?,
            vec![NawHimaya::NeacProtect]
        );
        assert_eq!(
            anwa_asma(&["NeacInterface.dll"])?,
            vec![NawHimaya::NeacProtect]
        );
        Ok(())
    }

    #[test]
    fn ngs_yuraf_bi_blackcipher_wa_bi_ngservice() -> NatijatIkhtibar {
        let (_masrah, ijmaa) = ifhas_asma(&[
            "M1/Binaries/Win64/BlackCipher/BlackCipher64.aes",
            "M1/Binaries/Win64/NGService.exe",
        ])?;
        assert_eq!(ijmaa.anwa(), vec![NawHimaya::NexonGameSecurity]);

        // `NGService.exe` is a name shared with an antivirus component, so it
        // is graded `Rajiha`; `BlackCipher` is nobody else's, so it is not.
        // Both refuse — the grade is what the user is shown, not a threshold.
        let mut darajat: BTreeMap<&str, Thiqa> = BTreeMap::new();
        for daleel in &ijmaa.adilla {
            if daleel.ayn.contains("NGService") {
                let _ = darajat.insert("ngservice", daleel.thiqa);
            } else if daleel.ayn.contains("BlackCipher64") {
                let _ = darajat.insert("blackcipher", daleel.thiqa);
            }
        }
        assert_eq!(
            darajat.get("ngservice"),
            Some(&Thiqa::Rajiha),
            "{darajat:?}"
        );
        assert_eq!(
            darajat.get("blackcipher"),
            Some(&Thiqa::Muakkada),
            "{darajat:?}"
        );
        Ok(())
    }

    #[test]
    fn mihoyo_yuraf_wa_yatahammal_ikhtilaf_halat_al_ahruf() -> NatijatIkhtibar {
        // Shipped as `mhyprot3.Sys`, with that capital S. A case-sensitive
        // extension test would miss the driver and keep only the file.
        let (_masrah, ijmaa) = ifhas_asma(&["mhyprot3.Sys", "HoYoKProtect.sys"])?;
        assert_eq!(ijmaa.anwa(), vec![NawHimaya::MihoyoProtect]);
        let mushaghghilat = ijmaa
            .adilla
            .iter()
            .filter(|daleel| daleel.sinf == NawDaleel::MushaghghilNawat)
            .count();
        assert_eq!(
            mushaghghilat, 2,
            "both files are kernel drivers: {:?}",
            ijmaa.adilla
        );
        Ok(())
    }

    #[test]
    fn luba_bila_himaya_la_tuntij_dalilan() -> NatijatIkhtibar {
        // The shapes the unprotected games in the library this was written
        // against actually have: a Unity runtime, an Unreal tree, an Agility
        // SDK redistributable, and two mod loaders somebody already installed.
        let (_masrah, ijmaa) = ifhas_asma(&[
            "UnityPlayer.dll",
            "GameAssembly.dll",
            "luba_Data/sharedassets0.assets",
            "Engine/Binaries/ThirdParty/DbgHelp/dbghelp.dll",
            "luba/Binaries/Win64/luba-Win64-Shipping.exe",
            "luba/Content/Paks/luba-WindowsNoEditor.pak",
            "D3D12-REDIST/D3D12Core.dll",
            "amd_fidelityfx_dx12.dll",
            "dinput8.dll",
            "ScriptHookV.dll",
            "version.dll",
            "steam_api64.dll",
        ])?;
        assert!(
            !mahmiya(&ijmaa),
            "a clean game must stay clean: {:?}",
            ijmaa.adilla
        );
        Ok(())
    }

    #[test]
    fn ramz_tawafuq_al_matjar_yusajjal_himaya_ghayr_musamma() -> NatijatIkhtibar {
        // Steam's own test verdict, in the three sibling maps it writes it to.
        let shajara = shajarat_appinfo(
            r##"
            "appinfo"
            {
                "common"
                {
                    "steam_deck_compatibility"
                    {
                        "tests"
                        {
                            "0"
                            {
                                "display" "2"
                                "token" "#SteamDeckVerified_TestResult_UnsupportedAntiCheatConfiguration"
                            }
                        }
                        "steamos_tests"
                        {
                            "0"
                            {
                                "display" "2"
                                "token" "#SteamOS_TestResult_UnsupportedAntiCheat_Other"
                            }
                        }
                    }
                }
            }
            "##,
        )?;
        let adilla = adillat_appinfo(&shajara, Path::new("appinfo.vdf"));
        assert_eq!(adilla.len(), 2, "{adilla:?}");
        for daleel in &adilla {
            assert_eq!(daleel.naw, NawHimaya::GhayrMusamma);
            assert_eq!(daleel.sinf, NawDaleel::TawqeeMatjar);
            // The store asserts an anti-cheat without naming which, and the
            // grade says exactly that much and no more.
            assert_eq!(daleel.thiqa, Thiqa::Rajiha);
        }
        Ok(())
    }

    #[test]
    fn ramz_tawafuq_akhar_la_yaddai_shayan() -> NatijatIkhtibar {
        // The same subtree carries verdicts about launchers, frame rates and
        // text size. None of them is a claim about an anti-cheat.
        let shajara = shajarat_appinfo(
            r##"
            "appinfo"
            {
                "common"
                {
                    "steam_deck_compatibility"
                    {
                        "tests"
                        {
                            "0"
                            {
                                "display" "1"
                                "token" "#SteamDeckVerified_TestResult_ExternalControllersNotSupported"
                            }
                        }
                    }
                }
            }
            "##,
        )?;
        assert!(adillat_appinfo(&shajara, Path::new("appinfo.vdf")).is_empty());
        Ok(())
    }

    #[test]
    fn tanfidhi_al_itlaq_fi_al_fahras_yusammi_javelin() -> NatijatIkhtibar {
        // What Steam's catalogue holds for a Javelin-protected app: the launch
        // entry starts the anti-cheat, not the game. Read off this machine's
        // own `appinfo.vdf`, where Battlefield 6 spells it this way seventeen
        // times — which answers for a game that is not installed at all.
        let shajara = shajarat_appinfo(
            r#"
            "appinfo"
            {
                "config"
                {
                    "launch"
                    {
                        "0"
                        {
                            "executable" "EAAntiCheat.GameServiceLauncher.exe"
                            "arguments" "-Steam"
                        }
                    }
                }
            }
            "#,
        )?;
        let adilla = adillat_appinfo(&shajara, Path::new("appinfo.vdf"));
        assert_eq!(
            adilla.iter().map(|daleel| daleel.naw).collect::<Vec<_>>(),
            vec![NawHimaya::EaJavelin]
        );
        Ok(())
    }

    #[test]
    fn la_naw_yuraf_bi_khidma_wahdaha() {
        // The structural property the whole design rests on: a sandboxed
        // process table hides running services, so any kind detectable *only*
        // by a service would become undetectable inside a container. Every kind
        // that carries a service marker must also carry one on disk.
        let mut khidmi: BTreeSet<NawHimaya> = BTreeSet::new();
        let mut qursi: BTreeSet<NawHimaya> = BTreeSet::new();
        for alama in ALAMAT {
            match alama.mahal {
                MahalAlama::Khidma => {
                    let _ = khidmi.insert(alama.naw);
                },
                MahalAlama::MalafJuzi | MahalAlama::MalafKamil | MahalAlama::Mushaghghil => {
                    let _ = qursi.insert(alama.naw);
                },
                // An import table is read off a file on disk, but it is the
                // game's file and not the anti-cheat's, so it is not counted
                // as an on-disk marker for this property.
                MahalAlama::Wahda => {},
            }
        }
        let wahidatan: Vec<NawHimaya> = khidmi.difference(&qursi).copied().collect();
        assert!(
            wahidatan.is_empty(),
            "service-only kinds are undetectable in a sandbox: {wahidatan:?}"
        );
    }
}
