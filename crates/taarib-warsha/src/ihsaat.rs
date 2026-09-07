//! Contribution statistics, measured from the review history and never from a stored counter.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_mustalahat::ruqaa::TareeqaTarjama;

/// The method bucket a string's transitions are counted under.
///
/// A statistics bucket rather than [`TareeqaTarjama`] itself, because a
/// string's method is optional: work done before a method was recorded is
/// still work, and folding it into any real method would misstate all three.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TareeqaIhsa {
    /// The string is recorded as fully human work.
    Bashariya,
    /// Machine-translated, then reviewed and corrected by a person.
    AaliyaThumBashariya,
    /// Machine-translated with no human review.
    AaliyaFaqat,
    /// The string carries no recorded method.
    GhayrMusajjala,
}

impl TareeqaIhsa {
    /// Every bucket, in the order the rendered table walks them.
    pub const KUL: [Self; 4] = [
        Self::Bashariya,
        Self::AaliyaThumBashariya,
        Self::AaliyaFaqat,
        Self::GhayrMusajjala,
    ];

    /// The bucket a string's recorded method lands in.
    #[must_use]
    pub const fn min_tareeqa(tareeqa: Option<TareeqaTarjama>) -> Self {
        match tareeqa {
            Some(TareeqaTarjama::BashariyaKamila) => Self::Bashariya,
            Some(TareeqaTarjama::AaliyaThumBashariya) => Self::AaliyaThumBashariya,
            Some(TareeqaTarjama::AaliyaFaqat) => Self::AaliyaFaqat,
            None => Self::GhayrMusajjala,
        }
    }

    /// The label the table shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Bashariya => TareeqaTarjama::BashariyaKamila.wasf_arabi(),
            Self::AaliyaThumBashariya => TareeqaTarjama::AaliyaThumBashariya.wasf_arabi(),
            Self::AaliyaFaqat => TareeqaTarjama::AaliyaFaqat.wasf_arabi(),
            Self::GhayrMusajjala => "طريقة غير مسجَّلة",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Bashariya => TareeqaTarjama::BashariyaKamila.wasf_injilizi(),
            Self::AaliyaThumBashariya => TareeqaTarjama::AaliyaThumBashariya.wasf_injilizi(),
            Self::AaliyaFaqat => TareeqaTarjama::AaliyaFaqat.wasf_injilizi(),
            Self::GhayrMusajjala => "method not recorded",
        }
    }
}

/// Transition counts for one contributor within one method bucket.
///
/// Every number is a count of recorded transitions, so a string a person
/// drafted twice counts twice: the credit line credits work done, and the
/// history is the record of the work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AdadAamal {
    /// Transitions into [`HalatMuraja::Musawwada`] — drafts and edits.
    pub tarjamat: u64,
    /// Transitions into [`HalatMuraja::Muakkada`] or [`HalatMuraja::Marfuda`]
    /// — review verdicts of either kind.
    pub murajaat: u64,
    /// Transitions into [`HalatMuraja::Muakkada`] only.
    pub iatimadat: u64,
}

impl AdadAamal {
    /// Whether nothing was counted.
    #[must_use]
    pub const fn farigh(&self) -> bool {
        self.tarjamat == 0 && self.murajaat == 0 && self.iatimadat == 0
    }

    /// Folds another count in.
    pub const fn jam(&mut self, akhar: &Self) {
        self.tarjamat = self.tarjamat.saturating_add(akhar.tarjamat);
        self.murajaat = self.murajaat.saturating_add(akhar.murajaat);
        self.iatimadat = self.iatimadat.saturating_add(akhar.iatimadat);
    }

    /// Counts one transition a person made.
    const fn adif_bashari(&mut self, ila: HalatMuraja) {
        match ila {
            HalatMuraja::Musawwada => self.tarjamat = self.tarjamat.saturating_add(1),
            HalatMuraja::Muakkada => {
                self.murajaat = self.murajaat.saturating_add(1);
                self.iatimadat = self.iatimadat.saturating_add(1);
            },
            HalatMuraja::Marfuda => self.murajaat = self.murajaat.saturating_add(1),
            HalatMuraja::LamTutarjam | HalatMuraja::TarjamaAaliya | HalatMuraja::LilMuraja => {},
        }
    }

    /// Counts one author-less transition.
    ///
    /// An author-less verdict cannot be produced by this build; where an
    /// imported history carries one anyway it stays in the machine bucket,
    /// because the one thing this module must never do is put it on a person.
    const fn adif_aali(&mut self, ila: HalatMuraja) {
        match ila {
            HalatMuraja::TarjamaAaliya | HalatMuraja::Musawwada => {
                self.tarjamat = self.tarjamat.saturating_add(1);
            },
            HalatMuraja::Muakkada => {
                self.murajaat = self.murajaat.saturating_add(1);
                self.iatimadat = self.iatimadat.saturating_add(1);
            },
            HalatMuraja::Marfuda => self.murajaat = self.murajaat.saturating_add(1),
            HalatMuraja::LamTutarjam | HalatMuraja::LilMuraja => {},
        }
    }
}

/// One contributor's counts, split across the four method buckets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MasfufatTareeqa {
    /// Counts on strings recorded as fully human work.
    pub bashariya: AdadAamal,
    /// Counts on machine-then-human strings.
    pub aaliya_thum_bashariya: AdadAamal,
    /// Counts on machine-only strings.
    pub aaliya_faqat: AdadAamal,
    /// Counts on strings with no recorded method.
    pub ghayr_musajjala: AdadAamal,
}

impl MasfufatTareeqa {
    /// The counts of one bucket.
    #[must_use]
    pub const fn li(&self, tareeqa: TareeqaIhsa) -> &AdadAamal {
        match tareeqa {
            TareeqaIhsa::Bashariya => &self.bashariya,
            TareeqaIhsa::AaliyaThumBashariya => &self.aaliya_thum_bashariya,
            TareeqaIhsa::AaliyaFaqat => &self.aaliya_faqat,
            TareeqaIhsa::GhayrMusajjala => &self.ghayr_musajjala,
        }
    }

    /// The counts of one bucket, writable.
    const fn li_mut(&mut self, tareeqa: TareeqaIhsa) -> &mut AdadAamal {
        match tareeqa {
            TareeqaIhsa::Bashariya => &mut self.bashariya,
            TareeqaIhsa::AaliyaThumBashariya => &mut self.aaliya_thum_bashariya,
            TareeqaIhsa::AaliyaFaqat => &mut self.aaliya_faqat,
            TareeqaIhsa::GhayrMusajjala => &mut self.ghayr_musajjala,
        }
    }

    /// The four buckets summed.
    #[must_use]
    pub const fn majmu(&self) -> AdadAamal {
        let mut majmu = self.bashariya;
        majmu.jam(&self.aaliya_thum_bashariya);
        majmu.jam(&self.aaliya_faqat);
        majmu.jam(&self.ghayr_musajjala);
        majmu
    }

    /// Whether every bucket is empty.
    #[must_use]
    pub const fn farigh(&self) -> bool {
        self.majmu().farigh()
    }
}

/// One contributor's measured statistics within one project.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IhsaatMusahim {
    /// Who.
    pub musahim: MusahimId,
    /// Their counts, per method bucket.
    pub tareeqat: MasfufatTareeqa,
    /// Their earliest recorded transition, Unix seconds.
    pub awwal_lahza: Option<u64>,
    /// Their latest recorded transition, Unix seconds.
    pub akhir_lahza: Option<u64>,
}

impl IhsaatMusahim {
    fn jadeed(musahim: MusahimId) -> Self {
        Self {
            musahim,
            tareeqat: MasfufatTareeqa::default(),
            awwal_lahza: None,
            akhir_lahza: None,
        }
    }

    /// Their counts across every method bucket.
    #[must_use]
    pub const fn majmu(&self) -> AdadAamal {
        self.tareeqat.majmu()
    }

    /// The credit line: totals with Arabic and English labels.
    #[must_use]
    pub fn satr_itiraf(&self) -> String {
        let majmu = self.majmu();
        format!(
            "{} — ترجم {} وراجع {} واعتمد {} | translated {}, reviewed {}, approved {}",
            self.musahim.mukhtasar(),
            majmu.tarjamat,
            majmu.murajaat,
            majmu.iatimadat,
            majmu.tarjamat,
            majmu.murajaat,
            majmu.iatimadat,
        )
    }

    fn sajjil(&mut self, tareeqa: TareeqaIhsa, ila: HalatMuraja, lahza: u64) {
        self.tareeqat.li_mut(tareeqa).adif_bashari(ila);
        self.awwal_lahza = Some(self.awwal_lahza.map_or(lahza, |q| q.min(lahza)));
        self.akhir_lahza = Some(self.akhir_lahza.map_or(lahza, |q| q.max(lahza)));
    }
}

/// The whole project's measured statistics.
///
/// Computed from `&[MudkhalNass]` on every call and stored nowhere: a counter
/// that lived anywhere else could drift from the history it claims to
/// summarize, and the history is the record.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IhsaatMashru {
    /// Per contributor, ordered by [`MusahimId`] so the table renders the
    /// same on every machine.
    pub musahimun: Vec<IhsaatMusahim>,
    /// Every author-less transition, kept apart and never on a person.
    pub aali: MasfufatTareeqa,
    /// How many strings were walked.
    pub adad_nusus: usize,
}

impl IhsaatMashru {
    /// The table the credit screen shows: one row per contributor, ordered by
    /// identity, each with its per-method breakdown, and the machine bucket
    /// last, all labelled in Arabic and English.
    #[must_use]
    pub fn jadwal(&self) -> String {
        let mut natija = String::new();
        let _ = writeln!(
            natija,
            "إحصاءات المساهمة، محسوبة من سجلّ المراجعة | contribution statistics, \
             measured from the review history"
        );
        let _ = writeln!(
            natija,
            "عدد النصوص المفحوصة | strings examined: {}",
            self.adad_nusus
        );

        for ihsaat in &self.musahimun {
            let _ = writeln!(natija, "{}", ihsaat.satr_itiraf());
            if let (Some(awwal), Some(akhir)) = (ihsaat.awwal_lahza, ihsaat.akhir_lahza) {
                let _ = writeln!(
                    natija,
                    "    المدة | span: {awwal} .. {akhir} (Unix seconds)"
                );
            }
            for tareeqa in TareeqaIhsa::KUL {
                satr_tareeqa(&mut natija, tareeqa, ihsaat.tareeqat.li(tareeqa));
            }
        }

        let majmu_aali = self.aali.majmu();
        if !majmu_aali.farigh() {
            let _ = writeln!(
                natija,
                "الآلة — لا يُنسب عملها لأحد | machine — attributed to nobody: \
                 ترجمات آلية {} | machine translations {}",
                majmu_aali.tarjamat, majmu_aali.tarjamat
            );
            for tareeqa in TareeqaIhsa::KUL {
                satr_tareeqa(&mut natija, tareeqa, self.aali.li(tareeqa));
            }
        }
        natija
    }
}

fn satr_tareeqa(natija: &mut String, tareeqa: TareeqaIhsa, adad: &AdadAamal) {
    if adad.farigh() {
        return;
    }
    let _ = writeln!(
        natija,
        "    {} | {}: ترجم {}، راجع {}، اعتمد {} | translated {}, reviewed {}, approved {}",
        tareeqa.wasf_arabi(),
        tareeqa.wasf_injilizi(),
        adad.tarjamat,
        adad.murajaat,
        adad.iatimadat,
        adad.tarjamat,
        adad.murajaat,
        adad.iatimadat,
    );
}

/// Measures every contributor's statistics from the string table's own
/// review histories.
///
/// The counting rule, exactly: a transition into
/// [`HalatMuraja::Musawwada`] by a person is a translation or edit; a
/// transition into [`HalatMuraja::Muakkada`] or [`HalatMuraja::Marfuda`] by a
/// person is a review, and into [`HalatMuraja::Muakkada`] also an approval;
/// every transition whose `musahim` is [`None`] lands in the machine bucket
/// whatever its target state, because a transition with no author is one no
/// person made. Each transition is counted under the bucket of its string's
/// recorded method.
#[must_use]
pub fn ihsib(nusus: &[MudkhalNass]) -> IhsaatMashru {
    let mut bil_musahim: BTreeMap<MusahimId, IhsaatMusahim> = BTreeMap::new();
    let mut aali = MasfufatTareeqa::default();

    for mudkhal in nusus {
        let tareeqa = TareeqaIhsa::min_tareeqa(mudkhal.tareeqa);
        for intiqal in mudkhal.muraja.tareekh() {
            match &intiqal.musahim {
                Some(musahim) => {
                    bil_musahim
                        .entry(musahim.clone())
                        .or_insert_with(|| IhsaatMusahim::jadeed(musahim.clone()))
                        .sajjil(tareeqa, intiqal.ila, intiqal.lahza);
                },
                None => aali.li_mut(tareeqa).adif_aali(intiqal.ila),
            }
        }
    }

    IhsaatMashru {
        musahimun: bil_musahim.into_values().collect(),
        aali,
        adad_nusus: nusus.len(),
    }
}
