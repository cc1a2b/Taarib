//! الورشة — the translation workspace's command surface: the table, the edits, the suggestions,
//! the machine runs, the quality summary, the preview, the comments, and the merge.

use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use taarib_istikhraj::mashru::{MALAF_MASHRU, MALAF_NUSUS, MashruMaftuh};
// The run's own table file, whose name collides with the project's; aliased so
// the two cannot be confused at a call site.
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muraja::{HalatMuraja, SijillMuraja};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId, TasnifNass};
use taarib_mustalahat::ruqaa::TareeqaTarjama;
use taarib_saff::Saff;
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_saff::talab::{KhiyaratTakhtit, TalabTakhtit};
use taarib_taqdeem::taaliq::Taaliq;
use taarib_tarjama::alamat::{
    AtabatAlamat, MudkhalatQiyas, ThiqaMublagha, ihsib_mashru, ihsib_wa_thabbit, thabbit_tanaqud,
};
use taarib_tarjama::dhakira::{AslQayd, Dhakira, QaydDhakira, QaydId, QaydJadid, TalabDhakira};
use taarib_tarjama::dufaat::{
    KhiyaratJawla, SijillJawla, TaqaddumJawla, shaghghil_jawla, tabbiq_sijill, thiqat_min_sijill,
};
use taarib_tarjama::khata::KhataTarjama;
use taarib_tarjama::masrad::{
    Masrad, MustalahMasrad, NitaqMustalah, TadarubMustalah, wahhid_tadarub,
};
use taarib_tarjama::muzawwidun::{
    IdadatAnthropic, IdadatDeepL, IdadatGemini, IdadatGoogleMajjani, IdadatMicrosoft,
    IdadatMuwafiqOpenAI, IdhnInfaq, Itimad, Muzawwid, MuzawwidAnthropic, MuzawwidDeepL,
    MuzawwidGemini, MuzawwidGoogleMajjani, MuzawwidMicrosoft, MuzawwidMuwafiqOpenAI,
    taklifat_anthropic, taklifat_gemini,
};
use taarib_tathbeet::bayan::waqt_alaan;
use taarib_tilqai::khata::KhataTilqai;
use taarib_tilqai::mashwar::{MALAF_NUSUS as MALAF_NUSUS_MASHWAR, MUJALLAD_MASHRU, ijrud};
use taarib_usus::idadat::{HalatMuzawwidin, Idadat, IdadatMuzawwid, MakhzanIdadat, NawMuzawwid};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, kitaba_dharra};
use taarib_warsha::damj::{BitaqatJanib, Damj, NawNizaa, Nizaa, Qarar, damj};
use taarib_warsha::salama::{self, HalatNusus, QiraatNusus, SatrTalif, TaqreerInqadh};
use taarib_warsha::tarikh::{TarikhMashru, damj_tarikh};
use taarib_warsha::tasdir::{
    UDW_MASRAD, UDW_TAALIQAT, UDW_TARIKH, UDW_TAWZI, aslaf_mashru, istawrid, sajjil_aslaf,
};
use taarib_warsha::tawzi::Tawzi;
use tauri::Emitter as _;

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

/// One line of the string file that did not read.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrTalifHie {
    /// The line's number in the file, counting from one.
    pub raqm: u32,
    /// The string's identity, when the line's head survived far enough to carry it.
    pub huwiya: Option<String>,
    /// The source text, when it survived.
    pub masdar: Option<String>,
    /// The line's opening, for the eye.
    pub muqtataf: String,
    /// The line, its reason and what it was, as one sentence in Arabic.
    pub wasf_arabi: String,
    /// The same sentence in English.
    pub wasf_injilizi: String,
}

/// Whether the string file read whole — three states no screen may confuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HalatNususHie {
    /// No file, or no rows: nothing has been extracted yet.
    Farigh,
    /// Every line read.
    Salima,
    /// At least one line did not read.
    Talifa,
}

/// The damage, when there is any: what did not read, and the sentences that say so.
///
/// The sentences travel from here rather than from the interface's string set,
/// so the count, the path and the advice are worded once, in both languages, by
/// the code that knows them.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TalafHie {
    /// How many lines did not read.
    pub talifa: u32,
    /// The lines that did not read, at most [`AQSA_SUTUR_TALIFA`] of them.
    pub sutur: Vec<SatrTalifHie>,
    /// The panel's title, in Arabic.
    pub unwan_arabi: String,
    /// The same title in English.
    pub unwan_injilizi: String,
    /// The short form beside the row count, in Arabic.
    pub mukhtasar_arabi: String,
    /// The same short form in English.
    pub mukhtasar_injilizi: String,
    /// The rescue button's label, in Arabic.
    pub zir_arabi: String,
    /// The same label in English.
    pub zir_injilizi: String,
}

/// What the workspace knows about the string file's integrity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SalamatMashruHie {
    /// Which of the three states the file is in.
    pub hala: HalatNususHie,
    /// How many rows read.
    pub najin: u32,
    /// The string file's path, for the user who wants to look at it.
    pub masar: String,
    /// The one sentence for this state, in Arabic.
    pub wasf_arabi: String,
    /// The same sentence in English.
    pub wasf_injilizi: String,
    /// The damage, exactly when `hala` is [`HalatNususHie::Talifa`].
    pub talaf: Option<TalafHie>,
}

/// What setting the damaged rows aside did, and where everything went.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct InqadhHie {
    /// How many rows the live table now holds.
    pub najin: u32,
    /// How many lines were set aside.
    pub talifa: u32,
    /// The damaged file, preserved byte for byte.
    pub mahfudh: String,
    /// The unreadable lines alone.
    pub marfud: String,
    /// The report naming every line set aside.
    pub taqreer: String,
    /// What happened, in Arabic.
    pub wasf_arabi: String,
    /// The same in English.
    pub wasf_injilizi: String,
}

/// What the configured provider list amounts to, as the settings crate states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HalatMuzawwidinHie {
    /// Not one provider has been added.
    Farigh,
    /// Providers exist and every one is switched off.
    Muattala,
    /// The elected provider is the one the user chose.
    Mukhtar,
    /// The default names a disabled or deleted provider, so another one is used — and billed.
    Badeel,
}

/// The provider a new translation would use, said before anything is spent.
///
/// The election is the settings crate's and so is the sentence; the workspace
/// only carries them, so a stale default that quietly bills a different
/// provider — or the built-in free provider standing in for an empty list — is
/// read on the screen where the run is started, not discovered on the invoice
/// or in the quality of the Arabic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MuzawwidWarshaHie {
    /// Which of the four states the list is in.
    pub hala: HalatMuzawwidinHie,
    /// The identifier of the provider a new translation uses: the elected one,
    /// or the built-in free one when the list elects nothing.
    pub ism: String,
    /// The state's own sentence, in Arabic.
    pub wasf_arabi: String,
    /// The same sentence in English.
    pub wasf_injilizi: String,
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
    /// How many rows the table holds — the rows that read, never the rows the file holds.
    pub adad: u32,
    /// Every row that read.
    pub sufuf: Vec<SafWarshaHie>,
    /// Whether the file read whole, and what to say when it did not.
    pub salama: SalamatMashruHie,
    /// The provider a run from this screen would use, and whether that is the user's choice.
    pub muzawwid: MuzawwidWarshaHie,
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
    /// Whether the product shipped this term rather than the project pinning
    /// it; a project entry for the same source form overrides the built-in one.
    pub mudmaj: bool,
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
    /// How many rows of the recorded ancestor did not read.
    ///
    /// Those strings were merged without an ancestor, which is the safe
    /// direction — more conflicts, never a silent choice — but a merge that
    /// quietly demoted itself would be this codebase's oldest defect again.
    pub aslaf_talifa: u32,
    /// The sentence about the ancestor, in Arabic, when `aslaf_talifa` is not zero.
    pub tanbih_arabi: Option<String>,
    /// The same sentence in English.
    pub tanbih_injilizi: Option<String>,
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
    kharita
        .entry(id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

async fn qufl_mashru_async(aqfal: &AqfalMashariya, id: LubaId) -> Arc<tokio::sync::Mutex<()>> {
    let mut kharita = aqfal.0.lock().await;
    kharita
        .entry(id)
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

fn khata_malaf(masar: &Path, sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataWarshaAmr::MalafTalif {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

pub(crate) fn lahza_alaan() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |muddat| muddat.as_secs())
}

fn jidhr_mashru(masarat_hala: &Masarat, id: LubaId) -> PathBuf {
    masarat_hala.mashari().join(id.to_string())
}

/// Opens the project for one game, refusing when none has been created yet.
///
/// A game with no project but with a finished automatic run behind it is
/// recovered rather than refused; see [`istaid_min_mashwar`].
fn iftah_mashru(masarat_hala: &Masarat, id: LubaId) -> Natija<MashruMaftuh> {
    let jidhr = jidhr_mashru(masarat_hala, id);
    if !jidhr.join(MALAF_MASHRU).is_file() {
        if let Some(mashru) = istaid_min_mashwar(masarat_hala, id, &jidhr)? {
            return Ok(mashru);
        }
        return Err(Khata::from(KhataWarshaAmr::MashruGhayrMawjud {
            ism: id.to_string(),
        }));
    }
    MashruMaftuh::iftah(jidhr).map_err(Khata::from)
}

/// Builds the workshop's project out of this game's newest automatic run.
///
/// Runs written before the run learned to publish its rows — and every run this
/// product shipped until it did — left their table under the run's own
/// identifier and nothing under the game's. The workshop then told somebody who
/// had finished a translation that no project existed and to start one. The
/// rows are on disk the whole time; this is the one read that connects them.
///
/// Newest run, and only one: the runs under a game are the same table
/// translated again, so the newest is the most complete, and folding older ones
/// in would resurrect strings a later extraction stopped finding. Rows are taken
/// whole — their translations, their review state, their provenance — because
/// they are the same `MudkhalNass` the workshop writes.
///
/// `None` when there is no run, no table in it, or nothing readable in the
/// table: those are genuinely "no project yet", and the refusal above is the
/// honest answer. A table past [`taarib_tilqai::tarjama::HADD_HAJM_NUSUS`] is
/// not one of them — it is raised, because nothing on that path ever reaches a
/// project and the user is the only one who can clear it.
fn istaid_min_mashwar(
    masarat_hala: &Masarat,
    id: LubaId,
    jidhr_mashru_hali: &Path,
) -> Natija<Option<MashruMaftuh>> {
    let jidhr_mashawir = crate::tilqai_awamir::jidhr_mashawir(masarat_hala, id);
    let Some(mawjuz) = ijrud(&jidhr_mashawir).into_iter().next() else {
        return Ok(None);
    };
    let masar_nusus = mawjuz
        .mujallad
        .join(MUJALLAD_MASHRU)
        .join(MALAF_NUSUS_MASHWAR);
    if !masar_nusus.is_file() {
        return Ok(None);
    }
    let sufuf = match taarib_tilqai::tarjama::iqra_nusus(&masar_nusus) {
        Ok(sufuf) => sufuf,
        // Raised rather than reported, unlike every other table that will not
        // read. This one was refused before a byte was taken, so "no project
        // yet" would hide it behind a button whose whole job is to build a run
        // — and the next open would meet the same file again.
        Err(KhataTilqai::NususKabira { hajm, hadd, .. }) => {
            return Err(Khata::from(KhataWarshaAmr::JadwalMashwarKabir {
                masar: masar_nusus,
                hajm,
                hadd,
            }));
        },
        Err(sabab) => {
            // Reported and not raised: an unreadable run table is a run's
            // problem, and answering "no project yet" is both true and
            // actionable — pressing translate again rebuilds it.
            tracing::warn!(
                masar = %masar_nusus.display(),
                %sabab,
                "a finished run's table would not read; the workshop is left with no project"
            );
            return Ok(None);
        },
    };
    if sufuf.is_empty() {
        return Ok(None);
    }

    let sijill = taarib_tilqai::mashwar::SijillMashwar::iftah(&mawjuz.mujallad).ok();
    let imkaniyat = sijill.as_ref().and_then(|sijill| {
        match sijill.qayd(taarib_tilqai::taqaddum::MarhalaTilqai::Fahs) {
            Some(taarib_tilqai::mashwar::QaydMarhala::Fahs { imkaniyat }) => {
                Some(imkaniyat.as_ref())
            },
            _ => None,
        }
    });
    let bayan = taarib_tilqai::warsha::bayan(
        imkaniyat,
        rafd_mashwar(&mawjuz.mujallad),
        taarib_usus::ISDAR,
        &mawjuz.waqt,
    );

    let adad = sufuf.len();
    taarib_tilqai::warsha::anshir(
        jidhr_mashru_hali,
        id,
        &mawjuz.ism_luba,
        bayan,
        &sufuf,
        &mawjuz.waqt,
    )
    .map_err(Khata::from)?;
    tracing::info!(
        luba = %id,
        mashwar = %mawjuz.id,
        adad,
        "the workshop's project was recovered from a finished automatic run"
    );
    MashruMaftuh::iftah(jidhr_mashru_hali.to_path_buf())
        .map(Some)
        .map_err(Khata::from)
}

/// The refusal report a finished run stored beside its table, or an empty one.
///
/// Only the refusal report is deserialized: the same file holds the whole
/// string table, and building a hundred thousand rows to fill in a field that
/// answers "why does this project only have the menus" is a cost paid for
/// nothing.
fn rafd_mashwar(mujallad: &Path) -> taarib_istikhraj::rafd::TaqreerRafd {
    /// The stored table, read for its refusal report and nothing else.
    #[derive(serde::Deserialize)]
    struct RafdFaqat {
        /// What was read and what was refused.
        rafd: taarib_istikhraj::rafd::TaqreerRafd,
    }

    let masar = mujallad
        .join(MUJALLAD_MASHRU)
        .join(taarib_tilqai::mashwar::MALAF_JADWAL);
    let Ok(malaf) = std::fs::File::open(&masar) else {
        return taarib_istikhraj::rafd::TaqreerRafd::default();
    };
    serde_json::from_reader::<_, RafdFaqat>(std::io::BufReader::new(malaf))
        .map(|makhzun| makhzun.rafd)
        .unwrap_or_default()
}

/// The table for a command that only reads it: the rows that read, with the damage beside them.
fn qiraat_nusus(mashru: &MashruMaftuh) -> Natija<QiraatNusus> {
    salama::iqra_nusus(mashru).map_err(Khata::from)
}

/// The table for a command that will write it back, refused whole when any row did not read.
///
/// A rewrite from a partial read makes the loss permanent with no message, so a damaged
/// file is written to by nothing but the rescue the user chooses.
fn sufuf_lil_kitaba(mashru: &MashruMaftuh) -> Natija<Vec<MudkhalNass>> {
    let qiraa = qiraat_nusus(mashru)?;
    if !qiraa.salima() {
        return Err(Khata::from(KhataWarshaAmr::MashruTalif {
            talifa: qiraa.talifa.len(),
            masar: mashru.jidhr().join(MALAF_NUSUS),
        }));
    }
    Ok(qiraa.sufuf)
}

/// Parses a string identity the interface sent back.
fn huwiyat_nass(nass: &str) -> Natija<NassId> {
    serde_json::from_value::<NassId>(serde_json::Value::String(nass.to_owned())).map_err(|_| {
        Khata::from(KhataWarshaAmr::NassGhayrMawjud {
            nass: nass.to_owned(),
        })
    })
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
    let bayt = serde_json::to_vec(&MalafMuharrir {
        musahim: musahim.nass().to_owned(),
    })
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
        crate::mukawwinat_tahmil::judhur_khutut(masarat_hala)
            .into_iter()
            .skip(1),
    );
    let mut khutut = Vec::new();
    for masar in crate::mukawwinat_tahmil::milaffat_khutut(&mujalladat) {
        let Ok(bayt) = std::fs::read(&masar) else {
            continue;
        };
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
        tareeqa_arabi: mudkhal
            .tareeqa
            .map(|tareeqa| tareeqa.wasf_arabi().to_owned()),
        muzawwid: mudkhal.muzawwid.clone(),
        muayyan: tawzi.mukallaf(mudkhal.id).map(MusahimId::mukhtasar),
        aqsa_ard: mudkhal.quyud.aqsa_ard.map(f64::from),
        hajm_khatt: mudkhal.quyud.hajm_khatt.map(f64::from),
    }
}

/// How many damaged lines travel to the screen; the report the rescue writes names them all.
pub const AQSA_SUTUR_TALIFA: usize = 200;

/// The longest source text quoted inside a damaged line's sentence, in characters.
const AQSA_MASDAR_FI_JUMLA: usize = 60;

/// A count of lines in Arabic, in the form the number demands; `majrur` picks the
/// form that follows a preposition or a verbal noun, which differs only at two.
fn sutur_arabi(adad: usize, majrur: bool) -> String {
    match adad {
        0 => "لا أسطر".to_owned(),
        1 => "سطر واحد".to_owned(),
        2 if majrur => "سطرين".to_owned(),
        2 => "سطران".to_owned(),
        3..=10 => format!("{adad} أسطر"),
        _ => format!("{adad} سطرًا"),
    }
}

/// A count of rows in English.
fn sutur_injilizi(adad: usize) -> String {
    if adad == 1 {
        "1 row".to_owned()
    } else {
        format!("{adad} rows")
    }
}

fn qassir(nass: &str) -> String {
    let mut ahruf = nass.chars();
    let mut awwal: String = ahruf.by_ref().take(AQSA_MASDAR_FI_JUMLA).collect();
    if ahruf.next().is_some() {
        awwal.push('…');
    }
    awwal
}

fn satr_talif_hie(satr: &SatrTalif) -> SatrTalifHie {
    let mukhtasar = satr.huwiya.map(NassId::mukhtasar);
    let masdar_qasir = satr.masdar.as_deref().map(qassir);
    let (dhayl_arabi, dhayl_injilizi) = match (masdar_qasir, mukhtasar) {
        (Some(masdar), _) => (
            format!("النص الأصلي: «{masdar}»"),
            format!("source: \"{masdar}\""),
        ),
        (None, Some(id)) => (format!("الهوية {id}"), format!("identity {id}")),
        (None, None) => (
            format!("بدايته: {}", satr.muqtataf),
            format!("opens with: {}", satr.muqtataf),
        ),
    };
    SatrTalifHie {
        raqm: adad_u32(satr.raqm),
        huwiya: satr.huwiya.map(|id| id.to_string()),
        masdar: satr.masdar.clone(),
        muqtataf: satr.muqtataf.clone(),
        wasf_arabi: format!(
            "السطر {}: {} — {dhayl_arabi}",
            satr.raqm,
            satr.sabab.wasf_arabi()
        ),
        wasf_injilizi: format!(
            "Line {}: {} — {dhayl_injilizi}",
            satr.raqm,
            satr.sabab.wasf_injilizi()
        ),
    }
}

/// The two sentences for a file that did not read whole.
///
/// They say what the count means, what was and was not done, and what the two choices
/// are — and they say not to re-extract, because re-extraction recreates the project and is
/// the advice the old empty state gave for exactly this file.
fn wasf_talaf(qiraa: &QiraatNusus, masar: &str) -> (String, String) {
    let talifa = qiraa.talifa.len();
    let najin = qiraa.sufuf.len();
    let (arabi, injilizi) = if najin == 0 {
        (
            format!(
                "تعذّرت قراءة كل أسطر ملف النصوص ({}). هذا ليس مشروعًا فارغًا: الملف موجود، \
                 حجمه {} بايت، وهو تالف. لم يُكتب شيء فوقه. لا تُعِد الاستخراج، فذلك يُنشئ \
                 المشروع من جديد ويُتلف ما بقي؛ افحص الملف {masar}، أو أبقِ أسطره جانبًا وتابع \
                 بجدول فارغ مع بقاء الأصل محفوظًا.",
                sutur_arabi(talifa, true),
                qiraa.hajm
            ),
            format!(
                "Every row of the string file failed to read ({}). This is not an empty \
                 project: the file exists, is {} bytes long, and is damaged. Nothing has been \
                 written over it. Do not re-extract — that recreates the project and destroys \
                 what is left; inspect {masar}, or set its rows aside and continue with an \
                 empty table while the original stays preserved.",
                sutur_injilizi(talifa),
                qiraa.hajm
            ),
        )
    } else {
        (
            format!(
                "تعذّرت قراءة {} من ملف النصوص، وما قُرئ منه {}. هذا ليس مشروعًا ناقصًا، بل \
                 ملفٌ فيه عطب: كتابة انقطعت، أو خطأ في القرص، أو شكل كتبته نسخة أخرى. لم \
                 يُكتب شيء فوق الملف، ولن يُكتب حتى تختار: أبقِ الأسطر التالفة جانبًا وتابع \
                 على ما قُرئ، أو أغلق الورشة وافحص الملف {masar} بنفسك. لا تُعِد الاستخراج \
                 قبل أن تحفظ نسخة منه، فإعادة الاستخراج تُنشئ المشروع من جديد.",
                sutur_arabi(talifa, true),
                sutur_arabi(najin, false)
            ),
            format!(
                "{} of the string file could not be read; {} read. This is not a project that \
                 is merely incomplete — it is a file with damage in it: a write that was cut \
                 off, a disk fault, or a shape written by another build. Nothing has been \
                 written over the file, and nothing will be until you choose: set the damaged \
                 rows aside and continue on what read, or close the workshop and inspect \
                 {masar} yourself. Do not re-extract before keeping a copy of it; re-extraction \
                 recreates the project.",
                sutur_injilizi(talifa),
                sutur_injilizi(najin)
            ),
        )
    };
    if talifa > AQSA_SUTUR_TALIFA {
        let dhayl_arabi = format!(
            " تُعرض أدناه أول {} من التالفة؛ التقرير الذي يُكتب عند الإبقاء جانبًا يسمّيها كلّها.",
            sutur_arabi(AQSA_SUTUR_TALIFA, true)
        );
        let dhayl_injilizi = format!(
            " The first {} are listed below; the report written when they are set aside names \
             them all.",
            sutur_injilizi(AQSA_SUTUR_TALIFA)
        );
        return (arabi + &dhayl_arabi, injilizi + &dhayl_injilizi);
    }
    (arabi, injilizi)
}

fn talaf_hie(qiraa: &QiraatNusus) -> TalafHie {
    let talifa = qiraa.talifa.len();
    let (unwan_arabi, unwan_injilizi) = if qiraa.sufuf.is_empty() {
        (
            "ملف النصوص تالف ولم يُقرأ منه سطر".to_owned(),
            "The string file is damaged and no row read".to_owned(),
        )
    } else {
        (
            "ملف النصوص تالف جزئيًا".to_owned(),
            "The string file is partly unreadable".to_owned(),
        )
    };
    TalafHie {
        talifa: adad_u32(talifa),
        sutur: qiraa
            .talifa
            .iter()
            .take(AQSA_SUTUR_TALIFA)
            .map(satr_talif_hie)
            .collect(),
        unwan_arabi,
        unwan_injilizi,
        mukhtasar_arabi: format!("تعذّرت قراءة {}", sutur_arabi(talifa, true)),
        mukhtasar_injilizi: if talifa == 1 {
            "1 unreadable row".to_owned()
        } else {
            format!("{talifa} unreadable rows")
        },
        zir_arabi: "أبقِ الأسطر التالفة جانبًا وتابع".to_owned(),
        zir_injilizi: "Set the damaged rows aside and continue".to_owned(),
    }
}

/// The integrity of one read, with its sentence, in the state's own words.
fn salamat_hie(qiraa: &QiraatNusus, masar: &Path) -> SalamatMashruHie {
    let najin = adad_u32(qiraa.sufuf.len());
    let masar_nass = masar.display().to_string();
    match qiraa.hala() {
        HalatNusus::Farigh => SalamatMashruHie {
            hala: HalatNususHie::Farigh,
            najin,
            masar: masar_nass,
            wasf_arabi: "لا نصوص في هذا المشروع بعد: لم يُستخرج شيء، ولا ملف تالف.".to_owned(),
            wasf_injilizi: "This project holds no strings yet: nothing has been extracted, and \
                            no file is damaged."
                .to_owned(),
            talaf: None,
        },
        HalatNusus::Salima => SalamatMashruHie {
            hala: HalatNususHie::Salima,
            najin,
            masar: masar_nass,
            wasf_arabi: "قُرئ ملف النصوص كاملًا.".to_owned(),
            wasf_injilizi: "The string file read whole.".to_owned(),
            talaf: None,
        },
        HalatNusus::Talifa => {
            let (wasf_arabi, wasf_injilizi) = wasf_talaf(qiraa, &masar_nass);
            SalamatMashruHie {
                hala: HalatNususHie::Talifa,
                najin,
                masar: masar_nass,
                wasf_arabi,
                wasf_injilizi,
                talaf: Some(talaf_hie(qiraa)),
            }
        },
    }
}

/// Opens the project for the workspace: the rows that read, folded with the run journal only
/// when every row read.
fn iftah_warsha(
    masarat_hala: &Masarat,
    hali: &Idadat,
    muarrif: String,
    id: LubaId,
) -> Natija<WarshaHie> {
    let mut mashru = iftah_mashru(masarat_hala, id)?;
    let qiraa = qiraat_nusus(&mashru)?;
    let salama = salamat_hie(&qiraa, &mashru.jidhr().join(MALAF_NUSUS));
    let salima = qiraa.salima();
    let mut sufuf = qiraa.sufuf;

    // A crash between a run's journal write and its table fold leaves finished
    // translations invisible; the fold is pure, so opening folds the remainder — but only
    // over a file that read whole. Folding over the survivors and writing them back is how
    // a damaged file becomes a shorter one with no message.
    let jidhr_mashru_hali = mashru.jidhr().to_path_buf();
    if salima {
        let sijill_jawla = SijillJawla::iftah(&jidhr_mashru_hali).map_err(Khata::from)?;
        if !sijill_jawla.quyud().is_empty() {
            let waqt = waqt_alaan();
            let tatbiq = tabbiq_sijill(&mut sufuf, &sijill_jawla, &waqt);
            if tatbiq.mutabbaqa > 0 || tatbiq.fashila_muallama > 0 {
                let thiqat = thiqat_min_sijill(&sijill_jawla);
                aid_hisab_alamat(masarat_hala, hali, &mut sufuf, &jidhr_mashru_hali, &thiqat);
                mashru.uktub_kul(&sufuf, waqt)?;
            }
        }
    }
    let tawzi: Tawzi = iqra_janibi(&mashru.jidhr().join(UDW_TAWZI))?;
    Ok(WarshaHie {
        muarrif,
        ism_luba: mashru.rasm().ism_luba.clone(),
        musahimi: muharrir_mahalli(masarat_hala)?.mukhtasar(),
        adad: adad_u32(sufuf.len()),
        sufuf: sufuf
            .iter()
            .map(|mudkhal| saf_hie(mudkhal, &tawzi))
            .collect(),
        salama,
        muzawwid: muzawwid_warsha_hie(hali),
    })
}

/// The provider list's state and elected provider, in the settings crate's own words.
fn muzawwid_warsha_hie(hali: &Idadat) -> MuzawwidWarshaHie {
    let hala = hali.muzawwidun.hala();
    MuzawwidWarshaHie {
        hala: match hala {
            HalatMuzawwidin::Faragh => HalatMuzawwidinHie::Farigh,
            HalatMuzawwidin::Muattala => HalatMuzawwidinHie::Muattala,
            HalatMuzawwidin::Mukhtar => HalatMuzawwidinHie::Mukhtar,
            HalatMuzawwidin::Badeel => HalatMuzawwidinHie::Badeel,
        },
        ism: muzawwid_muntakhab(hali).muarrif,
        wasf_arabi: hala.arabi().to_owned(),
        wasf_injilizi: hala.injilizi().to_owned(),
    }
}

/// The whole project, every row that read, as the workspace opens it.
///
/// A file with rows that did not read is opened as it is: the survivors are shown, the
/// damage is reported beside them, and nothing is written — not even the run journal's
/// pending fold. What to do about the damage is the user's choice, made through
/// [`anqidh_mashru`].
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
    let hali = idadat.hali();
    iftah_warsha(&masarat_hala, &hali, muarrif, id)
}

fn inqadh_hie(taqreer: &TaqreerInqadh) -> InqadhHie {
    let talifa = taqreer.talifa.len();
    let mahfudh = taqreer.mahfudh.display().to_string();
    let marfud = taqreer.marfud.display().to_string();
    let malaf_taqreer = taqreer.taqreer.display().to_string();
    let (tabi_arabi, tabi_injilizi) = if taqreer.najin == 0 {
        (
            "وتتابع الورشة بجدول فارغ".to_owned(),
            "the workshop continues with an empty table".to_owned(),
        )
    } else {
        (
            format!("وتتابع الورشة على {}", sutur_arabi(taqreer.najin, true)),
            format!(
                "the workshop continues on {}",
                sutur_injilizi(taqreer.najin)
            ),
        )
    };
    InqadhHie {
        najin: adad_u32(taqreer.najin),
        talifa: adad_u32(talifa),
        wasf_arabi: format!(
            "جرى إبقاء {} جانبًا، {tabi_arabi}. الأصل محفوظ كما هو بايتًا ببايت في {mahfudh}، \
             والأسطر التالفة وحدها في {marfud}، وتقرير بأرقامها وأسبابها وهويّاتها في \
             {malaf_taqreer}. لا يحذف تعريب هذه الملفات؛ احتفظ بها إلى أن تطمئن.",
            sutur_arabi(talifa, true)
        ),
        wasf_injilizi: format!(
            "{} set aside; {tabi_injilizi}. The original is preserved byte for byte at \
             {mahfudh}, the damaged rows alone at {marfud}, and a report of their line numbers, \
             reasons and identities at {malaf_taqreer}. Taarib never deletes these files; keep \
             them until you are sure.",
            sutur_injilizi(talifa)
        ),
        mahfudh,
        marfud,
        taqreer: malaf_taqreer,
    }
}

/// Sets a damaged string file's unreadable rows aside; the caller holds the project lock.
fn anqidh_dakhili(masarat_hala: &Masarat, id: LubaId, waqt: &str) -> Natija<InqadhHie> {
    let mut mashru = iftah_mashru(masarat_hala, id)?;
    let taqreer = salama::anqidh(&mut mashru, waqt).map_err(Khata::from)?;
    Ok(inqadh_hie(&taqreer))
}

/// Sets the unreadable rows of a damaged string file aside and lets the workspace continue
/// on the rows that read.
///
/// Nothing is replaced before the original is safe: the damaged file is copied byte for byte
/// under a stamped name beside the project and read back to verify, the unreadable lines and
/// a report naming each are written, and only then is the live table rewritten from what
/// read. None of those files is ever deleted by Taarib.
///
/// # Errors
///
/// [`KhataWarshaAmr::MashruGhayrMawjud`] when no project exists; the rescue's own refusals
/// when every row reads (nothing is written) or the preserved copy does not verify (the live
/// table is untouched); and whatever writing beside the project raises.
#[tauri::command]
#[specta::specta]
pub fn anqidh_mashru(
    muarrif: String,
    masarat_hala: tauri::State<'_, Masarat>,
    aqfal: tauri::State<'_, AqfalMashariya>,
) -> Result<InqadhHie, Khata> {
    let id = huwiya(muarrif)?;
    let qufl = qufl_mashru(&aqfal, id);
    let _harasa = qufl.blocking_lock();
    anqidh_dakhili(&masarat_hala, id, &waqt_alaan())
}

/// The glossary in force: terms harvested from the table, the project's own
/// file, and the built-in game-interface terminology underneath both.
///
/// The built-in list is merged last and still loses every collision — its
/// scope ranks below both of the others — so it only ever fills a gap. That
/// is why the harvested-then-file order above it is untouched: the project's
/// file must keep beating a name the extractor guessed at.
pub(crate) fn masrad_kamil(jidhr: &Path, sufuf: &[MudkhalNass]) -> Masrad {
    let mut masrad = Masrad::min_nusus(sufuf);
    let masar = jidhr.join(UDW_MASRAD);
    if masar.is_file()
        && let Ok(mustalahat) = Masrad::min_malaf(&masar)
    {
        masrad.admij(mustalahat);
    }
    masrad.admij(Masrad::min_mudmaj().mustalahat().cloned().collect());
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
    let mut qiyas = silsila.map(|khutut| MudkhalatQiyas {
        saff: &mut saff,
        khutut,
    });
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
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();

    let jadeed = if hadaf.trim().is_empty() {
        None
    } else {
        Some(hadaf)
    };
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
        (
            sabiq,
            mudkhal.tasnif,
            mudkhal.masdar.clone(),
            mudkhal.nasq_masdar.clone(),
        )
    };

    let masar_tarikh = mashru.jidhr().join(UDW_TARIKH);
    let mut tarikh: TarikhMashru = iqra_janibi(&masar_tarikh)?;
    if tarikh.sajjil_tabdeel(
        nass_id,
        sabiq,
        jadeed.clone(),
        muharrir.clone(),
        waqt.clone(),
        None,
    ) {
        uktub_janibi(&masar_tarikh, &tarikh)?;
    }

    if let Some(hadaf_jadeed) = &jadeed
        && let Some(asl) = AslQayd::min_halat(HalatMuraja::Musawwada, Some(muharrir), None, None)
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
    let sufuf = qiraat_nusus(&mashru)?.sufuf;
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
            mudmaj: mustalah.nitaq == NitaqMustalah::Mudmaj,
        })
        .collect();

    let dhakira = Dhakira::iftah(&masarat_hala)?;
    let hasad =
        dhakira.ibhath(&TalabDhakira::jadeed(&mudkhal.masdar, mudkhal.tasnif).bi_luba(&muarrif))?;

    Ok(IqtirahatHie {
        mustalahat,
        tatbiq: hasad
            .tatbiq
            .as_ref()
            .map(|tatbiq| iqtirah_hie(&tatbiq.qayd, 1000)),
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
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
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
        return Err(Khata::from(KhataWarshaAmr::IqtirahGhayrMawjud {
            qayd: qayd.raqm(),
        }));
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

/// The provider a new translation uses: the one the settings elect, or the built-in free
/// one when they elect nothing.
///
/// The election is not re-implemented here: `IdadatMuzawwidin::muntakhab_aw_majjani`
/// decides, and `hala` says whether the choice is the user's own, a fallthrough from a stale
/// default, or the free provider standing in for an empty or switched-off list. None of the
/// three is silent — [`MuzawwidWarshaHie`] puts the state's own sentence on the screen
/// before a run is started — and none of them refuses: the product used to answer "no
/// provider" here and stop, which left a fresh installation unable to translate one string.
pub(crate) fn muzawwid_muntakhab(hali: &Idadat) -> IdadatMuzawwid {
    hali.muzawwidun.muntakhab_aw_majjani()
}

/// Builds the elected provider: credential from the keychain, ceiling as its confirmation.
///
/// The built-in free provider is the one arm that opens no keychain and takes no ceiling —
/// there is no credential to fetch and nothing to cap — so the verdict screen may build it
/// on mount where it may not build any other.
pub(crate) fn bin_muzawwid(
    tarif: &IdadatMuzawwid,
    saqf: u64,
    lahza: u64,
) -> Natija<Box<dyn Muzawwid>> {
    let idhn = IdhnInfaq::baad_taakid(saqf, lahza);
    let hisab = tarif.hisab_miftah.as_deref().unwrap_or(&tarif.muarrif);
    let itimad = || -> Natija<Itimad> {
        Itimad::min_khazina(hisab)
            .map_err(Khata::from)?
            .ok_or_else(|| {
                Khata::from(KhataWarshaAmr::LaItimad {
                    muzawwid: tarif.muarrif.clone(),
                })
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
            Ok(Box::new(
                MuzawwidAnthropic::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?,
            ))
        },
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
        },
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
            Ok(Box::new(
                MuzawwidMuwafiqOpenAI::mahalli(tarkib).map_err(Khata::from)?,
            ))
        },
        NawMuzawwid::Gemini => {
            let mut tarkib = IdadatGemini::default();
            if !tarif.namudhaj.is_empty() {
                tarkib.taklifa = taklifat_gemini(&tarif.namudhaj);
                tarkib.namudhaj.clone_from(&tarif.namudhaj);
            }
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(
                MuzawwidGemini::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?,
            ))
        },
        NawMuzawwid::Deepl => {
            let mut tarkib = IdadatDeepL::default();
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(
                MuzawwidDeepL::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?,
            ))
        },
        NawMuzawwid::MicrosoftTarjama => {
            let mut tarkib = IdadatMicrosoft::default();
            if let Some(nz) = hadd {
                tarkib.hadd_talabat = nz;
            }
            Ok(Box::new(
                MuzawwidMicrosoft::jadeed(tarkib, itimad()?, idhn).map_err(Khata::from)?,
            ))
        },
        NawMuzawwid::GoogleTarjama => Err(Khata::from(KhataWarshaAmr::MuzawwidGhayrMadum {
            muzawwid: tarif.muarrif.clone(),
            sabab_arabi: "مزوّد ترجمة Google السحابي يتطلّب معرّف مشروع سحابي لا تحمله \
                          الإعدادات بعد."
                .to_owned(),
            sabab_injilizi: "the Google Cloud translation provider needs a Cloud project id \
                             settings do not yet carry"
                .to_owned(),
        })),
        // The request spacing and the in-flight bound are the endpoint's tolerances,
        // not the user's to tune, so the row's own request limit is not read: the
        // provider's defaults are the configuration.
        NawMuzawwid::GoogleMajjani => Ok(Box::new(
            MuzawwidGoogleMajjani::jadeed(IdadatGoogleMajjani::default()).map_err(Khata::from)?,
        )),
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
    let muraja: BTreeMap<NassId, SijillMuraja> = sufuf
        .iter()
        .map(|mudkhal| (mudkhal.id, mudkhal.muraja.clone()))
        .collect();
    let alamat_masrad: BTreeMap<NassId, Vec<AlamJawda>> = sufuf
        .iter()
        .map(|mudkhal| (mudkhal.id, masrad.afhas(mudkhal)))
        .collect();
    let mut saff = Saff::jadeed();
    let mut qiyas = silsila.as_ref().map(|khutut| MudkhalatQiyas {
        saff: &mut saff,
        khutut,
    });
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
/// [`KhataWarshaAmr::LaItimad`] when the elected provider's key is not in the keychain,
/// [`KhataWarshaAmr::MuzawwidGhayrMadum`] when it cannot be built as configured, and
/// whatever the project store or the journal raise. A provider-side failure is answered,
/// not raised: it comes back as `najahat: false` with its recorded reason.
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
    let tarif = muzawwid_muntakhab(&hali);
    let lahza = lahza_alaan();
    let saqf_nano = tarif
        .mizaniya
        .and_then(|mablagh| nano_min_dolar(mablagh).ok())
        .unwrap_or(u64::MAX);
    let muzawwid = bin_muzawwid(&tarif, saqf_nano, lahza)?;

    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
    let mufrad: Vec<MudkhalNass> = sufuf
        .iter()
        .filter(|mudkhal| mudkhal.id == nass_id)
        .cloned()
        .collect();
    if mufrad.is_empty() {
        return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
    }

    let waqt = waqt_alaan();
    let khiyarat = KhiyaratJawla {
        saqf_takalif: None,
        lahza,
        waqt: waqt.clone(),
        // Somebody is looking at this one row and has pressed translate on it.
        // The bulk run skips what the classifier called internal, because a
        // provider asked about a key answers with prose; here the press is the
        // override, and refusing it while the screen blamed the provider would
        // be a lie about what happened.
        yashmal_dakhili: true,
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
        (
            false,
            Some("عمل إنسان أحدث في الطريق؛ لم تُكتب الترجمة الآلية فوقه.".to_owned()),
        )
    } else if takhatti.mutarjama_musbaqan > 0 {
        (
            false,
            Some("النص مترجم بالفعل؛ الترجمة الآلية لا تكتب فوق ترجمة قائمة.".to_owned()),
        )
    } else if takhatti.mujammada > 0 {
        (
            false,
            Some("النص مجمّد؛ العمليات الجماعية لا تلمسه.".to_owned()),
        )
    } else if takhatti.farigha > 0 {
        (false, Some("النص الأصلي فارغ فلا شيء يُترجم.".to_owned()))
    } else if takhatti.dakhiliya > 0 {
        (
            false,
            Some("صُنّف هذا النص داخليًّا — مفتاحٌ أو رمز لا جملة — فلم يُرسَل.".to_owned()),
        )
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
    Ok(NatijatTarjamaHie {
        saf,
        najahat,
        sabab_arabi,
    })
}

/// A batch run to a cost ceiling, committed as it lands; progress streams on its event.
///
/// # Errors
///
/// [`KhataWarshaAmr::SaqfGhayrSalih`], [`KhataWarshaAmr::LaItimad`],
/// [`KhataWarshaAmr::MuzawwidGhayrMadum`], and whatever the project store or the journal
/// raise. A stopped run is not an error: what stopped it is in the returned accounting.
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
    let tarif = muzawwid_muntakhab(&hali);
    let lahza = lahza_alaan();
    let muzawwid = bin_muzawwid(&tarif, saqf_nano, lahza)?;

    let mut mashru = iftah_mashru(&masarat_hala, id)?;
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
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
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
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
    Ok(AlamatMashruHie {
        adad_alamat,
        adad_khatira,
        tadarubat,
    })
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
    let mut sufuf = sufuf_lil_kitaba(&mashru)?;
    let waqt = waqt_alaan();
    let lahza = lahza_alaan();
    let jidhr = mashru.jidhr().to_path_buf();

    let masrad = masrad_kamil(&jidhr, &sufuf);
    let tadarubat = masrad.tadarubat(&sufuf);
    let Some(tadarub) = tadarubat
        .iter()
        .find(|tadarub| tadarub.mustalah == mustalah)
    else {
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
    let sufuf = qiraat_nusus(&mashru)?.sufuf;
    let Some(mudkhal) = sufuf.iter().find(|mudkhal| mudkhal.id == nass_id) else {
        return Err(Khata::from(KhataWarshaAmr::NassGhayrMawjud { nass }));
    };
    let mutah_maruf = mudkhal.quyud.aqsa_ard.map(f64::from);

    let hali = idadat.hali();
    let Some(silsila) = silsilat_khutut(&masarat_hala, &hali) else {
        return Ok(MuayanaHie {
            ghayr_qabil: true,
            sabab_ghayr_qabil: Some("لا خط عربي صالح في مجلد الخطوط؛ لا قياس بلا خط.".to_owned()),
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
        },
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
                .map_or((0.0_f32, 0.0_f32), |taqreer| {
                    (taqreer.zaid(), taqreer.nisba())
                });
            Ok(MuayanaHie {
                ghayr_qabil: false,
                sabab_ghayr_qabil: None,
                hajm: Some(f64::from(takhtit.hajm)),
                mutah: Some(f64::from(mutah)),
                sutur,
                tajawuz_biksil: Some(f64::from(zaid)),
                tajawuz_nisba: Some(f64::from(nisba)),
            })
        },
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
    // The held merge ends in a rewrite of the live table, so it is refused on a file that
    // did not read whole, exactly as an edit is.
    let sufuf = sufuf_lil_kitaba(&mashru)?;

    let muhtawa = istawrid(Path::new(&masar)).map_err(Khata::from)?;
    // A bundle with rows that did not read would merge as if the colleague never had
    // them; their work on those rows would vanish from the result with no message.
    if muhtawa.talifa > 0 {
        return Err(Khata::from(KhataWarshaAmr::HuzmaBihaTalaf {
            talifa: muhtawa.talifa,
        }));
    }
    muhtawa
        .tahaqquq_tatabuq(mashru.rasm())
        .map_err(Khata::from)?;
    let aslaf = aslaf_mashru(mashru.jidhr()).map_err(Khata::from)?;
    let aslaf_talifa = aslaf.as_ref().map_or(0, |(_, talifa)| *talifa);
    // A partly unreadable ancestor is kept as it is: the rows it lost merge without an
    // ancestor, which is more conflicts and never a silent choice. Said out loud below.
    let (tanbih_arabi, tanbih_injilizi) = if aslaf_talifa > 0 {
        (
            Some(format!(
                "لقطة آخر تصدير — الأصل المشترك للدمج — فيها ما لا يُقرأ: {} من أسطرها. دُمجت \
                 تلك الأسطر بلا أصل مشترك، فتوقّع تعارضات أكثر بينها واحسم كلًّا منها بيدك.",
                sutur_arabi(aslaf_talifa, false)
            )),
            Some(format!(
                "The last-export snapshot, the merge's shared ancestor, has {} that did not \
                 read. Those strings were merged without an ancestor: expect more conflicts \
                 among them, and decide each by hand.",
                sutur_injilizi(aslaf_talifa)
            )),
        )
    } else {
        (None, None)
    };

    let natijat_damj = damj(
        sufuf,
        muhtawa.nusus,
        aslaf.as_ref().map(|(nusus, _)| nusus.as_slice()),
    );
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
        aslaf_talifa: adad_u32(aslaf_talifa),
        tanbih_arabi,
        tanbih_injilizi,
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
        let mut kharita = jalasat.0.lock();
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

    let (mut sufuf, husum) = mahfudh
        .damj
        .itmam(&talabat, &muharrir, lahza)
        .map_err(Khata::from)?;
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

    Ok(DamjHie {
        sufuf: adad_u32(sufuf.len()),
        husum: adad_u32(husum.len()),
    })
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

    /// The string file has rows that did not read, so nothing is written over it.
    #[error("{talifa} row(s) of the string file did not read, so it was not written to")]
    MashruTalif {
        /// How many lines did not read.
        talifa: usize,
        /// The file.
        masar: PathBuf,
    },

    /// A colleague's bundle carries rows that did not read, so it was not merged.
    #[error("{talifa} row(s) of the bundle did not read, so it was not merged")]
    HuzmaBihaTalaf {
        /// How many lines did not read.
        talifa: usize,
    },

    /// The automatic run this game would be recovered from left a table past
    /// the byte cap, so nothing was read out of it.
    #[error(
        "the run's string table at {} is {hajm} bytes, over the {hadd} this build reads",
        masar.display()
    )]
    JadwalMashwarKabir {
        /// The run's table.
        masar: PathBuf,
        /// What the directory entry said.
        hajm: u64,
        /// The cap it passed.
        hadd: u64,
    },
}

impl Tafsir for KhataWarshaAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MashruGhayrMawjud { .. } => 40,
                    Self::NassGhayrMawjud { .. } => 41,
                    // 42 was "no provider is elected", retired when the built-in
                    // free provider made every list elect something; the number
                    // stays unused so an old diagnostics bundle still reads.
                    Self::LaItimad { .. } => 43,
                    Self::MuzawwidGhayrMadum { .. } => 44,
                    Self::IqtirahGhayrMawjud { .. } => 45,
                    Self::TadarubGhayrMawjud { .. } => 46,
                    Self::LaDamjMaftuh => 47,
                    Self::SaqfGhayrSalih => 48,
                    Self::MalafTalif { .. } => 49,
                    Self::MashruTalif { .. } => 50,
                    Self::HuzmaBihaTalaf { .. } => 51,
                    Self::JadwalMashwarKabir { .. } => 52,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Not a failure. A game nobody has started translating yet is the
            // workshop's empty state, and the screen has to be able to tell it
            // from a warning by the severity rather than by matching the code.
            Self::MashruGhayrMawjud { .. } => Khutura::Maluma,
            // Asked about something that is not there; nothing was touched.
            Self::NassGhayrMawjud { .. }
            | Self::IqtirahGhayrMawjud { .. }
            | Self::TadarubGhayrMawjud { .. }
            | Self::LaDamjMaftuh
            | Self::SaqfGhayrSalih
            | Self::HuzmaBihaTalaf { .. } => Khutura::Tanbeeh,
            // A run was requested and cannot start, and the user can fix the
            // settings; or a file beside somebody's work, or the work itself,
            // does not read.
            Self::LaItimad { .. }
            | Self::MuzawwidGhayrMadum { .. }
            | Self::MalafTalif { .. }
            | Self::MashruTalif { .. }
            | Self::JadwalMashwarKabir { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MashruGhayrMawjud { .. } => {
                "لا مشروع ترجمة لهذه اللعبة بعد. ابدأ الترجمة من شاشة اللعبة أولًا.".to_owned()
            },
            Self::NassGhayrMawjud { .. } => {
                "هذا النص لم يعد في جدول المشروع. أعد فتح الورشة لتحميل الجدول الحالي.".to_owned()
            },
            Self::LaItimad { muzawwid } => format!(
                "لا اعتماد في سلسلة مفاتيح النظام للمزوّد {muzawwid}. أدخل مفتاحه في \
                 الإعدادات ليُخزَّن في السلسلة."
            ),
            Self::MuzawwidGhayrMadum { sabab_arabi, .. } => sabab_arabi.clone(),
            Self::IqtirahGhayrMawjud { .. } => {
                "لم تعد الذاكرة تعرض هذا القيد لهذا النص. حدّث الاقتراحات ثم اختر من \
                 جديد."
                    .to_owned()
            },
            Self::TadarubGhayrMawjud { mustalah } => format!(
                "لا تضارب مصطلحات على «{mustalah}» الآن؛ ربما حُسم في تحرير سابق. أعد \
                 حساب العلامات."
            ),
            Self::LaDamjMaftuh => {
                "لا دمج معلّقًا لهذا المشروع. استورد حزمة زميل أولًا ثم احسم تعارضاتها.".to_owned()
            },
            Self::SaqfGhayrSalih => {
                "سقف التكلفة يجب أن يكون مبلغًا موجبًا محدودًا بالدولار.".to_owned()
            },
            Self::MalafTalif { masar, .. } => format!(
                "الملف {} موجود ولا يُقرأ. لن يُكتب فوقه؛ افحصه أو انقله ثم أعد المحاولة.",
                masar.display()
            ),
            Self::MashruTalif { talifa, .. } => format!(
                "تعذّرت قراءة {} من ملف نصوص هذا المشروع، فلن يُكتب فوقه شيء. افتح الورشة: \
                 فيها بيان بالأسطر التالفة وخيار إبقائها جانبًا مع حفظ الأصل.",
                sutur_arabi(*talifa, true)
            ),
            Self::HuzmaBihaTalaf { talifa } => format!(
                "في حزمة الزميل ما لا يُقرأ — {} من أسطرها — فلم تُدمج: دمجٌ يُسقط أسطرًا بصمت \
                 ليس دمجًا. اطلب من الزميل فتح ورشته، فستدلّه على الأسطر التالفة، ثم إعادة \
                 التصدير.",
                sutur_arabi(*talifa, false)
            ),
            Self::JadwalMashwarKabir { masar, .. } => format!(
                "جدول نصوص آخر جولة تلقائية لهذه اللعبة أكبر ممّا يقرأه هذا الإصدار، فلم \
                 يُقرأ منه شيء ولم يُستعد المشروع. انقل الملف {} جانبًا أو احذف مجلّد تلك \
                 الجولة، ثمّ أعد الترجمة لتُبنى جولة جديدة.",
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
            },
            Self::SaqfGhayrSalih => {
                "The cost ceiling must be a positive, finite dollar amount.".to_owned()
            },
            Self::MalafTalif { masar, sabab } => format!(
                "{} exists and does not read ({sabab}). Nothing will be written over it; \
                 inspect or move it, then retry.",
                masar.display()
            ),
            Self::MashruTalif { talifa, .. } => format!(
                "{} of this project's string file did not read, so nothing will be written \
                 over it. Open the workshop: it lists the damaged rows and offers to set them \
                 aside while preserving the original.",
                sutur_injilizi(*talifa)
            ),
            Self::HuzmaBihaTalaf { talifa } => format!(
                "The colleague's bundle carries {} that did not read, so it was not merged: a \
                 merge that drops rows in silence is not a merge. Ask them to open their \
                 workshop, which will point at the damaged rows, and export again.",
                sutur_injilizi(*talifa)
            ),
            Self::JadwalMashwarKabir { masar, hajm, hadd } => format!(
                "The last automatic run for this game left a string table of {hajm} bytes, over \
                 the {hadd} this build reads, so nothing was read out of it and no project was \
                 recovered. Move {} aside or delete that run's directory, then translate again \
                 so a fresh run is built.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MashruGhayrMawjud { .. } | Self::MashruTalif { .. } => Khutwa::FathNusus,
            // Neither opens on this machine: one is the colleague's to fix, the
            // other is a file the sentence already names and points at.
            Self::HuzmaBihaTalaf { .. } | Self::JadwalMashwarKabir { .. } => Khutwa::LaShay,
            Self::NassGhayrMawjud { .. }
            | Self::IqtirahGhayrMawjud { .. }
            | Self::TadarubGhayrMawjud { .. }
            | Self::LaDamjMaftuh
            | Self::SaqfGhayrSalih
            | Self::MalafTalif { .. } => Khutwa::AadaMuhawala,
            Self::LaItimad { .. } | Self::MuzawwidGhayrMadum { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Muzawwidun,
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MashruGhayrMawjud { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::NassGhayrMawjud { nass } => {
                let _ = siyaq.insert("nass".to_owned(), QeemaSiyaq::Nass(nass.clone()));
            },
            Self::LaItimad { muzawwid } | Self::MuzawwidGhayrMadum { muzawwid, .. } => {
                let _ = siyaq.insert("muzawwid".to_owned(), QeemaSiyaq::Nass(muzawwid.clone()));
            },
            Self::IqtirahGhayrMawjud { qayd } => {
                let _ = siyaq.insert("qayd".to_owned(), QeemaSiyaq::Raqm(*qayd));
            },
            Self::TadarubGhayrMawjud { mustalah } => {
                let _ = siyaq.insert("mustalah".to_owned(), QeemaSiyaq::Nass(mustalah.clone()));
            },
            Self::MalafTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MashruTalif { talifa, masar } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert(
                    "talifa".to_owned(),
                    QeemaSiyaq::Hajm(u64::try_from(*talifa).unwrap_or(u64::MAX)),
                );
            },
            Self::HuzmaBihaTalaf { talifa } => {
                let _ = siyaq.insert(
                    "talifa".to_owned(),
                    QeemaSiyaq::Hajm(u64::try_from(*talifa).unwrap_or(u64::MAX)),
                );
            },
            Self::JadwalMashwarKabir { masar, hajm, hadd } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("hajm".to_owned(), QeemaSiyaq::Hajm(*hajm));
                let _ = siyaq.insert("hadd".to_owned(), QeemaSiyaq::Hajm(*hadd));
            },
            Self::LaDamjMaftuh | Self::SaqfGhayrSalih => {},
        }
        siyaq
    }
}

khata_min!(KhataWarshaAmr);

#[cfg(test)]
mod ikhtibarat {
    use std::io::Write as _;

    use taarib_istikhraj::mashru::BayanIstikhraj;
    use taarib_mustalahat::luba::MasdarLuba;
    use taarib_mustalahat::nass::{MasdarIstikhraj, QuyudNass, SiyaqNass};
    use taarib_tarjama::dufaat::{HasilatNass, QaydJawla};
    use taarib_usus::idadat::{IdadatMuzawwidin, MUARRIF_GOOGLE_MAJJANI};

    use super::*;

    /// Anything a test can fail on: a `Khata` from the code under test, a store
    /// refusal, or an [`std::io::Error`] from the scratch directory it staged.
    type NatijatIkhtibar = Result<(), Box<dyn std::error::Error>>;

    /// The refusal's code, so the assertions name the failure and not the enum.
    const RAMZ_MASHRU_TALIF: u16 = arqam::STUDIO + 50;

    /// The rescue's refusal when nothing is damaged, from the workspace crate.
    const RAMZ_LA_TALAF: u16 = arqam::WARSHA + 12;

    /// The moment every write in these tests carries.
    const WAQT: &str = "2026-09-06T10:00:00Z";

    /// A torn head: identity and source text intact, everything after cut off.
    const RAAS_MABTUR: &str =
        r#"{"id":"1b4e28ba-2fa1-5d68-9d3a-3a0f0b1c2d3e","masdar":"Press any key","hadaf":nu"#;

    /// Bytes that are not UTF-8 at all, then a partial object.
    const BAYT_TALIFA: &[u8] = b"\xff\xfe\x00{\"id\":\"";

    /// A scratch directory that removes itself, so a failed assertion does not
    /// leave one behind in the machine's temporary directory.
    struct JidhrMuaqqat(PathBuf);

    impl Drop for JidhrMuaqqat {
        fn drop(&mut self) {
            // Best effort: a test that already failed must not fail twice.
            #[expect(
                clippy::disallowed_methods,
                reason = "a scratch directory under `std::env::temp_dir()` removing itself, never \
                          a data root or a game directory"
            )]
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn jidhr_muaqqat() -> JidhrMuaqqat {
        JidhrMuaqqat(std::env::temp_dir().join(format!("taarib-warsha-{}", uuid::Uuid::new_v4())))
    }

    fn masarat_muaqqata(haris: &JidhrMuaqqat) -> Masarat {
        Masarat::min_judhur(haris.0.join("bayanat"), haris.0.join("idadat"))
    }

    fn luba_ikhtibar() -> LubaId {
        LubaId::min_masdar(&MasdarLuba::Steam(480), "Spacewar")
    }

    fn saf(mawqi: &str, masdar: &str) -> MudkhalNass {
        MudkhalNass {
            id: NassId::min_mawqi("data/menu.json", mawqi, masdar),
            masdar: masdar.to_owned(),
            hadaf: None,
            muraja: SijillMuraja::jadeed(),
            siyaq: SiyaqNass {
                hawiya: "data/menu.json".to_owned(),
                mawqi: mawqi.to_owned(),
                ..SiyaqNass::default()
            },
            quyud: QuyudNass::default(),
            nasq_masdar: Vec::new(),
            nasq_hadaf: Vec::new(),
            takrar: 1,
            majmua: None,
            alamat: Vec::new(),
            tareeqa: None,
            muzawwid: None,
            muharrir: None,
            akhir_tabdeel: None,
            tasnif: TasnifNass::Ikhtiyar,
            thiqat_tasnif: 80,
            masdar_istikhraj: MasdarIstikhraj::Sakin,
            tarmiz: None,
        }
    }

    /// A finished automatic run for the test game, with `adad` rows in its own
    /// table and nothing in the workshop's store.
    ///
    /// This is the shape every run this product shipped left behind before the
    /// run learned to publish, so it is the shape recovery has to read.
    fn mashwar_bi_sufuf(
        masarat_hala: &Masarat,
        adad: usize,
    ) -> Result<LubaId, Box<dyn std::error::Error>> {
        let id = luba_ikhtibar();
        let mujallad = crate::tilqai_awamir::jidhr_mashawir(masarat_hala, id)
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(mujallad.join(MUJALLAD_MASHRU))?;

        // The journal's header alone: `ijrud` needs it to see the directory as
        // a run at all, and recovery reads the game's name and moment from it.
        let mut sijill =
            std::fs::File::create(mujallad.join(taarib_tilqai::mashwar::MALAF_SIJILL))?;
        writeln!(
            sijill,
            "{}",
            serde_json::json!({
                "naw": "tarwisa",
                "isdar": taarib_tilqai::mashwar::ISDAR_SIJILL,
                "id": uuid::Uuid::new_v4().to_string(),
                "ism_luba": "Spacewar",
                "jidhr_luba": "/luba",
                "waqt": WAQT,
            })
        )?;

        let sufuf: Vec<MudkhalNass> = (0..adad)
            .map(|raqm| {
                let mut mudkhal = saf(&format!("menu/{raqm}"), &format!("Option {raqm}"));
                mudkhal.hadaf = Some(format!("خيار {raqm}"));
                mudkhal
            })
            .collect();
        taarib_tilqai::tarjama::uktub_nusus(
            &mujallad.join(MUJALLAD_MASHRU).join(MALAF_NUSUS_MASHWAR),
            &sufuf,
        )?;
        Ok(id)
    }

    /// The defect a user met on every game: a finished translation, and a
    /// workshop saying no project exists and to start one.
    #[test]
    fn alfath_yastaid_almashru_min_mashwar_muntah() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat = masarat_muaqqata(&haris);
        let id = mashwar_bi_sufuf(&masarat, 3)?;
        assert!(
            !jidhr_mashru(&masarat, id).join(MALAF_MASHRU).is_file(),
            "the run left nothing in the workshop's store, which is the case under test"
        );

        let mashru = iftah_mashru(&masarat, id)?;
        let (sufuf, talifa) = mashru.iqra_nusus()?;
        assert_eq!(talifa, 0);
        assert_eq!(sufuf.len(), 3, "every row the run had reached the workshop");
        let awwal = sufuf.first().ok_or("the first recovered row")?;
        assert_eq!(
            awwal.hadaf.as_deref(),
            Some("خيار 0"),
            "the run's translations travelled with its rows"
        );
        assert_eq!(mashru.rasm().ism_luba, "Spacewar");
        assert!(
            jidhr_mashru(&masarat, id).join(MALAF_MASHRU).is_file(),
            "the recovered project is on disk, so the next open is an ordinary open"
        );
        Ok(())
    }

    /// Recovery must not reach for a run that is not there.
    #[test]
    fn alfath_yarfud_luban_bila_mashwar() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat = masarat_muaqqata(&haris);
        let khata = iftah_mashru(&masarat, luba_ikhtibar())
            .err()
            .ok_or("a game with neither a project nor a run is refused")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 40);
        Ok(())
    }

    /// A run whose table is empty is "no project yet", not an empty project.
    #[test]
    fn alfath_yarfud_mashwaran_bila_sufuf() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat = masarat_muaqqata(&haris);
        let id = mashwar_bi_sufuf(&masarat, 0)?;
        let khata = iftah_mashru(&masarat, id)
            .err()
            .ok_or("a run that extracted nothing leaves no project to open")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 40);
        Ok(())
    }

    /// A run table past the cap is named, not hidden behind "no project yet":
    /// the recovery meets that same file on every open, so a user told to start
    /// a translation would press the button for ever.
    #[test]
    fn alfath_yusammi_jadwal_mashwar_kabiran() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat = masarat_muaqqata(&haris);
        let id = mashwar_bi_sufuf(&masarat, 3)?;
        let jidhr_mashawir = crate::tilqai_awamir::jidhr_mashawir(&masarat, id);
        let mawjuz = ijrud(&jidhr_mashawir)
            .into_iter()
            .next()
            .ok_or("the run this test wrote")?;
        let masar = mawjuz
            .mujallad
            .join(MUJALLAD_MASHRU)
            .join(MALAF_NUSUS_MASHWAR);
        // Extended rather than filled: the cap is judged from the directory
        // entry before a byte is taken, so the test costs a sparse file.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&masar)?
            .set_len(taarib_tilqai::tarjama::HADD_HAJM_NUSUS + 1)?;

        let khata = iftah_mashru(&masarat, id)
            .err()
            .ok_or("a run table past the cap is refused")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 52);
        assert_ne!(
            khata.ramz.raqm(),
            arqam::STUDIO + 40,
            "not the empty state: there is a run, and its table is the problem"
        );
        assert!(
            !jidhr_mashru(&masarat, id).join(MALAF_MASHRU).is_file(),
            "nothing was published from a table that was never read"
        );
        Ok(())
    }

    /// Recovery never runs over a project that already exists, whatever a run
    /// beside it holds — that is where a person's edits live.
    #[test]
    fn alistiada_la_tamuss_mashruan_qaiman() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat = masarat_muaqqata(&haris);
        let (id, _) = mashru_bi_sufuf(&masarat, 2)?;
        let _ = mashwar_bi_sufuf(&masarat, 9)?;

        let mashru = iftah_mashru(&masarat, id)?;
        let (sufuf, _) = mashru.iqra_nusus()?;
        assert_eq!(
            sufuf.len(),
            2,
            "the existing project was opened, not replaced by the run's nine rows"
        );
        Ok(())
    }

    /// A project of `adad` rows, written where the workspace looks for it.
    fn mashru_bi_sufuf(
        masarat_hala: &Masarat,
        adad: usize,
    ) -> Result<(LubaId, PathBuf), Box<dyn std::error::Error>> {
        let id = luba_ikhtibar();
        let jidhr = jidhr_mashru(masarat_hala, id);
        let mut mashru = MashruMaftuh::ansha(
            jidhr.clone(),
            id,
            "Spacewar".to_owned(),
            BayanIstikhraj::default(),
            WAQT.to_owned(),
        )?;
        let sufuf: Vec<MudkhalNass> = (0..adad)
            .map(|raqm| saf(&format!("menu/{raqm}"), &format!("Option {raqm}")))
            .collect();
        mashru.adif(sufuf)?;
        mashru.ikhtim(WAQT.to_owned())?;
        Ok((id, jidhr))
    }

    /// Appends two damaged lines to the string file, raw.
    fn atlif(jidhr: &Path) -> std::io::Result<()> {
        let mut malaf = std::fs::OpenOptions::new()
            .append(true)
            .open(jidhr.join(MALAF_NUSUS))?;
        malaf.write_all(RAAS_MABTUR.as_bytes())?;
        malaf.write_all(b"\n")?;
        malaf.write_all(BAYT_TALIFA)?;
        malaf.write_all(b"\n")
    }

    /// One finished machine translation in the run journal, awaiting its fold.
    fn sajjil_jawla(
        jidhr: &Path,
        id: NassId,
        hadaf: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut sijill = SijillJawla::iftah(jidhr)?;
        let mut muraja = SijillMuraja::jadeed();
        muraja.sajjil_aali(1);
        sijill.sajjil(QaydJawla {
            id,
            muzawwid: "mahalli".to_owned(),
            taklifa: 0,
            lahza: 1,
            hasila: HasilatNass::Tarjumat {
                hadaf: hadaf.to_owned(),
                nasq: Vec::new(),
                thiqa: None,
                alamat: Vec::new(),
                muraja,
            },
        })?;
        Ok(())
    }

    fn asma(jidhr: &Path) -> std::io::Result<Vec<String>> {
        let mut asma = Vec::new();
        for mudkhal in std::fs::read_dir(jidhr)? {
            asma.push(mudkhal?.file_name().to_string_lossy().into_owned());
        }
        asma.sort();
        Ok(asma)
    }

    /// A damaged file refuses every command that would write it back, and is not touched.
    #[test]
    fn alkitaba_turfad_ala_mashru_talif() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 4)?;
        atlif(&jidhr)?;
        let qabl = std::fs::read(jidhr.join(MALAF_NUSUS))?;

        let mashru = iftah_mashru(&masarat_hala, id)?;
        let khata = sufuf_lil_kitaba(&mashru)
            .err()
            .ok_or("a damaged file must refuse a write")?;

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_MASHRU_TALIF));
        assert_eq!(std::fs::read(jidhr.join(MALAF_NUSUS))?, qabl);
        Ok(())
    }

    /// A clean file is handed to a writing command whole.
    #[test]
    fn alkitaba_tumnah_ala_mashru_salim() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, _) = mashru_bi_sufuf(&masarat_hala, 4)?;

        let mashru = iftah_mashru(&masarat_hala, id)?;
        let sufuf = sufuf_lil_kitaba(&mashru)?;

        assert_eq!(sufuf.len(), 4);
        Ok(())
    }

    /// Opening a damaged project reports every damaged line, shows the survivors, and
    /// writes nothing — not even the run journal's pending fold, which is the path that
    /// used to rewrite the file from the survivors on open.
    #[test]
    fn alfath_la_yaktub_fawq_mashru_talif_hatta_maa_sijill() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 4)?;
        sajjil_jawla(&jidhr, saf("menu/1", "Option 1").id, "الخيار الأول")?;
        atlif(&jidhr)?;
        let qabl = std::fs::read(jidhr.join(MALAF_NUSUS))?;
        let asma_qabl = asma(&jidhr)?;

        let warsha = iftah_warsha(&masarat_hala, &Idadat::default(), "spacewar".to_owned(), id)?;

        assert_eq!(warsha.salama.hala, HalatNususHie::Talifa);
        assert_eq!(warsha.adad, 4, "the survivors are shown");
        assert_eq!(warsha.salama.najin, 4);
        let talaf = warsha
            .salama
            .talaf
            .ok_or("the damage must be reported beside the rows")?;
        assert_eq!(talaf.talifa, 2);
        assert_eq!(talaf.sutur.len(), 2);
        assert_eq!(talaf.sutur.first().map(|satr| satr.raqm), Some(5));
        assert_eq!(
            talaf.sutur.first().and_then(|satr| satr.masdar.as_deref()),
            Some("Press any key"),
            "a torn line still names the string it was"
        );
        assert_eq!(talaf.sutur.get(1).map(|satr| satr.raqm), Some(6));
        assert!(!talaf.zir_arabi.is_empty() && !talaf.zir_injilizi.is_empty());
        assert!(warsha.salama.wasf_arabi.contains("لا تُعِد الاستخراج"));
        assert!(warsha.salama.wasf_injilizi.contains("Do not re-extract"));
        assert_eq!(
            std::fs::read(jidhr.join(MALAF_NUSUS))?,
            qabl,
            "nothing was written"
        );
        assert_eq!(asma(&jidhr)?, asma_qabl, "nothing was created");
        Ok(())
    }

    /// On a clean file the pending fold still runs and still persists.
    #[test]
    fn alfath_yatwi_alsijill_ala_mashru_salim() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 4)?;
        let nass = saf("menu/1", "Option 1").id;
        sajjil_jawla(&jidhr, nass, "الخيار الأول")?;
        let qabl = std::fs::read(jidhr.join(MALAF_NUSUS))?;

        let warsha = iftah_warsha(&masarat_hala, &Idadat::default(), "spacewar".to_owned(), id)?;

        assert_eq!(warsha.salama.hala, HalatNususHie::Salima);
        assert!(warsha.salama.talaf.is_none());
        let saf_1 = warsha
            .sufuf
            .iter()
            .find(|saf| saf.nass == nass.to_string())
            .ok_or("the journaled row is in the table")?;
        assert_eq!(saf_1.hadaf.as_deref(), Some("الخيار الأول"));
        let baad = std::fs::read(jidhr.join(MALAF_NUSUS))?;
        assert_ne!(baad, qabl, "the fold is persisted");
        assert!(String::from_utf8_lossy(&baad).contains("الخيار الأول"));
        Ok(())
    }

    /// A project with nothing extracted is empty, and says so as empty — not as damaged.
    #[test]
    fn alfarigh_yuqal_farighan_la_talifan() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 0)?;
        assert!(!jidhr.join(MALAF_NUSUS).exists(), "no batch, no file");

        let warsha = iftah_warsha(&masarat_hala, &Idadat::default(), "spacewar".to_owned(), id)?;

        assert_eq!(warsha.salama.hala, HalatNususHie::Farigh);
        assert!(warsha.salama.talaf.is_none());
        assert_eq!(warsha.adad, 0);
        Ok(())
    }

    /// The rescue keeps the original, sets the damaged lines aside, and the next open
    /// finds a whole file holding exactly the survivors.
    #[test]
    fn alinqadh_yuid_almashru_saliman_wa_yahfaz_alasl() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 4)?;
        atlif(&jidhr)?;
        let asl = std::fs::read(jidhr.join(MALAF_NUSUS))?;

        let inqadh = anqidh_dakhili(&masarat_hala, id, WAQT)?;

        assert_eq!(inqadh.najin, 4);
        assert_eq!(inqadh.talifa, 2);
        assert_eq!(
            std::fs::read(&inqadh.mahfudh)?,
            asl,
            "the original is preserved byte for byte"
        );
        assert!(Path::new(&inqadh.marfud).is_file());
        assert!(Path::new(&inqadh.taqreer).is_file());
        assert!(inqadh.wasf_arabi.contains(&inqadh.mahfudh));
        assert!(inqadh.wasf_injilizi.contains(&inqadh.mahfudh));

        let warsha = iftah_warsha(&masarat_hala, &Idadat::default(), "spacewar".to_owned(), id)?;
        assert_eq!(warsha.salama.hala, HalatNususHie::Salima);
        assert_eq!(warsha.adad, 4);
        Ok(())
    }

    /// The rescue refuses a whole file and creates nothing.
    #[test]
    fn alinqadh_yarfud_mashruan_saliman() -> NatijatIkhtibar {
        let haris = jidhr_muaqqat();
        let masarat_hala = masarat_muaqqata(&haris);
        let (id, jidhr) = mashru_bi_sufuf(&masarat_hala, 2)?;
        let asma_qabl = asma(&jidhr)?;

        let khata = anqidh_dakhili(&masarat_hala, id, WAQT)
            .err()
            .ok_or("a whole file has nothing to set aside")?;

        assert_eq!(khata.ramz, Ramz::jadeed(RAMZ_LA_TALAF));
        assert_eq!(asma(&jidhr)?, asma_qabl);
        Ok(())
    }

    fn tarif(muarrif: &str, mufaal: bool) -> IdadatMuzawwid {
        IdadatMuzawwid {
            muarrif: muarrif.to_owned(),
            naw: NawMuzawwid::Anthropic,
            namudhaj: String::new(),
            asas: None,
            hisab_miftah: None,
            mufaal,
            hadd_talabat: 1,
            mizaniya: None,
        }
    }

    fn idadat_bi_muzawwidin(qaima: Vec<IdadatMuzawwid>, iftiradi: Option<&str>) -> Idadat {
        Idadat {
            muzawwidun: IdadatMuzawwidin {
                qaima,
                iftiradi: iftiradi.map(str::to_owned),
            },
            ..Idadat::default()
        }
    }

    /// The election is the settings crate's: an empty list and a list of switched-off
    /// providers both hand the run the built-in free provider, and the screen is told which
    /// of the two states it is in; a stale default is honoured as a fallthrough and said out
    /// loud. Nothing here refuses — the refusal this used to pin is what left a fresh
    /// installation unable to translate.
    #[test]
    fn alintikhab_yufawwad_lil_idadat_wa_almajjani_yaqif_makan_alghaib() {
        let faragh = idadat_bi_muzawwidin(Vec::new(), None);
        let badeel_faragh = muzawwid_muntakhab(&faragh);
        assert_eq!(badeel_faragh.naw, NawMuzawwid::GoogleMajjani);
        assert_eq!(badeel_faragh.muarrif, MUARRIF_GOOGLE_MAJJANI);
        let hie_faragh = muzawwid_warsha_hie(&faragh);
        assert_eq!(hie_faragh.hala, HalatMuzawwidinHie::Farigh);
        assert_eq!(hie_faragh.ism, MUARRIF_GOOGLE_MAJJANI);
        assert_eq!(hie_faragh.wasf_arabi, HalatMuzawwidin::Faragh.arabi());
        assert_eq!(hie_faragh.wasf_injilizi, HalatMuzawwidin::Faragh.injilizi());

        let muattala = idadat_bi_muzawwidin(vec![tarif("a", false)], Some("a"));
        assert_eq!(
            muzawwid_muntakhab(&muattala).naw,
            NawMuzawwid::GoogleMajjani
        );
        let hie_muattala = muzawwid_warsha_hie(&muattala);
        assert_eq!(hie_muattala.hala, HalatMuzawwidinHie::Muattala);
        assert_eq!(hie_muattala.ism, MUARRIF_GOOGLE_MAJJANI);
        assert_ne!(
            hie_muattala.wasf_injilizi, hie_faragh.wasf_injilizi,
            "two states, two remedies"
        );

        let badeel = idadat_bi_muzawwidin(vec![tarif("a", false), tarif("b", true)], Some("a"));
        assert_eq!(
            muzawwid_muntakhab(&badeel).muarrif,
            "b",
            "the settings' own fallthrough"
        );
        let hie = muzawwid_warsha_hie(&badeel);
        assert_eq!(hie.hala, HalatMuzawwidinHie::Badeel);
        assert_eq!(hie.ism, "b");
        assert_eq!(hie.wasf_injilizi, HalatMuzawwidin::Badeel.injilizi());
        assert_eq!(hie.wasf_arabi, HalatMuzawwidin::Badeel.arabi());

        let mukhtar = idadat_bi_muzawwidin(vec![tarif("a", true), tarif("b", true)], Some("b"));
        assert_eq!(
            muzawwid_muntakhab(&mukhtar).muarrif,
            "b",
            "the default wins when enabled"
        );
        assert_eq!(
            muzawwid_warsha_hie(&mukhtar).hala,
            HalatMuzawwidinHie::Mukhtar
        );
    }

    /// The built-in provider builds with no keychain, no credential and no ceiling, and
    /// declares itself free — so the surfaces that build it on mount are not unlocking a
    /// secret, and the spend gates have nothing to gate.
    #[test]
    fn almajjani_yubna_bila_khazina_wa_bila_thaman() -> NatijatIkhtibar {
        let muzawwid = bin_muzawwid(&IdadatMuzawwid::google_majjani(), u64::MAX, 0)?;
        assert_eq!(muzawwid.ism(), MUARRIF_GOOGLE_MAJJANI);
        assert_eq!(muzawwid.namudhaj(), "gtx");
        assert!(!muzawwid.qudrat().taklifa.madfu());
        assert_eq!(muzawwid.takalif().saqf(), 0);
        Ok(())
    }

    /// Both refusals carry both languages and the count.
    #[test]
    fn alrafdan_bilughatayn() {
        let mashru = KhataWarshaAmr::MashruTalif {
            talifa: 2,
            masar: PathBuf::from("nusus.jsonl"),
        };
        assert!(mashru.arabi().contains("سطرين"));
        assert!(mashru.injilizi().contains("2 rows"));
        assert_eq!(mashru.khutwa(), Khutwa::FathNusus);

        let huzma = KhataWarshaAmr::HuzmaBihaTalaf { talifa: 1 };
        assert!(huzma.arabi().contains("سطر واحد"));
        assert!(huzma.injilizi().contains("1 row"));
        assert_eq!(huzma.khutura(), Khutura::Tanbeeh);
    }

    /// The oversize refusal names the file in both languages and says what to
    /// do with it, because nothing in the application can open it for them.
    #[test]
    fn rafd_aljadwal_alkabir_yusammi_almalaf() {
        let khata = KhataWarshaAmr::JadwalMashwarKabir {
            masar: PathBuf::from("/bayanat/tilqai/luba/jawla/mashru/nusus.json"),
            hajm: 300 * 1024 * 1024,
            hadd: taarib_tilqai::tarjama::HADD_HAJM_NUSUS,
        };
        assert_eq!(khata.ramz(), Ramz::jadeed(arqam::STUDIO + 52));
        assert_eq!(khata.khutwa(), Khutwa::LaShay);
        assert_eq!(khata.khutura(), Khutura::Khatar);
        assert!(khata.arabi().contains("nusus.json"));
        assert!(khata.injilizi().contains("nusus.json"));
        assert!(
            khata
                .injilizi()
                .contains(&taarib_tilqai::tarjama::HADD_HAJM_NUSUS.to_string()),
            "the sentence says the cap, so the number is not a mystery"
        );
        assert_eq!(
            khata.siyaq().get("hadd"),
            Some(&QeemaSiyaq::Hajm(taarib_tilqai::tarjama::HADD_HAJM_NUSUS))
        );
    }
}
