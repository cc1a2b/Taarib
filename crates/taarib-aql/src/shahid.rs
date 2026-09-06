//! الشاهد — the input behind an answer, so that a wrong answer names a wrong input.

use serde::Serialize;

/// Which of the held inputs an observation came from.
///
/// One variant per producer this crate composes, and no variant for anything it
/// derives itself. That is the point: an answer that cannot name one of these is
/// an answer this crate invented, and there is nowhere for it to hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MasdarMarifa {
    /// The launcher's own record of the game — `taarib_kashf::LubaMuktashafa`
    /// and the hints on it.
    Iktishaf,
    /// The capability report as a whole — `taarib_muharrik::imkaniyat::taqreer`.
    Imkaniyat,
    /// One observation inside the engine probe's own evidence trail
    /// (`taarib_mustalahat::muharrik::Daleel`).
    DaleelMuharrik,
    /// The anti-cheat scan — `taarib_aman::kashf_himaya`.
    KashfHimaya,
    /// What became of the store catalogue read — `taarib_aman::matjar`.
    FahrasMatjar,
    /// The multiplayer scan — `taarib_aman::kashf_shabaka`.
    KashfShabaka,
    /// The official-Arabic verdict — `taarib_kashf::lugha_rasmiya`.
    LughaRasmiya,
    /// The proxy and mod survey — `taarib_tathbeet::wukala`.
    MasahWukala,
    /// Taarib's own component store — `taarib_tathbeet::bayan_makhzan`.
    MakhzanMukawwinat,
    /// The registry's build-matching verdict — `taarib_mustawda::mutabaqa`.
    Mustawda,
    /// The user's settings and the acknowledgements they have given.
    Idadat,
    /// The stored probe record, which is what says whether this game was ever
    /// examined — `taarib_muharrik::bitaqa`.
    Bitaqa,
}

impl MasdarMarifa {
    /// A stable machine name, for logs and for the diagnostics bundle.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Iktishaf => "iktishaf",
            Self::Imkaniyat => "imkaniyat",
            Self::DaleelMuharrik => "daleel_muharrik",
            Self::KashfHimaya => "kashf_himaya",
            Self::FahrasMatjar => "fahras_matjar",
            Self::KashfShabaka => "kashf_shabaka",
            Self::LughaRasmiya => "lugha_rasmiya",
            Self::MasahWukala => "masah_wukala",
            Self::MakhzanMukawwinat => "makhzan_mukawwinat",
            Self::Mustawda => "mustawda",
            Self::Idadat => "idadat",
            Self::Bitaqa => "bitaqa",
        }
    }

    /// The producer's name, as a maintainer reading a diagnostics bundle would
    /// look for it.
    #[must_use]
    pub const fn muntij(self) -> &'static str {
        match self {
            Self::Iktishaf => "taarib_kashf::fahs",
            Self::Imkaniyat => "taarib_muharrik::imkaniyat",
            Self::DaleelMuharrik => "taarib_muharrik::dalail",
            Self::KashfHimaya => "taarib_aman::kashf_himaya",
            Self::FahrasMatjar => "taarib_aman::matjar",
            Self::KashfShabaka => "taarib_aman::kashf_shabaka",
            Self::LughaRasmiya => "taarib_kashf::lugha_rasmiya",
            Self::MasahWukala => "taarib_tathbeet::wukala",
            Self::MakhzanMukawwinat => "taarib_tathbeet::bayan_makhzan",
            Self::Mustawda => "taarib_mustawda::mutabaqa",
            Self::Idadat => "taarib_usus::idadat",
            Self::Bitaqa => "taarib_muharrik::bitaqa",
        }
    }
}

/// One input, named, that an answer rests on.
///
/// Diagnostic rather than user-facing, and deliberately so. The sentences a
/// player reads are the producers' own — they travel on [`crate::mani::Mani`],
/// [`crate::khatar::Khatar`] and [`crate::muntaj::Waad`] in both languages.
/// This carries the producer's words in whatever language the producer wrote
/// them, because the question it answers is "why does this game say that", and
/// translating an observation would put a sentence in the trail that no producer
/// ever wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Shahid {
    /// Which held input it came from.
    pub masdar: MasdarMarifa,
    /// What that input said, in the producer's own words.
    pub wasf: String,
    /// Where it was seen, when the input names a place.
    pub mawqi: Option<String>,
}

impl Shahid {
    /// An observation with no place attached.
    #[must_use]
    pub fn jadeed(masdar: MasdarMarifa, wasf: impl Into<String>) -> Self {
        Self {
            masdar,
            wasf: wasf.into(),
            mawqi: None,
        }
    }

    /// An observation that names where it was seen.
    #[must_use]
    pub fn fi(masdar: MasdarMarifa, wasf: impl Into<String>, mawqi: impl Into<String>) -> Self {
        Self {
            masdar,
            wasf: wasf.into(),
            mawqi: Some(mawqi.into()),
        }
    }
}

impl std::fmt::Display for Shahid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.mawqi {
            Some(mawqi) => write!(f, "{}: {} ({mawqi})", self.masdar.ism(), self.wasf),
            None => write!(f, "{}: {}", self.masdar.ism(), self.wasf),
        }
    }
}

/// A value together with every input that produced it.
///
/// The shape every answer in this crate comes back in. `مُسنَد` is the word for a
/// report carried with its chain of transmission, which is exactly the property
/// asked of this crate: a verdict that cannot name what produced it is a cache,
/// and a verdict that can is traceable to a wrong *input* rather than to a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Musnad<T> {
    /// The answer.
    pub qeema: T,
    /// Every input it rests on, in the order they were consulted.
    pub shawahid: Vec<Shahid>,
}

impl<T> Musnad<T> {
    /// Binds an answer to the inputs behind it.
    #[must_use]
    pub const fn jadeed(qeema: T, shawahid: Vec<Shahid>) -> Self {
        Self { qeema, shawahid }
    }

    /// Whether any held input at all stands behind this answer.
    ///
    /// `false` is not a failure — "nobody looked" is a legitimate answer — but it
    /// is the one state a caller must never present as a finding.
    #[must_use]
    pub const fn musnad(&self) -> bool {
        !self.shawahid.is_empty()
    }

    /// Whether a named producer is among the inputs behind this answer.
    #[must_use]
    pub fn min(&self, masdar: MasdarMarifa) -> bool {
        self.shawahid.iter().any(|shahid| shahid.masdar == masdar)
    }

    /// Rewrites the answer, keeping the same chain.
    pub fn hawwil<U>(self, mubaddil: impl FnOnce(T) -> U) -> Musnad<U> {
        Musnad {
            qeema: mubaddil(self.qeema),
            shawahid: self.shawahid,
        }
    }
}
