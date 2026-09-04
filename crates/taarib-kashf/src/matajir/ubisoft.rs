//! متجر يوبيسوفت — Ubisoft Connect, from the registry and from a file format
//! nobody has published.
//!
//! Two sources, and they are not equal. The registry is authoritative and
//! simple: one key per installed game, named by the launcher's own numeric
//! install identifier, holding an `InstallDir`. That is where every game in
//! this adapter comes from, and a game is reported whether or not anything else
//! can be read about it.
//!
//! ```text
//! HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\<id>\InstallDir
//! HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\InstallDir      the launcher itself
//! <launcher>\cache\configuration\configurations              titles and executables
//! <launcher>\cache\assets\<name>                             images already downloaded
//! ```
//!
//! Windows only. Ubisoft Connect has no Linux or macOS client; a Ubisoft game
//! on Linux is running under a compatibility layer that another launcher owns.
//!
//! # The `configurations` file is undocumented, and this adapter says so
//!
//! `configurations` is a binary blob the launcher writes: a concatenation of
//! records, each preceded by a length prefix, each holding a YAML-ish document
//! describing one product — its title, its localizations, its executables, the
//! names of the images the launcher cached for it. Ubisoft has never published
//! the framing, it is not versioned in any way this adapter could check, and it
//! has changed shape more than once.
//!
//! **So the framing is not parsed at all.** Nothing here reads a length prefix,
//! seeks by an offset, or assumes a record boundary — because a reader built on
//! a guessed framing does not fail loudly when the guess goes stale, it silently
//! attributes one game's title to another game. Instead:
//!
//! 1. The file is decoded lossily and split into runs of readable text.
//!    Control bytes, length prefixes and anything that is not valid UTF-8
//!    become separators. Nothing is assumed about what lies between the runs.
//! 2. Inside those runs, a game is found by an **anchor that cannot be
//!    coincidental**: its own record names its registry key, as
//!    `…\Ubisoft\Launcher\Installs\<id>\InstallDir`. The install identifier in
//!    that string is the same identifier the registry key is named by, so a
//!    record is tied to a game by the launcher's own cross-reference rather than
//!    by position.
//! 3. Around the anchor, a bounded window is taken — from the previous record
//!    header to the next one, capped in size either way — and read as flat
//!    `key: value` lines. A key that is not recognised is ignored.
//!
//! When any of that fails — no anchor, no window, no title in the window — the
//! adapter **falls back to the install directory's own name**, which Ubisoft
//! names after the game, and reports the game anyway. It never guesses a title
//! from an adjacent record, and it never reports a game it cannot name.
//!
//! # Localized titles
//!
//! A record's `name:` is frequently not a title but a key such as `l1`, resolved
//! from a localization block in the same record. The window is searched for that
//! key, and only a value found inside the same window is used — a resolution
//! that had to leave the record is not a resolution.
//!
//! # Artwork
//!
//! Only files that exist. A record names its images (`thumb_image`,
//! `logo_image`, `background_image`) and the launcher caches them under
//! `cache\assets`. Each name is joined to that directory and returned **only if
//! the file is really there**, which makes the layout self-verifying: if the
//! cache moves, the check fails and [`MasadirSuwar`] stays empty rather than
//! pointing at nothing. Icons are skipped, because `.ico` is not a format the
//! artwork pipeline decodes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::{dakhil, qira};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "ubisoft";

/// Platforms this adapter can find anything on.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The configuration blob, relative to the launcher's own directory.
const MASAR_TASHKEELAT: [&str; 3] = ["cache", "configuration", "configurations"];

/// The image cache, relative to the launcher's own directory.
const MASAR_USUL: [&str; 2] = ["cache", "assets"];

/// A refusal rather than an allocation: a configuration blob larger than this
/// is not a configuration blob.
const HADD_TASHKEELAT: u64 = 64 * 1024 * 1024;

/// The shortest run of readable text worth looking at.
const HADD_ADNA_MINTAQA: usize = 24;

/// The largest window read around one anchor, in bytes. Records are a few
/// kilobytes; this is generous and bounded.
const HADD_NAFIDHA: usize = 32 * 1024;

/// Line prefixes that begin a record in every shape of this file seen so far.
/// Used only to bound a window, never to trust a boundary.
const RUUS_SIJILL: [&str; 2] = ["\nroot:", "\nversion:"];

/// Executables that belong to the launcher or to an anti-cheat service rather
/// than to the game.
const TANFIDHIYAT_MUSTABADA: [&str; 8] = [
    "upc.exe",
    "uplay.exe",
    "ubisoftconnect.exe",
    "ubisoftgamelauncher.exe",
    "uplaywebcore.exe",
    "easyanticheat",
    "battleye",
    "beservice",
];

/// Anti-cheat services, matched case-insensitively against the executables a
/// record names. Hints for Phase 16, never conclusions.
const ATHAR_HIMAYA: [(&str, &str); 5] = [
    ("easyanticheat", "EasyAntiCheat"),
    ("eac_launcher", "EasyAntiCheat"),
    ("battleye", "BattlEye"),
    ("beservice", "BattlEye"),
    ("punkbuster", "PunkBuster"),
];

/// Keys whose values name an image in the launcher's cache, with the artwork
/// slot each one fills.
const MAFATIH_SUWAR: [(&str, DawrSura); 5] = [
    ("thumb_image", DawrSura::Ghilaf),
    ("cover_image", DawrSura::Ghilaf),
    ("background_image", DawrSura::Batl),
    ("splash_image", DawrSura::Batl),
    ("logo_image", DawrSura::Shiar),
];

/// Image file extensions the artwork pipeline can decode.
const IMTIDADAT_SURA: [&str; 4] = ["png", "jpg", "jpeg", "webp"];

/// Which artwork slot an image fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DawrSura {
    /// The vertical cover.
    Ghilaf,
    /// The wide banner.
    Batl,
    /// The logo.
    Shiar,
}

/// Ubisoft Connect.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarUbisoft;

impl MatjarUbisoft {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarUbisoft {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "يوبيسوفت"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Ubisoft Connect"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        if let Some(tajawuz) = siyaq.manassat.ubisoft.as_ref() {
            return Some(tajawuz.clone());
        }
        // One registry value and one existence check. Nothing is parsed here.
        jidhr_musajjal()
            .filter(|masar| masar.is_dir())
            .or_else(|| jidhr_taqleedi().filter(|masar| masar.is_dir()))
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::JidhrMuhaddadMafqud`] when a configured launcher
    /// root is not there. Nothing else is fatal: an unreadable configuration
    /// blob costs every game its title and nothing else, because the titles
    /// then come from the install directories, and a game whose directory was
    /// deleted is one [`TanbihFahs`].
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }
        if let Some(tajawuz) = siyaq.manassat.ubisoft.as_ref()
            && !tajawuz.exists()
        {
            return Err(KhataKashf::JidhrMuhaddadMafqud {
                matjar: MUARRIF,
                masar: tajawuz.clone(),
            }
            .into());
        }

        let tathbitat = tathbitat_musajjala();
        let jidhr = siyaq
            .manassat
            .ubisoft
            .clone()
            .or_else(jidhr_musajjal)
            .or_else(jidhr_taqleedi)
            .filter(|masar| masar.is_dir());
        if tathbitat.is_empty() && jidhr.is_none() {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            jidhr_matjar: jidhr.clone(),
            ..NatijatMatjar::default()
        };

        let manatiq = jidhr.as_ref().map_or_else(Vec::new, |jidhr| {
            match manatiq_tashkeelat(jidhr) {
                Ok(manatiq) => manatiq,
                Err(tanbih) => {
                    natija.tanbihat.push(tanbih);
                    Vec::new()
                },
            }
        });
        let usul = jidhr.as_ref().map(|jidhr| {
            MASAR_USUL.iter().fold(jidhr.clone(), |mabni, juz| mabni.join(juz))
        });

        let mut maruf: BTreeSet<u32> = BTreeSet::new();
        for tathbeet in tathbitat {
            if !maruf.insert(tathbeet.muarrif) {
                continue;
            }
            if !tathbeet.jidhr.is_dir() {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    tathbeet.jidhr.display().to_string(),
                    "Ubisoft Connect still lists this game, but its install directory is gone"
                        .to_owned(),
                ));
                continue;
            }

            let sijill = sijill_luba(&manatiq, tathbeet.muarrif);
            if sijill.as_ref().is_some_and(|sijill| sijill.idafa) {
                // Downloadable content, registered like a game because the
                // launcher installs it like one. It has no executable and lives
                // inside the base game's directory, so it is not a second entry.
                continue;
            }

            let ism = sijill
                .as_ref()
                .and_then(|sijill| sijill.unwan.clone())
                .or_else(|| ism_min_mujallad(&tathbeet.jidhr))
                .unwrap_or_else(|| format!("Ubisoft {}", tathbeet.muarrif));

            let tanfidhi = sijill
                .as_ref()
                .and_then(|sijill| sijill.tanfidhiyat.iter().find_map(|nisbi| {
                    masar_dakhili(&tathbeet.jidhr, nisbi).filter(|masar| masar.is_file())
                }));

            let simat = sijill.as_ref().map_or_else(Vec::new, |sijill| {
                sijill
                    .himayat
                    .iter()
                    .map(|ism| SimatLuba::HimayaMuhtamala(ism.clone()))
                    .collect()
            });

            natija.alaab.push(LubaMuktashafa {
                masdar: MasdarLuba::Ubisoft(tathbeet.muarrif),
                hala_matjar: None,
                ism,
                jidhr: tathbeet.jidhr.clone(),
                tanfidhi,
                // The launcher records no size for an install, and measuring one
                // would mean walking every game's file tree on every scan.
                hajm: 0,
                // Nor a build identifier, nor an update or last-played time —
                // which is exactly why content fingerprints exist.
                bina_manassa: None,
                akhir_tahdith: None,
                akhir_laab: None,
                beea: BeeatTawafuq::Asli,
                suwar: usul
                    .as_ref()
                    .zip(sijill.as_ref())
                    .map_or_else(MasadirSuwar::default, |(usul, sijill)| {
                        suwar_mahalliya(usul, sijill)
                    }),
                khiyarat_tashghil: None,
                // A registry key appears when the install completes, so a key
                // that is present describes a finished install.
                muktamila: true,
                simat,
            });
        }

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let Some(jidhr) =
            siyaq.manassat.ubisoft.clone().or_else(jidhr_musajjal).or_else(jidhr_taqleedi)
        else {
            return Vec::new();
        };
        // The registry is where an install really appears, and no filesystem
        // watch can see a registry write. The configuration directory is the
        // proxy: the launcher rewrites `configurations` as part of installing,
        // so a change there is the signal that the registry changed too. The
        // default games directory is watched as well, for the same reason.
        [jidhr.join("cache").join("configuration"), jidhr.join("games")]
            .into_iter()
            .filter(|masar| masar.is_dir())
            .collect()
    }
}

/// The launcher's own directory, as the registry records it.
#[cfg_attr(
    not(windows),
    expect(
        clippy::missing_const_for_fn,
        reason = "on Windows this reads the registry; only the non-Windows body is a constant"
    )
)]
fn jidhr_musajjal() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        sijill::jidhr_launcher()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The launcher's conventional directory, used only when it is really there.
fn jidhr_taqleedi() -> Option<PathBuf> {
    let barnamij86 = std::env::var_os("ProgramFiles(x86)")
        .map_or_else(|| PathBuf::from(r"C:\Program Files (x86)"), PathBuf::from);
    let masar = barnamij86.join("Ubisoft").join("Ubisoft Game Launcher");
    masar.is_dir().then_some(masar)
}

/// One installed game, as the registry describes it.
#[derive(Debug, Clone)]
struct TathbeetUbisoft {
    /// The launcher's own install identifier, which is the registry key's name.
    muarrif: u32,
    /// The installation root.
    jidhr: PathBuf,
}

/// Every install the registry knows about. Empty on every platform but Windows.
#[cfg_attr(
    not(windows),
    expect(
        clippy::missing_const_for_fn,
        reason = "on Windows this reads the registry; only the non-Windows body is a constant"
    )
)]
fn tathbitat_musajjala() -> Vec<TathbeetUbisoft> {
    #[cfg(windows)]
    {
        sijill::tathbitat()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// The configuration blob
// ---------------------------------------------------------------------------

/// What one record says about one game.
#[derive(Debug, Clone, Default)]
struct SijillTashkeel {
    /// The title, already resolved through the record's own localizations.
    unwan: Option<String>,
    /// Executables the record names, in the order it names them.
    tanfidhiyat: Vec<String>,
    /// Image file names, with the slot each fills.
    suwar: Vec<(DawrSura, String)>,
    /// Anti-cheat services named by the record's executables.
    himayat: Vec<String>,
    /// Whether the record marks this entry as downloadable content.
    idafa: bool,
}

/// Reads the configuration blob and recovers its readable regions.
///
/// # Errors
///
/// Returns a [`TanbihFahs`] when the file is missing, too large to be what it
/// claims to be, or unreadable. None of those is fatal: without it every game
/// simply takes its install directory's name.
fn manatiq_tashkeelat(jidhr: &Path) -> Result<Vec<String>, TanbihFahs> {
    let masar = MASAR_TASHKEELAT.iter().fold(jidhr.to_path_buf(), |mabni, juz| mabni.join(juz));
    if !masar.is_file() {
        return Ok(Vec::new());
    }
    let hajm = std::fs::metadata(&masar).map_or(0, |bayanat| bayanat.len());
    if hajm > HADD_TASHKEELAT {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            format!(
                "the launcher's configuration file is {hajm} bytes, far larger than this format \
                 ever is; it was not read, and games will be named after their folders"
            ),
        ));
    }
    let bayt = qira(&masar)
        .map_err(|khata| TanbihFahs::jadeed(MUARRIF, masar.display().to_string(), khata.injilizi))?;
    Ok(manatiq_nass(&bayt))
}

/// Splits a binary blob into its runs of readable text.
///
/// Length prefixes, padding and anything that is not valid UTF-8 decode to
/// control characters or to the replacement character, and every one of those
/// ends a run. Nothing is assumed about what a run is or where it begins in the
/// file — only that its contents are text.
fn manatiq_nass(bayt: &[u8]) -> Vec<String> {
    let nass = String::from_utf8_lossy(bayt);
    let mut manatiq = Vec::new();
    let mut hali = String::new();
    for harf in nass.chars() {
        let fasil = harf == char::REPLACEMENT_CHARACTER
            || (harf.is_control() && harf != '\n' && harf != '\r' && harf != '\t');
        if fasil {
            if hali.chars().count() >= HADD_ADNA_MINTAQA {
                manatiq.push(std::mem::take(&mut hali));
            } else {
                hali.clear();
            }
        } else {
            hali.push(harf);
        }
    }
    if hali.chars().count() >= HADD_ADNA_MINTAQA {
        manatiq.push(hali);
    }
    manatiq
}

/// Finds and reads the record belonging to one install identifier.
fn sijill_luba(manatiq: &[String], muarrif: u32) -> Option<SijillTashkeel> {
    let mirasat = [
        format!(r"Installs\\{muarrif}\\"),
        format!(r"Installs\{muarrif}\"),
        format!("Installs/{muarrif}/"),
    ];
    for mintaqa in manatiq {
        let Some(mawqi) = mirasat.iter().find_map(|mirsat| mintaqa.find(mirsat.as_str())) else {
            continue;
        };
        let Some(nafitha) = nafitha_hawl(mintaqa, mawqi) else {
            continue;
        };
        return Some(iqra_sijill(nafitha));
    }
    None
}

/// The bounded window around an anchor.
///
/// The bounds are the nearest record header on either side, or the size cap
/// when there is none. Every offset used here comes from a search result or
/// from the string's own length, so no slice is taken at a position that was
/// calculated rather than found.
fn nafitha_hawl(mintaqa: &str, mawqi: usize) -> Option<&str> {
    let qabl = mintaqa.get(..mawqi)?;
    let bidaya = RUUS_SIJILL
        .iter()
        .filter_map(|raas| qabl.rfind(*raas))
        .max()
        .unwrap_or(0);
    let baqi = mintaqa.get(mawqi..)?;
    let nihaya = RUUS_SIJILL
        .iter()
        .filter_map(|raas| baqi.find(*raas))
        .min()
        .map_or(mintaqa.len(), |izaha| mawqi.saturating_add(izaha));
    let nafitha = mintaqa.get(bidaya..nihaya)?;
    Some(qusas(nafitha, HADD_NAFIDHA))
}

/// Truncates to a byte budget without ever splitting a character.
fn qusas(nass: &str, hadd: usize) -> &str {
    if nass.len() <= hadd {
        return nass;
    }
    let mawqi = nass
        .char_indices()
        .map(|(izaha, _)| izaha)
        .take_while(|izaha| *izaha <= hadd)
        .last()
        .unwrap_or(0);
    nass.get(..mawqi).unwrap_or(nass)
}

/// Reads one window as flat `key: value` lines.
fn iqra_sijill(nafitha: &str) -> SijillTashkeel {
    let azwaj = azwaj_nafitha(nafitha);
    let mut sijill = SijillTashkeel::default();

    let qeema_awwal = |matlub: &str| -> Option<String> {
        azwaj
            .iter()
            .find(|(miftah, _)| miftah == matlub)
            .map(|(_, qeema)| qeema.clone())
            .filter(|qeema| !qeema.is_empty())
    };

    // `name:` is as often a localization key as a title. A key is resolved only
    // from this same window, because a value found in another record belongs to
    // another game.
    if let Some(unwan) = qeema_awwal("name") {
        sijill.unwan = if huwa_miftah_lugha(&unwan) {
            qeema_awwal(unwan.as_str()).filter(|qeema| !huwa_miftah_lugha(qeema))
        } else {
            Some(unwan)
        };
    }
    if sijill.unwan.is_none() {
        sijill.unwan = qeema_awwal("display_name").or_else(|| qeema_awwal("title"));
    }

    sijill.idafa = qeema_awwal("is_ulc").is_some_and(|qeema| {
        matches!(qeema.to_ascii_lowercase().as_str(), "yes" | "true" | "1")
    });

    let mut himayat: BTreeSet<String> = BTreeSet::new();
    for (miftah, qeema) in &azwaj {
        let munkhafid = qeema.to_ascii_lowercase();
        for (athar, ism) in ATHAR_HIMAYA {
            if munkhafid.contains(athar) {
                let _ = himayat.insert(ism.to_owned());
            }
        }
        if qeema.rsplit_once('.').is_some_and(|(_, imtidad)| imtidad.eq_ignore_ascii_case("exe"))
            && !TANFIDHIYAT_MUSTABADA.iter().any(|mustabad| munkhafid.contains(*mustabad))
        {
            // `relative` and `path` are the keys the executable lives under in
            // every shape of this file seen; anything else ending in `.exe` is
            // accepted after them rather than instead of them.
            if miftah == "relative" || miftah == "path" || miftah == "exe" {
                sijill.tanfidhiyat.insert(0, qeema.clone());
            } else {
                sijill.tanfidhiyat.push(qeema.clone());
            }
        }
        for (miftah_sura, dawr) in MAFATIH_SUWAR {
            if miftah == miftah_sura && !qeema.is_empty() {
                sijill.suwar.push((dawr, qeema.clone()));
            }
        }
    }
    sijill.himayat = himayat.into_iter().collect();
    sijill.tanfidhiyat.dedup();
    sijill
}

/// Splits a window into its `key: value` pairs.
///
/// Deliberately flat: indentation is not interpreted, because the framing that
/// would make indentation meaningful is the part of this format that is not
/// known. A line whose key is not a plain identifier is skipped, which is what
/// keeps a URL or a Windows path in a value from being read as a key.
fn azwaj_nafitha(nafitha: &str) -> Vec<(String, String)> {
    let mut azwaj = Vec::new();
    for satr in nafitha.lines() {
        let munazzam = satr.trim();
        let munazzam = munazzam.strip_prefix("- ").unwrap_or(munazzam);
        let Some((miftah, qeema)) = munazzam.split_once(':') else {
            continue;
        };
        let miftah = miftah.trim();
        if miftah.is_empty()
            || !miftah
                .chars()
                .all(|harf| harf.is_ascii_alphanumeric() || harf == '_' || harf == '-')
        {
            continue;
        }
        azwaj.push((miftah.to_ascii_lowercase(), bila_iqtibas(qeema)));
    }
    azwaj
}

/// Strips the quoting a YAML scalar may carry.
fn bila_iqtibas(qeema: &str) -> String {
    let munazzam = qeema.trim();
    let mukhaffaf = munazzam
        .strip_prefix('"')
        .and_then(|baqi| baqi.strip_suffix('"'))
        .or_else(|| munazzam.strip_prefix('\'').and_then(|baqi| baqi.strip_suffix('\'')))
        .unwrap_or(munazzam);
    mukhaffaf.trim().to_owned()
}

/// Whether a value is a localization key rather than a title.
fn huwa_miftah_lugha(qeema: &str) -> bool {
    let mut huruf = qeema.chars();
    huruf.next() == Some('l') && qeema.len() > 1 && huruf.all(|harf| harf.is_ascii_digit())
}

/// The images this record names, kept only when the cache really holds them.
fn suwar_mahalliya(usul: &Path, sijill: &SijillTashkeel) -> MasadirSuwar {
    let mut suwar = MasadirSuwar::default();
    for (dawr, ism) in &sijill.suwar {
        let maqbula = Path::new(ism)
            .extension()
            .and_then(|imtidad| imtidad.to_str())
            .is_some_and(|imtidad| {
                IMTIDADAT_SURA.iter().any(|maqbul| imtidad.eq_ignore_ascii_case(maqbul))
            });
        if !maqbula {
            continue;
        }
        let Ok(masar) = dakhil(usul, &ism.replace('\\', "/")) else {
            continue;
        };
        if !masar.is_file() {
            continue;
        }
        match dawr {
            DawrSura::Ghilaf if suwar.ghilaf.is_none() => {
                suwar.ghilaf = Some(MasdarSura::Malaf(masar));
            },
            DawrSura::Batl if suwar.batl.is_none() => suwar.batl = Some(MasdarSura::Malaf(masar)),
            DawrSura::Shiar if suwar.shiar.is_none() => {
                suwar.shiar = Some(MasdarSura::Malaf(masar));
            }
            DawrSura::Ghilaf | DawrSura::Batl | DawrSura::Shiar => {},
        }
    }
    suwar
}

/// Joins a record-supplied relative path onto an install root through the one
/// join that refuses to leave its root.
fn masar_dakhili(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munazzam = nisbi.replace('\\', "/");
    let munazzam = munazzam.trim_start_matches('/');
    dakhil(jidhr, munazzam).ok()
}

/// An install directory's own name, which is what Ubisoft names them after.
fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().trim().to_owned())
        .filter(|ism| !ism.is_empty())
}

// ---------------------------------------------------------------------------
// The Windows registry
// ---------------------------------------------------------------------------

/// Ubisoft Connect's installs, read from the registry.
///
/// Three views of the same key are tried because a launcher is not consistent
/// about which one it writes: the literal `WOW6432Node` path read without
/// redirection, the 32-bit view of the plain path (which the redirector maps to
/// `WOW6432Node`), and the 64-bit view of the plain path. Passing
/// `KEY_WOW64_32KEY` *and* naming `WOW6432Node` in the path would ask the
/// redirector to redirect an already-redirected path, so the two are never
/// combined.
#[cfg(windows)]
mod sijill {
    use std::path::PathBuf;

    use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_EXPAND_SZ,
        REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
        RegQueryValueExW,
    };
    use windows::core::{PCWSTR, PWSTR};

    use super::TathbeetUbisoft;

    /// The launcher's own key, in the three views it may live in.
    const MASARAT_LAUNCHER: [(&str, REG_SAM_FLAGS); 3] = [
        (r"SOFTWARE\WOW6432Node\Ubisoft\Launcher", KEY_WOW64_64KEY),
        (r"SOFTWARE\Ubisoft\Launcher", KEY_WOW64_32KEY),
        (r"SOFTWARE\Ubisoft\Launcher", KEY_WOW64_64KEY),
    ];

    /// Longest registry string this reader will accept, in bytes.
    const HADD_QEEMA: u32 = 64 * 1024;

    /// Registry key names are limited to 255 characters by the API itself.
    const HADD_ISM: usize = 512;

    /// A hard stop on enumeration, so a corrupt hive cannot spin.
    const HADD_MAFATIH: u32 = 4096;

    /// An open key that closes itself.
    struct Miftah(HKEY);

    impl Drop for Miftah {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful RegOpenKeyExW, is not
            // copied anywhere else, and is closed exactly once, here.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    /// A null-terminated wide string, alive for as long as the caller holds it.
    fn wide(nass: &str) -> Vec<u16> {
        nass.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Opens a key under `HKEY_LOCAL_MACHINE`.
    fn fath(masar: &str, ruya: REG_SAM_FLAGS) -> Option<Miftah> {
        let masar_w = wide(masar);
        let mut miftah = HKEY::default();
        // SAFETY: `masar_w` is a live, null-terminated wide string for the whole
        // call, and `miftah` is a live, correctly typed out-parameter.
        let natija = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                PCWSTR(masar_w.as_ptr()),
                None,
                KEY_READ | ruya,
                &raw mut miftah,
            )
        };
        (natija == ERROR_SUCCESS).then_some(Miftah(miftah))
    }

    /// Opens a subkey of an open key.
    fn fath_farii(walid: &Miftah, ism: &str, ruya: REG_SAM_FLAGS) -> Option<Miftah> {
        let ism_w = wide(ism);
        let mut miftah = HKEY::default();
        // SAFETY: `ism_w` is a live, null-terminated wide string for the whole
        // call, `walid.0` is an open key, and `miftah` is a live out-parameter.
        let natija = unsafe {
            RegOpenKeyExW(walid.0, PCWSTR(ism_w.as_ptr()), None, KEY_READ | ruya, &raw mut miftah)
        };
        (natija == ERROR_SUCCESS).then_some(Miftah(miftah))
    }

    /// Every immediate subkey name.
    fn mafatih_farya(walid: &Miftah) -> Vec<String> {
        let mut asmaa = Vec::new();
        let mut buffer = vec![0u16; HADD_ISM];
        let mut fahras: u32 = 0;
        while fahras < HADD_MAFATIH {
            let Ok(mut tul) = u32::try_from(buffer.len()) else {
                break;
            };
            // SAFETY: `buffer` holds `tul` u16 slots for the whole call, `tul`
            // is a live out-parameter, and every optional argument the call does
            // not need is passed as None rather than a dangling pointer.
            let natija = unsafe {
                RegEnumKeyExW(
                    walid.0,
                    fahras,
                    Some(PWSTR(buffer.as_mut_ptr())),
                    &raw mut tul,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if natija == ERROR_NO_MORE_ITEMS {
                break;
            }
            if natija == ERROR_MORE_DATA {
                buffer = vec![0u16; buffer.len().saturating_mul(2).min(1 << 16)];
                continue;
            }
            if natija != ERROR_SUCCESS {
                break;
            }
            let adad = usize::try_from(tul).unwrap_or(0).min(buffer.len());
            if let Some(harfiyat) = buffer.get(..adad) {
                let ism = String::from_utf16_lossy(harfiyat);
                let ism = ism.trim_end_matches('\0').trim().to_owned();
                if !ism.is_empty() {
                    asmaa.push(ism);
                }
            }
            fahras = fahras.saturating_add(1);
        }
        asmaa
    }

    /// A string value, or `None` when it is absent, too large, or not a string.
    fn qeema_nass(miftah: &Miftah, ism: &str) -> Option<String> {
        let ism_w = wide(ism);
        let mut naw = REG_VALUE_TYPE::default();
        let mut hajm: u32 = 0;
        // SAFETY: `ism_w` is a live, null-terminated wide string; `naw` and
        // `hajm` are live out-parameters; no data buffer is requested by this
        // first call, which is how the required size is learned.
        let natija = unsafe {
            RegQueryValueExW(
                miftah.0,
                PCWSTR(ism_w.as_ptr()),
                None,
                Some(&raw mut naw),
                None,
                Some(&raw mut hajm),
            )
        };
        if natija != ERROR_SUCCESS && natija != ERROR_MORE_DATA {
            return None;
        }
        if naw != REG_SZ && naw != REG_EXPAND_SZ {
            return None;
        }
        if hajm == 0 || hajm > HADD_QEEMA {
            return None;
        }

        let adad = usize::try_from(hajm).ok()?.div_ceil(2);
        let mut buffer = vec![0u16; adad];
        let mut hajm_mutah = hajm;
        // SAFETY: `buffer` is `adad` u16 slots, which is at least `hajm` bytes,
        // and `hajm_mutah` tells the call exactly that. The pointer is cast to
        // u8 because the API counts bytes; the allocation's alignment is that of
        // u16, which is stricter, so the write is in bounds and aligned.
        let natija = unsafe {
            RegQueryValueExW(
                miftah.0,
                PCWSTR(ism_w.as_ptr()),
                None,
                None,
                Some(buffer.as_mut_ptr().cast::<u8>()),
                Some(&raw mut hajm_mutah),
            )
        };
        if natija != ERROR_SUCCESS {
            return None;
        }
        let adad_harfiyat = usize::try_from(hajm_mutah).ok()?.div_euclid(2).min(buffer.len());
        let harfiyat = buffer.get(..adad_harfiyat)?;
        let tul = harfiyat.iter().position(|harf| *harf == 0).unwrap_or(harfiyat.len());
        let nass = String::from_utf16_lossy(harfiyat.get(..tul)?).trim().to_owned();
        (!nass.is_empty()).then_some(nass)
    }

    /// The launcher's own directory.
    pub(super) fn jidhr_launcher() -> Option<PathBuf> {
        MASARAT_LAUNCHER.into_iter().find_map(|(masar, ruya)| {
            let miftah = fath(masar, ruya)?;
            let qeema = qeema_nass(&miftah, "InstallDir")?;
            Some(PathBuf::from(qeema.replace('/', "\\")))
        })
    }

    /// Every install the launcher has registered.
    pub(super) fn tathbitat() -> Vec<TathbeetUbisoft> {
        let mut tathbitat: Vec<TathbeetUbisoft> = Vec::new();
        let mut maruf: Vec<u32> = Vec::new();

        for (jidhr_masar, ruya) in MASARAT_LAUNCHER {
            let masar = format!(r"{jidhr_masar}\Installs");
            let Some(walid) = fath(&masar, ruya) else {
                continue;
            };
            for ism in mafatih_farya(&walid) {
                let Ok(muarrif) = ism.trim().parse::<u32>() else {
                    continue;
                };
                if maruf.contains(&muarrif) {
                    continue;
                }
                let Some(miftah) = fath_farii(&walid, &ism, ruya) else {
                    continue;
                };
                let Some(masar_luba) = qeema_nass(&miftah, "InstallDir") else {
                    continue;
                };
                maruf.push(muarrif);
                // The launcher writes this value with forward slashes and a
                // trailing separator; both are normalized here so that the path
                // compares and joins like every other path in the product.
                let munazzam = masar_luba.replace('/', "\\");
                tathbitat.push(TathbeetUbisoft {
                    muarrif,
                    jidhr: PathBuf::from(munazzam.trim_end_matches('\\')),
                });
            }
        }
        tathbitat
    }
}
