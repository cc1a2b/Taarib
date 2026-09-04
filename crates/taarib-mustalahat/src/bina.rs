//! البناء — a build, and the content fingerprint that outlives it.
//!
//! A launcher's build identifier changes every time the store pushes anything:
//! a shader recompile, a new platform binary, a storefront asset. Most of those
//! updates do not touch a single translatable string, yet a patch bound only to
//! a build identifier breaks on all of them — which is why almost every
//! community patch dies within weeks of release.
//!
//! Taarib binds a patch to both: the launcher's build identifier *and* a
//! [`Basma`] over the game's text-bearing files. When the identifier moves and
//! the fingerprint does not, the patch still applies exactly, and the client
//! says so instead of asking the user to hope.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A 256-bit content fingerprint.
///
/// Computed with BLAKE3 over a canonical, ordered selection of the files that
/// can carry text, each contributing its relative path, its length, and its
/// contents. Ordering is by normalized relative path so that two machines
/// hashing the same installation agree byte for byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(
    feature = "mukhattatat",
    derive(schemars::JsonSchema),
    schemars(extend("pattern" = "^[0-9a-f]{64}$"))
)]
pub struct Basma(
    // The wire form is 64 lowercase hexadecimal characters. A byte array would
    // serialize as a JSON array of numbers, which is unreadable in a registry
    // record a human is expected to be able to audit.
    #[cfg_attr(feature = "wajiha", specta(type = String))]
    #[cfg_attr(feature = "mukhattatat", schemars(with = "String"))]
    [u8; 32],
);

impl Basma {
    /// Wraps a digest produced by the hashing layer.
    #[must_use]
    pub const fn min_bayt(bayt: [u8; 32]) -> Self {
        Self(bayt)
    }

    /// The raw digest.
    #[must_use]
    pub const fn bayt(&self) -> &[u8; 32] {
        &self.0
    }

    /// The first four bytes as text, which is what the interface shows when a
    /// full fingerprint would be noise.
    #[must_use]
    pub fn mukhtasara(&self) -> String {
        self.0.iter().take(4).fold(String::with_capacity(8), |mut out, bayt| {
            use fmt::Write as _;
            let _ = write!(out, "{bayt:02x}");
            out
        })
    }
}

impl fmt::Display for Basma {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for bayt in &self.0 {
            write!(f, "{bayt:02x}")?;
        }
        Ok(())
    }
}

/// The failure of parsing a fingerprint from text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a content fingerprint is 64 lowercase hexadecimal characters")]
pub struct BasmaGhayrSaliha;

impl FromStr for Basma {
    type Err = BasmaGhayrSaliha;

    fn from_str(nass: &str) -> Result<Self, Self::Err> {
        if nass.len() != 64 {
            return Err(BasmaGhayrSaliha);
        }
        let mut bayt = [0u8; 32];
        let mut huruf = nass.as_bytes().chunks_exact(2);
        for khana in &mut bayt {
            let Some(zawj) = huruf.next() else { return Err(BasmaGhayrSaliha) };
            let nass_zawj = std::str::from_utf8(zawj).map_err(|_| BasmaGhayrSaliha)?;
            if nass_zawj.bytes().any(|b| b.is_ascii_uppercase()) {
                return Err(BasmaGhayrSaliha);
            }
            *khana = u8::from_str_radix(nass_zawj, 16).map_err(|_| BasmaGhayrSaliha)?;
        }
        Ok(Self(bayt))
    }
}

impl Serialize for Basma {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Basma {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let nass = String::deserialize(d)?;
        nass.parse().map_err(serde::de::Error::custom)
    }
}

/// Which build of a game is installed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct BinaId {
    /// The launcher's own build identifier, where the launcher has one. Steam
    /// reports a numeric `buildid`; GOG reports a build hash; several launchers
    /// report nothing at all, which is exactly why the fingerprint exists.
    pub manassa: Option<String>,
    /// The fingerprint of the text-bearing files.
    pub basma: Basma,
    /// How many files went into the fingerprint, so that a fingerprint computed
    /// over a partially downloaded installation is recognisable as one.
    pub adad_malaffat: u32,
    /// When the fingerprint was computed, RFC 3339.
    pub waqt: String,
}

impl BinaId {
    /// A short label for the interface: the launcher's build when there is one,
    /// otherwise the first bytes of the fingerprint.
    #[must_use]
    pub fn wasm(&self) -> String {
        self.manassa.clone().unwrap_or_else(|| self.basma.mukhtasara())
    }
}

/// How well a patch matches the build a user actually has.
///
/// Ordered from best to worst, and shown to the user with exactly this wording
/// rather than hidden behind a single "compatible" flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum MutabaqaBina {
    /// The patch declares this exact build identifier.
    Tamma,
    /// The build identifier moved but the text-bearing files did not, so the
    /// patch still applies exactly. This is the tier that keeps patches alive
    /// through routine store updates.
    Basma,
    /// The build falls inside a compatibility range the contributor declared,
    /// with neither an exact identifier nor a fingerprint match.
    Nitaq,
    /// Nothing matches. Shown with the reason, never silently hidden.
    Ghayr,
}

impl MutabaqaBina {
    /// The label the interface shows, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Tamma => "متوافقة تمامًا مع نسختك",
            Self::Basma => "متوافقة — التحديث الأخير لم يغيّر النصوص",
            Self::Nitaq => "متوافقة تقريبًا — قد تظهر نصوص غير مترجمة",
            Self::Ghayr => "غير متوافقة مع نسختك الحالية",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Tamma => "Exactly matches your build",
            Self::Basma => "Compatible — the last update did not change any text",
            Self::Nitaq => "Approximately compatible — some text may be untranslated",
            Self::Ghayr => "Not compatible with your build",
        }
    }

    /// Whether installing requires the user to acknowledge a risk first.
    #[must_use]
    pub const fn yahtaj_iqrar(self) -> bool {
        matches!(self, Self::Nitaq)
    }

    /// Whether the client will install this at all.
    #[must_use]
    pub const fn qabila_lil_tathbeet(self) -> bool {
        !matches!(self, Self::Ghayr)
    }
}
