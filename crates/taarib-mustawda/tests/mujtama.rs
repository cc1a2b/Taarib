//! The community index against directory sources: served, cached, stale and served anyway, refused.
//!
//! Every index these tests serve is cast here, through the same writer the
//! registry's own tool uses, under a throwaway key generated in this file. The
//! owner's private half lives in one keychain and in no repository, fixture,
//! environment variable or test.

use std::error::Error;
use std::path::Path;

use jiff::{SignedDuration, Timestamp};
use taarib_khatm::{MiftahAam, MiftahKhass};
use taarib_mustawda::khata::KhataMustawda;
use taarib_mustawda::masadir::{MasdarMustawda, SilsilatMasadir};
use taarib_mustawda::mujtama::{
    AslFahrasMujtama, HADD_HAJM_FAHRAS_MUJTAMA, KatibFahrasMujtama, MASAR_FAHRAS_MUJTAMA,
    NAFIDHAT_FAHRAS_MUJTAMA, NatijatJalbMujtama, iqra_makhbaa, jalb_fahras_mujtama,
    masar_makhbaa_mujtama, sijill_jalb,
};
use taarib_usus::masarat::Masarat;

/// Every test returns this so that a fixture failure propagates with `?`.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// The unsigned index body the unit tests read — what a maintainer edits.
const AYYINA: &[u8] = include_bytes!("mujtama/tarjamat.json");

/// The throwaway owner key every index here is cast under.
fn malik() -> MiftahKhass {
    MiftahKhass::min_bayt(&[21_u8; 32])
}

/// A second key, for the source that signs with the wrong one.
fn ghareeb() -> MiftahKhass {
    MiftahKhass::min_bayt(&[22_u8; 32])
}

/// The fixture body cast into a signed document under `khass`.
fn wathiqa(khass: &MiftahKhass) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(KatibFahrasMujtama::min_bayt(AYYINA)?.uktub(khass)?)
}

fn al_aan() -> Result<Timestamp, Box<dyn Error>> {
    Ok("2026-09-12T12:00:00Z".parse::<Timestamp>()?)
}

/// A data root of its own for every test, so nothing shares a cache.
fn masarat(masrah: &tempfile::TempDir) -> Masarat {
    Masarat::min_judhur(masrah.path().join("bayanat"), masrah.path().join("idadat"))
}

/// A repository directory carrying `bayt` at the index's path.
fn mustawda(jidhr: &Path, bayt: &[u8]) -> NatijatIkhtibar {
    let masar = jidhr.join(MASAR_FAHRAS_MUJTAMA);
    if let Some(walid) = masar.parent() {
        std::fs::create_dir_all(walid)?;
    }
    std::fs::write(masar, bayt)?;
    Ok(())
}

/// Puts `bayt` in the cache the way an older build would have left it there.
fn makhbaa(masarat: &Masarat, bayt: &[u8]) -> NatijatIkhtibar {
    let masar = masar_makhbaa_mujtama(masarat);
    if let Some(walid) = masar.parent() {
        std::fs::create_dir_all(walid)?;
    }
    std::fs::write(masar, bayt)?;
    Ok(())
}

fn silsila(jidhr: &Path) -> SilsilatMasadir {
    SilsilatMasadir::min_masdar(MasdarMustawda::MujalladMahalli {
        jidhr: jidhr.to_path_buf(),
    })
}

/// A chain with nothing in it: offline mode with no local copy.
fn la_masadir() -> SilsilatMasadir {
    SilsilatMasadir::jadida(Vec::new())
}

fn baad(waqt: Timestamp, muddat: SignedDuration) -> Result<Timestamp, Box<dyn Error>> {
    Ok(waqt.checked_add(muddat)?)
}

fn miftah() -> MiftahAam {
    malik().aam()
}

/// A served index is verified, answered, cached at the repository's own path,
/// stamped, and answered from the cache — with no source asked — inside the
/// window.
#[tokio::test]
async fn yujlab_wa_yukhzan_wa_yuqaddam_min_al_makhbaa() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    let muwaqqa = wathiqa(&malik())?;
    mustawda(&jidhr, &muwaqqa)?;
    let waqt = al_aan()?;

    let majlub = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), waqt).await?;
    assert_eq!(majlub.fahras.tarjamat().len(), 7);
    match &majlub.asl {
        AslFahrasMujtama::Masdar { masdar } => assert!(masdar.contains("bundled mirror")),
        akhar => return Err(format!("served from {akhar:?}, not from the source").into()),
    }
    assert!(majlub.asl.hadith());

    let masar = masar_makhbaa_mujtama(&masarat);
    assert_eq!(
        std::fs::read(&masar)?,
        muwaqqa,
        "the cache is the bytes the source served"
    );
    assert!(
        masar.ends_with(Path::new("makhbaa/mustawda/fahras/tarjamat.json")),
        "{}",
        masar.display()
    );
    let sijill = sijill_jalb(&masarat);
    assert_eq!(
        sijill.akhir_najah.as_ref().map(|najah| najah.waqt),
        Some(waqt)
    );
    assert!(matches!(
        sijill
            .akhir_muhawala
            .as_ref()
            .map(|muhawala| &muhawala.natija),
        Some(NatijatJalbMujtama::Najah { .. })
    ));

    let mukhazzan = iqra_makhbaa(&masarat, &miftah())?.ok_or("the cache did not read back")?;
    assert_eq!(mukhazzan.fahras, majlub.fahras);
    assert!(mukhazzan.hadith(waqt));

    // An hour later, with no source anywhere: still answered, from the cache.
    let baad_saa = baad(waqt, SignedDuration::from_hours(1))?;
    let thani = jalb_fahras_mujtama(&la_masadir(), &masarat, &miftah(), baad_saa).await?;
    assert_eq!(thani.fahras, majlub.fahras);
    assert_eq!(
        thani.asl,
        AslFahrasMujtama::Makhbaa {
            umr: SignedDuration::from_hours(1)
        }
    );
    Ok(())
}

/// The signature holds for the copy on disk and not only for the fetch that
/// brought it: the cached document is verified again, by itself, from a data
/// root nothing else has touched.
#[tokio::test]
async fn al_makhbaa_yuhaqqaq_baad_rihlat_qurs() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, &wathiqa(&malik())?)?;
    let waqt = al_aan()?;
    let majlub = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), waqt).await?;

    // A second reader with the same data root and nothing in memory: the
    // document on disk is read and verified from scratch.
    let thani = masarat.clone();
    let mukhazzan = iqra_makhbaa(&thani, &miftah())?.ok_or("the cache did not read back")?;
    assert_eq!(mukhazzan.fahras, majlub.fahras);
    assert!(mukhazzan.fahras.mudifun().contains("www.nexusmods.com"));

    // And the same bytes, read under any other key, are nobody's index.
    assert!(matches!(
        iqra_makhbaa(&thani, &ghareeb().aam()),
        Err(KhataMustawda::FahrasMujtamaTawqeeBatil { .. })
    ));
    Ok(())
}

/// Past the window and with every source refusing, the stale cache is served
/// and says so, and the record says what refused.
#[tokio::test]
async fn al_qadeem_yuqaddam_inda_fashal_kull_masdar() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, &wathiqa(&malik())?)?;
    let waqt = al_aan()?;
    let _ = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), waqt).await?;

    let baad_yawmayn = baad(
        waqt,
        NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?,
    )?;
    let majlub = jalb_fahras_mujtama(&la_masadir(), &masarat, &miftah(), baad_yawmayn).await?;
    assert_eq!(majlub.fahras.tarjamat().len(), 7);
    match &majlub.asl {
        AslFahrasMujtama::MakhbaaQadeem { umr, sabab } => {
            assert_eq!(
                *umr,
                Some(NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?)
            );
            assert!(sabab.contains("no registry source"), "{sabab}");
        },
        akhar => return Err(format!("served as {akhar:?}, not as the stale cache").into()),
    }
    assert!(!majlub.asl.hadith());

    let sijill = sijill_jalb(&masarat);
    assert_eq!(
        sijill.akhir_najah.as_ref().map(|najah| najah.waqt),
        Some(waqt),
        "a failed attempt does not move the last success"
    );
    assert!(matches!(
        sijill
            .akhir_muhawala
            .as_ref()
            .map(|muhawala| &muhawala.natija),
        Some(NatijatJalbMujtama::MustawdaGhayrMutah { .. })
    ));
    Ok(())
}

/// A source that answers with something unreadable neither replaces the cache
/// nor counts as unreachable.
#[tokio::test]
async fn fahras_talif_la_yastabdil_al_makhbaa() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    let muwaqqa = wathiqa(&malik())?;
    mustawda(&jidhr, &muwaqqa)?;
    let waqt = al_aan()?;
    let _ = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), waqt).await?;

    let talif = masrah.path().join("talif");
    mustawda(&talif, b"<!doctype html><title>404</title>")?;
    let baad_yawmayn = baad(
        waqt,
        NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?,
    )?;
    let majlub = jalb_fahras_mujtama(&silsila(&talif), &masarat, &miftah(), baad_yawmayn).await?;
    assert_eq!(majlub.fahras.tarjamat().len(), 7);
    assert!(matches!(majlub.asl, AslFahrasMujtama::MakhbaaQadeem { .. }));
    assert_eq!(
        std::fs::read(masar_makhbaa_mujtama(&masarat))?,
        muwaqqa,
        "the unreadable answer never reached the cache"
    );
    assert!(matches!(
        sijill_jalb(&masarat)
            .akhir_muhawala
            .as_ref()
            .map(|muhawala| &muhawala.natija),
        Some(NatijatJalbMujtama::FahrasTalif { .. })
    ));
    Ok(())
}

/// A source serving a well-formed index nobody this build trusts signed: the
/// cache keeps what it had, and the record says the signature was refused
/// rather than that the source served rubbish.
#[tokio::test]
async fn tawqee_marfud_la_yastabdil_al_makhbaa() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    let muwaqqa = wathiqa(&malik())?;
    mustawda(&jidhr, &muwaqqa)?;
    let waqt = al_aan()?;
    let _ = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), waqt).await?;

    // The same entries, cast by somebody else.
    let mughayyar = masrah.path().join("mughayyar");
    mustawda(&mughayyar, &wathiqa(&ghareeb())?)?;
    let baad_yawmayn = baad(
        waqt,
        NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?,
    )?;
    let majlub =
        jalb_fahras_mujtama(&silsila(&mughayyar), &masarat, &miftah(), baad_yawmayn).await?;
    assert!(matches!(majlub.asl, AslFahrasMujtama::MakhbaaQadeem { .. }));
    assert_eq!(
        std::fs::read(masar_makhbaa_mujtama(&masarat))?,
        muwaqqa,
        "an index signed by another key never reached the cache"
    );
    match sijill_jalb(&masarat)
        .akhir_muhawala
        .as_ref()
        .map(|muhawala| muhawala.natija.clone())
    {
        Some(NatijatJalbMujtama::TawqeeMarfud { sabab, .. }) => {
            assert!(sabab.contains("signature"), "{sabab}");
        },
        akhar => return Err(format!("recorded as {akhar:?}, not as a refused signature").into()),
    }
    Ok(())
}

/// With nothing cached and nothing served, the answer is the refusal — not an
/// empty index that would read as "no community translation exists".
#[tokio::test]
async fn la_makhbaa_wa_la_masdar_khata() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    match jalb_fahras_mujtama(&la_masadir(), &masarat, &miftah(), al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaGhayrMutah { .. }) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} with nothing to answer from", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as unreachable").into()),
    }
    assert!(iqra_makhbaa(&masarat, &miftah())?.is_none());
    assert!(matches!(
        sijill_jalb(&masarat)
            .akhir_muhawala
            .as_ref()
            .map(|muhawala| &muhawala.natija),
        Some(NatijatJalbMujtama::MustawdaGhayrMutah { .. })
    ));

    let talif = masrah.path().join("talif");
    mustawda(&talif, b"not json")?;
    match jalb_fahras_mujtama(&silsila(&talif), &masarat, &miftah(), al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaTalif { .. }) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} from an unreadable source", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as unreadable").into()),
    }
    Ok(())
}

/// The upgrade: a machine whose cache holds the unsigned index an older build
/// wrote. It is never read, never answered and never counted, and the user is
/// told which refusal it earned rather than shown an empty panel; the first
/// source that serves a signed index replaces it and the panel fills.
#[tokio::test]
async fn makhbaa_ghayr_muwaqqa_marfud_bil_ism_thumma_yushfa() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    makhbaa(&masarat, AYYINA)?;

    assert!(matches!(
        iqra_makhbaa(&masarat, &miftah()),
        Err(KhataMustawda::FahrasMujtamaGhayrMuwaqqa)
    ));

    // Offline: the refusal the held copy earned is the answer, not the
    // chain's "nothing is cached" — which would be a lie about this machine.
    match jalb_fahras_mujtama(&la_masadir(), &masarat, &miftah(), al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaGhayrMuwaqqa) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} out of an unsigned cache", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as unsigned").into()),
    }
    assert_eq!(
        std::fs::read(masar_makhbaa_mujtama(&masarat))?,
        AYYINA,
        "a refused cache is left where it is, so the refusal keeps explaining itself"
    );

    // Online again, and the registry now serves a signed index.
    let jidhr = masrah.path().join("mustawda");
    let muwaqqa = wathiqa(&malik())?;
    mustawda(&jidhr, &muwaqqa)?;
    let majlub = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), al_aan()?).await?;
    assert_eq!(majlub.fahras.tarjamat().len(), 7);
    assert!(matches!(majlub.asl, AslFahrasMujtama::Masdar { .. }));
    assert_eq!(
        std::fs::read(masar_makhbaa_mujtama(&masarat))?,
        muwaqqa,
        "the signed index replaced the unsigned cache"
    );
    assert!(iqra_makhbaa(&masarat, &miftah())?.is_some());
    Ok(())
}

/// A cache signed by a key this build is not anchored to — the older key after
/// a rotation, or a substituted body — is refused on its own terms, and by the
/// severer of the two names.
#[tokio::test]
async fn makhbaa_bi_miftah_qadeem_marfud_bil_ism() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    makhbaa(&masarat, &wathiqa(&ghareeb())?)?;

    assert!(matches!(
        iqra_makhbaa(&masarat, &miftah()),
        Err(KhataMustawda::FahrasMujtamaTawqeeBatil { .. })
    ));
    match jalb_fahras_mujtama(&la_masadir(), &masarat, &miftah(), al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaTawqeeBatil { .. }) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} out of a foreign cache", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as a bad signature").into()),
    }

    // And it heals the same way: one source serving an index this build's
    // anchor verifies.
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, &wathiqa(&malik())?)?;
    let majlub = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), al_aan()?).await?;
    assert!(matches!(majlub.asl, AslFahrasMujtama::Masdar { .. }));
    assert!(iqra_makhbaa(&masarat, &miftah())?.is_some());
    Ok(())
}

/// An index over the cap is refused, and nothing of it is cached.
#[tokio::test]
async fn al_hajm_al_mufrit_marfud() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    let hadd = usize::try_from(HADD_HAJM_FAHRAS_MUJTAMA)?;
    // The bytes are a real signed index padded past the cap, so it is the cap
    // and not the parse or the signature that refuses it.
    let mut kabir = wathiqa(&malik())?;
    kabir.resize(hadd + 1, b' ');
    mustawda(&jidhr, &kabir)?;

    match jalb_fahras_mujtama(&silsila(&jidhr), &masarat, &miftah(), al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaKabir {
            hajm,
            hadd: muallan,
        }) => {
            assert_eq!(hajm, HADD_HAJM_FAHRAS_MUJTAMA + 1);
            assert_eq!(muallan, HADD_HAJM_FAHRAS_MUJTAMA);
        },
        Ok(majlub) => {
            return Err(format!("an index over the cap was answered from {:?}", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not by size").into()),
    }
    assert!(!masar_makhbaa_mujtama(&masarat).exists());
    Ok(())
}
