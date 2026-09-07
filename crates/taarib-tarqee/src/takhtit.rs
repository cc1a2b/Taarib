//! التخطيط — precomputation, and the glyph set that falls out of it.
//!
//! Every translated string is laid out here, once, on a desktop, at each size
//! [`crate::tahdid_maqasat`] discovered for it — with the real font chain, the
//! real size, the real available width and the string's real style spans — and
//! the positioned glyphs are stored. What the game does at run time is read an
//! array and draw it.
//!
//! Everything else in this module follows from one property of that trade: a
//! precomputed layout is *believed*. An adapter does not check it, cannot check
//! it, and has nothing to compare it against. So a layout that is subtly wrong
//! is worse than no layout at all — the runtime path would have produced the
//! right answer slowly, and this produces the wrong one at full speed, in the
//! container, past every report. Each refusal below exists because the
//! alternative was a layout nobody would have caught.
//!
//! ## Unknown width is laid out unwrapped, and says so
//!
//! The wrap width comes from [`QuyudNass::aqsa_ard`] and from nowhere else.
//! When it is absent the text is laid out with **no width constraint at all**
//! and the result carries [`ALAM_TAKHTIT_BILA_QAYD_ARD`].
//!
//! It is not estimated. Not from the rectangle the original occupied —
//! `QuyudNass::mustatil` is where the old text *ended up*, which is the box only
//! when the component was fixed and is the text's own extent when it was not,
//! and nothing recorded says which. Capture already refuses that promotion at
//! its own end, for the same reason, and it would be strange for the compiler to
//! make the inference the measuring code declined to.
//!
//! Not from a sibling string either, and not from a screen resolution. An
//! invented width produces line breaks in places the game will not break, and
//! every baseline after the first wrong break is wrong too — a whole paragraph
//! displaced by a guess about one number.
//!
//! The flag is what makes an unwrapped layout distinguishable from one computed
//! inside a real box. Without it the two are the same record, and an adapter
//! drawing a paragraph into a narrow panel would trust a single line that never
//! fit anything. There was no existing `ALAM_TAKHTIT_*` bit for this — the four
//! `taarib-ruqaa` defines cover direction, truncation, overflow and shrinking,
//! all of which are properties of a layout that *had* bounds — so the bit is
//! defined here, in the module that produces it, at the next free position.
//!
//! ## The glyph set is derived by shaping, never declared
//!
//! [`JamiAshkal`] is fed the finished layouts and nothing else. Every key in the
//! atlas is a `(font index, glyph id, size, mode, subpixel bucket)` that
//! shaping actually produced while laying out a real translated string at a real
//! discovered size.
//!
//! No Unicode range is enumerated anywhere in this module, and the omission is
//! Phase 2's rule rather than an optimisation. Enumerating ranges is wrong in
//! both directions at once: it misses every ligature and contextual alternate
//! the font would have produced — lam-alef, the `rlig` forms, the `calt`
//! alternates that are the difference between Naskh and a font pretending to be
//! Naskh — because those glyphs have no codepoint to enumerate; and it includes
//! thousands of glyphs nothing will ever draw, which is memory taken out of a
//! process that is not ours. Shaping the real strings produces exactly the set
//! that will be asked for. That is the entire reason a Taarib atlas is small.
//!
//! The same rule bans the small conveniences. No `.notdef` is added for every
//! font and size in case one is needed — when a chain fails to cover a
//! character, shaping emits `.notdef` and the collector picks it up from the
//! layout like any other glyph. No ellipsis is seeded for the truncation policy
//! — truncation inserts it during layout, so it is in the output already. A
//! glyph that is in the atlas is a glyph something drew.
//!
//! ## The overflow report is built here, from these layouts
//!
//! [`TakhtitMusbaq::tajawuz`] is produced in the same pass, from the same
//! [`TakhtitNass`] values the container records are made from, and from
//! nowhere else. The report used to be an input to the assembler, and every
//! caller that had no second layout pass to spend handed it an empty one — so
//! every package declared "no measured overflow" without a single string
//! having been measured. Building it here removes the field a caller could
//! leave empty: the numbers in the report are the numbers in the package,
//! because they are the same numbers.
//!
//! Every translated string reaches the report. One laid out at a size is
//! submitted at that size with its layout, and [`crate::taqrir_tajawuz`]
//! decides whether a recorded width exists to compare it against. One the
//! work list declined before any size took part — no size discovered, an
//! inline sprite, a hard break inside an atom, no container handle — is
//! recorded once with that cause. Only a string with no translation is left
//! out, because overflow of text that does not exist is not a question, and
//! coverage already counts it.
//!
//! ## The atlas is built through the gate, and only through the gate
//!
//! [`crate::bawwaba::rassim`] is the only call that produces pages, and
//! [`crate::bawwaba::SafhatMasmuha::min_lawha`] is the only thing that lets them
//! into a package. There is no [`taarib_ruqaa::katib::SafhaMabniya`] constructed
//! anywhere in this file, no `Vec<u8>` of texels assembled by hand, and no path
//! by which a byte that did not come out of a bundled font could reach either.
//!
//! ## What this refuses to lay out
//!
//! Three cases, each recorded as a [`NassBilaTakhtit`] rather than dropped.
//!
//! **An inline sprite.** [`NawNasq::Sura`] is drawn by the engine and occupies
//! width, and nothing in the project records what that width is. Phase 1's atom
//! needs a number; there is no honest number, so the string gets no precomputed
//! layout and the runtime path — where the engine knows its own sprite metrics —
//! handles it.
//!
//! **A mandatory line break inside an atom.** A width that is slightly wrong
//! displaces glyphs. A break that is missing changes how many lines there are,
//! and a flag cannot repair that, so the layout is not produced.
//!
//! **No discovered size.** The rule the previous module exists to enforce.
//!
//! ## What it lays out and flags instead
//!
//! A format placeholder is measured at the width of its own raw text — `%s`,
//! `{name}`, `\V[7]` — at the layout's size, through the same engine. That is a
//! real measurement of a real string, and it is right for a placeholder the game
//! leaves standing and wrong for one it substitutes into or acts on, which the
//! vocabulary does not distinguish. So the layout carries
//! [`ALAM_TAKHTIT_DHARRAT`], and an adapter that substitutes reads the flag and
//! lays the string out again. Stated in the record beats absent from it.
//!
//! ## Parallelism, and what `Saff` allows
//!
//! Each layout is independent of every other, so the stage is
//! [`rayon`]-parallel over the whole work list.
//!
//! [`Saff`] is not shared. Inspecting it, nothing in it is a `Cell`, a `RefCell`
//! or an `Rc`, so the auto-derived `Sync` bound very likely holds — and that is
//! not the question. Every entry point that lays anything out takes `&mut self`,
//! because the engine *is* a mutable cache of prepared shapers, so a shared
//! `&Saff` cannot produce a layout no matter what its bounds say. Its own
//! documentation settles it: hold one per thread.
//!
//! So this uses `map_init(Saff::jadeed, …)`: one engine per rayon worker,
//! created once and reused for every item that worker takes. Not one per item —
//! preparing a font's layout tables is the expensive half of shaping, and a
//! fresh engine per string would redo it for every string in the project, which
//! is the cost the cache exists to remove.
//!
//! Determinism survives the parallelism. `map_init` over a slice is an indexed
//! parallel iterator, so results collect back in work-list order, and the first
//! failure is chosen by scanning that order rather than by whichever worker
//! failed first — two compiles of one project report the same string.

use std::collections::{BTreeMap, BTreeSet};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use taarib_lawha::khareeta::{MiftahShakl, NamatSafha};
use taarib_lawha::rasf::abaad_masmuha;
use taarib_lawha::{JamiAshkal, KhiyaratRasf, Lawha};
use taarib_mustalahat::nass::{MudkhalNass, NassId, NawNasq, NitaqNasq, QuyudNass, TasnifNass};
use taarib_ruqaa::jadawil::{
    ALAM_HARF_ALAMA, ALAM_SATR_AKHIR, ALAM_SATR_YAMEEN, ALAM_TAKHTIT_BILA_QAYD_ARD,
    ALAM_TAKHTIT_DHARRAT, ALAM_TAKHTIT_MAQSUS, ALAM_TAKHTIT_MUSAGHGHAR, ALAM_TAKHTIT_TAJAWUZ,
    ALAM_TAKHTIT_YAMEEN, SijillHarf, SijillSatr,
};
use taarib_ruqaa::katib::{HuwiyatNass, KhattMabni, TakhtitMabni};
use taarib_saff::natija::{Harf, SatrMansuq, TakhtitNass};
use taarib_saff::talab::{
    Dharra, Ittijah, IttijahAsas, KhiyaratTakhtit, LughaNass, Muhadhaha, NamatDabt, NitaqUslub,
    SiyasatArqam, SiyasatTajawuz, SiyasatTashkeel, TalabTakhtit, Uslub,
};
use taarib_saff::{Saff, SilsilatKhutut};

use crate::bawwaba::{KhattMujammaa, SafhatMasmuha, rassim};
use crate::khata::{KhataTarqee, tul_u64};
use crate::tahdid_maqasat::{HajmMuqannan, SababLaHajm, TaqreerMaqasat};
use crate::taqrir_tajawuz::{BaniTaqrirTajawuz, MudkhalQiyas, SababAdamAltahaqquq, TaqrirTajawuz};

/// How many glyph slots the runtime atlas reserves beyond the compiled set,
/// unless the caller says otherwise.
///
/// Five hundred and twelve. The runtime path exists for what the compiler could
/// not see — a player's name, a number that reached five digits for the first
/// time, a line a capture session pulled out of a menu nobody extracted, a
/// string a mod added. That is a working set of dozens of glyphs at a handful of
/// sizes, not thousands: the Arabic letters a name is spelled from are already
/// in the compiled atlas, and what is genuinely new is the Latin and the digits
/// around them.
///
/// Five hundred and twelve is roughly an order of magnitude above that, which
/// buys headroom for a game that surprises the compiler without reserving a
/// second atlas's worth of a player's video memory for text that will never
/// exist.
pub const KHANAT_IQAMA_IFTIRADIYA: u32 = 512;

/// The largest reservation this compiler will record.
///
/// Eight thousand one hundred and ninety-two. Past this the honest reading is
/// not "the runtime needs more room" but "the compiler is missing most of the
/// game's text", and the fix is another extraction pass rather than a bigger
/// reservation. Recording an enormous budget would hide that: the atlas would
/// absorb the misses, the hit rate would look acceptable, and nobody would learn
/// that the patch is shaping half its strings at run time.
pub const AQSA_KHANAT_IQAMA: u32 = 8192;

/// The variable-font weight a bold span asks for.
///
/// Seven hundred is `Bold` in the OpenType weight class, and it is the value the
/// extraction side already treats as the boundary when it turns a weight back
/// into a bold span — so a string that round-trips through the vocabulary comes
/// back with the weight it went in with.
pub const WAZN_GHALIZ: u16 = 700;

/// How much of a string an error message carries.
const TUL_MUKHTASAR: usize = 48;

/// The width a string has, when anybody measured it.
///
/// Two states and no third, deliberately. A width is either something the
/// project measured or it is absent, and a type that could also hold "probably
/// about this much" is a type somebody will eventually put a guess in.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum QayasArd {
    /// A width was measured, in the game's own pixels.
    Maqis {
        /// The width.
        ard: f32,
    },
    /// No width was measured. The layout runs unwrapped and says so.
    Majhul,
}

impl QayasArd {
    /// Reads the width out of a string's constraints.
    ///
    /// [`QuyudNass::aqsa_ard`] only. `QuyudNass::mustatil` is deliberately not
    /// consulted — see the module header for why the rectangle the original
    /// occupied is not the box it was drawn in.
    #[must_use]
    pub fn min_quyud(quyud: &QuyudNass) -> Self {
        match quyud.aqsa_ard {
            Some(ard) if ard.is_finite() && ard > 0.0 => Self::Maqis { ard },
            _ => Self::Majhul,
        }
    }

    /// The width to hand the layout engine, which is [`None`] when unmeasured.
    #[must_use]
    pub const fn qeema(self) -> Option<f32> {
        match self {
            Self::Maqis { ard } => Some(ard),
            Self::Majhul => None,
        }
    }

    /// Whether the layout will run unwrapped.
    #[must_use]
    pub const fn majhul(self) -> bool {
        matches!(self, Self::Majhul)
    }
}

/// Why a string has no precomputed layout.
///
/// Tagged `naw` and not `sabab`, because [`SababLaTakhtit::BilaHajm`] carries a
/// [`SababLaHajm`] in a field named `sabab` and a tag of the same name would
/// write two keys called `sabab` into one JSON object — a document most parsers
/// accept and then silently keep half of.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "naw", rename_all = "snake_case")]
pub enum SababLaTakhtit {
    /// It has no translation yet. There is nothing to lay out.
    BilaTarjama,
    /// Its translation is empty while its source is not.
    ///
    /// Kept separate from [`SababLaTakhtit::BilaTarjama`] because it is a
    /// quality flag on the string rather than unfinished work, and the two want
    /// different actions from a contributor.
    TarjamaFarigha,
    /// No size was ever observed for it.
    BilaHajm {
        /// What size discovery said.
        sabab: SababLaHajm,
    },
    /// The caller supplied no container handle for it.
    ///
    /// Structural rather than editorial: a layout is bound to a string by the
    /// handle `taarib_ruqaa::katib::Katib` hands out, and a string that was
    /// never added to the writer has no handle to bind to. Reported instead of
    /// skipped silently, because it means the assembler and this stage were
    /// given different string tables.
    BilaHuwiya,
    /// It contains an inline sprite whose width nobody measured.
    SuraBilaQiyas {
        /// The engine's own reference to the sprite.
        marja: String,
    },
    /// It contains an atom carrying a mandatory line break.
    KasrSatrSarih {
        /// The atom's raw text.
        khaam: String,
    },
}

impl SababLaTakhtit {
    /// The sentence the compile report shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        match self {
            Self::BilaTarjama => "no translation to lay out".to_owned(),
            Self::TarjamaFarigha => "the translation is empty".to_owned(),
            Self::BilaHajm { sabab } => sabab.wasf(),
            Self::BilaHuwiya => "no container handle was supplied for this string".to_owned(),
            Self::SuraBilaQiyas { marja } => format!(
                "contains the inline sprite {marja}, whose width was never measured; laid out \
                 at run time where the engine knows its own sprite metrics"
            ),
            Self::KasrSatrSarih { khaam } => format!(
                "contains a mandatory line break inside the atom {khaam:?}, which a \
                 precomputed layout cannot express"
            ),
        }
    }

    /// Whether this string will be laid out by the runtime path instead.
    ///
    /// True for everything except the two that mean the string has no text to
    /// draw at all — a runtime path cannot lay out a translation nobody wrote.
    #[must_use]
    pub const fn ila_zaman_tashghil(&self) -> bool {
        !matches!(self, Self::BilaTarjama | Self::TarjamaFarigha)
    }

    /// What the overflow report records for a string this stage declined.
    ///
    /// [`None`] for the two causes that mean there is no translation: those
    /// strings are coverage's business and are not submitted for measurement.
    /// Every other cause is one entry, so the report can say the string was
    /// not checked and why.
    #[must_use]
    pub const fn sabab_adam_altahaqquq(&self) -> Option<SababAdamAltahaqquq> {
        match self {
            Self::BilaTarjama | Self::TarjamaFarigha => None,
            Self::BilaHajm { .. } => Some(SababAdamAltahaqquq::BilaHajm),
            Self::BilaHuwiya => Some(SababAdamAltahaqquq::BilaHuwiya),
            Self::SuraBilaQiyas { .. } => Some(SababAdamAltahaqquq::SuraBilaQiyas),
            Self::KasrSatrSarih { .. } => Some(SababAdamAltahaqquq::KasrSatrSarih),
        }
    }
}

/// A string that will not carry a precomputed layout, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NassBilaTakhtit {
    /// The string.
    pub nass: NassId,
    /// Why.
    pub sabab: SababLaTakhtit,
}

/// What the runtime adapter does when the atlas runs out of room.
///
/// Recorded in the package as data rather than compiled into an adapter,
/// because the adapters are five separate implementations in four languages and
/// a policy hardcoded in each is a policy that is different in each. A patch
/// that says what it wants gets the same behaviour from the C# adapter, the Ruby
/// one and the JavaScript one; a patch that says nothing gets whatever each
/// author assumed.
///
/// ## What a silently exhausted atlas looks like to a player
///
/// It does not look like an error. It looks like the game.
///
/// With no policy and no counter, an atlas that has run out either drops the
/// glyph — a word with a hole in it, or a line that renders as most of a
/// sentence — or reassigns a rectangle another glyph is still using, and then
/// one letter of one word is briefly some other letter. Both are intermittent,
/// both depend on how full the atlas happened to be in that scene, and both
/// produce the bug report "sometimes the text is wrong", which nothing
/// reproduces.
///
/// Every variant here is chosen against that. [`SiyasatNamu::Rafd`] turns a full
/// atlas into a named refusal that reaches the diagnostics screen.
/// [`SiyasatNamu::IkhlaFaqat`] and [`SiyasatNamu::NumuThummaIkhla`] both go
/// through the least-recently-used rectangle evictor in `taarib_lawha::namu`,
/// which never touches a rectangle the frame being drawn has referenced —
/// so the failure mode is a re-rasterization, which costs time and is counted,
/// rather than a wrong glyph, which costs nothing and is invisible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum SiyasatNamu {
    /// Open another page while the byte budget pays for one, and evict the
    /// least-recently-used rectangles after that.
    ///
    /// The default, and what the runtime atlas does natively. Growth is bounded
    /// by the budget rather than unbounded, and every page opened after the
    /// first is counted, so a patch that grows constantly is visible as a
    /// number rather than as an unexplained memory figure.
    #[default]
    NumuThummaIkhla,
    /// Never open another page; evict to make room inside the pages already
    /// allocated.
    ///
    /// For a platform where the text budget is fixed and exceeding it is worse
    /// than re-rasterizing: a console, a handheld, a game already close to its
    /// own limit. Thrash is possible and is what the eviction counter is for.
    IkhlaFaqat,
    /// Refuse: draw nothing for a glyph that will not fit, and report it.
    ///
    /// The strictest, and the right choice while a patch is being developed. A
    /// refusal names the size and the glyph in the diagnostics, which turns
    /// "the atlas is too small" into a compile the contributor can fix, instead
    /// of a cost the runtime quietly absorbs.
    Rafd,
}

impl SiyasatNamu {
    /// Whether the runtime may open pages beyond the ones it starts with.
    #[must_use]
    pub const fn yanmu(self) -> bool {
        matches!(self, Self::NumuThummaIkhla)
    }

    /// Whether the runtime may reclaim rectangles.
    #[must_use]
    pub const fn yukhli(self) -> bool {
        matches!(self, Self::NumuThummaIkhla | Self::IkhlaFaqat)
    }

    /// The sentence the diagnostics screen shows.
    #[must_use]
    pub const fn wasf(self) -> &'static str {
        match self {
            Self::NumuThummaIkhla => {
                "open another page while the budget allows, then evict the least recently used"
            },
            Self::IkhlaFaqat => "evict the least recently used; never open another page",
            Self::Rafd => "refuse the glyph and report it; never evict and never grow",
        }
    }
}

/// The residency plan a package carries for its runtime atlas.
///
/// Read by the adapter at load time and handed to
/// `taarib_lawha::namu::LawhaHayya::jadeeda`, which takes a budget in **bytes**
/// — so that is the unit here. A budget in pages means nothing until the page
/// size is known, and a budget in entries behaves completely differently
/// depending on whether the entries turn out to be ten thousand small glyphs or
/// four enormous ones.
///
/// The byte figure is not chosen; it is *derived*. The compiled atlas is
/// measured — total glyph area over glyph count — and the reservation is that
/// measured mean multiplied by the reserved slot count, rounded up to whole
/// pages because a page is the unit the runtime allocator actually buys. A patch
/// whose glyphs are large at the sizes it draws at therefore reserves more bytes
/// for the same number of slots, without anybody having to know that.
///
/// Both [`Serialize`] and [`Deserialize`], unlike everything in
/// [`crate::tahdid_maqasat`], and the difference is worth naming: a size carries
/// an invariant a document could forge, and this carries a plan. There is
/// nothing here a stored value could assert that it has not earned — the worst a
/// hand-edited budget produces is an atlas of the wrong size, which the growth
/// counters report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MizaniyatIqama {
    /// How many glyphs the compiled atlas holds.
    pub ashkal_mabniya: u32,
    /// What the compiled pages cost, in bytes, before compression.
    ///
    /// The pages as they ship, which is not the size they were packed at:
    /// `taarib_lawha::Lawha::ibni` returns the rows nothing reached, so a few
    /// hundred glyphs cost a few hundred kilobytes rather than the sixteen
    /// megabytes a 4096-row page was allocated as. This figure and
    /// [`MizaniyatIqama::bayt_safha`] therefore differ by a large factor and
    /// are both right — see that field for what it measures instead.
    pub bayt_mabniya: u64,
    /// How many pages the compiled atlas took.
    pub safahat_mabniya: u16,
    /// The measured mean bytes per compiled glyph, or [`None`] when there was
    /// no compiled set to measure.
    ///
    /// [`None`] is a real answer and not a zero: it means the reservation below
    /// was sized by the page floor rather than by measurement, and a reviewer
    /// reading a budget is entitled to know which.
    pub mutawassit_bayt_shakl: Option<u32>,
    /// How many glyph slots are reserved for the runtime path.
    pub khanat_mahjuza: u32,
    /// What those slots cost, in bytes, rounded up to whole pages.
    ///
    /// **This is the number the adapter passes to the runtime atlas.**
    pub bayt_mahjuza: u64,
    /// One **runtime** page's cost in bytes, after the packer's own page policy.
    ///
    /// The full page dimension, deliberately, and not what a compiled page
    /// measures after being cropped to its pack.
    /// `taarib_lawha::namu::LawhaHayya` opens pages at the declared maximum and
    /// never crops one — it exists to hold glyphs the compiler never saw, so
    /// there is no finished set to crop to — and it refuses a budget that will
    /// not pay for a whole one. A figure taken from the compiled pages would
    /// therefore describe an atlas the runtime would refuse to build.
    pub bayt_safha: u64,
    /// What the runtime does when it runs out.
    pub siyasa: SiyasatNamu,
    /// The rasterization mode, as the byte the container stores.
    pub namat: u8,
    /// Every size the runtime may be asked for, in quarter pixels.
    ///
    /// The union of the compiled sizes and the engine-wide sizes an adapter
    /// declared. An adapter warming its atlas has the list rather than having to
    /// wait for a miss at each size.
    pub ahjam_rubi: Vec<u16>,
}

impl MizaniyatIqama {
    /// The total byte budget for the runtime atlas.
    ///
    /// The compiled pages are uploaded once and are not part of this: the
    /// runtime atlas is a second atlas for glyphs the compiler never saw, and
    /// its budget is the reservation alone.
    #[must_use]
    pub const fn mizaniyat_zaman_tashghil(&self) -> u64 {
        self.bayt_mahjuza
    }

    /// How many runtime pages the reservation pays for.
    #[must_use]
    pub const fn safahat_mahjuza(&self) -> u64 {
        match self.bayt_mahjuza.checked_div(self.bayt_safha) {
            Some(adad) => adad,
            None => 0,
        }
    }

    /// The sentence the diagnostics screen and the package listing show.
    #[must_use]
    pub fn wasf(&self) -> String {
        let mutawassit = match self.mutawassit_bayt_shakl {
            Some(qeema) => format!("{qeema} measured bytes per compiled glyph"),
            None => "no compiled glyph to measure a per-glyph cost from".to_owned(),
        };
        // The two byte figures differ by a large factor and a reader is owed
        // the reason in the sentence rather than in a field comment: compiled
        // pages ship cropped to their pack, runtime pages are opened whole.
        format!(
            "{} compiled glyph(s) across {} page(s) shipping {} bytes ({mutawassit}); {} \
             runtime slot(s) reserved, costing {} bytes — {} whole runtime page(s) of {} \
             bytes each; on exhaustion, {}",
            self.ashkal_mabniya,
            self.safahat_mabniya,
            self.bayt_mabniya,
            self.khanat_mahjuza,
            self.bayt_mahjuza,
            self.safahat_mahjuza(),
            self.bayt_safha,
            self.siyasa.wasf(),
        )
    }
}

/// What precomputation was asked to do.
#[derive(Debug, Clone)]
pub struct KhiyaratTasbeeq {
    /// The layout decisions this patch records: direction, language,
    /// justification, diacritics, digits, overflow.
    pub takhtit: KhiyaratTakhtit,
    /// The atlas packing options.
    pub rasf: KhiyaratRasf,
    /// Coverage or distance field, for the whole patch.
    pub namat: NamatSafha,
    /// How many glyph slots to reserve for the runtime path.
    pub khanat_mahjuza: u32,
    /// What the runtime does when it runs out.
    pub siyasat_namu: SiyasatNamu,
}

impl Default for KhiyaratTasbeeq {
    fn default() -> Self {
        Self {
            takhtit: KhiyaratTakhtit::default(),
            rasf: KhiyaratRasf::default(),
            // Coverage, matching the atlas layer's own default: sharper at the
            // fixed small sizes interface text uses, and correct for a patch
            // that only ever draws at the sizes its compiler discovered.
            namat: NamatSafha::Taghtiya,
            khanat_mahjuza: KHANAT_IQAMA_IFTIRADIYA,
            siyasat_namu: SiyasatNamu::default(),
        }
    }
}

/// What precomputation did.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TaqreerTakhtit {
    /// How many layouts were produced.
    pub takhtitat: usize,
    /// How many strings carry at least one.
    pub nusus: usize,
    /// How many distinct glyph images the atlas holds.
    pub ashkal: usize,
    /// How many atlas pages.
    pub safahat: usize,
    /// The sizes present in the atlas, in quarter pixels, ascending.
    pub ahjam_rubi: Vec<u16>,
    /// How many layouts were computed with no width constraint.
    ///
    /// The number to read first. A patch where most layouts are unwrapped is a
    /// patch with no runtime capture behind it, and its line breaking is
    /// entirely the runtime's problem.
    pub bila_qayd_ard: usize,
    /// How many layouts contain format placeholders.
    pub bi_dharrat: usize,
    /// How many overflowed the bounds they were measured against.
    pub mutajawiza: usize,
    /// How many the overflow policy shrank.
    pub musaghghara: usize,
    /// How many the overflow policy truncated.
    pub maqsusa: usize,
    /// Strings with no precomputed layout, and why.
    pub matwiya: Vec<NassBilaTakhtit>,
}

impl TaqreerTakhtit {
    /// The sentence the compile report shows.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!(
            "{} layout(s) for {} string(s), {} of them unwrapped and {} carrying placeholders; \
             {} glyph image(s) at {} distinct size(s) across {} atlas page(s); {} overflowed, \
             {} were shrunk, {} were truncated; {} string(s) were not precomputed",
            self.takhtitat,
            self.nusus,
            self.bila_qayd_ard,
            self.bi_dharrat,
            self.ashkal,
            self.ahjam_rubi.len(),
            self.safahat,
            self.mutajawiza,
            self.musaghghara,
            self.maqsusa,
            self.matwiya.len(),
        )
    }

    /// How many skipped strings the runtime path will have to lay out.
    #[must_use]
    pub fn ila_zaman_tashghil(&self) -> usize {
        self.matwiya
            .iter()
            .filter(|bila| bila.sabab.ila_zaman_tashghil())
            .count()
    }
}

/// Everything precomputation produced.
///
/// Not [`Clone`]: it owns a [`SafhatMasmuha`], which owns the single
/// non-copyable proof that its bytes were generated by Taarib. A clonable
/// result would be a clonable proof, which is the property
/// [`crate::bawwaba`] removes on purpose.
#[derive(Debug)]
pub struct TakhtitMusbaq {
    takhtitat: Vec<TakhtitMabni>,
    ashkal: Vec<MiftahShakl>,
    safahat: Option<SafhatMasmuha>,
    khutut: Vec<KhattMabni>,
    iqama: MizaniyatIqama,
    taqreer: TaqreerTakhtit,
    tajawuz: TaqrirTajawuz,
}

impl TakhtitMusbaq {
    /// The layouts, ready for `taarib_ruqaa::katib::Katib::takhtit`.
    #[must_use]
    pub fn takhtitat(&self) -> &[TakhtitMabni] {
        &self.takhtitat
    }

    /// The overflow report, measured from these same layouts.
    ///
    /// Untrimmed: the passing rows are still present, so a caller beside the
    /// project can answer per string. [`TaqrirTajawuz::lil_huzma`] is the form
    /// the assembler puts in the package.
    #[must_use]
    pub const fn tajawuz(&self) -> &TaqrirTajawuz {
        &self.tajawuz
    }

    /// The glyph set the atlas was built from, sorted.
    #[must_use]
    pub fn ashkal(&self) -> &[MiftahShakl] {
        &self.ashkal
    }

    /// The atlas pages and their proof.
    ///
    /// [`None`] when no layout produced a single glyph, which is the correct
    /// answer for a project with nothing translated yet: an empty page in a
    /// package is a page a game uploads and never samples.
    #[must_use]
    pub const fn safahat(&self) -> Option<&SafhatMasmuha> {
        self.safahat.as_ref()
    }

    /// The font records the package declares, in chain order.
    #[must_use]
    pub fn khutut(&self) -> &[KhattMabni] {
        &self.khutut
    }

    /// The residency plan.
    #[must_use]
    pub const fn iqama(&self) -> &MizaniyatIqama {
        &self.iqama
    }

    /// What was produced and what was skipped.
    #[must_use]
    pub const fn taqreer(&self) -> &TaqreerTakhtit {
        &self.taqreer
    }

    /// Takes the pieces apart, for the assembler.
    ///
    /// Consuming rather than borrowing because the pages have to *move* into
    /// `crate::bawwaba::MuhtawaMasmuh::Safahat`: the proof they carry is not
    /// [`Clone`], and a borrowing accessor would leave the assembler with
    /// nothing it could put in a package. The overflow report travels with
    /// them so the assembler cannot write a package without it.
    #[must_use]
    pub fn ikhrij(
        self,
    ) -> (
        Vec<TakhtitMabni>,
        Option<SafhatMasmuha>,
        Vec<KhattMabni>,
        TaqrirTajawuz,
    ) {
        (self.takhtitat, self.safahat, self.khutut, self.tajawuz)
    }
}

/// Precomputes every layout the discovered sizes call for, and builds the atlas
/// from the glyphs those layouts produced.
///
/// `huwiyat` maps a string identity to the handle
/// `taarib_ruqaa::katib::Katib::nass` returned when it was added to the writer.
/// This stage takes the map rather than a writer because a layout is bound to a
/// string by that handle and by nothing else: a stage that added strings itself
/// could produce a layout attached to a string the assembler never added, and
/// the container has no way to notice.
///
/// `khutut` must be the same fonts as `silsila`, in the same order.
/// `taarib_saff::natija::Harf::khatt` is an index into the chain, and the
/// container stores it verbatim, so a chain and a font list that disagree
/// produce a patch whose every glyph names the wrong face.
///
/// # Errors
///
/// [`KhataTarqee::BayanNaqis`] when the font list and the shaping chain are not
/// the same length, which is the disagreement above and is refused before a
/// single glyph is shaped.
///
/// [`KhataTarqee::TakhtitFashil`] when a string will not lay out: a style span
/// that does not fit its text, markup nested past the bidirectional algorithm's
/// limit, a font that cannot shape the run it was given. A hard check — the
/// package is not written — because a string that will not lay out here is a
/// string that will not draw in the game either.
///
/// [`KhataTarqee::RasfFashil`] when the atlas will not build: a glyph set larger
/// than the page budget, a font that will not rasterize.
pub fn sabbiq(
    nusus: &[MudkhalNass],
    huwiyat: &BTreeMap<NassId, HuwiyatNass>,
    maqasat: &TaqreerMaqasat,
    silsila: &SilsilatKhutut,
    khutut: &[KhattMujammaa],
    khiyarat: &KhiyaratTasbeeq,
) -> Result<TakhtitMusbaq, KhataTarqee> {
    if khutut.len() != silsila.adad() {
        return Err(KhataTarqee::BayanNaqis {
            haql: "one bundled font record for every font in the shaping chain",
        });
    }

    // Built once. The alternative is a linear scan of the refusal list per
    // string, which is quadratic on exactly the project where most strings were
    // never measured — the one this lookup exists for.
    let asbab: BTreeMap<NassId, SababLaHajm> = maqasat
        .bila_hajm()
        .iter()
        .map(|bila| (bila.nass, bila.sabab))
        .collect();

    let mut wahdat: Vec<WahdatTakhtit<'_>> = Vec::new();
    let mut matwiya: Vec<NassBilaTakhtit> = Vec::new();
    let mut bani = BaniTaqrirTajawuz::jadeed();
    for mudkhal in nusus {
        match wahhid(mudkhal, huwiyat, maqasat, &asbab) {
            Ok(jadida) => wahdat.extend(jadida),
            Err(sabab) => {
                if let Some(adam) = sabab.sabab_adam_altahaqquq() {
                    bani.sajjil_bila_takhtit(mudkhal, adam);
                }
                matwiya.push(NassBilaTakhtit {
                    nass: mudkhal.id,
                    sabab,
                });
            },
        }
    }

    let qiyas = khiyarat_qiyas_dharra(&khiyarat.takhtit);
    let siyaq = SiyaqTasbeeq {
        silsila,
        khiyarat: &khiyarat.takhtit,
        qiyas: &qiyas,
    };

    // One `Saff` per rayon worker, reused across every item that worker takes.
    // See the module header for why this is `map_init` and not a shared engine.
    let natayij: Vec<Result<NatijatWahda, KhataTarqee>> = wahdat
        .par_iter()
        .map_init(Saff::jadeed, |saff, wahda| {
            khattit_wahda(saff, wahda, &siyaq)
        })
        .collect();

    let mut jami = JamiAshkal::jadeed(khiyarat.namat);
    let mut takhtitat: Vec<TakhtitMabni> = Vec::with_capacity(natayij.len());
    let mut asma: BTreeSet<NassId> = BTreeSet::new();
    // Scanned in work-list order, so the failure reported is the first one in
    // the project and not the first one a worker happened to reach. The
    // indexed parallel map keeps results in work-list order, which is what
    // lets each layout be paired back with the string it was made from.
    for (wahda, natija) in wahdat.iter().zip(natayij) {
        let natija = natija?;
        jami.idif_takhtit(&natija.takhtit, natija.hajm_matlub.biksal());
        let _ = asma.insert(natija.nass);
        // Submitted with the requested size, not the settled one: the report
        // records what was asked for and reads the settled size off the layout.
        bani.sajjil(&MudkhalQiyas {
            madkhal: wahda.mudkhal,
            hajm: natija.hajm_matlub.biksal(),
            takhtit: Some(&natija.takhtit),
            takhtit_asl: None,
        });
        takhtitat.push(ila_mabni(&natija));
    }
    let tajawuz = bani.ikhtim();

    let ashkal = jami.ashkal();
    let (safahat, lawha) = if ashkal.is_empty() {
        // No glyph was produced, so there is nothing to rasterize and no proof
        // to mint. An empty page in a package is a page a game uploads and
        // never samples.
        (None, None)
    } else {
        let (mabniya, ithbat) = rassim(&ashkal, khutut, silsila, khiyarat.rasf, khiyarat.namat)?;
        let masmuha = SafhatMasmuha::min_lawha(&mabniya, ithbat);
        (Some(masmuha), Some(mabniya))
    };

    let mut ahjam_lawha: Vec<u16> = ashkal.iter().map(|miftah| miftah.hajm_rubi).collect();
    ahjam_lawha.sort_unstable();
    ahjam_lawha.dedup();

    let mut ahjam_iqama = ahjam_lawha.clone();
    ahjam_iqama.extend(maqasat.ahjam_iqama().iter().map(|hajm| hajm.rubi()));
    ahjam_iqama.sort_unstable();
    ahjam_iqama.dedup();

    let iqama = qis_iqama(lawha.as_ref(), khiyarat, ahjam_iqama)?;

    matwiya.sort_by_key(|bila| bila.nass);
    let taqreer = TaqreerTakhtit {
        takhtitat: takhtitat.len(),
        nusus: asma.len(),
        ashkal: ashkal.len(),
        safahat: safahat
            .as_ref()
            .map_or(0, |safahat| safahat.safahat().len()),
        ahjam_rubi: ahjam_lawha,
        bila_qayd_ard: adad_bi_alam(&takhtitat, ALAM_TAKHTIT_BILA_QAYD_ARD),
        bi_dharrat: adad_bi_alam(&takhtitat, ALAM_TAKHTIT_DHARRAT),
        mutajawiza: adad_bi_alam(&takhtitat, ALAM_TAKHTIT_TAJAWUZ),
        musaghghara: adad_bi_alam(&takhtitat, ALAM_TAKHTIT_MUSAGHGHAR),
        maqsusa: adad_bi_alam(&takhtitat, ALAM_TAKHTIT_MAQSUS),
        matwiya,
    };

    tracing::info!(
        takhtitat = taqreer.takhtitat,
        nusus = taqreer.nusus,
        ashkal = taqreer.ashkal,
        safahat = taqreer.safahat,
        bila_qayd_ard = taqreer.bila_qayd_ard,
        matwiya = taqreer.matwiya.len(),
        tajawuz = %tajawuz.wasf_injilizi(),
        "precomputation finished"
    );

    Ok(TakhtitMusbaq {
        takhtitat,
        ashkal,
        safahat,
        khutut: khutut.iter().map(KhattMujammaa::sijill).collect(),
        iqama,
        taqreer,
        tajawuz,
    })
}

// ---------------------------------------------------------------------------
// The work list
// ---------------------------------------------------------------------------

/// One string at one size: everything a layout call needs, resolved once.
#[derive(Debug, Clone)]
struct WahdatTakhtit<'a> {
    /// The string's identity, for the report.
    nass: NassId,
    /// The whole entry, which the overflow report reads its constraint and its
    /// class from once the layout exists.
    mudkhal: &'a MudkhalNass,
    /// Its handle in the container being built.
    huwiya: HuwiyatNass,
    /// The clean Arabic text.
    hadaf: &'a str,
    /// Markup and placeholders over that text.
    nasq: &'a [NitaqNasq],
    /// The width available, measured or absent.
    ard: QayasArd,
    /// The height available, when one was measured.
    irtifa_mutah: Option<f32>,
    /// Whether the engine refuses to wrap this string.
    satr_wahid: bool,
    /// Whether this string is dialogue, which the diacritic policy keys on.
    hiwar: bool,
    /// Whether it carries format placeholders.
    dharrat: bool,
    /// The size to lay it out at.
    hajm: HajmMuqannan,
}

/// One finished layout, before it becomes a container record.
#[derive(Debug)]
struct NatijatWahda {
    /// The string's identity, which is what the report counts by. Counted by
    /// identity and not by handle because the report is read beside the project
    /// and a handle means nothing outside the container being built.
    nass: NassId,
    /// The string's handle.
    huwiya: HuwiyatNass,
    /// The size that was asked for.
    hajm_matlub: HajmMuqannan,
    /// The size it settled on, which differs when the overflow policy shrank it.
    hajm_nihai: HajmMuqannan,
    /// The `ALAM_TAKHTIT_*` bits.
    alam: u16,
    /// The layout.
    takhtit: TakhtitNass,
}

/// What every layout call in one compile shares.
#[derive(Debug, Clone, Copy)]
struct SiyaqTasbeeq<'a> {
    /// The fonts.
    silsila: &'a SilsilatKhutut,
    /// The patch's own layout decisions.
    khiyarat: &'a KhiyaratTakhtit,
    /// The neutral options a placeholder's own text is measured under.
    qiyas: &'a KhiyaratTakhtit,
}

/// Turns one string into one work item per discovered size.
fn wahhid<'a>(
    mudkhal: &'a MudkhalNass,
    huwiyat: &BTreeMap<NassId, HuwiyatNass>,
    maqasat: &TaqreerMaqasat,
    asbab: &BTreeMap<NassId, SababLaHajm>,
) -> Result<Vec<WahdatTakhtit<'a>>, SababLaTakhtit> {
    let Some(hadaf) = mudkhal.hadaf.as_deref() else {
        return Err(SababLaTakhtit::BilaTarjama);
    };
    if hadaf.is_empty() {
        return Err(SababLaTakhtit::TarjamaFarigha);
    }
    let Some(huwiya) = huwiyat.get(&mudkhal.id).copied() else {
        return Err(SababLaTakhtit::BilaHuwiya);
    };
    let Some(maqas) = maqasat.maqasat_nass(mudkhal.id) else {
        // The reason travels from size discovery rather than being restated
        // here. "No size" and "the size was 0.0" are different news, and this
        // stage is not the one that knows which.
        let sabab = asbab
            .get(&mudkhal.id)
            .copied()
            .unwrap_or(SababLaHajm::LamYuqas);
        return Err(SababLaTakhtit::BilaHajm { sabab });
    };

    let dharrat = fahs_nasq(&mudkhal.nasq_hadaf)?;
    let ard = QayasArd::min_quyud(&mudkhal.quyud);
    let irtifa_mutah = mudkhal
        .quyud
        .aqsa_irtifa
        .filter(|qeema| qeema.is_finite() && *qeema > 0.0);

    Ok(maqas
        .shuhud
        .iter()
        .map(|shahid| WahdatTakhtit {
            nass: mudkhal.id,
            mudkhal,
            huwiya,
            hadaf,
            nasq: &mudkhal.nasq_hadaf,
            ard,
            irtifa_mutah,
            satr_wahid: mudkhal.quyud.satr_wahid,
            hiwar: matches!(mudkhal.tasnif, TasnifNass::Hiwar),
            dharrat,
            hajm: shahid.hajm,
        })
        .collect())
}

/// Whether a string's spans can be laid out at all, and whether they hold
/// placeholders.
///
/// Run once per string rather than once per size, so a refusal produces one
/// report entry instead of one per discovered size.
///
/// The two refusals are the two things a precomputed layout cannot express
/// honestly. An inline sprite occupies width nobody recorded, and the layout
/// after it would be positioned from a number this compiler invented. A
/// mandatory line break inside an atom changes how many lines there are, and
/// unlike a wrong width — which displaces glyphs and can be flagged — a missing
/// break moves every baseline after it and there is no flag that repairs that.
fn fahs_nasq(nasq: &[NitaqNasq]) -> Result<bool, SababLaTakhtit> {
    let mut dharrat = false;
    for nitaq in nasq {
        match &nitaq.naw {
            NawNasq::Sura { marja } => {
                return Err(SababLaTakhtit::SuraBilaQiyas {
                    marja: marja.clone(),
                });
            },
            NawNasq::Satr => {
                return Err(SababLaTakhtit::KasrSatrSarih {
                    khaam: "\n".to_owned(),
                });
            },
            NawNasq::Mawdi { khaam } => {
                if khaam.chars().any(huwa_kasr_satr) {
                    return Err(SababLaTakhtit::KasrSatrSarih {
                        khaam: khaam.clone(),
                    });
                }
                dharrat = true;
            },
            // Listed rather than wildcarded. A new markup construct added to
            // the vocabulary is a construct this function has never decided
            // about, and a `_` arm would decide "it is fine to precompute"
            // on its behalf, silently, for every patch built afterwards.
            NawNasq::Lawn { .. }
            | NawNasq::Ghaliz
            | NawNasq::Maail
            | NawNasq::TahtKhat
            | NawNasq::Shatb
            | NawNasq::Hajm { .. }
            | NawNasq::Khatt { .. }
            | NawNasq::Rabt { .. }
            | NawNasq::Tawaqquf { .. }
            | NawNasq::BilaTahleel
            | NawNasq::Muhadhaha { .. } => {},
        }
    }
    Ok(dharrat)
}

/// Whether a character is a mandatory break under UAX #14.
///
/// Tests real control characters, not the two-character escapes several script
/// formats write. RPG Maker's `\n[1]` is a backslash and a letter — a name
/// substitution, not a break — and treating it as one would refuse half a
/// project for a break that is not there.
const fn huwa_kasr_satr(harf: char) -> bool {
    matches!(
        harf,
        '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

// ---------------------------------------------------------------------------
// One layout
// ---------------------------------------------------------------------------

/// Lays one string out at one size.
fn khattit_wahda(
    saff: &mut Saff,
    wahda: &WahdatTakhtit<'_>,
    siyaq: &SiyaqTasbeeq<'_>,
) -> Result<NatijatWahda, KhataTarqee> {
    let hajm = wahda.hajm.biksal();
    let nitaqat = hayyi_nitaqat(saff, wahda, siyaq, hajm)?;

    // Cloned per item rather than shared, because two of its fields are
    // properties of the string and not of the patch: a single-line field must
    // not wrap whatever the patch's default is, and the diacritic policy's
    // dialogue mode is meaningless without knowing which strings are dialogue.
    let mut khiyarat = siyaq.khiyarat.clone();
    khiyarat.satr_wahid = siyaq.khiyarat.satr_wahid || wahda.satr_wahid;
    khiyarat.hiwar = wahda.hiwar;

    let talab = TalabTakhtit {
        nass: wahda.hadaf,
        khutut: siyaq.silsila,
        hajm,
        // The measured width, or nothing at all. Never a substitute.
        ard_mutah: wahda.ard.qeema(),
        irtifa_mutah: wahda.irtifa_mutah,
        nitaqat: &nitaqat,
        khiyarat: &khiyarat,
    };
    let takhtit = saff
        .khattit(&talab)
        .map_err(|khata| KhataTarqee::TakhtitFashil {
            nass: mukhtasar(wahda.hadaf),
            hajm,
            sabab: khata.injilizi,
        })?;

    // The size the layout settled on, which is what the container records and
    // what the atlas was keyed by — not the size that was requested. A layout
    // the overflow policy shrank is drawn at the shrunk size, and storing the
    // requested one would put the wrong image beside the right positions.
    let hajm_nihai = HajmMuqannan::min_biksal(takhtit.hajm).unwrap_or(wahda.hajm);

    let mut alam = 0u16;
    if matches!(takhtit.ittijah, Ittijah::Yameen) {
        alam |= ALAM_TAKHTIT_YAMEEN;
    }
    if takhtit.maqsus {
        alam |= ALAM_TAKHTIT_MAQSUS;
    }
    if takhtit.tajawuz.is_some() {
        alam |= ALAM_TAKHTIT_TAJAWUZ;
    }
    // Compared as the quarter-pixel integers both sizes are already on, so the
    // comparison is exact and there is no float equality anywhere in it.
    if hajm_nihai.rubi() < wahda.hajm.rubi() {
        alam |= ALAM_TAKHTIT_MUSAGHGHAR;
    }
    if wahda.ard.majhul() {
        alam |= ALAM_TAKHTIT_BILA_QAYD_ARD;
    }
    if wahda.dharrat {
        alam |= ALAM_TAKHTIT_DHARRAT;
    }

    Ok(NatijatWahda {
        nass: wahda.nass,
        huwiya: wahda.huwiya,
        hajm_matlub: wahda.hajm,
        hajm_nihai,
        alam,
        takhtit,
    })
}

/// Turns the project's markup vocabulary into the layout engine's style spans.
///
/// Only what changes *layout* is translated. Colour, underline, strikethrough,
/// links, typewriter pauses and no-parse regions all become spans that set
/// nothing: the span still exists, so its identifier survives onto every glyph
/// it covers through `Harf::nitaq`, and an adapter drawing an underline knows
/// which glyphs to draw it under.
///
/// Three mappings are **declined** rather than guessed, and each one costs less
/// than the guess would have.
///
/// A [`NawNasq::Khatt`] naming a font *family* is not resolved to a chain index.
/// The vocabulary records `#3` when the span round-tripped through extraction,
/// and that is read; a real family name — `NotoSansArabic-Bold` — has no
/// recorded mapping to a position in this patch's chain, and picking one would
/// shape a word with the wrong face in a way nothing reports.
///
/// A [`NawNasq::Muhadhaha`] is not applied. Alignment in this engine is a
/// leading or trailing edge, and the vocabulary carries the engine's own word —
/// `right`, `left` — whose meaning depends on the base direction the *engine*
/// resolved, which is not recorded. Mapping `right` to the leading edge is
/// correct for Arabic and wrong for the Latin run beside it, and the layout
/// would be silently mirrored.
///
/// A colour name the table does not know becomes no colour. Colour has no
/// effect on layout, and the package carries the span's own recorded value for
/// the adapter regardless, so declining here loses nothing.
fn hayyi_nitaqat(
    saff: &mut Saff,
    wahda: &WahdatTakhtit<'_>,
    siyaq: &SiyaqTasbeeq<'_>,
    hajm: f32,
) -> Result<Vec<NitaqUslub>, KhataTarqee> {
    let mut nitaqat: Vec<NitaqUslub> = Vec::with_capacity(wahda.nasq.len());
    for nitaq in wahda.nasq {
        let uslub = match &nitaq.naw {
            NawNasq::Lawn { qeema } => Uslub {
                lawn: lawn_min_nass(qeema),
                ..Uslub::default()
            },
            NawNasq::Ghaliz => Uslub {
                wazn: Some(WAZN_GHALIZ),
                ..Uslub::default()
            },
            NawNasq::Maail => Uslub {
                maail: true,
                ..Uslub::default()
            },
            NawNasq::Hajm { qeema } => Uslub {
                hajm: hajm_nitaq(*qeema),
                ..Uslub::default()
            },
            NawNasq::Khatt { ism } => Uslub {
                khatt: fahras_khatt(ism, siyaq.silsila),
                ..Uslub::default()
            },
            NawNasq::Mawdi { khaam } => {
                let ard = qis_dharra(saff, khaam, siyaq, hajm)?;
                Uslub {
                    dharra: Some(Dharra {
                        ard,
                        // Zero height and zero baseline offset, matching what
                        // `taarib_saff::nasq` gives its own atoms: a
                        // placeholder does not raise the line above what the
                        // font's own metrics ask for, and claiming it does
                        // would move every baseline under it.
                        irtifa: 0.0,
                        asas: 0.0,
                        marja: u32::from(nitaq.id),
                    }),
                    ..Uslub::default()
                }
            },
            // Refused when the work list was built, and refused again here
            // rather than given a default: there is no honest width for a
            // sprite, so there is no default to fall back to.
            NawNasq::Sura { marja } => {
                return Err(KhataTarqee::TakhtitFashil {
                    nass: mukhtasar(wahda.hadaf),
                    hajm,
                    sabab: format!(
                        "the inline sprite {marja} reached layout with no measured width"
                    ),
                });
            },
            // No layout effect. The span is still emitted, so its identifier
            // reaches the glyphs it covers. `Satr` is here rather than beside
            // the sprite because a span that styles nothing *is* the honest
            // answer for it — what a hard break needs is a line boundary, and
            // that was refused before the work list was built.
            NawNasq::TahtKhat
            | NawNasq::Shatb
            | NawNasq::Rabt { .. }
            | NawNasq::Satr
            | NawNasq::Tawaqquf { .. }
            | NawNasq::BilaTahleel
            | NawNasq::Muhadhaha { .. } => Uslub::default(),
        };
        nitaqat.push(NitaqUslub {
            id: nitaq.id,
            bidaya: nitaq.bidaya,
            tul: nitaq.tul,
            uslub,
        });
    }
    Ok(nitaqat)
}

/// The options a placeholder's own raw text is measured under.
///
/// Deliberately not the patch's options. A placeholder is text the game
/// substitutes into or acts on, and it survives translation byte for byte — so
/// the digit policy must not rewrite the `1` in `%1$s` into an Arabic-Indic
/// digit before measuring it, and the diacritic policy has nothing to strip.
/// Direction and language are left on detection, because a placeholder is
/// usually Latin inside an Arabic string and forcing the paragraph's language
/// on it would shape it under `locl` rules meant for the prose around it.
///
/// Justification, alignment and the spacing additions are all off: those are
/// properties of a line, and an atom is being measured on its own.
fn khiyarat_qiyas_dharra(asas: &KhiyaratTakhtit) -> KhiyaratTakhtit {
    KhiyaratTakhtit {
        ittijah: IttijahAsas::Tilqai,
        lugha: LughaNass::Tilqai,
        dabt: NamatDabt::Bila,
        muhadhaha: Muhadhaha::Bidaya,
        tashkeel: SiyasatTashkeel::Ibqa,
        arqam: SiyasatArqam::KamaHiya,
        tajawuz: SiyasatTajawuz::Ballagh,
        irtifa_satr: asas.irtifa_satr,
        tabaud_ahruf: 0.0,
        tabaud_kalimat: 0.0,
        hiwar: false,
        satr_wahid: true,
        // The patch's feature overrides are kept: a patch that turns off a
        // ligature turns it off everywhere, and measuring an atom with a
        // different feature set would measure something the patch will not
        // draw.
        sifat: asas.sifat.clone(),
    }
}

/// Measures a placeholder's own raw text.
///
/// This is a real measurement of a real string, and it is the *only* width this
/// compiler has for an atom. It is right for a placeholder the engine leaves
/// standing, and wrong for one it substitutes into, and the vocabulary does not
/// distinguish the two — which is what [`ALAM_TAKHTIT_DHARRAT`] on the finished
/// layout exists to say out loud.
fn qis_dharra(
    saff: &mut Saff,
    khaam: &str,
    siyaq: &SiyaqTasbeeq<'_>,
    hajm: f32,
) -> Result<f32, KhataTarqee> {
    if khaam.is_empty() {
        return Ok(0.0);
    }
    let talab = TalabTakhtit {
        nass: khaam,
        khutut: siyaq.silsila,
        hajm,
        ard_mutah: None,
        irtifa_mutah: None,
        nitaqat: &[],
        khiyarat: siyaq.qiyas,
    };
    let qiyas = saff
        .qis(&talab)
        .map_err(|khata| KhataTarqee::TakhtitFashil {
            nass: mukhtasar(khaam),
            hajm,
            sabab: khata.injilizi,
        })?;
    Ok(if qiyas.ard.is_finite() && qiyas.ard > 0.0 {
        qiyas.ard
    } else {
        0.0
    })
}

/// A span's size override, on the same quarter-pixel grid as everything else.
///
/// Quantized here too, so a `<size=13.0001>` span does not open a second glyph
/// set beside the `<size=13>` one in the string next to it. An unusable value
/// becomes [`None`], which makes the span inherit the run's size rather than
/// poison the layout with it.
fn hajm_nitaq(qeema: f32) -> Option<f32> {
    HajmMuqannan::min_biksal(qeema)
        .ok()
        .map(HajmMuqannan::biksal)
}

/// A font span's chain index, when the span names one.
///
/// `#3` is what extraction writes when a layout span round-trips through the
/// vocabulary, and it is the only form that can be resolved: it is already an
/// index into this same chain. Anything else is a family name with no recorded
/// mapping, and is declined.
fn fahras_khatt(ism: &str, silsila: &SilsilatKhutut) -> Option<u8> {
    let raqm = ism.strip_prefix('#')?.parse::<u8>().ok()?;
    (usize::from(raqm) < silsila.adad()).then_some(raqm)
}

/// A colour span's value, as `#rrggbb`, `#rgb`, `#rrggbbaa`, or a name.
fn lawn_min_nass(qeema: &str) -> Option<[u8; 4]> {
    let munaqqa = qeema.trim();
    munaqqa.strip_prefix('#').map_or_else(
        || lawn_bil_ism(munaqqa).or_else(|| lawn_min_sittasi(munaqqa)),
        lawn_min_sittasi,
    )
}

/// A hexadecimal colour, in any of the four lengths the dialects write.
fn lawn_min_sittasi(rumuz: &str) -> Option<[u8; 4]> {
    let mut arqam: Vec<u8> = Vec::with_capacity(8);
    for harf in rumuz.chars() {
        let raqm = harf.to_digit(16)?;
        arqam.push(u8::try_from(raqm).ok()?);
    }
    // Slice patterns rather than indices: the length is the discriminant, and
    // matching on it is what makes every arm total.
    match arqam.as_slice() {
        [ahmar, akhdar, azraq] => Some([dabl(*ahmar), dabl(*akhdar), dabl(*azraq), u8::MAX]),
        [ahmar, akhdar, azraq, shaffaf] => {
            Some([dabl(*ahmar), dabl(*akhdar), dabl(*azraq), dabl(*shaffaf)])
        },
        [ah1, ah0, ak1, ak0, az1, az0] => {
            Some([dam(*ah1, *ah0), dam(*ak1, *ak0), dam(*az1, *az0), u8::MAX])
        },
        [ah1, ah0, ak1, ak0, az1, az0, sh1, sh0] => Some([
            dam(*ah1, *ah0),
            dam(*ak1, *ak0),
            dam(*az1, *az0),
            dam(*sh1, *sh0),
        ]),
        _ => None,
    }
}

/// One hexadecimal digit as a byte: `f` becomes `ff`, the way every shorthand
/// colour notation defines it.
const fn dabl(raqm: u8) -> u8 {
    raqm.saturating_mul(17)
}

/// Two hexadecimal digits as a byte.
const fn dam(aala: u8, adna: u8) -> u8 {
    aala.saturating_mul(16).saturating_add(adna)
}

/// The named colours the rich-text dialects define.
///
/// The engines' own table, not a palette invented here. A name outside it is
/// declined rather than approximated — an approximate colour is a colour
/// somebody has to notice is wrong.
fn lawn_bil_ism(ism: &str) -> Option<[u8; 4]> {
    let saghir = ism.to_ascii_lowercase();
    let (ahmar, akhdar, azraq) = match saghir.as_str() {
        "aqua" | "cyan" => (0x00, 0xFF, 0xFF),
        "black" => (0x00, 0x00, 0x00),
        "blue" => (0x00, 0x00, 0xFF),
        "brown" => (0xA5, 0x2A, 0x2A),
        "darkblue" => (0x00, 0x00, 0xA0),
        "fuchsia" | "magenta" => (0xFF, 0x00, 0xFF),
        "green" => (0x00, 0x80, 0x00),
        "grey" | "gray" => (0x80, 0x80, 0x80),
        "lightblue" => (0xAD, 0xD8, 0xE6),
        "lime" => (0x00, 0xFF, 0x00),
        "maroon" => (0x80, 0x00, 0x00),
        "navy" => (0x00, 0x00, 0x80),
        "olive" => (0x80, 0x80, 0x00),
        "orange" => (0xFF, 0xA5, 0x00),
        "pink" => (0xFF, 0xC0, 0xCB),
        "purple" => (0x80, 0x00, 0x80),
        "red" => (0xFF, 0x00, 0x00),
        "silver" => (0xC0, 0xC0, 0xC0),
        "teal" => (0x00, 0x80, 0x80),
        "white" => (0xFF, 0xFF, 0xFF),
        "yellow" => (0xFF, 0xFF, 0x00),
        _ => return None,
    };
    Some([ahmar, akhdar, azraq, u8::MAX])
}

// ---------------------------------------------------------------------------
// Engine output to container record
// ---------------------------------------------------------------------------

/// Turns a finished layout into the record the container stores.
fn ila_mabni(natija: &NatijatWahda) -> TakhtitMabni {
    TakhtitMabni {
        nass: natija.huwiya,
        hajm_rubi: natija.hajm_nihai.rubi(),
        ard: natija.takhtit.ard,
        irtifa: natija.takhtit.irtifa,
        alam: natija.alam,
        huruf: natija.takhtit.huruf.iter().map(sijill_harf).collect(),
        sutur: natija.takhtit.sutur.iter().map(sijill_satr).collect(),
    }
}

/// One positioned glyph, engine form to stored form.
///
/// Every field carries across unchanged except the mark flag, which is a `bool`
/// in the engine and a bit in the record because the record is a fixed-width
/// struct that C#, JavaScript, Python and Ruby all read by overlay.
///
/// Note what is not translated, because there is nothing to translate: `anqud`
/// is a byte offset into the same clean text the container stores, and `nitaq`
/// is the span identifier this module put on the span it came from. Both are
/// already in the container's own terms.
const fn sijill_harf(harf: &Harf) -> SijillHarf {
    SijillHarf {
        muarrif: harf.muarrif,
        anqud: harf.anqud,
        s: harf.s,
        a: harf.a,
        taqaddum: harf.taqaddum,
        nitaq: harf.nitaq,
        khatt: harf.khatt,
        alam: if harf.alama { ALAM_HARF_ALAMA } else { 0 },
    }
}

/// One line, engine form to stored form.
///
/// `awwal_harf` stays **relative to this layout's own glyph array**, which is
/// what the reader expects: `taarib_ruqaa::qari::takhtit_wahid` slices the
/// section's glyphs down to one layout first and then hands a line's indices
/// into that slice. The writer concatenates layouts and rebases each layout's
/// head, and never touches a line's index — so rebasing here would double the
/// offset and every line after the first would draw another line's glyphs.
const fn sijill_satr(satr: &SatrMansuq) -> SijillSatr {
    let mut alam = 0u32;
    if satr.akhir {
        alam |= ALAM_SATR_AKHIR;
    }
    if matches!(satr.ittijah, Ittijah::Yameen) {
        alam |= ALAM_SATR_YAMEEN;
    }
    SijillSatr {
        awwal_harf: satr.huruf.start,
        adad_huruf: satr.huruf.end.saturating_sub(satr.huruf.start),
        bidayat_mantiqi: satr.mantiqi.start,
        nihayat_mantiqi: satr.mantiqi.end,
        asas: satr.asas,
        bidaya: satr.bidaya,
        ard: satr.ard,
        irtifa: satr.irtifa,
        suud: satr.suud,
        hubut: satr.hubut,
        dabt: satr.dabt,
        alam,
    }
}

// ---------------------------------------------------------------------------
// The residency budget
// ---------------------------------------------------------------------------

/// Derives the runtime atlas's budget from the compiled one.
///
/// Nothing here is chosen. The per-glyph cost is the compiled atlas's own
/// occupied area divided by its own glyph count — a measurement of this patch's
/// fonts at this patch's sizes, so a patch drawing at forty pixels reserves more
/// bytes for the same slot count than one drawing at twelve, without anybody
/// having to encode that relationship.
///
/// The one figure that is not measured is the floor. When there is no compiled
/// glyph there is nothing to measure, and the reservation becomes a single page
/// — not because a page is the right amount, but because
/// `taarib_lawha::namu::LawhaHayya` refuses a budget that will not pay for one
/// page, and an atlas with no page cannot hold a glyph. That case is visible in
/// the record: `mutawassit_bayt_shakl` is [`None`], which says the number below
/// it came from the floor rather than from a measurement.
fn qis_iqama(
    lawha: Option<&Lawha>,
    khiyarat: &KhiyaratTasbeeq,
    ahjam_rubi: Vec<u16>,
) -> Result<MizaniyatIqama, KhataTarqee> {
    // The packer's own page policy, not the declared maximum: a page is smaller
    // than `aqsa_ard * aqsa_irtifa` whenever the power-of-two policy is on, and
    // a budget computed from the maximum would buy fewer pages than it paid
    // for.
    //
    // And the *allocation* size, not what a compiled page ends up measuring.
    // `Lawha::ibni` crops a finished page to the rows its pack reached, so
    // `bayt_mabniya` below is a fraction of this — but the reservation is for
    // the runtime atlas, which opens whole pages and refuses a budget that will
    // not pay for one. Deriving this from the compiled pages would hand
    // `LawhaHayya::jadeeda` a budget it rejects, and the diagnostics would read
    // as an atlas that cannot be built rather than as a plan.
    let (ard, irtifa) = abaad_masmuha(khiyarat.rasf).map_err(|khata| KhataTarqee::RasfFashil {
        sabab: khata.injilizi,
    })?;
    let bayt_safha = u64::from(ard).saturating_mul(u64::from(irtifa)).max(1);

    let ashkal_mabniya = lawha.map_or(0, |mabniya| adad_u32(mabniya.adad_ashkal()));
    let bayt_mabniya = lawha.map_or(0, |mabniya| tul_u64(mabniya.bayt()));
    let safahat_mabniya = lawha.map_or(0, |mabniya| adad_u16(mabniya.adad_safahat()));
    let masaha = lawha.map_or(0, masahat_ashkal);

    // `checked_div` rather than the operator: division by the glyph count is
    // division by zero for an empty atlas, and the answer to "what does a glyph
    // cost when there are no glyphs" is that there is no answer.
    let mutawassit_bayt_shakl = masaha
        .checked_div(u64::from(ashkal_mabniya))
        .and_then(|qeema| u32::try_from(qeema).ok());

    let khanat_mahjuza = khiyarat.khanat_mahjuza.min(AQSA_KHANAT_IQAMA);
    let matlub =
        u64::from(khanat_mahjuza).saturating_mul(u64::from(mutawassit_bayt_shakl.unwrap_or(0)));
    // Rounded up to whole pages because a page is the unit the runtime
    // allocator actually buys, and floored at one for the reason above.
    let safahat_mahjuza = matlub.div_ceil(bayt_safha).max(1);
    let bayt_mahjuza = safahat_mahjuza.saturating_mul(bayt_safha);

    Ok(MizaniyatIqama {
        ashkal_mabniya,
        bayt_mabniya,
        safahat_mabniya,
        mutawassit_bayt_shakl,
        khanat_mahjuza,
        bayt_mahjuza,
        bayt_safha,
        siyasa: khiyarat.siyasat_namu,
        namat: khiyarat.namat.bayt(),
        ahjam_rubi,
    })
}

/// The area the compiled atlas's glyph images actually cover, in texels.
///
/// The occupied area rather than the page area, because the page area includes
/// the shelf allocator's gutters and whatever the last shelf did not use —
/// which is a property of the packing, not a cost per glyph. Dividing page
/// bytes by glyph count would make a patch that packed badly reserve more
/// runtime room than one that packed well, for exactly the same text.
fn masahat_ashkal(lawha: &Lawha) -> u64 {
    lawha
        .khareeta
        .murattaba()
        .iter()
        .map(|(_, mawdi)| u64::from(mawdi.ard).saturating_mul(u64::from(mawdi.irtifa)))
        .fold(0u64, u64::saturating_add)
}

// ---------------------------------------------------------------------------
// Small conversions
// ---------------------------------------------------------------------------

/// How many layouts carry a flag.
fn adad_bi_alam(takhtitat: &[TakhtitMabni], alam: u16) -> usize {
    takhtitat
        .iter()
        .filter(|takhtit| takhtit.alam & alam != 0)
        .count()
}

/// A count as the `u32` the record stores, saturating rather than wrapping.
fn adad_u32(adad: usize) -> u32 {
    u32::try_from(adad).unwrap_or(u32::MAX)
}

/// A count as the `u16` the record stores, saturating rather than wrapping.
fn adad_u16(adad: usize) -> u16 {
    u16::try_from(adad).unwrap_or(u16::MAX)
}

/// A short form of a string, for an error message.
///
/// By characters and not by bytes: truncating Arabic in the middle of a
/// codepoint would produce an error message that is itself malformed, which is
/// a poor way to report a malformed string.
fn mukhtasar(nass: &str) -> String {
    nass.chars().take(TUL_MUKHTASAR).collect()
}
