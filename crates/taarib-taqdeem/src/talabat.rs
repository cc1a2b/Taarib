//! The translation requests board: demand per game, deduplicated per person.

use std::collections::{BTreeMap, BTreeSet};

use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::RuqaaId;

/// The longest note a request may carry.
pub const AQSA_TUL_MULAHAZA: usize = 500;

/// One person asking for one game.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TalabTarjama {
    /// The game.
    pub luba: LubaId,
    /// Its name, denormalized so the board renders without a second lookup.
    pub ism_luba: String,
    /// Who asked.
    pub talib: MusahimId,
    /// When, RFC 3339, supplied by the caller.
    pub waqt: String,
    /// What they said, truncated to [`AQSA_TUL_MULAHAZA`] characters.
    pub mulahaza: Option<String>,
}

impl TalabTarjama {
    /// Records one request, trimming an over-long note rather than refusing it.
    #[must_use]
    pub fn jadeed(
        luba: LubaId,
        ism_luba: String,
        talib: MusahimId,
        waqt: String,
        mulahaza: Option<String>,
    ) -> Self {
        let mulahaza = mulahaza.and_then(|nass| {
            let maqsus: String = nass.trim().chars().take(AQSA_TUL_MULAHAZA).collect();
            (!maqsus.is_empty()).then_some(maqsus)
        });
        Self { luba, ism_luba, talib, waqt, mulahaza }
    }
}

/// Why a request is no longer open.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "sabab", rename_all = "snake_case")]
pub enum IghlaqTalab {
    /// A patch was published for the game.
    Nushirat {
        /// The patch that closed it.
        ruqaa: RuqaaId,
    },
    /// The owner closed it without a patch, with a reason.
    Ughliq {
        /// Why.
        bayan: String,
    },
}

/// Every request for one game, and whether it is still open.
///
/// Demand is the size of the requester set, never a stored counter: a count a
/// caller increments is a count a caller can increment twice.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TalabatLuba {
    /// The game.
    pub luba: LubaId,
    /// Its name.
    pub ism_luba: String,
    /// The requests, keyed by requester so one person counts once.
    pub talabat: BTreeMap<MusahimId, TalabTarjama>,
    /// Set when the request closed. A closed request is kept, not deleted.
    pub ighlaq: Option<IghlaqTalab>,
}

impl TalabatLuba {
    /// An empty board entry for one game.
    #[must_use]
    pub const fn jadeed(luba: LubaId, ism_luba: String) -> Self {
        Self { luba, ism_luba, talabat: BTreeMap::new(), ighlaq: None }
    }

    /// Records a request, replacing that person's earlier one.
    ///
    /// Returns whether this added a new requester rather than updating one.
    pub fn adif(&mut self, talab: TalabTarjama) -> bool {
        self.talabat.insert(talab.talib.clone(), talab).is_none()
    }

    /// Withdraws one person's request.
    pub fn ishab(&mut self, talib: &MusahimId) -> bool {
        self.talabat.remove(talib).is_some()
    }

    /// How many distinct people asked.
    #[must_use]
    pub fn talab(&self) -> usize {
        self.talabat.len()
    }

    /// Whether the board still shows this as wanted.
    #[must_use]
    pub const fn maftuh(&self) -> bool {
        self.ighlaq.is_none()
    }

    /// Everyone to notify when it closes.
    #[must_use]
    pub fn lil_ishaar(&self) -> Vec<MusahimId> {
        self.talabat.keys().cloned().collect()
    }
}

/// The whole board.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LawhatTalabat {
    alaab: BTreeMap<LubaId, TalabatLuba>,
}

impl LawhatTalabat {
    /// An empty board.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::default()
    }

    /// Records a request, creating the game's entry if it is the first.
    ///
    /// Returns whether the requester was new for this game.
    pub fn utlub(&mut self, talab: TalabTarjama) -> bool {
        let luba = talab.luba;
        let ism = talab.ism_luba.clone();
        self.alaab
            .entry(luba)
            .or_insert_with(|| TalabatLuba::jadeed(luba, ism))
            .adif(talab)
    }

    /// One game's requests.
    #[must_use]
    pub fn luba(&self, luba: LubaId) -> Option<&TalabatLuba> {
        self.alaab.get(&luba)
    }

    /// How many distinct people asked for one game.
    #[must_use]
    pub fn talab(&self, luba: LubaId) -> usize {
        self.alaab.get(&luba).map_or(0, TalabatLuba::talab)
    }

    /// Open requests, most-wanted first, ties broken by game identity so two
    /// runs order identically.
    #[must_use]
    pub fn hasab_talab(&self) -> Vec<&TalabatLuba> {
        let mut murattaba: Vec<&TalabatLuba> =
            self.alaab.values().filter(|wahid| wahid.maftuh()).collect();
        murattaba.sort_by(|awwal, thani| {
            thani.talab().cmp(&awwal.talab()).then(awwal.luba.cmp(&thani.luba))
        });
        murattaba
    }

    /// Closes the request a published patch answers.
    ///
    /// Returns everyone who asked, so they can be notified. An already-closed
    /// or absent request notifies nobody.
    pub fn ughliq(&mut self, luba: LubaId, ruqaa: RuqaaId) -> Vec<MusahimId> {
        let Some(wahid) = self.alaab.get_mut(&luba) else { return Vec::new() };
        if wahid.ighlaq.is_some() {
            return Vec::new();
        }
        wahid.ighlaq = Some(IghlaqTalab::Nushirat { ruqaa });
        wahid.lil_ishaar()
    }

    /// Every game with an open request, for the contributor's browse view.
    #[must_use]
    pub fn maftuha(&self) -> BTreeSet<LubaId> {
        self.alaab
            .values()
            .filter(|wahid| wahid.maftuh())
            .map(|wahid| wahid.luba)
            .collect()
    }
}
