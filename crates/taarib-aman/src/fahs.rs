//! The detection pipeline: the only place an install authorisation is minted.

use std::path::{Path, PathBuf};

use taarib_khatm::MirsatThiqa;
use taarib_mustalahat::bina::Basma;
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::ruqaa::RuqaaId;
use taarib_ruqaa::qari::MalafRuqaa;

use crate::idhn::IdhnTathbeet;
use crate::iqrar::{SijillIqrar, yahtaj_iqrar};
use crate::kashf_himaya::{HalatMatjar, IjmaaHimaya, ifhas_himaya_bi_matjar, mahmiya};
use crate::kashf_shabaka::{IjmaaShabaka, ifhas_shabaka, mutaaddid};
use crate::qaimat_sahb::QaimatSahb;
use crate::tahaqquq_tawqee::{SababTawqee, tahaqquq};

/// Why installation was refused. Every variant names its evidence.
#[derive(Debug)]
pub enum Rafd {
    /// The first-run statement has not been acknowledged.
    IqrarNaqis,
    /// The game runs anti-cheat. No override exists.
    Himaya(Box<IjmaaHimaya>),
    /// The anti-cheat check could not run, so its silence proves nothing.
    ///
    /// VAC is declared in Steam's catalogue and leaves nothing at all inside a
    /// game folder. A scan that never read the catalogue therefore produces
    /// exactly the report a genuinely clean game produces — no evidence, and on
    /// the install path no evidence is what mints the permit. A VAC ban is
    /// permanent and applies to the account rather than the game, so "the check
    /// did not run" is refused here rather than rounded down to "the check
    /// passed". No override exists, for the same reason [`Rafd::Himaya`] has
    /// none.
    FahsMatjarLamYajri {
        /// The `appinfo.vdf` that was tried, when a Steam root was known at
        /// all. [`None`] means none was: the game is a Steam game and this
        /// machine could not say where Steam is.
        masar: Option<PathBuf>,
        /// Why, as the same short label the scan's gap list carries.
        sabab: String,
    },
    /// The package signature was rejected.
    Tawqee(SababTawqee),
    /// The signing key, lineage or content hash is revoked.
    Mulgha {
        /// Why it was revoked, from the list.
        sabab: String,
    },
    /// The game is multiplayer and the user has not acknowledged the warning.
    ShabakaBilaIqrar(Box<IjmaaShabaka>),
}

impl Rafd {
    /// The sentence shown to the user, in Arabic.
    #[must_use]
    pub fn arabi(&self) -> String {
        match self {
            Self::IqrarNaqis => "لم يُقرَّ بيان تعريب بعد.".to_owned(),
            Self::Himaya(ijmaa) => {
                let mut nass = "تعمل هذه اللعبة بنظام مكافحة غش، ولا يُثبَّت تعريب فيها؛ قد \
                                 يُحظر حسابك حظرًا دائمًا. الدليل:"
                    .to_owned();
                for daleel in &ijmaa.adilla {
                    nass.push_str("\n- ");
                    nass.push_str(&daleel.arabi());
                }
                nass
            }
            Self::FahsMatjarLamYajri { masar, sabab } => {
                let mawdi = masar.as_ref().map_or_else(
                    || "لم يُعرف موضع تثبيت ستيم على هذا الجهاز".to_owned(),
                    |masar| format!("تعذّرت قراءة {} ({sabab})", masar.display()),
                );
                format!(
                    "لم يُستكمل فحص مكافحة الغش: حماية VAC لا تُعلَن إلا في فهرس متجر ستيم، ولا \
                     تترك أثرًا في مجلّد اللعبة، فسكوت الفحص هنا ليس براءة. {mawdi}. لا يُثبَّت \
                     شيء قبل قراءة الفهرس."
                )
            }
            Self::Tawqee(sabab) => sabab.arabi(),
            Self::Mulgha { sabab } => format!("أُبطلت هذه الحزمة أو مفتاحها: {sabab}"),
            Self::ShabakaBilaIqrar(_) => {
                "هذه لعبة متعدّدة اللاعبين؛ يلزم إقرارك بمخاطر التعديل قبل المتابعة.".to_owned()
            }
        }
    }

    /// The same, in English.
    #[must_use]
    pub fn injilizi(&self) -> String {
        match self {
            Self::IqrarNaqis => "the first-run statement has not been acknowledged".to_owned(),
            Self::Himaya(ijmaa) => {
                let mut nass =
                    "this game runs anti-cheat; installing can permanently ban your account. \
                     Evidence:"
                        .to_owned();
                for daleel in &ijmaa.adilla {
                    nass.push_str("\n- ");
                    nass.push_str(&daleel.injilizi());
                }
                nass
            }
            Self::FahsMatjarLamYajri { masar, sabab } => {
                let mawdi = masar.as_ref().map_or_else(
                    || "no Steam installation could be located on this machine".to_owned(),
                    |masar| format!("{} could not be read ({sabab})", masar.display()),
                );
                format!(
                    "the anti-cheat check did not finish: VAC is declared only in Steam's \
                     catalogue and leaves nothing in the game folder, so silence here is not a \
                     clean result. {mawdi}. Nothing is installed until the catalogue is read."
                )
            }
            Self::Tawqee(sabab) => sabab.injilizi(),
            Self::Mulgha { sabab } => format!("this package or its key was revoked: {sabab}"),
            Self::ShabakaBilaIqrar(_) => {
                "this is a multiplayer game; the modification risk must be acknowledged first"
                    .to_owned()
            }
        }
    }
}

/// The pipeline's verdict.
#[derive(Debug)]
pub enum NatijatFahs {
    /// Every check passed; the authorisation is minted.
    Masmuh(IdhnTathbeet),
    /// A check refused, with its reason.
    Marfud(Box<Rafd>),
}

/// Everything the pipeline needs to decide.
pub struct TalabFahs<'a> {
    /// The game.
    pub luba: LubaId,
    /// Its install root.
    pub jidhr_luba: &'a Path,
    /// Its Steam app id, when it has one.
    pub appid: Option<u32>,
    /// The Steam install root, for the `appinfo.vdf` reads.
    pub jidhr_steam: Option<&'a Path>,
    /// The quarantined, sealed package.
    pub malaf: &'a MalafRuqaa,
    /// The trust anchor compiled into this client.
    pub mirsa: &'a MirsatThiqa,
    /// The verified revocation list.
    pub qaima: &'a QaimatSahb,
    /// The first-run acknowledgement record, when one exists.
    pub iqrar: Option<&'a SijillIqrar>,
    /// Whether the user has acknowledged the multiplayer warning for this game.
    pub iqrar_shabaka: bool,
}

impl std::fmt::Debug for TalabFahs<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabFahs")
            .field("luba", &self.luba)
            .field("appid", &self.appid)
            .field("iqrar_shabaka", &self.iqrar_shabaka)
            .finish()
    }
}

/// Runs every refusal check and mints [`IdhnTathbeet`] only if all pass.
///
/// The order is by severity: an unacknowledged product statement, then
/// anti-cheat (an account is at stake and there is no override), then the
/// package signature, then revocation, then the per-game multiplayer warning.
/// The first refusal wins, so the user sees the most serious one.
///
/// Anti-cheat is two refusals, not one, and they are in this order for a
/// reason: a game with hard evidence is refused by *naming* the evidence, and
/// only a game with none reaches the question of whether the store catalogue
/// was ever read. On this path an empty evidence list is the input to a
/// decision, so it has to be an empty list that was actually looked for.
#[must_use]
pub fn fahs(talab: &TalabFahs<'_>) -> NatijatFahs {
    if yahtaj_iqrar(talab.iqrar) {
        return NatijatFahs::Marfud(Box::new(Rafd::IqrarNaqis));
    }

    let (himaya, halat_matjar) =
        ifhas_himaya_bi_matjar(talab.jidhr_luba, talab.appid, talab.jidhr_steam);
    if mahmiya(&himaya) {
        return NatijatFahs::Marfud(Box::new(Rafd::Himaya(Box::new(himaya))));
    }
    if let Some(rafd) = rafd_matjar(&halat_matjar) {
        return NatijatFahs::Marfud(Box::new(rafd));
    }

    let basmat = match tahaqquq(talab.malaf, talab.mirsa) {
        Ok(basmat) => basmat,
        Err(sabab) => return NatijatFahs::Marfud(Box::new(Rafd::Tawqee(sabab))),
    };

    if let Some(sabab) = mulgha(talab.malaf, talab.qaima, &basmat) {
        return NatijatFahs::Marfud(Box::new(Rafd::Mulgha { sabab }));
    }

    let shabaka = ifhas_shabaka(talab.jidhr_luba, talab.appid, talab.jidhr_steam);
    if mutaaddid(&shabaka) && !talab.iqrar_shabaka {
        return NatijatFahs::Marfud(Box::new(Rafd::ShabakaBilaIqrar(Box::new(shabaka))));
    }

    NatijatFahs::Masmuh(IdhnTathbeet::jadeed(talab.luba, basmat))
}

/// The refusal an unread store catalogue earns, or [`None`] when it was read or
/// never applied to this game at all.
///
/// The distinction this turns on is the one the product models everywhere else:
/// "we looked and there is nothing" against "nothing could be looked at". A game
/// with no Steam identity is the first — Steam has no opinion to withhold about
/// a GOG title, and refusing one would block a legitimate install over a
/// catalogue that was never relevant. A Steam game whose catalogue would not
/// open is the second, and the two produce identical evidence lists, which is
/// exactly why the caller is handed [`HalatMatjar`] rather than left to infer it
/// from a gap.
fn rafd_matjar(hala: &HalatMatjar) -> Option<Rafd> {
    match hala {
        HalatMatjar::GhayrMatlub | HalatMatjar::Maqru => None,
        HalatMatjar::JidhrMajhul => Some(Rafd::FahsMatjarLamYajri {
            masar: None,
            sabab: "no Steam root was given for a Steam game".to_owned(),
        }),
        HalatMatjar::Mutaadhdhir { masar, sabab } => Some(Rafd::FahsMatjarLamYajri {
            masar: Some(masar.clone()),
            sabab: sabab.clone(),
        }),
    }
}

/// The revocation reason for a package, checking key, lineage and content hash.
fn mulgha(malaf: &MalafRuqaa, qaima: &QaimatSahb, basmat: &Basma) -> Option<String> {
    let miftah_tawqee = malaf.ruqaa().ok()?.tawqee().miftah;
    match huwiyat_ruqaa(malaf) {
        Some(id) => qaima.fahs_ruqaa(id, basmat, &miftah_tawqee).map(str::to_owned),
        None => qaima
            .mulgha_miftah(&miftah_tawqee)
            .or_else(|| qaima.mulgha_basma(basmat))
            .map(str::to_owned),
    }
}

/// The package's lineage id from its metadata section, when it parses.
fn huwiyat_ruqaa(malaf: &MalafRuqaa) -> Option<RuqaaId> {
    let bayan = malaf.ruqaa().ok()?.bayan_json().ok()?;
    serde_json::from_value(bayan.get("id")?.clone()).ok()
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::fs;

    use crate::kashf_himaya::{ifhas_himaya, ifhas_himaya_bi_matjar};

    use super::*;

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// An application identifier that exists on Steam and is not in the
    /// fixtures below, so nothing here can match it by accident.
    const TATBEEQ: u32 = 220;

    /// A Steam root whose catalogue is valid, readable, and names no apps.
    ///
    /// Twelve bytes: the `0x07564427` magic, the universe, and the zero
    /// application id that terminates the entry list — which is exactly how a
    /// real `appinfo.vdf` opens and closes. This is the "we looked and there is
    /// nothing" fixture, and it has to stay distinguishable from a file that
    /// could not be opened all the way to the verdict.
    fn ansha_jidhr_steam(jidhr: &Path) -> Result<(), std::io::Error> {
        let appcache = jidhr.join("appcache");
        fs::create_dir_all(&appcache)?;
        let mut bayt: Vec<u8> = Vec::with_capacity(12);
        bayt.extend(0x0756_4427_u32.to_le_bytes());
        bayt.extend(1_u32.to_le_bytes());
        bayt.extend(0_u32.to_le_bytes());
        fs::write(appcache.join("appinfo.vdf"), bayt)
    }

    /// A game folder holding nothing any anti-cheat marker matches.
    fn ansha_luba(jidhr: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(jidhr)?;
        fs::write(jidhr.join("luba.txt"), b"no marker in this file matches anything")
    }

    #[test]
    fn luba_ghayr_steam_la_tastadi_fahrasan() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_luba = masrah.path().join("luba");
        ansha_luba(&jidhr_luba)?;

        // Most games in most libraries. Steam has no opinion it is withholding
        // about a GOG title, so refusing this would block a legitimate install
        // over a catalogue that never applied.
        let (ijmaa, hala) = ifhas_himaya_bi_matjar(&jidhr_luba, None, None);
        assert!(!mahmiya(&ijmaa));
        assert_eq!(hala, HalatMatjar::GhayrMatlub);
        assert!(!hala.lam_yuqra());
        assert!(rafd_matjar(&hala).is_none(), "a game with no Steam identity still installs");
        Ok(())
    }

    #[test]
    fn fahras_maqru_bila_dalil_yasmah() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_luba = masrah.path().join("luba");
        let jidhr_steam = masrah.path().join("Steam");
        ansha_luba(&jidhr_luba)?;
        ansha_jidhr_steam(&jidhr_steam)?;

        let (ijmaa, hala) = ifhas_himaya_bi_matjar(&jidhr_luba, Some(TATBEEQ), Some(&jidhr_steam));
        assert!(!mahmiya(&ijmaa));
        assert_eq!(hala, HalatMatjar::Maqru);
        assert!(ijmaa.thughrat.is_empty(), "nothing was out of reach");
        assert!(rafd_matjar(&hala).is_none(), "a catalogue that was read and said nothing passes");
        Ok(())
    }

    #[test]
    fn tajawuz_steam_matbu_yurfad_wa_la_yamnah_idhnan() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_luba = masrah.path().join("luba");
        let jidhr_steam = masrah.path().join("Steam");
        ansha_luba(&jidhr_luba)?;
        ansha_jidhr_steam(&jidhr_steam)?;

        // What a user's typo looks like by the time it reaches this layer:
        // `MatjarSteam::mawqi` returns the configured path whether or not it
        // exists, deliberately, so that the dispatcher cannot swallow it.
        let matbu = masrah.path().join("Steamm");
        let (ijmaa, hala) = ifhas_himaya_bi_matjar(&jidhr_luba, Some(TATBEEQ), Some(&matbu));

        // The evidence list is empty and byte-identical to the one the readable
        // catalogue above produced. That identity is the whole defect: the
        // verdict cannot be read off it, and `HalatMatjar` is what separates
        // the two.
        let (maqru, _) = ifhas_himaya_bi_matjar(&jidhr_luba, Some(TATBEEQ), Some(&jidhr_steam));
        assert_eq!(ijmaa.adilla, maqru.adilla);
        assert!(!mahmiya(&ijmaa));

        assert_eq!(
            hala,
            HalatMatjar::Mutaadhdhir {
                masar: matbu.join("appcache").join("appinfo.vdf"),
                sabab: "NotFound".to_owned(),
            }
        );
        assert!(hala.lam_yuqra());

        let rafd = rafd_matjar(&hala).ok_or("an unreadable catalogue must refuse, not degrade")?;
        // The refusal names the path, because on this machine the path is the
        // mistake.
        assert!(
            rafd.injilizi().contains(&matbu.display().to_string()),
            "the refusal must name the path the user typed: {}",
            rafd.injilizi()
        );
        assert!(!rafd.arabi().is_empty());
        Ok(())
    }

    #[test]
    fn luba_steam_bila_jidhr_turfad() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_luba = masrah.path().join("luba");
        ansha_luba(&jidhr_luba)?;

        // Steam uninstalled, or never found. The game is still a Steam game and
        // VAC is still declared in a catalogue nobody can open.
        let (ijmaa, hala) = ifhas_himaya_bi_matjar(&jidhr_luba, Some(TATBEEQ), None);
        assert!(!mahmiya(&ijmaa));
        assert_eq!(hala, HalatMatjar::JidhrMajhul);
        assert!(rafd_matjar(&hala).is_some(), "an unrunnable check is not a passed check");
        Ok(())
    }

    #[test]
    fn al_thughra_tazallu_ala_taqreer_al_qira() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        let jidhr_luba = masrah.path().join("luba");
        ansha_luba(&jidhr_luba)?;
        let matbu = masrah.path().join("Steamm");

        // The display path is unchanged: a diagnostics screen still shows the
        // same fact the install path now refuses over, and shows it as a gap
        // rather than as a clean bill of health.
        let ijmaa = ifhas_himaya(&jidhr_luba, Some(TATBEEQ), Some(&matbu));
        assert_eq!(
            ijmaa.thughrat.iter().map(|thughra| thughra.masar.clone()).collect::<Vec<_>>(),
            vec![matbu.join("appcache").join("appinfo.vdf")]
        );
        Ok(())
    }
}
