//! Assignment and soft locking per string, advisory on both counts.

use std::collections::BTreeMap;

use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::NassId;

/// Lock lifetime in seconds: one day, so a closed laptop frees its strings by tomorrow.
pub const MUDDAT_QUFL: u64 = 86_400;

/// Who each string is assigned to.
///
/// Advisory metadata: an assignment routes work and colours a table, and
/// nothing anywhere refuses an edit because of one.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tawzi {
    takleefat: BTreeMap<NassId, MusahimId>,
}

impl Tawzi {
    /// An empty assignment map.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Assigns one string, returning the assignee it displaced.
    pub fn ayyin(&mut self, nass: NassId, musahim: MusahimId) -> Option<MusahimId> {
        self.takleefat.insert(nass, musahim)
    }

    /// Assigns every listed string to one contributor, returning how many
    /// changed hands or gained an assignee.
    pub fn ayyin_kul(
        &mut self,
        nusus: impl IntoIterator<Item = NassId>,
        musahim: &MusahimId,
    ) -> usize {
        let mut taghayyarat = 0_usize;
        for nass in nusus {
            let sabiq = self.takleefat.insert(nass, musahim.clone());
            if sabiq.as_ref() != Some(musahim) {
                taghayyarat = taghayyarat.saturating_add(1);
            }
        }
        taghayyarat
    }

    /// Removes one string's assignment, returning who held it.
    pub fn alghi(&mut self, nass: NassId) -> Option<MusahimId> {
        self.takleefat.remove(&nass)
    }

    /// Removes every assignment one contributor holds, returning how many.
    pub fn alghi_musahim(&mut self, musahim: &MusahimId) -> usize {
        let qabl = self.takleefat.len();
        self.takleefat.retain(|_, mukallaf| mukallaf != musahim);
        qabl.saturating_sub(self.takleefat.len())
    }

    /// The assignee of one string.
    #[must_use]
    pub fn mukallaf(&self, nass: NassId) -> Option<&MusahimId> {
        self.takleefat.get(&nass)
    }

    /// Every string assigned to one contributor, in identity order.
    #[must_use]
    pub fn bi_musahim(&self, musahim: &MusahimId) -> Vec<NassId> {
        self.takleefat
            .iter()
            .filter(|(_, mukallaf)| *mukallaf == musahim)
            .map(|(nass, _)| *nass)
            .collect()
    }

    /// How many strings carry an assignment.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.takleefat.len()
    }

    /// Whether nothing is assigned.
    #[must_use]
    pub fn farigh(&self) -> bool {
        self.takleefat.is_empty()
    }

    /// Every assignment, in identity order.
    pub fn kull(&self) -> impl Iterator<Item = (NassId, &MusahimId)> {
        self.takleefat
            .iter()
            .map(|(nass, musahim)| (*nass, musahim))
    }

    /// The listed strings that carry no assignment, in the order given.
    #[must_use]
    pub fn bila_takleef(&self, nusus: &[NassId]) -> Vec<NassId> {
        nusus
            .iter()
            .filter(|nass| !self.takleefat.contains_key(*nass))
            .copied()
            .collect()
    }

    /// How many strings each contributor is assigned, in identity order.
    #[must_use]
    pub fn hisas(&self) -> BTreeMap<MusahimId, usize> {
        let mut ahmal: BTreeMap<MusahimId, usize> = BTreeMap::new();
        for musahim in self.takleefat.values() {
            let himl = ahmal.entry(musahim.clone()).or_default();
            *himl = himl.saturating_add(1);
        }
        ahmal
    }
}

/// One string both copies assigned to different people.
///
/// Neither assignment survives the merge; both names survive here, so the
/// report can show what each side intended without forcing anyone to lose.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IkhtilafTawzi {
    /// The string.
    pub nass: NassId,
    /// The local side's assignee.
    pub ana: MusahimId,
    /// The imported side's assignee.
    pub hum: MusahimId,
}

/// Merges two assignment maps: equal entries kept, one-sided entries taken,
/// disagreements left unassigned and reported whole.
#[must_use]
pub fn damj_tawzi(ana: &Tawzi, hum: &Tawzi) -> (Tawzi, Vec<IkhtilafTawzi>) {
    let mut madmuj = Tawzi::jadeed();
    let mut ikhtilafat = Vec::new();

    for (nass, mukallaf_ana) in &ana.takleefat {
        match hum.takleefat.get(nass) {
            Some(mukallaf_hum) if mukallaf_hum == mukallaf_ana => {
                let _ = madmuj.takleefat.insert(*nass, mukallaf_ana.clone());
            },
            Some(mukallaf_hum) => {
                ikhtilafat.push(IkhtilafTawzi {
                    nass: *nass,
                    ana: mukallaf_ana.clone(),
                    hum: mukallaf_hum.clone(),
                });
            },
            None => {
                let _ = madmuj.takleefat.insert(*nass, mukallaf_ana.clone());
            },
        }
    }
    for (nass, mukallaf_hum) in &hum.takleefat {
        if !ana.takleefat.contains_key(nass) {
            let _ = madmuj.takleefat.insert(*nass, mukallaf_hum.clone());
        }
    }

    (madmuj, ikhtilafat)
}

/// A soft lock one translator holds on one string while editing it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Qufl {
    /// The string.
    pub nass: NassId,
    /// Who holds it.
    pub hamil: MusahimId,
    /// When it was claimed, RFC 3339, supplied by the caller.
    pub waqt: String,
    /// The same instant in Unix seconds, supplied by the caller, for expiry
    /// arithmetic.
    pub lahza: u64,
}

impl Qufl {
    /// A lock claimed at a caller-supplied instant.
    #[must_use]
    pub const fn jadeed(nass: NassId, hamil: MusahimId, waqt: String, lahza: u64) -> Self {
        Self {
            nass,
            hamil,
            waqt,
            lahza,
        }
    }

    /// Whether the lock has outlived [`MUDDAT_QUFL`] at the given instant.
    #[must_use]
    pub const fn muntahi(&self, alaan: u64) -> bool {
        alaan.saturating_sub(self.lahza) >= MUDDAT_QUFL
    }

    /// The seconds left before this lock expires, zero once it has.
    #[must_use]
    pub const fn baqi(&self, alaan: u64) -> u64 {
        MUDDAT_QUFL.saturating_sub(alaan.saturating_sub(self.lahza))
    }
}

/// What a claim did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "hasil", rename_all = "snake_case")]
pub enum HasilQufl {
    /// The claimant holds the lock now.
    Mumsak,
    /// Somebody else holds an unexpired lock; the claim changed nothing.
    Mahjuz {
        /// The unexpired locks in the way, holders shown.
        aqfal: Vec<Qufl>,
    },
}

impl HasilQufl {
    /// Whether the claim succeeded.
    #[must_use]
    pub const fn mumsak(&self) -> bool {
        matches!(self, Self::Mumsak)
    }
}

/// Every lock currently recorded on one project's strings.
///
/// Normal use keeps one lock per string; a merge may leave several, because
/// two people having edited one string is a signal to show, not a state to
/// erase.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AqfalMashru {
    aqfal: BTreeMap<NassId, Vec<Qufl>>,
}

impl AqfalMashru {
    /// An empty lock set.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Claims a string: the claimant's own lock is refreshed, expired locks
    /// are displaced, and an unexpired lock held by anyone else blocks the
    /// claim softly.
    pub fn talib(&mut self, qufl: Qufl, alaan: u64) -> HasilQufl {
        let mawjuda = self.aqfal.entry(qufl.nass).or_default();
        let hajiza: Vec<Qufl> = mawjuda
            .iter()
            .filter(|qaim| qaim.hamil != qufl.hamil && !qaim.muntahi(alaan))
            .cloned()
            .collect();
        if !hajiza.is_empty() {
            return HasilQufl::Mahjuz { aqfal: hajiza };
        }
        *mawjuda = vec![qufl];
        HasilQufl::Mumsak
    }

    /// Releases one holder's lock on one string, answering whether one was
    /// held.
    pub fn harrir(&mut self, nass: NassId, hamil: &MusahimId) -> bool {
        let Some(mawjuda) = self.aqfal.get_mut(&nass) else {
            return false;
        };
        let qabl = mawjuda.len();
        mawjuda.retain(|qufl| qufl.hamil != *hamil);
        let hudhifa = mawjuda.len() != qabl;
        if mawjuda.is_empty() {
            let _ = self.aqfal.remove(&nass);
        }
        hudhifa
    }

    /// Removes every lock expired at the given instant, returning how many.
    pub fn iknus(&mut self, alaan: u64) -> usize {
        let mut mahdhufa = 0_usize;
        self.aqfal.retain(|_, quyud| {
            let qabl = quyud.len();
            quyud.retain(|qufl| !qufl.muntahi(alaan));
            mahdhufa = mahdhufa.saturating_add(qabl.saturating_sub(quyud.len()));
            !quyud.is_empty()
        });
        mahdhufa
    }

    /// Every lock recorded on one string, expired ones included.
    #[must_use]
    pub fn ala(&self, nass: NassId) -> &[Qufl] {
        match self.aqfal.get(&nass) {
            Some(quyud) => quyud,
            None => &[],
        }
    }

    /// The unexpired locks on one string at the given instant.
    #[must_use]
    pub fn faal(&self, nass: NassId, alaan: u64) -> Vec<&Qufl> {
        self.ala(nass)
            .iter()
            .filter(|qufl| !qufl.muntahi(alaan))
            .collect()
    }

    /// Whether anyone other than `hamil` holds an unexpired lock on a string.
    #[must_use]
    pub fn mahjuz_ala(&self, nass: NassId, hamil: &MusahimId, alaan: u64) -> bool {
        self.faal(nass, alaan)
            .iter()
            .any(|qufl| qufl.hamil != *hamil)
    }

    /// Every lock one contributor holds, in string-identity order.
    #[must_use]
    pub fn bi_hamil(&self, hamil: &MusahimId) -> Vec<&Qufl> {
        self.kull().filter(|qufl| qufl.hamil == *hamil).collect()
    }

    /// How many locks are recorded in total.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.aqfal.values().map(Vec::len).sum()
    }

    /// Whether no lock is recorded.
    #[must_use]
    pub fn farigh(&self) -> bool {
        self.aqfal.is_empty()
    }

    /// Every recorded lock, grouped by string, in identity order.
    pub fn kull(&self) -> impl Iterator<Item = &Qufl> {
        self.aqfal.values().flatten()
    }

    /// The sole unexpired holder of one string, when exactly one exists.
    #[must_use]
    pub fn hamil_wahid(&self, nass: NassId, alaan: u64) -> Option<&MusahimId> {
        let faala = self.faal(nass, alaan);
        match faala.as_slice() {
            [qufl] => Some(&qufl.hamil),
            _ => None,
        }
    }

    /// The strings more than one contributor holds unexpired locks on —
    /// what a merge leaves behind when both sides were editing one string.
    #[must_use]
    pub fn mushtaraka(&self, alaan: u64) -> Vec<NassId> {
        self.aqfal
            .iter()
            .filter(|(_, quyud)| {
                let mut hamiluha: Vec<&MusahimId> = quyud
                    .iter()
                    .filter(|qufl| !qufl.muntahi(alaan))
                    .map(|qufl| &qufl.hamil)
                    .collect();
                hamiluha.sort_unstable();
                hamiluha.dedup();
                hamiluha.len() > 1
            })
            .map(|(nass, _)| *nass)
            .collect()
    }
}

/// Merges two lock sets: the union of both sides' locks unexpired at the
/// given instant, identical records kept once, two holders on one string
/// kept as two locks.
#[must_use]
pub fn damj_aqfal(ana: &AqfalMashru, hum: &AqfalMashru, alaan: u64) -> AqfalMashru {
    let mut madmuja = AqfalMashru::jadeed();
    for qufl in ana.kull().chain(hum.kull()) {
        if qufl.muntahi(alaan) {
            continue;
        }
        let quyud = madmuja.aqfal.entry(qufl.nass).or_default();
        if !quyud.contains(qufl) {
            quyud.push(qufl.clone());
        }
    }
    madmuja
}
