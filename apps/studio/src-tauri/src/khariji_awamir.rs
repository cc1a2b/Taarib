//! الرقع الخارجية — patches other teams made.
//!
//! The catalogue the game screen lists, what is already in the game that one of
//! them cannot sit beside, the install, and the owner's re-pin of an artifact
//! whose bytes have moved.
//!
//! Nothing here decides anything. Every judgement belongs to a crate:
//! `taarib_mustawda` says what the registry lists, `taarib_tathbeet::tasadum`
//! says what is in the way, `taarib_aman::fahs_khariji` is the only place an
//! install authorisation is minted, `taarib_tathbeet::khariji` writes and
//! records, and `taarib_taqdeem::nashr_mustawda` holds the owner's ledger. This
//! module assembles their inputs, projects their answers for the interface, and
//! supplies the one thing none of them may hold — an HTTP client.
//!
//! ## Why the transport lives here
//!
//! `taarib_tathbeet::jalb_khariji::NaqilKhariji` is a port, and the crate says
//! why: a component whose other job is restoring people's games must not carry
//! a network stack. The application already has one, with this product's user
//! agent and this product's timeouts, so it supplies the implementation. The
//! verification stays where it was — the size and the digest are reproduced by
//! `jalb_khariji` over the file on disk, so a transport can never be the
//! component that decides an artifact matched.
//!
//! The install itself is synchronous and the client is asynchronous, so the two
//! are bridged by a channel rather than by a nested runtime: the install runs on
//! a blocking thread and asks for each artifact through
//! [`NaqilQanat`](struct@NaqilQanat), and the command's own task serves those
//! requests while it waits for the install to finish.
//!
//! ## Where third-party bytes are allowed to land
//!
//! In the install quarantine, never in a game. `jalb_khariji` carves a
//! per-entry directory under [`Masarat::hajr`] and proves both halves of the pin
//! there; only then does anything reach the game directory, and every write that
//! does goes through the manifest with the original beside it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use jiff::Timestamp;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use taarib_aman::fahs_khariji::{NatijatFahsKhariji, RafdKhariji, TalabFahsKhariji, fahs_khariji};
use taarib_makhzan::wasl::Makhzan;
use taarib_mustalahat::bina::BinaId;
use taarib_mustalahat::khariji::{
    HalatMira, QitaatTanzeel, RuqaaKharijiya, TahdheerKhariji, TakhtitKhariji,
};
use taarib_mustalahat::luba::{Luba, LubaId};
use taarib_mustalahat::ruqaa::{MasdarKhariji, RuqaaId, RuqaaRevision};
use taarib_mustawda::jalb_fahras;
use taarib_mustawda::khariji::badhrat_kharijiya;
use taarib_mustawda::masadir::MUHLAT_ITTISAL;
use taarib_taqdeem::nashr_mustawda::SijillNashr;
use taarib_tathbeet::bayan::{NawTathbeet, TarifLuba};
use taarib_tathbeet::jalb_khariji::{AQSA_HAJM_QITAA, NaqilKhariji};
use taarib_tathbeet::khariji::{
    TalabTathbeetKhariji, azil_khariji, jidhr_nusakh_khariji, thabbit_khariji,
};
use taarib_tathbeet::khata::{KhataTathbeet, NatijatTathbeet};
use taarib_tathbeet::masar_tathbeet::la_tashtaghil;
use taarib_tathbeet::taraju::SiyasatIstiada;
use taarib_tathbeet::tasadum::{AtharTasadum, JihatTasadum, masah_tasadum};
use taarib_tathbeet::wukala::{WakeelQaim, masah, shaghil_slot, wakeel_nizam};
use taarib_usus::ISDAR;
use taarib_usus::idadat::{Idadat, MakhzanIdadat};
use taarib_usus::khata::{Khata, Khutura, Khutwa, Natija, QeemaSiyaq, Ramz, Tafsir, arqam};
use taarib_usus::khata_min;
use taarib_usus::masarat::{self, Masarat};

use crate::luba_awamir::{huwiya, ijlib_luba, jidhr_nusakh, jidhr_steam_lil_fahs, muthabbat};
use crate::tathbeet_awamir::{
    KhataTathbeetAmr, TaqreerIzalaHie, bil_hajb, huwiyat_ruqaa, masar_iqrar, nafidhat_sahb,
    silsilat_masadir, taqreer_izala_hie,
};

/// Where the watch's observations are cached, under the registry cache.
const MASAR_MURAQABA: &str = "khariji/muraqaba.json";

/// The quarantine subdirectory third-party artifacts are staged in.
///
/// Under [`Masarat::hajr`] and therefore never inside a game, which is the one
/// thing `jalb_khariji` requires of the directory it is handed.
const MUJALLAD_MARHALA: &str = "khariji";

/// How long one artifact's transfer may stall before it is abandoned.
///
/// A read timeout rather than a whole-request one, for the reason the package
/// downloader gives: a ceiling on the whole transfer would abort a large archive
/// on a slow line, and a ceiling on the gap between bytes catches the thing that
/// actually goes wrong — a server that accepted the connection and stopped.
const MUHLAT_QITA: Duration = Duration::from_secs(45);

/// How many redirects one artifact fetch will follow.
const HADD_TAHWIL: usize = 3;

// ---------------------------------------------------------------------------
// What the interface reads
// ---------------------------------------------------------------------------

/// Where a third-party entry's bytes come from, as the catalogue panel reads it.
///
/// A projection rather than [`HalatMira`] itself. The shared type is tagged
/// internally, because that is the shape the registry's shards are published in
/// and a shard's bytes decide its hash; the interface decodes the ordinary
/// external tagging. Projecting here changes nothing about the catalogue on
/// disk and keeps the two spellings from meeting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum HalatMiraHie {
    /// Fetched from the author, always. The default.
    MinAlmuallif,
    /// Mirrored on the registry.
    MinAlsijill {
        /// The registry's own endpoint for the artifact.
        rabt: String,
    },
}

impl HalatMiraHie {
    /// One entry's mirroring state, projected.
    fn min_asli(mira: &HalatMira) -> Self {
        match mira {
            HalatMira::MinAlmuallif => Self::MinAlmuallif,
            HalatMira::MinAlsijill { rabt } => Self::MinAlsijill { rabt: rabt.clone() },
        }
    }
}

/// One third-party entry, as the game screen lists it.
///
/// The catalogue record whole, plus the two facts that are about this machine
/// rather than about the entry: which build it was matched against here, and
/// whether it is installed here right now.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct MudkhalKharijiHie {
    /// The lineage the catalogue lists it under.
    pub id: RuqaaId,
    /// The title, as the work is known.
    pub unwan: String,
    /// The game it patches.
    pub luba: LubaId,
    /// The game as each launcher names it.
    pub hawiyat_manassa: Vec<String>,
    /// The game builds it supports; empty means every build.
    pub abniya: Vec<String>,
    /// Who made it, where, under what licence, and what permits carrying it.
    pub masdar: MasdarKhariji,
    /// The team name, when the work is a team's.
    pub fariq: Option<String>,
    /// The version the author released.
    pub isdar: String,
    /// Everything fetched, each pinned.
    pub qitaa: Vec<QitaatTanzeel>,
    /// What the install writes and what it clears first.
    pub takhtit: TakhtitKhariji,
    /// The author's own safety sentences, both languages.
    pub tahdheerat: Vec<TahdheerKhariji>,
    /// Where the bytes come from.
    pub mira: HalatMiraHie,
    /// The installed build this entry was matched against, or [`None`] when no
    /// build has been measured for this game.
    ///
    /// The interface lists every artifact when this is absent, because the
    /// honest answer to "which of these apply" is then "we cannot tell".
    pub bina_mutabaqa: Option<String>,
    /// Whether this entry is installed into this game right now.
    pub muthabbata: bool,
}

/// What is already in the game directory that one entry cannot sit beside.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct TadakhulKharijiHie {
    /// Whether anything conflicting is there right now.
    pub mutadakhil: bool,
    /// What is installed, named, in Arabic.
    pub sahib_arabi: String,
    /// The same, in English.
    pub sahib_injilizi: String,
    /// The loader slot the two are fighting over, e.g. `dinput8.dll`.
    pub manfadh: String,
    /// Paths, relative to the game root, that must go first.
    pub yuzal: Vec<String>,
    /// Whether Taarib's own removal can do it.
    ///
    /// True only when what is in the way is Taarib's own install, which the
    /// removal strip on the same screen takes off and restores. Another team's
    /// files are theirs; Taarib did not write them and does not delete them.
    pub taarib_yuzil: bool,
}

/// What an install of a third-party entry actually did.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct NatijatKharijiyaHie {
    /// Whether every fetched artifact reproduced its pinned hash.
    pub basmat_mutabiqa: bool,
    /// How many paths the archives wrote into the game.
    pub adad_maktub: u32,
    /// How many of the author's listed removals were backed up before removal.
    pub adad_muhtafaz: u32,
    /// Where the record a removal reads back lives.
    pub sijill: String,
}

/// One artifact whose bytes have moved since the owner pinned them.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct BasmaMualaqaHie {
    /// The entry the changed artifact belongs to.
    pub ruqaa: RuqaaId,
    /// The entry's title.
    pub unwan: String,
    /// Who made it. A console row is still somebody else's work.
    pub muallif: String,
    /// Their team, when the work is a team's.
    pub fariq: Option<String>,
    /// Their page.
    pub rabt_muallif: String,
    /// The release the entry declares the author ships.
    pub isdar: String,
    /// The release the standing pin was taken against.
    pub isdar_mathbut: String,
    /// Which artifact changed.
    pub qitaa: String,
    /// Where it was fetched from.
    pub rabt: String,
    /// Bytes, as the author's endpoint served them.
    #[specta(type = specta_typescript::Number)]
    pub hajm: u64,
    /// Bytes, as the standing pin records them.
    #[specta(type = specta_typescript::Number)]
    pub hajm_mathbut: u64,
    /// The hash the owner pinned, lowercase hex.
    pub sha256_mathbut: String,
    /// The hash the artifact actually has now, lowercase hex.
    pub sha256_marsud: String,
    /// When the change was observed, RFC 3339.
    pub waqt: String,
}

// ---------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------

/// Every third-party patch the registry lists for one game.
///
/// The registry's own listing path: only the shard this game's identity falls
/// in is fetched, and a shard the manifest still vouches for is read from the
/// cache. The compiled-in seed catalogue is listed beside it for any entry the
/// shard does not already carry, so a machine whose registry has not published
/// one yet still sees what this build knows about — credited, explained, and
/// with its install refused by the gate for exactly the reason the entry
/// carries.
///
/// # Errors
///
/// [`KhataTathbeetAmr::GhayrMuttasil`] when offline mode leaves no source to
/// try, and whatever the store, the registry client or the game lookup raise.
#[tauri::command]
#[specta::specta]
pub async fn ruqaa_kharijiya(
    muarrif: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<Vec<MudkhalKharijiHie>, Khata> {
    let id = huwiya(muarrif)?;
    let hali = idadat.hali();
    let makhbaa = masarat.makhbaa();

    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    let madakhil = katalog_luba(&hali, id, &makhbaa).await?;
    sajjil_mudifin(&madakhil);

    let nusakh = {
        let masarat = Masarat::clone(&masarat);
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || jidhr_nusakh(&masarat, &makhzan, id)).await?
    };

    // The manifests live on disk, so the installed-or-not question is a
    // filesystem walk per entry and belongs off the window's thread.
    let jidhr_luba = luba.jidhr.clone();
    let huwiyat: Vec<RuqaaId> = madakhil.iter().map(|madkhal| madkhal.id).collect();
    let mathbita = bil_hajb(move || {
        Ok(huwiyat
            .into_iter()
            .map(|id| {
                muthabbat(
                    &jidhr_luba,
                    &jidhr_nusakh_khariji(&nusakh, id),
                    NawTathbeet::Nass,
                )
            })
            .collect::<Vec<bool>>())
    })
    .await?;

    let bina = luba.bina.as_ref().map(BinaId::wasm);
    Ok(madakhil
        .into_iter()
        .zip(mathbita)
        .map(|(madkhal, muthabbata)| hie_madkhal(madkhal, bina.clone(), muthabbata))
        .collect())
}

/// What is already in one game's directory that one entry cannot sit beside.
///
/// The same two refusals `thabbit_khariji` runs before it writes anything, in
/// the same order — Taarib's own install first, then somebody else's — so the
/// report and the refusal can never disagree about which one is in the way.
/// Nothing is written and nothing is fetched beyond the catalogue read that
/// establishes the entry is real.
///
/// # Errors
///
/// [`KhataKharijiAmr::MadkhalGhayrMawjud`] when the catalogue lists no such
/// entry for this game, and whatever the store, the registry client or the game
/// lookup raise.
#[tauri::command]
#[specta::specta]
pub async fn tadakhul_kharijiya(
    muarrif: String,
    ruqaa: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<TadakhulKharijiHie, Khata> {
    let id = huwiya(muarrif)?;
    let matlub = huwiyat_ruqaa(ruqaa)?;
    let hali = idadat.hali();
    let makhbaa = masarat.makhbaa();

    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    let madakhil = katalog_luba(&hali, id, &makhbaa).await?;
    sajjil_mudifin(&madakhil);
    let madkhal = madkhal_bi_huwiya(madakhil, matlub, &luba.ism)?;

    let nusakh = {
        let masarat = Masarat::clone(&masarat);
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || jidhr_nusakh(&masarat, &makhzan, id)).await?
    };

    // The marker sweep reads the game directory and the loader survey opens
    // every proxy module beside the executable to identify it.
    let jidhr_luba = luba.jidhr.clone();
    bil_hajb(move || Ok(hie_tadakhul(&jidhr_luba, &nusakh, &madkhal.takhtit))).await
}

/// Fetches one third-party entry against its pins and installs it.
///
/// The order is the installer's and is not negotiable here: the gate is asked
/// first and is the only thing that can mint the permit, the permit is compared
/// against the entry before a byte moves, every artifact is fetched into the
/// quarantine and proved against both halves of its pin there, and only then is
/// the manifest opened and anything written into the game.
///
/// `iqrar` is the person's acknowledgement of the warnings the entry carries and
/// of the online-play sentence the gate composes itself. It is passed through to
/// [`TalabFahsKhariji::iqrar_tahdheerat`] and nowhere else: a caller that showed
/// nothing and sends `true` is lying to its own user, and a caller that sends
/// `false` gets back the list it should have shown.
///
/// # Errors
///
/// [`KhataKharijiAmr::MadkhalGhayrMawjud`] when the catalogue lists no such
/// entry, [`KhataKharijiAmr::BinaMajhula`] when the entry pins different files
/// to different builds and no build has been measured for this game,
/// [`KhataKharijiAmr::BawwabaRafadat`] carrying the gate's own refusal, and
/// whatever the installer raises — a collision, a running game, a transfer that
/// did not complete, or an artifact whose bytes did not reproduce its pin.
#[tauri::command]
#[specta::specta]
pub async fn thabbit_kharijiya(
    muarrif: String,
    ruqaa: String,
    iqrar: bool,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<NatijatKharijiyaHie, Khata> {
    let id = huwiya(muarrif)?;
    let matlub = huwiyat_ruqaa(ruqaa)?;
    let hali = idadat.hali();
    let makhbaa = masarat.makhbaa();

    let luba = {
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || ijlib_luba(&makhzan, id)).await?
    };

    let madakhil = katalog_luba(&hali, id, &makhbaa).await?;
    sajjil_mudifin(&madakhil);
    let madkhal = madkhal_bi_huwiya(madakhil, matlub, &luba.ism)?;

    let bina = bina_lil_fahs(luba.bina.as_ref().map(BinaId::wasm), &luba.ism, &madkhal)?;
    let tanfidhi = ism_tanfidhi(&luba)?;
    let jidhr_steam = jidhr_steam_lil_fahs(&masarat, &hali, &luba)?;
    let appid = crate::luba_awamir::appid_steam(&luba);
    let sijill_iqrar = taarib_aman::iqrar::iqra(&masar_iqrar(&masarat))?;

    let nusakh = {
        let masarat = Masarat::clone(&masarat);
        let makhzan = Makhzan::clone(&makhzan);
        bil_hajb(move || jidhr_nusakh(&masarat, &makhzan, id)).await?
    };
    let marhala = masarat.hajr().join(MUJALLAD_MARHALA);

    let Some(masdar) = luba.masadir.first().cloned() else {
        return Err(Khata::from(KhataTathbeetAmr::LaTathbeet {
            ism: luba.ism.clone(),
        }));
    };
    let tarif = TarifLuba {
        luba: id,
        masdar,
        ism: luba.ism.clone(),
        jidhr: luba.jidhr.clone(),
        // The manifest records the entry's own lineage; the installer refuses
        // outright when it would record anything else.
        ruqaa: madkhal.id,
        // A third-party entry has no Taarib revision — nothing here was
        // compiled, reviewed or sealed — so the manifest records the first.
        murajaa: RuqaaRevision::AWWAL,
        basma_bina: luba.bina.as_ref().map(|bina| bina.basma),
    };

    let amil = amil_khariji()?;
    let (mursil, mut mutalaqqi) = tokio::sync::mpsc::channel::<TalabJalb>(1);

    let ism_luba = luba.ism.clone();
    let muhimma = tokio::task::spawn_blocking(move || -> Natija<NatijatKharijiyaHie> {
        let talab_fahs = TalabFahsKhariji {
            luba: id,
            ism_luba: &ism_luba,
            jidhr_luba: &tarif.jidhr,
            appid,
            jidhr_steam: jidhr_steam.as_deref(),
            bina: &bina,
            ruqaa: &madkhal,
            iqrar: sijill_iqrar.as_ref(),
            iqrar_tahdheerat: iqrar,
        };
        // The one place a third-party install authorisation exists. There is no
        // other constructor and nothing here builds one.
        let idhn = match fahs_khariji(&talab_fahs) {
            NatijatFahsKhariji::Masmuh(idhn) => idhn,
            NatijatFahsKhariji::Marfud(rafd) => return Err(khata_rafd(&rafd, &ism_luba)),
        };

        let mut naqil = NaqilQanat { mursil };
        let talab = TalabTathbeetKhariji {
            luba: &tarif,
            ruqaa: &madkhal,
            tanfidhi: &tanfidhi,
            jidhr_nusakh: &nusakh,
            jidhr_tajmee: &marhala,
        };
        let natija = thabbit_khariji(&talab, idhn, &mut naqil).map_err(Khata::from)?;

        Ok(NatijatKharijiyaHie {
            // Reaching this line is the proof. `ijlib_qitaa` reproduces the
            // declared size and the declared sha256 over the staged file for
            // every artifact the permit approved, and refuses by name — with
            // the disagreeing copy deleted — the moment either half differs, so
            // an install that returned a result is an install whose every
            // artifact matched.
            basmat_mutabiqa: true,
            adad_maktub: adad(natija.adad_maktub),
            adad_muhtafaz: adad(natija.adad_mahdhuf),
            sijill: natija.jidhr_nusakh.to_string_lossy().into_owned(),
        })
    });

    // The install blocks on each artifact; this is what answers it. The loop
    // ends when the blocking task drops the transport, which is the moment it
    // has finished with or without a result.
    while let Some(talab) = mutalaqqi.recv().await {
        let natija = jalb_ila_malaf(&amil, &talab.ism, &talab.rabt, &talab.hadaf).await;
        drop(talab.radd.send(natija));
    }

    let natija = muhimma.await.map_err(|sabab| {
        Khata::from(KhataTathbeetAmr::MuhimmaMutawaqqifa {
            tafsil: sabab.to_string(),
        })
    })??;

    tracing::info!(
        luba = %id,
        ruqaa = %matlub,
        maktub = natija.adad_maktub,
        muhtafaz = natija.adad_muhtafaz,
        "a third-party patch was fetched against its pins and installed"
    );
    Ok(natija)
}

/// Takes one third-party patch back off, and puts the game back.
///
/// The reason this product is worth using for a patch it did not build. The
/// author's own installer writes into the game and keeps no record of what was
/// there before; this went in through the same manifest a Taarib patch does,
/// with the original bytes of every file it overwrote **and** of every file the
/// author's instructions told it to delete first. So the way out is the restore
/// machinery unchanged — the one that verifies every byte against its recorded
/// fingerprint, reapplies timestamps and permissions, resumes an interrupted
/// run, and refuses to call a partial removal a success.
///
/// It reads this entry's own backup root and no other, which is what keeps two
/// patches installed side by side from removing each other's files.
///
/// # Errors
///
/// [`KhataTathbeetAmr`] when the identity is not a game or the game is not in
/// the store, [`taarib_tathbeet::khata::KhataTathbeet::LubaTashtaghil`] while
/// the game is running, and whatever the restore refuses — a replaced file
/// under [`SiyasatIstiada::Rafd`], a fingerprint that does not match, a backup
/// that cannot be read.
#[tauri::command]
#[specta::specta]
pub async fn azil_kharijiya(
    muarrif: String,
    ruqaa: String,
    masarat: tauri::State<'_, Masarat>,
    makhzan: tauri::State<'_, Makhzan>,
) -> Result<TaqreerIzalaHie, Khata> {
    let id = huwiya(muarrif)?;
    let lineage = huwiyat_ruqaa(ruqaa)?;
    let luba = ijlib_luba(&makhzan, id)?;
    let nusakh = jidhr_nusakh(&masarat, &makhzan, id)?;

    if let Some(tanfidhi) = luba.tanfidhi.as_deref()
        && let Some(nass) = tanfidhi.to_str()
    {
        la_tashtaghil(nass).map_err(Khata::from)?;
    }

    let jidhr_luba = luba.jidhr.clone();
    // The restore walks a whole game directory and rewrites files, so it runs
    // off the thread the window is painted from, like every other install path
    // here.
    let taqreer = tokio::task::spawn_blocking(move || {
        // `Muhafiza`: a file the store has since replaced with its own newer
        // copy keeps that copy. Taarib's bytes are already gone from it, and
        // writing a months-old original back over a store update would undo the
        // store rather than this patch.
        azil_khariji(&jidhr_luba, &nusakh, lineage, SiyasatIstiada::Muhafiza)
    })
    .await
    .map_err(|sabab| {
        Khata::from(KhataTathbeetAmr::MuhimmaMutawaqqifa {
            tafsil: sabab.to_string(),
        })
    })?
    .map_err(Khata::from)?;

    Ok(taqreer_izala_hie(taqreer.naw, &taqreer))
}

/// Every pinned artifact whose author is now serving different bytes.
///
/// The owner's, and only the owner's: the pins are theirs, the ledger this
/// reads is the one the next cast rebuilds the catalogue from, and accepting a
/// change is a decision nobody else can take.
///
/// Each artifact is fetched from the author's own endpoint and hashed; nothing
/// is written anywhere, because the answer is a digest over a stream. The poll
/// is bounded by the registry's own refresh window — the interval
/// `masadir.fatra_tahdith` names, which the manifest and the revocation list
/// already refresh on — so a console opened twice in that window asks the
/// author's server once. There is no second schedule and no second timer.
///
/// # Errors
///
/// [`crate::taqdeem_awamir::KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// and whatever the ledger or the observation cache raise. A poll that could not
/// reach an endpoint is logged and leaves that artifact's last observation
/// standing, because "the author's server did not answer" is not "the bytes
/// changed".
#[tauri::command]
#[specta::specta]
pub async fn basmat_mualaqa(
    masarat: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<Vec<BasmaMualaqaHie>, Khata> {
    let _salahiya = crate::taqdeem_awamir::salahiyat_malik()?;
    let hali = idadat.hali();

    let masar_sijill = SijillNashr::masar(&masarat).map_err(Khata::from)?;
    let madakhil: Vec<RuqaaKharijiya> = {
        let sijill = SijillNashr::iftah(&masar_sijill).map_err(Khata::from)?;
        sijill.kharijiya().cloned().collect()
    };
    sajjil_mudifin(&madakhil);

    let masar_muraqaba = masar_muraqaba(&masarat)?;
    let mut muraqaba = SijillMuraqaba::iftah(&masar_muraqaba)?;
    let nafidha = nafidhat_sahb(&hali);
    let alaan = Timestamp::now();

    let amil = if hali.masadir.wadaa_ghayr_muttasil {
        None
    } else {
        Some(amil_khariji()?)
    };

    let mut mualaqa = Vec::new();
    let mut taghayyar = false;
    for madkhal in &madakhil {
        for qitaa in &madkhal.qitaa {
            let miftah = miftah_rasd(madkhal.id, &qitaa.ism);
            // An observation taken inside the window against exactly this pin
            // is not asked for again: it would be the same question to the same
            // server, and the window is the registry's own.
            if !muraqaba.hadith(&miftah, qitaa, nafidha, alaan)
                && let Some(amil) = amil.as_ref()
            {
                // The release the standing pin belongs to, carried forward while
                // the pin itself has not moved. A ledger whose `isdar` was bumped
                // without the artifacts being re-pinned is exactly the case the
                // console's two version columns exist to show, and overwriting
                // this with the new release would hide it.
                let isdar_mathbut = muraqaba
                    .rasd(&miftah)
                    .filter(|rasd| rasd.yuqaran(qitaa))
                    .map_or_else(|| madkhal.isdar.clone(), |rasd| rasd.isdar_mathbut.clone());
                match arsud(amil, &qitaa.ism, &qitaa.rabt).await {
                    Ok((sha256, hajm)) => {
                        muraqaba.sajjil(
                            miftah.clone(),
                            RasdQitaa {
                                sha256_mathbut: qitaa.sha256.to_lowercase(),
                                hajm_mathbut: qitaa.hajm,
                                isdar_mathbut,
                                sha256,
                                hajm,
                                waqt: alaan.to_string(),
                            },
                        );
                        taghayyar = true;
                    },
                    Err(khata) => {
                        tracing::warn!(
                            ruqaa = %madkhal.id,
                            qitaa = %qitaa.ism,
                            khata = %khata.li_sijill(),
                            "a pinned artifact could not be read, so its last observation stands"
                        );
                    },
                }
            }

            let Some(rasd) = muraqaba.rasd(&miftah) else {
                continue;
            };
            // The pin may have moved under a standing observation — the owner
            // re-pinned, or edited the ledger — in which case the observation
            // describes a comparison nobody is making any more.
            if !rasd.yuqaran(qitaa) || rasd.mutabiq() {
                continue;
            }
            mualaqa.push(hie_basma(madkhal, qitaa, rasd));
        }
    }

    if taghayyar {
        muraqaba.ihfadh(&masar_muraqaba)?;
    }
    Ok(mualaqa)
}

/// The owner accepting one artifact's new bytes, by echoing back their hash.
///
/// `sha256` is what the owner read off the console and transcribed, and it is
/// checked twice over: the artifact is fetched from the author's endpoint again
/// here, and the pin is moved only if the bytes that arrive hash to exactly what
/// was sent. A server that changes what it serves between the observation and
/// the acceptance therefore pins nothing — which is the entire reason the hash
/// travels in the request rather than being looked up from the last poll.
///
/// The new pin is written into the owner's publication ledger, through the same
/// gate that admits any third-party entry to it, so the next cast publishes it
/// and every client verifies against it. Nothing on this machine installs
/// anything differently until that cast happens: a client's install still checks
/// the bytes it fetched against the pin the registry published, and this command
/// touches neither.
///
/// It answers `true`. The shape was never agreed and the interface ignores the
/// value — it refetches the pending set instead — so the answer is the smallest
/// thing that can mean "accepted", and the refusals carry everything else.
///
/// # Errors
///
/// [`crate::taqdeem_awamir::KhataTaqdeemAmr::MalikFaqat`] without the owner key,
/// [`KhataKharijiAmr::MadkhalGhayrMawjud`] when the ledger lists no such entry,
/// [`KhataKharijiAmr::QitaaGhayrMawjuda`] when it carries no such artifact,
/// [`KhataKharijiAmr::BasmaGhayrSaliha`] when what was sent is not a sha256,
/// [`KhataKharijiAmr::BasmaGhayrMutabaqa`] when the endpoint is not serving those
/// bytes, [`KhataKharijiAmr::RasdMutaadhdhir`] when it could not be read at all,
/// and whatever the ledger's own gate refuses.
#[tauri::command]
#[specta::specta]
pub async fn athbit_basma(
    ruqaa: String,
    qitaa: String,
    sha256: String,
    masarat: tauri::State<'_, Masarat>,
    idadat: tauri::State<'_, Arc<MakhzanIdadat>>,
) -> Result<bool, Khata> {
    let _salahiya = crate::taqdeem_awamir::salahiyat_malik()?;
    let matlub = huwiyat_ruqaa(ruqaa)?;
    let hali = idadat.hali();

    let maqbula = sha256.trim().to_ascii_lowercase();
    if maqbula.len() != 64 || !maqbula.bytes().all(|harf| harf.is_ascii_hexdigit()) {
        return Err(Khata::from(KhataKharijiAmr::BasmaGhayrSaliha {
            qitaa: qitaa.clone(),
            basma: sha256,
        }));
    }

    let masar_sijill = SijillNashr::masar(&masarat).map_err(Khata::from)?;
    let mut sijill = SijillNashr::iftah(&masar_sijill).map_err(Khata::from)?;
    let mut madkhal =
        sijill
            .khariji(matlub)
            .cloned()
            .ok_or_else(|| KhataKharijiAmr::MadkhalGhayrMawjud {
                ruqaa: matlub.to_string(),
                ism: "the publication ledger".to_owned(),
            })?;
    let Some(rabt) = madkhal
        .qitaa
        .iter()
        .find(|wahid| wahid.ism == qitaa)
        .map(|wahid| wahid.rabt.clone())
    else {
        return Err(Khata::from(KhataKharijiAmr::QitaaGhayrMawjuda {
            ruqaa: matlub.to_string(),
            qitaa,
        }));
    };

    if hali.masadir.wadaa_ghayr_muttasil {
        return Err(Khata::from(KhataKharijiAmr::RasdMutaadhdhir {
            qitaa,
            rabt: String::new(),
            sabab: "offline mode is on, and a pin is never moved onto bytes nobody read".to_owned(),
        }));
    }

    let amil = amil_khariji()?;
    let (marsuda, hajm) = arsud(&amil, &qitaa, &rabt).await?;
    if marsuda != maqbula {
        return Err(Khata::from(KhataKharijiAmr::BasmaGhayrMutabaqa {
            qitaa,
            rabt,
            maqbula,
            marsuda,
        }));
    }

    for wahid in &mut madkhal.qitaa {
        if wahid.ism == qitaa {
            wahid.sha256.clone_from(&maqbula);
            wahid.hajm = hajm;
        }
    }
    // Back through the gate rather than written straight in: this is the same
    // door any third-party entry enters the ledger by, and a re-pin is not a
    // reason to walk past it.
    sijill.sajjil_khariji(madkhal).map_err(Khata::from)?;
    sijill.ihfadh(&masar_sijill).map_err(Khata::from)?;

    // The row's comparison is now against the bytes that were just read, so it
    // leaves the pending list without waiting for the window to turn over.
    let masar_muraqaba = masar_muraqaba(&masarat)?;
    let mut muraqaba = SijillMuraqaba::iftah(&masar_muraqaba)?;
    muraqaba.sajjil(
        miftah_rasd(matlub, &qitaa),
        RasdQitaa {
            sha256_mathbut: maqbula.clone(),
            hajm_mathbut: hajm,
            isdar_mathbut: sijill
                .khariji(matlub)
                .map_or_else(String::new, |madkhal| madkhal.isdar.clone()),
            sha256: maqbula,
            hajm,
            waqt: Timestamp::now().to_string(),
        },
    );
    muraqaba.ihfadh(&masar_muraqaba)?;

    tracing::info!(
        ruqaa = %matlub,
        qitaa = %qitaa,
        hajm,
        "the owner re-pinned a third-party artifact against bytes the endpoint served"
    );
    Ok(true)
}

// ---------------------------------------------------------------------------
// The catalogue
// ---------------------------------------------------------------------------

/// Every third-party entry this client knows about for one game.
///
/// The verified shard first — `ShareehaMuwaththaqa` has no constructor but the
/// one that hashes the bytes against the signed manifest, so nothing unverified
/// can be listed — then the compiled-in seed for any lineage the shard does not
/// carry. The shard always wins: a published entry is the owner's current word
/// about it and the seed is what this build shipped with.
async fn katalog_luba(hali: &Idadat, id: LubaId, makhbaa: &Path) -> Natija<Vec<RuqaaKharijiya>> {
    let silsila = silsilat_masadir(hali)?;
    let fahras = jalb_fahras(&silsila, &[id], makhbaa, None).await?;
    let mut madakhil: Vec<RuqaaKharijiya> = fahras
        .shareeha_luba(id)
        .map(|shareeha| shareeha.kharijiya(id).to_vec())
        .unwrap_or_default();

    for badhra in badhrat_kharijiya() {
        if badhra.luba == id && !madakhil.iter().any(|madkhal| madkhal.id == badhra.id) {
            madakhil.push(badhra);
        }
    }
    Ok(madakhil)
}

/// One entry out of a catalogue, or the refusal that names what was asked for.
fn madkhal_bi_huwiya(
    madakhil: Vec<RuqaaKharijiya>,
    matlub: RuqaaId,
    ism_luba: &str,
) -> Natija<RuqaaKharijiya> {
    madakhil
        .into_iter()
        .find(|madkhal| madkhal.id == matlub)
        .ok_or_else(|| {
            Khata::from(KhataKharijiAmr::MadkhalGhayrMawjud {
                ruqaa: matlub.to_string(),
                ism: ism_luba.to_owned(),
            })
        })
}

/// One catalogue entry, projected for the game screen.
fn hie_madkhal(
    madkhal: RuqaaKharijiya,
    bina_mutabaqa: Option<String>,
    muthabbata: bool,
) -> MudkhalKharijiHie {
    MudkhalKharijiHie {
        id: madkhal.id,
        unwan: madkhal.unwan,
        luba: madkhal.luba,
        hawiyat_manassa: madkhal.hawiyat_manassa,
        abniya: madkhal.abniya,
        masdar: madkhal.masdar,
        fariq: madkhal.fariq,
        isdar: madkhal.isdar,
        qitaa: madkhal.qitaa,
        takhtit: madkhal.takhtit,
        tahdheerat: madkhal.tahdheerat,
        mira: HalatMiraHie::min_asli(&madkhal.mira),
        bina_mutabaqa,
        muthabbata,
    }
}

/// The installed build the gate is handed, or the refusal that stands in for it.
///
/// An entry that pins different files to different builds cannot be installed
/// into a game whose build nobody has measured: `qitaa_li_bina` would select
/// nothing and the gate would refuse with a build identifier the reader has
/// never seen. So the case is named here instead, before anything is fetched.
///
/// An entry that draws no build distinction at all is unaffected — every
/// artifact applies to every build and the value is never compared against
/// anything — so a game with no measured build still installs one of those.
fn bina_lil_fahs(
    maqisa: Option<String>,
    ism_luba: &str,
    madkhal: &RuqaaKharijiya,
) -> Natija<String> {
    if let Some(bina) = maqisa {
        return Ok(bina);
    }
    let yufarriq =
        !madkhal.abniya.is_empty() || madkhal.qitaa.iter().any(|qitaa| !qitaa.abniya.is_empty());
    if yufarriq {
        return Err(Khata::from(KhataKharijiAmr::BinaMajhula {
            ism: ism_luba.to_owned(),
            unwan: madkhal.unwan.clone(),
        }));
    }
    Ok(String::new())
}

/// The game executable's own name, which the running-game guard needs.
fn ism_tanfidhi(luba: &Luba) -> Natija<String> {
    luba.tanfidhi
        .as_deref()
        .and_then(Path::file_name)
        .and_then(|ism| ism.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            Khata::from(KhataTathbeetAmr::TanfidhiMajhul {
                ism: luba.ism.clone(),
            })
        })
}

/// A count the interface can hold, saturating rather than wrapping.
fn adad(qeema: usize) -> u32 {
    u32::try_from(qeema).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------------------
// The collision report
// ---------------------------------------------------------------------------

/// What is in the way, in the order the install itself asks.
fn hie_tadakhul(
    jidhr_luba: &Path,
    jidhr_nusakh: &Path,
    takhtit: &TakhtitKhariji,
) -> TadakhulKharijiHie {
    let athar = masah_tasadum(jidhr_luba, jidhr_nusakh);
    let Some(awwal) = athar.first() else {
        return TadakhulKharijiHie {
            mutadakhil: false,
            sahib_arabi: String::new(),
            sahib_injilizi: String::new(),
            manfadh: String::new(),
            yuzal: Vec::new(),
            taarib_yuzil: false,
        };
    };

    let yuzal = masarat_izala(awwal);
    TadakhulKharijiHie {
        mutadakhil: true,
        sahib_arabi: awwal.jiha.wasf_arabi().to_owned(),
        sahib_injilizi: awwal.jiha.wasf_injilizi().to_owned(),
        manfadh: manfadh(awwal.jiha, jidhr_luba, takhtit, &yuzal),
        yuzal,
        // Taarib's own install comes off with Taarib's own removal, which
        // restores every byte it replaced. Another team's does not: this product
        // did not write those files and does not delete them for somebody.
        taarib_yuzil: awwal.jiha == JihatTasadum::Taarib,
    }
}

/// The paths a reader can act on, out of everything the sweep found.
///
/// The sweep also reports the manifests it read, annotated, because those are
/// what proved the case. They are not paths anybody should delete by hand, and a
/// list headed "what has to go first" must not name one.
fn masarat_izala(athar: &AtharTasadum) -> Vec<String> {
    athar
        .alamat
        .iter()
        .filter(|alama| !alama.contains(" ("))
        .cloned()
        .collect()
}

/// The loader slot the two are fighting over.
///
/// Read off the game directory rather than assumed: the survey in
/// `taarib_tathbeet::wukala` identifies which product holds each proxy slot, so
/// Taarib's own is named by the evidence that it is Taarib's, and the other
/// side's is named as whichever occupied slot the entry about to be installed
/// also declares it writes — which is exactly the collision.
fn manfadh(
    jiha: JihatTasadum,
    jidhr_luba: &Path,
    takhtit: &TakhtitKhariji,
    yuzal: &[String],
) -> String {
    let wukala = masah(jidhr_luba).unwrap_or_default();
    let maakhudh = match jiha {
        JihatTasadum::Taarib => wukala
            .iter()
            .find(|wakeel| wakeel.huwiya.huwa_taarib())
            .map(|wakeel| wakeel.ism.clone()),
        JihatTasadum::Khariji => slot_mutanaza(&wukala, takhtit).or_else(|| {
            wukala
                .iter()
                .find(|wakeel| !wakeel.huwiya.huwa_taarib())
                .map(|wakeel| wakeel.ism.clone())
        }),
    };
    // With no module identified, the honest answer is the first thing the sweep
    // actually found rather than a slot name nobody read off this disk.
    maakhudh.unwrap_or_else(|| yuzal.first().cloned().unwrap_or_default())
}

/// The occupied slot the entry being installed wants for itself, if any.
fn slot_mutanaza(wukala: &[WakeelQaim], takhtit: &TakhtitKhariji) -> Option<String> {
    takhtit
        .yaktub
        .iter()
        .filter_map(|masar| wakeel_nizam(masar))
        .find_map(|slot| shaghil_slot(wukala, slot).map(|wakeel| wakeel.ism.clone()))
}

// ---------------------------------------------------------------------------
// The transport
// ---------------------------------------------------------------------------

/// One artifact the install is waiting for.
struct TalabJalb {
    /// The artifact's name, for the refusal that names it.
    ism: String,
    /// Where to fetch it from.
    rabt: String,
    /// The staged file the body is written to.
    hadaf: PathBuf,
    /// Where the answer goes.
    radd: tokio::sync::oneshot::Sender<NatijatTathbeet<()>>,
}

/// The transport the installer is handed: a request, and a wait for the answer.
///
/// The installer is synchronous and the client is asynchronous, and neither is
/// allowed to change for the other's convenience — a runtime built inside a
/// blocking thread is a second runtime in a process that already has one. So the
/// two talk: this side blocks on a channel, and the command's own task performs
/// the transfer and answers. It carries no policy at all, which is the point.
struct NaqilQanat {
    mursil: tokio::sync::mpsc::Sender<TalabJalb>,
}

impl NaqilKhariji for NaqilQanat {
    fn ijlib(&mut self, ism: &str, rabt: &str, hadaf: &Path) -> NatijatTathbeet<()> {
        let (radd, jawab) = tokio::sync::oneshot::channel();
        self.mursil
            .blocking_send(TalabJalb {
                ism: ism.to_owned(),
                rabt: rabt.to_owned(),
                hadaf: hadaf.to_path_buf(),
                radd,
            })
            .map_err(|_| khata_naql(ism, rabt, "the transfer task is no longer listening"))?;
        jawab
            .blocking_recv()
            .map_err(|_| khata_naql(ism, rabt, "the transfer task ended without answering"))?
    }
}

/// One transfer that did not happen, in the installer's own vocabulary.
fn khata_naql(ism: &str, rabt: &str, sabab: &str) -> KhataTathbeet {
    KhataTathbeet::TanzeelMutaadhdhir {
        ism: ism.to_owned(),
        rabt: rabt.to_owned(),
        sabab: sabab.to_owned(),
    }
}

/// The client every third-party fetch goes through.
///
/// The same shape as the package downloader's and the update channel's: this
/// product's user agent, this product's connection timeout, a bounded redirect
/// chain, https only and no referer. No whole-request timeout, because one would
/// abort a large archive on a slow line; a read timeout instead, which catches
/// the thing that actually goes wrong.
///
/// # Errors
///
/// [`KhataKharijiAmr::AmilMutaadhdhir`] when the TLS backend cannot be built.
fn amil_khariji() -> Natija<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(format!(
            "Taarib/{ISDAR} (+https://github.com/cc1a2b/taarib)"
        ))
        .connect_timeout(MUHLAT_ITTISAL)
        .read_timeout(MUHLAT_QITA)
        .redirect(reqwest::redirect::Policy::limited(HADD_TAHWIL))
        .https_only(true)
        .referer(false)
        .build()
        .map_err(|sabab| {
            Khata::from(KhataKharijiAmr::AmilMutaadhdhir {
                sabab: sabab.to_string(),
            })
        })
}

/// Fetches one address into one file, and writes nothing else anywhere.
///
/// The body is capped at [`AQSA_HAJM_QITAA`], the largest artifact this build
/// will fetch at all, so a server that streams forever fills a quarantine file
/// to a known ceiling and is then refused rather than filling the disk. The
/// size and the digest this is checked against are `jalb_khariji`'s, computed
/// over the file left behind — nothing here is allowed to conclude that an
/// artifact matched.
async fn jalb_ila_malaf(
    amil: &reqwest::Client,
    ism: &str,
    rabt: &str,
    hadaf: &Path,
) -> NatijatTathbeet<()> {
    use tokio::io::AsyncWriteExt as _;

    let fashil = |sabab: String| khata_naql(ism, rabt, &sabab);

    let mut istijaba = amil
        .get(rabt)
        .send()
        .await
        .map_err(|sabab| fashil(sabab.to_string()))?;
    if !istijaba.status().is_success() {
        return Err(fashil(format!(
            "the endpoint answered {}",
            istijaba.status()
        )));
    }

    let mut malaf = tokio::fs::File::create(hadaf)
        .await
        .map_err(|sabab| fashil(format!("{} could not be opened: {sabab}", hadaf.display())))?;
    let mut manqul = 0_u64;
    while let Some(qita) = istijaba
        .chunk()
        .await
        .map_err(|sabab| fashil(sabab.to_string()))?
    {
        manqul = manqul.saturating_add(qita.len() as u64);
        if manqul > AQSA_HAJM_QITAA {
            return Err(fashil(format!(
                "the body passed the {AQSA_HAJM_QITAA} byte(s) this build will fetch at all"
            )));
        }
        malaf.write_all(&qita).await.map_err(|sabab| {
            fashil(format!("{} could not be written: {sabab}", hadaf.display()))
        })?;
    }
    malaf
        .flush()
        .await
        .map_err(|sabab| fashil(format!("{} could not be flushed: {sabab}", hadaf.display())))?;
    Ok(())
}

/// What one address is serving right now: its digest and its length.
///
/// Nothing is written. The observation exists to be read off a screen and
/// compared against a pin, and a file on disk would be a copy of somebody
/// else's archive kept for no reason.
///
/// # Errors
///
/// [`KhataKharijiAmr::RasdMutaadhdhir`] naming the address and what the
/// transport reported, including a body past [`AQSA_HAJM_QITAA`].
async fn arsud(amil: &reqwest::Client, ism: &str, rabt: &str) -> Natija<(String, u64)> {
    let fashil = |sabab: String| {
        Khata::from(KhataKharijiAmr::RasdMutaadhdhir {
            qitaa: ism.to_owned(),
            rabt: rabt.to_owned(),
            sabab,
        })
    };

    let mut istijaba = amil
        .get(rabt)
        .send()
        .await
        .map_err(|sabab| fashil(sabab.to_string()))?;
    if !istijaba.status().is_success() {
        return Err(fashil(format!(
            "the endpoint answered {}",
            istijaba.status()
        )));
    }

    let mut hashib = Sha256::new();
    let mut hajm = 0_u64;
    while let Some(qita) = istijaba
        .chunk()
        .await
        .map_err(|sabab| fashil(sabab.to_string()))?
    {
        hajm = hajm.saturating_add(qita.len() as u64);
        if hajm > AQSA_HAJM_QITAA {
            return Err(fashil(format!(
                "the body passed the {AQSA_HAJM_QITAA} byte(s) this build will read at all"
            )));
        }
        hashib.update(&qita);
    }
    Ok((hex::encode(hashib.finalize()), hajm))
}

// ---------------------------------------------------------------------------
// The watch's own record
// ---------------------------------------------------------------------------

/// One artifact as the author's endpoint last served it, and the pin it was
/// compared against.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RasdQitaa {
    /// The pinned digest at the moment of the observation, lowercase hex.
    sha256_mathbut: String,
    /// The pinned size at that moment.
    hajm_mathbut: u64,
    /// The release the pin belonged to.
    isdar_mathbut: String,
    /// What the endpoint served, lowercase hex.
    sha256: String,
    /// How long it was.
    hajm: u64,
    /// When, RFC 3339.
    waqt: String,
}

impl RasdQitaa {
    /// Whether this observation still describes the pin that is standing now.
    fn yuqaran(&self, qitaa: &QitaatTanzeel) -> bool {
        self.hajm_mathbut == qitaa.hajm && self.sha256_mathbut.eq_ignore_ascii_case(&qitaa.sha256)
    }

    /// Whether what was served is what was pinned.
    fn mutabiq(&self) -> bool {
        self.hajm == self.hajm_mathbut && self.sha256.eq_ignore_ascii_case(&self.sha256_mathbut)
    }
}

/// Everything the watch has observed, keyed by entry and artifact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SijillMuraqaba {
    #[serde(default)]
    marsud: BTreeMap<String, RasdQitaa>,
}

impl SijillMuraqaba {
    /// Reads the record, treating an absent file as an empty one.
    ///
    /// A file that exists and will not parse is a named failure rather than a
    /// fresh empty record: an empty one says every artifact matches its pin,
    /// which is the one thing this must never claim without having looked.
    fn iftah(masar: &Path) -> Natija<Self> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            },
            Err(sabab) => {
                return Err(Khata::from(KhataKharijiAmr::SijillMuraqabaTalif {
                    masar: masar.to_path_buf(),
                    sabab: sabab.to_string(),
                }));
            },
        };
        serde_json::from_slice(&bayt).map_err(|sabab| {
            Khata::from(KhataKharijiAmr::SijillMuraqabaTalif {
                masar: masar.to_path_buf(),
                sabab: sabab.to_string(),
            })
        })
    }

    /// Writes the record atomically.
    fn ihfadh(&self, masar: &Path) -> Natija<()> {
        let bayt = serde_json::to_vec_pretty(self).map_err(|sabab| {
            Khata::from(KhataKharijiAmr::SijillMuraqabaTalif {
                masar: masar.to_path_buf(),
                sabab: sabab.to_string(),
            })
        })?;
        masarat::kitaba_dharra(masar, &bayt)
    }

    fn rasd(&self, miftah: &str) -> Option<&RasdQitaa> {
        self.marsud.get(miftah)
    }

    fn sajjil(&mut self, miftah: String, rasd: RasdQitaa) {
        let _ = self.marsud.insert(miftah, rasd);
    }

    /// Whether one artifact was already read inside the refresh window, against
    /// exactly the pin that is standing now.
    fn hadith(
        &self,
        miftah: &str,
        qitaa: &QitaatTanzeel,
        nafidha: jiff::SignedDuration,
        alaan: Timestamp,
    ) -> bool {
        let Some(rasd) = self.marsud.get(miftah) else {
            return false;
        };
        if !rasd.yuqaran(qitaa) {
            return false;
        }
        rasd.waqt
            .parse::<Timestamp>()
            .is_ok_and(|marsud| alaan.duration_since(marsud) < nafidha)
    }
}

/// One artifact's identity in the watch's record.
fn miftah_rasd(ruqaa: RuqaaId, qitaa: &str) -> String {
    format!("{ruqaa}:{qitaa}")
}

/// Where the watch's record lives, under the registry cache.
///
/// # Errors
///
/// Whatever [`masarat::dakhil`] refuses, which is nothing this build can send.
fn masar_muraqaba(masarat: &Masarat) -> Natija<PathBuf> {
    masarat::dakhil(&masarat.makhbaa(), MASAR_MURAQABA)
}

/// One pending re-pin, projected for the console.
///
/// `isdar` is the release the ledger declares the author ships, and
/// `isdar_mathbut` is the release the standing pin belongs to. They agree
/// wherever a release was listed and pinned together, which is the ordinary
/// case; they come apart when the owner lists a new release and leaves its
/// artifacts pinned to the old one, which is what the console's two version
/// columns are there to show.
///
/// The ledger is the only statement anywhere of what the author ships. A
/// release the owner has not listed does not exist as far as this console is
/// concerned, and this row never guesses one out of an author's own feed.
fn hie_basma(madkhal: &RuqaaKharijiya, qitaa: &QitaatTanzeel, rasd: &RasdQitaa) -> BasmaMualaqaHie {
    BasmaMualaqaHie {
        ruqaa: madkhal.id,
        unwan: madkhal.unwan.clone(),
        muallif: madkhal.masdar.ism.clone(),
        fariq: madkhal.fariq.clone(),
        rabt_muallif: madkhal.masdar.rabt.clone(),
        isdar: madkhal.isdar.clone(),
        isdar_mathbut: rasd.isdar_mathbut.clone(),
        qitaa: qitaa.ism.clone(),
        rabt: qitaa.rabt.clone(),
        hajm: rasd.hajm,
        hajm_mathbut: rasd.hajm_mathbut,
        sha256_mathbut: rasd.sha256_mathbut.clone(),
        sha256_marsud: rasd.sha256.clone(),
        waqt: rasd.waqt.clone(),
    }
}

// ---------------------------------------------------------------------------
// The author addresses the browser command may open
// ---------------------------------------------------------------------------

/// Author addresses named by a third-party entry this process has listed.
///
/// Held in memory rather than on disk, and that is the whole of its security
/// story: a host is in here only because a catalogue this process read named it
/// as somebody's author page, and the catalogues it can be read from are the
/// verified shard — which cannot be constructed from bytes the signed manifest
/// did not vouch for — the owner's own publication ledger, and the entries
/// compiled into this binary. Nothing writes it from a file and nothing survives
/// the process, so there is no document anybody can edit to teach
/// `mujtama_awamir::iftah_rabt` a new host.
static MUDIFU_MUALLIFIN: LazyLock<RwLock<BTreeSet<String>>> =
    LazyLock::new(|| RwLock::new(BTreeSet::new()));

/// Records the author addresses of every entry a catalogue read just produced.
fn sajjil_mudifin(madakhil: &[RuqaaKharijiya]) {
    let jadeed: BTreeSet<String> = madakhil
        .iter()
        .filter_map(|madkhal| mudif_muallif(&madkhal.masdar))
        .collect();
    if jadeed.is_empty() {
        return;
    }
    MUDIFU_MUALLIFIN.write().extend(jadeed);
}

/// Every host a third-party entry's author page may live on.
///
/// The compiled-in seed catalogue unconditionally — those addresses are in this
/// binary and are exactly as trustworthy as the platform list `iftah_rabt`
/// already carries — plus whatever a verified catalogue named this session.
///
/// Exact hosts, never subdomains: the set is consulted by membership, so a host
/// is here because an entry named it and for no other reason. This is what keeps
/// the allowance from widening into "open any address", which is what the
/// command would become if the third-party panels had simply been exempted.
#[must_use]
pub fn mudifu_muallifin() -> BTreeSet<String> {
    let mut mudifun: BTreeSet<String> = badhrat_kharijiya()
        .iter()
        .filter_map(|madkhal| mudif_muallif(&madkhal.masdar))
        .collect();
    mudifun.extend(MUDIFU_MUALLIFIN.read().iter().cloned());
    mudifun
}

/// The host an author's page lives on, when the address is one that could be
/// opened at all.
fn mudif_muallif(masdar: &MasdarKhariji) -> Option<String> {
    let rabt = reqwest::Url::parse(masdar.rabt.trim()).ok()?;
    if rabt.scheme() != "https" {
        return None;
    }
    rabt.host_str().map(str::to_ascii_lowercase)
}

// ---------------------------------------------------------------------------
// Failures
// ---------------------------------------------------------------------------

/// One gate refusal, as the interface's error type.
///
/// Every arm of [`RafdKhariji`] is answered rather than collapsed, for the
/// reason the signed path's own mapping gives: a code is the only thing on the
/// wire a screen can branch on, and these refusals have different remedies
/// between them. The match is exhaustive on purpose — a refusal added to the
/// gate stops this build here rather than falling into a bucket whose sentence
/// would send the reader somewhere that is not the problem.
///
/// None of it lifts anything. Anti-cheat still ends at [`Khutwa::LaShay`], and
/// the permission refusal has no override anywhere in this product.
fn khata_rafd(rafd: &RafdKhariji, ism: &str) -> Khata {
    let khutwa = match rafd {
        // The one refusal a setting answers: VAC is declared in Steam's
        // catalogue alone, so a scan that could not find Steam is a scan the
        // manual launcher override lets run.
        RafdKhariji::FahsMatjarLamYajri { .. } => Khutwa::FathIdadat {
            qism: taarib_usus::khata::QismIdadat::Manassat,
        },
        // An unacknowledged statement is answered by the person, and an action
        // offering to answer it would be answering it for them — the same
        // reason the signed path's own `IqrarNaqis` offers nothing. The rest —
        // anti-cheat, a permission nobody recorded, a mirror nobody permitted,
        // a build the entry does not claim — no button fixes at all.
        RafdKhariji::IqrarNaqis
        | RafdKhariji::Himaya(_)
        | RafdKhariji::IdhnMasdarNaqis { .. }
        | RafdKhariji::MiraGhayrMasmuha { .. }
        | RafdKhariji::BinaGhayrMadumma { .. }
        | RafdKhariji::TahdheeratGhayrMuqarra { .. } => Khutwa::LaShay,
    };
    Khata::from(KhataKharijiAmr::BawwabaRafadat {
        ism: ism.to_owned(),
        arabi: rafd.arabi(),
        injilizi: rafd.injilizi(),
        khutwa,
    })
}

/// Failures of the third-party patch surface itself.
#[derive(Debug, thiserror::Error)]
pub enum KhataKharijiAmr {
    /// The catalogue lists no such third-party entry.
    #[error("no third-party entry {ruqaa} is listed for {ism}")]
    MadkhalGhayrMawjud {
        /// The lineage that was asked for.
        ruqaa: String,
        /// Where it was looked for.
        ism: String,
    },

    /// The entry carries no artifact of that name.
    #[error("{ruqaa} pins no artifact called {qitaa}")]
    QitaaGhayrMawjuda {
        /// The entry.
        ruqaa: String,
        /// The artifact that was asked for.
        qitaa: String,
    },

    /// The installed build could not be measured and the entry needs one.
    #[error("the installed build of {ism} is unknown and {unwan} pins files per build")]
    BinaMajhula {
        /// The game.
        ism: String,
        /// The entry.
        unwan: String,
    },

    /// The pre-install gate refused, in its own words.
    #[error("{injilizi}")]
    BawwabaRafadat {
        /// The game.
        ism: String,
        /// The gate's sentence, in Arabic.
        arabi: String,
        /// The same, in English.
        injilizi: String,
        /// What the reader can do, when anything.
        khutwa: Khutwa,
    },

    /// The author's endpoint could not be read.
    #[error("{rabt} could not be read: {sabab}")]
    RasdMutaadhdhir {
        /// The artifact, when the caller named one.
        qitaa: String,
        /// The address.
        rabt: String,
        /// What the transport reported.
        sabab: String,
    },

    /// The hash the owner accepted is not what the endpoint is serving.
    #[error("{qitaa} now hashes to {marsuda} and the acceptance carried {maqbula}")]
    BasmaGhayrMutabaqa {
        /// The artifact.
        qitaa: String,
        /// Where it was read from.
        rabt: String,
        /// What the owner accepted.
        maqbula: String,
        /// What actually arrived.
        marsuda: String,
    },

    /// What was sent as a hash is not one.
    #[error("{basma} is not a sha256")]
    BasmaGhayrSaliha {
        /// The artifact.
        qitaa: String,
        /// The text that was sent.
        basma: String,
    },

    /// The watch's own record exists and cannot be read or written.
    #[error("{} could not be used: {sabab}", masar.display())]
    SijillMuraqabaTalif {
        /// Where it lives.
        masar: PathBuf,
        /// Why.
        sabab: String,
    },

    /// The HTTP client could not be built.
    #[error("the network client could not be built: {sabab}")]
    AmilMutaadhdhir {
        /// What the TLS backend reported.
        sabab: String,
    },
}

impl Tafsir for KhataKharijiAmr {
    fn ramz(&self) -> Ramz {
        Ramz::jadeed(
            arqam::STUDIO
                + match self {
                    Self::MadkhalGhayrMawjud { .. } => 160,
                    Self::QitaaGhayrMawjuda { .. } => 161,
                    Self::BinaMajhula { .. } => 162,
                    Self::BawwabaRafadat { .. } => 163,
                    Self::RasdMutaadhdhir { .. } => 164,
                    Self::BasmaGhayrMutabaqa { .. } => 165,
                    Self::BasmaGhayrSaliha { .. } => 166,
                    Self::SijillMuraqabaTalif { .. } => 167,
                    Self::AmilMutaadhdhir { .. } => 168,
                },
        )
    }

    fn khutura(&self) -> Khutura {
        match self {
            // Nothing was fetched and nothing was written in any of these.
            Self::MadkhalGhayrMawjud { .. }
            | Self::QitaaGhayrMawjuda { .. }
            | Self::BinaMajhula { .. }
            | Self::RasdMutaadhdhir { .. }
            | Self::BasmaGhayrSaliha { .. }
            | Self::SijillMuraqabaTalif { .. }
            | Self::AmilMutaadhdhir { .. } => Khutura::Tanbeeh,
            // A gate refusal and a hash that moved under an acceptance are both
            // statements about safety, and neither is a notice.
            Self::BawwabaRafadat { .. } | Self::BasmaGhayrMutabaqa { .. } => Khutura::Khatar,
        }
    }

    fn arabi(&self) -> String {
        match self {
            Self::MadkhalGhayrMawjud { ism, .. } => {
                format!("لم يعد سجلّ تعريب يعرض هذه الرقعة الخارجية لـ«{ism}». أعد تحميل الصفحة.")
            },
            Self::QitaaGhayrMawjuda { qitaa, .. } => {
                format!("لا يحمل هذا المدخل ملفًّا باسم {qitaa}؛ لم يتغيّر شيء.")
            },
            Self::BinaMajhula { ism, unwan } => format!(
                "«{unwan}» يربط ملفّات مختلفة بأبنية مختلفة، وبناء «{ism}» المثبَّت لم يُقس على \
                 هذا الجهاز. لا يُجلب شيء ولا يُكتب شيء قبل معرفة البناء."
            ),
            Self::BawwabaRafadat { arabi, .. } => arabi.clone(),
            Self::RasdMutaadhdhir { rabt, sabab, .. } => {
                if rabt.is_empty() {
                    format!("تعذّرت قراءة ملفّ المؤلّف: {sabab}")
                } else {
                    format!("تعذّرت قراءة {rabt}: {sabab}. تبقى البصمة المثبَّتة كما هي.")
                }
            },
            Self::BasmaGhayrMutabaqa {
                qitaa,
                maqbula,
                marsuda,
                ..
            } => format!(
                "ما يخدمه المؤلّف الآن لـ{qitaa} بصمته {marsuda}، والبصمة التي قبِلتها {maqbula}. \
                 لم تُثبَّت بصمة على بايتات لم تُقرأ: أعد قراءة البصمة الواردة واقبلها من جديد."
            ),
            Self::BasmaGhayrSaliha { qitaa, .. } => {
                format!("ما أُرسل لـ{qitaa} ليس بصمة sha256 من أربعة وستّين رمزًا ستّ عشريًّا.")
            },
            Self::SijillMuraqabaTalif { sabab, .. } => {
                format!("تعذّر استعمال سجلّ مراقبة البصمات: {sabab}")
            },
            Self::AmilMutaadhdhir { sabab, .. } => {
                format!("تعذّر تجهيز عميل الشبكة في هذه النسخة: {sabab}")
            },
        }
    }

    fn injilizi(&self) -> String {
        match self {
            Self::MadkhalGhayrMawjud { ism, .. } => {
                format!(
                    "Taarib's registry no longer lists this third-party patch for {ism}. Reload the page."
                )
            },
            Self::QitaaGhayrMawjuda { qitaa, .. } => {
                format!("This entry carries no artifact called {qitaa}; nothing was changed.")
            },
            Self::BinaMajhula { ism, unwan } => format!(
                "{unwan} pins different files to different game builds, and the installed build of \
                 {ism} has not been measured on this machine. Nothing is fetched and nothing is \
                 written until the build is known."
            ),
            Self::BawwabaRafadat { injilizi, .. } => injilizi.clone(),
            Self::RasdMutaadhdhir { rabt, sabab, .. } => {
                if rabt.is_empty() {
                    format!("The author's file could not be read: {sabab}")
                } else {
                    format!("{rabt} could not be read: {sabab}. The pinned hash stands unchanged.")
                }
            },
            Self::BasmaGhayrMutabaqa {
                qitaa,
                maqbula,
                marsuda,
                ..
            } => format!(
                "{qitaa} now hashes to {marsuda} and the acceptance carried {maqbula}. No pin is \
                 moved onto bytes nobody read: read the incoming hash again and accept it afresh."
            ),
            Self::BasmaGhayrSaliha { qitaa, .. } => {
                format!("What was sent for {qitaa} is not a 64-character hex sha256.")
            },
            Self::SijillMuraqabaTalif { sabab, .. } => {
                format!("The hash watch's own record could not be used: {sabab}")
            },
            Self::AmilMutaadhdhir { sabab, .. } => {
                format!("This build could not construct its network client: {sabab}")
            },
        }
    }

    fn khutwa(&self) -> Khutwa {
        match self {
            // The gate wrote its own way out, or said there is none.
            Self::BawwabaRafadat { khutwa, .. } => khutwa.clone(),
            // A catalogue that moved under the screen, an endpoint that did not
            // answer, a client that would not build: asking again is the answer.
            Self::MadkhalGhayrMawjud { .. }
            | Self::RasdMutaadhdhir { .. }
            | Self::AmilMutaadhdhir { .. } => Khutwa::AadaMuhawala,
            Self::SijillMuraqabaTalif { .. } => Khutwa::FathTashkhis,
            // A hash that moved under an acceptance is re-read off the console
            // and accepted again; the other three are statements about what was
            // asked for, and no action changes any of them.
            Self::BasmaGhayrMutabaqa { .. }
            | Self::QitaaGhayrMawjuda { .. }
            | Self::BinaMajhula { .. }
            | Self::BasmaGhayrSaliha { .. } => Khutwa::LaShay,
        }
    }

    fn siyaq(&self) -> BTreeMap<String, QeemaSiyaq> {
        let mut siyaq = BTreeMap::new();
        match self {
            Self::MadkhalGhayrMawjud { ruqaa, ism } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::QitaaGhayrMawjuda { ruqaa, qitaa } => {
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(ruqaa.clone()));
                let _ = siyaq.insert("qitaa".to_owned(), QeemaSiyaq::Nass(qitaa.clone()));
            },
            Self::BinaMajhula { ism, unwan } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
                let _ = siyaq.insert("ruqaa".to_owned(), QeemaSiyaq::Nass(unwan.clone()));
            },
            Self::BawwabaRafadat { ism, .. } => {
                let _ = siyaq.insert("luba".to_owned(), QeemaSiyaq::Nass(ism.clone()));
            },
            Self::RasdMutaadhdhir { qitaa, rabt, sabab } => {
                let _ = siyaq.insert("qitaa".to_owned(), QeemaSiyaq::Nass(qitaa.clone()));
                let _ = siyaq.insert("rabt".to_owned(), QeemaSiyaq::Nass(rabt.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::BasmaGhayrMutabaqa {
                qitaa,
                rabt,
                maqbula,
                marsuda,
            } => {
                let _ = siyaq.insert("qitaa".to_owned(), QeemaSiyaq::Nass(qitaa.clone()));
                let _ = siyaq.insert("rabt".to_owned(), QeemaSiyaq::Nass(rabt.clone()));
                let _ = siyaq.insert("maqbula".to_owned(), QeemaSiyaq::Nass(maqbula.clone()));
                let _ = siyaq.insert("marsuda".to_owned(), QeemaSiyaq::Nass(marsuda.clone()));
            },
            Self::BasmaGhayrSaliha { qitaa, basma } => {
                let _ = siyaq.insert("qitaa".to_owned(), QeemaSiyaq::Nass(qitaa.clone()));
                let _ = siyaq.insert("basma".to_owned(), QeemaSiyaq::Nass(basma.clone()));
            },
            Self::SijillMuraqabaTalif { masar, sabab } => {
                let _ = siyaq.insert("masar".to_owned(), QeemaSiyaq::Masar(masar.clone()));
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
            Self::AmilMutaadhdhir { sabab } => {
                let _ = siyaq.insert("sabab".to_owned(), QeemaSiyaq::Nass(sabab.clone()));
            },
        }
        siyaq
    }
}

khata_min!(KhataKharijiAmr);

#[cfg(test)]
mod ikhtibarat {
    use std::path::Path;

    use taarib_mustalahat::khariji::{HalatMira, QitaatTanzeel, TakhtitKhariji};
    use taarib_mustawda::khariji::badhrat_rtea;

    use super::{
        HalatMiraHie, RasdQitaa, SijillMuraqaba, bina_lil_fahs, masarat_izala, miftah_rasd,
        mudif_muallif, mudifu_muallifin, slot_mutanaza,
    };
    use taarib_tathbeet::tasadum::{AtharTasadum, JihatTasadum};

    fn qitaa() -> QitaatTanzeel {
        QitaatTanzeel {
            ism: "update.zip".to_owned(),
            rabt: "https://rt.example/update.zip".to_owned(),
            hajm: 10,
            sha256: "ffd54019f146b8db8a02d6413c592222ae9de397c522631d3b65fe882b3e1343".to_owned(),
            abniya: Vec::new(),
        }
    }

    /// The one field whose Rust spelling and wire spelling differ: the shared
    /// type is tagged internally so the published shards keep their bytes, and
    /// the catalogue panel decodes the ordinary external tagging.
    #[test]
    fn halat_almira_tuktab_bi_wasm_khariji() -> Result<(), serde_json::Error> {
        assert_eq!(
            serde_json::to_string(&HalatMiraHie::min_asli(&HalatMira::MinAlmuallif))?,
            r#""min_almuallif""#
        );
        assert_eq!(
            serde_json::to_string(&HalatMiraHie::min_asli(&HalatMira::MinAlsijill {
                rabt: "https://mira.example/rtea".to_owned(),
            }))?,
            r#"{"min_alsijill":{"rabt":"https://mira.example/rtea"}}"#
        );
        Ok(())
    }

    /// An entry that pins files per build is not installed into a game whose
    /// build nobody measured; one that draws no distinction is unaffected.
    #[test]
    fn albina_almajhula_turfad_lilmudkhal_alladhi_yufarriq() {
        let ism = "Red Dead Redemption 2";
        let mufarriq = badhrat_rtea();
        assert!(bina_lil_fahs(None, ism, &mufarriq).is_err());
        assert_eq!(
            bina_lil_fahs(Some("1491".to_owned()), ism, &mufarriq).ok(),
            Some("1491".to_owned())
        );

        let mut aam = badhrat_rtea();
        aam.abniya.clear();
        for wahid in &mut aam.qitaa {
            wahid.abniya.clear();
        }
        assert_eq!(bina_lil_fahs(None, ism, &aam).ok(), Some(String::new()));
    }

    /// The list headed "what has to go first" names paths a reader can act on,
    /// never the manifests that proved the case.
    #[test]
    fn masarat_alizala_tastabidu_albayanat() {
        let athar = AtharTasadum {
            jiha: JihatTasadum::Khariji,
            alamat: vec![
                "/home/x/nusakh/khariji/abc (a third-party install Taarib recorded)".to_owned(),
                "lml".to_owned(),
                "dinput8.dll".to_owned(),
            ],
        };
        assert_eq!(masarat_izala(&athar), vec!["lml", "dinput8.dll"]);
    }

    /// Nothing in the game directory means nothing to contest, so the slot is
    /// read off the survey and not guessed from the entry's own wish list.
    #[test]
    fn slot_mutanaza_yaqra_almashghul_faqat() {
        let takhtit = TakhtitKhariji {
            yaktub: vec!["dinput8.dll".to_owned(), "lml".to_owned()],
            yahdhif: Vec::new(),
        };
        assert_eq!(slot_mutanaza(&[], &takhtit), None);
    }

    /// The seed's author address is compiled in, so the browser control on its
    /// card works on a machine that has never reached the registry.
    #[test]
    fn mudif_albadhra_masmuh() {
        let mudifun = mudifu_muallifin();
        assert!(mudifun.contains("github.com"));
        assert_eq!(
            mudif_muallif(&badhrat_rtea().masdar).as_deref(),
            Some("github.com")
        );
    }

    /// An observation is reused only while it still describes the pin that is
    /// standing, so a re-pin is never compared against an older reading.
    #[test]
    fn alrasd_yasqut_ind_taghyeer_albasma() {
        let wahid = qitaa();
        let miftah = miftah_rasd(badhrat_rtea().id, &wahid.ism);
        let mut sijill = SijillMuraqaba::default();
        sijill.sajjil(
            miftah.clone(),
            RasdQitaa {
                sha256_mathbut: wahid.sha256.clone(),
                hajm_mathbut: wahid.hajm,
                isdar_mathbut: "1.7".to_owned(),
                sha256: "b".repeat(64),
                hajm: 11,
                waqt: jiff::Timestamp::now().to_string(),
            },
        );

        let nafidha = jiff::SignedDuration::from_mins(30);
        let alaan = jiff::Timestamp::now();
        assert!(sijill.hadith(&miftah, &wahid, nafidha, alaan));
        // A pin that moved retires it; so does a window that has turned over.
        let mut mukhtalif = wahid.clone();
        mukhtalif.sha256 = "c".repeat(64);
        assert!(!sijill.hadith(&miftah, &mukhtalif, nafidha, alaan));
        assert!(!sijill.hadith(&miftah, &wahid, jiff::SignedDuration::ZERO, alaan));

        let rasd = sijill.rasd(&miftah);
        assert!(rasd.is_some_and(|rasd| rasd.yuqaran(&wahid) && !rasd.mutabiq()));
    }

    /// The bridge the whole install rests on, proved without a network.
    ///
    /// The installer is synchronous and blocks inside `ijlib` until the client
    /// answers, and the client is asynchronous and cannot be driven from the
    /// thread that is blocking. This runs exactly that arrangement — a blocking
    /// task asking twice while the task that owns the runtime serves both — so a
    /// tokio whose blocking rules changed under us fails here rather than at the
    /// moment somebody presses install.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn alqanat_tanqul_wa_turajji_alkhata() -> Result<(), Box<dyn std::error::Error>> {
        use taarib_tathbeet::jalb_khariji::NaqilKhariji as _;

        let mujallad = tempfile::tempdir()?;
        let hadaf = mujallad.path().join("update.zip");
        let mafqud = mujallad.path().join("extra.zip");
        let (mursil, mut mutalaqqi) = tokio::sync::mpsc::channel::<super::TalabJalb>(1);

        let khaam = hadaf.clone();
        let ghaib = mafqud.clone();
        let muhimma = tokio::task::spawn_blocking(move || {
            let mut naqil = super::NaqilQanat { mursil };
            let awwal = naqil.ijlib("update.zip", "https://rt.example/update.zip", &khaam);
            let thani = naqil.ijlib("extra.zip", "https://rt.example/extra.zip", &ghaib);
            (awwal.is_ok(), thani.err())
        });

        // The fixture stands in for the client: one address is served, the other
        // is refused in the installer's own vocabulary.
        while let Some(talab) = mutalaqqi.recv().await {
            let natija = if talab.rabt.ends_with("update.zip") {
                std::fs::write(&talab.hadaf, b"abc")
                    .map_err(|sabab| super::khata_naql(&talab.ism, &talab.rabt, &sabab.to_string()))
            } else {
                Err(super::khata_naql(
                    &talab.ism,
                    &talab.rabt,
                    "no such address in the fixture",
                ))
            };
            drop(talab.radd.send(natija));
        }

        let (najah, khata) = muhimma.await?;
        assert!(
            najah,
            "the served address wrote its body through the channel"
        );
        assert_eq!(std::fs::read(&hadaf)?, b"abc");
        assert!(!mafqud.exists());
        assert!(matches!(
            khata,
            Some(taarib_tathbeet::khata::KhataTathbeet::TanzeelMutaadhdhir { ref ism, .. })
                if ism == "extra.zip"
        ));
        Ok(())
    }

    /// The watch's record lives under the registry cache and nowhere else.
    #[test]
    fn masar_almuraqaba_tahta_almakhbaa() {
        let masarat = taarib_usus::masarat::Masarat::min_judhur("/bayanat", "/idadat");
        let masar = super::masar_muraqaba(&masarat);
        assert!(
            masar.is_ok_and(|masar| masar.ends_with("muraqaba.json")
                && masar.starts_with(Path::new("/bayanat")))
        );
    }
}
