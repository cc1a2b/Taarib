//! التثبيت — the patches offered for a game, the download, the check, and the removal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use taarib_aman::iqrar::{self, SijillIqrar};
use taarib_makhzan::sijillat::{SijillMuharrik, SijillRuqaa, SijillTathbeet};
use taarib_makhzan::wasl::{Makhzan, alaan};
use taarib_mustalahat::bina::MutabaqaBina;
use taarib_mustalahat::luba::{Luba, LubaId};
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_mustawda::masadir::{MasdarMustawda, SilsilatMasadir};
use taarib_mustawda::mutabaqa::MutabiqBina;
use taarib_mustawda::tanzeel::{self, MukhbirTaqaddum, TalabTanzeel, Taqaddum, nazzil};
use taarib_mustawda::tarteeb::{FiatTaqyeem, KhiyaratTarteeb, MudkhalTarteeb, rattib};
use taarib_mustawda::{MarhalatTanzeel, jalb_fahras};
use taarib_tathbeet::bayan::{NawTathbeet, Tathbeet};
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::masar_tathbeet::la_tashtaghil;
use taarib_tathbeet::tahaqquq::{TaqreerTahaqquq, tahaqquq_kamil};
use taarib_tathbeet::taraju::{
    RadLaShay, SiyasatIstiada, TaqreerIstiada, TaqreerKul, istiada_kul, istiada_nass,
    istiada_sawt, nazzif_nusakh,
};
use taarib_usus::ISDAR;
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::Masarat;
use tauri::Emitter as _;

use crate::luba_awamir::{BinaHie, bina_hie, huwiya, ijlib_luba, jidhr_nusakh, muthabbat};

/// The window event every download progress report is delivered on.
pub const ISM_HADATH_TANZEEL: &str = "taarib://taqaddum-tanzeel";

/// The file the first-run acknowledgement is recorded in, under the data root.
pub const ISM_MALAF_IQRAR: &str = "iqrar.json";

/// Which of the two installations a command is about.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum NawTathbeetHie {
    /// Translated text.
    Nass,
    /// Translated voice.
    Sawt,
}

impl NawTathbeetHie {
    /// The interface's kind for one of the installer's.
    const fn min_asli(naw: NawTathbeet) -> Self {
        match naw {
            NawTathbeet::Nass => Self::Nass,
            NawTathbeet::Sawt => Self::Sawt,
        }
    }
}

/// Which installations a removal is asked to take off.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type,
)]
#[serde(rename_all = "snake_case")]
pub enum MatlabIzala {
    /// The text installation only.
    Nass,
    /// The voice installation only.
    Sawt,
    /// Both, independently.
    Kul,
}

/// One patch the registry offers for a game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MudkhalRuqaaHie {
    /// The patch lineage.
    pub id: RuqaaId,
    /// The published revision.
    pub murajaa: RuqaaRevision,
    /// The title the contributor gave it.
    pub unwan: String,
    /// The contributor's display name.
    pub musahim: String,
    /// How much of the game it covers.
    pub taghtiya: Taghtiya,
    /// Coverage by string, 0.0 to 1.0.
    pub nisbat_taghtiya: f32,
    /// How many strings it carries.
    pub adad_nusus: u32,
    /// The package size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// The same size as the interface writes it.
    pub hajm_maqru: String,
    /// The tier it installs at, 1 to 3.
    pub tabaqa_raqm: u8,
    /// The tier's name in Arabic.
    pub tabaqa_arabi: String,
    /// How it was translated, in Arabic.
    pub tareeqa_arabi: String,
    /// The licence identifier.
    pub rukhsa: String,
    /// The rating, as one Arabic line.
    pub taqyeem_arabi: String,
    /// When this revision was published, RFC 3339.
    pub waqt_nashr: String,
    /// The package's content hash, which is also where it is cached on disk.
    pub basmat_muhtawa: String,
    /// How well it matches the installed build, absent when no build
    /// fingerprint has been computed for this game yet.
    pub mutabaqa: Option<MutabaqaBina>,
    /// The whole match verdict as one Arabic sentence, on the same terms.
    pub mutabaqa_arabi: Option<String>,
    /// Whether the client will install it at all.
    pub qabila_lil_tathbeet: bool,
    /// Whether installing requires the user to acknowledge a risk first.
    pub yahtaj_iqrar: bool,
}

/// Everything the patches panel draws for one game.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct RuqaaLuba {
    /// Taarib's identity for the game.
    pub muarrif: String,
    /// The build every verdict was measured against.
    pub bina: Option<BinaHie>,
    /// Every patch the registry lists, best match first.
    pub mudkhalat: Vec<MudkhalRuqaaHie>,
    /// How many of them this build can install.
    pub adad_mutawafiq: u32,
    /// No build fingerprint has been computed, so nothing was judged.
    pub bila_bina: bool,
    /// The sources that were tried, in the order they were tried.
    pub masadir: Vec<String>,
}

/// One download progress report.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqaddumTanzeel {
    /// The game the package is for.
    pub muarrif: String,
    /// The patch lineage being fetched.
    pub ruqaa: String,
    /// Bytes accounted for so far.
    #[specta(type = specta_typescript::Number)]
    pub manqul: u64,
    /// Bytes the listing declared.
    #[specta(type = specta_typescript::Number)]
    pub majmu: u64,
    /// Bytes done, as the interface writes them.
    pub manqul_maqru: String,
    /// Bytes declared, as the interface writes them.
    pub majmu_maqru: String,
    /// The stage, in Arabic.
    pub marhala_arabi: String,
    /// Whether this is the last report for this download.
    pub tamma: bool,
}

impl TaqaddumTanzeel {
    /// One report, addressed to the game and the patch it belongs to.
    fn jadeed(muarrif: &str, ruqaa: &str, taqaddum: Taqaddum) -> Self {
        Self {
            muarrif: muarrif.to_owned(),
            ruqaa: ruqaa.to_owned(),
            manqul: taqaddum.manqul,
            majmu: taqaddum.majmu,
            manqul_maqru: taqaddum.manqul_maqru(),
            majmu_maqru: taqaddum.majmu_maqru(),
            marhala_arabi: taqaddum.marhala.wasf_arabi().to_owned(),
            tamma: matches!(taqaddum.marhala, MarhalatTanzeel::Tamma),
        }
    }
}

/// A verified package, in place.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HasilatTanzeel {
    /// Where the verified file landed.
    pub masar: String,
    /// Its size in bytes.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// The content hash it was verified against.
    pub basma: String,
    /// The last progress report, so a caller that missed the events still has
    /// the final numbers.
    pub akhir: TaqaddumTanzeel,
}

/// What verification found in one installation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerTahaqquqHie {
    /// Which installation was checked.
    pub naw: NawTathbeetHie,
    /// The game, as the manifest recorded it.
    pub luba: String,
    /// When it was installed, RFC 3339.
    pub waqt_tathbeet: String,
    /// How many recorded paths were checked.
    pub adad_masarat: u32,
    /// How many changed.
    pub adad_munharif: u32,
    /// How many are gone.
    pub adad_mafqud: u32,
    /// How many are back to their original bytes.
    pub adad_mustaad: u32,
    /// How many had to be hashed.
    pub adad_mahsub: u32,
    /// Whether every recorded path is still what Taarib wrote.
    pub salim: bool,
    /// The verdict, in Arabic.
    pub natija_arabi: String,
    /// The same verdict in English.
    pub natija_injilizi: String,
    /// Every path that is not in the state the manifest describes.
    pub munharifa: Vec<String>,
    /// Files inside directories Taarib created that the manifest does not name.
    pub zaida: Vec<String>,
}

/// What one installation's removal did.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct TaqreerIzalaHie {
    /// Which installation was removed.
    pub naw: NawTathbeetHie,
    /// The game, as the manifest recorded it.
    pub luba: String,
    /// Originals written back and verified.
    pub mustaada: u32,
    /// Paths already back to their original bytes when the run reached them.
    pub kanat_asliya: u32,
    /// Files Taarib had added, deleted.
    pub mahdhufa: u32,
    /// Directories Taarib had created, removed because they were empty.
    pub mujalladat_muzala: u32,
    /// Directories left in place because they still hold somebody else's files.
    pub mujalladat_matruka: u32,
    /// Settings put back to their previous values.
    pub idadat_mustaada: u32,
    /// Paths the store replaced with its own newer versions, left as they are.
    pub mustabdala: Vec<String>,
    /// Whether the game is now exactly as the store shipped it.
    pub nazif: bool,
}

/// Both installations' outcomes, gathered independently.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HasilatIzala {
    /// Whether every installation that was present came off.
    pub najahat: bool,
    /// The text installation's outcome, absent when there was none.
    pub nass: Option<TaqreerIzalaHie>,
    /// The voice installation's outcome, absent when there was none.
    pub sawt: Option<TaqreerIzalaHie>,
    /// Every failure, as an Arabic sentence.
    pub akhta: Vec<String>,
    /// Bytes reclaimed by discarding the preserved originals, zero when the
    /// user asked for them to be kept.
    #[specta(type = specta_typescript::Number)]
    pub hajm_muharrar: u64,
    /// The report lines, for the log and the diagnostics bundle.
    pub sutur: Vec<String>,
}

/// Whether the game's own executable is running right now.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HalatTashghil {
    /// Whether the game must be treated as running.
    ///
    /// Read together with [`Self::majhul`]: `true` with `majhul` set is a
    /// precaution, not an observation.
    pub tashtaghil: bool,
    /// Whether this answer is a guess because the process table could not be
    /// read.
    ///
    /// A sandboxed build — Flatpak, Snap, a container — sees its own process
    /// table rather than the host's, so every game looks stopped. Reporting
    /// that as an observation is the mistake that let a patch be written into
    /// an open game; reporting it as an error instead loses the ability to say
    /// which game and why. This field is how the screen distinguishes "it is
    /// not running" from "nobody here can tell".
    pub majhul: bool,
    /// The process, as the system names it.
    pub amaliya: Option<String>,
    /// The executable it was matched against.
    pub tanfidhi: Option<String>,
}

/// Where the first-run acknowledgement stands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HalatIqrar {
    /// Whether the statement still needs acknowledging.
    pub yahtaj: bool,
    /// The statement version this build shows.
    pub isdar_nass: u32,
    /// The statement itself, in Arabic.
    pub nass_arabi: String,
    /// When the acknowledgement was given, RFC 3339.
    pub waqt: Option<String>,
    /// Which build of Taarib asked.
    pub isdar_taarib: Option<String>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Every patch the registry offers for one game, best match first.
///
/// Only the shard the game's identity falls in is fetched, and a shard the
/// manifest still vouches for is read from the cache, so an unchanged registry
/// costs no network at all. Listings are cached into the store on the way past,
/// so the panel still has something to draw with no source reachable.
///
/// # Errors
///
/// [`KhataTathbeetAmr::GhayrMuttasil`] when offline mode leaves no source to
/// try, and whatever the store, the registry client or the game lookup raise.
#[tauri::command]
#[specta::specta]
pub async fn ruqaa_luba(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<RuqaaLuba, Khata> {
    let id = huwiya(muarrif)?;
    let hali = idadat.hali();
    let makhbaa = masarat.makhbaa();

    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    let silsila = silsilat_masadir(&hali)?;
    let masadir: Vec<String> =
        silsila.masadir().iter().map(MasdarMustawda::wasf).collect();

    let fahras = jalb_fahras(&silsila, &[id], &makhbaa, None).await?;
    let mulakhkhasat: Vec<MulakhkhasRuqaa> = fahras
        .shareeha_luba(id)
        .map(|shareeha| shareeha.ruqaa(id).to_vec())
        .unwrap_or_default();

    if let Some((aila, muarrif_matjar)) = miftah_mustawda(&luba) {
        let makhzan = Makhzan::clone(&makhzan);
        let li_khazn = mulakhkhasat.clone();
        bil_hajb(move || {
            makhzan.bi_muamala(|muamala| {
                let sijill = SijillRuqaa::jadeed(muamala);
                for mulakhkhas in &li_khazn {
                    sijill.sajjil(mulakhkhas, &aila, &muarrif_matjar)?;
                }
                Ok(())
            })
        })
        .await?;
    }

    let mudkhalat = rattib_murashshahat(&luba, mulakhkhasat);
    let adad_mutawafiq = u32::try_from(
        mudkhalat.iter().filter(|mudkhal| mudkhal.qabila_lil_tathbeet).count(),
    )
    .unwrap_or(u32::MAX);

    Ok(RuqaaLuba {
        muarrif: id.to_string(),
        bina: luba.bina.as_ref().map(bina_hie),
        bila_bina: luba.bina.is_none(),
        mudkhalat,
        adad_mutawafiq,
        masadir,
    })
}

/// Fetches one patch package, verifying it during transfer.
///
/// Progress is emitted on [`ISM_HADATH_TANZEEL`] as it goes; the returned value
/// repeats the last report so a caller that subscribed late is not left
/// guessing. A partial file left by an interrupted run is resumed rather than
/// refetched, and the verified file lands under the content-addressed patch
/// store, which is where the installer looks for it.
///
/// # Errors
///
/// [`KhataTathbeetAmr::MuarrifRuqaaGhayrSalih`] when the patch identity is not
/// one the registry issues, [`KhataTathbeetAmr::RuqaaGhayrMawjuda`] when the
/// registry lists no such patch for this game, and whatever the registry client
/// raises — an unreachable source, a size overrun, or bytes that do not hash to
/// what the listing declared.
#[tauri::command]
#[specta::specta]
pub async fn nazzil_ruqaa(
    muarrif: String,
    ruqaa: String,
    tatbiq: tauri::AppHandle,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<HasilatTanzeel, Khata> {
    let id = huwiya(muarrif)?;
    let matlub = huwiyat_ruqaa(ruqaa)?;
    let hali = idadat.hali();
    let makhbaa = masarat.makhbaa();

    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    let silsila = silsilat_masadir(&hali)?;
    let fahras = jalb_fahras(&silsila, &[id], &makhbaa, None).await?;
    let mulakhkhas = fahras
        .shareeha_luba(id)
        .and_then(|shareeha| shareeha.ruqaa(id).iter().find(|wahid| wahid.id == matlub))
        .cloned()
        .ok_or_else(|| {
            Khata::from(KhataTathbeetAmr::RuqaaGhayrMawjuda {
                ruqaa: matlub.to_string(),
                ism: luba.ism,
            })
        })?;

    let basma = mulakhkhas.basmat_muhtawa.to_string();
    let hadaf = masarat.ruqaa_bi_basma(&basma)?;
    let talab = TalabTanzeel::min_ruqaa(&mulakhkhas, hadaf);
    let amil = tanzeel::amil_tanzeel()?;

    let muarrif_hadath = id.to_string();
    let ruqaa_hadath = matlub.to_string();
    let mukhbir = MukhbirTaqaddum::min_nida(move |taqaddum| {
        let hadath = TaqaddumTanzeel::jadeed(&muarrif_hadath, &ruqaa_hadath, taqaddum);
        if let Err(sabab) = tatbiq.emit(ISM_HADATH_TANZEEL, hadath) {
            tracing::debug!(sabab = %sabab, "a download progress report was not delivered");
        }
    });

    let masar = nazzil(&amil, &talab, &mukhbir).await?;
    tracing::info!(
        luba = %id,
        ruqaa = %matlub,
        hajm = mulakhkhas.hajm,
        masar = %masar.display(),
        "patch package verified and in place"
    );

    let akhir = TaqaddumTanzeel::jadeed(
        &id.to_string(),
        &matlub.to_string(),
        Taqaddum {
            manqul: mulakhkhas.hajm,
            majmu: mulakhkhas.hajm,
            marhala: MarhalatTanzeel::Tamma,
        },
    );

    Ok(HasilatTanzeel {
        masar: masar.to_string_lossy().into_owned(),
        hajm: mulakhkhas.hajm,
        basma,
        akhir,
    })
}

/// Verifies every installation this game has, hashing every recorded path.
///
/// The full check rather than the size-and-time one: this answer is shown next
/// to a button that overwrites somebody's game, and a decision that acts on a
/// change detector is a decision resting on a filesystem's timestamp resolution.
///
/// # Errors
///
/// [`KhataTathbeetAmr::LaTathbeet`] when the game has no installation to check,
/// and whatever the verifier raises — an unreadable manifest, or a path it
/// cannot hash because the game is holding it open.
#[tauri::command]
#[specta::specta]
pub fn tahaqquq_ruqaa(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<Vec<TaqreerTahaqquqHie>, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;

    let mut taqareer = Vec::new();
    for naw in NawTathbeet::KULL {
        if !Tathbeet::mawjud(&nusakh, naw) {
            continue;
        }
        let taqreer = tahaqquq_kamil(&luba.jidhr, &nusakh, naw)?;
        taqareer.push(tahaqquq_hie(naw, &taqreer));
    }

    if taqareer.is_empty() {
        return Err(Khata::from(KhataTathbeetAmr::LaTathbeet { ism: luba.ism }));
    }
    Ok(taqareer)
}

/// Removes an installation and puts the game's own files back.
///
/// The game is checked for a running process first, so a locked file is named
/// before anything is touched rather than met partway through. Text and voice
/// are removed independently: a failure in one never skips the other, and both
/// outcomes are returned whatever either of them did.
///
/// The preserved originals are discarded afterwards unless the user asked for
/// backups to be kept, which is what `takhzin.ibqa_nusakh` means. Discarding
/// them is refused by the installer itself while any record is still
/// outstanding, so an interrupted removal keeps everything it still needs.
///
/// # Errors
///
/// [`KhataTathbeetAmr::LaTathbeet`] when the game has no installation of the
/// requested kind, and [`KhataTathbeet::LubaTashtaghil`] when the game is
/// running. A restore that fails is carried inside the result rather than
/// raised, because the other kind's outcome is still owed to the user.
#[tauri::command]
#[specta::specta]
pub fn azil_ruqaa(
    muarrif: String,
    matlab: MatlabIzala,
    sarim: bool,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<HasilatIzala, Khata> {
    let id = huwiya(muarrif)?;
    let hali = idadat.hali();
    let luba = ijlib_luba(&makhzan, id)?;
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;

    if let Some(tanfidhi) = luba.tanfidhi.as_deref()
        && let Some(nass) = tanfidhi.to_str()
    {
        la_tashtaghil(nass)?;
    }

    let siyasa = if sarim { SiyasatIstiada::Sarima } else { SiyasatIstiada::Muhafiza };
    let mut radd = RadLaShay;
    let kul = match matlab {
        MatlabIzala::Kul => istiada_kul(&luba.jidhr, &nusakh, siyasa, &mut radd),
        MatlabIzala::Nass => TaqreerKul {
            nass: Tathbeet::mawjud(&nusakh, NawTathbeet::Nass)
                .then(|| istiada_nass(&luba.jidhr, &nusakh, siyasa, &mut radd)),
            sawt: None,
        },
        MatlabIzala::Sawt => TaqreerKul {
            nass: None,
            sawt: Tathbeet::mawjud(&nusakh, NawTathbeet::Sawt)
                .then(|| istiada_sawt(&luba.jidhr, &nusakh, siyasa, &mut radd)),
        },
    };

    if !kul.wujidat() {
        return Err(Khata::from(KhataTathbeetAmr::LaTathbeet { ism: luba.ism }));
    }

    let najahat = kul.najahat();
    let akhta: Vec<String> = kul.akhta().into_iter().map(Tafsir::arabi).collect();
    let sutur = kul.taqreer();
    let nass = izala_hie(NawTathbeet::Nass, kul.nass.as_ref());
    let sawt = izala_hie(NawTathbeet::Sawt, kul.sawt.as_ref());

    let hajm_muharrar = if najahat && !hali.takhzin.ibqa_nusakh {
        nazzif(&luba.jidhr, &nusakh, matlab)
    } else {
        0
    };

    if najahat {
        sajjil_izala(&makhzan, &luba, &nusakh, id)?;
    }

    Ok(HasilatIzala { najahat, nass, sawt, akhta, hajm_muharrar, sutur })
}

/// Whether the game's own executable is running right now.
///
/// What the detail screen asks before it offers to install or remove anything,
/// so the answer is a disabled button with a reason rather than a refusal after
/// the user committed.
///
/// # Errors
///
/// Whatever the game lookup raises.
#[tauri::command]
#[specta::specta]
pub fn hal_tashtaghil(
    muarrif: String,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<HalatTashghil, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;

    let Some(tanfidhi) = luba.tanfidhi.as_deref().and_then(Path::to_str) else {
        // No executable was ever resolved, so there is no process to look for.
        // Not running as far as anything here can see, and `majhul` says that
        // the second half of that sentence is doing the work.
        return Ok(HalatTashghil {
            tashtaghil: false,
            majhul: true,
            amaliya: None,
            tanfidhi: None,
        });
    };

    match la_tashtaghil(tanfidhi) {
        Ok(()) => Ok(HalatTashghil {
            tashtaghil: false,
            majhul: false,
            amaliya: None,
            tanfidhi: Some(tanfidhi.to_owned()),
        }),
        Err(KhataTathbeet::LubaTashtaghil { amaliya, tanfidhi: masar }) => Ok(HalatTashghil {
            tashtaghil: true,
            majhul: false,
            amaliya: Some(amaliya),
            tanfidhi: Some(masar.to_string_lossy().into_owned()),
        }),
        // The sandbox verdict, which is an answer rather than a failure. It
        // stays on the safe side — the install path refuses this case outright
        // — but it is reported so the screen can say why instead of showing a
        // bare error for a question it could simply not answer.
        Err(KhataTathbeet::HalatLubaMajhula { .. }) => Ok(HalatTashghil {
            tashtaghil: true,
            majhul: true,
            amaliya: None,
            tanfidhi: Some(tanfidhi.to_owned()),
        }),
        Err(sabab) => Err(Khata::from(sabab)),
    }
}

/// Where the first-run acknowledgement stands.
///
/// # Errors
///
/// [`taarib_aman::KhataAman`] when the record exists and cannot be read.
#[tauri::command]
#[specta::specta]
pub fn iqrar_aman(masarat: tauri::State<'_, Masarat>) -> Result<HalatIqrar, Khata> {
    let sijill = iqrar::iqra(&masar_iqrar(&masarat))?;
    Ok(iqrar_hie(sijill.as_ref()))
}

/// Records the user's acknowledgement of the current statement.
///
/// The timestamp comes from the store's own clock rather than the host's, so
/// two records written in one session cannot disagree about when it was.
///
/// # Errors
///
/// [`taarib_aman::KhataAman`] when the record cannot be written, and whatever
/// the store raises.
#[tauri::command]
#[specta::specta]
pub fn sajjil_iqrar_aman(
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<HalatIqrar, Khata> {
    let waqt = makhzan.bil_qira(alaan)?;
    let sijill = iqrar::ahfaz(&masar_iqrar(&masarat), waqt, ISDAR.to_owned())?;
    tracing::info!(isdar_nass = sijill.isdar_nass, "the safety statement was acknowledged");
    Ok(iqrar_hie(Some(&sijill)))
}

// ---------------------------------------------------------------------------
// Registry sources, matching and ranking
// ---------------------------------------------------------------------------

/// The sources to try, in the order they are tried.
///
/// Local copies first: they cost nothing, they work with no network, and a user
/// who configured one did so to be asked before the forge is.
fn silsilat_masadir(idadat: &Idadat) -> Natija<SilsilatMasadir> {
    let mut masadir: Vec<MasdarMustawda> = idadat
        .masadir
        .mahalliya
        .iter()
        .map(|jidhr| MasdarMustawda::MujalladMahalli { jidhr: jidhr.clone() })
        .collect();

    if !idadat.masadir.wadaa_ghayr_muttasil {
        let rasmi = idadat.masadir.rasmi.trim();
        if !rasmi.is_empty() {
            masadir.push(MasdarMustawda::Shabaka { jidhr: rasmi.to_owned() });
        }
        for mira in &idadat.masadir.maraya {
            let mira = mira.trim();
            if !mira.is_empty() {
                masadir.push(MasdarMustawda::Mira { jidhr: mira.to_owned() });
            }
        }
    }

    if masadir.is_empty() {
        return Err(Khata::from(KhataTathbeetAmr::GhayrMuttasil));
    }
    Ok(SilsilatMasadir::jadida(masadir))
}

/// The registry shard family and identifier this game is listed under.
fn miftah_mustawda(luba: &Luba) -> Option<(String, String)> {
    let asl = luba.masdar_asli()?;
    let kamil = asl.muarrif();
    let dhayl = kamil
        .split_once(':')
        .map_or_else(|| kamil.clone(), |(_, dhayl)| dhayl.to_owned());
    Some((asl.aila().slug().to_owned(), dhayl))
}

/// Judges every listing against the installed build and orders them.
///
/// With no build fingerprint recorded there is nothing to judge against, so
/// nothing is judged: the listings come back newest first with no verdict,
/// rather than with a verdict invented from an absence.
fn rattib_murashshahat(
    luba: &Luba,
    mut mulakhkhasat: Vec<MulakhkhasRuqaa>,
) -> Vec<MudkhalRuqaaHie> {
    let khiyarat = KhiyaratTarteeb::default();

    let Some(bina) = luba.bina.clone() else {
        mulakhkhasat.sort_by(|awwal, thani| {
            thani
                .waqt_nashr
                .cmp(&awwal.waqt_nashr)
                .then_with(|| thani.murajaa.cmp(&awwal.murajaa))
        });
        return mulakhkhasat
            .iter()
            .map(|mulakhkhas| mudkhal_hie(mulakhkhas, None, None, khiyarat))
            .collect();
    };

    let mutabiq = MutabiqBina::jadeed(bina);
    let ahkam = mutabiq.ruqaa_kul(&mulakhkhasat);
    let mudkhalat: Vec<MudkhalTarteeb> = mulakhkhasat
        .into_iter()
        .zip(ahkam.iter())
        .map(|(mulakhkhas, hukm)| {
            MudkhalTarteeb::jadeed(mulakhkhas, hukm.mutabaqa(), Some(hukm.wasf_arabi()))
        })
        .collect();

    let murattaba = rattib(mudkhalat);
    murattaba
        .iter()
        .map(|mudkhal| {
            mudkhal_hie(
                &mudkhal.ruqaa,
                Some(mudkhal.mutabaqa),
                mudkhal.sabab.clone(),
                khiyarat,
            )
        })
        .collect()
}

/// One listing as the patches panel writes it.
fn mudkhal_hie(
    mulakhkhas: &MulakhkhasRuqaa,
    mutabaqa: Option<MutabaqaBina>,
    mutabaqa_arabi: Option<String>,
    khiyarat: KhiyaratTarteeb,
) -> MudkhalRuqaaHie {
    MudkhalRuqaaHie {
        id: mulakhkhas.id,
        murajaa: mulakhkhas.murajaa,
        unwan: mulakhkhas.unwan.clone(),
        musahim: mulakhkhas.ism_musahim.clone(),
        taghtiya: mulakhkhas.taghtiya,
        nisbat_taghtiya: mulakhkhas.taghtiya.nisba(),
        adad_nusus: mulakhkhas.adad_nusus,
        hajm: mulakhkhas.hajm,
        hajm_maqru: tanzeel::hajm_maqru(mulakhkhas.hajm),
        tabaqa_raqm: mulakhkhas.tabaqa.raqm(),
        tabaqa_arabi: mulakhkhas.tabaqa.ism_arabi().to_owned(),
        tareeqa_arabi: mulakhkhas.tareeqa.wasf_arabi().to_owned(),
        rukhsa: mulakhkhas.rukhsa.muarrif().to_owned(),
        taqyeem_arabi: FiatTaqyeem::min_mulakhkhas(mulakhkhas, khiyarat).wasf_arabi(),
        waqt_nashr: mulakhkhas.waqt_nashr.clone(),
        basmat_muhtawa: mulakhkhas.basmat_muhtawa.to_string(),
        mutabaqa,
        mutabaqa_arabi,
        qabila_lil_tathbeet: mutabaqa.is_some_and(MutabaqaBina::qabila_lil_tathbeet),
        yahtaj_iqrar: mutabaqa.is_some_and(MutabaqaBina::yahtaj_iqrar),
    }
}

// ---------------------------------------------------------------------------
// Rendering and small steps
// ---------------------------------------------------------------------------

/// One verification report as the check panel writes it.
fn tahaqquq_hie(naw: NawTathbeet, taqreer: &TaqreerTahaqquq) -> TaqreerTahaqquqHie {
    let natija = taqreer.natija();
    TaqreerTahaqquqHie {
        naw: NawTathbeetHie::min_asli(naw),
        luba: taqreer.luba.clone(),
        waqt_tathbeet: taqreer.waqt_tathbeet.clone(),
        adad_masarat: adad(taqreer.halat.len()),
        adad_munharif: adad(taqreer.adad_munharif),
        adad_mafqud: adad(taqreer.adad_mafqud),
        adad_mustaad: adad(taqreer.adad_mustaad),
        adad_mahsub: adad(taqreer.adad_mahsub),
        salim: taqreer.salim(),
        natija_arabi: natija.arabi().to_owned(),
        natija_injilizi: natija.injilizi().to_owned(),
        munharifa: taqreer.masarat_munharifa().into_iter().map(ToOwned::to_owned).collect(),
        zaida: taqreer.zaida.clone(),
    }
}

/// One removal outcome, or nothing when that kind was not installed and
/// nothing when it failed — the failure travels in [`HasilatIzala::akhta`].
fn izala_hie(
    naw: NawTathbeet,
    natija: Option<&Result<TaqreerIstiada, KhataTathbeet>>,
) -> Option<TaqreerIzalaHie> {
    let taqreer = natija?.as_ref().ok()?;
    Some(TaqreerIzalaHie {
        naw: NawTathbeetHie::min_asli(naw),
        luba: taqreer.luba.clone(),
        mustaada: adad(taqreer.mustaada),
        kanat_asliya: adad(taqreer.kanat_asliya),
        mahdhufa: adad(taqreer.mahdhufa),
        mujalladat_muzala: adad(taqreer.mujalladat_muzala),
        mujalladat_matruka: adad(taqreer.mujalladat_matruka),
        idadat_mustaada: adad(taqreer.idadat_mustaada),
        mustabdala: taqreer.mustabdala.clone(),
        nazif: taqreer.nazif(),
    })
}

/// The acknowledgement record as the first-run dialogue writes it.
fn iqrar_hie(sijill: Option<&SijillIqrar>) -> HalatIqrar {
    HalatIqrar {
        yahtaj: iqrar::yahtaj_iqrar(sijill),
        isdar_nass: iqrar::ISDAR_NASS,
        nass_arabi: iqrar::NASS_ARABI.to_owned(),
        waqt: sijill.map(|wahid| wahid.waqt.clone()),
        isdar_taarib: sijill.map(|wahid| wahid.isdar_taarib.clone()),
    }
}

/// Discards the preserved originals for whichever kinds were asked for.
///
/// A refusal here is reported and no more: the game is already restored, and a
/// backup that could not be discarded costs disk rather than correctness.
fn nazzif(jidhr_luba: &Path, jidhr_nusakh: &Path, matlab: MatlabIzala) -> u64 {
    let mut hajm = 0_u64;
    for naw in NawTathbeet::KULL {
        let matlub = match matlab {
            MatlabIzala::Kul => true,
            MatlabIzala::Nass => naw == NawTathbeet::Nass,
            MatlabIzala::Sawt => naw == NawTathbeet::Sawt,
        };
        if !matlub || !Tathbeet::mawjud(jidhr_nusakh, naw) {
            continue;
        }
        match nazzif_nusakh(jidhr_luba, jidhr_nusakh, naw) {
            Ok(muharrar) => hajm = hajm.saturating_add(muharrar),
            Err(sabab) => tracing::warn!(
                naw = naw.ism(),
                khata = %Khata::from(sabab).li_sijill(),
                "the preserved originals were kept because they could not be discarded"
            ),
        }
    }
    hajm
}

/// Marks the game's active installation removed, once nothing is left applied.
fn sajjil_izala(
    makhzan: &Makhzan,
    luba: &Luba,
    jidhr_nusakh: &Path,
    id: LubaId,
) -> Natija<()> {
    if NawTathbeet::KULL.into_iter().any(|naw| muthabbat(&luba.jidhr, jidhr_nusakh, naw)) {
        return Ok(());
    }
    let Some(sabiq) = makhzan.bil_qira(|ittisal| SijillTathbeet::jadeed(ittisal).nashit(id))?
    else {
        return Ok(());
    };
    makhzan.bi_muamala(|muamala| SijillTathbeet::jadeed(muamala).azil(sabiq.id))
}

/// Where the acknowledgement record lives.
pub(crate) fn masar_iqrar(masarat: &Masarat) -> PathBuf {
    masarat.jidhr_bayanat().join(ISM_MALAF_IQRAR)
}

/// Parses the patch lineage the interface sends back.
fn huwiyat_ruqaa(ruqaa: String) -> Natija<RuqaaId> {
    serde_json::from_value::<RuqaaId>(serde_json::Value::String(ruqaa.clone()))
        .map_err(|_| Khata::from(KhataTathbeetAmr::MuarrifRuqaaGhayrSalih { ruqaa }))
}

/// A count the interface can hold, saturating rather than wrapping.
fn adad(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

/// Runs blocking store work off the async runtime's worker threads.
///
/// `taarib-makhzan` is synchronous by design and says so: every call into it
/// blocks the thread it is on, and choosing which thread that is belongs to the
/// application rather than to the store.
async fn bil_hajb<T, F>(amal: F) -> Natija<T>
where
    F: FnOnce() -> Natija<T> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(amal).await {
        Ok(natija) => natija,
        Err(sabab) => Err(Khata::from(KhataTathbeetAmr::MuhimmaMutawaqqifa {
            tafsil: sabab.to_string(),
        })),
    }
}

// ---------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------

/// Failures of the installation surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataTathbeetAmr {
    /// The patch identity the interface sent is not one the registry issues.
    #[error("{ruqaa} is not a patch identity")]
    MuarrifRuqaaGhayrSalih {
        /// The text that was sent.
        ruqaa: String,
    },

    /// The registry lists no such patch for this game.
    #[error("the registry lists no patch {ruqaa} for {ism}")]
    RuqaaGhayrMawjuda {
        /// The patch identity that was looked up.
        ruqaa: String,
        /// The game it was looked up for.
        ism: String,
    },

    /// The game has no installation of the requested kind.
    #[error("{ism} has no Taarib installation")]
    LaTathbeet {
        /// The game's name.
        ism: String,
    },

    /// Offline mode is on and no local registry copy is configured, so there is
    /// nothing left to ask.
    #[error("offline mode is on and no local registry copy is configured")]
    GhayrMuttasil,

    /// A blocking task did not finish, which means the runtime is shutting down.
    #[error("the game's executable is not identified; probe {ism} again first")]
    TanfidhiMajhul {
        /// The game.
        ism: String,
    },

    /// The package rewrites whole game containers, which this build does not do.
    #[error("the package rewrites {adad} game container(s), which this build does not install")]
    FuruqGhayrMaduma {
        /// How many container differences it carries.
        adad: u64,
    },

    /// The package is present and its container will not open.
    #[error("the package could not be read: {tafsil}")]
    HuzmaTalifa {
        /// What the container said.
        tafsil: String,
    },

    /// No build fingerprint is on record, so nothing can be matched against it.
    #[error("{ism} has no stored build fingerprint; probe it first")]
    BinaMajhula {
        /// The game.
        ism: String,
    },

    /// The install pipeline refused, and carries its own sentence.
    #[error("{injilizi}")]
    TathbeetFashil {
        /// The pipeline's Arabic sentence.
        arabi: String,
        /// Its English sentence.
        injilizi: String,
    },

    /// A task the command spawned did not return an answer.
    #[error("a background task did not finish: {tafsil}")]
    MuhimmaMutawaqqifa {
        /// What the runtime reported.
        tafsil: String,
    },

    /// The game's publisher already ships Arabic, and the setting that would
    /// allow replacing it is off.
    #[error("{ism} already ships official Arabic")]
    LughaRasmiyaMawjuda {
        /// The game's name, as its launcher gives it.
        ism: String,
    },
}

impl Tafsir for KhataTathbeetAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MuarrifRuqaaGhayrSalih { .. } => 20,
                    Self::RuqaaGhayrMawjuda { .. } => 21,
                    Self::LaTathbeet { .. } => 22,
                    Self::GhayrMuttasil => 23,
                    Self::MuhimmaMutawaqqifa { .. } => 24,
                    Self::TanfidhiMajhul { .. } => 25,
                    Self::FuruqGhayrMaduma { .. } => 26,
                    Self::HuzmaTalifa { .. } => 27,
                    Self::BinaMajhula { .. } => 28,
                    Self::TathbeetFashil { .. } => 29,
                    Self::LughaRasmiyaMawjuda { .. } => 30,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Nothing was written in any of these; the operation simply did not
            // start.
            Self::MuarrifRuqaaGhayrSalih { .. }
            | Self::LughaRasmiyaMawjuda { .. }
            | Self::RuqaaGhayrMawjuda { .. }
            | Self::LaTathbeet { .. }
            | Self::GhayrMuttasil
            | Self::TanfidhiMajhul { .. }
            | Self::FuruqGhayrMaduma { .. }
            | Self::HuzmaTalifa { .. }
            | Self::BinaMajhula { .. }
            | Self::TathbeetFashil { .. } => Khutura::Khatar,
            Self::MuhimmaMutawaqqifa { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MuarrifRuqaaGhayrSalih { .. } => {
                "المعرّف المطلوب ليس معرّف رقعة. أعد تحميل قائمة الرقع لهذه اللعبة."
                    .to_owned()
            }
            Self::RuqaaGhayrMawjuda { ism, .. } => format!(
                "لم يعد المستودع يعرض هذه الرقعة لـ{ism}. ربما سُحبت بعد آخر تحديث للفهرس؛ \
                 أعد تحميل القائمة."
            ),
            Self::LaTathbeet { ism } => {
                format!("لا يوجد تثبيت من تعريب في {ism}، فليس هناك ما يُفحص أو يُزال.")
            }
            Self::GhayrMuttasil => {
                "وضع العمل دون اتصال مفعَّل ولا توجد نسخة محلية من المستودع، فلا مصدر يُسأل. \
                 أضف مجلدًا محليًا أو عطّل وضع دون اتصال من الإعدادات."
                    .to_owned()
            }
            Self::MuhimmaMutawaqqifa { .. } => {
                "توقّفت مهمة في الخلفية قبل أن تنتهي. أعد المحاولة؛ إن تكرّر الأمر فراجع سجلّ \
                 التشخيص."
                    .to_owned()
            }
            Self::TanfidhiMajhul { ism } => format!(
                "لم يُحدَّد الملف التنفيذي لـ{ism} بعد. أعد فحص اللعبة أولًا ثم أعد المحاولة."
            ),
            Self::FuruqGhayrMaduma { adad } => format!(
                "تعيد هذه الرقعة كتابة {adad} حاوية من حاويات اللعبة، وهذه النسخة لا تثبّت \
                 هذا النوع بعد. لم يُكتب شيء."
            ),
            Self::HuzmaTalifa { .. } => {
                "تعذّرت قراءة ملف الرقعة؛ ربما لم يكتمل تنزيله. أعد تنزيله ثم أعد المحاولة."
                    .to_owned()
            }
            Self::BinaMajhula { ism } => format!(
                "لا توجد بصمة بناء محفوظة لـ{ism}. افحص اللعبة أولًا ثم أعد المحاولة."
            ),
            Self::TathbeetFashil { arabi, .. } => arabi.clone(),
            Self::LughaRasmiyaMawjuda { ism } => format!(
                "{ism} تصدر بعربية رسمية من ناشرها، فلا تُثبَّت عليها رقعة. الرقعة تستبدل \
                 بترجمة آلية عملًا ترجمه محترفون وروجع وجُرِّب داخل اللعبة. إن كانت العربية \
                 الرسمية رديئة فعلًا، فعّل استبدال العربية الرسمية في الإعدادات."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MuarrifRuqaaGhayrSalih { ruqaa } => {
                format!("{ruqaa} is not a patch identity. Reload the patch list for this game.")
            }
            Self::RuqaaGhayrMawjuda { ruqaa, ism } => format!(
                "The registry no longer lists patch {ruqaa} for {ism}. It was probably \
                 withdrawn since the index was last refreshed; reload the list."
            ),
            Self::LaTathbeet { ism } => {
                format!("{ism} has no Taarib installation, so there is nothing to check or remove.")
            }
            Self::GhayrMuttasil => {
                "Offline mode is on and no local registry copy is configured, so there is no \
                 source left to ask. Add a local folder, or turn offline mode off in Settings."
                    .to_owned()
            }
            Self::MuhimmaMutawaqqifa { tafsil } => {
                format!("A background task did not finish ({tafsil}). Try again.")
            }
            Self::TanfidhiMajhul { ism } => {
                format!("The executable of {ism} is not identified; probe the game again first.")
            }
            Self::FuruqGhayrMaduma { adad } => format!(
                "This package rewrites {adad} game container(s), which this build does not \
                 install yet. Nothing was written."
            ),
            Self::HuzmaTalifa { tafsil } => {
                format!("The package file could not be read: {tafsil}. Download it again.")
            }
            Self::BinaMajhula { ism } => {
                format!("{ism} has no stored build fingerprint; probe the game first.")
            }
            Self::TathbeetFashil { injilizi, .. } => injilizi.clone(),
            Self::LughaRasmiyaMawjuda { ism } => format!(
                "{ism} already ships official Arabic from its publisher, so no patch is \
                 installed over it. A patch would replace professionally translated, reviewed \
                 and play-tested work with machine output. If that official Arabic really is \
                 poor, turn on replacing official Arabic in settings."
            ),
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            Self::MuarrifRuqaaGhayrSalih { .. } | Self::RuqaaGhayrMawjuda { .. } => {
                Khutwa::IadatMutabaqaBina
            }
            Self::LaTathbeet { .. } => Khutwa::LaShay,
            Self::GhayrMuttasil => Khutwa::FathIdadat { qism: QismIdadat::Masadir },
            Self::MuhimmaMutawaqqifa { .. } | Self::HuzmaTalifa { .. } => Khutwa::AadaMuhawala,
            Self::TanfidhiMajhul { .. } | Self::BinaMajhula { .. } => Khutwa::AadaFahsMuharrik,
            Self::FuruqGhayrMaduma { .. } => Khutwa::TahdithTaarib,
            Self::TathbeetFashil { .. } => Khutwa::FathTashkhis,
            // The remedy is a setting, and it is the only one this refusal has.
            Self::LughaRasmiyaMawjuda { .. } => Khutwa::FathIdadat { qism: QismIdadat::Lugha },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MuarrifRuqaaGhayrSalih { ruqaa } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
            }
            Self::RuqaaGhayrMawjuda { ruqaa, ism } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::LaTathbeet { ism }
            | Self::TanfidhiMajhul { ism }
            | Self::BinaMajhula { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::GhayrMuttasil => {}
            Self::MuhimmaMutawaqqifa { tafsil } | Self::HuzmaTalifa { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            }
            Self::FuruqGhayrMaduma { adad } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Hajm(*adad));
            }
            Self::TathbeetFashil { injilizi, .. } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(injilizi.clone()));
            }
            Self::LughaRasmiyaMawjuda { ism } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
        }
        siyaq
    }
}

khata_min!(KhataTathbeetAmr);

/// The bundled revocation list, signed by the owner key at sequence 1.
///
/// The starting point every client verifies before the registry has served a
/// newer one; `QaimatSahb::min_bayt` refuses it unless the signature holds.
const QAIMAT_SAHB_ASLIYA: &[u8] = include_bytes!("../../../../assets/qaimat_sahb.json");

/// The bundled revocation list, verified against the owner anchor.
///
/// One function rather than the two lines at each gate, because both the
/// one-click install and the automatic pipeline hand the same list to the same
/// safety gate, and a second spelling of "which anchor verifies it" is a second
/// place for that answer to drift.
///
/// # Errors
///
/// [`Khata`] when the compiled anchor is not a public key, or when the bundled
/// list does not verify against it — which would mean the build itself is
/// inconsistent, not that the user did anything.
pub(crate) fn qaimat_sahb() -> Result<taarib_aman::qaimat_sahb::QaimatSahb, Khata> {
    let miftah_aam = taarib_khatm::MiftahAam::min_bayt(&taarib_khatm::MIRSAT_MALIK.miftah)
        .map_err(|q| Khata::min_tafsir(&q))?;
    taarib_aman::qaimat_sahb::QaimatSahb::min_bayt(QAIMAT_SAHB_ASLIYA, &miftah_aam)
        .map_err(|q| Khata::min_tafsir(&q))
}

/// One install stage, as the progress event names it.
pub const ISM_HADATH_TATHBEET: &str = "taarib://marhalat-tathbeet";

/// What a completed one-click install reports.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct NatijatTathbeetHie {
    /// The compatibility decision that was acted on, in Arabic.
    pub tawafuq_arabi: String,
    /// How many content placements were written.
    pub adad_muhtawa: u32,
    /// The post-write verification verdict, in Arabic.
    pub tahaqquq_arabi: String,
    /// Whether that verification found every path exactly as written.
    pub tahaqquq_salim: bool,
}

fn khata_naqra(fashal: &taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet) -> Khata {
    Khata::min_tafsir(&KhataTathbeetAmr::TathbeetFashil {
        arabi: fashal.arabi(),
        injilizi: fashal.injilizi(),
    })
}

/// Installs a downloaded or imported package into a game, end to end.
///
/// Quarantine, safety verdict, permit, backup, framework, placement and the
/// post-write verification all run inside `taarib-mustawda`'s pipeline; this
/// command assembles its inputs from the store and the package manifest and
/// reports each stage on [`ISM_HADATH_TATHBEET`].
///
/// # Errors
///
/// [`Khata`] naming whichever gate refused: an unreadable package, a build
/// mismatch without acknowledgement, anti-cheat evidence, a revoked package,
/// or the installer's own refusals — each in its own words. Also
/// [`crate::luba_awamir::KhataLuba::JidhrSteamMajhul`] when the game is a Steam
/// game and Steam itself cannot be found, because the anti-cheat verdict would
/// then be missing the half of its evidence that only Steam's catalogue holds.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::needless_pass_by_value,
    reason = "tauri commands receive owned arguments and managed state by value"
)]
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the two acknowledgements are separate keys in the IPC payload the interface \
              already sends; folding them into one struct would change that contract"
)]
pub fn thabbit_ruqaa(
    nafidha: tauri::Window,
    muarrif: String,
    masar_malaf: String,
    iqrar_shabaka: bool,
    iqrar_taqribi: bool,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<NatijatTathbeetHie, Khata> {
    use taarib_mustawda::tathbeet_bilnaqra::{TalabNaqra, thabbit_bilnaqra};
    use taarib_tathbeet::masar_tathbeet::WadaMuhtawa;
    use taarib_tathbeet::mawdi::WajhatLuba;

    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let hali = idadat.hali();

    // The interface already hides the patch listing for a game whose publisher
    // ships Arabic, but hiding a button is not the same as refusing the action:
    // this command is reachable without it. The setting is honoured here rather
    // than ignored — it exists precisely for the publisher whose own Arabic is
    // unreadable, and a user who turned it on has said so deliberately. An
    // unchecked game passes, because "nobody looked" is not "there is Arabic".
    if !hali.istibdal_lugha_rasmiya
        && let Some(hukm) = crate::luba_awamir::hukm_mukhazzan(id)
        && hukm.hala().yatakallam_arabi()
    {
        return Err(Khata::from(KhataTathbeetAmr::LughaRasmiyaMawjuda { ism: luba.ism }));
    }

    let malaf_munazzal = PathBuf::from(&masar_malaf);
    let ruqaa = taarib_ruqaa::qari::MalafRuqaa::iftah(&malaf_munazzal)
        .map_err(|q| Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa { tafsil: q.to_string() }))?;
    let bayan = ruqaa
        .ruqaa()
        .and_then(|r| r.bayan_json())
        .map_err(|q| Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa { tafsil: q.to_string() }))?;

    let ruqaa_id: RuqaaId = qeema_bayan(&bayan, "id")?;
    let murajaa: RuqaaRevision = qeema_bayan(&bayan, "murajaa")?;
    let irtibat: taarib_tarqee::irtibat::IrtibatBina = qeema_bayan(&bayan, "irtibat")?;
    let furuq = bayan
        .get("bawwaba")
        .and_then(|b| b.get("furuq"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if furuq > 0 {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::FuruqGhayrMaduma { adad: furuq }));
    }

    let taqreer = makhzan
        .bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?
        .ok_or_else(|| Khata::min_tafsir(&KhataTathbeetAmr::LaTathbeet { ism: luba.ism.clone() }))?;
    let bina = makhzan
        .bil_qira(|ittisal| taarib_makhzan::sijillat::SijillBina::jadeed(ittisal).haliya(id))?
        .ok_or_else(|| {
            Khata::min_tafsir(&KhataTathbeetAmr::BinaMajhula { ism: luba.ism.clone() })
        })?;
    let Some(tanfidhi) = luba.tanfidhi.clone() else {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::TanfidhiMajhul { ism: luba.ism }));
    };
    let Some(masdar) = luba.masadir.first().cloned() else {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::LaTathbeet { ism: luba.ism }));
    };

    let qaima = qaimat_sahb()?;
    let sijill_iqrar = iqrar::iqra(&masar_iqrar(&masarat))?;

    let bayt_ruqaa = std::fs::read(&malaf_munazzal).map_err(|q| {
        Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa { tafsil: q.to_string() })
    })?;
    let wajha = WajhatLuba::dakhil_taarib(&format!("{ruqaa_id}.ruqaa"))
        .map_err(|q| Khata::min_tafsir(&q))?;
    let muhtawa = vec![WadaMuhtawa { wajha, bayt: bayt_ruqaa }];

    let tarif = taarib_tathbeet::bayan::TarifLuba {
        luba: id,
        masdar: masdar.clone(),
        ism: luba.ism.clone(),
        jidhr: luba.jidhr.clone(),
        ruqaa: ruqaa_id,
        murajaa,
        basma_bina: Some(bina.basma),
    };

    let ism_tanfidhi = tanfidhi
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();

    let jidhr_hajr = masarat.hajr();
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;
    let mukawwinat = masarat.mukawwinat();
    // Resolved rather than read off the settings override, and refusing when a
    // Steam game's Steam cannot be found at all: the gate below reads VAC out of
    // Steam's catalogue and out of nowhere else.
    let jidhr_steam = crate::luba_awamir::jidhr_steam_lil_fahs(&masarat, &hali, &luba)?;

    let luba_muhallala = taarib_tathbeet::tarkib::LubaMuhallala {
        jidhr: luba.jidhr.clone(),
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

    let talab = TalabNaqra {
        luba: id,
        tarif: &tarif,
        appid: crate::luba_awamir::appid_steam(&luba),
        jidhr_steam: jidhr_steam.as_deref(),
        malaf_munazzal: &malaf_munazzal,
        jidhr_hajr: &jidhr_hajr,
        jidhr_nusakh: &nusakh,
        mirsa: &taarib_khatm::MIRSAT_MALIK,
        qaima: &qaima,
        iqrar: sijill_iqrar.as_ref(),
        iqrar_shabaka,
        iqrar_taqribi,
        tanfidhi: &ism_tanfidhi,
        muhtawa,
        bina: &bina,
        irtibat: &irtibat,
        huwiya: format!("{ruqaa_id}@{murajaa}"),
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
        |marhala| {
            let _ = nafidha.emit(ISM_HADATH_TATHBEET, marhala.arabi());
        },
    )
    .map_err(|fashal| khata_naqra(&fashal))?;

    Ok(NatijatTathbeetHie {
        tawafuq_arabi: wasf_tawafuq(natija.tawafuq).to_owned(),
        adad_muhtawa: u32::try_from(natija.adad_muhtawa).unwrap_or(u32::MAX),
        tahaqquq_arabi: natija.tahaqquq.arabi().to_owned(),
        tahaqquq_salim: natija.tahaqquq.salim(),
    })
}

fn qeema_bayan<T: serde::de::DeserializeOwned>(
    bayan: &serde_json::Value,
    haql: &'static str,
) -> Result<T, Khata> {
    bayan
        .get(haql)
        .cloned()
        .and_then(|q| serde_json::from_value(q).ok())
        .ok_or_else(|| {
            Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa {
                tafsil: format!("the package manifest carries no readable {haql}"),
            })
        })
}

const fn wasf_tawafuq(qarar: taarib_tathbeet::masar_tathbeet::QararTawafuq) -> &'static str {
    use taarib_tathbeet::masar_tathbeet::QararTawafuq as Q;
    match qarar {
        Q::Tamma => "متوافقة تمامًا مع نسختك",
        Q::BiBasma => "متوافقة — التحديث الأخير لم يغيّر النصوص",
        Q::BiIqrar => "متوافقة تقريبًا، ثُبِّتت بعد إقرارك",
    }
}
