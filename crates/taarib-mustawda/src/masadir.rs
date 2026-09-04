//! Registry sources: the forge, a CDN mirror, a bundled directory, a LAN share.

use std::path::{Path, PathBuf};
use std::time::Duration;

use taarib_usus::masarat::dakhil;
use tokio::io::AsyncReadExt as _;
use tokio::sync::OnceCell;

use crate::fahras::ADAD_SHARAIH;
use crate::khata::{KhataMustawda, NatijatMustawda};

/// The repository path of the global manifest.
pub const MASAR_BAYAN: &str = "bayan.json";

/// The directory every shard lives in, directly under the repository root.
pub const MUJALLAD_SHARAIH: &str = "sharaih";

/// The extension a shard file carries.
pub const IMTIDAD_SHAREEHA: &str = "json";

/// Longest a single request may take, from the first byte sent to the last byte
/// of the body.
///
/// A manifest is a few kilobytes and a shard a few hundred. Thirty seconds is
/// generous for a slow mobile connection and short enough that a mirror which
/// accepts the connection and then stops talking loses its turn to the next
/// source instead of holding the whole fetch open.
pub const MUHLAT_TALAB: Duration = Duration::from_secs(30);

/// Longest the connection itself may take to establish.
///
/// A source that has not completed a handshake in eight seconds is down, and
/// the chain has another source to try.
pub const MUHLAT_ITTISAL: Duration = Duration::from_secs(8);

/// Largest body accepted from any source, in bytes.
///
/// Roughly sixteen times the largest shard a catalogue of fifty thousand
/// patches produces. The cap is for the source that answers a request for a
/// small JSON document with an endless stream, and it is enforced while the
/// body arrives rather than after, so the bytes are never taken in the first
/// place.
pub const HADD_HAJM_ISTIJABA: u64 = 8 * 1024 * 1024;

/// How many redirect hops a request may follow.
const HADD_TAHWIL: usize = 3;

/// Longest source root accepted.
const HADD_TUL_RABT: usize = 2048;

/// Longest repository path accepted.
const HADD_TUL_MASAR_NISBI: usize = 256;

/// What a local read is doing, for the failure that names it.
const AMAL_QIRA: &str = "reading a local registry source";

/// What a local join is doing, for the failure that names it.
const AMAL_HALL: &str = "resolving a path inside a local registry source";

/// The `User-Agent` every request carries.
fn wakil() -> String {
    format!("Taarib/{} (+https://github.com/cc1a2b/taarib)", taarib_usus::ISDAR)
}

/// The repository path of one shard: `sharaih/{raqm:02x}.json`.
///
/// # Errors
///
/// [`KhataMustawda::ShareehaMajhula`] when `raqm` is not below
/// [`ADAD_SHARAIH`](crate::fahras::ADAD_SHARAIH).
pub fn masar_shareeha(raqm: u16) -> NatijatMustawda<String> {
    Ok(format!("{MUJALLAD_SHARAIH}/{}", ism_shareeha(raqm)?))
}

/// The file name of one shard: `{raqm:02x}.json`.
///
/// # Errors
///
/// As [`masar_shareeha`].
pub fn ism_shareeha(raqm: u16) -> NatijatMustawda<String> {
    if raqm >= ADAD_SHARAIH {
        return Err(KhataMustawda::ShareehaMajhula { raqm });
    }
    Ok(format!("{raqm:02x}.{IMTIDAD_SHAREEHA}"))
}

/// Whether a repository path is one this build will resolve against a source.
#[must_use]
pub fn masar_salih(nisbi: &str) -> bool {
    if nisbi.is_empty() || nisbi.len() > HADD_TUL_MASAR_NISBI {
        return false;
    }
    if nisbi.starts_with('/') || nisbi.ends_with('/') {
        return false;
    }
    // Refusing a leading dot covers `.` and `..` together, and the byte set
    // admits no `\`, `:` or `%` that a later decode could turn back into one.
    nisbi.split('/').all(|juz| {
        !juz.is_empty()
            && !juz.starts_with('.')
            && juz.bytes().all(|q| {
                q.is_ascii_lowercase() || q.is_ascii_digit() || q == b'.' || q == b'-' || q == b'_'
            })
    })
}

/// Where a registry repository is read from.
///
/// All four resolve the same layout — [`MASAR_BAYAN`] at the root and
/// [`masar_shareeha`] beneath it — and none is privileged over another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MasdarMustawda {
    /// The forge's raw-content endpoint.
    Shabaka {
        /// The repository root: an `https` address with no query and no
        /// fragment.
        jidhr: String,
    },

    /// A CDN mirror of the same repository.
    Mira {
        /// The repository root: an `https` address with no query and no
        /// fragment.
        jidhr: String,
    },

    /// A directory holding the repository, shipped with the application.
    MujalladMahalli {
        /// The repository root on disk.
        jidhr: PathBuf,
    },

    /// A directory holding the repository, reached over a mounted share.
    MushtarakShabaki {
        /// The repository root as the platform mounts it.
        jidhr: PathBuf,
    },
}

impl MasdarMustawda {
    /// Whether reaching this source needs the network.
    #[must_use]
    pub const fn shabakiya(&self) -> bool {
        matches!(self, Self::Shabaka { .. } | Self::Mira { .. })
    }

    /// How this source is named in a failure.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Shabaka { jidhr } => format!("forge {jidhr}"),
            Self::Mira { jidhr } => format!("mirror {jidhr}"),
            Self::MujalladMahalli { jidhr } => format!("bundled mirror {}", jidhr.display()),
            Self::MushtarakShabaki { jidhr } => format!("network share {}", jidhr.display()),
        }
    }

    /// Fetches one repository path from this source, `amil` being ignored by
    /// the two local kinds.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::IstijabaFashila`] when a network source answers
    /// outside 2xx, [`KhataMustawda::TanzeelFashil`] when the transfer breaks,
    /// [`KhataMustawda::HajmMufrit`] when the body passes
    /// [`HADD_HAJM_ISTIJABA`], [`KhataMustawda::KhataMalaf`] when a local
    /// source cannot be read, and [`KhataMustawda::LaMasdar`] when the root or
    /// the path is not one this build resolves.
    pub async fn jalb(&self, amil: &reqwest::Client, nisbi: &str) -> NatijatMustawda<Vec<u8>> {
        match self {
            Self::Shabaka { jidhr } | Self::Mira { jidhr } => {
                jalb_shabaki(amil, jidhr, nisbi).await
            }
            Self::MujalladMahalli { jidhr } | Self::MushtarakShabaki { jidhr } => {
                jalb_min_qurs(jidhr, nisbi).await
            }
        }
    }
}

/// The sources to try, in the order they are tried.
#[derive(Debug)]
pub struct SilsilatMasadir {
    masadir: Vec<MasdarMustawda>,
    // Built on first use: a chain of local sources alone never constructs a TLS
    // stack and never resolves a name.
    amil: OnceCell<reqwest::Client>,
}

impl SilsilatMasadir {
    /// Builds a chain that tries `masadir` in the order given.
    #[must_use]
    pub fn jadida(masadir: Vec<MasdarMustawda>) -> Self {
        Self { masadir, amil: OnceCell::new() }
    }

    /// Builds a chain of one source.
    #[must_use]
    pub fn min_masdar(masdar: MasdarMustawda) -> Self {
        Self::jadida(vec![masdar])
    }

    /// The same chain, with the HTTP client supplied rather than built.
    ///
    /// The client is otherwise constructed on first use inside a private cell,
    /// which is right for the product — a chain of local sources never stands
    /// up a TLS stack — and leaves the network path with no seam. The download
    /// side already takes its client as a parameter and is therefore testable
    /// against a local server; the index side was not, so nothing could
    /// exercise a real fetch end to end. This is that seam, and it is the whole
    /// difference between a fetch path with a test and one without.
    ///
    /// Every existing caller is unaffected: they go through [`Self::jadida`],
    /// the cell is still empty, and it still builds the same client on demand.
    #[must_use]
    pub fn bi_amil(masadir: Vec<MasdarMustawda>, amil: reqwest::Client) -> Self {
        let khazina = OnceCell::new();
        // Cannot fail: the cell was created one line above and nothing else can
        // reach it yet, so there is no race and no previous value to reject.
        let _ = khazina.set(amil);
        Self { masadir, amil: khazina }
    }

    /// The sources, in the order they are tried.
    #[must_use]
    pub fn masadir(&self) -> &[MasdarMustawda] {
        &self.masadir
    }

    /// Whether any source in the chain needs the network.
    #[must_use]
    pub fn tahtaj_shabaka(&self) -> bool {
        self.masadir.iter().any(MasdarMustawda::shabakiya)
    }

    /// Fetches one repository path, trying each source until one answers.
    ///
    /// # Errors
    ///
    /// [`KhataMustawda::LaMasdar`] when every source refused, carrying the
    /// reason the last one gave.
    pub async fn jalb(&self, nisbi: &str) -> NatijatMustawda<Vec<u8>> {
        let mut akhir = String::from("no registry source is configured");

        for masdar in &self.masadir {
            let wasf = masdar.wasf();
            let natija = match masdar {
                MasdarMustawda::Shabaka { jidhr } | MasdarMustawda::Mira { jidhr } => {
                    match self.amil_shabaka().await {
                        Ok(amil) => jalb_shabaki(amil, jidhr, nisbi).await,
                        Err(khata) => Err(khata),
                    }
                }
                MasdarMustawda::MujalladMahalli { jidhr }
                | MasdarMustawda::MushtarakShabaki { jidhr } => jalb_min_qurs(jidhr, nisbi).await,
            };

            match natija {
                Ok(bayt) => {
                    tracing::debug!(masdar = %wasf, nisbi, hajm = bayt.len(), "source answered");
                    return Ok(bayt);
                }
                Err(khata) => {
                    tracing::debug!(masdar = %wasf, nisbi, khata = %khata, "source refused");
                    akhir = format!("{wasf}: {khata}");
                }
            }
        }

        Err(KhataMustawda::LaMasdar { sabab: akhir })
    }

    /// The HTTP client, built once and only when a network source is reached.
    async fn amil_shabaka(&self) -> NatijatMustawda<&reqwest::Client> {
        self.amil
            .get_or_try_init(|| async { bina_amil() })
            .await
            .map_err(|khata| KhataMustawda::LaMasdar {
                sabab: format!("no HTTP client could be built: {khata}"),
            })
    }
}

/// Reads a local file, refusing anything that is not a regular file and
/// stopping before [`HADD_HAJM_ISTIJABA`] bytes have been taken.
///
/// # Errors
///
/// [`KhataMustawda::KhataMalaf`] when the path cannot be opened, described or
/// read, or is not a regular file, and [`KhataMustawda::HajmMufrit`] when it is
/// larger than the cap.
pub async fn qira_mahdouda(masar: &Path) -> NatijatMustawda<Vec<u8>> {
    let khata_malaf = |sabab: std::io::Error| KhataMustawda::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: AMAL_QIRA,
        sabab,
    };

    let malaf = tokio::fs::File::open(masar).await.map_err(khata_malaf)?;
    let bayanat = malaf.metadata().await.map_err(khata_malaf)?;

    if !bayanat.is_file() {
        // A character device answers a read forever and reports no length.
        return Err(khata_malaf(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "not a regular file",
        )));
    }
    if bayanat.len() > HADD_HAJM_ISTIJABA {
        return Err(KhataMustawda::HajmMufrit { muallan: HADD_HAJM_ISTIJABA });
    }

    let saa = usize::try_from(bayanat.len()).unwrap_or(0);
    let mut bayt = Vec::with_capacity(saa);
    let mut mahdud = malaf.take(HADD_HAJM_ISTIJABA.saturating_add(1));
    let _ = mahdud.read_to_end(&mut bayt).await.map_err(khata_malaf)?;

    if u64::try_from(bayt.len()).unwrap_or(u64::MAX) > HADD_HAJM_ISTIJABA {
        return Err(KhataMustawda::HajmMufrit { muallan: HADD_HAJM_ISTIJABA });
    }
    Ok(bayt)
}

/// Fetches one repository path over HTTP.
async fn jalb_shabaki(
    amil: &reqwest::Client,
    jidhr: &str,
    nisbi: &str,
) -> NatijatMustawda<Vec<u8>> {
    let rabt = rabt_kamil(jidhr, nisbi)?;
    let nass = rabt.to_string();

    let mut radd = amil.get(rabt).send().await.map_err(|khata| KhataMustawda::TanzeelFashil {
        rabt: nass.clone(),
        sabab: khata.to_string(),
    })?;

    let hala = radd.status();
    if !hala.is_success() {
        return Err(KhataMustawda::IstijabaFashila { rabt: nass, ramz: hala.as_u16() });
    }
    if radd.content_length().is_some_and(|muallan| muallan > HADD_HAJM_ISTIJABA) {
        return Err(KhataMustawda::HajmMufrit { muallan: HADD_HAJM_ISTIJABA });
    }

    let mut bayt: Vec<u8> = Vec::new();
    loop {
        let qita = radd.chunk().await.map_err(|khata| KhataMustawda::TanzeelFashil {
            rabt: nass.clone(),
            sabab: khata.to_string(),
        })?;
        let Some(qita) = qita else { break };

        let majmu = u64::try_from(bayt.len().saturating_add(qita.len())).unwrap_or(u64::MAX);
        if majmu > HADD_HAJM_ISTIJABA {
            return Err(KhataMustawda::HajmMufrit { muallan: HADD_HAJM_ISTIJABA });
        }
        bayt.extend_from_slice(&qita);
    }

    Ok(bayt)
}

/// Reads one repository path from a directory on disk or on a mounted share.
async fn jalb_min_qurs(jidhr: &Path, nisbi: &str) -> NatijatMustawda<Vec<u8>> {
    if !masar_salih(nisbi) {
        return Err(KhataMustawda::LaMasdar {
            sabab: format!("{nisbi:?} is not a repository path this build resolves"),
        });
    }

    let masar = dakhil(jidhr, nisbi).map_err(|khata| KhataMustawda::KhataMalaf {
        masar: jidhr.to_path_buf(),
        amal: AMAL_HALL,
        sabab: std::io::Error::other(khata),
    })?;

    qira_mahdouda(&masar).await
}

/// Joins a repository path onto a network root, refusing a root this build will
/// not open.
fn rabt_kamil(jidhr: &str, nisbi: &str) -> NatijatMustawda<reqwest::Url> {
    if !masar_salih(nisbi) {
        return Err(KhataMustawda::LaMasdar {
            sabab: format!("{nisbi:?} is not a repository path this build resolves"),
        });
    }

    let asas = jidhr.trim_end_matches('/');
    // A root carrying a query or a fragment would put the shard name after it,
    // and the request would ask for a path no repository has.
    let mustaqim = !asas.is_empty()
        && asas.len() <= HADD_TUL_RABT
        && !asas.contains(['?', '#', '\\'])
        && !asas.bytes().any(|q| q.is_ascii_control() || q == b' ');
    if !mustaqim {
        return Err(KhataMustawda::LaMasdar {
            sabab: format!("{jidhr:?} is not a usable registry root"),
        });
    }

    let rabt = reqwest::Url::parse(&format!("{asas}/{nisbi}")).map_err(|khata| {
        KhataMustawda::LaMasdar {
            sabab: format!("{jidhr:?} is not a usable registry root: {khata}"),
        }
    })?;

    if rabt.scheme() != "https" {
        return Err(KhataMustawda::LaMasdar {
            sabab: format!("{jidhr:?} is not https, and a registry is never read in the clear"),
        });
    }

    Ok(rabt)
}

/// Builds the one client every network source shares.
fn bina_amil() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(wakil())
        .timeout(MUHLAT_TALAB)
        .connect_timeout(MUHLAT_ITTISAL)
        .redirect(reqwest::redirect::Policy::limited(HADD_TAHWIL))
        // Closes the redirect downgrade, where an https root answers with a
        // Location pointing at http and only the first address was checked.
        .https_only(true)
        .referer(false)
        .build()
}
