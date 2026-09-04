//! Publishing: the owner's key signs, then staging is promoted and the index moves.

use taarib_khatm::{MiftahKhass, MudaqqiqEd25519};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_ruqaa::tawqee::{DawrMiftah, KutlatTawqee, Khwarizmiya};
use taarib_ruqaa::muhadhah::BaytMuhadhah;

use crate::hawiya::SalahiyatMalik;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};

/// Which step of the publish sequence a run reached.
///
/// An interruption resumes from the recorded step; a failure rolls the staging
/// promotion back, so a binary with no metadata and metadata pointing at
/// nothing are both unreachable states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarhalatNashr {
    /// The package has been signed with the owner's key.
    Wuqqia,
    /// The binary has been promoted from staging to the public release area.
    Ruqqia,
    /// The metadata record is in its shard.
    Fuhrisa,
    /// The shard hash and the global manifest are updated.
    Buyyina,
    /// The audit entry is appended.
    Suijjila,
}

impl MarhalatNashr {
    /// The step name a resume or a rollback reports.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Wuqqia => "signing",
            Self::Ruqqia => "promoting the binary",
            Self::Fuhrisa => "writing the metadata record",
            Self::Buyyina => "updating the manifest",
            Self::Suijjila => "appending the audit entry",
        }
    }
}

/// The provenance recorded inside a published patch.
///
/// Travels with the patch permanently: a client reading a package can name who
/// approved it and when, without consulting the registry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ItimadManshur {
    /// The reviewer.
    pub murajii: MusahimId,
    /// When it was approved, RFC 3339, supplied by the caller.
    pub waqt: String,
    /// The owner key that signed it.
    pub miftah: String,
}

/// A package sealed with the owner's key.
///
/// No public constructor, no public fields, not `Clone`, no `Deserialize`. The
/// only way to obtain one is [`waqqi`], which requires the owner authority and
/// the owner's private key. Promotion and indexing take one, so a patch cannot
/// reach the public release area unsigned.
#[derive(Debug)]
pub struct HuzmaMakhtuma {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    luba: LubaId,
    basma: Basma,
    itimad: ItimadManshur,
    bayt: Vec<u8>,
}

impl HuzmaMakhtuma {
    /// The patch lineage.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The revision.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// The game it targets, which decides the shard.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// The sealed package's content hash.
    #[must_use]
    pub const fn basma(&self) -> Basma {
        self.basma
    }

    /// The provenance to record in the metadata.
    #[must_use]
    pub const fn itimad(&self) -> &ItimadManshur {
        &self.itimad
    }

    /// The sealed bytes, for promotion.
    #[must_use]
    pub fn bayt(&self) -> &[u8] {
        &self.bayt
    }
}

/// Signs an approved package with the owner's key.
///
/// The only source of [`HuzmaMakhtuma`]. `khass` must be the owner's key: the
/// authority token proves this session holds it, and
/// `taarib_ruqaa::katib::khatm` re-verifies the block against the container's
/// own hash before writing a byte, so a block made for another package cannot
/// seal this one.
///
/// `waqt` is an RFC 3339 timestamp and `lahza` its Unix-seconds form, both
/// supplied by the caller; nothing here reads a clock.
///
/// # Errors
///
/// [`KhataTaqdeem::NashrFashil`] when `khass` is not the owner's key or the
/// container refuses the seal.
pub fn waqqi(
    salahiya: &SalahiyatMalik,
    khass: &MiftahKhass,
    murajii: MusahimId,
    waqt: String,
    lahza: i64,
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    luba: LubaId,
    mut bayt_huzma: BaytMuhadhah,
) -> NatijatTaqdeem<HuzmaMakhtuma> {
    let miftah = khass.aam().bayt();
    if &miftah != salahiya.miftah_aam() {
        return Err(KhataTaqdeem::NashrFashil {
            marhala: MarhalatNashr::Wuqqia.ism(),
            sabab: "the signing key is not the owner key this session proved".to_owned(),
        });
    }

    let basma = huzma_basma(&bayt_huzma).ok_or_else(|| KhataTaqdeem::NashrFashil {
        marhala: MarhalatNashr::Wuqqia.ism(),
        sabab: "the package header could not be read".to_owned(),
    })?;

    let mut kutla = KutlatTawqee {
        dawr: DawrMiftah::Malik,
        khwarizmiya: Khwarizmiya::Ed25519,
        waqt: lahza,
        miftah,
        tawqee: [0; 64],
    };
    kutla.tawqee = khass.waqqi(&kutla.risala(&basma));

    taarib_ruqaa::katib::khatm(bayt_huzma.bayt_mut(), &kutla, &MudaqqiqEd25519).map_err(
        |khata| KhataTaqdeem::NashrFashil {
            marhala: MarhalatNashr::Wuqqia.ism(),
            sabab: khata.to_string(),
        },
    )?;

    Ok(HuzmaMakhtuma {
        ruqaa,
        murajaa,
        luba,
        basma: Basma::min_bayt(basma),
        itimad: ItimadManshur { murajii, waqt, miftah: hex_32(&miftah) },
        bayt: bayt_huzma.ila_shuaa(),
    })
}

/// The container's content hash, read back from the built header.
fn huzma_basma(bayt: &BaytMuhadhah) -> Option<[u8; 32]> {
    taarib_ruqaa::qari::Ruqaa::iftah(bayt.bayt()).ok().map(|ruqaa| ruqaa.tarwisa().basma)
}

fn hex_32(bayt: &[u8; 32]) -> String {
    let mut nass = String::with_capacity(64);
    for wahid in bayt {
        use std::fmt::Write as _;
        let _ = write!(nass, "{wahid:02x}");
    }
    nass
}

/// Where a publish run had reached, so an interruption resumes rather than
/// restarts.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqaddumNashr {
    /// The patch being published.
    pub ruqaa: RuqaaId,
    /// Which revision.
    pub murajaa: RuqaaRevision,
    /// The last step that completed.
    pub akhir: Option<MarhalatNashr>,
}

impl TaqaddumNashr {
    /// A run that has not started.
    #[must_use]
    pub const fn jadeed(ruqaa: RuqaaId, murajaa: RuqaaRevision) -> Self {
        Self { ruqaa, murajaa, akhir: None }
    }

    /// Whether a step still has to run.
    #[must_use]
    pub fn yabqa(&self, marhala: MarhalatNashr) -> bool {
        self.akhir.is_none_or(|akhir| akhir < marhala)
    }

    /// Records a completed step.
    pub const fn tamma(&mut self, marhala: MarhalatNashr) {
        self.akhir = Some(marhala);
    }

    /// Whether the sequence finished.
    #[must_use]
    pub fn muktamil(&self) -> bool {
        self.akhir == Some(MarhalatNashr::Suijjila)
    }
}
