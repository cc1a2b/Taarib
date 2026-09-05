//! Downloading a package: range resume, mirror failover, streamed hashing.

use std::collections::HashMap;
use std::fmt;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, RANGE};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use sysinfo::{Disk, Disks};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::ruqaa::MulakhkhasRuqaa;
use taarib_mustalahat::sawt::MulakhkhasSawt;
use taarib_usus::masarat::kitaba_dharra;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, AsyncWriteExt as _};
use tokio::sync::mpsc::UnboundedSender;

use crate::fahras::MulakhkhasDhakira;
use crate::khata::{KhataMustawda, NatijatMustawda};

/// The sidecar schema this build writes and reads.
pub const ISDAR_SIJILL: u32 = 1;

/// Appended to the destination to name the partial file.
const IMTIDAD_JUZ: &str = ".juz";

/// Appended to the destination to name the sidecar.
const IMTIDAD_SIJILL: &str = ".juz.json";

/// Longest the connection itself may take to establish.
const MUHLAT_ITTISAL: Duration = Duration::from_secs(12);

/// Longest a source may take to answer with its headers.
const MUHLAT_TARWISA: Duration = Duration::from_mins(1);

/// Longest a source may go without delivering another chunk.
const MUHLAT_QITA: Duration = Duration::from_secs(45);

/// How many redirect hops a download may follow.
const HADD_TAHWIL: usize = 3;

/// Longest URL accepted from a listing.
const HADD_TUL_RABT: usize = 2048;

/// Largest sidecar read before it is treated as somebody else's file.
const HADD_SIJILL: u64 = 4096;

/// The buffer a local hashing pass reads through.
const HAJM_MAHFAZA: usize = 1 << 20;

/// How many bytes pass between two progress reports.
const KHUTWAT_TAQREER: u64 = 512 * 1024;

/// The units a readable size is written in.
const WAHDAT: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

const AMAL_QIRA: &str = "reading a partial download";
const AMAL_KITABA: &str = "writing a partial download";
const AMAL_INSHA: &str = "creating the download folder";
const AMAL_NAQL: &str = "moving a verified download into place";
const AMAL_HADHF: &str = "discarding a partial download";

/// Which part of a download the bytes being reported belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarhalatTanzeel {
    /// Re-hashing the bytes already on disk before the transfer continues.
    Istinaf,
    /// Transferring from a source.
    Naql,
    /// Hashing a complete file against the declared fingerprint.
    Tahaqquq,
    /// Verified and in place.
    Tamma,
}

impl MarhalatTanzeel {
    /// The label the interface shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Istinaf => "استئناف ما نُزّل سابقًا",
            Self::Naql => "جارٍ التنزيل",
            Self::Tahaqquq => "التحقّق من البصمة",
            Self::Tamma => "اكتمل التنزيل",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Istinaf => "Resuming what was already downloaded",
            Self::Naql => "Downloading",
            Self::Tahaqquq => "Verifying the fingerprint",
            Self::Tamma => "Download complete",
        }
    }
}

/// One progress report: bytes done, bytes declared, and the stage they belong
/// to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taqaddum {
    /// Bytes accounted for so far.
    pub manqul: u64,
    /// Bytes the listing declared.
    pub majmu: u64,
    /// The stage.
    pub marhala: MarhalatTanzeel,
}

impl Taqaddum {
    /// Bytes still to come.
    #[must_use]
    pub const fn mutabaqqi(&self) -> u64 {
        self.majmu.saturating_sub(self.manqul)
    }

    /// Bytes done, as the interface writes them.
    #[must_use]
    pub fn manqul_maqru(&self) -> String {
        hajm_maqru(self.manqul)
    }

    /// Bytes declared, as the interface writes them.
    #[must_use]
    pub fn majmu_maqru(&self) -> String {
        hajm_maqru(self.majmu)
    }
}

/// Where progress goes.
#[derive(Default)]
pub enum MukhbirTaqaddum {
    /// Nowhere.
    #[default]
    Samit,
    /// Onto an unbounded channel, which a slow reader cannot stall.
    Qanat(UnboundedSender<Taqaddum>),
    /// Into a callback, called on the task driving the download.
    Nida(Box<dyn Fn(Taqaddum) + Send + Sync>),
}

impl MukhbirTaqaddum {
    /// Wraps a callback.
    #[must_use]
    pub fn min_nida<F: Fn(Taqaddum) + Send + Sync + 'static>(nida: F) -> Self {
        Self::Nida(Box::new(nida))
    }

    /// Delivers one report, ignoring a receiver that has gone away.
    pub fn ballagh(&self, taqaddum: Taqaddum) {
        match self {
            Self::Samit => {}
            Self::Qanat(mursil) => {
                let _ = mursil.send(taqaddum);
            }
            Self::Nida(nida) => nida(taqaddum),
        }
    }
}


impl fmt::Debug for MukhbirTaqaddum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let naw = match self {
            Self::Samit => "Samit",
            Self::Qanat(_) => "Qanat",
            Self::Nida(_) => "Nida",
        };
        f.debug_struct("MukhbirTaqaddum").field("naw", &naw).finish()
    }
}

/// The record written beside a partial file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SijillJuzii {
    /// The sidecar schema version.
    pub isdar: u32,
    /// The source the bytes on disk came from.
    pub rabt: String,
    /// The size the listing declared.
    pub hajm: u64,
    /// The content hash the listing declared.
    pub basma: Basma,
}

impl SijillJuzii {
    /// The record for one request against one source.
    #[must_use]
    pub fn min_talab(talab: &TalabTanzeel, rabt: &str) -> Self {
        Self {
            isdar: ISDAR_SIJILL,
            rabt: rabt.to_owned(),
            hajm: talab.hajm,
            basma: talab.basma,
        }
    }

    /// Whether the bytes this record describes are the bytes a request wants.
    #[must_use]
    pub fn mutabiq(&self, talab: &TalabTanzeel) -> bool {
        self.isdar == ISDAR_SIJILL
            && self.hajm == talab.hajm
            && self.basma == talab.basma
            && talab.rawabit.iter().any(|rabt| rabt == &self.rabt)
    }

    /// Reads a sidecar, treating an absent, oversized or unparsable one as
    /// absent.
    ///
    /// Unparsable can only mean somebody else's file, never a half-written one:
    /// [`Self::uktub`] renames a complete sidecar over the old one, so a reader
    /// sees either generation whole and never a prefix of the newer.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::KhataMalaf`] when the file exists and cannot be read.
    pub async fn qira(masar: &Path) -> NatijatMustawda<Option<Self>> {
        let bayanat = match tokio::fs::metadata(masar).await {
            Ok(bayanat) => bayanat,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(sabab) => return Err(khata_malaf(masar, AMAL_QIRA, sabab)),
        };
        if !bayanat.is_file() || bayanat.len() > HADD_SIJILL {
            return Ok(None);
        }
        let bayt = tokio::fs::read(masar)
            .await
            .map_err(|sabab| khata_malaf(masar, AMAL_QIRA, sabab))?;
        Ok(serde_json::from_slice(&bayt).ok())
    }

    /// Writes a sidecar: beside, flushed, renamed over, through
    /// [`taarib_usus::masarat::kitaba_dharra`].
    ///
    /// Never in place. An in-place write that is cut short leaves a prefix that
    /// [`Self::qira`] reads as absent, which silently throws away a partial file
    /// worth gigabytes and restarts the transfer from zero.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::KhataMalaf`] when it cannot be written.
    pub async fn uktub(&self, masar: &Path) -> NatijatMustawda<()> {
        let bayt = serde_json::to_vec(self).map_err(|sabab| {
            khata_malaf(masar, AMAL_KITABA, std::io::Error::other(sabab))
        })?;
        let wijha = masar.to_path_buf();
        match tokio::task::spawn_blocking(move || kitaba_dharra(&wijha, &bayt)).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(khata)) => Err(khata_malaf(masar, AMAL_KITABA, std::io::Error::other(khata))),
            Err(khata) => Err(khata_malaf(masar, AMAL_KITABA, std::io::Error::other(khata))),
        }
    }
}

/// One package to fetch: where it is, how large it is, what it must hash to,
/// and where it lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TalabTanzeel {
    rawabit: Vec<String>,
    hajm: u64,
    basma: Basma,
    hadaf: PathBuf,
}

impl TalabTanzeel {
    /// A request built from its parts, sources in the order they are tried.
    #[must_use]
    pub fn jadeed(
        rawabit: Vec<String>,
        hajm: u64,
        basma: Basma,
        hadaf: impl Into<PathBuf>,
    ) -> Self {
        Self { rawabit, hajm, basma, hadaf: hadaf.into() }
    }

    /// The request one patch listing describes.
    #[must_use]
    pub fn min_ruqaa(mulakhkhas: &MulakhkhasRuqaa, hadaf: impl Into<PathBuf>) -> Self {
        Self::jadeed(
            rawabit(&mulakhkhas.rabt, mulakhkhas.rabt_mira.as_deref()),
            mulakhkhas.hajm,
            mulakhkhas.basmat_muhtawa,
            hadaf,
        )
    }

    /// The request one voice pack listing describes.
    #[must_use]
    pub fn min_sawt(mulakhkhas: &MulakhkhasSawt, hadaf: impl Into<PathBuf>) -> Self {
        Self::jadeed(
            rawabit(&mulakhkhas.rabt, mulakhkhas.rabt_mira.as_deref()),
            mulakhkhas.hajm,
            mulakhkhas.basmat_muhtawa,
            hadaf,
        )
    }

    /// The request one memory-share listing describes.
    ///
    /// A share is not a patch — nothing installs, and nothing here mints a
    /// permit — but it is a hashed file fetched from the release area, and
    /// that is the whole of what this module does. Resume, mirror failover,
    /// free-space checking and the final hash comparison all apply unchanged,
    /// which is the argument for listing shares in the index at all rather
    /// than building a second way to move bytes.
    #[must_use]
    pub fn min_dhakira(
        mulakhkhas: &MulakhkhasDhakira,
        hadaf: impl Into<PathBuf>,
    ) -> Self {
        Self::jadeed(
            rawabit(&mulakhkhas.rabt, mulakhkhas.rabt_mira.as_deref()),
            mulakhkhas.hajm,
            mulakhkhas.basma,
            hadaf,
        )
    }

    /// The sources, in the order they will be tried.
    #[must_use]
    pub fn rawabit(&self) -> &[String] {
        &self.rawabit
    }

    /// The size the listing declared, known before a byte is fetched.
    #[must_use]
    pub const fn hajm(&self) -> u64 {
        self.hajm
    }

    /// The same size as the interface writes it.
    #[must_use]
    pub fn hajm_maqru(&self) -> String {
        hajm_maqru(self.hajm)
    }

    /// The hash the bytes must produce.
    #[must_use]
    pub const fn basma(&self) -> Basma {
        self.basma
    }

    /// Where the verified file lands.
    #[must_use]
    pub fn hadaf(&self) -> &Path {
        &self.hadaf
    }

    /// The partial file.
    #[must_use]
    pub fn masar_juz(&self) -> PathBuf {
        damm_lahiqa(&self.hadaf, IMTIDAD_JUZ)
    }

    /// The sidecar beside it.
    #[must_use]
    pub fn masar_sijill(&self) -> PathBuf {
        damm_lahiqa(&self.hadaf, IMTIDAD_SIJILL)
    }
}

/// What one attempt against one source concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Muhawala {
    /// The file is verified and in place.
    Tamma,
    /// The source would not honour the range; everything on disk is unusable.
    IadatBidaya,
}

/// Throttles progress so a multi-gigabyte transfer does not report per chunk.
#[derive(Debug)]
struct Muraqib<'a> {
    mukhbir: &'a MukhbirTaqaddum,
    majmu: u64,
    akhir: u64,
    marhala: Option<MarhalatTanzeel>,
}

impl<'a> Muraqib<'a> {
    const fn jadeed(mukhbir: &'a MukhbirTaqaddum, majmu: u64) -> Self {
        Self { mukhbir, majmu, akhir: 0, marhala: None }
    }

    fn ballagh(&mut self, manqul: u64, marhala: MarhalatTanzeel) {
        let mustaqirra = self.marhala == Some(marhala);
        if mustaqirra && manqul.saturating_sub(self.akhir) < KHUTWAT_TAQREER {
            return;
        }
        self.hatman(manqul, marhala);
    }

    fn hatman(&mut self, manqul: u64, marhala: MarhalatTanzeel) {
        self.akhir = manqul;
        self.marhala = Some(marhala);
        self.mukhbir.ballagh(Taqaddum { manqul, majmu: self.majmu, marhala });
    }
}

/// Builds the client every package download goes through.
///
/// # Errors
///
/// [`KhataMustawda::TanzeelFashil`] when the TLS backend cannot be built.
pub fn amil_tanzeel() -> NatijatMustawda<Client> {
    Client::builder()
        .user_agent(format!("Taarib/{}", taarib_usus::ISDAR))
        .connect_timeout(MUHLAT_ITTISAL)
        // No whole-request timeout: it would abort a four-gigabyte voice pack.
        .redirect(reqwest::redirect::Policy::limited(HADD_TAHWIL))
        .https_only(true)
        .referer(false)
        .build()
        .map_err(|sabab| KhataMustawda::TanzeelFashil {
            rabt: "(client)".to_owned(),
            sabab: sabab.to_string(),
        })
}

/// The registry's own type, so the `static` below reads as one thing.
type KharitatAqfal = tokio::sync::Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>;

/// One lock per destination, so two transfers into the same file queue instead
/// of interleaving their appends.
///
/// A `static` because there is no owning type to hang it on and adding one
/// would not close the hole. [`nazzil`] is a free function; its callers share
/// nothing — the Studio builds a throwaway [`Client`] per command, and a
/// background refresh and a hand-started download arrive through separate
/// commands holding separate state. The invariant is per-process and per-path,
/// which is a `static`'s exact scope; a field would have to be threaded through
/// every caller and two callers each holding their own registry would still
/// race. The map is keyed by the destination as given: callers derive it from
/// the content hash, so one package is always spelled one way.
///
/// `tokio::sync::Mutex`, not `parking_lot`: the guard is held across the
/// transfer's awaits.
static AQFAL_TANZEEL: LazyLock<KharitatAqfal> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));

/// The lock for one destination, taken for the whole of a download.
///
/// Dead entries are swept on the way in rather than released on the way out: a
/// guard cannot run an async unlock from `Drop`. A strong count of one means
/// only the map holds that entry, so nobody owns the lock and nobody is queued
/// behind it — holding or awaiting one keeps a clone alive. The map is bounded
/// by the downloads in flight, not by the downloads ever attempted.
async fn qufl_hadaf(hadaf: &Path) -> tokio::sync::OwnedMutexGuard<()> {
    let qufl = {
        let mut kharita = AQFAL_TANZEEL.lock().await;
        kharita.retain(|_, qufl| Arc::strong_count(qufl) > 1);
        Arc::clone(
            kharita
                .entry(hadaf.to_path_buf())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    };
    qufl.lock_owned().await
}

/// Fetches one package, verifies it during transfer, and returns the verified
/// file.
///
/// Calls naming the same destination are serialised: a second caller waits for
/// the first to finish and then finds the file already in place, rather than
/// racing it into a hash mismatch that discards both transfers. The wait is the
/// whole transfer, so a caller that wants to know whether one is already under
/// way must track that itself.
///
/// # Errors
///
/// [`KhataMustawda::TanzeelFashil`] when no source delivered the bytes,
/// [`KhataMustawda::IstijabaFashila`] when the last source answered without
/// success, [`KhataMustawda::MisahaGhayrKafiya`] when the destination's
/// filesystem lacks room for the declared size plus its headroom,
/// [`KhataMustawda::HajmMufrit`] when a transfer passed the declared size,
/// [`KhataMustawda::TanzeelGhayrMutabiq`] when what arrived does not hash to
/// what the listing declared, and [`KhataMustawda::KhataMalaf`] when the
/// partial file or its sidecar cannot be read or written.
pub async fn nazzil(
    amil: &Client,
    talab: &TalabTanzeel,
    mukhbir: &MukhbirTaqaddum,
) -> NatijatMustawda<PathBuf> {
    if let Some(walid) = talab.hadaf.parent() {
        tokio::fs::create_dir_all(walid)
            .await
            .map_err(|sabab| khata_malaf(walid, AMAL_INSHA, sabab))?;
    }

    // Held from the resume read through the rename, because those are one
    // decision: everything below reads the sidecar, decides an offset, truncates
    // the partial file to it and appends. A second transfer that reads the same
    // offset appends over the first one's bytes, and each advances only its own
    // cursor, so the hash check rejects a transfer that was never at fault.
    let _harasa = qufl_hadaf(&talab.hadaf).await;

    if hadaf_muwaththaq(talab, mukhbir).await? {
        nazzif_hurr(talab).await?;
        tamma(talab, mukhbir);
        return Ok(talab.hadaf.clone());
    }
    if juz_kamil(talab, mukhbir).await? {
        tamma(talab, mukhbir);
        return Ok(talab.hadaf.clone());
    }

    let mut akhir: Option<KhataMustawda> = None;
    for rabt in &talab.rawabit {
        if !rabt_amin(rabt) {
            akhir = Some(KhataMustawda::TanzeelFashil {
                rabt: rabt.chars().take(HADD_TUL_RABT).collect(),
                sabab: "refused: packages are fetched over https only, from a plain address"
                    .to_owned(),
            });
            continue;
        }
        match min_masdar(amil, rabt, talab, mukhbir).await {
            Ok(()) => {
                tamma(talab, mukhbir);
                return Ok(talab.hadaf.clone());
            }
            Err(khata) if yatakarrar(&khata) => {
                tracing::warn!(rabt = rabt.as_str(), sabab = %khata, "a download source failed");
                akhir = Some(khata);
            }
            Err(khata) => return Err(khata),
        }
    }

    Err(akhir.unwrap_or_else(|| KhataMustawda::TanzeelFashil {
        rabt: "(none)".to_owned(),
        sabab: "the listing names no source to fetch from".to_owned(),
    }))
}

/// Bytes on disk a resumed download would keep, discarding leftovers that no
/// longer match the request.
///
/// Waits for a transfer already running into this destination, so the answer is
/// never a figure another task is in the middle of invalidating.
///
/// # Errors
///
/// [`KhataMustawda::KhataMalaf`] when a leftover cannot be read or removed.
pub async fn bayt_mustanafa(talab: &TalabTanzeel) -> NatijatMustawda<u64> {
    let _harasa = qufl_hadaf(&talab.hadaf).await;
    nuqtat_istinaf(talab).await
}

/// Discards a partial download and its sidecar.
///
/// Waits for a transfer already running into this destination rather than
/// deleting the file out from under it.
///
/// # Errors
///
/// [`KhataMustawda::KhataMalaf`] when either exists and cannot be removed.
pub async fn nazzif(talab: &TalabTanzeel) -> NatijatMustawda<()> {
    let _harasa = qufl_hadaf(&talab.hadaf).await;
    nazzif_hurr(talab).await
}

/// The same cleanup for callers already holding the destination's lock, which
/// [`tokio::sync::Mutex`] would deadlock on if they took it twice.
async fn nazzif_hurr(talab: &TalabTanzeel) -> NatijatMustawda<()> {
    hadhf(&talab.masar_juz()).await?;
    hadhf(&talab.masar_sijill()).await
}

/// A size as the interface writes it, in the largest unit that stays readable.
#[must_use]
pub fn hajm_maqru(hajm: u64) -> String {
    let mut qeema = hajm;
    let mut khana = 0_usize;
    // A shift: `integer_division` is denied and dividing by 1024 is one.
    while qeema >= 1024 && khana.saturating_add(1) < WAHDAT.len() {
        qeema >>= 10;
        khana = khana.saturating_add(1);
    }
    format!("{qeema} {}", WAHDAT.get(khana).copied().unwrap_or("B"))
}

/// Runs one source to a conclusion, restarting once from zero when it refuses
/// to serve a range.
async fn min_masdar(
    amil: &Client,
    rabt: &str,
    talab: &TalabTanzeel,
    mukhbir: &MukhbirTaqaddum,
) -> NatijatMustawda<()> {
    let mut istinaf = true;
    for _ in 0..2 {
        match naql(amil, rabt, talab, mukhbir, istinaf).await? {
            Muhawala::Tamma => return Ok(()),
            Muhawala::IadatBidaya => {
                nazzif_hurr(talab).await?;
                istinaf = false;
            }
        }
    }
    Err(KhataMustawda::TanzeelFashil {
        rabt: rabt.to_owned(),
        sabab: "the source served neither the requested range nor the whole file".to_owned(),
    })
}

/// One transfer against one source.
async fn naql(
    amil: &Client,
    rabt: &str,
    talab: &TalabTanzeel,
    mukhbir: &MukhbirTaqaddum,
    istinaf: bool,
) -> NatijatMustawda<Muhawala> {
    let juz = talab.masar_juz();
    let bidaya = if istinaf { nuqtat_istinaf(talab).await? } else { 0 };
    akkid_misaha(talab, bidaya).await?;

    // `identity`: a range offset must name the byte that is hashed and written.
    let mut mabni = amil.get(rabt).header(ACCEPT_ENCODING, "identity");
    if bidaya > 0 {
        mabni = mabni.header(RANGE, format!("bytes={bidaya}-"));
    }
    let mut radd = tokio::time::timeout(MUHLAT_TARWISA, mabni.send())
        .await
        .map_err(|_| KhataMustawda::TanzeelFashil {
            rabt: rabt.to_owned(),
            sabab: format!("no answer within {} seconds", MUHLAT_TARWISA.as_secs()),
        })?
        .map_err(|sabab| KhataMustawda::TanzeelFashil {
            rabt: rabt.to_owned(),
            sabab: sabab.to_string(),
        })?;

    let hala = radd.status();
    let mustanaf = match hala {
        StatusCode::PARTIAL_CONTENT if bidaya > 0 => {
            let qeema = radd.headers().get(CONTENT_RANGE).and_then(|qeema| qeema.to_str().ok());
            match qeema.and_then(bidayat_nitaq) {
                Some((awwal, kull))
                    if awwal == bidaya && kull.is_none_or(|majmu| majmu == talab.hajm) =>
                {
                    true
                }
                _ => return Ok(Muhawala::IadatBidaya),
            }
        }
        // A 200 to a range request is the whole file; appending it corrupts.
        StatusCode::OK => false,
        StatusCode::RANGE_NOT_SATISFIABLE => return Ok(Muhawala::IadatBidaya),
        _ => {
            return Err(KhataMustawda::IstijabaFashila {
                rabt: rabt.to_owned(),
                ramz: hala.as_u16(),
            });
        }
    };

    let mabda = if mustanaf { bidaya } else { 0 };
    if radd.content_length().is_some_and(|tul| tul > talab.hajm.saturating_sub(mabda)) {
        return Err(KhataMustawda::HajmMufrit { muallan: talab.hajm });
    }

    let mut muraqib = Muraqib::jadeed(mukhbir, talab.hajm);
    let mut hashi = blake3::Hasher::new();
    if mustanaf {
        let maqru =
            basmat_juzii(&juz, bidaya, &mut hashi, &mut muraqib, MarhalatTanzeel::Istinaf).await?;
        if maqru != bidaya {
            return Ok(Muhawala::IadatBidaya);
        }
    }

    let mut malaf =
        if mustanaf { fath_ilhaq(&juz, bidaya).await? } else { fath_jadeed(&juz).await? };
    SijillJuzii::min_talab(talab, rabt).uktub(&talab.masar_sijill()).await?;

    let mut manqul = mabda;
    muraqib.hatman(manqul, MarhalatTanzeel::Naql);
    loop {
        let qita = match tokio::time::timeout(MUHLAT_QITA, radd.chunk()).await {
            Ok(Ok(Some(qita))) => qita,
            Ok(Ok(None)) => break,
            Ok(Err(sabab)) => {
                return Err(inqita(&mut malaf, rabt, &sabab.to_string()).await);
            }
            Err(_) => {
                let sabab = format!("stalled for {} seconds", MUHLAT_QITA.as_secs());
                return Err(inqita(&mut malaf, rabt, &sabab).await);
            }
        };

        manqul = manqul.saturating_add(u64::try_from(qita.len()).unwrap_or(u64::MAX));
        if manqul > talab.hajm {
            drop(malaf);
            nazzif_hurr(talab).await?;
            return Err(KhataMustawda::HajmMufrit { muallan: talab.hajm });
        }
        hashi.update(&qita);
        malaf
            .write_all(&qita)
            .await
            .map_err(|sabab| khata_malaf(&juz, AMAL_KITABA, sabab))?;
        muraqib.ballagh(manqul, MarhalatTanzeel::Naql);
    }

    aghliq(&mut malaf, &juz).await?;
    drop(malaf);

    if manqul != talab.hajm {
        return Err(KhataMustawda::TanzeelFashil {
            rabt: rabt.to_owned(),
            sabab: format!("the source closed after {manqul} of {} bytes", talab.hajm),
        });
    }

    muraqib.hatman(manqul, MarhalatTanzeel::Tahaqquq);
    akkid_wa_anhi(talab, Basma::min_bayt(*hashi.finalize().as_bytes()), rabt).await?;
    Ok(Muhawala::Tamma)
}

/// Where a resumed transfer would continue from, after leftovers that do not
/// belong to this request have been discarded.
async fn nuqtat_istinaf(talab: &TalabTanzeel) -> NatijatMustawda<u64> {
    let juz = talab.masar_juz();
    let Some(sijill) = SijillJuzii::qira(&talab.masar_sijill()).await? else {
        nazzif_hurr(talab).await?;
        return Ok(0);
    };
    if !sijill.mutabiq(talab) {
        nazzif_hurr(talab).await?;
        return Ok(0);
    }

    let bayanat = match tokio::fs::metadata(&juz).await {
        Ok(bayanat) => bayanat,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            hadhf(&talab.masar_sijill()).await?;
            return Ok(0);
        }
        Err(sabab) => return Err(khata_malaf(&juz, AMAL_QIRA, sabab)),
    };
    if !bayanat.is_file() || bayanat.len() >= talab.hajm {
        nazzif_hurr(talab).await?;
        return Ok(0);
    }
    Ok(bayanat.len())
}

/// Refuses a transfer the destination's filesystem cannot hold, crediting the
/// bytes a partial file already holds against the declared size and a tenth of
/// headroom. Skipped when no mounted disk can be matched to the destination.
async fn akkid_misaha(talab: &TalabTanzeel, mahjuz: u64) -> NatijatMustawda<()> {
    let Some(walid) = talab.hadaf.parent() else { return Ok(()) };
    let Ok(walid) = tokio::fs::canonicalize(walid).await else { return Ok(()) };
    let Some(mutah) = misaha_mutaha(&walid) else { return Ok(()) };

    #[expect(
        clippy::integer_division,
        reason = "a tenth of headroom: the divisor is a constant and truncation is wanted"
    )]
    let hamish = talab.hajm / 10;
    let matlub = talab.hajm.saturating_add(hamish).saturating_sub(mahjuz);
    if mutah >= matlub {
        return Ok(());
    }
    Err(KhataMustawda::MisahaGhayrKafiya { matlub, mutah, masar: talab.hadaf.clone() })
}

/// Free bytes on the mounted filesystem whose mount point is the longest
/// prefix of `masar`, or `None` when no mount point matches.
fn misaha_mutaha(masar: &Path) -> Option<u64> {
    let masar = bila_badiya_harfiya(masar);
    let aqras = Disks::new_with_refreshed_list();
    aqras
        .list()
        .iter()
        .filter(|qurs| masar.starts_with(qurs.mount_point()))
        .max_by_key(|qurs| qurs.mount_point().as_os_str().len())
        .map(Disk::available_space)
}

/// The path with a Windows verbatim prefix removed, so it compares against the
/// mount points the platform reports.
fn bila_badiya_harfiya(masar: &Path) -> PathBuf {
    let Some(nass) = masar.to_str() else { return masar.to_path_buf() };
    if let Some(baqi) = nass.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{baqi}"));
    }
    nass.strip_prefix(r"\\?\").map_or_else(|| masar.to_path_buf(), PathBuf::from)
}

/// Whether the destination already holds this exact package.
async fn hadaf_muwaththaq(
    talab: &TalabTanzeel,
    mukhbir: &MukhbirTaqaddum,
) -> NatijatMustawda<bool> {
    match tokio::fs::metadata(&talab.hadaf).await {
        Ok(bayanat) if bayanat.is_file() && bayanat.len() == talab.hajm => {}
        Ok(_) => return Ok(false),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(sabab) => return Err(khata_malaf(&talab.hadaf, AMAL_QIRA, sabab)),
    }

    let mut muraqib = Muraqib::jadeed(mukhbir, talab.hajm);
    let mut hashi = blake3::Hasher::new();
    let maqru = basmat_juzii(
        &talab.hadaf,
        talab.hajm,
        &mut hashi,
        &mut muraqib,
        MarhalatTanzeel::Tahaqquq,
    )
    .await?;
    Ok(maqru == talab.hajm && Basma::min_bayt(*hashi.finalize().as_bytes()) == talab.basma)
}

/// Whether a partial file already holds every declared byte and verifies.
async fn juz_kamil(talab: &TalabTanzeel, mukhbir: &MukhbirTaqaddum) -> NatijatMustawda<bool> {
    let juz = talab.masar_juz();
    let Some(sijill) = SijillJuzii::qira(&talab.masar_sijill()).await? else {
        return Ok(false);
    };
    if !sijill.mutabiq(talab) {
        return Ok(false);
    }
    match tokio::fs::metadata(&juz).await {
        Ok(bayanat) if bayanat.is_file() && bayanat.len() == talab.hajm => {}
        Ok(_) => return Ok(false),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(sabab) => return Err(khata_malaf(&juz, AMAL_QIRA, sabab)),
    }

    let mut muraqib = Muraqib::jadeed(mukhbir, talab.hajm);
    let mut hashi = blake3::Hasher::new();
    let maqru =
        basmat_juzii(&juz, talab.hajm, &mut hashi, &mut muraqib, MarhalatTanzeel::Tahaqquq).await?;
    if maqru != talab.hajm {
        nazzif_hurr(talab).await?;
        return Ok(false);
    }
    akkid_wa_anhi(talab, Basma::min_bayt(*hashi.finalize().as_bytes()), &sijill.rabt).await?;
    Ok(true)
}

/// Compares a computed hash to the declared one, then puts the file in place.
async fn akkid_wa_anhi(talab: &TalabTanzeel, mahsuba: Basma, rabt: &str) -> NatijatMustawda<()> {
    if mahsuba != talab.basma {
        nazzif_hurr(talab).await?;
        return Err(KhataMustawda::TanzeelGhayrMutabiq {
            rabt: rabt.to_owned(),
            muallana: talab.basma.to_string(),
            mahsuba: mahsuba.to_string(),
        });
    }
    tokio::fs::rename(&talab.masar_juz(), &talab.hadaf)
        .await
        .map_err(|sabab| khata_malaf(&talab.hadaf, AMAL_NAQL, sabab))?;
    hadhf(&talab.masar_sijill()).await
}

/// Feeds a hasher from a file, up to a byte budget, reporting as it goes.
async fn basmat_juzii(
    masar: &Path,
    hadd: u64,
    hashi: &mut blake3::Hasher,
    muraqib: &mut Muraqib<'_>,
    marhala: MarhalatTanzeel,
) -> NatijatMustawda<u64> {
    let mut malaf = File::open(masar)
        .await
        .map_err(|sabab| khata_malaf(masar, AMAL_QIRA, sabab))?;
    let mut mahfaza = vec![0_u8; HAJM_MAHFAZA];
    let mut maqru = 0_u64;

    muraqib.hatman(0, marhala);
    while maqru < hadd {
        let matlub = usize::try_from(hadd.saturating_sub(maqru))
            .unwrap_or(usize::MAX)
            .min(HAJM_MAHFAZA);
        let Some(nafidha) = mahfaza.get_mut(..matlub) else { break };
        let adad = malaf
            .read(nafidha)
            .await
            .map_err(|sabab| khata_malaf(masar, AMAL_QIRA, sabab))?;
        if adad == 0 {
            break;
        }
        let Some(qism) = nafidha.get(..adad) else { break };
        hashi.update(qism);
        maqru = maqru.saturating_add(u64::try_from(adad).unwrap_or(0));
        muraqib.ballagh(maqru, marhala);
    }
    Ok(maqru)
}

/// Opens a partial file for appending exactly at the byte that was hashed.
async fn fath_ilhaq(juz: &Path, bidaya: u64) -> NatijatMustawda<File> {
    let mut malaf = OpenOptions::new()
        .write(true)
        .open(juz)
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))?;
    malaf
        .set_len(bidaya)
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))?;
    malaf
        .seek(SeekFrom::Start(bidaya))
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))?;
    Ok(malaf)
}

/// Opens a partial file from nothing, discarding whatever was there.
async fn fath_jadeed(juz: &Path) -> NatijatMustawda<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(juz)
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))
}

/// Flushes a partial file to the device so its length is a resume point.
async fn aghliq(malaf: &mut File, juz: &Path) -> NatijatMustawda<()> {
    malaf
        .flush()
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))?;
    malaf
        .sync_all()
        .await
        .map_err(|sabab| khata_malaf(juz, AMAL_KITABA, sabab))
}

/// Ends a broken transfer, keeping what arrived so the next run resumes it.
async fn inqita(malaf: &mut File, rabt: &str, sabab: &str) -> KhataMustawda {
    let _ = malaf.flush().await;
    let _ = malaf.sync_all().await;
    KhataMustawda::TanzeelFashil { rabt: rabt.to_owned(), sabab: sabab.to_owned() }
}

/// Reports the last state a caller sees.
fn tamma(talab: &TalabTanzeel, mukhbir: &MukhbirTaqaddum) {
    mukhbir.ballagh(Taqaddum {
        manqul: talab.hajm,
        majmu: talab.hajm,
        marhala: MarhalatTanzeel::Tamma,
    });
}

/// Whether another source is worth trying after this failure.
const fn yatakarrar(khata: &KhataMustawda) -> bool {
    matches!(
        khata,
        KhataMustawda::TanzeelFashil { .. } | KhataMustawda::IstijabaFashila { .. }
    )
}

/// Deletes a file, treating "already gone" as success.
async fn hadhf(masar: &Path) -> NatijatMustawda<()> {
    match tokio::fs::remove_file(masar).await {
        Ok(()) => Ok(()),
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(sabab) => Err(khata_malaf(masar, AMAL_HADHF, sabab)),
    }
}

fn khata_malaf(masar: &Path, amal: &'static str, sabab: std::io::Error) -> KhataMustawda {
    KhataMustawda::KhataMalaf { masar: masar.to_path_buf(), amal, sabab }
}

/// The primary source followed by its mirror, skipping an absent mirror.
fn rawabit(rabt: &str, mira: Option<&str>) -> Vec<String> {
    let mut kul = Vec::with_capacity(2);
    kul.push(rabt.to_owned());
    if let Some(mira) = mira {
        kul.push(mira.to_owned());
    }
    kul
}

/// Names a sibling of the destination.
fn damm_lahiqa(masar: &Path, lahiqa: &str) -> PathBuf {
    let mut ism = masar.as_os_str().to_owned();
    ism.push(lahiqa);
    PathBuf::from(ism)
}

/// Whether an address from a listing is one worth opening.
fn rabt_amin(rabt: &str) -> bool {
    if rabt.len() <= "https://".len() || rabt.len() > HADD_TUL_RABT {
        return false;
    }
    if rabt.bytes().any(|bayt| bayt.is_ascii_control() || bayt == b' ') {
        return false;
    }
    rabt.get(.."https://".len()).is_some_and(|badiya| badiya.eq_ignore_ascii_case("https://"))
}

/// The first byte and the total length a `Content-Range` header states.
fn bidayat_nitaq(qeema: &str) -> Option<(u64, Option<u64>)> {
    let baqi = qeema.trim().strip_prefix("bytes ")?;
    let (nitaq, kull) = baqi.split_once('/')?;
    let (awwal, _) = nitaq.split_once('-')?;
    let bidaya = awwal.trim().parse::<u64>().ok()?;
    let majmu = match kull.trim() {
        "*" => None,
        raqm => Some(raqm.parse::<u64>().ok()?),
    };
    Some((bidaya, majmu))
}
