//! The run: its identity, its directory, and the journal that makes it
//! resumable.
//!
//! ## Where a run's state lives, and why it is not in the database
//!
//! `taarib-makhzan` is the product's store, and adding a `mashwar` table to it
//! is three edits inside `crates/taarib-makhzan/src/hijra.rs` — a `HIJRA_2`
//! statement, an entry appended to `HIJRAT` at line 91, and `ISDAR_MADUM`
//! bumped at line 99. This crate does not own that file, and three reasons say
//! the journal belongs here even if it did:
//!
//! 1. **It records paid work.** The translation stage's own checkpoint is a
//!    file in the project directory for exactly this reason
//!    (`taarib_tarjama::dufaat`, `MALAF_SIJILL_JAWLA`), and a run whose stage
//!    record lived somewhere else could disagree with it about what was paid
//!    for. One directory, one truth.
//! 2. **It must survive a copy.** A user who moves the run directory to another
//!    machine moves the run. A row in `taarib.db` does not travel.
//! 3. **It must be enumerable.** [`ijrud`] answers "what was I in the middle
//!    of?" by reading the directory. `taarib_makhzan::SijillHalat` — the one
//!    table an outside crate may write without a migration — is exact-key
//!    lookup with no prefix scan, so it cannot answer that question at all.
//!
//! The state machine is `taarib_makhzan::SijillTathbeet`'s: a run *opens*
//! incomplete, so a process killed mid-run leaves a record that says so rather
//! than nothing.
//!
//! ## The layout on disk
//!
//! ```text
//! <runs root>/<run id>/
//!   mashwar.jsonl           the stage journal — header line, then one line per stage
//!   mashru/                 the project directory, as taarib-tarjama means it
//!     jadwal.json           the extracted table and the refusal report
//!     nusus.json            the project rows, translations folded in
//!     jawlat_tarjama.jsonl  taarib_tarjama::dufaat's own journal
//!   khutut/                 the fonts, staged where the font gate accepts them
//!   huzma.ruqaa             the compiled and sealed package
//!   nusakh/                 the install's backup root, and its manifest
//! ```

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use uuid::Uuid;

use crate::khata::{KhataTilqai, NatijatTilqai, khata_malaf};
use crate::taqaddum::MarhalaTilqai;

/// The journal schema this build writes and reads.
pub const ISDAR_SIJILL: u32 = 1;

/// The journal's file name inside the run directory.
pub const MALAF_SIJILL: &str = "mashwar.jsonl";

/// The project directory inside the run directory.
///
/// This is the path handed to `taarib_tarjama::dufaat::shaghghil_jawla`, which
/// writes its own journal into it.
pub const MUJALLAD_MASHRU: &str = "mashru";

/// The extracted table and its refusal report, inside the project directory.
pub const MALAF_JADWAL: &str = "jadwal.json";

/// The project rows, inside the project directory.
pub const MALAF_NUSUS: &str = "nusus.json";

/// Where fonts are staged, inside the run directory.
///
/// A directory rather than a list of files because
/// `taarib_tarqee::bawwaba::KhattMujammaa::min_majmua` gates on the font being
/// *inside* a directory it was handed, which is what stops a manifest from
/// naming a font that is somewhere else on the machine.
pub const MUJALLAD_KHUTUT: &str = "khutut";

/// The sealed package, inside the run directory.
pub const MALAF_HUZMA: &str = "huzma.ruqaa";

/// The install's backup root, inside the run directory.
pub const MUJALLAD_NUSAKH: &str = "nusakh";

/// A run's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MashwarId(Uuid);

impl MashwarId {
    /// A fresh identity.
    #[must_use]
    pub fn jadeed() -> Self {
        Self(Uuid::new_v4())
    }

    /// The identity a directory name spells, when it spells one.
    #[must_use]
    pub fn min_nass(nass: &str) -> Option<Self> {
        Uuid::parse_str(nass).ok().map(Self)
    }

    /// The directory this run owns under a runs root.
    #[must_use]
    pub fn mujallad(self, jidhr: &Path) -> PathBuf {
        jidhr.join(self.to_string())
    }
}

impl fmt::Display for MashwarId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

/// The journal's first line: what this run is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarwisatMashwar {
    /// The journal schema version.
    pub isdar: u32,
    /// The run.
    pub id: MashwarId,
    /// The game's name, denormalized so a listing needs nothing else.
    pub ism_luba: String,
    /// The game's root at the time the run opened.
    pub jidhr_luba: PathBuf,
    /// When the run opened, RFC 3339, supplied by the caller.
    pub waqt: String,
}

/// One stage's outcome, as the journal records it.
///
/// Every variant carries what a resumed run needs to *skip* that stage, and
/// nothing it would have to recompute. The two heavy outputs — the string table
/// and the sealed package — are files beside the journal, named by the
/// constants above, because a hundred thousand strings do not belong on one
/// line of a log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "marhala", rename_all = "snake_case")]
pub enum QaydMarhala {
    /// The probe resolved, and the capability report it produced.
    Fahs {
        /// The report, boxed: it is by far the largest variant and an unboxed
        /// one would set the size of every journal line.
        imkaniyat: Box<TaqreerImkaniyat>,
    },
    /// Extraction finished. The table itself is in [`MALAF_JADWAL`].
    Istikhraj {
        /// Distinct strings in the table.
        adad: u64,
        /// How many of them a player is believed to see.
        adad_zahir: u64,
        /// Containers read.
        maqrua: u64,
        /// Containers refused.
        marfuda: u64,
        /// Whether a capture session was merged in.
        multaqat: bool,
    },
    /// The translation run walked its whole list, or stopped at the ceiling.
    ///
    /// Recorded even when it stopped: `taarib_tarjama`'s own journal is what
    /// resumes the translation, and this line only records that the stage was
    /// entered and what it ended at.
    Tarjama {
        /// Strings the fold applied a translation to.
        mutabbaqa: u64,
        /// Strings that failed, with their reasons recorded on the entry.
        fashila: u64,
        /// Settled spend, in nano-dollars, across every run of this project.
        munfaq: u64,
        /// What stopped the run early, when something did.
        tawaqquf: Option<String>,
    },
    /// Fonts validated and sizes discovered.
    Takhtit {
        /// The staged font file names, in chain order.
        khutut: Vec<String>,
        /// The sizes every string will be laid out at, in quarter-pixels.
        ahjam_rubi: Vec<u32>,
        /// How many (string, size) pairs the compiler will lay out.
        azwaj: u64,
    },
    /// The container was written. It is in [`MALAF_HUZMA`], unsigned at this
    /// point.
    Tarqee {
        /// Its size in bytes.
        hajm: u64,
        /// Strings it holds.
        nusus: u64,
        /// Precomputed layouts it holds.
        takhtitat: u64,
        /// Atlas pages it holds.
        safahat: u64,
        /// The fingerprint of the translations this container was compiled
        /// from, as [`crate::bina::basmat_tarjamat`] takes it.
        ///
        /// What makes a resumed run notice that the package on disk is behind
        /// the table. Without it the run reused any sealed container it found:
        /// a resume that translated another fifteen hundred strings recompiled
        /// nothing, installed the package the first run had sealed, and left
        /// the game in the language the user had just paid to leave.
        ///
        /// Defaulted so a journal written before this field existed reads as
        /// the empty fingerprint, which matches no table and therefore
        /// recompiles. That is the safe direction: the cost of being wrong is
        /// one compile, and the cost of the other direction is a patch that
        /// silently omits everything translated since.
        #[serde(default)]
        basmat_nusus: String,
    },
    /// The container was sealed in place.
    Khatm {
        /// The public half of the sealing key, hexadecimal.
        miftah: String,
        /// The container's content hash the block commits to.
        basma: String,
    },
    /// The patch was installed.
    Tathbeet {
        /// The backup root, relative to the run directory.
        jidhr_nusakh: String,
        /// How many content files were placed.
        adad_muhtawa: u64,
    },
}

impl QaydMarhala {
    /// Which stage this record closes.
    #[must_use]
    pub const fn marhala(&self) -> MarhalaTilqai {
        match self {
            Self::Fahs { .. } => MarhalaTilqai::Fahs,
            Self::Istikhraj { .. } => MarhalaTilqai::Istikhraj,
            Self::Tarjama { .. } => MarhalaTilqai::Tarjama,
            Self::Takhtit { .. } => MarhalaTilqai::Takhtit,
            Self::Tarqee { .. } => MarhalaTilqai::Tarqee,
            Self::Khatm { .. } => MarhalaTilqai::Khatm,
            Self::Tathbeet { .. } => MarhalaTilqai::Tathbeet,
        }
    }
}

/// One journal line after the header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatrMarhala {
    /// When the stage closed, seconds since the Unix epoch, caller-supplied.
    pub lahza: u64,
    /// How long it took, in milliseconds.
    pub milli: u64,
    /// What it produced.
    pub qayd: QaydMarhala,
}

/// One line of the journal, in either shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
enum SatrSijill {
    /// The header.
    Tarwisa(TarwisatMashwar),
    /// A closed stage.
    Marhala(SatrMarhala),
}

/// The run journal: the checkpoint, explicit and on disk.
///
/// One JSON object per line, appended as each stage closes. On open the whole
/// file is read back, **last line wins per stage** — a stage re-run by a later
/// resume has both lines in the file and the newer outcome in the map, and the
/// history of attempts is itself worth keeping, which is why the file is never
/// rewritten in place.
///
/// A torn final line — the signature of a process killed mid-append — is
/// skipped and counted, exactly as `taarib_tarjama::dufaat` and
/// `taarib_istikhraj::iltiqat` handle theirs. One damaged line must not cost a
/// user the four thousand translated strings above it.
#[derive(Debug)]
pub struct SijillMashwar {
    /// The journal file.
    masar: PathBuf,
    /// The header, once one has been written.
    tarwisa: Option<TarwisatMashwar>,
    /// The last outcome recorded for each stage.
    marahil: Vec<SatrMarhala>,
    /// How many lines would not parse.
    talifa: usize,
}

impl SijillMashwar {
    /// Opens the journal inside a run directory, reading back whatever a
    /// previous run left.
    ///
    /// A missing file is an empty journal — the state every first run starts in
    /// — not an error.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when the directory cannot be created or the
    /// file exists and cannot be read. A run blind to what it already did would
    /// re-translate what it already paid for, so this stops the run rather than
    /// starting from zero.
    ///
    /// [`KhataTilqai::SijillAhdath`] when the header names a schema version
    /// this build does not read.
    pub fn iftah(jidhr_mashwar: &Path) -> NatijatTilqai<Self> {
        fs::create_dir_all(jidhr_mashwar)
            .map_err(|sabab| khata_malaf(jidhr_mashwar, "created", sabab))?;
        let masar = jidhr_mashwar.join(MALAF_SIJILL);

        let mut sijill = Self {
            masar: masar.clone(),
            tarwisa: None,
            marahil: Vec::new(),
            talifa: 0,
        };
        let malaf = match fs::File::open(&masar) {
            Ok(malaf) => malaf,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => return Ok(sijill),
            Err(sabab) => return Err(khata_malaf(&masar, "read", sabab)),
        };

        for satr in BufReader::new(malaf).lines() {
            let satr = match satr {
                Ok(satr) => satr,
                // Bytes that are not UTF-8: the reader has already found the
                // newline, so this is one damaged line and not a damaged file.
                Err(sabab) if sabab.kind() == std::io::ErrorKind::InvalidData => {
                    sijill.talifa = sijill.talifa.saturating_add(1);
                    continue;
                },
                Err(sabab) => return Err(khata_malaf(&masar, "read", sabab)),
            };
            if satr.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<SatrSijill>(&satr) {
                Ok(SatrSijill::Tarwisa(tarwisa)) => {
                    if tarwisa.isdar > ISDAR_SIJILL {
                        return Err(KhataTilqai::SijillAhdath {
                            masar,
                            wujid: tarwisa.isdar,
                            maqru: ISDAR_SIJILL,
                        });
                    }
                    sijill.tarwisa = Some(tarwisa);
                },
                Ok(SatrSijill::Marhala(marhala)) => {
                    sijill
                        .marahil
                        .retain(|sabiq| sabiq.qayd.marhala() != marhala.qayd.marhala());
                    sijill.marahil.push(marhala);
                },
                Err(_) => sijill.talifa = sijill.talifa.saturating_add(1),
            }
        }
        Ok(sijill)
    }

    /// The header, when the journal has one.
    #[must_use]
    pub const fn tarwisa(&self) -> Option<&TarwisatMashwar> {
        self.tarwisa.as_ref()
    }

    /// How many lines would not parse.
    #[must_use]
    pub const fn talifa(&self) -> usize {
        self.talifa
    }

    /// Whether a stage already closed in this or a previous run.
    #[must_use]
    pub fn tamma(&self, marhala: MarhalaTilqai) -> bool {
        self.marahil
            .iter()
            .any(|satr| satr.qayd.marhala() == marhala)
    }

    /// The last outcome recorded for a stage.
    #[must_use]
    pub fn qayd(&self, marhala: MarhalaTilqai) -> Option<&QaydMarhala> {
        self.marahil
            .iter()
            .find(|satr| satr.qayd.marhala() == marhala)
            .map(|satr| &satr.qayd)
    }

    /// Every closed stage, in the order they were recorded.
    #[must_use]
    pub fn marahil(&self) -> &[SatrMarhala] {
        &self.marahil
    }

    /// The furthest stage this run has reached.
    #[must_use]
    pub fn baligha(&self) -> Option<MarhalaTilqai> {
        self.marahil.iter().map(|satr| satr.qayd.marhala()).max()
    }

    /// Writes the header, if it is not already there.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when the line cannot be appended.
    pub fn ibda(&mut self, tarwisa: TarwisatMashwar) -> NatijatTilqai<()> {
        if self.tarwisa.is_some() {
            return Ok(());
        }
        self.alhiq(&SatrSijill::Tarwisa(tarwisa.clone()))?;
        self.tarwisa = Some(tarwisa);
        Ok(())
    }

    /// Records a stage's outcome, disk first.
    ///
    /// The append happens before the in-memory map is updated, so a write that
    /// fails cannot leave this process believing a stage is checkpointed when
    /// nothing on disk says so.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when the line cannot be appended.
    pub fn sajjil(&mut self, qayd: QaydMarhala, lahza: u64, milli: u64) -> NatijatTilqai<()> {
        let satr = SatrMarhala { lahza, milli, qayd };
        self.alhiq(&SatrSijill::Marhala(satr.clone()))?;
        self.marahil
            .retain(|sabiq| sabiq.qayd.marhala() != satr.qayd.marhala());
        self.marahil.push(satr);
        Ok(())
    }

    /// Forgets everything recorded from a stage onwards, on disk and in memory.
    ///
    /// The one operation that rewrites the file, and it exists for one reason:
    /// a run whose install was rolled back must not be resumable straight past
    /// the install into "already done". Everything before the named stage is
    /// preserved verbatim, including the money the translation stage spent.
    ///
    /// # Errors
    ///
    /// [`KhataTilqai::KhataMalaf`] when the file cannot be rewritten.
    pub fn insa_min(&mut self, marhala: MarhalaTilqai) -> NatijatTilqai<()> {
        self.marahil.retain(|satr| satr.qayd.marhala() < marhala);
        let mut nass = String::new();
        if let Some(tarwisa) = &self.tarwisa {
            nass.push_str(&sattir(&SatrSijill::Tarwisa(tarwisa.clone()))?);
        }
        for satr in &self.marahil {
            nass.push_str(&sattir(&SatrSijill::Marhala(satr.clone()))?);
        }
        taarib_usus::masarat::kitaba_dharra_nass(&self.masar, &nass).map_err(|sabab| {
            khata_malaf(
                &self.masar,
                "rewritten",
                std::io::Error::other(sabab.to_string()),
            )
        })
    }

    fn alhiq(&self, satr: &SatrSijill) -> NatijatTilqai<()> {
        let nass = sattir(satr)?;
        let mut malaf = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.masar)
            .map_err(|sabab| khata_malaf(&self.masar, "opened for append", sabab))?;
        malaf
            .write_all(nass.as_bytes())
            .map_err(|sabab| khata_malaf(&self.masar, "appended to", sabab))
    }
}

/// One journal line as text, newline included.
fn sattir(satr: &SatrSijill) -> NatijatTilqai<String> {
    let mut nass = serde_json::to_string(satr).map_err(|sabab| {
        khata_malaf(
            Path::new(MALAF_SIJILL),
            "serialized",
            std::io::Error::other(sabab),
        )
    })?;
    nass.push('\n');
    Ok(nass)
}

/// One resumable run, as a listing shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MashwarMawjuz {
    /// The run.
    pub id: MashwarId,
    /// Its directory.
    pub mujallad: PathBuf,
    /// What it is a translation of.
    pub ism_luba: String,
    /// The game's root as the run recorded it.
    pub jidhr_luba: PathBuf,
    /// When it opened.
    pub waqt: String,
    /// The furthest stage it reached, or [`None`] if it recorded none.
    pub baligha: Option<MarhalaTilqai>,
    /// Whether the run finished: the install stage closed.
    pub tammat: bool,
}

/// Every run under a runs root, newest journal entry first.
///
/// This is the "what was I in the middle of?" query, and it is a directory
/// read: a run directory that exists and holds a header is a run, whatever any
/// database does or does not say about it. A directory whose journal is
/// unreadable is skipped rather than reported — the caller asked what can be
/// resumed, and that one cannot be.
#[must_use]
pub fn ijrud(jidhr: &Path) -> Vec<MashwarMawjuz> {
    let Ok(qira) = fs::read_dir(jidhr) else {
        return Vec::new();
    };
    let mut mawjuzat: Vec<MashwarMawjuz> = Vec::new();
    for madkhal in qira.flatten() {
        let masar = madkhal.path();
        if !masar.is_dir() {
            continue;
        }
        let Some(id) = masar
            .file_name()
            .and_then(|ism| ism.to_str())
            .and_then(MashwarId::min_nass)
        else {
            continue;
        };
        let Ok(sijill) = SijillMashwar::iftah(&masar) else {
            continue;
        };
        let Some(tarwisa) = sijill.tarwisa() else {
            continue;
        };
        mawjuzat.push(MashwarMawjuz {
            id,
            mujallad: masar.clone(),
            ism_luba: tarwisa.ism_luba.clone(),
            jidhr_luba: tarwisa.jidhr_luba.clone(),
            waqt: tarwisa.waqt.clone(),
            baligha: sijill.baligha(),
            tammat: sijill.tamma(MarhalaTilqai::Tathbeet),
        });
    }
    mawjuzat.sort_by(|awwal, thani| thani.waqt.cmp(&awwal.waqt).then(awwal.id.cmp(&thani.id)));
    mawjuzat
}
