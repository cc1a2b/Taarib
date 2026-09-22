//! التثبيت — the patches offered for a game, the download, the check, and the removal.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use taarib_aman::KhataAman;
use taarib_aman::iqrar::{self, SijillIqrar};
use taarib_aman::qaimat_sahb::{QaimaMuraqaba, QaimatSahb};
use taarib_khatm::MiftahAam;
use taarib_makhzan::sijillat::{SijillBina, SijillMuharrik, SijillRuqaa, SijillTathbeet};
use taarib_makhzan::wasl::{Makhzan, alaan};
use taarib_mustalahat::bina::{BinaId, MutabaqaBina};
use taarib_mustalahat::luba::{Luba, LubaId};
use taarib_mustalahat::muharrik::{Tabaqa, TaqreerImkaniyat};
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_mustawda::masadir::{MasdarMustawda, SilsilatMasadir};
use taarib_mustawda::mutabaqa::MutabiqBina;
use taarib_mustawda::sahb::{NatijatTajdid, jaddid_qaimat_sahb, jaddid_qaimat_sahb_bi_bayan};
use taarib_mustawda::tanzeel::{self, MukhbirTaqaddum, TalabTanzeel, Taqaddum, nazzil};
use taarib_mustawda::tarteeb::{FiatTaqyeem, KhiyaratTarteeb, MudkhalTarteeb, rattib};
use taarib_mustawda::{FahrasMajlub, MarhalatTanzeel, jalb_fahras};
use taarib_tarqee::irtibat::MukhattatBasma;
use taarib_tathbeet::bayan::{NawTathbeet, Tathbeet};
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::masar_tathbeet::la_tashtaghil;
use taarib_tathbeet::tahaqquq::{TaqreerTahaqquq, tahaqquq_kamil};
use taarib_tathbeet::taraju::{
    KhuttatIstiada, RadLaShay, SiyasatIstiada, TaqreerIstiada, TaqreerKul, istiada_kul,
    istiada_nass, istiada_sawt, nazzif_nusakh,
};
use taarib_tathbeet::tarkib::{HajatItar, KhuttatTarkib, LubaMuhallala, NawMudkhal, TalabItlaq};
use taarib_tathbeet::wukala::WakeelQaim;
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{
    Khata, Khutura, Khutwa, MasarMatlub, Natija, QeemaSiyaq, QismIdadat, Ramz, Tafsir, arqam,
};
use taarib_usus::khata_min;
use taarib_usus::masarat::Masarat;
use taarib_usus::{ISDAR, Lugha};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum MatlabIzala {
    /// The text installation only.
    Nass,
    /// The voice installation only.
    Sawt,
    /// Both, independently.
    Kul,
}

/// How far a removal may go, as the user answered it.
///
/// Three, because the installer has three and the middle one was the only one
/// the interface could ask for. A game that ran once with a vendored loader
/// comes back holding that loader's log, cache and configuration — written
/// after the manifest was sealed, so no record names them — and the
/// conservative answer leaves the directory and keeps the record open forever.
/// [`SiyasatIzala::Kanasa`] is what finishes it, and it is a separate value
/// rather than a flag on the other two because the screen must have seen
/// `khuttat_izala`'s residue list before a person can mean it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SiyasatIzala {
    /// Leave a file the store replaced, and leave a directory that still holds
    /// files Taarib did not write. The default answer.
    Muhafiza,
    /// Refuse the whole removal when the store has replaced a patched file.
    Sarima,
    /// Also take what is left inside a directory Taarib created, and the
    /// directory with it.
    Kanasa,
}

impl SiyasatIzala {
    /// The installer's policy for this answer.
    const fn asliya(self) -> SiyasatIstiada {
        match self {
            Self::Muhafiza => SiyasatIstiada::Muhafiza,
            Self::Sarima => SiyasatIstiada::Sarima,
            Self::Kanasa => SiyasatIstiada::Kanasa,
        }
    }
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
    /// Which of the three products it installs, as the discriminant.
    ///
    /// Each row carries its own install button, so each row has to name what
    /// pressing it produces — and it has to do that in the reader's language.
    /// [`Self::tabaqa_raqm`] is a number and [`Self::tabaqa_arabi`] is Arabic;
    /// neither lets an English session say which product this patch is without
    /// a second copy of the tier taxonomy in TypeScript.
    pub tabaqa: Tabaqa,
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

/// What one directory Taarib created still holds that Taarib never put there.
///
/// Named rather than counted, because the choice the sweep offers is a choice
/// about *these files*: a framework's own log, a mod's configuration, a save a
/// loader wrote beside itself. A number cannot be consented to.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct BaqiyaMujalladHie {
    /// The directory, relative to the game root.
    pub mujallad: String,
    /// The unrecorded entries inside it, sorted, a directory carrying a trailing
    /// slash. Capped by the installer, so this may be shorter than
    /// [`Self::adad`].
    pub madakhil: Vec<String>,
    /// How many unrecorded entries there are in total.
    pub adad: u32,
}

/// What removing one installation would do, computed before it is done.
///
/// A projection of one [`taarib_tathbeet::taraju::KhuttatIstiada`]. The dry run
/// existed and was called by nothing, so the removal confirmation asked for a
/// decision — sweep the directories or keep them — while showing neither what
/// would be swept nor what the store had already replaced.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhuttatIzalaHie {
    /// Which installation this is about.
    pub naw: NawTathbeetHie,
    /// When it was installed, RFC 3339.
    pub waqt_tathbeet: String,
    /// How many of the game's own files would be written back.
    pub li_istiada: u32,
    /// How many files Taarib added would be deleted.
    pub li_hadhf: u32,
    /// How many created directories would be considered for removal.
    pub mujalladat: u32,
    /// Paths the store has already replaced since the install, which a removal
    /// leaves exactly as they are.
    pub mustabdala: Vec<String>,
    /// Recorded paths that are not on disk at all.
    pub mafquda: Vec<String>,
    /// What is inside the created directories that no manifest line names.
    pub baqaya: Vec<BaqiyaMujalladHie>,
    /// How many bytes the preserved originals occupy, which removing frees.
    #[specta(type = specta_typescript::Number)]
    pub hajm_nusakh: u64,
    /// The same size as the interface writes it.
    pub hajm_nusakh_maqru: String,
    /// Whether removing this would leave the game byte-for-byte as it shipped.
    pub nazif: bool,
    /// The dry run's own report text, line for line.
    pub sutur: Vec<String>,
}

impl KhuttatIzalaHie {
    /// One dry run, projected for the confirmation that asks about it.
    fn min_asli(naw: NawTathbeet, khutta: &KhuttatIstiada) -> Self {
        Self {
            naw: NawTathbeetHie::min_asli(naw),
            waqt_tathbeet: khutta.waqt_tathbeet.clone(),
            li_istiada: adad(khutta.li_istiada),
            li_hadhf: adad(khutta.li_hadhf),
            mujalladat: adad(khutta.mujalladat),
            mustabdala: khutta.mustabdala.clone(),
            mafquda: khutta.mafquda.clone(),
            baqaya: khutta
                .baqaya
                .iter()
                .map(|baqiya| BaqiyaMujalladHie {
                    mujallad: baqiya.mujallad.clone(),
                    madakhil: baqiya.madakhil.clone(),
                    adad: adad(baqiya.adad),
                })
                .collect(),
            hajm_nusakh: khutta.hajm_nusakh,
            hajm_nusakh_maqru: tanzeel::hajm_maqru(khutta.hajm_nusakh),
            nazif: khutta.nazif(),
            sutur: khutta.taqreer(),
        }
    }
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
    /// The same statement in English, so a session running in English is not
    /// asked to accept words it cannot read.
    pub nass_injilizi: String,
    /// When the acknowledgement was given, RFC 3339.
    pub waqt: Option<String>,
    /// Which build of Taarib asked.
    pub isdar_taarib: Option<String>,
    /// Which of the two renderings was on screen when it was given, or [`None`]
    /// for a record written before that was recorded.
    pub lugha_nass: Option<Lugha>,
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
    let masadir: Vec<String> = silsila.masadir().iter().map(MasdarMustawda::wasf).collect();

    let fahras = jalb_fahras(&silsila, &[id], &makhbaa, None).await?;
    // The revocation list rides on the manifest fetch that just happened. The
    // page that offers the install button is the moment before the install,
    // and the gate behind that button reads the cache on a thread that must
    // not wait for a network; this is where the cache is made current for it.
    let _ = jaddid_sahb_bi_bayan(&masarat, &silsila, &fahras).await;
    dhamin_mujaddid_sahb(&masarat, &idadat);
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
        mudkhalat
            .iter()
            .filter(|mudkhal| mudkhal.qabila_lil_tathbeet)
            .count(),
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
    // The download is the last asynchronous step before the one-click install,
    // so the list the install's gate reads is at most this fetch old.
    let _ = jaddid_sahb_bi_bayan(&masarat, &silsila, &fahras).await;
    dhamin_mujaddid_sahb(&masarat, &idadat);
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
pub async fn tahaqquq_ruqaa(
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
/// `siyasa` carries how far the user said the removal may go. The sweeping
/// answer is the only one that can take a file no manifest names, and the
/// screen may only send it after showing what `khuttat_izala` found — which is
/// why it is asked for per removal rather than stored as a setting.
///
/// # Errors
///
/// [`KhataTathbeetAmr::LaTathbeet`] when the game has no installation of the
/// requested kind, and [`KhataTathbeet::LubaTashtaghil`] when the game is
/// running. A restore that fails is carried inside the result rather than
/// raised, because the other kind's outcome is still owed to the user.
#[tauri::command]
#[specta::specta]
pub async fn azil_ruqaa(
    muarrif: String,
    matlab: MatlabIzala,
    siyasa: SiyasatIzala,
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

    let siyasa = siyasa.asliya();
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

    Ok(HasilatIzala {
        najahat,
        nass,
        sawt,
        akhta,
        hajm_muharrar,
        sutur,
    })
}

/// What removing an installation would do, before it is done.
///
/// The dry run [`taarib_tathbeet::taraju::khutta`] has always computed and
/// nobody has ever been shown. It matters most for the one choice on this screen
/// that can destroy something the user did not put there: the removal offers to
/// sweep the directories Taarib created, and a sweep can only be consented to if
/// what is inside them is named first. It also names the paths the store has
/// replaced since the install, which a removal deliberately leaves alone — so
/// "the game is back to how it shipped" is claimed only when it is true.
///
/// Nothing is written. An installation that is not there is simply absent from
/// the answer rather than an error, because the confirmation asks about
/// whichever of the two are present.
///
/// # Errors
///
/// Whatever reading a manifest raises. A game with no installation at all
/// answers with an empty list.
#[tauri::command]
#[specta::specta]
pub async fn khuttat_izala(
    muarrif: String,
    matlab: MatlabIzala,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<Vec<KhuttatIzalaHie>, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;

    let matlub: &[NawTathbeet] = match matlab {
        MatlabIzala::Nass => &[NawTathbeet::Nass],
        MatlabIzala::Sawt => &[NawTathbeet::Sawt],
        MatlabIzala::Kul => &[NawTathbeet::Nass, NawTathbeet::Sawt],
    };

    let mut khutat = Vec::with_capacity(matlub.len());
    for naw in matlub {
        if !Tathbeet::mawjud(&nusakh, *naw) {
            continue;
        }
        let khutta =
            taarib_tathbeet::taraju::khutta(&luba.jidhr, &nusakh, *naw).map_err(Khata::from)?;
        khutat.push(KhuttatIzalaHie::min_asli(*naw, &khutta));
    }
    Ok(khutat)
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
        Err(KhataTathbeet::LubaTashtaghil {
            amaliya,
            tanfidhi: masar,
        }) => Ok(HalatTashghil {
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
/// `lugha` is the rendering the panel actually drew, which the caller sends
/// rather than the backend reading it out of the settings: a session can be
/// showing one language while the stored preference says another, and the
/// record has to name the words the person read.
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
    lugha: Lugha,
) -> Result<HalatIqrar, Khata> {
    let waqt = makhzan.bil_qira(alaan)?;
    let sijill =
        iqrar::ahfaz_bi_lugha(&masar_iqrar(&masarat), waqt, ISDAR.to_owned(), Some(lugha))?;
    tracing::info!(
        isdar_nass = sijill.isdar_nass,
        lugha = lugha.wasm(),
        "the safety statement was acknowledged"
    );
    Ok(iqrar_hie(Some(&sijill)))
}

// ---------------------------------------------------------------------------
// Registry sources, matching and ranking
// ---------------------------------------------------------------------------

/// The sources to try, in the order they are tried.
///
/// Local copies first: they cost nothing, they work with no network, and a user
/// who configured one did so to be asked before the forge is.
pub(crate) fn silsilat_masadir(idadat: &Idadat) -> Natija<SilsilatMasadir> {
    let mut masadir: Vec<MasdarMustawda> = idadat
        .masadir
        .mahalliya
        .iter()
        .map(|jidhr| MasdarMustawda::MujalladMahalli {
            jidhr: jidhr.clone(),
        })
        .collect();

    if !idadat.masadir.wadaa_ghayr_muttasil {
        let rasmi = idadat.masadir.rasmi.trim();
        if !rasmi.is_empty() {
            masadir.push(MasdarMustawda::Shabaka {
                jidhr: rasmi.to_owned(),
            });
        }
        for mira in &idadat.masadir.maraya {
            let mira = mira.trim();
            if !mira.is_empty() {
                masadir.push(MasdarMustawda::Mira {
                    jidhr: mira.to_owned(),
                });
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
                WasfMutabaqa {
                    arabi: hukm.wasf_arabi(),
                    injilizi: hukm.wasf_injilizi(),
                },
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
        tabaqa: mulakhkhas.tabaqa,
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
        munharifa: taqreer
            .masarat_munharifa()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
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
        nass_injilizi: iqrar::NASS_INJILIZI.to_owned(),
        waqt: sijill.map(|wahid| wahid.waqt.clone()),
        isdar_taarib: sijill.map(|wahid| wahid.isdar_taarib.clone()),
        lugha_nass: sijill.and_then(|wahid| wahid.lugha_nass),
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
fn sajjil_izala(makhzan: &Makhzan, luba: &Luba, jidhr_nusakh: &Path, id: LubaId) -> Natija<()> {
    if NawTathbeet::KULL
        .into_iter()
        .any(|naw| muthabbat(&luba.jidhr, jidhr_nusakh, naw))
    {
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
pub(crate) async fn bil_hajb<T, F>(amal: F) -> Natija<T>
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

    /// The build had to be measured and the game is not where the library left
    /// it, so there is nothing to measure.
    ///
    /// Its own refusal rather than a shared one, because the way out differs: a
    /// game that is gone is pointed at or reinstalled, a game that is there and
    /// will not measure has files its own launcher's verify repairs.
    #[error("{ism} is no longer at {}, so its build cannot be measured", jidhr.display())]
    BinaBilaLuba {
        /// The game's name, as its launcher gives it.
        ism: String,
        /// Where the library last saw it.
        jidhr: PathBuf,
    },

    /// The installed build could not be fingerprinted through the package's own
    /// recipe.
    ///
    /// `9028` used to read "no build fingerprint is on record; probe the game
    /// first", and nothing in this product has ever written that record — the
    /// probe least of all, so the one step it named was the one action
    /// guaranteed not to help. The measurement is taken here now. The number is
    /// kept because it still says the same thing about the same game — the build
    /// this package would be matched against could not be established — and an
    /// old diagnostics bundle naming `9028` still reads as what it meant.
    #[error("the build of {ism} could not be measured at {}: {sabab}", jidhr.display())]
    BinaMutaadhdhira {
        /// The game's name.
        ism: String,
        /// Where its files were read from.
        jidhr: PathBuf,
        /// What the fingerprint recipe said.
        sabab: String,
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

    /// No anti-cheat scan was run on the game at all.
    ///
    /// Distinct from [`Self::FahsHimayaLamYajri`], and kept distinct for the
    /// remedy rather than for the taxonomy: that one is fixed by naming Steam's
    /// folder, and offering the same advice here would send someone to correct a
    /// setting that is not wrong. Nothing walked the directory, which on the
    /// install path means a caller assembled the inputs without the scan — so
    /// the way out is a diagnostics bundle, not a preference.
    #[error("{ism}: no anti-cheat scan was run on {jidhr}")]
    MashHimayaLamYajri {
        /// The game.
        ism: String,
        /// The folder nothing walked.
        jidhr: String,
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

    /// The registry is answering and its revocation list is not.
    ///
    /// Not a revocation and not a network failure. An unreachable registry is
    /// an offline machine, which installs from its local copy with the list's
    /// state said out loud; this is a registry that served its manifest and
    /// withheld, or corrupted, the one document able to withdraw a patch. On a
    /// machine that can ask, "we could not check" is not allowed to become
    /// "nothing is revoked", so nothing is installed until a later refresh
    /// finds the list.
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
                    Self::BinaMutaadhdhira { .. } => 28,
                    Self::TathbeetFashil { .. } => 29,
                    Self::LughaRasmiyaMawjuda { .. } => 30,
                    Self::IqrarNaqis => 34,
                    Self::TawqeeMarfud { .. } => 35,
                    Self::RuqaaMulgha { .. } => 36,
                    // Not aligned with anything, and it takes one of the three
                    // reserved numbers below rather than 40, because the
                    // alignment those three protect is worth more than
                    // contiguity: 40 would be the first code past the block that
                    // the automatic path has no twin for.
                    Self::MashHimayaLamYajri { .. } => 33,
                    // The second of the reserved three, for the same reason.
                    // Its automatic-path twin is `9137`; the `9130` that would
                    // have aligned with it was taken by the sharing surface.
                    Self::QaimatSahbMahjuba { .. } => 31,
                    // The last of the reserved three, spent on what the
                    // measurement behind `9028` leaves over: a game that is not
                    // on disk at all. It is the headroom the alignment below was
                    // kept clear of, used for what it was kept for.
                    Self::BinaBilaLuba { .. } => 32,
                    // The last digit is the automatic path's, deliberately: the
                    // two install routes refuse for the same three reasons out
                    // of the same scan, and `9037`/`9038`/`9039` against
                    // `9127`/`9128`/`9129` says so at a glance in a log, a
                    // diagnostics bundle and a support thread.
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
            | Self::TathbeetFashil { .. }
            | Self::TawqeeMarfud { .. }
            | Self::RuqaaMulgha { .. } => Khutura::Khatar,
            Self::MuhimmaMutawaqqifa { .. } => Khutura::Tanbeeh,
            // Both are a disagreement between the library and the disk that the
            // user can see and settle, on the same reading the submission
            // surface gives the same two refusals. Nothing was written in
            // either: the measurement runs before the backup is taken.
            Self::BinaBilaLuba { .. } | Self::BinaMutaadhdhira { .. } => Khutura::Tanbeeh,
            // Two questions waiting for their answers rather than two things
            // that went wrong, on the same reading `KhataTilqaiAmr` gives the
            // multiplayer refusal. Reporting either as a fault would teach a
            // reader to expect a broken product where there is a consent
            // prompt.
            Self::IqrarNaqis | Self::ShabakaBilaIqrar { .. } => Khutura::Maluma,
            // The account is what is at stake in both, so neither is routine:
            // one names anti-cheat evidence, the other names a check that could
            // not be completed and must not be read as a pass.
            Self::HimayaMuktashafa { .. }
            | Self::FahsHimayaLamYajri { .. }
            | Self::MashHimayaLamYajri { .. } => Khutura::Tanbeeh,
            // A check that could not be completed and must not be read as a
            // pass, on the same reading as the catalogue one above — but about
            // the registry rather than the account, which is why it is its own
            // arm and not folded into theirs.
            Self::QaimatSahbMahjuba { .. } => Khutura::Tanbeeh,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MuarrifRuqaaGhayrSalih { .. } => {
                "المعرّف المطلوب ليس معرّف رقعة. أعد تحميل قائمة الرقع لهذه اللعبة.".to_owned()
            },
            Self::RuqaaGhayrMawjuda { ism, .. } => format!(
                "لم يعد المستودع يعرض هذه الرقعة لـ{ism}. ربما سُحبت بعد آخر تحديث للفهرس؛ \
                 أعد تحميل القائمة."
            ),
            Self::LaTathbeet { ism } => {
                format!("لا يوجد تثبيت من تعريب في {ism}، فليس هناك ما يُفحص أو يُزال.")
            },
            Self::GhayrMuttasil => {
                "وضع العمل دون اتصال مفعَّل ولا توجد نسخة محلية من المستودع، فلا مصدر يُسأل. \
                 أضف مجلدًا محليًا أو عطّل وضع دون اتصال من الإعدادات."
                    .to_owned()
            },
            Self::MuhimmaMutawaqqifa { .. } => {
                "توقّفت مهمة في الخلفية قبل أن تنتهي. أعد المحاولة؛ إن تكرّر الأمر فراجع سجلّ \
                 التشخيص."
                    .to_owned()
            },
            Self::TanfidhiMajhul { ism } => {
                format!("لم يُحدَّد الملف التنفيذي لـ{ism} بعد. أعد فحص اللعبة أولًا ثم أعد المحاولة.")
            },
            Self::FuruqGhayrMaduma { adad } => format!(
                "تعيد هذه الرقعة كتابة {adad} حاوية من حاويات اللعبة، وهذه النسخة لا تثبّت \
                 هذا النوع بعد. لم يُكتب شيء."
            ),
            Self::HuzmaTalifa { .. } => {
                "تعذّرت قراءة ملف الرقعة؛ ربما لم يكتمل تنزيله. أعد تنزيله ثم أعد المحاولة."
                    .to_owned()
            },
            Self::BinaBilaLuba { ism, jidhr } => format!(
                "قبل التثبيت تُقاس بصمة البناء من ملفات {ism} نفسها، ولم يعد مجلدها في {}. \
                 لم يُكتب شيء. أعد تثبيت اللعبة من مشغّلها أو دلّ تعريب على مكانها الجديد ثم \
                 أعد المحاولة.",
                jidhr.display()
            ),
            Self::BinaMutaadhdhira { ism, jidhr, sabab } => format!(
                "تعذّر قياس بصمة بناء {ism} من ملفاتها في {}: {sabab}. القياس يقرأ الحاويات \
                 التي تسمّيها الرقعة نفسها، فإن نقص منها ملف فإمّا أنّ التنزيل لم يكتمل وإمّا \
                 أنّ هذه الرقعة لنسخة أخرى من اللعبة. لم يُكتب شيء. تحقّق من سلامة ملفات \
                 اللعبة من مشغّلها ثم أعد المحاولة.",
                jidhr.display()
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
            },
            Self::HimayaMuktashafa {
                anwa, dalail_arabi, ..
            } => format!(
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
            },
            Self::MashHimayaLamYajri { jidhr, .. } => format!(
                "لم يُجرَ فحص مكافحة الغش على {jidhr} أصلًا، فلم يُثبَّت شيء. خلوّ الأدلّة هنا \
                 يعني أنّ أحدًا لم ينظر، لا أنّ اللعبة سليمة. أعد فتح صفحة اللعبة ليُجرى \
                 الفحص؛ فإن تكرّر هذا فأرسل حزمة التشخيص."
            ),
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
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => format!(
                "أجاب المستودع ({masdar}) في {waqt} لكنّه لم يقدّم قائمة الإبطال ({sabab})، \
                 فلم يُثبَّت شيء. ما دام المستودع يجيب فلا يُثبَّت شيء قبل قراءة قائمته، لأنّ \
                 الرقعة التي سُحبت لا تُعرف إلا منها. أعد المحاولة بعد قليل؛ وإن كان المصدر \
                 مجلّدًا محليًا فتأكّد من أنّ ملف القائمة موجود فيه."
            ),
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MuarrifRuqaaGhayrSalih { ruqaa } => {
                format!("{ruqaa} is not a patch identity. Reload the patch list for this game.")
            },
            Self::RuqaaGhayrMawjuda { ruqaa, ism } => format!(
                "The registry no longer lists patch {ruqaa} for {ism}. It was probably \
                 withdrawn since the index was last refreshed; reload the list."
            ),
            Self::LaTathbeet { ism } => {
                format!("{ism} has no Taarib installation, so there is nothing to check or remove.")
            },
            Self::GhayrMuttasil => {
                "Offline mode is on and no local registry copy is configured, so there is no \
                 source left to ask. Add a local folder, or turn offline mode off in Settings."
                    .to_owned()
            },
            Self::MuhimmaMutawaqqifa { tafsil } => {
                format!("A background task did not finish ({tafsil}). Try again.")
            },
            Self::TanfidhiMajhul { ism } => {
                format!("The executable of {ism} is not identified; probe the game again first.")
            },
            Self::FuruqGhayrMaduma { adad } => format!(
                "This package rewrites {adad} game container(s), which this build does not \
                 install yet. Nothing was written."
            ),
            Self::HuzmaTalifa { tafsil } => {
                format!("The package file could not be read: {tafsil}. Download it again.")
            },
            Self::BinaBilaLuba { ism, jidhr } => format!(
                "Installing measures the build fingerprint from {ism}'s own files first, and \
                 {} is no longer there. Nothing was written. Reinstall the game from its \
                 launcher, or point Taarib at where it is now, and try again.",
                jidhr.display()
            ),
            Self::BinaMutaadhdhira { ism, jidhr, sabab } => format!(
                "The build fingerprint of {ism} could not be measured from its files at {}: \
                 {sabab}. The measurement reads the containers the patch itself names, so a \
                 missing one means either the game is still downloading or this patch is for a \
                 different edition of it. Nothing was written. Verify the game's files through \
                 its launcher and try again.",
                jidhr.display()
            ),
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
            },
            Self::HimayaMuktashafa {
                anwa,
                dalail_injilizi,
                ..
            } => format!(
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
            },
            Self::MashHimayaLamYajri { jidhr, .. } => format!(
                "No anti-cheat scan was run on {jidhr} at all, so nothing was installed. An \
                 empty evidence list here means nobody looked, not that the game is clean. \
                 Reopen the game's page so the scan runs; if this repeats, send a diagnostics \
                 bundle."
            ),
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
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => format!(
                "The registry ({masdar}) answered at {waqt} but did not serve its revocation \
                 list ({sabab}), so nothing was installed. While the registry is reachable \
                 nothing is installed until its list can be read, because a withdrawn patch is \
                 known only from it. Try again shortly; if the source is a local folder, make \
                 sure the list file is in it."
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
            // Both used to promise `IadatMutabaqaBina`, which neither can
            // deliver: re-matching a patch against a build does not turn a
            // malformed identity into a patch identity and does not bring back a
            // listing the registry has withdrawn. Both sentences say to reload
            // the list, and reloading it is the same operation attempted again.
            Self::MuarrifRuqaaGhayrSalih { .. } | Self::RuqaaGhayrMawjuda { .. } => {
                Khutwa::AadaMuhawala
            },
            Self::LaTathbeet { .. } => Khutwa::LaShay,
            Self::GhayrMuttasil => Khutwa::FathIdadat {
                qism: QismIdadat::Masadir,
            },
            Self::MuhimmaMutawaqqifa { .. } | Self::HuzmaTalifa { .. } => Khutwa::AadaMuhawala,
            // Lifted by the next refresh that finds the list, which the next
            // press of the same button runs; nothing on this machine is wrong.
            Self::QaimatSahbMahjuba { .. } => Khutwa::AadaMuhawala,
            Self::TanfidhiMajhul { .. } => Khutwa::AadaFahsMuharrik,
            // The measurement needs the game's own files, so the two steps are
            // the two ways of putting them back: a folder Taarib can find, or a
            // folder whose contents the launcher has checked. The game screen
            // offers the picker for the first through its own children; the
            // second is done in the launcher and nowhere else, which is why the
            // sentence carries it rather than a button.
            Self::BinaBilaLuba { .. } => Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba,
            },
            Self::BinaMutaadhdhira { .. } => Khutwa::TahaqquqSalamatLuba,
            Self::FuruqGhayrMaduma { .. } => Khutwa::TahdithTaarib,
            // The package is in hand and refused; the diagnostics bundle is
            // what a report of any of the three carries, and none of them is
            // fixed by pressing the button again.
            // The last of these is not a fault in the world but a fault in us:
            // nothing walked the folder, and no setting the reader can reach
            // changes that.
            Self::TathbeetFashil { .. }
            | Self::TawqeeMarfud { .. }
            | Self::MashHimayaLamYajri { .. }
            | Self::RuqaaMulgha { .. } => Khutwa::FathTashkhis,
            // The remedy is a setting, and it is the only one this refusal has.
            Self::LughaRasmiyaMawjuda { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Lugha,
            },
            // The manual Steam path is the one way out, and it is the same one
            // `KhataLuba::JidhrSteamMajhul` sends the reader to.
            Self::FahsHimayaLamYajri { .. } => Khutwa::FathIdadat {
                qism: QismIdadat::Manassat,
            },
            // Three refusals with no button, for three different reasons and one
            // shared rule: none of them may be clicked past *here*. Anti-cheat is
            // lifted by nothing at all — no override for it exists anywhere in
            // this product, and this would be the first one. The other two are
            // lifted by an acknowledgement the person gives, and an action
            // offering to give it would be giving it for them; the interface
            // reopens the question off the code instead, which is what the code
            // is for. Each sentence names its own way out, or says plainly that
            // there is none.
            Self::HimayaMuktashafa { .. } | Self::IqrarNaqis | Self::ShabakaBilaIqrar { .. } => {
                Khutwa::LaShay
            },
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MuarrifRuqaaGhayrSalih { ruqaa } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
            },
            Self::RuqaaGhayrMawjuda { ruqaa, ism } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::LaTathbeet { ism } | Self::TanfidhiMajhul { ism } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            // The same two keys the submission surface writes for the same two
            // refusals, so one log filter reads the measurement wherever it was
            // taken from.
            Self::BinaBilaLuba { ism, jidhr } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
            },
            Self::BinaMutaadhdhira { ism, jidhr, sabab } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("jidhr".to_owned(), QeemaSiyaq::Masar(jidhr.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            // Neither carries a fact worth a column: one is a mode the user set
            // and the other is a statement they have not read yet.
            Self::GhayrMuttasil | Self::IqrarNaqis => {},
            Self::MuhimmaMutawaqqifa { tafsil } | Self::HuzmaTalifa { tafsil } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(tafsil.clone()));
            },
            Self::FuruqGhayrMaduma { adad } => {
                let _ = siyaq.insert("adad".to_owned(), QeemaSiyaq::Hajm(*adad));
            },
            Self::TathbeetFashil { injilizi, .. } | Self::TawqeeMarfud { injilizi, .. } => {
                let _ = siyaq.insert("tafsil".to_owned(), QeemaSiyaq::Nass(injilizi.clone()));
            },
            Self::LughaRasmiyaMawjuda { ism } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::RuqaaMulgha { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::QaimatSahbMahjuba {
                masdar,
                sabab,
                waqt,
            } => {
                let _ = siyaq.insert("masdar".to_owned(), QeemaSiyaq::Nass(masdar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
                let _ = siyaq.insert("waqt".to_owned(), QeemaSiyaq::Nass(waqt.clone()));
            },
            // The same keys the automatic path writes for the same three facts,
            // so one log filter reads both routes.
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
            Self::MashHimayaLamYajri { ism, jidhr } => {
                let _ = siyaq.insert("ism".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Nass(jidhr.clone()));
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
        }
        siyaq
    }
}

khata_min!(KhataTathbeetAmr);

/// The bundled revocation list, signed by the owner key at sequence 1.
///
/// The floor, not the answer. Every client verifies it, and the registry's
/// current list supersedes it the moment one is fetched; it cannot be updated
/// without shipping a new binary, which is why no install path reads it
/// directly any more — see [`qaimat_sahb_lil_bawwaba`].
const QAIMAT_SAHB_ASLIYA: &[u8] = include_bytes!("../../../../assets/qaimat_sahb.json");

/// The compiled-in list and the anchor that verifies it.
pub(crate) struct AsasSahb {
    /// The owner key every list is verified against.
    pub(crate) miftah: MiftahAam,
    /// The list this build shipped, verified.
    pub(crate) asliya: QaimatSahb,
}

/// The compiled anchor and the bundled list, verified against each other.
///
/// One function rather than two lines at each gate, because every gate hands
/// the same floor to the same store, and a second spelling of "which anchor
/// verifies it" is a second place for that answer to drift.
///
/// # Errors
///
/// [`KhataAman::QaimatSahbFashila`] when the compiled anchor is not a public
/// key, or when the bundled list does not verify against it — which would mean
/// the build itself is inconsistent, not that the user did anything.
pub(crate) fn asas_sahb() -> Result<AsasSahb, KhataAman> {
    let miftah = MiftahAam::min_bayt(&taarib_khatm::MIRSAT_MALIK.miftah).map_err(|khata| {
        KhataAman::QaimatSahbFashila {
            amal: "anchored",
            sabab: khata.to_string(),
        }
    })?;
    let asliya = QaimatSahb::min_bayt(QAIMAT_SAHB_ASLIYA, &miftah)?;
    Ok(AsasSahb { miftah, asliya })
}

/// The refresh window: how long a successful fetch counts as current.
///
/// The same interval the settings name for the manifest refresh. The list is
/// fetched with the manifest, on the same schedule, and there is no reason for
/// the two to disagree. The settings layer already refuses anything under five
/// minutes; the floor is repeated here so a hand-edited file cannot turn the
/// background loop into a busy one.
fn nafidhat_sahb(hali: &Idadat) -> SignedDuration {
    SignedDuration::from_mins(i64::from(hali.masadir.fatra_tahdith.max(5)))
}

/// The revocation list this machine holds, beside where it stands. No network.
///
/// What every synchronous gate reads: the newer of the verified cache and the
/// compiled-in floor, with the refresh record's account of whether the
/// registry confirmed it and when. Synchronous commands run on the thread the
/// window is driven from, so the fetch that keeps the cache current lives at
/// the asynchronous moments before an install — the page that lists the
/// patches, the download, the start of an automatic run, the submission — and
/// in the background loop, never here.
///
/// # Errors
///
/// Only when the build's own anchor or list is inconsistent; every way the
/// cache can be wrong is a state the result carries, not an error.
pub(crate) fn iqra_qaimat_sahb(masarat: &Masarat, hali: &Idadat) -> Result<QaimaMuraqaba, Khata> {
    let asas = asas_sahb().map_err(|khata| Khata::min_tafsir(&khata))?;
    let qaima = QaimaMuraqaba::iqra(
        masarat,
        &asas.miftah,
        asas.asliya,
        Timestamp::now(),
        nafidhat_sahb(hali),
    );
    tracing::info!(
        hala = qaima.hala().ism(),
        tasalsul = qaima.qaima().tasalsul(),
        adad = qaima.qaima().adad(),
        "{}",
        qaima.wasf_injilizi()
    );
    Ok(qaima)
}

/// [`iqra_qaimat_sahb`], refusing when the state earns a refusal.
///
/// The one refusal: the latest attempt within the window reached the registry
/// and came back without a usable list. Stale, unreachable and never-fetched
/// all pass, with their state carried into the result the screen shows —
/// this product is built to install from a local copy with no network at all,
/// and a machine that cannot ask is not a machine that was answered.
///
/// # Errors
///
/// [`KhataTathbeetAmr::QaimatSahbMahjuba`], and whatever [`iqra_qaimat_sahb`]
/// raises.
pub(crate) fn qaimat_sahb_lil_bawwaba(
    masarat: &Masarat,
    hali: &Idadat,
) -> Result<QaimaMuraqaba, Khata> {
    let qaima = iqra_qaimat_sahb(masarat, hali)?;
    if let Some(rafd) = qaima.rafd() {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::QaimatSahbMahjuba {
            masdar: rafd.masdar.clone(),
            sabab: rafd.sabab.clone(),
            waqt: rafd.waqt.to_string(),
        }));
    }
    Ok(qaima)
}

/// One refresh of the revocation list from the configured sources.
///
/// The outcome is written beside the cache by the registry client and logged
/// here; it is returned for a caller that acts on it and ignored by the ones
/// that only want the cache current. Offline mode with no local copy is
/// recorded as an attempt that had no source to ask, so the next gate says
/// that rather than "no refresh has been attempted".
pub(crate) async fn jaddid_sahb(masarat: &Masarat, hali: &Idadat) -> NatijatTajdid {
    // Offline mode with no local copy is a chain with nothing in it rather than
    // a branch of its own: the chain answers "no registry source is
    // configured", which the refresh records as an attempt that had nothing to
    // ask — so the next gate says exactly that instead of "no refresh has been
    // attempted", which would be a different and untrue statement.
    let silsila = silsilat_masadir(hali).unwrap_or_else(|_| SilsilatMasadir::jadida(Vec::new()));
    let natija = match asas_sahb() {
        Ok(asas) => {
            jaddid_qaimat_sahb(
                &silsila,
                masarat,
                &asas.miftah,
                &asas.asliya,
                Timestamp::now(),
            )
            .await
        },
        Err(khata) => NatijatTajdid::Khata(khata),
    };
    sajjil_natijat_tajdid(&natija);
    natija
}

/// As [`jaddid_sahb`], riding on a manifest the caller has just fetched, so
/// the registry is asked once for it rather than twice.
pub(crate) async fn jaddid_sahb_bi_bayan(
    masarat: &Masarat,
    silsila: &SilsilatMasadir,
    fahras: &FahrasMajlub,
) -> NatijatTajdid {
    let natija = match asas_sahb() {
        Ok(asas) => {
            jaddid_qaimat_sahb_bi_bayan(
                silsila,
                &fahras.bayan,
                &fahras.masdar_bayan,
                masarat,
                &asas.miftah,
                &asas.asliya,
                Timestamp::now(),
            )
            .await
        },
        Err(khata) => NatijatTajdid::Khata(khata),
    };
    sajjil_natijat_tajdid(&natija);
    natija
}

/// The log line every refresh ends on, at the level its outcome deserves.
fn sajjil_natijat_tajdid(natija: &NatijatTajdid) {
    match natija {
        // Offline is a normal state of this product, not a fault.
        NatijatTajdid::Najah { .. } | NatijatTajdid::MustawdaGhayrMutah { .. } => {
            tracing::info!("{}", natija.wasf());
        },
        NatijatTajdid::QaimaMutaadhdhira { .. } | NatijatTajdid::Aqdam { .. } => {
            tracing::warn!("{}", natija.wasf());
        },
        NatijatTajdid::Khata(_) => tracing::error!("{}", natija.wasf()),
    }
}

/// The background refresh, started once per process by the first command that
/// needs the list current.
///
/// Started from the commands rather than from the application's setup hook
/// because that file is not this change's to edit; the first listing, download
/// or install starts it and every later call is a no-op. It refreshes at once,
/// then at the interval the settings name, re-reading the settings on every
/// tick so a source added or offline mode switched on takes effect at the
/// next one. Only the command wrappers call it, so no test starts a loop.
pub(crate) fn dhamin_mujaddid_sahb(masarat: &Masarat, idadat: &Arc<MakhzanIdadat>) {
    static MUJADDID: OnceLock<()> = OnceLock::new();
    let masarat = masarat.clone();
    let idadat = Arc::clone(idadat);
    MUJADDID.get_or_init(|| {
        drop(tauri::async_runtime::spawn(async move {
            loop {
                let hali = idadat.hali();
                let _ = jaddid_sahb(&masarat, &hali).await;
                let thawani = u64::try_from(nafidhat_sahb(&hali).as_secs()).unwrap_or(300);
                tokio::time::sleep(Duration::from_secs(thawani)).await;
            }
        }));
        // The cell holds nothing; reaching it at all is the whole record.
    });
}

/// Where the revocation list a gate consulted stood, as the interface shows it.
///
/// Carried on every result an install produces, because "nothing is revoked"
/// is a different sentence from "nothing was checked against the registry",
/// and the result used to say the first whenever it meant the second.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct HalatSahbHie {
    /// The state as a stable key: `muhaddatha`, `mukhazzana`, `muntahiya`,
    /// `lam_tujlab` or `talifa`.
    pub hala: String,
    /// Whether the registry confirmed the list within the refresh window — the
    /// only state under which a clean revocation answer is a statement about
    /// the registry rather than about this machine.
    pub muhaddatha: bool,
    /// The list's sequence.
    #[specta(type = specta_typescript::Number)]
    pub tasalsul: u64,
    /// How many revocations it carries.
    pub adad: u32,
    /// The whole standing, in Arabic.
    pub arabi: String,
    /// The same, in English.
    pub injilizi: String,
}

/// One gate's list and state, projected for the screen.
pub(crate) fn sahb_hie(qaima: &QaimaMuraqaba) -> HalatSahbHie {
    HalatSahbHie {
        hala: qaima.hala().ism().to_owned(),
        muhaddatha: qaima.hala().muhaddatha(),
        tasalsul: qaima.qaima().tasalsul(),
        adad: adad(qaima.qaima().adad()),
        arabi: qaima.wasf_arabi(),
        injilizi: qaima.wasf_injilizi(),
    }
}

/// One install stage, as the progress event names it.
pub const ISM_HADATH_TATHBEET: &str = "taarib://marhalat-tathbeet";

/// The stage label for the fingerprint that runs before the gate.
///
/// Announced like every other stage, and for the same reason the automatic path
/// counts it as its own step: hashing a Unity game's containers is seconds of
/// work, and a window that says nothing while it happens is a window that has
/// frozen as far as the person in front of it can tell. The payload is an Arabic
/// sentence because [`taarib_mustawda::tathbeet_bilnaqra::MarhalatTathbeet`]'s
/// labels are, and the screen renders whatever arrives.
const MARHALAT_QIYAS_BINA: &str = "قياس بصمة بناء اللعبة";

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
    /// The revocation list the safety gate checked against, and where it stood.
    pub sahb: HalatSahbHie,
}

/// One loader slot beside the game that a third-party mod already holds.
///
/// Three fields, and two of them are whole sentences the installer wrote.
/// [`WakeelQaim`] composes its own line — the file, its size, what is beside it
/// and which product the evidence named — in both languages, and the confirmation
/// screen shows that line rather than rebuilding one out of the parts. A
/// projection that carried the parts instead would be a second place for the
/// wording to be decided, and the wording is the whole value here: "an ASI plugin
/// loader, from the string \"Alexander Blade\" inside it" is an answer, and
/// "`dinput8.dll`, 131072, `muhammil_asi`" is a puzzle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct WakeelQaimHie {
    /// The file exactly as it is spelled on disk, which is the key a reader
    /// looks for in their own game directory.
    pub ism: String,
    /// The whole finding as one Arabic line, [`WakeelQaim`]'s own.
    pub arabi: String,
    /// The same line in English.
    pub injilizi: String,
}

impl WakeelQaimHie {
    /// One surveyed slot, rendered once in each language.
    fn min_asli(wakeel: &WakeelQaim) -> Self {
        Self {
            ism: wakeel.ism.clone(),
            arabi: wakeel.wasf_arabi(),
            injilizi: wakeel.wasf_injilizi(),
        }
    }
}

/// One file the plan would write, and whether the game already has it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MudkhalKhuttaHie {
    /// The destination relative to the game root, as the plan names it.
    pub nisbi: String,
    /// Whether the game already has this file.
    ///
    /// The distinction the confirmation screen is built around: an added file is
    /// invisible to the store's integrity check and survives it, and a modified
    /// one has its original preserved first and is put straight back by that
    /// same check. Both halves matter, and they part company — see
    /// [`KhuttatTathbeetHie::malhuzat_tahaqquq_arabi`].
    pub tadeel: bool,
}

/// What the plan learned about the launcher that owns this game's launch
/// options, when the plan needs a launch-time change.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct MalhuzatManassaHie {
    /// The launcher, as a person reading the plan knows it.
    ///
    /// Carried beside the finished sentences, not instead of them: a screen that
    /// wants to offer "close {ism}" as a button needs the bare name, and a
    /// screen that builds the sentence out of it is how the two languages drift.
    pub ism: String,
    /// The whole note, in Arabic, as the installer words it.
    ///
    /// The two sandbox names this used to carry are gone. They existed so the
    /// screen could assemble one of two sentences from them, and the assembly
    /// lived in TypeScript while the same two sentences lived in Rust —
    /// [`MalhuzatManassa::wasf_arabi`] is now the only place either exists.
    pub wasf_arabi: String,
    /// The same note in English, from the same producer.
    pub wasf_injilizi: String,
}

/// The whole deployment plan for one game, before anything is written.
///
/// A projection of one [`taarib_tathbeet::tarkib::KhuttatTarkib`] and of nothing
/// else. Every sentence in it was written by the installer — the reason no
/// framework is needed, each launch requirement, each loader already in the game
/// — and is carried in both languages because the installer writes both. What
/// this layer adds is the *shape*: counts the screen groups by, and paths, which
/// have no language.
///
/// [`Self::sutur`] is the plan's own report text, unedited. It is the artifact a
/// user pastes into a bug report and the one the install log carries, and it is
/// here for the same reason [`HasilatIzala::sutur`] is there.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct KhuttatTathbeetHie {
    /// The tier this plan was built under, 1 to 3.
    pub tabaqa_raqm: u8,
    /// The tier's name in Arabic.
    pub tabaqa_arabi: String,
    /// The tier's name in English.
    pub tabaqa_injilizi: String,
    /// Whether the game's own files are changed at all.
    ///
    /// False at tier 3 and nowhere else, and read from the plan's own decision
    /// rather than from an empty file list: a plan that writes nothing because
    /// the component store is thin is not the same statement as a plan that
    /// writes nothing because the product does not touch this game.
    pub tughayyar_al_luba: bool,
    /// Whether the plan writes nothing into the game at all.
    pub faragha: bool,
    /// Why no framework is deployed, when none is, in Arabic.
    pub sabab_faragh_arabi: Option<String>,
    /// The same reason in English.
    pub sabab_faragh_injilizi: Option<String>,
    /// The framework component to deploy, as the component table describes it.
    ///
    /// English only, and deliberately not translated here: it is a build
    /// identifier — "BepInEx for Unity 2019.1-2021.3 (Mono, x64)" — and the
    /// three things it names are proper nouns in every language.
    pub itar: Option<String>,
    /// Where that framework's loader lands, relative to the game root.
    pub jidhr_muhammil: Option<String>,
    /// Directories the additive layer creates, in creation order.
    pub mujalladat: Vec<String>,
    /// The additive layer's files, in write order.
    pub mudkhalat: Vec<MudkhalKhuttaHie>,
    /// How many files the game does not have yet.
    pub adad_idafat: u32,
    /// How many files the game already has, whose originals are preserved first.
    pub adad_tadeelat: u32,
    /// The Arabic face deployed into a Ren'Py game, relative to `game/`.
    pub khatt_renpy: Option<String>,
    /// What launching the game will require afterwards, in Arabic.
    pub talabat_arabi: Vec<String>,
    /// The same requirements in English.
    pub talabat_injilizi: Vec<String>,
    /// Loader slots beside the game that a third-party mod already holds.
    ///
    /// Empty is the normal answer. A game with `BepInEx`, `ReShade`, an ASI
    /// loader or `re4_tweaks` already in it is not the game the publisher
    /// shipped, and this is the field that says so before the user agrees
    /// rather than after.
    pub huqn_qaim: Vec<WakeelQaimHie>,
    /// The launcher note, when the plan needs a launch-time change.
    pub manassa: Option<MalhuzatManassaHie>,
    /// The store-verify note, in Arabic, when the plan warrants one.
    ///
    /// The sentence rather than the boolean that used to stand here. A verify
    /// compares the tree against the depot manifest, so it restores every
    /// modified file and leaves every added one — the halves come apart, and the
    /// game launches with Taarib loaded and its own original text. The screen
    /// used to hold its own Arabic for that and key it off a `bool`, which meant
    /// two sentences describing one thing with nothing tying them together;
    /// [`KhuttatTarkib::malhuzat_tahaqquq_arabi`] is now the only one.
    pub malhuzat_tahaqquq_arabi: Option<String>,
    /// The same note in English, from the same producer.
    pub malhuzat_tahaqquq_injilizi: Option<String>,
    /// The plan's own report text, line for line, in the installer's words.
    pub sutur: Vec<String>,
}

impl KhuttatTathbeetHie {
    /// One plan, projected for the screen that has to show it.
    fn min_asli(mukhattat: &KhuttatTarkib) -> Self {
        let tabaqa = mukhattat.qarar().tabaqa();
        let itar = match &mukhattat.hajat {
            HajatItar::Matlub(mukawwin) => {
                Some(format!("{} — {}", mukawwin.wasf, mukawwin.tahmil.wasf()))
            },
            HajatItar::LaHaja(_) => None,
        };
        Self {
            tabaqa_raqm: tabaqa.raqm(),
            tabaqa_arabi: tabaqa.ism_arabi().to_owned(),
            tabaqa_injilizi: tabaqa.ism_injilizi().to_owned(),
            tughayyar_al_luba: mukhattat.qarar().tughayyar_al_luba(),
            faragha: mukhattat.faragha(),
            sabab_faragh_arabi: mukhattat
                .sabab_faragh()
                .map(|sabab| sabab.wasf_arabi().to_owned()),
            sabab_faragh_injilizi: mukhattat
                .sabab_faragh()
                .map(|sabab| sabab.wasf_injilizi().to_owned()),
            itar,
            jidhr_muhammil: mukhattat.jidhr_muhammil.as_ref().map(ToString::to_string),
            mujalladat: mukhattat
                .mujalladat
                .iter()
                .map(|m| m.nisbi.clone())
                .collect(),
            mudkhalat: mukhattat
                .mudkhalat
                .iter()
                .map(|mudkhal| MudkhalKhuttaHie {
                    nisbi: mudkhal.nisbi.clone(),
                    tadeel: matches!(mudkhal.naw, NawMudkhal::Tadeel),
                })
                .collect(),
            adad_idafat: adad(mukhattat.adad_idafat()),
            adad_tadeelat: adad(mukhattat.adad_tadeelat()),
            khatt_renpy: mukhattat.khatt_renpy.clone(),
            talabat_arabi: mukhattat
                .talabat
                .iter()
                .map(TalabItlaq::wasf_arabi)
                .collect(),
            talabat_injilizi: mukhattat
                .talabat
                .iter()
                .map(TalabItlaq::wasf_injilizi)
                .collect(),
            huqn_qaim: mukhattat
                .huqn_qaim
                .iter()
                .map(WakeelQaimHie::min_asli)
                .collect(),
            manassa: mukhattat
                .manassa_taamil
                .as_ref()
                .map(|malhuza| MalhuzatManassaHie {
                    ism: malhuza.ism.clone(),
                    wasf_arabi: malhuza.wasf_arabi(),
                    wasf_injilizi: malhuza.wasf_injilizi(),
                }),
            malhuzat_tahaqquq_arabi: mukhattat.malhuzat_tahaqquq_arabi().map(ToOwned::to_owned),
            malhuzat_tahaqquq_injilizi: mukhattat.malhuzat_tahaqquq().map(ToOwned::to_owned),
            sutur: mukhattat.taqreer(),
        }
    }
}

/// One one-click install failure, as the interface's error type.
///
/// The safety layer's refusals are unpacked into their own codes; everything
/// else keeps the single [`KhataTathbeetAmr::TathbeetFashil`] it always had,
/// because the quarantine and installer errors underneath it already carry their
/// own sentences and there is nothing here to discriminate between.
pub(crate) fn khata_naqra(
    fashal: &taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet,
    ism: &str,
) -> Khata {
    use taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet;

    match fashal {
        FashalTathbeet::Aman(rafd) => Khata::min_tafsir(&khata_rafd(rafd, ism)),
        FashalTathbeet::Sahb(rafd) => Khata::min_tafsir(&KhataTathbeetAmr::QaimatSahbMahjuba {
            masdar: rafd.masdar.clone(),
            sabab: rafd.sabab.clone(),
            waqt: rafd.waqt.to_string(),
        }),
        FashalTathbeet::Mustawda(_) | FashalTathbeet::Tathbeet(_) => {
            Khata::min_tafsir(&KhataTathbeetAmr::TathbeetFashil {
                arabi: fashal.arabi(),
                injilizi: fashal.injilizi(),
            })
        },
    }
}

/// One safety refusal, as the refusal it actually is.
///
/// Every arm of [`taarib_aman::fahs::Rafd`] gets its own code rather than the
/// one `TAARIB-E-9029` they all used to collapse into. The reason is not tidiness:
/// a code is the only thing on the wire a screen can branch on — `siyaq` is
/// rendered, never matched, everywhere in this product — and the seven refusals
/// have four different remedies between them. Without the split the manual
/// install path could not tell "you have not accepted the multiplayer risk",
/// which one tick and one press fixes, from a revoked signing key, which nothing
/// on this machine fixes; so it offered the same generic affordance for both.
///
/// The match is exhaustive on purpose, and it has since earned it:
/// [`Rafd::MashHimayaLamYajri`] was added to the safety layer and the compiler
/// stopped the build here, which is exactly the intent — the seventh refusal got
/// its own sentence and its own way out instead of falling into a bucket that
/// would have sent the reader to a Steam setting that was not the problem.
/// An eighth must be answered here too.
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
        Rafd::MashHimayaLamYajri { jidhr } => KhataTathbeetAmr::MashHimayaLamYajri {
            ism: ism.to_owned(),
            jidhr: jidhr.display().to_string(),
        },
        Rafd::Tawqee(sabab) => KhataTathbeetAmr::TawqeeMarfud {
            arabi: sabab.arabi(),
            injilizi: sabab.injilizi(),
        },
        Rafd::Mulgha { sabab } => KhataTathbeetAmr::RuqaaMulgha {
            sabab: sabab.clone(),
        },
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
    satrat
        .map(|satr| format!("- {satr}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Installs a downloaded or imported package into a game, end to end.
///
/// Quarantine, safety verdict, permit, backup, framework, placement and the
/// post-write verification all run inside `taarib-mustawda`'s pipeline; this
/// command assembles its inputs from the store and the package manifest and
/// reports each stage on [`ISM_HADATH_TATHBEET`].
///
/// The build the package is judged against is measured here, by [`qis_bina`],
/// from the recipe the package carries — not read out of the store. It used to
/// be read out of the store, and nothing in this product had ever written the
/// row, so every press of every install button on every game refused at
/// `TAARIB-E-9028` and sent the reader to a probe that writes a different ledger.
///
/// # Errors
///
/// [`Khata`] naming whichever gate refused: an unreadable package, a build that
/// cannot be measured, a build mismatch without acknowledgement, anti-cheat
/// evidence, a revoked package, or the installer's own refusals — each in its
/// own words. The safety layer's six refusals carry their own codes rather than
/// one shared code, so a screen can tell the one the user answers
/// ([`KhataTathbeetAmr::ShabakaBilaIqrar`], `TAARIB-E-9039`) from the ones
/// nobody can. Also [`crate::luba_awamir::KhataLuba::JidhrSteamMajhul`] when the
/// game is a Steam game and Steam itself cannot be found, because the anti-cheat
/// verdict would then be missing the half of its evidence that only Steam's
/// catalogue holds.
#[tauri::command]
#[specta::specta]
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the two acknowledgements are separate keys in the IPC payload the interface \
              already sends; folding them into one struct would change that contract"
)]
pub async fn thabbit_ruqaa(
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
        return Err(Khata::from(KhataTathbeetAmr::LughaRasmiyaMawjuda {
            ism: luba.ism,
        }));
    }

    let malaf_munazzal = PathBuf::from(&masar_malaf);
    let ruqaa = taarib_ruqaa::qari::MalafRuqaa::iftah(&malaf_munazzal).map_err(|q| {
        Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa {
            tafsil: q.to_string(),
        })
    })?;
    let bayan = ruqaa.ruqaa().and_then(|r| r.bayan_json()).map_err(|q| {
        Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa {
            tafsil: q.to_string(),
        })
    })?;

    let ruqaa_id: RuqaaId = qeema_bayan(&bayan, "id")?;
    let murajaa: RuqaaRevision = qeema_bayan(&bayan, "murajaa")?;
    let irtibat: taarib_tarqee::irtibat::IrtibatBina = qeema_bayan(&bayan, "irtibat")?;
    let furuq = bayan
        .get("bawwaba")
        .and_then(|b| b.get("furuq"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    if furuq > 0 {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::FuruqGhayrMaduma {
            adad: furuq,
        }));
    }

    let taqreer = makhzan
        .bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?
        .ok_or_else(|| {
            Khata::min_tafsir(&KhataTathbeetAmr::LaTathbeet {
                ism: luba.ism.clone(),
            })
        })?;
    let _ = nafidha.emit(ISM_HADATH_TATHBEET, MARHALAT_QIYAS_BINA);
    let bina = {
        let makhzan = Makhzan::clone(&makhzan);
        let luba = luba.clone();
        let mukhattat = irtibat.mukhattat.clone();
        bil_hajb(move || qis_bina(&makhzan, &luba, &mukhattat)).await?
    };
    // Built once, here, and handed to both the plan and the manifest. It used to
    // be assembled inline further down, which meant the description of the game
    // the planner saw was constructed separately from the one the preview beside
    // the button had constructed.
    let luba_muhallala = luba_lil_tarkib(&luba, &taqreer)?;

    // The cache as the page and the download left it; this command runs on the
    // window's thread and does not fetch. The loop is started here as well, for
    // the imported-file install that reaches this gate without either.
    dhamin_mujaddid_sahb(&masarat, &idadat);
    let qaima = qaimat_sahb_lil_bawwaba(&masarat, &hali)?;
    let sijill_iqrar = iqrar::iqra(&masar_iqrar(&masarat))?;

    let bayt_ruqaa = std::fs::read(&malaf_munazzal).map_err(|q| {
        Khata::min_tafsir(&KhataTathbeetAmr::HuzmaTalifa {
            tafsil: q.to_string(),
        })
    })?;
    let wajha = WajhatLuba::dakhil_taarib(&format!("{ruqaa_id}.ruqaa"))
        .map_err(|q| Khata::min_tafsir(&q))?;
    // Placed as this engine's adapter reads it: sealed bytes for every engine
    // but Unity, whose takeover reads only an uncompressed working copy.
    let bayt_ruqaa = taarib_tathbeet::masar_tathbeet::muhtawa_ruqaa(
        taqreer.muharrik.aila,
        bayt_ruqaa,
        &malaf_munazzal,
    )
    .map_err(Khata::from)?;
    let mut muhtawa = vec![WadaMuhtawa {
        wajha,
        bayt: bayt_ruqaa,
    }];
    // The faces the package was shaped against travel with it, from this
    // machine's font store, for the one engine whose adapter draws text itself.
    // Refused here, before the backup, when the store cannot supply them.
    muhtawa.extend(
        taarib_tathbeet::masar_tathbeet::muhtawa_khutut(
            taqreer.muharrik.aila,
            &irtibat.khutut,
            &crate::mukawwinat_tahmil::judhur_khutut_musannafa(&masarat),
        )
        .map_err(Khata::from)?,
    );

    let tarif = taarib_tathbeet::bayan::TarifLuba {
        luba: id,
        masdar: luba_muhallala.masdar.clone(),
        ism: luba.ism.clone(),
        jidhr: luba.jidhr.clone(),
        ruqaa: ruqaa_id,
        murajaa,
        basma_bina: Some(bina.basma),
    };

    let ism_tanfidhi = luba_muhallala
        .masar_tanfidhi
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

    let halat_idadat = taarib_tathbeet::tarkib::HalatIdadat {
        khiyarat_tashghil: None,
        tajawuzat_dll: None,
        tahmil_musbaq: None,
        malaf_idadat_manassa: None,
    };

    // The plan, decided before the confirmation rather than inside the writer.
    // `nashr` would have built one here anyway and thrown it away unseen; built
    // here it is the same value `khuttat_tathbeet` renders beside the button, so
    // the lines the user was shown are the lines that execute. It is also the
    // point where a missing component or an unbuilt compatibility prefix is
    // refused — before a backup is taken, rather than half way through one.
    let mukhattat = taarib_tathbeet::tarkib::khutta(&taqreer, &luba_muhallala, &mukawwinat)
        .map_err(Khata::from)?;

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
            let _ = taarib_tathbeet::tarkib::nashr_bi_khutta(
                &mukhattat,
                &luba_muhallala,
                &halat_idadat,
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
        sahb: sahb_hie(&qaima),
    })
}

/// Measures the installed build through the package's own recipe, and records it.
///
/// Measured, not remembered. The value decides whether this package may be
/// written into this game at all — [`taarib_tarqee::irtibat::IrtibatBina::ihkum`]
/// is handed exactly this fingerprint — and a fingerprint taken the last time
/// somebody installed describes the files as they were then, so reusing it would
/// pass a patch onto a build the store has replaced since. The automatic path
/// measures on every run for that reason; this is the same measurement, through
/// the same [`taarib_tarqee::irtibat::MukhattatBasma::ihsab`], against the recipe
/// that travelled inside this package.
///
/// The row it writes is the first one anything in this product has ever written.
/// A fingerprint needs a recipe and a recipe exists only inside a package, so a
/// library scan cannot produce one — which is why `luba.basma_haliya` was NULL
/// for every game, why [`rattib_murashshahat`] judged no listing against
/// anything, and why the diagnostics screen showed no build. One install now
/// settles all three.
///
/// # Errors
///
/// [`KhataTathbeetAmr::BinaBilaLuba`] when the game is not where the library
/// left it, [`KhataTathbeetAmr::BinaMutaadhdhira`] when it is there and the
/// recipe cannot be run over it, and whatever the store raises.
fn qis_bina(makhzan: &Makhzan, luba: &Luba, mukhattat: &MukhattatBasma) -> Natija<BinaId> {
    if !luba.mawjuda || !luba.jidhr.is_dir() {
        return Err(Khata::from(KhataTathbeetAmr::BinaBilaLuba {
            ism: luba.ism.clone(),
            jidhr: luba.jidhr.clone(),
        }));
    }

    let (basma, adad_malaffat) = mukhattat.ihsab(&luba.jidhr).map_err(|khata| {
        Khata::min_tafsir(&KhataTathbeetAmr::BinaMutaadhdhira {
            ism: luba.ism.clone(),
            jidhr: luba.jidhr.clone(),
            sabab: khata.to_string(),
        })
        .bi_sabab(Khata::from(khata))
    })?;

    // Left empty rather than carried over from whatever row was there before.
    // A launcher's build identifier belongs to the files it was read beside, and
    // a fingerprint that has moved is a different set of files; attaching the
    // old identifier to it would claim an exact match for a build nobody looked
    // at. The upsert keeps the identifier already stored against *this*
    // fingerprint, which is the one case where it still describes these bytes.
    let bina = BinaId {
        manassa: None,
        basma,
        adad_malaffat,
        waqt: makhzan.bil_qira(alaan)?,
    };
    makhzan.bi_muamala(|muamala| SijillBina::jadeed(muamala).sajjil(luba.id, &bina))?;

    tracing::info!(
        luba = %luba.id,
        basma = %bina.basma,
        adad_malaffat,
        "the installed build was fingerprinted and recorded"
    );
    Ok(bina)
}

/// The game as [`taarib_tathbeet::tarkib`] needs it, assembled from the store.
///
/// Shared by the install and by the plan shown beside its button, deliberately.
/// A plan is a promise about what pressing install will do, and it is only that
/// if both were built from the same description of the game; two constructions
/// of [`LubaMuhallala`] in two commands is a second place for the executable,
/// the compatibility environment or the launcher identity to drift, and every
/// instance of this defect in this project has been exactly that shape.
///
/// # Errors
///
/// [`KhataTathbeetAmr::TanfidhiMajhul`] when no executable was ever resolved for
/// the game — the loader directory is read off it — and
/// [`KhataTathbeetAmr::LaTathbeet`] when the game carries no launcher identity,
/// which is what a launch-time requirement would have to be recorded against.
fn luba_lil_tarkib(luba: &Luba, taqreer: &TaqreerImkaniyat) -> Result<LubaMuhallala, Khata> {
    let Some(masar_tanfidhi) = luba.tanfidhi.clone() else {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::TanfidhiMajhul {
            ism: luba.ism.clone(),
        }));
    };
    let Some(masdar) = luba.masadir.first().cloned() else {
        return Err(Khata::min_tafsir(&KhataTathbeetAmr::LaTathbeet {
            ism: luba.ism.clone(),
        }));
    };
    Ok(LubaMuhallala {
        jidhr: luba.jidhr.clone(),
        masar_tanfidhi,
        muharrik: taqreer.muharrik.clone(),
        beea: luba.beea.clone(),
        nizam: taarib_usus::manassa::NizamTashghil::hali(),
        masdar,
    })
}

/// What installing into this game would write, before anything is written.
///
/// The same [`taarib_tathbeet::tarkib::khutta`] call [`thabbit_ruqaa`] makes,
/// over the same game description, so this is not a description of the install
/// — it is the install's own plan, read early. It answers the four questions a
/// person is entitled to have answered before they agree: which files are
/// created and which of the game's own are replaced, what else is already
/// hooked into this game, whether verifying the game through its store would
/// undo it, and what launching it will need afterwards.
///
/// It walks the game directory — the loader survey opens every module in the
/// slot list to identify it — so a screen asks for it deliberately rather than
/// on mount, the way it asks for the evidence chain.
///
/// # Errors
///
/// [`KhataTathbeetAmr::LaTathbeet`] when the game has never been probed, so
/// there is no capability report to plan against, and whatever the planner
/// raises: a report the safety layer refused, a compatibility prefix that has
/// never been built, a component the store does not hold. Each of those is a
/// reason this install would fail, said before it is attempted instead of
/// during it.
#[tauri::command]
#[specta::specta]
pub fn khuttat_tathbeet(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<KhuttatTathbeetHie, Khata> {
    let id = huwiya(muarrif)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let taqreer = makhzan
        .bil_qira(|ittisal| SijillMuharrik::jadeed(ittisal).wahid(id))?
        .ok_or_else(|| {
            Khata::min_tafsir(&KhataTathbeetAmr::LaTathbeet {
                ism: luba.ism.clone(),
            })
        })?;

    let luba_muhallala = luba_lil_tarkib(&luba, &taqreer)?;
    let mukhattat =
        taarib_tathbeet::tarkib::khutta(&taqreer, &luba_muhallala, &masarat.mukawwinat())
            .map_err(Khata::from)?;
    Ok(KhuttatTathbeetHie::min_asli(&mukhattat))
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
    use taarib_aman::qaimat_sahb::{MuhawalatTajdid, NatijatMuhawala};
    use taarib_aman::tahaqquq_tawqee::SababTawqee;
    use taarib_makhzan::sijillat::{IdkhalLuba, SijillAlaab};
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
    const MUSAHIM: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    /// The game every refusal below is about.
    const ISM: &str = "Luba Ikhtibar";

    /// Whether a sentence carries any Arabic script at all.
    ///
    /// The assertion the English field exists for: a reader who chose English is
    /// shown a sentence with no Arabic in it, rather than the Arabic one under
    /// an English heading.
    fn fiha_arabi(nass: &str) -> bool {
        nass.chars()
            .any(|harf| matches!(harf, '\u{0600}'..='\u{06ff}' | '\u{0750}'..='\u{077f}'))
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
            masdar_khariji: None,
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
        let taqribi = mulakhkhas(vec!["12345".to_owned()], Vec::new(), "2026-01-02T00:00:00Z")?;
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
        assert!(
            thani.yahtaj_iqrar,
            "an approximate match asks for an acknowledgement"
        );

        for mudkhal in &mudkhalat {
            let tabaqa = mudkhal.mutabaqa.ok_or("a judged listing lost its tier")?;
            let arabi = mudkhal
                .mutabaqa_arabi
                .as_deref()
                .ok_or("no Arabic verdict")?;
            let injilizi = mudkhal
                .mutabaqa_injilizi
                .as_deref()
                .ok_or("no English verdict")?;
            assert!(arabi.contains(tabaqa.wasf_arabi()), "{arabi}");
            assert!(injilizi.contains(tabaqa.wasf_injilizi()), "{injilizi}");
            assert!(
                !fiha_arabi(injilizi),
                "an English verdict must hold no Arabic: {injilizi}"
            );
        }
        Ok(())
    }

    /// Every listing names the product it installs as the discriminant, not
    /// only as a number and an Arabic string.
    ///
    /// Each row draws its own install button, so each row has to say what
    /// pressing it produces — and an English session cannot read that off
    /// `tabaqa_arabi`. The number is checked against the same value so the two
    /// can never come to describe different tiers for one row.
    #[test]
    fn kull_mudkhal_yusammi_tabaqatahu_ka_ramz() -> NatijatIkhtibar {
        let basma = Basma::min_bayt([3u8; 32]);
        let luba = luba_bi_bina(Some("12345"), basma);
        let listing = mulakhkhas(vec!["12345".to_owned()], Vec::new(), "2026-01-01T00:00:00Z")?;
        let mutawaqqa = listing.tabaqa;

        let mudkhalat = rattib_murashshahat(&luba, vec![listing]);

        let wahid = mudkhalat.first().ok_or("the listing was dropped")?;
        assert_eq!(wahid.tabaqa, mutawaqqa);
        assert_eq!(wahid.tabaqa_raqm, mutawaqqa.raqm());
        assert_eq!(wahid.tabaqa_arabi, mutawaqqa.ism_arabi());
        Ok(())
    }

    /// With no build on record nothing is judged, in either language.
    #[test]
    fn bila_bina_la_hukm_bi_ayy_lugha() -> NatijatIkhtibar {
        let mut luba = luba_bi_bina(Some("12345"), Basma::min_bayt([3u8; 32]));
        luba.bina = None;
        let listing = mulakhkhas(vec!["12345".to_owned()], Vec::new(), "2026-01-01T00:00:00Z")?;

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
            Rafd::FahsMatjarLamYajri {
                masar: None,
                sabab: "NotFound".to_owned(),
            },
            Rafd::Tawqee(SababTawqee::GhayrMuwaqqaa),
            Rafd::Mulgha {
                sabab: "the signing key was withdrawn".to_owned(),
            },
            Rafd::ShabakaBilaIqrar(Box::new(ijmaa_shabaka())),
        ];

        let rumuz: Vec<u16> = rufud
            .iter()
            .map(|rafd| khata_rafd(rafd, ISM).ramz().raqm())
            .collect();
        let mufrada: BTreeSet<u16> = rumuz.iter().copied().collect();

        assert_eq!(
            mufrada.len(),
            rumuz.len(),
            "two refusals share one code: {rumuz:?}"
        );
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
        assert_eq!(
            khata.siyaq.get("ism"),
            Some(&QeemaSiyaq::Nass(ISM.to_owned()))
        );
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
        assert_eq!(
            khata.khutwa,
            Khutwa::FathIdadat {
                qism: QismIdadat::Manassat
            }
        );
        // The path is the mistake on this machine, so the refusal names it.
        assert!(khata.injilizi.contains("appinfo.vdf"), "{}", khata.injilizi);
        assert_eq!(
            khata.siyaq.get("masar"),
            Some(&QeemaSiyaq::Nass("/steam/appcache/appinfo.vdf".to_owned()))
        );
    }

    /// A scratch data root that removes itself, so an assertion that fails does
    /// not leave a cache behind in the machine's temporary directory.
    struct JidhrMuaqqat(PathBuf);

    impl Drop for JidhrMuaqqat {
        fn drop(&mut self) {
            // Best effort. A test that has already failed must not fail twice.
            #[expect(
                clippy::disallowed_methods,
                reason = "a scratch directory under `std::env::temp_dir()` removing itself, \
                          never a data root or a game directory"
            )]
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A data root of its own, with nothing in it.
    fn jidhr_muaqqat() -> (Masarat, JidhrMuaqqat) {
        let jidhr = std::env::temp_dir().join(format!("taarib-sahb-{}", uuid::Uuid::new_v4()));
        let haris = JidhrMuaqqat(jidhr.clone());
        (
            Masarat::min_judhur(jidhr.join("bayanat"), jidhr.join("idadat")),
            haris,
        )
    }

    /// The three states a machine that has not fetched can be in, and which of
    /// them installs.
    ///
    /// The whole of the finding this gate exists for: a machine that never went
    /// online, and one whose registry could not be reached, both install — this
    /// product is built to work with no internet — and neither is told the
    /// revocation check passed. A registry that answers and withholds its list
    /// is the one case that refuses.
    #[test]
    fn bawwabat_al_sahb_tasmah_lil_ghayr_muttasil_wa_tarfud_al_mahjuba() -> NatijatIkhtibar {
        let (masarat, _haris) = jidhr_muaqqat();
        let hali = Idadat::default();

        // Never fetched: installs, and says so rather than claiming a pass.
        let qaima = qaimat_sahb_lil_bawwaba(&masarat, &hali)?;
        assert_eq!(qaima.hala().ism(), "lam_tujlab");
        assert!(
            !qaima.hala().muhaddatha(),
            "a list nobody fetched is not a confirmed one"
        );
        assert!(sahb_hie(&qaima).injilizi.contains("has ever been fetched"));

        // Offline: still installs, and the reason is now on the record.
        QaimatSahb::sajjil_muhawala(
            &masarat,
            MuhawalatTajdid {
                waqt: Timestamp::now(),
                natija: NatijatMuhawala::MustawdaGhayrMutah {
                    sabab: "dns error: no such host".to_owned(),
                },
            },
        )?;
        let qaima = qaimat_sahb_lil_bawwaba(&masarat, &hali)?;
        assert!(!qaima.hala().muhaddatha());
        assert!(
            sahb_hie(&qaima)
                .injilizi
                .contains("could not reach the registry")
        );

        // Reachable and withholding: refused, with its own code and remedy.
        QaimatSahb::sajjil_muhawala(
            &masarat,
            MuhawalatTajdid {
                waqt: Timestamp::now(),
                natija: NatijatMuhawala::QaimaMutaadhdhira {
                    masdar: "forge https://example.invalid".to_owned(),
                    sabab: "sahb/qaima.json answered 404".to_owned(),
                },
            },
        )?;
        let khata = qaimat_sahb_lil_bawwaba(&masarat, &hali)
            .err()
            .ok_or("a reachable registry withholding its list must refuse")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 31);
        assert_eq!(khata.khutwa, Khutwa::AadaMuhawala);
        assert!(khata.injilizi.contains("404"), "{}", khata.injilizi);
        assert!(fiha_arabi(&khata.arabi));
        assert!(!fiha_arabi(&khata.injilizi), "{}", khata.injilizi);
        assert_eq!(
            khata.siyaq.get("masdar"),
            Some(&QeemaSiyaq::Nass(
                "forge https://example.invalid".to_owned()
            ))
        );
        Ok(())
    }

    /// The withheld-list refusal is told apart from every safety refusal, and
    /// from the generic install failure, by its code alone.
    #[test]
    fn rafd_al_sahb_lahu_ramz_yakhussuhu() {
        let fashal = taarib_mustawda::tathbeet_bilnaqra::FashalTathbeet::Sahb(
            taarib_aman::qaimat_sahb::RafdQaima {
                masdar: "mirror https://example.invalid".to_owned(),
                sabab: "the list did not verify".to_owned(),
                waqt: Timestamp::now(),
            },
        );

        let khata = khata_naqra(&fashal, ISM);

        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 31);
        // Not the bucket every non-safety failure collapses into: a withheld
        // revocation list and a game that is running are not one situation.
        assert_ne!(khata.ramz.raqm(), arqam::STUDIO + 29);
        // Nor any of the seven safety refusals.
        let rufud = [
            Rafd::IqrarNaqis,
            Rafd::Himaya(Box::new(ijmaa_himaya())),
            Rafd::FahsMatjarLamYajri {
                masar: None,
                sabab: "NotFound".to_owned(),
            },
            Rafd::MashHimayaLamYajri {
                jidhr: PathBuf::from("/luba-ikhtibar"),
            },
            Rafd::Tawqee(SababTawqee::GhayrMuwaqqaa),
            Rafd::Mulgha {
                sabab: "the signing key was withdrawn".to_owned(),
            },
            Rafd::ShabakaBilaIqrar(Box::new(ijmaa_shabaka())),
        ];
        for rafd in &rufud {
            assert_ne!(khata_rafd(rafd, ISM).ramz().raqm(), khata.ramz.raqm());
        }
        // "The registry withheld its list" and "this patch is revoked" are
        // opposite findings and must never wear one code.
        assert_ne!(
            khata.ramz.raqm(),
            khata_rafd(
                &Rafd::Mulgha {
                    sabab: String::new()
                },
                ISM
            )
            .ramz()
            .raqm()
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

    /// The whole re-ask, along the path `iqrar_aman` and `sajjil_iqrar_aman`
    /// take: a record written by a build that shipped statement version one is
    /// read off disk, reported as outstanding, and stops being outstanding only
    /// once the current statement is acknowledged — and the new record names
    /// the rendering that was read.
    #[test]
    fn iqrar_qadeem_yuad_talabuhu_thumma_yuqfal() -> NatijatIkhtibar {
        let (masarat, _haris) = jidhr_muaqqat();
        let malaf = masar_iqrar(&masarat);
        std::fs::create_dir_all(masarat.jidhr_bayanat())?;
        std::fs::write(
            &malaf,
            br#"{"isdar_nass":1,"waqt":"2026-01-01T00:00:00Z","isdar_taarib":"0.1.0"}"#,
        )?;

        let qadeem = iqrar_hie(iqrar::iqra(&malaf)?.as_ref());
        assert!(
            qadeem.yahtaj,
            "a record of statement version one is asked for again"
        );
        assert_eq!(qadeem.isdar_nass, iqrar::ISDAR_NASS);
        assert_eq!(qadeem.lugha_nass, None, "the old record names no rendering");
        assert!(qadeem.nass_arabi.contains("خدمة ترجمة Google المجانية"));
        assert!(
            qadeem
                .nass_injilizi
                .contains("Google Translate web service")
        );

        let jadeed = iqrar_hie(Some(&iqrar::ahfaz_bi_lugha(
            &malaf,
            "2026-09-18T00:00:00Z".to_owned(),
            ISDAR.to_owned(),
            Some(Lugha::Injilizi),
        )?));
        assert!(!jadeed.yahtaj, "the current statement is not asked twice");
        assert_eq!(jadeed.lugha_nass, Some(Lugha::Injilizi));

        let baad_ilaqa = iqrar_hie(iqrar::iqra(&malaf)?.as_ref());
        assert!(!baad_ilaqa.yahtaj, "and the answer survives a reread");
        assert_eq!(baad_ilaqa.lugha_nass, Some(Lugha::Injilizi));
        Ok(())
    }

    /// The one container the fixture game ships and the fixture recipe names.
    const HAWIYA: &str = "Luba_Data/resources.assets";

    /// A store, a game directory, and the library row a scan would have left.
    struct MasrahBina {
        makhzan: Makhzan,
        luba: Luba,
        mukhattat: MukhattatBasma,
        /// Held for its [`Drop`]; nothing reads it. Last so that it runs after
        /// the store's connection pool has closed — on Windows a directory
        /// holding an open database file will not delete.
        _jidhr: JidhrMuaqqat,
    }

    /// Sets one up, with the game's one container present or absent.
    ///
    /// The row goes in through [`SijillAlaab::sajjil`] with nothing else beside
    /// it, exactly as `main`'s library scan writes it, so what the measurement
    /// finds is what a scanned library really holds rather than anything this
    /// fixture arranged.
    fn masrah_bina(bil_hawiya: bool) -> Result<MasrahBina, Box<dyn std::error::Error>> {
        let jidhr = std::env::temp_dir().join(format!("taarib-bina-{}", uuid::Uuid::new_v4()));
        let haris = JidhrMuaqqat(jidhr.clone());
        let masarat = Masarat::min_judhur(jidhr.join("bayanat"), jidhr.join("idadat"));
        let makhzan = Makhzan::min_masar(&masarat.qaida_bayanat())?;

        let jidhr_luba = jidhr.join("luba");
        std::fs::create_dir_all(&jidhr_luba)?;
        if bil_hawiya {
            let masar = jidhr_luba.join(HAWIYA);
            if let Some(mujallad) = masar.parent() {
                std::fs::create_dir_all(mujallad)?;
            }
            std::fs::write(&masar, b"container bytes")?;
        }

        let masdar = MasdarLuba::Steam(480);
        let luba = Luba {
            id: LubaId::min_masdar(&masdar, ISM),
            masadir: vec![masdar],
            ism: ISM.to_owned(),
            jidhr: jidhr_luba,
            tanfidhi: None,
            hajm: 0,
            akhir_laab: None,
            akhir_tahdith: None,
            bina: None,
            suwar: SuwarLuba::default(),
            beea: BeeatTawafuq::Asli,
            mawjuda: true,
            mukhfiya: false,
        };
        makhzan.bi_muamala(|muamala| {
            SijillAlaab::jadeed(muamala).sajjil(&IdkhalLuba {
                luba: &luba,
                muktamila: true,
                khiyarat_tashghil: None,
                simat: &[],
                fahs: 1,
            })
        })?;

        Ok(MasrahBina {
            makhzan,
            luba,
            mukhattat: MukhattatBasma::min_masarat([HAWIYA])?,
            _jidhr: haris,
        })
    }

    /// The whole finding: a scanned game carries no build row, and installing
    /// into it used to stop there.
    ///
    /// Before the measurement the ledger is empty, which is the state every game
    /// in every library was permanently in. After it the row exists, the game
    /// points at it, and the listing screen has something to judge against.
    #[test]
    fn awwal_tathbeet_yaqees_al_bina_wa_yaktubuhu() -> NatijatIkhtibar {
        let masrah = masrah_bina(true)?;
        let id = masrah.luba.id;

        let qabl = masrah
            .makhzan
            .bil_qira(|ittisal| SijillBina::jadeed(ittisal).haliya(id))?;
        assert_eq!(qabl, None, "a scanned library records no build for a game");

        let bina = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)?;
        assert_eq!(bina.adad_malaffat, 1);
        assert_eq!(bina.manassa, None);

        let baad = masrah
            .makhzan
            .bil_qira(|ittisal| SijillBina::jadeed(ittisal).haliya(id))?
            .ok_or("the measurement was taken and not recorded")?;
        assert_eq!(baad.basma, bina.basma);
        assert_eq!(baad.adad_malaffat, 1);
        Ok(())
    }

    /// A second install measures again rather than trusting the row it wrote,
    /// and a game the store has changed underneath produces a different build.
    ///
    /// The reason the value is not read back out of the store: it is the
    /// evidence `IrtibatBina::ihkum` decides on, and a remembered fingerprint
    /// would pass a patch onto files that have been replaced since. Measuring
    /// the same unchanged game twice adds no row — the ledger is keyed by the
    /// fingerprint — so the cost of honesty here is one upsert.
    #[test]
    fn al_qiyas_yutakarrar_wa_yatba_al_luba_hina_tataghayyar() -> NatijatIkhtibar {
        let masrah = masrah_bina(true)?;
        let id = masrah.luba.id;

        let awwal = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)?;
        let thani = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)?;
        assert_eq!(
            awwal.basma, thani.basma,
            "the same files fingerprint the same"
        );
        let tarikh = masrah
            .makhzan
            .bil_qira(|ittisal| SijillBina::jadeed(ittisal).tarikh(id))?;
        assert_eq!(tarikh.len(), 1, "an unchanged build is one row, not two");

        std::fs::write(
            masrah.luba.jidhr.join(HAWIYA),
            b"the store pushed an update",
        )?;
        let baad_tahdith = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)?;
        assert_ne!(
            baad_tahdith.basma, awwal.basma,
            "a changed container must not fingerprint as the old build"
        );
        let haliya = masrah
            .makhzan
            .bil_qira(|ittisal| SijillBina::jadeed(ittisal).haliya(id))?
            .ok_or("the game lost its current build")?;
        assert_eq!(haliya.basma, baad_tahdith.basma);
        assert_eq!(
            masrah
                .makhzan
                .bil_qira(|ittisal| SijillBina::jadeed(ittisal).tarikh(id))?
                .len(),
            2,
            "the build it was at is kept beside the one it is at"
        );
        Ok(())
    }

    /// A game that is not on disk refuses with the step that puts it back.
    #[test]
    fn luba_ghayr_mawjuda_turfad_bi_ikhtiyar_al_mujallad() -> NatijatIkhtibar {
        let mut masrah = masrah_bina(false)?;
        #[expect(
            clippy::disallowed_methods,
            reason = "the scratch game directory this fixture made, removed to stage the game \
                      being uninstalled; never a real game directory"
        )]
        std::fs::remove_dir_all(&masrah.luba.jidhr)?;
        masrah.luba.mawjuda = false;

        let khata = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)
            .err()
            .ok_or("a game that is gone cannot be measured")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 32);
        assert_eq!(
            khata.khutwa,
            Khutwa::IkhtiyarMasar {
                matlub: MasarMatlub::MujalladLuba
            }
        );
        assert!(fiha_arabi(&khata.arabi));
        assert!(!fiha_arabi(&khata.injilizi), "{}", khata.injilizi);
        assert!(khata.injilizi.contains("Nothing was written"));
        assert_eq!(
            khata.siyaq.get("jidhr"),
            Some(&QeemaSiyaq::Masar(masrah.luba.jidhr))
        );
        Ok(())
    }

    /// A game that is there and will not measure refuses with a different step,
    /// and keeps the number the old dead refusal had.
    #[test]
    fn hawiya_mafquda_turfad_bi_tahaqquq_salamat_al_luba() -> NatijatIkhtibar {
        let masrah = masrah_bina(false)?;

        let khata = qis_bina(&masrah.makhzan, &masrah.luba, &masrah.mukhattat)
            .err()
            .ok_or("a recipe naming a file that is not there cannot be run")?;
        assert_eq!(khata.ramz.raqm(), arqam::STUDIO + 28);
        assert_eq!(khata.khutwa, Khutwa::TahaqquqSalamatLuba);
        assert!(fiha_arabi(&khata.arabi));
        assert!(!fiha_arabi(&khata.injilizi), "{}", khata.injilizi);
        assert!(
            khata.injilizi.contains("resources.assets"),
            "the sentence names the container that is missing: {}",
            khata.injilizi
        );
        assert!(
            !khata.injilizi.contains("probe"),
            "the step this refusal names must be one that can be taken: {}",
            khata.injilizi
        );
        assert_eq!(
            khata.siyaq.get("ism"),
            Some(&QeemaSiyaq::Nass(ISM.to_owned()))
        );
        Ok(())
    }
}
