//! The revocation refresh against directory sources: served, withheld, replayed, forged, offline.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

use jiff::{SignedDuration, Timestamp};
use taarib_aman::qaimat_sahb::{
    HalatQaima, KatibQaima, NatijatMuhawala, QaimaMuraqaba, QaimatSahb, sijill_tajdid,
};
use taarib_khatm::MiftahKhass;
use taarib_mustalahat::bina::{Basma, BinaId};
use taarib_mustalahat::luba::{LubaId, MasdarLuba};
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};
use taarib_mustawda::fahras::{BayanMustawda, ISDAR_BAYAN};
use taarib_mustawda::masadir::{MASAR_BAYAN, MasdarMustawda, SilsilatMasadir};
use taarib_mustawda::sahb::{NatijatTajdid, jaddid_qaimat_sahb};
use taarib_mustawda::tathbeet_bilnaqra::{FashalTathbeet, TalabNaqra, thabbit_bilnaqra};
use taarib_tarqee::irtibat::{IrtibatBina, MukhattatBasma};
use taarib_tathbeet::bayan::TarifLuba;
use taarib_tathbeet::nusus::Nashir;
use taarib_usus::masarat::Masarat;

/// Every test returns this so that a fixture failure propagates with `?`.
/// `unwrap` and `expect` are denied workspace-wide, tests included.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// Where the caster puts the list inside a repository.
const MASAR_QAIMA: &str = "sahb/qaima.json";

const USDIRAT: &str = "2026-09-01T00:00:00Z";
const SALIH_HATTA: &str = "2027-09-01T00:00:00Z";

/// The refresh window every gate here is read under.
const NAFIDHA: SignedDuration = SignedDuration::from_hours(3);

/// The signing key a compromised contributor used.
const MIFTAH_MULGHA: [u8; 32] = [1u8; 32];

fn malik() -> MiftahKhass {
    MiftahKhass::min_bayt(&[7u8; 32])
}

fn al_aan() -> Result<Timestamp, Box<dyn Error>> {
    Ok("2026-09-06T12:00:00Z".parse::<Timestamp>()?)
}

/// A data root of its own for every test, so nothing shares a cache.
fn masarat(masrah: &tempfile::TempDir) -> Masarat {
    Masarat::min_judhur(masrah.path().join("bayanat"), masrah.path().join("idadat"))
}

/// The empty list at sequence 1 a build ships.
fn asliya() -> Result<QaimatSahb, Box<dyn Error>> {
    let bayt = KatibQaima::jadeed(1, USDIRAT, SALIH_HATTA).uktub(&malik())?;
    Ok(QaimatSahb::min_bayt(&bayt, &malik().aam())?)
}

/// A signed list at `tasalsul` revoking [`MIFTAH_MULGHA`].
fn qaima(tasalsul: u64, salih_hatta: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(KatibQaima::jadeed(tasalsul, USDIRAT, salih_hatta)
        .ilgha_miftah(MIFTAH_MULGHA, "the signing key was compromised", USDIRAT)
        .uktub(&malik())?)
}

/// A repository directory: a manifest naming `rabt`, and the list when given.
fn mustawda(jidhr: &Path, rabt: &str, qaima: Option<&[u8]>) -> NatijatIkhtibar {
    let bayan = BayanMustawda {
        isdar: ISDAR_BAYAN,
        tasalsul: 1,
        waqt: USDIRAT.to_owned(),
        sharaih: BTreeMap::new(),
        rabt_qaimat_sahb: rabt.to_owned(),
        tajawuzat: Vec::new(),
    };
    std::fs::create_dir_all(jidhr)?;
    std::fs::write(jidhr.join(MASAR_BAYAN), serde_json::to_vec(&bayan)?)?;
    if let Some(bayt) = qaima {
        let masar = jidhr.join(MASAR_QAIMA);
        if let Some(walid) = masar.parent() {
            std::fs::create_dir_all(walid)?;
        }
        std::fs::write(masar, bayt)?;
    }
    Ok(())
}

fn silsila(jidhr: PathBuf) -> SilsilatMasadir {
    SilsilatMasadir::min_masdar(MasdarMustawda::MujalladMahalli { jidhr })
}

/// The gate's read of the cache, as every install path makes it.
fn iqra(masarat: &Masarat, al_aan: Timestamp) -> Result<QaimaMuraqaba, Box<dyn Error>> {
    Ok(QaimaMuraqaba::iqra(masarat, &malik().aam(), asliya()?, al_aan, NAFIDHA))
}

#[tokio::test]
async fn al_tajdid_min_mujallad_mahalli_yanjah_wa_yukhazzan() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, MASAR_QAIMA, Some(&qaima(2, SALIH_HATTA)?))?;
    let waqt = al_aan()?;

    let natija =
        jaddid_qaimat_sahb(&silsila(jidhr), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(natija.najah(), "{}", natija.wasf());

    let muraqaba = iqra(&masarat, waqt.checked_add(SignedDuration::from_secs(60))?)?;
    assert!(matches!(muraqaba.hala(), HalatQaima::Muhaddatha { .. }), "{:?}", muraqaba.hala());
    assert_eq!(muraqaba.qaima().tasalsul(), 2);
    assert!(
        muraqaba.qaima().mulgha_miftah(&MIFTAH_MULGHA).is_some(),
        "the revocation the registry published reached the gate"
    );
    assert!(muraqaba.rafd().is_none());

    let sijill = sijill_tajdid(&masarat);
    let najah = sijill.akhir_najah.ok_or("no success was recorded")?;
    assert_eq!(najah.tasalsul, 2);
    assert!(najah.masdar.contains("bundled mirror"), "{}", najah.masdar);
    Ok(())
}

#[tokio::test]
async fn mustawda_yujib_bila_qaima_yurfad_ala_al_bawwaba() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, MASAR_QAIMA, None)?;
    let waqt = al_aan()?;

    let natija =
        jaddid_qaimat_sahb(&silsila(jidhr), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(matches!(natija, NatijatTajdid::QaimaMutaadhdhira { .. }), "{}", natija.wasf());

    let muraqaba = iqra(&masarat, waqt.checked_add(SignedDuration::from_secs(60))?)?;
    let rafd = muraqaba.rafd().ok_or("a reachable registry without a list must refuse")?;
    assert!(rafd.masdar.contains("bundled mirror"), "{}", rafd.masdar);
    assert!(matches!(muraqaba.hala(), HalatQaima::LamTujlab { akhir: Some(_) }));

    // The one-click pipeline answers it before it opens the package: the
    // package path below does not exist, and a pipeline that reached
    // quarantine would fail on that instead.
    let luba = LubaId::min_masdar(&MasdarLuba::Steam(480), "Luba Ikhtibar");
    let tarif = TarifLuba {
        luba,
        masdar: MasdarLuba::Steam(480),
        ism: "Luba Ikhtibar".to_owned(),
        jidhr: masrah.path().join("luba"),
        ruqaa: RuqaaId::jadeeda(),
        murajaa: RuqaaRevision::AWWAL,
        basma_bina: None,
    };
    let bina = BinaId {
        manassa: None,
        basma: Basma::min_bayt([0u8; 32]),
        adad_malaffat: 0,
        waqt: USDIRAT.to_owned(),
    };
    // Never reached: the gate refuses above this, which is the point of the
    // assertion below. It only has to be a well-formed binding.
    let irtibat = IrtibatBina {
        manassat: vec!["1".to_owned()],
        basmat: vec![Basma::min_bayt([0u8; 32])],
        adad_malaffat: 1,
        mukhattat: MukhattatBasma::min_masarat(["data.txt"])?,
        nitaq: None,
        khutut: Vec::new(),
    };
    let talab = TalabNaqra {
        luba,
        tarif: &tarif,
        appid: None,
        jidhr_steam: None,
        malaf_munazzal: &masrah.path().join("ghayr-mawjud.ruqaa"),
        jidhr_hajr: &masrah.path().join("hajr"),
        jidhr_nusakh: &masrah.path().join("nusakh"),
        mirsa: &taarib_khatm::MIRSAT_MALIK,
        qaima: &muraqaba,
        iqrar: None,
        iqrar_shabaka: false,
        iqrar_taqribi: false,
        tanfidhi: "luba.exe",
        muhtawa: Vec::new(),
        bina: &bina,
        irtibat: &irtibat,
        huwiya: "ikhtibar".to_owned(),
    };
    let fashal = thabbit_bilnaqra(
        talab,
        |_muthabbit: &mut Nashir<'_>| Ok(()),
        |_marhala| {},
    )
    .err()
    .ok_or("the pipeline must not install while the registry withholds its list")?;
    assert!(matches!(fashal, FashalTathbeet::Sahb(_)), "{}", fashal.injilizi());
    assert!(fashal.injilizi().contains("did not serve its revocation list"), "{}", fashal.injilizi());
    assert!(fashal.arabi().contains("قائمة الإبطال"), "{}", fashal.arabi());
    Ok(())
}

#[tokio::test]
async fn mustawda_ghayr_mutah_la_yamnaa_wa_la_yaddai_al_fahs() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let waqt = al_aan()?;
    // A share that is not mounted, which is what an offline machine with a
    // network source configured also looks like to the chain: nothing answers.
    let ghaib = masrah.path().join("ghayr-mawjud");

    let natija =
        jaddid_qaimat_sahb(&silsila(ghaib), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(matches!(natija, NatijatTajdid::MustawdaGhayrMutah { .. }), "{}", natija.wasf());

    let muraqaba = iqra(&masarat, waqt)?;
    assert!(muraqaba.rafd().is_none(), "offline installs; it is never refused for being offline");
    assert!(matches!(muraqaba.hala(), HalatQaima::LamTujlab { akhir: Some(_) }));
    assert!(!muraqaba.hala().muhaddatha(), "and it is never told the check passed");
    assert_eq!(muraqaba.qaima().tasalsul(), 1, "the compiled-in list is the floor");
    assert!(muraqaba.wasf_injilizi().contains("could not reach the registry"));
    Ok(())
}

#[tokio::test]
async fn qaima_aqdam_min_al_mahfudha_turfad_wa_tubqa_al_mahfudha() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let waqt = al_aan()?;

    let hadith = masrah.path().join("hadith");
    mustawda(&hadith, MASAR_QAIMA, Some(&qaima(3, SALIH_HATTA)?))?;
    let natija =
        jaddid_qaimat_sahb(&silsila(hadith), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(natija.najah(), "{}", natija.wasf());

    // A lagging mirror, or a replay: sequence 2 after sequence 3 was cached.
    let qadeem = masrah.path().join("qadeem");
    let bila_ilgha = KatibQaima::jadeed(2, USDIRAT, SALIH_HATTA).uktub(&malik())?;
    mustawda(&qadeem, MASAR_QAIMA, Some(&bila_ilgha))?;
    let baad = waqt.checked_add(SignedDuration::from_secs(60))?;
    let natija =
        jaddid_qaimat_sahb(&silsila(qadeem), &masarat, &malik().aam(), &asliya()?, baad).await;
    assert!(
        matches!(natija, NatijatTajdid::Aqdam { jadeeda: 2, asas: 3, .. }),
        "{}",
        natija.wasf()
    );

    let muraqaba = iqra(&masarat, baad)?;
    assert_eq!(muraqaba.qaima().tasalsul(), 3, "the replay must not un-revoke anything");
    assert!(muraqaba.qaima().mulgha_miftah(&MIFTAH_MULGHA).is_some());
    assert!(muraqaba.rafd().is_none());
    // Not "refreshed": the latest thing the registry said was an older list.
    assert!(
        matches!(
            muraqaba.hala(),
            HalatQaima::Mukhazzana { akhir: Some(akhir), .. }
                if matches!(akhir.natija, NatijatMuhawala::Aqdam { .. })
        ),
        "{:?}",
        muraqaba.hala()
    );
    Ok(())
}

#[tokio::test]
async fn qaima_bi_tawqee_talif_turfad_wa_la_tukhazzan() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let waqt = al_aan()?;

    let salim = String::from_utf8(qaima(2, SALIH_HATTA)?)?;
    let mutalaab = salim.replace("was compromised", "is perfectly fine");
    assert_ne!(mutalaab, salim);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, MASAR_QAIMA, Some(mutalaab.as_bytes()))?;

    let natija =
        jaddid_qaimat_sahb(&silsila(jidhr), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(matches!(natija, NatijatTajdid::QaimaMutaadhdhira { .. }), "{}", natija.wasf());
    assert!(
        QaimatSahb::min_makhbaa(&masarat, &malik().aam())?.is_none(),
        "forged bytes never reach the cache"
    );

    // A registry that answers with a forged list is a registry that answered
    // without one.
    let muraqaba = iqra(&masarat, waqt.checked_add(SignedDuration::from_secs(60))?)?;
    assert!(muraqaba.rafd().is_some());
    Ok(())
}

#[tokio::test]
async fn qaima_muntahiya_tuqbal_wa_tuqal_bila_rafd() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let waqt = al_aan()?;
    let jidhr = masrah.path().join("mustawda");
    // Signed by the owner, and past its own validity: a registry whose owner
    // forgot to re-sign. It is accepted — the signature is real — and the gate
    // says out loud that it is stale.
    mustawda(&jidhr, MASAR_QAIMA, Some(&qaima(2, "2026-01-01T00:00:00Z")?))?;

    let natija =
        jaddid_qaimat_sahb(&silsila(jidhr), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(natija.najah(), "{}", natija.wasf());

    let muraqaba = iqra(&masarat, waqt)?;
    assert!(matches!(muraqaba.hala(), HalatQaima::Muntahiya { julibat: Some(_), .. }));
    assert!(muraqaba.rafd().is_none());
    assert!(!muraqaba.hala().muhaddatha());
    assert!(muraqaba.qaima().mulgha_miftah(&MIFTAH_MULGHA).is_some());
    Ok(())
}

#[tokio::test]
async fn bayan_talif_aw_rabt_kharij_yuad_qaima_mutaadhdhira() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let waqt = al_aan()?;

    // A manifest aiming the fetch outside the repository.
    let kharij = masrah.path().join("kharij");
    mustawda(&kharij, "https://example.invalid/qaima.json", Some(&qaima(2, SALIH_HATTA)?))?;
    let natija =
        jaddid_qaimat_sahb(&silsila(kharij), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(matches!(natija, NatijatTajdid::QaimaMutaadhdhira { .. }), "{}", natija.wasf());

    // A source that answers the manifest with something that is not one.
    let talif = masrah.path().join("talif");
    std::fs::create_dir_all(&talif)?;
    std::fs::write(talif.join(MASAR_BAYAN), b"<html>captive portal</html>")?;
    let natija =
        jaddid_qaimat_sahb(&silsila(talif), &masarat, &malik().aam(), &asliya()?, waqt).await;
    assert!(matches!(natija, NatijatTajdid::QaimaMutaadhdhira { .. }), "{}", natija.wasf());

    let muraqaba = iqra(&masarat, waqt)?;
    assert!(muraqaba.rafd().is_some(), "an answering source with no usable list refuses");
    Ok(())
}
