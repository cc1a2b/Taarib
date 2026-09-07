//! One-click install: quarantine, safety verdict, permit, install — in that order.

use std::path::Path;

use taarib_aman::fahs::{NatijatFahs, Rafd, TalabFahs, fahs};
use taarib_aman::iqrar::SijillIqrar;
use taarib_aman::qaimat_sahb::{QaimaMuraqaba, RafdQaima};
use taarib_aman::sandooq_fak::fak_ila_hajr;
use taarib_khatm::{MirsatThiqa, MudaqqiqEd25519};
use taarib_mustalahat::luba::LubaId;
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_tathbeet::bayan::TarifLuba;
use taarib_tathbeet::khata::KhataTathbeet;
use taarib_tathbeet::masar_tathbeet::{
    NatijatTathbeetKamil, TalabTathbeet, WadaMuhtawa, thabbit,
};
use taarib_tathbeet::nusus::Nashir;

use crate::khata::KhataMustawda;

/// Which stage the install reached, for the progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarhalatTathbeet {
    /// Placing the downloaded package in quarantine.
    Hajr,
    /// Running the safety checks.
    Fahs,
    /// Writing into the game.
    Kitaba,
    /// Finished.
    Tamma,
}

impl MarhalatTathbeet {
    /// The label shown beside the progress bar, in Arabic.
    #[must_use]
    pub const fn arabi(self) -> &'static str {
        match self {
            Self::Hajr => "عزل الحزمة والتحقّق من سلامتها",
            Self::Fahs => "فحص الأمان",
            Self::Kitaba => "التثبيت في اللعبة",
            Self::Tamma => "اكتمل",
        }
    }

    /// The same, in English.
    #[must_use]
    pub const fn injilizi(self) -> &'static str {
        match self {
            Self::Hajr => "quarantining and checking the package",
            Self::Fahs => "running safety checks",
            Self::Kitaba => "installing into the game",
            Self::Tamma => "done",
        }
    }
}

/// Why a one-click install did not complete.
#[derive(Debug)]
pub enum FashalTathbeet {
    /// Quarantine or registry-side failure.
    Mustawda(KhataMustawda),
    /// The safety layer refused, with its evidence.
    Aman(Box<Rafd>),
    /// The registry is answering and its revocation list is not.
    ///
    /// Raised before the package is even quarantined, because it is a fact
    /// about the registry rather than about the package: nothing is installed
    /// while the one document able to withdraw a patch is being withheld by a
    /// registry that can be reached. An unreachable registry never raises it.
    Sahb(RafdQaima),
    /// The installer refused or failed.
    Tathbeet(KhataTathbeet),
}

impl FashalTathbeet {
    /// The sentence shown to the user, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        use taarib_usus::khata::Tafsir as _;
        match self {
            Self::Mustawda(khata) => khata.arabi(),
            Self::Aman(rafd) => rafd.arabi(),
            Self::Sahb(rafd) => rafd.arabi(),
            Self::Tathbeet(khata) => khata.arabi(),
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        use taarib_usus::khata::Tafsir as _;
        match self {
            Self::Mustawda(khata) => khata.injilizi(),
            Self::Aman(rafd) => rafd.injilizi(),
            Self::Sahb(rafd) => rafd.injilizi(),
            Self::Tathbeet(khata) => khata.injilizi(),
        }
    }

    /// The concrete next step the user can take.
    #[must_use]
    pub fn khutwa(&self) -> Option<taarib_usus::khata::Khutwa> {
        use taarib_usus::khata::Tafsir as _;
        match self {
            Self::Mustawda(khata) => Some(khata.khutwa()),
            Self::Tathbeet(khata) => Some(khata.khutwa()),
            // A registry that is up and withholding its list is fixed by the
            // next successful refresh, which the next press triggers.
            Self::Sahb(_) => Some(taarib_usus::khata::Khutwa::AadaMuhawala),
            Self::Aman(_) => None,
        }
    }
}

/// Everything one install needs that is not produced inside it.
pub struct TalabNaqra<'a> {
    /// The game.
    pub luba: LubaId,
    /// Its launcher identity, root and patch lineage.
    pub tarif: &'a TarifLuba,
    /// Its Steam app id, when it has one.
    pub appid: Option<u32>,
    /// The Steam install root.
    pub jidhr_steam: Option<&'a Path>,
    /// The downloaded, hash-verified package file.
    pub malaf_munazzal: &'a Path,
    /// The quarantine directory.
    pub jidhr_hajr: &'a Path,
    /// The backup directory for this game.
    pub jidhr_nusakh: &'a Path,
    /// The trust anchor compiled into this client.
    pub mirsa: &'a MirsatThiqa,
    /// The signature-verified revocation list, beside where it stands.
    ///
    /// The pair rather than the list, so that no caller can hand this pipeline
    /// a list without having read the cache that says whether the registry
    /// ever confirmed it — the only way to reach a `QaimaMuraqaba` is through
    /// that read. Its refusal, when the state earns one, is answered here
    /// before anything else happens.
    pub qaima: &'a QaimaMuraqaba,
    /// The first-run acknowledgement record.
    pub iqrar: Option<&'a SijillIqrar>,
    /// Whether the multiplayer warning was acknowledged for this game.
    pub iqrar_shabaka: bool,
    /// Whether an approximate build match was acknowledged.
    pub iqrar_taqribi: bool,
    /// The game executable's name, to refuse installing while it runs.
    pub tanfidhi: &'a str,
    /// The patch content to place.
    pub muhtawa: Vec<WadaMuhtawa>,
    /// The build currently installed.
    pub bina: &'a taarib_mustalahat::bina::BinaId,
    /// The patch's binding.
    pub irtibat: &'a taarib_tarqee::irtibat::IrtibatBina,
    /// A label identifying this installation in its manifest.
    pub huwiya: String,
}

impl std::fmt::Debug for TalabNaqra<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabNaqra")
            .field("luba", &self.luba)
            .field("malaf_munazzal", &self.malaf_munazzal)
            .field("adad_muhtawa", &self.muhtawa.len())
            .finish()
    }
}

/// Installs a patch from a local file, downloaded or imported.
///
/// The order is forced: the package is quarantined and its framing checked,
/// then [`fahs`] decides, then its permit is moved into [`thabbit`]. There is
/// no argument that skips a stage and no branch that reaches `thabbit` without
/// a permit, because the permit is the only value that opens it and only
/// `fahs` mints one.
///
/// An imported file and a downloaded one take this same path: `malaf_munazzal`
/// pointing at a file on disk is the offline install, gated identically.
///
/// `nashr` is the deployment step, exactly as [`thabbit`] requires: it is handed
/// a [`Nashir`] and owns every write into the game's own files — the framework
/// and adapter through the recorder, and the script-engine write through
/// [`Nashir::raqqi`], under the plan it was given. A step that builds the plan
/// with `taarib_tathbeet::tarkib::khutta` and executes it with
/// `taarib_tathbeet::tarkib::nashr_bi_khutta` gets both without doing anything
/// else — and building it first is what lets the same plan be put in front of
/// the user before this function is called at all.
///
/// # Errors
///
/// [`FashalTathbeet::Sahb`] when the registry is reachable and withholding its
/// revocation list, before the package is touched;
/// [`FashalTathbeet::Mustawda`] when quarantine or the container refuses,
/// [`FashalTathbeet::Aman`] with the evidence when a safety check refuses, and
/// [`FashalTathbeet::Tathbeet`] when the installer refuses or fails.
pub fn thabbit_bilnaqra<F, P>(
    talab: TalabNaqra<'_>,
    nashr: F,
    mut taqaddum: P,
) -> Result<NatijatTathbeetKamil, FashalTathbeet>
where
    F: FnOnce(&mut Nashir<'_>) -> Result<(), KhataTathbeet>,
    P: FnMut(MarhalatTathbeet),
{
    // First and free: no file is opened to learn it, and a package that is
    // about to be refused over the registry's state should not be unpacked.
    if let Some(rafd) = talab.qaima.rafd() {
        return Err(FashalTathbeet::Sahb(rafd.clone()));
    }
    tracing::info!(
        hala = talab.qaima.hala().ism(),
        tasalsul = talab.qaima.qaima().tasalsul(),
        adad = talab.qaima.qaima().adad(),
        "{}",
        talab.qaima.wasf_injilizi()
    );

    taqaddum(MarhalatTathbeet::Hajr);
    let muhtawa_hajr = fak_ila_hajr(talab.malaf_munazzal, talab.jidhr_hajr)
        .map_err(|khata| FashalTathbeet::Mustawda(khata_hajr(&khata)))?;
    let masar_hajr = muhtawa_hajr.masar_masbur().ok_or_else(|| {
        FashalTathbeet::Mustawda(KhataMustawda::TanzeelFashil {
            rabt: talab.malaf_munazzal.display().to_string(),
            sabab: "quarantine produced no sealed package".to_owned(),
        })
    })?;
    let malaf = MalafRuqaa::iftah(masar_hajr).map_err(|khata| {
        FashalTathbeet::Mustawda(KhataMustawda::TanzeelFashil {
            rabt: masar_hajr.display().to_string(),
            sabab: khata.to_string(),
        })
    })?;

    taqaddum(MarhalatTathbeet::Fahs);
    let idhn = match fahs(&TalabFahs {
        luba: talab.luba,
        jidhr_luba: &talab.tarif.jidhr,
        appid: talab.appid,
        jidhr_steam: talab.jidhr_steam,
        malaf: &malaf,
        mirsa: talab.mirsa,
        qaima: talab.qaima.qaima(),
        iqrar: talab.iqrar,
        iqrar_shabaka: talab.iqrar_shabaka,
    }) {
        NatijatFahs::Masmuh(idhn) => idhn,
        NatijatFahs::Marfud(rafd) => return Err(FashalTathbeet::Aman(rafd)),
    };

    taqaddum(MarhalatTathbeet::Kitaba);
    let natija = thabbit(
        &TalabTathbeet {
            luba: talab.tarif,
            bina: talab.bina,
            irtibat: talab.irtibat,
            muhtawa: talab.muhtawa,
            iqrar_taqribi: talab.iqrar_taqribi,
            tanfidhi: talab.tanfidhi,
            // The same root the safety gate above was given. Resolving it twice
            // is how the gate and the launcher edit come to disagree about which
            // Steam this machine has.
            jidhr_steam: talab.jidhr_steam,
        },
        &idhn,
        &malaf,
        &MudaqqiqEd25519,
        talab.jidhr_nusakh,
        talab.huwiya,
        nashr,
    )
    .map_err(FashalTathbeet::Tathbeet)?;

    taqaddum(MarhalatTathbeet::Tamma);
    Ok(natija)
}

fn khata_hajr(khata: &taarib_aman::KhataAman) -> KhataMustawda {
    use taarib_usus::khata::Tafsir as _;
    KhataMustawda::TanzeelFashil {
        rabt: khata.masar().map_or_else(String::new, |m| m.display().to_string()),
        sabab: khata.injilizi(),
    }
}
