//! كشف الحماية — anti-cheat detection by hard evidence: file, module, service, driver and store signature.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use object::read::Object as _;
use serde::{Deserialize, Serialize};
use taarib_kashf::matajir::vdf::{self, QeemaVdf};
use taarib_usus::manassa;

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
    /// Denuvo Anti-Cheat, distinct from the Denuvo anti-tamper DRM.
    Denuvo,
    /// Riot Vanguard.
    Vanguard,
    /// nProtect `GameGuard`.
    GameGuard,
    /// XIGNCODE3.
    Xigncode3,
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
}

impl NawHimaya {
    /// The Arabic display name.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::EasyAntiCheat => "إيزي أنتي-تشيت",
            Self::EasyAntiCheatEos => "إيزي أنتي-تشيت (عبر خدمات إيبك أونلاين)",
            Self::BattlEye => "باتل آي",
            Self::Denuvo => "دينوفو لمكافحة الغش",
            Self::Vanguard => "ريوت فانغارد",
            Self::GameGuard => "جيم غارد من nProtect",
            Self::Xigncode3 => "زين كود ٣",
            Self::PunkBuster => "بانك باستر",
            Self::FaceitAc => "مضاد الغش من فيسإت",
            Self::Esea => "مضاد الغش من ESEA",
            Self::Ricochet => "ريكوشيه من أكتيفجن",
            Self::Vac => "مكافحة الغش من فالف (VAC)",
        }
    }

    /// The English display name.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::EasyAntiCheat => "Easy Anti-Cheat",
            Self::EasyAntiCheatEos => "Easy Anti-Cheat (EOS)",
            Self::BattlEye => "BattlEye",
            Self::Denuvo => "Denuvo Anti-Cheat",
            Self::Vanguard => "Riot Vanguard",
            Self::GameGuard => "nProtect GameGuard",
            Self::Xigncode3 => "XIGNCODE3",
            Self::PunkBuster => "PunkBuster",
            Self::FaceitAc => "FACEIT Anti-Cheat",
            Self::Esea => "ESEA Anti-Cheat",
            Self::Ricochet => "Activision Ricochet",
            Self::Vac => "Valve Anti-Cheat (VAC)",
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
        format!("{}: {} — {}", self.naw.injilizi(), self.sinf.injilizi(), self.ayn)
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
const KALIMAT_MATJAR: [(&str, NawHimaya); 5] = [
    ("start_protected_game", NawHimaya::EasyAntiCheat),
    ("easyanticheat_eos", NawHimaya::EasyAntiCheatEos),
    ("easyanticheat", NawHimaya::EasyAntiCheat),
    ("easy anti-cheat", NawHimaya::EasyAntiCheat),
    ("battleye", NawHimaya::BattlEye),
];

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
/// One pass, not two: `appinfo.vdf` is a few hundred megabytes on a mature
/// account, and a caller that wanted both answers by calling twice would pay
/// for it twice.
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
    let mut musajjil = Musajjil::default();

    imsah_luba(jidhr_luba, &mut musajjil);
    ifhas_khidmat(jidhr_luba, &mut musajjil);

    let halat_matjar = match (masdar_luba_appid, jidhr_steam) {
        (None, _) => HalatMatjar::GhayrMatlub,
        (Some(_), None) => HalatMatjar::JidhrMajhul,
        (Some(appid), Some(jidhr_steam)) => match fahs_matjar(jidhr_steam, appid) {
            Ok(adilla) => {
                for daleel in adilla {
                    musajjil.sajjil(daleel);
                }
                HalatMatjar::Maqru
            }
            Err(khata) => {
                let masar = jidhr_steam.join("appcache").join("appinfo.vdf");
                let sabab = format!("{:?}", khata.kind());
                // Still a gap on the report as well: the surface that only
                // displays this scan must keep showing the same fact the
                // install path now refuses over.
                musajjil.thughra(masar.clone(), sabab.clone());
                HalatMatjar::Mutaadhdhir { masar, sabab }
            }
        },
    };

    let ijmaa = IjmaaHimaya {
        jidhr: jidhr_luba.to_path_buf(),
        adilla: musajjil.adilla,
        thughrat: musajjil.thughrat,
        mabtur: musajjil.mabtur,
    };
    (ijmaa, halat_matjar)
}

/// Reads Steam's `appinfo.vdf` for the store signatures recorded against one app.
///
/// # Errors
///
/// The underlying [`std::io::Error`] when `appcache/appinfo.vdf` cannot be read,
/// and an [`std::io::ErrorKind::Other`] carrying the reader's message when the
/// catalogue's own framing is corrupt.
pub fn fahs_matjar(jidhr_steam: &Path, appid: u32) -> Result<Vec<DaleelHimaya>, io::Error> {
    let masar = jidhr_steam.join("appcache").join("appinfo.vdf");
    let bayt = std::fs::read(&masar)?;

    let mut adilla: Vec<DaleelHimaya> = Vec::new();
    let natija = vdf::murur_appinfo(&masar, &bayt, &mut |madkhal| {
        let Ok(madkhal) = madkhal else {
            return;
        };
        if madkhal.app != appid {
            return;
        }
        adilla.extend(adillat_appinfo(&madkhal.bayanat, &masar));
    });
    natija.map_err(|khata| io::Error::other(khata.injilizi))?;
    Ok(adilla)
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
        self.thughrat.push(ThughraFahs { masar, sabab: sabab.into() });
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
                let masar = khata.path().map_or_else(|| jidhr.to_path_buf(), Path::to_path_buf);
                let sabab = khata
                    .io_error()
                    .map_or_else(|| "walk error".to_owned(), |io| format!("{:?}", io.kind()));
                musajjil.thughra(masar, sabab);
                continue;
            }
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
            }
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
        }
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
        for alama in ALAMAT.iter().filter(|alama| matches!(alama.mahal, MahalAlama::Wahda)) {
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

    for alama in ALAMAT.iter().filter(|alama| matches!(alama.mahal, MahalAlama::Khidma)) {
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
fn adillat_appinfo(bayanat: &QeemaVdf, masar: &Path) -> Vec<DaleelHimaya> {
    let mut adilla: Vec<DaleelHimaya> = Vec::new();

    let fiat = bayanat.kain_bi_masar(&["appinfo", "common", "category"]).unwrap_or(&[]);
    for (miftah, qeema) in fiat {
        // A cleared category is written as zero, not removed, so the value is checked too.
        if qeema.raqm().unwrap_or(1) == 0 {
            continue;
        }
        let raqm = miftah.rsplit('_').next().and_then(|raqm| raqm.parse::<u32>().ok());
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
            let dhayl = qeema.nass().map(|nass| format!(" = {nass}")).unwrap_or_default();
            adilla.push(tawqee(
                NawHimaya::Vac,
                format!("appinfo.vdf: config/{miftah}{dhayl}"),
                masar,
                Thiqa::Muakkada,
            ));
        }
    }

    for (_, madkhal) in bayanat.kain_bi_masar(&["appinfo", "config", "launch"]).unwrap_or(&[]) {
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

    for (_, ittifaq) in bayanat.kain_bi_masar(&["appinfo", "common", "eulas"]).unwrap_or(&[]) {
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

    adilla
}

/// Builds one store-signature detection.
fn tawqee(naw: NawHimaya, ayn: String, masar: &Path, thiqa: Thiqa) -> DaleelHimaya {
    let masar = Some(masar.to_path_buf());
    DaleelHimaya { naw, sinf: NawDaleel::TawqeeMatjar, ayn, masar, thiqa }
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
    masar.strip_prefix(jidhr).unwrap_or(masar).display().to_string()
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
