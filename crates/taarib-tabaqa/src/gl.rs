//! جي‌إل — the OpenGL backend, and the one API where "restore the state" is a
//! promise that can be broken silently.
//!
//! Direct3D hands the overlay a device with an explicit context object, and
//! Vulkan hands it a command buffer nobody else is recording into. OpenGL hands
//! it a **global state machine that the game owns**, in which every binding the
//! overlay makes is a binding the game did not make and will not expect. There
//! is no scoped state block, no push/pop that covers what a modern draw touches,
//! and no error that fires when the game's next draw call reads a uniform out of
//! the overlay's program. The failure looks like the game's own renderer going
//! wrong two hundred frames later, and there is no line in it with Taarib's name
//! on it.
//!
//! So this module is written around one discipline: [`HalatGl`] enumerates every
//! piece of state the draw touches, reads all of it before the draw, and writes
//! all of it back after — unconditionally, including on the error paths — and
//! then asks the driver whether the restore itself produced an error. That last
//! step is why [`KhataTabaqa::HalaGhayrMustaada`] exists at all. In D3D and
//! Vulkan a failed restore is a `HRESULT` or a `VkResult`; in OpenGL it is a flag
//! in a queue that nobody reads, so the overlay reads it.
//!
//! ## There is no `gl` crate here, deliberately
//!
//! This workspace has no OpenGL bindings dependency and does not want one. The
//! overlay is injected into somebody else's process, where `opengl32.dll` or
//! `libGL.so.1` is *already loaded and already has a current context*; what is
//! needed is not a loader that creates a context but a table of fifty-five
//! function pointers resolved out of the module the game is using. A binding
//! crate would bring thousands of entry points, its own extension registry and
//! its own initialisation order, to do a job that [`DawallGl`] does in one pass.
//!
//! The resolution order is the part that is easy to get wrong, so it is written
//! down: on Windows, `wglGetProcAddress` **first** and `GetProcAddress` on
//! `opengl32.dll` second, because `opengl32.dll` exports only the OpenGL 1.1
//! entry points and every symbol this overlay needs above `glGetError` is an
//! extension address the ICD supplies; on Linux, `glXGetProcAddress` or
//! `eglGetProcAddress` first and `dlsym` second, for the same reason. And
//! `wglGetProcAddress` is documented to return `1`, `2`, `3` or `-1` on failure
//! rather than null on several drivers, which is checked here, because a
//! function pointer of `0x3` called on a game's render thread is a crash whose
//! stack names nothing.
//!
//! ## OpenGL 3.3 core, and nothing older
//!
//! Every entry point in the table is core in 3.3. No `glBegin`, no matrix stack,
//! no client-side vertex arrays, no fixed-function texture environment. This is
//! not modernism for its own sake: a core-profile context — which is what a game
//! shipped after about 2012 asks for — *removes* the fixed-function path, and an
//! overlay that drew through it would draw nothing and report success on exactly
//! the games it exists to serve. The shaders are GLSL `#version 330 core`,
//! compiled at runtime, with the driver's info log carried into
//! [`KhataTabaqa::MawridFashil`] on failure — an overlay that says "shader
//! failed" without the driver's own sentence is an overlay nobody can debug from
//! a bug report.
//!
//! ## The capture is upside down until it is not
//!
//! `glReadPixels` reads from an origin at the **bottom left**. Every rectangle in
//! this crate — [`MustatilBiksel`], [`crate::wajiha::MustatilNisbi`], the atlas
//! mapping, the recognizer's own coordinate space — has its origin at the **top
//! left**. So [`Khattaf::iltaqit`] flips the rows before it returns them. Getting this
//! wrong does not produce an error or a warning: it produces a correctly sized,
//! correctly formatted, upside-down image, out of which the recognizer reads
//! nothing at all, every frame, forever.
//!
//! ## What `sath` can honestly report
//!
//! OpenGL has no equivalent of `IDXGISwapChain::GetDesc`. There is no query that
//! says "the surface you are presenting is B8G8R8A8_UNORM_SRGB at 2560×1440".
//! What there is: the viewport, whatever the platform layer knows about the
//! drawable, and whether `GL_FRAMEBUFFER_SRGB` is enabled. So [`SighatSath::Rgba8`]
//! is reported — and that is honest rather than a guess, because in OpenGL the
//! capture format is the format the *reader* asks for: `glReadPixels` converts
//! on read-back, and this backend asks for `GL_RGBA`/`GL_UNSIGNED_BYTE`. In D3D
//! the format is a property of the swap chain and the overlay must accept what
//! it finds; here it is a property of the request, and the request is RGBA8.

use core::ffi::{c_char, c_void};
use core::fmt;

use crate::khata::KhataTabaqa;
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

// ---------------------------------------------------------------------------
// The scalar vocabulary
// ---------------------------------------------------------------------------

/// `GLenum`, `GLuint` and `GLbitfield` — every unsigned 32-bit GL scalar.
///
/// One alias for three C typedefs because they are one machine type and keeping
/// them apart in Rust would buy nothing but three names to get wrong at a call
/// site the compiler cannot check anyway.
type Adad = u32;

/// `GLint` and `GLsizei`.
type Sahih = i32;

/// `GLboolean`, which is a byte and not a `bool`.
///
/// Kept as `u8` all the way to the call because `bool` in Rust is one byte with
/// exactly two valid bit patterns, and a driver writing `2` into a `bool` out
/// parameter would be undefined behaviour rather than a surprising value.
type Mantiqi = u8;

/// `GLsizeiptr` and `GLintptr` — a byte count or byte offset into a buffer.
type Masafa = isize;

// ---------------------------------------------------------------------------
// The constants this backend uses, at the values the specification gives them
// ---------------------------------------------------------------------------

/// `GL_NO_ERROR`.
const GL_NO_ERROR: Adad = 0;
/// `GL_INVALID_ENUM`.
const GL_INVALID_ENUM: Adad = 0x0500;
/// `GL_INVALID_VALUE`.
const GL_INVALID_VALUE: Adad = 0x0501;
/// `GL_INVALID_OPERATION`.
const GL_INVALID_OPERATION: Adad = 0x0502;
/// `GL_OUT_OF_MEMORY`.
const GL_OUT_OF_MEMORY: Adad = 0x0505;
/// `GL_INVALID_FRAMEBUFFER_OPERATION`.
const GL_INVALID_FRAMEBUFFER_OPERATION: Adad = 0x0506;

/// `GL_FALSE`.
const GL_FALSE: Mantiqi = 0;
/// `GL_TRUE`.
const GL_TRUE: Mantiqi = 1;

/// `GL_VENDOR`.
const GL_VENDOR: Adad = 0x1F00;
/// `GL_RENDERER`.
const GL_RENDERER: Adad = 0x1F01;
/// `GL_VERSION`.
const GL_VERSION: Adad = 0x1F02;

/// `GL_MAX_TEXTURE_SIZE`.
const GL_MAX_TEXTURE_SIZE: Adad = 0x0D33;

/// `GL_VIEWPORT`, four values.
const GL_VIEWPORT: Adad = 0x0BA2;
/// `GL_SCISSOR_BOX`, four values.
const GL_SCISSOR_BOX: Adad = 0x0C10;
/// `GL_SCISSOR_TEST`.
const GL_SCISSOR_TEST: Adad = 0x0C11;
/// `GL_BLEND`.
const GL_BLEND: Adad = 0x0BE2;
/// `GL_DEPTH_TEST`.
const GL_DEPTH_TEST: Adad = 0x0B71;
/// `GL_DEPTH_WRITEMASK`.
const GL_DEPTH_WRITEMASK: Adad = 0x0B72;
/// `GL_CULL_FACE`.
const GL_CULL_FACE: Adad = 0x0B44;
/// `GL_CULL_FACE_MODE`.
const GL_CULL_FACE_MODE: Adad = 0x0B45;
/// `GL_FRONT_FACE`.
const GL_FRONT_FACE: Adad = 0x0B46;
/// `GL_COLOR_WRITEMASK`, four values.
const GL_COLOR_WRITEMASK: Adad = 0x0C23;
/// `GL_POLYGON_MODE`, two values.
const GL_POLYGON_MODE: Adad = 0x0B40;
/// `GL_FRAMEBUFFER_SRGB`.
const GL_FRAMEBUFFER_SRGB: Adad = 0x8DB9;
/// `GL_CURRENT_PROGRAM`.
const GL_CURRENT_PROGRAM: Adad = 0x8B8D;
/// `GL_VERTEX_ARRAY_BINDING`.
const GL_VERTEX_ARRAY_BINDING: Adad = 0x85B5;
/// `GL_ARRAY_BUFFER_BINDING`.
const GL_ARRAY_BUFFER_BINDING: Adad = 0x8894;
/// `GL_ELEMENT_ARRAY_BUFFER_BINDING`.
const GL_ELEMENT_ARRAY_BUFFER_BINDING: Adad = 0x8895;
/// `GL_ACTIVE_TEXTURE`.
const GL_ACTIVE_TEXTURE: Adad = 0x84E0;
/// `GL_TEXTURE_BINDING_2D`, reported for whichever unit is active.
const GL_TEXTURE_BINDING_2D: Adad = 0x8069;
/// `GL_BLEND_SRC_RGB`.
const GL_BLEND_SRC_RGB: Adad = 0x80C9;
/// `GL_BLEND_DST_RGB`.
const GL_BLEND_DST_RGB: Adad = 0x80C8;
/// `GL_BLEND_SRC_ALPHA`.
const GL_BLEND_SRC_ALPHA: Adad = 0x80CB;
/// `GL_BLEND_DST_ALPHA`.
const GL_BLEND_DST_ALPHA: Adad = 0x80CA;
/// `GL_BLEND_EQUATION_RGB`, which is the same token as `GL_BLEND_EQUATION`.
const GL_BLEND_EQUATION_RGB: Adad = 0x8009;
/// `GL_BLEND_EQUATION_ALPHA`.
const GL_BLEND_EQUATION_ALPHA: Adad = 0x883D;
/// `GL_DRAW_FRAMEBUFFER_BINDING`.
const GL_DRAW_FRAMEBUFFER_BINDING: Adad = 0x8CA6;
/// `GL_READ_FRAMEBUFFER_BINDING`.
const GL_READ_FRAMEBUFFER_BINDING: Adad = 0x8CAA;
/// `GL_READ_FRAMEBUFFER`.
const GL_READ_FRAMEBUFFER: Adad = 0x8CA8;
/// `GL_DRAW_FRAMEBUFFER`.
const GL_DRAW_FRAMEBUFFER: Adad = 0x8CA9;
/// `GL_READ_BUFFER`.
const GL_READ_BUFFER: Adad = 0x0C02;
/// `GL_BACK`.
const GL_BACK: Adad = 0x0405;
/// `GL_UNPACK_ALIGNMENT`.
const GL_UNPACK_ALIGNMENT: Adad = 0x0CF5;
/// `GL_PACK_ALIGNMENT`.
const GL_PACK_ALIGNMENT: Adad = 0x0D05;

/// `GL_TEXTURE_2D`.
const GL_TEXTURE_2D: Adad = 0x0DE1;
/// `GL_TEXTURE0`.
const GL_TEXTURE0: Adad = 0x84C0;
/// `GL_RGBA`.
const GL_RGBA: Adad = 0x1908;
/// `GL_RGBA8`.
const GL_RGBA8: Sahih = 0x8058;
/// `GL_UNSIGNED_BYTE`.
const GL_UNSIGNED_BYTE: Adad = 0x1401;
/// `GL_UNSIGNED_INT`.
const GL_UNSIGNED_INT: Adad = 0x1405;
/// `GL_FLOAT`.
const GL_FLOAT: Adad = 0x1406;
/// `GL_TEXTURE_MAG_FILTER`.
const GL_TEXTURE_MAG_FILTER: Adad = 0x2800;
/// `GL_TEXTURE_MIN_FILTER`.
const GL_TEXTURE_MIN_FILTER: Adad = 0x2801;
/// `GL_TEXTURE_WRAP_S`.
const GL_TEXTURE_WRAP_S: Adad = 0x2802;
/// `GL_TEXTURE_WRAP_T`.
const GL_TEXTURE_WRAP_T: Adad = 0x2803;
/// `GL_LINEAR`.
const GL_LINEAR: Sahih = 0x2601;
/// `GL_CLAMP_TO_EDGE`.
const GL_CLAMP_TO_EDGE: Sahih = 0x812F;

/// `GL_ARRAY_BUFFER`.
const GL_ARRAY_BUFFER: Adad = 0x8892;
/// `GL_ELEMENT_ARRAY_BUFFER`.
const GL_ELEMENT_ARRAY_BUFFER: Adad = 0x8893;
/// `GL_STREAM_DRAW`.
const GL_STREAM_DRAW: Adad = 0x88E0;

/// `GL_VERTEX_SHADER`.
const GL_VERTEX_SHADER: Adad = 0x8B31;
/// `GL_FRAGMENT_SHADER`.
const GL_FRAGMENT_SHADER: Adad = 0x8B30;
/// `GL_COMPILE_STATUS`.
const GL_COMPILE_STATUS: Adad = 0x8B81;
/// `GL_LINK_STATUS`.
const GL_LINK_STATUS: Adad = 0x8B82;
/// `GL_INFO_LOG_LENGTH`.
const GL_INFO_LOG_LENGTH: Adad = 0x8B84;

/// `GL_TRIANGLES`.
const GL_TRIANGLES: Adad = 0x0004;
/// `GL_ONE`, as a blend factor.
const GL_ONE: Adad = 1;
/// `GL_ONE_MINUS_SRC_ALPHA`.
const GL_ONE_MINUS_SRC_ALPHA: Adad = 0x0303;
/// `GL_FUNC_ADD`.
const GL_FUNC_ADD: Adad = 0x8006;
/// `GL_FILL`, the polygon mode the overlay forces.
///
/// A game left in `GL_LINE` would otherwise draw the overlay's quads as
/// wireframe triangles, which is a real and frequently reported symptom of
/// overlays that save the polygon mode and forget to set it.
const GL_FILL: Adad = 0x1B02;
/// `GL_FRONT_AND_BACK`.
const GL_FRONT_AND_BACK: Adad = 0x0408;

/// The largest atlas this build will upload, in bytes.
///
/// A hundred and twenty-eight mebibytes of RGBA8 is a 5792×5792 atlas, which is
/// larger than any font page Taarib produces by two orders of magnitude. The
/// ceiling exists because `arfa_lawha` takes a length from a caller and turns it
/// into a GPU allocation inside somebody's game, and a wrong length there is an
/// allocation failure the player experiences as their game dying.
const AQSA_LAWHA: u64 = 128 * 1024 * 1024;

/// The most quads one batch may carry.
///
/// Sixteen thousand quads is about eleven thousand glyphs plus their backing
/// plates — more text than fits on a 4K screen at a readable size. A batch above
/// it is a bug in the batch builder, not a frame worth drawing, and drawing it
/// anyway would allocate sixty megabytes on a render thread.
const AQSA_QITAAT: usize = 16_384;

/// The vertex attribute slot the position is bound to.
const MAWQI_MAWDI: Adad = 0;
/// The vertex attribute slot the atlas coordinate is bound to.
const MAWQI_KHAREETA: Adad = 1;
/// The vertex attribute slot the premultiplied colour is bound to.
const MAWQI_LAWN: Adad = 2;
/// The vertex attribute slot the "sample the atlas" flag is bound to.
const MAWQI_NASIJ: Adad = 3;

// ---------------------------------------------------------------------------
// Resolving the entry points
// ---------------------------------------------------------------------------

/// Reinterprets a resolved address as a function pointer of the wanted shape.
///
/// Returns [`None`] rather than transmuting when `T` is not pointer-sized, which
/// cannot happen for a function pointer type and is checked anyway because the
/// alternative is a `transmute_copy` reading past the end of its source.
///
/// # Safety
///
/// `muashir` must be the address the platform returned for an OpenGL entry point
/// whose C signature is exactly `T`. Nothing here can check that, and a `T` that
/// disagrees with the symbol is undefined behaviour at the first call.
const unsafe fn ka_dalla<T: Copy>(muashir: *const c_void) -> Option<T> {
    if size_of::<T>() != size_of::<*const c_void>() {
        return None;
    }
    // SAFETY: `T` is pointer-sized by the check above, and the caller guarantees
    // the address is a function of exactly that signature. `transmute_copy`
    // reads `size_of::<T>()` bytes out of a live `*const c_void` local, which is
    // the same size.
    Some(unsafe { core::mem::transmute_copy::<*const c_void, T>(&muashir) })
}

/// A NUL-terminated copy of an ASCII symbol name.
///
/// `CString::new` would do this and return a `Result` for an interior NUL that
/// cannot occur in a symbol literal, which would put a failure branch in the
/// loader for a case the loader cannot reach. This cannot fail, so it does not
/// return a `Result`.
fn ism_c(ism: &str) -> Vec<u8> {
    let mut khaam = Vec::with_capacity(ism.len() + 1);
    khaam.extend_from_slice(ism.as_bytes());
    khaam.push(0);
    khaam
}

/// The module the entry points come out of, and the platform's own resolver.
///
/// Held for the life of the backend rather than closed after loading, because
/// the fallback path — the module's direct exports — is consulted per symbol and
/// closing the handle would invalidate every address already handed out on the
/// platforms where a `dlclose` can unmap.
struct MasdarDawall {
    /// The module handle, as the platform's loader returned it.
    maktaba: *mut c_void,
    /// The module's name, for [`KhataTabaqa::MaktabaMafquda`].
    ism: &'static str,
    /// The extension-address function, when the platform has one.
    ///
    /// [`None`] on macOS, where every OpenGL entry point is a direct export of
    /// the framework and there is no `wglGetProcAddress` equivalent.
    mustakhrij: Option<unsafe extern "system" fn(*const c_char) -> *const c_void>,
}

impl MasdarDawall {
    /// Resolves one symbol, extension path first.
    ///
    /// The order is not interchangeable. On Windows `opengl32.dll` exports the
    /// OpenGL 1.1 entry points and *stubs that route through the ICD*, and on
    /// several drivers the exported `glActiveTexture` is not the one the current
    /// context wants; `wglGetProcAddress` is. On Linux the two agree for core
    /// functions and disagree for anything an ICD supplies through GLX, which is
    /// the same rule with different names on it.
    fn unwan(&self, ramz: &str) -> Option<*const c_void> {
        let ism = ism_c(ramz);
        if let Some(mustakhrij) = self.mustakhrij {
            // SAFETY: `ism` is a live NUL-terminated byte buffer that outlives
            // the call, and `mustakhrij` is the platform's own extension
            // resolver, which reads the name and returns an address or a
            // failure sentinel. It is the documented signature for
            // `wglGetProcAddress`, `glXGetProcAddress` and `eglGetProcAddress`
            // alike.
            let khaam = unsafe { mustakhrij(ism.as_ptr().cast::<c_char>()) };
            if maqbul(khaam) {
                return Some(khaam);
            }
        }
        // SAFETY: `self.maktaba` is a handle this type obtained from the
        // platform loader and has not closed, and `ism` is NUL-terminated and
        // live for the call.
        let khaam = unsafe { ramz_maktaba(self.maktaba, ism.as_ptr().cast::<c_char>()) };
        if maqbul(khaam) { Some(khaam) } else { None }
    }

    /// Resolves one entry point into its function-pointer type, or names it.
    ///
    /// # Safety
    ///
    /// `T` must be the function-pointer type of the C signature the OpenGL
    /// specification gives `ramz`.
    unsafe fn dalla<T: Copy>(&self, ramz: &'static str) -> Result<T, KhataTabaqa> {
        let mafquda = || KhataTabaqa::MaktabaMafquda {
            maktaba: self.ism.to_owned(),
            ramz: ramz.to_owned(),
        };
        let muashir = self.unwan(ramz).ok_or_else(mafquda)?;
        // SAFETY: the caller guarantees `T` matches this symbol's signature, and
        // `muashir` is the address the platform returned for that symbol.
        unsafe { ka_dalla::<T>(muashir) }.ok_or_else(mafquda)
    }
}

/// Whether a resolver's return value is an address rather than a failure.
///
/// `wglGetProcAddress` is documented to return `NULL`, and is observed on
/// several Windows ICDs to return `1`, `2`, `3` or `-1` instead. Those are the
/// values MSDN itself lists, and a table built without this check contains a
/// function pointer of `0x00000003` that crashes at the first draw with a stack
/// naming nothing at all.
fn maqbul(muashir: *const c_void) -> bool {
    let raqm = muashir as usize;
    raqm > 3 && raqm != usize::MAX
}

// ---------------------------------------------------------------------------
// Platform: Windows
// ---------------------------------------------------------------------------

/// The Windows loader entry points, declared rather than imported.
///
/// The `windows` crate is a dependency of this crate and is deliberately not
/// used here. Three symbols with signatures fixed since Windows NT 3.1 do not
/// need a binding generator, and declaring them keeps this module's two platform
/// paths the same shape — a `dlopen`/`dlsym` pair on one side and a
/// `LoadLibrary`/`GetProcAddress` pair on the other — which is what makes the
/// resolution *order* readable as one idea instead of two.
#[cfg(windows)]
unsafe extern "system" {
    /// `GetModuleHandleA`, which finds a module the process already loaded.
    fn GetModuleHandleA(ism: *const c_char) -> *mut c_void;
    /// `LoadLibraryA`, used only when the module is not already present.
    fn LoadLibraryA(ism: *const c_char) -> *mut c_void;
    /// `GetProcAddress`, the direct-export fallback.
    fn GetProcAddress(maktaba: *mut c_void, ism: *const c_char) -> *const c_void;
}

/// One symbol out of a module's own export table.
///
/// # Safety
///
/// `maktaba` must be a live module handle from this platform's loader and `ism`
/// a NUL-terminated name readable for the call.
#[cfg(windows)]
unsafe fn ramz_maktaba(maktaba: *mut c_void, ism: *const c_char) -> *const c_void {
    if maktaba.is_null() {
        return core::ptr::null();
    }
    // SAFETY: the caller guarantees the handle is live and the name readable,
    // which is exactly `GetProcAddress`'s contract.
    unsafe { GetProcAddress(maktaba, ism) }
}

/// Opens the OpenGL module the game is already using.
///
/// `GetModuleHandleA` before `LoadLibraryA`: the overlay runs inside a process
/// that is *already drawing*, so `opengl32.dll` is loaded and has a current
/// context. Calling `LoadLibraryA` first would increment a reference count on a
/// module this crate does not own and would not be able to drop safely from a
/// render thread.
#[cfg(windows)]
fn iftah_masdar() -> Result<MasdarDawall, KhataTabaqa> {
    const ISM: &str = "opengl32.dll";
    let ism = ism_c(ISM);
    // SAFETY: `ism` is a live NUL-terminated buffer for both calls. Both
    // functions are total over any name and report absence by returning null.
    let mut maktaba = unsafe { GetModuleHandleA(ism.as_ptr().cast::<c_char>()) };
    if maktaba.is_null() {
        // SAFETY: as above.
        maktaba = unsafe { LoadLibraryA(ism.as_ptr().cast::<c_char>()) };
    }
    if maktaba.is_null() {
        return Err(KhataTabaqa::MaktabaMafquda {
            maktaba: ISM.to_owned(),
            ramz: "the module itself".to_owned(),
        });
    }

    let ism_wgl = ism_c("wglGetProcAddress");
    // SAFETY: the handle is non-null and came from the loader above; the name is
    // live and NUL-terminated.
    let khaam = unsafe { GetProcAddress(maktaba, ism_wgl.as_ptr().cast::<c_char>()) };
    let mustakhrij = if maqbul(khaam) {
        // SAFETY: `wglGetProcAddress` has exactly the signature
        // `PROC WINAPI wglGetProcAddress(LPCSTR)`, which is
        // `extern "system" fn(*const c_char) -> *const c_void`.
        unsafe { ka_dalla::<unsafe extern "system" fn(*const c_char) -> *const c_void>(khaam) }
    } else {
        None
    };

    Ok(MasdarDawall { maktaba, ism: ISM, mustakhrij })
}

// ---------------------------------------------------------------------------
// Platform: everything with a dynamic linker
// ---------------------------------------------------------------------------

// The dynamic linker, declared rather than depended on. This crate has no
// `libc` dependency and adding one to call two functions whose signatures have
// not changed since SunOS would be a dependency for its own sake.
// `extern "system"` and `extern "C"` are the same ABI on every non-Windows
// target this product builds for, which is why one declaration serves both
// platform paths. Plain comments rather than a doc comment because rustdoc
// documents the items inside an extern block, never the block itself.
#[cfg(all(unix, not(windows)))]
unsafe extern "system" {
    /// `dlopen`.
    fn dlopen(malaf: *const c_char, alam: i32) -> *mut c_void;
    /// `dlsym`.
    fn dlsym(maqbad: *mut c_void, ism: *const c_char) -> *mut c_void;
}

/// `RTLD_LAZY`, the same value on glibc, musl and Darwin.
#[cfg(all(unix, not(windows)))]
const RTLD_LAZY: i32 = 1;

/// `RTLD_NOLOAD`, which differs between glibc and Darwin.
///
/// Consulted first so the overlay never *loads* a GL implementation into a
/// process that does not have one. A game rendering through Vulkan that had
/// `libGL.so.1` pulled in by a Taarib probe would be a game whose driver stack
/// changed because Taarib looked at it.
#[cfg(all(unix, not(target_vendor = "apple"), not(windows)))]
const RTLD_NOLOAD: i32 = 0x0000_0004;

/// `RTLD_NOLOAD` on Darwin, where the bit is `0x10`.
#[cfg(all(unix, target_vendor = "apple"))]
const RTLD_NOLOAD: i32 = 0x0000_0010;

/// One symbol out of a module's own export table.
///
/// # Safety
///
/// `maktaba` must be a live handle from `dlopen` and `ism` a NUL-terminated name
/// readable for the call.
#[cfg(all(unix, not(windows)))]
unsafe fn ramz_maktaba(maktaba: *mut c_void, ism: *const c_char) -> *const c_void {
    if maktaba.is_null() {
        return core::ptr::null();
    }
    // SAFETY: the caller guarantees the handle is live and the name readable,
    // which is `dlsym`'s contract.
    unsafe { dlsym(maktaba, ism) }.cast_const()
}

/// The libraries a GL context can be behind, in the order they are tried.
///
/// GLX before EGL before the Apple framework: a game on X11 resolves through
/// `libGL.so.1`, a game on Wayland or one using EGL on X11 resolves through
/// `libEGL.so.1`, and both may be present at once — in which case the one the
/// game actually made current is the one that answers `glGetString`, which is
/// what [`iftah_masdar`] checks before accepting a candidate.
#[cfg(all(unix, not(target_vendor = "apple"), not(windows)))]
const MURASHAHAT: &[(&str, &[&str])] = &[
    ("libGL.so.1", &["glXGetProcAddress", "glXGetProcAddressARB"]),
    ("libGL.so", &["glXGetProcAddress", "glXGetProcAddressARB"]),
    ("libEGL.so.1", &["eglGetProcAddress"]),
    ("libEGL.so", &["eglGetProcAddress"]),
    ("libGLESv2.so.2", &["eglGetProcAddress"]),
];

/// The Apple framework, which has no extension resolver at all.
#[cfg(all(unix, target_vendor = "apple"))]
const MURASHAHAT: &[(&str, &[&str])] =
    &[("/System/Library/Frameworks/OpenGL.framework/OpenGL", &[])];

/// Opens the OpenGL module the game is already using.
///
/// A candidate is accepted only when it exports `glGetString`, because a process
/// can have `libEGL.so.1` present for a display server binding while rendering
/// through GLX, and a table built out of the wrong one resolves every symbol and
/// draws into nothing.
#[cfg(all(unix, not(windows)))]
fn iftah_masdar() -> Result<MasdarDawall, KhataTabaqa> {
    let shahid = ism_c("glGetString");
    let mut jurribat: Vec<&str> = Vec::new();

    for &(ism, asma_mustakhrij) in MURASHAHAT {
        jurribat.push(ism);
        let masar = ism_c(ism);
        // SAFETY: `masar` is a live NUL-terminated path for the call. `dlopen`
        // is total over any path and reports failure by returning null.
        let mut maktaba =
            unsafe { dlopen(masar.as_ptr().cast::<c_char>(), RTLD_LAZY | RTLD_NOLOAD) };
        if maktaba.is_null() {
            // SAFETY: as above. The second attempt drops `RTLD_NOLOAD`, which is
            // reached only when the process does not already have the library —
            // a case the probe is allowed to resolve by loading it, because at
            // that point nothing is rendering through it either.
            maktaba = unsafe { dlopen(masar.as_ptr().cast::<c_char>(), RTLD_LAZY) };
        }
        if maktaba.is_null() {
            continue;
        }
        // SAFETY: the handle is non-null and came from `dlopen`; the name is
        // live and NUL-terminated.
        if !maqbul(unsafe { ramz_maktaba(maktaba, shahid.as_ptr().cast::<c_char>()) }) {
            continue;
        }

        let mut mustakhrij = None;
        for &ism_dalla in asma_mustakhrij {
            let ism_khaam = ism_c(ism_dalla);
            // SAFETY: live handle, live NUL-terminated name.
            let khaam = unsafe { ramz_maktaba(maktaba, ism_khaam.as_ptr().cast::<c_char>()) };
            if !maqbul(khaam) {
                continue;
            }
            // SAFETY: `glXGetProcAddress`, `glXGetProcAddressARB` and
            // `eglGetProcAddress` all have the signature
            // `void (*(*)(const char *))(void)`, which is pointer-in,
            // pointer-out; `extern "system"` is `extern "C"` on this target.
            mustakhrij = unsafe {
                ka_dalla::<unsafe extern "system" fn(*const c_char) -> *const c_void>(khaam)
            };
            if mustakhrij.is_some() {
                break;
            }
        }
        return Ok(MasdarDawall { maktaba, ism, mustakhrij });
    }

    Err(KhataTabaqa::MaktabaMafquda {
        maktaba: jurribat.join(", "),
        ramz: "glGetString".to_owned(),
    })
}

// ---------------------------------------------------------------------------
// Platform: nothing at all
// ---------------------------------------------------------------------------

/// One symbol out of a module's own export table, where there is no loader.
///
/// # Safety
///
/// Trivially satisfied: this build reads nothing.
#[cfg(not(any(windows, unix)))]
unsafe fn ramz_maktaba(_maktaba: *mut c_void, _ism: *const c_char) -> *const c_void {
    core::ptr::null()
}

/// Refuses, on a target with no dynamic loader.
///
/// The crate compiles for every platform including the ones where no graphics
/// API in [`WajihatRusum`] exists — that is [`crate::wajiha`]'s stated rule — so
/// this path exists to keep that true rather than to be reached.
#[cfg(not(any(windows, unix)))]
fn iftah_masdar() -> Result<MasdarDawall, KhataTabaqa> {
    Err(KhataTabaqa::MaktabaMafquda {
        maktaba: "an OpenGL implementation".to_owned(),
        ramz: "this target has no dynamic loader".to_owned(),
    })
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// Declares the entry-point table and the one pass that fills it.
///
/// A macro rather than two hand-written lists, because the failure mode of two
/// hand-written lists is a field that is declared and never resolved: it holds
/// whatever the struct literal put there, the loader reports success, and the
/// crash arrives at the first draw. Here a symbol exists in exactly one place
/// and cannot be half-added.
macro_rules! jadwal_gl {
    ( $( $haql:ident : $ramz:literal => $sigha:ty ; )+ ) => {
        /// Every OpenGL entry point the overlay calls, resolved once.
        ///
        /// Each field is a non-optional function pointer: a table that exists at
        /// all is a table where every symbol was found, because the alternative
        /// is an `Option` unwrapped at fifty call sites on a render thread. A
        /// symbol the driver does not export stops construction with
        /// [`KhataTabaqa::MaktabaMafquda`] naming that exact symbol, which is the
        /// sentence a bug report needs.
        #[allow(
            dead_code,
            reason = "`glBlendFunc` is resolved and never called: the restore uses \
                      `glBlendFuncSeparate` because the state it reads back is separate. It \
                      stays in the table because its absence is diagnostic — a module that \
                      does not export it is not an OpenGL implementation, and finding that \
                      out while resolving is better than finding it out at the first draw"
        )]
        pub struct DawallGl {
            $( $haql: $sigha, )+
            /// The context's version, as `(major, minor)`.
            isdar: (u32, u32),
            /// `GL_VENDOR`, `GL_RENDERER` and `GL_VERSION`, for the report.
            wasf: String,
        }

        impl DawallGl {
            /// Resolves every entry point out of one module.
            fn min_masdar(masdar: &MasdarDawall) -> Result<Self, KhataTabaqa> {
                // SAFETY: every signature in the table below is transcribed from
                // the OpenGL 3.3 core profile specification, where all of these
                // entry points are declared, so each resolution asks for the
                // shape its own symbol has. `extern "system"` is `APIENTRY` on
                // Windows and `extern "C"` everywhere else, which is what the
                // specification's `GLAPIENTRY` expands to on each.
                let mut dawall = unsafe {
                    Self {
                        $( $haql: masdar.dalla::<$sigha>($ramz)?, )+
                        isdar: (0, 0),
                        wasf: String::new(),
                    }
                };
                dawall.isdar = dawall.iqra_isdar();
                dawall.wasf = dawall.iqra_wasf();
                Ok(dawall)
            }

            /// How many entry points this table carries.
            #[must_use]
            pub const fn adad_dawall() -> usize {
                [ $( stringify!($haql) ),+ ].len()
            }
        }
    };
}

jadwal_gl! {
    get_integerv: "glGetIntegerv" => unsafe extern "system" fn(Adad, *mut Sahih);
    get_integeri_v: "glGetIntegeri_v" => unsafe extern "system" fn(Adad, Adad, *mut Sahih);
    get_booleanv: "glGetBooleanv" => unsafe extern "system" fn(Adad, *mut Mantiqi);
    get_error: "glGetError" => unsafe extern "system" fn() -> Adad;
    enable: "glEnable" => unsafe extern "system" fn(Adad);
    disable: "glDisable" => unsafe extern "system" fn(Adad);
    is_enabled: "glIsEnabled" => unsafe extern "system" fn(Adad) -> Mantiqi;
    blend_func: "glBlendFunc" => unsafe extern "system" fn(Adad, Adad);
    blend_func_separate: "glBlendFuncSeparate"
        => unsafe extern "system" fn(Adad, Adad, Adad, Adad);
    blend_equation: "glBlendEquation" => unsafe extern "system" fn(Adad);
    get_string: "glGetString" => unsafe extern "system" fn(Adad) -> *const u8;
    viewport: "glViewport" => unsafe extern "system" fn(Sahih, Sahih, Sahih, Sahih);
    scissor: "glScissor" => unsafe extern "system" fn(Sahih, Sahih, Sahih, Sahih);
    gen_buffers: "glGenBuffers" => unsafe extern "system" fn(Sahih, *mut Adad);
    bind_buffer: "glBindBuffer" => unsafe extern "system" fn(Adad, Adad);
    buffer_data: "glBufferData"
        => unsafe extern "system" fn(Adad, Masafa, *const c_void, Adad);
    buffer_sub_data: "glBufferSubData"
        => unsafe extern "system" fn(Adad, Masafa, Masafa, *const c_void);
    delete_buffers: "glDeleteBuffers" => unsafe extern "system" fn(Sahih, *const Adad);
    gen_vertex_arrays: "glGenVertexArrays" => unsafe extern "system" fn(Sahih, *mut Adad);
    bind_vertex_array: "glBindVertexArray" => unsafe extern "system" fn(Adad);
    delete_vertex_arrays: "glDeleteVertexArrays"
        => unsafe extern "system" fn(Sahih, *const Adad);
    vertex_attrib_pointer: "glVertexAttribPointer"
        => unsafe extern "system" fn(Adad, Sahih, Adad, Mantiqi, Sahih, *const c_void);
    enable_vertex_attrib_array: "glEnableVertexAttribArray"
        => unsafe extern "system" fn(Adad);
    create_shader: "glCreateShader" => unsafe extern "system" fn(Adad) -> Adad;
    shader_source: "glShaderSource"
        => unsafe extern "system" fn(Adad, Sahih, *const *const c_char, *const Sahih);
    compile_shader: "glCompileShader" => unsafe extern "system" fn(Adad);
    get_shaderiv: "glGetShaderiv" => unsafe extern "system" fn(Adad, Adad, *mut Sahih);
    get_shader_info_log: "glGetShaderInfoLog"
        => unsafe extern "system" fn(Adad, Sahih, *mut Sahih, *mut c_char);
    create_program: "glCreateProgram" => unsafe extern "system" fn() -> Adad;
    attach_shader: "glAttachShader" => unsafe extern "system" fn(Adad, Adad);
    link_program: "glLinkProgram" => unsafe extern "system" fn(Adad);
    get_programiv: "glGetProgramiv" => unsafe extern "system" fn(Adad, Adad, *mut Sahih);
    get_program_info_log: "glGetProgramInfoLog"
        => unsafe extern "system" fn(Adad, Sahih, *mut Sahih, *mut c_char);
    use_program: "glUseProgram" => unsafe extern "system" fn(Adad);
    delete_shader: "glDeleteShader" => unsafe extern "system" fn(Adad);
    delete_program: "glDeleteProgram" => unsafe extern "system" fn(Adad);
    get_uniform_location: "glGetUniformLocation"
        => unsafe extern "system" fn(Adad, *const c_char) -> Sahih;
    uniform_matrix4fv: "glUniformMatrix4fv"
        => unsafe extern "system" fn(Sahih, Sahih, Mantiqi, *const f32);
    uniform1i: "glUniform1i" => unsafe extern "system" fn(Sahih, Sahih);
    gen_textures: "glGenTextures" => unsafe extern "system" fn(Sahih, *mut Adad);
    bind_texture: "glBindTexture" => unsafe extern "system" fn(Adad, Adad);
    tex_image2d: "glTexImage2D"
        => unsafe extern "system" fn(
            Adad, Sahih, Sahih, Sahih, Sahih, Sahih, Adad, Adad, *const c_void,
        );
    tex_parameteri: "glTexParameteri" => unsafe extern "system" fn(Adad, Adad, Sahih);
    delete_textures: "glDeleteTextures" => unsafe extern "system" fn(Sahih, *const Adad);
    active_texture: "glActiveTexture" => unsafe extern "system" fn(Adad);
    draw_elements: "glDrawElements"
        => unsafe extern "system" fn(Adad, Sahih, Adad, *const c_void);
    pixel_storei: "glPixelStorei" => unsafe extern "system" fn(Adad, Sahih);
    read_pixels: "glReadPixels"
        => unsafe extern "system" fn(Sahih, Sahih, Sahih, Sahih, Adad, Adad, *mut c_void);
    read_buffer: "glReadBuffer" => unsafe extern "system" fn(Adad);
    depth_mask: "glDepthMask" => unsafe extern "system" fn(Mantiqi);
    color_mask: "glColorMask" => unsafe extern "system" fn(Mantiqi, Mantiqi, Mantiqi, Mantiqi);
    cull_face: "glCullFace" => unsafe extern "system" fn(Adad);
    front_face: "glFrontFace" => unsafe extern "system" fn(Adad);
    polygon_mode: "glPolygonMode" => unsafe extern "system" fn(Adad, Adad);
    bind_framebuffer: "glBindFramebuffer" => unsafe extern "system" fn(Adad, Adad);
}

impl fmt::Debug for DawallGl {
    // The entry points themselves are elided: their addresses say nothing a
    // reader of a diagnostics bundle can use, and printing a hundred of them
    // buries the version and driver strings that answer every real question.
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("DawallGl")
            .field("isdar", &self.isdar)
            .field("wasf", &self.wasf)
            .field("adad_dawall", &Self::adad_dawall())
            .finish_non_exhaustive()
    }
}

impl DawallGl {
    /// Resolves every entry point out of the module the game is using.
    ///
    /// **Must be called with the game's context current on this thread.** Two
    /// reasons, and the second is the one that bites: `wglGetProcAddress` is
    /// specified to resolve against the *current* context, so a table built with
    /// no context current is a table of nulls on Windows; and the version and
    /// driver strings are read here, through `glGetString`, which returns null
    /// without a context. The hook in `khataf` calls this from inside the swap
    /// hook, which is the one place a context is guaranteed.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MaktabaMafquda`] naming the module and the first symbol
    /// that could not be resolved. Every symbol is required: this backend has no
    /// reduced path it could take with a subset, because the subset that would
    /// be missing on a real driver is the one that makes a core profile work.
    pub fn hammil() -> Result<Self, KhataTabaqa> {
        let masdar = iftah_masdar()?;
        Self::min_masdar(&masdar)
    }

    /// The context's version, as `(major, minor)`.
    #[must_use]
    pub const fn isdar(&self) -> (u32, u32) {
        self.isdar
    }

    /// The vendor, renderer and version strings, for the diagnostics bundle.
    ///
    /// Every OpenGL bug report in existence is answered with "which driver?",
    /// and this is the line that answers it before anyone has to ask.
    #[must_use]
    pub fn wasf(&self) -> &str {
        &self.wasf
    }
}

// ---------------------------------------------------------------------------
// Reading the machine
// ---------------------------------------------------------------------------

impl DawallGl {
    /// A `glGetString` result as an owned Rust string.
    ///
    /// Bounded at four kibibytes. The strings this reads are vendor-supplied and
    /// NUL-terminated by the specification, and a driver that returns an
    /// unterminated buffer would otherwise walk off the end of a mapping on a
    /// game's render thread. Four kibibytes is an order of magnitude past the
    /// longest `GL_VERSION` any driver has ever produced.
    fn nass(&self, mawdu: Adad) -> String {
        const AQSA: usize = 4096;
        // SAFETY: `glGetString` takes a name and returns either null or a
        // pointer to a NUL-terminated, statically allocated, driver-owned
        // string, which is its whole contract.
        let khaam = unsafe { (self.get_string)(mawdu) };
        if khaam.is_null() {
            return String::new();
        }
        let mut tul = 0usize;
        while tul < AQSA {
            // SAFETY: the driver's string is NUL-terminated, so every byte up to
            // and including the terminator is inside the allocation; the scan
            // stops at the terminator and is bounded regardless.
            if unsafe { khaam.add(tul).read() } == 0 {
                break;
            }
            tul += 1;
        }
        // SAFETY: `tul` bytes were just read one at a time from this pointer, so
        // the whole span is readable and `u8` has no alignment requirement.
        let bayt = unsafe { core::slice::from_raw_parts(khaam, tul) };
        String::from_utf8_lossy(bayt).into_owned()
    }

    /// The context's version, parsed out of `GL_VERSION`.
    ///
    /// `GL_VERSION` is `"<major>.<minor>[.<release>] <vendor text>"` on a
    /// desktop context and `"OpenGL ES <major>.<minor> <vendor text>"` on an ES
    /// one, so the parse skips to the first digit and then reads two integers.
    /// A version that does not parse is reported as `(0, 0)`, which every
    /// version-gated decision in this module treats as "assume the older path".
    fn iqra_isdar(&self) -> (u32, u32) {
        let nass = self.nass(GL_VERSION);
        let mut raqmiya = nass.split(|harf: char| !harf.is_ascii_digit());
        let kabir = raqmiya.find(|juz: &&str| !juz.is_empty()).and_then(|juz| juz.parse().ok());
        let sagheer = raqmiya.next().and_then(|juz| juz.parse().ok());
        match (kabir, sagheer) {
            (Some(kabir), Some(sagheer)) => (kabir, sagheer),
            (Some(kabir), None) => (kabir, 0),
            _ => (0, 0),
        }
    }

    /// Vendor, renderer and version as one line.
    fn iqra_wasf(&self) -> String {
        format!(
            "{} / {} / {}",
            self.nass(GL_VENDOR),
            self.nass(GL_RENDERER),
            self.nass(GL_VERSION)
        )
    }

    /// One integer of state.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state the OpenGL 3.3 specification gives **exactly
    /// one** value. A multi-valued name here writes past the end of the local.
    unsafe fn sahih(&self, mawdu: Adad) -> Sahih {
        let mut qeema: Sahih = 0;
        // SAFETY: the caller guarantees `mawdu` is single-valued, and `qeema` is
        // one live, aligned `GLint` for the duration of the call.
        unsafe { (self.get_integerv)(mawdu, &raw mut qeema) };
        qeema
    }

    /// Two integers of state, such as `GL_POLYGON_MODE`.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state with **exactly two** values.
    unsafe fn sahih_ithnan(&self, mawdu: Adad) -> [Sahih; 2] {
        let mut qeema: [Sahih; 2] = [0; 2];
        // SAFETY: the caller guarantees `mawdu` is two-valued, and `qeema` is a
        // live, aligned array of two `GLint`s for the duration of the call.
        unsafe { (self.get_integerv)(mawdu, qeema.as_mut_ptr()) };
        qeema
    }

    /// Four integers of state, such as `GL_VIEWPORT` and `GL_SCISSOR_BOX`.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state with **exactly four** values.
    unsafe fn sahih_arbaa(&self, mawdu: Adad) -> [Sahih; 4] {
        let mut qeema: [Sahih; 4] = [0; 4];
        // SAFETY: the caller guarantees `mawdu` is four-valued, and `qeema` is a
        // live, aligned array of four `GLint`s for the duration of the call.
        unsafe { (self.get_integerv)(mawdu, qeema.as_mut_ptr()) };
        qeema
    }

    /// One integer of an indexed state, at one index.
    ///
    /// # Safety
    ///
    /// `mawdu` must name an indexed state with exactly one value per index, and
    /// `fahras` must be below that state's limit for this context.
    unsafe fn sahih_mufahras(&self, mawdu: Adad, fahras: Adad) -> Sahih {
        let mut qeema: Sahih = 0;
        // SAFETY: the caller guarantees the name is a single-valued indexed
        // state and the index is in range; `qeema` is one live, aligned `GLint`.
        unsafe { (self.get_integeri_v)(mawdu, fahras, &raw mut qeema) };
        qeema
    }

    /// One boolean of state.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state with **exactly one** value.
    unsafe fn mantiqi(&self, mawdu: Adad) -> Mantiqi {
        let mut qeema: Mantiqi = GL_FALSE;
        // SAFETY: the caller guarantees `mawdu` is single-valued, and `qeema` is
        // one live `GLboolean`, which is a byte with no alignment requirement.
        unsafe { (self.get_booleanv)(mawdu, &raw mut qeema) };
        qeema
    }

    /// Four booleans of state, which is `GL_COLOR_WRITEMASK` and nothing else.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state with **exactly four** values.
    unsafe fn mantiqi_arbaa(&self, mawdu: Adad) -> [Mantiqi; 4] {
        let mut qeema: [Mantiqi; 4] = [GL_FALSE; 4];
        // SAFETY: the caller guarantees `mawdu` is four-valued, and `qeema` is a
        // live array of four bytes for the duration of the call.
        unsafe { (self.get_booleanv)(mawdu, qeema.as_mut_ptr()) };
        qeema
    }

    /// Whether a capability is enabled.
    fn mufaal(&self, hala: Adad) -> bool {
        // SAFETY: `glIsEnabled` takes a capability name and returns a
        // `GLboolean`, writing nothing. An unrecognised name sets
        // `GL_INVALID_ENUM` and returns `GL_FALSE`; it is not a memory hazard.
        unsafe { (self.is_enabled)(hala) != GL_FALSE }
    }

    /// Enables or disables a capability, from a saved boolean.
    fn ashil(&self, hala: Adad, mufaal: bool) {
        if mufaal {
            // SAFETY: `glEnable` takes a capability name and writes nothing.
            unsafe { (self.enable)(hala) };
        } else {
            // SAFETY: `glDisable` takes a capability name and writes nothing.
            unsafe { (self.disable)(hala) };
        }
    }

    /// Drains the error queue and returns the first error it held.
    ///
    /// OpenGL keeps a *set* of flags rather than one, and `glGetError` clears
    /// and reports one per call, so a single call leaves the queue dirty for the
    /// next reader. The loop is bounded at thirty-two because a driver that
    /// never returns `GL_NO_ERROR` would otherwise spin forever on a render
    /// thread — which is a hang the player experiences as their game freezing,
    /// caused by an overlay that was trying to be careful.
    fn ifragh(&self) -> Adad {
        let mut awwal = GL_NO_ERROR;
        for _ in 0..32u32 {
            // SAFETY: `glGetError` takes nothing, writes nothing, and returns
            // the next error flag or `GL_NO_ERROR`.
            let khata = unsafe { (self.get_error)() };
            if khata == GL_NO_ERROR {
                break;
            }
            if awwal == GL_NO_ERROR {
                awwal = khata;
            }
        }
        awwal
    }
}

/// The specification's own name for an error code.
///
/// Written out rather than printed as a number because the number is the thing
/// nobody remembers and the name is the thing that can be searched for.
const fn ism_khata(khata: Adad) -> &'static str {
    match khata {
        GL_NO_ERROR => "GL_NO_ERROR",
        GL_INVALID_ENUM => "GL_INVALID_ENUM",
        GL_INVALID_VALUE => "GL_INVALID_VALUE",
        GL_INVALID_OPERATION => "GL_INVALID_OPERATION",
        GL_OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
        GL_INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
        _ => "an error code this build does not name",
    }
}

// ---------------------------------------------------------------------------
// The shaders
// ---------------------------------------------------------------------------

/// The vertex stage: pixels in, clip space out.
///
/// The projection is a uniform rather than a constant folded into the vertices
/// because the surface changes — a fullscreen transition, a resolution change —
/// and rebuilding every vertex for a new surface would mean the batch built one
/// frame earlier could not be drawn at all. With the matrix in a uniform, a
/// batch is geometry in surface pixels and the surface is a separate fact.
const SHIFRAT_RAAS: &str = "#version 330 core\n\
layout(location = 0) in vec2 mawdi;\n\
layout(location = 1) in vec2 khareeta;\n\
layout(location = 2) in vec4 lawn;\n\
layout(location = 3) in float nasij;\n\
uniform mat4 isqat;\n\
out vec2 khareeta_marra;\n\
out vec4 lawn_marra;\n\
out float nasij_marra;\n\
void main() {\n\
    khareeta_marra = khareeta;\n\
    lawn_marra = lawn;\n\
    nasij_marra = nasij;\n\
    gl_Position = isqat * vec4(mawdi, 0.0, 1.0);\n\
}\n";

/// The fragment stage: premultiplied linear RGBA, encoded only when it must be.
///
/// Two things in here are load-bearing and neither is obvious.
///
/// The first is `nasij_marra`. A backing plate has no atlas rectangle
/// ([`QitaRasm::khareeta`] is [`None`]) and a glyph does, and the difference
/// travels as a per-vertex float rather than a uniform so that plates and glyphs
/// go out in **one** draw call. As a uniform it would be one draw call per quad,
/// which for a dialogue box of two hundred glyphs is two hundred state changes
/// per frame on a thread that has sixteen milliseconds for everything.
///
/// The second is `tarmiz_sirgb`. Colours arrive premultiplied and *linear*. When
/// `GL_FRAMEBUFFER_SRGB` is enabled the driver encodes on write and this shader
/// must emit linear, so the flag is zero. When it is disabled the values land in
/// the framebuffer verbatim and are displayed as sRGB, so the shader has to
/// encode them itself — and because they are premultiplied, encoding means
/// dividing the alpha out, applying the transfer function, and multiplying it
/// back. Skipping the un-premultiply is the mistake that makes anti-aliased
/// glyph edges look bitten out of the text rather than smooth.
const SHIFRAT_QITA: &str = "#version 330 core\n\
in vec2 khareeta_marra;\n\
in vec4 lawn_marra;\n\
in float nasij_marra;\n\
uniform sampler2D lawha;\n\
uniform int tarmiz_sirgb;\n\
out vec4 natija;\n\
vec3 ila_sirgb(vec3 khatti) {\n\
    vec3 amin = max(khatti, vec3(0.0));\n\
    vec3 munkhafid = amin * 12.92;\n\
    vec3 murtafi = 1.055 * pow(amin, vec3(1.0 / 2.4)) - 0.055;\n\
    return mix(murtafi, munkhafid, step(amin, vec3(0.0031308)));\n\
}\n\
void main() {\n\
    vec4 asas = mix(vec4(1.0), texture(lawha, khareeta_marra), nasij_marra);\n\
    vec4 mazuj = asas * lawn_marra;\n\
    if (tarmiz_sirgb != 0 && mazuj.a > 0.0) {\n\
        vec3 mufakkak = mazuj.rgb / mazuj.a;\n\
        mazuj = vec4(ila_sirgb(mufakkak) * mazuj.a, mazuj.a);\n\
    }\n\
    natija = mazuj;\n\
}\n";

// ---------------------------------------------------------------------------
// Conversions that keep the bits and lose nothing
// ---------------------------------------------------------------------------

/// A `GLint` read out of state, as the `GLenum` the setter wants.
///
/// `glGetIntegerv` is the only way to read a blend factor, a cull mode or a
/// front-face winding, and it reports them as `GLint` while every function that
/// *sets* them takes a `GLenum`. The round trip is bit-preserving by
/// construction here rather than by an `as`, so the workspace's cast lints stay
/// denied and the conversion cannot silently become a saturating one.
const fn ka_adad(qeema: Sahih) -> Adad {
    Adad::from_ne_bytes(qeema.to_ne_bytes())
}

/// A pixel count as a float, for the projection and the vertices.
///
/// `u32` to `f32` is exact below 2^24, which is 16.7 million — four orders of
/// magnitude past any surface dimension or glyph coordinate that will exist.
const fn qeema_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and glyph coordinates are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// A byte count as the `GLsizeiptr` the buffer calls take.
///
/// [`None`] when the count does not fit, which on a 64-bit target means a length
/// above `isize::MAX` — a value no allocation this crate makes can reach, and
/// one that would be a wrapped negative size handed to a driver if it did.
fn qeema_masafa(qeema: usize) -> Option<Masafa> {
    Masafa::try_from(qeema).ok()
}

/// A count as the `GLsizei` the draw and object calls take.
fn qeema_sahih(qeema: usize) -> Option<Sahih> {
    Sahih::try_from(qeema).ok()
}

// ---------------------------------------------------------------------------
// The state the draw disturbs
// ---------------------------------------------------------------------------

/// Everything [`KhattafGl::irsim`] changes, read before it changes any of it.
///
/// This struct is the contract with the game. Every field is a piece of context
/// state the overlay's draw writes, and there is no field here that the draw
/// does not write and no write the draw makes that is not a field here. When
/// those two sentences stop being true the overlay corrupts a renderer, so the
/// list is kept as a list rather than as scattered save/restore pairs — a
/// scattered pair can be half-deleted in a refactor and this cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each bool is one `glIsEnabled` capability, named after the GL enum it mirrors; \
              folding them into flags or an array is what breaks the one-field-per-state \
              correspondence the paragraph above makes this struct's whole correctness argument"
)]
struct HalatGl {
    /// `GL_CURRENT_PROGRAM`.
    barnamij: Sahih,
    /// `GL_VERTEX_ARRAY_BINDING`.
    tarteeb: Sahih,
    /// `GL_ARRAY_BUFFER_BINDING`, which is context state and not VAO state.
    mahfazat_raas: Sahih,
    /// `GL_ELEMENT_ARRAY_BUFFER_BINDING`, which *is* VAO state.
    mahfazat_fahras: Sahih,
    /// `GL_ACTIVE_TEXTURE`.
    wahdat_nasij: Sahih,
    /// `GL_TEXTURE_BINDING_2D` on unit zero, the only unit the overlay touches.
    nasij_wahda_sifr: Sahih,
    /// Whether `GL_BLEND` was enabled.
    mazj: bool,
    /// `GL_BLEND_SRC_RGB`, `GL_BLEND_DST_RGB`, `GL_BLEND_SRC_ALPHA` and
    /// `GL_BLEND_DST_ALPHA`, in that order.
    aamil_mazj: [Sahih; 4],
    /// `GL_BLEND_EQUATION_RGB` and `GL_BLEND_EQUATION_ALPHA`.
    muadalat_mazj: [Sahih; 2],
    /// Whether `GL_DEPTH_TEST` was enabled.
    ikhtibar_umq: bool,
    /// `GL_DEPTH_WRITEMASK`.
    qina_umq: Mantiqi,
    /// Whether `GL_CULL_FACE` was enabled.
    hadhf_wujuh: bool,
    /// `GL_CULL_FACE_MODE`.
    namat_hadhf: Sahih,
    /// `GL_FRONT_FACE`.
    wajh_amami: Sahih,
    /// Whether `GL_SCISSOR_TEST` was enabled.
    qass: bool,
    /// `GL_SCISSOR_BOX`.
    sunduq_qass: [Sahih; 4],
    /// `GL_VIEWPORT`.
    manfath: [Sahih; 4],
    /// `GL_DRAW_FRAMEBUFFER_BINDING`.
    itar_rasm: Sahih,
    /// `GL_READ_FRAMEBUFFER_BINDING`.
    itar_qira: Sahih,
    /// `GL_COLOR_WRITEMASK`.
    qina_lawn: [Mantiqi; 4],
    /// `GL_POLYGON_MODE`, front and back.
    namat_mudalla: [Sahih; 2],
    /// Whether `GL_FRAMEBUFFER_SRGB` was enabled.
    tarmiz_sirgb: bool,
}

impl HalatGl {
    /// Reads the whole of it.
    ///
    /// The active texture unit is read *before* the bound texture, and the unit
    /// is switched to zero in between, because `GL_TEXTURE_BINDING_2D` reports
    /// the binding of whichever unit is currently active. Reading it without
    /// switching first records some other unit's texture and restores it onto
    /// unit zero, which puts the game's shadow map where its diffuse map was.
    fn iltaqit(dawall: &DawallGl) -> Self {
        // SAFETY: every name below is queried through the arity its own entry in
        // the OpenGL 3.3 state tables gives it — one value for the bindings, the
        // masks and the modes; two for `GL_POLYGON_MODE` and the blend
        // equations; four for `GL_VIEWPORT`, `GL_SCISSOR_BOX` and
        // `GL_COLOR_WRITEMASK`. That correspondence is the invariant the
        // `sahih*`/`mantiqi*` helpers document, and this is the one place it is
        // established.
        unsafe {
            let barnamij = dawall.sahih(GL_CURRENT_PROGRAM);
            let tarteeb = dawall.sahih(GL_VERTEX_ARRAY_BINDING);
            let mahfazat_raas = dawall.sahih(GL_ARRAY_BUFFER_BINDING);
            let mahfazat_fahras = dawall.sahih(GL_ELEMENT_ARRAY_BUFFER_BINDING);

            let wahdat_nasij = dawall.sahih(GL_ACTIVE_TEXTURE);
            (dawall.active_texture)(GL_TEXTURE0);
            let nasij_wahda_sifr = dawall.sahih(GL_TEXTURE_BINDING_2D);

            Self {
                barnamij,
                tarteeb,
                mahfazat_raas,
                mahfazat_fahras,
                wahdat_nasij,
                nasij_wahda_sifr,
                mazj: dawall.mufaal(GL_BLEND),
                aamil_mazj: dawall.awamil_mazj(),
                muadalat_mazj: [
                    dawall.sahih(GL_BLEND_EQUATION_RGB),
                    dawall.sahih(GL_BLEND_EQUATION_ALPHA),
                ],
                ikhtibar_umq: dawall.mufaal(GL_DEPTH_TEST),
                qina_umq: dawall.mantiqi(GL_DEPTH_WRITEMASK),
                hadhf_wujuh: dawall.mufaal(GL_CULL_FACE),
                namat_hadhf: dawall.sahih(GL_CULL_FACE_MODE),
                wajh_amami: dawall.sahih(GL_FRONT_FACE),
                qass: dawall.mufaal(GL_SCISSOR_TEST),
                sunduq_qass: dawall.sahih_arbaa(GL_SCISSOR_BOX),
                manfath: dawall.sahih_arbaa(GL_VIEWPORT),
                itar_rasm: dawall.sahih(GL_DRAW_FRAMEBUFFER_BINDING),
                itar_qira: dawall.sahih(GL_READ_FRAMEBUFFER_BINDING),
                qina_lawn: dawall.mantiqi_arbaa(GL_COLOR_WRITEMASK),
                namat_mudalla: dawall.sahih_ithnan(GL_POLYGON_MODE),
                tarmiz_sirgb: dawall.mufaal(GL_FRAMEBUFFER_SRGB),
            }
        }
    }

    /// Writes the whole of it back.
    ///
    /// The order is the reverse of the order things were bound in, with three
    /// exceptions that are not stylistic:
    ///
    /// * The **vertex array object goes back before the element array buffer**,
    ///   because the element array binding is part of a VAO's state and writing
    ///   it while the overlay's VAO is still bound would change the overlay's
    ///   VAO and leave the game's untouched.
    /// * The element array buffer is **not restored when the game's VAO was
    ///   zero**. In a core profile, `glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, …)`
    ///   with no vertex array object bound is a `GL_INVALID_OPERATION` — so a
    ///   restore written without this check *creates* the error it is being
    ///   audited for, and disables the overlay on every core-profile game.
    /// * The **texture unit is switched back last**, after unit zero's texture
    ///   has been put back, for the same reason the save switched to it first.
    fn ustud(&self, dawall: &DawallGl) {
        let [masdar_rgb, hadaf_rgb, masdar_alfa, hadaf_alfa] = self.aamil_mazj;
        let [muadala_rgb, _muadala_alfa] = self.muadalat_mazj;
        let [qass_x, qass_y, qass_ard, qass_irtifa] = self.sunduq_qass;
        let [manfath_x, manfath_y, manfath_ard, manfath_irtifa] = self.manfath;
        let [ahmar, akhdar, azraq, shaffaf] = self.qina_lawn;
        let [mudalla_amami, _mudalla_khalfi] = self.namat_mudalla;

        // SAFETY: every call below writes context state and touches no memory
        // the overlay owns. Each argument is a value this same context reported
        // through `glGetIntegerv` or `glGetBooleanv` moments earlier, so each is
        // an accepted value for the state it is being written back into — a
        // driver cannot report a blend factor it would reject.
        unsafe {
            (dawall.use_program)(ka_adad(self.barnamij));
            (dawall.bind_vertex_array)(ka_adad(self.tarteeb));
            if self.tarteeb != 0 {
                (dawall.bind_buffer)(GL_ELEMENT_ARRAY_BUFFER, ka_adad(self.mahfazat_fahras));
            }
            (dawall.bind_buffer)(GL_ARRAY_BUFFER, ka_adad(self.mahfazat_raas));

            (dawall.bind_texture)(GL_TEXTURE_2D, ka_adad(self.nasij_wahda_sifr));
            (dawall.active_texture)(ka_adad(self.wahdat_nasij));

            (dawall.blend_func_separate)(
                ka_adad(masdar_rgb),
                ka_adad(hadaf_rgb),
                ka_adad(masdar_alfa),
                ka_adad(hadaf_alfa),
            );
            // `glBlendEquation` sets both the RGB and the alpha equation, and a
            // core-profile context has no unindexed way to set them apart. They
            // differ only where a game called `glBlendEquationSeparate`, and the
            // overlay never changes the alpha equation on its own, so writing
            // the RGB value into both is the closest restore this table can make
            // and is recorded here as such rather than hidden.
            (dawall.blend_equation)(ka_adad(muadala_rgb));
            dawall.ashil(GL_BLEND, self.mazj);

            (dawall.depth_mask)(self.qina_umq);
            dawall.ashil(GL_DEPTH_TEST, self.ikhtibar_umq);

            (dawall.cull_face)(ka_adad(self.namat_hadhf));
            (dawall.front_face)(ka_adad(self.wajh_amami));
            dawall.ashil(GL_CULL_FACE, self.hadhf_wujuh);

            (dawall.scissor)(qass_x, qass_y, qass_ard, qass_irtifa);
            dawall.ashil(GL_SCISSOR_TEST, self.qass);

            (dawall.viewport)(manfath_x, manfath_y, manfath_ard, manfath_irtifa);
            (dawall.color_mask)(ahmar, akhdar, azraq, shaffaf);
            // A core profile rejects a separate front and back polygon mode, so
            // the front value is written to both faces. That is not a loss: a
            // context that could report two different values is a compatibility
            // context, and on one of those `GL_FRONT_AND_BACK` with the front
            // mode is what the game itself had to have set.
            (dawall.polygon_mode)(GL_FRONT_AND_BACK, ka_adad(mudalla_amami));

            (dawall.bind_framebuffer)(GL_DRAW_FRAMEBUFFER, ka_adad(self.itar_rasm));
            (dawall.bind_framebuffer)(GL_READ_FRAMEBUFFER, ka_adad(self.itar_qira));
            dawall.ashil(GL_FRAMEBUFFER_SRGB, self.tarmiz_sirgb);
        }
    }
}

impl DawallGl {
    /// The four blend factors, read the way the game most likely set them.
    ///
    /// On a context at OpenGL 4.0 or later, blending is per draw buffer and a
    /// game may have set draw buffer zero's factors through
    /// `glBlendFuncSeparatei`. The unindexed `glGetIntegerv(GL_BLEND_SRC_RGB)`
    /// is *specified* to report draw buffer zero, and there are shipped drivers
    /// on which it reports the last value passed to the unindexed
    /// `glBlendFunc` instead. Restoring a blend factor the game never set is not
    /// a visible failure in the overlay — it is a visible failure in the game's
    /// next transparent surface, three draw calls later, with nothing pointing
    /// back here. Reading it back through the same indexed path it was set
    /// through is what survives that.
    fn awamil_mazj(&self) -> [Sahih; 4] {
        if self.isdar.0 >= 4 {
            // SAFETY: on a 4.0 context each of these four names is an indexed
            // state with exactly one value per draw buffer, and index zero
            // exists on every context — `GL_MAX_DRAW_BUFFERS` is at least eight.
            unsafe {
                [
                    self.sahih_mufahras(GL_BLEND_SRC_RGB, 0),
                    self.sahih_mufahras(GL_BLEND_DST_RGB, 0),
                    self.sahih_mufahras(GL_BLEND_SRC_ALPHA, 0),
                    self.sahih_mufahras(GL_BLEND_DST_ALPHA, 0),
                ]
            }
        } else {
            // SAFETY: on a 3.3 context each of these four names is a plain,
            // single-valued state in the specification's blending table.
            unsafe {
                [
                    self.sahih(GL_BLEND_SRC_RGB),
                    self.sahih(GL_BLEND_DST_RGB),
                    self.sahih(GL_BLEND_SRC_ALPHA),
                    self.sahih(GL_BLEND_DST_ALPHA),
                ]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The geometry
// ---------------------------------------------------------------------------

/// One vertex: thirty-six bytes, laid out the way the shader reads them.
///
/// `#[repr(C)]` is not decoration. The whole array is handed to `glBufferSubData`
/// as bytes and read back by `glVertexAttribPointer` at fixed offsets, so a Rust
/// layout the compiler is free to reorder would put the colour where the
/// position is expected — which draws a screenful of triangles in the wrong
/// place and is the kind of bug that looks like a driver problem.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
#[allow(
    dead_code,
    reason = "no Rust code reads these fields and none ever will: they are written by the \
              batch builder and read by the GPU, through the byte offsets \
              `glVertexAttribPointer` was given. That is the whole purpose of the `repr(C)` \
              layout, and a field the compiler calls unread is one the vertex shader is \
              consuming"
)]
struct RaasQita {
    /// Position in surface pixels, origin top-left.
    mawdi: [f32; 2],
    /// Atlas coordinate, normalized. Ignored when `nasij` is zero.
    khareeta: [f32; 2],
    /// Premultiplied linear RGBA.
    lawn: [f32; 4],
    /// One when this quad samples the atlas, zero when it is a solid plate.
    nasij: f32,
}

/// Where each attribute starts inside [`RaasQita`], in bytes.
///
/// Written as `offset_of!` rather than as literals so that adding a field cannot
/// leave the pointer setup describing the previous layout.
const IZAHAT: [usize; 4] = [
    core::mem::offset_of!(RaasQita, mawdi),
    core::mem::offset_of!(RaasQita, khareeta),
    core::mem::offset_of!(RaasQita, lawn),
    core::mem::offset_of!(RaasQita, nasij),
];

/// An orthographic projection from surface pixels to clip space, column-major.
///
/// Origin top-left, y increasing downward — the convention every rectangle in
/// this crate uses — which is why the y row is negated. OpenGL's own clip space
/// has y increasing upward, and a matrix built without the negation renders the
/// overlay mirrored vertically about the middle of the screen: text at the top
/// of the dialogue box appears at the bottom of it, correctly shaped.
fn isqat_mustawi(ard: u32, irtifa: u32) -> [f32; 16] {
    let ard = qeema_f32(ard).max(1.0);
    let irtifa = qeema_f32(irtifa).max(1.0);
    [
        2.0 / ard,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / irtifa,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        -1.0,
        1.0,
        0.0,
        1.0,
    ]
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// What a GL error surviving the restore means for the overlay.
///
/// The distinction is [`crate::wajiha::Khattaf`]'s, not this module's: a failure
/// that leaves state consistent is retried next frame, and one that does not
/// disables the overlay for the session. Making it a parameter of the one
/// save-draw-restore helper is what keeps a future method from picking the
/// wrong one by writing its own restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaddFasad {
    /// Report [`KhataTabaqa::MawridFashil`] — the frame is skipped and retried.
    Mawrid,
    /// Report [`KhataTabaqa::HalaGhayrMustaada`] — the overlay stops for good.
    Hala,
    /// Report [`KhataTabaqa::IltiqatFashil`] — the read-back did not complete.
    Iltiqat,
}

/// The OpenGL overlay renderer.
///
/// ## Why this is `Send` and what that does not mean
///
/// [`Khattaf`] requires `Send`, and this type satisfies it because everything in
/// it is a function pointer, a plain integer or a `Vec`. That is a statement
/// about memory, not about OpenGL: every name in here belongs to one context,
/// and a context belongs to one thread at a time. Moving this value to another
/// thread and calling into it there deletes objects in whatever context that
/// thread has current, which may be a different game's. The hook in
/// [`crate::khataf`] calls this only from the thread that owns the context, and
/// nothing in Rust's type system says so — which is why it is said here.
///
/// ## Why there is no `Drop`
///
/// Deleting a texture requires a current context. A `Drop` impl runs wherever
/// the value happens to be dropped, which after a terminal fault is whatever
/// thread unwound — very possibly one with no context current, or with the
/// wrong one. Deleting name `7` in the wrong context deletes somebody else's
/// object. So release is [`Khattaf::ahmil`], called deliberately, on the render
/// thread, and dropping this value without calling it leaks GPU objects into a
/// process that is about to end anyway. That is the better of the two failures.
pub struct KhattafGl {
    /// The resolved entry points.
    dawall: DawallGl,
    /// The linked program, or zero.
    barnamij: Adad,
    /// The `isqat` uniform's location, or `-1`.
    mawdi_isqat: Sahih,
    /// The `lawha` sampler uniform's location, or `-1`.
    mawdi_lawha: Sahih,
    /// The `tarmiz_sirgb` uniform's location, or `-1`.
    mawdi_tarmiz: Sahih,
    /// The vertex array object, or zero.
    tarteeb: Adad,
    /// The vertex buffer, or zero.
    mahfazat_raas: Adad,
    /// The index buffer, or zero.
    mahfazat_fahras: Adad,
    /// How many bytes the vertex buffer currently holds.
    saat_raas: usize,
    /// How many bytes the index buffer currently holds.
    saat_fahras: usize,
    /// The atlas texture, or zero.
    nasij: Adad,
    /// The atlas's dimensions, once uploaded.
    qiyas_lawha: (u32, u32),
    /// `GL_MAX_TEXTURE_SIZE`, read once.
    aqsa_nasij: u32,
    /// The drawable size the platform layer reported, when it knows one.
    qiyas_mubarmaj: Option<(u32, u32)>,
    /// The context this backend's object names belong to.
    siyaq: u64,
    /// The surface the last [`Khattaf::hayyi`] was given.
    sath: Option<WasfSath>,
    /// The vertex staging buffer, reused across frames.
    ruus: Vec<RaasQita>,
    /// The index staging buffer, reused across frames.
    faharis: Vec<u32>,
    /// What has happened here, for the diagnostics bundle.
    athar: Vec<String>,
}

impl fmt::Debug for KhattafGl {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("KhattafGl")
            .field("dawall", &self.dawall)
            .field("barnamij", &self.barnamij)
            .field("tarteeb", &self.tarteeb)
            .field("mahfazat_raas", &self.mahfazat_raas)
            .field("mahfazat_fahras", &self.mahfazat_fahras)
            .field("nasij", &self.nasij)
            .field("qiyas_lawha", &self.qiyas_lawha)
            .field("sath", &self.sath)
            .finish_non_exhaustive()
    }
}

impl KhattafGl {
    /// Resolves the entry points and prepares an unbuilt backend.
    ///
    /// Nothing is created here. [`Khattaf::hayyi`] creates the program and the
    /// buffers, and it is called by [`crate::wajiha::Tabaqa::shaghghil`] on the
    /// render thread — which is the only thread where creating a GL object means
    /// anything. A constructor that created objects would be a constructor that
    /// had to be called from exactly one place and could not say so.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MaktabaMafquda`] when the OpenGL module cannot be found or
    /// does not export one of the entry points the overlay calls.
    pub fn jadeed() -> Result<Self, KhataTabaqa> {
        let dawall = DawallGl::hammil()?;
        Ok(Self::bi_dawall(dawall))
    }

    /// Prepares a backend over an already-resolved table.
    ///
    /// Exists so the renderer can be exercised against a context created for the
    /// purpose, with a table built by the test rather than by
    /// [`DawallGl::hammil`]. That is the same separation
    /// [`crate::wajiha::Khattaf`] draws by not owning the hook: a backend is
    /// testable exactly to the degree that it does not go looking for its own
    /// inputs.
    #[must_use]
    pub fn bi_dawall(dawall: DawallGl) -> Self {
        let athar = vec![format!("OpenGL {}.{}: {}", dawall.isdar.0, dawall.isdar.1, dawall.wasf)];
        Self {
            dawall,
            barnamij: 0,
            mawdi_isqat: -1,
            mawdi_lawha: -1,
            mawdi_tarmiz: -1,
            tarteeb: 0,
            mahfazat_raas: 0,
            mahfazat_fahras: 0,
            saat_raas: 0,
            saat_fahras: 0,
            nasij: 0,
            qiyas_lawha: (0, 0),
            aqsa_nasij: 0,
            qiyas_mubarmaj: None,
            siyaq: 0,
            sath: None,
            ruus: Vec::new(),
            faharis: Vec::new(),
            athar,
        }
    }

    /// The entry points this backend resolved.
    #[must_use]
    pub const fn dawall(&self) -> &DawallGl {
        &self.dawall
    }

    /// What has happened to this backend, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Records the drawable size the platform layer knows and OpenGL does not.
    ///
    /// Core OpenGL has no query for the size of the default framebuffer. The
    /// viewport is the usual stand-in and it is wrong precisely when it matters:
    /// a game that renders its world into a half-height viewport for a
    /// letterboxed cutscene has a viewport that is not the surface, and an
    /// overlay sized to it puts the subtitles in the middle of the screen. The
    /// hook has the `HDC`, the `GLXDrawable` or the `EGLSurface` and can ask the
    /// window system directly; when it does, that answer wins.
    pub const fn hadith_qiyas(&mut self, ard: u32, irtifa: u32) {
        self.qiyas_mubarmaj = if ard == 0 || irtifa == 0 { None } else { Some((ard, irtifa)) };
    }

    /// Records which context the object names belong to.
    ///
    /// A game that destroys its context and makes a new one — which happens on a
    /// device reset, on a driver update applied live, and on every renderer that
    /// recreates its context for a fullscreen transition — invalidates every
    /// name this backend holds. The names are then **forgotten rather than
    /// deleted**: `glDeleteTextures` in the new context would delete whatever
    /// object happens to hold that name there, which is the game's, and the
    /// symptom is one of the game's own textures turning black for no reason
    /// anybody can trace.
    ///
    /// `muarrif` is whatever the platform uses to identify a context — an
    /// `HGLRC`, a `GLXContext`, an `EGLContext` — as an integer. Zero means
    /// "unknown", and is never treated as a change.
    pub fn sajjil_siyaq(&mut self, muarrif: u64) {
        if muarrif == 0 || muarrif == self.siyaq {
            return;
        }
        if self.siyaq != 0 {
            self.athar.push(format!(
                "the GL context changed from {:#x} to {muarrif:#x}; every object name this \
                 backend held belongs to a context that no longer exists and was forgotten \
                 rather than deleted",
                self.siyaq
            ));
        }
        self.siyaq = muarrif;
        self.barnamij = 0;
        self.mawdi_isqat = -1;
        self.mawdi_lawha = -1;
        self.mawdi_tarmiz = -1;
        self.tarteeb = 0;
        self.mahfazat_raas = 0;
        self.mahfazat_fahras = 0;
        self.saat_raas = 0;
        self.saat_fahras = 0;
        self.nasij = 0;
        self.qiyas_lawha = (0, 0);
        self.sath = None;
    }

    /// Runs one piece of work with the game's state saved around it.
    ///
    /// This is the whole discipline of the module in one function, and every
    /// method that touches the context goes through it. The sequence is fixed:
    /// drain the error queue so what is found afterwards is the overlay's own,
    /// read the state, do the work, write the state back **whether the work
    /// succeeded or not**, and only then ask whether the restore itself
    /// produced an error.
    ///
    /// Draining first is itself a state change and is done knowingly. OpenGL's
    /// error flags are cleared by reading them, so a game that called
    /// `glGetError` after its own present would see nothing where the overlay
    /// consumed a flag. The alternative is worse in both directions: the overlay
    /// either disables itself over an error the game left behind, or it cannot
    /// tell its own errors from anyone's and the check is theatre.
    fn bi_hifz_hala<T>(
        &mut self,
        radd: RaddFasad,
        amal: impl FnOnce(&mut Self) -> Result<T, KhataTabaqa>,
    ) -> Result<T, KhataTabaqa> {
        let sabiq = self.dawall.ifragh();
        if sabiq != GL_NO_ERROR {
            self.athar.push(format!(
                "{} was already set when the overlay was entered; it belongs to the game and \
                 was cleared so the overlay's own check means something",
                ism_khata(sabiq)
            ));
        }

        let hala = HalatGl::iltaqit(&self.dawall);
        let natija = amal(self);
        hala.ustud(&self.dawall);

        let baqi = self.dawall.ifragh();
        if baqi == GL_NO_ERROR {
            return natija;
        }
        let sabab = format!(
            "{} was set across the overlay's draw and its state restore, on {}",
            ism_khata(baqi),
            self.dawall.wasf
        );
        match radd {
            RaddFasad::Hala => Err(KhataTabaqa::HalaGhayrMustaada {
                hala: "the OpenGL context's binding, blend, depth, cull, scissor, viewport, \
                       framebuffer, colour-mask and polygon state",
                sabab,
            }),
            RaddFasad::Mawrid => {
                Err(KhataTabaqa::MawridFashil { mawrid: "an OpenGL object", sabab })
            }
            RaddFasad::Iltiqat => Err(KhataTabaqa::IltiqatFashil { sabab }),
        }
    }
}

// ---------------------------------------------------------------------------
// Building the program
// ---------------------------------------------------------------------------

/// The largest info log this build will read back, in bytes.
///
/// Sixty-four kibibytes. Drivers have been observed returning info logs of
/// several megabytes for a shader with one syntax error, by repeating the same
/// line once per instruction; reading all of it on a render thread to put it in
/// an error message nobody will finish reading is a stall the player feels.
const AQSA_SIJILL: usize = 64 * 1024;

impl KhattafGl {
    /// One shader's info log, as the driver wrote it.
    fn sijill_shifra(&self, shifra: Adad) -> String {
        let mut tul: Sahih = 0;
        // SAFETY: `glGetShaderiv` with `GL_INFO_LOG_LENGTH` writes exactly one
        // `GLint`, and `tul` is one live, aligned `GLint`.
        unsafe { (self.dawall.get_shaderiv)(shifra, GL_INFO_LOG_LENGTH, &raw mut tul) };
        let Some(saa) = usize::try_from(tul).ok().filter(|saa| *saa > 1) else {
            return String::new();
        };
        let saa = saa.min(AQSA_SIJILL);
        let Some(saa_gl) = qeema_sahih(saa) else {
            return String::new();
        };
        let mut khazan = vec![0u8; saa];
        let mut katab: Sahih = 0;
        // SAFETY: `khazan` is `saa` writable bytes and `saa_gl` is that same
        // count, so the driver cannot write past the end; `katab` is one live
        // `GLint` the driver fills with how much it wrote.
        unsafe {
            (self.dawall.get_shader_info_log)(
                shifra,
                saa_gl,
                &raw mut katab,
                khazan.as_mut_ptr().cast::<c_char>(),
            );
        }
        khazan.truncate(usize::try_from(katab).unwrap_or(0).min(saa));
        String::from_utf8_lossy(&khazan).trim().to_owned()
    }

    /// One program's info log, as the driver wrote it.
    fn sijill_barnamij(&self, barnamij: Adad) -> String {
        let mut tul: Sahih = 0;
        // SAFETY: `glGetProgramiv` with `GL_INFO_LOG_LENGTH` writes exactly one
        // `GLint`, and `tul` is one live, aligned `GLint`.
        unsafe { (self.dawall.get_programiv)(barnamij, GL_INFO_LOG_LENGTH, &raw mut tul) };
        let Some(saa) = usize::try_from(tul).ok().filter(|saa| *saa > 1) else {
            return String::new();
        };
        let saa = saa.min(AQSA_SIJILL);
        let Some(saa_gl) = qeema_sahih(saa) else {
            return String::new();
        };
        let mut khazan = vec![0u8; saa];
        let mut katab: Sahih = 0;
        // SAFETY: as in `sijill_shifra` — the buffer and the count the driver is
        // given describe the same allocation.
        unsafe {
            (self.dawall.get_program_info_log)(
                barnamij,
                saa_gl,
                &raw mut katab,
                khazan.as_mut_ptr().cast::<c_char>(),
            );
        }
        khazan.truncate(usize::try_from(katab).unwrap_or(0).min(saa));
        String::from_utf8_lossy(&khazan).trim().to_owned()
    }

    /// Compiles one stage, carrying the driver's complaint out on failure.
    ///
    /// The info log is not decoration. A shader that fails to compile inside a
    /// stranger's game fails on their driver, on their GPU, on a GLSL compiler
    /// this machine does not have — and the only artefact that ever reaches a
    /// maintainer is the error string in a bug report. "The vertex shader would
    /// not compile" is unactionable; `0:14(23): error: no matching function for
    /// call to 'texture'` names the line.
    fn ibni_shifra(
        &self,
        naw: Adad,
        masdar: &str,
        ism: &'static str,
    ) -> Result<Adad, KhataTabaqa> {
        // SAFETY: `glCreateShader` takes a stage name and returns a name or
        // zero; it reads and writes no memory of ours.
        let shifra = unsafe { (self.dawall.create_shader)(naw) };
        if shifra == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: ism,
                sabab: "glCreateShader returned zero, which means the context has no GLSL \
                        compiler — a core profile built without one, or a context that is no \
                        longer current"
                    .to_owned(),
            });
        }

        let Some(tul) = qeema_sahih(masdar.len()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: ism,
                sabab: "the shader source is longer than a GLsizei can express".to_owned(),
            });
        };
        let muashir = masdar.as_ptr().cast::<c_char>();
        // SAFETY: one source string is passed, so the two arrays the driver
        // reads are the single-element arrays formed by `&raw const` over the
        // two locals. `muashir` points at `masdar`'s bytes, which outlive the
        // call, and `tul` is exactly that many bytes — so the driver never reads
        // for a terminator that a `&str` does not have.
        unsafe {
            (self.dawall.shader_source)(shifra, 1, &raw const muashir, &raw const tul);
            (self.dawall.compile_shader)(shifra);
        }

        let mut hala: Sahih = 0;
        // SAFETY: `GL_COMPILE_STATUS` is a single-valued shader parameter and
        // `hala` is one live, aligned `GLint`.
        unsafe { (self.dawall.get_shaderiv)(shifra, GL_COMPILE_STATUS, &raw mut hala) };
        if hala != Sahih::from(GL_TRUE) {
            let sijill = self.sijill_shifra(shifra);
            // SAFETY: `shifra` is a name this function created and has not
            // deleted; deleting it releases the failed compile's storage.
            unsafe { (self.dawall.delete_shader)(shifra) };
            let sijill = if sijill.is_empty() {
                "the driver returned an empty info log, which is a driver bug and leaves \
                 nothing to report but the failure itself"
                    .to_owned()
            } else {
                sijill
            };
            return Err(KhataTabaqa::MawridFashil { mawrid: ism, sabab: sijill });
        }
        Ok(shifra)
    }

    /// Compiles both stages, links them, and finds the uniforms.
    ///
    /// Both shaders are deleted immediately after the link. That is not early
    /// cleanup — a shader object attached to a linked program is kept alive by
    /// the program, and `glDeleteShader` on an attached shader only marks it, so
    /// this is the documented way to say "the program owns these now" and is
    /// what keeps `ahmil` from having to track two more names.
    fn ibni_barnamij(&mut self) -> Result<(), KhataTabaqa> {
        let raas = self.ibni_shifra(GL_VERTEX_SHADER, SHIFRAT_RAAS, "vertex shader")?;
        let qita = match self.ibni_shifra(GL_FRAGMENT_SHADER, SHIFRAT_QITA, "fragment shader") {
            Ok(qita) => qita,
            Err(khata) => {
                // SAFETY: `raas` was created by `ibni_shifra` above, has not been
                // attached to anything, and has not been deleted.
                unsafe { (self.dawall.delete_shader)(raas) };
                return Err(khata);
            }
        };

        // SAFETY: `glCreateProgram` takes nothing and returns a name or zero.
        let barnamij = unsafe { (self.dawall.create_program)() };
        if barnamij == 0 {
            // SAFETY: both names were created above and neither is attached.
            unsafe {
                (self.dawall.delete_shader)(raas);
                (self.dawall.delete_shader)(qita);
            }
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "shader program",
                sabab: "glCreateProgram returned zero".to_owned(),
            });
        }

        // SAFETY: all three names were created above and are live; attaching and
        // linking read and write only driver-side objects.
        unsafe {
            (self.dawall.attach_shader)(barnamij, raas);
            (self.dawall.attach_shader)(barnamij, qita);
            (self.dawall.link_program)(barnamij);
            (self.dawall.delete_shader)(raas);
            (self.dawall.delete_shader)(qita);
        }

        let mut hala: Sahih = 0;
        // SAFETY: `GL_LINK_STATUS` is a single-valued program parameter and
        // `hala` is one live, aligned `GLint`.
        unsafe { (self.dawall.get_programiv)(barnamij, GL_LINK_STATUS, &raw mut hala) };
        if hala != Sahih::from(GL_TRUE) {
            let sijill = self.sijill_barnamij(barnamij);
            // SAFETY: `barnamij` was created above and has not been deleted.
            unsafe { (self.dawall.delete_program)(barnamij) };
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "shader program",
                sabab: if sijill.is_empty() {
                    "the link failed and the driver returned an empty info log".to_owned()
                } else {
                    sijill
                },
            });
        }

        self.barnamij = barnamij;
        self.mawdi_isqat = self.mawdi_zayy("isqat");
        self.mawdi_lawha = self.mawdi_zayy("lawha");
        self.mawdi_tarmiz = self.mawdi_zayy("tarmiz_sirgb");
        Ok(())
    }

    /// One uniform's location, or `-1` when the linker optimised it away.
    ///
    /// `-1` is not an error and is not treated as one: `glUniform*` with a
    /// location of `-1` is defined to do nothing, so a uniform a driver removed
    /// because the shader stopped reading it costs a no-op call and not a
    /// failure. Refusing to run over it would make the overlay depend on a
    /// GLSL compiler's dead-code decisions.
    fn mawdi_zayy(&self, ism: &str) -> Sahih {
        let ism_khaam = ism_c(ism);
        // SAFETY: `self.barnamij` is a linked program created above, and
        // `ism_khaam` is a live NUL-terminated name for the duration of the
        // call, which is `glGetUniformLocation`'s whole contract.
        unsafe {
            (self.dawall.get_uniform_location)(self.barnamij, ism_khaam.as_ptr().cast::<c_char>())
        }
    }

    /// Creates the vertex array object and the two buffers, and wires them.
    ///
    /// The attribute pointers are set once, here, and never again: a vertex
    /// array object exists precisely so that the six calls it takes to describe
    /// a thirty-six-byte vertex happen at startup rather than sixty times a
    /// second on a thread with a sixteen-millisecond budget.
    fn ibni_tarteeb(&mut self) -> Result<(), KhataTabaqa> {
        let mut tarteeb: Adad = 0;
        let mut mahafiz: [Adad; 2] = [0; 2];
        // SAFETY: `tarteeb` is one live `GLuint` and `mahafiz` is two, and the
        // counts passed match those capacities exactly.
        unsafe {
            (self.dawall.gen_vertex_arrays)(1, &raw mut tarteeb);
            (self.dawall.gen_buffers)(2, mahafiz.as_mut_ptr());
        }
        let [mahfazat_raas, mahfazat_fahras] = mahafiz;
        if tarteeb == 0 || mahfazat_raas == 0 || mahfazat_fahras == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "vertex array object and stream buffers",
                sabab: format!(
                    "the driver returned name {tarteeb} for the vertex array and names \
                     {mahfazat_raas}/{mahfazat_fahras} for the buffers; a zero there means \
                     the context is not current on this thread"
                ),
            });
        }

        let Some(khatwa) = qeema_sahih(size_of::<RaasQita>()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "vertex layout",
                sabab: "the vertex stride does not fit a GLsizei".to_owned(),
            });
        };
        let [izahat_mawdi, izahat_khareeta, izahat_lawn, izahat_nasij] = IZAHAT;

        // SAFETY: the vertex array and the array buffer are bound immediately
        // before the attribute pointers are described, which is what makes each
        // `glVertexAttribPointer` record an offset into *this* buffer rather
        // than a client-side address. The pointers passed are byte offsets built
        // from `offset_of!` on the very struct whose stride is passed alongside
        // them, so no attribute can name a byte outside a vertex.
        unsafe {
            (self.dawall.bind_vertex_array)(tarteeb);
            (self.dawall.bind_buffer)(GL_ARRAY_BUFFER, mahfazat_raas);
            (self.dawall.bind_buffer)(GL_ELEMENT_ARRAY_BUFFER, mahfazat_fahras);

            (self.dawall.enable_vertex_attrib_array)(MAWQI_MAWDI);
            (self.dawall.vertex_attrib_pointer)(
                MAWQI_MAWDI,
                2,
                GL_FLOAT,
                GL_FALSE,
                khatwa,
                core::ptr::without_provenance(izahat_mawdi),
            );
            (self.dawall.enable_vertex_attrib_array)(MAWQI_KHAREETA);
            (self.dawall.vertex_attrib_pointer)(
                MAWQI_KHAREETA,
                2,
                GL_FLOAT,
                GL_FALSE,
                khatwa,
                core::ptr::without_provenance(izahat_khareeta),
            );
            (self.dawall.enable_vertex_attrib_array)(MAWQI_LAWN);
            (self.dawall.vertex_attrib_pointer)(
                MAWQI_LAWN,
                4,
                GL_FLOAT,
                GL_FALSE,
                khatwa,
                core::ptr::without_provenance(izahat_lawn),
            );
            (self.dawall.enable_vertex_attrib_array)(MAWQI_NASIJ);
            (self.dawall.vertex_attrib_pointer)(
                MAWQI_NASIJ,
                1,
                GL_FLOAT,
                GL_FALSE,
                khatwa,
                core::ptr::without_provenance(izahat_nasij),
            );
        }

        self.tarteeb = tarteeb;
        self.mahfazat_raas = mahfazat_raas;
        self.mahfazat_fahras = mahfazat_fahras;
        self.saat_raas = 0;
        self.saat_fahras = 0;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Streaming the batch
// ---------------------------------------------------------------------------

/// Uploads one staging buffer, growing the GPU allocation only when it must.
///
/// The two-step — `glBufferData` with a null pointer, then `glBufferSubData` —
/// is buffer orphaning, and it is the reason this backend does not stall. A
/// `glBufferSubData` into a buffer the GPU is still reading from the previous
/// frame makes the driver either block until that read finishes or make a
/// shadow copy; `glBufferData(…, NULL, …)` tells it the old contents are dead,
/// so it hands back fresh storage and the write never waits. Reallocating with
/// the data in one call would work and would give up that guarantee on every
/// frame where the batch size happened not to change.
///
/// Returns the buffer's capacity in bytes afterwards.
fn arfa_mahfaza(
    dawall: &DawallGl,
    hadaf: Adad,
    bayt: &[u8],
    saa: usize,
    ism: &'static str,
) -> Result<usize, KhataTabaqa> {
    let matlub = bayt.len();
    let Some(matlub_gl) = qeema_masafa(matlub) else {
        return Err(KhataTabaqa::MawridFashil {
            mawrid: ism,
            sabab: format!("{matlub} bytes does not fit a GLsizeiptr"),
        });
    };

    if matlub > saa {
        // A growing batch reallocates to what it needs plus half, so a batch
        // that creeps up by one glyph a frame does not reallocate a thousand
        // times on the way.
        #[expect(
            clippy::integer_division,
            reason = "a growth factor of one and a half, where truncating the halving is the \
                      intended rounding and cannot underflow a usize"
        )]
        let jadeed = matlub.saturating_add(matlub / 2);
        let Some(jadeed_gl) = qeema_masafa(jadeed) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: ism,
                sabab: format!("{jadeed} bytes does not fit a GLsizeiptr"),
            });
        };
        // SAFETY: a null data pointer with a non-zero size is the documented way
        // to allocate a buffer's storage without initialising it, and `hadaf` is
        // a binding target this backend bound its own buffer to immediately
        // before the call.
        unsafe { (dawall.buffer_data)(hadaf, jadeed_gl, core::ptr::null(), GL_STREAM_DRAW) };
        // SAFETY: the buffer was just allocated with `jadeed_gl >= matlub_gl`
        // bytes, and `bayt` is a live slice of exactly `matlub` bytes, so the
        // driver reads inside both allocations.
        unsafe {
            (dawall.buffer_sub_data)(hadaf, 0, matlub_gl, bayt.as_ptr().cast::<c_void>());
        }
        return Ok(jadeed);
    }

    let Some(saa_gl) = qeema_masafa(saa) else {
        return Err(KhataTabaqa::MawridFashil {
            mawrid: ism,
            sabab: format!("{saa} bytes does not fit a GLsizeiptr"),
        });
    };
    // SAFETY: orphaning the existing storage at its existing size, then writing
    // `matlub` bytes into it from a live slice of that length. `matlub <= saa`
    // on this branch, so the write is inside the allocation.
    unsafe {
        (dawall.buffer_data)(hadaf, saa_gl, core::ptr::null(), GL_STREAM_DRAW);
        (dawall.buffer_sub_data)(hadaf, 0, matlub_gl, bayt.as_ptr().cast::<c_void>());
    }
    Ok(saa)
}

impl KhattafGl {
    /// Turns the batch into vertices and indices, touching no GL state.
    ///
    /// Deliberately separate from the draw. Everything in here can go wrong — an
    /// empty rectangle, a colour out of range, a quad past the ceiling — and all
    /// of it goes wrong *before* the game's state has been saved, which means a
    /// bad batch costs a skipped frame instead of a save-and-restore cycle
    /// around a draw that was never going to happen.
    fn ibni_dufaa(&mut self, lawha: &LawhatRasm) {
        self.ruus.clear();
        self.faharis.clear();
        self.ruus.reserve(lawha.qitaat.len().saturating_mul(4));
        self.faharis.reserve(lawha.qitaat.len().saturating_mul(6));

        for qita in &lawha.qitaat {
            let QitaRasm { mawdi, khareeta, lawn } = *qita;
            if mawdi.ard == 0 || mawdi.irtifa == 0 {
                continue;
            }
            let yasar = qeema_f32(mawdi.yasar);
            let aala = qeema_f32(mawdi.aala);
            let yameen = yasar + qeema_f32(mawdi.ard);
            let asfal = aala + qeema_f32(mawdi.irtifa);

            let (u0, v0, u1, v1, nasij) = match khareeta {
                Some(khareeta) => (
                    khareeta.yasar,
                    khareeta.aala,
                    khareeta.yasar + khareeta.ard,
                    khareeta.aala + khareeta.irtifa,
                    1.0,
                ),
                // A backing plate samples nothing. Its atlas coordinates are
                // still written, as zeros, because a vertex with an
                // uninitialised attribute is a vertex whose value depends on
                // whatever the previous batch left in that buffer slot.
                None => (0.0, 0.0, 0.0, 0.0, 0.0),
            };

            let Some(asas) = qeema_fahras(self.ruus.len()) else {
                break;
            };
            self.ruus.push(RaasQita {
                mawdi: [yasar, aala],
                khareeta: [u0, v0],
                lawn,
                nasij,
            });
            self.ruus.push(RaasQita {
                mawdi: [yameen, aala],
                khareeta: [u1, v0],
                lawn,
                nasij,
            });
            self.ruus.push(RaasQita {
                mawdi: [yameen, asfal],
                khareeta: [u1, v1],
                lawn,
                nasij,
            });
            self.ruus.push(RaasQita {
                mawdi: [yasar, asfal],
                khareeta: [u0, v1],
                lawn,
                nasij,
            });
            self.faharis.extend_from_slice(&[
                asas,
                asas.saturating_add(1),
                asas.saturating_add(2),
                asas,
                asas.saturating_add(2),
                asas.saturating_add(3),
            ]);
        }
    }

    /// Sets the pipeline the overlay needs and issues the one draw call.
    ///
    /// Everything this function changes is a field of [`HalatGl`], which is what
    /// makes the restore complete. Adding a state change here without adding the
    /// matching field there is the single way this module can corrupt a
    /// renderer, and the two are kept adjacent in the file so that a reader
    /// comparing them does not have to scroll.
    fn arsil_dufaa(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        let bayt_ruus = size_of_val(self.ruus.as_slice());
        let bayt_faharis = size_of_val(self.faharis.as_slice());
        // SAFETY: `RaasQita` is `#[repr(C)]` over `f32` fields only, so it has no
        // padding and no uninitialised byte, and `u32` likewise. Viewing exactly
        // `size_of_val` bytes of a live slice as `u8` stays inside the same
        // allocation, and `u8`'s alignment of one is satisfied by any address.
        let (khaam_ruus, khaam_faharis) = unsafe {
            (
                core::slice::from_raw_parts(self.ruus.as_ptr().cast::<u8>(), bayt_ruus),
                core::slice::from_raw_parts(self.faharis.as_ptr().cast::<u8>(), bayt_faharis),
            )
        };

        // SAFETY: both names were created by `ibni_tarteeb` in this context and
        // have not been deleted; binding them makes the two uploads below write
        // into the overlay's own buffers.
        unsafe {
            (self.dawall.bind_vertex_array)(self.tarteeb);
            (self.dawall.bind_buffer)(GL_ARRAY_BUFFER, self.mahfazat_raas);
            (self.dawall.bind_buffer)(GL_ELEMENT_ARRAY_BUFFER, self.mahfazat_fahras);
        }
        let saat_raas = arfa_mahfaza(
            &self.dawall,
            GL_ARRAY_BUFFER,
            khaam_ruus,
            self.saat_raas,
            "vertex buffer",
        )?;
        let saat_fahras = arfa_mahfaza(
            &self.dawall,
            GL_ELEMENT_ARRAY_BUFFER,
            khaam_faharis,
            self.saat_fahras,
            "index buffer",
        )?;
        self.saat_raas = saat_raas;
        self.saat_fahras = saat_fahras;

        let Some(adad) = qeema_sahih(self.faharis.len()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "index buffer",
                sabab: format!("{} indices does not fit a GLsizei", self.faharis.len()),
            });
        };
        let Some(manfath_ard) = qeema_sahih_min_adad(sath.ard) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "viewport",
                sabab: format!("a surface {} pixels wide does not fit a GLsizei", sath.ard),
            });
        };
        let Some(manfath_irtifa) = qeema_sahih_min_adad(sath.irtifa) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "viewport",
                sabab: format!("a surface {} pixels tall does not fit a GLsizei", sath.irtifa),
            });
        };
        let isqat = isqat_mustawi(sath.ard, sath.irtifa);
        // The transfer function is applied in the shader exactly when the
        // hardware is not applying it. See `SHIFRAT_QITA`.
        let tarmiz = Sahih::from(!sath.sirgb);

        // SAFETY: every call below writes context state or reads memory this
        // function owns. `isqat` is sixteen live floats and the count of one
        // matrix matches; the draw reads `adad` indices out of the element
        // buffer that was filled with exactly `self.faharis.len()` of them a few
        // lines above, and every index in it addresses a vertex that was written
        // in the same pass. The null offset is a byte offset into the bound
        // element buffer, which is what `glDrawElements` takes when a buffer is
        // bound rather than a client pointer.
        unsafe {
            (self.dawall.bind_framebuffer)(GL_DRAW_FRAMEBUFFER, 0);
            (self.dawall.viewport)(0, 0, manfath_ard, manfath_irtifa);
            (self.dawall.disable)(GL_SCISSOR_TEST);
            (self.dawall.disable)(GL_DEPTH_TEST);
            (self.dawall.disable)(GL_CULL_FACE);
            (self.dawall.depth_mask)(GL_FALSE);
            (self.dawall.color_mask)(GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE);
            (self.dawall.polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);

            (self.dawall.enable)(GL_BLEND);
            (self.dawall.blend_equation)(GL_FUNC_ADD);
            (self.dawall.blend_func_separate)(
                GL_ONE,
                GL_ONE_MINUS_SRC_ALPHA,
                GL_ONE,
                GL_ONE_MINUS_SRC_ALPHA,
            );

            (self.dawall.use_program)(self.barnamij);
            (self.dawall.uniform_matrix4fv)(self.mawdi_isqat, 1, GL_FALSE, isqat.as_ptr());
            (self.dawall.uniform1i)(self.mawdi_lawha, 0);
            (self.dawall.uniform1i)(self.mawdi_tarmiz, tarmiz);

            (self.dawall.active_texture)(GL_TEXTURE0);
            (self.dawall.bind_texture)(GL_TEXTURE_2D, self.nasij);

            (self.dawall.draw_elements)(GL_TRIANGLES, adad, GL_UNSIGNED_INT, core::ptr::null());
        }
        Ok(())
    }
}

/// A vertex count as the `GLuint` index that names it.
///
/// [`None`] past `u32::MAX`, which [`AQSA_QITAAT`] makes unreachable and which is
/// checked anyway because an index that wrapped would name a vertex from
/// somewhere else in the buffer and draw a triangle across the screen.
fn qeema_fahras(qeema: usize) -> Option<u32> {
    u32::try_from(qeema).ok()
}

/// A surface dimension as the `GLsizei` the viewport call takes.
fn qeema_sahih_min_adad(qeema: u32) -> Option<Sahih> {
    Sahih::try_from(qeema).ok()
}

// ---------------------------------------------------------------------------
// The atlas
// ---------------------------------------------------------------------------

impl KhattafGl {
    /// Creates the texture if it does not exist and writes the atlas into it.
    ///
    /// `GL_UNPACK_ALIGNMENT` is set to one and put back. The default is four,
    /// which is right for RGBA8 and wrong the moment an atlas is uploaded whose
    /// row length is not a multiple of four bytes — and it is *also* a piece of
    /// the game's pixel-store state, so leaving it at one would make the game's
    /// next `glTexSubImage2D` read its own rows at the wrong stride. It is not
    /// in [`HalatGl`] because it is not part of the draw; it is handled here,
    /// where it is the only state this method touches beyond the texture.
    fn arfa_nasij(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if self.nasij == 0 {
            let mut nasij: Adad = 0;
            // SAFETY: `nasij` is one live, aligned `GLuint` and the count of one
            // matches it exactly.
            unsafe { (self.dawall.gen_textures)(1, &raw mut nasij) };
            if nasij == 0 {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "glyph atlas texture",
                    sabab: "glGenTextures returned name zero, which means no context is \
                            current on this thread"
                        .to_owned(),
                });
            }
            self.nasij = nasij;
        }

        let Some(ard_gl) = qeema_sahih_min_adad(ard) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("an atlas {ard} pixels wide does not fit a GLsizei"),
            });
        };
        let Some(irtifa_gl) = qeema_sahih_min_adad(irtifa) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("an atlas {irtifa} pixels tall does not fit a GLsizei"),
            });
        };

        // SAFETY: `GL_UNPACK_ALIGNMENT` is a single-valued pixel-store state.
        let muhadhat_sabiqa = unsafe { self.dawall.sahih(GL_UNPACK_ALIGNMENT) };

        // SAFETY: the texture name was created above or on an earlier upload and
        // has not been deleted. `glTexImage2D` reads exactly
        // `ard * irtifa * 4` bytes at an alignment of one from `bayt`, whose
        // length the caller has already been checked to equal that product, so
        // the driver's read stays inside the slice.
        unsafe {
            (self.dawall.active_texture)(GL_TEXTURE0);
            (self.dawall.bind_texture)(GL_TEXTURE_2D, self.nasij);
            (self.dawall.pixel_storei)(GL_UNPACK_ALIGNMENT, 1);
            (self.dawall.tex_image2d)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                ard_gl,
                irtifa_gl,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                bayt.as_ptr().cast::<c_void>(),
            );
            // Linear filtering with no mip chain: the atlas is sampled at or
            // very near one texel per pixel, so a minification chain would cost
            // a third of the memory to be never selected. Clamp to edge because
            // a glyph at the edge of its cell sampled with `GL_REPEAT` picks up
            // the texel from the opposite side of the atlas, which draws a
            // one-pixel line of some unrelated letter along the edge of this one.
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            (self.dawall.pixel_storei)(GL_UNPACK_ALIGNMENT, muhadhat_sabiqa);
        }

        self.qiyas_lawha = (ard, irtifa);
        Ok(())
    }

    /// Reads one rectangle of the presented frame back into memory.
    ///
    /// Two adjustments make this correct and neither is visible in the call:
    ///
    /// **The origin.** `glReadPixels` measures from the bottom-left of the
    /// framebuffer. The rectangle it is given has been measured from the
    /// top-left, so the y it is handed is `height - (top + rect_height)` — and
    /// then the rows that come back are reversed, because the caller's image is
    /// also top-left. Both halves are needed: flipping without moving the origin
    /// reads the wrong band of the screen, and moving the origin without
    /// flipping reads the right band upside down.
    ///
    /// **The pack alignment.** The default of four means a row whose width is
    /// not a multiple of four *pixels* is padded, and the tightly packed buffer
    /// the trait promises would have gaps in it. It is set to one and put back,
    /// for the same reason the unpack alignment is.
    fn iqra_bikselat(
        &mut self,
        mintaqa: MustatilBiksel,
        sath: WasfSath,
    ) -> Result<Vec<u8>, KhataTabaqa> {
        let Some(asfal) = sath
            .irtifa
            .checked_sub(mintaqa.aala)
            .and_then(|baqi| baqi.checked_sub(mintaqa.irtifa))
        else {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: format!(
                    "{}×{} at {},{}",
                    mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala
                ),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        };

        let (Some(x), Some(y), Some(ard), Some(irtifa)) = (
            qeema_sahih_min_adad(mintaqa.yasar),
            qeema_sahih_min_adad(asfal),
            qeema_sahih_min_adad(mintaqa.ard),
            qeema_sahih_min_adad(mintaqa.irtifa),
        ) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region's pixel coordinates do not fit a GLint".to_owned(),
            });
        };

        let bayt_biksel = usize::try_from(SighatSath::Rgba8.bayt_lil_biksel()).unwrap_or(4);
        let Some(satr_bayt) =
            usize::try_from(mintaqa.ard).ok().and_then(|ard| ard.checked_mul(bayt_biksel))
        else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "a row of the region does not fit in memory on this target".to_owned(),
            });
        };
        let Some(tul) =
            usize::try_from(mintaqa.irtifa).ok().and_then(|irtifa| satr_bayt.checked_mul(irtifa))
        else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region does not fit in memory on this target".to_owned(),
            });
        };

        // SAFETY: `GL_PACK_ALIGNMENT` and `GL_READ_BUFFER` are single-valued
        // pixel-store and framebuffer states.
        let (muhadhat_sabiqa, mahfazat_qira) =
            unsafe { (self.dawall.sahih(GL_PACK_ALIGNMENT), self.dawall.sahih(GL_READ_BUFFER)) };

        let mut khaam = vec![0u8; tul];
        // SAFETY: the read framebuffer is bound to zero and its back buffer
        // selected immediately before the read, so the rectangle is inside the
        // default framebuffer that `sath` measured. `khaam` is exactly
        // `ard * irtifa * 4` writable bytes at an alignment of one, which with
        // `GL_RGBA`/`GL_UNSIGNED_BYTE` and a pack alignment of one is exactly
        // what the driver writes.
        unsafe {
            (self.dawall.bind_framebuffer)(GL_READ_FRAMEBUFFER, 0);
            (self.dawall.read_buffer)(GL_BACK);
            (self.dawall.pixel_storei)(GL_PACK_ALIGNMENT, 1);
            (self.dawall.read_pixels)(
                x,
                y,
                ard,
                irtifa,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                khaam.as_mut_ptr().cast::<c_void>(),
            );
            (self.dawall.pixel_storei)(GL_PACK_ALIGNMENT, muhadhat_sabiqa);
            (self.dawall.read_buffer)(ka_adad(mahfazat_qira));
        }

        let mut maqlub = Vec::with_capacity(tul);
        for satr in khaam.chunks_exact(satr_bayt).rev() {
            maqlub.extend_from_slice(satr);
        }
        Ok(maqlub)
    }

    /// Deletes every object this backend created, and forgets the names.
    ///
    /// Idempotent by construction: each name is zeroed as it is deleted, and
    /// zero is the name OpenGL defines as "no object", which every delete call
    /// silently ignores. A second `ahmil` therefore issues no calls at all,
    /// which matters because the second call is the common one — a backend whose
    /// initialisation failed is exactly the backend that gets torn down.
    fn atlif(&mut self) {
        // SAFETY: every name below was created by this backend in this context
        // and has not been deleted, and each count matches the single-element
        // array beside it. Deleting a name of zero is defined as a no-op, which
        // is what makes the repeat call safe rather than merely tolerated.
        unsafe {
            if self.barnamij != 0 {
                (self.dawall.delete_program)(self.barnamij);
                self.barnamij = 0;
            }
            if self.tarteeb != 0 {
                (self.dawall.delete_vertex_arrays)(1, &raw const self.tarteeb);
                self.tarteeb = 0;
            }
            if self.mahfazat_raas != 0 {
                (self.dawall.delete_buffers)(1, &raw const self.mahfazat_raas);
                self.mahfazat_raas = 0;
            }
            if self.mahfazat_fahras != 0 {
                (self.dawall.delete_buffers)(1, &raw const self.mahfazat_fahras);
                self.mahfazat_fahras = 0;
            }
            if self.nasij != 0 {
                (self.dawall.delete_textures)(1, &raw const self.nasij);
                self.nasij = 0;
            }
        }
        self.mawdi_isqat = -1;
        self.mawdi_lawha = -1;
        self.mawdi_tarmiz = -1;
        self.saat_raas = 0;
        self.saat_fahras = 0;
        self.qiyas_lawha = (0, 0);
        self.sath = None;
        self.ruus = Vec::new();
        self.faharis = Vec::new();
    }
}

// ---------------------------------------------------------------------------
// The contract
// ---------------------------------------------------------------------------

impl Khattaf for KhattafGl {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::OpenGl
    }

    /// The surface, as far as OpenGL will admit to having one.
    ///
    /// The size comes from the platform layer when it has told us
    /// ([`KhattafGl::hadith_qiyas`]) and from `GL_VIEWPORT` when it has not,
    /// which is the best OpenGL itself offers.
    ///
    /// The format is always [`SighatSath::Rgba8`], and that is a statement of
    /// fact rather than a default. In Direct3D the capture format is a property
    /// of the swap chain and the overlay has to take whatever it finds. In
    /// OpenGL the read-back format is a property of the *request*:
    /// `glReadPixels` is given the format and type it should produce and the
    /// driver converts from whatever the framebuffer really holds. This backend
    /// asks for `GL_RGBA`/`GL_UNSIGNED_BYTE`, so RGBA8 is what comes back, on a
    /// 10-bit surface and on a float one alike.
    ///
    /// The sRGB flag is `GL_FRAMEBUFFER_SRGB`, because that — not the
    /// framebuffer's true encoding, which core OpenGL will not report — is what
    /// determines whether the hardware applies the transfer function to what the
    /// overlay writes. That is the fact the blend needs, and it is the fact
    /// [`SHIFRAT_QITA`] is switched on.
    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        // SAFETY: `GL_VIEWPORT` is a four-valued state in the specification's
        // viewport table.
        let [_, _, manfath_ard, manfath_irtifa] = unsafe { self.dawall.sahih_arbaa(GL_VIEWPORT) };

        let (ard, irtifa) = if let Some(qiyas) = self.qiyas_mubarmaj { qiyas } else {
            let ard = u32::try_from(manfath_ard).unwrap_or(0);
            let irtifa = u32::try_from(manfath_irtifa).unwrap_or(0);
            (ard, irtifa)
        };
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the drawable measures {ard}×{irtifa}, which a window being resized or a \
                     context that is not current on this thread both produce"
                ),
            });
        }

        Ok(WasfSath {
            ard,
            irtifa,
            sigha: SighatSath::Rgba8,
            sirgb: self.dawall.mufaal(GL_FRAMEBUFFER_SRGB),
        })
    }

    /// Creates what does not exist yet and records the surface.
    ///
    /// Nothing this backend owns is sized against the surface: the projection is
    /// a uniform, the vertex buffer is sized by the batch, and the atlas is
    /// sized by the font. So a resolution change rebuilds *nothing*, and saying
    /// that plainly is better than performing a rebuild to look diligent —
    /// recompiling two shaders on a fullscreen transition is a stall the player
    /// sees, in exchange for objects that were already correct.
    ///
    /// What the first call does create is the program and the vertex array, and
    /// it does so with the game's state saved around it, because creating a
    /// vertex array means binding one.
    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        if sath.ard == 0 || sath.irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("a {}×{} surface has no pixels to draw on", sath.ard, sath.irtifa),
            });
        }

        let natija = self.bi_hifz_hala(RaddFasad::Mawrid, |hadha| {
            if hadha.aqsa_nasij == 0 {
                // SAFETY: `GL_MAX_TEXTURE_SIZE` is a single-valued
                // implementation limit.
                let khaam = unsafe { hadha.dawall.sahih(GL_MAX_TEXTURE_SIZE) };
                hadha.aqsa_nasij = u32::try_from(khaam).unwrap_or(0);
            }
            if hadha.barnamij == 0 {
                hadha.ibni_barnamij()?;
            }
            if hadha.tarteeb == 0 {
                hadha.ibni_tarteeb()?;
            }
            Ok(())
        });

        match natija {
            Ok(()) => {
                if self.sath != Some(sath) {
                    self.athar.push(format!(
                        "surface {}×{}{}",
                        sath.ard,
                        sath.irtifa,
                        if sath.sirgb { ", GL_FRAMEBUFFER_SRGB enabled" } else { "" }
                    ));
                }
                self.sath = Some(sath);
                Ok(())
            }
            Err(khata) => {
                // A half-built backend is torn down here rather than left for
                // the next frame to trip over: `ibni_tarteeb` can fail after
                // `ibni_barnamij` succeeded, and a program with no vertex array
                // is a state the draw path has no branch for.
                self.atlif();
                Err(khata)
            }
        }
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("an atlas of {ard}×{irtifa} has no texels"),
            });
        }
        let bayt_biksel = u64::from(SighatSath::Rgba8.bayt_lil_biksel());
        let Some(matlub) = u64::from(ard)
            .checked_mul(u64::from(irtifa))
            .and_then(|bikselat| bikselat.checked_mul(bayt_biksel))
        else {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas",
                qeema: u64::MAX,
                saqf: AQSA_LAWHA,
            });
        };
        if matlub > AQSA_LAWHA {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas",
                qeema: matlub,
                saqf: AQSA_LAWHA,
            });
        }
        if self.aqsa_nasij != 0 && (ard > self.aqsa_nasij || irtifa > self.aqsa_nasij) {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas edge",
                qeema: u64::from(ard.max(irtifa)),
                saqf: u64::from(self.aqsa_nasij),
            });
        }
        let mawjud = crate::khata::tul_u64(bayt.len());
        if mawjud != matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "the atlas declares {ard}×{irtifa} RGBA8, which is {matlub} bytes, and \
                     {mawjud} bytes were supplied"
                ),
            });
        }

        self.bi_hifz_hala(RaddFasad::Mawrid, |hadha| hadha.arfa_nasij(bayt, ard, irtifa))
    }

    /// Saves the context, draws, restores the context, and audits the restore.
    ///
    /// The audit is the part that has no counterpart in the other three
    /// backends. D3D and Vulkan report a failed state operation as a value the
    /// call site can look at; OpenGL sets a flag in a queue and returns nothing.
    /// So this is the one API where a restore that did not take is genuinely
    /// undetectable unless somebody asks — and if nobody asks, the game carries
    /// on rendering with the overlay's blend function, the overlay's program, or
    /// the overlay's viewport, and every frame after it is wrong in a way that
    /// looks like the game's own bug.
    ///
    /// A non-zero `glGetError` after the restore is therefore
    /// [`KhataTabaqa::HalaGhayrMustaada`], which
    /// [`crate::wajiha::Tabaqa::itar`] treats as terminal. Not a retry: the
    /// state the retry would run against is already wrong.
    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if lawha.qitaat.len() > AQSA_QITAAT {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: format!(
                    "the batch carries {} quads, above this build's ceiling of {AQSA_QITAAT}; \
                     a batch this large is a fault in whatever built it rather than a frame \
                     worth drawing",
                    lawha.qitaat.len()
                ),
            });
        }
        if self.barnamij == 0 || self.tarteeb == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: "the overlay was asked to draw before its program and vertex array \
                        existed, which means `hayyi` has not run or failed"
                    .to_owned(),
            });
        }

        self.ibni_dufaa(lawha);
        if self.faharis.is_empty() {
            // Every quad in the batch had a zero dimension. Not an error — a
            // shaped line with no visible glyphs produces exactly this — and
            // saving and restoring the whole context to draw nothing would be
            // the expensive way to do nothing.
            return Ok(());
        }

        let sath = lawha.sath;
        self.bi_hifz_hala(RaddFasad::Hala, |hadha| hadha.arsil_dufaa(sath))
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = Khattaf::sath(self)?;
        let wasf = || {
            format!("{}×{} at {},{}", mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala)
        };
        if mintaqa.ard == 0 || mintaqa.irtifa == 0 {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: wasf(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        }
        let yameen = mintaqa.yasar.checked_add(mintaqa.ard);
        let asfal = mintaqa.aala.checked_add(mintaqa.irtifa);
        match (yameen, asfal) {
            (Some(yameen), Some(asfal)) if yameen <= sath.ard && asfal <= sath.irtifa => {}
            _ => {
                return Err(KhataTabaqa::MintaqaKharij {
                    mintaqa: wasf(),
                    ard: sath.ard,
                    irtifa: sath.irtifa,
                });
            }
        }

        self.bi_hifz_hala(RaddFasad::Iltiqat, |hadha| hadha.iqra_bikselat(mintaqa, sath))
    }

    /// Releases everything, and says so if the driver complained on the way out.
    ///
    /// The error is reported and not acted on, which is
    /// [`Khattaf::ahmil`]'s documented posture: by the time this runs the
    /// overlay is either shutting down or already disabled, and there is nothing
    /// useful left to do about a delete that did not take. It is still returned,
    /// because a driver that errors on `glDeleteProgram` is a fact worth having
    /// in a diagnostics bundle.
    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        if self.barnamij == 0
            && self.tarteeb == 0
            && self.mahfazat_raas == 0
            && self.mahfazat_fahras == 0
            && self.nasij == 0
        {
            return Ok(());
        }
        let _ = self.dawall.ifragh();
        self.atlif();
        let baqi = self.dawall.ifragh();
        if baqi == GL_NO_ERROR {
            self.athar.push("released every OpenGL object the overlay created".to_owned());
            return Ok(());
        }
        let sabab = format!(
            "{} was set while releasing the overlay's OpenGL objects, on {}",
            ism_khata(baqi),
            self.dawall.wasf
        );
        self.athar.push(sabab.clone());
        Err(KhataTabaqa::MawridFashil { mawrid: "the overlay's OpenGL objects", sabab })
    }
}
