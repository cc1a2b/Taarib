//! هيرويك — Heroic Games Launcher, and why its games do not belong to it.
//!
//! Heroic is not a store. It is a client for other people's stores: it installs
//! Epic titles through `legendary` and GOG titles through its own GOG
//! integration, and on Linux it runs them inside Wine or Proton prefixes it
//! manages itself.
//!
//! ## The single most important thing in this file
//!
//! **Every game found here is recorded as
//! `MasdarLuba::Heroic(Box::new(<the real store's source>))`** — never as a
//! Heroic identity of its own. An Epic title installed through Heroic is
//! `Heroic(Box::new(Epic("Fortnite-appname")))`; a GOG title is
//! `Heroic(Box::new(Gog(1207658691)))`.
//!
//! That is not decoration. [`MasdarLuba::aila`] — the function that decides
//! which registry shard a game's patches are looked up in — unwraps the box and
//! answers `"epic"` or `"gog"`, exactly as it would for a game found by the
//! Epic or GOG adapter directly. [`MasdarLuba::muarrif`] does the same, so the
//! deterministic `LubaId` derived from it is *identical* to the one the native
//! adapter would derive. The consequences are the whole point:
//!
//! - A patch published by a translator on Windows against `epic:<appname>` is
//!   offered, unchanged, to a Linux user who installed the same game through
//!   Heroic. Neither of them has to know the other's launcher exists.
//! - The same person who owns a game on Steam on their desktop and through
//!   Heroic on their Steam Deck gets one library entry, because
//!   [`Luba::yatba`] compares sources through `asl()`.
//! - Nothing has to publish a patch twice, and no shard called `heroic` exists
//!   in the registry at all.
//!
//! What the wrapper *does* preserve is how the game is installed, which is the
//! part that is genuinely Heroic's: its prefix, its Wine or Proton build, and
//! the fact that a Windows executable is sitting on a Linux filesystem. That
//! travels in [`LubaMuktashafa::beea`], not in the identity.
//!
//! ## What is read
//!
//! | file | what it carries |
//! | --- | --- |
//! | `legendaryConfig/legendary/installed.json` | every installed Epic title: app name, title, install path, version, executable, size |
//! | `store_cache/legendary_library.json` | Epic cover art, when Heroic has cached the library |
//! | `gog_store/installed.json` | every installed GOG title: product id, install path, build id, platform |
//! | `gog_store/library.json` | GOG titles and cover art |
//! | `GamesConfig/<app>.json` | that one game's Wine prefix and Wine or Proton build |
//!
//! Heroic's Amazon (`nile`) and sideloaded libraries are deliberately not read
//! here. A sideloaded entry has no store identity at all — it is a path the
//! user pointed Heroic at — and it is `matajir::yadawi` that owns paths the
//! user pointed at, with an identity derived the way that module derives it.
//! Reading them here would mint a second, conflicting identity for the same
//! files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier. It names the *adapter*, not the shard family: the
/// games it produces shard under `epic` and `gog`.
const MUARRIF: &str = "heroic";

/// Heroic runs on all three desktop systems, Linux first.
const MANASSAT: [NizamTashghil; 3] =
    [NizamTashghil::Linux, NizamTashghil::Windows, NizamTashghil::Mac];

/// The Flatpak application id.
const HAWIYAT_FLATPAK: &str = "com.heroicgameslauncher.hgl";

/// legendary's record of installed Epic titles, relative to the Heroic root.
const MASAR_LEGENDARY: [&str; 3] = ["legendaryConfig", "legendary", "installed.json"];

/// Heroic's cached Epic library, which is where Epic cover art lives.
const MASAR_MAKHZAN_EPIC: [&str; 2] = ["store_cache", "legendary_library.json"];

/// Heroic's record of installed GOG titles.
const MASAR_GOG_MUTHABBAT: [&str; 2] = ["gog_store", "installed.json"];

/// Heroic's GOG library metadata.
const MASAR_GOG_MAKTABA: [&str; 2] = ["gog_store", "library.json"];

/// Per-game configuration, one file per app name.
const MUJALLAD_IDADAT: &str = "GamesConfig";

/// Heroic Games Launcher.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarHeroic;

impl MatjarHeroic {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Every configuration root worth looking in, most likely first.
    ///
    /// The Flatpak layout is checked on Linux whenever the scan is allowed to
    /// look inside containers, because Heroic's Flatpak build is how a large
    /// share of Linux users have it and its configuration is nowhere near
    /// `~/.config`.
    fn judhur_muhtamala(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur = Vec::new();
        match siyaq.nizam {
            NizamTashghil::Linux => {
                judhur.push(siyaq.manzil.join(".config").join("heroic"));
                if siyaq.yashmal_hawiyat {
                    judhur.push(
                        siyaq
                            .manzil
                            .join(".var")
                            .join("app")
                            .join(HAWIYAT_FLATPAK)
                            .join("config")
                            .join("heroic"),
                    );
                }
            },
            NizamTashghil::Windows => {
                judhur.extend(
                    siyaq.bayanat_mutajawwila.as_ref().map(|bayanat| bayanat.join("heroic")),
                );
            },
            NizamTashghil::Mac => {
                judhur
                    .push(siyaq.manzil.join("Library").join("Application Support").join("heroic"));
            },
        }
        judhur
    }

    /// Whether a directory looks like a Heroic configuration root.
    fn huwa_jidhr(jidhr: &Path) -> bool {
        jidhr.is_dir()
            && (jidhr.join("config.json").is_file()
                || dam(jidhr, &MASAR_LEGENDARY).is_file()
                || dam(jidhr, &MASAR_GOG_MUTHABBAT).is_file()
                || jidhr.join(MUJALLAD_IDADAT).is_dir())
    }
}

impl Matjar for MatjarHeroic {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "هيرويك"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Heroic Games Launcher"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if let Some(tajawuz) = siyaq.manassat.heroic.as_ref() {
            return Some(tajawuz.clone());
        }
        Self::judhur_muhtamala(siyaq).into_iter().find(|jidhr| Self::huwa_jidhr(jidhr))
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when the user configured a
    /// Heroic root that is not there. Nothing else fails the scan: a library
    /// file that will not parse costs that library, a game whose install
    /// directory has been deleted costs that game, and a prefix that is missing
    /// costs only the compatibility detail — the game is still listed.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        if let Some(tajawuz) = siyaq.manassat.heroic.as_ref()
            && !tajawuz.is_dir()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: tajawuz.clone(),
            }
            .into());
        }

        let Some(jidhr) = self.mawqi(siyaq) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: Some(jidhr.clone()),
            ..NatijatMatjar::default()
        };

        let idadat = idadat_al_alaab(&jidhr);
        jama_epic(&jidhr, &idadat, siyaq.nizam, &mut natija);
        jama_gog(&jidhr, &idadat, siyaq.nizam, &mut natija);

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let Some(jidhr) = self.mawqi(siyaq) else {
            return Vec::new();
        };
        [
            jidhr.join("gog_store"),
            jidhr.join("legendaryConfig").join("legendary"),
            jidhr.join(MUJALLAD_IDADAT),
        ]
        .into_iter()
        .filter(|masar| masar.is_dir())
        .collect()
    }
}

/// Joins a fixed relative path onto a root.
///
/// The segments are module constants, never anything read from a file, so this
/// is an ordinary join rather than a containment check.
fn dam(jidhr: &Path, ajzaa: &[&str]) -> PathBuf {
    ajzaa.iter().fold(jidhr.to_path_buf(), |masar, juz| masar.join(juz))
}

// ---------------------------------------------------------------------------
// per-game configuration and the compatibility environment
// ---------------------------------------------------------------------------

/// One game's Heroic configuration, reduced to what decides how it runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct IdadLuba {
    /// `winePrefix` — the directory containing `drive_c`.
    beea: Option<PathBuf>,
    /// `wineVersion.name` — the build's display name, as Heroic writes it:
    /// `Proton - GE-Proton9-20`, `Wine - lutris-fshack-7.2`, `System Wine`.
    isdar_wine: Option<String>,
    /// `wineVersion.type` — `wine`, `proton`, `crossover` or `toolkit`.
    naw_wine: Option<String>,
    /// `targetExe` — an executable the user chose instead of the store's.
    tanfidhi_badil: Option<String>,
    /// The launch arguments the user already set, so Phase 15 extends them
    /// rather than replacing them.
    khiyarat: Option<String>,
}

impl IdadLuba {
    /// Fills in from Heroic's global defaults whatever this game did not set.
    fn bi_asas(mut self, asas: &Self) -> Self {
        self.beea = self.beea.or_else(|| asas.beea.clone());
        self.isdar_wine = self.isdar_wine.or_else(|| asas.isdar_wine.clone());
        self.naw_wine = self.naw_wine.or_else(|| asas.naw_wine.clone());
        self
    }

    /// Reads one configuration object.
    fn min_qeema(qeema: &Value) -> Self {
        let build = qeema.get("wineVersion");
        Self {
            beea: nass_haql(qeema, "winePrefix")
                .or_else(|| nass_haql(qeema, "wineCrossoverBottle"))
                .map(PathBuf::from),
            isdar_wine: build.and_then(|build| nass_haql(build, "name")),
            naw_wine: build.and_then(|build| nass_haql(build, "type")),
            tanfidhi_badil: nass_haql(qeema, "targetExe"),
            khiyarat: nass_haql(qeema, "launcherArgs")
                .or_else(|| nass_haql(qeema, "otherOptions")),
        }
    }

    /// Whether this configuration says anything at all.
    const fn faragh(&self) -> bool {
        self.beea.is_none() && self.isdar_wine.is_none() && self.naw_wine.is_none()
    }
}

/// Every per-game configuration under `GamesConfig`, keyed by app name.
///
/// Each file is `{"<app name>": { … }}` with a couple of bookkeeping keys
/// alongside, so the entry is matched by file name first and by shape second —
/// a Heroic version that renames `version` or adds another sibling key must not
/// turn a bookkeeping value into a game's Wine settings.
fn idadat_al_alaab(jidhr: &Path) -> BTreeMap<String, IdadLuba> {
    let asas = idadat_amma(jidhr);
    let mut idadat = BTreeMap::new();
    let Ok(qaima) = std::fs::read_dir(jidhr.join(MUJALLAD_IDADAT)) else {
        return idadat;
    };

    for madkhal in qaima.flatten() {
        let masar = madkhal.path();
        if masar.extension().is_none_or(|imtidad| !imtidad.eq_ignore_ascii_case("json")) {
            continue;
        }
        let Some(ism) = masar.file_stem().map(|ism| ism.to_string_lossy().into_owned()) else {
            continue;
        };
        let Some(qeema) = iqra_json(&masar) else {
            continue;
        };
        let Some(kaain) = qeema.as_object() else {
            continue;
        };

        let mukhtar = kaain
            .get(&ism)
            .filter(|dakhili| dakhili.is_object())
            .or_else(|| {
                kaain.values().find(|dakhili| {
                    dakhili.is_object()
                        && (dakhili.get("winePrefix").is_some()
                            || dakhili.get("wineVersion").is_some())
                })
            })
            .map(IdadLuba::min_qeema);

        if let Some(idad) = mukhtar {
            let _ = idadat.insert(ism, idad.bi_asas(&asas));
        }
    }
    idadat
}

/// Heroic's own defaults, which every game inherits until it overrides them.
fn idadat_amma(jidhr: &Path) -> IdadLuba {
    let Some(qeema) = iqra_json(&jidhr.join("config.json")) else {
        return IdadLuba::default();
    };
    qeema
        .get("defaultSettings")
        .map(IdadLuba::min_qeema)
        .filter(|idad| !idad.faragh())
        .unwrap_or_default()
}

/// Resolves the compatibility layer one game runs behind.
///
/// A game whose declared platform is native to the running system runs
/// natively, whatever prefix Heroic keeps beside it. Everything else is a
/// Windows program, and the prefix is what makes it runnable — so the prefix is
/// resolved through `beea`, which is the one place in this crate that knows how
/// to read `user.reg`, `system.reg` and the `dosdevices` drive map. This
/// adapter calls it and never reimplements it.
fn beea_luba(
    idad: &IdadLuba,
    manassat_luba: Option<&str>,
    nizam: NizamTashghil,
    ism: &str,
    tanbihat: &mut Vec<TanbihFahs>,
) -> BeeatTawafuq {
    if asliya(manassat_luba, nizam) {
        return BeeatTawafuq::Asli;
    }

    let Some(beea) = idad.beea.clone() else {
        if nizam != NizamTashghil::Windows {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                ism.to_owned(),
                "Heroic has this Windows game installed but records no Wine prefix for it, so \
                 Taarib cannot tell where the game's own C: drive is. Launch it once from Heroic \
                 so the prefix is created, then rescan.",
            ));
        }
        return BeeatTawafuq::Asli;
    };

    // `hal_beea` is where prefix knowledge lives. A failure here is not fatal:
    // the prefix path Heroic recorded is still the right answer for Phase 15,
    // and the missing part is only the drive map and the detected Wine build.
    let maktashaf = match crate::beea::hal_beea(&beea) {
        Ok(maalumat) => maalumat.isdar_wine,
        Err(khata) => {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                beea.display().to_string(),
                format!("{ism}: {}", khata.injilizi),
            ));
            None
        },
    };

    let isdar = idad.isdar_wine.clone().or(maktashaf);
    if idad.naw_wine.as_deref().is_some_and(|naw| naw.eq_ignore_ascii_case("proton")) {
        BeeatTawafuq::Proton {
            isdar: isdar.unwrap_or_else(|| "Proton".to_owned()),
            beea,
        }
    } else {
        BeeatTawafuq::Wine { isdar, beea }
    }
}

/// Whether a store's platform string names this machine's own platform.
///
/// Epic writes `Windows`, `Mac` and `Linux`; GOG writes `windows`, `osx` and
/// `linux`. Both spellings are accepted, and an absent platform is treated as
/// Windows, because that is what both stores default to and what every Heroic
/// prefix exists for.
fn asliya(manassat_luba: Option<&str>, nizam: NizamTashghil) -> bool {
    let muallan = manassat_luba.unwrap_or("windows").to_ascii_lowercase();
    match nizam {
        NizamTashghil::Windows => matches!(muallan.as_str(), "windows" | "win32" | "win64" | "pc"),
        NizamTashghil::Linux => muallan == "linux",
        NizamTashghil::Mac => matches!(muallan.as_str(), "mac" | "osx" | "macos" | "darwin"),
    }
}

// ---------------------------------------------------------------------------
// small JSON readers
// ---------------------------------------------------------------------------

/// Reads and parses a JSON file, or nothing.
///
/// Nothing here distinguishes "absent" from "malformed", because the callers
/// treat them identically: both mean this library contributed no games, and
/// both are reported by the caller with the path it tried.
fn iqra_json(masar: &Path) -> Option<Value> {
    let bayt = std::fs::read(masar).ok()?;
    let nass = String::from_utf8_lossy(&bayt);
    serde_json::from_str(nass.trim_start_matches('\u{feff}')).ok()
}

/// A string field, trimmed, or nothing when it is absent or empty.
fn nass_haql(qeema: &Value, miftah: &str) -> Option<String> {
    let nass = qeema.get(miftah)?.as_str()?.trim();
    (!nass.is_empty()).then(|| nass.to_owned())
}

/// A whole-number field, however the writer chose to encode it.
///
/// Heroic writes install sizes as numbers in one library and as decimal
/// strings in another, so both are accepted rather than one being declared
/// correct.
fn raqm_haql(qeema: &Value, miftah: &str) -> Option<u64> {
    let haql = qeema.get(miftah)?;
    haql.as_u64().or_else(|| haql.as_str()?.trim().parse().ok())
}

/// A boolean field, or nothing.
fn sawab_haql(qeema: &Value, miftah: &str) -> Option<bool> {
    qeema.get(miftah)?.as_bool()
}

// ---------------------------------------------------------------------------
// one installed entry, whichever store it came from
// ---------------------------------------------------------------------------

/// One installed title, after the store-specific record has been read and
/// before the Heroic wrapper goes on.
///
/// The two libraries agree on nothing — different key names, different types,
/// different platform spellings — so each is read on its own terms and reduced
/// to this, and the identity rule is applied in exactly one place below.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MadkhalMuthabbat {
    /// The **store's own** identity. Never a Heroic identity.
    masdar_asli: MasdarLuba,
    /// The key the per-game configuration file is named after.
    muarrif_idad: String,
    /// The title, as the store gives it.
    ism: String,
    /// The install root.
    jidhr: PathBuf,
    /// The executable the store names, relative to the root.
    tanfidhi_nisbi: Option<String>,
    /// Size on disk in bytes, zero when the store does not say.
    hajm: u64,
    /// The store's build or version identifier.
    isdar: Option<String>,
    /// The platform the installed build is for.
    manassa: Option<String>,
    /// Whether the install is finished and verified.
    muktamila: bool,
    /// Cover art, from Heroic's cached library metadata.
    suwar: MasadirSuwar,
}

/// Applies the rule this whole module exists for.
///
/// The store's source goes inside the box; `MasdarLuba::Heroic` goes around it.
/// Nothing downstream — not the registry lookup, not the identity derivation,
/// not the duplicate merge — sees a Heroic identity, because
/// [`MasdarLuba::aila`] and [`MasdarLuba::muarrif`] both unwrap it. The only
/// thing that survives as Heroic's is how the game runs, which is what
/// [`beea_luba`] fills in.
fn luba_min_madkhal(
    madkhal: MadkhalMuthabbat,
    idadat: &BTreeMap<String, IdadLuba>,
    nizam: NizamTashghil,
    tanbihat: &mut Vec<TanbihFahs>,
) -> LubaMuktashafa {
    let idad = idadat.get(&madkhal.muarrif_idad).cloned().unwrap_or_default();
    let beea =
        beea_luba(&idad, madkhal.manassa.as_deref(), nizam, &madkhal.ism, tanbihat);

    let mut simat = Vec::new();
    match &beea {
        BeeatTawafuq::Proton { isdar, .. } => {
            simat.push(SimatLuba::TabaqatTawafuq(format!("Proton, {isdar}")));
        },
        BeeatTawafuq::Wine { isdar, .. } => {
            simat.push(SimatLuba::TabaqatTawafuq(
                isdar.clone().unwrap_or_else(|| "Wine".to_owned()),
            ));
        },
        BeeatTawafuq::Asli | BeeatTawafuq::Rosetta => {},
    }

    let tanfidhi = idad
        .tanfidhi_badil
        .clone()
        .or_else(|| madkhal.tanfidhi_nisbi.clone())
        .and_then(|nisbi| tanfidhi_dakhil(&madkhal.jidhr, &nisbi));

    LubaMuktashafa {
        masdar: MasdarLuba::Heroic(Box::new(madkhal.masdar_asli)),
        hala_matjar: None,
        ism: madkhal.ism,
        jidhr: madkhal.jidhr,
        tanfidhi,
        hajm: madkhal.hajm,
        bina_manassa: madkhal.isdar,
        akhir_tahdith: None,
        akhir_laab: None,
        beea,
        suwar: madkhal.suwar,
        khiyarat_tashghil: idad.khiyarat,
        muktamila: madkhal.muktamila,
        simat,
    }
}

/// Resolves a store-declared executable inside an install root.
///
/// The value comes out of a launcher's JSON, so it goes through the containment
/// check rather than being joined: a relative path with `..` in it would
/// otherwise let a tampered library file point Taarib's probe at a file outside
/// the game. An absolute path is accepted only when it is already inside the
/// root.
fn tanfidhi_dakhil(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munaqqa = nisbi.trim().replace('\\', std::path::MAIN_SEPARATOR_STR);
    if munaqqa.is_empty() {
        return None;
    }
    let murashah = Path::new(&munaqqa);
    let kamil = if murashah.is_absolute() {
        murashah.starts_with(jidhr).then(|| murashah.to_path_buf())?
    } else {
        dakhil(jidhr, &munaqqa).ok()?
    };
    kamil.is_file().then_some(kamil)
}

// ---------------------------------------------------------------------------
// Epic, through legendary
// ---------------------------------------------------------------------------

/// Reads every Epic title legendary has installed.
fn jama_epic(
    jidhr: &Path,
    idadat: &BTreeMap<String, IdadLuba>,
    nizam: NizamTashghil,
    natija: &mut NatijatMatjar,
) {
    let masar = dam(jidhr, &MASAR_LEGENDARY);
    if !masar.is_file() {
        // Heroic with no Epic account is ordinary, not a fault.
        return;
    }
    let Some(qeema) = iqra_json(&masar) else {
        natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            "legendary's installed-games file could not be read as JSON, so no Epic title \
             installed through Heroic can be listed",
        ));
        return;
    };
    let Some(madakhil) = qeema.as_object() else {
        natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            "legendary's installed-games file is not the object of app names this reader \
             expects, so its layout has changed",
        ));
        return;
    };

    let suwar = suwar_epic(jidhr);
    for (miftah, sijill) in madakhil {
        if sawab_haql(sijill, "is_dlc").unwrap_or(false) {
            continue;
        }
        let ism_tatbeeq = nass_haql(sijill, "app_name").unwrap_or_else(|| miftah.clone());
        let Some(jidhr_luba) = nass_haql(sijill, "install_path").map(PathBuf::from) else {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                format!("legendary: {ism_tatbeeq}"),
                "this Epic title records no install path, so there is nothing to probe",
            ));
            continue;
        };
        if !jidhr_luba.is_dir() {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                jidhr_luba.display().to_string(),
                format!(
                    "Heroic lists {ism_tatbeeq} as installed here and the folder is gone; it was \
                     deleted outside Heroic, or lives on a drive that is not mounted"
                ),
            ));
            continue;
        }

        let luba = luba_min_madkhal(
            MadkhalMuthabbat {
                masdar_asli: MasdarLuba::Epic(ism_tatbeeq.clone()),
                ism: nass_haql(sijill, "title").unwrap_or_else(|| ism_tatbeeq.clone()),
                jidhr: jidhr_luba,
                tanfidhi_nisbi: nass_haql(sijill, "executable"),
                hajm: raqm_haql(sijill, "install_size").unwrap_or(0),
                isdar: nass_haql(sijill, "version"),
                manassa: nass_haql(sijill, "platform"),
                // legendary sets this flag when a download was interrupted or
                // a repair is outstanding, which is exactly the state in which
                // a patch must not be offered: the launcher is about to
                // overwrite the files.
                muktamila: !sawab_haql(sijill, "needs_verification").unwrap_or(false),
                suwar: suwar.get(&ism_tatbeeq).cloned().unwrap_or_default(),
                muarrif_idad: ism_tatbeeq,
            },
            idadat,
            nizam,
            &mut natija.tanbihat,
        );
        natija.alaab.push(luba);
    }
}

/// Epic cover art out of Heroic's cached library, keyed by app name.
fn suwar_epic(jidhr: &Path) -> BTreeMap<String, MasadirSuwar> {
    let Some(qeema) = iqra_json(&dam(jidhr, &MASAR_MAKHZAN_EPIC)) else {
        return BTreeMap::new();
    };
    qaimat_alaab(&qeema)
        .iter()
        .filter_map(|sijill| {
            let ism = nass_haql(sijill, "app_name")?;
            Some((ism, suwar_min_sijill(sijill)))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// GOG, through Heroic's own integration
// ---------------------------------------------------------------------------

/// Reads every GOG title Heroic has installed.
fn jama_gog(
    jidhr: &Path,
    idadat: &BTreeMap<String, IdadLuba>,
    nizam: NizamTashghil,
    natija: &mut NatijatMatjar,
) {
    let masar = dam(jidhr, &MASAR_GOG_MUTHABBAT);
    if !masar.is_file() {
        return;
    }
    let Some(qeema) = iqra_json(&masar) else {
        natija.tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            "Heroic's GOG installed-games file could not be read as JSON, so no GOG title \
             installed through Heroic can be listed",
        ));
        return;
    };

    let madakhil = qeema
        .get("installed")
        .and_then(Value::as_array)
        .or_else(|| qeema.as_array())
        .cloned()
        .unwrap_or_default();
    if madakhil.is_empty() {
        return;
    }

    let maktaba = maktabat_gog(jidhr);
    for sijill in &madakhil {
        if sawab_haql(sijill, "is_dlc").unwrap_or(false) {
            continue;
        }
        let Some(ism_tatbeeq) =
            nass_haql(sijill, "appName").or_else(|| nass_haql(sijill, "app_name"))
        else {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar.display().to_string(),
                "a GOG installation record names no product, so it cannot be identified",
            ));
            continue;
        };

        // GOG product identifiers are numeric, and the vocabulary stores them
        // as a number. A record whose product id is not one is not something to
        // guess at: it would produce an identity no patch could ever match.
        let Ok(muarrif_gog) = ism_tatbeeq.trim().parse::<u64>() else {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                format!("gog: {ism_tatbeeq}"),
                "this GOG record's product identifier is not a number, so Taarib cannot derive \
                 the identity the patch registry shards GOG titles by",
            ));
            continue;
        };

        let Some(jidhr_luba) = nass_haql(sijill, "install_path").map(PathBuf::from) else {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                format!("gog: {ism_tatbeeq}"),
                "this GOG title records no install path, so there is nothing to probe",
            ));
            continue;
        };
        if !jidhr_luba.is_dir() {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                jidhr_luba.display().to_string(),
                format!(
                    "Heroic lists GOG product {ism_tatbeeq} as installed here and the folder is \
                     gone; it was deleted outside Heroic, or lives on a drive that is not mounted"
                ),
            ));
            continue;
        }

        let bitaqa = maktaba.get(&ism_tatbeeq);
        let luba = luba_min_madkhal(
            MadkhalMuthabbat {
                masdar_asli: MasdarLuba::Gog(muarrif_gog),
                ism: bitaqa
                    .and_then(|bitaqa| bitaqa.ism.clone())
                    .or_else(|| nass_haql(sijill, "title"))
                    .unwrap_or_else(|| format!("GOG {ism_tatbeeq}")),
                jidhr: jidhr_luba,
                tanfidhi_nisbi: nass_haql(sijill, "executable"),
                hajm: raqm_haql(sijill, "install_size").unwrap_or(0),
                // `buildId` is the identifier GOG's own patch metadata keys on;
                // `versionName` is the human string beside it.
                isdar: nass_haql(sijill, "buildId")
                    .or_else(|| nass_haql(sijill, "versionName"))
                    .or_else(|| nass_haql(sijill, "version")),
                manassa: nass_haql(sijill, "platform"),
                muktamila: true,
                suwar: bitaqa.map(|bitaqa| bitaqa.suwar.clone()).unwrap_or_default(),
                muarrif_idad: ism_tatbeeq,
            },
            idadat,
            nizam,
            &mut natija.tanbihat,
        );
        natija.alaab.push(luba);
    }
}

/// One title's metadata out of Heroic's GOG library file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BitaqatMaktaba {
    /// The title as GOG gives it.
    ism: Option<String>,
    /// Its artwork.
    suwar: MasadirSuwar,
}

/// Heroic's GOG library metadata, keyed by product identifier.
fn maktabat_gog(jidhr: &Path) -> BTreeMap<String, BitaqatMaktaba> {
    let Some(qeema) = iqra_json(&dam(jidhr, &MASAR_GOG_MAKTABA)) else {
        return BTreeMap::new();
    };
    qaimat_alaab(&qeema)
        .iter()
        .filter_map(|sijill| {
            let ism = nass_haql(sijill, "app_name").or_else(|| nass_haql(sijill, "appName"))?;
            Some((
                ism,
                BitaqatMaktaba {
                    ism: nass_haql(sijill, "title"),
                    suwar: suwar_min_sijill(sijill),
                },
            ))
        })
        .collect()
}

/// The array of titles inside one of Heroic's library files.
///
/// The wrapping key has been `games`, then `library`, and the file has also
/// been a bare array. All three are accepted rather than one being required,
/// because the only cost of accepting all three is this function.
fn qaimat_alaab(qeema: &Value) -> Vec<Value> {
    for miftah in ["games", "library"] {
        if let Some(qaima) = qeema.get(miftah).and_then(Value::as_array) {
            return qaima.clone();
        }
    }
    qeema.as_array().cloned().unwrap_or_default()
}

/// The three artwork addresses a Heroic library record carries.
///
/// They are URLs, not files: Heroic caches the images inside its own Electron
/// storage rather than as loose files a second program can find, so the address
/// is what is passed on and `suwar` fetches it once, later, off the path the
/// library screen waits on.
fn suwar_min_sijill(sijill: &Value) -> MasadirSuwar {
    let rabt = |miftah: &str| nass_haql(sijill, miftah).map(MasdarSura::Rabt);
    MasadirSuwar {
        // `art_square` is the vertical box art the grid wants; `art_cover` is
        // the wide image behind the detail header.
        ghilaf: rabt("art_square").or_else(|| rabt("art_cover")),
        batl: rabt("art_cover").or_else(|| rabt("art_background")),
        shiar: rabt("art_logo"),
    }
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    #[test]
    fn jidhr_windows_min_al_bayanat_al_mutajawwila_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let mutajawwila = masrah.path().join("Roaming");
        fs::create_dir_all(mutajawwila.join("heroic").join(MUJALLAD_IDADAT))?;

        // `%APPDATA%` is never set here — it cannot be, since edition 2024 — so
        // the candidate can only have come from the context.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        siyaq.bayanat_mutajawwila = Some(mutajawwila.clone());
        assert_eq!(
            MatjarHeroic::jadeed().mawqi(&siyaq),
            Some(mutajawwila.join("heroic")),
            "the configuration root must come from the context"
        );
        Ok(())
    }

    #[test]
    fn bila_bayanat_mutajawwila_la_murashah_ala_windows() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // The old resolver invented `<home>/AppData/Roaming/heroic` on a machine
        // with no such layout; there is now simply nothing to probe.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Windows, masrah.path());
        assert!(MatjarHeroic::judhur_muhtamala(&siyaq).is_empty());
        Ok(())
    }
}
