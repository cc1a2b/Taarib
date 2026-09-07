//! تحديد المقاسات — which text sizes the game actually draws at.
//!
//! Precomputation is a product of two sets: the strings, and the sizes each one
//! is drawn at. The first is known exactly. The second is the subject of this
//! module, and getting it wrong is expensive in both directions — a size that
//! was never drawn puts a whole second copy of a string's glyphs into the atlas
//! for nothing, and a size that *was* drawn and was not discovered is text the
//! game falls back to the runtime path for, forever, on every frame it appears.
//!
//! ## Where a size is allowed to come from
//!
//! Three sources, in descending authority, and the ordering is not a
//! tie-breaking convenience — it is a statement about what each one knows.
//!
//! | source | what it observed | authority |
//! | --- | --- | --- |
//! | [`MasdarHajm::IltiqatZaman`] | a string on screen, in a frame, at a measured size, after the engine's own auto-sizing had already run | direct observation |
//! | [`MasdarHajm::BalaghMuhawwil`] | the size a text component declares in the game's own data, read by static extraction or reported by the adapter | a declaration |
//! | [`MasdarHajm::FahsQudra`] | the capability probe | see below |
//!
//! Runtime capture is strongest because it is the only source that measures
//! what the player saw. A Unity component declaring `fontSize = 24` with
//! auto-sizing enabled draws at whatever fits, which is frequently not 24, and
//! a patch compiled for 24 alone would miss every string the component shrank.
//!
//! ## What the probe can and cannot say, factually
//!
//! [`TaqreerImkaniyat`] — the capability probe this product actually ships —
//! **carries no font size at all.** It reports the engine, the tier, the text
//! systems Taarib will take over, the expected quality and the limits. There is
//! no field in it from which a pixel size could be read.
//!
//! So the probe contributes zero sizes here, and [`IktishafMaqasat::sajjil_fahs`]
//! records that it was consulted rather than pretending it answered. Deriving a
//! size from the tier, or from the text framework, or from a table of "what
//! TextMeshPro usually uses" would be exactly the invention the rest of this
//! module exists to refuse — and it would be an invention wearing the authority
//! of a probe, which is worse than an obvious guess.
//!
//! ## A string with no observed size gets no precomputed layout
//!
//! This is the discipline of the phase, and it mirrors Phase 13 refusing to
//! synthesise a confidence for a translation nobody scored.
//!
//! There is no global default, no interpolation from the sizes of neighbouring
//! strings, no median of the project, and no "the engine's default is probably
//! sixteen". A string nobody measured is reported as
//! [`NassBilaHajm`] and falls to the runtime path, where it is laid out by the
//! real engine at the size the real engine chose. That costs a shaping call the
//! first time it appears and it is *correct*. A precomputed layout at an
//! invented size is wrong at every appearance, is baked into the container, and
//! is invisible in every report — the compiler that produced it believes it,
//! the reviewer sees a layout count that looks complete, and the defect only
//! surfaces as text that is subtly the wrong size on somebody's screen.
//!
//! ## Why quarter pixels, and why *these* quarter pixels
//!
//! Sizes arrive as `f32`. Left as floats they are a set that never closes:
//! 13.0 from one sighting and `13.000_1` from the next are two sizes, two glyph
//! sets, two atlas pages, and two layouts of the same string that differ
//! nowhere a person could see. So they are quantized to integers on a fixed
//! grid, and after that a size set is compared, sorted, deduplicated and hashed
//! as integers, with no float comparison anywhere in this module.
//!
//! The grid is quarter pixels, and specifically the grid
//! [`MiftahShakl::hajm_rubi`] already uses, for two reasons that are worth
//! separating.
//!
//! The first is arithmetic. A quarter is a negative power of two, so
//! `rubi / 4.0` and `hajm * 4.0` are both exact in binary floating point at
//! every magnitude this module accepts. [`HajmMuqannan::biksal`] followed by
//! [`MiftahShakl::jadeed`] is therefore the identity on `hajm_rubi` — the
//! integer that comes back is the integer that went in, with no rounding drift
//! to reason about. On a tenth-pixel grid it would not be: 13.1 is not
//! representable, the round trip would land a step away somewhere, and the
//! layout record and the glyph key would disagree about the size of the same
//! text by a quantum that nothing in the format can express.
//!
//! The second is that three separate records in this product store a size, and
//! all three store it in this unit: [`MiftahShakl::hajm_rubi`] on the atlas key,
//! `SijillTakhtit::hajm_rubi` on the layout record, and the ABI's own layout
//! key. Quantizing on the same grid means the size a layout was computed at and
//! the size the glyph images were rasterized at are *the same `u16`*, not two
//! numbers that ought to agree. "Two floats that ought to be equal and are not"
//! is a class of defect this pipeline simply does not have, and it does not
//! have it because the quantization happens here, once, at the point where a
//! measurement stops being a measurement and becomes a key.
//!
//! ## The report is serialized, never deserialized
//!
//! Every type here derives [`Serialize`] and none derives `Deserialize`, for
//! the reason [`crate::bawwaba`] gives about its proof token: a `Deserialize`
//! impl is a public constructor. [`HajmMuqannan`] guarantees a size inside the
//! range the rasterizer will accept, and a stored JSON object able to mint one
//! outside it would turn a document on disk into a source of sizes that never
//! passed [`HajmMuqannan::min_biksal`].

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_mustalahat::muharrik::TaqreerImkaniyat;
use taarib_mustalahat::nass::{MudkhalNass, NassId};

/// Quarter pixels per pixel: the quantization grid, and the one the atlas key,
/// the layout record and the ABI all already store a size on.
pub const RUBI_LIL_BIKSAL: f32 = 4.0;

/// The smallest size that is a size.
///
/// One pixel, which is the floor `taarib_saff::rasm` refuses below. A component
/// reporting less than a pixel has had a scale factor applied to something that
/// was not a font size, and rasterizing it would produce an empty image that
/// costs an atlas entry.
pub const ADNA_HAJM_BIKSAL: f32 = 1.0;

/// The largest size that is a size.
///
/// A thousand and twenty-four pixels, matching the same rasterizer's ceiling.
/// Well past any interface text; a sighting above it came from a transform, a
/// unit mix-up, or a frame captured mid-zoom, and none of those is a size the
/// compiler should bake a layout for.
pub const AQSA_HAJM_BIKSAL: f32 = 1024.0;

/// [`ADNA_HAJM_BIKSAL`] in quarter pixels.
const ADNA_RUBI: u16 = 4;

/// [`AQSA_HAJM_BIKSAL`] in quarter pixels.
const AQSA_RUBI: u16 = 4096;

/// How many distinct sizes one string may keep.
///
/// Eight. The cost of an extra size for one string is that string's whole glyph
/// set again, and a string genuinely drawn at more than a handful of sizes is
/// rare: a label appears in a menu, in a tooltip and on a HUD, which is three,
/// and a couple of interface scale factors take it to six. Past eight the
/// sightings are almost always one animated or auto-sized component sampled at
/// several points of the same tween, and the honest answer to those is the
/// runtime path rather than eight baked layouts of one moving thing.
///
/// The sizes past the cap are **reported**, not dropped quietly — see
/// [`TaqreerMaqasat::muhmala`]. A contributor who really does need the ninth
/// size can see which one went and why.
pub const AQSA_MAQASAT_LILNASS: usize = 8;

/// How many distinct sizes the whole project may keep.
///
/// Thirty-two. Every distinct size in the union multiplies the *entire*
/// compiled glyph set, because the atlas is keyed by size and a size is a
/// separate image of every glyph any string uses at it. Thirty-two distinct
/// quarter-pixel sizes across one game's interface is already a game whose text
/// scales continuously somewhere; past that the compile stops failing for a
/// reason anybody can act on and starts failing inside the packer with
/// "the page budget ran out", which names the symptom and not the cause.
///
/// Capping here, with a report naming the sizes and how many strings wanted
/// them, is what turns that into a sentence a contributor can do something
/// about: use distance-field pages, or find the component that is scaling.
pub const AQSA_MAQASAT_MASHRU: usize = 32;

/// A pixel size on the quarter-pixel grid.
///
/// The only size type that leaves this module. Constructing one proves the
/// value is finite, positive, inside the rasterizer's range, and on the same
/// grid as [`MiftahShakl::hajm_rubi`] — so everything downstream compares sizes
/// as `u16` and no part of this crate ever compares two sizes as floats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct HajmMuqannan {
    /// The size in quarter pixels, always in `ADNA_RUBI..=AQSA_RUBI`.
    rubi: u16,
}

impl HajmMuqannan {
    /// Quantizes an observed size.
    ///
    /// Rounds to the nearest quarter pixel, exactly as [`MiftahShakl::jadeed`]
    /// does, because the two numbers have to be the same integer and the surest
    /// way to make two roundings agree is to perform the same one.
    ///
    /// # Errors
    ///
    /// [`SababLaHajm`] naming which way the observation failed: not a number at
    /// all, below [`ADNA_HAJM_BIKSAL`], or above [`AQSA_HAJM_BIKSAL`]. The
    /// observed value travels in the error, because "a size was reported and it
    /// was 0.0" and "no size was reported" lead a contributor to two different
    /// places and the report has to keep them apart.
    pub fn min_biksal(hajm: f32) -> Result<Self, SababLaHajm> {
        if !hajm.is_finite() {
            return Err(SababLaHajm::GhayrMuntahi);
        }
        if hajm < ADNA_HAJM_BIKSAL {
            return Err(SababLaHajm::SaghirJiddan { hajm });
        }
        if hajm > AQSA_HAJM_BIKSAL {
            return Err(SababLaHajm::KabirJiddan { hajm });
        }
        let rubi = (hajm * RUBI_LIL_BIKSAL).round();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the two bounds above put hajm in [1.0, 1024.0], so hajm * 4.0 rounds \
                      into [4.0, 4096.0]: non-negative, integral, and far inside u16"
        )]
        let rubi = rubi as u16;
        Ok(Self { rubi })
    }

    /// Wraps a stored quarter-pixel value, refusing one outside the range.
    ///
    /// The only other way to make one, and it validates rather than trusting:
    /// this is what a size read back out of a record goes through, and a record
    /// is a file a person can edit.
    #[must_use]
    pub const fn min_rubi(rubi: u16) -> Option<Self> {
        if rubi < ADNA_RUBI || rubi > AQSA_RUBI {
            return None;
        }
        Some(Self { rubi })
    }

    /// The size in quarter pixels — the integer every record stores.
    #[must_use]
    pub const fn rubi(self) -> u16 {
        self.rubi
    }

    /// The size in pixels.
    ///
    /// Exact: a quarter is a negative power of two, so this division loses
    /// nothing and `min_biksal(x.biksal()) == x` for every value that can
    /// exist.
    #[must_use]
    pub fn biksal(self) -> f32 {
        f32::from(self.rubi) / RUBI_LIL_BIKSAL
    }

    /// The atlas key for one glyph at this size.
    ///
    /// Goes through [`MiftahShakl::jadeed`] rather than building the key by
    /// hand, so that if the atlas ever changes its quantization this stops
    /// compiling instead of silently producing keys the packer will not find.
    #[must_use]
    pub fn miftah(self, khatt: u8, muarrif: u32, namat: NamatSafha, bakat: u8) -> MiftahShakl {
        MiftahShakl::jadeed(khatt, muarrif, self.biksal(), namat, bakat)
    }
}

/// Why an observation did not become a size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "sabab", rename_all = "snake_case")]
pub enum SababLaHajm {
    /// Nothing was ever observed for this string.
    ///
    /// The ordinary case for a game with no runtime capture session, and the
    /// one that sends a string to the runtime path.
    LamYuqas,
    /// A size was reported and it was `NaN` or infinite.
    ///
    /// Almost always a division by a zero-sized rect during a frame where the
    /// component had not been laid out yet.
    GhayrMuntahi,
    /// A size was reported and it was below [`ADNA_HAJM_BIKSAL`].
    SaghirJiddan {
        /// What was reported.
        hajm: f32,
    },
    /// A size was reported and it was above [`AQSA_HAJM_BIKSAL`].
    KabirJiddan {
        /// What was reported.
        hajm: f32,
    },
    /// Every size this string had was dropped by the project-wide cap.
    ///
    /// Distinct from [`SababLaHajm::LamYuqas`] and the distinction is the whole
    /// point: this string *was* measured, and the compile chose not to carry
    /// its sizes. That is a decision a contributor may want to revisit, and it
    /// is not the same news as "nobody ever saw this string".
    HudhifaBiHadd,
}

impl SababLaHajm {
    /// The sentence the compile report shows.
    #[must_use]
    pub fn wasf(self) -> String {
        match self {
            Self::LamYuqas => {
                "no size was ever observed for this string; it falls to the runtime path".to_owned()
            },
            Self::GhayrMuntahi => "the reported size was not a finite number".to_owned(),
            Self::SaghirJiddan { hajm } => {
                format!("the reported size {hajm} is below the {ADNA_HAJM_BIKSAL}px floor")
            },
            Self::KabirJiddan { hajm } => {
                format!("the reported size {hajm} is above the {AQSA_HAJM_BIKSAL}px ceiling")
            },
            Self::HudhifaBiHadd => {
                "every size this string was measured at was dropped by the project size cap"
                    .to_owned()
            },
        }
    }

    /// Whether a size was reported at all.
    ///
    /// What separates "unmeasured" from "measured and unusable" in the report,
    /// which are two different things to fix.
    #[must_use]
    pub const fn qeesa(self) -> bool {
        matches!(
            self,
            Self::GhayrMuntahi | Self::SaghirJiddan { .. } | Self::KabirJiddan { .. }
        )
    }
}

/// Where a size came from.
///
/// Declared strongest first, and [`Ord`] follows the declaration order, so
/// sorting a list of observations ascending puts the most authoritative first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MasdarHajm {
    /// Measured on screen during a runtime capture session.
    ///
    /// The strongest: it is the size the player's own machine drew the string
    /// at, after auto-sizing, after the interface scale factor, after whatever
    /// the game does to its own text that no static reading can predict.
    IltiqatZaman,
    /// Declared by the game's own data, read by static extraction or reported
    /// by the adapter for the component the string is drawn in.
    ///
    /// True as far as it goes, and it does not go as far as the screen: a
    /// declared size is what the component asks for, not necessarily what it
    /// gets.
    BalaghMuhawwil,
    /// The capability probe.
    ///
    /// Present for completeness and for the record, and it never produces a
    /// per-string size — see the module header. [`TaqreerImkaniyat`] has no
    /// size field, and the honest thing to do with a source that does not know
    /// the answer is to record that it was asked.
    FahsQudra,
}

impl MasdarHajm {
    /// The authority rank, three for the strongest.
    #[must_use]
    pub const fn sulta(self) -> u8 {
        match self {
            Self::IltiqatZaman => 3,
            Self::BalaghMuhawwil => 2,
            Self::FahsQudra => 1,
        }
    }

    /// Whether a size from this source may be attached to an individual string.
    ///
    /// False for the probe, which observes the engine rather than any string.
    #[must_use]
    pub const fn yakhussu_nassan(self) -> bool {
        matches!(self, Self::IltiqatZaman | Self::BalaghMuhawwil)
    }

    /// The phrase the report names this source with.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::IltiqatZaman => "measured during runtime capture",
            Self::BalaghMuhawwil => "declared by the game's data or the adapter",
            Self::FahsQudra => "the capability probe",
        }
    }
}

/// One size, where it came from, and how often it was seen.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ShahidHajm {
    /// The quantized size.
    pub hajm: HajmMuqannan,
    /// The strongest source that produced it.
    pub masdar: MasdarHajm,
    /// The observation before quantization, kept so a report can say that
    /// 13.02 and 12.99 were folded into 13.0 rather than leaving a contributor
    /// wondering where a size they can see in their own capture log went.
    pub khaam: f32,
    /// How many observations were folded into this one.
    pub adad: u32,
}

impl ShahidHajm {
    /// Folds a second observation of the same quantized size into this one.
    ///
    /// The strongest source wins, and the raw value is kept from the sighting
    /// that supplied that source — so `khaam` beside a
    /// [`MasdarHajm::IltiqatZaman`] is a number that was really on a screen,
    /// not a declared size that happened to arrive first.
    fn idmij(&mut self, akhar: Self) {
        if akhar.masdar < self.masdar {
            self.masdar = akhar.masdar;
            self.khaam = akhar.khaam;
        }
        self.adad = self.adad.saturating_add(akhar.adad);
    }
}

/// Which cap dropped a size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HaddMaqasat {
    /// [`AQSA_MAQASAT_LILNASS`], the per-string cap.
    LilNass,
    /// [`AQSA_MAQASAT_MASHRU`], the project-wide cap.
    LilMashru,
}

impl HaddMaqasat {
    /// The cap's value.
    #[must_use]
    pub const fn qeema(self) -> usize {
        match self {
            Self::LilNass => AQSA_MAQASAT_LILNASS,
            Self::LilMashru => AQSA_MAQASAT_MASHRU,
        }
    }

    /// The constant's name, so a report names something a contributor can grep.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::LilNass => "AQSA_MAQASAT_LILNASS",
            Self::LilMashru => "AQSA_MAQASAT_MASHRU",
        }
    }
}

/// A size that was discovered and then dropped by a cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HajmMuhmal {
    /// The size.
    pub hajm: HajmMuqannan,
    /// The strongest source that produced it.
    pub masdar: MasdarHajm,
    /// How many strings wanted it.
    pub nusus: u32,
    /// Which cap dropped it.
    pub hadd: HaddMaqasat,
}

/// The sizes one string is precomputed at.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MaqasatNass {
    /// The string.
    pub nass: NassId,
    /// Its sizes, ascending, deduplicated on the quarter-pixel grid.
    pub shuhud: Vec<ShahidHajm>,
    /// What the per-string cap dropped, for this string.
    pub muhmala: Vec<ShahidHajm>,
}

impl MaqasatNass {
    /// The sizes, ascending.
    #[must_use]
    pub fn ahjam(&self) -> Vec<HajmMuqannan> {
        self.shuhud.iter().map(|shahid| shahid.hajm).collect()
    }

    /// The strongest source behind any of this string's sizes.
    #[must_use]
    pub fn aqwa_masdar(&self) -> Option<MasdarHajm> {
        self.shuhud.iter().map(|shahid| shahid.masdar).min()
    }

    /// Whether any size survived the caps.
    #[must_use]
    pub const fn khali(&self) -> bool {
        self.shuhud.is_empty()
    }
}

/// A string that will not be precomputed, and why.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct NassBilaHajm {
    /// The string.
    pub nass: NassId,
    /// Why it has no size.
    pub sabab: SababLaHajm,
}

/// What size discovery found.
///
/// Serialized into the package so a reviewer reads the same numbers the
/// compiler acted on, rather than re-deriving them from a capture session that
/// may not still exist.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaqreerMaqasat {
    maqasat: BTreeMap<NassId, MaqasatNass>,
    ittihad: Vec<HajmMuqannan>,
    amma: Vec<ShahidHajm>,
    bila_hajm: Vec<NassBilaHajm>,
    muhmala: Vec<HajmMuhmal>,
    fuhus: u32,
}

impl TaqreerMaqasat {
    /// The sizes one string is precomputed at.
    ///
    /// [`None`] means the string has no precomputed layout at all, which is a
    /// different answer from an empty set and is why this is not a `&[]`.
    #[must_use]
    pub fn maqasat_nass(&self, nass: NassId) -> Option<&MaqasatNass> {
        self.maqasat.get(&nass)
    }

    /// Every string that has at least one size, in identity order.
    ///
    /// Ordered rather than hashed because the whole compile is deterministic:
    /// the order layouts are produced in decides the order they are packed in,
    /// and a hash map's iteration would make two compiles of one project
    /// produce two different containers.
    pub fn maqasat(&self) -> impl Iterator<Item = &MaqasatNass> {
        self.maqasat.values()
    }

    /// The union of every string's sizes, ascending.
    ///
    /// What the atlas is sized against: a glyph is rasterized once per size,
    /// and this is the set of sizes.
    #[must_use]
    pub fn ittihad(&self) -> &[HajmMuqannan] {
        &self.ittihad
    }

    /// Sizes the engine draws at that belong to no individual string.
    ///
    /// An engine-wide size an adapter declares — the one a Ren'Py or VX Ace
    /// install is configured with — is real, and it is not an observation of
    /// any string, so it never produces a layout. It is still worth carrying:
    /// it is a size the *runtime* path will be asked for, which is what the
    /// residency budget in [`crate::takhtit`] is planned against.
    #[must_use]
    pub fn amma(&self) -> &[ShahidHajm] {
        &self.amma
    }

    /// Every size the runtime may be asked for: the union plus the
    /// engine-wide sizes, ascending and deduplicated.
    #[must_use]
    pub fn ahjam_iqama(&self) -> Vec<HajmMuqannan> {
        let mut ahjam = self.ittihad.clone();
        ahjam.extend(self.amma.iter().map(|shahid| shahid.hajm));
        ahjam.sort_unstable();
        ahjam.dedup();
        ahjam
    }

    /// Strings that will not be precomputed, and why.
    #[must_use]
    pub fn bila_hajm(&self) -> &[NassBilaHajm] {
        &self.bila_hajm
    }

    /// Sizes a cap dropped, so nothing was truncated silently.
    #[must_use]
    pub fn muhmala(&self) -> &[HajmMuhmal] {
        &self.muhmala
    }

    /// How many capability probes were consulted.
    ///
    /// Carried because zero and "consulted and it had nothing to say" are
    /// different facts, and only one of them means the probe was skipped.
    #[must_use]
    pub const fn fuhus(&self) -> u32 {
        self.fuhus
    }

    /// How many strings have at least one size.
    #[must_use]
    pub fn adad_maqisa(&self) -> usize {
        self.maqasat.len()
    }

    /// How many string-and-size pairs precomputation will produce.
    ///
    /// The real size of the layout stage, and the number a progress bar should
    /// be scaled against — not the string count, which understates it by
    /// however many sizes the project draws at.
    #[must_use]
    pub fn adad_takhtitat(&self) -> usize {
        self.maqasat.values().map(|maqas| maqas.shuhud.len()).sum()
    }

    /// The sentence the compile report shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} string(s) measured at {} distinct size(s), giving {} layout(s) to compute; \
             {} string(s) have no observed size and fall to the runtime path; {} size(s) were \
             dropped by a cap; {} engine-wide size(s) recorded for residency; {} capability \
             probe(s) consulted, contributing no size",
            self.maqasat.len(),
            self.ittihad.len(),
            self.adad_takhtitat(),
            self.bila_hajm.len(),
            self.muhmala.len(),
            self.amma.len(),
            self.fuhus
        )
    }
}

/// Collects size observations and turns them into a per-string size set.
///
/// Observations arrive from several places and in no particular order, and the
/// caps can only be applied once everything is in — so this accumulates, and
/// [`IktishafMaqasat::ahsi`] is where the answer is produced.
#[derive(Debug, Default)]
pub struct IktishafMaqasat {
    /// Per string, per quantized size, the folded observation.
    shuhud: BTreeMap<NassId, BTreeMap<u16, ShahidHajm>>,
    /// Engine-wide sizes, which belong to no string.
    amma: BTreeMap<u16, ShahidHajm>,
    /// Strings whose only observation was unusable, and how.
    marfuda: BTreeMap<NassId, SababLaHajm>,
    /// How many capability probes were consulted.
    fuhus: u32,
}

impl IktishafMaqasat {
    /// An empty discovery pass.
    #[must_use]
    pub fn jadeed() -> Self {
        Self::default()
    }

    /// Reads whatever size a string's own record carries.
    ///
    /// The source is taken from the string's extraction provenance rather than
    /// guessed: [`taarib_mustalahat::nass::MasdarIstikhraj::multaqat`] is true
    /// exactly when runtime capture contributed to this entry, and the size in
    /// [`taarib_mustalahat::nass::QuyudNass::hajm_khatt`] came out of the same
    /// merge that set it. A string whose provenance says capture never ran
    /// carries a declared size, not a measured one, and is recorded as such.
    ///
    /// A string with no size at all is recorded as unmeasured here rather than
    /// left absent, so that [`IktishafMaqasat::ahsi`] can tell "never offered"
    /// from "offered and refused" without re-reading the table.
    pub fn sajjil_mudkhal(&mut self, mudkhal: &MudkhalNass) {
        let Some(hajm) = mudkhal.quyud.hajm_khatt else {
            return;
        };
        let masdar = if mudkhal.masdar_istikhraj.multaqat() {
            MasdarHajm::IltiqatZaman
        } else {
            MasdarHajm::BalaghMuhawwil
        };
        self.sajjil_nass(mudkhal.id, hajm, masdar);
    }

    /// Records a size a runtime capture measured for one string.
    ///
    /// Separate from [`IktishafMaqasat::sajjil_mudkhal`] because a capture
    /// session sees a string at several sizes across a play-through and the
    /// merged record keeps only one; feeding the sightings in directly is what
    /// gives a string more than one precomputed layout.
    pub fn sajjil_iltiqat(&mut self, nass: NassId, hajm: f32) {
        self.sajjil_nass(nass, hajm, MasdarHajm::IltiqatZaman);
    }

    /// Records a size the adapter reports for one string's component.
    pub fn sajjil_muhawwil(&mut self, nass: NassId, hajm: f32) {
        self.sajjil_nass(nass, hajm, MasdarHajm::BalaghMuhawwil);
    }

    /// Records a size the adapter declares for the engine as a whole.
    ///
    /// **Never attached to a string.** A VX Ace install configured to draw at
    /// twenty-four is a fact about the runtime, not an observation of any
    /// particular line, and attaching it to every string would be exactly the
    /// global default this module refuses. It is kept for the residency budget,
    /// where a size the runtime will be asked for is precisely the input
    /// wanted.
    pub fn sajjil_muhawwil_amm(&mut self, hajm: f32) {
        if let Ok(muqannan) = HajmMuqannan::min_biksal(hajm) {
            let shahid = ShahidHajm {
                hajm: muqannan,
                masdar: MasdarHajm::BalaghMuhawwil,
                khaam: hajm,
                adad: 1,
            };
            self.amma
                .entry(muqannan.rubi())
                .and_modify(|mawjud| mawjud.idmij(shahid))
                .or_insert(shahid);
        }
    }

    /// Consults the capability probe.
    ///
    /// Records that it was asked and takes nothing from it, because there is
    /// nothing in it to take: [`TaqreerImkaniyat`] describes the engine, the
    /// tier, the text systems and the limits, and holds no font size anywhere.
    /// The argument is read — a refused game is not counted as a consultation,
    /// since nothing was probed — and then deliberately discarded.
    ///
    /// Written as a method that does almost nothing rather than left out
    /// entirely, so that the report can say "the probe was consulted and
    /// contributed no size" instead of leaving a reader to wonder whether the
    /// third source was forgotten.
    pub const fn sajjil_fahs(&mut self, taqreer: &TaqreerImkaniyat) {
        if taqreer.marfuda {
            return;
        }
        self.fuhus = self.fuhus.saturating_add(1);
    }

    /// Folds one observation into a string's set.
    fn sajjil_nass(&mut self, nass: NassId, hajm: f32, masdar: MasdarHajm) {
        match HajmMuqannan::min_biksal(hajm) {
            Ok(muqannan) => {
                let shahid = ShahidHajm {
                    hajm: muqannan,
                    masdar,
                    khaam: hajm,
                    adad: 1,
                };
                let _ = self
                    .shuhud
                    .entry(nass)
                    .or_default()
                    .entry(muqannan.rubi())
                    .and_modify(|mawjud| mawjud.idmij(shahid))
                    .or_insert(shahid);
                // A string that once produced a usable size is no longer a
                // refusal, whatever an earlier bad sighting said.
                let _ = self.marfuda.remove(&nass);
            },
            Err(sabab) => {
                if !self.shuhud.contains_key(&nass) {
                    let _ = self.marfuda.insert(nass, sabab);
                }
            },
        }
    }

    /// How many strings currently have at least one usable size.
    #[must_use]
    pub fn adad(&self) -> usize {
        self.shuhud.len()
    }

    /// Applies the caps and produces the report.
    ///
    /// `nusus` is the whole string table, and it is needed rather than implied
    /// because the most important thing in the report is the list of strings
    /// that have *no* size — which cannot be derived from a map of the strings
    /// that do.
    #[must_use]
    pub fn ahsi(&self, nusus: &[MudkhalNass]) -> TaqreerMaqasat {
        let mut maqasat: BTreeMap<NassId, MaqasatNass> = BTreeMap::new();
        let mut muhmala_lilnass: BTreeMap<u16, HajmMuhmal> = BTreeMap::new();

        // Pass one: the per-string cap. Keeps the most authoritative sizes,
        // then the most frequently seen, then the smallest — smallest last
        // because it is a tie-break and not a preference, and it only has to be
        // deterministic.
        for (nass, bil_hajm) in &self.shuhud {
            let mut shuhud: Vec<ShahidHajm> = bil_hajm.values().copied().collect();
            shuhud.sort_by(|awwal, thani| {
                awwal
                    .masdar
                    .cmp(&thani.masdar)
                    .then(thani.adad.cmp(&awwal.adad))
                    .then(awwal.hajm.cmp(&thani.hajm))
            });
            let muhmala: Vec<ShahidHajm> = shuhud.split_off(shuhud.len().min(AQSA_MAQASAT_LILNASS));
            shuhud.sort_by_key(|shahid| shahid.hajm);

            for shahid in &muhmala {
                muhmala_lilnass
                    .entry(shahid.hajm.rubi())
                    .and_modify(|muhmal| muhmal.nusus = muhmal.nusus.saturating_add(1))
                    .or_insert(HajmMuhmal {
                        hajm: shahid.hajm,
                        masdar: shahid.masdar,
                        nusus: 1,
                        hadd: HaddMaqasat::LilNass,
                    });
            }

            let _ = maqasat.insert(
                *nass,
                MaqasatNass {
                    nass: *nass,
                    shuhud,
                    muhmala,
                },
            );
        }

        // Pass two: the project cap, over what pass one kept.
        let majmu = jami_ahjam(&maqasat);
        let (mahfuza, masqata) = qassim_ittihad(&majmu);

        let mut muhmala: Vec<HajmMuhmal> = muhmala_lilnass.into_values().collect();
        muhmala.extend(masqata.iter().copied());
        muhmala.sort_by_key(|muhmal| (muhmal.hajm, muhmal.hadd));

        // Pass three: remove the dropped sizes from every string that wanted
        // one. A string left holding a size the atlas does not carry would ship
        // a layout referring to glyph images nobody rasterized, which is the
        // one failure mode worse than having no layout.
        let mut mufragha: BTreeSet<NassId> = BTreeSet::new();
        maqasat.retain(|nass, maqas| {
            maqas.shuhud.retain(|shahid| mahfuza.contains(&shahid.hajm));
            if maqas.shuhud.is_empty() {
                let _ = mufragha.insert(*nass);
                return false;
            }
            true
        });

        let mut bila_hajm: Vec<NassBilaHajm> = mufragha
            .iter()
            .map(|nass| NassBilaHajm {
                nass: *nass,
                sabab: SababLaHajm::HudhifaBiHadd,
            })
            .collect();
        for mudkhal in nusus {
            if maqasat.contains_key(&mudkhal.id) || mufragha.contains(&mudkhal.id) {
                continue;
            }
            let sabab = self
                .marfuda
                .get(&mudkhal.id)
                .copied()
                .unwrap_or(SababLaHajm::LamYuqas);
            bila_hajm.push(NassBilaHajm {
                nass: mudkhal.id,
                sabab,
            });
        }
        bila_hajm.sort_by_key(|bila| bila.nass);
        bila_hajm.dedup_by_key(|bila| bila.nass);

        let mut amma: Vec<ShahidHajm> = self.amma.values().copied().collect();
        amma.sort_by_key(|shahid| shahid.hajm);

        TaqreerMaqasat {
            maqasat,
            ittihad: mahfuza,
            amma,
            bila_hajm,
            muhmala,
            fuhus: self.fuhus,
        }
    }
}

/// Every size any string kept, with its strongest source and how many strings
/// wanted it.
fn jami_ahjam(
    maqasat: &BTreeMap<NassId, MaqasatNass>,
) -> BTreeMap<u16, (HajmMuqannan, MasdarHajm, u32)> {
    let mut majmu: BTreeMap<u16, (HajmMuqannan, MasdarHajm, u32)> = BTreeMap::new();
    for maqas in maqasat.values() {
        for shahid in &maqas.shuhud {
            majmu
                .entry(shahid.hajm.rubi())
                .and_modify(|(_, masdar, nusus)| {
                    *masdar = (*masdar).min(shahid.masdar);
                    *nusus = nusus.saturating_add(1);
                })
                .or_insert((shahid.hajm, shahid.masdar, 1));
        }
    }
    majmu
}

/// Splits the union at [`AQSA_MAQASAT_MASHRU`], keeping the sizes the project
/// most depends on.
///
/// Ranked by authority first, then by how many strings wanted the size, then by
/// the size itself. Authority first because a size thirty strings declared and
/// nothing was ever seen at is a weaker claim on the atlas than one a capture
/// session measured twice: the first may be a default the engine overrides
/// everywhere, and the second was on a screen.
fn qassim_ittihad(
    majmu: &BTreeMap<u16, (HajmMuqannan, MasdarHajm, u32)>,
) -> (Vec<HajmMuqannan>, Vec<HajmMuhmal>) {
    let mut murattaba: Vec<(HajmMuqannan, MasdarHajm, u32)> = majmu.values().copied().collect();
    if murattaba.len() <= AQSA_MAQASAT_MASHRU {
        let mut mahfuza: Vec<HajmMuqannan> =
            murattaba.into_iter().map(|(hajm, _, _)| hajm).collect();
        mahfuza.sort_unstable();
        return (mahfuza, Vec::new());
    }

    murattaba.sort_by(|awwal, thani| {
        awwal
            .1
            .cmp(&thani.1)
            .then(thani.2.cmp(&awwal.2))
            .then(awwal.0.cmp(&thani.0))
    });
    let masqata_khaam = murattaba.split_off(AQSA_MAQASAT_MASHRU);

    let mut mahfuza: Vec<HajmMuqannan> = murattaba.into_iter().map(|(hajm, _, _)| hajm).collect();
    mahfuza.sort_unstable();

    let masqata: Vec<HajmMuhmal> = masqata_khaam
        .into_iter()
        .map(|(hajm, masdar, nusus)| HajmMuhmal {
            hajm,
            masdar,
            nusus,
            hadd: HaddMaqasat::LilMashru,
        })
        .collect();
    (mahfuza, masqata)
}
