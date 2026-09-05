//! الوتيرة — what the overlay gives up when it cannot afford itself, and how it
//! says so.
//!
//! [`crate::wajiha::MeezaniyatItar`] already answers one question: may the
//! overlay *draw* this frame. Its answer is to skip the draw, and for the thing
//! it guards — one quad batch through the game's own device — that is the right
//! answer, because a skipped draw is one frame without Arabic and the next frame
//! puts it back.
//!
//! It is the wrong answer for everything else the overlay does. Capture,
//! preprocessing, recognition and translation are two orders of magnitude more
//! expensive than the draw and they do not happen on the frame at all; when a
//! session is struggling, it is almost never the draw that is costing the money.
//! Skipping draws while the machine is buried under recognition produces the
//! worst possible outcome: the game still stutters, and the Arabic flickers on
//! and off while it does.
//!
//! So this module degrades the *refresh rate* instead. The overlay keeps drawing
//! the most recent completed translation on every single frame — that is nearly
//! free, the batch is already built — and what slows down is how often it goes
//! and reads the screen again. A player who has degraded to one recognition a
//! second sees dialogue appear a little late. A player whose frames were being
//! dropped sees a game that feels broken and has no idea why.
//!
//! ## Reporting is not optional
//!
//! An overlay that silently halves its own refresh rate is an overlay that a
//! user experiences as "the translations are late sometimes" and can do nothing
//! about. [`MunazzimWatira::wasf`] states the base rate, the rate in force, how
//! many frames went over budget, and the worst frame measured — the unflattering
//! numbers included, exactly as [`crate::wajiha::MeezaniyatItar::wasf`] does.
//! [`MunazzimWatira::sajjil_itar`] returns the moment of every change so a
//! caller can put it in the log as it happens rather than only in a summary
//! nobody opens.
//!
//! ## Hysteresis, and why recovery is slower than degradation
//!
//! Degrading after a short run of over-budget frames and recovering after a long
//! run of cheap ones is deliberately asymmetric. A game that alternates between
//! a quiet menu and a busy scene would otherwise oscillate — degrade in the
//! scene, recover in the menu, degrade again — and an overlay whose refresh rate
//! changes every two seconds is worse than one that simply stayed slow. The
//! product of the two windows is that degradation is believed quickly and
//! recovery has to be earned.
//!
//! ## No floating point in the interval
//!
//! The interval is multiplied and divided by a hundredths factor in integer
//! arithmetic. That is not fussiness: this runs on a game's render thread, the
//! interval is compared against a microsecond counter, and a `f32` round trip
//! through a value near a million microseconds does not come back the same
//! number. An interval that drifts by a microsecond per degradation is an
//! interval that stops being the number the panel is displaying.

use std::fmt::Write as _;

/// The slowest this build will let the refresh rate fall to, in microseconds.
///
/// Four seconds. Past that the overlay is not a translation overlay any more —
/// a line of dialogue is on screen for two or three seconds, so a four-second
/// interval means whole lines are never read at all. A session that cannot
/// afford this is a session that should be told to turn the overlay off, which
/// is what [`MunazzimWatira::mustanfada`] is for.
pub const AQSA_FASIL_MIKRO: u64 = 4_000_000;

/// The fastest a caller may ask for, in microseconds.
///
/// Fifty milliseconds, matching [`crate::manatiq::ADNA_FASILA_MILLI`], so the
/// governor and the per-region interval agree about what "as fast as this
/// product goes" means rather than disagreeing by a factor a user would have to
/// discover.
pub const ADNA_FASIL_MIKRO: u64 = 50_000;

/// What one degradation step multiplies the interval by, in hundredths.
///
/// Doubling. A smaller step takes many windows to reach a rate that actually
/// helps, and during all of them the machine is still over budget; a larger one
/// overshoots past a rate the session could have sustained. Doubling reaches
/// four seconds from a quarter of a second in four steps, which is four windows
/// of sustained overrun — long enough that a single expensive scene transition
/// does not move it at all.
pub const MUAMIL_IFTIRADI_MIA: u32 = 200;

/// How many consecutive over-budget frames degrade the rate.
///
/// Thirty. At sixty frames a second that is half a second of sustained overrun,
/// which no scene transition, shader compile or texture stream-in lasts and
/// every genuinely overloaded session exceeds continuously.
pub const NAFIDHAT_TADAHWUR: u32 = 30;

/// How many consecutive within-budget frames recover one step.
///
/// Eight times the degradation window — four seconds at sixty frames a second.
/// See this module's header on hysteresis.
pub const MUDAAF_TAAFI: u32 = 8;

/// Which way the refresh rate just moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaghyeerWatira {
    /// The overlay could not afford its budget and slowed itself down.
    Tadahwur {
        /// The interval before, in microseconds.
        min_mikro: u64,
        /// The interval now, in microseconds.
        ila_mikro: u64,
        /// How many steps of degradation are now in force.
        daraja: u32,
    },

    /// The overlay has been inside its budget long enough to speed back up.
    Taafi {
        /// The interval before, in microseconds.
        min_mikro: u64,
        /// The interval now, in microseconds.
        ila_mikro: u64,
        /// How many steps of degradation remain.
        daraja: u32,
    },
}

impl TaghyeerWatira {
    /// The interval now in force.
    #[must_use]
    pub const fn ila_mikro(self) -> u64 {
        match self {
            Self::Tadahwur { ila_mikro, .. } | Self::Taafi { ila_mikro, .. } => ila_mikro,
        }
    }

    /// Whether the overlay just got slower.
    #[must_use]
    pub const fn tadahwur(self) -> bool {
        matches!(self, Self::Tadahwur { .. })
    }

    /// The log line this change writes, in English, with the real numbers.
    #[must_use]
    pub fn satr(self) -> String {
        match self {
            Self::Tadahwur { min_mikro, ila_mikro, daraja } => format!(
                "refresh degraded from {:.2}/s to {:.2}/s (step {daraja}) after sustained \
                 over-budget frames",
                fi_thania(min_mikro),
                fi_thania(ila_mikro)
            ),
            Self::Taafi { min_mikro, ila_mikro, daraja } => format!(
                "refresh recovered from {:.2}/s to {:.2}/s ({daraja} step(s) still in force)",
                fi_thania(min_mikro),
                fi_thania(ila_mikro)
            ),
        }
    }

    /// The sentence the in-game panel shows, in Arabic.
    #[must_use]
    pub fn unwan(self) -> String {
        match self {
            Self::Tadahwur { ila_mikro, .. } => format!(
                "تجاوزت الطبقة ميزانية الإطار، فخُفّضت وتيرة القراءة إلى {:.2} مرة في الثانية.",
                fi_thania(ila_mikro)
            ),
            Self::Taafi { ila_mikro, .. } => format!(
                "عادت الطبقة داخل ميزانيتها، ورُفعت وتيرة القراءة إلى {:.2} مرة في الثانية.",
                fi_thania(ila_mikro)
            ),
        }
    }
}

/// The governor: one interval, one budget, and the record of what it did to
/// them.
#[derive(Debug, Clone)]
pub struct MunazzimWatira {
    asas_mikro: u64,
    hali_mikro: u64,
    aqsa_mikro: u64,
    saqf_mikro: u32,
    muamil_mia: u32,
    nafidha: u32,
    tatawul: u32,
    tahassun: u32,
    daraja: u32,
    marrat_tadahwur: u64,
    marrat_taafi: u64,
    itarat: u64,
    itarat_fawq: u64,
    akhir_mikro: u32,
    dhurwa_mikro: u32,
}

impl MunazzimWatira {
    /// A governor over a base interval and the per-frame ceiling it defends.
    ///
    /// The interval is clamped into [`ADNA_FASIL_MIKRO`]..=[`AQSA_FASIL_MIKRO`]
    /// and the ceiling to a floor of one hundred microseconds, for the reason
    /// [`crate::wajiha::Tabaqa::ihdud`] gives: a ceiling of zero is an overlay
    /// that never draws while claiming to be on.
    #[must_use]
    pub fn jadeed(asas_mikro: u64, saqf_mikro: u32) -> Self {
        let asas = asas_mikro.clamp(ADNA_FASIL_MIKRO, AQSA_FASIL_MIKRO);
        Self {
            asas_mikro: asas,
            hali_mikro: asas,
            aqsa_mikro: AQSA_FASIL_MIKRO,
            saqf_mikro: saqf_mikro.max(100),
            muamil_mia: MUAMIL_IFTIRADI_MIA,
            nafidha: NAFIDHAT_TADAHWUR,
            tatawul: 0,
            tahassun: 0,
            daraja: 0,
            marrat_tadahwur: 0,
            marrat_taafi: 0,
            itarat: 0,
            itarat_fawq: 0,
            akhir_mikro: 0,
            dhurwa_mikro: 0,
        }
    }

    /// A governor at the default poll interval and the default frame ceiling.
    #[must_use]
    pub fn iftiradi() -> Self {
        Self::jadeed(
            crate::iltiqat_shasha::MuqayyidMuadal::FASIL_IFTIRADI_MIKRO,
            crate::wajiha::MeezaniyatItar::SAQF_IFTIRADI,
        )
    }

    /// The interval the user asked for, in microseconds.
    #[must_use]
    pub const fn asas_mikro(&self) -> u64 {
        self.asas_mikro
    }

    /// The interval actually in force, in microseconds.
    ///
    /// What a capture scheduler must poll at. Equal to
    /// [`MunazzimWatira::asas_mikro`] until something degraded it.
    #[must_use]
    pub const fn fasil_mikro(&self) -> u64 {
        self.hali_mikro
    }

    /// The per-frame ceiling being defended, in microseconds.
    #[must_use]
    pub const fn saqf_mikro(&self) -> u32 {
        self.saqf_mikro
    }

    /// How many steps of degradation are in force. Zero when healthy.
    #[must_use]
    pub const fn daraja(&self) -> u32 {
        self.daraja
    }

    /// Whether the overlay is running slower than it was asked to.
    ///
    /// The one boolean the control panel needs in order to show a warning
    /// rather than a number. A user who can see this is a user who knows the
    /// overlay is struggling instead of wondering why the game feels wrong.
    #[must_use]
    pub const fn mutadahwira(&self) -> bool {
        self.daraja > 0
    }

    /// Whether degradation has run out of room.
    ///
    /// True when the interval has reached [`AQSA_FASIL_MIKRO`] and frames are
    /// still over budget. Nothing further can be given up: the honest next step
    /// is the user turning the overlay off, and the panel is where that is said.
    #[must_use]
    pub const fn mustanfada(&self) -> bool {
        self.hali_mikro >= self.aqsa_mikro && self.tatawul >= self.nafidha
    }

    /// How many frames have been measured.
    #[must_use]
    pub const fn itarat(&self) -> u64 {
        self.itarat
    }

    /// How many of them were over budget.
    #[must_use]
    pub const fn itarat_fawq(&self) -> u64 {
        self.itarat_fawq
    }

    /// The worst frame measured, in microseconds.
    #[must_use]
    pub const fn dhurwa_mikro(&self) -> u32 {
        self.dhurwa_mikro
    }

    /// Changes the base interval, keeping whatever degradation is in force.
    ///
    /// Degradation is expressed as steps rather than as an absolute interval
    /// precisely so this works: a user who lowers the base rate while the
    /// overlay is degraded gets the lower rate, still degraded by the same
    /// amount, rather than silently recovering because the number changed.
    pub fn ihdud(&mut self, asas_mikro: u64) {
        self.asas_mikro = asas_mikro.clamp(ADNA_FASIL_MIKRO, AQSA_FASIL_MIKRO);
        self.hali_mikro = self.min_daraja(self.daraja);
    }

    /// Changes the per-frame ceiling being defended.
    pub const fn ihdud_saqf(&mut self, saqf_mikro: u32) {
        self.saqf_mikro = if saqf_mikro < 100 { 100 } else { saqf_mikro };
    }

    /// Puts the rate back to the base and forgets the run counters.
    ///
    /// For a surface change or a level transition, where the frames just
    /// measured describe a scene that no longer exists.
    pub const fn istanif(&mut self) {
        self.daraja = 0;
        self.hali_mikro = self.asas_mikro;
        self.tatawul = 0;
        self.tahassun = 0;
    }

    /// Records one frame's overlay cost and reports any change it caused.
    ///
    /// `mikro` is the caller's measurement of the overlay's own cost on this
    /// frame — the same number [`crate::wajiha::Tabaqa::itar`] is given. This
    /// crate reads no clock.
    ///
    /// Returns [`Some`] only on the frame the rate actually moved, so a caller
    /// can log every change without logging every frame.
    pub fn sajjil_itar(&mut self, mikro: u32) -> Option<TaghyeerWatira> {
        self.itarat = self.itarat.saturating_add(1);
        self.akhir_mikro = mikro;
        self.dhurwa_mikro = self.dhurwa_mikro.max(mikro);

        if mikro > self.saqf_mikro {
            self.itarat_fawq = self.itarat_fawq.saturating_add(1);
            self.tahassun = 0;
            self.tatawul = self.tatawul.saturating_add(1);
            if self.tatawul < self.nafidha {
                return None;
            }
            self.tatawul = 0;
            return self.anzil();
        }

        self.tatawul = 0;
        self.tahassun = self.tahassun.saturating_add(1);
        if self.daraja == 0 || self.tahassun < self.nafidha.saturating_mul(MUDAAF_TAAFI) {
            return None;
        }
        self.tahassun = 0;
        self.arfa()
    }

    /// The sentence the control panel and the log carry.
    ///
    /// Every number, including the ones that make the overlay look bad. A user
    /// deciding whether to keep tier 3 on is owed the worst frame it produced,
    /// not the average.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = format!(
            "refresh {:.2}/s of a requested {:.2}/s; {} frame(s) measured, {} over the {} µs \
             budget, worst {} µs",
            fi_thania(self.hali_mikro),
            fi_thania(self.asas_mikro),
            self.itarat,
            self.itarat_fawq,
            self.saqf_mikro,
            self.dhurwa_mikro
        );
        if self.daraja > 0 {
            let _ = write!(
                wasf,
                "; degraded {} step(s) ({} degradation(s), {} recovery(ies))",
                self.daraja, self.marrat_tadahwur, self.marrat_taafi
            );
        }
        if self.mustanfada() {
            wasf.push_str("; the slowest refresh this build offers is not enough for this scene");
        }
        wasf
    }

    /// The same sentence in Arabic, for the in-game panel.
    #[must_use]
    pub fn unwan(&self) -> String {
        if self.daraja == 0 {
            return format!(
                "وتيرة القراءة {:.2} مرة في الثانية، وهي المطلوبة. أسوأ إطار {} ميكروثانية.",
                fi_thania(self.hali_mikro),
                self.dhurwa_mikro
            );
        }
        let mut unwan = format!(
            "خُفّضت وتيرة القراءة إلى {:.2} مرة في الثانية بدل {:.2}، بعد {} إطارًا تجاوز \
             ميزانية {} ميكروثانية.",
            fi_thania(self.hali_mikro),
            fi_thania(self.asas_mikro),
            self.itarat_fawq,
            self.saqf_mikro
        );
        if self.mustanfada() {
            unwan.push_str(" ولم يبقَ ما يمكن تخفيضه؛ إيقاف الطبقة هو الخيار الصادق هنا.");
        }
        unwan
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// The interval at a given number of degradation steps.
    ///
    /// Derived from the base rather than accumulated, so repeatedly degrading
    /// and recovering cannot drift the interval away from the number the panel
    /// is displaying.
    fn min_daraja(&self, daraja: u32) -> u64 {
        let mut fasil = self.asas_mikro;
        let muamil = u64::from(self.muamil_mia.max(110));
        for _ in 0..daraja.min(16) {
            fasil = fasil
                .saturating_mul(muamil)
                .checked_div(100)
                .unwrap_or(self.aqsa_mikro)
                .min(self.aqsa_mikro);
        }
        fasil.clamp(ADNA_FASIL_MIKRO, self.aqsa_mikro)
    }

    /// Takes one step down, or reports nothing when there is no room left.
    fn anzil(&mut self) -> Option<TaghyeerWatira> {
        if self.hali_mikro >= self.aqsa_mikro {
            return None;
        }
        let min_mikro = self.hali_mikro;
        self.daraja = self.daraja.saturating_add(1);
        self.hali_mikro = self.min_daraja(self.daraja);
        if self.hali_mikro == min_mikro {
            // The multiplier could not move it — the interval is already at the
            // ceiling. The step is rolled back so the reported degradation level
            // stays a true description of the interval in force.
            self.daraja = self.daraja.saturating_sub(1);
            return None;
        }
        self.marrat_tadahwur = self.marrat_tadahwur.saturating_add(1);
        Some(TaghyeerWatira::Tadahwur {
            min_mikro,
            ila_mikro: self.hali_mikro,
            daraja: self.daraja,
        })
    }

    /// Takes one step back up.
    fn arfa(&mut self) -> Option<TaghyeerWatira> {
        if self.daraja == 0 {
            return None;
        }
        let min_mikro = self.hali_mikro;
        self.daraja = self.daraja.saturating_sub(1);
        self.hali_mikro = self.min_daraja(self.daraja);
        self.marrat_taafi = self.marrat_taafi.saturating_add(1);
        Some(TaghyeerWatira::Taafi {
            min_mikro,
            ila_mikro: self.hali_mikro,
            daraja: self.daraja,
        })
    }
}

impl Default for MunazzimWatira {
    fn default() -> Self {
        Self::iftiradi()
    }
}

/// An interval in microseconds, as a rate per second.
///
/// For display only. Every comparison in this module is integer arithmetic on
/// the interval itself — see the module header on why the interval never round
/// trips through a float.
fn fi_thania(mikro: u64) -> f64 {
    if mikro == 0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "intervals are capped at AQSA_FASIL_MIKRO, four million, exact in f64"
    )]
    let ashari = mikro as f64;
    1_000_000.0 / ashari
}
