//! المسوّدة — the resumable submission draft, and the transitions that move it.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::nass::NassId;
use taarib_mustalahat::ruqaa::{HalatRuqaa, RukhsaRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_mustalahat::taghtiya::Taghtiya;
use taarib_tarqee::bawwaba::ShahadatBawwaba;
use taarib_tarqee::bayan::BayanHuzma;
use taarib_tarqee::fuhusat::WasfHuzma;
use taarib_tarqee::irtibat::IrtibatBina;
use taarib_tarqee::mustawrid::SighatIstirad;
use taarib_tarqee::taghtiya_ruqaa::TaqrirTaghtiya;
use taarib_tarqee::taqrir_tajawuz::TaqrirTajawuz;

use crate::bawwaba::IjtiyazTaqdeem;
use crate::hawiya::HawiyatMusahim;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};
use crate::muraja::NawIjraMuraja;

/// The draft schema this build writes and reads.
pub const ISDAR_MUSAWWADA: u32 = 1;

/// The directory drafts live in, under the platform data root.
pub const MUJALLAD_MUSAWWADAT: &str = "taqdeem";

/// The extension a saved draft carries.
pub const LAHIQAT_MUSAWWADA: &str = "json";

/// The extension the half-written copy carries until the rename completes.
pub const LAHIQAT_MUAQQATA: &str = "jadeed";

/// Where a submission stands.
///
/// Reached only through the transitions on [`Musawwada`], each of which consumes
/// the submission and returns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "hala", rename_all = "snake_case")]
pub enum HalatTaqdeem {
    /// A local draft, visible to nobody else.
    Musawwada,
    /// Sent, and waiting in the owner's queue.
    Muqaddama {
        /// When, RFC 3339.
        waqt: String,
    },
    /// The owner has opened it.
    QaydMuraja {
        /// When, RFC 3339.
        waqt: String,
    },
    /// Returned for revision, on a new revision of the same submission.
    MatlubTaadil {
        /// When, RFC 3339.
        waqt: String,
        /// The written summary of what to change.
        mulakhkhas: String,
        /// The strings the summary points at.
        nusus: Vec<NassId>,
    },
    /// Approved; the publishing sequence is running.
    MawafaqYunshar {
        /// When, RFC 3339.
        waqt: String,
    },
    /// Published.
    Manshura {
        /// When, RFC 3339.
        waqt: String,
        /// The revision that was published.
        murajaa: RuqaaRevision,
    },
    /// Rejected, with a written reason.
    Marfuda {
        /// When, RFC 3339.
        waqt: String,
        /// Why.
        sabab: String,
    },
    /// Withdrawn by the contributor.
    Mashuba {
        /// When, RFC 3339.
        waqt: String,
    },
    /// Revoked by the owner after publication, with a written reason.
    Masbuba {
        /// When, RFC 3339.
        waqt: String,
        /// Why, as the owner wrote it.
        sabab: String,
    },
}

impl HalatTaqdeem {
    /// The state's stable key, for messages and for the transition record.
    #[must_use]
    pub const fn ism(&self) -> &'static str {
        match self {
            Self::Musawwada => "musawwada",
            Self::Muqaddama { .. } => "muqaddama",
            Self::QaydMuraja { .. } => "qayd_muraja",
            Self::MatlubTaadil { .. } => "matlub_taadil",
            Self::MawafaqYunshar { .. } => "mawafaq_yunshar",
            Self::Manshura { .. } => "manshura",
            Self::Marfuda { .. } => "marfuda",
            Self::Mashuba { .. } => "mashuba",
            Self::Masbuba { .. } => "masbuba",
        }
    }

    /// The shared patch state this one shows as, so the Contributions screen and
    /// the owner's queue keep one vocabulary.
    #[must_use]
    pub const fn halat_ruqaa(&self) -> HalatRuqaa {
        match self {
            Self::Musawwada => HalatRuqaa::Musawwada,
            Self::Muqaddama { .. } => HalatRuqaa::Muqaddama,
            Self::QaydMuraja { .. } => HalatRuqaa::QaydMuraja,
            Self::MatlubTaadil { .. } => HalatRuqaa::MatlubTaadil,
            Self::MawafaqYunshar { .. } => HalatRuqaa::MawafaqYunshar,
            Self::Manshura { .. } => HalatRuqaa::Manshura,
            Self::Marfuda { .. } => HalatRuqaa::Marfuda,
            Self::Mashuba { .. } => HalatRuqaa::Mashuba,
            Self::Masbuba { .. } => HalatRuqaa::Masbuba,
        }
    }

    /// Whether the contributor may still edit the submission.
    #[must_use]
    pub const fn qabila_lil_tahreer(&self) -> bool {
        matches!(self, Self::Musawwada | Self::MatlubTaadil { .. })
    }

    /// Whether the submission is waiting on the owner.
    #[must_use]
    pub const fn fi_intizar_almalik(&self) -> bool {
        matches!(self, Self::Muqaddama { .. } | Self::QaydMuraja { .. })
    }

    /// Whether nothing further happens to the submission.
    #[must_use]
    pub const fn nihaiya(&self) -> bool {
        matches!(
            self,
            Self::Manshura { .. }
                | Self::Marfuda { .. }
                | Self::Mashuba { .. }
                | Self::Masbuba { .. }
        )
    }

    /// Whether this state accepts one owner decision.
    ///
    /// The single answer to "is this control worth offering", and the same
    /// answer the transitions below gate on — each of them asks this rather
    /// than restating its own condition, so a decision the console offers and
    /// a transition that refuses it cannot become two different rules. An
    /// offered decision that the state forbids is how the console came to
    /// present *Approve* on an already published submission, which answered
    /// `TAARIB-E-9068` when pressed.
    #[must_use]
    pub const fn yaqbal(&self, ijra: NawIjraMuraja) -> bool {
        match ijra {
            // A comment moves nothing, so no transition guards it; what it
            // needs is somebody to address, and a local draft has not reached
            // the owner yet.
            NawIjraMuraja::Taaliq => !matches!(self, Self::Musawwada),
            NawIjraMuraja::TalabTaadil | NawIjraMuraja::Rafd | NawIjraMuraja::Iaatimad => {
                self.fi_intizar_almalik()
            },
            // Approved counts as well as published. Approval seals the package
            // and a separate cast puts it in the registry; an approval the
            // owner changed their mind about before that cast is past every
            // other transition, so without this it would have no way out.
            NawIjraMuraja::Sahb => {
                matches!(self, Self::Manshura { .. } | Self::MawafaqYunshar { .. })
            },
        }
    }

    /// Every owner decision this state accepts, in the order the console lays
    /// its controls out.
    #[must_use]
    pub fn afal_mutaha(&self) -> Vec<NawIjraMuraja> {
        NawIjraMuraja::KULL
            .into_iter()
            .filter(|ijra| self.yaqbal(*ijra))
            .collect()
    }

    /// When the submission entered this state, where it was recorded.
    #[must_use]
    pub fn waqt(&self) -> Option<&str> {
        match self {
            Self::Musawwada => None,
            Self::Muqaddama { waqt }
            | Self::QaydMuraja { waqt }
            | Self::MatlubTaadil { waqt, .. }
            | Self::MawafaqYunshar { waqt }
            | Self::Manshura { waqt, .. }
            | Self::Marfuda { waqt, .. }
            | Self::Mashuba { waqt }
            | Self::Masbuba { waqt, .. } => Some(waqt),
        }
    }

    /// The label the Contributions screen shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(&self) -> &'static str {
        match self {
            Self::Masbuba { .. } => "مسحوبة من التداول",
            Self::Musawwada
            | Self::Muqaddama { .. }
            | Self::QaydMuraja { .. }
            | Self::MatlubTaadil { .. }
            | Self::MawafaqYunshar { .. }
            | Self::Manshura { .. }
            | Self::Marfuda { .. }
            | Self::Mashuba { .. } => self.halat_ruqaa().wasf_arabi(),
        }
    }

    /// The same, in English.
    ///
    /// Beside the Arabic because a refused transition names the state it is in
    /// inside a sentence a person reads, and the slug this enum is keyed by is
    /// not a word in either language.
    #[must_use]
    pub const fn wasf_injilizi(&self) -> &'static str {
        match self {
            Self::Musawwada => "a local draft",
            Self::Muqaddama { .. } => "submitted",
            Self::QaydMuraja { .. } => "under review",
            Self::MatlubTaadil { .. } => "returned for revision",
            Self::MawafaqYunshar { .. } => "approved, publishing",
            Self::Manshura { .. } => "published",
            Self::Marfuda { .. } => "rejected",
            Self::Mashuba { .. } => "withdrawn by its contributor",
            Self::Masbuba { .. } => "revoked after publication",
        }
    }
}

/// One transition, as the submission recorded it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaydMusawwada {
    /// The revision the submission was on when it moved.
    pub murajaa: RuqaaRevision,
    /// The state it moved into.
    pub ila: HalatTaqdeem,
    /// When, RFC 3339, as the caller supplied it.
    pub waqt: String,
}

/// Why a transition did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SababManIntiqal {
    /// The current state does not allow it.
    HalaGhayrMulaima,
    /// The transition needs something the caller did not supply.
    BayanNaqis {
        /// Which field.
        haql: &'static str,
    },
    /// The gate proof was minted for another submission or another revision.
    IjtiyazLaYakhussuha,
    /// An import mapping is still waiting on the contributor.
    IrtibatMuallaq {
        /// How many.
        adad: usize,
    },
}

/// A refused transition, carrying the submission it did not move.
///
/// Both states are kept whole rather than as their two keys: whoever shows
/// this refusal has to word it, and `manshura` and `mawafaq_yunshar` are record
/// keys rather than anything a person reads.
#[derive(Debug, thiserror::Error)]
#[error("a submission in {} was not moved to {}", .min.ism(), .ila.ism())]
pub struct IntiqalMarfud {
    musawwada: Box<Musawwada>,
    min: HalatTaqdeem,
    ila: HalatTaqdeem,
    sabab: SababManIntiqal,
}

impl IntiqalMarfud {
    /// The state the submission is still in.
    #[must_use]
    pub const fn min(&self) -> &'static str {
        self.min.ism()
    }

    /// That state, in Arabic.
    #[must_use]
    pub const fn min_arabi(&self) -> &'static str {
        self.min.wasf_arabi()
    }

    /// The same, in English.
    #[must_use]
    pub const fn min_injilizi(&self) -> &'static str {
        self.min.wasf_injilizi()
    }

    /// The state it was asked to move to.
    #[must_use]
    pub const fn ila(&self) -> &'static str {
        self.ila.ism()
    }

    /// That state, in Arabic.
    #[must_use]
    pub const fn ila_arabi(&self) -> &'static str {
        self.ila.wasf_arabi()
    }

    /// The same, in English.
    #[must_use]
    pub const fn ila_injilizi(&self) -> &'static str {
        self.ila.wasf_injilizi()
    }

    /// Why it did not move.
    #[must_use]
    pub const fn sabab(&self) -> SababManIntiqal {
        self.sabab
    }

    /// The submission, unchanged.
    #[must_use]
    pub fn istaridd(self) -> Musawwada {
        *self.musawwada
    }
}

/// An import entry that matched nothing, and the strings it might belong to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrtibatMuallaq {
    /// The entry's key, or its source text where the format has no key.
    pub miftah: String,
    /// The line it came from.
    pub satr: usize,
    /// The strings the importer offered as candidates.
    pub murashshahat: Vec<NassId>,
}

/// What one import brought in, and what it left for the contributor to decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SijillIstirad {
    /// The format the importer read.
    pub sigha: SighatIstirad,
    /// The file it read.
    pub masdar: PathBuf,
    /// How many entries matched a string in the project.
    pub mutatabiqa: usize,
    /// How many matched nothing.
    pub ghayr_mutatabiqa: usize,
    /// The entries still waiting on a decision.
    pub muallaqa: Vec<IrtibatMuallaq>,
}

/// How many strings a submission covers, as the compile counted them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdadNusus {
    /// Every extracted string.
    pub majmu: u32,
    /// Strings with a translation.
    pub mutarjam: u32,
    /// Strings a human review approved.
    pub muakkad: u32,
    /// Strings the package itself carries.
    pub fil_huzma: usize,
}

/// Everything both entry points need.
#[derive(Debug)]
pub struct BidayatMusawwada<'a> {
    /// The manifest the compile wrote, which carries the patch identity, the
    /// declaration, the binding, the gate certificate and both reports.
    pub bayan: &'a BayanHuzma,
    /// Taarib's identity for the game.
    pub luba: LubaId,
    /// Every launcher identity the game is known by.
    pub masadir: Vec<MasdarLuba>,
    /// Fingerprints of the original files the translation was built from.
    pub basmat_asliya: Vec<Basma>,
    /// Where the compiled package is on this machine.
    pub masar_huzma: PathBuf,
    /// The package's own content fingerprint.
    pub basmat_huzma: Basma,
    /// Its size in bytes.
    pub hajm_huzma: u64,
    /// Who is submitting, and how they want to be credited.
    pub musahim: HawiyatMusahim,
    /// The description shown on the listing.
    pub sharh: String,
    /// The changelog for this revision.
    pub taghyeerat: String,
    /// When the draft was started, RFC 3339, supplied by the caller.
    pub waqt: String,
}

/// What an edit does to the credit line, which the contributor may also clear.
///
/// The credit line is the one editable field that is itself optional, so an
/// edit has to say which of two different things it means: leaving the field
/// out of the edit keeps whatever line the submission carries, while
/// [`Hadhf`](Self::Hadhf) is the deliberate instruction to carry none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TahreerItimad {
    /// Credit the contributor under this line.
    Jadeed(String),
    /// Carry no credit line at all.
    Hadhf,
}

/// The fields a contributor may change while the submission is still theirs.
///
/// A field left `None` is a field the edit does not touch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TahreerMusawwada {
    /// A new title.
    pub unwan: Option<String>,
    /// A new description.
    pub sharh: Option<String>,
    /// A new changelog.
    pub taghyeerat: Option<String>,
    /// A new licence.
    pub rukhsa: Option<RukhsaRuqaa>,
    /// What becomes of the credit line.
    pub itimad: Option<TahreerItimad>,
}

/// A handle that exists only while the submission is still the contributor's.
#[derive(Debug)]
pub struct MuharrirMusawwada<'a> {
    musawwada: &'a mut Musawwada,
}

impl MuharrirMusawwada<'_> {
    /// Applies an edit, all of it or none of it.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::BayanNaqis`] naming the first field a blank replacement
    /// was given for.
    pub fn tabbiq(&mut self, tahreer: TahreerMusawwada) -> NatijatTaqdeem<()> {
        if let Some(unwan) = tahreer.unwan.as_deref() {
            matlub(unwan, "a title")?;
        }
        if let Some(sharh) = tahreer.sharh.as_deref() {
            matlub(sharh, "a description")?;
        }
        if let Some(RukhsaRuqaa::Ukhra { ism }) = tahreer.rukhsa.as_ref() {
            matlub(ism, "the name of the licence")?;
        }
        if let Some(TahreerItimad::Jadeed(itimad)) = tahreer.itimad.as_ref() {
            matlub(itimad, "a credit line")?;
        }

        if let Some(unwan) = tahreer.unwan {
            self.musawwada.wasf.unwan = unwan;
        }
        if let Some(sharh) = tahreer.sharh {
            self.musawwada.sharh = sharh;
        }
        if let Some(taghyeerat) = tahreer.taghyeerat {
            self.musawwada.taghyeerat = taghyeerat;
        }
        if let Some(rukhsa) = tahreer.rukhsa {
            self.musawwada.wasf.rukhsa = rukhsa;
        }
        if let Some(itimad) = tahreer.itimad {
            self.musawwada.musahim.itimad = match itimad {
                TahreerItimad::Jadeed(nass) => Some(nass),
                TahreerItimad::Hadhf => None,
            };
        }
        Ok(())
    }

    /// Marks one import mapping as decided, and answers how many it cleared.
    pub fn hall_irtibat(&mut self, miftah: &str) -> usize {
        let mut mahlul = 0_usize;
        for sijill in &mut self.musawwada.istirad {
            let qabl = sijill.muallaqa.len();
            sijill.muallaqa.retain(|muallaq| muallaq.miftah != miftah);
            mahlul = mahlul.saturating_add(qabl.saturating_sub(sijill.muallaqa.len()));
        }
        mahlul
    }

    /// Marks every remaining import mapping as decided, and answers how many.
    pub fn hall_kull_irtibatat(&mut self) -> usize {
        let mut mahlul = 0_usize;
        for sijill in &mut self.musawwada.istirad {
            mahlul = mahlul.saturating_add(sijill.muallaqa.len());
            sijill.muallaqa.clear();
        }
        mahlul
    }
}

/// A saved draft file that would not load, and the failure that stopped it.
///
/// Not an absence: the file is on disk and the contributor's work is in it.
/// The whole [`KhataTaqdeem`] is kept rather than a sentence, so the code, the
/// Arabic and English wordings and the one next step all survive to whoever
/// shows it.
#[derive(Debug)]
pub struct MusawwadaMutaadhira {
    /// The file that would not load.
    pub masar: PathBuf,
    /// What failed.
    pub khata: KhataTaqdeem,
}

/// Every saved draft that read, and every one that did not.
///
/// The same shape the library scan uses for the same problem: one unreadable
/// file names itself instead of costing the user everything beside it.
#[derive(Debug)]
pub struct SijillMusawwadat {
    /// The drafts that read, oldest first.
    pub musawwadat: Vec<Musawwada>,
    /// The files that did not, by path, each naming its own failure.
    pub mutaadhira: Vec<MusawwadaMutaadhira>,
}

/// A submission: what it targets, what it carries, and where it stands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Musawwada {
    isdar: u32,
    id: RuqaaId,
    murajaa: RuqaaRevision,
    hala: HalatTaqdeem,
    tareekh: Vec<QaydMusawwada>,
    luba: LubaId,
    masadir: Vec<MasdarLuba>,
    irtibat: IrtibatBina,
    basmat_asliya: Vec<Basma>,
    masar_huzma: PathBuf,
    basmat_huzma: Basma,
    hajm_huzma: u64,
    bayan: serde_json::Value,
    wasf: WasfHuzma,
    shahada: ShahadatBawwaba,
    musahim: HawiyatMusahim,
    sharh: String,
    taghyeerat: String,
    taghtiya: Taghtiya,
    taqrir_taghtiya: TaqrirTaghtiya,
    tajawuz: TaqrirTajawuz,
    istirad: Vec<SijillIstirad>,
    ansha: String,
}

impl Musawwada {
    /// Starts a draft from a translation finished in Taarib's own workspace.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::BayanNaqis`] when the description, the timestamp, the
    /// package path or the game's launcher identities are missing, and
    /// [`KhataTaqdeem::KhataMalaf`] when the manifest will not serialize.
    pub fn min_warsha(bidaya: BidayatMusawwada<'_>) -> NatijatTaqdeem<Self> {
        Self::min_bidaya(bidaya, Vec::new())
    }

    /// Starts a draft from a translation brought in through an importer.
    ///
    /// # Errors
    ///
    /// Whatever [`Musawwada::min_warsha`] refuses, and
    /// [`KhataTaqdeem::BayanNaqis`] when no import record is supplied — a draft
    /// that came through an importer and records no import cannot state what it
    /// left unresolved.
    pub fn min_istirad(
        bidaya: BidayatMusawwada<'_>,
        istirad: Vec<SijillIstirad>,
    ) -> NatijatTaqdeem<Self> {
        if istirad.is_empty() {
            return Err(KhataTaqdeem::BayanNaqis {
                haql: "at least one import record",
            });
        }
        Self::min_bidaya(bidaya, istirad)
    }

    fn min_bidaya(
        bidaya: BidayatMusawwada<'_>,
        istirad: Vec<SijillIstirad>,
    ) -> NatijatTaqdeem<Self> {
        matlub(&bidaya.sharh, "a description")?;
        matlub(&bidaya.waqt, "a timestamp")?;
        if bidaya.masar_huzma.as_os_str().is_empty() {
            return Err(KhataTaqdeem::BayanNaqis {
                haql: "the compiled package",
            });
        }
        if bidaya.masadir.is_empty() {
            return Err(KhataTaqdeem::BayanNaqis {
                haql: "at least one launcher identity for the game",
            });
        }

        let bayan = serde_json::to_value(bidaya.bayan).map_err(|sabab| {
            khata_malaf(
                &bidaya.masar_huzma,
                "recording the package manifest",
                sabab.into(),
            )
        })?;

        let hala = HalatTaqdeem::Musawwada;
        let qayd = QaydMusawwada {
            murajaa: bidaya.bayan.murajaa,
            ila: hala.clone(),
            waqt: bidaya.waqt.clone(),
        };
        Ok(Self {
            isdar: ISDAR_MUSAWWADA,
            id: bidaya.bayan.id,
            murajaa: bidaya.bayan.murajaa,
            hala,
            tareekh: vec![qayd],
            luba: bidaya.luba,
            masadir: bidaya.masadir,
            irtibat: bidaya.bayan.irtibat.clone(),
            basmat_asliya: bidaya.basmat_asliya,
            masar_huzma: bidaya.masar_huzma,
            basmat_huzma: bidaya.basmat_huzma,
            hajm_huzma: bidaya.hajm_huzma,
            bayan,
            wasf: bidaya.bayan.wasf.clone(),
            shahada: bidaya.bayan.bawwaba.clone(),
            musahim: bidaya.musahim,
            sharh: bidaya.sharh,
            taghyeerat: bidaya.taghyeerat,
            taghtiya: bidaya.bayan.taghtiya.kulli,
            taqrir_taghtiya: bidaya.bayan.taghtiya.clone(),
            tajawuz: bidaya.bayan.tajawuz.clone(),
            istirad,
            ansha: bidaya.waqt,
        })
    }

    /// The patch lineage this submission belongs to.
    #[must_use]
    pub const fn id(&self) -> RuqaaId {
        self.id
    }

    /// Which revision of it this is.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// Where it stands.
    #[must_use]
    pub const fn hala(&self) -> &HalatTaqdeem {
        &self.hala
    }

    /// Every transition it has made, oldest first.
    #[must_use]
    pub fn tareekh(&self) -> &[QaydMusawwada] {
        &self.tareekh
    }

    /// The game it targets.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// Every launcher identity that game is known by.
    #[must_use]
    pub fn masadir(&self) -> &[MasdarLuba] {
        &self.masadir
    }

    /// The builds and fonts the package is bound to.
    #[must_use]
    pub const fn irtibat(&self) -> &IrtibatBina {
        &self.irtibat
    }

    /// Fingerprints of the original files the translation was built from.
    #[must_use]
    pub fn basmat_asliya(&self) -> &[Basma] {
        &self.basmat_asliya
    }

    /// Where the compiled package is on this machine.
    #[must_use]
    pub fn masar_huzma(&self) -> &Path {
        &self.masar_huzma
    }

    /// The package's content fingerprint.
    #[must_use]
    pub const fn basmat_huzma(&self) -> Basma {
        self.basmat_huzma
    }

    /// Its size in bytes.
    #[must_use]
    pub const fn hajm_huzma(&self) -> u64 {
        self.hajm_huzma
    }

    /// The package manifest, as the data it was written as.
    #[must_use]
    pub const fn bayan(&self) -> &serde_json::Value {
        &self.bayan
    }

    /// What the contributor declared: title, licence, method, names.
    #[must_use]
    pub const fn wasf(&self) -> &WasfHuzma {
        &self.wasf
    }

    /// What the asset gate certified about the package.
    #[must_use]
    pub const fn shahada(&self) -> &ShahadatBawwaba {
        &self.shahada
    }

    /// Who is submitting, and their credit line.
    #[must_use]
    pub const fn musahim(&self) -> &HawiyatMusahim {
        &self.musahim
    }

    /// The description shown on the listing.
    #[must_use]
    pub fn sharh(&self) -> &str {
        &self.sharh
    }

    /// The changelog for this revision.
    #[must_use]
    pub fn taghyeerat(&self) -> &str {
        &self.taghyeerat
    }

    /// The declared translation method.
    #[must_use]
    pub const fn tareeqa(&self) -> TareeqaTarjama {
        self.wasf.tareeqa
    }

    /// Coverage, as the compile measured it.
    #[must_use]
    pub const fn taghtiya(&self) -> Taghtiya {
        self.taghtiya
    }

    /// The full coverage report.
    #[must_use]
    pub const fn taqrir_taghtiya(&self) -> &TaqrirTaghtiya {
        &self.taqrir_taghtiya
    }

    /// The overflow report.
    #[must_use]
    pub const fn tajawuz(&self) -> &TaqrirTajawuz {
        &self.tajawuz
    }

    /// What every import brought in.
    #[must_use]
    pub fn istirad(&self) -> &[SijillIstirad] {
        &self.istirad
    }

    /// When the draft was started, RFC 3339.
    #[must_use]
    pub fn ansha(&self) -> &str {
        &self.ansha
    }

    /// How many strings this submission covers.
    #[must_use]
    pub const fn adad_nusus(&self) -> AdadNusus {
        AdadNusus {
            majmu: self.taghtiya.majmu,
            mutarjam: self.taghtiya.mutarjam,
            muakkad: self.taghtiya.muakkad,
            fil_huzma: self.shahada.nusus,
        }
    }

    /// Every import mapping still waiting on the contributor.
    #[must_use]
    pub fn irtibatat_muallaqa(&self) -> Vec<&IrtibatMuallaq> {
        self.istirad
            .iter()
            .flat_map(|sijill| sijill.muallaqa.iter())
            .collect()
    }

    /// How many import mappings are still waiting.
    #[must_use]
    pub fn adad_irtibatat_muallaqa(&self) -> usize {
        self.istirad.iter().fold(0_usize, |majmu, sijill| {
            majmu.saturating_add(sijill.muallaqa.len())
        })
    }

    /// The edit handle, while the submission is still the contributor's.
    #[must_use]
    pub const fn tahreer(&mut self) -> Option<MuharrirMusawwada<'_>> {
        if !self.hala.qabila_lil_tahreer() {
            return None;
        }
        Some(MuharrirMusawwada { musawwada: self })
    }

    /// Records the submission as sent, against the proof the gate minted for it.
    ///
    /// # Errors
    ///
    /// The submission unchanged, with [`SababManIntiqal`] naming which of the
    /// state, the proof, an unresolved import mapping or the timestamp refused
    /// it.
    pub fn ursilat(self, ijtiyaz: &IjtiyazTaqdeem, waqt: &str) -> Result<Self, IntiqalMarfud> {
        let ila = HalatTaqdeem::Muqaddama {
            waqt: waqt.to_owned(),
        };
        if !ijtiyaz.yakhuss(&self) {
            return Err(self.rafd(ila, SababManIntiqal::IjtiyazLaYakhussuha));
        }
        let muallaqa = self.adad_irtibatat_muallaqa();
        if muallaqa > 0 {
            return Err(self.rafd(ila, SababManIntiqal::IrtibatMuallaq { adad: muallaqa }));
        }
        let masmuh = self.hala.qabila_lil_tahreer();
        self.hawwil(ila, waqt, masmuh)
    }

    /// Records the owner opening it.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it is not waiting in the queue or the
    /// timestamp is blank.
    pub fn futihat(self, waqt: &str) -> Result<Self, IntiqalMarfud> {
        let masmuh = matches!(self.hala, HalatTaqdeem::Muqaddama { .. });
        self.hawwil(
            HalatTaqdeem::QaydMuraja {
                waqt: waqt.to_owned(),
            },
            waqt,
            masmuh,
        )
    }

    /// Returns it for revision, as the next revision of the same submission.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it is not with the owner, the summary is
    /// blank, or the timestamp is blank.
    pub fn tulib_taadil(
        mut self,
        waqt: &str,
        mulakhkhas: &str,
        nusus: Vec<NassId>,
    ) -> Result<Self, IntiqalMarfud> {
        let ila = HalatTaqdeem::MatlubTaadil {
            waqt: waqt.to_owned(),
            mulakhkhas: mulakhkhas.to_owned(),
            nusus,
        };
        if !self.hala.yaqbal(NawIjraMuraja::TalabTaadil) {
            return Err(self.rafd(ila, SababManIntiqal::HalaGhayrMulaima));
        }
        if mulakhkhas.trim().is_empty() {
            let sabab = SababManIntiqal::BayanNaqis {
                haql: "a written summary of the changes",
            };
            return Err(self.rafd(ila, sabab));
        }
        // The bump precedes the record, so the entry names the revision the
        // contributor is about to work on rather than the one being returned.
        self.murajaa = self.murajaa.talia();
        self.hawwil(ila, waqt, true)
    }

    /// Records the owner's approval.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it is not with the owner or the timestamp
    /// is blank.
    pub fn wufiq_alayha(self, waqt: &str) -> Result<Self, IntiqalMarfud> {
        let masmuh = self.hala.yaqbal(NawIjraMuraja::Iaatimad);
        self.hawwil(
            HalatTaqdeem::MawafaqYunshar {
                waqt: waqt.to_owned(),
            },
            waqt,
            masmuh,
        )
    }

    /// Records publication.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when the publishing sequence was not running or
    /// the timestamp is blank.
    pub fn nushirat(self, waqt: &str) -> Result<Self, IntiqalMarfud> {
        let masmuh = matches!(self.hala, HalatTaqdeem::MawafaqYunshar { .. });
        let ila = HalatTaqdeem::Manshura {
            waqt: waqt.to_owned(),
            murajaa: self.murajaa,
        };
        self.hawwil(ila, waqt, masmuh)
    }

    /// Records a rejection, which a written reason is part of.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it is not with the owner, the reason is
    /// blank, or the timestamp is blank.
    pub fn rufidat(self, waqt: &str, sabab: &str) -> Result<Self, IntiqalMarfud> {
        let ila = HalatTaqdeem::Marfuda {
            waqt: waqt.to_owned(),
            sabab: sabab.to_owned(),
        };
        if !self.hala.yaqbal(NawIjraMuraja::Rafd) {
            return Err(self.rafd(ila, SababManIntiqal::HalaGhayrMulaima));
        }
        if sabab.trim().is_empty() {
            let naqis = SababManIntiqal::BayanNaqis {
                haql: "a written reason for the rejection",
            };
            return Err(self.rafd(ila, naqis));
        }
        self.hawwil(ila, waqt, true)
    }

    /// Records the contributor withdrawing it.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it has already reached a final state or
    /// the timestamp is blank.
    pub fn suhibat(self, waqt: &str) -> Result<Self, IntiqalMarfud> {
        let tunshar = matches!(self.hala, HalatTaqdeem::MawafaqYunshar { .. });
        let masmuh = !self.hala.nihaiya() && !tunshar;
        self.hawwil(
            HalatTaqdeem::Mashuba {
                waqt: waqt.to_owned(),
            },
            waqt,
            masmuh,
        )
    }

    /// Records the owner pulling it, which a written reason is part of.
    ///
    /// Accepted from `MawafaqYunshar` as well as from `Manshura`: approval
    /// seals the package and the registry cast publishes it, and between the
    /// two there is a submission the owner has approved and may want back. If
    /// this only took a published one, an approval the owner changed their mind
    /// about would have no way out at all — it is past every contributor
    /// transition and has not reached the one that would let it be revoked.
    ///
    /// # Errors
    ///
    /// The submission unchanged, when it is neither approved nor published, the
    /// reason is blank, or the timestamp is blank.
    pub fn suhibat_min_almalik(self, waqt: &str, sabab: &str) -> Result<Self, IntiqalMarfud> {
        let ila = HalatTaqdeem::Masbuba {
            waqt: waqt.to_owned(),
            sabab: sabab.to_owned(),
        };
        if !self.hala.yaqbal(NawIjraMuraja::Sahb) {
            return Err(self.rafd(ila, SababManIntiqal::HalaGhayrMulaima));
        }
        if sabab.trim().is_empty() {
            let naqis = SababManIntiqal::BayanNaqis {
                haql: "a written reason for the revocation",
            };
            return Err(self.rafd(ila, naqis));
        }
        self.hawwil(ila, waqt, true)
    }

    fn hawwil(
        mut self,
        ila: HalatTaqdeem,
        waqt: &str,
        masmuh: bool,
    ) -> Result<Self, IntiqalMarfud> {
        if !masmuh {
            return Err(self.rafd(ila, SababManIntiqal::HalaGhayrMulaima));
        }
        if waqt.trim().is_empty() {
            let naqis = SababManIntiqal::BayanNaqis {
                haql: "a timestamp",
            };
            return Err(self.rafd(ila, naqis));
        }
        self.tareekh.push(QaydMusawwada {
            murajaa: self.murajaa,
            ila: ila.clone(),
            waqt: waqt.to_owned(),
        });
        self.hala = ila;
        Ok(self)
    }

    fn rafd(self, ila: HalatTaqdeem, sabab: SababManIntiqal) -> IntiqalMarfud {
        let min = self.hala.clone();
        IntiqalMarfud {
            musawwada: Box::new(self),
            min,
            ila,
            sabab,
        }
    }

    /// Where this draft is stored under the data root.
    #[must_use]
    pub fn masar(&self, jidhr_bayanat: &Path) -> PathBuf {
        masar_musawwada(jidhr_bayanat, self.id)
    }

    /// Reads a stored draft.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the file cannot be read, when it was
    /// written by a build with another draft schema, or when it does not parse —
    /// which is a named failure and never a fresh empty draft.
    pub fn iqra(masar: &Path) -> NatijatTaqdeem<Self> {
        let bayt =
            fs::read(masar).map_err(|sabab| khata_malaf(masar, "reading a saved draft", sabab))?;

        let tarwisa: TarwisatMusawwada = serde_json::from_slice(&bayt).map_err(|sabab| {
            khata_malaf(masar, "reading the schema of a saved draft", sabab.into())
        })?;
        if tarwisa.isdar != ISDAR_MUSAWWADA {
            let sabab = std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("draft schema {} is not {ISDAR_MUSAWWADA}", tarwisa.isdar),
            );
            return Err(khata_malaf(
                masar,
                "reading a draft from another build",
                sabab,
            ));
        }

        serde_json::from_slice(&bayt)
            .map_err(|sabab| khata_malaf(masar, "parsing a saved draft", sabab.into()))
    }

    /// Writes the draft so that the file on disk is either entirely the previous
    /// draft or entirely this one.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the directory cannot be created, the
    /// draft will not serialize, or the temporary copy cannot be written,
    /// flushed or renamed over the target.
    pub fn ihfaz(&self, masar: &Path) -> NatijatTaqdeem<()> {
        let Some(mujallad) = masar.parent() else {
            let sabab = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "a draft path with no directory to write into",
            );
            return Err(khata_malaf(masar, "locating the draft directory", sabab));
        };
        fs::create_dir_all(mujallad)
            .map_err(|sabab| khata_malaf(mujallad, "creating the draft directory", sabab))?;

        let bayt = serde_json::to_vec_pretty(self)
            .map_err(|sabab| khata_malaf(masar, "serializing a draft", sabab.into()))?;

        let muaqqat = masar.with_extension(LAHIQAT_MUAQQATA);
        let natija = kitaba(&muaqqat, &bayt).and_then(|()| {
            fs::rename(&muaqqat, masar)
                .map_err(|sabab| khata_malaf(masar, "replacing the saved draft", sabab))
        });
        if natija.is_err() {
            let _ = fs::remove_file(&muaqqat);
            return natija;
        }

        // The rename is atomic; the new directory entry is not durable until the
        // directory itself is flushed.
        #[cfg(unix)]
        if let Ok(maftuh) = fs::File::open(mujallad) {
            drop(maftuh.sync_all());
        }
        Ok(())
    }

    /// Reads every draft saved under the data root, oldest first, and reports
    /// the files that would not load beside them.
    ///
    /// A draft that does not parse neither disappears from the listing nor
    /// stops it. It comes back in [`SijillMusawwadat::mutaadhira`] naming its
    /// file and its failure, so twelve readable drafts are never lost to one
    /// corrupt file and the one corrupt file is never lost either. This is the
    /// shape the library scan uses for the same problem.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] only when the directory itself cannot be
    /// listed, which is a failure of the listing rather than of any one draft.
    pub fn iqra_kull(jidhr_bayanat: &Path) -> NatijatTaqdeem<SijillMusawwadat> {
        let mujallad = mujallad_musawwadat(jidhr_bayanat);
        let qira = match fs::read_dir(&mujallad) {
            Ok(qira) => qira,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SijillMusawwadat {
                    musawwadat: Vec::new(),
                    mutaadhira: Vec::new(),
                });
            },
            Err(sabab) => return Err(khata_malaf(&mujallad, "listing saved drafts", sabab)),
        };

        let mut musawwadat: Vec<Self> = Vec::new();
        let mut mutaadhira: Vec<MusawwadaMutaadhira> = Vec::new();
        for madkhal in qira {
            // A directory entry that will not even yield its name belongs to the
            // listing, not to a draft: there is no path to attribute it to.
            let madkhal =
                madkhal.map_err(|sabab| khata_malaf(&mujallad, "listing saved drafts", sabab))?;
            let masar = madkhal.path();
            let lahiqa = masar.extension().and_then(|juz| juz.to_str());
            if lahiqa.is_none_or(|juz| juz != LAHIQAT_MUSAWWADA) {
                continue;
            }
            match Self::iqra(&masar) {
                Ok(musawwada) => musawwadat.push(musawwada),
                Err(khata) => mutaadhira.push(MusawwadaMutaadhira { masar, khata }),
            }
        }
        musawwadat.sort_by(|awwal, thani| {
            awwal
                .ansha
                .cmp(&thani.ansha)
                .then_with(|| awwal.id.cmp(&thani.id))
        });
        // `read_dir` promises no order, and a list of failures that reshuffles
        // between two calls reads as a different set of failures.
        mutaadhira.sort_by(|awwal, thani| awwal.masar.cmp(&thani.masar));
        Ok(SijillMusawwadat {
            musawwadat,
            mutaadhira,
        })
    }
}

/// The directory drafts are saved in, under the data root.
#[must_use]
pub fn mujallad_musawwadat(jidhr_bayanat: &Path) -> PathBuf {
    jidhr_bayanat.join(MUJALLAD_MUSAWWADAT)
}

/// Where the draft for one patch lineage is saved.
#[must_use]
pub fn masar_musawwada(jidhr_bayanat: &Path, id: RuqaaId) -> PathBuf {
    // The segment is a hyphenated UUID: no separator, no device name and no
    // trailing dot can appear in it, which is what makes the join safe here.
    mujallad_musawwadat(jidhr_bayanat).join(format!("{id}.{LAHIQAT_MUSAWWADA}"))
}

#[derive(Debug, Deserialize)]
struct TarwisatMusawwada {
    isdar: u32,
}

fn matlub(qeema: &str, haql: &'static str) -> NatijatTaqdeem<()> {
    if qeema.trim().is_empty() {
        return Err(KhataTaqdeem::BayanNaqis { haql });
    }
    Ok(())
}

fn khata_malaf(masar: &Path, amal: &'static str, sabab: std::io::Error) -> KhataTaqdeem {
    KhataTaqdeem::KhataMalaf {
        masar: masar.to_path_buf(),
        amal,
        sabab,
    }
}

fn kitaba(masar: &Path, bayt: &[u8]) -> NatijatTaqdeem<()> {
    let mut malaf = fs::File::create(masar)
        .map_err(|sabab| khata_malaf(masar, "creating the draft's temporary copy", sabab))?;
    malaf
        .write_all(bayt)
        .map_err(|sabab| khata_malaf(masar, "writing the draft's temporary copy", sabab))?;
    malaf
        .flush()
        .map_err(|sabab| khata_malaf(masar, "flushing the draft's temporary copy", sabab))?;
    malaf
        .sync_all()
        .map_err(|sabab| khata_malaf(masar, "syncing the draft's temporary copy", sabab))
}
