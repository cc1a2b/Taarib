//! الاستيلاء — the Godot 3 takeover: Taarib shapes, Taarib positions, Taarib
//! draws.
//!
//! Godot 4 has a text server and this module would be an insult to it. Godot 3
//! has `Font::draw` and `Font::draw_char`, and between them there is no shaper,
//! no bidirectional algorithm, and no notion that a letter's form depends on its
//! neighbours. Handed Arabic, Godot 3 draws the isolated form of every letter,
//! left to right, in logical order. That is not "Arabic with a few problems"; it
//! is the exact failure this product exists to prevent, and there is no
//! correction that fixes it — only a replacement.
//!
//! So this module does not correct the engine. It intercepts the engine's text
//! drawing, lays the string out through `jisr`, and emits the resulting glyphs
//! itself. The engine's own drawing for those strings never runs.
//!
//! ## This module shapes; it does not translate
//!
//! Worth stating because it decides what a working installation looks like.
//! Every interception here begins by copying **the string the game already
//! decided to draw** and asking [`yahtaj`] whether it contains Arabic. On a game
//! that has not been translated the answer is `false` for every string, every
//! hook forwards to the engine, and the takeover — correctly installed,
//! correctly removable, doing exactly what it was built to do — changes nothing
//! a player can see. Making the game's strings Arabic in the first place is
//! [`crate::tawseel`]'s work, and the two halves are useless apart.
//!
//! ## Three interception points, and why the third is not optional
//!
//! | target | what it carries |
//! | --- | --- |
//! | `Font::draw` | a whole string in one call |
//! | `Font::draw_char` | one character at a time |
//! | `Font::get_string_size` | measurement |
//!
//! The second is not a redundant version of the first. Godot 3's `Label` does
//! **not** call `Font::draw`: it walks the line one character at a time through
//! `Font::draw_char`, so a takeover that hooked only the whole-string entry
//! point would leave every `Label` in the game unshaped, which is most of the
//! text in most games. What that costs to reassemble is [`Mujammi`]'s subject.
//!
//! The third looks like a luxury and is not. A game asks the font how wide a
//! string is, and then uses the answer to centre it, to right-align it, to size
//! the panel behind it, and to decide where the next widget starts. If the
//! glyphs are shaped correctly and the measurement still reports the width of
//! the unjoined logical string, every one of those decisions is made against a
//! number that no longer describes anything on screen: the text is drawn
//! perfectly and placed wrongly, boxes are the wrong size, and columns do not
//! line up. A player reads that as a broken mod just as readily as they read
//! unjoined letters as one.
//!
//! ## Addresses are supplied, never guessed
//!
//! [`AhdafIstila`] is filled in by the caller. Nothing in this file contains a
//! byte pattern, a symbol name to scan for, or an offset from a module base.
//!
//! That is a deliberate refusal rather than an omission. Godot 3 export
//! templates are built by whoever shipped the game: from any of a dozen point
//! releases, with or without symbols, statically or dynamically linked, with
//! modules compiled in or out, and frequently from a fork. A signature written
//! here would be a number that matched the handful of binaries someone happened
//! to have, could not be verified against the ones they did not, and would
//! silently point at the wrong function on the next release. A hook installed on
//! the wrong function is not a failed hook — it is a corrupted one, and it
//! corrupts a stranger's game. Whatever resolves addresses does so against the
//! binary in front of it and can be checked; this module is given the answer and
//! says so.
//!
//! ## Drawing through the one API that has not moved
//!
//! Glyphs are emitted through `VisualServer::canvas_item_add_texture_rect_region`
//! — a textured quad with a source rectangle, a destination rectangle and a
//! modulate colour, appended to a canvas item's command list.
//!
//! It is chosen for exactly one property: it is the same call in every Godot
//! 3.x. Its parameter list, its semantics and its place in `VisualServer` are
//! unchanged from 3.0 to 3.6, because it is the primitive that the engine's own
//! `BitmapFont::draw_char` has always ended in. Anything higher — a `Font`
//! subclass, a `CanvasItem` helper, a `TextLine` — either does not exist in 3.x
//! or changed shape inside it. Anything lower is a rasterizer backend, and there
//! are three of them. A takeover has to end in a call that will still be there in
//! the build that was not tested, and this is that call.
//!
//! What this module does *not* do is create the textures. Uploading the atlas
//! pages into the engine and telling this module their identifiers is the
//! caller's, through [`sajjil_safha`]. A module that went looking for a texture
//! would be a module that touches a game asset.
//!
//! ## Degrading is descending, not failing
//!
//! A hook that will not install is a [`KhataGodot::KhatfFashil`] naming the
//! target, kept in [`Istila::tanbeehat`], and the other hooks still go in. A
//! canvas binding that is missing or refuses is [`KhataGodot::RasmGhayrMutah`],
//! and because there is no takeover at all without somewhere to draw, that one
//! comes back from [`Istila::rakkib`] as an `Err`.
//!
//! Neither is the end of the phase. Both are the signal to descend to the glyph
//! transport in `naql`, which needs no hook and no canvas call, and which is why
//! [`Istila::nuzul_matlub`] exists as a question the caller can ask instead of
//! having to reconstruct the answer from a list of warnings.
//!
//! ## Uninstalling
//!
//! Every hook records the sixteen bytes it replaced before it replaces them, and
//! [`Istila::azil`] restores each one and compares. A restoration that reports
//! success but does not produce the recorded bytes is reported with both byte
//! strings, because a process that is not byte-identical after an uninstall is
//! something somebody has to be told about with evidence. [`Istila`] also drops
//! that way, since a live detour into an unmapped library is a crash with this
//! module's name on it.

use core::cell::{Cell, RefCell};
use core::ffi::{c_int, c_void};
use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use retour::RawDetour;
use taarib_jisr::anwa::{
    TAARIB_KHIYAR_SATR_WAHID, TaaribHarf, TaaribKhiyarat, TaaribLawha, TaaribMakhzanTakhtit,
    TaaribMawdiShakl, TaaribMiftahShakl, TaaribQiyasNass, TaaribSatr, TaaribSilsila, TaaribSiyaq,
    TaaribTalab, TaaribTaqreerTajawuz,
};
use taarib_jisr::awamir::{
    taarib_lawha_ibda_itar, taarib_lawha_shakl, taarib_qiyas, taarib_takhtit,
};
use taarib_jisr::khata_c::{TAARIB_NAJAH, TAARIB_SIAT_QASIRA};

use crate::khata::KhataGodot;

// ---------------------------------------------------------------------------
// Fixed quantities
// ---------------------------------------------------------------------------

/// How much of a target's prologue is recorded before it is replaced.
///
/// Sixteen bytes covers every patch either supported architecture needs: five
/// for a relative jump on `x86_64`, fourteen for an absolute indirect jump, and
/// sixteen for an ARM64 branch island. Both linkers pad function entry points to
/// at least sixteen bytes, so reading this many from a function address stays
/// inside the same executable section.
pub const TUL_LAQTA: usize = 16;

/// The longest string this takeover will copy out of the engine, in code units.
///
/// A ceiling on what a hostile or corrupt `String` pointer can cost, not a limit
/// on the game: sixteen thousand code units is far past the longest line any
/// game draws in one call and far short of a number that matters to a frame
/// budget. A string that declares more is left to the engine's own drawing
/// rather than copied, because the alternative is walking a foreign heap looking
/// for a terminator that may not be there.
pub const AQSA_WAHDAT: usize = 16_384;

/// How many atlas pages this module will hold identifiers for.
///
/// Eight one-megapixel pages is more shaped Arabic than any game has on screen
/// at once. A page beyond this is not drawn and is reported once rather than
/// once per glyph per frame, because a diagnostic that fires sixty times a
/// second is a diagnostic nobody reads.
pub const AQSA_SAFAHAT: usize = 8;

/// The glyph buffer's starting capacity, per thread.
///
/// Sized for a long line of dialogue so that the first frame a string is drawn
/// does not also pay for a reallocation. The buffer grows to fit whatever the
/// game actually draws and then stops growing for the life of the process.
pub const ADAD_HURUF_IBTIDAI: usize = 512;

/// The line buffer's starting capacity, per thread.
///
/// Both intercepted paths lay out a single line — Godot 3's `Font::draw` does
/// not wrap — so this is only ever one in practice, and is not one here because
/// a buffer negotiation that can never succeed on the first try is a code path
/// that never gets exercised.
pub const ADAD_SUTUR_IBTIDAI: usize = 8;

/// A zeroed glyph, used to fill the reusable buffer.
///
/// The buffer is fully initialised before `jisr` writes into it, which is why
/// nothing here needs `MaybeUninit` or `set_len`: the vector's length always
/// equals its capacity, `jisr` reports how many entries it wrote, and reading
/// that prefix is an ordinary slice of ordinary initialised values.
const HARF_SIFRI: TaaribHarf = TaaribHarf {
    muarrif: 0,
    anqud: 0,
    s: 0.0,
    a: 0.0,
    taqaddum: 0.0,
    nitaq: 0,
    khatt: 0,
    alam: 0,
};

/// A zeroed line, used to fill the reusable buffer, for [`HARF_SIFRI`]'s reason.
const SATR_SIFRI: TaaribSatr = TaaribSatr {
    awwal_harf: 0,
    adad_huruf: 0,
    bidayat_mantiqi: 0,
    nihayat_mantiqi: 0,
    asas: 0.0,
    bidaya: 0.0,
    ard: 0.0,
    irtifa: 0.0,
    suud: 0.0,
    hubut: 0.0,
    dabt: 0.0,
    alam: 0,
};

// ---------------------------------------------------------------------------
// Addresses and the engine's small value types
// ---------------------------------------------------------------------------

/// An address in the game process.
///
/// A newtype rather than a bare `usize` so that null is refused once, here,
/// instead of at each of the places that would otherwise have to remember: null
/// is the failure return of every symbol-lookup and vtable-read path that can
/// produce one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unwan(usize);

impl Unwan {
    /// Wraps a raw address, refusing null.
    #[must_use]
    pub const fn jadeed(khaam: usize) -> Option<Self> {
        if khaam == 0 { None } else { Some(Self(khaam)) }
    }

    /// Wraps a pointer from a platform lookup, refusing null.
    #[must_use]
    pub fn min_muashir(muashir: *const c_void) -> Option<Self> {
        Self::jadeed(muashir.expose_provenance())
    }

    /// The address as an integer, for logging and comparison.
    #[must_use]
    pub const fn raqm(self) -> usize {
        self.0
    }

    /// The address as a pointer, restoring the provenance that
    /// [`Unwan::min_muashir`] exposed.
    ///
    /// Only as valid as the module it came from: it points into a foreign image
    /// that could in principle be unloaded, which is why every hook installed on
    /// one is removed before this library unloads.
    #[must_use]
    pub const fn muashir(self) -> *const c_void {
        core::ptr::with_exposed_provenance(self.0)
    }
}

/// Godot's `RID` — a handle to an engine-side resource.
///
/// Godot 3's `RID` is a class whose only member is a `RID_Data *`, with no
/// virtual functions, no user-declared copy constructor and no destructor. That
/// makes it trivially copyable, which is the property both supported calling
/// conventions key on when deciding how to pass an eight-byte class: in an
/// integer register, exactly as a bare pointer would be. `#[repr(transparent)]`
/// over a raw pointer therefore reproduces the C++ ABI for it rather than
/// approximating it.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hawiya(*mut c_void);

impl Hawiya {
    /// The null identifier, which is what a default-constructed `RID` is and
    /// what the normal-map argument of a text draw always is.
    #[must_use]
    pub const fn batila() -> Self {
        Self(core::ptr::null_mut())
    }

    /// Wraps an identifier the caller obtained from the engine.
    #[must_use]
    pub const fn min_raqm(khaam: usize) -> Self {
        Self(core::ptr::with_exposed_provenance_mut(khaam))
    }

    /// The identifier as an integer, for the registry and for logging.
    #[must_use]
    pub fn raqm(self) -> usize {
        self.0.expose_provenance()
    }

    /// Whether this identifier names nothing.
    #[must_use]
    pub const fn khaliya(self) -> bool {
        self.0.is_null()
    }
}

/// Godot's `Vector2`, which is also its `Point2` and its `Size2`.
///
/// Two `real_t`. This module handles the single-precision `real_t` that every
/// official Godot 3 export template is built with; a custom `float=64` build is
/// refused at install by [`DiqqatHaqiqi`] rather than read at the wrong width,
/// because reading eight bytes as two `f32` turns a screen position into a
/// denormal and puts every string at the origin.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Muttajih2 {
    /// The horizontal component.
    pub s: f32,
    /// The vertical component.
    pub a: f32,
}

/// Godot's `Rect2` — a position and a size, four `real_t` in that order.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mustatil {
    /// The top-left corner.
    pub mawdi: Muttajih2,
    /// The extent.
    pub hajm: Muttajih2,
}

/// Godot's `Color` — four `float`, red green blue alpha, in that order.
///
/// `Color` is `float` in Godot 3 regardless of the `real_t` build flag, so
/// unlike [`Muttajih2`] this one has no precision variant to get wrong.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Lawn {
    /// Red.
    pub ahmar: f32,
    /// Green.
    pub akhdar: f32,
    /// Blue.
    pub azraq: f32,
    /// Alpha.
    pub shaffafiya: f32,
}

impl Lawn {
    /// Opaque white — the modulate that leaves an atlas glyph its own colour,
    /// and the fallback for a colour pointer the engine did not supply.
    #[must_use]
    pub const fn abyad() -> Self {
        Self {
            ahmar: 1.0,
            akhdar: 1.0,
            azraq: 1.0,
            shaffafiya: 1.0,
        }
    }

    /// Whether every component is finite.
    ///
    /// A NaN modulate is not a drawing error in any backend — it is a quad that
    /// renders as nothing at all, which would read as text that vanished rather
    /// than text that was mis-coloured.
    #[must_use]
    pub const fn salih(self) -> bool {
        self.ahmar.is_finite()
            && self.akhdar.is_finite()
            && self.azraq.is_finite()
            && self.shaffafiya.is_finite()
    }
}

// ---------------------------------------------------------------------------
// The two width questions that decide whether anything read is real
// ---------------------------------------------------------------------------

/// The width of one code unit in Godot 3's `String`.
///
/// Godot 3 stores text as `CharType`, which is `wchar_t`, which is **two bytes
/// on Windows and four on Linux and macOS**. That single sentence is the whole
/// hazard: a Windows build's `String` is UTF-16 with surrogate pairs, a Linux
/// build's is UTF-32 with none, and the two are indistinguishable by looking at
/// the pointer. Read a UTF-16 buffer four bytes at a time and every second
/// character is folded into the high half of its neighbour, producing codepoints
/// far outside Unicode; read a UTF-32 buffer two bytes at a time and every
/// character is followed by a phantom NUL, which terminates the string after one
/// letter. Neither failure looks like a decoding bug from the outside — the
/// first draws garbage and the second draws nothing.
///
/// The width is therefore stated by the caller, which knows which binary it
/// opened, and [`Majhula`](ArdWahda::Majhula) is refused at install rather than
/// guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArdWahda {
    /// Two bytes: `wchar_t` on Windows, so UTF-16 with surrogate pairs.
    Thunaiya,
    /// Four bytes: `wchar_t` on Linux and macOS, so UTF-32.
    Rubaiya,
    /// Not stated.
    ///
    /// The default on purpose. A default of either real width would be a guess
    /// that is right half the time and silently draws nonsense the other half,
    /// and a caller that forgot to set this should be told so at install.
    #[default]
    Majhula,
}

impl ArdWahda {
    /// The width in bytes, or [`None`] when it was never stated.
    #[must_use]
    pub const fn bayt(self) -> Option<usize> {
        match self {
            Self::Thunaiya => Some(2),
            Self::Rubaiya => Some(4),
            Self::Majhula => None,
        }
    }
}

/// The width of Godot 3's `real_t`, which decides the shape of every coordinate.
///
/// Every official Godot 3 export template is built single-precision. A build
/// with `float=64` doubles `Vector2`, `Rect2` and every position argument, and a
/// takeover that read those at the wrong width would place text at coordinates
/// in the order of `1e-315`. There is no correct reading of a `Vector2` without
/// knowing which it is, so the double-precision case is declined at install with
/// [`KhataGodot::MuharrikGhayrMadum`] and the caller descends to the transport,
/// which has no coordinates in it at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiqqatHaqiqi {
    /// `real_t` is `float`. Every official 3.x export template.
    #[default]
    Mufrada,
    /// `real_t` is `double`, from a custom `float=64` build.
    Mudaafa,
}

// ---------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------

/// Which of the three interception points a hook is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HadafIstila {
    /// `Font::draw` — a whole string in one call.
    Rasm,
    /// `Font::draw_char` — one character at a time, which is how `Label` draws.
    RasmHarf,
    /// `Font::get_string_size` — measurement.
    Qiyas,
}

impl HadafIstila {
    /// The name that appears in [`KhataGodot::KhatfFashil`] and in the log.
    ///
    /// A `&'static str` because the error variant takes one, and because a hook
    /// failure that named a heap string would be a hook failure whose message
    /// depended on an allocation succeeding in a process already in trouble.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Rasm => "Font::draw",
            Self::RasmHarf => "Font::draw_char",
            Self::Qiyas => "Font::get_string_size",
        }
    }

    /// What a missing address costs, per target.
    ///
    /// Three sentences rather than one, because the three losses are not
    /// comparable. Without `Font::draw` the strings drawn whole stay unjoined;
    /// without `Font::draw_char` every `Label` does; without the measurement the
    /// glyphs are right and everything positioned from their width is wrong.
    #[must_use]
    pub const fn tafsil_ghiyab(self) -> &'static str {
        match self {
            Self::Rasm => {
                "no address was supplied for Font::draw, so strings drawn through it keep \
                 the engine's own unshaped output; the other interception points still run"
            },
            Self::RasmHarf => {
                "no address was supplied for Font::draw_char, which is the path Godot 3's \
                 Label draws through, so labels keep the engine's own unshaped output"
            },
            Self::Qiyas => {
                "no address was supplied for Font::get_string_size, so the game keeps \
                 measuring the unshaped string: glyphs will be drawn correctly and anything \
                 centred, right-aligned or sized from that measurement will be placed wrongly"
            },
        }
    }
}

/// The addresses the takeover is installed at.
///
/// Supplied, never discovered — see this module's header for why a signature
/// written into this file would be a number nobody could verify. Every field is
/// optional, and a [`None`] is one interception point that does not run while
/// the others do.
#[derive(Debug, Clone, Copy, Default)]
pub struct AhdafIstila {
    /// `Font::draw`.
    pub rasm: Option<Unwan>,
    /// `Font::draw_char`.
    pub rasm_harf: Option<Unwan>,
    /// `Font::get_string_size`.
    pub qiyas: Option<Unwan>,
}

/// Where the glyphs go: the `VisualServer` singleton and the one call on it.
///
/// `canvas_item_add_texture_rect_region` is a virtual member function, so its
/// address is not a link-time constant even in a build with symbols — it lives
/// in the singleton's vtable, at a slot index that moves whenever a virtual is
/// added above it, which happened several times inside 3.x. The caller resolves
/// it; [`WaslRasm::min_jadwal`] is offered for the common way of doing that, and
/// takes the slot index as an argument rather than knowing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaslRasm {
    /// The `VisualServer` singleton, which is the call's `this`.
    hadha: usize,
    /// The resolved address of `canvas_item_add_texture_rect_region`.
    dalla: usize,
}

impl WaslRasm {
    /// Binds an already-resolved singleton and function address.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::RasmGhayrMutah`] when either address is null. Both are
    /// checked here rather than at the first draw, because a null discovered
    /// mid-frame is a null discovered after half a string has been emitted.
    pub fn jadeed(hadha: Unwan, dalla: Unwan) -> Result<Self, KhataGodot> {
        let wasl = Self {
            hadha: hadha.raqm(),
            dalla: dalla.raqm(),
        };
        if wasl.hadha == 0 || wasl.dalla == 0 {
            return Err(KhataGodot::RasmGhayrMutah {
                sabab: "the VisualServer singleton or its canvas draw entry point was null"
                    .to_owned(),
            });
        }
        Ok(wasl)
    }

    /// Resolves the call out of the singleton's vtable at a caller-supplied slot.
    ///
    /// The index is an argument because it is not knowable from here: it is a
    /// property of the exact engine build, and a constant written into this file
    /// would name a different virtual function on any release it was not
    /// measured against — which is a call into unrelated engine code with a
    /// text draw's arguments.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::RasmGhayrMutah`] when the singleton is null, when the slot
    /// index is past [`AQSA_JADWAL`], or when the slot holds null.
    ///
    /// # Safety
    ///
    /// `hadha` must point at a live polymorphic C++ object whose first
    /// pointer-sized word is its vtable pointer — which is what both supported
    /// ABIs put there — and `fahras` must be a slot that exists in that vtable.
    /// A slot index past the end of a real vtable reads whatever follows it in
    /// read-only data, and [`AQSA_JADWAL`] bounds the damage without being able
    /// to prevent it.
    pub unsafe fn min_jadwal(hadha: Unwan, fahras: usize) -> Result<Self, KhataGodot> {
        if fahras >= AQSA_JADWAL {
            return Err(KhataGodot::RasmGhayrMutah {
                sabab: format!(
                    "vtable slot {fahras} is past the {AQSA_JADWAL} this build will read, \
                     which is a slot index that did not come from a real vtable"
                ),
            });
        }
        // Three levels: the object points at its vtable, the vtable is an array
        // of function pointers, and the slot holds one.
        let hadha_ptr: *const *const *const c_void = hadha.muashir().cast();
        // SAFETY: the caller guarantees `hadha` is a live polymorphic object, so
        // its first pointer-sized word is its vtable pointer.
        let jadwal = unsafe { hadha_ptr.read() };
        // SAFETY: the caller guarantees `fahras` names a slot in that vtable,
        // and the bound above keeps the offset inside what a real table holds.
        let dalla = unsafe { jadwal.add(fahras).read() };
        let Some(dalla) = Unwan::min_muashir(dalla) else {
            return Err(KhataGodot::RasmGhayrMutah {
                sabab: format!("vtable slot {fahras} of the VisualServer singleton holds null"),
            });
        };
        Self::jadeed(hadha, dalla)
    }

    /// The singleton address.
    #[must_use]
    pub const fn hadha(self) -> usize {
        self.hadha
    }

    /// The resolved draw call's address.
    #[must_use]
    pub const fn dalla(self) -> usize {
        self.dalla
    }
}

/// The furthest vtable slot [`WaslRasm::min_jadwal`] will read.
///
/// `VisualServer` is a large interface and the canvas commands sit well into it,
/// so this is generous. It is a bound on a mistake rather than a description of
/// the engine: a slot index that lands here did not come from reading a real
/// vtable, and stopping is better than dereferencing whatever sits after one.
pub const AQSA_JADWAL: usize = 1024;

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

/// Everything the detours read, fixed once at install.
///
/// Every field is a scalar, which is what makes the whole structure [`Send`] and
/// [`Sync`] without an `unsafe impl`. That matters because the detours are plain
/// `extern "C"` functions with no captured environment: a `OnceLock` holding
/// this is the only channel they have, and a channel that needed a lock would be
/// a lock taken on the rendering thread for every glyph.
#[derive(Debug, Clone, Copy)]
pub struct HalatIstila {
    /// A live `jisr` context, as an exposed address. Zero disables the takeover
    /// entirely and every hook forwards to the engine.
    pub siyaq: usize,
    /// The `jisr` font chain the patch's fonts were registered into, as an
    /// exposed address.
    pub silsila: usize,
    /// The `jisr` atlas the glyphs are rasterized into, as an exposed address.
    pub lawha: usize,
    /// Where the glyphs are emitted.
    pub wasl: WaslRasm,
    /// The width of one `String` code unit in this build.
    pub ard_wahda: ArdWahda,
    /// The width of `real_t` in this build.
    pub diqqa: DiqqatHaqiqi,
    /// The pixel size text is laid out and rasterized at.
    ///
    /// From the patch, not from the engine's font object. A `DynamicFont`'s size
    /// lives in a private field whose offset differs between point releases, and
    /// reading it would be exactly the unverifiable guess this module refuses to
    /// make about addresses. A patch is built for a game, and the size the game
    /// draws at is one of the things it records.
    pub hajm: f32,
    /// Whether every intercepted string is taken over, or only the ones that
    /// contain Arabic.
    ///
    /// False by default and for good reason. A takeover of every string replaces
    /// the game's own Latin typography with the patch's font, and a player who
    /// watches the English half of a menu change typeface reads that as damage
    /// done by the mod — which it is. Only the strings that Godot 3 cannot draw
    /// are taken from it.
    pub kull_nass: bool,
    /// The direction discriminant handed to `jisr`, from
    /// [`taarib_jisr::anwa::ittijah_min_raqm`]'s vocabulary.
    pub ittijah: u32,
    /// The language discriminant handed to `jisr`.
    pub lugha: u32,
    /// The diacritic policy, so runtime layout agrees with what the patch
    /// compiler measured offline.
    pub tashkeel: u32,
    /// The digit policy, for the same reason.
    pub arqam: u32,
    /// Extra letter spacing in pixels, applied after shaping.
    pub tabaud_ahruf: f32,
    /// Extra spacing added to every space.
    pub tabaud_kalimat: f32,
    /// The longest run this takeover will copy out of the engine, in code units,
    /// clamped at install to [`AQSA_WAHDAT`].
    pub aqsa_wahdat: usize,
}

impl Default for HalatIstila {
    fn default() -> Self {
        Self {
            siyaq: 0,
            silsila: 0,
            lawha: 0,
            wasl: WaslRasm { hadha: 0, dalla: 0 },
            ard_wahda: ArdWahda::Majhula,
            diqqa: DiqqatHaqiqi::Mufrada,
            hajm: 0.0,
            kull_nass: false,
            ittijah: 1,
            lugha: 1,
            tashkeel: 0,
            arqam: 0,
            tabaud_ahruf: 0.0,
            tabaud_kalimat: 0.0,
            aqsa_wahdat: AQSA_WAHDAT,
        }
    }
}

impl HalatIstila {
    /// Whether this policy has everything a draw needs.
    ///
    /// Checked once per intercepted call rather than trusted, because a policy
    /// that was published with a zero handle would otherwise reach `jisr` as a
    /// null and be refused there — correctly, but per glyph, in a log the player
    /// is the one who has to read.
    #[must_use]
    pub fn jahiza(&self) -> bool {
        self.siyaq != 0
            && self.silsila != 0
            && self.lawha != 0
            && self.wasl.hadha != 0
            && self.wasl.dalla != 0
            && self.hajm.is_finite()
            && self.hajm > 0.0
    }

    /// The `jisr` context handle.
    const fn maqbad_siyaq(&self) -> TaaribSiyaq {
        core::ptr::with_exposed_provenance_mut(self.siyaq)
    }

    /// The `jisr` font chain handle.
    const fn maqbad_silsila(&self) -> TaaribSilsila {
        core::ptr::with_exposed_provenance_mut(self.silsila)
    }

    /// The `jisr` atlas handle.
    const fn maqbad_lawha(&self) -> TaaribLawha {
        core::ptr::with_exposed_provenance_mut(self.lawha)
    }

    /// The layout options, which are the same for every intercepted string.
    ///
    /// Single line unconditionally: Godot 3's `Font::draw` does not wrap, and a
    /// layout that wrapped would draw a second line the game reserved no space
    /// for, over whatever is beneath it.
    const fn khiyarat(&self) -> TaaribKhiyarat {
        TaaribKhiyarat {
            sifat: core::ptr::null(),
            adad_sifat: 0,
            ittijah: self.ittijah,
            lugha: self.lugha,
            dabt: 0,
            muhadhaha: 0,
            tashkeel: self.tashkeel,
            arqam: self.arqam,
            tajawuz: 0,
            hajm_adna: 0.0,
            irtifa_satr: 0.0,
            tabaud_ahruf: self.tabaud_ahruf,
            tabaud_kalimat: self.tabaud_kalimat,
            alam: TAARIB_KHIYAR_SATR_WAHID,
        }
    }
}

// ---------------------------------------------------------------------------
// Shared state the detours read
// ---------------------------------------------------------------------------

/// The policy, published once at install and read by every detour afterwards.
static HALA: OnceLock<HalatIstila> = OnceLock::new();

/// The trampoline back to the original `Font::draw`.
static ASL_RASM: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to the original `Font::draw_char`.
static ASL_RASM_HARF: AtomicUsize = AtomicUsize::new(0);
/// The trampoline back to the original `Font::get_string_size`.
static ASL_QIYAS: AtomicUsize = AtomicUsize::new(0);

/// Texture identifiers for the atlas pages, one slot per page.
///
/// Atomics rather than a lock because this is written between frames by whoever
/// uploads the atlas and read inside the draw path for every glyph, and a
/// rendering thread that took a lock per glyph would be a rendering thread that
/// stalled on the uploader.
static SAFAHAT: [AtomicUsize; AQSA_SAFAHAT] = [const { AtomicUsize::new(0) }; AQSA_SAFAHAT];

/// The policy, or one that changes nothing, for a detour that somehow ran before
/// the policy was published.
///
/// Cannot happen in the installation order this module enforces — the policy is
/// published before the first hook is enabled — and is written as a fallback
/// rather than an assertion because the alternative inside an `extern "C"`
/// detour is a panic crossing an ABI boundary, which aborts a player's game.
fn hala() -> HalatIstila {
    *HALA.get().unwrap_or(&HalatIstila {
        siyaq: 0,
        silsila: 0,
        lawha: 0,
        wasl: WaslRasm { hadha: 0, dalla: 0 },
        ard_wahda: ArdWahda::Majhula,
        diqqa: DiqqatHaqiqi::Mufrada,
        hajm: 0.0,
        kull_nass: false,
        ittijah: 0,
        lugha: 0,
        tashkeel: 0,
        arqam: 0,
        tabaud_ahruf: 0.0,
        tabaud_kalimat: 0.0,
        aqsa_wahdat: 0,
    })
}

/// Publishes the engine texture identifier for one atlas page.
///
/// Called by whoever uploads the atlas, after creating the texture and before
/// the frame that draws from it. Uploading is deliberately not this module's
/// work: creating an engine texture means touching the engine's resource system,
/// and a text-drawing module that also managed textures would be two modules.
///
/// # Errors
///
/// [`KhataGodot::HajmMufrit`] when `fahras` is past [`AQSA_SAFAHAT`], naming the
/// page count and the ceiling. Refused rather than ignored: a page that is not
/// registered is a page whose glyphs silently do not draw, and a caller with
/// more pages than this build holds needs to know that now rather than from a
/// screenshot with holes in the text.
pub fn sajjil_safha(fahras: usize, nasij: Hawiya) -> Result<(), KhataGodot> {
    let Some(khana) = SAFAHAT.get(fahras) else {
        return Err(KhataGodot::HajmMufrit {
            haql: "atlas page index",
            qeema: u64::try_from(fahras).unwrap_or(u64::MAX),
            saqf: u64::try_from(AQSA_SAFAHAT).unwrap_or(u64::MAX),
        });
    };
    khana.store(nasij.raqm(), Ordering::Release);
    Ok(())
}

/// The engine texture identifier registered for one atlas page, if any.
#[must_use]
pub fn hawiyat_safha(fahras: usize) -> Option<Hawiya> {
    let khaam = SAFAHAT.get(fahras)?.load(Ordering::Acquire);
    if khaam == 0 {
        None
    } else {
        Some(Hawiya::min_raqm(khaam))
    }
}

/// Forgets every registered page.
///
/// Called when the atlas is destroyed or rebuilt. Leaving stale identifiers
/// behind would have the draw path handing the engine a texture identifier that
/// no longer names a texture, which every backend answers differently and none
/// of them answers well.
pub fn imsah_safahat() {
    for khana in &SAFAHAT {
        khana.store(0, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// Re-entrancy
// ---------------------------------------------------------------------------

thread_local! {
    /// Whether this thread is already inside a Taarib interception.
    ///
    /// Thread-local rather than global: Godot 3 draws from one thread, but a
    /// global flag would mean a second thread's draw silently forwarded because
    /// the first happened to be mid-string, and a rule that depends on timing is
    /// a bug that only appears on somebody else's machine.
    static DAKHIL: Cell<bool> = const { Cell::new(false) };
}

/// Held for as long as this thread is inside an interception.
///
/// The takeover draws by calling back into the engine, and the engine's canvas
/// code is entitled to call anything — including, on some builds, back through a
/// `Font` entry point this module has hooked. Without a guard that is unbounded
/// recursion on the rendering thread, and the shallow version of it is worse
/// than the deep one: `Font::draw`'s own implementation walks the string calling
/// `Font::draw_char`, so a takeover that hooked both and let the outer one fall
/// through to the original would draw every string twice, once shaped and once
/// not, one on top of the other.
///
/// The rule is one line long: if the guard is already held, the hook forwards to
/// the original and does nothing else. Whatever the engine was doing continues
/// to happen, exactly once, in the engine's own code.
struct Hars;

impl Hars {
    /// Takes the guard, or returns [`None`] because this thread already has it.
    fn ihjiz() -> Option<Self> {
        DAKHIL
            .try_with(|dakhil| {
                if dakhil.get() {
                    None
                } else {
                    dakhil.set(true);
                    Some(Self)
                }
            })
            .ok()
            .flatten()
    }
}

impl Drop for Hars {
    fn drop(&mut self) {
        let _ = DAKHIL.try_with(|dakhil| dakhil.set(false));
    }
}

// ---------------------------------------------------------------------------
// Reusable buffers
// ---------------------------------------------------------------------------

/// The buffers the draw path reuses, one set per thread.
///
/// The draw path runs for every string every frame, so what it allocates
/// matters. It allocates on exactly two occasions: the first time a thread
/// intercepts anything, and any later time a string is longer than every string
/// that thread has seen. After a few seconds of play the buffers are as large as
/// the game's longest line and nothing here touches the allocator again — the
/// same steady state `jisr`'s own buffer negotiation is built around. The text
/// buffer is a `String` reused by clearing rather than a fresh one per call for
/// precisely this reason, and the glyph and line buffers are pre-filled with
/// zeroed values so that reading back what `jisr` wrote is an ordinary slice
/// rather than a length invariant somebody has to maintain.
struct Muaddat {
    /// The intercepted string, converted to UTF-8 for `jisr`.
    nass: String,
    /// The glyph buffer handed to `jisr`.
    huruf: Vec<TaaribHarf>,
    /// The line buffer handed to `jisr`.
    sutur: Vec<TaaribSatr>,
}

impl Muaddat {
    /// The starting sizes, paid once per thread.
    fn jadeed() -> Self {
        Self {
            nass: String::with_capacity(1024),
            huruf: vec![HARF_SIFRI; ADAD_HURUF_IBTIDAI],
            sutur: vec![SATR_SIFRI; ADAD_SUTUR_IBTIDAI],
        }
    }
}

thread_local! {
    static MUAADDAT: RefCell<Muaddat> = RefCell::new(Muaddat::jadeed());
}

/// The most glyphs one intercepted string may produce.
///
/// Four per code unit is past what any shaping of any script produces from a
/// bounded input, and the bound exists so that a pathological string cannot make
/// a thread's buffer permanent at an arbitrary size.
const AQSA_HURUF: usize = AQSA_WAHDAT * 4;

// ---------------------------------------------------------------------------
// The character-at-a-time run
// ---------------------------------------------------------------------------

/// A run of characters recovered from consecutive `Font::draw_char` calls.
///
/// Godot 3's `Label` does not hand the font a string. It walks the line one
/// character at a time, and a single character cannot be shaped: whether a letter
/// is initial, medial, final or isolated is a fact about its neighbours, and a
/// takeover that shaped each call in isolation would produce exactly the unjoined
/// letters it exists to replace.
///
/// So the calls are reassembled. A run opens at the first character the takeover
/// claims, records the pen position of that character as its anchor, and
/// accumulates every character after it. `Font::draw_char` is handed the *next*
/// character as an argument, and Godot passes zero there for the last one — that
/// is the end-of-run signal, and it is in the ABI rather than inferred. A change
/// of canvas item, colour or outline pass also ends the run, because those are
/// three different draws and not one.
///
/// Nothing is drawn until the run closes; then the whole run is shaped and drawn
/// at the anchor.
///
/// ## The advance this returns, and what it costs
///
/// `Font::draw_char` returns how far the caller's pen should move. Every
/// accumulated character returns zero and the character that closes the run
/// returns the run's whole shaped width, so the pen ends exactly where the shaped
/// text ends and anything the game draws after the run lands correctly.
///
/// The cost is that the pen does not move *within* the run, which matters only to
/// a caller that clips character by character against its own pen. Godot 3's
/// `Label` does not — it decides line contents up front — and for a caller that
/// does, the failure is a run drawn slightly past a clip rather than a run not
/// drawn at all. The alternative is measuring the growing prefix on every
/// character, which is a shaping call per character per frame for the life of the
/// label.
struct Mujammi {
    /// The accumulated characters, in logical order.
    nass: String,
    /// The canvas item the run is being drawn into.
    band: usize,
    /// The pen position of the run's first character: baseline, leading edge.
    mawdi: Muttajih2,
    /// The modulate colour the run was opened with.
    lawn: Lawn,
    /// Whether this is an outline pass.
    hashiya: bool,
    /// Whether a run is open at all.
    mufaal: bool,
}

impl Mujammi {
    /// An empty accumulator.
    fn jadeed() -> Self {
        Self {
            nass: String::with_capacity(256),
            band: 0,
            mawdi: Muttajih2::default(),
            lawn: Lawn::abyad(),
            hashiya: false,
            mufaal: false,
        }
    }

    /// Whether a call belongs to the run that is currently open.
    ///
    /// Colour is compared bitwise rather than numerically. These are the same
    /// four floats copied from the same widget frame after frame; a tolerance
    /// would let two genuinely different colours join into one run, and an exact
    /// comparison at worst splits one run into two, which draws both correctly.
    const fn yatabi(&self, band: usize, lawn: Lawn, hashiya: bool) -> bool {
        self.mufaal
            && self.band == band
            && self.hashiya == hashiya
            && self.lawn.ahmar.to_bits() == lawn.ahmar.to_bits()
            && self.lawn.akhdar.to_bits() == lawn.akhdar.to_bits()
            && self.lawn.azraq.to_bits() == lawn.azraq.to_bits()
            && self.lawn.shaffafiya.to_bits() == lawn.shaffafiya.to_bits()
    }

    /// Opens a run at a pen position.
    fn iftah(&mut self, band: usize, mawdi: Muttajih2, lawn: Lawn, hashiya: bool) {
        self.nass.clear();
        self.band = band;
        self.mawdi = mawdi;
        self.lawn = lawn;
        self.hashiya = hashiya;
        self.mufaal = true;
    }

    /// Closes the run without drawing it, returning what it held.
    const fn aghliq(&mut self) -> bool {
        let kana = self.mufaal;
        self.mufaal = false;
        kana
    }
}

thread_local! {
    static JARA: RefCell<Mujammi> = RefCell::new(Mujammi::jadeed());
}

// ---------------------------------------------------------------------------
// Reading Godot 3's String
// ---------------------------------------------------------------------------

/// Copies a Godot 3 `String` into `khuruj` as UTF-8, returning whether it could
/// be read at all.
///
/// ## Where the pointer comes from
///
/// `String`'s only data member is a `CowData<CharType>`, whose only data member
/// is the pointer to the characters, and neither class has a virtual function.
/// The first pointer-sized word of a `String` object is therefore the character
/// buffer. That is a layout fact from the engine's public headers rather than an
/// offset somebody measured in a debugger, which is the difference between this
/// and the address discovery this module refuses to do.
///
/// A null there is not a failure: `CowData` leaves the pointer null for an empty
/// string, and the correct reading of an empty string is an empty string.
///
/// ## Why the length is found by scanning
///
/// `CowData` keeps the element count in the words immediately *before* the
/// buffer, and this deliberately does not read them. That prefix is private
/// layout — its width and its position relative to the reference count are
/// implementation details of a header that has been rearranged inside 3.x — and
/// reading a number that decides how much memory is touched out of a layout
/// nobody can verify is precisely the class of guess this module exists without.
/// Godot 3's `String` is always kept NUL-terminated, so the terminator is the
/// documented, public way to find the end, and the scan is bounded by `aqsa` so
/// that a corrupt pointer costs a bounded read instead of a walk through the
/// heap.
///
/// The cost is that a string with an embedded NUL is truncated at it. Godot's UI
/// text does not contain one, and truncating a pathological string is a smaller
/// failure than trusting a private field.
///
/// # Safety
///
/// `kaain` must be null or point at a live Godot 3 `String`, and `ard` must be
/// the code-unit width that string was built with. A `String` read at the wrong
/// width is not a decoding error — it is a different buffer.
unsafe fn nass_min_string(
    kaain: *const c_void,
    ard: ArdWahda,
    aqsa: usize,
    khuruj: &mut String,
) -> bool {
    khuruj.clear();
    if kaain.is_null() || aqsa == 0 {
        return false;
    }
    let jidhr: *const *const c_void = kaain.cast();
    // SAFETY: the caller guarantees `kaain` points at a live `String`, whose
    // first pointer-sized word is its character buffer, per this function's
    // header. One aligned pointer is read from it and nothing else.
    let bayanat = unsafe { jidhr.read() };
    if bayanat.is_null() {
        return true;
    }
    match ard {
        ArdWahda::Majhula => false,
        // SAFETY: the caller guarantees the buffer's code-unit width, and the
        // buffer is NUL-terminated by `String`'s own invariant, so the bounded
        // scan below stays inside it.
        ArdWahda::Thunaiya => unsafe { min_utf16(bayanat.cast(), aqsa, khuruj) },
        // SAFETY: as above, at four bytes per unit.
        ArdWahda::Rubaiya => unsafe { min_utf32(bayanat.cast(), aqsa, khuruj) },
    }
}

/// Decodes a NUL-terminated UTF-16 buffer — the Windows `wchar_t` case.
///
/// An unpaired surrogate becomes U+FFFD rather than aborting: a malformed unit
/// is one wrong glyph, and refusing the string would hand the whole line back to
/// the engine's unshaped drawing over a single bad code unit.
///
/// # Safety
///
/// `bayanat` must be a NUL-terminated UTF-16 buffer, readable up to and
/// including its terminator or up to `aqsa` units, whichever comes first.
unsafe fn min_utf16(bayanat: *const u16, aqsa: usize, khuruj: &mut String) -> bool {
    let mut tul = 0usize;
    while tul < aqsa {
        // SAFETY: `tul` is below `aqsa`, and the caller guarantees the buffer is
        // readable for that many units or until the terminator, whichever is
        // sooner; the loop stops at the terminator.
        if unsafe { bayanat.add(tul).read() } == 0 {
            break;
        }
        tul += 1;
    }
    if tul >= aqsa {
        return false;
    }
    // SAFETY: `tul` units were just read one at a time from this pointer, so the
    // whole span is readable and correctly aligned for `u16`.
    let wahdat = unsafe { core::slice::from_raw_parts(bayanat, tul) };
    for harf in char::decode_utf16(wahdat.iter().copied()) {
        khuruj.push(harf.unwrap_or(char::REPLACEMENT_CHARACTER));
    }
    true
}

/// Decodes a NUL-terminated UTF-32 buffer — the Linux and macOS `wchar_t` case.
///
/// # Safety
///
/// As [`min_utf16`], at four bytes per unit.
unsafe fn min_utf32(bayanat: *const u32, aqsa: usize, khuruj: &mut String) -> bool {
    let mut tul = 0usize;
    while tul < aqsa {
        // SAFETY: as the scan in `min_utf16`, at four bytes per unit.
        let wahda = unsafe { bayanat.add(tul).read() };
        if wahda == 0 {
            break;
        }
        khuruj.push(char::from_u32(wahda).unwrap_or(char::REPLACEMENT_CHARACTER));
        tul += 1;
    }
    if tul >= aqsa {
        khuruj.clear();
        return false;
    }
    true
}

/// Decodes one `CharType` argument into a character.
///
/// A `CharType` parameter arrives in an integer register, zero-extended, so the
/// declaration reads it as a `u32` on both platforms and this narrows it to the
/// build's real width. On a two-byte build the value is one UTF-16 unit: a lone
/// surrogate cannot be a character by itself, and becomes U+FFFD rather than
/// half of one.
#[must_use]
pub fn harf_min_wahda(wahda: u32, ard: ArdWahda) -> Option<char> {
    match ard {
        ArdWahda::Majhula => None,
        ArdWahda::Thunaiya => {
            let dayyiq = u16::try_from(wahda & 0xFFFF).ok()?;
            if dayyiq == 0 {
                return None;
            }
            Some(char::from_u32(u32::from(dayyiq)).unwrap_or(char::REPLACEMENT_CHARACTER))
        },
        ArdWahda::Rubaiya => {
            if wahda == 0 {
                return None;
            }
            Some(char::from_u32(wahda).unwrap_or(char::REPLACEMENT_CHARACTER))
        },
    }
}

// ---------------------------------------------------------------------------
// What is claimed, and what is left to the engine
// ---------------------------------------------------------------------------

/// Whether a character belongs to the Arabic script.
///
/// The blocks are the Arabic script's own, not "everything right to left".
/// Hebrew needs no joining and Godot 3 draws it acceptably once the string is
/// reordered, and this module has no Hebrew font in the patch to draw it with; a
/// takeover that claimed it would replace text the engine was handling with text
/// it could not draw at all. The presentation-form blocks are included because a
/// game whose strings were already mangled into presentation forms by some
/// earlier tool is exactly the game that needs this most.
#[must_use]
pub fn arabi(harf: char) -> bool {
    matches!(u32::from(harf),
        0x0600..=0x06FF   // Arabic
        | 0x0750..=0x077F // Arabic Supplement
        | 0x0870..=0x089F // Arabic Extended-B
        | 0x08A0..=0x08FF // Arabic Extended-A
        | 0xFB50..=0xFDFF // Presentation Forms-A
        | 0xFE70..=0xFEFF // Presentation Forms-B
        | 0x10E60..=0x10E7F // Rumi numeral symbols
        | 0x1EE00..=0x1EEFF // Arabic mathematical alphabetic symbols
    )
}

/// Whether this string is one the takeover claims.
#[must_use]
pub fn yahtaj(nass: &str, kull: bool) -> bool {
    if nass.is_empty() {
        return false;
    }
    kull || nass.chars().any(arabi)
}

// ---------------------------------------------------------------------------
// The coordinate transform
// ---------------------------------------------------------------------------

/// The subpixel bucket every glyph is asked for.
///
/// Zero, always, and glyph positions are snapped to whole pixels to match. The
/// patch compiler rasterized this game's glyphs offline at bucket zero; asking
/// the atlas for a different bucket at runtime would find nothing there and
/// rasterize a second copy of every glyph in the game during the first frame it
/// appeared, which is the one frame a player is most likely to notice.
pub const BAKAT_MUTHABBAT: u8 = 0;

/// Turns a `jisr` glyph position into an engine pen position.
///
/// **This is the transform.** Getting it wrong by one ascent is the single most
/// likely way for this module to look broken while being nearly right, because
/// every string lands exactly one line too high and the result reads as a mod
/// that does not work rather than as an arithmetic error.
///
/// The two spaces disagree about what the y coordinate names:
///
/// - Godot 3's `Font::draw` and `Font::draw_char` take a position that is the
///   **pen at the baseline** — the engine's own documentation says in as many
///   words that the position specifies the baseline and not the top — and the
///   engine's `draw_char` is what subtracts the ascent before it emits a quad.
/// - `jisr` reports [`TaaribHarf::a`] as the glyph origin measured **downwards
///   from the layout's top edge**, and [`TaaribSatr::asas`] as where that line's
///   baseline sits in the same downward-from-the-top space.
///
/// So the layout's top edge lies `asas` above the engine's pen, and one glyph's
/// origin is:
///
/// ```text
/// engine.x = pen.x + harf.s
/// engine.y = pen.y - satr.asas + harf.a
/// ```
///
/// where `satr` is the first line's, since both intercepted entry points draw a
/// single line and the pen names that line's baseline. For a glyph sitting on
/// the baseline `harf.a` equals `satr.asas` and the y collapses back to `pen.y`,
/// which is the check worth remembering: the transform must be the identity for
/// an ordinary letter and must move marks and superscripts and nothing else.
///
/// Both components are rounded to whole pixels, which is what makes
/// [`BAKAT_MUTHABBAT`] correct: the quad is drawn at the position the glyph was
/// rasterized for, so no glyph is resampled between two texels.
#[must_use]
pub fn mawdi_harf(qalam: Muttajih2, asas: f32, harf: &TaaribHarf) -> Muttajih2 {
    Muttajih2 {
        s: (qalam.s + harf.s).round(),
        a: (qalam.a - asas + harf.a).round(),
    }
}

/// Places a rasterized glyph's quad around an origin.
///
/// The atlas reports the image's left bearing as how far right of the pen it
/// starts, and its top bearing as how far **above** the baseline the image's top
/// edge sits — so one is added and the other subtracted, and swapping the signs
/// there flips every letter's vertical placement about its own baseline.
#[must_use]
pub fn mustatil_shakl(asl: Muttajih2, shakl: &TaaribMawdiShakl) -> Mustatil {
    Mustatil {
        mawdi: Muttajih2 {
            s: asl.s + f32::from(shakl.izaha_s),
            a: asl.a - f32::from(shakl.izaha_a),
        },
        hajm: Muttajih2 {
            s: f32::from(shakl.ard),
            a: f32::from(shakl.irtifa),
        },
    }
}

/// The glyph's rectangle inside its atlas page.
#[must_use]
pub fn mustatil_masdar(shakl: &TaaribMawdiShakl) -> Mustatil {
    Mustatil {
        mawdi: Muttajih2 {
            s: f32::from(shakl.s),
            a: f32::from(shakl.a),
        },
        hajm: Muttajih2 {
            s: f32::from(shakl.ard),
            a: f32::from(shakl.irtifa),
        },
    }
}

/// A pixel size as the quarter-pixel unit the atlas key uses.
///
/// Rounded and clamped before the narrowing conversion, and narrowed through a
/// checked one, so the conversion is exact rather than merely likely to be.
///
/// The size arrives from the patch file — see [`HalatIstila::hajm`] — so a
/// negative or non-finite one is a malformed patch and not an impossibility. The
/// intermediate is signed and the conversion checked so that such a size answers
/// zero, which draws nothing, rather than folding to a plausible atlas key.
#[expect(
    clippy::cast_possible_truncation,
    reason = "no expression form of a float-to-integer cast satisfies this lint; the value \
              is rounded to an integer and clamped into 0..=u16::MAX on the four lines \
              above, so the cast is exact and the `try_from` below cannot fail"
)]
#[must_use]
pub fn hajm_rubi(hajm: f32) -> u16 {
    let rub = (hajm * 4.0).round();
    if !rub.is_finite() || rub <= 0.0 {
        return 0;
    }
    if rub >= f32::from(u16::MAX) {
        return u16::MAX;
    }
    u16::try_from(rub as i32).unwrap_or(u16::MAX)
}

// ---------------------------------------------------------------------------
// Laying out through jisr, and drawing what comes back
// ---------------------------------------------------------------------------

/// The most lines one intercepted string may produce.
///
/// Both entry points ask for a single line, so this is one in practice. It is
/// not one here because a buffer negotiation that can never fail is a code path
/// that is never exercised until the day it matters.
const AQSA_SUTUR: usize = 64;

/// How many glyphs have been skipped for want of a registered atlas page.
///
/// Counted so the warning can be issued once. A diagnostic that fires per glyph
/// per frame is sixty thousand lines a second in a log nobody then reads.
static SAFAHAT_MAFQUDA: AtomicUsize = AtomicUsize::new(0);

/// What a successful layout produced.
#[derive(Debug, Clone, Copy)]
struct NatijatTakhtit {
    /// How many glyphs were written into the buffer.
    adad_huruf: usize,
    /// How many lines were written.
    adad_sutur: usize,
    /// The widest line's width, which is the advance the engine is told about.
    ard: f32,
}

/// Lays the buffered text out into the buffered glyph and line arrays.
///
/// Grows the buffers and retries once on a short-buffer answer, which is the
/// negotiation `jisr` documents: grow, retry, and after the first few frames the
/// buffers never grow again. Two attempts and no more, because a second short
/// answer after growing to the reported size means the reported size was not the
/// requirement, and looping on that would be looping in a frame.
fn khattit(
    hala: &HalatIstila,
    nass: &str,
    huruf: &mut Vec<TaaribHarf>,
    sutur: &mut Vec<TaaribSatr>,
) -> Option<NatijatTakhtit> {
    if nass.is_empty() {
        return Some(NatijatTakhtit {
            adad_huruf: 0,
            adad_sutur: 0,
            ard: 0.0,
        });
    }
    let talab = TaaribTalab {
        nass: nass.as_ptr(),
        tul_nass: nass.len(),
        nitaqat: core::ptr::null(),
        adad_nitaqat: 0,
        silsila: hala.maqbad_silsila(),
        hajm: hala.hajm,
        // No available width and no available height: Godot 3 draws one line and
        // does its own clipping, so a layout that wrapped would put a second
        // line into space the game reserved nothing for.
        ard_mutah: 0.0,
        irtifa_mutah: 0.0,
        khiyarat: hala.khiyarat(),
    };
    for _ in 0..2u8 {
        let mut makhzan = TaaribMakhzanTakhtit {
            huruf: huruf.as_mut_ptr(),
            siaat_huruf: huruf.len(),
            adad_huruf: 0,
            sutur: sutur.as_mut_ptr(),
            siaat_sutur: sutur.len(),
            adad_sutur: 0,
            ard: 0.0,
            irtifa: 0.0,
            hajm: 0.0,
            alam: 0,
            tajawuz: TaaribTaqreerTajawuz::default(),
        };
        // SAFETY: `talab` and `makhzan` are live locals of exactly the declared
        // types, aligned and valid for the whole call. `talab.nass` points into
        // `nass`, which outlives it, and its span pointers are null with a count
        // of zero. `makhzan`'s two arrays are this thread's own vectors, whose
        // capacities are their lengths, so the counts handed over are writable.
        // The context and chain were published by whoever installed the takeover
        // and are the only party that can destroy them.
        let ramz =
            unsafe { taarib_takhtit(hala.maqbad_siyaq(), &raw const talab, &raw mut makhzan) };
        if ramz == TAARIB_NAJAH {
            return Some(NatijatTakhtit {
                adad_huruf: makhzan.adad_huruf.min(huruf.len()),
                adad_sutur: makhzan.adad_sutur.min(sutur.len()),
                ard: if makhzan.ard.is_finite() {
                    makhzan.ard
                } else {
                    0.0
                },
            });
        }
        if ramz != TAARIB_SIAT_QASIRA
            || makhzan.adad_huruf > AQSA_HURUF
            || makhzan.adad_sutur > AQSA_SUTUR
        {
            return None;
        }
        if makhzan.adad_huruf > huruf.len() {
            huruf.resize(makhzan.adad_huruf, HARF_SIFRI);
        }
        if makhzan.adad_sutur > sutur.len() {
            sutur.resize(makhzan.adad_sutur, SATR_SIFRI);
        }
    }
    None
}

/// One string's worth of drawing, gathered so the emitter takes one argument.
struct TalabRasm<'a> {
    /// The policy.
    hala: &'a HalatIstila,
    /// The canvas item the quads are appended to.
    band: Hawiya,
    /// The modulate colour the engine asked for.
    lawn: Lawn,
    /// The engine's pen: baseline, leading edge.
    qalam: Muttajih2,
    /// `Font::draw`'s clip width, when it gave one.
    qass: Option<f32>,
    /// The laid-out glyphs, in visual order.
    huruf: &'a [TaaribHarf],
    /// The laid-out lines.
    sutur: &'a [TaaribSatr],
}

/// Emits a laid-out string as textured quads, and says whether it drew it.
///
/// Returns `false` only when nothing was drawn *and* something should have been
/// — every glyph landed on an atlas page with no registered texture. That
/// distinction is what lets the caller forward to the engine's own drawing
/// instead: unjoined text is bad and no text at all is worse, so the one case
/// this module will not produce is an empty box where a sentence was.
///
/// A partial failure — some pages registered, some not — draws what it can,
/// returns `true`, and warns once. Forwarding after half a string has already
/// been emitted would draw the other half on top of it.
fn irsim(talab: &TalabRasm<'_>) -> bool {
    let hala = talab.hala;
    let asas = talab.sutur.first().map_or(0.0, |satr| satr.asas);
    let rubi = hajm_rubi(hala.hajm);
    let siyaq = hala.maqbad_siyaq();
    let lawha = hala.maqbad_lawha();
    let silsila = hala.maqbad_silsila();
    let hadha: *mut c_void = core::ptr::with_exposed_provenance_mut(hala.wasl.hadha);
    // SAFETY: `hala.wasl` was built by `WaslRasm::jadeed` or
    // `WaslRasm::min_jadwal`, both of which refuse null, and its contract is
    // that the address is `VisualServer::canvas_item_add_texture_rect_region`
    // resolved for this build — which has exactly `DallaMustatil`'s signature.
    // The singleton outlives the takeover, which is removed before this library
    // unloads.
    let dalla: DallaMustatil = unsafe { core::mem::transmute(hala.wasl.dalla) };

    let mut rusim = 0usize;
    let mut mafqud = 0usize;
    for harf in talab.huruf {
        let miftah = TaaribMiftahShakl {
            muarrif: harf.muarrif,
            hajm_rubi: rubi,
            khatt: harf.khatt,
            bakat: BAKAT_MUTHABBAT,
        };
        let mut shakl = TaaribMawdiShakl::default();
        // SAFETY: `miftah` and `shakl` are live locals of exactly the declared
        // types, aligned and valid for the call; the three handles were
        // published at install and belong to one context.
        let ramz =
            unsafe { taarib_lawha_shakl(siyaq, lawha, silsila, &raw const miftah, &raw mut shakl) };
        if ramz != TAARIB_NAJAH || shakl.ard == 0 || shakl.irtifa == 0 {
            // A space has no image and neither has a glyph the atlas refused;
            // both advance the pen and draw nothing, which is already true of
            // the positions `jisr` produced.
            continue;
        }
        let wijha = mustatil_shakl(mawdi_harf(talab.qalam, asas, harf), &shakl);
        if let Some(hadd) = talab.qass
            && wijha.mawdi.s + wijha.hajm.s > talab.qalam.s + hadd
        {
            // Godot's own `Font::draw` drops whole characters at the clip width
            // rather than scissoring them, so this drops whole glyphs. Matching
            // the engine matters more here than being cleverer than it.
            continue;
        }
        let Some(nasij) = hawiyat_safha(usize::from(shakl.safha)) else {
            mafqud += 1;
            continue;
        };
        let masdar = mustatil_masdar(&shakl);
        // SAFETY: `dalla` has the signature justified where it was transmuted,
        // `hadha` is the singleton it is a member of, the two rectangles and the
        // colour are live locals valid for the call, and the two identifiers are
        // by-value handles the engine itself produced. `p_normal_map` is the null
        // RID a default-constructed one is.
        unsafe {
            dalla(
                hadha,
                talab.band,
                &raw const wijha,
                nasij,
                &raw const masdar,
                &raw const talab.lawn,
                false,
                Hawiya::batila(),
                // Clamp sampling to the source rectangle: two glyphs are
                // neighbours in one atlas page, and without the clamp a bilinear
                // sample at the edge of one picks up the other.
                true,
            );
        }
        rusim += 1;
    }

    if mafqud > 0 && SAFAHAT_MAFQUDA.fetch_add(mafqud, Ordering::Relaxed) == 0 {
        tracing::warn!(
            adad = mafqud,
            "glyphs landed on an atlas page with no registered engine texture; call \
             sajjil_safha for every page after uploading it"
        );
    }
    rusim > 0 || mafqud == 0
}

/// Measures a string through `jisr` and returns its width.
///
/// [`None`] when the context is not ready, when the request is refused, or when
/// the width is not finite — each of which leaves the engine's own measurement
/// in place, which is wrong for Arabic but is at least the number the rest of
/// the game was built around.
fn qis(hala: &HalatIstila, nass: &str) -> Option<f32> {
    if nass.is_empty() {
        return None;
    }
    let talab = TaaribTalab {
        nass: nass.as_ptr(),
        tul_nass: nass.len(),
        nitaqat: core::ptr::null(),
        adad_nitaqat: 0,
        silsila: hala.maqbad_silsila(),
        hajm: hala.hajm,
        ard_mutah: 0.0,
        irtifa_mutah: 0.0,
        khiyarat: hala.khiyarat(),
    };
    let mut khuruj = TaaribQiyasNass::default();
    // SAFETY: `talab` and `khuruj` are live locals of exactly the declared
    // types, aligned and valid for the call; `talab.nass` points into `nass`,
    // which outlives it, and its two span pointers are null with a count of
    // zero. The context and chain were published at install.
    let ramz = unsafe { taarib_qiyas(hala.maqbad_siyaq(), &raw const talab, &raw mut khuruj) };
    if ramz != TAARIB_NAJAH || !khuruj.ard.is_finite() {
        return None;
    }
    Some(khuruj.ard)
}

// ---------------------------------------------------------------------------
// Signatures
// ---------------------------------------------------------------------------

/// `Font::draw` as Godot 3 declares it.
///
/// ```text
/// void Font::draw(RID p_canvas_item, const Point2 &p_pos, const String &p_text,
///                 const Color &p_modulate, int p_clip_w,
///                 const Color &p_outline_modulate) const;
/// ```
///
/// A `const T &` parameter is a pointer at the ABI, which is why the position,
/// the string and the two colours are pointers here and not values; `this` is
/// the leading parameter under both supported conventions, which is what an
/// `extern "C"` function pointer with a leading raw pointer compiles to.
///
/// The trailing two parameters were added during 3.x. Passing arguments a target
/// does not read is harmless under both conventions this ships for — the callee
/// simply never looks at those registers — so one declaration serves the older
/// builds as well. Reading them is the part that is not free, and
/// [`hadd_qass`] treats a clip width outside a sane range as no clip for exactly
/// that reason.
pub type DallaRasm = unsafe extern "C" fn(
    hadha: *mut c_void,
    band: Hawiya,
    mawdi: *const Muttajih2,
    nass: *const c_void,
    lawn: *const Lawn,
    qass: c_int,
    lawn_hashiya: *const Lawn,
);

/// `Font::draw_char` as Godot 3 declares it.
///
/// ```text
/// float Font::draw_char(RID p_canvas_item, const Point2 &p_pos, CharType p_char,
///                       CharType p_next, const Color &p_modulate,
///                       bool p_outline) const;
/// ```
///
/// `CharType` is `wchar_t`, which is two bytes on Windows and four elsewhere;
/// either way an integer argument narrower than a register arrives zero-extended
/// in one, so both are declared `u32` here and narrowed by
/// [`harf_min_wahda`] according to [`HalatIstila::ard_wahda`]. Declaring the
/// parameter at its native width instead would be declaring two functions to
/// describe one calling convention.
pub type DallaRasmHarf = unsafe extern "C" fn(
    hadha: *mut c_void,
    band: Hawiya,
    mawdi: *const Muttajih2,
    harf: u32,
    tali: u32,
    lawn: *const Lawn,
    hashiya: bool,
) -> f32;

/// `Font::get_string_size` as Godot 3 declares it.
///
/// ```text
/// Size2 Font::get_string_size(const String &p_string) const;
/// ```
///
/// `Size2` is [`Muttajih2`], eight bytes and trivially copyable, which the two
/// conventions return by two entirely different mechanisms — an integer register
/// on Windows, a floating-point register pair under System V. Declaring the
/// return as a `#[repr(C)]` struct is what makes the compiler reproduce whichever
/// of those the engine's own compiler used, rather than this file having to know.
pub type DallaQiyas = unsafe extern "C" fn(hadha: *mut c_void, nass: *const c_void) -> Muttajih2;

/// `VisualServer::canvas_item_add_texture_rect_region`, the one call this
/// takeover ends in.
///
/// ```text
/// void canvas_item_add_texture_rect_region(RID p_item, const Rect2 &p_rect,
///     RID p_texture, const Rect2 &p_src_rect, const Color &p_modulate,
///     bool p_transpose, RID p_normal_map, bool p_clip_uv);
/// ```
///
/// The two `RID` arguments are passed by value; see [`Hawiya`] for why that is a
/// register and not a hidden copy.
pub type DallaMustatil = unsafe extern "C" fn(
    hadha: *mut c_void,
    band: Hawiya,
    wijha: *const Mustatil,
    nasij: Hawiya,
    masdar: *const Mustatil,
    lawn: *const Lawn,
    naql: bool,
    khareeta: Hawiya,
    qass_uv: bool,
);

// ---------------------------------------------------------------------------
// Reading the engine's arguments
// ---------------------------------------------------------------------------

/// Reads a position argument, refusing null and non-finite coordinates.
///
/// A non-finite pen is not a position that draws badly — it is a quad whose
/// vertices are NaN, which every backend discards, so the string would vanish.
/// Refusing here forwards to the engine instead, which at least draws something.
///
/// # Safety
///
/// `mawdi` must be null or valid for an aligned read of one [`Muttajih2`].
const unsafe fn qiraat_mawdi(mawdi: *const Muttajih2) -> Option<Muttajih2> {
    if mawdi.is_null() {
        return None;
    }
    // SAFETY: `mawdi` is non-null and the caller guarantees it is valid for an
    // aligned read of one Muttajih2.
    let qalam = unsafe { mawdi.read() };
    if qalam.s.is_finite() && qalam.a.is_finite() {
        Some(qalam)
    } else {
        None
    }
}

/// Reads a modulate colour, substituting opaque white for null or non-finite.
///
/// White is the identity modulate, so substituting it draws the glyph in its own
/// colour rather than not at all.
///
/// # Safety
///
/// `lawn` must be null or valid for an aligned read of one [`Lawn`].
const unsafe fn qiraat_lawn(lawn: *const Lawn) -> Lawn {
    if lawn.is_null() {
        return Lawn::abyad();
    }
    // SAFETY: `lawn` is non-null and the caller guarantees it is valid for an
    // aligned read of one Lawn.
    let qeema = unsafe { lawn.read() };
    if qeema.salih() { qeema } else { Lawn::abyad() }
}

/// Turns `Font::draw`'s clip width into a limit, or [`None`] for no clip.
///
/// Godot passes a negative number for "do not clip". Anything above sixty-five
/// thousand pixels is also treated as no clip, which is both true and defensive:
/// no game clips at that width, and on a build predating the parameter the
/// register holds whatever the caller last left in it.
#[must_use]
pub fn hadd_qass(qass: c_int) -> Option<f32> {
    u16::try_from(qass).ok().map(f32::from)
}

// ---------------------------------------------------------------------------
// The detours
// ---------------------------------------------------------------------------

/// How many accumulated runs were closed without being drawn.
static JARA_MAFQUD: AtomicUsize = AtomicUsize::new(0);

/// Takes `Font::draw` over, or declines it.
///
/// Declining forwards to the engine's own drawing **while still holding the
/// re-entrancy guard**, which is deliberate. `Font::draw` walks the string
/// calling `Font::draw_char`, and those calls land on this module's other hook;
/// holding the guard across the forward is what makes them pass straight through
/// to the engine. Releasing it first would have the character hook re-claim a
/// string the whole-string hook had just decided not to claim, and draw it twice.
unsafe extern "C" fn khatf_rasm(
    hadha: *mut c_void,
    band: Hawiya,
    mawdi: *const Muttajih2,
    nass: *const c_void,
    lawn: *const Lawn,
    qass: c_int,
    lawn_hashiya: *const Lawn,
) {
    let asl = ASL_RASM.load(Ordering::Acquire);
    if asl == 0 {
        return;
    }
    // SAFETY: `asl` is the trampoline `retour` produced for this target, which
    // carries the target's own signature. It is published before the detour is
    // enabled and cleared only after the detour is disabled, so a non-zero value
    // here is a live trampoline for the whole time this detour can be entered.
    let dalla: DallaRasm = unsafe { core::mem::transmute(asl) };
    let hars = Hars::ihjiz();
    let istawla = hars.is_some()
        // SAFETY: every pointer forwarded is the one the engine passed to the
        // function this detour replaced, and each is read only as the type the
        // signature declares, for the duration of this call.
        && unsafe { istawla_rasm(band, mawdi, nass, lawn, qass) };
    if !istawla {
        // SAFETY: every argument is the engine's own, forwarded unchanged to the
        // code that was going to receive it.
        unsafe { dalla(hadha, band, mawdi, nass, lawn, qass, lawn_hashiya) };
    }
    drop(hars);
}

/// Takes `Font::draw_char` over, or declines it.
///
/// Returns the advance the caller's pen should take: zero while a run is
/// accumulating, the run's whole shaped width on the character that closes it,
/// and the engine's own answer for a character this module does not claim. See
/// [`Mujammi`] for why those are the right three numbers.
unsafe extern "C" fn khatf_rasm_harf(
    hadha: *mut c_void,
    band: Hawiya,
    mawdi: *const Muttajih2,
    harf: u32,
    tali: u32,
    lawn: *const Lawn,
    hashiya: bool,
) -> f32 {
    let asl = ASL_RASM_HARF.load(Ordering::Acquire);
    if asl == 0 {
        return 0.0;
    }
    // SAFETY: as `khatf_rasm` — the trampoline carries the target's signature
    // and is published before the detour is enabled.
    let dalla: DallaRasmHarf = unsafe { core::mem::transmute(asl) };
    let hars = Hars::ihjiz();
    let natija = if hars.is_some() {
        // SAFETY: `mawdi` and `lawn` are the engine's own pointers, read only as
        // the declared types for the duration of this call.
        unsafe { istawla_harf(band, mawdi, harf, tali, lawn, hashiya) }
    } else {
        None
    };
    let taqaddum = match natija {
        Some(taqaddum) => taqaddum,
        // SAFETY: every argument is the engine's own, forwarded unchanged.
        None => unsafe { dalla(hadha, band, mawdi, harf, tali, lawn, hashiya) },
    };
    drop(hars);
    taqaddum
}

/// Replaces the measured width, and keeps the engine's height.
///
/// The width is replaced outright rather than widened, which is the opposite of
/// what the Unreal adapter does to Slate's measurement and is right for the
/// opposite reason. There, the engine still drew, and a measurement was only
/// advice to a line breaker, so erring wide cost a loose line and erring narrow
/// cost a clipped sentence. Here Taarib draws, so the measurement is not advice:
/// it is a description of what is about to appear, and a game centring a string
/// on any other number puts the box somewhere the text is not.
///
/// The height stays the engine's own. Godot 3 returns the font's height there,
/// not the string's, and a game spacing its rows by that number would have every
/// row move if Taarib answered with the laid-out height of one particular line.
unsafe extern "C" fn khatf_qiyas(hadha: *mut c_void, nass: *const c_void) -> Muttajih2 {
    let asl = ASL_QIYAS.load(Ordering::Acquire);
    if asl == 0 {
        return Muttajih2::default();
    }
    // SAFETY: as `khatf_rasm`.
    let dalla: DallaQiyas = unsafe { core::mem::transmute(asl) };
    let hars = Hars::ihjiz();
    // SAFETY: both arguments are the engine's own, forwarded unchanged. The
    // original runs first so its height is available whatever this module then
    // decides about the width.
    let asli = unsafe { dalla(hadha, nass) };
    let natija = if hars.is_some() {
        // SAFETY: `nass` is the engine's own `String` reference.
        unsafe { qis_nass(nass) }
    } else {
        None
    };
    drop(hars);
    match natija {
        Some(ard) => Muttajih2 { s: ard, a: asli.a },
        None => asli,
    }
}

// ---------------------------------------------------------------------------
// What the detours actually do
// ---------------------------------------------------------------------------

/// Lays a whole intercepted string out and draws it, or declines.
///
/// Everything that can decline does so **before** the first quad is emitted: the
/// policy, the pen, the string, whether it is claimed, and the layout itself.
/// After that the emission is committed, because a string half drawn by Taarib
/// and half by the engine is worse than either alone.
///
/// # Safety
///
/// `mawdi`, `nass` and `lawn` must be the arguments Godot passed to `Font::draw`
/// — a `Point2`, a `String` and a `Color`, each valid for the call.
unsafe fn istawla_rasm(
    band: Hawiya,
    mawdi: *const Muttajih2,
    nass: *const c_void,
    lawn: *const Lawn,
    qass: c_int,
) -> bool {
    let hala = hala();
    if !hala.jahiza() {
        return false;
    }
    // SAFETY: delegated to this function's contract.
    let Some(qalam) = (unsafe { qiraat_mawdi(mawdi) }) else {
        return false;
    };
    // SAFETY: delegated to this function's contract.
    let lawn = unsafe { qiraat_lawn(lawn) };

    MUAADDAT
        .try_with(|khana| {
            let Ok(mut muaddat) = khana.try_borrow_mut() else {
                return false;
            };
            let Muaddat {
                nass: nass_utf8,
                huruf,
                sutur,
            } = &mut *muaddat;
            // SAFETY: `nass` is the engine's `String`, per this function's
            // contract, and `hala.ard_wahda` is the code-unit width the caller
            // stated for this build — the two together are exactly what
            // `nass_min_string` requires.
            if !unsafe { nass_min_string(nass, hala.ard_wahda, hala.aqsa_wahdat, nass_utf8) } {
                return false;
            }
            if !yahtaj(nass_utf8, hala.kull_nass) {
                return false;
            }
            let Some(natija) = khattit(&hala, nass_utf8, huruf, sutur) else {
                return false;
            };
            let (Some(huruf), Some(sutur)) = (
                huruf.get(..natija.adad_huruf),
                sutur.get(..natija.adad_sutur),
            ) else {
                return false;
            };
            irsim(&TalabRasm {
                hala: &hala,
                band,
                lawn,
                qalam,
                qass: hadd_qass(qass),
                huruf,
                sutur,
            })
        })
        .unwrap_or(false)
}

/// Accumulates one character into a run, and draws the run when it closes.
///
/// [`None`] means the engine should draw this character itself.
///
/// An outline pass is swallowed rather than drawn or forwarded. Taarib's atlas
/// holds the fill of each glyph and no outline variant of it, so drawing the run
/// twice — once in the outline colour and once in the fill — would put a solid
/// coloured copy of the text behind the text. Forwarding it instead would draw
/// the engine's own unjoined outline underneath the shaped fill, which is the
/// failure this module exists to remove, in a slightly paler colour. Swallowing
/// it loses the outline and keeps the letters, which is the only one of the three
/// a player would call correct.
///
/// # Safety
///
/// `mawdi` and `lawn` must be the arguments Godot passed to `Font::draw_char`.
unsafe fn istawla_harf(
    band: Hawiya,
    mawdi: *const Muttajih2,
    harf: u32,
    tali: u32,
    lawn: *const Lawn,
    hashiya: bool,
) -> Option<f32> {
    let hala = hala();
    if !hala.jahiza() {
        return None;
    }
    // SAFETY: delegated to this function's contract.
    let qalam = unsafe { qiraat_mawdi(mawdi) }?;
    // SAFETY: delegated to this function's contract.
    let lawn = unsafe { qiraat_lawn(lawn) };
    let harf = harf_min_wahda(harf, hala.ard_wahda)?;
    let tali = harf_min_wahda(tali, hala.ard_wahda);
    let yakhussuna = hala.kull_nass || arabi(harf);

    JARA.try_with(|khana| {
        let Ok(mut jara) = khana.try_borrow_mut() else {
            return None;
        };
        if jara.mufaal && !jara.yatabi(band.raqm(), lawn, hashiya) {
            sajjil_faqd(usdur(&hala, &mut jara));
        }
        if hashiya {
            return if yakhussuna { Some(0.0) } else { None };
        }
        if !jara.mufaal {
            if !yakhussuna {
                return None;
            }
            jara.iftah(band.raqm(), qalam, lawn, hashiya);
        }
        jara.nass.push(harf);
        // Godot passes zero as the "next character" for the last one in a
        // string, which is the end-of-run signal being in the ABI rather than
        // guessed at. The length bound is the second way a run ends, for a caller
        // that never passes zero at all.
        if tali.is_none() || jara.nass.len() >= hala.aqsa_wahdat {
            let ard = usdur(&hala, &mut jara);
            sajjil_faqd(ard);
            return Some(ard.unwrap_or(0.0));
        }
        Some(0.0)
    })
    .ok()
    .flatten()
}

/// Notes a run that was closed without being drawn, once.
///
/// The character-at-a-time path cannot un-accumulate: by the time a layout is
/// refused, the characters that would have been forwarded to the engine have
/// already been answered with a zero advance and are gone. So a refused run is a
/// run that does not appear, and that is worth exactly one warning — it is also
/// why [`HadafIstila::Rasm`] is the better of the two draw hooks when a build
/// offers both, since that one can still decline and forward.
fn sajjil_faqd(natija: Option<f32>) {
    if natija.is_none() && JARA_MAFQUD.fetch_add(1, Ordering::Relaxed) == 0 {
        tracing::warn!(
            "a run recovered from Font::draw_char could not be laid out and was dropped; \
             the characters had already been answered with a zero advance and could not be \
             handed back to the engine"
        );
    }
}

/// Shapes and draws whatever the accumulator holds, and closes it.
///
/// Returns the run's shaped width, which is what the closing `Font::draw_char`
/// hands back as its advance so the caller's pen ends where the text does.
fn usdur(hala: &HalatIstila, jara: &mut Mujammi) -> Option<f32> {
    if !jara.aghliq() {
        return None;
    }
    let jara: &Mujammi = jara;
    if jara.nass.is_empty() {
        return None;
    }
    let band = Hawiya::min_raqm(jara.band);
    MUAADDAT
        .try_with(|khana| {
            let Ok(mut muaddat) = khana.try_borrow_mut() else {
                return None;
            };
            let Muaddat { huruf, sutur, .. } = &mut *muaddat;
            let natija = khattit(hala, &jara.nass, huruf, sutur)?;
            let (Some(huruf), Some(sutur)) = (
                huruf.get(..natija.adad_huruf),
                sutur.get(..natija.adad_sutur),
            ) else {
                return None;
            };
            let rusim = irsim(&TalabRasm {
                hala,
                band,
                lawn: jara.lawn,
                qalam: jara.mawdi,
                // The engine clips a character run by not calling `draw_char`
                // again, so there is no clip width to honour here.
                qass: None,
                huruf,
                sutur,
            });
            if rusim { Some(natija.ard) } else { None }
        })
        .ok()
        .flatten()
}

/// Measures an intercepted string, or declines.
///
/// # Safety
///
/// `nass` must be the `String` Godot passed to `Font::get_string_size`.
unsafe fn qis_nass(nass: *const c_void) -> Option<f32> {
    let hala = hala();
    if !hala.jahiza() {
        return None;
    }
    MUAADDAT
        .try_with(|khana| {
            let Ok(mut muaddat) = khana.try_borrow_mut() else {
                return None;
            };
            // SAFETY: `nass` is the engine's `String`, per this function's
            // contract, at the code-unit width the caller stated for this build.
            if !unsafe {
                nass_min_string(nass, hala.ard_wahda, hala.aqsa_wahdat, &mut muaddat.nass)
            } {
                return None;
            }
            if !yahtaj(&muaddat.nass, hala.kull_nass) {
                return None;
            }
            qis(&hala, &muaddat.nass)
        })
        .ok()
        .flatten()
}

// ---------------------------------------------------------------------------
// One installed hook
// ---------------------------------------------------------------------------

/// A detour, the bytes it replaced, and the target it replaced them at.
///
/// The recorded bytes are what makes uninstalling checkable rather than merely
/// attempted. `retour` restores the prologue itself; this records it
/// independently so that [`KhatfIstila::azil`] can say whether the restoration
/// actually produced the original bytes, instead of reporting success because no
/// error was returned.
pub struct KhatfIstila {
    hadaf: HadafIstila,
    unwan: Unwan,
    laqta: [u8; TUL_LAQTA],
    khatf: RawDetour,
    mufaal: bool,
}

impl fmt::Debug for KhatfIstila {
    fn fmt(&self, muharrir: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `khatf` is left out rather than forgotten: `RawDetour`'s own `Debug`
        // prints the trampoline's internal state, which says nothing about this
        // interception and a great deal about `retour`'s current internals.
        muharrir
            .debug_struct("KhatfIstila")
            .field("hadaf", &self.hadaf.ism())
            .field("unwan", &format!("{:#x}", self.unwan.raqm()))
            .field("laqta", &sittasi(&self.laqta))
            .field("mufaal", &self.mufaal)
            .finish_non_exhaustive()
    }
}

impl KhatfIstila {
    /// Which interception point this is.
    #[must_use]
    pub const fn hadaf(&self) -> HadafIstila {
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
    /// The order is not incidental: the prologue is recorded, the detour is
    /// constructed, the trampoline is published into its atomic, and only then is
    /// the detour enabled. Enabling before publishing would open a window in
    /// which the game calls the detour, reads a zero trampoline, and takes the
    /// do-nothing branch — which for a draw is one frame of missing text and for
    /// a measurement is a zero width, which is a widget that collapses.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::KhatfFashil`] naming the target, when `retour` refuses the
    /// prologue — an instruction that cannot be relocated, a target too close to
    /// the end of its section, or a page that could not be made writable.
    ///
    /// # Safety
    ///
    /// `unwan` must be the address of a function whose signature is exactly the
    /// one `khatf` declares, in a module that stays loaded for as long as this
    /// value lives. The first half comes from the caller resolving the address
    /// against the binary rather than guessing it, and the second from
    /// [`Istila::azil`] running before this library unloads.
    pub unsafe fn rakkib(
        hadaf: HadafIstila,
        unwan: Unwan,
        khatf: *const c_void,
        asl: &'static AtomicUsize,
    ) -> Result<Self, KhataGodot> {
        // SAFETY: `unwan` is a function address in mapped executable memory, per
        // this function's contract. Both supported linkers align and pad function
        // entry points to at least `TUL_LAQTA` bytes, so the read stays inside
        // the same executable section as the entry point.
        let laqta = unsafe { iqra_laqta(unwan) };

        // SAFETY: `unwan` names a function with `khatf`'s signature, per this
        // function's contract, and `khatf` is one of this module's own detours,
        // whose address is valid for the life of the loaded library. `retour`
        // decodes the prologue rather than assuming it, so an unrelocatable
        // prologue is the error below and not a corrupted instruction boundary.
        let detour =
            unsafe { RawDetour::new(unwan.muashir().cast(), khatf.cast()) }.map_err(|khata| {
                KhataGodot::KhatfFashil {
                    hadaf: hadaf.ism(),
                    tafsil: khata.to_string(),
                }
            })?;

        asl.store(
            core::ptr::from_ref(detour.trampoline()).expose_provenance(),
            Ordering::Release,
        );

        // SAFETY: the detour was constructed for this exact target and its
        // trampoline is published, so a call arriving the instant this returns
        // finds a complete interception. Enabling writes only within the
        // prologue `retour` measured.
        unsafe { detour.enable() }.map_err(|khata| {
            asl.store(0, Ordering::Release);
            KhataGodot::KhatfFashil {
                hadaf: hadaf.ism(),
                tafsil: khata.to_string(),
            }
        })?;

        Ok(Self {
            hadaf,
            unwan,
            laqta,
            khatf: detour,
            mufaal: true,
        })
    }

    /// Removes the detour and checks that the target's bytes came back.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::KhatfFashil`] naming the target when `retour` could not
    /// disable the detour, and — the check that makes the contract real — when
    /// disabling reported success but the prologue is not the one that was
    /// recorded. The second case is reported with both byte strings, because a
    /// process that is not byte-identical after an uninstall is something
    /// somebody has to be told about with evidence.
    pub fn azil(&mut self, asl: &'static AtomicUsize) -> Result<(), KhataGodot> {
        if !self.mufaal {
            return Ok(());
        }
        // SAFETY: `self.khatf` was constructed by `rakkib` for `self.unwan`,
        // which is still mapped because this value's contract requires it, and
        // disabling restores the prologue `retour` itself saved.
        let natija = unsafe { self.khatf.disable() };
        self.mufaal = false;
        asl.store(0, Ordering::Release);
        natija.map_err(|khata| KhataGodot::KhatfFashil {
            hadaf: self.hadaf.ism(),
            tafsil: khata.to_string(),
        })?;

        // SAFETY: as the read in `rakkib` — the same address, the same section,
        // the same length.
        let baad = unsafe { iqra_laqta(self.unwan) };
        if baad == self.laqta {
            return Ok(());
        }
        Err(KhataGodot::KhatfFashil {
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
    // `laqta` is a live local of exactly that length. The two cannot overlap: one
    // is a stack local of this frame and the other is a foreign module's
    // executable section.
    unsafe {
        core::ptr::copy_nonoverlapping(unwan.muashir().cast::<u8>(), laqta.as_mut_ptr(), TUL_LAQTA);
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
// The takeover
// ---------------------------------------------------------------------------

/// Every interception that was installed, and every one that was not.
///
/// Holding one of these means the process is patched. Dropping it means the
/// process is not.
#[derive(Debug)]
pub struct Istila {
    khutuf: Vec<(KhatfIstila, &'static AtomicUsize)>,
    tanbeehat: Vec<KhataGodot>,
    hala: HalatIstila,
}

impl Istila {
    /// Installs every interception whose address was supplied.
    ///
    /// The policy is published before the first hook is enabled, so no detour can
    /// run against an unpublished one. Publishing is idempotent: a second
    /// takeover in one process keeps the first policy and says so through a
    /// warning, because two policies with different font chains would be two
    /// answers to one question and the detours can only read one.
    ///
    /// A target that was not supplied, and a hook that would not install, are
    /// both recorded in [`Istila::tanbeehat`] and neither stops the others going
    /// in. The one arrangement this refuses to leave standing is a measurement
    /// hook with no draw hook behind it — see [`Istila::nuzul_matlub`].
    ///
    /// # Errors
    ///
    /// - [`KhataGodot::IsdarMajhul`] when the `String` code-unit width was never
    ///   stated. Two bytes and four bytes are two different buffers, and a
    ///   default of either would read half of every Windows string or one letter
    ///   of every Linux one.
    /// - [`KhataGodot::MuharrikGhayrMadum`] for a `float=64` build, whose
    ///   coordinates are twice the width this takeover reads. Declining is the
    ///   only honest answer: there is no reading of a `Vector2` that is right for
    ///   both widths.
    /// - [`KhataGodot::KhattMarfud`] when no font chain was supplied, because the
    ///   takeover would then intercept every string and have nothing to draw it
    ///   with.
    /// - [`KhataGodot::HajmMufrit`] when the per-string ceiling is above
    ///   [`AQSA_WAHDAT`]. The buffers it sizes are per-thread and live for the
    ///   process, so an unbounded ceiling is unbounded resident memory.
    /// - [`KhataGodot::RasmGhayrMutah`] when the context, the atlas, the canvas
    ///   binding or the pixel size is missing, since there is no takeover at all
    ///   without somewhere to draw and something to draw with.
    ///
    /// Every one of those is a reason to descend to the glyph transport rather
    /// than a reason to stop: none of them is fatal to the phase, and `naql`
    /// needs no address, no canvas and no coordinate.
    pub fn rakkib(ahdaf: &AhdafIstila, hala: HalatIstila) -> Result<Self, KhataGodot> {
        tahaqquq(&hala)?;

        let mut tanbeehat = Vec::new();
        if HALA.set(hala).is_err() {
            tanbeehat.push(KhataGodot::KhatfFashil {
                hadaf: "the takeover policy",
                tafsil: "a takeover was already installed in this process, so its policy was \
                         kept; the second policy was discarded rather than letting two answers \
                         to one question reach the detours"
                    .to_owned(),
            });
        }

        let mut khutuf: Vec<(KhatfIstila, &'static AtomicUsize)> = Vec::new();
        {
            let mut daa = |hadaf: HadafIstila,
                           unwan: Option<Unwan>,
                           khatf: *const c_void,
                           asl: &'static AtomicUsize| {
                let Some(unwan) = unwan else {
                    tanbeehat.push(KhataGodot::KhatfFashil {
                        hadaf: hadaf.ism(),
                        tafsil: hadaf.tafsil_ghiyab().to_owned(),
                    });
                    return;
                };
                // SAFETY: `unwan` was resolved against this binary by the caller
                // and names a function with the signature of the detour paired
                // with it here; the pairing is fixed in this function and cannot
                // be supplied from outside. The module it lives in stays loaded
                // because `azil` runs before this library unloads.
                match unsafe { KhatfIstila::rakkib(hadaf, unwan, khatf, asl) } {
                    Ok(khatf) => khutuf.push((khatf, asl)),
                    Err(khata) => tanbeehat.push(khata),
                }
            };

            // Each detour is bound to its declared signature before its address
            // is taken, so the pairing of target and detour is checked by the
            // compiler here rather than trusted at the transmute in the body.
            let dalla_rasm: DallaRasm = khatf_rasm;
            let dalla_rasm_harf: DallaRasmHarf = khatf_rasm_harf;
            let dalla_qiyas: DallaQiyas = khatf_qiyas;

            daa(
                HadafIstila::Rasm,
                ahdaf.rasm,
                dalla_rasm as *const c_void,
                &ASL_RASM,
            );
            daa(
                HadafIstila::RasmHarf,
                ahdaf.rasm_harf,
                dalla_rasm_harf as *const c_void,
                &ASL_RASM_HARF,
            );
            daa(
                HadafIstila::Qiyas,
                ahdaf.qiyas,
                dalla_qiyas as *const c_void,
                &ASL_QIYAS,
            );
        }

        let mut istila = Self {
            khutuf,
            tanbeehat,
            hala,
        };
        istila.azil_qiyas_yateem();

        for tanbeeh in &istila.tanbeehat {
            tracing::warn!(khata = %tanbeeh, "a Godot 3 interception was not installed");
        }
        for (khatf, _) in &istila.khutuf {
            tracing::info!(
                hadaf = khatf.hadaf().ism(),
                unwan = format!("{:#x}", khatf.unwan().raqm()),
                "a Godot 3 interception is installed"
            );
        }
        Ok(istila)
    }

    /// Removes a measurement hook that has no draw hook behind it.
    ///
    /// A measurement hook alone is not a partial takeover — it is a regression.
    /// It reports the width of text as Taarib would shape it while the engine
    /// goes on drawing the unshaped string, so every centred label, every
    /// right-aligned column and every auto-sized panel is positioned for text
    /// that is not what appears. Unhooked, the game is at least self-consistent.
    fn azil_qiyas_yateem(&mut self) {
        let yarsim = self
            .khutuf
            .iter()
            .any(|(khatf, _)| matches!(khatf.hadaf(), HadafIstila::Rasm | HadafIstila::RasmHarf));
        if yarsim {
            return;
        }
        let mut mabqi = Vec::new();
        let mut wujid = false;
        for (mut khatf, asl) in core::mem::take(&mut self.khutuf) {
            if khatf.hadaf() == HadafIstila::Qiyas {
                wujid = true;
                if let Err(khata) = khatf.azil(asl) {
                    self.tanbeehat.push(khata);
                }
            } else {
                mabqi.push((khatf, asl));
            }
        }
        self.khutuf = mabqi;
        if wujid {
            self.tanbeehat.push(KhataGodot::KhatfFashil {
                hadaf: HadafIstila::Qiyas.ism(),
                tafsil: "neither draw entry point could be hooked, so the measurement hook was \
                         removed again: reporting shaped widths for text the engine still draws \
                         unshaped positions every label for a string that never appears"
                    .to_owned(),
            });
        }
    }

    /// How many interceptions are installed.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.khutuf.len()
    }

    /// Whether any interception is installed.
    #[must_use]
    pub const fn mufaal(&self) -> bool {
        !self.khutuf.is_empty()
    }

    /// Whether the caller should descend to the glyph transport.
    ///
    /// True when no draw entry point was taken over, which is the only question
    /// that matters at this boundary: without one, every Arabic string in the
    /// game is still drawn by an engine that cannot shape it, and `naql` — which
    /// needs no hook, no canvas and no address — is the remaining path. Asking
    /// this is meant to replace reconstructing the same answer from
    /// [`Istila::tanbeehat`], which every caller would otherwise do slightly
    /// differently.
    #[must_use]
    pub fn nuzul_matlub(&self) -> bool {
        !self
            .khutuf
            .iter()
            .any(|(khatf, _)| matches!(khatf.hadaf(), HadafIstila::Rasm | HadafIstila::RasmHarf))
    }

    /// The policy this takeover was installed with.
    #[must_use]
    pub const fn hala(&self) -> HalatIstila {
        self.hala
    }

    /// The interceptions, for diagnostics.
    pub fn khutuf(&self) -> impl Iterator<Item = &KhatfIstila> {
        self.khutuf.iter().map(|(khatf, _)| khatf)
    }

    /// Every interception that could not be installed, with its named target.
    #[must_use]
    pub fn tanbeehat(&self) -> &[KhataGodot] {
        &self.tanbeehat
    }

    /// Opens a frame: draws any run left accumulating, then unpins the atlas.
    ///
    /// Called once per frame by whoever drives the adapter, before the engine
    /// draws anything. The order is the point. A run recovered from
    /// `Font::draw_char` is normally closed by the engine itself, and one that is
    /// still open at a frame boundary belongs to the frame that just ended; it is
    /// drawn first, and only then does the atlas release the previous frame's
    /// pins, so its glyphs are still where it left them.
    ///
    /// Only the calling thread's pending run is flushed, because that is the only
    /// one that exists: the accumulator is thread-local, and Godot 3 draws from
    /// one thread.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::RasmGhayrMutah`] when the atlas refuses to open a frame,
    /// naming the code it returned. Drawing continues to be attempted after one —
    /// an atlas that will not unpin still serves the glyphs it already holds —
    /// but the caller is told, because the next thing that happens is the atlas
    /// filling up.
    pub fn ibda_itar(&self) -> Result<(), KhataGodot> {
        let hars = Hars::ihjiz();
        if hars.is_some() {
            let _ = JARA.try_with(|khana| {
                if let Ok(mut jara) = khana.try_borrow_mut()
                    && jara.mufaal
                {
                    sajjil_faqd(usdur(&self.hala, &mut jara));
                }
            });
        }
        drop(hars);

        // SAFETY: both handles were supplied by the caller at install, validated
        // as non-zero by `tahaqquq`, and belong to one `jisr` context that the
        // installer owns for the life of the takeover.
        let ramz =
            unsafe { taarib_lawha_ibda_itar(self.hala.maqbad_siyaq(), self.hala.maqbad_lawha()) };
        if ramz == TAARIB_NAJAH {
            return Ok(());
        }
        Err(KhataGodot::RasmGhayrMutah {
            sabab: format!("the glyph atlas refused to open a frame, returning {ramz}"),
        })
    }

    /// Removes every interception and returns whatever refused to come out.
    ///
    /// An empty result is the contract being met: every prologue restored, every
    /// trampoline cleared, and the process byte-identical to the one this module
    /// entered. A non-empty result names each target that is not.
    ///
    /// Removal runs in reverse installation order, which is not decorative: the
    /// draw hooks are the ones a rendering thread is most likely to be inside, so
    /// the measurement hook stops being entered first.
    pub fn azil(&mut self) -> Vec<KhataGodot> {
        let mut tanbeehat = Vec::new();
        while let Some((mut khatf, asl)) = self.khutuf.pop() {
            match khatf.azil(asl) {
                Ok(()) => tracing::info!(
                    hadaf = khatf.hadaf().ism(),
                    "a Godot 3 interception was removed and its target restored"
                ),
                Err(khata) => {
                    tracing::warn!(khata = %khata, "an interception did not come out cleanly");
                    tanbeehat.push(khata);
                },
            }
        }
        tanbeehat
    }
}

impl Drop for Istila {
    /// Removes every interception if the caller did not.
    ///
    /// `taarib-haqn`'s rule is that a resource without a drop path does not ship,
    /// and a set of live detours in a foreign process is the sharpest version of
    /// that resource: dropping this value without removing them would leave the
    /// game jumping into a library that is no longer mapped, on the next string
    /// it draws. [`Istila::azil`] is still the way to call it, because it returns
    /// what failed and this cannot.
    fn drop(&mut self) {
        for tanbeeh in self.azil() {
            tracing::warn!(khata = %tanbeeh, "an interception was still installed at drop");
        }
    }
}

/// Refuses a policy that cannot produce correct drawing, before anything is
/// patched.
///
/// Every check here is one that would otherwise fail silently and per string
/// rather than loudly and once. See [`Istila::rakkib`] for what each one means.
fn tahaqquq(hala: &HalatIstila) -> Result<(), KhataGodot> {
    if hala.ard_wahda.bayt().is_none() {
        return Err(KhataGodot::IsdarMajhul {
            sabab: "the width of Godot's CharType was not stated, and it is two bytes on \
                    Windows and four elsewhere; a String read at the wrong width is a \
                    different buffer, not a mis-decoded one"
                .to_owned(),
        });
    }
    if hala.diqqa == DiqqatHaqiqi::Mudaafa {
        return Err(KhataGodot::MuharrikGhayrMadum {
            wujid: "3.x built with float=64, whose real_t is double".to_owned(),
        });
    }
    if hala.silsila == 0 {
        return Err(KhataGodot::KhattMarfud {
            sabab: "no jisr font chain was supplied, so the takeover would intercept every \
                    string and have no font to draw it with"
                .to_owned(),
        });
    }
    if hala.aqsa_wahdat == 0 || hala.aqsa_wahdat > AQSA_WAHDAT {
        return Err(KhataGodot::HajmMufrit {
            haql: "the per-string code-unit ceiling",
            qeema: u64::try_from(hala.aqsa_wahdat).unwrap_or(u64::MAX),
            saqf: u64::try_from(AQSA_WAHDAT).unwrap_or(u64::MAX),
        });
    }
    if !hala.jahiza() {
        return Err(KhataGodot::RasmGhayrMutah {
            sabab: "the jisr context, the glyph atlas, the VisualServer binding or the pixel \
                    size was missing, and a takeover without all four has somewhere to \
                    intercept text and nowhere to put it"
                .to_owned(),
        });
    }
    Ok(())
}
