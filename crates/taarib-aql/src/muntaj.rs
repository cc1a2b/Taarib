//! المنتَج — which of Taarib's three products a game gets, and what that promises.

use serde::Serialize;
use taarib_mustalahat::muharrik::{Hadd, JahiziyatTashghil, Tabaqa};

/// Which of Taarib's products a game gets.
///
/// A type rather than the tier *number* the interface used to render. A number
/// does not tell anybody that their text will be drawn on top of the game rather
/// than inside it; and the discriminant existing only in Rust is what made a
/// hand-written paragraph in TypeScript the thing that actually reached the
/// screen, free to drift from the sentence the backend was writing at the same
/// moment.
///
/// Five values, not three. [`Self::LaShay`] and [`Self::Majhul`] are the two
/// answers that are not products, and they are not the same answer: a game
/// something blocks outright has a final reason, and a game nobody has examined
/// has no reason at all yet. Collapsing them is how a library row ends up saying
/// "unknown engine, tier 3" about a game that was never looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Muntaj {
    /// The game's own text is replaced inside the engine and looks native.
    Istibdal,
    /// Taarib draws the text itself over the engine's own text objects.
    RasmMubashir,
    /// The game is not modified at all; Arabic is shown over it as a reading
    /// aid.
    TabaqaFawqiya,
    /// Nothing. A blocker stands, and it is final until whatever it names
    /// changes.
    LaShay,
    /// Not yet known. This game has not been examined.
    Majhul,
}

impl Muntaj {
    /// The product a tier entitles a game to.
    ///
    /// Total in this direction so that a tier can never arrive here without a
    /// product, and so that the mapping is written once rather than at each
    /// surface that has to render one.
    #[must_use]
    pub const fn min_tabaqa(tabaqa: Tabaqa) -> Self {
        match tabaqa {
            Tabaqa::Kamil => Self::Istibdal,
            Tabaqa::RasmMubashir => Self::RasmMubashir,
            Tabaqa::TarjamaFawqiya => Self::TabaqaFawqiya,
        }
    }

    /// The tier behind this product, when it is a product at all.
    #[must_use]
    pub const fn tabaqa(self) -> Option<Tabaqa> {
        match self {
            Self::Istibdal => Some(Tabaqa::Kamil),
            Self::RasmMubashir => Some(Tabaqa::RasmMubashir),
            Self::TabaqaFawqiya => Some(Tabaqa::TarjamaFawqiya),
            Self::LaShay | Self::Majhul => None,
        }
    }

    /// The tier number the interface labels this with, when there is one.
    ///
    /// [`None`] rather than a fourth number for the two answers that are not
    /// tiers, because a number printed beside "nothing" is a number that
    /// contradicts the word next to it.
    #[must_use]
    pub const fn raqm(self) -> Option<u8> {
        match self.tabaqa() {
            Some(tabaqa) => Some(tabaqa.raqm()),
            None => None,
        }
    }

    /// The product's name, as the interface writes it in Arabic.
    ///
    /// The three tier names are the tier's own — a second copy of "طبقة ترجمة"
    /// anywhere is a copy that will one day say something else.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self.tabaqa() {
            Some(tabaqa) => tabaqa.ism_arabi(),
            None => match self {
                Self::Majhul => "لم تُفحص بعد",
                _ => "لا تعريب",
            },
        }
    }

    /// The same in English.
    #[must_use]
    pub const fn ism_injilizi(self) -> &'static str {
        match self.tabaqa() {
            Some(tabaqa) => tabaqa.ism_injilizi(),
            None => match self {
                Self::Majhul => "Not examined yet",
                _ => "No Arabization",
            },
        }
    }

    /// Whether anything at all is written into the game's own files.
    ///
    /// The one fact a user most often gets wrong about the overlay, and the
    /// reason it is asked of the product rather than of the tier number: a
    /// surface that has this answer can stop promising a restore for a product
    /// that never touched anything to restore.
    #[must_use]
    pub const fn yaktub(self) -> bool {
        matches!(self, Self::Istibdal | Self::RasmMubashir)
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Istibdal => "istibdal",
            Self::RasmMubashir => "rasm_mubashir",
            Self::TabaqaFawqiya => "tabaqa_fawqiya",
            Self::LaShay => "la_shay",
            Self::Majhul => "majhul",
        }
    }
}

/// What this game is promised, in both languages, in the backend's own words.
///
/// Every sentence here comes out of a producer. The names are the tier's own,
/// the reason is the capability report's own matched pair, and the gap sentence
/// is the readiness verdict's own [`Hadd`]. Nothing in this crate writes a
/// sentence about a product, because the sentence a surface renders and the
/// sentence the backend writes have to be the same sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Waad {
    /// Which product.
    pub muntaj: Muntaj,
    /// The product's name in Arabic.
    pub ism_arabi: &'static str,
    /// The product's name in English.
    pub ism_injilizi: &'static str,
    /// Why this product and not a better one, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// Whether this build of Taarib can actually deliver it.
    ///
    /// A tier is a permission and this is whether the half that does the work
    /// exists yet. They are separate on purpose: an inherent limit never
    /// improves, an unfinished adapter arrives in an update, and a reader can
    /// act on the difference.
    pub jahiziya: JahiziyatTashghil,
    /// What is unfinished, named, when something is.
    pub naqs: Option<Hadd>,
}

impl Waad {
    /// Whether anything this product promises reaches the screen in this build.
    #[must_use]
    pub const fn tasil(&self) -> bool {
        self.jahiziya.tasil()
    }

    /// Whether the interface has something extra to say about this build's
    /// reach.
    #[must_use]
    pub const fn tastahiq_tanbeeh(&self) -> bool {
        self.jahiziya.tastahiq_tanbeeh()
    }
}
