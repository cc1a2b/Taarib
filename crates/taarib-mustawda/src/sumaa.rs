//! Contributor reputation: aggregating and rendering the shared `Sumaa`.

use serde::{Deserialize, Serialize};
use taarib_mustalahat::musahim::{MusahimId, Sumaa};
use taarib_mustalahat::ruqaa::MulakhkhasRuqaa;

/// How many decided submissions a record needs before its acceptance ratio is
/// shown as a judgement rather than as a first impression.
pub const ADNA_QARARAT: u32 = 3;

/// The acceptance ratio below which a record is called out on the listing.
pub const HADD_QUBUL_MUNKHAFID: f32 = 0.5;

/// The review outcomes the caller counted.
///
/// This crate does not own the review history; Phase 18 does. These are the
/// counts it hands over, and nothing here derives them from anything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdadMuraja {
    /// Patches published.
    pub manshura: u32,
    /// Submissions accepted with no changes requested.
    pub qubila_bila_taadil: u32,
    /// Submissions returned for revision.
    pub tulib_taadil: u32,
    /// Submissions rejected outright.
    pub marfuda: u32,
    /// Published patches later revoked.
    pub masbuba: u32,
}

impl AdadMuraja {
    /// How many submissions were decided one way or another.
    #[must_use]
    pub const fn qararat(&self) -> u32 {
        self.qubila_bila_taadil
            .saturating_add(self.tulib_taadil)
            .saturating_add(self.marfuda)
    }

    /// Whether the caller supplied no history at all.
    #[must_use]
    pub const fn faarigh(&self) -> bool {
        self.qararat() == 0 && self.manshura == 0 && self.masbuba == 0
    }

    /// Folds another set of counts into this one.
    pub const fn damm(&mut self, akhar: &Self) {
        self.manshura = self.manshura.saturating_add(akhar.manshura);
        self.qubila_bila_taadil =
            self.qubila_bila_taadil.saturating_add(akhar.qubila_bila_taadil);
        self.tulib_taadil = self.tulib_taadil.saturating_add(akhar.tulib_taadil);
        self.marfuda = self.marfuda.saturating_add(akhar.marfuda);
        self.masbuba = self.masbuba.saturating_add(akhar.masbuba);
    }
}

/// One published patch's rating aggregate, as its listing carries it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaqyeemManshur {
    /// The average, when anyone has rated.
    pub mutawassit: Option<f32>,
    /// How many ratings that average rests on.
    pub adad: u32,
}

impl TaqyeemManshur {
    /// Reads the aggregate off a registry listing.
    #[must_use]
    pub const fn min_mulakhkhas(ruqaa: &MulakhkhasRuqaa) -> Self {
        Self { mutawassit: ruqaa.taqyeem, adad: ruqaa.adad_taqyeemat }
    }

    /// The average and its count, when the pair is usable.
    #[must_use]
    pub fn salih(self) -> Option<(f32, u32)> {
        let mutawassit = self.mutawassit?;
        if self.adad == 0 || !mutawassit.is_finite() {
            return None;
        }
        Some((mutawassit, self.adad))
    }
}

/// Combines per-patch rating aggregates into one weighted average.
///
/// Returns `None` for the average when no patch carries a usable rating, and
/// never substitutes a zero for one.
#[must_use]
pub fn jami_taqyeemat(
    taqyeemat: impl IntoIterator<Item = TaqyeemManshur>,
) -> (Option<f32>, u32) {
    let mut majmu = 0.0_f64;
    let mut adad = 0_u32;
    for taqyeem in taqyeemat {
        let Some((mutawassit, adad_ruqaa)) = taqyeem.salih() else { continue };
        majmu = f64::from(mutawassit).mul_add(f64::from(adad_ruqaa), majmu);
        adad = adad.saturating_add(adad_ruqaa);
    }
    if adad == 0 {
        return (None, 0);
    }
    (Some(ila_f32(majmu / f64::from(adad))), adad)
}

/// Builds a reputation record from caller-supplied counts and rating
/// aggregates.
#[must_use]
pub fn ijma(
    adad: &AdadMuraja,
    taqyeemat: impl IntoIterator<Item = TaqyeemManshur>,
) -> Sumaa {
    let (mutawassit_taqyeem, adad_taqyeemat) = jami_taqyeemat(taqyeemat);
    Sumaa {
        ruqaa_manshura: adad.manshura,
        qubila_bila_taadil: adad.qubila_bila_taadil,
        tulib_taadil: adad.tulib_taadil,
        marfuda: adad.marfuda,
        masbuba: adad.masbuba,
        mutawassit_taqyeem,
        adad_taqyeemat,
    }
}

/// Builds a reputation record from counts and the contributor's own listings.
#[must_use]
pub fn ijma_min_mulakhkhasat(
    musahim: &MusahimId,
    adad: &AdadMuraja,
    ruqaa: &[MulakhkhasRuqaa],
) -> Sumaa {
    ijma(adad, taqyeemat_lil_musahim(musahim, ruqaa))
}

/// Every rating aggregate belonging to one contributor's listings.
pub fn taqyeemat_lil_musahim<'a>(
    musahim: &'a MusahimId,
    ruqaa: &'a [MulakhkhasRuqaa],
) -> impl Iterator<Item = TaqyeemManshur> + 'a {
    ruqaa
        .iter()
        .filter(move |wahida| &wahida.musahim == musahim)
        .map(TaqyeemManshur::min_mulakhkhas)
}

/// How many published listings a contributor has in a set.
#[must_use]
pub fn manshura_lil_musahim(musahim: &MusahimId, ruqaa: &[MulakhkhasRuqaa]) -> u32 {
    let adad = ruqaa.iter().filter(|wahida| &wahida.musahim == musahim).count();
    u32::try_from(adad).unwrap_or(u32::MAX)
}

/// How much history a reputation rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HalatSumaa {
    /// Nothing published and nothing decided: no reputation exists yet.
    LaTareekh,
    /// Some history, but fewer decisions than the display threshold.
    Mubtadi,
    /// Enough decisions for the acceptance ratio to carry meaning.
    Mustaqirra,
}

impl HalatSumaa {
    /// Whether there is any history to render.
    #[must_use]
    pub const fn yujad_tareekh(self) -> bool {
        !matches!(self, Self::LaTareekh)
    }

    /// The state's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::LaTareekh => "لا سجل بعد",
            Self::Mubtadi => "سجل قصير",
            Self::Mustaqirra => "سجل مستقر",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::LaTareekh => "No history yet",
            Self::Mubtadi => "Short history",
            Self::Mustaqirra => "Established history",
        }
    }
}

/// A revocation notice, kept separate because a revoked patch is the strongest
/// negative signal a listing can carry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TahdheerSahb {
    /// How many published patches were revoked.
    pub adad: u32,
    /// How many were published in total.
    pub min_manshura: u32,
    /// The share of published patches revoked, when anything was published.
    pub nisba: Option<f32>,
}

impl TahdheerSahb {
    /// The notice as one line, in Arabic.
    #[must_use]
    pub fn satr_arabi(self) -> String {
        match self.nisba {
            Some(nisba) => format!(
                "أُلغيت {} من {} رقعة منشورة بعد نشرها ({:.0}٪).",
                self.adad,
                self.min_manshura,
                nisba * 100.0
            ),
            None => format!("أُلغيت {} رقعة بعد نشرها.", self.adad),
        }
    }

    /// The same line in English.
    #[must_use]
    pub fn satr_injilizi(self) -> String {
        match self.nisba {
            Some(nisba) => format!(
                "{} of {} published patches were revoked after release ({:.0}%).",
                self.adad,
                self.min_manshura,
                nisba * 100.0
            ),
            None => format!("{} published patches were revoked after release.", self.adad),
        }
    }
}

/// A contributor's standing, ready to render on a listing.
///
/// No `Deserialize`: [`HalatSumaa`] is classified from the counts by
/// [`MulakhkhasSumaa::jadeed`], and a wire form could contradict them.
#[derive(Debug, Clone, Serialize)]
pub struct MulakhkhasSumaa {
    musahim: MusahimId,
    hala: HalatSumaa,
    sumaa: Sumaa,
}

impl MulakhkhasSumaa {
    /// Wraps a reputation record and classifies how much history it rests on.
    #[must_use]
    pub const fn jadeed(musahim: MusahimId, sumaa: Sumaa) -> Self {
        let qararat = sumaa
            .qubila_bila_taadil
            .saturating_add(sumaa.tulib_taadil)
            .saturating_add(sumaa.marfuda);
        let hala = if qararat == 0 && sumaa.ruqaa_manshura == 0 && sumaa.masbuba == 0 {
            HalatSumaa::LaTareekh
        } else if qararat < ADNA_QARARAT {
            HalatSumaa::Mubtadi
        } else {
            HalatSumaa::Mustaqirra
        };
        Self { musahim, hala, sumaa }
    }

    /// Aggregates counts and listings, then classifies the result.
    #[must_use]
    pub fn min_muraja(
        musahim: MusahimId,
        adad: &AdadMuraja,
        ruqaa: &[MulakhkhasRuqaa],
    ) -> Self {
        let sumaa = ijma_min_mulakhkhasat(&musahim, adad, ruqaa);
        Self::jadeed(musahim, sumaa)
    }

    /// Whose standing this is.
    #[must_use]
    pub const fn musahim(&self) -> &MusahimId {
        &self.musahim
    }

    /// How much history it rests on.
    #[must_use]
    pub const fn hala(&self) -> HalatSumaa {
        self.hala
    }

    /// The underlying record.
    #[must_use]
    pub const fn sumaa(&self) -> &Sumaa {
        &self.sumaa
    }

    /// The share of submissions accepted without a revision round.
    ///
    /// `None` when nothing has been decided, because there is no ratio to show
    /// and a zero would read as one.
    #[must_use]
    pub fn nisbat_qubul(&self) -> Option<f32> {
        let qararat = self
            .sumaa
            .qubila_bila_taadil
            .saturating_add(self.sumaa.tulib_taadil)
            .saturating_add(self.sumaa.marfuda);
        (qararat > 0).then(|| self.sumaa.nisbat_qubul())
    }

    /// Whether the acceptance ratio is below the call-out threshold.
    #[must_use]
    pub fn qubul_munkhafid(&self) -> bool {
        self.hala == HalatSumaa::Mustaqirra
            && self
                .nisbat_qubul()
                .is_some_and(|nisba| nisba.total_cmp(&HADD_QUBUL_MUNKHAFID).is_lt())
    }

    /// How many published patches were revoked.
    #[must_use]
    pub const fn masbuba(&self) -> u32 {
        self.sumaa.masbuba
    }

    /// The revocation notice, when there is anything to report.
    #[must_use]
    pub fn tahdheer_sahb(&self) -> Option<TahdheerSahb> {
        if self.sumaa.masbuba == 0 {
            return None;
        }
        let manshura = self.sumaa.ruqaa_manshura;
        Some(TahdheerSahb {
            adad: self.sumaa.masbuba,
            min_manshura: manshura,
            nisba: (manshura > 0).then(|| {
                ila_f32(f64::from(self.sumaa.masbuba) / f64::from(manshura))
            }),
        })
    }

    /// The whole standing as one line, in Arabic.
    #[must_use]
    pub fn satr_arabi(&self) -> String {
        use std::fmt::Write as _;
        if self.hala == HalatSumaa::LaTareekh {
            return "لا سجل لهذا المساهم بعد.".to_owned();
        }
        let mut satr = format!("{} رقعة منشورة", self.sumaa.ruqaa_manshura);
        if let Some(nisba) = self.nisbat_qubul() {
            let qararat = self
                .sumaa
                .qubila_bila_taadil
                .saturating_add(self.sumaa.tulib_taadil)
                .saturating_add(self.sumaa.marfuda);
            let _ = write!(
                satr,
                "، وقُبلت {} من {} دون تعديل ({:.0}٪)",
                self.sumaa.qubila_bila_taadil,
                qararat,
                nisba * 100.0
            );
        }
        if let Some(mutawassit) = self.sumaa.mutawassit_taqyeem {
            let _ = write!(
                satr,
                "، ومتوسط التقييم {mutawassit:.1} من ٥ عن {} تقييمًا",
                self.sumaa.adad_taqyeemat
            );
        }
        match self.tahdheer_sahb() {
            Some(tahdheer) => {
                satr.push_str("، و");
                satr.push_str(&tahdheer.satr_arabi());
            }
            None => satr.push('.'),
        }
        if self.hala == HalatSumaa::Mubtadi {
            satr.push_str(" (سجل قصير — لم تُتخذ قرارات كافية بعد.)");
        }
        satr
    }

    /// The same line in English.
    #[must_use]
    pub fn satr_injilizi(&self) -> String {
        use std::fmt::Write as _;
        if self.hala == HalatSumaa::LaTareekh {
            return "No history yet for this contributor.".to_owned();
        }
        let mut satr = format!("{} published patches", self.sumaa.ruqaa_manshura);
        if let Some(nisba) = self.nisbat_qubul() {
            let qararat = self
                .sumaa
                .qubila_bila_taadil
                .saturating_add(self.sumaa.tulib_taadil)
                .saturating_add(self.sumaa.marfuda);
            let _ = write!(
                satr,
                ", {} of {} accepted with no changes requested ({:.0}%)",
                self.sumaa.qubila_bila_taadil,
                qararat,
                nisba * 100.0
            );
        }
        if let Some(mutawassit) = self.sumaa.mutawassit_taqyeem {
            let _ = write!(
                satr,
                ", rated {mutawassit:.1} of 5 across {} ratings",
                self.sumaa.adad_taqyeemat
            );
        }
        match self.tahdheer_sahb() {
            Some(tahdheer) => {
                satr.push_str(". ");
                satr.push_str(&tahdheer.satr_injilizi());
            }
            None => satr.push('.'),
        }
        if self.hala == HalatSumaa::Mubtadi {
            satr.push_str(" (Short history — too few decisions to judge yet.)");
        }
        satr
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "ratings are summed in f64 for accuracy, and `Sumaa` stores the result as the \
              f32 it was averaged from, so narrowing back is what this function is for"
)]
const fn ila_f32(qeema: f64) -> f32 {
    qeema as f32
}
