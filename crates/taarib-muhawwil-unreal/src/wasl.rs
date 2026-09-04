//! وصل — the seam between the ladder and the live process.
//!
//! [`crate::tashghil`] declares what it needs from a running game — set a
//! console variable, read one back, activate a culture, register a font — and
//! deliberately does not know how any of it is done. [`crate::slate`] knows how
//! to reach C++ objects and dispatch their virtual methods, and deliberately
//! does not know what anybody wants done with them. This file is the twenty
//! lines that join them, and it exists as a separate file for one reason: it is
//! the only place in the crate where a policy ("force full shaping") meets a
//! mechanism ("write through `IConsoleVariable::Set`"), and that meeting should
//! be somewhere a reader can find rather than buried in either side.
//!
//! ## Why the console manager is bound here and not in `slate`
//!
//! `slate` binds `FSlateApplication`, which is Slate. `IConsoleManager` is not
//! Slate — it lives in Core, it is reached differently, and a game may export
//! one and not the other. Putting it in `slate` would have meant a module named
//! after one subsystem that silently owned another, and a build where Core is
//! reachable but Slate is not — which is common, because Core is what everything
//! links — would have reported "Slate unavailable" and given up the one rung
//! that would have worked.
//!
//! ## What this file will honestly fail at
//!
//! In a monolithic shipping build almost nothing is exported, and
//! `IConsoleManager::Get` is usually among the nothing. So [`Musaddir::muhayya`]
//! returns false for most released games and the ladder falls back to the ini
//! and command-line rungs — which are the preferred rungs anyway, are additive
//! and reversible, and do not require being inside the process at all. This
//! module is the third rung, and the third rung being unavailable is the normal
//! case rather than a failure.
//!
//! No byte patterns are invented here. Phase 8's roadmap does not call for a
//! signature database and writing unverifiable patterns to fill this gap would
//! be dishonest engineering: it would look like coverage and behave like a coin
//! flip. When a build exports nothing usable, this file says so by name.

use std::ffi::{CString, c_void};
use std::sync::OnceLock;

use crate::khata::KhataUnreal;
use crate::slate::{Unwan, WaslSlate, dalla_min_jadwal, ramz_itanium, unwan_ramz_raisi,
    unwan_ramz_wahda};
use crate::tashghil::Musaddir;

/// The modules that export `IConsoleManager` when anything does.
///
/// Core first, because that is where it lives; the others are the names a
/// modular build gives the same module on different platforms and engine
/// versions.
pub const WAHDAT_QAIDA: &[&str] = &[
    "UnrealEditor-Core",
    "UE4Editor-Core",
    "Core",
    "libUnrealEditor-Core.so",
    "libUE4Editor-Core.so",
];

/// The vtable slot of `IConsoleManager::FindConsoleVariable`.
///
/// Not a constant this file invents: it is supplied by the caller, because a
/// vtable index is a property of one engine build and guessing one is how a
/// mod framework calls an unrelated virtual and corrupts a process. The
/// constant here is only the default the caller may override, and it is used
/// **only** when the caller has told this module it verified it.
pub const FAHRAS_IJAD_IFTIRADI: usize = 3;

/// The vtable slot of `IConsoleVariable::Set`.
///
/// The same caveat as [`FAHRAS_IJAD_IFTIRADI`].
pub const FAHRAS_DAA_IFTIRADI: usize = 2;

/// The vtable slot of `IConsoleVariable::GetString`.
///
/// The same caveat as [`FAHRAS_IJAD_IFTIRADI`].
pub const FAHRAS_IQRA_IFTIRADI: usize = 8;

/// Which vtable slots the console interfaces use in this build.
///
/// Supplied rather than assumed. A caller that has not verified them against
/// the build it is running in should not construct this at all, and
/// [`WaslAwamir::bila_jadwal`] is the constructor for that case: it binds the
/// console manager for reporting and refuses every write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaharisAwamir {
    /// `IConsoleManager::FindConsoleVariable`.
    pub ijad: usize,
    /// `IConsoleVariable::Set`.
    pub daa: usize,
    /// `IConsoleVariable::GetString`.
    pub iqra: usize,
}

impl FaharisAwamir {
    /// The defaults, for a caller that has verified them against its build.
    ///
    /// Named `muhaqqaqa` — verified — because constructing it is an assertion
    /// the caller is making, not a fact this crate knows.
    #[must_use]
    pub const fn muhaqqaqa() -> Self {
        Self {
            ijad: FAHRAS_IJAD_IFTIRADI,
            daa: FAHRAS_DAA_IFTIRADI,
            iqra: FAHRAS_IQRA_IFTIRADI,
        }
    }
}

/// `IConsoleManager& IConsoleManager::Get()`.
type DallaMudirAwamir = unsafe extern "C" fn() -> *mut c_void;

/// `IConsoleVariable* IConsoleManager::FindConsoleVariable(const TCHAR*, bool)`.
///
/// The trailing `bool` is `bTrackFrequentCalls`, present since UE4.14 and
/// defaulted in C++ — a default argument is a caller-side fiction, so it is
/// spelled out here as the parameter it actually is on the wire.
type DallaIjad =
    unsafe extern "C" fn(hadha: *mut c_void, ism: *const u16, tatabbu: bool) -> *mut c_void;

/// `void IConsoleVariable::Set(const TCHAR*, EConsoleVariableFlags)`.
type DallaDaa = unsafe extern "C" fn(hadha: *mut c_void, qeema: *const u16, alam: u32);

/// `FString IConsoleVariable::GetString() const`.
///
/// Returns an `FString`, which is a `TArray<TCHAR>` — a pointer, a count and a
/// capacity — and is not trivially copyable, so it comes back through the
/// hidden return pointer. Spelled out as a declared parameter for the same
/// reason `slate`'s shared-reference call is.
type DallaIqra =
    unsafe extern "C" fn(makhraj: *mut MasfufatNass, hadha: *mut c_void) -> *mut MasfufatNass;

/// Unreal's `TArray<TCHAR>` as it comes back from `GetString`.
///
/// ```text
/// MasfufatNass — 16 bytes, 8-byte aligned
///
///   offset  size  field   type    meaning
///        0     8  bayanat *u16    the UTF-16 code units, NUL-terminated
///        8     4  adad    i32     how many units, including the terminator
///       12     4  siaa    i32     how many the allocation holds
/// ```
///
/// Never freed by this crate. The allocation belongs to Unreal's allocator,
/// which this module has no handle on, and the string is read and dropped
/// within one call — so the cost of not freeing it is one short-lived leak per
/// read-back, and read-backs happen a handful of times at startup. Freeing it
/// through the wrong allocator would be a heap corruption in somebody's game,
/// which is not a trade worth making for a few dozen bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MasfufatNass {
    /// The code units.
    pub bayanat: *mut u16,
    /// How many, including the NUL terminator.
    pub adad: i32,
    /// How many the allocation holds.
    pub siaa: i32,
}

impl Default for MasfufatNass {
    fn default() -> Self {
        Self { bayanat: std::ptr::null_mut(), adad: 0, siaa: 0 }
    }
}

/// The console-variable binding, and the [`Musaddir`] the ladder drives.
#[derive(Debug)]
pub struct WaslAwamir {
    mudir: Option<Unwan>,
    faharis: Option<FaharisAwamir>,
    sabab: String,
}

impl WaslAwamir {
    /// Binds the console manager, if this build exports it.
    ///
    /// # Errors
    ///
    /// Never. A build that exports nothing produces a binding whose
    /// [`Musaddir::muhayya`] is false and whose refusals name what was tried,
    /// which is what lets the ladder fall back rather than stop.
    #[must_use]
    pub fn ijlib(faharis: Option<FaharisAwamir>) -> Self {
        let itanium = ramz_itanium("IConsoleManager", "Get");
        let msvc = "?Get@IConsoleManager@@SAAEAV1@XZ";

        let mut jurribat = Vec::with_capacity(2 + WAHDAT_QAIDA.len());

        if let Some(unwan) = unwan_ramz_raisi(&itanium) {
            return Self::min_unwan(unwan, faharis);
        }
        jurribat.push(format!("primary image: no {itanium}"));

        if let Some(unwan) = unwan_ramz_raisi(msvc) {
            return Self::min_unwan(unwan, faharis);
        }
        jurribat.push(format!("primary image: no {msvc}"));

        for wahda in WAHDAT_QAIDA {
            if let Some(unwan) = unwan_ramz_wahda(wahda, &itanium) {
                return Self::min_unwan(unwan, faharis);
            }
            if let Some(unwan) = unwan_ramz_wahda(wahda, msvc) {
                return Self::min_unwan(unwan, faharis);
            }
            jurribat.push(format!("{wahda}: not loaded or no IConsoleManager::Get"));
        }

        Self {
            mudir: None,
            faharis,
            sabab: format!(
                "IConsoleManager::Get is not exported by this build, which is the ordinary \
                 case for a shipping monolithic game ({})",
                jurribat.join("; ")
            ),
        }
    }

    /// A binding that reaches the console manager but will not write through it.
    ///
    /// For a caller that has not verified this build's vtable slots. It reports
    /// the manager as found — which is worth knowing — and refuses every write
    /// with a sentence saying why, so the ladder records rung three as tried and
    /// declined rather than as absent.
    #[must_use]
    pub fn bila_jadwal() -> Self {
        let mut wasl = Self::ijlib(None);
        wasl.faharis = None;
        wasl
    }

    const fn min_unwan(unwan: Unwan, faharis: Option<FaharisAwamir>) -> Self {
        Self { mudir: Some(unwan), faharis, sabab: String::new() }
    }

    /// Whether the console manager itself was found, regardless of whether this
    /// binding may write through it.
    #[must_use]
    pub const fn wujid(&self) -> bool {
        self.mudir.is_some()
    }

    /// Why the binding is unusable, or an empty string when it is usable.
    #[must_use]
    pub fn sabab(&self) -> &str {
        &self.sabab
    }

    /// The live `IConsoleManager`, called through its exported accessor.
    fn mudir_hayy(&self) -> Result<*mut c_void, KhataUnreal> {
        let Some(unwan) = self.mudir else {
            return Err(KhataUnreal::SlateGhayrMawjud { sabab: self.sabab.clone() });
        };
        // SAFETY: `unwan` came from a symbol lookup for `IConsoleManager::Get`,
        // whose C++ signature is `IConsoleManager&()` — a static member with no
        // parameters returning a reference, which on both the MSVC and Itanium
        // ABIs is a plain pointer return with no arguments. Transmuting the
        // resolved address to that shape is exact. The call itself is Unreal's
        // and constructs the singleton on first use, which is what it is for.
        let kaain = unsafe {
            let dalla: DallaMudirAwamir = std::mem::transmute(unwan.muashir());
            dalla()
        };
        if kaain.is_null() {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: "IConsoleManager::Get returned null, so the console manager has not \
                        been constructed yet"
                    .to_owned(),
            });
        }
        Ok(kaain)
    }

    /// Finds one console variable by name.
    fn jid(&self, miftah: &str) -> Result<*mut c_void, KhataUnreal> {
        let Some(faharis) = self.faharis else {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: "this build's console vtable slots were not verified, so no write is \
                        attempted"
                    .to_owned(),
            });
        };
        let mudir = self.mudir_hayy()?;
        let ism = utf16(miftah);

        // SAFETY: `mudir` is a live `IConsoleManager` from its own accessor, so
        // it has a vtable. `faharis.ijad` is a slot index the caller asserted
        // it verified against this build — the whole point of requiring
        // `FaharisAwamir` to be constructed explicitly. `dalla_min_jadwal`
        // bounds-checks the read of the slot itself.
        let kaain = unsafe {
            let Some(unwan) = dalla_min_jadwal(mudir, faharis.ijad) else {
                return Err(KhataUnreal::SlateGhayrMawjud {
                    sabab: "the console manager's vtable could not be read".to_owned(),
                });
            };
            let dalla: DallaIjad = std::mem::transmute(unwan.muashir());
            dalla(mudir, ism.as_ptr(), false)
        };

        if kaain.is_null() {
            return Err(KhataUnreal::TashghilFashil {
                sabab: format!(
                    "this build registers no console variable named \"{miftah}\""
                ),
            });
        }
        Ok(kaain)
    }
}

impl Musaddir for WaslAwamir {
    fn muhayya(&self) -> bool {
        self.mudir.is_some() && self.faharis.is_some()
    }

    fn daa(&self, miftah: &str, qeema: &str) -> Result<(), KhataUnreal> {
        let Some(faharis) = self.faharis else {
            return Err(KhataUnreal::SlateGhayrMawjud { sabab: self.sabab.clone() });
        };
        let mutaghayyir = self.jid(miftah)?;
        let nass = utf16(qeema);

        // SAFETY: `mutaghayyir` is a live `IConsoleVariable` the manager just
        // handed back, so it has a vtable, and `faharis.daa` is a slot the
        // caller asserted it verified. The value pointer is a NUL-terminated
        // UTF-16 buffer owned by `nass`, which outlives the call.
        //
        // The flag argument is `ECVF_SetByCode` (0x0800_0000). It matters: a
        // value set at a lower priority than the game's own configuration is
        // silently ignored, and one set at a higher priority than the player's
        // console input would override something a person typed. Setting by
        // code is the correct middle.
        unsafe {
            let Some(unwan) = dalla_min_jadwal(mutaghayyir, faharis.daa) else {
                return Err(KhataUnreal::TashghilFashil {
                    sabab: "the console variable's vtable could not be read".to_owned(),
                });
            };
            let dalla: DallaDaa = std::mem::transmute(unwan.muashir());
            dalla(mutaghayyir, nass.as_ptr(), ALAM_DAA_BIL_RAMZ);
        }
        Ok(())
    }

    fn iqra(&self, miftah: &str) -> Result<String, KhataUnreal> {
        let Some(faharis) = self.faharis else {
            return Err(KhataUnreal::SlateGhayrMawjud { sabab: self.sabab.clone() });
        };
        let mutaghayyir = self.jid(miftah)?;
        let mut makhraj = MasfufatNass::default();

        // SAFETY: `mutaghayyir` is a live `IConsoleVariable`, `faharis.iqra` is
        // a caller-verified slot, and `makhraj` is a caller-owned sixteen bytes
        // matching `FString`'s layout, passed as the hidden return pointer that
        // a non-trivially-copyable C++ return uses on both ABIs.
        let khaam = unsafe {
            let Some(unwan) = dalla_min_jadwal(mutaghayyir, faharis.iqra) else {
                return Err(KhataUnreal::TashghilFashil {
                    sabab: "the console variable's vtable could not be read".to_owned(),
                });
            };
            let dalla: DallaIqra = std::mem::transmute(unwan.muashir());
            let _ = dalla(&raw mut makhraj, mutaghayyir);
            makhraj
        };

        nass_min_masfufa(&khaam)
    }
}

/// `ECVF_SetByCode`, the priority a programmatic write uses.
const ALAM_DAA_BIL_RAMZ: u32 = 0x0800_0000;

/// The most code units a console variable's value may be before it is refused.
///
/// Console variable values are flags, numbers and short words. A value claiming
/// to be longer than this is a pointer that is not what this code thinks it is,
/// and reading it would walk memory that belongs to something else.
const AQSA_WAHDAT: i32 = 4096;

/// A Rust string as a NUL-terminated UTF-16 buffer Unreal can read.
///
/// Returned as an owned `Vec` so the buffer outlives the call that borrows it;
/// a function returning a pointer into a temporary would be a use-after-free
/// that happens to work until the optimiser notices.
fn utf16(nass: &str) -> Vec<u16> {
    let mut wahdat: Vec<u16> = nass.encode_utf16().collect();
    wahdat.push(0);
    wahdat
}

/// Reads an `FString` Unreal returned, without freeing it.
fn nass_min_masfufa(masfufa: &MasfufatNass) -> Result<String, KhataUnreal> {
    if masfufa.bayanat.is_null() || masfufa.adad <= 0 {
        return Ok(String::new());
    }
    if masfufa.adad > AQSA_WAHDAT {
        return Err(KhataUnreal::HajmMufrit {
            haql: "console variable value",
            qeema: u64::try_from(masfufa.adad).unwrap_or(u64::MAX),
            saqf: u64::try_from(AQSA_WAHDAT).unwrap_or(u64::MAX),
        });
    }

    // Unreal counts the NUL terminator in the length; the string is everything
    // before it. A reader that kept the terminator would compare unequal to
    // every value it was asked about, which would make the watchdog re-assert
    // forever on a variable that was already correct.
    let adad = usize::try_from(masfufa.adad).unwrap_or(0).saturating_sub(1);
    if adad == 0 {
        return Ok(String::new());
    }

    // SAFETY: `bayanat` is a non-null buffer Unreal allocated and populated,
    // and `adad` is one less than the count it reported, bounded above by
    // `AQSA_WAHDAT`. The slice is read and copied out before this function
    // returns, and nothing in this crate frees or mutates the buffer.
    let wahdat = unsafe { std::slice::from_raw_parts(masfufa.bayanat, adad) };
    String::from_utf16(wahdat).map_err(|_| KhataUnreal::NassGhayrSalih { fahras: 0, mawqi: 0 })
}

/// The process-wide console binding, resolved once.
///
/// A `OnceLock` rather than a lazily rebuilt value because symbol resolution
/// walks the loaded module list, and doing that repeatedly inside somebody's
/// game to re-derive an answer that cannot change would be a cost paid for
/// nothing.
static WASL: OnceLock<WaslAwamir> = OnceLock::new();

/// The process's console binding, resolved on first use.
///
/// # Errors
///
/// Never fails; an unusable binding is a binding whose [`Musaddir::muhayya`] is
/// false, which is how the ladder learns to use a different rung.
pub fn wasl(faharis: Option<FaharisAwamir>) -> &'static WaslAwamir {
    WASL.get_or_init(|| WaslAwamir::ijlib(faharis))
}

/// Whether Slate itself is reachable, for the corrections that need it rather
/// than the console.
///
/// Separate from the console binding on purpose: a build can export Core and
/// not Slate, and reporting one answer for both would disable the console rung
/// on a game where it would have worked.
///
/// # Errors
///
/// [`KhataUnreal::SlateGhayrMawjud`] naming what was tried.
pub fn slate(isdar: crate::slate::IsdarSlate) -> Result<WaslSlate, KhataUnreal> {
    WaslSlate::ijlib(isdar)
}

/// A C string, for the one platform call that wants one.
///
/// Kept here rather than reached for at a call site, because a `CString` built
/// from a name containing an interior NUL would be a silent truncation, and
/// this is where that is refused.
///
/// # Errors
///
/// [`KhataUnreal::SlateGhayrMawjud`] when the name contains an interior NUL.
pub fn nass_c(ism: &str) -> Result<CString, KhataUnreal> {
    CString::new(ism).map_err(|_| KhataUnreal::SlateGhayrMawjud {
        sabab: format!("the symbol name \"{ism}\" contains an interior NUL"),
    })
}
