//! التتبّع — one line of dialogue's identity across frames, and the moment its
//! text stops moving.
//!
//! Everything before this module is per-frame. [`crate::qira`] reads a
//! rectangle and returns strings; it has no memory, so the same sentence read on
//! two hundred consecutive frames is two hundred unrelated strings as far as
//! anything downstream can tell. That is survivable for drawing — the same
//! Arabic is shaped twice and looks identical — and it is not survivable for
//! anything that *costs* something. A translation request is paid for per
//! string, so a subtitle with no identity is a subtitle billed two hundred
//! times, and a reading history with no identity is two hundred rows saying the
//! same thing.
//!
//! So this module gives a line an identity that survives across frames while it
//! is on screen and ends when the line genuinely changes. Everything expensive
//! hangs off that identity: one translation, one history row, one context entry.
//!
//! ## Growth is not change
//!
//! A great many games reveal dialogue a character at a time. To a recognizer
//! polled four times a second, one sentence being typed out is four different
//! strings — `"The old"`, `"The old road nor"`, `"The old road north is"`,
//! `"The old road north is closed."` — and a pipeline that translated each of
//! them would pay four times to produce three translations of things nobody
//! said and one of the thing they did.
//!
//! The rule that fixes it is simple and is the reason [`tashabuh`] is not the
//! only test: when the previous reading is a **prefix** of the new one, the line
//! is the same line and it is still being revealed. Growth updates the text,
//! extends the identity, and translates nothing. Only when the text stops moving
//! — [`SiyasatIstiqrar::marrat_sukun`] consecutive readings identical, *and*
//! [`SiyasatIstiqrar::mudat_sukun_mikro`] elapsed since the last change — does
//! the line settle, and settling is what asks for exactly one translation.
//!
//! Both conditions rather than either. The count alone is wrong for a caller
//! polling faster than a reveal advances; the elapsed time alone is wrong for a
//! caller polling so slowly that a whole reveal fits between two readings.
//!
//! ## Why matching is not equality
//!
//! Recognition is not stable to the character. The same unchanged dialogue box
//! read twice can come back with a comma that turned into a full stop, because
//! the panel behind it has a particle effect and one glyph's anti-aliasing moved.
//! Matching only on equality would call that a new line, end the old one, and
//! buy a second translation of a sentence already on screen.
//!
//! So an observation is matched to a live line by a score, not by equality:
//! [`tashabuh`] on the text and vertical overlap on the box, with prefix growth
//! short-circuiting to a certain match. The score is deliberately weighted
//! toward the text — a paragraph that reflows moves every box while the words
//! stay put, and the words are the thing being translated.
//!
//! ## Ending, and the two reasons a line disappears
//!
//! A live line that no observation matched has either been replaced or been
//! missed. The two are told apart by whether the region produced anything at
//! all on that pass: a region that returned other lines and not this one has
//! genuinely moved on, and the line ends immediately; a region that returned
//! nothing may simply have blinked — a fade, a frame the recognizer read
//! nothing out of — and the line is given [`SiyasatIstiqrar::sabr_ghiyab`]
//! passes before it ends. Treating both the same way either leaves a stale line
//! drawn over the next speaker's dialogue or drops a line the player is still
//! reading.
//!
//! ## The context ring
//!
//! A settled line is pushed onto a small per-region ring of what was said
//! before it. That ring is what [`crate::qissa`] hands the translator as
//! surrounding context, and it is why it lives here rather than in the session:
//! the only moment at which a line is known to be final *and* known to be the
//! most recent final one is the moment it settles, and that moment is this
//! module's.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;

use crate::manatiq::MuarrifMintaqa;
use crate::qira::SatrMaqru;
use crate::wajiha::MustatilBiksel;

/// The longest reading this module compares character by character.
///
/// Recognition over a region that holds a texture rather than text produces
/// nonsense of unbounded length, and [`tashabuh`] walks both strings. The cap
/// matches [`crate::sijill_qira::AQSA_TUL_SATR`], which is where such a reading
/// is truncated on its way into the history, so the two agree about what a line
/// is rather than disagreeing by a factor nobody would find.
pub const AQSA_TUL_MUQARANA: usize = 2_000;

/// How many settled lines of context one region remembers.
///
/// Four. The next line's translation is improved most by the line immediately
/// before it, materially by the one before that, and hardly at all beyond;
/// meanwhile every entry is tokens in a prompt that is paid for per request.
/// Four is two exchanges of dialogue, which is the unit a translator needs to
/// get a pronoun right.
pub const SAA_SIYAQ: usize = 4;

/// A tracked line's identity.
///
/// Monotonic within one tracker and never reused, so a translation that comes
/// back after its line has ended can be discarded by identity rather than by
/// comparing text against a line that may since have been replaced by an
/// identical one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MuarrifSatr(u64);

impl MuarrifSatr {
    /// The underlying number, for a log line and a diagnostics bundle.
    #[must_use]
    pub const fn raqm(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for MuarrifSatr {
    fn fmt(&self, mukhraj: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(mukhraj, "s{}", self.0)
    }
}

/// What a tracked line's text did on the most recent pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatSatr {
    /// The previous reading is a prefix of this one: a reveal in progress.
    ///
    /// The state that must never be translated. See this module's header.
    Yanmu,

    /// The text changed in a way that is not growth — a correction, a reflow,
    /// or the first reading of a brand-new line.
    Yataghayyar,

    /// The text has been still long enough to be treated as final.
    ///
    /// The only state that asks for a translation, and it asks exactly once.
    Mustaqirr,
}

impl HalatSatr {
    /// Whether a line in this state is worth spending a translation on.
    #[must_use]
    pub const fn yastahiq_tarjama(self) -> bool {
        matches!(self, Self::Mustaqirr)
    }

    /// The name that appears in the control panel and the log.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Yanmu => "revealing",
            Self::Yataghayyar => "changing",
            Self::Mustaqirr => "settled",
        }
    }
}

/// One reading, with its box already moved onto the surface.
///
/// The conversion is the caller's, made once, exactly as
/// [`crate::talqeem::SatrMulaqqam::min_maqru`] requires it: [`SatrMaqru::mawdi`]
/// is in the captured image's coordinates and using it unconverted is the one
/// mistake this signature exists to make impossible to write by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QiraaMulahaza {
    /// What the recognizer read.
    pub nass: String,
    /// Where it read it, in surface pixels.
    pub mawdi: MustatilBiksel,
    /// The recognizer's confidence, zero to a hundred.
    pub thiqa: u8,
    /// Whether [`QiraaMulahaza::thiqa`] is the engine's own number.
    ///
    /// `false` means it is [`crate::qira::THIQA_GHAYR_MAQISA`], a constant
    /// rather than a measurement. Carried the whole way down because everything
    /// that eventually reads the number — the panel's confidence band, the
    /// reading history, the shared translation memory's decision about whether
    /// an accumulation may be shared — is wrong if it treats the stand-in as an
    /// observation.
    pub maqisa: bool,
}

impl QiraaMulahaza {
    /// An observation from a recognized line and its box on the surface.
    #[must_use]
    pub fn min_maqru(maqru: &SatrMaqru, mawdi: MustatilBiksel) -> Self {
        Self {
            nass: maqru.nass.clone(),
            mawdi,
            thiqa: maqru.thiqa,
            maqisa: maqru.maqisa,
        }
    }
}

/// One line the tracker is following.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrMutatabba {
    /// Its identity.
    pub muarrif: MuarrifSatr,
    /// Which region it came from.
    pub mintaqa: MuarrifMintaqa,
    /// The most recent reading, whitespace-normalized.
    pub nass: String,
    /// Where the most recent reading was, in surface pixels.
    pub mawdi: MustatilBiksel,
    /// The best confidence any reading of this line carried.
    ///
    /// The best rather than the latest: a fade-in finishing or a background
    /// settling makes a later reading more certain, and the number the history
    /// and the panel want is how well this line was ever read.
    pub thiqa: u8,
    /// Whether [`SatrMutatabba::thiqa`] is a measurement.
    ///
    /// The **conjunction** across every reading matched to this line, which is
    /// the rule [`SatrMaqru::mudmaj`] applies for the same reason: a line is
    /// only as measured as its least measured part, and reporting a merged
    /// line as measured because one of its readings was would attach a number
    /// to text that number never covered.
    pub maqisa: bool,
    /// What the text did on the most recent pass.
    pub hala: HalatSatr,
    /// When the line was first seen, on the caller's microsecond counter.
    pub awwal_mikro: u64,
    /// When its text last changed.
    pub akhir_taghyeer_mikro: u64,
    /// When it was last matched by an observation.
    pub akhir_ruya_mikro: u64,
    /// How many readings have been matched to it.
    pub marrat: u32,
    /// How many consecutive readings have been identical.
    pub thabat: u32,
    /// How many consecutive passes have gone by without matching it.
    pub ghiyab: u32,
    /// Whether the settled text has already been handed out for translation.
    ///
    /// Cleared whenever the text changes, because a line that grew past the text
    /// it settled at is a different sentence and the old translation is a
    /// translation of half of it.
    pub ursilat: bool,
}

impl SatrMutatabba {
    /// How long this line has been on screen, in microseconds.
    #[must_use]
    pub const fn mudda_mikro(&self) -> u64 {
        self.akhir_ruya_mikro.saturating_sub(self.awwal_mikro)
    }

    /// Whether this line is settled and has not yet been translated.
    #[must_use]
    pub const fn yantazir_tarjama(&self) -> bool {
        self.hala.yastahiq_tarjama() && !self.ursilat
    }

    /// The one-line form a log line and the diagnostics bundle carry.
    #[must_use]
    pub fn satr(&self) -> String {
        let mut satr = String::new();
        let _ = write!(
            satr,
            "{} [{}] {} ×{} — {}",
            self.muarrif,
            self.mintaqa,
            self.hala.ism(),
            self.marrat,
            self.nass
        );
        satr
    }
}

/// A line that has left the screen, and what it finally said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SatrMuntahi {
    /// Its identity, so a caller holding one can drop what it was keeping.
    pub muarrif: MuarrifSatr,
    /// Which region it came from.
    pub mintaqa: MuarrifMintaqa,
    /// The last reading.
    pub nass: String,
    /// Whether it ever settled.
    ///
    /// A line that never settled was never translated, and a caller reconciling
    /// its own state needs to tell "the translation is stale" from "there was
    /// never one".
    pub istaqarr: bool,
}

/// When a reading is the same line, and when a line is done moving.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SiyasatIstiqrar {
    /// How many consecutive identical readings mean the text has stopped.
    ///
    /// One, and one is a real choice rather than the smallest number that
    /// works. A typewriter reveal at its fastest advances every sixty to eighty
    /// milliseconds; two readings a quarter of a second apart that agree
    /// character for character did not happen in the middle of one. Requiring
    /// two would cost a further poll interval of latency on every line in the
    /// game to defend against a reveal speed no engine uses.
    pub marrat_sukun: u32,

    /// How long the text must have been still, in microseconds.
    ///
    /// The guard on [`SiyasatIstiqrar::marrat_sukun`] for a caller polling fast.
    /// Two hundred milliseconds is under one poll at the default interval, so it
    /// binds only when somebody has lowered that interval — which is exactly the
    /// case where a repeat count alone would settle mid-reveal.
    pub mudat_sukun_mikro: u64,

    /// How many passes a line survives without being matched, when the region
    /// produced nothing at all.
    ///
    /// A region that produced *other* lines and not this one has moved on, and
    /// that ends the line regardless of this number. See the module header.
    pub sabr_ghiyab: u32,

    /// The score above which an observation is the same line.
    ///
    /// Below it the observation starts a new identity and the old line ends.
    pub adna_darja: f32,

    /// The shortest previous reading that may be treated as a prefix.
    ///
    /// Every string starts with the empty string and almost every English
    /// sentence starts with `"T"`. Without a floor, a one-character misread
    /// would adopt the next line of dialogue as a continuation of itself.
    pub adna_bidaya: usize,

    /// How many settled lines of context one region keeps.
    pub saa_siyaq: usize,
}

impl SiyasatIstiqrar {
    /// The defaults, tuned against the default poll interval of
    /// [`crate::iltiqat_shasha::MuqayyidMuadal::FASIL_IFTIRADI_MIKRO`].
    #[must_use]
    pub const fn iftiradiya() -> Self {
        Self {
            marrat_sukun: 1,
            mudat_sukun_mikro: 200_000,
            sabr_ghiyab: 1,
            // Deliberately strict, and the direction of the error is the
            // reason. Merging two lines that are not the same line loses the
            // second one: it inherits an identity that has already been
            // translated, and the player is shown Arabic for the sentence
            // before it. Splitting one line into two costs one extra request
            // and loses nothing. So the threshold is set where a *contiguous*
            // difference of about one part in fourteen still matches — which
            // covers a comma read as a full stop, or a clipped first letter —
            // and where a line that differs by a whole word does not. Two
            // lines whose only difference is a counter ticking over score
            // just under it, and are correctly treated as two lines.
            adna_darja: 0.93,
            adna_bidaya: 3,
            saa_siyaq: SAA_SIYAQ,
        }
    }
}

impl Default for SiyasatIstiqrar {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

/// What one pass of the tracker did.
///
/// Returned rather than left for a caller to diff against the previous state,
/// because "which lines settled on *this* pass" is the question the whole module
/// exists to answer and recomputing it outside would be recomputing it wrongly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HasilatDawra {
    /// Identities created on this pass.
    pub jadeeda: Vec<MuarrifSatr>,
    /// Identities that settled on this pass and have not been translated.
    ///
    /// One translation each, and this is the list that is paid for.
    pub mustaqirra: Vec<MuarrifSatr>,
    /// Identities still being revealed, which are deliberately not translated.
    pub namiya: Vec<MuarrifSatr>,
    /// Lines that ended on this pass.
    pub muntahiya: Vec<SatrMuntahi>,
}

impl HasilatDawra {
    /// Whether this pass changed nothing a caller has to act on.
    #[must_use]
    pub const fn hadia(&self) -> bool {
        self.jadeeda.is_empty()
            && self.mustaqirra.is_empty()
            && self.namiya.is_empty()
            && self.muntahiya.is_empty()
    }
}

/// What the tracker has done, for the control panel and the report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IhsaatTatabbu {
    /// Readings fed in.
    pub mulahazat: u64,
    /// Identities created.
    pub hawiyat: u64,
    /// Readings matched to a line that already existed.
    pub tatabuqat: u64,
    /// Readings that extended a reveal rather than starting anything.
    pub numuwwat: u64,
    /// Lines that settled — the count of translations this module asked for.
    pub istiqrarat: u64,
    /// Lines that ended.
    pub inhaat: u64,
}

impl IhsaatTatabbu {
    /// How many translations were avoided by tracking, over this session.
    ///
    /// Every reading that matched a line that already existed is a translation
    /// request that a per-frame pipeline would have made and this one did not.
    /// It is the honest form of the number: not "we are fast", but "this many
    /// requests were not sent, and here is the count of the ones that were".
    #[must_use]
    pub const fn muwaffara(&self) -> u64 {
        self.mulahazat.saturating_sub(self.istiqrarat)
    }

    /// The sentence the control panel displays.
    ///
    /// Stated the way [`crate::wajiha::MeezaniyatItar::wasf`] states the frame
    /// cost: the real numbers, the unflattering ones included.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} reading(s) over {} line(s); {} matched an existing line, {} extended a reveal, \
             {} settled and were translated, {} ended",
            self.mulahazat,
            self.hawiyat,
            self.tatabuqat,
            self.numuwwat,
            self.istiqrarat,
            self.inhaat
        )
    }
}

/// The tracker: live lines per region, a context ring per region, and the
/// policy that decides when a reading is the same line.
#[derive(Debug, Clone)]
pub struct Mutatabbi {
    siyasa: SiyasatIstiqrar,
    hayya: BTreeMap<MuarrifMintaqa, Vec<SatrMutatabba>>,
    siyaq: BTreeMap<MuarrifMintaqa, VecDeque<String>>,
    talee: u64,
    ihsaat: IhsaatTatabbu,
}

impl Mutatabbi {
    /// A tracker with the default policy.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::bi_siyasa(SiyasatIstiqrar::iftiradiya())
    }

    /// A tracker with an explicit policy.
    #[must_use]
    pub fn bi_siyasa(siyasa: SiyasatIstiqrar) -> Self {
        Self {
            siyasa,
            hayya: BTreeMap::new(),
            siyaq: BTreeMap::new(),
            talee: 1,
            ihsaat: IhsaatTatabbu::default(),
        }
    }

    /// The policy in force.
    #[must_use]
    pub const fn siyasa(&self) -> &SiyasatIstiqrar {
        &self.siyasa
    }

    /// Changes the policy.
    pub const fn ayyin_siyasa(&mut self, siyasa: SiyasatIstiqrar) {
        self.siyasa = siyasa;
    }

    /// What the tracker has done.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatTatabbu {
        &self.ihsaat
    }

    /// Every live line, in region order then in the order they were seen.
    pub fn hayya(&self) -> impl Iterator<Item = &SatrMutatabba> {
        self.hayya.values().flat_map(|sutur| sutur.iter())
    }

    /// The live lines of one region.
    #[must_use]
    pub fn hayyat_mintaqa(&self, mintaqa: MuarrifMintaqa) -> &[SatrMutatabba] {
        self.hayya.get(&mintaqa).map_or(&[], Vec::as_slice)
    }

    /// One live line by identity.
    #[must_use]
    pub fn satr(&self, muarrif: MuarrifSatr) -> Option<&SatrMutatabba> {
        self.hayya().find(|satr| satr.muarrif == muarrif)
    }

    /// Marks a settled line as having had its translation asked for.
    ///
    /// Returns whether the line was still live. Called by the session the moment
    /// it dispatches, not when the translation comes back: the gap between the
    /// two is several poll intervals, and a line that could be dispatched twice
    /// inside it is a line that costs twice.
    pub fn sajjil_irsal(&mut self, muarrif: MuarrifSatr) -> bool {
        for sutur in self.hayya.values_mut() {
            if let Some(satr) = sutur.iter_mut().find(|satr| satr.muarrif == muarrif) {
                satr.ursilat = true;
                return true;
            }
        }
        false
    }

    /// What was said in this region before the line being translated, oldest
    /// first.
    ///
    /// The surrounding context a translator is given. Only settled lines are in
    /// it: a partial reading of a reveal is not something anybody said, and
    /// putting one in the context would make the next line's translation
    /// continue a sentence that was never finished.
    #[must_use]
    pub fn siyaq(&self, mintaqa: MuarrifMintaqa) -> Vec<String> {
        self.siyaq
            .get(&mintaqa)
            .map(|halqa| halqa.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The same context, without the line that is about to be translated.
    ///
    /// A line joins the ring on the pass it settles, which is the same pass its
    /// translation is asked for — so the plain [`Mutatabbi::siyaq`] would hand a
    /// provider the very sentence it is being asked to translate, labelled as
    /// something said before it. Only a *trailing* match is dropped, and only
    /// one, because that is exactly where a just-settled line is and a genuine
    /// repetition further back is something the speaker really did say twice.
    #[must_use]
    pub fn siyaq_duna(&self, mintaqa: MuarrifMintaqa, nass: &str) -> Vec<String> {
        let mut jiwar = self.siyaq(mintaqa);
        if jiwar.last().is_some_and(|akhir| akhir == nass) {
            let _ = jiwar.pop();
        }
        jiwar
    }

    /// Forgets one region's lines and its context.
    ///
    /// Called when the region moves or is deleted. The identity counter is kept
    /// so a translation still in flight for an old line cannot be delivered to a
    /// new one that reused its number.
    pub fn ansa_mintaqa(&mut self, mintaqa: MuarrifMintaqa) {
        let _ = self.hayya.remove(&mintaqa);
        let _ = self.siyaq.remove(&mintaqa);
    }

    /// Forgets every line and every region's context.
    ///
    /// For a level transition or a surface change, where every rectangle on
    /// screen is about to be somewhere else.
    pub fn amsah(&mut self) {
        self.hayya.clear();
        self.siyaq.clear();
    }

    /// Feeds one region's recognition pass in, and reports what it did.
    ///
    /// `lahza_mikro` is the caller's microsecond counter. This crate reads no
    /// clock — it runs inside a game's process and the caller is already timing
    /// frames for [`crate::wajiha::MeezaniyatItar`].
    ///
    /// Passing an empty slice is meaningful and is not a no-op: it is how the
    /// tracker learns that a region the recognizer looked at held nothing, which
    /// is what eventually ends the line that was there.
    pub fn sajjil(
        &mut self,
        mintaqa: MuarrifMintaqa,
        lahza_mikro: u64,
        mulahazat: &[QiraaMulahaza],
    ) -> HasilatDawra {
        let munaqqaha: Vec<QiraaMulahaza> = mulahazat
            .iter()
            .filter_map(|mulahaza| {
                let nass = wahhid(&mulahaza.nass);
                if nass.is_empty() {
                    return None;
                }
                Some(QiraaMulahaza {
                    nass,
                    mawdi: mulahaza.mawdi,
                    thiqa: mulahaza.thiqa,
                    maqisa: mulahaza.maqisa,
                })
            })
            .collect();
        self.ihsaat.mulahazat = self
            .ihsaat
            .mulahazat
            .saturating_add(u64::try_from(munaqqaha.len()).unwrap_or(u64::MAX));

        let mut hasila = HasilatDawra::default();
        let sutur = self.hayya.entry(mintaqa).or_default();
        let mut mutabaqa = vec![false; munaqqaha.len()];
        let mut mumassa = vec![false; sutur.len()];

        // Greedy over the whole score matrix rather than first-fit over the
        // observations. First-fit gives the first observation whichever line it
        // matches at all, which on a reflowed paragraph hands line two's text to
        // line one and then has nothing left that fits line two.
        loop {
            let mut afdal: Option<(usize, usize, f32)> = None;
            for (fahras_satr, satr) in sutur.iter().enumerate() {
                if mumassa.get(fahras_satr).copied().unwrap_or(true) {
                    continue;
                }
                for (fahras_mulahaza, mulahaza) in munaqqaha.iter().enumerate() {
                    if mutabaqa.get(fahras_mulahaza).copied().unwrap_or(true) {
                        continue;
                    }
                    let darja = darjat_tatabuq(satr, mulahaza, self.siyasa.adna_bidaya);
                    if darja < self.siyasa.adna_darja {
                        continue;
                    }
                    if afdal.is_none_or(|(_, _, sabiqa)| darja > sabiqa) {
                        afdal = Some((fahras_satr, fahras_mulahaza, darja));
                    }
                }
            }
            let Some((fahras_satr, fahras_mulahaza, _)) = afdal else {
                break;
            };
            if let Some(khana) = mumassa.get_mut(fahras_satr) {
                *khana = true;
            }
            if let Some(khana) = mutabaqa.get_mut(fahras_mulahaza) {
                *khana = true;
            }
            let (Some(satr), Some(mulahaza)) =
                (sutur.get_mut(fahras_satr), munaqqaha.get(fahras_mulahaza))
            else {
                break;
            };
            match damm(satr, mulahaza, lahza_mikro, self.siyasa.adna_bidaya) {
                AtharDamm::Nama => self.ihsaat.numuwwat = self.ihsaat.numuwwat.saturating_add(1),
                AtharDamm::Thabat | AtharDamm::Taghayyar => {},
            }
            self.ihsaat.tatabuqat = self.ihsaat.tatabuqat.saturating_add(1);
        }

        // Unmatched observations are new lines.
        for (fahras, mulahaza) in munaqqaha.iter().enumerate() {
            if mutabaqa.get(fahras).copied().unwrap_or(true) {
                continue;
            }
            let muarrif = MuarrifSatr(self.talee);
            self.talee = self.talee.saturating_add(1);
            self.ihsaat.hawiyat = self.ihsaat.hawiyat.saturating_add(1);
            sutur.push(SatrMutatabba {
                muarrif,
                mintaqa,
                nass: mulahaza.nass.clone(),
                mawdi: mulahaza.mawdi,
                thiqa: mulahaza.thiqa,
                maqisa: mulahaza.maqisa,
                hala: HalatSatr::Yataghayyar,
                awwal_mikro: lahza_mikro,
                akhir_taghyeer_mikro: lahza_mikro,
                akhir_ruya_mikro: lahza_mikro,
                marrat: 1,
                thabat: 0,
                ghiyab: 0,
                ursilat: false,
            });
            hasila.jadeeda.push(muarrif);
        }

        // Unmatched lines age. A region that returned something and not this
        // line has moved on; a region that returned nothing may have blinked.
        let ra_shayan = !munaqqaha.is_empty();
        let sabr = self.siyasa.sabr_ghiyab;
        for (fahras, satr) in sutur.iter_mut().enumerate() {
            // Beyond the length `mumassa` was built at are the lines created a
            // few statements ago, which were seen on this very pass by
            // construction. Defaulting an unknown index to "not seen" would age
            // every brand-new line on the pass that created it, and — because a
            // region that produced something ends an unseen line at once —
            // delete it before it could ever settle.
            if mumassa.get(fahras).copied().unwrap_or(true) {
                continue;
            }
            satr.ghiyab = satr.ghiyab.saturating_add(1);
        }
        sutur.retain(|satr| {
            let intaha = satr.ghiyab > 0 && (ra_shayan || satr.ghiyab > sabr);
            if intaha {
                hasila.muntahiya.push(SatrMuntahi {
                    muarrif: satr.muarrif,
                    mintaqa: satr.mintaqa,
                    nass: satr.nass.clone(),
                    istaqarr: satr.hala.yastahiq_tarjama() || satr.ursilat,
                });
            }
            !intaha
        });
        self.ihsaat.inhaat = self
            .ihsaat
            .inhaat
            .saturating_add(u64::try_from(hasila.muntahiya.len()).unwrap_or(u64::MAX));

        // Settling, last, so a line created on this pass is judged by the same
        // rule as one that has been on screen for a minute.
        let marrat_sukun = self.siyasa.marrat_sukun;
        let mudat_sukun = self.siyasa.mudat_sukun_mikro;
        let mut istaqarrat: Vec<(MuarrifMintaqa, String)> = Vec::new();
        for satr in sutur.iter_mut() {
            if satr.hala.yastahiq_tarjama() {
                if !satr.ursilat {
                    hasila.mustaqirra.push(satr.muarrif);
                }
                continue;
            }
            // Stillness is judged from the clock and the repeat count, never
            // from what the *previous* pass called the line. A reveal that has
            // finished looks exactly like any other unchanged line, and reading
            // the last pass's [`HalatSatr::Yanmu`] as "still growing" is how a
            // typed-out sentence would sit on screen forever, complete and
            // never translated.
            let sakan = satr.thabat >= marrat_sukun
                && lahza_mikro.saturating_sub(satr.akhir_taghyeer_mikro) >= mudat_sukun;
            if sakan {
                satr.hala = HalatSatr::Mustaqirr;
                hasila.mustaqirra.push(satr.muarrif);
                istaqarrat.push((satr.mintaqa, satr.nass.clone()));
            } else if matches!(satr.hala, HalatSatr::Yanmu) {
                hasila.namiya.push(satr.muarrif);
            }
        }
        self.ihsaat.istiqrarat = self
            .ihsaat
            .istiqrarat
            .saturating_add(u64::try_from(istaqarrat.len()).unwrap_or(u64::MAX));

        let saa = self.siyasa.saa_siyaq;
        for (mintaqa_satr, nass) in istaqarrat {
            self.adhif_siyaq(mintaqa_satr, nass, saa);
        }
        hasila
    }

    /// Records that a region's picture has not changed since the last pass.
    ///
    /// The perceptual-hash gate in [`crate::iltiqat_shasha::MuqayyidMuadal`]
    /// answers the exact question the settle policy asks — has the text stopped
    /// moving — and answers it for the price of one preprocessing pass instead
    /// of a recognition, which is two orders of magnitude cheaper. So a capture
    /// the gate held back is **not** a pass that did not happen: it is a pass
    /// that confirmed every live line in the region without reading any of them
    /// again, and this is where that confirmation lands.
    ///
    /// Feeding it through [`Mutatabbi::sajjil`] with the previous readings
    /// re-presented would be the alternative, and it would be wrong twice: the
    /// caller would have to keep a copy of what it last recognized, and the
    /// tracker would run a matching pass to rediscover an answer the gate had
    /// already given.
    pub fn thabbit(&mut self, mintaqa: MuarrifMintaqa, lahza_mikro: u64) -> HasilatDawra {
        let mut hasila = HasilatDawra::default();
        let marrat_sukun = self.siyasa.marrat_sukun;
        let mudat_sukun = self.siyasa.mudat_sukun_mikro;
        let saa = self.siyasa.saa_siyaq;
        let Some(sutur) = self.hayya.get_mut(&mintaqa) else {
            return hasila;
        };

        let mut istaqarrat: Vec<(MuarrifMintaqa, String)> = Vec::new();
        for satr in sutur.iter_mut() {
            satr.ghiyab = 0;
            satr.akhir_ruya_mikro = lahza_mikro;
            satr.marrat = satr.marrat.saturating_add(1);
            if satr.hala.yastahiq_tarjama() {
                if !satr.ursilat {
                    hasila.mustaqirra.push(satr.muarrif);
                }
                continue;
            }
            satr.thabat = satr.thabat.saturating_add(1);
            let sakan = satr.thabat >= marrat_sukun
                && lahza_mikro.saturating_sub(satr.akhir_taghyeer_mikro) >= mudat_sukun;
            if sakan {
                satr.hala = HalatSatr::Mustaqirr;
                hasila.mustaqirra.push(satr.muarrif);
                istaqarrat.push((satr.mintaqa, satr.nass.clone()));
            } else if matches!(satr.hala, HalatSatr::Yanmu) {
                hasila.namiya.push(satr.muarrif);
            }
        }
        self.ihsaat.tatabuqat = self
            .ihsaat
            .tatabuqat
            .saturating_add(u64::try_from(sutur.len()).unwrap_or(u64::MAX));
        self.ihsaat.istiqrarat = self
            .ihsaat
            .istiqrarat
            .saturating_add(u64::try_from(istaqarrat.len()).unwrap_or(u64::MAX));
        for (mintaqa_satr, nass) in istaqarrat {
            self.adhif_siyaq(mintaqa_satr, nass, saa);
        }
        hasila
    }

    /// Pushes a settled line onto its region's context ring.
    ///
    /// A repeat of the ring's last entry is not pushed. A line that settled,
    /// jittered by one character and settled again is one thing that was said,
    /// and two copies of it in the context would tell the translator the speaker
    /// said it twice.
    fn adhif_siyaq(&mut self, mintaqa: MuarrifMintaqa, nass: String, saa: usize) {
        if saa == 0 {
            return;
        }
        let halqa = self.siyaq.entry(mintaqa).or_default();
        if halqa.back().is_some_and(|akhir| *akhir == nass) {
            return;
        }
        halqa.push_back(nass);
        while halqa.len() > saa {
            let _ = halqa.pop_front();
        }
    }
}

impl Default for Mutatabbi {
    fn default() -> Self {
        Self::jadeed()
    }
}

/// What absorbing an observation into a line did to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtharDamm {
    /// The text is unchanged.
    Thabat,
    /// The previous text is a prefix of the new one.
    Nama,
    /// The text changed some other way.
    Taghayyar,
}

/// Absorbs an observation into the line it matched.
fn damm(
    satr: &mut SatrMutatabba,
    mulahaza: &QiraaMulahaza,
    lahza_mikro: u64,
    adna_bidaya: usize,
) -> AtharDamm {
    satr.marrat = satr.marrat.saturating_add(1);
    satr.ghiyab = 0;
    satr.akhir_ruya_mikro = lahza_mikro;
    satr.mawdi = mulahaza.mawdi;
    // A measured reading replaces an unmeasured line's stand-in outright; two
    // readings of the same kind compete on the number. Taking `max` across the
    // two kinds would let the constant 80 outrank a genuine 62.
    if mulahaza.maqisa && !satr.maqisa {
        satr.thiqa = mulahaza.thiqa;
    } else if mulahaza.maqisa == satr.maqisa {
        satr.thiqa = satr.thiqa.max(mulahaza.thiqa);
    }
    satr.maqisa = satr.maqisa && mulahaza.maqisa;

    if satr.nass == mulahaza.nass {
        satr.thabat = satr.thabat.saturating_add(1);
        return AtharDamm::Thabat;
    }

    let nama = yanmu(&satr.nass, &mulahaza.nass, adna_bidaya);
    satr.nass.clone_from(&mulahaza.nass);
    satr.akhir_taghyeer_mikro = lahza_mikro;
    satr.thabat = 0;
    // Cleared on every change, growth included. A line that settled, was
    // translated and then grew is a longer sentence, and the Arabic already in
    // hand is a translation of the first half of it.
    satr.ursilat = false;
    satr.hala = if nama {
        HalatSatr::Yanmu
    } else {
        HalatSatr::Yataghayyar
    };
    if nama {
        AtharDamm::Nama
    } else {
        AtharDamm::Taghayyar
    }
}

/// How well an observation matches a live line, zero to one.
///
/// Prefix growth short-circuits to one: a reveal in progress is the same line
/// with certainty, and letting the score decide would end the line the moment
/// the reveal had added more than a sixth of its final length in one poll.
fn darjat_tatabuq(satr: &SatrMutatabba, mulahaza: &QiraaMulahaza, adna_bidaya: usize) -> f32 {
    if yanmu(&satr.nass, &mulahaza.nass, adna_bidaya) {
        return 1.0;
    }
    // Weighted toward the text. A paragraph that reflows moves every box while
    // the words stay put, and the words are what is being translated; a box that
    // moved by its own height with the same text in it is the same line.
    tashabuh(&satr.nass, &mulahaza.nass)
        .mul_add(0.75, tadakhul_amudi(satr.mawdi, mulahaza.mawdi) * 0.25)
}

/// Whether the second reading is the first one still being revealed.
///
/// A strict prefix, and only above [`SiyasatIstiqrar::adna_bidaya`] characters.
/// The floor is what stops a one-character misread from adopting the next line
/// of dialogue as its own continuation.
#[must_use]
pub fn yanmu(qadeem: &str, jadeed: &str, adna_bidaya: usize) -> bool {
    if qadeem.chars().count() < adna_bidaya {
        return false;
    }
    jadeed.len() > qadeem.len() && jadeed.starts_with(qadeem)
}

/// How alike two readings are, zero to one.
///
/// The shared prefix plus the shared suffix, over the longer string's length.
/// Two properties make it the right measure here and an edit distance the wrong
/// one. It is linear rather than quadratic, which matters because it runs on a
/// worker competing with a game for a core and a region full of texture produces
/// readings thousands of characters long. And it scores exactly the two things
/// recognition actually does: it appends at the end — a reveal — and it wobbles
/// one glyph in the middle — a comma that read as a full stop. An edit distance
/// scores those the same as a completely different sentence with the same length.
///
/// **It measures one contiguous difference, and that is a real limit.** A
/// reading that wobbles in two places at once scores as though everything
/// between them had changed, so it falls below the threshold and starts a new
/// identity. The consequence is one extra translation request for that reading,
/// which is the failure this whole module chooses: the alternative measure that
/// tolerates scattered differences also tolerates a counter ticking over inside
/// an otherwise identical line, and treating *that* as the same line means never
/// translating the changed value at all.
///
/// Both strings are compared as characters rather than bytes, so a multi-byte
/// codepoint counts once and the score does not depend on how the text is
/// encoded.
#[must_use]
pub fn tashabuh(awwal: &str, thani: &str) -> f32 {
    if awwal == thani {
        return 1.0;
    }
    let awwal: Vec<char> = awwal.chars().take(AQSA_TUL_MUQARANA).collect();
    let thani: Vec<char> = thani.chars().take(AQSA_TUL_MUQARANA).collect();
    let aqsar = awwal.len().min(thani.len());
    let atwal = awwal.len().max(thani.len());
    if atwal == 0 {
        return 1.0;
    }

    let bidaya = awwal
        .iter()
        .zip(thani.iter())
        .take_while(|(yasar, yameen)| yasar == yameen)
        .count();
    // Capped so the prefix and the suffix cannot count the same characters
    // twice, which on `"aaa"` against `"aaaa"` would otherwise report a shared
    // length of six over a longer string of four.
    let nihaya = awwal
        .iter()
        .rev()
        .zip(thani.iter().rev())
        .take_while(|(yasar, yameen)| yasar == yameen)
        .count()
        .min(aqsar.saturating_sub(bidaya));

    tul_f32(bidaya.saturating_add(nihaya)) / tul_f32(atwal)
}

/// How much of the shorter box's height two boxes share, zero to one.
///
/// The shorter box, for the reason [`SatrMaqru::tadakhul_amudi`] gives: a short
/// box entirely inside a tall one overlaps the tall one by a small fraction of
/// its height and by all of its own, and only the second number means "these are
/// the same line of text".
#[must_use]
pub fn tadakhul_amudi(awwal: MustatilBiksel, thani: MustatilBiksel) -> f32 {
    let aala = awwal.aala.max(thani.aala);
    let asfal = awwal
        .aala
        .saturating_add(awwal.irtifa)
        .min(thani.aala.saturating_add(thani.irtifa));
    let mushtarak = asfal.saturating_sub(aala);
    let aqsar = awwal.irtifa.min(thani.irtifa);
    if aqsar == 0 {
        return 0.0;
    }
    madaa_f32(mushtarak) / madaa_f32(aqsar)
}

/// A reading with its whitespace collapsed and its edges trimmed.
///
/// Every comparison in this module runs on the normalized form. Recognition
/// returns `"The  old   road"` and `"The old road"` for the same unchanged line
/// depending on how the detector split the word boxes, and two strings that
/// differ only in how many spaces are between two words are the same sentence
/// for every purpose this module has.
#[must_use]
pub fn wahhid(nass: &str) -> String {
    let mut munaqqah = String::with_capacity(nass.len());
    let mut fi_faragh = false;
    for harf in nass.trim().chars() {
        if harf.is_whitespace() {
            fi_faragh = true;
            continue;
        }
        if fi_faragh && !munaqqah.is_empty() {
            munaqqah.push(' ');
        }
        fi_faragh = false;
        munaqqah.push(harf);
    }
    munaqqah
}

/// A length as a float.
///
/// Capped at [`AQSA_TUL_MUQARANA`] by the caller, four orders of magnitude below
/// 2^24 where `usize` to `f32` stops being exact. Written once so the lint is
/// answered in one place rather than at three call sites.
const fn tul_f32(qeema: usize) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "compared lengths are capped at AQSA_TUL_MUQARANA, well below 2^24"
    )]
    {
        qeema as f32
    }
}

/// A pixel dimension as a float.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}
