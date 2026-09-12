//! الخزينة — the translation cache that outlives the process, and the layers
//! that answer before a provider is ever asked.
//!
//! A game's menu shows the same thirty strings ten thousand times. The session
//! memory in [`crate::mutarjim::DhakiraJalsa`] stops a session paying twice for
//! them; nothing in the payload build stopped the *next* session paying again,
//! because the shared memory that would is a `SQLite` database behind a feature
//! the payload never enables. This module is the cache the payload does have:
//! one append-only file per source language under the data root, read whole at
//! open, consulted before any provider is considered, and written the moment a
//! provider answers.
//!
//! ## The key
//!
//! The recognized text with its whitespace collapsed — [`crate::tatabbu::wahhid`]
//! — and the source language. Not the game: `Continue` means the same thing on
//! every title, and a cache scoped per game would make the first hour of every
//! new game pay for strings the player already paid for elsewhere. The game is
//! kept on the record as provenance, so a pair can still be traced to where it
//! was first read.
//!
//! ## The layers
//!
//! [`DhakiraMurakkaba`] queries a list in order and writes to one. The order the
//! bootstrap builds is: the patch beside the payload, whose string table is
//! human work keyed by the exact source text; then this file. A provider is
//! reached only when both are silent, and its answer lands in the file so that
//! it is the last time that sentence is asked for on this machine.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use taarib_ruqaa::JadwalNusus;
use taarib_ruqaa::miftah_min_nass;

use crate::khata::KhataTabaqa;
use crate::mutarjim::{DhakiraTabaqa, QaydTabaqa, RaddSatr, TalabDhakira};
use crate::sijill_qira::MasdarTarjama;
use crate::tatabbu::wahhid;

/// The directory, under this tier's data directory, that holds the cache files.
pub const DALIL_KHAZINA: &str = "khazina";

/// The longest source text the cache will hold, in characters.
///
/// Matches [`crate::tatabbu::AQSA_TUL_MUQARANA`]: a reading longer than this is
/// a texture read as text, and storing its translation would fill the file with
/// pairs nothing will ever ask for again.
pub const AQSA_TUL_ASL: usize = 2_000;

/// The path of the cache file for one source language.
///
/// `mujallad_tabaqa` is the tier's own directory — [`crate::DALIL_TABAQA`] under
/// the data root. The language is part of the file name rather than of the key
/// so that a machine reading two games in two languages keeps two files that
/// can be inspected and deleted separately.
#[must_use]
pub fn masar_khazina(mujallad_tabaqa: &Path, lugha: &str) -> PathBuf {
    let ism: String = lugha
        .chars()
        .filter(|harf| harf.is_ascii_alphanumeric() || *harf == '-')
        .collect();
    let ism = if ism.is_empty() {
        "asl".to_owned()
    } else {
        ism
    };
    mujallad_tabaqa
        .join(DALIL_KHAZINA)
        .join(format!("{}.jsonl", ism.to_ascii_lowercase()))
}

/// The form a source text is keyed under.
///
/// Whitespace collapsed and trimmed, exactly as the tracker compares readings,
/// so that a line the tracker treated as one line is one key here.
#[must_use]
pub fn miftah_khazina(asl: &str) -> String {
    wahhid(asl)
}

/// One line of the cache file.
///
/// Every field beyond the pair is provenance, kept for the reason
/// [`crate::mutarjim::QaydTabaqa`] carries it: a pair that came off a screen and
/// through a machine is a weaker claim than one a person wrote, and the file
/// says which it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct QaydKhazina {
    /// The normalized source text.
    asl: String,
    /// The Arabic.
    arabi: String,
    /// [`MasdarTarjama::ism`] of how the Arabic was produced.
    masdar: String,
    /// The game it was first read in, by display name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    luba: Option<String>,
    /// Which provider produced the Arabic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    muzawwid: Option<String>,
    /// Which recognizer read the source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    qari: Option<String>,
    /// The recognizer's confidence, when it measured one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    thiqa: Option<u8>,
}

impl QaydKhazina {
    /// The reply this record answers a lookup with.
    fn radd(&self) -> RaddSatr {
        let masdar = match self.masdar.as_str() {
            "project" => MasdarTarjama::Mashru,
            "machine" => MasdarTarjama::Aaliya,
            _ => MasdarTarjama::Mulahaza,
        };
        RaddSatr {
            arabi: self.arabi.clone(),
            masdar,
        }
    }
}

/// The counters the control panel reports the cache by.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IhsaatKhazina {
    /// Pairs held in memory.
    pub adad: u64,
    /// Lookups answered.
    pub isabat: u64,
    /// Lookups not answered.
    pub ikhfaqat: u64,
    /// Pairs appended to the file this session.
    pub kutibat: u64,
    /// Lines of the file this build could not read, skipped at open.
    pub talifa: u64,
    /// Appends that failed.
    pub ikhfaqat_kitaba: u64,
}

impl IhsaatKhazina {
    /// The hit rate as a percentage, or [`None`] before the first lookup.
    #[must_use]
    pub fn nisbat_isaba(&self) -> Option<f64> {
        let kull = self.isabat.saturating_add(self.ikhfaqat);
        if kull == 0 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "lookup counts over one session are far below 2^53"
        )]
        Some(self.isabat as f64 * 100.0 / kull as f64)
    }

    /// The sentence the panel and the log carry, in English.
    #[must_use]
    pub fn wasf(&self) -> String {
        let nisba = self.nisbat_isaba().map_or_else(
            || "no lookups yet".to_owned(),
            |n| format!("{n:.0}% hit rate"),
        );
        let mut wasf = format!(
            "{} pair(s) on disk; {} hit(s), {} miss(es), {nisba}; {} written this session",
            self.adad, self.isabat, self.ikhfaqat, self.kutibat
        );
        if self.talifa > 0 {
            let _ = write!(wasf, ", {} unreadable line(s) skipped", self.talifa);
        }
        if self.ikhfaqat_kitaba > 0 {
            let _ = write!(wasf, ", {} write(s) failed", self.ikhfaqat_kitaba);
        }
        wasf
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let nisba = self.nisbat_isaba().map_or_else(
            || "لم يُبحث فيها بعد".to_owned(),
            |n| format!("نسبة الإصابة {n:.0}٪"),
        );
        let mut wasf = format!(
            "{} زوجًا محفوظًا على القرص؛ {} إصابة و{} إخفاقًا، {nisba}؛ كُتب {} في هذه الجلسة",
            self.adad, self.isabat, self.ikhfaqat, self.kutibat
        );
        if self.talifa > 0 {
            let _ = write!(wasf, "، وتُخطّي {} سطرًا تالفًا", self.talifa);
        }
        if self.ikhfaqat_kitaba > 0 {
            let _ = write!(wasf, "، وأخفقت {} كتابة", self.ikhfaqat_kitaba);
        }
        wasf
    }
}

/// What the cache holds and where it writes.
#[derive(Debug)]
struct DakhilKhazina {
    qiyud: HashMap<String, QaydKhazina>,
    katib: Option<BufWriter<File>>,
    ihsaat: IhsaatKhazina,
}

/// The file-backed cache: one JSON record per line, appended, never rewritten.
///
/// Append-only on purpose. A rewrite of a file that holds every translation a
/// player has ever been shown is a rewrite a crash can truncate, and a cache that
/// can lose itself is one the panel cannot honestly call persistent. Appending a
/// line is atomic for a line this short on every platform this product targets,
/// and a partial last line — the one crash mode left — is skipped and counted at
/// the next open rather than refusing the whole file.
#[derive(Debug)]
pub struct DhakiraMalaf {
    masar: PathBuf,
    lugha: String,
    dakhil: Mutex<DakhilKhazina>,
}

impl DhakiraMalaf {
    /// Opens the cache for one source language, reading every pair it holds.
    ///
    /// A missing file is the first run and is not a failure. The file is not
    /// created until the first write, so a session that translates nothing
    /// leaves nothing behind.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when a file that exists cannot be read.
    pub fn iftah(masar: PathBuf, lugha: &str) -> Result<Self, KhataTabaqa> {
        let mut qiyud: HashMap<String, QaydKhazina> = HashMap::new();
        let mut talifa = 0_u64;
        match std::fs::read_to_string(&masar) {
            Ok(nass) => {
                for satr in nass.lines() {
                    if satr.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<QaydKhazina>(satr) {
                        Ok(qayd) => {
                            let _ = qiyud.insert(miftah_khazina(&qayd.asl), qayd);
                        },
                        Err(_) => talifa = talifa.saturating_add(1),
                    }
                }
            },
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {},
            Err(sabab) => {
                return Err(KhataTabaqa::KhataMalaf { masar, sabab });
            },
        }
        let adad = u64::try_from(qiyud.len()).unwrap_or(u64::MAX);
        Ok(Self {
            masar,
            lugha: lugha.to_owned(),
            dakhil: Mutex::new(DakhilKhazina {
                qiyud,
                katib: None,
                ihsaat: IhsaatKhazina {
                    adad,
                    talifa,
                    ..IhsaatKhazina::default()
                },
            }),
        })
    }

    /// Where the file is.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// The source language this cache is for.
    #[must_use]
    pub fn lugha(&self) -> &str {
        &self.lugha
    }

    /// The counters.
    #[must_use]
    pub fn ihsaat(&self) -> IhsaatKhazina {
        self.dakhil.lock().ihsaat
    }

    /// How many pairs are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.dakhil.lock().qiyud.len()
    }

    /// Whether a source text is already answered, without counting a lookup.
    #[must_use]
    pub fn yahwi(&self, asl: &str) -> bool {
        self.dakhil.lock().qiyud.contains_key(&miftah_khazina(asl))
    }

    /// The sentence the panel shows, in English.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} ({}): {}",
            self.masar.display(),
            self.lugha,
            self.ihsaat().wasf()
        )
    }

    /// The same sentence in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        self.ihsaat().wasf_arabi()
    }

    /// Appends one record, opening the file on first use.
    fn iktub(dakhil: &mut DakhilKhazina, masar: &Path, qayd: &QaydKhazina) -> std::io::Result<()> {
        if dakhil.katib.is_none() {
            if let Some(walid) = masar.parent() {
                std::fs::create_dir_all(walid)?;
            }
            let malaf = OpenOptions::new().create(true).append(true).open(masar)?;
            dakhil.katib = Some(BufWriter::new(malaf));
        }
        let Some(katib) = dakhil.katib.as_mut() else {
            return Err(std::io::Error::other("the cache writer was not opened"));
        };
        let satr = serde_json::to_string(qayd).map_err(std::io::Error::other)?;
        katib.write_all(satr.as_bytes())?;
        katib.write_all(b"\n")?;
        katib.flush()
    }
}

impl DhakiraTabaqa for DhakiraMalaf {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait returns `&str` so an implementation can name itself after something \
                  it owns; an impl signature cannot narrow the trait's"
    )]
    fn ism(&self) -> &str {
        "on-disk cache"
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        let miftah = miftah_khazina(talab.asl);
        let mut dakhil = self.dakhil.lock();
        let radd = dakhil.qiyud.get(&miftah).map(QaydKhazina::radd);
        if radd.is_some() {
            dakhil.ihsaat.isabat = dakhil.ihsaat.isabat.saturating_add(1);
        } else {
            dakhil.ihsaat.ikhfaqat = dakhil.ihsaat.ikhfaqat.saturating_add(1);
        }
        radd
    }

    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        let asl = miftah_khazina(qayd.asl);
        if asl.is_empty() || asl.chars().count() > AQSA_TUL_ASL || qayd.arabi.trim().is_empty() {
            return Ok(());
        }
        let sijill = QaydKhazina {
            asl: asl.clone(),
            arabi: qayd.arabi.to_owned(),
            // Stored as an observation: this session read the source off a
            // screen and had a machine translate the reading, and the next
            // session that finds it here is told exactly that.
            masdar: MasdarTarjama::Mulahaza.ism().to_owned(),
            luba: qayd.ism_luba.map(str::to_owned),
            muzawwid: qayd.muzawwid.map(str::to_owned),
            qari: qayd.qari.map(str::to_owned),
            thiqa: qayd.thiqa,
        };
        let mut dakhil = self.dakhil.lock();
        if dakhil
            .qiyud
            .get(&asl)
            .is_some_and(|mawjud| mawjud.arabi == sijill.arabi)
        {
            return Ok(());
        }
        let natija = Self::iktub(&mut dakhil, &self.masar, &sijill);
        let jadeed = dakhil.qiyud.insert(asl, sijill).is_none();
        if jadeed {
            dakhil.ihsaat.adad = dakhil.ihsaat.adad.saturating_add(1);
        }
        match natija {
            Ok(()) => {
                dakhil.ihsaat.kutibat = dakhil.ihsaat.kutibat.saturating_add(1);
                Ok(())
            },
            Err(sabab) => {
                dakhil.ihsaat.ikhfaqat_kitaba = dakhil.ihsaat.ikhfaqat_kitaba.saturating_add(1);
                // The writer is dropped so the next append reopens the file:
                // a disk that was momentarily full is the common cause, and a
                // writer holding a half-written buffer would repeat it.
                dakhil.katib = None;
                Err(KhataTabaqa::KhataMalaf {
                    masar: self.masar.clone(),
                    sabab,
                })
            },
        }
    }

    fn daaima(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// The patch beside the payload
// ---------------------------------------------------------------------------

/// The patch's own string table, as a read-only memory layer.
///
/// A patch installed for a game carries every string a human translated for it,
/// keyed by the exact source text. A recognizer that read a menu label cleanly
/// produces that exact text, and a hit here is a person's translation rather
/// than a machine's — which is why this layer is queried first and answers as
/// [`MasdarTarjama::Mashru`].
///
/// The table is copied out of the mapping at construction rather than borrowed:
/// the mapping lives behind the bootstrap's lock, the lookup runs on the worker,
/// and a worker that took the bootstrap's lock four times a second would stall
/// the render thread every time a frame arrived while it held it.
#[derive(Debug)]
pub struct DhakiraRuqaa {
    nusus: HashMap<u64, String>,
    isabat: Mutex<u64>,
}

impl DhakiraRuqaa {
    /// Copies a patch's string table.
    #[must_use]
    pub fn min_jadwal(jadwal: &JadwalNusus<'_>) -> Self {
        let mut nusus = HashMap::with_capacity(jadwal.adad());
        for sijill in jadwal.sijillat {
            if let Some(arabi) = jadwal.nass(sijill.nass)
                && !arabi.trim().is_empty()
            {
                let _ = nusus.insert(sijill.miftah, arabi.to_owned());
            }
        }
        Self {
            nusus,
            isabat: Mutex::new(0),
        }
    }

    /// An empty layer, for a font-only patch.
    #[must_use]
    pub fn khaliya() -> Self {
        Self {
            nusus: HashMap::new(),
            isabat: Mutex::new(0),
        }
    }

    /// How many strings the patch carries.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.nusus.len()
    }

    /// How many lookups the patch answered.
    #[must_use]
    pub fn isabat(&self) -> u64 {
        *self.isabat.lock()
    }
}

impl DhakiraTabaqa for DhakiraRuqaa {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait returns `&str` so an implementation can name itself after something \
                  it owns; an impl signature cannot narrow the trait's"
    )]
    fn ism(&self) -> &str {
        "patch"
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        if self.nusus.is_empty() {
            return None;
        }
        // The exact reading first, then its normalized form: a patch's key is
        // BLAKE3 over the source as the game ships it, and a recognizer that
        // split two words with two spaces has not read a different string.
        let mawjud = self
            .nusus
            .get(&miftah_min_nass(talab.asl))
            .or_else(|| self.nusus.get(&miftah_min_nass(&miftah_khazina(talab.asl))))
            .or_else(|| self.nusus.get(&miftah_min_nass(talab.asl.trim())))?;
        let mut isabat = self.isabat.lock();
        *isabat = isabat.saturating_add(1);
        Some(RaddSatr::basharia(mawjud.clone()))
    }

    fn sajjil(&self, _: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        Ok(())
    }

    fn daaima(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Layers
// ---------------------------------------------------------------------------

/// Several memories asked in order, one of them written.
#[derive(Debug)]
pub struct DhakiraMurakkaba {
    tabaqat: Vec<Arc<dyn DhakiraTabaqa>>,
    kitaba: Arc<dyn DhakiraTabaqa>,
    ism: String,
}

impl DhakiraMurakkaba {
    /// Layers queried in the given order, with writes going to `kitaba`.
    ///
    /// `kitaba` is queried too, after the others, so a pair it holds is found
    /// before a provider is asked for it again.
    #[must_use]
    pub fn jadeeda(qabl: Vec<Arc<dyn DhakiraTabaqa>>, kitaba: Arc<dyn DhakiraTabaqa>) -> Self {
        let mut tabaqat = qabl;
        tabaqat.push(Arc::clone(&kitaba));
        let ism = tabaqat
            .iter()
            .map(|tabaqa| tabaqa.ism().to_owned())
            .collect::<Vec<_>>()
            .join(" → ");
        Self {
            tabaqat,
            kitaba,
            ism,
        }
    }
}

impl DhakiraTabaqa for DhakiraMurakkaba {
    fn ism(&self) -> &str {
        &self.ism
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        self.tabaqat.iter().find_map(|tabaqa| tabaqa.ibhath(talab))
    }

    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        self.kitaba.sajjil(qayd)
    }

    fn daaima(&self) -> bool {
        self.kitaba.daaima()
    }
}
