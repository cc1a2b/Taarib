//! الملاحظات — what the overlay saw, kept so that seeing it again costs
//! nothing.
//!
//! The overlay can only translate what a player walks past. A menu nobody
//! opened, an item description nobody read, a document nobody picked up: none
//! of it exists to translate until somebody passes through it. Nothing in the
//! recognizer or the graphics backends changes that, and the only thing that
//! does is refusing to pay twice — within a session, across sessions, and
//! across a second playthrough — and then letting one player's walk-through
//! serve the next player's.
//!
//! This module is the first half of that: the per-game accumulation. The
//! second half — a shareable, signed artifact — is `taarib_warsha`'s, because
//! sharing is that crate's subject and a signing key has no business inside a
//! translation pipeline.
//!
//! ## Why this is not just `Dhakira`
//!
//! [`Dhakira`] already stores pairs and already answers exact lookups. What
//! it does not have is a shape for the caller the overlay actually is:
//!
//! * **One game, for the whole session.** Every observation belongs to the
//!   game being played, and threading the identity through every call is how
//!   one line ends up filed under the wrong title. [`DhakiratTabaqa`] takes
//!   it once.
//! * **No human in front of it.** A fuzzy match is *never* applied by
//!   anything ([`Dhakira`]'s own rule), and the overlay has nobody to offer
//!   one to, so it asks [`Dhakira::tatbiq_tamm`] and pays for one indexed
//!   equality rather than a trigram probe and sixteen edit distances per
//!   recognized line.
//! * **A repeat within the frame budget.** A dialogue box sits on screen for
//!   two hundred frames and the recognizer reads it out of every poll. The
//!   durable lookup is a `SQLite` round trip; the session map is a hash. The
//!   durable store is still the real memory — an eviction here costs a
//!   database read, never a translation.
//!
//! ## What is counted, and why it is counted
//!
//! [`IhsaatTabaqa::ikhfaqat`] is the number of translations that actually had
//! to be paid for. It is the only number in this module that costs money, and
//! it is reported rather than derived so that "the same line twice costs one
//! translation" is a thing a caller can assert instead of a thing a caller
//! hopes.
//!
//! ## What this module refuses to store
//!
//! A reading that is too short to be a sentence, or that folds to nothing, or
//! that carries no letters at all — a health bar's `100/100`, a frame counter,
//! the decoration between two menu items. Storing those fills a memory with
//! rows nothing will ever match and pollutes a share with a hundred readings
//! of a number that changed. [`MulahazaTabaqa::salih`] is the rule and it is
//! deliberately conservative: it drops things that might have been text, and
//! the cost of that is one translation somebody pays again.

use std::collections::{HashMap, VecDeque};

use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::nass::TasnifNass;

use crate::dhakira::{
    AslQayd, Dhakira, NawAsl, QaydJadid, ThiqatQira, miftah_muwahhad,
};
use crate::khata::KhataTarjama;

/// The fewest characters a reading may have and still be worth storing.
///
/// Two. One character is an icon the recognizer mistook for a letter far more
/// often than it is a word, and the Arabic for a single Latin character is
/// not a translation anybody needs cached.
pub const ADNA_TUL_MULAHAZA: usize = 2;

/// The most characters a reading may carry.
///
/// Two thousand, matching `taarib_tabaqa::sijill_qira::AQSA_TUL_SATR` —
/// the overlay's own history has already cut anything longer, so a reading
/// arriving longer than this came from somewhere that did not, and the memory
/// applies the same ceiling rather than trusting the caller's.
pub const AQSA_TUL_MULAHAZA: usize = 2_000;

/// How many distinct readings the session map holds before it starts evicting.
///
/// Four thousand. A play session's on-screen text is bounded by what a player
/// walks past in a sitting, and this is comfortably past that for the games
/// tier 3 exists for. It matters little either way: an eviction costs one
/// indexed `SQLite` read, because the durable memory is the real cache and
/// this map is only there to keep a dialogue box that is on screen for two
/// hundred frames from being two hundred round trips.
pub const SAA_JALSA: usize = 4_096;

/// One line the overlay read and had translated.
///
/// The ingest type, deliberately owned by this crate and not by the overlay.
/// `taarib-tabaqa` compiles a graphics stack, an inference runtime and four
/// platform APIs; a translation pipeline that depended on it to name a struct
/// would drag all of that into every build of the Studio's translation
/// screens. The conversion is a field mapping and it belongs at the overlay's
/// edge.
///
/// The mapping from `taarib_tabaqa::sijill_qira::MadkhalQira` is exact, and the
/// field that makes it exact is the whole point of [`ThiqatQira`]: the history
/// entry carries `thiqa` **and** `maqisa`, so a caller harvesting a history
/// file can tell a real measurement from `taarib_tabaqa::qira`'s stand-in
/// constant and passes [`ThiqatQira::Ghayr`] only where no engine reported a
/// number. A caller whose source lacks that bit — anything that has only a
/// `thiqa` — has no way to tell and must pass [`ThiqatQira::Ghayr`] for all of
/// it, because the alternative is laundering a constant into a store that
/// refuses to hold one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MulahazaTabaqa {
    /// What the recognizer read, verbatim.
    pub asl: String,
    /// The Arabic that came back for it.
    pub arabi: String,
    /// The game being played.
    ///
    /// A [`LubaId`] rather than the string the memory stores, because the
    /// column holds one particular rendering of that identity and a `String`
    /// parameter is an invitation to pass a display name into it. The
    /// rendering happens once, in [`MulahazaTabaqa::ila_qayd`].
    pub luba: LubaId,
    /// Its display name, so a suggestion can name the game later.
    pub ism_luba: Option<String>,
    /// The overlay region the line came from, for the suggestion card.
    ///
    /// Never used for matching — the memory keys on text — but a translator
    /// looking at a shared reading wants to know it came from the subtitle
    /// strip rather than the quest log.
    pub mintaqa: Option<String>,
    /// What kind of string it appears to be.
    pub tasnif: TasnifNass,
    /// Which recognizer read it, as `taarib_tabaqa::qira` names itself.
    pub qari: Option<String>,
    /// Which provider translated the reading.
    pub muzawwid: Option<String>,
    /// What the recognizer said about the reading, or that it said nothing.
    pub thiqa: ThiqatQira,
}

impl MulahazaTabaqa {
    /// A reading with only the fields that have no honest default.
    ///
    /// Every optional field is a builder step, so a call site that does not
    /// know the recognizer's name says nothing rather than inventing one.
    #[must_use]
    pub fn jadeeda(
        asl: impl Into<String>,
        arabi: impl Into<String>,
        luba: LubaId,
        thiqa: ThiqatQira,
    ) -> Self {
        Self {
            asl: asl.into(),
            arabi: arabi.into(),
            luba,
            ism_luba: None,
            mintaqa: None,
            tasnif: TasnifNass::Majhul,
            qari: None,
            muzawwid: None,
            thiqa,
        }
    }

    /// Names the game.
    #[must_use]
    pub fn bi_ism_luba(mut self, ism: impl Into<String>) -> Self {
        self.ism_luba = Some(ism.into());
        self
    }

    /// Names the overlay region the line came from.
    #[must_use]
    pub fn bi_mintaqa(mut self, mintaqa: impl Into<String>) -> Self {
        self.mintaqa = Some(mintaqa.into());
        self
    }

    /// Classifies the reading.
    #[must_use]
    pub const fn bi_tasnif(mut self, tasnif: TasnifNass) -> Self {
        self.tasnif = tasnif;
        self
    }

    /// Names the recognizer and the provider.
    #[must_use]
    pub fn bi_muharrikayn(
        mut self,
        qari: Option<String>,
        muzawwid: Option<String>,
    ) -> Self {
        self.qari = qari;
        self.muzawwid = muzawwid;
        self
    }

    /// Whether this reading is worth storing at all.
    ///
    /// Four refusals, each answering a thing recognizers actually return:
    /// an empty or whitespace reading, a reading below
    /// [`ADNA_TUL_MULAHAZA`], a reading with no letter anywhere in it — a
    /// hit-point counter, a frame time, a row of box-drawing decoration — and
    /// a translation that is empty. Nothing here judges *quality*; that is
    /// [`ThiqatQira`]'s job and it is carried rather than acted on.
    #[must_use]
    pub fn salih(&self) -> bool {
        let asl = self.asl.trim();
        let arabi = self.arabi.trim();
        asl.chars().count() >= ADNA_TUL_MULAHAZA
            && !arabi.is_empty()
            && asl.chars().any(char::is_alphabetic)
            && !miftah_muwahhad(asl).is_empty()
            && !miftah_muwahhad(arabi).is_empty()
    }

    /// The reading as a memory record, cut to [`AQSA_TUL_MULAHAZA`].
    ///
    /// Cut by characters rather than bytes, which is the difference between a
    /// shortened line and a panic on a multi-byte codepoint.
    #[must_use]
    pub fn ila_qayd(&self) -> QaydJadid {
        QaydJadid {
            masdar: iqtata(&self.asl),
            hadaf: iqtata(&self.arabi),
            tasnif: self.tasnif,
            // A reading belongs to no project: nobody opened one, and filing
            // it under one would put it in a project's statistics and its
            // export.
            mashru: None,
            luba: Some(self.luba.to_string()),
            ism_luba: self.ism_luba.clone(),
            siyaq: self.mintaqa.clone(),
            // The overlay draws plain text into a scraped rectangle; there is
            // no markup on either side to carry.
            nasq_masdar: Vec::new(),
            nasq_hadaf: Vec::new(),
            asl: AslQayd::Mulahaza {
                qari: self.qari.clone(),
                muzawwid: self.muzawwid.clone(),
                thiqa: self.thiqa,
            },
        }
    }
}

/// A line trimmed and cut to [`AQSA_TUL_MULAHAZA`] characters.
fn iqtata(nass: &str) -> String {
    let munaqqah = nass.trim();
    if munaqqah.chars().count() <= AQSA_TUL_MULAHAZA {
        return munaqqah.to_owned();
    }
    munaqqah.chars().take(AQSA_TUL_MULAHAZA).collect()
}

/// What the memory had to say about a line the overlay just read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaddTabaqa {
    /// The Arabic is already known and costs nothing.
    Jahiza {
        /// The Arabic to draw.
        arabi: String,
        /// Which of the three kinds it is, so the overlay's own panel can
        /// say whether a person ever read this sentence.
        naw: NawAsl,
        /// What the recognizer that first read it said about the reading.
        thiqa: ThiqatQira,
        /// Whether the answer came from the session map rather than the file.
        ///
        /// Reported because it is the difference between "free" and "one
        /// indexed read", and a caller measuring its own frame budget wants
        /// to know which it just paid.
        min_jalsa: bool,
    },
    /// Nothing is known and the caller has to translate it.
    Majhula,
}

impl RaddTabaqa {
    /// The Arabic, when there is any.
    #[must_use]
    pub fn arabi(&self) -> Option<&str> {
        match self {
            Self::Jahiza { arabi, .. } => Some(arabi),
            Self::Majhula => None,
        }
    }

    /// Whether this answer cost a translation.
    #[must_use]
    pub const fn majjaniya(&self) -> bool {
        matches!(self, Self::Jahiza { .. })
    }
}

/// What one session did, in numbers a caller can assert on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IhsaatTabaqa {
    /// Lines asked about.
    pub istifsarat: u64,
    /// Answered from the session map.
    pub isabat_jalsa: u64,
    /// Answered from the memory file.
    pub isabat_dhakira: u64,
    /// Not answered — the lines that cost a translation.
    pub ikhfaqat: u64,
    /// Readings recorded.
    pub masjula: u64,
    /// Readings refused by [`MulahazaTabaqa::salih`].
    pub muhmala: u64,
    /// Entries dropped from the session map to stay inside [`SAA_JALSA`].
    pub mustabaada: u64,
}

impl IhsaatTabaqa {
    /// How many translations this session actually had to pay for.
    ///
    /// Named rather than left as [`IhsaatTabaqa::ikhfaqat`] because that is
    /// the number the whole module exists to hold down, and a caller
    /// asserting on it should not have to know that a miss and a paid
    /// translation are the same event.
    #[must_use]
    pub const fn tarjamat_madfua(&self) -> u64 {
        self.ikhfaqat
    }

    /// How many lookups were answered without a translation.
    #[must_use]
    pub const fn isabat(&self) -> u64 {
        self.isabat_jalsa.saturating_add(self.isabat_dhakira)
    }

    /// The session as one line, the unflattering numbers included.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} line(s) asked, {} free ({} session, {} memory), {} paid for; \
             {} reading(s) recorded, {} rejected, {} evicted",
            self.istifsarat,
            self.isabat(),
            self.isabat_jalsa,
            self.isabat_dhakira,
            self.ikhfaqat,
            self.masjula,
            self.muhmala,
            self.mustabaada,
        )
    }
}

/// One remembered answer, as the session map holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SatrMahfuz {
    arabi: String,
    naw: NawAsl,
    thiqa: ThiqatQira,
}

/// The memory as the overlay uses it: one game, one session, one counter set.
///
/// Owns its [`Dhakira`] rather than borrowing one, because the overlay's
/// lifetime *is* the session's and a borrowed handle would put a lifetime
/// parameter on a type that is otherwise plain. [`DhakiratTabaqa::dhakira`]
/// hands the durable store back for the sharing path, and
/// [`DhakiratTabaqa::ila_dhakira`] gives it up entirely.
#[derive(Debug)]
pub struct DhakiratTabaqa {
    dhakira: Dhakira,
    luba: LubaId,
    muarrif: String,
    ism_luba: Option<String>,
    jalsa: HashMap<String, SatrMahfuz>,
    tarteeb: VecDeque<String>,
    saa: usize,
    ihsaat: IhsaatTabaqa,
}

impl DhakiratTabaqa {
    /// Wraps a memory for one game's session.
    #[must_use]
    pub fn jadeeda(dhakira: Dhakira, luba: LubaId) -> Self {
        Self {
            dhakira,
            luba,
            // Rendered once, here, so every query and every recorded row uses
            // the same spelling of the identity.
            muarrif: luba.to_string(),
            ism_luba: None,
            jalsa: HashMap::new(),
            tarteeb: VecDeque::new(),
            saa: SAA_JALSA,
            ihsaat: IhsaatTabaqa::default(),
        }
    }

    /// Names the game, so recorded readings carry a display name.
    #[must_use]
    pub fn bi_ism_luba(mut self, ism: impl Into<String>) -> Self {
        self.ism_luba = Some(ism.into());
        self
    }

    /// Sets the session map's capacity, floored at one.
    ///
    /// Zero would make every lookup a database read and every eviction a
    /// counter bump, which is a configuration with no use and one surprising
    /// symptom; one is the smallest honest cache.
    #[must_use]
    pub const fn bi_saa(mut self, saa: usize) -> Self {
        self.saa = if saa == 0 { 1 } else { saa };
        self
    }

    /// The game this session is for.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// What the session did so far.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatTabaqa {
        &self.ihsaat
    }

    /// The durable memory underneath.
    #[must_use]
    pub const fn dhakira(&self) -> &Dhakira {
        &self.dhakira
    }

    /// Gives the durable memory up, ending the session.
    #[must_use]
    pub fn ila_dhakira(self) -> Dhakira {
        self.dhakira
    }

    /// How many readings this game has accumulated, across every session.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the query cannot run.
    pub fn adad_mulahazat(&self) -> Result<u64, KhataTarjama> {
        self.dhakira.adad_luba(&self.muarrif, NawAsl::Mulahaza)
    }

    /// Asks whether a recognized line is already known.
    ///
    /// The session map first, the memory file second, and nothing else: a
    /// fuzzy match is never applied by anything in this product, and applying
    /// one here — with no human to show the similarity to and a source string
    /// that was itself a guess — would be stacking one guess on another.
    ///
    /// A hit from the file is recorded as a use ([`Dhakira::alim_istikhdam`])
    /// and cached, so the second occurrence in the same session costs a hash
    /// lookup and the first occurrence in the next session costs one indexed
    /// read.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when a statement fails or a stored
    /// row does not decode.
    pub fn istafhim(&mut self, asl: &str) -> Result<RaddTabaqa, KhataTarjama> {
        self.ihsaat.istifsarat = self.ihsaat.istifsarat.saturating_add(1);

        let miftah = miftah_muwahhad(asl);
        if miftah.is_empty() {
            self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
            return Ok(RaddTabaqa::Majhula);
        }

        if let Some(mahfuz) = self.jalsa.get(&miftah) {
            self.ihsaat.isabat_jalsa = self.ihsaat.isabat_jalsa.saturating_add(1);
            return Ok(RaddTabaqa::Jahiza {
                arabi: mahfuz.arabi.clone(),
                naw: mahfuz.naw,
                thiqa: mahfuz.thiqa,
                min_jalsa: true,
            });
        }

        let Some(tatbiq) = self.dhakira.tatbiq_tamm(asl)? else {
            self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
            return Ok(RaddTabaqa::Majhula);
        };

        self.dhakira.alim_istikhdam(tatbiq.qayd.id)?;
        let mahfuz = SatrMahfuz {
            arabi: tatbiq.qayd.hadaf.clone(),
            naw: tatbiq.qayd.asl.naw,
            thiqa: tatbiq.qayd.asl.thiqa_qira,
        };
        self.ihfaz(miftah, mahfuz.clone());
        self.ihsaat.isabat_dhakira = self.ihsaat.isabat_dhakira.saturating_add(1);
        Ok(RaddTabaqa::Jahiza {
            arabi: mahfuz.arabi,
            naw: mahfuz.naw,
            thiqa: mahfuz.thiqa,
            min_jalsa: false,
        })
    }

    /// Records a reading and its translation, and remembers it for the rest
    /// of the session.
    ///
    /// The game identity and display name are this session's, not the
    /// reading's: a `DhakiratTabaqa` is for one game and letting a caller
    /// hand it a reading labelled with another one is how a line ends up
    /// filed under the wrong title and later shared as that game's.
    ///
    /// Answers `false` for a reading [`MulahazaTabaqa::salih`] refused,
    /// counted in [`IhsaatTabaqa::muhmala`] rather than returned as an error:
    /// a recognizer returning a hit-point counter is the recognizer working
    /// normally, not a failure anybody should see.
    ///
    /// # Errors
    ///
    /// [`KhataTarjama::DhakiraMughlaqa`] when the write fails.
    pub fn sajjil(&mut self, mulahaza: &MulahazaTabaqa) -> Result<bool, KhataTarjama> {
        let mut mulahaza = mulahaza.clone();
        mulahaza.luba = self.luba;
        if mulahaza.ism_luba.is_none() {
            mulahaza.ism_luba.clone_from(&self.ism_luba);
        }

        if !mulahaza.salih() {
            self.ihsaat.muhmala = self.ihsaat.muhmala.saturating_add(1);
            return Ok(false);
        }

        self.dhakira.sajjil(&mulahaza.ila_qayd())?;
        self.ihsaat.masjula = self.ihsaat.masjula.saturating_add(1);

        let miftah = miftah_muwahhad(&mulahaza.asl);
        // The session map holds what the *memory* would now answer, which is
        // not necessarily what was just written: a reading of a pair a human
        // has reviewed leaves the reviewed text in place, and caching the
        // reading would hand the overlay the worse of the two for the rest of
        // the session. One indexed read settles it.
        let mahfuz = match self.dhakira.tatbiq_tamm(&mulahaza.asl)? {
            Some(tatbiq) => SatrMahfuz {
                arabi: tatbiq.qayd.hadaf,
                naw: tatbiq.qayd.asl.naw,
                thiqa: tatbiq.qayd.asl.thiqa_qira,
            },
            None => SatrMahfuz {
                arabi: iqtata(&mulahaza.arabi),
                naw: NawAsl::Mulahaza,
                thiqa: mulahaza.thiqa,
            },
        };
        self.ihfaz(miftah, mahfuz);
        Ok(true)
    }

    /// Puts one answer in the session map, evicting the oldest if it is full.
    ///
    /// First-in-first-out rather than least-recently-used, and the reason is
    /// that the choice barely matters: an eviction costs one indexed read
    /// against a `SQLite` file, not a translation, so a second structure to
    /// track recency would be complexity bought for a saving nobody has
    /// measured. The durable memory is the cache; this is a frame-budget
    /// shortcut in front of it.
    fn ihfaz(&mut self, miftah: String, mahfuz: SatrMahfuz) {
        if self.jalsa.insert(miftah.clone(), mahfuz).is_none() {
            self.tarteeb.push_back(miftah);
        }
        while self.tarteeb.len() > self.saa {
            let Some(aqdam) = self.tarteeb.pop_front() else {
                break;
            };
            if self.jalsa.remove(&aqdam).is_some() {
                self.ihsaat.mustabaada = self.ihsaat.mustabaada.saturating_add(1);
            }
        }
    }
}

/// The accumulation, shareable between threads.
///
/// [`DhakiratTabaqa`] takes `&mut self` because it owns a `SQLite`
/// connection and mutates counters, and a `Connection` is [`Send`] but not
/// [`Sync`]. The overlay's own seam — `taarib_tabaqa::mutarjim::DhakiraTabaqa`
/// — requires `Send + Sync` with `&self` methods, because one memory is
/// shared between the worker thread that writes it and the control panel that
/// reads it. Somebody therefore has to hold a lock.
///
/// It is held here rather than by each integrator, so that the lock's scope
/// is one documented thing: **it is taken for the duration of one lookup or
/// one write and never across a translation.** That matters because a
/// provider call is a network round trip and a lock held across one would
/// stall the panel behind it for as long as somebody's endpoint takes.
/// [`DhakiraMushtaraka::istafhim`] answers before any translation begins and
/// [`DhakiraMushtaraka::sajjil`] runs after one has finished, so no call in
/// this type can span one.
#[derive(Debug)]
pub struct DhakiraMushtaraka(parking_lot::Mutex<DhakiratTabaqa>);

impl DhakiraMushtaraka {
    /// Shares one session's memory.
    #[must_use]
    pub const fn jadeeda(tabaqa: DhakiratTabaqa) -> Self {
        Self(parking_lot::Mutex::new(tabaqa))
    }

    /// The game this memory is for.
    #[must_use]
    pub fn luba(&self) -> LubaId {
        self.0.lock().luba()
    }

    /// The file this memory lives in.
    #[must_use]
    pub fn masar(&self) -> std::path::PathBuf {
        self.0.lock().dhakira().masar().to_path_buf()
    }

    /// Whether this memory outlives the process. Always true: it is a file.
    ///
    /// The overlay's control panel says "a second playthrough will be free"
    /// on the strength of this, and it is only allowed to say it about
    /// something durable.
    #[must_use]
    #[expect(
        clippy::unused_self,
        reason = "shaped to `taarib_tabaqa::mutarjim::DhakiraTabaqa::daaima`, whose receiver \
                  the trait fixes; an associated function here would make the adapter over \
                  this type write the answer itself, which is the fact this method owns"
    )]
    pub const fn daaima(&self) -> bool {
        true
    }

    /// Asks whether a recognized line is already known.
    ///
    /// # Errors
    ///
    /// As [`DhakiratTabaqa::istafhim`].
    pub fn istafhim(&self, asl: &str) -> Result<RaddTabaqa, KhataTarjama> {
        self.0.lock().istafhim(asl)
    }

    /// Records a reading and its translation.
    ///
    /// # Errors
    ///
    /// As [`DhakiratTabaqa::sajjil`].
    pub fn sajjil(&self, mulahaza: &MulahazaTabaqa) -> Result<bool, KhataTarjama> {
        self.0.lock().sajjil(mulahaza)
    }

    /// What the session did so far, copied out rather than borrowed.
    #[must_use]
    pub fn ihsaat(&self) -> IhsaatTabaqa {
        *self.0.lock().ihsaat()
    }

    /// Runs one operation against the durable memory under the lock.
    ///
    /// The escape hatch the sharing path needs: gathering a share reads
    /// [`Dhakira`] directly, and handing out a `&Dhakira` past the lock would
    /// let a caller read the file while the worker writes it. The closure is
    /// the scope, and it is the caller's job not to do anything slow inside
    /// one — the lock's contract above says why.
    pub fn maa<T>(&self, amal: impl FnOnce(&Dhakira) -> T) -> T {
        amal(self.0.lock().dhakira())
    }
}

#[cfg(test)]
mod fuhus {
    #![allow(
        clippy::panic,
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "a test reports failure by panicking; the lints are written for library \
                  code, and honouring them here would mean a test that cannot fail"
    )]

    use super::{AQSA_TUL_MULAHAZA, LubaId, MulahazaTabaqa, ThiqatQira, iqtata};
    use taarib_mustalahat::luba::MasdarLuba;

    fn luba() -> LubaId {
        // Hollow Knight, as this machine's Steam library identifies it.
        LubaId::min_masdar(&MasdarLuba::Steam(367_520), "Hollow Knight")
    }

    fn mulahaza(asl: &str, arabi: &str) -> MulahazaTabaqa {
        MulahazaTabaqa::jadeeda(asl, arabi, luba(), ThiqatQira::maqisa(90))
    }

    /// What a recognizer really returns between the lines that matter, and
    /// which of it is worth a row in a memory somebody may one day share.
    #[test]
    fn ma_yustahaqq_hifzuhu() {
        assert!(mulahaza("Save and Quit", "احفظ واخرج").salih());
        assert!(mulahaza("Are you sure?", "هل أنت متأكد؟").salih());

        // A health bar, a frame counter, a row of decoration: no letters.
        assert!(!mulahaza("100 / 100", "١٠٠ / ١٠٠").salih());
        assert!(!mulahaza("59.9", "٥٩٫٩").salih());
        assert!(!mulahaza("- - -", "- - -").salih());
        // One character is an icon far more often than it is a word.
        assert!(!mulahaza("E", "إي").salih());
        // Nothing to store on either side.
        assert!(!mulahaza("   ", "فراغ").salih());
        assert!(!mulahaza("Continue", "  ").salih());
    }

    /// A reading longer than the ceiling is cut on a character boundary, not
    /// a byte one — the difference between a shortened line and a panic.
    #[test]
    fn alqat_bil_ahruf_la_bil_bayt() {
        let taweel: String = "الطريق ".repeat(600);
        assert!(taweel.chars().count() > AQSA_TUL_MULAHAZA);
        let maqtu = iqtata(&taweel);
        assert_eq!(maqtu.chars().count(), AQSA_TUL_MULAHAZA);
        assert!(taweel.starts_with(&maqtu));
    }

    /// The shared handle really is shareable, which is the whole reason it
    /// exists — the overlay's seam takes `Send + Sync` and a `SQLite`
    /// connection is not `Sync` on its own.
    #[test]
    fn almushtaraka_tuqbal_bayn_alkhuyut() {
        const fn yaqbal<T: Send + Sync>() {}
        yaqbal::<super::DhakiraMushtaraka>();
    }

    /// An unmeasured reading is carried as unmeasured all the way into the
    /// record, and never as the stand-in number the overlay's panel shows.
    #[test]
    fn ghayr_al_maqisa_tabqa_ghayr_maqisa() {
        let qayd =
            MulahazaTabaqa::jadeeda("Loading", "جارٍ التحميل", luba(), ThiqatQira::Ghayr)
                .ila_qayd();
        assert_eq!(qayd.asl.thiqat_qira(), ThiqatQira::Ghayr);
        assert_eq!(qayd.asl.thiqat_qira().mia(), None);
        assert!(!qayd.asl.thiqat_qira().yajtaz(0));
    }
}
