//! التغطية — how much of a game is actually translated.
//!
//! A plain "82% translated" is close to meaningless, because the untranslated
//! 18% is usually the part the player reads first. Taarib therefore measures
//! three different things and shows all of them:
//!
//! * **by string** — the honest denominator, and the one contributors work
//!   against;
//! * **by occurrence** — weighted by how often each string is actually drawn,
//!   which is what the player experiences;
//! * **by first hour** — the strings the runtime capture saw in the opening
//!   session, which is what decides whether a patch feels finished.

use serde::{Deserialize, Serialize};

/// A coverage measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
pub struct Taghtiya {
    /// Every extracted string.
    pub majmu: u32,
    /// Strings with any translation at all.
    pub mutarjam: u32,
    /// Strings that were reviewed and accepted.
    pub muakkad: u32,
    /// Total occurrences of every string, as extraction and capture counted them.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub majmu_takrar: u64,
    /// Occurrences covered by a translation.
    #[cfg_attr(feature = "wajiha", specta(type = specta_typescript::Number))]
    pub mutarjam_takrar: u64,
    /// Strings seen in the opening session of play.
    pub majmu_awwal: u32,
    /// Of those, how many are translated.
    pub mutarjam_awwal: u32,
}

impl Taghtiya {
    /// Coverage by string, 0.0 to 1.0.
    #[must_use]
    pub fn nisba(&self) -> f32 {
        nisba_min(self.mutarjam.into(), self.majmu.into())
    }

    /// Coverage counting only reviewed strings.
    #[must_use]
    pub fn nisba_muakkada(&self) -> f32 {
        nisba_min(self.muakkad.into(), self.majmu.into())
    }

    /// Coverage weighted by how often each string is drawn.
    #[must_use]
    pub fn nisba_mawzuna(&self) -> f32 {
        nisba_min(self.mutarjam_takrar, self.majmu_takrar)
    }

    /// Coverage of the strings a player meets in the first hour.
    #[must_use]
    pub fn nisba_awwal(&self) -> f32 {
        nisba_min(self.mutarjam_awwal.into(), self.majmu_awwal.into())
    }

    /// Strings still waiting for a translation.
    #[must_use]
    pub const fn mutabaqqi(&self) -> u32 {
        self.majmu.saturating_sub(self.mutarjam)
    }

    /// Whether this patch clears the publishable floor.
    ///
    /// The floor is deliberately low and deliberately about the *opening* of
    /// the game rather than the whole of it: a patch that translates every menu
    /// and the first chapter is worth publishing, and a patch that translates
    /// scattered strings across the whole game is not.
    #[must_use]
    pub fn qabila_lil_nashr(&self) -> bool {
        self.majmu > 0 && self.nisba() >= 0.60 && self.nisba_awwal() >= 0.85
    }

    /// A single sentence for the interface, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        format!(
            "{:.0}٪ من النصوص ({} من {})، و{:.0}٪ مما يظهر في أول ساعة لعب.",
            self.nisba() * 100.0,
            self.mutarjam,
            self.majmu,
            self.nisba_awwal() * 100.0
        )
    }

    /// The same sentence in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        format!(
            "{:.0}% of strings ({} of {}), and {:.0}% of what appears in the first hour.",
            self.nisba() * 100.0,
            self.mutarjam,
            self.majmu,
            self.nisba_awwal() * 100.0
        )
    }

    /// Folds another measurement into this one, for a project assembled from
    /// several containers or several collaborators.
    pub const fn damm(&mut self, akhar: &Self) {
        self.majmu = self.majmu.saturating_add(akhar.majmu);
        self.mutarjam = self.mutarjam.saturating_add(akhar.mutarjam);
        self.muakkad = self.muakkad.saturating_add(akhar.muakkad);
        self.majmu_takrar = self.majmu_takrar.saturating_add(akhar.majmu_takrar);
        self.mutarjam_takrar = self.mutarjam_takrar.saturating_add(akhar.mutarjam_takrar);
        self.majmu_awwal = self.majmu_awwal.saturating_add(akhar.majmu_awwal);
        self.mutarjam_awwal = self.mutarjam_awwal.saturating_add(akhar.mutarjam_awwal);
    }
}

/// A ratio that treats an empty denominator as complete rather than as a
/// division by zero, because a container with no strings is fully covered.
#[expect(
    clippy::cast_precision_loss,
    reason = "string counts are far below 2^24, where f32 is still exact, and the result \
              is a display ratio"
)]
fn nisba_min(juz: u64, kull: u64) -> f32 {
    if kull == 0 {
        return 1.0;
    }
    // Divided in f32 rather than divided in f64 and narrowed: the narrowing was
    // the wider of the two roundings and bought nothing, because both operands
    // are counts that f32 represents exactly.
    juz as f32 / kull as f32
}
