//! القصّة — reading a story through the overlay, rather than showing text on it.
//!
//! Every other module in this crate does one step well. This one is the only
//! place they are joined, and joining them is where the difference between a
//! demo and a three-hour session lives.
//!
//! ## The budget rule, which governs everything here
//!
//! Recognition and translation are, respectively, two and four orders of
//! magnitude more expensive than drawing a quad batch. Neither may happen on the
//! presentation path. So the path is split in three and the split is the whole
//! architecture:
//!
//! 1. **On the frame**, and only on the frame: measure the last overlay cost,
//!    ask [`crate::watira::MunazzimWatira`] whether the refresh rate has to be
//!    given up, decide which regions are due, and read back their pixels. That
//!    is [`Munassiq`], and everything it does is arithmetic and a GPU copy.
//! 2. **Off the frame**: preprocess, gate on the perceptual hash, recognize,
//!    track, translate, remember, record. That is [`Qissa`], and in a real
//!    session it runs on the worker [`KhaytQissa`] owns.
//! 3. **On the frame again**: draw the most recent *completed* result. Not the
//!    result for this frame — there is no such thing — the most recent one that
//!    finished. [`Munassiq::laqta`] never blocks and never waits for the worker.
//!
//! When the per-frame budget is exceeded, what is given up is the **refresh
//! rate**, not frames. The overlay keeps drawing the Arabic it already has on
//! every frame, and goes back to read the screen less often. And it says so:
//! [`Munassiq::itar`] returns the moment the rate moved, and
//! [`KhulasatQissa::wasf`] carries it for as long as it is in force. An overlay
//! that quietly halves its own refresh rate is an overlay a player experiences
//! as a game that feels wrong for no reason they can name.
//!
//! ## What a three-hour session costs
//!
//! A dialogue-heavy session polled four times a second produces around forty
//! thousand recognition opportunities. Three gates stand between that number and
//! the number of translations paid for, and they are applied cheapest first:
//!
//! * the **clock**, in [`Munassiq`], which is the poll interval itself;
//! * the **picture**, [`crate::iltiqat_shasha::MuqayyidMuadal`]'s perceptual
//!   hash, which stops an unchanged region before recognition runs;
//! * the **line**, [`crate::tatabbu::Mutatabbi`], which gives a sentence one
//!   identity across every frame it is on screen and asks for one translation
//!   when it settles — never while it is still being typed out;
//! * and the **memory**, which answers a line this game has already produced
//!   without asking anybody.
//!
//! What is left is one request per distinct line of dialogue, which for a
//! three-hour session is on the order of a thousand. The first playthrough pays
//! for those; a second, with the shared memory attached, pays for none of them.
//!
//! ## Why the tracker and the history live on the worker
//!
//! Both are written by recognition and read by the panel, and the render thread
//! must never wait for either. The worker owns them outright and publishes a
//! *copy* of what to draw; the render thread reads that copy through a lock it
//! only ever tries, falling back to the previous frame's copy when the worker
//! holds it. One frame of slightly stale text is invisible. One frame of hitch
//! is not. This is the same trade
//! [`crate::sijill_qira::SijillMushtarak::laqta_in_amkan`] makes, for the same
//! reason, and it is made once here rather than per caller.
//!
//! ## Honesty about the tier is not softened anywhere in this module
//!
//! The text is scraped off a picture. It is not selectable, the timing is a poll
//! interval behind the game, the box the Arabic is drawn into is where the
//! recognizer last saw the original, and stylised fonts are read wrongly or not
//! at all. [`crate::sidq::Iqrar`] makes the user see that before any of this can
//! start, and nothing here restates it more kindly.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use parking_lot::{Mutex, RwLock};
use taarib_mustalahat::luba::LubaId;
use taarib_mustalahat::nass::{SiyaqNass, TasnifNass};
use taarib_usus::khata::Tafsir as _;

use crate::iltiqat_shasha::{
    IdadatTahsin, MuhassinSura, MuqayyidMuadal, SuraMultaqata, basmat_mutawassit,
};
use crate::khata::KhataTabaqa;
use crate::manatiq::{MajmuatManatiq, MuarrifMintaqa, QaidatTarjama};
use crate::mutarjim::{
    DhakiraJalsaMushtaraka, DhakiraTabaqa, MutarjimTabaqa, QaydTabaqa, RaddSatr, TASNIF_IFTIRADI,
    TalabDhakira, TalabSatr,
};
use crate::qira::{IkhtiyarQari, SatrMaqru};
use crate::sijill_qira::{MuarrifMadkhal, SijillMushtarak, TaqreerSijill, ThiqatSatr};
use crate::talqeem::SatrMulaqqam;
use crate::tatabbu::{
    HasilatDawra, IhsaatTatabbu, MuarrifSatr, Mutatabbi, QiraaMulahaza, SiyasatIstiqrar,
};
use crate::wajiha::{MustatilBiksel, WasfSath};
use crate::watira::{MunazzimWatira, TaghyeerWatira};

/// How many lines the overlay will draw at once.
///
/// Thirty-two. A screen holding more than that is a screen where either the
/// regions are drawn around the whole interface or the recognizer is reading a
/// texture as text, and in both cases drawing all of it produces an unreadable
/// wall over the game rather than a translation of it.
pub const AQSA_SUTUR_MARSUMA: usize = 32;

/// How many captures the worker's queue holds before the render thread stops
/// posting.
///
/// Four. The queue is not a buffer to be filled — a capture that has been
/// waiting four poll intervals describes a screen that has moved on, and
/// recognizing it produces a translation of what was said a second ago. When it
/// is full the render thread drops the capture and counts it, which is a fact
/// the report carries rather than one it hides.
pub const SAA_TABUR: usize = 4;

/// The confidence below which a reading is not worth translating.
///
/// Thirty-five. Both platform recognizers report confident nonsense rather than
/// failing on a region that holds a texture, and a fluent Arabic sentence saying
/// something the game did not say is the most expensive mistake this tier makes
/// — worse than showing nothing, because the player has no way to tell.
pub const ADNA_THIQA_LIL_TARJAMA: u8 = 35;

// ---------------------------------------------------------------------------
// What is on screen right now
// ---------------------------------------------------------------------------

/// The most recent completed translation of what is on screen.
///
/// Published by the worker, read by the render thread. Carries the surface it
/// was positioned against for the reason [`crate::wajiha::LawhatRasm`] does: a
/// snapshot built at 1920×1080 places text by that resolution's pixels, and
/// drawing it onto a 2560×1440 backbuffer puts every line in the wrong place.
#[derive(Debug, Clone, Default)]
pub struct LaqtaTarjama {
    /// The lines to draw, in the order they were recognized.
    pub sutur: Vec<SatrMulaqqam>,
    /// The surface the boxes are in the pixels of, once there is one.
    pub sath: Option<WasfSath>,
    /// Which publication this is. Monotonic, so a reader can tell a new
    /// snapshot from the same one read twice without comparing the lines.
    pub jeel: u64,
    /// The caller's microsecond counter at the moment it was published.
    pub lahza_mikro: u64,
}

impl LaqtaTarjama {
    /// Whether there is anything to draw.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.sutur.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// How a session reads, settles, translates and remembers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhiyaratQissa {
    /// What kind of string the regions of this game hold.
    ///
    /// One setting per session rather than per region, because the overlay has
    /// no extractor to derive it from — see
    /// [`crate::mutarjim::TASNIF_IFTIRADI`].
    pub tasnif: TasnifNass,

    /// How many times one settled line may be sent to a provider.
    ///
    /// Two. One retry covers a provider that was restarting; a third would mean
    /// a session with no network sends three requests per line of dialogue for
    /// three hours and gets three failures each time.
    pub aqsa_muhawalat: u32,

    /// The most lines drawn at once.
    pub saa_laqta: usize,

    /// The confidence below which a reading is not translated at all.
    pub adna_thiqa: u8,

    /// Whether settled lines are written to the reading history.
    pub yasjil: bool,

    /// How captures are preprocessed before recognition.
    pub tahsin: IdadatTahsin,

    /// When a reading is the same line, and when a line has stopped moving.
    pub istiqrar: SiyasatIstiqrar,

    /// The Hamming distance below which a region counts as unchanged.
    pub masafat_basma: u32,
}

impl KhiyaratQissa {
    /// The defaults.
    #[must_use]
    pub fn iftiradiya() -> Self {
        Self {
            tasnif: TASNIF_IFTIRADI,
            aqsa_muhawalat: 2,
            saa_laqta: AQSA_SUTUR_MARSUMA,
            adna_thiqa: ADNA_THIQA_LIL_TARJAMA,
            yasjil: true,
            tahsin: IdadatTahsin::default(),
            istiqrar: SiyasatIstiqrar::iftiradiya(),
            masafat_basma: MuqayyidMuadal::MASAFA_IFTIRADIYA,
        }
    }
}

impl Default for KhiyaratQissa {
    fn default() -> Self {
        Self::iftiradiya()
    }
}

// ---------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------

/// What the off-frame half of a session has done.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IhsaatQissa {
    /// Captures handed to [`Qissa::aalij`].
    pub iltiqatat: u64,
    /// Captures the perceptual hash let through to recognition.
    pub mumarrara: u64,
    /// Captures it stopped because the region had not changed.
    pub mahjuba: u64,
    /// Lines recognition returned.
    pub maqruaat: u64,
    /// Requests actually sent to a translation provider.
    ///
    /// The number this whole module exists to keep small.
    pub talabat: u64,
    /// Settled lines answered out of the translation memory instead.
    pub isabat_dhakira: u64,
    /// Settled lines skipped because the recognizer was not confident enough.
    pub thiqa_dunya: u64,
    /// Translation attempts that failed.
    pub ikhfaqat: u64,
    /// Pairs the memory would not store.
    pub ikhfaqat_hifz: u64,
    /// Snapshots published to the render thread.
    pub laqtat: u64,
}

impl IhsaatQissa {
    /// Requests a per-frame pipeline would have made and this one did not.
    ///
    /// Every recognized line is a string a pipeline with no identity would have
    /// sent. The difference against [`IhsaatQissa::talabat`] is what tracking,
    /// settling and the memory bought — stated as a subtraction of two numbers
    /// that are both printed, so it can be checked rather than believed.
    #[must_use]
    pub const fn muwaffara(&self) -> u64 {
        self.maqruaat.saturating_sub(self.talabat)
    }

    /// The sentence the control panel and the report carry.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = format!(
            "{} capture(s): {} recognized, {} held as unchanged. {} line(s) read produced {} \
             provider request(s) and {} memory hit(s)",
            self.iltiqatat,
            self.mumarrara,
            self.mahjuba,
            self.maqruaat,
            self.talabat,
            self.isabat_dhakira
        );
        if self.thiqa_dunya > 0 {
            let _ = write!(
                wasf,
                ", {} skipped as too uncertain to translate",
                self.thiqa_dunya
            );
        }
        if self.ikhfaqat > 0 {
            let _ = write!(wasf, ", {} translation failure(s)", self.ikhfaqat);
        }
        if self.ikhfaqat_hifz > 0 {
            let _ = write!(
                wasf,
                ", {} pair(s) the memory would not store",
                self.ikhfaqat_hifz
            );
        }
        wasf
    }
}

/// Everything a session can be asked about, in one value.
#[derive(Debug, Clone)]
pub struct KhulasatQissa {
    /// The off-frame counters.
    pub qissa: IhsaatQissa,
    /// What line tracking did.
    pub tatabbu: IhsaatTatabbu,
    /// The refresh rate and what happened to it.
    pub watira: String,
    /// Whether the refresh rate is currently below what was asked for.
    pub mutadahwira: bool,
    /// The translation memory's own account of itself.
    pub dhakira: String,
    /// The reading history's summary, when there is a history.
    pub sijill: Option<TaqreerSijill>,
    /// The translator's name, when one is attached.
    pub mutarjim: Option<String>,
}

impl KhulasatQissa {
    /// The whole account, in English, for the log and the diagnostics bundle.
    ///
    /// Every number, the unflattering ones included, in the order somebody
    /// debugging a session asks for them.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mut wasf = String::new();
        let _ = writeln!(wasf, "{}", self.qissa.wasf());
        let _ = writeln!(wasf, "{}", self.tatabbu.wasf());
        let _ = writeln!(wasf, "{}", self.watira);
        let _ = writeln!(wasf, "memory: {}", self.dhakira);
        let _ = writeln!(
            wasf,
            "translator: {}",
            self.mutarjim
                .as_deref()
                .unwrap_or("none attached — nothing will be translated")
        );
        if let Some(sijill) = &self.sijill {
            let _ = writeln!(wasf, "history: {}", sijill.wasf());
        }
        wasf
    }
}

/// What one pass over one region did.
#[derive(Debug, Clone, Default)]
pub struct KhulasatMualaja {
    /// Whether the perceptual hash let this capture through.
    pub taghayyarat: bool,
    /// How many lines recognition returned.
    pub maqru: usize,
    /// What tracking made of them.
    pub hasila: HasilatDawra,
    /// How many provider requests this pass sent.
    pub talabat: u32,
    /// How many settled lines the memory answered.
    pub isabat: u32,
    /// How many translation attempts failed.
    pub ikhfaqat: u32,
}

// ---------------------------------------------------------------------------
// The off-frame half
// ---------------------------------------------------------------------------

/// The session's off-frame half: everything that must not touch a game's frame.
///
/// Owns the tracker, the picture gates, the memory, the translator and the
/// history, and publishes what to draw. In a real session it lives on the worker
/// [`KhaytQissa`] spawns; it is a plain value with no threading of its own so
/// that the whole pipeline can be driven step by step without a GPU, a game or a
/// clock in scope.
pub struct Qissa {
    khiyarat: KhiyaratQissa,
    luba: LubaId,
    ism_luba: String,
    muhassin: MuhassinSura,
    mutatabbi: Mutatabbi,
    bawwabat: BTreeMap<MuarrifMintaqa, MuqayyidMuadal>,
    dhakira: Arc<dyn DhakiraTabaqa>,
    mutarjim: Option<Box<dyn MutarjimTabaqa>>,
    sijill: Option<SijillMushtarak>,
    ism_qari: Option<String>,
    marsuma: BTreeMap<MuarrifSatr, SatrMulaqqam>,
    /// How many times each live line's *current text* has been sent.
    ///
    /// The text is held beside the count, not just the identity. A line that
    /// grows, is corrected, or is replaced keeps its identity and becomes a
    /// different sentence, and a cap counted per identity would refuse to
    /// translate the third thing a long-lived line ever said.
    muhawalat: BTreeMap<MuarrifSatr, (String, u32)>,
    madakhil: BTreeMap<MuarrifSatr, MuarrifMadkhal>,
    manshura: Arc<RwLock<LaqtaTarjama>>,
    sath: Option<WasfSath>,
    jeel: u64,
    ihsaat: IhsaatQissa,
    athar: Vec<String>,
}

impl std::fmt::Debug for Qissa {
    fn fmt(&self, matbaa: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        matbaa
            .debug_struct("Qissa")
            .field("luba", &self.luba)
            .field("khiyarat", &self.khiyarat)
            .field("mutatabbi", self.mutatabbi.ihsaat())
            .field("dhakira", &self.dhakira.ism())
            .field(
                "mutarjim",
                &self.mutarjim.as_ref().map(|m| m.ism().to_owned()),
            )
            .field("marsuma", &self.marsuma.len())
            .field("ihsaat", &self.ihsaat)
            .finish_non_exhaustive()
    }
}

impl Qissa {
    /// A session for one game, with a session-lifetime memory and no
    /// translator.
    ///
    /// The memory and the translator are attached separately because they come
    /// from different places: the memory is the workspace's shared one, opened
    /// by whoever has a database; the translator is a provider, configured by
    /// whoever has credentials or a local model server. A session with neither
    /// still tracks, still settles and still records a reading history — which
    /// is what a player with no network gets, and it is more than nothing.
    #[must_use]
    pub fn jadeeda(luba: LubaId, ism_luba: impl Into<String>, khiyarat: KhiyaratQissa) -> Self {
        Self {
            muhassin: MuhassinSura::jadeed(khiyarat.tahsin),
            mutatabbi: Mutatabbi::bi_siyasa(khiyarat.istiqrar),
            khiyarat,
            luba,
            ism_luba: ism_luba.into(),
            bawwabat: BTreeMap::new(),
            dhakira: Arc::new(DhakiraJalsaMushtaraka::iftiradiya()),
            mutarjim: None,
            sijill: None,
            ism_qari: None,
            marsuma: BTreeMap::new(),
            muhawalat: BTreeMap::new(),
            madakhil: BTreeMap::new(),
            manshura: Arc::new(RwLock::new(LaqtaTarjama::default())),
            sath: None,
            jeel: 0,
            ihsaat: IhsaatQissa::default(),
            athar: Vec::new(),
        }
    }

    /// Attaches the shared translation memory.
    ///
    /// Replaces the session-lifetime one. This is what makes a second
    /// playthrough free, and it is the only thing that does — see
    /// [`crate::mutarjim::DhakiraJalsa`].
    #[must_use]
    pub fn bi_dhakira(mut self, dhakira: Arc<dyn DhakiraTabaqa>) -> Self {
        self.athar.push(format!(
            "memory: {} ({})",
            dhakira.ism(),
            if dhakira.daaima() {
                "persistent"
            } else {
                "session only"
            }
        ));
        self.dhakira = dhakira;
        self
    }

    /// Attaches a translation provider.
    #[must_use]
    pub fn bi_mutarjim(mut self, mutarjim: Box<dyn MutarjimTabaqa>) -> Self {
        self.athar.push(format!("translator: {}", mutarjim.ism()));
        self.mutarjim = Some(mutarjim);
        self
    }

    /// Attaches the reading history.
    #[must_use]
    pub fn bi_sijill(mut self, sijill: SijillMushtarak) -> Self {
        self.athar.push("reading history attached".to_owned());
        self.sijill = Some(sijill);
        self
    }

    /// Records which recognizer is reading, for the memory's provenance.
    #[must_use]
    pub fn bi_qari(mut self, ism: impl Into<String>) -> Self {
        self.ism_qari = Some(ism.into());
        self
    }

    /// The handle the render thread reads the drawn lines through.
    #[must_use]
    pub fn manshura(&self) -> Arc<RwLock<LaqtaTarjama>> {
        Arc::clone(&self.manshura)
    }

    /// The counters.
    #[must_use]
    pub const fn ihsaat(&self) -> &IhsaatQissa {
        &self.ihsaat
    }

    /// The line tracker, for a panel showing what is on screen.
    #[must_use]
    pub const fn mutatabbi(&self) -> &Mutatabbi {
        &self.mutatabbi
    }

    /// The reading history, when one is attached.
    #[must_use]
    pub const fn sijill(&self) -> Option<&SijillMushtarak> {
        self.sijill.as_ref()
    }

    /// What has happened to this session, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Everything the control panel asks about, given the governor's own
    /// account.
    ///
    /// The governor lives on the render half, so its sentence is passed in
    /// rather than reached for. That is deliberate: the alternative is a second
    /// governor here, which would report a refresh rate nothing is polling at.
    #[must_use]
    pub fn khulasa(&self, watira: &MunazzimWatira) -> KhulasatQissa {
        KhulasatQissa {
            qissa: self.ihsaat,
            tatabbu: *self.mutatabbi.ihsaat(),
            watira: watira.wasf(),
            mutadahwira: watira.mutadahwira(),
            dhakira: self.dhakira.ism().to_owned(),
            sijill: self
                .sijill
                .as_ref()
                .and_then(SijillMushtarak::taqreer_in_amkan),
            mutarjim: self
                .mutarjim
                .as_ref()
                .map(|mutarjim| mutarjim.ism().to_owned()),
        }
    }

    /// Whether a captured region differs enough from the last one recognized.
    ///
    /// The second of the three gates, and the expensive-to-cheap ordering is why
    /// it is separate from [`Qissa::aalij`]: the clock gate on the render half
    /// has already rejected most frames for free, and this one costs one
    /// preprocessing pass to reject the rest before recognition — which costs a
    /// hundred times as much — is allowed to run.
    ///
    /// [`QaidatTarjama::Mustamirra`] bypasses it, which is exactly what that
    /// rule means and what its documented cost is.
    ///
    /// # Errors
    ///
    /// Whatever [`MuhassinSura::hassin`] refuses for a capture whose bytes do
    /// not match its own geometry.
    pub fn hal_taghayyarat(
        &mut self,
        mintaqa: MuarrifMintaqa,
        qaida: QaidatTarjama,
        sura: &SuraMultaqata,
    ) -> Result<bool, KhataTabaqa> {
        let ramadi = self.muhassin.hassin(sura)?;
        let basma = basmat_mutawassit(&ramadi);
        let masafa = self.khiyarat.masafat_basma;
        let bawwaba = self
            .bawwabat
            .entry(mintaqa)
            .or_insert_with(|| MuqayyidMuadal::jadeed(0, masafa));
        let taghayyarat = bawwaba.hal_taghayyarat(basma);
        Ok(taghayyarat || qaida.yatajahal_muqarana())
    }

    /// Runs one whole pass over one captured region.
    ///
    /// Gate, recognize, track, translate what settled, publish. The call a
    /// worker makes and the one every step of the budget rule is expressed in.
    ///
    /// # Errors
    ///
    /// Whatever preprocessing and recognition refuse.
    /// [`KhataTabaqa::LaNassMaqru`] is **not** an error here: a region between
    /// lines of dialogue produces it on most passes, and it is fed to the
    /// tracker as an empty pass — which is what eventually ends the line that
    /// was there. See [`crate::tatabbu::Mutatabbi::sajjil`].
    pub fn aalij(
        &mut self,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        qaida: QaidatTarjama,
        lahza_mikro: u64,
        sura: &SuraMultaqata,
        qari: &mut IkhtiyarQari,
    ) -> Result<KhulasatMualaja, KhataTabaqa> {
        self.ihsaat.iltiqatat = self.ihsaat.iltiqatat.saturating_add(1);
        if !self.hal_taghayyarat(mintaqa, qaida, sura)? {
            self.ihsaat.mahjuba = self.ihsaat.mahjuba.saturating_add(1);
            // Held back, not skipped. "The picture has not changed" is the
            // settle policy's own question answered for the price of one
            // preprocessing pass, so the pass still confirms every live line in
            // the region — and a line that had stopped moving settles here,
            // without a second recognition of a sentence nobody has changed.
            return Ok(self.thabbit(mintaqa, ism_mintaqa, lahza_mikro));
        }
        self.ihsaat.mumarrara = self.ihsaat.mumarrara.saturating_add(1);

        let sutur = match qari.iqra_mudmaj(sura, None) {
            Ok(sutur) => sutur,
            // Nothing readable is the ordinary answer for a region between
            // lines. It is fed through as an empty pass rather than returned as
            // an error, because an empty pass is information: it is what ends
            // the line that was on screen a moment ago.
            Err(KhataTabaqa::LaNassMaqru { .. }) => Vec::new(),
            Err(khata) => return Err(khata),
        };
        let mulahazat: Vec<QiraaMulahaza> = sutur
            .iter()
            .map(|satr| QiraaMulahaza::min_maqru(satr, sura.ila_sath(satr.mawdi)))
            .collect();
        let mut khulasa = self.aalij_sutur(mintaqa, ism_mintaqa, lahza_mikro, &mulahazat);
        khulasa.taghayyarat = true;
        khulasa.maqru = sutur.len();
        Ok(khulasa)
    }

    /// Confirms a region whose picture the change gate found unchanged.
    ///
    /// Public because a caller driving capture itself — through
    /// [`Qissa::hal_taghayyarat`] rather than [`Qissa::aalij`] — owes the tracker
    /// this. Skipping it means a subtitle that never moves also never settles,
    /// and is therefore never translated at all.
    pub fn thabbit(
        &mut self,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        lahza_mikro: u64,
    ) -> KhulasatMualaja {
        let hasila = self.mutatabbi.thabbit(mintaqa, lahza_mikro);
        let mut khulasa = KhulasatMualaja {
            hasila: hasila.clone(),
            ..KhulasatMualaja::default()
        };
        for muarrif in &hasila.mustaqirra {
            match self.aalij_mustaqirr(*muarrif, mintaqa, ism_mintaqa, lahza_mikro) {
                NatijatSatr::Talab => khulasa.talabat = khulasa.talabat.saturating_add(1),
                NatijatSatr::Dhakira => khulasa.isabat = khulasa.isabat.saturating_add(1),
                NatijatSatr::Ikhfaq => khulasa.ikhfaqat = khulasa.ikhfaqat.saturating_add(1),
                NatijatSatr::LaShay => {},
            }
        }
        if !hasila.mustaqirra.is_empty() {
            self.inshur(lahza_mikro);
        }
        khulasa
    }

    /// The same pass, from lines that have already been recognized.
    ///
    /// Split out because everything after recognition is where the cost is
    /// decided, and it is testable without a graphics device, a model file or a
    /// game. The boxes are expected in **surface** pixels — see
    /// [`QiraaMulahaza::min_maqru`].
    pub fn aalij_sutur(
        &mut self,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        lahza_mikro: u64,
        mulahazat: &[QiraaMulahaza],
    ) -> KhulasatMualaja {
        self.ihsaat.maqruaat = self
            .ihsaat
            .maqruaat
            .saturating_add(u64::try_from(mulahazat.len()).unwrap_or(u64::MAX));

        let hasila = self.mutatabbi.sajjil(mintaqa, lahza_mikro, mulahazat);
        let mut khulasa = KhulasatMualaja {
            taghayyarat: true,
            maqru: mulahazat.len(),
            hasila: hasila.clone(),
            ..KhulasatMualaja::default()
        };

        for muntahi in &hasila.muntahiya {
            let _ = self.marsuma.remove(&muntahi.muarrif);
            let _ = self.muhawalat.remove(&muntahi.muarrif);
            let _ = self.madakhil.remove(&muntahi.muarrif);
        }

        for muarrif in &hasila.mustaqirra {
            match self.aalij_mustaqirr(*muarrif, mintaqa, ism_mintaqa, lahza_mikro) {
                NatijatSatr::Talab => khulasa.talabat = khulasa.talabat.saturating_add(1),
                NatijatSatr::Dhakira => khulasa.isabat = khulasa.isabat.saturating_add(1),
                NatijatSatr::Ikhfaq => khulasa.ikhfaqat = khulasa.ikhfaqat.saturating_add(1),
                NatijatSatr::LaShay => {},
            }
        }

        // Rebuilt from the tracker every pass rather than patched, so a line
        // whose box moved is drawn where it is now and a line that ended stops
        // being drawn on the same pass it ended.
        self.inshur(lahza_mikro);
        khulasa
    }

    /// Tells the session the surface changed under it.
    ///
    /// Every tracked box is in the old surface's pixels and every picture gate
    /// is holding a hash of a differently sized region. Both are forgotten, and
    /// the published snapshot is emptied rather than left to be drawn in the
    /// wrong place for one frame.
    pub fn sath_taghayyar(&mut self, sath: WasfSath) {
        self.mutatabbi.amsah();
        self.bawwabat.clear();
        self.marsuma.clear();
        self.muhawalat.clear();
        self.madakhil.clear();
        self.sath = Some(sath);
        self.athar
            .push(format!("surface changed to {}×{}", sath.ard, sath.irtifa));
        self.inshur(0);
    }

    /// Records which surface the boxes coming in are in the pixels of.
    ///
    /// Called once when the session starts and again after every change. A
    /// snapshot with no surface is one the render half will not draw, which is
    /// the correct behaviour before anything has said what it was built for.
    pub fn ayyin_sath(&mut self, sath: WasfSath) {
        if self.sath == Some(sath) {
            return;
        }
        self.sath_taghayyar(sath);
    }

    /// Makes every region re-read on its next capture.
    ///
    /// What the panel's translate-now command does. A user who asks for a
    /// re-read and is told nothing changed has been given a correct answer to a
    /// question they did not ask — the same reasoning
    /// [`MuqayyidMuadal::ansa`] is written for.
    pub fn iqra_alan(&mut self) {
        for bawwaba in self.bawwabat.values_mut() {
            bawwaba.ansa();
        }
        self.athar
            .push("re-read requested; every region's change gate was cleared".to_owned());
    }

    /// Forgets one region entirely.
    ///
    /// For a region the user moved or deleted: its rectangle now frames
    /// something else, so its lines are not the same lines and its hash is of a
    /// different picture.
    pub fn ansa_mintaqa(&mut self, mintaqa: MuarrifMintaqa) {
        let muarrifat: Vec<MuarrifSatr> = self
            .mutatabbi
            .hayyat_mintaqa(mintaqa)
            .iter()
            .map(|satr| satr.muarrif)
            .collect();
        for muarrif in muarrifat {
            let _ = self.marsuma.remove(&muarrif);
            let _ = self.muhawalat.remove(&muarrif);
            let _ = self.madakhil.remove(&muarrif);
        }
        self.mutatabbi.ansa_mintaqa(mintaqa);
        let _ = self.bawwabat.remove(&mintaqa);
        self.inshur(0);
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    /// Settles one line: history, memory, provider, in that order.
    fn aalij_mustaqirr(
        &mut self,
        muarrif: MuarrifSatr,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        lahza_mikro: u64,
    ) -> NatijatSatr {
        let Some((asl, mawdi, thiqa)) = self.mutatabbi.satr(muarrif).map(|satr| {
            (
                satr.nass.clone(),
                satr.mawdi,
                ThiqatSatr::min_maqru(satr.thiqa, satr.maqisa),
            )
        }) else {
            return NatijatSatr::LaShay;
        };

        // Marked before anything can fail. The alternative — mark on success —
        // means a provider that is down is asked again on the very next pass,
        // which for a session with no network is four requests a second per
        // region for three hours.
        let _ = self.mutatabbi.sajjil_irsal(muarrif);

        let mawdi_sijill =
            self.qayyid_sijill(muarrif, mintaqa, ism_mintaqa, lahza_mikro, &asl, thiqa);
        let _ = mawdi_sijill;

        // An engine that reports nothing is not a low-confidence engine, so an
        // unmeasured reading is *not* held back by the floor: doing so would
        // translate nothing at all on Windows and Linux, where nothing measures.
        // The floor screens out the readings an engine itself called doubtful.
        if thiqa
            .mia_in_wujidat()
            .is_some_and(|mia| mia < self.khiyarat.adna_thiqa)
        {
            self.ihsaat.thiqa_dunya = self.ihsaat.thiqa_dunya.saturating_add(1);
            return NatijatSatr::LaShay;
        }

        let talab = TalabDhakira {
            asl: &asl,
            luba: self.luba,
            tasnif: self.khiyarat.tasnif,
        };
        if let Some(radd) = self.dhakira.ibhath(&talab) {
            self.ihsaat.isabat_dhakira = self.ihsaat.isabat_dhakira.saturating_add(1);
            self.arsi(muarrif, mawdi, thiqa, &radd);
            self.alhiq_tarjama(muarrif, &radd);
            return NatijatSatr::Dhakira;
        }

        let Some(mutarjim) = self.mutarjim.as_ref() else {
            return NatijatSatr::LaShay;
        };
        if !mutarjim.mutah() {
            self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
            return NatijatSatr::Ikhfaq;
        }
        let saqf = self.khiyarat.aqsa_muhawalat;
        let muhawala = self
            .muhawalat
            .entry(muarrif)
            .or_insert_with(|| (asl.clone(), 0));
        if muhawala.0 != asl {
            *muhawala = (asl.clone(), 0);
        }
        if muhawala.1 >= saqf {
            return NatijatSatr::LaShay;
        }
        muhawala.1 = muhawala.1.saturating_add(1);

        let siyaq = self.siyaq_li(mintaqa, ism_mintaqa, &asl);
        let talab = TalabSatr {
            asl: &asl,
            siyaq: &siyaq,
            tasnif: self.khiyarat.tasnif,
            luba: self.luba,
            ism_luba: Some(&self.ism_luba),
            mawdi,
            thiqa: thiqa.mia(),
        };
        self.ihsaat.talabat = self.ihsaat.talabat.saturating_add(1);
        let radd = match mutarjim.tarjim(&talab) {
            Ok(radd) if !radd.khali() => radd,
            Ok(_) => {
                self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
                self.athar
                    .push(format!("{muarrif}: the translator returned nothing"));
                return NatijatSatr::Ikhfaq;
            },
            Err(khata) => {
                self.ihsaat.ikhfaqat = self.ihsaat.ikhfaqat.saturating_add(1);
                self.athar
                    .push(format!("{muarrif}: translation failed: {khata}"));
                return NatijatSatr::Ikhfaq;
            },
        };

        let qayd = QaydTabaqa {
            asl: &asl,
            arabi: &radd.arabi,
            luba: self.luba,
            ism_luba: Some(&self.ism_luba),
            mintaqa: Some(ism_mintaqa),
            tasnif: self.khiyarat.tasnif,
            qari: self.ism_qari.as_deref(),
            muzawwid: Some(mutarjim.ism()),
            // The recognizer's own number **only when it measured one**. The
            // memory treats unmeasured as a distinct state and refuses to
            // invent a number for it; handing over the stand-in constant here
            // would launder it into the store as an observation and quietly
            // defeat the guard that withholds unmeasured accumulations.
            thiqa: thiqa.mia_in_wujidat(),
        };
        if let Err(khata) = self.dhakira.sajjil(&qayd) {
            // Counted, never fatal. A translation that could not be remembered
            // is still a translation, and refusing to draw it would be losing
            // the line to protect the memory.
            self.ihsaat.ikhfaqat_hifz = self.ihsaat.ikhfaqat_hifz.saturating_add(1);
            self.athar.push(format!(
                "{muarrif}: the memory would not store the pair: {khata}"
            ));
        }

        self.arsi(muarrif, mawdi, thiqa, &radd);
        self.alhiq_tarjama(muarrif, &radd);
        NatijatSatr::Talab
    }

    /// The surrounding context for one line, in the shape the static path uses.
    ///
    /// `asl` is the line being translated, and it is passed so that
    /// [`Mutatabbi::siyaq_duna`] can keep it out of its own context: a line
    /// joins the ring on the pass it settles, which is the pass its translation
    /// is asked for.
    fn siyaq_li(&self, mintaqa: MuarrifMintaqa, ism_mintaqa: &str, asl: &str) -> SiyaqNass {
        SiyaqNass {
            // There is no container: tier 3 exists because the game's own data
            // could not be read. Naming the overlay is the honest answer, and
            // it is what a translator looking at a suggestion card needs to
            // know about where the pair came from.
            hawiya: crate::DALIL_TABAQA.to_owned(),
            mawqi: mintaqa.to_string(),
            haql: None,
            mutakallim: None,
            jiwar: self.mutatabbi.siyaq_duna(mintaqa, asl),
            mashhad: Some(ism_mintaqa.to_owned()),
            laqta: None,
        }
    }

    /// Records a settled line in the reading history, before it is translated.
    ///
    /// Before, deliberately. The history's whole value to a player is that it
    /// holds *what was missed*, and a line whose translation never arrived is
    /// exactly the line somebody is trying to find out about. See
    /// [`crate::sijill_qira`].
    fn qayyid_sijill(
        &mut self,
        muarrif: MuarrifSatr,
        mintaqa: MuarrifMintaqa,
        ism_mintaqa: &str,
        lahza_mikro: u64,
        asl: &str,
        thiqa: ThiqatSatr,
    ) -> Option<MuarrifMadkhal> {
        if !self.khiyarat.yasjil {
            return None;
        }
        let sijill = self.sijill.as_ref()?;
        // Seconds, from the caller's own microsecond counter. This crate reads
        // no clock — see `MadkhalQira::lahza`.
        let thawani = lahza_mikro.checked_div(1_000_000).unwrap_or(0);
        let natija = match sijill.qayyid(thawani, mintaqa, ism_mintaqa, asl, thiqa) {
            Ok(natija) => natija,
            Err(khata) => {
                self.athar
                    .push(format!("the reading history could not be written: {khata}"));
                sijill.sajjil(thawani, mintaqa, ism_mintaqa, asl, thiqa)
            },
        };
        let madkhal = natija.muarrif()?;
        let _ = self.madakhil.insert(muarrif, madkhal);
        Some(madkhal)
    }

    /// Attaches a translation to the history row this line was recorded as.
    fn alhiq_tarjama(&self, muarrif: MuarrifSatr, radd: &RaddSatr) {
        let (Some(sijill), Some(madkhal)) =
            (self.sijill.as_ref(), self.madakhil.get(&muarrif).copied())
        else {
            return;
        };
        let _ = sijill.adkhil_tarjama(madkhal, &radd.arabi, radd.masdar);
    }

    /// Puts a translated line into the set that gets drawn.
    fn arsi(
        &mut self,
        muarrif: MuarrifSatr,
        mawdi: MustatilBiksel,
        thiqa: ThiqatSatr,
        radd: &RaddSatr,
    ) {
        let _ = self.marsuma.insert(
            muarrif,
            SatrMulaqqam {
                nass: radd.arabi.clone(),
                mawdi,
                thiqa: thiqa.mia(),
            },
        );
    }

    /// Publishes the current set of drawn lines to the render half.
    ///
    /// Boxes are taken from the tracker rather than from what was stored when
    /// the translation arrived, so a line that moved is drawn where it is now.
    fn inshur(&mut self, lahza_mikro: u64) {
        let mut sutur: Vec<SatrMulaqqam> = Vec::new();
        for satr in self.mutatabbi.hayya() {
            let Some(marsum) = self.marsuma.get(&satr.muarrif) else {
                continue;
            };
            if sutur.len() >= self.khiyarat.saa_laqta {
                break;
            }
            sutur.push(SatrMulaqqam {
                nass: marsum.nass.clone(),
                mawdi: satr.mawdi,
                thiqa: satr.thiqa,
            });
        }
        self.jeel = self.jeel.saturating_add(1);
        self.ihsaat.laqtat = self.ihsaat.laqtat.saturating_add(1);
        let laqta = LaqtaTarjama {
            sutur,
            sath: self.sath,
            jeel: self.jeel,
            lahza_mikro,
        };
        *self.manshura.write() = laqta;
    }
}

/// What settling one line ended up costing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NatijatSatr {
    /// A provider request was sent.
    Talab,
    /// The memory answered it.
    Dhakira,
    /// An attempt failed.
    Ikhfaq,
    /// Nothing was spent: no translator, too uncertain, or out of attempts.
    LaShay,
}

// ---------------------------------------------------------------------------
// The on-frame half
// ---------------------------------------------------------------------------

/// One region the clock says it is time to capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintaqaMustahiqqa {
    /// Which region.
    pub muarrif: MuarrifMintaqa,
    /// Its name at this moment, copied so the worker does not hold the set.
    pub ism: String,
    /// Where to read, in surface pixels.
    pub mustatil: MustatilBiksel,
    /// The rule it is on, which decides whether the picture gate applies.
    pub qaida: QaidatTarjama,
}

/// The session's on-frame half: the budget, the clock, and the last snapshot.
///
/// Everything here is arithmetic. It never recognizes, never translates, never
/// allocates a picture and never waits for the worker.
#[derive(Debug)]
pub struct Munassiq {
    watira: MunazzimWatira,
    muaqqitat: BTreeMap<MuarrifMintaqa, MuqayyidMuadal>,
    manshura: Arc<RwLock<LaqtaTarjama>>,
    marsuma: Vec<SatrMulaqqam>,
    sath: Option<WasfSath>,
    jeel: u64,
    tark_qufl: u64,
    athar: Vec<String>,
}

impl Munassiq {
    /// A coordinator over a session's published snapshot.
    #[must_use]
    pub const fn jadeed(manshura: Arc<RwLock<LaqtaTarjama>>, watira: MunazzimWatira) -> Self {
        Self {
            watira,
            muaqqitat: BTreeMap::new(),
            manshura,
            marsuma: Vec::new(),
            sath: None,
            jeel: 0,
            tark_qufl: 0,
            athar: Vec::new(),
        }
    }

    /// The refresh governor.
    #[must_use]
    pub const fn watira(&self) -> &MunazzimWatira {
        &self.watira
    }

    /// The refresh governor, to change its base interval or its ceiling.
    pub const fn watira_mut(&mut self) -> &mut MunazzimWatira {
        &mut self.watira
    }

    /// How many frames could not read the snapshot because the worker held it.
    ///
    /// Reported rather than hidden: a number that climbs means the worker is
    /// publishing faster than the render thread can pick it up, which is worth
    /// knowing and is never worth waiting for.
    #[must_use]
    pub const fn tark_qufl(&self) -> u64 {
        self.tark_qufl
    }

    /// What has happened, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Records one frame's overlay cost and picks up any completed result.
    ///
    /// The single call a present hook makes before drawing. `mikro` is the
    /// caller's measurement of the overlay's own cost on the previous frame,
    /// the same number [`crate::wajiha::Tabaqa::itar`] is given.
    ///
    /// Returns [`Some`] on the frame the refresh rate moved, so the caller can
    /// log it as it happens. See this module's header on why the rate is what
    /// gets given up.
    pub fn itar(&mut self, mikro: u32) -> Option<TaghyeerWatira> {
        self.iltaqit_laqta();
        let taghyeer = self.watira.sajjil_itar(mikro);
        if let Some(taghyeer) = taghyeer {
            self.athar.push(taghyeer.satr());
        }
        taghyeer
    }

    /// The most recent completed result, for the surface being drawn now.
    ///
    /// Empty when the snapshot was positioned against a different surface. Not
    /// scaled — scaled text is blurry text, and the next snapshot will be
    /// correct. Never blocks: see this module's header.
    #[must_use]
    pub fn laqta(&self, sath: WasfSath) -> &[SatrMulaqqam] {
        if self.sath == Some(sath) {
            &self.marsuma
        } else {
            &[]
        }
    }

    /// Which regions the clock says to capture on this frame.
    ///
    /// The interval each region is held to is the slower of its own and the
    /// governor's, so a degraded session slows every region down and a region
    /// that asked to be read every five seconds is still read every five
    /// seconds when nothing is degraded.
    ///
    /// [`QaidatTarjama::IndaTalab`] regions never appear here — that is what the
    /// rule means and what its documented cost of exactly nothing depends on.
    pub fn mustahiqqa(
        &mut self,
        lahza_mikro: u64,
        majmua: &MajmuatManatiq,
        sath: WasfSath,
    ) -> Vec<MintaqaMustahiqqa> {
        let iftiradi = majmua.fasila_iftiradiya_milli();
        let fasil_watira = self.watira.fasil_mikro();
        let mut mustahiqqa = Vec::new();
        for mintaqa in majmua.ala_muaqqit() {
            let Some(mustatil) = mintaqa.fi_bikselat(sath) else {
                continue;
            };
            let fasil_mintaqa = u64::from(mintaqa.fasila_faaila(iftiradi)).saturating_mul(1_000);
            let fasil = fasil_mintaqa.max(fasil_watira);
            let muaqqit = self
                .muaqqitat
                .entry(mintaqa.muarrif)
                .or_insert_with(|| MuqayyidMuadal::jadeed(fasil, 0));
            muaqqit.ihdud(fasil);
            if !muaqqit.hal_yalzam(lahza_mikro) {
                continue;
            }
            mustahiqqa.push(MintaqaMustahiqqa {
                muarrif: mintaqa.muarrif,
                ism: mintaqa.ism.clone(),
                mustatil,
                qaida: mintaqa.qaida,
            });
        }
        mustahiqqa
    }

    /// Makes every region due on the next frame.
    ///
    /// The clock half of the panel's translate-now command;
    /// [`Qissa::iqra_alan`] is the picture half, and both are needed — clearing
    /// one and not the other means a user who asks for a re-read waits for the
    /// interval, or gets told nothing changed.
    pub fn iqra_alan(&mut self) {
        for muaqqit in self.muaqqitat.values_mut() {
            muaqqit.ansa();
        }
    }

    /// Forgets one region's clock.
    pub fn ansa_mintaqa(&mut self, mintaqa: MuarrifMintaqa) {
        let _ = self.muaqqitat.remove(&mintaqa);
    }

    /// Puts the refresh rate back to the base after a surface change.
    ///
    /// The frames just measured describe a scene that no longer exists, and
    /// holding a degradation earned by a loading screen against the game that
    /// follows it would leave a session permanently slow for the one moment it
    /// was legitimately expensive.
    pub fn sath_taghayyar(&mut self) {
        self.watira.istanif();
        self.muaqqitat.clear();
        self.marsuma.clear();
        self.sath = None;
        self.athar
            .push("refresh rate reset after a surface change".to_owned());
    }

    /// Copies the worker's snapshot out, if it is not being written.
    fn iltaqit_laqta(&mut self) {
        let Some(manshura) = self.manshura.try_read() else {
            self.tark_qufl = self.tark_qufl.saturating_add(1);
            return;
        };
        if manshura.jeel == self.jeel {
            return;
        }
        self.jeel = manshura.jeel;
        self.sath = manshura.sath;
        self.marsuma.clear();
        self.marsuma.extend(manshura.sutur.iter().cloned());
    }
}

// ---------------------------------------------------------------------------
// The worker
// ---------------------------------------------------------------------------

/// What the render half posts to the worker.
#[derive(Debug)]
enum RisalatQissa {
    /// A captured region, to be recognized and translated.
    Iltiqat {
        /// Which region it came from.
        mintaqa: MuarrifMintaqa,
        /// Its name at the moment of capture.
        ism: String,
        /// The rule it is on.
        qaida: QaidatTarjama,
        /// The caller's microsecond counter at the moment of capture.
        lahza_mikro: u64,
        /// The pixels.
        sura: SuraMultaqata,
    },
    /// The surface changed; forget every box and every hash.
    Sath(WasfSath),
    /// Clear every picture gate, for a translate-now.
    IqraAlan,
    /// Forget one region.
    Nisyan(MuarrifMintaqa),
    /// Stop.
    Tawaqquf,
}

/// What the worker's most recent refusal was, kept beside the count.
///
/// A count alone cannot say whether it is looking at one noisy frame or at a
/// recognizer that will never work again, and the two want opposite things
/// from the user: nothing, and a two-minute language-pack install. So the
/// refusal's identity survives, in both languages, and the terminal kind also
/// closes the door — see [`KhaytQissa::adfa`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum HalatKhayt {
    /// Every pass so far completed, or read nothing, which is ordinary.
    #[default]
    Salima,
    /// The most recent refusal was one capture's; the next is worth posting.
    Aabira {
        /// The refusal, in English.
        sabab: String,
        /// The same, in Arabic.
        sabab_arabi: String,
    },
    /// The recognizer is gone for the session, and no later capture changes
    /// that. Captures are refused at the door until a re-read is asked for.
    Mutawaqqifa {
        /// The refusal, in English.
        sabab: String,
        /// The same, in Arabic.
        sabab_arabi: String,
    },
}

impl HalatKhayt {
    /// Whether recognition has stopped for the session.
    #[must_use]
    pub const fn mutawaqqifa(&self) -> bool {
        matches!(self, Self::Mutawaqqifa { .. })
    }

    /// The sentence the control panel shows, in English.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Salima => "every pass completed or read nothing".to_owned(),
            Self::Aabira { sabab, .. } => {
                format!("the most recent refusal was one capture's: {sabab}")
            },
            Self::Mutawaqqifa { sabab, .. } => format!(
                "recognition has stopped for this session: {sabab}. Captures are refused until a \
                 re-read is asked for"
            ),
        }
    }

    /// The same sentence, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        match self {
            Self::Salima => "اكتملت كل الجولات أو لم تجد نصًا.".to_owned(),
            Self::Aabira { sabab_arabi, .. } => {
                format!("آخر رفض كان لالتقاطة واحدة: {sabab_arabi}")
            },
            Self::Mutawaqqifa { sabab_arabi, .. } => format!(
                "توقّفت القراءة في هذه الجلسة: {sabab_arabi} تُرفض الالتقاطات حتى يُطلب إعادة \
                 القراءة."
            ),
        }
    }
}

/// What happened to one posted capture.
///
/// Three answers rather than a `bool`, because the two ways a capture does not
/// reach the worker mean different things: dropped is "the worker is behind and
/// this frame is stale", refused is "nothing will be read until somebody acts".
/// A `false` that meant both would be the same integer this type replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatDaf {
    /// Queued for the worker.
    Qubilat,
    /// Dropped because the queue was full. Ordinary under load.
    Turikat,
    /// Refused at the door: recognition has stopped for the session.
    Rufidat,
}

/// The counters and the last refusal, readable from the render half without
/// touching the worker.
#[derive(Debug, Default)]
struct AdaadKhayt {
    mursala: AtomicU64,
    matruka: AtomicU64,
    marfuda: AtomicU64,
    muaalaja: AtomicU64,
    akhta: AtomicU64,
    /// Mirrors [`HalatKhayt::mutawaqqifa`] so the present path reads one
    /// atomic rather than taking a lock.
    mutawaqqif: AtomicBool,
    hala: Mutex<HalatKhayt>,
}

/// The worker thread that runs everything off the presentation path.
///
/// Owns a [`Qissa`] and the recognizer. The render half holds only a bounded
/// sender and the published snapshot, so posting a capture is a move of a `Vec`
/// into a queue and nothing else.
///
/// The queue is **bounded and lossy on purpose**. A capture that has been
/// waiting four poll intervals describes a screen that has moved on;
/// recognizing it produces a translation of a sentence the player has already
/// read past. Dropping it is the honest behaviour, and
/// [`KhaytQissa::matruka`] is the count that says how often it happened.
///
/// A refused pass keeps its identity. [`KhataTabaqa::yunhi_al_qiraa`] separates
/// the refusal that ends the session — the recognizer is gone — from the one
/// that ends a capture, and only the first closes the door: after it,
/// [`KhaytQissa::adfa`] answers [`HalatDaf::Rufidat`] without queueing, the
/// panel reads [`KhaytQissa::hala`] for the reason, and
/// [`KhaytQissa::iqra_alan`] — the user's translate-now — is what reopens it.
/// An overlay that retried a dead recognizer four times a second forever would
/// show a blank panel and a climbing integer to a user whose fix was a
/// language-pack install.
#[derive(Debug)]
pub struct KhaytQissa {
    mursil: std::sync::mpsc::SyncSender<RisalatQissa>,
    khayt: Option<std::thread::JoinHandle<()>>,
    manshura: Arc<RwLock<LaqtaTarjama>>,
    adaad: Arc<AdaadKhayt>,
}

impl KhaytQissa {
    /// Starts a worker over a session and a recognizer.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the thread cannot be spawned, which on
    /// a machine with no thread budget left is a real answer and not one to
    /// panic on: the overlay declines rather than taking the game down.
    pub fn ibda(mut qissa: Qissa, mut qari: IkhtiyarQari) -> Result<Self, KhataTabaqa> {
        let (mursil, mutalaqqi) = std::sync::mpsc::sync_channel(SAA_TABUR);
        let manshura = qissa.manshura();
        let adaad = Arc::new(AdaadKhayt::default());
        let adaad_khayt = Arc::clone(&adaad);
        let khayt = std::thread::Builder::new()
            .name("taarib-tabaqa-qissa".to_owned())
            .spawn(move || {
                while let Ok(risala) = mutalaqqi.recv() {
                    match risala {
                        RisalatQissa::Iltiqat {
                            mintaqa,
                            ism,
                            qaida,
                            lahza_mikro,
                            sura,
                        } => {
                            match qissa.aalij(mintaqa, &ism, qaida, lahza_mikro, &sura, &mut qari) {
                                Ok(_) => {
                                    let _ = adaad_khayt.muaalaja.fetch_add(1, Ordering::Relaxed);
                                },
                                Err(khata) => {
                                    // Kept and never printed: this runs inside
                                    // somebody's game, where there is no
                                    // console, and the panel reads it from here.
                                    // The state lands before the count moves,
                                    // so a reader that saw the count sees why.
                                    let sabab_arabi = khata.arabi();
                                    let sabab = khata.to_string();
                                    if khata.yunhi_al_qiraa() {
                                        *adaad_khayt.hala.lock() =
                                            HalatKhayt::Mutawaqqifa { sabab, sabab_arabi };
                                        adaad_khayt.mutawaqqif.store(true, Ordering::Release);
                                    } else {
                                        *adaad_khayt.hala.lock() =
                                            HalatKhayt::Aabira { sabab, sabab_arabi };
                                    }
                                    let _ = adaad_khayt.akhta.fetch_add(1, Ordering::Release);
                                },
                            }
                        },
                        RisalatQissa::Sath(sath) => qissa.sath_taghayyar(sath),
                        RisalatQissa::IqraAlan => {
                            // The user's re-read is the one thing that reopens
                            // a stopped session: the next pass either succeeds
                            // or stops it again with a fresh reason.
                            *adaad_khayt.hala.lock() = HalatKhayt::Salima;
                            qissa.iqra_alan();
                        },
                        RisalatQissa::Nisyan(mintaqa) => qissa.ansa_mintaqa(mintaqa),
                        RisalatQissa::Tawaqquf => break,
                    }
                }
            })
            .map_err(|sabab| KhataTabaqa::MawridFashil {
                mawrid: "recognition worker thread",
                sabab: sabab.to_string(),
            })?;

        Ok(Self {
            mursil,
            khayt: Some(khayt),
            manshura,
            adaad,
        })
    }

    /// The snapshot handle to give [`Munassiq`].
    #[must_use]
    pub fn manshura(&self) -> Arc<RwLock<LaqtaTarjama>> {
        Arc::clone(&self.manshura)
    }

    /// Posts a capture, drops it when the worker is behind, or refuses it when
    /// recognition has stopped.
    ///
    /// Never blocks — this is called from inside a present hook, and waiting
    /// for a worker there is exactly the stall the whole split exists to
    /// avoid. The refusal costs one atomic read and no allocation, which is
    /// what makes a stopped session cheaper than a running one rather than a
    /// running one that translates nothing.
    #[must_use]
    pub fn adfa(
        &self,
        mintaqa: MuarrifMintaqa,
        ism: &str,
        qaida: QaidatTarjama,
        lahza_mikro: u64,
        sura: SuraMultaqata,
    ) -> HalatDaf {
        if self.adaad.mutawaqqif.load(Ordering::Acquire) {
            let _ = self.adaad.marfuda.fetch_add(1, Ordering::Relaxed);
            return HalatDaf::Rufidat;
        }
        let risala = RisalatQissa::Iltiqat {
            mintaqa,
            ism: ism.to_owned(),
            qaida,
            lahza_mikro,
            sura,
        };
        if self.mursil.try_send(risala).is_ok() {
            let _ = self.adaad.mursala.fetch_add(1, Ordering::Relaxed);
            return HalatDaf::Qubilat;
        }
        let _ = self.adaad.matruka.fetch_add(1, Ordering::Relaxed);
        HalatDaf::Turikat
    }

    /// Tells the worker the surface changed.
    pub fn sath_taghayyar(&self, sath: WasfSath) {
        let _ = self.mursil.try_send(RisalatQissa::Sath(sath));
    }

    /// Tells the worker to clear every picture gate, and reopens a stopped
    /// session.
    ///
    /// The door is reopened only when the request reached the queue: a
    /// re-read the worker never receives would leave the mirror and the
    /// worker's own state disagreeing about whether the session is stopped.
    pub fn iqra_alan(&self) {
        if self.mursil.try_send(RisalatQissa::IqraAlan).is_ok() {
            self.adaad.mutawaqqif.store(false, Ordering::Release);
        }
    }

    /// Tells the worker to forget one region.
    pub fn ansa_mintaqa(&self, mintaqa: MuarrifMintaqa) {
        let _ = self.mursil.try_send(RisalatQissa::Nisyan(mintaqa));
    }

    /// How many captures were posted.
    #[must_use]
    pub fn mursala(&self) -> u64 {
        self.adaad.mursala.load(Ordering::Relaxed)
    }

    /// How many were dropped because the queue was full.
    #[must_use]
    pub fn matruka(&self) -> u64 {
        self.adaad.matruka.load(Ordering::Relaxed)
    }

    /// How many passes completed.
    #[must_use]
    pub fn muaalaja(&self) -> u64 {
        self.adaad.muaalaja.load(Ordering::Relaxed)
    }

    /// How many passes refused.
    ///
    /// Acquire, paired with the worker's release: a caller that reads this and
    /// then [`KhaytQissa::hala`] sees the refusal the count is counting.
    #[must_use]
    pub fn akhta(&self) -> u64 {
        self.adaad.akhta.load(Ordering::Acquire)
    }

    /// How many captures were refused at the door because recognition had
    /// stopped.
    #[must_use]
    pub fn marfuda(&self) -> u64 {
        self.adaad.marfuda.load(Ordering::Relaxed)
    }

    /// The most recent refusal, and whether it ended the session.
    #[must_use]
    pub fn hala(&self) -> HalatKhayt {
        self.adaad.hala.lock().clone()
    }

    /// The sentence the control panel shows about the worker itself.
    ///
    /// The counts first, then which kind of refusal the last one was — the
    /// integer without the sentence is the blank panel this type exists to
    /// prevent.
    #[must_use]
    pub fn wasf(&self) -> String {
        let hala = self.hala();
        let mut wasf = format!(
            "{} capture(s) posted, {} dropped because the worker was behind, {} refused at the \
             door, {} processed, {} refused",
            self.mursala(),
            self.matruka(),
            self.marfuda(),
            self.muaalaja(),
            self.akhta()
        );
        if hala != HalatKhayt::Salima {
            let _ = write!(wasf, "; {}", hala.wasf());
        }
        wasf
    }

    /// The same sentence, in Arabic.
    #[must_use]
    pub fn wasf_arabi(&self) -> String {
        let hala = self.hala();
        let mut wasf = format!(
            "أُرسلت {} التقاطة، وأُسقطت {} لتأخّر العامل، ورُفضت {} عند الباب، وعولجت {}، \
             ورُفضت {} أثناء المعالجة",
            self.mursala(),
            self.matruka(),
            self.marfuda(),
            self.muaalaja(),
            self.akhta()
        );
        if hala != HalatKhayt::Salima {
            let _ = write!(wasf, "؛ {}", hala.wasf_arabi());
        }
        wasf
    }

    /// Stops the worker and waits for it.
    ///
    /// Called on shutdown. A worker that is mid-recognition finishes that pass
    /// first, which is bounded by one recognition and is the difference between
    /// unloading cleanly and unloading a module a thread is still executing in.
    pub fn awqif(&mut self) {
        let _ = self.mursil.try_send(RisalatQissa::Tawaqquf);
        if let Some(khayt) = self.khayt.take() {
            let _ = khayt.join();
        }
    }
}

impl Drop for KhaytQissa {
    fn drop(&mut self) {
        self.awqif();
    }
}

/// The boxes a recognition pass produced, moved onto the surface.
///
/// A free function rather than a method so a caller that already holds both the
/// capture and the recognized lines can make the one conversion the whole
/// pipeline depends on without going through a session. See
/// [`SuraMultaqata::ila_sath`] for why the conversion exists at all.
#[must_use]
pub fn mulahazat_min_maqru(sura: &SuraMultaqata, sutur: &[SatrMaqru]) -> Vec<QiraaMulahaza> {
    sutur
        .iter()
        .map(|satr| QiraaMulahaza::min_maqru(satr, sura.ila_sath(satr.mawdi)))
        .collect()
}
