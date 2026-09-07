//! The fetch flow: the manifest, the shards a user's games fall in, revocation.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use futures::StreamExt as _;
use futures::stream::FuturesUnordered;
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_usus::masarat::kitaba_dharra;

use crate::fahras::{BayanMustawda, ShareehaMuwaththaqa, shareeha};
use crate::khata::{KhataMustawda, NatijatMustawda};
use crate::masadir::{
    MASAR_BAYAN, MUJALLAD_SHARAIH, SilsilatMasadir, ism_shareeha, masar_salih, masar_shareeha,
    qira_mahdouda,
};

/// The registry cache's own directory under
/// [`Masarat::makhbaa`](taarib_usus::masarat::Masarat::makhbaa).
///
/// Shards sit beneath it at their repository paths, so a cached copy and a
/// [`crate::masadir::MasdarMustawda::MujalladMahalli`] copy are the same file
/// in the same place.
pub const MUJALLAD_MAKHBAA: &str = "mustawda";

/// How many shards may be in flight at once.
///
/// A library of forty games touches at most forty shards, and six at a time
/// clears that in seven rounds without ever opening more than six sockets
/// against one forge — enough that one slow shard does not serialize the rest,
/// few enough that a client is not mistaken for an attack and rate-limited.
pub const HADD_MUTAWAZI: usize = 6;

/// The shard indices a set of games falls in, deduplicated and ordered.
#[must_use]
pub fn sharaih_matluba(alab: &[LubaId]) -> BTreeSet<u16> {
    alab.iter().copied().map(shareeha).collect()
}

/// A manifest and the shards that were verified against it.
///
/// Kept together so a caller cannot pair shards with a manifest other than the
/// one that vouched for them.
#[derive(Debug)]
pub struct FahrasMajlub {
    /// The manifest.
    pub bayan: BayanMustawda,
    /// The source that served the manifest, as [`MasdarMustawda::wasf`]
    /// names it.
    ///
    /// Carried so that the revocation refresh that rides on this fetch can
    /// record which source vouched for the manifest without asking the chain a
    /// second time.
    ///
    /// [`MasdarMustawda::wasf`]: crate::masadir::MasdarMustawda::wasf
    pub masdar_bayan: String,
    /// The shards, ordered by index.
    pub sharaih: Vec<ShareehaMuwaththaqa>,
}

impl FahrasMajlub {
    /// The shard holding one game's records, when that shard was fetched.
    #[must_use]
    pub fn shareeha_luba(&self, luba: LubaId) -> Option<&ShareehaMuwaththaqa> {
        let raqm = shareeha(luba);
        self.sharaih.iter().find(|wahida| wahida.raqm() == raqm)
    }
}

/// Fetches the manifest and then only the shards the given games fall in.
///
/// # Errors
///
/// As [`jalb_bayan`] and [`jalb_sharaih`].
pub async fn jalb_fahras(
    silsila: &SilsilatMasadir,
    alab: &[LubaId],
    makhbaa: &Path,
    mukhazzan: Option<u64>,
) -> NatijatMustawda<FahrasMajlub> {
    let (bayan, masdar_bayan) = jalb_bayan_maa_masdar(silsila, mukhazzan).await?;
    let sharaih = jalb_sharaih(silsila, &bayan, alab, makhbaa).await?;
    Ok(FahrasMajlub {
        bayan,
        masdar_bayan,
        sharaih,
    })
}

/// Where one shard's verified bytes are cached.
///
/// # Errors
///
/// [`KhataMustawda::ShareehaMajhula`] when `raqm` is not a shard index.
pub fn masar_makhbaa_shareeha(makhbaa: &Path, raqm: u16) -> NatijatMustawda<PathBuf> {
    Ok(makhbaa
        .join(MUJALLAD_MAKHBAA)
        .join(MUJALLAD_SHARAIH)
        .join(ism_shareeha(raqm)?))
}

/// Fetches the global manifest and parses it.
///
/// # Errors
///
/// [`KhataMustawda::LaMasdar`] when no source answered, and whatever
/// [`BayanMustawda::min_bayt`] refuses: a schema this build does not read, or a
/// revision below `mukhazzan`.
pub async fn jalb_bayan(
    silsila: &SilsilatMasadir,
    mukhazzan: Option<u64>,
) -> NatijatMustawda<BayanMustawda> {
    jalb_bayan_maa_masdar(silsila, mukhazzan)
        .await
        .map(|(bayan, _)| bayan)
}

/// As [`jalb_bayan`], naming the source that served the manifest.
///
/// # Errors
///
/// As [`jalb_bayan`].
pub async fn jalb_bayan_maa_masdar(
    silsila: &SilsilatMasadir,
    mukhazzan: Option<u64>,
) -> NatijatMustawda<(BayanMustawda, String)> {
    let (bayt, masdar) = silsila.jalb_maa_masdar(MASAR_BAYAN).await?;
    let bayan = BayanMustawda::min_bayt(&bayt, mukhazzan)?;
    tracing::debug!(
        tasalsul = bayan.tasalsul,
        sharaih = bayan.sharaih.len(),
        masdar = %masdar,
        "manifest read"
    );
    Ok((bayan, masdar))
}

/// Fetches every shard the given games fall in, and nothing else.
///
/// A shard whose cached copy the manifest still vouches for is read from
/// `makhbaa` and never requested, so an unchanged manifest costs no network at
/// all. The rest are fetched at most [`HADD_MUTAWAZI`] at a time.
///
/// # Errors
///
/// [`KhataMustawda::LaMasdar`] when no source answered for a shard that had to
/// be fetched, and whatever [`ShareehaMuwaththaqa::min_bayt`] refuses.
pub async fn jalb_sharaih(
    silsila: &SilsilatMasadir,
    bayan: &BayanMustawda,
    alab: &[LubaId],
    makhbaa: &Path,
) -> NatijatMustawda<Vec<ShareehaMuwaththaqa>> {
    let arqam = sharaih_matluba(alab);
    if arqam.is_empty() {
        return Ok(Vec::new());
    }

    let mut baqiya = arqam.into_iter();
    let mut jariya = FuturesUnordered::new();
    let mut hasad: Vec<ShareehaMuwaththaqa> = Vec::new();

    for raqm in baqiya.by_ref().take(HADD_MUTAWAZI) {
        jariya.push(jalb_shareeha(silsila, bayan, raqm, makhbaa));
    }
    while let Some(natija) = jariya.next().await {
        hasad.push(natija?);
        if let Some(raqm) = baqiya.next() {
            jariya.push(jalb_shareeha(silsila, bayan, raqm, makhbaa));
        }
    }

    hasad.sort_by_key(ShareehaMuwaththaqa::raqm);
    Ok(hasad)
}

/// Fetches one shard, preferring a cached copy the manifest still vouches for.
///
/// # Errors
///
/// [`KhataMustawda::ShareehaMajhula`] when `raqm` is not a shard index,
/// [`KhataMustawda::LaMasdar`] when no source answered, and whatever
/// [`ShareehaMuwaththaqa::min_bayt`] refuses.
pub async fn jalb_shareeha(
    silsila: &SilsilatMasadir,
    bayan: &BayanMustawda,
    raqm: u16,
    makhbaa: &Path,
) -> NatijatMustawda<ShareehaMuwaththaqa> {
    let masar = masar_makhbaa_shareeha(makhbaa, raqm)?;

    if let Some(bayt) = min_makhbaa(&masar, raqm, bayan).await {
        tracing::debug!(raqm, "shard served from the cache");
        return ShareehaMuwaththaqa::min_bayt(raqm, &bayt, bayan);
    }

    let nisbi = masar_shareeha(raqm)?;
    let bayt = silsila.jalb(&nisbi).await?;
    let shareeha = ShareehaMuwaththaqa::min_bayt(raqm, &bayt, bayan)?;
    // The only write, and it is downstream of the constructor: bytes that did
    // not verify never reach the cache for a later run to read back.
    ila_makhbaa(masar, bayt).await;
    Ok(shareeha)
}

/// Fetches the revocation list bytes and returns them unread: only
/// `taarib_aman::qaimat_sahb::QaimatSahb::min_bayt` verifies the owner's
/// signature over them and is allowed to decide what they mean.
///
/// # Errors
///
/// [`KhataMustawda::BayanTalif`] when the manifest's revocation path is not a
/// repository path this build resolves, and [`KhataMustawda::LaMasdar`] when no
/// source answered.
pub async fn jalb_qaimat_sahb(
    silsila: &SilsilatMasadir,
    bayan: &BayanMustawda,
) -> NatijatMustawda<Vec<u8>> {
    jalb_qaimat_sahb_maa_masdar(silsila, bayan)
        .await
        .map(|(bayt, _)| bayt)
}

/// As [`jalb_qaimat_sahb`], naming the source that served the list.
///
/// # Errors
///
/// As [`jalb_qaimat_sahb`].
pub async fn jalb_qaimat_sahb_maa_masdar(
    silsila: &SilsilatMasadir,
    bayan: &BayanMustawda,
) -> NatijatMustawda<(Vec<u8>, String)> {
    let nisbi = bayan.rabt_qaimat_sahb.trim();
    if !masar_salih(nisbi) {
        // Repository-relative only: an absolute address is a manifest aiming
        // the fetch at a host of its choosing, and no offline mirror serves it.
        return Err(KhataMustawda::BayanTalif {
            sabab: format!("the revocation path {nisbi:?} is not a repository path"),
        });
    }
    silsila.jalb_maa_masdar(nisbi).await
}

/// The cached bytes of one shard, when the manifest still vouches for them.
async fn min_makhbaa(masar: &Path, raqm: u16, bayan: &BayanMustawda) -> Option<Vec<u8>> {
    // Missing, unreadable and stale are all one answer here — a miss, never a
    // failure, because the chain still has to be asked either way.
    let bayt = qira_mahdouda(masar).await.ok()?;
    let mahsuba = Basma::min_bayt(*blake3::hash(&bayt).as_bytes());
    bayan.muhaddatha(raqm, &mahsuba).then_some(bayt)
}

/// Writes verified shard bytes to the cache, reporting a failure and no more.
async fn ila_makhbaa(masar: PathBuf, bayt: Vec<u8>) {
    let wasf = masar.display().to_string();
    match tokio::task::spawn_blocking(move || kitaba_dharra(&masar, &bayt)).await {
        Ok(Ok(())) => {},
        Ok(Err(khata)) => {
            tracing::warn!(masar = %wasf, khata = %khata.li_sijill(), "shard was not cached");
        },
        Err(khata) => tracing::warn!(masar = %wasf, khata = %khata, "shard was not cached"),
    }
}
