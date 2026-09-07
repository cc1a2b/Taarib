//! The review actions the owner can take, and the permanent log they write.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

use crate::hawiya::SalahiyatMalik;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};
use crate::taaliq::TaaliqId;

/// A written reason that cannot be empty.
///
/// Rejection and revocation both take one, so neither is representable without
/// words a contributor can read.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct SababRafd(String);

impl SababRafd {
    /// Trims the reason and keeps it only when something is left to read.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::RafdBilaSabab`] when nothing survives trimming.
    pub fn jadeed(matn: impl Into<String>) -> NatijatTaqdeem<Self> {
        let matn = matn.into();
        let munaqqa = matn.trim();
        if munaqqa.is_empty() {
            return Err(KhataTaqdeem::RafdBilaSabab);
        }
        Ok(Self(munaqqa.to_owned()))
    }

    /// The reason.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// The first line, for a log table that shows one row per action.
    #[must_use]
    pub fn satr_awwal(&self) -> &str {
        self.0.lines().next().unwrap_or_default()
    }
}

impl std::fmt::Display for SababRafd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SababRafd {
    fn deserialize<D>(muharrik: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let khaam = String::deserialize(muharrik)?;
        Self::jadeed(khaam).map_err(serde::de::Error::custom)
    }
}

/// The written summary that accompanies a request for changes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct MulakhkhasTaadil(String);

impl MulakhkhasTaadil {
    /// Trims the summary and keeps it only when something is left to read.
    #[must_use]
    pub fn jadeed(matn: impl Into<String>) -> Option<Self> {
        let matn = matn.into();
        let munaqqa = matn.trim();
        if munaqqa.is_empty() {
            return None;
        }
        Some(Self(munaqqa.to_owned()))
    }

    /// The summary.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// The first line, for a log table that shows one row per action.
    #[must_use]
    pub fn satr_awwal(&self) -> &str {
        self.0.lines().next().unwrap_or_default()
    }
}

impl std::fmt::Display for MulakhkhasTaadil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MulakhkhasTaadil {
    fn deserialize<D>(muharrik: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let khaam = String::deserialize(muharrik)?;
        Self::jadeed(khaam)
            .ok_or_else(|| serde::de::Error::custom("a change request requires a written summary"))
    }
}

/// What the owner did to a submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "ijra", rename_all = "snake_case")]
pub enum IjraMuraja {
    /// A comment, which moves nothing.
    Taaliq {
        /// The comment that was filed.
        taaliq: TaaliqId,
    },
    /// Returned to the contributor for revision.
    TalabTaadil {
        /// The written summary.
        mulakhkhas: MulakhkhasTaadil,
        /// The anchored comments the summary points at.
        taaliqat: Vec<TaaliqId>,
    },
    /// Rejected outright.
    Rafd {
        /// The written reason.
        sabab: SababRafd,
    },
    /// Approved, which is what [`QararIaatimad`] records.
    Iaatimad,
    /// A published revision withdrawn from circulation.
    Sahb {
        /// The written reason.
        sabab: SababRafd,
    },
}

impl IjraMuraja {
    /// The stable key the log is filtered and grouped by.
    #[must_use]
    pub const fn ramz(&self) -> &'static str {
        match self {
            Self::Taaliq { .. } => "taaliq",
            Self::TalabTaadil { .. } => "talab_taadil",
            Self::Rafd { .. } => "rafd",
            Self::Iaatimad => "iaatimad",
            Self::Sahb { .. } => "sahb",
        }
    }

    /// The written reason or summary this action carries, when it carries one.
    #[must_use]
    pub fn sabab(&self) -> Option<&str> {
        match self {
            Self::Rafd { sabab } | Self::Sahb { sabab } => Some(sabab.nass()),
            Self::TalabTaadil { mulakhkhas, .. } => Some(mulakhkhas.nass()),
            Self::Taaliq { .. } | Self::Iaatimad => None,
        }
    }

    /// The anchored comments this action points at.
    #[must_use]
    pub fn taaliqat(&self) -> Vec<TaaliqId> {
        match self {
            Self::Taaliq { taaliq } => vec![*taaliq],
            Self::TalabTaadil { taaliqat, .. } => taaliqat.clone(),
            Self::Rafd { .. } | Self::Iaatimad | Self::Sahb { .. } => Vec::new(),
        }
    }

    /// Whether the submission stops moving after this action.
    #[must_use]
    pub const fn nihaiya(&self) -> bool {
        matches!(self, Self::Rafd { .. } | Self::Sahb { .. })
    }

    /// The action's name as the log renders it, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(&self) -> &'static str {
        match self {
            Self::Taaliq { .. } => "تعليق",
            Self::TalabTaadil { .. } => "طلب تعديل",
            Self::Rafd { .. } => "رفض",
            Self::Iaatimad => "اعتماد",
            Self::Sahb { .. } => "سحب بعد النشر",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(&self) -> &'static str {
        match self {
            Self::Taaliq { .. } => "comment",
            Self::TalabTaadil { .. } => "changes requested",
            Self::Rafd { .. } => "rejected",
            Self::Iaatimad => "approved",
            Self::Sahb { .. } => "revoked after publication",
        }
    }
}

/// Who is reviewing what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarjiMuraja {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision under review.
    pub murajaa: RuqaaRevision,
    /// Who is reviewing it.
    pub murajii: MusahimId,
}

impl MarjiMuraja {
    /// Names one revision of one submission and who is reviewing it.
    #[must_use]
    pub const fn jadeed(ruqaa: RuqaaId, murajaa: RuqaaRevision, murajii: MusahimId) -> Self {
        Self {
            ruqaa,
            murajaa,
            murajii,
        }
    }

    /// The identity a bulk action is issued against.
    #[must_use]
    pub const fn hadaf(&self) -> (RuqaaId, RuqaaRevision) {
        (self.ruqaa, self.murajaa)
    }
}

/// Evidence that the owner approved one specific revision.
///
/// No public constructor: [`iaatimad`] is the only route, and it takes
/// [`SalahiyatMalik`]. Neither `Clone` nor `Deserialize`, so one approval
/// authorises exactly one publish and none can be read in from a file.
#[derive(Debug)]
pub struct QararIaatimad {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    murajii: MusahimId,
    miftah_malik: [u8; 32],
    waqt: String,
}

impl QararIaatimad {
    /// The approved lineage.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The approved revision.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// Who approved it.
    #[must_use]
    pub const fn murajii(&self) -> &MusahimId {
        &self.murajii
    }

    /// The owner's public key, which the published metadata carries.
    #[must_use]
    pub const fn miftah_malik(&self) -> &[u8; 32] {
        &self.miftah_malik
    }

    /// When it was approved, RFC 3339.
    #[must_use]
    pub fn waqt(&self) -> &str {
        &self.waqt
    }

    /// Consumes the decision into the provenance the publishing sequence
    /// records inside the patch metadata.
    #[must_use]
    pub fn ajza(self) -> (RuqaaId, RuqaaRevision, MusahimId, String) {
        (self.ruqaa, self.murajaa, self.murajii, self.waqt)
    }
}

/// Files a comment against a submission.
#[must_use]
pub fn allaq(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    taaliq: TaaliqId,
    waqt: String,
) -> QaydMuraja {
    qayd(salahiya, marji, IjraMuraja::Taaliq { taaliq }, waqt)
}

/// Returns a submission to its contributor for revision.
#[must_use]
pub fn utlub_taadil(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    mulakhkhas: MulakhkhasTaadil,
    taaliqat: Vec<TaaliqId>,
    waqt: String,
) -> QaydMuraja {
    qayd(
        salahiya,
        marji,
        IjraMuraja::TalabTaadil {
            mulakhkhas,
            taaliqat,
        },
        waqt,
    )
}

/// Rejects a submission.
#[must_use]
pub fn urfud(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    sabab: SababRafd,
    waqt: String,
) -> QaydMuraja {
    qayd(salahiya, marji, IjraMuraja::Rafd { sabab }, waqt)
}

/// Withdraws a published revision from circulation.
#[must_use]
pub fn ishab(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    sabab: SababRafd,
    waqt: String,
) -> QaydMuraja {
    qayd(salahiya, marji, IjraMuraja::Sahb { sabab }, waqt)
}

/// Approves a submission, minting the evidence the publishing sequence
/// consumes alongside the log entry that records it.
///
/// Deliberately not offered in bulk: approval is the one action that puts a
/// patch in front of every client, and it is taken one submission at a time.
#[must_use]
pub fn iaatimad(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    waqt: String,
) -> (QararIaatimad, QaydMuraja) {
    let qarar = QararIaatimad {
        ruqaa: marji.ruqaa,
        murajaa: marji.murajaa,
        murajii: marji.murajii.clone(),
        miftah_malik: *salahiya.miftah_aam(),
        waqt: waqt.clone(),
    };
    let sijill = qayd(salahiya, marji, IjraMuraja::Iaatimad, waqt);
    (qarar, sijill)
}

/// Rejects every selected submission for the same written reason, producing one
/// record per identity.
#[must_use]
pub fn urfud_jumla(
    salahiya: &SalahiyatMalik,
    murajii: &MusahimId,
    ahdaf: &[(RuqaaId, RuqaaRevision)],
    sabab: &SababRafd,
    waqt: &str,
) -> Vec<QaydMuraja> {
    jumla(salahiya, murajii, ahdaf, waqt, |_| IjraMuraja::Rafd {
        sabab: sabab.clone(),
    })
}

/// Withdraws every selected published revision for the same written reason.
#[must_use]
pub fn ishab_jumla(
    salahiya: &SalahiyatMalik,
    murajii: &MusahimId,
    ahdaf: &[(RuqaaId, RuqaaRevision)],
    sabab: &SababRafd,
    waqt: &str,
) -> Vec<QaydMuraja> {
    jumla(salahiya, murajii, ahdaf, waqt, |_| IjraMuraja::Sahb {
        sabab: sabab.clone(),
    })
}

/// Returns every selected submission for revision with the same summary.
#[must_use]
pub fn utlub_taadil_jumla(
    salahiya: &SalahiyatMalik,
    murajii: &MusahimId,
    ahdaf: &[(RuqaaId, RuqaaRevision)],
    mulakhkhas: &MulakhkhasTaadil,
    waqt: &str,
) -> Vec<QaydMuraja> {
    jumla(salahiya, murajii, ahdaf, waqt, |_| {
        IjraMuraja::TalabTaadil {
            mulakhkhas: mulakhkhas.clone(),
            taaliqat: Vec::new(),
        }
    })
}

fn jumla(
    salahiya: &SalahiyatMalik,
    murajii: &MusahimId,
    ahdaf: &[(RuqaaId, RuqaaRevision)],
    waqt: &str,
    bani: impl Fn((RuqaaId, RuqaaRevision)) -> IjraMuraja,
) -> Vec<QaydMuraja> {
    ahdaf
        .iter()
        .map(|(ruqaa, murajaa)| {
            let marji = MarjiMuraja::jadeed(*ruqaa, *murajaa, murajii.clone());
            qayd(salahiya, &marji, bani((*ruqaa, *murajaa)), waqt.to_owned())
        })
        .collect()
}

fn qayd(
    salahiya: &SalahiyatMalik,
    marji: &MarjiMuraja,
    ijra: IjraMuraja,
    waqt: String,
) -> QaydMuraja {
    QaydMuraja {
        ruqaa: marji.ruqaa,
        murajaa: marji.murajaa,
        murajii: marji.murajii.clone(),
        miftah_malik: *salahiya.miftah_aam(),
        ijra,
        waqt,
    }
}

/// One entry of the audit log: who, when, what, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QaydMuraja {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    murajii: MusahimId,
    miftah_malik: [u8; 32],
    ijra: IjraMuraja,
    waqt: String,
}

impl QaydMuraja {
    /// The lineage acted on.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The revision acted on.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// Who acted.
    #[must_use]
    pub const fn murajii(&self) -> &MusahimId {
        &self.murajii
    }

    /// The owner's public key at the moment of the action.
    #[must_use]
    pub const fn miftah_malik(&self) -> &[u8; 32] {
        &self.miftah_malik
    }

    /// What was done.
    #[must_use]
    pub const fn ijra(&self) -> &IjraMuraja {
        &self.ijra
    }

    /// When, RFC 3339.
    #[must_use]
    pub fn waqt(&self) -> &str {
        &self.waqt
    }

    /// The identity this entry belongs to.
    #[must_use]
    pub const fn hadaf(&self) -> (RuqaaId, RuqaaRevision) {
        (self.ruqaa, self.murajaa)
    }

    /// The entry as the console's log table shows it, in Arabic.
    #[must_use]
    pub fn aard_arabi(&self) -> SatrSijill {
        SatrSijill {
            waqt: self.waqt.clone(),
            ruqaa: self.ruqaa.mukhtasar(),
            murajaa: self.murajaa.to_string(),
            murajii: self.murajii.mukhtasar(),
            ijra: self.ijra.wasf_arabi().to_owned(),
            sabab: self.ijra.sabab().map(str::to_owned),
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn aard_injilizi(&self) -> SatrSijill {
        SatrSijill {
            waqt: self.waqt.clone(),
            ruqaa: self.ruqaa.mukhtasar(),
            murajaa: self.murajaa.to_string(),
            murajii: self.murajii.mukhtasar(),
            ijra: self.ijra.wasf_injilizi().to_owned(),
            sabab: self.ijra.sabab().map(str::to_owned),
        }
    }
}

/// One rendered line of the audit log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatrSijill {
    /// When the action was taken, RFC 3339.
    pub waqt: String,
    /// The lineage, abbreviated.
    pub ruqaa: String,
    /// The revision.
    pub murajaa: String,
    /// Who acted, abbreviated.
    pub murajii: String,
    /// What was done.
    pub ijra: String,
    /// The written reason, where the action carries one.
    pub sabab: Option<String>,
}

impl SatrSijill {
    /// The line joined into one string, for a plain-text rendering.
    #[must_use]
    pub fn satr(&self) -> String {
        let asas = format!(
            "{} · {} {} · {} · {}",
            self.waqt, self.ruqaa, self.murajaa, self.murajii, self.ijra
        );
        match &self.sabab {
            Some(sabab) => format!("{asas} · {}", sabab.lines().next().unwrap_or_default()),
            None => asas,
        }
    }
}

/// The append-only record of every review action.
///
/// Written into the registry, and therefore public and permanent. There is no
/// method here that mutates or removes an entry, and adding one would break the
/// guarantee the published log is read under.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SijillMuraja {
    quyud: Vec<QaydMuraja>,
}

impl SijillMuraja {
    /// An empty log.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { quyud: Vec::new() }
    }

    /// Appends one entry.
    pub fn alhiq(&mut self, qayd: QaydMuraja) {
        tracing::info!(
            ruqaa = %qayd.ruqaa,
            murajaa = qayd.murajaa.qeema(),
            ijra = qayd.ijra.ramz(),
            "review action recorded"
        );
        self.quyud.push(qayd);
    }

    /// Appends several entries in order.
    pub fn alhiq_kull(&mut self, quyud: impl IntoIterator<Item = QaydMuraja>) {
        for qayd in quyud {
            self.alhiq(qayd);
        }
    }

    /// Every entry, oldest first.
    #[must_use]
    pub fn quyud(&self) -> &[QaydMuraja] {
        &self.quyud
    }

    /// How many entries there are.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.quyud.len()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub const fn farigh(&self) -> bool {
        self.quyud.is_empty()
    }

    /// The most recent entry.
    #[must_use]
    pub fn akhir(&self) -> Option<&QaydMuraja> {
        self.quyud.last()
    }

    /// Every entry against one lineage.
    #[must_use]
    pub fn li_ruqaa(&self, ruqaa: RuqaaId) -> Vec<&QaydMuraja> {
        self.quyud
            .iter()
            .filter(|qayd| qayd.ruqaa == ruqaa)
            .collect()
    }

    /// Every entry against one revision.
    #[must_use]
    pub fn li_murajaa(&self, ruqaa: RuqaaId, murajaa: RuqaaRevision) -> Vec<&QaydMuraja> {
        self.quyud
            .iter()
            .filter(|qayd| qayd.ruqaa == ruqaa && qayd.murajaa == murajaa)
            .collect()
    }

    /// The most recent entry against one lineage.
    #[must_use]
    pub fn akhir_li_ruqaa(&self, ruqaa: RuqaaId) -> Option<&QaydMuraja> {
        self.quyud.iter().rev().find(|qayd| qayd.ruqaa == ruqaa)
    }

    /// Every entry by one reviewer.
    #[must_use]
    pub fn li_murajii(&self, murajii: &MusahimId) -> Vec<&QaydMuraja> {
        self.quyud
            .iter()
            .filter(|qayd| qayd.murajii == *murajii)
            .collect()
    }

    /// Every lineage that has been withdrawn after publication.
    #[must_use]
    pub fn masbuba(&self) -> BTreeSet<RuqaaId> {
        self.quyud
            .iter()
            .filter(|qayd| matches!(qayd.ijra, IjraMuraja::Sahb { .. }))
            .map(|qayd| qayd.ruqaa)
            .collect()
    }

    /// Whether one revision carries a recorded approval that no later
    /// revocation has undone.
    #[must_use]
    pub fn muaatamada(&self, ruqaa: RuqaaId, murajaa: RuqaaRevision) -> bool {
        let mut muaatamad = false;
        for qayd in &self.quyud {
            if qayd.ruqaa != ruqaa || qayd.murajaa != murajaa {
                continue;
            }
            match qayd.ijra {
                IjraMuraja::Iaatimad => muaatamad = true,
                IjraMuraja::Sahb { .. } | IjraMuraja::Rafd { .. } => muaatamad = false,
                IjraMuraja::Taaliq { .. } | IjraMuraja::TalabTaadil { .. } => {},
            }
        }
        muaatamad
    }

    /// Whether this log extends `sabiq` entry for entry, which is what makes
    /// replacing the stored file safe.
    #[must_use]
    pub fn yatba(&self, sabiq: &Self) -> bool {
        self.quyud.len() >= sabiq.quyud.len()
            && self
                .quyud
                .iter()
                .zip(sabiq.quyud.iter())
                .all(|(jadid, qadeem)| jadid == qadeem)
    }

    /// The whole log rendered for display, in Arabic.
    #[must_use]
    pub fn aard_arabi(&self) -> Vec<SatrSijill> {
        self.quyud.iter().map(QaydMuraja::aard_arabi).collect()
    }

    /// The same, in English.
    #[must_use]
    pub fn aard_injilizi(&self) -> Vec<SatrSijill> {
        self.quyud.iter().map(QaydMuraja::aard_injilizi).collect()
    }

    /// Reads the log from the registry working copy.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the file cannot be read, or when its
    /// contents are not a log this build can parse.
    pub fn iqra(masar: &Path) -> NatijatTaqdeem<Self> {
        let nass = fs::read_to_string(masar)
            .map_err(|sabab| khata_malaf(masar, "reading the review log", sabab))?;
        serde_json::from_str(&nass)
            .map_err(|sabab| khata_malaf(masar, "parsing the review log", io::Error::other(sabab)))
    }

    /// Reads the log, treating a missing file as an empty one.
    ///
    /// # Errors
    ///
    /// As [`SijillMuraja::iqra`], except that a missing file is not a failure.
    pub fn iqra_aw_farigh(masar: &Path) -> NatijatTaqdeem<Self> {
        if !masar.exists() {
            return Ok(Self::jadeed());
        }
        Self::iqra(masar)
    }

    /// Writes the log back, refusing to replace a stored log this one does not
    /// extend.
    ///
    /// # Errors
    ///
    /// [`KhataTaqdeem::KhataMalaf`] when the stored log cannot be read, when
    /// this log would drop or rewrite an entry already in it, or when the file
    /// cannot be written.
    pub fn uktub(&self, masar: &Path) -> NatijatTaqdeem<()> {
        let mahfuz = Self::iqra_aw_farigh(masar)?;
        if !self.yatba(&mahfuz) {
            return Err(khata_malaf(
                masar,
                "writing the review log",
                io::Error::other("the stored log has entries this one would drop or rewrite"),
            ));
        }
        let nass = serde_json::to_string_pretty(self).map_err(|sabab| {
            khata_malaf(masar, "encoding the review log", io::Error::other(sabab))
        })?;
        fs::write(masar, nass.as_bytes())
            .map_err(|sabab| khata_malaf(masar, "writing the review log", sabab))
    }
}

fn khata_malaf(masar: &Path, amal: &'static str, sabab: io::Error) -> KhataTaqdeem {
    KhataTaqdeem::KhataMalaf {
        masar: masar.to_path_buf(),
        amal,
        sabab,
    }
}
