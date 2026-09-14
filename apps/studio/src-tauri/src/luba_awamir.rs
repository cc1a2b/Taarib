//! تفاصيل اللعبة — the detail screen: the probe, the build, and the protection scan.

use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

use parking_lot::Mutex;
use taarib_aman::kashf_himaya::{self, DaleelHimaya, IjmaaHimaya};
use taarib_kashf::fahs::{Matjar as _, SimatLuba};
use taarib_kashf::lugha_rasmiya::{
    Fahis, FahisMawarid, KhazinaDhakira, KhazinatLugha as _, LughatMuallana, MawridLugha,
    TalabLugha,
};
use taarib_kashf::matajir::MatjarSteam;
use taarib_makhzan::sijillat::{SijillAlaab, SijillMuharrik, SijillTathbeet, SimaMukhzana, sima};
use taarib_makhzan::wasl::{Makhzan, alaan};
use taarib_muharrik::fahs::SiyaqFahs;
use taarib_muharrik::{ISDAR_FAHS, Mifhas};
use taarib_muhawwil_unreal::mawarid::{Mawrid as _, iostore, locmeta, locres, pak};
use taarib_mustalahat::bina::BinaId;
use taarib_mustalahat::luba::{DaleelLugha, HalatLughaRasmiya, HukmLughaRasmiya, NawDaleelLugha};
use taarib_mustalahat::luba::{LawnBariz, Luba, LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::{
    AilatMuharrik, Daleel, KhalfiyaBarmajiya, NawDaleel, Tabaqa, TaqreerImkaniyat,
};
use taarib_tathbeet::bayan::{NawTathbeet, Tathbeet};
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, MasarMatlub, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::manassa::NizamTashghil;
use taarib_usus::masarat::Masarat;

/// The artwork cache's subdirectory under `makhbaa`, as `taarib-kashf` writes it.
const MUJALLAD_SUWAR: &str = "suwar";

/// The identified engine, rendered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MuharrikHie {
    /// The engine family, under its own name.
    pub aila: String,
    /// The same family as the discriminant, so the interface can tell a named
    /// engine from an unidentified one.
    ///
    /// [`Self::aila`] cannot answer that question: `AilatMuharrik::ism` renders
    /// [`AilatMuharrik::Majhul`] as the Arabic literal `غير معروف`, so an
    /// English session reads Arabic for the one answer most of this library
    /// gives, and no caller can key behaviour off the miss without matching on a
    /// display string. Sent beside the rendered name rather than instead of it
    /// because the header still draws a name for every other family, and the
    /// interface narrows this to a union it may match exhaustively.
    pub aila_ramz: AilatMuharrik,
    /// The version string exactly as it was found in the game.
    pub isdar: Option<String>,
    /// How the game's code runs.
    pub khalfiya: String,
    /// Every graphics API the probe saw.
    pub rusum: Vec<String>,
    /// Confidence in the identification, 0 to 100.
    pub thiqa: u8,
}

/// What Taarib can do to this game, rendered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerHie {
    /// Which of the three products this game gets, as the discriminant.
    ///
    /// The tier is a product decision — text replaced inside the engine, files
    /// patched directly, or a translation drawn over the top — and the interface
    /// says which of the three a game gets rather than printing a number. It
    /// cannot derive that from [`Self::tabaqa_raqm`] without keeping a second
    /// copy of this taxonomy in TypeScript, and a second copy mislabels rather
    /// than fails when the taxonomy moves.
    pub tabaqa: Tabaqa,
    /// The tier number, 1 to 3.
    pub tabaqa_raqm: u8,
    /// The tier's name in Arabic.
    pub tabaqa_arabi: String,
    /// The same name in English.
    ///
    /// [`crate::tilqai_awamir::HukmTilqaiHie`] has sent both names since it was
    /// written, and this record was the odd one out: an English session read the
    /// tier number, the reason and the systems in English and then hit
    /// `طبقة ترجمة` where the tier's name should be.
    pub tabaqa_injilizi: String,
    /// The paragraph that explains what the tier actually does, in Arabic.
    ///
    /// Sent so the interface stops carrying its own wording. The tier's meaning
    /// is decided by [`Tabaqa`] and nowhere else, and a locale file that
    /// paraphrases it is a second definition that drifts silently the first time
    /// the tier's behaviour changes.
    pub sharh_arabi: String,
    /// The same paragraph in English.
    pub sharh_injilizi: String,
    /// Why that tier and not a better one, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// The text systems Taarib will take over.
    pub anzimat: Vec<String>,
    /// Everything that will not work, named specifically, in Arabic.
    pub hudud: Vec<String>,
    /// The same list in English.
    ///
    /// Sent beside the Arabic rather than instead of it because this list was
    /// Arabic-only, which meant an English user read the tier, the reason and
    /// the systems in English and then hit a paragraph of Arabic for the one
    /// part that says what will not work.
    pub hudud_injilizi: Vec<String>,
    /// Whether the safety layer refuses this game outright.
    pub marfuda: bool,
    /// Whether the in-game half of this engine's support is finished.
    ///
    /// Distinct from [`Self::hudud`], and the distinction is the whole point: a
    /// limit is something that will still be true when everything works — text
    /// baked into an image cannot be translated by any amount of engineering.
    /// This says the runtime itself is unfinished, which is a fact about
    /// Taarib rather than about the game, and it is temporary. Conflating the
    /// two would be dishonest in one direction or the other.
    pub jahiziya: String,
    /// What specifically is missing, in Arabic, when anything is.
    pub naqs_arabi: Option<String>,
    /// The same, in English.
    pub naqs_injilizi: Option<String>,
}

/// The installed build, rendered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BinaHie {
    /// The launcher's own build identifier, where the launcher has one.
    pub manassa: Option<String>,
    /// The first bytes of the content fingerprint.
    pub basma_mukhtasara: String,
    /// How many files went into the fingerprint.
    pub adad_malaffat: u32,
    /// When the fingerprint was computed, RFC 3339.
    pub waqt: String,
    /// The short label the interface shows for this build.
    pub wasm: String,
}

/// What an anti-cheat scan found.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HimayaHie {
    /// Whether any evidence at all was found.
    pub mahmiya: bool,
    /// The distinct anti-cheats named by the evidence, in Arabic.
    pub anwa: Vec<String>,
    /// Every piece of evidence, one Arabic line each.
    pub adilla: Vec<String>,
    /// Places the scan could not read, so a thin report is told from a clean one.
    pub thughrat: Vec<String>,
    /// The walk stopped at a bound before it finished.
    pub mabtur: bool,
}

/// One observation behind an engine identification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DaleelHie {
    /// How it was observed, in Arabic.
    pub naw: String,
    /// What was observed, as the detector stated it.
    pub wasf: String,
    /// Where, relative to the game's root.
    pub mawqi: Option<String>,
    /// How much this observation is worth, 0 to 100.
    pub wazn: u8,
}

/// Everything the detail screen draws for one game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TafasilLuba {
    /// Taarib's identity for the game.
    pub muarrif: String,
    /// The name its launcher gives it, verbatim.
    pub ism: String,
    /// The installation root.
    pub jidhr: String,
    /// The executable Taarib probes and launches, once it is known.
    pub tanfidhi: Option<String>,
    /// Every launcher that reports it, under its Arabic name.
    pub manassat: Vec<String>,
    /// The resolved cover, when the artwork cascade has produced one.
    pub ghilaf: Option<String>,
    /// The cover's dominant colour, which the detail screen uses as that game's
    /// accent.
    ///
    /// Sent even though the interface is holding the cover already, because the
    /// alternative is reading the pixels back out of a canvas: that needs the
    /// image decoded before the screen can be tinted, it depends on the asset
    /// protocol continuing to answer with a permissive CORS header, and it
    /// recomputes on every visit a value the importer computed once. It is
    /// `None` for a game with no cover, which is the same condition that makes
    /// `ghilaf` `None`, and the screen falls back to the product accent.
    pub lawn: Option<LawnBariz>,
    /// The identified engine.
    pub muharrik: MuharrikHie,
    /// The capability report.
    pub taqreer: TaqreerHie,
    /// The build currently installed, when a fingerprint has been computed.
    pub bina: Option<BinaHie>,
    /// The protection scan, present only when a launcher's own metadata named
    /// an anti-cheat: a full walk of a game directory is not something the
    /// detail screen does every time it opens.
    pub himaya: Option<HimayaHie>,
    /// Whether a text patch is installed.
    pub muthabbat_nass: bool,
    /// Whether a voice patch is installed.
    pub muthabbat_sawt: bool,
}

/// Everything the detail screen draws for one game.
///
/// The capability report is served from the store when one is cached and was
/// produced by this build's probe; otherwise the probe runs here and the result
/// is persisted, so the next open is a single indexed read.
///
/// # Errors
///
/// [`KhataLuba::MuarrifGhayrSalih`] when the identity is not one Taarib issued,
/// [`KhataLuba::LubaMafquda`] when no game carries it,
/// [`KhataLuba::LubaGhayrMawjuda`] when the game's files are no longer on disk,
/// and whatever the store or the probe raise.
#[tauri::command]
#[specta::specta]
pub fn tafasil_luba(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<TafasilLuba, Khata> {
    let id = huwiya(muarrif)?;
    let hali = idadat.hali();
    let luba = ijlib_luba(&makhzan, id)?;
    let simat = simat_luba(&makhzan, id)?;

    let taqreer = taqreer_luba(&makhzan, &luba, &simat, false)?;
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;

    let himaya = if yushar_ila_himaya(&simat) {
        Some(ifhas(&masarat, &hali, &luba)?)
    } else {
        None
    };

    Ok(TafasilLuba {
        muarrif: id.to_string(),
        ism: luba.ism.clone(),
        jidhr: luba.jidhr.to_string_lossy().into_owned(),
        tanfidhi: luba
            .tanfidhi
            .as_ref()
            .map(|q| q.to_string_lossy().into_owned()),
        manassat: luba
            .masadir
            .iter()
            .map(|q| q.ism_arabi().to_owned())
            .collect(),
        // The stored value is a content-addressed cache key, not a path and
        // not a URL. Handed to the interface as-is it becomes the `src` of an
        // `<img>`, which the webview resolves against the document origin and
        // 404s — invisibly, because the generated plate is drawn underneath.
        // Resolved here rather than in the interface: `mahalli` is the one
        // sanctioned key-to-path conversion and it refuses a key that does not
        // map, which is the check that keeps `../../../etc` from becoming a
        // path at all.
        ghilaf: luba
            .suwar
            .ghilaf
            .as_deref()
            .and_then(|miftah| masar_ghilaf(&masarat, miftah)),
        lawn: luba.suwar.lawn,
        muharrik: muharrik_hie(&taqreer),
        taqreer: taqreer_hie(&taqreer),
        bina: luba.bina.as_ref().map(bina_hie),
        himaya,
        muthabbat_nass: muthabbat(&luba.jidhr, &nusakh, NawTathbeet::Nass),
        muthabbat_sawt: muthabbat(&luba.jidhr, &nusakh, NawTathbeet::Sawt),
    })
}

/// Re-probes the engine and replaces the cached capability report.
///
/// What the "probe again" button calls. The stored report is overwritten rather
/// than versioned: a superseded report describes a probe that no longer exists.
///
/// # Errors
///
/// As [`tafasil_luba`].
#[tauri::command]
#[specta::specta]
pub async fn afhas_muharrik(
    muarrif: String,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<TaqreerHie, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let simat = simat_luba(&makhzan, id)?;
    let taqreer = taqreer_luba(&makhzan, &luba, &simat, true)?;
    Ok(taqreer_hie(&taqreer))
}

/// Every observation behind the engine identification, strongest first.
///
/// The evidence the diagnostics panel shows, and the thing a maintainer reads
/// when an identification is wrong.
///
/// # Errors
///
/// As [`tafasil_luba`].
#[tauri::command]
#[specta::specta]
pub fn dalail_muharrik(
    muarrif: String,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<Vec<DaleelHie>, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let simat = simat_luba(&makhzan, id)?;
    let taqreer = taqreer_luba(&makhzan, &luba, &simat, false)?;

    let mut dalail: Vec<&Daleel> = taqreer.muharrik.dalail.iter().collect();
    dalail.sort_by_key(|daleel| Reverse(daleel.wazn));
    Ok(dalail.into_iter().map(daleel_hie).collect())
}

/// Scans the game's files for anti-cheat evidence.
///
/// Explicit rather than automatic, because it walks the game directory and
/// reads import tables: on a large installation that is seconds, and a detail
/// screen that paid it on every open would be a detail screen nobody opens.
///
/// # Errors
///
/// As [`tafasil_luba`]. The scan itself never fails — a place it cannot read
/// becomes an entry in [`HimayaHie::thughrat`], and so does a Steam catalogue
/// that was never reachable at all.
#[tauri::command]
#[specta::specta]
pub async fn fahs_himaya(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<HimayaHie, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    ifhas(&masarat, &idadat.hali(), &luba)
}

/// Hides a game from the library, or brings it back.
///
/// # Errors
///
/// [`KhataLuba::MuarrifGhayrSalih`] when the identity is not one Taarib issued,
/// and whatever the store raises.
#[tauri::command]
#[specta::specta]
pub fn ikhfa_luba(
    muarrif: String,
    mukhfiya: bool,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<bool, Khata> {
    let id = huwiya(muarrif)?;
    makhzan.bi_muamala(|muamala| SijillAlaab::jadeed(muamala).ghayyir_ikhfa(id, mukhfiya))?;
    Ok(mukhfiya)
}

// ---------------------------------------------------------------------------
// Shared by this module and the installation surface
// ---------------------------------------------------------------------------

/// Parses the identity the interface sends back.
///
/// Goes through serde rather than a `Uuid` parse so that the text accepted here
/// is exactly the text [`LubaId`] serializes to, and there is no second
/// spelling of an identity anywhere in the product.
///
/// # Errors
///
/// [`KhataLuba::MuarrifGhayrSalih`] when the text is not an identity.
pub fn huwiya(muarrif: String) -> Natija<LubaId> {
    serde_json::from_value::<LubaId>(serde_json::Value::String(muarrif.clone()))
        .map_err(|_| Khata::from(KhataLuba::MuarrifGhayrSalih { muarrif }))
}

/// One game, by identity.
///
/// # Errors
///
/// [`KhataLuba::LubaMafquda`] when no game carries the identity, and whatever
/// the store raises.
pub fn ijlib_luba(makhzan: &Makhzan, id: LubaId) -> Natija<Luba> {
    makhzan
        .bil_qira(|ittisal| SijillAlaab::jadeed(ittisal).wahida(id))?
        .ok_or_else(|| {
            Khata::from(KhataLuba::LubaMafquda {
                muarrif: id.to_string(),
            })
        })
}

/// The launcher metadata hints recorded for one game, in the probe's vocabulary.
///
/// # Errors
///
/// Whatever the store raises.
pub fn simat_luba(makhzan: &Makhzan, id: LubaId) -> Natija<Vec<SimatLuba>> {
    let mukhzana = makhzan.bil_qira(|ittisal| SijillAlaab::jadeed(ittisal).simat(id))?;
    Ok(simat_min_makhzan(&mukhzana))
}

/// The artwork cache's absolute path for one key, when the file is really there.
///
/// [`None`] for a key that does not resolve, which the interface renders as
/// "no cover" and falls back to the generated plate — the same answer it gives
/// for a game whose artwork was never fetched.
fn masar_ghilaf(masarat: &Masarat, miftah: &str) -> Option<String> {
    let jidhr = masarat.makhbaa().join(MUJALLAD_SUWAR);
    taarib_kashf::suwar::KhaziantSuwar::jadeeda(jidhr)
        .ok()?
        .mahalli(miftah)
        .map(|masar| masar.to_string_lossy().into_owned())
}

/// The backup directory this game's installations share.
///
/// Taken from the active installation's own record where there is one, so a
/// directory named at install time is the directory an uninstall opens; a game
/// with nothing installed falls back to the layout's default for its identity.
///
/// # Errors
///
/// Whatever the store raises, and [`taarib_usus::masarat`] when the recorded
/// identity cannot be a path segment.
pub fn jidhr_nusakh(masarat: &Masarat, makhzan: &Makhzan, id: LubaId) -> Natija<PathBuf> {
    let sabiq = makhzan.bil_qira(|ittisal| SijillTathbeet::jadeed(ittisal).nashit(id))?;
    let ism = sabiq.map_or_else(|| id.to_string(), |wahid| wahid.jidhr_nusakh);
    masarat.nusakh_luba(&ism)
}

/// Whether one kind of installation is currently applied to a game.
///
/// A manifest on disk is not enough: an uninstall restores every recorded path
/// and leaves the manifest behind on purpose, so that a user who asks
/// afterwards why their game is not right still has the record. What "installed"
/// means here is that the manifest exists and something in it is still
/// outstanding.
///
/// A manifest that cannot be read at all answers **true**, because a backup
/// directory nobody can interpret is a state that needs attention rather than
/// one to report as an absence.
#[must_use]
pub fn muthabbat(jidhr_luba: &Path, jidhr_nusakh: &Path, naw: NawTathbeet) -> bool {
    if !Tathbeet::mawjud(jidhr_nusakh, naw) {
        return false;
    }
    match Tathbeet::istanif(jidhr_luba, jidhr_nusakh, naw) {
        Ok(tathbeet) => !tathbeet.bayan().ustuidat_bilkamil(),
        Err(_) => true,
    }
}

/// The installed build, rendered.
#[must_use]
pub fn bina_hie(bina: &BinaId) -> BinaHie {
    BinaHie {
        manassa: bina.manassa.clone(),
        basma_mukhtasara: bina.basma.mukhtasara(),
        adad_malaffat: bina.adad_malaffat,
        waqt: bina.waqt.clone(),
        wasm: bina.wasm(),
    }
}

/// The Steam application identifier behind a game, when one of its launcher
/// identities resolves to Steam.
#[must_use]
pub fn appid_steam(luba: &Luba) -> Option<u32> {
    luba.masadir.iter().find_map(|masdar| match masdar.asl() {
        MasdarLuba::Steam(raqm) => Some(*raqm),
        _ => None,
    })
}

/// Where Steam is, resolved exactly the way the library scan resolves it.
///
/// Through [`MatjarSteam`] rather than off `Idadat::manassat`, because that
/// field is only the user's manual override and almost nobody sets one. The
/// adapter reads the Windows registry first, then the directories Steam is
/// known to install itself in, and still lets the override win when it is
/// there — so this answers on a machine where nothing was configured, which is
/// every machine but the developer's.
///
/// # Errors
///
/// Whatever [`taarib_kashf::siyaq_fahs`] raises, which is currently nothing.
pub fn jidhr_steam(masarat: &Masarat, idadat: &Idadat) -> Natija<Option<PathBuf>> {
    let siyaq = taarib_kashf::siyaq_fahs(idadat.manassat.clone(), masarat.manzil().to_path_buf())?;
    Ok(MatjarSteam.mawqi(&siyaq))
}

/// The Steam root the safety layer will be handed for one game, refusing when
/// the game came from Steam and no root could be found.
///
/// The two scans in `taarib-aman` read `appcache/appinfo.vdf` only when they are
/// given both an application id and a root, and they say nothing at all when
/// either is missing. For a game that is not on Steam that silence is correct —
/// there is no catalogue to consult. For a Steam game it is not: VAC is
/// declared in the catalogue and leaves nothing whatever in the game folder, so
/// a scan run without the root reports the same clean verdict for a
/// VAC-secured game as for a game that really is clean, and the install
/// proceeds. Refusing is the only answer that does not turn "the check could
/// not run" into "the check passed", and the user has a way out: the manual
/// override, which is what [`Khutwa::FathIdadat`] sends them to.
///
/// # Errors
///
/// [`KhataLuba::JidhrSteamMajhul`] for that case, and whatever
/// [`jidhr_steam`] raises.
pub fn jidhr_steam_lil_fahs(
    masarat: &Masarat,
    idadat: &Idadat,
    luba: &Luba,
) -> Natija<Option<PathBuf>> {
    hasm_jidhr_steam(jidhr_steam(masarat, idadat)?, appid_steam(luba), &luba.ism)
}

/// The refusal rule on its own, apart from the lookup that feeds it.
///
/// Separated because the two are different kinds of answer: where Steam lives
/// is a fact about the machine, and what a missing Steam means for a given game
/// is a policy the three install paths share. Keeping them apart is what lets
/// the policy be stated once and checked without a Steam installation.
///
/// # Errors
///
/// [`KhataLuba::JidhrSteamMajhul`] when there is an application id and no root.
fn hasm_jidhr_steam(
    jidhr: Option<PathBuf>,
    appid: Option<u32>,
    ism: &str,
) -> Natija<Option<PathBuf>> {
    if jidhr.is_none() && appid.is_some() {
        return Err(Khata::from(KhataLuba::JidhrSteamMajhul {
            ism: ism.to_owned(),
        }));
    }
    Ok(jidhr)
}

// ---------------------------------------------------------------------------
// Probing and persistence
// ---------------------------------------------------------------------------

/// The capability report for one game, cached or freshly probed.
///
/// A cached report is served only when the probe that produced it is this
/// build's; an older stamp re-probes, which is the whole reason the stamp is
/// written. `ijbar` forces a probe regardless.
pub(crate) fn taqreer_luba(
    makhzan: &Makhzan,
    luba: &Luba,
    simat: &[SimatLuba],
    ijbar: bool,
) -> Natija<TaqreerImkaniyat> {
    if !ijbar {
        let mukhazzan =
            makhzan.bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(luba.id))?;
        if let Some(taqreer) = mukhazzan
            && taqreer.isdar_fahs >= ISDAR_FAHS
        {
            return Ok(taqreer);
        }
    }

    if !luba.mawjuda || !luba.jidhr.exists() {
        return Err(Khata::from(KhataLuba::LubaGhayrMawjuda {
            ism: luba.ism.clone(),
            jidhr: luba.jidhr.clone(),
        }));
    }

    let waqt = makhzan.bil_qira(alaan)?;
    let siyaq = SiyaqFahs {
        jidhr: &luba.jidhr,
        tanfidhi: luba.tanfidhi.as_deref(),
        ism: &luba.ism,
        nizam: NizamTashghil::hali(),
        beea: &luba.beea,
    };
    let taqreer = Mifhas::jadeed().taqreer(&siyaq, simat, waqt)?;

    let basma = luba.bina.as_ref().map(|bina| bina.basma);
    makhzan.bi_muamala(|muamala| {
        SijillMuharrik::jadeed(muamala).sajjil(luba.id, &taqreer, basma.as_ref())
    })?;

    tracing::info!(
        luba = %luba.id,
        aila = ?taqreer.muharrik.aila,
        thiqa = taqreer.muharrik.thiqa,
        tabaqa = taqreer.tabaqa.raqm(),
        "capability report written"
    );
    Ok(taqreer)
}

/// Runs the anti-cheat scan and renders it, reading Steam's catalogue whenever
/// the game is a Steam game and Steam is anywhere this machine can be told
/// about.
///
/// This is the reading surface, so it reports rather than refuses: a banner
/// that showed nothing would be the same banner a clean game gets, and the
/// difference between the two is exactly what
/// [`jidhr_steam_lil_fahs`] refuses over on the install path.
///
/// # Errors
///
/// Whatever [`jidhr_steam`] raises. The scan itself never fails.
fn ifhas(masarat: &Masarat, idadat: &Idadat, luba: &Luba) -> Natija<HimayaHie> {
    let appid = appid_steam(luba);
    let jidhr = jidhr_steam(masarat, idadat)?;
    let ijmaa = kashf_himaya::ifhas_himaya(&luba.jidhr, appid, jidhr.as_deref());
    Ok(himaya_hie(&ijmaa, appid.is_some() && jidhr.is_none()))
}

/// Whether a launcher's own metadata already named an anti-cheat for this game.
fn yushar_ila_himaya(simat: &[SimatLuba]) -> bool {
    simat
        .iter()
        .any(|sima| matches!(sima, SimatLuba::HimayaMuhtamala(_) | SimatLuba::MuammanaVac))
}

/// Whether the launcher's own catalogue already says this game is played with
/// other people, so a screen can ask before the button rather than after.
///
/// Both hints count, not only the online one, because the gate this announces is
/// [`taarib_aman::kashf_shabaka::mutaaddid`], and that answers on *any* evidence
/// — a shared-screen title is refused by it exactly as an online one is. Warning
/// on the narrower set would leave the wider refusal unannounced.
///
/// This is a hint, never a verdict: it reads the stored library scan, while the
/// gate walks the game directory at install time. A `false` here means "the
/// launcher did not say so", never "you will not be asked", and
/// [`crate::tathbeet_awamir::thabbit_ruqaa`] still refuses at the door when the
/// walk disagrees.
///
/// `tilqai_awamir` asks the same question of the same rows for the automatic
/// path and currently carries its own copy of this predicate. The two must stay
/// in step; this is the copy the manual path uses, and the one to keep when they
/// are folded together.
pub(crate) fn yalzam_iqrar_shabaka(simat: &[SimatLuba]) -> bool {
    simat
        .iter()
        .any(|sima| matches!(sima, SimatLuba::JamaiOnline | SimatLuba::JamaiMahalli))
}

/// Rebuilds the probe's metadata hints from the rows the store holds.
///
/// Only the six discriminators the schema defines are answered. A hint the
/// store does not carry is absent rather than invented: a probe told a game is
/// multiplayer when nothing said so would attach a warning nobody earned.
fn simat_min_makhzan(mukhzana: &[SimaMukhzana]) -> Vec<SimatLuba> {
    mukhzana
        .iter()
        .filter_map(|wahida| match wahida.naw.as_str() {
            sima::JAMAI_MAHALLI => Some(SimatLuba::JamaiMahalli),
            sima::JAMAI_ONLINE => Some(SimatLuba::JamaiOnline),
            sima::HIMAYA_MUHTAMALA => Some(SimatLuba::HimayaMuhtamala(wahida.qeema.clone())),
            sima::MUAMMANA_VAC => Some(SimatLuba::MuammanaVac),
            sima::LAYSAT_LUBA => Some(SimatLuba::LaysatLuba(wahida.qeema.clone())),
            sima::TABAQAT_TAWAFUQ => Some(SimatLuba::TabaqatTawafuq(wahida.qeema.clone())),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// The engine as the header writes it.
fn muharrik_hie(taqreer: &TaqreerImkaniyat) -> MuharrikHie {
    let muharrik = &taqreer.muharrik;
    MuharrikHie {
        aila: muharrik.aila.ism().to_owned(),
        aila_ramz: muharrik.aila,
        isdar: muharrik.isdar.as_ref().map(ToString::to_string),
        khalfiya: wasf_khalfiya(muharrik.khalfiya).to_owned(),
        rusum: muharrik.rusum.iter().map(|q| q.ism().to_owned()).collect(),
        thiqa: muharrik.thiqa,
    }
}

/// The tier's own explanation in English, twin of [`Tabaqa::sharh_arabi`].
///
/// It belongs in `taarib-mustalahat` beside the Arabic one and is written here
/// only because that crate is being edited by another agent this wave. It is a
/// *first* copy rather than a second — no English wording for the tiers existed
/// anywhere before this — so moving it later is one paste and a deleted
/// function, not a reconciliation. The `match` is exhaustive on purpose: a
/// fourth tier must fail this build rather than fall through to a sentence
/// written for a different product.
const fn sharh_injilizi(tabaqa: Tabaqa) -> &'static str {
    match tabaqa {
        Tabaqa::Kamil => {
            "The game's own text is replaced from the inside. Menus, dialogue and \
             interface appear in Arabic as though the game had been built that way."
        },
        Tabaqa::RasmMubashir => {
            "Taarib draws the text itself over the game's own text objects. The result \
             looks native in most cases, and the effects the game applies to its text \
             are reproduced by Taarib rather than lost."
        },
        Tabaqa::TarjamaFawqiya => {
            "The game is not modified at all. Taarib reads what is on screen and shows \
             Arabic over it. This is a reading aid, not a translation installed inside \
             the game."
        },
    }
}

/// The capability report as the tier card writes it.
fn taqreer_hie(taqreer: &TaqreerImkaniyat) -> TaqreerHie {
    TaqreerHie {
        tabaqa: taqreer.tabaqa,
        tabaqa_raqm: taqreer.tabaqa.raqm(),
        tabaqa_arabi: taqreer.tabaqa.ism_arabi().to_owned(),
        tabaqa_injilizi: taqreer.tabaqa.ism_injilizi().to_owned(),
        sharh_arabi: taqreer.tabaqa.sharh_arabi().to_owned(),
        sharh_injilizi: sharh_injilizi(taqreer.tabaqa).to_owned(),
        sabab_arabi: taqreer.sabab_arabi.clone(),
        sabab_injilizi: taqreer.sabab_injilizi.clone(),
        anzimat: taqreer
            .anzimat_qabila
            .iter()
            .map(|q| q.wasf_arabi().to_owned())
            .collect(),
        hudud: taqreer
            .hudud
            .iter()
            .map(|hadd| hadd.arabi.clone())
            .collect(),
        hudud_injilizi: taqreer
            .hudud
            .iter()
            .map(|hadd| hadd.injilizi.clone())
            .collect(),
        marfuda: taqreer.marfuda,
        // The machine name rather than the enum: this crossing is already a
        // string for every other verdict on this record, and the interface
        // narrows it to a union of the three.
        jahiziya: taqreer.jahiziya.ism().to_owned(),
        naqs_arabi: taqreer.naqs.as_ref().map(|naqs| naqs.arabi.clone()),
        naqs_injilizi: taqreer.naqs.as_ref().map(|naqs| naqs.injilizi.clone()),
    }
}

/// The gap recorded when Steam's catalogue was not merely unreadable but never
/// located, so `taarib-aman` had nothing to record a gap against.
///
/// Written in the shape the other entries take — a file, then why it was not
/// read — because the banner draws them all as one list.
const THUGHRAT_MATJAR_STEAM: &str = "appcache/appinfo.vdf: Steam's install root was not found, \
     so the store catalogue was not read; VAC is declared only there and leaves nothing in the \
     game folder for the file scan to see";

/// The protection scan as the safety banner writes it.
///
/// `faqad_matjar` says the store catalogue was never opened for a game that has
/// one. It cannot be derived from `ijmaa`: an unread catalogue and an absent one
/// both arrive here as no evidence and no gap, and the banner has to tell them
/// apart.
fn himaya_hie(ijmaa: &IjmaaHimaya, faqad_matjar: bool) -> HimayaHie {
    let mut thughrat: Vec<String> = ijmaa
        .thughrat
        .iter()
        .map(|thughra| format!("{}: {}", thughra.masar.display(), thughra.sabab))
        .collect();
    if faqad_matjar {
        thughrat.push(THUGHRAT_MATJAR_STEAM.to_owned());
    }
    HimayaHie {
        mahmiya: kashf_himaya::mahmiya(ijmaa),
        anwa: ijmaa
            .anwa()
            .into_iter()
            .map(|naw| naw.arabi().to_owned())
            .collect(),
        adilla: ijmaa.adilla.iter().map(DaleelHimaya::arabi).collect(),
        thughrat,
        mabtur: ijmaa.mabtur,
    }
}

/// One observation as the evidence list writes it.
fn daleel_hie(daleel: &Daleel) -> DaleelHie {
    DaleelHie {
        naw: wasf_naw_daleel(daleel.naw).to_owned(),
        wasf: daleel.wasf.clone(),
        mawqi: daleel.mawqi.clone(),
        wazn: daleel.wazn,
    }
}

/// The scripting backend, named the way the report names it.
const fn wasf_khalfiya(khalfiya: KhalfiyaBarmajiya) -> &'static str {
    match khalfiya {
        KhalfiyaBarmajiya::Mono => "Mono",
        KhalfiyaBarmajiya::Il2cpp => "IL2CPP",
        KhalfiyaBarmajiya::UnrealNative => "C++ وبلوبرنت",
        KhalfiyaBarmajiya::GdScript => "GDScript",
        KhalfiyaBarmajiya::GodotCSharp => "‏C# في غودوت",
        KhalfiyaBarmajiya::GodotNative => "بناء غودوت مخصّص",
        KhalfiyaBarmajiya::JavaScript => "جافاسكربت",
        KhalfiyaBarmajiya::Python => "بايثون",
        KhalfiyaBarmajiya::Ruby => "روبي",
        KhalfiyaBarmajiya::GameMakerVm => "بايت‌كود GameMaker",
        KhalfiyaBarmajiya::Majhula => "غير معروفة",
    }
}

/// What kind of observation produced a piece of evidence.
const fn wasf_naw_daleel(naw: NawDaleel) -> &'static str {
    match naw {
        NawDaleel::BinyatMujallad => "بنية المجلدات",
        NawDaleel::TawqiThunai => "توقيع في ملف ثنائي",
        NawDaleel::BayanatMudmaja => "بيانات مدمجة في اللعبة",
        NawDaleel::TarwisatHawiya => "ترويسة حاوية أصول",
    }
}

// ---------------------------------------------------------------------------
// اللغة الرسمية — whether the publisher already ships Arabic
//
// `taarib-kashf` decides the question and states, at length, why it cannot open
// a `.pak` or a Unity serialized file to do it: the readers for those live in
// `taarib-muhawwil-unreal` and `taarib-istikhraj`, both of which depend on
// `taarib-muharrik`, which depends on `taarib-kashf`. The deep probe therefore
// arrives as a port. This is the one crate in the workspace that can implement
// it, because nothing depends on this one.
// ---------------------------------------------------------------------------

/// Codepoint ranges that mean a string is actually written in Arabic script.
///
/// Arabic and Arabic Supplement. A culture directory named `ar` full of English
/// is a stub the publisher shipped and never filled, and counting it as a
/// translation would deny the player the only Arabic they were going to get —
/// so the verdict needs the count of entries that carry the script, not the
/// count of entries in the culture.
const NITAQAT_ARABIYA: [(char, char); 2] = [('\u{600}', '\u{6ff}'), ('\u{750}', '\u{77f}')];

/// The prefix Unreal's cooked container paths carry, and which is not part of
/// the game's own layout: `../../../Atlas/Content/…`.
const BADIYAT_TARKEEB: &str = "../";

/// The container directory Unreal's own engine content sits under.
///
/// Load-bearing rather than tidy. Unreal has shipped `OnlineSubsystem` and
/// `OnlineSubsystemSteam` with a full culture set including `ar` in every
/// project built against a modern engine, so a probe that counted engine
/// targets would report official Arabic for a large share of every Unreal game
/// ever released — Little Nightmares, which has no Arabic at all, carries `ar`
/// in both.
const MUJALLAD_MUHARRIK: &str = "Engine/";

/// Whether a string carries Arabic script.
fn fihi_arabi(nass: &str) -> bool {
    nass.chars().any(|harf| {
        NITAQAT_ARABIYA
            .iter()
            .any(|(min, ila)| (*min..=*ila).contains(&harf))
    })
}

/// A count that cannot be represented is reported as the ceiling rather than
/// wrapping: the verdict compares counts, and a wrapped one would compare wrong.
fn adad_u64(adad: usize) -> u64 {
    u64::try_from(adad).unwrap_or(u64::MAX)
}

/// One culture's counted contents, before the fold turns it into evidence.
#[derive(Debug)]
struct HasilatThaqafa {
    /// The localization target the culture belongs to.
    hadaf: String,
    /// The culture, as the resource spells it.
    thaqafa: String,
    /// Where it was found, container and inner path.
    mawqi: String,
    /// How many entries it carries.
    adad: u64,
    /// How many of those carry Arabic script.
    arabi: u64,
    /// Whether the target belongs to the game rather than to the engine.
    li_luba: bool,
    /// Every culture the target's manifest declares.
    thaqafat: Vec<String>,
}

/// Turns counted cultures into the port's own vocabulary, with each target's
/// reference culture resolved.
///
/// The reference is the target's largest culture, which is the only definition
/// available without the `.locmeta`'s native-culture field for every target —
/// and it is the right one anyway: parity against the fullest culture is what
/// separates "the publisher translated the game" from "the publisher translated
/// the menus".
fn mawarid_min_hasilat(hasilat: Vec<HasilatThaqafa>, muharrik: &'static str) -> Vec<MawridLugha> {
    let mut marja: BTreeMap<String, u64> = BTreeMap::new();
    for hasila in &hasilat {
        let khana = marja.entry(hasila.hadaf.clone()).or_insert(0);
        *khana = (*khana).max(hasila.adad);
    }
    hasilat
        .into_iter()
        .map(|hasila| MawridLugha {
            muharrik,
            marja: marja.get(&hasila.hadaf).copied(),
            hadaf: hasila.hadaf,
            thaqafa: hasila.thaqafa,
            mawqi: hasila.mawqi,
            adad: hasila.adad,
            arabi: hasila.arabi,
            li_luba: hasila.li_luba,
            thaqafat: hasila.thaqafat,
        })
        .collect()
}

/// Strips the mount prefix a cooked container writes in front of every path.
fn masar_nazeef(masar: &str) -> String {
    masar.trim_start_matches(BADIYAT_TARKEEB).replace('\\', "/")
}

/// `Atlas/Content/Localization/Game/ar/Game.locres` → `("Game", "ar")`.
///
/// The culture is the file's own directory and the target is its parent, which
/// is the layout Unreal's cooker produces and the one every `.locres` in a
/// shipped game sits in.
fn hadaf_wa_thaqafa(masar: &str) -> Option<(String, String)> {
    let ajza: Vec<&str> = masar.split('/').collect();
    let akhir = ajza.len().checked_sub(2)?;
    let qabl = ajza.len().checked_sub(3)?;
    Some((
        (*ajza.get(qabl)?).to_owned(),
        (*ajza.get(akhir)?).to_owned(),
    ))
}

/// The localization target a `.locmeta` describes, taken from its directory.
fn hadaf_min_locmeta(masar: &str) -> String {
    Path::new(masar)
        .parent()
        .and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default()
        .to_owned()
}

/// Counts one `.locres`: its entries, and the entries written in Arabic.
fn ihsi_locres(bayt: &[u8]) -> Option<(u64, u64)> {
    let mawrid = locres::MawridLocres::min_bayt(bayt).ok()?;
    let arabi = mawrid
        .madakhil()
        .filter(|(_, madkhal)| mawrid.nass_madkhal(madkhal).is_some_and(fihi_arabi))
        .count();
    Some((mawrid.adad_madakhil(), adad_u64(arabi)))
}

/// The Unreal deep probe: the cultures the publisher compiled into the game.
#[derive(Debug, Default)]
struct MassahUnreal;

impl MassahUnreal {
    /// Reads one container's `.locmeta` manifests and `.locres` cultures.
    ///
    /// `manifests` and `hasilat` are filled rather than returned so that a game
    /// with several containers accumulates one target table across all of them,
    /// which is what a chunked cook produces.
    fn min_pak(masar: &Path, hasilat: &mut Vec<HasilatThaqafa>, majhul: &mut Vec<String>) {
        let hawiya = match pak::HawiyatPak::iftah(masar, None) {
            Ok(hawiya) => hawiya,
            Err(sabab) => {
                majhul.push(format!(
                    "the Unreal container {} would not open ({sabab}), so any Arabic compiled \
                     inside it was not seen",
                    masar.display()
                ));
                return;
            },
        };
        // A pruned index names no files, so the container is present and
        // unreadable rather than present and empty — a distinction the verdict
        // has to carry, because the second reads as "no Arabic here".
        if !hawiya.fahras().dalil_kamil() {
            majhul.push(format!(
                "{} carries a pruned directory index, so the localization files inside it \
                 cannot be enumerated",
                masar.display()
            ));
            return;
        }

        let ism_hawiya = masar
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default()
            .to_owned();
        let mut manifests: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let asma_meta: Vec<String> = hawiya.masarat_locmeta().map(str::to_owned).collect();
        for asl in asma_meta {
            if let Ok(bayt) = hawiya.iqra_masar(&asl)
                && let Ok(bayan) = locmeta::MawridLocmeta::min_bayt(&bayt)
            {
                let _ = manifests.insert(
                    hadaf_min_locmeta(&masar_nazeef(&asl)),
                    bayan.asma_thaqafat().map(str::to_owned).collect(),
                );
            }
        }

        let asma_res: Vec<String> = hawiya.masarat_locres().map(str::to_owned).collect();
        for asl in asma_res {
            let munaddaf = masar_nazeef(&asl);
            let Some((hadaf, thaqafa)) = hadaf_wa_thaqafa(&munaddaf) else {
                continue;
            };
            let Ok(bayt) = hawiya.iqra_masar(&asl) else {
                majhul.push(format!(
                    "{munaddaf} could not be decompressed out of {ism_hawiya}"
                ));
                continue;
            };
            let Some((adad, arabi)) = ihsi_locres(&bayt) else {
                continue;
            };
            hasilat.push(HasilatThaqafa {
                thaqafat: manifests.get(&hadaf).cloned().unwrap_or_default(),
                hadaf,
                thaqafa,
                mawqi: format!("{ism_hawiya}!{munaddaf}"),
                adad,
                arabi,
                li_luba: !munaddaf.starts_with(MUJALLAD_MUHARRIK),
            });
        }
    }

    /// The same, for an IoStore container.
    ///
    /// Read as well as `.pak` rather than instead of it: a UE5 game ships both,
    /// and a probe that read only the older format would answer "no Arabic" for
    /// every game cooked in the last four years.
    fn min_iostore(masar: &Path, hasilat: &mut Vec<HasilatThaqafa>, majhul: &mut Vec<String>) {
        let hawiya = match iostore::HawiyatIoStore::iftah(masar, None) {
            Ok(hawiya) => hawiya,
            Err(sabab) => {
                majhul.push(format!(
                    "the IoStore container {} would not open ({sabab}), so any Arabic compiled \
                     inside it was not seen",
                    masar.display()
                ));
                return;
            },
        };

        let ism_hawiya = masar
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_default()
            .to_owned();
        let mut manifests: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let asma_meta: Vec<String> = hawiya
            .masarat_locmeta()
            .map(|(dakhili, _)| dakhili.to_owned())
            .collect();
        for asl in asma_meta {
            if let Ok(bayt) = hawiya.iqra_masar(&asl)
                && let Ok(bayan) = locmeta::MawridLocmeta::min_bayt(&bayt)
            {
                let _ = manifests.insert(
                    hadaf_min_locmeta(&masar_nazeef(&asl)),
                    bayan.asma_thaqafat().map(str::to_owned).collect(),
                );
            }
        }

        let asma_res: Vec<String> = hawiya
            .masarat_locres()
            .map(|(dakhili, _)| dakhili.to_owned())
            .collect();
        for asl in asma_res {
            let munaddaf = masar_nazeef(&asl);
            let Some((hadaf, thaqafa)) = hadaf_wa_thaqafa(&munaddaf) else {
                continue;
            };
            let Ok(bayt) = hawiya.iqra_masar(&asl) else {
                majhul.push(format!(
                    "{munaddaf} could not be decompressed out of {ism_hawiya}"
                ));
                continue;
            };
            let Some((adad, arabi)) = ihsi_locres(&bayt) else {
                continue;
            };
            hasilat.push(HasilatThaqafa {
                thaqafat: manifests.get(&hadaf).cloned().unwrap_or_default(),
                hadaf,
                thaqafa,
                mawqi: format!("{ism_hawiya}!{munaddaf}"),
                adad,
                arabi,
                li_luba: !munaddaf.starts_with(MUJALLAD_MUHARRIK),
            });
        }
    }

    /// Every container this build carries, or nothing when it is not Unreal.
    fn hawiyat(jidhr: &Path) -> Vec<PathBuf> {
        taarib_muhawwil_unreal::afhas(jidhr)
            .map(|bina| bina.hawiyat)
            .unwrap_or_default()
    }
}

impl FahisMawarid for MassahUnreal {
    fn muarrif(&self) -> &'static str {
        "unreal"
    }

    fn imshi(&self, jidhr: &Path) -> Vec<MawridLugha> {
        let mut hasilat: Vec<HasilatThaqafa> = Vec::new();
        let mut mahdur: Vec<String> = Vec::new();
        for hawiya in Self::hawiyat(jidhr) {
            match hawiya.extension().and_then(std::ffi::OsStr::to_str) {
                Some("pak") => Self::min_pak(&hawiya, &mut hasilat, &mut mahdur),
                Some("utoc") => Self::min_iostore(&hawiya, &mut hasilat, &mut mahdur),
                _ => {},
            }
        }
        mawarid_min_hasilat(hasilat, "Unreal")
    }

    fn majhul(&self, jidhr: &Path) -> Vec<String> {
        // Re-walked rather than remembered from `imshi`: the port's two calls
        // are independent by contract, and a container that refused to open is
        // cheap to try again — it fails at the header.
        let mut hasilat: Vec<HasilatThaqafa> = Vec::new();
        let mut mahdur: Vec<String> = Vec::new();
        for hawiya in Self::hawiyat(jidhr) {
            match hawiya.extension().and_then(std::ffi::OsStr::to_str) {
                Some("pak") => Self::min_pak(&hawiya, &mut hasilat, &mut mahdur),
                Some("utoc") => Self::min_iostore(&hawiya, &mut hasilat, &mut mahdur),
                _ => {},
            }
        }
        mahdur
    }
}

/// The Unity deep probe: the locales its string tables were built for.
#[derive(Debug, Default)]
struct MassahUnity;

/// The key prefix Unity Localization writes for every string it owns.
const BADIYAT_UNITY: &str = "UnityLocalization/";

/// The key prefix the third-party `I2 Localization` asset writes.
const BADIYAT_I2: &str = "I2/";

/// The string-table collection a Unity engine key belongs to.
///
/// `UnityLocalization/<collection>/<key>@<locale>` names its collection; I2
/// keeps one flat table and is reported under its own name, because a target
/// with no name would collapse every collection into one and destroy the parity
/// figure the verdict rests on.
fn majmuat_unity(jidhr: &str) -> Option<String> {
    if let Some(baqi) = jidhr.strip_prefix(BADIYAT_UNITY) {
        let ism = baqi.split('/').next().unwrap_or_default();
        return (!ism.is_empty()).then(|| ism.to_owned());
    }
    jidhr.starts_with(BADIYAT_I2).then(|| "I2".to_owned())
}

impl FahisMawarid for MassahUnity {
    fn muarrif(&self) -> &'static str {
        "unity"
    }

    fn imshi(&self, jidhr: &Path) -> Vec<MawridLugha> {
        let (jadwal, _rafd) = taarib_istikhraj::unity::istakhrij(jidhr);
        let mut majmuat: BTreeMap<(String, String), (u64, u64)> = BTreeMap::new();
        for madkhal in jadwal.madakhil() {
            let Some(miftah) = madkhal.mawqi.miftah_muharrik.as_deref() else {
                continue;
            };
            let Some((asas, lugha)) = miftah.rsplit_once('@') else {
                continue;
            };
            let Some(majmua) = majmuat_unity(asas).filter(|_| !lugha.is_empty()) else {
                continue;
            };
            let khana = majmuat
                .entry((majmua, lugha.to_lowercase()))
                .or_insert((0, 0));
            khana.0 = khana.0.saturating_add(1);
            if fihi_arabi(&madkhal.khaam) {
                khana.1 = khana.1.saturating_add(1);
            }
        }

        let hasilat: Vec<HasilatThaqafa> = majmuat
            .into_iter()
            .map(|((majmua, lugha), (adad, arabi))| HasilatThaqafa {
                mawqi: format!("{majmua}@{lugha}"),
                hadaf: majmua,
                thaqafa: lugha,
                adad,
                arabi,
                // Unity has no engine-owned localization target the way Unreal
                // does: every string table in a build was authored by whoever
                // made the game.
                li_luba: true,
                thaqafat: Vec::new(),
            })
            .collect();
        mawarid_min_hasilat(hasilat, "Unity")
    }

    fn majhul(&self, jidhr: &Path) -> Vec<String> {
        let mut majhul = Vec::new();
        let Some(bayanat) = mujallad_bayanat_unity(jidhr) else {
            return majhul;
        };
        let mudara = bayanat.join("Managed");
        let Ok(qira) = std::fs::read_dir(&mudara) else {
            return majhul;
        };
        // A game that ships its own localization assembly keeps its language
        // data somewhere this probe does not read. Hollow Knight is the case
        // that made this necessary: it answers `Ghaib` at forty percent
        // confidence, and forty is the honest number.
        let khassa: Vec<String> = qira
            .flatten()
            .filter_map(|madkhal| {
                let ism = madkhal.file_name().to_string_lossy().into_owned();
                let munkhafid = ism.to_lowercase();
                let lil_muharrik = munkhafid.starts_with("unity")
                    || munkhafid.starts_with("com.unity")
                    || munkhafid.starts_with("sirenix");
                (munkhafid.contains("localization") && !lil_muharrik).then_some(ism)
            })
            .collect();
        if !khassa.is_empty() {
            majhul.push(format!(
                "this game ships its own localization assembly ({}), whose language data is not \
                 a Unity string table and is not read by this probe",
                khassa.join(", ")
            ));
        }
        majhul
    }
}

/// The `<Game>_Data` directory a Unity build keeps its assemblies under.
fn mujallad_bayanat_unity(jidhr: &Path) -> Option<PathBuf> {
    std::fs::read_dir(jidhr)
        .ok()?
        .flatten()
        .find_map(|madkhal| {
            let masar = madkhal.path();
            let ism = madkhal.file_name().to_string_lossy().into_owned();
            (ism.ends_with("_Data") && masar.is_dir()).then_some(masar)
        })
}

/// Everything one process remembers about official Arabic.
///
/// Process-global rather than managed state, and the reason is the pre-flight
/// gate: `taqdeem_awamir` reaches the verdict from seven places, none of which
/// holds the game's install root, and threading a store through all of them
/// would put plumbing in a file this change has no other business in. The
/// precedent is `mukawwinat_tahmil`'s resource root, which is global for the
/// same kind of reason.
///
/// Nothing here survives the process. `taarib-kashf` states why it persists
/// nothing itself, and this does not change that: a verdict is a fact about
/// files on disk that a game update can move, and a stale one on disk would
/// outlive the reason it was true.
#[derive(Debug, Default)]
struct DhakiratLugha {
    /// Verdicts from the shallow pass — the launcher's declared languages and
    /// the locale-shaped paths under the install root, which is what a library
    /// scan can afford for every game at once.
    sathiya: KhazinaDhakira,
    /// Verdicts that also opened the engine's own containers. Kept apart from
    /// the shallow ones because the cache key does not record which probes ran,
    /// so one store holding both would serve a shallow answer to a caller that
    /// asked for a deep one.
    amiqa: KhazinaDhakira,
    /// Steam's declared languages, read once per library scan and shared by
    /// every game in it: `appinfo.vdf` holds a quarter of a million
    /// applications and streaming it once per game would be the same file read
    /// seventeen times.
    lughat: Mutex<BTreeMap<u32, LughatMuallana>>,
}

static DHAKIRA: LazyLock<DhakiratLugha> = LazyLock::new(DhakiratLugha::default);

/// Records the launcher-declared languages a library scan read.
pub fn sajjil_lughat_matjar(fahras: BTreeMap<u32, LughatMuallana>) {
    *DHAKIRA.lughat.lock() = fahras;
}

/// What Steam declares for one game, when the last scan read anything for it.
#[must_use]
pub fn lughat_matjar(appid: Option<u32>) -> Option<LughatMuallana> {
    let raqm = appid?;
    DHAKIRA.lughat.lock().get(&raqm).cloned()
}

/// The launcher identity the verdict is keyed on: the store the game really
/// belongs to, with any manager unwrapped.
///
/// A library row always has one. The fallback is for a row that somehow does
/// not, and it reports what such a row would actually be — a game no store
/// knows about — rather than making the verdict unanswerable.
fn masdar_lil_hukm(luba: &Luba) -> MasdarLuba {
    luba.masdar_asli()
        .cloned()
        .unwrap_or_else(|| MasdarLuba::Yadawi(luba.id.to_string()))
}

/// The shallow verdict for one game: the store's listing and the locale-shaped
/// paths, with no container opened.
///
/// What the library grid badges from. It is deliberately not what the install
/// path gates on — a store listing naming Arabic for a console SKU the PC build
/// never shipped produces [`HalatLughaRasmiya::Mubhama`] here and `Ghaib` once
/// the game's own files are read — but it is enough to mark a card, and it is
/// the only answer that costs a bounded directory walk rather than a container
/// decode per game.
#[must_use]
pub fn hukm_sathi(luba: &Luba, lughat: Option<&LughatMuallana>) -> HukmLughaRasmiya {
    let masdar = masdar_lil_hukm(luba);
    Fahis::jadeed()
        .bi_khazina(&DHAKIRA.sathiya)
        .ifhas(&TalabLugha {
            luba: luba.id,
            masdar: &masdar,
            jidhr: &luba.jidhr,
            bina: luba.bina.as_ref().and_then(|bina| bina.manassa.as_deref()),
            lughat,
        })
}

/// The full verdict for one game, engine containers included.
///
/// Computed once per game per process. The walk costs about a second on an
/// Unreal game and rather more on a large Unity one, which is why it is a
/// command of its own rather than part of the detail screen's first answer.
#[must_use]
pub fn hukm_amiq(luba: &Luba, lughat: Option<&LughatMuallana>) -> HukmLughaRasmiya {
    let unreal = MassahUnreal;
    let unity = MassahUnity;
    let massah: [&dyn FahisMawarid; 2] = [&unreal, &unity];
    let masdar = masdar_lil_hukm(luba);
    Fahis::jadeed()
        .bi_massah(&massah)
        .bi_khazina(&DHAKIRA.amiqa)
        .ifhas(&TalabLugha {
            luba: luba.id,
            masdar: &masdar,
            jidhr: &luba.jidhr,
            bina: luba.bina.as_ref().and_then(|bina| bina.manassa.as_deref()),
            lughat,
        })
}

/// The best verdict this process already holds for one game, without computing
/// anything.
///
/// Deep before shallow, because evidence only accumulates: a deep verdict saw
/// everything the shallow one did and the containers as well. [`None`] when
/// neither pass has run, which is what the pre-flight gate reports as "not
/// checked" rather than as "no Arabic".
#[must_use]
pub fn hukm_mukhazzan(luba: LubaId) -> Option<HukmLughaRasmiya> {
    DHAKIRA
        .amiqa
        .jalb(luba)
        .or_else(|| DHAKIRA.sathiya.jalb(luba))
        .map(|sijill| sijill.hukm)
}

/// One observation behind an official-Arabic verdict, rendered.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DaleelLughaHie {
    /// How it was observed, in Arabic.
    pub naw: String,
    /// What was observed, as the detector stated it.
    pub wasf: String,
    /// Where — a store key, or a path inside a container.
    pub mawqi: Option<String>,
    /// How much this observation is worth, 0 to 100.
    pub wazn: u8,
}

/// Whether a game's publisher already ships Arabic, as the interface draws it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LughaRasmiyaHie {
    /// The badge state.
    pub hala: HalatLughaRasmiya,
    /// Whether the publisher ships Arabic in any amount. The single question
    /// the arabization surface asks, sent as its own field so the interface
    /// cannot re-derive it from the badge and get it wrong.
    pub yatakallam_arabi: bool,
    /// Whether the verdict rests on anything that was actually seen. A `Ghaib`
    /// with this `false` is the absence of evidence, not evidence of absence.
    pub hasim: bool,
    /// How much the verdict is worth, 0 to 100.
    pub thiqa: u8,
    /// Everything that was observed, heaviest first.
    pub dalail: Vec<DaleelLughaHie>,
    /// What could not be seen, one sentence each.
    pub majhul: Vec<String>,
    /// The launcher's declared languages, in its own spelling.
    pub lughat_muallana: Vec<String>,
    /// Whether the engine's own containers were opened for this verdict.
    pub amiq: bool,
    /// When it was decided, RFC 3339.
    pub waqt: String,
}

/// The verdict as the detail screen writes it.
#[must_use]
pub fn lugha_hie(hukm: &HukmLughaRasmiya, amiq: bool) -> LughaRasmiyaHie {
    LughaRasmiyaHie {
        hala: hukm.hala(),
        yatakallam_arabi: hukm.yatakallam_arabi(),
        hasim: hukm.hasim(),
        thiqa: hukm.thiqa,
        dalail: hukm.dalail.iter().map(daleel_lugha_hie).collect(),
        majhul: hukm.majhul.clone(),
        lughat_muallana: hukm.lughat_muallana.clone(),
        amiq,
        waqt: hukm.waqt.clone(),
    }
}

/// One piece of language evidence as the list writes it.
fn daleel_lugha_hie(daleel: &DaleelLugha) -> DaleelLughaHie {
    DaleelLughaHie {
        naw: wasf_naw_lugha(daleel.naw).to_owned(),
        wasf: daleel.wasf.clone(),
        mawqi: daleel.mawqi.clone(),
        wazn: daleel.wazn,
    }
}

/// What kind of observation produced a piece of language evidence.
const fn wasf_naw_lugha(naw: NawDaleelLugha) -> &'static str {
    match naw {
        NawDaleelLugha::LughatMatjar => "لغات المتجر المعلنة",
        NawDaleelLugha::MawridMuharrik => "موارد المحرّك المترجَمة",
        NawDaleelLugha::MasarThaqafa => "مسار ثقافة في ملفات اللعبة",
    }
}

/// Decides whether a game's publisher already ships Arabic, containers included.
///
/// A command of its own rather than a field on [`tafasil_luba`], because the
/// walk it pays for is seconds on a large game and the rest of the detail
/// screen has no reason to wait for it. The interface holds the arabization
/// surface closed until this answers, which is the only safe order: offering a
/// patch and withdrawing it a second later is worse than not offering it yet.
///
/// # Errors
///
/// As [`tafasil_luba`], minus the probe: the language walk never fails, and a
/// container it cannot read becomes a sentence in
/// [`LughaRasmiyaHie::majhul`].
#[tauri::command]
#[specta::specta]
pub async fn lugha_rasmiya(
    muarrif: String,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<LughaRasmiyaHie, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    if !luba.mawjuda || !luba.jidhr.exists() {
        return Err(Khata::from(KhataLuba::LubaGhayrMawjuda {
            ism: luba.ism,
            jidhr: luba.jidhr,
        }));
    }
    let lughat = lughat_matjar(appid_steam(&luba));
    let hukm = hukm_amiq(&luba, lughat.as_ref());
    tracing::info!(
        luba = %id,
        hala = ?hukm.hala(),
        thiqa = hukm.thiqa,
        hasim = hukm.hasim(),
        "official-language verdict"
    );
    Ok(lugha_hie(&hukm, true))
}

// ---------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------

/// Failures of the game-detail surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataLuba {
    /// The identity the interface sent is not one Taarib issues.
    #[error("{muarrif} is not a Taarib game identity")]
    MuarrifGhayrSalih {
        /// The text that was sent.
        muarrif: String,
    },

    /// No game in the library carries that identity.
    #[error("no game in the library is {muarrif}")]
    LubaMafquda {
        /// The identity that was looked up.
        muarrif: String,
    },

    /// The game is recorded but its files are gone, so nothing can be probed.
    #[error("{ism} is not on disk at {}", jidhr.display())]
    LubaGhayrMawjuda {
        /// The game's name.
        ism: String,
        /// Where the library last saw it.
        jidhr: PathBuf,
    },

    /// The game came from Steam and Steam's own root could not be found, so the
    /// half of the safety scan that reads Steam's catalogue cannot run.
    #[error("{ism} is a Steam game and Steam's install root could not be found")]
    JidhrSteamMajhul {
        /// The game's name.
        ism: String,
    },
}

impl Tafsir for KhataLuba {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MuarrifGhayrSalih { .. } => 10,
                    Self::LubaMafquda { .. } => 11,
                    Self::LubaGhayrMawjuda { .. } => 12,
                    Self::JidhrSteamMajhul { .. } => 13,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // The interface asked about something that is not there; nothing
            // was touched and nothing is left in a half state.
            Self::MuarrifGhayrSalih { .. } | Self::LubaMafquda { .. } => Khutura::Khatar,
            // Both are a disagreement the user can see and settle themselves:
            // the library against the disk, or Taarib against where Steam is.
            // Nothing was written in either.
            Self::LubaGhayrMawjuda { .. } | Self::JidhrSteamMajhul { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MuarrifGhayrSalih { .. } => {
                "المعرّف المطلوب ليس معرّف لعبة يصدره تعريب. أعد فحص المكتبة ثم افتح اللعبة \
                 من جديد."
                    .to_owned()
            },
            Self::LubaMafquda { .. } => {
                "لم تعد هذه اللعبة في مكتبة تعريب. ربما حُذفت من مشغّلها بعد آخر فحص؛ أعد \
                 فحص المكتبة."
                    .to_owned()
            },
            Self::LubaGhayrMawjuda { ism, jidhr } => format!(
                "لم يعد مجلد {ism} موجودًا في {}. لا يمكن فحص محرّك لعبة ليست على القرص؛ \
                 أعد تثبيتها من مشغّلها أو دلّ تعريب على مكانها الجديد.",
                jidhr.display()
            ),
            Self::JidhrSteamMajhul { ism } => format!(
                "{ism} لعبة من ستيم، ولم يعثر تعريب على مجلد ستيم نفسه. نصف فحص مكافحة الغش \
                 يقرأ كتالوج ستيم، وحماية VAC لا تُعلَن إلا فيه ولا تترك أثرًا في مجلد \
                 اللعبة — ففحصٌ بلا كتالوج يعطي النتيجة نفسها التي تعطيها لعبة نظيفة. لم \
                 يُكتب شيء. حدِّد مجلد ستيم في الإعدادات ثم أعد المحاولة."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MuarrifGhayrSalih { muarrif } => {
                format!("{muarrif} is not a game identity Taarib issues. Rescan the library.")
            },
            Self::LubaMafquda { muarrif } => format!(
                "No game in the library is {muarrif} any more. It was probably removed from \
                 its launcher after the last scan; rescan the library."
            ),
            Self::LubaGhayrMawjuda { ism, jidhr } => format!(
                "{ism} is no longer at {}. An engine probe reads the game's files, so there \
                 is nothing to probe. Reinstall it from its launcher, or point Taarib at \
                 where it is now.",
                jidhr.display()
            ),
            Self::JidhrSteamMajhul { ism } => format!(
                "{ism} is a Steam game and Steam's own install root could not be found. Half \
                 the anti-cheat scan reads Steam's catalogue, and VAC is declared only there \
                 and leaves nothing in the game folder — so a scan without it returns the \
                 same clean verdict a genuinely clean game gets. Nothing was written. Set \
                 Steam's folder in Settings and try again."
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MuarrifGhayrSalih { .. } | Self::LubaMafquda { .. } => Khutwa::AadaFahsMaktaba,
            Self::LubaGhayrMawjuda { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            // The launcher-locations section, which is where the override that
            // makes this answerable is typed.
            Self::JidhrSteamMajhul { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Manassat,
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MuarrifGhayrSalih { muarrif } | Self::LubaMafquda { muarrif } => {
                let _ = siyaq.insert("muarrif".to_owned(), QeemaSiyaq::Nass(muarrif.clone()));
            },
            Self::LubaGhayrMawjuda { ism, jidhr } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            },
            Self::JidhrSteamMajhul { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataLuba);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ikhtibarat {
    use taarib_aman::kashf_himaya::ThughraFahs;
    use taarib_mustalahat::luba::SuwarLuba;
    use taarib_mustalahat::muharrik::Muharrik;
    use taarib_usus::idadat::IdadatManassat;
    use taarib_usus::manassa::{BeeatTawafuq, Mimariya};

    use super::*;

    /// Anything a test can fail on: a `Khata` from the code under test, or an
    /// [`std::io::Error`] from the scratch directory it staged.
    type NatijatIkhtibar = Result<(), Box<dyn std::error::Error>>;

    /// The refusal's code, so the assertions name the failure and not the enum.
    const RAMZ_JIDHR_MAJHUL: u16 = arqam::STUDIO + 13;

    /// Steam's own test application, which every Steam install carries.
    const TATBEEQ_IKHTIBAR: u32 = 480;

    /// A scratch directory that removes itself, so a failed assertion does not
    /// leave one behind in the machine's temporary directory.
    struct JidhrMuaqqat(PathBuf);

    impl Drop for JidhrMuaqqat {
        fn drop(&mut self) {
            // Best effort: a test that already failed must not fail twice.
            #[expect(
                clippy::disallowed_methods,
                reason = "a scratch directory under `std::env::temp_dir()` removing itself, never a data root or a game directory"
            )]
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A fresh scratch root under the temporary directory.
    fn jidhr_muaqqat() -> JidhrMuaqqat {
        let jidhr = std::env::temp_dir().join(format!("taarib-luba-{}", uuid::Uuid::new_v4()));
        JidhrMuaqqat(jidhr)
    }

    /// A directory that passes `taarib-kashf`'s "this is really a Steam root"
    /// test, which wants a `steamapps` or a `config` beside it.
    fn jidhr_steam_zaif(masar: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(masar.join("steamapps"))?;
        Ok(masar.to_path_buf())
    }

    /// A library row, from whichever launcher identity the caller wants.
    fn luba_min_masdar(masdar: MasdarLuba, jidhr: PathBuf) -> Luba {
        Luba {
            id: LubaId::min_masdar(&masdar, "Luba Ikhtibar"),
            masadir: vec![masdar],
            ism: "Luba Ikhtibar".to_owned(),
            jidhr,
            tanfidhi: None,
            hajm: 0,
            akhir_laab: None,
            akhir_tahdith: None,
            bina: None,
            suwar: SuwarLuba::default(),
            beea: BeeatTawafuq::Asli,
            mawjuda: true,
            mukhfiya: false,
        }
    }

    /// Settings with one launcher override set and nothing else.
    fn idadat_bi_tajawuz(steam: Option<PathBuf>) -> Idadat {
        Idadat {
            manassat: IdadatManassat {
                steam,
                ..IdadatManassat::default()
            },
            ..Idadat::default()
        }
    }

    /// The override still wins, on every platform, exactly as the scan lets it.
    #[test]
    fn tajawuz_yaghlib_ala_alkashf() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let tajawuz = jidhr_steam_zaif(&haris.0.join("steam-yadawi"))?;
        let masarat = Masarat::min_judhur(haris.0.join("bayanat"), haris.0.join("idadat"));

        let mahsul = jidhr_steam(&masarat, &idadat_bi_tajawuz(Some(tajawuz.clone())))?;

        assert_eq!(mahsul, Some(tajawuz));
        Ok(())
    }

    /// The defect itself: with no override set at all, the root is still found.
    ///
    /// Linux-only because that is where the candidate list is rooted at the home
    /// directory, and `Masarat::min_judhur` puts the home directory inside the
    /// scratch root — so the whole thing is decided by files this test made. On
    /// Windows the same code path reads the registry, which no test can stage.
    #[cfg(target_os = "linux")]
    #[test]
    fn jidhr_yuhall_bila_ayy_tajawuz() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let manzil = haris.0.join("manzil");
        let mutawaqqa = jidhr_steam_zaif(&manzil.join(".steam").join("steam"))?;
        let masarat = Masarat::min_judhur(manzil.join("bayanat"), manzil.join("idadat"));

        let mahsul = jidhr_steam(&masarat, &Idadat::default())?;

        assert_eq!(mahsul, Some(mutawaqqa));
        Ok(())
    }

    /// A Steam game with no root anywhere is refused, by name and by code.
    #[test]
    fn hasm_yarfud_luba_steam_bila_jidhr() {
        let khata = hasm_jidhr_steam(None, Some(TATBEEQ_IKHTIBAR), "Luba Ikhtibar")
            .err()
            .map(|khata| khata.ramz);

        assert_eq!(khata, Some(Ramz::jadeed(RAMZ_JIDHR_MAJHUL)));
    }

    /// A game no store owns passes with nothing, because there is genuinely no
    /// catalogue to consult — the case the refusal must not swallow.
    #[test]
    fn hasm_yasmah_luba_ghayr_steam() -> NatijatIkhtibar {
        assert_eq!(hasm_jidhr_steam(None, None, "Luba Ikhtibar")?, None);
        Ok(())
    }

    /// A Steam game with a root passes the root through untouched.
    #[test]
    fn hasm_yumarrir_aljidhr() -> NatijatIkhtibar {
        let jidhr = PathBuf::from("steam");
        let mahsul =
            hasm_jidhr_steam(Some(jidhr.clone()), Some(TATBEEQ_IKHTIBAR), "Luba Ikhtibar")?;

        assert_eq!(mahsul, Some(jidhr));
        Ok(())
    }

    /// The composed gate the three install paths call: a Steam game and no
    /// override. On a machine that has Steam it answers with a root; on one that
    /// does not it refuses. What it never does any more is hand the safety layer
    /// an empty root and let the install go on.
    #[test]
    fn bawwabat_alfahs_la_tuiid_faragh_li_luba_steam() {
        let haris = jidhr_muaqqat();
        let masarat = Masarat::min_judhur(haris.0.join("bayanat"), haris.0.join("idadat"));
        let luba = luba_min_masdar(MasdarLuba::Steam(TATBEEQ_IKHTIBAR), haris.0.join("luba"));

        match jidhr_steam_lil_fahs(&masarat, &Idadat::default(), &luba) {
            Ok(mahsul) => assert!(mahsul.is_some(), "a Steam game may not pass with no root"),
            Err(khata) => assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_JIDHR_MAJHUL)),
        }
    }

    /// A game outside Steam is never refused for Steam's absence.
    #[test]
    fn bawwabat_alfahs_tasmah_li_luba_ghayr_steam() {
        let haris = jidhr_muaqqat();
        let masarat = Masarat::min_judhur(haris.0.join("bayanat"), haris.0.join("idadat"));
        let luba = luba_min_masdar(
            MasdarLuba::Yadawi("luba-ikhtibar".to_owned()),
            haris.0.join("luba"),
        );

        assert!(jidhr_steam_lil_fahs(&masarat, &Idadat::default(), &luba).is_ok());
    }

    /// An empty scan report with no store catalogue behind it.
    fn ijmaa_farigh() -> IjmaaHimaya {
        IjmaaHimaya {
            jidhr: PathBuf::from("luba"),
            adilla: Vec::new(),
            thughrat: Vec::new(),
            mabtur: false,
        }
    }

    /// The banner tells "we read the catalogue and it named nothing" apart from
    /// "the catalogue was never opened" — the display half of the same defect.
    #[test]
    fn albanar_yusammi_kataloge_lam_yuqra() {
        let maqru = himaya_hie(&ijmaa_farigh(), false);
        let mafqud = himaya_hie(&ijmaa_farigh(), true);

        assert!(maqru.thughrat.is_empty());
        assert_eq!(mafqud.thughrat, vec![THUGHRAT_MATJAR_STEAM.to_owned()]);
        // Neither is evidence of anti-cheat; the gap is a gap, not a detection.
        assert!(!maqru.mahmiya && !mafqud.mahmiya);
    }

    /// The gap is added to the scan's own gaps rather than in place of them.
    #[test]
    fn albanar_yahfaz_thughrat_almassah() {
        let mut ijmaa = ijmaa_farigh();
        ijmaa.thughrat.push(ThughraFahs {
            masar: PathBuf::from("luba/bin"),
            sabab: "not a directory".to_owned(),
        });

        let hie = himaya_hie(&ijmaa, true);

        assert_eq!(hie.thughrat.len(), 2);
        assert!(
            hie.thughrat
                .iter()
                .any(|satr| satr.contains("not a directory"))
        );
        assert!(
            hie.thughrat
                .iter()
                .any(|satr| satr == THUGHRAT_MATJAR_STEAM)
        );
    }

    /// Both multiplayer hints raise the question, and nothing else does.
    ///
    /// The narrow reading — online only — is the tempting one and the wrong one:
    /// the gate this announces, `taarib_aman::kashf_shabaka::mutaaddid`, refuses
    /// a shared-screen title exactly as it refuses an online one, so warning on
    /// the narrower set would leave the wider refusal unannounced.
    #[test]
    fn kilaa_simatay_aljamai_tastadaiyan_alsual() {
        assert!(yalzam_iqrar_shabaka(&[SimatLuba::JamaiOnline]));
        assert!(yalzam_iqrar_shabaka(&[SimatLuba::JamaiMahalli]));
        assert!(yalzam_iqrar_shabaka(&[
            SimatLuba::TabaqatTawafuq("verified".to_owned()),
            SimatLuba::JamaiMahalli,
        ]));
    }

    /// A game the launcher said nothing about asks nothing, and an anti-cheat
    /// hint is not a multiplayer hint.
    ///
    /// The store carries the two independently and this predicate reads only
    /// its own two rows: a game whose catalogue names EAC and nothing else is
    /// refused by the anti-cheat gate, not asked a question it never earned.
    #[test]
    fn simat_ukhra_la_tastadai_sual_alshabaka() {
        assert!(!yalzam_iqrar_shabaka(&[]));
        assert!(!yalzam_iqrar_shabaka(&[SimatLuba::MuammanaVac]));
        assert!(!yalzam_iqrar_shabaka(&[
            SimatLuba::HimayaMuhtamala("EasyAntiCheat".to_owned()),
            SimatLuba::LaysatLuba("tool".to_owned()),
            SimatLuba::TabaqatTawafuq("platinum".to_owned()),
        ]));
    }

    /// Whether a sentence carries any Arabic script at all.
    ///
    /// The assertion the English fields exist for: a reader who chose English is
    /// shown a sentence with no Arabic in it, rather than the Arabic one under
    /// an English heading.
    fn fiha_arabi(nass: &str) -> bool {
        nass.chars()
            .any(|harf| matches!(harf, '\u{0600}'..='\u{06ff}' | '\u{0750}'..='\u{077f}'))
    }

    /// One identified engine, with nothing invented beyond the family.
    fn muharrik_ikhtibar(aila: AilatMuharrik, khalfiya: KhalfiyaBarmajiya) -> Muharrik {
        Muharrik {
            aila,
            isdar: None,
            khalfiya,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 90,
            dalail: Vec::new(),
        }
    }

    /// A real capability report, written by the same function the probe calls.
    fn taqreer_ikhtibar(aila: AilatMuharrik, khalfiya: KhalfiyaBarmajiya) -> TaqreerImkaniyat {
        taarib_muharrik::imkaniyat::taqreer(
            muharrik_ikhtibar(aila, khalfiya),
            &[],
            "2026-01-01T00:00:00Z".to_owned(),
        )
    }

    /// The tier reaches the card as the discriminant and in both languages, and
    /// the English half holds no Arabic.
    ///
    /// The defect this closes: the record carried the tier's Arabic name and a
    /// number, so an English session read `طبقة ترجمة` where the name of the
    /// product belongs. `HukmTilqaiHie` had sent both names all along, which is
    /// what made this record the odd one out rather than a consistent choice.
    #[test]
    fn bitaqat_altabaqa_tasil_bil_lughatayn() {
        for aila in [
            AilatMuharrik::Unity,
            AilatMuharrik::Renpy,
            AilatMuharrik::Majhul,
        ] {
            let asli = taqreer_ikhtibar(aila, KhalfiyaBarmajiya::Majhula);
            let hie = taqreer_hie(&asli);

            assert_eq!(hie.tabaqa, asli.tabaqa, "{aila:?}");
            assert_eq!(hie.tabaqa_raqm, asli.tabaqa.raqm(), "{aila:?}");
            assert_eq!(hie.tabaqa_arabi, asli.tabaqa.ism_arabi(), "{aila:?}");
            assert_eq!(hie.tabaqa_injilizi, asli.tabaqa.ism_injilizi(), "{aila:?}");
            assert!(
                !fiha_arabi(&hie.tabaqa_injilizi),
                "an English tier name must hold no Arabic: {}",
                hie.tabaqa_injilizi
            );
        }
    }

    /// The tier's own paragraph crosses in both languages, worded by the
    /// backend rather than paraphrased in a locale file.
    #[test]
    fn sharh_altabaqa_yasil_bil_lughatayn() {
        for tabaqa in [Tabaqa::Kamil, Tabaqa::RasmMubashir, Tabaqa::TarjamaFawqiya] {
            let injilizi = sharh_injilizi(tabaqa);
            assert!(!injilizi.is_empty(), "{tabaqa:?}");
            assert!(
                !fiha_arabi(injilizi),
                "an English explanation holds no Arabic: {injilizi}"
            );
            assert!(fiha_arabi(tabaqa.sharh_arabi()), "{tabaqa:?}");
        }

        // And on a report, so the wiring is proved and not only the table.
        let asli = taqreer_ikhtibar(AilatMuharrik::Unity, KhalfiyaBarmajiya::Mono);
        let hie = taqreer_hie(&asli);

        assert_eq!(hie.sharh_arabi, asli.tabaqa.sharh_arabi());
        assert_eq!(hie.sharh_injilizi, sharh_injilizi(asli.tabaqa));
    }

    /// The engine family crosses as the discriminant, so an unidentified engine
    /// is a value the interface can match rather than an Arabic literal.
    ///
    /// `aila` is checked here too, and deliberately: it is what proves the
    /// rendered name is Arabic for `Majhul`, which is the whole reason the
    /// discriminant had to be sent beside it. Most of a real library lands on
    /// that arm.
    #[test]
    fn ailat_almuharrik_tasil_ka_ramz_la_ka_nass() {
        let maruf = muharrik_hie(&taqreer_ikhtibar(
            AilatMuharrik::Unity,
            KhalfiyaBarmajiya::Il2cpp,
        ));
        assert_eq!(maruf.aila_ramz, AilatMuharrik::Unity);
        assert_eq!(maruf.aila, "Unity");

        let majhul = muharrik_hie(&taqreer_ikhtibar(
            AilatMuharrik::Majhul,
            KhalfiyaBarmajiya::Majhula,
        ));
        assert_eq!(majhul.aila_ramz, AilatMuharrik::Majhul);
        assert!(
            fiha_arabi(&majhul.aila),
            "the rendered name for an unidentified engine is Arabic, which is why the \
             discriminant is sent: {}",
            majhul.aila
        );
    }
}
