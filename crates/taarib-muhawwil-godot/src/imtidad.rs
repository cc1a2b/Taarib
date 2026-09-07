//! الامتداد — the one exported C function Godot 4 calls, and everything the
//! adapter reaches the engine through.
//!
//! A `GDExtension` is a shared library and a `.gdextension` manifest naming one
//! symbol in it. Godot loads the library, resolves that symbol, and calls it
//! with three things: a way to reach the rest of the engine, an opaque handle
//! identifying this library, and a structure to fill in with the initialisation
//! level this extension wants and the two callbacks to run at it. Everything
//! else — every class, every method, every string — is reached through the
//! first of those three.
//!
//! ## The 4.0-against-4.1 cliff
//!
//! Godot 4.0 passed `const GDExtensionInterface *`: a struct of some two hundred
//! function pointers in a fixed order, so a member was reached by its offset and
//! adding a function anywhere in the middle broke every extension ever built.
//! Godot 4.1 replaced it with `GDExtensionInterfaceGetProcAddress`, a single
//! function that returns a function pointer for a name, and the interface became
//! extensible: an extension asks for what it uses, by name, and a build that
//! gained a function in the middle breaks nothing.
//!
//! The signature of the entry point did not change. The *first parameter* went
//! from a data pointer to a function pointer, silently, and an extension that
//! guesses wrong calls an address that is not a function, or reads two hundred
//! bytes of machine code as a table of pointers and calls one of them. Either
//! way the symptom is a crash at load with the patch's name on it.
//!
//! ### How this file decides which one it was handed
//!
//! By reading the first two 32-bit words at the address, exactly as `godot-cpp`
//! does, and asking whether they are `4` and `0`:
//!
//! - In the 4.0 case the address is the interface struct, whose first three
//!   members are `uint32_t version_major`, `uint32_t version_minor`,
//!   `uint32_t version_patch`. On any Godot 4.0.x those first two words read
//!   `4` and `0`.
//! - In the 4.1-and-later case the address is the entry point of
//!   `get_proc_address`. Its first eight bytes are that function's opening
//!   instructions. The pair `00000004 00000000` is not a plausible function
//!   prologue on any architecture this product targets: on x86-64 it decodes as
//!   `add [rax], al` twice over, and on `AArch64` the first word is a reserved
//!   encoding that traps. A compiler does not emit either at the top of a
//!   function.
//!
//! [`nawa_wasila`] is that test, and it is the only place in the crate that
//! reads through a pointer whose kind is not yet known. The read is eight bytes
//! at an address the engine just handed over, which is either the start of a
//! live static struct or the start of a mapped executable page — readable in
//! both cases on every platform Godot ships a 4.x editor and export template
//! for.
//!
//! Taarib's `.gdextension` sets `compatibility_minimum = 4.1`, which makes any
//! 4.1-or-later engine refuse to load a library it is too old for. That does not
//! remove the need for the test: `compatibility_minimum` was itself introduced
//! in 4.1, so a 4.0 runtime does not read the field and loads the library
//! anyway. The test is what turns that into a named refusal instead of a crash.
//!
//! On the 4.0 branch this extension **declines**. It reports the exact version
//! it found and stops, because the 4.0 interface is reached by offset into a
//! struct whose member order this build does not carry, and an offset guessed
//! from memory would call an unrelated engine function with the arguments of a
//! different one. That is the same refusal `slate.rs` makes about byte patterns
//! and `wasl.rs` makes about vtable slots, for the same reason: a mechanism that
//! cannot be verified is worse than a mechanism that is absent, because the
//! absent one is honest about it.
//!
//! ## Why the scene level, and not core, and not editor
//!
//! [`MustawaTahyia::Sina`] — `GDEXTENSION_INITIALIZATION_SCENE`.
//!
//! - **Core** runs while the engine is still assembling itself. `ClassDB` does
//!   not yet carry `FontFile`, `Theme` or `Control`; the singletons this adapter
//!   needs do not exist; and `ProjectSettings` has been read but nothing has
//!   acted on it. Constructing a `FontFile` there fails, and if it did not it
//!   would produce an object registered against a class database that is
//!   repopulated afterwards.
//! - **Servers** brings the rendering and text servers up but not the scene
//!   classes, so the theme database and the node types are still absent.
//! - **Scene** is where `ClassDB` is complete, `ThemeDB` has built the default
//!   theme, and `TranslationServer` exists — and it is still before the main
//!   scene is instantiated, so a font assigned here is the font the first frame
//!   draws with. That is exactly the window a font-and-locale patch needs: late
//!   enough that everything exists, early enough that nothing has been drawn.
//! - **Editor** runs only in the editor. A shipped game never reaches it, so an
//!   extension that registered there would work perfectly for whoever built the
//!   patch and do nothing at all for the player who installed it — the worst
//!   failure mode available, because it passes every test the author runs.
//!
//! ## Strings
//!
//! Godot 4 has no C string API. A `String` is an opaque handle constructed by an
//! interface call into a buffer the caller owns, and a `StringName` is a second,
//! interned kind that is not interchangeable with it — `classdb_get_method_bind`
//! takes `StringName` and `ProjectSettings::set_setting` takes `String`, and
//! passing one where the other belongs reads a different structure through the
//! same pointer.
//!
//! Both are reference-counted inside the engine and both must be destroyed, and
//! the destructor is itself obtained through the interface, per variant type,
//! from `variant_get_ptr_destructor`. [`NassGodot`] and [`IsmGodot`] pair the
//! construction with the destruction in [`Drop`], so that the pairing cannot be
//! got wrong by an early return. A `StringName` leaked in a player's game is a
//! permanent entry in the engine's intern table for the life of the process, and
//! this adapter constructs one per method bind and per theme entry — a leak
//! there is small, real, and entirely avoidable.
//!
//! ## Method hashes are not invented here
//!
//! `classdb_get_method_bind` takes a hash that identifies a method's exact
//! signature in the build being run, and the engine refuses a bind whose hash it
//! does not recognise. Those hashes come out of `extension_api.json` for a
//! specific engine version; there is no way to derive one, and a hash written
//! from memory would be a hash that fails to bind on every build. So
//! [`JadwalTuruq`] is **supplied by the caller**, which has the version's own
//! table, exactly as `wasl.rs` requires its vtable indices to be supplied rather
//! than assumed.

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::khadim_nusus::{
    IttijahNass, IttijahTakhtit, KhadimNusus, MadkhalSima, Musajjil, NatijatKhadim, QeemaIdad,
};
use crate::khata::KhataGodot;

// ---------------------------------------------------------------------------
// The interface's own types
// ---------------------------------------------------------------------------

/// `GDExtensionBool` — the engine's boolean at the ABI, one byte.
pub type MantiqGodot = u8;

/// `GDExtensionInt` — the engine's integer at the ABI, always 64-bit signed
/// whatever the target's `int` is.
pub type RaqmGodot = i64;

/// `GDExtensionObjectPtr` — an `Object *`.
pub type MaqbadKaain = *mut c_void;

/// `GDExtensionTypePtr` — a pointer to a built-in value's storage.
pub type MaqbadNaw = *mut c_void;

/// `GDExtensionVariantPtr` — a pointer to a `Variant`'s storage.
pub type MaqbadMutaghayyir = *mut c_void;

/// `GDExtensionMethodBindPtr` — a bound engine method.
pub type MaqbadTareeqa = *mut c_void;

/// `GDExtensionClassLibraryPtr` — the handle identifying this library.
pub type MaqbadMaktaba = *mut c_void;

/// `GDExtensionVariantType` — which built-in type a value is.
///
/// Only the three this adapter constructs are named. The numbering is the order
/// `Variant::Type` declares them in and has not changed across Godot 4.
pub type NawQeema = u32;

/// `Variant::STRING`.
pub const NAW_NASS: NawQeema = 4;

/// `Variant::STRING_NAME`.
pub const NAW_ISM: NawQeema = 21;

/// `Variant::PACKED_BYTE_ARRAY`.
pub const NAW_MASFUFAT_BAYT: NawQeema = 29;

/// The value a `PackedByteArray`'s default constructor sits at.
///
/// `variant_get_ptr_constructor` indexes a type's constructors in declaration
/// order and zero is the no-argument one for every built-in.
pub const BANI_IFTIRADI: i32 = 0;

/// `GDExtensionInterfaceGetProcAddress` — Godot 4.1's whole interface.
///
/// Takes a NUL-terminated ASCII name and returns the function, or null when the
/// engine does not have it. The returned pointer is untyped; giving it a type is
/// the transmute each resolver performs, and the name is the only guarantee that
/// the type is right — which is why every name in [`ASMA_DAWAL`] is spelled
/// exactly as `gdextension_interface.h` spells it.
pub type DallaJalbUnwan =
    unsafe extern "C" fn(ism: *const core::ffi::c_char) -> Option<DallaWasila>;

/// An interface function, before it is given a type.
pub type DallaWasila = unsafe extern "C" fn();

/// `GDExtensionInitializationLevel` — when a callback runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MustawaTahyia {
    /// Core: memory, `Object`, the variant system. Nothing this adapter needs.
    Asas,
    /// Servers: rendering, physics, and the text server.
    Khadamat,
    /// Scene: the full class database, the theme database, the node types.
    ///
    /// Where this extension registers. See the module header for why.
    Sina,
    /// Editor: never reached in a shipped game.
    Muharrir,
}

impl MustawaTahyia {
    /// The value the engine uses.
    #[must_use]
    pub const fn qeema(self) -> u32 {
        match self {
            Self::Asas => 0,
            Self::Khadamat => 1,
            Self::Sina => 2,
            Self::Muharrir => 3,
        }
    }

    /// The level for a value the engine passed to a callback.
    ///
    /// [`None`] for a value outside the four, which a newer engine could
    /// introduce. An unknown level is ignored rather than guessed at: a callback
    /// that ran at a level it did not recognise would be running at a moment
    /// nobody characterised.
    #[must_use]
    pub const fn min_qeema(qeema: u32) -> Option<Self> {
        match qeema {
            0 => Some(Self::Asas),
            1 => Some(Self::Khadamat),
            2 => Some(Self::Sina),
            3 => Some(Self::Muharrir),
            _ => None,
        }
    }

    /// A stable short name for logs.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Asas => "core",
            Self::Khadamat => "servers",
            Self::Sina => "scene",
            Self::Muharrir => "editor",
        }
    }
}

/// `GDExtensionInitialization` — what the entry point fills in.
///
/// ```text
/// TahyiatImtidad — 32 bytes on a 64-bit target, 8-byte aligned
///
///   offset  size  field    meaning
///        0     4  mustawa  the lowest level the callbacks run at
///        8     8  bayanat  passed back to both callbacks untouched
///       16     8  tahyia   run once per level from `mustawa` upwards
///       24     8  inha     run once per level from the top back down
/// ```
///
/// The engine allocates it and passes a pointer; the entry point writes all four
/// fields. Leaving `tahyia` null is legal and means "nothing to do at any
/// level", which is what this extension writes when it has decided to decline.
#[repr(C)]
#[derive(Debug)]
pub struct TahyiatImtidad {
    /// The lowest initialisation level the callbacks are invoked at.
    pub mustawa: u32,
    /// Opaque, handed back to both callbacks. Null here: this extension keeps
    /// its state in one process-wide [`OnceLock`], because the engine calls the
    /// callbacks from the main thread and there is exactly one library.
    pub bayanat: *mut c_void,
    /// Called once for each level from `mustawa` up to editor.
    pub tahyia: Option<unsafe extern "C" fn(bayanat: *mut c_void, mustawa: u32)>,
    /// Called once for each level on the way back down.
    pub inha: Option<unsafe extern "C" fn(bayanat: *mut c_void, mustawa: u32)>,
}

/// `GDExtensionGodotVersion` — what `get_godot_version` fills in.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IsdarGodot {
    /// Major, always 4 on any engine that can load this.
    pub kabir: u32,
    /// Minor.
    pub sagheer: u32,
    /// Patch.
    pub tasheeh: u32,
    /// The full version as a NUL-terminated C string the engine owns.
    pub nass: *const core::ffi::c_char,
}

impl Default for IsdarGodot {
    fn default() -> Self {
        Self {
            kabir: 0,
            sagheer: 0,
            tasheeh: 0,
            nass: core::ptr::null(),
        }
    }
}

impl IsdarGodot {
    /// The version as `major.minor.patch`, without touching the engine's string.
    ///
    /// The three integers are enough and the pointer is not read. Reading it
    /// would mean trusting a NUL to appear in memory this crate did not write,
    /// for a value that adds nothing to a version already in hand.
    #[must_use]
    pub fn wasf(&self) -> String {
        format!("{}.{}.{}", self.kabir, self.sagheer, self.tasheeh)
    }
}

// ---------------------------------------------------------------------------
// Which ABI the entry point was handed
// ---------------------------------------------------------------------------

/// The head of Godot 4.0's `GDExtensionInterface`.
///
/// Only the three integers, because only they are read. The rest of the 4.0
/// struct is two hundred function pointers whose order this build deliberately
/// does not carry — see the module header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RasWasilaQadima {
    kabir: u32,
    sagheer: u32,
    tasheeh: u32,
}

/// Which of the two first parameters the entry point received.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NawWasila {
    /// Godot 4.1 and later: a `get_proc_address` function.
    Jadida,
    /// Godot 4.0.x: a pointer to the fixed interface struct, carrying the
    /// version it reported.
    Qadima {
        /// Minor, always 0 on this branch.
        sagheer: u32,
        /// Patch.
        tasheeh: u32,
    },
}

/// Decides which ABI the first parameter uses.
///
/// The whole argument for this test is in the module header. In short: on 4.0
/// the address is a data pointer whose first two words are `4` and `0`; on 4.1
/// and later it is a code address, whose first eight bytes are a function
/// prologue and cannot be that pair.
///
/// # Safety
///
/// `khaam` must be the non-null first argument Godot passed to the entry point.
/// The guarantee is the engine's: it is either the address of a static
/// `GDExtensionInterface` that lives for the process, or the entry point of an
/// exported function in the engine's own image. Eight bytes are readable at the
/// start of either on every platform Godot 4 ships for.
#[must_use]
pub const unsafe fn nawa_wasila(khaam: *const c_void) -> NawWasila {
    // SAFETY: the caller guarantees `khaam` is one of the two engine-owned
    // addresses above. The read is of twelve bytes at offset zero, which is
    // within the 4.0 struct (whose first members are three `uint32_t`) and
    // within the first mapped page of a function body in the other case. The
    // read is unaligned-tolerant because `read_unaligned` is used: a function
    // entry point carries no alignment promise beyond the architecture's
    // instruction alignment, which on x86-64 is one byte.
    let ras = unsafe { khaam.cast::<RasWasilaQadima>().read_unaligned() };
    if ras.kabir == 4 && ras.sagheer == 0 {
        NawWasila::Qadima {
            sagheer: ras.sagheer,
            tasheeh: ras.tasheeh,
        }
    } else {
        NawWasila::Jadida
    }
}

// ---------------------------------------------------------------------------
// The interface functions this adapter resolves
// ---------------------------------------------------------------------------

/// `void get_godot_version(GDExtensionGodotVersion *)`.
type DallaIsdar = unsafe extern "C" fn(makhraj: *mut IsdarGodot);

/// `void string_new_with_utf8_chars(GDExtensionUninitializedStringPtr, const char *)`.
type DallaNassJadeed = unsafe extern "C" fn(makhraj: MaqbadNaw, nass: *const core::ffi::c_char);

/// `GDExtensionInt string_to_utf8_chars(GDExtensionConstStringPtr, char *, GDExtensionInt)`.
///
/// With a null buffer it returns the length it would have written, which is how
/// [`WaslGodot`] sizes its read without a second call convention.
type DallaNassIla = unsafe extern "C" fn(
    dhat: *const c_void,
    makhraj: *mut core::ffi::c_char,
    aqsa: RaqmGodot,
) -> RaqmGodot;

/// `void string_name_new_with_utf8_chars(GDExtensionUninitializedStringNamePtr, const char *)`.
type DallaIsmJadeed = unsafe extern "C" fn(makhraj: MaqbadNaw, nass: *const core::ffi::c_char);

/// `GDExtensionPtrDestructor variant_get_ptr_destructor(GDExtensionVariantType)`.
type DallaJalbHadm = unsafe extern "C" fn(naw: NawQeema) -> Option<DallaHadm>;

/// `void (*GDExtensionPtrDestructor)(GDExtensionTypePtr)`.
pub type DallaHadm = unsafe extern "C" fn(qaida: MaqbadNaw);

/// `GDExtensionPtrConstructor variant_get_ptr_constructor(GDExtensionVariantType, int32_t)`.
type DallaJalbBina = unsafe extern "C" fn(naw: NawQeema, fahras: i32) -> Option<DallaBina>;

/// `void (*GDExtensionPtrConstructor)(GDExtensionUninitializedTypePtr,
/// const GDExtensionConstTypePtr *)`.
pub type DallaBina = unsafe extern "C" fn(qaida: MaqbadNaw, muamalat: *const *const c_void);

/// `GDExtensionPtrBuiltInMethod variant_get_ptr_builtin_method(GDExtensionVariantType,
/// GDExtensionConstStringNamePtr, GDExtensionInt)`.
type DallaJalbTareeqaDakhiliya = unsafe extern "C" fn(
    naw: NawQeema,
    ism: *const c_void,
    basma: RaqmGodot,
) -> Option<DallaTareeqaDakhiliya>;

/// `void (*GDExtensionPtrBuiltInMethod)(GDExtensionTypePtr, const GDExtensionConstTypePtr *,
/// GDExtensionTypePtr, int)`.
pub type DallaTareeqaDakhiliya = unsafe extern "C" fn(
    qaida: MaqbadNaw,
    muamalat: *const *const c_void,
    makhraj: MaqbadNaw,
    adad: i32,
);

/// `GDExtensionVariantFromTypeConstructorFunc get_variant_from_type_constructor(
/// GDExtensionVariantType)`.
type DallaJalbTahweel = unsafe extern "C" fn(naw: NawQeema) -> Option<DallaTahweel>;

/// `void (*GDExtensionVariantFromTypeConstructorFunc)(GDExtensionUninitializedVariantPtr,
/// GDExtensionTypePtr)`.
pub type DallaTahweel = unsafe extern "C" fn(makhraj: MaqbadMutaghayyir, masdar: MaqbadNaw);

/// `void variant_stringify(GDExtensionConstVariantPtr, GDExtensionStringPtr)`.
type DallaSard = unsafe extern "C" fn(dhat: *const c_void, makhraj: MaqbadNaw);

/// `uint8_t *packed_byte_array_operator_index(GDExtensionTypePtr, GDExtensionInt)`.
type DallaFahrasBayt = unsafe extern "C" fn(dhat: MaqbadNaw, fahras: RaqmGodot) -> *mut u8;

/// `GDExtensionObjectPtr global_get_singleton(GDExtensionConstStringNamePtr)`.
type DallaMufrad = unsafe extern "C" fn(ism: *const c_void) -> MaqbadKaain;

/// `GDExtensionObjectPtr classdb_construct_object(GDExtensionConstStringNamePtr)`.
type DallaBinaKaain = unsafe extern "C" fn(sanf: *const c_void) -> MaqbadKaain;

/// `GDExtensionMethodBindPtr classdb_get_method_bind(GDExtensionConstStringNamePtr,
/// GDExtensionConstStringNamePtr, GDExtensionInt)`.
type DallaRabtTareeqa = unsafe extern "C" fn(
    sanf: *const c_void,
    ism: *const c_void,
    basma: RaqmGodot,
) -> MaqbadTareeqa;

/// `void object_method_bind_ptrcall(GDExtensionMethodBindPtr, GDExtensionObjectPtr,
/// const GDExtensionConstTypePtr *, GDExtensionTypePtr)`.
type DallaNidaMubashir = unsafe extern "C" fn(
    tareeqa: MaqbadTareeqa,
    kaain: MaqbadKaain,
    muamalat: *const *const c_void,
    makhraj: MaqbadNaw,
);

/// `void object_destroy(GDExtensionObjectPtr)`.
type DallaHadmKaain = unsafe extern "C" fn(kaain: MaqbadKaain);

/// The engine's spelling of `get_godot_version`.
pub const ISM_ISDAR: &str = "get_godot_version";
/// The engine's spelling of `string_new_with_utf8_chars`.
pub const ISM_NASS_JADEED: &str = "string_new_with_utf8_chars";
/// The engine's spelling of `string_to_utf8_chars`.
pub const ISM_NASS_ILA: &str = "string_to_utf8_chars";
/// The engine's spelling of `string_name_new_with_utf8_chars`.
pub const ISM_ISM_JADEED: &str = "string_name_new_with_utf8_chars";
/// The engine's spelling of `variant_get_ptr_destructor`.
pub const ISM_JALB_HADM: &str = "variant_get_ptr_destructor";
/// The engine's spelling of `variant_get_ptr_constructor`.
pub const ISM_JALB_BINA: &str = "variant_get_ptr_constructor";
/// The engine's spelling of `variant_get_ptr_builtin_method`.
pub const ISM_JALB_TAREEQA: &str = "variant_get_ptr_builtin_method";
/// The engine's spelling of `get_variant_from_type_constructor`.
pub const ISM_JALB_TAHWEEL: &str = "get_variant_from_type_constructor";
/// The engine's spelling of `variant_stringify`.
pub const ISM_SARD: &str = "variant_stringify";
/// The engine's spelling of `packed_byte_array_operator_index`.
pub const ISM_FAHRAS_BAYT: &str = "packed_byte_array_operator_index";
/// The engine's spelling of `global_get_singleton`.
pub const ISM_MUFRAD: &str = "global_get_singleton";
/// The engine's spelling of `classdb_construct_object`.
pub const ISM_BINA_KAAIN: &str = "classdb_construct_object";
/// The engine's spelling of `classdb_get_method_bind`.
pub const ISM_RABT_TAREEQA: &str = "classdb_get_method_bind";
/// The engine's spelling of `object_method_bind_ptrcall`.
pub const ISM_NIDA_MUBASHIR: &str = "object_method_bind_ptrcall";
/// The engine's spelling of `object_destroy`.
pub const ISM_HADM_KAAIN: &str = "object_destroy";
/// The engine's spelling of `variant_destroy`.
pub const ISM_HADM_MUTAGHAYYIR: &str = "variant_destroy";

/// Every interface function this adapter asks for, in resolution order.
///
/// Sixteen, and this list is the whole of what the Godot 4 path reaches the
/// engine through. Public because a refusal names the one that was missing: a
/// bug report carrying that name identifies the engine build far better than
/// "the extension did not load".
pub const ASMA_DAWAL: [&str; 16] = [
    ISM_ISDAR,
    ISM_NASS_JADEED,
    ISM_NASS_ILA,
    ISM_ISM_JADEED,
    ISM_JALB_HADM,
    ISM_JALB_BINA,
    ISM_JALB_TAREEQA,
    ISM_JALB_TAHWEEL,
    ISM_SARD,
    ISM_FAHRAS_BAYT,
    ISM_MUFRAD,
    ISM_BINA_KAAIN,
    ISM_RABT_TAREEQA,
    ISM_NIDA_MUBASHIR,
    ISM_HADM_KAAIN,
    ISM_HADM_MUTAGHAYYIR,
];

// ---------------------------------------------------------------------------
// The resolved interface
// ---------------------------------------------------------------------------

/// Every interface function, resolved once and held for the process.
///
/// Resolution is by name and happens exactly once, at load. Asking
/// `get_proc_address` for a name is a lookup in the engine's own table and is
/// not free; doing it per call, inside a game's frame, would be a cost paid
/// forever to re-derive an answer that cannot change.
///
/// Every field is a plain function pointer, which is [`Send`] and [`Sync`], so
/// this whole structure lives in a `OnceLock` with no `unsafe impl` and no lock
/// — which is what the workspace's ban on [`std::sync::Mutex`] is for.
#[derive(Debug, Clone, Copy)]
pub struct Wasila {
    nass_jadeed: DallaNassJadeed,
    nass_ila: DallaNassIla,
    ism_jadeed: DallaIsmJadeed,
    jalb_bina: DallaJalbBina,
    jalb_tareeqa: DallaJalbTareeqaDakhiliya,
    sard: DallaSard,
    fahras_bayt: DallaFahrasBayt,
    mufrad: DallaMufrad,
    bina_kaain: DallaBinaKaain,
    rabt_tareeqa: DallaRabtTareeqa,
    nida_mubashir: DallaNidaMubashir,
    hadm_kaain: DallaHadmKaain,
    /// The `String` destructor, fetched once from `variant_get_ptr_destructor`.
    hadm_nass: DallaHadm,
    /// The `StringName` destructor, likewise.
    hadm_ism: DallaHadm,
    /// The `PackedByteArray` destructor, likewise.
    hadm_masfufa: DallaHadm,
    /// The `Variant` destructor — `variant_destroy`, not a per-type one.
    ///
    /// A boxed value is not released by the destructor of the type inside it:
    /// `Variant` owns whatever it holds, and destroying the inner value through
    /// a pointer to the box would read the box's type tag as the value.
    hadm_mutaghayyir: DallaHadm,
    /// `Variant`-from-`String`, for the one setting write that needs a variant.
    tahweel_nass: DallaTahweel,
    /// `Variant`-from-`int`.
    tahweel_raqm: DallaTahweel,
    /// The engine's version, as three integers.
    kabir: u32,
    sagheer: u32,
    tasheeh: u32,
}

/// A NUL-terminated copy of a name or a string handed to the engine.
///
/// Returns [`None`] for text with an interior NUL, which every C-string consumer
/// on the other side would read as the end. For an interface function name the
/// consequence is a successfully resolved pointer to a *different* function; for
/// a project-setting key it is a shorter key that the engine will happily store
/// and nothing will ever read. Neither announces itself, so both are refused
/// here, once, rather than at each of the call sites.
fn ism_c(ism: &str) -> Option<Vec<u8>> {
    if ism.as_bytes().contains(&0) {
        return None;
    }
    let mut bayt = Vec::with_capacity(ism.len() + 1);
    bayt.extend_from_slice(ism.as_bytes());
    bayt.push(0);
    Some(bayt)
}

/// Asks the engine for one interface function by name.
///
/// # Errors
///
/// [`KhataGodot::ImtidadMarfud`] naming the function, which is the whole point:
/// the extension declines rather than half-registering, and the message says
/// which of the fifteen this engine did not answer to.
///
/// # Safety
///
/// `jalb` must be the `get_proc_address` function Godot passed to the entry
/// point on a 4.1-or-later engine, established by [`nawa_wasila`].
unsafe fn dalla_bism(jalb: DallaJalbUnwan, ism: &str) -> Result<DallaWasila, KhataGodot> {
    let Some(mawsum) = ism_c(ism) else {
        return Err(KhataGodot::ImtidadMarfud {
            sabab: format!("the interface function name \"{ism}\" contains an interior NUL"),
        });
    };
    // SAFETY: `jalb` is the engine's own `get_proc_address`, per this
    // function's contract. `mawsum` is a NUL-terminated buffer with no interior
    // NUL that outlives the call, and the engine only reads it. The engine
    // returns null for a name it does not have, which `Option` models exactly.
    let dalla = unsafe { jalb(mawsum.as_ptr().cast::<core::ffi::c_char>()) };
    dalla.ok_or_else(|| KhataGodot::ImtidadMarfud {
        sabab: format!(
            "this Godot build does not provide the GDExtension interface function \
             \"{ism}\", so the adapter declined rather than register half of itself"
        ),
    })
}

impl Wasila {
    /// Resolves every function in [`ASMA_DAWAL`], or declines naming the first
    /// one missing.
    ///
    /// Resolution is all-or-nothing. A partially bound extension is worse than
    /// an absent one: it registers a font and then cannot set a locale, so the
    /// game draws Arabic glyphs for English text, and no error anybody sees
    /// explains it.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] naming the function that did not resolve,
    /// or the variant type whose destructor or converter the engine refused.
    ///
    /// # Safety
    ///
    /// `jalb` must be the `get_proc_address` Godot passed to the entry point,
    /// on an engine [`nawa_wasila`] identified as [`NawWasila::Jadida`].
    pub unsafe fn ijlib(jalb: DallaJalbUnwan) -> Result<Self, KhataGodot> {
        // SAFETY: every one of these delegates verbatim to this function's own
        // contract. Each transmute reinterprets an untyped function pointer as
        // the signature `gdextension_interface.h` declares for the name it was
        // fetched by, and the name is the only thing that can make it wrong —
        // which is why the names are constants rather than literals at the
        // call site.
        let (isdar, nass_jadeed, nass_ila, ism_jadeed) = unsafe {
            (
                core::mem::transmute::<DallaWasila, DallaIsdar>(dalla_bism(jalb, ISM_ISDAR)?),
                core::mem::transmute::<DallaWasila, DallaNassJadeed>(dalla_bism(
                    jalb,
                    ISM_NASS_JADEED,
                )?),
                core::mem::transmute::<DallaWasila, DallaNassIla>(dalla_bism(jalb, ISM_NASS_ILA)?),
                core::mem::transmute::<DallaWasila, DallaIsmJadeed>(dalla_bism(
                    jalb,
                    ISM_ISM_JADEED,
                )?),
            )
        };

        // SAFETY: as above.
        let (jalb_hadm, jalb_bina, jalb_tareeqa, jalb_tahweel) = unsafe {
            (
                core::mem::transmute::<DallaWasila, DallaJalbHadm>(dalla_bism(
                    jalb,
                    ISM_JALB_HADM,
                )?),
                core::mem::transmute::<DallaWasila, DallaJalbBina>(dalla_bism(
                    jalb,
                    ISM_JALB_BINA,
                )?),
                core::mem::transmute::<DallaWasila, DallaJalbTareeqaDakhiliya>(dalla_bism(
                    jalb,
                    ISM_JALB_TAREEQA,
                )?),
                core::mem::transmute::<DallaWasila, DallaJalbTahweel>(dalla_bism(
                    jalb,
                    ISM_JALB_TAHWEEL,
                )?),
            )
        };

        // SAFETY: as above.
        let (sard, fahras_bayt, mufrad, bina_kaain) = unsafe {
            (
                core::mem::transmute::<DallaWasila, DallaSard>(dalla_bism(jalb, ISM_SARD)?),
                core::mem::transmute::<DallaWasila, DallaFahrasBayt>(dalla_bism(
                    jalb,
                    ISM_FAHRAS_BAYT,
                )?),
                core::mem::transmute::<DallaWasila, DallaMufrad>(dalla_bism(jalb, ISM_MUFRAD)?),
                core::mem::transmute::<DallaWasila, DallaBinaKaain>(dalla_bism(
                    jalb,
                    ISM_BINA_KAAIN,
                )?),
            )
        };

        // SAFETY: as above.
        let (rabt_tareeqa, nida_mubashir, hadm_kaain, hadm_mutaghayyir) = unsafe {
            (
                core::mem::transmute::<DallaWasila, DallaRabtTareeqa>(dalla_bism(
                    jalb,
                    ISM_RABT_TAREEQA,
                )?),
                core::mem::transmute::<DallaWasila, DallaNidaMubashir>(dalla_bism(
                    jalb,
                    ISM_NIDA_MUBASHIR,
                )?),
                core::mem::transmute::<DallaWasila, DallaHadmKaain>(dalla_bism(
                    jalb,
                    ISM_HADM_KAAIN,
                )?),
                core::mem::transmute::<DallaWasila, DallaHadm>(dalla_bism(
                    jalb,
                    ISM_HADM_MUTAGHAYYIR,
                )?),
            )
        };

        // The per-type destructors and converters are fetched here, once,
        // rather than at each construction. They are the other half of every
        // string this adapter builds, and a type whose destructor the engine
        // will not hand over is a type this adapter must not construct at all.
        //
        // SAFETY: `jalb_hadm` and `jalb_tahweel` are the engine's own
        // accessors, resolved above by their documented names. They take a
        // variant type and return null for one they do not know, modelled by
        // `Option`.
        let (hadm_nass, hadm_ism, hadm_masfufa, tahweel_nass, tahweel_raqm) = unsafe {
            (
                jalb_hadm(NAW_NASS).ok_or_else(|| hadm_marfud("String"))?,
                jalb_hadm(NAW_ISM).ok_or_else(|| hadm_marfud("StringName"))?,
                jalb_hadm(NAW_MASFUFAT_BAYT).ok_or_else(|| hadm_marfud("PackedByteArray"))?,
                jalb_tahweel(NAW_NASS).ok_or_else(|| tahweel_marfud("String"))?,
                jalb_tahweel(NAW_RAQM).ok_or_else(|| tahweel_marfud("int"))?,
            )
        };

        let mut wasf = IsdarGodot::default();
        // SAFETY: `isdar` is the engine's `get_godot_version`, and `wasf` is a
        // live, correctly aligned buffer of exactly the layout it writes. The
        // string pointer it fills in is never dereferenced — see
        // `IsdarGodot::wasf`.
        unsafe { isdar(&raw mut wasf) };

        Ok(Self {
            nass_jadeed,
            nass_ila,
            ism_jadeed,
            jalb_bina,
            jalb_tareeqa,
            sard,
            fahras_bayt,
            mufrad,
            bina_kaain,
            rabt_tareeqa,
            nida_mubashir,
            hadm_kaain,
            hadm_nass,
            hadm_ism,
            hadm_masfufa,
            hadm_mutaghayyir,
            tahweel_nass,
            tahweel_raqm,
            kabir: wasf.kabir,
            sagheer: wasf.sagheer,
            tasheeh: wasf.tasheeh,
        })
    }

    /// The engine version this interface reported, as `major.minor.patch`.
    #[must_use]
    pub fn isdar(&self) -> String {
        format!("{}.{}.{}", self.kabir, self.sagheer, self.tasheeh)
    }

    /// The engine's major version.
    #[must_use]
    pub const fn kabir(&self) -> u32 {
        self.kabir
    }

    /// The engine's minor version.
    #[must_use]
    pub const fn sagheer(&self) -> u32 {
        self.sagheer
    }
}

/// `Variant::INT`, for the one integer setting this adapter writes.
pub const NAW_RAQM: NawQeema = 2;

/// The refusal for a variant type whose destructor the engine withheld.
fn hadm_marfud(naw: &str) -> KhataGodot {
    KhataGodot::ImtidadMarfud {
        sabab: format!(
            "this Godot build does not provide a destructor for {naw}, so the adapter \
             declined rather than construct a value it could not release"
        ),
    }
}

/// The refusal for a variant type whose converter the engine withheld.
fn tahweel_marfud(naw: &str) -> KhataGodot {
    KhataGodot::ImtidadMarfud {
        sabab: format!(
            "this Godot build does not provide a Variant converter for {naw}, so no project \
             setting could be written"
        ),
    }
}

// ---------------------------------------------------------------------------
// Values the engine owns and this file must release
// ---------------------------------------------------------------------------

/// How many bytes a caller-owned buffer for one Godot built-in reserves.
///
/// Forty-eight, where the engine writes eight for a `String` or a `StringName`
/// and twenty-four to forty for a `Variant` — forty in a double-precision build,
/// where `real_t` is `double` and the variant's inline storage grows with it.
/// The asymmetry is deliberate and in the safe direction: a buffer larger than
/// the value costs stack bytes nobody notices, and a buffer smaller than it is
/// the engine writing past the end of a local in somebody's game.
pub const HAJM_HASHWA: usize = 48;

/// The most UTF-8 bytes this adapter will read back out of an engine string.
///
/// Locales, setting values and server names are short. A length above this is
/// not a long string, it is a pointer that is not what this code thinks it is,
/// and reading it would walk memory belonging to something else.
pub const AQSA_NASS: i64 = 64 * 1024;

/// A caller-owned buffer for one Godot built-in value.
///
/// Sixteen-byte aligned, which is at least the alignment of every built-in the
/// engine writes here. Under-aligning would be an unaligned store inside the
/// engine, which is a fault on some targets and silently slow on the rest.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct Hashwa([u8; HAJM_HASHWA]);

impl Hashwa {
    /// A zeroed buffer, which is what every construction writes into.
    #[must_use]
    pub const fn khali() -> Self {
        Self([0; HAJM_HASHWA])
    }

    /// The buffer as the engine's mutable type pointer.
    #[must_use]
    pub const fn maqbad(&mut self) -> MaqbadNaw {
        core::ptr::from_mut(self).cast::<c_void>()
    }

    /// The buffer as the engine's constant type pointer.
    #[must_use]
    pub const fn maqbad_thabit(&self) -> *const c_void {
        core::ptr::from_ref(self).cast::<c_void>()
    }

    /// The raw bytes, for a diagnostic that wants to show what the engine left
    /// in a buffer a call was expected to fill and did not.
    #[must_use]
    pub const fn bayt(&self) -> &[u8] {
        &self.0
    }
}

/// One constructed engine value, and the destructor that matches it.
///
/// The pairing lives in [`Drop`] so that it cannot be got wrong by an early
/// return, which is the way it is normally got wrong: a function that
/// constructs three `StringName`s, fails on the second method bind, and returns
/// leaves two permanent entries in the engine's intern table. This adapter
/// constructs a handful per registration, and a handful per registration for
/// the life of a game process is a real leak in somebody else's program.
#[derive(Debug)]
struct QeemaGodot {
    hashwa: Hashwa,
    hadm: DallaHadm,
}

impl QeemaGodot {
    /// Takes ownership of a value the caller has just constructed in `hashwa`.
    ///
    /// The buffer is copied into the new owner, which relocates the value by
    /// `memcpy`. That is sound for every type constructed here — a `String`, a
    /// `StringName` and a `PackedByteArray` are each one pointer into a shared
    /// buffer, and a `Variant` is a tag beside an inline union of pointers and
    /// scalars. None of them is self-referential, and Godot itself relocates all
    /// four this way when it grows an array. What must not happen is a second
    /// destruction of the source, and there is none: `Hashwa` is `Copy` and
    /// carries no destructor, so only the owner built here ever releases the
    /// value.
    const fn jadeeda(hashwa: Hashwa, hadm: DallaHadm) -> Self {
        Self { hashwa, hadm }
    }

    const fn maqbad(&mut self) -> MaqbadNaw {
        self.hashwa.maqbad()
    }

    const fn maqbad_thabit(&self) -> *const c_void {
        self.hashwa.maqbad_thabit()
    }
}

impl Drop for QeemaGodot {
    fn drop(&mut self) {
        // SAFETY: `hashwa` holds a value constructed by the engine's own
        // constructor for the type `hadm` destroys — the two are set together
        // in one expression at each construction site and cannot be paired
        // wrongly. The buffer is live and owned here, this is the only
        // destruction of it, and the type is not `Copy`, so no second value
        // exists to destroy it twice.
        unsafe { (self.hadm)(self.hashwa.maqbad()) }
    }
}

/// A Godot `String`.
///
/// Distinct from [`IsmGodot`] as a type and not only in the documentation:
/// `classdb_get_method_bind` takes `StringName` and `ProjectSettings::set_setting`
/// takes `String`, the two are different structures behind the same pointer
/// width, and handing over the wrong one reads an intern-table entry as a
/// character buffer.
#[derive(Debug)]
pub struct NassGodot(QeemaGodot);

impl NassGodot {
    /// The value as the engine's mutable type pointer.
    #[must_use]
    pub const fn maqbad(&mut self) -> MaqbadNaw {
        self.0.maqbad()
    }

    /// The value as the engine's constant type pointer.
    #[must_use]
    pub const fn maqbad_thabit(&self) -> *const c_void {
        self.0.maqbad_thabit()
    }
}

/// A Godot `StringName` — interned, and not interchangeable with a `String`.
#[derive(Debug)]
pub struct IsmGodot(QeemaGodot);

impl IsmGodot {
    /// The value as the engine's constant type pointer.
    #[must_use]
    pub const fn maqbad_thabit(&self) -> *const c_void {
        self.0.maqbad_thabit()
    }
}

/// A Godot `Variant` — the boxed form every `set_setting` argument takes.
#[derive(Debug)]
pub struct MutaghayyirGodot(QeemaGodot);

impl MutaghayyirGodot {
    /// The value as the engine's constant type pointer.
    #[must_use]
    pub const fn maqbad_thabit(&self) -> *const c_void {
        self.0.maqbad_thabit()
    }
}

/// A Godot `PackedByteArray` holding the patch's font.
#[derive(Debug)]
pub struct MasfufatBaytGodot(QeemaGodot);

impl MasfufatBaytGodot {
    /// The value as the engine's constant type pointer.
    #[must_use]
    pub const fn maqbad_thabit(&self) -> *const c_void {
        self.0.maqbad_thabit()
    }
}

impl Wasila {
    /// Constructs a Godot `String` from a Rust one.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the text contains an interior NUL,
    /// which the engine's UTF-8 constructor would read as the end of the
    /// string — a silent truncation that turns a setting key into a shorter,
    /// different key.
    pub fn nass(&self, nass: &str) -> Result<NassGodot, KhataGodot> {
        let Some(mawsum) = ism_c(nass) else {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: "a string handed to the engine contains an interior NUL".to_owned(),
            });
        };
        let mut hashwa = Hashwa::khali();
        // SAFETY: `hashwa` is a live, zeroed, 16-byte-aligned buffer of 48
        // bytes, which is larger than the eight the engine writes for a
        // `String`. `mawsum` is NUL-terminated with no interior NUL and
        // outlives the call. The value constructed here is handed straight to
        // `QeemaGodot`, which owns its destruction.
        unsafe { (self.nass_jadeed)(hashwa.maqbad(), mawsum.as_ptr().cast()) };
        Ok(NassGodot(QeemaGodot::jadeeda(hashwa, self.hadm_nass)))
    }

    /// Constructs a Godot `StringName`.
    ///
    /// # Errors
    ///
    /// As [`Wasila::nass`].
    pub fn ism(&self, ism: &str) -> Result<IsmGodot, KhataGodot> {
        let Some(mawsum) = ism_c(ism) else {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: "a name handed to the engine contains an interior NUL".to_owned(),
            });
        };
        let mut hashwa = Hashwa::khali();
        // SAFETY: as `Wasila::nass`, for the interned kind. The destructor
        // paired here is the `StringName` one, fetched at resolution time for
        // exactly this purpose.
        unsafe { (self.ism_jadeed)(hashwa.maqbad(), mawsum.as_ptr().cast()) };
        Ok(IsmGodot(QeemaGodot::jadeeda(hashwa, self.hadm_ism)))
    }

    /// Reads a Godot `String` back into a Rust one.
    ///
    /// Two calls: the first with a null buffer asks how long it is, the second
    /// fills a buffer of exactly that length. That is the engine's documented
    /// protocol and it is why no fixed-size buffer appears here.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::HajmMufrit`] when the engine reports a length above
    /// [`AQSA_NASS`], and [`KhataGodot::ImtidadMarfud`] naming the byte offset
    /// when the bytes are not valid UTF-8 — refused rather than replaced,
    /// because a locale tag with a replacement character in it would compare
    /// unequal to everything forever.
    pub fn nass_rust(&self, nass: &NassGodot) -> Result<String, KhataGodot> {
        // SAFETY: `nass` owns a live `String` constructed by this same
        // interface. A null buffer with a zero maximum is the engine's
        // documented way of asking for the length and writes nothing.
        let tul = unsafe { (self.nass_ila)(nass.maqbad_thabit(), core::ptr::null_mut(), 0) };
        if tul <= 0 {
            return Ok(String::new());
        }
        if tul > AQSA_NASS {
            return Err(KhataGodot::HajmMufrit {
                haql: "engine string length",
                qeema: u64::try_from(tul).unwrap_or(u64::MAX),
                saqf: u64::try_from(AQSA_NASS).unwrap_or(u64::MAX),
            });
        }
        let Ok(hajm) = usize::try_from(tul) else {
            return Ok(String::new());
        };
        let mut bayt = vec![0_u8; hajm];
        // SAFETY: as above, and `bayt` is a live buffer of exactly `tul` bytes,
        // which is the maximum passed, so the engine cannot write past it. The
        // engine writes UTF-8 without a terminator when the buffer is exact.
        let kutiba =
            unsafe { (self.nass_ila)(nass.maqbad_thabit(), bayt.as_mut_ptr().cast(), tul) };
        let kutiba = usize::try_from(kutiba.max(0)).unwrap_or(0).min(hajm);
        bayt.truncate(kutiba);
        String::from_utf8(bayt).map_err(|khata| KhataGodot::ImtidadMarfud {
            sabab: format!(
                "an engine string is not valid UTF-8 at byte {}; it was refused rather than \
                 decoded lossily, because a locale tag carrying a replacement character \
                 compares unequal to every locale forever",
                khata.utf8_error().valid_up_to()
            ),
        })
    }

    /// Boxes a `String` into a `Variant`.
    #[must_use]
    pub fn mutaghayyir_nass(&self, nass: &mut NassGodot) -> MutaghayyirGodot {
        let mut hashwa = Hashwa::khali();
        // SAFETY: `hashwa` is 48 bytes, larger than a `Variant` in either
        // precision build, and `nass` owns a live `String`. The converter
        // fetched for `Variant::STRING` at resolution time is the one that
        // matches the value being boxed.
        unsafe { (self.tahweel_nass)(hashwa.maqbad(), nass.maqbad()) };
        MutaghayyirGodot(QeemaGodot::jadeeda(hashwa, self.hadm_mutaghayyir))
    }

    /// Boxes an integer into a `Variant`.
    #[must_use]
    pub fn mutaghayyir_raqm(&self, raqm: i64) -> MutaghayyirGodot {
        let mut masdar = raqm;
        let mut hashwa = Hashwa::khali();
        // SAFETY: as above. The source is a live, correctly aligned `i64`,
        // which is exactly what `Variant::INT`'s converter reads.
        unsafe {
            (self.tahweel_raqm)(
                hashwa.maqbad(),
                core::ptr::from_mut(&mut masdar).cast::<c_void>(),
            );
        }
        MutaghayyirGodot(QeemaGodot::jadeeda(hashwa, self.hadm_mutaghayyir))
    }

    /// Renders a `Variant` as a Rust string.
    ///
    /// # Errors
    ///
    /// Whatever [`Wasila::nass_rust`] refuses.
    pub fn sard_mutaghayyir(&self, mutaghayyir: &MutaghayyirGodot) -> Result<String, KhataGodot> {
        let mut hashwa = Hashwa::khali();
        // SAFETY: `mutaghayyir` owns a live `Variant`, and `hashwa` is a live
        // buffer larger than the `String` the engine writes into it.
        unsafe { (self.sard)(mutaghayyir.maqbad_thabit(), hashwa.maqbad()) };
        let nass = NassGodot(QeemaGodot::jadeeda(hashwa, self.hadm_nass));
        self.nass_rust(&nass)
    }

    /// Builds a `PackedByteArray` holding a copy of the patch's bytes.
    ///
    /// Default-construct, resize once, then one contiguous copy. Not a loop over
    /// bytes: `packed_byte_array_operator_index` returns a pointer into the
    /// array's own storage, which the engine documents as contiguous, so the
    /// whole buffer moves in a single `copy_nonoverlapping`.
    ///
    /// `basmat_hajm` is `PackedByteArray::resize`'s method hash for this engine
    /// version, supplied by the caller — see [`JadwalTuruq`].
    ///
    /// # Errors
    ///
    /// [`KhataGodot::KhattMarfud`] when the engine refuses the constructor, the
    /// resize or the element pointer, and [`KhataGodot::HajmMufrit`] when the
    /// length does not fit the engine's signed length type.
    pub fn masfufat_bayt(
        &self,
        bayt: &[u8],
        basmat_hajm: i64,
    ) -> Result<MasfufatBaytGodot, KhataGodot> {
        let Ok(tul) = RaqmGodot::try_from(bayt.len()) else {
            return Err(KhataGodot::HajmMufrit {
                haql: "font byte count",
                qeema: u64::try_from(bayt.len()).unwrap_or(u64::MAX),
                saqf: u64::try_from(RaqmGodot::MAX).unwrap_or(u64::MAX),
            });
        };

        // SAFETY: the accessor is the engine's own, resolved by name. It
        // returns null for a constructor index a type does not have, which
        // `Option` models.
        let Some(bina) = (unsafe { (self.jalb_bina)(NAW_MASFUFAT_BAYT, BANI_IFTIRADI) }) else {
            return Err(KhataGodot::KhattMarfud {
                sabab: "this Godot build has no default constructor for PackedByteArray".to_owned(),
            });
        };
        let mut hashwa = Hashwa::khali();
        let bila: [*const c_void; 0] = [];
        // SAFETY: `hashwa` is a live buffer larger than the one pointer a
        // `PackedByteArray` occupies. The default constructor reads no
        // arguments, so an empty argument array is exactly what it expects.
        unsafe { bina(hashwa.maqbad(), bila.as_ptr()) };
        let mut masfufa = MasfufatBaytGodot(QeemaGodot::jadeeda(hashwa, self.hadm_masfufa));

        if tul == 0 {
            return Ok(masfufa);
        }

        let ism_hajm = self.ism("resize")?;
        // SAFETY: the accessor is the engine's own; `ism_hajm` owns a live
        // `StringName`; and `basmat_hajm` is a hash the caller took from this
        // engine version's own table. The engine returns null for a hash it does
        // not recognise rather than binding the wrong method.
        let Some(hajm) = (unsafe {
            (self.jalb_tareeqa)(NAW_MASFUFAT_BAYT, ism_hajm.maqbad_thabit(), basmat_hajm)
        }) else {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!(
                    "PackedByteArray::resize did not bind at hash {basmat_hajm} on Godot {}",
                    self.isdar()
                ),
            });
        };

        let mut natijat_hajm: i64 = 0;
        let muamalat: [*const c_void; 1] = [core::ptr::from_ref(&tul).cast::<c_void>()];
        // SAFETY: `masfufa` owns a live, default-constructed
        // `PackedByteArray`; `muamalat` holds one live `i64` for the one
        // argument `resize` takes; and `natijat_hajm` is a live `i64` for the
        // `Error` code it returns. The argument count matches the signature the
        // hash identifies.
        unsafe {
            hajm(
                masfufa.0.maqbad(),
                muamalat.as_ptr(),
                core::ptr::from_mut(&mut natijat_hajm).cast::<c_void>(),
                1,
            );
        }

        // SAFETY: the array was just resized to `tul`, so element zero exists.
        let awwal = unsafe { (self.fahras_bayt)(masfufa.0.maqbad(), 0) };
        if awwal.is_null() {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!(
                    "PackedByteArray::resize({tul}) reported {natijat_hajm} and left the \
                     array without storage"
                ),
            });
        }
        // SAFETY: `awwal` points at element zero of an array the engine just
        // resized to `bayt.len()` elements, whose storage the engine documents
        // as contiguous. Source and destination are distinct allocations — one
        // is the patch's buffer, the other the engine's — so they cannot
        // overlap.
        unsafe { core::ptr::copy_nonoverlapping(bayt.as_ptr(), awwal, bayt.len()) };
        Ok(masfufa)
    }
}

// ---------------------------------------------------------------------------
// The engine methods this adapter calls
// ---------------------------------------------------------------------------

/// `ProjectSettings`, reached as a singleton.
pub const SANF_IDADAT: &str = "ProjectSettings";
/// `ThemeDB`, reached as a singleton.
pub const SANF_QAIDAT_SIMA: &str = "ThemeDB";
/// `TranslationServer`, reached as a singleton.
pub const SANF_KHADIM_TARJAMA: &str = "TranslationServer";
/// `TextServerManager`, reached as a singleton.
pub const SANF_MUDIR_KHADIM: &str = "TextServerManager";
/// `TextServer`, reached through the manager's primary interface.
pub const SANF_KHADIM: &str = "TextServer";
/// `Theme`, reached through the theme database's default theme.
pub const SANF_SIMA: &str = "Theme";
/// `FontFile`, constructed from the patch's bytes.
pub const SANF_KHATT: &str = "FontFile";

/// `ProjectSettings::set_setting(String, Variant)`.
pub const TAREEQA_IDAD_DAA: &str = "set_setting";
/// `ProjectSettings::get_setting(String, Variant) -> Variant`.
pub const TAREEQA_IDAD_IQRA: &str = "get_setting";
/// `FontFile::set_data(PackedByteArray)`.
pub const TAREEQA_KHATT_BAYANAT: &str = "set_data";
/// `ThemeDB::get_default_theme() -> Ref<Theme>`.
pub const TAREEQA_SIMA_QAIDA: &str = "get_default_theme";
/// `Theme::set_default_font(Ref<Font>)`.
pub const TAREEQA_SIMA_IFTIRADI: &str = "set_default_font";
/// `Theme::set_font(StringName, StringName, Ref<Font>)`.
pub const TAREEQA_SIMA_KHATT: &str = "set_font";
/// `TranslationServer::set_locale(String)`.
pub const TAREEQA_THAQAFA_DAA: &str = "set_locale";
/// `TranslationServer::get_locale() -> String`.
pub const TAREEQA_THAQAFA_IQRA: &str = "get_locale";
/// `TextServerManager::get_primary_interface() -> Ref<TextServer>`.
pub const TAREEQA_KHADIM_ASASI: &str = "get_primary_interface";
/// `TextServer::get_name() -> String`.
pub const TAREEQA_KHADIM_ISM: &str = "get_name";

/// The method hashes for the engine build this extension is loaded into.
///
/// Every field is the value `extension_api.json` records for that method on
/// that exact version. They are **supplied**, never derived and never guessed:
/// a hash identifies a signature, the engine refuses a bind whose hash it does
/// not know, and a number written from memory would fail to bind on every build
/// while looking like an engine that does not have the method. That is the same
/// rule `wasl.rs` applies to vtable indices, and it exists so that a wrong
/// number is a caller's assertion rather than a constant nobody can check.
///
/// A field left at zero simply fails to bind and the method it names refuses by
/// name, so a caller that has hashes for some methods and not others still gets
/// the ones it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JadwalTuruq {
    /// `ProjectSettings::set_setting`.
    pub idad_daa: i64,
    /// `ProjectSettings::get_setting`.
    pub idad_iqra: i64,
    /// `FontFile::set_data`.
    pub khatt_bayanat: i64,
    /// `ThemeDB::get_default_theme`.
    pub sima_qaida: i64,
    /// `Theme::set_default_font`.
    pub sima_iftiradi: i64,
    /// `Theme::set_font`.
    pub sima_khatt: i64,
    /// `TranslationServer::set_locale`.
    pub thaqafa_daa: i64,
    /// `TranslationServer::get_locale`.
    pub thaqafa_iqra: i64,
    /// `TextServerManager::get_primary_interface`.
    pub khadim_asasi: i64,
    /// `TextServer::get_name`.
    pub khadim_ism: i64,
    /// `PackedByteArray::resize`, a built-in method rather than a class one.
    pub masfufa_hajm: i64,
}

// ---------------------------------------------------------------------------
// The binding
// ---------------------------------------------------------------------------

/// The `FontFile` this extension built, remembered between calls.
///
/// The address is held as an integer so the whole binding stays [`Send`] and
/// [`Sync`] without an `unsafe impl` nobody can check, and the pointer is
/// reconstructed through [`core::ptr::with_exposed_provenance_mut`] — the
/// documented way to rebuild a pointer to memory this program did not allocate.
#[derive(Debug)]
struct KhattMusajjal {
    kaain: usize,
    ism: String,
}

/// A live connection to Godot 4, and the [`Musajjil`] the ladder drives.
///
/// Holding one is the proof that all sixteen interface functions resolved and
/// that the engine reported a version this adapter understands. Nothing in
/// `khadim_nusus` touches the engine without one.
#[derive(Debug)]
pub struct WaslGodot {
    wasila: Wasila,
    turuq: JadwalTuruq,
    khatt: OnceLock<KhattMusajjal>,
}

impl WaslGodot {
    /// Wraps a resolved interface and the caller's hash table.
    #[must_use]
    pub const fn jadeed(wasila: Wasila, turuq: JadwalTuruq) -> Self {
        Self {
            wasila,
            turuq,
            khatt: OnceLock::new(),
        }
    }

    /// The resolved interface, for a caller that needs to build values itself.
    #[must_use]
    pub const fn wasila(&self) -> &Wasila {
        &self.wasila
    }

    /// The engine version this binding is talking to.
    #[must_use]
    pub fn isdar(&self) -> String {
        self.wasila.isdar()
    }

    /// Reaches one engine singleton by class name.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the engine has no such singleton,
    /// which before the scene initialisation level is the ordinary answer for
    /// every singleton this adapter wants.
    pub fn mufrad(&self, sanf: &str) -> Result<MaqbadKaain, KhataGodot> {
        let ism = self.wasila.ism(sanf)?;
        // SAFETY: `mufrad` is the engine's `global_get_singleton`, resolved by
        // name, and `ism` owns a live `StringName` for the duration of the
        // call. The engine returns null for a singleton that does not exist.
        let kaain = unsafe { (self.wasila.mufrad)(ism.maqbad_thabit()) };
        if kaain.is_null() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "the {sanf} singleton does not exist in this process yet, which means \
                     the extension ran before the scene initialisation level"
                ),
            });
        }
        Ok(kaain)
    }

    /// Binds one engine method by class, name and the caller's hash.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] naming the class, the method and the hash
    /// that did not bind — which is the message a maintainer needs, because it
    /// says whether the patch's hash table is for a different engine version.
    pub fn tareeqa(
        &self,
        sanf: &str,
        ism_tareeqa: &str,
        basma: i64,
    ) -> Result<MaqbadTareeqa, KhataGodot> {
        let sanf_godot = self.wasila.ism(sanf)?;
        let ism_godot = self.wasila.ism(ism_tareeqa)?;
        // SAFETY: `rabt_tareeqa` is the engine's `classdb_get_method_bind`,
        // both names own live `StringName`s for the duration of the call, and
        // the engine returns null for a class, method or hash it does not
        // recognise rather than binding something else.
        let tareeqa = unsafe {
            (self.wasila.rabt_tareeqa)(sanf_godot.maqbad_thabit(), ism_godot.maqbad_thabit(), basma)
        };
        if tareeqa.is_null() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "{sanf}::{ism_tareeqa} did not bind at hash {basma} on Godot {}; the \
                     patch's method table is for a different engine version",
                    self.wasila.isdar()
                ),
            });
        }
        Ok(tareeqa)
    }

    /// Calls a bound method.
    ///
    /// # Safety
    ///
    /// `tareeqa` must be a bind obtained from [`WaslGodot::tareeqa`] for a
    /// method of `kaain`'s class; `kaain` must be a live object of that class;
    /// `muamalat` must hold exactly the arguments that method's signature takes,
    /// each a pointer to a live value of the right type; and `makhraj` must be
    /// null for a method returning nothing or a live buffer large enough for
    /// what it returns. Every one of those comes from the hash: it identifies
    /// the signature, and the caller asserted the hash by putting it in
    /// [`JadwalTuruq`].
    pub unsafe fn nadi(
        &self,
        tareeqa: MaqbadTareeqa,
        kaain: MaqbadKaain,
        muamalat: &[*const c_void],
        makhraj: MaqbadNaw,
    ) {
        // SAFETY: delegated verbatim to this function's own contract. The
        // argument array is a live slice for the duration of the call and the
        // engine only reads it.
        unsafe {
            (self.wasila.nida_mubashir)(tareeqa, kaain, muamalat.as_ptr(), makhraj);
        }
    }

    /// The registered `FontFile`, or a refusal naming what is missing.
    fn khatt_musajjal(&self, ism: &str) -> Result<MaqbadKaain, KhataGodot> {
        let Some(khatt) = self.khatt.get() else {
            return Err(KhataGodot::KhattMarfud {
                sabab: "no font has been registered with this binding yet".to_owned(),
            });
        };
        if khatt.ism != ism {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!(
                    "the registered font is \"{}\" and \"{ism}\" was asked for",
                    khatt.ism
                ),
            });
        }
        Ok(core::ptr::with_exposed_provenance_mut::<c_void>(
            khatt.kaain,
        ))
    }

    /// The engine's default `Theme`.
    fn sima_qaida(&self) -> Result<MaqbadKaain, KhataGodot> {
        let qaida = self.mufrad(SANF_QAIDAT_SIMA)?;
        let tareeqa = self.tareeqa(SANF_QAIDAT_SIMA, TAREEQA_SIMA_QAIDA, self.turuq.sima_qaida)?;
        let mut sima: MaqbadKaain = core::ptr::null_mut();
        // SAFETY: `qaida` is the live `ThemeDB` singleton and `tareeqa` binds
        // its no-argument `get_default_theme`. A `Ref<Theme>` return is encoded
        // into the return slot as one object pointer, so a live `MaqbadKaain`
        // is exactly the buffer it needs.
        unsafe {
            self.nadi(
                tareeqa,
                qaida,
                &[],
                core::ptr::from_mut(&mut sima).cast::<c_void>(),
            );
        }
        if sima.is_null() {
            return Err(KhataGodot::KhattMarfud {
                sabab: "ThemeDB has no default theme, which means the scene level has not \
                        initialised"
                    .to_owned(),
            });
        }
        Ok(sima)
    }
}

impl WaslGodot {
    /// Destroys an object this extension constructed and never handed over.
    ///
    /// Only for the failure paths in [`Musajjil::sajjil_khatt`]. An object the
    /// engine has taken a reference to is the engine's, and destroying it here
    /// would be a use-after-free in the theme; an object that was constructed
    /// and then abandoned is this extension's, and leaving it would be a
    /// `Resource` alive for the life of the game with nothing pointing at it.
    fn ahdim(&self, kaain: MaqbadKaain) {
        if kaain.is_null() {
            return;
        }
        // SAFETY: `kaain` came from `classdb_construct_object` in this same
        // function's caller, has not been passed to any engine method that
        // would have taken a reference to it, and is destroyed exactly once
        // here on a path that then returns an error.
        unsafe { (self.wasila.hadm_kaain)(kaain) }
    }

    /// Boxes a setting value into the `Variant` `set_setting` expects.
    fn mutaghayyir_idad(&self, qeema: &QeemaIdad) -> Result<MutaghayyirGodot, KhataGodot> {
        match qeema {
            QeemaIdad::Nass(nass) => {
                let mut nass_godot = self.wasila.nass(nass)?;
                Ok(self.wasila.mutaghayyir_nass(&mut nass_godot))
            },
            QeemaIdad::Raqm(raqm) => Ok(self.wasila.mutaghayyir_raqm(*raqm)),
            // Refused rather than approximated. Writing a `PackedStringArray`
            // live would need a second element accessor and a `String`
            // assignment per entry, and it would buy nothing: the translation
            // list is read by `TranslationServer` while the engine starts, long
            // before an extension registered at the scene level runs, so a
            // value written here is a value nothing will ever read. The list
            // belongs to the configuration rung and is complete there.
            QeemaIdad::Qaima(qaima) => Err(KhataGodot::ThaqafaMarfuda {
                sabab: format!(
                    "a list of {} paths cannot usefully be written into a running process: \
                     the engine reads the translation list during start, so this key is the \
                     configuration rung's alone",
                    qaima.len()
                ),
            }),
        }
    }
}

impl Musajjil for WaslGodot {
    fn muhayya(&self) -> bool {
        self.mufrad(SANF_IDADAT).is_ok()
    }

    fn daa_idad(&self, miftah: &str, qeema: &QeemaIdad) -> Result<(), KhataGodot> {
        let idadat = self.mufrad(SANF_IDADAT)?;
        let tareeqa = self.tareeqa(SANF_IDADAT, TAREEQA_IDAD_DAA, self.turuq.idad_daa)?;
        let miftah_godot = self.wasila.nass(miftah)?;
        let qeema_godot = self.mutaghayyir_idad(qeema)?;
        let muamalat = [miftah_godot.maqbad_thabit(), qeema_godot.maqbad_thabit()];
        // SAFETY: `idadat` is the live `ProjectSettings` singleton; `tareeqa`
        // binds its `set_setting`, whose signature the caller asserted by
        // supplying the hash; the two arguments are a live `String` and a live
        // `Variant` in that order; and the method returns nothing, so the
        // return slot is null. Both values outlive the call and are destroyed
        // by their own `Drop` afterwards.
        unsafe { self.nadi(tareeqa, idadat, &muamalat, core::ptr::null_mut()) };
        Ok(())
    }

    fn iqra_idad(&self, miftah: &str) -> Result<String, KhataGodot> {
        let idadat = self.mufrad(SANF_IDADAT)?;
        let tareeqa = self.tareeqa(SANF_IDADAT, TAREEQA_IDAD_IQRA, self.turuq.idad_iqra)?;
        let miftah_godot = self.wasila.nass(miftah)?;
        let mut farigh = self.wasila.nass("")?;
        let iftiradi = self.wasila.mutaghayyir_nass(&mut farigh);
        let muamalat = [miftah_godot.maqbad_thabit(), iftiradi.maqbad_thabit()];
        let mut hashwa = Hashwa::khali();
        // SAFETY: as `daa_idad`, and the return slot is a live 48-byte buffer,
        // larger than a `Variant` in either precision build, which is what
        // `get_setting` writes into it.
        unsafe { self.nadi(tareeqa, idadat, &muamalat, hashwa.maqbad()) };
        // Wrapped immediately so the returned `Variant` is released even if the
        // conversion below fails.
        let makhraj = MutaghayyirGodot(QeemaGodot::jadeeda(hashwa, self.wasila.hadm_mutaghayyir));
        self.wasila.sard_mutaghayyir(&makhraj)
    }

    fn sajjil_khatt(&self, ism: &str, bayt: &[u8]) -> Result<(), KhataGodot> {
        if let Some(mawjud) = self.khatt.get() {
            return if mawjud.ism == ism {
                Ok(())
            } else {
                Err(KhataGodot::KhattMarfud {
                    sabab: format!(
                        "this binding already registered \"{}\" and will not replace it with \
                         \"{ism}\"; a second FontFile would leave the first assigned in the \
                         theme with nothing owning it",
                        mawjud.ism
                    ),
                })
            };
        }

        let sanf = self.wasila.ism(SANF_KHATT)?;
        // SAFETY: `bina_kaain` is the engine's `classdb_construct_object`, and
        // `sanf` owns a live `StringName` for the duration of the call. The
        // engine returns null for a class it does not have.
        let kaain = unsafe { (self.wasila.bina_kaain)(sanf.maqbad_thabit()) };
        if kaain.is_null() {
            return Err(KhataGodot::KhattMarfud {
                sabab: format!("this Godot build could not construct a {SANF_KHATT}"),
            });
        }

        let masfufa = match self.wasila.masfufat_bayt(bayt, self.turuq.masfufa_hajm) {
            Ok(masfufa) => masfufa,
            Err(khata) => {
                self.ahdim(kaain);
                return Err(khata);
            },
        };
        let tareeqa =
            match self.tareeqa(SANF_KHATT, TAREEQA_KHATT_BAYANAT, self.turuq.khatt_bayanat) {
                Ok(tareeqa) => tareeqa,
                Err(khata) => {
                    self.ahdim(kaain);
                    return Err(khata);
                },
            };

        let muamalat = [masfufa.maqbad_thabit()];
        // SAFETY: `kaain` is a live `FontFile` this call constructed; `tareeqa`
        // binds `FontFile::set_data`, whose one argument is a
        // `PackedByteArray`; `masfufa` owns a live one holding a copy of the
        // patch's bytes; and the method returns nothing. The engine copies the
        // array into the resource, so `masfufa` may be dropped afterwards.
        unsafe { self.nadi(tareeqa, kaain, &muamalat, core::ptr::null_mut()) };

        let musajjal = KhattMusajjal {
            kaain: kaain.cast_const().expose_provenance(),
            ism: ism.to_owned(),
        };
        if self.khatt.set(musajjal).is_err() {
            // Another thread won the race. Godot calls the initialisation
            // callbacks from the main thread, so this cannot happen in the
            // engine's own flow; handling it costs one branch and the
            // alternative is a `FontFile` nobody owns.
            self.ahdim(kaain);
        }
        Ok(())
    }

    fn asnid_khatt_iftiradi(&self, ism: &str) -> Result<(), KhataGodot> {
        let khatt = self.khatt_musajjal(ism)?;
        let sima = self.sima_qaida()?;
        let tareeqa = self.tareeqa(SANF_SIMA, TAREEQA_SIMA_IFTIRADI, self.turuq.sima_iftiradi)?;
        let muamalat = [core::ptr::from_ref(&khatt).cast::<c_void>()];
        // SAFETY: `sima` is the live default `Theme`; `tareeqa` binds its
        // `set_default_font`, whose one argument is a `Ref<Font>`; and a
        // `Ref<T>` argument is encoded as a pointer to the object pointer,
        // which is exactly what `&khatt` is. The engine takes a reference to
        // the resource, which is why nothing here destroys it afterwards.
        unsafe { self.nadi(tareeqa, sima, &muamalat, core::ptr::null_mut()) };
        Ok(())
    }

    fn asnid_khatt_naw(&self, madkhal: &MadkhalSima, ism: &str) -> Result<(), KhataGodot> {
        let khatt = self.khatt_musajjal(ism)?;
        let sima = self.sima_qaida()?;
        let tareeqa = self.tareeqa(SANF_SIMA, TAREEQA_SIMA_KHATT, self.turuq.sima_khatt)?;
        let ism_madkhal = self.wasila.ism(&madkhal.madkhal)?;
        let ism_naw = self.wasila.ism(&madkhal.naw)?;
        let muamalat = [
            ism_madkhal.maqbad_thabit(),
            ism_naw.maqbad_thabit(),
            core::ptr::from_ref(&khatt).cast::<c_void>(),
        ];
        // SAFETY: as `asnid_khatt_iftiradi`. `Theme::set_font` takes the entry
        // name, then the theme type, then the font — both names are live
        // `StringName`s and the order is the engine's, not this file's guess:
        // reversing them would define a font entry named after the widget.
        unsafe { self.nadi(tareeqa, sima, &muamalat, core::ptr::null_mut()) };
        Ok(())
    }

    fn hammil_tarjama(&self, wasm: &str, bayt: &[u8]) -> Result<(), KhataGodot> {
        Err(KhataGodot::ThaqafaMarfuda {
            sabab: format!(
                "Godot loads a Translation through ResourceLoader, which takes a res:// path \
                 and has no interface call that turns {} bytes into a Resource. The patch's \
                 {wasm} translation reaches the engine the way Godot expects — as a resource \
                 inside the patch package, listed in the translation setting — and this \
                 binding sets the locale rather than pretending to hand the bytes over",
                bayt.len()
            ),
        })
    }

    fn thabbit_thaqafa(&self, wasm: &str) -> Result<(), KhataGodot> {
        let khadim = self.mufrad(SANF_KHADIM_TARJAMA)?;
        let tareeqa = self.tareeqa(
            SANF_KHADIM_TARJAMA,
            TAREEQA_THAQAFA_DAA,
            self.turuq.thaqafa_daa,
        )?;
        let wasm_godot = self.wasila.nass(wasm)?;
        let muamalat = [wasm_godot.maqbad_thabit()];
        // SAFETY: `khadim` is the live `TranslationServer` singleton, `tareeqa`
        // binds its `set_locale`, whose one argument is a `String`, and
        // `wasm_godot` owns a live one for the duration of the call.
        unsafe { self.nadi(tareeqa, khadim, &muamalat, core::ptr::null_mut()) };
        Ok(())
    }

    fn thaqafa_haliya(&self) -> Result<String, KhataGodot> {
        let khadim = self.mufrad(SANF_KHADIM_TARJAMA)?;
        let tareeqa = self.tareeqa(
            SANF_KHADIM_TARJAMA,
            TAREEQA_THAQAFA_IQRA,
            self.turuq.thaqafa_iqra,
        )?;
        let mut hashwa = Hashwa::khali();
        // SAFETY: as above, for the no-argument `get_locale`. The return slot is
        // a live buffer larger than the `String` the engine writes into it, and
        // the value is wrapped immediately so it is released on every path.
        unsafe { self.nadi(tareeqa, khadim, &[], hashwa.maqbad()) };
        let nass = NassGodot(QeemaGodot::jadeeda(hashwa, self.wasila.hadm_nass));
        self.wasila.nass_rust(&nass)
    }

    fn thabbit_ittijah(&self, takhtit: IttijahTakhtit) -> Result<(), KhataGodot> {
        Err(KhataGodot::ImtidadMarfud {
            sabab: format!(
                "the root window's layout direction is a project setting on this path, not a \
                 live call: reaching the window means Engine::get_main_loop, SceneTree::get_root \
                 and Window::set_layout_direction, three more hashes for no gain, because \
                 writing {} into the root direction setting reaches the same code. See the \
                 configuration rung",
                takhtit.qeema()
            ),
        })
    }

    fn thabbit_ittijah_uqda(
        &self,
        masar_uqda: &str,
        takhtit: IttijahTakhtit,
        nass: IttijahNass,
    ) -> Result<(), KhataGodot> {
        Err(KhataGodot::ImtidadMarfud {
            sabab: format!(
                "setting {} and {} on \"{masar_uqda}\" needs a NodePath, a scene-tree walk and \
                 two more method hashes, and this binding does not do it: a Control left at \
                 its inherited direction already follows the root, and one that hard-codes a \
                 direction is a game-specific correction the patch names rather than a \
                 mechanism this file guesses at",
                takhtit.ism(),
                nass.ism()
            ),
        })
    }

    fn ism_khadim(&self) -> Result<String, KhataGodot> {
        let mudir = self.mufrad(SANF_MUDIR_KHADIM)?;
        let asasi = self.tareeqa(
            SANF_MUDIR_KHADIM,
            TAREEQA_KHADIM_ASASI,
            self.turuq.khadim_asasi,
        )?;
        let mut khadim: MaqbadKaain = core::ptr::null_mut();
        // SAFETY: `mudir` is the live `TextServerManager` singleton and `asasi`
        // binds its no-argument `get_primary_interface`, whose `Ref<TextServer>`
        // return is encoded into the slot as one object pointer.
        unsafe {
            self.nadi(
                asasi,
                mudir,
                &[],
                core::ptr::from_mut(&mut khadim).cast::<c_void>(),
            );
        }
        if khadim.is_null() {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: "TextServerManager has no primary interface yet".to_owned(),
            });
        }
        let ism = self.tareeqa(SANF_KHADIM, TAREEQA_KHADIM_ISM, self.turuq.khadim_ism)?;
        let mut hashwa = Hashwa::khali();
        // SAFETY: `khadim` is the live primary `TextServer` the manager just
        // returned, and `ism` binds its no-argument `get_name`, which writes a
        // `String` into the return slot.
        unsafe { self.nadi(ism, khadim, &[], hashwa.maqbad()) };
        let nass = NassGodot(QeemaGodot::jadeeda(hashwa, self.wasila.hadm_nass));
        self.wasila.nass_rust(&nass)
    }
}

// ---------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------

/// The binding, resolved once at load.
static WASL: OnceLock<WaslGodot> = OnceLock::new();

/// Why the extension declined, when it did.
static SABAB: OnceLock<String> = OnceLock::new();

/// The method hashes, installed by the patch's companion before the scene level.
static TURUQ: OnceLock<JadwalTuruq> = OnceLock::new();

/// The configuration to apply, installed the same way.
static KHADIM: OnceLock<KhadimNusus> = OnceLock::new();

/// What the scene-level callback achieved.
static NATIJA: OnceLock<NatijatKhadim> = OnceLock::new();

/// Nothing has been decided yet.
const HALA_MAJHULA: u8 = 0;
/// The interface resolved and the extension registered.
const HALA_MARBUTA: u8 = 1;
/// The extension declined; [`sabab_rafd`] says why.
const HALA_MARFUDA: u8 = 2;

/// Whether the entry point bound, declined, or has not run.
///
/// An atomic rather than a lock: it is written once from the loading thread and
/// read from wherever a diagnostic asks, and the workspace bans
/// [`std::sync::Mutex`] precisely so that a one-byte fact does not acquire one.
static HALA: AtomicU8 = AtomicU8::new(HALA_MAJHULA);

/// The highest initialisation level the engine has called this extension at.
static MUSTAWA_BALAGH: AtomicU8 = AtomicU8::new(0);

/// The process's binding, once the entry point has run and succeeded.
#[must_use]
pub fn wasl() -> Option<&'static WaslGodot> {
    WASL.get()
}

/// Why the extension declined, if it did.
#[must_use]
pub fn sabab_rafd() -> Option<&'static str> {
    SABAB.get().map(String::as_str)
}

/// Whether the extension is bound.
#[must_use]
pub fn marbuta() -> bool {
    HALA.load(Ordering::Acquire) == HALA_MARBUTA
}

/// The highest initialisation level Godot has called this extension at.
///
/// [`None`] before the first callback. Worth reporting: an extension that bound
/// and was never called at [`MustawaTahyia::Sina`] is an extension the engine
/// loaded and then unloaded before the scene existed, which looks identical from
/// the outside to one that applied nothing.
#[must_use]
pub fn mustawa_balagh() -> Option<MustawaTahyia> {
    MustawaTahyia::min_qeema(u32::from(MUSTAWA_BALAGH.load(Ordering::Acquire)))
}

/// Installs the method hash table for the engine build being run.
///
/// Called by the patch's generated companion before Godot reaches the scene
/// level. Returns `false` when a table is already installed, which is not an
/// error and not a replacement: the first table wins, because a second one
/// arriving mid-session would rebind methods a font was already registered
/// through.
///
/// A table that was never installed leaves every hash at zero, and every method
/// then refuses by name with `hash 0` in the message — which reads, correctly,
/// as "the patch's method table was not installed" rather than as an engine
/// that lacks the method.
pub fn thabbit_turuq(turuq: JadwalTuruq) -> bool {
    TURUQ.set(turuq).is_ok()
}

/// The installed method hash table, or an empty one.
#[must_use]
pub fn turuq() -> JadwalTuruq {
    TURUQ.get().copied().unwrap_or_default()
}

/// Installs the configuration the scene-level callback applies.
///
/// Returns `false` when one is already installed.
pub fn thabbit_khadim(khadim: KhadimNusus) -> bool {
    KHADIM.set(khadim).is_ok()
}

/// What the scene-level callback achieved, once it has run.
#[must_use]
pub fn natija() -> Option<&'static NatijatKhadim> {
    NATIJA.get()
}

/// Records a refusal once and returns false, which is what the entry point
/// returns to Godot.
fn urfud(sabab: String) -> MantiqGodot {
    tracing::warn!(sabab = %sabab, "the Taarib GDExtension declined to register");
    let _ = SABAB.set(sabab);
    HALA.store(HALA_MARFUDA, Ordering::Release);
    0
}

/// Fills the initialisation structure for an extension that is declining.
///
/// Both callbacks are left null, which the engine accepts and which means
/// nothing of Taarib's runs at any level. Writing the structure even on the
/// failure path matters: an engine that reads an uninitialised
/// `GDExtensionInitialization` after a false return would read whatever the
/// allocation held.
///
/// # Safety
///
/// `tahyia` must be the non-null, writable structure Godot passed.
unsafe fn imla_rafd(tahyia: *mut TahyiatImtidad) {
    // SAFETY: the caller guarantees `tahyia` points at the live, writable,
    // correctly aligned structure the engine allocated for this call.
    unsafe {
        tahyia.write(TahyiatImtidad {
            mustawa: MustawaTahyia::Sina.qeema(),
            bayanat: core::ptr::null_mut(),
            tahyia: None,
            inha: None,
        });
    }
}

// ---------------------------------------------------------------------------
// The entry point
// ---------------------------------------------------------------------------

/// The symbol Godot 4 calls, named by the `.gdextension` manifest's
/// `entry_symbol`.
///
/// Returns 1 when the extension registered and 0 when it declined. A false
/// return is a supported outcome: the engine logs it and carries on loading the
/// game, which is exactly what should happen when a font-and-locale patch
/// cannot bind — the player gets the game in its original language rather than
/// no game.
///
/// Nothing in here can unwind into the engine. Every path runs inside
/// [`std::panic::catch_unwind`], because the workspace builds with
/// `panic = "unwind"` so that a fault in a library loaded into somebody's game
/// is caught at the ABI edge and reported, and an unwind across an `extern "C"`
/// boundary would abort the player's process instead.
///
/// # Safety
///
/// Called by Godot 4 with the three arguments its loader passes: an interface
/// getter (or, on 4.0, the interface struct — see [`nawa_wasila`]), the library
/// handle, and a writable initialisation structure. No other caller is
/// supported, and every guarantee below rests on the engine being the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_imtidad(
    wasila: *const c_void,
    maktaba: MaqbadMaktaba,
    tahyia: *mut TahyiatImtidad,
) -> MantiqGodot {
    // The library handle identifies this extension when it registers classes
    // with the engine. This one registers none — it sets a font, a theme entry,
    // a direction and a locale, and introduces no new type — so the handle is
    // acknowledged and not kept.
    let _ = maktaba;

    if tahyia.is_null() {
        return urfud(
            "Godot passed a null initialisation structure, so the extension could not even \
             record that it was declining"
                .to_owned(),
        );
    }

    let natija = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: `tahyia` is non-null and, per this function's contract, the
        // live writable structure the engine allocated.
        unsafe { imla_rafd(tahyia) };

        if wasila.is_null() {
            return urfud(
                "Godot passed a null interface pointer, which no released engine does".to_owned(),
            );
        }

        // SAFETY: `wasila` is non-null and, per this function's contract, is
        // either the 4.0 interface struct or the 4.1 `get_proc_address`.
        let naw = unsafe { nawa_wasila(wasila) };
        let NawWasila::Jadida = naw else {
            let NawWasila::Qadima { sagheer, tasheeh } = naw else {
                return urfud("the interface kind could not be determined".to_owned());
            };
            return urfud(format!(
                "this is Godot 4.{sagheer}.{tasheeh}, which passes the GDExtension interface \
                 as a fixed-layout struct rather than as get_proc_address. Reaching a member \
                 of that struct means indexing it at an offset this build does not carry, and \
                 a guessed offset calls an unrelated engine function. Taarib declines here \
                 rather than crash the game; Godot 4.1 or newer is required"
            ));
        };

        // SAFETY: `nawa_wasila` established that `wasila` is the address of
        // `get_proc_address`, whose C signature is
        // `GDExtensionInterfaceFunctionPtr (*)(const char *)`. Transmuting a
        // code address to a function pointer of the matching signature is the
        // only way to call it.
        let jalb = unsafe { core::mem::transmute::<*const c_void, DallaJalbUnwan>(wasila) };

        // SAFETY: `jalb` is the engine's own `get_proc_address`, established
        // immediately above, which is the whole of `Wasila::ijlib`'s contract.
        let mawsula = match unsafe { Wasila::ijlib(jalb) } {
            Ok(mawsula) => mawsula,
            Err(khata) => return urfud(khata.to_string()),
        };

        if mawsula.kabir() == 0 {
            return urfud(
                KhataGodot::IsdarMajhul {
                    sabab: "get_godot_version reported 0.0.0, so nothing about this engine \
                            could be established and no assumption was made about it"
                        .to_owned(),
                }
                .to_string(),
            );
        }
        if mawsula.kabir() != 4 {
            return urfud(
                KhataGodot::MuharrikGhayrMadum {
                    wujid: mawsula.isdar(),
                }
                .to_string(),
            );
        }

        tracing::info!(
            isdar = %mawsula.isdar(),
            dawal = ASMA_DAWAL.len(),
            "the Taarib GDExtension bound Godot's interface"
        );
        let _ = WASL.set(WaslGodot::jadeed(mawsula, turuq()));
        HALA.store(HALA_MARBUTA, Ordering::Release);

        // SAFETY: as the first write. The structure is rewritten with the real
        // callbacks now that there is something for them to do.
        unsafe {
            tahyia.write(TahyiatImtidad {
                mustawa: MustawaTahyia::Sina.qeema(),
                bayanat: core::ptr::null_mut(),
                tahyia: Some(tahyia_mustawa),
                inha: Some(inha_mustawa),
            });
        }
        1
    }));

    natija.unwrap_or_else(|_| {
        HALA.store(HALA_MARFUDA, Ordering::Release);
        0
    })
}

// ---------------------------------------------------------------------------
// The initialisation callbacks
// ---------------------------------------------------------------------------

/// Godot's per-level initialisation callback.
///
/// The engine calls this once for each level from the one the entry point asked
/// for up to the editor level, so it runs more than once and must do its work
/// at exactly one of them. Everything Taarib does happens at
/// [`MustawaTahyia::Sina`]; every other level is acknowledged and skipped.
///
/// # Safety
///
/// Called only by Godot, with `bayanat` being the pointer the entry point wrote
/// into the initialisation structure — null here — and `mustawa` one of the
/// engine's initialisation levels.
unsafe extern "C" fn tahyia_mustawa(bayanat: *mut c_void, mustawa: u32) {
    let _ = bayanat;
    let _ = std::panic::catch_unwind(|| {
        let Some(mustawa) = MustawaTahyia::min_qeema(mustawa) else {
            tracing::debug!(mustawa, "an initialisation level this build does not know");
            return;
        };
        MUSTAWA_BALAGH.store(
            u8::try_from(mustawa.qeema()).unwrap_or(u8::MAX),
            Ordering::Release,
        );
        if mustawa != MustawaTahyia::Sina {
            return;
        }

        let (Some(wasl), Some(khadim)) = (WASL.get(), KHADIM.get()) else {
            tracing::info!(
                marbuta = marbuta(),
                "the scene level was reached with no configuration installed; the patch's \
                 companion did not call thabbit_khadim"
            );
            return;
        };

        match khadim.hayyi(Some(wasl)) {
            Ok(hasila) => {
                tracing::info!(
                    khatt = ?hasila.khatt.nafidha,
                    sima = ?hasila.sima.nafidha,
                    ittijah = ?hasila.ittijah.nafidha,
                    thaqafa = ?hasila.thaqafa.nafidha,
                    khadim = hasila.khadim.as_deref().unwrap_or(""),
                    "the Taarib GDExtension applied the patch at the scene level"
                );
                let _ = NATIJA.set(hasila);
            },
            Err(khata) => tracing::warn!(
                sabab = %khata,
                "the Taarib GDExtension reached the scene level and applied nothing; the game \
                 runs in its own language"
            ),
        }
    });
}

/// Godot's per-level deinitialisation callback.
///
/// There is nothing to undo. This path installs no hook, patches no code and
/// rewrites no game file: what it did was construct a `FontFile`, hand it to the
/// theme — which took its own reference and now owns it — and write settings the
/// engine holds. Releasing the font here would be freeing a resource the theme
/// is still drawing with.
///
/// The callback exists anyway, because leaving it null would mean the engine had
/// no symmetric point at which to unload the library, and because the log line
/// is how a maintainer confirms the extension came out cleanly.
///
/// # Safety
///
/// As [`tahyia_mustawa`].
unsafe extern "C" fn inha_mustawa(bayanat: *mut c_void, mustawa: u32) {
    let _ = bayanat;
    let _ = std::panic::catch_unwind(|| {
        let Some(mustawa) = MustawaTahyia::min_qeema(mustawa) else {
            return;
        };
        if mustawa != MustawaTahyia::Sina {
            return;
        }
        tracing::info!(
            mustawa = mustawa.ism(),
            tubbiqat = NATIJA.get().is_some(),
            "the Taarib GDExtension is unloading; nothing is undone because nothing was \
             hooked, and the registered font belongs to the theme that took a reference to it"
        );
    });
}
