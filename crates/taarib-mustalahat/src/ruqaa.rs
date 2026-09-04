//! الرقعة — a patch: its identity, its lineage, its method, and its licence.
//!
//! A patch has two identities on purpose. [`RuqaaId`] is the *lineage*: it
//! survives every revision, so a contributor improving their own work is
//! recognisably the same patch and a user who installed revision 3 is offered
//! revision 4. [`RuqaaRevision`] is the *version*: it increments on every
//! publication and is what the client compares to decide whether an update
//! exists.
//!
//! The full container metadata lives with the container format itself. What is
//! here is the vocabulary every other part of the product needs to speak about
//! a patch without depending on the compiler that produced it.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::bina::Basma;
use crate::muharrik::{AilatMuharrik, KhalfiyaBarmajiya, Tabaqa};
use crate::musahim::MusahimId;
use crate::taghtiya::Taghtiya;

/// A patch lineage, stable across every revision of the same work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct RuqaaId(Uuid);

impl RuqaaId {
    /// Starts a new lineage.
    ///
    /// Version 7, so identities sort by creation time — which makes the
    /// registry's per-game listings naturally chronological without a separate
    /// index, and makes a directory of patch files readable in the order they
    /// were made.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps a stored identity.
    #[must_use]
    pub const fn min_uuid(qeema: Uuid) -> Self {
        Self(qeema)
    }

    /// The underlying value.
    #[must_use]
    pub const fn uuid(self) -> Uuid {
        self.0
    }

    /// A short form for tables.
    #[must_use]
    pub fn mukhtasar(self) -> String {
        self.0.as_simple().to_string().chars().take(8).collect()
    }
}

impl std::fmt::Display for RuqaaId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

/// A published revision of a patch, monotonic within its lineage.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct RuqaaRevision(u32);

impl RuqaaRevision {
    /// The first revision.
    pub const AWWAL: Self = Self(1);

    /// Wraps a stored revision.
    #[must_use]
    pub const fn jadeeda(qeema: u32) -> Self {
        Self(qeema)
    }

    /// The numeric value.
    #[must_use]
    pub const fn qeema(self) -> u32 {
        self.0
    }

    /// The next revision.
    #[must_use]
    pub const fn talia(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl std::fmt::Display for RuqaaRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "r{}", self.0)
    }
}

/// How a translation was actually produced.
///
/// Declared by the contributor, shown on every listing, and never inferred:
/// a machine-only patch is a legitimate thing to publish and a dishonest thing
/// to disguise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TareeqaTarjama {
    /// Translated by a person, start to finish.
    BashariyaKamila,
    /// Machine-translated, then reviewed and corrected by a person.
    AaliyaThumBashariya,
    /// Machine-translated with no human review.
    AaliyaFaqat,
}

impl TareeqaTarjama {
    /// The label shown on the listing, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::BashariyaKamila => "ترجمة بشرية كاملة",
            Self::AaliyaThumBashariya => "ترجمة آلية راجعها إنسان",
            Self::AaliyaFaqat => "ترجمة آلية دون مراجعة",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::BashariyaKamila => "Fully human translation",
            Self::AaliyaThumBashariya => "Machine-assisted, human-reviewed",
            Self::AaliyaFaqat => "Machine only, unreviewed",
        }
    }

    /// Whether submitting with this method requires an explicit acknowledgement
    /// from the contributor.
    #[must_use]
    pub const fn yahtaj_iqrar(self) -> bool {
        matches!(self, Self::AaliyaFaqat)
    }
}

/// The licence a contributor publishes their translation under.
///
/// This covers the translated text and nothing else. A patch never contains any
/// original game asset, so no licence here has anything to say about the game
/// itself.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum RukhsaRuqaa {
    /// Public domain dedication.
    Cc0,
    /// Attribution.
    CcBy,
    /// Attribution, share alike.
    CcBySa,
    /// All rights reserved by the contributor; redistribution outside Taarib is
    /// not granted.
    MilkiyaKhassa,
    /// Something else, named by the contributor.
    Ukhra {
        /// The licence identifier or name.
        ism: String,
    },
}

impl RukhsaRuqaa {
    /// The short identifier used in metadata and on listings.
    #[must_use]
    pub fn muarrif(&self) -> &str {
        match self {
            Self::Cc0 => "CC0-1.0",
            Self::CcBy => "CC-BY-4.0",
            Self::CcBySa => "CC-BY-SA-4.0",
            Self::MilkiyaKhassa => "all-rights-reserved",
            Self::Ukhra { ism } => ism,
        }
    }

    /// Whether the licence permits mirroring the patch outside the registry,
    /// which decides whether it is copied into an offline mirror.
    #[must_use]
    pub const fn yasmah_bil_mira(&self) -> bool {
        matches!(self, Self::Cc0 | Self::CcBy | Self::CcBySa)
    }

    /// Whether the licence permits building a new translation on top of this
    /// one.
    ///
    /// A **different question** from [`RukhsaRuqaa::yasmah_bil_mira`], and the
    /// two are easy to conflate because the three permissive licences answer
    /// both the same way. They come apart at [`RukhsaRuqaa::Ukhra`]: a
    /// no-derivatives licence permits redistribution and forbids exactly this,
    /// and a licence Taarib has not been taught about could be either.
    ///
    /// Unknown licences answer **false**, and the asymmetry is deliberate.
    /// Refusing to seed from a patch that would have allowed it costs a
    /// convenience — the user starts from the game's own strings, which is what
    /// they would have done anyway. Seeding from one that forbids it puts
    /// somebody else's work into a stranger's project under a licence neither
    /// of them agreed to, and does so silently.
    #[must_use]
    pub const fn yasmah_bil_ishtiqaq(&self) -> bool {
        matches!(self, Self::Cc0 | Self::CcBy | Self::CcBySa)
    }
}

/// Where a patch stands, from a contributor's draft to a published release.
///
/// The same value drives the Contributions screen and the owner's queue; there
/// is no second state machine for review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum HalatRuqaa {
    /// A local draft, resumable, visible to nobody else.
    Musawwada,
    /// Submitted and waiting in the owner's queue.
    Muqaddama,
    /// The owner has opened it.
    QaydMuraja,
    /// Returned for revision, with a written summary and anchored comments.
    MatlubTaadil,
    /// Approved; the publishing sequence is running.
    MawafaqYunshar,
    /// Published and fetchable by every client.
    Manshura,
    /// Rejected, with a written reason.
    Marfuda,
    /// Withdrawn by the contributor.
    Mashuba,
    /// Published and then revoked, and being uninstalled from clients that
    /// already have it.
    Masbuba,
}

impl HalatRuqaa {
    /// The label the Contributions screen shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Musawwada => "مسوّدة",
            Self::Muqaddama => "مُرسَلة",
            Self::QaydMuraja => "قيد المراجعة",
            Self::MatlubTaadil => "مطلوب تعديل",
            Self::MawafaqYunshar => "مقبولة — قيد النشر",
            Self::Manshura => "منشورة",
            Self::Marfuda => "مرفوضة",
            Self::Mashuba => "مسحوبة",
            Self::Masbuba => "ملغاة بعد النشر",
        }
    }

    /// Whether the contributor can still edit the submission in this state.
    #[must_use]
    pub const fn qabila_lil_tahreer(self) -> bool {
        matches!(self, Self::Musawwada | Self::MatlubTaadil)
    }

    /// Whether the submission is waiting on the owner.
    #[must_use]
    pub const fn fi_intizar_almalik(self) -> bool {
        matches!(self, Self::Muqaddama | Self::QaydMuraja)
    }

    /// Whether clients can install it.
    #[must_use]
    pub const fn mutaha_lil_tathbeet(self) -> bool {
        matches!(self, Self::Manshura)
    }
}

/// The small record the registry index carries for one patch.
///
/// Deliberately small: a shard holds every patch for a slice of the catalogue,
/// and a client fetches whole shards. Anything that is only needed once a user
/// opens a game's detail screen belongs in the full metadata record, not here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MulakhkhasRuqaa {
    /// The lineage.
    pub id: RuqaaId,
    /// The published revision.
    pub murajaa: RuqaaRevision,
    /// The title the contributor gave it.
    pub unwan: String,
    /// Who made it.
    pub musahim: MusahimId,
    /// Their display name, denormalized so a listing needs no second fetch.
    pub ism_musahim: String,
    /// How much of the game it covers.
    pub taghtiya: Taghtiya,
    /// How many strings it carries.
    pub adad_nusus: u32,
    /// The package size in bytes.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub hajm: u64,
    /// Launcher build identifiers this patch was compiled against.
    pub bina_manassa: Vec<String>,
    /// Content fingerprints it matches exactly.
    pub basmat: Vec<Basma>,
    /// The engine it targets.
    pub aila: AilatMuharrik,
    /// The scripting backend it targets.
    pub khalfiya: KhalfiyaBarmajiya,
    /// The tier it installs at.
    pub tabaqa: Tabaqa,
    /// How it was translated.
    pub tareeqa: TareeqaTarjama,
    /// Its licence.
    pub rukhsa: RukhsaRuqaa,
    /// Average rating, when it has any.
    pub taqyeem: Option<f32>,
    /// How many ratings.
    pub adad_taqyeemat: u32,
    /// When this revision was published, RFC 3339.
    pub waqt_nashr: String,
    /// The hash of the package itself, verified during download.
    pub basmat_muhtawa: Basma,
    /// Where the package is, as a release asset.
    pub rabt: String,
    /// A mirror, tried when the primary is unreachable.
    pub rabt_mira: Option<String>,
}
