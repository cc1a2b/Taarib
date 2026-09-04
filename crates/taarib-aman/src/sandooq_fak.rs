//! Quarantined unpacking: the sealed container is copied in only after it is proven safe.

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use taarib_ruqaa::SIHR;
use taarib_usus::masarat;

use crate::khata::{KhataAman, NatijatAman};

/// The largest a single quarantined entry may be.
// 256 MiB: far above any real Arabic patch, low enough to refuse disk-fill and bound the copy.
pub const AQSA_HAJM_MADKHAL: u64 = 256 * 1024 * 1024;

/// The largest total an unpack may write into quarantine.
// Equal to the per-entry cap: one sealed container is the whole download.
pub const AQSA_HAJM_KULLI: u64 = 256 * 1024 * 1024;

/// The most entries one package may unpack to.
// A sealed .ruqaa is exactly one file; a second entry is an attack, not a package.
pub const AQSA_ADAD_MADAKHIL: usize = 1;

/// A validated relative destination inside the quarantine directory.
///
/// The only path type the unpacker writes to. Holds the normalized relative
/// form; a parent, root or drive-prefix component is unrepresentable because
/// [`MadkhalHajr::jadeed`] refuses it. `Deserialize` routes through that
/// constructor so package metadata cannot bypass validation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct MadkhalHajr {
    nisbi: String,
}

impl MadkhalHajr {
    /// Validates one relative quarantine entry name.
    ///
    /// # Errors
    ///
    /// [`KhataAman::MadkhalKharij`] for a parent, root or drive-prefix component
    /// that escapes the quarantine; [`KhataAman::FakFashil`] for an empty name, a
    /// NUL byte, a non-UTF-8 component, a reserved device name, or a component
    /// ending in a dot or space.
    pub fn jadeed(khaam: &str) -> NatijatAman<Self> {
        let mushawwah = |sabab: &str| KhataAman::FakFashil {
            sabab: format!("refused quarantine entry name {khaam:?}: {sabab}"),
        };
        if khaam.is_empty() {
            return Err(mushawwah("the name is empty"));
        }
        if khaam.contains('\0') {
            return Err(mushawwah("the name contains a NUL byte"));
        }

        // Backslashes convert first: `..\\x` traverses on Windows, past a walk-time check.
        let mubaddal = khaam.replace('\\', "/");
        let mut ajza: Vec<&str> = Vec::new();
        for juz in Path::new(&mubaddal).components() {
            match juz {
                Component::Normal(ism) => {
                    let Some(nass) = ism.to_str() else {
                        return Err(mushawwah("a path component is not valid UTF-8"));
                    };
                    if ism_jihaz(nass) {
                        return Err(mushawwah(
                            "a component is a reserved device name or ends in a dot or space",
                        ));
                    }
                    ajza.push(nass);
                }
                Component::CurDir => {}
                Component::ParentDir
                | Component::RootDir
                | Component::Prefix(_) => {
                    return Err(KhataAman::MadkhalKharij { madkhal: khaam.to_owned() });
                }
            }
        }
        if ajza.is_empty() {
            return Err(mushawwah("the name resolves to the quarantine root itself"));
        }
        Ok(Self { nisbi: ajza.join("/") })
    }

    /// The normalized relative form, forward-slashed.
    #[must_use]
    pub fn nisbi(&self) -> &str {
        &self.nisbi
    }

    /// The file name component.
    #[must_use]
    pub fn ism(&self) -> &str {
        self.nisbi.rsplit('/').next().unwrap_or(&self.nisbi)
    }

    /// The case-folded key for case-insensitive-collision detection.
    #[must_use]
    pub fn mutamathil_saghir(&self) -> String {
        self.nisbi.to_lowercase()
    }
}

impl fmt::Display for MadkhalHajr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.nisbi)
    }
}

impl From<MadkhalHajr> for String {
    fn from(madkhal: MadkhalHajr) -> Self {
        madkhal.nisbi
    }
}

impl TryFrom<String> for MadkhalHajr {
    type Error = KhataAman;

    fn try_from(khaam: String) -> NatijatAman<Self> {
        Self::jadeed(&khaam)
    }
}

impl TryFrom<&str> for MadkhalHajr {
    type Error = KhataAman;

    fn try_from(khaam: &str) -> NatijatAman<Self> {
        Self::jadeed(khaam)
    }
}

fn ism_jihaz(nass: &str) -> bool {
    if nass.ends_with('.') || nass.ends_with(' ') {
        return true;
    }
    let jidhr = nass.split('.').next().unwrap_or(nass);
    matches!(
        jidhr.to_ascii_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
            | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    )
}

/// The size and count budget one unpack may not exceed, refused before a write.
#[derive(Debug, Clone)]
pub struct MizaniyatHajr {
    majmu: u64,
    adad: usize,
}

impl Default for MizaniyatHajr {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl MizaniyatHajr {
    /// An empty budget.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self { majmu: 0, adad: 0 }
    }

    /// Charges one entry of `hajm` bytes against the budget.
    ///
    /// # Errors
    ///
    /// [`KhataAman::FakFashil`] when the entry, the entry count, or the running
    /// total would exceed its ceiling.
    pub fn hisab(&mut self, hajm: u64) -> NatijatAman<()> {
        if hajm > AQSA_HAJM_MADKHAL {
            return Err(KhataAman::FakFashil {
                sabab: format!(
                    "an entry of {hajm} bytes exceeds the per-entry ceiling of \
                     {AQSA_HAJM_MADKHAL} bytes"
                ),
            });
        }
        if self.adad >= AQSA_ADAD_MADAKHIL {
            return Err(KhataAman::FakFashil {
                sabab: format!(
                    "the package holds more than the {AQSA_ADAD_MADAKHIL} permitted entries"
                ),
            });
        }
        let majmu = self.majmu.checked_add(hajm).ok_or_else(|| KhataAman::FakFashil {
            sabab: "the total unpacked size overflowed".to_owned(),
        })?;
        if majmu > AQSA_HAJM_KULLI {
            return Err(KhataAman::FakFashil {
                sabab: format!(
                    "the total unpacked size {majmu} exceeds the ceiling of {AQSA_HAJM_KULLI} bytes"
                ),
            });
        }
        self.majmu = majmu;
        self.adad += 1;
        Ok(())
    }

    /// The bytes charged so far.
    #[must_use]
    pub const fn majmu(&self) -> u64 {
        self.majmu
    }

    /// The entries charged so far.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.adad
    }
}

/// The case-folded names already placed, guarding the case-insensitive overwrite attack.
#[derive(Debug, Clone, Default)]
pub struct SijillTasadum {
    raut: BTreeSet<String>,
}

impl SijillTasadum {
    /// An empty registry.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Records one entry, refusing a case-insensitive collision with a prior one.
    ///
    /// # Errors
    ///
    /// [`KhataAman::FakFashil`] when another entry already folds to the same name.
    pub fn sajjil(&mut self, madkhal: &MadkhalHajr) -> NatijatAman<()> {
        // A case-insensitive filesystem folds these to one name; a second is an overwrite.
        let miftah = madkhal.mutamathil_saghir();
        if self.raut.insert(miftah.clone()) {
            Ok(())
        } else {
            Err(KhataAman::FakFashil {
                sabab: format!(
                    "two entries collide under case-insensitive comparison at {miftah:?}"
                ),
            })
        }
    }

    /// The number of distinct entries recorded.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.raut.len()
    }
}

// Archive shapes refused by name. The product ingests one thing: the sealed .ruqaa.
// `as_slice` unifies the differing magic lengths to one element type the array can hold.
const MARFUD_BIDAYA: &[(&[u8], &str)] = &[
    (b"PK\x03\x04".as_slice(), "a ZIP archive"),
    (b"PK\x05\x06".as_slice(), "an empty ZIP archive"),
    (b"PK\x07\x08".as_slice(), "a spanned ZIP archive"),
    (b"\x1f\x8b".as_slice(), "a gzip stream"),
    (b"BZh".as_slice(), "a bzip2 stream"),
    (b"\xfd7zXZ\x00".as_slice(), "an xz stream"),
    (b"\x28\xb5\x2f\xfd".as_slice(), "a raw zstd stream"),
    (b"7z\xbc\xaf\x27\x1c".as_slice(), "a 7-Zip archive"),
    (b"Rar!\x1a\x07".as_slice(), "a RAR archive"),
];

fn naw_marfud(bayt: &[u8]) -> Option<&'static str> {
    for &(sihr, ism) in MARFUD_BIDAYA {
        if bayt.starts_with(sihr) {
            return Some(ism);
        }
    }
    // ustar lives at offset 257 of a POSIX tar header, not at the start.
    if bayt.get(257..262) == Some(b"ustar".as_slice()) {
        return Some("a tar archive");
    }
    None
}

fn tahaqquq_hawiya(bayt: &[u8]) -> NatijatAman<()> {
    if bayt.starts_with(&SIHR) {
        return Ok(());
    }
    if let Some(ism) = naw_marfud(bayt) {
        return Err(KhataAman::FakFashil {
            sabab: format!("{ism} is not a package; only a sealed .ruqaa container is accepted"),
        });
    }
    Err(KhataAman::FakFashil {
        sabab: "not a sealed .ruqaa container; its leading bytes are not the TRQ1 magic"
            .to_owned(),
    })
}

/// One entry that was written into quarantine.
#[derive(Debug, Clone)]
pub struct MadkhalMakhruj {
    madkhal: MadkhalHajr,
    hajm: u64,
    mutlaq: PathBuf,
}

impl MadkhalMakhruj {
    /// The validated relative name inside quarantine.
    #[must_use]
    pub const fn madkhal(&self) -> &MadkhalHajr {
        &self.madkhal
    }

    /// The number of bytes written.
    #[must_use]
    pub const fn hajm(&self) -> u64 {
        self.hajm
    }

    /// The canonical absolute path, proven inside quarantine.
    #[must_use]
    pub fn mutlaq(&self) -> &Path {
        &self.mutlaq
    }
}

/// Everything one unpack placed in quarantine, ready for verification.
#[derive(Debug, Clone)]
pub struct MuhtawaHajr {
    jidhr: PathBuf,
    madakhil: Vec<MadkhalMakhruj>,
    hajm_kulli: u64,
}

impl MuhtawaHajr {
    /// The quarantine root the entries live under.
    #[must_use]
    pub fn jidhr(&self) -> &Path {
        &self.jidhr
    }

    /// The entries written, in the order they were placed.
    #[must_use]
    pub fn madakhil(&self) -> &[MadkhalMakhruj] {
        &self.madakhil
    }

    /// The number of entries written.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// The total bytes written across all entries.
    #[must_use]
    pub const fn hajm_kulli(&self) -> u64 {
        self.hajm_kulli
    }

    /// The sealed container's path, which is the single entry verification opens.
    #[must_use]
    pub fn masar_masbur(&self) -> Option<&Path> {
        self.madakhil.first().map(MadkhalMakhruj::mutlaq)
    }
}

fn tul(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

fn iqra_masdar_amin(masdar: &Path) -> NatijatAman<Vec<u8>> {
    let wasf = fs::symlink_metadata(masdar).map_err(|sabab| KhataAman::KhataMalaf {
        masar: masdar.to_path_buf(),
        amal: "reading the package metadata",
        sabab,
    })?;
    if wasf.file_type().is_symlink() {
        return Err(KhataAman::FakFashil {
            sabab: "the package path is a symbolic link, which is not a package".to_owned(),
        });
    }
    if wasf.is_dir() {
        return Err(KhataAman::FakFashil {
            sabab: "a directory is not a sealed .ruqaa package".to_owned(),
        });
    }

    let malaf = File::open(masdar).map_err(|sabab| KhataAman::KhataMalaf {
        masar: masdar.to_path_buf(),
        amal: "opening the package",
        sabab,
    })?;
    // Measure the open handle, not the path: a swap after the metadata check does not lie here.
    let bayanat = malaf.metadata().map_err(|sabab| KhataAman::KhataMalaf {
        masar: masdar.to_path_buf(),
        amal: "measuring the package",
        sabab,
    })?;
    if !bayanat.is_file() {
        return Err(KhataAman::FakFashil {
            sabab: "the package path is not a regular file".to_owned(),
        });
    }
    if bayanat.len() > AQSA_HAJM_MADKHAL {
        return Err(KhataAman::FakFashil {
            sabab: format!(
                "the package is {} bytes, past the {AQSA_HAJM_MADKHAL}-byte per-entry ceiling",
                bayanat.len()
            ),
        });
    }

    let mut bayt = Vec::with_capacity(usize::try_from(bayanat.len()).unwrap_or(0));
    (&malaf)
        .take(AQSA_HAJM_MADKHAL + 1)
        .read_to_end(&mut bayt)
        .map_err(|sabab| KhataAman::KhataMalaf {
            masar: masdar.to_path_buf(),
            amal: "reading the package",
            sabab,
        })?;
    if tul(bayt.len()) > AQSA_HAJM_MADKHAL {
        return Err(KhataAman::FakFashil {
            sabab: format!(
                "the package grew past the {AQSA_HAJM_MADKHAL}-byte ceiling while being read"
            ),
        });
    }
    Ok(bayt)
}

/// Places already-read bytes into quarantine under `ism`, validated before writing.
///
/// # Errors
///
/// [`KhataAman::FakFashil`] when `bayt` is not a sealed `.ruqaa`, when `ism` is
/// not a usable relative name, or when a size or count budget is exceeded;
/// [`KhataAman::MadkhalKharij`] when the destination resolves outside quarantine.
pub fn fak_bayt_ila_hajr(bayt: &[u8], ism: &str, jidhr_hajr: &Path) -> NatijatAman<MuhtawaHajr> {
    masarat::insha_mujallad(jidhr_hajr).map_err(|khata| KhataAman::FakFashil {
        sabab: format!("the quarantine directory could not be prepared: {khata}"),
    })?;

    tahaqquq_hawiya(bayt)?;
    let madkhal = MadkhalHajr::jadeed(ism)?;

    let mut mizan = MizaniyatHajr::jadeed();
    mizan.hisab(tul(bayt.len()))?;

    let mut sijill = SijillTasadum::jadeed();
    sijill.sajjil(&madkhal)?;

    // The one join permitted for untrusted input: quarantine_root + validated relative.
    let wijha = masarat::dakhil(jidhr_hajr, madkhal.nisbi()).map_err(|khata| {
        KhataAman::FakFashil { sabab: format!("the quarantine destination was refused: {khata}") }
    })?;

    masarat::kitaba_dharra(&wijha, bayt).map_err(|khata| KhataAman::FakFashil {
        sabab: format!("the package could not be written into quarantine: {khata}"),
    })?;

    // Re-prove containment after the write: a symlink planted between join and write escapes it.
    let Ok(haqiqi) = masarat::tahaqquq_ihtiwa(jidhr_hajr, &wijha) else {
        drop(masarat::hadhf(&wijha));
        return Err(KhataAman::MadkhalKharij { madkhal: madkhal.nisbi().to_owned() });
    };

    let makhruj = MadkhalMakhruj { madkhal, hajm: tul(bayt.len()), mutlaq: haqiqi };
    Ok(MuhtawaHajr {
        jidhr: jidhr_hajr.to_path_buf(),
        hajm_kulli: mizan.majmu(),
        madakhil: vec![makhruj],
    })
}

/// Copies a downloaded package file into quarantine, validated before a byte is
/// written, and returns what landed there.
///
/// # Errors
///
/// [`KhataAman::KhataMalaf`] for an I/O failure reading `masdar`;
/// [`KhataAman::FakFashil`] when `masdar` is not a sealed `.ruqaa` container, is
/// a directory or symbolic link, or exceeds a size or count budget;
/// [`KhataAman::MadkhalKharij`] when the destination resolves outside quarantine.
pub fn fak_ila_hajr(masdar: &Path, jidhr_hajr: &Path) -> NatijatAman<MuhtawaHajr> {
    let bayt = iqra_masdar_amin(masdar)?;
    let Some(ism) = masdar.file_name().and_then(|q| q.to_str()) else {
        return Err(KhataAman::FakFashil {
            sabab: "the package file has no usable name".to_owned(),
        });
    };
    fak_bayt_ila_hajr(&bayt, ism, jidhr_hajr)
}

/// Removes a quarantine directory and everything in it, treating "gone" as success.
///
/// # Errors
///
/// [`KhataAman::FakFashil`] when the directory exists and cannot be removed.
pub fn tanzif_hajr(jidhr_hajr: &Path) -> NatijatAman<()> {
    masarat::hadhf_mujallad(jidhr_hajr).map_err(|khata| KhataAman::FakFashil {
        sabab: format!("the quarantine directory could not be cleared: {khata}"),
    })
}
