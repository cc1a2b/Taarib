//! Per-string change history: append-only, browsable, and recoverable.

use std::collections::{BTreeMap, BTreeSet};

use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{MudkhalNass, NassId};

use crate::damj::{Qarar, QaydHasm};

/// One recorded change to one string's translation.
///
/// Field order is the record's identity order: string first, then the
/// timestamp string, so a deduplicated set already reads per string in time
/// order — for display, never for resolution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct QaydTarikh {
    /// The string.
    pub nass: NassId,
    /// When, RFC 3339, supplied by the caller.
    pub waqt: String,
    /// Who made the change.
    pub kaatib: MusahimId,
    /// The translation before the change.
    pub sabiq: Option<String>,
    /// The translation after it.
    pub jadeed: Option<String>,
    /// Why, where a reason was given.
    pub sabab: Option<String>,
}

/// Every recorded change in one project, append-only.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TarikhMashru {
    quyud: Vec<QaydTarikh>,
}

impl TarikhMashru {
    /// An empty history.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Appends one record.
    pub fn adkhil(&mut self, qayd: QaydTarikh) {
        self.quyud.push(qayd);
    }

    /// Appends a list of records in the order given.
    pub fn adkhil_kul(&mut self, quyud: impl IntoIterator<Item = QaydTarikh>) {
        self.quyud.extend(quyud);
    }

    /// Records one edit, refusing a record whose before and after are equal,
    /// and answering whether one was appended.
    pub fn sajjil_tabdeel(
        &mut self,
        nass: NassId,
        sabiq: Option<String>,
        jadeed: Option<String>,
        kaatib: MusahimId,
        waqt: String,
        sabab: Option<String>,
    ) -> bool {
        if sabiq == jadeed {
            return false;
        }
        self.quyud.push(QaydTarikh {
            nass,
            waqt,
            kaatib,
            sabiq,
            jadeed,
            sabab,
        });
        true
    }

    /// Appends one record per merge resolution, carrying the discarded side's
    /// text as `sabiq` so it stays recoverable; `jadeed` is the kept text
    /// looked up in the merged table. Returns how many were appended.
    pub fn adkhil_husum(
        &mut self,
        husum: &[QaydHasm],
        madmuja: &[MudkhalNass],
        waqt: &str,
    ) -> usize {
        let bil_id: BTreeMap<NassId, &MudkhalNass> = madmuja
            .iter()
            .map(|mudkhal| (mudkhal.id, mudkhal))
            .collect();
        for hasm in husum {
            let jadeed = bil_id
                .get(&hasm.nass)
                .and_then(|mudkhal| mudkhal.hadaf.clone());
            self.quyud.push(QaydTarikh {
                nass: hasm.nass,
                waqt: waqt.to_owned(),
                kaatib: hasm.hasim.clone(),
                sabiq: Some(hasm.mutrah.hadaf.clone()),
                jadeed,
                sabab: Some(wasf_hasm(&hasm.qarar)),
            });
        }
        husum.len()
    }

    /// Every record, in the order it was appended.
    #[must_use]
    pub fn kull(&self) -> &[QaydTarikh] {
        &self.quyud
    }

    /// How many records the history holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.quyud.len()
    }

    /// How many records one string holds.
    #[must_use]
    pub fn adad_li(&self, nass: NassId) -> usize {
        self.quyud.iter().filter(|qayd| qayd.nass == nass).count()
    }

    /// Whether nothing has been recorded.
    #[must_use]
    pub const fn farigh(&self) -> bool {
        self.quyud.is_empty()
    }

    /// One string's records, ordered by timestamp string for display.
    #[must_use]
    pub fn bi_nass(&self, nass: NassId) -> Vec<&QaydTarikh> {
        let mut mahdar: Vec<&QaydTarikh> =
            self.quyud.iter().filter(|qayd| qayd.nass == nass).collect();
        mahdar.sort_by(|awwal, thani| awwal.waqt.cmp(&thani.waqt));
        mahdar
    }

    /// One string's records inside a timestamp-string range, inclusive,
    /// ordered for display.
    #[must_use]
    pub fn bayn(&self, nass: NassId, min: &str, ila: &str) -> Vec<&QaydTarikh> {
        self.bi_nass(nass)
            .into_iter()
            .filter(|qayd| qayd.waqt.as_str() >= min && qayd.waqt.as_str() <= ila)
            .collect()
    }

    /// Every record one author wrote, ordered by timestamp string for display.
    #[must_use]
    pub fn li_kaatib(&self, kaatib: &MusahimId) -> Vec<&QaydTarikh> {
        let mut mahdar: Vec<&QaydTarikh> = self
            .quyud
            .iter()
            .filter(|qayd| qayd.kaatib == *kaatib)
            .collect();
        mahdar.sort_by(|awwal, thani| awwal.waqt.cmp(&thani.waqt));
        mahdar
    }

    /// The latest record of one string, by timestamp string.
    #[must_use]
    pub fn akhir(&self, nass: NassId) -> Option<&QaydTarikh> {
        self.quyud
            .iter()
            .filter(|qayd| qayd.nass == nass)
            .max_by(|awwal, thani| awwal.waqt.cmp(&thani.waqt))
    }

    /// The record in force at a timestamp string — the latest one at or
    /// before it — whose `jadeed` is the text the string held then.
    #[must_use]
    pub fn inda(&self, nass: NassId, waqt: &str) -> Option<&QaydTarikh> {
        self.quyud
            .iter()
            .filter(|qayd| qayd.nass == nass && qayd.waqt.as_str() <= waqt)
            .max_by(|awwal, thani| awwal.waqt.cmp(&thani.waqt))
    }

    /// The translation a string held at a timestamp string, when history
    /// reaches back that far: the outer [`None`] means no record exists at or
    /// before that point, the inner one means the string was untranslated.
    #[must_use]
    #[expect(
        clippy::option_option,
        reason = "the two levels answer two questions, as the paragraph above says: whether \
                  history reaches that far, and whether the string was translated when it did"
    )]
    pub fn istirja(&self, nass: NassId, waqt: &str) -> Option<Option<String>> {
        self.inda(nass, waqt).map(|qayd| qayd.jadeed.clone())
    }

    /// The record that would revert one string to its state at `ila_waqt`,
    /// with the current text as `sabiq`, or [`None`] when history does not
    /// reach that point or the text is already there.
    ///
    /// Returned, never applied: the project layer owns the write and appends
    /// this record itself, so a revert is one more entry, not an erasure.
    #[must_use]
    pub fn qayd_istirja(
        &self,
        nass: NassId,
        ila_waqt: &str,
        hali: Option<String>,
        kaatib: MusahimId,
        waqt: String,
    ) -> Option<QaydTarikh> {
        let hadaf = self.istirja(nass, ila_waqt)?;
        if hadaf == hali {
            return None;
        }
        Some(QaydTarikh {
            nass,
            waqt,
            kaatib,
            sabiq: hali,
            jadeed: hadaf,
            sabab: Some(format!("استُرجع إلى الحالة المسجّلة عند {ila_waqt}")),
        })
    }

    /// The strings that carry any history, in identity order.
    #[must_use]
    pub fn nusus(&self) -> BTreeSet<NassId> {
        self.quyud.iter().map(|qayd| qayd.nass).collect()
    }

    /// Everyone who appears as an author, in identity order.
    #[must_use]
    pub fn kuttab(&self) -> BTreeSet<&MusahimId> {
        self.quyud.iter().map(|qayd| &qayd.kaatib).collect()
    }

    /// One string's ledger, ordered for display.
    #[must_use]
    pub fn mahdar(&self, nass: NassId) -> MahdarNass<'_> {
        MahdarNass {
            nass,
            quyud: self.bi_nass(nass),
        }
    }
}

/// One string's history as the browser shows it, in timestamp-string order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MahdarNass<'a> {
    nass: NassId,
    quyud: Vec<&'a QaydTarikh>,
}

impl<'a> MahdarNass<'a> {
    /// The string this ledger belongs to.
    #[must_use]
    pub const fn nass(&self) -> NassId {
        self.nass
    }

    /// The records, oldest first.
    #[must_use]
    pub fn quyud(&self) -> &[&'a QaydTarikh] {
        &self.quyud
    }

    /// How many records the ledger holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.quyud.len()
    }

    /// The oldest record.
    #[must_use]
    pub fn awwal(&self) -> Option<&'a QaydTarikh> {
        self.quyud.first().copied()
    }

    /// The newest record.
    #[must_use]
    pub fn akhir(&self) -> Option<&'a QaydTarikh> {
        self.quyud.last().copied()
    }

    /// The authors who appear in this ledger, in identity order.
    #[must_use]
    pub fn kuttab(&self) -> BTreeSet<&'a MusahimId> {
        self.quyud.iter().map(|qayd| &qayd.kaatib).collect()
    }

    /// The ledger's header line, in Arabic.
    #[must_use]
    pub fn tarwisa_arabiya(&self) -> String {
        format!(
            "{} تغييرًا على هذه العبارة بأقلام {} مساهمًا.",
            self.adad(),
            self.kuttab().len()
        )
    }

    /// The same, in English.
    #[must_use]
    pub fn tarwisa_injiliziya(&self) -> String {
        format!(
            "{} change(s) to this string by {} contributor(s).",
            self.adad(),
            self.kuttab().len()
        )
    }
}

/// The sentence a merge resolution leaves in the history.
fn wasf_hasm(qarar: &Qarar) -> String {
    match qarar {
        Qarar::KhudhLi => "حُسم تعارض دمج بإبقاء الجانب المحلي".to_owned(),
        Qarar::KhudhHum => "حُسم تعارض دمج بإبقاء الجانب الوارد".to_owned(),
        Qarar::Thalith { .. } => "حُسم تعارض دمج بنصّ ثالث".to_owned(),
    }
}

/// Merges two histories: the union by full content identity, never by
/// position, ordered per string by timestamp string — an ordering for
/// display, not a resolution by timestamp.
#[must_use]
pub fn damj_tarikh(ana: &TarikhMashru, hum: &TarikhMashru) -> TarikhMashru {
    let muwahhada: BTreeSet<QaydTarikh> =
        ana.quyud.iter().chain(hum.quyud.iter()).cloned().collect();
    TarikhMashru {
        quyud: muwahhada.into_iter().collect(),
    }
}
