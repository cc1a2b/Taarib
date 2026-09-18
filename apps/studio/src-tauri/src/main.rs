//! # استوديو تعريب — the desktop application
//!
//! This binary owns four things and delegates everything else:
//!
//! 1. **Startup order.** Paths are resolved and created before anything reads
//!    them, settings are loaded before diagnostics start (the log level is a
//!    setting), and diagnostics start before the window opens, so a failure in
//!    the window is already in the log by the time anyone looks for it.
//! 2. **Managed state.** The resolved layout, the live settings store, the
//!    database pool and the log writer guard are handed to Tauri, which keeps
//!    them alive for the process and lends them to commands by type.
//! 3. **The command surface.** Every command is a thin, typed function
//!    returning [`Result<T, Khata>`], so a failure arrives at the interface
//!    already carrying its code, its Arabic sentence, its English sentence and
//!    the one action the user can take next.
//! 4. **The TypeScript boundary.** In a debug build the command signatures and
//!    every type they mention are written to `../src/mustalahat/awamir.ts` by
//!    `tauri-specta`. No request or response shape is ever written twice.
//!
//! The Rust side never renders Arabic and never decides how anything looks. It
//! answers questions with data; the interface decides what to do with it.

// A release build must not open a console window behind the application on
// Windows. Debug builds keep it, because that is where the log goes when the
// file writer itself is what failed.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// `#[tauri::command]` fixes the shape of every command signature in this crate:
// state arrives as a `State<'_, T>` guard by value, and every other argument is
// deserialized out of the IPC payload and therefore owned. Neither can be a
// borrow. All 113 of these sit on a command; none is a function this crate is
// free to write differently.
#![expect(
    clippy::needless_pass_by_value,
    reason = "the command macro owns these signatures — state comes by value and arguments \
              are deserialized owned"
)]
// Two more the macros own rather than this crate. `#[tauri::command]` wraps an
// async command in a body ending in `unreachable!()`, and `Builder::run`
// expands to a `process::exit` after the event loop returns. Both are reported
// against the span of the item they were written onto — the `pub async fn`
// line, the `.run(...)` line — so there is no narrower place to put this than
// the crate. Nothing in this crate writes either of them by hand; the greps
// for `unreachable!` and `process::exit` over `src/` come back empty.
#![expect(
    clippy::unreachable,
    reason = "emitted by `#[tauri::command]`'s async wrapper, attributed to our fn's span"
)]
// `tauri_specta::collect_commands!` coerces every command from a fn item to a
// fn pointer so it can hold them in one list. rustc reports each coercion
// against the command's own definition, so the warning lands on a line that
// contains no cast at all.
#![expect(
    trivial_casts,
    reason = "emitted by `collect_commands!` coercing each command to a fn pointer"
)]
#![expect(
    clippy::exit,
    clippy::disallowed_methods,
    reason = "emitted by `tauri::Builder::run` after the event loop returns; the ban on \
              `process::exit` is about this crate's own code, and no line of it calls one"
)]

// Declared `pub` rather than private because `#[tauri::command]` emits `pub`
// items beside every command it wraps, and a `pub` item inside a private module
// is what `unreachable_pub` refuses.
pub mod jawla_awamir;
pub mod luba_awamir;
pub mod maktaba_awamir;
pub mod mujtama_awamir;
pub mod tathbeet_awamir;
// No commands inside these three, so they stay private.
pub mod aql_awamir;
mod bidaya;
pub mod idadat_awamir;
mod istiada_cli;
mod mukawwinat_tahmil;
pub mod musharaka_awamir;
pub mod suwar_awamir;
pub mod tabaqa_awamir;
pub mod tahdith_awamir;
pub mod taqdeem_awamir;
pub mod tashkhis_awamir;
pub mod tilqai_awamir;
pub mod warsha_awamir;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use taarib_kashf::fahs::{LubaMuktashafa, SimatLuba};
use taarib_kashf::tawheed;
use taarib_kashf::wujud::{self, SijillWujud, TalabWujud};
use taarib_makhzan::sijillat::{HasilatMash, SijillAlaab, SijillMuharrik};
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::ghiyab::{SababGhiyab, ShahidTanfidhi};
use taarib_mustalahat::lawha_badila::{LawhaBadila, SimatLawha};
use taarib_mustalahat::luba::{LawnBariz, Luba, Manassa, MasdarLuba};
use taarib_mustalahat::muharrik::{AilatMuharrik, JahiziyatTashghil, Tabaqa, TaqreerImkaniyat};
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::manassa::{Mimariya, NizamTashghil};
use taarib_usus::masarat::Masarat;
use taarib_usus::{ISDAR, khata_min, sijill};

/// The banner written at the top of every generated binding file, in both
/// languages, so that nobody edits it by hand and then loses the edit on the
/// next debug build.
#[cfg(debug_assertions)]
const TARWEESAT_TAWLEED: &str = concat!(
    "// مولَّد من مفردات رست عبر tauri-specta. أي تعديل يدوي هنا يُفقَد عند أول بناء تصحيحي.\n",
    "// Generated from the Rust vocabulary by tauri-specta. ",
    "Hand edits are lost on the next debug build.\n",
);

/// Where the generated command and type bindings are written.
///
/// Anchored to the crate directory at compile time, not to the process's
/// working directory. A relative path here only resolved when the binary
/// happened to be launched from `src-tauri` — which is what `tauri dev` does,
/// so it worked — and silently wrote nowhere, or refused, from anywhere else.
/// A generator whose output location depends on how it was started is a
/// generator that quietly stops regenerating.
#[cfg(debug_assertions)]
const MASAR_RUBUT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/mustalahat/awamir.ts");

/// The staged artifact tree inside the bundle's resource directory, named by
/// `bundle.resources` in `tauri.conf.json` and written by `taarib-tajmee`.
const MUJALLAD_MAWARID: &str = "mawarid";

/// The title every dialog this file raises carries.
///
/// Both names and no sentence. These dialogs are shown while the settings store
/// may be exactly the thing that failed, so the interface language is not yet a
/// known quantity, and a title bar is not the place to guess at one. The body of
/// every one of them says everything twice for the same reason.
const UNWAN_HIWAR: &str = "تعريب — Taarib";

/// The record a launch that never reached the window leaves behind.
///
/// The rolling log belongs to `sijill::hayyi`, and two of the aborts below
/// happen before it exists. Appended rather than truncated: the fifth identical
/// failure and the first are both worth having, and each is a handful of lines.
const ISM_SIJILL_IQLA: &str = "iqla_fashil.log";

/// The extension the installers register Taarib as the handler for
/// (`bundle.fileAssociations` in `tauri.conf.json`, the NSIS program
/// identifier, and the Linux MIME type).
const LAHIQAT_RUQAA: &str = "ruqaa";

/// What this build is and where it keeps its data.
///
/// Small on purpose: it is read once at startup by the interface shell and then
/// shown in the status strip and copied into diagnostics reports, so every
/// field in it has to be something a maintainer would ask for first.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
struct MaalumatTaarib {
    /// The application version, matching the workspace version.
    isdar: String,
    /// The operating system this build is running on.
    nizam: NizamTashghil,
    /// The processor architecture this build was compiled for.
    mimariya: Mimariya,
    /// The resolved data root, after the `TAARIB_BAYANAT` override and the
    /// platform defaults have both been applied.
    jidhr_bayanat: String,
    /// Which signing identity this build trusts: `tatwir` or `isdar`.
    hawiyat_thiqa: String,
}

/// The current settings.
///
/// Returned by value rather than watched, because the interface holds them in
/// its query cache and a settings change republishes the whole tree anyway.
///
/// # Errors
///
/// Cannot currently fail; the signature is the uniform command contract.
#[allow(
    clippy::unnecessary_wraps,
    reason = "every command returns Result<T, Khata> so the generated TypeScript has one call \
              shape and one error shape across the whole surface; `allow` rather than `expect` \
              because a command that grows a real failure path must not then trip an unfulfilled \
              expectation"
)]
#[tauri::command]
#[specta::specta]
fn idadat_hali(makhzan: tauri::State<'_, Arc<MakhzanIdadat>>) -> Result<Idadat, Khata> {
    Ok(Arc::unwrap_or_clone(makhzan.hali()))
}

/// What this build is and where it keeps its data.
///
/// # Errors
///
/// Cannot currently fail; the signature is the uniform command contract.
#[allow(
    clippy::unnecessary_wraps,
    reason = "every command returns Result<T, Khata> so the generated TypeScript has one call \
              shape and one error shape across the whole surface; `allow` rather than `expect` \
              because a command that grows a real failure path must not then trip an unfulfilled \
              expectation"
)]
#[tauri::command]
#[specta::specta]
fn maalumat_taarib(masarat: tauri::State<'_, Masarat>) -> Result<MaalumatTaarib, Khata> {
    Ok(MaalumatTaarib {
        isdar: ISDAR.to_owned(),
        nizam: NizamTashghil::hali(),
        mimariya: Mimariya::hali(),
        // Lossy rather than `display()` so a path with a lone surrogate on
        // Windows still reaches the interface as text instead of vanishing.
        jidhr_bayanat: masarat.jidhr_bayanat().to_string_lossy().into_owned(),
        hawiyat_thiqa: taarib_khatm::MIRSAT_MALIK.hawiya.wasm().to_owned(),
    })
}

/// One row of the library, exactly as the grid draws it.
///
/// Assembled here rather than in the interface because every field is the
/// result of a decision the Rust side already made — which launcher won the
/// deduplication, whether the existence gate admitted the game, which plate a
/// game with no artwork gets. Recomputing any of that in TypeScript would be a
/// second opinion, and the two would drift the first time either changed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
struct SijillMaktaba {
    /// Taarib's identity for the game.
    muarrif: String,
    /// Whether the probe has ever examined this game, so the three fields below
    /// are answers rather than defaults.
    ///
    /// [`Self::muharrik`] and [`Self::tabaqa`] both fall back to a pessimistic
    /// value when no report is stored, and each fallback is indistinguishable
    /// from a real answer: `Majhul` is what the probe records for a game it
    /// examined and did not recognise, and it is also what this row carries for
    /// a game nothing has examined. `TarjamaFawqiya` is a verdict when probed
    /// and a floor when not. Without this field the card cannot tell "we looked
    /// and found nothing" from "nothing has been looked at" — the distinction
    /// [`Self::lugha_rasmiya`] documents below, and a sharper one here, because
    /// `Majhul` is the answer for most of a real library.
    ///
    /// True for a report an older probe version wrote, which is deliberate: a
    /// stale report is still served rather than withheld, so it is still an
    /// examination and still what the interface is showing. The re-probe sweep
    /// replaces it in the background and this row is rebuilt from the new one.
    mafhusa: bool,
    /// The identified engine family, from the cached probe; unknown until probed.
    muharrik: AilatMuharrik,
    /// The injection tier, from the cached probe; the overlay floor until probed.
    tabaqa: Tabaqa,
    /// Whether this build can actually drive that tier on that engine.
    ///
    /// The tier says what the engine allows; this says whether Taarib has
    /// finished the code that uses it. A card must be able to say "installs
    /// but changes nothing on screen", and only this field carries that.
    jahiziya: JahiziyatTashghil,
    /// The Arabization status the card badges, computed from real local facts.
    hala: taarib_mustalahat::luba::HalatLuba,
    /// Whether the publisher already ships Arabic, from the shallow pass.
    ///
    /// Shallow on purpose: the launcher's declared languages and the
    /// locale-shaped paths under the install root, with no container opened. A
    /// grid of a hundred games cannot pay a container decode each, and this
    /// answer is only ever used to mark a card. The install path asks
    /// [`luba_awamir::lugha_rasmiya`], which opens them.
    /// A `Ghaib` here can mean "we looked and there is none" or "nothing could
    /// be looked at"; the card draws neither, because it only badges a game
    /// that *does* already speak Arabic. The difference is on the game screen,
    /// where [`luba_awamir::lugha_rasmiya`] carries the evidence and what it
    /// could not see.
    lugha_rasmiya: taarib_mustalahat::luba::HalatLughaRasmiya,
    /// The text installation's card state.
    nass: taarib_mustalahat::sawt::HalatSawt,
    /// The voice installation's card state.
    sawt: taarib_mustalahat::sawt::HalatSawt,
    /// The primary launcher's name, in Arabic.
    ism_manassa: String,
    /// The name its launcher gives it, verbatim.
    ism: String,
    /// Every launcher that reports it, primary first.
    masadir: Vec<String>,
    /// The shard family its patches are published under.
    manassa: Manassa,
    /// The install root.
    jidhr: String,
    /// Size on disk as the launcher recorded it, zero when it did not.
    #[specta(type = specta_typescript::Number)]
    hajm: u64,
    /// When it was last played, RFC 3339, where the launcher records it.
    akhir_laab: Option<String>,
    /// When the launcher last updated it.
    akhir_tathbeet: Option<String>,
    /// The vertical cover, when the artwork cache already holds one.
    ///
    /// An absolute path under the cache, not a key: the stored value is a
    /// content address, and turning it into a path is the check that keeps a
    /// corrupted row from naming a file outside the cache. `None` here means
    /// "not on this disk yet", never "this game has no cover" — the artwork pass
    /// that runs after the grid announces every one it resolves on
    /// [`suwar_awamir::ISM_HADATH_GHILAF`].
    ghilaf: Option<String>,
    /// The landscape banner, on the same terms.
    ///
    /// Sent beside the cover rather than instead of it because the two are
    /// available in different proportions: every launcher downloads a header for
    /// its own list, and only some of them download the portrait. A card that
    /// wants a wide well can have one for every game; a card that wants a
    /// portrait falls back to the plate more often.
    batl: Option<String>,
    /// The dominant colour of whichever picture the cover cascade settled on.
    ///
    /// Extracted once at import and stored beside the image, so the grid gets it
    /// for the cost of reading the row. It is what lets a card tint its own
    /// frame from the artwork it is showing.
    lawn: Option<LawnBariz>,
    /// The generated plate, present whenever `ghilaf` is `None`.
    lawha: Option<LawhaBadila>,
    /// How many Arabic translations other teams have published for this game,
    /// as the cached community index lists them.
    ///
    /// Read from the cache alone, never from the network: this scan runs on the
    /// thread the window is driven from, and the refresh that keeps the cache
    /// current runs in the background. Zero therefore means either that the
    /// index lists nothing for this game or that no index is cached yet, and
    /// the card draws neither — the mark is for the game that has one. The game
    /// screen, which can wait for a fetch, is where the two are told apart.
    tarjamat_mujtama: u32,
}

/// What a library row takes from a game's stored probe report.
///
/// Named rather than a tuple because [`Self::mafhusa`] is the only thing that
/// tells the other four apart from the values that stand in for them when no
/// report exists — and because five values, two of them `bool`, is a tuple that
/// can be destructured in the wrong order without the compiler noticing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KhulasatFahs {
    /// Whether a report was found at all.
    mafhusa: bool,
    /// The engine family the probe identified.
    aila: AilatMuharrik,
    /// The tier the report awards.
    tabaqa: Tabaqa,
    /// Whether the safety layer refuses the game outright.
    marfuda: bool,
    /// Whether this build can drive that tier on that engine.
    jahiziya: JahiziyatTashghil,
}

impl KhulasatFahs {
    /// Reads a stored report, or states what stands in for a missing one.
    ///
    /// The stand-ins are the pessimistic values on purpose: an unexamined game
    /// is described as unrecognised, overlay-only and unimplemented rather than
    /// promised anything. `mafhusa` is what keeps that pessimism from reading as
    /// a verdict — every one of those four is also a real answer the probe
    /// gives, so without the flag the row cannot say which it is holding.
    fn min_taqreer(taqreer: Option<&TaqreerImkaniyat>) -> Self {
        taqreer.map_or(
            Self {
                mafhusa: false,
                aila: AilatMuharrik::Majhul,
                tabaqa: Tabaqa::TarjamaFawqiya,
                marfuda: false,
                jahiziya: JahiziyatTashghil::Ghaiba,
            },
            |taqreer| Self {
                mafhusa: true,
                aila: taqreer.muharrik.aila,
                tabaqa: taqreer.tabaqa,
                marfuda: taqreer.marfuda,
                jahiziya: taqreer.jahiziya,
            },
        )
    }
}

/// A game a launcher lists that is not in the library, and why.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
struct SijillGhaib {
    /// Taarib's identity for it.
    muarrif: String,
    /// Its name.
    ism: String,
    /// Which launcher reports it.
    manassa: Manassa,
    /// Why it is absent.
    sabab: SababGhiyab,
}

/// A game that exists and whose own record could not be read or written.
///
/// Separate from [`SijillGhaib`] because it is not an absence. The game is on
/// disk and the launcher knows about it; what failed is Taarib's own database
/// or backup directory, and telling the user their game is missing would be
/// false. Kept as a whole [`Khata`] rather than a sentence so the code and the
/// next step survive the crossing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
struct SijillMutaadhir {
    /// Taarib's identity for it.
    muarrif: String,
    /// Its name.
    ism: String,
    /// Which launcher reports it.
    manassa: Manassa,
    /// What failed.
    khata: Khata,
}

/// The whole library, in one answer.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
struct HasilatMaktaba {
    /// Every game that passed the existence gate.
    alaab: Vec<SijillMaktaba>,
    /// Every game that did not, with its reason.
    ghaiba: Vec<SijillGhaib>,
    /// Every game that exists but whose record this scan could not complete.
    mutaadhira: Vec<SijillMutaadhir>,
    /// Which launchers were searched, so an empty library can name them.
    ///
    /// Only a launcher whose catalogue was read end to end is in here. One
    /// that is installed and could not be read is not "searched" in any sense
    /// the empty-library sentence can honestly use — and it is the case that
    /// sentence used to lie about, naming Epic on a machine whose Epic data
    /// folder had no manifest directory. Those launchers are in
    /// [`Self::matajir`] with their reason.
    manassat: Vec<String>,
    /// What the scan learned about every launcher on this platform — found or
    /// not, read whole or not, and every warning — with a sentence about each
    /// in both languages. The library's own report on where its games did and
    /// did not come from.
    matajir: Vec<maktaba_awamir::MatjarMaktabaHie>,
    /// What the deduplication pass did, for the diagnostics bundle.
    tawheed: String,
    /// What the existence gate concluded, likewise.
    wujud: String,
}

/// Scans every launcher and returns the library.
///
/// The whole pipeline in one call, in the order the phases established and for
/// the reasons they established it:
///
/// 1. **Scan** every adapter. Each reads its launcher's own catalogue and
///    returns what that launcher believes.
/// 2. **Deduplicate** by content fingerprint. A user with Steam, Heroic and
///    Lutris can have one game listed three times; merging by title would be
///    wrong in both directions, so this merges by what is on disk.
/// 3. **Gate** on existence. A catalogue entry is a belief; this is where the
///    filesystem is asked. Nothing is silently dropped — a game that fails is
///    returned in `ghaiba` with the reason, because a player who owns a game
///    and cannot find it in Taarib will conclude the product does not support
///    it, and they will be reasonable.
///
/// Artwork is **never fetched** here, and this is structural rather than a
/// matter of discipline: nothing on this path can reach the network. What it
/// does do is answer from Taarib's own artwork cache, which is a `stat` per
/// picture and costs nothing — a rescan therefore draws real covers on the first
/// frame instead of flashing plates and replacing them. Everything the cache
/// does not have is left to [`suwar_awamir::hassil_suwar_maktaba`], which the
/// library screen calls once the grid is on screen and which streams each cover
/// as it lands. Where each game's pictures live is handed over in
/// [`suwar_awamir::DhakiratSuwar`], because the launcher adapters are the only
/// thing that knows and re-deriving it would mean running this scan twice.
///
/// # Errors
///
/// [`Khata`] only when the scan context itself cannot be built — no home
/// directory, which is a machine this application cannot run on. An individual
/// launcher that fails is a warning inside the result, never an error out of
/// it: one unreadable catalogue must not cost the user their other sixteen
/// launchers.
#[tauri::command]
#[specta::specta]
fn maktaba(
    tatbiq: tauri::AppHandle,
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
    masarat: tauri::State<'_, Masarat>,
    qaida: tauri::State<'_, Makhzan>,
    dhakira: tauri::State<'_, suwar_awamir::DhakiratSuwar>,
    jawla: tauri::State<'_, Arc<jawla_awamir::HalatJawla>>,
) -> Result<HasilatMaktaba, Khata> {
    use taarib_makhzan::sijillat::{IdkhalLuba, SijillFahs, SijillRuqaa, SimaMukhzana, sima};
    use taarib_mustalahat::luba::LubaId;
    use taarib_mustalahat::sawt::HalatSawt;
    use taarib_tathbeet::bayan::NawTathbeet;

    let bidaya = std::time::Instant::now();
    let idadat = makhzan.hali();
    let siyaq = taarib_kashf::siyaq_fahs(idadat.manassat.clone(), masarat.manzil().to_path_buf())?;

    let kashif = taarib_kashf::Kashif::jadeed();
    let natija = kashif.ifhas(&siyaq)?;

    // Everything a launcher said about itself, taken before the games are
    // pulled out of the result: the warnings, the verdicts and the roots used
    // to be dropped on this line, which left every adapter's "reported in
    // Diagnostics" a promise nothing kept — and left the absence sweep below
    // keyed on whether a launcher's folder existed, which is not the question.
    maktaba_awamir::dawwin(&natija);
    let matajir = maktaba_awamir::matajir_hie(kashif.matajir(), &natija);
    let sijillat_matajir = maktaba_awamir::mukhzan_min_fahs(&natija);
    // Only a launcher whose catalogue was read end to end was searched in the
    // sense the empty-library sentence uses, and only its stored games may be
    // marked absent below.
    let manassat: Vec<String> = natija
        .matajir_tamma()
        .into_iter()
        .map(str::to_owned)
        .collect();

    let madkhalat: Vec<LubaMuktashafa> = natija
        .matajir
        .into_iter()
        .flat_map(|wahid| wahid.alaab)
        .collect();
    let (muwahhada, taqreer_tawheed) = tawheed::wahhid(madkhalat);

    // Steam's declared languages, read once for the whole scan and shared by
    // every row in it. It is the only launcher whose catalogue this build reads
    // a language list out of, and `appinfo.vdf` is one file holding a quarter of
    // a million applications: streaming it once per game would be the same read
    // repeated per row. Kept for the rest of the process too, so a detail screen
    // opened afterwards does not read it again.
    let arqam_steam: Vec<u32> = {
        let mut arqam: Vec<u32> = muwahhada
            .iter()
            .filter_map(|luba| match luba.asasi.masdar.asl() {
                MasdarLuba::Steam(raqm) => Some(*raqm),
                _ => None,
            })
            .collect();
        arqam.sort_unstable();
        arqam.dedup();
        arqam
    };
    let mut tanbihat_lugha = Vec::new();
    let lughat_muallana = kashif
        .matajir()
        .iter()
        .find(|matjar| matjar.muarrif() == "steam")
        .and_then(|matjar| matjar.mawqi(&siyaq))
        .map(|jidhr_steam| {
            taarib_kashf::lugha_rasmiya::lughat_steam(
                &jidhr_steam,
                &arqam_steam,
                &mut tanbihat_lugha,
            )
        })
        .unwrap_or_default();
    for tanbih in &tanbihat_lugha {
        tracing::warn!(mawdi = %tanbih.mawdi, sabab = %tanbih.sabab, "declared languages");
    }
    luba_awamir::sajjil_lughat_matjar(lughat_muallana.clone());

    // The scan and its launcher records open together: the sweep at the end
    // reads `fahs_matjar.najah` for this generation, and a generation with no
    // launcher rows is one the store refuses to sweep for at all.
    let fahs = qaida.bi_muamala(|muamala| {
        let sijill = SijillFahs::jadeed(muamala);
        let fahs = sijill.ibda(true)?;
        for (matjar, tanbihat) in &sijillat_matajir {
            sijill.sajjil_natijat_matjar(fahs, matjar, tanbihat)?;
        }
        Ok(fahs)
    })?;

    let mut alaab = Vec::with_capacity(muwahhada.len());
    let mut ghaiba = Vec::new();
    let mut mutaadhira = Vec::new();
    let mut sijill = SijillWujud::jadeed();

    // Both are read once, before the loop, and neither can reach the network.
    // A cache root that cannot be opened is a run with no cover art rather than
    // a scan that fails, so it degrades to `None` and every card gets its plate.
    let khazina = suwar_awamir::khazina(&masarat).ok();
    let suwar_makhzuna = suwar_awamir::suwar_makhzuna(&qaida);
    let mut dhakhira_suwar: BTreeMap<LubaId, suwar_awamir::TalabSuwar> = BTreeMap::new();
    // The community index on the same terms: the cached copy or nothing, read
    // once for every row. The background refresh started below is what fills
    // it for the next scan when there is none yet.
    let fahras_mujtama = mujtama_awamir::fahras_mukhazzan(&masarat);

    for luba in muwahhada {
        let asasi = &luba.asasi;
        if !asasi.hiya_luba() {
            continue;
        }
        let talab = TalabWujud {
            masdar: &asasi.masdar,
            jidhr: &asasi.jidhr,
            tanfidhi: asasi.tanfidhi.as_deref(),
            shahid: ShahidTanfidhi::khali(),
            hala_matjar: asasi.hala_matjar.clone(),
            beea: None,
            masar_windows: None,
        };
        let hukm = wujud::ihkum(&talab);
        let id = LubaId::min_masdar(&asasi.masdar, &asasi.ism);

        if let Some(sabab) = hukm.ghiyab.clone() {
            ghaiba.push(SijillGhaib {
                muarrif: asasi.masdar.muarrif(),
                ism: asasi.ism.clone(),
                manassa: asasi.masdar.aila(),
                sabab,
            });
            sijill.sajjil(&asasi.masdar, hukm);
            continue;
        }
        sijill.sajjil(&asasi.masdar, hukm);

        let sajl = Luba {
            id,
            masadir: luba.masadir(),
            ism: asasi.ism.clone(),
            jidhr: asasi.jidhr.clone(),
            tanfidhi: asasi.tanfidhi.clone(),
            hajm: asasi.hajm,
            akhir_laab: asasi.akhir_laab.clone(),
            akhir_tahdith: asasi.akhir_tahdith.clone(),
            bina: None,
            suwar: taarib_mustalahat::luba::SuwarLuba::default(),
            beea: asasi.beea.clone(),
            mawjuda: true,
            mukhfiya: false,
        };
        let simat: Vec<SimaMukhzana> = asasi
            .simat
            .iter()
            .filter_map(|sima| match sima {
                SimatLuba::JamaiMahalli => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::JAMAI_MAHALLI,
                    "",
                )),
                SimatLuba::JamaiOnline => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::JAMAI_ONLINE,
                    "",
                )),
                SimatLuba::HimayaMuhtamala(ism) => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::HIMAYA_MUHTAMALA,
                    ism,
                )),
                SimatLuba::MuammanaVac => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::MUAMMANA_VAC,
                    "",
                )),
                SimatLuba::LaysatLuba(naw) => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::LAYSAT_LUBA,
                    naw,
                )),
                SimatLuba::TabaqatTawafuq(tabaqa) => Some(SimaMukhzana::jadeeda(
                    asasi.masdar.aila().slug(),
                    sima::TABAQAT_TAWAFUQ,
                    tabaqa,
                )),
                SimatLuba::MuktashafaBilIstidlal(_) | SimatLuba::MuhakatRum(_) => None,
            })
            .collect();
        // Everything from here down touches Taarib's own database and backup
        // directory, per game. A failure in any of it used to leave the whole
        // scan by `?`, which cost the user their entire library because one
        // game's row would not write. It is collected instead: the scan
        // finishes, and the games it could not finish are named.
        let mahsula: Result<SijillMaktaba, Khata> = (|| {
            qaida.bi_muamala(|muamala| {
                SijillAlaab::jadeed(muamala).sajjil(&IdkhalLuba {
                    luba: &sajl,
                    muktamila: asasi.muktamila,
                    khiyarat_tashghil: asasi.khiyarat_tashghil.as_deref(),
                    simat: &simat,
                    fahs,
                })
            })?;

            let taqreer = qaida.bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?;
            let KhulasatFahs {
                mafhusa,
                aila,
                tabaqa,
                marfuda,
                jahiziya,
            } = KhulasatFahs::min_taqreer(taqreer.as_ref());

            let nusakh = luba_awamir::jidhr_nusakh(&masarat, &qaida, id)?;
            let nass_muthabbat = luba_awamir::muthabbat(&sajl.jidhr, &nusakh, NawTathbeet::Nass);
            let sawt_muthabbat = luba_awamir::muthabbat(&sajl.jidhr, &nusakh, NawTathbeet::Sawt);

            let ruqa_makhbua = qaida.bil_qira(|ittisal| {
                SijillRuqaa::jadeed(ittisal)
                    .li_luba(asasi.masdar.aila().slug(), &asasi.masdar.muarrif())
            })?;
            // One rule, shared with the sweep that patches this row in place
            // once its probe has run: the badge a scan draws and the badge a
            // probe event redraws must not be decided twice.
            let hala = jawla_awamir::halat_luba(jawla_awamir::HaqaiqHala {
                marfuda,
                nass_muthabbat,
                ruqaa_mutaha: !ruqa_makhbua.is_empty(),
                tabaqa,
            });
            let nass_hala = if nass_muthabbat {
                HalatSawt::Mutabbaqa
            } else if !ruqa_makhbua.is_empty() {
                HalatSawt::Mutaha
            } else {
                HalatSawt::LaShay
            };
            let sawt_hala = if sawt_muthabbat {
                HalatSawt::Mutabbaqa
            } else {
                HalatSawt::LaShay
            };

            // Every game gets a plate. The cover, when one resolves, is drawn over
            // it — see the card, where the two share one rectangle so the well is
            // never empty for a frame.
            let lawha = LawhaBadila::jadeeda(&asasi.masdar, &asasi.ism, SimatLawha::Daken);

            // Whatever the artwork cache already holds for this game, as paths.
            // The keys were written by a previous artwork pass; a key whose file
            // has since been evicted resolves to `None` and the plate stands.
            // The cache's own recency stamp is deliberately not touched here —
            // one write per card on every scan is what the eviction ledger was
            // built to avoid, and the artwork pass stamps what it resolves.
            let makhzuna = khazina
                .as_ref()
                .zip(suwar_makhzuna.get(&id))
                .map_or_else(suwar_awamir::GhilafMakhzun::default, |(khazina, suwar)| {
                    suwar_awamir::ghilaf_makhzun(khazina, suwar)
                });

            // The shallow official-Arabic pass. It opens nothing and answers
            // from the store's listing and the locale-shaped paths on disk,
            // which is what the card needs to mark a game that already speaks
            // Arabic. The deep pass belongs to the game screen, where the
            // install decision is actually made.
            let hukm_lugha = luba_awamir::hukm_sathi(
                &sajl,
                luba_awamir::appid_steam(&sajl).and_then(|raqm| lughat_muallana.get(&raqm)),
            );

            Ok(SijillMaktaba {
                muarrif: id.to_string(),
                mafhusa,
                muharrik: aila,
                tabaqa,
                jahiziya,
                hala,
                lugha_rasmiya: hukm_lugha.hala(),
                nass: nass_hala,
                sawt: sawt_hala,
                ism_manassa: asasi.masdar.ism_arabi().to_owned(),
                ism: asasi.ism.clone(),
                masadir: luba.masadir().iter().map(MasdarLuba::muarrif).collect(),
                manassa: asasi.masdar.aila(),
                jidhr: asasi.jidhr.to_string_lossy().into_owned(),
                hajm: asasi.hajm,
                akhir_laab: asasi.akhir_laab.clone(),
                akhir_tathbeet: asasi.akhir_tahdith.clone(),
                ghilaf: makhzuna.ghilaf,
                batl: makhzuna.batl,
                lawn: makhzuna.lawn,
                lawha: Some(lawha),
                tarjamat_mujtama: fahras_mujtama
                    .as_ref()
                    .map_or(0, |fahras| mujtama_awamir::adad_li_luba(fahras, &sajl)),
            })
        })();
        match mahsula {
            Ok(sijill_luba) => {
                // Where this game's pictures live, for the pass that runs once
                // the grid is drawn. Recorded only for a game that made it into
                // the library: artwork for a row nobody can see is a fetch
                // nobody asked for.
                let _ = dhakhira_suwar.insert(
                    id,
                    suwar_awamir::TalabSuwar {
                        ism: asasi.ism.clone(),
                        masadir: asasi.suwar.clone(),
                    },
                );
                alaab.push(sijill_luba);
            },
            Err(khata) => mutaadhira.push(SijillMutaadhir {
                muarrif: id.to_string(),
                ism: asasi.ism.clone(),
                manassa: asasi.masdar.aila(),
                khata,
            }),
        }
    }

    qaida.bi_muamala(|muamala| {
        // Marking rows absent is the game record's own operation, not the scan
        // record's: `SijillAlaab` owns the `luba` table this updates. The list
        // holds only launchers read whole, and the store checks that again
        // against what this scan recorded; a refusal here means the two
        // disagree, which is worth a line in the log rather than silence.
        for manassa in &manassat {
            match SijillAlaab::jadeed(muamala).allim_ghayr_mawjud(fahs, manassa)? {
                HasilatMash::Jarat { adad } => {
                    tracing::debug!(manassa = %manassa, adad, "absence sweep");
                },
                HasilatMash::Rufidat => {
                    tracing::warn!(
                        manassa = %manassa,
                        "the store refused an absence sweep for a launcher this scan did not \
                         record as read whole"
                    );
                },
            }
        }
        SijillFahs::jadeed(muamala).anhi(fahs, u32::try_from(alaab.len()).unwrap_or(u32::MAX))
    })?;

    dhakira.ikhzin(dhakhira_suwar);
    // After the rows are built, never before: the scan reads the cache and
    // nothing else, and this is the refresh that fills it for the next one.
    mujtama_awamir::dhamin_mujaddid_mujtama(&masarat, &makhzan);

    tracing::info!(
        alaab = alaab.len(),
        ghaiba = ghaiba.len(),
        mutaadhira = mutaadhira.len(),
        bi_ghilaf = alaab.iter().filter(|saf| saf.ghilaf.is_some()).count(),
        muddat_ms = bidaya.elapsed().as_millis(),
        "the library scan finished"
    );

    // The answer is complete; the probes this scan did not run go to a task of
    // their own, so a card says "not probed yet" for seconds rather than for
    // ever and nothing above waits on a single executable being read.
    jawla_awamir::ibda_jawla(
        tatbiq,
        Arc::clone(&jawla),
        Makhzan::clone(&qaida),
        masarat.inner().clone(),
    );

    Ok(HasilatMaktaba {
        alaab,
        ghaiba,
        mutaadhira,
        manassat,
        matajir,
        tawheed: taqreer_tawheed.satr(),
        wujud: sijill.taqreer(),
    })
}

/// Opens the game's folder in the platform file manager.
///
/// # Errors
///
/// [`Khata`] when the game is unknown or the file manager will not start.
#[tauri::command]
#[specta::specta]
fn iftah_manassa(muarrif: String, qaida: tauri::State<'_, Makhzan>) -> Result<bool, Khata> {
    let id = luba_awamir::huwiya(muarrif)?;
    let luba = luba_awamir::ijlib_luba(&qaida, id)?;
    let (barnamij, wusata): (&str, Vec<std::ffi::OsString>) = match NizamTashghil::hali() {
        NizamTashghil::Windows => ("explorer", vec![luba.jidhr.into()]),
        NizamTashghil::Mac => ("open", vec![luba.jidhr.into()]),
        NizamTashghil::Linux => ("xdg-open", vec![luba.jidhr.into()]),
    };
    std::process::Command::new(barnamij)
        .args(wusata)
        .spawn()
        .map_err(|sabab| {
            Khata::min_tafsir(&KhataStudio::FathMujalladFashil {
                tafsil: sabab.to_string(),
            })
        })?;
    Ok(true)
}

/// Adds a folder to the scanned set, so its games enter the next scan.
///
/// # Errors
///
/// [`Khata`] when the settings cannot be written.
#[tauri::command]
#[specta::specta]
fn adif_mujallad_fahs(
    masar: String,
    makhzan: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<u32, Khata> {
    // The settings hold real paths, not the text the interface sent.
    let masar = PathBuf::from(masar);
    if !masar.is_absolute() {
        return Err(Khata::from(KhataStudio::MujalladGhayrMutlaq {
            masar: masar.to_string_lossy().into_owned(),
        }));
    }
    makhzan.ghayyir(|idadat| {
        if !idadat.manassat.mujalladat_idafiya.contains(&masar) {
            idadat.manassat.mujalladat_idafiya.push(masar.clone());
        }
    })?;
    let adad = makhzan.hali().manassat.mujalladat_idafiya.len();
    Ok(u32::try_from(adad).unwrap_or(u32::MAX))
}

fn main() -> Natija<()> {
    // Where a failure that lands before the rolling log exists can still be
    // written down. Filled the moment the layout resolves, which is the first
    // thing `iqla` does and the only step that can abort before it.
    let mut mujallad_sijillat: Option<PathBuf> = None;
    match iqla(&mut mujallad_sijillat) {
        Ok(()) => Ok(()),
        Err(khata) => {
            aalin_fashal_bidaya(&khata, mujallad_sijillat.as_deref());
            Err(khata)
        },
    }
}

/// The whole launch, so that every way it can end early ends in one place.
///
/// `main` used to be this function, and each `?` in it was a process that
/// disappeared. A release build is `windows_subsystem = "windows"`: the `Err`
/// rustc prints for a failing `main` goes to a handle that does not exist, so a
/// user double-clicked the icon and nothing happened at all — no window, no
/// message, and for the two aborts that came before `sijill::hayyi`, not even a
/// line in any log. Splitting the body out gives every abort a caller, and
/// [`aalin_fashal_bidaya`] gives it a face.
///
/// # Errors
///
/// Whatever the launch could not proceed without: the path layout, the
/// diagnostics pipeline, the database, the generated bindings in a debug build,
/// or the window itself. The settings are deliberately no longer on that list —
/// see [`iftah_idadat`].
fn iqla(mujallad_sijillat: &mut Option<PathBuf>) -> Natija<()> {
    let masarat = Masarat::iktashif()?;
    *mujallad_sijillat = Some(masarat.sijillat());
    // Collect-and-continue rather than fail-fast: a first run with one
    // unwritable corner must still reach the window, where the diagnostics
    // screen can say which corner.
    let mut tahdheerat_bidaya = bidaya::jahhiz(&masarat);

    // Never fails, for the reason [`iftah_idadat`] gives: settings this process
    // cannot read cost the user their preferences for one launch, not the
    // launch itself.
    let (makhzan, taadhur_idadat) = iftah_idadat(&masarat);
    let idadat = makhzan.hali();

    // Only now, because the setting that redirects it lives in the file the
    // sweep above had to create the directory for. Settings → Storage has
    // always let people move where patches are kept, and until this line
    // nothing on this side read the value: they set it, and Taarib went on
    // writing to the default root.
    let masarat = masarat.maa_jidhr_ruqaa(idadat.takhzin.jidhr_ruqaa.as_deref());
    tahdheerat_bidaya.extend(bidaya::jahhiz_ruqaa(&masarat));

    let haris = sijill::hayyi(&masarat, idadat.tashkhis.mustawa)?;
    // After the subscriber exists, so nothing is emitted into a dropped sink,
    // and before the first failure these warnings may explain.
    bidaya::sajjil(&tahdheerat_bidaya);
    if let Some(taadhur) = &taadhur_idadat {
        aalin_taadhur_idadat(taadhur, makhzan.masar());
    }

    // The uninstaller's restore offer runs headless and exits; it must come
    // after the stores are reachable but before any window is built.
    if istiada_cli::shaghghil_idha_talab() {
        return Ok(());
    }

    // An update that was interrupted mid-swap left the previous version beside
    // the new one. This is the only place that can tell the difference — the
    // running process is the proof that the swap either finished or did not.
    match std::env::current_exe() {
        Ok(tanfidhi) => match taarib_tahdith::tahaqqaq_bad_iqla(&tanfidhi) {
            Ok(Some(sabiq)) => {
                tracing::info!(sabiq = %sabiq.display(), "the superseded version was removed");
            },
            Ok(None) => {},
            Err(khata) => {
                // The previous version was put back, so this launch is the old
                // one running: a warning, not a failure to start.
                tracing::warn!(khata = %Khata::min_tafsir(&khata).li_sijill(), "update rollback");
            },
        },
        Err(sabab) => {
            tracing::warn!(%sabab, "the running executable could not be located");
        },
    }

    // Opened after diagnostics start, because bringing the schema forward is
    // the first thing in this process that can fail on a user's machine for a
    // reason only the log will explain.
    let qaida = Makhzan::iftah(&masarat)?;

    tracing::info!(
        isdar = ISDAR,
        nizam = ?NizamTashghil::hali(),
        mimariya = ?Mimariya::hali(),
        bayanat = %masarat.jidhr_bayanat().display(),
        idadat = %makhzan.masar().display(),
        qaida = %qaida.masar().display(),
        sijillat = %haris.masar().display(),
        "Taarib Studio starting"
    );

    let banni = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            idadat_hali,
            maalumat_taarib,
            maktaba,
            maktaba_awamir::fahs_akhir,
            suwar_awamir::hassil_suwar_maktaba,
            iftah_manassa,
            adif_mujallad_fahs,
            jawla_awamir::halat_jawla,
            luba_awamir::tafasil_luba,
            luba_awamir::afhas_muharrik,
            luba_awamir::dalail_muharrik,
            luba_awamir::lugha_rasmiya,
            luba_awamir::fahs_himaya,
            luba_awamir::ikhfa_luba,
            aql_awamir::aql_luba,
            tathbeet_awamir::ruqaa_luba,
            tathbeet_awamir::nazzil_ruqaa,
            tathbeet_awamir::tahaqquq_ruqaa,
            tathbeet_awamir::azil_ruqaa,
            tathbeet_awamir::hal_tashtaghil,
            tathbeet_awamir::iqrar_aman,
            tathbeet_awamir::sajjil_iqrar_aman,
            tathbeet_awamir::thabbit_ruqaa,
            tathbeet_awamir::khuttat_tathbeet,
            tathbeet_awamir::khuttat_izala,
            warsha_awamir::nusus_warsha,
            warsha_awamir::haddith_tarjama,
            warsha_awamir::iqtirahat_nass,
            warsha_awamir::tatbiq_iqtirah,
            warsha_awamir::tarjim_nass,
            warsha_awamir::tarjim_dufa,
            warsha_awamir::alamat_mashru,
            warsha_awamir::wahhid_mustalah,
            warsha_awamir::muayana,
            warsha_awamir::taaliqat_warsha,
            warsha_awamir::idmaj_huzma,
            warsha_awamir::qarrir_nizaat,
            warsha_awamir::anqidh_mashru,
            taqdeem_awamir::jalsati,
            taqdeem_awamir::musawwadat_luba,
            taqdeem_awamir::jahhiz_taqdeem,
            taqdeem_awamir::aqirr_tahdheer,
            taqdeem_awamir::sallim_taqdeem,
            taqdeem_awamir::musahamati,
            taqdeem_awamir::tabur_muraja,
            taqdeem_awamir::tafasil_muraja,
            taqdeem_awamir::allaq_muraja,
            taqdeem_awamir::qarrir_muraja,
            taqdeem_awamir::iaatimad_muraja,
            taqdeem_awamir::sijill_muraja_kull,
            taqdeem_awamir::sandooq_thabbit,
            taqdeem_awamir::sandooq_atliq,
            taqdeem_awamir::sandooq_imsah,
            taqdeem_awamir::lawhat_talabat,
            taqdeem_awamir::utlub_tarjama,
            taqdeem_awamir::adad_talabat,
            taqdeem_awamir::abda_tawthiq_taqdeem,
            idadat_awamir::haddith_idadat,
            idadat_awamir::khzin_itimad_muzawwid,
            idadat_awamir::imsah_itimad_muzawwid,
            idadat_awamir::hal_itimad_muzawwid,
            idadat_awamir::ikhtar_khatt,
            idadat_awamir::khutut_mutaha,
            tabaqa_awamir::manatiq_luba,
            tabaqa_awamir::adif_mintaqa,
            tabaqa_awamir::haddith_mintaqa,
            tabaqa_awamir::ihdhif_mintaqa,
            tabaqa_awamir::sijill_qira_luba,
            tabaqa_awamir::imsah_sijill_qira,
            tabaqa_awamir::nass_ifsah,
            tabaqa_awamir::aqirr_ifsah,
            musharaka_awamir::jahhiz_musharaka,
            musharaka_awamir::saddir_musharaka,
            musharaka_awamir::afhas_musharaka,
            musharaka_awamir::idmij_musharaka,
            tahdith_awamir::tahaqquq_tahdith,
            tahdith_awamir::nazzil_tahdith,
            tashkhis_awamir::sijillat_akhira,
            tashkhis_awamir::taqreer_tawafuq,
            tashkhis_awamir::huzmat_tashkhis,
            tashkhis_awamir::iftah_tashkhis,
            tilqai_awamir::hukm_tilqai,
            tilqai_awamir::laqtat_tilqai,
            tilqai_awamir::ibda_tilqai,
            tilqai_awamir::alghi_tilqai,
            tilqai_awamir::halat_iltiqat,
            tilqai_awamir::sajjil_iltiqat,
            mujtama_awamir::tarjamat_mujtama,
            mujtama_awamir::iftah_rabt,
        ])
        // A type the interface consumes that no command returns. The library
        // screen groups unavailable games into four headings and derives the
        // grouping itself — the wire form is the flat list the store already
        // has — so `FiatGhiyab` reaches the frontend without ever appearing in
        // a command's signature. Registered explicitly so it is generated from
        // its Rust definition rather than written a second time by hand.
        .typ::<taarib_mustalahat::ghiyab::FiatGhiyab>()
        // The same situation, for the same reason: the artwork pass announces
        // each cover on a window event rather than in its answer, so no command
        // signature mentions the payload and nothing would generate it. The
        // interface subscribes with `listen<GhilafHie>`, and this is what makes
        // that type the one Rust declared rather than one written twice.
        .typ::<suwar_awamir::GhilafHie>()
        // And again: the background engine sweep announces each finished game
        // on a window event, and the grid patches the row from that payload.
        // Its standing is a command's answer and generates itself; this one is
        // not, so it is registered here.
        .typ::<jawla_awamir::FahsMuharrikHie>();

    // Debug only: a release build ships the bindings that were generated when
    // it was developed, and must never write into the source tree it was
    // built from.
    #[cfg(debug_assertions)]
    banni
        .export(
            // No global bigint policy any more: `specta-typescript` 0.0.12
            // refuses a bare `u64`/`usize`/`i64` outright, so each field that
            // crosses says for itself whether it accepts f64 precision, with
            // `#[specta(type = Number)]` at its declaration.
            specta_typescript::Typescript::default().header(TARWEESAT_TAWLEED),
            MASAR_RUBUT,
        )
        .map_err(|q| KhataStudio::TasdirRubut {
            masar: MASAR_RUBUT.to_owned(),
            tafsil: q.to_string(),
        })?;

    let mujallad_sijillat = masarat.sijillat();
    // Proved here, once, rather than inside the sweep: the quarantine directory
    // is one component below the data root, and this is what makes it
    // impossible for the sweep to be handed the root itself.
    let jidhr_hajr = masarat.hadaf_hadhf(&masarat.hajr())?;
    // The setup closure needs the paths, and `masarat` itself is moved into
    // managed state before it runs.
    let masarat_zamin = masarat.clone();
    // The settings store is moved into managed state below, and the setup hook
    // needs it to start the revocation refresh.
    let makhzan_zamin = Arc::clone(&makhzan);
    let ayyam_hifz = idadat.tashkhis.ayyam_hifz;
    let hadd_hajm_mb = idadat.tashkhis.hadd_hajm_mb;
    let sima_mabdai = idadat.sima;

    tauri::Builder::default()
        // First, as the plugin requires. Registering `.ruqaa` as a file type
        // means the operating system launches this executable again while it is
        // already running; the second launch hands its arguments here and exits,
        // instead of standing up a second window over the same SQLite store. The
        // window the user already has is brought forward, and the patch — if the
        // launch carried one — goes to [`istaqbil_ruqaa`], which is the same
        // route the cold launch takes from `setup`.
        .plugin(tauri_plugin_single_instance::init(|tatbiq, hujaj, _mujallad| {
            tracing::info!(hujaj = ?hujaj, "a second launch handed its arguments over");
            let nafidha =
                tauri::Manager::get_webview_window(tatbiq, idadat_awamir::NAFIDHA_RAISIYA);
            if let Some(nafidha) = nafidha
                && let Err(sabab) = nafidha.set_focus()
            {
                tracing::warn!(sabab = %sabab, "the running window could not be brought forward");
            }
            // After the focus call, so the window the confirmation belongs to is
            // already in front of the user when the confirmation appears.
            if let Some(masar) = masar_ruqaa(hujaj.iter().map(PathBuf::from)) {
                istaqbil_ruqaa(tatbiq, masar);
            }
        }))
        // The one way this process hands an address to the browser. The
        // interface never calls the plugin's own command; it calls
        // `mujtama_awamir::iftah_rabt`, which checks the address against the
        // community index before this plugin ever sees it. The capability still
        // grants the plugin's command for `https://**` alone, so that if the
        // interface ever did call it, nothing but an https page could open.
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(banni.invoke_handler())
        .manage(masarat)
        .manage(makhzan)
        .manage(qaida)
        .manage(haris)
        .manage(suwar_awamir::DhakiratSuwar::default())
        // The one background engine sweep this process may run, and where it
        // stands. Behind an `Arc` because the task that runs it outlives the
        // command that started it and must share the same counters the
        // `halat_jawla` command reads.
        .manage(Arc::new(jawla_awamir::HalatJawla::default()))
        .manage(warsha_awamir::JalasatDamj::default())
        .manage(taqdeem_awamir::JihazMuallaq::default())
        .manage(taqdeem_awamir::QuflTaqdeem::default())
        .manage(warsha_awamir::AqfalMashariya::default())
        // The gathered share draft and the verified bundle, held for the length of one consent
        // decision. Not on disk, and deliberately: a permit is about the entry set a person was
        // shown a moment ago, and a draft that outlived the window it was read in would let an
        // acknowledgement be spent on a payload nobody has looked at since.
        .manage(musharaka_awamir::HalatMusharaka::default())
        // The automatic runs this process started: their cancel handles and the
        // snapshot each one last published. Held here rather than in the store
        // because a cancel handle is a live object, and because a run that is
        // still going is a fact about this process and not about the disk — what
        // is on the disk is the journal, which is what a run resumes from after
        // the application has been closed and reopened.
        .manage(tilqai_awamir::MashawirTilqai::default())
        .setup(move |tatbiq| {
            // The bundle's own tree, before anything reads a component or a
            // bundled font. A build launched from cargo has no resource
            // directory, and every reader treats that as "nothing bundled".
            if let Ok(jidhr_mawarid) = tauri::Manager::path(tatbiq).resource_dir() {
                let jidhr_mawarid = jidhr_mawarid.join(MUJALLAD_MAWARID);
                mukawwinat_tahmil::sajjil_jidhr_mawarid(jidhr_mawarid.clone());
                let natija = mukawwinat_tahmil::zamin_mukawwinat(&jidhr_mawarid, &masarat_zamin);
                for satr in natija.taqreer() {
                    if natija.salima() {
                        tracing::info!("{satr}");
                    } else {
                        tracing::warn!("{satr}");
                    }
                }
            }

            // The chrome wears the saved palette from the first frame the window
            // shows; `tauri.conf.json` only knows the default.
            idadat_awamir::tabbiq_sima(tatbiq, sima_mabdai);

            // The revocation list refreshes from here, not only from the first
            // command that happens to need it. It is the product's one kill
            // switch — the way a compromised signing key or a patch that bricks
            // a game is withdrawn from every machine — and started from the
            // commands alone it never runs at all for somebody who opens the
            // application and does not visit a game page. The call is idempotent
            // per process, so the command-side starts stay correct and become
            // no-ops.
            tathbeet_awamir::dhamin_mujaddid_sahb(&masarat_zamin, &makhzan_zamin);

            // The artwork cache, and nothing else, is readable through the asset
            // protocol. The scope cannot be a static glob in `tauri.conf.json`
            // because the data root moves with `TAARIB_BAYANAT`, so it is
            // registered here against the path that was actually resolved.
            //
            // Recursive, and it has to be: the store shards by the first byte of
            // each digest, so every file the interface is ever handed lives one
            // level down at `suwar/<xx>/<rest>.<ext>`, and a non-recursive scope
            // would admit the root and refuse all 256 directories that actually
            // hold the pictures. Both paths this build emits — the cover and the
            // banner, from the scan and from the artwork pass alike — are
            // produced by `KhaziantSuwar::mahalli`, which builds nothing outside
            // this root.
            //
            // A cache directory that cannot be added to the scope is a run with
            // no cover art, not a run that should fail to start.
            if let Err(sabab) = tauri::Manager::asset_protocol_scope(tatbiq)
                .allow_directory(suwar_awamir::jidhr_suwar(&masarat_zamin), true)
            {
                tracing::warn!(sabab = %sabab, "cover art will not load this run");
            }

            // Log pruning reads and deletes files, so it never runs on the
            // thread the window is painted from: a user with a month of trace
            // logs would see the first frame arrive late for no visible reason.
            tauri::async_runtime::spawn(async move {
                // An interrupted install can leave a verified-or-not package in
                // quarantine; the sweep reclaims it before anything downloads.
                let hajr_natija = tokio::task::spawn_blocking({
                    let jidhr_hajr = jidhr_hajr.clone();
                    move || taarib_aman::sandooq_fak::tanzif_hajr(&jidhr_hajr)
                })
                .await;
                match hajr_natija {
                    Ok(Ok(())) => {}
                    Ok(Err(khata)) => {
                        use taarib_usus::khata::Tafsir as _;
                        tracing::warn!(khata = %khata.injilizi(), "quarantine sweep incomplete");
                    }
                    Err(sabab) => {
                        tracing::warn!(sabab = %sabab, "the quarantine sweep task did not finish");
                    }
                }
                let natija = tokio::task::spawn_blocking(move || {
                    sijill::nazzif_sijillat(&mujallad_sijillat, ayyam_hifz, hadd_hajm_mb);
                })
                .await;
                match natija {
                    Ok(()) => {}
                    Err(sabab) => {
                        tracing::warn!(sabab = %sabab, "the log pruning task did not finish");
                    }
                }
            });

            // The cold launch's half of the file association: this process was
            // started *by* a double-clicked `.ruqaa`, and the path is sitting in
            // its own arguments. Read here rather than in `iqla` because routing
            // it needs the window and the managed state, and neither exists
            // until this closure runs.
            if let Some(masar) = masar_ruqaa(std::env::args_os().map(PathBuf::from)) {
                istaqbil_ruqaa(tauri::Manager::app_handle(tatbiq), masar);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|q| KhataStudio::TaadhurTashghil { tafsil: q.to_string() })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// A launch that fails where nobody can see it
// ---------------------------------------------------------------------------

/// Puts a startup failure in front of the person who is waiting for a window.
///
/// Three surfaces, in increasing order of how likely each is to exist. The
/// tracing event, which reaches the rolling log for a failure raised after
/// `sijill::hayyi` and is dropped for one raised before it. A block appended to
/// `sijillat/iqla_fashil.log`, written to the file directly rather than through
/// tracing, because the case this exists for is the one where the diagnostics
/// pipeline is itself what refused. And a native dialog, which is the only one
/// of the three the user will actually see — a `windows_subsystem = "windows"`
/// binary has nowhere to print, so without it the entire failure is a
/// double-click that does nothing.
///
/// The dialog's result is ignored, and so is every failure writing the file: by
/// the time either could fail this function is already the last resort.
fn aalin_fashal_bidaya(khata: &Khata, mujallad_sijillat: Option<&Path>) {
    tracing::error!(khata = %khata.li_sijill(), "Taarib could not start");

    let matn = format!(
        "تعذّر بدء تشغيل تعريب.\n\n{}\n\nTaarib could not start.\n\n{}\n\n[{}]",
        khata.arabi, khata.injilizi, khata.ramz
    );
    if let Some(mujallad) = mujallad_sijillat {
        uktub_sijill_iqla(mujallad, &matn);
    }
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(UNWAN_HIWAR)
        .set_description(&matn)
        .set_buttons(MessageButtons::Ok)
        .show();
}

/// Appends one dated block to the pre-window failure log.
///
/// The directory is created here rather than assumed: this runs for failures
/// that happen before, during and after `bidaya::jahhiz`, and one of the things
/// `jahhiz` can fail to create is this directory.
fn uktub_sijill_iqla(mujallad: &Path, matn: &str) {
    let _ = std::fs::create_dir_all(mujallad);
    let Ok(mut malaf) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(mujallad.join(ISM_SIJILL_IQLA))
    else {
        return;
    };
    let _ = writeln!(
        malaf,
        "--- {} — Taarib {ISDAR}\n{matn}",
        jiff::Timestamp::now()
    );
    let _ = malaf.sync_all();
}

// ---------------------------------------------------------------------------
// Settings that will not load
// ---------------------------------------------------------------------------

/// Which layer of the settings refused, because the two are not the same loss
/// and must not produce the same sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TabaqatIdadat {
    /// The saved file itself. This launch runs on the product defaults, and the
    /// file is copied aside untouched before anything can overwrite it.
    Malaf,
    /// The `TAARIB_*` overlay applied over the file. Whatever was underneath the
    /// overlay is intact and is what this launch runs on; only the overlay is
    /// dropped.
    Beea {
        /// Whether there was a saved file underneath it at all. A first run has
        /// none, and telling that user their file survived would be describing
        /// a file that does not exist.
        maa_malaf: bool,
    },
}

impl TabaqatIdadat {
    /// The value the log carries, so which layer this launch blamed is a fact a
    /// maintainer can grep for rather than infer from the wording.
    const fn wasm(self) -> &'static str {
        match self {
            Self::Malaf => "malaf",
            Self::Beea { maa_malaf: true } => "beea",
            Self::Beea { maa_malaf: false } => "beea_bila_malaf",
        }
    }
}

/// What loading the settings cost this launch.
#[derive(Debug)]
struct TaadhurIdadat {
    /// The failure, whole: code, both sentences, remedy.
    khata: Khata,
    /// Which layer refused.
    tabaqa: TabaqatIdadat,
    /// Where the unreadable file was preserved, or why it could not be. `None`
    /// when there was nothing to preserve.
    nuskha: Option<Result<PathBuf, String>>,
}

/// Opens the settings store, and never fails.
///
/// [`MakhzanIdadat::iftah`] refuses a settings file it cannot read, and it is
/// right to: replacing somebody's configuration with defaults in silence is how
/// people lose their provider setup and their scan folders. But `main` used to
/// answer that refusal with `?`, thirty-nine lines before the database did the
/// same, and in a `windows_subsystem = "windows"` binary that is a process which
/// ends before the log exists and before any window does. A zero-byte
/// `idadat.json` — one interrupted write, one full disk — made the application
/// unlaunchable with nothing written anywhere and nothing shown.
///
/// So the refusal is kept and its consequence is not, on the same terms
/// `bidaya::jahhiz` already sets for an unwritable directory: degrade, name it,
/// and reach the window where the diagnostics screen can explain. Nothing is
/// silently replaced — [`ihfaz_idadat`] copies the file aside first, and
/// [`aalin_taadhur_idadat`] tells the user to their face.
///
/// The two layers fail independently and are not the same accident. A mistyped
/// `TAARIB_*` name must not cost the user settings that are sitting on disk
/// perfectly readable, so the file is re-read on its own; when it reads clean,
/// the overlay is what broke, the file's own values are what this launch runs
/// on, and nothing is copied aside because nothing is damaged.
fn iftah_idadat(masarat: &Masarat) -> (Arc<MakhzanIdadat>, Option<TaadhurIdadat>) {
    let khata = match MakhzanIdadat::iftah(masarat) {
        Ok(makhzan) => return (Arc::new(makhzan), None),
        Err(khata) => khata,
    };

    let masar = masarat.malaf_idadat();
    let (tabaqa, idadat) = if masar.exists() {
        match taarib_usus::mukhattat::iqra_malaf::<Idadat>(&masar) {
            Ok(min_malaf) => (TabaqatIdadat::Beea { maa_malaf: true }, min_malaf),
            Err(_) => (TabaqatIdadat::Malaf, Idadat::default()),
        }
    } else {
        // No file at all, so `iftah` started from the defaults and the overlay
        // is the only layer that can have refused.
        (TabaqatIdadat::Beea { maa_malaf: false }, Idadat::default())
    };

    let nuskha = matches!(tabaqa, TabaqatIdadat::Malaf).then(|| ihfaz_idadat(&masar));
    let makhzan = MakhzanIdadat::min_qeema(masar, idadat);
    (
        Arc::new(makhzan),
        Some(TaadhurIdadat {
            khata,
            tabaqa,
            nuskha,
        }),
    )
}

/// Copies an unreadable settings file beside itself before defaults take over.
///
/// The store this launch runs on writes to the same path the moment the user
/// changes any setting, so without this the file that would not parse — the only
/// evidence of what they had configured, and the only thing a maintainer can
/// debug — is gone on the first click. Copied, never moved: the launch that
/// preserves the file must not also be the launch that removes it.
fn ihfaz_idadat(masar: &Path) -> Result<PathBuf, String> {
    let hadaf = masar.with_extension("json.talif");
    match std::fs::copy(masar, &hadaf) {
        Ok(_) => Ok(hadaf),
        Err(sabab) => Err(sabab.to_string()),
    }
}

/// Logs the settings failure and tells the user their preferences did not
/// survive this launch.
///
/// A dialog and not the diagnostics screen alone, because the visible symptom is
/// that everything they chose — language, theme, scan folders, providers — is
/// back at the product default, and a person who is not told will conclude the
/// application reset itself and that anything else it holds may be gone too.
fn aalin_taadhur_idadat(taadhur: &TaadhurIdadat, masar: &Path) {
    tracing::error!(
        tabaqa = taadhur.tabaqa.wasm(),
        masar = %masar.display(),
        nuskha = ?taadhur.nuskha,
        khata = %taadhur.khata.li_sijill(),
        "the settings did not load; this launch is degraded rather than aborted"
    );

    let (arabi_athar, injilizi_athar) = match taadhur.tabaqa {
        TabaqatIdadat::Malaf => (
            "يعمل تعريب في هذه الجلسة بالإعدادات الافتراضية. ملفك لم يُحذف ولم يُعدَّل.",
            "Taarib is running on the product defaults for this session. Your file has been \
             neither deleted nor changed.",
        ),
        TabaqatIdadat::Beea { maa_malaf: true } => (
            "يعمل تعريب بالإعدادات المحفوظة في ملفك، وأُهملت متغيّرات البيئة ‏TAARIB_*‏ في هذه \
             الجلسة وحدها.",
            "Taarib is running on the settings saved in your file; the TAARIB_* environment \
             overrides were dropped for this session only.",
        ),
        TabaqatIdadat::Beea { maa_malaf: false } => (
            "لا يوجد ملف إعدادات محفوظ بعد، فيعمل تعريب بالإعدادات الافتراضية، وأُهملت متغيّرات \
             البيئة ‏TAARIB_*‏ في هذه الجلسة وحدها.",
            "There is no saved settings file yet, so Taarib is running on the product defaults; \
             the TAARIB_* environment overrides were dropped for this session only.",
        ),
    };
    let (arabi_nuskha, injilizi_nuskha) = match &taadhur.nuskha {
        None => (String::new(), String::new()),
        Some(Ok(hadaf)) => (
            format!("\nحُفظت نسخة من الملف كما هو في {}.", hadaf.display()),
            format!(
                "\nA copy of the file as it stands was saved to {}.",
                hadaf.display()
            ),
        ),
        Some(Err(sabab)) => (
            format!("\nتعذّر حفظ نسخة احتياطية من الملف: {sabab}"),
            format!("\nA backup copy of the file could not be saved: {sabab}"),
        ),
    };

    let matn = format!(
        "تعذّرت قراءة إعدادات تعريب من {}.\n\n{}\n\n{arabi_athar}{arabi_nuskha}\n\n\
         Taarib could not load its settings from {}.\n\n{}\n\n{injilizi_athar}{injilizi_nuskha}\
         \n\n[{}]",
        masar.display(),
        taadhur.khata.arabi,
        masar.display(),
        taadhur.khata.injilizi,
        taadhur.khata.ramz
    );
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title(UNWAN_HIWAR)
        .set_description(&matn)
        .set_buttons(MessageButtons::Ok)
        .show();
}

// ---------------------------------------------------------------------------
// A `.ruqaa` the operating system handed over
// ---------------------------------------------------------------------------

/// The first patch file in a launch's arguments.
///
/// Only an existing regular file carrying the registered extension qualifies.
/// Explorer, the Finder and every desktop launcher hand over one absolute path;
/// anything else on the command line belongs to a flag (`--istiada`, and
/// whatever the webview adds to its own process), and a flag is not a file.
/// The first argument is skipped because it is this executable.
fn masar_ruqaa<I: IntoIterator<Item = PathBuf>>(hujaj: I) -> Option<PathBuf> {
    hujaj.into_iter().skip(1).find(|masar| {
        masar
            .extension()
            .is_some_and(|lahiqa| lahiqa.eq_ignore_ascii_case(LAHIQAT_RUQAA))
            && masar.is_file()
    })
}

/// Hands a patch file off to [`wajjih_ruqaa`] on a thread of its own.
///
/// Off the thread that paints the window, and off the one the single-instance
/// plugin calls back on: reading the package, judging it against the library and
/// running the installer all block, and the confirmation in the middle blocks
/// until a person answers it. Doing any of that inline would freeze the window
/// the user is being asked about.
fn istaqbil_ruqaa(tatbiq: &tauri::AppHandle, masar: PathBuf) {
    tracing::info!(masar = %masar.display(), "a patch file was handed to this launch");
    let tatbiq = tatbiq.clone();
    drop(std::thread::spawn(move || wajjih_ruqaa(&tatbiq, &masar)));
}

/// Serializes [`wajjih_ruqaa`], which is the only thing in this process that
/// can be entered twice at once from outside it.
static QUFL_FATH_RUQAA: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

/// What the package says about itself, as much of it as routing needs.
#[derive(Debug)]
struct TalabRuqaa {
    /// The patch's title, for the confirmation.
    unwan: String,
    /// The game's title as the contributor declared it — the fallback when the
    /// build binding cannot pick a game on its own.
    ism_luba: String,
    /// The builds and fingerprints this patch declares itself compatible with.
    irtibat: taarib_tarqee::irtibat::IrtibatBina,
}

/// Takes a patch file the operating system handed over all the way to the
/// install command, or says why it could not.
///
/// The route is: read the package, decide which game in this library it is for,
/// ask, install. The install is [`tathbeet_awamir::thabbit_ruqaa`] itself — the
/// same command the game screen's button calls, with the same quarantine, the
/// same revocation list, the same anti-cheat gate and the same post-write
/// verification — because a second installer reachable from a double-click is a
/// second set of gates to keep in step, and it would be the set nobody looks at.
///
/// Both acknowledgements are passed as `false`, and that is the point rather
/// than an omission: they are the user's answer to a specific question about a
/// specific risk, and a file manager did not ask them anything. A package that
/// needs one is refused here by name, and the refusal says to install it from
/// the game's screen, which is where the question can actually be put.
fn wajjih_ruqaa(tatbiq: &tauri::AppHandle, masar: &Path) {
    // One patch at a time. Two files selected together, or two double-clicks in
    // a row, arrive as two of these on two threads, and the library screen
    // already states why installs are run one after another: two of them writing
    // one store race. The second waits behind the first's confirmation rather
    // than being dropped, because a file the user opened deserves an answer.
    let _haris = QUFL_FATH_RUQAA.lock();

    let talab = match iqra_talab(masar) {
        Ok(talab) => talab,
        Err(khata) => return aarid_khata_ruqaa(&khata),
    };

    // `state` rather than `try_state`: all four values are handed to Tauri
    // before `setup` runs, and both callers of this function are reachable only
    // after that. A `None` here would mean the builder had been rewritten to
    // manage them conditionally, which is not a case to degrade around.
    let qaida = tauri::Manager::state::<Makhzan>(tatbiq);
    let luba = match ijlib_hadaf(&qaida, &talab) {
        Ok(luba) => luba,
        Err(khata) => return aarid_khata_ruqaa(&khata),
    };

    let Some(nafidha) = tauri::Manager::get_webview_window(tatbiq, idadat_awamir::NAFIDHA_RAISIYA)
    else {
        tracing::warn!("the patch arrived with no window to install it from");
        return;
    };
    // `thabbit_ruqaa` reports its stages on a `Window`; `Manager::get_window`,
    // which would answer with one directly, is behind tauri's `unstable`
    // feature. This is the same window arriving through the type its webview
    // already exposes, not a second handle to it.
    let nafidha = AsRef::<tauri::Webview<tauri::Wry>>::as_ref(&nafidha).window();

    if !istaadhin_tathbeet(&talab, &luba, masar) {
        tracing::info!(
            ruqaa = %masar.display(),
            luba = %luba.ism,
            "the patch was not installed: the confirmation was declined or could not be shown"
        );
        return;
    }

    // Blocked on deliberately: this is the file-association handler, which runs
    // outside any command and has no window to keep responsive yet.
    let mahsula = tauri::async_runtime::block_on(tathbeet_awamir::thabbit_ruqaa(
        nafidha,
        luba.id.to_string(),
        masar.to_string_lossy().into_owned(),
        false,
        false,
        tauri::Manager::state::<Masarat>(tatbiq),
        tauri::Manager::state::<Makhzan>(tatbiq),
        tauri::Manager::state::<Arc<MakhzanIdadat>>(tatbiq),
    ));
    match mahsula {
        Ok(natija) => {
            tracing::info!(
                luba = %luba.ism,
                adad_muhtawa = natija.adad_muhtawa,
                tahaqquq_salim = natija.tahaqquq_salim,
                "a patch handed over by the operating system was installed"
            );
            let _ = MessageDialog::new()
                .set_level(MessageLevel::Info)
                .set_title(UNWAN_HIWAR)
                .set_description(format!(
                    "ثُبِّتت الرقعة «{}» على «{}».\n{}\n{}\n\nحدِّث المكتبة في تعريب لترى \
                     الحالة الجديدة.\n\nInstalled the patch \"{}\" into \"{}\". Rescan the \
                     library in Taarib to see its new state.",
                    talab.unwan,
                    luba.ism,
                    natija.tawafuq_arabi,
                    natija.tahaqquq_arabi,
                    talab.unwan,
                    luba.ism
                ))
                .set_buttons(MessageButtons::Ok)
                .show();
        },
        Err(khata) => aarid_khata_ruqaa(&khata),
    }
}

/// Reads the package's manifest, which is the only thing that knows what the
/// file the user double-clicked actually is.
///
/// # Errors
///
/// [`KhataStudio::RuqaaLaTuqra`] for a file that is not a Taarib package, is
/// damaged, or carries a manifest without the two fields routing depends on.
fn iqra_talab(masar: &Path) -> Result<TalabRuqaa, Khata> {
    let malaf = taarib_ruqaa::qari::MalafRuqaa::iftah(masar)
        .map_err(|q| khata_ruqaa(masar, &q.to_string()))?;
    let bayan = malaf
        .ruqaa()
        .and_then(|ruqaa| ruqaa.bayan_json())
        .map_err(|q| khata_ruqaa(masar, &q.to_string()))?;

    let irtibat = bayan
        .get("irtibat")
        .cloned()
        .and_then(|qeema| serde_json::from_value(qeema).ok())
        .ok_or_else(|| khata_ruqaa(masar, "its manifest carries no readable build binding"))?;
    let nass = |haql: &str| {
        bayan
            .get("wasf")
            .and_then(|wasf| wasf.get(haql))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let unwan = {
        let muallan = nass("unwan");
        // A package with a blank title still has to be nameable in the
        // confirmation, and the file name is what the user just clicked on.
        if muallan.trim().is_empty() {
            masar
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        } else {
            muallan
        }
    };
    Ok(TalabRuqaa {
        unwan,
        ism_luba: nass("ism_luba"),
        irtibat,
    })
}

/// One [`KhataStudio::RuqaaLaTuqra`], from wherever the reading gave out.
fn khata_ruqaa(masar: &Path, tafsil: &str) -> Khata {
    Khata::from(KhataStudio::RuqaaLaTuqra {
        masar: masar.to_string_lossy().into_owned(),
        tafsil: tafsil.to_owned(),
    })
}

/// Which game in this library the package is for.
///
/// A package does not name a game; it names *builds*. `irtibat` carries the
/// launcher build identifiers and the content fingerprints it was verified
/// against, and [`taarib_tarqee::irtibat::IrtibatBina::ihkum`] is what judges
/// those against a build somebody actually has — the same judgement the install
/// pipeline makes a moment later, so a file that routes here is a file that
/// installs. Every game whose recorded build the package would accept is a
/// candidate, and exactly one candidate is an answer.
///
/// The declared game title is the fallback, and only when the binding leaves
/// zero or several candidates. It is weaker evidence and it is used as such: a
/// library where no build has been fingerprinted yet — a fresh install, before
/// any engine probe — would otherwise have no route at all, and matching the
/// title the contributor wrote against the title the launcher reports is what a
/// person would do with the same two facts. Case is folded on the ASCII range
/// only, which is where the difference between two spellings of a Latin title
/// lives; nothing else about the two strings is guessed at.
///
/// Hidden games are searched alongside visible ones: hiding a game from the grid
/// was never a statement about installing patches into it.
///
/// # Errors
///
/// Whatever the store raises listing the library, and
/// [`KhataStudio::LubaGhayrMuhaddada`] when the two passes together leave
/// anything other than exactly one game.
fn ijlib_hadaf(qaida: &Makhzan, talab: &TalabRuqaa) -> Result<Luba, Khata> {
    use taarib_makhzan::sijillat::TalabMaktaba;

    let alaab = qaida.bil_qira(|ittisal| {
        let sijill = SijillAlaab::jadeed(ittisal);
        let mut kull = sijill.qaima(&TalabMaktaba::default())?;
        let makhfiya = sijill.qaima(&TalabMaktaba {
            mukhfiya: true,
            ..TalabMaktaba::default()
        })?;
        kull.extend(makhfiya);
        Ok(kull)
    })?;

    let mut murashshaha: Vec<&Luba> = alaab
        .iter()
        .filter(|luba| {
            luba.bina
                .as_ref()
                .is_some_and(|bina| talab.irtibat.ihkum(bina).qabila_lil_tathbeet())
        })
        .collect();

    let hadaf = talab.ism_luba.trim();
    if murashshaha.len() != 1 && !hadaf.is_empty() {
        let bil_ism: Vec<&Luba> = alaab
            .iter()
            .filter(|luba| luba.ism.trim().eq_ignore_ascii_case(hadaf))
            .collect();
        if bil_ism.len() == 1 {
            murashshaha = bil_ism;
        }
    }

    match murashshaha.as_slice() {
        [wahida] => Ok((*wahida).clone()),
        akhar => Err(Khata::from(KhataStudio::LubaGhayrMuhaddada {
            ism_luba: if hadaf.is_empty() {
                talab.unwan.clone()
            } else {
                hadaf.to_owned()
            },
            adad: akhar.len(),
        })),
    }
}

/// Asks whether to install, and treats every answer that is not an explicit yes
/// as a no.
///
/// `rfd`'s synchronous dialog has no error channel: a machine that cannot draw
/// one reports [`MessageDialogResult::Cancel`], which a two-button question can
/// never produce from a person. Folding that into "no" is the only safe reading
/// — writing into somebody's game files on a question nobody answered is exactly
/// what a file association must never do — and it needs no display probe to get
/// right, because the unanswerable case and the declined case deserve the same
/// outcome.
fn istaadhin_tathbeet(talab: &TalabRuqaa, luba: &Luba, masar: &Path) -> bool {
    let jawab = MessageDialog::new()
        .set_level(MessageLevel::Warning)
        .set_title(UNWAN_HIWAR)
        .set_description(format!(
            "أتريد تثبيت الرقعة «{}» على لعبة «{}»؟\n\nالملف: {}\nمجلد اللعبة: {}\n\nستُحفظ \
             نسخة من الملفات الأصلية، ويمكن التراجع عن التثبيت من شاشة اللعبة.\n\n\
             Install the patch \"{}\" into \"{}\"?\n\nThe original files are backed up first, \
             and the install can be undone from the game's screen in Taarib.",
            talab.unwan,
            luba.ism,
            masar.display(),
            luba.jidhr.display(),
            talab.unwan,
            luba.ism
        ))
        .set_buttons(MessageButtons::YesNo)
        .show();
    jawab == MessageDialogResult::Yes
}

/// Shows a refusal from the patch route, in both languages, and logs it.
///
/// The route has no interface to put an error block into: it was reached from a
/// file manager, and the window behind it is showing whatever the user last left
/// on screen. So the failure gets the same two sentences and the same code the
/// interface would have drawn, in the one surface that exists here.
fn aarid_khata_ruqaa(khata: &Khata) {
    tracing::warn!(khata = %khata.li_sijill(), "a patch handed over by the operating system \
                                               was not installed");
    let _ = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(UNWAN_HIWAR)
        .set_description(format!(
            "{}\n\n{}\n\n[{}]",
            khata.arabi, khata.injilizi, khata.ramz
        ))
        .set_buttons(MessageButtons::Ok)
        .show();
}

/// Failures that belong to the application shell itself, rather than to any of
/// the layers underneath it.
#[derive(Debug, thiserror::Error)]
enum KhataStudio {
    /// The TypeScript bindings could not be written during a debug build.
    #[error("cannot write the generated bindings to {masar}: {tafsil}")]
    TasdirRubut {
        /// The path that was being written, relative to the crate directory.
        masar: String,
        /// What the exporter reported.
        tafsil: String,
    },

    /// The window could not be created, or the event loop stopped abnormally.
    #[error("the application window could not run: {tafsil}")]
    TaadhurTashghil {
        /// What Tauri reported.
        tafsil: String,
    },

    /// The platform file manager would not open a folder the user asked for.
    #[error("the file manager would not start: {tafsil}")]
    FathMujalladFashil {
        /// What the operating system said.
        tafsil: String,
    },

    /// A scan folder was given as something other than an absolute path.
    #[error("{masar} is not an absolute path")]
    MujalladGhayrMutlaq {
        /// What was sent.
        masar: String,
    },

    /// A file the operating system handed over is not a patch this build can
    /// read.
    #[error("{masar} is not a readable Taarib patch: {tafsil}")]
    RuqaaLaTuqra {
        /// The file, as the launch named it.
        masar: String,
        /// What the reader said.
        tafsil: String,
    },

    /// The patch does not resolve to exactly one game in this library.
    #[error("{ism_luba} resolves to {adad} game(s) in the library")]
    LubaGhayrMuhaddada {
        /// The game title the package declares, or the patch title when it
        /// declares none.
        ism_luba: String,
        /// How many games the two passes left standing — never one.
        adad: usize,
    },
}

impl Tafsir for KhataStudio {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::TasdirRubut { .. } => 0,
                    Self::TaadhurTashghil { .. } => 1,
                    Self::FathMujalladFashil { .. } => 2,
                    Self::MujalladGhayrMutlaq { .. } => 3,
                    Self::RuqaaLaTuqra { .. } => 4,
                    Self::LubaGhayrMuhaddada { .. } => 5,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Nothing opened, so nothing the user asked for can happen at all.
            Self::TaadhurTashghil { .. } => Khutura::Fadih,
            // A file that will not open as a package is the whole of what the
            // user asked for by double-clicking it.
            Self::RuqaaLaTuqra { .. } => Khutura::Khatar,
            // A developer-only step, a folder that did not open, a path that was
            // not added, a patch that could not be routed from a file manager:
            // the application still runs, and every one of these has another
            // route to the same result that still works.
            Self::TasdirRubut { .. }
            | Self::FathMujalladFashil { .. }
            | Self::MujalladGhayrMutlaq { .. }
            | Self::LubaGhayrMuhaddada { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::TasdirRubut { masar, .. } => format!(
                "تعذّرت كتابة ملفات الربط المولَّدة إلى {masar}. تأكّد أن المجلد قابل للكتابة، \
                 ثم أعِد تشغيل البناء التصحيحي."
            ),
            Self::TaadhurTashghil { .. } => {
                "تعذّر فتح نافذة تعريب على هذا الجهاز. راجع سجلّ التشخيص لمعرفة ما رفضه \
                 نظام العرض."
                    .to_owned()
            },
            Self::FathMujalladFashil { .. } => {
                "تعذّر فتح المجلد في مدير ملفات النظام. افتح مجلد اللعبة يدويًا من مساره \
                 الظاهر في الشاشة."
                    .to_owned()
            },
            Self::MujalladGhayrMutlaq { masar } => format!(
                "المسار «{masar}» ليس مسارًا مطلقًا، ولم يُضف إلى مجلدات الفحص. اكتب المسار \
                 كاملًا من جذر القرص."
            ),
            Self::RuqaaLaTuqra { masar, tafsil } => format!(
                "تعذّرت قراءة الملف {masar} كرقعة تعريب: {tafsil}. الملف إمّا تالف أو ليس رقعة \
                 تعريب أصلًا. أعد تنزيله من مصدره ثم افتحه مرة أخرى."
            ),
            Self::LubaGhayrMuhaddada { ism_luba, adad } => {
                let matches = if *adad == 0 {
                    "ولم تطابقها أي لعبة في مكتبتك".to_owned()
                } else {
                    format!("وطابقتها {adad} من ألعاب مكتبتك")
                };
                format!(
                    "لم يستطع تعريب تحديد اللعبة التي تخصّها هذه الرقعة («{ism_luba}») {matches}. \
                     افتح تعريب، وحدِّث المكتبة حتى تُعرَف نسخة كل لعبة، ثم ثبّت الرقعة من شاشة \
                     اللعبة نفسها."
                )
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::TasdirRubut { masar, .. } => format!(
                "Could not write the generated bindings to {masar}. Make sure the folder is \
                 writable, then run the debug build again."
            ),
            Self::TaadhurTashghil { .. } => {
                "Could not open the Taarib window on this machine. The diagnostics log records \
                 what the display system refused."
                    .to_owned()
            },
            Self::FathMujalladFashil { .. } => {
                "The system file manager would not open the folder. Open the game's folder \
                 by the path shown on the screen."
                    .to_owned()
            },
            Self::MujalladGhayrMutlaq { masar } => format!(
                "{masar} is not an absolute path and was not added to the scanned folders. \
                 Give the full path from the root of the drive."
            ),
            Self::RuqaaLaTuqra { masar, tafsil } => format!(
                "Could not read {masar} as a Taarib patch: {tafsil}. The file is damaged, or it \
                 is not a Taarib patch at all. Download it again from wherever it came from and \
                 open it once more."
            ),
            Self::LubaGhayrMuhaddada { ism_luba, adad } => {
                let matches = if *adad == 0 {
                    "no game in your library".to_owned()
                } else {
                    format!("{adad} of the games in your library")
                };
                format!(
                    "Taarib could not tell which game this patch is for: it declares \
                     \"{ism_luba}\", which matched {matches}. Open Taarib, rescan the library so \
                     each game's build is known, then install the patch from that game's screen."
                )
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The library is the missing fact, and rescanning is what produces
            // it: a game with no recorded build cannot be matched by binding.
            Self::LubaGhayrMuhaddada { .. } => Khutwa::AadaFahsMaktaba,
            // Nothing in this application can repair somebody else's download.
            Self::RuqaaLaTuqra { .. } => Khutwa::LaShay,
            Self::TasdirRubut { .. }
            | Self::TaadhurTashghil { .. }
            | Self::FathMujalladFashil { .. }
            | Self::MujalladGhayrMutlaq { .. } => Khutwa::FathTashkhis,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TasdirRubut { masar, tafsil } | Self::RuqaaLaTuqra { masar, tafsil } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::TaadhurTashghil { tafsil } | Self::FathMujalladFashil { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::MujalladGhayrMutlaq { masar } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(masar.clone()));
            },
            Self::LubaGhayrMuhaddada { ism_luba, adad } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism_luba.clone()));
                let _ = siyaq.insert(
                    "adad".to_owned(),
                    QeemaSiyaq::Hajm(u64::try_from(*adad).unwrap_or(u64::MAX)),
                );
            },
        }
        siyaq
    }
}

khata_min!(KhataStudio);

#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::muharrik::{KhalfiyaBarmajiya, Muharrik};

    use super::{
        AilatMuharrik, JahiziyatTashghil, KhulasatFahs, LAHIQAT_RUQAA, Mimariya, Tabaqa,
        masar_ruqaa,
    };

    /// A directory of this test's own, so two tests never see each other's files.
    #[expect(
        clippy::disallowed_methods,
        reason = "a scratch directory under `std::env::temp_dir()`, never a data root or a game \
                  directory"
    )]
    fn mujallad(ism: &str) -> std::path::PathBuf {
        let masar = std::env::temp_dir().join(format!("taarib_fath_{ism}"));
        let _ = std::fs::remove_dir_all(&masar);
        std::fs::create_dir_all(&masar).map_or_else(|_| std::env::temp_dir(), |()| masar)
    }

    /// Writes an empty file and answers with its path.
    fn malaf(mujallad: &std::path::Path, ism: &str) -> std::path::PathBuf {
        let masar = mujallad.join(ism);
        let _ = std::fs::write(&masar, b"");
        masar
    }

    #[test]
    fn taakhudh_awwal_ruqaa_wa_tatakhatta_al_tanfidhi() {
        let mujallad = mujallad("awwal");
        let tanfidhi = malaf(&mujallad, "taarib-studio");
        let ula = malaf(&mujallad, "ula.ruqaa");
        let thaniya = malaf(&mujallad, "thaniya.ruqaa");
        assert_eq!(masar_ruqaa([tanfidhi, ula.clone(), thaniya]), Some(ula));
    }

    /// The executable itself is skipped by position, not by name: an installer
    /// that names the binary `taarib.ruqaa` would otherwise hand it to itself.
    #[test]
    fn tatakhatta_al_wasita_al_ula_hatta_law_kanat_ruqaa() {
        let mujallad = mujallad("ula_ruqaa");
        let tanfidhi = malaf(&mujallad, "taarib-studio.ruqaa");
        assert_eq!(masar_ruqaa([tanfidhi]), None);
    }

    /// The extension is matched case-insensitively, because Windows hands the
    /// path over in whatever case the file carries.
    #[test]
    fn al_lahiqa_ghayr_hassasa_lil_halat() {
        let mujallad = mujallad("halat");
        let tanfidhi = malaf(&mujallad, "taarib-studio");
        let kabira = malaf(&mujallad, "MITHAL.RUQAA");
        assert_eq!(masar_ruqaa([tanfidhi, kabira.clone()]), Some(kabira));
    }

    /// `--istiada` and every other flag are arguments, not paths.
    #[test]
    fn al_aalam_laysat_masarat() {
        let mujallad = mujallad("aalam");
        let tanfidhi = malaf(&mujallad, "taarib-studio");
        assert_eq!(
            masar_ruqaa([tanfidhi, std::path::PathBuf::from("--istiada")]),
            None
        );
    }

    /// A path with the right extension that names nothing on disk is not a file
    /// the operating system opened; it is a typed argument.
    #[test]
    fn al_masar_ghayr_al_mawjud_marfud() {
        let mujallad = mujallad("ghayr_mawjud");
        let tanfidhi = malaf(&mujallad, "taarib-studio");
        let wahmi = mujallad.join(format!("la_shay.{LAHIQAT_RUQAA}"));
        assert_eq!(masar_ruqaa([tanfidhi, wahmi]), None);
    }

    /// A directory named `something.ruqaa` is not a package either.
    #[test]
    fn al_mujallad_dhu_al_lahiqa_marfud() {
        let mujallad = mujallad("mujallad_ruqaa");
        let tanfidhi = malaf(&mujallad, "taarib-studio");
        let mutanakkir = mujallad.join("mutanakkir.ruqaa");
        let _ = std::fs::create_dir_all(&mutanakkir);
        assert_eq!(masar_ruqaa([tanfidhi, mutanakkir]), None);
    }

    /// A game nothing has examined is told apart from one the probe examined
    /// and did not recognise, though both carry the same four values.
    ///
    /// This is the whole reason `mafhusa` exists: `Majhul` and
    /// `TarjamaFawqiya` are real answers as well as stand-ins, so the row is
    /// byte-identical in the two cases apart from this flag, and "we looked and
    /// found nothing" is a different sentence from "nothing has been looked at".
    #[test]
    fn khulasat_alfahs_tumayyiz_ghayr_almafhus_min_ghayr_almaruf() {
        let ghayr_mafhusa = KhulasatFahs::min_taqreer(None);
        assert!(!ghayr_mafhusa.mafhusa);
        assert_eq!(ghayr_mafhusa.aila, AilatMuharrik::Majhul);
        assert_eq!(ghayr_mafhusa.tabaqa, Tabaqa::TarjamaFawqiya);
        assert!(!ghayr_mafhusa.marfuda);
        assert_eq!(ghayr_mafhusa.jahiziya, JahiziyatTashghil::Ghaiba);

        // A real report, from the same function the probe writes with, for a
        // game whose engine Taarib does not recognise.
        let asli = taarib_muharrik::imkaniyat::taqreer(
            Muharrik {
                aila: AilatMuharrik::Majhul,
                isdar: None,
                khalfiya: KhalfiyaBarmajiya::Majhula,
                itarat: Vec::new(),
                rusum: Vec::new(),
                mimariya: Mimariya::X8664,
                thiqa: 20,
                dalail: Vec::new(),
            },
            &[],
            "2026-01-01T00:00:00Z".to_owned(),
        );
        let mafhusa = KhulasatFahs::min_taqreer(Some(&asli));

        assert!(mafhusa.mafhusa);
        assert_eq!(mafhusa.aila, ghayr_mafhusa.aila);
        assert_eq!(mafhusa.tabaqa, ghayr_mafhusa.tabaqa);
        assert_eq!(
            KhulasatFahs {
                mafhusa: false,
                ..mafhusa
            },
            ghayr_mafhusa,
            "the two differ in nothing but the flag, which is why the flag is sent"
        );
    }
}
