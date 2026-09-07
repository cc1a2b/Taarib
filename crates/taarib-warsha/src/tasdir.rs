//! The shared bundle: a hashed, per-member zstd-compressed directory that round-trips whole.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use taarib_istikhraj::mashru::{MALAF_MASHRU, MALAF_NUSUS, Mashru, MashruMaftuh};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::nass::MudkhalNass;
use taarib_taqdeem::taaliq::Taaliq;
use taarib_tarjama::masrad::MustalahMasrad;
use taarib_usus::khata::Khata;
use taarib_usus::mukhattat::DhuMukhattat;
use taarib_usus::{masarat, mukhattat};

use crate::khata::{KhataWarsha, NatijatWarsha};
use crate::tarikh::TarikhMashru;
use crate::tawzi::{AqfalMashru, Tawzi};

/// The bundle manifest's schema version.
pub const ISDAR_HUZMA: u32 = 1;

/// The manifest file at the bundle root, written uncompressed.
pub const MALAF_BAYAN: &str = "bayan.json";

/// The suffix every compressed member carries.
pub const LAHIQAT_DAGHT: &str = "zst";

/// zstd level for members: the library default; a bundle is written once and read once.
pub const MUSTAWA_DAGHT: i32 = 3;

/// The last-export snapshot beside the project, the merge's common ancestor.
pub const MALAF_ASLAF: &str = "aslaf_tasdir.jsonl";

/// The glossary member.
pub const UDW_MASRAD: &str = "masrad.json";

/// The translation-memory database member.
pub const UDW_DHAKIRA: &str = "dhakira.db";

/// The comments member.
pub const UDW_TAALIQAT: &str = "taaliqat.json";

/// The assignments member.
pub const UDW_TAWZI: &str = "tawzi.json";

/// The locks member.
pub const UDW_AQFAL: &str = "aqfal.json";

/// The history member.
pub const UDW_TARIKH: &str = "tarikh.json";

/// Every member a bundle carries, in the order it is written.
pub const ADAA_HUZMA: [&str; 8] = [
    MALAF_MASHRU,
    MALAF_NUSUS,
    UDW_MASRAD,
    UDW_DHAKIRA,
    UDW_TAALIQAT,
    UDW_TAWZI,
    UDW_AQFAL,
    UDW_TARIKH,
];

/// Who exported a bundle, from which machine, when, and from which project.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AslHuzma {
    /// Who exported.
    pub musaddir: MusahimId,
    /// The exporting machine's label, as its owner names it.
    pub jihaz: String,
    /// When, RFC 3339, supplied by the caller.
    pub waqt: String,
    /// The game the project belongs to.
    pub luba: LubaId,
    /// The project schema the header was written with.
    pub mukhattat_mashru: u32,
    /// The game's display name, for the import screen.
    pub ism_luba: String,
}

impl AslHuzma {
    /// An origin record for one export of one project.
    #[must_use]
    pub fn jadeed(musaddir: MusahimId, jihaz: String, waqt: String, rasm: &Mashru) -> Self {
        Self {
            musaddir,
            jihaz,
            waqt,
            luba: rasm.luba,
            // The schema a loaded project is at, which is always the one this
            // build reads: `mukhattat` migrates a file forward before it ever
            // becomes a `Mashru`, and the version itself is that module's
            // metadata rather than a field of the project.
            mukhattat_mashru: <Mashru as DhuMukhattat>::ISDAR,
            ism_luba: rasm.ism_luba.clone(),
        }
    }

    /// Whether this origin names the given project.
    #[must_use]
    pub fn yutabiq(&self, rasm: &Mashru) -> bool {
        self.luba == rasm.luba && self.mukhattat_mashru == <Mashru as DhuMukhattat>::ISDAR
    }
}

/// One member of the bundle: its name, its hash, and its size before
/// compression.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UdwHuzma {
    /// The member's name, without the compression suffix.
    pub ism: String,
    /// BLAKE3 of the uncompressed bytes, lowercase hex.
    pub basma: String,
    /// The uncompressed length in bytes.
    pub hajm: u64,
}

/// The manifest at the bundle root.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BayanHuzma {
    /// The bundle schema this manifest was written with.
    pub mukhattat: u32,
    /// Who exported the bundle, and from which project.
    pub asl: AslHuzma,
    /// Every member, with its hash.
    pub adaa: Vec<UdwHuzma>,
}

impl BayanHuzma {
    /// The recorded member of one name.
    #[must_use]
    pub fn udw(&self, ism: &str) -> Option<&UdwHuzma> {
        self.adaa.iter().find(|udw| udw.ism == ism)
    }
}

/// A bundle on disk: its directory and the manifest it was written with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuzmatWarsha {
    jidhr: PathBuf,
    bayan: BayanHuzma,
}

impl HuzmatWarsha {
    /// The bundle's directory.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// The manifest.
    #[must_use]
    pub const fn bayan(&self) -> &BayanHuzma {
        &self.bayan
    }
}

/// The pieces exported alongside the project's own two files.
#[derive(Debug, Clone, Copy)]
pub struct AjzaHuzma<'a> {
    /// The glossary in force.
    pub masrad: &'a [MustalahMasrad],
    /// The comments travelling with the project, as values.
    pub taaliqat: &'a [Taaliq],
    /// The assignments.
    pub tawzi: &'a Tawzi,
    /// The locks.
    pub aqfal: &'a AqfalMashru,
    /// The history.
    pub tarikh: &'a TarikhMashru,
    /// The translation-memory database file, when the machine has one.
    pub dhakira: Option<&'a Path>,
}

/// Everything a bundle carries, loaded and hash-verified.
#[derive(Debug, Clone)]
pub struct MuhtawaHuzma {
    /// The manifest, origin record included.
    pub bayan: BayanHuzma,
    /// The project header exactly as the exporter's copy held it.
    pub rasm: Mashru,
    /// The strings — the `hum` side of [`crate::damj::damj`].
    pub nusus: Vec<MudkhalNass>,
    /// How many JSONL lines were malformed and skipped.
    pub talifa: usize,
    /// The glossary terms.
    pub masrad: Vec<MustalahMasrad>,
    /// The comments, deserialized and never re-constructed.
    pub taaliqat: Vec<Taaliq>,
    /// The assignments.
    pub tawzi: Tawzi,
    /// The locks.
    pub aqfal: AqfalMashru,
    /// The history.
    pub tarikh: TarikhMashru,
    /// The translation-memory database file's raw bytes.
    pub dhakira: Vec<u8>,
}

impl MuhtawaHuzma {
    /// Refuses these pieces for merging into a project they do not belong to.
    ///
    /// # Errors
    ///
    /// [`KhataWarsha::MashruMukhtalif`] when the bundle's recorded project
    /// identity is not the open project's.
    pub fn tahaqquq_tatabuq(&self, rasm: &Mashru) -> NatijatWarsha<()> {
        if self.bayan.asl.yutabiq(rasm) {
            Ok(())
        } else {
            Err(KhataWarsha::MashruMukhtalif)
        }
    }

    /// Writes the bundled translation-memory file where the caller chooses,
    /// atomically.
    ///
    /// # Errors
    ///
    /// [`KhataWarsha::Mawrid`] when the write fails.
    pub fn sarrih_dhakira(&self, ila: &Path) -> NatijatWarsha<()> {
        masarat::kitaba_dharra(ila, &self.dhakira).map_err(min_usus)
    }
}

/// Exports one project and its collaboration pieces as a bundle directory.
///
/// Pending strings are committed first, so what is sent is exactly what was
/// worked on, and the exported table is recorded beside the project as the next
/// merge's common ancestor.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when the project cannot commit or a piece cannot
/// be serialized or written, and [`KhataWarsha::KhataMalaf`] when a source
/// file cannot be read or a member cannot be compressed.
pub fn saddir(
    mashru: &mut MashruMaftuh,
    ajza: &AjzaHuzma<'_>,
    musaddir: MusahimId,
    jihaz: String,
    waqt: String,
    ila: &Path,
) -> NatijatWarsha<HuzmatWarsha> {
    mashru.adif_dufa().map_err(|khata| KhataWarsha::Mawrid {
        sabab: khata.to_string(),
    })?;
    masarat::insha_mujallad(ila).map_err(min_usus)?;

    let masar_rasm = mashru.jidhr().join(MALAF_MASHRU);
    let bayt_rasm = std::fs::read(&masar_rasm).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: masar_rasm,
        amal: "reading the project header for export",
        sabab,
    })?;
    let bayt_nusus = iqra_in_wujid(&mashru.jidhr().join(MALAF_NUSUS))?;

    let bayt_masrad = ila_json(ajza.masrad)?;
    let bayt_taaliqat = ila_json(ajza.taaliqat)?;
    let bayt_tawzi = ila_json(ajza.tawzi)?;
    let bayt_aqfal = ila_json(ajza.aqfal)?;
    let bayt_tarikh = ila_json(ajza.tarikh)?;
    let bayt_dhakira = match ajza.dhakira {
        Some(masar) => std::fs::read(masar).map_err(|sabab| KhataWarsha::KhataMalaf {
            masar: masar.to_path_buf(),
            amal: "reading the translation memory for export",
            sabab,
        })?,
        None => Vec::new(),
    };

    let azwaj: [(&str, &[u8]); 8] = [
        (MALAF_MASHRU, &bayt_rasm),
        (MALAF_NUSUS, &bayt_nusus),
        (UDW_MASRAD, &bayt_masrad),
        (UDW_DHAKIRA, &bayt_dhakira),
        (UDW_TAALIQAT, &bayt_taaliqat),
        (UDW_TAWZI, &bayt_tawzi),
        (UDW_AQFAL, &bayt_aqfal),
        (UDW_TARIKH, &bayt_tarikh),
    ];
    let mut adaa = Vec::with_capacity(azwaj.len());
    for (ism, bayt) in azwaj {
        adaa.push(iktub_udw(ila, ism, bayt)?);
    }

    let asl = AslHuzma::jadeed(musaddir, jihaz, waqt, mashru.rasm());
    let bayan = BayanHuzma {
        mukhattat: ISDAR_HUZMA,
        asl,
        adaa,
    };
    let bayt_bayan = serde_json::to_vec_pretty(&bayan).map_err(|sabab| KhataWarsha::Mawrid {
        sabab: sabab.to_string(),
    })?;
    masarat::kitaba_dharra(&ila.join(MALAF_BAYAN), &bayt_bayan).map_err(min_usus)?;

    masarat::kitaba_dharra(&mashru.jidhr().join(MALAF_ASLAF), &bayt_nusus).map_err(min_usus)?;

    Ok(HuzmatWarsha {
        jidhr: ila.to_path_buf(),
        bayan,
    })
}

/// Loads a bundle directory, verifying the schema and every member hash.
///
/// # Errors
///
/// [`KhataWarsha::KhataMalaf`] when the manifest or a member file cannot be
/// read, [`KhataWarsha::IsdarMajhul`] when the bundle schema is newer than
/// this build, and [`KhataWarsha::HuzmaTalifa`] when the manifest does not
/// parse, a member is absent, fails decompression, disagrees with its
/// recorded hash or length, or does not deserialize.
pub fn istawrid(jidhr: &Path) -> NatijatWarsha<MuhtawaHuzma> {
    let masar_bayan = jidhr.join(MALAF_BAYAN);
    let bayt_bayan = std::fs::read(&masar_bayan).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: masar_bayan,
        amal: "reading the bundle manifest",
        sabab,
    })?;
    let bayan: BayanHuzma =
        serde_json::from_slice(&bayt_bayan).map_err(|sabab| KhataWarsha::HuzmaTalifa {
            sabab: format!("the manifest does not parse: {sabab}"),
        })?;
    if bayan.mukhattat > ISDAR_HUZMA {
        return Err(KhataWarsha::IsdarMajhul {
            wujid: bayan.mukhattat,
            madum: ISDAR_HUZMA,
        });
    }

    let mut adaa: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
    for ism in ADAA_HUZMA {
        let _ = adaa.insert(ism, iqra_udw(jidhr, &bayan, ism)?);
    }

    let bayt_rasm = adaa.remove(MALAF_MASHRU).unwrap_or_default();
    let rasm: Mashru = mukhattat::iqra(&bayt_rasm).map_err(|khata| KhataWarsha::HuzmaTalifa {
        sabab: format!("the project header does not read: {}", khata.injilizi),
    })?;

    let (nusus, talifa) = iqra_jsonl(&adaa.remove(MALAF_NUSUS).unwrap_or_default());
    let masrad = min_json(UDW_MASRAD, &adaa.remove(UDW_MASRAD).unwrap_or_default())?;
    let taaliqat = min_json(UDW_TAALIQAT, &adaa.remove(UDW_TAALIQAT).unwrap_or_default())?;
    let tawzi = min_json(UDW_TAWZI, &adaa.remove(UDW_TAWZI).unwrap_or_default())?;
    let aqfal = min_json(UDW_AQFAL, &adaa.remove(UDW_AQFAL).unwrap_or_default())?;
    let tarikh = min_json(UDW_TARIKH, &adaa.remove(UDW_TARIKH).unwrap_or_default())?;
    let dhakira = adaa.remove(UDW_DHAKIRA).unwrap_or_default();

    Ok(MuhtawaHuzma {
        bayan,
        rasm,
        nusus,
        talifa,
        masrad,
        taaliqat,
        tawzi,
        aqfal,
        tarikh,
        dhakira,
    })
}

/// Exports, re-imports, and compares every member hash, so a bundle is
/// proven to round-trip before anybody sends it.
///
/// # Errors
///
/// As [`saddir`], plus [`KhataWarsha::TabadulGhayrMutabiq`] when the
/// re-import fails or yields a manifest other than the one written.
pub fn saddir_wa_tahaqqaq(
    mashru: &mut MashruMaftuh,
    ajza: &AjzaHuzma<'_>,
    musaddir: MusahimId,
    jihaz: String,
    waqt: String,
    ila: &Path,
) -> NatijatWarsha<HuzmatWarsha> {
    let huzma = saddir(mashru, ajza, musaddir, jihaz, waqt, ila)?;
    let muhtawa = istawrid(huzma.jidhr()).map_err(|khata| KhataWarsha::TabadulGhayrMutabiq {
        sigha: "huzma",
        sabab: khata.to_string(),
    })?;
    if muhtawa.bayan != huzma.bayan {
        return Err(KhataWarsha::TabadulGhayrMutabiq {
            sigha: "huzma",
            sabab: "the re-imported manifest is not the one written".to_owned(),
        });
    }
    Ok(huzma)
}

/// Reads the last-export snapshot beside a project — the `aslaf` argument of
/// [`crate::damj::damj`] — with its malformed-line count; [`None`] when no
/// export has been recorded and the merge runs two-way.
///
/// # Errors
///
/// [`KhataWarsha::KhataMalaf`] when the snapshot exists and cannot be read.
pub fn aslaf_mashru(jidhr: &Path) -> NatijatWarsha<Option<(Vec<MudkhalNass>, usize)>> {
    let masar = jidhr.join(MALAF_ASLAF);
    if !masar.is_file() {
        return Ok(None);
    }
    let bayt = std::fs::read(&masar).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar,
        amal: "reading the shared-ancestor snapshot",
        sabab,
    })?;
    Ok(Some(iqra_jsonl(&bayt)))
}

/// Records the given table as the shared point beside the project — what a
/// caller does right after committing a finished merge.
///
/// # Errors
///
/// [`KhataWarsha::Mawrid`] when a row cannot be serialized or the snapshot
/// cannot be written.
pub fn sajjil_aslaf(jidhr: &Path, nusus: &[MudkhalNass]) -> NatijatWarsha<()> {
    let mut bayt = Vec::with_capacity(nusus.len().saturating_mul(256));
    for mudkhal in nusus {
        let satr = serde_json::to_vec(mudkhal).map_err(|sabab| KhataWarsha::Mawrid {
            sabab: sabab.to_string(),
        })?;
        bayt.extend_from_slice(&satr);
        bayt.push(b'\n');
    }
    masarat::kitaba_dharra(&jidhr.join(MALAF_ASLAF), &bayt).map_err(min_usus)
}

fn min_usus(khata: Khata) -> KhataWarsha {
    KhataWarsha::Mawrid {
        sabab: khata.injilizi,
    }
}

fn ila_json<T: serde::Serialize + ?Sized>(qeema: &T) -> NatijatWarsha<Vec<u8>> {
    serde_json::to_vec(qeema).map_err(|sabab| KhataWarsha::Mawrid {
        sabab: sabab.to_string(),
    })
}

fn min_json<T: serde::de::DeserializeOwned>(ism: &str, bayt: &[u8]) -> NatijatWarsha<T> {
    serde_json::from_slice(bayt).map_err(|sabab| KhataWarsha::HuzmaTalifa {
        sabab: format!("member {ism} does not parse: {sabab}"),
    })
}

fn masar_udw(jidhr: &Path, ism: &str) -> PathBuf {
    jidhr.join(format!("{ism}.{LAHIQAT_DAGHT}"))
}

fn iqra_in_wujid(masar: &Path) -> NatijatWarsha<Vec<u8>> {
    if !masar.is_file() {
        return Ok(Vec::new());
    }
    std::fs::read(masar).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: masar.to_path_buf(),
        amal: "reading the project strings for export",
        sabab,
    })
}

fn iktub_udw(jidhr: &Path, ism: &str, bayt: &[u8]) -> NatijatWarsha<UdwHuzma> {
    let masar = masar_udw(jidhr, ism);
    let madghut =
        zstd::encode_all(bayt, MUSTAWA_DAGHT).map_err(|sabab| KhataWarsha::KhataMalaf {
            masar: masar.clone(),
            amal: "compressing a bundle member",
            sabab,
        })?;
    masarat::kitaba_dharra(&masar, &madghut).map_err(min_usus)?;
    Ok(UdwHuzma {
        ism: ism.to_owned(),
        basma: blake3::hash(bayt).to_hex().to_string(),
        hajm: u64::try_from(bayt.len()).unwrap_or(u64::MAX),
    })
}

fn iqra_udw(jidhr: &Path, bayan: &BayanHuzma, ism: &str) -> NatijatWarsha<Vec<u8>> {
    let Some(udw) = bayan.udw(ism) else {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!("the manifest names no member {ism}"),
        });
    };
    let masar = masar_udw(jidhr, ism);
    if !masar.is_file() {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!("member {ism} is missing from the bundle"),
        });
    }
    let madghut = std::fs::read(&masar).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar,
        amal: "reading a bundle member",
        sabab,
    })?;
    let bayt = zstd::decode_all(madghut.as_slice()).map_err(|sabab| KhataWarsha::HuzmaTalifa {
        sabab: format!("member {ism} does not decompress: {sabab}"),
    })?;
    if u64::try_from(bayt.len()).unwrap_or(u64::MAX) != udw.hajm {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!(
                "member {ism} is {} bytes where the manifest records {}",
                bayt.len(),
                udw.hajm
            ),
        });
    }
    if blake3::hash(&bayt).to_hex().to_string() != udw.basma {
        return Err(KhataWarsha::HuzmaTalifa {
            sabab: format!("member {ism} does not match its recorded hash"),
        });
    }
    Ok(bayt)
}

/// The rows that read and how many lines did not — one reader for every JSONL
/// table this crate meets, so a bundle and a snapshot cannot disagree with the
/// live file about what counts as damaged.
fn iqra_jsonl(bayt: &[u8]) -> (Vec<MudkhalNass>, usize) {
    let (madakhil, talifa) = crate::salama::hallil_jsonl(bayt);
    (madakhil, talifa.len())
}
