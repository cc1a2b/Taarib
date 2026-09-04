//! المساهم — a contributor and their standing.
//!
//! Identity is a public key, not an account. There is no server holding a user
//! table, no password to leak, and nothing to recover if a machine is lost
//! except a key the contributor kept. A contributor is exactly whoever can sign
//! with the key their published patches were signed with, which is the same
//! trust model the registry itself runs on.

use serde::{Deserialize, Serialize};

use crate::taghtiya::Taghtiya;

/// A contributor's identity: the fingerprint of their public signing key.
///
/// Rendered as 64 lowercase hexadecimal characters and shown to users
/// abbreviated to its first eight, which is enough to distinguish contributors
/// while staying readable in a table.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(
    feature = "mukhattatat",
    derive(schemars::JsonSchema),
    schemars(extend("pattern" = "^[0-9a-f]{64}$"))
)]
#[serde(transparent)]
pub struct MusahimId(String);

impl MusahimId {
    /// Wraps a fingerprint produced by the signing layer.
    ///
    /// # Errors
    ///
    /// Returns [`MusahimIdGhayrSalih`] unless the value is exactly 64 lowercase
    /// hexadecimal characters, because an identity that is not a key
    /// fingerprint is an identity somebody made up.
    pub fn jadeed(basma: impl Into<String>) -> Result<Self, MusahimIdGhayrSalih> {
        let basma = basma.into();
        let salih = basma.len() == 64
            && basma.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
        if salih { Ok(Self(basma)) } else { Err(MusahimIdGhayrSalih) }
    }

    /// The full fingerprint.
    #[must_use]
    pub fn nass(&self) -> &str {
        &self.0
    }

    /// The first eight characters, which is what tables and cards show.
    #[must_use]
    pub fn mukhtasar(&self) -> String {
        self.0.chars().take(8).collect()
    }
}

impl std::fmt::Display for MusahimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The failure of building an identity from something that is not a key
/// fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a contributor identity is the 64-character fingerprint of a public signing key")]
pub struct MusahimIdGhayrSalih;

/// A contributor's record, as it appears on every listing they published.
///
/// The counters are derived from the registry's own public history — every
/// approval, every requested change, every revocation is a commit — so nothing
/// here is a score anyone can grant, revoke, or inflate outside the review
/// process itself.
// No `Eq`: the average rating is a float, and equality on floats is not
// reflexive. `PartialEq` is the most this type can honestly claim.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Sumaa {
    /// Patches published.
    pub ruqaa_manshura: u32,
    /// Submissions accepted with no changes requested — the single most honest
    /// signal of a careful contributor.
    pub qubila_bila_taadil: u32,
    /// Submissions that were returned for revision.
    pub tulib_taadil: u32,
    /// Submissions rejected outright.
    pub marfuda: u32,
    /// Published patches that were later revoked.
    pub masbuba: u32,
    /// Average rating across their published patches, when anyone has rated.
    pub mutawassit_taqyeem: Option<f32>,
    /// How many ratings that average is built on.
    pub adad_taqyeemat: u32,
}

impl Sumaa {
    /// The share of submissions accepted without a revision round.
    #[must_use]
    pub fn nisbat_qubul(&self) -> f32 {
        let majmu = self
            .qubila_bila_taadil
            .saturating_add(self.tulib_taadil)
            .saturating_add(self.marfuda);
        if majmu == 0 {
            return 0.0;
        }
        // Both counts are exact in f32; dividing in f64 and narrowing rounded
        // twice to reach the same answer.
        nisba_f32(self.qubila_bila_taadil, majmu)
    }
}

/// One count over another, as a display ratio.
#[expect(
    clippy::cast_precision_loss,
    reason = "submission counts are far below 2^24, where f32 is still exact"
)]
fn nisba_f32(juz: u32, kull: u32) -> f32 {
    juz as f32 / kull as f32
}

/// A contributor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Musahim {
    /// Their key fingerprint.
    pub id: MusahimId,
    /// The name they chose to be known by.
    pub ism: String,
    /// The credit line that appears inside the patch and on its listing,
    /// exactly as the contributor wrote it.
    pub satr_itiraf: Option<String>,
    /// A link they chose to publish — a profile, a site, nothing at all.
    pub rabt: Option<String>,
    /// Their standing.
    pub sumaa: Sumaa,
    /// When their first patch was published, RFC 3339.
    pub mundhu: Option<String>,
    /// Total coverage across everything they have published, which is a fairer
    /// measure of contribution than a patch count.
    pub majmu_taghtiya: Taghtiya,
}
