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

use crate::luba_awamir::{
    BinaHie, bina_hie, huwiya, ijlib_luba, jidhr_nusakh, muthabbat, simat_luba,
    yalzam_iqrar_shabaka,
};

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
    /// The same sentence in English.
    ///
    /// Both are sent because both are asked for: this verdict is the evidence
    /// line under the approximate-match acknowledgement, and an English session
    /// shown the Arabic one is being asked to accept a risk described in a
    /// language it did not choose. The two are produced from one
    /// [`taarib_mustawda::mutabaqa::MutabaqatRuqaa`], so they cannot come to
    /// describe different verdicts for the same row.
    pub mutabaqa_injilizi: Option<String>,
    /// Whether the client will install it at all.
    pub qabila_lil_tathbeet: bool,
    /// Whether installing requires the user to acknowledge a risk first.
    ///
    /// About the **build match** and nothing else: it is true exactly when
    /// [`MutabaqaBina`] is `Nitaq`. It says nothing whatever about multiplayer,
    /// which is [`RuqaaLuba::yalzam_iqrar_shabaka`] and lives on the enclosing
    /// row because it is a fact about the game.
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
    /// Whether installing anything into this game will want the multiplayer
    /// acknowledgement.
    ///
    /// On the row rather than on each patch, because it is a fact about the
    /// *game*: it is read off the launcher's own catalogue entry that the
    /// library scan stored, so every patch in [`Self::mudkhalat`] would carry
    /// the same value. Repeating it per patch would invite two rows of one
    /// listing to disagree about one game, and would read as a verdict the
    /// registry passed on the patch, which it is not.
    ///
    /// A hint, not the verdict. The install gate decides again by walking the
    /// game directory, so `false` here means the launcher did not say so — never
    /// that the question will not be asked.
    pub yalzam_iqrar_shabaka: bool,
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

    // One trip off the runtime for both reads: the metadata hints come out of
    // the same store, on the same connection, and the multiplayer question is
    // decided from them before the panel is drawn rather than after a refusal.
    let (luba, simat) = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || {
            let luba = ijlib_luba(&makhzan, id)?;
            let simat = simat_luba(&makhzan, id)?;
            Ok((luba, simat))
        })
        .await?
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
        yalzam_iqrar_shabaka: yalzam_iqrar_shabaka(&simat),
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
    // Keyed by the pair the interface itself uses to identify a row, because the
    // registry may list two revisions of one lineage and the ranking below
    // reorders them. Both sentences are taken from here rather than one from
    // here and one from `MudkhalTarteeb::sabab`, so a row cannot end up with an
    // Arabic verdict and an English one that disagree.
    let awsaf: BTreeMap<(RuqaaId, RuqaaRevision), WasfMutabaqa> = ahkam
        .iter()
        .map(|hukm| {
            (
                (hukm.id, hukm.murajaa),
                WasfMutabaqa { arabi: hukm.wasf_arabi(), injilizi: hukm.wasf_injilizi() },
            )
        })
        .collect();

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
                awsaf.get(&(mudkhal.ruqaa.id, mudkhal.ruqaa.murajaa)),
                khiyarat,
            )
        })
        .collect()
}

/// One match verdict as one sentence, in both languages.
///
/// A pair rather than two loose `Option<String>` arguments: they are produced
/// together from one verdict and consumed together, and two adjacent optional
/// strings at a call site are two strings that can be passed the wrong way
/// round without the compiler noticing.
#[derive(Debug, Clone)]
struct WasfMutabaqa {
    /// The Arabic sentence.
    arabi: String,
    /// The same sentence in English.
    injilizi: String,
}

/// One listing as the patches panel writes it.
fn mudkhal_hie(
    mulakhkhas: &MulakhkhasRuqaa,
    mutabaqa: Option<MutabaqaBina>,
    wasf: Option<&WasfMutabaqa>,
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
        mutabaqa_arabi: wasf.map(|wasf| wasf.arabi.clone()),
        mutabaqa_injilizi: wasf.map(|wasf| wasf.injilizi.clone()),
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

    /// The first-run statement has not been acknowledged.
    ///
    /// The safety layer asks this first, before it looks at the game at all, so
    /// it is the one refusal here that is about the product rather than about
    /// this game or this package.
    #[error("the Taarib statement has not been acknowledged")]
    IqrarNaqis,

    /// The game runs anti-cheat, and nothing in this product lifts that.
    ///
    /// The manual path's `9037` against the automatic path's `9127`: the same
    /// verdict, read out of the same scan, refusing the same thing. Both carry
    /// the evidence rather than only the conclusion, because a permanent ban on
    /// somebody's account is not a thing to assert without showing why.
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

    /// The anti-cheat check was owed Steam's catalogue and could not read it.
    ///
    /// Not a pass. VAC is declared only in the catalogue and leaves nothing in
    /// the game folder, so a scan that never opened it produces exactly the
    /// empty evidence list a clean game produces.
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
    /// One of the two refusals here the user can answer and then press again,
    /// and the only one they answer on this screen — which is precisely why it
    /// needs a code of its own. Flattened into [`Self::TathbeetFashil`] it was
    /// indistinguishable from a corrupt package or a revoked key, so the
    /// interface had to offer "review the acknowledgements" after every failure
    /// or after none.
    #[error("{ism} is multiplayer and the modification risk was not acknowledged")]
    ShabakaBilaIqrar {
        /// The game.
        ism: String,
        /// What the scan found, as the acknowledgement text quotes it, in Arabic.
        wasf_arabi: String,
        /// The same, in English.
        wasf_injilizi: String,
    },

    /// The package's signature was rejected.
    ///
    /// Separated from the generic failure because pressing again cannot change
    /// it: an unsigned, mis-signed or tampered package is refused identically
    /// every time, so an interface that offers a retry here is offering nothing.
    #[error("{injilizi}")]
    TawqeeMarfud {
        /// The signature verdict, in Arabic.
        arabi: String,
        /// The same verdict in English.
        injilizi: String,
    },

    /// The package, its lineage or its signing key is on the revocation list.
    ///
    /// Its own code for the same reason as [`Self::TawqeeMarfud`], and because a
    /// revocation is a fact about the registry rather than about this machine:
    /// nothing the user changes here lifts it.
    #[error("this package or its key was revoked: {sabab}")]
    RuqaaMulgha {
        /// Why it was revoked, as the list words it.
        sabab: String,
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
                    Self::IqrarNaqis => 34,
                    Self::TawqeeMarfud { .. } => 35,
                    Self::RuqaaMulgha { .. } => 36,
                    // The last digit is the automatic path's, deliberately: the
                    // two install routes refuse for the same three reasons out
                    // of the same scan, and `9037`/`9038`/`9039` against
                    // `9127`/`9128`/`9129` says so at a glance in a log, a
                    // diagnostics bundle and a support thread. 31 to 33 stay
                    // free so the alignment costs no headroom.
                    Self::HimayaMuktashafa { .. } => 37,
                    Self::FahsHimayaLamYajri { .. } => 38,
                    Self::ShabakaBilaIqrar { .. } => 39,
                },
        )
    }

    #[expect(
        clippy::match_same_arms,
        reason = "a severity is shared by refusals that have nothing else in common — a \
                  stalled background task and an anti-cheat detection are both `Tanbeeh` for \
                  unrelated reasons, and merging them would attach one comment to two facts \
                  and let a change to either move the other"
    )]
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
            | Self::TathbeetFashil { .. }
            | Self::TawqeeMarfud { .. }
            | Self::RuqaaMulgha { .. } => Khutura::Khatar,
            Self::MuhimmaMutawaqqifa { .. } => Khutura::Tanbeeh,
            // Two questions waiting for their answers rather than two things
            // that went wrong, on the same reading `KhataTilqaiAmr` gives the
            // multiplayer refusal. Reporting either as a fault would teach a
            // reader to expect a broken product where there is a consent
            // prompt.
            Self::IqrarNaqis | Self::ShabakaBilaIqrar { .. } => Khutura::Maluma,
            // The account is what is at stake in both, so neither is routine:
            // one names anti-cheat evidence, the other names a check that could
            // not be completed and must not be read as a pass.
            Self::HimayaMuktashafa { .. } | Self::FahsHimayaLamYajri { .. } => Khutura::Tanbeeh,
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
            Self::IqrarNaqis => {
                "لم يُقرَّ بيان تعريب بعد، ولا يُكتب شيء في أيّ لعبة قبل الإقرار به. البيان \
                 يظهر عند أوّل تشغيل؛ اقرأه ووافق عليه ثم أعد المحاولة."
                    .to_owned()
            }
            Self::HimayaMuktashafa { anwa, dalail_arabi, .. } => format!(
                "تعمل هذه اللعبة بنظام مكافحة غش ({anwa})، ولا يُثبَّت فيها تعريب: تعديل \
                 ملفاتها قد يكلّفك حظرًا دائمًا لحسابك، والحظر يلحق بالحساب لا باللعبة. \
                 لم يُكتب شيء. الدليل:\n{dalail_arabi}"
            ),
            Self::FahsHimayaLamYajri { mawdi, sabab, .. } => {
                let mawdi = mawdi.as_ref().map_or_else(
                    || "لم يُعرف موضع تثبيت ستيم على هذا الجهاز".to_owned(),
                    |mawdi| format!("تعذّرت قراءة {mawdi} ({sabab})"),
                );
                format!(
                    "لم يُستكمل فحص مكافحة الغش، فلم يُثبَّت شيء. حماية VAC لا تُعلَن إلا في \
                     فهرس متجر ستيم ولا تترك أثرًا في مجلّد اللعبة، فسكوت الفحص هنا ليس \
                     براءة. {mawdi}. حدِّد مجلد ستيم في الإعدادات ← المنصّات ثم أعد المحاولة."
                )
            }
            Self::ShabakaBilaIqrar { wasf_arabi, .. } => format!(
                "هذه لعبة متعدّدة اللاعبين، ويلزم إقرارك بمخاطر التعديل قبل التثبيت. تعديل \
                 لعبة تُلعب مع آخرين قد يُفقدك حسابك أو يمنعك من الخوادم، والقرار قرارك وحدك. \
                 لم يُكتب شيء.\n{wasf_arabi}"
            ),
            Self::TawqeeMarfud { arabi, .. } => format!(
                "{arabi} لم يُكتب شيء، وإعادة المحاولة بالملفّ نفسه ستُرفض بالنتيجة نفسها؛ \
                 أعد تنزيل الرقعة من المستودع."
            ),
            Self::RuqaaMulgha { sabab } => format!(
                "أُبطلت هذه الحزمة أو مفتاح توقيعها في قائمة الإبطال: {sabab}. الإبطال قرار \
                 من المستودع لا يُلغى من هذا الجهاز؛ اختر رقعة أخرى لهذه اللعبة."
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
            Self::IqrarNaqis => {
                "The Taarib statement has not been acknowledged, and nothing is written into any \
                 game until it is. It is shown on first run; read it, accept it, and try again."
                    .to_owned()
            }
            Self::HimayaMuktashafa { anwa, dalail_injilizi, .. } => format!(
                "This game runs anti-cheat ({anwa}), and Taarib does not install into such a \
                 title: modifying its files can cost you a permanent ban, and the ban attaches \
                 to your account rather than to the game. Nothing was written. Evidence:\n\
                 {dalail_injilizi}"
            ),
            Self::FahsHimayaLamYajri { mawdi, sabab, .. } => {
                let mawdi = mawdi.as_ref().map_or_else(
                    || "no Steam installation could be located on this machine".to_owned(),
                    |mawdi| format!("{mawdi} could not be read ({sabab})"),
                );
                format!(
                    "The anti-cheat check did not finish, so nothing was installed. VAC is \
                     declared only in Steam's catalogue and leaves nothing in the game folder, \
                     so silence here is not a clean result. {mawdi}. Set Steam's folder in \
                     Settings, under Launchers, and try again."
                )
            }
            Self::ShabakaBilaIqrar { wasf_injilizi, .. } => format!(
                "This is a multiplayer game, and the modification risk has to be acknowledged \
                 before anything is installed. Modifying a game played with other people can \
                 cost you your account or your access to its servers, and that decision is \
                 yours alone. Nothing was written.\n{wasf_injilizi}"
            ),
            Self::TawqeeMarfud { injilizi, .. } => format!(
                "{injilizi} Nothing was written, and pressing again with the same file earns \
                 the same refusal; download the patch again from the registry."
            ),
            Self::RuqaaMulgha { sabab } => format!(
                "This package or its signing key is on the revocation list: {sabab}. A \
                 revocation is the registry's decision and cannot be lifted from this machine; \
                 choose another patch for this game."
            ),
        }
    }

    #[expect(
        clippy::match_same_arms,
        reason = "`LaShay` is reached by two different arguments — nothing to do because the \
                  sentence is complete, and no button because offering one would answer a \
                  consent question for the user — and the comments below are the reason each \
                  arm is where it is"
    )]
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
            // The package is in hand and refused; the diagnostics bundle is
            // what a report of any of the three carries, and none of them is
            // fixed by pressing the button again.
            Self::TathbeetFashil { .. }
            | Self::TawqeeMarfud { .. }
            | Self::RuqaaMulgha { .. } => Khutwa::FathTashkhis,
            // The remedy is a setting, and it is the only one this refusal has.
            Self::LughaRasmiyaMawjuda { .. } => Khutwa::FathIdadat { qism: QismIdadat::Lugha },
            // The manual Steam path is the one way out, and it is the same one
            // `KhataLuba::JidhrSteamMajhul` sends the reader to.
            Self::FahsHimayaLamYajri { .. } => {
                Khutwa::FathIdadat { qism: QismIdadat::Manassat }
            }
            // Three refusals with no button, for three different reasons and one
            // shared rule: none of them may be clicked past *here*. Anti-cheat is
            // lifted by nothing at all — no override for it exists anywhere in
            // this product, and this would be the first one. The other two are
            // lifted by an acknowledgement the person gives, and an action
            // offering to give it would be giving it for them; the interface
            // reopens the question off the code instead, which is what the code
            // is for. Each sentence names its own way out, or says plainly that
            // there is none.
            Self::HimayaMuktashafa { .. }
            | Self::IqrarNaqis
            | Self::ShabakaBilaIqrar { .. } => Khutwa::LaShay,
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
            // Neither carries a fact worth a column: one is a mode the user set
            // and the other is a statement they have not read yet.
            Self::GhayrMuttasil | Self::IqrarNaqis => {}
            Self::MuhimmaMutawaqqifa { tafsil } | Self::HuzmaTalifa { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            }
            Self::FuruqGhayrMaduma { adad } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Hajm(*adad));
            }
            Self::TathbeetFashil { injilizi, .. } | Self::TawqeeMarfud { injilizi, .. } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(injilizi.clone()));
            }
            Self::LughaRasmiyaMawjuda { ism } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            }
            Self::RuqaaMulgha { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            }
            // The same keys the automatic path writes for the same three facts,
            // so one log filter reads both routes.
            Self::HimayaMuktashafa { ism, anwa, .. } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("himaya".to_owned(), QeemaSiyaq::Nass(anwa.clone()));
            }
            Self::FahsHimayaLamYajri { ism, mawdi, sabab } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
                if let Some(mawdi) = mawdi {
                    let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(mawdi.clone()));
                }
            }
            Self::ShabakaBilaIqrar { ism, wasf_injilizi, .. } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ =
                    siyaq.insert("shabaka".to_owned(), QeemaSiyaq::Nass(wasf_injilizi.clone()));
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

/// One one-click install failure, as the interface's error type.
///
/// The safety layer's refusals are unpacked into their own codes; everything
/// else keeps the single [`KhataTathbeetAmr::TathbeetFashil`] it always had,
/// because the quarantine and installer errors underneath it already carry their
/// own sentences and there is nothing here to discriminate between.
fn khata_naqra(
    fashal: &taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet,
    ism: &str,
) -> Khata {
    use taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet;

    match fashal {
        FashalTathbeet::Aman(rafd) => Khata::min_tafsir(&khata_rafd(rafd, ism)),
        FashalTathbeet::Mustawda(_) | FashalTathbeet::Tathbeet(_) => {
            Khata::min_tafsir(&KhataTathbeetAmr::TathbeetFashil {
                arabi: fashal.arabi(),
                injilizi: fashal.injilizi(),
            })
        }
    }
}

/// One safety refusal, as the refusal it actually is.
///
/// Every arm of [`taarib_aman::fahs::Rafd`] gets its own code rather than the
/// one `TAARIB-E-9029` they all used to collapse into. The reason is not tidiness:
/// a code is the only thing on the wire a screen can branch on — `siyaq` is
/// rendered, never matched, everywhere in this product — and the six refusals
/// have four different remedies between them. Without the split the manual
/// install path could not tell "you have not accepted the multiplayer risk",
/// which one tick and one press fixes, from a revoked signing key, which nothing
/// on this machine fixes; so it offered the same generic affordance for both.
///
/// The match is exhaustive on purpose. A seventh refusal added to the safety
/// layer must be answered here rather than falling silently into a bucket that
/// tells the user nothing.
///
/// None of this lifts anything. Every arm still refuses, the sentences still
/// name the evidence, and anti-cheat still ends at [`Khutwa::LaShay`] with no
/// override anywhere.
fn khata_rafd(rafd: &taarib_aman::fahs::Rafd, ism: &str) -> KhataTathbeetAmr {
    use taarib_aman::fahs::Rafd;
    use taarib_aman::kashf_himaya::DaleelHimaya;

    match rafd {
        Rafd::IqrarNaqis => KhataTathbeetAmr::IqrarNaqis,
        Rafd::Himaya(ijmaa) => KhataTathbeetAmr::HimayaMuktashafa {
            ism: ism.to_owned(),
            anwa: asma_himaya(ijmaa),
            dalail_arabi: sutur(ijmaa.adilla.iter().map(DaleelHimaya::arabi)),
            dalail_injilizi: sutur(ijmaa.adilla.iter().map(DaleelHimaya::injilizi)),
        },
        Rafd::FahsMatjarLamYajri { masar, sabab } => KhataTathbeetAmr::FahsHimayaLamYajri {
            ism: ism.to_owned(),
            mawdi: masar.as_ref().map(|masar| masar.display().to_string()),
            sabab: sabab.clone(),
        },
        Rafd::Tawqee(sabab) => {
            KhataTathbeetAmr::TawqeeMarfud { arabi: sabab.arabi(), injilizi: sabab.injilizi() }
        }
        Rafd::Mulgha { sabab } => KhataTathbeetAmr::RuqaaMulgha { sabab: sabab.clone() },
        Rafd::ShabakaBilaIqrar(ijmaa) => KhataTathbeetAmr::ShabakaBilaIqrar {
            ism: ism.to_owned(),
            wasf_arabi: ijmaa.wasf_iqrar(),
            wasf_injilizi: ijmaa.wasf_injilizi(),
        },
    }
}

/// The anti-cheats a scan named, joined for the log and the context table.
fn asma_himaya(ijmaa: &taarib_aman::kashf_himaya::IjmaaHimaya) -> String {
    ijmaa
        .anwa()
        .into_iter()
        .map(taarib_aman::kashf_himaya::NawHimaya::injilizi)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Evidence lines, one to a line, as the refusal sentences interpolate them.
fn sutur(satrat: impl Iterator<Item = String>) -> String {
    satrat.map(|satr| format!("- {satr}")).collect::<Vec<_>>().join("\n")
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
/// or the installer's own refusals — each in its own words. The safety layer's
/// six refusals carry their own codes rather than one shared code, so a screen
/// can tell the one the user answers ([`KhataTathbeetAmr::ShabakaBilaIqrar`],
/// `TAARIB-E-9039`) from the ones nobody can. Also
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
    .map_err(|fashal| khata_naqra(&fashal, &luba.ism))?;

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ikhtibarat {
    use std::collections::BTreeSet;

    use taarib_aman::fahs::Rafd;
    use taarib_aman::kashf_himaya::{
        DaleelHimaya, IjmaaHimaya, NawDaleel as NawDaleelHimaya, NawHimaya, Thiqa,
    };
    use taarib_aman::kashf_shabaka::{DalalatShabaka, DaleelShabaka, IjmaaShabaka, NawDaleel};
    use taarib_aman::tahaqquq_tawqee::SababTawqee;
    use taarib_mustalahat::bina::{Basma, BinaId};
    use taarib_mustalahat::luba::{MasdarLuba, SuwarLuba};
    use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
    use taarib_mustalahat::musahim::MusahimId;
    use taarib_mustalahat::ruqaa::{RukhsaRuqaa, TareeqaTarjama};
    use taarib_usus::manassa::BeeatTawafuq;

    use super::*;

    /// Anything a test here can fail on: a refusal from the code under test, or
    /// a fixture that would not build. `unwrap` and `expect` are denied
    /// workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn std::error::Error>>;

    /// A contributor identity, which is 64 lowercase hexadecimal characters and
    /// nothing else.
    const MUSAHIM: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    /// The game every refusal below is about.
    const ISM: &str = "Luba Ikhtibar";

    /// Whether a sentence carries any Arabic script at all.
    ///
    /// The assertion the English field exists for: a reader who chose English is
    /// shown a sentence with no Arabic in it, rather than the Arabic one under
    /// an English heading.
    fn fiha_arabi(nass: &str) -> bool {
        nass.chars().any(|harf| {
            matches!(harf, '\u{0600}'..='\u{06ff}' | '\u{0750}'..='\u{077f}')
        })
    }

    /// A registry listing bound to whichever builds the caller names.
    fn mulakhkhas(
        manassat: Vec<String>,
        basmat: Vec<Basma>,
        waqt_nashr: &str,
    ) -> Result<MulakhkhasRuqaa, Box<dyn std::error::Error>> {
        Ok(MulakhkhasRuqaa {
            id: RuqaaId::min_uuid(uuid::Uuid::new_v4()),
            murajaa: RuqaaRevision::AWWAL,
            unwan: "رقعة اختبار".to_owned(),
            musahim: MusahimId::jadeed(MUSAHIM)?,
            ism_musahim: "مساهم اختبار".to_owned(),
            taghtiya: Taghtiya::default(),
            adad_nusus: 100,
            hajm: 1024,
            bina_manassa: manassat,
            basmat,
            aila: AilatMuharrik::Unity,
            khalfiya: KhalfiyaBarmajiya::Mono,
            tabaqa: Tabaqa::Kamil,
            tareeqa: TareeqaTarjama::AaliyaFaqat,
            rukhsa: RukhsaRuqaa::Cc0,
            taqyeem: None,
            adad_taqyeemat: 0,
            waqt_nashr: waqt_nashr.to_owned(),
            basmat_muhtawa: Basma::min_bayt([7u8; 32]),
            rabt: "https://example.invalid/ruqaa".to_owned(),
            rabt_mira: None,
        })
    }

    /// A library row whose build is on record, so the listings get judged.
    fn luba_bi_bina(manassa: Option<&str>, basma: Basma) -> Luba {
        let masdar = MasdarLuba::Steam(480);
        Luba {
            id: LubaId::min_masdar(&masdar, ISM),
            masadir: vec![masdar],
            ism: ISM.to_owned(),
            jidhr: PathBuf::from("/luba-ikhtibar"),
            tanfidhi: None,
            hajm: 0,
            akhir_laab: None,
            akhir_tahdith: None,
            bina: Some(BinaId {
                manassa: manassa.map(ToOwned::to_owned),
                basma,
                adad_malaffat: 1_000,
                waqt: "2026-01-01T00:00:00Z".to_owned(),
            }),
            suwar: SuwarLuba::default(),
            beea: BeeatTawafuq::Asli,
            mawjuda: true,
            mukhfiya: false,
        }
    }

    /// A multiplayer scan that found one piece of catalogue evidence.
    fn ijmaa_shabaka() -> IjmaaShabaka {
        IjmaaShabaka {
            jidhr: PathBuf::from("/luba-ikhtibar"),
            dalail: vec![DaleelShabaka {
                naw: NawDaleel::BayanMatjar,
                dalala: DalalatShabaka::MutaaddidOnline,
                ayn: "category/1".to_owned(),
                masar: None,
            }],
            thughrat: Vec::new(),
            mabtur: false,
        }
    }

    /// An anti-cheat scan that found one certain marker on disk.
    fn ijmaa_himaya() -> IjmaaHimaya {
        IjmaaHimaya {
            jidhr: PathBuf::from("/luba-ikhtibar"),
            adilla: vec![DaleelHimaya {
                naw: NawHimaya::EasyAntiCheat,
                sinf: NawDaleelHimaya::MalafMawjud,
                ayn: "EasyAntiCheat/easyanticheat.sys".to_owned(),
                masar: None,
                thiqa: Thiqa::Muakkada,
            }],
            thughrat: Vec::new(),
            mabtur: false,
        }
    }

    /// The match verdict reaches the interface in both languages, on the right
    /// row.
    #[test]
    fn wasf_almutabaqa_yasil_bil_lughatayn_ala_saffihi() -> NatijatIkhtibar {
        let basma = Basma::min_bayt([3u8; 32]);
        let luba = luba_bi_bina(Some("12345"), basma);

        // Worst first on purpose. The ranking puts the fingerprint match above
        // the approximate one, so a lookup that trusted the input order would
        // hand each row the other row's sentence — which is the only way this
        // change can be wrong, and the reason the sentences are keyed by the
        // lineage-and-revision pair rather than by position.
        let taqribi =
            mulakhkhas(vec!["12345".to_owned()], Vec::new(), "2026-01-02T00:00:00Z")?;
        let mutabiq = mulakhkhas(Vec::new(), vec![basma], "2026-01-01T00:00:00Z")?;
        let (id_taqribi, id_mutabiq) = (taqribi.id, mutabiq.id);

        let mudkhalat = rattib_murashshahat(&luba, vec![taqribi, mutabiq]);
        assert_eq!(mudkhalat.len(), 2);

        let awwal = mudkhalat.first().ok_or("the ranking dropped a listing")?;
        let thani = mudkhalat.get(1).ok_or("the ranking dropped a listing")?;
        assert_eq!(awwal.id, id_mutabiq, "the fingerprint match ranks first");
        assert_eq!(thani.id, id_taqribi);
        assert_eq!(awwal.mutabaqa, Some(MutabaqaBina::Basma));
        assert_eq!(thani.mutabaqa, Some(MutabaqaBina::Nitaq));
        assert!(!awwal.yahtaj_iqrar, "an exact-enough match asks nothing");
        assert!(thani.yahtaj_iqrar, "an approximate match asks for an acknowledgement");

        for mudkhal in &mudkhalat {
            let tabaqa = mudkhal.mutabaqa.ok_or("a judged listing lost its tier")?;
            let arabi = mudkhal.mutabaqa_arabi.as_deref().ok_or("no Arabic verdict")?;
            let injilizi =
                mudkhal.mutabaqa_injilizi.as_deref().ok_or("no English verdict")?;
            assert!(arabi.contains(tabaqa.wasf_arabi()), "{arabi}");
            assert!(injilizi.contains(tabaqa.wasf_injilizi()), "{injilizi}");
            assert!(
                !fiha_arabi(injilizi),
                "an English verdict must hold no Arabic: {injilizi}"
            );
        }
        Ok(())
    }

    /// With no build on record nothing is judged, in either language.
    #[test]
    fn bila_bina_la_hukm_bi_ayy_lugha() -> NatijatIkhtibar {
        let mut luba = luba_bi_bina(Some("12345"), Basma::min_bayt([3u8; 32]));
        luba.bina = None;
        let listing =
            mulakhkhas(vec!["12345".to_owned()], Vec::new(), "2026-01-01T00:00:00Z")?;

        let mudkhalat = rattib_murashshahat(&luba, vec![listing]);

        let wahid = mudkhalat.first().ok_or("the listing was dropped")?;
        assert_eq!(wahid.mutabaqa, None);
        assert_eq!(wahid.mutabaqa_arabi, None);
        // Symmetric with the Arabic one deliberately: a verdict invented from an
        // absence is no better in English than it is in Arabic.
        assert_eq!(wahid.mutabaqa_injilizi, None);
        Ok(())
    }

    /// Every safety refusal is told apart from every other by its code alone.
    #[test]
    fn kull_rafd_yahmil_ramzan_yakhussuhu() {
        let rufud = [
            Rafd::IqrarNaqis,
            Rafd::Himaya(Box::new(ijmaa_himaya())),
            Rafd::FahsMatjarLamYajri { masar: None, sabab: "NotFound".to_owned() },
            Rafd::Tawqee(SababTawqee::GhayrMuwaqqaa),
            Rafd::Mulgha { sabab: "the signing key was withdrawn".to_owned() },
            Rafd::ShabakaBilaIqrar(Box::new(ijmaa_shabaka())),
        ];

        let rumuz: Vec<u16> =
            rufud.iter().map(|rafd| khata_rafd(rafd, ISM).ramz().raqm()).collect();
        let mufrada: BTreeSet<u16> = rumuz.iter().copied().collect();

        assert_eq!(mufrada.len(), rumuz.len(), "two refusals share one code: {rumuz:?}");
        // The bucket they all used to collapse into. Nothing that came out of
        // the safety layer may still be wearing it.
        assert!(
            !rumuz.contains(&(arqam::STUDIO + 29)),
            "a safety refusal is still answering with the generic install code"
        );
    }

    /// The refusal the user can answer says so with a code, not with a sentence
    /// somebody has to match on.
    #[test]
    fn rafd_alshabaka_lahu_ramz_yumakkin_min_iadat_alsual() {
        let rafd = Rafd::ShabakaBilaIqrar(Box::new(ijmaa_shabaka()));
        let khata = Khata::min_tafsir(&khata_rafd(&rafd, ISM));

        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 39);
        assert_eq!(khata.khutura, Khutura::Maluma);
        // Not a button: the acknowledgement is the person's to give, and an
        // action offering to give it would be giving it for them. The interface
        // reopens its own question off the code above.
        assert_eq!(khata.khutwa, Khutwa::LaShay);
        assert!(fiha_arabi(&khata.arabi));
        assert!(!fiha_arabi(&khata.injilizi), "{}", khata.injilizi);
        // The evidence the acknowledgement quotes, so the question can be put
        // again with what it is about beside it.
        assert!(khata.injilizi.contains("multiplayer"));
        assert_eq!(khata.siyaq.get("ism"), Some(&QeemaSiyaq::Nass(ISM.to_owned())));
    }

    /// Naming the anti-cheat refusal did not give it a way out.
    #[test]
    fn rafd_alhimaya_yabqa_bila_makhraj() {
        let rafd = Rafd::Himaya(Box::new(ijmaa_himaya()));
        let khata = Khata::min_tafsir(&khata_rafd(&rafd, ISM));

        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 37);
        // No override for anti-cheat exists anywhere in this product, and
        // giving the refusal a code of its own must not have invented the first.
        assert_eq!(khata.khutwa, Khutwa::LaShay);
        assert_eq!(khata.khutura, Khutura::Tanbeeh);
        // The evidence, not only the conclusion.
        assert!(khata.injilizi.contains(NawHimaya::EasyAntiCheat.injilizi()));
        assert!(khata.arabi.contains("easyanticheat.sys"));
    }

    /// The unread-catalogue refusal keeps the one remedy it has.
    #[test]
    fn rafd_fahs_almatjar_yadullu_ala_idadat_almanassat() {
        let rafd = Rafd::FahsMatjarLamYajri {
            masar: Some(PathBuf::from("/steam/appcache/appinfo.vdf")),
            sabab: "NotFound".to_owned(),
        };
        let khata = Khata::min_tafsir(&khata_rafd(&rafd, ISM));

        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 38);
        assert_eq!(khata.khutwa, Khutwa::FathIdadat { qism: QismIdadat::Manassat });
        // The path is the mistake on this machine, so the refusal names it.
        assert!(khata.injilizi.contains("appinfo.vdf"), "{}", khata.injilizi);
        assert_eq!(
            khata.siyaq.get("masar"),
            Some(&QeemaSiyaq::Nass("/steam/appcache/appinfo.vdf".to_owned()))
        );
    }

    /// A refusal from outside the safety layer still answers with the generic
    /// install code, which is the whole of what it can say.
    #[test]
    fn fashal_ghayr_amni_yabqa_ala_alramz_alaam() {
        let fashal = taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet::Tathbeet(
            KhataTathbeet::LubaTashtaghil {
                amaliya: "luba.exe".to_owned(),
                tanfidhi: PathBuf::from("/luba-ikhtibar/luba.exe"),
            },
        );

        let khata = khata_naqra(&fashal, ISM);

        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 29);
    }
}
