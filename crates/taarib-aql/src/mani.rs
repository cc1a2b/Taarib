//! المانع — what stops Taarib touching a game, and the one order they are read in.

use serde::Serialize;

/// How far a blocker reaches.
///
/// Two scopes, because the product has two doors and they are not the same door.
/// A game whose engine adapter is unfinished may still have a patch installed —
/// the capability report says so in as many words, "you can install the patch
/// now" — while the one-button run refuses it, because a run that ends in an
/// unchanged game has spent a translation budget for nothing. A single
/// `blocked` flag would have to be wrong about one of the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NitaqMani {
    /// Nothing at all happens to this game: no patch, no overlay, no run.
    Kul,
    /// Only the automatic end-to-end run is refused. A published patch may
    /// still be installed by hand.
    Tashghil,
}

/// Why Taarib will not act on a game.
///
/// **The order is here, once.** Every surface that has to pick one reason out of
/// several reads [`Self::rutba`], and that is the whole point of this type: the
/// verdict screen and the run button used to each apply their own order and
/// named different refusals for the same game — one said anti-cheat, the other
/// said official Arabic — and there was no structure preventing the next
/// divergence.
///
/// The order extends the one `taarib_aman::fahs::fahs` already keeps and does
/// not contradict it. Anti-cheat first, because an account is at stake and no
/// release and no setting lifts it; then the same question when it could not be
/// asked at all, because a check that did not run is not a check that passed;
/// then the publisher's own Arabic, because it is permanent too and it is the
/// one a reader can act on today; then the three facts about what this entry
/// *is*; then the fact that nobody has looked at it yet; and last, and in its
/// own scope, the one that a Taarib release changes.
///
/// The consent-shaped refusals are deliberately not here. An unacknowledged
/// product statement, an unacknowledged multiplayer warning and an approximate
/// build match are all answers the user can give, and they are
/// [`crate::khatar::Khatar`]. A blocker is what no answer opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NawMani {
    /// Anti-cheat, from evidence on disk or in the store's own catalogue.
    Himaya,
    /// The anti-cheat check could not be completed, so its silence proves
    /// nothing. VAC is declared only in Steam's catalogue and leaves nothing in
    /// a game folder, so an unread catalogue produces exactly the report a clean
    /// game produces.
    FahsHimayaLamYajri,
    /// The publisher already ships Arabic, and the user has not asked to be
    /// offered those games anyway.
    LughaRasmiya,
    /// The launcher marks this entry as something other than a game.
    LaysatLuba,
    /// The entry is a directory of ROM or disc images for an emulator.
    MuhakatRum,
    /// The game is not on this disk, or not all of it is.
    GhayrHadira,
    /// Nobody has examined this game, so nothing can be decided about it yet.
    LamYufhas,
    /// The part of Taarib that delivers this game's tier is unfinished in this
    /// build, so the automatic run would end in an unchanged game.
    JahiziyaGhaiba,
    /// No translation provider is configured, or every configured one is off.
    ///
    /// Last, and the only entry that is a fact about the machine rather than
    /// about the game. It ranks below an unfinished adapter deliberately: it is
    /// the one blocker a reader clears in half a minute, and putting it above a
    /// fact about the game would bury the thing they cannot change under the
    /// thing they can.
    LaMuzawwid,
}

impl NawMani {
    /// Every blocker, in the one order.
    ///
    /// Written out rather than iterated because the enum has no iterator, and in
    /// full so that adding a blocker breaks the ordering test rather than
    /// quietly slipping in at whatever rank its author happened to pick.
    pub const KUL: [Self; 9] = [
        Self::Himaya,
        Self::FahsHimayaLamYajri,
        Self::LughaRasmiya,
        Self::LaysatLuba,
        Self::MuhakatRum,
        Self::GhayrHadira,
        Self::LamYufhas,
        Self::JahiziyaGhaiba,
        Self::LaMuzawwid,
    ];

    /// Where this blocker sits in the one order. Lower is more serious.
    #[must_use]
    pub const fn rutba(self) -> u8 {
        match self {
            Self::Himaya => 0,
            Self::FahsHimayaLamYajri => 1,
            Self::LughaRasmiya => 2,
            Self::LaysatLuba => 3,
            Self::MuhakatRum => 4,
            Self::GhayrHadira => 5,
            Self::LamYufhas => 6,
            Self::JahiziyaGhaiba => 7,
            Self::LaMuzawwid => 8,
        }
    }

    /// How far this blocker reaches.
    #[must_use]
    pub const fn nitaq(self) -> NitaqMani {
        match self {
            Self::JahiziyaGhaiba | Self::LaMuzawwid => NitaqMani::Tashghil,
            Self::Himaya
            | Self::FahsHimayaLamYajri
            | Self::LughaRasmiya
            | Self::LaysatLuba
            | Self::MuhakatRum
            | Self::GhayrHadira
            | Self::LamYufhas => NitaqMani::Kul,
        }
    }

    /// Whether this is a fact about the game that nothing will change.
    ///
    /// The single most useful thing a reader takes off a refusal is whether to
    /// wait or to give up. Anti-cheat and a publisher's own Arabic are final;
    /// an unexamined game, a game still downloading and an unfinished adapter
    /// are not, and telling somebody to give up on one of those is the wrong
    /// sentence to leave them with.
    #[must_use]
    pub const fn nihai(self) -> bool {
        match self {
            Self::Himaya | Self::LughaRasmiya | Self::LaysatLuba | Self::MuhakatRum => true,
            Self::FahsHimayaLamYajri
            | Self::GhayrHadira
            | Self::LamYufhas
            | Self::JahiziyaGhaiba
            | Self::LaMuzawwid => false,
        }
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Himaya => "himaya",
            Self::FahsHimayaLamYajri => "fahs_himaya_lam_yajri",
            Self::LughaRasmiya => "lugha_rasmiya",
            Self::LaysatLuba => "laysat_luba",
            Self::MuhakatRum => "muhakat_rum",
            Self::GhayrHadira => "ghayr_hadira",
            Self::LamYufhas => "lam_yufhas",
            Self::JahiziyaGhaiba => "jahiziya_ghaiba",
            Self::LaMuzawwid => "la_muzawwid",
        }
    }
}

/// One standing blocker, carrying the sentences its producer wrote.
///
/// The two sentences are not composed here. Each comes out of the crate that
/// owns the question — the safety layer's own refusal text for anti-cheat, the
/// probe's own gap sentence for an unfinished adapter, the launcher state's own
/// sentence for a game that is not on disk — so that a reworded refusal reaches
/// every surface at once and no surface holds a second copy of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Mani {
    /// Which blocker this is, and therefore where it sits in the order.
    pub naw: NawMani,
    /// The sentence shown to the user, in Arabic.
    pub arabi: String,
    /// The same in English.
    pub injilizi: String,
}

impl Mani {
    /// Binds a blocker to the sentences behind it.
    #[must_use]
    pub fn jadeed(naw: NawMani, arabi: impl Into<String>, injilizi: impl Into<String>) -> Self {
        Self {
            naw,
            arabi: arabi.into(),
            injilizi: injilizi.into(),
        }
    }

    /// Where this blocker sits in the one order.
    #[must_use]
    pub const fn rutba(&self) -> u8 {
        self.naw.rutba()
    }

    /// How far it reaches.
    #[must_use]
    pub const fn nitaq(&self) -> NitaqMani {
        self.naw.nitaq()
    }

    /// Whether it is a fact nothing will change.
    #[must_use]
    pub const fn nihai(&self) -> bool {
        self.naw.nihai()
    }
}
