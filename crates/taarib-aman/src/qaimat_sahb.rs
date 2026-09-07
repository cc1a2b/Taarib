//! The owner-signed revocation list: verified, refreshed, cached under a lock, and never
//! mistaken for a list nobody fetched.

use std::collections::BTreeMap;
use std::fs::File;
use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};
use taarib_khatm::{MiftahAam, MiftahKhass};
use taarib_mustalahat::{Basma, RuqaaId};
use taarib_usus::masarat::{Masarat, insha_mujallad, kitaba_dharra};

use crate::khata::{KhataAman, NatijatAman};

/// The revocation-list format version this build writes and verifies.
pub const ISDAR_QAIMA: u32 = 1;

/// The refresh-record format version this build writes and reads.
const ISDAR_SIJILL: u32 = 1;

/// The cache file under [`Masarat::makhbaa`].
const ISM_MALAF_QAIMA: &str = "qaimat_sahb.json";

/// The refresh record beside it: when the registry last vouched for the cached
/// list, and how the latest attempt to ask it again ended.
const ISM_MALAF_SIJILL: &str = "qaimat_sahb.tajdid.json";

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
    salih_hatta_lahza: Timestamp,
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
    /// signature, timestamp or duplicate entry, or when the embedded signature
    /// does not verify against `miftah_malik`.
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

        // Both dates are judged later, against a clock, and a list whose dates
        // cannot be judged would be a list nobody could call current or stale.
        if khaam.usdirat.parse::<Timestamp>().is_err() {
            return Err(fashila("parsed", "usdirat is not an RFC 3339 timestamp"));
        }
        let salih_hatta_lahza = khaam
            .salih_hatta
            .parse::<Timestamp>()
            .map_err(|_| fashila("parsed", "salih_hatta is not an RFC 3339 timestamp"))?;

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
            salih_hatta_lahza,
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

    /// Judges freshly fetched bytes against the machine's baseline and, when
    /// they win, makes them the cached list.
    ///
    /// The network fetch belongs to the registry client; this takes the already
    /// fetched bytes and the source they came from. The baseline is the newer of
    /// the verified cached list and `asas`, the list compiled into this build:
    /// a fetched list is refused when its sequence is below either, so a
    /// replayed older list can never supersede a newer one — not through an
    /// emptied cache, and not on a machine that has never fetched at all.
    ///
    /// What the bytes turn out to be is answered as a value, because none of it
    /// is a failure of this machine: [`Qabul::Qubilat`] when they are now the
    /// cached list, [`Qabul::Aqdam`] when the baseline stays, and
    /// [`Qabul::Marfuda`] when they did not verify. Every one of the three is
    /// written into the refresh record, so a gate that reads the cache later
    /// can say what the registry last answered.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when the refresh lock cannot be taken,
    /// or when the cache or its refresh record cannot be written.
    pub fn hadith_makhbaa(
        masarat: &Masarat,
        bayt_jadeed: &[u8],
        miftah_malik: &MiftahAam,
        asas: &Self,
        masdar: &str,
        al_aan: Timestamp,
    ) -> NatijatAman<Qabul> {
        // The rollback rule only holds if reading the baseline, checking the
        // sequence against it and writing the result are one step. Unsynchronised,
        // two refreshes that both read sequence 5 write 7 and then 6, and the key
        // the newer list revoked becomes trusted again.
        let _qufl = qufl_makhbaa(masarat)?;

        // absent or unverifiable cache = no cached baseline; the build's own list still floors
        let mukhazzana = Self::min_makhbaa(masarat, miftah_malik).ok().flatten();
        let tasalsul_asas =
            mukhazzana.as_ref().map_or(asas.tasalsul, |qaima| qaima.tasalsul.max(asas.tasalsul));

        let jadeeda = match Self::min_bayt(bayt_jadeed, miftah_malik) {
            Ok(jadeeda) => jadeeda,
            Err(khata) => {
                let natija = NatijatMuhawala::QaimaMutaadhdhira {
                    masdar: masdar.to_owned(),
                    sabab: khata.to_string(),
                };
                sajjil_muhawala_maqfula(masarat, MuhawalatTajdid { waqt: al_aan, natija })?;
                return Ok(Qabul::Marfuda(khata));
            }
        };

        if jadeeda.tasalsul < tasalsul_asas {
            let natija = NatijatMuhawala::Aqdam {
                masdar: masdar.to_owned(),
                jadeeda: jadeeda.tasalsul,
                asas: tasalsul_asas,
            };
            sajjil_muhawala_maqfula(masarat, MuhawalatTajdid { waqt: al_aan, natija })?;
            return Ok(Qabul::Aqdam { jadeeda: jadeeda.tasalsul, asas: tasalsul_asas });
        }

        kitaba_dharra(&masar_qaima(masarat), bayt_jadeed)
            .map_err(|khata| fashila("written", khata.to_string()))?;

        let mut sijill = sijill_tajdid(masarat);
        sijill.akhir_najah = Some(NajahTajdid {
            waqt: al_aan,
            masdar: masdar.to_owned(),
            tasalsul: jadeeda.tasalsul,
        });
        sijill.akhir_muhawala = Some(MuhawalatTajdid {
            waqt: al_aan,
            natija: NatijatMuhawala::Najah { masdar: masdar.to_owned(), tasalsul: jadeeda.tasalsul },
        });
        uktub_sijill(masarat, &sijill)?;
        Ok(Qabul::Qubilat(jadeeda))
    }

    /// Records a refresh attempt that produced no bytes to judge: the registry
    /// could not be reached, or it answered and withheld the list.
    ///
    /// Written so that the next gate to read the cache can say what the latest
    /// attempt found, rather than leaving "not refreshed" to be inferred from
    /// an old timestamp.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when the refresh lock cannot be taken
    /// or the record cannot be written.
    pub fn sajjil_muhawala(masarat: &Masarat, muhawala: MuhawalatTajdid) -> NatijatAman<()> {
        let _qufl = qufl_makhbaa(masarat)?;
        sajjil_muhawala_maqfula(masarat, muhawala)
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
        if al_aan <= self.salih_hatta_lahza {
            HalatSalahiya::Sariya
        } else {
            HalatSalahiya::Muntahiya
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

    /// The same instant, as a timestamp.
    #[must_use]
    pub const fn salih_hatta_lahza(&self) -> Timestamp {
        self.salih_hatta_lahza
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
}

impl HalatSalahiya {
    /// Whether the list may still be trusted without refetching.
    #[must_use]
    pub const fn sariya(self) -> bool {
        matches!(self, Self::Sariya)
    }
}

/// What [`QaimatSahb::hadith_makhbaa`] decided about fetched bytes.
#[derive(Debug)]
pub enum Qabul {
    /// Verified, at or above the baseline, and now the cached list.
    Qubilat(QaimatSahb),
    /// Verified, but older than the baseline, which stays.
    Aqdam {
        /// The sequence the bytes carried.
        jadeeda: u64,
        /// The sequence this machine already held.
        asas: u64,
    },
    /// The bytes did not parse or did not verify against the owner key.
    Marfuda(KhataAman),
}

/// The owner's side of the format: a list to be signed, in the exact canonical
/// form [`QaimatSahb::min_bayt`] verifies.
///
/// This produces bytes and nothing else. It cannot produce a [`QaimatSahb`],
/// which keeps the one rule the verifier exists for: a value of that type
/// always came through the signature check.
#[derive(Debug, Clone)]
pub struct KatibQaima {
    tasalsul: u64,
    usdirat: String,
    salih_hatta: String,
    mafatih: BTreeMap<[u8; 32], Ilgha>,
    ruqa: BTreeMap<RuqaaId, Ilgha>,
    basmat: BTreeMap<Basma, Ilgha>,
}

impl KatibQaima {
    /// An empty list at `tasalsul`, issued at `usdirat` and current until
    /// `salih_hatta`, both RFC 3339.
    #[must_use]
    pub fn jadeed(tasalsul: u64, usdirat: &str, salih_hatta: &str) -> Self {
        Self {
            tasalsul,
            usdirat: usdirat.to_owned(),
            salih_hatta: salih_hatta.to_owned(),
            mafatih: BTreeMap::new(),
            ruqa: BTreeMap::new(),
            basmat: BTreeMap::new(),
        }
    }

    /// Revokes a signing key. Revoking the same key twice keeps the later reason.
    #[must_use]
    pub fn ilgha_miftah(mut self, miftah: [u8; 32], sabab: &str, waqt: &str) -> Self {
        let _ = self.mafatih.insert(miftah, Ilgha::jadeed(sabab, waqt));
        self
    }

    /// Revokes a patch lineage.
    #[must_use]
    pub fn ilgha_ruqaa(mut self, ruqaa: RuqaaId, sabab: &str, waqt: &str) -> Self {
        let _ = self.ruqa.insert(ruqaa, Ilgha::jadeed(sabab, waqt));
        self
    }

    /// Revokes a content hash.
    #[must_use]
    pub fn ilgha_basma(mut self, basma: Basma, sabab: &str, waqt: &str) -> Self {
        let _ = self.basmat.insert(basma, Ilgha::jadeed(sabab, waqt));
        self
    }

    /// The signed document, pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// [`KhataAman::QaimatSahbFashila`] when the document cannot be serialised.
    pub fn uktub(&self, khass: &MiftahKhass) -> NatijatAman<Vec<u8>> {
        let matn = matn_lil_tawqee(
            ISDAR_QAIMA,
            self.tasalsul,
            &self.usdirat,
            &self.salih_hatta,
            &self.mafatih,
            &self.ruqa,
            &self.basmat,
        );
        let tawqee = khass.waqqi(&matn);
        let wathiqa = serde_json::json!({
            "isdar": ISDAR_QAIMA,
            "tasalsul": self.tasalsul,
            "usdirat": self.usdirat,
            "salih_hatta": self.salih_hatta,
            "mafatih_mulgha": self
                .mafatih
                .iter()
                .map(|(miftah, ilgha)| serde_json::json!({
                    "miftah": hex::encode(miftah),
                    "sabab": ilgha.sabab,
                    "waqt": ilgha.waqt,
                }))
                .collect::<Vec<_>>(),
            "ruqa_mulgha": self
                .ruqa
                .iter()
                .map(|(ruqaa, ilgha)| serde_json::json!({
                    "ruqaa": ruqaa,
                    "sabab": ilgha.sabab,
                    "waqt": ilgha.waqt,
                }))
                .collect::<Vec<_>>(),
            "basmat_mulgha": self
                .basmat
                .iter()
                .map(|(basma, ilgha)| serde_json::json!({
                    "basma": basma,
                    "sabab": ilgha.sabab,
                    "waqt": ilgha.waqt,
                }))
                .collect::<Vec<_>>(),
            "tawqee": hex::encode(tawqee),
        });
        serde_json::to_vec_pretty(&wathiqa).map_err(|khata| fashila("written", khata.to_string()))
    }
}

/// When the registry last vouched for the cached list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NajahTajdid {
    /// When.
    pub waqt: Timestamp,
    /// The source that served it, as the registry client names sources.
    pub masdar: String,
    /// The sequence it served.
    pub tasalsul: u64,
}

/// How one attempt to refresh the list ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MuhawalatTajdid {
    /// When the attempt was made.
    pub waqt: Timestamp,
    /// What it found.
    pub natija: NatijatMuhawala,
}

impl MuhawalatTajdid {
    /// One clause, in Arabic, describing this attempt.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match &self.natija {
            NatijatMuhawala::Najah { masdar, tasalsul } => {
                format!("آخر محاولة في {} نجحت من {masdar} بالتسلسل #{tasalsul}.", self.waqt)
            }
            NatijatMuhawala::MustawdaGhayrMutah { sabab } => {
                format!("آخر محاولة في {} لم تبلغ المستودع: {sabab}.", self.waqt)
            }
            NatijatMuhawala::QaimaMutaadhdhira { masdar, sabab } => format!(
                "آخر محاولة في {} بلغت {masdar} لكنّه لم يقدّم قائمة الإبطال: {sabab}.",
                self.waqt
            ),
            NatijatMuhawala::Aqdam { masdar, jadeeda, asas } => format!(
                "آخر محاولة في {} عُرضت عليها من {masdar} قائمة أقدم (#{jadeeda}) من \
                 المعتمدة (#{asas})، فأُبقيت المعتمدة.",
                self.waqt
            ),
        }
    }

    /// The same clause in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match &self.natija {
            NatijatMuhawala::Najah { masdar, tasalsul } => format!(
                "the last attempt at {} succeeded from {masdar} at sequence #{tasalsul}.",
                self.waqt
            ),
            NatijatMuhawala::MustawdaGhayrMutah { sabab } => {
                format!("the last attempt at {} could not reach the registry: {sabab}.", self.waqt)
            }
            NatijatMuhawala::QaimaMutaadhdhira { masdar, sabab } => format!(
                "the last attempt at {} reached {masdar}, which did not serve the revocation \
                 list: {sabab}.",
                self.waqt
            ),
            NatijatMuhawala::Aqdam { masdar, jadeeda, asas } => format!(
                "the last attempt at {} was offered an older list (#{jadeeda}) than the one \
                 held (#{asas}) by {masdar}, and the held one was kept.",
                self.waqt
            ),
        }
    }
}

/// What a refresh attempt found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum NatijatMuhawala {
    /// A list was served, verified, and is now the cached list.
    Najah {
        /// The source that served it.
        masdar: String,
        /// Its sequence.
        tasalsul: u64,
    },
    /// No source answered the manifest at all.
    MustawdaGhayrMutah {
        /// What the last source said.
        sabab: String,
    },
    /// The manifest was served and the list was not, or did not verify.
    ///
    /// The one outcome that is not the network's fault and not this machine's:
    /// the registry is up and the one document that can withdraw a patch is
    /// missing from it.
    QaimaMutaadhdhira {
        /// The source that answered the manifest.
        masdar: String,
        /// Why the list was not accepted.
        sabab: String,
    },
    /// A verified list older than the one held was offered; the held one stays.
    Aqdam {
        /// The source that offered it.
        masdar: String,
        /// The sequence offered.
        jadeeda: u64,
        /// The sequence held.
        asas: u64,
    },
}

/// The refresh record beside the cached list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillTajdid {
    /// The record format version.
    #[serde(default)]
    pub isdar: u32,
    /// The last time the registry vouched for a list, and which.
    #[serde(default)]
    pub akhir_najah: Option<NajahTajdid>,
    /// The latest attempt, whatever it found.
    #[serde(default)]
    pub akhir_muhawala: Option<MuhawalatTajdid>,
}

/// Where a gate's revocation list stands, beside the list itself.
///
/// Every variant but the first is a way of saying "the check will run against
/// a list the registry has not confirmed recently", and each says which way.
/// They exist so an empty revocation answer can never be read as "nothing is
/// revoked" when it means "nothing was asked".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalatQaima {
    /// The latest refresh succeeded within the refresh window; the list in hand
    /// is the one the registry vouched for.
    Muhaddatha {
        /// When.
        julibat: Timestamp,
        /// From which source.
        masdar: String,
    },
    /// A list the registry vouched for earlier, still inside its own validity;
    /// the latest attempt did not renew it, or none has been made since.
    Mukhazzana {
        /// When the registry last vouched for it, when the record says.
        julibat: Option<Timestamp>,
        /// The latest attempt, when one is recorded.
        akhir: Option<MuhawalatTajdid>,
    },
    /// The list in hand is past its own `salih_hatta` and was not refreshed.
    Muntahiya {
        /// When the registry last vouched for it; [`None`] for the compiled-in list.
        julibat: Option<Timestamp>,
        /// When it expired.
        salih_hatta: Timestamp,
        /// The latest attempt, when one is recorded.
        akhir: Option<MuhawalatTajdid>,
    },
    /// No list has ever been fetched on this machine; the compiled-in list is
    /// all there is.
    LamTujlab {
        /// The latest attempt, when one is recorded.
        akhir: Option<MuhawalatTajdid>,
    },
    /// The cache exists, or the record says it should, and it will not read;
    /// the compiled-in list is in use until the next refresh replaces it.
    Talifa {
        /// What is wrong with it.
        sabab: String,
        /// When the registry last vouched for the list that should be there.
        julibat: Option<Timestamp>,
        /// The latest attempt, when one is recorded.
        akhir: Option<MuhawalatTajdid>,
    },
}

impl HalatQaima {
    /// The state as a stable key, for a wire or a log filter.
    #[must_use]
    pub const fn ism(&self) -> &'static str {
        match self {
            Self::Muhaddatha { .. } => "muhaddatha",
            Self::Mukhazzana { .. } => "mukhazzana",
            Self::Muntahiya { .. } => "muntahiya",
            Self::LamTujlab { .. } => "lam_tujlab",
            Self::Talifa { .. } => "talifa",
        }
    }

    /// Whether the registry confirmed the list in hand within the window — the
    /// only state under which "nothing is revoked" is a statement about the
    /// registry rather than about this machine.
    #[must_use]
    pub const fn muhaddatha(&self) -> bool {
        matches!(self, Self::Muhaddatha { .. })
    }

    /// The latest recorded attempt, whatever the state.
    #[must_use]
    pub const fn akhir(&self) -> Option<&MuhawalatTajdid> {
        match self {
            Self::Muhaddatha { .. } => None,
            Self::Mukhazzana { akhir, .. }
            | Self::Muntahiya { akhir, .. }
            | Self::LamTujlab { akhir }
            | Self::Talifa { akhir, .. } => akhir.as_ref(),
        }
    }
}

/// The refusal a gate raises when the registry is answering and its revocation
/// list is not.
///
/// An unreachable registry is an offline machine and is never refused. A
/// reachable registry that withholds the one document able to withdraw a patch
/// is either broken or being interfered with, and in both cases "we could not
/// check" must not become "nothing is revoked" on a machine that could ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RafdQaima {
    /// The source that answered the manifest.
    pub masdar: String,
    /// Why the list was not accepted.
    pub sabab: String,
    /// When that attempt was made.
    pub waqt: Timestamp,
}

impl RafdQaima {
    /// The sentence shown to the user, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        format!(
            "أجاب المستودع ({}) في {} لكنّه لم يقدّم قائمة الإبطال ({}). ما دام المستودع \
             يجيب فلا يُثبَّت شيء قبل قراءة قائمته؛ أعد المحاولة، وإن كان المصدر مجلّدًا \
             محليًا فتأكّد من وجود ملف القائمة فيه.",
            self.masdar, self.waqt, self.sabab
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        format!(
            "The registry ({}) answered at {} but did not serve its revocation list ({}). While \
             the registry is reachable nothing is installed until its list can be read; try \
             again, and if the source is a local folder make sure the list file is in it.",
            self.masdar, self.waqt, self.sabab
        )
    }
}

/// A verified revocation list beside where it stands: the pair every gate
/// consults, so that the list can never travel without its state.
#[derive(Debug, Clone)]
pub struct QaimaMuraqaba {
    qaima: QaimatSahb,
    hala: HalatQaima,
    rafd: Option<RafdQaima>,
}

impl QaimaMuraqaba {
    /// The best list this machine holds and what is known about it, at `al_aan`.
    ///
    /// The list is the newer of the verified cache and `asliya`, the one
    /// compiled into this build, so a new build never regresses a machine and a
    /// stale build never overrides a fresh fetch. The state comes from the
    /// refresh record: a success within `nafidha` is [`HalatQaima::Muhaddatha`];
    /// anything else names what the latest attempt found. A cache that will
    /// not read is reported, not skipped.
    ///
    /// Never fails: every way the cache can be wrong is a state, and the
    /// compiled-in list is always there to fall back on.
    #[must_use]
    pub fn iqra(
        masarat: &Masarat,
        miftah_malik: &MiftahAam,
        asliya: QaimatSahb,
        al_aan: Timestamp,
        nafidha: SignedDuration,
    ) -> Self {
        let sijill = sijill_tajdid(masarat);
        let akhir = sijill.akhir_muhawala.clone();
        let julibat_musajjal = sijill.akhir_najah.as_ref().map(|najah| najah.waqt);

        let (qaima, hala) = match QaimatSahb::min_makhbaa(masarat, miftah_malik) {
            Err(khata) => (
                asliya,
                HalatQaima::Talifa { sabab: khata.to_string(), julibat: julibat_musajjal, akhir },
            ),
            Ok(None) if sijill.akhir_najah.is_some() => (
                asliya,
                HalatQaima::Talifa {
                    sabab: "the cached list the record vouches for is missing".to_owned(),
                    julibat: julibat_musajjal,
                    akhir,
                },
            ),
            Ok(None) => {
                let hala = if asliya.hala_salahiya(al_aan).sariya() {
                    HalatQaima::LamTujlab { akhir }
                } else {
                    HalatQaima::Muntahiya {
                        julibat: None,
                        salih_hatta: asliya.salih_hatta_lahza,
                        akhir,
                    }
                };
                (asliya, hala)
            }
            Ok(Some(mukhazzana)) => {
                // The record vouches for the cache only when it names the same
                // sequence: a crash between the two writes, or a file copied in
                // from elsewhere, leaves a list whose fetch time is unknown.
                let najah = sijill
                    .akhir_najah
                    .as_ref()
                    .filter(|najah| najah.tasalsul == mukhazzana.tasalsul);
                let julibat = najah.map(|najah| najah.waqt);
                let hadith = najah.is_some_and(|najah| {
                    akhir.as_ref().is_some_and(|muhawala| {
                        matches!(muhawala.natija, NatijatMuhawala::Najah { .. })
                            && muhawala.waqt >= najah.waqt
                            && al_aan.duration_since(najah.waqt) <= nafidha
                    })
                });
                // the newer of the two; a tie keeps the fetched one, which is the same list
                let qaima = if asliya.tasalsul > mukhazzana.tasalsul { asliya } else { mukhazzana };
                let hala = if !qaima.hala_salahiya(al_aan).sariya() {
                    HalatQaima::Muntahiya { julibat, salih_hatta: qaima.salih_hatta_lahza, akhir }
                } else if hadith && let Some(najah) = najah {
                    HalatQaima::Muhaddatha { julibat: najah.waqt, masdar: najah.masdar.clone() }
                } else {
                    HalatQaima::Mukhazzana { julibat, akhir }
                };
                (qaima, hala)
            }
        };

        Self::min_ajzaa(qaima, hala, al_aan, nafidha)
    }

    /// The pair from its parts, deciding the refusal from the state.
    ///
    /// The refusal fires on exactly one finding: the latest attempt, made
    /// within `nafidha`, reached the registry and came back without a list.
    /// Nothing about staleness, and nothing about a registry that could not be
    /// reached, refuses — those are the offline machine, which installs from a
    /// local copy with its state said out loud.
    #[must_use]
    pub fn min_ajzaa(
        qaima: QaimatSahb,
        hala: HalatQaima,
        al_aan: Timestamp,
        nafidha: SignedDuration,
    ) -> Self {
        let rafd = hala.akhir().and_then(|muhawala| match &muhawala.natija {
            NatijatMuhawala::QaimaMutaadhdhira { masdar, sabab }
                if al_aan.duration_since(muhawala.waqt) <= nafidha =>
            {
                Some(RafdQaima {
                    masdar: masdar.clone(),
                    sabab: sabab.clone(),
                    waqt: muhawala.waqt,
                })
            }
            _ => None,
        });
        Self { qaima, hala, rafd }
    }

    /// The list a gate checks against.
    #[must_use]
    pub const fn qaima(&self) -> &QaimatSahb {
        &self.qaima
    }

    /// Where that list stands.
    #[must_use]
    pub const fn hala(&self) -> &HalatQaima {
        &self.hala
    }

    /// The refusal, when the state earns one.
    #[must_use]
    pub const fn rafd(&self) -> Option<&RafdQaima> {
        self.rafd.as_ref()
    }

    /// The whole standing, in Arabic: which list, and what the registry has
    /// and has not confirmed about it.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let ras = format!(
            "قائمة الإبطال المستخدمة: التسلسل #{}، أُصدرت في {}، وتحمل {} إبطالًا. ",
            self.qaima.tasalsul,
            self.qaima.usdirat,
            self.qaima.adad()
        );
        let akhir_arabi = |akhir: &Option<MuhawalatTajdid>| {
            akhir.as_ref().map_or_else(
                || "ولم تُجرَ أيّ محاولة تحديث بعد.".to_owned(),
                MuhawalatTajdid::wasf_arabi,
            )
        };
        // The one state that needs no caveat, so it returns before the tail.
        if let HalatQaima::Muhaddatha { julibat, masdar } = &self.hala {
            return format!("{ras}حُدِّثت من {masdar} في {julibat}.");
        }
        let dhayl = match &self.hala {
            // Answered above.
            HalatQaima::Muhaddatha { .. } => String::new(),
            HalatQaima::Mukhazzana { julibat, akhir } => {
                let waqt =
                    julibat.map_or_else(|| "وقت غير مسجّل".to_owned(), |julibat| julibat.to_string());
                format!("آخر تأكيد لها من المستودع في {waqt}؛ {}", akhir_arabi(akhir))
            }
            HalatQaima::Muntahiya { julibat, salih_hatta, akhir } => {
                let asl = julibat.map_or_else(
                    || "وهي المضمّنة في هذه النسخة".to_owned(),
                    |julibat| format!("وآخر تأكيد لها من المستودع في {julibat}"),
                );
                format!(
                    "انتهت صلاحيتها في {salih_hatta} ولم تُحدَّث، {asl}؛ {}",
                    akhir_arabi(akhir)
                )
            }
            HalatQaima::LamTujlab { akhir } => format!(
                "لم تُجلب أيّ قائمة من المستودع على هذا الجهاز قط، فالمستخدمة هي المضمّنة في \
                 هذه النسخة؛ {}",
                akhir_arabi(akhir)
            ),
            HalatQaima::Talifa { sabab, julibat, akhir } => {
                let asl = julibat.map_or_else(String::new, |julibat| {
                    format!(" مع أنّ المستودع أكّد قائمة في {julibat}")
                });
                format!(
                    "تعذّرت قراءة القائمة المخزّنة ({sabab}){asl}، فالمستخدمة هي المضمّنة في \
                     هذه النسخة؛ {}",
                    akhir_arabi(akhir)
                )
            }
        };
        format!("{ras}{dhayl} فحص الإبطال جرى على هذه القائمة لا على قائمة المستودع الحالية.")
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let ras = format!(
            "Revocation list in use: sequence #{}, issued {}, carrying {} revocation(s). ",
            self.qaima.tasalsul,
            self.qaima.usdirat,
            self.qaima.adad()
        );
        let akhir_injilizi = |akhir: &Option<MuhawalatTajdid>| {
            akhir.as_ref().map_or_else(
                || "and no refresh has been attempted yet.".to_owned(),
                MuhawalatTajdid::wasf_injilizi,
            )
        };
        if let HalatQaima::Muhaddatha { julibat, masdar } = &self.hala {
            return format!("{ras}Refreshed from {masdar} at {julibat}.");
        }
        let dhayl = match &self.hala {
            HalatQaima::Muhaddatha { .. } => String::new(),
            HalatQaima::Mukhazzana { julibat, akhir } => {
                let waqt = julibat
                    .map_or_else(|| "an unrecorded time".to_owned(), |julibat| julibat.to_string());
                format!("The registry last confirmed it at {waqt}; {}", akhir_injilizi(akhir))
            }
            HalatQaima::Muntahiya { julibat, salih_hatta, akhir } => {
                let asl = julibat.map_or_else(
                    || "it is the one compiled into this build".to_owned(),
                    |julibat| format!("the registry last confirmed it at {julibat}"),
                );
                format!(
                    "It expired at {salih_hatta} and was not refreshed; {asl}; {}",
                    akhir_injilizi(akhir)
                )
            }
            HalatQaima::LamTujlab { akhir } => format!(
                "No list has ever been fetched from the registry on this machine, so the one in \
                 use is the one compiled into this build; {}",
                akhir_injilizi(akhir)
            ),
            HalatQaima::Talifa { sabab, julibat, akhir } => {
                let asl = julibat.map_or_else(String::new, |julibat| {
                    format!(" although the registry confirmed a list at {julibat}")
                });
                format!(
                    "The cached list could not be read ({sabab}){asl}, so the one in use is the \
                     one compiled into this build; {}",
                    akhir_injilizi(akhir)
                )
            }
        };
        format!(
            "{ras}{dhayl} The revocation check ran against this list, not against the \
             registry's current one."
        )
    }
}

/// Reads the refresh record; a missing or unreadable one is an empty record.
///
/// Unreadable is logged rather than raised: the record only says when the
/// cache was fetched, and losing it makes the state "confirmed at an
/// unrecorded time" — which is honest — rather than making the gate fail.
#[must_use]
pub fn sijill_tajdid(masarat: &Masarat) -> SijillTajdid {
    let masar = masar_sijill(masarat);
    match std::fs::read(&masar) {
        Ok(bayt) => match serde_json::from_slice::<SijillTajdid>(&bayt) {
            Ok(sijill) => sijill,
            Err(khata) => {
                tracing::warn!(
                    masar = %masar.display(),
                    khata = %khata,
                    "the revocation refresh record is unreadable and is treated as empty"
                );
                SijillTajdid::default()
            }
        },
        Err(khata) if khata.kind() == std::io::ErrorKind::NotFound => SijillTajdid::default(),
        Err(khata) => {
            tracing::warn!(
                masar = %masar.display(),
                khata = %khata,
                "the revocation refresh record could not be read and is treated as empty"
            );
            SijillTajdid::default()
        }
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

fn masar_sijill(masarat: &Masarat) -> PathBuf {
    masarat.makhbaa().join(ISM_MALAF_SIJILL)
}

// Caller holds the refresh lock: the record is read, changed and written as one step.
fn sajjil_muhawala_maqfula(masarat: &Masarat, muhawala: MuhawalatTajdid) -> NatijatAman<()> {
    let mut sijill = sijill_tajdid(masarat);
    sijill.akhir_muhawala = Some(muhawala);
    uktub_sijill(masarat, &sijill)
}

fn uktub_sijill(masarat: &Masarat, sijill: &SijillTajdid) -> NatijatAman<()> {
    let mubayyan = SijillTajdid { isdar: ISDAR_SIJILL, ..sijill.clone() };
    let bayt = serde_json::to_vec_pretty(&mubayyan)
        .map_err(|khata| fashila("written", khata.to_string()))?;
    kitaba_dharra(&masar_sijill(masarat), &bayt)
        .map_err(|khata| fashila("written", khata.to_string()))
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

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// The owner key every list here is signed with.
    fn malik() -> MiftahKhass {
        MiftahKhass::min_bayt(&[7u8; 32])
    }

    /// The refresh window the gates are read under.
    const NAFIDHA: SignedDuration = SignedDuration::from_hours(3);

    const USDIRAT: &str = "2026-09-01T00:00:00Z";
    const SALIH_HATTA: &str = "2027-09-01T00:00:00Z";

    /// A clock inside every fixture's validity window.
    fn al_aan() -> Result<Timestamp, Box<dyn Error>> {
        Ok("2026-09-06T12:00:00Z".parse::<Timestamp>()?)
    }

    fn masarat_muaqqata(masrah: &tempfile::TempDir) -> Masarat {
        Masarat::min_judhur(masrah.path().join("bayanat"), masrah.path().join("idadat"))
    }

    /// A signed, empty list at `tasalsul`: what a build ships.
    fn qaima_farigha(tasalsul: u64) -> Result<QaimatSahb, Box<dyn Error>> {
        let bayt = KatibQaima::jadeed(tasalsul, USDIRAT, SALIH_HATTA).uktub(&malik())?;
        Ok(QaimatSahb::min_bayt(&bayt, &malik().aam())?)
    }

    /// A signed list at `tasalsul` revoking one key, one lineage and one hash.
    fn bayt_mulgha(tasalsul: u64) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(KatibQaima::jadeed(tasalsul, USDIRAT, SALIH_HATTA)
            .ilgha_miftah([1u8; 32], "the signing key was compromised", USDIRAT)
            .ilgha_ruqaa(ruqaa_mulgha()?, "the patch bricks the game", USDIRAT)
            .ilgha_basma(Basma::min_bayt([2u8; 32]), "the package carries a bad pak", USDIRAT)
            .uktub(&malik())?)
    }

    /// A fixed lineage id, through the same parser the package manifest goes
    /// through; this crate has no reason to depend on the uuid crate for a test.
    fn ruqaa(raqm: u8) -> Result<RuqaaId, Box<dyn Error>> {
        let nass = format!("00000000-0000-4000-8000-0000000000{raqm:02x}");
        Ok(serde_json::from_value(serde_json::Value::String(nass))?)
    }

    fn ruqaa_mulgha() -> Result<RuqaaId, Box<dyn Error>> {
        ruqaa(3)
    }

    #[test]
    fn al_katib_wa_al_qari_yatafiqan_ala_al_thalatha() -> NatijatIkhtibar {
        let qaima = QaimatSahb::min_bayt(&bayt_mulgha(2)?, &malik().aam())?;

        assert_eq!(qaima.tasalsul(), 2);
        assert_eq!(qaima.adad(), 3);
        assert_eq!(qaima.mulgha_miftah(&[1u8; 32]), Some("the signing key was compromised"));
        assert_eq!(qaima.mulgha_ruqaa(ruqaa_mulgha()?), Some("the patch bricks the game"));
        assert_eq!(
            qaima.mulgha_basma(&Basma::min_bayt([2u8; 32])),
            Some("the package carries a bad pak")
        );
        assert_eq!(qaima.mulgha_miftah(&[9u8; 32]), None, "an unrelated key is not revoked");

        // The pre-flight order: key first, then lineage, then hash.
        let bariya = Basma::min_bayt([8u8; 32]);
        let ruqaa_bariya = ruqaa(4)?;
        assert_eq!(
            qaima.fahs_ruqaa(ruqaa_bariya, &bariya, &[1u8; 32]),
            Some("the signing key was compromised")
        );
        assert_eq!(
            qaima.fahs_ruqaa(ruqaa_mulgha()?, &bariya, &[9u8; 32]),
            Some("the patch bricks the game")
        );
        assert_eq!(
            qaima.fahs_ruqaa(ruqaa_bariya, &Basma::min_bayt([2u8; 32]), &[9u8; 32]),
            Some("the package carries a bad pak")
        );
        assert_eq!(qaima.fahs_ruqaa(ruqaa_bariya, &bariya, &[9u8; 32]), None);
        Ok(())
    }

    #[test]
    fn tawqee_mutalaab_aw_min_ghayr_al_malik_yurfad() -> NatijatIkhtibar {
        let salim = bayt_mulgha(2)?;
        let nass = String::from_utf8(salim.clone())?;

        // One hex digit of the signature changed.
        let mawdi = nass.find("\"tawqee\": \"").ok_or("no signature field")? + 11;
        let harf = nass.get(mawdi..=mawdi).ok_or("no signature digit")?;
        let badeel = if harf == "0" { "1" } else { "0" };
        let mutalaab = format!("{}{badeel}{}", &nass[..mawdi], &nass[mawdi + 1..]);
        assert!(QaimatSahb::min_bayt(mutalaab.as_bytes(), &malik().aam()).is_err());

        // A revocation reason edited after signing.
        let muharraf = nass.replace("bricks the game", "is perfectly fine");
        assert_ne!(muharraf, nass);
        assert!(QaimatSahb::min_bayt(muharraf.as_bytes(), &malik().aam()).is_err());

        // The right bytes under the wrong owner.
        let ghareeb = MiftahKhass::min_bayt(&[6u8; 32]).aam();
        assert!(QaimatSahb::min_bayt(&salim, &ghareeb).is_err());

        // And the untouched bytes still verify, so the three refusals above are
        // the signature's doing and not the fixture's.
        assert!(QaimatSahb::min_bayt(&salim, &malik().aam()).is_ok());
        Ok(())
    }

    #[test]
    fn tarikh_ghayr_salih_yurfad_ind_al_qiraa() -> NatijatIkhtibar {
        let bayt = KatibQaima::jadeed(1, "yesterday", SALIH_HATTA).uktub(&malik())?;
        assert!(QaimatSahb::min_bayt(&bayt, &malik().aam()).is_err());
        let bayt = KatibQaima::jadeed(1, USDIRAT, "2027-13-40").uktub(&malik())?;
        assert!(QaimatSahb::min_bayt(&bayt, &malik().aam()).is_err());
        Ok(())
    }

    #[test]
    fn al_tasalsul_al_aqdam_yurfad_wa_al_makhbaa_yabqa() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;

        let qabul = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(5)?,
            &malik().aam(),
            &asliya,
            "forge",
            waqt,
        )?;
        assert!(matches!(qabul, Qabul::Qubilat(ref qaima) if qaima.tasalsul() == 5));

        // A replay of sequence 3 against a cache at 5.
        let qabul = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(3)?,
            &malik().aam(),
            &asliya,
            "mirror",
            waqt,
        )?;
        assert!(matches!(qabul, Qabul::Aqdam { jadeeda: 3, asas: 5 }));

        let mukhazzana =
            QaimatSahb::min_makhbaa(&masarat, &malik().aam())?.ok_or("the cache vanished")?;
        assert_eq!(mukhazzana.tasalsul(), 5, "the replayed list must not touch the cache");
        assert!(mukhazzana.mulgha_miftah(&[1u8; 32]).is_some(), "the revocation survived");

        // The attempt was recorded as what it was.
        let sijill = sijill_tajdid(&masarat);
        assert_eq!(sijill.akhir_najah.as_ref().map(|najah| najah.tasalsul), Some(5));
        assert!(matches!(
            sijill.akhir_muhawala.as_ref().map(|muhawala| &muhawala.natija),
            Some(NatijatMuhawala::Aqdam { jadeeda: 3, asas: 5, .. })
        ));
        Ok(())
    }

    #[test]
    fn al_qaima_al_madmuna_tufarrish_al_tasalsul_bila_makhbaa() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        // A build that ships sequence 4 must not accept sequence 2 from a lagging
        // mirror even though this machine has never cached anything.
        let asliya = qaima_farigha(4)?;

        let qabul = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &asliya,
            "mirror",
            al_aan()?,
        )?;
        assert!(matches!(qabul, Qabul::Aqdam { jadeeda: 2, asas: 4 }));
        assert!(QaimatSahb::min_makhbaa(&masarat, &malik().aam())?.is_none());
        Ok(())
    }

    #[test]
    fn bayt_talifa_marfuda_wa_musajjala() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;

        let qabul = QaimatSahb::hadith_makhbaa(
            &masarat,
            b"{\"isdar\": 1, \"tasalsul\": 9, \"tawqee\": \"nope\"}",
            &malik().aam(),
            &asliya,
            "forge",
            al_aan()?,
        )?;
        assert!(matches!(qabul, Qabul::Marfuda(_)));
        assert!(QaimatSahb::min_makhbaa(&masarat, &malik().aam())?.is_none());
        assert!(matches!(
            sijill_tajdid(&masarat).akhir_muhawala.map(|muhawala| muhawala.natija),
            Some(NatijatMuhawala::QaimaMutaadhdhira { .. })
        ));
        Ok(())
    }

    #[test]
    fn lam_tujlab_qatt_tuqra_kadhalik_wa_la_turfad() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;

        let muraqaba = QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya, al_aan()?, NAFIDHA);

        assert!(matches!(muraqaba.hala(), HalatQaima::LamTujlab { akhir: None }));
        assert!(!muraqaba.hala().muhaddatha());
        assert!(muraqaba.rafd().is_none(), "a machine that never went online still installs");
        assert_eq!(muraqaba.qaima().tasalsul(), 1);
        assert!(muraqaba.wasf_injilizi().contains("has ever been fetched"));
        assert!(muraqaba.wasf_arabi().contains("لم تُجلب"));
        Ok(())
    }

    #[test]
    fn muhaddatha_dakhil_al_nafidha_wa_mukhazzana_baadaha() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;

        let _ = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &asliya,
            "forge",
            waqt,
        )?;

        let baad_saa = waqt.checked_add(SignedDuration::from_hours(1))?;
        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya.clone(), baad_saa, NAFIDHA);
        assert!(matches!(
            muraqaba.hala(),
            HalatQaima::Muhaddatha { masdar, .. } if masdar == "forge"
        ));
        assert!(muraqaba.hala().muhaddatha());
        assert_eq!(muraqaba.qaima().tasalsul(), 2);
        assert!(muraqaba.rafd().is_none());

        let baad_yawm = waqt.checked_add(SignedDuration::from_hours(24))?;
        let muraqaba = QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya, baad_yawm, NAFIDHA);
        assert!(matches!(
            muraqaba.hala(),
            HalatQaima::Mukhazzana { julibat: Some(julibat), .. } if *julibat == waqt
        ));
        assert!(!muraqaba.hala().muhaddatha());
        assert!(muraqaba.rafd().is_none(), "a stale but valid list does not block");
        assert_eq!(muraqaba.qaima().tasalsul(), 2, "stale still means the fetched list");
        Ok(())
    }

    #[test]
    fn muntahiya_baad_salih_hatta() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;
        let _ = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &asliya,
            "forge",
            waqt,
        )?;

        let baad_amayn = "2028-09-06T12:00:00Z".parse::<Timestamp>()?;
        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya.clone(), baad_amayn, NAFIDHA);
        assert!(matches!(muraqaba.hala(), HalatQaima::Muntahiya { julibat: Some(_), .. }));
        assert!(muraqaba.rafd().is_none(), "an expired list on an offline machine does not block");
        assert!(muraqaba.wasf_injilizi().contains("expired"));

        // The compiled-in list expires too, and says so without a fetch record.
        let farigh = tempfile::tempdir()?;
        let muraqaba = QaimaMuraqaba::iqra(
            &masarat_muaqqata(&farigh),
            &malik().aam(),
            asliya,
            baad_amayn,
            NAFIDHA,
        );
        assert!(matches!(muraqaba.hala(), HalatQaima::Muntahiya { julibat: None, .. }));
        Ok(())
    }

    #[test]
    fn makhbaa_talif_yuqal_wa_la_yukhfa() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;
        let _ = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &asliya,
            "forge",
            waqt,
        )?;
        std::fs::write(masar_qaima(&masarat), b"not a list any more")?;

        let muraqaba = QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya, waqt, NAFIDHA);
        assert!(matches!(muraqaba.hala(), HalatQaima::Talifa { julibat: Some(_), .. }));
        assert_eq!(muraqaba.qaima().tasalsul(), 1, "the compiled-in list is the floor");
        assert!(!muraqaba.hala().muhaddatha());
        assert!(muraqaba.wasf_injilizi().contains("could not be read"));

        // The list gone entirely, with a record that says it was fetched.
        std::fs::remove_file(masar_qaima(&masarat))?;
        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), qaima_farigha(1)?, waqt, NAFIDHA);
        assert!(matches!(muraqaba.hala(), HalatQaima::Talifa { .. }));
        Ok(())
    }

    #[test]
    fn al_mustawda_al_mujib_bila_qaima_yurfad_dakhil_al_nafidha_faqat() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;

        QaimatSahb::sajjil_muhawala(
            &masarat,
            MuhawalatTajdid {
                waqt,
                natija: NatijatMuhawala::QaimaMutaadhdhira {
                    masdar: "forge https://example.invalid".to_owned(),
                    sabab: "sahb/qaima.json answered 404".to_owned(),
                },
            },
        )?;

        let baad_daqiqa = waqt.checked_add(SignedDuration::from_secs(60))?;
        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya.clone(), baad_daqiqa, NAFIDHA);
        let rafd = muraqaba.rafd().ok_or("a reachable registry without a list must refuse")?;
        assert!(rafd.injilizi().contains("404"), "{}", rafd.injilizi());
        assert!(rafd.arabi().contains("404"), "{}", rafd.arabi());
        assert!(matches!(muraqaba.hala(), HalatQaima::LamTujlab { akhir: Some(_) }));

        // Long after the attempt, the finding is old news and no longer refuses;
        // the state still names it.
        let baad_yawm = waqt.checked_add(SignedDuration::from_hours(24))?;
        let muraqaba = QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya, baad_yawm, NAFIDHA);
        assert!(muraqaba.rafd().is_none());
        assert!(muraqaba.wasf_injilizi().contains("did not serve the revocation list"));
        Ok(())
    }

    #[test]
    fn al_mustawda_ghayr_al_mutah_la_yurfad() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let waqt = al_aan()?;
        QaimatSahb::sajjil_muhawala(
            &masarat,
            MuhawalatTajdid {
                waqt,
                natija: NatijatMuhawala::MustawdaGhayrMutah {
                    sabab: "dns error: no such host".to_owned(),
                },
            },
        )?;

        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), qaima_farigha(1)?, waqt, NAFIDHA);
        assert!(muraqaba.rafd().is_none(), "offline is not a refusal");
        assert!(matches!(muraqaba.hala(), HalatQaima::LamTujlab { akhir: Some(_) }));
        assert!(muraqaba.wasf_injilizi().contains("could not reach the registry"));
        assert!(muraqaba.wasf_arabi().contains("لم تبلغ المستودع"));
        Ok(())
    }

    #[test]
    fn al_madmuna_al_ajdad_taghlib_al_makhbaa_al_aqdam() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let waqt = al_aan()?;
        // Fetched at sequence 2 under an older build; this build ships 3.
        let _ = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &qaima_farigha(1)?,
            "forge",
            waqt,
        )?;

        let muraqaba =
            QaimaMuraqaba::iqra(&masarat, &malik().aam(), qaima_farigha(3)?, waqt, NAFIDHA);
        assert_eq!(muraqaba.qaima().tasalsul(), 3);
        Ok(())
    }

    #[test]
    fn akhir_muhawala_fashila_baad_najah_tunhi_al_tahdith() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let masarat = masarat_muaqqata(&masrah);
        let asliya = qaima_farigha(1)?;
        let waqt = al_aan()?;
        let _ = QaimatSahb::hadith_makhbaa(
            &masarat,
            &bayt_mulgha(2)?,
            &malik().aam(),
            &asliya,
            "forge",
            waqt,
        )?;
        let baad_saa = waqt.checked_add(SignedDuration::from_hours(1))?;
        QaimatSahb::sajjil_muhawala(
            &masarat,
            MuhawalatTajdid {
                waqt: baad_saa,
                natija: NatijatMuhawala::MustawdaGhayrMutah { sabab: "timed out".to_owned() },
            },
        )?;

        // A success an hour ago followed by a failure just now: the registry has
        // not confirmed anything recently, and the state must say so.
        let muraqaba = QaimaMuraqaba::iqra(&masarat, &malik().aam(), asliya, baad_saa, NAFIDHA);
        assert!(matches!(muraqaba.hala(), HalatQaima::Mukhazzana { julibat: Some(_), .. }));
        assert!(muraqaba.rafd().is_none());
        Ok(())
    }
}
