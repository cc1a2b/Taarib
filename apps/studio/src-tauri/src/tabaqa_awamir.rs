//! الطبقة — the overlay surface: the capture regions, the reading history, and the disclosure.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_makhzan::wasl::Makhzan;
use taarib_tabaqa::manatiq::{
    MajmuatManatiq, Mintaqa, MuarrifMintaqa, NatijatMintaqa, QaidatTarjama,
};
use taarib_tabaqa::sidq::{BasmatIfsah, NASS_IFSAH_ARABI, mahfuz_salih};
use taarib_tabaqa::sijill_qira::{MadkhalQira, SijillQira};
use taarib_tabaqa::wajiha::MustatilNisbi;
use taarib_tathbeet::bayan::{waqt_alaan, waqt_rfc3339};
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, kitaba_dharra};

use crate::luba_awamir::{huwiya, ijlib_luba};

/// When a region is read, as the interface names it.
///
/// A type rather than a `String` narrowed by hand on the TypeScript side. The
/// three slugs are a closed set the interface switches on, and a `String` in
/// the binding is a `string` in the interface — so the union that made the
/// switch exhaustive had to be written a second time, by hand, with nothing
/// checking it against this list.
///
/// Separate from `taarib_tabaqa::QaidatTarjama`, which is the overlay's own
/// vocabulary and has no business carrying the interface's spelling.
/// [`naql_qaida`] and [`qaida_min_nass`] are the one crossing between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum QaidaHie {
    /// Read again whenever the region's pixels change.
    Taghayyur,
    /// Read on a fixed interval.
    Muaqqit,
    /// Read only when the player asks.
    Yadawi,
}

/// A region's stable identity within one game's set, never reused.
///
/// A named type rather than a bare `u64` because it crosses the boundary in
/// both directions — out in every region row, back in on every edit and delete
/// — and `specta` refuses to export a 64-bit integer without being told what
/// to do with it. Said once here, the answer holds for every use.
///
/// `number`, not `bigint`: an identity is a counter that starts at one, and a
/// player who has created 2^53 capture regions has other problems.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(transparent)]
pub struct RaqmMintaqa(#[specta(type = specta_typescript::Number)] pub u64);

/// One capture region, as the overlay control edits it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MintaqaHie {
    /// The region's stable identity within this game's set, never reused.
    pub raqm: RaqmMintaqa,
    /// What the player called it.
    pub ism: String,
    /// Left edge, normalized zero to one.
    pub yasar: f64,
    /// Top edge, normalized zero to one.
    pub aala: f64,
    /// Width, normalized zero to one.
    pub ard: f64,
    /// Height, normalized zero to one.
    pub irtifa: f64,
    /// When it is read.
    pub qaida: QaidaHie,
    /// A per-region floor on the polling interval, in milliseconds, when one is set.
    pub fasila_milli: Option<u32>,
    /// Whether the scheduler considers it at all.
    pub mumakkana: bool,
}

/// One game's capture regions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ManatiqHie {
    /// The game's identity.
    pub muarrif: String,
    /// The game's display name.
    pub ism_luba: String,
    /// The interval a region with no override is polled at, in milliseconds.
    pub fasila_iftiradiya_milli: u32,
    /// Every region, in the order the scheduler visits them.
    pub manatiq: Vec<MintaqaHie>,
}

/// One reading-history entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MadkhalQiraHie {
    /// The entry's identity within this history.
    pub raqm: RaqmMintaqa,
    /// What the recognizer read, verbatim.
    pub nass: String,
    /// The Arabic, once there is any.
    pub arabi: Option<String>,
    /// Which side produced the Arabic, once something did.
    pub masdar: Option<String>,
    /// How long the line stayed on screen, in milliseconds, as far as the history saw.
    #[specta(type = specta_typescript::Number)]
    pub mudda_milli: u64,
    /// When the line was first recognized, RFC 3339.
    pub waqt: String,
}

/// The reading history for one game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SijillQiraHie {
    /// The requested tail of the history, newest first.
    pub sutur: Vec<MadkhalQiraHie>,
    /// How many entries the file holds in total, torn lines excluded.
    pub adad_kulli: u32,
}

/// Where the graphics-tier disclosure stands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct IfsahHie {
    /// The disclosure text this build ships, in Arabic.
    pub nass_arabi: String,
    /// Whether this exact text has been acknowledged.
    pub muqarr: bool,
    /// When it was acknowledged, RFC 3339, when it has been.
    pub waqt: Option<String>,
}

/// The acknowledgement as it is recorded on disk: the fingerprint and the moment.
#[derive(serde::Serialize, serde::Deserialize)]
struct QaydIfsah {
    basma: u64,
    waqt: String,
}

fn khata_malaf(masar: &Path, sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataTabaqaAmr::MalafTalif {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

/// One region as the overlay panel sends it, without its identity.
///
/// A named parameter rather than eight loose ones. `specta` implements its
/// command trait up to ten arguments and `haddith_mintaqa` needed twelve, so
/// spelling every field individually was not a contract that could be exported
/// at all — and `adif_mintaqa` sat exactly on the limit, one field from the
/// same wall.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct ShaklMintaqa {
    /// What the player called it.
    pub ism: String,
    /// The left edge, normalized to the frame's width.
    pub yasar: f64,
    /// The top edge, normalized to the frame's height.
    pub aala: f64,
    /// The width, normalized.
    pub ard: f64,
    /// The height, normalized.
    pub irtifa: f64,
    /// The reading rule.
    pub qaida: QaidaHie,
    /// The per-region interval override, absent to follow the set's own.
    pub fasila_milli: Option<u32>,
}

/// The directory the overlay tier's per-game files live under.
///
/// Shared with the sharing surface, which harvests the same per-game reading history this screen
/// draws; two spellings of one path is how a harvest reads a file nothing wrote.
pub(crate) fn mujallad_tabaqa(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join("tabaqa")
}

/// The file the disclosure acknowledgement is recorded in.
///
/// The name comes from the overlay crate rather than being spelled again here:
/// the overlay reads this file from inside a game to decide whether it may
/// start, and a rename on one side only would leave it refusing forever.
fn masar_ifsah(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join(taarib_tabaqa::ISM_MALAF_IQRAR)
}

/// The committed interface slug for a rule — the `qaida` union in `awamir.ts`.
const fn naql_qaida(qaida: QaidatTarjama) -> QaidaHie {
    match qaida {
        QaidatTarjama::IndaTaghyeer => QaidaHie::Taghayyur,
        QaidatTarjama::Mustamirra => QaidaHie::Muaqqit,
        QaidatTarjama::IndaTalab => QaidaHie::Yadawi,
    }
}

/// Turns the interface's rule into the overlay's own.
///
/// Infallible now that the wire carries a closed set: an unknown slug can no
/// longer arrive, because a value outside the three is refused by
/// deserialization before any command body runs.
const fn qaida_min_nass(qaida: QaidaHie) -> QaidatTarjama {
    match qaida {
        QaidaHie::Taghayyur => QaidatTarjama::IndaTaghyeer,
        QaidaHie::Muaqqit => QaidatTarjama::Mustamirra,
        QaidaHie::Yadawi => QaidatTarjama::IndaTalab,
    }
}

/// The interface's normalized edges as the drawing type, narrowed once.
const fn mustatil_min_abaad(yasar: f64, aala: f64, ard: f64, irtifa: f64) -> MustatilNisbi {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "normalized region edges narrow from the interface's f64 to the drawing \
                  type's f32 with sub-pixel loss at most; validity is judged afterwards"
    )]
    const fn dayyiq(qeema: f64) -> f32 {
        qeema as f32
    }
    MustatilNisbi {
        yasar: dayyiq(yasar),
        aala: dayyiq(aala),
        ard: dayyiq(ard),
        irtifa: dayyiq(irtifa),
    }
}

fn mintaqa_hie(mintaqa: &Mintaqa) -> MintaqaHie {
    MintaqaHie {
        raqm: RaqmMintaqa(mintaqa.muarrif.raqm()),
        ism: mintaqa.ism.clone(),
        yasar: f64::from(mintaqa.mustatil.yasar),
        aala: f64::from(mintaqa.mustatil.aala),
        ard: f64::from(mintaqa.mustatil.ard),
        irtifa: f64::from(mintaqa.mustatil.irtifa),
        qaida: naql_qaida(mintaqa.qaida),
        fasila_milli: mintaqa.fasila_milli,
        mumakkana: mintaqa.mumakkana,
    }
}

fn manatiq_hie(muarrif: String, majmua: &MajmuatManatiq) -> ManatiqHie {
    ManatiqHie {
        muarrif,
        ism_luba: majmua.ism_luba().to_owned(),
        fasila_iftiradiya_milli: majmua.fasila_iftiradiya_milli(),
        manatiq: majmua.manatiq().iter().map(mintaqa_hie).collect(),
    }
}

/// Loads one game's region set — empty on first visit — and the file it lives in.
fn majmuat_luba(
    masarat: &Masarat,
    makhzan: &Makhzan,
    muarrif: &str,
) -> Natija<(MajmuatManatiq, PathBuf)> {
    let id = huwiya(muarrif.to_owned())?;
    let luba = ijlib_luba(makhzan, id)?;
    let masar = MajmuatManatiq::masar_malaf(&mujallad_tabaqa(masarat), id);
    let majmua = MajmuatManatiq::hammil_aw_jadeeda(&masar, id, &luba.ism).map_err(Khata::from)?;
    Ok((majmua, masar))
}

/// A region-set answer as the command boundary needs it: applied, or the named refusal.
fn tatbeeq(natija: NatijatMintaqa) -> Natija<MuarrifMintaqa> {
    match natija {
        NatijatMintaqa::Tammat(muarrif) => Ok(muarrif),
        NatijatMintaqa::GhayrMawjuda(muarrif) => Err(Khata::from(KhataTabaqaAmr::MintaqaMajhula {
            raqm: muarrif.raqm(),
        })),
        NatijatMintaqa::LaMisaha => Err(Khata::from(KhataTabaqaAmr::MustatilGhayrSalih)),
        radd @ (NatijatMintaqa::IsmFarigh
        | NatijatMintaqa::IsmTaweel { .. }
        | NatijatMintaqa::IsmMakrur { .. }
        | NatijatMintaqa::TajawuzSaqf { .. }) => Err(Khata::from(KhataTabaqaAmr::TadeelMarfud {
            unwan: radd.unwan(),
            wasf: radd.wasf(),
        })),
    }
}

/// One history entry as the screen draws it.
fn madkhal_hie(madkhal: &MadkhalQira) -> MadkhalQiraHie {
    MadkhalQiraHie {
        raqm: RaqmMintaqa(madkhal.muarrif.raqm()),
        nass: madkhal.asl.clone(),
        arabi: madkhal.arabi.clone(),
        masdar: madkhal.masdar.map(|masdar| masdar.ism().to_owned()),
        mudda_milli: madkhal.mudda().saturating_mul(1_000),
        waqt: waqt_rfc3339(i64::try_from(madkhal.lahza).unwrap_or(i64::MAX)),
    }
}

/// Parses a history file's entry lines, skipping the header and any torn line.
pub(crate) fn hallil_sutur(bayt: &[u8]) -> Vec<MadkhalQira> {
    let mut sutur = bayt
        .split(|wahid| *wahid == b'\n')
        .filter(|satr| satr.iter().any(|wahid| !wahid.is_ascii_whitespace()));
    // The first line is the header; the path already binds the file to its game.
    let _ = sutur.next();
    sutur
        .filter_map(|satr| std::str::from_utf8(satr).ok())
        .filter_map(|nass| serde_json::from_str::<MadkhalQira>(nass).ok())
        .collect()
}

/// The capture regions for one game, an empty set for a game that has none yet.
///
/// # Errors
///
/// Whatever the game lookup raises — an identity Taarib never issued, a game no longer in
/// the library — and whatever reading the region file raises: a file that exists and does
/// not read, a schema this build does not know, another game's regions under this path.
#[tauri::command]
#[specta::specta]
pub fn manatiq_luba(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<ManatiqHie, Khata> {
    let (majmua, _masar) = majmuat_luba(&masarat, &makhzan, &muarrif)?;
    Ok(manatiq_hie(muarrif, &majmua))
}

/// Adds a region — enabled, with the given rule — and answers the whole set after.
///
/// # Errors
///
/// [`KhataTabaqaAmr::QaidaMajhula`] for a rule slug the interface does not define,
/// [`KhataTabaqaAmr::MustatilGhayrSalih`] for a rectangle with no usable area,
/// [`KhataTabaqaAmr::TadeelMarfud`] for a name the set refuses or a set already at its
/// ceiling, plus everything [`manatiq_luba`] raises and whatever saving the file raises.
#[tauri::command]
#[specta::specta]
pub fn adif_mintaqa(
    muarrif: String,
    shakl: ShaklMintaqa,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<ManatiqHie, Khata> {
    let qaida_muhallala = qaida_min_nass(shakl.qaida);
    let mustatil = mustatil_min_abaad(shakl.yasar, shakl.aala, shakl.ard, shakl.irtifa);
    let (mut majmua, masar) = majmuat_luba(&masarat, &makhzan, &muarrif)?;

    let jadeeda = tatbeeq(majmua.adif(&shakl.ism, mustatil, qaida_muhallala))?;
    if shakl.fasila_milli.is_some() {
        let _ = tatbeeq(majmua.ghayyir_fasila(jadeeda, shakl.fasila_milli))?;
    }
    majmua.ihfaz(&masar).map_err(Khata::from)?;
    Ok(manatiq_hie(muarrif, &majmua))
}

/// Rewrites one region in full — name, rectangle, rule, interval, enabled — and answers
/// the whole set after. Nothing is persisted unless every part of the edit is accepted.
///
/// # Errors
///
/// [`KhataTabaqaAmr::MintaqaMajhula`] when no region carries the identity,
/// [`KhataTabaqaAmr::QaidaMajhula`], [`KhataTabaqaAmr::MustatilGhayrSalih`],
/// [`KhataTabaqaAmr::TadeelMarfud`] for a name the set refuses, plus everything
/// [`manatiq_luba`] raises and whatever saving the file raises.
#[tauri::command]
#[specta::specta]
pub fn haddith_mintaqa(
    muarrif: String,
    raqm: RaqmMintaqa,
    shakl: ShaklMintaqa,
    mumakkana: bool,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<ManatiqHie, Khata> {
    let qaida_muhallala = qaida_min_nass(shakl.qaida);
    let mustatil = mustatil_min_abaad(shakl.yasar, shakl.aala, shakl.ard, shakl.irtifa);
    let (mut majmua, masar) = majmuat_luba(&masarat, &makhzan, &muarrif)?;

    let hadaf = MuarrifMintaqa::min_raqm(raqm.0);
    let _ = tatbeeq(majmua.ghayyir_ism(hadaf, &shakl.ism))?;
    let _ = tatbeeq(majmua.harrik(hadaf, mustatil))?;
    let _ = tatbeeq(majmua.ghayyir_qaida(hadaf, qaida_muhallala))?;
    // None clears the override: an edit carries the whole region, absence included.
    let _ = tatbeeq(majmua.ghayyir_fasila(hadaf, shakl.fasila_milli))?;
    let _ = tatbeeq(majmua.makkin(hadaf, mumakkana))?;
    majmua.ihfaz(&masar).map_err(Khata::from)?;
    Ok(manatiq_hie(muarrif, &majmua))
}

/// Deletes a region — its identity is never reused — and answers the whole set after.
///
/// # Errors
///
/// [`KhataTabaqaAmr::MintaqaMajhula`] when no region carries the identity, plus
/// everything [`manatiq_luba`] raises and whatever saving the file raises.
#[tauri::command]
#[specta::specta]
pub fn ihdhif_mintaqa(
    muarrif: String,
    raqm: RaqmMintaqa,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<ManatiqHie, Khata> {
    let (mut majmua, masar) = majmuat_luba(&masarat, &makhzan, &muarrif)?;
    let _ = tatbeeq(majmua.ihdhif(MuarrifMintaqa::min_raqm(raqm.0)))?;
    majmua.ihfaz(&masar).map_err(Khata::from)?;
    Ok(manatiq_hie(muarrif, &majmua))
}

/// The newest `hadd` lines of one game's reading history, newest first.
///
/// A history that was never written is an empty answer, not a failure. A line a torn
/// append left unreadable is skipped, never fatal: losing a session's transcript to one
/// half-written line would be losing the history to protect it.
///
/// # Errors
///
/// Whatever the game lookup raises, and [`KhataTabaqaAmr::MalafTalif`] when the history
/// file exists and cannot be read at all.
#[tauri::command]
#[specta::specta]
pub fn sijill_qira_luba(
    muarrif: String,
    hadd: u32,
    masarat: tauri::State<'_, Masarat>,
) -> Result<SijillQiraHie, Khata> {
    let id = huwiya(muarrif)?;
    let masar = SijillQira::masar_malaf(&mujallad_tabaqa(&masarat), id);
    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SijillQiraHie {
                sutur: Vec::new(),
                adad_kulli: 0,
            });
        },
        Err(sabab) => return Err(khata_malaf(&masar, sabab)),
    };

    let madakhil = hallil_sutur(&bayt);
    let maqsura = usize::try_from(hadd).unwrap_or(usize::MAX);
    Ok(SijillQiraHie {
        sutur: madakhil
            .iter()
            .rev()
            .take(maqsura)
            .map(madkhal_hie)
            .collect(),
        adad_kulli: u32::try_from(madakhil.len()).unwrap_or(u32::MAX),
    })
}

/// Deletes one game's reading history file; answers whether there was one to delete.
///
/// # Errors
///
/// Whatever the game lookup raises, and [`KhataTabaqaAmr::MalafTalif`] when the file
/// exists and cannot be removed.
#[tauri::command]
#[specta::specta]
pub fn imsah_sijill_qira(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
) -> Result<bool, Khata> {
    let id = huwiya(muarrif)?;
    let masar = SijillQira::masar_malaf(&mujallad_tabaqa(&masarat), id);
    match std::fs::remove_file(&masar) {
        Ok(()) => Ok(true),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(sabab) => Err(khata_malaf(&masar, sabab)),
    }
}

/// The tier-3 disclosure this build ships, and whether this exact wording was acknowledged.
///
/// # Errors
///
/// [`KhataTabaqaAmr::MalafTalif`] when an acknowledgement record exists and does not read.
#[tauri::command]
#[specta::specta]
pub fn nass_ifsah(masarat: tauri::State<'_, Masarat>) -> Result<IfsahHie, Khata> {
    let masar = masar_ifsah(&masarat);
    if !masar.is_file() {
        return Ok(IfsahHie {
            nass_arabi: NASS_IFSAH_ARABI.to_owned(),
            muqarr: false,
            waqt: None,
        });
    }
    let bayt = std::fs::read(&masar).map_err(|sabab| khata_malaf(&masar, sabab))?;
    let qayd: QaydIfsah =
        serde_json::from_slice(&bayt).map_err(|sabab| khata_malaf(&masar, sabab))?;
    // An acknowledgement of an older wording is not an acknowledgement of this one.
    let muqarr = mahfuz_salih(BasmatIfsah::min_raqm(qayd.basma));
    Ok(IfsahHie {
        nass_arabi: NASS_IFSAH_ARABI.to_owned(),
        muqarr,
        waqt: muqarr.then_some(qayd.waqt),
    })
}

/// Records that the disclosure was shown in full and acknowledged, fingerprint and moment.
///
/// # Errors
///
/// [`KhataTabaqaAmr::MalafTalif`] when the record will not serialize, and whatever the
/// atomic write raises.
#[tauri::command]
#[specta::specta]
pub fn aqirr_ifsah(masarat: tauri::State<'_, Masarat>) -> Result<IfsahHie, Khata> {
    let waqt = waqt_alaan();
    let qayd = QaydIfsah {
        basma: BasmatIfsah::hadhihi_al_bina().raqm(),
        waqt: waqt.clone(),
    };
    let masar = masar_ifsah(&masarat);
    let bayt = serde_json::to_vec(&qayd).map_err(|sabab| khata_malaf(&masar, sabab))?;
    kitaba_dharra(&masar, &bayt)?;
    Ok(IfsahHie {
        nass_arabi: NASS_IFSAH_ARABI.to_owned(),
        muqarr: true,
        waqt: Some(waqt),
    })
}

/// Failures of the overlay surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataTabaqaAmr {
    /// No region in this game's set carries the identity the interface sent.
    #[error("no region {raqm} is in this game's set")]
    MintaqaMajhula {
        /// The identity that was looked up.
        raqm: u64,
    },

    /// The rule slug the interface sent is not one it defines.
    #[error("{qaida} is not a translation rule")]
    QaidaMajhula {
        /// The text that was sent.
        qaida: String,
    },

    /// The rectangle has no usable area inside the surface.
    #[error("the region rectangle has no usable area inside the surface")]
    MustatilGhayrSalih,

    /// A file beside the overlay's data exists and does not read.
    #[error("{} does not read: {sabab}", masar.display())]
    MalafTalif {
        /// The file.
        masar: PathBuf,
        /// What the reader said.
        sabab: String,
    },

    /// The region set refused the change, in its own words.
    #[error("the region change was refused: {wasf}")]
    TadeelMarfud {
        /// The refusal as the editor shows it, in Arabic, remedy included.
        unwan: String,
        /// The same refusal in English.
        wasf: String,
    },
}

impl Tafsir for KhataTabaqaAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MintaqaMajhula { .. } => 80,
                    Self::QaidaMajhula { .. } => 81,
                    Self::MustatilGhayrSalih => 82,
                    Self::MalafTalif { .. } => 83,
                    Self::TadeelMarfud { .. } => 84,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Asked about something that is not there, or offered a change the set
            // refuses; nothing was touched and nothing is left in a half state.
            Self::MintaqaMajhula { .. }
            | Self::QaidaMajhula { .. }
            | Self::MustatilGhayrSalih
            | Self::TadeelMarfud { .. } => Khutura::Tanbeeh,
            // A file beside somebody's regions or play history does not read.
            Self::MalafTalif { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MintaqaMajhula { raqm } => format!(
                "لا منطقة بالرقم {raqm} لهذه اللعبة؛ ربما حُذفت للتو. حدّث قائمة المناطق ثم \
                 أعد المحاولة."
            ),
            Self::QaidaMajhula { qaida } => format!(
                "قاعدة الترجمة «{qaida}» غير معروفة. القيم المقبولة: taghayyur أو muaqqit \
                 أو yadawi."
            ),
            Self::MustatilGhayrSalih => {
                "مستطيل المنطقة بلا مساحة داخل الشاشة. اسحب مستطيلًا أوسع داخل حدود الشاشة \
                 ثم أعد المحاولة."
                    .to_owned()
            },
            Self::MalafTalif { masar, .. } => format!(
                "الملف {} موجود ولا يُقرأ. افحصه أو انقله ثم أعد المحاولة.",
                masar.display()
            ),
            Self::TadeelMarfud { unwan, .. } => unwan.clone(),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MintaqaMajhula { raqm } => format!(
                "No region {raqm} is in this game's set any more; it may have just been \
                 deleted. Refresh the region list and try again."
            ),
            Self::QaidaMajhula { qaida } => format!(
                "\"{qaida}\" is not a translation rule. The accepted values are taghayyur, \
                 muaqqit and yadawi."
            ),
            Self::MustatilGhayrSalih => {
                "The region rectangle has no usable area inside the screen. Drag a larger \
                 rectangle within the surface, then try again."
                    .to_owned()
            },
            Self::MalafTalif { masar, sabab } => format!(
                "{} exists and does not read ({sabab}). Inspect or move it, then retry.",
                masar.display()
            ),
            Self::TadeelMarfud { wasf, .. } => wasf.clone(),
        }
    }

    fn khutwa(&self) -> Khutwa {
        Khutwa::AadaMuhawala
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MintaqaMajhula { raqm } => {
                let _ = siyaq.insert(
                    "raqm".to_owned(),
                    QeemaSiyaq::Raqm(i64::try_from(*raqm).unwrap_or(i64::MAX)),
                );
            },
            Self::QaidaMajhula { qaida } => {
                let _ = siyaq.insert("qaida".to_owned(), QeemaSiyaq::Nass(qaida.clone()));
            },
            Self::MalafTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::TadeelMarfud { wasf, .. } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(wasf.clone()));
            },
            Self::MustatilGhayrSalih => {},
        }
        siyaq
    }
}

khata_min!(KhataTabaqaAmr);
