//! Submitting: device authorisation, the contributor's own seal, and the pull request.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use taarib_khatm::khata::KhataKhatm;
use taarib_khatm::{MiftahKhass, MudaqqiqEd25519, mafatih};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{HalatRuqaa, MulakhkhasRuqaa, RuqaaId, RuqaaRevision};
use taarib_mustawda::fahras::{MuhtawaShareeha, shareeha};
use taarib_mustawda::masadir::{MUJALLAD_SHARAIH, ism_shareeha};
use taarib_ruqaa::qari::Ruqaa;
use taarib_ruqaa::tawqee::{DawrMiftah, Khwarizmiya, KutlatTawqee};
use taarib_tarqee::bayan::BayanHuzma;
use taarib_tarqee::mujammi::HuzmaMabniya;
use taarib_tarqee::taghtiya_ruqaa::SababAdamAlnashr;
use taarib_usus::masarat::{self, Masarat};

use crate::bawwaba::IjtiyazTaqdeem;
use crate::hawiya::HawiyatMusahim;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};

/// Longest a single forge request may take, first byte to last.
pub const MUHLAT_TALAB: Duration = Duration::from_secs(30);

/// Longest a connection may take to establish.
pub const MUHLAT_ITTISAL: Duration = Duration::from_secs(10);

/// Longest a package upload may take.
pub const MUHLAT_RAFA: Duration = Duration::from_mins(15);

/// Largest JSON body accepted from the forge, in bytes.
pub const HADD_HAJM_ISTIJABA: u64 = 4 * 1024 * 1024;

/// Largest package this transport will upload, in bytes.
pub const HADD_HAJM_HUZMA: u64 = 512 * 1024 * 1024;

/// Polling interval used when the authorisation server names none.
pub const FASIL_ISTITLA: Duration = Duration::from_secs(5);

/// Shortest interval this client will poll at, whatever the server asks.
pub const ADNA_FASIL: Duration = Duration::from_secs(1);

/// Longest interval this client will wait between polls.
pub const AQSA_FASIL: Duration = Duration::from_mins(1);

/// How much `slow_down` adds to the interval, per RFC 8628.
pub const ZIYADAT_TABTEE: Duration = Duration::from_secs(5);

/// Longest the whole device authorisation may run.
pub const MUHLAT_TAWTHIQ: Duration = Duration::from_mins(15);

/// How many polls the device flow will make before giving up.
pub const HADD_MUHAWALAT: u32 = 600;

/// How many times one forge request is attempted before its failure is final.
///
/// Four — the original try and three retries — to match
/// `taarib_tarjama::muzawwidun::ADAD_MUHAWALAT`: enough to ride out one
/// rate-limit window and a transient gateway failure, few enough that a forge
/// that is genuinely down fails the submission in under a minute.
pub const ADAD_MUHAWALAT_TALAB: u32 = 4;

/// The first backoff step between retries, doubling per attempt.
pub const ASAS_TARAJU: Duration = Duration::from_millis(500);

/// The longest wait this client will accept before it stops retrying.
///
/// Two minutes, as `taarib_tarjama` uses for the same decision. A forge asking
/// for longer is not smoothing a burst, it is saturated, and holding a
/// submission open in silence is worse for the contributor than a refusal that
/// names the moment the window reopens.
pub const AQSA_TARAJU: Duration = Duration::from_mins(2);

/// The shortest wait after a rate-limit refusal that named none itself.
///
/// One minute, which is the forge's own published instruction for a secondary
/// rate limit that carries neither a `retry-after` nor a spent window. Coming
/// back sooner than a forge asked is what turns one refusal into a ban.
pub const ADNA_INTIZAR_MUADAL: Duration = Duration::from_mins(1);

/// How many bytes of the package one upload chunk carries.
///
/// One mebibyte: large enough that a half-gigabyte package is five hundred
/// writes rather than half a million, small enough that a stalled connection is
/// noticed by the request timeout rather than after the whole body is buffered.
pub const HAJM_QITAA_RAFA: usize = 1024 * 1024;

/// How many times the fork lookup is repeated after a fork is created.
pub const HADD_INTIZAR_SHAWKA: u32 = 20;

/// The keychain entry family every stored forge token lives under.
pub const ISM_KHAZNA: &str = "taqdeem.rumooz";

/// Longest access token this client will store, in bytes.
pub const HADD_TUL_RAMZ: usize = 4096;

/// The path segment a staging location must carry.
pub const MASAR_TAJHEEZ: &str = "tajheez";

/// The path segment the public release area carries, and staging must not.
pub const MASAR_ISDAR: &str = "isdar";

/// The path segment a forge's public download area carries, which is never
/// somewhere a package can be uploaded to.
const MASAR_TANZIL: &str = "/releases/download/";

/// The REST surface version every request pins itself to.
///
/// `2026-03-10`, the version `GET https://api.github.com/versions` lists first
/// and every current documentation sample sends. Left unsent the API falls back
/// to `2022-11-28`, whose support ends in March 2028; pinning is what stops a
/// field this transport reads from changing under a build nobody rebuilt.
const ISDAR_API_GITHUB: (&str, &str) = ("X-GitHub-Api-Version", "2026-03-10");

/// The settings field carrying the forge OAuth client identifier.
pub const HAQL_MUARRIF_AMIL: &str = "muarrif_amil";

/// The settings field carrying the staging upload endpoint.
pub const HAQL_RABT_TAJHEEZ: &str = "rabt_tajheez";

/// Values a half-filled configuration leaves behind, which are not identifiers.
///
/// A forge answers a placeholder client id with a generic `unauthorized_client`
/// after a network round trip, and the contributor reads that as "my account is
/// wrong". Catching it here makes the refusal name the setting instead.
const HASHW: &[&str] = &[
    "changeme",
    "client-id",
    "client_id",
    "placeholder",
    "todo",
    "unset",
    "xxx",
    "xxxx",
    "xxxxxxxx",
];

/// The shortest string any forge issues as a client identifier.
const ADNA_TUL_MUARRIF: usize = 8;

/// How many bytes one keychain entry carries.
const HAJM_QITAA: usize = 32;

/// How many redirect hops a request may follow.
const HADD_TAHWIL: usize = 3;

/// How many characters of a refused body are quoted back.
const HADD_IQTIBAS: usize = 512;

/// The local reference a fetched base branch lands on.
const MARJI_ASAS: &str = "refs/heads/taarib-asas";

/// The `User-Agent` every request carries.
fn wakil() -> String {
    format!("Taarib/{} (+https://github.com/cc1a2b/taarib)", taarib_usus::ISDAR)
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// A URL template whose `{name}` placeholders the configuration fills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QalabRabt(String);

impl QalabRabt {
    /// Wraps a template.
    #[must_use]
    pub fn jadeed(nass: impl Into<String>) -> Self {
        Self(nass.into())
    }

    /// The template as written.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// Substitutes every placeholder, refusing a value that is not a path
    /// segment this build will put in a URL.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::BayanNaqis`] when a value carries anything outside the
    /// permitted set, or when a placeholder is left unfilled.
    pub fn mila(&self, qeem: &[(&str, &str)]) -> NatijatTaqdeem<String> {
        let mut rabt = self.0.clone();
        for (miftah, qeema) in qeem {
            if !juz_salih(qeema) {
                return Err(KhataTaqdeem::BayanNaqis {
                    haql: "a repository name made only of letters, digits, dot, dash, \
                           underscore and slash",
                });
            }
            rabt = rabt.replace(&format!("{{{miftah}}}"), qeema);
        }
        if rabt.contains('{') || rabt.contains('}') {
            return Err(KhataTaqdeem::BayanNaqis { haql: "a URL template with every field filled" });
        }
        if !rabt.starts_with("https://") {
            return Err(KhataTaqdeem::BayanNaqis { haql: "an https endpoint" });
        }
        Ok(rabt)
    }
}

/// Whether a value may be substituted into a URL template.
fn juz_salih(nass: &str) -> bool {
    !nass.is_empty()
        && nass.len() <= 128
        && !nass.contains("..")
        && nass.bytes().all(|q| {
            q.is_ascii_alphanumeric() || q == b'-' || q == b'_' || q == b'.' || q == b'/'
        })
}

/// The device authorisation flow's endpoints and client identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdadatTawthiq {
    /// Where a device code is requested.
    pub rabt_ramz_jihaz: String,
    /// Where the device code is exchanged for an access token.
    pub rabt_ramz_wusul: String,
    /// Where the authenticated identity is read back.
    pub qalab_tadqiq: QalabRabt,
    /// This client's identifier at the forge.
    pub muarrif_amil: String,
    /// The scopes the token is asked for.
    pub nitaq: String,
    /// The keychain account name the token is stored under.
    pub hisab: String,
}

impl IdadatTawthiq {
    /// Whether a client identifier has actually been provisioned.
    ///
    /// The device authorisation flow is the one step of the transport that
    /// cannot be built without it: every request it makes carries `client_id`,
    /// and a forge has no way to answer a request that does not name the
    /// application asking. Everything downstream of holding a token — the
    /// identity lookup, the fork, the upload, the pull request — needs the
    /// token and not this, which is why the refusal is a separate question
    /// from whether the rest of the configuration is sound.
    #[must_use]
    pub fn muhayya(&self) -> bool {
        muarrif_muhayya(&self.muarrif_amil)
    }

    /// Checks that nothing required is blank or reachable in the clear.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::IrsalGhayrMuhayya`] when no client identifier has been
    /// provisioned, and [`KhataTaqdeem::BayanNaqis`] naming the first other
    /// field that is wrong.
    pub fn tahaqquq(&self) -> NatijatTaqdeem<()> {
        for (haql, qeema) in [
            ("a device code endpoint", self.rabt_ramz_jihaz.as_str()),
            ("a token endpoint", self.rabt_ramz_wusul.as_str()),
        ] {
            if !qeema.starts_with("https://") {
                return Err(KhataTaqdeem::BayanNaqis { haql });
            }
        }
        if !self.muhayya() {
            return Err(KhataTaqdeem::IrsalGhayrMuhayya { naqis: HAQL_MUARRIF_AMIL });
        }
        if !juz_salih(&self.hisab) {
            return Err(KhataTaqdeem::BayanNaqis { haql: "a keychain account name" });
        }
        Ok(())
    }
}

/// Whether a string is a client identifier a forge could have issued.
///
/// Deliberately permissive about shape — GitHub writes `Ov23li…`, GitLab writes
/// sixty-four hex characters, and a self-hosted forge writes whatever it likes
/// — and strict only about the things no forge ever issues: emptiness,
/// whitespace, an unfilled template, and the words a half-finished settings
/// file is left holding.
fn muarrif_muhayya(nass: &str) -> bool {
    let mahdhuf = nass.trim();
    if mahdhuf.len() < ADNA_TUL_MUARRIF {
        return false;
    }
    if mahdhuf.chars().any(|harf| harf.is_whitespace() || harf.is_control()) {
        return false;
    }
    if mahdhuf.contains(['{', '}', '<', '>']) {
        return false;
    }
    let saghir = mahdhuf.to_ascii_lowercase();
    if HASHW.contains(&saghir.as_str()) || saghir.starts_with("your") {
        return false;
    }
    // A run of one repeated character is a mask somebody typed, not an id.
    !saghir.bytes().all(|harf| harf == saghir.as_bytes().first().copied().unwrap_or(0))
}

/// Whether an upload endpoint is one a package may be sent to.
///
/// The endpoint has to name the staging area and must not name the public
/// release area, which is the rule `docs/mustawda.md` §7.2 states and which a
/// GitHub configuration satisfies through the asset name rather than the path:
/// the upload address carries a numeric release identifier, so the marker goes
/// in the query as `?name=tajheez-<file>` and comes back in the asset's own
/// download URL.
///
/// The third rule is new and is about a mistake rather than a policy: a release
/// *download* address is not writable at all, and an operator who pastes one
/// into the setting should be told so before a package is built rather than
/// after a forge answers `404`.
///
/// The binding guarantee is the one applied to the address the forge answers
/// with, in [`irfa_ila_tajheez`]: by then the forge has said where the bytes
/// landed rather than where they were sent.
fn qabul_tajheez(rabt: &str) -> bool {
    !rabt.contains(MASAR_ISDAR) && !rabt.contains(MASAR_TANZIL) && rabt.contains(MASAR_TAJHEEZ)
}

/// Where the registry repository, its API and its staging area live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdadatMustawda {
    /// The upstream repository's owner.
    pub malik: String,
    /// The upstream repository's name.
    pub mustawda: String,
    /// The branch a submission is opened against.
    pub far_asasi: String,
    /// The prefix every submission branch carries.
    pub bidayat_far: String,
    /// The upstream repository's https clone address.
    pub rabt_git: String,
    /// A fork's https clone address, `{malik_shawka}` and `{mustawda}`.
    pub qalab_git_shawka: QalabRabt,
    /// Where an existing fork is looked up, `{malik_shawka}` and `{mustawda}`.
    pub qalab_shawkati: QalabRabt,
    /// Where a fork is created, `{malik}` and `{mustawda}`.
    pub qalab_shawka: QalabRabt,
    /// Where a package is uploaded, `{ism}`. Staging only.
    pub qalab_tajheez: QalabRabt,
    /// Where a pull request is opened, `{malik}` and `{mustawda}`.
    pub qalab_talab_damj: QalabRabt,
}

impl IdadatMustawda {
    /// Whether a staging upload endpoint has actually been provisioned.
    #[must_use]
    pub fn muhayya(&self) -> bool {
        !self.qalab_tajheez.nass().trim().is_empty()
    }

    /// Checks the names, the clone address, and the staging separation.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::IrsalGhayrMuhayya`] when no staging endpoint has been
    /// provisioned, and [`KhataTaqdeem::BayanNaqis`] naming the first other
    /// field that is wrong, including an upload endpoint this build will not
    /// recognise as staging or that names the public release area.
    pub fn tahaqquq(&self) -> NatijatTaqdeem<()> {
        for (haql, qeema) in [
            ("a repository owner", self.malik.as_str()),
            ("a repository name", self.mustawda.as_str()),
            ("a base branch", self.far_asasi.as_str()),
            ("a branch prefix", self.bidayat_far.as_str()),
        ] {
            if !juz_salih(qeema) {
                return Err(KhataTaqdeem::BayanNaqis { haql });
            }
        }
        if !self.rabt_git.starts_with("https://") {
            return Err(KhataTaqdeem::BayanNaqis { haql: "an https clone address" });
        }
        if !self.muhayya() {
            return Err(KhataTaqdeem::IrsalGhayrMuhayya { naqis: HAQL_RABT_TAJHEEZ });
        }
        if !qabul_tajheez(self.qalab_tajheez.nass()) {
            return Err(KhataTaqdeem::BayanNaqis {
                haql: "an upload endpoint that names the staging area, as a release-asset \
                       address ending `?name=tajheez-<file>` does",
            });
        }
        Ok(())
    }
}

/// Everything the transport is configured with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdadatIrsal {
    /// The device authorisation endpoints.
    pub tawthiq: IdadatTawthiq,
    /// The repository, its API and its staging area.
    pub mustawda: IdadatMustawda,
}

impl IdadatIrsal {
    /// Whether both halves of the channel have been provisioned.
    ///
    /// A caller that wants to say "submission is not set up yet" before doing
    /// any work asks this; a caller that wants to know *which* piece is missing
    /// calls [`Self::tahaqquq`] and reads the refusal.
    #[must_use]
    pub fn muhayya(&self) -> bool {
        self.tawthiq.muhayya() && self.mustawda.muhayya()
    }

    /// Checks both halves.
    ///
    /// # Errors
    ///
    /// As [`IdadatTawthiq::tahaqquq`] and [`IdadatMustawda::tahaqquq`].
    pub fn tahaqquq(&self) -> NatijatTaqdeem<()> {
        self.tawthiq.tahaqquq()?;
        self.mustawda.tahaqquq()
    }
}

// ---------------------------------------------------------------------------
// The token, and where it is kept
// ---------------------------------------------------------------------------

/// A forge access token.
#[derive(Clone)]
pub struct RamzWusul {
    naw: String,
    sirr: String,
}

impl std::fmt::Debug for RamzWusul {
    /// Prints the scheme and never the secret.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RamzWusul").field("naw", &self.naw).finish_non_exhaustive()
    }
}

impl RamzWusul {
    /// Wraps a token the authorisation server returned.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::TawthiqFashil`] when the token is blank, longer than
    /// [`HADD_TUL_RAMZ`], or carries a control character.
    pub fn jadeed(sirr: impl Into<String>, naw: Option<String>) -> NatijatTaqdeem<Self> {
        let sirr = sirr.into();
        let naw = naw.unwrap_or_else(|| "Bearer".to_owned());
        let salih = !sirr.trim().is_empty()
            && sirr.len() <= HADD_TUL_RAMZ
            && !sirr.chars().any(char::is_control)
            && !naw.is_empty()
            && naw.len() <= 32
            && naw.bytes().all(|q| q.is_ascii_alphanumeric() || q == b'-');
        if !salih {
            return Err(KhataTaqdeem::TawthiqFashil {
                sabab: "the authorisation server returned a token this client will not store"
                    .to_owned(),
            });
        }
        Ok(Self { naw, sirr })
    }

    /// The `Authorization` header value.
    #[must_use]
    pub fn tarwisa(&self) -> String {
        format!("{} {}", self.naw, self.sirr)
    }

    /// The secret itself, for the git credential callback.
    #[must_use]
    pub fn sirr(&self) -> &str {
        &self.sirr
    }
}

/// The keychain entry name for one segment of a stored token.
fn ism_qitaa(hisab: &str, fahras: usize) -> String {
    format!("{ISM_KHAZNA}/{hisab}/{fahras}")
}

/// The keychain entry name for a stored token's header.
fn ism_tarwisa(hisab: &str) -> String {
    format!("{ISM_KHAZNA}/{hisab}/tarwisa")
}

/// One 32-byte slice of a buffer, zero padded.
fn qitaa(bayt: &[u8], fahras: usize) -> [u8; HAJM_QITAA] {
    let mut qita = [0u8; HAJM_QITAA];
    let bidaya = fahras.saturating_mul(HAJM_QITAA);
    let nihaya = bidaya.saturating_add(HAJM_QITAA).min(bayt.len());
    if bidaya < nihaya
        && let Some(masdar) = bayt.get(bidaya..nihaya)
        && let Some(hadaf) = qita.get_mut(..masdar.len())
    {
        hadaf.copy_from_slice(masdar);
    }
    qita
}

/// The keychain failure, as this crate's.
fn khata_khazna(khata: &KhataKhatm) -> KhataTaqdeem {
    KhataTaqdeem::TawthiqFashil { sabab: format!("the OS keychain refused: {khata}") }
}

/// Stores a token in the OS keychain and nowhere else.
///
/// # Errors
///
/// [`KhataTaqdeem::TawthiqFashil`] when the token is longer than
/// [`HADD_TUL_RAMZ`] or the keychain refuses a write.
pub fn khzin_ramz(hisab: &str, ramz: &RamzWusul) -> NatijatTaqdeem<()> {
    let mahtawa = format!("{}\u{1f}{}", ramz.naw, ramz.sirr);
    let bayt = mahtawa.as_bytes();
    if bayt.len() > HADD_TUL_RAMZ {
        return Err(KhataTaqdeem::TawthiqFashil {
            sabab: "the token is longer than this client will store".to_owned(),
        });
    }

    let adad = bayt.len().div_ceil(HAJM_QITAA);
    let mut tarwisa = [0u8; HAJM_QITAA];
    let tul = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
    if let Some(khana) = tarwisa.get_mut(..8) {
        khana.copy_from_slice(&tul.to_le_bytes());
    }

    for fahras in 0..adad {
        let qita = qitaa(bayt, fahras);
        mafatih::khzin(&ism_qitaa(hisab, fahras), &MiftahKhass::min_bayt(&qita))
            .map_err(|khata| khata_khazna(&khata))?;
    }
    mafatih::khzin(&ism_tarwisa(hisab), &MiftahKhass::min_bayt(&tarwisa))
        .map_err(|khata| khata_khazna(&khata))?;
    tracing::debug!(hisab, adad, "the forge token was stored in the OS keychain");
    Ok(())
}

/// Reads a stored token back, or [`None`] when none is stored.
///
/// # Errors
///
/// [`KhataTaqdeem::TawthiqFashil`] when the keychain refuses a read or the
/// stored material is not a token this client wrote.
pub fn hat_ramz(hisab: &str) -> NatijatTaqdeem<Option<RamzWusul>> {
    let tarwisa = match mafatih::hat(&ism_tarwisa(hisab)) {
        Ok(miftah) => miftah.bayt(),
        Err(KhataKhatm::MiftahMafqud { .. }) => return Ok(None),
        Err(khata) => return Err(khata_khazna(&khata)),
    };
    let Some(khana) = tarwisa.first_chunk::<8>() else {
        return Err(KhataTaqdeem::TawthiqFashil {
            sabab: "the stored token header is not the size this client writes".to_owned(),
        });
    };
    let tul = usize::try_from(u64::from_le_bytes(*khana)).unwrap_or(usize::MAX);
    if tul == 0 || tul > HADD_TUL_RAMZ {
        return Err(KhataTaqdeem::TawthiqFashil {
            sabab: "the stored token header names a length this client did not write".to_owned(),
        });
    }

    let mut bayt: Vec<u8> = Vec::with_capacity(tul);
    for fahras in 0..tul.div_ceil(HAJM_QITAA) {
        match mafatih::hat(&ism_qitaa(hisab, fahras)) {
            Ok(miftah) => bayt.extend_from_slice(&miftah.bayt()),
            Err(KhataKhatm::MiftahMafqud { .. }) => {
                return Err(KhataTaqdeem::TawthiqFashil {
                    sabab: "the stored token is missing a segment and cannot be reassembled"
                        .to_owned(),
                });
            }
            Err(khata) => return Err(khata_khazna(&khata)),
        }
    }
    bayt.truncate(tul);

    let mahtawa = String::from_utf8(bayt).map_err(|_| KhataTaqdeem::TawthiqFashil {
        sabab: "the stored token is not text this client wrote".to_owned(),
    })?;
    let (naw, sirr) = mahtawa.split_once('\u{1f}').ok_or_else(|| KhataTaqdeem::TawthiqFashil {
        sabab: "the stored token carries no scheme".to_owned(),
    })?;
    RamzWusul::jadeed(sirr, Some(naw.to_owned())).map(Some)
}

/// Deletes a stored token, treating an absent one as success.
///
/// # Errors
///
/// [`KhataTaqdeem::TawthiqFashil`] when the keychain refuses a deletion.
pub fn imsah_ramz(hisab: &str) -> NatijatTaqdeem<()> {
    mafatih::imsah(&ism_tarwisa(hisab)).map_err(|khata| khata_khazna(&khata))?;
    for fahras in 0..HADD_TUL_RAMZ.div_ceil(HAJM_QITAA) {
        mafatih::imsah(&ism_qitaa(hisab, fahras)).map_err(|khata| khata_khazna(&khata))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The device authorisation flow
// ---------------------------------------------------------------------------

/// What the authorisation server told the user to do.
#[derive(Clone)]
pub struct TalabJihaz {
    ramz_jihaz: String,
    /// The short code the user types.
    pub ramz_mustakhdim: String,
    /// The address the user opens.
    pub rabt_tahaqquq: String,
    /// The address with the code already in it, when the server offers one.
    pub rabt_kamil: Option<String>,
    /// How long the device code stays valid.
    pub yantahi_baad: Duration,
    /// How often the server asked to be polled.
    pub fasil: Duration,
}

impl std::fmt::Debug for TalabJihaz {
    /// Prints what the user is shown and never the device code.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabJihaz")
            .field("ramz_mustakhdim", &self.ramz_mustakhdim)
            .field("rabt_tahaqquq", &self.rabt_tahaqquq)
            .field("yantahi_baad", &self.yantahi_baad)
            .field("fasil", &self.fasil)
            .finish_non_exhaustive()
    }
}

/// How the short code and the verification address reach the user.
///
/// `Sync` is part of the contract, not decoration: [`wathiq`] shows the code
/// between two awaits, so `&dyn ArdRamz` is alive across an await point, and a
/// shared reference is only `Send` when its referent is `Sync`. Without it the
/// whole submission future stops being `Send` and cannot be spawned onto the
/// runtime that drives the commands.
pub trait ArdRamz: std::fmt::Debug + Sync {
    /// Shows one device authorisation request.
    fn ard(&self, talab: &TalabJihaz);
}

/// The presenter a headless caller gets: the code goes to the log.
#[derive(Debug, Clone, Copy, Default)]
pub struct ArdBilTatabbu;

impl ArdRamz for ArdBilTatabbu {
    fn ard(&self, talab: &TalabJihaz) {
        tracing::info!(
            ramz = %talab.ramz_mustakhdim,
            rabt = %talab.rabt_tahaqquq,
            "open the address and enter the code to authorise this device"
        );
    }
}

/// Where one poll of the token endpoint left the flow.
#[derive(Debug)]
enum HalatIstitla {
    Ramz(Box<RamzWusul>),
    Muntazir,
    Abti,
    Muntahi,
    Marfud(String),
}

/// The first of several candidate fields that carries anything.
///
/// Every forge answer below is read this way rather than with `serde(alias)`.
/// An alias makes two spellings of one field, and a body carrying both — which
/// is what GitHub sends, and what RFC 8628 servers send for the verification
/// address — is a duplicate field, so the whole response fails to parse. Listing
/// the spellings as separate optional fields and choosing between them here
/// makes both the "one of them" and the "both of them" case decidable, and the
/// order of the slice is the order of preference.
fn awwal_ghayr_farigh<'nass>(mureshshahat: &[Option<&'nass String>]) -> Option<&'nass str> {
    mureshshahat
        .iter()
        .copied()
        .flatten()
        .map(String::as_str)
        .find(|nass| !nass.trim().is_empty())
}

#[derive(Debug, Deserialize)]
struct RaddJihaz {
    #[serde(rename = "device_code")]
    ramz_jihaz: String,
    #[serde(rename = "user_code")]
    ramz_mustakhdim: String,
    /// RFC 8628 spells it `verification_uri`; some servers send `verification_url`
    /// as well, and at least one sends both.
    #[serde(default, rename = "verification_uri")]
    rabt_uri: Option<String>,
    #[serde(default, rename = "verification_url")]
    rabt_url: Option<String>,
    #[serde(default, rename = "verification_uri_complete")]
    rabt_kamil: Option<String>,
    #[serde(default, rename = "expires_in")]
    yantahi: Option<u64>,
    #[serde(default, rename = "interval")]
    fasil: Option<u64>,
}

impl RaddJihaz {
    /// The address the user opens, whichever spelling the server used.
    fn rabt(&self) -> Option<&str> {
        awwal_ghayr_farigh(&[self.rabt_uri.as_ref(), self.rabt_url.as_ref()])
    }
}

#[derive(Debug, Deserialize)]
struct RaddRamz {
    #[serde(default, rename = "access_token")]
    ramz: Option<String>,
    #[serde(default, rename = "token_type")]
    naw: Option<String>,
    #[serde(default, rename = "error")]
    khata: Option<String>,
    #[serde(default, rename = "error_description")]
    wasf: Option<String>,
    #[serde(default, rename = "interval")]
    fasil: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct RaddHawiya {
    /// GitHub calls the account name `login`, GitLab calls it `username`.
    #[serde(default)]
    login: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

impl RaddHawiya {
    /// The account name, whichever field the forge put it in.
    fn ism(&self) -> Option<&str> {
        awwal_ghayr_farigh(&[self.login.as_ref(), self.username.as_ref()])
    }
}

#[derive(Debug, Deserialize)]
struct RaddRafa {
    /// GitHub's public address for a release asset. Preferred, because it is
    /// the one a later download actually resolves and the one that carries the
    /// release tag, which is what proves the bytes landed in staging.
    #[serde(default)]
    browser_download_url: Option<String>,
    /// GitLab's generic-package address.
    #[serde(default)]
    direct_asset_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    /// The API handle for the asset. Last, because GitHub sends it alongside
    /// `browser_download_url` on every successful upload and it names
    /// `api.github.com`, which is not where the package can be fetched from.
    #[serde(default)]
    url: Option<String>,
}

impl RaddRafa {
    /// Where the forge says the package landed.
    fn rabt(&self) -> Option<&str> {
        awwal_ghayr_farigh(&[
            self.browser_download_url.as_ref(),
            self.direct_asset_url.as_ref(),
            self.download_url.as_ref(),
            self.url.as_ref(),
        ])
    }
}

#[derive(Debug, Deserialize)]
struct RaddTalabDamj {
    /// GitHub's browser address for the pull request.
    #[serde(default)]
    html_url: Option<String>,
    /// GitLab's browser address for the merge request.
    #[serde(default)]
    web_url: Option<String>,
    /// The API handle, which GitHub sends beside `html_url`; it is the address
    /// of the resource, not the page a reviewer opens.
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    number: Option<u64>,
    #[serde(default)]
    iid: Option<u64>,
}

impl RaddTalabDamj {
    /// The address a person opens to read the request.
    fn rabt(&self) -> Option<&str> {
        awwal_ghayr_farigh(&[self.html_url.as_ref(), self.web_url.as_ref(), self.url.as_ref()])
    }

    /// The request's number in its repository, when the forge gives one.
    const fn raqm(&self) -> Option<u64> {
        match self.number {
            Some(raqm) => Some(raqm),
            None => self.iid,
        }
    }
}

/// Requests a device code and returns what the user must be shown.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the endpoint answers outside 2xx or
/// with a body this build cannot read.
pub async fn ibda_tawthiq(
    amil: &reqwest::Client,
    idadat: &IdadatTawthiq,
) -> NatijatTaqdeem<TalabJihaz> {
    idadat.tahaqquq()?;
    let amal = "requesting a device code";
    let radd: RaddJihaz = jalb_json(
        || {
            amil.post(&idadat.rabt_ramz_jihaz)
                .header(reqwest::header::ACCEPT, "application/json")
                .form(&[
                    ("client_id", idadat.muarrif_amil.as_str()),
                    ("scope", idadat.nitaq.as_str()),
                ])
        },
        // Asking for a device code has no effect the user can see beyond
        // issuing one, so an unanswered attempt is safe to make again.
        SiyasatItada::Aid,
        amal,
    )
    .await?;

    let Some(rabt) = radd.rabt() else {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the authorisation server named no verification address".to_owned(),
        });
    };
    if !rabt.starts_with("https://") {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the authorisation server named a verification address that is not https"
                .to_owned(),
        });
    }
    let rabt = rabt.to_owned();

    Ok(TalabJihaz {
        ramz_jihaz: radd.ramz_jihaz,
        ramz_mustakhdim: radd.ramz_mustakhdim,
        rabt_tahaqquq: rabt,
        rabt_kamil: radd.rabt_kamil,
        yantahi_baad: radd
            .yantahi
            .map_or(MUHLAT_TAWTHIQ, |thawani| Duration::from_secs(thawani).min(MUHLAT_TAWTHIQ)),
        fasil: radd.fasil.map_or(FASIL_ISTITLA, Duration::from_secs).clamp(ADNA_FASIL, AQSA_FASIL),
    })
}

/// Polls the token endpoint until the user approves, refuses, or runs out of
/// time.
///
/// # Errors
///
/// [`KhataTaqdeem::TawthiqFashil`] when the device code expired, the user
/// refused, or the flow ran past its own ceiling, and
/// [`KhataTaqdeem::MustawdaRafad`] when the endpoint answered with something
/// this build cannot read.
pub async fn akmil_tawthiq(
    amil: &reqwest::Client,
    idadat: &IdadatTawthiq,
    talab: &TalabJihaz,
) -> NatijatTaqdeem<RamzWusul> {
    let mut fasil = talab.fasil;
    let nihaya = Instant::now().checked_add(talab.yantahi_baad);

    for _ in 0..HADD_MUHAWALAT {
        tokio::time::sleep(fasil).await;
        if nihaya.is_some_and(|hadd| Instant::now() >= hadd) {
            return Err(KhataTaqdeem::TawthiqFashil {
                sabab: "the device code expired before it was approved".to_owned(),
            });
        }

        match istitla(amil, idadat, talab).await? {
            HalatIstitla::Ramz(ramz) => return Ok(*ramz),
            HalatIstitla::Muntazir => {}
            HalatIstitla::Abti => {
                fasil = fasil.saturating_add(ZIYADAT_TABTEE).min(AQSA_FASIL);
                tracing::debug!(thawani = fasil.as_secs(), "the server asked for a slower poll");
            }
            HalatIstitla::Muntahi => {
                return Err(KhataTaqdeem::TawthiqFashil {
                    sabab: "the device code expired before it was approved".to_owned(),
                });
            }
            HalatIstitla::Marfud(sabab) => {
                return Err(KhataTaqdeem::TawthiqFashil { sabab });
            }
        }
    }

    Err(KhataTaqdeem::TawthiqFashil {
        sabab: "the device authorisation was polled to its ceiling without an answer".to_owned(),
    })
}

/// What one `error` value from the token endpoint means for the flow.
///
/// The status line is not consulted, and cannot be: GitHub answers a pending
/// authorisation with `200` and an error body, where RFC 8628 says `400`, and
/// answers an application whose device flow was never enabled with `400`. A
/// client that branched on the status would read a waiting user as a failure on
/// one forge and a misconfigured application as a waiting user on the other.
fn hala_min_khata(khata: &str, wasf: Option<&str>) -> HalatIstitla {
    let wasf = wasf.filter(|nass| !nass.trim().is_empty()).unwrap_or(khata);
    match khata {
        "authorization_pending" => HalatIstitla::Muntazir,
        "slow_down" => HalatIstitla::Abti,
        // Both spellings: the forge's own error table keys the row
        // `expired_token` and describes it in prose as `token_expired`, on
        // every page that carries it. Reading only one of them would turn an
        // expiry into the generic "the server said ..." refusal.
        "expired_token" | "token_expired" => HalatIstitla::Muntahi,
        "access_denied" => HalatIstitla::Marfud(format!("the request was refused: {wasf}")),
        // The answers that mean the application was never finished being
        // registered rather than that the person did anything wrong. Leaving
        // these to the generic branch would send a contributor to re-enter a
        // short code that was never going to work.
        "device_flow_disabled" => HalatIstitla::Marfud(
            "the registered application does not have the device flow enabled; the registry \
             operator has to turn it on in the application's settings"
                .to_owned(),
        ),
        "incorrect_client_credentials" | "unauthorized_client" => HalatIstitla::Marfud(
            "the forge does not recognise the client identifier this build was configured \
             with; the registry operator has to correct it"
                .to_owned(),
        ),
        "unsupported_grant_type" => HalatIstitla::Marfud(
            "the authorisation server does not accept the device grant at this endpoint"
                .to_owned(),
        ),
        "incorrect_device_code" => HalatIstitla::Marfud(
            "the authorisation server no longer recognises this device code; start the \
             authorisation again"
                .to_owned(),
        ),
        _ => HalatIstitla::Marfud(format!("the authorisation server said {khata}: {wasf}")),
    }
}

/// One poll of the token endpoint.
async fn istitla(
    amil: &reqwest::Client,
    idadat: &IdadatTawthiq,
    talab: &TalabJihaz,
) -> NatijatTaqdeem<HalatIstitla> {
    let amal = "exchanging the device code for a token";
    // The pending and slow-down answers arrive as HTTP 400 with a body that
    // names them, so the body is read before the status is judged. Polling is
    // by definition repeatable, so a lost attempt is simply made again.
    let radd_khaam = arsil(
        || {
            amil.post(&idadat.rabt_ramz_wusul)
                .header(reqwest::header::ACCEPT, "application/json")
                .form(&[
                    ("client_id", idadat.muarrif_amil.as_str()),
                    ("device_code", talab.ramz_jihaz.as_str()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ])
        },
        SiyasatItada::Aid,
        HADD_HAJM_ISTIJABA,
        amal,
    )
    .await?;
    let (hala, bayt) = (radd_khaam.hala, radd_khaam.bayt);
    let Ok(radd) = serde_json::from_slice::<RaddRamz>(&bayt) else {
        return Err(KhataTaqdeem::MustawdaRafad { amal, sabab: iqtibas(hala, &bayt) });
    };

    if let Some(khata) = radd.khata.as_deref() {
        return Ok(hala_min_khata(khata, radd.wasf.as_deref()));
    }
    // A server that raises the interval alongside a pending answer is honoured
    // even when it does not send `slow_down`.
    if radd.ramz.is_none() {
        if radd.fasil.is_some() {
            return Ok(HalatIstitla::Abti);
        }
        if !hala.is_success() {
            return Err(KhataTaqdeem::MustawdaRafad { amal, sabab: iqtibas(hala, &bayt) });
        }
        return Ok(HalatIstitla::Muntazir);
    }

    let sirr = radd.ramz.unwrap_or_default();
    RamzWusul::jadeed(sirr, radd.naw).map(|ramz| HalatIstitla::Ramz(Box::new(ramz)))
}

/// Runs the whole device authorisation and stores the token in the keychain.
///
/// # Errors
///
/// As [`ibda_tawthiq`] and [`akmil_tawthiq`], plus
/// [`KhataTaqdeem::TawthiqFashil`] when the keychain refuses the write.
pub async fn wathiq(idadat: &IdadatTawthiq, arid: &dyn ArdRamz) -> NatijatTaqdeem<RamzWusul> {
    let amil = bina_amil(MUHLAT_TALAB)?;
    let talab = ibda_tawthiq(&amil, idadat).await?;
    arid.ard(&talab);
    let ramz = akmil_tawthiq(&amil, idadat, &talab).await?;
    khzin_ramz(&idadat.hisab, &ramz)?;
    Ok(ramz)
}

/// The stored token, or a fresh device authorisation when none is stored.
///
/// # Errors
///
/// As [`hat_ramz`] and [`wathiq`].
pub async fn ramz_mukhazzan_aw_tawthiq(
    idadat: &IdadatTawthiq,
    arid: &dyn ArdRamz,
) -> NatijatTaqdeem<RamzWusul> {
    match hat_ramz(&idadat.hisab)? {
        Some(ramz) => Ok(ramz),
        None => wathiq(idadat, arid).await,
    }
}

// ---------------------------------------------------------------------------
// HTTP
// ---------------------------------------------------------------------------

/// Builds a client with a timeout, a redirect ceiling and no cleartext.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when no TLS stack can be built.
pub fn bina_amil(muhla: Duration) -> NatijatTaqdeem<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(wakil())
        .timeout(muhla)
        .connect_timeout(MUHLAT_ITTISAL)
        .redirect(reqwest::redirect::Policy::limited(HADD_TAHWIL))
        .https_only(true)
        .referer(false)
        .build()
        .map_err(|khata| KhataTaqdeem::MustawdaRafad {
            amal: "building the HTTP client",
            sabab: khata.to_string(),
        })
}

/// The headers every authenticated forge API call carries.
///
/// `X-GitHub-Api-Version` pins the REST surface so a future default cannot
/// quietly change a field this transport reads; every other forge ignores a
/// header it does not know, which is why it is sent unconditionally rather than
/// switched on a guess about who is answering.
fn tarwisat_api(talab: reqwest::RequestBuilder, tarwisa: &str) -> reqwest::RequestBuilder {
    talab
        .header(reqwest::header::AUTHORIZATION, tarwisa)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(ISDAR_API_GITHUB.0, ISDAR_API_GITHUB.1)
}

/// A refused response, quoted back with its body bounded.
fn iqtibas(hala: reqwest::StatusCode, bayt: &[u8]) -> String {
    let nass = String::from_utf8_lossy(bayt);
    let mukhtasar: String = nass.chars().take(HADD_IQTIBAS).collect();
    format!("HTTP {}: {}", hala.as_u16(), mukhtasar.trim())
}

/// Whether the same request may be put on the wire a second time.
///
/// Not a knob: the two values are two different questions about the endpoint,
/// and getting them the wrong way round either duplicates a pull request or
/// abandons a submission over one gateway hiccup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiyasatItada {
    /// The request may be repeated. Reads, and the writes a forge defines as
    /// idempotent — asking for a fork of a repository already forked returns
    /// the existing fork rather than making a second one.
    Aid,
    /// The request may be repeated only when the forge refused it *before*
    /// acting on it, which is what a rate-limit refusal is and what a gateway
    /// failure is not. A `502` from a pull-request call may mean the request
    /// went through and the answer was lost; sending it again would open a
    /// second request against the owner's queue, so it is not sent again.
    HaddFaqat,
}

/// What came back, with the parts of the envelope a retry decision needs.
#[derive(Debug)]
pub struct RaddMahdud {
    /// The status line.
    pub hala: reqwest::StatusCode,
    /// The body, never larger than the ceiling the caller named.
    pub bayt: Vec<u8>,
    /// `x-ratelimit-remaining`, when the forge sent one.
    pub baqi: Option<u64>,
    /// `x-ratelimit-reset`, as seconds since the epoch, when the forge sent one.
    pub istiada: Option<i64>,
    /// `retry-after`, in whole seconds, when the forge sent one.
    pub baad: Option<u64>,
}

impl RaddMahdud {
    /// Whether the status is one the caller asked for.
    #[must_use]
    pub fn najah(&self) -> bool {
        self.hala.is_success()
    }

    /// Whether this refusal is the forge saying the request was not processed
    /// because a limit was reached.
    ///
    /// `429` is the plain form. GitHub also answers a primary rate limit with
    /// `403` and `x-ratelimit-remaining: 0`, and a secondary rate limit with
    /// `403` and a `retry-after`; a `403` with neither is a permissions refusal
    /// and repeating it would only spend another request to be told the same
    /// thing.
    #[must_use]
    pub fn hadd_muadal(&self) -> bool {
        if self.hala == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return true;
        }
        self.hala == reqwest::StatusCode::FORBIDDEN
            && (self.baad.is_some() || self.baqi == Some(0))
    }

    /// Whether this refusal is transient in a way that says nothing about
    /// whether the request was acted on.
    #[must_use]
    pub const fn aarid(&self) -> bool {
        matches!(self.hala.as_u16(), 408 | 500 | 502 | 503 | 504)
    }

    /// How long to wait before sending the request again, when it may be sent
    /// again at all.
    ///
    /// The forge's own `retry-after` wins whenever it is longer than the
    /// schedule — waiting less than a server asked is how one rate limit
    /// becomes a ban — and a primary rate limit with no `retry-after` waits
    /// until its window resets. Anything longer than [`AQSA_TARAJU`] is not
    /// waited out: the caller is told when the window reopens instead of being
    /// held silent for it.
    fn intizar(&self, muhawala: u32, alan: i64) -> Option<Duration> {
        let jadwal = taraju(muhawala);
        // The reset header rides on every answer, spent window or not, so it
        // only names a wait when the window is actually spent. Reading it off a
        // gateway failure would park the submission until the top of the hour.
        let hatta =
            (self.baqi == Some(0)).then(|| self.istiada.map(|hadd| thawani_hatta(hadd, alan)));
        let matlub = self.baad.map(Duration::from_secs).or_else(|| hatta.flatten());
        // A rate-limit refusal that names no wait at all is the secondary
        // limiter, for which the forge's own guidance is a minute before trying
        // again — the exponential schedule alone would come back in half a
        // second and earn a ban.
        let ardiya = if matlub.is_none() && self.hadd_muadal() {
            ADNA_INTIZAR_MUADAL
        } else {
            Duration::ZERO
        };
        let mudda = matlub.map_or(jadwal, |talab| talab.max(jadwal)).max(ardiya);
        (mudda <= AQSA_TARAJU).then_some(mudda)
    }
}

/// The backoff for one attempt: doubling from [`ASAS_TARAJU`], capped.
///
/// No jitter. Jitter spreads a fleet of clients off one another, and this is
/// one desktop application making one submission; adding a random source to buy
/// nothing would be a dependency and a non-reproducible test for no gain.
fn taraju(muhawala: u32) -> Duration {
    // The shift is bounded only so it cannot overflow; eight doublings put
    // 500ms past 120s, so the ceiling below is what actually caps the wait and
    // not this. Across the four attempts a request is given, the schedule never
    // gets past four seconds anyway.
    ASAS_TARAJU.saturating_mul(1_u32 << muhawala.min(8)).min(AQSA_TARAJU)
}

/// How long until an absolute epoch second, never negative.
fn thawani_hatta(hadd: i64, alan: i64) -> Duration {
    Duration::from_secs(u64::try_from(hadd.saturating_sub(alan)).unwrap_or(0))
}

/// A response header as trimmed text, when present and readable.
fn qeemat_ras<'radd>(radd: &'radd reqwest::Response, ism: &str) -> Option<&'radd str> {
    radd.headers().get(ism)?.to_str().ok().map(str::trim)
}

/// A response header parsed as a number, when it is one.
fn qeemat_ras_raqm<T: std::str::FromStr>(radd: &reqwest::Response, ism: &str) -> Option<T> {
    qeemat_ras(radd, ism)?.parse::<T>().ok()
}

/// Reads a response's body under a ceiling, with its rate-limit envelope.
///
/// Only the `retry-after` delta-seconds form is read. The HTTP-date form would
/// mean comparing a clock that is not ours against one that is, and misreading
/// it as an enormous wait would hang a submission; every forge this transport
/// targets sends whole seconds. The reset header is different and *is* read as
/// an absolute time, because it is defined as an epoch second rather than as a
/// date string, and the wait it produces is bounded by [`AQSA_TARAJU`] anyway.
async fn iqra_radd(
    mut radd: reqwest::Response,
    hadd: u64,
    amal: &'static str,
) -> NatijatTaqdeem<RaddMahdud> {
    let hala = radd.status();
    let baqi = qeemat_ras_raqm::<u64>(&radd, "x-ratelimit-remaining");
    let istiada = qeemat_ras_raqm::<i64>(&radd, "x-ratelimit-reset");
    let baad = qeemat_ras_raqm::<u64>(&radd, reqwest::header::RETRY_AFTER.as_str());

    if radd.content_length().is_some_and(|muallan| muallan > hadd) {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: format!("the response declares more than the {hadd} byte ceiling"),
        });
    }

    let mut bayt: Vec<u8> = Vec::new();
    loop {
        let qita = radd.chunk().await.map_err(|khata| KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: khata.to_string(),
        })?;
        let Some(qita) = qita else { break };
        let majmu = u64::try_from(bayt.len().saturating_add(qita.len())).unwrap_or(u64::MAX);
        if majmu > hadd {
            return Err(KhataTaqdeem::MustawdaRafad {
                amal,
                sabab: format!("the response passed the {hadd} byte ceiling while arriving"),
            });
        }
        bayt.extend_from_slice(&qita);
    }

    Ok(RaddMahdud { hala, bayt, baqi, istiada, baad })
}

/// Sends a request, retrying under `siyasa` while the forge says it did not act
/// on it, and returns whatever finally came back.
///
/// `bani` builds the request afresh on every attempt rather than cloning one:
/// a streamed body cannot be cloned, and a builder that has already been sent
/// is gone.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the transport fails on the last attempt
/// this policy allows, or when the body passes the ceiling.
pub async fn arsil<F>(
    bani: F,
    siyasa: SiyasatItada,
    hadd: u64,
    amal: &'static str,
) -> NatijatTaqdeem<RaddMahdud>
where
    // `Send` is part of the contract, not decoration: the builder is held
    // across every await in the loop, and a submission runs on the same
    // multi-threaded runtime the interface's commands are spawned onto, which
    // will not take a future that is not `Send`.
    F: Fn() -> reqwest::RequestBuilder + Send,
{
    let mut akhir: Option<KhataTaqdeem> = None;
    for muhawala in 0..ADAD_MUHAWALAT_TALAB {
        let natija = bani().send().await;
        let radd = match natija {
            Ok(radd) => radd,
            Err(khata) => {
                // A transport failure leaves it unknown whether the forge saw
                // the request at all, so only a policy that says the request is
                // repeatable may repeat it.
                let sabab = khata.to_string();
                akhir = Some(KhataTaqdeem::MustawdaRafad { amal, sabab: sabab.clone() });
                if siyasa == SiyasatItada::Aid && muhawala.saturating_add(1) < ADAD_MUHAWALAT_TALAB
                {
                    tracing::debug!(amal, muhawala, %sabab, "retrying after a transport failure");
                    tokio::time::sleep(taraju(muhawala)).await;
                    continue;
                }
                break;
            }
        };

        let mahdud = iqra_radd(radd, hadd, amal).await?;
        if mahdud.najah() || muhawala.saturating_add(1) >= ADAD_MUHAWALAT_TALAB {
            return Ok(mahdud);
        }

        let yajuz = mahdud.hadd_muadal() || (siyasa == SiyasatItada::Aid && mahdud.aarid());
        if !yajuz {
            return Ok(mahdud);
        }
        let Some(intizar) = mahdud.intizar(muhawala, jiff::Timestamp::now().as_second()) else {
            tracing::warn!(
                amal,
                hala = mahdud.hala.as_u16(),
                "the forge asked for a longer wait than this client will hold a submission for"
            );
            return Ok(mahdud);
        };
        tracing::debug!(
            amal,
            muhawala,
            hala = mahdud.hala.as_u16(),
            thawani = intizar.as_secs(),
            "the forge refused without acting; waiting before sending again"
        );
        tokio::time::sleep(intizar).await;
    }

    Err(akhir.unwrap_or_else(|| KhataTaqdeem::MustawdaRafad {
        amal,
        sabab: "the request was not attempted".to_owned(),
    }))
}

/// Sends a request under a retry policy, refuses anything outside 2xx, and
/// parses the body.
async fn jalb_json<T, F>(bani: F, siyasa: SiyasatItada, amal: &'static str) -> NatijatTaqdeem<T>
where
    T: serde::de::DeserializeOwned,
    F: Fn() -> reqwest::RequestBuilder + Send,
{
    let radd = arsil(bani, siyasa, HADD_HAJM_ISTIJABA, amal).await?;
    if !radd.najah() {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: iqtibas(radd.hala, &radd.bayt),
        });
    }
    serde_json::from_slice(&radd.bayt).map_err(|khata| KhataTaqdeem::MustawdaRafad {
        amal,
        sabab: format!("the response did not parse: {khata}"),
    })
}

/// The authenticated identity at the forge.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the endpoint refuses or answers with a
/// body this build cannot read.
pub async fn hawiya_muwaththaqa(
    amil: &reqwest::Client,
    idadat: &IdadatTawthiq,
    ramz: &RamzWusul,
) -> NatijatTaqdeem<String> {
    let amal = "reading the authenticated identity";
    let rabt = idadat.qalab_tadqiq.mila(&[])?;
    let tarwisa = ramz.tarwisa();
    let radd: RaddHawiya =
        jalb_json(|| tarwisat_api(amil.get(&rabt), &tarwisa), SiyasatItada::Aid, amal).await?;
    let Some(login) = radd.ism() else {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the forge named no account for this token".to_owned(),
        });
    };
    if !juz_salih(login) {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the forge named an account this build will not put in a URL".to_owned(),
        });
    }
    Ok(login.to_owned())
}

/// Finds the caller's fork of the registry, creating it when there is none.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the forge refuses the lookup or the
/// creation, or when a created fork is not readable within the wait.
pub async fn shawka(
    amil: &reqwest::Client,
    idadat: &IdadatMustawda,
    ramz: &RamzWusul,
    login: &str,
) -> NatijatTaqdeem<String> {
    let mawjuda = idadat
        .qalab_shawkati
        .mila(&[("malik_shawka", login), ("mustawda", &idadat.mustawda)])?;
    let tarwisa = ramz.tarwisa();
    let rabt_shawka =
        || idadat.qalab_git_shawka.mila(&[("malik_shawka", login), ("mustawda", &idadat.mustawda)]);

    let amal = "looking for an existing fork";
    let radd = arsil(
        || tarwisat_api(amil.get(&mawjuda), &tarwisa),
        SiyasatItada::Aid,
        HADD_HAJM_ISTIJABA,
        amal,
    )
    .await?;
    if radd.najah() {
        return rabt_shawka();
    }
    // Only "there is no such repository" means there is no fork yet. A refused
    // token answers 401 and a token without the scope answers 403, and reading
    // either as "no fork" would go on to attempt a creation that fails for the
    // same reason, reporting the wrong step as the one that broke.
    if radd.hala != reqwest::StatusCode::NOT_FOUND {
        return Err(KhataTaqdeem::MustawdaRafad { amal, sabab: iqtibas(radd.hala, &radd.bayt) });
    }

    let amal = "creating a fork of the registry";
    let insha = idadat
        .qalab_shawka
        .mila(&[("malik", &idadat.malik), ("mustawda", &idadat.mustawda)])?;
    let radd = arsil(
        || {
            tarwisat_api(amil.post(&insha), &tarwisa)
                .header(reqwest::header::CONTENT_LENGTH, 0)
        },
        // Forking a repository already forked answers with the existing fork
        // rather than making a second one, so an unanswered attempt is safe to
        // make again.
        SiyasatItada::Aid,
        HADD_HAJM_ISTIJABA,
        amal,
    )
    .await?;
    if !radd.najah() {
        return Err(KhataTaqdeem::MustawdaRafad { amal, sabab: iqtibas(radd.hala, &radd.bayt) });
    }

    // A fork is created asynchronously on every forge this transport targets,
    // so the lookup is repeated until it answers rather than assumed.
    let amal = "waiting for the fork to become readable";
    for _ in 0..HADD_INTIZAR_SHAWKA {
        tokio::time::sleep(FASIL_ISTITLA).await;
        let radd = arsil(
            || tarwisat_api(amil.get(&mawjuda), &tarwisa),
            SiyasatItada::Aid,
            HADD_HAJM_ISTIJABA,
            amal,
        )
        .await?;
        if radd.najah() {
            return rabt_shawka();
        }
        if radd.hala != reqwest::StatusCode::NOT_FOUND {
            return Err(KhataTaqdeem::MustawdaRafad {
                amal,
                sabab: iqtibas(radd.hala, &radd.bayt),
            });
        }
    }

    Err(KhataTaqdeem::MustawdaRafad {
        amal,
        sabab: "the fork was created and was still not readable when the wait ran out".to_owned(),
    })
}

/// Uploads a package to the staging area and returns where it landed.
///
/// # Errors
///
/// [`KhataTaqdeem::BayanNaqis`] when the configured endpoint is not a staging
/// one, and [`KhataTaqdeem::MustawdaRafad`] when the package is larger than
/// [`HADD_HAJM_HUZMA`] or the forge refuses the upload.
pub async fn irfa_ila_tajheez(
    idadat: &IdadatMustawda,
    ramz: &RamzWusul,
    ism: &str,
    bayt: Arc<Vec<u8>>,
) -> NatijatTaqdeem<String> {
    idadat.tahaqquq()?;
    let amal = "uploading the package to staging";
    let hajm = u64::try_from(bayt.len()).unwrap_or(u64::MAX);
    if hajm > HADD_HAJM_HUZMA {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: format!("the package is larger than the {HADD_HAJM_HUZMA} byte ceiling"),
        });
    }
    if bayt.is_empty() {
        return Err(KhataTaqdeem::BayanNaqis { haql: "a package with bytes in it" });
    }

    let rabt = idadat.qalab_tajheez.mila(&[("ism", ism)])?;
    let amil = bina_amil(MUHLAT_RAFA)?;
    let tarwisa = ramz.tarwisa();
    let radd: RaddRafa = jalb_json(
        || {
            tarwisat_api(amil.post(&rabt), &tarwisa)
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                // Declared rather than left to chunked transfer encoding: a
                // release-asset endpoint sizes its object from this header, and
                // a forge that has to buffer the whole body to discover the
                // length is a forge that times out on a half-gigabyte package.
                .header(reqwest::header::CONTENT_LENGTH, hajm)
                .body(jism_muqattaa(Arc::clone(&bayt), hajm))
        },
        // Re-sending an upload the forge may already have accepted would leave
        // two objects under one name, so only a refusal that says the bytes
        // were never read is retried.
        SiyasatItada::HaddFaqat,
        amal,
    )
    .await?;

    let Some(mawqi) = radd.rabt() else {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the forge accepted the package and named no address for it".to_owned(),
        });
    };
    if !mawqi.starts_with("https://") {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the forge answered with a staging address that is not https".to_owned(),
        });
    }
    // The binding check on the whole staging separation: whatever the endpoint
    // looked like, the address the forge answers with has to be in staging and
    // not in the public release area, or the package is not where the publish
    // step will later promote it from.
    if !mawqi.contains(MASAR_TAJHEEZ) || mawqi.contains(MASAR_ISDAR) {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: format!(
                "the forge placed the package outside the staging area; a staging address has \
                 to carry `{MASAR_TAJHEEZ}`"
            ),
        });
    }
    Ok(mawqi.to_owned())
}

/// A buffer shared between upload attempts, readable as bytes.
///
/// A newtype only because `Arc<Vec<u8>>` is `AsRef<Vec<u8>>` and not
/// `AsRef<[u8]>`, which is what an async reader over a cursor needs.
#[derive(Debug, Clone)]
struct HumulaHuzma(Arc<Vec<u8>>);

impl AsRef<[u8]> for HumulaHuzma {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

/// The package as a body sent in [`HAJM_QITAA_RAFA`] pieces.
///
/// Built fresh for each attempt from the shared buffer: a streamed body is
/// consumed as it is sent and cannot be handed to a retry, and copying half a
/// gigabyte per attempt to avoid that would be worse than sharing it.
fn jism_muqattaa(bayt: Arc<Vec<u8>>, hajm: u64) -> reqwest::Body {
    let qari = MurqibRafa {
        dakhili: std::io::Cursor::new(HumulaHuzma(bayt)),
        marsal: 0,
        hajm,
        akhir_ushr: 0,
    };
    reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::with_capacity(
        qari,
        HAJM_QITAA_RAFA,
    ))
}

/// The upload cursor, which reports how far it has got as it is drained.
///
/// Progress is reported at each tenth rather than per chunk: a half-gigabyte
/// package is five hundred chunks, and five hundred log lines for one upload is
/// noise a person has to scroll past to find the failure.
struct MurqibRafa {
    dakhili: std::io::Cursor<HumulaHuzma>,
    marsal: u64,
    hajm: u64,
    akhir_ushr: u64,
}

impl tokio::io::AsyncRead for MurqibRafa {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        siyaq: &mut std::task::Context<'_>,
        wia: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let qabl = wia.filled().len();
        let nafs = self.get_mut();
        let natija = std::pin::Pin::new(&mut nafs.dakhili).poll_read(siyaq, wia);
        if natija.is_ready() {
            let zada = wia.filled().len().saturating_sub(qabl);
            nafs.marsal = nafs.marsal.saturating_add(u64::try_from(zada).unwrap_or(0));
            let ushr = nafs
                .marsal
                .saturating_mul(10)
                .checked_div(nafs.hajm.max(1))
                .unwrap_or(0);
            if ushr > nafs.akhir_ushr {
                nafs.akhir_ushr = ushr;
                tracing::debug!(
                    marsal = nafs.marsal,
                    hajm = nafs.hajm,
                    "the package upload is progressing"
                );
            }
        }
        natija
    }
}

/// Opens the pull request that carries the submission.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the forge refuses to open it.
pub async fn iftah_talab_damj(
    amil: &reqwest::Client,
    idadat: &IdadatMustawda,
    ramz: &RamzWusul,
    login: &str,
    far: &str,
    unwan: &str,
    matn: &str,
) -> NatijatTaqdeem<(String, Option<u64>)> {
    let amal = "opening the pull request";
    let rabt = idadat
        .qalab_talab_damj
        .mila(&[("malik", &idadat.malik), ("mustawda", &idadat.mustawda)])?;
    let jism = serde_json::json!({
        "title": unwan,
        "body": matn,
        "head": format!("{login}:{far}"),
        "source_branch": far,
        "target_branch": idadat.far_asasi,
        "base": idadat.far_asasi,
    });
    let tarwisa = ramz.tarwisa();
    let radd: RaddTalabDamj = jalb_json(
        || tarwisat_api(amil.post(&rabt), &tarwisa).json(&jism),
        // The one call that must never be made twice: a second request against
        // the owner's queue is a second thing for a person to read and close.
        // Only a refusal that says the forge never looked at it is retried.
        SiyasatItada::HaddFaqat,
        amal,
    )
    .await?;
    let Some(mawqi) = radd.rabt() else {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: "the forge opened the request and named no address for it".to_owned(),
        });
    };
    Ok((mawqi.to_owned(), radd.raqm()))
}

/// Checks that a package's bytes hash to the fingerprint its record claims.
///
/// The hash is recomputed rather than read: `Ruqaa::iftah` runs BLAKE3 over the
/// container's body and refuses the file unless it matches the header, so what
/// this function compares against the record is a measurement of these bytes
/// and not a claim they carry. The bytes need no particular alignment for that
/// — every check `iftah` makes is arithmetic on offsets, and nothing here casts
/// a section table.
///
/// The record travelling with a submission is what the registry will show and
/// what a client will check a download against, and until here nothing has
/// compared it to the bytes actually about to be uploaded. A record built
/// against one compile and a package file from another is not a corruption
/// anybody would notice: the upload succeeds, the pull request reads correctly,
/// and every install of it fails its integrity check months later.
///
/// # Errors
///
/// [`KhataTaqdeem::BayanNaqis`] when the bytes are not a package this build can
/// open, and [`KhataTaqdeem::MustawdaRafad`] when they open to a different
/// fingerprint than the record names.
pub fn tahaqquq_basma(bayt: &[u8], mutawaqqa: Basma) -> NatijatTaqdeem<()> {
    let ruqaa = Ruqaa::iftah(bayt).map_err(|khata| {
        tracing::error!(%khata, "the package about to be uploaded would not open");
        KhataTaqdeem::BayanNaqis { haql: "a package this build can read back" }
    })?;
    let mawjuda = Basma::min_bayt(*ruqaa.basma());
    if mawjuda != mutawaqqa {
        return Err(KhataTaqdeem::MustawdaRafad {
            amal: "checking the package against its own record",
            sabab: format!(
                "the record names content hash {mutawaqqa} and the package hashes to \
                 {mawjuda}; the record and the file are from different compiles"
            ),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Sealing, the metadata record, and the description
// ---------------------------------------------------------------------------

/// Seals a compiled package with the contributor's own key.
///
/// # Errors
///
/// [`KhataTaqdeem::BayanNaqis`] when the package will not reopen or the seal
/// does not commit to its content hash.
pub fn akhtim_musahim(
    huzma: &mut HuzmaMabniya,
    miftah: &MiftahKhass,
    waqt: i64,
) -> NatijatTaqdeem<Basma> {
    let basma = {
        let ruqaa = Ruqaa::iftah(huzma.bayt.bayt()).map_err(|khata| {
            tracing::error!(%khata, "the compiled package would not reopen for sealing");
            KhataTaqdeem::BayanNaqis { haql: "a package this build can read back" }
        })?;
        *ruqaa.basma()
    };

    let mut kutla = KutlatTawqee {
        dawr: DawrMiftah::Musahim,
        khwarizmiya: Khwarizmiya::Ed25519,
        waqt,
        miftah: miftah.aam().bayt(),
        tawqee: [0; 64],
    };
    kutla.tawqee = miftah.waqqi(&kutla.risala(&basma));

    huzma.akhtim(&kutla, &MudaqqiqEd25519).map_err(|khata| {
        tracing::error!(%khata, "the contributor self-signature did not seal the package");
        KhataTaqdeem::BayanNaqis { haql: "a package that seals under the contributor's own key" }
    })?;
    Ok(Basma::min_bayt(basma))
}

/// Builds the registry record one submission puts in its shard.
#[must_use]
pub fn mulakhkhas(
    bayan: &BayanHuzma,
    hawiya: &HawiyatMusahim,
    basma: Basma,
    hajm: u64,
    rabt: String,
    waqt: String,
) -> MulakhkhasRuqaa {
    MulakhkhasRuqaa {
        id: bayan.id,
        murajaa: bayan.murajaa,
        unwan: bayan.wasf.unwan.clone(),
        musahim: hawiya.musahim.clone(),
        ism_musahim: hawiya.ism.clone(),
        taghtiya: bayan.taghtiya.kulli,
        adad_nusus: u32::try_from(bayan.bawwaba.nusus).unwrap_or(u32::MAX),
        hajm,
        bina_manassa: bayan.irtibat.manassat.clone(),
        basmat: bayan.irtibat.basmat.clone(),
        aila: bayan.muharrik.aila,
        khalfiya: bayan.muharrik.khalfiya,
        tabaqa: bayan.muharrik.tabaqa,
        tareeqa: bayan.wasf.tareeqa,
        rukhsa: bayan.wasf.rukhsa.clone(),
        taqyeem: None,
        adad_taqyeemat: 0,
        // Both of these are rewritten by the publishing sequence: the record
        // enters the shard pointing at staging and leaves it pointing at the
        // release area, with the real publication time.
        waqt_nashr: waqt,
        basmat_muhtawa: basma,
        rabt,
        rabt_mira: None,
    }
}

/// The quality warnings a reviewer is owed, in English.
#[must_use]
pub fn tahdheerat(bayan: &BayanHuzma) -> Vec<String> {
    let mut sutur: Vec<String> = bayan
        .taghtiya
        .asbab
        .iter()
        .copied()
        .map(SababAdamAlnashr::wasf_injilizi)
        .collect();

    if !bayan.taghtiya.mawzuna.mawthuqa() {
        sutur.push(
            "The weighted coverage figure rests mostly on class weights rather than on an \
             observed capture session."
                .to_owned(),
        );
    }
    if bayan.wasf.tareeqa.yahtaj_iqrar() {
        sutur.push(format!(
            "Declared method: {}. This is labelled publicly on the listing.",
            bayan.wasf.tareeqa.wasf_injilizi()
        ));
    }
    let tajawuz = &bayan.tajawuz.mulakhkhas;
    if tajawuz.mutajawiz > 0 {
        sutur.push(format!(
            "{} measured (string, size) pair(s) overrun their width; {} of those are the \
             translation's own doing.",
            tajawuz.mutajawiz, tajawuz.bi_masuliyat_altarjama
        ));
    }
    if tajawuz.ghayr_mutahaqqaq > 0 {
        sutur.push(format!(
            "{} (string, size) pair(s) could not be measured at all, across {} distinct string(s).",
            tajawuz.ghayr_mutahaqqaq, tajawuz.nusus_ghayr_mutahaqqaqa
        ));
    }
    sutur
}

/// The pull request body a submission carries.
#[must_use]
pub fn wasf_talab_damj(
    bayan: &BayanHuzma,
    hawiya: &HawiyatMusahim,
    record: &MulakhkhasRuqaa,
    wasf: &str,
    sijill_taghyeer: Option<&str>,
) -> String {
    use std::fmt::Write as _;

    let mut matn = String::new();
    let kulli = &bayan.taghtiya.kulli;

    let _ = writeln!(matn, "## {} — {}\n", record.unwan, bayan.wasf.ism_luba);
    let _ = writeln!(
        matn,
        "- Patch `{}` revision `{}`\n- Contributor `{}` ({})\n- Credit line: {}",
        record.id,
        record.murajaa.qeema(),
        hawiya.musahim.mukhtasar(),
        hawiya.ism,
        hawiya.itimad.as_deref().unwrap_or("none supplied")
    );
    let _ = writeln!(
        matn,
        "- Engine {} / tier {}\n- Licence `{}` — method: {}\n- Built by Taarib {}\n",
        bayan.muharrik.aila.ism(),
        bayan.muharrik.tabaqa.ism_injilizi(),
        record.rukhsa.muarrif(),
        bayan.wasf.tareeqa.wasf_injilizi(),
        bayan.wasf.isdar_taarib
    );

    matn.push_str("### Coverage\n\n");
    let _ = writeln!(matn, "{}\n", bayan.taghtiya.wasf_injilizi());
    matn.push_str("| measure | value |\n| --- | --- |\n");
    let _ = writeln!(
        matn,
        "| strings translated | {} of {} |\n| strings reviewed and approved | {} |",
        kulli.mutarjam, kulli.majmu, kulli.muakkad
    );
    let _ = writeln!(
        matn,
        "| occurrences covered | {} of {} |\n| opening session | {} of {} |",
        kulli.mutarjam_takrar, kulli.majmu_takrar, kulli.mutarjam_awwal, kulli.majmu_awwal
    );
    let _ = writeln!(
        matn,
        "| strings in the package | {} |\n| precomputed layouts | {} |\n| atlas pages | {} |",
        bayan.bawwaba.nusus, bayan.bawwaba.takhtitat, bayan.bawwaba.safahat
    );
    let _ = writeln!(
        matn,
        "| original game bytes carried | {} |\n| package size | {} bytes |\n",
        bayan.bawwaba.bayt_min_alluba, record.hajm
    );

    matn.push_str("### Target builds\n\n");
    if record.bina_manassa.is_empty() {
        matn.push_str("- No launcher build identifier; the patch binds by fingerprint alone.\n");
    } else {
        for bina in &record.bina_manassa {
            let _ = writeln!(matn, "- launcher build `{bina}`");
        }
    }
    for basma in &record.basmat {
        let _ = writeln!(matn, "- installation fingerprint `{basma}`");
    }
    matn.push('\n');

    matn.push_str("### Quality warnings\n\n");
    let tanbeehat = tahdheerat(bayan);
    if tanbeehat.is_empty() {
        matn.push_str("- None raised by the compile.\n\n");
    } else {
        for tanbeeh in &tanbeehat {
            let _ = writeln!(matn, "- {tanbeeh}");
        }
        matn.push('\n');
    }

    matn.push_str("### Package\n\n");
    let _ = writeln!(
        matn,
        "- content hash `{}`\n- staged at {}\n",
        record.basmat_muhtawa, record.rabt
    );

    if !wasf.trim().is_empty() {
        let _ = writeln!(matn, "### From the contributor\n\n{}\n", wasf.trim());
    }
    if let Some(sijill) = sijill_taghyeer.map(str::trim).filter(|nass| !nass.is_empty()) {
        let _ = writeln!(matn, "### Changes since the last revision\n\n{sijill}");
    }
    matn
}

/// Inserts a record into a shard's bytes, replacing any earlier revision of the
/// same lineage.
///
/// # Errors
///
/// [`KhataTaqdeem::MustawdaRafad`] when the existing shard does not parse or
/// the new one will not serialize.
pub fn adif_ila_shareeha(
    khaam: &[u8],
    luba: LubaId,
    record: &MulakhkhasRuqaa,
) -> NatijatTaqdeem<Vec<u8>> {
    let amal = "writing the metadata record into its shard";
    // `Default`, not a struct literal: an exhaustive literal here would make a
    // new field on the shard listing a compile error in this crate, and the
    // registry's shape is not this crate's to hold still.
    let mut muhtawa: MuhtawaShareeha = if khaam.is_empty() {
        MuhtawaShareeha::default()
    } else {
        serde_json::from_slice(khaam).map_err(|khata| KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: format!("the shard did not parse: {khata}"),
        })?
    };

    let qaima = muhtawa.ruqaa.entry(luba).or_default();
    match qaima.iter().position(|mawjud| mawjud.id == record.id) {
        Some(mawqi) => {
            if let Some(khana) = qaima.get_mut(mawqi) {
                *khana = record.clone();
            }
        }
        None => qaima.push(record.clone()),
    }
    qaima.sort_by(|awwal, thani| {
        awwal.id.cmp(&thani.id).then_with(|| awwal.murajaa.cmp(&thani.murajaa))
    });

    let mut bayt = serde_json::to_vec_pretty(&muhtawa).map_err(|khata| {
        KhataTaqdeem::MustawdaRafad {
            amal,
            sabab: format!("the shard will not serialize: {khata}"),
        }
    })?;
    bayt.push(b'\n');
    Ok(bayt)
}

// ---------------------------------------------------------------------------
// The git half
// ---------------------------------------------------------------------------

/// Everything the branch, commit and push need, owned so it can cross onto a
/// blocking thread.
///
/// Deliberately not [`std::fmt::Debug`]: it holds the access token.
struct MudkhalGit {
    masar: PathBuf,
    rabt_asl: String,
    rabt_shawka: String,
    far_asasi: String,
    far: String,
    login: String,
    sirr: String,
    ism_musahim: String,
    barid: String,
    ism_shareeha: String,
    luba: LubaId,
    record: MulakhkhasRuqaa,
    risala: String,
}

/// A git failure, as this crate's.
fn khata_git(amal: &'static str, khata: &git2::Error) -> KhataTaqdeem {
    KhataTaqdeem::MustawdaRafad { amal, sabab: khata.message().to_owned() }
}

/// Fetches the base branch, writes the shard, commits, and pushes the branch to
/// the caller's fork.
fn adfa(mudkhal: &MudkhalGit) -> NatijatTaqdeem<String> {
    let amal = "pushing the submission branch";
    masarat::insha_mujallad(&mudkhal.masar).map_err(|khata| KhataTaqdeem::KhataMalaf {
        masar: mudkhal.masar.clone(),
        amal: "creating the local registry mirror",
        sabab: std::io::Error::other(khata.injilizi),
    })?;

    let mustawda = git2::Repository::open_bare(&mudkhal.masar)
        .or_else(|_| git2::Repository::init_bare(&mudkhal.masar))
        .map_err(|khata| khata_git("opening the local registry mirror", &khata))?;

    let mut jalb = git2::FetchOptions::new();
    jalb.remote_callbacks(nida(&mudkhal.login, &mudkhal.sirr));
    jalb.download_tags(git2::AutotagOption::None);
    let marji = format!("+refs/heads/{}:{MARJI_ASAS}", mudkhal.far_asasi);
    mustawda
        .remote_anonymous(&mudkhal.rabt_asl)
        .map_err(|khata| khata_git("naming the upstream registry", &khata))?
        .fetch(&[marji.as_str()], Some(&mut jalb), None)
        .map_err(|khata| khata_git("fetching the registry base branch", &khata))?;

    let asas = mustawda
        .find_reference(MARJI_ASAS)
        .and_then(|marji| marji.peel_to_commit())
        .map_err(|khata| khata_git("reading the registry base branch", &khata))?;
    let jidhr = asas.tree().map_err(|khata| khata_git("reading the registry tree", &khata))?;

    let sabiqa = jidhr
        .get_name(MUJALLAD_SHARAIH)
        .and_then(|madkhal| madkhal.to_object(&mustawda).ok())
        .and_then(|kain| kain.into_tree().ok());
    let khaam = sabiqa
        .as_ref()
        .and_then(|shajara| shajara.get_name(&mudkhal.ism_shareeha))
        .and_then(|madkhal| madkhal.to_object(&mustawda).ok())
        .and_then(|kain| kain.peel_to_blob().ok())
        .map_or_else(Vec::new, |kutla| kutla.content().to_vec());

    let jadeed = adif_ila_shareeha(&khaam, mudkhal.luba, &mudkhal.record)?;
    let kutla = mustawda
        .blob(&jadeed)
        .map_err(|khata| khata_git("writing the shard blob", &khata))?;

    let mut bani_sharaih = mustawda
        .treebuilder(sabiqa.as_ref())
        .map_err(|khata| khata_git("building the shard directory", &khata))?;
    let _ = bani_sharaih
        .insert(&mudkhal.ism_shareeha, kutla, i32::from(git2::FileMode::Blob))
        .map_err(|khata| khata_git("building the shard directory", &khata))?;
    let sharaih = bani_sharaih
        .write()
        .map_err(|khata| khata_git("building the shard directory", &khata))?;

    let mut bani_jidhr = mustawda
        .treebuilder(Some(&jidhr))
        .map_err(|khata| khata_git("building the registry tree", &khata))?;
    let _ = bani_jidhr
        .insert(MUJALLAD_SHARAIH, sharaih, i32::from(git2::FileMode::Tree))
        .map_err(|khata| khata_git("building the registry tree", &khata))?;
    let jidhr_jadeed = bani_jidhr
        .write()
        .map_err(|khata| khata_git("building the registry tree", &khata))?;
    let shajara = mustawda
        .find_tree(jidhr_jadeed)
        .map_err(|khata| khata_git("building the registry tree", &khata))?;

    let tawqee = git2::Signature::now(&mudkhal.ism_musahim, &mudkhal.barid)
        .map_err(|khata| khata_git("signing the commit", &khata))?;
    let iltizam = mustawda
        .commit(None, &tawqee, &tawqee, &mudkhal.risala, &shajara, &[&asas])
        .map_err(|khata| khata_git("committing the metadata record", &khata))?;
    let _ = mustawda
        .reference(&format!("refs/heads/{}", mudkhal.far), iltizam, true, "taarib submission")
        .map_err(|khata| khata_git("creating the submission branch", &khata))?;

    let mut dafa = git2::PushOptions::new();
    dafa.remote_callbacks(nida(&mudkhal.login, &mudkhal.sirr));
    let dafa_marji = format!("+refs/heads/{}:refs/heads/{}", mudkhal.far, mudkhal.far);
    mustawda
        .remote_anonymous(&mudkhal.rabt_shawka)
        .map_err(|khata| khata_git(amal, &khata))?
        .push(&[dafa_marji.as_str()], Some(&mut dafa))
        .map_err(|khata| khata_git(amal, &khata))?;

    Ok(iltizam.to_string())
}

/// The credential callback every fetch and push uses.
fn nida(login: &str, sirr: &str) -> git2::RemoteCallbacks<'static> {
    let login = login.to_owned();
    let sirr = sirr.to_owned();
    let mut nida = git2::RemoteCallbacks::new();
    let _ = nida.credentials(move |_rabt, _mustakhdim, _masmuh| {
        git2::Cred::userpass_plaintext(&login, &sirr)
    });
    nida
}

// ---------------------------------------------------------------------------
// Local status tracking
// ---------------------------------------------------------------------------

/// How far one submission got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarhalatIrsal {
    /// The package has been sealed with the contributor's key.
    Khatm,
    /// The registry record has been built.
    Bayanat,
    /// A fork of the registry exists.
    Shawka,
    /// The package sits in staging.
    Tajheez,
    /// The submission branch has been pushed.
    Far,
    /// The pull request is open.
    TalabDamj,
}

impl MarhalatIrsal {
    /// The step's name, for a report a person reads.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Khatm => "sealing the package",
            Self::Bayanat => "building the registry record",
            Self::Shawka => "preparing the fork",
            Self::Tajheez => "staging the package",
            Self::Far => "pushing the branch",
            Self::TalabDamj => "opening the pull request",
        }
    }
}

/// One submission, as this machine records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillIrsal {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision submitted.
    pub murajaa: RuqaaRevision,
    /// The game.
    pub luba: LubaId,
    /// Who submitted it.
    pub musahim: MusahimId,
    /// Where it stands.
    pub hala: HalatRuqaa,
    /// The furthest step it reached.
    pub marhala: MarhalatIrsal,
    /// When the submission started, RFC 3339.
    pub waqt_bad: String,
    /// When this record was last written, RFC 3339.
    pub waqt_akhir: String,
    /// The branch it was pushed to.
    pub far: Option<String>,
    /// Where the pull request is.
    pub rabt_talab_damj: Option<String>,
    /// Where the package is staged.
    pub rabt_tajheez: Option<String>,
    /// The package's content hash.
    pub basma: Option<String>,
    /// The last failure, when the submission did not finish.
    pub akhir_khata: Option<String>,
}

impl SijillIrsal {
    /// Opens a record for a submission whose pre-flight gate passed.
    fn ibda(
        _ijtiyaz: IjtiyazTaqdeem,
        ruqaa: RuqaaId,
        murajaa: RuqaaRevision,
        luba: LubaId,
        musahim: MusahimId,
    ) -> Self {
        let waqt = jiff::Timestamp::now().to_string();
        Self {
            ruqaa,
            murajaa,
            luba,
            musahim,
            hala: HalatRuqaa::Musawwada,
            marhala: MarhalatIrsal::Khatm,
            waqt_bad: waqt.clone(),
            waqt_akhir: waqt,
            far: None,
            rabt_talab_damj: None,
            rabt_tajheez: None,
            basma: None,
            akhir_khata: None,
        }
    }

    /// Moves the record forward one step.
    fn taqaddum(&mut self, marhala: MarhalatIrsal) {
        self.marhala = marhala;
        self.waqt_akhir = jiff::Timestamp::now().to_string();
    }

    /// Whether the submission reached the owner's queue.
    #[must_use]
    pub const fn wasalat(&self) -> bool {
        matches!(self.marhala, MarhalatIrsal::TalabDamj)
    }
}

/// Every submission this machine has made.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaftarIrsal {
    sijillat: BTreeMap<RuqaaId, SijillIrsal>,
}

impl DaftarIrsal {
    /// Where the ledger lives under Taarib's data root.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the path cannot be built inside the
    /// data root.
    pub fn masar(masarat: &Masarat) -> NatijatTaqdeem<PathBuf> {
        masarat::dakhil(masarat.jidhr_bayanat(), "taqdeem/irsal.json").map_err(|khata| {
            KhataTaqdeem::KhataMalaf {
                masar: masarat.jidhr_bayanat().to_path_buf(),
                amal: "locating the submission ledger",
                sabab: std::io::Error::other(khata.injilizi),
            }
        })
    }

    /// Reads the ledger, treating an absent file as an empty one.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the file exists and cannot be read or
    /// does not parse.
    pub fn iftah(masar: &Path) -> NatijatTaqdeem<Self> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(sabab) => {
                return Err(KhataTaqdeem::KhataMalaf {
                    masar: masar.to_path_buf(),
                    amal: "reading the submission ledger",
                    sabab,
                });
            }
        };
        serde_json::from_slice(&bayt).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "reading the submission ledger",
            sabab: std::io::Error::other(khata.to_string()),
        })
    }

    /// Writes the ledger atomically.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when it will not serialize or the write
    /// fails.
    pub fn ihfadh(&self, masar: &Path) -> NatijatTaqdeem<()> {
        let bayt = serde_json::to_vec_pretty(self).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "writing the submission ledger",
            sabab: std::io::Error::other(khata.to_string()),
        })?;
        masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataTaqdeem::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "writing the submission ledger",
            sabab: std::io::Error::other(khata.injilizi),
        })
    }

    /// Records or replaces one submission's state.
    pub fn sajjil(&mut self, sijill: SijillIrsal) {
        let _ = self.sijillat.insert(sijill.ruqaa, sijill);
    }

    /// One submission's state.
    #[must_use]
    pub fn sijill(&self, ruqaa: RuqaaId) -> Option<&SijillIrsal> {
        self.sijillat.get(&ruqaa)
    }

    /// Every submission, oldest lineage first.
    pub fn sijillat(&self) -> impl Iterator<Item = &SijillIrsal> {
        self.sijillat.values()
    }

    /// Every submission that never reached the owner's queue.
    #[must_use]
    pub fn ghayr_muktamila(&self) -> Vec<&SijillIrsal> {
        self.sijillat.values().filter(|sijill| !sijill.wasalat()).collect()
    }
}

// ---------------------------------------------------------------------------
// The entry point
// ---------------------------------------------------------------------------

/// Everything one submission carries that is not a proof.
pub struct TalabIrsal<'a> {
    /// The compiled package's bytes, already sealed by the contributor at
    /// build time; nothing here signs or reads a key.
    pub bayt: Vec<u8>,
    /// The registry record the build produced, its staging address and
    /// publication moment still blank for this sequence to fill.
    pub record: MulakhkhasRuqaa,
    /// The pull-request body the build wrote against the live manifest.
    pub matn_talab: String,
    /// The game's title, for the pull-request heading.
    pub ism_luba: String,
    /// Their credit line and identity.
    pub hawiya: &'a HawiyatMusahim,
    /// The game the patch targets.
    pub luba: LubaId,
}

impl std::fmt::Debug for TalabIrsal<'_> {
    /// Prints the identities and never the package bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabIrsal")
            .field("luba", &self.luba)
            .field("musahim", &self.hawiya.musahim)
            .finish_non_exhaustive()
    }
}

/// What one submission produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatijatIrsal {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision submitted.
    pub murajaa: RuqaaRevision,
    /// The shard the record went into.
    pub shareeha: u16,
    /// The branch that carries it.
    pub far: String,
    /// The commit that carries it.
    pub iltizam: String,
    /// Where the package is staged.
    pub rabt_tajheez: String,
    /// Where the pull request is.
    pub rabt_talab_damj: String,
    /// The pull request's number, when the forge gives one.
    pub raqm_talab_damj: Option<u64>,
    /// The package's content hash.
    pub basma: String,
}

/// Sends one sealed submission: record, fork, stage, push, open the request.
///
/// The gate proof is taken by value, so a submission that did not pass every
/// blocking check is not representable at this call.
///
/// # Errors
///
/// [`KhataTaqdeem::BayanNaqis`] when the configuration or the package is not
/// something this transport will send, [`KhataTaqdeem::MustawdaRafad`] when the
/// forge refuses any step, and [`KhataTaqdeem::KhataMalaf`] when the local
/// ledger cannot be written.
pub async fn irsal(
    mut talab: TalabIrsal<'_>,
    ijtiyaz: IjtiyazTaqdeem,
    idadat: &IdadatIrsal,
    ramz: &RamzWusul,
    masarat: &Masarat,
) -> NatijatTaqdeem<NatijatIrsal> {
    idadat.tahaqquq()?;

    // Moved out rather than cloned: the package is up to half a gigabyte, and
    // the upload needs it shared across retries rather than copied per attempt.
    let bayt = Arc::new(std::mem::take(&mut talab.bayt));

    let masar_daftar = DaftarIrsal::masar(masarat)?;
    let mut daftar = DaftarIrsal::iftah(&masar_daftar)?;
    let mut sijill = SijillIrsal::ibda(
        ijtiyaz,
        talab.record.id,
        talab.record.murajaa,
        talab.luba,
        talab.hawiya.musahim.clone(),
    );

    let natija = ajri(&talab, bayt, idadat, ramz, masarat, &mut sijill).await;
    match &natija {
        Ok(_) => sijill.hala = HalatRuqaa::Muqaddama,
        Err(khata) => sijill.akhir_khata = Some(khata.to_string()),
    }
    sijill.waqt_akhir = jiff::Timestamp::now().to_string();
    daftar.sajjil(sijill);
    daftar.ihfadh(&masar_daftar)?;
    natija
}

/// The submission sequence, with the ledger entry moved forward at each step.
async fn ajri(
    talab: &TalabIrsal<'_>,
    bayt: Arc<Vec<u8>>,
    idadat: &IdadatIrsal,
    ramz: &RamzWusul,
    masarat: &Masarat,
    sijill: &mut SijillIrsal,
) -> NatijatTaqdeem<NatijatIrsal> {
    let waqt = jiff::Timestamp::now();
    // The package arrived sealed from the build; the hash below is the seal's,
    // read back out of the container rather than taken on the record's word.
    let basma = talab.record.basmat_muhtawa;
    tahaqquq_basma(&bayt, basma)?;
    sijill.basma = Some(basma.to_string());
    sijill.taqaddum(MarhalatIrsal::Khatm);

    let raqm = shareeha(talab.luba);
    let ism = ism_shareeha(raqm).map_err(|_| KhataTaqdeem::BayanNaqis {
        haql: "a shard index inside the registry layout",
    })?;
    let ism_huzma =
        format!("{}-r{}-{basma}.ruqaa", talab.record.id, talab.record.murajaa.qeema());
    sijill.taqaddum(MarhalatIrsal::Bayanat);

    let amil = bina_amil(MUHLAT_TALAB)?;
    let login = hawiya_muwaththaqa(&amil, &idadat.tawthiq, ramz).await?;
    let rabt_shawka = shawka(&amil, &idadat.mustawda, ramz, &login).await?;
    sijill.taqaddum(MarhalatIrsal::Shawka);

    let rabt_tajheez = irfa_ila_tajheez(&idadat.mustawda, ramz, &ism_huzma, bayt).await?;
    sijill.rabt_tajheez = Some(rabt_tajheez.clone());
    sijill.taqaddum(MarhalatIrsal::Tajheez);

    let mut record = talab.record.clone();
    record.rabt = rabt_tajheez.clone();
    record.waqt_nashr = waqt.to_string();
    let far = format!(
        "{}/{}-r{}",
        idadat.mustawda.bidayat_far,
        record.id.mukhtasar(),
        record.murajaa.qeema()
    );
    if !juz_salih(&far) {
        return Err(KhataTaqdeem::BayanNaqis { haql: "a branch name this build will push" });
    }

    let unwan =
        format!("{} — {} r{}", talab.ism_luba, record.unwan, record.murajaa.qeema());
    let matn = talab.matn_talab.clone();

    let mudkhal = MudkhalGit {
        masar: masarat.jidhr_bayanat().join("taqdeem").join("mustawda.git"),
        rabt_asl: idadat.mustawda.rabt_git.clone(),
        rabt_shawka,
        far_asasi: idadat.mustawda.far_asasi.clone(),
        far: far.clone(),
        login: login.clone(),
        sirr: ramz.sirr().to_owned(),
        ism_musahim: talab.hawiya.ism.clone(),
        barid: format!("{}@musahim.taarib.invalid", talab.hawiya.musahim.nass()),
        ism_shareeha: ism,
        luba: talab.luba,
        record: record.clone(),
        risala: format!("{unwan}\n\nShard {raqm:02x}; content hash {basma}."),
    };
    let iltizam = tokio::task::spawn_blocking(move || adfa(&mudkhal)).await.map_err(|khata| {
        KhataTaqdeem::MustawdaRafad {
            amal: "pushing the submission branch",
            sabab: khata.to_string(),
        }
    })??;
    sijill.far = Some(far.clone());
    sijill.taqaddum(MarhalatIrsal::Far);

    let (rabt_talab_damj, raqm_talab_damj) =
        iftah_talab_damj(&amil, &idadat.mustawda, ramz, &login, &far, &unwan, &matn).await?;
    sijill.rabt_talab_damj = Some(rabt_talab_damj.clone());
    sijill.taqaddum(MarhalatIrsal::TalabDamj);

    tracing::info!(ruqaa = %record.id, far = %far, "the submission reached the owner's queue");
    Ok(NatijatIrsal {
        ruqaa: record.id,
        murajaa: record.murajaa,
        shareeha: raqm,
        far,
        iltizam,
        rabt_tajheez,
        rabt_talab_damj,
        raqm_talab_damj,
        basma: basma.to_string(),
    })
}


#[cfg(test)]
mod ikhtibarat {
    use taarib_mustalahat::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
    use taarib_mustalahat::ruqaa::{RukhsaRuqaa, TareeqaTarjama};
    use taarib_mustalahat::taghtiya::Taghtiya;

    use super::*;

    /// A client identifier shaped the way the forge issues them today.
    const MUARRIF_HAQIQI: &str = "Ov23liQ7bK2mXpR4nT8w";

    /// The staging endpoint a GitHub registry offers, exactly as
    /// `docs/mustawda.md` §7.2 writes it: a release-asset upload addressed by
    /// the release's numeric identifier, with the staging marker carried in the
    /// asset name because the path has nowhere to put it.
    const RAFA_GITHUB: &str =
        "https://uploads.github.com/repos/cc1a2b/taarib-registry/releases/301442889/assets\
         ?name=tajheez-{ism}";

    fn tawthiq(muarrif: &str) -> IdadatTawthiq {
        IdadatTawthiq {
            rabt_ramz_jihaz: "https://github.com/login/device/code".to_owned(),
            rabt_ramz_wusul: "https://github.com/login/oauth/access_token".to_owned(),
            qalab_tadqiq: QalabRabt::jadeed("https://api.github.com/user"),
            muarrif_amil: muarrif.to_owned(),
            nitaq: "public_repo".to_owned(),
            hisab: "taqdeem".to_owned(),
        }
    }

    fn mustawda(rafa: &str) -> IdadatMustawda {
        IdadatMustawda {
            malik: "cc1a2b".to_owned(),
            mustawda: "taarib-registry".to_owned(),
            far_asasi: "main".to_owned(),
            bidayat_far: "taqdeem".to_owned(),
            rabt_git: "https://github.com/cc1a2b/taarib-registry.git".to_owned(),
            qalab_git_shawka: QalabRabt::jadeed(
                "https://github.com/{malik_shawka}/{mustawda}.git",
            ),
            qalab_shawkati: QalabRabt::jadeed(
                "https://api.github.com/repos/{malik_shawka}/{mustawda}",
            ),
            qalab_shawka: QalabRabt::jadeed(
                "https://api.github.com/repos/{malik}/{mustawda}/forks",
            ),
            qalab_tajheez: QalabRabt::jadeed(rafa),
            qalab_talab_damj: QalabRabt::jadeed(
                "https://api.github.com/repos/{malik}/{mustawda}/pulls",
            ),
        }
    }

    fn radd_khaam(hala: u16) -> RaddMahdud {
        RaddMahdud {
            hala: reqwest::StatusCode::from_u16(hala).unwrap_or(reqwest::StatusCode::OK),
            bayt: Vec::new(),
            baqi: None,
            istiada: None,
            baad: None,
        }
    }

    // -----------------------------------------------------------------------
    // The line: what the client identifier is needed for, and what it is not
    // -----------------------------------------------------------------------

    #[test]
    fn bila_muarrif_amil_alirsal_ghayr_muhayya() {
        // The refusal a person reads when the operator has not registered the
        // application yet. It has to name the settings field and say whose job
        // filling it is, in both languages, and it must not be the generic
        // "cannot be sent without ..." that reads as the contributor's fault.
        use taarib_usus::khata::Tafsir as _;

        let khata = tawthiq("   ").tahaqquq().err();
        assert!(
            matches!(&khata, Some(KhataTaqdeem::IrsalGhayrMuhayya { naqis })
                if *naqis == HAQL_MUARRIF_AMIL),
            "a blank client identifier is the channel not being provisioned, not a bad draft"
        );
        let Some(khata) = khata else { return };

        let injilizi = khata.injilizi();
        assert!(injilizi.contains("not provisioned"), "{injilizi}");
        assert!(injilizi.contains(HAQL_MUARRIF_AMIL), "{injilizi}");
        assert!(injilizi.contains("registry operator"), "{injilizi}");
        assert!(injilizi.contains("stays recorded locally"), "{injilizi}");

        let arabi = khata.arabi();
        assert!(arabi.contains("غير مجهّزة"), "{arabi}");
        assert!(arabi.contains(HAQL_MUARRIF_AMIL), "{arabi}");
        assert!(arabi.contains("مشغّل السجلّ"), "{arabi}");

        // A notice, not a failure: nothing was lost and the submission is still
        // on this machine.
        assert_eq!(khata.khutura(), taarib_usus::khata::Khutura::Tanbeeh);
    }

    #[test]
    fn hashw_alqalab_yuqal_ghayr_muhayya_aydan() {
        for hashw in ["TODO", "changeme", "your-client-id", "<client id>", "xxxxxxxx", "Iv1"] {
            assert!(
                !muarrif_muhayya(hashw),
                "`{hashw}` is what a half-filled settings file holds, not an identifier"
            );
        }
    }

    #[test]
    fn muarrif_haqiqi_yajtaz() {
        assert!(muarrif_muhayya(MUARRIF_HAQIQI), "the shape the forge issues today");
        assert!(muarrif_muhayya("Iv23liAbCdEfGhIjKlMn"), "the GitHub App shape");
        assert!(
            muarrif_muhayya(&"a1b2c3d4".repeat(8)),
            "the sixty-four hex characters a self-hosted forge issues"
        );
        assert!(tawthiq(MUARRIF_HAQIQI).muhayya());
        assert!(tawthiq(MUARRIF_HAQIQI).tahaqquq().is_ok());
    }

    #[test]
    fn kul_shay_ghayr_altawthiq_yaamal_bila_muarrif() {
        // The point of the whole exercise: with no client identifier at all,
        // every part of the transport that is not the device flow still builds,
        // validates and answers. Only `tawthiq` refuses.
        let idadat = IdadatIrsal { tawthiq: tawthiq(""), mustawda: mustawda(RAFA_GITHUB) };
        assert!(!idadat.muhayya());
        assert!(idadat.mustawda.tahaqquq().is_ok(), "the repository half needs no client id");
        assert!(bina_amil(MUHLAT_TALAB).is_ok(), "the HTTP client needs no client id");
        assert!(
            idadat
                .mustawda
                .qalab_talab_damj
                .mila(&[("malik", "cc1a2b"), ("mustawda", "r")])
                .is_ok(),
            "building a request URL needs no client id"
        );
    }

    #[test]
    fn bila_nuqtat_tajheez_alirsal_ghayr_muhayya() {
        let khata = mustawda("").tahaqquq().err();
        assert!(
            matches!(&khata, Some(KhataTaqdeem::IrsalGhayrMuhayya { naqis })
                if *naqis == HAQL_RABT_TAJHEEZ),
            "an unset staging endpoint is the second half of the same refusal"
        );
    }

    // -----------------------------------------------------------------------
    // URL templates
    // -----------------------------------------------------------------------

    #[test]
    fn qalab_yamla_kul_alhuqul() {
        let qalab = QalabRabt::jadeed("https://api.github.com/repos/{malik}/{mustawda}/pulls");
        assert_eq!(
            qalab.mila(&[("malik", "cc1a2b"), ("mustawda", "taarib-registry")]).ok(),
            Some("https://api.github.com/repos/cc1a2b/taarib-registry/pulls".to_owned())
        );
    }

    #[test]
    fn qalab_yarfud_ma_la_yudkhal_fi_rabt() {
        let qalab = QalabRabt::jadeed("https://api.github.com/repos/{malik}/x");
        for radee in ["../../admin", "a b", "a?b=1", "a#b", "", "a%2f", "a\nb"] {
            assert!(
                qalab.mila(&[("malik", radee)]).is_err(),
                "`{radee}` must not reach a URL this build sends"
            );
        }
    }

    #[test]
    fn qalab_yarfud_haqlan_lam_yumla_wa_ghayr_https() {
        let naqis = QalabRabt::jadeed("https://x/{malik}/{mustawda}");
        assert!(naqis.mila(&[("malik", "cc1a2b")]).is_err(), "an unfilled placeholder");
        let sarih = QalabRabt::jadeed("http://api.github.com/x");
        assert!(sarih.mila(&[]).is_err(), "cleartext");
    }

    // -----------------------------------------------------------------------
    // The staging separation
    // -----------------------------------------------------------------------

    #[test]
    fn nuqtat_rafa_github_maqbula() {
        // The upload address carries a numeric release identifier and has
        // nowhere in its path to name the staging area, so the marker rides in
        // the asset name — and comes back out in the download URL the response
        // check reads.
        assert!(qabul_tajheez(RAFA_GITHUB));
        assert!(mustawda(RAFA_GITHUB).tahaqquq().is_ok());
    }

    #[test]
    fn nuqtat_rafa_tusammi_altajheez_maqbula() {
        assert!(qabul_tajheez("https://forge.example/api/tajheez/upload?name=x"));
    }

    #[test]
    fn nuqtat_rafa_bila_alama_marfuda() {
        // A release-asset endpoint with no staging marker at all: valid HTTP,
        // and refused, because nothing in it or in the address it answers with
        // would distinguish staging from the public release area.
        let bila = "https://uploads.github.com/repos/cc1a2b/taarib-registry/releases/1/assets";
        assert!(!qabul_tajheez(bila));
        let khata = mustawda(bila).tahaqquq().err();
        assert!(
            matches!(khata, Some(KhataTaqdeem::BayanNaqis { .. })),
            "an unmarked upload endpoint is a refusal"
        );
        let Some(KhataTaqdeem::BayanNaqis { haql }) = khata else { return };
        assert!(haql.contains("tajheez-"), "the refusal has to say what to write: {haql}");
    }

    #[test]
    fn nuqtat_rafa_fi_mintaqat_alisdar_marfuda() {
        for radee in [
            "https://forge.example/api/isdar/upload?name=tajheez-x",
            "https://github.com/cc1a2b/taarib-registry/releases/download/tajheez/x.ruqaa",
            "https://forge.example/api/objects/put",
        ] {
            assert!(!qabul_tajheez(radee), "`{radee}` is not somewhere a package may be staged");
            assert!(matches!(
                mustawda(radee).tahaqquq(),
                Err(KhataTaqdeem::BayanNaqis { .. })
            ));
        }
    }

    // -----------------------------------------------------------------------
    // Real forge responses
    // -----------------------------------------------------------------------

    /// `POST /repos/{owner}/{repo}/pulls`, trimmed from a live read of
    /// `https://api.github.com/repos/cli/cli/pulls/14356`. Both `url` and
    /// `html_url` are present, which is the shape every GitHub answer has.
    const TALAB_DAMJ_GITHUB: &str = r#"{
        "url": "https://api.github.com/repos/cli/cli/pulls/14356",
        "id": 2913884451,
        "html_url": "https://github.com/cli/cli/pull/14356",
        "number": 14356,
        "state": "open"
    }"#;

    /// `POST /repos/{owner}/{repo}/releases/{id}/assets`, trimmed from a live
    /// read of the `cli/cli` latest release. Both `url` and
    /// `browser_download_url` are present.
    const RAFA_GITHUB_RADD: &str = r#"{
        "url": "https://api.github.com/repos/cc1a2b/taarib-registry/releases/assets/542974244",
        "id": 542974244,
        "name": "patch-r1.ruqaa",
        "state": "uploaded",
        "size": 4096,
        "browser_download_url":
            "https://github.com/cc1a2b/taarib-registry/releases/download/tajheez/patch-r1.ruqaa"
    }"#;

    #[test]
    fn radd_talab_damj_min_github_yuqra() {
        // The regression this file exists for. Reading `html_url` and `url` as
        // two spellings of one field made every successful pull request come
        // back as `duplicate field`, so the one call that cannot be repeated
        // reported a failure it had not had.
        let radd = serde_json::from_str::<RaddTalabDamj>(TALAB_DAMJ_GITHUB);
        assert!(radd.is_ok(), "a real forge answer must parse: {radd:?}");
        let radd = radd.unwrap_or(RaddTalabDamj {
            html_url: None,
            web_url: None,
            url: None,
            number: None,
            iid: None,
        });
        assert_eq!(radd.rabt(), Some("https://github.com/cli/cli/pull/14356"));
        assert_eq!(radd.raqm(), Some(14356));
    }

    #[test]
    fn radd_rafa_min_github_yuqra() {
        let radd = serde_json::from_str::<RaddRafa>(RAFA_GITHUB_RADD);
        assert!(radd.is_ok(), "a real forge answer must parse: {radd:?}");
        let radd = radd.unwrap_or(RaddRafa {
            browser_download_url: None,
            direct_asset_url: None,
            download_url: None,
            url: None,
        });
        let mawqi = radd.rabt().unwrap_or_default();
        assert!(mawqi.contains(MASAR_TAJHEEZ), "the public address, not the API handle: {mawqi}");
        assert!(!mawqi.contains("api.github.com"));
    }

    #[test]
    fn radd_rafa_min_gitlab_yuqra() {
        let khaam = r#"{"direct_asset_url":"https://gl.example/-/tajheez/patch.ruqaa"}"#;
        let radd = serde_json::from_str::<RaddRafa>(khaam).ok();
        assert_eq!(
            radd.as_ref().and_then(RaddRafa::rabt),
            Some("https://gl.example/-/tajheez/patch.ruqaa")
        );
    }

    #[test]
    fn radd_talab_damj_min_gitlab_yuqra() {
        let khaam = r#"{"web_url":"https://gl.example/g/p/-/merge_requests/7","iid":7}"#;
        let radd = serde_json::from_str::<RaddTalabDamj>(khaam).ok();
        assert_eq!(
            radd.as_ref().and_then(RaddTalabDamj::rabt),
            Some("https://gl.example/g/p/-/merge_requests/7")
        );
        assert_eq!(radd.as_ref().and_then(RaddTalabDamj::raqm), Some(7));
    }

    #[test]
    fn radd_hawiya_yaqra_kilta_alkitabatayn() {
        // A live read of `https://api.github.com/user` answers with `login` and
        // a `twitter_username` that must not be mistaken for it.
        let jithab = r#"{"login":"cc1a2b","url":"https://api.github.com/users/cc1a2b",
                         "name":"cc1a2b","twitter_username":null}"#;
        let radd = serde_json::from_str::<RaddHawiya>(jithab).ok();
        assert_eq!(radd.as_ref().and_then(RaddHawiya::ism), Some("cc1a2b"));

        let jitlab = r#"{"username":"cc1a2b","id":9}"#;
        let radd = serde_json::from_str::<RaddHawiya>(jitlab).ok();
        assert_eq!(radd.as_ref().and_then(RaddHawiya::ism), Some("cc1a2b"));
    }

    #[test]
    fn radd_jihaz_yaqbal_alkitabatayn_maan() {
        // RFC 8628 spells it `verification_uri`; servers that also send
        // `verification_url` used to make the whole device-code answer
        // unreadable.
        let khaam = r#"{"device_code":"d","user_code":"WDJB-MJHT",
                        "verification_uri":"https://github.com/login/device",
                        "verification_url":"https://github.com/login/device",
                        "expires_in":900,"interval":5}"#;
        let radd = serde_json::from_str::<RaddJihaz>(khaam).ok();
        assert_eq!(
            radd.as_ref().and_then(RaddJihaz::rabt),
            Some("https://github.com/login/device")
        );
    }

    // -----------------------------------------------------------------------
    // The device flow's error vocabulary
    // -----------------------------------------------------------------------

    #[test]
    fn khata_jihaz_yufassar_min_aljism_la_min_alhala() {
        assert!(matches!(hala_min_khata("authorization_pending", None), HalatIstitla::Muntazir));
        assert!(matches!(hala_min_khata("slow_down", None), HalatIstitla::Abti));
        // Both spellings of the same answer: the forge's table and its prose
        // disagree, and a client that read one of them would report an expiry
        // as an unknown failure.
        assert!(matches!(hala_min_khata("expired_token", None), HalatIstitla::Muntahi));
        assert!(matches!(hala_min_khata("token_expired", None), HalatIstitla::Muntahi));
    }

    #[test]
    fn khata_jihaz_alghayr_muhayya_yusammi_almashghil() {
        let hala = hala_min_khata("device_flow_disabled", None);
        assert!(matches!(hala, HalatIstitla::Marfud(_)), "a disabled device flow is a refusal");
        let HalatIstitla::Marfud(sabab) = hala else { return };
        assert!(sabab.contains("device flow enabled"), "{sabab}");
        assert!(sabab.contains("registry operator"), "{sabab}");

        let hala = hala_min_khata("incorrect_client_credentials", None);
        assert!(matches!(hala, HalatIstitla::Marfud(_)), "an unrecognised client id is a refusal");
        let HalatIstitla::Marfud(sabab) = hala else { return };
        assert!(sabab.contains("client identifier"), "{sabab}");
        assert!(!sabab.contains("incorrect_client_credentials"), "not the raw code: {sabab}");
    }

    #[test]
    fn khata_jihaz_majhul_yunqal_kama_wasal() {
        let hala = hala_min_khata("something_new", Some("the forge added a code"));
        assert!(matches!(hala, HalatIstitla::Marfud(_)), "an unknown code is still a refusal");
        let HalatIstitla::Marfud(sabab) = hala else { return };
        assert!(sabab.contains("something_new"), "{sabab}");
        assert!(sabab.contains("the forge added a code"), "{sabab}");
    }

    // -----------------------------------------------------------------------
    // Rate limits, retries and backoff
    // -----------------------------------------------------------------------

    #[test]
    fn hadd_almuadal_yutaraf_bikul_ashkalihi() {
        let mut kathir = radd_khaam(429);
        assert!(kathir.hadd_muadal(), "the plain form");
        kathir.baad = Some(30);
        assert!(kathir.hadd_muadal());

        let mut awwali = radd_khaam(403);
        awwali.baqi = Some(0);
        assert!(awwali.hadd_muadal(), "a spent primary window answers 403");

        let mut thanawi = radd_khaam(403);
        thanawi.baad = Some(60);
        assert!(thanawi.hadd_muadal(), "a secondary limit answers 403 with retry-after");

        let mut mamnu = radd_khaam(403);
        mamnu.baqi = Some(4998);
        assert!(!mamnu.hadd_muadal(), "a plain 403 is a permissions refusal, not a limit");
        assert!(!radd_khaam(404).hadd_muadal());
        assert!(!radd_khaam(422).hadd_muadal());
    }

    #[test]
    fn alaarid_yatamayyaz_an_almarfud() {
        for hala in [408_u16, 500, 502, 503, 504] {
            assert!(radd_khaam(hala).aarid(), "{hala} says nothing about whether it was acted on");
        }
        for hala in [400_u16, 401, 403, 404, 409, 422] {
            assert!(!radd_khaam(hala).aarid(), "{hala} is an answer, not a hiccup");
        }
    }

    #[test]
    fn intizar_yahtarim_talab_alkhadim() {
        let mut radd = radd_khaam(429);
        radd.baad = Some(45);
        assert_eq!(radd.intizar(0, 0), Some(Duration::from_secs(45)));
        // Never less than the server asked, even when the schedule is shorter.
        radd.baad = Some(1);
        assert_eq!(radd.intizar(0, 0), Some(ASAS_TARAJU.max(Duration::from_secs(1))));
    }

    #[test]
    fn intizar_yantazir_fath_alnafidha() {
        let mut radd = radd_khaam(403);
        radd.baqi = Some(0);
        radd.istiada = Some(1_000_090);
        assert_eq!(radd.intizar(0, 1_000_000), Some(Duration::from_secs(90)));
    }

    #[test]
    fn nafidha_ghayr_manfuda_la_tuqrar_intizaran() {
        // The reset header rides on every answer. A gateway failure that
        // happens to carry one must not park the submission until the hour.
        let mut radd = radd_khaam(503);
        radd.baqi = Some(4_321);
        radd.istiada = Some(1_003_600);
        assert_eq!(radd.intizar(0, 1_000_000), Some(ASAS_TARAJU));
    }

    #[test]
    fn hadd_bila_tarwisa_yantazir_daqiqa() {
        // The secondary limiter states no wait and cannot be queried; the
        // forge's own instruction is a minute.
        assert_eq!(radd_khaam(429).intizar(0, 0), Some(ADNA_INTIZAR_MUADAL));
    }

    #[test]
    fn intizar_atwal_min_alsaqf_yatawaqqaf() {
        let mut radd = radd_khaam(429);
        radd.baad = Some(AQSA_TARAJU.as_secs().saturating_add(1));
        assert_eq!(radd.intizar(0, 0), None, "a submission is not held silent for that long");
    }

    #[test]
    fn altaraju_yudaif_wa_yatawaqqaf_ind_alsaqf() {
        assert_eq!(taraju(0), ASAS_TARAJU);
        assert_eq!(taraju(1), ASAS_TARAJU.saturating_mul(2));
        assert_eq!(taraju(2), ASAS_TARAJU.saturating_mul(4));
        assert_eq!(taraju(30), AQSA_TARAJU, "the doubling stops at the ceiling");
    }

    #[test]
    fn thawani_hatta_la_tasir_salba() {
        assert_eq!(thawani_hatta(50, 100), Duration::ZERO, "a window already past is no wait");
        assert_eq!(thawani_hatta(150, 100), Duration::from_secs(50));
    }

    // -----------------------------------------------------------------------
    // The token, and the refusals around it
    // -----------------------------------------------------------------------

    #[test]
    fn ramz_yarfud_ma_la_yukhzan() {
        assert!(RamzWusul::jadeed("", None).is_err(), "blank");
        assert!(RamzWusul::jadeed("   ", None).is_err(), "whitespace only");
        assert!(RamzWusul::jadeed("gho_a\nb", None).is_err(), "a control character");
        assert!(RamzWusul::jadeed("a".repeat(HADD_TUL_RAMZ + 1), None).is_err(), "over the limit");
        assert!(
            RamzWusul::jadeed("gho_x", Some("Bearer token".to_owned())).is_err(),
            "a scheme with a space in it"
        );
    }

    #[test]
    fn ramz_yabni_tarwisa_wa_la_yatba_alsirr() {
        let ramz = RamzWusul::jadeed("gho_secretsecret", None);
        assert!(ramz.is_ok());
        let Ok(ramz) = ramz else { return };
        assert_eq!(ramz.tarwisa(), "Bearer gho_secretsecret");
        let mutba = format!("{ramz:?}");
        assert!(!mutba.contains("gho_secretsecret"), "the secret must never print: {mutba}");
    }

    // -----------------------------------------------------------------------
    // Everything else the transport does without a token at all
    // -----------------------------------------------------------------------

    #[test]
    fn iqtibas_yahudd_jism_alkhata() {
        let tawil = "x".repeat(HADD_IQTIBAS * 2);
        let nass = iqtibas(reqwest::StatusCode::UNPROCESSABLE_ENTITY, tawil.as_bytes());
        assert!(nass.starts_with("HTTP 422: "));
        assert!(nass.len() < tawil.len(), "a four-kilobyte error page is not quoted whole");
    }

    fn sijill_tajribi(murajaa: u32) -> Option<MulakhkhasRuqaa> {
        Some(MulakhkhasRuqaa {
            id: RuqaaId::min_uuid(uuid_thabit()),
            murajaa: RuqaaRevision::jadeeda(murajaa),
            unwan: "تعريب كامل".to_owned(),
            musahim: MusahimId::jadeed("ab".repeat(32)).ok()?,
            ism_musahim: "cc1a2b".to_owned(),
            taghtiya: Taghtiya::default(),
            adad_nusus: 1,
            hajm: 4096,
            bina_manassa: Vec::new(),
            basmat: Vec::new(),
            aila: AilatMuharrik::Unity,
            khalfiya: KhalfiyaBarmajiya::Il2cpp,
            tabaqa: Tabaqa::Kamil,
            tareeqa: TareeqaTarjama::BashariyaKamila,
            rukhsa: RukhsaRuqaa::Cc0,
            taqyeem: None,
            adad_taqyeemat: 0,
            waqt_nashr: "2026-09-05T00:00:00Z".to_owned(),
            basmat_muhtawa: Basma::min_bayt([7; 32]),
            rabt: "https://example.invalid/tajheez/x.ruqaa".to_owned(),
            rabt_mira: None,
        })
    }

    fn uuid_thabit() -> uuid::Uuid {
        uuid::Uuid::from_bytes([
            0x01, 0x92, 0x3f, 0x4a, 0x5b, 0x6c, 0x7d, 0x8e, 0x9f, 0xa0, 0xb1, 0xc2, 0xd3, 0xe4,
            0xf5, 0x06,
        ])
    }

    #[test]
    fn shareeha_tastabdil_almurajaa_alsabiqa() {
        let zawj = (sijill_tajribi(1), sijill_tajribi(2));
        assert!(matches!(zawj, (Some(_), Some(_))), "the fixture record must build");
        let (Some(sabiq), Some(lahiq)) = zawj else { return };
        let luba = LubaId::min_uuid(uuid_thabit());

        let awwal = adif_ila_shareeha(&[], luba, &sabiq);
        assert!(awwal.is_ok());
        let awwal = awwal.unwrap_or_default();

        let thani = adif_ila_shareeha(&awwal, luba, &lahiq);
        assert!(thani.is_ok());
        let thani = thani.unwrap_or_default();
        let nass = String::from_utf8_lossy(&thani);
        assert_eq!(nass.matches("\"ism_musahim\"").count(), 1, "one lineage, one entry");
        assert!(nass.contains("\"murajaa\": 2"), "the newer revision replaces the older");
    }

    #[test]
    fn basma_la_tutabiq_alhuzma_turfad() {
        // The record and the file have to be from the same compile. Bytes that
        // are not a package at all fail first, and that is the case a wrong
        // path on disk produces.
        let khata = tahaqquq_basma(b"not a package", Basma::min_bayt([7; 32]));
        assert!(matches!(khata, Err(KhataTaqdeem::BayanNaqis { .. })));
    }

    #[test]
    fn juz_alrabt_yasmah_bima_yudkhal_faqat() {
        assert!(juz_salih("taqdeem/01923f4a-r1"));
        assert!(!juz_salih(""));
        assert!(!juz_salih("a..b"));
        assert!(!juz_salih("a b"));
        assert!(!juz_salih(&"a".repeat(129)));
    }
}

/// Round trips against real servers.
///
/// Ignored by default, so `cargo test` stays offline and deterministic. They
/// need no credential of any kind: everything the client identifier gates is
/// the device flow, and everything below is the transport underneath it.
///
/// ```text
/// cargo test -p taarib-taqdeem shabaka -- --ignored --test-threads 1
/// ```
#[cfg(test)]
mod ikhtibarat_shabaka {
    use super::*;

    /// A forge endpoint that answers every request, needs no token, and returns
    /// the rate-limit envelope this transport reads.
    const HUDUD: &str = "https://api.github.com/rate_limit";

    /// An echo service, for the request shapes a forge would only accept once.
    const SADA: &str = "https://httpbin.org/post";

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn shabaka_tarwisat_alhudud_tuqra_min_khadim_haqiqi() {
        let amil = bina_amil(MUHLAT_TALAB);
        assert!(amil.is_ok(), "the TLS client must build");
        let Ok(amil) = amil else { return };

        let radd = arsil(
            || amil.get(HUDUD).header(reqwest::header::ACCEPT, "application/json"),
            SiyasatItada::Aid,
            HADD_HAJM_ISTIJABA,
            "reading the rate limit",
        )
        .await;
        assert!(radd.is_ok(), "a real https round trip must complete: {radd:?}");
        let Ok(radd) = radd else { return };

        assert!(radd.najah(), "HTTP {}", radd.hala.as_u16());
        assert!(!radd.hadd_muadal());
        // The three headers every retry decision is made from, read off the
        // wire rather than off a fixture.
        assert!(radd.baqi.is_some(), "x-ratelimit-remaining was not read");
        assert!(radd.istiada.is_some(), "x-ratelimit-reset was not read");
        assert!(
            radd.istiada.is_some_and(|hadd| hadd > jiff::Timestamp::now().as_second()),
            "the window resets in the future"
        );
        assert!(!radd.bayt.is_empty(), "the body arrived");
    }

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn shabaka_alhawiya_taqra_hisab_ghayr_muwaththaq_kama_rafd() {
        // No token, so the identity endpoint refuses — and the refusal has to
        // arrive as this crate's own error naming the step, not as a panic and
        // not as a success with an empty account name.
        let amil = bina_amil(MUHLAT_TALAB);
        assert!(amil.is_ok(), "the TLS client must build");
        let ramz = RamzWusul::jadeed("ghp_notarealtokenatall", None);
        assert!(ramz.is_ok(), "the token must wrap");
        let (Ok(amil), Ok(ramz)) = (amil, ramz) else { return };

        let khata = hawiya_muwaththaqa(&amil, &tawthiq_ikhtibar(), &ramz).await;
        assert!(
            matches!(khata, Err(KhataTaqdeem::MustawdaRafad { .. })),
            "an unusable token is a named refusal: {khata:?}"
        );
        let Err(KhataTaqdeem::MustawdaRafad { amal, sabab }) = khata else { return };
        assert_eq!(amal, "reading the authenticated identity");
        assert!(sabab.starts_with("HTTP 401"), "the forge's own words: {sabab}");
    }

    fn tawthiq_ikhtibar() -> IdadatTawthiq {
        IdadatTawthiq {
            rabt_ramz_jihaz: "https://github.com/login/device/code".to_owned(),
            rabt_ramz_wusul: "https://github.com/login/oauth/access_token".to_owned(),
            qalab_tadqiq: QalabRabt::jadeed("https://api.github.com/user"),
            muarrif_amil: "Ov23liQ7bK2mXpR4nT8w".to_owned(),
            nitaq: "public_repo".to_owned(),
            hisab: "taqdeem.ikhtibar".to_owned(),
        }
    }

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn shabaka_alhuzma_tursal_muqattaa_bitul_muallan() {
        // The upload body as the forge receives it: one declared length, the
        // octet-stream type, and every header the transport claims to send.
        let bayt: Vec<u8> = (0..(HAJM_QITAA_RAFA * 2 + 7)).map(|n| {
            u8::try_from(n % 251).unwrap_or(0)
        }).collect();
        let hajm = u64::try_from(bayt.len()).unwrap_or(0);
        let humula = Arc::new(bayt);

        let amil = bina_amil(MUHLAT_RAFA);
        assert!(amil.is_ok(), "the TLS client must build");
        let Ok(amil) = amil else { return };
        let radd = arsil(
            || {
                tarwisat_api(amil.post(SADA), "Bearer gho_ikhtibar")
                    .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                    .header(reqwest::header::CONTENT_LENGTH, hajm)
                    .body(jism_muqattaa(Arc::clone(&humula), hajm))
            },
            SiyasatItada::HaddFaqat,
            HADD_HAJM_ISTIJABA,
            "echoing the package upload",
        )
        .await;
        assert!(radd.is_ok(), "the chunked body must reach a real server: {radd:?}");
        let Ok(radd) = radd else { return };
        assert!(radd.najah(), "HTTP {}", radd.hala.as_u16());

        // Compared in lower case: the echo re-cases header names it does not
        // recognise, and the assertion is about what was sent, not about how
        // somebody else spells it back.
        let sada = String::from_utf8_lossy(&radd.bayt).to_ascii_lowercase();
        assert!(sada.contains(&format!("\"content-length\": \"{hajm}\"")), "{sada}");
        assert!(sada.contains("\"content-type\": \"application/octet-stream\""), "{sada}");
        assert!(sada.contains("\"authorization\": \"bearer gho_ikhtibar\""), "{sada}");
        assert!(
            sada.contains(&format!(
                "\"{}\": \"{}\"",
                ISDAR_API_GITHUB.0.to_ascii_lowercase(),
                ISDAR_API_GITHUB.1
            )),
            "{sada}"
        );
        assert!(sada.contains("\"user-agent\": \"taarib/"), "{sada}");
        // No chunked framing beside a declared length: the body was sent in
        // pieces and still arrived as one sized object.
        assert!(!sada.contains("transfer-encoding"), "{sada}");
    }

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn shabaka_altahwil_yutba_hatta_alsaqf_thumma_yurfad() {
        let amil = bina_amil(MUHLAT_TALAB);
        assert!(amil.is_ok(), "the TLS client must build");
        let Ok(amil) = amil else { return };

        let dakhil = arsil(
            || amil.get("https://httpbin.org/redirect/2"),
            SiyasatItada::Aid,
            HADD_HAJM_ISTIJABA,
            "following redirects",
        )
        .await;
        assert!(dakhil.is_ok_and(|radd| radd.najah()), "two hops are inside the ceiling");

        let kharij = arsil(
            || amil.get("https://httpbin.org/redirect/6"),
            SiyasatItada::Aid,
            HADD_HAJM_ISTIJABA,
            "following redirects",
        )
        .await;
        assert!(
            matches!(kharij, Err(KhataTaqdeem::MustawdaRafad { .. })),
            "past the ceiling is a refusal, not a longer walk"
        );
        let Err(KhataTaqdeem::MustawdaRafad { sabab, .. }) = kharij else { return };
        assert!(sabab.contains("redirect"), "{sabab}");
    }

    #[tokio::test]
    #[ignore = "needs the network"]
    async fn shabaka_alaarid_yuada_taht_siyasa_wahida_faqat() {
        // The retry policy, observed rather than asserted about: a 503 is
        // repeated under `Aid` and the elapsed time carries the backoff
        // schedule, and is not repeated under `HaddFaqat`, where the same
        // failure returns immediately.
        let amil = bina_amil(MUHLAT_TALAB);
        assert!(amil.is_ok(), "the TLS client must build");
        let Ok(amil) = amil else { return };
        let radee = "https://httpbin.org/status/503";

        let bidaya = Instant::now();
        let marra = arsil(
            || amil.get(radee),
            SiyasatItada::HaddFaqat,
            HADD_HAJM_ISTIJABA,
            "a gateway failure, sent once",
        )
        .await;
        let mudda_marra = bidaya.elapsed();
        assert!(marra.is_ok_and(|radd| radd.hala.as_u16() == 503));

        let bidaya = Instant::now();
        let muada = arsil(
            || amil.get(radee),
            SiyasatItada::Aid,
            HADD_HAJM_ISTIJABA,
            "a gateway failure, sent again",
        )
        .await;
        let mudda_muada = bidaya.elapsed();
        assert!(muada.is_ok_and(|radd| radd.hala.as_u16() == 503));

        // Three waits before the fourth attempt: 500ms, 1s, 2s.
        let matluba = taraju(0).saturating_add(taraju(1)).saturating_add(taraju(2));
        assert!(
            mudda_muada >= matluba,
            "the retried call has to carry the backoff: {mudda_muada:?} < {matluba:?}"
        );
        assert!(
            mudda_marra < matluba,
            "the once-only call must not back off at all: {mudda_marra:?}"
        );
    }
}
