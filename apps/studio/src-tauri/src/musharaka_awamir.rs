//! المشاركة — the sharing loop: harvest what the overlay read, show it, sign it, take another's.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_khatm::MiftahAam;
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::luba::LubaId;
use taarib_tabaqa::sijill_qira::{MadkhalQira, MasdarTarjama, SijillQira};
use taarib_tarjama::dhakira::{Dhakira, NawAsl, ThiqatQira};
use taarib_tarjama::mulahazat::{DhakiratTabaqa, MulahazaTabaqa};
use taarib_tathbeet::bayan::waqt_alaan;
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::masarat::{Masarat, insha_mujallad, kitaba_dharra};
use taarib_warsha::mushtaraka::{
    self, AQSA_HAJM_JASAD, AQSA_HAJM_TARWISA, HuzmaMuwaththaqa, IqraratMusharaka,
    KhiyaratMusharaka, LAHIQA, MusawwadatMusharaka, QaydMushtarak, TahdheerMusharaka,
    TarwisatMushtaraka,
};

use crate::luba_awamir::{huwiya, ijlib_luba};
use crate::tabaqa_awamir::{hallil_sutur, mujallad_tabaqa};
use crate::warsha_awamir::{adad_u32, iqra_janibi, lahza_alaan, uktub_janibi};

/// The watermark file, beside the reading history it tracks.
const ISM_MALAF_HASAD: &str = "hasad.json";

/// Where signed shares are written, under the data root.
const MUJALLAD_MUSHARAKAT: &str = "musharakat";

/// The largest share file this build will even read off disk.
///
/// The crate bounds the header line and the *decompressed* body; neither bounds the file. A
/// stranger's `.dhakira` whose body is incompressible is legitimately about this large, and
/// anything past it cannot decompress within the crate's own ceiling anyway — so it is refused
/// before a byte is loaded rather than after.
fn aqsa_hajm_malaf() -> u64 {
    AQSA_HAJM_JASAD.saturating_add(u64::try_from(AQSA_HAJM_TARWISA).unwrap_or(0))
}

/// How far the harvest has already folded this game's reading history in.
///
/// Without it a second harvest re-records every line the first one did, and `mushahadat` — the
/// only evidence separating a line seen forty times from a line seen once — becomes a number the
/// user inflates by pressing a button twice. `mushtaraka::idmij` refuses the same inflation on
/// the import side by recording one sighting per entry however many the file claims; this is the
/// same refusal on the local side.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct QaydHasad {
    /// The highest history identity already dealt with, folded in or deliberately skipped.
    #[serde(default)]
    akhir: u64,
}

/// How an export or an import is scoped.
///
/// One type for both directions because the two are the same two decisions made by two different
/// people: `mushtaraka::idmij` applies the importer's floor again on the way in, and the fact
/// that the sharer already applied theirs is not a reason to skip it.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhiyaratMusharakaHie {
    /// The floor a measured reading must clear, zero to a hundred.
    pub atabaa: u8,
    /// Whether readings no recognizer measured are included at all.
    pub ghayr_maqisa: bool,
}

impl From<KhiyaratMusharakaHie> for KhiyaratMusharaka {
    fn from(khiyarat: KhiyaratMusharakaHie) -> Self {
        let asas = Self::iftiradiya().bi_atabaa(khiyarat.atabaa);
        if khiyarat.ghayr_maqisa {
            asas.maa_ghayr_maqisa()
        } else {
            asas
        }
    }
}

/// One reading, as the sharing screen lists it.
///
/// Every field is something the user is entitled to read before they decide, and the list is
/// sent whole rather than sampled — see [`MusawwadaMusharakaHie::sutur`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct SatrMusharakaHie {
    /// What the recognizer read off the screen, verbatim.
    pub asl: String,
    /// The Arabic it was given.
    pub arabi: String,
    /// The overlay region it came from, when one was named.
    pub mintaqa: Option<String>,
    /// Which recognizer read it, when the source said.
    pub qari: Option<String>,
    /// What the recognizer said about the reading, or that it said nothing.
    ///
    /// The memory's own type rather than an `Option<u8>` flattened for the wire. An unmeasured
    /// reading has no number, and a field that could hold one is a field a screen will
    /// eventually print a zero into.
    pub thiqa: ThiqatQira,
    /// How many independent sightings were recorded for it.
    pub mushahadat: u32,
}

/// One warning the user must acknowledge by name before anything leaves.
///
/// The wording travels from the crate rather than being restated in the interface's string set,
/// so a screen cannot soften it: the loudest one says these lines came off the user's screen and
/// may carry their character's name, other players' names, or anything else that was on it, and
/// that sentence is the crate's own bytes both here and in the log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TahdheerMusharakaHie {
    /// The stable key the acknowledgement is sent back under.
    pub ramz: String,
    /// The warning, in Arabic, verbatim from the crate.
    pub arabi: String,
    /// The same warning in English, verbatim from the crate.
    pub injilizi: String,
}

/// What one harvest of the reading history did.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HasadHie {
    /// History entries the watermark had not already dealt with.
    pub zurat: u32,
    /// Entries folded into the memory as observations.
    pub sujjilat: u32,
    /// Entries skipped: no Arabic yet, not this machine's own machine translation, or a reading
    /// the memory refuses to store at all.
    pub matruka: u32,
}

/// What an export would contain, gathered and not yet signed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MusawwadaMusharakaHie {
    /// The game's identity.
    pub muarrif: String,
    /// Its display name, as the header will carry it.
    pub ism_luba: String,
    /// How many readings this game has accumulated in total, before the scope is applied.
    #[specta(type = specta_typescript::Number)]
    pub majmu: u64,
    /// How many the scope keeps — the number that would actually leave.
    pub adad: u32,
    /// How many of those carry a confidence a recognizer really measured.
    pub adad_maqis: u32,
    /// How many carry none.
    pub adad_ghayr_maqis: u32,
    /// The scope this draft was gathered under.
    pub khiyarat: KhiyaratMusharakaHie,
    /// The fingerprint of this exact entry set, echoed back on export.
    pub basma: String,
    /// Every warning this payload raises, all of which must be acknowledged.
    pub tahdheerat: Vec<TahdheerMusharakaHie>,
    /// Every entry that would leave, in the order it would be written.
    ///
    /// Whole, never sampled, and never capped. A permit is minted against the fingerprint of
    /// the set the user was shown, so showing them a hundred rows of a two-thousand-row payload
    /// would make the consent a formality — the guarantee this loop is built on is that nothing
    /// leaves the machine without the user having seen it.
    pub sutur: Vec<SatrMusharakaHie>,
    /// What the harvest that ran first did.
    pub hasad: HasadHie,
}

/// A signed share, written.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TasdirMusharakaHie {
    /// The file, absolute.
    pub masar: String,
    /// Its size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// How many readings it carries.
    pub adad: u32,
    /// The fingerprint of the entry set it was signed over.
    pub basma: String,
    /// The signing key's public half, lowercase hex.
    ///
    /// What a recipient has to be given out of band: `mushtaraka::istawrid` verifies against the
    /// key its caller names, never the one the file carries, so a share is worthless to somebody
    /// who was not told whose it is.
    pub miftah: String,
}

/// What somebody else's share says about itself, once it has verified.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HuzmaMusharakaHie {
    /// The game the readings came from.
    pub muarrif: String,
    /// Its display name, as the sharer's machine had it.
    pub ism_luba: String,
    /// Who shared it, shortened.
    pub musahim: String,
    /// When, RFC 3339, as the sharer's machine stated it.
    pub waqt: String,
    /// How many readings it carries.
    pub adad: u32,
    /// How many carry a measured confidence.
    pub adad_maqis: u32,
    /// How many carry none.
    pub adad_ghayr_maqis: u32,
    /// The lowest measured confidence anywhere in it, when anything was measured.
    pub adna_thiqa: Option<u8>,
    /// The floor the sharer's own export applied.
    pub atabaa: u8,
    /// The fingerprint of its entry set, echoed back on merge.
    pub basma: String,
    /// The summary sentence the crate writes, in Arabic.
    pub unwan: String,
    /// The same in English.
    pub wasf: String,
    /// Every entry it carries, so the importer sees what lands in their memory.
    pub sutur: Vec<SatrMusharakaHie>,
}

/// What a merge did.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerIstiradHie {
    /// Entries read out of the share.
    pub zurat: u32,
    /// Entries recorded — inserted, or folded into a row that was already there.
    pub sujjilat: u32,
    /// Entries this machine's own floor rejected.
    pub marfuda: u32,
    /// Entries that fold to a key nothing could ever look up.
    pub talifa: u32,
}

/// The gathered draft and the verified bundle this session is holding.
///
/// Held in process state rather than recomputed per call, and that is the whole consent
/// mechanism rather than a cache: a permit is minted against the fingerprint of the entry set
/// the user was *shown*, so the export has to write the same set that was shown. Re-gathering at
/// export time would silently pick up whatever the overlay recorded in between and mint a permit
/// for a payload nobody looked at.
#[derive(Debug, Default)]
pub struct HalatMusharaka {
    musawwada: parking_lot::Mutex<Option<MusawwadatMusharaka>>,
    huzma: parking_lot::Mutex<Option<HuzmaMuwaththaqa>>,
}

/// The stable key one warning is acknowledged under.
const fn ramz_tahdheer(tahdheer: TahdheerMusharaka) -> &'static str {
    match tahdheer {
        TahdheerMusharaka::MuhtawaShakhsi => "muhtawa_shakhsi",
        TahdheerMusharaka::LamTuqas => "lam_tuqas",
        TahdheerMusharaka::ThiqaMunkhafida => "thiqa_munkhafida",
    }
}

/// The warning a key names, or nothing for a key this build does not define.
///
/// A second match rather than a lookup over a list, so adding a warning to the crate breaks
/// [`ramz_tahdheer`] at compile time instead of silently arriving with no key.
fn tahdheer_min_ramz(ramz: &str) -> Option<TahdheerMusharaka> {
    match ramz {
        "muhtawa_shakhsi" => Some(TahdheerMusharaka::MuhtawaShakhsi),
        "lam_tuqas" => Some(TahdheerMusharaka::LamTuqas),
        "thiqa_munkhafida" => Some(TahdheerMusharaka::ThiqaMunkhafida),
        _ => None,
    }
}

fn tahdheer_hie(tahdheer: TahdheerMusharaka) -> TahdheerMusharakaHie {
    TahdheerMusharakaHie {
        ramz: ramz_tahdheer(tahdheer).to_owned(),
        arabi: tahdheer.wasf_arabi().to_owned(),
        injilizi: tahdheer.wasf_injilizi().to_owned(),
    }
}

fn satr_hie(qayd: &QaydMushtarak) -> SatrMusharakaHie {
    SatrMusharakaHie {
        asl: qayd.asl.clone(),
        arabi: qayd.arabi.clone(),
        mintaqa: qayd.mintaqa.clone(),
        qari: qayd.qari.clone(),
        thiqa: qayd.thiqa,
        mushahadat: qayd.mushahadat,
    }
}

fn khata_malaf(masar: &Path, sabab: impl std::fmt::Display) -> Khata {
    Khata::from(KhataMusharakaAmr::MalafTalif {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

/// The watermark file for one game.
fn masar_hasad(masarat: &Masarat, luba: LubaId) -> PathBuf {
    mujallad_tabaqa(masarat)
        .join(luba.to_string())
        .join(ISM_MALAF_HASAD)
}

/// The directory signed shares are written into.
fn mujallad_musharakat(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join(MUJALLAD_MUSHARAKAT)
}

/// One history entry as an observation, or [`None`] when it is not one.
///
/// Three refusals, and the middle one is the important one. A history entry whose Arabic came
/// from a translation *project* was written by a person, and folding it in here would file a
/// human's work under [`NawAsl::Mulahaza`] and put it in a share — where
/// `mushtaraka::QaydMushtarak` has no field that could say a person wrote it, so the claim would
/// be lost rather than carried. An entry whose Arabic came from the shared memory is somebody
/// else's observation already in this machine's store; re-recording it would corroborate an
/// import with itself. What is left is exactly what this machine's overlay read and this
/// machine's provider translated.
fn mulahaza_min_madkhal(
    madkhal: &MadkhalQira,
    luba: LubaId,
    ism_luba: &str,
) -> Option<MulahazaTabaqa> {
    let arabi = madkhal.arabi.as_ref()?;
    if madkhal.masdar != Some(MasdarTarjama::Aaliya) {
        return None;
    }
    // The history's own bit, not a guess: `maqisa` is false for Windows Runtime OCR and for the
    // bundled portable engine, whose `thiqa` is a stand-in constant, and true only where an
    // engine really reported a number. Reading `thiqa` without it is how eighty becomes a
    // measurement three hops later.
    let thiqa = if madkhal.maqisa {
        ThiqatQira::maqisa(madkhal.thiqa)
    } else {
        ThiqatQira::Ghayr
    };
    Some(
        MulahazaTabaqa::jadeeda(madkhal.asl.as_str(), arabi.as_str(), luba, thiqa)
            .bi_ism_luba(ism_luba)
            .bi_mintaqa(madkhal.ism_mintaqa.clone()),
    )
}

/// Folds this game's reading history into the machine's memory as observations.
///
/// Runs before every gather, because a draft that missed the session the player just finished
/// would be a draft describing something other than what this machine holds.
///
/// # Errors
///
/// [`KhataMusharakaAmr::MalafTalif`] when a history file exists and cannot be read, and whatever
/// the memory raises. A write failure still advances the watermark over everything already
/// recorded, so the entries that did land are not recorded twice by the next attempt.
fn ihsid(masarat: &Masarat, luba: LubaId, ism_luba: &str) -> Natija<HasadHie> {
    let mujallad = mujallad_tabaqa(masarat);
    let masar_sijill = SijillQira::masar_malaf(&mujallad, luba);
    let bayt = match std::fs::read(&masar_sijill) {
        Ok(bayt) => bayt,
        // A game whose overlay never ran has nothing to harvest, which is a normal answer.
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HasadHie::default());
        },
        Err(sabab) => return Err(khata_malaf(&masar_sijill, sabab)),
    };

    let madakhil = hallil_sutur(&bayt);
    let masar_qayd = masar_hasad(masarat, luba);
    let sabiq: QaydHasad = iqra_janibi(&masar_qayd)?;
    let mut tabaqa = DhakiratTabaqa::jadeeda(Dhakira::iftah(masarat)?, luba).bi_ism_luba(ism_luba);

    let mut taqreer = HasadHie::default();
    let mut akhir = sabiq.akhir;
    let mut taathhur: Option<Khata> = None;
    for madkhal in &madakhil {
        let raqm = madkhal.muarrif.raqm();
        if raqm <= sabiq.akhir {
            continue;
        }
        taqreer.zurat = taqreer.zurat.saturating_add(1);
        match mulahaza_min_madkhal(madkhal, luba, ism_luba) {
            Some(mulahaza) => match tabaqa.sajjil(&mulahaza) {
                Ok(true) => taqreer.sujjilat = taqreer.sujjilat.saturating_add(1),
                Ok(false) => taqreer.matruka = taqreer.matruka.saturating_add(1),
                Err(khata) => {
                    taathhur = Some(Khata::from(khata));
                    break;
                },
            },
            None => taqreer.matruka = taqreer.matruka.saturating_add(1),
        }
        akhir = akhir.max(raqm);
    }

    // Before the error is propagated, so a partial harvest is not replayed from the start.
    if akhir != sabiq.akhir {
        insha_mujallad(&mujallad.join(luba.to_string()))?;
        uktub_janibi(&masar_qayd, &QaydHasad { akhir })?;
    }
    match taathhur {
        Some(khata) => Err(khata),
        None => Ok(taqreer),
    }
}

/// A 32-byte key from the hex a person pasted.
///
/// Case-insensitive and trimmed, unlike the crate's own reader for the *header's* key field:
/// that one enforces one canonical spelling because a header is machine-written, and this one is
/// read off whatever a person copied out of a message.
fn miftah_min_hex(nass: &str) -> Option<MiftahAam> {
    let munaqqah = nass.trim();
    if munaqqah.len() != 64 || !munaqqah.bytes().all(|harf| harf.is_ascii_hexdigit()) {
        return None;
    }
    let mut bayt = [0u8; 32];
    let mut azwaj = munaqqah.as_bytes().chunks_exact(2);
    for khana in &mut bayt {
        let zawj = std::str::from_utf8(azwaj.next()?).ok()?;
        *khana = u8::from_str_radix(zawj, 16).ok()?;
    }
    MiftahAam::min_bayt(&bayt).ok()
}

/// Reads a share file, refusing one too large to decompress within the crate's own ceiling.
fn iqra_musharaka(masar: &Path) -> Natija<Vec<u8>> {
    let bayanat = std::fs::metadata(masar).map_err(|sabab| khata_malaf(masar, sabab))?;
    if !bayanat.is_file() {
        return Err(khata_malaf(masar, "it is not a file"));
    }
    let hadd = aqsa_hajm_malaf();
    if bayanat.len() > hadd {
        return Err(Khata::from(KhataMusharakaAmr::MalafKabir {
            masar: masar.to_path_buf(),
            hajm: bayanat.len(),
            hadd,
        }));
    }
    std::fs::read(masar).map_err(|sabab| khata_malaf(masar, sabab))
}

/// Gathers what this game would share, harvesting the reading history first.
///
/// The answer holds every entry that would leave, its fingerprint, and every warning the payload
/// raises. Nothing is written by this call except the observations the harvest folds in;
/// signing happens in [`saddir_musharaka`], against this exact set.
///
/// # Errors
///
/// Whatever the game lookup, the reading history, the memory or the gather raise.
#[tauri::command]
#[specta::specta]
pub fn jahhiz_musharaka(
    muarrif: String,
    khiyarat: KhiyaratMusharakaHie,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    hala: tauri::State<'_, HalatMusharaka>,
) -> Result<MusawwadaMusharakaHie, Khata> {
    let id = huwiya(muarrif)?;
    // The display name is the library's, not the caller's: it goes into a signed header that
    // says which game these readings came from, and letting the interface name it would let a
    // share be labelled with something nobody's store ever agreed to.
    let luba = ijlib_luba(&makhzan, id)?;
    jahhiz(&masarat, id, &luba.ism, khiyarat, &hala)
}

/// The gather itself, with the game's name already resolved.
///
/// Split from the command so the whole loop — harvest, gather, consent, sign, verify, merge —
/// can be driven without a running application; the command is the library lookup and this is
/// everything that decides what leaves.
fn jahhiz(
    masarat: &Masarat,
    id: LubaId,
    ism_luba: &str,
    khiyarat: KhiyaratMusharakaHie,
    hala: &HalatMusharaka,
) -> Natija<MusawwadaMusharakaHie> {
    let hasad = ihsid(masarat, id, ism_luba)?;

    let dhakira = Dhakira::iftah(masarat)?;
    let majmu = dhakira.adad_luba(&id.to_string(), NawAsl::Mulahaza)?;
    let musawwada = mushtaraka::ijma(&dhakira, id, ism_luba, khiyarat.into())?;

    let hie = MusawwadaMusharakaHie {
        muarrif: id.to_string(),
        ism_luba: musawwada.ism_luba().to_owned(),
        majmu,
        adad: adad_u32(musawwada.adad()),
        adad_maqis: adad_u32(musawwada.adad_maqis()),
        adad_ghayr_maqis: adad_u32(musawwada.adad().saturating_sub(musawwada.adad_maqis())),
        khiyarat,
        basma: musawwada.basma().to_owned(),
        tahdheerat: musawwada
            .tahdheerat()
            .into_iter()
            .map(tahdheer_hie)
            .collect(),
        sutur: musawwada.qayyid().iter().map(satr_hie).collect(),
        hasad,
    };
    *hala.musawwada.lock() = Some(musawwada);
    Ok(hie)
}

/// Signs the gathered draft and writes it, once every warning it raised is acknowledged.
///
/// `basma` is the fingerprint the screen displayed. It is checked against the draft this session
/// is holding before a permit is minted, so an interface showing a stale preview cannot consent
/// on behalf of a payload the user never saw; the crate then checks the same fingerprint again
/// against the bytes it is actually writing.
///
/// # Errors
///
/// [`KhataMusharakaAmr::MusawwadaGhayrMujahhaza`] when nothing was gathered for this game,
/// [`KhataMusharakaAmr::BasmaMukhtalifa`] when the screen showed a different entry set,
/// [`KhataMusharakaAmr::TahdheerMajhul`] for an acknowledgement key this build does not define,
/// `taarib_warsha::khata::KhataWarsha::TahdheeratMuallaqa` naming every warning still
/// unacknowledged, and whatever the keychain, the signer or the write raise.
#[tauri::command]
#[specta::specta]
pub fn saddir_musharaka(
    muarrif: String,
    basma: String,
    iqrarat: Vec<String>,
    masarat: tauri::State<'_, Masarat>,
    hala: tauri::State<'_, HalatMusharaka>,
) -> Result<TasdirMusharakaHie, Khata> {
    let id = huwiya(muarrif)?;
    // Read before the draft is touched: minting the key on first use is a keychain call, and a
    // prompt raised while a lock over the draft is held would hold the sharing screen behind it.
    let khass = crate::taqdeem_awamir::miftah_musahim()?;
    saddir(&masarat, id, &basma, &iqrarat, &khass, &hala)
}

/// The export itself, with the signing key already in hand.
///
/// The key is a parameter rather than something this reads, so the consent path can be driven
/// against a key that is not the machine's — and so nothing here can quietly acquire signing
/// authority the caller did not pass in.
fn saddir(
    masarat: &Masarat,
    id: LubaId,
    basma: &str,
    iqrarat: &[String],
    khass: &taarib_khatm::MiftahKhass,
    hala: &HalatMusharaka,
) -> Natija<TasdirMusharakaHie> {
    let mut muqarra = IqraratMusharaka::jadeeda();
    for ramz in iqrarat {
        let Some(tahdheer) = tahdheer_min_ramz(ramz) else {
            return Err(Khata::from(KhataMusharakaAmr::TahdheerMajhul {
                tahdheer: ramz.clone(),
            }));
        };
        muqarra.aqirr(tahdheer);
    }

    let (bayt, adad, basma_mawquua) = {
        let mahfuza = hala.musawwada.lock();
        let Some(musawwada) = mahfuza.as_ref() else {
            return Err(Khata::from(KhataMusharakaAmr::MusawwadaGhayrMujahhaza));
        };
        if musawwada.luba() != id || musawwada.basma() != basma {
            return Err(Khata::from(KhataMusharakaAmr::BasmaMukhtalifa));
        }

        // Minted here and consumed by `saddir` below: the permit carries the fingerprint of the
        // set the screen showed, and a permit granted for one reading cannot write eight.
        let idhn = musawwada.idhn(&muqarra)?;
        let (musahim, _ism, _itimad) = crate::taqdeem_awamir::hawiyati(masarat)?;
        (
            mushtaraka::saddir(musawwada, idhn, musahim, khass, waqt_alaan())?,
            adad_u32(musawwada.adad()),
            musawwada.basma().to_owned(),
        )
    };

    let mujallad = mujallad_musharakat(masarat);
    insha_mujallad(&mujallad)?;
    let masar = mujallad.join(format!("{id}-{}.{LAHIQA}", lahza_alaan()));
    kitaba_dharra(&masar, &bayt)?;

    Ok(TasdirMusharakaHie {
        masar: masar.to_string_lossy().into_owned(),
        hajm: u64::try_from(bayt.len()).unwrap_or(u64::MAX),
        adad,
        basma: basma_mawquua,
        miftah: hex(&khass.aam().bayt()),
    })
}

/// Verifies somebody else's share against the key its recipient was given, and reports what it
/// holds without merging any of it.
///
/// The key is the caller's on purpose. A file that carries its own key and vouches for itself
/// proves only that somebody had a key, so the person importing has to say whose share they
/// think this is — from a listing, a revocation list, or a fingerprint they were sent.
///
/// # Errors
///
/// [`KhataMusharakaAmr::MiftahGhayrSalih`] when the pasted key is not 64 hex characters,
/// [`KhataMusharakaAmr::MalafKabir`] for a file too large to decompress within this build's
/// ceiling, [`KhataMusharakaAmr::MalafTalif`] when the file will not read, and whatever the
/// crate's verification raises — a corrupt header, a schema this build does not read, a body
/// that disagrees with its signed hash, or a signature that does not verify against this key.
#[tauri::command]
#[specta::specta]
pub fn afhas_musharaka(
    masar: String,
    miftah: String,
    hala: tauri::State<'_, HalatMusharaka>,
) -> Result<HuzmaMusharakaHie, Khata> {
    afhas(Path::new(&masar), &miftah, &hala)
}

fn afhas(masar: &Path, miftah: &str, hala: &HalatMusharaka) -> Natija<HuzmaMusharakaHie> {
    let Some(aam) = miftah_min_hex(miftah) else {
        return Err(Khata::from(KhataMusharakaAmr::MiftahGhayrSalih));
    };
    let bayt = iqra_musharaka(masar)?;
    let huzma = mushtaraka::istawrid(&bayt, &aam)?;

    let hie = huzma_hie(&huzma);
    *hala.huzma.lock() = Some(huzma);
    Ok(hie)
}

fn huzma_hie(huzma: &HuzmaMuwaththaqa) -> HuzmaMusharakaHie {
    let tarwisa: &TarwisatMushtaraka = huzma.tarwisa();
    HuzmaMusharakaHie {
        muarrif: huzma.luba().to_string(),
        ism_luba: tarwisa.ism_luba.clone(),
        musahim: tarwisa.musahim.mukhtasar(),
        waqt: tarwisa.waqt.clone(),
        adad: u32::try_from(tarwisa.adad).unwrap_or(u32::MAX),
        adad_maqis: u32::try_from(tarwisa.adad_maqis).unwrap_or(u32::MAX),
        adad_ghayr_maqis: u32::try_from(tarwisa.adad_ghayr_maqis()).unwrap_or(u32::MAX),
        adna_thiqa: tarwisa.adna_thiqa,
        atabaa: tarwisa.atabaa,
        basma: tarwisa.basma.clone(),
        unwan: tarwisa.unwan(),
        wasf: tarwisa.wasf(),
        sutur: huzma.qayyid().iter().map(satr_hie).collect(),
    }
}

/// Merges the verified share into this machine's memory.
///
/// `khiyarat` is this machine's own floor and is applied again on the way in; the sharer's is
/// only what their header states. The bundle is released after a merge that succeeded, because
/// merging one file twice would corroborate an import with itself — the same reason the crate
/// records one sighting per entry however many the file claims.
///
/// # Errors
///
/// [`KhataMusharakaAmr::HuzmaGhayrMafhusa`] when nothing has been verified in this session,
/// [`KhataMusharakaAmr::BasmaMukhtalifa`] when the screen showed a different share, and whatever
/// the memory raises.
#[tauri::command]
#[specta::specta]
pub fn idmij_musharaka(
    basma: String,
    khiyarat: KhiyaratMusharakaHie,
    masarat: tauri::State<'_, Masarat>,
    hala: tauri::State<'_, HalatMusharaka>,
) -> Result<TaqreerIstiradHie, Khata> {
    idmij(&masarat, &basma, khiyarat, &hala)
}

fn idmij(
    masarat: &Masarat,
    basma: &str,
    khiyarat: KhiyaratMusharakaHie,
    hala: &HalatMusharaka,
) -> Natija<TaqreerIstiradHie> {
    let huzma = {
        let mahfuza = hala.huzma.lock();
        let Some(huzma) = mahfuza.as_ref() else {
            return Err(Khata::from(KhataMusharakaAmr::HuzmaGhayrMafhusa));
        };
        if huzma.tarwisa().basma != basma {
            return Err(Khata::from(KhataMusharakaAmr::BasmaMukhtalifa));
        }
        huzma.clone()
    };

    let mut dhakira = Dhakira::iftah(masarat)?;
    let taqreer = mushtaraka::idmij(&mut dhakira, &huzma, khiyarat.into())?;

    // Released only now, and only if it is still the same one: a merge that failed leaves the
    // bundle in place so the user can retry without re-verifying the file.
    let mut mahfuza = hala.huzma.lock();
    if mahfuza
        .as_ref()
        .is_some_and(|mawjuda| mawjuda.tarwisa().basma == basma)
    {
        *mahfuza = None;
    }

    Ok(TaqreerIstiradHie {
        zurat: u32::try_from(taqreer.zurat).unwrap_or(u32::MAX),
        sujjilat: u32::try_from(taqreer.sujjilat).unwrap_or(u32::MAX),
        marfuda: u32::try_from(taqreer.marfuda).unwrap_or(u32::MAX),
        talifa: u32::try_from(taqreer.talifa).unwrap_or(u32::MAX),
    })
}

fn hex(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter().fold(
        String::with_capacity(bayt.len().saturating_mul(2)),
        |mut khraj, wahid| {
            let _ = write!(khraj, "{wahid:02x}");
            khraj
        },
    )
}

/// Failures of the sharing surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataMusharakaAmr {
    /// Nothing was gathered for this game in this session.
    #[error("no share has been gathered for this game in this session")]
    MusawwadaGhayrMujahhaza,

    /// The fingerprint the screen echoed back is not the one being held.
    #[error("the entry set the screen showed is not the one held here")]
    BasmaMukhtalifa,

    /// An acknowledgement key this build does not define.
    #[error("{tahdheer} is not a sharing warning")]
    TahdheerMajhul {
        /// The key that was sent.
        tahdheer: String,
    },

    /// Nothing has been verified in this session, so there is nothing to merge.
    #[error("no share has been verified in this session")]
    HuzmaGhayrMafhusa,

    /// The pasted key is not a 32-byte Ed25519 public key in hex.
    #[error("the key is not 64 hexadecimal characters")]
    MiftahGhayrSalih,

    /// The share file is larger than this build will decompress.
    #[error("{} is {hajm} bytes and this build reads at most {hadd}", masar.display())]
    MalafKabir {
        /// The file.
        masar: PathBuf,
        /// Its size.
        hajm: u64,
        /// The ceiling.
        hadd: u64,
    },

    /// A file beside the sharing path exists and does not read.
    #[error("{} does not read: {sabab}", masar.display())]
    MalafTalif {
        /// The file.
        masar: PathBuf,
        /// What the reader said.
        sabab: String,
    },
}

impl Tafsir for KhataMusharakaAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MusawwadaGhayrMujahhaza => 130,
                    Self::BasmaMukhtalifa => 131,
                    Self::TahdheerMajhul { .. } => 132,
                    Self::HuzmaGhayrMafhusa => 133,
                    Self::MiftahGhayrSalih => 134,
                    Self::MalafKabir { .. } => 135,
                    Self::MalafTalif { .. } => 136,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // A screen out of step with the session: nothing was written and nothing consented.
            Self::MusawwadaGhayrMujahhaza
            | Self::HuzmaGhayrMafhusa
            | Self::TahdheerMajhul { .. }
            | Self::MiftahGhayrSalih
            | Self::MalafKabir { .. } => Khutura::Tanbeeh,
            // Consent was about to be spent on a payload nobody looked at.
            Self::BasmaMukhtalifa => Khutura::Fadih,
            Self::MalafTalif { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MusawwadaGhayrMujahhaza => {
                "لم تُجهَّز مشاركة لهذه اللعبة في هذه الجلسة. اضغط «جهّز المشاركة» ثم أعد \
                 المحاولة."
                    .to_owned()
            },
            Self::BasmaMukhtalifa => {
                "ما عُرض على الشاشة ليس ما سيُكتب؛ ربما تغيّرت الأسطر منذ العرض. أعد التجهيز \
                 واقرأ القائمة من جديد قبل الإقرار."
                    .to_owned()
            },
            Self::TahdheerMajhul { tahdheer } => format!(
                "التحذير «{tahdheer}» غير معروف. القيم المقبولة: muhtawa_shakhsi أو lam_tuqas \
                 أو thiqa_munkhafida."
            ),
            Self::HuzmaGhayrMafhusa => {
                "لم تُفحص مشاركة في هذه الجلسة. افحص الملف بالمفتاح المعلن أولًا ثم أعد \
                 المحاولة."
                    .to_owned()
            },
            Self::MiftahGhayrSalih => {
                "المفتاح المعلن ليس 64 محرفًا ست عشريًّا. انسخه كاملًا ممّن شارك الملف ثم أعد \
                 المحاولة."
                    .to_owned()
            },
            Self::MalafKabir { masar, hajm, hadd } => format!(
                "الملف {} حجمه {hajm} بايت، وهذه النسخة تقرأ {hadd} بايت على الأكثر.",
                masar.display()
            ),
            Self::MalafTalif { masar, .. } => format!(
                "الملف {} موجود ولا يُقرأ. افحصه أو انقله ثم أعد المحاولة.",
                masar.display()
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MusawwadaGhayrMujahhaza => {
                "No share has been gathered for this game in this session. Gather one, then \
                 try again."
                    .to_owned()
            },
            Self::BasmaMukhtalifa => {
                "What the screen showed is not what would be written; the readings may have \
                 changed since. Gather again and read the list before acknowledging."
                    .to_owned()
            },
            Self::TahdheerMajhul { tahdheer } => format!(
                "\"{tahdheer}\" is not a sharing warning. The accepted values are \
                 muhtawa_shakhsi, lam_tuqas and thiqa_munkhafida."
            ),
            Self::HuzmaGhayrMafhusa => {
                "No share has been verified in this session. Verify the file against the \
                 sharer's key first, then try again."
                    .to_owned()
            },
            Self::MiftahGhayrSalih => {
                "The sharer's key is not 64 hexadecimal characters. Copy it in full from \
                 whoever shared the file, then try again."
                    .to_owned()
            },
            Self::MalafKabir { masar, hajm, hadd } => format!(
                "{} is {hajm} bytes and this build reads at most {hadd}.",
                masar.display()
            ),
            Self::MalafTalif { masar, sabab } => format!(
                "{} exists and does not read ({sabab}). Inspect or move it, then retry.",
                masar.display()
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MalafTalif { .. } | Self::MalafKabir { .. } => Khutwa::FathNusus,
            _ => Khutwa::AadaMuhawala,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::TahdheerMajhul { tahdheer } => {
                let _ = siyaq.insert("tahdheer".to_owned(), QeemaSiyaq::Nass(tahdheer.clone()));
            },
            Self::MalafKabir { masar, hajm, hadd } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert(
                    "hajm".to_owned(),
                    QeemaSiyaq::Raqm(i64::try_from(*hajm).unwrap_or(i64::MAX)),
                );
                let _ = siyaq.insert(
                    "hadd".to_owned(),
                    QeemaSiyaq::Raqm(i64::try_from(*hadd).unwrap_or(i64::MAX)),
                );
            },
            Self::MalafTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::MusawwadaGhayrMujahhaza
            | Self::BasmaMukhtalifa
            | Self::HuzmaGhayrMafhusa
            | Self::MiftahGhayrSalih => {},
        }
        siyaq
    }
}

khata_min!(KhataMusharakaAmr);

#[cfg(test)]
mod ikhtibarat {
    use taarib_khatm::MiftahKhass;
    use taarib_mustalahat::luba::MasdarLuba;
    use taarib_tabaqa::manatiq::MuarrifMintaqa;
    use taarib_tabaqa::sijill_qira::{NatijatIdraj, ThiqatSatr};
    use taarib_tarjama::mulahazat::RaddTabaqa;
    use taarib_usus::idadat::{IdadatMuzawwid, NawMuzawwid};

    use super::*;

    /// What every test here answers with, so a fixture that could not be built — a directory, a
    /// history file, a memory — propagates with `?` beside the product's own [`Khata`] instead
    /// of being unwrapped. `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// ELDEN RING, as a Steam library identifies it.
    ///
    /// A real application identifier, so the per-game keying is exercised against an identity
    /// the product would really compute. **The recognized strings below are authored fixtures**
    /// — shaped like what an overlay reads, but no engine ran and no game file was parsed.
    const APPID_ELDEN: u32 = 1_245_620;

    fn luba() -> LubaId {
        LubaId::min_masdar(&MasdarLuba::Steam(APPID_ELDEN), "ELDEN RING")
    }

    /// A signing key with no keychain behind it, so the consent path can be driven offline.
    fn miftah(badhra: u8) -> MiftahKhass {
        MiftahKhass::min_bayt(&[badhra; 32])
    }

    /// A directory of this test's own, so two tests never see each other's data root.
    #[expect(
        clippy::disallowed_methods,
        reason = "a scratch directory under `std::env::temp_dir()`, never a data root or a game \
                  directory"
    )]
    fn mujallad(ism: &str) -> NatijatIkhtibar<PathBuf> {
        let masar = std::env::temp_dir().join(format!("taarib_musharaka_{ism}"));
        let _ = std::fs::remove_dir_all(&masar);
        std::fs::create_dir_all(&masar)?;
        Ok(masar)
    }

    fn masarat(jidhr: &Path) -> NatijatIkhtibar<Masarat> {
        let masarat = Masarat::min_judhur(jidhr.join("bayanat"), jidhr.join("idadat"));
        masarat.takid()?;
        Ok(masarat)
    }

    /// What an overlay might read off a screen, and the Arabic a translator gives back.
    fn azwaj() -> [(&'static str, &'static str); 4] {
        [
            ("Press E to open the door", "اضغط E لفتح الباب"),
            ("You have obtained a Golden Rune", "حصلت على رونة ذهبية"),
            ("Save and Quit", "احفظ واخرج"),
            ("The old road north is closed", "الطريق القديم شمالًا مغلق"),
        ]
    }

    /// Writes a real reading history through the overlay's own writer.
    ///
    /// Each line is recorded three times, because that is what a recognizer does while a box
    /// sits on screen — the writer folds the repeats into one entry, and the harvest must not
    /// undo that.
    fn iktub_sijill(
        masarat: &Masarat,
        id: LubaId,
        maqisa: bool,
        masdar: MasdarTarjama,
    ) -> NatijatIkhtibar<()> {
        let mujallad = mujallad_tabaqa(masarat);
        insha_mujallad(&mujallad.join(id.to_string()))?;
        let masar = SijillQira::masar_malaf(&mujallad, id);
        let mut sijill = SijillQira::iftah(&masar, id, 512)?;
        let mintaqa = MuarrifMintaqa::min_raqm(1);
        for (raqm, &(asl, arabi)) in azwaj().iter().enumerate() {
            let lahza = 1_700_000_000_u64.saturating_add(u64::try_from(raqm).unwrap_or(0));
            let thiqa = if maqisa {
                ThiqatSatr::maqisa_bi(92)
            } else {
                ThiqatSatr::Ghayr
            };
            let mut awwal = None;
            for _ in 0..3 {
                if let NatijatIdraj::Judida(muarrif) =
                    sijill.qayyid(lahza, mintaqa, "شريط الحوار", asl, thiqa)?
                {
                    awwal = Some(muarrif);
                }
            }
            if let Some(muarrif) = awwal {
                assert!(sijill.adkhil_tarjama(muarrif, arabi, masdar));
            }
        }
        sijill.aghliq()?;
        Ok(())
    }

    fn iqrarat_kull(musawwada: &MusawwadaMusharakaHie) -> Vec<String> {
        musawwada
            .tahdheerat
            .iter()
            .map(|tahdheer| tahdheer.ramz.clone())
            .collect()
    }

    /// The whole loop, end to end: one player's session becomes a second player's free answer.
    ///
    /// Gather, refuse without consent, acknowledge, sign, verify on a second machine, merge, and
    /// then ask that second memory a line it never saw. The two data roots are real directories
    /// and the share really moves between them as bytes.
    #[test]
    fn ma_qaraahu_laib_yujib_laiban_aakhar() -> NatijatIkhtibar {
        let jidhr = mujallad("dawra_kamila")?;
        let awwal = masarat(&jidhr.join("awwal"))?;
        let thani = masarat(&jidhr.join("thani"))?;
        let id = luba();
        let khass = miftah(7);
        iktub_sijill(&awwal, id, true, MasdarTarjama::Aaliya)?;

        // --- gather -------------------------------------------------------------------
        let hala_awwal = HalatMusharaka::default();
        let khiyarat = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: false,
        };
        let musawwada = jahhiz(&awwal, id, "ELDEN RING", khiyarat, &hala_awwal)?;
        assert_eq!(
            musawwada.hasad.sujjilat, 4,
            "one observation per distinct line, not per read"
        );
        assert_eq!(musawwada.adad, 4);
        assert_eq!(musawwada.adad_maqis, 4);
        assert_eq!(musawwada.majmu, 4);
        assert_eq!(
            musawwada.sutur.len(),
            4,
            "every entry that would leave is shown, not a sample"
        );
        assert!(musawwada.sutur.iter().all(|satr| satr.thiqa.qisat()));

        // --- consent ------------------------------------------------------------------
        let shakhsi = musawwada
            .tahdheerat
            .iter()
            .find(|tahdheer| tahdheer.ramz == "muhtawa_shakhsi")
            .ok_or("the personal-content warning must fire whenever there is anything to share")?;
        assert_eq!(
            shakhsi.injilizi,
            TahdheerMusharaka::MuhtawaShakhsi.wasf_injilizi(),
            "the loudest warning reaches the screen as the crate's own bytes"
        );
        assert!(shakhsi.injilizi.contains("read off your screen"));
        assert!(shakhsi.arabi.contains("قُرئت من شاشتك"));

        // Nothing leaves while a warning is unacknowledged.
        let bila_iqrar = saddir(&awwal, id, &musawwada.basma, &[], &khass, &hala_awwal);
        assert!(
            bila_iqrar.is_err(),
            "an unacknowledged warning must block the export"
        );

        // Nor does consent for one entry set spend on another.
        let mukhtalifa = saddir(
            &awwal,
            id,
            "0000000000000000000000000000000000000000000000000000000000000000",
            &iqrarat_kull(&musawwada),
            &khass,
            &hala_awwal,
        );
        assert!(
            mukhtalifa.is_err(),
            "a permit is about the set the screen showed"
        );

        // --- export -------------------------------------------------------------------
        let tasdir = saddir(
            &awwal,
            id,
            &musawwada.basma,
            &iqrarat_kull(&musawwada),
            &khass,
            &hala_awwal,
        )?;
        assert_eq!(tasdir.adad, 4);
        assert_eq!(tasdir.basma, musawwada.basma);
        assert!(tasdir.masar.ends_with(".dhakira"));

        // --- import, on a second machine ----------------------------------------------
        let hala_thani = HalatMusharaka::default();
        let masar_huzma = PathBuf::from(&tasdir.masar);
        assert!(
            afhas(&masar_huzma, &hex(&miftah(9).aam().bayt()), &hala_thani).is_err(),
            "a share verifies against the key its recipient was given, never the file's own"
        );
        let huzma = afhas(&masar_huzma, &tasdir.miftah, &hala_thani)?;
        assert_eq!(huzma.adad, 4);
        assert_eq!(huzma.ism_luba, "ELDEN RING");
        assert_eq!(huzma.basma, tasdir.basma);
        assert_eq!(
            huzma.sutur.len(),
            4,
            "the importer sees what lands in their memory"
        );

        let taqreer = idmij(&thani, &huzma.basma, khiyarat, &hala_thani)?;
        assert_eq!(taqreer.zurat, 4);
        assert_eq!(taqreer.sujjilat, 4);
        assert_eq!(taqreer.marfuda, 0);
        assert_eq!(taqreer.talifa, 0);

        // --- the payoff ---------------------------------------------------------------
        let mut tabaqa = DhakiratTabaqa::jadeeda(Dhakira::iftah(&thani)?, id);
        for (asl, arabi) in azwaj() {
            match tabaqa.istafhim(asl)? {
                RaddTabaqa::Jahiza {
                    arabi: mahfuz, naw, ..
                } => {
                    assert_eq!(
                        mahfuz, arabi,
                        "the second player gets the first player's answer"
                    );
                    assert_eq!(
                        naw,
                        NawAsl::Mulahaza,
                        "an imported reading can never outrank reviewed text"
                    );
                },
                RaddTabaqa::Majhula => {
                    return Err(format!("{asl} was never answered on the importing side").into());
                },
            }
        }
        assert_eq!(
            tabaqa.ihsaat().tarjamat_madfua(),
            0,
            "a line answered from an import costs no translation"
        );
        Ok(())
    }

    /// A second gather harvests nothing again.
    ///
    /// The watermark is what keeps `mushahadat` — the only evidence separating a line seen forty
    /// times from a line seen once — from being a number a user inflates by pressing a button
    /// twice.
    #[test]
    fn al_hasad_la_yuad_nafsah() -> NatijatIkhtibar {
        let jidhr = mujallad("alama_ma")?;
        let masarat = masarat(&jidhr)?;
        let id = luba();
        iktub_sijill(&masarat, id, true, MasdarTarjama::Aaliya)?;

        let hala = HalatMusharaka::default();
        let khiyarat = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: false,
        };
        let awwal = jahhiz(&masarat, id, "ELDEN RING", khiyarat, &hala)?;
        assert_eq!(awwal.hasad.sujjilat, 4);

        let thani = jahhiz(&masarat, id, "ELDEN RING", khiyarat, &hala)?;
        assert_eq!(
            thani.hasad.zurat, 0,
            "the watermark leaves nothing to re-record"
        );
        assert_eq!(thani.hasad.sujjilat, 0);
        assert_eq!(
            thani.adad, awwal.adad,
            "and the payload is the same size it was"
        );
        Ok(())
    }

    /// A line a person translated in a project is never harvested as an observation.
    ///
    /// `QaydMushtarak` has no field that could say a human wrote it, so folding project text in
    /// here would not merely mislabel it — it would strip the claim on the way out and hand
    /// somebody's reviewed translation to the world as a screen reading.
    #[test]
    fn amal_al_insan_la_yudkhal_fil_mulahazat() -> NatijatIkhtibar {
        let jidhr = mujallad("amal_insan")?;
        let masarat = masarat(&jidhr)?;
        let id = luba();
        iktub_sijill(&masarat, id, true, MasdarTarjama::Mashru)?;

        let hala = HalatMusharaka::default();
        let khiyarat = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: false,
        };
        let musawwada = jahhiz(&masarat, id, "ELDEN RING", khiyarat, &hala)?;
        assert_eq!(musawwada.hasad.zurat, 4, "the entries were looked at");
        assert_eq!(
            musawwada.hasad.sujjilat, 0,
            "and every one of them was refused"
        );
        assert_eq!(musawwada.adad, 0);
        assert!(
            musawwada.tahdheerat.is_empty(),
            "an empty payload raises no warning"
        );
        Ok(())
    }

    /// An accumulation nothing measured is withheld until somebody says so by name.
    ///
    /// Windows Runtime OCR and the bundled portable engine report no confidence at all, so on
    /// those platforms this is every share. The default scope excludes the lot; including them
    /// is one acknowledged warning, and the alternative default is shipping unmeasured readings
    /// silently.
    #[test]
    fn ghayr_al_maqisa_muhtajaza_hatta_yuqarr_biha() -> NatijatIkhtibar {
        let jidhr = mujallad("ghayr_maqisa")?;
        let masarat = masarat(&jidhr)?;
        let id = luba();
        let khass = miftah(3);
        iktub_sijill(&masarat, id, false, MasdarTarjama::Aaliya)?;

        let hala = HalatMusharaka::default();
        let mabdai = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: false,
        };
        let mahjuba = jahhiz(&masarat, id, "ELDEN RING", mabdai, &hala)?;
        assert_eq!(mahjuba.majmu, 4, "the readings are in the memory");
        assert_eq!(
            mahjuba.adad, 0,
            "and none of them is in the default payload"
        );
        assert!(
            saddir(&masarat, id, &mahjuba.basma, &[], &khass, &hala).is_err(),
            "signing an empty share would put a file in the world that answers nothing"
        );

        let mawsa = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: true,
        };
        let musawwada = jahhiz(&masarat, id, "ELDEN RING", mawsa, &hala)?;
        assert_eq!(musawwada.adad, 4);
        assert_eq!(musawwada.adad_maqis, 0);
        assert_eq!(musawwada.adad_ghayr_maqis, 4);
        assert!(
            musawwada
                .sutur
                .iter()
                .all(|satr| satr.thiqa == ThiqatQira::Ghayr)
        );

        let asma: Vec<&str> = musawwada
            .tahdheerat
            .iter()
            .map(|tahdheer| tahdheer.ramz.as_str())
            .collect();
        assert!(
            asma.contains(&"lam_tuqas"),
            "the unmeasured warning must fire: {asma:?}"
        );
        assert!(asma.contains(&"muhtawa_shakhsi"));

        // Acknowledging only the loud one is not acknowledging both.
        assert!(
            saddir(
                &masarat,
                id,
                &musawwada.basma,
                &["muhtawa_shakhsi".to_owned()],
                &khass,
                &hala,
            )
            .is_err(),
            "each warning is acknowledged by name, not as a group"
        );
        let tasdir = saddir(
            &masarat,
            id,
            &musawwada.basma,
            &iqrarat_kull(&musawwada),
            &khass,
            &hala,
        )?;
        assert_eq!(tasdir.adad, 4);
        Ok(())
    }

    /// An acknowledgement key this build does not define is refused rather than ignored.
    #[test]
    fn tahdheer_majhul_marfud() -> NatijatIkhtibar {
        let jidhr = mujallad("tahdheer_majhul")?;
        let masarat = masarat(&jidhr)?;
        let id = luba();
        iktub_sijill(&masarat, id, true, MasdarTarjama::Aaliya)?;

        let hala = HalatMusharaka::default();
        let khiyarat = KhiyaratMusharakaHie {
            atabaa: 60,
            ghayr_maqisa: false,
        };
        let musawwada = jahhiz(&masarat, id, "ELDEN RING", khiyarat, &hala)?;
        let khata = saddir(
            &masarat,
            id,
            &musawwada.basma,
            &["kul_shay".to_owned()],
            &miftah(5),
            &hala,
        );
        assert!(
            khata.is_err(),
            "an unknown key must not silently acknowledge nothing"
        );
        Ok(())
    }

    /// Every warning the crate defines has a key, and every key names a warning.
    #[test]
    fn ramz_kul_tahdheer_yaud_ilayh() {
        for tahdheer in [
            TahdheerMusharaka::MuhtawaShakhsi,
            TahdheerMusharaka::LamTuqas,
            TahdheerMusharaka::ThiqaMunkhafida,
        ] {
            assert_eq!(tahdheer_min_ramz(ramz_tahdheer(tahdheer)), Some(tahdheer));
        }
        assert_eq!(tahdheer_min_ramz("la_shay"), None);
    }

    /// A pasted key is read in either case and refused at any other length.
    #[test]
    fn al_miftah_al_mulsaq_yuqra_bi_kilta_al_halatayn() {
        let bayt = miftah(11).aam().bayt();
        let saghir = hex(&bayt);
        let kabir = saghir.to_uppercase();
        assert_eq!(miftah_min_hex(&saghir).map(|aam| aam.bayt()), Some(bayt));
        assert_eq!(
            miftah_min_hex(&format!("  {kabir}  ")).map(|aam| aam.bayt()),
            Some(bayt)
        );
        assert!(miftah_min_hex("").is_none());
        assert!(miftah_min_hex(&saghir[..62]).is_none());
        assert!(miftah_min_hex(&"z".repeat(64)).is_none());
    }

    /// The accumulation this whole loop shares can be produced with no API key.
    ///
    /// The sharing loop is only worth building if a player who has paid nobody can fill a memory
    /// in the first place. The local arm is the one provider the Studio builds without opening
    /// the keychain, and its meter reports a ceiling of zero — which the overlay's seam reads as
    /// *free*, not as *exhausted*.
    #[test]
    fn al_muzawwid_al_mahalli_maslak_bila_miftah() -> NatijatIkhtibar {
        let tarif = IdadatMuzawwid {
            muarrif: "ollama".to_owned(),
            naw: NawMuzawwid::Mahalli,
            namudhaj: "qwen2.5:7b".to_owned(),
            asas: None,
            hisab_miftah: None,
            mufaal: true,
            hadd_talabat: 60,
            mizaniya: None,
        };
        let muzawwid = crate::warsha_awamir::bin_muzawwid(&tarif, 0, 1_700_000_000)?;
        assert_eq!(muzawwid.ism(), "ollama");
        assert_eq!(
            muzawwid.takalif().saqf(),
            0,
            "a free provider's ceiling is zero, and the overlay reads that as free"
        );
        assert_eq!(muzawwid.takalif().munfaq(), 0);
        assert!(
            !muzawwid.qudrat().taklifa.madfu(),
            "nothing here charges anybody"
        );
        Ok(())
    }
}
