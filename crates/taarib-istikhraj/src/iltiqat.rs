//! الالتقاط — recording every string a game draws, while it draws it.
//!
//! Capture is usually described as the fallback for games whose containers
//! cannot be read, and it is that. It is also something static extraction can
//! never be, and that second role is why this module exists even for a game
//! whose every string was read cleanly out of a `.locres`:
//!
//! **The width a string has on screen is not in the game's files.** It is the
//! product of a layout that ran — an anchored rectangle resolved against a
//! canvas scaler, a font size an auto-sizer chose, a parent that shrank because
//! a sibling grew. Nothing in a container declares it. A static extractor can
//! read `Continue` and cannot know whether it was drawn into a 400-pixel banner
//! or a 62-pixel button, and the Arabic for `Continue` fits one of those and
//! clips out of the other. Phase 14's overflow report is computed against
//! [`QuyudNass::aqsa_ard`], and the only place that number can honestly come
//! from is a measurement taken while the original was on screen. That is what a
//! capture session produces, and it is why capture *supplements* a complete
//! static extraction rather than only rescuing an incomplete one.
//!
//! ## This module opens no channel
//!
//! Nothing here opens a socket, a pipe, a port or a watcher. An adapter running
//! inside a game writes its session to a file — the format `Iltiqat` in
//! `Taarib.Unity.Mushtarak` writes out in full — and this module reads it. That
//! is the whole handoff, and it is deliberately not a live transport: a game
//! process can be killed at any moment, and a file already flushed is a session
//! that survives it, while a socket's buffered tail is not.
//!
//! [`JalsatIltiqat::sajjil_faqd`] exists because the adapter drops records when
//! its own bounded queue is full, and a session that quietly sampled forty
//! percent is a coverage number that is a lie. The adapter's drop count travels
//! in the file and is folded into this session's honesty accounting rather than
//! living in a log nobody correlates.
//!
//! ## The four things that are hard
//!
//! | problem | what happens if it is got wrong |
//! | --- | --- |
//! | a string is drawn every frame | forty thousand identical records for one label, and a session file larger than the game |
//! | the process can be killed at any moment | a two-hour play session that produced nothing, because the results were still in memory |
//! | capture shares a process with a game holding sixty frames a second | the user's game stutters, and they turn capture off — which costs every measurement, not just the expensive ones |
//! | a session that quietly sampled forty percent | a coverage number that is a lie, and a patch shipped believing it was finished |
//!
//! Each is answered below, in [`SijillMulahazat`], [`KatibJalsa`],
//! [`MizaniyatItar`] and [`TaqreerJalsa`] respectively.
//!
//! ## Two clocks, and the file does not pretend otherwise
//!
//! [`HadathIltiqat::waqt_mil`] is the *adapter's* milliseconds since it was told
//! to start capturing, because that is the only clock the adapter can read
//! cheaply on a render thread. [`TarwisatJalsa::bada`] is *Studio's* absolute
//! wall-clock start. They are not the same clock and the session file never
//! interpolates between them. What the file guarantees is monotonic ordering
//! within the adapter's own session, which is what "this string was drawn before
//! that one" needs; nothing downstream depends on absolute per-event time, and
//! writing an RFC 3339 timestamp per event would have meant formatting a string
//! sixty times a second for a precision nobody uses.

use std::collections::hash_map::Entry;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::hash::Hasher as _;
use std::io::{BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::time::Instant;

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use serde::{Deserialize, Serialize};
use taarib_mustalahat::nass::{Mustatil, QuyudNass};
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat;

use crate::khata::KhataIstikhraj;

// ---------------------------------------------------------------------------
// Ceilings and defaults
// ---------------------------------------------------------------------------

/// The schema version written into every session file's header.
///
/// Read back before anything else. A file from a newer build is refused rather
/// than parsed optimistically, because a record this build does not understand
/// is a measurement it would silently drop, and a silently dropped measurement
/// becomes a missing overflow warning three phases later.
pub const ISDAR_JALSA: u32 = 1;

/// The default per-frame budget for capture work, in microseconds.
///
/// Two hundred microseconds of a 16 667 µs frame — one and a fifth percent. The
/// number is chosen against what it buys rather than against what feels small:
/// hashing and folding a repeat observation costs on the order of a hundred
/// nanoseconds, so this budget covers roughly two thousand repeat observations
/// per frame, which is more text than any game draws in one. A frame that
/// exceeds it is a frame doing something unusual, and that is exactly when
/// backing off is correct.
pub const MIZANIYAT_ITAR_MIKRO: u32 = 200;

/// The hard ceiling on events accepted in a single frame.
///
/// The budget degrades gracefully; this does not. It exists because the graceful
/// path still has to hash every event to know whether the string is new, and a
/// frame that offers a hundred thousand events — a debug overlay left on, a text
/// system in a loop, a hostile build — would spend real milliseconds doing that
/// before the sampler could help. Events refused by this ceiling are counted
/// separately from sampled ones precisely because they *can* include strings
/// never seen before, which makes them the number that turns coverage into a
/// floor rather than a measurement.
pub const AQSA_AHDATH_ITAR: u32 = 4096;

/// The largest sampling stride the budget is allowed to reach.
///
/// At sixty-four, one repeat observation in sixty-four is folded and the rest
/// are counted and discarded. Past that the counter is so coarse that raising it
/// further buys nothing measurable, and an unbounded stride would let a single
/// pathological second suppress capture for the rest of the session.
pub const AQSA_KHUTWA: u32 = 64;

/// The default bound on distinct strings held in one session.
///
/// Two hundred thousand records at roughly two hundred bytes each is about forty
/// megabytes, which is a size Studio can hold while somebody plays. See
/// [`SijillMulahazat`] for what happens at the bound and why refusing is the
/// least dishonest of the available failures.
pub const AQSA_MULAHAZAT: usize = 200_000;

/// The batch size at which pending records are pushed to the device, in bytes.
pub const HADD_DUFA_BAYT: usize = 65_536;

/// The longest a batch may sit unflushed, in milliseconds.
///
/// This is the number that bounds what a crash costs. See [`KatibJalsa`].
pub const MUDDAT_DUFA_MIL: u64 = 2_000;

/// The longest captured string kept, in bytes.
///
/// A text system handed a megabyte of concatenated log output is not drawing a
/// translatable string, and a session file is not the place to find out. Longer
/// strings are refused with a counter rather than truncated: a truncated string
/// has no identity worth merging and would match nothing, so keeping a prefix
/// would only produce a table row that can never be written back.
pub const AQSA_TUL_NASS: usize = 16_384;

/// How many times a colliding key is re-probed before the event is refused.
///
/// Eight, which is unreachable in practice: it needs eight consecutive
/// hundred-and-twenty-eight-bit collisions. The bound exists because the
/// alternative to bounding the probe is a loop with no exit on a thread that a
/// game's frame is waiting behind.
pub const AQSA_TAHASSUS: u32 = 8;

/// Separates the two halves of a dedup key so `("ab", "c")` and `("a", "bc")`
/// cannot hash alike.
const FASIL_BASMA: u8 = 0x1F;

/// The leading salt of the dedup key's first half.
const MILH_AWWAL: u8 = 0xA5;

/// The leading salt of the dedup key's second half.
const MILH_THANI: u8 = 0x5A;

// ---------------------------------------------------------------------------
// One string, as it was drawn
// ---------------------------------------------------------------------------

/// One string exactly as a game drew it, with what it was drawn into.
///
/// Produced by an injected adapter at its takeover point and carried here over
/// `barid`. Everything on it is an observation; nothing on it is a conclusion.
///
/// The two width fields are the reason this type exists and they are *not*
/// interchangeable — see [`HadathIltiqat::quyud`], which is the only place they
/// are allowed to be turned into a constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HadathIltiqat {
    /// The text as the engine handed it to the layout call, markup included.
    ///
    /// Not cleaned here. Markup is lifted into spans exactly once in this
    /// product, by `jadwal::irfa_nasq`, and doing it on a render thread would
    /// put a markup parser on the game's frame path for no benefit.
    pub nass: String,

    /// The component path that drew it, in the engine's own terms:
    /// `Canvas/Panel/QuestList/Item(3)/Label` for Unity, a Slate widget path for
    /// Unreal, a node path for Godot.
    ///
    /// Half of the deduplication key, and the reason the other half is not
    /// enough: the same word drawn by two different widgets is two measurements
    /// with two different widths, and folding them on text alone would report
    /// the narrower of two unrelated boxes as the constraint on both.
    pub masar_mukawwin: String,

    /// The scene, level or map loaded when it was drawn.
    pub mashhad: Option<String>,

    /// The screen or interface state — `PauseMenu`, `Shop`, `Battle` — where the
    /// adapter can name one.
    ///
    /// Distinct from the scene: a Unity scene stays loaded while four different
    /// screens open and close inside it, and a translator reviewing "everything
    /// on the shop screen" is asking about this field, not that one.
    pub shasha: Option<String>,

    /// The rectangle the text actually occupied, in the game's own pixels.
    pub mustatil: Option<Mustatil>,

    /// The width the layout was *given*, in the game's own pixels.
    ///
    /// This is the constraint. [`HadathIltiqat::mustatil`] is what the original
    /// text happened to fill, which for an auto-sized label is the width of the
    /// English word and nothing more.
    pub ard_mutah: Option<f32>,

    /// The height the layout was given, where the engine bounded it.
    pub irtifa_mutah: Option<f32>,

    /// The size the text was drawn at, after any auto-sizing.
    pub hajm_khatt: Option<f32>,

    /// Whether the engine refused to wrap this text.
    pub satr_wahid: bool,

    /// The adapter's own monotonic send counter.
    ///
    /// Not decoration. `barid`'s sender is bounded and drops rather than blocking
    /// a game's render thread, so a session can legitimately be missing messages;
    /// a gap in this sequence is *direct* evidence of that, independent of the
    /// transport's own counter, and [`IhsaatJalsa::fajawat`] reports it.
    pub tasalsul: u64,

    /// Milliseconds since the adapter started capturing.
    ///
    /// The adapter's clock, not Studio's. See this module's header for why the
    /// two are never reconciled.
    pub waqt_mil: u64,

    /// A content-addressed screenshot crop of where it appeared, when the
    /// adapter's platform could take one.
    ///
    /// Carried through into the session file even though
    /// [`crate::jadwal::MudkhalMustakhraj`] has nowhere to put it: the workspace
    /// reads it from
    /// [`taarib_mustalahat::nass::SiyaqNass::laqta`] once a project row exists,
    /// and a crop that was discarded at capture time cannot be recovered by
    /// re-reading the game's files.
    pub laqta: Option<String>,
}

impl HadathIltiqat {
    /// The measured constraints this observation supports, and only those.
    ///
    /// ## The trap this method exists to avoid
    ///
    /// It is tempting to write `aqsa_ard: mustatil.map(|q| q.ard)` — a rectangle
    /// is right there and it is a width. It is the wrong width, and using it
    /// would poison every overflow check in the product.
    ///
    /// A great many interface labels size themselves to their content. For those
    /// the rectangle is the width of the English string, so recording it as the
    /// available width asserts that the translation must be no wider than the
    /// original — which for Arabic it very often is not, and the result would be
    /// an overflow warning on nearly every correctly translated string. A
    /// warning that fires on everything is a warning nobody reads.
    ///
    /// So [`QuyudNass::aqsa_ard`] is filled only from
    /// [`HadathIltiqat::ard_mutah`], which is a bound the engine imposed. The
    /// rectangle is still recorded, in [`QuyudNass::mustatil`], because it
    /// describes the occurrence and Phase 14 uses it to place a reflowed
    /// container — but it is never promoted into a bound it was not.
    #[must_use]
    pub fn quyud(&self) -> QuyudNass {
        QuyudNass {
            aqsa_ahruf: None,
            aqsa_ard: qeema_qiyas(self.ard_mutah),
            aqsa_irtifa: qeema_qiyas(self.irtifa_mutah),
            hajm_khatt: qeema_qiyas(self.hajm_khatt),
            mustatil: self.mustatil.filter(|mustatil| mustatil_salih(*mustatil)),
            satr_wahid: self.satr_wahid,
        }
    }

    /// Why this event cannot be recorded, when it cannot.
    ///
    /// Empty and whitespace-only draws are ordinary — a label cleared between
    /// screens, a placeholder before data loads — and they carry no string and no
    /// usable measurement. Over-long draws are a text system handed something
    /// that is not a translatable string.
    ///
    /// The rule lives here, in one place, rather than being restated inside
    /// [`JalsatIltiqat::istaqbil`]. A listener that wants to filter before paying
    /// for the send calls this; the session calls the same function; and the two
    /// cannot drift into disagreeing about what an empty string is.
    #[must_use]
    pub fn rafd(&self) -> Option<SababRafdHadath> {
        if self.nass.trim().is_empty() {
            return Some(SababRafdHadath::Farigh);
        }
        if self.nass.len() > AQSA_TUL_NASS {
            return Some(SababRafdHadath::Tawil {
                tul: self.nass.len(),
            });
        }
        None
    }

    /// Whether this event is worth recording at all.
    #[must_use]
    pub fn yustahaqq(&self) -> bool {
        self.rafd().is_none()
    }
}

/// A measurement that survives being trusted.
///
/// A degenerate layout — a component with a zero-sized parent, a canvas that has
/// not resolved yet, a division that produced an infinity — reports numbers that
/// are syntactically fine and semantically nothing. `NaN` in particular
/// propagates through `f32::min` in the table's constraint fold and would turn a
/// real width into a non-number for every later sighting of that string.
fn qeema_qiyas(qeema: Option<f32>) -> Option<f32> {
    qeema.filter(|q| q.is_finite() && *q > 0.0)
}

/// The same check for a rectangle: all four numbers finite, and a positive size.
fn mustatil_salih(mustatil: Mustatil) -> bool {
    mustatil.s.is_finite()
        && mustatil.a.is_finite()
        && mustatil.ard.is_finite()
        && mustatil.irtifa.is_finite()
        && mustatil.ard > 0.0
        && mustatil.irtifa > 0.0
}

// ---------------------------------------------------------------------------
// The deduplication key
// ---------------------------------------------------------------------------

/// The identity a session deduplicates on: text and component path together.
///
/// ## Why a hundred and twenty-eight bits, and why it is verified anyway
///
/// A sixty-four bit key over a hundred thousand distinct strings collides with
/// probability around three in ten billion. That is small, and the consequence
/// is not: two different strings folded into one record, one of them gone from
/// the session entirely, and no evidence anywhere that it happened. Nothing else
/// in this product accepts "very probably nothing was lost", so this does not
/// either.
///
/// Two hashes rather than one narrows that enormously — but `FxHash` is not a
/// cryptographic hash and two `FxHash` streams over the same bytes with
/// different leading salts are *not* provably independent, so the width alone is
/// not the argument. The argument is that [`MulahazaMutakarrira`] keeps the text
/// and the component path it was created from and a fold **compares them before
/// merging**, so a collision is detected rather than assumed away. When one is
/// detected the key is re-probed rather than the event dropped, which keeps both
/// strings. The comparison costs a length check and a `memcmp` of two identical
/// short strings on the overwhelmingly common path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiftahMulahaza {
    /// The first half.
    pub awwal: u64,
    /// The second half, incremented on a verified collision to re-probe.
    pub thani: u64,
}

impl MiftahMulahaza {
    /// Hashes a text and a component path into a key.
    #[must_use]
    pub fn min_hadath(nass: &str, masar_mukawwin: &str) -> Self {
        Self {
            awwal: shatr(MILH_AWWAL, nass, masar_mukawwin),
            thani: shatr(MILH_THANI, nass, masar_mukawwin),
        }
    }

    /// The key as thirty-two lowercase hexadecimal characters.
    ///
    /// The wire form used in the session file. Written as text rather than as
    /// two JSON numbers because a JSON number above 2^53 does not survive a
    /// round trip through a JavaScript reader, and Studio's interface is one.
    #[must_use]
    pub fn nass(self) -> String {
        let mut khaam = String::with_capacity(32);
        // Both halves are written full-width so the string is fixed length and
        // parses back by position rather than by searching for a separator.
        let _ = write!(&mut khaam, "{:016x}{:016x}", self.awwal, self.thani);
        khaam
    }

    /// Parses the form [`MiftahMulahaza::nass`] produces.
    ///
    /// Returns nothing for anything else, which is how a corrupted line in a
    /// session file is recognised and counted instead of being resurrected as a
    /// key that points at the wrong record.
    #[must_use]
    pub fn min_nass(khaam: &str) -> Option<Self> {
        if khaam.len() != 32 {
            return None;
        }
        let awwal = khaam
            .get(0..16)
            .and_then(|q| u64::from_str_radix(q, 16).ok())?;
        let thani = khaam
            .get(16..32)
            .and_then(|q| u64::from_str_radix(q, 16).ok())?;
        Some(Self { awwal, thani })
    }
}

/// One half of the key.
fn shatr(milh: u8, nass: &str, masar_mukawwin: &str) -> u64 {
    let mut hashib = FxHasher::default();
    hashib.write_u8(milh);
    hashib.write(nass.as_bytes());
    hashib.write_u8(FASIL_BASMA);
    hashib.write(masar_mukawwin.as_bytes());
    hashib.finish()
}

// ---------------------------------------------------------------------------
// One folded observation
// ---------------------------------------------------------------------------

/// Every sighting of one string by one component, folded into one record.
///
/// This is what a session accumulates and what [`crate::dammij`] merges into a
/// static extraction. A string drawn every frame for two hours is one of these
/// with a count of four hundred thousand, not four hundred thousand records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MulahazaMutakarrira {
    /// The key, as text.
    pub miftah: String,

    /// The text, exactly as drawn.
    pub nass: String,

    /// The component that drew it.
    pub masar_mukawwin: String,

    /// The scene it was first seen in.
    pub mashhad: Option<String>,

    /// The screen or interface state it was first seen in.
    pub shasha: Option<String>,

    /// How many times it was folded — sightings, not frames.
    ///
    /// Counts every sighting the session *accepted*, including the ones the
    /// sampler discarded without folding, so the number stays a true occurrence
    /// count even under degradation. What sampling costs is width tightening,
    /// not this.
    pub marrat: u64,

    /// The adapter's sequence number at the first sighting.
    pub awwal_tasalsul: u64,

    /// The adapter's clock at the first sighting, in milliseconds.
    pub awwal_waqt_mil: u64,

    /// The adapter's clock at the last folded sighting.
    pub akhir_waqt_mil: u64,

    /// The folded constraints: the tightest width ever observed, and the first
    /// rectangle and size.
    pub quyud: QuyudNass,

    /// The screenshot crop from the first sighting that had one.
    pub laqta: Option<String>,

    /// Whether any sighting of this string was discarded by the frame sampler.
    ///
    /// Per string rather than only per session, because "the count on this one
    /// entry is complete and its width is the tightest that was offered" is a
    /// different claim from "the session was under pressure at some point", and a
    /// reviewer asking whether a specific measurement can be trusted is asking
    /// the first question.
    pub muayyana: bool,
}

impl MulahazaMutakarrira {
    /// The first sighting, as a record.
    #[must_use]
    pub fn min_hadath(miftah: MiftahMulahaza, hadath: &HadathIltiqat) -> Self {
        Self {
            miftah: miftah.nass(),
            nass: hadath.nass.clone(),
            masar_mukawwin: hadath.masar_mukawwin.clone(),
            mashhad: hadath.mashhad.clone(),
            shasha: hadath.shasha.clone(),
            marrat: 1,
            awwal_tasalsul: hadath.tasalsul,
            awwal_waqt_mil: hadath.waqt_mil,
            akhir_waqt_mil: hadath.waqt_mil,
            quyud: hadath.quyud(),
            laqta: hadath.laqta.clone(),
            muayyana: false,
        }
    }

    /// Whether this record is really the one that key names.
    ///
    /// The collision check. See [`MiftahMulahaza`].
    #[must_use]
    pub fn yutabiq(&self, nass: &str, masar_mukawwin: &str) -> bool {
        self.nass == nass && self.masar_mukawwin == masar_mukawwin
    }

    /// Folds a repeat sighting in.
    ///
    /// **The tightest observed width wins.** This is the same rule
    /// `jadwal::dammij_quyud` applies when two sightings meet in the table,
    /// applied here to sightings of one string inside one session — so that the
    /// table is handed one already-folded constraint instead of four hundred
    /// thousand. It is not a second merge: what happens when capture meets
    /// static extraction is still decided in exactly one place, by
    /// [`crate::jadwal::JadwalNusus::adif`], and this module calls nothing else.
    ///
    /// Returns whether anything about the constraints actually changed, because
    /// the writer only spends a line on a change.
    pub fn dammij(&mut self, hadath: &HadathIltiqat) -> bool {
        self.marrat = self.marrat.saturating_add(1);
        self.akhir_waqt_mil = hadath.waqt_mil;

        let jadeed = hadath.quyud();
        let mut taghayyar = false;

        // `f32::min` rather than `<`. The incoming side is filtered by
        // `qeema_qiyas`, but the stored side can have come from a session file
        // this build did not write, and `f32::min` returns the operand that is
        // *not* NaN — so a corrupted stored width is repaired by the next
        // sighting rather than propagated into every later one. `<` on a NaN
        // answers nothing useful, which is why `float_cmp` is denied.
        if let Some(ard) = jadeed.aqsa_ard {
            let baada = Some(self.quyud.aqsa_ard.map_or(ard, |sabiq| sabiq.min(ard)));
            taghayyar |= farq_ashri(self.quyud.aqsa_ard, baada);
            self.quyud.aqsa_ard = baada;
        }
        if let Some(irtifa) = jadeed.aqsa_irtifa {
            let baada = Some(
                self.quyud
                    .aqsa_irtifa
                    .map_or(irtifa, |sabiq| sabiq.min(irtifa)),
            );
            taghayyar |= farq_ashri(self.quyud.aqsa_irtifa, baada);
            self.quyud.aqsa_irtifa = baada;
        }

        // The size and the rectangle describe an occurrence rather than a bound,
        // so they take the first measurement and keep it. Minimising a rectangle
        // across sightings would produce a box no occurrence ever had, and
        // averaging one would produce a box the game never drew.
        if self.quyud.hajm_khatt.is_none() && jadeed.hajm_khatt.is_some() {
            self.quyud.hajm_khatt = jadeed.hajm_khatt;
            taghayyar = true;
        }
        if self.quyud.mustatil.is_none() && jadeed.mustatil.is_some() {
            self.quyud.mustatil = jadeed.mustatil;
            taghayyar = true;
        }
        if jadeed.satr_wahid && !self.quyud.satr_wahid {
            self.quyud.satr_wahid = true;
            taghayyar = true;
        }
        if self.laqta.is_none() && hadath.laqta.is_some() {
            self.laqta.clone_from(&hadath.laqta);
        }
        if self.mashhad.is_none() && hadath.mashhad.is_some() {
            self.mashhad.clone_from(&hadath.mashhad);
        }
        if self.shasha.is_none() && hadath.shasha.is_some() {
            self.shasha.clone_from(&hadath.shasha);
        }
        taghayyar
    }

    /// Records that a sighting was counted but not folded.
    pub const fn sajjil_muayana(&mut self) {
        self.marrat = self.marrat.saturating_add(1);
        self.muayyana = true;
    }
}

/// Whether two optional measurements differ enough to be worth a durable line.
///
/// Written as a bit comparison of the encodings rather than as `!=` on the
/// floats: `float_cmp` is denied for the good reason that equality on measured
/// values is meaningless, and what this actually asks is "did the stored bytes
/// change", which `to_bits` answers exactly and `==` on `f32` does not (`-0.0`
/// and `0.0` compare equal, two `NaN`s compare unequal, and neither answer is
/// the one the writer wants).
const fn farq_ashri(sabiq: Option<f32>, baada: Option<f32>) -> bool {
    match (sabiq, baada) {
        (Some(a), Some(b)) => a.to_bits() != b.to_bits(),
        (None, None) => false,
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// What happened to one offered event
// ---------------------------------------------------------------------------

/// Why an offered event was not recorded.
///
/// Every variant is counted and every count reaches [`TaqreerJalsa`]. A session
/// that refused things silently would be a session whose coverage number cannot
/// be checked against anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababRafdHadath {
    /// The text was empty or whitespace only.
    ///
    /// Ordinary rather than exceptional: a label cleared between screens draws an
    /// empty string every frame until something fills it.
    Farigh,

    /// The text was longer than [`AQSA_TUL_NASS`].
    Tawil {
        /// How long it was, in bytes.
        tul: usize,
    },

    /// This frame had already offered [`AQSA_AHDATH_ITAR`] events.
    ///
    /// **The refusal that makes coverage a floor.** Unlike sampling, this one can
    /// discard a string never seen before, so it is counted apart from every
    /// other kind of loss and it is what [`TaqreerJalsa::taghtiya_mawthuqa`]
    /// keys on.
    SaqfItar,

    /// The session already holds [`KhiyaratJalsa::aqsa_mulahazat`] distinct
    /// strings and this one is not among them.
    Imtila,

    /// The key collided with a different string [`AQSA_TAHASSUS`] times.
    ///
    /// Not reachable in practice — it needs eight consecutive 128-bit collisions
    /// — and present because the alternative to bounding the probe is a loop that
    /// runs forever on a render-adjacent thread.
    Tasadum,
}

/// What a session did with one offered event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatijatIstiqbal {
    /// A string this session had not seen. Written to the file immediately.
    Jadeeda {
        /// Its key.
        miftah: MiftahMulahaza,
    },

    /// A string already known, folded in.
    Mutakarrira {
        /// Its key.
        miftah: MiftahMulahaza,
        /// Whether the fold tightened a constraint, which is what decides
        /// between a durable line now and a count carried to the next batch.
        taghayyar: bool,
    },

    /// A known string counted but not folded, because the frame budget was
    /// under pressure.
    ///
    /// Costs an occurrence's worth of width tightening. Costs no strings — see
    /// [`MizaniyatItar`] for why the sampler is never allowed to reach a key it
    /// has not seen.
    Muayyana {
        /// Its key.
        miftah: MiftahMulahaza,
    },

    /// Not recorded.
    Marfuda {
        /// Why.
        sabab: SababRafdHadath,
    },
}

impl NatijatIstiqbal {
    /// The key, where the event reached a record.
    #[must_use]
    pub const fn miftah(&self) -> Option<MiftahMulahaza> {
        match self {
            Self::Jadeeda { miftah }
            | Self::Mutakarrira { miftah, .. }
            | Self::Muayyana { miftah } => Some(*miftah),
            Self::Marfuda { .. } => None,
        }
    }

    /// Whether this event contributed a string the session did not have.
    #[must_use]
    pub const fn jadeeda(&self) -> bool {
        matches!(self, Self::Jadeeda { .. })
    }
}

// ---------------------------------------------------------------------------
// The bounded observation table
// ---------------------------------------------------------------------------

/// Every distinct string the session has seen, keyed for folding.
///
/// ## Why the table is bounded, and why the bound refuses rather than evicts
///
/// A game that composes text procedurally produces unbounded distinct strings: a
/// damage number per hit, a timestamp per log line, a coordinate per debug
/// readout. An unbounded table in Studio while somebody plays is an
/// out-of-memory in the middle of their session, caused by a tool that was meant
/// to be invisible.
///
/// Eviction is the obvious alternative and it is worse. An evicted string is
/// re-admitted the next time it is drawn, which writes it to the file again, and
/// again, and again — so eviction converts a memory bound into an I/O flood
/// while also discarding the folded widths that were the point. It would make a
/// session under pressure both slower *and* less accurate, which is the wrong
/// direction on both axes.
///
/// So the table refuses new keys at the bound, keeps folding into the ones it
/// has, counts every refusal, and raises [`SijillMulahazat::mumtali`]. This is
/// the one place in this crate where data is dropped rather than kept — and it
/// is bounded, counted, and reported as a qualifier on the coverage number
/// rather than absorbed into it. The remedy the interface offers is a shorter
/// session or a larger bound, both of which are actions a user can take.
#[derive(Debug)]
pub struct SijillMulahazat {
    /// The folded records.
    mulahazat: FxHashMap<MiftahMulahaza, MulahazaMutakarrira>,
    /// The bound on distinct strings.
    hadd: usize,
    /// How many new strings were refused because the table was full.
    marfuda_imtila: u64,
    /// How many times a key collided with a different string.
    tasadumat: u64,
    /// Keys whose counts have moved since the last durable flush.
    ///
    /// A set rather than a list, and the difference is not cosmetic: at sixty
    /// frames a second a two-second window holds thousands of repeat sightings
    /// of a few hundred strings, and a `Vec::contains` per sighting would be a
    /// linear scan on the hot path — millions of comparisons per frame to avoid
    /// writing a duplicate line.
    muallaqa: FxHashSet<MiftahMulahaza>,
}

impl SijillMulahazat {
    /// An empty table with a bound.
    #[must_use]
    pub fn jadeed(hadd: usize) -> Self {
        Self {
            mulahazat: FxHashMap::default(),
            hadd: hadd.max(1),
            marfuda_imtila: 0,
            tasadumat: 0,
            muallaqa: FxHashSet::default(),
        }
    }

    /// How many distinct strings are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.mulahazat.len()
    }

    /// Whether the bound has been reached and new strings are being refused.
    #[must_use]
    pub const fn mumtali(&self) -> bool {
        self.marfuda_imtila > 0
    }

    /// How many new strings the bound refused.
    #[must_use]
    pub const fn marfuda_imtila(&self) -> u64 {
        self.marfuda_imtila
    }

    /// How many key collisions were detected and re-probed.
    #[must_use]
    pub const fn tasadumat(&self) -> u64 {
        self.tasadumat
    }

    /// Every folded record, in no particular order.
    pub fn mulahazat(&self) -> impl Iterator<Item = &MulahazaMutakarrira> {
        self.mulahazat.values()
    }

    /// One record by key.
    #[must_use]
    pub fn mulahaza(&self, miftah: MiftahMulahaza) -> Option<&MulahazaMutakarrira> {
        self.mulahazat.get(&miftah)
    }

    /// Folds one event in.
    ///
    /// `yaftil` is the frame budget's answer — `true` to fold the event's
    /// measurements, `false` to count it and move on — and it is taken as a
    /// closure rather than as a `bool` for one reason that matters: it is invoked
    /// **only** on the branch where the key already exists. A new string is
    /// always admitted in full, whatever the pressure, and it must not even
    /// consume a place in the sampler's stride, because sampling that could lose
    /// a string would turn every coverage figure in the product into a guess.
    /// Taking a pre-computed `bool` would have consulted the sampler before
    /// knowing which branch this event lands on.
    pub fn qayyid<F>(&mut self, hadath: &HadathIltiqat, mut yaftil: F) -> NatijatIstiqbal
    where
        F: FnMut() -> bool,
    {
        let mut miftah = MiftahMulahaza::min_hadath(&hadath.nass, &hadath.masar_mukawwin);
        let mut tahassus = 0_u32;

        loop {
            // Read before the entry borrows the map: `Entry::Vacant` cannot ask
            // the map how full it is.
            let mumtali = self.mulahazat.len() >= self.hadd;

            match self.mulahazat.entry(miftah) {
                Entry::Occupied(mut mawjud) => {
                    if mawjud.get().yutabiq(&hadath.nass, &hadath.masar_mukawwin) {
                        if yaftil() {
                            let taghayyar = mawjud.get_mut().dammij(hadath);
                            return NatijatIstiqbal::Mutakarrira { miftah, taghayyar };
                        }
                        mawjud.get_mut().sajjil_muayana();
                        return NatijatIstiqbal::Muayyana { miftah };
                    }
                },
                Entry::Vacant(farigh) => {
                    if mumtali {
                        self.marfuda_imtila = self.marfuda_imtila.saturating_add(1);
                        return NatijatIstiqbal::Marfuda {
                            sabab: SababRafdHadath::Imtila,
                        };
                    }
                    let _ = farigh.insert(MulahazaMutakarrira::min_hadath(miftah, hadath));
                    return NatijatIstiqbal::Jadeeda { miftah };
                },
            }

            // A verified collision: the key is taken by a different string. Move
            // to the next probe rather than folding, which would merge two
            // strings and lose one of them.
            self.tasadumat = self.tasadumat.saturating_add(1);
            tahassus = tahassus.saturating_add(1);
            if tahassus >= AQSA_TAHASSUS {
                return NatijatIstiqbal::Marfuda {
                    sabab: SababRafdHadath::Tasadum,
                };
            }
            miftah.thani = miftah.thani.wrapping_add(1);
        }
    }

    /// Marks a key as having a count that has not reached the device yet.
    ///
    /// Deduplicated, so a label drawn every frame for two seconds produces one
    /// pending entry rather than a hundred and twenty.
    fn allim(&mut self, miftah: MiftahMulahaza) {
        let _ = self.muallaqa.insert(miftah);
    }

    /// Takes the pending updates, leaving the set empty.
    fn ista_muallaqa(&mut self) -> FxHashSet<MiftahMulahaza> {
        std::mem::take(&mut self.muallaqa)
    }

    /// Inserts a record read back from a session file.
    ///
    /// Used by [`iqra_jalsa`] and by nothing else. Returns whether it was
    /// admitted, so a file holding more distinct strings than the reader's bound
    /// is reported rather than silently shortened.
    fn ahill(&mut self, mulahaza: MulahazaMutakarrira) -> bool {
        let Some(miftah) = MiftahMulahaza::min_nass(&mulahaza.miftah) else {
            return false;
        };
        if self.mulahazat.len() >= self.hadd && !self.mulahazat.contains_key(&miftah) {
            self.marfuda_imtila = self.marfuda_imtila.saturating_add(1);
            return false;
        }
        let _ = self.mulahazat.insert(miftah, mulahaza);
        true
    }
}

// ---------------------------------------------------------------------------
// The frame budget
// ---------------------------------------------------------------------------

/// What capture is allowed to cost, and what it does when it costs more.
///
/// ## The policy, stated plainly
///
/// Capture gets [`MIZANIYAT_ITAR_MIKRO`] microseconds per frame. Time is measured
/// per event and accumulated per frame. When a frame ends over budget the
/// **sampling stride doubles**, up to [`AQSA_KHUTWA`]; when a frame ends at under
/// half the budget it halves, down to one. Nothing is ever deferred to a later
/// frame and no frame is ever made late to finish capture work — the game's
/// frame is not capture's to spend.
///
/// ## Why sampling is degraded rather than frames dropped
///
/// The two ways to stay inside a budget are to do less work per frame or to skip
/// whole frames. Skipping whole frames is worse for the same total saving: the
/// strings that appear on a skipped frame are systematically the ones that
/// appear on *few* frames — a one-shot notification, a hit marker, a line of
/// dialogue that advances — while the strings that survive are the ones drawn
/// constantly, which are the ones already recorded. Frame-dropping therefore
/// biases the session against exactly the strings capture is most needed for.
/// Sampling within the frame costs occurrence counts, which are a weighting, not
/// a set.
///
/// ## The sampler is never allowed to lose a string
///
/// A repeat sighting contributes a count and possibly a tighter width. A first
/// sighting contributes the string itself. The two are not equally expendable,
/// and telling them apart costs only the hash — the expensive part of an event
/// is serialising it and writing it, not hashing sixty bytes. So every event is
/// hashed and looked up, and the sampler applies only to keys that are already
/// present. [`SijillMulahazat::qayyid`] enforces that, not this type.
///
/// [`AQSA_AHDATH_ITAR`] is the exception and it is deliberately a different
/// mechanism with a different counter: past that many events in one frame, even
/// hashing is too much, and events are refused outright. Those refusals *can*
/// include new strings, so they are reported separately and they are what makes
/// a session's coverage a floor rather than a measurement.
#[derive(Debug)]
pub struct MizaniyatItar {
    /// Microseconds per frame capture may spend.
    mizaniya_mikro: u32,
    /// Events accepted in one frame before refusal.
    saqf_ahdath: u32,
    /// The current sampling stride; one means every event is folded.
    khutwa: u32,
    /// How many more events to skip before folding the next one.
    mutabaqqi: u32,
    /// Microseconds spent so far in this frame.
    munfaq_mikro: u64,
    /// Events offered so far in this frame.
    ahdath_itar: u32,
    /// The running totals.
    ihsaat: IhsaatMizaniya,
}

impl MizaniyatItar {
    /// A budget.
    ///
    /// A zero or absurd microsecond budget is clamped to
    /// [`MIZANIYAT_ITAR_MIKRO`] rather than honoured: a configuration mistake
    /// that silently disabled capture would look exactly like a game with no
    /// text, and that is a bug report nobody can diagnose.
    #[must_use]
    pub fn jadeeda(mizaniya_mikro: u32, saqf_ahdath: u32) -> Self {
        Self {
            mizaniya_mikro: if mizaniya_mikro == 0 {
                MIZANIYAT_ITAR_MIKRO
            } else {
                mizaniya_mikro
            },
            saqf_ahdath: saqf_ahdath.max(1),
            khutwa: 1,
            mutabaqqi: 0,
            munfaq_mikro: 0,
            ahdath_itar: 0,
            ihsaat: IhsaatMizaniya::default(),
        }
    }

    /// Whether this frame may accept another event at all.
    ///
    /// The hard ceiling, checked before the event is even hashed.
    pub const fn yaqbal(&mut self) -> bool {
        if self.ahdath_itar >= self.saqf_ahdath {
            self.ihsaat.mahdhufa_kulliya = self.ihsaat.mahdhufa_kulliya.saturating_add(1);
            return false;
        }
        self.ahdath_itar = self.ahdath_itar.saturating_add(1);
        true
    }

    /// Whether this event's measurements should be folded, or only counted.
    ///
    /// Consulted only for keys the session already holds; see this type's
    /// header.
    pub const fn yaftil(&mut self) -> bool {
        if self.khutwa <= 1 {
            return true;
        }
        if self.mutabaqqi == 0 {
            self.mutabaqqi = self.khutwa.saturating_sub(1);
            return true;
        }
        self.mutabaqqi = self.mutabaqqi.saturating_sub(1);
        self.ihsaat.mahdhufa_mutakarrira = self.ihsaat.mahdhufa_mutakarrira.saturating_add(1);
        false
    }

    /// Records what one event cost.
    pub const fn sajjil(&mut self, munfaq_mikro: u64) {
        self.munfaq_mikro = self.munfaq_mikro.saturating_add(munfaq_mikro);
    }

    /// Closes the frame: updates the totals and adjusts the stride.
    ///
    /// Multiplicative increase and multiplicative decrease, rather than the
    /// additive decrease a congestion controller would use. The asymmetry a
    /// network wants is about fairness between competing senders, and there is
    /// no competition here — there is one producer and a budget it either fits
    /// in or does not. A stride that took sixty-four frames to walk back down
    /// from sixty-four would keep sampling for a second after the pressure had
    /// gone, which costs measurements for nothing.
    pub fn itar_jadeed(&mut self) {
        let mizaniya = u64::from(self.mizaniya_mikro);
        self.ihsaat.itarat = self.ihsaat.itarat.saturating_add(1);
        self.ihsaat.majmu_mikro = self.ihsaat.majmu_mikro.saturating_add(self.munfaq_mikro);
        self.ihsaat.aqsa_mikro = self.ihsaat.aqsa_mikro.max(self.munfaq_mikro);

        if self.munfaq_mikro > mizaniya {
            self.ihsaat.itarat_mutajawiza = self.ihsaat.itarat_mutajawiza.saturating_add(1);
            self.khutwa = self.khutwa.saturating_mul(2).min(AQSA_KHUTWA);
        } else if self.munfaq_mikro.saturating_mul(2) < mizaniya {
            // checked_div rather than `/`: integer division is denied workspace
            // wide, and the None arm is unreachable for a literal two but is
            // still given the only answer that keeps the stride legal.
            self.khutwa = self.khutwa.checked_div(2).unwrap_or(1).max(1);
        }
        self.ihsaat.aqsa_khutwa = self.ihsaat.aqsa_khutwa.max(self.khutwa);

        self.munfaq_mikro = 0;
        self.ahdath_itar = 0;
        self.mutabaqqi = 0;
    }

    /// The current stride.
    #[must_use]
    pub const fn khutwa(&self) -> u32 {
        self.khutwa
    }

    /// The totals so far.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatMizaniya {
        &self.ihsaat
    }
}

/// What the frame budget did over a session.
///
/// Serialised into the session file's footer, because a measurement taken under
/// sampling is a different claim from one taken at full rate and a reader with
/// only the strings cannot tell which it has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IhsaatMizaniya {
    /// Frames the session saw.
    pub itarat: u64,
    /// Frames that ended over budget.
    pub itarat_mutajawiza: u64,
    /// Total microseconds capture spent.
    pub majmu_mikro: u64,
    /// The worst single frame, in microseconds.
    pub aqsa_mikro: u64,
    /// The largest sampling stride reached.
    pub aqsa_khutwa: u32,
    /// Repeat sightings the sampler counted but did not fold.
    ///
    /// Costs occurrence weighting and width tightening. Costs no strings.
    pub mahdhufa_mutakarrira: u64,
    /// Events refused outright by the per-frame ceiling.
    ///
    /// **May include strings never seen before**, which is why it is counted
    /// apart from everything else here.
    pub mahdhufa_kulliya: u64,
}

impl IhsaatMizaniya {
    /// The share of frames that went over budget, zero to one.
    #[must_use]
    pub fn nisbat_tajawuz(&self) -> f32 {
        nisba(self.itarat_mutajawiza, self.itarat)
    }

    /// The average microseconds per frame capture cost.
    #[must_use]
    pub fn mutawassit_mikro(&self) -> f32 {
        nisba(self.majmu_mikro, self.itarat)
    }

    /// Whether the session ran without ever degrading.
    #[must_use]
    pub const fn bila_tadhur(&self) -> bool {
        self.mahdhufa_mutakarrira == 0 && self.mahdhufa_kulliya == 0
    }

    /// The sentence the capture panel shows while a session runs.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        if self.bila_tadhur() {
            return format!(
                "Capture cost {:.0}µs per frame on average, never exceeding its budget.",
                self.mutawassit_mikro()
            );
        }
        format!(
            "Capture cost {:.0}µs per frame on average and exceeded its budget on {:.1}% of \
             frames. {} repeat sighting(s) were counted but not measured, and {} event(s) were \
             refused entirely — those may have included strings never recorded.",
            self.mutawassit_mikro(),
            self.nisbat_tajawuz() * 100.0,
            self.mahdhufa_mutakarrira,
            self.mahdhufa_kulliya
        )
    }
}

/// A ratio that answers zero for an empty denominator.
///
/// Zero rather than one, unlike coverage: "no frames ran, so no frames went over
/// budget" is the honest reading, whereas a coverage ratio with an empty
/// denominator means "nothing to cover", which is complete.
///
/// Divides in `f32` rather than promoting to `f64` and narrowing back, because
/// the narrowing is a second lossy conversion bought for a precision no display
/// here uses — these are ratios rendered to one decimal place and averages
/// rendered to the nearest microsecond.
fn nisba(juz: u64, kull: u64) -> f32 {
    if kull == 0 {
        return 0.0;
    }
    ila_kasr(juz) / ila_kasr(kull)
}

// ---------------------------------------------------------------------------
// The session file
// ---------------------------------------------------------------------------

/// A session file's first line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TarwisatJalsa {
    /// [`ISDAR_JALSA`] at the time of writing.
    pub isdar: u32,
    /// What the session is called, and what a capture-only string's location
    /// will name once [`crate::dammij`] runs.
    pub jalsa: String,
    /// The game, as the library knows it.
    pub luba: Option<String>,
    /// Studio's wall clock when the session opened, RFC 3339.
    pub bada: String,
    /// How many strings static extraction had found, when the caller knew.
    ///
    /// Written into the file rather than kept in Studio's memory so that a
    /// session read back weeks later can still state its own coverage estimate
    /// instead of losing the denominator.
    pub adad_sakin: Option<usize>,
    /// The bound this session ran with, so a truncated capture is explicable.
    pub aqsa_mulahazat: usize,
    /// The per-frame budget this session ran with, in microseconds.
    pub mizaniyat_itar_mikro: u32,
}

/// A count-and-constraint update for a string already in the file.
///
/// Written at flush time, once per key whose numbers moved during the window,
/// rather than once per sighting. A label drawn every frame for two seconds
/// produces one of these, not a hundred and twenty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TahdithMulahaza {
    /// Which record.
    pub miftah: String,
    /// The sighting count as of this line.
    pub marrat: u64,
    /// The adapter's clock at the last folded sighting.
    pub akhir_waqt_mil: u64,
    /// The folded constraints as of this line.
    ///
    /// The whole set rather than only what changed, so a reader can rebuild a
    /// record's final state from the last update it sees without replaying every
    /// earlier one — which matters because the earlier ones are exactly what a
    /// truncated file is missing.
    pub quyud: QuyudNass,
    /// Whether any sighting of this string has been sampled out.
    pub muayyana: bool,
}

/// A session file's last line.
///
/// Its **absence** is the signal: a file with no footer is a session that was
/// killed, and [`JalsaMuhammala::iktamalat`] reports that rather than presenting
/// a partial capture as a complete one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KhitamJalsa {
    /// Studio's wall clock when the session closed, RFC 3339.
    pub intaha: String,
    /// Everything the session counted.
    pub ihsaat: IhsaatJalsa,
}

/// Everything one session counted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IhsaatJalsa {
    /// Events offered to the session, before any filtering.
    pub ahdath: u64,
    /// Events that reached a record.
    pub musajjala: u64,
    /// Distinct strings held.
    pub nusus: u64,
    /// Events refused for having no text.
    pub marfuda_farigh: u64,
    /// Events refused for exceeding [`AQSA_TUL_NASS`].
    pub marfuda_tawil: u64,
    /// New strings refused because the table was full.
    pub marfuda_imtila: u64,
    /// Events refused after exhausting the collision probe.
    pub marfuda_tasadum: u64,
    /// Missing adapter sequence numbers.
    ///
    /// Direct evidence that messages were lost between the game and Studio,
    /// independent of what the transport reported about itself.
    pub fajawat: u64,
    /// Messages the transport itself reported dropping.
    ///
    /// Folded in through [`JalsatIltiqat::sajjil_faqd`]. `barid`'s sender is
    /// bounded and drops rather than blocking a game's render thread, and the
    /// count it keeps belongs in the session's honesty accounting rather than in
    /// a log nobody correlates with the result.
    pub faqd_naql: u64,
    /// Records that could not be serialised and were not written.
    pub asturr_talifa: u64,
    /// What the frame budget did.
    pub mizaniya: IhsaatMizaniya,
}

impl IhsaatJalsa {
    /// Every way this session lost an event, added up.
    #[must_use]
    pub const fn majmu_faqd(&self) -> u64 {
        self.marfuda_imtila
            .saturating_add(self.marfuda_tasadum)
            .saturating_add(self.fajawat)
            .saturating_add(self.faqd_naql)
            .saturating_add(self.asturr_talifa)
            .saturating_add(self.mizaniya.mahdhufa_kulliya)
    }

    /// Whether every string this session was offered reached a record.
    ///
    /// Empty and over-long draws do not count against it: neither carries a
    /// translatable string, so refusing them loses nothing. Sampled repeats do
    /// not count against it either — they cost occurrence counts, not strings.
    /// Everything else does.
    #[must_use]
    pub const fn kamila(&self) -> bool {
        self.majmu_faqd() == 0
    }

    /// Counts one refusal against the counter that describes it.
    ///
    /// One exhaustive match, so that adding a reason to [`SababRafdHadath`]
    /// without giving it a counter fails to compile rather than quietly
    /// disappearing from every report. That is the whole point of the enum being
    /// closed.
    const fn sajjil_rafd(&mut self, sabab: SababRafdHadath) {
        match sabab {
            SababRafdHadath::Farigh => {
                self.marfuda_farigh = self.marfuda_farigh.saturating_add(1);
            },
            SababRafdHadath::Tawil { .. } => {
                self.marfuda_tawil = self.marfuda_tawil.saturating_add(1);
            },
            SababRafdHadath::Imtila => {
                self.marfuda_imtila = self.marfuda_imtila.saturating_add(1);
            },
            SababRafdHadath::Tasadum => {
                self.marfuda_tasadum = self.marfuda_tasadum.saturating_add(1);
            },
            // Counted inside the budget, which is the only thing that knows a
            // frame's ceiling was reached, and which reports it as
            // [`IhsaatMizaniya::mahdhufa_kulliya`] — apart from every other loss
            // because it is the only one that can cost an unseen string.
            SababRafdHadath::SaqfItar => {},
        }
    }
}

/// One line of a session file.
///
/// JSON Lines rather than one JSON document, because a document has to be closed
/// to be valid and a session that was killed never closes anything. Every line
/// stands alone, a truncated final line damages only itself, and appending costs
/// no rewrite of what is already there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SatrJalsa {
    /// The header.
    Tarwisa(TarwisatJalsa),
    /// A string seen for the first time, in full.
    Nass(MulahazaMutakarrira),
    /// Counts and constraints for a string already written.
    Tahdith(TahdithMulahaza),
    /// The footer.
    Khitam(KhitamJalsa),
}

/// The append-only writer behind a session.
///
/// ## Durability, in two levels, because a crash and a power cut are not the
/// same failure
///
/// The rule this crate's sibling `muhawwil-nusus::hifz` enforces is that a
/// record reaches the device before the thing it describes is considered done.
/// That rule was written for a patcher that touches a handful of files, and
/// applying it literally here would mean an `fsync` per captured string. A
/// capture session running at sixty frames a second cannot `fsync` per event —
/// on a spinning disk that is single-digit milliseconds each, which would not
/// merely miss the frame budget, it would miss the frame.
///
/// So durability is split, because the two failures it protects against are not
/// equally likely and do not cost the same:
///
/// | level | when | survives | costs |
/// | --- | --- | --- | --- |
/// | the write | immediately for a header, a new string, or a footer | the process being killed, the game crashing, a force quit | nothing beyond the line being formatted at that instant |
/// | the sync | when the batch reaches [`HADD_DUFA_BAYT`] or [`MUDDAT_DUFA_MIL`] elapses | the machine losing power | the last batch — at most two seconds of *count updates*, and no strings, because every new string was already pushed to the operating system |
///
/// The common failure by an enormous margin is the first one: a game crashed, or
/// the user closed everything, or Studio was killed. Against that, a session
/// loses nothing at all. Against a power cut, it loses occurrence counts and
/// width refinements from the last two seconds of play. That is the honest
/// statement of the trade and it is not zero.
#[derive(Debug)]
pub struct KatibJalsa {
    /// Where the file is.
    masar: PathBuf,
    /// The open handle, in append mode.
    malaf: File,
    /// Lines formatted but not yet handed to the operating system.
    dufa: Vec<u8>,
    /// Bytes handed over but not yet synced.
    ghayr_mutazamin: usize,
    /// The byte threshold for a sync.
    hadd_dufa: usize,
    /// The time threshold for a sync, in milliseconds.
    muddat_dufa_mil: u64,
    /// When the last sync happened.
    akhir_tazamun: Instant,
    /// How many lines were written.
    sutur: u64,
    /// How many records could not be serialised.
    talifa: u64,
}

impl KatibJalsa {
    /// Opens a session file for appending, creating it and its directory.
    ///
    /// Append mode rather than truncate: a session resumed against an existing
    /// file adds to it, and a run that opened with truncation would destroy the
    /// previous session's measurements the moment somebody reused a name.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the directory
    /// cannot be created and
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the file
    /// cannot be opened.
    pub fn iftah(masar: &Path, hadd_dufa: usize, muddat_dufa_mil: u64) -> Natija<Self> {
        if let Some(mujallad) = masar.parent() {
            masarat::insha_mujallad(mujallad)?;
        }
        let malaf = OpenOptions::new()
            .create(true)
            .append(true)
            .open(masar)
            .map_err(|sabab| khata_kitaba(masar, &sabab))?;

        Ok(Self {
            masar: masar.to_path_buf(),
            malaf,
            dufa: Vec::with_capacity(hadd_dufa.max(4096)),
            ghayr_mutazamin: 0,
            hadd_dufa: hadd_dufa.max(4096),
            muddat_dufa_mil,
            akhir_tazamun: Instant::now(),
            sutur: 0,
            talifa: 0,
        })
    }

    /// The file being written.
    #[must_use]
    pub fn masar(&self) -> &Path {
        &self.masar
    }

    /// How many lines have been written.
    #[must_use]
    pub const fn sutur(&self) -> u64 {
        self.sutur
    }

    /// How many records could not be serialised and were dropped.
    #[must_use]
    pub const fn talifa(&self) -> u64 {
        self.talifa
    }

    /// Appends one line.
    ///
    /// `fawri` pushes the line to the operating system before returning, which
    /// is what a header, a newly seen string and a footer get. A count update
    /// does not: it is superseded by the next one, so losing it to a killed
    /// process costs a number that the following window would have corrected
    /// anyway.
    ///
    /// A record that cannot be serialised is counted and skipped rather than
    /// failing the session. Ending a two-hour capture because one string
    /// produced a value `serde_json` refused would throw away everything that
    /// worked in order to report the one thing that did not.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the write or
    /// the sync fails — a full disk, a removed drive, a revoked permission.
    pub fn aktub(&mut self, satr: &SatrJalsa, fawri: bool) -> Natija<()> {
        if let Ok(bayt) = serde_json::to_vec(satr) {
            self.dufa.extend_from_slice(&bayt);
            self.dufa.push(b'\n');
            self.sutur = self.sutur.saturating_add(1);
        } else {
            self.talifa = self.talifa.saturating_add(1);
            return Ok(());
        }

        if fawri || self.dufa.len() >= self.hadd_dufa {
            self.dafq()?;
        }
        if self.ghayr_mutazamin >= self.hadd_dufa {
            self.zamin()?;
        }
        Ok(())
    }

    /// Whether the sync threshold has been reached.
    ///
    /// Public because the *time* half of the cadence is driven by the session
    /// rather than by this type: only the session knows there are pending count
    /// updates that have to be written before the sync, and a sync that happened
    /// first would leave them a window behind. The byte half is enforced here,
    /// so a caller that never asks still cannot accumulate more than
    /// [`HADD_DUFA_BAYT`] of unsynced data.
    #[must_use]
    pub fn hana_tazamun(&self) -> bool {
        if self.ghayr_mutazamin == 0 {
            return false;
        }
        if self.ghayr_mutazamin >= self.hadd_dufa {
            return true;
        }
        let madat = self.akhir_tazamun.elapsed();
        u64::try_from(madat.as_millis()).unwrap_or(u64::MAX) >= self.muddat_dufa_mil
    }

    /// Hands the pending bytes to the operating system.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the write
    /// fails.
    fn dafq(&mut self) -> Natija<()> {
        if self.dufa.is_empty() {
            return Ok(());
        }
        if let Err(sabab) = self.malaf.write_all(&self.dufa) {
            // The buffer is deliberately left intact. A write that failed for a
            // transient reason — a disk that filled and was then cleared — is
            // retried by the next flush with nothing missing from the middle of
            // the file, and a file with a hole in it is a file no reader can
            // trust.
            return Err(khata_kitaba(&self.masar, &sabab));
        }
        self.ghayr_mutazamin = self.ghayr_mutazamin.saturating_add(self.dufa.len());
        self.dufa.clear();
        Ok(())
    }

    /// Pushes everything to the device.
    ///
    /// `sync_data` rather than `sync_all`: the file's length and contents must be
    /// durable, its access time need not be, and the metadata flush is the
    /// expensive half on several filesystems. The final close uses `sync_all`,
    /// where the cost is paid once.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the write or
    /// the sync fails.
    pub fn zamin(&mut self) -> Natija<()> {
        self.dafq()?;
        if self.ghayr_mutazamin > 0 {
            if let Err(sabab) = self.malaf.sync_data() {
                return Err(khata_kitaba(&self.masar, &sabab));
            }
            self.ghayr_mutazamin = 0;
        }
        self.akhir_tazamun = Instant::now();
        Ok(())
    }

    /// Writes the footer and closes the file durably.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the footer, the
    /// final write, or the final sync fails.
    pub fn aghliq(mut self, khitam: KhitamJalsa) -> Natija<()> {
        self.aktub(&SatrJalsa::Khitam(khitam), true)?;
        self.dafq()?;
        if let Err(sabab) = self.malaf.sync_all() {
            return Err(khata_kitaba(&self.masar, &sabab));
        }
        Ok(())
    }
}

/// A write failure against a capture session's own file.
///
/// Carries this crate's code rather than `taarib-usus`'s generic path error,
/// and the reason is what a permanent code is *for*: a user pasting
/// `TAARIB-E-5007` into a bug report has said "the capture session's file broke",
/// and one pasting the path error has said "some file broke" — which is true of
/// every subsystem in the product and locates nothing.
///
/// The underlying `io::Error` is carried through, so the sentence and the
/// remedy still come from what the filesystem actually said. What changes is
/// only which subsystem the code names, which is the one thing a generic error
/// cannot express.
fn khata_kitaba(masar: &Path, sabab: &std::io::Error) -> Khata {
    Khata::from(KhataIstikhraj::KhataIltiqat {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

/// A read failure against a capture session's own file.
fn khata_qira(masar: &Path, sabab: &std::io::Error) -> Khata {
    Khata::from(KhataIstikhraj::KhataIltiqat {
        masar: masar.to_path_buf(),
        sabab: sabab.to_string(),
    })
}

// ---------------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------------

/// How a capture session is set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KhiyaratJalsa {
    /// The session file.
    pub masar: PathBuf,
    /// What this session is called.
    ///
    /// Becomes the `hawiya` of every capture-only string's location, so it is
    /// what a translator sees under a string that has no file to come from. A
    /// date and a game name read better there than a UUID.
    pub jalsa: String,
    /// The game, as the library knows it.
    pub luba: Option<String>,
    /// How many strings static extraction produced, for the coverage estimate.
    pub adad_sakin: Option<usize>,
    /// The bound on distinct strings.
    pub aqsa_mulahazat: usize,
    /// Microseconds per frame capture may spend.
    pub mizaniyat_itar_mikro: u32,
    /// The hard per-frame event ceiling.
    pub saqf_ahdath_itar: u32,
    /// The batch size for syncing, in bytes.
    pub hadd_dufa_bayt: usize,
    /// The batch interval for syncing, in milliseconds.
    pub muddat_dufa_mil: u64,
}

impl KhiyaratJalsa {
    /// Options with this build's defaults.
    #[must_use]
    pub fn jadeeda(masar: impl Into<PathBuf>, jalsa: impl Into<String>) -> Self {
        Self {
            masar: masar.into(),
            jalsa: jalsa.into(),
            luba: None,
            adad_sakin: None,
            aqsa_mulahazat: AQSA_MULAHAZAT,
            mizaniyat_itar_mikro: MIZANIYAT_ITAR_MIKRO,
            saqf_ahdath_itar: AQSA_AHDATH_ITAR,
            hadd_dufa_bayt: HADD_DUFA_BAYT,
            muddat_dufa_mil: MUDDAT_DUFA_MIL,
        }
    }
}

/// The file a session by this name writes to, under a directory.
///
/// Kept here rather than left to callers so that Studio, the merge step and the
/// diagnostics bundle all name the same file. The name is used verbatim after
/// path separators are replaced, because a session called `2026-08-28 Chrono
/// Trigger` must not become a directory tree.
#[must_use]
pub fn masar_jalsa(mujallad: &Path, jalsa: &str) -> PathBuf {
    let aamin: String = jalsa
        .chars()
        .map(|harf| {
            if harf.is_control() || "/\\:*?\"<>|".contains(harf) {
                '_'
            } else {
                harf
            }
        })
        .collect();
    mujallad.join(format!("{aamin}.jsonl"))
}

/// A running capture session.
///
/// Fed by whatever is listening on `barid`; it opens nothing itself. The order
/// of operations over one frame is: the listener drains what arrived, calls
/// [`JalsatIltiqat::istaqbil`] per message, and calls
/// [`JalsatIltiqat::itar_jadeed`] once. A caller with no frame signal — a merge
/// tool replaying a file, a test — simply never calls the second one, and the
/// budget then never degrades because no frame ever closes over budget.
#[derive(Debug)]
pub struct JalsatIltiqat {
    /// The session's name.
    jalsa: String,
    /// The game.
    luba: Option<String>,
    /// The static denominator for the coverage estimate.
    adad_sakin: Option<usize>,
    /// Studio's wall clock at the start, RFC 3339.
    bada: String,
    /// The monotonic start, for the event rate.
    bada_lahza: Instant,
    /// Distinct strings.
    sijill: SijillMulahazat,
    /// The frame budget.
    mizaniya: MizaniyatItar,
    /// The file.
    katib: KatibJalsa,
    /// Everything counted.
    ihsaat: IhsaatJalsa,
    /// The adapter's last sequence number, for gap detection.
    akhir_tasalsul: Option<u64>,
    /// Strings first seen in this run, as opposed to loaded from a resumed file.
    jadeeda: u64,
}

impl JalsatIltiqat {
    /// Opens a session and writes its header.
    ///
    /// The header goes to the operating system before this returns. A session
    /// file that exists and cannot say what version, what game or what bound it
    /// was written under is a file nobody can safely merge, and the cheapest
    /// moment to guarantee it is here.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] or
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the directory
    /// or the file cannot be created, or the header cannot be written.
    pub fn ibda(khiyarat: &KhiyaratJalsa) -> Natija<Self> {
        let katib = KatibJalsa::iftah(
            &khiyarat.masar,
            khiyarat.hadd_dufa_bayt,
            khiyarat.muddat_dufa_mil,
        )?;
        let mut jalsa = Self {
            jalsa: khiyarat.jalsa.clone(),
            luba: khiyarat.luba.clone(),
            adad_sakin: khiyarat.adad_sakin,
            bada: jiff::Timestamp::now().to_string(),
            bada_lahza: Instant::now(),
            sijill: SijillMulahazat::jadeed(khiyarat.aqsa_mulahazat),
            mizaniya: MizaniyatItar::jadeeda(
                khiyarat.mizaniyat_itar_mikro,
                khiyarat.saqf_ahdath_itar,
            ),
            katib,
            ihsaat: IhsaatJalsa::default(),
            akhir_tasalsul: None,
            jadeeda: 0,
        };
        let tarwisa = TarwisatJalsa {
            isdar: ISDAR_JALSA,
            jalsa: jalsa.jalsa.clone(),
            luba: jalsa.luba.clone(),
            bada: jalsa.bada.clone(),
            adad_sakin: jalsa.adad_sakin,
            aqsa_mulahazat: khiyarat.aqsa_mulahazat,
            mizaniyat_itar_mikro: khiyarat.mizaniyat_itar_mikro,
        };
        jalsa.katib.aktub(&SatrJalsa::Tarwisa(tarwisa), true)?;
        Ok(jalsa)
    }

    /// Reopens a session against the file it already wrote.
    ///
    /// The reason the file is opened for appending rather than truncated. A user
    /// plays for forty minutes, stops, and comes back the next evening; the
    /// strings from the first evening are still the strings, their measurements
    /// are still the measurements, and starting over would ask them to walk the
    /// same screens again to recover what is already on disk.
    ///
    /// The resumed run writes a fresh header, which is correct rather than
    /// redundant: it records the new start time and the bound this run is using,
    /// and a reader folds every header it meets, taking the first one's identity
    /// and the last one's settings.
    ///
    /// # Errors
    ///
    /// As [`JalsatIltiqat::ibda`].
    pub fn istanif(khiyarat: &KhiyaratJalsa, muhammala: JalsaMuhammala) -> Natija<Self> {
        let mut jalsa = Self::ibda(khiyarat)?;
        jalsa.ihsaat = muhammala.khitam.as_ref().map_or_else(
            || {
                // No footer: the previous run was killed. Its counters are gone
                // and the strings are not, so the totals restart at what can
                // actually be verified — the records that survived — rather
                // than at a number reconstructed from a file that stops
                // mid-sentence.
                IhsaatJalsa {
                    nusus: u64::try_from(muhammala.sijill.adad()).unwrap_or(u64::MAX),
                    ..IhsaatJalsa::default()
                }
            },
            |khitam| khitam.ihsaat,
        );
        jalsa.sijill = muhammala.sijill;
        // The bound is raised to whatever the file already held. A resumed
        // session must not start by refusing strings it is holding: that would
        // report a saturated table for a run that has admitted nothing.
        let hadd = khiyarat.aqsa_mulahazat.max(jalsa.sijill.adad());
        jalsa.sijill.hadd = hadd;
        Ok(jalsa)
    }

    /// Takes one event.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when a newly seen
    /// string cannot be written. A refused or sampled event never writes and
    /// therefore never fails.
    pub fn istaqbil(&mut self, hadath: &HadathIltiqat) -> Natija<NatijatIstiqbal> {
        let bidaya = Instant::now();
        self.ihsaat.ahdath = self.ihsaat.ahdath.saturating_add(1);
        self.tafaqqad_tasalsul(hadath.tasalsul);

        if let Some(sabab) = hadath.rafd() {
            self.ihsaat.sajjil_rafd(sabab);
            return Ok(NatijatIstiqbal::Marfuda { sabab });
        }
        if !self.mizaniya.yaqbal() {
            return Ok(NatijatIstiqbal::Marfuda {
                sabab: SababRafdHadath::SaqfItar,
            });
        }

        // Split borrow: the sampler and the table are separate fields, and the
        // closure must reach the sampler only on the already-known branch.
        let mizaniya = &mut self.mizaniya;
        let natija = self.sijill.qayyid(hadath, || mizaniya.yaftil());

        match natija {
            NatijatIstiqbal::Jadeeda { miftah } => {
                self.ihsaat.musajjala = self.ihsaat.musajjala.saturating_add(1);
                self.jadeeda = self.jadeeda.saturating_add(1);
                if let Some(mulahaza) = self.sijill.mulahaza(miftah) {
                    let satr = SatrJalsa::Nass(mulahaza.clone());
                    self.katib.aktub(&satr, true)?;
                }
            },
            NatijatIstiqbal::Mutakarrira { miftah, taghayyar } => {
                self.ihsaat.musajjala = self.ihsaat.musajjala.saturating_add(1);
                if taghayyar {
                    // A tightened width is a measurement, not a counter. The
                    // narrow tooltip that produced it may never open again, so
                    // it is pushed out now rather than waiting for the batch; a
                    // count that is one window stale corrects itself, a lost
                    // measurement does not.
                    if let Some(satr) = self.tahdith(miftah) {
                        self.katib.aktub(&satr, true)?;
                    }
                } else {
                    self.sijill.allim(miftah);
                }
            },
            NatijatIstiqbal::Muayyana { miftah } => {
                self.ihsaat.musajjala = self.ihsaat.musajjala.saturating_add(1);
                self.sijill.allim(miftah);
            },
            NatijatIstiqbal::Marfuda { sabab } => self.ihsaat.sajjil_rafd(sabab),
        }

        self.mizaniya
            .sajjil(u64::try_from(bidaya.elapsed().as_micros()).unwrap_or(u64::MAX));
        Ok(natija)
    }

    /// Closes a frame: flushes what is due and lets the budget adjust.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the pending
    /// updates or the sync fail.
    pub fn itar_jadeed(&mut self) -> Natija<()> {
        if self.katib.hana_tazamun() {
            self.adfiq()?;
        }
        self.mizaniya.itar_jadeed();
        Ok(())
    }

    /// Writes the pending count updates and syncs.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when a write or the
    /// sync fails.
    pub fn adfiq(&mut self) -> Natija<()> {
        for miftah in self.sijill.ista_muallaqa() {
            if let Some(satr) = self.tahdith(miftah) {
                self.katib.aktub(&satr, false)?;
            }
        }
        self.katib.zamin()
    }

    /// Folds in what the transport says it dropped.
    ///
    /// `barid`'s sender is bounded and drops rather than blocking a game's render
    /// thread — which is the right trade, because freezing somebody's game to
    /// deliver a diagnostic is worse than losing the diagnostic. What is not
    /// acceptable is for the count to live only in the transport's own log while
    /// this session reports a coverage figure computed as though nothing was
    /// lost. Called by the listener with whatever `barid` reports.
    pub const fn sajjil_faqd(&mut self, adad: u64) {
        self.ihsaat.faqd_naql = self.ihsaat.faqd_naql.saturating_add(adad);
    }

    /// Live progress, cheap enough to poll every frame.
    #[must_use]
    pub fn taqaddum(&self) -> TaqaddumIltiqat {
        let mudda = self.bada_lahza.elapsed().as_secs_f32();
        let nusus = self.sijill.adad();
        TaqaddumIltiqat {
            nusus,
            jadeeda: self.jadeeda,
            ahdath: self.ihsaat.ahdath,
            mudda_thawani: mudda,
            fi_thaniya: if mudda > 0.0 {
                ila_kasr(self.ihsaat.ahdath) / mudda
            } else {
                0.0
            },
            taghtiya_muqaddara: taghtiya_muqaddara(nusus, self.adad_sakin),
            khutwa: self.mizaniya.khutwa(),
            muayyan: self.mizaniya.ihsaat().mahdhufa_mutakarrira > 0,
            mumtali: self.sijill.mumtali(),
            mawthuqa: self.mawthuqa(),
        }
    }

    /// Ends the session, writes the footer, and syncs everything.
    ///
    /// # Errors
    ///
    /// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the pending
    /// updates, the footer, or the final sync fail. The observations are still
    /// in the returned error's caller's hands only if it succeeded, which is why
    /// the file is the source of truth and not this object.
    pub fn anhi(mut self) -> Natija<TaqreerJalsa> {
        self.adfiq()?;
        let ihsaat = self.ihsaat_hadith();
        let intaha = jiff::Timestamp::now().to_string();
        let masar = self.katib.masar().to_path_buf();
        let taqreer = TaqreerJalsa {
            jalsa: self.jalsa.clone(),
            masar,
            bada: self.bada.clone(),
            intaha: intaha.clone(),
            nusus: self.sijill.adad(),
            adad_sakin: self.adad_sakin,
            ihsaat,
        };
        self.katib.aghliq(KhitamJalsa { intaha, ihsaat })?;
        Ok(taqreer)
    }

    /// The observations, for a merge that runs without going through the file.
    pub fn mulahazat(&self) -> impl Iterator<Item = &MulahazaMutakarrira> {
        self.sijill.mulahazat()
    }

    /// The observation table.
    #[must_use]
    pub const fn sijill(&self) -> &SijillMulahazat {
        &self.sijill
    }

    /// The counters, brought up to date with the table and the budget.
    #[must_use]
    pub fn ihsaat(&self) -> IhsaatJalsa {
        self.ihsaat_hadith()
    }

    /// Whether every number this session reports can be taken at face value.
    ///
    /// See [`TaqreerJalsa::taghtiya_mawthuqa`] for what each disqualifier means.
    #[must_use]
    pub fn mawthuqa(&self) -> bool {
        self.ihsaat_hadith().kamila()
    }

    /// The counters, plus the values that live in the table and the budget.
    fn ihsaat_hadith(&self) -> IhsaatJalsa {
        let mut ihsaat = self.ihsaat;
        ihsaat.nusus = u64::try_from(self.sijill.adad()).unwrap_or(u64::MAX);
        ihsaat.mizaniya = *self.mizaniya.ihsaat();
        ihsaat.asturr_talifa = self.katib.talifa();
        ihsaat
    }

    /// An update line for one key, if the key still names a record.
    fn tahdith(&self, miftah: MiftahMulahaza) -> Option<SatrJalsa> {
        let mulahaza = self.sijill.mulahaza(miftah)?;
        Some(SatrJalsa::Tahdith(TahdithMulahaza {
            miftah: mulahaza.miftah.clone(),
            marrat: mulahaza.marrat,
            akhir_waqt_mil: mulahaza.akhir_waqt_mil,
            quyud: mulahaza.quyud.clone(),
            muayyana: mulahaza.muayyana,
        }))
    }

    /// Counts a break in the adapter's sequence.
    ///
    /// Only forward gaps count. A sequence that goes backwards is an adapter that
    /// restarted its own counter — a scene reload that reinitialised the capture
    /// hook, most often — which loses nothing and is not evidence of a dropped
    /// message.
    const fn tafaqqad_tasalsul(&mut self, tasalsul: u64) {
        if let Some(sabiq) = self.akhir_tasalsul
            && tasalsul > sabiq.saturating_add(1)
        {
            let fajwa = tasalsul.saturating_sub(sabiq).saturating_sub(1);
            self.ihsaat.fajawat = self.ihsaat.fajawat.saturating_add(fajwa);
        }
        self.akhir_tasalsul = Some(tasalsul);
    }
}

/// What a session has done so far.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TaqaddumIltiqat {
    /// Distinct strings held.
    pub nusus: usize,
    /// Of those, how many were first seen in this run.
    pub jadeeda: u64,
    /// Events offered.
    pub ahdath: u64,
    /// How long the session has been running, in seconds.
    pub mudda_thawani: f32,
    /// Events per second, averaged over the whole session.
    pub fi_thaniya: f32,
    /// The coverage estimate, when a static count was supplied.
    ///
    /// See [`taghtiya_muqaddara`] for exactly what this number is and — more
    /// importantly — what it is not.
    pub taghtiya_muqaddara: Option<f32>,
    /// The sampler's current stride; one means nothing is being sampled out.
    pub khutwa: u32,
    /// Whether the sampler has discarded anything at all this session.
    pub muayyan: bool,
    /// Whether the observation table is full and refusing new strings.
    pub mumtali: bool,
    /// Whether every number here can be taken at face value.
    pub mawthuqa: bool,
}

impl TaqaddumIltiqat {
    /// The line the capture panel shows, in English.
    #[must_use]
    pub fn wasf_injilizi(&self) -> String {
        let mut satr = format!(
            "{} string(s) captured ({} new), {} event(s) at {:.0}/s",
            self.nusus, self.jadeeda, self.ahdath, self.fi_thaniya
        );
        if let Some(taghtiya) = self.taghtiya_muqaddara {
            let miya = taghtiya * 100.0;
            let _ = write!(&mut satr, ", about {miya:.0}% of the static table's size");
        }
        if self.mumtali {
            satr.push_str("; the observation table is full and new strings are being refused");
        } else if self.muayyan {
            satr.push_str("; sampling is active, so occurrence counts are approximate");
        }
        satr
    }
}

/// What a finished session produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaqreerJalsa {
    /// The session's name.
    pub jalsa: String,
    /// The file it wrote.
    pub masar: PathBuf,
    /// When it started, RFC 3339.
    pub bada: String,
    /// When it ended, RFC 3339.
    pub intaha: String,
    /// Distinct strings captured.
    pub nusus: usize,
    /// The static denominator, when one was supplied.
    pub adad_sakin: Option<usize>,
    /// Everything counted.
    pub ihsaat: IhsaatJalsa,
}

impl TaqreerJalsa {
    /// The coverage estimate.
    #[must_use]
    pub fn taghtiya(&self) -> Option<f32> {
        taghtiya_muqaddara(self.nusus, self.adad_sakin)
    }

    /// Whether the coverage number is a measurement or a floor.
    ///
    /// **A session that silently sampled forty percent would report a number
    /// that is a lie**, so every way a session can lose an event disqualifies its
    /// figure and the interface says so instead of rounding it off:
    ///
    /// - the per-frame ceiling refused events, which can include unseen strings;
    /// - the observation table filled and refused new strings;
    /// - the transport reported dropping messages;
    /// - the adapter's sequence has gaps, so messages were lost regardless of
    ///   what the transport reported;
    /// - a record could not be serialised.
    ///
    /// Sampled *repeats* are deliberately not on that list. They cost occurrence
    /// weighting and width tightening, never a string, so they do not make the
    /// count of distinct strings wrong — see [`MizaniyatItar`].
    #[must_use]
    pub const fn taghtiya_mawthuqa(&self) -> bool {
        self.ihsaat.kamila()
    }

    /// The report as lines, for the log and the diagnostics bundle.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "capture session \"{}\": {} distinct string(s) from {} event(s), {} to {}",
            self.jalsa, self.nusus, self.ihsaat.ahdath, self.bada, self.intaha
        )];
        if let Some(taghtiya) = self.taghtiya() {
            sutur.push(format!(
                "  about {:.0}% of the static table's size{}",
                taghtiya * 100.0,
                if self.taghtiya_mawthuqa() {
                    ""
                } else {
                    " — and this is a floor, not a figure"
                }
            ));
        }
        sutur.push(format!("  {}", self.ihsaat.mizaniya.wasf_injilizi()));
        if self.ihsaat.marfuda_imtila > 0 {
            sutur.push(format!(
                "  {} new string(s) refused: the session's observation table was full",
                self.ihsaat.marfuda_imtila
            ));
        }
        if self.ihsaat.fajawat > 0 || self.ihsaat.faqd_naql > 0 {
            sutur.push(format!(
                "  {} message(s) missing from the adapter's sequence, and {} the transport \
                 reported dropping",
                self.ihsaat.fajawat, self.ihsaat.faqd_naql
            ));
        }
        if self.ihsaat.marfuda_farigh > 0 || self.ihsaat.marfuda_tawil > 0 {
            sutur.push(format!(
                "  {} empty draw(s) and {} over-long draw(s) ignored, which cost no strings",
                self.ihsaat.marfuda_farigh, self.ihsaat.marfuda_tawil
            ));
        }
        if self.ihsaat.asturr_talifa > 0 {
            sutur.push(format!(
                "  {} record(s) could not be serialised and were not written",
                self.ihsaat.asturr_talifa
            ));
        }
        sutur
    }
}

/// The coverage estimate, and an honest statement of what it means.
///
/// It is the number of distinct strings capture holds divided by the number
/// static extraction produced, clamped to one.
///
/// **It is not the fraction of the static table that has been seen.** The two
/// sets overlap; they are not nested. Capture holds strings static extraction
/// never had — anything composed at runtime — and static extraction holds
/// strings on screens the player has not opened. So a session can sit at ninety
/// percent while having missed a third of the menus, and the true intersection is
/// only known after [`crate::dammij`] has matched them by content.
///
/// It is reported anyway, and reported as an *estimate*, because while somebody
/// is playing it is the only signal available and it answers the question they
/// are actually asking: is this working, and is there more to find. The precise
/// answer arrives one step later.
#[must_use]
pub fn taghtiya_muqaddara(nusus: usize, adad_sakin: Option<usize>) -> Option<f32> {
    let kull = adad_sakin?;
    if kull == 0 {
        return None;
    }
    let juz = u64::try_from(nusus).unwrap_or(u64::MAX);
    let maqam = u64::try_from(kull).unwrap_or(u64::MAX);
    Some(nisba(juz, maqam).min(1.0))
}

/// A counter as a float, for a rate.
#[expect(
    clippy::cast_precision_loss,
    reason = "event counts are far below 2^53, and the result is a rate shown to the nearest \
              whole event per second"
)]
const fn ila_kasr(qeema: u64) -> f32 {
    qeema as f32
}

// ---------------------------------------------------------------------------
// Reading a session back
// ---------------------------------------------------------------------------

/// A session file, read back.
///
/// Produced by [`iqra_jalsa`]. Carries what the file said *and* what was wrong
/// with it, because a session file is written by a process that can be killed at
/// any instant and a reader that returned only the good records would present a
/// truncated capture as a complete one.
#[derive(Debug)]
pub struct JalsaMuhammala {
    /// The first header found.
    pub tarwisa: TarwisatJalsa,
    /// The folded records.
    pub sijill: SijillMulahazat,
    /// The footer, when the session closed cleanly.
    pub khitam: Option<KhitamJalsa>,
    /// Lines that could not be parsed, excluding a truncated final one.
    pub sutur_talifa: u64,
    /// Whether the file's last line was cut off mid-record.
    ///
    /// Distinguished from a corrupt line in the middle: a truncated tail is the
    /// ordinary signature of a process that died with bytes in flight, and a
    /// corrupt line in the middle is not ordinary at all.
    pub mabtura: bool,
    /// Update lines whose key names no record in the file.
    ///
    /// A count update with no preceding string means the line carrying the
    /// string was lost — which is only possible if the file was edited or the
    /// filesystem reordered writes, and is worth reporting rather than ignoring.
    pub tahdithat_yatima: u64,
}

impl JalsaMuhammala {
    /// Whether the session that wrote this file ended cleanly.
    #[must_use]
    pub const fn iktamalat(&self) -> bool {
        self.khitam.is_some()
    }

    /// How many distinct strings were recovered.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.sijill.adad()
    }

    /// The records.
    pub fn mulahazat(&self) -> impl Iterator<Item = &MulahazaMutakarrira> {
        self.sijill.mulahazat()
    }

    /// The lines a diagnostics bundle shows for this file.
    #[must_use]
    pub fn taqreer(&self) -> Vec<String> {
        let mut sutur = vec![format!(
            "session \"{}\" ({}): {} distinct string(s)",
            self.tarwisa.jalsa,
            if self.iktamalat() {
                "closed cleanly"
            } else {
                "ended without a footer"
            },
            self.adad()
        )];
        if self.mabtura {
            sutur.push(
                "  the last line was cut off, which is what a killed process leaves behind; \
                 everything before it is intact"
                    .to_owned(),
            );
        }
        if self.sutur_talifa > 0 {
            sutur.push(format!(
                "  {} line(s) in the middle of the file could not be parsed and were skipped",
                self.sutur_talifa
            ));
        }
        if self.tahdithat_yatima > 0 {
            sutur.push(format!(
                "  {} update(s) referred to a string this file does not contain",
                self.tahdithat_yatima
            ));
        }
        if self.sijill.marfuda_imtila() > 0 {
            sutur.push(format!(
                "  {} string(s) in the file were not loaded: the reader's bound was reached",
                self.sijill.marfuda_imtila()
            ));
        }
        sutur
    }
}

/// Reads a session file back into records.
///
/// Streams the file line by line rather than loading it: a long session is
/// hundreds of megabytes of update lines, and every one of them is superseded by
/// a later one for the same key, so the peak memory is the observation table and
/// not the file.
///
/// **A malformed line is skipped and counted, never fatal.** The file is written
/// by a process that can be killed between two `write` calls; refusing to read a
/// two-hour capture because its final forty bytes are missing would throw away
/// everything the session was for. What *is* fatal is a file that cannot be
/// opened, or whose header is missing or from a build newer than this one —
/// because in that case there is no way to know what the rest of the lines mean.
///
/// # Errors
///
/// [`crate::khata::KhataIstikhraj::KhataIltiqat`] when the file cannot be
/// opened or read, when it carries no readable header, or when its
/// [`TarwisatJalsa::isdar`] is newer than [`ISDAR_JALSA`].
pub fn iqra_jalsa(masar: &Path, hadd: usize) -> Natija<JalsaMuhammala> {
    let malaf = File::open(masar).map_err(|sabab| khata_qira(masar, &sabab))?;
    let qari = BufReader::new(malaf);

    let mut tarwisa: Option<TarwisatJalsa> = None;
    let mut sijill = SijillMulahazat::jadeed(hadd);
    let mut khitam: Option<KhitamJalsa> = None;
    let mut sutur_talifa = 0_u64;
    let mut tahdithat_yatima = 0_u64;
    let mut akhir_talif = false;

    for satr in qari.lines() {
        let satr = match satr {
            Ok(satr) => satr,
            // Bytes that are not UTF-8. `lines` has already found the newline,
            // so the reader's position is intact and the next line is readable;
            // this is one damaged line, not a damaged file, and the same policy
            // applies to it as to one that is not valid JSON.
            Err(sabab) if sabab.kind() == std::io::ErrorKind::InvalidData => {
                sutur_talifa = sutur_talifa.saturating_add(1);
                akhir_talif = true;
                continue;
            },
            Err(sabab) => return Err(khata_qira(masar, &sabab)),
        };
        if satr.trim().is_empty() {
            continue;
        }
        let Ok(mufakkak) = serde_json::from_str::<SatrJalsa>(&satr) else {
            sutur_talifa = sutur_talifa.saturating_add(1);
            akhir_talif = true;
            continue;
        };
        akhir_talif = false;

        match mufakkak {
            SatrJalsa::Tarwisa(jadeeda) => {
                let isdar = jadeeda.isdar;
                if isdar > ISDAR_JALSA {
                    return Err(khata_qira(
                        masar,
                        &std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!(
                                "this session file was written by a newer build of Taarib \
                                 (format {isdar}, this build reads {ISDAR_JALSA}); reading it \
                                 would mean silently discarding measurements this build does \
                                 not understand"
                            ),
                        ),
                    ));
                }
                // The first header wins for identity; a resumed session writes
                // another one and its settings are the ones now in force, so the
                // bound and the budget are taken from the latest.
                match tarwisa.as_mut() {
                    Some(sabiq) => {
                        sabiq.aqsa_mulahazat = jadeeda.aqsa_mulahazat;
                        sabiq.mizaniyat_itar_mikro = jadeeda.mizaniyat_itar_mikro;
                        if sabiq.adad_sakin.is_none() {
                            sabiq.adad_sakin = jadeeda.adad_sakin;
                        }
                    },
                    None => tarwisa = Some(jadeeda),
                }
            },
            SatrJalsa::Nass(mulahaza) => {
                let _ = sijill.ahill(mulahaza);
            },
            SatrJalsa::Tahdith(tahdith) => {
                if !tabbiq_tahdith(&mut sijill, &tahdith) {
                    tahdithat_yatima = tahdithat_yatima.saturating_add(1);
                }
            },
            SatrJalsa::Khitam(nihaya) => khitam = Some(nihaya),
        }
    }

    let Some(tarwisa) = tarwisa else {
        return Err(khata_qira(
            masar,
            &std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "this session file has no header, so there is no way to know what format its \
                 records are in or which game they belong to",
            ),
        ));
    };

    // A parse failure on the final line, with no footer after it, is a truncated
    // tail rather than corruption — and it is not counted as corruption, because
    // reporting a normal crash signature as file damage would send somebody
    // looking for a disk fault that is not there.
    let mabtura = akhir_talif && khitam.is_none();
    if mabtura {
        sutur_talifa = sutur_talifa.saturating_sub(1);
    }

    Ok(JalsaMuhammala {
        tarwisa,
        sijill,
        khitam,
        sutur_talifa,
        mabtura,
        tahdithat_yatima,
    })
}

/// Applies one update line, answering whether it found its record.
///
/// Last writer wins for every field the line carries. That is why
/// [`TahdithMulahaza`] carries the whole constraint set rather than a delta: a
/// reader that has lost intermediate lines to a truncation still lands on the
/// state the last surviving line describes, instead of applying a delta to a
/// base that never arrived.
fn tabbiq_tahdith(sijill: &mut SijillMulahazat, tahdith: &TahdithMulahaza) -> bool {
    let Some(miftah) = MiftahMulahaza::min_nass(&tahdith.miftah) else {
        return false;
    };
    let Some(mulahaza) = sijill.mulahazat.get_mut(&miftah) else {
        return false;
    };
    mulahaza.marrat = mulahaza.marrat.max(tahdith.marrat);
    mulahaza.akhir_waqt_mil = mulahaza.akhir_waqt_mil.max(tahdith.akhir_waqt_mil);
    mulahaza.quyud = tahdith.quyud.clone();
    mulahaza.muayyana = mulahaza.muayyana || tahdith.muayyana;
    true
}
