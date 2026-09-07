//! المناطق — where the overlay looks, when it is allowed to look there, and how
//! a player tells it.
//!
//! A region is a rectangle on the screen that Taarib captures, reads and draws
//! over. Everything about this module follows from one decision made in
//! [`crate::wajiha`]: a region is stored in **normalized** coordinates, zero to
//! one on both axes, never in pixels. A player draws a box around a dialogue
//! box at 1920×1080, alt-tabs, switches to borderless at 2560×1440, and the box
//! is still around the dialogue box. Storing pixels would have put it somewhere
//! in the upper-left quarter and the player would have to draw it again — once
//! per resolution, once per monitor, once per fullscreen toggle.
//!
//! ## Three rules, and one of them is expensive
//!
//! [`QaidatTarjama`] is per region because the cost is per region. A dialogue
//! box that changes twice a minute and a subtitle strip that changes every
//! second do not deserve the same polling, and a region on
//! [`QaidatTarjama::Mustamirra`] costs a capture, a recognition pass and a
//! translation request on every interval whether anything changed or not. The
//! descriptor says so in the same plain terms
//! [`crate::wajiha::MeezaniyatItar::wasf`] uses, because a user choosing the
//! expensive option should be choosing it, not discovering it.
//!
//! ## The editor is a state machine, not a callback soup
//!
//! [`MuharrirManatiq`] holds one drag at a time and nothing else. It is fed
//! normalized points by whatever the input hook is, it answers what the
//! renderer should draw right now, and on release it returns exactly one
//! outcome. The single behaviour it is most careful about is the one that looks
//! like nothing: a click with no drag. That produces a zero-area rectangle,
//! which [`crate::wajiha::MustatilNisbi::salih`] already refuses, and an editor
//! that created the region anyway would leave an invisible entry in the list
//! that captures nothing and cannot be grabbed to resize. So a degenerate drag
//! cancels and says why.
//!
//! ## Detection is a decay, not a ring buffer
//!
//! [`IktishafTilqai`] answers "which of these text blocks is furniture?" A
//! dialogue box is in the same place every time it appears. A damage number is
//! not, and neither is a floating quest marker. The distinction is stability
//! across samples, and it is tracked with an explicit generation counter and a
//! decay rather than with a fixed-size window of the last N frames.
//!
//! That choice is not stylistic. A ring buffer of the last N samples forgets
//! silently: a dialogue box that has been on screen for a minute and a dialogue
//! box that has been on screen for N frames are indistinguishable inside it, and
//! a block that flickers off for one frame in the middle of the window is
//! scored exactly like a block that was never there. A counter that climbs while
//! a block is seen and falls by a fixed amount while it is not gives both a
//! memory longer than the window and a tolerance for a single dropped frame,
//! with the whole history compressed into one number per candidate instead of N
//! rectangles per candidate.
//!
//! ## Refusals are values, not errors
//!
//! Adding a region with a name that is already taken is not a failure of the
//! machine and does not belong in [`KhataTabaqa`], which carries permanent codes
//! that go into bug reports. It is an answer the editor shows inline, next to
//! the field, naming the region that already holds the name. So the mutating
//! methods here return [`NatijatMintaqa`] and only the disk touches
//! [`KhataTabaqa`]. `lawhat_tahakkum` does the same thing for a keyboard chord
//! that is already bound.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use taarib_mustalahat::luba::LubaId;
use taarib_usus::khata::{Khata, Natija};
use taarib_usus::masarat;
use taarib_usus::mukhattat::{self, DhuMukhattat};

use crate::khata::KhataTabaqa;
use crate::wajiha::{MustatilBiksel, MustatilNisbi, WasfSath};

/// The file one game's regions are stored in.
pub const ISM_MALAF: &str = "manatiq.json";

/// The most regions one game may hold.
///
/// Sixty-four. The ceiling is not about memory — sixty-four rectangles is
/// nothing — it is about the frame budget: every enabled region on a timed rule
/// is a capture and a recognition pass, and a set large enough to exhaust
/// [`crate::wajiha::MeezaniyatItar`] every frame would present as "the overlay
/// makes my game stutter" rather than as "I added too many regions".
pub const AQSA_MANATIQ: usize = 64;

/// The longest region name, in characters.
pub const AQSA_TUL_ISM: usize = 64;

/// The fastest any region may be polled, in milliseconds.
///
/// Fifty, which is twenty times a second. Below that a capture, a conversion
/// and a recognition pass have not finished before the next one is due, so the
/// only thing a smaller number buys is a queue that grows until the region is
/// disabled.
pub const ADNA_FASILA_MILLI: u32 = 50;

/// The slowest a region may be polled, in milliseconds.
pub const AQSA_FASILA_MILLI: u32 = 60_000;

/// A region's stable identity within one game's set.
///
/// Monotonic within a set and never reused, including after a region is
/// deleted. `sijill_qira` stores this number against every line it recorded, so
/// reusing an identity would silently reattribute a deleted region's history to
/// whatever was created next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MuarrifMintaqa(u64);

impl MuarrifMintaqa {
    /// Wraps an identity read back from disk or from a history entry.
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

impl std::fmt::Display for MuarrifMintaqa {
    fn fmt(&self, mukhraj: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(mukhraj, "#{}", self.0)
    }
}

/// When a region's contents are read and translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaidatTarjama {
    /// Poll on the interval, and translate only when the pixels changed.
    ///
    /// The default, and the right answer for a dialogue box.
    IndaTaghyeer,

    /// Read nothing until the user presses translate-now.
    ///
    /// Costs exactly zero while it is not being asked. For a region over a menu
    /// the player opens twice an hour, this is the honest setting.
    IndaTalab,

    /// Poll on the interval and translate every time, changed or not.
    ///
    /// The expensive one. See [`QaidatTarjama::wasf`].
    Mustamirra,
}

impl QaidatTarjama {
    /// The name used in the region file, in log lines and in the report.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::IndaTaghyeer => "on change",
            Self::IndaTalab => "on request",
            Self::Mustamirra => "continuous",
        }
    }

    /// The label the in-game panel shows, in Arabic.
    ///
    /// Separate from [`QaidatTarjama::ism`] on purpose: one of these two strings
    /// is read by a person playing a game in Arabic and the other is read by
    /// somebody holding a log file, and translating the log would make it
    /// unsearchable for the contributor it exists for.
    #[must_use]
    pub const fn unwan(self) -> &'static str {
        match self {
            Self::IndaTaghyeer => "عند التغيّر",
            Self::IndaTalab => "عند الطلب",
            Self::Mustamirra => "مستمرّة",
        }
    }

    /// What this rule costs, stated the way the frame budget states its cost.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::IndaTaghyeer => {
                "One capture per interval. The captured pixels are compared against the \
                 previous capture and recognition runs only when they differ, so a dialogue \
                 box sitting still costs a capture and nothing else."
            },
            Self::IndaTalab => {
                "Nothing at all until you press translate-now, then one capture, one \
                 recognition pass and one translation. A region on this rule does not appear \
                 in the frame budget between requests."
            },
            Self::Mustamirra => {
                "The expensive one: a capture, a recognition pass and a translation request \
                 every interval, whether the text changed or not, for as long as the game is \
                 running. Use it for a region whose pixels change constantly and whose text \
                 does not — a scrolling combat log — and expect it in the frame budget."
            },
        }
    }

    /// Whether a scheduler polls this region on a timer at all.
    #[must_use]
    pub const fn ala_muaqqit(self) -> bool {
        matches!(self, Self::IndaTaghyeer | Self::Mustamirra)
    }

    /// Whether recognition runs even when the captured pixels are unchanged.
    #[must_use]
    pub const fn yatajahal_muqarana(self) -> bool {
        matches!(self, Self::Mustamirra)
    }

    /// Every rule, in the order the panel lists them: cheapest first.
    #[must_use]
    pub const fn jamee() -> [Self; 3] {
        [Self::IndaTalab, Self::IndaTaghyeer, Self::Mustamirra]
    }
}

/// Serde for [`MustatilNisbi`], which derives none of it where it is defined.
///
/// The drawing types in [`crate::wajiha`] are deliberately serde-free: a
/// renderer struct that can be deserialized is a renderer struct that somebody
/// will eventually deserialize straight out of a game's own data files. The
/// on-disk shape of a rectangle therefore lives here, beside the only thing in
/// this crate that writes one to disk.
mod hifz_mustatil {
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::wajiha::MustatilNisbi;

    #[derive(Serialize, Deserialize)]
    struct Khaam {
        yasar: f32,
        aala: f32,
        ard: f32,
        irtifa: f32,
    }

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "`&MustatilNisbi` rather than by value even though the type is small and \
                  `Copy`: serde's `with` attribute calls this with a reference to the field \
                  and no other signature compiles"
    )]
    pub(super) fn serialize<S>(qeema: &MustatilNisbi, mukhraj: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Khaam {
            yasar: qeema.yasar,
            aala: qeema.aala,
            ard: qeema.ard,
            irtifa: qeema.irtifa,
        }
        .serialize(mukhraj)
    }

    pub(super) fn deserialize<'de, D>(madkhal: D) -> Result<MustatilNisbi, D::Error>
    where
        D: Deserializer<'de>,
    {
        let khaam = Khaam::deserialize(madkhal)?;
        // A non-finite edge is rejected here rather than downstream. JSON has no
        // literal for one, but a hand-edited file with 1e400 in it parses as f64
        // infinity and narrows to f32 infinity, and an infinite edge poisons
        // every comparison the editor and the detector make afterwards — a
        // region that can never be hit-tested and never be drawn.
        if !khaam.yasar.is_finite()
            || !khaam.aala.is_finite()
            || !khaam.ard.is_finite()
            || !khaam.irtifa.is_finite()
        {
            return Err(D::Error::custom(
                "a region rectangle has an edge that is not a finite number",
            ));
        }
        Ok(MustatilNisbi {
            yasar: khaam.yasar,
            aala: khaam.aala,
            ard: khaam.ard,
            irtifa: khaam.irtifa,
        })
    }
}

/// One region: a rectangle, a rule, and whether it is on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mintaqa {
    /// Stable within this game's set, never reused.
    pub muarrif: MuarrifMintaqa,

    /// What the player called it.
    ///
    /// Shown in the panel and in the history, so it is the string a user reads
    /// when they are working out which region missed a line.
    pub ism: String,

    /// Where it is, normalized, so it survives a resolution change.
    #[serde(with = "hifz_mustatil")]
    pub mustatil: MustatilNisbi,

    /// When it is read.
    pub qaida: QaidatTarjama,

    /// Whether the scheduler considers it at all.
    ///
    /// Distinct from deleting it. A player turns a region off while a cutscene
    /// is running and back on afterwards; making that a delete would lose the
    /// rectangle they drew.
    pub mumakkana: bool,

    /// A per-region floor on the polling interval, in milliseconds.
    ///
    /// [`None`] means the set's default applies. It exists because the right
    /// interval is a property of the region and not of the game: a subtitle
    /// strip wants two hundred milliseconds and a quest log wants five seconds,
    /// and forcing both to one number means one of them is wrong.
    pub fasila_milli: Option<u32>,
}

impl Mintaqa {
    /// A region with the default rule, enabled, with no interval override.
    #[must_use]
    pub fn jadeeda(
        muarrif: MuarrifMintaqa,
        ism: impl Into<String>,
        mustatil: MustatilNisbi,
    ) -> Self {
        Self {
            muarrif,
            ism: ism.into(),
            mustatil,
            qaida: QaidatTarjama::IndaTaghyeer,
            mumakkana: true,
            fasila_milli: None,
        }
    }

    /// Whether this region is worth capturing at all.
    #[must_use]
    pub fn salih(&self) -> bool {
        !self.ism.trim().is_empty() && self.mustatil.salih()
    }

    /// The interval this region is actually polled at, in milliseconds.
    ///
    /// The override when there is one, the set's default otherwise, clamped
    /// into [`ADNA_FASILA_MILLI`]..=[`AQSA_FASILA_MILLI`] in both cases — a
    /// stored value from an older build, or a hand-edited file, does not get to
    /// ask for a hundred captures a second.
    #[must_use]
    pub const fn fasila_faaila(&self, iftiradi_milli: u32) -> u32 {
        let khaam = match self.fasila_milli {
            Some(qeema) => qeema,
            None => iftiradi_milli,
        };
        if khaam < ADNA_FASILA_MILLI {
            ADNA_FASILA_MILLI
        } else if khaam > AQSA_FASILA_MILLI {
            AQSA_FASILA_MILLI
        } else {
            khaam
        }
    }

    /// Whether a timer-driven scheduler should be looking at this region.
    #[must_use]
    pub const fn ala_muaqqit(&self) -> bool {
        self.mumakkana && self.qaida.ala_muaqqit()
    }

    /// This region in surface pixels, or [`None`] when it lands on nothing.
    #[must_use]
    pub fn fi_bikselat(&self, sath: WasfSath) -> Option<MustatilBiksel> {
        self.mustatil.fi_bikselat(sath.ard, sath.irtifa)
    }

    /// The one-line summary the panel's region page shows.
    #[must_use]
    pub fn satr(&self) -> String {
        format!(
            "{} — {} — {}",
            self.ism,
            self.qaida.unwan(),
            if self.mumakkana {
                "مفعّلة"
            } else {
                "موقوفة"
            }
        )
    }
}

/// The answer to a request to change the region set.
///
/// Not an error type. Every variant here is something the editor shows beside
/// the control the user just touched — "that name belongs to this region", "you
/// did not drag far enough" — and none of them is a fault worth a permanent
/// code in a bug report. [`KhataTabaqa`] is reserved for the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NatijatMintaqa {
    /// It happened, and this is the region it happened to.
    Tammat(MuarrifMintaqa),

    /// The name was empty or only whitespace.
    IsmFarigh,

    /// The name was longer than [`AQSA_TUL_ISM`] characters.
    IsmTaweel {
        /// The ceiling that was exceeded.
        saqf: usize,
    },

    /// Another region already has that name.
    IsmMakrur {
        /// Which region holds it.
        sahib: MuarrifMintaqa,
        /// The name, as it was offered.
        ism: String,
    },

    /// The rectangle has no area, so it would capture nothing.
    LaMisaha,

    /// The set is already at [`AQSA_MANATIQ`].
    TajawuzSaqf {
        /// The ceiling.
        saqf: usize,
    },

    /// No region in this set has that identity.
    ///
    /// Reached when the panel acts on a region another thread just deleted,
    /// which is ordinary rather than exceptional.
    GhayrMawjuda(MuarrifMintaqa),
}

impl NatijatMintaqa {
    /// Whether the change was made.
    #[must_use]
    pub const fn najahat(&self) -> bool {
        matches!(self, Self::Tammat(_))
    }

    /// The region the change applied to, when it applied.
    #[must_use]
    pub const fn muarrif(&self) -> Option<MuarrifMintaqa> {
        match self {
            Self::Tammat(muarrif) => Some(*muarrif),
            _ => None,
        }
    }

    /// The sentence the editor shows beside the control, in Arabic.
    #[must_use]
    pub fn unwan(&self) -> String {
        match self {
            Self::Tammat(muarrif) => format!("تمّ ({muarrif})."),
            Self::IsmFarigh => "لا يمكن ترك اسم المنطقة فارغًا.".to_owned(),
            Self::IsmTaweel { saqf } => {
                format!("اسم المنطقة أطول من {saqf} حرفًا.")
            },
            Self::IsmMakrur { sahib, ism } => {
                format!("الاسم ({ism}) مستعمل بالفعل للمنطقة ({sahib}).")
            },
            Self::LaMisaha => "المستطيل بلا مساحة، ولن تُلتقط منه صورة. اسحب مسافة أكبر.".to_owned(),
            Self::TajawuzSaqf { saqf } => {
                format!("بلغ عدد المناطق حدّه الأقصى ({saqf}).")
            },
            Self::GhayrMawjuda(muarrif) => format!("لا توجد منطقة بالمعرّف ({muarrif})."),
        }
    }

    /// The same sentence in English, for a log line and the report.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::Tammat(muarrif) => format!("applied to region {muarrif}"),
            Self::IsmFarigh => "a region name cannot be empty".to_owned(),
            Self::IsmTaweel { saqf } => format!("a region name is at most {saqf} characters"),
            Self::IsmMakrur { sahib, ism } => {
                format!("the name {ism} is already held by region {sahib}")
            },
            Self::LaMisaha => "the rectangle has no area and would capture nothing".to_owned(),
            Self::TajawuzSaqf { saqf } => format!("this game already holds {saqf} regions"),
            Self::GhayrMawjuda(muarrif) => format!("no region {muarrif} is in this set"),
        }
    }
}

/// A region name reduced to the form two names are compared in.
///
/// Trimmed, internal whitespace runs collapsed to one space, and lowercased.
/// The comparison is deliberately looser than equality: a user who types
/// `Dialogue` and then `dialogue  ` has made one region twice as far as they are
/// concerned, and telling them the name is free would produce a list with two
/// entries they cannot tell apart. Lowercasing is a no-op for Arabic names,
/// which is fine — it is there for the Latin ones.
#[must_use]
pub fn wahhid_ism(ism: &str) -> String {
    ism.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// One game's regions, as they are held in memory and written to disk.
///
/// Serialized with a schema version through [`taarib_usus::mukhattat`] and
/// written through [`taarib_usus::masarat::kitaba_dharra`], so a set is never
/// half-written and never read back under a shape this build does not know.
/// There is no second write path: a region file that got truncated by a crash
/// during a save is a player redrawing every region they own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajmuatManatiq {
    /// Which game these belong to.
    luba: LubaId,

    /// The game's display name, kept so a set found on disk can be identified
    /// without the library being available — which is the case for a set copied
    /// between machines, and the case inside a game process.
    ism_luba: String,

    /// The regions, in the order the panel lists them and the scheduler visits
    /// them.
    manatiq: Vec<Mintaqa>,

    /// The next identity to hand out.
    ///
    /// Persisted rather than derived from the maximum identity present, because
    /// deleting the newest region would otherwise make the next one reuse its
    /// number and inherit its history.
    talee: u64,

    /// The interval a region with no override is polled at, in milliseconds.
    fasila_iftiradiya_milli: u32,

    /// What was repaired when this set was last read.
    ///
    /// Not persisted. See [`MajmuatManatiq::mulahazat`].
    #[serde(skip)]
    mulahazat: Vec<String>,
}

impl MajmuatManatiq {
    /// The default polling interval for a region with no override.
    ///
    /// Four hundred milliseconds — two and a half samples a second. Chosen
    /// against what a player notices rather than against what the machine can
    /// do: a line of dialogue that appears and is translated within half a
    /// second reads as immediate, and polling faster spends frame budget to
    /// improve a number nobody perceives.
    pub const FASILA_IFTIRADIYA_MILLI: u32 = 400;

    /// An empty set for a game.
    #[must_use]
    pub fn jadeeda(luba: LubaId, ism_luba: impl Into<String>) -> Self {
        Self {
            luba,
            ism_luba: ism_luba.into(),
            manatiq: Vec::new(),
            talee: 1,
            fasila_iftiradiya_milli: Self::FASILA_IFTIRADIYA_MILLI,
            mulahazat: Vec::new(),
        }
    }

    /// Which game this set belongs to.
    #[must_use]
    pub const fn luba(&self) -> LubaId {
        self.luba
    }

    /// The game's display name.
    #[must_use]
    pub fn ism_luba(&self) -> &str {
        &self.ism_luba
    }

    /// Every region, in order.
    #[must_use]
    pub fn manatiq(&self) -> &[Mintaqa] {
        &self.manatiq
    }

    /// How many regions this set holds.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.manatiq.len()
    }

    /// Whether the set holds nothing.
    #[must_use]
    pub const fn khaliya(&self) -> bool {
        self.manatiq.is_empty()
    }

    /// The regions a scheduler should be polling right now.
    pub fn ala_muaqqit(&self) -> impl Iterator<Item = &Mintaqa> {
        self.manatiq.iter().filter(|mintaqa| mintaqa.ala_muaqqit())
    }

    /// The regions that are enabled, whatever their rule.
    pub fn mumakkana(&self) -> impl Iterator<Item = &Mintaqa> {
        self.manatiq.iter().filter(|mintaqa| mintaqa.mumakkana)
    }

    /// One region by identity.
    #[must_use]
    pub fn mintaqa(&self, muarrif: MuarrifMintaqa) -> Option<&Mintaqa> {
        self.manatiq
            .iter()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
    }

    /// The interval a region with no override is polled at.
    #[must_use]
    pub const fn fasila_iftiradiya_milli(&self) -> u32 {
        self.fasila_iftiradiya_milli
    }

    /// Changes the set's default interval, clamped into the allowed range.
    pub const fn ihdud_fasila(&mut self, milli: u32) {
        self.fasila_iftiradiya_milli = if milli < ADNA_FASILA_MILLI {
            ADNA_FASILA_MILLI
        } else if milli > AQSA_FASILA_MILLI {
            AQSA_FASILA_MILLI
        } else {
            milli
        };
    }

    /// What was repaired the last time this set was read from disk.
    ///
    /// Empty for a set that has not been read, and for one that needed nothing.
    /// A non-empty list means the file held something this build could parse but
    /// could not use — a rectangle with no area, two regions with one identity —
    /// and the set in memory is the repaired one. The distinction against
    /// [`KhataTabaqa::MalafGhayrMafhum`] is deliberate: an unknown *schema* is
    /// refused because guessing at it loses data, while unusable *content*
    /// inside a schema this build knows is dropped and reported, because
    /// refusing the whole file over one bad rectangle would lose the other
    /// sixty-three regions the player drew.
    #[must_use]
    pub fn mulahazat(&self) -> &[String] {
        &self.mulahazat
    }

    /// Adds a region, refusing a duplicate name and a rectangle with no area.
    pub fn adif(
        &mut self,
        ism: &str,
        mustatil: MustatilNisbi,
        qaida: QaidatTarjama,
    ) -> NatijatMintaqa {
        if self.manatiq.len() >= AQSA_MANATIQ {
            return NatijatMintaqa::TajawuzSaqf { saqf: AQSA_MANATIQ };
        }
        if let Some(radd) = self.tahaqquq_ism(ism, None) {
            return radd;
        }
        if !mustatil.salih() {
            return NatijatMintaqa::LaMisaha;
        }

        let muarrif = MuarrifMintaqa(self.talee);
        self.talee = self.talee.saturating_add(1);
        let mut mintaqa = Mintaqa::jadeeda(muarrif, ism.trim(), mustatil);
        mintaqa.qaida = qaida;
        self.manatiq.push(mintaqa);
        NatijatMintaqa::Tammat(muarrif)
    }

    /// Removes a region.
    ///
    /// The identity is not returned to the pool. See [`MuarrifMintaqa`].
    pub fn ihdhif(&mut self, muarrif: MuarrifMintaqa) -> NatijatMintaqa {
        let mawqi = self.mawqi(muarrif);
        match mawqi {
            Some(mawqi) => {
                let _ = self.manatiq.remove(mawqi);
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Renames a region.
    pub fn ghayyir_ism(&mut self, muarrif: MuarrifMintaqa, ism: &str) -> NatijatMintaqa {
        if self.mawqi(muarrif).is_none() {
            return NatijatMintaqa::GhayrMawjuda(muarrif);
        }
        if let Some(radd) = self.tahaqquq_ism(ism, Some(muarrif)) {
            return radd;
        }
        let munaqqah = ism.trim().to_owned();
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.ism = munaqqah;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Moves or resizes a region.
    ///
    /// One method for both because the editor produces one rectangle either way
    /// — a drag on the body and a drag on a handle differ in how the rectangle
    /// was computed, not in what is stored.
    pub fn harrik(&mut self, muarrif: MuarrifMintaqa, mustatil: MustatilNisbi) -> NatijatMintaqa {
        if !mustatil.salih() {
            return NatijatMintaqa::LaMisaha;
        }
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.mustatil = mustatil;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Turns a region on or off without deleting it.
    pub fn makkin(&mut self, muarrif: MuarrifMintaqa, mumakkana: bool) -> NatijatMintaqa {
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.mumakkana = mumakkana;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Flips a region between on and off.
    pub fn baddil_tamkeen(&mut self, muarrif: MuarrifMintaqa) -> NatijatMintaqa {
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.mumakkana = !mintaqa.mumakkana;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Changes when a region is read.
    pub fn ghayyir_qaida(
        &mut self,
        muarrif: MuarrifMintaqa,
        qaida: QaidatTarjama,
    ) -> NatijatMintaqa {
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.qaida = qaida;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Sets or clears a region's own polling interval.
    ///
    /// The value is clamped on write as well as on read. Clamping only on read
    /// would leave a number in the file that the panel displays and the
    /// scheduler ignores, which is the kind of disagreement a user reports as a
    /// setting that does nothing.
    pub fn ghayyir_fasila(
        &mut self,
        muarrif: MuarrifMintaqa,
        milli: Option<u32>,
    ) -> NatijatMintaqa {
        let mahdud = milli.map(|qeema| qeema.clamp(ADNA_FASILA_MILLI, AQSA_FASILA_MILLI));
        match self
            .manatiq
            .iter_mut()
            .find(|mintaqa| mintaqa.muarrif == muarrif)
        {
            Some(mintaqa) => {
                mintaqa.fasila_milli = mahdud;
                NatijatMintaqa::Tammat(muarrif)
            },
            None => NatijatMintaqa::GhayrMawjuda(muarrif),
        }
    }

    /// Moves a region to another place in the list.
    ///
    /// Order is not decoration. It is the order the scheduler visits regions in
    /// and therefore the order they compete for the frame budget in: the region
    /// a player cares most about belongs first, so that a frame which runs out
    /// of budget drops the quest log rather than the dialogue.
    pub fn rattib(&mut self, muarrif: MuarrifMintaqa, ila: usize) -> NatijatMintaqa {
        let Some(min) = self.mawqi(muarrif) else {
            return NatijatMintaqa::GhayrMawjuda(muarrif);
        };
        let akhir = self.manatiq.len().saturating_sub(1);
        let hadaf = ila.min(akhir);
        if hadaf == min {
            return NatijatMintaqa::Tammat(muarrif);
        }
        let mintaqa = self.manatiq.remove(min);
        self.manatiq.insert(hadaf, mintaqa);
        NatijatMintaqa::Tammat(muarrif)
    }

    /// Where a region sits in the list.
    #[must_use]
    pub fn mawqi(&self, muarrif: MuarrifMintaqa) -> Option<usize> {
        self.manatiq
            .iter()
            .position(|mintaqa| mintaqa.muarrif == muarrif)
    }

    /// Checks a proposed name, ignoring the region it is being applied to.
    ///
    /// [`None`] when the name is usable.
    fn tahaqquq_ism(&self, ism: &str, nafsuha: Option<MuarrifMintaqa>) -> Option<NatijatMintaqa> {
        let munaqqah = ism.trim();
        if munaqqah.is_empty() {
            return Some(NatijatMintaqa::IsmFarigh);
        }
        if munaqqah.chars().count() > AQSA_TUL_ISM {
            return Some(NatijatMintaqa::IsmTaweel { saqf: AQSA_TUL_ISM });
        }
        let miftah = wahhid_ism(munaqqah);
        self.manatiq
            .iter()
            .find(|mintaqa| Some(mintaqa.muarrif) != nafsuha && wahhid_ism(&mintaqa.ism) == miftah)
            .map(|mintaqa| NatijatMintaqa::IsmMakrur {
                sahib: mintaqa.muarrif,
                ism: munaqqah.to_owned(),
            })
    }

    /// Where this game's region file lives under a directory of them.
    ///
    /// One directory per game identity, so two games with the same display name
    /// — a remaster beside the original, the same title on two storefronts — do
    /// not share a file.
    #[must_use]
    pub fn masar_malaf(mujallad: &Path, luba: LubaId) -> PathBuf {
        mujallad.join(luba.to_string()).join(ISM_MALAF)
    }

    /// Writes the set atomically, stamped with the schema version.
    ///
    /// Beside, flushed, renamed, through
    /// [`taarib_usus::masarat::kitaba_dharra`]. There is no non-atomic path: a
    /// region file truncated by a crash mid-save is every region the player drew.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] naming the path when the write cannot
    /// complete, and [`KhataTabaqa::MalafGhayrMafhum`] when the set does not
    /// serialize to a JSON object — which can only mean this type stopped being
    /// a struct, and is worth reporting rather than papering over.
    pub fn ihfaz(&self, masar: &Path) -> Result<(), KhataTabaqa> {
        let bayt = mukhattat::iktub(self).map_err(|khata| KhataTabaqa::MalafGhayrMafhum {
            masar: masar.to_path_buf(),
            sigha: Self::ISM,
            sabab: khata.injilizi,
        })?;
        masarat::kitaba_dharra(masar, &bayt).map_err(|khata| KhataTabaqa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab: std::io::Error::other(khata.injilizi),
        })
    }

    /// Reads a set from disk, refusing a schema version this build does not
    /// write.
    ///
    /// The file is read with [`std::fs::read`] rather than through
    /// [`taarib_usus::masarat::qira`] on purpose: [`KhataTabaqa::KhataMalaf`]
    /// carries a real [`std::io::Error`], and `khata.rs` maps its *kind* onto
    /// the action that actually fixes it. Going through the helper would flatten
    /// "the directory is not there" and "you do not have permission" into one
    /// sentence and one useless next step. Writes still go through the helper,
    /// because atomicity is what matters on that side and nothing else supplies
    /// it.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::KhataMalaf`] when the file cannot be read, and
    /// [`KhataTabaqa::MalafGhayrMafhum`] when it is not JSON, not an object,
    /// carries no schema version, or carries one this build does not know —
    /// naming both the version found and the version understood. Nothing here
    /// migrates: this file has had exactly one shape, so a different number
    /// means a *newer* build wrote it, and silently reading a newer file means
    /// dropping the fields this build cannot see and then writing them away on
    /// the next save.
    pub fn hammil(masar: &Path) -> Result<Self, KhataTabaqa> {
        let bayt = std::fs::read(masar).map_err(|sabab| KhataTabaqa::KhataMalaf {
            masar: masar.to_path_buf(),
            sabab,
        })?;
        let mut majmua = Self::min_bayt(masar, &bayt)?;
        majmua.tashih();
        Ok(majmua)
    }

    /// Reads a set, or produces an empty one when the file is not there yet.
    ///
    /// A missing file is the first run for a game and is not a failure. Every
    /// other I/O failure still is — a region file that exists and cannot be read
    /// is not treated as an empty set, because overwriting it on the next save
    /// would destroy regions that are sitting right there on disk.
    ///
    /// # Errors
    ///
    /// As [`MajmuatManatiq::hammil`], minus the missing file, plus
    /// [`KhataTabaqa::MalafGhayrMafhum`] when the file holds a different game's
    /// regions than the one asked for.
    pub fn hammil_aw_jadeeda(
        masar: &Path,
        luba: LubaId,
        ism_luba: &str,
    ) -> Result<Self, KhataTabaqa> {
        let bayt = match std::fs::read(masar) {
            Ok(bayt) => bayt,
            Err(sabab) if sabab.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::jadeeda(luba, ism_luba));
            },
            Err(sabab) => {
                return Err(KhataTabaqa::KhataMalaf {
                    masar: masar.to_path_buf(),
                    sabab,
                });
            },
        };
        let mut majmua = Self::min_bayt(masar, &bayt)?;
        if majmua.luba != luba {
            return Err(KhataTabaqa::MalafGhayrMafhum {
                masar: masar.to_path_buf(),
                sigha: Self::ISM,
                sabab: format!(
                    "it holds regions for game {}, and regions for game {luba} were asked \
                     for. Reading it would attach one game's rectangles to another game's \
                     screen.",
                    majmua.luba
                ),
            });
        }
        majmua.tashih();
        Ok(majmua)
    }

    /// Parses bytes into a set, checking the schema version before anything else.
    fn min_bayt(masar: &Path, bayt: &[u8]) -> Result<Self, KhataTabaqa> {
        let ghalat = |sabab: String| KhataTabaqa::MalafGhayrMafhum {
            masar: masar.to_path_buf(),
            sigha: Self::ISM,
            sabab,
        };

        let qeema: Value = serde_json::from_slice(bayt)
            .map_err(|khata| ghalat(format!("it is not JSON: {khata}")))?;
        let Some(kain) = qeema.as_object() else {
            return Err(ghalat("its top level is not a JSON object".to_owned()));
        };
        let Some(mawjud) = kain.get(mukhattat::HAQL).and_then(Value::as_u64) else {
            return Err(ghalat(format!(
                "it carries no `{}` field, so there is no way to tell which shape it is in",
                mukhattat::HAQL
            )));
        };
        if mawjud != u64::from(Self::ISDAR) {
            return Err(ghalat(format!(
                "it declares schema version {mawjud}, and this build reads and writes \
                 version {}. Nothing is guessed at here: a version this build does not \
                 know was written by a build that knew more, and reading it anyway would \
                 drop every field this one cannot see and then save the file without them.",
                Self::ISDAR
            )));
        }
        mukhattat::min_qeema::<Self>(qeema).map_err(|khata| ghalat(khata.injilizi))
    }

    /// Repairs content this build can parse but cannot use, recording what it
    /// dropped.
    ///
    /// Runs on every read. Refusing the whole file over one unusable rectangle
    /// would cost a player every other region they drew, so the unusable ones
    /// are dropped and named in [`MajmuatManatiq::mulahazat`]. The set is left
    /// in the state the rest of this module's invariants assume: unique
    /// identities, unique names, every rectangle with area, every interval in
    /// range, and `talee` above every identity present.
    fn tashih(&mut self) {
        let mut mulahazat = Vec::new();
        let mut muarrifat = std::collections::BTreeSet::new();
        let mut asma = std::collections::BTreeSet::new();
        let mut mahfuza: Vec<Mintaqa> = Vec::with_capacity(self.manatiq.len());

        for mut mintaqa in std::mem::take(&mut self.manatiq) {
            if mahfuza.len() >= AQSA_MANATIQ {
                mulahazat.push(format!(
                    "region {} ({}) was dropped: the file holds more than the ceiling of \
                     {AQSA_MANATIQ}",
                    mintaqa.muarrif, mintaqa.ism
                ));
                continue;
            }
            if !muarrifat.insert(mintaqa.muarrif) {
                mulahazat.push(format!(
                    "a second region claiming identity {} was dropped; history entries \
                     recorded against it would have been ambiguous",
                    mintaqa.muarrif
                ));
                continue;
            }
            if mintaqa.ism.trim().is_empty() {
                mulahazat.push(format!(
                    "region {} was dropped: it has no name",
                    mintaqa.muarrif
                ));
                continue;
            }
            if !mintaqa.mustatil.salih() {
                mulahazat.push(format!(
                    "region {} ({}) was dropped: its rectangle has no area on any surface",
                    mintaqa.muarrif, mintaqa.ism
                ));
                continue;
            }
            // Trimmed in place, trailing end first: `bidaya` is a byte index into
            // the name, and truncating only removes bytes after it. The
            // all-whitespace name was dropped above, so the drain never empties
            // what the truncate left.
            let bidaya = mintaqa.ism.len() - mintaqa.ism.trim_start().len();
            mintaqa.ism.truncate(mintaqa.ism.trim_end().len());
            let _ = mintaqa.ism.drain(..bidaya);
            if mintaqa.ism.chars().count() > AQSA_TUL_ISM {
                let mukhtasar: String = mintaqa.ism.chars().take(AQSA_TUL_ISM).collect();
                mulahazat.push(format!(
                    "region {}'s name was shortened to {AQSA_TUL_ISM} characters",
                    mintaqa.muarrif
                ));
                mintaqa.ism = mukhtasar;
            }
            if !asma.insert(wahhid_ism(&mintaqa.ism)) {
                let mumayyaz = format!("{} ({})", mintaqa.ism, mintaqa.muarrif.raqm());
                mulahazat.push(format!(
                    "region {} was renamed to {mumayyaz}: another region already held its \
                     name and the two could not be told apart in the panel",
                    mintaqa.muarrif
                ));
                let _ = asma.insert(wahhid_ism(&mumayyaz));
                mintaqa.ism = mumayyaz;
            }
            if let Some(milli) = mintaqa.fasila_milli {
                let mahdud = milli.clamp(ADNA_FASILA_MILLI, AQSA_FASILA_MILLI);
                if mahdud != milli {
                    mulahazat.push(format!(
                        "region {}'s interval of {milli} ms was clamped to {mahdud} ms",
                        mintaqa.muarrif
                    ));
                    mintaqa.fasila_milli = Some(mahdud);
                }
            }
            mahfuza.push(mintaqa);
        }

        let aqsa = muarrifat
            .iter()
            .next_back()
            .map_or(0, |muarrif| muarrif.raqm());
        if self.talee <= aqsa {
            mulahazat.push(format!(
                "the next identity was {} and a region already held {aqsa}; it was raised so \
                 a new region cannot inherit a deleted one's history",
                self.talee
            ));
            self.talee = aqsa.saturating_add(1);
        }
        self.ihdud_fasila(self.fasila_iftiradiya_milli);

        self.manatiq = mahfuza;
        self.mulahazat = mulahazat;
    }
}

impl DhuMukhattat for MajmuatManatiq {
    const ISM: &'static str = "manatiq_luba";

    /// One. There has never been another shape of this file.
    const ISDAR: u32 = 1;

    fn hijra(min: u32, _qeema: Value) -> Natija<Value> {
        // Reached only for a version below the first one, which no build ever
        // wrote. There is nothing to migrate from and nothing to guess at: a
        // rectangle read out of a shape this build does not know would be a
        // rectangle in the wrong place on somebody's screen.
        Err(Khata::from(KhataTabaqa::MalafGhayrMafhum {
            masar: PathBuf::from(ISM_MALAF),
            sigha: Self::ISM,
            sabab: format!(
                "it claims schema {min}, and this build knows only {}",
                Self::ISDAR
            ),
        }))
    }
}

/// A point on the surface, normalized, origin top-left.
///
/// The editor is fed these by whatever the input hook is. It never sees a pixel
/// coordinate, which is what lets the same drag produce the same region on a
/// 1080p window and a 4K one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NuqtaNisbiya {
    /// Distance from the left edge, zero to one.
    pub ufuqi: f32,
    /// Distance from the top edge, zero to one.
    pub raasi: f32,
}

impl NuqtaNisbiya {
    /// A point, clamped into the surface.
    ///
    /// Clamped rather than refused for the same reason
    /// [`MustatilNisbi::min_hudud`] clamps: a player dragging past the edge of
    /// the window means "to the edge", and a non-finite coordinate — which is
    /// what a hook produces when it divides by a window size of zero during a
    /// minimize — is treated as the origin rather than allowed to make every
    /// later comparison false.
    #[must_use]
    pub fn jadeeda(ufuqi: f32, raasi: f32) -> Self {
        let sahih = |qeema: f32| {
            if qeema.is_finite() {
                qeema.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        Self {
            ufuqi: sahih(ufuqi),
            raasi: sahih(raasi),
        }
    }

    /// Whether this point is inside a rectangle, edges included.
    #[must_use]
    pub fn dakhil(self, mustatil: MustatilNisbi) -> bool {
        self.ufuqi >= mustatil.yasar
            && self.ufuqi <= mustatil.yasar + mustatil.ard
            && self.raasi >= mustatil.aala
            && self.raasi <= mustatil.aala + mustatil.irtifa
    }
}

/// One of the eight grips on a region's outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Miqbad {
    /// Top-left corner.
    AalaYasar,
    /// Top edge, centred.
    Aala,
    /// Top-right corner.
    AalaYameen,
    /// Left edge, centred.
    Yasar,
    /// Right edge, centred.
    Yameen,
    /// Bottom-left corner.
    AsfalYasar,
    /// Bottom edge, centred.
    Asfal,
    /// Bottom-right corner.
    AsfalYameen,
}

impl Miqbad {
    /// All eight, in reading order for the top row first.
    #[must_use]
    pub const fn jamee() -> [Self; 8] {
        [
            Self::AalaYasar,
            Self::Aala,
            Self::AalaYameen,
            Self::Yasar,
            Self::Yameen,
            Self::AsfalYasar,
            Self::Asfal,
            Self::AsfalYameen,
        ]
    }

    /// The name used in a log line.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::AalaYasar => "top-left",
            Self::Aala => "top",
            Self::AalaYameen => "top-right",
            Self::Yasar => "left",
            Self::Yameen => "right",
            Self::AsfalYasar => "bottom-left",
            Self::Asfal => "bottom",
            Self::AsfalYameen => "bottom-right",
        }
    }

    /// Whether dragging this grip moves the left edge.
    #[must_use]
    pub const fn yumsik_yasar(self) -> bool {
        matches!(self, Self::AalaYasar | Self::Yasar | Self::AsfalYasar)
    }

    /// Whether dragging this grip moves the right edge.
    #[must_use]
    pub const fn yumsik_yameen(self) -> bool {
        matches!(self, Self::AalaYameen | Self::Yameen | Self::AsfalYameen)
    }

    /// Whether dragging this grip moves the top edge.
    #[must_use]
    pub const fn yumsik_aala(self) -> bool {
        matches!(self, Self::AalaYasar | Self::Aala | Self::AalaYameen)
    }

    /// Whether dragging this grip moves the bottom edge.
    #[must_use]
    pub const fn yumsik_asfal(self) -> bool {
        matches!(self, Self::AsfalYasar | Self::Asfal | Self::AsfalYameen)
    }

    /// Where this grip sits on a rectangle.
    #[must_use]
    pub fn markaz(self, mustatil: MustatilNisbi) -> NuqtaNisbiya {
        let yasar = mustatil.yasar;
        let yameen = mustatil.yasar + mustatil.ard;
        let aala = mustatil.aala;
        let asfal = mustatil.aala + mustatil.irtifa;
        let ufuqi = if self.yumsik_yasar() {
            yasar
        } else if self.yumsik_yameen() {
            yameen
        } else {
            f32::midpoint(yasar, yameen)
        };
        let raasi = if self.yumsik_aala() {
            aala
        } else if self.yumsik_asfal() {
            asfal
        } else {
            f32::midpoint(aala, asfal)
        };
        NuqtaNisbiya { ufuqi, raasi }
    }
}

/// What the pointer is over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsabatMuharrir {
    /// Which region.
    pub hadaf: MuarrifMintaqa,
    /// Which grip, or [`None`] for the body.
    pub miqbad: Option<Miqbad>,
}

/// What the editor is in the middle of.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WadaMuharrir {
    /// Nothing is being dragged.
    Khamil,

    /// A new region is being drawn from a corner.
    Rasm {
        /// Where the drag started.
        bidaya: NuqtaNisbiya,
        /// Where the pointer is now.
        hali: NuqtaNisbiya,
    },

    /// An existing region is being dragged bodily.
    Naql {
        /// Which region.
        hadaf: MuarrifMintaqa,
        /// Where inside the region the drag started.
        imsak: NuqtaNisbiya,
        /// The rectangle as it was when the drag started.
        asli: MustatilNisbi,
        /// Where the pointer is now.
        hali: NuqtaNisbiya,
    },

    /// An existing region is being resized by one grip.
    Tahjim {
        /// Which region.
        hadaf: MuarrifMintaqa,
        /// Which grip.
        miqbad: Miqbad,
        /// The rectangle as it was when the drag started.
        asli: MustatilNisbi,
        /// Where the pointer is now.
        hali: NuqtaNisbiya,
    },
}

impl WadaMuharrir {
    /// Whether a drag is in progress.
    #[must_use]
    pub const fn fi_sahb(self) -> bool {
        !matches!(self, Self::Khamil)
    }

    /// The region a drag is acting on, when it is acting on one.
    #[must_use]
    pub const fn hadaf(self) -> Option<MuarrifMintaqa> {
        match self {
            Self::Khamil | Self::Rasm { .. } => None,
            Self::Naql { hadaf, .. } | Self::Tahjim { hadaf, .. } => Some(hadaf),
        }
    }
}

/// What a released drag produced.
#[derive(Debug, Clone, PartialEq)]
pub enum NatijatTahrir {
    /// Nothing was in progress.
    LaShay,

    /// A rectangle for a new region, which the caller names and adds.
    Jadeeda(MustatilNisbi),

    /// A region was dragged to a new place.
    Munaqqala {
        /// Which region.
        hadaf: MuarrifMintaqa,
        /// Where it now is.
        mustatil: MustatilNisbi,
    },

    /// A region was resized.
    Muhajjama {
        /// Which region.
        hadaf: MuarrifMintaqa,
        /// Its new rectangle.
        mustatil: MustatilNisbi,
    },

    /// The drag produced nothing usable and was thrown away.
    Mulgha {
        /// Why, in English, for the log.
        sabab: &'static str,
        /// Why, in Arabic, for the player.
        unwan: &'static str,
    },
}

impl NatijatTahrir {
    /// The rectangle this outcome carries, when it carries one.
    #[must_use]
    pub const fn mustatil(&self) -> Option<MustatilNisbi> {
        match self {
            Self::Jadeeda(mustatil)
            | Self::Munaqqala { mustatil, .. }
            | Self::Muhajjama { mustatil, .. } => Some(*mustatil),
            Self::LaShay | Self::Mulgha { .. } => None,
        }
    }
}

/// A surface dimension as a float, with the precision question answered once.
///
/// `u32` to `f32` is exact below 2^24, which is 16.7 million — four orders of
/// magnitude past any surface dimension that will exist. Written here rather
/// than suppressed at four call sites.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// The in-game region editor: one drag at a time, and nothing else.
///
/// Deliberately not a widget. It owns no rendering, no input decoding and no
/// region set — it is handed normalized points, it answers what rectangle is
/// under construction so the overlay can outline it, and on release it produces
/// one [`NatijatTahrir`] the caller applies to a [`MajmuatManatiq`]. Keeping the
/// application out of here is what lets the panel confirm a name before the
/// region exists, and what lets a cancelled drag cost nothing.
#[derive(Debug, Clone, Copy)]
pub struct MuharrirManatiq {
    wada: WadaMuharrir,
    nisf_miqbad_biksel: f32,
    adna_dila: f32,
}

impl Default for MuharrirManatiq {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl MuharrirManatiq {
    /// Half a grip's side, in pixels.
    ///
    /// Fourteen pixels across. Measured in *pixels* rather than in normalized
    /// units on purpose: a grip sized in normalized units is a rectangle rather
    /// than a square on any surface that is not 1:1, and on a 21:9 monitor the
    /// corner grips end up nearly twice as tall as they are wide — so the corner
    /// a player aims at grabs an edge instead.
    pub const NISF_MIQBAD_BIKSEL: f32 = 7.0;

    /// The shortest side a drawn region may have, normalized.
    ///
    /// Six thousandths of the surface: eleven pixels across a 1920-wide window.
    /// The specification is "a click with no drag produces nothing", and this is
    /// the honest version of it — a two-pixel drag is a click as far as the hand
    /// that made it is concerned, and a two-pixel region captures nothing a
    /// recognizer can read while still being an entry in the list that the
    /// player cannot grab to delete.
    pub const ADNA_DILA: f32 = 0.006;

    /// An idle editor.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            wada: WadaMuharrir::Khamil,
            nisf_miqbad_biksel: Self::NISF_MIQBAD_BIKSEL,
            adna_dila: Self::ADNA_DILA,
        }
    }

    /// What is in progress.
    #[must_use]
    pub const fn wada(&self) -> WadaMuharrir {
        self.wada
    }

    /// Whether a drag is in progress.
    #[must_use]
    pub const fn fi_sahb(&self) -> bool {
        self.wada.fi_sahb()
    }

    /// Changes the grip size, in pixels, with a floor of two.
    pub fn ihdud_miqbad(&mut self, biksel: f32) {
        self.nisf_miqbad_biksel = if biksel.is_finite() && biksel > 2.0 {
            biksel
        } else {
            2.0
        };
    }

    /// Changes the shortest usable side, normalized, with a floor of one
    /// ten-thousandth.
    pub fn ihdud_adna_dila(&mut self, dila: f32) {
        self.adna_dila = if dila.is_finite() && dila > 0.0001 {
            dila
        } else {
            0.0001
        };
    }

    /// Starts drawing a new region from this point.
    ///
    /// Unconditional: this is the entry point for "the player is drawing", used
    /// when the editor is in add mode and an existing region under the pointer
    /// should be ignored.
    pub const fn bada_sahb(&mut self, nuqta: NuqtaNisbiya) {
        self.wada = WadaMuharrir::Rasm {
            bidaya: nuqta,
            hali: nuqta,
        };
    }

    /// Starts whichever drag this point implies, and reports which one.
    ///
    /// The entry point the input hook calls on a press. A grip starts a resize,
    /// a region's body starts a move, and empty screen starts a new region —
    /// which is the behaviour every editor of this kind has and the reason the
    /// hit test lives here rather than in the caller. The region set and the
    /// surface are both needed: the set to know what is under the pointer, the
    /// surface to make the grips square in pixels.
    pub fn bada_ala(
        &mut self,
        nuqta: NuqtaNisbiya,
        majmua: &MajmuatManatiq,
        sath: WasfSath,
    ) -> WadaMuharrir {
        let isaba = self.taht(nuqta, majmua, sath);
        self.wada = match isaba {
            Some(isaba) => {
                let asli = majmua
                    .mintaqa(isaba.hadaf)
                    .map_or(MustatilNisbi::KAAMIL, |mintaqa| mintaqa.mustatil);
                match isaba.miqbad {
                    Some(miqbad) => WadaMuharrir::Tahjim {
                        hadaf: isaba.hadaf,
                        miqbad,
                        asli,
                        hali: nuqta,
                    },
                    None => WadaMuharrir::Naql {
                        hadaf: isaba.hadaf,
                        imsak: nuqta,
                        asli,
                        hali: nuqta,
                    },
                }
            },
            None => WadaMuharrir::Rasm {
                bidaya: nuqta,
                hali: nuqta,
            },
        };
        self.wada
    }

    /// What is under a point: a grip, a region's body, or nothing.
    ///
    /// Walks the set from the end, because later regions are drawn over earlier
    /// ones and the one a player sees on top is the one they mean to grab. A
    /// region's grips are tested before its body so that a grip sitting inside
    /// an overlapping neighbour is still reachable.
    #[must_use]
    pub fn taht(
        &self,
        nuqta: NuqtaNisbiya,
        majmua: &MajmuatManatiq,
        sath: WasfSath,
    ) -> Option<IsabatMuharrir> {
        let (nisf_ufuqi, nisf_raasi) = self.nisf_miqbad_nisbi(sath);
        for mintaqa in majmua.manatiq().iter().rev() {
            for miqbad in Miqbad::jamee() {
                let markaz = miqbad.markaz(mintaqa.mustatil);
                if (nuqta.ufuqi - markaz.ufuqi).abs() <= nisf_ufuqi
                    && (nuqta.raasi - markaz.raasi).abs() <= nisf_raasi
                {
                    return Some(IsabatMuharrir {
                        hadaf: mintaqa.muarrif,
                        miqbad: Some(miqbad),
                    });
                }
            }
            if nuqta.dakhil(mintaqa.mustatil) {
                return Some(IsabatMuharrir {
                    hadaf: mintaqa.muarrif,
                    miqbad: None,
                });
            }
        }
        None
    }

    /// Half a grip's side in normalized units, per axis.
    ///
    /// Falls back to a square in normalized units when the surface reports a
    /// zero dimension, which happens for one frame around a minimize. Half a
    /// grip in the wrong shape for one frame is better than a division that
    /// produces infinity and a hit test that matches everything.
    fn nisf_miqbad_nisbi(&self, sath: WasfSath) -> (f32, f32) {
        let ufuqi = if sath.ard == 0 {
            self.adna_dila
        } else {
            self.nisf_miqbad_biksel / madaa_f32(sath.ard)
        };
        let raasi = if sath.irtifa == 0 {
            self.adna_dila
        } else {
            self.nisf_miqbad_biksel / madaa_f32(sath.irtifa)
        };
        (ufuqi, raasi)
    }

    /// Moves the drag to a new point.
    ///
    /// Does nothing when nothing is in progress, so an input hook that delivers
    /// motion events unconditionally does not have to filter them.
    pub const fn harrik(&mut self, nuqta: NuqtaNisbiya) {
        match &mut self.wada {
            WadaMuharrir::Khamil => {},
            WadaMuharrir::Rasm { hali, .. }
            | WadaMuharrir::Naql { hali, .. }
            | WadaMuharrir::Tahjim { hali, .. } => *hali = nuqta,
        }
    }

    /// The rectangle the overlay should be outlining right now.
    #[must_use]
    pub fn muaayana(&self) -> Option<MustatilNisbi> {
        match self.wada {
            WadaMuharrir::Khamil => None,
            WadaMuharrir::Rasm { bidaya, hali } => Some(MustatilNisbi::min_hudud(
                bidaya.ufuqi,
                bidaya.raasi,
                hali.ufuqi,
                hali.raasi,
            )),
            WadaMuharrir::Naql {
                imsak, asli, hali, ..
            } => Some(Self::mustatil_naql(asli, imsak, hali)),
            WadaMuharrir::Tahjim {
                miqbad, asli, hali, ..
            } => Some(Self::mustatil_tahjim(asli, miqbad, hali)),
        }
    }

    /// Abandons whatever is in progress, and reports whether anything was.
    ///
    /// What the Escape key does. A cancelled drag leaves the region set exactly
    /// as it was, because nothing was applied to it in the first place.
    pub const fn alghi(&mut self) -> bool {
        let kan = self.wada.fi_sahb();
        self.wada = WadaMuharrir::Khamil;
        kan
    }

    /// Ends the drag and produces its one outcome.
    ///
    /// The degenerate case is the one this method exists to get right. A press
    /// and a release with no movement between them produces a rectangle with no
    /// area, which [`MustatilNisbi::salih`] already refuses; creating the region
    /// anyway would put an entry in the list that draws nothing, captures
    /// nothing and cannot be grabbed to remove, because it has no body and its
    /// eight grips are all in the same place. So it cancels, and says which of
    /// the two reasons it cancelled for.
    pub fn anhi_sahb(&mut self) -> NatijatTahrir {
        let wada = std::mem::replace(&mut self.wada, WadaMuharrir::Khamil);
        match wada {
            WadaMuharrir::Khamil => NatijatTahrir::LaShay,
            WadaMuharrir::Rasm { bidaya, hali } => {
                let mustatil =
                    MustatilNisbi::min_hudud(bidaya.ufuqi, bidaya.raasi, hali.ufuqi, hali.raasi);
                if !mustatil.salih() {
                    return NatijatTahrir::Mulgha {
                        sabab: "the drag had no area, so no region was created",
                        unwan: "لم تُرسم مساحة، فلم تُنشأ منطقة.",
                    };
                }
                if self.saghir(mustatil) {
                    return NatijatTahrir::Mulgha {
                        sabab: "the drag was too small to hold readable text",
                        unwan: "المساحة المرسومة أصغر من أن تحوي نصًّا يُقرأ.",
                    };
                }
                NatijatTahrir::Jadeeda(mustatil)
            },
            WadaMuharrir::Naql {
                hadaf,
                imsak,
                asli,
                hali,
            } => NatijatTahrir::Munaqqala {
                hadaf,
                mustatil: Self::mustatil_naql(asli, imsak, hali),
            },
            WadaMuharrir::Tahjim {
                hadaf,
                miqbad,
                asli,
                hali,
            } => {
                let mustatil = Self::mustatil_tahjim(asli, miqbad, hali);
                if !mustatil.salih() || self.saghir(mustatil) {
                    return NatijatTahrir::Mulgha {
                        sabab: "the resize collapsed the region, so it was left as it was",
                        unwan: "أدّى التحجيم إلى طيّ المنطقة، فبقيت كما كانت.",
                    };
                }
                NatijatTahrir::Muhajjama { hadaf, mustatil }
            },
        }
    }

    /// Whether a rectangle is below the shortest usable side on either axis.
    fn saghir(&self, mustatil: MustatilNisbi) -> bool {
        mustatil.ard < self.adna_dila || mustatil.irtifa < self.adna_dila
    }

    /// A region translated by a drag, kept whole and kept on the surface.
    ///
    /// Clamped as a translation rather than through
    /// [`MustatilNisbi::min_hudud`]: clamping the edges independently would let
    /// a region dragged past the top of the screen come back shorter, and a
    /// player who moves a region and finds it resized has been given a bug
    /// rather than a feature.
    fn mustatil_naql(
        asli: MustatilNisbi,
        imsak: NuqtaNisbiya,
        hali: NuqtaNisbiya,
    ) -> MustatilNisbi {
        let saqf_ufuqi = (1.0 - asli.ard).max(0.0);
        let saqf_raasi = (1.0 - asli.irtifa).max(0.0);
        let yasar = (asli.yasar + (hali.ufuqi - imsak.ufuqi)).clamp(0.0, saqf_ufuqi);
        let aala = (asli.aala + (hali.raasi - imsak.raasi)).clamp(0.0, saqf_raasi);
        MustatilNisbi {
            yasar,
            aala,
            ard: asli.ard,
            irtifa: asli.irtifa,
        }
    }

    /// A region with the edges one grip owns moved to the pointer.
    ///
    /// Built through [`MustatilNisbi::min_hudud`], which sorts the edges, so
    /// dragging the left grip past the right one flips the rectangle instead of
    /// producing a negative width — the behaviour a player expects from every
    /// other rectangle tool they have used.
    fn mustatil_tahjim(asli: MustatilNisbi, miqbad: Miqbad, hali: NuqtaNisbiya) -> MustatilNisbi {
        let mut yasar = asli.yasar;
        let mut yameen = asli.yasar + asli.ard;
        let mut aala = asli.aala;
        let mut asfal = asli.aala + asli.irtifa;
        if miqbad.yumsik_yasar() {
            yasar = hali.ufuqi;
        }
        if miqbad.yumsik_yameen() {
            yameen = hali.ufuqi;
        }
        if miqbad.yumsik_aala() {
            aala = hali.raasi;
        }
        if miqbad.yumsik_asfal() {
            asfal = hali.raasi;
        }
        MustatilNisbi::min_hudud(yasar, aala, yameen, asfal)
    }
}

/// A pixel text block turned into a stored region's coordinates.
///
/// The bridge from what `iltiqat`'s `KutalNass` reports — rectangles in the
/// captured frame's own pixels — into the only coordinate system a region is
/// ever stored in. [`None`] when the surface or the block has no area, which is
/// what a detector returns for a frame it could not segment.
#[must_use]
pub fn nisbi_min_biksel(kutla: MustatilBiksel, sath: WasfSath) -> Option<MustatilNisbi> {
    if sath.ard == 0 || sath.irtifa == 0 || kutla.ard == 0 || kutla.irtifa == 0 {
        return None;
    }
    let ard = madaa_f32(sath.ard);
    let irtifa = madaa_f32(sath.irtifa);
    let nisbi = MustatilNisbi {
        yasar: madaa_f32(kutla.yasar) / ard,
        aala: madaa_f32(kutla.aala) / irtifa,
        ard: madaa_f32(kutla.ard) / ard,
        irtifa: madaa_f32(kutla.irtifa) / irtifa,
    };
    nisbi.salih().then_some(nisbi)
}

/// Whether two rectangles are in the same place to within a tolerance.
///
/// Every edge, not the centre and not the area. A box that keeps its centre and
/// grows a line taller each time the speaker's name gets longer is the same
/// piece of furniture; a box that has moved half a screen with the same size is
/// not, and comparing centres alone would confuse the two in the direction that
/// matters — proposing a region where text no longer appears.
#[must_use]
pub fn yatashabah(awwal: MustatilNisbi, thani: MustatilNisbi, tasamuh: f32) -> bool {
    (awwal.yasar - thani.yasar).abs() <= tasamuh
        && (awwal.aala - thani.aala).abs() <= tasamuh
        && (awwal.ard - thani.ard).abs() <= tasamuh
        && (awwal.irtifa - thani.irtifa).abs() <= tasamuh
}

/// One place text keeps appearing, and how sure the detector is about it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Murashshah {
    mustatil: MustatilNisbi,
    thabat: u32,
    marrat: u64,
    jeel_awwal: u64,
    jeel_akhir: u64,
}

impl Murashshah {
    /// Where the text keeps appearing, smoothed across every sighting.
    #[must_use]
    pub const fn mustatil(&self) -> MustatilNisbi {
        self.mustatil
    }

    /// The stability score: raised by one per sample seen, lowered by the decay
    /// per sample missed.
    #[must_use]
    pub const fn thabat(&self) -> u32 {
        self.thabat
    }

    /// How many samples this block has been seen in, ever.
    #[must_use]
    pub const fn marrat(&self) -> u64 {
        self.marrat
    }

    /// The sample this block was first seen in.
    #[must_use]
    pub const fn jeel_awwal(&self) -> u64 {
        self.jeel_awwal
    }

    /// The sample this block was last seen in.
    #[must_use]
    pub const fn jeel_akhir(&self) -> u64 {
        self.jeel_akhir
    }

    /// Whether this block has earned a proposal at the given threshold.
    #[must_use]
    pub const fn mustaqirr(&self, hadd: u32) -> bool {
        self.thabat >= hadd
    }

    /// The sentence the panel shows beside a proposal.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "seen in {} of the samples since {}, stability {}",
            self.marrat, self.jeel_awwal, self.thabat
        )
    }
}

/// Finds the places text keeps appearing, so the player does not have to draw
/// every box by hand.
///
/// Fed a batch of pixel text blocks per sample — whatever `iltiqat`'s
/// `KutalNass` found in that frame — and asked, later, which of them are
/// furniture. A dialogue box is in the same place every time it appears; a
/// damage number, a floating quest marker and a hit indicator are not.
///
/// ## Why a generation counter and a decay
///
/// The obvious implementation is a window of the last N samples per candidate.
/// It is wrong in three ways at once, and all three show up in real games:
///
/// * It forgets silently. A box seen for a minute and a box seen for N frames
///   score identically, so the detector cannot prefer the one it is sure about.
/// * It is brittle across a single dropped frame. One sample where the
///   segmenter missed the box resets a window and keeps a counter honest.
/// * It costs N rectangles per candidate to store the same fact one number
///   stores.
///
/// So [`Murashshah::thabat`] climbs by one for every sample a block is seen in,
/// falls by [`IktishafTilqai::inhilal`] for every sample it is missing from, and
/// a candidate that reaches zero is dropped. At the defaults and four samples a
/// second, a dialogue box proposes itself after about six seconds on screen, and
/// a damage number that stops appearing is gone from the table in two.
///
/// ## Why a full table refuses rather than evicts
///
/// A game that paints a hundred uniquely positioned text blocks per frame — a
/// scrolling combat log, a bullet-hell damage readout — would, under a
/// least-recently-used eviction, push the dialogue box out of the table on every
/// single frame and the detector would propose nothing forever. So the table
/// refuses newcomers once it is full unless there is a candidate that is both at
/// the floor and absent this sample, and counts the refusals in
/// [`IktishafTilqai::muhmala`] so the panel can say why detection is finding
/// nothing.
#[derive(Debug, Clone)]
pub struct IktishafTilqai {
    murashshahat: Vec<Murashshah>,
    jeel: u64,
    tasamuh: f32,
    hadd_iqtirah: u32,
    inhilal: u32,
    saqf_thabat: u32,
    aqsa_murashshahat: usize,
    muhmala: u64,
}

impl Default for IktishafTilqai {
    fn default() -> Self {
        Self::jadeed()
    }
}

impl IktishafTilqai {
    /// How far two rectangles may differ and still be the same place.
    ///
    /// Twelve thousandths of the surface — twenty-three pixels across a
    /// 1920-wide window. Wide enough to absorb a segmenter that finds the box
    /// one text row taller when the speaker's name is long, narrow enough that
    /// two stacked subtitle lines are not merged into one candidate.
    pub const TASAMUH_IFTIRADI: f32 = 0.012;

    /// The stability a block must reach to be proposed.
    pub const HADD_IQTIRAH_IFTIRADI: u32 = 24;

    /// What a missed sample costs a candidate.
    ///
    /// Three, against a gain of one, so a block has to be present in three
    /// samples out of four merely to hold its ground. That asymmetry is the
    /// whole discriminator: furniture is present in nearly every sample, and
    /// anything that is present in half of them decays away.
    pub const INHILAL_IFTIRADI: u32 = 3;

    /// The ceiling on a stability score.
    ///
    /// Two hundred and forty. Without it a box that has been on screen for an
    /// hour would take four minutes of absence to decay, and a player who
    /// rearranged the game's HUD would keep being offered the old position.
    pub const SAQF_THABAT: u32 = 240;

    /// How many candidates are tracked at once.
    pub const AQSA_MURASHSHAHAT: usize = 128;

    /// How much of a new sighting is blended into a candidate's rectangle.
    ///
    /// A quarter, exponentially. A running mean over every sighting would make
    /// a candidate that has been seen a thousand times unable to follow a box
    /// the game genuinely moved; blending a fixed fraction lets the rectangle
    /// track a real move within a second while still averaging out the
    /// one-pixel jitter a segmenter produces frame to frame.
    pub const MUAMIL_TANEEM: f32 = 0.25;

    /// A detector with nothing seen yet.
    #[must_use]
    pub const fn jadeed() -> Self {
        Self {
            murashshahat: Vec::new(),
            jeel: 0,
            tasamuh: Self::TASAMUH_IFTIRADI,
            hadd_iqtirah: Self::HADD_IQTIRAH_IFTIRADI,
            inhilal: Self::INHILAL_IFTIRADI,
            saqf_thabat: Self::SAQF_THABAT,
            aqsa_murashshahat: Self::AQSA_MURASHSHAHAT,
            muhmala: 0,
        }
    }

    /// How many samples have been recorded.
    #[must_use]
    pub const fn jeel(&self) -> u64 {
        self.jeel
    }

    /// How many sightings were dropped because the table was full of candidates
    /// that were holding their ground.
    #[must_use]
    pub const fn muhmala(&self) -> u64 {
        self.muhmala
    }

    /// What a missed sample costs a candidate.
    #[must_use]
    pub const fn inhilal(&self) -> u32 {
        self.inhilal
    }

    /// The stability a block must reach to be proposed.
    #[must_use]
    pub const fn hadd_iqtirah(&self) -> u32 {
        self.hadd_iqtirah
    }

    /// Every candidate currently tracked, proposed or not.
    #[must_use]
    pub fn murashshahat(&self) -> &[Murashshah] {
        &self.murashshahat
    }

    /// Changes how far two rectangles may differ and still match.
    pub const fn ihdud_tasamuh(&mut self, tasamuh: f32) {
        self.tasamuh = if tasamuh.is_finite() {
            tasamuh.clamp(0.0, 0.25)
        } else {
            self.tasamuh
        };
    }

    /// Changes the stability a block must reach to be proposed, with a floor of
    /// two — a block seen once has told us nothing.
    pub const fn ihdud_hadd_iqtirah(&mut self, hadd: u32) {
        self.hadd_iqtirah = if hadd < 2 { 2 } else { hadd };
    }

    /// Changes what a missed sample costs, with a floor of one.
    ///
    /// A decay of zero would mean nothing is ever forgotten, and every text
    /// block a game ever drew would eventually be proposed as a region.
    pub const fn ihdud_inhilal(&mut self, inhilal: u32) {
        self.inhilal = if inhilal == 0 { 1 } else { inhilal };
    }

    /// Forgets everything, keeping the settings.
    ///
    /// What the panel calls when the player changes resolution or HUD scale:
    /// every candidate describes where text used to be, and none of them is
    /// evidence about where it is now.
    pub fn imsah(&mut self) {
        self.murashshahat.clear();
        self.jeel = 0;
        self.muhmala = 0;
    }

    /// Records one sample of text blocks and returns the generation it became.
    ///
    /// Blocks are normalized against the surface they were found on, so samples
    /// taken before and after a resolution change still describe the same
    /// candidate. Two blocks in one sample that both match the same candidate
    /// raise it once, not twice: a game that draws its dialogue box's border and
    /// its text as two overlapping blocks would otherwise score twice as fast as
    /// one that draws it as one, for no reason a player could ever discover.
    pub fn sajjil(&mut self, sath: WasfSath, kutal: &[MustatilBiksel]) -> u64 {
        self.jeel = self.jeel.saturating_add(1);
        let jeel = self.jeel;

        for kutla in kutal {
            let Some(nisbi) = nisbi_min_biksel(*kutla, sath) else {
                continue;
            };
            let mutabiq = self.mawqi_mutabiq(nisbi);
            match mutabiq {
                Some(mawqi) => {
                    if let Some(murashshah) = self.murashshahat.get_mut(mawqi) {
                        if murashshah.jeel_akhir == jeel {
                            continue;
                        }
                        murashshah.mustatil =
                            Self::naam(murashshah.mustatil, nisbi, Self::MUAMIL_TANEEM);
                        murashshah.thabat =
                            murashshah.thabat.saturating_add(1).min(self.saqf_thabat);
                        murashshah.marrat = murashshah.marrat.saturating_add(1);
                        murashshah.jeel_akhir = jeel;
                    }
                },
                None => self.adif_murashshah(nisbi, jeel),
            }
        }

        for murashshah in &mut self.murashshahat {
            if murashshah.jeel_akhir != jeel {
                murashshah.thabat = murashshah.thabat.saturating_sub(self.inhilal);
            }
        }
        self.murashshahat.retain(|murashshah| murashshah.thabat > 0);
        jeel
    }

    /// Adds a candidate, making room only by dropping one that is at the floor
    /// and absent from this sample.
    fn adif_murashshah(&mut self, mustatil: MustatilNisbi, jeel: u64) {
        if self.murashshahat.len() >= self.aqsa_murashshahat {
            let daeef = self
                .murashshahat
                .iter()
                .enumerate()
                .filter(|(_, murashshah)| murashshah.jeel_akhir != jeel && murashshah.thabat <= 1)
                .min_by_key(|(_, murashshah)| (murashshah.thabat, murashshah.jeel_akhir))
                .map(|(mawqi, _)| mawqi);
            match daeef {
                Some(mawqi) if mawqi < self.murashshahat.len() => {
                    let _ = self.murashshahat.remove(mawqi);
                },
                _ => {
                    self.muhmala = self.muhmala.saturating_add(1);
                    return;
                },
            }
        }
        self.murashshahat.push(Murashshah {
            mustatil,
            thabat: 1,
            marrat: 1,
            jeel_awwal: jeel,
            jeel_akhir: jeel,
        });
    }

    /// The candidate a sighting belongs to, if any.
    ///
    /// The closest match rather than the first, measured as the largest edge
    /// disagreement, so a sighting that falls inside two overlapping candidates'
    /// tolerances joins the one it resembles most instead of the one that
    /// happens to be earlier in the table.
    fn mawqi_mutabiq(&self, nisbi: MustatilNisbi) -> Option<usize> {
        self.murashshahat
            .iter()
            .enumerate()
            .filter(|(_, murashshah)| yatashabah(murashshah.mustatil, nisbi, self.tasamuh))
            .min_by(|(_, awwal), (_, thani)| {
                Self::buad(awwal.mustatil, nisbi).total_cmp(&Self::buad(thani.mustatil, nisbi))
            })
            .map(|(mawqi, _)| mawqi)
    }

    /// The largest disagreement between two rectangles, over all four edges.
    fn buad(awwal: MustatilNisbi, thani: MustatilNisbi) -> f32 {
        (awwal.yasar - thani.yasar)
            .abs()
            .max((awwal.aala - thani.aala).abs())
            .max((awwal.ard - thani.ard).abs())
            .max((awwal.irtifa - thani.irtifa).abs())
    }

    /// A candidate's rectangle with a fraction of a new sighting blended in.
    fn naam(qadeem: MustatilNisbi, jadeed: MustatilNisbi, muamil: f32) -> MustatilNisbi {
        let mazij = |min: f32, ila: f32| (ila - min).mul_add(muamil, min);
        MustatilNisbi {
            yasar: mazij(qadeem.yasar, jadeed.yasar),
            aala: mazij(qadeem.aala, jadeed.aala),
            ard: mazij(qadeem.ard, jadeed.ard),
            irtifa: mazij(qadeem.irtifa, jadeed.irtifa),
        }
    }

    /// The candidates stable enough to offer, strongest first.
    ///
    /// Ties break by position — top-left before bottom-right — so the list the
    /// panel shows does not reorder itself between two frames that scored the
    /// same, which is the sort of movement that makes a list impossible to
    /// click.
    #[must_use]
    pub fn iqtirahat(&self) -> Vec<Murashshah> {
        let mut mustaqirra: Vec<Murashshah> = self
            .murashshahat
            .iter()
            .filter(|murashshah| murashshah.mustaqirr(self.hadd_iqtirah))
            .copied()
            .collect();
        mustaqirra.sort_by(|awwal, thani| {
            thani
                .thabat
                .cmp(&awwal.thabat)
                .then_with(|| awwal.mustatil.aala.total_cmp(&thani.mustatil.aala))
                .then_with(|| awwal.mustatil.yasar.total_cmp(&thani.mustatil.yasar))
        });
        mustaqirra
    }

    /// The name a proposal is offered under.
    ///
    /// Numbered rather than described, because the detector knows where text
    /// appears and nothing whatsoever about what it says. A name like "dialogue"
    /// would be a guess presented as a fact, and the player renames it in the
    /// editor the moment they see what is in it.
    #[must_use]
    pub fn ism_muqtarah(tarteeb: usize) -> String {
        format!("منطقة مكتشفة {}", tarteeb.saturating_add(1))
    }

    /// Adds every proposal that is not already covered by an existing region.
    ///
    /// Returns one [`NatijatMintaqa`] per proposal, in proposal order, so the
    /// panel can report exactly which were taken and why the rest were not — a
    /// full set, a name collision, a rectangle an existing region already
    /// covers. Proposals that duplicate an existing region are skipped silently
    /// rather than reported as failures, because a detector re-offering the box
    /// the player already drew is the normal case and not a problem.
    pub fn tabanna(&self, majmua: &mut MajmuatManatiq) -> Vec<NatijatMintaqa> {
        let mut natai = Vec::new();
        for (tarteeb, murashshah) in self.iqtirahat().into_iter().enumerate() {
            let mawjuda = majmua
                .manatiq()
                .iter()
                .any(|mintaqa| yatashabah(mintaqa.mustatil, murashshah.mustatil, self.tasamuh));
            if mawjuda {
                continue;
            }
            let mut ism = Self::ism_muqtarah(tarteeb);
            let mut mahawala = 0_usize;
            // A detected name can collide with one the player typed. Suffixing
            // rather than refusing keeps a detection run from stopping at the
            // first clash and losing every proposal behind it.
            while majmua
                .manatiq()
                .iter()
                .any(|mintaqa| wahhid_ism(&mintaqa.ism) == wahhid_ism(&ism))
            {
                mahawala = mahawala.saturating_add(1);
                if mahawala > AQSA_MANATIQ {
                    break;
                }
                ism = format!("{} ({mahawala})", Self::ism_muqtarah(tarteeb));
            }
            natai.push(majmua.adif(&ism, murashshah.mustatil, QaidatTarjama::IndaTaghyeer));
        }
        natai
    }
}
