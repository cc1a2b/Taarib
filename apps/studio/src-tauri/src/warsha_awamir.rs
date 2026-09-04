//! الورشة — the translation workspace's command surface: the table, the edits, the suggestions,
//! the machine runs, the quality summary, the preview, the comments, and the merge.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use taarib_istikhraj::mashru::{MALAF_MASHRU, MashruMaftuh};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muraja::{HalatMuraja, SijillMuraja};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId, TasnifNass};
use taarib_mustalahat::ruqaa::TareeqaTarjama;
use taarib_saff::Saff;
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_saff::talab::{KhiyaratTakhtit, TalabTakhtit};
use taarib_tarjama::alamat::{
    AtabatAlamat, MudkhalatQiyas, ThiqaMublagha, ihsib_mashru, ihsib_wa_thabbit,
    thabbit_tanaqud,
};
use taarib_tarjama::dhakira::{AslQayd, Dhakira, QaydDhakira, QaydId, QaydJadid, TalabDhakira};
use taarib_tarjama::dufaat::{
    KhiyaratJawla, SijillJawla, TaqaddumJawla, shaghghil_jawla, tabbiq_sijill, thiqat_min_sijill,
};
use taarib_tarjama::khata::KhataTarjama;
use taarib_tarjama::masrad::{Masrad, MustalahMasrad, TadarubMustalah, wahhid_tadarub};
use taarib_tarjama::muzawwidun::{
    IdadatAnthropic, IdadatDeepL, IdadatGemini, IdadatMicrosoft, IdadatMuwafiqOpenAI, IdhnInfaq,
    Itimad, Muzawwid, MuzawwidAnthropic, MuzawwidDeepL, MuzawwidGemini, MuzawwidMicrosoft,
    MuzawwidMuwafiqOpenAI, taklifat_anthropic, taklifat_gemini,
};
use taarib_taqdeem::taaliq::Taaliq;
use taarib_tathbeet::bayan::waqt_alaan;
use taarib_usus::idadat::{Idadat, IdadatMuzawwid, MakhzanIdadat, NawMuzawwid};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, kitaba_dharra};
use taarib_warsha::damj::{BitaqatJanib, Damj, NawNizaa, Nizaa, Qarar, damj};
use taarib_warsha::tarikh::{TarikhMashru, damj_tarikh};
use tauri::Emitter as _;
use taarib_warsha::tasdir::{
    UDW_MASRAD, UDW_TAALIQAT, UDW_TARIKH, UDW_TAWZI, aslaf_mashru, istawrid, sajjil_aslaf,
};
use taarib_warsha::tawzi::Tawzi;

use crate::luba_awamir::huwiya;

/// The window event a batch translation reports progress on.
pub const ISM_HADATH_DUFA: &str = "taarib://taqaddum-dufa";

/// The provenance label a memory application records as its provider.
const MUZAWWID_DHAKIRA: &str = "الذاكرة";

/// One quality flag on a workspace row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AlamHie {
    /// The flag's kind, as its serde tag spells it.
    pub naw: String,
    /// The sentence shown beside the string, in Arabic.
    pub wasf_arabi: String,
    /// The same sentence in English.
    pub wasf_injilizi: String,
    /// Whether this flag blocks the string from shipping.
    pub khatir: bool,
}

/// One string row as the workspace draws it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SafWarshaHie {
    /// The string's identity.
    pub nass: String,
    /// The clean source text.
    pub masdar: String,
    /// The clean Arabic translation, when there is one.
    pub hadaf: Option<String>,
    /// Where it stands in the review workflow.
    pub hala: HalatMuraja,
    /// Everything currently wrong with it.
    pub alamat: Vec<AlamHie>,
    /// What kind of string it is, as its serde tag spells it.
    pub tasnif: String,
    /// The same classification as the interface groups it, in Arabic.
    pub tasnif_arabi: String,
    /// Whether a player is believed never to see it.
    pub dakhili: bool,
    /// The container it came from.
    pub hawiya: String,
    /// The location inside that container.
    pub mawqi: String,
    /// The speaker, for dialogue.
    pub mutakallim: Option<String>,
    /// The lines around it.
    pub jiwar: Vec<String>,
    /// How many times this exact source occurs in the game.
    pub takrar: u32,
    /// How the translation was produced, in Arabic, when recorded.
    pub tareeqa_arabi: Option<String>,
    /// The provider that produced it, when a machine did.
    pub muzawwid: Option<String>,
    /// The short form of the assignee, when the string is assigned.
    pub muayyan: Option<String>,
    /// The width available, in the game's pixels, when it was measured.
    pub aqsa_ard: Option<f64>,
    /// The size the game draws it at, when it was recorded.
    pub hajm_khatt: Option<f64>,
}

/// The whole project as the workspace opens it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct WarshaHie {
    /// The game's identity.
    pub muarrif: String,
    /// The game's display name.
    pub ism_luba: String,
    /// The local identity's short form, which `muayyan` values are compared against.
    pub musahimi: String,
    /// How many rows the table holds.
    pub adad: u32,
    /// Every row.
    pub sufuf: Vec<SafWarshaHie>,
}

/// One glossary hit for the selected string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MustalahHie {
    /// The source form.
    pub masdar: String,
    /// The approved Arabic form.
    pub arabi: String,
    /// The translator's note, when one was written.
    pub mulahaza: Option<String>,
}

/// One translation-memory suggestion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IqtirahHie {
    /// The record's identity in this machine's memory.
    pub qayd: QaydId,
    /// The stored Arabic.
    pub hadaf: String,
    /// The measured similarity, per mille.
    pub tashabuh: u16,
    /// Whether a human ever read the pair.
    pub muraja_bashariya: bool,
    /// The record's own source text, for comparison against the current one.
    pub masdar_asli: String,
}

/// Everything the suggestion rail shows for one string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IqtirahatHie {
    /// The glossary terms found in the source.
    pub mustalahat: Vec<MustalahHie>,
    /// The exact match, when the memory holds one.
    pub tatbiq: Option<IqtirahHie>,
    /// Near matches, best rank first. Never applied by anything automatic.
    pub iqtirahat: Vec<IqtirahHie>,
}

/// One terminology conflict across the project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TadarubHie {
    /// The source term.
    pub mustalah: String,
    /// The competing forms, the glossary's approved one first.
    pub ashkal: Vec<String>,
    /// Every string involved, as identities.
    pub mawaqi: Vec<String>,
    /// How many strings are involved.
    pub adad: u32,
}

/// The project-wide quality summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AlamatMashruHie {
    /// Every flag currently on the table.
    pub adad_alamat: u32,
    /// The flags that block shipping.
    pub adad_khatira: u32,
    /// The terminology conflicts.
    pub tadarubat: Vec<TadarubHie>,
}

/// One measured line of the preview.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrMuayanaHie {
    /// The text of the line.
    pub nass: String,
    /// Its measured width, in the game's pixels.
    pub ard: f64,
}

/// The live preview measured through the real layout engine.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MuayanaHie {
    /// Whether the verdict cannot be given, because something was never measured.
    pub ghayr_qabil: bool,
    /// Why, when it cannot.
    pub sabab_ghayr_qabil: Option<String>,
    /// The size the text was measured at.
    pub hajm: Option<f64>,
    /// The width available.
    pub mutah: Option<f64>,
    /// The measured lines.
    pub sutur: Vec<SatrMuayanaHie>,
    /// How many pixels the widest line exceeds the width by.
    pub tajawuz_biksil: Option<f64>,
    /// The same excess as a fraction of the width.
    pub tajawuz_nisba: Option<f64>,
}

/// One anchored comment surfaced in the workspace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaaliqWarshaHie {
    /// The string it anchors to, or null when it is about the submission.
    pub nass: Option<String>,
    /// The writer's short form.
    pub kaatib: String,
    /// Whether the owner's authority minted it.
    pub min_almalik: bool,
    /// The body.
    pub matn: String,
    /// When it was written, RFC 3339.
    pub waqt: String,
    /// Whether it has been marked resolved.
    pub muhall: bool,
}

/// One side of a merge conflict, with its full provenance.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct JanibNizaaHie {
    /// The translation text.
    pub hadaf: String,
    /// Its review state, in Arabic.
    pub hala: String,
    /// How it was produced, in Arabic, when recorded.
    pub tareeqa_arabi: Option<String>,
    /// The provider, when a machine produced it.
    pub muzawwid: Option<String>,
    /// The short form of who last changed it.
    pub muharrir: Option<String>,
    /// When it was last changed, RFC 3339.
    pub akhir_tabdeel: Option<String>,
}

/// One merge conflict awaiting an explicit choice.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct NizaaHie {
    /// The string's identity.
    pub nass: String,
    /// Its source text.
    pub masdar: String,
    /// What the disagreement is about.
    pub naw: String,
    /// The local side.
    pub ana: JanibNizaaHie,
    /// The imported side.
    pub hum: JanibNizaaHie,
}

/// What an import-and-merge produced, held open until every conflict is decided.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct NizaatHie {
    /// The open conflicts.
    pub nizaat: Vec<NizaaHie>,
    /// Strings identical on both sides.
    pub mutatabiqa: u32,
    /// Strings taken from the local side without conflict.
    pub min_ana: u32,
    /// Strings taken from the imported side without conflict.
    pub min_hum: u32,
    /// Strings only one copy carried.
    pub munfarida: u32,
    /// Whether a common ancestor was available.
    pub thulathi: bool,
}

/// One explicit conflict resolution, as the interface sends it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "qarar", rename_all = "snake_case")]
pub enum QararHie {
    /// Keep the local side.
    KhudhLi {
        /// The string.
        nass: String,
    },
    /// Keep the imported side.
    KhudhHum {
        /// The string.
        nass: String,
    },
    /// Write a third text, which enters review from the beginning.
    Thalith {
        /// The string.
        nass: String,
        /// The new translation.
        hadaf: String,
    },
}

/// What finalizing a merge did.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DamjHie {
    /// How many rows the merged table holds.
    pub sufuf: u32,
    /// How many conflicts were resolved.
    pub husum: u32,
}

/// One machine-translation outcome for one string.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct NatijatTarjamaHie {
    /// The row afterwards.
    pub saf: SafWarshaHie,
    /// Whether a translation landed.
    pub najahat: bool,
    /// Why not, when it did not.
    pub sabab_arabi: Option<String>,
}

/// A batch run's accounting, live and final.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct DufaHie {
    /// Successfully translated and committed.
    pub mutarjama: u32,
    /// Failed, each with its recorded reason.
    pub fashila: u32,
    /// Settled spend so far, US dollars.
    pub taklifa: f64,
    /// The ceiling the run honours, US dollars.
    pub saqf: f64,
    /// Whether the ceiling is what stopped it.
    pub tawaqqafat_lil_saqf: bool,
}

/// A held merge: the open conflicts and the imported pieces to fold on completion.
#[derive(Debug)]
struct DamjMahfudh {
    damj: Damj,
    tarikh_hum: TarikhMashru,
    taaliqat_hum: Vec<Taaliq>,
    masrad_hum: Vec<MustalahMasrad>,
}

/// The open merge sessions, one at most per project.
///
/// `parking_lot`, not `std`: the guard is taken, the map is read or written,
/// and it is released before any I/O — there is no poisoning to recover from
/// and no unwrap to justify.
#[derive(Debug, Default)]
pub struct JalasatDamj(parking_lot::Mutex<HashMap<LubaId, DamjMahfudh>>);

/// One writer at a time per project: every read-modify-write of a project's
/// files takes its lock, so two commands cannot interleave a lost update.
///
/// `tokio`, not `parking_lot`: a project lock is held across the file I/O it
/// protects, and blocking a runtime thread there would stall every other
/// command for the duration of a merge.
#[derive(Debug, Default)]
pub struct AqfalMashariya(pub tokio::sync::Mutex<HashMap<LubaId, Arc<tokio::sync::Mutex<()>>>>);

fn qufl_mashru(aqfal: &AqfalMashariya, id: LubaId) -> Arc<tokio::sync::Mutex<()>> {
    let mut kharita = aqfal.0.blocking_lock();
    kharita.entry(id).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
}

async fn qufl_mashru_async(aqfal: &AqfalMashariya, id: LubaId) -> Arc<tokio::sync::Mutex<()>> {
    let mut kharita = aqfal.0.lock().await;
    kharita.entry(id).or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))).clone()
}

fn khata_malaf(masar: &Path, sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataWarshaAmr::MalafTalif {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

pub(crate) fn lahza_alaan() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |muddat| muddat.as_secs())
}

fn jidhr_mashru(masarat_hala: &Masarat, id: LubaId) -> PathBuf {
    masarat_hala.mashari().join(id.to_string())
}

/// Opens the project for one game, refusing when none has been created yet.
fn iftah_mashru(masarat_hala: &Masarat, id: LubaId) -> Natija<MashruMaftuh> {
    let jidhr = jidhr_mashru(masarat_hala, id);
    if !jidhr.join(MALAF_MASHRU).is_file() {
        return Err(Khata::from(KhataWarshaAmr::MashruGhayrMawjud { ism: id.to_string() }));
    }
    MashruMaftuh::iftah(jidhr).map_err(Khata::from)
}

/// Parses a string identity the interface sent back.
fn huwiyat_nass(nass: &str) -> Natija<NassId> {
    serde_json::from_value::<NassId>(serde_json::Value::String(nass.to_owned()))
        .map_err(|_| Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass: nass.to_owned() }))
}

/// This machine's editor identity, minted once and kept beside the data.
pub(crate) fn muharrir_mahalli(masarat_hala: &Masarat) -> Natija<MusahimId> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct MalafMuharrir {
        musahim: String,
    }

    let masar = masarat_hala.jidhr_bayanat().join("muharrir.json");
    if masar.is_file() {
        let bayt = std::fs::read(&masar).map_err(|sabab| khata_malaf(&masar, sabab))?;
        let malaf: MalafMuharrir =
            serde_json::from_slice(&bayt).map_err(|sabab| khata_malaf(&masar, sabab))?;
        return MusahimId::jadeed(malaf.musahim).map_err(|_| {
            Khata::from(KhataWarshaAmr::MalafTalif {
                masar,
                sabab: "the stored editor identity is not a fingerprint".to_owned(),
            })
        });
    }

    let basma = format!(
        "{}{}",
        uuid::Uuid::new_v4().as_simple(),
        uuid::Uuid::new_v4().as_simple()
    );
    let musahim = MusahimId::jadeed(basma).map_err(|_| {
        Khata::from(KhataWarshaAmr::MalafTalif {
            masar: masar.clone(),
            sabab: "a freshly minted identity failed its own validation".to_owned(),
        })
    })?;
    let bayt = serde_json::to_vec(&MalafMuharrir { musahim: musahim.nass().to_owned() })
        .map_err(|sabab| khata_malaf(&masar, sabab))?;
    kitaba_dharra(&masar, &bayt)?;
    Ok(musahim)
}

/// Reads an optional JSON file beside the project; absent answers the default.
pub(crate) fn iqra_janibi<T: DeserializeOwned + Default>(masar: &Path) -> Natija<T> {
    if !masar.is_file() {
        return Ok(T::default());
    }
    let bayt = std::fs::read(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
    serde_json::from_slice(&bayt).map_err(|sabab| khata_malaf(masar, sabab))
}

/// Reads a required JSON file beside the project.
///
/// Separate from [`iqra_janibi`] because the two answer different questions.
/// That one exists so an absent file means "nothing configured yet"; this one
/// is for a document the caller has already established must be there, and
/// whose type has no meaningful empty value — a pull request with no record and
/// no body is not an empty pull request, it is a missing one.
pub(crate) fn iqra_janibi_matlub<T: DeserializeOwned>(masar: &Path) -> Natija<T> {
    let bayt = std::fs::read(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
    serde_json::from_slice(&bayt).map_err(|sabab| khata_malaf(masar, sabab))
}

pub(crate) fn uktub_janibi<T: serde::Serialize>(masar: &Path, qeema: &T) -> Natija<()> {
    let bayt = serde_json::to_vec(qeema).map_err(|sabab| khata_malaf(masar, sabab))?;
    kitaba_dharra(masar, &bayt)
}

/// Every usable font in the font directories, as one fallback chain, or none.
fn silsilat_khutut(masarat_hala: &Masarat, idadat: &Idadat) -> Option<SilsilatKhutut> {
    // A directory the user pointed at outranks the bundle and is placed ahead
    // of it: a font the build shipped must not shadow the one they chose.
    let mut mujalladat = vec![masarat_hala.khutut()];
    if let Some(masar) = &idadat.khutut.masar_khutut_mustakhdim {
        mujalladat.push(PathBuf::from(masar));
    }
    mujalladat.extend(
        crate::mukawwinat_tahmil::judhur_khutut(masarat_hala).into_iter().skip(1),
    );
    let mut khutut = Vec::new();
    for masar in crate::mukawwinat_tahmil::milaffat_khutut(&mujalladat) {
        let Ok(bayt) = std::fs::read(&masar) else { continue };
        let bayt = Arc::new(bayt);
        let mawrid = MawridKhatt::jadeed(Arc::clone(&bayt), 0)
            .or_else(|_| MawridKhatt::jadeed_latini(bayt, 0));
        if let Ok(mawrid) = mawrid {
            khutut.push(Arc::new(mawrid));
        }
    }
    // The chain's head must shape Arabic, so Arabic-capable fonts sort first.
    khutut.sort_by_key(|mawrid| mawrid.fahs_arabi().is_err());
    SilsilatKhutut::jadeeda(khutut).ok()
}

/// The kind tag of a quality flag, as its serde representation spells it.
const fn naw_alam(alam: &AlamJawda) -> &'static str {
    match alam {
        AlamJawda::KhatarTajawuz { .. } => "khatar_tajawuz",
        AlamJawda::NassLatiniMutabaqqi { .. } => "nass_latini_mutabaqqi",
        AlamJawda::MustalahMukhtalif { .. } => "mustalah_mukhtalif",
        AlamJawda::NasqMaksur { .. } => "nasq_maksur",
        AlamJawda::ThiqaMunkhafida { .. } => "thiqa_munkhafida",
        AlamJawda::NisbaShadha { .. } => "nisba_shadha",
        AlamJawda::Farigh => "farigh",
        AlamJawda::TarjamaMutanaqida { .. } => "tarjama_mutanaqida",
        AlamJawda::AaliyaBilaMuraja => "aaliya_bila_muraja",
    }
}

/// The kind tag of a classification, as its serde representation spells it.
const fn naw_tasnif(tasnif: TasnifNass) -> &'static str {
    match tasnif {
        TasnifNass::Hiwar => "hiwar",
        TasnifNass::Ikhtiyar => "ikhtiyar",
        TasnifNass::Ism => "ism",
        TasnifNass::Wasf => "wasf",
        TasnifNass::Qaima => "qaima",
        TasnifNass::Tafseer => "tafseer",
        TasnifNass::Nizam => "nizam",
        TasnifNass::Khata => "khata",
        TasnifNass::Nusub => "nusub",
        TasnifNass::Dakhili => "dakhili",
        TasnifNass::Majhul => "majhul",
    }
}

fn alam_hie(alam: &AlamJawda) -> AlamHie {
    AlamHie {
        naw: naw_alam(alam).to_owned(),
        wasf_arabi: alam.wasf_arabi(),
        wasf_injilizi: alam.wasf_injilizi(),
        khatir: matches!(alam.khutura(), Khutura::Fadih),
    }
}

fn saf_hie(mudkhal: &MudkhalNass, tawzi: &Tawzi) -> SafWarshaHie {
    SafWarshaHie {
        nass: mudkhal.id.to_string(),
        masdar: mudkhal.masdar.clone(),
        hadaf: mudkhal.hadaf.clone(),
        hala: mudkhal.muraja.hala(),
        alamat: mudkhal.alamat.iter().map(alam_hie).collect(),
        tasnif: naw_tasnif(mudkhal.tasnif).to_owned(),
        tasnif_arabi: mudkhal.tasnif.wasf_arabi().to_owned(),
        dakhili: !mudkhal.tasnif.yaraha_allaib(),
        hawiya: mudkhal.siyaq.hawiya.clone(),
        mawqi: mudkhal.siyaq.mawqi.clone(),
        mutakallim: mudkhal.siyaq.mutakallim.clone(),
        jiwar: mudkhal.siyaq.jiwar.clone(),
        takrar: mudkhal.takrar,
        tareeqa_arabi: mudkhal.tareeqa.map(|tareeqa| tareeqa.wasf_arabi().to_owned()),
        muzawwid: mudkhal.muzawwid.clone(),
        muayyan: tawzi.mukallaf(mudkhal.id).map(MusahimId::mukhtasar),
        aqsa_ard: mudkhal.quyud.aqsa_ard.map(f64::from),
        hajm_khatt: mudkhal.quyud.hajm_khatt.map(f64::from),
    }
}

/// The whole project, every row, as the workspace opens it.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`] when no project exists for the game yet, and whatever
/// the project store raises reading it.
#[tauri::command]
#[specta::specta]
pub fn nusus_warsha(
    muarrif: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<WarshaHie, Khata> {
    let id = huwiya(muarrif.clone())?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;

    // A crash between a run's journal write and its table fold leaves finished
    // translations invisible; the fold is pure, so opening folds the remainder.
    let jidhr_mashru_hali = mashru.jidhr().to_path_buf();
    let sijill_jawla = SijillJawla::iftah(&jidhr_mashru_hali).map_err(Khata::from)?;
    if !sijill_jawla.quyud().is_empty() {
        let waqt = waqt_alaan();
        let tatbiq = tabbiq_sijill(&mut sufuf, &sijill_jawla, &waqt);
        if tatbiq.mutabbaqa > 0 || tatbiq.fashila_muallama > 0 {
            let hali = idadat.hali();
            let thiqat = thiqat_min_sijill(&sijill_jawla);
            aid_hisab_alamat(&masarat_hala, &hali, &mut sufuf, &jidhr_mashru_hali, &thiqat);
            mashru.uktub_kul(&sufuf, waqt)?;
        }
    }
    let tawzi: Tawzi = iqra_janibi(&mashru.jidhr().join(UDW_TAWZI))?;
    Ok(WarshaHie {
        muarrif,
        ism_luba: mashru.rasm().ism_luba.clone(),
        musahimi: muharrir_mahalli(&masarat_hala)?.mukhtasar(),
        adad: u32::try_from(sufuf.len()).unwrap_or(u32::MAX),
        sufuf: sufuf.iter().map(|mudkhal| saf_hie(mudkhal, &tawzi)).collect(),
    })
}

/// The glossary in force: terms harvested from the table plus the project's own file.
pub(crate) fn masrad_kamil(jidhr: &Path, sufuf: &[MudkhalNass]) -> Masrad {
    let mut masrad = Masrad::min_nusus(sufuf);
    let masar = jidhr.join(UDW_MASRAD);
    if masar.is_file()
        && let Ok(mustalahat) = Masrad::min_malaf(&masar)
    {
        masrad.admij(mustalahat);
    }
    masrad
}

/// Recomputes one edited row's flags, then the cross-string pass over the table.
fn haddith_alamat_saf(
    sufuf: &mut [MudkhalNass],
    id: NassId,
    masrad: &Masrad,
    silsila: Option<&SilsilatKhutut>,
) {
    let mut saff = Saff::jadeed();
    let mut qiyas = silsila.map(|khutut| MudkhalatQiyas { saff: &mut saff, khutut });
    if let Some(mudkhal) = sufuf.iter_mut().find(|mudkhal| mudkhal.id == id) {
        let sijill = mudkhal.muraja.clone();
        let alamat_masrad = masrad.afhas(mudkhal);
        ihsib_wa_thabbit(
            mudkhal,
            Some(&sijill),
            None,
            &alamat_masrad,
            qiyas.as_mut(),
            &AtabatAlamat::default(),
        );
    }
    let _ = thabbit_tanaqud(sufuf);
}

/// One edit written through: the updated row.
///
/// An empty `hadaf` clears the translation. A human edit lands as a draft; editing machine
/// output records the machine-assisted method rather than claiming a fully human one.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`], [`KhataWarshaAmr::NassGhayrMawjud`], and whatever
/// the project store, the history file, or the memory raise.
#[tauri::command]
#[specta::specta]
pub fn haddith_tarjama(
    muarrif: String,
    nass: String,
    hadaf: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<SafWarshaHie, Khata> {
    let id = huwiya(muarrif.clone())?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let nass_id = huwiyat_nass(&nass)?;
    let muharrir = muharrir_mahalli(&masarat_hala)?;
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();

    let jadeed = if hadaf.trim().is_empty() { None } else { Some(hadaf) };
    let (sabiq, tasnif, masdar_nass, nasq_masdar) = {
        let Some(mudkhal) = sufuf.iter_mut().find(|mudkhal| mudkhal.id == nass_id) else {
            return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
        };
        let sabiq = mudkhal.hadaf.clone();
        let kana_aali = matches!(
            mudkhal.tareeqa,
            Some(TareeqaTarjama::AaliyaFaqat | TareeqaTarjama::AaliyaThumBashariya)
        );
        mudkhal.hadaf.clone_from(&jadeed);
        // A hand-typed translation carries no machine spans; stale offsets would lie.
        mudkhal.nasq_hadaf = Vec::new();
        if jadeed.is_some() {
            mudkhal.muraja.sajjil_musawwada(muharrir.clone(), lahza);
            if kana_aali && sabiq.is_some() {
                mudkhal.tareeqa = Some(TareeqaTarjama::AaliyaThumBashariya);
            } else {
                mudkhal.tareeqa = Some(TareeqaTarjama::BashariyaKamila);
                mudkhal.muzawwid = None;
            }
        } else {
            mudkhal.muraja.imsah(Some(muharrir.clone()), lahza);
            mudkhal.tareeqa = None;
            mudkhal.muzawwid = None;
        }
        mudkhal.muharrir = Some(muharrir.clone());
        mudkhal.akhir_tabdeel = Some(waqt.clone());
        (sabiq, mudkhal.tasnif, mudkhal.masdar.clone(), mudkhal.nasq_masdar.clone())
    };

    let masar_tarikh = mashru.jidhr().join(UDW_TARIKH);
    let mut tarikh: TarikhMashru = iqra_janibi(&masar_tarikh)?;
    if tarikh.sajjil_tabdeel(nass_id, sabiq, jadeed.clone(), muharrir.clone(), waqt.clone(), None)
    {
        uktub_janibi(&masar_tarikh, &tarikh)?;
    }

    if let Some(hadaf_jadeed) = &jadeed
        && let Some(asl) =
            AslQayd::min_halat(HalatMuraja::Musawwada, Some(muharrir), None, None)
    {
        let mut dhakira = Dhakira::iftah(&masarat_hala)?;
        dhakira.sajjil(&QaydJadid {
            masdar: masdar_nass,
            hadaf: hadaf_jadeed.clone(),
            tasnif,
            mashru: Some(muarrif.clone()),
            luba: Some(muarrif),
            ism_luba: Some(mashru.rasm().ism_luba.clone()),
            siyaq: None,
            nasq_masdar,
            nasq_hadaf: Vec::new(),
            asl,
        })?;
    }

    let hali = idadat.hali();
    let silsila = silsilat_khutut(&masarat_hala, &hali);
    let masrad = masrad_kamil(mashru.jidhr(), &sufuf);
    haddith_alamat_saf(&mut sufuf, nass_id, &masrad, silsila.as_ref());
    mashru.uktub_kul(&sufuf, waqt)?;

    let tawzi: Tawzi = iqra_janibi(&mashru.jidhr().join(UDW_TAWZI))?;
    sufuf
        .iter()
        .find(|mudkhal| mudkhal.id == nass_id)
        .map(|mudkhal| saf_hie(mudkhal, &tawzi))
        .ok_or_else(|| Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }))
}

fn iqtirah_hie(qayd: &QaydDhakira, tashabuh: u16) -> IqtirahHie {
    IqtirahHie {
        qayd: qayd.id,
        hadaf: qayd.hadaf.clone(),
        tashabuh,
        muraja_bashariya: qayd.asl.muraja_bashariya,
        masdar_asli: qayd.masdar.clone(),
    }
}

/// Glossary and memory suggestions for one string.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`], [`KhataWarshaAmr::NassGhayrMawjud`], and whatever
/// the memory raises.
#[tauri::command]
#[specta::specta]
pub fn iqtirahat_nass(
    muarrif: String,
    nass: String,
    masarat_hala: tauri::State<'_, Masarat>,
) -> Result<IqtirahatHie, Khata> {
    let id = huwiya(muarrif.clone())?;
    let nass_id = huwiyat_nass(&nass)?;
    let mashru = iftah_mashru(&masarat_hala, id)?;
    let (sufuf, _talifa) = mashru.iqra_nusus()?;
    let Some(mudkhal) = sufuf.iter().find(|mudkhal| mudkhal.id == nass_id) else {
        return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
    };

    let masrad = masrad_kamil(mashru.jidhr(), &sufuf);
    let mustalahat = masrad
        .mustalahat_fi(&mudkhal.masdar)
        .into_iter()
        .map(|mustalah| MustalahHie {
            masdar: mustalah.masdar.clone(),
            arabi: mustalah.arabi.clone(),
            mulahaza: mustalah.mulahaza.clone(),
        })
        .collect();

    let dhakira = Dhakira::iftah(&masarat_hala)?;
    let hasad = dhakira
        .ibhath(&TalabDhakira::jadeed(&mudkhal.masdar, mudkhal.tasnif).bi_luba(&muarrif))?;

    Ok(IqtirahatHie {
        mustalahat,
        tatbiq: hasad.tatbiq.as_ref().map(|tatbiq| iqtirah_hie(&tatbiq.qayd, 1000)),
        iqtirahat: hasad
            .iqtirahat
            .iter()
            .map(|iqtirah| iqtirah_hie(&iqtirah.qayd, iqtirah.tashabuh.alf()))
            .collect(),
    })
}

/// A memory record applied to one string, landing as unreviewed machine-state output.
///
/// # Errors
///
/// [`KhataWarshaAmr::IqtirahGhayrMawjud`] when the record is no longer offered for this
/// string, plus everything [`iqtirahat_nass`] and the project store raise.
#[tauri::command]
#[specta::specta]
pub fn tatbiq_iqtirah(
    muarrif: String,
    nass: String,
    qayd: QaydId,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<SafWarshaHie, Khata> {
    let id = huwiya(muarrif.clone())?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let nass_id = huwiyat_nass(&nass)?;
    let muharrir = muharrir_mahalli(&masarat_hala)?;
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();

    let masdar_nass = sufuf
        .iter()
        .find(|mudkhal| mudkhal.id == nass_id)
        .map(|mudkhal| (mudkhal.masdar.clone(), mudkhal.tasnif))
        .ok_or_else(|| Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass: nass.clone() }))?;

    let dhakira = Dhakira::iftah(&masarat_hala)?;
    let hasad =
        dhakira.ibhath(&TalabDhakira::jadeed(&masdar_nass.0, masdar_nass.1).bi_luba(&muarrif))?;
    let makhtar: Option<QaydDhakira> = hasad
        .tatbiq
        .as_ref()
        .map(|tatbiq| tatbiq.qayd.clone())
        .filter(|sajl| sajl.id == qayd)
        .or_else(|| {
            hasad
                .iqtirahat
                .iter()
                .find(|iqtirah| iqtirah.qayd.id == qayd)
                .map(|iqtirah| iqtirah.qayd.clone())
        });
    let Some(sajl) = makhtar else {
        return Err(Khata::from(KhataWarshaAmr::IqtirahGhayrMawjud { qayd: qayd.raqm() }));
    };

    let sabiq = {
        let Some(mudkhal) = sufuf.iter_mut().find(|mudkhal| mudkhal.id == nass_id) else {
            return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
        };
        let sabiq = mudkhal.hadaf.clone();
        mudkhal.hadaf = Some(sajl.hadaf.clone());
        mudkhal.nasq_hadaf.clone_from(&sajl.nasq_hadaf);
        // No human in this project has read this text; it enters the review queue as
        // machine-state output whatever its origin was.
        mudkhal.muraja.sajjil_aali(lahza);
        mudkhal.tareeqa = Some(if sajl.asl.muraja_bashariya {
            TareeqaTarjama::AaliyaThumBashariya
        } else {
            TareeqaTarjama::AaliyaFaqat
        });
        mudkhal.muzawwid = Some(MUZAWWID_DHAKIRA.to_owned());
        mudkhal.muharrir = None;
        mudkhal.akhir_tabdeel = Some(waqt.clone());
        sabiq
    };
    dhakira.alim_istikhdam(sajl.id)?;

    let masar_tarikh = mashru.jidhr().join(UDW_TARIKH);
    let mut tarikh: TarikhMashru = iqra_janibi(&masar_tarikh)?;
    if tarikh.sajjil_tabdeel(
        nass_id,
        sabiq,
        Some(sajl.hadaf),
        muharrir,
        waqt.clone(),
        Some("تطبيق قيد من ذاكرة الترجمة".to_owned()),
    ) {
        uktub_janibi(&masar_tarikh, &tarikh)?;
    }

    let hali = idadat.hali();
    let silsila = silsilat_khutut(&masarat_hala, &hali);
    let masrad = masrad_kamil(mashru.jidhr(), &sufuf);
    haddith_alamat_saf(&mut sufuf, nass_id, &masrad, silsila.as_ref());
    mashru.uktub_kul(&sufuf, waqt)?;

    let tawzi: Tawzi = iqra_janibi(&mashru.jidhr().join(UDW_TAWZI))?;
    sufuf
        .iter()
        .find(|mudkhal| mudkhal.id == nass_id)
        .map(|mudkhal| saf_hie(mudkhal, &tawzi))
        .ok_or_else(|| Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }))
}

#[expect(clippy::cast_precision_loss, reason = "currency display only")]
pub(crate) fn dolar(nano: u64) -> f64 {
    nano as f64 / 1_000_000_000.0
}

pub(crate) fn nano_min_dolar(mablagh: f64) -> Natija<u64> {
    if !mablagh.is_finite() || mablagh <= 0.0 {
        return Err(Khata::from(KhataWarshaAmr::SaqfGhayrSalih));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "finite, positive, and clamped below u64::MAX"
    )]
    let nano = (mablagh * 1_000_000_000.0).min(1.8e19) as u64;
    Ok(nano)
}

/// The enabled provider the settings elect, or the named refusal.
pub(crate) fn muzawwid_muntakhab(hali: &Idadat) -> Natija<IdadatMuzawwid> {
    let qaima = &hali.muzawwidun.qaima;
    hali.muzawwidun
        .iftiradi
        .as_ref()
        .and_then(|ism| qaima.iter().find(|tarif| &tarif.muarrif == ism && tarif.mufaal))
        .or_else(|| qaima.iter().find(|tarif| tarif.mufaal))
        .cloned()
        .ok_or_else(|| Khata::from(KhataWarshaAmr::LaMuzawwid))
}

/// Builds the elected provider: credential from the keychain, ceiling as its confirmation.
pub(crate) fn bin_muzawwid(
    tarif: &IdadatMuzawwid,
    saqf: u64,
    lahza: u64,
) -> Natija<Box<dyn Muzawwid>> {
    let idhn = IdhnInfaq::baad_taakid(saqf, lahza);
    let hisab = tarif.hisab_miftah.as_deref().unwrap_or(&tarif.muarrif);
    let itimad = || -> Natija<Itimad> {
        Itimad::min_khazina(hisab).map_err(Khata::from)?.ok_or_else(|| {
            Khata::from(KhataWarshaAmr::LaItimad { muzawwid: tarif.muarrif.clone() })
        })
    };
    let hadd = NonZeroU32::new(tarif.hadd_talabat);
    match tarif.naw {
        NawMuzawwid::Anthropic => {
            let mut tarkib = IdadatAnthropic::default();
            if !tarif.namudhaj.is_empty() {
                tarkib.taklifa = taklifat_anthropic(&tarif.namudhaj);
                tarkib.namudhaj.clone_from(&tarif.namudhaj);
            }
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(MuzawwidAnthropic::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?))
        }
        NawMuzawwid::OpenAiMutawafiq => {
            let mut tarkib = IdadatMuwafiqOpenAI::openai();
            tarkib.ism.clone_from(&tarif.muarrif);
            if !tarif.namudhaj.is_empty() {
                tarkib.namudhaj.clone_from(&tarif.namudhaj);
            }
            if let Some(asas) = &tarif.asas {
                tarkib.asas.clone_from(asas);
            }
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(
                MuzawwidMuwafiqOpenAI::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?,
            ))
        }
        NawMuzawwid::Mahalli => {
            if tarif.namudhaj.is_empty() {
                return Err(Khata::from(KhataWarshaAmr::MuzawwidGhayrMadum {
                    muzawwid: tarif.muarrif.clone(),
                    sabab_arabi: "اسم النموذج فارغ في إعدادات المزوّد المحلي.".to_owned(),
                    sabab_injilizi: "the local provider's model name is empty in settings"
                        .to_owned(),
                }));
            }
            let mut tarkib = IdadatMuwafiqOpenAI::ollama(tarif.namudhaj.clone());
            tarkib.ism.clone_from(&tarif.muarrif);
            if let Some(asas) = &tarif.asas {
                tarkib.asas.clone_from(asas);
            }
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(MuzawwidMuwafiqOpenAI::mahalli(tarkib).map_err(Khata::from)?))
        }
        NawMuzawwid::Gemini => {
            let mut tarkib = IdadatGemini::default();
            if !tarif.namudhaj.is_empty() {
                tarkib.taklifa = taklifat_gemini(&tarif.namudhaj);
                tarkib.namudhaj.clone_from(&tarif.namudhaj);
            }
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(MuzawwidGemini::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?))
        }
        NawMuzawwid::Deepl => {
            let mut tarkib = IdadatDeepL::default();
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(MuzawwidDeepL::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?))
        }
        NawMuzawwid::MicrosoftTarjama => {
            let mut tarkib = IdadatMicrosoft::default();
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(MuzawwidMicrosoft::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?))
        }
        NawMuzawwid::GoogleTarjama => Err(Khata::from(KhataWarshaAmr::MuzawwidGhayrMadum {
            muzawwid: tarif.muarrif.clone(),
            sabab_arabi: "مزوّد ترجمة Google السحابي يتطلّب معرّف مشروع سحابي لا تحمله \
                          الإعدادات بعد."
                .to_owned(),
            sabab_injilizi: "the Google Cloud translation provider needs a Cloud project id \
                             settings do not yet carry"
                .to_owned(),
        })),
    }
}

/// Recomputes every flag on the table, journal-reported confidence included.
fn aid_hisab_alamat(
    masarat_hala: &Masarat,
    hali: &Idadat,
    sufuf: &mut [MudkhalNass],
    jidhr: &Path,
    thiqat: &BTreeMap<NassId, ThiqaMublagha>,
) {
    let silsila = silsilat_khutut(masarat_hala, hali);
    let masrad = masrad_kamil(jidhr, sufuf);
    let muraja: BTreeMap<NassId, SijillMuraja> =
        sufuf.iter().map(|mudkhal| (mudkhal.id, mudkhal.muraja.clone())).collect();
    let alamat_masrad: BTreeMap<NassId, Vec<AlamJawda>> =
        sufuf.iter().map(|mudkhal| (mudkhal.id, masrad.afhas(mudkhal))).collect();
    let mut saff = Saff::jadeed();
    let mut qiyas = silsila.as_ref().map(|khutut| MudkhalatQiyas { saff: &mut saff, khutut });
    let _ = ihsib_mashru(
        sufuf,
        Some(&muraja),
        thiqat,
        &alamat_masrad,
        qiyas.as_mut(),
        &AtabatAlamat::default(),
    );
}

pub(crate) fn adad_u32(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// One string machine-translated through the real batch machinery, or its named refusal.
///
/// # Errors
///
/// [`KhataWarshaAmr::LaMuzawwid`], [`KhataWarshaAmr::LaItimad`], and whatever the project
/// store or the journal raise. A provider-side failure is answered, not raised: it comes
/// back as `najahat: false` with its recorded reason.
#[tauri::command]
#[specta::specta]
pub async fn tarjim_nass(
    muarrif: String,
    nass: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<NatijatTarjamaHie, Khata> {
    let id = huwiya(muarrif.clone())?;
    let qufl = qufl_mashru_async(&aqfal, id).await;
    let _harasa = qufl.lock().await;
    let nass_id = huwiyat_nass(&nass)?;
    let hali = idadat.hali();
    let tarif = muzawwid_muntakhab(&hali)?;
    let lahza = lahza_alaan();
    let saqf_nano =
        tarif.mizaniya.and_then(|mablagh| nano_min_dolar(mablagh).ok()).unwrap_or(u64::MAX);
    let muzawwid = bin_muzawwid(&tarif, saqf_nano, lahza)?;

    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let mufrad: Vec<MudkhalNass> =
        sufuf.iter().filter(|mudkhal| mudkhal.id == nass_id).cloned().collect();
    if mufrad.is_empty() {
        return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
    }

    let waqt = waqt_alaan();
    let khiyarat = KhiyaratJawla {
        saqf_takalif: None,
        lahza,
        waqt: waqt.clone(),
        ..KhiyaratJawla::default()
    };
    let jidhr = mashru.jidhr().to_path_buf();
    let taqreer = shaghghil_jawla(&*muzawwid, &mufrad, &jidhr, &khiyarat, None).await;

    let sijill = SijillJawla::iftah(&jidhr).map_err(Khata::from)?;
    let tatbiq = tabbiq_sijill(&mut sufuf, &sijill, &waqt);
    let thiqat = thiqat_min_sijill(&sijill);
    aid_hisab_alamat(&masarat_hala, &hali, &mut sufuf, &jidhr, &thiqat);
    mashru.uktub_kul(&sufuf, waqt)?;

    let fashila = taqreer
        .taqaddum
        .fashila
        .iter()
        .find(|(fashil, _)| *fashil == nass_id)
        .map(|(_, sabab)| sabab.clone());
    let takhatti = &taqreer.taqaddum.mutakhattaha;
    let (najahat, sabab_arabi) = if let Some(sabab) = fashila {
        (false, Some(sabab))
    } else if taqreer.taqaddum.mutarjama > 0 || tatbiq.mutabbaqa > 0 {
        (true, None)
    } else if tatbiq.mahmiya > 0 {
        (false, Some("عمل إنسان أحدث في الطريق؛ لم تُكتب الترجمة الآلية فوقه.".to_owned()))
    } else if takhatti.mutarjama_musbaqan > 0 {
        (false, Some("النص مترجم بالفعل؛ الترجمة الآلية لا تكتب فوق ترجمة قائمة.".to_owned()))
    } else if takhatti.mujammada > 0 {
        (false, Some("النص مجمّد؛ العمليات الجماعية لا تلمسه.".to_owned()))
    } else if takhatti.farigha > 0 {
        (false, Some("النص الأصلي فارغ فلا شيء يُترجم.".to_owned()))
    } else if let Some(khata) = &taqreer.tawaqquf {
        (false, Some(khata.arabi()))
    } else {
        (false, Some("لم تصل إجابة من المزوّد.".to_owned()))
    };

    let tawzi: Tawzi = iqra_janibi(&jidhr.join(UDW_TAWZI))?;
    let saf = sufuf
        .iter()
        .find(|mudkhal| mudkhal.id == nass_id)
        .map(|mudkhal| saf_hie(mudkhal, &tawzi))
        .ok_or_else(|| Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }))?;
    Ok(NatijatTarjamaHie { saf, najahat, sabab_arabi })
}

/// A batch run to a cost ceiling, committed as it lands; progress streams on its event.
///
/// # Errors
///
/// [`KhataWarshaAmr::SaqfGhayrSalih`], [`KhataWarshaAmr::LaMuzawwid`],
/// [`KhataWarshaAmr::LaItimad`], and whatever the project store or the journal raise. A
/// stopped run is not an error: what stopped it is in the returned accounting.
#[tauri::command]
#[specta::specta]
pub async fn tarjim_dufa(
    nafidha: tauri::Window,
    muarrif: String,
    saqf: f64,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<DufaHie, Khata> {
    let id = huwiya(muarrif)?;
    let qufl = qufl_mashru_async(&aqfal, id).await;
    let _harasa = qufl.lock().await;
    let saqf_nano = nano_min_dolar(saqf)?;
    let hali = idadat.hali();
    let tarif = muzawwid_muntakhab(&hali)?;
    let lahza = lahza_alaan();
    let muzawwid = bin_muzawwid(&tarif, saqf_nano, lahza)?;

    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let waqt = waqt_alaan();
    let khiyarat = KhiyaratJawla {
        saqf_takalif: Some(saqf_nano),
        lahza,
        waqt: waqt.clone(),
        ..KhiyaratJawla::default()
    };

    let (irsal, mut istiqbal) = tokio::sync::watch::channel(TaqaddumJawla::default());
    let nafidha_nuskha = nafidha.clone();
    let muraqib = tauri::async_runtime::spawn(async move {
        while istiqbal.changed().await.is_ok() {
            let taqaddum = istiqbal.borrow().clone();
            let _ = nafidha_nuskha.emit(
                ISM_HADATH_DUFA,
                DufaHie {
                    mutarjama: adad_u32(taqaddum.mutarjama),
                    fashila: adad_u32(taqaddum.fashila.len()),
                    taklifa: dolar(taqaddum.munfaq),
                    saqf,
                    tawaqqafat_lil_saqf: false,
                },
            );
        }
    });

    let jidhr = mashru.jidhr().to_path_buf();
    let taqreer = shaghghil_jawla(&*muzawwid, &sufuf, &jidhr, &khiyarat, Some(&irsal)).await;
    drop(irsal);
    let _ = muraqib.await;

    let sijill = SijillJawla::iftah(&jidhr).map_err(Khata::from)?;
    let _ = tabbiq_sijill(&mut sufuf, &sijill, &waqt);
    let thiqat = thiqat_min_sijill(&sijill);
    aid_hisab_alamat(&masarat_hala, &hali, &mut sufuf, &jidhr, &thiqat);
    mashru.uktub_kul(&sufuf, waqt)?;

    Ok(DufaHie {
        mutarjama: adad_u32(taqreer.taqaddum.mutarjama),
        fashila: adad_u32(taqreer.taqaddum.fashila.len()),
        taklifa: dolar(taqreer.taqaddum.munfaq),
        saqf,
        tawaqqafat_lil_saqf: matches!(taqreer.tawaqquf, Some(KhataTarjama::SaqfTakalif { .. })),
    })
}

fn tadarub_hie(tadarub: &TadarubMustalah) -> TadarubHie {
    let mut ashkal = Vec::new();
    if !tadarub.mutamad.trim().is_empty() {
        ashkal.push(tadarub.mutamad.clone());
    }
    for sura in &tadarub.suwar {
        if !ashkal.contains(&sura.shakl) {
            ashkal.push(sura.shakl.clone());
        }
    }
    TadarubHie {
        mustalah: tadarub.mustalah.clone(),
        ashkal,
        mawaqi: tadarub
            .suwar
            .iter()
            .flat_map(|sura| sura.mawaqi.iter().map(ToString::to_string))
            .collect(),
        adad: adad_u32(tadarub.adad_mawaqi()),
    }
}

/// The project-wide quality summary: a full recompute, then the counts and the conflicts.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`] and whatever the project store or the run journal
/// raise.
#[tauri::command]
#[specta::specta]
pub fn alamat_mashru(
    muarrif: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<AlamatMashruHie, Khata> {
    let id = huwiya(muarrif)?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let hali = idadat.hali();
    let jidhr = mashru.jidhr().to_path_buf();

    let sijill = SijillJawla::iftah(&jidhr).map_err(Khata::from)?;
    let thiqat = thiqat_min_sijill(&sijill);
    aid_hisab_alamat(&masarat_hala, &hali, &mut sufuf, &jidhr, &thiqat);
    mashru.uktub_kul(&sufuf, waqt_alaan())?;

    let mut adad_alamat = 0_u32;
    let mut adad_khatira = 0_u32;
    for mudkhal in &sufuf {
        for alam in &mudkhal.alamat {
            adad_alamat = adad_alamat.saturating_add(1);
            if matches!(alam.khutura(), Khutura::Fadih) {
                adad_khatira = adad_khatira.saturating_add(1);
            }
        }
    }

    let masrad = masrad_kamil(&jidhr, &sufuf);
    let tadarubat = masrad.tadarubat(&sufuf).iter().map(tadarub_hie).collect();
    Ok(AlamatMashruHie { adad_alamat, adad_khatira, tadarubat })
}

/// One term unified across every occurrence; answers how many strings changed.
///
/// # Errors
///
/// [`KhataWarshaAmr::TadarubGhayrMawjud`] when the conflict is no longer present, and
/// whatever the project store raises.
#[tauri::command]
#[specta::specta]
pub fn wahhid_mustalah(
    muarrif: String,
    mustalah: String,
    shakl: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<u32, Khata> {
    let id = huwiya(muarrif)?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let muharrir = muharrir_mahalli(&masarat_hala)?;
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let (mut sufuf, _talifa) = mashru.iqra_nusus()?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();
    let jidhr = mashru.jidhr().to_path_buf();

    let masrad = masrad_kamil(&jidhr, &sufuf);
    let tadarubat = masrad.tadarubat(&sufuf);
    let Some(tadarub) = tadarubat.iter().find(|tadarub| tadarub.mustalah == mustalah) else {
        return Err(Khata::from(KhataWarshaAmr::TadarubGhayrMawjud { mustalah }));
    };
    let taadilat = wahhid_tadarub(tadarub, &shakl, &sufuf);

    let mut tarikh: TarikhMashru = iqra_janibi(&jidhr.join(UDW_TARIKH))?;
    let mut adad = 0_u32;
    for tadeel in &taadilat {
        let Some(mudkhal) = sufuf.iter_mut().find(|mudkhal| mudkhal.id == tadeel.id) else {
            continue;
        };
        // A target that moved since the conflict was computed is a string somebody is
        // working on; it is skipped, never overwritten.
        if mudkhal.hadaf.as_deref() != Some(tadeel.min.as_str()) {
            continue;
        }
        let kana_aali = matches!(
            mudkhal.tareeqa,
            Some(TareeqaTarjama::AaliyaFaqat | TareeqaTarjama::AaliyaThumBashariya)
        );
        mudkhal.hadaf = Some(tadeel.ila.clone());
        mudkhal.muraja.sajjil_musawwada(muharrir.clone(), lahza);
        if kana_aali {
            mudkhal.tareeqa = Some(TareeqaTarjama::AaliyaThumBashariya);
        } else {
            mudkhal.tareeqa = Some(TareeqaTarjama::BashariyaKamila);
            mudkhal.muzawwid = None;
        }
        mudkhal.muharrir = Some(muharrir.clone());
        mudkhal.akhir_tabdeel = Some(waqt.clone());
        let _ = tarikh.sajjil_tabdeel(
            tadeel.id,
            Some(tadeel.min.clone()),
            Some(tadeel.ila.clone()),
            muharrir.clone(),
            waqt.clone(),
            Some("توحيد مصطلح عبر المشروع".to_owned()),
        );
        adad = adad.saturating_add(1);
    }
    uktub_janibi(&jidhr.join(UDW_TARIKH), &tarikh)?;

    let thiqat = BTreeMap::new();
    aid_hisab_alamat(&masarat_hala, &idadat.hali(), &mut sufuf, &jidhr, &thiqat);
    mashru.uktub_kul(&sufuf, waqt)?;
    Ok(adad)
}

/// The selected string with the editor's current text, measured through the real layout
/// engine; an unmeasured constraint answers unverifiable, never passing.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`], [`KhataWarshaAmr::NassGhayrMawjud`], and whatever
/// the project store raises. A layout refusal is answered as unverifiable, not raised.
#[tauri::command]
#[specta::specta]
pub fn muayana(
    muarrif: String,
    nass: String,
    hadaf: String,
    masarat_hala: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<MuayanaHie, Khata> {
    let id = huwiya(muarrif)?;
    let nass_id = huwiyat_nass(&nass)?;
    let mashru = iftah_mashru(&masarat_hala, id)?;
    let (sufuf, _talifa) = mashru.iqra_nusus()?;
    let Some(mudkhal) = sufuf.iter().find(|mudkhal| mudkhal.id == nass_id) else {
        return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
    };
    let mutah_maruf = mudkhal.quyud.aqsa_ard.map(f64::from);

    let hali = idadat.hali();
    let Some(silsila) = silsilat_khutut(&masarat_hala, &hali) else {
        return Ok(MuayanaHie {
            ghayr_qabil: true,
            sabab_ghayr_qabil: Some(
                "لا خط عربي صالح في مجلد الخطوط؛ لا قياس بلا خط.".to_owned(),
            ),
            hajm: None,
            mutah: mutah_maruf,
            sutur: Vec::new(),
            tajawuz_biksil: None,
            tajawuz_nisba: None,
        });
    };
    let Some(hajm) = mudkhal.quyud.hajm_khatt else {
        return Ok(MuayanaHie {
            ghayr_qabil: true,
            sabab_ghayr_qabil: Some(
                "لم يُرصد حجم الخط لهذا النص؛ قياسٌ بغير حجمه الحقيقي ادّعاء.".to_owned(),
            ),
            hajm: None,
            mutah: mutah_maruf,
            sutur: Vec::new(),
            tajawuz_biksil: None,
            tajawuz_nisba: None,
        });
    };

    let khiyarat = KhiyaratTakhtit {
        satr_wahid: mudkhal.quyud.satr_wahid,
        ..KhiyaratTakhtit::default()
    };
    let talab = TalabTakhtit {
        nass: &hadaf,
        khutut: &silsila,
        hajm,
        ard_mutah: mudkhal.quyud.aqsa_ard,
        irtifa_mutah: mudkhal.quyud.aqsa_irtifa,
        nitaqat: &[],
        khiyarat: &khiyarat,
    };
    let mut saff = Saff::jadeed();
    let takhtit = match saff.khattit(&talab) {
        Ok(takhtit) => takhtit,
        Err(khata) => {
            return Ok(MuayanaHie {
                ghayr_qabil: true,
                sabab_ghayr_qabil: Some(khata.arabi),
                hajm: Some(f64::from(hajm)),
                mutah: mutah_maruf,
                sutur: Vec::new(),
                tajawuz_biksil: None,
                tajawuz_nisba: None,
            });
        }
    };

    let sutur = takhtit
        .sutur
        .iter()
        .map(|satr| SatrMuayanaHie {
            nass: hadaf
                .get(
                    usize::try_from(satr.mantiqi.start).unwrap_or(0)
                        ..usize::try_from(satr.mantiqi.end).unwrap_or(0),
                )
                .unwrap_or("")
                .to_owned(),
            ard: f64::from(satr.ard),
        })
        .collect();

    match mudkhal.quyud.aqsa_ard {
        Some(mutah) => {
            let (zaid, nisba) = takhtit
                .tajawuz
                .as_ref()
                .map_or((0.0_f32, 0.0_f32), |taqreer| (taqreer.zaid(), taqreer.nisba()));
            Ok(MuayanaHie {
                ghayr_qabil: false,
                sabab_ghayr_qabil: None,
                hajm: Some(f64::from(takhtit.hajm)),
                mutah: Some(f64::from(mutah)),
                sutur,
                tajawuz_biksil: Some(f64::from(zaid)),
                tajawuz_nisba: Some(f64::from(nisba)),
            })
        }
        None => Ok(MuayanaHie {
            ghayr_qabil: true,
            sabab_ghayr_qabil: Some(
                "العرض المتاح لهذا النص غير معروف؛ القياس حقيقي والحكم ممتنع.".to_owned(),
            ),
            hajm: Some(f64::from(takhtit.hajm)),
            mutah: None,
            sutur,
            tajawuz_biksil: None,
            tajawuz_nisba: None,
        }),
    }
}

pub(crate) fn taaliq_hie(taaliq: &Taaliq) -> TaaliqWarshaHie {
    TaaliqWarshaHie {
        nass: taaliq.mawdi().map(|mawdi| mawdi.to_string()),
        kaatib: taaliq.kaatib().mukhtasar(),
        min_almalik: taaliq.min_almalik(),
        matn: taaliq.matn().nass().to_owned(),
        waqt: taaliq.waqt().to_owned(),
        muhall: taaliq.muhall(),
    }
}

/// The anchored comments travelling with this project; none is an empty list, never an
/// invention.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`] and [`KhataWarshaAmr::MalafTalif`] when the
/// comment file exists and does not read.
#[tauri::command]
#[specta::specta]
pub fn taaliqat_warsha(
    muarrif: String,
    masarat_hala: tauri::State<'_, Masarat>,
) -> Result<Vec<TaaliqWarshaHie>, Khata> {
    let id = huwiya(muarrif)?;
    let mashru = iftah_mashru(&masarat_hala, id)?;
    let taaliqat: Vec<Taaliq> = iqra_janibi(&mashru.jidhr().join(UDW_TAALIQAT))?;
    Ok(taaliqat.iter().map(taaliq_hie).collect())
}

fn janib_hie(janib: &BitaqatJanib) -> JanibNizaaHie {
    JanibNizaaHie {
        hadaf: janib.hadaf.clone(),
        hala: janib.hala.wasf_arabi().to_owned(),
        tareeqa_arabi: janib.tareeqa.map(|tareeqa| tareeqa.wasf_arabi().to_owned()),
        muzawwid: janib.muzawwid.clone(),
        muharrir: janib.muharrir.as_ref().map(MusahimId::mukhtasar),
        akhir_tabdeel: janib.akhir_tabdeel.clone(),
    }
}

fn nizaa_hie(nizaa: &Nizaa) -> NizaaHie {
    NizaaHie {
        nass: nizaa.nass.to_string(),
        masdar: nizaa.masdar.clone(),
        naw: match nizaa.naw {
            NawNizaa::Tarjama => "tarjama",
            NawNizaa::Hala => "hala",
        }
        .to_owned(),
        ana: janib_hie(&nizaa.ana),
        hum: janib_hie(&nizaa.hum),
    }
}

/// Imports a colleague's bundle and merges; conflicts are held open, nothing is persisted.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`] and whatever the bundle reader raises — a torn
/// member, a hash mismatch, a bundle from a different project.
#[tauri::command]
#[specta::specta]
pub fn idmaj_huzma(
    muarrif: String,
    masar: String,
    masarat_hala: tauri::State<'_, Masarat>,
    jalasat: tauri::State<'_, JalasatDamj>,
) -> Result<NizaatHie, Khata> {
    let id = huwiya(muarrif)?;
    let mashru = iftah_mashru(&masarat_hala, id)?;
    let (sufuf, _talifa) = mashru.iqra_nusus()?;

    let muhtawa = istawrid(Path::new(&masar)).map_err(Khata::from)?;
    muhtawa.tahaqquq_tatabuq(mashru.rasm()).map_err(Khata::from)?;
    let aslaf = aslaf_mashru(mashru.jidhr()).map_err(Khata::from)?;

    let natijat_damj =
        damj(sufuf, muhtawa.nusus, aslaf.as_ref().map(|(nusus, _)| nusus.as_slice()));
    let taqreer = natijat_damj.taqreer().clone();
    let nizaat = natijat_damj.nizaat().iter().map(nizaa_hie).collect();

    let mahfudh = DamjMahfudh {
        damj: natijat_damj,
        tarikh_hum: muhtawa.tarikh,
        taaliqat_hum: muhtawa.taaliqat,
        masrad_hum: muhtawa.masrad,
    };
    let mut kharita = jalasat.0.lock();
    let _ = kharita.insert(id, mahfudh);

    Ok(NizaatHie {
        nizaat,
        mutatabiqa: adad_u32(taqreer.mutatabiqa),
        min_ana: adad_u32(taqreer.min_ana),
        min_hum: adad_u32(taqreer.min_hum),
        munfarida: adad_u32(taqreer.munfarida),
        thulathi: taqreer.thulathi,
    })
}

/// Finalizes the held merge with one explicit choice per conflict, folds the imported
/// history, comments and glossary, and records the merged table as the next ancestor.
///
/// # Errors
///
/// [`KhataWarshaAmr::LaDamjMaftuh`] when no merge is held for the project, the merge
/// layer's own refusals for missing or surplus resolutions, and whatever persisting
/// raises.
#[tauri::command]
#[specta::specta]
pub fn qarrir_nizaat(
    muarrif: String,
    qararat: Vec<QararHie>,
    masarat_hala: tauri::State<'_, Masarat>,
    jalasat: tauri::State<'_, JalasatDamj>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<DamjHie, Khata> {
    let id = huwiya(muarrif)?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    let muharrir = muharrir_mahalli(&masarat_hala)?;
    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();

    let mahfudh = {
        let mut kharita =
            jalasat.0.lock();
        kharita.remove(&id)
    };
    let Some(mahfudh) = mahfudh else {
        return Err(Khata::from(KhataWarshaAmr::LaDamjMaftuh));
    };

    let mut talabat: BTreeMap<NassId, Qarar> = BTreeMap::new();
    for qarar in qararat {
        let (nass, qeema) = match qarar {
            QararHie::KhudhLi { nass } => (nass, Qarar::KhudhLi),
            QararHie::KhudhHum { nass } => (nass, Qarar::KhudhHum),
            QararHie::Thalith { nass, hadaf } => (nass, Qarar::Thalith { nass: hadaf }),
        };
        let _ = talabat.insert(huwiyat_nass(&nass)?, qeema);
    }

    let (mut sufuf, husum) =
        mahfudh.damj.itmam(&talabat, &muharrir, lahza).map_err(Khata::from)?;
    let jidhr = mashru.jidhr().to_path_buf();

    let tarikh_ana: TarikhMashru = iqra_janibi(&jidhr.join(UDW_TARIKH))?;
    let mut tarikh = damj_tarikh(&tarikh_ana, &mahfudh.tarikh_hum);
    let _ = tarikh.adkhil_husum(&husum, &sufuf, &waqt);
    uktub_janibi(&jidhr.join(UDW_TARIKH), &tarikh)?;

    let mut taaliqat: Vec<Taaliq> = iqra_janibi(&jidhr.join(UDW_TAALIQAT))?;
    for taaliq in mahfudh.taaliqat_hum {
        if !taaliqat.iter().any(|mawjud| mawjud.id() == taaliq.id()) {
            taaliqat.push(taaliq);
        }
    }
    uktub_janibi(&jidhr.join(UDW_TAALIQAT), &taaliqat)?;

    let masar_masrad = jidhr.join(UDW_MASRAD);
    let mahalli: Vec<MustalahMasrad> = if masar_masrad.is_file() {
        Masrad::min_malaf(&masar_masrad).map_err(Khata::from)?
    } else {
        Vec::new()
    };
    let mut masrad = Masrad::bi_mustalahat(mahalli);
    masrad.admij(mahfudh.masrad_hum);
    let mustalahat: Vec<&MustalahMasrad> = masrad.mustalahat().collect();
    uktub_janibi(&masar_masrad, &mustalahat)?;

    let _ = thabbit_tanaqud(&mut sufuf);
    mashru.uktub_kul(&sufuf, waqt)?;
    sajjil_aslaf(&jidhr, &sufuf).map_err(Khata::from)?;

    Ok(DamjHie { sufuf: adad_u32(sufuf.len()), husum: adad_u32(husum.len()) })
}

/// Failures of the workspace surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataWarshaAmr {
    /// No translation project exists for this game yet.
    #[error("no translation project exists for {ism}")]
    MashruGhayrMawjud {
        /// The game's identity.
        ism: String,
    },

    /// The table holds no string under the identity the interface sent.
    #[error("{nass} is not a string in this project")]
    NassGhayrMawjud {
        /// The identity that was looked up.
        nass: String,
    },

    /// No machine-translation provider is configured and enabled.
    #[error("no machine-translation provider is configured")]
    LaMuzawwid,

    /// The provider is configured but its credential is not in the keychain.
    #[error("no credential in the keychain for provider {muzawwid}")]
    LaItimad {
        /// The provider's identifier.
        muzawwid: String,
    },

    /// The provider cannot be built as configured, for the stated reason.
    #[error("provider {muzawwid} cannot be used: {sabab_injilizi}")]
    MuzawwidGhayrMadum {
        /// The provider's identifier.
        muzawwid: String,
        /// Why, in Arabic.
        sabab_arabi: String,
        /// Why, in English.
        sabab_injilizi: String,
    },

    /// The memory no longer offers that record for this string.
    #[error("memory record {qayd} is not offered for this string")]
    IqtirahGhayrMawjud {
        /// The record's identity.
        qayd: i64,
    },

    /// The terminology conflict named is no longer present.
    #[error("no terminology conflict on {mustalah}")]
    TadarubGhayrMawjud {
        /// The term that was named.
        mustalah: String,
    },

    /// No merge is held open for this project.
    #[error("no merge is held open for this project")]
    LaDamjMaftuh,

    /// The batch ceiling is not a positive finite amount.
    #[error("the batch cost ceiling is not a positive finite amount")]
    SaqfGhayrSalih,

    /// A file beside the project exists and does not read.
    #[error("{} does not read: {sabab}", masar.display())]
    MalafTalif {
        /// The file.
        masar: PathBuf,
        /// What the reader said.
        sabab: String,
    },
}

impl Tafsir for KhataWarshaAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MashruGhayrMawjud { .. } => 40,
                    Self::NassGhayrMawjud { .. } => 41,
                    Self::LaMuzawwid => 42,
                    Self::LaItimad { .. } => 43,
                    Self::MuzawwidGhayrMadum { .. } => 44,
                    Self::IqtirahGhayrMawjud { .. } => 45,
                    Self::TadarubGhayrMawjud { .. } => 46,
                    Self::LaDamjMaftuh => 47,
                    Self::SaqfGhayrSalih => 48,
                    Self::MalafTalif { .. } => 49,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Asked about something that is not there; nothing was touched.
            Self::MashruGhayrMawjud { .. }
            | Self::NassGhayrMawjud { .. }
            | Self::IqtirahGhayrMawjud { .. }
            | Self::TadarubGhayrMawjud { .. }
            | Self::LaDamjMaftuh
            | Self::SaqfGhayrSalih => Khutura::Tanbeeh,
            // A run was requested and cannot start; the user can fix the settings.
            Self::LaMuzawwid | Self::LaItimad { .. } | Self::MuzawwidGhayrMadum { .. } => {
                Khutura::Khatar
            }
            // A file beside somebody's work does not read.
            Self::MalafTalif { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MashruGhayrMawjud { .. } => {
                "لا مشروع ترجمة لهذه اللعبة بعد. ابدأ الترجمة من شاشة اللعبة أولًا."
                    .to_owned()
            }
            Self::NassGhayrMawjud { .. } => {
                "هذا النص لم يعد في جدول المشروع. أعد فتح الورشة لتحميل الجدول الحالي."
                    .to_owned()
            }
            Self::LaMuzawwid => {
                "لا مزوّد ترجمة آلية مهيّأ ومفعّل. أضف مزوّدًا في الإعدادات ثم عد."
                    .to_owned()
            }
            Self::LaItimad { muzawwid } => format!(
                "لا اعتماد في سلسلة مفاتيح النظام للمزوّد {muzawwid}. أدخل مفتاحه في \
                 الإعدادات ليُخزَّن في السلسلة."
            ),
            Self::MuzawwidGhayrMadum { sabab_arabi, .. } => sabab_arabi.clone(),
            Self::IqtirahGhayrMawjud { .. } => {
                "لم تعد الذاكرة تعرض هذا القيد لهذا النص. حدّث الاقتراحات ثم اختر من \
                 جديد."
                    .to_owned()
            }
            Self::TadarubGhayrMawjud { mustalah } => format!(
                "لا تضارب مصطلحات على «{mustalah}» الآن؛ ربما حُسم في تحرير سابق. أعد \
                 حساب العلامات."
            ),
            Self::LaDamjMaftuh => {
                "لا دمج معلّقًا لهذا المشروع. استورد حزمة زميل أولًا ثم احسم تعارضاتها."
                    .to_owned()
            }
            Self::SaqfGhayrSalih => {
                "سقف التكلفة يجب أن يكون مبلغًا موجبًا محدودًا بالدولار.".to_owned()
            }
            Self::MalafTalif { masar, .. } => format!(
                "الملف {} موجود ولا يُقرأ. لن يُكتب فوقه؛ افحصه أو انقله ثم أعد المحاولة.",
                masar.display()
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MashruGhayrMawjud { ism } => format!(
                "No translation project exists for {ism} yet. Start translating from the \
                 game screen first."
            ),
            Self::NassGhayrMawjud { nass } => format!(
                "{nass} is no longer a string in this project. Reopen the workspace to \
                 load the current table."
            ),
            Self::LaMuzawwid => {
                "No machine-translation provider is configured and enabled. Add one in \
                 Settings, then return."
                    .to_owned()
            }
            Self::LaItimad { muzawwid } => format!(
                "The system keychain holds no credential for provider {muzawwid}. Enter \
                 its key in Settings so it is stored there."
            ),
            Self::MuzawwidGhayrMadum { sabab_injilizi, .. } => sabab_injilizi.clone(),
            Self::IqtirahGhayrMawjud { qayd } => format!(
                "The memory no longer offers record {qayd} for this string. Refresh the \
                 suggestions and pick again."
            ),
            Self::TadarubGhayrMawjud { mustalah } => format!(
                "There is no terminology conflict on \"{mustalah}\" now; it may have been \
                 resolved by an earlier edit. Recompute the quality flags."
            ),
            Self::LaDamjMaftuh => {
                "No merge is held open for this project. Import a colleague's bundle \
                 first, then resolve its conflicts."
                    .to_owned()
            }
            Self::SaqfGhayrSalih => {
                "The cost ceiling must be a positive, finite dollar amount.".to_owned()
            }
            Self::MalafTalif { masar, sabab } => format!(
                "{} exists and does not read ({sabab}). Nothing will be written over it; \
                 inspect or move it, then retry.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MashruGhayrMawjud { .. } => Khutwa::FathNusus,
            Self::NassGhayrMawjud { .. }
            | Self::IqtirahGhayrMawjud { .. }
            | Self::TadarubGhayrMawjud { .. }
            | Self::LaDamjMaftuh
            | Self::SaqfGhayrSalih
            | Self::MalafTalif { .. } => Khutwa::AadaMuhawala,
            Self::LaMuzawwid | Self::LaItimad { .. } | Self::MuzawwidGhayrMadum { .. } => {
                Khutwa::FathIdadat { qism: QismIdadat::Muzawwidun }
            }
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MashruGhayrMawjud { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::NassGhayrMawjud { nass } => {
                let _ = siyaq.insert("nass".to_owned(), QeemaSiyaq::Nass(nass.clone()));
            }
            Self::LaItimad { muzawwid } | Self::MuzawwidGhayrMadum { muzawwid, .. } => {
                let _ =
                    siyaq.insert("muzawwid".to_owned(), QeemaSiyaq::Nass(muzawwid.clone()));
            }
            Self::IqtirahGhayrMawjud { qayd } => {
                let _ = siyaq.insert("qayd".to_owned(), QeemaSiyaq::Raqm(*qayd));
            }
            Self::TadarubGhayrMawjud { mustalah } => {
                let _ =
                    siyaq.insert("mustalah".to_owned(), QeemaSiyaq::Nass(mustalah.clone()));
            }
            Self::MalafTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::LaMuzawwid | Self::LaDamjMaftuh | Self::SaqfGhayrSalih => {}
        }
        siyaq
    }
}

khata_min!(KhataWarshaAmr);
