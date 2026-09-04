//! قياس أنريل — `qiyas_unreal`: correcting what Slate resolves wrongly for
//! right-to-left text.
//!
//! Slate measures and lays out correctly. `HarfBuzz` shapes correctly, ICU
//! segments correctly, and `tashghil` has already switched full shaping on by
//! the time anything here runs. What is left is a short list of *resolutions* —
//! decisions Slate or the game makes about direction, alignment and where a
//! line breaks — that are wrong for Arabic in ways that have nothing to do with
//! shaping. This module corrects those and nothing else.
//!
//! Three corrections, plus the in-world case, each installed only when its
//! target was found, each removable, each degrading alone.
//!
//! ## Flow direction
//!
//! `ETextFlowDirection` has three values: `Auto`, `LeftToRight`,
//! `RightToLeft`. `Auto` resolves from the current culture and is correct once
//! `alam` has made Arabic active. The failure is that a great many games never
//! use `Auto`. They set `LeftToRight` in a widget blueprint because that was
//! the value in the dropdown when the widget was authored, or they read a flow
//! direction out of a settings object written before Arabic existed in the
//! build. The result is Arabic that is shaped and joined correctly and then
//! laid out from the wrong edge, which reads as a sentence written backwards.
//!
//! The correction intercepts the point where the wrong value is delivered and
//! substitutes the resolved one. It does not touch `Auto`, because `Auto` was
//! already going to be right.
//!
//! ## Alignment: "left" is not "leading"
//!
//! This is the single most common right-to-left bug in any user-interface
//! toolkit, and it is a naming failure rather than a text failure.
//!
//! A designer aligning a label writes "left". What the designer *means*, almost
//! always, is "the edge the reader's eye starts at" — the **leading** edge. In
//! a left-to-right language those two are the same edge, so nothing ever forces
//! the distinction to be made, and the toolkit's enumeration ends up spelling
//! the physical edge rather than the logical one. Unreal's `ETextJustify` is
//! exactly this: `Left`, `Center`, `Right`. There is no `Leading`, no
//! `Trailing`, and therefore no way for a widget to record which of the two it
//! meant.
//!
//! In Arabic the leading edge is the right one. A paragraph resolved to `Left`
//! in a right-to-left context is pinned to the trailing edge: the text starts
//! flush against the edge the reader finishes at, ragged on the side the reader
//! starts from. Every line of a menu is then one word out of place, and it is
//! subtly wrong in a way that is hard to name and impossible to unsee.
//!
//! CSS solved this by adding `start` and `end` beside `left` and `right`, and
//! every toolkit that did not eventually grew a mirroring pass. This module is
//! that mirroring pass for Slate: under a right-to-left base direction, `Left`
//! resolves to the trailing edge of the box — which is `Right` in Slate's
//! physical vocabulary — and `Right` resolves to `Left`. `Center` is unchanged,
//! because centre is the one alignment that has no leading edge to get wrong.
//!
//! Because the mirror cannot distinguish a designer who meant "leading" from
//! one who meant "physically left", [`SiyasatMuhadhaha`] exists: the default
//! mirrors both, and a game that genuinely needs a physically-left column can
//! be given [`SiyasatMuhadhaha::BidayaFaqat`], which corrects only `Left`.
//!
//! ## Wrapping
//!
//! Slate breaks lines against its own measurement. When that measurement
//! disagrees with the width the text actually occupies once shaped, the break
//! lands in the wrong place: too late, and the last word is clipped by the
//! widget's own bounds; too early, and a line ends short for no visible reason.
//!
//! This is where `jisr` earns its place in an adapter that has no atlas.
//! Taarib measures the same run through its own shaping pipeline — the same
//! HarfBuzz-equivalent path, the same font chain, the same feature set the
//! patch recorded — and compares. The comparison is deliberately asymmetric:
//! the correction only ever reports a run as **wider** than Slate believed,
//! never narrower.
//!
//! The asymmetry is the whole design. Reporting a run as narrower than it is
//! makes Slate fit more onto a line than the line can hold, and the overflow
//! is clipped by the widget: a player loses the end of a sentence. Reporting it
//! as wider makes Slate break one word early, and a player sees a line that
//! could have held one more word. One of those is a lost sentence and the other
//! is slightly loose ragging, and they are not comparable failures.
//!
//! ## In-world text
//!
//! `UTextRenderComponent` and Slate-in-3D need no separate correction logic,
//! and that is a property of Unreal rather than a convenience.
//!
//! `UTextRenderComponent` builds its glyph runs from the same font measure
//! service the menus use, so the wrapping correction reaches it through the
//! measure hook without anything else being installed. A Slate widget rendered
//! into the world through `UWidgetComponent` is a genuine `SWidget` tree with a
//! genuine `FTextLayout`, so the direction and alignment corrections reach it
//! through the same layout entry points: the widget does not know or care that
//! its render target is a material on a mesh.
//!
//! What remains is the small number of builds where `UTextRenderComponent` was
//! given its own measurement entry point at a distinct address. That is what
//! [`AhdafQiyas::mujassam`] is for — the same detour body, installed a second
//! time at a second address — and it is optional for exactly that reason.
//!
//! ## Hooking is a contract
//!
//! Each correction records the bytes it replaced before it replaces them, and
//! [`TasheehQiyas::azil`] restores every one and reports any target whose bytes
//! did not come back identical. A module that unloads leaves the process
//! byte-for-byte as it found it; that is `taarib-haqn`'s rule and this module
//! does not get an exemption from it because its hooks are small.
//!
//! A hook that cannot be installed is a [`KhataUnreal::KhatfFashil`] naming the
//! target, logged as a warning, and the remaining hooks are installed anyway. A
//! game with correct direction and correct alignment and Slate's own wrapping
//! is a better game than one where all three were abandoned because a wrapping
//! entry point could not be found.

use core::ffi::c_void;
use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use retour::RawDetour;
use taarib_jisr::anwa::{
    TAARIB_KHIYAR_SATR_WAHID, TaaribKhiyarat, TaaribQiyasNass, TaaribSilsila, TaaribTalab,
};
use taarib_jisr::awamir::taarib_qiyas;
use taarib_jisr::khata_c::TAARIB_NAJAH;

use crate::khata::KhataUnreal;
use crate::slate::{DiqqatMuttajih, Muttajih2D, Muttajih2F, Unwan, WaslSlate};

// ---------------------------------------------------------------------------
// The vocabulary Slate uses, as Slate spells it
// ---------------------------------------------------------------------------

/// `ETextFlowDirection::Auto` — resolve from the active culture.
pub const SAYALAN_TILQAI: u8 = 0;
/// `ETextFlowDirection::LeftToRight`.
pub const SAYALAN_YASAR: u8 = 1;
/// `ETextFlowDirection::RightToLeft`.
pub const SAYALAN_YAMEEN: u8 = 2;

/// `ETextJustify::Left` — the physical left edge, which games use to mean the
/// leading edge. See this module's header.
pub const MUHADHAHA_YASAR: u8 = 0;
/// `ETextJustify::Center`.
pub const MUHADHAHA_WASAT: u8 = 1;
/// `ETextJustify::Right` — the physical right edge.
pub const MUHADHAHA_YAMEEN: u8 = 2;

/// How much of a target's prologue is recorded before it is replaced.
///
/// Sixteen bytes covers every patch either supported architecture needs: five
/// for a relative jump on `x86_64`, fourteen for an absolute indirect jump, and
/// sixteen for an ARM64 branch island. Both linkers pad function entry points
/// to at least sixteen bytes, so reading this many from a function address
/// stays inside the same executable section.
pub const TUL_LAQTA: usize = 16;

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// How far the alignment mirror goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SiyasatMuhadhaha {
    /// Mirror both edges: `Left` becomes `Right` and `Right` becomes `Left`
    /// under a right-to-left base direction.
    ///
    /// The default, and what every mature toolkit's `start`/`end` semantics
    /// amount to. A right-aligned column in a left-to-right layout is a column
    /// pinned to the trailing edge, and in a mirrored layout the trailing edge
    /// is the left one.
    #[default]
    Miraya,
    /// Correct only `Left`.
    ///
    /// For the game whose right alignment is genuinely physical — a
    /// heads-up-display element pinned to a screen corner rather than to a
    /// column of text. Rare, and it has to be asked for, because assuming it
    /// would leave the common case broken.
    BidayaFaqat,
}

/// The decisions the corrections read, fixed once at install.
///
/// Every field is a scalar so the whole structure is [`Send`] and [`Sync`]
/// without an `unsafe impl`, which matters because the detours are plain
/// `extern "C"` functions with no captured environment: they reach this through
/// a `OnceLock` and there is no other channel they could use.
#[derive(Debug, Clone, Copy)]
pub struct HalatTasheeh {
    /// Whether the patch's base direction is right to left. Everything else in
    /// this structure is inert when it is false.
    pub asas_yameen: bool,
    /// How far the alignment mirror goes.
    pub siyasat_muhadhaha: SiyasatMuhadhaha,
    /// How much wider than Slate's own measurement Taarib's has to be before
    /// the wrapping correction reports the difference, in Slate units.
    ///
    /// Not zero. Two shaping pipelines over the same font agree to within a
    /// fraction of a pixel, and a correction that fired on that fraction would
    /// fire on every run in the game, replacing a measurement with an identical
    /// one at the cost of a shaping call per string per frame.
    pub tafawut_laff: f32,
    /// A live `jisr` context, held as an exposed address so this structure stays
    /// a plain scalar record. Zero disables the wrapping correction's
    /// comparison, which then returns Slate's own answer untouched.
    pub siyaq_jisr: usize,
    /// The `jisr` font chain to measure against, as an exposed address.
    pub silsila_jisr: usize,
    /// The direction discriminant handed to `jisr`, from
    /// [`taarib_jisr::anwa::ittijah_min_raqm`]'s vocabulary.
    pub ittijah_jisr: u32,
    /// The language discriminant handed to `jisr`.
    pub lugha_jisr: u32,
    /// The diacritic policy handed to `jisr`, so a measurement made here agrees
    /// with what the patch compiler measured offline.
    pub tashkeel_jisr: u32,
    /// The digit policy handed to `jisr`, for the same reason.
    pub arqam_jisr: u32,
}

impl Default for HalatTasheeh {
    fn default() -> Self {
        Self {
            asas_yameen: true,
            siyasat_muhadhaha: SiyasatMuhadhaha::Miraya,
            // Half a Slate unit: below the width of any glyph, above the
            // disagreement two correct shapers have about one.
            tafawut_laff: 0.5,
            siyaq_jisr: 0,
            silsila_jisr: 0,
            ittijah_jisr: 1,
            lugha_jisr: 1,
            tashkeel_jisr: 0,
            arqam_jisr: 0,
        }
    }
}

/// The addresses the corrections are installed at.
///
/// Supplied rather than discovered. Which function carries Slate's flow
/// direction, which carries its justification, and which carries the
/// measurement wrapping consumes are all questions whose answers vary with the
/// engine version, and a number invented in this file would be a number nobody
/// could verify — the same objection that keeps a signature database out of
/// [`crate::slate`]. Whatever resolves them fills this in; every field is
/// optional and a `None` is one correction that does not run.
#[derive(Debug, Clone, Copy, Default)]
pub struct AhdafQiyas {
    /// The flow-direction setter on the text layout.
    pub ittijah: Option<Unwan>,
    /// The justification setter on the text layout.
    pub muhadhaha: Option<Unwan>,
    /// The measurement the line breaker consumes.
    pub laff: Option<Unwan>,
    /// A second measurement entry point, for the builds where in-world text
    /// does not share the menus'. See this module's header.
    pub mujassam: Option<Unwan>,
}

/// Which correction a hook is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HadafKhatf {
    /// Flow direction.
    Ittijah,
    /// Alignment resolution.
    Muhadhaha,
    /// Wrapping, through the measurement the line breaker consumes.
    Laff,
    /// In-world text's own measurement, where it has one.
    Mujassam,
}

impl HadafKhatf {
    /// The name that appears in [`KhataUnreal::KhatfFashil`] and in the log.
    ///
    /// A `&'static str` because the error variant takes one, and because a hook
    /// failure that named a heap string would be a hook failure whose message
    /// depended on an allocation succeeding in a process already in trouble.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Ittijah => "FTextLayout::SetTextFlowDirection",
            Self::Muhadhaha => "FTextLayout::SetJustification",
            Self::Laff => "FSlateFontMeasure::Measure",
            Self::Mujassam => "UTextRenderComponent's own text measure",
        }
    }

    /// What a missing address means for this particular correction.
    ///
    /// Not one message for all four, because they do not mean the same thing.
    /// A missing direction or alignment target is a correction the game does
    /// not get. A missing in-world target is the *ordinary* case — in-world
    /// text shares the menus' measurement on almost every build — and a warning
    /// that read like a failure there would train the reader to ignore the
    /// three that are.
    #[must_use]
    pub const fn tafsil_ghiyab(self) -> &'static str {
        match self {
            Self::Mujassam => {
                "no separate in-world measurement entry point was supplied, which is the \
                 usual case: UTextRenderComponent and Slate-in-3D go through the shared \
                 measure hook and are already corrected by it"
            }
            _ => {
                "no address was supplied for this engine version, so this one correction \
                 does not run and the others do"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The corrections, as pure decisions
// ---------------------------------------------------------------------------

/// Resolves a flow direction for a right-to-left base direction.
///
/// `Auto` is returned unchanged: it was already going to resolve correctly from
/// the culture `alam` activated, and overriding it would replace a correct
/// dynamic answer with a fixed one that stops being correct the moment a player
/// switches back to English.
#[must_use]
pub const fn sahih_ittijah(muallan: u8, asas_yameen: bool) -> u8 {
    if !asas_yameen || muallan == SAYALAN_TILQAI {
        return muallan;
    }
    SAYALAN_YAMEEN
}

/// Resolves an alignment for a right-to-left base direction.
///
/// The mirror described in this module's header: `Left` means leading and
/// leading is the right edge, so `Left` becomes `Right`; under
/// [`SiyasatMuhadhaha::Miraya`] the converse holds too. `Center` is returned
/// unchanged, and so is any value outside the enumeration — a number Slate
/// never produced is a number this module has no business reinterpreting.
#[must_use]
pub const fn sahih_muhadhaha(
    muallan: u8,
    asas_yameen: bool,
    siyasa: SiyasatMuhadhaha,
) -> u8 {
    if !asas_yameen {
        return muallan;
    }
    match (muallan, siyasa) {
        (MUHADHAHA_YASAR, _) => MUHADHAHA_YAMEEN,
        (MUHADHAHA_YAMEEN, SiyasatMuhadhaha::Miraya) => MUHADHAHA_YASAR,
        _ => muallan,
    }
}

/// Reconciles Slate's measured width with Taarib's.
///
/// Returns the width the line breaker should use. Only ever widens, for the
/// reason set out in this module's header: a run reported narrower than it is
/// gets clipped, and a run reported wider breaks one word early.
///
/// A non-finite measurement from either side is discarded in favour of the
/// other, and two non-finite measurements return Slate's, because a NaN width
/// propagated into a line breaker is a layout loop that never terminates.
#[must_use]
pub fn sahih_ard(slate: f32, taarib: f32, tafawut: f32) -> f32 {
    if !slate.is_finite() {
        return if taarib.is_finite() { taarib } else { 0.0 };
    }
    if !taarib.is_finite() {
        return slate;
    }
    if taarib > slate + tafawut.max(0.0) { taarib } else { slate }
}

// ---------------------------------------------------------------------------
// Shared state the detours read
// ---------------------------------------------------------------------------

/// The policy, published once at install and read by every detour afterwards.
static HALA: OnceLock<HalatTasheeh> = OnceLock::new();

/// The trampoline back to the original flow-direction setter.
static ASL_ITTIJAH: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to the original justification setter.
static ASL_MUHADHAHA: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to the original single-precision measurement.
static ASL_LAFF_MUFRAD: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to the original double-precision measurement.
static ASL_LAFF_MUDAAF: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to in-world text's own single-precision measurement.
static ASL_MUJASSAM_MUFRAD: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to in-world text's own double-precision measurement.
static ASL_MUJASSAM_MUDAAF: AtomicUsize = AtomicUsize::new(0);

/// The policy, or a default that changes nothing, for a detour that somehow ran
/// before the policy was published.
///
/// Cannot happen in the installation order this module enforces — the policy is
/// published before the first hook is enabled — and is written as a fallback
/// rather than an assertion because the alternative inside an `extern "C"`
/// detour is a panic crossing an ABI boundary, which aborts a player's game.
fn hala() -> HalatTasheeh {
    *HALA.get().unwrap_or(&HalatTasheeh {
        asas_yameen: false,
        siyasat_muhadhaha: SiyasatMuhadhaha::Miraya,
        tafawut_laff: f32::INFINITY,
        siyaq_jisr: 0,
        silsila_jisr: 0,
        ittijah_jisr: 0,
        lugha_jisr: 0,
        tashkeel_jisr: 0,
        arqam_jisr: 0,
    })
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

/// A member function taking `this` and one `uint8` enumerator.
///
/// Both Unreal enumerations corrected here are `: uint8`, and both setters take
/// the enumerator by value. `this` is the leading parameter under both calling
/// conventions, which is what an `extern "C"` function pointer with a leading
/// raw pointer compiles to; [`crate::slate::dalla_min_jadwal`] documents why
/// that is exact rather than approximate.
pub type DallaTaayeen = unsafe extern "C" fn(hadha: *mut c_void, qeema: u8);

/// A measurement taking `this`, a UTF-16 range, a font and a scale, returning
/// `FVector2f`.
///
/// The `int32` bounds are Unreal's own `int32`, which is `i32` on every target
/// the engine supports.
pub type DallaQiyasMufrad = unsafe extern "C" fn(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2F;

/// The same measurement on the engine versions where it returns `FVector2d`.
///
/// Two declarations rather than one generic, because the return width is a
/// calling-convention difference and not a type parameter: on Windows x64 the
/// single-precision form comes back in `RAX` and the double-precision form
/// through a hidden pointer. [`crate::slate::Muttajih2F`] sets this out.
pub type DallaQiyasMudaaf = unsafe extern "C" fn(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2D;

// ---------------------------------------------------------------------------
// The detours
// ---------------------------------------------------------------------------

/// Substitutes the resolved flow direction, then calls the original setter.
///
/// The original is still called, with the corrected value, rather than being
/// skipped. Slate's setter invalidates layout caches and marks the widget
/// dirty; a correction that wrote the field directly and returned would leave
/// the widget displaying a stale line layout until something else happened to
/// invalidate it.
unsafe extern "C" fn khatf_ittijah(hadha: *mut c_void, qeema: u8) {
    let musahhah = sahih_ittijah(qeema, hala().asas_yameen);
    let asl = ASL_ITTIJAH.load(Ordering::Acquire);
    if asl == 0 {
        return;
    }
    // SAFETY: `asl` is the trampoline `retour` produced for this target, which
    // has the target's own signature — a member function taking `this` and one
    // `uint8`. It is published by `TasheehQiyas::rakkib` after the detour was
    // constructed and before it was enabled, so a non-zero value here is a live
    // trampoline for the whole time this detour can be entered.
    let asl: DallaTaayeen = unsafe { core::mem::transmute(asl) };
    // SAFETY: `hadha` is the `this` Slate passed to the function this detour
    // replaced; it is a live object of that class for the duration of the call,
    // and the trampoline expects exactly the arguments being forwarded.
    unsafe { asl(hadha, musahhah) };
}

/// Substitutes the mirrored alignment, then calls the original setter.
unsafe extern "C" fn khatf_muhadhaha(hadha: *mut c_void, qeema: u8) {
    let hala = hala();
    let musahhah = sahih_muhadhaha(qeema, hala.asas_yameen, hala.siyasat_muhadhaha);
    let asl = ASL_MUHADHAHA.load(Ordering::Acquire);
    if asl == 0 {
        return;
    }
    // SAFETY: as `khatf_ittijah` — the trampoline carries the target's own
    // signature and is published before the detour is enabled.
    let asl: DallaTaayeen = unsafe { core::mem::transmute(asl) };
    // SAFETY: as `khatf_ittijah`.
    unsafe { asl(hadha, musahhah) };
}

/// The single-precision measurement detour for menu text.
unsafe extern "C" fn khatf_laff_mufrad(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2F {
    // SAFETY: every argument is forwarded unchanged to the trampoline this
    // detour replaced, and `nass` is read only for the `[bidaya, nihaya)` range
    // Slate itself is about to read. The contract is restated on
    // `qis_wa_sahhih_mufrad`.
    unsafe {
        qis_wa_sahhih_mufrad(&ASL_LAFF_MUFRAD, hadha, nass, bidaya, nihaya, khatt, miqyas)
    }
}

/// The double-precision measurement detour for menu text.
unsafe extern "C" fn khatf_laff_mudaaf(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2D {
    // SAFETY: as `khatf_laff_mufrad`.
    unsafe {
        qis_wa_sahhih_mudaaf(&ASL_LAFF_MUDAAF, hadha, nass, bidaya, nihaya, khatt, miqyas)
    }
}

/// The single-precision measurement detour for in-world text.
unsafe extern "C" fn khatf_mujassam_mufrad(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2F {
    // SAFETY: as `khatf_laff_mufrad`. The body is identical because the
    // correction is identical; only the trampoline slot differs, because the
    // two entry points are two addresses.
    unsafe {
        qis_wa_sahhih_mufrad(&ASL_MUJASSAM_MUFRAD, hadha, nass, bidaya, nihaya, khatt, miqyas)
    }
}

/// The double-precision measurement detour for in-world text.
unsafe extern "C" fn khatf_mujassam_mudaaf(
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2D {
    // SAFETY: as `khatf_laff_mudaaf`.
    unsafe {
        qis_wa_sahhih_mudaaf(&ASL_MUJASSAM_MUDAAF, hadha, nass, bidaya, nihaya, khatt, miqyas)
    }
}

/// Calls the original single-precision measurement and widens it where Taarib's
/// own measurement says the run is wider.
///
/// # Safety
///
/// `asl` must hold a trampoline with [`DallaQiyasMufrad`]'s signature; `hadha`
/// must be the `this` the engine passed; `nass` must be readable for at least
/// `nihaya` UTF-16 code units, which is the range Slate is itself about to
/// read. Every one of those is guaranteed by this function only ever being
/// called from a detour installed on a function with that signature.
unsafe fn qis_wa_sahhih_mufrad(
    asl: &AtomicUsize,
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2F {
    let unwan = asl.load(Ordering::Acquire);
    if unwan == 0 {
        return Muttajih2F::default();
    }
    // SAFETY: delegated to this function's own contract.
    let dalla: DallaQiyasMufrad = unsafe { core::mem::transmute(unwan) };
    // SAFETY: every argument is the one the engine supplied, forwarded
    // unchanged to the code that was going to receive it.
    let asli = unsafe { dalla(hadha, nass, bidaya, nihaya, khatt, miqyas) };

    // SAFETY: `nass` is readable for the range the engine is about to read, per
    // this function's contract.
    let Some(nass) = (unsafe { nass_min_utf16(nass, bidaya, nihaya) }) else {
        return asli;
    };
    let hala = hala();
    let Some(taarib) = qis_bi_jisr(&hala, &nass, miqyas) else {
        return asli;
    };
    Muttajih2F { s: sahih_ard(asli.s, taarib, hala.tafawut_laff), a: asli.a }
}

/// The double-precision counterpart of [`qis_wa_sahhih_mufrad`].
///
/// The comparison happens in `f32` because that is the width `jisr` reports and
/// the width the patch was compiled against; only the value handed back to
/// Slate is widened again. A comparison performed in `f64` against an `f32`
/// measurement would be comparing a number to a rounded copy of itself.
///
/// # Safety
///
/// As [`qis_wa_sahhih_mufrad`], with [`DallaQiyasMudaaf`]'s signature.
unsafe fn qis_wa_sahhih_mudaaf(
    asl: &AtomicUsize,
    hadha: *mut c_void,
    nass: *const u16,
    bidaya: i32,
    nihaya: i32,
    khatt: *const c_void,
    miqyas: f32,
) -> Muttajih2D {
    let unwan = asl.load(Ordering::Acquire);
    if unwan == 0 {
        return Muttajih2D::default();
    }
    // SAFETY: delegated to this function's own contract.
    let dalla: DallaQiyasMudaaf = unsafe { core::mem::transmute(unwan) };
    // SAFETY: as `qis_wa_sahhih_mufrad`.
    let asli = unsafe { dalla(hadha, nass, bidaya, nihaya, khatt, miqyas) };

    // SAFETY: as `qis_wa_sahhih_mufrad`.
    let Some(nass) = (unsafe { nass_min_utf16(nass, bidaya, nihaya) }) else {
        return asli;
    };
    let hala = hala();
    let Some(taarib) = qis_bi_jisr(&hala, &nass, miqyas) else {
        return asli;
    };
    let mufrad = asli.mufrad();
    Muttajih2D {
        s: f64::from(sahih_ard(mufrad.s, taarib, hala.tafawut_laff)),
        a: asli.a,
    }
}

/// Copies the measured range out of Slate's UTF-16 buffer as UTF-8.
///
/// Returns [`None`] for a range that is empty or that does not describe a
/// forward span, which is not an error: Slate measures empty runs routinely and
/// the correct answer for one is Slate's own.
///
/// An unpaired surrogate becomes U+FFFD rather than aborting the conversion.
/// The alternative — refusing the run — would hand the wrapping decision back
/// to Slate for exactly the strings most likely to be mismeasured, and the
/// replacement character's width is a measurement error confined to one glyph
/// in a run that was already malformed.
///
/// # Safety
///
/// `nass` must be readable for at least `nihaya` UTF-16 code units.
unsafe fn nass_min_utf16(nass: *const u16, bidaya: i32, nihaya: i32) -> Option<String> {
    if nass.is_null() || bidaya < 0 || nihaya <= bidaya {
        return None;
    }
    let bidaya = usize::try_from(bidaya).ok()?;
    let nihaya = usize::try_from(nihaya).ok()?;
    let tul = nihaya.checked_sub(bidaya)?;
    if tul == 0 || tul > AQSA_WAHDAT {
        return None;
    }
    // SAFETY: the caller guarantees `nass` is readable for `nihaya` code units,
    // and `bidaya + tul == nihaya`, so the slice stays inside that guarantee.
    let wahdat = unsafe { core::slice::from_raw_parts(nass.add(bidaya), tul) };
    Some(
        char::decode_utf16(wahdat.iter().copied())
            .map(|harf| harf.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect(),
    )
}

/// The longest run this correction will copy and re-measure.
///
/// A ceiling rather than a limit on Slate: a measurement call whose declared
/// range is longer than any real line is a range this module refuses to
/// allocate for, and Slate's own answer is returned instead. Sixteen thousand
/// code units is far past the longest line of dialogue any game has and far
/// short of a number that matters to a frame budget.
const AQSA_WAHDAT: usize = 16_384;

/// Measures a run through `jisr` and returns its width.
///
/// Returns [`None`] when no context was supplied, when the request is refused,
/// or when the reported width is not finite — each of which leaves Slate's own
/// measurement in place, which is the correct fallback and not a silent one:
/// the correction's whole contract is that it improves a measurement or leaves
/// it alone.
fn qis_bi_jisr(hala: &HalatTasheeh, nass: &str, miqyas: f32) -> Option<f32> {
    if hala.siyaq_jisr == 0 || hala.silsila_jisr == 0 || !miqyas.is_finite() || miqyas <= 0.0 {
        return None;
    }
    let khiyarat = TaaribKhiyarat {
        sifat: core::ptr::null(),
        adad_sifat: 0,
        ittijah: hala.ittijah_jisr,
        lugha: hala.lugha_jisr,
        // No justification and no overflow policy: this is a measurement of
        // what the run occupies, not a layout decision. Asking `jisr` to
        // justify or shrink here would be Taarib answering a question Slate has
        // not asked yet and is about to answer itself.
        dabt: 0,
        muhadhaha: 0,
        tashkeel: hala.tashkeel_jisr,
        arqam: hala.arqam_jisr,
        tajawuz: 0,
        hajm_adna: 0.0,
        irtifa_satr: 0.0,
        tabaud_ahruf: 0.0,
        tabaud_kalimat: 0.0,
        // Single line, unconditionally: the caller is asking how wide this run
        // is, and a measurement that wrapped would answer a different question
        // and answer it with the width of the box rather than of the text.
        alam: TAARIB_KHIYAR_SATR_WAHID,
    };
    let silsila: TaaribSilsila = core::ptr::with_exposed_provenance_mut(hala.silsila_jisr);
    let talab = TaaribTalab {
        nass: nass.as_ptr(),
        tul_nass: nass.len(),
        nitaqat: core::ptr::null(),
        adad_nitaqat: 0,
        silsila,
        hajm: miqyas,
        ard_mutah: 0.0,
        irtifa_mutah: 0.0,
        khiyarat,
    };
    let mut khuruj = TaaribQiyasNass::default();
    let siyaq = core::ptr::with_exposed_provenance_mut(hala.siyaq_jisr);
    // SAFETY: `talab` and `khuruj` are live locals of exactly the declared
    // types, correctly aligned, and valid for the duration of the call, which
    // is all `taarib_qiyas` requires of them; `talab.nass` points at `nass`,
    // which outlives the call, and its two array pointers are null with a count
    // of zero. `siyaq` and `silsila` were published by whoever installed this
    // correction, which owns the `jisr` context for the life of the process and
    // is the only party that can destroy it.
    let ramz = unsafe { taarib_qiyas(siyaq, &raw const talab, &raw mut khuruj) };
    if ramz != TAARIB_NAJAH || !khuruj.ard.is_finite() {
        return None;
    }
    Some(khuruj.ard)
}

// ---------------------------------------------------------------------------
// One installed hook
// ---------------------------------------------------------------------------

/// A detour, the bytes it replaced, and the target it replaced them at.
///
/// The recorded bytes are what makes uninstalling checkable rather than merely
/// attempted. `retour` restores the prologue itself; this records the prologue
/// independently so that [`TasheehQiyas::azil`] can say whether the restoration
/// actually produced the original bytes, instead of reporting success because
/// no error was returned.
pub struct KhatfQiyas {
    hadaf: HadafKhatf,
    unwan: Unwan,
    laqta: [u8; TUL_LAQTA],
    khatf: RawDetour,
    mufaal: bool,
}

impl fmt::Debug for KhatfQiyas {
    fn fmt(&self, muharrir: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unwan = format!("{:#x}", self.unwan.raqm());
        let laqta = sittasi(&self.laqta);
        muharrir
            .debug_struct("KhatfQiyas")
            .field("hadaf", &self.hadaf.ism())
            .field("unwan", &unwan)
            .field("laqta", &laqta)
            .field("mufaal", &self.mufaal)
            // `khatf` is a `RawDetour`, which has no `Debug`.
            .finish_non_exhaustive()
    }
}

impl KhatfQiyas {
    /// Which correction this is.
    #[must_use]
    pub const fn hadaf(&self) -> HadafKhatf {
        self.hadaf
    }

    /// Where it is installed.
    #[must_use]
    pub const fn unwan(&self) -> Unwan {
        self.unwan
    }

    /// The bytes that were at the target before it was patched.
    #[must_use]
    pub const fn laqta(&self) -> &[u8; TUL_LAQTA] {
        &self.laqta
    }

    /// Whether the detour is currently active.
    #[must_use]
    pub const fn mufaal(&self) -> bool {
        self.mufaal
    }

    /// Installs one detour and publishes its trampoline.
    ///
    /// The order matters and is not incidental: the prologue is recorded, the
    /// detour is constructed, the trampoline is published into its atomic, and
    /// only then is the detour enabled. Enabling before publishing would open a
    /// window in which the game calls the detour, reads a zero trampoline, and
    /// gets the do-nothing branch — for a setter that is a lost call, and for a
    /// measurement it is a zero width, which is a widget that collapses for one
    /// frame.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhatfFashil`] naming the target, when `retour` refuses the
    /// prologue — an instruction that cannot be relocated, a target too close to
    /// the end of its section, or a page that could not be made writable.
    ///
    /// # Safety
    ///
    /// `unwan` must be the address of a function whose signature is exactly the
    /// one `khatf` declares, in a module that stays loaded for as long as this
    /// value lives. The guarantee comes from the caller resolving the address
    /// against the engine version rather than guessing it, and from
    /// [`TasheehQiyas::azil`] running before this library unloads.
    pub unsafe fn rakkib(
        hadaf: HadafKhatf,
        unwan: Unwan,
        khatf: *const c_void,
        asl: &'static AtomicUsize,
    ) -> Result<Self, KhataUnreal> {
        // SAFETY: `unwan` is a function address in mapped executable memory, per
        // this function's contract. Both supported linkers align and pad
        // function entry points to at least `TUL_LAQTA` bytes, so the read stays
        // inside the same executable section as the entry point itself.
        let laqta = unsafe { iqra_laqta(unwan) };

        // SAFETY: `unwan` names a function with `khatf`'s signature, per this
        // function's contract, and `khatf` is one of this module's own detours,
        // whose address is valid for the life of the loaded library. `retour`
        // decodes the prologue rather than assuming it, so an unrelocatable
        // prologue is the error below and not a corrupted instruction boundary.
        let detour = unsafe { RawDetour::new(unwan.muashir().cast(), khatf.cast()) }
            .map_err(|khata| KhataUnreal::KhatfFashil {
                hadaf: hadaf.ism(),
                tafsil: khata.to_string(),
            })?;

        asl.store(core::ptr::from_ref(detour.trampoline()).expose_provenance(), Ordering::Release);

        // SAFETY: the detour was constructed for this exact target and its
        // trampoline is published, so a call arriving the instant this returns
        // finds a complete correction. Enabling writes only within the
        // prologue `retour` measured.
        unsafe { detour.enable() }.map_err(|khata| {
            asl.store(0, Ordering::Release);
            KhataUnreal::KhatfFashil { hadaf: hadaf.ism(), tafsil: khata.to_string() }
        })?;

        Ok(Self { hadaf, unwan, laqta, khatf: detour, mufaal: true })
    }

    /// Removes the detour and checks that the target's bytes came back.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::KhatfFashil`] naming the target when `retour` could not
    /// disable the detour, and — the check that makes the contract real — when
    /// disabling reported success but the prologue is not the one that was
    /// recorded. The second case is reported with both byte strings, because a
    /// process that is not byte-identical after an uninstall is a process
    /// somebody has to be told about with evidence.
    pub fn azil(&mut self, asl: &'static AtomicUsize) -> Result<(), KhataUnreal> {
        if !self.mufaal {
            return Ok(());
        }
        // SAFETY: `self.khatf` was constructed by `rakkib` for `self.unwan`,
        // which is still mapped because this value's contract requires it, and
        // disabling restores the prologue `retour` itself saved.
        let natija = unsafe { self.khatf.disable() };
        self.mufaal = false;
        asl.store(0, Ordering::Release);
        natija.map_err(|khata| KhataUnreal::KhatfFashil {
            hadaf: self.hadaf.ism(),
            tafsil: khata.to_string(),
        })?;

        // SAFETY: as the read in `rakkib` — the same address, the same section,
        // the same length.
        let baad = unsafe { iqra_laqta(self.unwan) };
        if baad == self.laqta {
            return Ok(());
        }
        Err(KhataUnreal::KhatfFashil {
            hadaf: self.hadaf.ism(),
            tafsil: format!(
                "the detour was removed but the prologue did not come back: {} was recorded \
                 and {} is present, so the process is not byte-identical to the one this \
                 module entered",
                sittasi(&self.laqta),
                sittasi(&baad),
            ),
        })
    }
}

/// Reads a target's prologue.
///
/// # Safety
///
/// `unwan` must be the address of a function in mapped, readable executable
/// memory with at least [`TUL_LAQTA`] bytes available from it.
const unsafe fn iqra_laqta(unwan: Unwan) -> [u8; TUL_LAQTA] {
    let mut laqta = [0u8; TUL_LAQTA];
    // SAFETY: the caller guarantees `TUL_LAQTA` readable bytes at `unwan`, and
    // `laqta` is a live local of exactly that length. The two cannot overlap:
    // one is a stack local of this frame and the other is a foreign module's
    // executable section.
    unsafe {
        core::ptr::copy_nonoverlapping(
            unwan.muashir().cast::<u8>(),
            laqta.as_mut_ptr(),
            TUL_LAQTA,
        );
    }
    laqta
}

/// Bytes as lower-case hexadecimal, for the one diagnostic that needs them.
fn sittasi(bayt: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut nass = String::with_capacity(bayt.len() * 2);
    for wahda in bayt {
        let _ = write!(nass, "{wahda:02x}");
    }
    nass
}

// ---------------------------------------------------------------------------
// The correction set
// ---------------------------------------------------------------------------

/// Every correction that could be installed, and every one that could not.
///
/// Constructing one never fails. A target that was not supplied is skipped, a
/// hook that would not install is recorded in [`TasheehQiyas::tanbeehat`], and
/// the rest go in — which is the degradation rule this adapter is built on,
/// written into a type rather than left to each caller to remember.
#[derive(Debug)]
pub struct TasheehQiyas {
    khutuf: Vec<(KhatfQiyas, &'static AtomicUsize)>,
    tanbeehat: Vec<KhataUnreal>,
    diqqa: DiqqatMuttajih,
}

impl TasheehQiyas {
    /// Installs every correction whose target was found.
    ///
    /// The policy is published before the first hook is enabled, so no detour
    /// can run against an unpublished policy. Publishing is idempotent: a second
    /// installation in one process keeps the first policy and says so through a
    /// warning, because two correction sets with different base directions
    /// would be two answers to one question and the detours can only read one.
    pub fn rakkib(wasl: &WaslSlate, ahdaf: &AhdafQiyas, hala: HalatTasheeh) -> Self {
        let diqqa = wasl.diqqa();
        let mut tanbeehat = Vec::new();
        if HALA.set(hala).is_err() {
            tanbeehat.push(KhataUnreal::KhatfFashil {
                hadaf: "the correction policy",
                tafsil: "a correction set was already installed in this process, so its \
                         policy was kept; the second set's policy was discarded rather \
                         than letting two answers to one question reach the detours"
                    .to_owned(),
            });
        }

        let mut khutuf = Vec::new();
        let mut daa = |hadaf: HadafKhatf,
                       unwan: Option<Unwan>,
                       khatf: *const c_void,
                       asl: &'static AtomicUsize| {
            let Some(unwan) = unwan else {
                tanbeehat.push(KhataUnreal::KhatfFashil {
                    hadaf: hadaf.ism(),
                    tafsil: hadaf.tafsil_ghiyab().to_owned(),
                });
                return;
            };
            // SAFETY: `unwan` was resolved for this engine version by the caller
            // and names a function with the signature of the detour paired with
            // it here; the pairing is fixed in this function and cannot be
            // supplied from outside. The module it lives in stays loaded because
            // `azil` runs before this library unloads.
            match unsafe { KhatfQiyas::rakkib(hadaf, unwan, khatf, asl) } {
                Ok(khatf) => khutuf.push((khatf, asl)),
                Err(khata) => tanbeehat.push(khata),
            }
        };

        // Each detour is bound to its declared signature before its address is
        // taken, so the pairing of target and detour is checked by the compiler
        // here rather than trusted at the transmute inside the detour body.
        let dalla_ittijah: DallaTaayeen = khatf_ittijah;
        let dalla_muhadhaha: DallaTaayeen = khatf_muhadhaha;
        let dalla_laff_mufrad: DallaQiyasMufrad = khatf_laff_mufrad;
        let dalla_laff_mudaaf: DallaQiyasMudaaf = khatf_laff_mudaaf;
        let dalla_mujassam_mufrad: DallaQiyasMufrad = khatf_mujassam_mufrad;
        let dalla_mujassam_mudaaf: DallaQiyasMudaaf = khatf_mujassam_mudaaf;

        daa(HadafKhatf::Ittijah, ahdaf.ittijah, dalla_ittijah as *const c_void, &ASL_ITTIJAH);
        daa(
            HadafKhatf::Muhadhaha,
            ahdaf.muhadhaha,
            dalla_muhadhaha as *const c_void,
            &ASL_MUHADHAHA,
        );
        match diqqa {
            DiqqatMuttajih::Mufrada => {
                daa(
                    HadafKhatf::Laff,
                    ahdaf.laff,
                    dalla_laff_mufrad as *const c_void,
                    &ASL_LAFF_MUFRAD,
                );
                daa(
                    HadafKhatf::Mujassam,
                    ahdaf.mujassam,
                    dalla_mujassam_mufrad as *const c_void,
                    &ASL_MUJASSAM_MUFRAD,
                );
            }
            DiqqatMuttajih::Mudaafa => {
                daa(
                    HadafKhatf::Laff,
                    ahdaf.laff,
                    dalla_laff_mudaaf as *const c_void,
                    &ASL_LAFF_MUDAAF,
                );
                daa(
                    HadafKhatf::Mujassam,
                    ahdaf.mujassam,
                    dalla_mujassam_mudaaf as *const c_void,
                    &ASL_MUJASSAM_MUDAAF,
                );
            }
        }

        for tanbeeh in &tanbeehat {
            tracing::warn!(khata = %tanbeeh, "a Slate correction was not installed");
        }
        for (khatf, _) in &khutuf {
            tracing::info!(
                hadaf = khatf.hadaf().ism(),
                unwan = format!("{:#x}", khatf.unwan().raqm()),
                "a Slate correction is installed"
            );
        }

        Self { khutuf, tanbeehat, diqqa }
    }

    /// How many corrections are installed.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.khutuf.len()
    }

    /// Whether any correction is installed.
    ///
    /// False is survivable and is not the same as an error: it means every
    /// target was absent, the game keeps its own direction, alignment and
    /// wrapping, and the Arabic text installed offline is still on screen.
    #[must_use]
    pub const fn mufaal(&self) -> bool {
        !self.khutuf.is_empty()
    }

    /// The measurement width this set was installed for.
    #[must_use]
    pub const fn diqqa(&self) -> DiqqatMuttajih {
        self.diqqa
    }

    /// The corrections, for diagnostics.
    pub fn khutuf(&self) -> impl Iterator<Item = &KhatfQiyas> {
        self.khutuf.iter().map(|(khatf, _)| khatf)
    }

    /// Every correction that could not be installed, with its named target.
    #[must_use]
    pub fn tanbeehat(&self) -> &[KhataUnreal] {
        &self.tanbeehat
    }

    /// Removes every correction and returns whatever refused to come out.
    ///
    /// An empty result is the contract being met: every prologue restored, every
    /// trampoline cleared, and the process byte-identical to the one this module
    /// entered. A non-empty result names each target that is not.
    ///
    /// Removal runs in reverse installation order, which is not decorative: the
    /// measurement hooks were installed last and are the ones a rendering thread
    /// is most likely to be inside, so they are the first to stop being entered.
    pub fn azil(&mut self) -> Vec<KhataUnreal> {
        let mut tanbeehat = Vec::new();
        while let Some((mut khatf, asl)) = self.khutuf.pop() {
            match khatf.azil(asl) {
                Ok(()) => tracing::info!(
                    hadaf = khatf.hadaf().ism(),
                    "a Slate correction was removed and its target restored"
                ),
                Err(khata) => {
                    tracing::warn!(khata = %khata, "a Slate correction did not come out cleanly");
                    tanbeehat.push(khata);
                }
            }
        }
        tanbeehat
    }
}

impl Drop for TasheehQiyas {
    /// Removes every correction if the caller did not.
    ///
    /// `taarib-haqn`'s rule is that a resource without a drop path does not
    /// ship, and a set of live detours in a foreign process is the sharpest
    /// version of that resource: dropping this value without removing them
    /// would leave the game jumping into a library that is no longer mapped.
    /// [`TasheehQiyas::azil`] is still the way to call it, because it returns
    /// what failed and this cannot.
    fn drop(&mut self) {
        let tanbeehat = self.azil();
        for tanbeeh in tanbeehat {
            tracing::warn!(khata = %tanbeeh, "a Slate correction was still installed at drop");
        }
    }
}
