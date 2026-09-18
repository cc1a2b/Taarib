//! بيان المخزن — the component store's manifest, and what "complete" means.

use std::path::Path;

use sha2::{Digest as _, Sha256};
use taarib_usus::masarat;

use crate::khata::KhataTathbeet;

/// The staging manifest's file name, identical in the bundle and in the store.
///
/// Spelled here and nowhere else. `taarib-tajmee` writes this file, the Studio
/// mirrors it into the component store, and this crate reads it to decide
/// whether an install may proceed. Three crates that cannot see each other, one
/// name: a rename in any single copy would leave the installer finding no
/// manifest and refusing every component, from a one-word edit.
pub const ISM_MALAF_BAYAN: &str = "bayan_mukawwinat.json";

/// The one manifest schema number this build understands (`tawzee.md` §4).
pub const MUKHATTAT_MADUM: u32 = 1;

/// The manifest prefix that maps into the component store.
///
/// An entry `mukawwinat/<baqi>` mirrors to `<store>/<baqi>`. Everything else —
/// fonts, the signature database — rides beside the binary and is verified
/// without being mirrored.
pub const BADIYAT_MAKHZAN: &str = "mukawwinat/";

/// The component catalogue's file name, identical in the bundle and in the
/// store.
///
/// Spelled here for the reason [`ISM_MALAF_BAYAN`] is: `taarib-tajmee` writes
/// it and this crate reads it, and the two cannot see each other.
pub const ISM_MALAF_FIHRIS: &str = "fihris_mukawwinat.json";

/// The one catalogue schema number this build understands (`tawzee.md` §4a).
pub const MUKHATTAT_FIHRIS_MADUM: u32 = 1;

/// The largest manifest this build will read, in bytes.
///
/// A real one is a few kilobytes; anything near this limit is not a manifest.
const AQSA_HAJM_BAYAN: u64 = 8 * 1024 * 1024;

/// The largest catalogue this build will read, in bytes.
///
/// Larger than [`AQSA_HAJM_BAYAN`] because the catalogue lists every file of
/// every component the release publishes, not only the ones this bundle
/// carries: the full matrix is about sixteen hundred entries.
const AQSA_HAJM_FIHRIS: u64 = 32 * 1024 * 1024;

/// One staged file: a forward-slash path relative to `mawarid/`, its size, and
/// the sha256 the staging tool verified before writing it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MalafMudraj {
    /// The path, forward-slash, relative to `mawarid/`.
    pub masar: String,
    /// The size in bytes.
    pub hajm: u64,
    /// The content hash, 64 lowercase hexadecimal characters.
    pub sha256: String,
}

/// The size and hash of the component catalogue this bundle carries.
///
/// The catalogue cannot list itself — it is written before the manifest, and a
/// document that recorded its own hash would have to be written twice — so the
/// manifest carries the fingerprint instead. That is what makes the catalogue
/// trustworthy: the manifest rides inside the signed installer the user chose
/// to run, and nothing is read out of the catalogue until its bytes hash to
/// this value.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BasmatFihris {
    /// The catalogue's size in bytes.
    pub hajm: u64,
    /// Its content hash, 64 lowercase hexadecimal characters.
    pub sha256: String,
}

/// The staging manifest `taarib-tajmee` writes, `tawzee.md` §4 verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BayanMukawwinat {
    /// The manifest schema number; this build understands [`MUKHATTAT_MADUM`].
    pub mukhattat: u32,
    /// The workspace version the bundle was staged from.
    pub isdar: String,
    /// The target triple the bundle was staged for.
    pub hadaf: String,
    /// Every staged file, in the order the staging tool wrote them.
    pub milaffat: Vec<MalafMudraj>,
    /// The component set this bundle was staged with, `kamil` or `nahif`.
    ///
    /// Defaulted rather than required so that a manifest written before the
    /// two variants existed still reads, and reads as what it is: a bundle
    /// carrying the whole matrix.
    #[serde(default = "taqm_iftiradi")]
    pub taqm: String,
    /// The fingerprint of the catalogue beside this manifest, when one was
    /// staged. [`None`] for a bundle staged before the catalogue existed, in
    /// which case nothing is fetchable and every absence is final.
    #[serde(default)]
    pub fihris: Option<BasmatFihris>,
}

/// The component set a manifest without the field was staged with.
fn taqm_iftiradi() -> String {
    TAQM_KAMIL.to_owned()
}

/// The component set that carries every component of the matrix — the offline
/// bundle, and what every build before this field existed was.
pub const TAQM_KAMIL: &str = "kamil";

/// The component set that leaves the IL2CPP BepInEx builds out of the bundle.
pub const TAQM_NAHIF: &str = "nahif";

/// One component of the release, as the catalogue records it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MukawwinMufahras {
    /// The component's store path, `/`-separated, exactly as
    /// `taarib_tathbeet::tarkib::MukawwinItar::ism` builds it.
    pub ism: String,
    /// Whether the bundle that carries this catalogue also carries the
    /// component's bytes.
    pub fi_alhuzma: bool,
    /// The component's total size in bytes, over every file it holds.
    pub hajm: u64,
    /// Every file of the component, its path relative to the component root.
    pub milaffat: Vec<MalafMudraj>,
}

/// The catalogue of every component the release publishes, carried or not.
///
/// Both bundle variants carry the same catalogue, so the two differ only in
/// which components' bytes are present and in each entry's `fi_alhuzma`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FihrisMukawwinat {
    /// The catalogue schema number; this build understands
    /// [`MUKHATTAT_FIHRIS_MADUM`].
    pub mukhattat: u32,
    /// The workspace version the release was staged from.
    pub isdar: String,
    /// The target triple the release was staged for.
    pub hadaf: String,
    /// Every component of the matrix, sorted by name.
    pub mukawwinat: Vec<MukawwinMufahras>,
}

impl FihrisMukawwinat {
    /// The catalogue entry for one component, by its store path.
    #[must_use]
    pub fn mukawwin(&self, ism: &str) -> Option<&MukawwinMufahras> {
        self.mukawwinat.iter().find(|mukawwin| mukawwin.ism == ism)
    }
}

/// Where the store stands on one component the per-engine table asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HalatMukawwin {
    /// The store holds every file the store's manifest lists for it.
    Mawjud,
    /// The release publishes it and this bundle does not carry it. The two
    /// numbers are what a fetch costs, so a surface can state them before it
    /// starts one.
    QabilLiljalb {
        /// Total bytes across every file of the component.
        hajm: u64,
        /// How many files it holds.
        adad: usize,
    },
    /// Neither the store nor the catalogue knows this name. A build defect or
    /// a damaged store, never something a download fixes.
    Majhul,
}

impl HalatMukawwin {
    /// The sentence a report carries, in English.
    #[must_use]
    pub fn injilizi(&self, ism: &str) -> String {
        match *self {
            Self::Mawjud => format!("{ism} is in the component store."),
            Self::QabilLiljalb { hajm, adad } => format!(
                "{ism} is published by this release and is not in this bundle: {adad} file(s), \
                 {hajm} byte(s). The offline bundle carries it; this one fetches it once and \
                 verifies every byte against the catalogue its own manifest vouches for."
            ),
            Self::Majhul => format!(
                "{ism} is named by neither the component store nor the catalogue. This is a \
                 defect in this build, not in your machine; report it to the project owner."
            ),
        }
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub fn arabi(&self, ism: &str) -> String {
        match *self {
            Self::Mawjud => format!("المكوّن {ism} موجود في مخزن المكوّنات."),
            Self::QabilLiljalb { hajm, adad } => format!(
                "المكوّن {ism} يَنشره هذا الإصدار ولا تحمله هذه النسخة: {adad} ملفًا، {hajm} بايت. \
                 النسخة الكاملة تحمله، وهذه النسخة تجلبه مرة واحدة وتتحقق من كل بايت مقابل فهرس \
                 المكوّنات الذي يضمنه بيان النسخة نفسه."
            ),
            Self::Majhul => format!(
                "المكوّن {ism} لا يرد في مخزن المكوّنات ولا في فهرس المكوّنات. هذا خلل في هذه \
                 النسخة لا في جهازك؛ أبلغ مالك المشروع."
            ),
        }
    }
}

/// Reads the component store's manifest, when it has one.
///
/// [`None`] means the store carries no manifest at all — a development build
/// run from cargo, or a mirror that never settled. That is not an error here;
/// it is the caller's decision what an unmanifested store may be used for.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when the manifest is present and cannot be
/// read, is larger than this build will parse, is not JSON, or declares a
/// schema this build does not understand. A manifest that exists and cannot be
/// trusted is worse than none, so it is refused rather than ignored.
pub fn iqra_bayan(jidhr_makhzan: &Path) -> Result<Option<BayanMukawwinat>, KhataTathbeet> {
    let masar = jidhr_makhzan.join(ISM_MALAF_BAYAN);
    let Ok(wasf) = std::fs::metadata(&masar) else {
        return Ok(None);
    };
    let marfud = |sabab: &str| KhataTathbeet::MukawwinMafqud {
        mukawwin: format!("{ISM_MALAF_BAYAN} ({sabab})"),
        masar: masar.clone(),
    };
    if !wasf.is_file() || wasf.len() > AQSA_HAJM_BAYAN {
        return Err(marfud("not a readable manifest"));
    }
    let bayt = std::fs::read(&masar).map_err(|_| marfud("could not be read"))?;
    let bayan: BayanMukawwinat = serde_json::from_slice(&bayt)
        .map_err(|_| marfud("is not the manifest this build reads"))?;
    if bayan.mukhattat != MUKHATTAT_MADUM {
        return Err(marfud("declares a schema this build does not understand"));
    }
    Ok(Some(bayan))
}

/// Proves every file the manifest lists for a component is present at its
/// declared size.
///
/// "The directory holds at least one file" is not completeness, and the
/// difference is the product's worst silent failure: a mirror interrupted after
/// three of a framework's forty files leaves a directory that answers yes to
/// that question, and the install then deploys three files, records a success,
/// and leaves the game in English with nothing logged.
///
/// Sizes rather than hashes. The bytes were hashed on the way into the store,
/// and re-hashing a BepInEx tree on every install would put seconds onto a
/// confirmation screen to re-answer a question already answered. Truncation and
/// replacement both change a file's length, which is what this catches.
///
/// A store with no manifest is accepted. A development build run from cargo has
/// no manifest and must still be able to install; the missing manifest is
/// already reported at startup as its own named problem.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when a listed file is absent or when the
/// manifest lists nothing at all under this component, and
/// [`KhataTathbeet::MukawwinNaqis`] when a file is present at the wrong size.
pub fn kamil_hasab_bayan(jidhr_makhzan: &Path, ism: &str) -> Result<(), KhataTathbeet> {
    let Some(bayan) = iqra_bayan(jidhr_makhzan)? else {
        return Ok(());
    };

    let badiya = format!("{BADIYAT_MAKHZAN}{ism}/");
    let mut adad = 0_usize;
    for malaf in &bayan.milaffat {
        if !malaf.masar.starts_with(&badiya) {
            continue;
        }
        let Some(dhayl) = malaf.masar.strip_prefix(BADIYAT_MAKHZAN) else {
            continue;
        };
        adad = adad.saturating_add(1);
        let masar =
            masarat::dakhil(jidhr_makhzan, dhayl).map_err(|khata| KhataTathbeet::MasarKharij {
                masar: std::path::PathBuf::from(dhayl),
                jidhr: jidhr_makhzan.to_path_buf(),
                sabab: khata.injilizi,
            })?;
        let Ok(wasf) = std::fs::metadata(&masar) else {
            return Err(KhataTathbeet::MukawwinMafqud {
                mukawwin: format!("{ism}/{dhayl}"),
                masar,
            });
        };
        if !wasf.is_file() || wasf.len() != malaf.hajm {
            return Err(KhataTathbeet::MukawwinNaqis {
                mukawwin: ism.to_owned(),
                masar,
                muallan: malaf.hajm,
                mawjud: wasf.len(),
            });
        }
    }

    // A component the manifest never listed is not a component this build
    // ships, whatever happens to be sitting in a directory with its name.
    if adad == 0 {
        return Err(KhataTathbeet::MukawwinMafqud {
            mukawwin: ism.to_owned(),
            masar: jidhr_makhzan.join(ism),
        });
    }
    Ok(())
}

/// Reads the component catalogue that sits beside a manifest, proving its
/// bytes against the fingerprint that manifest carries.
///
/// [`None`] means the manifest names no catalogue: a bundle staged before the
/// two variants existed, or a development store with no manifest at all.
/// Nothing is fetchable there and every absence is final, which is exactly the
/// behaviour those builds already had.
///
/// The fingerprint, not the catalogue, is the trust anchor. `bayan_mukawwinat.json`
/// rides inside the installer the user chose to run, so a catalogue that hashes
/// to the value it records is as trustworthy as the bundle itself — and a
/// catalogue that does not is refused rather than parsed. That is what lets a
/// later fetch trust the network for nothing at all: the bytes it accepts are
/// the ones whose sha256 the user's own installer already committed to.
///
/// # Errors
///
/// [`KhataTathbeet::MukawwinMafqud`] when the manifest names a catalogue that
/// is absent, unreadable, larger than this build will parse, not JSON, of a
/// schema this build does not understand, or whose bytes do not hash to the
/// recorded fingerprint.
pub fn iqra_fihris(jidhr: &Path) -> Result<Option<FihrisMukawwinat>, KhataTathbeet> {
    let Some(bayan) = iqra_bayan(jidhr)? else {
        return Ok(None);
    };
    let Some(basma) = bayan.fihris else {
        return Ok(None);
    };

    let masar = jidhr.join(ISM_MALAF_FIHRIS);
    let marfud = |sabab: &str| KhataTathbeet::MukawwinMafqud {
        mukawwin: format!("{ISM_MALAF_FIHRIS} ({sabab})"),
        masar: masar.clone(),
    };
    let wasf =
        std::fs::metadata(&masar).map_err(|_| marfud("is named by the manifest and absent"))?;
    if !wasf.is_file() || wasf.len() > AQSA_HAJM_FIHRIS {
        return Err(marfud("not a readable catalogue"));
    }
    if wasf.len() != basma.hajm {
        return Err(marfud("is not the size the manifest records"));
    }
    let bayt = std::fs::read(&masar).map_err(|_| marfud("could not be read"))?;

    let mahsuba = hex_saghir(Sha256::digest(&bayt).as_slice());
    if !mahsuba.eq_ignore_ascii_case(&basma.sha256) {
        return Err(marfud("does not hash to the value the manifest records"));
    }

    let fihris: FihrisMukawwinat = serde_json::from_slice(&bayt)
        .map_err(|_| marfud("is not the catalogue this build reads"))?;
    if fihris.mukhattat != MUKHATTAT_FIHRIS_MADUM {
        return Err(marfud("declares a schema this build does not understand"));
    }
    Ok(Some(fihris))
}

/// Where the store stands on one component, and whether a fetch would fix it.
///
/// The refusal this builds on is the one that already exists: a component the
/// store's manifest never listed is [`KhataTathbeet::MukawwinMafqud`] from
/// [`kamil_hasab_bayan`], and that is what a slim bundle produces for every
/// component it does not carry. This adds only the second question — does the
/// release publish it — so that the answer can be "not here, {n} bytes away"
/// instead of "not here".
///
/// A store that carries no manifest answers [`HalatMukawwin::Mawjud`], for the
/// reason [`kamil_hasab_bayan`] accepts one: a development build run from cargo
/// has no manifest and must still be able to install, and its missing manifest
/// is already reported at startup as its own named problem.
///
/// # Errors
///
/// Whatever [`kamil_hasab_bayan`] refuses for a reason a download does not fix:
/// [`KhataTathbeet::MukawwinNaqis`] for a file present at the wrong size, and
/// [`KhataTathbeet::MasarKharij`] for a name that does not join onto the store
/// root. A store somebody truncated is a damaged store, not a missing one.
pub fn hala_mukawwin(
    jidhr_makhzan: &Path,
    fihris: Option<&FihrisMukawwinat>,
    ism: &str,
) -> Result<HalatMukawwin, KhataTathbeet> {
    match kamil_hasab_bayan(jidhr_makhzan, ism) {
        Ok(()) => Ok(HalatMukawwin::Mawjud),
        Err(KhataTathbeet::MukawwinMafqud { .. }) => {
            let Some(mukawwin) = fihris.and_then(|fihris| fihris.mukawwin(ism)) else {
                return Ok(HalatMukawwin::Majhul);
            };
            Ok(HalatMukawwin::QabilLiljalb {
                hajm: mukawwin.hajm,
                adad: mukawwin.milaffat.len(),
            })
        },
        Err(khata) => Err(khata),
    }
}

/// Lowercase hexadecimal, the spelling both documents compare hashes in.
fn hex_saghir(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut nass = String::with_capacity(bayt.len().saturating_mul(2));
    for wahid in bayt {
        let _ = write!(nass, "{wahid:02x}");
    }
    nass
}

#[cfg(test)]
mod ikhtibarat {
    use std::error::Error;

    use sha2::{Digest as _, Sha256};

    use super::{
        BasmatFihris, BayanMukawwinat, FihrisMukawwinat, HalatMukawwin, ISM_MALAF_BAYAN,
        ISM_MALAF_FIHRIS, MUKHATTAT_FIHRIS_MADUM, MUKHATTAT_MADUM, MalafMudraj, MukawwinMufahras,
        TAQM_KAMIL, TAQM_NAHIF, hala_mukawwin, hex_saghir, iqra_bayan, iqra_fihris,
    };

    /// Every test returns this so that a fixture failure propagates with `?`.
    /// `unwrap` and `expect` are denied workspace-wide, tests included.
    type NatijatIkhtibar = Result<(), Box<dyn Error>>;

    /// One component's worth of catalogue, with one file in it.
    fn mukawwin_mufahras(ism: &str, fi_alhuzma: bool) -> MukawwinMufahras {
        MukawwinMufahras {
            ism: ism.to_owned(),
            fi_alhuzma,
            hajm: 4,
            milaffat: vec![MalafMudraj {
                masar: "muhammil.dll".to_owned(),
                hajm: 4,
                sha256: hex_saghir(Sha256::digest(b"bayt").as_slice()),
            }],
        }
    }

    /// Writes a manifest and a catalogue into a root, the second vouched for by
    /// the first, exactly as staging writes them.
    fn uktub(
        jidhr: &std::path::Path,
        milaffat: Vec<MalafMudraj>,
        taqm: &str,
        asmaa: &[&str],
    ) -> NatijatIkhtibar {
        let fihris = FihrisMukawwinat {
            mukhattat: MUKHATTAT_FIHRIS_MADUM,
            isdar: "1.0.1".to_owned(),
            hadaf: "x86_64-unknown-linux-gnu".to_owned(),
            mukawwinat: asmaa
                .iter()
                .map(|ism| mukawwin_mufahras(ism, taqm == TAQM_KAMIL))
                .collect(),
        };
        let nass = serde_json::to_vec_pretty(&fihris)?;
        std::fs::write(jidhr.join(ISM_MALAF_FIHRIS), &nass)?;

        let bayan = BayanMukawwinat {
            mukhattat: MUKHATTAT_MADUM,
            isdar: "1.0.1".to_owned(),
            hadaf: "x86_64-unknown-linux-gnu".to_owned(),
            milaffat,
            taqm: taqm.to_owned(),
            fihris: Some(BasmatFihris {
                hajm: nass.len() as u64,
                sha256: hex_saghir(Sha256::digest(&nass).as_slice()),
            }),
        };
        std::fs::write(
            jidhr.join(ISM_MALAF_BAYAN),
            serde_json::to_vec_pretty(&bayan)?,
        )?;
        Ok(())
    }

    #[test]
    fn fihris_yuqra_hin_tutabiq_basmatuh() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        uktub(
            masrah.path(),
            Vec::new(),
            TAQM_NAHIF,
            &["bepinex/windows/il2cpp-hadith-x64"],
        )?;

        let fihris = iqra_fihris(masrah.path())?.ok_or("a staged catalogue must read")?;
        assert_eq!(fihris.mukawwinat.len(), 1);
        assert!(
            fihris
                .mukawwin("bepinex/windows/il2cpp-hadith-x64")
                .is_some(),
            "the catalogue must find the component it lists"
        );
        Ok(())
    }

    #[test]
    fn fihris_yurfad_hin_tataghayyar_bayta_wahida() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        uktub(
            masrah.path(),
            Vec::new(),
            TAQM_NAHIF,
            &["mudkhal/windows/x64"],
        )?;

        // One byte, inside a value, keeping the length: the size check passes
        // and only the hash can catch it. This is the whole trust story — a
        // fetch reads its hashes out of this document, so a document nothing
        // vouches for must never be parsed.
        let masar = masrah.path().join(ISM_MALAF_FIHRIS);
        let nass = std::fs::read_to_string(&masar)?;
        let mabtur = nass.replacen("x86_64-unknown-linux-gnu", "x86_64-unknown-linux-gnU", 1);
        assert_eq!(mabtur.len(), nass.len());
        std::fs::write(&masar, mabtur)?;

        assert!(
            iqra_fihris(masrah.path()).is_err(),
            "a catalogue whose bytes changed must be refused, not parsed"
        );
        Ok(())
    }

    #[test]
    fn bayan_bila_fihris_yuqra_wa_la_shay_qabil_liljalb() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        // A manifest as every build before the catalogue existed wrote one.
        let qadeem = r#"{"mukhattat":1,"isdar":"1.0.0","hadaf":"x86_64-unknown-linux-gnu",
            "milaffat":[{"masar":"mukawwinat/mudkhal/windows/x64/version.dll","hajm":0,
            "sha256":"0000000000000000000000000000000000000000000000000000000000000000"}]}"#;
        std::fs::write(masrah.path().join(ISM_MALAF_BAYAN), qadeem)?;

        let bayan =
            iqra_bayan(masrah.path())?.ok_or("a manifest without the new fields must read")?;
        assert_eq!(bayan.taqm, TAQM_KAMIL);
        assert!(bayan.fihris.is_none());
        assert!(
            iqra_fihris(masrah.path())?.is_none(),
            "a manifest that names no catalogue makes every absence final"
        );
        Ok(())
    }

    #[test]
    fn mukawwin_ghayr_mahmul_yuqal_annahu_qabil_liljalb() -> NatijatIkhtibar {
        let masrah = tempfile::tempdir()?;
        // A slim store: the manifest lists the loader it carries and says
        // nothing about the IL2CPP component, which is what makes
        // `kamil_hasab_bayan` refuse that one by name — the refusal this builds
        // on rather than replaces.
        std::fs::create_dir_all(masrah.path().join("mudkhal/windows/x64"))?;
        std::fs::write(
            masrah.path().join("mudkhal/windows/x64/version.dll"),
            b"bayt",
        )?;
        uktub(
            masrah.path(),
            vec![MalafMudraj {
                masar: "mukawwinat/mudkhal/windows/x64/version.dll".to_owned(),
                hajm: 4,
                sha256: hex_saghir(Sha256::digest(b"bayt").as_slice()),
            }],
            TAQM_NAHIF,
            &["bepinex/windows/il2cpp-hadith-x64"],
        )?;
        let fihris = iqra_fihris(masrah.path())?;

        assert_eq!(
            hala_mukawwin(masrah.path(), fihris.as_ref(), "mudkhal/windows/x64")?,
            HalatMukawwin::Mawjud
        );
        assert_eq!(
            hala_mukawwin(
                masrah.path(),
                fihris.as_ref(),
                "bepinex/windows/il2cpp-hadith-x64"
            )?,
            HalatMukawwin::QabilLiljalb { hajm: 4, adad: 1 }
        );
        assert_eq!(
            hala_mukawwin(
                masrah.path(),
                fihris.as_ref(),
                "bepinex/windows/mono-hadith-x64"
            )?,
            HalatMukawwin::Majhul
        );
        Ok(())
    }

    #[test]
    fn kul_hala_tanti_jumlatayn_ghayr_farightayn() {
        for hala in [
            HalatMukawwin::Mawjud,
            HalatMukawwin::QabilLiljalb {
                hajm: 77_916_458,
                adad: 231,
            },
            HalatMukawwin::Majhul,
        ] {
            let ism = "bepinex/windows/il2cpp-hadith-x64";
            assert!(hala.injilizi(ism).contains(ism));
            assert!(hala.arabi(ism).contains(ism));
        }
    }
}
