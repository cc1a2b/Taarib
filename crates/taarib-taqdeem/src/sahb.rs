//! Revocation: pulling a published patch, and the notice its installers get.

use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

use crate::hawiya::SalahiyatMalik;
use crate::khata::{KhataTaqdeem, NatijatTaqdeem};

/// Why a patch was pulled from circulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababSahb {
    /// It damages a game or does not install cleanly.
    Yudirr,
    /// It carries content its licence does not permit.
    IntihakRukhsa,
    /// It was published in error.
    Khata,
    /// Its signing key is no longer trusted.
    MiftahMasbub,
}

impl SababSahb {
    /// The sentence a user sees, in Arabic.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::Yudirr => "سُحبت هذه الرقعة لأنّها تُضرّ باللعبة أو لا تُثبَّت سليمة.",
            Self::IntihakRukhsa => "سُحبت هذه الرقعة لمخالفتها رخصة المحتوى.",
            Self::Khata => "سُحبت هذه الرقعة لأنّها نُشرت خطأً.",
            Self::MiftahMasbub => "سُحبت هذه الرقعة لأنّ مفتاح توقيعها لم يعد موثوقًا.",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::Yudirr => "it damages the game or does not install cleanly",
            Self::IntihakRukhsa => "it carries content its licence does not permit",
            Self::Khata => "it was published in error",
            Self::MiftahMasbub => "its signing key is no longer trusted",
        }
    }
}

/// One revocation, as the list and the listing both record it.
///
/// Constructed only by [`ishab`], which requires the owner authority.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QaydSahb {
    /// The patch lineage.
    pub ruqaa: RuqaaId,
    /// The revision pulled.
    pub murajaa: RuqaaRevision,
    /// Its content hash, so a client holding the file recognises it.
    pub basma: Basma,
    /// Why.
    pub sabab: SababSahb,
    /// The reviewer's own words, required.
    pub bayan: String,
    /// Who revoked it.
    pub murajii: MusahimId,
    /// When, RFC 3339, supplied by the caller.
    pub waqt: String,
}

/// Pulls a published patch from circulation.
///
/// Requires the owner authority: there is no other caller, because there is no
/// other way to obtain a [`SalahiyatMalik`].
///
/// # Errors
///
/// [`KhataTaqdeem::RafdBilaSabab`] when `bayan` is blank — a revocation with no
/// stated reason is a patch vanishing from a user's library with no
/// explanation.
pub fn ishab(
    _salahiya: &SalahiyatMalik,
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    basma: Basma,
    sabab: SababSahb,
    bayan: String,
    murajii: MusahimId,
    waqt: String,
) -> NatijatTaqdeem<QaydSahb> {
    if bayan.trim().is_empty() {
        return Err(KhataTaqdeem::RafdBilaSabab);
    }
    Ok(QaydSahb {
        ruqaa,
        murajaa,
        basma,
        sabab,
        bayan,
        murajii,
        waqt,
    })
}

/// What a user who installed a revoked patch is told on next launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IshaarSahb {
    /// The revocation.
    pub qayd: QaydSahb,
    /// Whether the patch is still installed on this machine.
    pub muthabbata: bool,
}

impl IshaarSahb {
    /// The notice text, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        let mut nass = self.qayd.sabab.arabi().to_owned();
        nass.push(' ');
        nass.push_str(&self.qayd.bayan);
        if self.muthabbata {
            nass.push_str(" يمكنك إزالتها وإرجاع لعبتك إلى حالتها الأصلية بنقرة واحدة.");
        }
        nass
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        let mut nass = format!("This patch was withdrawn: {}", self.qayd.sabab.injilizi());
        nass.push_str(". ");
        nass.push_str(&self.qayd.bayan);
        if self.muthabbata {
            nass.push_str(" You can remove it and restore your game in one click.");
        }
        nass
    }

    /// Whether a clean uninstall should be offered.
    #[must_use]
    pub const fn yuarad_ilgha(&self) -> bool {
        self.muthabbata
    }
}
