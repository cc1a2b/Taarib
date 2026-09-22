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
    /// Whether the licence itself grants the right to redistribute the work.
    ///
    /// All rights reserved is the author keeping them, and a licence this build
    /// cannot name is one nobody here has read. Both answer "ask first", which
    /// for a registry that publishes to strangers is the same answer as no.
    #[must_use]
    pub const fn tasmah_biiadat_alnashr(&self) -> bool {
        match self {
            Self::Cc0 | Self::CcBy | Self::CcBySa => true,
            Self::MilkiyaKhassa | Self::Ukhra { .. } => false,
        }
    }
}

/// What establishes a right to redistribute somebody else's translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum IdhnMasdar {
    /// The licence it was published under grants it.
    Rukhsa,
    /// The author granted it directly, and this is how they said so.
    Katabi {
        /// The permission in the importer's own words: where it was given and
        /// when, so a reader can go and check it.
        bayan: String,
    },
    /// Nothing establishes it.
    ///
    /// The default for anything found on the internet. No licence file is not a
    /// permissive licence; it is the absence of one, and the absence of one
    /// reserves every right.
    LamYuthbat,
}

/// A translation this patch took from somebody outside Taarib.
///
/// Recorded so the people who did the work are named wherever the patch goes,
/// and so the question that decides whether it may be published at all is
/// answered before a signing key is put to it rather than after somebody
/// complains. Attribution and permission are different things: a patch that
/// credits an author it had no licence from is still a patch that should not
/// have been published, and crediting them makes the breach easier to find, not
/// smaller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct MasdarKhariji {
    /// Who made it, in their own spelling of their own name.
    pub ism: String,
    /// Where it was taken from, so the credit points somewhere.
    pub rabt: String,
    /// The licence it was offered under, as the importer read it.
    pub rukhsa: RukhsaRuqaa,
    /// What establishes the right to redistribute it.
    pub idhn: IdhnMasdar,
    /// Whether that permission also covers Taarib hosting a copy of the file.
    ///
    /// A separate grant from redistribution, recorded separately. An author who
    /// agrees to their work being offered through Taarib has agreed to a
    /// listing, an installer and a credit; they have not thereby agreed to
    /// Taarib serving their bytes off its own infrastructure, which moves who
    /// pays for the bandwidth, who sees the download counts and who is
    /// answerable for the copy. Conflating the two would decide that for them.
    ///
    /// `false` unless somebody wrote otherwise, and serde-defaulted so every
    /// record written before this field existed reads as the answer nobody gave.
    #[serde(default)]
    pub yasmah_bilmira: bool,
}

impl MasdarKhariji {
    /// Whether this may be redistributed through the registry.
    #[must_use]
    pub const fn yajuz_nashruh(&self) -> bool {
        match self.idhn {
            // A grant from the author outranks what the licence file says,
            // because the author is who the licence file speaks for.
            IdhnMasdar::Katabi { .. } => true,
            IdhnMasdar::Rukhsa => self.rukhsa.tasmah_biiadat_alnashr(),
            IdhnMasdar::LamYuthbat => false,
        }
    }
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
    /// The translation it was taken from, when it was taken from one.
    ///
    /// Carried into the catalogue itself, not left behind in the contributor's
    /// own record, because the credit is owed wherever the patch is read and a
    /// listing is where most people will read it. Absent on every entry
    /// published before this existed, and on every patch translated from
    /// nothing but the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masdar_khariji: Option<MasdarKhariji>,
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

#[cfg(test)]
mod ikhtibarat_masdar {
    use super::{IdhnMasdar, MasdarKhariji, RukhsaRuqaa};

    fn masdar(rukhsa: RukhsaRuqaa, idhn: IdhnMasdar) -> MasdarKhariji {
        MasdarKhariji {
            ism: "Emad Adel".to_owned(),
            rabt: "https://github.com/emadadeldev/RTEA".to_owned(),
            rukhsa,
            idhn,
            yasmah_bilmira: false,
        }
    }

    /// Work found with no licence may not be republished, however well credited.
    ///
    /// This is the case that matters, because it is the ordinary one: a
    /// translation on a forge with no LICENSE file is not permissively
    /// licensed, it is licensed to nobody. Crediting the author does not change
    /// that — it only makes the breach easier to find — and the registry
    /// publishes under the owner's signing key to strangers, so the question has
    /// to be settled before the key is used.
    #[test]
    fn bila_rukhsa_la_yunshar() {
        assert!(!masdar(RukhsaRuqaa::MilkiyaKhassa, IdhnMasdar::LamYuthbat).yajuz_nashruh());
        assert!(!masdar(RukhsaRuqaa::Cc0, IdhnMasdar::LamYuthbat).yajuz_nashruh());
        assert!(
            !masdar(RukhsaRuqaa::MilkiyaKhassa, IdhnMasdar::Rukhsa).yajuz_nashruh(),
            "all rights reserved is the author keeping them"
        );
        assert!(
            !masdar(
                RukhsaRuqaa::Ukhra {
                    ism: "RTEA-custom".to_owned()
                },
                IdhnMasdar::Rukhsa
            )
            .yajuz_nashruh(),
            "a licence this build cannot name is one nobody here has read"
        );
    }

    /// A licence that grants redistribution, or the author saying so, is enough.
    #[test]
    fn rukhsa_aw_idhn_yasmah() {
        for rukhsa in [RukhsaRuqaa::Cc0, RukhsaRuqaa::CcBy, RukhsaRuqaa::CcBySa] {
            assert!(masdar(rukhsa, IdhnMasdar::Rukhsa).yajuz_nashruh());
        }
        // The author outranks the licence file, because the file speaks for them.
        assert!(
            masdar(
                RukhsaRuqaa::MilkiyaKhassa,
                IdhnMasdar::Katabi {
                    bayan: "granted by email, 2026-09-22".to_owned()
                }
            )
            .yajuz_nashruh()
        );
    }

    /// A record written before mirroring was a separate question reads as
    /// nobody having answered it, which is the answer that keeps the bytes on
    /// the author's own endpoint.
    #[test]
    fn sijill_qadeem_la_yasmah_bilmira() -> Result<(), serde_json::Error> {
        let qadeem = br#"{"ism":"Emad Adel","rabt":"https://example.invalid",
            "rukhsa":{"naw":"cc0"},"idhn":"rukhsa"}"#;
        let masdar: MasdarKhariji = serde_json::from_slice(qadeem)?;
        assert!(masdar.yajuz_nashruh());
        assert!(!masdar.yasmah_bilmira);
        Ok(())
    }
}
