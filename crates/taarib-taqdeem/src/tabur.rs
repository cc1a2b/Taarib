//! The owner's queue of pending submissions, and the working set over it.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::muharrik::AilatMuharrik;
use taarib_mustalahat::musahim::{MusahimId, Sumaa};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_mustalahat::taghtiya::Taghtiya;

use crate::hawiya::SalahiyatMalik;

/// Seconds in a minute.
const THAWANI_DAQIQA: u64 = 60;

/// Minutes in an hour.
const DAQAIQ_SAA: u64 = 60;

/// Minutes in a day.
const DAQAIQ_YAWM: u64 = 1_440;

/// Where the automated pre-flight checks stand for one pending submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "halat", rename_all = "snake_case")]
pub enum HalatFuhus {
    /// Every check passed.
    Najahat,
    /// Checks failed.
    Akhfaqat {
        /// How many.
        adad: u32,
    },
    /// The checks have not run against this revision.
    LamTujra,
}

impl HalatFuhus {
    /// Whether this status blocks the submission from being approved.
    #[must_use]
    pub const fn yamnaa(self) -> bool {
        matches!(self, Self::Akhfaqat { .. })
    }

    /// How many checks failed, which is zero for every other status.
    #[must_use]
    pub const fn adad_ikhfaq(self) -> u32 {
        match self {
            Self::Akhfaqat { adad } => adad,
            Self::Najahat | Self::LamTujra => 0,
        }
    }

    /// The grouping key, failures first.
    #[must_use]
    pub const fn rutba(self) -> u8 {
        match self {
            Self::Akhfaqat { .. } => 0,
            Self::LamTujra => 1,
            Self::Najahat => 2,
        }
    }

    /// The stable key the status column is filtered and stored by.
    #[must_use]
    pub const fn ramz(self) -> &'static str {
        match self {
            Self::Najahat => "najahat",
            Self::Akhfaqat { .. } => "akhfaqat",
            Self::LamTujra => "lam_tujra",
        }
    }

    /// The status column's text, in Arabic.
    #[must_use]
    pub fn wasf_arabi(self) -> String {
        match self {
            Self::Najahat => "اجتازت الفحوص".to_owned(),
            Self::Akhfaqat { adad } => format!("أخفقت في {adad} فحصًا"),
            Self::LamTujra => "لم تُجرَ الفحوص".to_owned(),
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn wasf_injilizi(self) -> String {
        match self {
            Self::Najahat => "checks passed".to_owned(),
            Self::Akhfaqat { adad } => format!("{adad} check(s) failed"),
            Self::LamTujra => "checks not run".to_owned(),
        }
    }
}

/// One pending submission, as the queue lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MudkhalTabur {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision waiting.
    pub murajaa: RuqaaRevision,
    /// The title the contributor gave it.
    pub unwan: String,
    /// The game it targets.
    pub luba: LubaId,
    /// That game's name, denormalized so the row needs no second lookup.
    pub ism_luba: String,
    /// The engine it targets.
    pub aila: AilatMuharrik,
    /// Who submitted it.
    pub musahim: MusahimId,
    /// Their display name.
    pub ism_musahim: String,
    /// Their standing at the moment the row was built.
    pub sumaa: Sumaa,
    /// How the translation was produced, as the contributor declared it.
    pub tareeqa: TareeqaTarjama,
    /// How much of the game it covers.
    pub taghtiya: Taghtiya,
    /// When it was submitted, RFC 3339.
    pub waqt_taqdeem: String,
    /// How long it has waited, in minutes.
    pub umr_daqaiq: u64,
    /// Where the automated checks stand.
    pub fuhus: HalatFuhus,
}

impl MudkhalTabur {
    /// Recomputes [`MudkhalTabur::umr_daqaiq`] against `alaan`, answering
    /// whether the row's own timestamp and `alaan` were both readable.
    pub fn jaddid_umr(&mut self, alaan: &str) -> bool {
        match umr_bil_daqaiq(&self.waqt_taqdeem, alaan) {
            Some(daqaiq) => {
                self.umr_daqaiq = daqaiq;
                true
            },
            None => false,
        }
    }

    /// The wait, written the way the column shows it, in Arabic.
    #[must_use]
    pub fn umr_arabi(&self) -> String {
        let ayyam = self.umr_daqaiq.div_euclid(DAQAIQ_YAWM);
        if ayyam > 0 {
            return format!("{ayyam} يومًا");
        }
        let saaat = self.umr_daqaiq.div_euclid(DAQAIQ_SAA);
        if saaat > 0 {
            return format!("{saaat} ساعة");
        }
        format!("{} دقيقة", self.umr_daqaiq)
    }

    /// The same, in English.
    #[must_use]
    pub fn umr_injilizi(&self) -> String {
        let ayyam = self.umr_daqaiq.div_euclid(DAQAIQ_YAWM);
        if ayyam > 0 {
            return format!("{ayyam}d");
        }
        let saaat = self.umr_daqaiq.div_euclid(DAQAIQ_SAA);
        if saaat > 0 {
            return format!("{saaat}h");
        }
        format!("{}m", self.umr_daqaiq)
    }

    /// The row's identity, which is what a bulk action is issued against.
    #[must_use]
    pub const fn hadaf(&self) -> (RuqaaId, RuqaaRevision) {
        (self.ruqaa, self.murajaa)
    }
}

/// Minutes between two RFC 3339 instants.
///
/// Answers [`None`] when either does not parse, or when `ila` precedes
/// `mundhu`; no clock is read here or anywhere in this module.
#[must_use]
pub fn umr_bil_daqaiq(mundhu: &str, ila: &str) -> Option<u64> {
    let bidaya = mundhu.parse::<Timestamp>().ok()?;
    let nihaya = ila.parse::<Timestamp>().ok()?;
    let thawani = nihaya.as_second().saturating_sub(bidaya.as_second());
    u64::try_from(thawani)
        .ok()
        .map(|qeema| qeema.div_euclid(THAWANI_DAQIQA))
}

/// What the queue is narrowed to, with [`None`] meaning no constraint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MurashshihTabur {
    /// One game.
    pub luba: Option<LubaId>,
    /// One engine family.
    pub aila: Option<AilatMuharrik>,
    /// One contributor.
    pub musahim: Option<MusahimId>,
    /// One declared translation method.
    pub tareeqa: Option<TareeqaTarjama>,
    /// The lowest coverage by string, 0.0 to 1.0, that still appears.
    pub hadd_taghtiya: Option<f32>,
    /// The shortest wait, in minutes, that still appears.
    pub hadd_umr_daqaiq: Option<u64>,
    /// One automated-check status, matched by [`HalatFuhus::ramz`] so a failure
    /// count does not have to be guessed to filter for failures.
    pub ramz_fuhus: Option<String>,
}

impl MurashshihTabur {
    /// A filter that constrains nothing.
    #[must_use]
    pub const fn maftuh() -> Self {
        Self {
            luba: None,
            aila: None,
            musahim: None,
            tareeqa: None,
            hadd_taghtiya: None,
            hadd_umr_daqaiq: None,
            ramz_fuhus: None,
        }
    }

    /// Whether every constraint is absent.
    #[must_use]
    pub const fn farigh(&self) -> bool {
        self.luba.is_none()
            && self.aila.is_none()
            && self.musahim.is_none()
            && self.tareeqa.is_none()
            && self.hadd_taghtiya.is_none()
            && self.hadd_umr_daqaiq.is_none()
            && self.ramz_fuhus.is_none()
    }

    /// Whether one row survives every constraint.
    #[must_use]
    pub fn yaqbal(&self, madkhal: &MudkhalTabur) -> bool {
        if self.luba.is_some_and(|luba| luba != madkhal.luba) {
            return false;
        }
        if self.aila.is_some_and(|aila| aila != madkhal.aila) {
            return false;
        }
        if self
            .musahim
            .as_ref()
            .is_some_and(|musahim| *musahim != madkhal.musahim)
        {
            return false;
        }
        if self
            .tareeqa
            .is_some_and(|tareeqa| tareeqa != madkhal.tareeqa)
        {
            return false;
        }
        // `total_cmp`, never `==`: `float_cmp` is denied and a coverage floor
        // compared by equality would drop the row it is meant to keep.
        if self
            .hadd_taghtiya
            .is_some_and(|hadd| madkhal.taghtiya.nisba().total_cmp(&hadd).is_lt())
        {
            return false;
        }
        if self
            .hadd_umr_daqaiq
            .is_some_and(|hadd| madkhal.umr_daqaiq < hadd)
        {
            return false;
        }
        if self
            .ramz_fuhus
            .as_ref()
            .is_some_and(|ramz| ramz != madkhal.fuhus.ramz())
        {
            return false;
        }
        true
    }
}

/// The order the queue is worked in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TarteebTabur {
    /// Longest wait first.
    #[default]
    Intizar,
    /// Highest coverage first.
    Taghtiya,
    /// Highest contributor acceptance rate first.
    Sumaa,
}

impl TarteebTabur {
    /// The label the column header shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Intizar => "مدّة الانتظار",
            Self::Taghtiya => "التغطية",
            Self::Sumaa => "سمعة المساهم",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Intizar => "wait time",
            Self::Taghtiya => "coverage",
            Self::Sumaa => "contributor reputation",
        }
    }
}

/// The filtered, sorted queue.
///
/// The authority argument is unused for logic and required anyway: it is the
/// gate, and there is no boolean here for a later edit to invert.
#[must_use]
pub fn tabur<'a>(
    _salahiya: &SalahiyatMalik,
    sufuf: &'a [MudkhalTabur],
    murashshih: &MurashshihTabur,
    tarteeb: TarteebTabur,
) -> Vec<&'a MudkhalTabur> {
    let mut natija: Vec<&MudkhalTabur> = sufuf
        .iter()
        .filter(|madkhal| murashshih.yaqbal(madkhal))
        .collect();
    natija.sort_by(|awwal, thani| qarin(awwal, thani, tarteeb));
    natija
}

/// The queue's shape at a glance, for the console header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IhsaTabur {
    /// How many rows the filter kept.
    pub majmu: usize,
    /// Of those, how many failed their automated checks.
    pub akhfaqat: usize,
    /// How many have not had their checks run.
    pub lam_tujra: usize,
    /// How many passed.
    pub najahat: usize,
    /// The longest wait among them, in minutes.
    pub aqsa_umr_daqaiq: u64,
}

/// Counts the filtered queue by check status and longest wait.
#[must_use]
pub fn ihsa(_salahiya: &SalahiyatMalik, sufuf: &[&MudkhalTabur]) -> IhsaTabur {
    let mut natija = IhsaTabur::default();
    for madkhal in sufuf {
        natija.majmu = natija.majmu.saturating_add(1);
        natija.aqsa_umr_daqaiq = natija.aqsa_umr_daqaiq.max(madkhal.umr_daqaiq);
        match madkhal.fuhus {
            HalatFuhus::Akhfaqat { .. } => {
                natija.akhfaqat = natija.akhfaqat.saturating_add(1);
            },
            HalatFuhus::LamTujra => natija.lam_tujra = natija.lam_tujra.saturating_add(1),
            HalatFuhus::Najahat => natija.najahat = natija.najahat.saturating_add(1),
        }
    }
    natija
}

/// Orders two rows, breaking every tie down to the row identity so that two
/// runs over the same data produce the same sequence.
fn qarin(awwal: &MudkhalTabur, thani: &MudkhalTabur, tarteeb: TarteebTabur) -> Ordering {
    let asasi = match tarteeb {
        TarteebTabur::Intizar => qarin_waqt(&awwal.waqt_taqdeem, &thani.waqt_taqdeem),
        TarteebTabur::Taghtiya => thani.taghtiya.nisba().total_cmp(&awwal.taghtiya.nisba()),
        TarteebTabur::Sumaa => thani
            .sumaa
            .nisbat_qubul()
            .total_cmp(&awwal.sumaa.nisbat_qubul()),
    };
    asasi
        .then_with(|| qarin_waqt(&awwal.waqt_taqdeem, &thani.waqt_taqdeem))
        .then_with(|| awwal.ruqaa.cmp(&thani.ruqaa))
        .then_with(|| awwal.murajaa.cmp(&thani.murajaa))
}

/// Oldest first, with an unreadable timestamp sorted last rather than dropped.
fn qarin_waqt(awwal: &str, thani: &str) -> Ordering {
    match (
        awwal.parse::<Timestamp>().ok(),
        thani.parse::<Timestamp>().ok(),
    ) {
        (Some(lahza_awwal), Some(lahza_thani)) => lahza_awwal.cmp(&lahza_thani),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => awwal.cmp(thani),
    }
}

/// A keyboard-driven working set over one ordered queue.
///
/// Positions index the queue **as it was last returned by [`tabur`]**; any
/// refilter or resort must call [`IkhtiyarTabur::ghayyir_tul`], which drops
/// positions the shorter list no longer has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IkhtiyarTabur {
    tul: usize,
    murakkaz: Option<usize>,
    murtakaz: Option<usize>,
    mukhtar: BTreeSet<usize>,
}

impl IkhtiyarTabur {
    /// A working set over a queue of `tul` rows, focused on the first.
    #[must_use]
    pub fn jadeed(tul: usize) -> Self {
        Self {
            tul,
            murakkaz: (tul > 0).then_some(0),
            murtakaz: None,
            mukhtar: BTreeSet::new(),
        }
    }

    /// How many rows the queue had when this working set was last resized.
    #[must_use]
    pub const fn tul(&self) -> usize {
        self.tul
    }

    /// The focused position.
    #[must_use]
    pub const fn murakkaz(&self) -> Option<usize> {
        self.murakkaz
    }

    /// The anchor a range selection extends from.
    #[must_use]
    pub const fn murtakaz(&self) -> Option<usize> {
        self.murtakaz
    }

    /// Every selected position, ascending.
    #[must_use]
    pub const fn mukhtar(&self) -> &BTreeSet<usize> {
        &self.mukhtar
    }

    /// How many rows are selected.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mukhtar.len()
    }

    /// Whether nothing is selected.
    #[must_use]
    pub fn farigh(&self) -> bool {
        self.mukhtar.is_empty()
    }

    /// Whether one position is selected.
    #[must_use]
    pub fn muhaddad(&self, mawdi: usize) -> bool {
        self.mukhtar.contains(&mawdi)
    }

    /// Moves the focus one row down.
    pub fn talia(&mut self) -> Option<usize> {
        self.harrik(|mawdi, akhir| mawdi.saturating_add(1).min(akhir))
    }

    /// Moves the focus one row up.
    pub fn sabiq(&mut self) -> Option<usize> {
        self.harrik(|mawdi, _| mawdi.saturating_sub(1))
    }

    /// Moves the focus to the first row.
    pub fn awwal(&mut self) -> Option<usize> {
        self.harrik(|_, _| 0)
    }

    /// Moves the focus to the last row.
    pub fn akhir(&mut self) -> Option<usize> {
        self.harrik(|_, akhir| akhir)
    }

    /// Focuses one position, answering whether the queue has it.
    pub const fn ila(&mut self, mawdi: usize) -> bool {
        if mawdi >= self.tul {
            return false;
        }
        self.murakkaz = Some(mawdi);
        true
    }

    /// Selects the focused row and makes it the range anchor.
    pub fn ikhtar(&mut self) -> bool {
        let Some(mawdi) = self.murakkaz else {
            return false;
        };
        self.murtakaz = Some(mawdi);
        self.mukhtar.insert(mawdi)
    }

    /// Deselects the focused row.
    pub fn ilghi(&mut self) -> bool {
        let Some(mawdi) = self.murakkaz else {
            return false;
        };
        self.mukhtar.remove(&mawdi)
    }

    /// Toggles the focused row, answering whether it ended up selected.
    pub fn baddil(&mut self) -> Option<bool> {
        let mawdi = self.murakkaz?;
        if self.mukhtar.remove(&mawdi) {
            return Some(false);
        }
        self.murtakaz = Some(mawdi);
        let _ = self.mukhtar.insert(mawdi);
        Some(true)
    }

    /// Selects every row between the anchor and the focus, inclusive, and
    /// answers how many positions that added.
    pub fn madd(&mut self) -> usize {
        let Some(murakkaz) = self.murakkaz else {
            return 0;
        };
        let murtakaz = self.murtakaz.unwrap_or(murakkaz);
        let (min, aqsa) = if murtakaz <= murakkaz {
            (murtakaz, murakkaz)
        } else {
            (murakkaz, murtakaz)
        };
        let mut mudaf = 0_usize;
        for mawdi in min..=aqsa.min(self.tul.saturating_sub(1)) {
            if self.mukhtar.insert(mawdi) {
                mudaf = mudaf.saturating_add(1);
            }
        }
        self.murtakaz = Some(murtakaz);
        mudaf
    }

    /// Selects every row in the queue.
    pub fn ikhtar_alkull(&mut self) {
        self.mukhtar = (0..self.tul).collect();
    }

    /// Selects exactly the rows that were not selected.
    pub fn aks(&mut self) {
        let qadeem = std::mem::take(&mut self.mukhtar);
        self.mukhtar = (0..self.tul)
            .filter(|mawdi| !qadeem.contains(mawdi))
            .collect();
    }

    /// Clears the selection and the range anchor, leaving the focus alone.
    pub fn imsah(&mut self) {
        self.mukhtar.clear();
        self.murtakaz = None;
    }

    /// Resizes the working set after a refilter or a resort, dropping every
    /// position the new queue does not have.
    pub fn ghayyir_tul(&mut self, tul: usize) {
        self.tul = tul;
        self.mukhtar.retain(|mawdi| *mawdi < tul);
        self.murakkaz = match self.murakkaz {
            Some(mawdi) if mawdi < tul => Some(mawdi),
            _ => (tul > 0).then_some(0),
        };
        self.murtakaz = self.murtakaz.filter(|mawdi| *mawdi < tul);
    }

    /// The focused row of `saff`.
    #[must_use]
    pub fn saff_murakkaz<'a>(&self, saff: &[&'a MudkhalTabur]) -> Option<&'a MudkhalTabur> {
        saff.get(self.murakkaz?).copied()
    }

    /// The selected rows of `saff`, in queue order.
    #[must_use]
    pub fn mukhtarat<'a>(&self, saff: &[&'a MudkhalTabur]) -> Vec<&'a MudkhalTabur> {
        self.mukhtar
            .iter()
            .filter_map(|mawdi| saff.get(*mawdi).copied())
            .collect()
    }

    /// The selected rows as the identities a bulk action is issued against.
    #[must_use]
    pub fn ahdaf(&self, saff: &[&MudkhalTabur]) -> Vec<(RuqaaId, RuqaaRevision)> {
        self.mukhtar
            .iter()
            .filter_map(|mawdi| saff.get(*mawdi).map(|madkhal| madkhal.hadaf()))
            .collect()
    }

    fn harrik(&mut self, hisab: impl Fn(usize, usize) -> usize) -> Option<usize> {
        if self.tul == 0 {
            self.murakkaz = None;
            return None;
        }
        let akhir = self.tul.saturating_sub(1);
        let jadid = match self.murakkaz {
            Some(mawdi) => hisab(mawdi, akhir),
            None => 0,
        };
        self.murakkaz = Some(jadid.min(akhir));
        self.murakkaz
    }
}
