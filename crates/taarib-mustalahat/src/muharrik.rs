//! المحرّك — an engine, its backend, and what Taarib can actually do to it.
//!
//! The capability report ([`TaqreerImkaniyat`]) produced from these types is a
//! product artifact, not a debug dump. It is what the user reads on a game's
//! detail screen before deciding to install anything, and it is the single
//! value every downstream dispatch decision is made from: which adapter runs,
//! which framework is installed, which extraction path is taken, and what the
//! interface promises.

use serde::{Deserialize, Serialize};
use taarib_usus::manassa::Mimariya;

/// The engine family a game is built on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum AilatMuharrik {
    /// Unity, either scripting backend.
    Unity,
    /// Unreal Engine 4 or 5.
    Unreal,
    /// Godot 3 or 4.
    Godot,
    /// RPG Maker MV.
    RpgMakerMv,
    /// RPG Maker MZ.
    RpgMakerMz,
    /// RPG Maker VX Ace.
    RpgMakerVxAce,
    /// Ren'Py.
    Renpy,
    /// GameMaker Studio.
    GameMaker,
    /// Electron, NW.js, or anything else drawing its interface in a browser.
    Electron,
    /// Capcom's `BIO4` codebase — the GameCube-era in-house engine written for
    /// Resident Evil 4 in 2005 and carried into its later ports.
    ///
    /// Named after the string Capcom's own binary carries: the Windows version
    /// resource of `bio4.exe` gives `InternalName` as `BIO4`, which is the
    /// project name the engine was built under and the name of the data
    /// directory it reads. Capcom never published a name or a version for it, so
    /// there is no marketing name to use instead and no version number to
    /// report.
    ///
    /// **It is not MT Framework, and the belief that it is is widespread and
    /// wrong.** MT Framework was written for the seventh generation, ships its
    /// assets as `.tex` inside `ARC\0` archives, and Capcom's own MT Framework
    /// titles carry that string. `bio4.exe` contains no occurrence of
    /// `MT Framework` or `MTFramework`; its textures are `.tpl`, the GameCube
    /// texture format, magic `0x12345678`; its compression container is `RDLX`;
    /// and the eight files under it that do end in `.arc` open with
    /// `0x55AA382D`, which is Nintendo's U8 archive and not MT Framework's.
    /// [`crate::muharrik`]'s detector in `taarib-muharrik` records each of those
    /// so that a reader who expects MT Framework can see why it is not.
    Bio4,
    /// Nothing Taarib recognises. A first-class answer, not a failure.
    Majhul,
}

impl AilatMuharrik {
    /// The engine's name as the interface writes it.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Unity => "Unity",
            Self::Unreal => "Unreal Engine",
            Self::Godot => "Godot",
            Self::RpgMakerMv => "RPG Maker MV",
            Self::RpgMakerMz => "RPG Maker MZ",
            Self::RpgMakerVxAce => "RPG Maker VX Ace",
            Self::Renpy => "Ren'Py",
            Self::GameMaker => "GameMaker Studio",
            Self::Electron => "Electron",
            // The engine has no published name. This is the one Capcom itself
            // wrote into the binary, with the company in front of it so that a
            // reader who has never met the string knows whose engine it is.
            Self::Bio4 => "Capcom BIO4",
            Self::Majhul => "غير معروف",
        }
    }

    /// Whether Taarib can replace text inside this engine at all, or whether
    /// the overlay is the only route.
    #[must_use]
    pub const fn qabil_lil_tarqee(self) -> bool {
        !matches!(self, Self::Majhul)
    }
}

/// An engine version, kept both parsed and raw.
///
/// The raw string is preserved because engine version strings carry more than
/// three numbers — `2021.3.16f1`, `4.27.2-17155196+++UE4`, `3.5.2.stable` — and
/// the extra part is exactly what a signature database keys on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct IsdarMuharrik {
    /// Major version.
    pub kabir: u16,
    /// Minor version.
    pub sagheer: u16,
    /// Patch version.
    pub tasheeh: u16,
    /// The version string exactly as it was found in the game.
    pub khaam: String,
    /// Whether this was inferred rather than read.
    ///
    /// A container format version narrows the engine to a span of releases —
    /// pak format 11 means "4.27 or any UE5" — so the numbers above are the
    /// bottom of a range and not a version any file states. Resolution has to
    /// know the difference, because a derived range must never outrank an exact
    /// version however strong the detector that carried it, and a reader has to
    /// know it before treating three numbers as a fact about the game.
    ///
    /// It is a field rather than a marker word in `khaam` because `khaam` is
    /// prose shown to a user, and a rule that depends on prose breaks silently
    /// the first time someone rewords a sentence.
    #[serde(default)]
    pub mushtaqq: bool,
}

impl std::fmt::Display for IsdarMuharrik {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.khaam)
    }
}

/// How the game's own code is executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum KhalfiyaBarmajiya {
    /// Unity's Mono runtime, with managed assemblies on disk.
    Mono,
    /// Unity's IL2CPP, with the managed code compiled to a native binary.
    Il2cpp,
    /// Unreal's compiled C++ and Blueprints.
    UnrealNative,
    /// Godot's own scripting language.
    GdScript,
    /// Godot with C#.
    GodotCSharp,
    /// A Godot project compiled into a custom engine build.
    GodotNative,
    /// JavaScript, in NW.js, Electron, or a browser.
    JavaScript,
    /// Python, as Ren'Py uses it.
    Python,
    /// Ruby, as RPG Maker VX Ace uses it.
    Ruby,
    /// GameMaker's own bytecode.
    GameMakerVm,
    /// Nothing identifiable.
    Majhula,
}

/// A text system present in the game, and therefore a surface Taarib can take
/// over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ItarNusus {
    /// TextMeshPro, in both its UI and world-space components.
    TextMeshPro,
    /// Unity's legacy UI text.
    UnityUiText,
    /// NGUI's labels.
    NGui,
    /// `FairyGUI`'s text fields.
    FairyGui,
    /// Unity world-space text meshes.
    TextMesh,
    /// Unreal's Slate, which already shapes text and only needs correcting.
    Slate,
    /// Godot's plain labels.
    GodotLabel,
    /// Godot's rich text labels.
    GodotRichText,
    /// RPG Maker's message and menu windows.
    NafidhatRpg,
    /// Ren'Py's text displayables.
    NassRenpy,
    /// GameMaker's text drawing.
    RasmGameMaker,
    /// Text nodes in a document.
    Dom,
    /// A canvas the game draws text into itself, where the browser's own
    /// shaping never runs.
    Canvas,
}

impl ItarNusus {
    /// The system's name as the capability report writes it in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::TextMeshPro => "نظام TextMeshPro",
            Self::UnityUiText => "نصوص واجهة Unity",
            Self::NGui => "نصوص NGUI",
            Self::FairyGui => "نصوص FairyGUI",
            Self::TextMesh => "نصوص ثلاثية الأبعاد",
            Self::Slate => "نظام Slate في أنريل",
            Self::GodotLabel => "عناصر Label في غودوت",
            Self::GodotRichText => "عناصر RichTextLabel في غودوت",
            Self::NafidhatRpg => "نوافذ الرسائل والقوائم",
            Self::NassRenpy => "نصوص رِن باي",
            Self::RasmGameMaker => "رسم النصوص في GameMaker",
            Self::Dom => "نصوص المستند",
            Self::Canvas => "النصوص المرسومة على اللوحة",
        }
    }
}

/// A graphics API the game was seen to use, which decides how the overlay
/// attaches when it is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum WajihaRusum {
    /// Direct3D 11.
    D3d11,
    /// Direct3D 12.
    D3d12,
    /// OpenGL.
    OpenGl,
    /// Vulkan.
    Vulkan,
    /// Metal.
    Metal,
    /// Not determined.
    Majhula,
}

/// What kind of observation produced a piece of evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NawDaleel {
    /// A directory or file the engine always ships.
    BinyatMujallad,
    /// An imported module, a section name, or a byte pattern in a binary.
    TawqiThunai,
    /// A version string or structure embedded in the game's own data.
    BayanatMudmaja,
    /// The header of an asset container.
    TarwisatHawiya,
}

/// One observation that contributed to an engine identification.
///
/// Evidence is kept, not discarded, because a wrong identification is diagnosed
/// by looking at what was seen — and because a user is entitled to know why
/// Taarib decided their game is what it says it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Daleel {
    /// How it was observed.
    pub naw: NawDaleel,
    /// What was observed, stated plainly.
    pub wasf: String,
    /// Where, relative to the game's root.
    pub mawqi: Option<String>,
    /// How much this observation is worth, 0 to 100.
    #[cfg_attr(feature = "mukhattatat", schemars(range(max = 100)))]
    pub wazn: u8,
}

/// An identified engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Muharrik {
    /// The family.
    pub aila: AilatMuharrik,
    /// The version, when one could be read.
    pub isdar: Option<IsdarMuharrik>,
    /// How the game's code runs.
    pub khalfiya: KhalfiyaBarmajiya,
    /// Every text system found.
    pub itarat: Vec<ItarNusus>,
    /// Every graphics API found.
    pub rusum: Vec<WajihaRusum>,
    /// The architecture of the game's main executable.
    pub mimariya: Mimariya,
    /// Confidence in this identification, 0 to 100. Conflicting evidence lowers
    /// it and is reported rather than averaged away.
    #[cfg_attr(feature = "mukhattatat", schemars(range(max = 100)))]
    pub thiqa: u8,
    /// Everything that was observed.
    pub dalail: Vec<Daleel>,
}

/// Which injection tier applies to a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Tabaqa {
    /// Text is replaced inside the game and looks native.
    Kamil,
    /// Taarib draws the text itself over the engine's own text objects.
    RasmMubashir,
    /// The game is not modified; Arabic is shown over it. A reading aid, and
    /// described as one everywhere it appears.
    TarjamaFawqiya,
}

impl Tabaqa {
    /// The tier number, 1 to 3, as the interface labels it.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Kamil => 1,
            Self::RasmMubashir => 2,
            Self::TarjamaFawqiya => 3,
        }
    }

    /// The tier's name in Arabic, used verbatim in the interface.
    #[must_use]
    pub const fn ism_arabi(self) -> &'static str {
        match self {
            Self::Kamil => "تعريب كامل",
            Self::RasmMubashir => "تعريب بالرسم المباشر",
            Self::TarjamaFawqiya => "طبقة ترجمة",
        }
    }

    /// The tier's name in English.
    #[must_use]
    pub const fn ism_injilizi(self) -> &'static str {
        match self {
            Self::Kamil => "Full Arabization",
            Self::RasmMubashir => "Arabization by direct drawing",
            Self::TarjamaFawqiya => "Translation overlay",
        }
    }

    /// The one-paragraph explanation shown under the tier name, in Arabic.
    #[must_use]
    pub const fn sharh_arabi(self) -> &'static str {
        match self {
            Self::Kamil => {
                "تُستبدل نصوص اللعبة من الداخل. القوائم والحوارات والواجهة تظهر بالعربية كأن \
                 اللعبة صُنعت بها."
            }
            Self::RasmMubashir => {
                "يرسم تعريب النص بنفسه فوق عناصر النص في اللعبة. النتيجة تبدو أصلية في أغلب \
                 الحالات، والمؤثرات التي تضيفها اللعبة على نصوصها يعيد تعريب إنتاجها بنفسه."
            }
            Self::TarjamaFawqiya => {
                "لا تُعدَّل اللعبة إطلاقًا. يقرأ تعريب ما يظهر على الشاشة ويعرض العربية فوقه. \
                 هذه وسيلة قراءة مساعدة، وليست ترجمة مثبّتة داخل اللعبة."
            }
        }
    }
}

/// Whether the code that delivers a tier is finished, for one engine, in this
/// build.
///
/// [`Tabaqa`] says what Taarib is *entitled* to do to a game. This says whether
/// the half that does it — the adapter that runs inside the game — exists yet.
/// The two are separate values on purpose, and collapsing them would be a lie in
/// one direction or the other: an engine whose adapter is unwritten would either
/// promise a tier nothing delivers, or be described as if the engine itself made
/// Arabization impossible. Neither is true, and a user can act on the difference
/// — an inherent limit never improves, an unfinished adapter arrives in an
/// update.
///
/// This is a fact about the *build*, not about the game. Two users with the same
/// Taarib version get the same answer for the same engine, and the answer moves
/// when Taarib is updated rather than when the game is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum JahiziyatTashghil {
    /// Everything the tier promises reaches the screen.
    Mukammala,
    /// Part of the tier reaches the screen and a named part does not.
    Naqisa,
    /// None of it reaches the screen. The patch installs, the game starts, and
    /// it runs in its original language.
    ///
    /// The default, and deliberately the pessimistic one: a report stored by a
    /// build that predates this field carries no value for it, and defaulting to
    /// the state that warns rather than the state that promises is the only safe
    /// direction. Such a report is replaced on the next launch anyway, because
    /// adding this field moved the probe version.
    #[default]
    Ghaiba,
}

impl JahiziyatTashghil {
    /// Whether anything the tier promises reaches the screen.
    #[must_use]
    pub const fn tasil(self) -> bool {
        !matches!(self, Self::Ghaiba)
    }

    /// Whether the report has something extra to say about this build's reach.
    ///
    /// False for a game that works fully, which is what stops the interface
    /// adding a notice to every screen it draws.
    #[must_use]
    pub const fn tastahiq_tanbeeh(self) -> bool {
        !matches!(self, Self::Mukammala)
    }

    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mukammala => "mukammala",
            Self::Naqisa => "naqisa",
            Self::Ghaiba => "ghaiba",
        }
    }
}

/// How good the result is expected to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum JawdaMutawaqqaa {
    /// Everything the player reads will be Arabic, correctly shaped.
    Mumtaza,
    /// The great majority will be Arabic, with named exceptions.
    Jayida,
    /// The main text will be Arabic; some systems will stay in the original.
    Maqbula,
    /// Only part of the text is reachable at all.
    Mahduda,
}

/// One named limitation the user will actually meet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Hadd {
    /// The limitation in Arabic, written as a complete sentence.
    pub arabi: String,
    /// The same sentence in English.
    pub injilizi: String,
}

/// What Taarib can do to one specific game, in language a player understands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct TaqreerImkaniyat {
    /// The identified engine and the evidence behind it.
    pub muharrik: Muharrik,
    /// The tier the engine qualifies for.
    ///
    /// What Taarib is entitled to do to this game, and the value every
    /// downstream dispatch reads. Whether the code that does it exists yet is
    /// [`Self::jahiziya`], and the two must be read together: a tier is a
    /// permission, not a promise.
    pub tabaqa: Tabaqa,
    /// Why that tier and not a better one, in Arabic.
    pub sabab_arabi: String,
    /// The same reason in English.
    pub sabab_injilizi: String,
    /// Whether this build can actually deliver [`Self::tabaqa`] for this engine.
    ///
    /// Defaulted on read so that a report stored by a build that predates the
    /// field still parses; see [`JahiziyatTashghil::Ghaiba`] for why the default
    /// is the pessimistic one.
    #[serde(default)]
    pub jahiziya: JahiziyatTashghil,
    /// What is unfinished, named, in Arabic and English.
    ///
    /// `None` when [`Self::jahiziya`] is [`JahiziyatTashghil::Mukammala`], and
    /// also when an older stored report is being read back — a report from
    /// before this field existed cannot say what its build could not do, and
    /// inventing a sentence for it would be worse than saying nothing.
    ///
    /// Deliberately not a [`Self::hudud`] entry. A limitation is something about
    /// the game that no update will change; this is something about Taarib that
    /// an update will. Presenting them as one list would tell a user their game
    /// cannot be Arabized when in fact it has not been Arabized *yet*.
    #[serde(default)]
    pub naqs: Option<Hadd>,
    /// The text systems Taarib will take over.
    ///
    /// Empty whenever nothing is taken over — at tier 3, and whenever
    /// [`Self::jahiziya`] is [`JahiziyatTashghil::Ghaiba`], because naming
    /// systems no code reaches would be the report claiming a tier it cannot
    /// deliver.
    pub anzimat_qabila: Vec<ItarNusus>,
    /// The expected quality of the result.
    pub jawda: JawdaMutawaqqaa,
    /// Everything that will not work, named specifically.
    pub hudud: Vec<Hadd>,
    /// Whether the safety layer refuses this game outright.
    pub marfuda: bool,
    /// The version of the probe that produced this report, so an improved probe
    /// re-examines the game instead of trusting a stale conclusion.
    pub isdar_fahs: u32,
    /// When the probe ran, RFC 3339.
    pub waqt: String,
}

/// File names an engine itself defines, which more than one crate must spell.
///
/// Not a convenience module. Each of these is a name an engine reads, written
/// by one crate and planned for by another, where the two crates deliberately
/// cannot depend on each other — the installer must not pull in a payload that
/// runs inside somebody's game. Spelled separately, a rename in one would leave
/// the installer writing a file the engine never reads: the patch installs, the
/// game starts, no Arabic appears, and nothing logs.
pub mod asmaa_muharrik {
    /// Godot's project override, read from beside the executable at startup.
    ///
    /// `taarib-muhawwil-godot` writes it; `taarib-tathbeet` plans where it goes.
    pub const TAJAWUZ_GODOT: &str = "override.cfg";
}
