//! The ordered install pipeline: authorise, verify, match, back up, deploy, place, confirm.

use std::path::{Path, PathBuf};

use taarib_aman::IdhnTathbeet;
use taarib_muhawwil_nusus::tarkeeb::TaqreerTarkeeb;
use taarib_mustalahat::bina::{Basma, BinaId};
use taarib_ruqaa::qari::MalafRuqaa;
use taarib_ruqaa::tawqee::MudaqqiqTawqee;
use taarib_tarqee::irtibat::{IrtibatBina, SababMutabaqa};
use taarib_usus::manassa::{self, HalatTashghil};

use crate::bayan::{Muthabbit, NawTathbeet, TarifLuba, Tathbeet};
use crate::khata::{KhataTathbeet, NatijatTathbeet};
use crate::mawdi::WajhatLuba;
use crate::nusus;
use crate::tahaqquq::{NatijatTahaqquq, tahaqquq_kamil};

/// One patch-content placement: a validated in-game destination and its bytes.
#[derive(Debug)]
pub struct WadaMuhtawa {
    /// Where it goes, validated against the game root.
    pub wajha: WajhatLuba,
    /// What is written there.
    pub bayt: Vec<u8>,
}

/// The compatibility decision reached before any write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QararTawafuq {
    /// Exact build match.
    Tamma,
    /// Fingerprint match after the build id moved.
    BiBasma,
    /// Approximate, proceeding only on the user's acknowledgement.
    BiIqrar,
}

/// What the caller supplies to run one installation.
pub struct TalabTathbeet<'a> {
    /// The game, its launcher and its root.
    pub luba: &'a TarifLuba,
    /// The build currently installed.
    pub bina: &'a BinaId,
    /// The patch's binding, from the package.
    pub irtibat: &'a IrtibatBina,
    /// The patch content to place.
    pub muhtawa: Vec<WadaMuhtawa>,
    /// Whether the user acknowledged an approximate-match install.
    pub iqrar_taqribi: bool,
    /// The game executable's name, to refuse installing while it runs.
    pub tanfidhi: &'a str,
}

impl std::fmt::Debug for TalabTathbeet<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TalabTathbeet")
            .field("luba", &self.luba.ism)
            .field("bina", &self.bina)
            .field("adad_muhtawa", &self.muhtawa.len())
            .field("iqrar_taqribi", &self.iqrar_taqribi)
            .finish()
    }
}

/// What one installation did.
#[derive(Debug)]
pub struct NatijatTathbeetKamil {
    /// The compatibility decision acted on.
    pub tawafuq: QararTawafuq,
    /// How many content placements were written.
    pub adad_muhtawa: usize,
    /// The post-write verification verdict.
    pub tahaqquq: NatijatTahaqquq,
    /// What the script-engine write did, when the game is on one of the four
    /// engines Taarib patches as data.
    ///
    /// [`None`] for every other game, which is most of them: a Unity or Unreal
    /// install reads its translations out of the placed package at run time and
    /// has nothing written into its own files.
    pub nusus: Option<TaqreerTarkeeb>,
}

/// Runs the full install pipeline for one game and one package.
///
/// `nashr` deploys the framework and adapter through the same recording guard
/// every write uses; it runs after the manifest is durable and before content
/// is placed. Passing it as a step keeps the per-engine deployment table in
/// `tarkib` and the write ordering here.
///
/// The manifest is durable before `nashr` or any placement runs, because
/// [`Tathbeet::ibda`] flushes it and [`Tathbeet`] is the only writer either
/// step is given. A failure after the first write leaves that manifest on
/// disk; the caller rolls back rather than leaving a partial install.
///
/// # Errors
///
/// [`KhataTathbeet::IdhnGhayrMutabiq`] when the safety proof covers a different
/// game or package; [`KhataTathbeet::LubaTashtaghil`] when the game is running
/// and [`KhataTathbeet::HalatLubaMajhula`] when a sandbox makes that
/// unanswerable; [`KhataTathbeet::RuqaaMarfuda`] when the package fails
/// verification;
/// [`KhataTathbeet::TawafuqMarfud`] when the build does not match and no
/// acknowledgement was given; and whatever `nashr`, the manifest or the guard
/// raise.
pub fn thabbit<F>(
    talab: &TalabTathbeet<'_>,
    idhn: &IdhnTathbeet,
    ruqaa: &MalafRuqaa,
    mudaqqiq: &dyn MudaqqiqTawqee,
    jidhr_nusakh: &Path,
    huwiya: impl Into<String>,
    nashr: F,
) -> NatijatTathbeet<NatijatTathbeetKamil>
where
    F: FnOnce(&mut dyn Muthabbit) -> NatijatTathbeet<()>,
{
    let basmat = basmat_ruqaa(&talab.luba.jidhr, ruqaa)?;
    if !idhn.yushmal(talab.luba.luba, basmat) {
        return Err(KhataTathbeet::IdhnGhayrMutabiq);
    }

    la_tashtaghil(talab.tanfidhi)?;
    tahaqquq_ruqaa(&talab.luba.jidhr, ruqaa, mudaqqiq)?;
    let tawafuq = qarrir_tawafuq(talab)?;

    let mut tathbeet = Tathbeet::ibda(jidhr_nusakh, NawTathbeet::Nass, talab.luba, huwiya)?;
    // Before `nashr`, and RPG Maker is why: its extraction records carry byte
    // offsets into `js/plugins.js`, and deploying the adapter appends Taarib's
    // registration to that file. Splicing against offsets measured before the
    // append would be splicing against a file whose length has moved — the
    // adapter would notice and refuse, which is the safe failure and still a
    // wasted install. `taarib_muhawwil_nusus::rpgmaker::rakkib_mulhaq` documents
    // the same ordering for the same reason.
    let nusus = nusus::raqqi_nusus(
        &talab.luba.jidhr,
        ruqaa,
        nusus::makhzan_mukawwinat().as_deref(),
        &mut tathbeet,
    )?;
    nashr(&mut tathbeet)?;
    let adad_muhtawa = ida_muhtawa(&mut tathbeet, &talab.muhtawa)?;

    let taqreer = tahaqquq_kamil(&talab.luba.jidhr, jidhr_nusakh, NawTathbeet::Nass)?;
    Ok(NatijatTathbeetKamil { tawafuq, adad_muhtawa, tahaqquq: taqreer.natija(), nusus })
}

fn basmat_ruqaa(jidhr: &Path, ruqaa: &MalafRuqaa) -> NatijatTathbeet<Basma> {
    let mafateeh = ruqaa.ruqaa().map_err(|khata| khata_ruqaa(jidhr, &khata))?;
    Ok(Basma::min_bayt(mafateeh.tarwisa().basma))
}

/// Refuses when the game's own executable is running — or when that cannot be
/// established.
///
/// Both install and uninstall call this first, so a running game is named
/// before anything is touched rather than surfaced as a file lock partway
/// through. The restore path keeps its own reactive lock guard: a restore must
/// not depend on a process enumeration succeeding.
///
/// The three-state [`manassa::halat_tashghil`] is what is consulted, never the
/// raw process list. Inside a sandbox the list is the sandbox's own and comes
/// back empty for every process on the machine, so reading "no match" as "not
/// running" would silently disarm this guard exactly where it matters most —
/// a Steam Deck, where Steam and a game are the normal state of the machine.
///
/// # Errors
///
/// [`KhataTathbeet::LubaTashtaghil`] naming the process and its executable when
/// one was seen; [`KhataTathbeet::HalatLubaMajhula`] naming the sandbox when the
/// question could not be answered at all.
pub fn la_tashtaghil(tanfidhi: &str) -> NatijatTathbeet<()> {
    let ism = Path::new(tanfidhi).file_name().and_then(|s| s.to_str()).unwrap_or(tanfidhi);
    hukm_tashghil(manassa::halat_tashghil(ism), ism, tanfidhi)
}

/// Turns one running-state verdict into the refusal it warrants.
///
/// Split from [`la_tashtaghil`] so all three verdicts can be proved without a
/// process table to arrange — the one state that matters most is the one a test
/// machine cannot produce on demand.
fn hukm_tashghil(hala: HalatTashghil, ism: &str, tanfidhi: &str) -> NatijatTathbeet<()> {
    match hala {
        HalatTashghil::LaTashtaghil => Ok(()),
        // The enumeration is repeated only here, on the path that is already
        // aborting, because the verdict alone does not carry which process
        // matched and the message is worth a second pass. A process that exited
        // in between leaves the executable Taarib was asked about, which is
        // still true and still names the right thing to close.
        HalatTashghil::Tashtaghil => Err(match manassa::amaliyat_bism(ism).into_iter().next() {
            Some(amaliya) => KhataTathbeet::LubaTashtaghil {
                amaliya: amaliya.ism,
                tanfidhi: amaliya.masar.unwrap_or_else(|| PathBuf::from(tanfidhi)),
            },
            None => KhataTathbeet::LubaTashtaghil {
                amaliya: ism.to_owned(),
                tanfidhi: PathBuf::from(tanfidhi),
            },
        }),
        HalatTashghil::GhayrMaaruf { sunduq } => {
            Err(KhataTathbeet::HalatLubaMajhula { sunduq, tanfidhi: PathBuf::from(tanfidhi) })
        }
    }
}

fn tahaqquq_ruqaa(
    jidhr: &Path,
    ruqaa: &MalafRuqaa,
    mudaqqiq: &dyn MudaqqiqTawqee,
) -> NatijatTathbeet<()> {
    let mafateeh = ruqaa.ruqaa().map_err(|khata| khata_ruqaa(jidhr, &khata))?;
    let basma = mafateeh.tarwisa().basma;
    mafateeh.tawqee().tahaqquq(&basma, mudaqqiq).map_err(|khata| KhataTathbeet::RuqaaMarfuda {
        masar: jidhr.to_path_buf(),
        sabab: khata.to_string(),
    })
}

fn qarrir_tawafuq(talab: &TalabTathbeet<'_>) -> NatijatTathbeet<QararTawafuq> {
    let hukm = talab.irtibat.ihkum(talab.bina);
    let marfud = |yumkin_bi_iqrar: bool| KhataTathbeet::TawafuqMarfud {
        hukm: hukm.sabab.wasf_injilizi().to_owned(),
        yumkin_bi_iqrar,
    };
    match hukm.sabab {
        SababMutabaqa::MuarrifWaBasma => Ok(QararTawafuq::Tamma),
        SababMutabaqa::BasmaFaqat => Ok(QararTawafuq::BiBasma),
        SababMutabaqa::MuarrifBilaBasma | SababMutabaqa::DakhilNitaq => {
            if talab.iqrar_taqribi { Ok(QararTawafuq::BiIqrar) } else { Err(marfud(true)) }
        }
        SababMutabaqa::BilaTatabuq => Err(marfud(false)),
    }
}

fn ida_muhtawa(tathbeet: &mut Tathbeet, muhtawa: &[WadaMuhtawa]) -> NatijatTathbeet<usize> {
    let mut adad = 0_usize;
    for wada in muhtawa {
        let mutlaq = tathbeet.jidhr_luba().join(wada.wajha.nisbi());
        // Package content is an addition, never a replacement: the game shipped
        // none of it. So it goes through the path that records the directories
        // it creates along with the file. The replace path also marks an absent
        // file as added, which is why this looked right, but it records no
        // directory — and a restore that deletes `taarib/nusus.ruqaa` and
        // leaves `taarib/` standing has not given the game back as shipped.
        // The add path's refusal to write over something already there is the
        // right guard here too: a file at this path that no manifest knows is
        // not Taarib's to delete on uninstall, so it is not Taarib's to replace.
        tathbeet.ansha(&mutlaq, &wada.bayt)?;
        adad = adad.saturating_add(1);
    }
    Ok(adad)
}

fn khata_ruqaa(jidhr: &Path, khata: &taarib_ruqaa::khata::KhataRuqaa) -> KhataTathbeet {
    KhataTathbeet::RuqaaMarfuda { masar: jidhr.to_path_buf(), sabab: khata.to_string() }
}

#[cfg(test)]
#[allow(
    clippy::panic,
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "a test reports failure by panicking; the lints are written for library code, \
              and honouring them here would mean a test that cannot fail"
)]
mod ikhtibarat {
    use std::path::{Path, PathBuf};

    use taarib_usus::khata::{Khutwa, Tafsir as _};
    use taarib_usus::manassa::{RuyatAmaliyat, Sunduq, ruyat_amaliyat};

    use super::{HalatTashghil, KhataTathbeet, la_tashtaghil};
    use crate::masar_tathbeet::hukm_tashghil;

    /// A path whose file name no process on any machine carries.
    const TANFIDHI_MUSTAHIL: &str = "/taarib/la-yujad-hadha-al-tanfidhi-abadan";

    /// The file name inside [`TANFIDHI_MUSTAHIL`], which is what is looked up.
    const ISM_MUSTAHIL: &str = "la-yujad-hadha-al-tanfidhi-abadan";

    /// The child test the sandbox proof re-runs this binary to reach.
    #[cfg(target_os = "linux")]
    const ISM_IBN: &str = "masar_tathbeet::ikhtibarat::ibn_al_sunduq";

    #[test]
    fn la_tashtaghil_tuqbal_ghayr_al_mawjud_faqat() {
        let maftuh = hukm_tashghil(HalatTashghil::LaTashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL);
        assert!(maftuh.is_ok(), "a process seen absent is the one state that clears the guard");
        for sunduq in [Sunduq::Flatpak, Sunduq::Snap, Sunduq::Hawiya] {
            let hala = HalatTashghil::GhayrMaaruf { sunduq };
            assert!(
                hukm_tashghil(hala, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL).is_err(),
                "a question that could not be answered is not a no"
            );
        }
        assert!(hukm_tashghil(HalatTashghil::Tashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL).is_err());
    }

    #[test]
    fn al_rafd_yusammi_al_sunduq_wala_yaddai_al_tashghil() {
        let khata = hukm_tashghil(
            HalatTashghil::GhayrMaaruf { sunduq: Sunduq::Flatpak },
            ISM_MUSTAHIL,
            TANFIDHI_MUSTAHIL,
        )
        .expect_err("a blind guard refuses");
        match &khata {
            KhataTathbeet::HalatLubaMajhula { sunduq, tanfidhi } => {
                assert_eq!(*sunduq, Sunduq::Flatpak);
                assert_eq!(tanfidhi, Path::new(TANFIDHI_MUSTAHIL));
            }
            akhar => panic!("expected HalatLubaMajhula, got {akhar:?}"),
        }
        // The whole point of the variant: it names where it is, and it does not
        // assert the thing it could not observe.
        assert!(khata.injilizi().contains("Flatpak"));
        assert!(khata.injilizi().contains("cannot tell whether"));
        assert!(khata.arabi().contains("فلاتباك"));
        assert!(khata.arabi().contains("تعذّر عليه معرفة"));
        assert!(!khata.injilizi().contains("is running ("));
        // Retrying inside the same sandbox produces the same refusal forever,
        // so no retry is offered and no action is invented.
        assert!(!khata.qabil_lil_iada());
        assert_eq!(khata.khutwa(), Khutwa::LaShay);
        assert_eq!(khata.masar(), Some(Path::new(TANFIDHI_MUSTAHIL)));
    }

    #[test]
    fn al_tashghil_yasqut_ila_ma_sammahu_al_muttasil() {
        // No process carries this name, so the second enumeration finds nothing
        // and the refusal names the executable the caller asked about rather
        // than inventing a process that was never seen.
        let khata = hukm_tashghil(HalatTashghil::Tashtaghil, ISM_MUSTAHIL, TANFIDHI_MUSTAHIL)
            .expect_err("a running game refuses");
        match khata {
            KhataTathbeet::LubaTashtaghil { amaliya, tanfidhi } => {
                assert_eq!(amaliya, ISM_MUSTAHIL);
                assert_eq!(tanfidhi, PathBuf::from(TANFIDHI_MUSTAHIL));
            }
            akhar => panic!("expected LubaTashtaghil, got {akhar:?}"),
        }
    }

    #[test]
    fn al_hirasa_tatbaa_ruyat_hadhihi_al_ala() {
        // On an ordinary machine an absent process clears the guard, which is
        // the behaviour nothing here may change. On a machine that is itself
        // sandboxed the same call must refuse — that is not a reason to skip
        // the test, it is the other half of the proof.
        match ruyat_amaliyat() {
            RuyatAmaliyat::Kamila => {
                assert!(la_tashtaghil(TANFIDHI_MUSTAHIL).is_ok());
            }
            RuyatAmaliyat::Maazula { sunduq } => {
                let khata = la_tashtaghil(TANFIDHI_MUSTAHIL).expect_err("a blind guard refuses");
                assert!(matches!(
                    khata,
                    KhataTathbeet::HalatLubaMajhula { sunduq: mawjud, .. } if mawjud == sunduq
                ));
            }
        }
    }

    /// Drives the sandboxed branch for real, rather than by constructing the
    /// state by hand: a child of this same test binary carrying Flatpak's own
    /// marker, which is what `fi_sunduq` reads.
    #[cfg(target_os = "linux")]
    #[test]
    fn fi_sunduq_haqiqi_yarfud_al_tathbeet() {
        let exe = std::env::current_exe().expect("the test binary's own path");
        let natija = std::process::Command::new(exe)
            .args([ISM_IBN, "--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env("FLATPAK_ID", "org.taarib.Studio")
            .output()
            .expect("re-running this test binary");
        assert!(
            natija.status.success(),
            "the sandboxed child failed:\n{}\n{}",
            String::from_utf8_lossy(&natija.stdout),
            String::from_utf8_lossy(&natija.stderr)
        );
    }

    /// The child of [`fi_sunduq_haqiqi_yarfud_al_tathbeet`]. Ignored because it
    /// proves nothing without the marker its parent sets.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "needs the sandbox marker its parent process sets; run by that parent"]
    fn ibn_al_sunduq() {
        assert_eq!(
            taarib_usus::manassa::fi_sunduq(),
            Some(Sunduq::Flatpak),
            "the marker did not reach the child, so nothing below proves anything"
        );

        // The install guard. Before this change the same call returned Ok(()).
        let khata = la_tashtaghil(TANFIDHI_MUSTAHIL)
            .expect_err("a guard that cannot see the process table must refuse");
        match khata {
            KhataTathbeet::HalatLubaMajhula { sunduq, tanfidhi } => {
                assert_eq!(sunduq, Sunduq::Flatpak);
                assert_eq!(tanfidhi, PathBuf::from(TANFIDHI_MUSTAHIL));
            }
            akhar => panic!("expected HalatLubaMajhula, got {akhar:?}"),
        }

        // The launcher guard, through the same marker. The executable name is
        // one nothing can be running, so the only way to reach a refusal is the
        // blindness itself rather than a process that happened to be up.
        let malaf = Path::new("/taarib/localconfig.vdf");
        let khata = crate::itlaq::manassa_mughlaqa(&[ISM_MUSTAHIL], "Steam", malaf)
            .expect_err("a launcher guard that cannot see must refuse too");
        match khata {
            KhataTathbeet::HalatManassaMajhula { sunduq, manassa, malaf: mawdi } => {
                assert_eq!(sunduq, Sunduq::Flatpak);
                assert_eq!(manassa, "Steam");
                assert_eq!(mawdi, malaf);
            }
            akhar => panic!("expected HalatManassaMajhula, got {akhar:?}"),
        }
    }
}
