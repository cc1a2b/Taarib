//! ليجندري — legendary, the command-line Epic client, whose games are Epic's.
//!
//! legendary is what a large share of Linux users actually have instead of the
//! Epic Games Launcher, which has no Linux build at all. It is a Python program
//! that talks to Epic's own API, downloads the **Windows** build of a game, and
//! leaves the user to run it under Wine. Heroic bundles it and drives it; a
//! great many people run it directly.
//!
//! ## The single most important thing in this file
//!
//! **Every game found here is recorded as
//! `MasdarLuba::Legendary(Box::new(MasdarLuba::Epic(app_name)))`** — never as a
//! legendary identity of its own.
//!
//! [`MasdarLuba::aila`] unwraps the box and answers `"epic"`;
//! [`MasdarLuba::muarrif`] answers `epic:<app_name>`. The deterministic
//! [`LubaId`](taarib_mustalahat::luba::LubaId) derived from it is therefore
//! *identical* to the one the Epic adapter derives on Windows, and the
//! consequences are the entire reason the wrapper exists:
//!
//! - A patch published by a translator on Windows against `epic:Fortnite` is
//!   offered, unchanged, to a Linux user who installed the same game with
//!   `legendary install`. Neither of them has to know the other's client exists.
//! - The same person who has a game through Heroic on one machine and through
//!   bare legendary on another gets one library entry, because
//!   [`Luba::yatba`](taarib_mustalahat::luba::Luba::yatba) compares sources
//!   through `asl()` and both unwrap to the same Epic identity.
//! - No shard called `legendary` exists in the registry, and nothing has to
//!   publish a patch twice.
//!
//! What the wrapper *does* preserve is that the game is installed by legendary
//! rather than by Epic's own client — which is what decides how it is launched
//! and which prefix it runs in. That travels in
//! [`LubaMuktashafa::beea`](crate::fahs::LubaMuktashafa::beea), not in the
//! identity.
//!
//! ## What is read
//!
//! | file | what it carries |
//! | --- | --- |
//! | `installed.json` | a map of app name to the install record: title, version, install path, executable, size, platform |
//! | `config.ini` | legendary's own settings, including the Wine prefix, per game and by default |
//! | `metadata/<app_name>.json` | Epic's catalogue entry: the artwork in `keyImages`, and the categories that say whether it is a game at all |
//!
//! `user.json` holds the account's refresh token and is **never opened**. There
//! is nothing in it discovery needs, and a scanner that reads somebody's
//! credentials is a scanner that has to be trusted rather than merely used.
//!
//! ## Where legendary keeps that, and where this adapter refuses to look
//!
//! | location | when |
//! | --- | --- |
//! | `$LEGENDARY_CONFIG_PATH` | whenever it is set to an absolute path — it is legendary's own override and it wins |
//! | `$XDG_CONFIG_HOME/legendary` | on Linux, for the releases that honour the base-directory specification |
//! | `~/.config/legendary` | on every platform, including Windows and macOS, because legendary hardcodes it |
//! | `~/.var/app/com.heroicgameslauncher.hgl/config/legendary` | the copy Heroic's Flatpak drives, which is a real and common installation |
//!
//! A candidate that lies under a `legendaryConfig` directory is **skipped**.
//! That path is Heroic's own bundled copy and [`crate::matajir::heroic`] owns
//! it: reading it here as well would produce a second record of every Epic game
//! Heroic manages, and while the two identities unwrap to the same Epic source
//! and would therefore merge, the duplicate diagnostics and the duplicated work
//! are pure cost.
//!
//! ## `platform` is `Windows`, and the prefix is somewhere else
//!
//! Every install record on a Linux machine says `"platform": "Windows"`, because
//! that is what legendary downloaded — Epic ships no Linux builds for anything
//! that matters, and legendary's job ends at putting the Windows files on disk.
//! The prefix is not legendary's business at all: it is a setting the user
//! writes into `config.ini`, or one the wrapper driving legendary supplies in
//! the environment.
//!
//! So the prefix is looked for in five places, in this order, each of which is a
//! more specific statement about *this* game than the one after it:
//!
//! 1. `[<app_name>.env] WINEPREFIX` — this game, set as an environment override.
//! 2. `[<app_name>] wine_prefix` — this game, set as a legendary option.
//! 3. Heroic's `GamesConfig/<app_name>.json` → `winePrefix`, when the legendary
//!    root is one Heroic drives, then Heroic's `config.json` default.
//! 4. `[default.env] WINEPREFIX`, then `[default] wine_prefix` — every game.
//! 5. `$LEGENDARY_WINE_PREFIX`, then `$WINEPREFIX` in this process's own
//!    environment, which is a deliberate statement by a user running Taarib from
//!    a shell that already has one set.
//!
//! **When none of them answer, the compatibility layer is unknown — not
//! absent.** [`BeeatTawafuq`] has no variant for that, and the two ways of
//! encoding it are not equally honest. Reporting a prefix path that does not
//! exist is the worse one: every path derived from it would resolve to
//! *something*, the installer would write files, the manifest would record them,
//! and the game would launch and find nothing — a silent failure with no error
//! anywhere to explain it. So the environment is reported as
//! [`BeeatTawafuq::Asli`], and the fact that a compatibility layer is involved
//! and could not be identified is carried explicitly in a
//! [`SimatLuba::TabaqatTawafuq`] and in a warning naming the remedy. That is the
//! same encoding [`crate::matajir::heroic`] uses for the same situation, and
//! keeping the two the same is deliberate: the interface should not have to know
//! which client produced the entry.
//!
//! ## Artwork is extracted now so Phase 20B does not have to re-read anything
//!
//! Epic's catalogue entry carries a `keyImages` array, and the three shapes
//! Taarib wants are in it under fixed type names: `DieselGameBoxTall` is the
//! vertical cover the library grid shows, `DieselGameBox` is the horizontal
//! banner behind the detail header, and `DieselGameBoxLogo` is the overlaid
//! logo. They are **URLs**, not files — legendary caches no images at all, and
//! Heroic caches its own inside Electron storage that nothing else can address —
//! so they travel as [`MasdarSura::Rabt`] and `suwar` fetches them later, off the
//! path the library screen waits on. Pulling them out during discovery means the
//! metadata directory is read once per scan rather than once per game per phase.

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

/// The stable identifier. It names the *adapter*; the games it produces shard
/// under `epic`, because that is what they are.
const MUARRIF: &str = "legendary";

/// legendary is a Python program and runs wherever Python does. Its Linux use is
/// what motivates this adapter, but a macOS user running it under `CrossOver`
/// and a Windows user who prefers a command line both exist.
const MANASSAT: [NizamTashghil; 3] =
    [NizamTashghil::Linux, NizamTashghil::Windows, NizamTashghil::Mac];

/// legendary's own override of where its configuration lives.
const MUTAGHAYYIR_JIDHR: &str = "LEGENDARY_CONFIG_PATH";

/// The directory legendary uses under a configuration root.
const MUJALLAD_LEGENDARY: &str = "legendary";

/// Heroic's Flatpak application id, whose bundled legendary is a real location.
const HAWIYAT_HEROIC: &str = "com.heroicgameslauncher.hgl";

/// The directory name that marks Heroic's *own* bundled legendary, which
/// [`crate::matajir::heroic`] owns and this adapter steps around.
const MUJALLAD_HEROIC_LEGENDARY: &str = "legendaryConfig";

/// legendary's record of what it has installed.
const ISM_MUTHABBAT: &str = "installed.json";

/// legendary's settings file.
const ISM_IDADAT: &str = "config.ini";

/// Where legendary caches Epic's catalogue entry for each owned title.
const MUJALLAD_BAYANAT: &str = "metadata";

/// The `config.ini` section that applies to every game.
const QISM_IFTIRADI: &str = "default";

/// How many metadata files are read in one scan.
///
/// This is one file per *owned* title, not per installed one, and a long-standing
/// Epic account that has claimed every weekly giveaway runs to several hundred.
/// Four thousand is far past that and bounds the cost against a directory
/// somebody else filled.
const HADD_BAYANAT: usize = 4096;

/// The `keyImages` type that is the vertical cover, most preferred first.
const ANWA_GHILAF: [&str; 3] = ["DieselGameBoxTall", "DieselStoreFrontTall", "OfferImageTall"];

/// The `keyImages` type that is the wide banner, most preferred first.
const ANWA_BATL: [&str; 4] =
    ["DieselGameBox", "DieselGameBoxWide", "DieselStoreFrontWide", "OfferImageWide"];

/// The `keyImages` type that is the overlaid logo, most preferred first.
///
/// `Thumbnail` is the fallback and is not really a logo — it is the small square
/// Epic shows in a list. It is accepted because a logo is optional in Epic's own
/// catalogue and most titles have none, and a small square image is a better
/// answer for the detail header than nothing at all.
const ANWA_SHIAR: [&str; 3] = ["DieselGameBoxLogo", "DieselGameBoxLogoWide", "Thumbnail"];

/// legendary, the command-line Epic client.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarLegendary;

impl MatjarLegendary {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }

    /// Whether a directory looks like a legendary configuration root.
    ///
    /// Either file is enough. A user who has authenticated and installed nothing
    /// has a `config.ini` and no `installed.json`; a user who installed a game
    /// and never changed a setting has the reverse. Requiring both would report
    /// legendary as absent for both of them.
    fn huwa_jidhr(jidhr: &Path) -> bool {
        jidhr.is_dir()
            && (jidhr.join(ISM_MUTHABBAT).is_file() || jidhr.join(ISM_IDADAT).is_file())
    }
}

impl Matjar for MatjarLegendary {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "ليجندري"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Legendary"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        judhur_muhtamala(siyaq).into_iter().find(|jidhr| Self::huwa_jidhr(jidhr))
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::TarwisatFahrasTalifa`] when `installed.json` exists
    /// and is not the object of app names legendary writes — a truncated file, a
    /// file replaced by something else, or a layout change this reader does not
    /// know. That single file is the whole catalogue, so there is nothing else
    /// to fall back to and nothing narrower to degrade.
    ///
    /// Everything else degrades one entry: a game whose install directory has
    /// been deleted, a game with no prefix recorded, a metadata file that will
    /// not parse. Each is a [`TanbihFahs`] on the result and the other games
    /// still arrive. A configuration root with a `config.ini` and no
    /// `installed.json` is not a failure at all — it is an authenticated
    /// legendary with nothing installed — and returns an empty result.
    ///
    /// There is no user override for legendary in `IdadatManassat`, so
    /// [`KhataKashf::JidhrMuhaddadMafqud`] cannot arise here; when one is added,
    /// this is where it belongs.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let Some(jidhr) = self.mawqi(siyaq) else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: Some(jidhr.clone()),
            ..NatijatMatjar::default()
        };

        let masar_muthabbat = jidhr.join(ISM_MUTHABBAT);
        if !masar_muthabbat.is_file() {
            // Authenticated, nothing installed. Not a fault, and reporting the
            // root is what tells the diagnostics screen that legendary is here.
            natija.muddat = bidaya.elapsed();
            return Ok(natija);
        }

        let Some(qeema) = iqra_json(&masar_muthabbat) else {
            return Err(KhataKashf::TaadhurQiraatFahras {
                matjar: MUARRIF,
                masar: masar_muthabbat,
                sabab: std::io::Error::other(
                    "the installed-games file could not be read as JSON",
                ),
            }
            .into());
        };

        let Some(madakhil) = qeema.as_object() else {
            return Err(KhataKashf::TarwisatFahrasTalifa {
                matjar: MUARRIF,
                masar: masar_muthabbat,
                tafsil: "the installed-games file is not the object of app names this reader \
                         expects, so legendary's layout has changed"
                    .to_owned(),
                mawdi: None,
            }
            .into());
        };

        let idadat = IdadIni::iqra(&jidhr.join(ISM_IDADAT)).unwrap_or_default();
        let heroic = IdadHeroic::iqra(&jidhr);
        let suwar = bayanat_epic(&jidhr.join(MUJALLAD_BAYANAT));

        let siyaq_legendary =
            SiyaqLegendary { nizam: siyaq.nizam, idadat: &idadat, heroic: &heroic };
        for (miftah, sijill) in madakhil {
            if let Some(luba) = luba_min_sijill(
                miftah,
                sijill,
                &siyaq_legendary,
                &suwar,
                &mut natija.tanbihat,
            ) {
                natija.alaab.push(luba);
            }
        }

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let Some(jidhr) = self.mawqi(siyaq) else {
            return Vec::new();
        };
        // The root is where `installed.json` and `config.ini` are rewritten on
        // every install, uninstall and setting change; the metadata directory is
        // where a newly claimed title's catalogue entry appears without either
        // of those being touched. Watching the root alone would miss artwork
        // arriving for a game the user already owns.
        [jidhr.clone(), jidhr.join(MUJALLAD_BAYANAT)]
            .into_iter()
            .filter(|masar| masar.is_dir())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// where legendary keeps its configuration
// ---------------------------------------------------------------------------

/// Every configuration root worth looking at, most likely first.
///
/// Duplicates are removed, because `$XDG_CONFIG_HOME` is very often exactly
/// `~/.config` and probing the same directory twice would double every warning
/// it produces. Heroic's own bundled copy is removed for the reason on this
/// module's header.
fn judhur_muhtamala(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
    let mut murashahat: Vec<PathBuf> = Vec::new();

    // Taarib's own setting outranks legendary's environment variable. Both are
    // deliberate statements, and when they disagree the one made in the
    // application the user is looking at is the one they expect to win.
    if let Some(masar) = siyaq.manassat.legendary.as_ref() {
        murashahat.push(masar.clone());
    }

    // legendary's own override, and the only other one that is a deliberate
    // statement about where its configuration is rather than a convention.
    if let Some(mutlaq) = masar_min_beea(MUTAGHAYYIR_JIDHR) {
        murashahat.push(mutlaq);
    }

    // From the context rather than `$XDG_CONFIG_HOME` directly: the base
    // directory is an ambient fact, and the context already applied the
    // specification's absolute-only rule and its `~/.config` default. When the
    // variable is unset this is exactly the candidate pushed below, which the
    // deduplication at the end of this function then folds away.
    if siyaq.nizam == NizamTashghil::Linux {
        murashahat.push(siyaq.khazina_idadat.join(MUJALLAD_LEGENDARY));
    }

    // Hardcoded in legendary on every platform, including Windows: it does not
    // use `%APPDATA%`, and looking there would find nothing on the machines
    // where a Windows user runs it.
    murashahat.push(siyaq.manzil.join(".config").join(MUJALLAD_LEGENDARY));

    if siyaq.nizam == NizamTashghil::Linux && siyaq.yashmal_hawiyat {
        murashahat.push(
            siyaq
                .manzil
                .join(".var")
                .join("app")
                .join(HAWIYAT_HEROIC)
                .join("config")
                .join(MUJALLAD_LEGENDARY),
        );
    }

    let mut judhur: Vec<PathBuf> = Vec::with_capacity(murashahat.len());
    for murashah in murashahat {
        let heroic_dakhili = std::ffi::OsStr::new(MUJALLAD_HEROIC_LEGENDARY);
        if murashah.components().any(|juz| juz.as_os_str() == heroic_dakhili) {
            continue;
        }
        if !judhur.contains(&murashah) {
            judhur.push(murashah);
        }
    }
    judhur
}

/// Reads one environment variable as an absolute path.
///
/// The three variables still read here — `LEGENDARY_CONFIG_PATH`,
/// `LEGENDARY_WINE_PREFIX` and `WINEPREFIX` — are the only ones any adapter
/// reads for itself, and they stay for the reason [`SiyaqFahs`] gives: each is
/// one tool naming its own directory, so there is no second adapter for it to
/// disagree with and nothing to hoist onto a context the other sixteen share.
/// Every *ambient* directory this adapter needs now comes off that context.
///
/// Relative values are ignored rather than resolved. A relative
/// `LEGENDARY_CONFIG_PATH` would resolve against whatever directory this process
/// happened to start in — the user's home on a desktop launch, wherever they
/// were standing in a terminal — and silently accepting it would make discovery
/// depend on how Taarib was started.
fn masar_min_beea(mutaghayyir: &str) -> Option<PathBuf> {
    let qeema = std::env::var_os(mutaghayyir)?;
    if qeema.is_empty() {
        return None;
    }
    let masar = PathBuf::from(qeema);
    masar.is_absolute().then_some(masar)
}

// ---------------------------------------------------------------------------
// config.ini
// ---------------------------------------------------------------------------

/// legendary's `config.ini`, reduced to sections of string values.
///
/// ## The subset this reader accepts
///
/// `config.ini` is written by Python's `configparser`, and this reads the part
/// of that format legendary actually emits:
///
/// | construct | example |
/// | --- | --- |
/// | a section header | `[default]`, `[Fortnite.env]` |
/// | an assignment with `=` or `:` | `wine_prefix = /home/u/pfx`, `wine_prefix: /home/u/pfx` |
/// | a comment line, introduced by `#` or `;` at the start of the line | `; installed by hand` |
/// | a key before any section | filed under `default`, which is where `configparser` puts it |
///
/// Option names are lowercased on the way in and on the way out, because
/// `configparser` lowercases them by default and legendary does not override
/// that — so `WINEPREFIX` written by a user in a `[…​.env]` section is the same
/// option as `wineprefix`. **Section** names are kept verbatim, because they are
/// Epic app names and those are case-sensitive; a lookup falls back to a
/// case-insensitive match only after the exact one misses.
///
/// ## What it refuses
///
/// Values are taken verbatim and are never unquoted: `configparser` does not
/// strip quotes either, so a `wine_prefix = "/home/u/pfx"` is a path with quote
/// characters in it in legendary too, and stripping them here would make Taarib
/// find a prefix legendary cannot. Indented continuation lines — which
/// `configparser` supports and legendary never writes — are dropped rather than
/// folded into the previous value, because folding them wrongly would silently
/// extend a path. A line that is neither a section, a comment, nor an assignment
/// is skipped.
#[derive(Debug, Clone, Default)]
struct IdadIni {
    /// Section name to lowercased option name to raw value.
    aqsam: BTreeMap<String, BTreeMap<String, String>>,
}

impl IdadIni {
    /// Reads and parses `config.ini`, or nothing when it is absent or
    /// unreadable. The two are not distinguished, because both mean the same
    /// thing to every caller: legendary recorded no settings this adapter can
    /// see.
    fn iqra(masar: &Path) -> Option<Self> {
        let bayt = std::fs::read(masar).ok()?;
        let nass = String::from_utf8_lossy(&bayt);
        let mut aqsam: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut hali = QISM_IFTIRADI.to_owned();

        for satr_kham in nass.trim_start_matches('\u{feff}').lines() {
            // A continuation line is indented; legendary writes none, and
            // guessing which value it continues would be a way to extend a path
            // by a line the user cannot see.
            if satr_kham.starts_with(' ') || satr_kham.starts_with('\t') {
                continue;
            }
            let satr = satr_kham.trim();
            if satr.is_empty() || satr.starts_with('#') || satr.starts_with(';') {
                continue;
            }
            if let Some(dakhili) = satr.strip_prefix('[') {
                if let Some(ism) = dakhili.strip_suffix(']') {
                    ism.trim().clone_into(&mut hali);
                }
                continue;
            }

            // `=` and `:` are both assignment in `configparser`, and whichever
            // comes first is the separator — which matters for a Windows path
            // written `wine_prefix = C:\pfx`, where the colon is inside the
            // value and the equals sign is the real separator.
            let mawqi_musawi = satr.find('=');
            let mawqi_nuqta = satr.find(':');
            let mawqi = match (mawqi_musawi, mawqi_nuqta) {
                (Some(musawi), Some(nuqta)) => musawi.min(nuqta),
                (Some(musawi), None) => musawi,
                (None, Some(nuqta)) => nuqta,
                (None, None) => continue,
            };
            let Some(ism) = satr.get(..mawqi) else {
                continue;
            };
            let Some(qeema) = satr.get(mawqi.saturating_add(1)..) else {
                continue;
            };
            let ism = ism.trim().to_ascii_lowercase();
            if ism.is_empty() {
                continue;
            }
            let _ = aqsam
                .entry(hali.clone())
                .or_default()
                .insert(ism, qeema.trim().to_owned());
        }

        Some(Self { aqsam })
    }

    /// One option, by section and name.
    fn qeema(&self, qism: &str, ism: &str) -> Option<&str> {
        let matlub = ism.to_ascii_lowercase();
        let qeem = self.aqsam.get(qism).or_else(|| {
            let qism_saghir = qism.to_ascii_lowercase();
            self.aqsam
                .iter()
                .find(|(mawjud, _)| mawjud.to_ascii_lowercase() == qism_saghir)
                .map(|(_, qeem)| qeem)
        })?;
        qeem.get(matlub.as_str()).map(String::as_str).filter(|qeema| !qeema.trim().is_empty())
    }
}

// ---------------------------------------------------------------------------
// Heroic's own settings, when Heroic is what drives this legendary
// ---------------------------------------------------------------------------

/// The prefixes Heroic records for the legendary installation beside it.
///
/// Read only when the legendary root has a Heroic configuration directory as a
/// sibling — which is exactly the Flatpak layout
/// `~/.var/app/com.heroicgameslauncher.hgl/config/{legendary,heroic}`. In that
/// arrangement legendary's own `config.ini` is empty of Wine settings, because
/// Heroic passes the prefix on the command line instead, and reading Heroic's
/// side is the only way to learn where the game's `C:` drive is.
///
/// This is deliberately narrow: it reads two keys out of Heroic's files and
/// nothing else. [`crate::matajir::heroic`] owns Heroic's catalogue, and
/// duplicating any of it here would mean two readers to keep in agreement.
#[derive(Debug, Clone, Default)]
struct IdadHeroic {
    /// The prefix Heroic records for one app name.
    beeat: BTreeMap<String, PathBuf>,
    /// Heroic's own default, which a game with no override inherits.
    iftiradi: Option<PathBuf>,
}

impl IdadHeroic {
    /// Looks for a Heroic configuration directory beside a legendary root.
    fn iqra(jidhr_legendary: &Path) -> Self {
        let Some(jidhr) = jidhr_legendary.parent().map(|walid| walid.join("heroic")) else {
            return Self::default();
        };
        if !jidhr.is_dir() {
            return Self::default();
        }

        let amma = iqra_json(&jidhr.join("config.json"));
        let iftiradi = amma
            .as_ref()
            .and_then(|qeema| qeema.get("defaultSettings"))
            .and_then(|qeema| nass_haql(qeema, "winePrefix"))
            .map(PathBuf::from);

        let mut beeat = BTreeMap::new();
        if let Ok(qaima) = std::fs::read_dir(jidhr.join("GamesConfig")) {
            for madkhal in qaima.take(HADD_BAYANAT).flatten() {
                let masar = madkhal.path();
                if masar.extension().is_none_or(|imtidad| !imtidad.eq_ignore_ascii_case("json")) {
                    continue;
                }
                let Some(ism) = masar.file_stem().map(|ism| ism.to_string_lossy().into_owned())
                else {
                    continue;
                };
                let Some(qeema) = iqra_json(&masar) else {
                    continue;
                };
                // Heroic writes `{"<app name>": { … }}` with bookkeeping keys
                // beside it, so the entry is matched by file name first and by
                // shape second — a Heroic release that adds a sibling key must
                // not turn a bookkeeping value into a prefix.
                let mukhtar = qeema
                    .get(&ism)
                    .and_then(|dakhili| nass_haql(dakhili, "winePrefix"))
                    .or_else(|| {
                        qeema.as_object()?.values().find_map(|dakhili| {
                            nass_haql(dakhili, "winePrefix")
                        })
                    });
                if let Some(beea) = mukhtar {
                    let _ = beeat.insert(ism, PathBuf::from(beea));
                }
            }
        }

        Self { beeat, iftiradi }
    }

    /// The prefix Heroic records for one app name, then its default.
    fn beea(&self, ism_tatbeeq: &str) -> Option<PathBuf> {
        self.beeat.get(ism_tatbeeq).cloned().or_else(|| self.iftiradi.clone())
    }
}

// ---------------------------------------------------------------------------
// resolving a game's compatibility environment
// ---------------------------------------------------------------------------

/// What every install record is read against.
#[derive(Debug, Clone, Copy)]
struct SiyaqLegendary<'a> {
    /// The operating system this scan is running on.
    nizam: NizamTashghil,
    /// legendary's own settings.
    idadat: &'a IdadIni,
    /// Heroic's settings, when Heroic is what drives this legendary.
    heroic: &'a IdadHeroic,
}

/// Finds the prefix for one app, in the order argued on this module's header.
fn beea_lil_tatbeeq(ism_tatbeeq: &str, siyaq: &SiyaqLegendary<'_>) -> Option<PathBuf> {
    let qism_beea = format!("{ism_tatbeeq}.env");
    let murashahat = [
        siyaq.idadat.qeema(&qism_beea, "wineprefix").map(PathBuf::from),
        siyaq.idadat.qeema(ism_tatbeeq, "wine_prefix").map(PathBuf::from),
        siyaq.heroic.beea(ism_tatbeeq),
        siyaq
            .idadat
            .qeema(&format!("{QISM_IFTIRADI}.env"), "wineprefix")
            .map(PathBuf::from),
        siyaq.idadat.qeema(QISM_IFTIRADI, "wine_prefix").map(PathBuf::from),
        masar_min_beea("LEGENDARY_WINE_PREFIX"),
        masar_min_beea("WINEPREFIX"),
    ];
    murashahat.into_iter().flatten().find(|masar| masar.is_dir())
}

/// The Wine build legendary is configured to use, when its executable names one.
///
/// legendary records a path to a `wine` binary rather than a build name, and a
/// path is where the build name is: distributions of Wine and Proton both unpack
/// as `<build name>/files/bin/wine` or `<build name>/bin/wine`, so the component
/// before `files` or `bin` is what the user would call the build. Nothing is
/// invented when the path has no such shape — a system `wine` at `/usr/bin/wine`
/// names no build, and reporting `usr` would be worse than reporting nothing.
fn isdar_min_tanfidhi(tanfidhi: &str) -> Option<String> {
    let mut sabiq: Option<String> = None;
    for juz in Path::new(tanfidhi.trim()).components() {
        let std::path::Component::Normal(ism) = juz else {
            continue;
        };
        let nass = ism.to_string_lossy();
        if matches!(nass.as_ref(), "bin" | "files" | "dist") {
            return sabiq.filter(|ism| !matches!(ism.as_str(), "usr" | "local" | "opt"));
        }
        sabiq = Some(nass.into_owned());
    }
    None
}

/// Resolves the compatibility layer one legendary title runs behind.
///
/// A title whose declared platform is native to the running system runs
/// natively, whatever `config.ini` says. Everything else is a Windows program,
/// and the prefix is what makes it runnable — so the prefix is classified by
/// [`crate::beea::naw_beea`], which is the one place in this crate that knows
/// Proton's markers from a plain Wine prefix, and this adapter never
/// reimplements that test.
///
/// Three outcomes, and the middle one is the reason this function is longer than
/// it looks like it should be:
///
/// - **A populated prefix**: reported as Proton or Wine with whatever build the
///   prefix or `config.ini` names.
/// - **A recorded prefix that was never built**: `Asli` plus an explicit trait
///   and a warning. legendary creates nothing; the directory appears the first
///   time the user runs the game through Wine, and until then there is no `C:`
///   drive to install into.
/// - **No prefix recorded anywhere**: the same encoding, with a different
///   remedy in the warning. The layer exists — a Windows executable on a Linux
///   filesystem does not run without one — and it is unidentified, which is a
///   different statement from "this game is native" and is why the trait is
///   attached rather than the environment being left silently at `Asli`.
fn beea_luba(
    ism_tatbeeq: &str,
    unwan: &str,
    manassat_luba: Option<&str>,
    siyaq: &SiyaqLegendary<'_>,
    simat: &mut Vec<SimatLuba>,
    tanbihat: &mut Vec<TanbihFahs>,
) -> BeeatTawafuq {
    if asliya(manassat_luba, siyaq.nizam) {
        return BeeatTawafuq::Asli;
    }

    let isdar_muallan = siyaq
        .idadat
        .qeema(ism_tatbeeq, "wine_executable")
        .or_else(|| siyaq.idadat.qeema(QISM_IFTIRADI, "wine_executable"))
        .and_then(isdar_min_tanfidhi);

    let Some(masar_beea) = beea_lil_tatbeeq(ism_tatbeeq, siyaq) else {
        simat.push(SimatLuba::TabaqatTawafuq(
            "Wine or Proton, prefix unknown".to_owned(),
        ));
        if siyaq.nizam != NizamTashghil::Windows {
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                unwan.to_owned(),
                "legendary installed the Windows build of this game and records no Wine prefix \
                 for it anywhere — not in config.ini, not in a Heroic configuration beside it, \
                 and not in the environment — so Taarib cannot tell where the game's own C: \
                 drive is. Set wine_prefix for it in legendary's config.ini, or launch it once \
                 through the wrapper you use, then rescan.",
            ));
        }
        return BeeatTawafuq::Asli;
    };

    match crate::beea::naw_beea(&masar_beea) {
        BeeatTawafuq::Proton { isdar, beea } => BeeatTawafuq::Proton {
            isdar: if isdar == "Proton" { isdar_muallan.unwrap_or(isdar) } else { isdar },
            beea,
        },
        BeeatTawafuq::Wine { isdar, beea } => {
            BeeatTawafuq::Wine { isdar: isdar.or(isdar_muallan), beea }
        },
        // `naw_beea` answers `Asli` for a path that is not a populated prefix,
        // which here means the directory is recorded and empty rather than that
        // the game is native.
        BeeatTawafuq::Asli | BeeatTawafuq::Rosetta => {
            simat.push(SimatLuba::TabaqatTawafuq(
                "Wine or Proton, prefix not built yet".to_owned(),
            ));
            tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                masar_beea.display().to_string(),
                format!(
                    "{unwan}: the Wine prefix recorded for this game has no C: drive in it yet, \
                     so it was configured and never built. Launch the game once so the prefix is \
                     created, then rescan."
                ),
            ));
            BeeatTawafuq::Asli
        },
    }
}

/// Whether Epic's platform string names this machine's own platform.
///
/// Epic writes `Windows`, `Win32` and `Mac`; legendary passes them through
/// unchanged. An absent platform is treated as Windows, because that is what
/// Epic defaults to and what every prefix in this file exists for.
fn asliya(manassat_luba: Option<&str>, nizam: NizamTashghil) -> bool {
    let muallan = manassat_luba.unwrap_or("Windows").to_ascii_lowercase();
    match nizam {
        NizamTashghil::Windows => matches!(muallan.as_str(), "windows" | "win32" | "win64" | "pc"),
        NizamTashghil::Linux => muallan == "linux",
        NizamTashghil::Mac => matches!(muallan.as_str(), "mac" | "macos" | "osx" | "darwin"),
    }
}

// ---------------------------------------------------------------------------
// Epic's cached catalogue entries
// ---------------------------------------------------------------------------

/// What one `metadata/<app_name>.json` contributes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct BitaqatEpic {
    /// The title as Epic's catalogue gives it, which is occasionally better than
    /// the one in the install record — legendary copies the title at install
    /// time and Epic sometimes renames a title afterwards.
    unwan: Option<String>,
    /// The artwork addresses.
    suwar: MasadirSuwar,
    /// The category paths Epic files the entry under.
    fiat: Vec<String>,
}

impl BitaqatEpic {
    /// Whether Epic files this entry as a game.
    ///
    /// The test is the presence of a `games` category, not the absence of an
    /// `applications` one — and that distinction is load-bearing, because Epic
    /// files ordinary games under **both**: a typical base game carries
    /// `applications`, `games`, `games/edition` and `games/edition/base`. A rule
    /// written the other way round would mark every game in the library as not a
    /// game.
    ///
    /// An entry with no categories at all is treated as a game. Some older
    /// catalogue entries carry none, and hiding a game the user owns because
    /// Epic's metadata is thin would be the wrong way to be wrong.
    fn hiya_luba(&self) -> bool {
        self.fiat.is_empty()
            || self.fiat.iter().any(|fia| fia == "games" || fia.starts_with("games/"))
    }
}

/// Reads every cached catalogue entry, keyed by app name.
///
/// One `read_dir` and one parse per owned title, once per scan. Doing it here
/// rather than per game is what keeps Phase 20B from re-reading the same
/// directory: the artwork addresses travel on the discovered game and nothing
/// downstream has to know this directory exists.
fn bayanat_epic(mujallad: &Path) -> BTreeMap<String, BitaqatEpic> {
    let mut bitaqat = BTreeMap::new();
    let Ok(qaima) = std::fs::read_dir(mujallad) else {
        // No metadata directory at all is ordinary: legendary writes it when it
        // syncs the library, and a fresh installation that has only ever run
        // `legendary install` by app name has none. The games still arrive, with
        // no artwork, which `suwar` then goes looking for elsewhere.
        return bitaqat;
    };

    for madkhal in qaima.take(HADD_BAYANAT).flatten() {
        let masar = madkhal.path();
        if masar.extension().is_none_or(|imtidad| !imtidad.eq_ignore_ascii_case("json")) {
            continue;
        }
        let Some(qeema) = iqra_json(&masar) else {
            continue;
        };
        let Some(ism) = nass_haql(&qeema, "app_name")
            .or_else(|| masar.file_stem().map(|ism| ism.to_string_lossy().into_owned()))
        else {
            continue;
        };

        // Everything worth reading is under `metadata`; the outer object is
        // legendary's own bookkeeping — asset manifests, base URLs, namespaces.
        let maalumat = qeema.get("metadata").unwrap_or(&qeema);
        let _ = bitaqat.insert(
            ism,
            BitaqatEpic {
                unwan: nass_haql(maalumat, "title"),
                suwar: suwar_min_bitaqa(maalumat),
                fiat: fiat_min_bitaqa(maalumat),
            },
        );
    }

    bitaqat
}

/// Picks the three artwork shapes out of Epic's `keyImages` array.
///
/// Addresses rather than files, and that is not a shortcut: legendary caches no
/// images at all, and Heroic caches its own inside Electron storage that is not
/// addressable by another program. So there is nothing local to prefer, and the
/// URL is what is passed on for `suwar` to fetch once, later, off the path the
/// library screen waits on.
///
/// Each shape is searched for by type name in preference order, so a title that
/// has only a storefront image still gets a cover instead of nothing.
fn suwar_min_bitaqa(maalumat: &Value) -> MasadirSuwar {
    let Some(qaima) = maalumat.get("keyImages").and_then(Value::as_array) else {
        return MasadirSuwar::default();
    };

    let bi_naw = |anwa: &[&str]| -> Option<MasdarSura> {
        for naw in anwa {
            for sura in qaima {
                if nass_haql(sura, "type").as_deref() != Some(*naw) {
                    continue;
                }
                if let Some(rabt) = nass_haql(sura, "url") {
                    return Some(MasdarSura::Rabt(rabt));
                }
            }
        }
        None
    };

    MasadirSuwar {
        ghilaf: bi_naw(ANWA_GHILAF.as_slice()),
        batl: bi_naw(ANWA_BATL.as_slice()),
        shiar: bi_naw(ANWA_SHIAR.as_slice()),
    }
}

/// The category paths Epic files an entry under, lowercased.
fn fiat_min_bitaqa(maalumat: &Value) -> Vec<String> {
    maalumat
        .get("categories")
        .and_then(Value::as_array)
        .map(|qaima| {
            qaima
                .iter()
                .filter_map(|fia| nass_haql(fia, "path").map(|nass| nass.to_ascii_lowercase()))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// one install record becomes one game
// ---------------------------------------------------------------------------

/// Turns one `installed.json` entry into a discovered game.
fn luba_min_sijill(
    miftah: &str,
    sijill: &Value,
    siyaq: &SiyaqLegendary<'_>,
    bitaqat: &BTreeMap<String, BitaqatEpic>,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    // A DLC is installed into the base game's directory and has no library entry
    // of its own. Listing it would put a second card in the grid pointing at the
    // same files as the first.
    if sawab_haql(sijill, "is_dlc").unwrap_or(false) {
        return None;
    }

    let ism_tatbeeq = nass_haql(sijill, "app_name").unwrap_or_else(|| miftah.to_owned());
    let bitaqa = bitaqat.get(&ism_tatbeeq);
    let unwan = nass_haql(sijill, "title")
        .or_else(|| bitaqa.and_then(|bitaqa| bitaqa.unwan.clone()))
        .unwrap_or_else(|| ism_tatbeeq.clone());

    let Some(jidhr) = nass_haql(sijill, "install_path").map(PathBuf::from) else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{MUARRIF}: {unwan}"),
            "this title records no install path, so there is nothing on disk to probe. Running \
             `legendary repair` for it rewrites the record.",
        ));
        return None;
    };
    if !jidhr.is_dir() {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "legendary lists {unwan} as installed here and the folder is gone; it was deleted \
                 outside legendary, or lives on a drive that is not mounted"
            ),
        ));
        return None;
    }

    let manassat_luba = nass_haql(sijill, "platform");
    let mut simat = Vec::new();
    let beea = beea_luba(
        &ism_tatbeeq,
        &unwan,
        manassat_luba.as_deref(),
        siyaq,
        &mut simat,
        tanbihat,
    );

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

    if let Some(bitaqa) = bitaqa
        && !bitaqa.hiya_luba()
    {
        simat.push(SimatLuba::LaysatLuba(format!(
            "Epic files this as {} rather than as a game",
            if bitaqa.fiat.is_empty() {
                "an uncategorised entry".to_owned()
            } else {
                bitaqa.fiat.join(", ")
            }
        )));
    }

    Some(LubaMuktashafa {
        // The rule this whole module exists for. Epic's app name goes inside the
        // box; `Legendary` goes around it. Nothing downstream — not the registry
        // lookup, not the identity derivation, not the duplicate merge — sees a
        // legendary identity, because `aila` and `muarrif` both unwrap it.
        masdar: MasdarLuba::Legendary(Box::new(MasdarLuba::Epic(ism_tatbeeq.clone()))),
        hala_matjar: None,
        ism: unwan,
        tanfidhi: nass_haql(sijill, "executable")
            .and_then(|nisbi| tanfidhi_dakhil(&jidhr, &nisbi)),
        jidhr,
        hajm: raqm_haql(sijill, "install_size").unwrap_or(0),
        // Epic's build version string, which is what its own patch metadata keys
        // on and the only build identifier legendary records.
        bina_manassa: nass_haql(sijill, "version"),
        // legendary writes one `installed.json` for the whole library and stamps
        // nothing per game, so the file's own modification time says only that
        // *something* changed. Reporting it as this game's update time would be
        // inventing a fact, and a wrong "updated 3 minutes ago" on two hundred
        // games is worse than an empty column on all of them.
        akhir_tahdith: None,
        // legendary tracks no play times at all.
        akhir_laab: None,
        beea,
        suwar: bitaqa.map(|bitaqa| bitaqa.suwar.clone()).unwrap_or_default(),
        // `start_params` is what the *user* set in config.ini, and it is the only
        // thing this field may carry. The manifest's own `launch_parameters` are
        // Epic's arguments for the game and are supplied by whatever launches it;
        // copying them here would make Phase 15 "extend" options the user never
        // chose, and then pass them twice.
        khiyarat_tashghil: siyaq
            .idadat
            .qeema(&ism_tatbeeq, "start_params")
            .map(str::to_owned),
        // legendary sets this flag when a download was interrupted or a repair is
        // outstanding, which is exactly the state in which a patch must not be
        // offered: the client is about to overwrite the files.
        muktamila: !sawab_haql(sijill, "needs_verification").unwrap_or(false),
        simat,
    })
}

/// Resolves a store-declared executable inside an install root.
///
/// The value comes out of legendary's JSON with Windows separators, so it is
/// normalized and then goes through the containment check rather than being
/// joined: a relative path with `..` in it would otherwise let a tampered
/// `installed.json` point Taarib's probe at a file outside the game. An absolute
/// path is accepted only when it is already inside the root.
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
// small JSON readers
// ---------------------------------------------------------------------------

/// Reads and parses a JSON file, or nothing.
///
/// Absent and malformed are deliberately not distinguished: every caller treats
/// them identically — this file contributed nothing — and every caller already
/// names the path it tried in whatever it reports.
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
/// legendary writes install sizes as numbers and, in older releases, as decimal
/// strings. Both are accepted rather than one being declared correct, because
/// the cost of accepting both is this function and the cost of guessing wrong is
/// every game reporting a size of zero.
fn raqm_haql(qeema: &Value, miftah: &str) -> Option<u64> {
    let haql = qeema.get(miftah)?;
    haql.as_u64().or_else(|| haql.as_str()?.trim().parse().ok())
}

/// A boolean field, or nothing.
fn sawab_haql(qeema: &Value, miftah: &str) -> Option<bool> {
    qeema.get(miftah)?.as_bool()
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    #[test]
    fn khazinat_al_idadat_min_al_siyaq_la_min_al_beea() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let khazina = masrah.path().join("khazina");

        // `$XDG_CONFIG_HOME` is an *ambient* directory and now arrives on the
        // context; `LEGENDARY_CONFIG_PATH` is legendary naming its own and stays
        // in this file, which is why nothing here tries to set either.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, masrah.path());
        siyaq.khazina_idadat = khazina.clone();

        let judhur = judhur_muhtamala(&siyaq);
        assert!(judhur.contains(&khazina.join(MUJALLAD_LEGENDARY)), "{judhur:?}");
        // The hardcoded `~/.config/legendary` is still probed beside it:
        // legendary uses that path on every platform whatever XDG says.
        assert!(
            judhur.contains(&masrah.path().join(".config").join(MUJALLAD_LEGENDARY)),
            "{judhur:?}"
        );
        Ok(())
    }

    #[test]
    fn al_khazina_al_iftiradiya_tundamm_ila_masar_al_manzil() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;

        // With the variable unset the context carries `~/.config`, which is
        // exactly the hardcoded candidate — so the deduplication must fold the
        // two into one probe rather than warning about one directory twice.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, masrah.path());
        let mutawaqqa = masrah.path().join(".config").join(MUJALLAD_LEGENDARY);
        let judhur = judhur_muhtamala(&siyaq);
        let marrat = judhur.iter().filter(|masar| **masar == mutawaqqa).count();
        assert_eq!(marrat, 1, "{judhur:?}");
        Ok(())
    }
}
