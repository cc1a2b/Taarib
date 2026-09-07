//! متجر روكستار — the Rockstar Games Launcher, which keeps its whole catalogue
//! in the Windows registry and nowhere else.
//!
//! ```text
//! HKLM\SOFTWARE\WOW6432Node\Rockstar Games\<Title>      InstallFolder, Version
//! HKLM\SOFTWARE\WOW6432Node\Rockstar Games\Launcher     InstallFolder
//! HKCU\Software\Rockstar Games\<Title>                  InstallFolder, Version
//! ```
//!
//! There is no manifest directory, no JSON, no database. A title is installed
//! exactly when a key with its name exists and its `InstallFolder` points at a
//! real directory, and everything else this adapter reports — the executable,
//! the anti-cheat, whether the entry is a game at all — is derived from the key
//! name and from what is on disk under that folder.
//!
//! Windows only, and not because of an untested platform: the launcher has no
//! macOS or Linux build, and a Rockstar title running under Proton was bought
//! on Steam or on the Epic store and belongs to those adapters.
//!
//! # Three views of one key
//!
//! The launcher is not consistent about which registry view it writes, so three
//! machine-wide paths and two per-user paths are read and merged. The literal
//! `WOW6432Node` path is read through the 64-bit view, because naming
//! `WOW6432Node` *and* passing `KEY_WOW64_32KEY` asks the redirector to
//! redirect an already-redirected path and finds nothing at all. The plain path
//! is read through both views. The per-user hive is read last and only fills
//! gaps, because a title installed for one account and then repaired
//! machine-wide leaves a stale user key behind.
//!
//! Every handle opened here is wrapped in a guard whose `Drop` calls
//! `RegCloseKey`. Taarib is a desktop application that runs for hours and
//! rescans on every filesystem event; a registry handle leaked once per game
//! per scan is a handle leak that ends in a process that cannot open anything.
//!
//! # Identity: the numbers here are Taarib's
//!
//! [`MasdarLuba::Rockstar`] carries a `u32`, and that number is what the patch
//! registry shards on: `rockstar:1` has to mean Grand Theft Auto V on every
//! machine in the world or a patch published on one computer is invisible on
//! the next.
//!
//! Rockstar publishes no title-id list, and the launcher's own internal ids are
//! not written anywhere this adapter can read them on a machine where the
//! launcher is merely installed. So the numbers in `JADWAL_MUARRIFAT` are
//! **Taarib's own**, assigned once and frozen: the table is the contract, not a
//! cache of somebody else's identifiers. Changing an entry in it would orphan
//! every patch already published against the old number, which is why the table
//! is written out in full rather than generated.
//!
//! Three sources are consulted, in this order, and the order is the whole
//! design:
//!
//! 1. **The table**, matched on the key name with punctuation and spacing
//!    normalized away, so `Grand Theft Auto V`, `GTAV` and `Grand Theft Auto V`
//!    with a trailing space are one title. Deterministic on every machine.
//! 2. **A numeric id the key itself declares**, when Rockstar wrote one. This is
//!    Rockstar's own number, so two machines with the same title agree on it.
//! 3. **A hash of the normalized key name**, for a title released after this
//!    table was last written. See `muarrif_mushtaq`: the derived ids live in
//!    the top half of the `u32` range and can therefore never collide with a
//!    table entry, and they are identical on every machine because the key name
//!    is. A game the table does not know is still discovered, still shown, and
//!    still patchable — it is simply keyed by a number nobody has published a
//!    patch against yet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::dakhil;

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs, TanbihFahs,
};

/// The stable identifier, matching the registry's shard family.
const MUARRIF: &str = "rockstar";

/// Platforms this adapter can find anything on.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Windows];

/// The lowest identifier [`muarrif_mushtaq`] may produce.
///
/// The table below occupies small numbers and will never reach this, so a
/// derived identifier and a table identifier cannot collide however many titles
/// Rockstar ships.
const ASAS_MUARRIF_MUSHTAQ: u32 = 0x8000_0000;

/// Taarib's own title identifiers, keyed by the normalized registry key name.
///
/// Normalized by [`wahhid_ism`]: lowercased, with every character that is not
/// an ASCII letter or digit removed. That is what folds `L.A. Noire`,
/// `LA Noire` and `L.A.Noire` into one entry, and it is why several spellings
/// of the same title appear here mapping to one number.
///
/// The numbers are frozen. Adding a title means adding a row with the next free
/// number; changing a row means orphaning the patches published against it.
const JADWAL_MUARRIFAT: [(&str, u32); 24] = [
    ("grandtheftautov", 1),
    ("gtav", 1),
    ("grandtheftautovenhanced", 2),
    ("gtavenhanced", 2),
    ("grandtheftautoiv", 3),
    ("grandtheftautoivcompleteedition", 3),
    ("gtaivcompleteedition", 3),
    ("grandtheftautoiiidefinitiveedition", 4),
    ("gtaiiidefinitiveedition", 4),
    ("grandtheftautovicecitydefinitiveedition", 5),
    ("gtavicecitydefinitiveedition", 5),
    ("grandtheftautosanandreasdefinitiveedition", 6),
    ("gtasanandreasdefinitiveedition", 6),
    ("reddeadredemption", 7),
    ("reddeadredemption2", 8),
    ("maxpayne3", 9),
    ("lanoire", 10),
    ("lanoirecompleteedition", 10),
    ("bully", 11),
    ("bullyscholarshipedition", 11),
    ("manhunt", 12),
    ("midnightclubii", 13),
    ("midnightclub2", 13),
    ("grandtheftautovlegacy", 14),
];

/// Key names that are the launcher's own machinery rather than a game.
///
/// Normalized the same way as [`JADWAL_MUARRIFAT`]. These carry an
/// `InstallFolder` exactly like a title does, and emitting them would put a
/// card in somebody's library for the launcher itself and another for the
/// Social Club runtime, both pointing at directories with no game in them.
const ASMAA_MUSTATHNAA: [&str; 7] = [
    "launcher",
    "rockstargameslauncher",
    "socialclub",
    "rockstargamessocialclub",
    "rockstargameslibrary",
    "rockstargamessubscription",
    "rockstargamesservices",
];

/// The executables each known title is launched by, relative to its install
/// folder, best first.
///
/// Rockstar records no executable anywhere in the registry, so this is the only
/// way to name one without walking a fifty-thousand-file game directory. The
/// launcher shim — `PlayGTAV.exe`, `PlayRDR2.exe` — is listed first on purpose:
/// it is what the launcher itself runs, and running the raw binary instead
/// skips the entitlement check and produces a game that closes on startup.
const TANFIDHIYAT: [(&str, &[&str]); 13] = [
    ("grandtheftautov", &["PlayGTAV.exe", "GTA5.exe", "GTAVLauncher.exe"]),
    ("grandtheftautovenhanced", &["PlayGTAV.exe", "GTA5_Enhanced.exe", "GTA5.exe"]),
    ("grandtheftautoiv", &["PlayGTAIV.exe", "GTAIV.exe", "LaunchGTAIV.exe"]),
    (
        "grandtheftautoiiidefinitiveedition",
        &["Gameface/Binaries/Win64/LibertyCity.exe", "PlayGTAIII.exe"],
    ),
    (
        "grandtheftautovicecitydefinitiveedition",
        &["Gameface/Binaries/Win64/ViceCity.exe", "PlayGTAVC.exe"],
    ),
    (
        "grandtheftautosanandreasdefinitiveedition",
        &["Gameface/Binaries/Win64/SanAndreas.exe", "PlayGTASA.exe"],
    ),
    ("reddeadredemption", &["RDR.exe", "PlayRDR.exe"]),
    ("reddeadredemption2", &["PlayRDR2.exe", "RDR2.exe"]),
    ("maxpayne3", &["MaxPayne3.exe", "PlayMP3.exe"]),
    ("lanoire", &["LANoire.exe", "PlayLANoire.exe"]),
    ("bully", &["Bully.exe"]),
    ("manhunt", &["manhunt.exe"]),
    ("midnightclubii", &["mc2.exe"]),
];

/// Titles whose online mode the safety layer needs to know about.
const JAMAI_ONLINE: [&str; 6] = [
    "grandtheftautov",
    "grandtheftautovenhanced",
    "grandtheftautoiv",
    "reddeadredemption2",
    "maxpayne3",
    "midnightclubii",
];

/// Titles that ship a kernel-level anti-cheat, and which one.
///
/// Grand Theft Auto V's online mode has shipped `BattlEye` since 2022 and the
/// Enhanced release carries it too. Naming it here lets the interface warn
/// before the user clicks; Phase 16 still refuses on its own evidence, and this
/// only makes the refusal immediate.
const HIMAYAT: [(&str, &str); 2] =
    [("grandtheftautov", "BattlEye"), ("grandtheftautovenhanced", "BattlEye")];

/// How many files are examined at an install root when the executable table
/// does not know the title.
const HADD_MALAFFAT: usize = 512;

/// File-name fragments that are never the game's own executable.
///
/// Every Rockstar install ships a redistributable directory and an uninstaller,
/// and a probe that returned one of those would give the library a card that
/// launches an installer.
const ATHAR_LAYSA_LUBA: [&str; 8] = [
    "unins",
    "setup",
    "redist",
    "vcredist",
    "directx",
    "dxsetup",
    "crashhandler",
    "launcherpatcher",
];

/// The Rockstar Games Launcher.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarRockstar;

impl MatjarRockstar {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarRockstar {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "روكستار"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Rockstar Games"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return None;
        }
        // The launcher's own key first. Failing that, the directory a title
        // was installed *into* — `…\Rockstar Games\Grand Theft Auto V` sits
        // under `…\Rockstar Games`, which is as close to a launcher root as a
        // machine without the launcher key has. Returning `None` here instead
        // would tell the scan orchestrator there is nothing to look at, and a
        // player who removed the launcher and kept the game would lose it.
        // The user's override first: Rockstar keeps its catalogue in the
        // registry, so an override cannot point at a catalogue the way it does
        // for every other adapter. It names the launcher's own folder, which is
        // what the "open the launcher at this game" button needs on a machine
        // whose registry does not record it.
        siyaq
            .manassat
            .rockstar
            .clone()
            .filter(|masar| masar.is_dir())
            .or_else(|| jidhr_mushghil().filter(|masar| masar.is_dir()))
            .or_else(jidhr_ayy_luba)
    }

    /// # Errors
    ///
    /// Never. This adapter has no whole-catalogue failure to report, and that
    /// is a property of the registry rather than an omission: a hive that
    /// cannot be opened and a hive with no `Rockstar Games` key in it are
    /// indistinguishable from outside, both mean "no Rockstar titles here", and
    /// raising an error for the first would mean raising it on every machine
    /// that simply does not have the launcher.
    ///
    /// Everything narrower degrades as it does everywhere else: a key whose
    /// folder is gone, a key the launcher marks uninstalled, and a key whose
    /// name carries no identity each become a [`TanbihFahs`] on the result, and
    /// the rest of the library still arrives.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let mushghil = jidhr_mushghil();
        let madakhil = madakhil_sijill();
        if mushghil.is_none() && madakhil.is_empty() {
            // The catalogue is the registry, and it names nothing. A configured
            // folder that exists is still the user saying the launcher is here,
            // which `mawqi` honours; so this answers "installed, unreadable"
            // rather than contradicting it with "not installed".
            let tajawuz = siyaq.manassat.rockstar.clone().filter(|masar| masar.is_dir());
            let Some(tajawuz) = tajawuz else {
                return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
            };
            return Ok(NatijatMatjar::naqisa(
                MUARRIF,
                Some(tajawuz.clone()),
                tajawuz.display().to_string(),
                "the configured Rockstar folder exists, but the registry — where the Rockstar \
                 launcher keeps its catalogue — names neither the launcher nor any title, so \
                 nothing could be listed from it",
            ));
        }

        let mut natija =
            NatijatMatjar::muthabbat(MUARRIF, mushghil.filter(|masar| masar.is_dir()));

        let mut fahras: BTreeMap<u32, LubaMuktashafa> = BTreeMap::new();
        for madkhal in madakhil {
            match luba_min_madkhal(&madkhal, siyaq) {
                Ok(luba) => damm(&mut fahras, luba),
                Err(tanbih) => natija.tanbihat.push(tanbih),
            }
        }

        natija.alaab = fahras.into_values().collect();
        natija.alaab.sort_by(|awwal, thani| awwal.ism.cmp(&thani.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    /// The registry cannot be watched by a filesystem watcher, so what is
    /// returned is the *parent* of every install folder — the directory a new
    /// Rockstar install appears inside.
    ///
    /// That is a weaker signal than a launcher with a manifest directory gives:
    /// it fires when a game is installed or removed next to one that already
    /// exists, and it does not fire for the first install on a machine or for a
    /// title installed onto a drive nothing is yet installed on. The interface
    /// always offers a manual rescan for exactly this reason, and this adapter
    /// is the reason it has to.
    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        if !MANASSAT.contains(&siyaq.nizam) {
            return Vec::new();
        }
        let mut murashahat: Vec<PathBuf> = madakhil_sijill()
            .into_iter()
            .filter_map(|madkhal| madkhal.jidhr.parent().map(Path::to_path_buf))
            .collect();
        if let Some(mushghil) = jidhr_mushghil() {
            murashahat.push(mushghil);
        }

        let mut judhur: Vec<PathBuf> = Vec::new();
        let mut maruf: Vec<String> = Vec::new();
        for masar in murashahat {
            if !masar.is_dir() {
                continue;
            }
            let miftah = muwahhad(&masar, siyaq.nizam);
            if !maruf.contains(&miftah) {
                maruf.push(miftah);
                judhur.push(masar);
            }
        }
        judhur
    }
}

// ---------------------------------------------------------------------------
// What the registry holds
// ---------------------------------------------------------------------------

/// One `Rockstar Games\<Title>` key, in the form the rest of this module reads.
///
/// Compiled on every platform even though only Windows can produce one, so that
/// nothing below is behind a `cfg` and a change to this module cannot compile
/// on Linux and fail on Windows.
#[derive(Debug, Clone)]
struct MadkhalSijill {
    /// The key's own name, verbatim, which is the only title Rockstar records.
    ism: String,
    /// The installation root the key names.
    jidhr: PathBuf,
    /// The version string, when the key carries one.
    isdar: Option<String>,
    /// A numeric title id the key declares, when Rockstar wrote one.
    muarrif_musajjal: Option<u32>,
    /// The installed flag, when the key carries one. `None` means the key does
    /// not say, which is the common case and is treated as installed — the key
    /// existing at all is Rockstar's way of saying so.
    mansub: Option<bool>,
}

/// Every Rockstar title key the registry holds, merged across all five views.
///
/// Empty on every platform but Windows, where the registry is the only place
/// the launcher records anything at all.
#[cfg_attr(
    not(windows),
    expect(
        clippy::missing_const_for_fn,
        reason = "on Windows this reads the registry; only the non-Windows body is a constant"
    )
)]
fn madakhil_sijill() -> Vec<MadkhalSijill> {
    #[cfg(windows)]
    {
        sijill::alaab_rockstar()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// The launcher's own installation root, from its `Launcher` key.
#[cfg_attr(
    not(windows),
    expect(
        clippy::missing_const_for_fn,
        reason = "on Windows this reads the registry; only the non-Windows body is a constant"
    )
)]
fn jidhr_mushghil() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        sijill::jidhr_mushghil()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// The directory the first Rockstar title found is installed *into*.
///
/// The parent of a title's install folder, which on a stock machine is
/// `…\Rockstar Games`. Used by [`Matjar::mawqi`] on a machine where the
/// launcher's own key is absent, so that removing the launcher and keeping the
/// games does not make the whole source look uninstalled.
///
/// Stops at the first title with a real directory rather than enumerating them
/// all, because `mawqi` runs before every scan and is meant to be an existence
/// check rather than a catalogue read.
#[cfg_attr(
    not(windows),
    expect(
        clippy::missing_const_for_fn,
        reason = "on Windows this reads the registry; only the non-Windows body is a constant"
    )
)]
fn jidhr_ayy_luba() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        sijill::jidhr_ayy_luba()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Normalizes a registry key name for table lookup.
///
/// Lowercases and drops everything that is not an ASCII letter or digit, which
/// folds the punctuation and spacing Rockstar is inconsistent about:
/// `L.A. Noire`, `LA Noire` and `L.A.Noire` are one title, and so are
/// `Grand Theft Auto V` and `Grand Theft Auto  V`.
///
/// Non-ASCII characters are dropped rather than transliterated. A Rockstar key
/// name has never contained one, and dropping is the behaviour that keeps the
/// result identical on every machine — a locale-dependent lowercasing of a
/// non-ASCII character would produce a different identifier on a Turkish
/// system, which is exactly the class of bug this normalization exists to stop.
fn wahhid_ism(ism: &str) -> String {
    ism.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|harf| harf.to_ascii_lowercase())
        .collect()
}

/// The identifier for a key name: the table, then the registry, then a hash.
///
/// A number Rockstar itself wrote is only used when it is not zero and is not
/// already spoken for by [`JADWAL_MUARRIFAT`]. The second condition is the one
/// that matters: Rockstar's own numbering and Taarib's are unrelated sequences
/// that both start small, so a title whose key declares `TitleId 1` would
/// otherwise be filed as Grand Theft Auto V and inherit its patches. A
/// collision falls through to the hash, which cannot collide with anything.
fn muarrif_luba(madkhal: &MadkhalSijill) -> u32 {
    let muwahhad = wahhid_ism(&madkhal.ism);
    if let Some((_, raqm)) = JADWAL_MUARRIFAT.iter().find(|(ism, _)| *ism == muwahhad) {
        return *raqm;
    }
    let musajjal = madkhal.muarrif_musajjal.filter(|raqm| {
        *raqm > 0
            && *raqm < ASAS_MUARRIF_MUSHTAQ
            && !JADWAL_MUARRIFAT.iter().any(|(_, mahjuz)| *mahjuz == *raqm)
    });
    if let Some(raqm) = musajjal {
        return raqm;
    }
    muarrif_mushtaq(&muwahhad)
}

/// A stable identifier for a title the table does not know.
///
/// The first four bytes of a BLAKE3 hash of the normalized key name, with the
/// top bit forced on so the result always lands in
/// `0x8000_0000..=0xFFFF_FFFF` — a range no table entry can ever occupy.
///
/// This is deliberately a hash and not a counter. A counter would depend on the
/// order the registry happened to enumerate keys in, so the same game would get
/// a different number on two machines and every patch published for it would be
/// invisible to everybody else. A hash of the key name is identical everywhere
/// the key name is, which is everywhere Rockstar installed the title.
///
/// The trade is stated rather than hidden: a title released after this table
/// was written gets a number that is stable but arbitrary, and if it is later
/// added to [`JADWAL_MUARRIFAT`] its identifier changes and any patch published
/// against the derived number has to be republished. That is why the table
/// covers everything the launcher has shipped, and why a new title should be
/// added to it rather than left to the hash.
fn muarrif_mushtaq(muwahhad: &str) -> u32 {
    let basma = blake3::hash(muwahhad.as_bytes());
    let arbaa = basma
        .as_bytes()
        .get(..4)
        .and_then(|juz| <[u8; 4]>::try_from(juz).ok())
        .unwrap_or([0u8; 4]);
    u32::from_le_bytes(arbaa) | ASAS_MUARRIF_MUSHTAQ
}

// ---------------------------------------------------------------------------
// One key becomes one game
// ---------------------------------------------------------------------------

/// Turns one registry key into a game, or into the warning that explains why it
/// is not in the library.
fn luba_min_madkhal(
    madkhal: &MadkhalSijill,
    siyaq: &SiyaqFahs,
) -> Result<LubaMuktashafa, TanbihFahs> {
    let muwahhad = wahhid_ism(&madkhal.ism);
    if muwahhad.is_empty() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            madkhal.ism.clone(),
            "this Rockstar registry key has a name with nothing in it a title could be derived \
             from, so it has no identity a patch could be published against"
                .to_owned(),
        ));
    }
    if ASMAA_MUSTATHNAA.contains(&muwahhad.as_str()) {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            madkhal.ism.clone(),
            "this is the Rockstar launcher's own registry key, not a game, and was skipped"
                .to_owned(),
        ));
    }
    if madkhal.mansub == Some(false) {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            madkhal.ism.clone(),
            "the Rockstar launcher records this title as owned but not installed".to_owned(),
        ));
    }
    if !madkhal.jidhr.is_dir() {
        return Err(TanbihFahs::jadeed(
            MUARRIF,
            madkhal.jidhr.display().to_string(),
            "the Rockstar launcher still lists this game, but its install folder is gone; \
             reinstall it or let the launcher repair it"
                .to_owned(),
        ));
    }

    let mut simat = Vec::new();
    if JAMAI_ONLINE.contains(&muwahhad.as_str()) {
        simat.push(SimatLuba::JamaiOnline);
    }
    for (ism, himaya) in HIMAYAT {
        if ism == muwahhad {
            simat.push(SimatLuba::HimayaMuhtamala((*himaya).to_owned()));
        }
    }
    for himaya in himayat_ala_al_qurs(&madkhal.jidhr) {
        let sima = SimatLuba::HimayaMuhtamala(himaya);
        if !simat.contains(&sima) {
            simat.push(sima);
        }
    }

    Ok(LubaMuktashafa {
        masdar: MasdarLuba::Rockstar(muarrif_luba(madkhal)),
        hala_matjar: None,
        ism: unwan(&madkhal.ism, &madkhal.jidhr),
        jidhr: madkhal.jidhr.clone(),
        tanfidhi: tanfidhi(&madkhal.jidhr, &muwahhad, siyaq.nizam),
        // The registry records no size. Measuring one would mean a recursive
        // walk of a hundred-gigabyte install on every scan, so the honest
        // answer is zero and the interface shows nothing rather than a number
        // it made up.
        hajm: 0,
        bina_manassa: madkhal.isdar.clone(),
        // Nor an update or last-played timestamp. The key's own write time is
        // not either of those — the launcher rewrites it on a repair and on a
        // settings change — so nothing is claimed.
        akhir_tahdith: None,
        akhir_laab: None,
        beea: BeeatTawafuq::Asli,
        // Rockstar caches its artwork inside the launcher's own resources in a
        // packed format nothing here can read, and publishes no address for it.
        // An empty result is what lets the artwork pipeline fall back to the
        // registry's own covers instead of showing a broken image.
        suwar: MasadirSuwar::default(),
        khiyarat_tashghil: None,
        // The launcher writes a title's key when the install finishes and
        // removes it when the game is uninstalled; there is no partial state in
        // between that the registry exposes. See the module note on what that
        // means for the existence gate.
        muktamila: true,
        simat,
    })
}

/// Files a game, preferring the entry that already resolved an executable.
///
/// The same title appears in both the machine-wide and the per-user hive on a
/// machine where it was installed for one account and later repaired for all of
/// them. Both keys describe one install, and emitting both would put two cards
/// in the library over one directory.
fn damm(fahras: &mut BTreeMap<u32, LubaMuktashafa>, luba: LubaMuktashafa) {
    let muarrif = match &luba.masdar {
        MasdarLuba::Rockstar(raqm) => *raqm,
        _ => return,
    };
    match fahras.get_mut(&muarrif) {
        Some(mawjud) => {
            if mawjud.tanfidhi.is_none() && luba.tanfidhi.is_some() {
                mawjud.tanfidhi = luba.tanfidhi;
                mawjud.jidhr = luba.jidhr;
            }
            if mawjud.bina_manassa.is_none() {
                mawjud.bina_manassa = luba.bina_manassa;
            }
            for sima in luba.simat {
                if !mawjud.simat.contains(&sima) {
                    mawjud.simat.push(sima);
                }
            }
        },
        None => {
            let _ = fahras.insert(muarrif, luba);
        },
    }
}

/// The name shown in the library.
///
/// The registry key name is used verbatim, because it is the only title
/// Rockstar records and it is already the title a player would recognise. The
/// install directory's own name is the fallback for a key whose name is empty
/// after trimming, which a hand-edited hive can produce.
fn unwan(ism: &str, jidhr: &Path) -> String {
    let munazzam = ism.trim();
    if !munazzam.is_empty() {
        return munazzam.to_owned();
    }
    jidhr
        .file_name()
        .map(|ism| ism.to_string_lossy().trim().to_owned())
        .filter(|ism| !ism.is_empty())
        .unwrap_or_else(|| "Rockstar Games".to_owned())
}

/// The executable to launch, from the table when the title is known and from a
/// bounded probe of the install root when it is not.
fn tanfidhi(jidhr: &Path, muwahhad: &str, nizam: NizamTashghil) -> Option<PathBuf> {
    if let Some((_, murashahat)) = TANFIDHIYAT.iter().find(|(ism, _)| *ism == muwahhad) {
        for nisbi in *murashahat {
            if let Some(masar) = masar_dakhili(jidhr, nisbi).filter(|masar| masar.is_file()) {
                return Some(masar);
            }
        }
    }
    tanfidhi_mustanbat(jidhr, nizam)
}

/// Finds an executable at an install root for a title the table does not know.
///
/// Only the root itself is read — no recursion — and only the first
/// [`HADD_MALAFFAT`] entries, because a probe that walked a Red Dead Redemption
/// 2 install would read a hundred thousand files on every scan. A name matching
/// [`ATHAR_LAYSA_LUBA`] is skipped, and the largest remaining executable wins:
/// a game's own binary is invariably far larger than the crash handler and the
/// activation shim that sit beside it.
///
/// `None` when nothing qualifies, which is a correct and common answer. Phase 5
/// probes the directory properly and does not need this to have succeeded.
fn tanfidhi_mustanbat(jidhr: &Path, nizam: NizamTashghil) -> Option<PathBuf> {
    let imtidad = nizam.imtidad_tanfidh();
    let mut afdal: Option<(u64, PathBuf)> = None;
    for madkhal in std::fs::read_dir(jidhr).ok()?.take(HADD_MALAFFAT).flatten() {
        let masar = madkhal.path();
        if !masar.is_file() {
            continue;
        }
        let mutabiq = masar
            .extension()
            .and_then(|qeema| qeema.to_str())
            .is_some_and(|qeema| qeema.eq_ignore_ascii_case(imtidad));
        if !mutabiq {
            continue;
        }
        let Some(ism) = masar.file_stem().and_then(|ism| ism.to_str()).map(str::to_lowercase)
        else {
            continue;
        };
        if ATHAR_LAYSA_LUBA.iter().any(|athar| ism.contains(athar)) {
            continue;
        }
        let hajm = madkhal.metadata().map_or(0, |bayanat| bayanat.len());
        let abaad = afdal.as_ref().is_none_or(|(sabiq, _)| hajm > *sabiq);
        if abaad {
            afdal = Some((hajm, masar));
        }
    }
    afdal.map(|(_, masar)| masar)
}

/// Anti-cheat products whose own directory sits at a Rockstar install root.
///
/// Corroborates [`HIMAYAT`] and covers a title the table does not list. Only
/// the root is examined and only for a directory name, which costs three
/// metadata calls; anything deeper is Phase 16's job and is done properly
/// there, against file signatures rather than against a folder name.
fn himayat_ala_al_qurs(jidhr: &Path) -> Vec<String> {
    let mut wujida = Vec::new();
    for (athar, ism) in [
        ("BattlEye", "BattlEye"),
        ("EasyAntiCheat", "EasyAntiCheat"),
        ("EasyAntiCheat_EOS", "EasyAntiCheat"),
    ] {
        if jidhr.join(athar).is_dir() && !wujida.iter().any(|mawjud| mawjud == ism) {
            wujida.push(ism.to_owned());
        }
    }
    wujida
}

/// Joins a relative path onto an install root through the one join that refuses
/// to leave its root.
fn masar_dakhili(jidhr: &Path, nisbi: &str) -> Option<PathBuf> {
    let munazzam = nisbi.replace('\\', "/");
    let munazzam = munazzam.trim_start_matches('/');
    dakhil(jidhr, munazzam).ok()
}

/// A path in the one form two paths can be compared in on this platform.
fn muwahhad(masar: &Path, nizam: NizamTashghil) -> String {
    let nass = masar.to_string_lossy().replace('\\', "/");
    let nass = nass.trim_end_matches('/').to_owned();
    if nizam.hassas_lil_ahruf() { nass } else { nass.to_lowercase() }
}

// ---------------------------------------------------------------------------
// The Windows registry
// ---------------------------------------------------------------------------

/// Reading `Rockstar Games` out of the registry, in every view it is written to.
#[cfg(windows)]
mod sijill {
    use std::path::PathBuf;

    use windows::Win32::Foundation::{ERROR_MORE_DATA, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        REG_DWORD, REG_EXPAND_SZ, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE, RegCloseKey,
        RegEnumKeyExW, RegOpenKeyExW, RegQueryValueExW,
    };
    use windows::core::{PCWSTR, PWSTR};

    use super::MadkhalSijill;

    /// The key name whose `InstallFolder` is the launcher's own root.
    const ISM_MUSHGHIL: &str = "Launcher";

    /// Value names that may carry an installation directory, best first.
    const QEEM_MASAR: [&str; 4] =
        ["InstallFolder", "InstallLocation", "InstallDir", "Install Folder"];

    /// Value names that may carry a version string.
    const QEEM_ISDAR: [&str; 4] = ["Version", "PatchVersion", "InstalledVersion", "GameVersion"];

    /// Value names that may carry Rockstar's own numeric title id.
    const QEEM_MUARRIF: [&str; 4] = ["TitleId", "titleId", "RockstarTitleId", "GameId"];

    /// Value names that may carry an installed flag.
    const QEEM_MANSUB: [&str; 2] = ["Installed", "IsInstalled"];

    /// Longest registry string this reader will accept, in bytes. Registry
    /// values may be far larger; an installation path is not.
    const HADD_QEEMA: u32 = 64 * 1024;

    /// Registry key names are limited to 255 characters by the API itself; the
    /// buffer starts at twice that and grows only on `ERROR_MORE_DATA`.
    const HADD_ISM: usize = 512;

    /// A hard stop on enumeration, so a corrupt or hostile hive cannot spin.
    const HADD_MAFATIH: u32 = 4096;

    /// The five places the launcher writes a title key.
    ///
    /// A function rather than a `const` so the `HKEY` roots are read from the
    /// crate's own constants at the point of use, and so the table can be
    /// reordered without thinking about const evaluation of a handle type.
    ///
    /// Order is precedence: machine-wide first, per-user last, because a title
    /// installed for one account and later repaired machine-wide leaves a stale
    /// per-user key that must not win.
    fn masarat() -> [(HKEY, &'static str, REG_SAM_FLAGS); 5] {
        [
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Rockstar Games", KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Rockstar Games", KEY_WOW64_32KEY),
            (HKEY_LOCAL_MACHINE, r"SOFTWARE\Rockstar Games", KEY_WOW64_64KEY),
            (HKEY_CURRENT_USER, r"Software\WOW6432Node\Rockstar Games", KEY_WOW64_64KEY),
            (HKEY_CURRENT_USER, r"Software\Rockstar Games", KEY_WOW64_64KEY),
        ]
    }

    /// An open key that closes itself.
    ///
    /// The whole reason this type exists: `RegOpenKeyExW` hands out a kernel
    /// handle, this adapter opens one per title per view per scan, and Taarib
    /// rescans on every filesystem event for as long as the application is
    /// running. A handle dropped on an early return here is a handle leaked for
    /// the lifetime of the process.
    struct Miftah(HKEY);

    impl Drop for Miftah {
        fn drop(&mut self) {
            // SAFETY: the handle came from a successful RegOpenKeyExW, is never
            // copied out of this type, and is closed exactly once, here.
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    /// A null-terminated wide string, alive for as long as the caller holds it.
    fn wide(nass: &str) -> Vec<u16> {
        nass.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Opens a key under a predefined root.
    fn fath(jidhr: HKEY, masar: &str, ruya: REG_SAM_FLAGS) -> Option<Miftah> {
        let masar_w = wide(masar);
        let mut miftah = HKEY::default();
        // SAFETY: `masar_w` is a live, null-terminated wide string for the whole
        // call, and `miftah` is a live, correctly typed out-parameter.
        let natija = unsafe {
            RegOpenKeyExW(
                jidhr,
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
            // not need is passed as None rather than as a dangling pointer.
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

    /// The raw bytes and declared type of one value.
    ///
    /// Both reads are done here — the sizing call and the data call — because
    /// they have to agree about the buffer, and splitting them across functions
    /// is how a reader ends up passing a length that describes a different
    /// allocation.
    fn qeema_khaam(miftah: &Miftah, ism: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
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
        if hajm == 0 || hajm > HADD_QEEMA {
            return None;
        }

        let mut buffer = vec![0u8; usize::try_from(hajm).ok()?];
        let mut hajm_mutah = hajm;
        // SAFETY: `buffer` is exactly `hajm` bytes and `hajm_mutah` tells the
        // call exactly that, so the write is in bounds. Alignment is one byte,
        // which any allocation satisfies.
        let natija = unsafe {
            RegQueryValueExW(
                miftah.0,
                PCWSTR(ism_w.as_ptr()),
                None,
                None,
                Some(buffer.as_mut_ptr()),
                Some(&raw mut hajm_mutah),
            )
        };
        if natija != ERROR_SUCCESS {
            return None;
        }
        let adad = usize::try_from(hajm_mutah).ok()?.min(buffer.len());
        buffer.truncate(adad);
        Some((naw, buffer))
    }

    /// A string value, or `None` when it is absent, too large, or not a string.
    fn qeema_nass(miftah: &Miftah, ism: &str) -> Option<String> {
        let (naw, bayt) = qeema_khaam(miftah, ism)?;
        if naw != REG_SZ && naw != REG_EXPAND_SZ {
            return None;
        }
        let harfiyat: Vec<u16> = bayt
            .chunks_exact(2)
            .filter_map(|juz| <[u8; 2]>::try_from(juz).ok())
            .map(u16::from_le_bytes)
            .collect();
        let tul = harfiyat.iter().position(|harf| *harf == 0).unwrap_or(harfiyat.len());
        let nass = String::from_utf16_lossy(harfiyat.get(..tul)?).trim().to_owned();
        (!nass.is_empty()).then_some(nass)
    }

    /// A numeric value, accepting both the `REG_DWORD` form and a decimal
    /// string — the launcher has written this value both ways.
    fn qeema_raqm(miftah: &Miftah, ism: &str) -> Option<u32> {
        let (naw, bayt) = qeema_khaam(miftah, ism)?;
        if naw == REG_DWORD {
            let arbaa = bayt.get(..4).and_then(|juz| <[u8; 4]>::try_from(juz).ok())?;
            return Some(u32::from_le_bytes(arbaa));
        }
        qeema_nass(miftah, ism)?.trim().parse::<u32>().ok()
    }

    /// The first of several value names that is present, as a string.
    fn awwal_nass(miftah: &Miftah, asmaa: &[&str]) -> Option<String> {
        asmaa.iter().find_map(|ism| qeema_nass(miftah, ism))
    }

    /// The first of several value names that is present, as a number.
    fn awwal_raqm(miftah: &Miftah, asmaa: &[&str]) -> Option<u32> {
        asmaa.iter().find_map(|ism| qeema_raqm(miftah, ism))
    }

    /// Every Rockstar title the registry knows about, across all five views.
    ///
    /// A key name already seen in an earlier view is skipped rather than
    /// merged, because the earlier views are the machine-wide ones and a stale
    /// per-user key pointing at a folder that was moved would otherwise
    /// overwrite the correct path with a dead one.
    pub(super) fn alaab_rockstar() -> Vec<MadkhalSijill> {
        let mut madakhil: Vec<MadkhalSijill> = Vec::new();
        let mut maruf: Vec<String> = Vec::new();

        for (jidhr, masar, ruya) in masarat() {
            let Some(walid) = fath(jidhr, masar, ruya) else {
                continue;
            };
            for ism in mafatih_farya(&walid) {
                let muwahhad = super::wahhid_ism(&ism);
                if maruf.contains(&muwahhad) {
                    continue;
                }
                let Some(miftah) = fath_farii(&walid, &ism, ruya) else {
                    continue;
                };
                let Some(masar_luba) = awwal_nass(&miftah, &QEEM_MASAR) else {
                    // A key with no install folder is a settings key the
                    // launcher keeps beside its titles — graphics preferences,
                    // a language choice. Not a game and not a fault.
                    continue;
                };
                maruf.push(muwahhad);
                madakhil.push(MadkhalSijill {
                    ism,
                    jidhr: PathBuf::from(masar_luba),
                    isdar: awwal_nass(&miftah, &QEEM_ISDAR),
                    muarrif_musajjal: awwal_raqm(&miftah, &QEEM_MUARRIF),
                    mansub: awwal_raqm(&miftah, &QEEM_MANSUB).map(|raqm| raqm != 0),
                });
            }
        }
        madakhil
    }

    /// The launcher's own installation root.
    pub(super) fn jidhr_mushghil() -> Option<PathBuf> {
        for (jidhr, masar, ruya) in masarat() {
            let Some(walid) = fath(jidhr, masar, ruya) else {
                continue;
            };
            let Some(miftah) = fath_farii(&walid, ISM_MUSHGHIL, ruya) else {
                continue;
            };
            if let Some(masar_mushghil) = awwal_nass(&miftah, &QEEM_MASAR) {
                return Some(PathBuf::from(masar_mushghil));
            }
        }
        None
    }

    /// The parent directory of the first installed title, stopping there.
    ///
    /// Deliberately not `alaab_rockstar().first()`: this runs before every scan
    /// and must not read the version, the title id and the installed flag of
    /// every game on the machine to answer "is Rockstar here at all".
    pub(super) fn jidhr_ayy_luba() -> Option<PathBuf> {
        for (jidhr, masar, ruya) in masarat() {
            let Some(walid) = fath(jidhr, masar, ruya) else {
                continue;
            };
            for ism in mafatih_farya(&walid) {
                if super::ASMAA_MUSTATHNAA.contains(&super::wahhid_ism(&ism).as_str()) {
                    continue;
                }
                let Some(miftah) = fath_farii(&walid, &ism, ruya) else {
                    continue;
                };
                let Some(masar_luba) = awwal_nass(&miftah, &QEEM_MASAR) else {
                    continue;
                };
                let masar_luba = PathBuf::from(masar_luba);
                if !masar_luba.is_dir() {
                    continue;
                }
                if let Some(walid_luba) = masar_luba.parent() {
                    return Some(walid_luba.to_path_buf());
                }
            }
        }
        None
    }
}
