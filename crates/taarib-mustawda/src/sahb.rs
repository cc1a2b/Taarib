//! Refreshing the revocation list: the manifest names it, the chain serves it, the cache keeps it.

use jiff::Timestamp;
use taarib_aman::KhataAman;
use taarib_aman::qaimat_sahb::{MuhawalatTajdid, NatijatMuhawala, Qabul, QaimatSahb};
use taarib_khatm::MiftahAam;
use taarib_usus::masarat::Masarat;

use crate::fahras::BayanMustawda;
use crate::jalb::jalb_qaimat_sahb_maa_masdar;
use crate::masadir::{MASAR_BAYAN, SilsilatMasadir};

/// How one refresh of the revocation list ended.
///
/// Four of the five are answers rather than failures, and each is written into
/// the cache's refresh record before it is returned, so the gate that reads the
/// cache later — on a thread that must not wait for a network — can say what
/// the registry last answered instead of guessing from a timestamp.
#[derive(Debug)]
pub enum NatijatTajdid {
    /// A list was served, verified, accepted and cached.
    Najah {
        /// The list now in the cache.
        qaima: QaimatSahb,
        /// The source that served it.
        masdar: String,
    },
    /// No source answered the manifest: the machine is offline, or every
    /// configured source is down. Not a refusal anywhere.
    MustawdaGhayrMutah {
        /// What the last source said.
        sabab: String,
    },
    /// The manifest was served and the list was not, was unusable, or did not
    /// verify. The registry is up and its kill switch is missing, which every
    /// gate refuses until a later attempt finds it.
    QaimaMutaadhdhira {
        /// The source that answered the manifest.
        masdar: String,
        /// Why no list was accepted.
        sabab: String,
    },
    /// A verified list older than the one held was served; the held one stays.
    Aqdam {
        /// The source that served it.
        masdar: String,
        /// The sequence served.
        jadeeda: u64,
        /// The sequence held.
        asas: u64,
    },
    /// The cache itself failed: the lock could not be taken or the disk refused
    /// the write. Nothing about the registry is known from this.
    Khata(KhataAman),
}

impl NatijatTajdid {
    /// Whether a list was accepted and cached.
    #[must_use]
    pub const fn najah(&self) -> bool {
        matches!(self, Self::Najah { .. })
    }

    /// One line for the log.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Najah { qaima, masdar } => format!(
                "revocation list #{} accepted from {masdar} ({} revocation(s))",
                qaima.tasalsul(),
                qaima.adad()
            ),
            Self::MustawdaGhayrMutah { sabab } => {
                format!("no registry source answered the manifest: {sabab}")
            }
            Self::QaimaMutaadhdhira { masdar, sabab } => {
                format!("{masdar} served the manifest and not a usable revocation list: {sabab}")
            }
            Self::Aqdam { masdar, jadeeda, asas } => format!(
                "{masdar} served revocation list #{jadeeda}, older than the #{asas} held; kept the \
                 held one"
            ),
            Self::Khata(khata) => format!("the revocation cache failed: {khata}"),
        }
    }
}

/// Fetches the manifest and the revocation list it names, and updates the cache.
///
/// `asliya` is the list compiled into the calling build; it floors the sequence
/// so that no source can serve an older list than the build shipped, and it is
/// what the cache falls back to when a fetched list is refused. Every outcome
/// is recorded beside the cache; see [`NatijatTajdid`].
pub async fn jaddid_qaimat_sahb(
    silsila: &SilsilatMasadir,
    masarat: &Masarat,
    miftah_malik: &MiftahAam,
    asliya: &QaimatSahb,
    al_aan: Timestamp,
) -> NatijatTajdid {
    let (bayt, masdar) = match silsila.jalb_maa_masdar(MASAR_BAYAN).await {
        Ok(majlub) => majlub,
        Err(khata) => {
            let sabab = khata.to_string();
            sajjil(masarat, al_aan, NatijatMuhawala::MustawdaGhayrMutah { sabab: sabab.clone() })
                .await;
            return NatijatTajdid::MustawdaGhayrMutah { sabab };
        }
    };
    // A source that answers the manifest with something unreadable is a
    // reachable registry with no usable kill switch, not an unreachable one:
    // treating it as offline would let a source suppress the list by serving
    // garbage in front of it.
    let bayan = match BayanMustawda::min_bayt(&bayt, None) {
        Ok(bayan) => bayan,
        Err(khata) => {
            let sabab = khata.to_string();
            sajjil(
                masarat,
                al_aan,
                NatijatMuhawala::QaimaMutaadhdhira { masdar: masdar.clone(), sabab: sabab.clone() },
            )
            .await;
            return NatijatTajdid::QaimaMutaadhdhira { masdar, sabab };
        }
    };
    jaddid_qaimat_sahb_bi_bayan(silsila, &bayan, &masdar, masarat, miftah_malik, asliya, al_aan)
        .await
}

/// As [`jaddid_qaimat_sahb`], for a caller that has already fetched the
/// manifest and knows which source served it.
pub async fn jaddid_qaimat_sahb_bi_bayan(
    silsila: &SilsilatMasadir,
    bayan: &BayanMustawda,
    masdar_bayan: &str,
    masarat: &Masarat,
    miftah_malik: &MiftahAam,
    asliya: &QaimatSahb,
    al_aan: Timestamp,
) -> NatijatTajdid {
    let (bayt, masdar) = match jalb_qaimat_sahb_maa_masdar(silsila, bayan).await {
        Ok(majlub) => majlub,
        Err(khata) => {
            let sabab = khata.to_string();
            sajjil(
                masarat,
                al_aan,
                NatijatMuhawala::QaimaMutaadhdhira {
                    masdar: masdar_bayan.to_owned(),
                    sabab: sabab.clone(),
                },
            )
            .await;
            return NatijatTajdid::QaimaMutaadhdhira { masdar: masdar_bayan.to_owned(), sabab };
        }
    };

    let masarat_lil_kitaba = masarat.clone();
    let miftah = miftah_malik.clone();
    let asliya = asliya.clone();
    let masdar_lil_kitaba = masdar.clone();
    let natija = tokio::task::spawn_blocking(move || {
        QaimatSahb::hadith_makhbaa(
            &masarat_lil_kitaba,
            &bayt,
            &miftah,
            &asliya,
            &masdar_lil_kitaba,
            al_aan,
        )
    })
    .await;

    match natija {
        Ok(Ok(Qabul::Qubilat(qaima))) => NatijatTajdid::Najah { qaima, masdar },
        Ok(Ok(Qabul::Aqdam { jadeeda, asas })) => NatijatTajdid::Aqdam { masdar, jadeeda, asas },
        Ok(Ok(Qabul::Marfuda(khata))) => {
            NatijatTajdid::QaimaMutaadhdhira { masdar, sabab: khata.to_string() }
        }
        Ok(Err(khata)) => NatijatTajdid::Khata(khata),
        Err(khata) => NatijatTajdid::Khata(KhataAman::QaimatSahbFashila {
            amal: "refreshed",
            sabab: format!("the cache task did not finish: {khata}"),
        }),
    }
}

/// Records an attempt that produced no bytes to judge. A record that cannot be
/// written is logged and nothing more: the outcome is still returned to the
/// caller, and the next gate reads whatever record there is.
async fn sajjil(masarat: &Masarat, al_aan: Timestamp, natija: NatijatMuhawala) {
    let masarat = masarat.clone();
    let muhawala = MuhawalatTajdid { waqt: al_aan, natija };
    match tokio::task::spawn_blocking(move || QaimatSahb::sajjil_muhawala(&masarat, muhawala))
        .await
    {
        Ok(Ok(())) => {}
        Ok(Err(khata)) => {
            tracing::warn!(khata = %khata, "the revocation refresh attempt was not recorded");
        }
        Err(khata) => {
            tracing::warn!(khata = %khata, "the revocation refresh record task did not finish");
        }
    }
}
