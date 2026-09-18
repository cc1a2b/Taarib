//! Merging the shared resources: the glossary by source term, the memory by re-recording.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_tarjama::dhakira::{
    AslQayd, Dhakira, NawAsl, QaydDhakira, QaydId, QaydJadid, miftah_muwahhad,
};
use taarib_tarjama::khata::KhataTarjama;
use taarib_tarjama::masrad::{
    Masrad, MustalahMasrad, NitaqMustalah, TadarubMustalah, TadeelMustalah, wahhid_tadarub,
};
use taarib_usus::khata::Tafsir;

use crate::damj::{Janib, Qarar};
use crate::khata::{KhataWarsha, NatijatWarsha};

/// One glossary conflict: one source term with two different approved forms.
///
/// Both sides are carried whole — raw source form, approved Arabic, note,
/// scope, do-not-translate flag — so the resolver shows provenance rather
/// than a diff of two strings. Resolved only by an explicit [`Qarar`]; there
/// is no default and no timestamp to rank by, because a glossary term has no
/// clock and a vocabulary decision is not a race.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NizaaMustalah {
    /// The shared identity: the folded source form both entries answer to.
    pub miftah: String,
    /// The local entry, whole.
    pub ana: MustalahMasrad,
    /// The imported entry, whole.
    pub hum: MustalahMasrad,
}

impl NizaaMustalah {
    /// The conflict card: the term, then both sides whole with their scope,
    /// flag and note, labelled in Arabic and English for the resolver.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut natija = String::new();
        let _ = writeln!(natija, "المصطلح | term: {}", self.ana.masdar);
        satr_janib(&mut natija, "نسختي | mine", &self.ana);
        satr_janib(&mut natija, "نسختهم | theirs", &self.hum);
        natija
    }
}

fn satr_janib(natija: &mut String, unwan: &str, mustalah: &MustalahMasrad) {
    let _ = write!(
        natija,
        "  {unwan}: «{}» — النطاق | scope: {} | {}",
        mustalah.arabi,
        mustalah.nitaq.wasf_arabi(),
        mustalah.nitaq.wasf_injilizi(),
    );
    if mustalah.la_yutarjam {
        let _ = write!(natija, " — لا يُترجم | do not translate");
    }
    match &mustalah.mulahaza {
        Some(mulahaza) => {
            let _ = writeln!(natija, " — ملاحظة | note: {mulahaza}");
        },
        None => {
            let _ = writeln!(natija);
        },
    }
}

/// Counts of what the glossary merge did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerMasrad {
    /// Terms both copies carried with the same approved form.
    pub mutatabiqa: usize,
    /// Terms only one copy carried, merged silently.
    pub munfarida: usize,
    /// Conflicts raised.
    pub nizaat: usize,
}

/// What one glossary resolution chose, what it discarded, and the one-action
/// rewrite it earns.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HasmMustalah {
    /// The term's folded identity.
    pub miftah: String,
    /// The entry as it entered the merged glossary.
    pub mustalah: MustalahMasrad,
    /// The choice made.
    pub qarar: Qarar,
    /// The side that was kept, when a side was.
    pub ukhidh: Option<Janib>,
    /// The discarded entry, whole, so it is recoverable.
    pub mutrah: MustalahMasrad,
    /// Who resolved it.
    pub hasim: MusahimId,
    /// When, Unix seconds, supplied by the caller.
    pub lahza: u64,
    /// Every whole-string occurrence the merged table renders otherwise, as
    /// ready edits for the project layer to apply in one action. Occurrences
    /// buried inside sentences are policed by [`Masrad::afhas`] instead —
    /// flagged, never rewritten.
    pub taadilat: Vec<TadeelMustalah>,
}

/// A glossary merge with its conflicts held open.
///
/// No public constructor and no accessor that hands out a merged glossary
/// while conflicts remain: the only way to a finished [`Masrad`] is
/// [`DamjMasrad::itmam`], which demands one explicit [`Qarar`] per conflict.
#[derive(Debug)]
pub struct DamjMasrad {
    muttafaq: BTreeMap<String, MustalahMasrad>,
    nizaat: Vec<NizaaMustalah>,
    taqreer: TaqreerMasrad,
}

impl DamjMasrad {
    /// The open conflicts, for the resolver, ordered by term identity.
    #[must_use]
    pub fn nizaat(&self) -> &[NizaaMustalah] {
        &self.nizaat
    }

    /// What the merge did so far.
    #[must_use]
    pub const fn taqreer(&self) -> &TaqreerMasrad {
        &self.taqreer
    }

    /// Whether the merge can finish without any resolution.
    #[must_use]
    pub const fn bila_nizaa(&self) -> bool {
        self.nizaat.is_empty()
    }

    /// Finalizes the merge, consuming one explicit resolution per conflict.
    ///
    /// Each resolution is applied once — the chosen entry becomes the
    /// glossary's single form for its term — and earns its one-action
    /// rewrite: the merged glossary's consistency pass
    /// ([`Masrad::tadarubat`]) runs once over `nusus`, the merged string
    /// table, and every resolved term's divergent whole-string occurrences
    /// come back as [`TadeelMustalah`] edits via [`wahhid_tadarub`]. Nothing
    /// is written here: the project layer is the table's only writer and
    /// owns the review transition each edit implies.
    ///
    /// A third form ([`Qarar::Thalith`]) keeps the local entry's source
    /// form, scope and note, carries the new Arabic, and recomputes the
    /// do-not-translate flag from it.
    ///
    /// # Errors
    ///
    /// [`KhataWarsha::NizaatMuallaqa`] when any conflict has no resolution,
    /// [`KhataWarsha::QararBilaNizaa`] when a resolution names a term that
    /// is not in conflict, and [`KhataWarsha::Mawrid`] when a third form
    /// folds to nothing and would enter the glossary unenforceable.
    pub fn itmam(
        mut self,
        qararat: &BTreeMap<String, Qarar>,
        nusus: &[MudkhalNass],
        hasim: &MusahimId,
        lahza: u64,
    ) -> NatijatWarsha<(Masrad, Vec<HasmMustalah>)> {
        let maruf: BTreeSet<&str> = self
            .nizaat
            .iter()
            .map(|nizaa| nizaa.miftah.as_str())
            .collect();
        if qararat
            .keys()
            .any(|miftah| !maruf.contains(miftah.as_str()))
        {
            return Err(KhataWarsha::QararBilaNizaa);
        }
        let muallaq = self
            .nizaat
            .iter()
            .filter(|n| !qararat.contains_key(&n.miftah))
            .count();
        if muallaq > 0 {
            return Err(KhataWarsha::NizaatMuallaqa { adad: muallaq });
        }

        let mut makhtara = Vec::with_capacity(self.nizaat.len());
        for nizaa in self.nizaat {
            let Some(qarar) = qararat.get(&nizaa.miftah) else {
                return Err(KhataWarsha::NizaatMuallaqa { adad: 1 });
            };
            let (mustalah, ukhidh, mutrah) = match qarar {
                Qarar::KhudhLi => (nizaa.ana, Some(Janib::Ana), nizaa.hum),
                Qarar::KhudhHum => (nizaa.hum, Some(Janib::Hum), nizaa.ana),
                Qarar::Thalith { nass } => {
                    let thalith = MustalahMasrad {
                        masdar: nizaa.ana.masdar.clone(),
                        arabi: nass.clone(),
                        mulahaza: nizaa
                            .ana
                            .mulahaza
                            .clone()
                            .or_else(|| nizaa.hum.mulahaza.clone()),
                        nitaq: nizaa.ana.nitaq,
                        la_yutarjam: miftah_muwahhad(nass) == nizaa.miftah,
                    };
                    (thalith, None, nizaa.hum)
                },
            };
            if !mustalah.salih() {
                return Err(KhataWarsha::Mawrid {
                    sabab: format!(
                        "the resolution for the term {:?} has no enforceable approved \
                         form and was not applied",
                        mustalah.masdar
                    ),
                });
            }
            let _ = self.muttafaq.insert(nizaa.miftah.clone(), mustalah.clone());
            makhtara.push((nizaa.miftah, mustalah, qarar.clone(), ukhidh, mutrah));
        }

        let masrad = Masrad::bi_mustalahat(self.muttafaq.into_values().collect());

        let tadarubat: BTreeMap<String, TadarubMustalah> = masrad
            .tadarubat(nusus)
            .into_iter()
            .map(|tadarub| (miftah_muwahhad(&tadarub.mustalah), tadarub))
            .collect();

        let mut husum = Vec::with_capacity(makhtara.len());
        for (miftah, mustalah, qarar, ukhidh, mutrah) in makhtara {
            let taadilat = tadarubat
                .get(&miftah)
                .map(|tadarub| wahhid_tadarub(tadarub, &mustalah.arabi, nusus))
                .unwrap_or_default();
            husum.push(HasmMustalah {
                miftah,
                mustalah,
                qarar,
                ukhidh,
                mutrah,
                hasim: hasim.clone(),
                lahza,
                taadilat,
            });
        }
        Ok((masrad, husum))
    }
}

/// Merges two glossaries by source term identity ([`MustalahMasrad::miftah`]).
///
/// A term one copy carries merges silently. A term both carry with one
/// approved form — equal after [`miftah_muwahhad`] folding, so «القوّة» and
/// «القوة» are one form — keeps the local entry, adopting the imported note
/// only where the local has none. A term both carry with two approved forms
/// is a [`NizaaMustalah`], held open until [`DamjMasrad::itmam`].
///
/// Built-in terms ([`NitaqMustalah::Mudmaj`]) are dropped from both sides
/// before anything is compared, because the merged glossary is written to the
/// project's own file and shared in bundles, and a built-in term is neither
/// side's data: persisting it would freeze today's shipped list into a
/// project that should simply get tomorrow's, and would raise a conflict card
/// between two copies of a default neither collaborator wrote. Both machines
/// already have the built-in list; it is in force either way.
#[must_use]
pub fn idmij_masrad(ana: &Masrad, hum: &Masrad) -> DamjMasrad {
    let mut ana_bil_miftah: BTreeMap<String, MustalahMasrad> = ana
        .mustalahat()
        .filter(|mustalah| mustalah.nitaq != NitaqMustalah::Mudmaj)
        .map(|mustalah| (mustalah.miftah(), mustalah.clone()))
        .collect();

    let mut muttafaq = BTreeMap::new();
    let mut nizaat = Vec::new();
    let mut taqreer = TaqreerMasrad::default();

    for mustalah_hum in hum
        .mustalahat()
        .filter(|mustalah| mustalah.nitaq != NitaqMustalah::Mudmaj)
    {
        let miftah = mustalah_hum.miftah();
        match ana_bil_miftah.remove(&miftah) {
            None => {
                taqreer.munfarida = taqreer.munfarida.saturating_add(1);
                let _ = muttafaq.insert(miftah, mustalah_hum.clone());
            },
            Some(mustalah_ana) => {
                let arabi_ana = miftah_muwahhad(&mustalah_ana.arabi);
                let arabi_hum = miftah_muwahhad(&mustalah_hum.arabi);
                if arabi_ana == arabi_hum {
                    taqreer.mutatabiqa = taqreer.mutatabiqa.saturating_add(1);
                    let mut makhtar = mustalah_ana;
                    if makhtar.mulahaza.is_none() {
                        makhtar.mulahaza.clone_from(&mustalah_hum.mulahaza);
                    }
                    let _ = muttafaq.insert(miftah, makhtar);
                } else {
                    nizaat.push(NizaaMustalah {
                        miftah,
                        ana: mustalah_ana,
                        hum: mustalah_hum.clone(),
                    });
                }
            },
        }
    }

    for (miftah, mustalah) in ana_bil_miftah {
        taqreer.munfarida = taqreer.munfarida.saturating_add(1);
        let _ = muttafaq.insert(miftah, mustalah);
    }

    nizaat.sort_by(|awwal, thani| awwal.miftah.cmp(&thani.miftah));
    taqreer.nizaat = nizaat.len();

    DamjMasrad {
        muttafaq,
        nizaat,
        taqreer,
    }
}

/// How many records one page of a memory merge reads.
///
/// 256: large enough that a hundred-thousand-record merge is a few hundred
/// statements, small enough that a page never holds more than a screenful of
/// cutscene text in memory at once.
pub const HAJM_SAFHA_DAMJ: u32 = 256;

/// Counts of what a memory merge did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerDamjDhakira {
    /// Records read from the source memory.
    pub zurat: u64,
    /// Records re-recorded into the target — inserted new, or folded into an
    /// existing row by [`Dhakira::sajjil`]'s upsert.
    pub sujjilat: u64,
}

/// Merges one memory into another by re-recording every pair.
///
/// Pages through `masdar` with [`Dhakira::safha`] and re-enters each record
/// through [`Dhakira::sajjil`], whose upsert holds the provenance ratchet —
/// `max(existing, incoming)` on both the review bit and the origin rank — so
/// this function carries the origin faithfully and the database enforces that
/// no merge order can demote or manufacture a human review. The mapping back
/// to the write-side origin is exact and total over [`NawAsl`]: a
/// human-reviewed record travels as [`AslQayd::Bashari`] with its
/// contributor, a machine-only record as [`AslQayd::AaliFaqat`] with its
/// provider and confidence, an overlay reading as [`AslQayd::Mulahaza`] with
/// its recognizer and its measured-or-not confidence, and there is no fourth
/// path.
///
/// One counter does not survive a merge and cannot: `mushahadat`. The upsert
/// *adds* the incoming sighting count, so re-recording a record that already
/// carries ten sightings into a memory that carries them too would claim
/// twenty independent readings where there may have been ten. This function
/// therefore re-enters each reading as **one** sighting, which understates
/// corroboration rather than inflating it — the direction to be wrong in for
/// a number that decides which of two readings a player is shown.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when reading a page from the source or
/// re-recording a pair into the target fails, carrying the memory's own
/// explanation.
pub fn idmij_dhakira(hadaf: &mut Dhakira, masdar: &Dhakira) -> NatijatWarsha<TaqreerDamjDhakira> {
    let hadd_safha = usize::try_from(HAJM_SAFHA_DAMJ).unwrap_or(usize::MAX);
    let mut taqreer = TaqreerDamjDhakira::default();
    let mut baad: Option<QaydId> = None;

    loop {
        let safha = masdar
            .safha(baad, HAJM_SAFHA_DAMJ)
            .map_err(|khata| mawrid(&khata))?;
        let Some(akhir) = safha.last() else {
            return Ok(taqreer);
        };
        baad = Some(akhir.id);

        for qayd in &safha {
            taqreer.zurat = taqreer.zurat.saturating_add(1);
            if miftah_muwahhad(&qayd.masdar).is_empty() || miftah_muwahhad(&qayd.hadaf).is_empty() {
                // sajjil skips an unlookupable pair silently; visited, not stored.
                continue;
            }
            hadaf
                .sajjil(&qayd_lil_tasjil(qayd))
                .map_err(|khata| mawrid(&khata))?;
            taqreer.sujjilat = taqreer.sujjilat.saturating_add(1);
        }

        if safha.len() < hadd_safha {
            return Ok(taqreer);
        }
    }
}

/// A stored record dressed for re-recording, origin mapped faithfully.
pub(crate) fn qayd_lil_tasjil(qayd: &QaydDhakira) -> QaydJadid {
    QaydJadid {
        masdar: qayd.masdar.clone(),
        hadaf: qayd.hadaf.clone(),
        tasnif: qayd.tasnif,
        mashru: qayd.asl.mashru.clone(),
        luba: qayd.asl.luba.clone(),
        ism_luba: qayd.asl.ism_luba.clone(),
        siyaq: qayd.siyaq.clone(),
        nasq_masdar: qayd.nasq_masdar.clone(),
        nasq_hadaf: qayd.nasq_hadaf.clone(),
        asl: asl_amin(qayd),
    }
}

/// The origin mapping: the stored kind decides the variant, nothing else does.
///
/// A `match` over [`NawAsl`] rather than a branch on the review bit, so that
/// the day a fourth kind exists this function fails to compile instead of
/// quietly filing it as a machine translation. A machine-only or observed
/// record can never travel as `Bashari`, because that would record a human
/// review nobody performed.
pub(crate) fn asl_amin(qayd: &QaydDhakira) -> AslQayd {
    match qayd.asl.naw {
        NawAsl::Bashari => AslQayd::Bashari {
            musahim: qayd.asl.musahim.clone(),
        },
        NawAsl::Aali => AslQayd::AaliFaqat {
            muzawwid: qayd.asl.muzawwid.clone(),
            thiqa: qayd.thiqa,
        },
        NawAsl::Mulahaza => AslQayd::Mulahaza {
            qari: qayd.asl.qari.clone(),
            muzawwid: qayd.asl.muzawwid.clone(),
            thiqa: qayd.asl.thiqa_qira,
        },
    }
}

/// The memory's or glossary's own refusal, wrapped as this crate's.
fn mawrid(khata: &KhataTarjama) -> KhataWarsha {
    KhataWarsha::Mawrid {
        sabab: khata.injilizi(),
    }
}
