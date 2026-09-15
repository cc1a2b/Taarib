//! التعريب التلقائي — the command layer over `taarib-tilqai`: the verdict, the run, and the stop.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use taarib_aman::iqrar::{self, SijillIqrar};
use taarib_aman::kashf_himaya::{
    DaleelHimaya, HalatMatjar, IjmaaHimaya, NawHimaya, ifhas_himaya_bi_qiraa, mahmiya,
};
use taarib_aman::kashf_shabaka::{ifhas_shabaka_bi_qiraa, mutaaddid};
use taarib_aman::matjar::QiraatMatjar;
use taarib_aman::qaimat_sahb::QaimaMuraqaba;
use taarib_istikhraj::rafd::TaqreerRafd;
use taarib_kashf::fahs::SimatLuba;
use taarib_khatm::MiftahKhass;
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::luba::{Luba, LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::{AilatMuharrik, TaqreerImkaniyat};
use taarib_mustalahat::ruqaa::RukhsaRuqaa;
use taarib_saff::khatt::MawridKhatt;
use taarib_tarjama::muzawwidun::Muzawwid;
use taarib_tathbeet::bayan::waqt_alaan;
use taarib_tilqai::mashwar::{MALAF_JADWAL, MUJALLAD_MASHRU};
use taarib_tilqai::{
    HalatMashwar, IhsaIstikhraj, KhiyaratTilqai, LubaTilqai, MarhalaTilqai, MashwarId, MiqbadIlgha,
    MudkhalatAman, MukhbirTaqaddum, NatijatMashwar, QaydMarhala, SijillMashwar, TalabTilqai,
    Taqaddum, WasfTilqai, arrib, ijrud, naqs_jahiziya, tahaqquq_jahiziya,
};
use taarib_usus::ISDAR;
use taarib_usus::idadat::{HalatMuzawwidin, Idadat, MakhzanIdadat, NawMuzawwid};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::Masarat;
use tauri::Emitter as _;

use crate::luba_awamir::{appid_steam, huwiya, ijlib_luba, simat_luba, taqreer_luba};
use crate::tathbeet_awamir::{HalatSahbHie, iqra_qaimat_sahb, sahb_hie};
use crate::warsha_awamir::{bin_muzawwid, dolar, lahza_alaan, muzawwid_muntakhab, nano_min_dolar};

/// The window event every run snapshot is published on.
pub const ISM_HADATH_TILQAI: &str = "taarib://tilqai";

/// Where automatic runs keep their directories, under the data root.
pub(crate) const MUJALLAD_TILQAI: &str = "tilqai";

/// The longest font chain a run is handed.
///
/// Every font in the chain is copied into the run directory and put through the
/// Arabic validator by `taarib_tilqai::bina::hayyi`, so a chain of everything
/// installed would pay a copy and a parse for faces no string ever reaches. Four
/// is the head plus three fallbacks, which is what shaping actually walks.
const AQSA_KHUTUT: usize = 4;

/// The shortest gap between two published snapshots.
///
/// The translation stage reports once per settled reply, which for a four
/// thousand string game is four thousand events; an IPC message each would cost
/// the window more than the run does. A stage change is published immediately
/// whatever this says, so nothing a user is waiting to see is ever delayed.
const MUDDAT_BATH: Duration = Duration::from_millis(120);

// ---------------------------------------------------------------------------
// The vocabulary the screen reads
// ---------------------------------------------------------------------------

/// The five stages the interface draws, in the order it draws them.
///
/// Five, not the pipeline's seven. The probe is folded into extraction — it is
/// milliseconds and the verdict already reported what it concluded — and the
/// compile and the seal are one row, because signing is the tail of compiling
/// and a row that appears for a hundred milliseconds is noise.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum MarhalatTilqaiHie {
    /// Reading the game's strings.
    Istikhraj,
    /// Machine translation.
    Tarjama,
    /// Fonts and sizes.
    Takhtit,
    /// Compiling and sealing the patch.
    Tajmee,
    /// Installing it.
    Tathbeet,
}

impl MarhalatTilqaiHie {
    /// The five, in run order.
    const KUL: [Self; 5] = [
        Self::Istikhraj,
        Self::Tarjama,
        Self::Takhtit,
        Self::Tajmee,
        Self::Tathbeet,
    ];
}

/// Where one stage stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HalatMarhalaHie {
    /// Not started.
    Muntazira,
    /// In progress.
    Jariya,
    /// Finished.
    Tammat,
    /// Stopped by a failure.
    Fashilat,
}

/// Where the run as a whole stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum WadTilqaiHie {
    /// Running right now.
    Jariya,
    /// Interrupted and resumable: the window closed, the machine slept.
    Mutawaqqifa,
    /// Installed and ready to play.
    Jahiz,
    /// Stopped because the user asked.
    Mulgha,
    /// Stopped because something failed.
    Fashal,
    /// Nothing readable in the files; the route is runtime capture.
    Iltiqat,
}

/// Which of the three routes a game takes, decided before anything runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MasarTilqaiHie {
    /// Taarib can do this end to end.
    Tilqai,
    /// The game must be played once with capture on before anything can be read.
    Iltiqat,
    /// The safety layer refuses this game outright.
    Marfud,
}

/// A failure, in the three fields every failure in this product carries.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhataTilqaiHie {
    /// The permanent code.
    pub ramz: String,
    /// The sentence a user reads, in Arabic.
    pub arabi: String,
    /// The same sentence in English.
    pub injilizi: String,
}

/// How far one stage has got.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqaddumMarhalaHie {
    /// Which stage.
    pub marhala: MarhalatTilqaiHie,
    /// Where it stands.
    pub hala: HalatMarhalaHie,
    /// Units finished.
    pub tamma: u32,
    /// How many units there are, or `None` when the stage genuinely has no
    /// denominator — which the interface draws as words rather than as a bar.
    pub majmu: Option<u32>,
    /// The machine's own detail: a container, a batch, a font.
    pub tafsil: Option<String>,
}

/// One group of things extraction refused, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrRafdHie {
    /// What was refused, named the way the extractor named it.
    pub mawdu: String,
    /// Why, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// How many containers the group accounts for, or `None` when it is one.
    pub adad: Option<u32>,
}

/// What extraction read, what it skipped, and why it skipped it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerQiraHie {
    /// Containers read.
    pub maqru: u32,
    /// Containers refused.
    pub matruk: u32,
    /// Strings a player is believed to see.
    pub nusus: u32,
    /// The refusals, grouped by reason.
    pub asbab: Vec<SatrRafdHie>,
}

/// Money, against the ceiling it may not cross.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TakalifHie {
    /// Spent so far, in whole currency units.
    pub munfaq: f64,
    /// The ceiling this run honours; zero when there is none.
    pub saqf: f64,
    /// ISO 4217, for the interface's own number formatter.
    pub umla: String,
}

/// The verdict: everything the user needs in order to say yes or no, once.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HukmTilqaiHie {
    /// The game's identity.
    pub muarrif: String,
    /// Its name.
    pub ism: String,
    /// Which of the three routes it takes.
    pub masar: MasarTilqaiHie,
    /// The tier, 1 to 3.
    pub tabaqa_raqm: u8,
    /// The tier's name in Arabic.
    pub tabaqa_arabi: String,
    /// The same name in English.
    pub tabaqa_injilizi: String,
    /// Why that tier and not a better one, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// Everything that will not work, named specifically, in Arabic.
    pub hudud_arabi: Vec<String>,
    /// The same list in English.
    pub hudud_injilizi: Vec<String>,
    /// Roughly how many strings are involved, or `None` when only a run can say.
    pub nusus_taqribi: Option<u32>,
    /// Whether the run will require the multiplayer acknowledgement, so the
    /// interface asks for it *before* the button rather than after the money.
    ///
    /// Read off the launcher's own catalogue entry, which the library scan
    /// already stored — the only source of this fact a verdict may consult,
    /// because deciding it properly means walking the game directory and this
    /// command is answered on mount. [`ibda`] decides it again from the walk and
    /// refuses at the door when the two disagree, so a `false` here is "the
    /// launcher did not say so", never "you will not be asked".
    pub yalzam_iqrar_shabaka: bool,
    /// What it has cost so far and what it may cost.
    pub takalif: TakalifHie,
    /// The revocation list as this machine holds it right now, and where it
    /// stands, so the screen can say before the button whether the registry
    /// has confirmed it. The run refreshes it again before spending.
    pub sahb: HalatSahbHie,
    /// The cover's absolute path, when the artwork cache holds one.
    pub ghilaf: Option<String>,
}

/// One run, as it stands at one instant.
///
/// A whole snapshot rather than a delta, which is what makes the screen correct
/// after a remount: a subscriber that joined half way through gets the complete
/// picture on the next report instead of increments it has no base to apply.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LaqtatTilqaiHie {
    /// The game's identity.
    pub muarrif: String,
    /// This run's own identity, so a late report from a previous run is dropped.
    pub tashghila: String,
    /// Where the run stands.
    pub wad: WadTilqaiHie,
    /// The stage in progress, or the one a stopped run would resume at.
    pub marhala: Option<MarhalatTilqaiHie>,
    /// Always all five, always in run order.
    pub marahil: Vec<TaqaddumMarhalaHie>,
    /// What extraction read and refused, once it has reported.
    pub qira: Option<TaqreerQiraHie>,
    /// What the run has spent.
    pub takalif: TakalifHie,
    /// The failure, when the run ended in one.
    pub khata: Option<KhataTilqaiHie>,
    /// Whether anything has been written into the game yet.
    pub muthabbata: bool,
    /// The revocation list the run's gate checks against, and where it stood
    /// the last time this run read it: as the door found the cache, then as
    /// the run's own refresh left it. Absent only for a snapshot read back from
    /// disk, which records stages and not what the registry said.
    pub sahb: Option<HalatSahbHie>,
    /// When the run last moved, RFC 3339.
    pub waqt: String,
}

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// One live run's mutable half: the snapshot, and when it was last published.
#[derive(Debug)]
struct HalatHay {
    /// The last snapshot built for this run.
    laqta: LaqtatTilqaiHie,
    /// When it was last put on the wire, for the publish throttle.
    akhir_bath: Option<Instant>,
}

/// One run this process started, whether it is still going or has finished.
///
/// A finished run is kept rather than removed, because the snapshot it ended on
/// is the freshest answer this process has for the game — fresher than the
/// journal on disk, which records stages and not why the run stopped.
#[derive(Debug)]
struct MashwarHay {
    /// The run's identity.
    tashghila: MashwarId,
    /// Its stop button.
    miqbad: MiqbadIlgha,
    /// Its snapshot and publish clock.
    hala: parking_lot::Mutex<HalatHay>,
}

/// The runs this process knows about, one at most per game.
///
/// `parking_lot`, not `tokio`: the guard is taken, the map is read or written,
/// and it is released before anything blocks — there is no poisoning to recover
/// from and no `await` held across it. One run per game rather than one run
/// overall, because two games share nothing: not a run directory, not a project,
/// not a game to install into. What they do share is a cost ceiling, and the
/// ceiling is enforced per request by `taarib_tarjama::dufaat` rather than here.
#[derive(Debug, Default)]
pub struct MashawirTilqai(parking_lot::Mutex<HashMap<LubaId, Arc<MashwarHay>>>);

impl MashawirTilqai {
    /// The run recorded for one game, if this process started one.
    fn wahid(&self, id: LubaId) -> Option<Arc<MashwarHay>> {
        self.0.lock().get(&id).map(Arc::clone)
    }

    /// Records a run, refusing when the one already there is still going.
    fn sajjil(&self, id: LubaId, hay: Arc<MashwarHay>) -> bool {
        let mut kharita = self.0.lock();
        if let Some(sabiq) = kharita.get(&id)
            && sabiq.hala.lock().laqta.wad == WadTilqaiHie::Jariya
        {
            return false;
        }
        let _ = kharita.insert(id, hay);
        true
    }
}

// ---------------------------------------------------------------------------
// The verdict
// ---------------------------------------------------------------------------

/// What automatic arabization would do to one game, decided without running it.
///
/// Everything here is either cached or a directory read: the capability report
/// comes from the store when this build already wrote one, the string count and
/// the spend come from a previous run's journal, and the cover comes from the
/// artwork cache. Nothing opens a container and nothing reaches a provider, so
/// the screen can ask for this the moment it mounts.
///
/// # Errors
///
/// [`Khata`] when the identity is not a game, when the game is not in the store,
/// or when the probe itself will not run on a game that is no longer on disk.
#[tauri::command]
#[specta::specta]
pub fn hukm_tilqai(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<HukmTilqaiHie, Khata> {
    // One of the entry points that starts the background refresh, so a session
    // spent entirely on this screen still keeps the revocation list current.
    // In the wrapper rather than in `hukm`, so nothing driving that function
    // directly — every test in this file — starts a loop.
    crate::tathbeet_awamir::dhamin_mujaddid_sahb(&masarat, &idadat);
    hukm(muarrif, &masarat, &makhzan, &idadat)
}

/// The verdict, as everything but the IPC boundary sees it.
///
/// The four commands in this module are each a two-line adapter over a function
/// like this one. `tauri::State` and `tauri::Window` cannot be built outside a
/// running application, so a body written inside a command is a body nothing can
/// drive; written out here, the whole surface can be exercised against a real
/// store, a real run directory and a real provider.
///
/// # Errors
///
/// As [`hukm_tilqai`].
pub(crate) fn hukm(
    muarrif: String,
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &MakhzanIdadat,
) -> Natija<HukmTilqaiHie> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(makhzan, id)?;
    let simat = simat_luba(makhzan, id)?;
    let taqreer = taqreer_luba(makhzan, &luba, &simat, false)?;
    let hali = idadat.hali();

    let mujallad = jidhr_mashawir(masarat, id);
    let akhir = akhir_mashwar(&mujallad);
    let (nusus_taqribi, munfaq) = akhir.as_ref().map_or((None, 0), |sijill| {
        (adad_zahir_min_sijill(sijill), munfaq_min_sijill(sijill))
    });

    let mut hudud_arabi: Vec<String> = taqreer
        .hudud
        .iter()
        .map(|hadd| hadd.arabi.clone())
        .collect();
    let mut hudud_injilizi: Vec<String> = taqreer
        .hudud
        .iter()
        .map(|hadd| hadd.injilizi.clone())
        .collect();
    // What this build cannot deliver yet is not a limitation of the game, and
    // the capability report keeps the two apart for exactly that reason. The
    // verdict has one list, and a user deciding whether to press the button
    // needs to be told either way — so it is appended, not dropped.
    if let Some(naqs) = &taqreer.naqs {
        hudud_arabi.push(naqs.arabi.clone());
        hudud_injilizi.push(naqs.injilizi.clone());
    }
    // Which provider the run would use, in the settings crate's own sentence —
    // the same one the workshop shows before a batch, so the two surfaces
    // cannot disagree about one machine. It covers the two states a local
    // sentence used to miss: the built-in free provider standing in for an
    // empty or switched-off list, and a chosen default switched off while
    // another provider is silently the one billed. The one state that needs no
    // sentence is the one doing what it says.
    let tarif = muzawwid_muntakhab(&hali);
    let halat_muzawwidin = hali.muzawwidun.hala();
    if halat_muzawwidin != HalatMuzawwidin::Mukhtar {
        hudud_arabi.push(halat_muzawwidin.arabi().to_owned());
        hudud_injilizi.push(halat_muzawwidin.injilizi().to_owned());
    }
    let saqf = tarif
        .mizaniya
        .filter(|mablagh| mablagh.is_finite() && *mablagh > 0.0)
        .unwrap_or(0.0);
    // The same condition `jahhiz` refuses on, reported here rather than only
    // there. The screen's own cost sentence reads "the run stops at the ceiling
    // and never crosses it" and is handed `saqf` — which is zero for exactly
    // this configuration, so without this line the verdict promises a ceiling
    // that does not exist and the button then refuses with a code the reader had
    // no warning of. `nano_min_dolar` rejects the same values this filter drops,
    // so the two commands cannot disagree about which budgets count.
    if yatqada_ajran(tarif.naw) && saqf <= 0.0 {
        let ism = tarif.muarrif;
        hudud_arabi.push(format!(
            "المزوّد «{ism}» يتقاضى أجرًا على الترجمة ولم يُضبط له سقف إنفاق، والتعريب \
             التلقائي لا يبدأ بدون سقف. اضبط «الميزانية» لهذا المزوّد في الإعدادات ← \
             المزوّدون، وهي لكلّ جولة لا لكلّ شهر."
        ));
        hudud_injilizi.push(format!(
            "{ism} charges for translation and has no spend ceiling set, and an automatic run \
             will not start without one. Set this provider's budget in Settings, Providers; it \
             is per run, not per month."
        ));
    }

    // The registry's word on what is withdrawn, as the cache holds it. The run
    // refreshes it before spending; a registry that is answering and
    // withholding its list is named here so the button is not the first to
    // say so.
    let sahb = iqra_qaimat_sahb(masarat, &hali)?;
    if let Some(rafd) = sahb.rafd() {
        hudud_arabi.push(rafd.arabi());
        hudud_injilizi.push(rafd.injilizi());
    }

    let hukm = HukmTilqaiHie {
        muarrif: id.to_string(),
        ism: luba.ism.clone(),
        masar: masar_hukm(&taqreer),
        tabaqa_raqm: taqreer.tabaqa.raqm(),
        tabaqa_arabi: taqreer.tabaqa.ism_arabi().to_owned(),
        tabaqa_injilizi: taqreer.tabaqa.ism_injilizi().to_owned(),
        sabab_arabi: taqreer.sabab_arabi.clone(),
        sabab_injilizi: taqreer.sabab_injilizi.clone(),
        hudud_arabi,
        hudud_injilizi,
        nusus_taqribi,
        yalzam_iqrar_shabaka: yalzam_iqrar_shabaka(&simat),
        sahb: sahb_hie(&sahb),
        takalif: TakalifHie {
            munfaq: dolar(munfaq),
            saqf,
            umla: UMLA.to_owned(),
        },
        ghilaf: ghilaf_luba(masarat, &luba),
    };
    // Both exclusions are applied last, and to the finished verdict rather than
    // inside it, so the tier, the reasons and the cover are exactly what the
    // rest of the product shows for this game and only the offer changes.
    //
    // Three refusals can hold at once and they are applied in the order a user
    // needs to hear them, which is the order [`ibda`] raises them in — the two
    // commands answering the same game with two different reasons is the defect
    // this ordering exists to prevent:
    //
    // 1. the safety layer's, already in `masar_hukm` above, because an
    //    anti-cheat association puts an account at stake and no release lifts
    //    it;
    // 2. the publisher's own Arabic, because it is permanent too and it is the
    //    one a user can act on — by deciding the shipped Arabic really is
    //    unusable and saying so in Settings;
    // 3. readiness last, because it is the only one of the three an update
    //    lifts, and telling somebody to give up on a game Taarib will patch
    //    next release is the wrong sentence to leave them with.
    //
    // Both helpers defer to a refusal that is already standing and add their own
    // fact to the limits instead, so nothing is hidden by being outranked.
    let hukm = tabbiq_lugha_rasmiya(
        hukm,
        crate::luba_awamir::hukm_mukhazzan(id).as_ref(),
        hali.istibdal_lugha_rasmiya,
    );
    let mut hukm = tabbiq_jahiziya(hukm, &taqreer);
    // A question about a run that is not offered is a control on a screen with
    // no button, and an acknowledgement box beside a refusal reads as an
    // invitation for the same reason `nusus_taqribi` is dropped above.
    if hukm.masar == MasarTilqaiHie::Marfud {
        hukm.yalzam_iqrar_shabaka = false;
    }
    Ok(hukm)
}

/// Whether the launcher's own catalogue already says this game is played with
/// other people, so the verdict can ask before the button rather than after.
///
/// Both hints count, not only the online one, because the gate this warns about
/// is `taarib_aman::kashf_shabaka::mutaaddid`, and that answers on *any*
/// evidence — a shared-screen title is refused by it exactly as an online one
/// is. Warning on the narrower set would leave the wider refusal unannounced,
/// which is the defect being fixed rather than a smaller version of it.
fn yalzam_iqrar_shabaka(simat: &[SimatLuba]) -> bool {
    simat
        .iter()
        .any(|sima| matches!(sima, SimatLuba::JamaiOnline | SimatLuba::JamaiMahalli))
}

/// Takes the offer away from a game whose engine this build cannot patch yet.
///
/// The refusal a user meets before they press the button, and the refusal
/// [`ibda`] raises if they press it anyway, are one sentence written in one
/// place — `taarib_tilqai::naqs_jahiziya`, out of the capability report's own
/// words. Nothing here decides *whether* an engine is ready; it only decides
/// what the verdict looks like once the pipeline has said it is not.
///
/// Shaped on [`tabbiq_lugha_rasmiya`] so the interface has one refusal to draw
/// rather than two: the route becomes [`MasarTilqaiHie::Marfud`], the reason
/// becomes the refusal, and the button is simply not offered.
///
/// It overrides neither of the two refusals that outrank it. An anti-cheat
/// association is handled by the capability report itself — `naqs_jahiziya`
/// answers [`None`] for a refused report — and a publisher's own Arabic is
/// handled here, by the standing-refusal check below. Both are permanent facts
/// about the game; this one is a temporary fact about Taarib, and overwriting a
/// permanent reason with a temporary one would tell a user to wait for a release
/// that will not change their answer. The gap is still stated: the report's own
/// `naqs` sentence is already in the limits list either way.
fn tabbiq_jahiziya(mut hukm: HukmTilqaiHie, taqreer: &TaqreerImkaniyat) -> HukmTilqaiHie {
    let Some(sabab) = naqs_jahiziya(taqreer) else {
        return hukm;
    };
    if hukm.masar == MasarTilqaiHie::Marfud {
        return hukm;
    }
    hukm.masar = MasarTilqaiHie::Marfud;
    hukm.sabab_arabi = sabab.arabi;
    hukm.sabab_injilizi = sabab.injilizi;
    // Whatever a previous run counted was counted for a game this build no
    // longer offers, and printing it beside a refusal reads as an invitation.
    hukm.nusus_taqribi = None;
    hukm
}

/// The one currency every provider in this build meters in.
const UMLA: &str = "USD";

/// Takes the offer away from a game whose publisher already ships Arabic.
///
/// The verdict is the remembered one — whatever the library scan's shallow pass
/// or the game screen's deep pass last concluded — and this path never computes
/// one of its own, because a container decode is not something a screen may pay
/// for on mount. `None` means neither pass has run: nobody looked, and a game
/// nobody looked at is offered rather than refused, exactly as a low-confidence
/// [`taarib_mustalahat::luba::HalatLughaRasmiya::Ghaib`] is.
///
/// What is being protected is not a badge. That Arabic was written by paid
/// humans, reviewed by native speakers and tested in context; machine output
/// written over it is not discovered until a player is inside the game reading
/// it, at which point the original is gone. So this refuses rather than warns —
/// unless the user has turned the exclusion off themselves, in which case the
/// run is offered *and* told, in the verdict's own words, what it would replace.
fn tabbiq_lugha_rasmiya(
    mut hukm: HukmTilqaiHie,
    rasmiya: Option<&taarib_mustalahat::luba::HukmLughaRasmiya>,
    istibdal: bool,
) -> HukmTilqaiHie {
    let Some(rasmiya) = rasmiya.filter(|rasmiya| rasmiya.hala().yatakallam_arabi()) else {
        return hukm;
    };
    let hala = rasmiya.hala();
    if istibdal {
        hukm.hudud_arabi.insert(
            0,
            format!(
                "هذه اللعبة تحمل عربية رسمية من الناشر ({})، وظهرت هنا لأنّك فعّلت استبدال \
                 اللغة الرسمية في الإعدادات. الجولة ستكتب ترجمة آلية غير مراجَعة فوق تلك العربية.",
                hala.ism_arabi()
            ),
        );
        hukm.hudud_injilizi.insert(
            0,
            format!(
                "This game already ships official Arabic ({}), and it is offered here only \
                 because you turned on replacing the official language in Settings. The run \
                 writes unreviewed machine translation over that Arabic.",
                hala.ism_injilizi()
            ),
        );
        return hukm;
    }

    // A game the safety layer already refused keeps that refusal's sentence: it
    // is the more serious of the two and replacing it would hide it.
    if hukm.masar == MasarTilqaiHie::Marfud {
        hukm.hudud_arabi.insert(
            0,
            format!(
                "الناشر يشحن عربية رسمية مع هذه اللعبة ({}).",
                hala.ism_arabi()
            ),
        );
        hukm.hudud_injilizi.insert(
            0,
            format!(
                "The publisher already ships Arabic with this game ({}).",
                hala.ism_injilizi()
            ),
        );
        return hukm;
    }

    hukm.masar = MasarTilqaiHie::Marfud;
    hukm.sabab_arabi = format!(
        "الناشر يشحن عربية رسمية مع هذه اللعبة ({}). الترجمة الآلية فوقها تستبدل عملًا بشريًّا \
         مدفوعًا راجعه ناطقون بالعربية واختُبر داخل اللعبة، ولا يكتشف اللاعب ذلك إلّا وهو يقرأ. \
         لهذا لا يُعرض التعريب التلقائي هنا. إن كانت العربية الرسمية غير صالحة فعلًا، فعّل \
         «استبدال اللغة الرسمية» في الإعدادات.",
        hala.ism_arabi()
    );
    hukm.sabab_injilizi = format!(
        "The publisher already ships Arabic with this game ({}). A machine translation written \
         over it replaces paid human work that native speakers reviewed and that was tested \
         inside the game, and a player does not find out until they are reading it. Automatic \
         arabization is therefore not offered here. If that official Arabic is genuinely \
         unusable, turn on replacing the official language in Settings.",
        hala.ism_injilizi()
    );
    // Whatever a previous run counted was counted for a game this build no
    // longer offers, and printing it beside a refusal reads as an invitation.
    hukm.nusus_taqribi = None;
    hukm
}

/// Whether a game's own publisher already ships Arabic, per the remembered
/// verdict, and the user has not turned the exclusion off.
fn mahmiya_bil_lugha(
    id: LubaId,
    hali: &Idadat,
) -> Option<taarib_mustalahat::luba::HalatLughaRasmiya> {
    if hali.istibdal_lugha_rasmiya {
        return None;
    }
    crate::luba_awamir::hukm_mukhazzan(id)
        .map(|rasmiya| rasmiya.hala())
        .filter(|hala| hala.yatakallam_arabi())
}

/// Whether a provider of this kind charges for translation.
///
/// The verdict may not answer this the way `jahhiz` does — by building the
/// provider and reading `qudrat().taklifa` — because building one opens the
/// keychain, and a screen may not unlock a credential on mount. It is decided
/// from the kind instead, and the two agree because `bin_muzawwid` is what makes
/// them agree: [`NawMuzawwid::Mahalli`] is built through
/// `MuzawwidMuwafiqOpenAI::mahalli`, which forces a free meter whatever the
/// configuration claimed, [`NawMuzawwid::GoogleMajjani`] is built on
/// `TakalifJarya::majani` with no account behind it to bill, and every other
/// arm it builds is metered.
const fn yatqada_ajran(naw: NawMuzawwid) -> bool {
    !matches!(naw, NawMuzawwid::Mahalli | NawMuzawwid::GoogleMajjani)
}

/// Which of the three routes a game takes.
const fn masar_hukm(taqreer: &TaqreerImkaniyat) -> MasarTilqaiHie {
    if taqreer.marfuda {
        return MasarTilqaiHie::Marfud;
    }
    if yuqra_sakinan(taqreer.muharrik.aila) {
        MasarTilqaiHie::Tilqai
    } else {
        MasarTilqaiHie::Iltiqat
    }
}

/// Whether this build has a static string reader for an engine family at all.
///
/// The same three families `taarib_tilqai::istikhraj` dispatches on. A game
/// outside them produces an empty table and the capture route, and the verdict
/// says so before the run rather than after it — which is the difference between
/// offering a path that works and reporting a failure.
const fn yuqra_sakinan(aila: AilatMuharrik) -> bool {
    matches!(
        aila,
        AilatMuharrik::Unity | AilatMuharrik::Unreal | AilatMuharrik::Godot
    )
}

/// The cover's absolute path, when the artwork cache still holds one.
fn ghilaf_luba(masarat: &Masarat, luba: &Luba) -> Option<String> {
    let khazina = crate::suwar_awamir::khazina(masarat).ok()?;
    crate::suwar_awamir::ghilaf_makhzun(&khazina, &luba.suwar).ghilaf
}

// ---------------------------------------------------------------------------
// The snapshot
// ---------------------------------------------------------------------------

/// The unfinished or finished run for one game, or nothing.
///
/// Answered from this process's own memory when it started the run, and from the
/// run directory otherwise — which is what makes a run survive the application
/// closing. A game with no run directory answers `None`, and that is the ordinary
/// case rather than a failure.
///
/// # Errors
///
/// [`Khata`] when the identity is not a game.
#[tauri::command]
#[specta::specta]
pub fn laqtat_tilqai(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    mashawir: tauri::State<'_, MashawirTilqai>,
) -> Result<Option<LaqtatTilqaiHie>, Khata> {
    laqta(muarrif, &masarat, &idadat, &mashawir)
}

/// The run for one game, as everything but the IPC boundary sees it.
///
/// # Errors
///
/// As [`laqtat_tilqai`].
pub(crate) fn laqta(
    muarrif: String,
    masarat: &Masarat,
    idadat: &MakhzanIdadat,
    mashawir: &MashawirTilqai,
) -> Natija<Option<LaqtatTilqaiHie>> {
    let id = huwiya(muarrif)?;
    if let Some(hay) = mashawir.wahid(id) {
        return Ok(Some(hay.hala.lock().laqta.clone()));
    }
    Ok(laqta_min_qurs(masarat, &idadat.hali(), id))
}

/// The newest run on disk for one game, as a snapshot.
fn laqta_min_qurs(masarat: &Masarat, hali: &Idadat, id: LubaId) -> Option<LaqtatTilqaiHie> {
    let jidhr = jidhr_mashawir(masarat, id);
    let mawjuz = ijrud(&jidhr).into_iter().next()?;
    let sijill = SijillMashwar::iftah(&mawjuz.mujallad).ok()?;

    let mut marahil = marahil_farigha();
    for satr in sijill.marahil() {
        tabbiq_qayd(&mut marahil, &satr.qayd);
    }
    // The stage a resume would enter: the first of the five that no journal line
    // closed. The translation stage is deliberately included even when it closed
    // once, because its own per-string journal is what decides whether anything
    // is left to buy — but a run whose install closed has nothing left at all.
    let tammat = sijill.tamma(MarhalaTilqai::Tathbeet);
    let waqifa = marahil
        .iter()
        .find(|saf| saf.hala != HalatMarhalaHie::Tammat)
        .map(|saf| saf.marhala);
    // The stage the run stopped in stays `Muntazira`, which is what the journal
    // actually establishes: it never closed. `Jariya` would claim it is running
    // when nothing is, and `Fashilat` would claim it failed when all the journal
    // records is a run that did not get back to writing a line.
    for saf in &mut marahil {
        match (tammat, waqifa) {
            (true, _) | (false, None) => saf.hala = HalatMarhalaHie::Tammat,
            (false, Some(waqifa)) => {
                if saf.marhala > waqifa {
                    saf.hala = HalatMarhalaHie::Muntazira;
                }
            },
        }
    }

    let munfaq = munfaq_min_sijill(&sijill);
    Some(LaqtatTilqaiHie {
        muarrif: id.to_string(),
        tashghila: mawjuz.id.to_string(),
        wad: if tammat {
            WadTilqaiHie::Jahiz
        } else {
            WadTilqaiHie::Mutawaqqifa
        },
        marhala: if tammat { None } else { waqifa },
        marahil,
        qira: qira_min_sijill(&sijill, &mawjuz.mujallad),
        takalif: takalif_hie(munfaq, hali),
        khata: None,
        muthabbata: tammat,
        // The journal records stages, not what the registry said; a resumed
        // run reads the list again at its door and says so then.
        sahb: None,
        waqt: mawjuz.waqt,
    })
}

/// The five stages with nothing reported about any of them.
fn marahil_farigha() -> Vec<TaqaddumMarhalaHie> {
    MarhalatTilqaiHie::KUL
        .into_iter()
        .map(|marhala| TaqaddumMarhalaHie {
            marhala,
            hala: HalatMarhalaHie::Muntazira,
            tamma: 0,
            majmu: None,
            tafsil: None,
        })
        .collect()
}

/// Folds one journal record into the five rows.
fn tabbiq_qayd(marahil: &mut [TaqaddumMarhalaHie], qayd: &QaydMarhala) {
    // Every arm names its own counted unit in `tafsil`, because the numbers on
    // their own would be five different things all rendered as "n of n".
    let (marhala, tamma, majmu, tafsil) = match qayd {
        // The probe is not one of the five rows: folding its detector count into
        // extraction would fill that row's bar and then empty it again when the
        // real extraction starts counting containers.
        QaydMarhala::Fahs { .. } => return,
        QaydMarhala::Istikhraj {
            adad,
            adad_zahir,
            maqrua,
            marfuda,
            ..
        } => (
            MarhalatTilqaiHie::Istikhraj,
            *maqrua,
            Some(*maqrua),
            format!(
                "{maqrua} container(s) read, {marfuda} refused, {adad} distinct string(s), \
                 {adad_zahir} of them a player sees"
            ),
        ),
        QaydMarhala::Tarjama {
            mutabbaqa, fashila, ..
        } => (
            MarhalatTilqaiHie::Tarjama,
            *mutabbaqa,
            // The journal records what the fold wrote, never how many strings
            // were eligible, so a resumed run has no denominator to divide by.
            None,
            format!("{mutabbaqa} string(s) translated, {fashila} failed"),
        ),
        QaydMarhala::Takhtit { khutut, azwaj, .. } => {
            let adad = raqm_u32(khutut.len().try_into().unwrap_or(u64::MAX));
            (
                MarhalatTilqaiHie::Takhtit,
                u64::from(adad),
                Some(u64::from(adad)),
                format!("{adad} font(s) validated, {azwaj} layout(s) to compute"),
            )
        },
        QaydMarhala::Tarqee { nusus, safahat, .. } => (
            MarhalatTilqaiHie::Tajmee,
            *nusus,
            Some(*nusus),
            format!("{nusus} string(s) compiled into {safahat} atlas page(s)"),
        ),
        QaydMarhala::Khatm { basma, .. } => (
            MarhalatTilqaiHie::Tajmee,
            1,
            Some(1),
            format!("sealed: {basma}"),
        ),
        QaydMarhala::Tathbeet { adad_muhtawa, .. } => (
            MarhalatTilqaiHie::Tathbeet,
            *adad_muhtawa,
            Some(*adad_muhtawa),
            format!("{adad_muhtawa} file(s) placed in the game"),
        ),
    };
    // The seal overwrites the compile's own row, so its detail must not throw
    // away what the compile reported: they are one row and one sentence.
    let sabiq = marahil
        .iter()
        .find(|saf| saf.marhala == marhala)
        .and_then(|saf| saf.tafsil.clone());
    for saf in marahil.iter_mut().filter(|saf| saf.marhala == marhala) {
        saf.hala = HalatMarhalaHie::Tammat;
        saf.tamma = raqm_u32(tamma);
        saf.majmu = majmu.map(raqm_u32);
        saf.tafsil = Some(match (&sabiq, marhala) {
            (Some(sabiq), MarhalatTilqaiHie::Tajmee) => format!("{sabiq}; {tafsil}"),
            _ => tafsil.clone(),
        });
    }
}

/// What extraction read, rebuilt from a journal record and the stored table.
fn qira_min_sijill(sijill: &SijillMashwar, mujallad: &Path) -> Option<TaqreerQiraHie> {
    let QaydMarhala::Istikhraj {
        adad_zahir,
        maqrua,
        marfuda,
        ..
    } = sijill.qayd(MarhalaTilqai::Istikhraj)?
    else {
        return None;
    };
    Some(TaqreerQiraHie {
        maqru: raqm_u32(*maqrua),
        matruk: raqm_u32(*marfuda),
        nusus: raqm_u32(*adad_zahir),
        asbab: asbab_rafd(mujallad),
    })
}

/// The visible string count a previous run recorded, when one did.
fn adad_zahir_min_sijill(sijill: &SijillMashwar) -> Option<u32> {
    match sijill.qayd(MarhalaTilqai::Istikhraj)? {
        QaydMarhala::Istikhraj { adad_zahir, .. } => Some(raqm_u32(*adad_zahir)),
        _ => None,
    }
}

/// What a previous run of this game has already spent, in nano-dollars.
fn munfaq_min_sijill(sijill: &SijillMashwar) -> u64 {
    match sijill.qayd(MarhalaTilqai::Tarjama) {
        Some(QaydMarhala::Tarjama { munfaq, .. }) => *munfaq,
        _ => 0,
    }
}

/// The newest run's journal for one game, when there is one.
fn akhir_mashwar(jidhr: &Path) -> Option<SijillMashwar> {
    let mawjuz = ijrud(jidhr).into_iter().next()?;
    SijillMashwar::iftah(&mawjuz.mujallad).ok()
}

/// The refusal groups a finished extraction left in the run's stored table.
///
/// Only the refusal report is deserialized: the same file holds the whole string
/// table, and building a hundred thousand rows in order to render four lines is
/// a cost this is asked to pay every time the screen opens.
fn asbab_rafd(mujallad: &Path) -> Vec<SatrRafdHie> {
    /// The stored table, read for its refusal report and nothing else.
    #[derive(serde::Deserialize)]
    struct RafdFaqat {
        /// What was read and what was refused.
        rafd: TaqreerRafd,
    }

    let masar = mujallad.join(MUJALLAD_MASHRU).join(MALAF_JADWAL);
    let Ok(malaf) = std::fs::File::open(&masar) else {
        return Vec::new();
    };
    let makhzun: RafdFaqat = match serde_json::from_reader(std::io::BufReader::new(malaf)) {
        Ok(makhzun) => makhzun,
        Err(sabab) => {
            tracing::warn!(
                masar = %masar.display(),
                %sabab,
                "the stored extraction table would not read; its refusals are not shown"
            );
            return Vec::new();
        },
    };
    makhzun
        .rafd
        .majmua()
        .into_values()
        .filter_map(|majmua| {
            let awwal = majmua.first()?;
            Some(SatrRafdHie {
                mawdu: awwal.hawiya.clone(),
                sabab_arabi: awwal.sabab.arabi(),
                sabab_injilizi: awwal.sabab.injilizi(),
                // Null for a group of one, which is what the interface reads as
                // "this is one thing, not a count".
                adad: (majmua.len() > 1)
                    .then(|| raqm_u32(majmua.len().try_into().unwrap_or(u64::MAX))),
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Starting a run
// ---------------------------------------------------------------------------

/// Starts an automatic run for one game, or resumes the unfinished one.
///
/// Answers the moment the run is on its own task, with the snapshot it starts
/// from; everything after that arrives on [`ISM_HADATH_TILQAI`]. `istinaf` picks
/// up the newest run directory instead of opening a new one, which is what stops
/// a resumed run from paying a second time for strings it already bought.
///
/// `iqrar_shabaka` is the user's own answer to the multiplayer warning, obtained
/// the way [`crate::tathbeet_awamir::thabbit_ruqaa`] obtains it: a key in the
/// IPC payload, filled from a control the person ticked. Nothing in this process
/// may assert it on their behalf — the risk it acknowledges is a permanent ban
/// on their account — so an unticked box costs a refusal here and never a run.
///
/// # Errors
///
/// [`KhataTilqaiAmr::MashwarJari`] when a run for this game is already going,
/// [`KhataTilqaiAmr::HimayaMuktashafa`] when the anti-cheat scan finds evidence
/// on disk or in Steam's catalogue, [`KhataTilqaiAmr::FahsHimayaLamYajri`] when
/// that scan could not read the catalogue it needs,
/// [`KhataTilqaiAmr::ShabakaBilaIqrar`] when the game is multiplayer and
/// `iqrar_shabaka` is false, [`KhataTilqaiAmr::LughaRasmiya`] when the publisher
/// already ships Arabic, `taarib_tilqai::KhataTilqai::MuharrikGhayrJahiz` when
/// this build has no working in-game half for the detected engine and
/// `taarib_tilqai::KhataTilqai::TabaqaGhayrMadauma` when the safety layer
/// refuses the game — both from the same gate the verdict reports,
/// [`crate::luba_awamir::KhataLuba::JidhrSteamMajhul`] when the game is a Steam
/// game and Steam's own root cannot be found, so the install gate this run ends
/// at could not read the catalogue VAC is declared in,
/// [`crate::warsha_awamir::KhataWarshaAmr::LaItimad`] when the elected
/// provider's key is not in the keychain, [`KhataTilqaiAmr::LaKhattArabi`] when no font on this machine can
/// carry Arabic, [`KhataTilqaiAmr::LaIstinaf`] when a resume was asked for and
/// there is nothing to resume, and whatever the store, the keychain and the
/// acknowledgement record raise.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the resume flag and the acknowledgement are separate keys in the IPC payload, \
              shaped on `thabbit_ruqaa`, which the interface already fills the same way; \
              folding them into one struct would change that contract and make the automatic \
              path's payload differ from the one-click path's for no gain"
)]
pub fn ibda_tilqai(
    nafidha: tauri::Window,
    muarrif: String,
    istinaf: bool,
    iqrar_shabaka: bool,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    mashawir: tauri::State<'_, MashawirTilqai>,
) -> Result<LaqtatTilqaiHie, Khata> {
    ibda(
        Arc::new(move |laqta: &LaqtatTilqaiHie| {
            let _ = nafidha.emit(ISM_HADATH_TILQAI, laqta);
        }),
        muarrif,
        istinaf,
        iqrar_shabaka,
        &masarat,
        &makhzan,
        &idadat,
        &mashawir,
    )
}

/// Where a snapshot goes once it has been built.
///
/// A closure rather than the window itself, for the reason
/// `taarib_tilqai::MukhbirTaqaddum` is one: the run has no business knowing that
/// a webview exists, and a run that only a webview can receive is a run nothing
/// can be driven against.
pub(crate) type MudheeLaqta = Arc<dyn Fn(&LaqtatTilqaiHie) + Send + Sync>;

/// Starts a run, as everything but the IPC boundary sees it.
///
/// # Errors
///
/// As [`ibda_tilqai`].
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the two mirror `ibda_tilqai`'s payload exactly, which is the point of this \
              function: it is the same call with the IPC boundary taken off"
)]
pub(crate) fn ibda(
    mudhee: MudheeLaqta,
    muarrif: String,
    istinaf: bool,
    iqrar_shabaka: bool,
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &Arc<MakhzanIdadat>,
    mashawir: &MashawirTilqai,
) -> Natija<LaqtatTilqaiHie> {
    let id = huwiya(muarrif)?;
    if mashawir
        .wahid(id)
        .is_some_and(|hay| hay.hala.lock().laqta.wad == WadTilqaiHie::Jariya)
    {
        return Err(Khata::from(KhataTilqaiAmr::MashwarJari {
            ism: id.to_string(),
        }));
    }

    // The same three refusals `hukm` reports, enforced where the run actually
    // starts and in the order `hukm` applies them, because a verdict is advice:
    // this command is reachable on its own, a click can race the verdict it was
    // drawn from, and a build finishing between the two changes the answer. All
    // three sit before `jahhiz`, so a refused game costs no provider build, no
    // font copy and no unlocked signing key — and the report itself is the one
    // the whole product already reads, cached in the store.
    let luba = ijlib_luba(makhzan, id)?;
    let simat = simat_luba(makhzan, id)?;
    let taqreer = taqreer_luba(makhzan, &luba, &simat, false)?;
    // First, and on its own, because the safety layer's refusal outranks both of
    // the others and is the one no release and no setting lifts.
    // `tahaqquq_jahiziya` answers that refusal before it answers the readiness
    // one, so calling it under `marfuda` takes exactly the first of the two.
    if taqreer.marfuda {
        tahaqquq_jahiziya(&taqreer)?;
    }
    let hali = idadat.hali();
    // Then the publisher's own Arabic. Permanent like the one above and, unlike
    // the one below, something the reader can act on today — which is why it
    // must not be buried under a refusal that an update lifts.
    if let Some(hala) = mahmiya_bil_lugha(id, &hali) {
        return Err(Khata::from(KhataTilqaiAmr::LughaRasmiya {
            ism: id.to_string(),
            hala_arabi: hala.ism_arabi().to_owned(),
            hala_injilizi: hala.ism_injilizi().to_owned(),
        }));
    }
    // Readiness last of the three: it is the only one a release changes.
    tahaqquq_jahiziya(&taqreer)?;

    // And then the two refusals no verdict can hold, brought forward from the
    // end of the run to here.
    //
    // Both used to fire in stage 7 of 7 — after extraction, after every paid
    // batch, after the atlas, the compile and the seal — over facts that were
    // already true and already readable before the first string was sent. The
    // multiplayer one could not even be satisfied: this layer hard-coded the
    // acknowledgement to `false`, so every multiplayer game was guaranteed to
    // spend the whole budget and then refuse. Neither refusal is softened here;
    // both are simply asked at the door, which costs a fraction of a second
    // against a run that costs real money.
    //
    // They come after the three above rather than before them because `hukm` is
    // a promise the screen made and these two are facts it could not have known:
    // the verdict does no I/O beyond the store, and deciding either of these
    // properly means walking the game folder and opening Steam's catalogue. A
    // game the verdict already refused must be refused for the verdict's own
    // reason, or the button and the screen name two different blockers for the
    // same game. What is added here is strictly a fourth and fifth refusal for
    // games the verdict offered — never a different answer to a question the
    // verdict answered.
    //
    // Resolved here rather than inside `jahhiz` so the scan can be handed the
    // root, and passed down afterwards so Steam is located once per press.
    let jidhr_steam = crate::luba_awamir::jidhr_steam_lil_fahs(masarat, &hali, &luba)?;
    // One catalogue read for both halves of the scan, exactly as
    // `taarib_aman::fahs` reads it once for both: `appinfo.vdf` is megabytes on
    // a mature account, and each half would otherwise materialise every app
    // entry in it all over again.
    let matjar = QiraatMatjar::iqra(appid_steam(&luba), jidhr_steam.as_deref());
    hima_al_bab(&luba, &matjar)?;
    // The first-run statement, asked at the door for the same reason the
    // multiplayer question is.
    //
    // It is the install gate's refusal, and the gate runs in stage 7 of 7 —
    // after extraction, after every translated string, after the atlas, the
    // compile and the seal. A person who had never been shown the statement
    // therefore paid for a whole translation and was then told the install was
    // refused, with the one thing that would satisfy it being a single tick on
    // a different screen. Nothing about that answer needs the patch to exist.
    iqrar_al_bab(masarat)?;
    // The multiplayer question last, for the reason `taarib_aman::fahs` puts it
    // last: it is the only refusal in the set with an answer the person can
    // give, and giving it is a tick rather than a wait or a lost account.
    shabakat_al_bab(&luba, &matjar, iqrar_shabaka)?;

    let mudkhalat = jahhiz(
        masarat,
        makhzan,
        idadat,
        id,
        istinaf,
        jidhr_steam,
        iqrar_shabaka,
    )?;
    // A resumed run starts from what the journal already knows rather than from
    // five blank rows: the stages it will skip are finished, and a list that
    // showed them as waiting would tell the user their four thousand translated
    // strings are about to be done again.
    let sabiqa = istinaf
        .then(|| laqta_min_qurs(masarat, &hali, id))
        .flatten();
    let laqta = LaqtatTilqaiHie {
        muarrif: id.to_string(),
        tashghila: mudkhalat.tashghila.to_string(),
        wad: WadTilqaiHie::Jariya,
        marhala: sabiqa
            .as_ref()
            .and_then(|sabiqa| sabiqa.marhala)
            .or(Some(MarhalatTilqaiHie::Istikhraj)),
        marahil: sabiqa
            .as_ref()
            .map_or_else(marahil_farigha, |sabiqa| sabiqa.marahil.clone()),
        qira: sabiqa.as_ref().and_then(|sabiqa| sabiqa.qira.clone()),
        takalif: TakalifHie {
            munfaq: dolar(mudkhalat.munfaq_sabiq),
            saqf: mudkhalat.saqf_dolar,
            umla: UMLA.to_owned(),
        },
        khata: None,
        muthabbata: false,
        sahb: Some(sahb_hie(&mudkhalat.qaima)),
        waqt: waqt_alaan(),
    };
    let hay = Arc::new(MashwarHay {
        tashghila: mudkhalat.tashghila,
        miqbad: MiqbadIlgha::jadeed(),
        hala: parking_lot::Mutex::new(HalatHay {
            laqta: laqta.clone(),
            akhir_bath: None,
        }),
    });
    // Recorded before the task exists, and under the map's own guard, so two
    // presses of the button that raced each other cannot both get past it.
    if !mashawir.sajjil(id, Arc::clone(&hay)) {
        return Err(Khata::from(KhataTilqaiAmr::MashwarJari {
            ism: id.to_string(),
        }));
    }

    tracing::info!(
        luba = %id,
        mashwar = %mudkhalat.tashghila,
        istinaf,
        muzawwid = mudkhalat.muzawwid.ism(),
        khutut = mudkhalat.khutut.len(),
        "an automatic run is starting"
    );
    // Six of the seven stages are synchronous and CPU-bound, so the run gets a
    // task of its own: `arrib` awaited inline would hold this command open for
    // the whole pipeline and the button would never come back. Detached on
    // purpose — the run outlives this call by minutes, and what it produces
    // reaches the screen on the event rather than through a handle nobody holds.
    drop(tauri::async_runtime::spawn(shaghghil(
        mudhee, hay, mudkhalat,
    )));
    Ok(laqta)
}

/// The anti-cheat half of the install gate, asked before the run instead of
/// after it.
///
/// This is the same question `taarib_aman::fahs` puts at stage 7 of 7, and it is
/// the same two answers: hard evidence refuses by naming it, and a catalogue
/// that could not be read refuses because VAC is declared there and nowhere
/// else, so silence from an unread catalogue is byte-identical to silence from a
/// clean game. Asking it there and only there meant a genuinely protected title
/// paid for its entire translation first — extraction, every batch, the atlas,
/// the compile and the seal — and met the refusal with the money already gone.
/// The scan costs a fraction of a second — between a sixth and three quarters of
/// one on the installed games it was measured against, the spread being the size
/// of the folder it walks. The run it precedes costs real money and cannot be
/// refunded, and a permanent ban cannot be undone at all.
///
/// It is asked here rather than in [`hukm`] because it walks the game directory
/// and reads Steam's catalogue, and the verdict is answered on mount and does no
/// I/O beyond the store. The stored capability report is the launcher's own
/// declaration and is checked before this one; this is the half that finds an
/// Easy Anti-Cheat or `BattlEye` payload a launcher never mentioned.
///
/// # Errors
///
/// [`KhataTilqaiAmr::HimayaMuktashafa`] for evidence, and
/// [`KhataTilqaiAmr::FahsHimayaLamYajri`] for a catalogue the scan was owed and
/// could not read.
fn hima_al_bab(luba: &Luba, matjar: &QiraatMatjar) -> Natija<()> {
    let (himaya, hala) = ifhas_himaya_bi_qiraa(&luba.jidhr, matjar);
    if mahmiya(&himaya) {
        return Err(Khata::from(KhataTilqaiAmr::HimayaMuktashafa {
            ism: luba.ism.clone(),
            anwa: asma_himaya(&himaya),
            dalail_arabi: sutur(himaya.adilla.iter().map(DaleelHimaya::arabi)),
            dalail_injilizi: sutur(himaya.adilla.iter().map(DaleelHimaya::injilizi)),
        }));
    }
    match hala {
        HalatMatjar::GhayrMatlub | HalatMatjar::Maqru => Ok(()),
        // Unreachable for a Steam game, which `jidhr_steam_lil_fahs` already
        // refused above, and impossible for one that is not — but the evidence
        // list this arm carries is indistinguishable from a clean game's, so it
        // is answered rather than assumed away.
        HalatMatjar::JidhrMajhul => Err(Khata::from(KhataTilqaiAmr::FahsHimayaLamYajri {
            ism: luba.ism.clone(),
            mawdi: None,
            sabab: "no Steam root was given for a Steam game".to_owned(),
        })),
        HalatMatjar::Mutaadhdhir { masar, sabab } => {
            Err(Khata::from(KhataTilqaiAmr::FahsHimayaLamYajri {
                ism: luba.ism.clone(),
                mawdi: Some(masar.display().to_string()),
                sabab,
            }))
        },
    }
}

/// The multiplayer half of the same gate, asked before the run instead of after.
///
/// The acknowledgement is never asserted here. `iqrar_shabaka` arrives from the
/// IPC payload, filled by a control the person ticked, exactly as the one-click
/// install obtains it — and a command layer that set it for them would be
/// agreeing, on their behalf, that they accept a permanent ban on their account.
/// What changes is only *when* the unticked box is discovered: at the door,
/// where the answer costs a press, rather than after a paid translation.
///
/// # Errors
///
/// [`KhataTilqaiAmr::ShabakaBilaIqrar`], carrying the evidence the scan found so
/// the interface can show what is being acknowledged.
/// Refuses before the run when the first-run statement is still unacknowledged.
///
/// The same record `taarib_aman::fahs` reads at the install and the same rule —
/// `yahtaj_iqrar` decides in both places, so the door cannot drift from the
/// gate and start letting through a run the gate will refuse.
///
/// # Errors
///
/// [`KhataTilqaiAmr::IqrarNaqis`] when the statement has not been acknowledged,
/// or has been superseded by a newer one.
fn iqrar_al_bab(masarat: &Masarat) -> Natija<()> {
    let sijill = iqrar::iqra(&crate::tathbeet_awamir::masar_iqrar(masarat))?;
    if iqrar::yahtaj_iqrar(sijill.as_ref()) {
        return Err(Khata::from(KhataTilqaiAmr::IqrarNaqis));
    }
    Ok(())
}

fn shabakat_al_bab(luba: &Luba, matjar: &QiraatMatjar, iqrar_shabaka: bool) -> Natija<()> {
    if iqrar_shabaka {
        return Ok(());
    }
    let shabaka = ifhas_shabaka_bi_qiraa(&luba.jidhr, matjar);
    if !mutaaddid(&shabaka) {
        return Ok(());
    }
    Err(Khata::from(KhataTilqaiAmr::ShabakaBilaIqrar {
        ism: luba.ism.clone(),
        wasf_arabi: shabaka.wasf_iqrar(),
        wasf_injilizi: shabaka.wasf_injilizi(),
    }))
}

/// The anti-cheats a scan named, joined for the log and the context table.
fn asma_himaya(himaya: &IjmaaHimaya) -> String {
    himaya
        .anwa()
        .into_iter()
        .map(NawHimaya::injilizi)
        .collect::<Vec<_>>()
        .join(", ")
}

/// One evidence line per row, as the refusal sentences quote them.
fn sutur(mutakarrir: impl Iterator<Item = String>) -> String {
    mutakarrir
        .map(|satr| format!("- {satr}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Asks the run for one game to stop, and answers with the snapshot as it stands.
///
/// The terminal snapshot arrives on [`ISM_HADATH_TILQAI`] once the pipeline has
/// unwound, which for a cancellation during translation is milliseconds and for
/// one during the install is however long the install and its reversal take.
///
/// # Errors
///
/// [`KhataTilqaiAmr::LaMashwar`] when this process has no run for the game.
#[tauri::command]
#[specta::specta]
pub fn alghi_tilqai(
    muarrif: String,
    mashawir: tauri::State<'_, MashawirTilqai>,
) -> Result<LaqtatTilqaiHie, Khata> {
    alghi(muarrif, &mashawir)
}

/// Stops a run, as everything but the IPC boundary sees it.
///
/// # Errors
///
/// As [`alghi_tilqai`].
pub(crate) fn alghi(muarrif: String, mashawir: &MashawirTilqai) -> Natija<LaqtatTilqaiHie> {
    let id = huwiya(muarrif)?;
    let hay = mashawir.wahid(id).ok_or_else(|| {
        Khata::from(KhataTilqaiAmr::LaMashwar {
            ism: id.to_string(),
        })
    })?;
    hay.miqbad.alghi();
    tracing::info!(luba = %id, mashwar = %hay.tashghila, "the run was asked to stop");
    Ok(hay.hala.lock().laqta.clone())
}

// ---------------------------------------------------------------------------
// The run's own inputs
// ---------------------------------------------------------------------------

/// Everything one run needs, owned, so the task that drives it borrows nothing.
struct MudkhalatMashwar {
    /// The game.
    muarrif: LubaId,
    /// The run.
    tashghila: MashwarId,
    /// The runs root for this game.
    jidhr_amal: PathBuf,
    /// The game's name.
    ism_luba: String,
    /// Its install root.
    jidhr_luba: PathBuf,
    /// Its executable, when the store recorded one.
    tanfidhi: Option<PathBuf>,
    /// Which launcher it came from.
    masdar: MasdarLuba,
    /// The compatibility layer it runs under.
    beea: BeeatTawafuq,
    /// The operating system its files are for.
    nizam: NizamTashghil,
    /// The translation provider.
    muzawwid: Box<dyn Muzawwid>,
    /// The key the patch is sealed with.
    miftah: MiftahKhass,
    /// The anchor the install gate checks that seal against.
    mirsa: taarib_khatm::MirsatThiqa,
    /// The fonts to bundle, head first.
    khutut: Vec<PathBuf>,
    /// What the finished patch declares.
    wasf: WasfTilqai,
    /// The verified revocation list beside where it stands, as the door read
    /// it; [`shaghghil`] refreshes and re-reads it before the first stage.
    qaima: QaimaMuraqaba,
    /// The data root, for that refresh.
    masarat: Masarat,
    /// The settings the run started under, for the same.
    hali: Arc<Idadat>,
    /// The first-run acknowledgement, when one was given.
    iqrar: Option<SijillIqrar>,
    /// The user's own answer to the multiplayer warning, carried to the install
    /// gate at the end of the run so that gate asks the person, not this layer.
    iqrar_shabaka: bool,
    /// The Steam application id, when this is a Steam game.
    appid: Option<u32>,
    /// Steam's install root, resolved by [`ibda`] the way the library scan
    /// resolves it — registry, then the known install directories, with the
    /// settings override winning when there is one. [`None`] only for a game
    /// that is not on Steam; a Steam game with no resolvable root never gets
    /// this far.
    jidhr_steam: Option<PathBuf>,
    /// How the run behaves.
    khiyarat: KhiyaratTilqai,
    /// The ceiling, in whole currency units, for the snapshot.
    saqf_dolar: f64,
    /// What previous runs of this game already spent, in nano-dollars.
    munfaq_sabiq: u64,
}

/// Assembles everything one run needs, refusing by name when a piece is missing.
///
/// `jidhr_steam` and `iqrar_shabaka` are handed in rather than found here.
/// [`ibda`] needs both before this is called — the root to run the anti-cheat
/// and multiplayer scan at the door, the acknowledgement to answer that scan —
/// and locating Steam twice per press, once to refuse on and once to record,
/// would be two chances for the two to disagree about where Steam is.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "both are carried through from `ibda`'s own payload unchanged; a struct here \
              would exist only to satisfy the lint and would have to be unpacked again"
)]
fn jahhiz(
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &Arc<MakhzanIdadat>,
    id: LubaId,
    istinaf: bool,
    jidhr_steam: Option<PathBuf>,
    iqrar_shabaka: bool,
) -> Natija<MudkhalatMashwar> {
    let luba = ijlib_luba(makhzan, id)?;
    let masdar = luba.masadir.first().cloned().ok_or_else(|| {
        Khata::from(KhataTilqaiAmr::LubaBilaMasdar {
            ism: luba.ism.clone(),
        })
    })?;
    let hali = idadat.hali();
    let tarif = muzawwid_muntakhab(&hali);
    let lahza = lahza_alaan();
    let saqf_nano = tarif
        .mizaniya
        .and_then(|mablagh| nano_min_dolar(mablagh).ok());
    let muzawwid = bin_muzawwid(&tarif, saqf_nano.unwrap_or(u64::MAX), lahza)?;
    // Both spend gates read this one value, so an unset budget opens both at
    // once: the ledger below takes `saqf_takalif: None` and the provider meter
    // above took `u64::MAX`. Neither then refuses anything, and the run is
    // bounded only by the game's string table running out — while the verdict
    // screen tells the reader it "stops at the ceiling and never crosses it".
    // A free provider is exempt because there is nothing to cap.
    if saqf_nano.is_none() && muzawwid.qudrat().taklifa.madfu() {
        return Err(Khata::from(KhataTilqaiAmr::BilaSaqfInfaq {
            muzawwid: tarif.muarrif.clone(),
        }));
    }

    let jidhr_amal = jidhr_mashawir(masarat, id);
    let akhir = akhir_mashwar(&jidhr_amal);
    let munfaq_sabiq = akhir.as_ref().map_or(0, munfaq_min_sijill);
    let tashghila = if istinaf {
        // A resume is the same run id and the same runs root, and nothing else:
        // `arrib` skips every stage the journal closed on its own.
        ijrud(&jidhr_amal)
            .into_iter()
            .next()
            .map(|mawjuz| mawjuz.id)
            .ok_or_else(|| {
                Khata::from(KhataTilqaiAmr::LaIstinaf {
                    ism: luba.ism.clone(),
                })
            })?
    } else {
        MashwarId::jadeed()
    };

    let khutut = khutut_arabiya(masarat, &hali);
    if khutut.is_empty() {
        return Err(Khata::from(KhataTilqaiAmr::LaKhattArabi));
    }
    let (_musahim, ism_musahim, _itimad) = crate::taqdeem_awamir::hawiyati(masarat)?;
    let miftah = crate::taqdeem_awamir::miftah_musahim()?;
    // The install gate is anchored to this run's own signing key, and not to
    // `taarib_khatm::MIRSAT_MALIK`.
    //
    // That anchor is the right one for a package that arrived from somewhere:
    // the registry, a shared file, a mirror. It answers "did the owner sign
    // this", which is the only question worth asking about bytes with a
    // provenance. This package has none. It was extracted, translated, compiled
    // and sealed inside this process, seconds ago, with a key that has never
    // left this machine — so the honest question is "is this the package this
    // run just made", and the key that made it is exactly what answers it.
    //
    // It widens nothing. The anchor is derived from the very key handed to the
    // signer below, so the gate still refuses an unsigned container, a tampered
    // one, and one signed by anybody else — including the committed development
    // key. Every package that did arrive from somewhere still goes through
    // `tathbeet_awamir::thabbit_ruqaa`, which anchors to the owner.
    let mirsa = taarib_khatm::MirsatThiqa {
        miftah: miftah.aam().bayt(),
        // Not read by the gate, which compares keys. It carries this build's own
        // identity so a diagnostics bundle still says which client this was.
        hawiya: taarib_khatm::MIRSAT_MALIK.hawiya,
    };
    // The cache as the door finds it, refused here for the one thing it
    // refuses on; the run refreshes it before the first paid batch.
    let qaima = crate::tathbeet_awamir::qaimat_sahb_lil_bawwaba(masarat, &hali)?;
    let iqrar = iqrar::iqra(&crate::tathbeet_awamir::masar_iqrar(masarat))?;

    let waqt = waqt_alaan();
    let mut khiyarat = KhiyaratTilqai::jadeeda(lahza, waqt);
    khiyarat.saqf_takalif = saqf_nano;

    Ok(MudkhalatMashwar {
        muarrif: id,
        tashghila,
        jidhr_amal,
        ism_luba: luba.ism.clone(),
        jidhr_luba: luba.jidhr.clone(),
        tanfidhi: luba.tanfidhi.clone(),
        masdar,
        beea: luba.beea.clone(),
        nizam: NizamTashghil::hali(),
        muzawwid,
        miftah,
        mirsa,
        khutut,
        wasf: WasfTilqai {
            unwan: None,
            ism_musahim,
            // The one licence that lets a machine-translated patch be shared with
            // no further permission asked. A contributor who wants another one
            // takes the sealed package into the submission flow, which is where a
            // licence is actually chosen.
            rukhsa: RukhsaRuqaa::Cc0,
            isdar_taarib: ISDAR.to_owned(),
        },
        qaima,
        masarat: masarat.clone(),
        hali: Arc::clone(&hali),
        iqrar,
        iqrar_shabaka,
        appid: appid_steam(&luba),
        jidhr_steam,
        khiyarat,
        saqf_dolar: saqf_nano.map_or(0.0, dolar),
        munfaq_sabiq,
    })
}

/// This game's runs root: one directory per game, under the data root.
///
/// Per game rather than one root for everything, because `taarib_tilqai::ijrud`
/// enumerates a directory and its listing carries the game's *name* and root but
/// not its identity — so a shared root could not answer "what was I in the middle
/// of for this game" without opening every run on the machine and guessing.
pub(crate) fn jidhr_mashawir(masarat: &Masarat, id: LubaId) -> PathBuf {
    masarat
        .jidhr_bayanat()
        .join(MUJALLAD_TILQAI)
        .join(id.to_string())
}

/// Every font on this machine that can actually carry Arabic, head first.
///
/// The head is the font settings nominate for a new patch, because that is the
/// choice the user already made; everything after it is a fallback for the
/// characters the head does not have. A face that fails the Arabic validator is
/// left out rather than demoted: `taarib_tilqai::bina::hayyi` validates the whole
/// chain and one Latin face in it fails the entire run.
fn khutut_arabiya(masarat: &Masarat, hali: &Idadat) -> Vec<PathBuf> {
    let mut judhur = vec![masarat.khutut()];
    if let Some(masar) = &hali.khutut.masar_khutut_mustakhdim {
        judhur.push(masar.clone());
    }
    judhur.extend(
        crate::mukawwinat_tahmil::judhur_khutut(masarat)
            .into_iter()
            .skip(1),
    );

    let mufaddal = hali.khutut.khatt_luba_iftiradi.to_lowercase();
    let mut arabiya: Vec<(bool, PathBuf)> = Vec::new();
    for masar in crate::mukawwinat_tahmil::milaffat_khutut(&judhur) {
        let Ok(bayt) = std::fs::read(&masar) else {
            continue;
        };
        if MawridKhatt::jadeed(Arc::new(bayt), 0).is_err() {
            continue;
        }
        let ism = masar
            .file_stem()
            .map(|ism| ism.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        arabiya.push((!ism.starts_with(&mufaddal), masar));
    }
    // Stable, so the nominated face keeps its place and everything after it stays
    // in the order the font roots offered it.
    arabiya.sort_by_key(|(baad, _)| *baad);
    arabiya
        .into_iter()
        .take(AQSA_KHUTUT)
        .map(|(_, masar)| masar)
        .collect()
}

// ---------------------------------------------------------------------------
// The run itself
// ---------------------------------------------------------------------------

/// Drives one run to its end and publishes every snapshot it produces.
async fn shaghghil(mudhee: MudheeLaqta, hay: Arc<MashwarHay>, mut mudkhalat: MudkhalatMashwar) {
    // The moment before the money: the registry's current word on what is
    // withdrawn, fetched here because this task may wait for a network and the
    // command that started it may not. The door read the cache; this re-reads
    // it after the fetch, and a registry that answers and withholds its list
    // stops the run before the first string is sent.
    let _ = crate::tathbeet_awamir::jaddid_sahb(&mudkhalat.masarat, &mudkhalat.hali).await;
    match iqra_qaimat_sahb(&mudkhalat.masarat, &mudkhalat.hali) {
        Ok(qaima) => {
            if let Some(rafd) = qaima.rafd() {
                let khata = Khata::min_tafsir(&KhataTilqaiAmr::QaimatSahbMahjuba {
                    masdar: rafd.masdar.clone(),
                    sabab: rafd.sabab.clone(),
                    waqt: rafd.waqt.to_string(),
                });
                awqif_qabl_al_bidaya(&mudhee, &hay, &khata, Some(sahb_hie(&qaima)));
                return;
            }
            mudkhalat.qaima = qaima;
        },
        Err(khata) => {
            awqif_qabl_al_bidaya(&mudhee, &hay, &khata, None);
            return;
        },
    }

    let mukhbir = {
        let hay = Arc::clone(&hay);
        let mudhee = Arc::clone(&mudhee);
        MukhbirTaqaddum::min_nida(move |taqaddum: Taqaddum| {
            let mabthuth = {
                let mut hala = hay.hala.lock();
                let taghayyar = tabbiq_taqaddum(&mut hala.laqta, &taqaddum);
                let alaan = Instant::now();
                let mada = hala
                    .akhir_bath
                    .is_none_or(|sabiq| alaan.duration_since(sabiq) >= MUDDAT_BATH);
                if taghayyar || mada {
                    hala.akhir_bath = Some(alaan);
                    Some(hala.laqta.clone())
                } else {
                    None
                }
            };
            if let Some(laqta) = mabthuth {
                mudhee(&laqta);
            }
        })
    };

    let mujallad_mukawwinat = mudkhalat.masarat.mukawwinat();
    let mujallad_mashari = mudkhalat.masarat.mashari();
    let talab = TalabTilqai {
        id: mudkhalat.tashghila,
        jidhr_amal: &mudkhalat.jidhr_amal,
        luba: LubaTilqai {
            jidhr: &mudkhalat.jidhr_luba,
            ism: &mudkhalat.ism_luba,
            masdar: mudkhalat.masdar.clone(),
            tanfidhi: mudkhalat.tanfidhi.as_deref(),
            nizam: mudkhalat.nizam,
            beea: &mudkhalat.beea,
        },
        muzawwid: &*mudkhalat.muzawwid,
        miftah: &mudkhalat.miftah,
        khutut: &mudkhalat.khutut,
        // The capture route is offered by the screen, which then starts a run
        // with the same id and the session file that pass wrote. Nothing here
        // invents one, because a session nobody produced is a file that is not
        // there.
        jalsat_iltiqat: None,
        // The store this installation ships. Without it the run writes the
        // patch and its fonts into the game and no loader, and a game that was
        // never patched before starts in its original language with the whole
        // translation sitting on disk beside it.
        mukawwinat: Some(&mujallad_mukawwinat),
        // The same store the workshop opens from. Without it a finished run
        // leaves the workshop saying no project exists for this game yet —
        // which is what every game did, because nothing outside a test had ever
        // created one.
        mashari: Some(&mujallad_mashari),
        wasf: mudkhalat.wasf.clone(),
        aman: MudkhalatAman {
            mirsa: &mudkhalat.mirsa,
            qaima: mudkhalat.qaima.qaima(),
            iqrar: mudkhalat.iqrar.as_ref(),
            appid: mudkhalat.appid,
            jidhr_steam: mudkhalat.jidhr_steam.as_deref(),
            // Still never asserted here: this is the person's own answer, taken
            // from `ibda_tilqai`'s payload the way the one-click install takes
            // it, and carried down unchanged. Hard-coding `false` used to make
            // the end-of-run gate refuse *every* multiplayer game after the
            // whole translation was bought — a refusal nothing could satisfy,
            // reached only by spending. `shabakat_al_bab` now asks the same
            // question of the same scan before the run, so an unticked box costs
            // a press and a ticked one reaches this gate honestly.
            iqrar_shabaka: mudkhalat.iqrar_shabaka,
        },
        khiyarat: mudkhalat.khiyarat.clone(),
        mukhbir: &mukhbir,
        miqbad: &hay.miqbad,
    };

    let natija = arrib(&talab).await;
    let sabiqa = hay.hala.lock().laqta.clone();
    for satr in natija.taqreer.taqreer() {
        tracing::info!(luba = %mudkhalat.muarrif, mashwar = %natija.id, "{satr}");
    }
    if let Some(khata) = &natija.khata {
        tracing::warn!(
            luba = %mudkhalat.muarrif,
            mashwar = %natija.id,
            khata = %Khata::min_tafsir(khata).li_sijill(),
            "the automatic run stopped"
        );
    }

    let laqta = laqta_min_natija(&mudkhalat, &natija, &sabiqa);
    {
        let mut hala = hay.hala.lock();
        hala.laqta = laqta.clone();
        hala.akhir_bath = Some(Instant::now());
    }
    mudhee(&laqta);
}

/// Ends a run that was refused before its first stage, on the snapshot the
/// door published: the first row fails, the failure is named, and nothing was
/// spent or written.
fn awqif_qabl_al_bidaya(
    mudhee: &MudheeLaqta,
    hay: &MashwarHay,
    khata: &Khata,
    sahb: Option<HalatSahbHie>,
) {
    let laqta = {
        let mut hala = hay.hala.lock();
        hala.laqta.wad = WadTilqaiHie::Fashal;
        hala.laqta.khata = Some(KhataTilqaiHie {
            ramz: khata.ramz.to_string(),
            arabi: khata.arabi.clone(),
            injilizi: khata.injilizi.clone(),
        });
        if let Some(saf) = hala.laqta.marahil.first_mut() {
            saf.hala = HalatMarhalaHie::Fashilat;
        }
        if sahb.is_some() {
            hala.laqta.sahb = sahb;
        }
        hala.laqta.waqt = waqt_alaan();
        hala.akhir_bath = Some(Instant::now());
        hala.laqta.clone()
    };
    tracing::warn!(
        khata = %khata.li_sijill(),
        "the automatic run stopped before its first stage"
    );
    mudhee(&laqta);
}

/// Folds one progress report into the snapshot; answers whether the stage moved.
fn tabbiq_taqaddum(laqta: &mut LaqtatTilqaiHie, taqaddum: &Taqaddum) -> bool {
    laqta.takalif.munfaq = dolar(taqaddum.munfaq);
    if let Some(saqf) = taqaddum.saqf {
        laqta.takalif.saqf = dolar(saqf);
    }
    laqta.waqt = waqt_alaan();

    let Some(marhala) = marhala_hie(taqaddum.marhala) else {
        // The terminal report. Nothing about the five rows is decided from it —
        // the whole run's own report is what the next snapshot is built from.
        return false;
    };
    let taghayyar = laqta.marhala != Some(marhala);
    laqta.marhala = Some(marhala);

    // The probe reports against a detector count, and the row it lands on counts
    // containers. Reporting its numbers there would fill the extraction bar and
    // then empty it, so the probe speaks in words and the row stays undenominated
    // until extraction has something real to divide by.
    let fahs = taqaddum.marhala == MarhalaTilqai::Fahs;
    for saf in &mut laqta.marahil {
        match saf.marhala.cmp(&marhala) {
            Ordering::Less => saf.hala = HalatMarhalaHie::Tammat,
            Ordering::Equal => {
                saf.hala = HalatMarhalaHie::Jariya;
                saf.tamma = if fahs { 0 } else { raqm_u32(taqaddum.munjaz) };
                saf.majmu = if fahs {
                    None
                } else {
                    taqaddum.majmu.map(raqm_u32)
                };
                saf.tafsil = Some(taqaddum.amal.clone());
            },
            Ordering::Greater => {},
        }
    }
    taghayyar
}

/// The whole run's own report, as the snapshot the screen ends on.
fn laqta_min_natija(
    mudkhalat: &MudkhalatMashwar,
    natija: &NatijatMashwar,
    sabiqa: &LaqtatTilqaiHie,
) -> LaqtatTilqaiHie {
    let taqreer = &natija.taqreer;
    let wad = match taqreer.hala {
        HalatMashwar::Tammat => WadTilqaiHie::Jahiz,
        HalatMashwar::Mulgha => WadTilqaiHie::Mulgha,
        HalatMashwar::TahtajIltiqat => WadTilqaiHie::Iltiqat,
        HalatMashwar::Tawaqqafat => WadTilqaiHie::Fashal,
    };
    let waqifa = marhala_hie(taqreer.marhala);
    let hala_waqifa = match wad {
        // A run the user stopped did not fail. The screen paints a failed stage
        // red, and painting the user's own cancel red would be the interface
        // calling their decision a fault.
        WadTilqaiHie::Mulgha => HalatMarhalaHie::Jariya,
        WadTilqaiHie::Jahiz => HalatMarhalaHie::Tammat,
        _ => HalatMarhalaHie::Fashilat,
    };

    // Seeded from the last live snapshot rather than from nothing: the run's own
    // report carries what each closed stage accounted for and never carries the
    // sentence it was saying while it did, and a stage that stopped part-way has
    // no record at all — so a list built from the report alone would replace
    // "2 of 5 — the safety gate" with a blank row reading zero.
    let mut marahil = sabiqa.marahil.clone();
    for marhala in &taqreer.marahil {
        if marhala.marhala == MarhalaTilqai::Fahs {
            continue;
        }
        let Some(mawqi) = marhala_hie(marhala.marhala) else {
            continue;
        };
        for saf in marahil.iter_mut().filter(|saf| saf.marhala == mawqi) {
            saf.hala = HalatMarhalaHie::Tammat;
            saf.tamma = raqm_u32(marhala.munjaz);
            saf.majmu = marhala.majmu.map(raqm_u32);
        }
    }
    for saf in &mut marahil {
        match waqifa {
            None => saf.hala = HalatMarhalaHie::Tammat,
            Some(waqifa) => match saf.marhala.cmp(&waqifa) {
                Ordering::Less => saf.hala = HalatMarhalaHie::Tammat,
                Ordering::Equal => saf.hala = hala_waqifa,
                Ordering::Greater => saf.hala = HalatMarhalaHie::Muntazira,
            },
        }
    }
    if let Some(tarjama) = &taqreer.tarjama {
        for saf in marahil
            .iter_mut()
            .filter(|saf| saf.marhala == MarhalatTilqaiHie::Tarjama)
        {
            saf.tafsil = Some(format!(
                "{} of {} answered this run, {} applied in total, {} failed ({} refused by the \
                 placeholder guard)",
                tarjama.ujiba,
                tarjama.muahhala,
                tarjama.mutabbaqa,
                tarjama.fashila,
                tarjama.marfuda_bil_hima
            ));
        }
    }

    let munfaq = taqreer
        .tarjama
        .as_ref()
        .map_or(mudkhalat.munfaq_sabiq, |tarjama| tarjama.munfaq);
    LaqtatTilqaiHie {
        muarrif: mudkhalat.muarrif.to_string(),
        tashghila: natija.id.to_string(),
        wad,
        // Kept rather than nulled for a run that stopped: the resume line names
        // the stage it would continue from, and a cancelled run's own list has to
        // show which row the stop landed on.
        marhala: if wad == WadTilqaiHie::Jahiz {
            None
        } else {
            waqifa
        },
        marahil,
        qira: taqreer
            .istikhraj
            .as_ref()
            .map(|ihsa| qira_hie(ihsa, &natija.mujallad)),
        takalif: TakalifHie {
            munfaq: dolar(munfaq),
            saqf: mudkhalat.saqf_dolar,
            umla: UMLA.to_owned(),
        },
        khata: natija.khata.as_ref().map(khata_hie),
        sahb: Some(sahb_hie(&mudkhalat.qaima)),
        // A reversed install wrote nothing that survived it, so the game is
        // exactly as it was and the screen's cancel wording must say so.
        muthabbata: taqreer
            .tathbeet
            .as_ref()
            .is_some_and(|tathbeet| !tathbeet.rujia),
        waqt: waqt_alaan(),
    }
}

/// What extraction read, as the screen shows it.
fn qira_hie(ihsa: &IhsaIstikhraj, mujallad: &Path) -> TaqreerQiraHie {
    TaqreerQiraHie {
        maqru: raqm_u32(ihsa.maqrua),
        matruk: raqm_u32(ihsa.marfuda),
        nusus: raqm_u32(ihsa.adad_zahir),
        asbab: asbab_rafd(mujallad),
    }
}

/// One pipeline failure, in the three fields the screen reads.
fn khata_hie(khata: &taarib_tilqai::KhataTilqai) -> KhataTilqaiHie {
    let mabniyya = Khata::min_tafsir(khata);
    KhataTilqaiHie {
        ramz: mabniyya.ramz.to_string(),
        arabi: mabniyya.arabi,
        injilizi: mabniyya.injilizi,
    }
}

/// The row of the five one pipeline stage lands on.
const fn marhala_hie(marhala: MarhalaTilqai) -> Option<MarhalatTilqaiHie> {
    match marhala {
        MarhalaTilqai::Fahs | MarhalaTilqai::Istikhraj => Some(MarhalatTilqaiHie::Istikhraj),
        MarhalaTilqai::Tarjama => Some(MarhalatTilqaiHie::Tarjama),
        MarhalaTilqai::Takhtit => Some(MarhalatTilqaiHie::Takhtit),
        MarhalaTilqai::Tarqee | MarhalaTilqai::Khatm => Some(MarhalatTilqaiHie::Tajmee),
        MarhalaTilqai::Tathbeet => Some(MarhalatTilqaiHie::Tathbeet),
        MarhalaTilqai::Tamma => None,
    }
}

/// Money and its ceiling, with the ceiling taken from the elected provider.
fn takalif_hie(munfaq: u64, hali: &Idadat) -> TakalifHie {
    let saqf = muzawwid_muntakhab(hali)
        .mizaniya
        .filter(|mablagh| mablagh.is_finite() && *mablagh > 0.0)
        .unwrap_or(0.0);
    TakalifHie {
        munfaq: dolar(munfaq),
        saqf,
        umla: UMLA.to_owned(),
    }
}

/// A pipeline count at the width the interface reads numbers in.
fn raqm_u32(qeema: u64) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------

/// Failures that belong to this command layer rather than to the pipeline.
#[derive(Debug, thiserror::Error)]
pub enum KhataTilqaiAmr {
    /// A run for this game is already going.
    #[error("an automatic run for {ism} is already in progress")]
    MashwarJari {
        /// The game.
        ism: String,
    },

    /// Nothing to stop: this process started no run for the game.
    #[error("no automatic run for {ism} is in progress")]
    LaMashwar {
        /// The game.
        ism: String,
    },

    /// A resume was asked for and no run directory exists to resume.
    #[error("no unfinished automatic run exists for {ism}")]
    LaIstinaf {
        /// The game.
        ism: String,
    },

    /// No font on this machine passes the Arabic validator.
    #[error("no font on this machine can carry Arabic")]
    LaKhattArabi,

    /// The game's record carries no launcher identity.
    #[error("{ism} has no launcher identity, so its patch cannot be bound to a source")]
    LubaBilaMasdar {
        /// The game.
        ism: String,
    },

    /// The publisher already ships Arabic with this game.
    #[error("{ism} already ships official Arabic ({hala_injilizi})")]
    LughaRasmiya {
        /// The game.
        ism: String,
        /// How much Arabic, in Arabic.
        hala_arabi: String,
        /// The same, in English.
        hala_injilizi: String,
    },

    /// The elected provider charges money and has no spend ceiling set.
    ///
    /// A newly added provider starts on `mizaniya: None`, and an unset ceiling
    /// opened *both* gates at once: the run ledger took `saqf_takalif: None`
    /// and never refused, and the provider meter took `u64::MAX` and never
    /// refused either — so the run was bounded only by the string table running
    /// out. The verdict screen meanwhile promised "the run stops at the ceiling
    /// and never crosses it". Refusing to start is the only reading of that
    /// sentence that is true.
    #[error("{muzawwid} charges for translation and has no spend ceiling set")]
    BilaSaqfInfaq {
        /// The elected provider.
        muzawwid: String,
    },

    /// The anti-cheat scan found evidence, at the door rather than at the end.
    ///
    /// The identical refusal already existed as `taarib_aman::fahs::Rafd::Himaya`
    /// at stage 7 of 7, which is after extraction, after every paid batch, after
    /// the atlas, the compile and the seal. Nothing about the answer needed any
    /// of that: the evidence is on disk and in Steam's catalogue before the run
    /// begins. This is the same verdict, read at the same source, delivered
    /// while the user still has their money.
    #[error("{ism} runs anti-cheat ({anwa})")]
    HimayaMuktashafa {
        /// The game.
        ism: String,
        /// The anti-cheats named, joined, for the log and the context table.
        anwa: String,
        /// The evidence, one line each, in Arabic.
        dalail_arabi: String,
        /// The same lines in English.
        dalail_injilizi: String,
    },

    /// The anti-cheat scan was owed Steam's catalogue and could not read it.
    ///
    /// VAC leaves nothing whatever inside a game folder — it is declared in
    /// `appinfo.vdf` and nowhere else — so a scan that never opened the
    /// catalogue produces exactly the empty evidence list a genuinely clean game
    /// produces. Rounding "the check did not run" down to "the check passed"
    /// would let a VAC-secured title through, and a VAC ban is permanent and
    /// applies to the account rather than to the game.
    #[error("{ism}: the anti-cheat check could not read Steam's catalogue ({sabab})")]
    FahsHimayaLamYajri {
        /// The game.
        ism: String,
        /// The catalogue that was tried, when a Steam root was known at all.
        mawdi: Option<String>,
        /// Why, as the short label the scan's own gap list carries.
        sabab: String,
    },

    /// The game is multiplayer and the acknowledgement was not given.
    ///
    /// The one refusal here the user can lift, and the one that used to be
    /// unliftable: the command layer hard-coded the acknowledgement to `false`,
    /// so every multiplayer game — which is most of what people play together,
    /// including the title this build is developed against — was refused by the
    /// install gate at the end of a run whose translation had already been paid
    /// for, with no way to answer the question that refused it. It is asked
    /// before the run now, and the answer is the person's.
    #[error("{ism} is multiplayer and the modification risk was not acknowledged")]
    ShabakaBilaIqrar {
        /// The game.
        ism: String,
        /// What the scan found, as the acknowledgement text quotes it, in Arabic.
        wasf_arabi: String,
        /// The same, in English.
        wasf_injilizi: String,
    },

    /// The first-run statement has not been acknowledged.
    ///
    /// The install gate's own refusal, raised at the door instead. It is the
    /// one refusal in the set that has nothing to do with the game, the patch
    /// or the machine: it is a sentence the person has not read yet, and
    /// reading it costs a tick.
    #[error("the first-run statement has not been acknowledged")]
    IqrarNaqis,

    /// The registry is answering and its revocation list is not.
    ///
    /// The manual path's `9031`, raised here at the start of the run — after
    /// the door, before the first paid batch — from the refresh the run itself
    /// makes. An unreachable registry never raises it: an offline machine runs
    /// against its local copy with the list's state said out loud. A registry
    /// that serves its manifest and withholds the one document able to withdraw
    /// a patch is refused, because on a machine that can ask, "we could not
    /// check" must not become "nothing is revoked".
    #[error("the registry {masdar} answered and did not serve its revocation list: {sabab}")]
    QaimatSahbMahjuba {
        /// The source that answered the manifest.
        masdar: String,
        /// Why no list was accepted.
        sabab: String,
        /// When that attempt was made, RFC 3339.
        waqt: String,
    },
}

impl Tafsir for KhataTilqaiAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MashwarJari { .. } => 120,
                    Self::LaMashwar { .. } => 121,
                    Self::LaIstinaf { .. } => 122,
                    Self::LaKhattArabi => 123,
                    Self::LubaBilaMasdar { .. } => 124,
                    Self::LughaRasmiya { .. } => 125,
                    Self::BilaSaqfInfaq { .. } => 126,
                    Self::HimayaMuktashafa { .. } => 127,
                    Self::FahsHimayaLamYajri { .. } => 128,
                    Self::ShabakaBilaIqrar { .. } => 129,
                    // 130 to 136 belong to the sharing surface, so the twin of
                    // the manual path's `9031` takes the first free number
                    // after them.
                    Self::QaimatSahbMahjuba { .. } => 137,
                    Self::IqrarNaqis => 138,
                },
        )
    }

    #[expect(
        clippy::match_same_arms,
        reason = "a severity is shared by refusals that have nothing else in common — an \
                  anti-cheat detection and a registry withholding its revocation list are both \
                  `Tanbeeh` for unrelated reasons, and merging them would attach one comment to \
                  two facts and let a change to either move the other"
    )]
    fn khutura(&self) -> Khutura {
        match self {
            // Neither is a fault: one is the product refusing to run twice over
            // the same game, the other is a button pressed with nothing behind it.
            Self::MashwarJari { .. } | Self::LaMashwar { .. } | Self::LaIstinaf { .. } => {
                Khutura::Maluma
            },
            // Not a fault either, and deliberately not a warning: the product
            // is declining to write machine output over a publisher's own
            // Arabic, which is the correct outcome rather than a degraded one.
            // The multiplayer one is the same shape — a question waiting for its
            // answer, not something that went wrong.
            Self::LughaRasmiya { .. } | Self::ShabakaBilaIqrar { .. } => Khutura::Maluma,
            Self::LaKhattArabi | Self::LubaBilaMasdar { .. } | Self::BilaSaqfInfaq { .. } => {
                Khutura::Tanbeeh
            },
            // The account is what is at stake in both, so neither is reported as
            // routine: one names anti-cheat evidence, the other names a check
            // that could not be completed and must not be read as a pass.
            Self::HimayaMuktashafa { .. } | Self::FahsHimayaLamYajri { .. } => Khutura::Tanbeeh,
            // A check that could not be completed and must not be read as a
            // pass — about the registry rather than the account.
            Self::QaimatSahbMahjuba { .. } => Khutura::Tanbeeh,
            Self::IqrarNaqis => Khutura::Maluma,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MashwarJari { .. } => {
                "هناك تعريب تلقائي جارٍ لهذه اللعبة بالفعل. انتظر انتهاءه أو ألغِه أوّلًا.".to_owned()
            },
            Self::LaMashwar { .. } => {
                "لا يوجد تعريب تلقائي جارٍ لهذه اللعبة في هذه الجلسة.".to_owned()
            },
            Self::LaIstinaf { .. } => {
                "لا توجد جولة غير مكتملة لاستئنافها. ابدأ جولة جديدة.".to_owned()
            },
            Self::LaKhattArabi => {
                "لا يوجد على هذا الجهاز خطّ يجتاز فحص العربية، والرقعة لا تُبنى بدون خطّ. \
                 أضف خطًّا عربيًّا من الإعدادات ← الخطوط."
                    .to_owned()
            },
            Self::LubaBilaMasdar { .. } => {
                "سجلّ هذه اللعبة لا يحمل هوية منصّة، ولا يمكن ربط الرقعة بمصدر. أعد فحص المكتبة."
                    .to_owned()
            },
            Self::LughaRasmiya { hala_arabi, .. } => format!(
                "الناشر يشحن عربية رسمية مع هذه اللعبة ({hala_arabi})، والتعريب التلقائي لا \
                 يكتب ترجمة آلية فوق ترجمة بشرية مدفوعة راجعها ناطقون بالعربية. إن كانت تلك \
                 العربية غير صالحة فعلًا، فعّل «استبدال اللغة الرسمية» في الإعدادات."
            ),
            Self::BilaSaqfInfaq { muzawwid } => format!(
                "المزوّد «{muzawwid}» يتقاضى أجرًا على الترجمة ولم يُضبط له سقف إنفاق. \
                 بدون سقف لا يتوقّف التعريب التلقائي عند حدّ — يمضي حتى تنتهي نصوص اللعبة \
                 مهما بلغت التكلفة. اضبط «الميزانية» لهذا المزوّد في الإعدادات ← المزوّدون، \
                 وهي لكلّ جولة لا لكلّ شهر."
            ),
            Self::HimayaMuktashafa {
                anwa, dalail_arabi, ..
            } => format!(
                "تعمل هذه اللعبة بنظام مكافحة غش ({anwa})، ولا يُعرَّب عنوان كهذا: تعديل \
                 ملفاته قد يكلّفك حظرًا دائمًا لحسابك، والحظر يلحق بالحساب لا باللعبة. \
                 توقّف الأمر قبل أن يُنفَق شيء وقبل أن يُكتب شيء. الدليل:\n{dalail_arabi}"
            ),
            Self::FahsHimayaLamYajri { mawdi, sabab, .. } => {
                let mawdi = mawdi.as_ref().map_or_else(
                    || "لم يُعرف موضع تثبيت ستيم على هذا الجهاز".to_owned(),
                    |mawdi| format!("تعذّرت قراءة {mawdi} ({sabab})"),
                );
                format!(
                    "لم يُستكمل فحص مكافحة الغش، فلم تبدأ الجولة. حماية VAC لا تُعلَن إلا في \
                     فهرس متجر ستيم ولا تترك أثرًا في مجلّد اللعبة، فسكوت الفحص هنا ليس \
                     براءة. {mawdi}. حدِّد مجلد ستيم في الإعدادات ← المنصّات ثم أعد المحاولة."
                )
            },
            Self::ShabakaBilaIqrar { wasf_arabi, .. } => format!(
                "هذه لعبة متعدّدة اللاعبين، ويلزم إقرارك بمخاطر التعديل قبل أن تبدأ الجولة. \
                 تعديل لعبة تُلعب مع آخرين قد يُفقدك حسابك أو يمنعك من الخوادم، والقرار \
                 قرارك وحدك. لم يُنفَق شيء بعد.\n{wasf_arabi}"
            ),
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => format!(
                "أجاب المستودع ({masdar}) في {waqt} لكنّه لم يقدّم قائمة الإبطال ({sabab})، \
                 فلم تبدأ الجولة ولم يُنفَق شيء. ما دام المستودع يجيب فلا تُثبَّت رقعة قبل \
                 قراءة قائمته، لأنّ الرقعة التي سُحبت لا تُعرف إلا منها. أعد المحاولة بعد \
                 قليل؛ وإن كان المصدر مجلّدًا محليًا فتأكّد من أنّ ملف القائمة موجود فيه."
            ),
            Self::IqrarNaqis => "لم يُقرّ بعدُ بيان التشغيل الأوّل، وهو شرط تثبيت أي رقعة. \
                 البيان معروض الآن على هذه الشاشة: اقرأه وأقرّ به، ثم ابدأ من جديد. لم تبدأ \
                 الجولة ولم يُنفَق شيء: السؤال عنه هنا — قبل الاستخراج والترجمة — لأنّ \
                 الإجابة عنه لا تحتاج رقعةً أصلًا."
                .to_owned(),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MashwarJari { ism } => format!(
                "An automatic run for {ism} is already in progress. Wait for it to finish, or \
                 cancel it first."
            ),
            Self::LaMashwar { ism } => {
                format!("No automatic run for {ism} is in progress in this session.")
            },
            Self::LaIstinaf { ism } => {
                format!("There is no unfinished run for {ism} to resume. Start a new one.")
            },
            Self::LaKhattArabi => {
                "No font on this machine passes the Arabic validator, and a patch cannot be built \
                 without one. Add an Arabic font in Settings, under Fonts."
                    .to_owned()
            },
            Self::LubaBilaMasdar { ism } => format!(
                "{ism} has no launcher identity on record, so a patch cannot be bound to a \
                 source. Rescan the library."
            ),
            Self::LughaRasmiya { hala_injilizi, .. } => format!(
                "The publisher already ships Arabic with this game ({hala_injilizi}), and \
                 automatic arabization does not write machine output over paid human \
                 translation that native speakers reviewed. If that Arabic is genuinely \
                 unusable, turn on replacing the official language in Settings."
            ),
            Self::BilaSaqfInfaq { muzawwid } => format!(
                "{muzawwid} charges for translation and has no spend ceiling set. Without one \
                 the automatic run stops at no limit — it goes until the game's strings run \
                 out, whatever that costs. Set this provider's budget in Settings, Providers; \
                 it is per run, not per month."
            ),
            Self::HimayaMuktashafa {
                anwa,
                dalail_injilizi,
                ..
            } => format!(
                "This game runs anti-cheat ({anwa}), and Taarib does not Arabize such a title: \
                 modifying its files can cost you a permanent ban, and the ban attaches to your \
                 account rather than to the game. Nothing was spent and nothing was written. \
                 Evidence:\n{dalail_injilizi}"
            ),
            Self::FahsHimayaLamYajri { mawdi, sabab, .. } => {
                let mawdi = mawdi.as_ref().map_or_else(
                    || "no Steam installation could be located on this machine".to_owned(),
                    |mawdi| format!("{mawdi} could not be read ({sabab})"),
                );
                format!(
                    "The anti-cheat check did not finish, so the run did not start. VAC is \
                     declared only in Steam's catalogue and leaves nothing in the game folder, \
                     so silence here is not a clean result. {mawdi}. Set Steam's folder in \
                     Settings, under Launchers, and try again."
                )
            },
            Self::ShabakaBilaIqrar { wasf_injilizi, .. } => format!(
                "This is a multiplayer game, and the modification risk has to be acknowledged \
                 before the run starts. Modifying a game played with other people can cost you \
                 your account or your access to its servers, and that decision is yours alone. \
                 Nothing has been spent.\n{wasf_injilizi}"
            ),
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => format!(
                "The registry ({masdar}) answered at {waqt} but did not serve its revocation \
                 list ({sabab}), so the run did not start and nothing was spent. While the \
                 registry is reachable no patch is installed until its list can be read, \
                 because a withdrawn patch is known only from it. Try again shortly; if the \
                 source is a local folder, make sure the list file is in it."
            ),
            Self::IqrarNaqis => "The first-run statement has not been acknowledged, and no \
                 patch is installed until it is. The statement is on this screen now: read it, \
                 accept it, and start again. The run has not begun and nothing has been spent: \
                 it is asked here, before extraction and translation, because answering it \
                 needs no patch at all."
                .to_owned(),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MashwarJari { .. } | Self::LaMashwar { .. } | Self::LaIstinaf { .. } => {
                Khutwa::AadaMuhawala
            },
            Self::LaKhattArabi => Khutwa::FathIdadat {
                qism: QismIdadat::Khutut,
            },
            Self::LubaBilaMasdar { .. } => Khutwa::AadaFahsMaktaba,
            // Three refusals with no button, for three different reasons and
            // one shared rule: none of them may be clicked past. The
            // publisher's Arabic is lifted only by a setting whose whole point
            // is that it is chosen deliberately, elsewhere, rather than on the
            // screen that just refused. The multiplayer one is lifted by an
            // acknowledgement the person gives, and an action offering to give
            // it would be giving it for them. Anti-cheat is lifted by nothing
            // at all: no override for it exists anywhere in this product, and
            // this would be the first one. Each sentence names its own way out,
            // or says plainly that there is none.
            //
            // The first-run statement joins them for the same reason: it lives
            // on the game's screen and has to be read there, so a button that
            // accepted it from here would be accepting it on the reader's
            // behalf.
            Self::LughaRasmiya { .. }
            | Self::HimayaMuktashafa { .. }
            | Self::ShabakaBilaIqrar { .. }
            | Self::IqrarNaqis => Khutwa::LaShay,
            // The one refusal here with a mechanical remedy: the ceiling is a
            // field on a screen, so send the reader straight to it.
            Self::BilaSaqfInfaq { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Muzawwidun,
            },
            // The manual Steam path is the one way out, and it is the same one
            // `KhataLuba::JidhrSteamMajhul` sends the reader to.
            Self::FahsHimayaLamYajri { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Manassat,
            },
            // Lifted by the next refresh that finds the list, which the next
            // press runs; nothing on this machine is wrong.
            Self::QaimatSahbMahjuba { .. } => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> std::collections::BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = std::collections::BTreeMap::new();
        match self {
            Self::MashwarJari { ism }
            | Self::LaMashwar { ism }
            | Self::LaIstinaf { ism }
            | Self::LubaBilaMasdar { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::LughaRasmiya {
                ism, hala_injilizi, ..
            } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("lugha".to_owned(), QeemaSiyaq::Nass(hala_injilizi.clone()));
            },
            Self::BilaSaqfInfaq { muzawwid } => {
                let _ = siyaq.insert("muzawwid".to_owned(), QeemaSiyaq::Nass(muzawwid.clone()));
            },
            Self::HimayaMuktashafa { ism, anwa, .. } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("himaya".to_owned(), QeemaSiyaq::Nass(anwa.clone()));
            },
            Self::FahsHimayaLamYajri { ism, mawdi, sabab } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
                if let Some(mawdi) = mawdi {
                    let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(mawdi.clone()));
                }
            },
            Self::ShabakaBilaIqrar {
                ism, wasf_injilizi, ..
            } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert(
                    "shabaka".to_owned(),
                    QeemaSiyaq::Nass(wasf_injilizi.clone()),
                );
            },
            Self::LaKhattArabi | Self::IqrarNaqis => {},
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => {
                let _ = siyaq.insert("masdar".to_owned(), QeemaSiyaq::Nass(masdar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
                let _ = siyaq.insert("waqt".to_owned(), QeemaSiyaq::Nass(waqt.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataTilqaiAmr);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ikhtibarat {
    use taarib_makhzan::sijillat::{IdkhalLuba, SijillAlaab, SijillMuharrik, SimaMukhzana};
    use taarib_mustalahat::muharrik::{JahiziyatTashghil, KhalfiyaBarmajiya, Muharrik};
    use taarib_usus::manassa::Mimariya;

    use super::*;

    /// What every test here answers with, so a fixture that could not be built
    /// — a directory, a file, a store — propagates with `?` beside the
    /// product's own [`Khata`] instead of being unwrapped. `unwrap` and
    /// `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// The readiness refusal's code, as `taarib_tilqai::khata` allocates it.
    const RAMZ_GHAYR_JAHIZ: u16 = arqam::TILQAI + 13;

    /// A clause only `taarib_tilqai::naqs_jahiziya` writes.
    ///
    /// The tier's own reason already says the words "not finished in this
    /// build" whenever readiness is `ghaiba`, so testing for those would pass
    /// on a verdict where this gate did nothing at all. This clause belongs to
    /// the refusal and to nothing else.
    const ATHAR_RAFD: &str = "the one-button run is not offered here";

    /// The code for "no machine-translation provider is configured".
    ///
    /// No longer reachable from a default settings value: the free endpoint is
    /// built in, so a fresh installation always has a provider. It stays because
    /// several tests below assert the run did **not** stop here, and that is
    /// still worth proving.
    const RAMZ_LA_MUZAWWID: u16 = arqam::STUDIO + 42;

    /// The code for "no font on this machine can carry Arabic", which is the
    /// first thing `jahhiz` refuses on once the gate has let a game past.
    ///
    /// It took the provider refusal's place in that role, and the reason is a
    /// product change rather than a test convenience: the provider check comes
    /// first in `jahhiz` and cannot fail any more, because the built-in free
    /// endpoint is always there. A run that stops *here* is a run the gate let
    /// past, which is what these tests are for — and it needs no provider, no
    /// keychain and nobody's money to prove.
    const RAMZ_LA_KHATT: u16 = arqam::STUDIO + 123;

    /// The safety layer's own refusal code, as `taarib_tilqai::khata` allocates
    /// it. The most serious of the three and the one no release lifts.
    const RAMZ_TABAQA: u16 = arqam::TILQAI + 2;

    /// The missing-ceiling refusal's code, which `hukm` now warns about ahead of
    /// the press that would raise it.
    const RAMZ_BILA_SAQF: u16 = arqam::STUDIO + 126;

    /// The door's own anti-cheat refusal, raised from the disk-and-catalogue
    /// scan rather than from the launcher hint the stored report carries.
    const RAMZ_HIMAYA_BAB: u16 = arqam::STUDIO + 127;

    /// The door's refusal when Steam's catalogue was owed and unreadable.
    const RAMZ_FAHS_LAM_YAJRI: u16 = arqam::STUDIO + 128;

    /// The door's multiplayer refusal, which the acknowledgement lifts.
    const RAMZ_SHABAKA: u16 = arqam::STUDIO + 129;

    /// The run's refusal when the registry answers and withholds its
    /// revocation list — the twin of the manual path's `9031`.
    const RAMZ_SAHB_MAHJUBA: u16 = arqam::STUDIO + 137;

    /// A Steam application identifier no real catalogue can hold.
    ///
    /// The door now opens `appinfo.vdf` for every Steam game, and these tests
    /// run on developer machines with a real Steam and a real, enormous
    /// catalogue. A live application id would make what the scan finds depend on
    /// what Valve happens to publish about that title today; this one is beyond
    /// anything Steam has issued, so a real catalogue answers exactly what the
    /// fabricated empty one answers — read, and silent about this app.
    const TATBEEQ_WAHMI: u32 = 4_000_000_000;

    /// A clause only a refused report's own sentences carry, in both the tier's
    /// reason — which is what `hukm` shows — and the first limit, which is what
    /// `taarib_tilqai::tahaqquq_jahiziya` shows. Asserting on the clause the two
    /// share is what makes an agreement test about agreement rather than about
    /// one wording.
    const ATHAR_HIMAYA: &str = "protected by an anti-cheat system (VAC)";

    /// A clause only `tabbiq_lugha_rasmiya`'s own refusal writes.
    const ATHAR_LUGHA: &str = "replaces paid human work";

    /// A scratch data root that removes itself, so an assertion that fails does
    /// not leave a database behind in the machine's temporary directory.
    struct JidhrMuaqqat(PathBuf);

    impl Drop for JidhrMuaqqat {
        fn drop(&mut self) {
            // Best effort. A test that has already failed must not fail twice,
            // and there is nothing useful to do with an error here.
            #[expect(
                clippy::disallowed_methods,
                reason = "a scratch directory under `std::env::temp_dir()` removing itself, never a data root or a game directory"
            )]
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A data root, a store and a settings value, with one Unity game in them.
    struct Masrah {
        masarat: Masarat,
        makhzan: Makhzan,
        idadat: Arc<MakhzanIdadat>,
        id: LubaId,
        /// Held for its [`Drop`]; nothing reads it. Last so that it runs after
        /// the store's connection pool has closed — on Windows a directory
        /// holding an open database file will not delete.
        _jidhr: JidhrMuaqqat,
    }

    /// Sets one up, with the game's stored capability report carrying
    /// `jahiziya`.
    ///
    /// The report goes in through `SijillMuharrik` exactly as a library scan
    /// writes one, so both commands read it back through their own
    /// `taqreer_luba` rather than through anything this helper hands them —
    /// which is what makes these tests about the gate and not about a mock.
    fn masrah(jahiziya: JahiziyatTashghil) -> NatijatIkhtibar<Masrah> {
        masrah_bi(jahiziya, &[], &[])
    }

    /// The engine every fixture below is built on.
    ///
    /// IL2CPP, not Mono. Unity on Mono has been watched drawing Arabic in a
    /// running game, so its arm answers `mukammala` and carries no gap sentence
    /// — and a fixture that forced `ghaiba` onto it produced a refusal with
    /// nothing to say, which is a report `imkaniyat` would never write. IL2CPP is
    /// the backend whose in-game half really has not been watched, so the
    /// refusing direction is the table's own answer in the table's own words.
    ///
    /// Shared with the tests rather than built inside the fixture, because one
    /// of them asserts on the sentence the readiness table gives *this* engine
    /// and must not quote it from here.
    fn muharrik_masrah() -> Muharrik {
        Muharrik {
            aila: AilatMuharrik::Unity,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Il2cpp,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 95,
            dalail: Vec::new(),
        }
    }

    /// A game folder, some launcher hints, and a stored report, all at once.
    ///
    /// `simat` reaches both halves of the record a real scan writes: the game
    /// row, which is what `hukm` reads for its acknowledgement question, and
    /// `imkaniyat::taqreer`, which is the only thing that sets `marfuda`. So a
    /// refused fixture is refused for the report's own reason, in the report's
    /// own sentences, rather than in ones this file invented. Nothing here
    /// claims a real game runs anti-cheat or is multiplayer; the hints and the
    /// planted files are the fixture's.
    ///
    /// `milaffat` are named empty files placed in the game folder, which is what
    /// the door's scan actually walks. The folder is created even when the list
    /// is empty, so the clean case proves a directory that was walked and found
    /// bare rather than one the scan could not open.
    fn masrah_bi(
        jahiziya: JahiziyatTashghil,
        simat: &[SimatLuba],
        milaffat: &[&str],
    ) -> NatijatIkhtibar<Masrah> {
        let jidhr = std::env::temp_dir().join(format!("taarib-tilqai-{}", uuid::Uuid::new_v4()));
        let haris = JidhrMuaqqat(jidhr.clone());
        let masarat = Masarat::min_judhur(jidhr.join("bayanat"), jidhr.join("idadat"));
        // Creates the data root on the way: the store makes its own parent.
        let makhzan = Makhzan::min_masar(&masarat.qaida_bayanat())?;

        let jidhr_luba = jidhr.join("luba");
        std::fs::create_dir_all(&jidhr_luba)?;
        for ism in milaffat {
            std::fs::write(jidhr_luba.join(ism), b"fixture")?;
        }

        // The first-run statement, accepted. Every fixture below represents a
        // machine somebody is already using, and on such a machine the
        // statement was read at first launch — a fixture that had not accepted
        // it would be testing the door's refusal rather than what is past it.
        iqrar::ahfaz(
            &crate::tathbeet_awamir::masar_iqrar(&masarat),
            "2026-01-01T00:00:00Z".to_owned(),
            ISDAR.to_owned(),
        )?;

        let masdar = MasdarLuba::Steam(TATBEEQ_WAHMI);
        let luba = Luba {
            id: LubaId::min_masdar(&masdar, "Luba Ikhtibar"),
            masadir: vec![masdar],
            ism: "Luba Ikhtibar".to_owned(),
            jidhr: jidhr_luba,
            tanfidhi: None,
            hajm: 0,
            akhir_laab: None,
            akhir_tahdith: None,
            bina: None,
            suwar: taarib_mustalahat::luba::SuwarLuba::default(),
            beea: BeeatTawafuq::Asli,
            mawjuda: true,
            mukhfiya: false,
        };
        let id = luba.id;
        let mukhzana: Vec<SimaMukhzana> = simat.iter().filter_map(sima_mukhzana).collect();
        makhzan.bi_muamala(|muamala| {
            SijillAlaab::jadeed(muamala).sajjil(&IdkhalLuba {
                luba: &luba,
                muktamila: true,
                khiyarat_tashghil: None,
                simat: &mukhzana,
                fahs: 1,
            })
        })?;

        let muharrik = muharrik_masrah();
        let mut taqreer =
            taarib_muharrik::imkaniyat::taqreer(muharrik, simat, "2026-01-01T00:00:00Z".to_owned());
        // Forced rather than probed for the open direction only: this build's
        // IL2CPP arm answers `ghaiba`, and a fixture cannot wait for a backend to
        // be finished. Everything downstream of the field — the gate, the
        // sentence, both commands — is the production path.
        //
        // One artefact of forcing it: `sabab_*` was already written with the
        // `ghaiba` preamble in front, so a `mukammala` fixture carries a reason
        // no real report would. Nothing here asserts on that string; the
        // refusal's own clause is what these tests look for.
        //
        // A refused report carries no readiness sentence by contract, so forcing
        // the field on one would produce a report `imkaniyat` cannot write.
        if !taqreer.marfuda {
            taqreer.jahiziya = jahiziya;
            if jahiziya == JahiziyatTashghil::Mukammala {
                taqreer.naqs = None;
            }
        }
        makhzan.bi_muamala(|muamala| SijillMuharrik::jadeed(muamala).sajjil(id, &taqreer, None))?;

        // Offline, with no local copy: the run's first step refreshes the
        // revocation list from whatever sources the settings name, and a test
        // must never reach the real forge.
        let mut qeema = Idadat::default();
        qeema.masadir.wadaa_ghayr_muttasil = true;
        let idadat = Arc::new(MakhzanIdadat::min_qeema(masarat.malaf_idadat(), qeema));
        Ok(Masrah {
            masarat,
            makhzan,
            idadat,
            id,
            _jidhr: haris,
        })
    }

    /// One launcher hint in the shape the store keeps it in.
    ///
    /// The store holds hints as rows, and `luba_awamir::simat_min_makhzan` reads
    /// them back into `SimatLuba`. Writing them the long way round means `hukm`
    /// re-derives the fixture's hints through the production reader rather than
    /// being handed a list this file made up. Only the hints these tests use are
    /// spelled out; the rest answer [`None`] rather than a wrong row.
    fn sima_mukhzana(sima: &SimatLuba) -> Option<SimaMukhzana> {
        use taarib_makhzan::sijillat::sima;

        let naw = match sima {
            SimatLuba::JamaiMahalli => sima::JAMAI_MAHALLI,
            SimatLuba::JamaiOnline => sima::JAMAI_ONLINE,
            SimatLuba::MuammanaVac => sima::MUAMMANA_VAC,
            _ => return None,
        };
        Some(SimaMukhzana::jadeeda("steam", naw, ""))
    }

    /// A snapshot sink that drops everything, for a run that never starts.
    fn mudhee_samit() -> MudheeLaqta {
        Arc::new(|_laqta: &LaqtatTilqaiHie| {})
    }

    /// The verdict refuses an engine with no working in-game half, and names
    /// both the engine and what is missing, in both languages.
    #[test]
    fn hukm_yarfud_muharrikan_ghayr_jahiz() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Ghaiba)?;
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
        )?;

        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(hukm.sabab_arabi.contains("Unity"), "{}", hukm.sabab_arabi);
        assert!(
            hukm.sabab_injilizi.contains("Unity"),
            "{}",
            hukm.sabab_injilizi
        );
        assert!(
            hukm.sabab_injilizi.contains(ATHAR_RAFD),
            "{}",
            hukm.sabab_injilizi
        );
        // The limits list still carries the report's own account of the gap, so
        // nothing the user could have read before the gate existed is lost.
        //
        // Read out of the readiness table rather than quoted here. The phrase
        // this used to look for — "not finished in this build" — belongs to
        // `naqs_jahiziya`'s wrapper and reaches `sabab_*`, never the limits, so
        // the assertion passed on the wrapper and proved nothing about the list.
        let jumla = taarib_muharrik::imkaniyat::jahiziya(&muharrik_masrah())
            .1
            .map_or_else(String::new, |naqs| naqs.injilizi);
        assert!(
            !jumla.is_empty(),
            "the fixture's engine carries no sentence"
        );
        assert!(
            hukm.hudud_injilizi.iter().any(|hadd| hadd == &jumla),
            "{:?}",
            hukm.hudud_injilizi
        );
        // A string count beside a refusal reads as an invitation.
        assert!(hukm.nusus_taqribi.is_none());
        Ok(())
    }

    /// The verdict offers the run for an engine whose in-game half is finished.
    #[test]
    fn hukm_yaarid_muharrikan_jahizan() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
        )?;

        assert_eq!(hukm.masar, MasarTilqaiHie::Tilqai);
        assert!(
            !hukm.sabab_injilizi.contains(ATHAR_RAFD),
            "{}",
            hukm.sabab_injilizi
        );
        Ok(())
    }

    /// The start command refuses the same engine the verdict refused. The
    /// verdict is advice; this is the call that would spend money and write
    /// into somebody's game, and it enforces on its own.
    #[test]
    fn ibda_yarfud_muharrikan_ghayr_jahiz() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Ghaiba)?;
        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_eq!(ramz, Some(Ramz::jadeed(RAMZ_GHAYR_JAHIZ)));
        Ok(())
    }

    /// The refusal `luba_awamir` raises when a Steam game's Steam is nowhere to
    /// be found.
    const RAMZ_JIDHR_STEAM: u16 = arqam::STUDIO + 13;

    /// A settings store over the fixture's value with the Steam override set to
    /// a directory that really is one, holding a catalogue that really reads.
    ///
    /// A second store rather than a write through the first: the run only reads
    /// settings, and staging them on disk would be testing the settings writer.
    /// The override is what makes the Steam-root gate answer the same on a
    /// machine with Steam and on one without, which is the only way the tests
    /// past that gate stay about what they were written to be about.
    fn idadat_bi_steam(masrah: &Masrah) -> NatijatIkhtibar<Arc<MakhzanIdadat>> {
        let tajawuz = jidhr_steam_wahmi(masrah);
        std::fs::create_dir_all(tajawuz.join("steamapps"))?;
        ansha_fahras(&tajawuz)?;
        Ok(idadat_bi_tajawuz(masrah, tajawuz))
    }

    /// Where the fabricated Steam lives for one fixture.
    fn jidhr_steam_wahmi(masrah: &Masrah) -> PathBuf {
        masrah
            .masarat
            .jidhr_bayanat()
            .parent()
            .map_or_else(|| PathBuf::from("steam"), |jidhr| jidhr.join("steam"))
    }

    /// The same settings value with one Steam override written into it.
    fn idadat_bi_tajawuz(masrah: &Masrah, tajawuz: PathBuf) -> Arc<MakhzanIdadat> {
        let mut qeema = (*masrah.idadat.hali()).clone();
        qeema.manassat.steam = Some(tajawuz);
        Arc::new(MakhzanIdadat::min_qeema(
            masrah.masarat.malaf_idadat(),
            qeema,
        ))
    }

    /// A catalogue that is valid, readable, and names no applications.
    ///
    /// Twelve bytes: the `0x07564427` magic, the universe, and the zero
    /// application id that ends the entry list — which is exactly how a real
    /// `appinfo.vdf` opens and closes. The door's anti-cheat scan refuses a
    /// Steam game whose catalogue it could not read, and it is right to: VAC is
    /// declared there and nowhere else. So "we looked and Steam said nothing"
    /// has to be a file the tests can actually produce, or every test past that
    /// gate would be passing for the wrong reason.
    fn ansha_fahras(jidhr_steam: &Path) -> std::io::Result<()> {
        let appcache = jidhr_steam.join("appcache");
        std::fs::create_dir_all(&appcache)?;
        let mut bayt: Vec<u8> = Vec::with_capacity(12);
        bayt.extend(0x0756_4427_u32.to_le_bytes());
        bayt.extend(1_u32.to_le_bytes());
        bayt.extend(0_u32.to_le_bytes());
        std::fs::write(appcache.join("appinfo.vdf"), bayt)
    }

    /// The first-run statement is asked at the door, not at the install.
    ///
    /// It used to be the install gate's alone, and the install gate is stage 7
    /// of 7: a person who had never seen the statement paid for a whole
    /// translation and was then refused over a tick on another screen. The
    /// refusal has to arrive before the run starts, and it has to be this
    /// refusal rather than a later one.
    #[test]
    fn ibda_yasal_an_al_iqrar_ala_al_bab() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        // Undo what the fixture accepted, which is the whole point of this one.
        std::fs::remove_file(crate::tathbeet_awamir::masar_iqrar(&masrah.masarat))?;
        let idadat = idadat_bi_steam(&masrah)?;

        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_eq!(ramz, Some(Ramz::jadeed(arqam::STUDIO + 138)));
        Ok(())
    }

    /// The start command lets a ready engine through the gate. It still stops,
    /// on the provider this machine does not have — and stopping *there* is the
    /// proof, because that check sits after the gate rather than before it.
    #[test]
    fn ibda_yamurr_muharrikan_jahizan() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let idadat = idadat_bi_steam(&masrah)?;
        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_ne!(ramz, Some(Ramz::jadeed(RAMZ_GHAYR_JAHIZ)));
        assert_eq!(ramz, Some(Ramz::jadeed(RAMZ_LA_KHATT)));
        Ok(())
    }

    /// The one-button run resolves Steam for itself instead of reading the
    /// user's override, and refuses rather than handing the safety gate nothing.
    ///
    /// The fixture sets no override. Before this, `jahhiz` copied that empty
    /// value straight into the run's safety inputs and the install gate at the
    /// end of the run read no Steam catalogue at all — which is how a
    /// VAC-secured game reached a clean verdict. The expected stop is derived
    /// from the same resolver the command uses rather than guessed from the
    /// platform, so what is asserted is the property — the gate's answer follows
    /// the resolver, and is never a silent pass — on a machine with Steam and on
    /// one without alike.
    #[test]
    fn ibda_yahull_jidhr_steam_bila_tajawuz() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        assert!(
            masrah.idadat.hali().manassat.steam.is_none(),
            "the fixture must carry no override, or this proves nothing"
        );
        let mutawaqqa =
            if crate::luba_awamir::jidhr_steam(&masrah.masarat, &masrah.idadat.hali())?.is_some() {
                RAMZ_LA_KHATT
            } else {
                RAMZ_JIDHR_STEAM
            };

        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_eq!(ramz, Some(Ramz::jadeed(mutawaqqa)));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // The door: the two refusals that used to arrive after the money
    // -----------------------------------------------------------------------

    /// A file name whose stem is a networking marker `kashf_shabaka` knows.
    ///
    /// Mirror is a Unity networking library, and a file called this in a game
    /// folder is exactly the evidence the scan looks for. It is a fixture, not a
    /// claim about any installed title.
    const MALAF_SHABAKA: &str = "Mirror.dll";

    /// The same for the anti-cheat scan: an Easy Anti-Cheat payload's own name.
    const MALAF_HIMAYA: &str = "EasyAntiCheat.dll";

    /// The verdict asks for the multiplayer acknowledgement before the button,
    /// off the launcher hint the library scan already stored.
    ///
    /// This is the half that makes the acknowledgement obtainable at all. The
    /// run's own gate needs a `yes` or a `no`; the interface can only ask for
    /// one if the verdict tells it to, and the verdict may not walk a game
    /// folder to find out. The launcher's own catalogue entry is the one source
    /// that is already in the store.
    #[test]
    fn hukm_yasal_an_iqrar_al_shabaka_qabl_al_zirr() -> NatijatIkhtibar {
        let jamai = masrah_bi(JahiziyatTashghil::Mukammala, &[SimatLuba::JamaiOnline], &[])?;
        let hukm_jamai = hukm(
            jamai.id.to_string(),
            &jamai.masarat,
            &jamai.makhzan,
            &jamai.idadat,
        )?;
        assert_eq!(hukm_jamai.masar, MasarTilqaiHie::Tilqai);
        assert!(hukm_jamai.yalzam_iqrar_shabaka);

        // A shared-screen title counts too, because the gate it warns about
        // answers on any evidence at all rather than on the online kind.
        let mahalli = masrah_bi(
            JahiziyatTashghil::Mukammala,
            &[SimatLuba::JamaiMahalli],
            &[],
        )?;
        let hukm_mahalli = hukm(
            mahalli.id.to_string(),
            &mahalli.masarat,
            &mahalli.makhzan,
            &mahalli.idadat,
        )?;
        assert!(hukm_mahalli.yalzam_iqrar_shabaka);

        let wahid = masrah(JahiziyatTashghil::Mukammala)?;
        let hukm_wahid = hukm(
            wahid.id.to_string(),
            &wahid.masarat,
            &wahid.makhzan,
            &wahid.idadat,
        )?;
        assert!(!hukm_wahid.yalzam_iqrar_shabaka);
        Ok(())
    }

    /// A refused game is not asked to acknowledge anything: there is no run to
    /// acknowledge, and a tick box on a screen with no button is the same
    /// mistake as printing a string count beside a refusal.
    #[test]
    fn hukm_la_yasal_an_iqrar_ala_luba_marfuda() -> NatijatIkhtibar {
        let masrah = masrah_bi(JahiziyatTashghil::Ghaiba, &[SimatLuba::JamaiOnline], &[])?;
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
        )?;

        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(!hukm.yalzam_iqrar_shabaka);
        Ok(())
    }

    /// The multiplayer refusal arrives at the door, before a provider exists.
    ///
    /// This is the defect in one test. The acknowledgement was hard-coded to
    /// `false` in the run's own safety inputs, so the install gate — stage 7 of
    /// 7 — refused every multiplayer game with a sentence nothing could satisfy,
    /// and refused it only after extraction, the whole paid translation, the
    /// atlas, the compile and the seal had all been done. Stopping on
    /// `RAMZ_SHABAKA` rather than on `RAMZ_LA_MUZAWWID` is the proof: the
    /// provider is the very next thing `jahhiz` asks for, so a run that has not
    /// reached it has not priced a single string.
    #[test]
    fn ibda_yarfud_al_shabaka_ala_al_bab() -> NatijatIkhtibar {
        let masrah = masrah_bi(JahiziyatTashghil::Mukammala, &[], &[MALAF_SHABAKA])?;
        let idadat = idadat_bi_steam(&masrah)?;
        let khata = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .ok_or("a multiplayer game with no acknowledgement must not start")?;

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_SHABAKA));
        assert_ne!(khata.ramz, Ramz::jadeed(RAMZ_LA_MUZAWWID));
        // The refusal carries the scan's own evidence, so the interface can show
        // what is being acknowledged rather than asking for a blank yes.
        assert!(khata.injilizi.contains("Mirror"), "{}", khata.injilizi);
        assert!(
            khata.injilizi.contains("Nothing has been spent"),
            "{}",
            khata.injilizi
        );
        assert!(!khata.arabi.is_empty());
        Ok(())
    }

    /// The same game starts once the person has acknowledged it, and stops on
    /// the provider this machine does not have — which is the gate immediately
    /// after the door, and therefore the proof the door opened.
    #[test]
    fn al_iqrar_yaftah_bawwabat_al_shabaka() -> NatijatIkhtibar {
        let masrah = masrah_bi(JahiziyatTashghil::Mukammala, &[], &[MALAF_SHABAKA])?;
        let idadat = idadat_bi_steam(&masrah)?;
        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            true,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_eq!(ramz, Some(Ramz::jadeed(RAMZ_LA_KHATT)));
        Ok(())
    }

    /// The anti-cheat scan refuses at the door too, and the acknowledgement does
    /// not touch it.
    ///
    /// Passing `true` for the multiplayer question is deliberate: there is no
    /// acknowledgement for anti-cheat anywhere in this product, and a test that
    /// left the box unticked could not tell the two refusals apart. A VAC or EAC
    /// ban is permanent and attaches to the account, so this refusal has no
    /// override and must not acquire one by accident.
    #[test]
    fn ibda_yarfud_al_himaya_ala_al_bab() -> NatijatIkhtibar {
        let masrah = masrah_bi(JahiziyatTashghil::Mukammala, &[], &[MALAF_HIMAYA])?;
        let idadat = idadat_bi_steam(&masrah)?;
        let khata = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            true,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .ok_or("an anti-cheat payload on disk must not start a run")?;

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_HIMAYA_BAB));
        assert_ne!(khata.ramz, Ramz::jadeed(RAMZ_LA_MUZAWWID));
        assert!(
            khata.injilizi.contains("Easy Anti-Cheat"),
            "{}",
            khata.injilizi
        );
        assert!(
            khata.injilizi.contains("Nothing was spent"),
            "{}",
            khata.injilizi
        );
        assert!(!khata.arabi.is_empty());
        Ok(())
    }

    /// A Steam game whose catalogue cannot be read is refused at the door,
    /// rather than translated and then refused at the end.
    ///
    /// The override points at a real directory with no `appcache/appinfo.vdf` in
    /// it, which is what a wrong Steam path looks like by the time it reaches
    /// this layer. VAC is declared in that file and leaves nothing whatever in a
    /// game folder, so the empty evidence list this produces is byte-identical
    /// to a genuinely clean game's — and reading it as a pass is how a
    /// VAC-secured title would get through.
    #[test]
    fn ibda_yarfud_fahrasan_la_yuqra() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let tajawuz = jidhr_steam_wahmi(&masrah);
        std::fs::create_dir_all(tajawuz.join("steamapps"))?;
        let idadat = idadat_bi_tajawuz(&masrah, tajawuz);
        let khata = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            true,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .ok_or("an unreadable catalogue is not a passed check")?;

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_FAHS_LAM_YAJRI));
        assert_ne!(khata.ramz, Ramz::jadeed(RAMZ_LA_MUZAWWID));
        // The refusal names the path, because on this machine the path is the
        // mistake, and it says what to set.
        assert!(khata.injilizi.contains("appinfo.vdf"), "{}", khata.injilizi);
        Ok(())
    }

    /// Whether a sentence carries any Arabic script at all.
    ///
    /// Local to this module, as the same predicate is in `luba_awamir` and
    /// `tathbeet_awamir`: each is a private `#[cfg(test)]` module, and a shared
    /// one would have to be a non-test item compiled into the shipping binary.
    fn fiha_arabi(nass: &str) -> bool {
        nass.chars()
            .any(|harf| matches!(harf, '\u{0600}'..='\u{06ff}' | '\u{0750}'..='\u{077f}'))
    }

    /// The withheld-list refusal carries its own code, its own remedy and both
    /// languages, and shares neither with the run's other refusals.
    #[test]
    fn rafd_al_sahb_lahu_ramz_wa_makhraj_yakhussanihi() {
        let khata = Khata::min_tafsir(&KhataTilqaiAmr::QaimatSahbMahjuba {
            masdar: "forge https://example.invalid".to_owned(),
            sabab: "sahb/qaima.json answered 404".to_owned(),
            waqt: "2026-09-06T12:00:00Z".to_owned(),
        });

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_SAHB_MAHJUBA));
        for ramz in [
            RAMZ_HIMAYA_BAB,
            RAMZ_FAHS_LAM_YAJRI,
            RAMZ_SHABAKA,
            RAMZ_BILA_SAQF,
        ] {
            assert_ne!(khata.ramz, Ramz::jadeed(ramz));
        }
        // Nothing on this machine is wrong; the next press refreshes the list.
        assert_eq!(khata.khutwa, Khutwa::AadaMuhawala);
        assert_eq!(khata.khutura, Khutura::Tanbeeh);
        assert!(khata.injilizi.contains("404"), "{}", khata.injilizi);
        // The run's refusals all say what was and was not spent.
        assert!(
            khata.injilizi.contains("nothing was spent"),
            "{}",
            khata.injilizi
        );
        assert!(fiha_arabi(&khata.arabi));
        assert!(!fiha_arabi(&khata.injilizi), "{}", khata.injilizi);
        assert_eq!(
            khata.siyaq.get("sabab"),
            Some(&QeemaSiyaq::Nass("sahb/qaima.json answered 404".to_owned()))
        );
    }

    // -----------------------------------------------------------------------
    // The three refusals, and the order both commands take them in
    // -----------------------------------------------------------------------

    /// A verdict as `hukm` hands it to the two exclusions, with nothing
    /// excluded yet: the route the tier alone implies, and no refusal sentence.
    fn hukm_maftuh() -> HukmTilqaiHie {
        HukmTilqaiHie {
            muarrif: "ikhtibar".to_owned(),
            ism: "Luba Ikhtibar".to_owned(),
            masar: MasarTilqaiHie::Tilqai,
            tabaqa_raqm: 1,
            tabaqa_arabi: "كامل".to_owned(),
            tabaqa_injilizi: "full".to_owned(),
            sabab_arabi: "سبب الطبقة".to_owned(),
            sabab_injilizi: "the tier's own reason".to_owned(),
            hudud_arabi: Vec::new(),
            hudud_injilizi: Vec::new(),
            nusus_taqribi: Some(4_000),
            yalzam_iqrar_shabaka: false,
            sahb: HalatSahbHie {
                hala: "lam_tujlab".to_owned(),
                muhaddatha: false,
                tasalsul: 1,
                adad: 0,
                arabi: "لم تُجلب قائمة".to_owned(),
                injilizi: "no list was fetched".to_owned(),
            },
            takalif: TakalifHie {
                munfaq: 0.0,
                saqf: 0.0,
                umla: UMLA.to_owned(),
            },
            ghilaf: None,
        }
    }

    /// A stored official-Arabic verdict that says the publisher ships all of it.
    fn rasmiya_kamila() -> taarib_mustalahat::luba::HukmLughaRasmiya {
        taarib_mustalahat::luba::HukmLughaRasmiya {
            wajiha: taarib_mustalahat::luba::TughtiyaLugha::Muakkada,
            nusus: taarib_mustalahat::luba::TughtiyaLugha::Muakkada,
            thiqa: 95,
            dalail: Vec::new(),
            majhul: Vec::new(),
            lughat_muallana: vec!["Arabic".to_owned()],
            isdar_fahs: 1,
            waqt: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    /// A capability report for a Unity game with the readiness verdict forced,
    /// and the safety layer's refusal reached through its own trait.
    fn taqreer_bi(jahiziya: JahiziyatTashghil, marfuda: bool) -> TaqreerImkaniyat {
        let muharrik = Muharrik {
            aila: AilatMuharrik::Unity,
            isdar: None,
            khalfiya: KhalfiyaBarmajiya::Mono,
            itarat: Vec::new(),
            rusum: Vec::new(),
            mimariya: Mimariya::X8664,
            thiqa: 95,
            dalail: Vec::new(),
        };
        let simat: Vec<SimatLuba> = if marfuda {
            vec![SimatLuba::MuammanaVac]
        } else {
            Vec::new()
        };
        let mut taqreer = taarib_muharrik::imkaniyat::taqreer(
            muharrik,
            &simat,
            "2026-01-01T00:00:00Z".to_owned(),
        );
        if !marfuda {
            taqreer.jahiziya = jahiziya;
        }
        taqreer
    }

    /// Both exclusions, applied in the order `hukm` applies them.
    fn tabbiq_al_ithnayn(
        hukm: HukmTilqaiHie,
        taqreer: &TaqreerImkaniyat,
        rasmiya: Option<&taarib_mustalahat::luba::HukmLughaRasmiya>,
        istibdal: bool,
    ) -> HukmTilqaiHie {
        tabbiq_jahiziya(tabbiq_lugha_rasmiya(hukm, rasmiya, istibdal), taqreer)
    }

    /// When both hold, the sentence a user reads is the publisher's Arabic and
    /// not the readiness gap.
    ///
    /// Both are true of the same game today — every arm of the readiness table
    /// answers `ghaiba` in this build — so this is not a corner case, it is
    /// every game with official Arabic. The one that survives has to be the one
    /// the reader can act on: `ibda` raises `LughaRasmiya` for this game, and a
    /// verdict naming a different reason than the button does is the disagreement
    /// this ordering exists to prevent.
    #[test]
    fn al_lugha_al_rasmiya_taghlib_al_jahiziya() {
        let taqreer = taqreer_bi(JahiziyatTashghil::Ghaiba, false);
        let rasmiya = rasmiya_kamila();
        let hukm = tabbiq_al_ithnayn(hukm_maftuh(), &taqreer, Some(&rasmiya), false);

        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(
            hukm.sabab_injilizi.contains(ATHAR_LUGHA),
            "{}",
            hukm.sabab_injilizi
        );
        assert!(
            !hukm.sabab_injilizi.contains(ATHAR_RAFD),
            "{}",
            hukm.sabab_injilizi
        );
        // Outranked is not hidden: the report's own account of the gap is what
        // the limits list carries, and `hukm` appends it before either
        // exclusion runs.
        assert!(hukm.nusus_taqribi.is_none());
    }

    /// The safety layer's refusal outranks the publisher's Arabic, which is the
    /// order that was already right and must stay right after the swap.
    #[test]
    fn al_marfuda_taghlib_al_lugha_al_rasmiya() {
        let taqreer = taqreer_bi(JahiziyatTashghil::Ghaiba, true);
        let rasmiya = rasmiya_kamila();
        let mut hukm = hukm_maftuh();
        hukm.masar = masar_hukm(&taqreer);
        hukm.sabab_injilizi.clone_from(&taqreer.sabab_injilizi);
        hukm.sabab_arabi.clone_from(&taqreer.sabab_arabi);
        let hukm = tabbiq_al_ithnayn(hukm, &taqreer, Some(&rasmiya), false);

        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(
            hukm.sabab_injilizi.contains(ATHAR_HIMAYA),
            "{}",
            hukm.sabab_injilizi
        );
        assert!(
            !hukm.sabab_injilizi.contains(ATHAR_LUGHA),
            "{}",
            hukm.sabab_injilizi
        );
        // The other two facts are still stated, as limits.
        assert!(
            hukm.hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains("already ships Arabic")),
            "{:?}",
            hukm.hudud_injilizi
        );
    }

    /// A game with nothing against it but an unready engine still reads the
    /// readiness sentence: the swap must not have made that refusal unreachable.
    #[test]
    fn al_jahiziya_tabqa_hiya_al_sabab_wahdaha() {
        let taqreer = taqreer_bi(JahiziyatTashghil::Ghaiba, false);
        let hukm = tabbiq_al_ithnayn(hukm_maftuh(), &taqreer, None, false);

        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(
            hukm.sabab_injilizi.contains(ATHAR_RAFD),
            "{}",
            hukm.sabab_injilizi
        );
    }

    /// The start command takes the safety layer's refusal first, and the
    /// verdict reports the same one.
    #[test]
    fn hukm_wa_ibda_yattafiqan_ala_al_marfuda() -> NatijatIkhtibar {
        let masrah = masrah_bi(JahiziyatTashghil::Ghaiba, &[SimatLuba::MuammanaVac], &[])?;
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
        )?;
        assert_eq!(hukm.masar, MasarTilqaiHie::Marfud);
        assert!(
            hukm.sabab_injilizi.contains(ATHAR_HIMAYA),
            "{}",
            hukm.sabab_injilizi
        );
        assert!(
            !hukm.sabab_injilizi.contains(ATHAR_RAFD),
            "{}",
            hukm.sabab_injilizi
        );

        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);
        assert_eq!(ramz, Some(Ramz::jadeed(RAMZ_TABAQA)));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // The spend ceiling, as the verdict reports it
    // -----------------------------------------------------------------------

    /// A settings value electing one enabled provider.
    fn idadat_bi_muzawwid(
        masrah: &Masrah,
        naw: NawMuzawwid,
        mizaniya: Option<f64>,
    ) -> Arc<MakhzanIdadat> {
        let mut qeema = (*masrah.idadat.hali()).clone();
        qeema.muzawwidun.qaima = vec![taarib_usus::idadat::IdadatMuzawwid {
            muarrif: "muzawwid-ikhtibar".to_owned(),
            naw,
            namudhaj: "namudhaj".to_owned(),
            asas: None,
            hisab_miftah: None,
            mufaal: true,
            hadd_talabat: 60,
            mizaniya,
        }];
        qeema.muzawwidun.iftiradi = Some("muzawwid-ikhtibar".to_owned());
        Arc::new(MakhzanIdadat::min_qeema(
            masrah.masarat.malaf_idadat(),
            qeema,
        ))
    }

    /// A clause only the missing-ceiling limit and the refusal itself carry.
    const ATHAR_SAQF: &str = "charges for translation and has no spend ceiling set";

    /// The verdict warns about the ceiling the start command will refuse over.
    ///
    /// The screen's cost sentence is "the run stops at the ceiling and never
    /// crosses it", and it is handed `saqf` — which is zero for exactly this
    /// configuration. Without the limit the verdict promises a ceiling that does
    /// not exist and the button then refuses with a code the reader had no
    /// warning of, which is a button offered that then refuses.
    #[test]
    fn hukm_yunabbih_ila_ghiyab_saqf_al_infaq() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let idadat = idadat_bi_muzawwid(&masrah, NawMuzawwid::Anthropic, None);
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
        )?;

        assert!(
            hukm.hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains(ATHAR_SAQF)),
            "{:?}",
            hukm.hudud_injilizi
        );
        assert!(
            hukm.hudud_arabi
                .iter()
                .any(|hadd| hadd.contains("سقف إنفاق")),
            "{:?}",
            hukm.hudud_arabi
        );
        // The warning is about the refusal `jahhiz` raises, so the two have to
        // be about the same configuration; this is the code it raises.
        assert_eq!(
            Khata::from(KhataTilqaiAmr::BilaSaqfInfaq {
                muzawwid: "muzawwid-ikhtibar".to_owned()
            })
            .ramz,
            Ramz::jadeed(RAMZ_BILA_SAQF)
        );
        assert!(
            hukm.takalif.saqf.abs() < f64::EPSILON,
            "{}",
            hukm.takalif.saqf
        );
        Ok(())
    }

    /// A ceiling that is set is not warned about, and is the number shown.
    #[test]
    fn hukm_la_yunabbih_ala_saqf_mawdu() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let idadat = idadat_bi_muzawwid(&masrah, NawMuzawwid::Anthropic, Some(5.0));
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
        )?;

        assert!(
            !hukm
                .hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains(ATHAR_SAQF)),
            "{:?}",
            hukm.hudud_injilizi
        );
        assert!(
            (hukm.takalif.saqf - 5.0).abs() < f64::EPSILON,
            "{}",
            hukm.takalif.saqf
        );
        Ok(())
    }

    /// A free provider is not warned about either, and that is the half that
    /// matters: a gate nobody can pass is indistinguishable from a broken
    /// product, and the loopback path is the one that completes a whole run for
    /// nothing. `jahhiz` exempts it for the same reason — there is no ceiling to
    /// set when there is nothing to spend.
    #[test]
    fn hukm_la_yunabbih_ala_al_mahalli() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let idadat = idadat_bi_muzawwid(&masrah, NawMuzawwid::Mahalli, None);
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
        )?;

        assert!(
            !hukm
                .hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains(ATHAR_SAQF)),
            "{:?}",
            hukm.hudud_injilizi
        );
        assert!(yatqada_ajran(NawMuzawwid::Anthropic));
        assert!(!yatqada_ajran(NawMuzawwid::Mahalli));
        assert!(!yatqada_ajran(NawMuzawwid::GoogleMajjani));
        Ok(())
    }

    /// A machine with no provider of its own gets a verdict, not a refusal: the
    /// built-in free provider is named in the limits, in the settings crate's own
    /// sentence, and no spend ceiling is demanded of a service that bills nothing.
    #[test]
    fn hukm_yusammi_almajjani_ind_ghiyab_almuzawwid() -> NatijatIkhtibar {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        assert!(
            masrah.idadat.hali().muzawwidun.qaima.is_empty(),
            "the stage is a fresh installation's settings"
        );
        let hukm = hukm(
            masrah.id.to_string(),
            &masrah.masarat,
            &masrah.makhzan,
            &masrah.idadat,
        )?;

        assert!(
            hukm.hudud_injilizi
                .iter()
                .any(|hadd| hadd == HalatMuzawwidin::Faragh.injilizi()),
            "{:?}",
            hukm.hudud_injilizi
        );
        assert!(
            hukm.hudud_arabi
                .iter()
                .any(|hadd| hadd == HalatMuzawwidin::Faragh.arabi()),
            "{:?}",
            hukm.hudud_arabi
        );
        assert!(
            !hukm
                .hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains(ATHAR_SAQF)),
            "{:?}",
            hukm.hudud_injilizi
        );
        assert!(hukm.takalif.saqf.abs() < f64::EPSILON);
        Ok(())
    }
}
