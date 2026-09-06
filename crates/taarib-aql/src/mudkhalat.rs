//! المدخلات — every producer's answer about one game, held side by side.

use std::path::{Path, PathBuf};

use serde::Serialize;
use taarib_aman::kashf_himaya::{HalatMatjar, IjmaaHimaya};
use taarib_aman::kashf_shabaka::IjmaaShabaka;
use taarib_aman::matjar::QiraatMatjar;
use taarib_kashf::fahs::{LubaMuktashafa, SimatLuba};
use taarib_muharrik::bitaqa::SababFahs;
use taarib_mustalahat::bina::MutabaqaBina;
use taarib_mustalahat::ghiyab::SababGhiyab;
use taarib_mustalahat::luba::{HukmLughaRasmiya, LubaId, MasdarLuba};
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustawda::mutabaqa::{MutabaqatLuba, MutabaqatRuqaa};
use taarib_tathbeet::bayan_makhzan::{BayanMukawwinat, iqra_bayan, kamil_hasab_bayan};
use taarib_tathbeet::wukala::WakeelQaim;
use taarib_usus::idadat::Idadat;

use crate::khatar::NawKhatar;

/// Which game this is, where it is, and what its launcher said about it.
///
/// [`Self::naw_ghayr_luba`] and [`Self::ailat_rum`] are deliberately the same
/// two pattern matches [`LubaMuktashafa::hiya_luba`] and
/// [`LubaMuktashafa::ailat_rum`] make. They are not delegated to it because the
/// discovery record is not what survives a scan — the store keeps the merged
/// game and its hints, not the per-launcher entry — so a core that delegated
/// would be a core that could only answer for a game discovered in this process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuwiyatLuba {
    /// Taarib's own identity for the game.
    pub id: LubaId,
    /// The name the launcher gives, verbatim.
    pub ism: String,
    /// The launcher identity it came from.
    pub masdar: MasdarLuba,
    /// The installation root.
    pub jidhr: PathBuf,
    /// The executable, when one is known.
    pub tanfidhi: Option<PathBuf>,
    /// Where Steam is installed on this machine, when it could be located.
    ///
    /// A machine fact rather than a game fact, and it is here because it is what
    /// the one catalogue read needs. Its absence for a Steam game is the reason
    /// the anti-cheat check can fail to run at all, which is a refusal and not a
    /// clean result — see [`taarib_aman::fahs::Rafd::FahsMatjarLamYajri`].
    pub jidhr_steam: Option<PathBuf>,
    /// What the launcher's own metadata says.
    pub simat: Vec<SimatLuba>,
    /// Whether the launcher reports the game as fully downloaded.
    pub muktamila: bool,
    /// A state the launcher is reporting about this game right now.
    pub hala_matjar: Option<SababGhiyab>,
}

impl HuwiyatLuba {
    /// Takes the identity straight off a discovery record.
    #[must_use]
    pub fn min_muktashafa(id: LubaId, luba: &LubaMuktashafa, jidhr_steam: Option<PathBuf>) -> Self {
        Self {
            id,
            ism: luba.ism.clone(),
            masdar: luba.masdar.clone(),
            jidhr: luba.jidhr.clone(),
            tanfidhi: luba.tanfidhi.clone(),
            jidhr_steam,
            simat: luba.simat.clone(),
            muktamila: luba.muktamila,
            hala_matjar: luba.hala_matjar.clone(),
        }
    }

    /// The Steam application identifier, when this game has one.
    ///
    /// Reads through a manager: a Steam game installed through Heroic or
    /// Playnite is still a Steam game, and its catalogue entry is still the one
    /// that declares VAC.
    #[must_use]
    pub fn appid_steam(&self) -> Option<u32> {
        match self.masdar.asl() {
            MasdarLuba::Steam(appid) => Some(*appid),
            _ => None,
        }
    }

    /// What the launcher says this entry is, when it says it is not a game.
    #[must_use]
    pub fn naw_ghayr_luba(&self) -> Option<&str> {
        self.simat.iter().find_map(|sima| match sima {
            SimatLuba::LaysatLuba(naw) => Some(naw.as_str()),
            _ => None,
        })
    }

    /// The console or emulator family, when this entry is a ROM rather than a
    /// program on disk.
    #[must_use]
    pub fn ailat_rum(&self) -> Option<&str> {
        self.simat.iter().find_map(|sima| match sima {
            SimatLuba::MuhakatRum(aila) => Some(aila.as_str()),
            _ => None,
        })
    }

    /// Whether the launcher lists online play for this game.
    #[must_use]
    pub fn jamai_online(&self) -> bool {
        self.simat
            .iter()
            .any(|sima| matches!(sima, SimatLuba::JamaiOnline))
    }

    /// The compatibility layer the launcher records, when it records one.
    #[must_use]
    pub fn tabaqat_tawafuq(&self) -> Option<&str> {
        self.simat.iter().find_map(|sima| match sima {
            SimatLuba::TabaqatTawafuq(wasf) => Some(wasf.as_str()),
            _ => None,
        })
    }
}

/// Whether this game has been examined, and by what.
///
/// The two states that used to be one answer. A game the probe examined and
/// could not recognise resolves to an unrecognised engine at tier three with a
/// full report behind it; a game nobody has examined has no report at all, and
/// the honest thing to say about it is that nothing is known yet. Both used to
/// arrive at a surface as "unknown engine, tier 3", and a user cannot act on the
/// first and can act on the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalatFahs {
    /// The stored capability report, when one exists.
    pub taqreer: Option<TaqreerImkaniyat>,
    /// Why a probe is due, or why one is not — the probe's own answer.
    pub sabab: SababFahs,
}

impl HalatFahs {
    /// A game that has been examined by this build.
    #[must_use]
    pub const fn mafhusa(taqreer: TaqreerImkaniyat) -> Self {
        Self {
            taqreer: Some(taqreer),
            sabab: SababFahs::LaHaja,
        }
    }

    /// A game nobody has examined.
    #[must_use]
    pub const fn ghayr_mafhusa() -> Self {
        Self {
            taqreer: None,
            sabab: SababFahs::AwwalMarra,
        }
    }

    /// Whether a report exists to answer from at all.
    #[must_use]
    pub const fn ladayha_taqreer(&self) -> bool {
        self.taqreer.is_some()
    }
}

/// The two hard scans and the one catalogue read behind them.
///
/// Held together because they are produced together: [`QiraatMatjar::iqra`]
/// opens `appinfo.vdf` once and both scans read their half out of the same
/// traversal. Splitting them here would invite a second read for the second
/// answer, which is the shape `taarib_aman::matjar` exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasahAman {
    /// The anti-cheat evidence, and the places the scan could not reach.
    pub himaya: IjmaaHimaya,
    /// What became of the store-catalogue half. Never inferred from an empty
    /// evidence list: an unread catalogue and a clean game produce the same list.
    pub halat_matjar: HalatMatjar,
    /// The multiplayer evidence.
    pub shabaka: IjmaaShabaka,
}

impl MasahAman {
    /// Runs both scans off one catalogue read.
    ///
    /// Never fails, for the reason neither scan does: a place that cannot be
    /// read becomes a gap on the report rather than an error a caller can
    /// discard with `.ok()`.
    #[must_use]
    pub fn ifhas(jidhr_luba: &Path, appid: Option<u32>, jidhr_steam: Option<&Path>) -> Self {
        let matjar = QiraatMatjar::iqra(appid, jidhr_steam);
        let (himaya, halat_matjar) =
            taarib_aman::kashf_himaya::ifhas_himaya_bi_qiraa(jidhr_luba, &matjar);
        let shabaka = taarib_aman::kashf_shabaka::ifhas_shabaka_bi_qiraa(jidhr_luba, &matjar);
        Self {
            himaya,
            halat_matjar,
            shabaka,
        }
    }

    /// The scan a game that was never walked produces: no evidence, and a
    /// catalogue that was never owed.
    #[must_use]
    pub fn faragh(jidhr_luba: &Path) -> Self {
        Self {
            himaya: IjmaaHimaya {
                jidhr: jidhr_luba.to_path_buf(),
                adilla: Vec::new(),
                thughrat: Vec::new(),
                mabtur: false,
            },
            halat_matjar: HalatMatjar::GhayrMatlub,
            shabaka: IjmaaShabaka {
                jidhr: jidhr_luba.to_path_buf(),
                dalail: Vec::new(),
                thughrat: Vec::new(),
                mabtur: false,
            },
        }
    }
}

/// What else already holds a loader slot in the game's own folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasahWukala {
    /// The directory that was surveyed.
    pub jidhr: PathBuf,
    /// Every occupied slot, as the survey named it.
    pub qaima: Vec<WakeelQaim>,
    /// Why the survey could not run, when it could not.
    pub thughra: Option<String>,
}

impl MasahWukala {
    /// Surveys one directory.
    ///
    /// A directory that does not exist holds no slots and is not a failure; a
    /// directory that exists and will not open is recorded as a gap, because an
    /// empty list from an unreadable directory is the same list a clean game
    /// produces.
    #[must_use]
    pub fn imsah(jidhr: &Path) -> Self {
        match taarib_tathbeet::wukala::masah(jidhr) {
            Ok(qaima) => Self {
                jidhr: jidhr.to_path_buf(),
                qaima,
                thughra: None,
            },
            Err(sabab) => Self {
                jidhr: jidhr.to_path_buf(),
                qaima: Vec::new(),
                thughra: Some(format!("{:?}", sabab.kind())),
            },
        }
    }

    /// The slots held by something that is not Taarib.
    pub fn ghurabaa(&self) -> impl Iterator<Item = &WakeelQaim> {
        self.qaima.iter().filter(|qaim| !qaim.huwiya.huwa_taarib())
    }
}

/// One component of Taarib's own store, and whether it is really there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MukawwinMakhzan {
    /// The component's path inside the store, `/`-separated.
    pub ism: String,
    /// Whether every file the staging manifest lists for it is present at its
    /// declared size.
    pub kamil: bool,
    /// Why not, in the installer's own words, when it is not.
    pub sabab: Option<String>,
}

/// What the component store on this machine actually holds.
///
/// Held rather than consulted for a readiness verdict. `imkaniyat::jahiziya`
/// deliberately does not read this directory, and it is right not to: a report
/// is persisted and re-probed only when the probe version rises, so a verdict
/// taken from a directory that can change underneath it would be cached and go
/// stale. What this is for is the other half of the same question — when a
/// surface asks *why* nothing reached the screen, "the component this build
/// ships is not in the store on this machine" is an answer only this input can
/// give, and it travels as evidence rather than as a verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HalatMakhzan {
    /// The store root.
    pub jidhr: PathBuf,
    /// The staging manifest, when the store carries one. [`None`] is a
    /// development build run from cargo, not a failure.
    pub bayan: Option<BayanMukawwinat>,
    /// Why the manifest could not be trusted, when it exists and could not.
    pub khata_bayan: Option<String>,
    /// The components that were asked about, in the order they were asked.
    pub mukawwinat: Vec<MukawwinMakhzan>,
}

impl HalatMakhzan {
    /// Reads the manifest and checks each named component against it.
    ///
    /// Never fails: a store that cannot answer says so in its own fields, so
    /// that "the store is incomplete" and "the store was never looked at" stay
    /// distinguishable all the way to a surface.
    #[must_use]
    pub fn ifhas(jidhr: &Path, asmaa: &[&str]) -> Self {
        let (bayan, khata_bayan) = match iqra_bayan(jidhr) {
            Ok(bayan) => (bayan, None),
            Err(khata) => (None, Some(khata.to_string())),
        };
        let mukawwinat = asmaa
            .iter()
            .map(|ism| match kamil_hasab_bayan(jidhr, ism) {
                Ok(()) => MukawwinMakhzan {
                    ism: (*ism).to_owned(),
                    kamil: true,
                    sabab: None,
                },
                Err(khata) => MukawwinMakhzan {
                    ism: (*ism).to_owned(),
                    kamil: false,
                    sabab: Some(khata.to_string()),
                },
            })
            .collect();
        Self {
            jidhr: jidhr.to_path_buf(),
            bayan,
            khata_bayan,
            mukawwinat,
        }
    }

    /// A store nobody asked about — the honest state before anything consults
    /// it.
    #[must_use]
    pub fn ghayr_mafhus(jidhr: &Path) -> Self {
        Self {
            jidhr: jidhr.to_path_buf(),
            bayan: None,
            khata_bayan: None,
            mukawwinat: Vec::new(),
        }
    }

    /// The components that were asked about and are not there in full.
    pub fn naqisa(&self) -> impl Iterator<Item = &MukawwinMakhzan> {
        self.mukawwinat.iter().filter(|mukawwin| !mukawwin.kamil)
    }
}

/// What the registry offers for this game, projected once.
///
/// A projection rather than the whole [`MutabaqatLuba`] because the answers
/// below need exactly these four facts, and because the projection is the thing
/// that was being made separately on every surface. It is made here, from the
/// matcher's own accessors, so there is one of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HalatMustawda {
    /// How many patches the registry lists for this game at all.
    pub adad_ruqaa: usize,
    /// How many voice packs it lists.
    pub adad_aswat: usize,
    /// The best tier any listed patch reaches against the installed build.
    pub afdal: Option<MutabaqaBina>,
    /// Whether the best installable patch needs an acknowledgement first —
    /// which is exactly [`MutabaqaBina::Nitaq`], an approximate match.
    pub yahtaj_iqrar: bool,
    /// Whether the installation fingerprinted fewer files than the best
    /// package's recipe names.
    pub naqis: bool,
}

impl HalatMustawda {
    /// Projects the matcher's verdict.
    #[must_use]
    pub fn min_mutabaqa(mutabaqa: &MutabaqatLuba) -> Self {
        let afdal = mutabaqa.afdal_ruqaa().ok();
        Self {
            adad_ruqaa: mutabaqa.ruqaa.len(),
            adad_aswat: mutabaqa.aswat.len(),
            afdal: mutabaqa.tabaqat_ruqaa(),
            yahtaj_iqrar: afdal.is_some_and(MutabaqatRuqaa::yahtaj_iqrar),
            naqis: afdal.is_some_and(MutabaqatRuqaa::naqis),
        }
    }

    /// The registry listing nothing for this game.
    #[must_use]
    pub const fn faragh() -> Self {
        Self {
            adad_ruqaa: 0,
            adad_aswat: 0,
            afdal: None,
            yahtaj_iqrar: false,
            naqis: false,
        }
    }
}

/// The settings this core reads, and the risks the user has already accepted.
///
/// Only the settings that change an answer here. The whole of [`Idadat`] is not
/// held, because a core that carried every preference would invite an answer to
/// depend on one, and the set of settings a verdict may turn on should be short
/// enough to read in one screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MawqifMustakhdim {
    /// Offer arabization for games whose publisher already ships Arabic.
    ///
    /// Off by default. It reveals the surface and does not change the verdict:
    /// with it on, the publisher's Arabic stops being a blocker and becomes a
    /// risk the user has already chosen to take.
    pub istibdal_lugha_rasmiya: bool,
    /// The risks acknowledged for this game.
    pub iqrarat: Vec<NawKhatar>,
}

impl MawqifMustakhdim {
    /// Takes the settings half off the stored settings, leaving the
    /// acknowledgements to the caller that collected them.
    #[must_use]
    pub const fn min_idadat(idadat: &Idadat, iqrarat: Vec<NawKhatar>) -> Self {
        Self {
            istibdal_lugha_rasmiya: idadat.istibdal_lugha_rasmiya,
            iqrarat,
        }
    }

    /// Whether one risk has been acknowledged.
    #[must_use]
    pub fn muqarr(&self, naw: NawKhatar) -> bool {
        self.iqrarat.contains(&naw)
    }
}

/// Everything the core is handed about one game.
///
/// Built by the caller that already holds these answers, or assembled by
/// [`Self::ijma`], which calls the producers that can be called cheaply and
/// read-only. Nothing here is derived: every field is one producer's answer,
/// kept as that producer worded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MudkhalatAql {
    /// Which game, where, and what its launcher said.
    pub huwiya: HuwiyatLuba,
    /// The capability report, and whether one exists at all.
    pub fahs: HalatFahs,
    /// The anti-cheat and multiplayer scans.
    pub aman: MasahAman,
    /// The official-Arabic verdict, when it has been computed.
    ///
    /// [`None`] is "nobody looked", which is not "there is no Arabic" — the
    /// distinction the verdict's own `hasim` exists for — and it is why an
    /// unchecked game is offered rather than refused.
    pub lugha: Option<HukmLughaRasmiya>,
    /// What else already holds a loader slot beside the game.
    pub wukala: MasahWukala,
    /// What Taarib's component store holds on this machine.
    pub makhzan: HalatMakhzan,
    /// The registry's answer, when it has been fetched.
    pub mustawda: Option<HalatMustawda>,
    /// The settings and acknowledgements.
    pub mawqif: MawqifMustakhdim,
}

impl MudkhalatAql {
    /// Runs the producers that are cheap, local and read-only, and takes the
    /// rest from the caller.
    ///
    /// The three that are not run here are the three that cannot be: the
    /// capability report is persisted and belongs to the probe's own version
    /// stamp, the official-Arabic verdict needs the deep container probes that
    /// live above this crate in the graph, and the registry's answer needs the
    /// network. Each arrives as a value.
    #[must_use]
    pub fn ijma(huwiya: HuwiyatLuba, fahs: HalatFahs, makhzan: &Path, mukawwinat: &[&str]) -> Self {
        let aman = MasahAman::ifhas(
            &huwiya.jidhr,
            huwiya.appid_steam(),
            huwiya.jidhr_steam.as_deref(),
        );
        let wukala = MasahWukala::imsah(&huwiya.jidhr);
        let makhzan = HalatMakhzan::ifhas(makhzan, mukawwinat);
        Self {
            huwiya,
            fahs,
            aman,
            lugha: None,
            wukala,
            makhzan,
            mustawda: None,
            mawqif: MawqifMustakhdim::default(),
        }
    }

    /// Attaches the official-Arabic verdict.
    #[must_use]
    pub fn bi_lugha(mut self, hukm: HukmLughaRasmiya) -> Self {
        self.lugha = Some(hukm);
        self
    }

    /// Attaches the registry's answer.
    #[must_use]
    pub const fn bi_mustawda(mut self, hala: HalatMustawda) -> Self {
        self.mustawda = Some(hala);
        self
    }

    /// Attaches the settings and the acknowledgements.
    #[must_use]
    pub fn bi_mawqif(mut self, mawqif: MawqifMustakhdim) -> Self {
        self.mawqif = mawqif;
        self
    }
}
