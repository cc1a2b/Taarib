//! ستيم — the primary source, and the one everything else is calibrated against.
//!
//! Steam is the launcher most users have, the launcher with the most games per
//! user, and the launcher whose metadata is richest, so this adapter is the
//! deepest of the ten. It reads seven files, in this order and for these
//! reasons:
//!
//! | file | what it answers |
//! | --- | --- |
//! | the registry, or a handful of well-known directories | where Steam itself is |
//! | `steamapps/libraryfolders.vdf` | every library on every drive, in both the shape Steam wrote before 2021 and the one it writes now |
//! | `steamapps/appmanifest_*.acf` | one per installed game: id, name, install directory, build, size, timestamps, and the state flags that decide whether the files on disk are finished |
//! | `appcache/appinfo.vdf` | the store's own metadata: what kind of thing each app is, which categories it carries, whether it is VAC-secured, and which executable it launches |
//! | `config/config.vdf` | the compatibility tool mapped to each game, and the global default |
//! | `userdata/<id>/config/localconfig.vdf` | the launch options the user set, so Phase 15 extends them instead of erasing them |
//! | `userdata/<id>/config/shortcuts.vdf` | the non-Steam games added to the library, which on a Steam Deck is nearly everything else |
//!
//! ## `appinfo.vdf` is read once per scan
//!
//! It is a few hundred megabytes on a mature account and holds a quarter of a
//! million apps, of which a user owns a few hundred. Reading it per game would
//! turn a scan into minutes of I/O; reading it once and projecting each entry
//! into a small record before the next is parsed keeps the working set to a few
//! megabytes. [`vdf::murur_appinfo`] exists for exactly this.
//!
//! ## Non-Steam shortcuts are not Steam apps
//!
//! `shortcuts.vdf` entries carry an application identifier, but it is a CRC-32
//! of the executable path and the display name, generated on the machine that
//! added it. It indexes nothing in Steam's catalogue: there is no store page,
//! no depot, no build id, and the same game added on two machines gets two
//! different numbers. Publishing or looking up a patch against `steam:<that>`
//! would be publishing against a number that means one thing on one computer.
//!
//! So a shortcut is reported as [`MasdarLuba::Yadawi`] — the source that means
//! "the user pointed at an executable" — with the shortcut's identifier kept
//! inside it, because that identifier is still what Steam itself keys the
//! prefix, the artwork and the launch options by, and this adapter needs all
//! three. Everything Steam-specific about a shortcut is resolved here and
//! handed downstream as ordinary paths, so nothing after discovery has to know
//! a shortcut was involved. When the same game is also found by the launcher
//! that actually owns it, the merge stage upstream reunites them under one
//! identity, which is the outcome that matters.
//!
//! ## Read-only, always
//!
//! Nothing here writes. Not a cache, not a marker file, not a corrected
//! manifest. Steam is running while Taarib scans in the ordinary case, and a
//! discovery pass that wrote into a live client's configuration would be a
//! scanner nobody could leave enabled.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;
use crate::matajir::vdf::{self, QeemaVdf};

/// The identifier this adapter reports, matching the registry's shard family.
const MUARRIF: &str = "steam";

/// Steam's public artwork endpoint.
///
/// One constant because it is the only address in this module, and because
/// when Valve moves it there must be exactly one line to change. The four
/// asset names under it — `library_600x900.jpg`, `library_hero.jpg`,
/// `logo.png`, `header.jpg` — have been stable for years and are what the
/// client itself caches locally.
const RABT_SUWAR: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";

/// The base of a 64-bit Steam identity, subtracted to get the account number
/// that names a directory under `userdata/`.
const ASAS_HUWIYA: u64 = 76_561_197_960_265_728;

/// Steam's `EAppState` bits, as `appmanifest_*.acf` writes them.
///
/// These decide whether a game may be offered as patchable, which is the single
/// most consequential thing this adapter reads. Offering to patch a game that
/// is sixty percent downloaded is offering to overwrite files Steam is about to
/// overwrite itself, and the user would be left with a broken install and a
/// patch record that claims to be applied.
pub mod hala {
    /// Bit 0 — the app is not installed. A manifest can outlive an
    /// uninstall for a short while, and this is how it says so.
    pub const GHAYR_MUTHABBAT: u32 = 1 << 0;
    /// Bit 1 — an update is pending. The files on disk are the old build.
    pub const YALZAM_TAHDITH: u32 = 1 << 1;
    /// Bit 2 — **every file of every installed depot is present and
    /// verified.** This is the bit that means "finished", and nothing is
    /// offered as patchable without it.
    pub const MUTHABBAT_BALKAMIL: u32 = 1 << 2;
    /// Bit 3 — the content on disk is encrypted, which is what a pre-load of
    /// an unreleased game looks like. The bytes are ciphertext until the
    /// unlock, so there is nothing there to patch.
    pub const MUSHAFFAR: u32 = 1 << 3;
    /// Bit 4 — the client has locked the app. Not a statement about the files.
    pub const MAQFUL: u32 = 1 << 4;
    /// Bit 5 — files the manifest lists are not on disk.
    pub const MALAFAT_MAFQUDA: u32 = 1 << 5;
    /// Bit 6 — the game is running. Not a statement about the files either:
    /// refusing to patch a running game is Phase 15's check, not this one.
    pub const QAYD_TASHGHIL: u32 = 1 << 6;
    /// Bit 7 — files failed validation.
    pub const MALAFAT_TALIFA: u32 = 1 << 7;
    /// Bit 8 — an update is being applied right now.
    pub const TAHDITH_JARIN: u32 = 1 << 8;
    /// Bit 9 — an update is part-written and paused.
    pub const TAHDITH_MUAWWAQ: u32 = 1 << 9;
    /// Bit 10 — an update has begun.
    pub const TAHDITH_BADA: u32 = 1 << 10;
    /// Bit 11 — the app is being removed.
    pub const QAYD_IZALA: u32 = 1 << 11;
    /// Bit 12 — a backup is being taken. The files are being read, not
    /// written.
    pub const NUSKHA_JARIYA: u32 = 1 << 12;
    /// Bit 16 — the install is being reconfigured, which moves files.
    pub const IADAT_TAHYIA: u32 = 1 << 16;
    /// Bit 17 — a validation pass is running and may rewrite files.
    pub const TAHAQQUQ: u32 = 1 << 17;
    /// Bit 18 — files are being added.
    pub const IDAFAT_MALAFAT: u32 = 1 << 18;
    /// Bit 19 — space is being reserved for a download.
    pub const HAJZ_MASAHA: u32 = 1 << 19;
    /// Bit 20 — content is downloading.
    pub const TANZIL: u32 = 1 << 20;
    /// Bit 21 — downloaded content is being staged.
    pub const TAJHIZ: u32 = 1 << 21;
    /// Bit 22 — staged content is being committed into place.
    pub const ITHBAT: u32 = 1 << 22;
    /// Bit 23 — an update is being stopped, which unwinds partial writes.
    pub const IQAF_TAHDITH: u32 = 1 << 23;

    /// Every state in which the files on disk are about to change, are already
    /// wrong, or are not readable content at all.
    ///
    /// Deliberately excluded: [`MAQFUL`], [`QAYD_TASHGHIL`] and
    /// [`NUSKHA_JARIYA`], none of which says anything about whether the bytes
    /// on disk are the finished build.
    pub const QINA_TAGHYEER: u32 = YALZAM_TAHDITH
        | MUSHAFFAR
        | MALAFAT_MAFQUDA
        | MALAFAT_TALIFA
        | TAHDITH_JARIN
        | TAHDITH_MUAWWAQ
        | TAHDITH_BADA
        | QAYD_IZALA
        | IADAT_TAHYIA
        | TAHAQQUQ
        | IDAFAT_MALAFAT
        | HAJZ_MASAHA
        | TANZIL
        | TAJHIZ
        | ITHBAT
        | IQAF_TAHDITH;
}

/// Whether a game's files are finished and will not be rewritten under Taarib.
///
/// Two conditions, both required: [`hala::MUTHABBAT_BALKAMIL`] is set, and no
/// bit of [`hala::QINA_TAGHYEER`] is. The first alone is not enough — a fully
/// installed game with an update downloading carries both bit 2 and bit 20, and
/// patching it would put Taarib's files under an install Steam is in the middle
/// of replacing.
#[must_use]
pub const fn muktamila(alam: u32) -> bool {
    alam & hala::MUTHABBAT_BALKAMIL != 0 && alam & hala::QINA_TAGHYEER == 0
}

/// The Steam adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarSteam;

// ---------------------------------------------------------------------------
// Where Steam is
// ---------------------------------------------------------------------------

/// Whether a directory looks like a Steam root rather than any other folder.
///
/// A root always has either a `steamapps` directory — spelled with a capital A
/// on installations that predate the 2013 rename and were never touched since —
/// or a `config` directory. Requiring one of them keeps a user who points the
/// setting at their home folder from getting a scan of their entire disk.
fn jidhr_salih(masar: &Path) -> bool {
    steamapps(masar).is_some() || masar.join("config").is_dir()
}

/// The `steamapps` directory of a library or a root, in whichever case this
/// filesystem has it.
fn steamapps(jidhr: &Path) -> Option<PathBuf> {
    let saghir = jidhr.join("steamapps");
    if saghir.is_dir() {
        return Some(saghir);
    }
    let kabir = jidhr.join("SteamApps");
    if kabir.is_dir() { Some(kabir) } else { None }
}

/// Every place Steam is known to install itself on this platform, in the order
/// they are tried.
fn murashahat(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
    let mut murashahat = Vec::new();
    match siyaq.nizam {
        NizamTashghil::Windows => {
            murashahat.extend(murashahat_sijill());
            // From the context, never a literal `C:`. This branch is reached
            // only when the three registry values are missing or corrupt, and
            // the machine on which that coincides with a Windows installed off
            // `C:` is exactly the machine a hardcoded drive letter would send
            // to a folder that does not exist.
            murashahat
                .extend(siyaq.mujalladat_baramij.iter().map(|mujallad| mujallad.join("Steam")));
        }
        NizamTashghil::Linux => {
            murashahat.push(siyaq.manzil.join(".steam").join("steam"));
            murashahat.push(siyaq.manzil.join(".local").join("share").join("Steam"));
            murashahat.push(siyaq.manzil.join(".steam").join("root"));
            murashahat.push(siyaq.manzil.join(".steam").join("debian-installation"));
            if siyaq.yashmal_hawiyat {
                murashahat.push(
                    siyaq
                        .manzil
                        .join(".var")
                        .join("app")
                        .join("com.valvesoftware.Steam")
                        .join("data")
                        .join("Steam"),
                );
                murashahat.push(
                    siyaq
                        .manzil
                        .join("snap")
                        .join("steam")
                        .join("common")
                        .join(".local")
                        .join("share")
                        .join("Steam"),
                );
            }
        }
        NizamTashghil::Mac => {
            murashahat.push(
                siyaq.manzil.join("Library").join("Application Support").join("Steam"),
            );
        }
    }
    murashahat
}

/// The two registry values Steam writes on Windows.
///
/// `HKCU` first: it is written by the client that is actually in use, survives
/// a per-user install, and is correct on a machine where two accounts installed
/// Steam in two places. `HKLM\WOW6432Node` is the machine-wide fallback, and
/// `HKLM` without the redirection node covers an ARM64 host where the 32-bit
/// view is not where the value landed.
#[cfg(windows)]
fn murashahat_sijill() -> Vec<PathBuf> {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    [
        (HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"),
    ]
    .into_iter()
    .filter_map(|(jidhr, miftah, qeema)| qeemat_sijill(jidhr, miftah, qeema))
    .collect()
}

/// On anything but Windows there is no registry, and the directory candidates
/// carry the whole answer.
#[cfg(not(windows))]
const fn murashahat_sijill() -> Vec<PathBuf> {
    Vec::new()
}

/// Reads one `REG_SZ` value, or `None` for anything at all going wrong.
///
/// Two calls: the first asks how many bytes the value needs, the second reads
/// it. `RegGetValueW` opens, reads and closes the key itself, so there is no
/// handle to leak on an early return.
#[cfg(windows)]
fn qeemat_sijill(
    jidhr: windows::Win32::System::Registry::HKEY,
    miftah: &str,
    qeema: &str,
) -> Option<PathBuf> {
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::HSTRING;

    let miftah_w = HSTRING::from(miftah);
    let qeema_w = HSTRING::from(qeema);

    let mut hajm: u32 = 0;
    // SAFETY: both strings are live, null-terminated and wide for the duration
    // of the call, and `hajm` is a live out-parameter of the required width.
    // Passing no data pointer is the documented way to ask for the size only.
    let hala = unsafe {
        RegGetValueW(jidhr, &miftah_w, &qeema_w, RRF_RT_REG_SZ, None, None, Some(&raw mut hajm))
    };
    if hala != ERROR_SUCCESS || hajm == 0 {
        return None;
    }

    let adad = usize::try_from(hajm).ok()?.div_ceil(2);
    let mut mihfaza = vec![0u16; adad];
    let mut hajm_thani = hajm;
    // SAFETY: `mihfaza` holds at least `hajm` bytes, which is what
    // `hajm_thani` declares, and both remain live for the duration of the call.
    let hala = unsafe {
        RegGetValueW(
            jidhr,
            &miftah_w,
            &qeema_w,
            RRF_RT_REG_SZ,
            None,
            Some(mihfaza.as_mut_ptr().cast()),
            Some(&raw mut hajm_thani),
        )
    };
    if hala != ERROR_SUCCESS {
        return None;
    }

    let tul = mihfaza.iter().position(|wahda| *wahda == 0).unwrap_or(mihfaza.len());
    let nass = String::from_utf16(mihfaza.get(..tul)?).ok()?;
    if nass.is_empty() { None } else { Some(PathBuf::from(nass)) }
}

/// Resolves every Steam root on this machine, distinguishing "not installed"
/// from "the user told Taarib where it is and it is not there".
///
/// The distinction is the whole point of [`KhataKashf::JidhrMuhaddadMafqud`]:
/// falling back to automatic detection after an override fails would hide the
/// user's typo behind a library that is merely incomplete, and they would have
/// no way to find out why.
///
/// Every valid candidate is returned rather than the first. A Linux machine
/// routinely carries a native Steam and a Flatpak or Snap one at the same time,
/// each with its own `libraryfolders.vdf`, its own signed-in accounts and its
/// own compatibility mapping; taking the first would make the other's whole
/// library invisible with nothing said about it. `~/.steam/steam` is a symlink
/// to `~/.local/share/Steam` on most installations, so the list is
/// deduplicated through [`sawwi`] and that pair collapses to the one root it
/// really is.
fn hall_judhur(siyaq: &SiyaqFahs) -> Natija<Vec<PathBuf>> {
    if let Some(tajawuz) = siyaq.manassat.steam.as_ref() {
        if jidhr_salih(tajawuz) {
            return Ok(vec![tajawuz.clone()]);
        }
        return Err(Khata::min_tafsir(&KhataKashf::JidhrMuhaddadMafqud {
            matjar: MUARRIF,
            masar: tajawuz.clone(),
        }));
    }

    let mut judhur: Vec<PathBuf> = Vec::new();
    let mut maruf: Vec<PathBuf> = Vec::new();
    for murashah in murashahat(siyaq) {
        if !jidhr_salih(&murashah) {
            continue;
        }
        let muwahhad = sawwi(&murashah);
        if maruf.contains(&muwahhad) {
            continue;
        }
        maruf.push(muwahhad);
        judhur.push(murashah);
    }
    Ok(judhur)
}

// ---------------------------------------------------------------------------
// Reading Steam's files
// ---------------------------------------------------------------------------

/// Reads a text VDF file.
///
/// Decoded lossily on purpose. A `localconfig.vdf` holding one byte of a legacy
/// code page inside a persona name would otherwise cost the user every launch
/// option in the file, and a scan that loses a whole file to one bad byte is a
/// scan that loses games for a reason nobody can see.
fn iqra_vdf(masar: &Path) -> Natija<QeemaVdf> {
    let bayt = std::fs::read(masar).map_err(|sabab| {
        Khata::min_tafsir(&KhataKashf::TaadhurQiraatFahras {
            matjar: MUARRIF,
            masar: masar.to_path_buf(),
            sabab,
        })
    })?;
    let nass = String::from_utf8_lossy(&bayt);
    vdf::iqra_nassi_bi_masar(masar, nass.as_ref())
}

/// Reads a binary VDF file.
fn iqra_vdf_thunai(masar: &Path) -> Natija<QeemaVdf> {
    let bayt = std::fs::read(masar).map_err(|sabab| {
        Khata::min_tafsir(&KhataKashf::TaadhurQiraatFahras {
            matjar: MUARRIF,
            masar: masar.to_path_buf(),
            sabab,
        })
    })?;
    vdf::iqra_thunai_bi_masar(masar, &bayt)
}

/// A path in the form used for comparison: symbolic links resolved where the
/// filesystem will resolve them, so that `~/.steam/steam` and
/// `~/.local/share/Steam` are recognised as the one library they are.
fn sawwi(masar: &Path) -> PathBuf {
    std::fs::canonicalize(masar).unwrap_or_else(|_| masar.to_path_buf())
}

// ---------------------------------------------------------------------------
// Libraries
// ---------------------------------------------------------------------------

/// One Steam library: a drive, a folder, and the games under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaktabatSteam {
    /// The library root — the directory that holds `steamapps`.
    pub jidhr: PathBuf,
    /// The `steamapps` directory itself, in the case this filesystem has it.
    pub steamapps: PathBuf,
    /// The name the user gave the library, when they gave it one.
    pub laqab: Option<String>,
    /// The application identifiers `libraryfolders.vdf` lists for it. Advisory
    /// only: the manifests on disk are what a scan actually enumerates, because
    /// the index can be stale and a manifest cannot.
    pub alaab: Vec<u32>,
}

impl MaktabatSteam {
    /// Where compatibility prefixes for this library's games live.
    #[must_use]
    pub fn compatdata(&self) -> PathBuf {
        self.steamapps.join("compatdata")
    }

    /// Where this library's games are installed.
    #[must_use]
    pub fn common(&self) -> PathBuf {
        self.steamapps.join("common")
    }
}

/// Every library on every drive.
///
/// `libraryfolders.vdf` has had two shapes. Before 2021 a numbered key mapped
/// straight to a path string; since then it maps to an object carrying `path`,
/// `label`, `apps` and a handful of counters. Both are read, because an
/// installation that has not been opened in years still has the old one, and
/// the file also sits in two places depending on the client's age.
///
/// A library whose `steamapps` directory is not there is a drive that is
/// unplugged, a network share that is down, or a folder the user deleted by
/// hand. That is a warning and the scan continues: the other libraries still
/// have games in them, and telling the user which drive is missing is far more
/// useful than telling them the scan failed.
#[must_use]
pub fn maktabat(jidhr: &Path, tanbihat: &mut Vec<TanbihFahs>) -> Vec<MaktabatSteam> {
    let mut maktabat: Vec<MaktabatSteam> = Vec::new();
    let mut maruf: Vec<PathBuf> = Vec::new();

    let adif = |jidhr_maktaba: PathBuf,
                laqab: Option<String>,
                alaab: Vec<u32>,
                maktabat: &mut Vec<MaktabatSteam>,
                maruf: &mut Vec<PathBuf>,
                tanbihat: &mut Vec<TanbihFahs>| {
        let muwahhad = sawwi(&jidhr_maktaba);
        if maruf.contains(&muwahhad) {
            return;
        }
        let Some(steamapps) = steamapps(&jidhr_maktaba) else {
            tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                jidhr_maktaba.display().to_string(),
                "this Steam library has no steamapps folder — the drive is probably disconnected, \
                 or the folder was removed by hand",
            ));
            return;
        };
        maruf.push(muwahhad);
        maktabat.push(MaktabatSteam { jidhr: jidhr_maktaba, steamapps, laqab, alaab });
    };

    adif(jidhr.to_path_buf(), None, Vec::new(), &mut maktabat, &mut maruf, tanbihat);

    let mut fahras = None;
    for murashah in [
        jidhr.join("steamapps").join("libraryfolders.vdf"),
        jidhr.join("SteamApps").join("libraryfolders.vdf"),
        jidhr.join("config").join("libraryfolders.vdf"),
    ] {
        if murashah.is_file() {
            fahras = Some(murashah);
            break;
        }
    }
    let Some(masar_fahras) = fahras else { return maktabat };

    let shajara = match iqra_vdf(&masar_fahras) {
        Ok(shajara) => shajara,
        Err(khata) => {
            // The list of every other library is in this file; without it the
            // games on every other drive are unseen, not absent.
            tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                masar_fahras.display().to_string(),
                khata.injilizi,
            ));
            return maktabat;
        }
    };

    // The root key is `libraryfolders` on every client that writes the current
    // shape and `LibraryFolders` on the old one; lookups ignore case, so one
    // path covers both. A file with neither is read as if its root were the
    // list, which is what a hand-trimmed file looks like.
    let qaima = shajara.bi_masar(&["libraryfolders"]).unwrap_or(&shajara);
    let Some(abna) = qaima.kain() else { return maktabat };

    for (miftah, qeema) in abna {
        if miftah.parse::<u32>().is_err() {
            // `TimeNextStatsReport`, `ContentStatsID` and friends live beside
            // the numbered entries in the legacy shape.
            continue;
        }
        let (masar, laqab, alaab) = match qeema {
            QeemaVdf::Nass(masar) => (masar.clone(), None, Vec::new()),
            QeemaVdf::Kain(_) => {
                let Some(masar) = qeema.nass_bi_masar(&["path"]) else {
                    tanbihat.push(TanbihFahs::fahras(
                        MUARRIF,
                        format!("{}#{miftah}", masar_fahras.display()),
                        "a library entry has no path, so whatever library it names was not read",
                    ));
                    continue;
                };
                let laqab = qeema
                    .nass_bi_masar(&["label"])
                    .filter(|laqab| !laqab.is_empty())
                    .map(str::to_owned);
                let alaab = qeema
                    .kain_bi_masar(&["apps"])
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|(app, _)| app.parse::<u32>().ok())
                    .collect();
                (masar.to_owned(), laqab, alaab)
            }
            QeemaVdf::Raqm(_) | QeemaVdf::Kabir(_) => continue,
        };
        adif(PathBuf::from(masar), laqab, alaab, &mut maktabat, &mut maruf, tanbihat);
    }

    maktabat
}

// ---------------------------------------------------------------------------
// appmanifest_*.acf
// ---------------------------------------------------------------------------

/// One `appmanifest_*.acf`, read whole.
///
/// More than [`LubaMuktashafa`] carries, because the extra fields have owners
/// downstream: the branch decides which build a patch must match, the language
/// decides which string table was installed, and the depot list is what Phase
/// 14 fingerprints against.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BayanTathbeet {
    /// The application identifier.
    pub app: u32,
    /// The name Steam displays.
    pub ism: Option<String>,
    /// The folder under `steamapps/common`, relative and untrusted.
    pub mujallad: Option<String>,
    /// `StateFlags`, interpreted by [`muktamila`].
    pub alam: u32,
    /// The installed build.
    pub bina: Option<u64>,
    /// When Steam last updated the game, Unix seconds, zero when never.
    pub akhir_tahdith: i64,
    /// When the user last played, Unix seconds, zero when never.
    pub akhir_laab: i64,
    /// Size on disk in bytes as Steam accounts for it.
    pub hajm: u64,
    /// The language the user installed, from `UserConfig`.
    pub lugha: Option<String>,
    /// The beta branch the user selected, from `UserConfig`. Absent means the
    /// public branch.
    pub far: Option<String>,
    /// The account that installed it, from `LastOwner`.
    pub malik: Option<u64>,
    /// The depots installed, from `InstalledDepots`.
    pub mustawdaat: Vec<u64>,
}

/// Reads one `appmanifest_*.acf`.
///
/// # Errors
///
/// [`KhataKashf::TaadhurQiraatFahras`] when the file will not open and
/// [`KhataKashf::TarwisatFahrasTalifa`] when it will not parse. A manifest with
/// no `appid` is the second of those: without an identifier there is nothing
/// the entry could be attached to.
pub fn iqra_bayan(masar: &Path) -> Natija<BayanTathbeet> {
    let shajara = iqra_vdf(masar)?;
    // Steam writes `AppState`; a few third-party tools write `appstate`. The
    // lookup ignores case, and a file with neither is read as if its root were
    // the state, which is what a manifest edited by hand looks like.
    let hala_app = shajara.bi_masar(&["AppState"]).unwrap_or(&shajara);

    let app = hala_app
        .kabir_bi_masar(&["appid"])
        .and_then(|raqm| u32::try_from(raqm).ok())
        .ok_or_else(|| {
            Khata::min_tafsir(&KhataKashf::TarwisatFahrasTalifa {
                matjar: MUARRIF,
                masar: masar.to_path_buf(),
                tafsil: "the manifest carries no usable appid".to_owned(),
                mawdi: None,
            })
        })?;

    let mustawdaat = hala_app
        .kain_bi_masar(&["InstalledDepots"])
        .unwrap_or_default()
        .iter()
        .filter_map(|(mustawda, _)| mustawda.parse::<u64>().ok())
        .collect();

    let alam = hala_app
        .kabir_bi_masar(&["StateFlags"])
        .and_then(|raqm| u32::try_from(raqm).ok())
        .unwrap_or(0);

    Ok(BayanTathbeet {
        app,
        ism: hala_app.nass_bi_masar(&["name"]).map(str::to_owned),
        mujallad: hala_app
            .nass_bi_masar(&["installdir"])
            .filter(|mujallad| !mujallad.is_empty())
            .map(str::to_owned),
        alam,
        bina: hala_app.kabir_bi_masar(&["buildid"]),
        akhir_tahdith: hala_app.raqm_bi_masar(&["LastUpdated"]).unwrap_or(0),
        akhir_laab: hala_app.raqm_bi_masar(&["LastPlayed"]).unwrap_or(0),
        hajm: hala_app.kabir_bi_masar(&["SizeOnDisk"]).unwrap_or(0),
        lugha: hala_app
            .nass_bi_masar(&["UserConfig", "language"])
            .or_else(|| hala_app.nass_bi_masar(&["MountedConfig", "language"]))
            .filter(|lugha| !lugha.is_empty())
            .map(str::to_owned),
        far: hala_app
            .nass_bi_masar(&["UserConfig", "BetaKey"])
            .or_else(|| hala_app.nass_bi_masar(&["MountedConfig", "BetaKey"]))
            .filter(|far| !far.is_empty())
            .map(str::to_owned),
        malik: hala_app.kabir_bi_masar(&["LastOwner"]),
        mustawdaat,
    })
}

// ---------------------------------------------------------------------------
// appinfo.vdf
// ---------------------------------------------------------------------------

/// Store categories, by the numbers Steam uses for them, that mean the game is
/// played with other people over a network.
///
/// 1 is the generic multi-player flag, 20 is MMO, 27 is cross-platform
/// multiplayer, 36 is online `PvP` and 38 is online co-op.
const FIAT_ONLINE: [u32; 5] = [1, 20, 27, 36, 38];

/// The same for playing with other people on one machine: 9 co-op, 24 shared or
/// split screen, 37 shared-screen `PvP`, 39 shared-screen co-op, 44 Remote Play
/// Together, 47 LAN `PvP`, 48 LAN co-op.
const FIAT_MAHALLI: [u32; 7] = [9, 24, 37, 39, 44, 47, 48];

/// The category that means Valve Anti-Cheat is enabled for this app.
const FIAT_VAC: u32 = 8;

/// `common/type` values that are not games, lowercased.
///
/// They are discovered and reported like anything else, but marked so the
/// library hides them by default: a user with three hundred entries does not
/// want two hundred of them to be soundtracks, redistributables and demos.
const ANWA_LAYSAT_LUBA: [&str; 10] = [
    "tool",
    "application",
    "music",
    "video",
    "media",
    "demo",
    "config",
    "dlc",
    "hardware",
    "series",
];

/// Substrings that name an anti-cheat service wherever they appear in an app's
/// end-user agreements or its launch entries.
const KALIMAT_HIMAYA: [(&str, &str); 4] = [
    ("easyanticheat", "Easy Anti-Cheat"),
    ("easy anti-cheat", "Easy Anti-Cheat"),
    ("battleye", "BattlEye"),
    ("start_protected_game", "Easy Anti-Cheat"),
];

/// What `appinfo.vdf` says about one app, reduced to what a scan uses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaalumatApp {
    /// `common/type`, verbatim: `Game`, `Tool`, `Music`, `Demo`, and so on.
    pub naw: Option<String>,
    /// `common/name`, the store's own name for it.
    pub ism: Option<String>,
    /// What the store metadata implies, as hints for the safety layer.
    pub simat: Vec<SimatLuba>,
    /// The executable Steam launches, relative to the install directory and
    /// with separators as the store wrote them.
    pub tanfidhi: Option<String>,
    /// The arguments Steam passes it. Not the user's launch options — those
    /// live in `localconfig.vdf` and are read separately.
    pub muaamalat: Option<String>,
    /// Whether the executable chosen is a Windows one on a host that is not
    /// Windows, which is the store's own way of saying this title needs a
    /// compatibility layer.
    pub tawafuq_matlub: bool,
}

impl MaalumatApp {
    /// Whether anything at all was learned, so that empty records are never
    /// kept.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.naw.is_none()
            && self.ism.is_none()
            && self.simat.is_empty()
            && self.tanfidhi.is_none()
    }
}

/// Reads `appinfo.vdf` once and keeps only the apps asked for.
///
/// `matlub` is the sorted list of application identifiers the scan actually
/// found installed. A quarter of a million apps are in the file and a few
/// hundred are on the disk; filtering on the way through is the difference
/// between a few hundred small records and forty megabytes of them.
///
/// Every failure here degrades rather than fails. An `appinfo.vdf` that will
/// not parse costs categories, the anti-cheat hint and the executable name; it
/// does not cost the user their library, because the manifests already answered
/// the questions that matter for finding and patching a game.
#[must_use]
pub fn fahras_appinfo(
    jidhr: &Path,
    nizam: NizamTashghil,
    matlub: &[u32],
    tanbihat: &mut Vec<TanbihFahs>,
) -> BTreeMap<u32, MaalumatApp> {
    let mut fahras = BTreeMap::new();
    let masar = jidhr.join("appcache").join("appinfo.vdf");
    if !masar.is_file() || matlub.is_empty() {
        return fahras;
    }

    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) => {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                format!("cannot read Steam's app metadata cache: {:?}", sabab.kind()),
            ));
            return fahras;
        }
    };

    let mut talifa: usize = 0;
    let natija = vdf::murur_appinfo(&masar, &bayt, &mut |madkhal| match madkhal {
        Ok(madkhal) => {
            if matlub.binary_search(&madkhal.app).is_err() {
                return;
            }
            let maalumat = maalumat_app(&madkhal.bayanat, nizam);
            if !maalumat.khali() {
                let _ = fahras.insert(madkhal.app, maalumat);
            }
        }
        Err(_) => talifa += 1,
    });

    if let Err(khata) = natija {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            khata.injilizi,
        ));
    } else if talifa > 0 {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            format!(
                "{talifa} app entries in Steam's metadata cache could not be read; those games \
                 keep their manifest details but lose their store categories"
            ),
        ));
    }

    fahras
}

/// Projects one app's `appinfo.vdf` tree into the handful of facts a scan uses.
#[must_use]
pub fn maalumat_app(bayanat: &QeemaVdf, nizam: NizamTashghil) -> MaalumatApp {
    let mut maalumat = MaalumatApp {
        naw: bayanat
            .nass_bi_masar(&["appinfo", "common", "type"])
            .filter(|naw| !naw.is_empty())
            .map(str::to_owned),
        ism: bayanat
            .nass_bi_masar(&["appinfo", "common", "name"])
            .filter(|ism| !ism.is_empty())
            .map(str::to_owned),
        ..MaalumatApp::default()
    };

    if let Some(naw) = maalumat.naw.as_deref()
        && ANWA_LAYSAT_LUBA.contains(&naw.to_ascii_lowercase().as_str())
    {
        maalumat.simat.push(SimatLuba::LaysatLuba(naw.to_owned()));
    }

    let mut online = false;
    let mut mahalli = false;
    for (miftah, qeema) in bayanat.kain_bi_masar(&["appinfo", "common", "category"]).unwrap_or(&[])
    {
        // Steam writes these as `category_36` with a value of 1. A value that
        // is present but zero means the category was cleared, so both halves
        // are checked.
        if qeema.raqm().unwrap_or(1) == 0 {
            continue;
        }
        let Some(raqm) = miftah.rsplit('_').next().and_then(|raqm| raqm.parse::<u32>().ok())
        else {
            continue;
        };
        if raqm == FIAT_VAC {
            maalumat.simat.push(SimatLuba::MuammanaVac);
        }
        online |= FIAT_ONLINE.contains(&raqm);
        mahalli |= FIAT_MAHALLI.contains(&raqm);
    }
    if online {
        maalumat.simat.push(SimatLuba::JamaiOnline);
    }
    if mahalli {
        maalumat.simat.push(SimatLuba::JamaiMahalli);
    }

    if let Some(himaya) = himaya_muhtamala(bayanat) {
        maalumat.simat.push(SimatLuba::HimayaMuhtamala(himaya));
    }

    if let Some((tanfidhi, muaamalat, windows)) = ikhtar_tashghil(bayanat, nizam) {
        maalumat.tanfidhi = Some(tanfidhi);
        maalumat.muaamalat = muaamalat;
        maalumat.tawafuq_matlub = windows && nizam != NizamTashghil::Windows;
    }

    maalumat
}

/// Looks for an anti-cheat service in the two places `appinfo.vdf` names one.
///
/// A game that ships Easy Anti-Cheat or `BattlEye` carries that agreement in
/// `common/eulas`, and usually launches through a protected loader named in
/// `config/launch`. Neither is proof — Phase 16 decides by evidence on disk and
/// refuses on that basis — but both are enough for the interface to warn before
/// the user clicks, and enough to make that refusal instant instead of after a
/// scan of the install directory.
fn himaya_muhtamala(bayanat: &QeemaVdf) -> Option<String> {
    let mut mawadd: Vec<&str> = Vec::new();
    for (_, ittifaq) in bayanat.kain_bi_masar(&["appinfo", "common", "eulas"]).unwrap_or(&[]) {
        mawadd.extend(ittifaq.nass_bi_masar(&["name"]));
        mawadd.extend(ittifaq.nass_bi_masar(&["id"]));
    }
    for (_, madkhal) in bayanat.kain_bi_masar(&["appinfo", "config", "launch"]).unwrap_or(&[]) {
        mawadd.extend(madkhal.nass_bi_masar(&["executable"]));
    }

    for madda in mawadd {
        let saghir = madda.to_ascii_lowercase();
        for (kalima, ism) in KALIMAT_HIMAYA {
            if saghir.contains(kalima) {
                return Some(ism.to_owned());
            }
        }
    }
    None
}

/// The token `appinfo.vdf` uses for this operating system in an `oslist`.
const fn ism_nizam(nizam: NizamTashghil) -> &'static str {
    match nizam {
        NizamTashghil::Windows => "windows",
        NizamTashghil::Linux => "linux",
        NizamTashghil::Mac => "macos",
    }
}

/// Chooses the launch entry Steam itself would use, and says whether it is a
/// Windows one.
///
/// The order is: an entry for this operating system on the public branch, then
/// any entry for this operating system, then a Windows entry on the public
/// branch — which is how a Windows-only game reaches a Linux user through
/// Proton — then anything at all. Returning the executable rather than a
/// resolved path keeps the untrusted relative component untouched until the
/// caller joins it through a checked join.
fn ikhtar_tashghil(
    bayanat: &QeemaVdf,
    nizam: NizamTashghil,
) -> Option<(String, Option<String>, bool)> {
    /// One `config/launch/<n>` entry, reduced to what ranking needs.
    struct MadkhalTashghil<'a> {
        tarteeb: u32,
        tanfidhi: &'a str,
        muaamalat: Option<&'a str>,
        /// This entry declares the host operating system, or declares none.
        li_hadha: bool,
        /// This entry declares Windows.
        li_windows: bool,
        /// This entry belongs to the public branch.
        aam: bool,
    }

    let hadha = ism_nizam(nizam);
    let mut madakhil: Vec<MadkhalTashghil<'_>> = Vec::new();

    for (miftah, madkhal) in bayanat.kain_bi_masar(&["appinfo", "config", "launch"]).unwrap_or(&[])
    {
        let Some(tanfidhi) =
            madkhal.nass_bi_masar(&["executable"]).filter(|tanfidhi| !tanfidhi.is_empty())
        else {
            continue;
        };
        let qaima = madkhal.nass_bi_masar(&["config", "oslist"]).unwrap_or("").to_ascii_lowercase();
        let li_hadha =
            qaima.is_empty() || qaima.split(',').map(str::trim).any(|wahid| wahid == hadha);
        let li_windows = qaima.split(',').map(str::trim).any(|wahid| wahid == "windows");
        madakhil.push(MadkhalTashghil {
            tarteeb: miftah.parse::<u32>().unwrap_or(u32::MAX),
            tanfidhi,
            muaamalat: madkhal
                .nass_bi_masar(&["arguments"])
                .filter(|muaamalat| !muaamalat.is_empty()),
            li_hadha,
            li_windows,
            aam: madkhal.nass_bi_masar(&["config", "betakey"]).unwrap_or("").is_empty(),
        });
    }
    madakhil.sort_by_key(|madkhal| madkhal.tarteeb);

    let mukhtar = madakhil
        .iter()
        .find(|madkhal| madkhal.li_hadha && madkhal.aam)
        .or_else(|| madakhil.iter().find(|madkhal| madkhal.li_hadha))
        .or_else(|| madakhil.iter().find(|madkhal| madkhal.li_windows && madkhal.aam))
        .or_else(|| madakhil.iter().find(|madkhal| madkhal.li_windows))
        .or_else(|| madakhil.first())?;

    // A game that declares no `oslist` at all and ships a `.exe` is a Windows
    // game whose store entry predates the field, and there are thousands of
    // them.
    let windows =
        mukhtar.li_windows || mukhtar.tanfidhi.to_ascii_lowercase().ends_with(".exe");

    Some((mukhtar.tanfidhi.to_owned(), mukhtar.muaamalat.map(str::to_owned), windows))
}

// ---------------------------------------------------------------------------
// config.vdf — the compatibility tool mapping
// ---------------------------------------------------------------------------

/// The compatibility tool Steam has mapped to each game.
///
/// Key `0` is the global default — the tool applied to every title the user has
/// not overridden, which is what "Enable Steam Play for all other titles" sets.
/// The name is Steam's internal identifier for the tool (`proton_experimental`,
/// `proton_9`, `GE-Proton9-20`), which is what the launch path and the DLL
/// override syntax depend on, so it is kept verbatim rather than prettified.
#[must_use]
pub fn kharitat_tawafuq(jidhr: &Path, tanbihat: &mut Vec<TanbihFahs>) -> BTreeMap<u32, String> {
    let mut kharita = BTreeMap::new();
    let masar = jidhr.join("config").join("config.vdf");
    if !masar.is_file() {
        return kharita;
    }
    let shajara = match iqra_vdf(&masar) {
        Ok(shajara) => shajara,
        Err(khata) => {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                khata.injilizi,
            ));
            return kharita;
        }
    };

    // The client has spelled this path `Valve` and `valve` in different builds,
    // which is the whole reason lookups here ignore case.
    let Some(mapping) = shajara.bi_masar(&[
        "InstallConfigStore",
        "Software",
        "Valve",
        "Steam",
        "CompatToolMapping",
    ]) else {
        return kharita;
    };

    for (miftah, madkhal) in mapping.kain().unwrap_or(&[]) {
        let Ok(app) = miftah.parse::<u32>() else { continue };
        let Some(ism) = madkhal.nass_bi_masar(&["name"]).filter(|ism| !ism.is_empty()) else {
            continue;
        };
        let _ = kharita.insert(app, ism.to_owned());
    }
    kharita
}

// ---------------------------------------------------------------------------
// userdata
// ---------------------------------------------------------------------------

/// One Steam account with a profile directory on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MustakhdimSteam {
    /// The account number, which is what names the directory.
    pub raqm: u32,
    /// `userdata/<raqm>`.
    pub jidhr: PathBuf,
    /// The display name, when `loginusers.vdf` records one.
    pub ism: Option<String>,
    /// When this account last signed in, Unix seconds.
    pub akhir: i64,
    /// Whether Steam marks this as the account it would sign in as next.
    pub muakhkhar: bool,
}

impl MustakhdimSteam {
    /// This account's configuration directory.
    #[must_use]
    pub fn tahyia(&self) -> PathBuf {
        self.jidhr.join("config")
    }
}

/// Every account with a profile directory, most recently used first.
///
/// `config/loginusers.vdf` names the account Steam would sign in as and stamps
/// each with a timestamp, so it decides the order. When it is missing or says
/// nothing, the modification time of each account's own `localconfig.vdf` is
/// the fallback, which is written every time that account closes the client.
///
/// The order matters because a machine with two accounts has two sets of launch
/// options, and applying the wrong one would mean Phase 15 extending options
/// the person at the keyboard never set.
#[must_use]
pub fn mustakhdimun(jidhr: &Path) -> Vec<MustakhdimSteam> {
    let mut mustakhdimun: Vec<MustakhdimSteam> = Vec::new();
    let bayanat = jidhr.join("userdata");
    let Ok(mudkhalat) = std::fs::read_dir(&bayanat) else { return mustakhdimun };

    for madkhal in mudkhalat.flatten() {
        let masar = madkhal.path();
        if !masar.is_dir() {
            continue;
        }
        let Some(Ok(raqm)) = masar.file_name().and_then(|ism| ism.to_str()).map(str::parse::<u32>)
        else {
            continue;
        };
        // Account 0 is the directory Steam keeps for "no account", and it holds
        // no launch options.
        if raqm == 0 {
            continue;
        }
        let akhir = masar
            .join("config")
            .join("localconfig.vdf")
            .metadata()
            .ok()
            .and_then(|bayan| bayan.modified().ok())
            .and_then(|waqt| waqt.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|muddat| i64::try_from(muddat.as_secs()).ok())
            .unwrap_or(0);
        mustakhdimun.push(MustakhdimSteam {
            raqm,
            jidhr: masar,
            ism: None,
            akhir,
            muakhkhar: false,
        });
    }

    let masar_dukhul = jidhr.join("config").join("loginusers.vdf");
    if let Ok(shajara) = iqra_vdf(&masar_dukhul) {
        let qaima = shajara.bi_masar(&["users"]).unwrap_or(&shajara);
        for (huwiya, hisab) in qaima.kain().unwrap_or(&[]) {
            let Ok(kamila) = huwiya.parse::<u64>() else { continue };
            let Ok(raqm) = u32::try_from(kamila.saturating_sub(ASAS_HUWIYA)) else { continue };
            let Some(mustakhdim) = mustakhdimun.iter_mut().find(|wahid| wahid.raqm == raqm) else {
                continue;
            };
            mustakhdim.ism = hisab
                .nass_bi_masar(&["PersonaName"])
                .or_else(|| hisab.nass_bi_masar(&["AccountName"]))
                .filter(|ism| !ism.is_empty())
                .map(str::to_owned);
            mustakhdim.muakhkhar = hisab.raqm_bi_masar(&["MostRecent"]).unwrap_or(0) != 0;
            if let Some(waqt) = hisab.raqm_bi_masar(&["Timestamp"]).filter(|waqt| *waqt > 0) {
                mustakhdim.akhir = waqt;
            }
        }
    }

    mustakhdimun.sort_by(|awwal, thani| {
        thani
            .muakhkhar
            .cmp(&awwal.muakhkhar)
            .then(thani.akhir.cmp(&awwal.akhir))
            .then(awwal.raqm.cmp(&thani.raqm))
    });
    mustakhdimun
}

/// What one account's `localconfig.vdf` records about one app.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KhiyaratApp {
    /// The launch options the user typed, verbatim, including the `%command%`
    /// placeholder when they used one. Phase 15 extends this string; it never
    /// replaces it.
    pub khiyarat: Option<String>,
    /// When this account last played, Unix seconds, zero when never.
    pub akhir_laab: i64,
}

/// Reads every account's launch options, most recently used account first.
///
/// Later accounts fill in only what earlier ones did not mention, so the person
/// who used the machine last wins every conflict while a game only ever played
/// by the other account still keeps its options.
#[must_use]
pub fn khiyarat_mustakhdimin(
    mustakhdimun: &[MustakhdimSteam],
    tanbihat: &mut Vec<TanbihFahs>,
) -> BTreeMap<u32, KhiyaratApp> {
    let mut khiyarat: BTreeMap<u32, KhiyaratApp> = BTreeMap::new();

    for mustakhdim in mustakhdimun {
        let masar = mustakhdim.tahyia().join("localconfig.vdf");
        if !masar.is_file() {
            continue;
        }
        let shajara = match iqra_vdf(&masar) {
            Ok(shajara) => shajara,
            Err(khata) => {
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    masar.display().to_string(),
                    khata.injilizi.clone(),
                ));
                continue;
            }
        };
        let Some(alaab) = shajara.bi_masar(&[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
        ]) else {
            continue;
        };

        for (miftah, madkhal) in alaab.kain().unwrap_or(&[]) {
            let Ok(app) = miftah.parse::<u32>() else { continue };
            let wahid = KhiyaratApp {
                khiyarat: madkhal
                    .nass_bi_masar(&["LaunchOptions"])
                    .filter(|khiyarat| !khiyarat.is_empty())
                    .map(str::to_owned),
                akhir_laab: madkhal.raqm_bi_masar(&["LastPlayed"]).unwrap_or(0),
            };
            if wahid.khiyarat.is_none() && wahid.akhir_laab == 0 {
                continue;
            }
            khiyarat.entry(app).or_insert(wahid);
        }
    }

    khiyarat
}

// ---------------------------------------------------------------------------
// shortcuts.vdf
// ---------------------------------------------------------------------------

/// A non-Steam game the user added to their Steam library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ikhtisar {
    /// Steam's generated identifier for the entry: a CRC-32 of the stored
    /// executable string and the display name, with the top bit set. It names
    /// the compatibility prefix, the artwork files and the launch options, and
    /// it means nothing on any other machine.
    pub muarrif: u32,
    /// The name the user gave it.
    pub ism: String,
    /// The executable, with the quotes Steam stores around it removed.
    pub tanfidhi: PathBuf,
    /// The working directory, which is the install root for Taarib's purposes.
    pub jidhr: PathBuf,
    /// The launch options set on the shortcut itself.
    pub khiyarat: Option<String>,
    /// When it was last played, Unix seconds, zero when never.
    pub akhir_laab: i64,
    /// Whether the user hid it from their own library.
    pub mukhfi: bool,
    /// The Flatpak application this shortcut runs, when it runs one. Such a
    /// shortcut has no executable path of its own.
    pub flatpak: Option<String>,
}

/// The 64-bit identity Steam uses for a shortcut in `steam://rungameid` URLs.
///
/// The 32-bit form is what names directories on disk, which is why it is the
/// one [`Ikhtisar`] keeps; this is the form the client's own URL scheme takes.
#[must_use]
pub fn muarrif_ikhtisar_kamil(muarrif: u32) -> u64 {
    (u64::from(muarrif) << 32) | 0x0200_0000
}

/// Recomputes a shortcut's identifier the way Steam generates it.
///
/// CRC-32 over the executable string exactly as it is stored — including the
/// quotation marks Steam wraps it in — concatenated with the display name, then
/// the top bit set. Only needed for shortcuts written by clients old enough not
/// to store the `appid` field; every current client writes it, and the stored
/// value is preferred whenever it is there.
#[must_use]
pub fn muarrif_ikhtisar(tanfidhi_khaam: &str, ism: &str) -> u32 {
    let mut bidhra = String::with_capacity(tanfidhi_khaam.len() + ism.len());
    bidhra.push_str(tanfidhi_khaam);
    bidhra.push_str(ism);
    crc32(bidhra.as_bytes()) | 0x8000_0000
}

/// How a shortcut is reported to the rest of the product.
///
/// [`MasdarLuba::Yadawi`], never [`MasdarLuba::Steam`]. The reasoning is in the
/// module documentation: the identifier is a local hash, not a catalogue key,
/// and a patch published against it would be a patch that matches one computer.
#[must_use]
pub fn masdar_ikhtisar(muarrif: u32) -> MasdarLuba {
    MasdarLuba::Yadawi(format!("steam-shortcut:{muarrif}"))
}

/// CRC-32, the reflected IEEE polynomial, computed a bit at a time.
///
/// In-crate because it is needed in exactly one place, for strings a few dozen
/// bytes long, and a table would cost more to justify than the loop costs to
/// run.
fn crc32(bayt: &[u8]) -> u32 {
    let mut qeema: u32 = 0xFFFF_FFFF;
    for wahid in bayt {
        qeema ^= u32::from(*wahid);
        for _ in 0..8 {
            qeema = if qeema & 1 == 1 { (qeema >> 1) ^ 0xEDB8_8320 } else { qeema >> 1 };
        }
    }
    !qeema
}

/// Strips the quotation marks Steam stores around a shortcut's paths.
fn bila_iqtibas(nass: &str) -> &str {
    nass.trim().trim_matches('"')
}

/// Reads every account's `shortcuts.vdf`.
///
/// Every account, not just the most recently used one: the games are installed
/// on this machine whoever added them, and a second account's entries are the
/// same files on the same disk. Duplicates across accounts collapse on the
/// identifier, which is derived from the executable and the name and so is the
/// same for the same shortcut wherever it was added.
#[must_use]
pub fn iqra_ikhtisarat(
    mustakhdimun: &[MustakhdimSteam],
    tanbihat: &mut Vec<TanbihFahs>,
) -> Vec<Ikhtisar> {
    let mut ikhtisarat: Vec<Ikhtisar> = Vec::new();

    for mustakhdim in mustakhdimun {
        let masar = mustakhdim.tahyia().join("shortcuts.vdf");
        if !masar.is_file() {
            continue;
        }
        let shajara = match iqra_vdf_thunai(&masar) {
            Ok(shajara) => shajara,
            Err(khata) => {
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    masar.display().to_string(),
                    khata.injilizi.clone(),
                ));
                continue;
            }
        };
        let qaima = shajara.bi_masar(&["shortcuts"]).unwrap_or(&shajara);

        for (fahras, madkhal) in qaima.kain().unwrap_or(&[]) {
            let ism = madkhal.nass_bi_masar(&["AppName"]).unwrap_or("").trim().to_owned();
            let tanfidhi_khaam = madkhal.nass_bi_masar(&["Exe"]).unwrap_or("");
            let flatpak = madkhal
                .nass_bi_masar(&["FlatpakAppID"])
                .filter(|muarrif| !muarrif.is_empty())
                .map(str::to_owned);

            if ism.is_empty() || (tanfidhi_khaam.is_empty() && flatpak.is_none()) {
                tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    format!("{}#{fahras}", masar.display()),
                    "a non-Steam shortcut has neither a name nor an executable",
                ));
                continue;
            }

            let muarrif = madkhal
                .bi_masar(&["appid"])
                .and_then(QeemaVdf::raqm)
                .and_then(|raqm| u32::try_from(raqm & 0xFFFF_FFFF).ok())
                .filter(|muarrif| *muarrif != 0)
                .unwrap_or_else(|| muarrif_ikhtisar(tanfidhi_khaam, &ism));

            if ikhtisarat.iter().any(|sabiq| sabiq.muarrif == muarrif) {
                continue;
            }

            let tanfidhi = PathBuf::from(bila_iqtibas(tanfidhi_khaam));
            let bidaya = madkhal
                .nass_bi_masar(&["StartDir"])
                .map(bila_iqtibas)
                .filter(|bidaya| !bidaya.is_empty());
            let jidhr = match bidaya {
                Some(bidaya) => PathBuf::from(bidaya),
                // A shortcut with no working directory is one Steam launches
                // from beside the executable, so that is where its files are.
                None => tanfidhi.parent().map(Path::to_path_buf).unwrap_or_default(),
            };

            ikhtisarat.push(Ikhtisar {
                muarrif,
                ism,
                tanfidhi,
                jidhr,
                khiyarat: madkhal
                    .nass_bi_masar(&["LaunchOptions"])
                    .filter(|khiyarat| !khiyarat.is_empty())
                    .map(str::to_owned),
                akhir_laab: madkhal.raqm_bi_masar(&["LastPlayTime"]).unwrap_or(0),
                mukhfi: madkhal.raqm_bi_masar(&["IsHidden"]).unwrap_or(0) != 0,
                flatpak,
            });
        }
    }

    ikhtisarat
}

// ---------------------------------------------------------------------------
// Artwork
// ---------------------------------------------------------------------------

/// Wraps a path as an artwork source when the file is actually there.
fn sura_mahalliya(masar: PathBuf) -> Option<MasdarSura> {
    if masar.is_file() { Some(MasdarSura::Malaf(masar)) } else { None }
}

/// The first of these names that exists in this directory.
fn awwal_mawjud<I: AsRef<Path>>(mujallad: &Path, asmaa: &[I]) -> Option<MasdarSura> {
    asmaa.iter().find_map(|ism| sura_mahalliya(mujallad.join(ism)))
}

/// Fills an artwork slot, but only if it is still empty and only by running the
/// search when it is — so the preference order costs nothing once a piece has
/// been found.
fn imla(hadaf: &mut Option<MasdarSura>, bahth: impl FnOnce() -> Option<MasdarSura>) {
    if hadaf.is_none() {
        *hadaf = bahth();
    }
}

/// The first file in this directory whose name starts with one of these
/// prefixes and ends in an image extension.
///
/// Needed because the newest clients name the per-app cache files with a
/// content hash appended — `library_600x900_1a2b3c.jpg` — so an exact-name
/// probe finds nothing on a machine that has the artwork sitting right there.
fn awwal_bi_badiya(mujallad: &Path, badiyat: &[&str]) -> Option<MasdarSura> {
    let mut murashahat: Vec<PathBuf> = std::fs::read_dir(mujallad)
        .ok()?
        .flatten()
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            let Some(imtidad) = masar.extension().and_then(|imtidad| imtidad.to_str()) else {
                return false;
            };
            if !matches!(imtidad.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "webp") {
                return false;
            }
            masar.file_name().and_then(|ism| ism.to_str()).is_some_and(|ism| {
                let saghir = ism.to_ascii_lowercase();
                badiyat.iter().any(|badiya| saghir.starts_with(badiya))
            })
        })
        .collect();
    murashahat.sort();
    murashahat.into_iter().next().map(MasdarSura::Malaf)
}

/// Finds a game's three artwork pieces.
///
/// In order of preference: the artwork the user set themselves under
/// `userdata/<id>/config/grid`, which is what a Steam Deck user with a
/// `SteamGridDB` collection has and which must never be overridden by the
/// client's own cache; then `appcache/librarycache`, in both the flat layout
/// and the per-app directory the newer clients use; then, for a real Steam app
/// only, the public endpoint. A shortcut has no catalogue entry, so there is no
/// address to fall back to and it simply has no artwork until the user gives it
/// some.
#[must_use]
pub fn suwar_app(
    jidhr: &Path,
    mustakhdimun: &[MustakhdimSteam],
    app: u32,
    fi_al_matjar: bool,
) -> MasadirSuwar {
    let mut suwar = MasadirSuwar::default();

    for mustakhdim in mustakhdimun {
        let shabaka = mustakhdim.tahyia().join("grid");
        if !shabaka.is_dir() {
            continue;
        }
        imla(&mut suwar.ghilaf, || {
            awwal_mawjud(&shabaka, &[format!("{app}p.png"), format!("{app}p.jpg")])
        });
        imla(&mut suwar.batl, || {
            awwal_mawjud(
                &shabaka,
                &[
                    format!("{app}_hero.png"),
                    format!("{app}_hero.jpg"),
                    format!("{app}.png"),
                    format!("{app}.jpg"),
                ],
            )
        });
        imla(&mut suwar.shiar, || {
            awwal_mawjud(&shabaka, &[format!("{app}_logo.png"), format!("{app}_logo.jpg")])
        });
    }

    let makhzan = jidhr.join("appcache").join("librarycache");
    if makhzan.is_dir() {
        imla(&mut suwar.ghilaf, || {
            awwal_mawjud(&makhzan, &[format!("{app}_library_600x900.jpg")])
        });
        imla(&mut suwar.batl, || {
            awwal_mawjud(
                &makhzan,
                &[format!("{app}_library_hero.jpg"), format!("{app}_header.jpg")],
            )
        });
        imla(&mut suwar.shiar, || awwal_mawjud(&makhzan, &[format!("{app}_logo.png")]));

        let khass = makhzan.join(app.to_string());
        if khass.is_dir() {
            imla(&mut suwar.ghilaf, || {
                awwal_mawjud(&khass, &["library_600x900.jpg", "library_600x900.png"])
                    .or_else(|| awwal_bi_badiya(&khass, &["library_600x900"]))
            });
            imla(&mut suwar.batl, || {
                awwal_mawjud(&khass, &["library_hero.jpg", "header.jpg"])
                    .or_else(|| awwal_bi_badiya(&khass, &["library_hero", "header"]))
            });
            imla(&mut suwar.shiar, || {
                awwal_mawjud(&khass, &["logo.png", "logo.jpg"])
                    .or_else(|| awwal_bi_badiya(&khass, &["logo"]))
            });
        }
    }

    if fi_al_matjar {
        imla(&mut suwar.ghilaf, || {
            Some(MasdarSura::Rabt(format!("{RABT_SUWAR}/{app}/library_600x900.jpg")))
        });
        imla(&mut suwar.batl, || {
            Some(MasdarSura::Rabt(format!("{RABT_SUWAR}/{app}/library_hero.jpg")))
        });
        imla(&mut suwar.shiar, || {
            Some(MasdarSura::Rabt(format!("{RABT_SUWAR}/{app}/logo.png")))
        });
    }

    suwar
}

// ---------------------------------------------------------------------------
// Proton
// ---------------------------------------------------------------------------

/// Resolves the compatibility environment a Steam title runs in.
///
/// Only Linux has one: a Windows game on Windows is native, and macOS has no
/// Steam Play. The prefix is `steamapps/compatdata/<id>/pfx` **inside the
/// library that holds the game**, not inside the Steam root, because Steam puts
/// the prefix next to the install.
///
/// The Proton build is resolved down a ladder, most specific first: the tool
/// this game is mapped to in `config.vdf`, then the global default tool, then
/// the `version` file Proton writes beside the prefix when it builds it, then
/// whatever `beea` reads out of the prefix itself. A prefix with none of the
/// four is still a Proton prefix and is reported as one.
fn beeat_app(
    maktaba: &MaktabatSteam,
    app: u32,
    adaat: &BTreeMap<u32, String>,
    nizam: NizamTashghil,
    tanbihat: &mut Vec<TanbihFahs>,
) -> BeeatTawafuq {
    if nizam != NizamTashghil::Linux {
        return BeeatTawafuq::Asli;
    }
    let bayanat = maktaba.compatdata().join(app.to_string());
    let beea = bayanat.join("pfx");
    if !beea.is_dir() {
        return BeeatTawafuq::Asli;
    }

    let mut isdar = adaat.get(&app).or_else(|| adaat.get(&0)).cloned();

    if isdar.is_none() {
        // Proton writes `<unix time> <build name>` into this file when it
        // creates or upgrades the prefix, and it is the only record of which
        // build the files inside were actually made by.
        isdar = std::fs::read_to_string(bayanat.join("version")).ok().and_then(|nass| {
            nass.split_whitespace()
                .last()
                // A file holding only the timestamp names no build, and the
                // timestamp is not a version anybody could act on.
                .filter(|wasm| !wasm.chars().all(|harf| harf.is_ascii_digit()))
                .map(str::to_owned)
        });
    }

    match crate::beea::hal_beea(&beea) {
        Ok(maalumat) => {
            BeeatTawafuq::Proton {
                isdar: isdar.or(maalumat.isdar_wine).unwrap_or_else(|| "Proton".to_owned()),
                beea: maalumat.jidhr,
            }
        }
        Err(khata) => {
            // The prefix is there and the game does run through it, so the
            // environment is reported. What is lost is the drive map, and the
            // warning says so rather than pretending the game is native.
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                beea.display().to_string(),
                khata.injilizi,
            ));
            BeeatTawafuq::Proton {
                isdar: isdar.unwrap_or_else(|| "Proton".to_owned()),
                beea,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------

/// Turns Steam's Unix seconds into the RFC 3339 string the vocabulary types
/// carry.
///
/// Written here rather than taken from a date library because this crate does
/// not depend on one and a single UTC conversion does not justify adding it.
/// The civil-date arithmetic is the standard shifted-era algorithm: the year is
/// re-based to start in March so that the leap day lands at the end of it and
/// no month-length table is needed.
///
/// Zero and negative values are `None`: Steam writes zero for "never", and a
/// timestamp before 1970 in a Steam manifest is a corrupt field, not a date.
#[expect(
    clippy::integer_division,
    reason = "every division here is exact calendar arithmetic on values already \
              range-reduced by the line above it"
)]
fn waqt_rfc3339(thawani: i64) -> Option<String> {
    if thawani <= 0 {
        return None;
    }

    let ayyam = thawani.div_euclid(86_400);
    let fi_al_yawm = thawani.rem_euclid(86_400);
    let saa = fi_al_yawm / 3_600;
    let daqiqa = (fi_al_yawm % 3_600) / 60;
    let thaniya = fi_al_yawm % 60;

    let muzah = ayyam + 719_468;
    let mabda = if muzah >= 0 { muzah } else { muzah - 146_096 };
    let ahd = mabda / 146_097;
    let fi_al_ahd = muzah - ahd * 146_097;
    let sana_fi_al_ahd =
        (fi_al_ahd - fi_al_ahd / 1_460 + fi_al_ahd / 36_524 - fi_al_ahd / 146_096) / 365;
    let sana = sana_fi_al_ahd + ahd * 400;
    let fi_as_sana = fi_al_ahd - (365 * sana_fi_al_ahd + sana_fi_al_ahd / 4 - sana_fi_al_ahd / 100);
    let shahr_muzah = (5 * fi_as_sana + 2) / 153;
    let yawm = fi_as_sana - (153 * shahr_muzah + 2) / 5 + 1;
    let shahr = if shahr_muzah < 10 { shahr_muzah + 3 } else { shahr_muzah - 9 };
    let sana = if shahr <= 2 { sana + 1 } else { sana };

    Some(format!("{sana:04}-{shahr:02}-{yawm:02}T{saa:02}:{daqiqa:02}:{thaniya:02}Z"))
}

// ---------------------------------------------------------------------------
// The scan
// ---------------------------------------------------------------------------

/// Everything read once per scan and consulted once per game.
///
/// Assembled before the first manifest is turned into a game, so that no
/// per-game path opens a file that the whole scan could have opened once.
struct SiyaqSteam<'a> {
    /// The Steam root.
    jidhr: &'a Path,
    /// The host operating system.
    nizam: NizamTashghil,
    /// Store metadata, for the apps that are actually installed.
    fahras: BTreeMap<u32, MaalumatApp>,
    /// The compatibility tool mapped to each game, and the global default at
    /// key zero.
    adaat: BTreeMap<u32, String>,
    /// The user's own launch options and play times.
    khiyarat: BTreeMap<u32, KhiyaratApp>,
    /// Every account with a profile directory, most recently used first.
    mustakhdimun: Vec<MustakhdimSteam>,
}

/// Builds one game out of its manifest.
///
/// Returns `None` for an entry that cannot become a game, having pushed a
/// warning that says which file and why. A manifest whose install directory is
/// gone is the common case: Steam leaves the manifest behind for a while after
/// a folder is deleted from underneath it, and reporting a game whose files do
/// not exist would put an entry in the library that nothing downstream could
/// probe, patch, or launch.
fn luba_min_bayan(
    siyaq: &SiyaqSteam<'_>,
    maktaba: &MaktabatSteam,
    bayan: &BayanTathbeet,
    masar_bayan: &Path,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    let maalumat = siyaq.fahras.get(&bayan.app);

    let Some(mujallad) = bayan.mujallad.as_deref() else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar_bayan.display().to_string(),
            "the manifest names no install directory",
        ));
        return None;
    };

    let jidhr_luba = match dakhil(&maktaba.common(), mujallad) {
        Ok(jidhr_luba) => jidhr_luba,
        Err(khata) => {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar_bayan.display().to_string(),
                khata.injilizi,
            ));
            return None;
        }
    };

    if !jidhr_luba.is_dir() {
        let sabab = if bayan.alam & hala::GHAYR_MUTHABBAT != 0 {
            "the manifest is left over from an uninstall and its folder is gone"
        } else {
            "the install folder this manifest points at does not exist — the game was probably \
             deleted without Steam being told"
        };
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{} -> {}", masar_bayan.display(), jidhr_luba.display()),
            sabab,
        ));
        return None;
    }

    let beea = beeat_app(maktaba, bayan.app, &siyaq.adaat, siyaq.nizam, tanbihat);

    let mut simat = maalumat.map(|maalumat| maalumat.simat.clone()).unwrap_or_default();
    match &beea {
        BeeatTawafuq::Proton { isdar, .. } => {
            simat.push(SimatLuba::TabaqatTawafuq(isdar.clone()));
        }
        BeeatTawafuq::Asli | BeeatTawafuq::Wine { .. } | BeeatTawafuq::Rosetta => {
            if maalumat.is_some_and(|maalumat| maalumat.tawafuq_matlub)
                && let Some(ada) = siyaq.adaat.get(&bayan.app).or_else(|| siyaq.adaat.get(&0))
            {
                simat.push(SimatLuba::TabaqatTawafuq(ada.clone()));
            }
        }
    }

    // Steam writes launch executables with Windows separators even in the
    // metadata a Linux client reads, so they are normalised before the checked
    // join — otherwise `bin\game.exe` becomes one impossible file name.
    let tanfidhi = maalumat
        .and_then(|maalumat| maalumat.tanfidhi.as_deref())
        .map(|tanfidhi| tanfidhi.replace('\\', "/"))
        .and_then(|tanfidhi| dakhil(&jidhr_luba, &tanfidhi).ok())
        .filter(|tanfidhi| tanfidhi.is_file());

    let khiyarat_app = siyaq.khiyarat.get(&bayan.app);
    let akhir_laab = if bayan.akhir_laab > 0 {
        bayan.akhir_laab
    } else {
        khiyarat_app.map_or(0, |khiyarat| khiyarat.akhir_laab)
    };

    let ism = bayan
        .ism
        .clone()
        .filter(|ism| !ism.is_empty())
        .or_else(|| maalumat.and_then(|maalumat| maalumat.ism.clone()))
        .unwrap_or_else(|| mujallad.to_owned());

    Some(LubaMuktashafa {
        masdar: MasdarLuba::Steam(bayan.app),
        hala_matjar: None,
        ism,
        jidhr: jidhr_luba,
        tanfidhi,
        hajm: bayan.hajm,
        bina_manassa: bayan.bina.map(|bina| bina.to_string()),
        akhir_tahdith: waqt_rfc3339(bayan.akhir_tahdith),
        akhir_laab: waqt_rfc3339(akhir_laab),
        beea,
        suwar: suwar_app(siyaq.jidhr, &siyaq.mustakhdimun, bayan.app, true),
        khiyarat_tashghil: khiyarat_app.and_then(|khiyarat| khiyarat.khiyarat.clone()),
        muktamila: muktamila(bayan.alam),
        simat,
    })
}

/// Builds one game out of a non-Steam shortcut.
///
/// The shortcut's prefix is looked for in every library, because Steam creates
/// it under whichever library was current when the shortcut first ran — usually
/// the root one, but not on a Deck with an SD card.
fn luba_min_ikhtisar(
    siyaq: &SiyaqSteam<'_>,
    maktabat: &[MaktabatSteam],
    ikhtisar: &Ikhtisar,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    if !ikhtisar.jidhr.is_dir() {
        let sabab = ikhtisar.flatpak.as_ref().map_or_else(
            || {
                "this non-Steam shortcut points at a folder that is not there".to_owned()
            },
            |muarrif| {
                format!(
                    "this non-Steam shortcut runs the Flatpak application {muarrif} and records no \
                     folder of its own, so Taarib cannot locate its files"
                )
            },
        );
        tanbihat.push(TanbihFahs::jadeed(MUARRIF, ikhtisar.ism.clone(), sabab));
        return None;
    }

    let mut beea = BeeatTawafuq::Asli;
    if siyaq.nizam == NizamTashghil::Linux {
        for maktaba in maktabat {
            let muhtamal =
                beeat_app(maktaba, ikhtisar.muarrif, &siyaq.adaat, siyaq.nizam, tanbihat);
            if matches!(muhtamal, BeeatTawafuq::Proton { .. }) {
                beea = muhtamal;
                break;
            }
        }
    }

    let mut simat = Vec::new();
    if let BeeatTawafuq::Proton { isdar, .. } = &beea {
        simat.push(SimatLuba::TabaqatTawafuq(isdar.clone()));
    }

    let khiyarat_app = siyaq.khiyarat.get(&ikhtisar.muarrif);
    let akhir_laab = if ikhtisar.akhir_laab > 0 {
        ikhtisar.akhir_laab
    } else {
        khiyarat_app.map_or(0, |khiyarat| khiyarat.akhir_laab)
    };

    Some(LubaMuktashafa {
        masdar: masdar_ikhtisar(ikhtisar.muarrif),
        hala_matjar: None,
        ism: ikhtisar.ism.clone(),
        jidhr: ikhtisar.jidhr.clone(),
        tanfidhi: Some(ikhtisar.tanfidhi.clone()).filter(|tanfidhi| tanfidhi.is_file()),
        hajm: 0,
        bina_manassa: None,
        akhir_tahdith: None,
        akhir_laab: waqt_rfc3339(akhir_laab),
        beea,
        // No catalogue entry means no public address, so a shortcut gets
        // whatever the user set under `grid` and nothing else.
        suwar: suwar_app(siyaq.jidhr, &siyaq.mustakhdimun, ikhtisar.muarrif, false),
        khiyarat_tashghil: ikhtisar
            .khiyarat
            .clone()
            .or_else(|| khiyarat_app.and_then(|khiyarat| khiyarat.khiyarat.clone())),
        // A shortcut points at files the user already has. There is no download
        // to be part-way through.
        muktamila: true,
        simat,
    })
}

/// Collects every `appmanifest_*.acf` in one library.
fn bayanat_maktaba(
    maktaba: &MaktabatSteam,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Vec<(PathBuf, BayanTathbeet)> {
    let mut bayanat = Vec::new();
    let mudkhalat = match std::fs::read_dir(&maktaba.steamapps) {
        Ok(mudkhalat) => mudkhalat,
        Err(sabab) => {
            tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                maktaba.steamapps.display().to_string(),
                format!("cannot list this Steam library: {:?}", sabab.kind()),
            ));
            return bayanat;
        }
    };

    for madkhal in mudkhalat.flatten() {
        let masar = madkhal.path();
        let Some(ism) = masar.file_name().and_then(|ism| ism.to_str()) else { continue };
        let saghir = ism.to_ascii_lowercase();
        let imtidad_acf = Path::new(&saghir)
            .extension()
            .is_some_and(|imtidad| imtidad.eq_ignore_ascii_case("acf"));
        if !saghir.starts_with("appmanifest_") || !imtidad_acf {
            continue;
        }
        match iqra_bayan(&masar) {
            Ok(bayan) => bayanat.push((masar, bayan)),
            Err(khata) => tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                khata.injilizi.clone(),
            )),
        }
    }

    bayanat
}

impl Matjar for MatjarSteam {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "ستيم"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Steam"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &[NizamTashghil::Windows, NizamTashghil::Linux, NizamTashghil::Mac]
    }

    /// Where Steam is.
    ///
    /// When the user has set a location, that location is returned whether or
    /// not it exists. This is deliberate: a dispatcher that skips the scan
    /// because this returned `None` would swallow the user's mistake, and
    /// [`Matjar::ifhas`] is where they are told about it, with
    /// [`KhataKashf::JidhrMuhaddadMafqud`] naming the path they typed.
    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if let Some(tajawuz) = siyaq.manassat.steam.as_ref() {
            return Some(tajawuz.clone());
        }
        murashahat(siyaq).into_iter().find(|murashah| jidhr_salih(murashah))
    }

    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        let mut tanbihat: Vec<TanbihFahs> = Vec::new();

        let judhur = hall_judhur(siyaq)?;
        let Some(awwal) = judhur.first().cloned() else {
            let mut natija = NatijatMatjar::ghayr_mutah(MUARRIF);
            natija.muddat = bidaya.elapsed();
            return Ok(natija);
        };

        // One flat library list across every root, carrying the index of the
        // root each library came from. Two roots routinely list the same
        // external drive — a Flatpak Steam pointed at games a native Steam
        // already owns — and scanning it twice would put every game on it into
        // the library twice.
        let mut maktabat: Vec<MaktabatSteam> = Vec::new();
        let mut jidhr_maktaba: Vec<usize> = Vec::new();
        let mut maruf: Vec<PathBuf> = Vec::new();
        for (fahras_jidhr, jidhr) in judhur.iter().enumerate() {
            for maktaba in self::maktabat(jidhr, &mut tanbihat) {
                let muwahhad = sawwi(&maktaba.jidhr);
                if maruf.contains(&muwahhad) {
                    continue;
                }
                maruf.push(muwahhad);
                maktabat.push(maktaba);
                jidhr_maktaba.push(fahras_jidhr);
            }
        }

        let mut bayanat: Vec<(usize, PathBuf, BayanTathbeet)> = Vec::new();
        for (fahras, maktaba) in maktabat.iter().enumerate() {
            // Bound to a name before the loop: a `&mut` in a `for` head lives
            // as long as the loop, and the body needs the same borrow.
            let mahsuda = bayanat_maktaba(maktaba, &mut tanbihat);
            for (masar, bayan) in mahsuda {
                // The same app can have a manifest in two libraries after a
                // move that was interrupted. The first library wins, which is
                // the one Steam itself would find first.
                if bayanat.iter().any(|(_, _, sabiq)| sabiq.app == bayan.app) {
                    continue;
                }
                bayanat.push((fahras, masar, bayan));
            }
        }

        let mut matlub: Vec<u32> = bayanat.iter().map(|(_, _, bayan)| bayan.app).collect();
        matlub.sort_unstable();
        matlub.dedup();

        // The store index, the compatibility mapping and the signed-in accounts
        // are all per-root: a Flatpak Steam keeps its own `appinfo.vdf`, its own
        // `config.vdf` and its own `userdata`, and reading one root's metadata
        // against another root's games would mis-report both.
        let mut siyaqat: Vec<SiyaqSteam<'_>> = Vec::with_capacity(judhur.len());
        for jidhr in &judhur {
            let mustakhdimun = mustakhdimun(jidhr);
            siyaqat.push(SiyaqSteam {
                jidhr,
                nizam: siyaq.nizam,
                fahras: fahras_appinfo(jidhr, siyaq.nizam, &matlub, &mut tanbihat),
                adaat: kharitat_tawafuq(jidhr, &mut tanbihat),
                khiyarat: khiyarat_mustakhdimin(&mustakhdimun, &mut tanbihat),
                mustakhdimun,
            });
        }

        let mut alaab: Vec<LubaMuktashafa> = Vec::with_capacity(bayanat.len());
        for (fahras, masar, bayan) in &bayanat {
            let Some(maktaba) = maktabat.get(*fahras) else { continue };
            let Some(siyaq_steam) =
                jidhr_maktaba.get(*fahras).and_then(|fahras_jidhr| siyaqat.get(*fahras_jidhr))
            else {
                continue;
            };
            if let Some(luba) = luba_min_bayan(siyaq_steam, maktaba, bayan, masar, &mut tanbihat) {
                alaab.push(luba);
            }
        }

        // A shortcut's identifier is a CRC-32 of its executable string and its
        // name, so the same sideloaded game added under two roots carries the
        // same identifier and is admitted once.
        let mut ikhtisarat_maruf: Vec<u32> = Vec::new();
        for siyaq_steam in &siyaqat {
            let ikhtisarat = iqra_ikhtisarat(&siyaq_steam.mustakhdimun, &mut tanbihat);
            for ikhtisar in &ikhtisarat {
                if ikhtisarat_maruf.contains(&ikhtisar.muarrif) {
                    continue;
                }
                ikhtisarat_maruf.push(ikhtisar.muarrif);
                if let Some(luba) =
                    luba_min_ikhtisar(siyaq_steam, &maktabat, ikhtisar, &mut tanbihat)
                {
                    alaab.push(luba);
                }
            }
        }

        tracing::debug!(
            matjar = MUARRIF,
            judhur = judhur.len(),
            maktabat = maktabat.len(),
            alaab = alaab.len(),
            tanbihat = tanbihat.len(),
            "Steam scan complete"
        );

        // The first root is the one `mawqi` names and the one a diagnostics
        // screen shows; the others are folded into the same result because a
        // game is a game whichever client installed it.
        let mut natija = NatijatMatjar::muthabbat(MUARRIF, Some(awwal));
        natija.alaab = alaab;
        natija.tanbihat = tanbihat;
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let Ok(judhur_steam) = hall_judhur(siyaq) else { return Vec::new() };
        let mut mahmal: Vec<TanbihFahs> = Vec::new();
        let mut judhur: Vec<PathBuf> = Vec::new();

        for jidhr in &judhur_steam {
            judhur.extend(
                maktabat(jidhr, &mut mahmal).into_iter().map(|maktaba| maktaba.steamapps),
            );
            // The config directory carries the compatibility tool mapping, and
            // each account's directory carries its launch options and its
            // shortcuts — all three change without any manifest changing.
            judhur.push(jidhr.join("config"));
            judhur.extend(mustakhdimun(jidhr).into_iter().map(|mustakhdim| mustakhdim.tahyia()));
        }

        judhur.retain(|masar| masar.is_dir());
        judhur.sort();
        judhur.dedup();
        judhur
    }
}


#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a setup failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included, and a
    /// test that cannot report *why* its fixture failed to build is a test that
    /// wastes the next person's afternoon.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// Builds a Steam root holding one finished game in its own library.
    ///
    /// `StateFlags = 4` is [`hala::MUTHABBAT_BALKAMIL`] and nothing else, which
    /// is what [`muktamila`] requires. Anything less and the entry is correctly
    /// withheld, and the test would be measuring the wrong refusal.
    fn ansha_jidhr(jidhr: &Path, app: u32, mujallad: &str) -> Result<(), std::io::Error> {
        let steamapps = jidhr.join("steamapps");
        fs::create_dir_all(steamapps.join("common").join(mujallad))?;
        fs::write(
            steamapps.join(format!("appmanifest_{app}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app}\"\n\t\"name\"\t\t\"{mujallad}\"\n\t\
                 \"installdir\"\t\t\"{mujallad}\"\n\t\"StateFlags\"\t\t\"4\"\n}}\n"
            ),
        )
    }

    fn siyaq(manzil: &Path) -> SiyaqFahs {
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, manzil);
        siyaq.yashmal_hawiyat = true;
        siyaq
    }

    fn jidhr_hawiya(manzil: &Path) -> PathBuf {
        manzil.join(".var").join("app").join("com.valvesoftware.Steam").join("data").join("Steam")
    }

    #[test]
    fn ghiyab_steam_yueti_natija_farigha() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let natija = MatjarSteam.ifhas(&siyaq(manzil.path()))?;

        // Not installed is not a failure: `jidhr_matjar` is what tells the
        // library screen "this launcher is absent" rather than "it was scanned
        // and had nothing in it".
        assert!(natija.jidhr_matjar.is_none());
        assert!(natija.alaab.is_empty());
        assert!(natija.tanbihat.is_empty());
        Ok(())
    }

    #[test]
    fn jidhr_muhaddad_mafqud_khata_la_tarajju() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let mut siyaq = siyaq(manzil.path());
        siyaq.manassat.steam = Some(manzil.path().join("laysa-huna"));

        // The whole point of the override: a typo must not fall back to
        // automatic detection and be reported as an empty library.
        assert!(MatjarSteam.ifhas(&siyaq).is_err());
        Ok(())
    }

    #[test]
    fn jidhran_yumsahan_maan() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let asli = manzil.path().join(".local").join("share").join("Steam");
        let hawiya = jidhr_hawiya(manzil.path());
        ansha_jidhr(&asli, 220, "Half-Life 2")?;
        ansha_jidhr(&hawiya, 400, "Portal")?;

        let judhur = hall_judhur(&siyaq(manzil.path()))?;
        assert_eq!(judhur.len(), 2, "the native and the Flatpak root are both Steam roots");

        let natija = MatjarSteam.ifhas(&siyaq(manzil.path()))?;
        let mut asmaa: Vec<&str> = natija.alaab.iter().map(|luba| luba.ism.as_str()).collect();
        asmaa.sort_unstable();
        assert_eq!(asmaa, ["Half-Life 2", "Portal"]);
        assert_eq!(natija.jidhr_matjar.as_deref(), Some(asli.as_path()));
        Ok(())
    }

    #[test]
    fn maktaba_mushtaraka_bayn_jidhrayn_la_tudaaf_marratayn() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let asli = manzil.path().join(".local").join("share").join("Steam");
        let hawiya = jidhr_hawiya(manzil.path());
        ansha_jidhr(&asli, 220, "Half-Life 2")?;

        // A Flatpak Steam with no games of its own, pointed at the library the
        // native client owns. Both roots are real; the game behind them is one
        // game, and the native root reaches it first.
        fs::create_dir_all(hawiya.join("config"))?;
        fs::create_dir_all(hawiya.join("steamapps"))?;
        fs::write(
            hawiya.join("steamapps").join("libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                asli.display()
            ),
        )?;

        let natija = MatjarSteam.ifhas(&siyaq(manzil.path()))?;
        assert_eq!(natija.alaab.len(), 1, "one game, however many clients list its library");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn wasla_ila_nafs_al_jidhr_la_tuhsab_marratayn() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let asli = manzil.path().join(".local").join("share").join("Steam");
        ansha_jidhr(&asli, 220, "Half-Life 2")?;

        // `~/.steam/steam` is a symlink to `~/.local/share/Steam` on nearly
        // every Linux installation, and it is the first candidate tried. Both
        // are valid roots; they are one root, and `sawwi` is what says so.
        fs::create_dir_all(manzil.path().join(".steam"))?;
        std::os::unix::fs::symlink(&asli, manzil.path().join(".steam").join("steam"))?;

        assert_eq!(hall_judhur(&siyaq(manzil.path()))?.len(), 1);
        assert_eq!(MatjarSteam.ifhas(&siyaq(manzil.path()))?.alaab.len(), 1);
        Ok(())
    }

    /// A Windows context whose program directories are not on `C:`.
    ///
    /// `nizam` is a value rather than a compile-time fact, so the Windows
    /// branch of [`murashahat`] can be exercised from any host — and on a Unix
    /// host [`murashahat_sijill`] is empty by construction, which is precisely
    /// the state a Windows machine with missing or corrupt Steam registry
    /// values falls through from. That is the only state in which these
    /// candidates are ever read.
    fn siyaq_windows(manzil: &Path, baramij: &[PathBuf]) -> SiyaqFahs {
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, manzil);
        siyaq.mujalladat_baramij = baramij.to_vec();
        siyaq
    }

    #[test]
    fn murashahat_windows_min_al_siyaq_la_min_harf_qurs_thabit() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        let baramij = [
            PathBuf::from(r"D:\Program Files (x86)"),
            PathBuf::from(r"D:\Program Files"),
        ];
        let murashahat = murashahat(&siyaq_windows(manzil.path(), &baramij));
        let mutawaqqa: Vec<PathBuf> =
            baramij.iter().map(|mujallad| mujallad.join("Steam")).collect();

        // Everything past the registry's own answers, compared whole rather
        // than by suffix: a literal `C:\Program Files\Steam` left beside these
        // would still have satisfied a tail check.
        assert_eq!(
            murashahat.get(murashahat_sijill().len()..).unwrap_or(&[]),
            mutawaqqa.as_slice(),
            "every candidate the registry did not supply comes from the context"
        );
        Ok(())
    }

    #[test]
    fn murashahat_windows_bila_mujalladat_la_takhtari_shayan() -> NatijatIkhtibar {
        let manzil = tempfile::tempdir()?;
        assert_eq!(
            murashahat(&siyaq_windows(manzil.path(), &[])),
            murashahat_sijill(),
            "with no program directories there is nothing left to guess"
        );
        Ok(())
    }

    #[test]
    fn jidhr_windows_yuhall_min_mujallad_baramij_kharij_c() -> NatijatIkhtibar {
        let qurs = tempfile::tempdir()?;
        // The machine the hardcoded path was wrong on: Windows installed off
        // `C:`, and no registry to correct it.
        let baramij = qurs.path().join("Program Files (x86)");
        ansha_jidhr(&baramij.join("Steam"), 220, "Half-Life 2")?;

        let judhur = hall_judhur(&siyaq_windows(qurs.path(), std::slice::from_ref(&baramij)))?;
        assert!(
            judhur.contains(&baramij.join("Steam")),
            "the root under the context's program directory must be found: {judhur:?}"
        );
        Ok(())
    }
}
