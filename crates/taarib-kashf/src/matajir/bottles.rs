//! بوتلز — Bottles, which manages prefixes rather than games, and why that
//! changes what an identity has to be.
//!
//! Every other adapter in this crate reads a *store's* catalogue. Bottles has
//! none. It is a Wine prefix manager: the user creates a bottle, which is a
//! prefix with a name and a runner, and then registers programs inside it. There
//! is no purchase, no account, no product identifier, and nothing anywhere that
//! says which game a given `game.exe` is. What Bottles knows is that a bottle
//! exists, that a program inside it is called something, and where inside the
//! prefix's own `C:` drive that program lives.
//!
//! ## What is read
//!
//! | path | what it carries |
//! | --- | --- |
//! | `$XDG_DATA_HOME/bottles/bottles/` | one directory per bottle, each of which **is** a Wine prefix |
//! | `<bottle>/bottle.yml` | `Name`, `Path`, `Runner`, `Arch`, `Environment`, `Update_Date`, and the `External_Programs` map |
//! | `<bottle>/drive_c`, `<bottle>/user.reg` | the proof that the bottle was actually built, rather than only configured |
//!
//! The Flatpak build keeps the same tree under
//! `~/.var/app/com.usebottles.bottles/data/bottles/bottles/`, and both are
//! scanned rather than only the first one found. Unlike Lutris, where a
//! catalogue and a configuration directory have to be matched to each other, a
//! bottle is entirely self-contained: the prefix, its configuration and its
//! programs are all inside the one directory, so reading a native bottle and a
//! Flatpak bottle in the same scan mixes nothing.
//!
//! ## The identity is `<bottle>/<program>`, and both halves are load-bearing
//!
//! [`MasdarLuba::Bottles`] carries a single string, and this adapter puts a
//! composite in it. That is not a convenience:
//!
//! - **The bottle name alone would collide with itself.** A bottle is a prefix,
//!   and one prefix routinely holds several programs — a game, its launcher, a
//!   mod manager, the redistributables its installer left behind. Every one of
//!   them would derive the same [`LubaId`](taarib_mustalahat::luba::LubaId),
//!   because that identity is a hash of the source identifier and the name; the
//!   library would show one entry where there are four, and a patch installed
//!   for one would be recorded against all of them.
//! - **The program name alone would collide across bottles.** Bottles names a
//!   program after the executable it points at, and `Game.exe` is not a rare
//!   name. Two bottles built for two different games routinely hold programs
//!   with the same name, and merging them would offer one game's patch to the
//!   other.
//!
//! The two together are unique by construction: a bottle name is a directory
//! name, so it cannot contain `/`, so the composite always splits back at its
//! **first** slash into the bottle and everything after it. That also makes
//! `bottles:Elden Ring/Start Game` a string a user can recognise in a bug
//! report, which the alternatives — a hash, a UUID — would not be.
//!
//! The program half is the entry's `name` where it has one rather than its key
//! in the `External_Programs` map. The map key is a value Bottles generates and
//! regenerates: removing a shortcut and adding it back produces a new key for
//! the same executable, and keying on it would make the game a *different* game
//! afterwards, losing its installed patch, its artwork and its history for a
//! change the user experienced as renaming a shortcut.
//!
//! ## Bottles has no store behind it, and this adapter does not invent one
//!
//! [`MasdarLuba::Bottles`] is not a wrapper. Unlike Heroic, Legendary and
//! Lutris, there is nothing to wrap: the user pointed Bottles at an installer
//! they got from somewhere, and nothing on disk records where. So these entries
//! shard under `bottles` and match only patches published against a Bottles
//! identity. Guessing at a store from an executable name would produce an
//! identity that looks right and matches the wrong patches.
//!
//! ## The configuration reader is the Lutris one
//!
//! `bottle.yml` is YAML, this workspace has no YAML crate, and adding one to
//! read six keys would be a poor trade. [`crate::matajir::lutris::hallil_yaml`]
//! is the reader — a small, strict, exactly documented subset — and this module
//! imports it rather than growing a second one. Both launchers are Python
//! programs calling `yaml.dump`, so they emit the same shape; one reader is
//! correct for both, and two would be two things to keep in agreement. Every
//! construct outside the subset is refused by name and by line rather than
//! guessed at, and this module turns a refusal on a key it actually needed into
//! a warning naming the construct.
//!
//! `External_Programs` is the one three-level structure either launcher writes —
//! a mapping of entry keys to mappings of fields — and it is why the reader
//! follows nested block mappings at all rather than stopping at two levels.
//!
//! ## Linux only
//!
//! Bottles is a GTK application distributed as a Flatpak, and running Wine
//! inside a prefix is the entire point of it. There is no Windows build and no
//! macOS build, and claiming either would mean running a scan that can only ever
//! find nothing.

use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::MasdarLuba;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};

use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatMatjar, SimatLuba, SiyaqFahs,
    TanbihFahs,
};
use crate::khata::KhataKashf;
use crate::matajir::lutris::{QeemaYaml, WathiqatYaml, iqra_yaml, waqt_mahalli};
use crate::wujud::hall_masar_windows;

/// The stable identifier, matching the registry's shard family. Nothing unwraps
/// it, because there is nothing behind a bottle to unwrap to.
const MUARRIF: &str = "bottles";

/// Bottles is a Linux application and nothing else.
const MANASSAT: [NizamTashghil; 1] = [NizamTashghil::Linux];

/// The Flatpak application id, which is how most people have Bottles installed.
const HAWIYAT_FLATPAK: &str = "com.usebottles.bottles";

/// Bottles' directory under the XDG data root.
const MUJALLAD_BOTTLES: &str = "bottles";

/// The directory *inside* that one which holds the bottles themselves. The
/// repetition is Bottles' own layout, not a mistake: `bottles/bottles/<name>`.
const MUJALLAD_QANANI: &str = "bottles";

/// One bottle's configuration file, which lives inside the prefix.
const ISM_IDAD: &str = "bottle.yml";

/// The key holding the programs registered inside a bottle.
const MIFTAH_BARAMIJ: &str = "External_Programs";

/// How many directories one bottles root will be walked for.
///
/// Bottles users have a handful of bottles and occasionally a few dozen. Two
/// thousand bounds the cost against a directory somebody else filled.
const HADD_QANANI: usize = 2048;

/// How many programs one bottle will contribute.
///
/// Bottles registers every Start Menu shortcut a Windows installer creates, and
/// a bottle used as a general Windows environment accumulates them. Five hundred
/// is far past any real game bottle.
const HADD_BARAMIJ: usize = 512;

/// Executable stems that are never a game, matched exactly.
///
/// Bottles registers what a Windows installer put in the Start Menu, so a bottle
/// holding one game routinely also holds its uninstaller, whatever
/// redistributable the installer bundled, and the Wine tools Bottles itself adds
/// to the list. Marking rather than dropping is deliberate:
/// [`SimatLuba::LaysatLuba`] exists so the library can hide these by default and
/// still show them to somebody who goes looking, and dropping them would make a
/// user who *wants* to patch a launcher unable to find it at all.
const ASMAA_LAYSAT_ALAAB: [&str; 24] = [
    "unins000",
    "unins001",
    "uninstall",
    "uninstaller",
    "uninst",
    "setup",
    "winecfg",
    "wineboot",
    "winefile",
    "winemine",
    "winhlp32",
    "regedit",
    "explorer",
    "cmd",
    "control",
    "taskmgr",
    "notepad",
    "iexplore",
    "dxdiag",
    "msiexec",
    "rundll32",
    "conhost",
    "crashpad_handler",
    "wordpad",
];

/// Executable stem prefixes that are never a game.
///
/// Separate from the exact list because every one of these ships with a version
/// number welded to its name — `vcredist_x64`, `dotnetfx45`, `UnityCrashHandler64`
/// — and enumerating the versions would be a list that is wrong by next year.
const BIDAYAT_LAYSAT_ALAAB: [&str; 8] = [
    "vcredist",
    "vc_redist",
    "dotnetfx",
    "ndp4",
    "directx",
    "dxsetup",
    "oalinst",
    "unitycrashhandler",
];

/// Bottles.
#[derive(Debug, Clone, Copy, Default)]
pub struct MatjarBottles;

impl MatjarBottles {
    /// Builds the adapter.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self
    }
}

impl Matjar for MatjarBottles {
    fn muarrif(&self) -> &'static str {
        MUARRIF
    }

    fn ism_arabi(&self) -> &'static str {
        "بوتلز"
    }

    fn ism_injilizi(&self) -> &'static str {
        "Bottles"
    }

    fn manassat_maduma(&self) -> &'static [NizamTashghil] {
        &MANASSAT
    }

    fn mawqi(&self, siyaq: &SiyaqFahs) -> Option<PathBuf> {
        judhur_muhtamala(siyaq)
            .into_iter()
            .find(|jidhr| jidhr.join(MUJALLAD_QANANI).is_dir())
    }

    /// # Errors
    ///
    /// Returns [`KhataKashf::TaadhurQiraatFahras`] when a bottles directory
    /// exists and cannot be listed — a permissions problem, or a Flatpak data
    /// directory on a filesystem that has gone away. That makes the whole
    /// launcher unreadable, which is the only thing this trait fails a scan for.
    ///
    /// Everything narrower degrades one bottle or one program: a bottle whose
    /// `bottle.yml` will not parse, a bottle that was configured and never
    /// built, a program whose executable is no longer inside the prefix, a
    /// configuration key written with a YAML construct the reader refuses. Each
    /// is a [`TanbihFahs`] on the result and the rest still arrive.
    ///
    /// There is no user override for Bottles in `IdadatManassat`, so
    /// [`KhataKashf::JidhrMuhaddadMafqud`] cannot arise here; when one is added,
    /// this is where it belongs.
    fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatMatjar> {
        let bidaya = Instant::now();
        if !MANASSAT.contains(&siyaq.nizam) {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        }

        let judhur: Vec<PathBuf> = judhur_muhtamala(siyaq)
            .into_iter()
            .filter(|jidhr| jidhr.join(MUJALLAD_QANANI).is_dir())
            .collect();
        let Some(awwal) = judhur.first().cloned() else {
            return Ok(NatijatMatjar::ghayr_mutah(MUARRIF));
        };

        let mut natija = NatijatMatjar::muthabbat(MUARRIF, Some(awwal));

        for jidhr in &judhur {
            jama_qanani(&jidhr.join(MUJALLAD_QANANI), &mut natija)?;
        }

        natija.alaab.sort_by(|a, b| a.ism.cmp(&b.ism));
        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        // The container of bottles, not each bottle. A bottle directory is a
        // live Wine prefix: every launch rewrites `user.reg`, every shader
        // compile writes into it, and watching one would mean a rescan every few
        // seconds for the whole time a game is running. The container changes
        // only when a bottle is created or deleted, which is exactly when a
        // rescan is warranted.
        judhur_muhtamala(siyaq)
            .into_iter()
            .map(|jidhr| jidhr.join(MUJALLAD_QANANI))
            .filter(|masar| masar.is_dir())
            .collect()
    }
}

/// Every Bottles data root worth looking at, most likely first.
///
/// The native layout comes first only because it is cheaper to check; in
/// practice the Flatpak is the common one, which is why it is checked at all
/// rather than being treated as an exotic case. It is only considered when the
/// scan is allowed to look inside containers.
fn judhur_muhtamala(siyaq: &SiyaqFahs) -> Vec<PathBuf> {
    let mut judhur: Vec<PathBuf> = Vec::new();
    // The user's override first. A Bottles installation somewhere unusual is
    // exactly the case a setting exists for, and one that lost to the
    // conventional path would be a setting that does nothing.
    if let Some(masar) = siyaq.manassat.bottles.as_ref() {
        judhur.push(masar.clone());
    }
    judhur.push(siyaq.khazina_bayanat.join(MUJALLAD_BOTTLES));
    if siyaq.yashmal_hawiyat {
        judhur.push(
            siyaq
                .manzil
                .join(".var")
                .join("app")
                .join(HAWIYAT_FLATPAK)
                .join("data")
                .join(MUJALLAD_BOTTLES),
        );
    }
    judhur
}

// ---------------------------------------------------------------------------
// one container of bottles
// ---------------------------------------------------------------------------

/// Reads every bottle under one `bottles/bottles` directory.
///
/// # Errors
///
/// Returns [`KhataKashf::TaadhurQiraatFahras`] when the directory cannot be
/// listed. That is the only failure here that is not about one bottle: if the
/// container cannot be read there is no bottle to degrade, only an empty
/// library with no explanation, which is the outcome this crate exists to avoid.
fn jama_qanani(hawiya: &Path, natija: &mut NatijatMatjar) -> Natija<()> {
    let qaima = std::fs::read_dir(hawiya).map_err(|sabab| KhataKashf::TaadhurQiraatFahras {
        matjar: MUARRIF,
        masar: hawiya.to_path_buf(),
        sabab,
    })?;

    for madkhal in qaima.take(HADD_QANANI).flatten() {
        let jidhr = madkhal.path();
        if !jidhr.is_dir() {
            continue;
        }
        jama_qinnina(&jidhr, natija);
    }
    Ok(())
}

/// Reads one bottle and everything registered inside it.
fn jama_qinnina(jidhr: &Path, natija: &mut NatijatMatjar) {
    let masar_idad = jidhr.join(ISM_IDAD);
    if !masar_idad.is_file() {
        // Bottles keeps working directories beside its bottles — a trash
        // folder, a templates cache — and those are not broken bottles. A
        // directory that *is* a prefix and has no configuration is a different
        // thing: somebody copied a prefix in by hand, and Bottles will not show
        // it either, so saying so is the only way the user learns why.
        if crate::beea::hiya_beea(jidhr) {
            natija.tanbihat.push(TanbihFahs::jadeed(
                MUARRIF,
                jidhr.display().to_string(),
                "this is a Wine prefix inside the Bottles folder with no bottle.yml beside it, so \
                 Bottles does not manage it and Taarib cannot tell which programs in it are \
                 games. Import it from inside Bottles to have it scanned.",
            ));
        }
        return;
    }

    let Some(wathiqa) = iqra_yaml(&masar_idad) else {
        natija.tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            masar_idad.display().to_string(),
            "this bottle's configuration file could not be read, so none of the programs inside \
             it can be listed",
        ));
        return;
    };

    let ism_qinnina = wathiqa
        .nass(&["Name"])
        .or_else(|| {
            jidhr
                .file_name()
                .map(|ism| ism.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "Bottle".to_owned());

    // A bottle's configuration is written when the bottle is created and the
    // prefix is built afterwards, so an interrupted creation leaves exactly this
    // state: a `bottle.yml` describing a prefix that has no `drive_c` and no
    // `user.reg`. Every Windows path inside it would then resolve to nothing,
    // and reporting the programs anyway would fill the library with entries that
    // point at files that were never created.
    if !crate::beea::hiya_beea(jidhr) {
        natija.tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "the bottle {ism_qinnina} is configured and its Wine prefix was never finished \
                 building, so nothing inside it can be located. Open it once in Bottles to \
                 complete it, then rescan."
            ),
        ));
        return;
    }

    tahaqqaq_min_masar(jidhr, &wathiqa, &ism_qinnina, &mut natija.tanbihat);

    if let Some(rafd) = wathiqa.marfud(&[MIFTAH_BARAMIJ]) {
        natija.tanbihat.push(TanbihFahs::fahras(
            MUARRIF,
            masar_idad.display().to_string(),
            format!(
                "the program list of the bottle {ism_qinnina} is written as {} on line {}, which \
                 this reader refuses rather than guessing at, so none of its programs are listed",
                rafd.bina, rafd.satr
            ),
        ));
        return;
    }

    let Some(baramij) = wathiqa.khareeta(&[MIFTAH_BARAMIJ]) else {
        // A bottle with no registered programs is ordinary — it is what a bottle
        // looks like between being created and having something installed into
        // it — and warning about it would put a line in Diagnostics for every
        // bottle a user keeps for `winetricks`.
        //
        // Unless the file used constructs the reader refused, in which case the
        // program list may well be there and unreadable, and the difference
        // between "no programs" and "programs this reader could not see" is
        // exactly what the user needs to be told. The first refusal is enough:
        // one construct outside the subset explains the absence, and printing
        // all of them would be a wall.
        if let Some(rafd) = wathiqa.marfudat().first() {
            natija.tanbihat.push(TanbihFahs::fahras(
                MUARRIF,
                masar_idad.display().to_string(),
                format!(
                    "the bottle {ism_qinnina} lists no programs, and its configuration uses {} on \
                     line {}, which this reader refuses rather than guessing at. If the bottle \
                     does have programs in it, that construct is why they are missing.",
                    rafd.bina, rafd.satr
                ),
            ));
        }
        return;
    };

    let idad = IdadQinnina::iqra(jidhr, &wathiqa, &ism_qinnina);

    for (miftah, barnamaj) in baramij.iter().take(HADD_BARAMIJ) {
        if let Some(luba) = luba_min_barnamaj(&idad, miftah, barnamaj, &mut natija.tanbihat) {
            natija.alaab.push(luba);
        }
    }
}

/// Warns when Bottles' own record of where a bottle lives disagrees with where
/// it was found.
///
/// `Path` is a bare directory name for a bottle Bottles manages itself and an
/// absolute path for one the user placed elsewhere with `Custom_Path`. The
/// directory the file was found in is authoritative either way — it is the
/// prefix that actually exists, and it is the one every Windows path in the file
/// has to be resolved against. The value is still read, because an absolute
/// `Path` pointing somewhere else means the bottle was moved on disk without
/// Bottles being told, and that is the state in which Bottles' own launch
/// buttons stop working while Taarib's still do. The user should know.
fn tahaqqaq_min_masar(
    jidhr: &Path,
    wathiqa: &WathiqatYaml,
    ism_qinnina: &str,
    tanbihat: &mut Vec<TanbihFahs>,
) {
    let Some(musajjal) = wathiqa.nass(&["Path"]) else {
        return;
    };
    let murashah = Path::new(&musajjal);
    if !murashah.is_absolute() {
        return;
    }
    let mutabiq = std::fs::canonicalize(murashah)
        .ok()
        .zip(std::fs::canonicalize(jidhr).ok())
        .is_some_and(|(musajjal, mawjud)| musajjal == mawjud);
    if !mutabiq {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            jidhr.display().to_string(),
            format!(
                "the bottle {ism_qinnina} records its own location as {musajjal}, which is not \
                 where it was found. It has been moved without Bottles being told; Taarib reads \
                 it where it actually is, but Bottles itself will not launch it until the path is \
                 corrected."
            ),
        ));
    }
}

// ---------------------------------------------------------------------------
// one bottle, reduced to what every program inside it inherits
// ---------------------------------------------------------------------------

/// Everything a bottle contributes to each of its programs.
///
/// Resolved once per bottle rather than once per program: opening the prefix
/// costs a `dosdevices` listing and, when the runner is not named, a registry
/// read, and a bottle with forty Start Menu shortcuts in it would otherwise pay
/// for both forty times.
#[derive(Debug, Clone)]
struct IdadQinnina {
    /// The bottle's name, which is the first half of every identity below it.
    ism: String,
    /// The prefix root, which is the bottle's own directory.
    jidhr: PathBuf,
    /// The compatibility environment every program in the bottle runs in.
    beea: BeeatTawafuq,
    /// The trait describing that environment, built once and cloned.
    sima_tawafuq: SimatLuba,
    /// The bottle's last configuration change, RFC 3339, when it records one.
    tahdith: Option<String>,
}

impl IdadQinnina {
    /// Reads a bottle's own settings.
    fn iqra(jidhr: &Path, wathiqa: &WathiqatYaml, ism: &str) -> Self {
        let musajjal = wathiqa.nass(&["Runner"]);

        // `hal_beea` is the one place in this crate that knows how to open a
        // prefix. Bottles names its runner in the configuration, so the detected
        // build is only the fallback — but it is a real fallback: a bottle whose
        // runner was uninstalled keeps the name of a build that is no longer
        // there, and the prefix itself still records what made it.
        let maktashaf = crate::beea::hal_beea(jidhr)
            .ok()
            .and_then(|maalumat| maalumat.isdar_wine);
        let isdar = musajjal.or(maktashaf);

        // Bottles installs Proton builds alongside its Wine ones and runs them
        // as ordinary runners, so the runner *name* is the only evidence of
        // which it is. `naw_beea` would answer Wine here and be right by its own
        // rule — Bottles writes none of Proton's marker files into the prefix —
        // but the distinction matters to Phase 15, which injects differently
        // under the two, so the name is trusted for this one question.
        let proton = isdar
            .as_deref()
            .is_some_and(|ism| ism.to_ascii_lowercase().contains("proton"));

        let beea = if proton {
            BeeatTawafuq::Proton {
                isdar: isdar.clone().unwrap_or_else(|| "Proton".to_owned()),
                beea: jidhr.to_path_buf(),
            }
        } else {
            BeeatTawafuq::Wine {
                isdar: isdar.clone(),
                beea: jidhr.to_path_buf(),
            }
        };

        let wasf = wasf_tawafuq(wathiqa, isdar.as_deref());

        Self {
            ism: ism.to_owned(),
            jidhr: jidhr.to_path_buf(),
            beea,
            sima_tawafuq: SimatLuba::TabaqatTawafuq(wasf),
            // Bottles records no per-program time of any kind. `Update_Date` is
            // the bottle's, and it moves whenever a program inside it is added,
            // removed or reconfigured — so it is a true upper bound on when this
            // entry last changed, which is what it is reported as. It is not,
            // and is not claimed to be, when the game itself was updated.
            tahdith: wathiqa
                .nass(&["Update_Date"])
                .and_then(|nass| waqt_mahalli(&nass)),
        }
    }
}

/// Describes a bottle's compatibility environment in one line.
///
/// Everything in it is a fact Bottles recorded and a fact something downstream
/// needs, which is the test each part had to pass to be here:
///
/// - **the runner** identifies the Wine or Proton build, and the injection
///   method Phase 15 uses differs between them;
/// - **the architecture** is `win32` or `win64`, and it decides which build of
///   an injected framework is the one that will actually load — a 64-bit
///   BepInEx in a 32-bit prefix loads nothing and reports nothing;
/// - **DXVK and VKD3D** replace Wine's Direct3D with Vulkan translations, which
///   changes what an overlay can hook and is the first thing anybody asks about
///   when a Wine rendering problem is reported;
/// - **the synchronisation primitive** — esync or fsync — changes process and
///   thread behaviour enough to matter when a game will not start;
/// - **the bottle's declared environment** is the user's own statement of what
///   the bottle is for, and a bottle marked `Application` is one they created
///   for software rather than games.
///
/// None of it is inferred. A parameter Bottles did not write is simply absent
/// from the line rather than being reported as off, because the reader below
/// does no type inference at all and a missing key and a false one are different
/// facts.
fn wasf_tawafuq(wathiqa: &WathiqatYaml, isdar: Option<&str>) -> String {
    let mut ajza: Vec<String> = Vec::new();
    ajza.push(isdar.unwrap_or("Wine").to_owned());

    if let Some(mimariya) = wathiqa.nass(&["Arch"]) {
        ajza.push(mimariya);
    }
    if let Some(beea_amal) = wathiqa.nass(&["Environment"]) {
        ajza.push(beea_amal);
    }
    if mufaal(wathiqa, "dxvk") {
        ajza.push("DXVK".to_owned());
    }
    if mufaal(wathiqa, "vkd3d") {
        ajza.push("VKD3D".to_owned());
    }
    if mufaal(wathiqa, "fsr") {
        ajza.push("FSR".to_owned());
    }
    if mufaal(wathiqa, "gamemode") {
        ajza.push("GameMode".to_owned());
    }
    if let Some(tazamun) = wathiqa.nass(&["Parameters", "sync"]) {
        ajza.push(tazamun);
    }

    ajza.join(", ")
}

/// Whether one of a bottle's parameters is switched on.
///
/// This is the only place in either of these two adapters where a scalar is
/// interpreted as anything other than text, and it is deliberately here rather
/// than inside the reader. YAML 1.1 treats `no`, `off` and `n` as false, which
/// is the rule that famously turns the country code `NO` into `false`, and a
/// reader that applied it would corrupt every configuration value that happened
/// to spell one of those words. So the reader returns text, and the four
/// spellings Bottles actually writes are recognised here, where being wrong
/// costs a word in a description rather than a path.
fn mufaal(wathiqa: &WathiqatYaml, miftah: &str) -> bool {
    wathiqa.nass(&["Parameters", miftah]).is_some_and(|qeema| {
        matches!(
            qeema.to_ascii_lowercase().as_str(),
            "true" | "yes" | "on" | "1"
        )
    })
}

// ---------------------------------------------------------------------------
// one registered program becomes one game
// ---------------------------------------------------------------------------

/// Turns one `External_Programs` entry into a discovered game.
///
/// Returns nothing, plus a warning, for an entry whose executable is not inside
/// the prefix any more. That happens constantly and for an ordinary reason:
/// Bottles keeps the shortcut when the user deletes the game's folder inside the
/// bottle, so a bottle that has held three games over two years carries three
/// entries and one working executable. Listing the other two would put games in
/// the library that cannot be launched, cannot be probed and cannot be patched.
fn luba_min_barnamaj(
    idad: &IdadQinnina,
    miftah: &str,
    barnamaj: &QeemaYaml,
    tanbihat: &mut Vec<TanbihFahs>,
) -> Option<LubaMuktashafa> {
    // The entry's `name` rather than its key, and the reason is on this module's
    // header: Bottles regenerates the key when a shortcut is removed and added
    // back, and keying the identity on it would turn the same game into a
    // different game for a change the user experienced as a rename.
    let ism_barnamaj = barnamaj
        .nass_haql("name")
        .map(|ism| ism.trim().to_owned())
        .filter(|ism| !ism.is_empty())
        .unwrap_or_else(|| miftah.trim().to_owned());
    if ism_barnamaj.is_empty() {
        return None;
    }

    let masar_windows = barnamaj.nass_haql("path");
    let mujallad_windows = barnamaj.nass_haql("folder");
    let ism_tanfidhi = barnamaj.nass_haql("executable");

    let Some(tanfidhi) = tanfidhi_barnamaj(
        &idad.jidhr,
        masar_windows.as_deref(),
        mujallad_windows.as_deref(),
        ism_tanfidhi.as_deref(),
    ) else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            format!("{}/{ism_barnamaj}", idad.ism),
            format!(
                "Bottles has this program registered at {} inside the bottle {} and there is no \
                 such file in the prefix; it was deleted from inside the bottle, or the shortcut \
                 was made before the program was installed",
                masar_windows
                    .or(mujallad_windows)
                    .unwrap_or_else(|| "an unrecorded path".to_owned()),
                idad.ism
            ),
        ));
        return None;
    };

    let Some(jidhr) = mujallad_barnamaj(&idad.jidhr, mujallad_windows.as_deref(), &tanfidhi) else {
        tanbihat.push(TanbihFahs::jadeed(
            MUARRIF,
            tanfidhi.display().to_string(),
            format!(
                "the program {ism_barnamaj} in the bottle {} resolves to an executable with no \
                 readable folder around it, so there is nothing to probe",
                idad.ism
            ),
        ));
        return None;
    };

    let mut simat = vec![idad.sima_tawafuq.clone()];
    if let Some(sinf) = sinf_ghayr_luba(&tanfidhi, &ism_barnamaj) {
        simat.push(SimatLuba::LaysatLuba(sinf.to_owned()));
    }

    Some(LubaMuktashafa {
        // The composite identity this whole module is built around. The
        // separator is the first `/`; a bottle name is a directory name and
        // therefore cannot contain one, so the bottle half is always recoverable
        // even from a program the user named with a slash in it.
        masdar: MasdarLuba::Bottles(format!("{}/{ism_barnamaj}", idad.ism)),
        hala_matjar: None,
        ism: ism_barnamaj,
        jidhr,
        tanfidhi: Some(tanfidhi),
        // Bottles records no size for anything. Walking the program's folder to
        // compute one would be a directory tree per shortcut on the path the
        // library screen waits for, and would still be the folder's size rather
        // than the game's.
        hajm: 0,
        // There is no build identifier anywhere in Bottles, because there is no
        // store behind it to have issued one. Content fingerprints are the only
        // way to know which build of a game is inside a bottle.
        bina_manassa: None,
        akhir_tahdith: idad.tahdith.clone(),
        // Bottles does not record play times at all.
        akhir_laab: None,
        beea: idad.beea.clone(),
        suwar: suwar_barnamaj(barnamaj),
        khiyarat_tashghil: barnamaj
            .nass_haql("arguments")
            .map(|khiyarat| khiyarat.trim().to_owned())
            .filter(|khiyarat| !khiyarat.is_empty()),
        // Bottles has no download of its own and therefore no partial one: a
        // program is registered after the user installed it, and the executable
        // was proved to exist above.
        muktamila: true,
        simat,
    })
}

/// Resolves a registered program's executable into a real host path.
///
/// Two attempts, and the second is not redundant. `path` is the full Windows
/// path Bottles recorded and is right almost always; `folder` plus `executable`
/// is the same information split in two, and it is what survives when a Bottles
/// version wrote one field and not the other — which has happened across the
/// releases that changed how shortcuts are stored.
///
/// The translation itself is [`crate::wujud::hall_masar_windows`], which is the
/// one place in this crate that maps a prefix's `C:\` fiction onto the
/// filesystem. This module calls it and never writes a second one: a second
/// implementation would be a second set of assumptions about `dosdevices`, and
/// the two would drift.
fn tanfidhi_barnamaj(
    beea: &Path,
    masar_windows: Option<&str>,
    mujallad_windows: Option<&str>,
    ism_tanfidhi: Option<&str>,
) -> Option<PathBuf> {
    if let Some(masar) = masar_windows
        && let Some(mahalli) = hall_masar_windows(beea, masar)
        && mahalli.is_file()
    {
        return Some(mahalli);
    }

    let mujallad = mujallad_windows?.trim_end_matches(['\\', '/']);
    let ism = ism_tanfidhi?.trim();
    if mujallad.is_empty() || ism.is_empty() {
        return None;
    }
    let mahalli = hall_masar_windows(beea, &format!("{mujallad}\\{ism}"))?;
    mahalli.is_file().then_some(mahalli)
}

/// Resolves the folder a registered program lives in.
///
/// Bottles records it, so it is used; the executable's own parent is the
/// fallback for the entries that do not have it. The result has to be a
/// directory that exists, because Phase 5 probes it for an engine and Phase 15
/// writes into it.
fn mujallad_barnamaj(
    beea: &Path,
    mujallad_windows: Option<&str>,
    tanfidhi: &Path,
) -> Option<PathBuf> {
    mujallad_windows
        .and_then(|masar| hall_masar_windows(beea, masar))
        .filter(|masar| masar.is_dir())
        .or_else(|| tanfidhi.parent().map(Path::to_path_buf))
        .filter(|masar| masar.is_dir())
}

/// Whether a registered program is something other than a game, and what.
///
/// Both the executable's stem and the shortcut's display name are examined,
/// because a Windows installer writes the Start Menu entry as
/// `Uninstall <Game>` pointing at `unins000.exe` and either half is enough to
/// recognise it. The answer is a phrase, not a flag, because it goes into
/// [`SimatLuba::LaysatLuba`] and ends up in front of the user as the reason the
/// entry is hidden.
fn sinf_ghayr_luba(tanfidhi: &Path, ism: &str) -> Option<&'static str> {
    let ism_saghir = ism.to_ascii_lowercase();
    if ism_saghir.starts_with("uninstall") || ism_saghir.starts_with("remove ") {
        return Some("an uninstaller rather than a game");
    }

    let jidhr_ism = tanfidhi.file_stem()?.to_string_lossy().to_ascii_lowercase();

    if matches!(
        jidhr_ism.as_str(),
        "unins000" | "unins001" | "uninstall" | "uninstaller" | "uninst"
    ) {
        return Some("an uninstaller rather than a game");
    }
    if jidhr_ism == "setup" {
        return Some("an installer rather than a game");
    }
    if ASMAA_LAYSAT_ALAAB.contains(&jidhr_ism.as_str()) {
        return Some("a Windows or Wine utility rather than a game");
    }
    if BIDAYAT_LAYSAT_ALAAB
        .iter()
        .any(|bidaya| jidhr_ism.starts_with(bidaya))
    {
        return Some("a redistributable runtime installer rather than a game");
    }
    None
}

/// The one image Bottles has for a program.
///
/// Bottles holds no cover art and no banner — it is a prefix manager and has
/// never had a library grid to fill — so [`MasadirSuwar::ghilaf`] and
/// [`MasadirSuwar::batl`] are always empty here and Phase 20B's artwork sources
/// are what will fill them. The `icon` field is either an absolute path to an
/// icon Bottles extracted from the executable or the name of a themed icon from
/// the desktop's own set; only the first is a file this product can read, so the
/// second is discarded rather than passed on as a path that does not exist.
fn suwar_barnamaj(barnamaj: &QeemaYaml) -> MasadirSuwar {
    let shiar = barnamaj.nass_haql("icon").and_then(|ayqona| {
        let masar = PathBuf::from(ayqona.trim());
        (masar.is_absolute() && masar.is_file()).then_some(MasdarSura::Malaf(masar))
    });
    MasadirSuwar {
        ghilaf: None,
        batl: None,
        shiar,
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
    fn al_jidhr_min_khazinat_al_bayanat_fi_al_siyaq() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let khazina = masrah.path().join("khazina");
        fs::create_dir_all(khazina.join(MUJALLAD_BOTTLES).join(MUJALLAD_QANANI))?;

        // `lil_ikhtibar` seeds `khazina_bayanat` with the specification's default
        // *under the fixture's home*, so an adapter that ignored the field would
        // look under `<home>/.local/share` and find nothing here. Nothing sets
        // `$XDG_DATA_HOME`: since edition 2024 that is an `unsafe`, racy mutation
        // of the process every other test shares, which is exactly why the
        // resolver had to move onto the context to become testable at all.
        let mut siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, masrah.path());
        siyaq.khazina_bayanat = khazina.clone();

        assert_eq!(
            MatjarBottles::jadeed().mawqi(&siyaq),
            Some(khazina.join(MUJALLAD_BOTTLES)),
            "the data root must come from the context"
        );
        Ok(())
    }

    #[test]
    fn al_khazina_al_iftiradiya_taht_al_manzil() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let iftiradi = masrah
            .path()
            .join(".local")
            .join("share")
            .join(MUJALLAD_BOTTLES);
        fs::create_dir_all(iftiradi.join(MUJALLAD_QANANI))?;

        // The unset case: the context carries the home-relative default, and the
        // adapter is none the wiser about which of the two it was handed.
        let siyaq = SiyaqFahs::lil_ikhtibar(NizamTashghil::Linux, masrah.path());
        assert_eq!(MatjarBottles::jadeed().mawqi(&siyaq), Some(iftiradi));
        Ok(())
    }
}
