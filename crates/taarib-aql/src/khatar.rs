//! الخطر — a risk that has to be accepted before anything is written, not refused.

use serde::Serialize;

/// A risk the user can take, told apart from a blocker they cannot.
///
/// The difference is whether an answer opens it. A blocker
/// ([`crate::mani::Mani`]) is what no answer opens; these five are exactly the
/// questions the product is entitled to ask, and each one an untouched box
/// leaves unanswered is a real no.
///
/// The ordering is the same one `taarib_aman::fahs::fahs` keeps for its own
/// consent-shaped refusals: the product statement first, because it is asked
/// once for the whole product and not per game, and the multiplayer warning
/// last, because it is the one refusal in the set with an answer the person can
/// give at the moment they meet it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NawKhatar {
    /// Taarib's first-run statement has not been acknowledged.
    BayanAwwal,
    /// The publisher ships Arabic and the user turned the exclusion off, so the
    /// game is being offered on their own judgement.
    LughaRasmiyaMutajawaza,
    /// Something that is not Taarib already holds a loader slot in the game's
    /// own folder.
    WakeelGhareeb,
    /// The best published patch matches the installed build only approximately.
    MutabaqaTaqribiya,
    /// The game has online play and no anti-cheat was found in it.
    LaabJamai,
}

impl NawKhatar {
    /// Every risk, in the order they are asked.
    ///
    /// Written out rather than iterated, for the reason
    /// [`crate::mani::NawMani::KUL`] is: adding one must break the ordering test
    /// rather than slip in unranked.
    pub const KUL: [Self; 5] = [
        Self::BayanAwwal,
        Self::LughaRasmiyaMutajawaza,
        Self::WakeelGhareeb,
        Self::MutabaqaTaqribiya,
        Self::LaabJamai,
    ];

    /// Where this risk sits in the order it is asked in. Lower comes first.
    #[must_use]
    pub const fn rutba(self) -> u8 {
        match self {
            Self::BayanAwwal => 0,
            Self::LughaRasmiyaMutajawaza => 1,
            Self::WakeelGhareeb => 2,
            Self::MutabaqaTaqribiya => 3,
            Self::LaabJamai => 4,
        }
    }

    /// Whether the answer is given once for the whole product rather than per
    /// game.
    ///
    /// Only the first-run statement is. The other four are about this game and
    /// this install, and an answer carried over from another game would be an
    /// answer nobody gave.
    #[must_use]
    pub const fn amm(self) -> bool {
        matches!(self, Self::BayanAwwal)
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::BayanAwwal => "bayan_awwal",
            Self::LughaRasmiyaMutajawaza => "lugha_rasmiya_mutajawaza",
            Self::WakeelGhareeb => "wakeel_ghareeb",
            Self::MutabaqaTaqribiya => "mutabaqa_taqribiya",
            Self::LaabJamai => "laab_jamai",
        }
    }
}

/// One standing risk, its two sentences, and whether it has been accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Khatar {
    /// Which risk this is.
    pub naw: NawKhatar,
    /// What the user is being asked to accept, in Arabic.
    pub arabi: String,
    /// The same in English.
    pub injilizi: String,
    /// Whether the user has already accepted it.
    pub muqarr: bool,
}

impl Khatar {
    /// Binds a risk to its sentences and to the answer already on record.
    #[must_use]
    pub fn jadeed(
        naw: NawKhatar,
        arabi: impl Into<String>,
        injilizi: impl Into<String>,
        muqarr: bool,
    ) -> Self {
        Self {
            naw,
            arabi: arabi.into(),
            injilizi: injilizi.into(),
            muqarr,
        }
    }

    /// Where this risk sits in the order it is asked in.
    #[must_use]
    pub const fn rutba(&self) -> u8 {
        self.naw.rutba()
    }

    /// Whether this risk is still standing between the user and a write.
    #[must_use]
    pub const fn muallaq(&self) -> bool {
        !self.muqarr
    }
}
