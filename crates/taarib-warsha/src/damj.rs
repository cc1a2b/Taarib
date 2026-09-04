//! Merge by string identity, with conflicts held open until explicitly chosen.

use std::collections::BTreeMap;

use taarib_mustalahat::muraja::HalatMuraja;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_mustalahat::ruqaa::TareeqaTarjama;

use crate::khata::{KhataWarsha, NatijatWarsha};

/// Which copy a value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Janib {
    /// The local copy.
    Ana,
    /// The imported copy.
    Hum,
}

/// One side of a conflict, with its full provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BitaqatJanib {
    /// The translation text.
    pub hadaf: String,
    /// Its review state.
    pub hala: HalatMuraja,
    /// How it was produced, when recorded.
    pub tareeqa: Option<TareeqaTarjama>,
    /// The provider, when a machine produced it.
    pub muzawwid: Option<String>,
    /// Who last changed it.
    pub muharrir: Option<MusahimId>,
    /// When it was last changed, RFC 3339.
    pub akhir_tabdeel: Option<String>,
}

impl BitaqatJanib {
    fn min_mudkhal(mudkhal: &MudkhalNass) -> Option<Self> {
        let hadaf = mudkhal.hadaf.clone().filter(|nass| !nass.trim().is_empty())?;
        Some(Self {
            hadaf,
            hala: mudkhal.muraja.hala(),
            tareeqa: mudkhal.tareeqa,
            muzawwid: mudkhal.muzawwid.clone(),
            muharrir: mudkhal.muharrir.clone(),
            akhir_tabdeel: mudkhal.akhir_tabdeel.clone(),
        })
    }
}

/// What kind of disagreement a conflict is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawNizaa {
    /// Two different non-empty translations for one identity.
    Tarjama,
    /// One text, two incompatible human verdicts about it.
    Hala,
}

/// One conflict, both sides shown whole.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Nizaa {
    /// The string.
    pub nass: NassId,
    /// Its source text, for the resolver's context pane.
    pub masdar: String,
    /// What it is a disagreement about.
    pub naw: NawNizaa,
    /// The local side.
    pub ana: BitaqatJanib,
    /// The imported side.
    pub hum: BitaqatJanib,
}

/// An explicit resolution for one conflict.
///
/// There is no default, no "latest wins" variant and no policy that resolves
/// by timestamp: a conflict ends only by naming a side or writing a third
/// text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "ikhtiyar", rename_all = "snake_case")]
pub enum Qarar {
    /// Keep the local side.
    KhudhLi,
    /// Keep the imported side.
    KhudhHum,
    /// Write a third text, which enters review from the beginning.
    Thalith {
        /// The new translation.
        nass: String,
    },
}

/// What a resolution chose and what it discarded, kept recoverable.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QaydHasm {
    /// The string.
    pub nass: NassId,
    /// The choice made.
    pub qarar: Qarar,
    /// The side that was kept, when a side was.
    pub ukhidh: Option<Janib>,
    /// The discarded side, whole, so it is recoverable.
    pub mutrah: BitaqatJanib,
    /// Who resolved it.
    pub hasim: MusahimId,
    /// When, Unix seconds, supplied by the caller.
    pub lahza: u64,
}

/// Counts of what the merge did, per dimension.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaqreerDamj {
    /// Whether a common ancestor was available.
    pub thulathi: bool,
    /// Strings identical on both sides.
    pub mutatabiqa: usize,
    /// Strings taken from the local side without conflict.
    pub min_ana: usize,
    /// Strings taken from the imported side without conflict.
    pub min_hum: usize,
    /// Strings where a translation overrode an emptied one.
    pub tarjama_ala_faragh: usize,
    /// Strings only one copy carried.
    pub munfarida: usize,
    /// Conflicts raised.
    pub nizaat: usize,
    /// Strings the ancestor carried that both copies dropped.
    pub saqatat: usize,
}

/// A merge with its conflicts still open.
///
/// No public constructor and no accessor that hands out a merged table while
/// conflicts remain: the only way to a finished table is [`Damj::itmam`],
/// which demands one explicit [`Qarar`] per conflict.
#[derive(Debug)]
pub struct Damj {
    madmuja: Vec<MudkhalNass>,
    nizaat: Vec<Nizaa>,
    ajnab: BTreeMap<NassId, (MudkhalNass, MudkhalNass)>,
    taqreer: TaqreerDamj,
}

impl Damj {
    /// The open conflicts, for the resolver.
    #[must_use]
    pub fn nizaat(&self) -> &[Nizaa] {
        &self.nizaat
    }

    /// What the merge did so far.
    #[must_use]
    pub const fn taqreer(&self) -> &TaqreerDamj {
        &self.taqreer
    }

    /// Whether the merge can finish without any resolution.
    #[must_use]
    pub const fn bila_nizaa(&self) -> bool {
        self.nizaat.is_empty()
    }

    /// Finalizes the merge, consuming one explicit resolution per conflict.
    ///
    /// A kept side moves whole, so a review state only ever travels with the
    /// exact text it was granted against. A third text starts from the local
    /// record, is recorded as `hasim`'s draft, and is returned to needs-review
    /// through the public transitions — there is no path here that constructs
    /// an approval.
    ///
    /// # Errors
    ///
    /// [`KhataWarsha::NizaatMuallaqa`] when any conflict has no resolution,
    /// and [`KhataWarsha::QararBilaNizaa`] when a resolution names a string
    /// that is not in conflict.
    pub fn itmam(
        mut self,
        qararat: &BTreeMap<NassId, Qarar>,
        hasim: &MusahimId,
        lahza: u64,
    ) -> NatijatWarsha<(Vec<MudkhalNass>, Vec<QaydHasm>)> {
        let maruf: BTreeMap<NassId, &Nizaa> =
            self.nizaat.iter().map(|nizaa| (nizaa.nass, nizaa)).collect();
        if qararat.keys().any(|nass| !maruf.contains_key(nass)) {
            return Err(KhataWarsha::QararBilaNizaa);
        }
        let muallaq = self.nizaat.iter().filter(|n| !qararat.contains_key(&n.nass)).count();
        if muallaq > 0 {
            return Err(KhataWarsha::NizaatMuallaqa { adad: muallaq });
        }

        let mut husum = Vec::with_capacity(self.nizaat.len());
        for nizaa in &self.nizaat {
            let Some(qarar) = qararat.get(&nizaa.nass) else {
                return Err(KhataWarsha::NizaatMuallaqa { adad: 1 });
            };
            let Some((ana, hum)) = self.ajnab.remove(&nizaa.nass) else {
                return Err(KhataWarsha::QararBilaNizaa);
            };
            let (makhtar, ukhidh, mutrah) = match qarar {
                Qarar::KhudhLi => (ana, Some(Janib::Ana), nizaa.hum.clone()),
                Qarar::KhudhHum => (hum, Some(Janib::Hum), nizaa.ana.clone()),
                Qarar::Thalith { nass } => {
                    let mut thalith = ana;
                    thalith.hadaf = Some(nass.clone());
                    thalith.muraja.sajjil_musawwada(hasim.clone(), lahza);
                    thalith.muraja.tlub_muraja(
                        Some(hasim.clone()),
                        lahza,
                        Some("حُسم تعارض دمج بنصّ ثالث".to_owned()),
                    );
                    thalith.muharrir = Some(hasim.clone());
                    (thalith, None, nizaa.hum.clone())
                }
            };
            husum.push(QaydHasm {
                nass: nizaa.nass,
                qarar: qarar.clone(),
                ukhidh,
                mutrah,
                hasim: hasim.clone(),
                lahza,
            });
            self.madmuja.push(makhtar);
        }

        self.madmuja.sort_by_key(|mudkhal| mudkhal.id);
        Ok((self.madmuja, husum))
    }
}

/// Merges two copies of one project's string table by identity.
///
/// Three-way against `aslaf` when a common ancestor is available, two-way
/// otherwise; [`TaqreerDamj::thulathi`] states which ran. Non-conflicting
/// changes merge silently; a translation on one side against none on the
/// other is taken, not raised.
#[must_use]
pub fn damj(
    ana: Vec<MudkhalNass>,
    hum: Vec<MudkhalNass>,
    aslaf: Option<&[MudkhalNass]>,
) -> Damj {
    let aslaf_bil_id: BTreeMap<NassId, &MudkhalNass> = aslaf
        .unwrap_or(&[])
        .iter()
        .map(|mudkhal| (mudkhal.id, mudkhal))
        .collect();
    let mut taqreer = TaqreerDamj { thulathi: aslaf.is_some(), ..TaqreerDamj::default() };

    let mut hum_bil_id: BTreeMap<NassId, MudkhalNass> =
        hum.into_iter().map(|mudkhal| (mudkhal.id, mudkhal)).collect();

    let mut madmuja = Vec::new();
    let mut nizaat = Vec::new();
    let mut ajnab = BTreeMap::new();

    for mudkhal_ana in ana {
        let id = mudkhal_ana.id;
        let Some(mudkhal_hum) = hum_bil_id.remove(&id) else {
            taqreer.munfarida = taqreer.munfarida.saturating_add(1);
            madmuja.push(mudkhal_ana);
            continue;
        };
        let salaf = aslaf_bil_id.get(&id).copied();
        idmij_wahid(
            mudkhal_ana,
            mudkhal_hum,
            salaf,
            &mut madmuja,
            &mut nizaat,
            &mut ajnab,
            &mut taqreer,
        );
    }
    for (_, mudkhal_hum) in hum_bil_id {
        taqreer.munfarida = taqreer.munfarida.saturating_add(1);
        madmuja.push(mudkhal_hum);
    }
    let madmuja_ids: std::collections::BTreeSet<NassId> =
        madmuja.iter().map(|m| m.id).collect();
    taqreer.saqatat = aslaf_bil_id
        .keys()
        .filter(|id| !madmuja_ids.contains(id) && !ajnab.contains_key(id))
        .count();
    taqreer.nizaat = nizaat.len();

    Damj { madmuja, nizaat, ajnab, taqreer }
}

fn nass_faal(mudkhal: &MudkhalNass) -> Option<&str> {
    mudkhal.hadaf.as_deref().filter(|nass| !nass.trim().is_empty())
}

fn idmij_wahid(
    ana: MudkhalNass,
    hum: MudkhalNass,
    salaf: Option<&MudkhalNass>,
    madmuja: &mut Vec<MudkhalNass>,
    nizaat: &mut Vec<Nizaa>,
    ajnab: &mut BTreeMap<NassId, (MudkhalNass, MudkhalNass)>,
    taqreer: &mut TaqreerDamj,
) {
    let nass_ana = nass_faal(&ana).map(str::to_owned);
    let nass_hum = nass_faal(&hum).map(str::to_owned);
    let nass_salaf = salaf.and_then(nass_faal).map(str::to_owned);

    match (&nass_ana, &nass_hum) {
        (None, None) => {
            taqreer.mutatabiqa = taqreer.mutatabiqa.saturating_add(1);
            madmuja.push(ana);
        }
        (Some(_), None) => {
            if nass_salaf.is_some() && salaf.is_some() {
                taqreer.tarjama_ala_faragh = taqreer.tarjama_ala_faragh.saturating_add(1);
            } else {
                taqreer.min_ana = taqreer.min_ana.saturating_add(1);
            }
            madmuja.push(ana);
        }
        (None, Some(_)) => {
            if nass_salaf.is_some() && salaf.is_some() {
                taqreer.tarjama_ala_faragh = taqreer.tarjama_ala_faragh.saturating_add(1);
            } else {
                taqreer.min_hum = taqreer.min_hum.saturating_add(1);
            }
            madmuja.push(hum);
        }
        (Some(ni), Some(hu)) if ni == hu => {
            taqreer.mutatabiqa = taqreer.mutatabiqa.saturating_add(1);
            idmij_hala_mutatabiqa(ana, hum, nizaat, ajnab, madmuja);
        }
        (Some(ni), Some(hu)) => {
            if nass_salaf.as_deref() == Some(ni.as_str()) {
                taqreer.min_hum = taqreer.min_hum.saturating_add(1);
                madmuja.push(hum);
            } else if nass_salaf.as_deref() == Some(hu.as_str()) {
                taqreer.min_ana = taqreer.min_ana.saturating_add(1);
                madmuja.push(ana);
            } else {
                let (Some(bitaqat_ana), Some(bitaqat_hum)) =
                    (BitaqatJanib::min_mudkhal(&ana), BitaqatJanib::min_mudkhal(&hum))
                else {
                    madmuja.push(ana);
                    return;
                };
                nizaat.push(Nizaa {
                    nass: ana.id,
                    masdar: ana.masdar.clone(),
                    naw: NawNizaa::Tarjama,
                    ana: bitaqat_ana,
                    hum: bitaqat_hum,
                });
                let _ = ajnab.insert(ana.id, (ana, hum));
            }
        }
    }
}

/// Merges review state for one identical text.
///
/// The kept record moves whole, so a surviving approval is one that was
/// granted against exactly this text on the incoming side. Approved against
/// rejected is a human disagreement and is raised, never ranked.
fn idmij_hala_mutatabiqa(
    ana: MudkhalNass,
    hum: MudkhalNass,
    nizaat: &mut Vec<Nizaa>,
    ajnab: &mut BTreeMap<NassId, (MudkhalNass, MudkhalNass)>,
    madmuja: &mut Vec<MudkhalNass>,
) {
    let hala_ana = ana.muraja.hala();
    let hala_hum = hum.muraja.hala();
    if hala_ana == hala_hum {
        madmuja.push(ana);
        return;
    }

    let khilaf_bashari = matches!(
        (hala_ana, hala_hum),
        (HalatMuraja::Muakkada, HalatMuraja::Marfuda)
            | (HalatMuraja::Marfuda, HalatMuraja::Muakkada)
    );
    if khilaf_bashari {
        let (Some(bitaqat_ana), Some(bitaqat_hum)) =
            (BitaqatJanib::min_mudkhal(&ana), BitaqatJanib::min_mudkhal(&hum))
        else {
            madmuja.push(ana);
            return;
        };
        nizaat.push(Nizaa {
            nass: ana.id,
            masdar: ana.masdar.clone(),
            naw: NawNizaa::Hala,
            ana: bitaqat_ana,
            hum: bitaqat_hum,
        });
        let _ = ajnab.insert(ana.id, (ana, hum));
        return;
    }

    if rutbat_hala(hala_hum) > rutbat_hala(hala_ana) {
        madmuja.push(hum);
    } else {
        madmuja.push(ana);
    }
}

/// How far along the workflow a state is, for identical-text records only.
///
/// A human verdict outranks anything a machine or a draft holds; between the
/// two verdicts there is no rank, which is why that pair is a conflict.
const fn rutbat_hala(hala: HalatMuraja) -> u8 {
    match hala {
        HalatMuraja::LamTutarjam => 0,
        HalatMuraja::TarjamaAaliya => 1,
        HalatMuraja::Musawwada => 2,
        HalatMuraja::LilMuraja => 3,
        HalatMuraja::Marfuda | HalatMuraja::Muakkada => 4,
    }
}
