//! وصل سليت — reaching Unreal's live text stack from an injected Rust module.
//!
//! Everything in `qiyas_unreal` is a correction applied to a C++ object that
//! already exists in somebody else's process. This module is the only place that
//! knows how to find such an object and how to call into it, so that the
//! corrections themselves are ordinary Rust reading ordinary structs.
//!
//! ## What is actually reachable, and what is not
//!
//! Unreal ships in two shapes and they are not equally open.
//!
//! A **modular** build — every editor build, and a minority of shipping builds —
//! puts `SlateCore` in its own shared library, and every class marked
//! `SLATECORE_API` is `__declspec(dllexport)` or has default ELF visibility. Its
//! entry points are in an export table and can be resolved by name.
//!
//! A **monolithic** shipping build links the whole engine into one image and
//! exports almost nothing. `SLATECORE_API` expands to nothing at all, the
//! linker discards what nothing references, and there is no export table row for
//! `FSlateApplication::Get`. This is the common case for a shipped game, and it
//! has to be said plainly rather than papered over: **on such a build this
//! module finds nothing and refuses.**
//!
//! The refusal is [`KhataUnreal::SlateGhayrMawjud`] and it names every module it
//! opened and every symbol it asked for, because a warning that says "Slate was
//! not found" and stops is a warning nobody can act on.
//!
//! ## Why refusing is survivable
//!
//! The adapter's offline half — `.locres`, `.locmeta`, string tables, the patch
//! `.pak` — has already run by the time anything here is attempted. The game is
//! displaying Arabic text before this module is asked a single question. What a
//! refusal costs is the *runtime corrections*: flow direction, alignment
//! resolution, and the wrapping disagreement. That is a real loss and it is not
//! the whole product; the offline half is the bulk of the value and it does not
//! depend on one line of this file.
//!
//! ## What this module deliberately does not do
//!
//! It does not scan the image for byte patterns. A signature database is a
//! promise that a sequence of bytes identifies a function in a build nobody in
//! this repository has ever run, and Phase 8's roadmap does not call for one.
//! Byte patterns written from memory would be pattern *fiction*: they would
//! match something eventually, and the something would not be the function. A
//! documented refusal is worth more than a hook installed at the wrong address,
//! which is a crash in a player's game with Taarib's name on it.
//!
//! ## The `shims/` question
//!
//! There is no C++ shim in this crate, and there is no directory for one.
//!
//! Everything below is expressible with `extern "C"` and `#[repr(C)]`: a virtual
//! call is a load, an index and an indirect call; an aggregate return is a
//! calling convention Rust already implements correctly; and the one genuinely
//! hard case — a return value with a non-trivial copy constructor and
//! destructor, namely `TSharedRef` — is resolved not by writing C++ but by
//! [deciding never to release it](MarjaMushtarak). A shim would earn its place
//! only if some future target required Taarib to *construct* a C++ object with a
//! non-trivial constructor, or to destroy one it owned; neither is needed to
//! read a measurement and correct it. If that day arrives, the shim belongs
//! beside this file as `shims/`, compiled by a build script, exposing one
//! `extern "C"` function per construction — and not one line more, because a
//! shim that grows becomes a second implementation of this module.

use core::ffi::c_void;

use crate::khata::KhataUnreal;

// ---------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------

/// A resolved address inside the game process.
///
/// Held as an integer rather than as a raw pointer for two reasons that both
/// matter here. It is [`Send`] and [`Sync`], so a resolved target can live in a
/// `OnceLock` or an atomic without an `unsafe impl` nobody can check; and the
/// conversion back to a pointer goes through
/// [`core::ptr::with_exposed_provenance`], which is the documented way to
/// reconstruct a pointer to memory this program did not allocate. Casting an
/// integer with `as` would work today and is exactly the pattern strict
/// provenance exists to replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unwan(usize);

impl Unwan {
    /// Wraps a raw address, refusing null.
    ///
    /// Null is the failure return of every symbol-lookup API this module calls,
    /// so it is rejected here once instead of at each of the call sites that
    /// would otherwise each have to remember.
    #[must_use]
    pub const fn jadeed(khaam: usize) -> Option<Self> {
        if khaam == 0 { None } else { Some(Self(khaam)) }
    }

    /// Wraps a pointer returned by a platform lookup, refusing null.
    #[must_use]
    pub fn min_muashir(muashir: *const c_void) -> Option<Self> {
        Self::jadeed(muashir.expose_provenance())
    }

    /// The address as an integer, for logging and for comparison.
    #[must_use]
    pub const fn raqm(self) -> usize {
        self.0
    }

    /// The address as a pointer.
    ///
    /// Restoring provenance that [`Unwan::min_muashir`] exposed. The result is
    /// only as valid as the module it came from: it points into a foreign image
    /// that could in principle be unloaded, which is why every hook installed on
    /// one is removed before this library unloads.
    #[must_use]
    pub const fn muashir(self) -> *const c_void {
        core::ptr::with_exposed_provenance(self.0)
    }

    /// The address as a mutable pointer, for the one caller that needs to read
    /// the bytes at a hook target before replacing them.
    #[must_use]
    pub const fn muashir_mutaghayyir(self) -> *mut c_void {
        core::ptr::with_exposed_provenance_mut(self.0)
    }
}

// ---------------------------------------------------------------------------
// Engine version, and the one thing it decides here
// ---------------------------------------------------------------------------

/// The precision of the vector Slate returns from a measurement.
///
/// Not a stylistic detail: it is the difference between reading eight bytes as
/// two `f32` and reading sixteen bytes as two `f64`. Read at the wrong width,
/// a text extent comes back as a number in the order of `1e-41` or as a NaN,
/// every string then "fits", and wrapping silently stops happening.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiqqatMuttajih {
    /// `FVector2f` — two `f32`. Every Unreal 4 build, and Unreal 5.4 onwards,
    /// where Slate's measurement API moved back to single precision through
    /// `UE::Slate::FDeprecateVector2DResult`.
    Mufrada,
    /// `FVector2d` — two `f64`. Unreal 5.0 through 5.3, where large-world
    /// coordinates made `FVector2D` double-precision and Slate's measurement
    /// signatures followed it.
    Mudaafa,
}

/// The engine version, to the precision this module needs from it.
///
/// Deliberately not a full version type. The adapter's other modules have their
/// own reasons to care about the patch version and the build configuration;
/// what a *binding* needs is the one bit that changes the shape of a return
/// value, and a type that carried more would be a second owner of a question
/// answered elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsdarSlate {
    kabir: u32,
    sagheer: u32,
}

impl IsdarSlate {
    /// Records a major and minor engine version.
    #[must_use]
    pub const fn jadeed(kabir: u32, sagheer: u32) -> Self {
        Self { kabir, sagheer }
    }

    /// The major version, as reported by whoever determined it.
    #[must_use]
    pub const fn kabir(self) -> u32 {
        self.kabir
    }

    /// The minor version.
    #[must_use]
    pub const fn sagheer(self) -> u32 {
        self.sagheer
    }

    /// Which vector width Slate's measurement returns on this version.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::IsdarMajhul`] for a major version outside 4 and 5. This is
    /// the honest answer and not a defensive gesture: an unrecognised engine is
    /// an engine whose return width nobody has checked, and guessing produces
    /// measurements that are wrong without ever looking wrong.
    pub fn diqqa(self) -> Result<DiqqatMuttajih, KhataUnreal> {
        // 5.0 through 5.3 is the only double-precision window, so it is matched
        // before the single-precision arm that spans the rest of 4 and 5.
        match (self.kabir, self.sagheer) {
            (5, 0..=3) => Ok(DiqqatMuttajih::Mudaafa),
            (4 | 5, _) => Ok(DiqqatMuttajih::Mufrada),
            (kabir, sagheer) => Err(KhataUnreal::IsdarMajhul {
                sabab: format!(
                    "version {kabir}.{sagheer} is neither Unreal 4 nor Unreal 5, so the \
                     width of Slate's measurement return value is unknown and was not \
                     guessed at"
                ),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Return-value ABI
// ---------------------------------------------------------------------------

/// `FVector2f` — Slate's single-precision two-component vector.
///
/// ## Why this is a `#[repr(C)]` struct and not two out-parameters
///
/// An eight-byte aggregate of two floats is returned differently on the two
/// calling conventions Taarib targets. On Windows x64 it comes back packed in
/// `RAX`, because the convention returns any aggregate of eight bytes or fewer
/// in the integer return register regardless of what the members are. On System
/// V — Linux, macOS — the classification algorithm marks both eightbyte halves
/// SSE and the value comes back in `XMM0`, both floats in the low 64 bits.
///
/// Rust's `extern "C"` implements the platform's C ABI for `#[repr(C)]` types,
/// which means declaring the return type is the whole of the work: the compiler
/// emits the `RAX` read on Windows and the `XMM0` read on System V from the same
/// source. Hand-rolling it — declaring the function as returning `u64` and
/// bit-casting — would produce a value that is correct on Windows and garbage on
/// Linux and macOS, and would be garbage *silently*, because a bit pattern read
/// out of the wrong register is still a pair of finite-looking floats.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Muttajih2F {
    /// The horizontal component: a measured width, in Slate units.
    pub s: f32,
    /// The vertical component: a measured height.
    pub a: f32,
}

/// `FVector2d` — Slate's double-precision two-component vector, Unreal 5.0
/// through 5.3.
///
/// Sixteen bytes, so the conventions diverge again and again `extern "C"` is
/// what resolves it: Windows x64 returns it through a hidden pointer because it
/// exceeds eight bytes, System V returns it in `XMM0` and `XMM1`. Two entirely
/// different mechanisms, one declaration.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Muttajih2D {
    /// The horizontal component.
    pub s: f64,
    /// The vertical component.
    pub a: f64,
}

impl Muttajih2D {
    /// Narrows to single precision for the callers that work in `f32`.
    ///
    /// A widening-free conversion in the direction that can lose bits, done
    /// once and named, rather than an `as` cast scattered through the
    /// corrections. Slate extents are screen distances in the low thousands, so
    /// the loss is below a pixel; saying so here is cheaper than saying it at
    /// every call site.
    #[must_use]
    pub const fn mufrad(self) -> Muttajih2F {
        Muttajih2F {
            s: dayyiq(self.s),
            a: dayyiq(self.a),
        }
    }
}

/// One `f64` to `f32` conversion, in one place, with the loss named.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the only stable way to narrow a float, confined to this function so the \
              justification is written once: Slate extents are screen distances in the low \
              thousands of pixels, a range f32 represents to far better than a pixel, and the \
              alternative is propagating a precision variant into every correction"
)]
const fn dayyiq(qeema: f64) -> f32 {
    qeema as f32
}

/// A measurement, at whichever width the engine returned it.
///
/// The corrections work in `f32` because `jisr` does, so this collapses to
/// [`Muttajih2F`] at the boundary rather than propagating a precision variant
/// into every caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Qiyas2 {
    /// Returned as `FVector2f`.
    Mufrad(Muttajih2F),
    /// Returned as `FVector2d`.
    Mudaaf(Muttajih2D),
}

impl Qiyas2 {
    /// The measurement in single precision, whatever it arrived as.
    #[must_use]
    pub const fn mufrad(self) -> Muttajih2F {
        match self {
            Self::Mufrad(qiyas) => qiyas,
            Self::Mudaaf(qiyas) => qiyas.mufrad(),
        }
    }
}

// ---------------------------------------------------------------------------
// TSharedRef / TSharedPtr
// ---------------------------------------------------------------------------

/// The layout of `TSharedRef` and `TSharedPtr`: an object pointer and a
/// reference-controller pointer.
///
/// Unreal's shared pointers are two words. The first is the object; the second
/// points at the control block that owns the strong and weak counts. A
/// `TSharedRef` differs from a `TSharedPtr` only in that the object pointer is
/// promised non-null, which is a C++ invariant and not a layout difference, so
/// one structure models both.
///
/// ## Why this is never released, on purpose
///
/// Slate's font measure service is a process-lifetime object. Unreal creates it
/// with the renderer and holds it until the renderer is destroyed at shutdown.
/// Taarib wants a handle to it for exactly as long as the game runs, which is
/// exactly as long as Unreal itself wants one.
///
/// Copying a `TSharedRef` in C++ increments the strong count; destroying the
/// copy decrements it. Taarib performs the increment — by calling a function
/// that returns one — and **deliberately never performs the matching
/// decrement**. The consequence is that the strong count is permanently one
/// higher than it would otherwise be, which means the service cannot be
/// destroyed early. That is the desired property, not an accident of it.
///
/// This is the decision that makes a C++ shim unnecessary. Releasing the
/// reference correctly would mean running `~TSharedRef`, which means calling the
/// control block's virtual `DestroyObject` when the count reaches zero, which
/// means either linking against Unreal's headers or reimplementing an
/// interlocked decrement against a layout that varies with the build's
/// threading configuration. Every one of those is a way to corrupt a live
/// refcount. Holding one reference forever costs sixteen bytes and cannot be
/// wrong.
///
/// It is not a leak in the sense that matters. A leak is memory nobody can
/// reach and nobody will free; this is a live object the engine also holds,
/// kept alive for the process lifetime by design, released when the process
/// exits and the whole address space goes with it.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MarjaMushtarak {
    /// The object. Non-null for a `TSharedRef`; possibly null for a
    /// `TSharedPtr`.
    pub kaain: *mut c_void,
    /// The reference controller. Never dereferenced by this module, for the
    /// reasons in this type's documentation.
    pub mutahakkim: *mut c_void,
}

impl MarjaMushtarak {
    /// An empty shared pointer, which is what the hidden return slot is
    /// initialised to before the call writes into it.
    #[must_use]
    pub const fn khali() -> Self {
        Self {
            kaain: core::ptr::null_mut(),
            mutahakkim: core::ptr::null_mut(),
        }
    }

    /// Whether the call produced an object.
    #[must_use]
    pub const fn mawjud(&self) -> bool {
        !self.kaain.is_null()
    }
}

/// A shared reference this process holds for the rest of its life.
///
/// The type exists so that "we are keeping this forever" is written into the
/// type system rather than into a comment somebody deletes. It has no [`Drop`]
/// implementation, and that absence is the point; see [`MarjaMushtarak`].
#[derive(Debug)]
pub struct HamilKhidma {
    marja: MarjaMushtarak,
    wasf: &'static str,
}

impl HamilKhidma {
    /// Takes ownership of a shared reference and never gives it back.
    ///
    /// `wasf` names the service in diagnostics — the string appears in the log
    /// line that records the address, and a holder with no name is a holder
    /// nobody can attribute when two of them are alive.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the call returned an empty shared
    /// pointer, which means the service does not exist yet. That is a real state
    /// early in startup — the renderer is created after the application — and it
    /// is a reason to try again later, not a reason to stop.
    pub fn jadeed(marja: MarjaMushtarak, wasf: &'static str) -> Result<Self, KhataUnreal> {
        if marja.mawjud() {
            Ok(Self { marja, wasf })
        } else {
            Err(KhataUnreal::SlateGhayrMawjud {
                sabab: format!(
                    "{wasf} returned an empty shared pointer, which means Slate has not \
                     created it yet; this is expected before the renderer exists"
                ),
            })
        }
    }

    /// The service object.
    #[must_use]
    pub const fn kaain(&self) -> *mut c_void {
        self.marja.kaain
    }

    /// The service's address, for logging.
    #[must_use]
    pub fn unwan(&self) -> usize {
        self.marja.kaain.cast_const().expose_provenance()
    }

    /// What this holder holds.
    #[must_use]
    pub const fn wasf(&self) -> &'static str {
        self.wasf
    }
}

// SAFETY: the two pointers are moved between threads, never dereferenced by
// this type, and name an object whose own thread-safety is Unreal's affair.
// What must not happen is a Rust-side data race on the *holder*, and there is
// none: the fields are written once at construction and only read afterwards.
// The referenced service is used exclusively from the game thread, which is
// where every hook in `qiyas_unreal` runs.
unsafe impl Send for HamilKhidma {}
// SAFETY: as above. A shared reference to this type hands out a raw pointer,
// and every use of that pointer is inside an `unsafe` block whose invariant
// names the game thread.
unsafe impl Sync for HamilKhidma {}

// ---------------------------------------------------------------------------
// Virtual dispatch
// ---------------------------------------------------------------------------

/// Reads an object's vtable pointer.
///
/// The first word of any polymorphic C++ object is the pointer to its virtual
/// function table. This is true on the Itanium C++ ABI — Linux, macOS, and
/// every ELF and Mach-O target — and on the MSVC ABI, and it is true for the
/// single-inheritance case that covers everything Slate exposes here.
///
/// # Safety
///
/// `kaain` must point at a live, fully constructed C++ object with at least one
/// virtual function, allocated by the game and not freed. The guarantee comes
/// from the caller having obtained the pointer from Slate itself in this same
/// process — either from an exported accessor or from a shared reference this
/// module is holding — and from `qiyas_unreal` running every such call on the
/// game thread, where Slate is not tearing objects down underneath it.
#[must_use]
pub const unsafe fn jadwal_kaain(kaain: *const c_void) -> Option<*const *const c_void> {
    if kaain.is_null() {
        return None;
    }
    // SAFETY: the caller guarantees `kaain` points at a live polymorphic object,
    // whose first word is its vtable pointer on both target ABIs. The read is
    // of one pointer at offset zero, which is within any such object.
    let jadwal = unsafe { kaain.cast::<*const *const c_void>().read() };
    if jadwal.is_null() { None } else { Some(jadwal) }
}

/// Reads the `fahras`-th entry of an object's vtable.
///
/// ## Why indexing a vtable from Rust is correct
///
/// Both ABIs lay a vtable out as a contiguous array of code addresses in
/// declaration order, and both place the address of a virtual function at a
/// fixed index that the compiler assigns at the point of declaration. On the
/// Itanium ABI the object's stored pointer already points *past* the RTTI and
/// offset-to-top words, directly at entry zero, so `jadwal[n]` is the `n`-th
/// virtual function with no adjustment. On the MSVC ABI the stored pointer
/// points at entry zero and the RTTI pointer lives at index `-1`, so `jadwal[n]`
/// is again the `n`-th virtual function. The two ABIs disagree about almost
/// everything else and agree about this.
///
/// A virtual call is then: load the vtable pointer, load the slot, call the slot
/// with `this` as the first argument — implicit `this` is the leading parameter
/// under both conventions, in `RCX` on Windows x64 and in `RDI` on System V.
/// That is what an `extern "C"` function pointer whose first parameter is a raw
/// pointer compiles to, exactly.
///
/// ## Where it is *not* correct
///
/// This construction holds for single inheritance, non-variadic virtual member
/// functions. It does not hold for:
///
/// - **Multiple inheritance.** Casting to a secondary base adjusts `this` by a
///   non-zero offset, and the slot found through the secondary vtable is a
///   *thunk* that undoes the adjustment before jumping to the real body.
///   Calling such a slot with the primary `this` corrupts every member access
///   in the callee. A pointer obtained as a secondary base must be used with
///   that base's own vtable and its own `this`, or not at all.
/// - **Covariant returns.** An override whose return type is a derived pointer
///   gets a return-adjusting thunk in the base's slot. The thunk is correct to
///   call, but the pointer it returns has been adjusted to the *base's* type,
///   so treating it as the derived type is wrong by exactly the base offset.
/// - **Virtual inheritance.** The `this` adjustment is read out of a virtual
///   base table at run time, and there is no fixed offset to reason about at
///   all.
/// - **Variadic virtuals.** Legal C++, and the register-assignment rules for
///   the fixed prefix differ from the non-variadic case on System V.
///
/// None of the four appears on the path this module takes: Slate's application,
/// renderer and font-measure service are single-inheritance hierarchies with
/// non-variadic, non-covariant virtuals. The list is here because the next
/// person to add a call is the person who needs it.
///
/// # Safety
///
/// As [`jadwal_kaain`], plus: `fahras` must be a slot index that exists in this
/// object's vtable for this engine version. There is no way to check that from
/// Rust — a vtable carries no length — so the guarantee is the caller's, and in
/// this crate it comes from the version-keyed table that `qiyas_unreal` is
/// handed rather than from a number written into this file.
#[must_use]
pub unsafe fn dalla_min_jadwal(kaain: *const c_void, fahras: usize) -> Option<Unwan> {
    // SAFETY: delegated verbatim to this function's own contract, which is
    // `jadwal_kaain`'s contract plus the slot index.
    let jadwal = unsafe { jadwal_kaain(kaain) }?;
    // SAFETY: the caller guarantees `fahras` names a slot that exists in this
    // vtable. `add` stays in bounds of the vtable allocation under that
    // guarantee, and the read is of one code address.
    let dalla = unsafe { jadwal.add(fahras).read() };
    Unwan::min_muashir(dalla)
}

/// A virtual member function taking only `this` and returning `FVector2f`.
pub type DallaQiyasMufrad = unsafe extern "C" fn(hadha: *mut c_void) -> Muttajih2F;

/// A virtual member function taking only `this` and returning `FVector2d`.
pub type DallaQiyasMudaaf = unsafe extern "C" fn(hadha: *mut c_void) -> Muttajih2D;

/// A member function returning `TSharedRef`, in its true ABI shape.
///
/// ## Why the hidden pointer is written out and not left to Rust
///
/// `TSharedRef` has a non-trivial copy constructor and a non-trivial
/// destructor, which makes it a non-trivial class for the purposes of both
/// ABIs, which means it is returned through a caller-allocated buffer whose
/// address is passed as a hidden first argument — the `sret` convention — and
/// `this` follows it. That is true on Windows x64 and on System V alike.
///
/// Rust cannot be told to reproduce that from the return type, because
/// [`MarjaMushtarak`] is a plain two-pointer `#[repr(C)]` struct and therefore
/// *trivially* copyable. Declaring the function as returning `MarjaMushtarak`
/// would make Rust follow the C rule for a sixteen-byte aggregate: hidden
/// pointer on Windows x64, and **`RAX`/`RDX` register pair on System V**. On
/// Linux and macOS the two sides would then disagree about where the return
/// value lives, the callee would write the object into a buffer the caller
/// never reads, and the caller would read two registers the callee never set.
///
/// So the hidden pointer is a declared parameter. Both ABIs also specify that
/// the function returns the same pointer in the return register, which the
/// declaration mirrors and which the caller ignores.
pub type DallaMarjaMushtarak =
    unsafe extern "C" fn(makhraj: *mut MarjaMushtarak, hadha: *mut c_void) -> *mut MarjaMushtarak;

/// Calls a virtual measurement function and returns its result at the width the
/// engine uses.
///
/// # Safety
///
/// `dalla` must be the address of a virtual member function of `hadha`'s class
/// that takes no arguments beyond `this` and returns `FVector2D` at the width
/// `diqqa` names; `hadha` must be a live object of that class. Both guarantees
/// come from the caller having resolved `dalla` out of `hadha`'s own vtable at a
/// slot index checked against the engine version, and from the call happening on
/// the game thread.
#[must_use]
pub unsafe fn nadi_qiyas(dalla: Unwan, hadha: *mut c_void, diqqa: DiqqatMuttajih) -> Qiyas2 {
    match diqqa {
        DiqqatMuttajih::Mufrada => {
            // SAFETY: the caller guarantees `dalla` is a `this`-only member
            // function returning `FVector2f`. Transmuting a code address to a
            // function pointer of the matching signature is the only way to
            // call it, and the signature is the one the caller promised.
            let dalla: DallaQiyasMufrad = unsafe { core::mem::transmute(dalla.muashir()) };
            // SAFETY: `hadha` is a live object of `dalla`'s class, per the
            // caller's contract.
            Qiyas2::Mufrad(unsafe { dalla(hadha) })
        },
        DiqqatMuttajih::Mudaafa => {
            // SAFETY: as above, for the double-precision signature.
            let dalla: DallaQiyasMudaaf = unsafe { core::mem::transmute(dalla.muashir()) };
            // SAFETY: as above.
            Qiyas2::Mudaaf(unsafe { dalla(hadha) })
        },
    }
}

/// Calls a function returning `TSharedRef` through the hidden-return-pointer
/// convention, and keeps the reference.
///
/// The returned [`MarjaMushtarak`] carries a strong count this process took and
/// will not give back. That is the decision documented on [`MarjaMushtarak`],
/// and it is why nothing here has a destructor.
///
/// # Safety
///
/// `dalla` must be the address of a member function of `hadha`'s class taking no
/// arguments beyond `this` and returning `TSharedRef` or `TSharedPtr`; `hadha`
/// must be a live object of that class. The guarantee comes from the caller
/// having resolved `dalla` either from an export table by name or from `hadha`'s
/// own vtable at a version-checked slot.
#[must_use]
pub unsafe fn nadi_marja(dalla: Unwan, hadha: *mut c_void) -> MarjaMushtarak {
    let mut makhraj = MarjaMushtarak::khali();
    // SAFETY: the caller guarantees `dalla` has this signature. The `sret`
    // shape is spelled out in the parameter list rather than inferred from the
    // return type, for the reason documented on `DallaMarjaMushtarak`.
    let dalla: DallaMarjaMushtarak = unsafe { core::mem::transmute(dalla.muashir()) };
    // SAFETY: `makhraj` is a live, initialised, correctly aligned buffer of
    // exactly the size the callee will write, and `hadha` is a live object of
    // `dalla`'s class. The callee writes the two words and returns.
    let _ = unsafe { dalla(&raw mut makhraj, hadha) };
    makhraj
}

// ---------------------------------------------------------------------------
// Symbol lookup
// ---------------------------------------------------------------------------

/// How a Slate entry point was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasdarWasl {
    /// Exported by the process's own primary image. Common in monolithic builds
    /// that were linked with an export list, and in every build where the game's
    /// own plugins needed the symbol.
    WahdaRaisiya,
    /// Exported by a separate `SlateCore` module. Every editor build, and the
    /// modular shipping builds.
    WahdaMustaqilla,
}

/// One entry point this module knows how to ask for.
///
/// The three spellings are the same function under the two C++ manglings plus
/// the undecorated form a build with an explicit export list produces. They are
/// listed rather than derived because MSVC's decoration of a return type is not
/// reconstructible from a class and function name alone, and a name this file
/// computed wrongly would fail to resolve while looking like a build that does
/// not export Slate.
#[derive(Debug, Clone, Copy)]
pub struct HadafRamz {
    /// The C++ class, for the Itanium name and for diagnostics.
    pub nitaq: &'static str,
    /// The member function.
    pub dalla: &'static str,
    /// The MSVC decorated spelling, for Windows builds.
    pub msvc: &'static str,
}

/// `FSlateApplication::Get()` — the application singleton.
///
/// A static member function taking no arguments and returning a reference,
/// which at the ABI level is a plain pointer return in the return register. The
/// entry point every other reach into Slate starts from.
pub const HADAF_TATBEEQ: HadafRamz = HadafRamz {
    nitaq: "FSlateApplication",
    dalla: "Get",
    msvc: "?Get@FSlateApplication@@SAAEAV1@XZ",
};

/// `FSlateApplicationBase::Get()` — the same singleton through its base class.
///
/// Tried second because it is exported by more builds: the base lives in
/// `SlateCore` while the derived class lives in `Slate`, and a build that splits
/// the two exports the base more often than the derived. The object it returns
/// is the same object; only the static type differs, and this module does not
/// use the static type for anything but documentation.
pub const HADAF_TATBEEQ_QAIDA: HadafRamz = HadafRamz {
    nitaq: "FSlateApplicationBase",
    dalla: "Get",
    msvc: "?Get@FSlateApplicationBase@@SAAEAV1@XZ",
};

/// Every entry point tried, in the order they are tried.
pub const AHDAF: &[HadafRamz] = &[HADAF_TATBEEQ, HADAF_TATBEEQ_QAIDA];

/// Builds the Itanium C++ ABI mangled name of `Nitaq::Dalla()`.
///
/// Not a guess and not a table: the Itanium ABI specifies a nested name as
/// `_ZN`, then each component as its length in decimal followed by its
/// characters, then `E`, and a function taking no arguments as `v`. So
/// `FSlateApplication::Get()` mangles to
/// `_ZN17FSlateApplication3GetEv`, and that is derivation rather than
/// invention.
///
/// The rule as implemented covers exactly the shape this module calls: a
/// two-component nested name, no arguments, no template parameters, no
/// namespace nesting, no substitution compression. Every target in [`AHDAF`]
/// has that shape. A target that did not would need its Itanium spelling listed
/// beside its MSVC one, the same way and for the same reason.
#[must_use]
pub fn ramz_itanium(nitaq: &str, dalla: &str) -> String {
    format!("_ZN{}{}{}{}Ev", nitaq.len(), nitaq, dalla.len(), dalla)
}

/// The module names probed for a separate Slate library, in probe order.
///
/// Shipping builds first because a player's game is the case that matters, then
/// the editor spellings, which is what a translator testing against an editor
/// build will have. The engine has used three prefixes across its lifetime and
/// all three are still on disk somewhere.
pub const WAHDAT: &[&str] = &[
    "SlateCore",
    "Slate",
    "UnrealEditor-SlateCore",
    "UnrealEditor-Slate",
    "UE4Editor-SlateCore",
    "UE4Editor-Slate",
];

/// Resolves an exported symbol in the process's primary image.
///
/// Returns [`None`] when the symbol is not exported, which on a monolithic
/// shipping build is the expected answer and not an error.
#[must_use]
pub fn unwan_ramz_raisi(ism: &str) -> Option<Unwan> {
    let ism = ism_c(ism)?;
    manassa::ramz_raisi(&ism)
}

/// Resolves an exported symbol in a named module, if that module is loaded.
///
/// The module is looked up, never loaded: asking the operating system to load
/// `SlateCore` into a process that did not already have it would be Taarib
/// changing what the game is made of, and a module loaded that way would have
/// its own uninitialised globals rather than the ones the game is using.
#[must_use]
pub fn unwan_ramz_wahda(wahda: &str, ism: &str) -> Option<Unwan> {
    let ism = ism_c(ism)?;
    manassa::ramz_wahda(wahda, &ism)
}

/// Resolves one target through every spelling and every module, in order.
///
/// Returns the address, how it was reached, and the spelling that worked — the
/// third is kept because a bug report that says which of three manglings
/// resolved is a bug report that identifies the toolchain the game was built
/// with.
#[must_use]
pub fn unwan_hadaf(hadaf: &HadafRamz) -> Option<(Unwan, MasdarWasl, String)> {
    for ism in asma(hadaf) {
        if let Some(unwan) = unwan_ramz_raisi(&ism) {
            return Some((unwan, MasdarWasl::WahdaRaisiya, ism));
        }
        for wahda in WAHDAT {
            if let Some(unwan) = unwan_ramz_wahda(wahda, &ism) {
                return Some((unwan, MasdarWasl::WahdaMustaqilla, ism));
            }
        }
    }
    None
}

/// Every spelling of one target: undecorated, Itanium, MSVC.
fn asma(hadaf: &HadafRamz) -> Vec<String> {
    vec![
        ramz_itanium(hadaf.nitaq, hadaf.dalla),
        hadaf.msvc.to_owned(),
        format!("{}_{}", hadaf.nitaq, hadaf.dalla),
    ]
}

/// A NUL-terminated copy of a symbol name.
///
/// Returns [`None`] for a name containing an interior NUL, which would
/// otherwise be silently truncated into a lookup for a different symbol. No
/// name in this file contains one; the check is here because the function is
/// public and the next caller's name may not come from this file.
fn ism_c(ism: &str) -> Option<Vec<u8>> {
    if ism.as_bytes().contains(&0) {
        return None;
    }
    let mut bayt = Vec::with_capacity(ism.len() + 1);
    bayt.extend_from_slice(ism.as_bytes());
    bayt.push(0);
    Some(bayt)
}

// ---------------------------------------------------------------------------
// The binding
// ---------------------------------------------------------------------------

/// A live connection to Slate in this process.
///
/// Holding one is the proof that an entry point was found and that the engine
/// version is one whose return widths are known. Nothing in `qiyas_unreal`
/// installs a correction without one.
#[derive(Debug)]
pub struct WaslSlate {
    isdar: IsdarSlate,
    diqqa: DiqqatMuttajih,
    masdar: MasdarWasl,
    ramz: String,
    tatbeeq: Unwan,
}

impl WaslSlate {
    /// Finds Slate, or refuses and says what it tried.
    ///
    /// The layers, in order:
    ///
    /// 1. The engine version is resolved to a return width. An engine nobody
    ///    has characterised is refused here, before a single address is read,
    ///    because a binding that reads the right function at the wrong width
    ///    produces measurements that are wrong and plausible.
    /// 2. Each target in [`AHDAF`] is asked for by each of its three spellings
    ///    in the process's primary image.
    /// 3. Then in each module in [`WAHDAT`] that is already loaded.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::IsdarMajhul`] when the engine version does not determine a
    /// return width, and [`KhataUnreal::SlateGhayrMawjud`] when no entry point
    /// resolved — carrying the full list of symbols and modules that were
    /// asked, and stating that the adapter continues on its offline half.
    pub fn ijlib(isdar: IsdarSlate) -> Result<Self, KhataUnreal> {
        let diqqa = isdar.diqqa()?;
        for hadaf in AHDAF {
            if let Some((tatbeeq, masdar, ramz)) = unwan_hadaf(hadaf) {
                return Ok(Self {
                    isdar,
                    diqqa,
                    masdar,
                    ramz,
                    tatbeeq,
                });
            }
        }
        Err(KhataUnreal::SlateGhayrMawjud {
            sabab: taqreer_ikhfaq(),
        })
    }

    /// The engine version this binding was built for.
    #[must_use]
    pub const fn isdar(&self) -> IsdarSlate {
        self.isdar
    }

    /// The width Slate returns measurements at on this engine.
    #[must_use]
    pub const fn diqqa(&self) -> DiqqatMuttajih {
        self.diqqa
    }

    /// Whether the entry point came from the primary image or a Slate module.
    #[must_use]
    pub const fn masdar(&self) -> MasdarWasl {
        self.masdar
    }

    /// The spelling that resolved, which identifies the toolchain.
    #[must_use]
    pub fn ramz(&self) -> &str {
        &self.ramz
    }

    /// The address of the application accessor.
    #[must_use]
    pub const fn tatbeeq(&self) -> Unwan {
        self.tatbeeq
    }

    /// Calls the application accessor and returns the singleton.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the accessor returned null, which
    /// happens when it is called before Slate is initialised — early enough in
    /// startup that the correct response is to try again on a later frame
    /// rather than to give up.
    pub fn tatbeeq_hayy(&self) -> Result<*mut c_void, KhataUnreal> {
        type DallaTatbeeq = unsafe extern "C" fn() -> *mut c_void;
        // SAFETY: `self.tatbeeq` resolved from an export table as
        // `FSlateApplication::Get` or `FSlateApplicationBase::Get`, both of
        // which are static member functions taking no arguments and returning a
        // reference — a plain pointer return at the ABI level. Holding a
        // `WaslSlate` is the proof that the resolution happened.
        let dalla: DallaTatbeeq = unsafe { core::mem::transmute(self.tatbeeq.muashir()) };
        // SAFETY: as above; the function takes no arguments, so there is no
        // argument whose validity could be in question.
        let kaain = unsafe { dalla() };
        if kaain.is_null() {
            return Err(KhataUnreal::SlateGhayrMawjud {
                sabab: format!(
                    "{} returned null, which means Slate is not initialised yet in this \
                     process",
                    self.ramz
                ),
            });
        }
        Ok(kaain)
    }

    /// Calls a `TSharedRef`-returning accessor on a Slate object and holds the
    /// result for the life of the process.
    ///
    /// `dalla` is supplied by the caller rather than computed here. The address
    /// of a font-measure accessor, or the vtable slot it sits in, is a number
    /// that varies with the engine version, and a number this file invented
    /// would be a number nobody could verify — the same objection that keeps a
    /// signature database out of this module.
    ///
    /// # Errors
    ///
    /// [`KhataUnreal::SlateGhayrMawjud`] when the accessor produced an empty
    /// shared pointer.
    ///
    /// # Safety
    ///
    /// `dalla` must be a member function of `hadha`'s class taking no arguments
    /// beyond `this` and returning `TSharedRef` or `TSharedPtr`, and `hadha`
    /// must be a live object of that class obtained from Slate in this process.
    #[expect(
        clippy::unused_self,
        reason = "the receiver is the capability, not an input: holding a `WaslSlate` is this \
                  module's proof that Slate resolved in this process, which is half of the \
                  safety contract above. Making this an associated function would let a \
                  caller reach Slate's accessor without that proof"
    )]
    pub unsafe fn imsik_khidma(
        &self,
        dalla: Unwan,
        hadha: *mut c_void,
        wasf: &'static str,
    ) -> Result<HamilKhidma, KhataUnreal> {
        // SAFETY: delegated verbatim to this function's own contract, which is
        // `nadi_marja`'s contract restated.
        let marja = unsafe { nadi_marja(dalla, hadha) };
        HamilKhidma::jadeed(marja, wasf)
    }
}

/// The refusal message: every symbol asked for, every module opened, and what
/// the adapter does next.
fn taqreer_ikhfaq() -> String {
    let mut rumuz: Vec<String> = Vec::new();
    for hadaf in AHDAF {
        rumuz.extend(asma(hadaf));
    }
    format!(
        "none of {} resolved in the primary image or in any of {}. This is the expected \
         result for a monolithic shipping build, where SLATECORE_API expands to nothing and \
         the linker keeps no export table row for Slate. No byte-pattern scan was attempted, \
         because a pattern this build has never verified would eventually match the wrong \
         function and hook it. The adapter continues on its offline half: the localization \
         resources are installed and the game displays Arabic; what is not applied is the \
         runtime correction of flow direction, alignment and wrapping.",
        rumuz.join(", "),
        WAHDAT.join(", "),
    )
}

// ---------------------------------------------------------------------------
// Platform symbol lookup
// ---------------------------------------------------------------------------

/// The two platform implementations of "resolve an exported symbol".
///
/// Declared here rather than pulled from `libc` so that this crate's dependency
/// list does not grow for four function signatures, and so that the signatures
/// are visible beside the invariants that make calling them sound.
mod manassa {
    #[cfg(windows)]
    pub(super) use nawafidh::{ramz_raisi, ramz_wahda};
    #[cfg(unix)]
    pub(super) use yuniks::{ramz_raisi, ramz_wahda};

    /// A target that is neither Windows nor Unix has no dynamic loader this
    /// module knows how to ask, so it reports "not exported" and the binding
    /// refuses with the same named message as a monolithic build.
    #[cfg(not(any(windows, unix)))]
    pub(super) fn ramz_raisi(_ism: &[u8]) -> Option<super::Unwan> {
        None
    }

    /// See [`ramz_raisi`].
    #[cfg(not(any(windows, unix)))]
    pub(super) fn ramz_wahda(_wahda: &str, _ism: &[u8]) -> Option<super::Unwan> {
        None
    }

    #[cfg(windows)]
    mod nawafidh {
        use core::ffi::{c_char, c_void};

        use crate::slate::Unwan;

        #[allow(
            non_snake_case,
            reason = "an imported Win32 entry point is matched by name at link time, so \
                      renaming it to Rust's convention would be declaring a different \
                      symbol; the two names below are the operating system's spelling"
        )]
        unsafe extern "system" {
            /// Returns a handle to an already-loaded module, or to the primary
            /// image when the name is null. Never loads anything.
            fn GetModuleHandleW(ism: *const u16) -> *mut c_void;
            /// Resolves an exported symbol by its decorated name.
            fn GetProcAddress(wahda: *mut c_void, ism: *const c_char) -> *const c_void;
        }

        /// A NUL-terminated UTF-16 copy of a module name, with the extension.
        fn ism_wide(wahda: &str) -> Vec<u16> {
            let mut wide: Vec<u16> = format!("{wahda}.dll").encode_utf16().collect();
            wide.push(0);
            wide
        }

        pub(in crate::slate) fn ramz_raisi(ism: &[u8]) -> Option<Unwan> {
            // SAFETY: a null name asks for the process's primary image and is
            // the documented way to do so; the call cannot fail for a running
            // process and returns a handle this code only passes back to
            // `GetProcAddress`.
            let wahda = unsafe { GetModuleHandleW(core::ptr::null()) };
            if wahda.is_null() {
                return None;
            }
            // SAFETY: `wahda` is a live module handle, and `ism` was built by
            // `ism_c`, which guarantees a NUL terminator and no interior NUL,
            // so the pointer names a valid C string for the duration of the
            // call. `GetProcAddress` returns null rather than failing.
            let unwan = unsafe { GetProcAddress(wahda, ism.as_ptr().cast::<c_char>()) };
            Unwan::min_muashir(unwan)
        }

        pub(in crate::slate) fn ramz_wahda(wahda: &str, ism: &[u8]) -> Option<Unwan> {
            let ism_wahda = ism_wide(wahda);
            // SAFETY: `ism_wahda` is a NUL-terminated UTF-16 buffer that
            // outlives the call. `GetModuleHandleW` looks a module up without
            // loading it and returns null when it is not present, which is the
            // ordinary answer for most names in `WAHDAT`.
            let maqbad = unsafe { GetModuleHandleW(ism_wahda.as_ptr()) };
            if maqbad.is_null() {
                return None;
            }
            // SAFETY: as `ramz_raisi`.
            let unwan = unsafe { GetProcAddress(maqbad, ism.as_ptr().cast::<c_char>()) };
            Unwan::min_muashir(unwan)
        }
    }

    #[cfg(unix)]
    mod yuniks {
        use core::ffi::{c_char, c_int, c_void};

        use crate::slate::Unwan;

        unsafe extern "C" {
            fn dlopen(ism: *const c_char, alam: c_int) -> *mut c_void;
            fn dlsym(maqbad: *mut c_void, ism: *const c_char) -> *mut c_void;
            fn dlclose(maqbad: *mut c_void) -> c_int;
        }

        /// `RTLD_LAZY`, which is 1 on Linux, macOS and the BSDs alike.
        const KASUL: c_int = 1;

        /// `RTLD_NOLOAD`: resolve an already-loaded object or fail, never load.
        ///
        /// The value differs between the two loaders, which is why it is a
        /// constant per target rather than a number written inline. Loading a
        /// second copy of `SlateCore` into a game that already has one would
        /// give Taarib a module with its own uninitialised globals — an object
        /// graph that looks like Slate and shares nothing with the running one.
        #[cfg(target_os = "macos")]
        const BILA_TAHMEEL: c_int = 0x10;
        /// See the macOS arm.
        #[cfg(not(target_os = "macos"))]
        const BILA_TAHMEEL: c_int = 4;

        /// A NUL-terminated shared-object file name for a module.
        fn ism_malaf(wahda: &str) -> Vec<u8> {
            let ism = if cfg!(target_os = "macos") {
                format!("lib{wahda}.dylib\0")
            } else {
                format!("lib{wahda}.so\0")
            };
            ism.into_bytes()
        }

        pub(in crate::slate) fn ramz_raisi(ism: &[u8]) -> Option<Unwan> {
            // SAFETY: a null name asks the dynamic loader for a handle to the
            // running program and its already-loaded dependencies, which is the
            // documented behaviour on every Unix target here and loads nothing.
            let maqbad = unsafe { dlopen(core::ptr::null(), KASUL) };
            if maqbad.is_null() {
                return None;
            }
            // SAFETY: `maqbad` is a live loader handle, and `ism` is a
            // NUL-terminated C string with no interior NUL, guaranteed by
            // `ism_c`, valid for the duration of the call.
            let unwan = unsafe { dlsym(maqbad, ism.as_ptr().cast::<c_char>()) };
            // SAFETY: `maqbad` came from `dlopen` and has not been closed. The
            // global handle is reference-counted, so this drops the count this
            // call took and leaves the program's own handle untouched. The
            // resolved address stays valid because the object it lives in is
            // the running program, which is not unloadable.
            let _ = unsafe { dlclose(maqbad) };
            Unwan::min_muashir(unwan.cast_const())
        }

        pub(in crate::slate) fn ramz_wahda(wahda: &str, ism: &[u8]) -> Option<Unwan> {
            let ism_wahda = ism_malaf(wahda);
            // SAFETY: `ism_wahda` is NUL-terminated and outlives the call.
            // `RTLD_NOLOAD` makes this a lookup: it returns a handle when the
            // object is already mapped and null otherwise, and never maps
            // anything new.
            let maqbad =
                unsafe { dlopen(ism_wahda.as_ptr().cast::<c_char>(), KASUL | BILA_TAHMEEL) };
            if maqbad.is_null() {
                return None;
            }
            // SAFETY: as `ramz_raisi`.
            let unwan = unsafe { dlsym(maqbad, ism.as_ptr().cast::<c_char>()) };
            // SAFETY: as `ramz_raisi`. The count this call took is dropped; the
            // game's own reference keeps the module mapped, so the resolved
            // address remains valid.
            let _ = unsafe { dlclose(maqbad) };
            Unwan::min_muashir(unwan.cast_const())
        }
    }
}
