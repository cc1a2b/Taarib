//! سجلّ القراءة — every line the overlay read, kept so a player who missed one
//! can read it again.
//!
//! Tier 3 draws Arabic over a game it cannot modify, which means a line of
//! dialogue is on screen for exactly as long as the game leaves it there. A
//! player who was looking at the map when a quest-giver said something has no
//! way to get it back: there is no log window to open, because the game has no
//! idea any of this is happening. Reloading a save is the alternative, and for a
//! game with autosaves it is not one. So the overlay keeps its own history, and
//! this module is it.
//!
//! ## Deduplication is the whole difference between a log and a wall
//!
//! A dialogue box stays on screen for two hundred frames. The recognizer reads
//! the same sentence out of it every time it is polled, and a history that
//! appended each of those readings would hold one sentence, four hundred times,
//! and nothing else — the line the player actually wanted would be scrolled off
//! the end by the line they were already looking at.
//!
//! So a reading that repeats the immediately preceding entry *for the same
//! region*, with identical source text, does not append. It raises
//! [`MadkhalQira::takrar`] on the entry that is already there and moves its
//! last-seen timestamp forward. "For the same region" matters: two regions
//! alternating — a subtitle strip and a speaker-name strip — would otherwise
//! never be adjacent to their own previous reading, and the suppression would
//! never fire.
//!
//! ## Recognition and translation do not finish together
//!
//! [`MadkhalQira::arabi`] is an [`Option`] because it genuinely is one.
//! Recognition finishes in the overlay's own frame budget; translation goes out
//! to a pipeline that may be a local model, may be a project's human
//! translation, and may take a second. Recording the line at recognition time
//! and filling the Arabic in later is what lets the history show *what was
//! missed* even for a line whose translation never arrived — and a history that
//! only recorded fully translated lines would be missing exactly the lines a
//! player is trying to find out about.
//!
//! ## One bad line does not cost a history
//!
//! The file is JSON Lines: a header, then one entry per line, appended. A line
//! that will not parse — a torn write from a machine that lost power mid-append,
//! a byte flipped on a failing disk — is **skipped and counted**, not fatal.
//! Refusing to load a thousand lines of somebody's play session because the last
//! one is half-written would be losing the history to protect it. The count
//! comes back in [`TaqreerSijill::satur_talifa`] so the panel can say so plainly
//! rather than quietly showing a short history.
//!
//! The *schema version* is the opposite, and it is checked in the header before
//! a single entry is read. A version this build does not know means a newer
//! build wrote the file, and parsing its entries anyway would drop the fields
//! this build cannot see and then write them away on the first compaction.

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use taarib_mustalahat::luba::LubaId;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::khata::KhataTabaqa;
use crate::manatiq::MuarrifMintaqa;

/// The file one game's history is stored in.
///
/// `.jsonl` rather than `.json` because it is not one JSON document: it is a
/// header line and then one document per line, and naming it `.json` would
/// invite an editor to reformat it into something no appender can extend.
pub const ISM_MALAF: &str = "sijill_qira.jsonl";

/// The default number of entries held in memory and on disk.
pub const SAA_IFTIRADIYA: usize = 500;

/// The smallest capacity that is still a history rather than a status line.
pub const ADNA_SAA: usize = 16;

/// The largest capacity this build will hold.
///
/// Fifty thousand entries. The ceiling exists because the history lives in a
/// game's own address space: a capacity read from a settings file is a number a
/// user can type, and a number a user can type is a number that will one day be
/// ten million inside a process that also has to render a frame in sixteen
/// milliseconds.
pub const AQSA_SAA: usize = 50_000;

/// The longest source or translated line kept, in characters.
///
/// Recognition on a region containing a texture rather than text produces
/// nonsense of unbounded length, and a history full of it is a history nobody
/// can scroll. Longer lines are truncated on the way in and marked, rather than
/// dropped: a truncated line still tells a player which region produced it.
pub const AQSA_TUL_SATR: usize = 2_000;

/// What a recognizer said about a reading, or that it said nothing.
///
/// Two states, not a number with a sentinel. Of the three engines this crate
/// ships to, only macOS Vision reports a per-line confidence; the Windows
/// Runtime recognizer and the portable one report none, and
/// [`crate::qira::SatrMaqru`] carries [`crate::qira::THIQA_GHAYR_MAQISA`] — a
/// constant — in their place, with a `maqisa` flag beside it saying so.
///
/// That flag has to survive into the history, and this type is what carries it.
/// A history entry holding a bare `u8` cannot distinguish the stand-in constant
/// from a real measurement, and anything reading the file afterwards — the panel
/// deciding whether to show a confidence band, the shared translation memory
/// deciding whether an accumulation may be shared — is then reasoning about a
/// number nobody measured as though somebody had.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThiqatSatr {
    /// The engine reported this number itself, zero to a hundred.
    Maqisa(u8),

    /// The engine reports no confidence at all.
    ///
    /// Not "zero confidence" and not "low confidence" — no measurement exists.
    Ghayr,
}

impl ThiqatSatr {
    /// A measured confidence, clamped into the range engines report in.
    #[must_use]
    pub const fn maqisa_bi(mia: u8) -> Self {
        Self::Maqisa(if mia > 100 { 100 } else { mia })
    }

    /// A confidence from a recognized line, honouring its own `maqisa` flag.
    #[must_use]
    pub const fn min_maqru(mia: u8, maqisa: bool) -> Self {
        if maqisa { Self::maqisa_bi(mia) } else { Self::Ghayr }
    }

    /// Whether this is a measurement.
    #[must_use]
    pub const fn maqisa(self) -> bool {
        matches!(self, Self::Maqisa(_))
    }

    /// The number, or zero when there is none.
    ///
    /// Zero rather than an [`Option`] because the stored field is a `u8` and the
    /// flag beside it is what says whether to read it. A caller that wants the
    /// honest answer asks [`ThiqatSatr::mia_in_wujidat`].
    #[must_use]
    pub const fn mia(self) -> u8 {
        match self {
            Self::Maqisa(mia) => mia,
            Self::Ghayr => 0,
        }
    }

    /// The number, when there is one.
    #[must_use]
    pub const fn mia_in_wujidat(self) -> Option<u8> {
        match self {
            Self::Maqisa(mia) => Some(mia),
            Self::Ghayr => None,
        }
    }
}

/// Where a translation came from.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MasdarTarjama {
    /// Produced by a machine translator at play time.
    ///
    /// The tier-3 default, and the one the disclosure is about: it is fast, it
    /// is available for any game, and it is the weaker of the two.
    Aaliya,

    /// Taken from a Taarib translation project for this game.
    ///
    /// A human wrote it. Where a project covers a line, the overlay uses the
    /// project's text and says so — a player deciding whether to trust a
    /// sentence deserves to know which of the two produced it, and burying the
    /// difference would make the good translations look as uncertain as the
    /// machine ones.
    Mashru,

    /// Read off a screen by somebody, and machine-translated from that reading.
    ///
    /// The weakest of the three, and the only one whose **source text** may be
    /// wrong: a machine translation of an extracted string at least translated
    /// the string the game actually ships, while this one translated whatever a
    /// recognizer thought it saw. It arrives from the shared translation memory
    /// — possibly from another player's session on another machine — and the
    /// history says so rather than presenting it as this session's own work.
    ///
    /// Collapsing it into [`MasdarTarjama::Aaliya`] would tell a player
    /// scrolling back that Taarib translated the line here, when what actually
    /// happened is that somebody else's overlay read a similar screen and this
    /// one reused the answer. That is a different claim and a weaker one.
    Mulahaza,
}

impl MasdarTarjama {
    /// The name used in the history file and in log lines.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Aaliya => "machine",
            Self::Mashru => "project",
            Self::Mulahaza => "screen reading",
        }
    }

    /// The label the in-game panel shows, in Arabic.
    #[must_use]
    pub const fn unwan(self) -> &'static str {
        match self {
            Self::Aaliya => "ترجمة آلية",
            Self::Mashru => "من مشروع تعريب",
            Self::Mulahaza => "قراءة شاشة سابقة",
        }
    }

    /// Whether a human wrote this text.
    #[must_use]
    pub const fn basharia(self) -> bool {
        matches!(self, Self::Mashru)
    }

    /// Whether the source text this was translated from came off a screen.
    ///
    /// True for [`MasdarTarjama::Mulahaza`] alone. It is the question a player
    /// deciding how far to trust a line should be able to ask, because it is
    /// the only case where the *English* may already have been wrong.
    #[must_use]
    pub const fn min_shasha(self) -> bool {
        matches!(self, Self::Mulahaza)
    }
}

/// A history entry's identity.
///
/// Monotonic within one history and never reused, so a panel holding a
/// selection across a compaction can tell "the entry I had is gone" from "the
/// entry I had is now at a different index".
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MuarrifMadkhal(u64);

impl MuarrifMadkhal {
    /// Wraps an identity read back from disk.
    #[must_use]
    pub const fn min_raqm(raqm: u64) -> Self {
        Self(raqm)
    }

    /// The underlying number.
    #[must_use]
    pub const fn raqm(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for MuarrifMadkhal {
    fn fmt(&self, mukhraj: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(mukhraj, "#{}", self.0)
    }
}

/// One line the overlay read, and what it made of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MadkhalQira {
    /// Monotonic within this history.
    pub muarrif: MuarrifMadkhal,

    /// When the line was first recognized, in seconds since the Unix epoch.
    ///
    /// **Passed in.** This crate reads no clock: it runs on a game's render
    /// thread, where a time syscall is a cost paid inside somebody's frame for
    /// a number the caller already has. The same rule [`crate::sidq::Iqrar`]
    /// follows, for the same reason.
    pub lahza: u64,

    /// When the same line was last seen, in seconds since the Unix epoch.
    ///
    /// Equal to [`MadkhalQira::lahza`] until the line repeats. The pair is what
    /// lets the panel say "on screen for eleven seconds" instead of showing the
    /// same sentence eleven times.
    pub lahza_akhira: u64,

    /// Which region it came from.
    pub mintaqa: MuarrifMintaqa,

    /// The region's name at the moment the line was read.
    ///
    /// Copied rather than looked up. A player who renames a region should not
    /// find that yesterday's history now claims to have come from somewhere
    /// else, and a region that has since been deleted still has to be
    /// identifiable in the history it produced.
    pub ism_mintaqa: String,

    /// What the recognizer read, verbatim.
    pub asl: String,

    /// The Arabic, once there is any.
    ///
    /// [`None`] means recognition succeeded and translation has not come back —
    /// which is a normal state for the most recent entry on every single frame,
    /// not an error.
    pub arabi: Option<String>,

    /// The recognizer's confidence, zero to a hundred.
    ///
    /// Only meaningful when [`MadkhalQira::maqisa`] is true. Read it without
    /// that field and this is a number for every entry, including the ones
    /// where no engine ever produced one.
    pub thiqa: u8,

    /// Whether [`MadkhalQira::thiqa`] is a measurement.
    ///
    /// Only one of the three recognizers this crate ships to reports a
    /// per-line confidence at all; for the other two,
    /// [`crate::qira::SatrMaqru::thiqa`] carries
    /// [`crate::qira::THIQA_GHAYR_MAQISA`], which is a constant and not an
    /// observation. Without this bit a reader of a history file cannot tell the
    /// two apart, and the substituted constant leaks downstream as though an
    /// engine had said it.
    ///
    /// That is not a cosmetic distinction. The shared translation memory
    /// refuses to store a confidence it was not given — unmeasured is a state,
    /// not a low number — and withholds an unmeasured accumulation from being
    /// shared unless a person acknowledges it. A harvest from this file that
    /// could not tell the constant from a measurement would either have to call
    /// every entry unmeasured, losing the macOS readings that *are* measured,
    /// or call every entry measured, defeating the guard entirely.
    ///
    /// [`serde(default)`] is `false`, which is the correct reading of a file
    /// written before this field existed: such a file genuinely cannot tell.
    #[serde(default)]
    pub maqisa: bool,

    /// Which side produced [`MadkhalQira::arabi`], once something did.
    pub masdar: Option<MasdarTarjama>,

    /// How many consecutive readings produced this same line.
    ///
    /// One for a line read once. See the module documentation: without this
    /// field the history is one sentence repeated until it fills.
    pub takrar: u32,

    /// Whether the source text was longer than [`AQSA_TUL_SATR`] and was cut.
    #[serde(default)]
    pub maqtu: bool,
}

impl MadkhalQira {
    /// A freshly recognized line, with no translation yet.
    #[must_use]
    pub fn jadeed(
        muarrif: MuarrifMadkhal,
        lahza: u64,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: impl Into<String>,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> Self {
        let (asl, maqtu) = iqtata(asl);
        Self {
            muarrif,
            lahza,
            lahza_akhira: lahza,
            mintaqa,
            ism_mintaqa: ism_mintaqa.into(),
            asl,
            arabi: None,
            thiqa: thiqa.mia(),
            maqisa: thiqa.maqisa(),
            masdar: None,
            takrar: 1,
            maqtu,
        }
    }

    /// The same line with a translation already attached.
    #[must_use]
    pub fn mutarjam(mut self, arabi: &str, masdar: MasdarTarjama) -> Self {
        let (nass, _) = iqtata(arabi);
        self.arabi = Some(nass);
        self.masdar = Some(masdar);
        self
    }

    /// Whether the Arabic has arrived.
    #[must_use]
    pub const fn tarjuma_wasalat(&self) -> bool {
        self.arabi.is_some()
    }

    /// How long the line stayed on screen, in seconds, as far as the history saw.
    #[must_use]
    pub const fn mudda(&self) -> u64 {
        self.lahza_akhira.saturating_sub(self.lahza)
    }

    /// Whether either text contains a needle, case-folded.
    ///
    /// Folded with [`str::to_lowercase`], which is a no-op for Arabic and is
    /// there for the source text — a player searching a game's English for
    /// `Elden` should not have to know it was rendered `ELDEN`.
    #[must_use]
    pub fn yutabiq(&self, matlub: &str) -> bool {
        let matlub = matlub.trim().to_lowercase();
        if matlub.is_empty() {
            return true;
        }
        self.asl.to_lowercase().contains(&matlub)
            || self
                .arabi
                .as_ref()
                .is_some_and(|arabi| arabi.to_lowercase().contains(&matlub))
    }

    /// The recognizer's confidence, or the fact that it reported none.
    #[must_use]
    pub const fn thiqa(&self) -> ThiqatSatr {
        ThiqatSatr::min_maqru(self.thiqa, self.maqisa)
    }

    /// How the panel describes the recognizer's confidence.
    ///
    /// Four bands rather than three, and the fourth is the one that matters:
    /// an engine that reports no confidence at all is not a low-confidence
    /// reading, and showing "قراءة ضعيفة" for every line on Windows would be
    /// telling a player their whole session was read badly when nothing
    /// measured it either way. Three bands rather than a number for the rest,
    /// because a number invites a precision the recognizer does not have.
    #[must_use]
    pub const fn unwan_thiqa(&self) -> &'static str {
        if !self.maqisa {
            "لم يُقِس المحرّك ثقته"
        } else if self.thiqa >= 85 {
            "قراءة واضحة"
        } else if self.thiqa >= 60 {
            "قراءة محتملة الخطأ"
        } else {
            "قراءة ضعيفة"
        }
    }

    /// The one-line form the history page shows.
    #[must_use]
    pub fn satr(&self) -> String {
        let mut satr = String::new();
        let nass = self.arabi.as_deref().unwrap_or(&self.asl);
        let _ = write!(satr, "[{}] {nass}", self.ism_mintaqa);
        if self.takrar > 1 {
            let _ = write!(satr, " (×{})", self.takrar);
        }
        if self.arabi.is_none() {
            satr.push_str(" — لم تصل الترجمة بعد");
        }
        if self.maqtu {
            satr.push_str(" — نصّ مقطوع");
        }
        satr
    }
}

/// A line trimmed and cut to [`AQSA_TUL_SATR`] characters.
///
/// Cut on a character boundary by construction: taking `chars()` rather than
/// slicing bytes is the difference between a shortened line and a panic on a
/// multi-byte codepoint, and slicing is denied in this workspace for exactly
/// this class of mistake.
fn iqtata(nass: &str) -> (String, bool) {
    let munaqqah = nass.trim();
    if munaqqah.chars().count() <= AQSA_TUL_SATR {
        return (munaqqah.to_owned(), false);
    }
    (munaqqah.chars().take(AQSA_TUL_SATR).collect(), true)
}

/// What appending a reading did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatijatIdraj {
    /// A new entry was added.
    Judida(MuarrifMadkhal),

    /// The reading repeated the region's previous line, and raised its count.
    Mukarrara(MuarrifMadkhal),

    /// The reading was empty once trimmed, and nothing was recorded.
    ///
    /// The normal answer for a region whose text has just left the screen: the
    /// recognizer succeeded and found nothing. Recording it would put a blank
    /// row between every pair of real ones.
    Farigha,
}

impl NatijatIdraj {
    /// The entry this reading landed on, when it landed on one.
    #[must_use]
    pub const fn muarrif(self) -> Option<MuarrifMadkhal> {
        match self {
            Self::Judida(muarrif) | Self::Mukarrara(muarrif) => Some(muarrif),
            Self::Farigha => None,
        }
    }

    /// Whether this reading added a row rather than raising a count.
    #[must_use]
    pub const fn judida(self) -> bool {
        matches!(self, Self::Judida(_))
    }
}

/// What the control panel says about the history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaqreerSijill {
    /// How many entries are held right now.
    pub mahfuza: usize,

    /// The capacity they are held against.
    pub saa: usize,

    /// How many entries have been pushed off the end since this history was
    /// opened.
    ///
    /// Reported rather than hidden. A player who cannot find a line from an hour
    /// ago is owed the difference between "it was never read" and "it was read
    /// and the history is four hundred lines long".
    pub mustabaada: u64,

    /// How many distinct regions have produced at least one entry.
    pub manatiq: usize,

    /// How many lines of the file could not be parsed and were skipped.
    pub satur_talifa: u64,

    /// How many in-memory changes the file does not yet reflect.
    ///
    /// Raised by every repeat that bumped a count and by every translation that
    /// arrived after its line was already appended, and cleared by a compaction.
    pub muallaqa: u64,
}

impl TaqreerSijill {
    /// The sentence the panel shows, in English, for a log line and a report.
    ///
    /// Stated the way [`crate::wajiha::MeezaniyatItar::wasf`] states the frame
    /// cost: the real numbers, including the unflattering ones.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = format!(
            "{} of {} entries held, {} evicted, {} region(s) seen",
            self.mahfuza, self.saa, self.mustabaada, self.manatiq
        );
        if self.satur_talifa > 0 {
            let _ = write!(wasf, ", {} malformed line(s) skipped on load", self.satur_talifa);
        }
        if self.muallaqa > 0 {
            let _ = write!(wasf, ", {} change(s) not yet compacted to disk", self.muallaqa);
        }
        wasf
    }

    /// The same sentence in Arabic, for the panel.
    #[must_use]
    pub fn unwan(&self) -> String {
        let mut unwan = format!(
            "{} من {} سطرًا، وأُسقط {} سطرًا، من {} منطقة.",
            self.mahfuza, self.saa, self.mustabaada, self.manatiq
        );
        if self.satur_talifa > 0 {
            let _ = write!(unwan, " وتُخطّي {} سطرًا تالفًا عند التحميل.", self.satur_talifa);
        }
        unwan
    }
}

/// The first line of a history file: which game, which schema, what capacity.
///
/// A separate type from the entries because it is a separate concern. The
/// version lives here and nowhere else — repeating it on every line would cost
/// bytes on every append to answer a question that can only have one answer per
/// file, and would invite a file whose lines disagree with each other about
/// their own shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarwisatSijill {
    /// The game this history belongs to.
    pub luba: LubaId,

    /// The capacity the file was written against.
    pub saa: usize,

    /// The next entry identity to hand out.
    ///
    /// Persisted for the same reason the region set persists its own: an
    /// identity reused after a compaction dropped the entry that held it would
    /// make a panel's stored selection point at somebody else's line.
    pub talee: u64,
}

impl DhuMukhattat for TarwisatSijill {
    const ISM: &'static str = "sijill_qira";

    /// One. There has never been another shape of this file.
    const ISDAR: u32 = 1;

    fn hijra(min: u32, _qeema: Value) -> Natija<Value> {
        // No earlier version exists to migrate from. A header claiming one was
        // not written by any build of Taarib, and reading the entries behind it
        // would be reading a shape nothing here has ever produced.
        Err(Khata::from(KhataTabaqa::MalafGhayrMafhum {
            masar: PathBuf::from(ISM_MALAF),
            sigha: Self::ISM,
            sabab: format!(
                "the history header claims schema {min}, and this build knows only {}",
                Self::ISDAR
            ),
        }))
    }
}

/// The reading history: bounded, deduplicated, searchable, and durable.
///
/// Bounded because it lives inside a game's address space and a session lasts
/// hours. Oldest-first eviction, and the count of what was evicted is reported
/// rather than swallowed — see [`TaqreerSijill::mustabaada`].
#[derive(Debug, Clone)]
pub struct SijillQira {
    madakhil: VecDeque<MadkhalQira>,
    saa: usize,
    talee: u64,
    luba: LubaId,
    mustabaada: u64,
    satur_talifa: u64,
    masar: Option<PathBuf>,
    sutur_malaf: u64,
    muallaqa: u64,
}

impl SijillQira {
    /// How many lines the file may hold beyond the capacity before a compaction
    /// is worth doing.
    ///
    /// Twice. The file is append-only, so it grows past the in-memory capacity
    /// by exactly the number of entries that have been evicted; compacting on
    /// every eviction would rewrite five hundred lines to drop one, and never
    /// compacting would leave a session's whole history on disk under a header
    /// that says five hundred.
    pub const MUAMIL_RASS: u64 = 2;

    /// How many uncompacted in-memory changes are tolerated before a compaction
    /// is worth doing.
    ///
    /// Repeats and late-arriving translations mutate entries that are already on
    /// disk, and an append-only file cannot express a mutation. Sixty-four of
    /// them is a few minutes of play.
    pub const HADD_MUALLAQA: u64 = 64;

    /// An empty history at the default capacity, with no file behind it.
    #[must_use]
    pub fn jadeed(luba: LubaId) -> Self {
        Self::bi_saa(luba, SAA_IFTIRADIYA)
    }

    /// An empty history at a chosen capacity, clamped into range.
    #[must_use]
    pub fn bi_saa(luba: LubaId, saa: usize) -> Self {
        Self {
            madakhil: VecDeque::new(),
            saa: saa.clamp(ADNA_SAA, AQSA_SAA),
            talee: 1,
            luba,
            mustabaada: 0,
            satur_talifa: 0,
            masar: None,
            sutur_malaf: 0,
            muallaqa: 0,
        }
    }

    /// Which game this history belongs to.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// The capacity.
    #[must_use]
    pub const fn saa(&self) -> usize {
        self.saa
    }

    /// How many entries are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.madakhil.len()
    }

    /// Whether anything has been recorded.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.madakhil.is_empty()
    }

    /// The file this history appends to, once one is open.
    #[must_use]
    pub fn masar(&self) -> Option<&Path> {
        self.masar.as_deref()
    }

    /// Every entry, oldest first.
    pub fn madakhil(&self) -> impl Iterator<Item = &MadkhalQira> {
        self.madakhil.iter()
    }

    /// One entry by identity.
    #[must_use]
    pub fn madkhal(&self, muarrif: MuarrifMadkhal) -> Option<&MadkhalQira> {
        self.madakhil.iter().find(|madkhal| madkhal.muarrif == muarrif)
    }

    /// Changes the capacity, evicting immediately if it shrank.
    ///
    /// Evicting on the spot rather than lazily is deliberate: a player who
    /// lowers the capacity to free memory inside a game that is struggling
    /// should get the memory back on that frame, not on the next line of
    /// dialogue.
    pub fn ihdud_saa(&mut self, saa: usize) {
        self.saa = saa.clamp(ADNA_SAA, AQSA_SAA);
        self.qallim();
    }

    /// Records a reading, suppressing a repeat of the region's previous line.
    ///
    /// The entry point the recognition path calls on every completed pass. See
    /// the module documentation for why the suppression is scoped to the region
    /// rather than to the history as a whole.
    pub fn sajjil(
        &mut self,
        lahza: u64,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> NatijatIdraj {
        let (nass, maqtu) = iqtata(asl);
        if nass.is_empty() {
            return NatijatIdraj::Farigha;
        }

        let sabiq = self
            .madakhil
            .iter_mut()
            .rev()
            .find(|madkhal| madkhal.mintaqa == mintaqa);
        if let Some(madkhal) = sabiq
            && madkhal.asl == nass
        {
            madkhal.takrar = madkhal.takrar.saturating_add(1);
            madkhal.lahza_akhira = lahza.max(madkhal.lahza_akhira);
            // The recognizer can be more certain on a later frame — a fade-in
            // finishing, a background settling — and keeping the best reading's
            // confidence is more useful than keeping the first's. A measured
            // reading replaces an unmeasured one outright rather than
            // competing with it numerically: the stand-in constant is not a
            // number a later real measurement should have to beat.
            if thiqa.maqisa() && !madkhal.maqisa {
                madkhal.thiqa = thiqa.mia();
                madkhal.maqisa = true;
            } else if thiqa.maqisa() == madkhal.maqisa {
                madkhal.thiqa = madkhal.thiqa.max(thiqa.mia());
            }
            let muarrif = madkhal.muarrif;
            self.muallaqa = self.muallaqa.saturating_add(1);
            return NatijatIdraj::Mukarrara(muarrif);
        }

        let muarrif = MuarrifMadkhal(self.talee);
        self.talee = self.talee.saturating_add(1);
        let madkhal = MadkhalQira {
            muarrif,
            lahza,
            lahza_akhira: lahza,
            mintaqa,
            ism_mintaqa: ism_mintaqa.to_owned(),
            asl: nass,
            arabi: None,
            thiqa: thiqa.mia(),
            maqisa: thiqa.maqisa(),
            masdar: None,
            takrar: 1,
            maqtu,
        };
        self.madakhil.push_back(madkhal);
        self.qallim();
        NatijatIdraj::Judida(muarrif)
    }

    /// Attaches a translation to an entry that was recorded without one.
    ///
    /// Returns whether the entry was still in the history. A translation that
    /// comes back after its line has been evicted is dropped, and that is the
    /// right answer — the player has already scrolled past it by four hundred
    /// lines.
    pub fn adkhil_tarjama(
        &mut self,
        muarrif: MuarrifMadkhal,
        arabi: &str,
        masdar: MasdarTarjama,
    ) -> bool {
        let (nass, _) = iqtata(arabi);
        let Some(madkhal) = self.madakhil.iter_mut().find(|madkhal| madkhal.muarrif == muarrif)
        else {
            return false;
        };
        madkhal.arabi = Some(nass);
        madkhal.masdar = Some(masdar);
        self.muallaqa = self.muallaqa.saturating_add(1);
        true
    }

    /// Every entry whose source or translation contains a needle, oldest first.
    #[must_use]
    pub fn bahth(&self, matlub: &str) -> Vec<&MadkhalQira> {
        self.madakhil.iter().filter(|madkhal| madkhal.yutabiq(matlub)).collect()
    }

    /// Every entry from one region, oldest first.
    #[must_use]
    pub fn min_mintaqa(&self, mintaqa: MuarrifMintaqa) -> Vec<&MadkhalQira> {
        self.madakhil.iter().filter(|madkhal| madkhal.mintaqa == mintaqa).collect()
    }

    /// The most recent `adad` entries, still oldest first.
    ///
    /// Oldest first even though these are the *last* entries, because that is
    /// reading order for a transcript and reversing it in the panel would put
    /// the sentence a player is trying to catch up to at the top of the page
    /// rather than at the bottom where they were last looking.
    #[must_use]
    pub fn akhir(&self, adad: usize) -> Vec<&MadkhalQira> {
        let tajawuz = self.madakhil.len().saturating_sub(adad);
        self.madakhil.iter().skip(tajawuz).collect()
    }

    /// Which regions have produced at least one entry.
    #[must_use]
    pub fn manatiq_masjula(&self) -> BTreeSet<MuarrifMintaqa> {
        self.madakhil.iter().map(|madkhal| madkhal.mintaqa).collect()
    }

    /// Forgets every entry, keeping the capacity, the identity counter and the
    /// file.
    ///
    /// The counter is kept on purpose: clearing the history and then handing out
    /// identity 1 again would let a panel's stored selection resolve to a
    /// different line than the one it was pointing at.
    pub fn imsah(&mut self) {
        let masqut = u64::try_from(self.madakhil.len()).unwrap_or(u64::MAX);
        self.mustabaada = self.mustabaada.saturating_add(masqut);
        self.madakhil.clear();
        self.muallaqa = self.muallaqa.saturating_add(1);
    }

    /// The summary the control panel shows.
    #[must_use]
    pub fn taqreer(&self) -> TaqreerSijill {
        TaqreerSijill {
            mahfuza: self.madakhil.len(),
            saa: self.saa,
            mustabaada: self.mustabaada,
            manatiq: self.manatiq_masjula().len(),
            satur_talifa: self.satur_talifa,
            muallaqa: self.muallaqa,
        }
    }

    /// Drops entries off the front until the history fits its capacity.
    fn qallim(&mut self) {
        while self.madakhil.len() > self.saa {
            if self.madakhil.pop_front().is_some() {
                self.mustabaada = self.mustabaada.saturating_add(1);
            }
        }
    }

    /// Where this game's history file lives under a directory of them.
    #[must_use]
    pub fn masar_malaf(mujallad: &Path, luba: LubaId) -> PathBuf {
        mujallad.join(luba.to_string()).join(ISM_MALAF)
    }

    /// Opens a history file, reading whatever is already in it.
    ///
    /// A missing file is a first session and produces an empty history with the
    /// path remembered. A file that exists and cannot be read is *not* treated
    /// as empty: appending to it afterwards would interleave this session's
    /// lines with a header this build never verified, and compacting would
    /// destroy a history that is sitting right there.
    ///
    /// The file is compacted once on open. That guarantees the header exists
    /// and was written by this build, so every later append is a pure append
    /// onto a file whose first line has already been checked — and it is the
    /// one moment where dropping the lines that overflow the capacity costs
    /// nothing, because the whole file is being rewritten anyway.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the file or its directory cannot be read
    /// or created, and [`KhataTabaqa::MalafGhayrMafhum`] when the header is
    /// missing, is not an object, carries no schema version, carries one this
    /// build does not know, or names a different game.
    pub fn iftah(masar: &Path, luba: LubaId, saa: usize) -> Result<Self, KhataTabaqa> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(sabab) => {
                return Err(KhataTabaqa::KhataMalaf { masar: masar.to_path_buf(), sabab });
            }
        };

        let mut sijill = if bayt.iter().any(|wahid| !wahid.is_ascii_whitespace()) {
            Self::min_bayt(masar, luba, &bayt)?
        } else {
            // Zero bytes is what `create(true)` leaves behind when a process
            // died between creating the file and writing the header. It is a new
            // history, not a damaged one.
            Self::bi_saa(luba, saa)
        };
        sijill.saa = saa.clamp(ADNA_SAA, AQSA_SAA);
        sijill.qallim();
        sijill.masar = Some(masar.to_path_buf());
        sijill.irsus()?;
        Ok(sijill)
    }

    /// Parses a whole history file, skipping lines that will not parse.
    ///
    /// Split on bytes rather than decoded as one string first: a torn append
    /// leaves invalid UTF-8 at the end of the file, and decoding the whole file
    /// up front would turn one broken line into a refusal to load any of it.
    fn min_bayt(masar: &Path, luba: LubaId, bayt: &[u8]) -> Result<Self, KhataTabaqa> {
        let ghalat = |sabab: String| KhataTabaqa::MalafGhayrMafhum {
            masar: masar.to_path_buf(),
            sigha: TarwisatSijill::ISM,
            sabab,
        };

        let mut sutur = bayt
            .split(|wahid| *wahid == b'\n')
            .filter(|satr| satr.iter().any(|wahid| !wahid.is_ascii_whitespace()));

        let Some(satr_tarwisa) = sutur.next() else {
            return Err(ghalat("it holds no header line".to_owned()));
        };
        let tarwisa = Self::hallil_tarwisa(satr_tarwisa, &ghalat)?;
        if tarwisa.luba != luba {
            return Err(ghalat(format!(
                "its header names game {}, and history for game {luba} was asked for. \
                 Appending to it would mix two games' lines into one transcript.",
                tarwisa.luba
            )));
        }

        let mut sijill = Self::bi_saa(luba, tarwisa.saa);
        sijill.talee = tarwisa.talee.max(1);
        sijill.sutur_malaf = 1;

        for satr in sutur {
            sijill.sutur_malaf = sijill.sutur_malaf.saturating_add(1);
            let Ok(nass) = std::str::from_utf8(satr) else {
                sijill.satur_talifa = sijill.satur_talifa.saturating_add(1);
                continue;
            };
            match serde_json::from_str::<MadkhalQira>(nass) {
                Ok(mut madkhal) => {
                    madkhal.thiqa = madkhal.thiqa.min(100);
                    // A stored entry that claims no measurement carries no
                    // number either. Leaving one behind would let a harvest
                    // read a confidence out of an entry that says it has none.
                    if !madkhal.maqisa {
                        madkhal.thiqa = 0;
                    }
                    madkhal.takrar = madkhal.takrar.max(1);
                    madkhal.lahza_akhira = madkhal.lahza_akhira.max(madkhal.lahza);
                    sijill.talee = sijill.talee.max(madkhal.muarrif.raqm().saturating_add(1));
                    sijill.madakhil.push_back(madkhal);
                    sijill.qallim();
                }
                Err(_) => {
                    // Skipped and counted, never fatal. A history is not worth
                    // losing to one line that a power cut cut in half, and the
                    // count is surfaced in `TaqreerSijill` so the panel can say
                    // what happened instead of quietly showing a short history.
                    sijill.satur_talifa = sijill.satur_talifa.saturating_add(1);
                }
            }
        }
        Ok(sijill)
    }

    /// Reads the header line, refusing a schema version this build does not
    /// write.
    fn hallil_tarwisa(
        satr: &[u8],
        ghalat: &impl Fn(String) -> KhataTabaqa,
    ) -> Result<TarwisatSijill, KhataTabaqa> {
        let nass = std::str::from_utf8(satr)
            .map_err(|khata| ghalat(format!("its header line is not UTF-8: {khata}")))?;
        let qeema: Value = serde_json::from_str(nass)
            .map_err(|khata| ghalat(format!("its header line is not JSON: {khata}")))?;
        let Some(kain) = qeema.as_object() else {
            return Err(ghalat("its header line is not a JSON object".to_owned()));
        };
        let Some(mawjud) = kain.get(mukhattat::HAQL).and_then(Value::as_u64) else {
            return Err(ghalat(format!(
                "its header carries no `{}` field, so there is no way to tell which shape \
                 the lines behind it are in",
                mukhattat::HAQL
            )));
        };
        if mawjud != u64::from(TarwisatSijill::ISDAR) {
            return Err(ghalat(format!(
                "its header declares schema version {mawjud}, and this build reads and \
                 writes version {}. The entries are not looked at: a newer build wrote \
                 fields this one cannot see, and the first compaction would write them \
                 away for good.",
                TarwisatSijill::ISDAR
            )));
        }
        mukhattat::min_qeema::<TarwisatSijill>(qeema).map_err(|khata| ghalat(khata.injilizi))
    }

    /// Records a reading and makes it durable, compacting when it is time.
    ///
    /// The entry point for a caller that wants the history on disk.
    /// [`SijillQira::sajjil`] is the same logic without the file, for a caller
    /// that manages its own durability or has no path.
    ///
    /// A new line is appended. A repeat is not: an append-only file cannot
    /// express "the entry three hundred lines back now has a count of nine", so
    /// the change is held in memory and settled by the next compaction.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the append or the compaction cannot
    /// complete, and [`KhataTabaqa::MalafGhayrMafhum`] when an entry will not
    /// serialize — which cannot happen for a value this module built and is
    /// reported rather than swallowed if it ever does.
    pub fn qayyid(
        &mut self,
        lahza: u64,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> Result<NatijatIdraj, KhataTabaqa> {
        let natija = self.sajjil(lahza, mintaqa, ism_mintaqa, asl, thiqa);
        if let NatijatIdraj::Judida(muarrif) = natija {
            let satr = self
                .madakhil
                .back()
                .filter(|madkhal| madkhal.muarrif == muarrif)
                .map(|madkhal| self.satr_madkhal(madkhal))
                .transpose()?;
            if let Some(satr) = satr {
                self.alhiq_satr(&satr)?;
            }
        }
        if self.yahtaj_rass() {
            self.irsus()?;
        }
        Ok(natija)
    }

    /// Whether the file has drifted far enough from memory to be worth
    /// rewriting.
    #[must_use]
    pub fn yahtaj_rass(&self) -> bool {
        if self.masar.is_none() {
            return false;
        }
        let saa = u64::try_from(self.saa).unwrap_or(u64::MAX);
        self.muallaqa >= Self::HADD_MUALLAQA
            || self.sutur_malaf > saa.saturating_mul(Self::MUAMIL_RASS)
    }

    /// Rewrites the file as exactly what is in memory, atomically.
    ///
    /// Header first, then one line per entry, through
    /// [`taarib_usus::masarat::kitaba_dharra`] — beside, flushed, renamed. This
    /// is the only durable point in the file's life and it is deliberately not
    /// on the append path: see [`SijillQira::alhiq_satr`].
    ///
    /// Does nothing when no file is open.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the write cannot complete, and
    /// [`KhataTabaqa::MalafGhayrMafhum`] when the header or an entry will not
    /// serialize.
    pub fn irsus(&mut self) -> Result<(), KhataTabaqa> {
        let Some(masar) = self.masar.clone() else {
            return Ok(());
        };
        let tarwisa = TarwisatSijill { luba: self.luba, saa: self.saa, talee: self.talee };
        let mut nass = self.satr_tarwisa(&tarwisa)?;
        nass.push('\n');
        for madkhal in &self.madakhil {
            nass.push_str(&self.satr_madkhal(madkhal)?);
            nass.push('\n');
        }
        masarat::kitaba_dharra(&masar, nass.as_bytes()).map_err(|khata| {
            KhataTabaqa::KhataMalaf {
                masar: masar.clone(),
                sabab: std::io::Error::other(khata.injilizi),
            }
        })?;
        self.sutur_malaf = u64::try_from(self.madakhil.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        self.muallaqa = 0;
        Ok(())
    }

    /// Compacts and forgets the file, for a session that is ending.
    ///
    /// # Errors
    ///
    /// As [`SijillQira::irsus`].
    pub fn aghliq(&mut self) -> Result<(), KhataTabaqa> {
        self.irsus()?;
        self.masar = None;
        Ok(())
    }

    /// Appends one already-serialized line to the open file.
    ///
    /// **Not flushed to the device.** That is a decision rather than an
    /// omission: this runs on the path a game's frames go through, and an fsync
    /// per line of dialogue is a stall a player feels for a guarantee they do
    /// not need. A history is not a backup — the worst a power cut costs is the
    /// last few lines somebody read on screen a moment ago. The compaction *is*
    /// durable, because rewriting the file non-atomically is how a history gets
    /// destroyed rather than shortened.
    fn alhiq_satr(&mut self, satr: &str) -> Result<(), KhataTabaqa> {
        let Some(masar) = self.masar.as_ref() else {
            return Ok(());
        };
        let khata = |sabab: std::io::Error| KhataTabaqa::KhataMalaf {
            masar: masar.clone(),
            sabab,
        };
        let mut malaf = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(masar)
            .map_err(khata)?;
        malaf.write_all(satr.as_bytes()).map_err(khata)?;
        malaf.write_all(b"\n").map_err(khata)?;
        self.sutur_malaf = self.sutur_malaf.saturating_add(1);
        Ok(())
    }

    /// The header as one compact line, stamped with the schema version.
    ///
    /// Stamped here rather than through [`taarib_usus::mukhattat::iktub`],
    /// which pretty-prints: a header spread over six lines is a header that the
    /// line-oriented reader would treat as six malformed entries.
    fn satr_tarwisa(&self, tarwisa: &TarwisatSijill) -> Result<String, KhataTabaqa> {
        let mut qeema = serde_json::to_value(tarwisa).map_err(|khata| self.ghalat(&khata))?;
        let Some(kain) = qeema.as_object_mut() else {
            return Err(KhataTabaqa::MalafGhayrMafhum {
                masar: self.masar.clone().unwrap_or_else(|| PathBuf::from(ISM_MALAF)),
                sigha: TarwisatSijill::ISM,
                sabab: "the history header did not serialize to a JSON object".to_owned(),
            });
        };
        let _ = kain.insert(mukhattat::HAQL.to_owned(), Value::from(TarwisatSijill::ISDAR));
        serde_json::to_string(&qeema).map_err(|khata| self.ghalat(&khata))
    }

    /// One entry as one compact line.
    fn satr_madkhal(&self, madkhal: &MadkhalQira) -> Result<String, KhataTabaqa> {
        serde_json::to_string(madkhal).map_err(|khata| self.ghalat(&khata))
    }

    /// A serialization failure, named against the file it was going into.
    fn ghalat(&self, khata: &serde_json::Error) -> KhataTabaqa {
        KhataTabaqa::MalafGhayrMafhum {
            masar: self.masar.clone().unwrap_or_else(|| PathBuf::from(ISM_MALAF)),
            sigha: TarwisatSijill::ISM,
            sabab: format!("a history line could not be written as JSON: {khata}"),
        }
    }
}

/// A history two threads share: recognition writes it, the panel reads it.
///
/// The two sides have opposite requirements. Recognition runs off the render
/// thread and can afford to wait for the lock. The panel is built *inside*
/// somebody's frame, where waiting on a lock another thread is holding is a
/// stutter the player sees — so the render side has
/// [`SijillMushtarak::laqta_in_amkan`], which gives up rather than blocks, and
/// the panel simply shows the previous frame's rows when it does. One frame of
/// slightly stale history is invisible; one frame of hitch is not.
///
/// Snapshots are *copies*. The panel never holds the lock while it lays itself
/// out, because laying out is the expensive part and holding a lock across it
/// would hand the render thread's cost to the recognition thread.
#[derive(Debug, Clone)]
pub struct SijillMushtarak(Arc<RwLock<SijillQira>>);

impl SijillMushtarak {
    /// Wraps a history for sharing.
    #[must_use]
    pub fn jadeed(sijill: SijillQira) -> Self {
        Self(Arc::new(RwLock::new(sijill)))
    }

    /// Records a reading in memory only.
    #[must_use] 
    pub fn sajjil(
        &self,
        lahza: u64,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> NatijatIdraj {
        self.0.write().sajjil(lahza, mintaqa, ism_mintaqa, asl, thiqa)
    }

    /// Records a reading and makes it durable.
    ///
    /// # Errors
    ///
    /// As [`SijillQira::qayyid`].
    pub fn qayyid(
        &self,
        lahza: u64,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> Result<NatijatIdraj, KhataTabaqa> {
        self.0.write().qayyid(lahza, mintaqa, ism_mintaqa, asl, thiqa)
    }

    /// Attaches a translation that arrived after its line was recorded.
    #[must_use] 
    pub fn adkhil_tarjama(
        &self,
        muarrif: MuarrifMadkhal,
        arabi: &str,
        masdar: MasdarTarjama,
    ) -> bool {
        self.0.write().adkhil_tarjama(muarrif, arabi, masdar)
    }

    /// The most recent entries, copied out, waiting for the lock if it is held.
    #[must_use]
    pub fn laqta(&self, adad: usize) -> Vec<MadkhalQira> {
        self.0.read().akhir(adad).into_iter().cloned().collect()
    }

    /// The most recent entries, or [`None`] rather than a wait.
    ///
    /// What the render thread calls. See this type's documentation.
    #[must_use]
    pub fn laqta_in_amkan(&self, adad: usize) -> Option<Vec<MadkhalQira>> {
        self.0
            .try_read()
            .map(|sijill| sijill.akhir(adad).into_iter().cloned().collect())
    }

    /// Every entry matching a needle, capped so a one-character search does not
    /// copy the whole history into a frame.
    #[must_use]
    pub fn bahth(&self, matlub: &str, saqf: usize) -> Vec<MadkhalQira> {
        self.0.read().bahth(matlub).into_iter().take(saqf).cloned().collect()
    }

    /// Every entry from one region, capped.
    #[must_use]
    pub fn min_mintaqa(&self, mintaqa: MuarrifMintaqa, saqf: usize) -> Vec<MadkhalQira> {
        self.0.read().min_mintaqa(mintaqa).into_iter().take(saqf).cloned().collect()
    }

    /// The summary for the control panel.
    #[must_use]
    pub fn taqreer(&self) -> TaqreerSijill {
        self.0.read().taqreer()
    }

    /// The summary, or [`None`] rather than a wait.
    #[must_use]
    pub fn taqreer_in_amkan(&self) -> Option<TaqreerSijill> {
        self.0.try_read().map(|sijill| sijill.taqreer())
    }

    /// Compacts the file behind the history.
    ///
    /// # Errors
    ///
    /// As [`SijillQira::irsus`].
    pub fn irsus(&self) -> Result<(), KhataTabaqa> {
        self.0.write().irsus()
    }

    /// Compacts and closes.
    ///
    /// # Errors
    ///
    /// As [`SijillQira::aghliq`].
    pub fn aghliq(&self) -> Result<(), KhataTabaqa> {
        self.0.write().aghliq()
    }
}
