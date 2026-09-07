//! Ranking when a game has several patches: a default order, never a verdict.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use taarib_mustalahat::bina::MutabaqaBina;
use taarib_mustalahat::muharrik::Tabaqa;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{MulakhkhasRuqaa, RuqaaId, RuqaaRevision, TareeqaTarjama};
use taarib_mustalahat::taghtiya::Taghtiya;

use crate::khata::{KhataMustawda, NatijatMustawda};

/// The average at or above which a rating counts in a patch's favour.
pub const HADD_TAQYEEM_JAYYID: f32 = 3.5;

/// How many ratings an average must rest on before it ranks at all.
pub const ADNA_TAQYEEMAT: u32 = 3;

/// The rating band that neither helps nor hurts a listing's position.
const RUTBA_MUHAYYADA: u8 = 1;

/// The thresholds ranking applies to a rating.
#[derive(Debug, Clone, Copy)]
pub struct KhiyaratTarteeb {
    /// The average at or above which a rating counts in a patch's favour.
    pub hadd_jayyid: f32,
    /// How many ratings an average must rest on before it ranks at all.
    pub adna_taqyeemat: u32,
}

impl Default for KhiyaratTarteeb {
    fn default() -> Self {
        Self {
            hadd_jayyid: HADD_TAQYEEM_JAYYID,
            adna_taqyeemat: ADNA_TAQYEEMAT,
        }
    }
}

/// Where a listing's rating places it, with an unrated patch kept distinct from
/// a badly rated one.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiatTaqyeem {
    /// Rated at or above the threshold, on enough ratings.
    Jayyid {
        /// The average.
        mutawassit: f32,
        /// How many ratings it rests on.
        adad: u32,
    },
    /// Rated, but on fewer ratings than the threshold requires.
    AdillaQaleela {
        /// The average.
        mutawassit: f32,
        /// How many ratings it rests on.
        adad: u32,
    },
    /// Nobody has rated it.
    LamYuqayyam,
    /// Rated below the threshold, on enough ratings.
    Daeef {
        /// The average.
        mutawassit: f32,
        /// How many ratings it rests on.
        adad: u32,
    },
}

impl FiatTaqyeem {
    /// Classifies an average and the number of ratings behind it.
    #[must_use]
    pub fn jadeeda(mutawassit: Option<f32>, adad: u32, khiyarat: KhiyaratTarteeb) -> Self {
        let Some(mutawassit) = mutawassit else {
            return Self::LamYuqayyam;
        };
        // a non-finite average is not a rating, and is never rendered as one
        if !mutawassit.is_finite() || adad == 0 {
            return Self::LamYuqayyam;
        }
        if adad < khiyarat.adna_taqyeemat {
            return Self::AdillaQaleela { mutawassit, adad };
        }
        match mutawassit.total_cmp(&khiyarat.hadd_jayyid) {
            Ordering::Greater | Ordering::Equal => Self::Jayyid { mutawassit, adad },
            Ordering::Less => Self::Daeef { mutawassit, adad },
        }
    }

    /// Classifies the rating a listing carries.
    #[must_use]
    pub fn min_mulakhkhas(ruqaa: &MulakhkhasRuqaa, khiyarat: KhiyaratTarteeb) -> Self {
        Self::jadeeda(ruqaa.taqyeem, ruqaa.adad_taqyeemat, khiyarat)
    }

    /// The ranking band: 0 helps, 1 is neutral, 2 hurts.
    #[must_use]
    pub const fn rutba(self) -> u8 {
        match self {
            Self::Jayyid { .. } => 0,
            Self::AdillaQaleela { .. } | Self::LamYuqayyam => RUTBA_MUHAYYADA,
            Self::Daeef { .. } => 2,
        }
    }

    /// The average and its count, when there is one.
    #[must_use]
    pub const fn qeema(self) -> Option<(f32, u32)> {
        match self {
            Self::Jayyid { mutawassit, adad }
            | Self::AdillaQaleela { mutawassit, adad }
            | Self::Daeef { mutawassit, adad } => Some((mutawassit, adad)),
            Self::LamYuqayyam => None,
        }
    }

    /// The one-line rating label, in Arabic.
    #[must_use]
    pub fn wasf_arabi(self) -> String {
        match self {
            Self::Jayyid { mutawassit, adad } | Self::Daeef { mutawassit, adad } => {
                format!("{mutawassit:.1} من ٥ ({adad} تقييمًا)")
            },
            Self::AdillaQaleela { mutawassit, adad } => {
                format!("{mutawassit:.1} من ٥ ({adad} تقييمًا فقط — عدد قليل)")
            },
            Self::LamYuqayyam => "لم تُقيَّم بعد".to_owned(),
        }
    }

    /// The same label in English.
    #[must_use]
    pub fn wasf_injilizi(self) -> String {
        match self {
            Self::Jayyid { mutawassit, adad } | Self::Daeef { mutawassit, adad } => {
                format!("{mutawassit:.1} of 5 from {adad} ratings")
            },
            Self::AdillaQaleela { mutawassit, adad } => {
                format!("{mutawassit:.1} of 5 from only {adad} ratings")
            },
            Self::LamYuqayyam => "Not rated yet".to_owned(),
        }
    }
}

/// One listing paired with the match tier computed for the installed build.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MudkhalTarteeb {
    /// The registry listing.
    pub ruqaa: MulakhkhasRuqaa,
    /// The tier the match step produced for this build.
    pub mutabaqa: MutabaqaBina,
    /// Why it does not apply, when it does not.
    pub sabab: Option<String>,
}

impl MudkhalTarteeb {
    /// Pairs a listing with its match tier and the reason behind it.
    #[must_use]
    pub const fn jadeed(
        ruqaa: MulakhkhasRuqaa,
        mutabaqa: MutabaqaBina,
        sabab: Option<String>,
    ) -> Self {
        Self {
            ruqaa,
            mutabaqa,
            sabab,
        }
    }

    /// Whether the client will install this at all.
    #[must_use]
    pub const fn qabila_lil_tathbeet(&self) -> bool {
        self.mutabaqa.qabila_lil_tathbeet()
    }

    /// The lineage.
    #[must_use]
    pub const fn id(&self) -> RuqaaId {
        self.ruqaa.id
    }
}

/// Orders `mudkhalat` with the default thresholds, incompatible entries last.
#[must_use]
pub fn rattib(mudkhalat: Vec<MudkhalTarteeb>) -> Vec<MudkhalTarteeb> {
    rattib_bi(mudkhalat, KhiyaratTarteeb::default())
}

/// Orders `mudkhalat`, keeping every entry including the incompatible ones.
#[must_use]
pub fn rattib_bi(
    mut mudkhalat: Vec<MudkhalTarteeb>,
    khiyarat: KhiyaratTarteeb,
) -> Vec<MudkhalTarteeb> {
    mudkhalat.sort_by(|awwal, thani| qarin(awwal, thani, khiyarat));
    mudkhalat
}

/// Orders `mudkhalat` and refuses a set with nothing installable in it.
///
/// # Errors
///
/// [`KhataMustawda::LaMutabaqa`] when no entry clears
/// [`MutabaqaBina::qabila_lil_tathbeet`].
pub fn rattib_lil_tathbeet(
    mudkhalat: Vec<MudkhalTarteeb>,
    khiyarat: KhiyaratTarteeb,
) -> NatijatMustawda<Vec<MudkhalTarteeb>> {
    if !mudkhalat.iter().any(MudkhalTarteeb::qabila_lil_tathbeet) {
        return Err(KhataMustawda::LaMutabaqa);
    }
    Ok(rattib_bi(mudkhalat, khiyarat))
}

/// The full ranking comparison between two listings.
#[must_use]
pub fn qarin(
    awwal: &MudkhalTarteeb,
    thani: &MudkhalTarteeb,
    khiyarat: KhiyaratTarteeb,
) -> Ordering {
    awwal
        .mutabaqa
        .cmp(&thani.mutabaqa)
        .then_with(|| qarin_taghtiya(&awwal.ruqaa.taghtiya, &thani.ruqaa.taghtiya))
        .then_with(|| {
            qarin_taqyeem(
                FiatTaqyeem::min_mulakhkhas(&awwal.ruqaa, khiyarat),
                FiatTaqyeem::min_mulakhkhas(&thani.ruqaa, khiyarat),
            )
        })
        .then_with(|| qarin_waqt(&awwal.ruqaa.waqt_nashr, &thani.ruqaa.waqt_nashr))
        .then_with(|| awwal.ruqaa.id.cmp(&thani.ruqaa.id))
        .then_with(|| thani.ruqaa.murajaa.cmp(&awwal.ruqaa.murajaa))
}

fn qarin_taghtiya(awwal: &Taghtiya, thani: &Taghtiya) -> Ordering {
    thani
        .nisba_mawzuna()
        .total_cmp(&awwal.nisba_mawzuna())
        .then_with(|| thani.nisba_awwal().total_cmp(&awwal.nisba_awwal()))
        .then_with(|| thani.nisba().total_cmp(&awwal.nisba()))
        .then_with(|| thani.nisba_muakkada().total_cmp(&awwal.nisba_muakkada()))
}

fn qarin_taqyeem(awwal: FiatTaqyeem, thani: FiatTaqyeem) -> Ordering {
    match awwal.rutba().cmp(&thani.rutba()) {
        Ordering::Equal => {},
        ghayr => return ghayr,
    }
    // the neutral band mixes unrated with thinly rated: ordering them would invent a score
    if awwal.rutba() == RUTBA_MUHAYYADA {
        return Ordering::Equal;
    }
    match (awwal.qeema(), thani.qeema()) {
        (Some((qeema_awwal, adad_awwal)), Some((qeema_thani, adad_thani))) => qeema_thani
            .total_cmp(&qeema_awwal)
            .then_with(|| adad_thani.cmp(&adad_awwal)),
        _ => Ordering::Equal,
    }
}

fn qarin_waqt(awwal: &str, thani: &str) -> Ordering {
    match (
        awwal.parse::<Timestamp>().ok(),
        thani.parse::<Timestamp>().ok(),
    ) {
        (Some(lahza_awwal), Some(lahza_thani)) => lahza_thani.cmp(&lahza_awwal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => awwal.cmp(thani),
    }
}

/// One column of the side-by-side comparison a listing screen renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HaqlMuqarana {
    /// Who made it.
    Musahim,
    /// How much of the game it covers.
    Taghtiya,
    /// How it was translated.
    Tareeqa,
    /// The package size.
    Hajm,
    /// When the revision was published.
    WaqtNashr,
    /// The tier it installs at.
    Tabaqa,
    /// Its licence.
    Rukhsa,
    /// How well it matches the installed build.
    Mutabaqa,
    /// Its rating band.
    Taqyeem,
}

/// The differences of one listing, as a comparison row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarqMudkhal {
    /// The lineage.
    pub id: RuqaaId,
    /// The published revision.
    pub murajaa: RuqaaRevision,
    /// The title the contributor gave it.
    pub unwan: String,
    /// Who made it.
    pub musahim: MusahimId,
    /// Their display name.
    pub ism_musahim: String,
    /// How much of the game it covers.
    pub taghtiya: Taghtiya,
    /// How it was translated.
    pub tareeqa: TareeqaTarjama,
    /// The package size in bytes.
    pub hajm: u64,
    /// When this revision was published, RFC 3339.
    pub waqt_nashr: String,
    /// The tier it installs at.
    pub tabaqa: Tabaqa,
    /// Its licence identifier.
    pub rukhsa: String,
    /// How well it matches the installed build.
    pub mutabaqa: MutabaqaBina,
    /// Why it does not apply, when it does not.
    pub sabab: Option<String>,
    /// Its rating band.
    pub fiat_taqyeem: FiatTaqyeem,
}

impl FarqMudkhal {
    /// Builds one comparison row from a ranked entry.
    #[must_use]
    pub fn jadeed(mudkhal: &MudkhalTarteeb, khiyarat: KhiyaratTarteeb) -> Self {
        let ruqaa = &mudkhal.ruqaa;
        Self {
            id: ruqaa.id,
            murajaa: ruqaa.murajaa,
            unwan: ruqaa.unwan.clone(),
            musahim: ruqaa.musahim.clone(),
            ism_musahim: ruqaa.ism_musahim.clone(),
            taghtiya: ruqaa.taghtiya,
            tareeqa: ruqaa.tareeqa,
            hajm: ruqaa.hajm,
            waqt_nashr: ruqaa.waqt_nashr.clone(),
            tabaqa: ruqaa.tabaqa,
            rukhsa: ruqaa.rukhsa.muarrif().to_owned(),
            mutabaqa: mudkhal.mutabaqa,
            sabab: mudkhal.sabab.clone(),
            fiat_taqyeem: FiatTaqyeem::min_mulakhkhas(ruqaa, khiyarat),
        }
    }

    /// Whether the client will install this at all.
    #[must_use]
    pub const fn qabila_lil_tathbeet(&self) -> bool {
        self.mutabaqa.qabila_lil_tathbeet()
    }
}

/// Every alternative for one game, ordered and side by side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuqaranatRuqaa {
    mudkhalat: Vec<FarqMudkhal>,
}

impl MuqaranatRuqaa {
    /// Ranks `mudkhalat` and renders every one of them as a comparison row.
    #[must_use]
    pub fn jadeeda(mudkhalat: Vec<MudkhalTarteeb>, khiyarat: KhiyaratTarteeb) -> Self {
        let murattaba = rattib_bi(mudkhalat, khiyarat);
        Self {
            mudkhalat: murattaba
                .iter()
                .map(|mudkhal| FarqMudkhal::jadeed(mudkhal, khiyarat))
                .collect(),
        }
    }

    /// Every row, in ranked order, incompatible ones included.
    #[must_use]
    pub fn mudkhalat(&self) -> &[FarqMudkhal] {
        &self.mudkhalat
    }

    /// How many alternatives there are.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.mudkhalat.len()
    }

    /// How many of them the client will install.
    #[must_use]
    pub fn adad_mutawafiq(&self) -> usize {
        self.mudkhalat
            .iter()
            .filter(|farq| farq.qabila_lil_tathbeet())
            .count()
    }

    /// The rows the client will install, in ranked order.
    pub fn mutawafiqa(&self) -> impl Iterator<Item = &FarqMudkhal> {
        self.mudkhalat
            .iter()
            .filter(|farq| farq.qabila_lil_tathbeet())
    }

    /// The rows the client will not install, each keeping its reason.
    pub fn ghayr_mutawafiqa(&self) -> impl Iterator<Item = &FarqMudkhal> {
        self.mudkhalat
            .iter()
            .filter(|farq| !farq.qabila_lil_tathbeet())
    }

    /// Which columns actually differ across the set, so a screen can highlight
    /// those and leave the rest quiet.
    #[must_use]
    pub fn tabayun(&self) -> BTreeSet<HaqlMuqarana> {
        let mut huqul = BTreeSet::new();
        let Some(asas) = self.mudkhalat.first() else {
            return huqul;
        };
        for farq in self.mudkhalat.iter().skip(1) {
            let mut daa = |haql: HaqlMuqarana, mukhtalif: bool| {
                if mukhtalif {
                    let _ = huqul.insert(haql);
                }
            };
            daa(HaqlMuqarana::Musahim, farq.musahim != asas.musahim);
            daa(HaqlMuqarana::Taghtiya, farq.taghtiya != asas.taghtiya);
            daa(HaqlMuqarana::Tareeqa, farq.tareeqa != asas.tareeqa);
            daa(HaqlMuqarana::Hajm, farq.hajm != asas.hajm);
            daa(HaqlMuqarana::WaqtNashr, farq.waqt_nashr != asas.waqt_nashr);
            daa(HaqlMuqarana::Tabaqa, farq.tabaqa != asas.tabaqa);
            daa(HaqlMuqarana::Rukhsa, farq.rukhsa != asas.rukhsa);
            daa(HaqlMuqarana::Mutabaqa, farq.mutabaqa != asas.mutabaqa);
            daa(
                HaqlMuqarana::Taqyeem,
                farq.fiat_taqyeem.rutba() != asas.fiat_taqyeem.rutba(),
            );
        }
        huqul
    }

    /// Every contributor represented in the set.
    #[must_use]
    pub fn musahimun(&self) -> BTreeSet<&MusahimId> {
        self.mudkhalat.iter().map(|farq| &farq.musahim).collect()
    }
}
