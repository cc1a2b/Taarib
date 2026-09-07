//! البداية — the two ways this payload is entered, and the one initialisation
//! behind both.
//!
//! Every other adapter in this product has one bootstrap. This one has two, and
//! they exist for two different loaders that can both be present in the same
//! game directory at the same time.
//!
//! | entry | who calls it | what it can reach |
//! | --- | --- | --- |
//! | `taarib_gdnative_init` / `taarib_gdnative_singleton` / `taarib_gdnative_terminate` | Godot 3, through the generated `taarib.gdnlib` | the engine, because Godot hands over its own API struct |
//! | `taarib_bidaya` | `taarib-mudkhal`, when this module was placed as a plain preload payload | its own directory, its patch, and nothing of the engine |
//!
//! `taarib-tathbeet`'s `tarkib` writes a `taarib.gdnlib` with
//! `symbol_prefix="taarib_"` and `singleton=true`, which is what makes the three
//! GDNative symbols the real bootstrap for a Godot 3 game: the engine resolves
//! them out of this library and calls them with the option structures its
//! loader fills in. The first of those structures carries the
//! `godot_gdnative_core_api_struct *` — the only route from inside a shipped
//! export template to `VisualServer`, because an export template is stripped and
//! monolithic and exports nothing an adapter could resolve by name.
//!
//! ## Why the two entries cannot fight
//!
//! There is a single process-wide state, and the path that can act on it claims
//! it with a compare-and-exchange rather than a store. The GDNative path is the
//! only one
//! that claims it, because it is the only one that can initialise anything; the
//! preload path reads it and never writes it, so a preload call that lands
//! before, during or after the engine's own call cannot lock the engine's path
//! out of a process it is the only entry able to take over. Being loaded both
//! ways therefore initialises exactly once, and the second arrival logs what the
//! first did and returns.
//!
//! ## What the preload entry can honestly do, and what it declines
//!
//! A Godot 3 export template links the whole engine into the game's own
//! executable. There is no `godot.dll`, no `libgodot.so`, no named module for
//! [`taarib_haqn::mawqi::qaidat_wahda`] to find, and no exported
//! `VisualServer::get_singleton` for [`taarib_haqn::mawqi::ramz_wahda`] to
//! resolve — which is precisely why GDNative passes an API struct instead of
//! letting extensions link against the engine. So `taarib_bidaya` cannot start
//! the takeover, and saying so by name is the correct outcome rather than a gap.
//!
//! It is not, however, an unchecked outcome. Before declining, the preload entry
//! uses exactly those two functions on **this** module — `qaidat_wahda` on the
//! path [`taarib_haqn::mawqi::masar_nafsi`] reports, then `ramz_wahda` for
//! `taarib_gdnative_init` — to establish that the path it is deferring to is
//! actually present in the binary that is running. A build whose GDNative entry
//! was stripped or garbage-collected has no working path at all, and that is a
//! **refusal** naming the missing symbol, not a decline. The decline is only
//! ever logged once the alternative has been shown to exist.
//!
//! ## The handover, and the number this file refuses to invent
//!
//! [`istila`](crate::istila) takes its addresses and its vtable slot from the
//! caller and contains no signature, no offset and no slot index, for the reason
//! its header gives at length. The same rule applies one level up: Godot's core
//! API struct is a long run of function pointers whose ordering belongs to a
//! specific `gdnative_api.json`, and a member index written from memory here
//! would call an unrelated engine function with a singleton lookup's arguments.
//!
//! So this file reads exactly the head of that struct — the five members
//! GDNative 1.0 defines before the function region, which are `type`, `version`,
//! `next`, `num_extensions` and `extensions` — verifies that it was handed a
//! core API of major version 1, and stops. Everything past the head is reached
//! through [`WaslGdnative::dalla`] and [`WaslGdnative::mufrad`], both of which
//! take the member's ordinal as an argument, exactly as
//! [`crate::istila::WaslRasm::min_jadwal`] takes a vtable slot and
//! `imtidad::thabbit_turuq` takes a method-hash table.
//!
//! The build-specific half of the takeover therefore arrives through
//! [`thabbit_istila`], which the patch's generated companion calls with the
//! three draw addresses and the layout policy for the engine build in front of
//! it. That is the same division `imtidad`'s `thabbit_khadim` already makes for
//! Godot 4, and for the same reason: this module can be verified against any
//! build, and a number that cannot be verified is not written here.
//!
//! ## The two halves, and why the delivery is not behind that seam
//!
//! [`thabbit_istila`] is one of two. [`thabbit_tawseel`] is the other, and it
//! carries the patch's messages and two paths — nothing about the engine binary,
//! no address, no vtable slot and no ordinal. That is not a symmetry for its own
//! sake; it is the difference between a Godot 3 game that shows Arabic and one
//! that does not.
//!
//! The takeover shapes the strings the game draws. It does not decide what those
//! strings are, and against an untranslated game it correctly declines every one
//! of them. The delivery is what makes them Arabic, through
//! [`crate::tawseel`]'s configuration rung: a `.translation` in Godot 3's own
//! resource format and an `override.cfg` in Godot 3's own setting namespace,
//! both of which the engine reads by itself. So [`sallim`] walks the delivery
//! first and unconditionally, and the takeover after it — and reports the pair,
//! because either alone is a game a player would call broken for a different
//! reason. See [`HalatBidaya::Naqisa`].
//!
//! The delivery cannot take effect on the launch that runs it: Godot 3 reads
//! `override.cfg` in `ProjectSettings::_setup` and loads `locale/translations`
//! in `TranslationServer::setup`, both inside `Main::setup`, before the module
//! system constructs any `GDNative` singleton. Writing them from here writes
//! them for the next launch, which is exactly what the Godot 4 path's
//! configuration rung does and is said out loud rather than implied.
//!
//! ## Refusal discipline
//!
//! Every decline, refusal and failure is one line in `<own dir>/taarib.sijill`,
//! named, capped at [`AQSA_SIJILL`]. A payload inside a game has no console and
//! no window, so the file is the only surface it has — except on the GDNative
//! path, where the init options carry `report_loading_error` and the same
//! sentence also reaches Godot's own error log, which is a surface the studio
//! already reads.
//!
//! Nothing here can unwind into the engine. All four entry points run inside
//! [`std::panic::catch_unwind`], because the workspace builds with
//! `panic = "unwind"` so that a fault inside somebody's game is caught at the
//! ABI edge, and an unwind across an `extern "C"` boundary would abort the
//! player's process instead.

use core::ffi::{c_char, c_uint, c_void};
use core::sync::atomic::{AtomicU8, Ordering};
use std::ffi::{CStr, CString};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use taarib_haqn::mawqi::{masar_nafsi, mujallad_nafsi, qaidat_wahda, ramz_wahda};
use taarib_ruqaa::qari::MalafRuqaa;

use crate::istila::{AhdafIstila, HalatIstila, Istila, Unwan, imsah_safahat};
use crate::khata::KhataGodot;
use crate::tawseel::{NatijatTawseel, TawseelThalith};

// ---------------------------------------------------------------------------
// The log
// ---------------------------------------------------------------------------

/// The file every decline, refusal and failure is named in, beside this module.
pub const ISM_SIJILL: &str = "taarib.sijill";

/// The cap on that log, past which it is removed before the next append.
///
/// A game may be launched hundreds of times with this module in place, and a
/// payload that grew a log without bound inside somebody's game directory would
/// be a payload that eventually fills their disk. Removing and starting again is
/// chosen over rotating: a second file beside a game's own data is a second
/// thing an uninstall has to know about.
pub const AQSA_SIJILL: u64 = 262_144;

/// The five outcomes the bootstrap contract distinguishes.
///
/// They are not severities and they do not collapse into one another. Each says
/// something different about what happened to the game process, and the log line
/// leads with the name so that a reader who has never seen this file can tell
/// "there was nothing to do here" from "something was installed and then taken
/// back out".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HalatBidaya {
    /// This is not that kind of game, or no patch is installed. The game was
    /// not touched.
    Mumtania,
    /// Something is wrong that the user could fix. The game was not touched.
    Marfuda,
    /// A hook or a read failed unexpectedly. Anything installed was removed.
    Fashila,
    /// One half of the Godot 3 path is in place and the other is not.
    ///
    /// The two halves are independent and both are required: the delivery puts
    /// Arabic into the game's strings, and the takeover shapes the strings the
    /// game draws. Delivery without takeover is a game showing Arabic as
    /// isolated letters in the wrong order; takeover without delivery is a
    /// takeover intercepting English and forwarding every call. Neither is a
    /// failure and neither is success, and reporting either as one of those
    /// would be the report that let an unreadable patch ship.
    Naqisa,
    /// The adapter has control.
    Badiya,
}

impl HalatBidaya {
    /// The word the log line leads with.
    #[must_use]
    pub const fn ism(self) -> &'static str {
        match self {
            Self::Mumtania => "declined",
            Self::Marfuda => "refused",
            Self::Fashila => "failed",
            Self::Naqisa => "partial",
            Self::Badiya => "started",
        }
    }
}

/// Appends one named line to `<own dir>/taarib.sijill` and answers `hala`.
///
/// Returning the state it was handed is what lets every decision site read as a
/// single expression: the log entry and the outcome are written once, together,
/// and cannot drift apart.
///
/// Every failure here is swallowed. This runs inside somebody's game, and a
/// payload that could not write its own log has no business interrupting a
/// launch to say so — the `tracing` line is emitted either way, for the case
/// where a developer is attached.
fn sajjil(mujallad: &Path, hala: HalatBidaya, satr: &str) -> HalatBidaya {
    match hala {
        HalatBidaya::Badiya => tracing::info!(hala = hala.ism(), satr, "taarib-godot"),
        HalatBidaya::Mumtania => tracing::debug!(hala = hala.ism(), satr, "taarib-godot"),
        HalatBidaya::Naqisa | HalatBidaya::Marfuda | HalatBidaya::Fashila => {
            tracing::warn!(hala = hala.ism(), satr, "taarib-godot");
        },
    }

    let masar = mujallad.join(ISM_SIJILL);
    if std::fs::metadata(&masar).is_ok_and(|bayan| bayan.len() > AQSA_SIJILL) {
        let _ = std::fs::remove_file(&masar);
    }
    let Ok(mut malaf) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&masar)
    else {
        return hala;
    };
    let _ = writeln!(malaf, "taarib-muhawwil-godot {}: {satr}", hala.ism());
    hala
}

// ---------------------------------------------------------------------------
// GDNative's own types
// ---------------------------------------------------------------------------

/// `godot_bool` — `typedef bool godot_bool`, one byte at the ABI.
///
/// Spelled as a `u8` rather than a `bool` for the reason every FFI boundary in
/// this workspace spells it that way: a C `_Bool` holding anything other than 0
/// or 1 is a value a Rust `bool` may not hold, and reading one through a `bool`
/// is undefined behaviour rather than a surprising `true`.
pub type MantiqGdnative = u8;

/// `void (*)(const godot_object *, const char *)` — GDNative's
/// `report_loading_error`, the engine-side error channel the init options carry.
pub type DallaBalaghKhata = unsafe extern "C" fn(maktaba: *const c_void, nass: *const c_char);

/// `void (*)(const godot_object *, const char *, godot_gdnative_api_version,
/// godot_gdnative_api_version)` — GDNative's `report_version_mismatch`.
///
/// Recorded and never called: this library binds no versioned extension, so it
/// has no mismatch to report, and a payload that called an engine callback it
/// had no news for would be a payload writing into somebody's error log for
/// nothing.
pub type DallaBalaghTanaqud = unsafe extern "C" fn(
    maktaba: *const c_void,
    ism: *const c_char,
    matlub: IsdarWasila,
    mawjud: IsdarWasila,
);

/// `godot_gdnative_api_version` — two 32-bit words, major then minor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct IsdarWasila {
    /// The API's major version. `1` for every GDNative in Godot 3.x.
    pub kabir: u32,
    /// The API's minor version: 0, 1, 2 and 3 exist across the 3.x line, each
    /// adding functions through the `next` chain rather than moving any.
    pub sagheer: u32,
}

/// The head of `godot_gdnative_core_api_struct`, and nothing past it.
///
/// ```text
/// RasWasila — 40 bytes on a 64-bit target, 24 on a 32-bit one
///
///   field           C declaration
///   naw             unsigned int type
///   isdar           godot_gdnative_api_version version
///   tali            const godot_gdnative_api_struct *next
///   adad_imtidadat  unsigned int num_extensions
///   imtidadat       const godot_gdnative_api_struct **extensions
/// ```
///
/// Every member after `imtidadat` is a function pointer, and this build
/// deliberately does not carry their order — see the module header. The size of
/// this structure is therefore also the offset of the first of them, on both
/// pointer widths, which is what [`WaslGdnative::dalla`] indexes from.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RasWasila {
    naw: c_uint,
    isdar: IsdarWasila,
    tali: *const c_void,
    adad_imtidadat: c_uint,
    imtidadat: *const *const c_void,
}

/// `GDNATIVE_CORE` — the `type` a core API struct declares.
///
/// The extension structs (`NATIVESCRIPT`, `PLUGINSCRIPT`, `ANDROID`, `ARVR`,
/// `VIDEODECODER`, `NET`) number upwards from this, and none of them carries
/// `godot_global_get_singleton`. A struct that is not this one is refused by
/// number rather than indexed hopefully.
const NAW_WASILA_ASAS: c_uint = 0;

/// The only GDNative API major version this build knows the head of.
const KABIR_WASILA: u32 = 1;

/// `godot_gdnative_init_options` — what Godot passes `taarib_gdnative_init`.
///
/// ```text
/// KhiyaratTahyia — 64 bytes on a 64-bit target, 8-byte aligned
///
///   field           C declaration
///   fi_muharrir     godot_bool in_editor
///   basmat_asas     uint64_t core_api_hash
///   basmat_muharrir uint64_t editor_api_hash
///   basmat_bila     uint64_t no_api_hash
///   balligh_tanaqud void (*report_version_mismatch)(...)
///   balligh_khata   void (*report_loading_error)(const godot_object *, const char *)
///   maktaba         godot_object *gd_native_library
///   wasila          const godot_gdnative_core_api_struct *api_struct
///   masar_maktaba   const godot_string *active_library_path
/// ```
///
/// The engine owns the allocation and it is valid only for the duration of the
/// call, which is why [`WaslGdnative`] copies out of it rather than keeping a
/// pointer to it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KhiyaratTahyia {
    /// Whether the editor, rather than a shipped game, loaded this library.
    pub fi_muharrir: MantiqGdnative,
    /// The hash of the core API the engine was built against.
    pub basmat_asas: u64,
    /// The hash of the editor API.
    pub basmat_muharrir: u64,
    /// The hash of the no-API build.
    pub basmat_bila: u64,
    /// Reports a version mismatch back to the engine's error log.
    pub balligh_tanaqud: Option<DallaBalaghTanaqud>,
    /// Reports a loading failure back to the engine's error log.
    pub balligh_khata: Option<DallaBalaghKhata>,
    /// The `GDNativeLibrary` resource being initialised, as an `Object *`.
    pub maktaba: *mut c_void,
    /// The core API struct: every engine function a GDNative library can reach.
    pub wasila: *const c_void,
    /// The library's own `res://` path, as an opaque `godot_string *`.
    ///
    /// Recorded as an address and never dereferenced. A `godot_string` is a
    /// copy-on-write structure whose layout is not part of the C ABI, and
    /// reading one means calling the API struct's own string functions — which
    /// would need a member index this file refuses to invent. The directory this
    /// payload was loaded from comes from
    /// [`taarib_haqn::mawqi::mujallad_nafsi`], which needs nothing from the
    /// engine and is what the bootstrap contract names.
    pub masar_maktaba: *const c_void,
}

/// `godot_gdnative_terminate_options` — what Godot passes
/// `taarib_gdnative_terminate`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct KhiyaratInha {
    /// Whether the editor, rather than a shipped game, is unloading this
    /// library.
    pub fi_muharrir: MantiqGdnative,
}

// ---------------------------------------------------------------------------
// The engine binding
// ---------------------------------------------------------------------------

/// The furthest member [`WaslGdnative::dalla`] will read out of the core API
/// struct.
///
/// GDNative 1.0's core API declares fewer than four hundred functions and every
/// later minor version adds its own through the `next` chain rather than
/// extending this one, so this is a bound on a mistake rather than a description
/// of the engine. An ordinal that lands here did not come from a real
/// `gdnative_api.json`, and stopping is better than dereferencing whatever
/// follows the struct in read-only data.
pub const AQSA_AADA_WASILA: usize = 1024;

/// Everything the init options carried, as scalars.
///
/// Every field is an address, an integer, a flag or a function pointer, which is
/// what makes this [`Send`] and [`Sync`] without an `unsafe impl` — the same
/// property [`crate::istila::HalatIstila`] is built for, and for the same
/// reason: it is published once into a static and read afterwards by whatever
/// thread the engine happens to be on.
#[derive(Debug, Clone, Copy)]
pub struct WaslGdnative {
    wasila: Unwan,
    tali: Option<Unwan>,
    imtidadat: Option<Unwan>,
    maktaba: Option<Unwan>,
    masar_maktaba: Option<Unwan>,
    isdar: IsdarWasila,
    naw: c_uint,
    adad_imtidadat: c_uint,
    basmat_asas: u64,
    fi_muharrir: bool,
    balligh: Option<DallaBalaghKhata>,
}

impl WaslGdnative {
    /// Reads and checks the init options Godot passed.
    ///
    /// The head of the core API struct is read here and nowhere else, and the
    /// two things it is read for are the two things that can be checked without
    /// knowing a single member offset: that this is the **core** API rather than
    /// one of the extension structs, and that its major version is the one whose
    /// head this build knows the shape of.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::ImtidadMarfud`] when the options carry a null API struct,
    /// when the struct declares a `type` other than `GDNATIVE_CORE`, or when it
    /// declares a major version other than 1 — each naming what was found, so a
    /// future GDNative that changes either is reported rather than indexed.
    ///
    /// # Safety
    ///
    /// `khiyarat` must be the live `godot_gdnative_init_options` Godot's own
    /// loader filled in, and its `api_struct` must be either null or the
    /// engine's `godot_gdnative_core_api_struct`. No other caller is supported.
    pub unsafe fn min_khiyarat(khiyarat: &KhiyaratTahyia) -> Result<Self, KhataGodot> {
        let Some(wasila) = Unwan::min_muashir(khiyarat.wasila) else {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: "Godot passed a null GDNative API struct, which no released engine \
                        does, and there is no route to VisualServer without it"
                    .to_owned(),
            });
        };

        // SAFETY: the caller guarantees `wasila` is the engine's core API
        // struct, whose first five members are exactly `RasWasila` in exactly
        // this order on every GDNative 1.x. The read is of that head alone.
        let ras = unsafe { wasila.muashir().cast::<RasWasila>().read() };

        if ras.naw != NAW_WASILA_ASAS {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "the GDNative struct Godot passed declares type {}, and the core API is \
                     type {NAW_WASILA_ASAS}; an extension struct carries none of the \
                     functions this adapter needs and was refused rather than indexed",
                    ras.naw
                ),
            });
        }
        if ras.isdar.kabir != KABIR_WASILA {
            return Err(KhataGodot::ImtidadMarfud {
                sabab: format!(
                    "this is GDNative API {}.{}, and this build knows the layout of the \
                     {KABIR_WASILA}.x core struct only; a major version that moved the \
                     header would move every member index with it",
                    ras.isdar.kabir, ras.isdar.sagheer
                ),
            });
        }

        Ok(Self {
            wasila,
            tali: Unwan::min_muashir(ras.tali),
            imtidadat: Unwan::min_muashir(ras.imtidadat.cast::<c_void>()),
            maktaba: Unwan::min_muashir(khiyarat.maktaba.cast_const()),
            masar_maktaba: Unwan::min_muashir(khiyarat.masar_maktaba),
            isdar: ras.isdar,
            naw: ras.naw,
            adad_imtidadat: ras.adad_imtidadat,
            basmat_asas: khiyarat.basmat_asas,
            fi_muharrir: khiyarat.fi_muharrir != 0,
            balligh: khiyarat.balligh_khata,
        })
    }

    /// The next API struct in the engine's chain, when there is one.
    ///
    /// GDNative 1.1, 1.2 and 1.3 each hang off this pointer rather than
    /// extending the 1.0 struct, which is what makes the 1.0 member ordinals
    /// stable across all of Godot 3.x. Recorded so a caller that needs a later
    /// minor version's function can walk to it, and never walked here: this
    /// module reads one head and stops.
    #[must_use]
    pub const fn tali(self) -> Option<Unwan> {
        self.tali
    }

    /// The array of extension API structs the engine offered.
    ///
    /// [`WaslGdnative::adad_imtidadat`] entries long. Recorded and not read:
    /// none of the extension APIs — NativeScript, PluginScript, ARVR, NET —
    /// carries anything a text takeover needs.
    #[must_use]
    pub const fn imtidadat(self) -> Option<Unwan> {
        self.imtidadat
    }

    /// The `type` the core API struct declared, which is `GDNATIVE_CORE`.
    ///
    /// Kept after the check rather than discarded, because a diagnostics bundle
    /// that says which struct answered is a bundle that can be read against a
    /// future GDNative that renumbers them.
    #[must_use]
    pub const fn naw(self) -> u32 {
        self.naw
    }

    /// The core API struct's address.
    #[must_use]
    pub const fn wasila(self) -> Unwan {
        self.wasila
    }

    /// The `GDNativeLibrary` resource this library was loaded as.
    #[must_use]
    pub const fn maktaba(self) -> Option<Unwan> {
        self.maktaba
    }

    /// The `godot_string *` naming this library's `res://` path, unread.
    #[must_use]
    pub const fn masar_maktaba(self) -> Option<Unwan> {
        self.masar_maktaba
    }

    /// The core API version the engine declared.
    #[must_use]
    pub const fn isdar(self) -> IsdarWasila {
        self.isdar
    }

    /// How many extension structs the engine offered alongside the core API.
    #[must_use]
    pub const fn adad_imtidadat(self) -> u32 {
        self.adad_imtidadat
    }

    /// The hash of the core API the engine was built against.
    ///
    /// Carried for the diagnostics bundle: two games reporting the same hash
    /// were built from the same API, which is the fact that decides whether one
    /// game's member ordinals can be trusted for another.
    #[must_use]
    pub const fn basmat_asas(self) -> u64 {
        self.basmat_asas
    }

    /// Whether the editor, rather than a shipped game, loaded this library.
    #[must_use]
    pub const fn fi_muharrir(self) -> bool {
        self.fi_muharrir
    }

    /// Sends one sentence to Godot's own error log, when the engine gave a
    /// channel for it.
    ///
    /// Silent when the engine passed no callback, and silent for a message
    /// carrying an interior NUL — a diagnostic is not worth a second failure
    /// path inside somebody's game.
    pub fn balligh(self, nass: &str) {
        let Some(daala) = self.balligh else { return };
        let Ok(risala) = CString::new(nass) else {
            return;
        };
        let maktaba = self.maktaba.map_or(core::ptr::null(), Unwan::muashir);
        // SAFETY: `daala` is the `report_loading_error` Godot itself put in the
        // init options, whose C signature is
        // `void (*)(const godot_object *, const char *)`. `maktaba` is the
        // library object from the same structure or null, both of which that
        // callback accepts, and `risala` is NUL-terminated and outlives the
        // call.
        unsafe { daala(maktaba, risala.as_ptr()) };
    }

    /// The core API member at ordinal `fahras`, counted from the first function
    /// pointer.
    ///
    /// The ordinal is an argument because it is not knowable from here: it is a
    /// property of the exact `gdnative_api.json` the engine was generated from,
    /// and a constant written into this file would name a different function on
    /// any build it was not measured against — which is a call into unrelated
    /// engine code with this adapter's arguments. That is the same refusal
    /// [`crate::istila::WaslRasm::min_jadwal`] makes about vtable slots.
    ///
    /// # Errors
    ///
    /// [`KhataGodot::RasmGhayrMutah`] when `fahras` is past
    /// [`AQSA_AADA_WASILA`], or when the member holds null.
    ///
    /// # Safety
    ///
    /// `fahras` must be an ordinal that exists in the core API struct of the
    /// engine build being run, counted from `godot_color_new_rgba` — the first
    /// member after the head — as zero. An ordinal past the end of a real struct
    /// reads whatever follows it, and [`AQSA_AADA_WASILA`] bounds the damage
    /// without being able to prevent it.
    pub unsafe fn dalla(self, fahras: usize) -> Result<Unwan, KhataGodot> {
        if fahras >= AQSA_AADA_WASILA {
            return Err(KhataGodot::RasmGhayrMutah {
                sabab: format!(
                    "core API ordinal {fahras} is past the {AQSA_AADA_WASILA} this build \
                     will read, which is an ordinal that did not come from a real \
                     gdnative_api.json"
                ),
            });
        }
        // SAFETY: `min_khiyarat` established that this address is a GDNative 1.x
        // core API struct, whose head is `RasWasila` and whose function region
        // begins immediately after it — at `size_of::<RasWasila>()` on both
        // pointer widths, because the head's last member is pointer-sized and
        // pointer-aligned. The caller guarantees `fahras` names a member of that
        // region, and the read is of one pointer at a bounded offset.
        let khaam = unsafe {
            self.wasila
                .muashir()
                .cast::<u8>()
                .add(size_of::<RasWasila>())
                .cast::<*const c_void>()
                .add(fahras)
                .read()
        };
        Unwan::min_muashir(khaam).ok_or_else(|| KhataGodot::RasmGhayrMutah {
            sabab: format!("core API ordinal {fahras} holds null"),
        })
    }

    /// An engine singleton by name, through the caller's ordinal for
    /// `godot_global_get_singleton`.
    ///
    /// This is the one call the Godot 3 takeover cannot start without: it is how
    /// the `VisualServer` singleton — [`crate::istila::WaslRasm`]'s `this` — is
    /// obtained inside a stripped, monolithic export template that exports no
    /// symbol an adapter could resolve.
    ///
    /// # Errors
    ///
    /// Whatever [`WaslGdnative::dalla`] refuses for the ordinal, and
    /// [`KhataGodot::RasmGhayrMutah`] when the engine answers null — which is
    /// what it does for a singleton name it does not carry, and is a fact about
    /// the name rather than a fault in the call.
    ///
    /// # Safety
    ///
    /// `fahras` must be the ordinal of `godot_global_get_singleton`, whose C
    /// signature is `godot_object *(*)(char *)`, in the core API struct of the
    /// engine build being run. An ordinal naming any other member calls a
    /// different function with a `char *` in its first argument register.
    pub unsafe fn mufrad(self, fahras: usize, ism: &CStr) -> Result<Unwan, KhataGodot> {
        // SAFETY: the caller guarantees `fahras` names a member of the function
        // region, which is `dalla`'s whole contract.
        let unwan = unsafe { self.dalla(fahras) }?;

        type DallaMufrad = unsafe extern "C" fn(ism: *mut c_char) -> *mut c_void;
        // SAFETY: the caller guarantees the member at `fahras` is
        // `godot_global_get_singleton`. Transmuting a code address to a function
        // pointer of the matching signature is the only way to call it, and the
        // signature here is that function's, spelled as `gdnative.h` spells it.
        let daala = unsafe { core::mem::transmute::<*const c_void, DallaMufrad>(unwan.muashir()) };

        // SAFETY: `daala` is the engine's own singleton lookup, established
        // immediately above, and `ism` is a NUL-terminated name that outlives
        // the call. The engine does not write through the pointer despite the
        // missing `const` in its declaration.
        let kaain = unsafe { daala(ism.as_ptr().cast_mut()) };

        Unwan::min_muashir(kaain).ok_or_else(|| KhataGodot::RasmGhayrMutah {
            sabab: format!(
                "the engine has no singleton named {}, so there is nowhere to emit glyphs",
                ism.to_string_lossy()
            ),
        })
    }
}

// ---------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------

/// Nothing has been attempted.
const HALA_BIKR: u8 = 0;
/// A claim is held and the initialisation is running.
const HALA_JARIYA: u8 = 1;
/// The engine is bound and the handover has not happened yet.
const HALA_MARBUTA: u8 = 2;
/// The takeover is installed.
const HALA_BADIYA: u8 = 3;
/// This is not that kind of load, or there was nothing to do.
const HALA_MUMTANIA: u8 = 4;
/// Something the user could fix is wrong.
const HALA_MARFUDA: u8 = 5;
/// Something failed unexpectedly and whatever was installed came back out.
const HALA_FASHILA: u8 = 6;
/// The engine unloaded this library and every hook was removed.
const HALA_MUNTAHIYA: u8 = 7;

/// The one state both entry paths move through.
static HALA: AtomicU8 = AtomicU8::new(HALA_BIKR);

/// The engine binding, cleared at termination so no dangling API struct address
/// outlives the engine that owned it.
static WASL: Mutex<Option<WaslGdnative>> = Mutex::new(None);

/// The patches held open beside this module, with the paths they came from.
///
/// Behind a lock rather than in a `OnceLock` for one reason: a memory map keeps
/// a file open, and on Windows an open mapping is a file the uninstaller cannot
/// delete. Termination has to be able to drop these, and a `OnceLock` cannot be
/// emptied.
static RUQAA: Mutex<Vec<(PathBuf, MalafRuqaa)>> = Mutex::new(Vec::new());

/// The build-specific half of the takeover, installed by the patch's companion.
static TAHYIA: Mutex<Option<(AhdafIstila, HalatIstila)>> = Mutex::new(None);

/// The delivery — the translated text and where Godot 3 is to load it from.
///
/// Separate from [`TAHYIA`] because the two halves are independent: this one
/// needs no address, no vtable slot and nothing about the engine build, and it
/// is the half that decides whether the game's strings are Arabic at all.
static TAWSEEL: Mutex<Option<TawseelThalith>> = Mutex::new(None);

/// What the delivery achieved, once it has been walked.
///
/// Also the once-only guard: [`sallim`] runs at both `GDNative` moments and the
/// delivery writes files, so it is walked on whichever of them first has a
/// configuration and read from a record afterwards.
static NATIJAT_TAWSEEL: Mutex<Option<NatijatTawseel>> = Mutex::new(None);

/// The installed takeover. Holding it means the process is patched; dropping it
/// means it is not.
static ISTILA: Mutex<Option<Istila>> = Mutex::new(None);

/// A state's name, for the log line that reports one.
const fn ism_hala(hala: u8) -> &'static str {
    match hala {
        HALA_BIKR => "untouched",
        HALA_JARIYA => "initialising",
        HALA_MARBUTA => "bound",
        HALA_BADIYA => "started",
        HALA_MUMTANIA => "declined",
        HALA_MARFUDA => "refused",
        HALA_FASHILA => "failed",
        HALA_MUNTAHIYA => "terminated",
        _ => "unrecorded",
    }
}

/// Where this module's initialisation currently stands, as one word.
///
/// For the capability report and the diagnostics bundle. The words are the ones
/// [`HalatBidaya::ism`] uses, plus the three the two-step GDNative entry needs:
/// `untouched`, `bound` and `terminated`.
#[must_use]
pub fn marhala() -> &'static str {
    ism_hala(HALA.load(Ordering::Acquire))
}

/// Whether the engine is bound and the handover has not happened yet.
#[must_use]
pub fn marbuta() -> bool {
    HALA.load(Ordering::Acquire) == HALA_MARBUTA
}

/// Whether the takeover is installed and drawing.
#[must_use]
pub fn badiya() -> bool {
    HALA.load(Ordering::Acquire) == HALA_BADIYA
}

/// The engine binding, when one was made.
#[must_use]
pub fn wasl() -> Option<WaslGdnative> {
    *WASL.lock()
}

/// Installs the addresses and the layout policy the takeover runs with.
///
/// Called by the patch's generated companion, which is the only thing that has
/// the engine build's own draw addresses, its `VisualServer` vtable slot and the
/// pixel size the patch was compiled at. Returns `false` when a configuration is
/// already installed, which is not an error and not a replacement: the first one
/// wins, because a second arriving after the hooks are in would be a second
/// answer to a question the detours can only read once.
///
/// The handover happens at whichever of the two GDNative entries runs next, so a
/// companion that installs before `taarib_gdnative_init` and one that installs
/// between `init` and `taarib_gdnative_singleton` both take effect.
pub fn thabbit_istila(ahdaf: AhdafIstila, hala: HalatIstila) -> bool {
    let mut khana = TAHYIA.lock();
    if khana.is_some() {
        return false;
    }
    *khana = Some((ahdaf, hala));
    true
}

/// Installs the delivery: the translated text and where Godot 3 loads it from.
///
/// The other half of the Godot 3 path, and the half that has nothing
/// build-specific in it — it needs no address, no vtable slot and nothing about
/// the engine build, only the patch's messages and two paths. That is why it is
/// a separate seam from [`thabbit_istila`] rather than a field on it: a game
/// whose export template cannot be hooked still gets its text, and a game whose
/// text has not been translated yet still gets a correctly installed takeover
/// that has nothing to shape.
///
/// Returns `false` when a delivery is already installed. The first one wins, for
/// [`thabbit_istila`]'s reason: it writes files, and two configurations would be
/// two answers to where the resource lives.
///
/// The delivery is walked at whichever of the two `GDNative` entries runs next,
/// and exactly once however many times they run.
pub fn thabbit_tawseel(tawseel: TawseelThalith) -> bool {
    let mut khana = TAWSEEL.lock();
    if khana.is_some() {
        return false;
    }
    *khana = Some(tawseel);
    true
}

/// What the delivery achieved, once it has been walked.
#[must_use]
pub fn natijat_tawseel() -> Option<NatijatTawseel> {
    NATIJAT_TAWSEEL.lock().clone()
}

/// Reads the patches this bootstrap holds open.
///
/// Handed to a closure rather than returned because the mappings live behind the
/// lock that termination takes to drop them, and a borrow that outlived that
/// lock would be a borrow of an unmapped file.
pub fn iqra_ruqaa<T>(daala: impl FnOnce(&[(PathBuf, MalafRuqaa)]) -> T) -> T {
    daala(RUQAA.lock().as_slice())
}

// ---------------------------------------------------------------------------
// The patch
// ---------------------------------------------------------------------------

/// The extension `taarib_tathbeet::masar_tathbeet` writes patches under.
const LAHIQAT_RUQAA: &str = "ruqaa";

/// How many patches this bootstrap will map from one directory.
///
/// A game's Taarib directory holds one patch, or a small number when a patch is
/// split. A directory holding more than this is a directory something else
/// filled, and mapping all of them would be a launch that pages in whatever was
/// put there.
const AQSA_RUQAA: usize = 16;

/// Opens every `*.ruqaa` beside this module, in name order.
///
/// Returns the ones that validated and a sentence for each one that did not, so
/// a patch that is corrupt is named in the log rather than silently becoming
/// "no patch installed" — the two are different problems with different fixes.
fn ruqaa_mujallad(mujallad: &Path) -> (Vec<(PathBuf, MalafRuqaa)>, Vec<String>) {
    let mut shakawa = Vec::new();
    let Ok(qira) = std::fs::read_dir(mujallad) else {
        shakawa.push(format!(
            "{} could not be listed, so no patch could be found beside this module",
            mujallad.display()
        ));
        return (Vec::new(), shakawa);
    };

    let mut masarat: Vec<PathBuf> = qira
        .filter_map(Result::ok)
        .map(|madkhal| madkhal.path())
        .filter(|masar| {
            masar.is_file()
                && masar
                    .extension()
                    .is_some_and(|lahiqa| lahiqa.eq_ignore_ascii_case(LAHIQAT_RUQAA))
        })
        .collect();
    masarat.sort();

    if masarat.len() > AQSA_RUQAA {
        shakawa.push(format!(
            "{} holds {} .ruqaa files and this build maps {AQSA_RUQAA}; the rest were left \
             unopened rather than paged in at launch",
            mujallad.display(),
            masarat.len()
        ));
        masarat.truncate(AQSA_RUQAA);
    }

    let mut maftuha = Vec::new();
    for masar in masarat {
        match MalafRuqaa::iftah(&masar) {
            Ok(malaf) => maftuha.push((masar, malaf)),
            Err(khata) => shakawa.push(format!("{} was not read: {khata}", masar.display())),
        }
    }
    (maftuha, shakawa)
}

// ---------------------------------------------------------------------------
// The preload entry's own check
// ---------------------------------------------------------------------------

/// The GDNative entry Godot resolves out of this library, as `taarib.gdnlib`'s
/// `symbol_prefix="taarib_"` spells it.
const ISM_TAHYIA_GDNATIVE: &str = "taarib_gdnative_init";

/// Whether the path this module defers to is actually present in this binary.
///
/// [`taarib_haqn::mawqi::qaidat_wahda`] on this module's own full path — which
/// both platform loaders accept as a module name — and then
/// [`taarib_haqn::mawqi::ramz_wahda`] for the GDNative entry. A build that
/// stripped or garbage-collected that symbol has no working path into a Godot 3
/// game at all, and the preload entry has to say so rather than defer to
/// something that is not there.
fn madkhal_gdnlib() -> Option<Unwan> {
    let masar = masar_nafsi()?;
    let ism = masar.to_str()?;
    let qaida = qaidat_wahda(ism)?;
    // SAFETY: `qaida` is the base `qaidat_wahda` just reported for this module's
    // own path, so it is a module currently mapped in this process — which is
    // exactly `ramz_wahda`'s contract.
    let unwan = unsafe { ramz_wahda(qaida, ISM_TAHYIA_GDNATIVE) }?;
    Unwan::min_muashir(unwan)
}

// ---------------------------------------------------------------------------
// The shared initialisation
// ---------------------------------------------------------------------------

/// Which loader entered this module.
#[derive(Debug)]
enum MasdarBidaya {
    /// `taarib-mudkhal` opened this module as a plain preload payload.
    Hamula,
    /// Godot resolved this module's GDNative entry through `taarib.gdnlib`.
    Gdnlib(WaslGdnative),
}

/// The one initialisation both entry paths run.
///
/// Step 1 of the bootstrap contract is first and is common: the directory this
/// payload was loaded from, resolved from an address inside this module and
/// never from the process's arguments or its working directory, because both of
/// those belong to the game or its launcher. Without it there is nowhere to log
/// and nowhere to look for a patch, and the only surface left is `tracing`.
fn ibda(masdar: MasdarBidaya) -> HalatBidaya {
    let Some(mujallad) = mujallad_nafsi() else {
        tracing::warn!(
            "the Godot payload could not resolve its own directory, so it has no patch to \
             read and no log to write; the game is untouched"
        );
        return HalatBidaya::Fashila;
    };
    match masdar {
        MasdarBidaya::Hamula => ibda_hamula(&mujallad),
        MasdarBidaya::Gdnlib(wasl) => ibda_gdnlib(&mujallad, wasl),
    }
}

/// The preload path: check that the real path exists, then decline by name.
///
/// This function never writes the shared state. It cannot initialise anything,
/// so claiming it would be claiming it away from the entry that can — and
/// `taarib-mudkhal` loads its payloads on its own thread, so a claim here could
/// genuinely race Godot's own call and win.
fn ibda_hamula(mujallad: &Path) -> HalatBidaya {
    let hala = HALA.load(Ordering::Acquire);
    if hala != HALA_BIKR {
        return sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            &format!(
                "loaded as a preload payload into a process whose Godot initialisation is \
                 already {}; that initialisation stands and this call changed nothing",
                ism_hala(hala)
            ),
        );
    }

    let (ruqaa, shakawa) = ruqaa_mujallad(mujallad);
    for shakwa in &shakawa {
        let _ = sajjil(mujallad, HalatBidaya::Marfuda, shakwa);
    }
    if ruqaa.is_empty() {
        return sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            "no .ruqaa patch is installed beside this module, so there is nothing to apply \
             through either entry point",
        );
    }
    // Dropped here on purpose: this path installs nothing, and a payload that
    // held a game's patch mapped for the life of a process it never touched
    // would be a file the uninstaller cannot delete.
    let adad = ruqaa.len();
    drop(ruqaa);

    let Some(unwan) = madkhal_gdnlib() else {
        return sajjil(
            mujallad,
            HalatBidaya::Marfuda,
            "loaded as a preload payload, and this build exports no \
             taarib_gdnative_init: the taarib.gdnlib path that takes Godot 3 over cannot \
             reach this module either, so no path into the engine remains. Reinstall the \
             Godot component.",
        );
    };

    sajjil(
        mujallad,
        HalatBidaya::Mumtania,
        &format!(
            "loaded as a preload payload; this engine is taken over through taarib.gdnlib, \
             whose entry taarib_gdnative_init is exported at {:#x} and is what Godot calls. \
             {adad} patch(es) are installed and will be read on that path.",
            unwan.raqm()
        ),
    )
}

/// The GDNative path: claim the state, bind the engine, open the patch, then try
/// the handover.
fn ibda_gdnlib(mujallad: &Path, wasl: WaslGdnative) -> HalatBidaya {
    if HALA
        .compare_exchange(HALA_BIKR, HALA_JARIYA, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            &format!(
                "taarib_gdnative_init ran again in a process whose initialisation is already \
                 {}; the first one stands and this call changed nothing",
                ism_hala(HALA.load(Ordering::Acquire))
            ),
        );
    }

    if wasl.fi_muharrir() {
        HALA.store(HALA_MUMTANIA, Ordering::Release);
        let satr = "Godot loaded this library in the editor. A game patch takes over a \
                    running game's text drawing, and doing that to the editor's own \
                    interface is not what it is for.";
        wasl.balligh(satr);
        return sajjil(mujallad, HalatBidaya::Mumtania, satr);
    }

    let (ruqaa, shakawa) = ruqaa_mujallad(mujallad);
    for shakwa in &shakawa {
        let _ = sajjil(mujallad, HalatBidaya::Marfuda, shakwa);
        wasl.balligh(shakwa);
    }
    if ruqaa.is_empty() {
        HALA.store(HALA_MUMTANIA, Ordering::Release);
        let satr = "no .ruqaa patch is installed beside this module, so there is nothing to \
                    apply and the game keeps its own text";
        wasl.balligh(satr);
        return sajjil(mujallad, HalatBidaya::Mumtania, satr);
    }

    let adad = ruqaa.len();
    *RUQAA.lock() = ruqaa;
    *WASL.lock() = Some(wasl);
    HALA.store(HALA_MARBUTA, Ordering::Release);
    tracing::info!(
        isdar = format!("{}.{}", wasl.isdar().kabir, wasl.isdar().sagheer),
        imtidadat = wasl.adad_imtidadat(),
        basma = format!("{:#x}", wasl.basmat_asas()),
        ruqaa = adad,
        "the Taarib Godot 3 payload bound GDNative's core API"
    );

    sallim(mujallad)
}

// ---------------------------------------------------------------------------
// The handover
// ---------------------------------------------------------------------------

/// Walks the delivery, once, and says whether the game's text will be Arabic.
///
/// Runs before the takeover on purpose. The two halves do not depend on each
/// other, and this one is the one that can succeed on any build: it writes the
/// generated `.translation` and the project override, neither of which needs an
/// address, a vtable slot or anything else about the engine binary. A game whose
/// export template cannot be hooked still gets its text out of this, and the
/// report then says plainly that the text is there and is not shaped.
///
/// What it cannot do is take effect on *this* launch. Godot 3 reads
/// `override.cfg` and loads `locale/translations` inside
/// `ProjectSettings::_setup` and `TranslationServer::setup`, both of which run
/// in `Main::setup` before the module system constructs a `GDNative` singleton —
/// so by the time this code exists, the locale and the translation list for the
/// running process have already been decided. Writing them here is writing them
/// for the next launch, exactly as the Godot 4 path's configuration rung does,
/// and the log line says so rather than implying the current launch changed.
fn awsil(mujallad: &Path, wasl: WaslGdnative) -> bool {
    if let Some(sabiq) = NATIJAT_TAWSEEL.lock().as_ref() {
        return sabiq.wusul();
    }
    let tawseel = TAWSEEL.lock().clone();
    let Some(tawseel) = tawseel else {
        let _ = sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            "no delivery configuration was installed: the patch's companion did not call \
             thabbit_tawseel with the patch's messages and a place for the generated \
             .translation. Nothing was written and the game keeps its own strings.",
        );
        return false;
    };

    match tawseel.hayyi() {
        Ok(natija) => {
            let wusul = natija.wusul();
            for sijill in [&natija.mawrid, &natija.thaqafa] {
                for rutba in &sijill.rutab {
                    tracing::debug!(
                        rutba = rutba.rutba.ism(),
                        muakkada = rutba.muakkada,
                        "{}",
                        rutba.mulahaza
                    );
                }
            }
            let satr = format!(
                "the Godot 3 delivery is installed: {} and {}. Godot reads both during its \
                 own startup, so this launch is unchanged and the next one starts in Arabic.",
                tawseel.mawdi().mutlaq_masar().display(),
                tawseel.tajawuz().masar().display()
            );
            *NATIJAT_TAWSEEL.lock() = Some(natija);
            wasl.balligh(&satr);
            let _ = sajjil(mujallad, HalatBidaya::Badiya, &satr);
            wusul
        },
        Err(khata) => {
            let satr = format!("the Godot 3 delivery installed nothing: {khata}");
            wasl.balligh(&satr);
            let _ = sajjil(mujallad, HalatBidaya::Marfuda, &satr);
            false
        },
    }
}

/// Hands over to the two halves of the Godot 3 path: the delivery, then
/// [`Istila`].
///
/// Called at both GDNative moments — once from `taarib_gdnative_init` and once
/// from `taarib_gdnative_singleton` — because the companion that supplies either
/// configuration may install it either side of the first. The state guard makes
/// the second call a no-op once the takeover has gone in, and
/// [`NATIJAT_TAWSEEL`] makes the delivery run exactly once however many times
/// this does.
///
/// The two halves are reported separately and the outcome is the pair, because
/// each without the other is a game a player would call broken for a different
/// reason. See [`HalatBidaya::Naqisa`].
fn sallim(mujallad: &Path) -> HalatBidaya {
    let hala = HALA.load(Ordering::Acquire);
    if hala != HALA_MARBUTA {
        return sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            &format!(
                "the handover was reached with the initialisation at {}, so nothing was \
                 installed and nothing was changed",
                ism_hala(hala)
            ),
        );
    }

    let Some(wasl) = wasl() else {
        HALA.store(HALA_FASHILA, Ordering::Release);
        return sajjil(
            mujallad,
            HalatBidaya::Fashila,
            "the initialisation reported the engine as bound and no binding was recorded, \
             which cannot happen in this module's own ordering; nothing was installed",
        );
    };

    let wusul = awsil(mujallad, wasl);

    let tahyia = *TAHYIA.lock();
    let Some((ahdaf, hala_istila)) = tahyia else {
        if wusul {
            let satr = "the delivery is installed and no takeover configuration was: the \
                        patch's companion did not call thabbit_istila with this build's \
                        Font::draw, Font::draw_char and Font::get_string_size addresses, its \
                        VisualServer binding and the patch's layout policy. The game's text \
                        will be Arabic from the next launch and Godot 3 will draw it \
                        unshaped — every letter isolated, left to right — until a takeover \
                        or the glyph transport is in place.";
            wasl.balligh(satr);
            return sajjil(mujallad, HalatBidaya::Naqisa, satr);
        }
        let satr = "the engine is bound and neither half of the Godot 3 path was installed: \
                    the patch's companion called neither thabbit_tawseel, which delivers the \
                    translated text, nor thabbit_istila with this build's Font::draw, \
                    Font::draw_char and Font::get_string_size addresses, its VisualServer \
                    binding and the patch's layout policy. Nothing was written, nothing was \
                    hooked, and the game is exactly what it was.";
        wasl.balligh(satr);
        return sajjil(mujallad, HalatBidaya::Mumtania, satr);
    };

    let mut istila = match Istila::rakkib(&ahdaf, hala_istila) {
        Ok(istila) => istila,
        Err(khata) => {
            HALA.store(HALA_FASHILA, Ordering::Release);
            let satr = format!(
                "the takeover refused its own configuration before anything was patched: \
                 {khata}"
            );
            wasl.balligh(&satr);
            return sajjil(mujallad, HalatBidaya::Fashila, &satr);
        },
    };

    for tanbeeh in istila.tanbeehat() {
        let _ = sajjil(mujallad, HalatBidaya::Marfuda, &tanbeeh.to_string());
    }

    if istila.nuzul_matlub() {
        // Neither draw entry point was taken over, so this is not a partial
        // takeover — it is a takeover that would measure text the engine still
        // draws unshaped. Everything that did go in comes back out here, before
        // returning, rather than being left for a `Drop` at some later moment.
        for khata in istila.azil() {
            let _ = sajjil(mujallad, HalatBidaya::Fashila, &khata.to_string());
        }
        drop(istila);
        HALA.store(HALA_MUMTANIA, Ordering::Release);
        let satr = if wusul {
            "no draw entry point could be taken over, so every interception was removed \
             again and the process is byte-identical. The delivery stands: this game's text \
             is Arabic from the next launch, and Godot 3 draws it unshaped until the glyph \
             transport in naql — which needs no hook and no address — is generated for this \
             build."
        } else {
            "no draw entry point could be taken over, so every interception was removed \
             again and the process is byte-identical; the glyph transport in naql is the \
             remaining path for this build"
        };
        wasl.balligh(satr);
        return sajjil(
            mujallad,
            if wusul {
                HalatBidaya::Naqisa
            } else {
                HalatBidaya::Mumtania
            },
            satr,
        );
    }

    let adad = istila.adad();
    let qadim = ISTILA.lock().replace(istila);
    if let Some(qadim) = qadim {
        // Cannot happen while the state guard above holds, and is written as a
        // removal rather than an assertion because the alternative inside a
        // player's game is a leaked set of live detours into a module that is
        // about to unload.
        drop(qadim);
        let _ = sajjil(
            mujallad,
            HalatBidaya::Fashila,
            "a second takeover replaced one that was already installed; the earlier \
             interceptions were removed rather than left behind",
        );
    }
    HALA.store(HALA_BADIYA, Ordering::Release);

    if !wusul {
        // A takeover with nothing to shape. It is installed and correct and the
        // player sees no difference, because `istila` only claims a string that
        // contains Arabic and the game's strings are still its own. Reported as
        // the partial it is rather than as a success, since "the takeover is
        // installed" reads to everyone who is not holding this file as "the
        // patch works".
        let satr = format!(
            "the Godot 3 takeover is installed with {adad} interception(s) and no delivery \
             is: the patch's companion did not call thabbit_tawseel, so the strings this \
             game draws are still its own and every interception forwards them to the \
             engine untouched. Nothing on screen changes until the translation is delivered."
        );
        wasl.balligh(&satr);
        return sajjil(mujallad, HalatBidaya::Naqisa, &satr);
    }

    sajjil(
        mujallad,
        HalatBidaya::Badiya,
        &format!(
            "the Godot 3 path is complete: the delivery puts this game's text into Arabic \
             from the next launch, and the takeover is installed with {adad} \
             interception(s), so Taarib shapes, positions and draws it"
        ),
    )
}

/// Removes everything this module installed, and drops what it holds open.
///
/// Runs from `taarib_gdnative_terminate`, which Godot calls before it unloads
/// the library. A live detour into an unmapped module is a crash with this
/// module's name on it, so this is the one path that must not be skipped — and
/// it is idempotent, because an engine that calls terminate twice is an engine
/// that would otherwise unhook a slot somebody else has since taken.
fn anhi(mujallad: &Path) -> HalatBidaya {
    let mustaqarr = ISTILA.lock().take();
    let mut baqaya = Vec::new();
    if let Some(mut istila) = mustaqarr {
        baqaya = istila.azil();
        drop(istila);
    }

    // Stale texture identifiers outlive the textures the engine is destroying,
    // and a draw that reached one after this point would hand a backend an
    // identifier that no longer names anything.
    imsah_safahat();

    // The mappings go last, so nothing that was still drawing can reach a patch
    // that has been unmapped.
    RUQAA.lock().clear();
    *WASL.lock() = None;

    for khata in &baqaya {
        let _ = sajjil(mujallad, HalatBidaya::Fashila, &khata.to_string());
    }
    HALA.store(HALA_MUNTAHIYA, Ordering::Release);

    if baqaya.is_empty() {
        sajjil(
            mujallad,
            HalatBidaya::Mumtania,
            "the library was unloaded, every interception was removed and every prologue \
             came back; the process is byte-identical to the one this module entered",
        )
    } else {
        sajjil(
            mujallad,
            HalatBidaya::Fashila,
            &format!(
                "the library was unloaded and {} interception(s) did not come out cleanly, \
                 named above; a restart is what removes them",
                baqaya.len()
            ),
        )
    }
}

// ---------------------------------------------------------------------------
// The entry points Godot calls
// ---------------------------------------------------------------------------

/// Godot 3's GDNative initialisation entry, named by `taarib.gdnlib`'s
/// `symbol_prefix="taarib_"`.
///
/// This is the real bootstrap for a Godot 3 game: it is the only moment anything
/// inside a shipped export template is handed the core API struct, and without
/// that struct there is no route to `VisualServer` and therefore no takeover.
///
/// The engine ignores the return value, so every outcome is reported through the
/// log beside this module and through the options' own `report_loading_error`.
///
/// # Safety
///
/// Called by Godot 3 with the `godot_gdnative_init_options *` its loader filled
/// in. The structure must be live and writable for the duration of the call and
/// its `api_struct` must be the engine's own core API. No other caller is
/// supported, and every guarantee below rests on Godot being the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_gdnative_init(khiyarat: *const KhiyaratTahyia) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if khiyarat.is_null() {
            let satr = "Godot passed null GDNative init options, which no released engine \
                        does; nothing about this engine could be established and nothing \
                        was assumed about it";
            match mujallad_nafsi() {
                Some(mujallad) => {
                    let _ = sajjil(&mujallad, HalatBidaya::Marfuda, satr);
                },
                None => tracing::warn!(satr, "taarib-godot"),
            }
            return;
        }
        // SAFETY: `khiyarat` is non-null and, per this function's contract, the
        // live `godot_gdnative_init_options` the engine's loader filled in and
        // keeps valid for the duration of this call.
        let khiyarat = unsafe { khiyarat.read() };

        // SAFETY: `khiyarat` is the structure Godot filled in, so its
        // `api_struct` is either null or the engine's core API struct — which is
        // exactly what `min_khiyarat` is contracted for.
        match unsafe { WaslGdnative::min_khiyarat(&khiyarat) } {
            Ok(wasl) => {
                let _ = ibda(MasdarBidaya::Gdnlib(wasl));
            },
            Err(khata) => {
                let satr = khata.to_string();
                tracing::warn!(khata = %satr, "the Taarib Godot 3 payload declined to bind");
                if let Some(mujallad) = mujallad_nafsi() {
                    let _ = sajjil(&mujallad, HalatBidaya::Marfuda, &satr);
                }
                if let (Some(daala), Ok(risala)) = (khiyarat.balligh_khata, CString::new(satr)) {
                    // SAFETY: `daala` is the `report_loading_error` Godot itself
                    // put in the options, `maktaba` is the library object from
                    // the same structure, and `risala` is NUL-terminated and
                    // outlives the call.
                    unsafe { daala(khiyarat.maktaba.cast_const(), risala.as_ptr()) };
                }
            },
        }
    }));
}

/// Godot 3's GDNative singleton entry, called because `taarib.gdnlib` declares
/// `singleton=true`.
///
/// It runs from the engine's module registration, after the servers are up and
/// before the main scene is instantiated — which is the window a takeover of the
/// text path needs: `VisualServer` exists, and nothing has been drawn yet. It
/// takes no arguments and returns nothing, which is the shape Godot calls it
/// with.
///
/// This is the second of the two chances the handover gets. A companion that
/// installed its configuration before `taarib_gdnative_init` was already served
/// there; one that installs between the two is served here.
///
/// # Safety
///
/// Called only by Godot 3, from the thread that is bringing the engine up. It
/// takes no arguments, so there is nothing for a caller to get wrong except the
/// moment — and a call before `taarib_gdnative_init` finds no binding and
/// declines by name rather than acting on one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_gdnative_singleton() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Some(mujallad) = mujallad_nafsi() else {
            tracing::warn!(
                "the Godot payload's singleton entry could not resolve its own directory, \
                 so it has no log to write; the game is untouched"
            );
            return;
        };
        let _ = sallim(&mujallad);
    }));
}

/// Godot 3's GDNative termination entry.
///
/// Removes every interception, checks that each target's prologue came back, and
/// drops the patch mappings so the file the uninstaller deletes is not one this
/// module still holds open.
///
/// # Safety
///
/// Called by Godot 3 with the `godot_gdnative_terminate_options *` its loader
/// filled in, or with null — both are handled, because the one field that
/// structure carries is not consulted: a takeover is removed on the way out
/// whether the editor or a game is the one unloading it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn taarib_gdnative_terminate(khiyarat: *const KhiyaratInha) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Acknowledged and not read. Whether this is the editor changes nothing
        // about the obligation to take the hooks back out.
        let _ = khiyarat;

        let Some(mujallad) = mujallad_nafsi() else {
            // The log is unreachable, and the hooks still have to come out.
            let mustaqarr = ISTILA.lock().take();
            drop(mustaqarr);
            imsah_safahat();
            RUQAA.lock().clear();
            *WASL.lock() = None;
            HALA.store(HALA_MUNTAHIYA, Ordering::Release);
            tracing::warn!(
                "the Godot payload was unloaded without being able to resolve its own \
                 directory; every interception was removed and nothing was logged"
            );
            return;
        };
        let _ = anhi(&mujallad);
    }));
}

// ---------------------------------------------------------------------------
// The entry point `taarib-mudkhal` calls
// ---------------------------------------------------------------------------

/// The bootstrap symbol every Taarib payload exports.
///
/// `taarib-mudkhal` opens each payload beside it and calls this once, from its
/// own thread, after the module is mapped and before the game has finished
/// starting. For this payload that is not the path the engine is taken over
/// through — Godot 3 is taken over through `taarib.gdnlib`, which is what hands
/// this library the API struct — so this entry establishes that the gdnlib path
/// is present in this binary and then declines by name. See the module header
/// for why that is the correct outcome rather than a gap.
///
/// It never panics and never unwinds across the boundary.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn taarib_bidaya() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ibda(MasdarBidaya::Hamula)));
}

/// The bootstrap symbol every Taarib payload exports.
///
/// `taarib-mudkhal` opens each payload beside it and calls this once, from its
/// own thread, after the module is mapped and before the game has finished
/// starting. For this payload that is not the path the engine is taken over
/// through — Godot 3 is taken over through `taarib.gdnlib`, which is what hands
/// this library the API struct — so this entry establishes that the gdnlib path
/// is present in this binary and then declines by name. See the module header
/// for why that is the correct outcome rather than a gap.
///
/// It never panics and never unwinds across the boundary.
#[cfg(not(windows))]
#[unsafe(no_mangle)]
pub extern "C" fn taarib_bidaya() {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ibda(MasdarBidaya::Hamula)));
}
