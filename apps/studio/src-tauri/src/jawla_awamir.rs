//! الجولة — the library probing itself.
//!
//! ## Why a sweep
//!
//! The engine probe used to run only when a person opened a game's screen. On a
//! real machine that left most of the library unexamined for ever: four games
//! with a capability report and fifteen cards saying "not probed yet", so the
//! library could not say which product any of them gets. This module runs the
//! same probe over every game the game screen has not reached, after every
//! completed scan, on a task of its own.
//!
//! ## What is examined
//!
//! Every visible, present game whose report is missing or was written by an
//! older probe than [`ISDAR_FAHS`] — the two conditions
//! [`luba_awamir::taqreer_luba`] itself re-probes on, and that function is the
//! one this module calls, so the sweep and the game screen cannot write two
//! kinds of report. The probe is *asked for*, never forced: a game the screen
//! examined while the sweep was queued is served from the store when its turn
//! comes, and costs nothing twice.
//!
//! ## Bounds
//!
//! Two games at a time, each on a blocking thread — the probe reads executables
//! and walks directories, and none of that may share the thread the window is
//! painted from. One sweep at a time: a scan that finishes while one is running
//! does not start a second, it leaves a request for another round, and that
//! round selects afresh so it covers only what is still stale. Nothing here can
//! fail a scan: the scan's answer is already on its way when the sweep is
//! claimed, and a game the probe refuses is one entry in the sweep's own
//! record — logged under the game's name, and carried in the final event.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use parking_lot::Mutex;
use taarib_makhzan::sijillat::{SijillAlaab, SijillMuharrik, SijillRuqaa, TalabMaktaba};
use taarib_makhzan::wasl::Makhzan;
use taarib_muharrik::ISDAR_FAHS;
use taarib_mustalahat::luba::{HalatLuba, Luba, LubaId};
use taarib_mustalahat::muharrik::{AilatMuharrik, JahiziyatTashghil, Tabaqa};
use taarib_tathbeet::bayan::NawTathbeet;
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::masarat::Masarat;
use tauri::Emitter as _;
use tokio::task::JoinSet;

use crate::luba_awamir;

/// The window event one game's finished probe is announced on.
///
/// The payload is the game's identity and exactly the fields its library row
/// derives from the capability report, so the grid patches one row in place
/// rather than asking for the library again.
pub const ISM_HADATH_FAHS_MUHARRIK: &str = "taarib://fahs-muharrik";

/// The window event the sweep's own standing is announced on.
///
/// Once when a round starts, once after every game, and once when the sweep
/// ends — that last one with `jariya` false and every failure it met carried
/// by name.
pub const ISM_HADATH_JAWLA: &str = "taarib://jawla-muharrik";

/// How many games are probed at once.
///
/// Two, because a probe is disk-bound — it reads executables and walks the
/// install root — and two of them already keep a laptop drive busy without
/// starving the scan that just finished or the artwork pass running beside
/// them.
pub(crate) const HADD_MUTAWAZI: usize = 2;

// ---------------------------------------------------------------------------
// The wire
// ---------------------------------------------------------------------------

/// One game's finished probe, as the library row reads it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FahsMuharrikHie {
    /// Taarib's identity for the game, matching the library row's `muarrif`.
    pub muarrif: String,
    /// The name its launcher gives it, so a log line and a notice can name it.
    pub ism: String,
    /// Whether the probe has examined this game — always true for a game this
    /// event names, and sent anyway so the row's own flag is overwritten rather
    /// than inferred by the interface.
    pub mafhusa: bool,
    /// The identified engine family.
    pub muharrik: AilatMuharrik,
    /// The tier the report awards.
    pub tabaqa: Tabaqa,
    /// Whether this build can drive that tier on that engine.
    pub jahiziya: JahiziyatTashghil,
    /// The Arabization status the card badges, decided from the new report and
    /// the same install facts the scan reads, by the same rule.
    pub hala: HalatLuba,
}

/// One game the sweep could not probe.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct FashalJawlaHie {
    /// Taarib's identity for the game.
    pub muarrif: String,
    /// Its name.
    pub ism: String,
    /// What the probe raised, whole, so the code and the next step survive the
    /// crossing.
    pub khata: Khata,
}

/// Where the sweep stands.
///
/// The answer of [`halat_jawla`] and the payload of [`ISM_HADATH_JAWLA`] are
/// one shape, so a screen that mounts mid-sweep and a screen that watched it
/// start hold the same record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HalatJawlaHie {
    /// Whether a sweep is running.
    pub jariya: bool,
    /// How many games this round has probed.
    pub tamma: u32,
    /// How many games this round is over.
    pub majmu: u32,
    /// How many games this round could not probe.
    pub fashila: u32,
    /// Every game this round could not probe, in the order they failed.
    pub akhta: Vec<FashalJawlaHie>,
    /// A failure of the sweep itself rather than of one game — the store would
    /// not say which games are owed a probe — when there was one.
    pub khata: Option<Khata>,
}

// ---------------------------------------------------------------------------
// The one sweep
// ---------------------------------------------------------------------------

/// The counters, and whether another round is owed.
#[derive(Debug, Default)]
struct DakhilJawla {
    /// Whether a sweep holds the claim.
    jariya: bool,
    /// Whether a scan finished while it ran, so another round follows it.
    muallaqa: bool,
    /// Probed this round.
    tamma: u32,
    /// Selected this round.
    majmu: u32,
    /// Refused this round.
    akhta: Vec<FashalJawlaHie>,
    /// This round's own failure, when the selection could not be read.
    khata: Option<Khata>,
}

/// The one sweep this process may be running, and its counters.
///
/// Managed state, shared with the task that runs the sweep through an `Arc`.
/// The claim is the whole of the idempotence rule: [`Self::ihjiz`] hands the
/// sweep to exactly one caller, and a second caller leaves a request behind
/// that the running sweep honours by selecting again once it is done.
#[derive(Debug, Default)]
pub struct HalatJawla(Mutex<DakhilJawla>);

impl HalatJawla {
    /// Claims the right to run a sweep.
    ///
    /// `true` when nothing was running and the caller now owns the sweep;
    /// `false` when one is in flight, in which case another round is recorded
    /// and the running sweep will run it after its own.
    #[must_use]
    pub(crate) fn ihjiz(&self) -> bool {
        let mut dakhil = self.0.lock();
        if dakhil.jariya {
            dakhil.muallaqa = true;
            return false;
        }
        dakhil.jariya = true;
        dakhil.muallaqa = false;
        true
    }

    /// Opens a round over `majmu` games, with the counters at zero.
    fn ibda_dawra(&self, majmu: u32, khata: Option<Khata>) {
        let mut dakhil = self.0.lock();
        dakhil.tamma = 0;
        dakhil.majmu = majmu;
        dakhil.akhta.clear();
        dakhil.khata = khata;
    }

    /// Counts one probed game.
    fn sajjil_najah(&self) {
        let mut dakhil = self.0.lock();
        dakhil.tamma = dakhil.tamma.saturating_add(1);
    }

    /// Records one game the probe refused.
    fn sajjil_fashal(&self, fashal: FashalJawlaHie) {
        self.0.lock().akhta.push(fashal);
    }

    /// Ends the round the caller owns.
    ///
    /// `true` when another round was requested while it ran — the caller keeps
    /// the claim and runs it; `false` when the claim is released.
    fn anhi(&self) -> bool {
        let mut dakhil = self.0.lock();
        if dakhil.muallaqa {
            dakhil.muallaqa = false;
            return true;
        }
        dakhil.jariya = false;
        false
    }

    /// A snapshot for the wire.
    #[must_use]
    pub fn hie(&self) -> HalatJawlaHie {
        let dakhil = self.0.lock();
        HalatJawlaHie {
            jariya: dakhil.jariya,
            tamma: dakhil.tamma,
            majmu: dakhil.majmu,
            fashila: u32::try_from(dakhil.akhta.len()).unwrap_or(u32::MAX),
            akhta: dakhil.akhta.clone(),
            khata: dakhil.khata.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Which games, and what their rows say
// ---------------------------------------------------------------------------

/// One game the sweep will probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HadafJawla {
    /// Taarib's identity for it.
    pub(crate) id: LubaId,
    /// Its name, for the log and the failure record.
    pub(crate) ism: String,
}

/// Which games a round examines, in library order.
///
/// A visible, present game whose report is missing, or whose report an older
/// probe than `isdar_hali` wrote. A report from a *newer* probe is not stale:
/// it is what a build one version ahead left behind, and re-probing it would
/// overwrite a better answer with a worse one. Hidden and absent games are
/// excluded even though the store's own listing already excludes them, because
/// the rule belongs here where it is tested and not in a query's flags.
#[must_use]
pub(crate) fn mustahiqqa(
    alaab: &[Luba],
    isdarat: &BTreeMap<LubaId, u32>,
    isdar_hali: u32,
) -> Vec<HadafJawla> {
    alaab
        .iter()
        .filter(|luba| luba.mawjuda && !luba.mukhfiya)
        .filter(|luba| {
            isdarat
                .get(&luba.id)
                .is_none_or(|&isdar| isdar < isdar_hali)
        })
        .map(|luba| HadafJawla {
            id: luba.id,
            ism: luba.ism.clone(),
        })
        .collect()
}

/// The facts the library's Arabization badge is decided from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HaqaiqHala {
    /// Whether the safety layer refuses the game outright.
    pub(crate) marfuda: bool,
    /// Whether a text patch is installed.
    pub(crate) nass_muthabbat: bool,
    /// Whether the registry offers at least one patch for the game.
    pub(crate) ruqaa_mutaha: bool,
    /// The tier the report awards.
    pub(crate) tabaqa: Tabaqa,
}

/// The Arabization status a library row badges.
///
/// One derivation for the row the scan builds and the row the sweep patches
/// in place, so the badge a scan draws and the badge a probe event redraws
/// cannot be decided by two rules that drift apart.
#[must_use]
pub(crate) const fn halat_luba(haqaiq: HaqaiqHala) -> HalatLuba {
    if haqaiq.marfuda {
        HalatLuba::Marfuda
    } else if haqaiq.nass_muthabbat {
        HalatLuba::Mutabbaqa
    } else if haqaiq.ruqaa_mutaha {
        HalatLuba::Mutaha
    } else if matches!(haqaiq.tabaqa, Tabaqa::TarjamaFawqiya) {
        HalatLuba::TabaqaFaqat
    } else {
        HalatLuba::MadumBilaRuqaa
    }
}

// ---------------------------------------------------------------------------
// The sweep's world
// ---------------------------------------------------------------------------

/// What a sweep needs from the world: which games to examine, how to examine
/// one, and where to announce what happened.
///
/// A port rather than three closures, so the Studio's implementation — the
/// store, the probe, the window — and the test double that stands in for them
/// are the same shape, and the guard and the bound are tested against the real
/// runner rather than against a copy of it.
pub(crate) trait BeeatJawla: Send + Sync + 'static {
    /// The games still owed a probe, selected afresh for each round.
    ///
    /// # Errors
    ///
    /// Whatever reading the library raises.
    fn mustahiqqa(&self) -> Natija<Vec<HadafJawla>>;

    /// Probes one game and answers with what its library row derives from the
    /// new report. Called on a blocking thread.
    ///
    /// # Errors
    ///
    /// Whatever the probe or the store raise for that one game.
    fn ifhas(&self, hadaf: &HadafJawla) -> Natija<FahsMuharrikHie>;

    /// Announces one finished game.
    fn aalin_fahs(&self, fahs: &FahsMuharrikHie);

    /// Announces where the sweep stands.
    fn aalin_hala(&self, hala: &HalatJawlaHie);
}

/// Runs the sweep the caller claimed with [`HalatJawla::ihjiz`], round after
/// round, until no further round is owed.
pub(crate) async fn shaghghil<B: BeeatJawla>(halat: Arc<HalatJawla>, bee: Arc<B>) {
    loop {
        let (ahdaf, khata) = ikhtar(&bee).await;
        halat.ibda_dawra(u32::try_from(ahdaf.len()).unwrap_or(u32::MAX), khata);
        bee.aalin_hala(&halat.hie());
        tracing::info!(adad = ahdaf.len(), "the engine sweep started a round");

        let adad_ummal = HADD_MUTAWAZI.min(ahdaf.len());
        let tabur = Arc::new(Mutex::new(VecDeque::from(ahdaf)));
        let mut ummal: JoinSet<()> = JoinSet::new();
        for _ in 0..adad_ummal {
            let _ = ummal.spawn(amil(
                Arc::clone(&halat),
                Arc::clone(&bee),
                Arc::clone(&tabur),
            ));
        }
        while let Some(intiha) = ummal.join_next().await {
            if let Err(sabab) = intiha {
                tracing::warn!(sabab = %sabab, "an engine sweep worker did not finish");
            }
        }

        let yuad = halat.anhi();
        let hie = halat.hie();
        tracing::info!(
            tamma = hie.tamma,
            fashila = hie.fashila,
            majmu = hie.majmu,
            yuad,
            "the engine sweep ended a round"
        );
        bee.aalin_hala(&hie);
        if !yuad {
            break;
        }
    }
}

/// The games this round examines, or none and the reason when the store would
/// not say.
///
/// The reason travels with the round rather than ending it: a sweep that
/// cannot read the library has nothing to fail but itself, and the final event
/// is where the interface learns that.
async fn ikhtar<B: BeeatJawla>(bee: &Arc<B>) -> (Vec<HadafJawla>, Option<Khata>) {
    let natija = tokio::task::spawn_blocking({
        let bee = Arc::clone(bee);
        move || bee.mustahiqqa()
    })
    .await;
    match natija {
        Ok(Ok(ahdaf)) => (ahdaf, None),
        Ok(Err(khata)) => {
            tracing::warn!(
                khata = %khata.li_sijill(),
                "the engine sweep could not read which games are owed a probe"
            );
            (Vec::new(), Some(khata))
        },
        Err(sabab) => {
            let khata = Khata::from(KhataJawla::IkhtiyarLamYantahi {
                tafsil: sabab.to_string(),
            });
            tracing::warn!(khata = %khata.li_sijill(), "the engine sweep's selection did not finish");
            (Vec::new(), Some(khata))
        },
    }
}

/// One worker, probing one game at a time.
///
/// Takes the next game off the shared queue, probes it on a blocking thread,
/// records the outcome, and goes back for another until the queue is empty.
/// Two of these run per round, which is the whole of the bound.
async fn amil<B: BeeatJawla>(
    halat: Arc<HalatJawla>,
    bee: Arc<B>,
    tabur: Arc<Mutex<VecDeque<HadafJawla>>>,
) {
    loop {
        let tali = tabur.lock().pop_front();
        let Some(hadaf) = tali else {
            break;
        };
        let natija = tokio::task::spawn_blocking({
            let bee = Arc::clone(&bee);
            let hadaf = hadaf.clone();
            move || bee.ifhas(&hadaf)
        })
        .await;
        match natija {
            Ok(Ok(fahs)) => {
                halat.sajjil_najah();
                tracing::info!(
                    luba = %hadaf.id,
                    ism = %hadaf.ism,
                    aila = ?fahs.muharrik,
                    tabaqa = ?fahs.tabaqa,
                    "engine probed in the background"
                );
                bee.aalin_fahs(&fahs);
            },
            Ok(Err(khata)) => sajjil_fashal(&halat, &hadaf, khata),
            // The workspace unwinds rather than aborts precisely so that one
            // game's probe can die without taking the process with it; what it
            // owes is a record under the game's name.
            Err(sabab) => sajjil_fashal(
                &halat,
                &hadaf,
                Khata::from(KhataJawla::FahsLamYantahi {
                    ism: hadaf.ism.clone(),
                    tafsil: sabab.to_string(),
                }),
            ),
        }
        bee.aalin_hala(&halat.hie());
    }
}

/// Writes one refusal down, in the log and in the round's record.
fn sajjil_fashal(halat: &HalatJawla, hadaf: &HadafJawla, khata: Khata) {
    tracing::warn!(
        luba = %hadaf.id,
        ism = %hadaf.ism,
        khata = %khata.li_sijill(),
        "a background engine probe failed"
    );
    halat.sajjil_fashal(FashalJawlaHie {
        muarrif: hadaf.id.to_string(),
        ism: hadaf.ism.clone(),
        khata,
    });
}

// ---------------------------------------------------------------------------
// The Studio's world
// ---------------------------------------------------------------------------

/// The sweep as the Studio runs it: the store for the selection and the probe,
/// the window for the announcements.
#[derive(Clone)]
struct BeeatStudio {
    /// The window the events go to.
    tatbiq: tauri::AppHandle,
    /// The library and the report ledger.
    makhzan: Makhzan,
    /// Where the backups live, for the installed-patch check the badge needs.
    masarat: Masarat,
}

impl BeeatJawla for BeeatStudio {
    fn mustahiqqa(&self) -> Natija<Vec<HadafJawla>> {
        let (alaab, isdarat) = self.makhzan.bil_qira(|ittisal| {
            let alaab = SijillAlaab::jadeed(ittisal).qaima(&TalabMaktaba {
                mawjuda_faqat: true,
                ..TalabMaktaba::default()
            })?;
            let isdarat = SijillMuharrik::jadeed(ittisal).isdarat()?;
            Ok((alaab, isdarat))
        })?;
        Ok(mustahiqqa(&alaab, &isdarat, ISDAR_FAHS))
    }

    fn ifhas(&self, hadaf: &HadafJawla) -> Natija<FahsMuharrikHie> {
        let luba = luba_awamir::ijlib_luba(&self.makhzan, hadaf.id)?;
        let simat = luba_awamir::simat_luba(&self.makhzan, hadaf.id)?;
        // Asked for, not forced: the game screen may have written a current
        // report while this game waited its turn, and that report is served.
        let taqreer = luba_awamir::taqreer_luba(&self.makhzan, &luba, &simat, false)?;

        let nusakh = luba_awamir::jidhr_nusakh(&self.masarat, &self.makhzan, hadaf.id)?;
        let nass_muthabbat = luba_awamir::muthabbat(&luba.jidhr, &nusakh, NawTathbeet::Nass);
        let ruqaa_mutaha = match luba.masadir.first() {
            Some(masdar) => !self
                .makhzan
                .bil_qira(|ittisal| {
                    SijillRuqaa::jadeed(ittisal).li_luba(masdar.aila().slug(), &masdar.muarrif())
                })?
                .is_empty(),
            None => false,
        };

        Ok(FahsMuharrikHie {
            muarrif: hadaf.id.to_string(),
            ism: luba.ism,
            mafhusa: true,
            muharrik: taqreer.muharrik.aila,
            tabaqa: taqreer.tabaqa,
            jahiziya: taqreer.jahiziya,
            hala: halat_luba(HaqaiqHala {
                marfuda: taqreer.marfuda,
                nass_muthabbat,
                ruqaa_mutaha,
                tabaqa: taqreer.tabaqa,
            }),
        })
    }

    fn aalin_fahs(&self, fahs: &FahsMuharrikHie) {
        if let Err(sabab) = self.tatbiq.emit(ISM_HADATH_FAHS_MUHARRIK, fahs.clone()) {
            tracing::debug!(sabab = %sabab, "a probe report was not delivered");
        }
    }

    fn aalin_hala(&self, hala: &HalatJawlaHie) {
        if let Err(sabab) = self.tatbiq.emit(ISM_HADATH_JAWLA, hala.clone()) {
            tracing::debug!(sabab = %sabab, "the sweep's standing was not delivered");
        }
    }
}

/// Starts the sweep after a completed scan, or asks the running one for another
/// round.
///
/// Returns at once: the claim is taken under the lock and the work is spawned,
/// so the scan's answer is never held up by a probe. Called with the scan's own
/// handles rather than resolving them again, because the scan already holds
/// every one of them.
pub fn ibda_jawla(
    tatbiq: tauri::AppHandle,
    halat: Arc<HalatJawla>,
    makhzan: Makhzan,
    masarat: Masarat,
) {
    if !halat.ihjiz() {
        tracing::info!("an engine sweep is running; another round is queued behind it");
        return;
    }
    let bee = Arc::new(BeeatStudio {
        tatbiq,
        makhzan,
        masarat,
    });
    tauri::async_runtime::spawn(shaghghil(halat, bee));
}

/// Where the background engine sweep stands, for a screen that has just
/// mounted and missed the events.
///
/// Whether one is running, how many games it is over, how many are done, and
/// every game it could not probe — the same record the sweep announces on
/// [`ISM_HADATH_JAWLA`].
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
pub fn halat_jawla(jawla: tauri::State<'_, Arc<HalatJawla>>) -> Result<HalatJawlaHie, Khata> {
    Ok(jawla.hie())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Failures of the sweep's own machinery, as opposed to failures of a probe,
/// which arrive as whatever the probe raised.
#[derive(Debug, thiserror::Error)]
pub enum KhataJawla {
    /// The thread probing one game ended without answering.
    #[error("the background probe of {ism} did not finish: {tafsil}")]
    FahsLamYantahi {
        /// The game's name.
        ism: String,
        /// What the runtime reported.
        tafsil: String,
    },

    /// The thread reading which games are owed a probe ended without answering.
    #[error("the engine sweep's selection did not finish: {tafsil}")]
    IkhtiyarLamYantahi {
        /// What the runtime reported.
        tafsil: String,
    },
}

impl Tafsir for KhataJawla {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::FahsLamYantahi { .. } => 150,
                    Self::IkhtiyarLamYantahi { .. } => 151,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        // Nothing was written in either case: the report is written last, by
        // the probe, and the probe never answered. The library stands as it
        // stood, and the same probe can be asked for again from the screen.
        Khutura::Tanbeeh
    }

    fn arabi(&self) -> String {
        match self {
            Self::FahsLamYantahi { ism, .. } => format!(
                "توقّف فحص محرّك {ism} في الخلفية قبل أن يكتمل، ولم يُكتب شيء. افتح شاشة \
                 اللعبة واطلب فحص المحرّك من جديد."
            ),
            Self::IkhtiyarLamYantahi { .. } => {
                "توقّف فحص المحرّكات في الخلفية قبل أن يقرأ المكتبة، ولم يُكتب شيء. أعد فحص \
                 المكتبة ليبدأ من جديد."
                    .to_owned()
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::FahsLamYantahi { ism, .. } => format!(
                "The background engine probe of {ism} stopped before it finished, and nothing \
                 was written. Open the game's screen and probe the engine again."
            ),
            Self::IkhtiyarLamYantahi { .. } => {
                "The background engine sweep stopped before it read the library, and nothing \
                 was written. Rescan the library to start it again."
                    .to_owned()
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::FahsLamYantahi { .. } => Khutwa::AadaFahsMuharrik,
            Self::IkhtiyarLamYantahi { .. } => Khutwa::AadaFahsMaktaba,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::FahsLamYantahi { ism, tafsil } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::IkhtiyarLamYantahi { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataJawla);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use parking_lot::Condvar;
    use taarib_mustalahat::luba::{MasdarLuba, SuwarLuba};
    use taarib_usus::manassa::BeeatTawafuq;

    use super::*;

    /// Every test returns this so that a missing value reads as a sentence
    /// rather than a panic, which the workspace denies in tests too.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The probe version the tests call "current".
    const ISDAR_IKHTIBAR: u32 = 9;

    /// A visible, present library row known to Steam under `raqm`.
    fn luba(raqm: u32, ism: &str) -> Luba {
        let masdar = MasdarLuba::Steam(raqm);
        Luba {
            id: LubaId::min_masdar(&masdar, ism),
            masadir: vec![masdar],
            ism: ism.to_owned(),
            jidhr: std::path::PathBuf::from("/alaab").join(ism),
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

    /// The target the sweep would build for a row.
    fn hadaf(luba: &Luba) -> HadafJawla {
        HadafJawla {
            id: luba.id,
            ism: luba.ism.clone(),
        }
    }

    /// A finished probe for a target, as the double answers it.
    fn fahs(hadaf: &HadafJawla) -> FahsMuharrikHie {
        FahsMuharrikHie {
            muarrif: hadaf.id.to_string(),
            ism: hadaf.ism.clone(),
            mafhusa: true,
            muharrik: AilatMuharrik::Unity,
            tabaqa: Tabaqa::Kamil,
            jahiziya: JahiziyatTashghil::Mukammala,
            hala: HalatLuba::MadumBilaRuqaa,
        }
    }

    /// A world made of lists.
    ///
    /// The rounds the selection answers with, in order; the games whose probe
    /// must fail; a gate every probe waits at, so a test can hold the sweep
    /// mid-flight; and everything the sweep announced.
    struct BeeatIkhtibar {
        /// One entry per selection call, consumed front to back.
        jawlat: Mutex<VecDeque<Vec<HadafJawla>>>,
        /// How many times the selection was asked.
        adad_ikhtiyar: AtomicUsize,
        /// Probes inside the gate right now.
        jari: AtomicUsize,
        /// The most probes ever inside the gate at once.
        aqsa: AtomicUsize,
        /// Whether probes may leave the gate.
        maftuha: Mutex<bool>,
        /// Woken when the gate opens.
        jaras: Condvar,
        /// Names whose probe fails.
        fashila: Vec<String>,
        /// Every finished game, in the order announced.
        fuhus: Mutex<Vec<FahsMuharrikHie>>,
        /// Every standing, in the order announced.
        halat: Mutex<Vec<HalatJawlaHie>>,
    }

    impl BeeatIkhtibar {
        fn jadeeda(jawlat: Vec<Vec<HadafJawla>>, maftuha: bool, fashila: Vec<String>) -> Self {
            Self {
                jawlat: Mutex::new(VecDeque::from(jawlat)),
                adad_ikhtiyar: AtomicUsize::new(0),
                jari: AtomicUsize::new(0),
                aqsa: AtomicUsize::new(0),
                maftuha: Mutex::new(maftuha),
                jaras: Condvar::new(),
                fashila,
                fuhus: Mutex::new(Vec::new()),
                halat: Mutex::new(Vec::new()),
            }
        }

        /// Lets every waiting and every future probe through.
        fn iftah(&self) {
            *self.maftuha.lock() = true;
            let _ = self.jaras.notify_all();
        }
    }

    impl BeeatJawla for BeeatIkhtibar {
        fn mustahiqqa(&self) -> Natija<Vec<HadafJawla>> {
            let _ = self.adad_ikhtiyar.fetch_add(1, Ordering::SeqCst);
            Ok(self.jawlat.lock().pop_front().unwrap_or_default())
        }

        fn ifhas(&self, hadaf: &HadafJawla) -> Natija<FahsMuharrikHie> {
            let jari = self.jari.fetch_add(1, Ordering::SeqCst).saturating_add(1);
            let _ = self.aqsa.fetch_max(jari, Ordering::SeqCst);
            {
                let mut maftuha = self.maftuha.lock();
                while !*maftuha {
                    self.jaras.wait(&mut maftuha);
                }
            }
            let _ = self.jari.fetch_sub(1, Ordering::SeqCst);
            if self.fashila.contains(&hadaf.ism) {
                return Err(Khata::from(KhataJawla::FahsLamYantahi {
                    ism: hadaf.ism.clone(),
                    tafsil: "ikhtibar".to_owned(),
                }));
            }
            Ok(fahs(hadaf))
        }

        fn aalin_fahs(&self, fahs: &FahsMuharrikHie) {
            self.fuhus.lock().push(fahs.clone());
        }

        fn aalin_hala(&self, hala: &HalatJawlaHie) {
            self.halat.lock().push(hala.clone());
        }
    }

    /// Polls a condition until it holds or five seconds pass.
    async fn intazir(shart: impl Fn() -> bool + Send + Sync) -> NatijatIkhtibar {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !shart() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .map_err(|_| "the condition did not hold within five seconds".into())
    }

    /// A game is owed a probe when it has no report or an older probe wrote
    /// the one it has; a current or newer report, a hidden game and an absent
    /// game are left alone, and library order is kept.
    #[test]
    fn al_mustahiqqa_hiya_ghayr_al_mafhusa_wa_al_qadima() {
        let bila = luba(1, "Bila Taqreer");
        let qadima = luba(2, "Qadima");
        let haliya = luba(3, "Haliya");
        let ahdath = luba(4, "Ahdath");
        let mut mukhfiya = luba(5, "Mukhfiya");
        mukhfiya.mukhfiya = true;
        let mut ghaiba = luba(6, "Ghaiba");
        ghaiba.mawjuda = false;

        let isdarat: BTreeMap<LubaId, u32> = [
            (qadima.id, ISDAR_IKHTIBAR - 1),
            (haliya.id, ISDAR_IKHTIBAR),
            (ahdath.id, ISDAR_IKHTIBAR + 1),
        ]
        .into_iter()
        .collect();

        let alaab = [
            ghaiba,
            haliya,
            qadima.clone(),
            mukhfiya,
            ahdath,
            bila.clone(),
        ];
        let ahdaf = mustahiqqa(&alaab, &isdarat, ISDAR_IKHTIBAR);
        assert_eq!(ahdaf, vec![hadaf(&qadima), hadaf(&bila)]);
    }

    /// The badge follows one rule: refused beats installed, installed beats
    /// offered, offered beats the tier, and the overlay tier alone says
    /// "overlay only".
    #[test]
    fn halat_al_luba_tatba_qaida_wahida() {
        let asas = HaqaiqHala {
            marfuda: false,
            nass_muthabbat: false,
            ruqaa_mutaha: false,
            tabaqa: Tabaqa::Kamil,
        };
        assert_eq!(halat_luba(asas), HalatLuba::MadumBilaRuqaa);
        assert_eq!(
            halat_luba(HaqaiqHala {
                tabaqa: Tabaqa::TarjamaFawqiya,
                ..asas
            }),
            HalatLuba::TabaqaFaqat
        );
        assert_eq!(
            halat_luba(HaqaiqHala {
                ruqaa_mutaha: true,
                tabaqa: Tabaqa::TarjamaFawqiya,
                ..asas
            }),
            HalatLuba::Mutaha
        );
        assert_eq!(
            halat_luba(HaqaiqHala {
                nass_muthabbat: true,
                ruqaa_mutaha: true,
                ..asas
            }),
            HalatLuba::Mutabbaqa
        );
        assert_eq!(
            halat_luba(HaqaiqHala {
                marfuda: true,
                nass_muthabbat: true,
                ruqaa_mutaha: true,
                ..asas
            }),
            HalatLuba::Marfuda
        );
    }

    /// A sweep in flight is not started twice, at most two probes run at once,
    /// and a scan that finishes mid-sweep earns one more round — selected
    /// afresh — after which the claim is free again.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn al_jawla_la_tabda_marratayn_wa_taeed_li_ma_baqiya() -> NatijatIkhtibar {
        let ula: Vec<HadafJawla> = (1..=4)
            .map(|raqm| hadaf(&luba(raqm, &format!("Luba {raqm}"))))
            .collect();
        let baqiya = vec![hadaf(&luba(4, "Luba 4"))];
        let bee = Arc::new(BeeatIkhtibar::jadeeda(vec![ula, baqiya], false, Vec::new()));
        let halat = Arc::new(HalatJawla::default());

        assert!(halat.ihjiz(), "nothing is running, so the claim is granted");
        let mahamma = tokio::spawn(shaghghil(Arc::clone(&halat), Arc::clone(&bee)));

        // Two probes are at the gate and the other two are queued behind them.
        intazir(|| bee.jari.load(Ordering::SeqCst) == 2).await?;
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(
            bee.jari.load(Ordering::SeqCst),
            2,
            "the bound holds while probes wait"
        );
        assert!(halat.hie().jariya);
        assert_eq!(halat.hie().majmu, 4);
        assert!(
            !halat.ihjiz(),
            "a scan finishing mid-sweep is refused the claim and leaves a request"
        );

        bee.iftah();
        mahamma.await?;

        assert_eq!(
            bee.aqsa.load(Ordering::SeqCst),
            2,
            "never more than two at once"
        );
        assert_eq!(
            bee.adad_ikhtiyar.load(Ordering::SeqCst),
            2,
            "one selection per round: the first, and the one the request earned"
        );
        assert_eq!(
            bee.fuhus.lock().len(),
            5,
            "four games, then the one still stale"
        );

        let halat_mulana = bee.halat.lock();
        let awwal = halat_mulana.first().ok_or("no standing was announced")?;
        assert!(awwal.jariya);
        assert_eq!((awwal.tamma, awwal.majmu), (0, 4));
        let akhir = halat_mulana.last().ok_or("no standing was announced")?;
        assert!(!akhir.jariya, "the last word says the sweep is over");
        assert_eq!((akhir.tamma, akhir.majmu, akhir.fashila), (1, 1, 0));
        drop(halat_mulana);

        assert!(halat.ihjiz(), "once over, the next scan gets the claim");
        Ok(())
    }

    /// A game the probe refuses costs that game and no other: the round
    /// finishes, the refusal is counted and carried by name with the probe's
    /// own error, and the other games are announced as usual.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn al_fashal_yuhsab_wa_yuhmal_bil_ism() -> NatijatIkhtibar {
        let alaab = [
            luba(1, "Salima"),
            luba(2, "Fashila"),
            luba(3, "Salima Ukhra"),
        ];
        let ahdaf: Vec<HadafJawla> = alaab.iter().map(hadaf).collect();
        let bee = Arc::new(BeeatIkhtibar::jadeeda(
            vec![ahdaf],
            true,
            vec!["Fashila".to_owned()],
        ));
        let halat = Arc::new(HalatJawla::default());

        assert!(halat.ihjiz());
        shaghghil(Arc::clone(&halat), Arc::clone(&bee)).await;

        let akhir = halat.hie();
        assert!(!akhir.jariya);
        assert_eq!((akhir.tamma, akhir.majmu, akhir.fashila), (2, 3, 1));
        let fashal = akhir.akhta.first().ok_or("the refusal was not carried")?;
        assert_eq!(fashal.ism, "Fashila");
        assert_eq!(fashal.khata.ramz.raqm(), arqam::STUDIO + 150);
        assert!(akhir.khata.is_none());

        let fuhus = bee.fuhus.lock();
        let asmaa: Vec<&str> = fuhus.iter().map(|fahs| fahs.ism.as_str()).collect();
        assert_eq!(asmaa.len(), 2);
        assert!(asmaa.contains(&"Salima") && asmaa.contains(&"Salima Ukhra"));
        assert!(
            fuhus.iter().all(|fahs| fahs.mafhusa),
            "every announced row says it has been examined"
        );
        Ok(())
    }

    /// A round whose selection cannot be read still ends, releases the claim,
    /// and says why in its final standing rather than in the log alone.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fashal_al_ikhtiyar_yuhmal_fi_al_hala() -> NatijatIkhtibar {
        struct BeeatMuallala;

        impl BeeatJawla for BeeatMuallala {
            fn mustahiqqa(&self) -> Natija<Vec<HadafJawla>> {
                Err(Khata::from(KhataJawla::IkhtiyarLamYantahi {
                    tafsil: "ikhtibar".to_owned(),
                }))
            }

            fn ifhas(&self, hadaf: &HadafJawla) -> Natija<FahsMuharrikHie> {
                Ok(fahs(hadaf))
            }

            fn aalin_fahs(&self, _fahs: &FahsMuharrikHie) {}

            fn aalin_hala(&self, _hala: &HalatJawlaHie) {}
        }

        let halat = Arc::new(HalatJawla::default());
        assert!(halat.ihjiz());
        shaghghil(Arc::clone(&halat), Arc::new(BeeatMuallala)).await;

        let akhir = halat.hie();
        assert!(!akhir.jariya);
        assert_eq!(akhir.majmu, 0);
        let khata = akhir.khata.ok_or("the selection failure was dropped")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 151);
        Ok(())
    }
}
