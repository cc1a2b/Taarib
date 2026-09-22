//! سبك القناة — casting the update channel: built packages in, a signed
//! `tahdith.json` out.
//!
//! The maintainer's side of everything [`crate::bayan`] reads, and the only
//! thing in the tree that writes a channel manifest. Until it existed every
//! client's update check asked the registry for a document nobody had ever
//! published, and the request — correctly answered with `404` — arrived as a
//! transport failure.
//!
//! Every field a client verifies against is derived here from the package file
//! itself: the size and the SHA-256 are computed, never typed. The signature is
//! built through `taarib-khatm`, which stays the only crate that holds a
//! private key.
//!
//! Failures are sentences rather than an enum, exactly as the registry caster's
//! are. Every one of them is an operator's next action — a package that would
//! not open, an entry no client could use, a manifest already in the repository
//! that this key did not sign — and nothing branches on which.

use std::fs;
use std::path::Path;

use sha2::{Digest as _, Sha256};
use taarib_khatm::{HawiyatThiqa, MiftahAam, MirsatThiqa};

use crate::bayan::{BayanTahdith, MadkhalTahdith, ihlil};

/// The channel manifest's name under the registry root, as the client asks for
/// it.
pub const ISM_BAYAN: &str = "tahdith.json";

/// Its detached signature, beside it.
pub const ISM_TAWQEE: &str = "tahdith.json.tawqee";

/// The placeholder a release-asset base carries for the package's file name.
const QALAB_ISM: &str = "{ism}";

/// One built package and what the manifest must say about it.
#[derive(Debug)]
pub struct Huzma<'a> {
    /// The built file, read to derive its size and digest.
    pub masar: &'a Path,
    /// The target triple it runs on.
    pub hadaf: &'a str,
    /// Its full download address, when it is not under the run's base.
    pub rabt: Option<&'a str>,
}

/// What every entry in one cast shares.
#[derive(Debug)]
pub struct KhiyaratSabk<'a> {
    /// The channel being cast.
    pub qanat: &'a str,
    /// The version these packages are.
    pub isdar: &'a str,
    /// The lowest version they may update in place.
    pub adna: &'a str,
    /// The release-asset base an entry with no address of its own is built
    /// from.
    pub asas: Option<&'a str>,
}

/// Turns one built package into the entry a client downloads.
///
/// The size and the digest come out of the file, so the two facts a client
/// refuses a download over cannot be mistyped into agreement with each other.
///
/// # Errors
///
/// A sentence naming the package when it will not open, when it is larger than
/// this build counts, when it has no usable file name, or when the run gave it
/// neither an address of its own nor a base to build one from.
pub fn madkhal_min_huzma(
    huzma: &Huzma<'_>,
    khiyarat: &KhiyaratSabk<'_>,
) -> Result<MadkhalTahdith, String> {
    let bayt =
        fs::read(huzma.masar).map_err(|khata| format!("{}: {khata}", huzma.masar.display()))?;
    let hajm = u64::try_from(bayt.len())
        .map_err(|_| format!("{} is larger than this build counts", huzma.masar.display()))?;
    let sha256 = hex::encode(Sha256::digest(&bayt));

    Ok(MadkhalTahdith {
        hadaf: huzma.hadaf.to_owned(),
        qanat: khiyarat.qanat.to_owned(),
        isdar: khiyarat.isdar.to_owned(),
        rabt: rabt_asl(huzma, khiyarat.asas)?,
        hajm,
        sha256,
        adna_isdar: khiyarat.adna.to_owned(),
    })
}

/// Where one package is downloaded from: its own address, or the run's base
/// with the package's file name put into it.
///
/// A base holding `{ism}` gets the file name substituted, which is the only
/// form a forge's flat release area answers; one holding neither has the file
/// name appended, which is what a directory server wants.
///
/// # Errors
///
/// A sentence when the package has no usable file name, or when neither an
/// address nor a base was given.
fn rabt_asl(huzma: &Huzma<'_>, asas: Option<&str>) -> Result<String, String> {
    if let Some(rabt) = huzma.rabt {
        return Ok(rabt.to_owned());
    }
    let ism = huzma
        .masar
        .file_name()
        .and_then(|ism| ism.to_str())
        .ok_or_else(|| format!("{} has no usable file name", huzma.masar.display()))?;
    let asas = asas.ok_or_else(|| {
        format!(
            "{} was given neither --rabt nor a run --asas",
            huzma.masar.display()
        )
    })?;
    if asas.contains(QALAB_ISM) {
        Ok(asas.replace(QALAB_ISM, ism))
    } else {
        Ok(format!("{}/{ism}", asas.trim_end_matches('/')))
    }
}

/// The manifest already in the repository, verified against the key this run
/// signs with.
///
/// Absent is [`None`] and is the ordinary first cast. Present and unverifiable
/// is a refusal: merging into a document this key did not sign would publish
/// entries nobody vouched for under a signature that says otherwise.
///
/// # Errors
///
/// A sentence when either file will not open, and when the manifest does not
/// verify against `aam`.
pub fn iqra_sabiq(jidhr: &Path, aam: &MiftahAam) -> Result<Option<BayanTahdith>, String> {
    let masar_bayan = jidhr.join(ISM_BAYAN);
    if !masar_bayan.is_file() {
        return Ok(None);
    }
    let masar_tawqee = jidhr.join(ISM_TAWQEE);

    let matn =
        fs::read(&masar_bayan).map_err(|khata| format!("{}: {khata}", masar_bayan.display()))?;
    let tawqee = fs::read_to_string(&masar_tawqee)
        .map_err(|khata| format!("{}: {khata}", masar_tawqee.display()))?;
    let mirsa = MirsatThiqa {
        miftah: aam.bayt(),
        hawiya: HawiyatThiqa::Isdar,
    };
    ihlil(&matn, tawqee.trim(), &mirsa)
        .map(Some)
        .map_err(|khata| {
            format!(
                "{} does not verify against this key and will not be merged into: {khata}",
                masar_bayan.display()
            )
        })
}

/// Writes the cast manifest and its detached signature into the repository.
///
/// `matn` must be the exact bytes the signature was built over: re-serializing
/// them anywhere between the writer and here invalidates it.
///
/// # Errors
///
/// A sentence naming whichever path would not be created or written.
pub fn uktub(jidhr: &Path, matn: &[u8], tawqee: &str) -> Result<(), String> {
    uktub_malaf(jidhr, ISM_BAYAN, matn)?;
    uktub_malaf(jidhr, ISM_TAWQEE, tawqee.as_bytes())
}

fn uktub_malaf(jidhr: &Path, nisbi: &str, bayt: &[u8]) -> Result<(), String> {
    let masar = jidhr.join(nisbi);
    if let Some(walid) = masar.parent() {
        fs::create_dir_all(walid).map_err(|khata| format!("{}: {khata}", walid.display()))?;
    }
    fs::write(&masar, bayt).map_err(|khata| format!("{}: {khata}", masar.display()))
}

/// The floor a release takes when the run does not name one: the first version
/// of its own major line, which is the widest step that never crosses a
/// breaking boundary.
#[must_use]
pub fn adna_iftiradi(isdar: &str) -> Option<String> {
    let kabir = isdar.split('.').next()?;
    if kabir.is_empty() || !kabir.bytes().all(|bayt| bayt.is_ascii_digit()) {
        return None;
    }
    Some(format!("{kabir}.0.0"))
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;
    use std::path::{Path, PathBuf};

    use sha2::{Digest as _, Sha256};
    use taarib_khatm::MiftahKhass;

    use super::{
        Huzma, ISM_BAYAN, ISM_TAWQEE, KhiyaratSabk, adna_iftiradi, iqra_sabiq, madkhal_min_huzma,
        uktub,
    };
    use crate::bayan::{KatibBayan, QANAT_MUSTAQIRR, QANAT_TAJRIBI, intiqa};

    /// What every test here answers with, so a fixture failure propagates with
    /// `?`. `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar<T = ()> = Result<T, Box<dyn Error>>;

    const HADAF: &str = "x86_64-pc-windows-msvc";

    fn khiyarat<'a>(qanat: &'a str, isdar: &'a str, asas: Option<&'a str>) -> KhiyaratSabk<'a> {
        KhiyaratSabk {
            qanat,
            isdar,
            adna: "1.0.0",
            asas,
        }
    }

    /// Writes a package of `hajm` pseudo-random-but-fixed bytes and answers its
    /// path beside the digest it must be published under.
    fn huzma(jidhr: &Path, ism: &str, hajm: usize) -> NatijatIkhtibar<(PathBuf, String)> {
        let bayt: Vec<u8> = (0..hajm)
            .map(|fihris| u8::try_from(fihris % 251).unwrap_or(0))
            .collect();
        let masar = jidhr.join(ism);
        std::fs::write(&masar, &bayt)?;
        Ok((masar, hex::encode(Sha256::digest(&bayt))))
    }

    /// The two facts a client refuses a download over are read out of the file,
    /// so they cannot be typed into agreement with each other.
    #[test]
    fn al_hajm_wa_albasma_min_almalaf_nafsihi() -> NatijatIkhtibar {
        let mujallad = tempfile::tempdir()?;
        let (masar, basma) = huzma(mujallad.path(), "taarib-1.3.0-x64.exe", 4096)?;

        let madkhal = madkhal_min_huzma(
            &Huzma {
                masar: &masar,
                hadaf: HADAF,
                rabt: None,
            },
            &khiyarat(
                QANAT_MUSTAQIRR,
                "1.3.0",
                Some("https://example.invalid/v1.3.0/{ism}"),
            ),
        )?;

        assert_eq!(madkhal.hajm, 4096);
        assert_eq!(madkhal.sha256, basma);
        assert_eq!(
            madkhal.rabt, "https://example.invalid/v1.3.0/taarib-1.3.0-x64.exe",
            "{{ism}} takes the package's own file name"
        );
        assert_eq!(madkhal.isdar, "1.3.0");
        assert_eq!(madkhal.adna_isdar, "1.0.0");
        Ok(())
    }

    /// A base with no placeholder has the file name appended; an entry with its
    /// own address ignores the base entirely.
    #[test]
    fn asas_bila_qalab_yulhiq_wa_rabt_khass_yasbiq() -> NatijatIkhtibar {
        let mujallad = tempfile::tempdir()?;
        let (masar, _) = huzma(mujallad.path(), "taarib.AppImage", 32)?;

        let mulhaq = madkhal_min_huzma(
            &Huzma {
                masar: &masar,
                hadaf: HADAF,
                rabt: None,
            },
            &khiyarat(
                QANAT_MUSTAQIRR,
                "1.3.0",
                Some("https://example.invalid/isdar/"),
            ),
        )?;
        assert_eq!(mulhaq.rabt, "https://example.invalid/isdar/taarib.AppImage");

        let khass = madkhal_min_huzma(
            &Huzma {
                masar: &masar,
                hadaf: HADAF,
                rabt: Some("https://akhar.invalid/one-off"),
            },
            &khiyarat(QANAT_MUSTAQIRR, "1.3.0", Some("https://example.invalid/")),
        )?;
        assert_eq!(khass.rabt, "https://akhar.invalid/one-off");

        let bila = madkhal_min_huzma(
            &Huzma {
                masar: &masar,
                hadaf: HADAF,
                rabt: None,
            },
            &khiyarat(QANAT_MUSTAQIRR, "1.3.0", None),
        );
        assert!(bila.is_err(), "an entry with no address at all is refused");
        Ok(())
    }

    /// The whole cast, twice: a released channel, then a pre-release one into
    /// the same repository. The second run must leave the first standing, and
    /// both must read back through the client's own verifier.
    #[test]
    fn sabk_qanatayn_fi_mustawda_wahid() -> NatijatIkhtibar {
        let mustawda = tempfile::tempdir()?;
        let bina = tempfile::tempdir()?;
        let khass = MiftahKhass::min_bayt(&[5u8; 32]);
        let aam = khass.aam();
        let (masar, _) = huzma(bina.path(), "taarib-1.3.0-x64.exe", 8192)?;

        assert!(
            iqra_sabiq(mustawda.path(), &aam)?.is_none(),
            "an empty repository carries no manifest to merge into"
        );

        let awwal = KatibBayan::jadeed()
            .bila_qanat(QANAT_MUSTAQIRR)
            .adif(madkhal_min_huzma(
                &Huzma {
                    masar: &masar,
                    hadaf: HADAF,
                    rabt: None,
                },
                &khiyarat(
                    QANAT_MUSTAQIRR,
                    "1.3.0",
                    Some("https://example.invalid/v1.3.0/{ism}"),
                ),
            )?);
        let (matn, tawqee) = awwal.uktub(&khass)?;
        uktub(mustawda.path(), &matn, &tawqee)?;

        let sabiq = iqra_sabiq(mustawda.path(), &aam)?
            .ok_or("the cast manifest did not read back through the verifier")?;
        let thani = KatibBayan::min_bayan(&sabiq)
            .bila_qanat(QANAT_TAJRIBI)
            .adif(madkhal_min_huzma(
                &Huzma {
                    masar: &masar,
                    hadaf: HADAF,
                    rabt: None,
                },
                &khiyarat(
                    QANAT_TAJRIBI,
                    "1.4.0",
                    Some("https://example.invalid/v1.4.0/{ism}"),
                ),
            )?);
        let (matn, tawqee) = thani.uktub(&khass)?;
        uktub(mustawda.path(), &matn, &tawqee)?;

        let nihai =
            iqra_sabiq(mustawda.path(), &aam)?.ok_or("the second cast did not read back")?;
        assert_eq!(
            intiqa(&nihai, HADAF, QANAT_MUSTAQIRR, "1.2.0")?.map(|madkhal| madkhal.isdar.clone()),
            Some("1.3.0".to_owned()),
            "casting the pre-release channel withdrew the released one"
        );
        assert_eq!(
            intiqa(&nihai, HADAF, QANAT_TAJRIBI, "1.2.0")?.map(|madkhal| madkhal.isdar.clone()),
            Some("1.4.0".to_owned())
        );
        Ok(())
    }

    /// A manifest this key did not sign is never merged into: publishing over
    /// it would put entries nobody vouched for under a signature that says
    /// otherwise.
    #[test]
    fn mustawda_bitawqee_gharib_yurfad() -> NatijatIkhtibar {
        let mustawda = tempfile::tempdir()?;
        let gharib = MiftahKhass::min_bayt(&[6u8; 32]);
        let (matn, tawqee) = KatibBayan::jadeed().uktub(&gharib)?;
        uktub(mustawda.path(), &matn, &tawqee)?;

        let malik = MiftahKhass::min_bayt(&[5u8; 32]);
        let natija = iqra_sabiq(mustawda.path(), &malik.aam());
        match natija {
            Err(sabab) if sabab.contains("does not verify against this key") => Ok(()),
            Err(sabab) => Err(format!("refused for the wrong reason: {sabab}").into()),
            Ok(_) => Err("a manifest signed by another key was merged into".into()),
        }
    }

    /// A manifest published with no signature beside it is a broken
    /// publication, and the caster says which file is missing rather than
    /// casting over it.
    #[test]
    fn bayan_bila_tawqee_yusamma() -> NatijatIkhtibar {
        let mustawda = tempfile::tempdir()?;
        let khass = MiftahKhass::min_bayt(&[5u8; 32]);
        let (matn, _) = KatibBayan::jadeed().uktub(&khass)?;
        std::fs::write(mustawda.path().join(ISM_BAYAN), &matn)?;

        match iqra_sabiq(mustawda.path(), &khass.aam()) {
            Err(sabab) if sabab.contains(ISM_TAWQEE) => Ok(()),
            Err(sabab) => Err(format!("refused for the wrong reason: {sabab}").into()),
            Ok(_) => Err("a manifest with no signature was accepted".into()),
        }
    }

    #[test]
    fn al_hadd_alaadna_alaftiradi_yatbau_khatt_alisdar() {
        assert_eq!(adna_iftiradi("1.3.0"), Some("1.0.0".to_owned()));
        assert_eq!(adna_iftiradi("12.0.4"), Some("12.0.0".to_owned()));
        assert_eq!(adna_iftiradi("v1.3.0"), None);
        assert_eq!(adna_iftiradi(""), None);
    }
}
