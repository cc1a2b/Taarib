//! Fetching an update: free-space refusal, range resume, streamed hashing.

use std::path::{Path, PathBuf};

use reqwest::header::RANGE;
use sha2::{Digest as _, Sha256};
use sysinfo::{Disk, Disks};
use tokio::io::AsyncWriteExt as _;

use crate::bayan::MadkhalTahdith;
use crate::khata::{KhataTahdith, NatijatTahdith};

/// The suffix a partial download carries until it is whole.
const IMTIDAD_JUZ: &str = ".juz";

/// Headroom demanded beyond the declared size, as a divisor: a tenth.
const MUQAM_HAMISH: u64 = 10;

/// Longest a source may go without delivering another chunk.
const MUHLAT_QITA: std::time::Duration = std::time::Duration::from_mins(1);

/// A verified update file, ready for [`crate::tabdil`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalafMuhaqqaq {
    /// Where it landed.
    pub masar: PathBuf,
    /// The hash it was verified against, lowercase hex.
    pub sha256: String,
    /// Its length in bytes.
    pub hajm: u64,
}

/// Downloads one manifest entry into `sandooq` and verifies it.
///
/// Resumes a partial file left by an interrupted run through a `Range` request,
/// enforces the declared size as a ceiling while streaming, and hashes as it
/// writes so the bytes are never read twice. A file that fails verification is
/// deleted: a wrong update kept on disk is a wrong update something later
/// resumes.
///
/// # Errors
///
/// [`KhataTahdith::MisahaGhayrKafiya`] before anything is fetched when the
/// destination cannot hold the transfer, [`KhataTahdith::QanatGhayrMutaha`] when
/// the source cannot be reached, [`KhataTahdith::HajmMufrit`] when the body runs
/// past its declared size, [`KhataTahdith::IstinafMutaadhdhir`] when a partial
/// file cannot be continued, [`KhataTahdith::TanzeelGhayrMutabiq`] when the
/// completed file hashes to something else, and
/// [`KhataTahdith::KhataMalaf`] for the local reads and writes.
pub async fn ijlib(
    madkhal: &MadkhalTahdith,
    sandooq: &Path,
) -> NatijatTahdith<MalafMuhaqqaq> {
    let hadaf = sandooq.join(ism_min_rabt(&madkhal.rabt));
    let juz = masar_juz(&hadaf);

    let mahjuz = tokio::fs::metadata(&juz).await.map_or(0, |bayan| bayan.len());
    // A partial longer than the whole file describes a different file.
    let mahjuz = if mahjuz > madkhal.hajm { 0 } else { mahjuz };
    if mahjuz == 0 {
        let _ = tokio::fs::remove_file(&juz).await;
    }
    akkid_misaha(&hadaf, madkhal.hajm, mahjuz)?;

    // The resumed prefix must go through the hasher in order, so it is read
    // once here rather than re-read after the transfer.
    let mut hashib =
        if mahjuz > 0 { ihshi_juz(&juz, mahjuz).await? } else { Sha256::new() };

    let mut mabni = reqwest::Client::new().get(&madkhal.rabt);
    if mahjuz > 0 {
        mabni = mabni.header(RANGE, format!("bytes={mahjuz}-"));
    }
    let radd = mabni.send().await.map_err(|khata| KhataTahdith::QanatGhayrMutaha {
        sabab: khata.to_string(),
    })?;
    let radd = radd.error_for_status().map_err(|khata| KhataTahdith::QanatGhayrMutaha {
        sabab: khata.to_string(),
    })?;

    // A server that ignored the Range header answers 200 with the whole file,
    // and appending that to a partial would concatenate two prefixes.
    let mustanaf = radd.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    if mahjuz > 0 && !mustanaf {
        return Err(KhataTahdith::IstinafMutaadhdhir {
            rabt: madkhal.rabt.clone(),
            sabab: "the source answered the range request with the whole file".to_owned(),
        });
    }

    let baqi = madkhal.hajm.saturating_sub(mahjuz);
    if radd.content_length().is_some_and(|tul| tul > baqi) {
        return Err(KhataTahdith::HajmMufrit { muallan: madkhal.hajm });
    }

    let mut malaf = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&juz)
        .await
        .map_err(|sabab| KhataTahdith::KhataMalaf {
            masar: juz.clone(),
            amal: "opening the partial download",
            sabab,
        })?;

    let mut maktub = mahjuz;
    let mut radd = radd;
    loop {
        let qita = match tokio::time::timeout(MUHLAT_QITA, radd.chunk()).await {
            Ok(Ok(Some(qita))) => qita,
            Ok(Ok(None)) => break,
            Ok(Err(khata)) => {
                return Err(KhataTahdith::QanatGhayrMutaha { sabab: khata.to_string() });
            }
            Err(_) => {
                return Err(KhataTahdith::QanatGhayrMutaha {
                    sabab: format!("no data for {} seconds", MUHLAT_QITA.as_secs()),
                });
            }
        };
        maktub = maktub.saturating_add(qita.len() as u64);
        if maktub > madkhal.hajm {
            return Err(KhataTahdith::HajmMufrit { muallan: madkhal.hajm });
        }
        hashib.update(&qita);
        malaf.write_all(&qita).await.map_err(|sabab| KhataTahdith::KhataMalaf {
            masar: juz.clone(),
            amal: "writing the partial download",
            sabab,
        })?;
    }

    // Flushed to the device before the rename, so the length on disk is a real
    // resume point and the finished file is not a rename over unwritten pages.
    malaf.flush().await.map_err(|sabab| KhataTahdith::KhataMalaf {
        masar: juz.clone(),
        amal: "flushing the download",
        sabab,
    })?;
    malaf.sync_all().await.map_err(|sabab| KhataTahdith::KhataMalaf {
        masar: juz.clone(),
        amal: "flushing the download to the device",
        sabab,
    })?;
    drop(malaf);

    let mahsuba = hex_min_bayt(&hashib.finalize());
    if !mahsuba.eq_ignore_ascii_case(&madkhal.sha256) {
        let _ = tokio::fs::remove_file(&juz).await;
        return Err(KhataTahdith::TanzeelGhayrMutabiq {
            rabt: madkhal.rabt.clone(),
            muallana: madkhal.sha256.clone(),
            mahsuba,
        });
    }

    tokio::fs::rename(&juz, &hadaf).await.map_err(|sabab| KhataTahdith::KhataMalaf {
        masar: hadaf.clone(),
        amal: "naming the verified download",
        sabab,
    })?;

    Ok(MalafMuhaqqaq { masar: hadaf, sha256: mahsuba, hajm: maktub })
}

/// The partial file's path for a destination.
fn masar_juz(hadaf: &Path) -> PathBuf {
    let mut ism = hadaf.as_os_str().to_os_string();
    ism.push(IMTIDAD_JUZ);
    PathBuf::from(ism)
}

/// The file name a download URL implies, with the path stripped.
///
/// Falls back to a fixed name rather than trusting the last segment when it is
/// empty or would escape the sandbox directory.
fn ism_min_rabt(rabt: &str) -> String {
    let akhir = rabt
        .rsplit('/')
        .next()
        .map(|juz| juz.split(['?', '#']).next().unwrap_or(juz))
        .unwrap_or_default();
    if akhir.is_empty() || akhir.contains("..") || akhir.contains('\\') {
        return "tahdith.tanzeel".to_owned();
    }
    akhir.to_owned()
}

/// Feeds an already-downloaded prefix through a fresh hasher.
async fn ihshi_juz(juz: &Path, mahjuz: u64) -> NatijatTahdith<Sha256> {
    use tokio::io::AsyncReadExt as _;

    let mut malaf = tokio::fs::File::open(juz).await.map_err(|sabab| {
        KhataTahdith::KhataMalaf {
            masar: juz.to_path_buf(),
            amal: "reading the partial download",
            sabab,
        }
    })?;
    let mut hashib = Sha256::new();
    let mut hajiz = vec![0u8; 65_536];
    let mut maqru: u64 = 0;
    while maqru < mahjuz {
        let tul = malaf.read(&mut hajiz).await.map_err(|sabab| KhataTahdith::KhataMalaf {
            masar: juz.to_path_buf(),
            amal: "reading the partial download",
            sabab,
        })?;
        if tul == 0 {
            break;
        }
        let tul = usize::try_from(mahjuz - maqru).map_or(tul, |baqi| tul.min(baqi));
        // `get` rather than a range index: `tul` is clamped twice above and can
        // only be in bounds, but a resumed prefix is attacker-influenced input
        // and a panic inside an update path is the worst place to be right.
        let Some(qitaa) = hajiz.get(..tul) else { break };
        hashib.update(qitaa);
        maqru = maqru.saturating_add(tul as u64);
    }
    Ok(hashib)
}

/// Refuses a transfer the destination's filesystem cannot hold, crediting what
/// a partial file already holds and demanding a tenth of headroom.
///
/// Skipped when no mounted disk matches the destination: a check that cannot be
/// made is not a refusal.
fn akkid_misaha(hadaf: &Path, hajm: u64, mahjuz: u64) -> NatijatTahdith<()> {
    let Some(walid) = hadaf.parent() else { return Ok(()) };
    let Ok(walid) = std::fs::canonicalize(walid) else { return Ok(()) };
    let Some(mutah) = misaha_mutaha(&walid) else { return Ok(()) };

    #[expect(
        clippy::integer_division,
        reason = "a tenth of headroom: the divisor is a constant and truncation is wanted"
    )]
    let hamish = hajm / MUQAM_HAMISH;
    let matlub = hajm.saturating_add(hamish).saturating_sub(mahjuz);
    if mutah >= matlub {
        return Ok(());
    }
    Err(KhataTahdith::MisahaGhayrKafiya { matlub, mutah, masar: hadaf.to_path_buf() })
}

/// Free bytes on the mounted filesystem whose mount point is the longest
/// prefix of `masar`.
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

/// Strips Windows' verbatim prefix, which no mount point carries.
fn bila_badiya_harfiya(masar: &Path) -> PathBuf {
    let Some(nass) = masar.to_str() else { return masar.to_path_buf() };
    if let Some(baqi) = nass.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{baqi}"));
    }
    nass.strip_prefix(r"\\?\").map_or_else(|| masar.to_path_buf(), PathBuf::from)
}

/// Lowercase hex of a digest.
fn hex_min_bayt(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter().fold(String::with_capacity(bayt.len() * 2), |mut nass, bayta| {
        let _ = write!(nass, "{bayta:02x}");
        nass
    })
}
