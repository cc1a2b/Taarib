//! # أسس تعريب — Taarib foundations
//!
//! The six things every other crate in the workspace depends on, defined once:
//!
//! | module | what it owns |
//! | --- | --- |
//! | [`khata`] | the error model: codes, Arabic and English sentences, next actions |
//! | [`mukhattat`] | versioned data schemas and forward-only migration |
//! | [`masarat`] | the on-disk layout, path safety, and atomic writes |
//! | [`idadat`] | configuration, layered and hot-reloadable |
//! | [`sijill`] | logging, redaction, rotation, and the live diagnostics buffer |
//! | [`manassa`] | the platform: OS, architecture, processes, compatibility layers |
//! | [`kayanat`] | XML general entity references, resolved one way for every reader |
//!
//! No crate outside this one computes a path, invents an error shape, or reads
//! the environment directly. Everything that touches the machine lives behind
//! the `nizam` feature, so [`khata`] and [`mukhattat`] compile to
//! `wasm32-unknown-unknown` for the JavaScript-side adapters.
//!
//! The wasm half of that promise is kept by the **target**, not by the feature.
//! `nizam` is in this crate's `default`, and a workspace-inherited dependency
//! cannot switch a default off unless the root manifest sets `default-features
//! = false` for every consumer — which would mean the twenty crates that do
//! want `nizam` each opting back in. So the four machine-facing modules and the
//! six crates behind them are gated on `not(target_family = "wasm")` as well:
//! `taarib-wasm` reaches this crate through `taarib-saff` and `taarib-lawha`
//! as well as directly, and one of those three enabling `nizam` is enough to
//! put `tracing-appender` — which does not compile for wasm32 — in the graph.
//! Nothing Taarib ships for a non-wasm target is affected, because on every
//! such target the gate is transparent.

pub mod khata;
pub mod mukhattat;

#[cfg(feature = "kayanat")]
pub mod kayanat;

#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub mod idadat;
#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub mod manassa;
#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub mod masarat;
#[cfg(all(feature = "nizam", not(target_family = "wasm")))]
pub mod sijill;

pub use khata::{Khata, Khutura, Khutwa, Natija, Ramz, Tafsir};

use serde::{Deserialize, Serialize};

/// The version of Taarib this build is, used in diagnostics bundles, patch
/// provenance, and the registry's minimum-client check.
pub const ISDAR: &str = env!("CARGO_PKG_VERSION");

/// The interface language.
///
/// Arabic is the default and the primary text everywhere: English strings are
/// translations of the Arabic, not the other way round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Lugha {
    /// العربية.
    #[default]
    Arabi,
    /// English.
    Injilizi,
}

impl Lugha {
    /// The BCP 47 tag, for `lang` attributes and locale-aware formatting.
    #[must_use]
    pub const fn wasm(self) -> &'static str {
        match self {
            Self::Arabi => "ar",
            Self::Injilizi => "en",
        }
    }

    /// The base direction of the interface in this language.
    #[must_use]
    pub const fn ittijah(self) -> &'static str {
        match self {
            Self::Arabi => "rtl",
            Self::Injilizi => "ltr",
        }
    }

    /// Whether the interface is mirrored in this language.
    #[must_use]
    pub const fn maqlub(self) -> bool {
        matches!(self, Self::Arabi)
    }
}

/// Which digits numbers are written with.
///
/// This is a real decision, not a cosmetic one: Arabic-speaking regions differ,
/// and a game's own text may already use one system. It is chosen per patch for
/// game text and per user for the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "wajiha", derive(specta::Type))]
#[cfg_attr(feature = "mukhattatat", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum NizamArqam {
    /// 0123456789 — used across the Maghreb, and correct for identifiers,
    /// paths, hashes and build numbers everywhere.
    #[default]
    Latini,
    /// ٠١٢٣٤٥٦٧٨٩ — Arabic-Indic, used across the Mashriq.
    Arabi,
    /// ۰۱۲۳۴۵۶۷۸۹ — Eastern Arabic-Indic, used for Persian and Urdu.
    Farisi,
}

impl NizamArqam {
    /// The ten digit characters, in value order.
    #[must_use]
    pub const fn huruf(self) -> [char; 10] {
        match self {
            Self::Latini => ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'],
            Self::Arabi => ['٠', '١', '٢', '٣', '٤', '٥', '٦', '٧', '٨', '٩'],
            Self::Farisi => ['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹'],
        }
    }

    /// The codepoint the digit zero maps to.
    #[must_use]
    pub const fn sifr(self) -> char {
        match self {
            Self::Latini => '0',
            Self::Arabi => '٠',
            Self::Farisi => '۰',
        }
    }

    /// Rewrites the ASCII digits in `nass` into this system, leaving every
    /// other character untouched.
    #[must_use]
    pub fn hawwil(self, nass: &str) -> String {
        if matches!(self, Self::Latini) {
            return nass.to_owned();
        }
        let asas = self.sifr() as u32;
        nass.chars()
            .map(|h| {
                if h.is_ascii_digit() {
                    // 0..=9 offset from an ASCII digit always lands inside the
                    // ten-codepoint digit block of every supported system.
                    char::from_u32(asas + (h as u32 - '0' as u32)).unwrap_or(h)
                } else {
                    h
                }
            })
            .collect()
    }
}
