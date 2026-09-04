//! متجر رايوت — the Riot client, and the one adapter written so that its games
//! are **refused** rather than patched.
//!
//! ```text
//! %PROGRAMDATA%\Riot Games\RiotClientInstalls.json
//! %PROGRAMDATA%\Riot Games\Metadata\<product>.<patchline>\
//!     <product>.<patchline>.product_settings.yaml
//! ```
//!
//! Windows only. Riot has never shipped a Linux client, and the macOS layout —
//! under `/Users/Shared/Riot Games` — has a different metadata arrangement that
//! this build has not verified. Guessing at it would produce an adapter that
//! finds nothing and reports success, which is worse than an adapter that says
//! it does not look there.
//!
//! # Why this adapter exists to say no
//!
//! Valorant and League of Legends ship **Riot Vanguard**, a kernel-mode
//! anti-cheat driver that loads at boot and inspects every process on the
//! machine. Modifying a Vanguard-protected game — replacing a font, injecting a
//! loader, editing a data file the client hashes — is not a compatibility
//! problem. It is a hardware ban on the player's account, applied to the
//! machine, and it is not appealable.
//!
//! So every product this adapter emits carries
//! [`SimatLuba::HimayaMuhtamala`] naming Riot Vanguard, the safety layer sees it
//! before the interface offers anything, and the refusal happens before a user
//! can click. The hint is deliberately conservative in the one direction that
//! matters: a Riot product this adapter's table does not recognise is treated as
//! protected too, because Riot has shipped Vanguard with every new title since
//! 2020 and the cost of being wrong the other way is somebody's account.
//!
//! The games are still discovered, still shown, and still carry their real
//! identity. A user is entitled to see that Taarib knows the game is installed
//! and is choosing not to touch it; a game that silently never appears looks
//! like a product that does not support it.
//!
//! # The settings files are not YAML, and are not parsed as YAML
//!
//! `<product>.<patchline>.product_settings.yaml` carries a handful of scalars —
//! an install path, a version, a patchline name. There is no YAML parser in this
//! workspace and none is added for six keys: a general YAML implementation is a
//! large amount of code that will happily interpret anchors, merge keys, typed
//! tags and multi-document streams, all of which are attack surface for a file
//! Taarib does not own and a launcher can rewrite at any time.
//!
//! What is implemented instead is [`hallil_idadat`], a strict line reader for
//! exactly the shape these files have. It is documented the way
//! `taarib-muhawwil-nusus`'s pickle reader is documented — by stating the
//! accepted subset in full and refusing everything else explicitly, rather than
//! by guessing at what an unsupported construct probably meant.
//!
//! ## The accepted subset
//!
//! ```text
//! line    := blank | comment | entry
//! blank   := ""                      (spaces only)
//! comment := "#" ...                 (at column zero)
//! entry   := key ":" value
//! key     := [A-Za-z_] [A-Za-z0-9_.-]*        at column zero, no indentation
//! value   := plain | "'" single "'" | '"' double '"'
//! ```
//!
//! * `plain` is any run of characters with a trailing ` #` comment removed and
//!   the result trimmed. It must not be empty.
//! * `single` is a single-quoted scalar in which `''` is a literal quote. It
//!   must close on the same line.
//! * `double` is a double-quoted scalar that contains **no backslash at all**.
//!   It must close on the same line.
//!
//! ## What is refused, and why refusing beats guessing
//!
//! Each of these records a [`RafdSatr`] naming the line and the reason, and the
//! line contributes nothing. The file's other keys still parse, so a settings
//! file that grows a nested section does not cost Taarib the install path.
//!
//! | construct | example | why it is refused |
//! | --- | --- | --- |
//! | any indentation | `  default_locale: en_US` | it belongs to a block whose owner this reader does not model, so its key is not the top-level key it looks like |
//! | a key with no value | `locale_data:` | it opens a nested block or a sequence; treating it as an empty string would invent a value |
//! | sequence items | `- en_US` | the same, one level down |
//! | anchors and aliases | `&base`, `*base` | resolving them means keeping a graph, and mis-resolving one silently substitutes the wrong path |
//! | tags | `!!str 3` | a type override this reader does not apply |
//! | block scalars | `\|`, `>` | multi-line values, which a line reader cannot terminate correctly |
//! | flow collections | `[a, b]`, `{a: 1}` | a nested value in a reader that returns scalars |
//! | an escape in a double-quoted value | `"C:\\Riot"` | YAML's escape rules are not C's, and a path decoded by the wrong rules points somewhere that exists |
//! | an unterminated quote | `'C:/Riot` | the value continues onto a line this reader has already refused |
//! | a second document | `---` | later documents can redefine every key, so keeping the first silently is wrong |
//! | a duplicate key | two `product_version` lines | the file disagrees with itself and neither answer is safe to prefer |
//!
//! `RiotClientInstalls.json` is genuine JSON and is read with `serde_json`,
//! because it is genuine JSON and there is no reason to hand-roll that.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::Value;
use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs, TanbihFahs,
};
use crate::khata::KhataKashf;

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "riot";

/// Platforms this adapter can find anything on. See the module note on macOS
/// and on the Linux client that does not exist.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The client's directory under the machine-wide application data root.
const MUJALLAD_MATJAR: &str = "Riot Games";

/// The directory holding one subdirectory per installed product and patchline.
const ISM_MUJALLAD_BAYANAT: &str = "Metadata";

/// The client's own install record, beside the metadata directory.
const ISM_SIJILL_TATHBEET: &str = "RiotClientInstalls.json";

/// What a product settings file's name ends with.
const LAHIQAT_IDADAT: &str = ".product_settings.yaml";

/// The anti-cheat Riot ships, named the way every other adapter in this crate
/// names it so the safety layer matches one string rather than four spellings.
const ISM_HIMAYA: &str = "Riot Vanguard";

/// The key carrying the directory the product is installed in.
const MIFTAH_MASAR: &str = "product_install_full_path";

/// The key carrying the directory the product's patchlines live under, used
/// when the full path is absent.
const MIFTAH_JIDHR: &str = "product_install_root";

/// The key carrying the installed build.
const MIFTAH_ISDAR: &str = "product_version";

/// The key carrying the patchline, which the directory name also encodes.
const MIFTAH_KHATT: &str = "product_patchline";

/// The key carrying the product slug, which the directory name also encodes.
const MIFTAH_SILAA: &str = "product_id";

/// The key carrying the name Riot puts on the desktop shortcut, which is the
/// closest thing to a display title these files hold.
const MIFTAH_UNWAN: &str = "shortcut_name";

/// The patchline preferred when a product has more than one installed.
const KHATT_MUFADDAL: &str = "live";

/// Metadata directories that are the client itself rather than a game.
const ASMAA_MUSTATHNAA: [&str; 3] = ["riot_client", "riotclient", "riot_client_dev"];

/// The largest settings file this reader will read, in bytes.
///
/// A real one is under two kilobytes. A megabyte is three orders of magnitude
/// of headroom and still bounds what a replaced file can make this process
/// allocate.
const HADD_HAJM_IDADAT: u64 = 1024 * 1024;

/// How many lines of one settings file are considered.
const HADD_SUTUR: usize = 4096;

/// How long a single line may be before it is refused.
///
/// A Windows path cannot approach this; a line that does is not one.
const HADD_TUL_SATR: usize = 8192;

/// How many metadata subdirectories are examined.
const HADD_MUJALLADAT: usize = 256;

/// The largest `RiotClientInstalls.json` this reader will read, in bytes.
const HADD_HAJM_SIJILL: u64 = 1024 * 1024;

/// One Riot product this adapter knows by name.
#[derive(Debug, Clone, Copy)]
struct MuntajRiot {
    /// The slug Riot uses in the metadata directory name, which is also the
    /// identity [`MasdarLuba::Riot`] carries.
    silaa: &'static str,
    /// The title as a player would recognise it.
    unwan: &'static str,
    /// Executables to look for under the install path, best first.
    tanfidhiyat: &'static [&'static str],
    /// Whether Riot ships Vanguard with this product today.
    vanguard: bool,
}

/// The four products Riot's client installs.
///
/// `bacon` is Legends of Runeterra; the slug is Riot's internal code name and
/// is what is actually written on disk, which is why it appears here rather
/// than anything a player would recognise.
///
/// The `vanguard` column is a statement about today, not a guarantee: Riot
/// added Vanguard to League of Legends in 2024, years after the client shipped,
/// and can add it to any product in a patch. A `false` here therefore only
/// means "Taarib has no reason to believe this one is protected"; the safety
/// layer still probes the install for anti-cheat evidence and still has the
/// final word. See [`himayat`] for what happens to a product not in this table.
const MUNTAJAT: [MuntajRiot; 4] = [
    MuntajRiot {
        silaa: "valorant",
        unwan: "VALORANT",
        tanfidhiyat: &[
            "VALORANT.exe",
            "live/VALORANT.exe",
            "ShooterGame/Binaries/Win64/VALORANT-Win64-Shipping.exe",
        ],
        vanguard: true,
    },
    MuntajRiot {
        silaa: "league_of_legends",
        unwan: "League of Legends",
        tanfidhiyat: &[
            "LeagueClient.exe",
            "Game/League of Legends.exe",
            "live/LeagueClient.exe",
        ],
        vanguard: true,
    },
    MuntajRiot {
        silaa: "bacon",
        unwan: "Legends of Runeterra",
        tanfidhiyat: &["LoR.exe", "live/LoR.exe", "Game/LoR.exe"],
        vanguard: false,
    },
    MuntajRiot {
        silaa: "wildrift",
        unwan: "League of Legends: Wild Rift",
        tanfidhiyat: &["WildRift.exe", "live/WildRift.exe"],
        vanguard: false,
    },
];

/// The Riot client.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarRiot;

impl MatjarRiot {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarRiot {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "رايوت"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Riot Games"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        let jidhr = jidhr_tilqai(siyaq);
        // Existence only; nothing is parsed here, because this runs before
        // every scan to decide whether to scan at all.
        (jidhr.join(ISM_MUJALLAD_BAYANAT).is_dir() || jidhr.join(ISM_SIJILL_TATHBEET).is_file())
            .then_some(jidhr)
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurQiraatFahras`] when the metadata directory
    /// exists and cannot be listed at all — a permissions failure on
    /// `%PROGRAMDATA%`, which makes the whole Riot catalogue unreadable and
    /// which the user can act on.
    ///
    /// Everything narrower degrades: a settings file that will not open, a file
    /// whose bytes are not UTF-8, a product whose install path is gone, a
    /// construct the settings reader refuses. Each becomes a [`TanbihFahs`] and
    /// the other products still arrive.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let jidhr = jidhr_tilqai(siyaq);
        let mujallad = jidhr.join(ISM_MUJALLAD_BAYANAT);
        let sijill = sijill_tathbeet(&jidhr.join(ISM_SIJILL_TATHBEET));
        if !mujallad.is_dir() && sijill.is_none() {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let mut natija = NatijatMatjar {
            matjar: MUARRIF,
            // The Riot Client's own directory when the install record names it,
            // because that is where the launcher actually lives; the machine's
            // `Riot Games` data folder is the fallback, and is what exists on a
            // machine whose install record was lost.
            jidhr_matjar: sijill
                .as_ref()
                .and_then(SijillTathbeet::jidhr_mushghil)
                .or_else(|| jidhr.is_dir().then(|| jidhr.clone())),
            ..NatijatMatjar::default()
        };

        let mut murashahat: Vec<MurashahMuntaj> = Vec::new();
        if mujallad.is_dir() {
            let madakhil = std::fs::read_dir(&mujallad).map_err(|sabab| {
                KhataKashf::TaadhurQiraatFahras {
                    matjar: MUARRIF,
                    masar: mujallad.clone(),
                    sabab,
                }
            })?;
            for madkhal in madakhil.take(HADD_MUJALLADAT).flatten() {
                match murashah_min_mujallad(&madkhal.path()) {
                    Ok(Some(murashah)) => murashahat.push(murashah),
                    Ok(None) => {},
                    Err(tanbih) => natija.tanbihat.push(tanbih),
                }
            }
        }

        // One product, several patchlines: `league_of_legends.live` and
        // `league_of_legends.pbe` are two installs of one game and share one
        // identity. Emitting both would put two cards in the library backed by
        // one registry key, and a patch would install into whichever the merge
        // step happened to keep. The preferred patchline wins and the others
        // are reported rather than dropped in silence.
        murashahat.sort_by(|awwal, thani| {
            rutbat_khatt(&awwal.khatt)
                .cmp(&rutbat_khatt(&thani.khatt))
                .then_with(|| awwal.khatt.cmp(&thani.khatt))
        });

        // Every install the metadata directory described, whether or not it
        // ends up in the library, so that a patchline folded away below is not
        // then reported a second time as an install nothing describes.
        let mawaqi: BTreeSet<String> = murashahat
            .iter()
            .map(|murashah| muwahhad(&murashah.jidhr, siyaq.nizam))
            .collect();

        let mut mustakhdam: BTreeMap<String, String> = BTreeMap::new();
        for murashah in &murashahat {
            if let Some(mubqa) = mustakhdam.get(&murashah.silaa) {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    murashah.masar_idadat.display().to_string(),
                    format!(
                        "the {} patchline of {} was skipped because its {mubqa} patchline is \
                         installed and both share one identity",
                        murashah.khatt, murashah.silaa
                    ),
                ));
                continue;
            }
            let _ = mustakhdam.insert(murashah.silaa.clone(), murashah.khatt.clone());
            if let Some(mudaad) = murashah.silaa_mudada.as_ref() {
                natija.tanbihat.push(TanbihFahs::jadeed(
                    MUARRIF,
                    murashah.masar_idadat.display().to_string(),
                    format!(
                        "this settings file declares {MIFTAH_SILAA} as {mudaad} while sitting in \
                         the folder for {}; the folder name was used, because it is what the \
                         client keys on, but one of the two is stale",
                        murashah.silaa
                    ),
                ));
            }
            match luba_min_murashah(murashah, siyaq.nizam) {
                Ok(luba) => natija.alaab.push(luba),
                Err(tanbih) => natija.tanbihat.push(tanbih),
            }
        }

        if let Some(sijill) = sijill.as_ref() {
            tanbih_ala_ghayr_mawsuf(sijill, &mawaqi, siyaq.nizam, &mut natija.tanbihat);
        }

        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let jidhr = jidhr_tilqai(siyaq);
        // Both: the metadata directory gains and loses a subdirectory when a
        // product is installed or removed, and `RiotClientInstalls.json` in the
        // parent is rewritten by the client when it repairs itself.
        [jidhr.join(ISM_MUJALLAD_BAYANAT), jidhr]
            .into_iter()
            .filter(|masar| masar.is_dir())
            .collect()
    }
}

/// The client's data root.
///
/// Machine-wide rather than per-user: Riot installs once for the whole machine,
/// and reading a per-user path would find nothing on the account that did not
/// run the installer.
fn jidhr_tilqai(siyaq: &SiyaqFahs) -> PathBuf {
    // The user's override wins outright, and is folded in here rather than
    // checked at each call site for the same reason it is in every other
    // adapter: a resolver callers have to remember to wrap is one they will not.
    siyaq
        .manassat
        .riot
        .clone()
        .unwrap_or_else(|| bayanat_barnamij().join(MUJALLAD_MATJAR))
}

/// Windows' machine-wide application data directory.
///
/// The environment is the only source for it here: it is not derived from the
/// user's home directory, and the shell API that would resolve the known folder
/// is outside the `windows` feature set this workspace declares.
fn bayanat_barnamij() -> PathBuf {
    std::env::var_os("PROGRAMDATA")
        .filter(|qeema| !qeema.is_empty())
        .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from)
}

// ---------------------------------------------------------------------------
// The metadata directory
// ---------------------------------------------------------------------------

/// One `<product>.<patchline>` directory, after its settings file was read.
#[derive(Debug, Clone)]
struct MurashahMuntaj {
    /// The product slug, which is the identity.
    silaa: String,
    /// The patchline — `live`, `pbe`, and whatever Riot adds next.
    khatt: String,
    /// The settings file this came from, named in every warning about it.
    masar_idadat: PathBuf,
    /// The install directory the settings file names.
    jidhr: PathBuf,
    /// The build, when the settings file records one.
    isdar: Option<String>,
    /// The shortcut name, which is the closest thing to a title on disk.
    unwan: Option<String>,
    /// The slug the settings file declares, when it disagrees with the folder
    /// it is in. `None` in the ordinary case where the two agree.
    silaa_mudada: Option<String>,
}

/// Reads one metadata subdirectory.
///
/// `Ok(None)` for a directory that is not a product — the client's own
/// metadata, or anything whose name does not carry a slug and a patchline.
/// Those are not faults and produce no warning.
fn murashah_min_mujallad(mujallad: &Path) -> Result<Option<MurashahMuntaj>, TanbihFahs> {
    if !mujallad.is_dir() {
        return Ok(None);
    }
    let Some(ism) = mujallad.file_name().and_then(|ism| ism.to_str()) else {
        return Ok(None);
    };
    // `league_of_legends.live` splits at the first dot, which is correct: a
    // product slug never contains one and a patchline never does either.
    let Some((silaa, khatt)) = ism.split_once('.') else {
        return Ok(None);
    };
    let silaa = silaa.trim().to_ascii_lowercase();
    let khatt = khatt.trim().to_ascii_lowercase();
    if silaa.is_empty() || khatt.is_empty() || ASMAA_MUSTATHNAA.contains(&silaa.as_str()) {
        return Ok(None);
    }

    let masar_idadat = mujallad.join(format!("{ism}{LAHIQAT_IDADAT}"));
    if !masar_idadat.is_file() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            mujallad.display().to_string(),
            "this Riot product has a metadata folder with no product settings file in it, so \
             there is no install path to read"
                .to_owned(),
        ));
    }

    let nass = iqra_mahdud(&masar_idadat, HADD_HAJM_IDADAT).map_err(|sabab| {
        TanbihFahs::jadeed(MUARRIF, masar_idadat.display().to_string(), sabab)
    })?;
    let idadat = hallil_idadat(&nass);

    let Some(jidhr) = idadat
        .qeema(MIFTAH_MASAR)
        .or_else(|| idadat.qeema(MIFTAH_JIDHR))
        .map(masar_min_qeema)
    else {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            masar_idadat.display().to_string(),
            format!(
                "no {MIFTAH_MASAR} and no {MIFTAH_JIDHR} in this product's settings{}",
                lahiqat_rafd(&idadat)
            ),
        ));
    };

    // The directory name is the identity, not `product_id` inside the file.
    // The directory is what the client itself keys on when it launches a
    // product, and a settings file copied between products during a repair —
    // which the client does — carries the wrong slug inside it. A disagreement
    // is worth saying out loud, because it means one of the two is stale.
    let mudaad = idadat
        .qeema(MIFTAH_SILAA)
        .map(|qeema| qeema.trim().to_ascii_lowercase())
        .filter(|qeema| !qeema.is_empty() && *qeema != silaa);
    let khatt_muallan = idadat
        .qeema(MIFTAH_KHATT)
        .map(|qeema| qeema.trim().to_ascii_lowercase())
        .filter(|qeema| !qeema.is_empty());

    Ok(Some(MurashahMuntaj {
        silaa,
        khatt: khatt_muallan.unwrap_or(khatt),
        masar_idadat,
        jidhr,
        isdar: idadat
            .qeema(MIFTAH_ISDAR)
            .map(|qeema| qeema.trim().to_owned())
            .filter(|qeema| !qeema.is_empty()),
        unwan: idadat
            .qeema(MIFTAH_UNWAN)
            .map(|qeema| qeema.trim().to_owned())
            .filter(|qeema| !qeema.is_empty()),
        silaa_mudada: mudaad,
    }))
}

/// A sentence naming the refused lines, appended to a warning so that a
/// settings file Taarib could not read fully explains itself in Diagnostics.
///
/// Empty when nothing was refused, so an ordinary failure — a key that is
/// genuinely absent — does not gain a misleading tail about parsing.
fn lahiqat_rafd(idadat: &IdadatMuntaj) -> String {
    let marfudat = idadat.marfudat();
    if marfudat.is_empty() {
        return String::new();
    }
    let tafsil: Vec<String> = marfudat
        .iter()
        .take(4)
        .map(|rafd| format!("line {}: {}", rafd.raqm, rafd.sabab))
        .collect();
    format!("; the reader refused {} line(s) — {}", marfudat.len(), tafsil.join("; "))
}

/// The order patchlines are preferred in.
///
/// `live` first because it is what a player actually plays. Everything else
/// sorts after it and then alphabetically, so the choice is the same on every
/// machine rather than depending on the order the filesystem listed directories
/// in.
fn rutbat_khatt(khatt: &str) -> u8 {
    match khatt {
        KHATT_MUFADDAL => 0,
        "pbe" => 1,
        _ => 2,
    }
}

/// Turns a settings value into a path.
///
/// Riot writes forward slashes even on Windows and sometimes leaves a trailing
/// one. Both are normalized here so that two spellings of one install do not
/// look like two installs.
fn masar_min_qeema(qeema: &str) -> PathBuf {
    let munazzam = qeema.trim().trim_end_matches(['/', '\\']);
    PathBuf::from(munazzam)
}

// ---------------------------------------------------------------------------
// One product becomes one game
// ---------------------------------------------------------------------------

/// Turns one metadata directory into a game, or into the warning that explains
/// why it is not in the library.
fn luba_min_murashah(
    murashah: &MurashahMuntaj,
    nizam: NizamTashghil,
) -> Result<LubaMuktashafa, TanbihFahs> {
    if !murashah.jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            murashah.jidhr.display().to_string(),
            "the Riot client still lists this product, but its install folder is gone; repair it \
             from the client or remove it"
                .to_owned(),
        ));
    }

    let maruf = MUNTAJAT.iter().find(|muntaj| muntaj.silaa == murashah.silaa);
    let ism = murashah
        .unwan
        .clone()
        .or_else(|| maruf.map(|muntaj| muntaj.unwan.to_owned()))
        .or_else(|| ism_min_mujallad(&murashah.jidhr))
        .unwrap_or_else(|| murashah.silaa.clone());

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Riot(murashah.silaa.clone()),
        hala_matjar: None,
        ism,
        jidhr: murashah.jidhr.clone(),
        tanfidhi: tanfidhi(&murashah.jidhr, maruf, nizam),
        // The settings files record no size, and measuring one would mean a
        // recursive walk of a thirty-gigabyte install on every scan.
        hajm: 0,
        bina_manassa: murashah.isdar.clone(),
        // Riot records neither an update nor a last-played timestamp anywhere
        // this adapter reads. The settings file's own modification time is not
        // either of them — the client rewrites it when the user changes a
        // setting — so nothing is claimed.
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        // Riot ships its artwork inside the client's own packed assets and
        // publishes no address for it. An empty result is what lets the
        // artwork pipeline fall back to the community registry's covers
        // instead of showing a broken image.
        suwar: MasadirSuwar::default(),
        khiyarat_tashghil: None,
        // The client writes a product's metadata directory when the install
        // finishes and removes it on uninstall. There is no partial state in
        // between that these files expose, so completeness is what the folder's
        // existence says and the existence gate does the rest.
        muktamila: true,
        simat: simat(maruf),
    })
}

/// What Riot's own layout says about a product that the safety layer needs.
///
/// Every Riot product is online-only, so [`SimatLuba::JamaiOnline`] is
/// unconditional and is not a guess. The anti-cheat hint comes from
/// [`himayat`].
fn simat(maruf: Option<&MuntajRiot>) -> Vec<SimatLuba> {
    let mut simat = vec![SimatLuba::JamaiOnline];
    if himayat(maruf) {
        simat.push(SimatLuba::HimayaMuhtamala(ISM_HIMAYA.to_owned()));
    }
    simat
}

/// Whether a product is treated as Vanguard-protected.
///
/// True for every product in [`MUNTAJAT`] whose `vanguard` column says so, and
/// **true for every product that is not in the table at all**.
///
/// That second clause is the important one and it is deliberate. Riot has
/// shipped Vanguard with every title it has released since 2020 and retrofitted
/// it onto League of Legends years after launch. A product this build has never
/// heard of is far more likely to be a new Riot game with Vanguard than an
/// unprotected one, and the two ways of being wrong are not symmetric: refusing
/// to patch a game that turns out to be safe costs a user a feature they can
/// ask for, and patching a game that turns out to be protected costs them their
/// account.
fn himayat(maruf: Option<&MuntajRiot>) -> bool {
    maruf.is_none_or(|muntaj| muntaj.vanguard)
}

/// The executable to launch, from the product table when the product is known.
///
/// `None` when nothing in the table resolves, which is a correct answer rather
/// than a failure: Riot launches its games through `RiotClientServices.exe`
/// with a product argument, so the per-product binary is a convenience for the
/// interface and not the launch path. No directory walk is done to find one —
/// a Valorant install is a hundred thousand files and this runs on every scan.
fn tanfidhi(jidhr: &Path, maruf: Option<&MuntajRiot>, nizam: NizamTashghil) -> Option<PathBuf> {
    let muntaj = maruf?;
    let imtidad = nizam.imtidad_tanfidh();
    for nisbi in muntaj.tanfidhiyat {
        // The table is written with the Windows extension because the client is
        // Windows-only; the check keeps a future platform from silently looking
        // for a `.exe` where none can exist.
        if !imtidad.is_empty() && !nisbi.to_ascii_lowercase().ends_with(imtidad) {
            continue;
        }
        if let Some(masar) = masar_dakhili(jidhr, nisbi).filter(|masar| masar.is_file()) {
            return Some(masar);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// RiotClientInstalls.json
// ---------------------------------------------------------------------------

/// What `RiotClientInstalls.json` records.
#[derive(Debug, Clone, Default)]
struct SijillTathbeet {
    /// The Riot Client executables the file names, in the order it names them.
    mushghilat: Vec<PathBuf>,
    /// The install directories the file associates with a client.
    masarat: Vec<PathBuf>,
}

impl SijillTathbeet {
    /// The directory the Riot Client itself is installed in.
    ///
    /// The record names an executable — `…\Riot Client\RiotClientServices.exe`
    /// — and the launcher root is its parent. Only a path that is really there
    /// is returned, so a stale record left by an uninstall does not report a
    /// launcher that is gone.
    fn jidhr_mushghil(&self) -> Option<PathBuf> {
        self.mushghilat
            .iter()
            .filter(|masar| masar.is_file())
            .find_map(|masar| masar.parent().map(Path::to_path_buf))
            .filter(|masar| masar.is_dir())
    }
}

/// Reads the client's install record.
///
/// Real JSON, read with `serde_json`, bounded in size like everything else this
/// adapter opens. `None` for a file that is absent or unreadable, which is not
/// a fault: a machine with the metadata directory and no install record still
/// has every product this adapter needs.
fn sijill_tathbeet(masar: &Path) -> Option<SijillTathbeet> {
    let nass = iqra_mahdud(masar, HADD_HAJM_SIJILL).ok()?;
    let qeema: Value = serde_json::from_str(bila_bom(&nass)).ok()?;
    let kaain = qeema.as_object()?;

    let mut sijill = SijillTathbeet::default();
    // `rc_default`, `rc_live`, `rc_beta`, `rc_esports`: one key per Riot Client
    // channel, each naming that channel's executable.
    for (miftah, qeema) in kaain {
        if !miftah.starts_with("rc_") {
            continue;
        }
        if let Some(nass) = qeema.as_str().map(str::trim).filter(|nass| !nass.is_empty()) {
            let masar = PathBuf::from(nass);
            if !sijill.mushghilat.contains(&masar) {
                sijill.mushghilat.push(masar);
            }
        }
    }
    // `associated_client` maps an install directory to the client that owns it.
    if let Some(marbut) = kaain.get("associated_client").and_then(Value::as_object) {
        for miftah in marbut.keys() {
            let munazzam = miftah.trim().trim_end_matches(['/', '\\']);
            if !munazzam.is_empty() {
                sijill.masarat.push(PathBuf::from(munazzam));
            }
        }
    }
    Some(sijill)
}

/// Warns about an install the client knows and the metadata directory did not
/// describe.
///
/// This is the case a scan would otherwise lose silently: `associated_client`
/// lists a directory, no `<product>.<patchline>` folder describes it, and the
/// game is simply absent from the library with no explanation anywhere. The
/// warning names the path so a user can see that Taarib found the install and
/// could not identify which product it is.
fn tanbih_ala_ghayr_mawsuf(
    sijill: &SijillTathbeet,
    mawaqi: &BTreeSet<String>,
    nizam: NizamTashghil,
    tanbihat: &mut Vec<TanbihFahs>,
) {
    for masar in &sijill.masarat {
        if !masar.is_dir() || mawaqi.contains(&muwahhad(masar, nizam)) {
            continue;
        }
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            masar.display().to_string(),
            "the Riot client lists an install here that no product settings file describes, so \
             Taarib cannot tell which product it is"
                .to_owned(),
        ));
    }
}

// ---------------------------------------------------------------------------
// The settings reader
// ---------------------------------------------------------------------------

/// One line the settings reader would not interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RafdSatr {
    /// The line number, counting from one, so it matches what an editor shows.
    pub raqm: usize,
    /// Why the line was refused, in a phrase.
    pub sabab: String,
}

/// The scalars one product settings file declares.
///
/// Keys are kept in the file's own spelling and matched case-sensitively,
/// because Riot writes them in one fixed lowercase form and a
/// case-insensitive match would let a hand-edited file define `Product_Version`
/// and have it silently win.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdadatMuntaj {
    /// Every accepted `key: value` pair.
    qeem: BTreeMap<String, String>,
    /// Every line that was refused, in file order.
    marfudat: Vec<RafdSatr>,
}

impl IdadatMuntaj {
    /// One value, by key.
    #[must_use]
    pub fn qeema(&self, miftah: &str) -> Option<&str> {
        self.qeem.get(miftah).map(String::as_str)
    }

    /// Every line the reader refused, in file order.
    #[must_use]
    pub fn marfudat(&self) -> &[RafdSatr] {
        &self.marfudat
    }

    /// How many keys were accepted.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.qeem.len()
    }
}

/// Reads a Riot product settings file.
///
/// The accepted subset and every refused construct are stated in full in the
/// module documentation, and this function implements exactly that and nothing
/// more. It never fails: a file where every line is refused yields an empty
/// [`IdadatMuntaj`] carrying one [`RafdSatr`] per line, and the caller turns
/// that into a warning naming the file.
///
/// Reading stops after `HADD_SUTUR` lines. A settings file has fewer than
/// twenty; a file with four thousand is not one, and a reader with no ceiling
/// is a reader a replaced file can make spin.
#[must_use]
pub fn hallil_idadat(nass: &str) -> IdadatMuntaj {
    let mut natija = IdadatMuntaj::default();

    for (fahras, satr_khaam) in bila_bom(nass).lines().take(HADD_SUTUR).enumerate() {
        let raqm = fahras.saturating_add(1);
        let satr = satr_khaam.trim_end_matches(['\r', '\n']);

        if satr.len() > HADD_TUL_SATR {
            natija.marfudat.push(rafd(raqm, "longer than a settings line may be"));
            continue;
        }
        if satr.trim().is_empty() {
            continue;
        }
        // A document marker ends the part of the file this reader trusts. A
        // later document may redefine every key, and silently keeping the first
        // one would answer a question the file did not settle. Reading stops
        // here rather than refusing every remaining line one at a time, so the
        // refusal list stays a list of distinct problems.
        if satr.starts_with("---") || satr.starts_with("...") {
            natija.marfudat.push(rafd(raqm, "a document marker; nothing after it is read"));
            break;
        }
        if satr.starts_with('#') {
            continue;
        }
        // YAML forbids a tab in indentation, and a file that has one is not
        // shaped the way this reader assumes even where it looks like it is.
        if satr.starts_with('\t') {
            natija.marfudat.push(rafd(raqm, "indented with a tab"));
            continue;
        }
        if satr.starts_with(' ') {
            natija.marfudat.push(rafd(raqm, "indented, so it belongs to a nested block"));
            continue;
        }
        if satr.starts_with("- ") || satr == "-" {
            natija.marfudat.push(rafd(raqm, "a sequence item, not a scalar key"));
            continue;
        }

        let Some((miftah, baqi)) = satr.split_once(':') else {
            natija.marfudat.push(rafd(raqm, "no key separator"));
            continue;
        };
        if !miftah_maqbul(miftah) {
            natija.marfudat.push(rafd(raqm, "the key is not a plain scalar name"));
            continue;
        }
        if baqi.trim().is_empty() {
            natija
                .marfudat
                .push(rafd(raqm, "the key opens a nested block rather than naming a value"));
            continue;
        }
        let qeema = match qeema_maqbula(baqi) {
            Ok(qeema) => qeema,
            Err(sabab) => {
                natija.marfudat.push(rafd(raqm, sabab));
                continue;
            },
        };
        if natija.qeem.contains_key(miftah) {
            natija.marfudat.push(rafd(raqm, "a key the file already defined"));
            continue;
        }
        let _ = natija.qeem.insert(miftah.to_owned(), qeema);
    }

    natija
}

/// Builds one refusal.
fn rafd(raqm: usize, sabab: &str) -> RafdSatr {
    RafdSatr { raqm, sabab: sabab.to_owned() }
}

/// Whether a key is a plain scalar name this reader accepts.
///
/// A letter or an underscore, then letters, digits, underscores, dots and
/// hyphens. That covers every key Riot writes and excludes the quoted keys,
/// complex keys and merge keys a general YAML parser would have to model.
fn miftah_maqbul(miftah: &str) -> bool {
    let mut huruf = miftah.chars();
    let Some(awwal) = huruf.next() else {
        return false;
    };
    if !awwal.is_ascii_alphabetic() && awwal != '_' {
        return false;
    }
    huruf.all(|harf| harf.is_ascii_alphanumeric() || matches!(harf, '_' | '.' | '-'))
}

/// Interprets the part of a line after the key separator.
///
/// # Errors
///
/// Returns the phrase naming the construct that was refused, which the caller
/// records as a [`RafdSatr`]. Every refusal in the module's table is produced
/// here except the ones about the line's shape, which are decided before the
/// value is looked at.
fn qeema_maqbula(baqi: &str) -> Result<String, &'static str> {
    let munazzam = baqi.trim();
    let Some(awwal) = munazzam.chars().next() else {
        return Err("an empty value");
    };

    match awwal {
        '\'' => {
            // A single-quoted scalar. `''` is a literal quote, and there is no
            // other escape in this form, which is why it is accepted at all.
            let dakhil = munazzam.get(1..).unwrap_or_default();
            let mut mabni = String::with_capacity(dakhil.len());
            let mut baqi_huruf = dakhil.chars().peekable();
            let mut ughliq = false;
            while let Some(harf) = baqi_huruf.next() {
                if harf != '\'' {
                    mabni.push(harf);
                    continue;
                }
                if baqi_huruf.peek() == Some(&'\'') {
                    let _ = baqi_huruf.next();
                    mabni.push('\'');
                    continue;
                }
                ughliq = true;
                break;
            }
            if !ughliq {
                return Err("an unterminated single-quoted value");
            }
            // Only whitespace or a comment may follow the closing quote;
            // anything else means the line is a construct this reader does not
            // model rather than a scalar it can read.
            let dhayl: String = baqi_huruf.collect();
            let dhayl = dhayl.trim();
            if !dhayl.is_empty() && !dhayl.starts_with('#') {
                return Err("trailing text after a quoted value");
            }
            if mabni.trim().is_empty() {
                return Err("an empty value");
            }
            Ok(mabni)
        },
        '"' => {
            // A double-quoted scalar. YAML's escape rules are not C's — `\x41`
            // is one byte here and two characters there, and `\/` is legal —
            // so rather than implement a second escape grammar for a file with
            // six keys in it, any backslash refuses the line.
            if munazzam.contains('\\') {
                return Err("an escape inside a double-quoted value");
            }
            let dakhil = munazzam.get(1..).unwrap_or_default();
            let Some(nihaya) = dakhil.find('"') else {
                return Err("an unterminated double-quoted value");
            };
            let mabni = dakhil.get(..nihaya).unwrap_or_default().to_owned();
            let dhayl = dakhil.get(nihaya.saturating_add(1)..).unwrap_or_default().trim();
            if !dhayl.is_empty() && !dhayl.starts_with('#') {
                return Err("trailing text after a quoted value");
            }
            if mabni.trim().is_empty() {
                return Err("an empty value");
            }
            Ok(mabni)
        },
        '&' => Err("an anchor"),
        '*' => Err("an alias"),
        '!' => Err("a tag"),
        '|' | '>' => Err("a block scalar spanning several lines"),
        '[' | ']' => Err("a flow sequence"),
        '{' | '}' => Err("a flow mapping"),
        '?' => Err("a complex mapping key"),
        _ => {
            // A plain scalar. A comment is only a comment when a space
            // precedes the hash, which is YAML's own rule and is what keeps a
            // path containing `#` from being truncated.
            let bila_taliq = munazzam.split_once(" #").map_or(munazzam, |(qabl, _)| qabl);
            let mabni = bila_taliq.trim();
            if mabni.is_empty() {
                return Err("an empty value");
            }
            Ok(mabni.to_owned())
        },
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Reads at most `hadd` bytes of a file as UTF-8 text.
///
/// Bounded because both files this adapter opens live in a directory any
/// installer can write, and a settings reader must not become a way to make
/// Taarib allocate a gigabyte.
///
/// Strict UTF-8, not lossy. Riot writes these files as UTF-8, and a lossy
/// conversion would turn a path this reader cannot represent into a path with
/// a replacement character in it — which resolves to nothing, silently, and
/// looks exactly like a game that is not installed.
///
/// # Errors
///
/// Returns the phrase the caller puts in a [`TanbihFahs`], naming what went
/// wrong with the file rather than with the parse.
fn iqra_mahdud(masar: &Path, hadd: u64) -> Result<String, String> {
    let malaf = std::fs::File::open(masar).map_err(|sabab| format!("cannot be opened: {sabab}"))?;
    let mut bayt = Vec::new();
    let _ = malaf
        .take(hadd)
        .read_to_end(&mut bayt)
        .map_err(|sabab| format!("cannot be read: {sabab}"))?;
    String::from_utf8(bayt)
        .map_err(|_| "is not valid UTF-8, so nothing in it can be trusted".to_owned())
}

/// Joins a relative path onto an install root through the one join that refuses
/// to leave its root.
fn masar_dakhili(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munazzam = nisbi.replace('\\', "/");
    let munazzam = munazzam.trim_start_matches('/');
    dakhil(jidhr, munazzam).ok()
}

/// An install directory's own name.
fn ism_min_mujallad(jidhr: &Path) -> Option<String> {
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().trim().to_owned())
        .filter(|ism| !ism.is_empty())
}

/// A path in the one form two paths can be compared in on this platform.
fn muwahhad(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy().replace('\\', "/");
    let nass = nass.trim_end_matches('/').to_owned();
    if nizam.hassas_lil_ahruf() { nass } else { nass.to_lowercase() }
}

/// Strips a byte order mark, which no JSON parser accepts and which a settings
/// file edited on Windows can gain.
fn bila_bom(nass: &str) -> &str {
    nass.strip_prefix('\u{feff}').unwrap_or(nass)
}
