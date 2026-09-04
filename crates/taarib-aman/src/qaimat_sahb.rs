//! The owner-signed revocation list, verified before a single entry is trusted.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

use jiff::Timestamp;
use serde::Deserialize;
use taarib_khatm::MiftahAam;
use taarib_mustalahat::{Basma, RuqaaId};
use taarib_usus::masarat::{Masarat, insha_mujallad, kitaba_dharra};

use crate::khata::{KhataAman, NatijatAman};

/// The revocation-list format version this build writes and verifies.
pub const ISDAR_QAIMA: u32 = 1;

/// The cache file under [`Masarat::makhbaa`].
const ISM_MALAF_QAIMA: &str = "qaimat_sahb.json";

/// The lock file that serialises refreshes of [`ISM_MALAF_QAIMA`]. It is never
/// renamed or removed, so every refresh contends on the same inode; the cache
/// file itself is replaced by rename and would hand two racers two locks.
const ISM_MALAF_QUFL: &str = "qaimat_sahb.lock";

/// Domain separator for the signed canonical form, so these bytes can never be
/// mistaken for any other signed message in the product.
const FASIL: &[u8] = b"taarib.qaimat-sahb.v1\0";

/// A revocation list whose owner signature has been verified.
///
/// The only constructor is [`QaimatSahb::min_bayt`]; it verifies the embedded
/// signature over the canonical form before returning, so a value of this type
/// is proof the owner signed exactly these revocations. There is no
/// `Deserialize`: parsing goes through the private wire type and the verifier.
#[derive(Debug, Clone)]
pub struct QaimatSahb {
    tasalsul: u64,
    usdirat: String,
    salih_hatta: String,
    mafatih: BTreeMap<[u8; 32], Ilgha>,
    ruqa: BTreeMap<RuqaaId, Ilgha>,
    basmat: BTreeMap<Basma, Ilgha>,
}

impl QaimatSahb {
    /// Parses and verifies a revocation list from its signed JSON bytes.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when the bytes are not the expected
    /// format version, are structurally invalid, carry a malformed key,
    /// signature or duplicate entry, or when the embedded signature does not
    /// verify against `miftah_malik`.
    pub fn min_bayt(bayt: &[u8], miftah_malik: &MiftahAam) -> NatijatAman<Self> {
        let khaam: QaimatKhaam = serde_json::from_slice(bayt)
            .map_err(|khata| fashila("parsed", khata.to_string()))?;

        if khaam.isdar != ISDAR_QAIMA {
            return Err(fashila(
                "parsed",
                format!("unknown revocation-list format version {}", khaam.isdar),
            ));
        }

        let Some(tawqee) = min_hex::<64>(&khaam.tawqee) else {
            return Err(fashila("parsed", "signature is not 128 lowercase hex digits"));
        };

        let mut mafatih: BTreeMap<[u8; 32], Ilgha> = BTreeMap::new();
        for madkhal in &khaam.mafatih_mulgha {
            let Some(miftah) = min_hex::<32>(&madkhal.miftah) else {
                return Err(fashila("parsed", "a revoked signing key is not valid hex"));
            };
            let ilgha = Ilgha::jadeed(&madkhal.sabab, &madkhal.waqt);
            if mafatih.insert(miftah, ilgha).is_some() {
                return Err(fashila("parsed", "the same signing key is revoked more than once"));
            }
        }

        let mut ruqa: BTreeMap<RuqaaId, Ilgha> = BTreeMap::new();
        for madkhal in &khaam.ruqa_mulgha {
            let ilgha = Ilgha::jadeed(&madkhal.sabab, &madkhal.waqt);
            if ruqa.insert(madkhal.ruqaa, ilgha).is_some() {
                return Err(fashila("parsed", "the same patch lineage is revoked more than once"));
            }
        }

        let mut basmat: BTreeMap<Basma, Ilgha> = BTreeMap::new();
        for madkhal in &khaam.basmat_mulgha {
            let ilgha = Ilgha::jadeed(&madkhal.sabab, &madkhal.waqt);
            if basmat.insert(madkhal.basma, ilgha).is_some() {
                return Err(fashila("parsed", "the same content hash is revoked more than once"));
            }
        }

        let matn = matn_lil_tawqee(
            khaam.isdar,
            khaam.tasalsul,
            &khaam.usdirat,
            &khaam.salih_hatta,
            &mafatih,
            &ruqa,
            &basmat,
        );
        if !miftah_malik.tahaqquq(&matn, &tawqee) {
            return Err(fashila(
                "verified",
                "the signature does not verify against the owner key",
            ));
        }

        Ok(Self {
            tasalsul: khaam.tasalsul,
            usdirat: khaam.usdirat,
            salih_hatta: khaam.salih_hatta,
            mafatih,
            ruqa,
            basmat,
        })
    }

    /// Verifies `bayt_jadeed` and accepts it only if its sequence is at least
    /// this list's — the rollback rule that stops a replayed older list from
    /// superseding a newer one.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when `bayt_jadeed` does not verify, or
    /// when its sequence is older than this list's.
    pub fn baad(&self, bayt_jadeed: &[u8], miftah_malik: &MiftahAam) -> NatijatAman<Self> {
        let jadeeda = Self::min_bayt(bayt_jadeed, miftah_malik)?;
        if jadeeda.tasalsul < self.tasalsul {
            return Err(fashila(
                "accepted",
                format!(
                    "refusing to replace revocation list #{} with older #{}",
                    self.tasalsul, jadeeda.tasalsul
                ),
            ));
        }
        Ok(jadeeda)
    }

    /// Loads and verifies the cached list under [`Masarat::makhbaa`].
    ///
    /// Returns `Ok(None)` when no list has been cached yet.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when a cached file exists but cannot be
    /// read, parsed, or verified against `miftah_malik`.
    pub fn min_makhbaa(masarat: &Masarat, miftah_malik: &MiftahAam) -> NatijatAman<Option<Self>> {
        let masar = masar_qaima(masarat);
        let bayt = match std::fs::read(&masar) {
            Ok(bayt) => bayt,
            Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(khata) => {
                return Err(fashila("read", format!("{}: {khata}", masar.display())));
            }
        };
        Self::min_bayt(&bayt, miftah_malik).map(Some)
    }

    /// Replaces the cached list with freshly fetched bytes, but only if they
    /// verify and do not roll the sequence back below the cached list.
    ///
    /// The network fetch belongs to the registry client; this takes the already
    /// fetched bytes. Rollback rule: a validly signed cached list is the trusted
    /// baseline, and `bayt_jadeed` is refused when its `tasalsul` is lower than
    /// the baseline's, so a replayed older list can never supersede a newer one.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when the refresh lock cannot be taken,
    /// when `bayt_jadeed` does not verify, when its sequence is older than the
    /// cached list's, or when the cache file cannot be written.
    pub fn hadith_makhbaa(
        masarat: &Masarat,
        bayt_jadeed: &[u8],
        miftah_malik: &MiftahAam,
    ) -> NatijatAman<Self> {
        // The rollback rule only holds if reading the baseline, checking the
        // sequence against it and writing the result are one step. Unsynchronised,
        // two refreshes that both read sequence 5 write 7 and then 6, and the key
        // the newer list revoked becomes trusted again.
        let _qufl = qufl_makhbaa(masarat)?;

        // absent or unverifiable cache = no baseline; only a verified sequence blocks rollback
        let jadeeda = match Self::min_makhbaa(masarat, miftah_malik).ok().flatten() {
            Some(asas) => asas.baad(bayt_jadeed, miftah_malik)?,
            None => Self::min_bayt(bayt_jadeed, miftah_malik)?,
        };

        kitaba_dharra(&masar_qaima(masarat), bayt_jadeed)
            .map_err(|khata| fashila("written", khata.to_string()))?;
        Ok(jadeeda)
    }

    /// Why a signing key is revoked, when it is.
    #[must_use]
    pub fn mulgha_miftah(&self, miftah: &[u8; 32]) -> Option<&str> {
        self.mafatih.get(miftah).map(|ilgha| ilgha.sabab.as_str())
    }

    /// Why a patch lineage is revoked, when it is.
    #[must_use]
    pub fn mulgha_ruqaa(&self, ruqaa: RuqaaId) -> Option<&str> {
        self.ruqa.get(&ruqaa).map(|ilgha| ilgha.sabab.as_str())
    }

    /// Why a content hash is revoked, when it is.
    #[must_use]
    pub fn mulgha_basma(&self, basma: &Basma) -> Option<&str> {
        self.basmat.get(basma).map(|ilgha| ilgha.sabab.as_str())
    }

    /// The reason a patch is refused by key, lineage, or content hash — the
    /// three checks a pre-flight runs, in that order of severity.
    #[must_use]
    pub fn fahs_ruqaa(
        &self,
        ruqaa: RuqaaId,
        basma_muhtawa: &Basma,
        miftah_tawqee: &[u8; 32],
    ) -> Option<&str> {
        self.mulgha_miftah(miftah_tawqee)
            .or_else(|| self.mulgha_ruqaa(ruqaa))
            .or_else(|| self.mulgha_basma(basma_muhtawa))
    }

    /// The list's freshness at the instant `al_aan`, which the caller supplies.
    #[must_use]
    pub fn hala_salahiya(&self, al_aan: Timestamp) -> HalatSalahiya {
        match self.salih_hatta.parse::<Timestamp>() {
            Ok(hatta) if al_aan <= hatta => HalatSalahiya::Sariya,
            Ok(_) => HalatSalahiya::Muntahiya,
            Err(_) => HalatSalahiya::TarikhTalif,
        }
    }

    /// The monotonic sequence number of this list.
    #[must_use]
    pub const fn tasalsul(&self) -> u64 {
        self.tasalsul
    }

    /// When the list was issued, RFC 3339.
    #[must_use]
    pub fn usdirat(&self) -> &str {
        &self.usdirat
    }

    /// The instant after which the list is stale, RFC 3339.
    #[must_use]
    pub fn salih_hatta(&self) -> &str {
        &self.salih_hatta
    }

    /// How many revocations the list carries across all three kinds.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mafatih.len().saturating_add(self.ruqa.len()).saturating_add(self.basmat.len())
    }
}

/// A revocation list's freshness against a caller-supplied instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatSalahiya {
    /// Still within its validity window.
    Sariya,
    /// Past `salih_hatta`; the caller should refetch before trusting it.
    Muntahiya,
    /// `salih_hatta` is not a parseable RFC 3339 timestamp.
    TarikhTalif,
}

impl HalatSalahiya {
    /// Whether the list may still be trusted without refetching.
    #[must_use]
    pub const fn sariya(self) -> bool {
        matches!(self, Self::Sariya)
    }
}

/// One revocation's reason and its RFC 3339 timestamp, both signed.
#[derive(Debug, Clone)]
struct Ilgha {
    sabab: String,
    waqt: String,
}

impl Ilgha {
    fn jadeed(sabab: &str, waqt: &str) -> Self {
        Self { sabab: sabab.to_owned(), waqt: waqt.to_owned() }
    }
}

/// The wire form parsed before verification. Private and never constructed by
/// hand, so no path but [`QaimatSahb::min_bayt`] can produce a `QaimatSahb`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QaimatKhaam {
    isdar: u32,
    tasalsul: u64,
    usdirat: String,
    salih_hatta: String,
    #[serde(default)]
    mafatih_mulgha: Vec<MadkhalMiftahKhaam>,
    #[serde(default)]
    ruqa_mulgha: Vec<MadkhalRuqaaKhaam>,
    #[serde(default)]
    basmat_mulgha: Vec<MadkhalBasmaKhaam>,
    tawqee: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MadkhalMiftahKhaam {
    miftah: String,
    sabab: String,
    waqt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MadkhalRuqaaKhaam {
    ruqaa: RuqaaId,
    sabab: String,
    waqt: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MadkhalBasmaKhaam {
    basma: Basma,
    sabab: String,
    waqt: String,
}

fn masar_qaima(masarat: &Masarat) -> PathBuf {
    masarat.makhbaa().join(ISM_MALAF_QAIMA)
}

// The whole read-check-write of the cached list runs under this lock. It is a
// file lock, not a process-local mutex, for two reasons: `hadith_makhbaa` is an
// associated function over a path, so there is no owning struct to hang a mutex
// on and only a `static` would do; and the cache is one file shared by every
// Taarib process on the machine, which a `static` would not serialise at all.
// The lock is released when the returned handle closes, on every exit path.
fn qufl_makhbaa(masarat: &Masarat) -> NatijatAman<File> {
    let mujallad = masarat.makhbaa();
    insha_mujallad(&mujallad).map_err(|khata| fashila("locked", khata.to_string()))?;

    let masar = mujallad.join(ISM_MALAF_QUFL);
    let malaf = File::options()
        .read(true)
        .write(true)
        .create(true)
        // The handle exists only to be locked; truncating it would destroy the
        // lock file another process is holding at that moment.
        .truncate(false)
        .open(&masar)
        .map_err(|khata| fashila("locked", format!("{}: {khata}", masar.display())))?;
    malaf
        .lock()
        .map_err(|khata| fashila("locked", format!("{}: {khata}", masar.display())))?;
    Ok(malaf)
}

// canonical form the signer must reproduce: domain sep, LE length-prefixed, keys ascending
fn matn_lil_tawqee(
    isdar: u32,
    tasalsul: u64,
    usdirat: &str,
    salih_hatta: &str,
    mafatih: &BTreeMap<[u8; 32], Ilgha>,
    ruqa: &BTreeMap<RuqaaId, Ilgha>,
    basmat: &BTreeMap<Basma, Ilgha>,
) -> Vec<u8> {
    let mut matn = Vec::new();
    matn.extend_from_slice(FASIL);
    matn.extend_from_slice(&isdar.to_le_bytes());
    matn.extend_from_slice(&tasalsul.to_le_bytes());
    lp(&mut matn, usdirat.as_bytes());
    lp(&mut matn, salih_hatta.as_bytes());

    matn.extend_from_slice(&tul(mafatih.len()).to_le_bytes());
    for (miftah, ilgha) in mafatih {
        matn.extend_from_slice(miftah);
        lp(&mut matn, ilgha.sabab.as_bytes());
        lp(&mut matn, ilgha.waqt.as_bytes());
    }

    matn.extend_from_slice(&tul(ruqa.len()).to_le_bytes());
    for (id, ilgha) in ruqa {
        matn.extend_from_slice(id.uuid().as_bytes());
        lp(&mut matn, ilgha.sabab.as_bytes());
        lp(&mut matn, ilgha.waqt.as_bytes());
    }

    matn.extend_from_slice(&tul(basmat.len()).to_le_bytes());
    for (basma, ilgha) in basmat {
        matn.extend_from_slice(basma.bayt());
        lp(&mut matn, ilgha.sabab.as_bytes());
        lp(&mut matn, ilgha.waqt.as_bytes());
    }

    matn
}

fn lp(matn: &mut Vec<u8>, bayt: &[u8]) {
    matn.extend_from_slice(&tul(bayt.len()).to_le_bytes());
    matn.extend_from_slice(bayt);
}

fn tul(adad: usize) -> u64 {
    u64::try_from(adad).unwrap_or(u64::MAX)
}

fn min_hex<const N: usize>(nass: &str) -> Option<[u8; N]> {
    // lowercase hex only — the canonical form
    if !nass.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return None;
    }
    let mut khraj = [0u8; N];
    hex::decode_to_slice(nass, &mut khraj).ok()?;
    Some(khraj)
}

fn fashila(amal: &'static str, sabab: impl Into<String>) -> KhataAman {
    KhataAman::QaimatSahbFashila { amal, sabab: sabab.into() }
}
