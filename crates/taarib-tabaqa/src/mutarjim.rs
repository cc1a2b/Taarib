//! المترجم — the two seams between a line read off a screen and the Arabic that
//! replaces it.
//!
//! Neither of the things this module names is implemented here, and that is the
//! design rather than an omission. The overlay is a `cdylib` that is loaded into
//! somebody else's game process, and the two things it needs — a translation
//! provider and the shared translation memory — are, in this workspace, a
//! `reqwest`/`rustls` HTTP client on a Tokio runtime and a bundled `SQLite`
//! database. Linking either into the payload would put a TLS stack and a
//! database engine into the address space of a game that never asked for one,
//! next to `[profile.haqn]`, whose whole purpose is that injected libraries are
//! built small.
//!
//! So the payload takes both as trait objects, exactly as it already takes a
//! renderer through [`crate::wajiha::Khattaf`] and a recognizer through
//! [`crate::qira::Qari`]. The caller that has a runtime supplies one; the
//! caller that does not gets [`DhakiraJalsa`] and whatever project translation
//! it can reach, and still draws Arabic.
//!
//! ## The context is not a second mechanism
//!
//! [`TalabSatr::siyaq`] is [`taarib_mustalahat::nass::SiyaqNass`] — the *same*
//! type the static translation path fills in and hands to a provider, whose
//! `jiwar` field is already the neighbouring-lines carrier that
//! `taarib_tarjama::siyaq::SiyaqTalab::jiwar_mahdud` bounds and
//! `risalat_mustakhdim` writes into the prompt. Inventing an overlay-shaped
//! context type would have meant two things that mean "the lines around this
//! one", with only one of them exercised by the pipeline that reads it. The
//! overlay fills the fields it honestly knows — the region as the location, the
//! previously settled lines as the neighbours — and leaves the rest as the
//! [`Default`] the static path also produces when an extractor knows nothing.
//!
//! ## The memory contract, and what it is not
//!
//! [`DhakiraTabaqa`] is deliberately narrow: an exact lookup and a store. It is
//! not a fuzzy-match interface, because the overlay has nothing useful to do
//! with a 92% match of a line it just read — the source text came off a screen
//! and may itself be wrong, and offering a near-match of an uncertain reading is
//! how a player gets fluent Arabic saying something the game did not say. The
//! near-match machinery belongs to the translator's own workbench, where a human
//! is looking at it.
//!
//! [`QaydTabaqa`] carries the recognizer's name, the provider's name and the
//! reading confidence for one reason: the shared memory distinguishes a pair a
//! human wrote, a pair a machine translated from an extracted string, and a pair
//! the overlay read off a screen and had machine-translated — the last being the
//! only one whose *source text* may be wrong. Handing over the provenance is
//! what lets it keep that distinction instead of laundering screen readings into
//! translations of strings the game actually ships.
//!
//! [`DhakiraJalsa`] exists so that a session always has a memory to talk to. It
//! is bounded, it is in memory, and it disappears with the process — it is the
//! null implementation of the contract, not a second cache, and it says so in
//! its own documentation. A second playthrough costs nothing only when the
//! shared memory is attached.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;

use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::nass::{SiyaqNass, TasnifNass};

use crate::khata::KhataTabaqa;
use crate::sijill_qira::MasdarTarjama;
use crate::wajiha::MustatilBiksel;

/// The default classification for a line the overlay read.
///
/// Dialogue. The overlay's regions are drawn around dialogue boxes and subtitle
/// strips by the people who draw them, and there is no extractor here to derive
/// anything better: tier 3 exists precisely because the game's own data could
/// not be read. Naming the honest default in one place beats each caller
/// guessing, and a caller who knows better — a region drawn around a menu —
/// overrides it per region.
pub const TASNIF_IFTIRADI: TasnifNass = TasnifNass::Hiwar;

/// How many pairs [`DhakiraJalsa`] holds before it forgets the oldest.
///
/// Two thousand. A three-hour dialogue-heavy session produces on the order of a
/// thousand distinct lines, so this holds a whole session without evicting while
/// staying a bounded allocation inside a process that is not ours.
pub const SAA_DHAKIRAT_JALSA: usize = 2_000;

/// One settled reading, with everything a provider needs to translate it well.
#[derive(Debug, Clone)]
pub struct TalabSatr<'a> {
    /// What the recognizer read, whitespace-normalized.
    pub asl: &'a str,

    /// Where it came from and what was said around it.
    ///
    /// The static path's own context type. See this module's header.
    pub siyaq: &'a SiyaqNass,

    /// What kind of string the region says this is.
    pub tasnif: TasnifNass,

    /// The game, so a provider and a memory can scope what they know to it.
    pub luba: LubaId,

    /// The game's display name, where the caller knows it.
    pub ism_luba: Option<&'a str>,

    /// The rectangle the original text occupied, in surface pixels.
    ///
    /// A genuinely *measured* constraint, which almost nothing else in this
    /// product has: the recognizer read the line out of this box and the overlay
    /// will draw Arabic back into it, so the room the translation has is known
    /// exactly rather than estimated. It maps onto
    /// [`taarib_mustalahat::nass::QuyudNass::mustatil`], which is documented as
    /// "the rectangle the original occupied on screen" for precisely this case.
    pub mawdi: MustatilBiksel,

    /// The recognizer's confidence in the reading, zero to a hundred.
    ///
    /// Passed through rather than acted on here. A provider that wants to hedge
    /// a low-confidence reading can; one that does not can ignore it. What it is
    /// definitely for is the memory: a pair stored from a reading the recognizer
    /// was unsure of has to be stored *as* one.
    pub thiqa: u8,
}

/// What came back for one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaddSatr {
    /// The Arabic, in logical order, as ordinary Unicode.
    ///
    /// Never presentation forms. Joining and the four contextual shapes come out
    /// of the font's own tables when this is shaped, which is the one place in
    /// the product they are allowed to come from.
    pub arabi: String,

    /// Which of the three kinds of translation this is.
    ///
    /// [`MasdarTarjama`] rather than a boolean, and the third state is the one
    /// that costs something to lose: a line the shared memory answered was read
    /// off *somebody's* screen and machine-translated from that reading, which
    /// is a weaker claim than "a machine translated this game's own string" and
    /// a much weaker one than "a person wrote this". Collapsing the three into
    /// "human or not" leaves the reading history unable to tell a player where a
    /// sentence they are scrolling back to actually came from.
    pub masdar: MasdarTarjama,
}

impl RaddSatr {
    /// A machine translation of this game's own text, produced now.
    #[must_use]
    pub fn aaliya(arabi: impl Into<String>) -> Self {
        Self {
            arabi: arabi.into(),
            masdar: MasdarTarjama::Aaliya,
        }
    }

    /// A translation a human wrote, from a project for this game.
    #[must_use]
    pub fn basharia(arabi: impl Into<String>) -> Self {
        Self {
            arabi: arabi.into(),
            masdar: MasdarTarjama::Mashru,
        }
    }

    /// A translation of somebody's screen reading, out of the shared memory.
    #[must_use]
    pub fn mulahaza(arabi: impl Into<String>) -> Self {
        Self {
            arabi: arabi.into(),
            masdar: MasdarTarjama::Mulahaza,
        }
    }

    /// Whether there is anything to draw.
    #[must_use]
    pub fn khali(&self) -> bool {
        self.arabi.trim().is_empty()
    }
}

/// Something that turns a settled reading into Arabic.
///
/// [`Send`] because this is called from the worker the session runs recognition
/// on, never from the render thread. An implementation is expected to block:
/// that thread exists to be blocked on, and it is why the overlay's draw is not.
///
/// [`fmt::Debug`] is a supertrait for the reason it is one on
/// [`crate::qira::Qari`] and [`crate::wajiha::Khattaf`]: a diagnostics bundle
/// that cannot name which provider produced a bad sentence is a bundle missing
/// the first thing anybody asks.
pub trait MutarjimTabaqa: Send + fmt::Debug {
    /// The name shown in the control panel and written to the log.
    fn ism(&self) -> &str;

    /// Whether this translator can be reached right now.
    ///
    /// Asked before every dispatch rather than once at construction, because a
    /// local model server is a process a user can stop while the game is running
    /// and a network is a thing that goes away.
    fn mutah(&self) -> bool;

    /// Translates one settled line.
    ///
    /// # Errors
    ///
    /// Whatever the implementation cannot do, as a [`KhataTabaqa`]. The session
    /// counts the failure, records it against the line, and does not retry
    /// forever — see [`crate::qissa::KhiyaratQissa::aqsa_muhawalat`].
    fn tarjim(&self, talab: &TalabSatr<'_>) -> Result<RaddSatr, KhataTabaqa>;
}

/// One lookup against the shared translation memory.
#[derive(Debug, Clone, Copy)]
pub struct TalabDhakira<'a> {
    /// The reading, exactly as it will be stored.
    pub asl: &'a str,
    /// The game it was read in.
    pub luba: LubaId,
    /// What kind of string it is.
    pub tasnif: TasnifNass,
}

/// One pair on its way into the shared translation memory.
///
/// Every field beyond the pair itself is provenance, and all of it is carried
/// rather than dropped. See this module's header.
#[derive(Debug, Clone, Copy)]
pub struct QaydTabaqa<'a> {
    /// The reading.
    pub asl: &'a str,
    /// The Arabic that came back for it.
    pub arabi: &'a str,
    /// The game.
    pub luba: LubaId,
    /// The game's display name, kept so a stored pair can name its origin.
    pub ism_luba: Option<&'a str>,
    /// The region the line was read from, by the name the player gave it.
    ///
    /// Never used for matching — the memory keys on text — but a translator
    /// looking at a stored reading later wants to know it came off the subtitle
    /// strip rather than the quest log, and the overlay is the only thing that
    /// ever knows that.
    pub mintaqa: Option<&'a str>,
    /// What kind of string it is.
    pub tasnif: TasnifNass,
    /// Which recognizer read it, as [`crate::qira::Qari::ism`] names itself.
    pub qari: Option<&'a str>,
    /// Which translator produced the Arabic.
    pub muzawwid: Option<&'a str>,
    /// The recognizer's confidence, zero to a hundred, when it measured one.
    ///
    /// [`None`] is not "zero": it means the engine reports no confidence at all,
    /// which is a different thing from an engine that reported no confidence
    /// *in this line*. See [`crate::qira::THIQA_GHAYR_MAQISA`].
    pub thiqa: Option<u8>,
}

/// The overlay's side of the shared translation memory.
///
/// Implemented over the workspace's memory by whichever caller has it; the
/// overlay never opens a database itself. [`Sync`] as well as [`Send`] because
/// one memory is shared between the worker that writes it and any panel that
/// reads it.
pub trait DhakiraTabaqa: Send + Sync + fmt::Debug {
    /// The name shown in the control panel.
    fn ism(&self) -> &str;

    /// The stored Arabic for this exact reading, in this game.
    ///
    /// Exact, not near. See this module's header on why the overlay does not
    /// take fuzzy matches.
    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr>;

    /// Stores a pair the overlay produced.
    ///
    /// # Errors
    ///
    /// Whatever the store refuses. A failure here is counted and reported and
    /// never stops a line being drawn: a translation that could not be
    /// remembered is still a translation.
    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa>;

    /// Whether this memory outlives the process.
    ///
    /// The control panel says "a second playthrough will be free" only when this
    /// is true, and it says so because [`DhakiraJalsa`] answers `false`.
    fn daaima(&self) -> bool;
}

/// A memory that lives as long as the session and no longer.
///
/// **Not a second cache**, and not a substitute for one. The shared translation
/// memory is what makes a second playthrough free; this is what keeps a session
/// from paying twice for the pause menu the player opened nine times, and it
/// disappears when the game closes. A session always has one so that the lookup
/// path in [`crate::qissa::Qissa`] has exactly one shape rather than a branch on
/// whether a memory was configured.
///
/// Bounded and oldest-first, for the reason [`crate::sijill_qira::SijillQira`]
/// is bounded: it lives inside a game's address space and a session lasts hours.
#[derive(Debug, Clone)]
pub struct DhakiraJalsa {
    qiyud: HashMap<(LubaId, String), RaddSatr>,
    tarteeb: VecDeque<(LubaId, String)>,
    saa: usize,
    isabat: u64,
    ikhfaqat: u64,
}

impl DhakiraJalsa {
    /// An empty memory at the default capacity.
    #[must_use]
    pub fn jadeeda() -> Self {
        Self::bi_saa(SAA_DHAKIRAT_JALSA)
    }

    /// An empty memory at a chosen capacity, with a floor of one entry.
    #[must_use]
    pub fn bi_saa(saa: usize) -> Self {
        Self {
            qiyud: HashMap::new(),
            tarteeb: VecDeque::new(),
            saa: saa.max(1),
            isabat: 0,
            ikhfaqat: 0,
        }
    }

    /// How many pairs are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.qiyud.len()
    }

    /// How many lookups found something.
    #[must_use]
    pub const fn isabat(&self) -> u64 {
        self.isabat
    }

    /// How many did not.
    #[must_use]
    pub const fn ikhfaqat(&self) -> u64 {
        self.ikhfaqat
    }

    /// The sentence the control panel shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} pair(s) held of {}; {} hit(s), {} miss(es). This memory ends with the session — \
             attach the shared translation memory to make a second playthrough free.",
            self.qiyud.len(),
            self.saa,
            self.isabat,
            self.ikhfaqat
        )
    }

    /// Looks a reading up.
    fn jid(&mut self, luba: LubaId, asl: &str) -> Option<RaddSatr> {
        let mawjud = self.qiyud.get(&(luba, asl.to_owned())).cloned();
        if mawjud.is_some() {
            self.isabat = self.isabat.saturating_add(1);
        } else {
            self.ikhfaqat = self.ikhfaqat.saturating_add(1);
        }
        mawjud
    }

    /// Stores a pair, evicting the oldest when the capacity is reached.
    fn adhif(&mut self, luba: LubaId, asl: &str, radd: RaddSatr) {
        let miftah = (luba, asl.to_owned());
        if self.qiyud.insert(miftah.clone(), radd).is_none() {
            self.tarteeb.push_back(miftah);
        }
        while self.tarteeb.len() > self.saa {
            if let Some(aqdam) = self.tarteeb.pop_front() {
                let _ = self.qiyud.remove(&aqdam);
            }
        }
    }
}

impl Default for DhakiraJalsa {
    fn default() -> Self {
        Self::jadeeda()
    }
}

/// A session memory behind a lock, so it can implement the shared contract.
///
/// [`DhakiraTabaqa`] takes `&self` because the shared memory is shared; this
/// wrapper is what lets the in-session one satisfy the same signature without
/// every caller of the trait having to know which kind it holds.
#[derive(Debug)]
pub struct DhakiraJalsaMushtaraka(parking_lot::Mutex<DhakiraJalsa>);

impl DhakiraJalsaMushtaraka {
    /// Wraps a session memory for sharing.
    #[must_use]
    pub const fn jadeeda(dhakira: DhakiraJalsa) -> Self {
        Self(parking_lot::Mutex::new(dhakira))
    }

    /// A fresh session memory at the default capacity.
    #[must_use]
    pub fn iftiradiya() -> Self {
        Self::jadeeda(DhakiraJalsa::jadeeda())
    }

    /// The sentence the control panel shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        self.0.lock().wasf()
    }

    /// How many pairs are held.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.0.lock().adad()
    }
}

impl Default for DhakiraJalsaMushtaraka {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

impl DhakiraTabaqa for DhakiraJalsaMushtaraka {
    #[expect(
        clippy::unnecessary_literal_bound,
        reason = "the trait returns `&str` rather than `&'static str` because the shared memory \
                  names itself after the file it opened, which is owned by the implementation; \
                  an impl signature cannot narrow the trait's"
    )]
    fn ism(&self) -> &str {
        "session"
    }

    fn ibhath(&self, talab: &TalabDhakira<'_>) -> Option<RaddSatr> {
        self.0.lock().jid(talab.luba, talab.asl)
    }

    fn sajjil(&self, qayd: &QaydTabaqa<'_>) -> Result<(), KhataTabaqa> {
        // Stored as an observation, because that is what it is: this session
        // read the source off a screen and had a machine translate the reading.
        let radd = RaddSatr {
            arabi: qayd.arabi.to_owned(),
            masdar: MasdarTarjama::Mulahaza,
        };
        self.0.lock().adhif(qayd.luba, qayd.asl, radd);
        Ok(())
    }

    fn daaima(&self) -> bool {
        false
    }
}
