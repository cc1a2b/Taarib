//! The community index against directory sources: served, cached, stale and served anyway, refused.

use std::error::Error;
use std::path::Path;

use jiff::{SignedDuration, Timestamp};
use taarib_mustawda::khata::KhataMustawda;
use taarib_mustawda::masadir::{MasdarMustawda, SilsilatMasadir};
use taarib_mustawda::mujtama::{
    AslFahrasMujtama, HADD_HAJM_FAHRAS_MUJTAMA, MASAR_FAHRAS_MUJTAMA, NAFIDHAT_FAHRAS_MUJTAMA,
    NatijatJalbMujtama, iqra_makhbaa, jalb_fahras_mujtama, masar_makhbaa_mujtama, sijill_jalb,
};
use taarib_usus::masarat::Masarat;

/// Every test returns this so that a fixture failure propagates with `?`.
type NatijatIkhtibar = Result<(), Box<dyn Error>>;

/// The same trimmed index the unit tests read.
const AYYINA: &[u8] = include_bytes!("mujtama/tarjamat.json");

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

/// A served index is answered, cached at the repository's own path, stamped,
/// and answered from the cache — with no source asked — inside the window.
#[tokio::test]
async fn yujlab_wa_yukhzan_wa_yuqaddam_min_al_makhbaa() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, AYYINA)?;
    let waqt = al_aan()?;

    let majlub = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, waqt).await?;
    assert_eq!(majlub.fahras.tarjamat.len(), 7);
    match &majlub.asl {
        AslFahrasMujtama::Masdar { masdar } => assert!(masdar.contains("bundled mirror")),
        akhar => return Err(format!("served from {akhar:?}, not from the source").into()),
    }
    assert!(majlub.asl.hadith());

    let masar = masar_makhbaa_mujtama(&masarat);
    assert_eq!(
        std::fs::read(&masar)?,
        AYYINA,
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

    let mukhazzan = iqra_makhbaa(&masarat).ok_or("the cache did not read back")?;
    assert_eq!(mukhazzan.fahras, majlub.fahras);
    assert!(mukhazzan.hadith(waqt));

    // An hour later, with no source anywhere: still answered, from the cache.
    let baad_saa = baad(waqt, SignedDuration::from_hours(1))?;
    let thani = jalb_fahras_mujtama(&la_masadir(), &masarat, baad_saa).await?;
    assert_eq!(thani.fahras, majlub.fahras);
    assert_eq!(
        thani.asl,
        AslFahrasMujtama::Makhbaa {
            umr: SignedDuration::from_hours(1)
        }
    );
    Ok(())
}

/// Past the window and with every source refusing, the stale cache is served
/// and says so, and the record says what refused.
#[tokio::test]
async fn al_qadeem_yuqaddam_inda_fashal_kull_masdar() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    mustawda(&jidhr, AYYINA)?;
    let waqt = al_aan()?;
    let _ = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, waqt).await?;

    let baad_yawmayn = baad(
        waqt,
        NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?,
    )?;
    let majlub = jalb_fahras_mujtama(&la_masadir(), &masarat, baad_yawmayn).await?;
    assert_eq!(majlub.fahras.tarjamat.len(), 7);
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
    mustawda(&jidhr, AYYINA)?;
    let waqt = al_aan()?;
    let _ = jalb_fahras_mujtama(&silsila(&jidhr), &masarat, waqt).await?;

    let talif = masrah.path().join("talif");
    mustawda(&talif, b"<!doctype html><title>404</title>")?;
    let baad_yawmayn = baad(
        waqt,
        NAFIDHAT_FAHRAS_MUJTAMA.checked_mul(2).ok_or("overflow")?,
    )?;
    let majlub = jalb_fahras_mujtama(&silsila(&talif), &masarat, baad_yawmayn).await?;
    assert_eq!(majlub.fahras.tarjamat.len(), 7);
    assert!(matches!(majlub.asl, AslFahrasMujtama::MakhbaaQadeem { .. }));
    assert_eq!(
        std::fs::read(masar_makhbaa_mujtama(&masarat))?,
        AYYINA,
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

/// With nothing cached and nothing served, the answer is the refusal — not an
/// empty index that would read as "no community translation exists".
#[tokio::test]
async fn la_makhbaa_wa_la_masdar_khata() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    match jalb_fahras_mujtama(&la_masadir(), &masarat, al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaGhayrMutah { .. }) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} with nothing to answer from", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as unreachable").into()),
    }
    assert!(iqra_makhbaa(&masarat).is_none());
    assert!(matches!(
        sijill_jalb(&masarat)
            .akhir_muhawala
            .as_ref()
            .map(|muhawala| &muhawala.natija),
        Some(NatijatJalbMujtama::MustawdaGhayrMutah { .. })
    ));

    let talif = masrah.path().join("talif");
    mustawda(&talif, b"not json")?;
    match jalb_fahras_mujtama(&silsila(&talif), &masarat, al_aan()?).await {
        Err(KhataMustawda::FahrasMujtamaTalif { .. }) => {},
        Ok(majlub) => {
            return Err(format!("answered {:?} from an unreadable source", majlub.asl).into());
        },
        Err(akhar) => return Err(format!("refused as {akhar}, not as unreadable").into()),
    }
    Ok(())
}

/// An index over the cap is refused, and nothing of it is cached.
#[tokio::test]
async fn al_hajm_al_mufrit_marfud() -> NatijatIkhtibar {
    let masrah = tempfile::tempdir()?;
    let masarat = masarat(&masrah);
    let jidhr = masrah.path().join("mustawda");
    let hadd = usize::try_from(HADD_HAJM_FAHRAS_MUJTAMA)?;
    // The bytes are a real index padded past the cap, so it is the cap and
    // not the parse that refuses it.
    let mut kabir = AYYINA.to_vec();
    kabir.resize(hadd + 1, b' ');
    mustawda(&jidhr, &kabir)?;

    match jalb_fahras_mujtama(&silsila(&jidhr), &masarat, al_aan()?).await {
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
