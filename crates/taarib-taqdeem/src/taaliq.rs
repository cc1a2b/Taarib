//! Comments anchored to one string of a submission, and the threads they form.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::NassId;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

use crate::hawiya::SalahiyatMalik;

/// The longest comment body a thread will hold, in characters.
pub const AQSA_TUL_TAALIQ: usize = 4_000;

/// A comment's identity within one submission's thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaaliqId(u64);

impl TaaliqId {
    /// Wraps a stored identity.
    #[must_use]
    pub const fn jadeed(qeema: u64) -> Self {
        Self(qeema)
    }

    /// The underlying value.
    #[must_use]
    pub const fn qeema(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TaaliqId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A comment body that is neither empty nor longer than [`AQSA_TUL_TAALIQ`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct NassTaaliq(String);

impl NassTaaliq {
    /// Trims the body and keeps it only when something is left to read.
    #[must_use]
    pub fn jadeed(matn: impl Into<String>) -> Option<Self> {
        let matn = matn.into();
        let munaqqa = matn.trim();
        if munaqqa.is_empty() || munaqqa.chars().count() > AQSA_TUL_TAALIQ {
            return None;
        }
        Some(Self(munaqqa.to_owned()))
    }

    /// The body.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// The first line, for a dense list that shows one row per comment.
    #[must_use]
    pub fn satr_awwal(&self) -> &str {
        self.0.lines().next().unwrap_or_default()
    }
}

impl std::fmt::Display for NassTaaliq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for NassTaaliq {
    fn deserialize<D>(muharrik: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let khaam = String::deserialize(muharrik)?;
        Self::jadeed(khaam).ok_or_else(|| {
            serde::de::Error::custom("a comment body is neither empty nor over 4000 characters")
        })
    }
}

/// One comment on a submission.
///
/// `mawdi` is [`None`] for a comment about the submission as a whole. The
/// owner's key is recorded on owner comments and cannot be attached by
/// [`Taaliq::min_musahim`], so "who wrote this" is evidence rather than a flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taaliq {
    id: TaaliqId,
    mawdi: Option<NassId>,
    kaatib: MusahimId,
    miftah_malik: Option<[u8; 32]>,
    matn: NassTaaliq,
    murajaa: RuqaaRevision,
    waqt: String,
    muhall: bool,
}

impl Taaliq {
    /// A comment written by the owner.
    #[must_use]
    pub const fn min_malik(
        id: TaaliqId,
        salahiya: &SalahiyatMalik,
        kaatib: MusahimId,
        mawdi: Option<NassId>,
        matn: NassTaaliq,
        murajaa: RuqaaRevision,
        waqt: String,
    ) -> Self {
        Self {
            id,
            mawdi,
            kaatib,
            miftah_malik: Some(*salahiya.miftah_aam()),
            matn,
            murajaa,
            waqt,
            muhall: false,
        }
    }

    /// A reply written by the contributor on their own submission.
    #[must_use]
    pub const fn min_musahim(
        id: TaaliqId,
        kaatib: MusahimId,
        mawdi: Option<NassId>,
        matn: NassTaaliq,
        murajaa: RuqaaRevision,
        waqt: String,
    ) -> Self {
        Self {
            id,
            mawdi,
            kaatib,
            miftah_malik: None,
            matn,
            murajaa,
            waqt,
            muhall: false,
        }
    }

    /// Its identity within the thread.
    #[must_use]
    pub const fn id(&self) -> TaaliqId {
        self.id
    }

    /// The string it anchors to, or [`None`] when it is about the submission.
    #[must_use]
    pub const fn mawdi(&self) -> Option<NassId> {
        self.mawdi
    }

    /// Whether it anchors to a string.
    #[must_use]
    pub const fn murtabit(&self) -> bool {
        self.mawdi.is_some()
    }

    /// Who wrote it.
    #[must_use]
    pub const fn kaatib(&self) -> &MusahimId {
        &self.kaatib
    }

    /// Whether the owner's authority minted it.
    #[must_use]
    pub const fn min_almalik(&self) -> bool {
        self.miftah_malik.is_some()
    }

    /// The owner's public key, on an owner comment.
    #[must_use]
    pub const fn miftah_malik(&self) -> Option<&[u8; 32]> {
        self.miftah_malik.as_ref()
    }

    /// The body.
    #[must_use]
    pub const fn matn(&self) -> &NassTaaliq {
        &self.matn
    }

    /// The revision it was written against.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// When it was written, RFC 3339.
    #[must_use]
    pub fn waqt(&self) -> &str {
        &self.waqt
    }

    /// Whether it has been marked resolved.
    #[must_use]
    pub const fn muhall(&self) -> bool {
        self.muhall
    }

    /// Marks it resolved, answering whether that changed anything.
    pub const fn hall(&mut self, _salahiya: &SalahiyatMalik) -> bool {
        let taghayyar = !self.muhall;
        self.muhall = true;
        taghayyar
    }

    /// Reopens it, answering whether that changed anything.
    pub const fn afta(&mut self, _salahiya: &SalahiyatMalik) -> bool {
        let taghayyar = self.muhall;
        self.muhall = false;
        taghayyar
    }

    /// Whether it was written against a revision later than `murajaa`, which is
    /// what makes a second review pass show only what is new.
    #[must_use]
    pub fn baad(&self, murajaa: RuqaaRevision) -> bool {
        self.murajaa > murajaa
    }
}

/// Every comment on one submission, across every revision of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaaliqatMusawwada {
    ruqaa: RuqaaId,
    taaliqat: Vec<Taaliq>,
    talia: u64,
}

impl TaaliqatMusawwada {
    /// An empty thread for one submission.
    #[must_use]
    pub const fn jadeeda(ruqaa: RuqaaId) -> Self {
        Self {
            ruqaa,
            taaliqat: Vec::new(),
            talia: 1,
        }
    }

    /// The submission these comments belong to.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The identity the next comment should be built with.
    #[must_use]
    pub const fn muarrif_talia(&self) -> TaaliqId {
        TaaliqId::jadeed(self.talia)
    }

    /// Files a comment, answering `false` and changing nothing when its
    /// identity is already present.
    pub fn adif(&mut self, taaliq: Taaliq) -> bool {
        let id = taaliq.id();
        if self.taaliqat.iter().any(|mawjud| mawjud.id() == id) {
            return false;
        }
        self.talia = self.talia.max(id.qeema().saturating_add(1));
        self.taaliqat.push(taaliq);
        true
    }

    /// Every comment, in the order they were filed.
    #[must_use]
    pub fn kull(&self) -> &[Taaliq] {
        &self.taaliqat
    }

    /// How many comments there are.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.taaliqat.len()
    }

    /// Whether nothing has been said yet.
    #[must_use]
    pub const fn farigha(&self) -> bool {
        self.taaliqat.is_empty()
    }

    /// One comment by identity.
    #[must_use]
    pub fn bi_muarrif(&self, id: TaaliqId) -> Option<&Taaliq> {
        self.taaliqat.iter().find(|taaliq| taaliq.id() == id)
    }

    /// Every comment anchored to one string, which is what the contributor's
    /// workspace shows beside that string.
    #[must_use]
    pub fn bi_nass(&self, nass: NassId) -> Vec<&Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.mawdi() == Some(nass))
            .collect()
    }

    /// Every comment about the submission as a whole.
    #[must_use]
    pub fn ala_almusawwada(&self) -> Vec<&Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| !taaliq.murtabit())
            .collect()
    }

    /// Every comment written against one revision.
    #[must_use]
    pub fn bi_murajaa(&self, murajaa: RuqaaRevision) -> Vec<&Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.murajaa() == murajaa)
            .collect()
    }

    /// Every comment written after `murajaa`, for the "new since last revision"
    /// pass of a second review.
    #[must_use]
    pub fn baad(&self, murajaa: RuqaaRevision) -> Vec<&Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.baad(murajaa))
            .collect()
    }

    /// How many anchored comments are still unresolved.
    #[must_use]
    pub fn adad_muallaq(&self) -> usize {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.murtabit() && !taaliq.muhall())
            .count()
    }

    /// How many comments of any kind are still unresolved.
    #[must_use]
    pub fn adad_muallaq_kulli(&self) -> usize {
        self.taaliqat
            .iter()
            .filter(|taaliq| !taaliq.muhall())
            .count()
    }

    /// The strings that still carry an unresolved comment.
    #[must_use]
    pub fn nusus_muallaqa(&self) -> BTreeSet<NassId> {
        self.taaliqat
            .iter()
            .filter(|taaliq| !taaliq.muhall())
            .filter_map(Taaliq::mawdi)
            .collect()
    }

    /// How many unresolved anchored comments sit on one string.
    #[must_use]
    pub fn adad_muallaq_ala(&self, nass: NassId) -> usize {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.mawdi() == Some(nass) && !taaliq.muhall())
            .count()
    }

    /// Marks one comment resolved.
    pub fn hall(&mut self, salahiya: &SalahiyatMalik, id: TaaliqId) -> bool {
        self.taaliqat
            .iter_mut()
            .find(|taaliq| taaliq.id() == id)
            .is_some_and(|taaliq| taaliq.hall(salahiya))
    }

    /// Reopens one comment.
    pub fn afta(&mut self, salahiya: &SalahiyatMalik, id: TaaliqId) -> bool {
        self.taaliqat
            .iter_mut()
            .find(|taaliq| taaliq.id() == id)
            .is_some_and(|taaliq| taaliq.afta(salahiya))
    }

    /// Marks every anchored comment of one revision resolved, answering how
    /// many changed.
    pub fn hall_murajaa(&mut self, salahiya: &SalahiyatMalik, murajaa: RuqaaRevision) -> usize {
        let mut adad = 0_usize;
        for taaliq in &mut self.taaliqat {
            if taaliq.murajaa() == murajaa && taaliq.hall(salahiya) {
                adad = adad.saturating_add(1);
            }
        }
        adad
    }

    /// The revisions that have been commented on, ascending.
    #[must_use]
    pub fn murajaat(&self) -> BTreeSet<RuqaaRevision> {
        self.taaliqat.iter().map(Taaliq::murajaa).collect()
    }

    /// One thread per revision, ascending, each in filing order.
    #[must_use]
    pub fn khuyut(&self) -> Vec<KhaytMurajaa<'_>> {
        let mut majmuat: BTreeMap<RuqaaRevision, Vec<&Taaliq>> = BTreeMap::new();
        for taaliq in &self.taaliqat {
            majmuat.entry(taaliq.murajaa()).or_default().push(taaliq);
        }
        majmuat
            .into_iter()
            .map(|(murajaa, taaliqat)| KhaytMurajaa { murajaa, taaliqat })
            .collect()
    }

    /// The thread of one revision.
    #[must_use]
    pub fn khayt(&self, murajaa: RuqaaRevision) -> KhaytMurajaa<'_> {
        KhaytMurajaa {
            murajaa,
            taaliqat: self.bi_murajaa(murajaa),
        }
    }
}

/// Every comment written against one revision of a submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhaytMurajaa<'a> {
    murajaa: RuqaaRevision,
    taaliqat: Vec<&'a Taaliq>,
}

impl<'a> KhaytMurajaa<'a> {
    /// The revision this thread belongs to.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// The comments, in filing order.
    #[must_use]
    pub fn taaliqat(&self) -> &[&'a Taaliq] {
        &self.taaliqat
    }

    /// How many comments the thread holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.taaliqat.len()
    }

    /// How many anchored comments in it are unresolved.
    #[must_use]
    pub fn adad_muallaq(&self) -> usize {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.murtabit() && !taaliq.muhall())
            .count()
    }

    /// The comments the owner wrote.
    #[must_use]
    pub fn min_almalik(&self) -> Vec<&'a Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| taaliq.min_almalik())
            .copied()
            .collect()
    }

    /// The comments the contributor wrote.
    #[must_use]
    pub fn min_almusahim(&self) -> Vec<&'a Taaliq> {
        self.taaliqat
            .iter()
            .filter(|taaliq| !taaliq.min_almalik())
            .copied()
            .collect()
    }

    /// The thread's header line, in Arabic.
    #[must_use]
    pub fn tarwisa_arabiya(&self) -> String {
        format!(
            "المراجعة {}: {} تعليقًا، منها {} معلّقة.",
            self.murajaa.qeema(),
            self.adad(),
            self.adad_muallaq()
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn tarwisa_injiliziya(&self) -> String {
        format!(
            "Revision {}: {} comment(s), {} unresolved.",
            self.murajaa.qeema(),
            self.adad(),
            self.adad_muallaq()
        )
    }
}
