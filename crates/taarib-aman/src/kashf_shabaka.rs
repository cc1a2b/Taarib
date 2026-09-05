//! كشف الشبكة — online/multiplayer detection by hard evidence: categories, modules, servers.

use std::collections::BTreeSet;
use std::io;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use object::FileKind;
use object::read::Object as _;
use serde::{Deserialize, Serialize};
use taarib_kashf::matajir::vdf::QeemaVdf;

use crate::kashf_himaya::HalatMatjar;
use crate::matjar::QiraatMatjar;

use DalalatShabaka::{KhadimMukhassas, Mmo, MutaaddidMahalli, MutaaddidOnline, ShabakiAam};
use MahalShabaka::{MalafJidhr, MalafJuzi, Wahda};

/// The kind of concrete evidence one finding rests on, as a closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawDaleel {
    /// A Steam store category flag read from `appinfo.vdf`.
    FiaSteam,
    /// A known networking or online-service module in the game's own files.
    WahdaShabaka,
    /// A dedicated-server or matchmaking binary present on disk.
    MalafKhadim,
    /// Store metadata that declares multiplayer where the launcher provides it.
    BayanMatjar,
}

impl NawDaleel {
    /// The Arabic name of this evidence kind.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::FiaSteam => "فئة في متجر ستيم",
            Self::WahdaShabaka => "وحدة شبكات في ملفات اللعبة",
            Self::MalafKhadim => "ملف خادم أو مطابقة لاعبين",
            Self::BayanMatjar => "بيان من المتجر",
        }
    }

    /// The English name of this evidence kind.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::FiaSteam => "a Steam store category",
            Self::WahdaShabaka => "a networking module in the game files",
            Self::MalafKhadim => "a dedicated-server or matchmaking binary",
            Self::BayanMatjar => "store metadata",
        }
    }
}

/// What one finding indicates about how the game is played, as a closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DalalatShabaka {
    /// Played with other people over the internet.
    MutaaddidOnline,
    /// Played with other people on one machine, shared or split screen.
    MutaaddidMahalli,
    /// Networked play, over a LAN or otherwise, without a narrower claim.
    ShabakiAam,
    /// A massively multiplayer online title.
    Mmo,
    /// Ships or declares a dedicated server.
    KhadimMukhassas,
}

impl DalalatShabaka {
    /// Whether this indicates play over a network rather than on one machine.
    #[must_use]
    pub const fn shabaki(self) -> bool {
        !matches!(self, Self::MutaaddidMahalli)
    }

    /// The Arabic name of this indication.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::MutaaddidOnline => "لعب متعدّد عبر الإنترنت",
            Self::MutaaddidMahalli => "لعب متعدّد على جهاز واحد",
            Self::ShabakiAam => "لعب عبر الشبكة",
            Self::Mmo => "لعبة جماعية ضخمة عبر الإنترنت",
            Self::KhadimMukhassas => "خادم مُخصَّص",
        }
    }

    /// The English name of this indication.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::MutaaddidOnline => "online multiplayer",
            Self::MutaaddidMahalli => "local (same-machine) multiplayer",
            Self::ShabakiAam => "networked play",
            Self::Mmo => "massively multiplayer online",
            Self::KhadimMukhassas => "dedicated server",
        }
    }
}

/// One piece of hard evidence that a game is online or multiplayer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaleelShabaka {
    /// The kind of evidence this rests on.
    pub naw: NawDaleel,
    /// What it indicates about how the game is played.
    pub dalala: DalalatShabaka,
    /// Where it was found: a relative path with the marker, or a catalogue key.
    pub ayn: String,
    /// The absolute path behind the evidence, when there is one.
    pub masar: Option<PathBuf>,
}

impl DaleelShabaka {
    /// The evidence named in one Arabic line.
    #[must_use]
    pub fn arabi(&self) -> String {
        format!("{} — {}: {}", self.dalala.arabi(), self.naw.arabi(), self.ayn)
    }

    /// The evidence named in one English line.
    #[must_use]
    pub fn injilizi(&self) -> String {
        format!("{} — {}: {}", self.dalala.injilizi(), self.naw.injilizi(), self.ayn)
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
pub struct IjmaaShabaka {
    /// The game root that was scanned.
    pub jidhr: PathBuf,
    /// Every distinct piece of evidence, in the order it was found.
    pub dalail: Vec<DaleelShabaka>,
    /// Places the scan could not read, so a thin report is told from a clean one.
    pub thughrat: Vec<ThughraFahs>,
    /// The filesystem walk stopped at a bound before it finished.
    pub mabtur: bool,
}

impl IjmaaShabaka {
    /// The distinct indications the evidence carries, in a stable order.
    #[must_use]
    pub fn anwa_dalala(&self) -> Vec<DalalatShabaka> {
        let mut ruit: BTreeSet<DalalatShabaka> = BTreeSet::new();
        for daleel in &self.dalail {
            let _ = ruit.insert(daleel.dalala);
        }
        ruit.into_iter().collect()
    }

    /// Whether any evidence indicates play over a network rather than only on
    /// one machine.
    #[must_use]
    pub fn online(&self) -> bool {
        self.dalail.iter().any(|daleel| daleel.dalala.shabaki())
    }

    /// Exactly what was found, in Arabic, for the acknowledgement dialog.
    #[must_use]
    pub fn wasf_iqrar(&self) -> String {
        if self.dalail.is_empty() {
            return "لم يُعثر على أيّ دليل ملموس على اللعب متعدّد اللاعبين.".to_owned();
        }
        let mut sutur: Vec<String> =
            vec!["عُثر على أدلّة ملموسة على أنّ هذه اللعبة تُلعب مع لاعبين آخرين:".to_owned()];
        for daleel in &self.dalail {
            sutur.push(format!("• {}", daleel.arabi()));
        }
        if !self.thughrat.is_empty() {
            sutur.push("تعذّرت قراءة بعض المواضع أثناء الفحص، فقد يوجد ما لم يُكتشف.".to_owned());
        }
        sutur.join("\n")
    }

    /// The same, in English, for logs and diagnostics.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        if self.dalail.is_empty() {
            return "No concrete evidence of multiplayer was found.".to_owned();
        }
        let mut sutur: Vec<String> = vec!["Concrete evidence this game is multiplayer:".to_owned()];
        for daleel in &self.dalail {
            sutur.push(format!("- {}", daleel.injilizi()));
        }
        sutur.join("\n")
    }
}

/// Whether any evidence at all was found — the one question the warning asks.
#[must_use]
pub const fn mutaaddid(ijmaa: &IjmaaShabaka) -> bool {
    !ijmaa.dalail.is_empty()
}

/// Deepest level the walk descends; a filesystem root is refused, not walked whole.
const AQSA_UMQ: usize = 8;

/// Entry ceiling for one walk; past it the scan stops and marks the report partial.
const AQSA_MADAKHIL: usize = 200_000;

/// How many executables are opened for import inspection, bounding the reads.
const AQSA_TANFIDHIYAT: usize = 8;

/// How many candidate server binaries have their header read, bounding the reads.
const AQSA_KHADIM: usize = 16;

/// Largest executable read whole for imports; a larger one is left to disk markers.
const HADD_QIRA: u64 = 64 * 1024 * 1024;

/// Header prefix read to confirm a candidate server binary is a real object image.
const HADD_TARWISA: u64 = 8 * 1024;

/// Cap on recorded evidence, so one folder cannot produce an unbounded report.
const AQSA_ADILLA: usize = 256;

/// File extensions whose import table is worth reading.
const IMTIDADAT_MUSTAWRAD: [&str; 4] = ["exe", "dll", "so", "dylib"];

/// Steam store category numbers that mean the game is played with other people,
/// with what each one indicates. VAC (8) is anti-cheat and belongs to
/// `kashf_himaya`; in-app purchases (35) are not multiplayer and are excluded.
const FIAT: &[FiaMaerufa] = &[
    fia(1, ShabakiAam, "Multi-player"),
    fia(9, ShabakiAam, "Co-op"),
    fia(20, Mmo, "MMO"),
    fia(24, MutaaddidMahalli, "Shared/Split Screen"),
    fia(27, MutaaddidOnline, "Cross-Platform Multiplayer"),
    fia(36, MutaaddidOnline, "Online PvP"),
    fia(37, MutaaddidMahalli, "Shared/Split Screen PvP"),
    fia(38, MutaaddidOnline, "Online Co-op"),
    fia(39, MutaaddidMahalli, "Shared/Split Screen Co-op"),
    fia(44, MutaaddidMahalli, "Remote Play Together"),
    fia(47, ShabakiAam, "LAN PvP"),
    fia(48, ShabakiAam, "LAN Co-op"),
];

/// Every networking and online-service marker this build recognises. Generic
/// Steamworks wrappers (`steam_api`, `Steamworks.NET`) are deliberately absent:
/// single-player games ship them for achievements, so they are not evidence.
const ALAMAT_SHABAKA: &[AlamaShabaka] = &[
    wsm("gamenetworkingsockets", MalafJuzi, MutaaddidOnline, "GameNetworkingSockets"),
    wsm("gamenetworkingsockets", Wahda, MutaaddidOnline, "GameNetworkingSockets"),
    wsm("steamnetworkingsockets", MalafJuzi, MutaaddidOnline, "Steam networking sockets"),
    wsm("steamnetworkingsockets", Wahda, MutaaddidOnline, "Steam networking sockets"),
    wsm("eossdk", MalafJuzi, MutaaddidOnline, "Epic Online Services"),
    wsm("eossdk", Wahda, MutaaddidOnline, "Epic Online Services"),
    wsm("photon", MalafJuzi, MutaaddidOnline, "Photon"),
    wsm("photonrealtime", Wahda, MutaaddidOnline, "Photon Realtime"),
    wsm("mirror", MalafJidhr, ShabakiAam, "Mirror"),
    wsm("mirror", Wahda, ShabakiAam, "Mirror"),
    wsm("mirage", MalafJidhr, ShabakiAam, "Mirage"),
    wsm("unity.netcode", MalafJuzi, ShabakiAam, "Unity Netcode for GameObjects"),
    wsm("unity.netcode", Wahda, ShabakiAam, "Unity Netcode for GameObjects"),
    wsm("com.unity.transport", MalafJuzi, ShabakiAam, "Unity Transport"),
    wsm("com.unity.netcode", MalafJuzi, ShabakiAam, "Unity Netcode package"),
    wsm("com.unity.multiplayer", MalafJuzi, ShabakiAam, "Unity Multiplayer"),
    wsm("unity.networking", MalafJuzi, ShabakiAam, "Unity UNet (HLAPI)"),
    wsm("nakama", MalafJuzi, MutaaddidOnline, "Nakama"),
    wsm("nakama", Wahda, MutaaddidOnline, "Nakama"),
    wsm("playfab", MalafJuzi, MutaaddidOnline, "PlayFab"),
    wsm("playfab", Wahda, MutaaddidOnline, "PlayFab"),
    wsm("raknet", MalafJuzi, ShabakiAam, "RakNet"),
    wsm("raknet", Wahda, ShabakiAam, "RakNet"),
    wsm("enet", MalafJidhr, ShabakiAam, "ENet"),
    wsm("libenet", MalafJidhr, ShabakiAam, "ENet"),
    wsm("litenetlib", MalafJuzi, ShabakiAam, "LiteNetLib"),
    wsm("lidgren", MalafJuzi, ShabakiAam, "Lidgren.Network"),
    wsm("darkrift", MalafJuzi, ShabakiAam, "DarkRift"),
    wsm("fishnet", MalafJuzi, ShabakiAam, "FishNet"),
    wsm("riptide", MalafJuzi, ShabakiAam, "Riptide"),
];

/// Dedicated-server and matchmaking binary markers, matched on an executable
/// name and confirmed against the file's own object header. A bare `…Server.exe`
/// is not here: without another signal it is as often a non-multiplayer helper.
const KHUYUT_KHADIM: &[AlamaKhadim] = &[
    khd("dedicatedserver", KhadimMukhassas, "dedicated server"),
    khd("dedicated_server", KhadimMukhassas, "dedicated server"),
    khd("dedicated-server", KhadimMukhassas, "dedicated server"),
    khd("srcds", KhadimMukhassas, "Source dedicated server"),
    khd("gameserver", KhadimMukhassas, "game server"),
    khd("matchmaking", MutaaddidOnline, "matchmaking client"),
];

/// One Steam store category worth reporting.
#[derive(Debug, Clone, Copy)]
struct FiaMaerufa {
    raqm: u32,
    dalala: DalalatShabaka,
    injilizi: &'static str,
}

/// Builds a category-table row.
const fn fia(raqm: u32, dalala: DalalatShabaka, injilizi: &'static str) -> FiaMaerufa {
    FiaMaerufa { raqm, dalala, injilizi }
}

/// Where a networking marker is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MahalShabaka {
    /// A file or directory name that contains the marker.
    MalafJuzi,
    /// A file whose lowercased stem equals the marker.
    MalafJidhr,
    /// A module named in a game binary's import table.
    Wahda,
}

/// One networking marker: a needle, where it is looked for, and what it means.
#[derive(Debug, Clone, Copy)]
struct AlamaShabaka {
    ibra: &'static str,
    mahal: MahalShabaka,
    dalala: DalalatShabaka,
    wasf: &'static str,
}

/// Builds a networking-marker row.
const fn wsm(
    ibra: &'static str,
    mahal: MahalShabaka,
    dalala: DalalatShabaka,
    wasf: &'static str,
) -> AlamaShabaka {
    AlamaShabaka { ibra, mahal, dalala, wasf }
}

/// One server-binary marker: a needle, what it means, and how it is named.
#[derive(Debug, Clone, Copy)]
struct AlamaKhadim {
    ibra: &'static str,
    dalala: DalalatShabaka,
    wasf: &'static str,
}

/// Builds a server-binary-marker row.
const fn khd(ibra: &'static str, dalala: DalalatShabaka, wasf: &'static str) -> AlamaKhadim {
    AlamaKhadim { ibra, dalala, wasf }
}

/// Scans one game for online and multiplayer evidence.
///
/// Steam's catalogue is read too when both `appid` and `jidhr_steam` are given.
///
/// Never fails: a place it cannot read becomes a [`ThughraFahs`] in the report,
/// and the evidence is a value.
#[must_use]
pub fn ifhas_shabaka(
    jidhr_luba: &Path,
    appid: Option<u32>,
    jidhr_steam: Option<&Path>,
) -> IjmaaShabaka {
    ifhas_shabaka_bi_qiraa(jidhr_luba, &QiraatMatjar::iqra(appid, jidhr_steam))
}

/// The same scan against a catalogue reading the caller already has.
///
/// [`crate::fahs::fahs`] runs this scan *and* [`crate::kashf_himaya`]'s on the
/// same game before it will authorise anything, and both want the same
/// `appinfo.vdf`. This entry point is how that file gets read once instead of
/// twice; see [`crate::matjar`] for what the second read cost.
#[must_use]
pub fn ifhas_shabaka_bi_qiraa(jidhr_luba: &Path, matjar: &QiraatMatjar) -> IjmaaShabaka {
    let mut musajjil = Musajjil::default();

    imsah_luba(jidhr_luba, &mut musajjil);

    for daleel in &matjar.shabaka {
        musajjil.sajjil(daleel.clone());
    }
    // A catalogue that could not be opened is a gap on this report too, worded
    // the way every other gap in this file is: the short failure label beside
    // the path it happened to.
    if let HalatMatjar::Mutaadhdhir { masar, sabab } = &matjar.hala {
        musajjil.thughra(masar.clone(), sabab.clone());
    }

    IjmaaShabaka {
        jidhr: jidhr_luba.to_path_buf(),
        dalail: musajjil.dalail,
        thughrat: musajjil.thughrat,
        mabtur: musajjil.mabtur,
    }
}

/// Accumulates evidence, dropping duplicates, capping the total, and recording gaps.
#[derive(Debug, Default)]
struct Musajjil {
    dalail: Vec<DaleelShabaka>,
    ruyat: BTreeSet<String>,
    thughrat: Vec<ThughraFahs>,
    mabtur: bool,
}

impl Musajjil {
    /// Records one detection, unless it is a duplicate or the cap is reached.
    fn sajjil(&mut self, daleel: DaleelShabaka) {
        if self.dalail.len() >= AQSA_ADILLA {
            self.mabtur = true;
            return;
        }
        let ayn = daleel.ayn.to_ascii_lowercase();
        let miftah = format!("{:?}|{:?}|{ayn}", daleel.naw, daleel.dalala);
        if self.ruyat.insert(miftah) {
            self.dalail.push(daleel);
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
    /// The lowercased stem, for the exact-stem test.
    jidhr_ism: &'a str,
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
    let mut fuhus_khadim: usize = 0;
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
        // A symlink is never followed to decide presence; its target may leave the game root.
        if naw.is_symlink() {
            continue;
        }

        let ism = madkhal.file_name().to_string_lossy().to_ascii_lowercase();
        let masar = madkhal.path();
        let (jidhr_ism, imtidad) = qism_ism(&ism);
        let mafhus = MadkhalMafhus {
            ism: &ism,
            jidhr_ism: &jidhr_ism,
            mujallad: naw.is_dir(),
            masar,
            nisbi: nisbi(jidhr, masar),
        };
        fahs_madkhal(&mafhus, musajjil);

        if naw.is_file() {
            fahs_khadim(&mafhus, &mut fuhus_khadim, musajjil);
            if IMTIDADAT_MUSTAWRAD.contains(&imtidad.as_str())
                && tanfidhiyat.len() < AQSA_TANFIDHIYAT
                && madkhal.metadata().map_or(u64::MAX, |bayanat| bayanat.len()) <= HADD_QIRA
            {
                tanfidhiyat.push(masar.to_path_buf());
            }
        }
    }

    for masar in tanfidhiyat {
        ifhas_mustawradat(jidhr, &masar, musajjil);
    }
}

/// Tests one directory entry against every file-name networking marker.
fn fahs_madkhal(mafhus: &MadkhalMafhus<'_>, musajjil: &mut Musajjil) {
    for alama in ALAMAT_SHABAKA {
        let mutabiq = match alama.mahal {
            MalafJuzi => mafhus.ism.contains(alama.ibra),
            MalafJidhr => !mafhus.mujallad && mafhus.jidhr_ism == alama.ibra,
            Wahda => false,
        };
        if mutabiq {
            musajjil.sajjil(DaleelShabaka {
                naw: NawDaleel::WahdaShabaka,
                dalala: alama.dalala,
                ayn: format!("{}: {}", alama.wasf, mafhus.nisbi),
                masar: Some(mafhus.masar.to_path_buf()),
            });
        }
    }
}

/// Records a dedicated-server or matchmaking binary, confirmed by its own header.
fn fahs_khadim(mafhus: &MadkhalMafhus<'_>, adad: &mut usize, musajjil: &mut Musajjil) {
    let Some(alama) = KHUYUT_KHADIM.iter().find(|alama| mafhus.ism.contains(alama.ibra)) else {
        return;
    };
    if *adad >= AQSA_KHADIM {
        musajjil.mabtur = true;
        return;
    }
    *adad = adad.saturating_add(1);
    // A name is not enough: a `.txt` or config file that happens to say
    // `dedicatedserver` is not a server. Only a real object image counts.
    if !tanfidhi_haqiqi(mafhus.masar) {
        return;
    }
    musajjil.sajjil(DaleelShabaka {
        naw: NawDaleel::MalafKhadim,
        dalala: alama.dalala,
        ayn: format!("{}: {}", alama.wasf, mafhus.nisbi),
        masar: Some(mafhus.masar.to_path_buf()),
    });
}

/// Reads one binary's import table and records the networking modules it loads.
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
        for alama in ALAMAT_SHABAKA.iter().filter(|alama| matches!(alama.mahal, Wahda)) {
            if maktaba.contains(alama.ibra) {
                musajjil.sajjil(DaleelShabaka {
                    naw: NawDaleel::WahdaShabaka,
                    dalala: alama.dalala,
                    ayn: format!("{}: {ism} → {maktaba}", alama.wasf),
                    masar: Some(masar.to_path_buf()),
                });
            }
        }
    }
}

/// Projects one app's `appinfo.vdf` tree into the multiplayer evidence it carries.
pub(crate) fn dalail_appinfo(bayanat: &QeemaVdf, masar: &Path) -> Vec<DaleelShabaka> {
    let mut dalail: Vec<DaleelShabaka> = Vec::new();

    let fiat = bayanat.kain_bi_masar(&["appinfo", "common", "category"]).unwrap_or(&[]);
    for (miftah, qeema) in fiat {
        // A cleared category is written as zero, not removed, so the value is checked too.
        if qeema.raqm().unwrap_or(1) == 0 {
            continue;
        }
        let Some(raqm) = miftah.rsplit('_').next().and_then(|raqm| raqm.parse::<u32>().ok()) else {
            continue;
        };
        if let Some(fia) = FIAT.iter().find(|fia| fia.raqm == raqm) {
            dalail.push(DaleelShabaka {
                naw: NawDaleel::FiaSteam,
                dalala: fia.dalala,
                ayn: format!("appinfo.vdf: common/category/{miftah} ({})", fia.injilizi),
                masar: Some(masar.to_path_buf()),
            });
        }
    }

    if let Some(mujallad) = bayanat
        .nass_bi_masar(&["appinfo", "extended", "dedicatedserverfolder"])
        .filter(|mujallad| !mujallad.is_empty())
    {
        dalail.push(DaleelShabaka {
            naw: NawDaleel::BayanMatjar,
            dalala: KhadimMukhassas,
            ayn: format!("appinfo.vdf: extended/dedicatedserverfolder = {mujallad}"),
            masar: Some(masar.to_path_buf()),
        });
    }

    let tashghil = bayanat.kain_bi_masar(&["appinfo", "config", "launch"]).unwrap_or(&[]);
    for (miftah, madkhal) in tashghil {
        let wasf = madkhal.nass_bi_masar(&["description"]).unwrap_or("").to_ascii_lowercase();
        let tanfidhi = madkhal.nass_bi_masar(&["executable"]).unwrap_or("").to_ascii_lowercase();
        // `dedicated` is unambiguous; a bare `server` also matches `observer`, so it is not used.
        if wasf.contains("dedicated") || tanfidhi.contains("dedicated") {
            dalail.push(DaleelShabaka {
                naw: NawDaleel::BayanMatjar,
                dalala: KhadimMukhassas,
                ayn: format!("appinfo.vdf: config/launch/{miftah} declares a dedicated server"),
                masar: Some(masar.to_path_buf()),
            });
        }
    }

    dalail
}

/// Whether the file's own header identifies it as an executable object image.
fn tanfidhi_haqiqi(masar: &Path) -> bool {
    let Ok(tarwisa) = iqra_tarwisa(masar, HADD_TARWISA) else {
        return false;
    };
    matches!(
        FileKind::parse(tarwisa.as_slice()),
        Ok(FileKind::Pe32
            | FileKind::Pe64
            | FileKind::Elf32
            | FileKind::Elf64
            | FileKind::MachO32
            | FileKind::MachO64
            | FileKind::MachOFat32
            | FileKind::MachOFat64)
    )
}

/// Reads at most `hadd` bytes from the front of a file.
fn iqra_tarwisa(masar: &Path, hadd: u64) -> io::Result<Vec<u8>> {
    let malaf = std::fs::File::open(masar)?;
    let mut makhzan = Vec::new();
    malaf.take(hadd).read_to_end(&mut makhzan)?;
    Ok(makhzan)
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
