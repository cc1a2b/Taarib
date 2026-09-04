//! التقديم — the submission wizard, the contributor's trail, the owner's review console, and
//! the requests board.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_istikhraj::mashru::MashruMaftuh;
use taarib_khatm::malik::{MIRSAT_MALIK, huwa_malik};
use taarib_khatm::{MiftahKhass, MudaqqiqEd25519, mafatih};
use taarib_makhzan::sijillat::SijillMuharrik;
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::{Luba, LubaId};
use taarib_mustalahat::musahim::{MusahimId, Sumaa};
use taarib_mustalahat::nass::{AlamJawda, MudkhalNass, NassId};
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RukhsaRuqaa, RuqaaId, TareeqaTarjama};
use taarib_ruqaa::muhadhah::BaytMuhadhah;
use taarib_ruqaa::qari;
use taarib_ruqaa::tawqee::{DawrMiftah, Khwarizmiya, KutlatTawqee};
use taarib_saff::khatt::{MawridKhatt, SilsilatKhutut};
use taarib_taqdeem::bawwaba::{
    BandFahs, FahsHasim, HalatBand, Iqrarat, MudkhalatBawwaba, QaimatFahs, Tahdheer,
};
use taarib_taqdeem::hawiya::{HawiyatMusahim, SalahiyatMalik};
use taarib_taqdeem::muraja::{
    MarjiMuraja, MulakhkhasTaadil, SababRafd, SijillMuraja as SijillMurajaMalik, allaq,
    iaatimad, ishab, urfud, utlub_taadil,
};
use taarib_taqdeem::musawwada::{BidayatMusawwada, Musawwada, masar_musawwada};
use taarib_taqdeem::irsal::{
    IdadatIrsal, IdadatMustawda, IdadatTawthiq, MUHLAT_TALAB, QalabRabt, TalabIrsal, TalabJihaz,
    akmil_tawthiq, bina_amil, hat_ramz, ibda_tawthiq, irsal, khzin_ramz, mulakhkhas,
    wasf_talab_damj,
};
use taarib_taqdeem::nashr::waqqi;
use taarib_taqdeem::sandooq::BeeatSandooq;
use taarib_taqdeem::taaliq::{NassTaaliq, Taaliq, TaaliqId};
use taarib_taqdeem::tabur::{HalatFuhus, IhsaTabur, MudkhalTabur, ihsa};
use taarib_taqdeem::talabat::{LawhatTalabat, TalabTarjama};
use taarib_tarqee::bawwaba::KhattMujammaa;
use taarib_tarqee::bayan::MuharrikHuzma;
use taarib_tarqee::fuhusat::{MudkhalatFahs, WasfHuzma, ijri as ijri_fuhus};
use taarib_tarqee::irtibat::IrtibatBina;
use taarib_tarqee::mujammi::{MudkhalatTajmee, ijmaa};
use taarib_tarqee::tahdid_maqasat::IktishafMaqasat;
use taarib_tarqee::taqrir_tajawuz::TaqrirTajawuz;
use taarib_tarqee::takhtit::KhiyaratTasbeeq;
use taarib_tathbeet::bayan::waqt_alaan;
use taarib_usus::idadat::MakhzanIdadat;
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::masarat::{Masarat, kitaba_dharra};
use taarib_usus::{ISDAR, khata_min};

use crate::luba_awamir::{huwiya, ijlib_luba};
use crate::warsha_awamir::{TaaliqWarshaHie, masrad_kamil, muharrir_mahalli, taaliq_hie};

/// The keychain account holding the contributor signing key.
const ISM_MIFTAH_MUSAHIM: &str = "musahim";

/// Who this session is, and whether it holds the owner key.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct JalsaHie {
    /// Whether the owner key is in this machine's keychain.
    pub malik: bool,
    /// The local identity's fingerprint.
    pub musahim: String,
    /// The display name it signs with.
    pub ism: String,
}

/// One pre-flight checklist row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrFahsHie {
    /// The check's stable key.
    pub band: String,
    /// Whether it blocks rather than warns.
    pub hasim: bool,
    /// Where the row stands, as a stable key.
    pub hala: String,
    /// Where the row stands, in Arabic.
    ///
    /// Sent because the interface cannot derive it. `hala` is a slug with no
    /// generated union behind it, so a screen wanting to name the state had to
    /// keep its own map of slugs to words — and a map that has to guess is a map
    /// that labels an unrecognised state as something, which on a submission
    /// gate means showing a check as passed when nobody has run it. Every other
    /// state on this boundary already travels with its own wording; this one
    /// was the exception.
    pub hala_arabi: String,
    /// The row's label, in Arabic.
    pub wasf_arabi: String,
    /// The sentence beside it, in Arabic.
    pub tafsil_arabi: String,
    /// The strings it links to.
    pub nusus: Vec<String>,
    /// How many strings it found in total.
    pub adad: u32,
}

/// The whole pre-flight gate, blocking checks first.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct QaimatFahsHie {
    /// Every row.
    pub sutur: Vec<SatrFahsHie>,
    /// Whether nothing fails and nothing awaits an acknowledgement.
    pub jahiza: bool,
}

/// One recorded state transition of a submission.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IntiqalHie {
    /// The state it left.
    pub min: String,
    /// The state it entered.
    pub ila: String,
    /// When, RFC 3339.
    pub waqt: String,
}

/// A submission draft as the submission screen draws it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MusawwadaHie {
    /// The patch lineage.
    pub ruqaa: String,
    /// The revision this draft is on.
    pub murajaa: u32,
    /// The game's identity.
    pub muarrif: String,
    /// The game's title.
    pub ism_luba: String,
    /// The patch title.
    pub unwan: String,
    /// The listing description.
    pub sharh: String,
    /// The changelog for this revision.
    pub taghyeerat: String,
    /// The licence identifier.
    pub rukhsa: String,
    /// The declared method, in Arabic.
    pub tareeqa_arabi: String,
    /// The state's stable key.
    pub hala: String,
    /// The state, in Arabic.
    pub hala_arabi: String,
    /// Whether the contributor may still edit it.
    pub qabila_lil_tahreer: bool,
    /// The package size, bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm_huzma: u64,
    /// The package hash, abbreviated.
    pub basmat_huzma: String,
    /// Every string the project holds.
    pub adad_nusus: u32,
    /// The approved among them.
    pub muakkada: u32,
    /// Translated over total, 0 to 1.
    pub taghtiya_nisba: f64,
    /// The pre-flight gate as it stands now.
    pub qaima: QaimatFahsHie,
    /// When the draft was started, RFC 3339.
    pub ansha: String,
    /// Every state transition so far.
    pub tareekh: Vec<IntiqalHie>,
}

/// What a completed submission transport reports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IrsalHie {
    /// The patch lineage.
    pub ruqaa: String,
    /// The revision submitted.
    pub murajaa: u32,
    /// The pull request, when a forge carried it; null for a local handoff.
    pub rabt_talab_damj: Option<String>,
    /// The package's content hash.
    pub basma: String,
}

/// One audit-log row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrSijillHie {
    /// The action's stable key.
    pub ijra: String,
    /// The action, in Arabic.
    pub wasf_arabi: String,
    /// The written reason, where the action carries one.
    pub sabab: Option<String>,
    /// When, RFC 3339.
    pub waqt: String,
    /// The lineage acted on.
    pub ruqaa: String,
    /// The revision acted on.
    pub murajaa: u32,
}

/// One of my submissions, with its trail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MusahamaHie {
    /// The patch lineage.
    pub ruqaa: String,
    /// Its current revision.
    pub murajaa: u32,
    /// The game's identity.
    pub muarrif: String,
    /// The game's title.
    pub ism_luba: String,
    /// The patch title.
    pub unwan: String,
    /// The state, in Arabic.
    pub hala_arabi: String,
    /// Whether it has stopped moving.
    pub nihaiya: bool,
    /// When the draft was started, RFC 3339.
    pub waqt: String,
    /// The review conversation and every action taken on it.
    pub sijill: Vec<SatrSijillHie>,
}

/// One queue row in the review console.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MudkhalTaburHie {
    /// The patch lineage.
    pub ruqaa: String,
    /// The revision waiting.
    pub murajaa: u32,
    /// The title.
    pub unwan: String,
    /// The game's identity.
    pub muarrif: String,
    /// The game's title.
    pub ism_luba: String,
    /// The contributor's fingerprint, abbreviated.
    pub musahim: String,
    /// Their display name.
    pub ism_musahim: String,
    /// The declared method, in Arabic.
    pub tareeqa_arabi: String,
    /// Translated over total, 0 to 1.
    pub taghtiya_nisba: f64,
    /// When it was submitted, RFC 3339.
    pub waqt: String,
    /// How long it has waited, in Arabic.
    pub umr_arabi: String,
    /// The check status's stable key.
    pub fuhus: String,
    /// The check status, in Arabic.
    pub fuhus_arabi: String,
}

/// The console queue with its header counts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaburHie {
    /// The rows, longest wait first.
    pub sufuf: Vec<MudkhalTaburHie>,
    /// How many rows.
    pub majmu: u32,
    /// How many failed their checks.
    pub akhfaqat: u32,
    /// How many have not had checks run.
    pub lam_tujra: u32,
    /// How many passed.
    pub najahat: u32,
    /// The longest wait, in minutes.
    pub aqsa_umr_daqaiq: u32,
}

/// A contributor's standing, as the console shows it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SumaaHie {
    /// Patches published.
    pub manshura: u32,
    /// Accepted with no changes requested.
    pub qubila_bila_taadil: u32,
    /// Returned for revision.
    pub tulib_taadil: u32,
    /// Rejected outright.
    pub marfuda: u32,
    /// Published then revoked.
    pub masbuba: u32,
}

/// One package string pair, for the console's side-by-side table.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SafHuzmaHie {
    /// The source text.
    pub masdar: String,
    /// The Arabic.
    pub hadaf: String,
}

/// One overflow-report row, worst first.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TajawuzHie {
    /// The string, abbreviated.
    pub nass: String,
    /// Its measured width, in the game's pixels.
    pub ard: f64,
    /// The width it has.
    pub mutah: f64,
    /// The size it was measured at.
    pub hajm: f64,
}

/// Everything the console's detail screen draws for one submission.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TafasilMurajaHie {
    /// The submission itself, checklist included.
    pub musawwada: MusawwadaHie,
    /// The contributor's standing.
    pub sumaa: SumaaHie,
    /// The side-by-side table, from the local project when one exists.
    pub sufuf: Vec<SafHuzmaHie>,
    /// The overflow report, worst first.
    pub tajawuz: Vec<TajawuzHie>,
    /// The review conversation.
    pub taaliqat: Vec<TaaliqWarshaHie>,
    /// Every action taken on this lineage.
    pub sijill: Vec<SatrSijillHie>,
    /// A diff against the previous revision; null when it is not on this machine.
    pub farq_sabiq: Option<String>,
}

/// What one sandbox verification reported.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerSandooqHie {
    /// The outcome, in Arabic.
    pub hasila_arabi: String,
    /// Whether install and post-write verification both held.
    pub salim: bool,
    /// How many files the sandbox restore brought back.
    pub mustaada: u32,
    /// Paths the restore left behind, when any.
    pub mutabaqqi: Vec<String>,
}

/// One game's open translation requests.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TalabLubaHie {
    /// The game's identity.
    pub muarrif: String,
    /// Its title.
    pub ism_luba: String,
    /// How many people asked.
    pub adad: u32,
    /// When the newest request was filed, RFC 3339.
    pub akhir_waqt: Option<String>,
}

/// The demand-sorted requests board.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct LawhatTalabatHie {
    /// The rows, most requested first.
    pub sufuf: Vec<TalabLubaHie>,
}

fn masar_sijill_malik(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("muraja_sijill.json")
}

fn masar_taaliqat_muraja(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("taaliqat_muraja.json")
}

fn masar_sumaa(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("sumaa.json")
}

fn masar_talabat(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("talabat.json")
}

fn masar_hawiya(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("hawiya_musahim.json")
}

fn mujallad_huzam(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("taqdeem").join("huzam")
}

/// The pull-request material the build wrote beside the package.
#[derive(serde::Serialize, serde::Deserialize)]
struct MalafTalab {
    record: MulakhkhasRuqaa,
    matn: String,
    ism_luba: String,
}

/// The device-authorization request held between starting the flow and polling it.
///
/// `parking_lot`, not `std`: a poisoned lock here would mean answering a live
/// authorization poll with an unwrap, and the guard is held across no await.
#[derive(Debug, Default)]
pub struct JihazMuallaq(pub parking_lot::Mutex<Option<TalabJihaz>>);

/// One writer at a time over the submission side files — drafts, the audit
/// log, reputations, comments, the listing index, and the requests board.
///
/// `tokio`, not `parking_lot`: this guard is held across awaits — a submission
/// writes several files with I/O between them — and blocking the runtime
/// thread there would stall every other command.
#[derive(Debug, Default)]
pub struct QuflTaqdeem(pub tokio::sync::Mutex<()>);

fn masar_talab_json(masarat: &Masarat, ruqaa: RuqaaId, murajaa: u32) -> PathBuf {
    mujallad_huzam(masarat).join(format!("{ruqaa}-r{murajaa}.talab.json"))
}

fn masar_iqrarat(masarat: &Masarat, ruqaa: RuqaaId) -> PathBuf {
    masarat.jidhr_bayanat().join("taqdeem").join(format!("iqrarat-{ruqaa}.json"))
}

fn mujallad_manshurat(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("manshurat")
}

fn masar_fahras_manshurat(masarat: &Masarat) -> PathBuf {
    mujallad_manshurat(masarat).join("fahras.json")
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct MalafHawiya {
    #[serde(default)]
    ism: String,
    #[serde(default)]
    itimad: Option<String>,
}

/// This machine's contributor identity: fingerprint, display name, credit line.
pub(crate) fn hawiyati(masarat: &Masarat) -> Natija<(MusahimId, String, Option<String>)> {
    let musahim = muharrir_mahalli(masarat)?;
    let malaf: MalafHawiya = crate::warsha_awamir::iqra_janibi(&masar_hawiya(masarat))?;
    let ism =
        if malaf.ism.trim().is_empty() { musahim.mukhtasar() } else { malaf.ism.clone() };
    Ok((musahim, ism, malaf.itimad))
}

/// The contributor signing key, minted into the keychain on first use.
pub(crate) fn miftah_musahim() -> Natija<MiftahKhass> {
    if let Ok(khass) = mafatih::hat(ISM_MIFTAH_MUSAHIM) {
        return Ok(khass);
    }
    let _aam = mafatih::wallid(ISM_MIFTAH_MUSAHIM).map_err(|q| Khata::min_tafsir(&q))?;
    mafatih::hat(ISM_MIFTAH_MUSAHIM).map_err(|q| Khata::min_tafsir(&q))
}

/// The owner authority, or the refusal a contributor session gets.
fn salahiyat_malik() -> Natija<(SalahiyatMalik, MiftahKhass)> {
    let khass = taarib_khatm::malik::hat_malik()
        .map_err(|_| Khata::from(KhataTaqdeemAmr::MalikFaqat))?;
    let salahiya = SalahiyatMalik::bi_miftah(&khass, &MIRSAT_MALIK.miftah)
        .ok_or_else(|| Khata::from(KhataTaqdeemAmr::MalikFaqat))?;
    Ok((salahiya, khass))
}

fn musawwadat_kul(masarat: &Masarat) -> Natija<Vec<Musawwada>> {
    let sijill = Musawwada::iqra_kull(masarat.jidhr_bayanat()).map_err(Khata::from)?;
    // The three screens fed from here draw a flat list and have no row for a
    // draft that would not load, so each failure is named in the log instead of
    // vanishing. Giving it a row of its own is a change to `MusahamaHie` and the
    // Contributions screen, not to this helper.
    for taathhur in &sijill.mutaadhira {
        tracing::warn!(
            masar = %taathhur.masar.display(),
            khata = %Khata::min_tafsir(&taathhur.khata).li_sijill(),
            "a saved draft could not be read and is missing from the list"
        );
    }
    Ok(sijill.musawwadat)
}

fn musawwadat_lil_luba(masarat: &Masarat, id: LubaId) -> Natija<Option<Musawwada>> {
    let kul = musawwadat_kul(masarat)?;
    Ok(kul
        .into_iter()
        .filter(|musawwada| musawwada.luba() == id)
        .max_by(|awwal, thani| {
            awwal
                .murajaa()
                .qeema()
                .cmp(&thani.murajaa().qeema())
                .then_with(|| awwal.ansha().cmp(thani.ansha()))
        }))
}

fn ihfaz_musawwada(masarat: &Masarat, musawwada: &Musawwada) -> Natija<()> {
    musawwada
        .ihfaz(&masar_musawwada(masarat.jidhr_bayanat(), musawwada.id()))
        .map_err(Khata::from)
}

fn musawwada_bil_ruqaa(masarat: &Masarat, ruqaa: &str) -> Natija<Musawwada> {
    let id: RuqaaId = serde_json::from_value(serde_json::Value::String(ruqaa.to_owned()))
        .map_err(|_| Khata::from(KhataTaqdeemAmr::TaqdeemGhayrMawjud { ruqaa: ruqaa.to_owned() }))?;
    let masar = masar_musawwada(masarat.jidhr_bayanat(), id);
    if !masar.is_file() {
        return Err(Khata::from(KhataTaqdeemAmr::TaqdeemGhayrMawjud {
            ruqaa: ruqaa.to_owned(),
        }));
    }
    Musawwada::iqra(&masar).map_err(Khata::from)
}

const fn slug_tahdheer(tahdheer: Tahdheer) -> &'static str {
    match tahdheer {
        Tahdheer::TajawuzKatheer => "tajawuz_katheer",
        Tahdheer::MutabaqqiKabeer => "mutabaqqi_kabeer",
        Tahdheer::TareeqaAaliya => "tareeqa_aaliya",
        Tahdheer::MustalahMukhtalif => "mustalah_mukhtalif",
    }
}

fn tahdheer_min_slug(slug: &str) -> Option<Tahdheer> {
    Tahdheer::KULL.into_iter().find(|tahdheer| slug_tahdheer(*tahdheer) == slug)
}

const fn slug_hasim(fahs: FahsHasim) -> &'static str {
    match fahs {
        FahsHasim::Fuhusat => "fuhusat",
        FahsHasim::Shahada => "shahada",
        FahsHasim::Taghtiya => "taghtiya",
        FahsHasim::Takrar => "takrar",
        FahsHasim::IrtibatIstirad => "irtibat_istirad",
        FahsHasim::LughaRasmiya => "lugha_rasmiya",
    }
}

const fn slug_hala_band(hala: HalatBand) -> &'static str {
    match hala {
        HalatBand::Ijtaz => "ijtaz",
        HalatBand::Rasab => "rasab",
        HalatBand::YantazirIqrar => "yantazir_iqrar",
        HalatBand::Muqarr => "muqarr",
    }
}

fn qaimat_hie(qaima: &QaimatFahs) -> QaimatFahsHie {
    QaimatFahsHie {
        sutur: qaima
            .sutur()
            .iter()
            .map(|satr| SatrFahsHie {
                band: match satr.band {
                    BandFahs::Hasim(fahs) => slug_hasim(fahs).to_owned(),
                    BandFahs::Tahdheer(tahdheer) => slug_tahdheer(tahdheer).to_owned(),
                },
                hasim: matches!(satr.band, BandFahs::Hasim(_)),
                hala: slug_hala_band(satr.hala).to_owned(),
                hala_arabi: satr.hala.wasf_arabi().to_owned(),
                wasf_arabi: satr.band.wasf_arabi().to_owned(),
                tafsil_arabi: satr.tafsil_arabi.clone(),
                nusus: satr.nusus.iter().map(ToString::to_string).collect(),
                adad: crate::warsha_awamir::adad_u32(satr.adad),
            })
            .collect(),
        jahiza: qaima.jahiza(),
    }
}

fn iqra_iqrarat(masarat: &Masarat, ruqaa: RuqaaId) -> Natija<Iqrarat> {
    let aslama: Vec<String> = crate::warsha_awamir::iqra_janibi(&masar_iqrarat(masarat, ruqaa))?;
    let mut iqrarat = Iqrarat::jadeeda();
    for slug in aslama {
        if let Some(tahdheer) = tahdheer_min_slug(&slug) {
            iqrarat.aqirr(tahdheer);
        }
    }
    Ok(iqrarat)
}

fn uktub_iqrarat(masarat: &Masarat, ruqaa: RuqaaId, iqrarat: &Iqrarat) -> Natija<()> {
    let aslama: Vec<&'static str> = Tahdheer::KULL
        .into_iter()
        .filter(|tahdheer| iqrarat.muqarr(*tahdheer))
        .map(slug_tahdheer)
        .collect();
    crate::warsha_awamir::uktub_janibi(&masar_iqrarat(masarat, ruqaa), &aslama)
}

/// Opens the game's translation project rows, refusing when none exists.
fn nusus_mashru(masarat: &Masarat, id: LubaId) -> Natija<(MashruMaftuh, Vec<MudkhalNass>)> {
    let jidhr = masarat.mashari().join(id.to_string());
    if !jidhr.join(taarib_istikhraj::mashru::MALAF_MASHRU).is_file() {
        return Err(Khata::from(KhataTaqdeemAmr::MashruGhayrMawjud { ism: id.to_string() }));
    }
    let mashru = MashruMaftuh::iftah(jidhr).map_err(Khata::from)?;
    let (sufuf, _talifa) = mashru.iqra_nusus().map_err(Khata::from)?;
    Ok((mashru, sufuf))
}

/// Recomputes the gate checklist for one draft against the live project.
fn qaimat_lil(masarat: &Masarat, musawwada: &Musawwada) -> Natija<QaimatFahs> {
    let (mashru, sufuf) = nusus_mashru(masarat, musawwada.luba())?;
    let masrad = masrad_kamil(mashru.jidhr(), &sufuf);
    let mukhalafat: Vec<NassId> = sufuf
        .iter()
        .filter(|mudkhal| {
            masrad
                .afhas(mudkhal)
                .iter()
                .any(|alam| matches!(alam, AlamJawda::MustalahMukhtalif { .. }))
        })
        .map(|mudkhal| mudkhal.id)
        .collect();
    let manshurat: Vec<MulakhkhasRuqaa> =
        crate::warsha_awamir::iqra_janibi(&masar_fahras_manshurat(masarat))?;
    let iqrarat = iqra_iqrarat(masarat, musawwada.id())?;
    // Whatever this process already established about the game's own Arabic.
    // Read rather than computed: the gate runs from seven places here and the
    // verdict costs a container walk, so it is decided once on the game screen
    // and remembered — see `luba_awamir`, which owns both the walk and the
    // memory. A game nobody has opened yet answers `None`, and the gate reports
    // that as "not checked" rather than as "no Arabic".
    let hukm_lugha = crate::luba_awamir::hukm_mukhazzan(musawwada.luba());
    Ok(taarib_taqdeem::bawwaba::ifhas(
        &MudkhalatBawwaba {
            musawwada,
            madakhil: &sufuf,
            takhtitat_fashila: &[],
            manshurat: &manshurat,
            mukhalafat_mustalah: &mukhalafat,
            lugha_rasmiya: hukm_lugha.as_ref(),
        },
        &iqrarat,
    ))
}

fn nisbat_taghtiya(majmu: u32, mutarjam: u32) -> f64 {
    if majmu == 0 { 0.0 } else { f64::from(mutarjam) / f64::from(majmu) }
}

fn intiqalat(musawwada: &Musawwada) -> Vec<IntiqalHie> {
    let mut sabiq = "musawwada".to_owned();
    let mut natija = Vec::with_capacity(musawwada.tareekh().len());
    for qayd in musawwada.tareekh() {
        let ila = qayd.ila.ism().to_owned();
        natija.push(IntiqalHie { min: sabiq.clone(), ila: ila.clone(), waqt: qayd.waqt.clone() });
        sabiq = ila;
    }
    natija
}

fn musawwada_hie(musawwada: &Musawwada, qaima: QaimatFahsHie) -> MusawwadaHie {
    let adad = musawwada.adad_nusus();
    let taghtiya = musawwada.taghtiya();
    MusawwadaHie {
        ruqaa: musawwada.id().to_string(),
        murajaa: musawwada.murajaa().qeema(),
        muarrif: musawwada.luba().to_string(),
        ism_luba: musawwada.wasf().ism_luba.clone(),
        unwan: musawwada.wasf().unwan.clone(),
        sharh: musawwada.sharh().to_owned(),
        taghyeerat: musawwada.taghyeerat().to_owned(),
        rukhsa: musawwada.wasf().rukhsa.muarrif().to_owned(),
        tareeqa_arabi: musawwada.tareeqa().wasf_arabi().to_owned(),
        hala: musawwada.hala().ism().to_owned(),
        hala_arabi: musawwada.hala().wasf_arabi().to_owned(),
        qabila_lil_tahreer: musawwada.hala().qabila_lil_tahreer(),
        hajm_huzma: musawwada.hajm_huzma(),
        basmat_huzma: musawwada.basmat_huzma().mukhtasara(),
        adad_nusus: adad.majmu,
        muakkada: adad.muakkad,
        taghtiya_nisba: nisbat_taghtiya(taghtiya.majmu, taghtiya.mutarjam),
        qaima,
        ansha: musawwada.ansha().to_owned(),
        tareekh: intiqalat(musawwada),
    }
}

/// Every usable font under Taarib's own font directory, as the chain and the
/// bundling records, in one order.
fn khutut_lil_tajmee(masarat: &Masarat) -> Natija<(SilsilatKhutut, Vec<KhattMujammaa>)> {
    // Both roots: the bundled set is what a clean install bundles a patch with,
    // and without it a first run could not build one at all.
    let judhur = crate::mukawwinat_tahmil::judhur_khutut(masarat);
    let masarat_khutut: Vec<PathBuf> = crate::mukawwinat_tahmil::milaffat_khutut(&judhur);
    if masarat_khutut.is_empty() {
        return Err(Khata::from(KhataTaqdeemAmr::KhututNaqisa {
            sabab: "no usable font was found in either font directory".to_owned(),
        }));
    }

    let mut azwaj: Vec<(PathBuf, Arc<MawridKhatt>)> = Vec::new();
    for masar in masarat_khutut {
        let Ok(bayt) = std::fs::read(&masar) else { continue };
        let bayt = Arc::new(bayt);
        let mawrid = MawridKhatt::jadeed(Arc::clone(&bayt), 0)
            .or_else(|_| MawridKhatt::jadeed_latini(bayt, 0));
        if let Ok(mawrid) = mawrid {
            azwaj.push((masar, Arc::new(mawrid)));
        }
    }
    // The chain's head must shape Arabic; the bundling list mirrors the order.
    azwaj.sort_by_key(|(_, mawrid)| mawrid.fahs_arabi().is_err());

    let mut khutut = Vec::with_capacity(azwaj.len());
    for (fahras, (masar, _)) in azwaj.iter().enumerate() {
        let alam = u16::try_from(fahras).unwrap_or(u16::MAX);
        // The gate proves a bundled font came from a font root rather than
        // from somewhere else on disk, so it is given the root this one
        // actually came from — there is more than one, and a font from the
        // bundle is not inside the user's directory.
        let jidhr = judhur
            .iter()
            .find(|jidhr| masar.starts_with(jidhr))
            .ok_or_else(|| {
                Khata::from(KhataTaqdeemAmr::KhututNaqisa {
                    sabab: format!("{} is not inside any font root", masar.display()),
                })
            })?;
        khutut.push(KhattMujammaa::min_majmua(jidhr, masar, alam).map_err(Khata::from)?);
    }
    let silsila = SilsilatKhutut::jadeeda(azwaj.into_iter().map(|(_, mawrid)| mawrid).collect())
        .map_err(|khata| {
            Khata::from(KhataTaqdeemAmr::KhututNaqisa { sabab: khata.to_string() })
        })?;
    Ok((silsila, khutut))
}

fn rukhsa_min_slug(slug: &str, ism: Option<&str>) -> Natija<RukhsaRuqaa> {
    Ok(match slug {
        "cc0" => RukhsaRuqaa::Cc0,
        "cc_by" => RukhsaRuqaa::CcBy,
        "cc_by_sa" => RukhsaRuqaa::CcBySa,
        "milkiya_khassa" => RukhsaRuqaa::MilkiyaKhassa,
        "ukhra" => RukhsaRuqaa::Ukhra {
            ism: ism.unwrap_or_default().trim().to_owned(),
        },
        _ => {
            return Err(Khata::from(KhataTaqdeemAmr::RukhsaMajhula {
                rukhsa: slug.to_owned(),
            }));
        }
    })
}

fn tareeqa_min_slug(slug: &str) -> Natija<TareeqaTarjama> {
    Ok(match slug {
        "bashariya_kamila" => TareeqaTarjama::BashariyaKamila,
        "aaliya_thum_bashariya" => TareeqaTarjama::AaliyaThumBashariya,
        "aaliya_faqat" => TareeqaTarjama::AaliyaFaqat,
        _ => {
            return Err(Khata::from(KhataTaqdeemAmr::TareeqaMajhula {
                tareeqa: slug.to_owned(),
            }));
        }
    })
}

const fn sumaa_hie(sumaa: &Sumaa) -> SumaaHie {
    SumaaHie {
        manshura: sumaa.ruqaa_manshura,
        qubila_bila_taadil: sumaa.qubila_bila_taadil,
        tulib_taadil: sumaa.tulib_taadil,
        marfuda: sumaa.marfuda,
        masbuba: sumaa.masbuba,
    }
}

fn sumaa_li(masarat: &Masarat, musahim: &MusahimId) -> Natija<Sumaa> {
    let kharita: BTreeMap<String, Sumaa> =
        crate::warsha_awamir::iqra_janibi(&masar_sumaa(masarat))?;
    Ok(kharita.get(musahim.nass()).cloned().unwrap_or_default())
}

fn haddith_sumaa(
    masarat: &Masarat,
    musahim: &MusahimId,
    tabdeel: impl FnOnce(&mut Sumaa),
) -> Natija<()> {
    let mut kharita: BTreeMap<String, Sumaa> =
        crate::warsha_awamir::iqra_janibi(&masar_sumaa(masarat))?;
    let mudkhal = kharita.entry(musahim.nass().to_owned()).or_default();
    tabdeel(mudkhal);
    crate::warsha_awamir::uktub_janibi(&masar_sumaa(masarat), &kharita)
}

fn sijill_malik(masarat: &Masarat) -> Natija<SijillMurajaMalik> {
    SijillMurajaMalik::iqra_aw_farigh(&masar_sijill_malik(masarat)).map_err(Khata::from)
}

fn uktub_sijill_malik(masarat: &Masarat, sijill: &SijillMurajaMalik) -> Natija<()> {
    sijill.uktub(&masar_sijill_malik(masarat)).map_err(Khata::from)
}

fn satr_sijill_hie(qayd: &taarib_taqdeem::muraja::QaydMuraja) -> SatrSijillHie {
    SatrSijillHie {
        ijra: qayd.ijra().ramz().to_owned(),
        wasf_arabi: qayd.ijra().wasf_arabi().to_owned(),
        sabab: qayd.ijra().sabab().map(str::to_owned),
        waqt: qayd.waqt().to_owned(),
        ruqaa: qayd.ruqaa().to_string(),
        murajaa: qayd.murajaa().qeema(),
    }
}

fn taaliqat_muraja(masarat: &Masarat) -> Natija<BTreeMap<String, Vec<Taaliq>>> {
    crate::warsha_awamir::iqra_janibi(&masar_taaliqat_muraja(masarat))
}

/// Who this session is, and whether it holds the owner key.
///
/// # Errors
///
/// Whatever the identity file raises.
#[tauri::command]
#[specta::specta]
pub async fn jalsati(masarat: tauri::State<'_, Masarat>) -> Result<JalsaHie, Khata> {
    let (musahim, ism, _itimad) = hawiyati(&masarat)?;
    Ok(JalsaHie { malik: malik_bi_muhla().await, musahim: musahim.mukhtasar(), ism })
}

/// How long the owner-key probe may take before the answer is "not the owner".
///
/// The keychain read is a blocking platform call: a locked Secret Service
/// collection answers with a prompt whose completion is a D-Bus *signal* with
/// no reply timeout, and macOS raises an authorization dialog. Either one, on
/// the thread the first frame is painted from, is a window that never appears.
const MUHLAT_MALIK: std::time::Duration = std::time::Duration::from_secs(3);

/// Whether this machine holds the owner key, bounded in time and off the UI
/// thread.
///
/// A timeout answers `false` — a contributor session, which is the safe
/// narrower answer — and says so in the log rather than silently.
async fn malik_bi_muhla() -> bool {
    let amal = tauri::async_runtime::spawn_blocking(huwa_malik);
    match tokio::time::timeout(MUHLAT_MALIK, amal).await {
        Ok(Ok(malik)) => malik,
        Ok(Err(sabab)) => {
            tracing::warn!(%sabab, "the owner-key probe did not finish");
            false
        }
        Err(_) => {
            tracing::warn!(
                thawani = MUHLAT_MALIK.as_secs(),
                "the keychain did not answer in time; continuing as a contributor session"
            );
            false
        }
    }
}

/// The game's submission draft with a live checklist, or null when none exists.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MashruGhayrMawjud`] when a draft exists but its project is
/// gone, and whatever the draft store raises.
#[tauri::command]
#[specta::specta]
pub fn musawwadat_luba(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
) -> Result<Option<MusawwadaHie>, Khata> {
    let id = huwiya(muarrif)?;
    let Some(musawwada) = musawwadat_lil_luba(&masarat, id)? else {
        return Ok(None);
    };
    let qaima = qaimat_lil(&masarat, &musawwada)?;
    Ok(Some(musawwada_hie(&musawwada, qaimat_hie(&qaima))))
}

/// Compiles the project into a package and starts or replaces the draft.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::TaqdeemMuallaq`] while a submission is with the owner,
/// [`KhataTaqdeemAmr::KhututNaqisa`] when no usable Arabic font is bundled, and
/// whatever the compile pipeline, the keychain, or the draft store raise.
#[tauri::command]
#[specta::specta]
#[expect(clippy::too_many_lines, reason = "one compile, in its forced order")]
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a field of the wizard's IPC payload; the names are the JSON \
              keys the frontend sends, and `specta` allows ten"
)]
pub fn jahhiz_taqdeem(
    muarrif: String,
    unwan: String,
    sharh: String,
    taghyeerat: String,
    rukhsa: String,
    rukhsa_ism: Option<String>,
    tareeqa: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<MusawwadaHie, Khata> {
    use taarib_mustalahat::ruqaa::RuqaaRevision;
    use taarib_taqdeem::musawwada::HalatTaqdeem;

    let _harasa = qufl.0.blocking_lock();
    let id = huwiya(muarrif)?;
    let luba: Luba = ijlib_luba(&makhzan, id)?;
    let (musahim, ism, itimad) = hawiyati(&masarat)?;
    let (mashru, sufuf) = nusus_mashru(&masarat, id)?;

    let taqreer = makhzan
        .bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?
        .ok_or_else(|| Khata::from(KhataTaqdeemAmr::MuharrikMajhul { ism: luba.ism.clone() }))?;

    let sabiq = musawwadat_lil_luba(&masarat, id)?;
    let (ruqaa_id, murajaa) = match &sabiq {
        Some(qadeema) if qadeema.hala().qabila_lil_tahreer() => {
            let raqm = if matches!(qadeema.hala(), HalatTaqdeem::MatlubTaadil { .. }) {
                qadeema.murajaa().talia()
            } else {
                qadeema.murajaa()
            };
            (qadeema.id(), raqm)
        }
        Some(qadeema) if !qadeema.hala().nihaiya() => {
            return Err(Khata::from(KhataTaqdeemAmr::TaqdeemMuallaq {
                ruqaa: qadeema.id().to_string(),
            }));
        }
        _ => (RuqaaId::jadeeda(), RuqaaRevision::AWWAL),
    };

    let wasf = WasfHuzma {
        unwan,
        ism_musahim: ism.clone(),
        ism_luba: luba.ism.clone(),
        rukhsa: rukhsa_min_slug(&rukhsa, rukhsa_ism.as_deref())?,
        tareeqa: tareeqa_min_slug(&tareeqa)?,
        isdar_taarib: ISDAR.to_owned(),
    };

    let (silsila, khutut) = khutut_lil_tajmee(&masarat)?;

    let mut iktishaf = IktishafMaqasat::jadeed();
    for mudkhal in &sufuf {
        iktishaf.sajjil_mudkhal(mudkhal);
    }
    iktishaf.sajjil_fahs(&taqreer);
    let maqasat = iktishaf.ahsi(&sufuf);

    let irtibat = IrtibatBina::min_bayan(&mashru.rasm().bayan, &khutut, None)
        .map_err(Khata::from)?;
    let taghtiya = taarib_tarqee::taghtiya_ruqaa::ihsib_taghtiya(&sufuf, None, None);
    let tajawuz = TaqrirTajawuz::farigh();
    let khiyarat = KhiyaratTasbeeq::default();
    let muharrik = MuharrikHuzma {
        aila: taqreer.muharrik.aila,
        khalfiya: taqreer.muharrik.khalfiya,
        tabaqa: taqreer.tabaqa,
    };

    let ijtiyaz = ijri_fuhus(&MudkhalatFahs {
        madakhil: &sufuf,
        wasf: &wasf,
        takhtitat_fashila: &[],
    })
    .map_err(Khata::from)?;

    let mudkhalat = MudkhalatTajmee {
        nusus: &sufuf,
        id: ruqaa_id,
        murajaa,
        wasf: &wasf,
        muharrik,
        irtibat: &irtibat,
        maqasat: &maqasat,
        taghtiya: &taghtiya,
        tajawuz: &tajawuz,
        khiyarat: &khiyarat,
        mustawa: None,
    };
    let mut huzma = ijmaa(&mudkhalat, ijtiyaz, &silsila, &khutut).map_err(Khata::from)?;

    let khass = miftah_musahim()?;
    let basma_dakhil = {
        let maftuh = qari::Ruqaa::iftah(huzma.bayt.bayt()).map_err(Khata::from)?;
        maftuh.tarwisa().basma
    };
    let lahza = i64::try_from(crate::warsha_awamir::lahza_alaan()).unwrap_or(0);
    let mut kutla = KutlatTawqee {
        dawr: DawrMiftah::Musahim,
        khwarizmiya: Khwarizmiya::Ed25519,
        waqt: lahza,
        miftah: khass.aam().bayt(),
        tawqee: [0; 64],
    };
    kutla.tawqee = khass.waqqi(&kutla.risala(&basma_dakhil));
    huzma.akhtim(&kutla, &MudaqqiqEd25519).map_err(Khata::from)?;

    let mujallad = mujallad_huzam(&masarat);
    taarib_usus::masarat::insha_mujallad(&mujallad)?;
    let masar_huzma =
        mujallad.join(format!("{ruqaa_id}-r{}.ruqaa", murajaa.qeema()));
    kitaba_dharra(&masar_huzma, huzma.bayt.bayt())?;
    let basmat_huzma = Basma::min_bayt(*blake3::hash(huzma.bayt.bayt()).as_bytes());
    let hajm_huzma = u64::try_from(huzma.hajm()).unwrap_or(u64::MAX);

    let hawiya_musahim = HawiyatMusahim { musahim, ism, itimad, miftah: khass.aam().bayt() };
    // The registry record and the pull-request body are written now, while the
    // live manifest exists; the transport later sends them as data.
    let record = mulakhkhas(
        &huzma.bayan,
        &hawiya_musahim,
        Basma::min_bayt(basma_dakhil),
        hajm_huzma,
        String::new(),
        String::new(),
    );
    let matn_talab =
        wasf_talab_damj(&huzma.bayan, &hawiya_musahim, &record, &sharh, Some(&taghyeerat));
    crate::warsha_awamir::uktub_janibi(
        &masar_talab_json(&masarat, ruqaa_id, murajaa.qeema()),
        &MalafTalab { record, matn: matn_talab, ism_luba: luba.ism.clone() },
    )?;

    let bidaya = BidayatMusawwada {
        bayan: &huzma.bayan,
        luba: id,
        masadir: luba.masadir.iter().map(|wahid| wahid.asl().clone()).collect(),
        basmat_asliya: mashru.rasm().bayan.turuq.iter().filter_map(|t| t.basma).collect(),
        masar_huzma,
        basmat_huzma,
        hajm_huzma,
        musahim: hawiya_musahim,
        sharh,
        taghyeerat,
        waqt: waqt_alaan(),
    };
    let musawwada = Musawwada::min_warsha(bidaya).map_err(Khata::from)?;
    ihfaz_musawwada(&masarat, &musawwada)?;

    let qaima = qaimat_lil(&masarat, &musawwada)?;
    Ok(musawwada_hie(&musawwada, qaimat_hie(&qaima)))
}

/// Acknowledges one warning, or withdraws the acknowledgement.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::TahdheerMajhul`] for a warning this build does not know,
/// and whatever the draft store raises.
#[tauri::command]
#[specta::specta]
pub fn aqirr_tahdheer(
    muarrif: String,
    tahdheer: String,
    qeema: bool,
    masarat: tauri::State<'_, Masarat>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<MusawwadaHie, Khata> {
    let _harasa = qufl.0.blocking_lock();
    let id = huwiya(muarrif)?;
    let musawwada = musawwadat_lil_luba(&masarat, id)?.ok_or_else(|| {
        Khata::from(KhataTaqdeemAmr::TaqdeemGhayrMawjud { ruqaa: id.to_string() })
    })?;
    let Some(band) = tahdheer_min_slug(&tahdheer) else {
        return Err(Khata::from(KhataTaqdeemAmr::TahdheerMajhul { tahdheer }));
    };
    let mut iqrarat = iqra_iqrarat(&masarat, musawwada.id())?;
    if qeema {
        iqrarat.aqirr(band);
    } else {
        iqrarat.asqit(band);
    }
    uktub_iqrarat(&masarat, musawwada.id(), &iqrarat)?;
    let qaima = qaimat_lil(&masarat, &musawwada)?;
    Ok(musawwada_hie(&musawwada, qaimat_hie(&qaima)))
}

fn idadat_tawthiq(muarrif_amil: &str) -> IdadatTawthiq {
    IdadatTawthiq {
        rabt_ramz_jihaz: "https://github.com/login/device/code".to_owned(),
        rabt_ramz_wusul: "https://github.com/login/oauth/access_token".to_owned(),
        qalab_tadqiq: QalabRabt::jadeed("https://api.github.com/user"),
        muarrif_amil: muarrif_amil.to_owned(),
        nitaq: "public_repo".to_owned(),
        hisab: "taqdeem".to_owned(),
    }
}

/// The registry repository as `masadir.rasmi` names it, or the named refusal.
#[expect(
    clippy::literal_string_with_formatting_args,
    reason = "`QalabRabt` holds a URL template whose `{malik}` and `{mustawda}` placeholders \
              the transport substitutes; they are not `format!` arguments"
)]
fn mustawda_min_rasmi(rasmi: &str, rabt_tajheez: &str) -> Natija<IdadatMustawda> {
    let baqi = rasmi
        .strip_prefix("https://github.com/")
        .map(|nass| nass.trim_end_matches('/').trim_end_matches(".git"));
    let (malik, mustawda) = match baqi.map(|nass| nass.split_once('/')) {
        Some(Some((malik, mustawda)))
            if !malik.is_empty() && !mustawda.is_empty() && !mustawda.contains('/') =>
        {
            (malik.to_owned(), mustawda.to_owned())
        }
        _ => {
            return Err(Khata::from(KhataTaqdeemAmr::MustawdaGhayrMafhum {
                rasmi: rasmi.to_owned(),
            }));
        }
    };
    let rabt_git = format!("https://github.com/{malik}/{mustawda}.git");
    Ok(IdadatMustawda {
        malik,
        mustawda,
        far_asasi: "main".to_owned(),
        bidayat_far: "taqdeem".to_owned(),
        rabt_git,
        qalab_git_shawka: QalabRabt::jadeed(
            "https://github.com/{malik_shawka}/{mustawda}.git",
        ),
        qalab_shawkati: QalabRabt::jadeed(
            "https://api.github.com/repos/{malik_shawka}/{mustawda}",
        ),
        qalab_shawka: QalabRabt::jadeed("https://api.github.com/repos/{malik}/{mustawda}/forks"),
        qalab_tajheez: QalabRabt::jadeed(rabt_tajheez),
        qalab_talab_damj: QalabRabt::jadeed(
            "https://api.github.com/repos/{malik}/{mustawda}/pulls",
        ),
    })
}

/// Starts forge device authorization; answers what the user must be shown.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::IrsalGhayrMuhayya`] while no client identifier is provisioned, and
/// whatever the forge answers the code request with.
#[tauri::command]
#[specta::specta]
pub async fn abda_tawthiq_taqdeem(
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    jihaz: tauri::State<'_, JihazMuallaq>,
) -> Result<TalabJihazHie, Khata> {
    let hali = idadat.hali();
    let Some(muarrif_amil) =
        hali.masadir.muarrif_amil.as_deref().filter(|nass| !nass.trim().is_empty())
    else {
        return Err(Khata::from(KhataTaqdeemAmr::IrsalGhayrMuhayya {
            naqis: "muarrif_amil",
        }));
    };
    let tawthiq = idadat_tawthiq(muarrif_amil);
    let amil = bina_amil(MUHLAT_TALAB).map_err(Khata::from)?;
    let talab = ibda_tawthiq(&amil, &tawthiq).await.map_err(Khata::from)?;
    let hie = TalabJihazHie {
        ramz_mustakhdim: talab.ramz_mustakhdim.clone(),
        rabt: talab.rabt_tahaqquq.clone(),
        rabt_kamil: talab.rabt_kamil.clone(),
        thawani: talab.yantahi_baad.as_secs(),
    };
    let mut mahfudh = jihaz.0.lock();
    *mahfudh = Some(talab);
    Ok(hie)
}

/// What the device-authorization step shows the user.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TalabJihazHie {
    /// The short code the user types at the forge.
    pub ramz_mustakhdim: String,
    /// The address the user opens.
    pub rabt: String,
    /// The address with the code already in it, when the forge offers one.
    pub rabt_kamil: Option<String>,
    /// How long the code stays valid, in seconds.
    #[specta(type = specta_typescript::Number)]
    pub thawani: u64,
}

/// Passes the gate and hands the submission to the owner's queue.
///
/// The forge transport is not provisioned in this build, so the handoff is
/// local: the object moves to the queue on this machine and no network is
/// touched.
///
/// # Errors
///
/// The gate's own refusals — a blocking failure, an unacknowledged warning —
/// each in its own words, and whatever the draft store raises.
#[tauri::command]
#[specta::specta]
pub async fn sallim_taqdeem(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
    jihaz: tauri::State<'_, JihazMuallaq>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<IrsalHie, Khata> {
    let _harasa = qufl.0.lock().await;
    let id = huwiya(muarrif)?;
    let musawwada = musawwadat_lil_luba(&masarat, id)?.ok_or_else(|| {
        Khata::from(KhataTaqdeemAmr::TaqdeemGhayrMawjud { ruqaa: id.to_string() })
    })?;

    let (mashru, sufuf) = nusus_mashru(&masarat, id)?;
    let masrad = masrad_kamil(mashru.jidhr(), &sufuf);
    let mukhalafat: Vec<NassId> = sufuf
        .iter()
        .filter(|mudkhal| {
            masrad
                .afhas(mudkhal)
                .iter()
                .any(|alam| matches!(alam, AlamJawda::MustalahMukhtalif { .. }))
        })
        .map(|mudkhal| mudkhal.id)
        .collect();
    let manshurat: Vec<MulakhkhasRuqaa> =
        crate::warsha_awamir::iqra_janibi(&masar_fahras_manshurat(&masarat))?;
    let iqrarat = iqra_iqrarat(&masarat, musawwada.id())?;
    let hukm_lugha = crate::luba_awamir::hukm_mukhazzan(musawwada.luba());
    let ijtiyaz = taarib_taqdeem::bawwaba::ijri(
        &MudkhalatBawwaba {
            musawwada: &musawwada,
            madakhil: &sufuf,
            takhtitat_fashila: &[],
            manshurat: &manshurat,
            mukhalafat_mustalah: &mukhalafat,
            lugha_rasmiya: hukm_lugha.as_ref(),
        },
        &iqrarat,
    )
    .map_err(Khata::from)?;

    let waqt = waqt_alaan();
    let ruqaa = musawwada.id().to_string();
    let ruqaa_id = musawwada.id();
    let murajaa = musawwada.murajaa().qeema();
    let basma = musawwada.basmat_huzma().mukhtasara();
    let masar_huzma = musawwada.masar_huzma().to_path_buf();

    // The proof is borrowed by the local transition and consumed by the
    // transport, in that order; a draft already in the queue is re-sent
    // without a second transition.
    let _mahfudha = if musawwada.hala().qabila_lil_tahreer() {
        let muqaddama = musawwada.ursilat(&ijtiyaz, &waqt).map_err(|marfud| {
            Khata::from(KhataTaqdeemAmr::IntiqalMarfudAmr {
                min: marfud.min().to_owned(),
                ila: marfud.ila().to_owned(),
            })
        })?;
        ihfaz_musawwada(&masarat, &muqaddama)?;
        muqaddama
    } else if musawwada.hala().fi_intizar_almalik() {
        musawwada
    } else {
        return Err(Khata::from(KhataTaqdeemAmr::IntiqalMarfudAmr {
            min: musawwada.hala().ism().to_owned(),
            ila: "muqaddama".to_owned(),
        }));
    };

    let hali = idadat.hali();
    let amil_id = hali
        .masadir
        .muarrif_amil
        .as_deref()
        .filter(|nass| !nass.trim().is_empty())
        .map(str::to_owned);
    let rabt_tajheez = hali
        .masadir
        .rabt_tajheez
        .as_deref()
        .filter(|nass| !nass.trim().is_empty())
        .map(str::to_owned);

    let rabt_talab_damj = if let (Some(amil_id), Some(rabt_tajheez)) = (amil_id, rabt_tajheez) {
        let tawthiq = idadat_tawthiq(&amil_id);
        let ramz = if let Some(ramz) = hat_ramz("taqdeem").map_err(Khata::from)? {
            ramz
        } else {
            // The guard is released inside this block: it is `!Send`, and the
            // token exchange below is awaited.
            let muallaq = {
                jihaz.0.lock().take()
            };
            let Some(talab_jihaz) = muallaq else {
                return Err(Khata::from(KhataTaqdeemAmr::TawthiqNaqis));
            };
            let amil = bina_amil(MUHLAT_TALAB).map_err(Khata::from)?;
            let ramz =
                akmil_tawthiq(&amil, &tawthiq, &talab_jihaz).await.map_err(Khata::from)?;
            khzin_ramz("taqdeem", &ramz).map_err(Khata::from)?;
            ramz
        };

        let mustawda = mustawda_min_rasmi(&hali.masadir.rasmi, &rabt_tajheez)?;
        let idadat_irsal = IdadatIrsal { tawthiq, mustawda };

        let malaf_talab: MalafTalab = {
            let masar = masar_talab_json(&masarat, ruqaa_id, murajaa);
            if !masar.is_file() {
                return Err(Khata::from(KhataTaqdeemAmr::TalabNaqis));
            }
            crate::warsha_awamir::iqra_janibi_matlub(&masar)?
        };
        let bayt = std::fs::read(&masar_huzma).map_err(|_| {
            Khata::from(KhataTaqdeemAmr::HuzmaMafquda { masar: masar_huzma.clone() })
        })?;
        let khass = miftah_musahim()?;
        let (musahim_hali, ism_hali, itimad_hali) = hawiyati(&masarat)?;
        let hawiya = HawiyatMusahim {
            musahim: musahim_hali,
            ism: ism_hali,
            itimad: itimad_hali,
            miftah: khass.aam().bayt(),
        };
        let natija = irsal(
            TalabIrsal {
                bayt,
                record: malaf_talab.record,
                matn_talab: malaf_talab.matn,
                ism_luba: malaf_talab.ism_luba,
                hawiya: &hawiya,
                luba: id,
            },
            ijtiyaz,
            &idadat_irsal,
            &ramz,
            &masarat,
        )
        .await
        .map_err(Khata::from)?;
        Some(natija.rabt_talab_damj)
    } else {
        // Without a provisioned client id and staging endpoint the handoff is
        // local: the object already moved to this machine's queue above.
        drop(ijtiyaz);
        None
    };

    Ok(IrsalHie { ruqaa, murajaa, rabt_talab_damj, basma })
}

/// Every submission of mine, newest first, each with its trail.
///
/// # Errors
///
/// Whatever the draft store or the review log raise.
#[tauri::command]
#[specta::specta]
pub fn musahamati(masarat: tauri::State<'_, Masarat>) -> Result<Vec<MusahamaHie>, Khata> {
    let kul = musawwadat_kul(&masarat)?;
    let sijill = sijill_malik(&masarat)?;
    let mut natija: Vec<MusahamaHie> = kul
        .iter()
        .map(|musawwada| MusahamaHie {
            ruqaa: musawwada.id().to_string(),
            murajaa: musawwada.murajaa().qeema(),
            muarrif: musawwada.luba().to_string(),
            ism_luba: musawwada.wasf().ism_luba.clone(),
            unwan: musawwada.wasf().unwan.clone(),
            hala_arabi: musawwada.hala().wasf_arabi().to_owned(),
            nihaiya: musawwada.hala().nihaiya(),
            waqt: musawwada.ansha().to_owned(),
            sijill: sijill
                .li_ruqaa(musawwada.id())
                .into_iter()
                .map(satr_sijill_hie)
                .collect(),
        })
        .collect();
    natija.sort_by(|awwal, thani| thani.waqt.cmp(&awwal.waqt));
    Ok(natija)
}

/// The demand-sorted requests board.
///
/// # Errors
///
/// [`KhataWarshaAmr::MalafTalif`] when the board file exists and does not read.
#[tauri::command]
#[specta::specta]
pub fn lawhat_talabat(
    masarat: tauri::State<'_, Masarat>,
) -> Result<LawhatTalabatHie, Khata> {
    let lawha: LawhatTalabat = crate::warsha_awamir::iqra_janibi(&masar_talabat(&masarat))?;
    Ok(LawhatTalabatHie {
        sufuf: lawha
            .hasab_talab()
            .into_iter()
            .map(|talabat| TalabLubaHie {
                muarrif: talabat.luba.to_string(),
                ism_luba: talabat.ism_luba.clone(),
                adad: crate::warsha_awamir::adad_u32(talabat.talab()),
                akhir_waqt: talabat.talabat.values().map(|talab| talab.waqt.clone()).max(),
            })
            .collect(),
    })
}

/// Files a translation request for one game; answers its new count.
///
/// # Errors
///
/// Whatever the board file or the library raise.
#[tauri::command]
#[specta::specta]
pub fn utlub_tarjama(
    muarrif: String,
    mulahaza: Option<String>,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<u32, Khata> {
    let _harasa = qufl.0.blocking_lock();
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    // A request is somebody asking the community to spend evenings translating
    // this game. For a game the publisher already ships Arabic for, that work
    // would be thrown away at the submission gate, which refuses it — so the
    // refusal belongs here too, before the asking, rather than only in the
    // screen that usually does the asking. The board is reachable from more
    // than one place.
    if let Some(hukm) = crate::luba_awamir::hukm_mukhazzan(id)
        && hukm.hala().yatakallam_arabi()
    {
        return Err(Khata::from(KhataTaqdeemAmr::LughaRasmiyaMawjuda { ism: luba.ism }));
    }
    let (musahim, _ism, _itimad) = hawiyati(&masarat)?;
    let mut lawha: LawhatTalabat =
        crate::warsha_awamir::iqra_janibi(&masar_talabat(&masarat))?;
    let _ = lawha.utlub(TalabTarjama::jadeed(
        id,
        luba.ism,
        musahim,
        waqt_alaan(),
        mulahaza,
    ));
    crate::warsha_awamir::uktub_janibi(&masar_talabat(&masarat), &lawha)?;
    Ok(crate::warsha_awamir::adad_u32(lawha.talab(id)))
}

/// How many open requests one game has.
///
/// # Errors
///
/// [`KhataWarshaAmr::MalafTalif`] when the board file exists and does not read.
#[tauri::command]
#[specta::specta]
pub fn adad_talabat(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
) -> Result<u32, Khata> {
    let id = huwiya(muarrif)?;
    let lawha: LawhatTalabat = crate::warsha_awamir::iqra_janibi(&masar_talabat(&masarat))?;
    Ok(crate::warsha_awamir::adad_u32(lawha.talab(id)))
}

fn aila_min_bayan(musawwada: &Musawwada) -> taarib_mustalahat::muharrik::AilatMuharrik {
    musawwada
        .bayan()
        .get("muharrik")
        .and_then(|muharrik| muharrik.get("aila"))
        .cloned()
        .and_then(|qeema| serde_json::from_value(qeema).ok())
        .unwrap_or(taarib_mustalahat::muharrik::AilatMuharrik::Majhul)
}

fn halat_fuhus_hie(hala: HalatFuhus) -> (&'static str, String) {
    match hala {
        HalatFuhus::Najahat => ("najahat", hala.wasf_arabi()),
        HalatFuhus::Akhfaqat { .. } => ("akhfaqat", hala.wasf_arabi()),
        HalatFuhus::LamTujra => ("lam_tujra", hala.wasf_arabi()),
    }
}

/// The review queue: every submission waiting locally, longest wait first.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key, and whatever the
/// draft store raises.
#[tauri::command]
#[specta::specta]
pub fn tabur_muraja(masarat: tauri::State<'_, Masarat>) -> Result<TaburHie, Khata> {
    use taarib_taqdeem::tabur::{MurashshihTabur, TarteebTabur, tabur};

    let (salahiya, _khass) = salahiyat_malik()?;
    let alaan = waqt_alaan();
    let kul = musawwadat_kul(&masarat)?;

    let mut sufuf: Vec<MudkhalTabur> = Vec::new();
    for musawwada in &kul {
        if !musawwada.hala().fi_intizar_almalik() {
            continue;
        }
        let fuhus = match qaimat_lil(&masarat, musawwada) {
            Ok(qaima) if qaima.rasiba().is_empty() => HalatFuhus::Najahat,
            Ok(qaima) => HalatFuhus::Akhfaqat {
                adad: crate::warsha_awamir::adad_u32(qaima.rasiba().len()),
            },
            Err(_) => HalatFuhus::LamTujra,
        };
        let taghtiya = musawwada.taghtiya();
        let mut madkhal = MudkhalTabur {
            ruqaa: musawwada.id(),
            murajaa: musawwada.murajaa(),
            unwan: musawwada.wasf().unwan.clone(),
            luba: musawwada.luba(),
            ism_luba: musawwada.wasf().ism_luba.clone(),
            aila: aila_min_bayan(musawwada),
            musahim: musawwada.musahim().musahim.clone(),
            ism_musahim: musawwada.musahim().ism.clone(),
            sumaa: sumaa_li(&masarat, &musawwada.musahim().musahim)?,
            tareeqa: musawwada.tareeqa(),
            taghtiya,
            waqt_taqdeem: musawwada
                .hala()
                .waqt()
                .unwrap_or_else(|| musawwada.ansha())
                .to_owned(),
            umr_daqaiq: 0,
            fuhus,
        };
        let _ = madkhal.jaddid_umr(&alaan);
        sufuf.push(madkhal);
    }

    let murattaba = tabur(&salahiya, &sufuf, &MurashshihTabur::maftuh(), TarteebTabur::default());
    let ihsaat: IhsaTabur = ihsa(&salahiya, &murattaba);

    Ok(TaburHie {
        sufuf: murattaba
            .into_iter()
            .map(|madkhal| {
                let taghtiya = madkhal.taghtiya;
                let (fuhus, fuhus_arabi) = halat_fuhus_hie(madkhal.fuhus);
                MudkhalTaburHie {
                    ruqaa: madkhal.ruqaa.to_string(),
                    murajaa: madkhal.murajaa.qeema(),
                    unwan: madkhal.unwan.clone(),
                    muarrif: madkhal.luba.to_string(),
                    ism_luba: madkhal.ism_luba.clone(),
                    musahim: madkhal.musahim.mukhtasar(),
                    ism_musahim: madkhal.ism_musahim.clone(),
                    tareeqa_arabi: madkhal.tareeqa.wasf_arabi().to_owned(),
                    taghtiya_nisba: nisbat_taghtiya(taghtiya.majmu, taghtiya.mutarjam),
                    waqt: madkhal.waqt_taqdeem.clone(),
                    umr_arabi: madkhal.umr_arabi(),
                    fuhus: fuhus.to_owned(),
                    fuhus_arabi,
                }
            })
            .collect(),
        majmu: crate::warsha_awamir::adad_u32(ihsaat.majmu),
        akhfaqat: crate::warsha_awamir::adad_u32(ihsaat.akhfaqat),
        lam_tujra: crate::warsha_awamir::adad_u32(ihsaat.lam_tujra),
        najahat: crate::warsha_awamir::adad_u32(ihsaat.najahat),
        aqsa_umr_daqaiq: u32::try_from(ihsaat.aqsa_umr_daqaiq).unwrap_or(u32::MAX),
    })
}

/// One submission's whole detail, opening it when it was merely waiting.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// [`KhataTaqdeemAmr::TaqdeemGhayrMawjud`] for an unknown lineage, and whatever
/// the stores raise.
#[tauri::command]
#[specta::specta]
pub fn tafasil_muraja(
    ruqaa: String,
    masarat: tauri::State<'_, Masarat>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<TafasilMurajaHie, Khata> {
    use taarib_taqdeem::musawwada::HalatTaqdeem;

    let _harasa = qufl.0.blocking_lock();
    let (_salahiya, _khass) = salahiyat_malik()?;
    let mut musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    if matches!(musawwada.hala(), HalatTaqdeem::Muqaddama { .. }) {
        musawwada = musawwada.futihat(&waqt_alaan()).map_err(|marfud| {
            Khata::from(KhataTaqdeemAmr::IntiqalMarfudAmr {
                min: marfud.min().to_owned(),
                ila: marfud.ila().to_owned(),
            })
        })?;
        ihfaz_musawwada(&masarat, &musawwada)?;
    }

    let qaima = qaimat_lil(&masarat, &musawwada)
        .map_or(QaimatFahsHie { sutur: Vec::new(), jahiza: false }, |qaima| qaimat_hie(&qaima));

    let sufuf: Vec<SafHuzmaHie> = match nusus_mashru(&masarat, musawwada.luba()) {
        Ok((_mashru, madakhil)) => madakhil
            .iter()
            .map(|mudkhal| SafHuzmaHie {
                masdar: mudkhal.masdar.clone(),
                hadaf: mudkhal.hadaf.clone().unwrap_or_default(),
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    let tajawuz: Vec<TajawuzHie> = musawwada
        .tajawuz()
        .tajawuzat
        .iter()
        .map(|madkhal| TajawuzHie {
            nass: madkhal.muqtatas.clone(),
            ard: f64::from(madkhal.ard_maqis),
            mutah: f64::from(madkhal.ard_mutah),
            hajm: f64::from(madkhal.hajm),
        })
        .collect();

    let khuyut = taaliqat_muraja(&masarat)?;
    let taaliqat = khuyut
        .get(&musawwada.id().to_string())
        .map(|khayt| khayt.iter().map(taaliq_hie).collect())
        .unwrap_or_default();

    let sijill = sijill_malik(&masarat)?;
    let quyud = sijill.li_ruqaa(musawwada.id()).into_iter().map(satr_sijill_hie).collect();

    Ok(TafasilMurajaHie {
        sumaa: sumaa_hie(&sumaa_li(&masarat, &musawwada.musahim().musahim)?),
        sufuf,
        tajawuz,
        taaliqat,
        sijill: quyud,
        // Only the latest revision's package is on this machine; there is
        // nothing older to diff against without the registry.
        farq_sabiq: None,
        musawwada: musawwada_hie(&musawwada, qaima),
    })
}

/// Files an owner comment on a submission; answers the whole thread.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// [`KhataTaqdeemAmr::SababFarigh`] for an empty body, and whatever the stores
/// raise.
#[tauri::command]
#[specta::specta]
pub fn allaq_muraja(
    ruqaa: String,
    nass: Option<String>,
    matn: String,
    masarat: tauri::State<'_, Masarat>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<Vec<TaaliqWarshaHie>, Khata> {
    let _harasa = qufl.0.blocking_lock();
    let (salahiya, _khass) = salahiyat_malik()?;
    let (musahim, _ism, _itimad) = hawiyati(&masarat)?;
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    let Some(badan) = NassTaaliq::jadeed(matn) else {
        return Err(Khata::from(KhataTaqdeemAmr::SababFarigh));
    };
    let mawdi: Option<NassId> = match nass {
        Some(khaam) => Some(
            serde_json::from_value(serde_json::Value::String(khaam.clone())).map_err(|_| {
                Khata::from(KhataTaqdeemAmr::TaqdeemGhayrMawjud { ruqaa: khaam })
            })?,
        ),
        None => None,
    };

    let waqt = waqt_alaan();
    let mut khuyut = taaliqat_muraja(&masarat)?;
    let khayt = khuyut.entry(musawwada.id().to_string()).or_default();
    let id_taaliq = TaaliqId::jadeed(u64::try_from(khayt.len()).unwrap_or(u64::MAX));
    khayt.push(Taaliq::min_malik(
        id_taaliq,
        &salahiya,
        musahim.clone(),
        mawdi,
        badan,
        musawwada.murajaa(),
        waqt.clone(),
    ));
    let natija: Vec<TaaliqWarshaHie> = khayt.iter().map(taaliq_hie).collect();
    crate::warsha_awamir::uktub_janibi(&masar_taaliqat_muraja(&masarat), &khuyut)?;

    let marji = MarjiMuraja::jadeed(musawwada.id(), musawwada.murajaa(), musahim);
    let qayd = allaq(&salahiya, &marji, id_taaliq, waqt);
    let mut sijill = sijill_malik(&masarat)?;
    sijill.alhiq(qayd);
    uktub_sijill_malik(&masarat, &sijill)?;

    Ok(natija)
}

/// Requests changes, rejects, or revokes — one written reason, one record.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// [`KhataTaqdeemAmr::SababFarigh`] for an empty reason, the transition's own
/// refusal, and whatever the stores raise.
#[tauri::command]
#[specta::specta]
pub fn qarrir_muraja(
    ruqaa: String,
    ijra: String,
    sabab: String,
    masarat: tauri::State<'_, Masarat>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<MusawwadaHie, Khata> {
    let _harasa = qufl.0.blocking_lock();
    let (salahiya, _khass) = salahiyat_malik()?;
    let (musahim, _ism, _itimad) = hawiyati(&masarat)?;
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    let waqt = waqt_alaan();
    let marji = MarjiMuraja::jadeed(musawwada.id(), musawwada.murajaa(), musahim);
    let intiqal = |marfud: taarib_taqdeem::musawwada::IntiqalMarfud| {
        Khata::from(KhataTaqdeemAmr::IntiqalMarfudAmr {
            min: marfud.min().to_owned(),
            ila: marfud.ila().to_owned(),
        })
    };

    let (qayd, baada) = match ijra.as_str() {
        "talab_taadil" => {
            let Some(mulakhkhas) = MulakhkhasTaadil::jadeed(sabab.clone()) else {
                return Err(Khata::from(KhataTaqdeemAmr::SababFarigh));
            };
            let qayd = utlub_taadil(&salahiya, &marji, mulakhkhas, Vec::new(), waqt.clone());
            let baada =
                musawwada.tulib_taadil(&waqt, &sabab, Vec::new()).map_err(intiqal)?;
            haddith_sumaa(&masarat, &baada.musahim().musahim, |sumaa| {
                sumaa.tulib_taadil = sumaa.tulib_taadil.saturating_add(1);
            })?;
            (qayd, baada)
        }
        "rafd" => {
            let maktub = SababRafd::jadeed(sabab.clone()).map_err(Khata::from)?;
            let qayd = urfud(&salahiya, &marji, maktub, waqt.clone());
            let baada = musawwada.rufidat(&waqt, &sabab).map_err(intiqal)?;
            haddith_sumaa(&masarat, &baada.musahim().musahim, |sumaa| {
                sumaa.marfuda = sumaa.marfuda.saturating_add(1);
            })?;
            (qayd, baada)
        }
        "sahb" => {
            let sabab_nass = sabab.clone();
            let maktub = SababRafd::jadeed(sabab).map_err(Khata::from)?;
            let qayd = ishab(&salahiya, &marji, maktub, waqt.clone());
            let mut fahras: Vec<MulakhkhasRuqaa> =
                crate::warsha_awamir::iqra_janibi(&masar_fahras_manshurat(&masarat))?;
            fahras.retain(|mulakhkhas| {
                !(mulakhkhas.id == musawwada.id()
                    && mulakhkhas.murajaa == musawwada.murajaa())
            });
            crate::warsha_awamir::uktub_janibi(&masar_fahras_manshurat(&masarat), &fahras)?;
            haddith_sumaa(&masarat, &musawwada.musahim().musahim, |sumaa| {
                sumaa.masbuba = sumaa.masbuba.saturating_add(1);
            })?;
            let baada =
                musawwada.suhibat_min_almalik(&waqt, &sabab_nass).map_err(intiqal)?;
            (qayd, baada)
        }
        _ => {
            return Err(Khata::from(KhataTaqdeemAmr::IjraMajhul { ijra }));
        }
    };

    let mut sijill = sijill_malik(&masarat)?;
    sijill.alhiq(qayd);
    uktub_sijill_malik(&masarat, &sijill)?;
    ihfaz_musawwada(&masarat, &baada)?;

    let qaima = qaimat_lil(&masarat, &baada)
        .map_or(QaimatFahsHie { sutur: Vec::new(), jahiza: false }, |qaima| qaimat_hie(&qaima));
    Ok(musawwada_hie(&baada, qaima))
}

/// Approves a submission, signs it with the owner key, and publishes it into
/// this machine's release area and listing index.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// [`KhataTaqdeemAmr::HuzmaMafquda`] when the package file is gone, the
/// transition's own refusal, and whatever signing or the stores raise.
#[tauri::command]
#[specta::specta]
pub fn iaatimad_muraja(
    ruqaa: String,
    masarat: tauri::State<'_, Masarat>,
    qufl: tauri::State<'_, QuflTaqdeem>,
) -> Result<MusawwadaHie, Khata> {
    use taarib_taqdeem::musawwada::HalatTaqdeem;

    let _harasa = qufl.0.blocking_lock();
    let (salahiya, khass) = salahiyat_malik()?;
    let (musahim, _ism, _itimad) = hawiyati(&masarat)?;
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    let waqt = waqt_alaan();
    let lahza = i64::try_from(crate::warsha_awamir::lahza_alaan()).unwrap_or(0);

    let bayt = std::fs::read(musawwada.masar_huzma()).map_err(|_| {
        Khata::from(KhataTaqdeemAmr::HuzmaMafquda {
            masar: musawwada.masar_huzma().to_path_buf(),
        })
    })?;

    let marji = MarjiMuraja::jadeed(musawwada.id(), musawwada.murajaa(), musahim.clone());
    let (_qarar, qayd) = iaatimad(&salahiya, &marji, waqt.clone());

    let makhtuma = waqqi(
        &salahiya,
        &khass,
        musahim,
        waqt.clone(),
        lahza,
        musawwada.id(),
        musawwada.murajaa(),
        musawwada.luba(),
        BaytMuhadhah::min_bayt(&bayt),
    )
    .map_err(Khata::from)?;

    let mujallad = mujallad_manshurat(&masarat);
    taarib_usus::masarat::insha_mujallad(&mujallad)?;
    let basmat_muhtawa = Basma::min_bayt(*blake3::hash(makhtuma.bayt()).as_bytes());
    let masar_nashr = mujallad.join(format!(
        "{}-r{}-{}.ruqaa",
        musawwada.id(),
        musawwada.murajaa().qeema(),
        basmat_muhtawa.mukhtasara()
    ));
    kitaba_dharra(&masar_nashr, makhtuma.bayt())?;

    let mut fahras: Vec<MulakhkhasRuqaa> =
        crate::warsha_awamir::iqra_janibi(&masar_fahras_manshurat(&masarat))?;
    let adad = musawwada.adad_nusus();
    fahras.push(MulakhkhasRuqaa {
        id: musawwada.id(),
        murajaa: musawwada.murajaa(),
        unwan: musawwada.wasf().unwan.clone(),
        musahim: musawwada.musahim().musahim.clone(),
        ism_musahim: musawwada.musahim().ism.clone(),
        taghtiya: musawwada.taghtiya(),
        adad_nusus: adad.majmu,
        hajm: u64::try_from(makhtuma.bayt().len()).unwrap_or(u64::MAX),
        bina_manassa: musawwada.irtibat().manassat.clone(),
        basmat: musawwada.irtibat().basmat.clone(),
        aila: aila_min_bayan(&musawwada),
        khalfiya: khalfiya_min_bayan(&musawwada),
        tabaqa: tabaqa_min_bayan(&musawwada),
        tareeqa: musawwada.tareeqa(),
        rukhsa: musawwada.wasf().rukhsa.clone(),
        taqyeem: None,
        adad_taqyeemat: 0,
        waqt_nashr: waqt.clone(),
        basmat_muhtawa,
        rabt: masar_nashr.to_string_lossy().into_owned(),
        rabt_mira: None,
    });
    crate::warsha_awamir::uktub_janibi(&masar_fahras_manshurat(&masarat), &fahras)?;

    let mut lawha: LawhatTalabat =
        crate::warsha_awamir::iqra_janibi(&masar_talabat(&masarat))?;
    let _ = lawha.ughliq(musawwada.luba(), musawwada.id());
    crate::warsha_awamir::uktub_janibi(&masar_talabat(&masarat), &lawha)?;

    let intiqal = |marfud: taarib_taqdeem::musawwada::IntiqalMarfud| {
        Khata::from(KhataTaqdeemAmr::IntiqalMarfudAmr {
            min: marfud.min().to_owned(),
            ila: marfud.ila().to_owned(),
        })
    };
    let mowafaqa = musawwada.wufiq_alayha(&waqt).map_err(intiqal)?;
    let manshura = mowafaqa.nushirat(&waqt).map_err(intiqal)?;
    ihfaz_musawwada(&masarat, &manshura)?;

    let bila_taadil = !manshura
        .tareekh()
        .iter()
        .any(|qayd_intiqal| matches!(qayd_intiqal.ila, HalatTaqdeem::MatlubTaadil { .. }));
    haddith_sumaa(&masarat, &manshura.musahim().musahim, |sumaa| {
        sumaa.ruqaa_manshura = sumaa.ruqaa_manshura.saturating_add(1);
        if bila_taadil {
            sumaa.qubila_bila_taadil = sumaa.qubila_bila_taadil.saturating_add(1);
        }
    })?;

    let mut sijill = sijill_malik(&masarat)?;
    sijill.alhiq(qayd);
    uktub_sijill_malik(&masarat, &sijill)?;

    let qaima = qaimat_lil(&masarat, &manshura)
        .map_or(QaimatFahsHie { sutur: Vec::new(), jahiza: false }, |qaima| qaimat_hie(&qaima));
    Ok(musawwada_hie(&manshura, qaima))
}

fn khalfiya_min_bayan(
    musawwada: &Musawwada,
) -> taarib_mustalahat::muharrik::KhalfiyaBarmajiya {
    musawwada
        .bayan()
        .get("muharrik")
        .and_then(|muharrik| muharrik.get("khalfiya"))
        .cloned()
        .and_then(|qeema| serde_json::from_value(qeema).ok())
        .unwrap_or(taarib_mustalahat::muharrik::KhalfiyaBarmajiya::Majhula)
}

fn tabaqa_min_bayan(musawwada: &Musawwada) -> taarib_mustalahat::muharrik::Tabaqa {
    musawwada
        .bayan()
        .get("muharrik")
        .and_then(|muharrik| muharrik.get("tabaqa"))
        .cloned()
        .and_then(|qeema| serde_json::from_value(qeema).ok())
        .unwrap_or(taarib_mustalahat::muharrik::Tabaqa::TarjamaFawqiya)
}

/// The whole audit log, newest first.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key.
#[tauri::command]
#[specta::specta]
pub fn sijill_muraja_kull(
    masarat: tauri::State<'_, Masarat>,
) -> Result<Vec<SatrSijillHie>, Khata> {
    let (_salahiya, _khass) = salahiyat_malik()?;
    let sijill = sijill_malik(&masarat)?;
    let mut quyud: Vec<SatrSijillHie> = sijill.quyud().iter().map(satr_sijill_hie).collect();
    quyud.reverse();
    Ok(quyud)
}

/// The bundled revocation list, signed by the owner key at sequence 1.
const QAIMAT_SAHB_SANDOOQ: &[u8] = include_bytes!("../../../../assets/qaimat_sahb.json");

/// Copies a game tree into the sandbox, once; a populated sandbox is reused.
fn insakh_lil_sandooq(min: &Path, ila: &Path) -> Natija<u64> {
    let mamlu = std::fs::read_dir(ila).is_ok_and(|mut qaima| qaima.next().is_some());
    if mamlu {
        return Ok(0);
    }
    let mut adad = 0_u64;
    let mut rukam = vec![min.to_path_buf()];
    while let Some(mujallad) = rukam.pop() {
        let qaima = std::fs::read_dir(&mujallad).map_err(|sabab| {
            Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() })
        })?;
        for dakhla in qaima.flatten() {
            let masar = dakhla.path();
            let naw = dakhla.file_type().map_err(|sabab| {
                Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() })
            })?;
            // A link could reach outside the game tree; the copy refuses to follow.
            if naw.is_symlink() {
                continue;
            }
            let nisbi = masar.strip_prefix(min).map_err(|sabab| {
                Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() })
            })?;
            let hadaf = ila.join(nisbi);
            if naw.is_dir() {
                std::fs::create_dir_all(&hadaf).map_err(|sabab| {
                    Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() })
                })?;
                rukam.push(masar);
            } else {
                if let Some(waled) = hadaf.parent() {
                    std::fs::create_dir_all(waled).map_err(|sabab| {
                        Khata::from(KhataTaqdeemAmr::SandooqNaqis {
                            sabab: sabab.to_string(),
                        })
                    })?;
                }
                // A hard link makes the copy differential; a filesystem that
                // refuses links gets the byte copy instead.
                if std::fs::hard_link(&masar, &hadaf).is_err() {
                    let _ = std::fs::copy(&masar, &hadaf).map_err(|sabab| {
                        Khata::from(KhataTaqdeemAmr::SandooqNaqis {
                            sabab: sabab.to_string(),
                        })
                    })?;
                }
                adad = adad.saturating_add(1);
            }
        }
    }
    Ok(adad)
}

fn tanfidhi_sandooq(luba: &Luba, jidhr_sandooq: &Path) -> Natija<PathBuf> {
    let Some(tanfidhi) = &luba.tanfidhi else {
        return Err(Khata::from(KhataTaqdeemAmr::SandooqNaqis {
            sabab: "the game's executable is not recorded".to_owned(),
        }));
    };
    let nisbi = tanfidhi.strip_prefix(&luba.jidhr).map_err(|_| {
        Khata::from(KhataTaqdeemAmr::SandooqNaqis {
            sabab: "the executable is recorded outside the game directory".to_owned(),
        })
    })?;
    Ok(jidhr_sandooq.join(nisbi))
}

/// Copies the game into the sandbox and installs the submission there, for real.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key, and whichever gate of
/// the real install pipeline refuses, in its own words — including
/// [`crate::luba_awamir::KhataLuba::JidhrSteamMajhul`] for a Steam game whose
/// Steam root cannot be found, which is the one case where the anti-cheat gate
/// would otherwise reach a verdict with half its evidence unread.
#[tauri::command]
#[specta::specta]
#[expect(clippy::too_many_lines, reason = "the real install recipe, sandbox-rooted")]
pub fn sandooq_thabbit(
    ruqaa: String,
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<TaqreerSandooqHie, Khata> {
    use taarib_makhzan::sijillat::SijillBina;
    use taarib_mustawda::tathbeet_bilnaqra::{TalabNaqra, thabbit_bilnaqra};
    use taarib_tathbeet::masar_tathbeet::WadaMuhtawa;
    use taarib_tathbeet::mawdi::WajhatLuba;

    let (salahiya, _khass) = salahiyat_malik()?;
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let hali = idadat.hali();
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;

    let beea = BeeatSandooq::hayyi(&salahiya, &masarat.sandooq(), musawwada.id())
        .map_err(Khata::from)?;
    let manqula = insakh_lil_sandooq(&luba.jidhr, beea.jidhr_luba())?;
    let _ = manqula;

    let malaf_huzma = musawwada.masar_huzma().to_path_buf();
    let maftuh = qari::MalafRuqaa::iftah(&malaf_huzma).map_err(Khata::from)?;
    let bayan = maftuh
        .ruqaa()
        .and_then(|qari_ruqaa| qari_ruqaa.bayan_json())
        .map_err(Khata::from)?;
    let irtibat: IrtibatBina =
        serde_json::from_value(bayan.get("irtibat").cloned().unwrap_or_default()).map_err(
            |sabab| Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() }),
        )?;

    let taqreer = makhzan
        .bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?
        .ok_or_else(|| Khata::from(KhataTaqdeemAmr::MuharrikMajhul { ism: luba.ism.clone() }))?;
    let bina = makhzan
        .bil_qira(|ittisal| SijillBina::jadeed(ittisal).haliya(id))?
        .ok_or_else(|| Khata::from(KhataTaqdeemAmr::MuharrikMajhul { ism: luba.ism.clone() }))?;

    let tanfidhi = tanfidhi_sandooq(&luba, beea.jidhr_luba())?;
    let ism_tanfidhi =
        tanfidhi.file_name().and_then(|s| s.to_str()).unwrap_or_default().to_owned();

    let miftah_aam = taarib_khatm::MiftahAam::min_bayt(&MIRSAT_MALIK.miftah)
        .map_err(|q| Khata::min_tafsir(&q))?;
    let qaima = taarib_aman::qaimat_sahb::QaimatSahb::min_bayt(QAIMAT_SAHB_SANDOOQ, &miftah_aam)
        .map_err(|q| Khata::min_tafsir(&q))?;
    let sijill_iqrar = taarib_aman::iqrar::iqra(
        &masarat.jidhr_bayanat().join(crate::tathbeet_awamir::ISM_MALAF_IQRAR),
    )?;

    let bayt_ruqaa = std::fs::read(&malaf_huzma).map_err(|_| {
        Khata::from(KhataTaqdeemAmr::HuzmaMafquda { masar: malaf_huzma.clone() })
    })?;
    let wajha = WajhatLuba::dakhil_taarib(&format!("{}.ruqaa", musawwada.id()))
        .map_err(|q| Khata::min_tafsir(&q))?;
    let muhtawa = vec![WadaMuhtawa { wajha, bayt: bayt_ruqaa }];

    let Some(masdar) = luba.masadir.first().cloned() else {
        return Err(Khata::from(KhataTaqdeemAmr::SandooqNaqis {
            sabab: "the game has no launcher identity".to_owned(),
        }));
    };
    let tarif = taarib_tathbeet::bayan::TarifLuba {
        luba: id,
        masdar: masdar.clone(),
        ism: luba.ism.clone(),
        jidhr: beea.jidhr_luba().to_path_buf(),
        ruqaa: musawwada.id(),
        murajaa: musawwada.murajaa(),
        basma_bina: Some(bina.basma),
    };
    let luba_muhallala = taarib_tathbeet::tarkib::LubaMuhallala {
        jidhr: beea.jidhr_luba().to_path_buf(),
        masar_tanfidhi: tanfidhi,
        muharrik: taqreer.muharrik.clone(),
        beea: luba.beea.clone(),
        nizam: taarib_usus::manassa::NizamTashghil::hali(),
        masdar,
    };
    let halat_idadat = taarib_tathbeet::tarkib::HalatIdadat {
        khiyarat_tashghil: None,
        tajawuzat_dll: None,
        tahmil_musbaq: None,
        malaf_idadat_manassa: None,
    };
    let jidhr_hajr = masarat.hajr();
    let mukawwinat = masarat.mukawwinat();
    // The same resolution and the same refusal the real install path uses. The
    // copy in the sandbox is not the player's game, but the anti-cheat gate runs
    // over it all the same, and a gate that reached a verdict here with Steam's
    // catalogue unread would be telling a contributor their patch cleared a
    // check it never made.
    let jidhr_steam = crate::luba_awamir::jidhr_steam_lil_fahs(&masarat, &hali, &luba)?;

    let talab = TalabNaqra {
        luba: id,
        tarif: &tarif,
        appid: crate::luba_awamir::appid_steam(&luba),
        jidhr_steam: jidhr_steam.as_deref(),
        malaf_munazzal: &malaf_huzma,
        jidhr_hajr: &jidhr_hajr,
        jidhr_nusakh: beea.jidhr_nusakh(),
        mirsa: &MIRSAT_MALIK,
        qaima: &qaima,
        iqrar: sijill_iqrar.as_ref(),
        iqrar_shabaka: true,
        iqrar_taqribi: true,
        tanfidhi: &ism_tanfidhi,
        muhtawa,
        bina: &bina,
        irtibat: &irtibat,
        huwiya: format!("sandooq-{}@{}", musawwada.id(), musawwada.murajaa()),
    };

    let natija = thabbit_bilnaqra(
        talab,
        |muthabbit| {
            let _ = taarib_tathbeet::tarkib::nashr(
                &luba_muhallala,
                &halat_idadat,
                &taqreer,
                &mukawwinat,
                muthabbit,
            )?;
            Ok(())
        },
        |_marhala| {},
    )
    .map_err(|fashal| {
        Khata::min_tafsir(&crate::tathbeet_awamir::KhataTathbeetAmr::TathbeetFashil {
            arabi: fashal.arabi(),
            injilizi: fashal.injilizi(),
        })
    })?;

    Ok(TaqreerSandooqHie {
        hasila_arabi: format!(
            "ثُبِّتت داخل الصندوق: كُتب {} عنصرًا — {}",
            natija.adad_muhtawa,
            natija.tahaqquq.arabi()
        ),
        salim: natija.tahaqquq.salim(),
        mustaada: 0,
        mutabaqqi: Vec::new(),
    })
}

/// Launches the sandboxed copy's executable.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key, and
/// [`KhataTaqdeemAmr::SandooqNaqis`] when the process will not start.
#[tauri::command]
#[specta::specta]
pub fn sandooq_atliq(
    ruqaa: String,
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<bool, Khata> {
    let (salahiya, _khass) = salahiyat_malik()?;
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    let beea = BeeatSandooq::hayyi(&salahiya, &masarat.sandooq(), musawwada.id())
        .map_err(Khata::from)?;
    let tanfidhi = tanfidhi_sandooq(&luba, beea.jidhr_luba())?;
    let _ = std::process::Command::new(&tanfidhi)
        .current_dir(beea.jidhr_luba())
        .spawn()
        .map_err(|sabab| {
            Khata::from(KhataTaqdeemAmr::SandooqNaqis { sabab: sabab.to_string() })
        })?;
    Ok(true)
}

/// Restores the sandboxed game from its backups and removes the sandbox.
///
/// # Errors
///
/// [`KhataTaqdeemAmr::MalikFaqat`] without the owner key, and whatever the
/// restore or the removal raise.
#[tauri::command]
#[specta::specta]
pub fn sandooq_imsah(
    ruqaa: String,
    masarat: tauri::State<'_, Masarat>,
) -> Result<bool, Khata> {
    let (salahiya, _khass) = salahiyat_malik()?;
    let musawwada = musawwada_bil_ruqaa(&masarat, &ruqaa)?;
    let beea = BeeatSandooq::hayyi(&salahiya, &masarat.sandooq(), musawwada.id())
        .map_err(Khata::from)?;
    let (_mustaada, _mutabaqqi) = beea.istaid().map_err(Khata::from)?;
    beea.imsah().map_err(Khata::from)?;
    Ok(true)
}

/// Failures of the submission surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataTaqdeemAmr {
    /// A console entry point was reached without the owner key.
    #[error("the review console requires the owner key")]
    MalikFaqat,

    /// No submission carries the identity that was named.
    #[error("{ruqaa} is not a submission on this machine")]
    TaqdeemGhayrMawjud {
        /// The identity that was looked up.
        ruqaa: String,
    },

    /// No translation project exists for the game.
    #[error("no translation project exists for {ism}")]
    MashruGhayrMawjud {
        /// The game's identity.
        ism: String,
    },

    /// A submission is already with the owner and cannot be replaced.
    #[error("submission {ruqaa} is with the owner")]
    TaqdeemMuallaq {
        /// The lineage waiting.
        ruqaa: String,
    },

    /// The game has no capability report to compile against.
    #[error("no engine report is recorded for {ism}")]
    MuharrikMajhul {
        /// The game's name.
        ism: String,
    },

    /// No usable Arabic font is available to bundle.
    #[error("no usable Arabic font: {sabab}")]
    KhututNaqisa {
        /// What the font layer said.
        sabab: String,
    },

    /// The licence identifier is not one this build knows.
    #[error("{rukhsa} is not a licence identifier")]
    RukhsaMajhula {
        /// The identifier that was sent.
        rukhsa: String,
    },

    /// The translation-method identifier is not one this build knows.
    #[error("{tareeqa} is not a translation method")]
    TareeqaMajhula {
        /// The identifier that was sent.
        tareeqa: String,
    },

    /// The warning identifier is not one this build knows.
    #[error("{tahdheer} is not a warning this gate raises")]
    TahdheerMajhul {
        /// The identifier that was sent.
        tahdheer: String,
    },

    /// The review-action identifier is not one this console offers.
    #[error("{ijra} is not a review action")]
    IjraMajhul {
        /// The identifier that was sent.
        ijra: String,
    },

    /// A state transition was refused.
    #[error("the submission cannot move from {min} to {ila}")]
    IntiqalMarfudAmr {
        /// The state it is in.
        min: String,
        /// The state that was asked for.
        ila: String,
    },

    /// An action that requires a written reason arrived without one.
    #[error("a written reason is required and was empty")]
    SababFarigh,

    /// The sandbox could not be prepared or driven.
    #[error("the sandbox failed: {sabab}")]
    SandooqNaqis {
        /// What failed.
        sabab: String,
    },

    /// The submission's package file is no longer on disk.
    #[error("the package at {} is gone", masar.display())]
    HuzmaMafquda {
        /// Where it was recorded.
        masar: PathBuf,
    },

    /// The forge transport is not provisioned; the named piece is missing.
    #[error("the forge transport is not provisioned: {naqis} is missing")]
    IrsalGhayrMuhayya {
        /// Which settings field is absent.
        naqis: &'static str,
    },

    /// The registry address is not a repository this transport understands.
    #[error("{rasmi} is not a github.com owner/repository address")]
    MustawdaGhayrMafhum {
        /// The address as settings carry it.
        rasmi: String,
    },

    /// No device authorization is in flight and no token is stored.
    #[error("no device authorization is in flight")]
    TawthiqNaqis,

    /// The build wrote no pull-request material beside the package.
    #[error("the submission material beside the package is missing")]
    TalabNaqis,

    /// A request was filed for a game whose publisher already ships Arabic.
    #[error("{ism} already ships official Arabic")]
    LughaRasmiyaMawjuda {
        /// The game's name, as its launcher gives it.
        ism: String,
    },
}

impl Tafsir for KhataTaqdeemAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MalikFaqat => 60,
                    Self::TaqdeemGhayrMawjud { .. } => 61,
                    Self::MashruGhayrMawjud { .. } => 62,
                    Self::TaqdeemMuallaq { .. } => 63,
                    Self::MuharrikMajhul { .. } => 64,
                    Self::KhututNaqisa { .. } => 65,
                    Self::RukhsaMajhula { .. } => 66,
                    Self::TahdheerMajhul { .. } => 67,
                    Self::IntiqalMarfudAmr { .. } => 68,
                    Self::SababFarigh => 69,
                    Self::SandooqNaqis { .. } => 70,
                    Self::HuzmaMafquda { .. } => 71,
                    Self::IjraMajhul { .. } => 72,
                    Self::TareeqaMajhula { .. } => 73,
                    Self::IrsalGhayrMuhayya { .. } => 74,
                    Self::MustawdaGhayrMafhum { .. } => 75,
                    Self::TawthiqNaqis => 76,
                    Self::TalabNaqis => 77,
                    Self::LughaRasmiyaMawjuda { .. } => 78,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            Self::MalikFaqat
            | Self::KhututNaqisa { .. }
            | Self::SandooqNaqis { .. }
            | Self::LughaRasmiyaMawjuda { .. }
            | Self::HuzmaMafquda { .. } => Khutura::Khatar,
            Self::TaqdeemGhayrMawjud { .. }
            | Self::TaqdeemMuallaq { .. }
            | Self::RukhsaMajhula { .. }
            | Self::TareeqaMajhula { .. }
            | Self::TahdheerMajhul { .. }
            | Self::IjraMajhul { .. }
            | Self::IntiqalMarfudAmr { .. }
            | Self::SababFarigh
            | Self::MashruGhayrMawjud { .. }
            | Self::MuharrikMajhul { .. }
            | Self::IrsalGhayrMuhayya { .. }
            | Self::MustawdaGhayrMafhum { .. }
            | Self::TawthiqNaqis
            | Self::TalabNaqis => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MalikFaqat => {
                "هذه الشاشة تتطلب مفتاح المالك، وهذه الجلسة لا تحمله.".to_owned()
            }
            Self::TaqdeemGhayrMawjud { .. } => {
                "لا تقديم بهذا المعرّف على هذا الجهاز.".to_owned()
            }
            Self::MashruGhayrMawjud { .. } => {
                "لا مشروع ترجمة لهذه اللعبة بعد. ابدأ الترجمة من شاشة اللعبة أولًا."
                    .to_owned()
            }
            Self::TaqdeemMuallaq { .. } => {
                "تقديمك السابق ما زال عند المالك؛ انتظر رده أو اسحب التقديم قبل \
                 تجهيز نسخة جديدة."
                    .to_owned()
            }
            Self::MuharrikMajhul { ism } => format!(
                "لا تقرير محرّك مسجّلًا للعبة {ism}. افتح شاشة اللعبة ليُفحص محرّكها ثم \
                 عد."
            ),
            Self::KhututNaqisa { .. } => {
                "لا خط عربي صالح في مجلد خطوط تعريب، ولا حزمة بلا خط. أضف خطًا من \
                 الإعدادات."
                    .to_owned()
            }
            Self::RukhsaMajhula { rukhsa } => {
                format!("«{rukhsa}» ليست رخصة يعرفها هذا الإصدار.")
            }
            Self::TareeqaMajhula { tareeqa } => {
                format!("«{tareeqa}» ليست طريقة ترجمة يعرفها هذا الإصدار.")
            }
            Self::TahdheerMajhul { tahdheer } => {
                format!("«{tahdheer}» ليس تحذيرًا تُصدره بوابة التقديم.")
            }
            Self::IjraMajhul { ijra } => {
                format!("«{ijra}» ليس إجراء مراجعة تقدّمه هذه الشاشة.")
            }
            Self::IntiqalMarfudAmr { min, ila } => format!(
                "لا يمكن نقل التقديم من حالة «{min}» إلى «{ila}»؛ الحالة الحالية لا \
                 تسمح بذلك."
            ),
            Self::SababFarigh => {
                "هذا الإجراء يتطلب سببًا مكتوبًا، ولم يُكتب شيء.".to_owned()
            }
            Self::SandooqNaqis { sabab } => {
                format!("تعذّر تشغيل صندوق التحقق: {sabab}")
            }
            Self::HuzmaMafquda { masar } => format!(
                "ملف الحزمة لم يعد في {}. أعد تجهيز التقديم لتُبنى الحزمة من جديد.",
                masar.display()
            ),
            Self::IrsalGhayrMuhayya { naqis } => format!(
                "قناة الرفع إلى السجلّ غير مجهّزة بعد: الحقل {naqis} فارغ في الإعدادات. بقي \
                 تقديمك مسجّلًا محليًا، ويُرفع تلقائيًا متى جهّز مشغّل السجلّ القناة."
            ),
            Self::MustawdaGhayrMafhum { rasmi } => format!(
                "عنوان السجلّ «{rasmi}» ليس بصيغة github.com/مالك/مستودع التي يفهمها النقل. \
                 صحّح العنوان في الإعدادات."
            ),
            Self::TawthiqNaqis => {
                "لا تفويض جهاز جاريًا ولا رمز وصول محفوظًا. ابدأ تفويض الجهاز أولًا ثم أعد \
                 التسليم."
                    .to_owned()
            }
            Self::TalabNaqis => {
                "مادة طلب الدمج غير موجودة بجانب الحزمة؛ بُنيت الحزمة بإصدار أقدم. أعد تجهيز \
                 التقديم ثم سلّم."
                    .to_owned()
            }
            Self::LughaRasmiyaMawjuda { ism } => {
                format!(
                    "{ism} تصدر بعربية رسمية من ناشرها، فلا يُفتح لها طلب ترجمة. الطلب دعوة \
                     لمتطوّعين ينفقون أمسياتهم على عمل ترجمه محترفون بالفعل."
                )
            }
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MalikFaqat => {
                "This screen requires the owner key, and this session does not hold it."
                    .to_owned()
            }
            Self::TaqdeemGhayrMawjud { ruqaa } => {
                format!("{ruqaa} is not a submission on this machine.")
            }
            Self::MashruGhayrMawjud { ism } => format!(
                "No translation project exists for {ism} yet. Start translating from the \
                 game screen first."
            ),
            Self::TaqdeemMuallaq { ruqaa } => format!(
                "Submission {ruqaa} is still with the owner; wait for their answer or \
                 withdraw it before preparing a new build."
            ),
            Self::MuharrikMajhul { ism } => format!(
                "No engine report is recorded for {ism}. Open the game screen so its \
                 engine is probed, then return."
            ),
            Self::KhututNaqisa { sabab } => format!(
                "No usable Arabic font is available to bundle ({sabab}). Add one in \
                 Settings."
            ),
            Self::RukhsaMajhula { rukhsa } => {
                format!("\"{rukhsa}\" is not a licence identifier this build knows.")
            }
            Self::TareeqaMajhula { tareeqa } => {
                format!("\"{tareeqa}\" is not a translation method this build knows.")
            }
            Self::TahdheerMajhul { tahdheer } => {
                format!("\"{tahdheer}\" is not a warning this gate raises.")
            }
            Self::IjraMajhul { ijra } => {
                format!("\"{ijra}\" is not a review action this console offers.")
            }
            Self::IntiqalMarfudAmr { min, ila } => format!(
                "The submission cannot move from \"{min}\" to \"{ila}\"; its current \
                 state does not allow it."
            ),
            Self::SababFarigh => {
                "This action requires a written reason, and none was written.".to_owned()
            }
            Self::SandooqNaqis { sabab } => format!("The verification sandbox failed: {sabab}"),
            Self::HuzmaMafquda { masar } => format!(
                "The package at {} is gone. Prepare the submission again to rebuild it.",
                masar.display()
            ),
            Self::IrsalGhayrMuhayya { naqis } => format!(
                "The registry upload channel is not provisioned yet: the {naqis} field is \
                 empty in Settings. Your submission stays recorded locally and is \
                 sent the moment the registry operator provisions the channel."
            ),
            Self::MustawdaGhayrMafhum { rasmi } => format!(
                "The registry address \"{rasmi}\" is not the github.com/owner/repository \
                 form this transport understands. Correct it in Settings."
            ),
            Self::TawthiqNaqis => {
                "No device authorization is in flight and no access token is stored. Start \
                 device authorization first, then submit again."
                    .to_owned()
            }
            Self::TalabNaqis => {
                "The pull-request material beside the package is missing; the package was \
                 built by an older build. Prepare the submission again, then submit."
                    .to_owned()
            }
            Self::LughaRasmiyaMawjuda { ism } => {
                format!(
                    "{ism} already ships official Arabic from its publisher, so no translation \
                     request is opened for it. A request asks volunteers to spend their evenings \
                     on work professionals have already done."
                )
            }
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MalikFaqat => Khutwa::ManhSalahiya,
            Self::MashruGhayrMawjud { .. } => Khutwa::FathNusus,
            Self::MuharrikMajhul { .. } => Khutwa::AadaFahsMuharrik,
            Self::KhututNaqisa { .. } => Khutwa::FathIdadat { qism: QismIdadat::Khutut },
            Self::TaqdeemGhayrMawjud { .. }
            | Self::TaqdeemMuallaq { .. }
            | Self::RukhsaMajhula { .. }
            | Self::TareeqaMajhula { .. }
            | Self::TahdheerMajhul { .. }
            | Self::IjraMajhul { .. }
            | Self::IntiqalMarfudAmr { .. }
            | Self::SababFarigh
            | Self::SandooqNaqis { .. }
            | Self::HuzmaMafquda { .. }
            | Self::TawthiqNaqis
            | Self::TalabNaqis => Khutwa::AadaMuhawala,
            Self::LughaRasmiyaMawjuda { .. } => Khutwa::LaShay,
            Self::IrsalGhayrMuhayya { .. } | Self::MustawdaGhayrMafhum { .. } => {
                Khutwa::FathIdadat { qism: QismIdadat::Masadir }
            }
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TaqdeemGhayrMawjud { ruqaa } | Self::TaqdeemMuallaq { ruqaa } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
            }
            Self::MashruGhayrMawjud { ism } | Self::MuharrikMajhul { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::KhututNaqisa { sabab } | Self::SandooqNaqis { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            }
            Self::RukhsaMajhula { rukhsa } => {
                let _ = siyaq.insert("rukhsa".to_owned(), QeemaSiyaq::Nass(rukhsa.clone()));
            }
            Self::TareeqaMajhula { tareeqa } => {
                let _ =
                    siyaq.insert("tareeqa".to_owned(), QeemaSiyaq::Nass(tareeqa.clone()));
            }
            Self::TahdheerMajhul { tahdheer } => {
                let _ =
                    siyaq.insert("tahdheer".to_owned(), QeemaSiyaq::Nass(tahdheer.clone()));
            }
            Self::IjraMajhul { ijra } => {
                let _ = siyaq.insert("ijra".to_owned(), QeemaSiyaq::Nass(ijra.clone()));
            }
            Self::IntiqalMarfudAmr { min, ila } => {
                let _ = siyaq.insert("min".to_owned(), QeemaSiyaq::Nass(min.clone()));
                let _ = siyaq.insert("ila".to_owned(), QeemaSiyaq::Nass(ila.clone()));
            }
            Self::HuzmaMafquda { masar } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
            }
            Self::IrsalGhayrMuhayya { naqis } => {
                let _ = siyaq
                    .insert("naqis".to_owned(), QeemaSiyaq::Nass((*naqis).to_owned()));
            }
            Self::MustawdaGhayrMafhum { rasmi } => {
                let _ = siyaq.insert("rasmi".to_owned(), QeemaSiyaq::Nass(rasmi.clone()));
            }
            Self::MalikFaqat
            | Self::SababFarigh
            | Self::TawthiqNaqis
            | Self::TalabNaqis => {}
            Self::LughaRasmiyaMawjuda { ism } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataTaqdeemAmr);
