//! # كشف تعريب — finding the games
//!
//! Every game installed on the machine, from every source, on every platform,
//! with artwork, kept fresh. This is the first screen the user ever sees
//! populated, so it is also the first place the product either earns trust or
//! loses it: a library that misses half of someone's games is a library they
//! stop opening.
//!
//! Built in **Phase 4**, and it gates every adapter — Phase 5 cannot probe an
//! engine it has not been told about, and an adapter with no dispatch is
//! untestable inside the product.
//!
//! ## Modules
//!
//! | module | what it owns |
//! | --- | --- |
//! | `matajir` | one submodule per source, each implementing the same trait. Stores: `steam`, `epic`, `gog`, `ea`, `ubisoft`, `battlenet`, `xbox`, `itch`, `amazon`, `rockstar`, `riot`. Managers that wrap another store's catalogue: `heroic`, `legendary`, `lutris`, `bottles`, `playnite`. Path-based: `yadawi` for manual addition and `mahmul` for nominated folders |
//! | `ayquna` | icons out of a game's own executable, in each platform's native resource format: the PE resource tree and its `RT_GROUP_ICON`/`RT_ICON` pair, the ICO and DIB containers decoded by hand, macOS ICNS, and the freedesktop `.desktop` plus icon-theme lookup that stands in for the icon an ELF does not carry |
//! | `fann_luba` | artwork every engine ships *inside* the install — Unity splashes, Unreal `Resources/*.ico`, Godot's project icon, RPG Maker title screens — measured from file headers rather than decoded |
//! | `silsila` | the artwork cascade: five rungs in a fixed order, first success wins, and the upscale ceiling that stops a 32-pixel icon becoming a 240-pixel smear |
//! | `wujud` | the existence gate: whether a game a launcher lists is actually installed and actually launchable right now, and — when it is not — exactly why, in words a user can act on |
//! | `tawheed` | deciding when two catalogue entries are one game, by a fingerprint over the install's shape rather than by its title |
//! | `lugha_rasmiya` | whether a game's publisher already ships Arabic — the launcher's declared languages, the engine's own compiled localization resources, and locale-named paths on disk — with the evidence that produced the verdict and a cache that a store update invalidates |
//! | `beea` | the Proton and Wine environment: prefix location, `user.reg` and `system.reg`, the `dosdevices` drive map, and bidirectional translation between the paths the game sees and the paths the host has |
//! | `suwar` | artwork — local launcher caches first, the launcher's own public endpoints for what is missing, content-addressed storage, and the dominant colour extracted at import so the interface has it before the image decodes |
//! | `tahdith` | incremental refresh: a scan diffed against the store, debounced filesystem watches on library roots, and build-id changes flagged for Phase 15's update-survival path |
//!
//! ## Steam is the deep one
//!
//! `matajir::steam` does far more than list `appmanifest_*.acf`. It resolves
//! Steam from the registry on Windows, from `~/.steam/steam` and the Flatpak
//! path on Linux, and from Application Support on macOS; walks
//! `libraryfolders.vdf` in both its legacy and current shapes to reach every
//! library on every drive; reads `StateFlags` so a partially downloaded game is
//! not offered as patchable; parses the binary `appinfo.vdf` for launch
//! options, categories — which is where multiplayer and anti-cheat association
//! surface for Phase 16 — and depot identity; reads `localconfig.vdf` so Phase
//! 15 can *extend* the user's own launch options rather than overwrite them;
//! and reads `shortcuts.vdf`, because on a Steam Deck that is how nearly
//! everything non-Steam ends up in the library.
//!
//! VDF is read by a reader written inside this crate, both the text and the
//! binary dialect. That is a deliberate dependency refusal: binary VDF is
//! undocumented, versioned, and has changed shape more than once, and there is
//! no maintained crate that reads the current `appinfo.vdf`. Depending on one
//! that half-works would mean the primary discovery path on the primary
//! platform breaks on a Steam client update, which is exactly the failure mode
//! this product is built to avoid.
//!
//! ## Prefixes are a first-class environment, not a special case
//!
//! On Linux most Windows games run under Proton, and every path the game knows
//! is a fiction maintained by the prefix. `beea` resolves
//! `steamapps/compatdata/<appid>/pfx`, builds the drive-letter map, and
//! translates in both directions, so Phase 15 can install a Windows-side
//! BepInEx into the Windows-side game directory and write
//! `WINEDLLOVERRIDES=winhttp=n,b` into the launch options without any component
//! upstream needing to know that a prefix was involved. Heroic, Lutris, Bottles
//! and a bare `WINEPREFIX` are all recognised, and the Proton build in use is
//! identified because the override syntax and the correct injection method
//! depend on it.
//!
//! ## Hard constraints
//!
//! - Discovery is read-only. Nothing here writes into a game directory or a
//!   launcher's configuration; Phase 15 owns every write.
//! - No launcher is required to be running, and Taarib never starts one in
//!   order to discover anything.
//! - A malformed or unexpected catalogue file degrades that one entry and is
//!   reported with the file and the reason. It never fails the scan.
//! - Every discovered game gets a `LubaId` that is stable across rescans,
//!   reinstalls, and moves between drives — the same game found through two
//!   launchers resolves to one identity, with both source identifiers kept.
//! - A manually added game produces a `Luba` identical in every respect to a
//!   discovered one, so nothing downstream treats it as second class.
//! - The library screen never waits on this crate's network work. Artwork
//!   arrives after the grid, never before it.

//! ## What this crate does not do
//!
//! It does not persist anything. A scan returns values; storing them is
//! `taarib-makhzan`'s job and deciding to store them is Studio's, and this crate
//! deliberately does not depend on either. That boundary is worth stating
//! because the obvious design is the other one — a scanner that writes its own
//! results — and it is worse in two ways. A scanner that owns a database
//! connection cannot be run against a copy of somebody's machine to debug a
//! discovery bug, and a scanner that decides what to keep has quietly taken over
//! the question of what a library *is*, which belongs one layer up where the
//! user's own choices (hidden games, manual additions, merged entries) live.
//!
//! It also does not identify engines, and never opens a game's files to guess.
//! Discovery answers "what is installed and where"; Phase 5 answers "what is it
//! built with".
//!
//! `rusqlite` appears in this crate's dependencies for one reason that is not
//! persistence: two launchers — GOG's Galaxy and itch's butler — keep their own
//! catalogues in `SQLite` databases, and reading them read-only is how those
//! adapters work.

// The environment ban in `clippy.toml` exists so that no crate resolves
// *configuration* by reading the environment behind `taarib_usus::idadat`'s
// back. This crate is the other thing: its whole job is reading the machine,
// and $WINEPREFIX, $XDG_DATA_HOME, $STEAM_COMPAT_DATA_PATH and their kin are
// where launchers actually record where they put things. Stated once, here,
// rather than at twenty-three call sites that would each say the same.
#![expect(
    clippy::disallowed_methods,
    reason = "discovery reads the machine's environment; the ban is about \
              configuration resolved outside taarib_usus::idadat"
)]

pub mod ayquna;
pub mod beea;
pub mod fahs;
pub mod fann_luba;
pub mod khata;
pub mod lugha_rasmiya;
pub mod matajir;
pub mod silsila;
pub mod suwar;
pub mod tahdith;
pub mod tawheed;
pub mod wujud;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use taarib_mustalahat::luba::{Luba, LubaId, SuwarLuba};
use taarib_usus::idadat::IdadatManassat;
use taarib_usus::khata::Natija;
use taarib_usus::manassa::NizamTashghil;

pub use crate::fahs::{
    LubaMuktashafa, MasadirSuwar, MasdarSura, Matjar, NatijatFahs, NatijatMatjar, SimatLuba,
    SiyaqFahs, TanbihFahs,
};
pub use crate::khata::KhataKashf;
pub use crate::lugha_rasmiya::{
    Fahis, FahisMawarid, KhazinaDhakira, KhazinatLugha, LughaMuallana, LughatMuallana, MasdarLughat,
    MawridLugha, MiftahLugha, SijillLughaRasmiya, TalabLugha,
};
pub use crate::tahdith::{FarqFahs, Muraqib, TaghyeerLuba};

/// The scanner.
///
/// Owns the seventeen adapters and runs them. Deliberately not a long-lived
/// service:
/// it holds no connection, no cache and no thread, so a caller builds one,
/// scans, and drops it. The state that matters — the library — lives in the
/// store, and the state that is expensive — artwork — lives in
/// [`suwar::KhaziantSuwar`], which outlives any single scan.
pub struct Kashif {
    matajir: Vec<Box<dyn Matjar>>,
}

impl std::fmt::Debug for Kashif {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The identifiers, not the adapters. `Matjar` deliberately does not
        // require `Debug` — an adapter is behaviour, and forcing every one of
        // them to derive a trait only this line wants would be the tail wagging
        // the dog.
        let asmaa: Vec<&'static str> = self.matajir.iter().map(|matjar| matjar.muarrif()).collect();
        f.debug_struct("Kashif").field("matajir", &asmaa).finish()
    }
}

impl Default for Kashif {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl Kashif {
    /// A scanner over every adapter.
    #[must_use]
    pub fn jadeed() -> Self {
        Self { matajir: matajir::kul() }
    }

    /// A scanner over a chosen subset, for a caller refreshing one launcher
    /// after its watcher fired rather than rescanning the machine.
    #[must_use]
    pub fn min_matajir(matajir: Vec<Box<dyn Matjar>>) -> Self {
        Self { matajir }
    }

    /// The adapters this scanner will run.
    #[must_use]
    pub fn matajir(&self) -> &[Box<dyn Matjar>] {
        &self.matajir
    }

    /// Runs every adapter that applies to this platform.
    ///
    /// One adapter failing does not stop the others: its failure becomes a
    /// warning on its own [`NatijatMatjar`] and the scan continues. A user with
    /// a broken GOG installation still gets their Steam library, which is the
    /// difference between a product that works on a real machine and one that
    /// works on a clean one.
    ///
    /// # Errors
    ///
    /// Never, currently. The signature is `Natija` because a caller should not
    /// have to change when a future adapter needs to report a failure that
    /// genuinely stops a scan — and because every other entry point in the
    /// product returns one.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the paragraph above is the decision: the result type is part of the \
                  contract, not a consequence of today's implementation"
    )]
    pub fn ifhas(&self, siyaq: &SiyaqFahs) -> Natija<NatijatFahs> {
        let bidaya = Instant::now();
        let mut natija = NatijatFahs::default();

        for matjar in &self.matajir {
            if !matjar.manassat_maduma().contains(&siyaq.nizam) {
                continue;
            }
            match matjar.ifhas(siyaq) {
                Ok(wahid) => natija.matajir.push(wahid),
                Err(khata) => {
                    // The launcher is installed and unreadable. That is worth
                    // seeing in Diagnostics, and worth nothing at all to the
                    // nine other launchers whose games are still coming.
                    tracing::warn!(
                        matjar = matjar.muarrif(),
                        ramz = %khata.ramz,
                        "a launcher could not be scanned"
                    );
                    let mut fashil = NatijatMatjar::ghayr_mutah(matjar.muarrif());
                    fashil.tanbihat.push(TanbihFahs::jadeed(
                        matjar.muarrif(),
                        matjar.ism_injilizi(),
                        khata.injilizi.clone(),
                    ));
                    natija.matajir.push(fashil);
                }
            }
        }

        natija.muddat = bidaya.elapsed();
        Ok(natija)
    }

    /// Every directory worth watching, across every applicable adapter.
    #[must_use]
    pub fn judhur_muraqaba(&self, siyaq: &SiyaqFahs) -> Vec<PathBuf> {
        let mut judhur: Vec<PathBuf> = self
            .matajir
            .iter()
            .filter(|matjar| matjar.manassat_maduma().contains(&siyaq.nizam))
            .flat_map(|matjar| matjar.judhur_muraqaba(siyaq))
            .collect();
        judhur.sort();
        judhur.dedup();
        judhur
    }
}

/// Builds the context every adapter is handed.
///
/// Resolves the home directory once, here, so that seventeen adapters cannot
/// arrive at seventeen different answers about where the user's files are, and
/// for the same reason every other ambient directory a launcher hides its
/// catalogue in: Windows' program folders and its three data folders, and the
/// XDG base directories. This function is the only place in the crate that asks
/// the environment where those are.
///
/// # Errors
///
/// Never, currently — everything resolved here has a defined answer on every
/// platform. The result type is kept for the same reason [`Kashif::ifhas`]
/// keeps one: it is the contract every entry point in the product presents,
/// and a scan context that one day has to resolve something fallible should not
/// force every caller to change.
#[expect(
    clippy::unnecessary_wraps,
    reason = "see the Errors section: the result type is the contract, not the implementation"
)]
pub fn siyaq_fahs(manassat: IdadatManassat, manzil: PathBuf) -> Natija<SiyaqFahs> {
    let nizam = NizamTashghil::hali();
    let khazina_bayanat = khazina_xdg("XDG_DATA_HOME")
        .unwrap_or_else(|| manzil.join(".local").join("share"));
    Ok(SiyaqFahs {
        mujalladat_baramij: mujalladat_baramij(nizam),
        bayanat_barnamij: mujallad_windows(nizam, "PROGRAMDATA", || {
            PathBuf::from(r"C:\ProgramData")
        }),
        bayanat_mutajawwila: mujallad_windows(nizam, "APPDATA", || {
            manzil.join("AppData").join("Roaming")
        }),
        bayanat_mahalliya: mujallad_windows(nizam, "LOCALAPPDATA", || {
            manzil.join("AppData").join("Local")
        }),
        judhur_bayanat: judhur_bayanat(nizam, &khazina_bayanat, std::env::var_os("XDG_DATA_DIRS")),
        khazina_bayanat,
        khazina_idadat: khazina_xdg("XDG_CONFIG_HOME").unwrap_or_else(|| manzil.join(".config")),
        khazina_makhbaa: khazina_xdg("XDG_CACHE_HOME").unwrap_or_else(|| manzil.join(".cache")),
        nizam,
        manassat,
        manzil,
        yashmal_hawiyat: cfg!(target_os = "linux"),
    })
}

/// One of Windows' named directories, or [`None`] where the platform has none.
///
/// The fallback is a value the caller supplies rather than a literal here,
/// because the documented default differs in kind: `%PROGRAMDATA%` falls back to
/// an absolute machine path, and the two per-user folders fall back to a layout
/// under the home directory this scan already resolved. It is consulted only on
/// a Windows machine whose variable is unset — a stripped service environment,
/// not an ordinary desktop — and never off Windows, where returning
/// `C:\ProgramData` would put a path that cannot exist into a context whose
/// whole job is describing the machine it is running on.
fn mujallad_windows(
    nizam: NizamTashghil,
    mutaghayyir: &str,
    ihtiyati: impl FnOnce() -> PathBuf,
) -> Option<PathBuf> {
    if !matches!(nizam, NizamTashghil::Windows) {
        return None;
    }
    Some(mujallad_aw_ihtiyati(std::env::var_os(mutaghayyir), ihtiyati))
}

/// The rule the environment feeds: a value that is set and not empty wins, and
/// anything else falls back.
///
/// Separated from the read because the read is the untestable half — since
/// edition 2024 `std::env::set_var` is `unsafe` and racy, so a test cannot point
/// this at a value without mutating the process every other test shares. The
/// rule, given the value, is an ordinary function with an ordinary test.
fn mujallad_aw_ihtiyati(
    qeema: Option<std::ffi::OsString>,
    ihtiyati: impl FnOnce() -> PathBuf,
) -> PathBuf {
    qeema.filter(|qeema| !qeema.is_empty()).map_or_else(ihtiyati, PathBuf::from)
}

/// One XDG base directory, honoured only when it is absolute.
fn khazina_xdg(mutaghayyir: &str) -> Option<PathBuf> {
    khazina_mutlaqa(std::env::var_os(mutaghayyir))
}

/// How many entries of `$XDG_DATA_DIRS` are honoured.
///
/// The specification puts no limit on the list, and every entry costs a `stat`
/// per icon size per extension in the theme walk. Eight is more than any real
/// desktop sets — a Nix profile with Flatpak's two exports is five — and the
/// cap is what stops a hostile or broken value turning icon extraction into a
/// filesystem sweep.
const HADD_JUDHUR_BAYANAT: usize = 8;

/// The XDG data hierarchy to search, in the order the specification gives it.
///
/// Split from the read for the reason [`mujallad_aw_ihtiyati`] is: the read is
/// the untestable half, and the rule — home directory first, then the search
/// list, absolute entries only, deduplicated, capped — is an ordinary function
/// with an ordinary test.
///
/// Empty off Linux. The specification is a freedesktop one; a Windows machine
/// with `XDG_DATA_DIRS` set has a shell that has never heard of it, and letting
/// a value leak into the context there would put paths that cannot exist in
/// front of the adapters.
fn judhur_bayanat(
    nizam: NizamTashghil,
    khazina_bayanat: &Path,
    qeema: Option<std::ffi::OsString>,
) -> Vec<PathBuf> {
    if !matches!(nizam, NizamTashghil::Linux) {
        return Vec::new();
    }

    let mut judhur: Vec<PathBuf> = vec![khazina_bayanat.to_path_buf()];
    let khaam = qeema
        .filter(|qeema| !qeema.is_empty())
        .map_or_else(|| DHUR_BAYANAT_IFTIRADI.to_owned(), |qeema| {
            qeema.to_string_lossy().into_owned()
        });
    for juz in khaam.split(':') {
        if judhur.len() >= HADD_JUDHUR_BAYANAT {
            break;
        }
        let masar = PathBuf::from(juz);
        // The same absolute-only rule the single base directories are read
        // under, and for the same reason: a relative entry would resolve
        // against whatever directory this process happened to start in.
        if !juz.is_empty() && masar.is_absolute() && !judhur.contains(&masar) {
            judhur.push(masar);
        }
    }
    judhur
}

/// What `$XDG_DATA_DIRS` means when it is unset, quoted from the specification.
const DHUR_BAYANAT_IFTIRADI: &str = "/usr/local/share:/usr/share";

/// The absolute-only rule the XDG base directories are read under.
///
/// The specification is explicit that a relative value is invalid and must be
/// ignored, and the reason is worth keeping: a relative `XDG_DATA_HOME` would
/// resolve against whatever directory this process happens to have been started
/// in, which on a desktop launch is the user's home and in a terminal is
/// wherever they were standing. Silently accepting it would make discovery
/// depend on the working directory. Split from the read for the reason
/// [`mujallad_aw_ihtiyati`] is.
fn khazina_mutlaqa(qeema: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let masar = PathBuf::from(qeema?);
    masar.is_absolute().then_some(masar)
}

/// Windows' machine-wide program directories, most specific first.
///
/// `%ProgramFiles(x86)%` leads because the launchers that still ship a 32-bit
/// installer — Steam among them — land in it on every 64-bit Windows. The
/// documented `C:\` defaults are a last resort rather than the answer: Windows
/// is installed on a drive of the user's choosing, and a literal `C:` sends an
/// adapter to a folder that is not there on the machines where this list is
/// consulted at all. A 32-bit build sees the same directory in both variables,
/// which is why the list is deduplicated rather than trusted to differ.
fn mujalladat_baramij(nizam: NizamTashghil) -> Vec<PathBuf> {
    if !matches!(nizam, NizamTashghil::Windows) {
        return Vec::new();
    }
    let mut mujalladat: Vec<PathBuf> = Vec::new();
    for (mutaghayyir, ihtiyati) in
        [("ProgramFiles(x86)", r"C:\Program Files (x86)"), ("ProgramFiles", r"C:\Program Files")]
    {
        let mujallad =
            mujallad_aw_ihtiyati(std::env::var_os(mutaghayyir), || PathBuf::from(ihtiyati));
        if !mujalladat.contains(&mujallad) {
            mujalladat.push(mujallad);
        }
    }
    mujalladat
}

/// Folds a scan into library entries.
///
/// Two things happen here and nowhere else, and both are identity decisions that
/// no adapter is allowed to make for itself.
///
/// **Identity.** A game's `LubaId` is derived from its launcher identity and its
/// normalized name. Heroic unwraps first — `MasdarLuba::asl()` — so a game
/// installed through Heroic on Linux derives the *same* identity as the same
/// game installed through Epic on Windows, and a patch published against one is
/// offered to the other. That is not a convenience; it is most of what makes the
/// registry usable for Linux players.
///
/// **Merging.** Two entries merge into one library row when they resolve to the
/// same identity, or when they occupy the same canonical install root — the
/// second rule being how the Steam entry and the Heroic entry for one set of
/// files become one card rather than two.
///
/// Merging on *name alone* is deliberately not done. `Doom` is at least three
/// different games, `Prey` is two, and a library that silently fused them would
/// offer a patch for one as though it were for the other. Where two launchers
/// genuinely hold the same title in different directories, the user sees two
/// cards, which is honest, rather than one wrong one.
#[must_use]
pub fn ijma(natija: &NatijatFahs) -> Vec<Luba> {
    let mut bil_huwiya: BTreeMap<LubaId, Luba> = BTreeMap::new();
    let mut bil_masar: BTreeMap<String, LubaId> = BTreeMap::new();

    for muktashafa in natija.alaab() {
        let asl = muktashafa.masdar.asl().clone();
        let huwiya = LubaId::min_masdar(&asl, &muktashafa.ism);
        let miftah_masar = miftah_jidhr(&muktashafa.jidhr);

        // A root already claimed by another launcher is the same installation,
        // whatever the two catalogues call it.
        let hadaf = bil_masar.get(&miftah_masar).copied().unwrap_or(huwiya);

        match bil_huwiya.get_mut(&hadaf) {
            Some(mawjuda) => damm(mawjuda, muktashafa),
            None => {
                let _ = bil_huwiya.insert(hadaf, min_muktashafa(hadaf, muktashafa));
            }
        }
        let _ = bil_masar.insert(miftah_masar, hadaf);
    }

    bil_huwiya.into_values().collect()
}

/// Turns one discovered entry into a library row.
fn min_muktashafa(huwiya: LubaId, muktashafa: &LubaMuktashafa) -> Luba {
    Luba {
        id: huwiya,
        masadir: vec![muktashafa.masdar.clone()],
        ism: muktashafa.ism.clone(),
        jidhr: muktashafa.jidhr.clone(),
        tanfidhi: muktashafa.tanfidhi.clone(),
        hajm: muktashafa.hajm,
        akhir_laab: muktashafa.akhir_laab.clone(),
        akhir_tahdith: muktashafa.akhir_tahdith.clone(),
        // The build is not known until Phase 5 fingerprints the installation;
        // the launcher's own build id travels separately, on the discovered
        // entry, because it is evidence rather than identity.
        bina: None,
        suwar: SuwarLuba::default(),
        beea: muktashafa.beea.clone(),
        mawjuda: true,
        mukhfiya: !muktashafa.hiya_luba(),
    }
}

/// Folds a second sighting of the same installation into the row that exists.
///
/// The first launcher to report a field keeps it. That sounds arbitrary and is
/// not: adapters run in a fixed order with the most authoritative source first,
/// so "first wins" means "Steam's name for a Steam game", and a later launcher
/// only fills in what the earlier one did not know.
fn damm(mawjuda: &mut Luba, muktashafa: &LubaMuktashafa) {
    if !mawjuda.masadir.iter().any(|mawjud| mawjud.asl() == muktashafa.masdar.asl()) {
        mawjuda.masadir.push(muktashafa.masdar.clone());
    }
    if mawjuda.tanfidhi.is_none() {
        mawjuda.tanfidhi.clone_from(&muktashafa.tanfidhi);
    }
    if mawjuda.hajm == 0 {
        mawjuda.hajm = muktashafa.hajm;
    }
    if mawjuda.akhir_laab.is_none() {
        mawjuda.akhir_laab.clone_from(&muktashafa.akhir_laab);
    }
    if mawjuda.akhir_tahdith.is_none() {
        mawjuda.akhir_tahdith.clone_from(&muktashafa.akhir_tahdith);
    }
    // A compatibility layer reported by any launcher wins over none: a game seen
    // as native by one catalogue and as Proton by another is a Proton game, and
    // treating it as native would send the installer to the wrong filesystem.
    if matches!(mawjuda.beea, taarib_usus::manassa::BeeatTawafuq::Asli)
        && !matches!(muktashafa.beea, taarib_usus::manassa::BeeatTawafuq::Asli)
    {
        mawjuda.beea = muktashafa.beea.clone();
    }
}

/// The comparison key for an install root.
///
/// Case-folded on the platforms whose filesystems are, and with a trailing
/// separator removed, so that `C:\Games\X` and `c:\games\x\` are recognised as
/// one directory. Not canonicalized: a root on a drive that is currently
/// unplugged still has to compare equal to itself, and `canonicalize` on a
/// missing path fails.
fn miftah_jidhr(jidhr: &Path) -> String {
    let nass = jidhr.to_string_lossy();
    let maqsus = nass.trim_end_matches(['/', '\\']);
    if NizamTashghil::hali().hassas_lil_ahruf() {
        maqsus.to_owned()
    } else {
        maqsus.to_lowercase()
    }
}

#[cfg(test)]
mod ikhtibarat {
    use super::*;

    #[test]
    fn mujalladat_al_baramij_farigha_kharij_windows() {
        for nizam in [NizamTashghil::Linux, NizamTashghil::Mac] {
            assert!(
                mujalladat_baramij(nizam).is_empty(),
                "neither system has a machine-wide program folder to name"
            );
        }
    }

    #[test]
    fn mujalladat_al_baramij_ala_windows_maqsura_wa_bila_takrar() {
        let mujalladat = mujalladat_baramij(NizamTashghil::Windows);

        // One on a 32-bit build, where both variables name the same directory.
        assert!(
            (1..=2).contains(&mujalladat.len()),
            "at most one entry per variable: {mujalladat:?}"
        );
        let mut fareeda = mujalladat.clone();
        fareeda.dedup();
        assert_eq!(fareeda, mujalladat, "a repeated directory would be scanned twice");
        assert!(
            mujalladat.iter().all(|mujallad| !mujallad.as_os_str().is_empty()),
            "an empty variable must fall back, never become an empty path"
        );
    }

    #[test]
    fn siyaq_al_fahs_yahmil_mujalladat_al_baramij() -> Result<(), Box<dyn std::error::Error>> {
        let siyaq = siyaq_fahs(IdadatManassat::default(), PathBuf::from("/manzil"))?;

        // The point of the field: the adapters are handed the answer rather
        // than each reaching for the environment and disagreeing.
        assert_eq!(siyaq.mujalladat_baramij, mujalladat_baramij(NizamTashghil::hali()));
        Ok(())
    }

    #[test]
    fn mujalladat_windows_ghaiba_kharij_windows() {
        for nizam in [NizamTashghil::Linux, NizamTashghil::Mac] {
            assert_eq!(
                mujallad_windows(nizam, "PROGRAMDATA", || PathBuf::from("/la-yuqra")),
                None,
                "neither system has a %PROGRAMDATA%, and a context that claimed one \
                 would be describing a machine that is not there"
            );
        }
    }

    #[test]
    fn qeemat_al_beea_tafuz_wa_al_farigha_tasqut() {
        let ihtiyati = || PathBuf::from(r"C:\ProgramData");

        // Set and not empty: the machine's own answer, whatever it is.
        assert_eq!(
            mujallad_aw_ihtiyati(Some(std::ffi::OsString::from(r"D:\Bayanat")), ihtiyati),
            PathBuf::from(r"D:\Bayanat")
        );
        // Unset and empty are the same case: a variable a stripped service
        // environment cleared is not a directory named the empty string.
        assert_eq!(mujallad_aw_ihtiyati(None, ihtiyati), ihtiyati());
        assert_eq!(mujallad_aw_ihtiyati(Some(std::ffi::OsString::new()), ihtiyati), ihtiyati());
    }

    #[test]
    fn khazina_xdg_tarfud_al_masar_al_nisbi() {
        assert_eq!(
            khazina_mutlaqa(Some(std::ffi::OsString::from("bayanat"))),
            None,
            "a relative XDG value is invalid and is ignored, never resolved"
        );
        assert_eq!(khazina_mutlaqa(None), None);

        let mutlaq = if cfg!(windows) { r"D:\bayanat" } else { "/bayanat" };
        assert_eq!(
            khazina_mutlaqa(Some(std::ffi::OsString::from(mutlaq))),
            Some(PathBuf::from(mutlaq))
        );
    }

    #[test]
    fn judhur_al_bayanat_tatbaa_al_muwasafa() {
        let manzil = PathBuf::from("/manzil/.local/share");
        let bi = |qeema: Option<&str>| {
            judhur_bayanat(
                NizamTashghil::Linux,
                &manzil,
                qeema.map(std::ffi::OsString::from),
            )
        };

        // Unset and empty both mean the specification's own default, and the
        // user's own directory always leads.
        let iftiradi =
            vec![manzil.clone(), PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")];
        assert_eq!(bi(None), iftiradi);
        assert_eq!(bi(Some("")), iftiradi);

        // Relative entries are invalid per the specification and are dropped
        // rather than resolved; a repeat of the leading directory is dropped
        // too, because scanning it twice is two answers to one question.
        assert_eq!(
            bi(Some("share:/opt/x::/manzil/.local/share:/usr/share")),
            vec![manzil.clone(), PathBuf::from("/opt/x"), PathBuf::from("/usr/share")]
        );

        // The cap holds however long the value is.
        let tawil = (0..40).map(|raqm| format!("/d{raqm}")).collect::<Vec<_>>().join(":");
        assert_eq!(bi(Some(&tawil)).len(), HADD_JUDHUR_BAYANAT);

        // Off Linux the freedesktop hierarchy does not exist, and a value that
        // leaked in from a shell would be paths that cannot be there.
        for nizam in [NizamTashghil::Windows, NizamTashghil::Mac] {
            assert!(judhur_bayanat(nizam, &manzil, Some("/usr/share".into())).is_empty());
        }
    }

    #[test]
    fn siyaq_al_fahs_yahmil_khazain_xdg() -> Result<(), Box<dyn std::error::Error>> {
        // A drive letter on Windows, where a path with a root and no prefix —
        // `\manzil` — is *not* absolute, so a `/manzil` fixture would fail the
        // assertion below over the fixture rather than over the code.
        let manzil = PathBuf::from(if cfg!(windows) { r"D:\manzil" } else { "/manzil" });
        let siyaq = siyaq_fahs(IdadatManassat::default(), manzil.clone())?;

        // Absolute in every case: either the variable was absolute, or the
        // home-relative default the specification names took over. An adapter
        // that joined onto a relative value would be scanning whatever
        // directory the process was started in.
        for khazina in [&siyaq.khazina_bayanat, &siyaq.khazina_idadat, &siyaq.khazina_makhbaa] {
            assert!(khazina.is_absolute(), "{}", khazina.display());
        }
        if khazina_xdg("XDG_DATA_HOME").is_none() {
            assert_eq!(siyaq.khazina_bayanat, manzil.join(".local").join("share"));
        }
        Ok(())
    }
}
