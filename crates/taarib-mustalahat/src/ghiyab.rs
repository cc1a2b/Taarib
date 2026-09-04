//! غياب — why a game somebody owns is not in their library.
//!
//! Discovery reads a launcher's catalogue. A catalogue is that launcher's record
//! of what it *believes* is installed, and the gap between the belief and the
//! filesystem is where a library's confusing entries come from: a game deleted
//! by hand, a download paused at four percent, a shader pre-cache that made a
//! directory before any content arrived, an install on a drive nobody plugged
//! in. Every one of those is in the launcher's catalogue and none can be
//! patched.
//!
//! This module is the vocabulary for saying so. The check itself lives in
//! `taarib-kashf`, because it is filesystem work; the *reasons* live here,
//! because they reach the interface and the interface is where they matter.
//!
//! ## Why a failing game is louder than a passing one
//!
//! The obvious design filters the list and shows what survives. It is wrong in a
//! way that costs the user their trust rather than their time: a player who owns
//! a game, sees it in Steam, and does not see it in Taarib has no way to learn
//! why. They will conclude the product does not support it. They will be wrong,
//! and they will be reasonable.
//!
//! So every rejection carries a specific reason, the launcher's own name for the
//! game, and the path that failed. The interface shows them in a separate
//! collapsed section with a button that opens the launcher at that game.
//!
//! ## Offline is not absent
//!
//! A game on a disconnected drive is [`SababGhiyab::QursGhayrMuttasil`], not a
//! missing game, and [`SababGhiyab::yubqa`] is what tells the store layer to
//! keep the record rather than delete it. Deleting would lose the user's
//! installed patches, their artwork override and their play history — for a
//! drive that will be back in a minute.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Why a game the launcher listed is not in the library.
///
/// Ordered by how much the user can do about it, which is also the order the
/// interface groups them in: things they are already fixing, then things they
/// can fix, then things that need a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababGhiyab {
    /// The launcher is downloading it right now.
    QaydTanzil {
        /// How far along, when the launcher reports it: zero to one hundred.
        nisba: Option<u8>,
    },
    /// The launcher is updating it right now.
    QaydTahdith,
    /// A download that stopped part way and was not resumed.
    TanzilMutawaqqif {
        /// How far along it got, when the launcher reports it.
        nisba: Option<u8>,
    },
    /// The directory exists but holds far too little to be a game.
    ///
    /// The shader-precache stub, and the leftover of a manual delete. Named
    /// separately from [`SababGhiyab::MujalladMafqud`] because the remedy is
    /// different: this one is repaired by asking the launcher to verify, and a
    /// missing directory is repaired by reinstalling.
    TathbeetNaqis {
        /// What was found, in bytes.
        #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
        hajm: u64,
        /// What this build requires before it will call a directory a game.
        #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
        adna: u64,
    },
    /// The install directory is not there.
    MujalladMafqud {
        /// Where the launcher said it would be.
        masar: PathBuf,
    },
    /// The install directory is there and cannot be read.
    ///
    /// A permissions problem, almost always: a game installed by another user
    /// account, or a directory whose ACL a security product rewrote.
    MujalladMamnu {
        /// The path.
        masar: PathBuf,
        /// What the filesystem said.
        sabab: String,
    },
    /// The primary executable is not where the launcher said it is.
    TanfidhiMafqud {
        /// The path that was expected.
        masar: PathBuf,
    },
    /// The executable is there and is not the one the launcher recorded.
    ///
    /// Only ever raised on a size disagreement. A timestamp that has moved is a
    /// note on a passing verdict, never this.
    TanfidhiMukhtalif {
        /// The path.
        masar: PathBuf,
        /// The size the launcher recorded.
        #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
        musajjal: u64,
        /// The size on disk.
        #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
        mawjud: u64,
    },
    /// A cloud or streaming entry whose local payload has not been fetched.
    ///
    /// The entry is real and the user owns it; there is simply nothing on this
    /// machine to patch. Distinct from a paused download because there is no
    /// download to resume — the launcher will fetch it when the game is started.
    HimlSahabi,
    /// The compatibility prefix this game runs inside is not there.
    BeeaMafquda {
        /// Where the prefix was expected.
        masar: PathBuf,
    },
    /// The prefix exists and the executable does not resolve inside it.
    ///
    /// A Windows path that maps to nothing under the prefix's drive mapping.
    /// Worth its own reason because the remedy — run the game once and let the
    /// launcher rebuild the prefix — is not obvious from "executable missing".
    TanfidhiKharijBeea {
        /// The prefix.
        beea: PathBuf,
        /// The Windows path that did not resolve.
        masar_windows: String,
    },
    /// The volume holding this game is not connected.
    ///
    /// The one reason that is not a problem. The record is kept, its patch state
    /// is kept, and the game returns the moment the drive does.
    QursGhayrMuttasil {
        /// The volume's root, as the platform names it: `E:\` or `/media/games`.
        qurs: String,
    },
    /// A network location that did not answer.
    ///
    /// Kept like a disconnected drive rather than deleted, for the same reason,
    /// but named separately because "the server is down" and "the drive is
    /// unplugged" are different things to tell somebody.
    ShabakaGhayrMutaha {
        /// The location.
        masar: PathBuf,
    },
}

impl SababGhiyab {
    /// Whether the record survives this verdict.
    ///
    /// True for exactly the two reasons that are about *reachability* rather
    /// than about the install. Everything else means the game is genuinely not
    /// on this machine in a usable form, and keeping a record of it would mean
    /// keeping patch state for an install that no longer exists.
    ///
    /// This is the whole reason the enum exists rather than a string: a string
    /// reason would leave "should this be deleted?" to a comparison somewhere
    /// else, and a future reason added without updating that comparison would
    /// quietly start deleting people's patch state.
    #[must_use]
    pub const fn yubqa(&self) -> bool {
        matches!(self, Self::QursGhayrMuttasil { .. } | Self::ShabakaGhayrMutaha { .. })
    }

    /// Whether the launcher is actively working on this game.
    ///
    /// The interface polls these rather than waiting for a filesystem event: a
    /// download finishing is the single most common reason a game moves from the
    /// unavailable section into the grid, and a user watching it happen should
    /// see it happen.
    #[must_use]
    pub const fn jariya(&self) -> bool {
        matches!(self, Self::QaydTanzil { .. } | Self::QaydTahdith)
    }

    /// The section heading this verdict is grouped under.
    ///
    /// Four groups rather than eleven reasons, because a user scanning the
    /// collapsed section wants to know which of four situations they are in, and
    /// the specific reason is one line further in.
    #[must_use]
    pub const fn fia(&self) -> FiatGhiyab {
        match self {
            Self::QaydTanzil { .. } | Self::QaydTahdith => FiatGhiyab::Jariya,
            Self::TanzilMutawaqqif { .. } | Self::TathbeetNaqis { .. } | Self::HimlSahabi => {
                FiatGhiyab::Naqis
            }
            Self::MujalladMafqud { .. }
            | Self::MujalladMamnu { .. }
            | Self::TanfidhiMafqud { .. }
            | Self::TanfidhiMukhtalif { .. }
            | Self::BeeaMafquda { .. }
            | Self::TanfidhiKharijBeea { .. } => FiatGhiyab::Mafquda,
            Self::QursGhayrMuttasil { .. } | Self::ShabakaGhayrMutaha { .. } => {
                FiatGhiyab::GhayrMuttasila
            }
        }
    }

    /// The sentence the user reads, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::QaydTanzil { nisba } => nisba.map_or_else(
                || "قيد التنزيل الآن.".to_owned(),
                |q| format!("قيد التنزيل الآن ({q}٪)."),
            ),
            Self::QaydTahdith => "قيد التحديث الآن.".to_owned(),
            Self::TanzilMutawaqqif { nisba } => nisba.map_or_else(
                || "تنزيل متوقّف لم يكتمل.".to_owned(),
                |q| format!("تنزيل متوقّف عند {q}٪."),
            ),
            Self::TathbeetNaqis { .. } => {
                "مجلّد اللعبة موجود لكنه شبه فارغ؛ يبدو أن التثبيت لم يكتمل. اطلب من المتجر \
                 التحقّق من الملفات."
                    .to_owned()
            }
            Self::MujalladMafqud { .. } => {
                "مجلّد اللعبة غير موجود. ربما حُذفت اللعبة أو نُقلت خارج المتجر.".to_owned()
            }
            Self::MujalladMamnu { .. } => {
                "مجلّد اللعبة موجود لكن لا يمكن قراءته. قد تكون اللعبة مثبّتة باسم مستخدم آخر."
                    .to_owned()
            }
            Self::TanfidhiMafqud { .. } => {
                "ملف تشغيل اللعبة غير موجود في مكانه. اطلب من المتجر التحقّق من الملفات."
                    .to_owned()
            }
            Self::TanfidhiMukhtalif { .. } => {
                "ملف تشغيل اللعبة يختلف عمّا سجّله المتجر، وقد يكون التحديث لم يكتمل.".to_owned()
            }
            Self::HimlSahabi => {
                "هذه اللعبة سحابية ولم يُنزَّل منها شيء على هذا الجهاز بعد.".to_owned()
            }
            Self::BeeaMafquda { .. } => {
                "بيئة التوافق الخاصة بهذه اللعبة غير موجودة. شغّل اللعبة مرة واحدة لينشئها \
                 المتجر."
                    .to_owned()
            }
            Self::TanfidhiKharijBeea { .. } => {
                "ملف التشغيل لا يُحَلّ داخل بيئة التوافق. شغّل اللعبة مرة واحدة ليعيد المتجر \
                 بناء البيئة."
                    .to_owned()
            }
            Self::QursGhayrMuttasil { qurs } => {
                format!("القرص ({qurs}) غير متصل. ستظهر اللعبة فور توصيله.")
            }
            Self::ShabakaGhayrMutaha { .. } => {
                "موقع الشبكة الذي تُخزَّن فيه هذه اللعبة لا يستجيب.".to_owned()
            }
        }
    }

    /// The same sentence in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::QaydTanzil { nisba } => nisba.map_or_else(
                || "Downloading now.".to_owned(),
                |q| format!("Downloading now ({q}%)."),
            ),
            Self::QaydTahdith => "Updating now.".to_owned(),
            Self::TanzilMutawaqqif { nisba } => nisba.map_or_else(
                || "A download that stopped before it finished.".to_owned(),
                |q| format!("A download that stopped at {q}%."),
            ),
            Self::TathbeetNaqis { hajm, adna } => format!(
                "The folder is here but holds only {hajm} bytes, under the {adna} this build \
                 requires before it will call a folder a game. Ask the launcher to verify."
            ),
            Self::MujalladMafqud { masar } => {
                format!("The install folder is not at {}.", masar.display())
            }
            Self::MujalladMamnu { masar, sabab } => {
                format!("{} cannot be read: {sabab}", masar.display())
            }
            Self::TanfidhiMafqud { masar } => {
                format!("The executable is not at {}.", masar.display())
            }
            Self::TanfidhiMukhtalif { masar, musajjal, mawjud } => format!(
                "{} is {mawjud} bytes; the launcher recorded {musajjal}.",
                masar.display()
            ),
            Self::HimlSahabi => {
                "This is a cloud entry with nothing downloaded to this machine.".to_owned()
            }
            Self::BeeaMafquda { masar } => {
                format!("The compatibility prefix is not at {}.", masar.display())
            }
            Self::TanfidhiKharijBeea { beea, masar_windows } => format!(
                "{masar_windows} does not resolve inside the prefix at {}.",
                beea.display()
            ),
            Self::QursGhayrMuttasil { qurs } => {
                format!("The volume {qurs} is not connected.")
            }
            Self::ShabakaGhayrMutaha { masar } => {
                format!("The network location {} did not answer.", masar.display())
            }
        }
    }
}

/// The four groups the unavailable section is organised into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FiatGhiyab {
    /// The launcher is working on it now.
    Jariya,
    /// It is here and incomplete.
    Naqis,
    /// It is not here.
    Mafquda,
    /// It is somewhere that is not reachable right now.
    GhayrMuttasila,
}

impl FiatGhiyab {
    /// The heading in Arabic.
    #[must_use]
    pub const fn unwan_arabi(self) -> &'static str {
        match self {
            Self::Jariya => "قيد التنزيل أو التحديث",
            Self::Naqis => "تثبيت غير مكتمل",
            Self::Mafquda => "ملفات مفقودة",
            Self::GhayrMuttasila => "غير متصلة الآن",
        }
    }

    /// The heading in English.
    #[must_use]
    pub const fn unwan_injilizi(self) -> &'static str {
        match self {
            Self::Jariya => "Installing",
            Self::Naqis => "Incomplete install",
            Self::Mafquda => "Missing files",
            Self::GhayrMuttasila => "Moved or offline",
        }
    }
}

/// What the launcher recorded about an executable, for comparison against disk.
///
/// Both fields optional because most launchers record neither. A launcher that
/// records nothing produces a verdict based on existence alone, which is still
/// worth having — the check that catches the most games is "is the file there".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct ShahidTanfidhi {
    /// The size the launcher recorded, in bytes.
    pub hajm: Option<u64>,
    /// The modification time the launcher recorded, as seconds since the Unix
    /// epoch.
    pub waqt: Option<u64>,
}

impl ShahidTanfidhi {
    /// Nothing recorded, which is the common case.
    #[must_use]
    pub const fn khali() -> Self {
        Self { hajm: None, waqt: None }
    }

    /// Whether this witness constrains anything at all.
    #[must_use]
    pub const fn samit(&self) -> bool {
        self.hajm.is_none() && self.waqt.is_none()
    }
}

/// The gate's answer for one game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct HukmWujud {
    /// Why it is absent, or [`None`] when it is present and launchable.
    pub ghiyab: Option<SababGhiyab>,
    /// Observations that did not reject the game.
    ///
    /// A modification time that has drifted, an executable larger than recorded
    /// because the store patched it in place. Carried so the diagnostics bundle
    /// can explain a game that behaves oddly, and so the integrity check in
    /// Phase 20G has something to compare against later.
    pub mulahazat: Vec<String>,
    /// The executable the gate actually resolved, which for a prefixed game is
    /// not the path the launcher recorded.
    pub tanfidhi: Option<PathBuf>,
    /// The size the gate measured, when it measured one.
    pub hajm: Option<u64>,
}

impl HukmWujud {
    /// Whether the game is admitted to the library.
    #[must_use]
    pub const fn hadir(&self) -> bool {
        self.ghiyab.is_none()
    }

    /// Whether the record survives, present or not.
    ///
    /// A present game obviously survives. An absent one survives only if its
    /// reason says so. The two cases are answered here rather than by callers
    /// combining `hadir()` with a match, because the combination is exactly
    /// where a new reason would get missed.
    #[must_use]
    pub fn yubqa(&self) -> bool {
        self.ghiyab.as_ref().is_none_or(SababGhiyab::yubqa)
    }

    /// A present verdict, carrying whatever was noticed on the way.
    #[must_use]
    pub const fn hadira(tanfidhi: Option<PathBuf>, hajm: Option<u64>, mulahazat: Vec<String>) -> Self {
        Self { ghiyab: None, mulahazat, tanfidhi, hajm }
    }

    /// An absent verdict.
    #[must_use]
    pub const fn ghaiba(sabab: SababGhiyab) -> Self {
        Self { ghiyab: Some(sabab), mulahazat: Vec::new(), tanfidhi: None, hajm: None }
    }
}

