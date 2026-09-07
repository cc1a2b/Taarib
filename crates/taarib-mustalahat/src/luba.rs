//! اللعبة — a game, and the launchers that know it.
//!
//! One game can be known to several launchers at once: the same title bought on
//! Steam and later claimed on Epic is one game to the player, one entry in the
//! library, and one target for a patch. Taarib therefore keeps its own identity
//! ([`LubaId`]) derived from the launcher identifier and the normalized name,
//! and carries every launcher identity it has seen alongside it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use taarib_usus::manassa::BeeatTawafuq;
use uuid::Uuid;

use crate::bina::BinaId;
use crate::{NITAQ_TAARIB, wahhid_ism};

/// Taarib's own stable identity for a game.
///
/// A `UUIDv5` over the launcher identifier and the normalized name, so it is
/// identical on every machine, survives reinstalls, survives moving the game
/// between drives, and can be computed by the registry without coordination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct LubaId(Uuid);

impl LubaId {
    /// Derives the identity of a game from one of its launcher identities.
    #[must_use]
    pub fn min_masdar(masdar: &MasdarLuba, ism: &str) -> Self {
        let bidhra = format!("{}:{}", masdar.muarrif(), wahhid_ism(ism));
        Self(Uuid::new_v5(&NITAQ_TAARIB, bidhra.as_bytes()))
    }

    /// Wraps an identity that was already computed and stored.
    #[must_use]
    pub const fn min_uuid(qeema: Uuid) -> Self {
        Self(qeema)
    }

    /// The underlying value, for storage and for path segments.
    #[must_use]
    pub const fn uuid(self) -> Uuid {
        self.0
    }
}

impl std::fmt::Display for LubaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

/// Where a game came from, keeping that launcher's own identifier intact.
///
/// The launcher's identifier is preserved rather than normalized away because
/// it is what the registry shards on, what the launcher's own manifests key on,
/// and what a user will paste into a bug report.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "manassa", content = "muarrif", rename_all = "snake_case")]
// `MasdarLuba` rather than `Self` in the recursive variants below: `specta::Type`
// expands into an item outside this one, where `Self` does not name anything. The
// variants cannot be spelled one way with the feature and another way without it,
// so the expectation cannot be gated on the feature either.
#[expect(clippy::use_self, reason = "see above")]
pub enum MasdarLuba {
    /// Steam, by application identifier.
    Steam(u32),
    /// The Epic Games Store, by catalogue item identifier.
    Epic(String),
    /// GOG, by product identifier.
    Gog(u64),
    /// The EA app, by content identifier.
    Ea(String),
    /// Ubisoft Connect, by installation identifier.
    Ubisoft(u32),
    /// Battle.net, by product code.
    BattleNet(String),
    /// The Microsoft Store, by package family name.
    Xbox(String),
    /// itch.io, by game identifier.
    Itch(i64),
    /// Amazon Games, by the product ASIN its own catalogue keys on.
    Amazon(String),
    /// The Rockstar Games Launcher, by title identifier.
    Rockstar(u32),
    /// The Riot client, by product slug — `valorant`, `league_of_legends`.
    Riot(String),
    /// Heroic, which manages games that belong to another store; the wrapped
    /// source is the real identity, and Heroic is how it is installed.
    Heroic(Box<MasdarLuba>),
    /// Lutris, which does the same for a much wider set of sources, including
    /// ones with no store behind them at all.
    ///
    /// The wrapped source is `None` for a Lutris entry that is somebody's own
    /// script rather than a store's game — a GOG installer run by hand, a
    /// disc image, an emulated title. Those have no upstream identity to
    /// resolve to, and pretending otherwise would key a patch against a store
    /// identifier the game does not have. The string is then Lutris's own slug.
    Lutris(Box<MasdarLutris>),
    /// Bottles, which manages Wine prefixes rather than games; the identity is
    /// the bottle's name and the program's path inside it.
    Bottles(String),
    /// Legendary, the command-line Epic client. Its catalogue is Epic's, so the
    /// wrapped source is always an [`MasdarLuba::Epic`].
    Legendary(Box<MasdarLuba>),
    /// Playnite, a library manager that aggregates every other launcher.
    ///
    /// Like Heroic and Legendary it wraps a real source where it knows one, and
    /// unlike them it frequently does not: a Playnite library is full of manually
    /// added and emulated entries. `None` there, and the string is Playnite's
    /// own GUID.
    Playnite(Box<MasdarPlaynite>),
    /// A game the user pointed at directly, identified by a hash of its
    /// executable path so that the identity is stable across restarts.
    Yadawi(String),
}

/// What a Lutris entry actually is.
///
/// Modelled as a type rather than as `Option<MasdarLuba>` inside the variant so
/// that the slug is present in both cases. A Lutris game that *does* wrap a
/// store still has a Lutris slug, and losing it would mean losing the ability to
/// launch the game through Lutris — which is the only way it runs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MasdarLutris {
    /// Lutris's own slug for the entry, always present.
    pub silaa: String,
    /// The store behind it, when Lutris records one.
    pub asl: Option<MasdarLuba>,
}

/// What a Playnite entry actually is.
///
/// The same shape as [`MasdarLutris`] and for the same reason: Playnite's GUID
/// is how the entry is launched and is present whether or not a store identity
/// is known behind it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MasdarPlaynite {
    /// Playnite's own GUID for the entry, always present.
    pub muarrif: String,
    /// The store behind it, when Playnite's plugin recorded one.
    pub asl: Option<MasdarLuba>,
}

impl MasdarLuba {
    /// The launcher's name, as the interface writes it in Arabic.
    #[must_use]
    pub fn ism_arabi(&self) -> &'static str {
        match self {
            Self::Steam(_) => "ستيم",
            Self::Epic(_) => "إيبك",
            Self::Gog(_) => "غوغ",
            Self::Ea(_) => "إي إيه",
            Self::Ubisoft(_) => "يوبيسوفت",
            Self::BattleNet(_) => "باتل نت",
            Self::Xbox(_) => "إكس بوكس",
            Self::Itch(_) => "إتش",
            Self::Amazon(_) => "أمازون",
            Self::Rockstar(_) => "روكستار",
            Self::Riot(_) => "رايوت",
            Self::Heroic(asl) | Self::Legendary(asl) => asl.ism_arabi(),
            Self::Lutris(asl) => asl.asl.as_ref().map_or("لوتريس", |q| q.ism_arabi()),
            Self::Bottles(_) => "بوتلز",
            Self::Playnite(asl) => {
                asl.asl.as_ref().map_or("بلاي‌نايت", |q| q.ism_arabi())
            },
            Self::Yadawi(_) => "مضافة يدويًا",
        }
    }

    /// The launcher's name in English.
    #[must_use]
    pub fn ism_injilizi(&self) -> &'static str {
        match self {
            Self::Steam(_) => "Steam",
            Self::Epic(_) => "Epic Games",
            Self::Gog(_) => "GOG",
            Self::Ea(_) => "EA app",
            Self::Ubisoft(_) => "Ubisoft Connect",
            Self::BattleNet(_) => "Battle.net",
            Self::Xbox(_) => "Xbox",
            Self::Itch(_) => "itch.io",
            Self::Amazon(_) => "Amazon Games",
            Self::Rockstar(_) => "Rockstar Games",
            Self::Riot(_) => "Riot",
            Self::Heroic(asl) | Self::Legendary(asl) => asl.ism_injilizi(),
            Self::Lutris(asl) => asl.asl.as_ref().map_or("Lutris", |q| q.ism_injilizi()),
            Self::Bottles(_) => "Bottles",
            Self::Playnite(asl) => asl.asl.as_ref().map_or("Playnite", |q| q.ism_injilizi()),
            Self::Yadawi(_) => "Added manually",
        }
    }

    /// The registry shard family this source belongs to.
    ///
    /// Heroic resolves to the store it manages, so a patch published against a
    /// Steam identifier is found by a Heroic user who owns the same game.
    #[must_use]
    pub fn aila(&self) -> Manassa {
        match self {
            Self::Steam(_) => Manassa::Steam,
            Self::Epic(_) => Manassa::Epic,
            Self::Gog(_) => Manassa::Gog,
            Self::Ea(_) => Manassa::Ea,
            Self::Ubisoft(_) => Manassa::Ubisoft,
            Self::BattleNet(_) => Manassa::BattleNet,
            Self::Xbox(_) => Manassa::Xbox,
            Self::Itch(_) => Manassa::Itch,
            Self::Amazon(_) => Manassa::Amazon,
            Self::Rockstar(_) => Manassa::Rockstar,
            Self::Riot(_) => Manassa::Riot,
            Self::Heroic(asl) | Self::Legendary(asl) => asl.aila(),
            // A managed entry with no store behind it shards under the manager,
            // because there is no other family it could belong to. One with a
            // store behind it shards under that store, so a patch published
            // against `gog:1207658691` is found by the Lutris user who installed
            // the same game through a Lutris script.
            Self::Lutris(asl) => asl.asl.as_ref().map_or(Manassa::Lutris, Self::aila),
            Self::Bottles(_) => Manassa::Bottles,
            Self::Playnite(asl) => asl.asl.as_ref().map_or(Manassa::Playnite, Self::aila),
            Self::Yadawi(_) => Manassa::Yadawi,
        }
    }

    /// The identifier as text: `steam:427520`, `gog:1207658691`, and so on.
    /// This is the string every deterministic identity is derived from and the
    /// key the registry index is sharded by.
    #[must_use]
    pub fn muarrif(&self) -> String {
        match self {
            Self::Steam(q) => format!("steam:{q}"),
            Self::Epic(q) => format!("epic:{q}"),
            Self::Gog(q) => format!("gog:{q}"),
            Self::Ea(q) => format!("ea:{q}"),
            Self::Ubisoft(q) => format!("ubisoft:{q}"),
            Self::BattleNet(q) => format!("battlenet:{q}"),
            Self::Xbox(q) => format!("xbox:{q}"),
            Self::Itch(q) => format!("itch:{q}"),
            Self::Amazon(q) => format!("amazon:{q}"),
            Self::Rockstar(q) => format!("rockstar:{q}"),
            Self::Riot(q) => format!("riot:{q}"),
            Self::Heroic(asl) | Self::Legendary(asl) => asl.muarrif(),
            Self::Lutris(asl) => asl
                .asl
                .as_ref()
                .map_or_else(|| format!("lutris:{}", asl.silaa), Self::muarrif),
            Self::Bottles(q) => format!("bottles:{q}"),
            Self::Playnite(asl) => asl
                .asl
                .as_ref()
                .map_or_else(|| format!("playnite:{}", asl.muarrif), Self::muarrif),
            Self::Yadawi(q) => format!("yadawi:{q}"),
        }
    }

    /// The store's own identity behind a Heroic installation, or this source
    /// itself when there is nothing wrapping it.
    #[must_use]
    pub fn asl(&self) -> &Self {
        match self {
            Self::Heroic(asl) | Self::Legendary(asl) => asl.asl(),
            Self::Lutris(asl) => asl.asl.as_ref().map_or(self, Self::asl),
            Self::Playnite(asl) => asl.asl.as_ref().map_or(self, Self::asl),
            _ => self,
        }
    }

    /// Whether this source is a manager wrapping somebody else's catalogue.
    ///
    /// Four of them are — Heroic, Legendary, Lutris and Playnite — and the
    /// distinction is load-bearing at scan time: two managers can both report
    /// the same underlying game, and so can the store itself. Deduplication
    /// keeps the store's own record as primary and the managers' as additional
    /// install records, because the store is the one that knows the build.
    #[must_use]
    pub const fn mudir(&self) -> bool {
        matches!(
            self,
            Self::Heroic(_) | Self::Legendary(_) | Self::Lutris(_) | Self::Playnite(_)
        )
    }

    /// How the game is launched, when that is not simply "run the executable".
    ///
    /// A managed game frequently cannot be launched by running its own binary:
    /// Lutris applies a runner configuration, Bottles supplies a prefix, and
    /// Heroic passes credentials the game expects. Returning the manager's slug
    /// here is what lets the installer hand the launch back rather than trying
    /// to reproduce a configuration it does not own.
    #[must_use]
    pub fn miftah_tashghil(&self) -> Option<String> {
        match self {
            Self::Lutris(asl) => Some(asl.silaa.clone()),
            Self::Playnite(asl) => Some(asl.muarrif.clone()),
            Self::Bottles(q) => Some(q.clone()),
            Self::Heroic(asl) | Self::Legendary(asl) => asl.miftah_tashghil(),
            _ => None,
        }
    }
}

impl std::fmt::Display for MasdarLuba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.muarrif())
    }
}

/// A colour taken from a game's own cover artwork.
///
/// Used as that game's accent inside its detail screen, and as the placeholder
/// while the artwork itself is still decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct LawnBariz {
    /// Red channel.
    pub ahmar: u8,
    /// Green channel.
    pub akhdar: u8,
    /// Blue channel.
    pub azraq: u8,
}

impl LawnBariz {
    /// The colour as `#rrggbb`, which is how it reaches the stylesheet.
    #[must_use]
    pub fn sittasi(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.ahmar, self.akhdar, self.azraq)
    }

    /// Relative luminance, used to decide whether this colour can carry text
    /// and whether it is legible against the near-black foundation before it is
    /// promoted to an accent.
    #[must_use]
    pub fn idaa(self) -> f32 {
        let khat = |q: u8| {
            let n = f32::from(q) / 255.0;
            if n <= 0.039_28 {
                n / 12.92
            } else {
                ((n + 0.055) / 1.055).powf(2.4)
            }
        };
        0.0722f32.mul_add(
            khat(self.azraq),
            0.2126f32.mul_add(khat(self.ahmar), 0.7152 * khat(self.akhdar)),
        )
    }
}

/// The shard family a game's patches are published under.
///
/// A real type rather than the `&'static str` this used to be. The interface
/// needs these fifteen values — it keys launcher names, icons and filters off
/// them — and a function returning a string literal produces no definition for
/// the generated bindings at all, so the frontend carried a hand-written union
/// that nothing checked against this list. Fifteen values in two places, with
/// no compiler between them, is a rename away from an interface that silently
/// filters nothing.
///
/// A manager that wraps a store is not a family: Heroic, Legendary, Lutris and
/// Playnite resolve to whatever they wrap, and only stand for themselves when
/// there is no store behind the entry. That is [`MasdarLuba::aila`]'s job, not
/// this type's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Manassa {
    /// Steam.
    Steam,
    /// The Epic Games Store.
    Epic,
    /// GOG Galaxy.
    Gog,
    /// The EA app.
    Ea,
    /// Ubisoft Connect.
    Ubisoft,
    /// Battle.net.
    ///
    /// Renamed explicitly: `rename_all` would make this `battle_net`, and the
    /// wire form is the registry's shard key. Every patch already published
    /// against Battle.net is filed under `battlenet`, so the derived spelling
    /// would silently stop finding all of them.
    #[serde(rename = "battlenet")]
    BattleNet,
    /// The Xbox app and the Microsoft Store.
    Xbox,
    /// itch.io.
    Itch,
    /// Amazon Games.
    Amazon,
    /// The Rockstar Games Launcher.
    Rockstar,
    /// The Riot client.
    Riot,
    /// Lutris, for an entry with no store behind it.
    Lutris,
    /// Bottles, which manages prefixes rather than games.
    Bottles,
    /// Playnite, for an entry with no store behind it.
    Playnite,
    /// A game the user added by hand.
    Yadawi,
}

impl Manassa {
    /// The shard slug, which is the wire form and the registry's key.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Epic => "epic",
            Self::Gog => "gog",
            Self::Ea => "ea",
            Self::Ubisoft => "ubisoft",
            Self::BattleNet => "battlenet",
            Self::Xbox => "xbox",
            Self::Itch => "itch",
            Self::Amazon => "amazon",
            Self::Rockstar => "rockstar",
            Self::Riot => "riot",
            Self::Lutris => "lutris",
            Self::Bottles => "bottles",
            Self::Playnite => "playnite",
            Self::Yadawi => "yadawi",
        }
    }
}

impl std::fmt::Display for Manassa {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

/// A game's artwork, stored content-addressed in the cache.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct SuwarLuba {
    /// The vertical cover, which is what the library grid shows.
    pub ghilaf: Option<String>,
    /// The wide banner, which is what the detail header shows.
    pub batl: Option<String>,
    /// The small logo overlaid on the banner.
    pub shiar: Option<String>,
    /// The dominant colour extracted from the cover at import time.
    pub lawn: Option<LawnBariz>,
}

/// A game's Arabization status, as the library grid badges it.
///
/// This is computed, not stored: it depends on what the registry currently
/// offers and on what is installed right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HalatLuba {
    /// A patch is installed and active.
    Mutabbaqa,
    /// A patch exists for this build and can be installed now.
    Mutaha,
    /// The engine is supported, but nobody has published a patch yet.
    MadumBilaRuqaa,
    /// A patch exists but does not match this build.
    MutahaGhayrMutabiqa,
    /// The engine is not one Taarib can patch; only the overlay applies.
    TabaqaFaqat,
    /// Taarib refuses to touch this game, because of anti-cheat.
    Marfuda,
}

/// Whether one half of a game — its interface, or its subtitles — already
/// reaches the player in Arabic.
///
/// Three values rather than a `bool` because "we looked and it is not there"
/// and "we could not look" are different answers, and collapsing them is how a
/// product ends up asserting that a game has no Arabic when what actually
/// happened is that its text is sealed inside a container nothing opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TughtiyaLugha {
    /// Evidence says this half is Arabic.
    Muakkada,
    /// Evidence says this half is not Arabic.
    Ghaiba,
    /// Nothing seen either way.
    Majhula,
}

impl TughtiyaLugha {
    /// Whether this half is known to be covered.
    #[must_use]
    pub const fn mughattat(self) -> bool {
        matches!(self, Self::Muakkada)
    }

    /// Folds a second observation of equal standing into the first.
    ///
    /// A positive observation wins over a negative one, and both win over
    /// silence. That asymmetry is only correct between observations that are
    /// worth the same: a store listing naming Arabic and a store listing
    /// omitting it. Two observations of *different* weight are decided by
    /// weight instead — the game's own compiled resources outrank a store
    /// listing in both directions, because a listing can be wrong either way
    /// and a shipped culture cannot.
    #[must_use]
    pub const fn wa(self, ukhra: Self) -> Self {
        match (self, ukhra) {
            (Self::Muakkada, _) | (_, Self::Muakkada) => Self::Muakkada,
            (Self::Ghaiba, _) | (_, Self::Ghaiba) => Self::Ghaiba,
            _ => Self::Majhula,
        }
    }
}

/// How a fact about a game's official Arabic was learned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NawDaleelLugha {
    /// The launcher's own declared language metadata — Steam's
    /// `supported_languages`, an `AppxManifest` resource list, and their kin.
    LughatMatjar,
    /// A compiled localization resource the engine ships: an Unreal `.locres`
    /// under a culture directory, a Unity string table carrying a locale code.
    /// The strongest of the three, because it is the game's own files rather
    /// than a store listing somebody may have forgotten to update.
    MawridMuharrik,
    /// A locale-named file or directory in the game's own data, recognised by
    /// its shape rather than opened.
    MasarThaqafa,
}

/// One observation behind an official-Arabic verdict.
///
/// The same shape, and for the same reason, as
/// [`crate::muharrik::Daleel`]: a user who disagrees with the verdict has to
/// be able to see what produced it, and a maintainer fixing a wrong verdict
/// has to be able to see which observation was wrong.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct DaleelLugha {
    /// How it was observed.
    pub naw: NawDaleelLugha,
    /// What was observed, stated plainly.
    pub wasf: String,
    /// Where — a store key, a path inside a container, a path under the
    /// install root.
    pub mawqi: Option<String>,
    /// How much this observation is worth, 0 to 100.
    #[cfg_attr(feature = "mukhattatat", schemars(range(max = 100)))]
    pub wazn: u8,
}

/// What a game already offers an Arabic-speaking player, as the library grid
/// badges it.
///
/// Derived from [`HukmLughaRasmiya`] rather than stored beside it, so that the
/// badge and the evidence can never disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HalatLughaRasmiya {
    /// No official Arabic was found. Arabization is welcome here.
    Ghaib,
    /// The menus are Arabic; the dialogue is not.
    WajihaFaqat,
    /// The story is readable; the menus are not.
    NususFaqat,
    /// Interface and subtitles both. Nothing to offer.
    Kamila,
    /// Arabic is shipped, and how much of it could not be established.
    ///
    /// Not a failure and not a hedge: it is the difference between a store
    /// listing that names Arabic and a container that would say how far it
    /// goes but was never opened. Offering to arabize this game would still be
    /// wrong; promising the user which half is covered would also be wrong.
    Mubhama,
}

impl HalatLughaRasmiya {
    /// Whether the publisher already ships Arabic in any amount.
    ///
    /// The one question the install path asks: a game answering `true` is
    /// never offered for arabization, whatever the scope turned out to be.
    #[must_use]
    pub const fn yatakallam_arabi(self) -> bool {
        !matches!(self, Self::Ghaib)
    }

    /// The badge text, as the interface writes it.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::Ghaib => "بلا عربية",
            Self::WajihaFaqat => "واجهة عربية",
            Self::NususFaqat => "ترجمة عربية",
            Self::Kamila => "عربية كاملة",
            Self::Mubhama => "عربية رسمية",
        }
    }

    /// The same in English.
    #[must_use]
    pub const fn ism_injilizi(self) -> &'static str {
        match self {
            Self::Ghaib => "No Arabic",
            Self::WajihaFaqat => "Arabic interface",
            Self::NususFaqat => "Arabic subtitles",
            Self::Kamila => "Full Arabic",
            Self::Mubhama => "Official Arabic",
        }
    }
}

/// Whether a game already speaks Arabic, and everything that says so.
///
/// This is the value the whole question travels as. A game whose publisher
/// paid translators, ran QA and had a native speaker review the result must
/// never be offered a machine translation over the top of it, so this verdict
/// gates the install path — and because it gates it, it carries its own
/// reasons: [`dalail`](Self::dalail) is what a user reads when they disagree,
/// and [`majhul`](Self::majhul) is what they read when the answer is honestly
/// incomplete.
///
/// The two halves are modelled separately rather than as one enum because
/// that is how the underlying data is actually shaped — Steam records
/// `supported`, `subtitles` and `full_audio` as independent flags — and
/// because collapsing them early is what makes it impossible to say later
/// which half a verdict was really about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct HukmLughaRasmiya {
    /// Whether the menus, settings and prompts are Arabic.
    pub wajiha: TughtiyaLugha,
    /// Whether the dialogue and narration are Arabic.
    pub nusus: TughtiyaLugha,
    /// How much the verdict is worth, 0 to 100.
    #[cfg_attr(feature = "mukhattatat", schemars(range(max = 100)))]
    pub thiqa: u8,
    /// Everything that was observed, heaviest first.
    pub dalail: Vec<DaleelLugha>,
    /// What could not be seen, one sentence each.
    ///
    /// A game whose text lives inside an encrypted `.pak` produces an empty
    /// evidence list and a full one of these, and the difference matters: the
    /// first says "there is no Arabic here", the second says "nobody looked".
    pub majhul: Vec<String>,
    /// The launcher's declared languages, verbatim, in the launcher's own
    /// spelling and sorted.
    ///
    /// Kept on the verdict because it is what the cache key is derived from: a
    /// store update that adds Arabic to a game changes this list, and a verdict
    /// whose list no longer matches the store's is a verdict that has to be
    /// recomputed.
    pub lughat_muallana: Vec<String>,
    /// The version of the detector that produced this. A verdict from an older
    /// detector is recomputed rather than trusted.
    pub isdar_fahs: u32,
    /// When it was decided, RFC 3339.
    pub waqt: String,
}

impl HukmLughaRasmiya {
    /// The badge state.
    #[must_use]
    pub const fn hala(&self) -> HalatLughaRasmiya {
        match (self.wajiha, self.nusus) {
            (TughtiyaLugha::Muakkada, TughtiyaLugha::Muakkada) => HalatLughaRasmiya::Kamila,
            (TughtiyaLugha::Muakkada, TughtiyaLugha::Ghaiba) => HalatLughaRasmiya::WajihaFaqat,
            (TughtiyaLugha::Ghaiba, TughtiyaLugha::Muakkada) => HalatLughaRasmiya::NususFaqat,
            (TughtiyaLugha::Muakkada, TughtiyaLugha::Majhula)
            | (TughtiyaLugha::Majhula, TughtiyaLugha::Muakkada) => HalatLughaRasmiya::Mubhama,
            _ => HalatLughaRasmiya::Ghaib,
        }
    }

    /// Whether the publisher already ships Arabic in any amount.
    #[must_use]
    pub const fn yatakallam_arabi(&self) -> bool {
        self.hala().yatakallam_arabi()
    }

    /// Whether the verdict rests on anything that was actually seen.
    ///
    /// `false` means every probe declined — no launcher declared a language
    /// list and no container would open — and the [`HalatLughaRasmiya::Ghaib`]
    /// that comes back is the absence of evidence rather than evidence of
    /// absence. A caller that offers arabization on `false` is offering it
    /// blind, which is a choice it should make deliberately.
    #[must_use]
    pub const fn hasim(&self) -> bool {
        !self.dalail.is_empty()
    }
}

/// A discovered game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Luba {
    /// Taarib's identity for this game.
    pub id: LubaId,
    /// Every launcher identity it is known by, in discovery order.
    pub masadir: Vec<MasdarLuba>,
    /// The name as the launcher gives it, kept verbatim for display.
    pub ism: String,
    /// The installation root.
    pub jidhr: PathBuf,
    /// The executable Taarib will probe and launch, once it is known.
    pub tanfidhi: Option<PathBuf>,
    /// Size on disk in bytes, as the launcher reports it.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hajm: u64,
    /// When the user last played, RFC 3339, where the launcher records it.
    pub akhir_laab: Option<String>,
    /// When the launcher last updated the game, RFC 3339.
    pub akhir_tahdith: Option<String>,
    /// The build currently installed.
    pub bina: Option<BinaId>,
    /// Artwork and its extracted colour.
    pub suwar: SuwarLuba,
    /// The compatibility layer the game runs behind.
    pub beea: BeeatTawafuq,
    /// Whether the game is still present on disk. An uninstalled game is kept
    /// rather than deleted, so its patches, projects and backups survive a
    /// reinstall.
    pub mawjuda: bool,
    /// Hidden from the library by the user.
    pub mukhfiya: bool,
}

impl Luba {
    /// The launcher identity to prefer when talking to the registry: the store
    /// the game really belongs to, unwrapping Heroic.
    #[must_use]
    pub fn masdar_asli(&self) -> Option<&MasdarLuba> {
        self.masadir.first().map(MasdarLuba::asl)
    }

    /// Whether this game is known to the given launcher identity.
    #[must_use]
    pub fn yatba(&self, masdar: &MasdarLuba) -> bool {
        self.masadir.iter().any(|q| q.asl() == masdar.asl())
    }
}
