//! التعريب التلقائي — the command layer over `taarib-tilqai`: the verdict, the run, and the stop.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use taarib_aman::iqrar::{self, SijillIqrar};
use taarib_aman::qaimat_sahb::QaimatSahb;
use taarib_istikhraj::rafd::TaqreerRafd;
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
    HalatMashwar, IhsaIstikhraj, KhiyaratTilqai, LubaTilqai, MarhalaTilqai, MashwarId,
    MiqbadIlgha, MudkhalatAman, MukhbirTaqaddum, NatijatMashwar, QaydMarhala, SijillMashwar,
    TalabTilqai, Taqaddum, WasfTilqai, arrib, ijrud, naqs_jahiziya, tahaqquq_jahiziya,
};
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::manassa::{BeeatTawafuq, NizamTashghil};
use taarib_usus::masarat::Masarat;
use taarib_usus::ISDAR;
use tauri::Emitter as _;

use crate::luba_awamir::{appid_steam, huwiya, ijlib_luba, simat_luba, taqreer_luba};
use crate::warsha_awamir::{
    bin_muzawwid, dolar, lahza_alaan, muzawwid_muntakhab, nano_min_dolar,
};

/// The window event every run snapshot is published on.
pub const ISM_HADATH_TILQAI: &str = "taarib://tilqai";

/// Where automatic runs keep their directories, under the data root.
const MUJALLAD_TILQAI: &str = "tilqai";

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
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
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
    /// What it has cost so far and what it may cost.
    pub takalif: TakalifHie,
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

    let mut hudud_arabi: Vec<String> =
        taqreer.hudud.iter().map(|hadd| hadd.arabi.clone()).collect();
    let mut hudud_injilizi: Vec<String> =
        taqreer.hudud.iter().map(|hadd| hadd.injilizi.clone()).collect();
    // What this build cannot deliver yet is not a limitation of the game, and
    // the capability report keeps the two apart for exactly that reason. The
    // verdict has one list, and a user deciding whether to press the button
    // needs to be told either way — so it is appended, not dropped.
    if let Some(naqs) = &taqreer.naqs {
        hudud_arabi.push(naqs.arabi.clone());
        hudud_injilizi.push(naqs.injilizi.clone());
    }
    let tarif = muzawwid_muntakhab(&hali);
    if tarif.is_err() {
        hudud_arabi.push(
            "لا يوجد مزوّد ترجمة آلية مفعّل على هذا الجهاز، والتعريب التلقائي لا يبدأ بدونه. \
             أضف مزوّدًا من الإعدادات ← المزوّدون."
                .to_owned(),
        );
        hudud_injilizi.push(
            "No machine-translation provider is enabled on this machine, and an automatic run \
             cannot start without one. Add one in Settings, under Providers."
                .to_owned(),
        );
    }
    let saqf = tarif
        .as_ref()
        .ok()
        .and_then(|tarif| tarif.mizaniya)
        .filter(|mablagh| mablagh.is_finite() && *mablagh > 0.0)
        .unwrap_or(0.0);

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
    // Readiness first, because the official-language one is written to defer to
    // a refusal that is already standing and would otherwise be the one that
    // defers. Between the two, the publisher's own Arabic is the sentence a user
    // should end up reading: it is a permanent reason not to run and this is a
    // temporary one.
    let hukm = tabbiq_jahiziya(hukm, &taqreer);
    Ok(tabbiq_lugha_rasmiya(
        hukm,
        crate::luba_awamir::hukm_mukhazzan(id).as_ref(),
        hali.istibdal_lugha_rasmiya,
    ))
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
/// becomes the refusal, and the button is simply not offered. The one thing
/// this does not do is override a game the safety layer already refused —
/// `naqs_jahiziya` answers [`None`] for those, because an anti-cheat association
/// is the more serious of the two and is not undone by an update.
fn tabbiq_jahiziya(mut hukm: HukmTilqaiHie, taqreer: &TaqreerImkaniyat) -> HukmTilqaiHie {
    let Some(sabab) = naqs_jahiziya(taqreer) else {
        return hukm;
    };
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
        hukm.hudud_arabi
            .insert(0, format!("الناشر يشحن عربية رسمية مع هذه اللعبة ({}).", hala.ism_arabi()));
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
        QaydMarhala::Takhtit {
            khutut, azwaj, ..
        } => {
            let adad = raqm_u32(khutut.len().try_into().unwrap_or(u64::MAX));
            (
                MarhalatTilqaiHie::Takhtit,
                u64::from(adad),
                Some(u64::from(adad)),
                format!("{adad} font(s) validated, {azwaj} layout(s) to compute"),
            )
        },
        QaydMarhala::Tarqee {
            nusus, safahat, ..
        } => (
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
    let makhzun: RafdFaqat =
        match serde_json::from_reader(std::io::BufReader::new(malaf)) {
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
/// # Errors
///
/// [`KhataTilqaiAmr::MashwarJari`] when a run for this game is already going,
/// [`KhataTilqaiAmr::LughaRasmiya`] when the publisher already ships Arabic,
/// `taarib_tilqai::KhataTilqai::MuharrikGhayrJahiz` when this build has no
/// working in-game half for the detected engine and
/// `taarib_tilqai::KhataTilqai::TabaqaGhayrMadauma` when the safety layer
/// refuses the game — both from the same gate the verdict reports,
/// [`crate::luba_awamir::KhataLuba::JidhrSteamMajhul`] when the game is a Steam
/// game and Steam's own root cannot be found, so the install gate this run ends
/// at could not read the catalogue VAC is declared in,
/// [`crate::warsha_awamir::KhataWarshaAmr::LaMuzawwid`] when no provider is
/// configured, [`KhataTilqaiAmr::LaKhattArabi`] when no font on this machine can
/// carry Arabic, [`KhataTilqaiAmr::LaIstinaf`] when a resume was asked for and
/// there is nothing to resume, and whatever the store, the keychain and the
/// acknowledgement record raise.
#[tauri::command]
#[specta::specta]
pub fn ibda_tilqai(
    nafidha: tauri::Window,
    muarrif: String,
    istinaf: bool,
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
pub(crate) fn ibda(
    mudhee: MudheeLaqta,
    muarrif: String,
    istinaf: bool,
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &Arc<MakhzanIdadat>,
    mashawir: &MashawirTilqai,
) -> Natija<LaqtatTilqaiHie> {
    let id = huwiya(muarrif)?;
    // The same exclusion the verdict applies, enforced where the money is
    // actually spent. A gate that only the read path honours is not a gate: the
    // command is reachable on its own, and this is the call that writes into
    // somebody's game.
    if let Some(hala) = mahmiya_bil_lugha(id, &idadat.hali()) {
        return Err(Khata::from(KhataTilqaiAmr::LughaRasmiya {
            ism: id.to_string(),
            hala_arabi: hala.ism_arabi().to_owned(),
            hala_injilizi: hala.ism_injilizi().to_owned(),
        }));
    }
    if mashawir
        .wahid(id)
        .is_some_and(|hay| hay.hala.lock().laqta.wad == WadTilqaiHie::Jariya)
    {
        return Err(Khata::from(KhataTilqaiAmr::MashwarJari {
            ism: id.to_string(),
        }));
    }

    // The same gate `hukm` reports, enforced where the run actually starts, and
    // for the same reason the exclusion above is enforced here: a verdict is
    // advice. This command is reachable on its own, a click can race the verdict
    // it was drawn from, and a build finishing between the two changes the
    // answer. It sits after the two checks that need no I/O and before `jahhiz`,
    // so an unpatchable engine is refused without a provider being built, a font
    // being copied or a signing key being unlocked — and the report itself is the
    // one the whole product already reads, cached in the store.
    //
    // It is `tahaqquq_jahiziya` rather than `naqs_jahiziya` because a start
    // command has to refuse an anti-cheat-refused game too, and that refusal has
    // a sentence of its own which is not this one's to overwrite.
    let luba = ijlib_luba(makhzan, id)?;
    let simat = simat_luba(makhzan, id)?;
    tahaqquq_jahiziya(&taqreer_luba(makhzan, &luba, &simat, false)?)?;

    let mudkhalat = jahhiz(masarat, makhzan, idadat, id, istinaf)?;
    // A resumed run starts from what the journal already knows rather than from
    // five blank rows: the stages it will skip are finished, and a list that
    // showed them as waiting would tell the user their four thousand translated
    // strings are about to be done again.
    let sabiqa = istinaf
        .then(|| laqta_min_qurs(masarat, &idadat.hali(), id))
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
    drop(tauri::async_runtime::spawn(shaghghil(mudhee, hay, mudkhalat)));
    Ok(laqta)
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
pub(crate) fn alghi(
    muarrif: String,
    mashawir: &MashawirTilqai,
) -> Natija<LaqtatTilqaiHie> {
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
    /// The verified revocation list.
    qaima: QaimatSahb,
    /// The first-run acknowledgement, when one was given.
    iqrar: Option<SijillIqrar>,
    /// The Steam application id, when this is a Steam game.
    appid: Option<u32>,
    /// Steam's install root, resolved the way the library scan resolves it —
    /// registry, then the known install directories, with the settings override
    /// winning when there is one. [`None`] only for a game that is not on Steam;
    /// a Steam game with no resolvable root never gets this far.
    jidhr_steam: Option<PathBuf>,
    /// How the run behaves.
    khiyarat: KhiyaratTilqai,
    /// The ceiling, in whole currency units, for the snapshot.
    saqf_dolar: f64,
    /// What previous runs of this game already spent, in nano-dollars.
    munfaq_sabiq: u64,
}

/// Assembles everything one run needs, refusing by name when a piece is missing.
fn jahhiz(
    masarat: &Masarat,
    makhzan: &Makhzan,
    idadat: &Arc<MakhzanIdadat>,
    id: LubaId,
    istinaf: bool,
) -> Natija<MudkhalatMashwar> {
    let luba = ijlib_luba(makhzan, id)?;
    let masdar = luba.masadir.first().cloned().ok_or_else(|| {
        Khata::from(KhataTilqaiAmr::LubaBilaMasdar {
            ism: luba.ism.clone(),
        })
    })?;
    let hali = idadat.hali();
    // Before a provider is chosen and before a single string is priced. The run
    // ends at the same install gate the one-click path ends at, that gate reads
    // VAC out of Steam's catalogue and out of nothing else, and a run that spent
    // the user's money translating a game it was never going to be allowed to
    // touch is a worse answer than a refusal at the door.
    let jidhr_steam = crate::luba_awamir::jidhr_steam_lil_fahs(masarat, &hali, &luba)?;
    let tarif = muzawwid_muntakhab(&hali)?;
    let lahza = lahza_alaan();
    let saqf_nano = tarif.mizaniya.and_then(|mablagh| nano_min_dolar(mablagh).ok());
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
    let qaima = crate::tathbeet_awamir::qaimat_sahb()?;
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
        iqrar,
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
fn jidhr_mashawir(masarat: &Masarat, id: LubaId) -> PathBuf {
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
    judhur.extend(crate::mukawwinat_tahmil::judhur_khutut(masarat).into_iter().skip(1));

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
async fn shaghghil(mudhee: MudheeLaqta, hay: Arc<MashwarHay>, mudkhalat: MudkhalatMashwar) {
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
        wasf: mudkhalat.wasf.clone(),
        aman: MudkhalatAman {
            mirsa: &mudkhalat.mirsa,
            qaima: &mudkhalat.qaima,
            iqrar: mudkhalat.iqrar.as_ref(),
            appid: mudkhalat.appid,
            jidhr_steam: mudkhalat.jidhr_steam.as_deref(),
            // Never asserted here. The multiplayer acknowledgement is a sentence
            // a person agrees to on the install screen, and a command layer that
            // set it would be agreeing on their behalf; the gate refuses the
            // install instead, by name, and the refusal says what to do.
            iqrar_shabaka: false,
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
                saf.majmu = if fahs { None } else { taqaddum.majmu.map(raqm_u32) };
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
        .ok()
        .and_then(|tarif| tarif.mizaniya)
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
                },
        )
    }

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
            Self::LughaRasmiya { .. } => Khutura::Maluma,
            Self::LaKhattArabi | Self::LubaBilaMasdar { .. } | Self::BilaSaqfInfaq { .. } => {
                Khutura::Tanbeeh
            },
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
            // Deliberately no action. The one route out is a setting whose whole
            // point is that it is chosen deliberately rather than clicked past
            // on the screen that just refused, and the sentence names it.
            Self::LughaRasmiya { .. } => Khutwa::LaShay,
            // The one refusal here with a mechanical remedy: the ceiling is a
            // field on a screen, so send the reader straight to it.
            Self::BilaSaqfInfaq { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Muzawwidun,
            },
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
            Self::LughaRasmiya { ism, hala_injilizi, .. } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq
                    .insert("lugha".to_owned(), QeemaSiyaq::Nass(hala_injilizi.clone()));
            },
            Self::BilaSaqfInfaq { muzawwid } => {
                let _ = siyaq.insert("muzawwid".to_owned(), QeemaSiyaq::Nass(muzawwid.clone()));
            },
            Self::LaKhattArabi => {},
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
    use taarib_makhzan::sijillat::{IdkhalLuba, SijillAlaab, SijillMuharrik};
    use taarib_mustalahat::muharrik::{JahiziyatTashghil, KhalfiyaBarmajiya, Muharrik};
    use taarib_usus::manassa::Mimariya;

    use super::*;

    /// The readiness refusal's code, as `taarib_tilqai::khata` allocates it.
    const RAMZ_GHAYR_JAHIZ: u16 = arqam::TILQAI + 13;

    /// A clause only `taarib_tilqai::naqs_jahiziya` writes.
    ///
    /// The tier's own reason already says the words "not finished in this
    /// build" whenever readiness is `ghaiba`, so testing for those would pass
    /// on a verdict where this gate did nothing at all. This clause belongs to
    /// the refusal and to nothing else.
    const ATHAR_RAFD: &str = "the one-button run is not offered here";

    /// The code for "no machine-translation provider is configured", which is
    /// the next thing `jahhiz` asks for after the gate. A run that stops here
    /// is a run the gate let past, which is the only way to prove the open
    /// direction without a provider, a keychain and somebody's money.
    const RAMZ_LA_MUZAWWID: u16 = arqam::STUDIO + 42;

    /// A scratch data root that removes itself, so an assertion that fails does
    /// not leave a database behind in the machine's temporary directory.
    struct JidhrMuaqqat(PathBuf);

    impl Drop for JidhrMuaqqat {
        fn drop(&mut self) {
            // Best effort. A test that has already failed must not fail twice,
            // and there is nothing useful to do with an error here.
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
    fn masrah(jahiziya: JahiziyatTashghil) -> Natija<Masrah> {
        let jidhr = std::env::temp_dir().join(format!("taarib-tilqai-{}", uuid::Uuid::new_v4()));
        let haris = JidhrMuaqqat(jidhr.clone());
        let masarat = Masarat::min_judhur(jidhr.join("bayanat"), jidhr.join("idadat"));
        // Creates the data root on the way: the store makes its own parent.
        let makhzan = Makhzan::min_masar(&masarat.qaida_bayanat())?;

        let masdar = MasdarLuba::Steam(480);
        let luba = Luba {
            id: LubaId::min_masdar(&masdar, "Luba Ikhtibar"),
            masadir: vec![masdar],
            ism: "Luba Ikhtibar".to_owned(),
            jidhr: jidhr.join("luba"),
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
        makhzan.bi_muamala(|muamala| {
            SijillAlaab::jadeed(muamala).sajjil(&IdkhalLuba {
                luba: &luba,
                muktamila: true,
                khiyarat_tashghil: None,
                simat: &[],
                fahs: 1,
            })
        })?;

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
        let mut taqreer =
            taarib_muharrik::imkaniyat::taqreer(muharrik, &[], "2026-01-01T00:00:00Z".to_owned());
        // Forced rather than probed, because every arm of the real readiness
        // table answers `ghaiba` in this build and the open direction would
        // otherwise be untestable. Everything downstream of the field — the
        // gate, the sentence, both commands — is the production path.
        //
        // One artefact of forcing it: `sabab_*` was already written with the
        // `ghaiba` preamble in front, so a `mukammala` fixture carries a reason
        // no real report would. Nothing here asserts on that string; the
        // refusal's own clause is what these tests look for.
        taqreer.jahiziya = jahiziya;
        if jahiziya == JahiziyatTashghil::Mukammala {
            taqreer.naqs = None;
        }
        makhzan.bi_muamala(|muamala| SijillMuharrik::jadeed(muamala).sajjil(id, &taqreer, None))?;

        let idadat = Arc::new(MakhzanIdadat::min_qeema(
            masarat.malaf_idadat(),
            Idadat::default(),
        ));
        Ok(Masrah {
            masarat,
            makhzan,
            idadat,
            id,
            _jidhr: haris,
        })
    }

    /// A snapshot sink that drops everything, for a run that never starts.
    fn mudhee_samit() -> MudheeLaqta {
        Arc::new(|_laqta: &LaqtatTilqaiHie| {})
    }

    /// The verdict refuses an engine with no working in-game half, and names
    /// both the engine and what is missing, in both languages.
    #[test]
    fn hukm_yarfud_muharrikan_ghayr_jahiz() -> Natija<()> {
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
        assert!(
            hukm.hudud_injilizi
                .iter()
                .any(|hadd| hadd.contains("not finished in this build")),
            "{:?}",
            hukm.hudud_injilizi
        );
        // A string count beside a refusal reads as an invitation.
        assert!(hukm.nusus_taqribi.is_none());
        Ok(())
    }

    /// The verdict offers the run for an engine whose in-game half is finished.
    #[test]
    fn hukm_yaarid_muharrikan_jahizan() -> Natija<()> {
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
    fn ibda_yarfud_muharrikan_ghayr_jahiz() -> Natija<()> {
        let masrah = masrah(JahiziyatTashghil::Ghaiba)?;
        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
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
    /// a directory that really is one.
    ///
    /// A second store rather than a write through the first: the run only reads
    /// settings, and staging them on disk would be testing the settings writer.
    /// The override is what makes the Steam-root gate answer the same on a
    /// machine with Steam and on one without, which is the only way the tests
    /// past that gate stay about what they were written to be about.
    fn idadat_bi_steam(masrah: &Masrah) -> std::io::Result<Arc<MakhzanIdadat>> {
        let tajawuz = masrah
            .masarat
            .jidhr_bayanat()
            .parent()
            .map_or_else(|| PathBuf::from("steam"), |jidhr| jidhr.join("steam"));
        std::fs::create_dir_all(tajawuz.join("steamapps"))?;
        let mut qeema = (*masrah.idadat.hali()).clone();
        qeema.manassat.steam = Some(tajawuz);
        Ok(Arc::new(MakhzanIdadat::min_qeema(masrah.masarat.malaf_idadat(), qeema)))
    }

    /// The start command lets a ready engine through the gate. It still stops,
    /// on the provider this machine does not have — and stopping *there* is the
    /// proof, because that check sits after the gate rather than before it.
    #[test]
    fn ibda_yamurr_muharrikan_jahizan() -> Result<(), Box<dyn std::error::Error>> {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        let idadat = idadat_bi_steam(&masrah)?;
        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
            false,
            &masrah.masarat,
            &masrah.makhzan,
            &idadat,
            &MashawirTilqai::default(),
        )
        .err()
        .map(|khata| khata.ramz);

        assert_ne!(ramz, Some(Ramz::jadeed(RAMZ_GHAYR_JAHIZ)));
        assert_eq!(ramz, Some(Ramz::jadeed(RAMZ_LA_MUZAWWID)));
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
    fn ibda_yahull_jidhr_steam_bila_tajawuz() -> Natija<()> {
        let masrah = masrah(JahiziyatTashghil::Mukammala)?;
        assert!(
            masrah.idadat.hali().manassat.steam.is_none(),
            "the fixture must carry no override, or this proves nothing"
        );
        let mutawaqqa =
            if crate::luba_awamir::jidhr_steam(&masrah.masarat, &masrah.idadat.hali())?.is_some() {
                RAMZ_LA_MUZAWWID
            } else {
                RAMZ_JIDHR_STEAM
            };

        let ramz = ibda(
            mudhee_samit(),
            masrah.id.to_string(),
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
}
