//! سلامة النصوص — reading the string file without losing a row in silence, and setting
//! damaged rows aside only by explicit choice.
//!
//! The project's own reader skips a line it cannot parse and returns a count.
//! That is the right thing for one torn tail line, and the wrong thing for
//! everything a caller then does with the count: drop it, show the survivors as
//! the whole project, and write the survivors back over the file. A translator
//! who opened a project with two hundred unreadable rows would have lost them
//! for good by the time they typed one word.
//!
//! So this module reads the same file and keeps everything the project reader
//! throws away — which line, why, and whatever the line's head still says —
//! and it separates three states no screen may confuse: nothing was ever
//! extracted, everything read, and something did not. The rescue is the only
//! thing here that writes, it never runs on its own, and it copies the damaged
//! file byte for byte and verifies the copy before it replaces anything.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_istikhraj::mashru::{MALAF_NUSUS, MashruMaftuh};
use taarib_mustalahat::nass::{MudkhalNass, NassId};
use taarib_usus::masarat;

use crate::khata::{KhataWarsha, NatijatWarsha};

/// The longest opening a damaged line is quoted by, in characters.
pub const AQSA_MUQTATAF: usize = 96;

/// The stem every file the rescue writes carries, so they sort together beside
/// the project and nothing mistakes them for the live table.
pub const WASM_TALIF: &str = "nusus.talif";

/// How many name collisions the rescue tolerates before refusing.
const AQSA_MUHAWALAT_ISM: u32 = 1000;

/// Why one line did not read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababTalaf {
    /// Not UTF-8 — what an append cut mid-write leaves behind.
    Tarmiz,
    /// UTF-8, but not a row: torn JSON, or a shape this build does not read.
    Sigha {
        /// The parser's own words.
        sabab: String,
    },
}

impl SababTalaf {
    /// The reason, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Tarmiz => "ليس UTF-8 صالحًا؛ هذا ما يتركه إلحاقٌ قُطع في منتصفه".to_owned(),
            Self::Sigha { sabab } => format!("ليس صفًّا مقروءًا: {sabab}"),
        }
    }

    /// The reason, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        match self {
            Self::Tarmiz => {
                "not valid UTF-8, which is what an append cut mid-write leaves behind".to_owned()
            },
            Self::Sigha { sabab } => format!("not a readable row: {sabab}"),
        }
    }
}

/// One line that did not read, with whatever its head still says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatrTalif {
    /// The line's number in the file, counting from one.
    pub raqm: usize,
    /// The line's length in bytes.
    pub tul: usize,
    /// Why it did not read.
    pub sabab: SababTalaf,
    /// The string's identity, when the head survived far enough to carry it.
    pub huwiya: Option<NassId>,
    /// The source text, when it survived.
    pub masdar: Option<String>,
    /// The line's opening, decoded lossily, for the eye.
    pub muqtataf: String,
}

/// What a read amounts to — three states, no two of which a screen may confuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatNusus {
    /// No file, or a file with no rows: nothing has been extracted.
    Farigh,
    /// Every line read.
    Salima,
    /// At least one line did not read; the file is somebody's work in an
    /// unknown state.
    Talifa,
}

/// The string file, read whole: every row that read, and every line that did not.
#[derive(Debug, Clone, Default)]
pub struct QiraatNusus {
    /// The rows that read, in file order.
    pub sufuf: Vec<MudkhalNass>,
    /// The lines that did not, in file order.
    pub talifa: Vec<SatrTalif>,
    /// Whether the file exists at all.
    pub mawjud: bool,
    /// The file's length in bytes; zero when absent.
    pub hajm: u64,
}

impl QiraatNusus {
    /// Which of the three states this read is in.
    #[must_use]
    pub const fn hala(&self) -> HalatNusus {
        if !self.talifa.is_empty() {
            HalatNusus::Talifa
        } else if self.sufuf.is_empty() {
            HalatNusus::Farigh
        } else {
            HalatNusus::Salima
        }
    }

    /// Whether every line read, so the rows may stand for the file.
    #[must_use]
    pub const fn salima(&self) -> bool {
        self.talifa.is_empty()
    }
}

/// Reads a project's string file, keeping every line that did not parse.
///
/// # Errors
///
/// [`KhataWarsha::KhataMalaf`] when the file exists and cannot be read, or when
/// something other than a file sits at its path. An absent file is a read with
/// `mawjud == false`, which is what a project whose extraction died before its
/// first batch looks like — and is not what a damaged file looks like.
pub fn iqra_nusus(mashru: &MashruMaftuh) -> NatijatWarsha<QiraatNusus> {
    let masar = mashru.jidhr().join(MALAF_NUSUS);
    let bayan = match std::fs::metadata(&masar) {
        Ok(bayan) => bayan,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Ok(QiraatNusus::default());
        },
        Err(sabab) => {
            return Err(KhataWarsha::KhataMalaf {
                masar,
                amal: "reading the project strings",
                sabab,
            });
        },
    };
    if !bayan.is_file() {
        return Err(KhataWarsha::KhataMalaf {
            masar,
            amal: "reading the project strings",
            sabab: std::io::Error::other("the path is not a file"),
        });
    }
    let bayt = std::fs::read(&masar).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: masar.clone(),
        amal: "reading the project strings",
        sabab,
    })?;
    let (sufuf, talifa) = hallil_jsonl(&bayt);
    Ok(QiraatNusus {
        sufuf,
        talifa,
        mawjud: true,
        hajm: bayan.len(),
    })
}

/// Splits JSON Lines into the rows that read and the lines that did not.
///
/// Bytes are split before any decoding, so one line of invalid UTF-8 costs
/// that line and nothing else. Line numbers count every line in the file,
/// blank ones included, so a number here is the number an editor shows.
#[must_use]
pub fn hallil_jsonl(bayt: &[u8]) -> (Vec<MudkhalNass>, Vec<SatrTalif>) {
    let mut sufuf = Vec::new();
    let mut talifa = Vec::new();
    for (fihris, satr) in bayt.split(|harf| *harf == b'\n').enumerate() {
        // A trailing carriage return is a line ending, not content: a file that
        // crossed a Windows editor must not read as a file of torn rows.
        let satr = satr.strip_suffix(b"\r").unwrap_or(satr);
        if satr.is_empty() {
            continue;
        }
        let raqm = fihris.saturating_add(1);
        let Ok(nass) = std::str::from_utf8(satr) else {
            talifa.push(satr_talif(raqm, satr, SababTalaf::Tarmiz));
            continue;
        };
        match serde_json::from_str::<MudkhalNass>(nass) {
            Ok(mudkhal) => sufuf.push(mudkhal),
            Err(sabab) => {
                talifa.push(satr_talif(
                    raqm,
                    satr,
                    SababTalaf::Sigha {
                        sabab: sabab.to_string(),
                    },
                ));
            },
        }
    }
    (sufuf, talifa)
}

/// Describes one unreadable line from whatever its head still says.
fn satr_talif(raqm: usize, satr: &[u8], sabab: SababTalaf) -> SatrTalif {
    let nass = String::from_utf8_lossy(satr);
    SatrTalif {
        raqm,
        tul: satr.len(),
        sabab,
        huwiya: haql_nassi(&nass, "id").and_then(|khaam| serde_json::from_str(&khaam).ok()),
        masdar: haql_nassi(&nass, "masdar").and_then(|khaam| serde_json::from_str(&khaam).ok()),
        muqtataf: muqtataf(&nass),
    }
}

/// The quoted JSON string literal of a top-level field, when the line reaches
/// its closing quote.
///
/// A row is written with its identity first and its source text second, so a
/// line torn anywhere past its head still names the string it was. The literal
/// is returned with its quotes so `serde_json` decodes the escapes rather than
/// this function guessing at them.
fn haql_nassi(nass: &str, ism: &str) -> Option<String> {
    let miftah = format!("\"{ism}\":");
    let bidaya = nass.find(&miftah)?.saturating_add(miftah.len());
    let baqi = nass.get(bidaya..)?.trim_start();
    let qeema = baqi.strip_prefix('"')?;
    let mut mahrub = false;
    for (mawdi, harf) in qeema.char_indices() {
        if mahrub {
            mahrub = false;
        } else if harf == '\\' {
            mahrub = true;
        } else if harf == '"' {
            return qeema.get(..mawdi).map(|dakhil| format!("\"{dakhil}\""));
        }
    }
    None
}

/// The first [`AQSA_MUQTATAF`] characters of a line, marked when cut.
fn muqtataf(nass: &str) -> String {
    let mut ahruf = nass.chars();
    let mut awwal: String = ahruf.by_ref().take(AQSA_MUQTATAF).collect();
    if ahruf.next().is_some() {
        awwal.push('…');
    }
    awwal
}

/// What setting the damaged rows aside produced: where the original went, where
/// the unreadable lines went, and what the report says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaqreerInqadh {
    /// The damaged file, preserved byte for byte.
    pub mahfudh: PathBuf,
    /// The unreadable lines alone, raw, one per line.
    pub marfud: PathBuf,
    /// The JSON report: every unreadable line's number, reason and identity.
    pub taqreer: PathBuf,
    /// How many rows the live table now holds.
    pub najin: usize,
    /// The lines that were set aside.
    pub talifa: Vec<SatrTalif>,
    /// BLAKE3 of the preserved copy, lowercase hex, as the report records it.
    pub basma: String,
}

/// The report the rescue writes beside the project, so a contributor a month
/// later can answer "where did those rows go" without this program.
#[derive(Debug, Serialize, Deserialize)]
pub struct MalafTaqreerInqadh {
    /// When the rescue ran, RFC 3339, as the caller supplied it.
    pub waqt: String,
    /// The file that was damaged, by name.
    pub asl: String,
    /// The preserved copy, by name.
    pub mahfudh: String,
    /// The unreadable lines alone, by name.
    pub marfud: String,
    /// BLAKE3 of the preserved copy, lowercase hex.
    pub basma: String,
    /// The damaged file's length in bytes.
    pub hajm: u64,
    /// How many rows read and were kept as the live table.
    pub najin: usize,
    /// Every line that did not read.
    pub talifa: Vec<SatrTalif>,
}

/// Sets the unreadable lines of a project's string file aside and rewrites the
/// live table from the rows that read.
///
/// Nothing is replaced before the original is safe: the damaged file is copied
/// under [`WASM_TALIF`] with the given moment in its name, the copy is read
/// back and hashed against the original, and only then are the unreadable
/// lines and the report written and the live table rewritten. None of the files
/// this writes is ever deleted by this crate.
///
/// # Errors
///
/// [`KhataWarsha::LaTalaf`] when every line reads — there is nothing to set
/// aside and nothing is written; [`KhataWarsha::InqadhGhayrMuthbat`] when the
/// preserved copy does not match the original, in which case the live table is
/// untouched; [`KhataWarsha::KhataMalaf`] and [`KhataWarsha::Mawrid`] for a
/// file that cannot be read or written.
pub fn anqidh(mashru: &mut MashruMaftuh, waqt: &str) -> NatijatWarsha<TaqreerInqadh> {
    let jidhr = mashru.jidhr().to_path_buf();
    let masar = jidhr.join(MALAF_NUSUS);
    let bayt = match std::fs::read(&masar) {
        Ok(bayt) => bayt,
        Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
            return Err(KhataWarsha::LaTalaf);
        },
        Err(sabab) => {
            return Err(KhataWarsha::KhataMalaf {
                masar,
                amal: "reading the project strings before setting damaged rows aside",
                sabab,
            });
        },
    };
    let (sufuf, talifa) = hallil_jsonl(&bayt);
    if talifa.is_empty() {
        return Err(KhataWarsha::LaTalaf);
    }

    let (mahfudh, marfud, taqreer) = asma_inqadh(&jidhr, waqt)?;

    masarat::kitaba_dharra(&mahfudh, &bayt).map_err(min_usus)?;
    let nuskha = std::fs::read(&mahfudh).map_err(|sabab| KhataWarsha::KhataMalaf {
        masar: mahfudh.clone(),
        amal: "reading back the preserved copy",
        sabab,
    })?;
    let basma = blake3::hash(&bayt).to_hex().to_string();
    if nuskha != bayt {
        return Err(KhataWarsha::InqadhGhayrMuthbat { masar: mahfudh });
    }

    let arqam: BTreeSet<usize> = talifa.iter().map(|satr| satr.raqm).collect();
    let mut bayt_marfud = Vec::new();
    for (fihris, satr) in bayt.split(|harf| *harf == b'\n').enumerate() {
        if arqam.contains(&fihris.saturating_add(1)) {
            bayt_marfud.extend_from_slice(satr.strip_suffix(b"\r").unwrap_or(satr));
            bayt_marfud.push(b'\n');
        }
    }
    masarat::kitaba_dharra(&marfud, &bayt_marfud).map_err(min_usus)?;

    let malaf = MalafTaqreerInqadh {
        waqt: waqt.to_owned(),
        asl: MALAF_NUSUS.to_owned(),
        mahfudh: ism_malaf(&mahfudh),
        marfud: ism_malaf(&marfud),
        basma: basma.clone(),
        hajm: u64::try_from(bayt.len()).unwrap_or(u64::MAX),
        najin: sufuf.len(),
        talifa: talifa.clone(),
    };
    let bayt_taqreer = serde_json::to_vec_pretty(&malaf).map_err(|sabab| KhataWarsha::Mawrid {
        sabab: sabab.to_string(),
    })?;
    masarat::kitaba_dharra(&taqreer, &bayt_taqreer).map_err(min_usus)?;

    mashru
        .uktub_kul(&sufuf, waqt.to_owned())
        .map_err(|khata| KhataWarsha::Mawrid {
            sabab: khata.to_string(),
        })?;

    Ok(TaqreerInqadh {
        mahfudh,
        marfud,
        taqreer,
        najin: sufuf.len(),
        talifa,
        basma,
    })
}

/// Three names beside the project that do not exist yet, stamped with the
/// moment so two rescues of one project never overwrite each other.
fn asma_inqadh(jidhr: &Path, waqt: &str) -> NatijatWarsha<(PathBuf, PathBuf, PathBuf)> {
    let wasm = wasm_waqt(waqt);
    for muhawala in 0..AQSA_MUHAWALAT_ISM {
        let asas = if muhawala == 0 {
            format!("{WASM_TALIF}.{wasm}")
        } else {
            format!("{WASM_TALIF}.{wasm}.{}", muhawala.saturating_add(1))
        };
        let mahfudh = jidhr.join(format!("{asas}.jsonl"));
        let marfud = jidhr.join(format!("{asas}.marfud.jsonl"));
        let taqreer = jidhr.join(format!("{asas}.json"));
        if !mahfudh.exists() && !marfud.exists() && !taqreer.exists() {
            return Ok((mahfudh, marfud, taqreer));
        }
    }
    Err(KhataWarsha::Mawrid {
        sabab: format!(
            "{AQSA_MUHAWALAT_ISM} rescue files already carry the moment {wasm} beside this \
             project, so no free name was found"
        ),
    })
}

/// A moment as a file-name fragment: every character a filesystem might refuse
/// is dropped, so `2026-09-06T10:00:00Z` becomes `20260906T100000Z`.
fn wasm_waqt(waqt: &str) -> String {
    let wasm: String = waqt.chars().filter(char::is_ascii_alphanumeric).collect();
    if wasm.is_empty() {
        "0".to_owned()
    } else {
        wasm
    }
}

fn ism_malaf(masar: &Path) -> String {
    masar.file_name().map_or_else(
        || masar.display().to_string(),
        |ism| ism.to_string_lossy().into_owned(),
    )
}

fn min_usus(khata: taarib_usus::khata::Khata) -> KhataWarsha {
    KhataWarsha::Mawrid {
        sabab: khata.injilizi,
    }
}

#[cfg(test)]
mod ikhtibarat {
    use super::*;

    #[test]
    fn alhaql_yuqra_min_raas_satr_mabtur() {
        let satr = concat!(
            r#"{"id":"0f3c6a1e-2b4d-5e6f-8a9b-0c1d2e3f4a5b","#,
            r#""masdar":"Press \"A\" now","hadaf"#
        );
        assert_eq!(
            haql_nassi(satr, "id").as_deref(),
            Some("\"0f3c6a1e-2b4d-5e6f-8a9b-0c1d2e3f4a5b\"")
        );
        assert_eq!(
            haql_nassi(satr, "masdar").as_deref(),
            Some(r#""Press \"A\" now""#)
        );
        assert_eq!(haql_nassi(satr, "hadaf"), None);
    }

    #[test]
    fn alhaql_almabtur_dakhil_alqeema_la_yukhman() {
        assert_eq!(haql_nassi(r#"{"id":"0f3c6a1e-2b4d"#, "id"), None);
    }

    #[test]
    fn almuqtataf_yuqtaa_bi_alama() {
        let tawil: String = "x".repeat(AQSA_MUQTATAF + 5);
        let natija = muqtataf(&tawil);
        assert_eq!(natija.chars().count(), AQSA_MUQTATAF + 1);
        assert!(natija.ends_with('…'));
        assert_eq!(muqtataf("short"), "short");
    }

    #[test]
    fn wasm_alwaqt_yasluh_isman() {
        assert_eq!(wasm_waqt("2026-09-06T10:00:00Z"), "20260906T100000Z");
        assert_eq!(wasm_waqt("::"), "0");
    }

    #[test]
    fn alasfar_wa_alfawasil_la_tuad_talifa() {
        let (sufuf, talifa) = hallil_jsonl(b"\n\r\n\n");
        assert!(sufuf.is_empty());
        assert!(talifa.is_empty());
    }

    #[test]
    fn arqam_alsutur_hiya_arqam_almuharrir() {
        let (sufuf, talifa) = hallil_jsonl(b"\n\n{\"x\":1}\n\xff\xfe\n");
        assert!(sufuf.is_empty());
        let arqam: Vec<usize> = talifa.iter().map(|satr| satr.raqm).collect();
        assert_eq!(arqam, vec![3, 4]);
        assert!(matches!(
            talifa.first().map(|satr| &satr.sabab),
            Some(SababTalaf::Sigha { .. })
        ));
        assert_eq!(
            talifa.get(1).map(|satr| &satr.sabab),
            Some(&SababTalaf::Tarmiz)
        );
    }
}
