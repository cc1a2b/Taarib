//! جي‌إل الثابت — the overlay drawn through a pipeline that has no shaders.
//!
//! [`crate::gl`] is the OpenGL backend. This is not a variant of it and reading
//! it as one is how a fixed-function game ends up with a black screen. That
//! module resolves fifty-five entry points that are **core in 3.3**, compiles
//! two GLSL stages, binds a vertex array object and streams into buffer
//! objects. A context created before 2004 has none of those: no
//! `glCreateShader`, no `glGenVertexArrays`, no `glGenBuffers`, no
//! `glActiveTexture`. Its whole vocabulary is a matrix stack, a texture
//! environment, an attribute stack and `glBegin`.
//!
//! So this is a second backend rather than a branch in the first, and the two
//! share one thing only: the buffer swap they are reached at.
//!
//! ## What is hooked, and what is not
//!
//! `wglSwapBuffers` on Windows and `glXSwapBuffers` elsewhere — the same detour
//! [`crate::gl`] is reached through, installed once by the bootstrap and shared
//! by both. `SwapBuffers` in `gdi32.dll` is deliberately **not** hooked as well,
//! and that is worth stating because a great many legacy games call it: on
//! Windows the GDI entry point routes a window with a current OpenGL context
//! straight into `opengl32.dll`'s own swap, so hooking `wglSwapBuffers` catches
//! both callers. A second detour would catch the same frame twice.
//!
//! ## `wglGetProcAddress` is the wrong resolver here, exactly backwards
//!
//! [`crate::gl`] resolves through `wglGetProcAddress` first and falls back to
//! the module's exports, because every symbol a core profile needs above
//! `glGetError` is an extension address the ICD supplies. Every symbol *this*
//! backend needs is an OpenGL 1.1 entry point, and `wglGetProcAddress` is
//! documented to return null for those. The order is therefore reversed: the
//! module's own export table, and nothing else. A loader copied from the modern
//! backend would resolve nothing at all here.
//!
//! ## Saving state that no modern backend has heard of
//!
//! [`crate::gl::HalatGl`] enumerates twenty-two pieces of context state and
//! reads each one back. None of them is a matrix, a texture environment, a
//! material, a light, an alpha test or a fog setting, because a core profile
//! has none of those — and a fixed-function game has all of them, set, and will
//! not re-set what it believes is already there.
//!
//! Fixed-function OpenGL has the right instrument for this and the modern
//! profile removed it: `glPushAttrib(GL_ALL_ATTRIB_BITS)` saves every server
//! state group in one call, `glPushClientAttrib(GL_CLIENT_ALL_ATTRIB_BITS)`
//! saves the client ones, and the three matrix stacks save themselves. That is
//! what [`HifzThabit`] does, and it is the fixed-function analogue of Direct3D
//! 8's state block rather than of the modern backend's read-back list.
//!
//! **The stacks are checked before they are pushed.** This is the part that
//! looks like defensiveness and is not: the projection matrix stack is required
//! to be only **two** deep, and a game that has already pushed one projection
//! matrix leaves exactly one slot. A `glPushMatrix` past the end sets
//! `GL_STACK_OVERFLOW`, does nothing, and the matching `glPopMatrix` then pops
//! the **game's** matrix off instead of the overlay's — which is a game whose
//! camera changes the first frame the overlay draws, and nothing in it points
//! back here. So every stack is measured against its own limit and the frame is
//! skipped rather than drawn when there is no room.
//!
//! ## Where the transfer function goes when there is no fragment stage
//!
//! Nowhere in the pipeline, so onto the CPU. Colours arrive premultiplied and
//! linear; [`crate::d3d8::lawn_marmuz`] divides the alpha out, encodes, and
//! multiplies it back, once per quad. The blend that follows happens in the
//! framebuffer's own encoding — which is the space the game's own interface was
//! blended in, and matching the game is the target on a pipeline that predates
//! the question. That conversion is shared with [`crate::d3d8`] rather than
//! copied, because the two backends are making one decision and a second copy
//! is a second place it could drift.

use core::ffi::c_void;
use core::fmt;

use crate::khata::KhataTabaqa;
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

// ---------------------------------------------------------------------------
// The scalar vocabulary
// ---------------------------------------------------------------------------

/// `GLenum`, `GLuint` and `GLbitfield`.
type Adad = u32;
/// `GLint` and `GLsizei`.
type Sahih = i32;
/// `GLboolean`, which is a byte and not a `bool`.
type Mantiqi = u8;
/// `GLdouble`, which `glOrtho` alone among these takes.
type Muda = f64;

// ---------------------------------------------------------------------------
// The constants, at the values the specification gives them
// ---------------------------------------------------------------------------

/// `GL_NO_ERROR`.
const GL_NO_ERROR: Adad = 0;
/// `GL_INVALID_ENUM`.
const GL_INVALID_ENUM: Adad = 0x0500;
/// `GL_INVALID_VALUE`.
const GL_INVALID_VALUE: Adad = 0x0501;
/// `GL_INVALID_OPERATION`.
const GL_INVALID_OPERATION: Adad = 0x0502;
/// `GL_STACK_OVERFLOW`, which this backend goes to some length never to cause.
const GL_STACK_OVERFLOW: Adad = 0x0503;
/// `GL_STACK_UNDERFLOW`.
const GL_STACK_UNDERFLOW: Adad = 0x0504;
/// `GL_OUT_OF_MEMORY`.
const GL_OUT_OF_MEMORY: Adad = 0x0505;

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

/// `GL_ALL_ATTRIB_BITS`.
const GL_ALL_ATTRIB_BITS: Adad = 0x000F_FFFF;
/// `GL_CLIENT_ALL_ATTRIB_BITS`.
const GL_CLIENT_ALL_ATTRIB_BITS: Adad = 0xFFFF_FFFF;
/// `GL_ATTRIB_STACK_DEPTH`.
const GL_ATTRIB_STACK_DEPTH: Adad = 0x0BB0;
/// `GL_CLIENT_ATTRIB_STACK_DEPTH`.
const GL_CLIENT_ATTRIB_STACK_DEPTH: Adad = 0x0BB1;
/// `GL_MAX_ATTRIB_STACK_DEPTH`, which the specification floors at sixteen.
const GL_MAX_ATTRIB_STACK_DEPTH: Adad = 0x0D35;
/// `GL_MAX_CLIENT_ATTRIB_STACK_DEPTH`, floored at sixteen.
const GL_MAX_CLIENT_ATTRIB_STACK_DEPTH: Adad = 0x0D3B;

/// `GL_MATRIX_MODE`.
const GL_MATRIX_MODE: Adad = 0x0BA0;
/// `GL_MODELVIEW`.
const GL_MODELVIEW: Adad = 0x1700;
/// `GL_PROJECTION`.
const GL_PROJECTION: Adad = 0x1701;
/// `GL_TEXTURE`, as a matrix mode.
const GL_TEXTURE: Adad = 0x1702;
/// `GL_MODELVIEW_STACK_DEPTH`.
const GL_MODELVIEW_STACK_DEPTH: Adad = 0x0BA3;
/// `GL_PROJECTION_STACK_DEPTH`.
const GL_PROJECTION_STACK_DEPTH: Adad = 0x0BA4;
/// `GL_TEXTURE_STACK_DEPTH`.
const GL_TEXTURE_STACK_DEPTH: Adad = 0x0BA5;
/// `GL_MAX_MODELVIEW_STACK_DEPTH`, floored at thirty-two.
const GL_MAX_MODELVIEW_STACK_DEPTH: Adad = 0x0D36;
/// `GL_MAX_PROJECTION_STACK_DEPTH`, floored at **two**.
const GL_MAX_PROJECTION_STACK_DEPTH: Adad = 0x0D38;
/// `GL_MAX_TEXTURE_STACK_DEPTH`, floored at **two**.
const GL_MAX_TEXTURE_STACK_DEPTH: Adad = 0x0D39;

/// `GL_QUADS`, which a core profile removed and this pipeline is built on.
const GL_QUADS: Adad = 0x0007;
/// `GL_TEXTURE_2D`.
const GL_TEXTURE_2D: Adad = 0x0DE1;
/// `GL_TEXTURE_ENV`.
const GL_TEXTURE_ENV: Adad = 0x2300;
/// `GL_TEXTURE_ENV_MODE`.
const GL_TEXTURE_ENV_MODE: Adad = 0x2200;
/// `GL_MODULATE`, the texture environment the overlay needs.
const GL_MODULATE: Sahih = 0x2100;

/// `GL_BLEND`.
const GL_BLEND: Adad = 0x0BE2;
/// `GL_ONE`.
const GL_ONE: Adad = 1;
/// `GL_ONE_MINUS_SRC_ALPHA`.
const GL_ONE_MINUS_SRC_ALPHA: Adad = 0x0303;
/// `GL_DEPTH_TEST`.
const GL_DEPTH_TEST: Adad = 0x0B71;
/// `GL_CULL_FACE`.
const GL_CULL_FACE: Adad = 0x0B44;
/// `GL_SCISSOR_TEST`.
const GL_SCISSOR_TEST: Adad = 0x0C11;
/// `GL_LIGHTING`, which a fixed-function game leaves on and which would tint
/// every glyph by whatever material was last set.
const GL_LIGHTING: Adad = 0x0B50;
/// `GL_FOG`, which would fade the overlay with distance it does not have.
const GL_FOG: Adad = 0x0B60;
/// `GL_ALPHA_TEST`, which would discard every antialiased glyph edge.
const GL_ALPHA_TEST: Adad = 0x0BC0;
/// `GL_STENCIL_TEST`.
const GL_STENCIL_TEST: Adad = 0x0B90;
/// `GL_COLOR_MATERIAL`.
const GL_COLOR_MATERIAL: Adad = 0x0B57;
/// `GL_DITHER`.
const GL_DITHER: Adad = 0x0BD0;
/// `GL_TEXTURE_GEN_S`, which would replace the overlay's own coordinates.
const GL_TEXTURE_GEN_S: Adad = 0x0C60;
/// `GL_TEXTURE_GEN_T`.
const GL_TEXTURE_GEN_T: Adad = 0x0C61;

/// `GL_SMOOTH`.
const GL_SMOOTH: Adad = 0x1D01;
/// `GL_FILL`.
const GL_FILL: Adad = 0x1B02;
/// `GL_FRONT_AND_BACK`.
const GL_FRONT_AND_BACK: Adad = 0x0408;
/// `GL_FALSE`.
const GL_FALSE: Mantiqi = 0;
/// `GL_TRUE`.
const GL_TRUE: Mantiqi = 1;

/// `GL_RGBA`.
const GL_RGBA: Adad = 0x1908;
/// `GL_RGBA8`, a sized internal format OpenGL 1.1 introduced.
const GL_RGBA8: Sahih = 0x8058;
/// `GL_UNSIGNED_BYTE`.
const GL_UNSIGNED_BYTE: Adad = 0x1401;
/// `GL_LINEAR`.
const GL_LINEAR: Sahih = 0x2601;
/// `GL_CLAMP`, which is all OpenGL 1.1 has.
const GL_CLAMP: Sahih = 0x2900;
/// `GL_CLAMP_TO_EDGE`, which arrived in 1.2 and is preferred when it exists.
const GL_CLAMP_TO_EDGE: Sahih = 0x812F;
/// `GL_TEXTURE_MAG_FILTER`.
const GL_TEXTURE_MAG_FILTER: Adad = 0x2800;
/// `GL_TEXTURE_MIN_FILTER`.
const GL_TEXTURE_MIN_FILTER: Adad = 0x2801;
/// `GL_TEXTURE_WRAP_S`.
const GL_TEXTURE_WRAP_S: Adad = 0x2802;
/// `GL_TEXTURE_WRAP_T`.
const GL_TEXTURE_WRAP_T: Adad = 0x2803;
/// `GL_UNPACK_ALIGNMENT`.
const GL_UNPACK_ALIGNMENT: Adad = 0x0CF5;
/// `GL_PACK_ALIGNMENT`.
const GL_PACK_ALIGNMENT: Adad = 0x0D05;
/// `GL_BACK`.
const GL_BACK: Adad = 0x0405;

/// The most quads one batch may carry.
///
/// Immediate mode costs four `glVertex2f` calls and four `glTexCoord2f` calls
/// per quad, so sixteen thousand quads is a hundred and twenty-eight thousand
/// entry-point calls in one frame. That is already more than any dialogue box
/// produces and far more than a driver of this generation enjoys; above it, the
/// batch is a fault in whatever built it rather than a frame worth drawing.
const AQSA_QITAAT: usize = 16_384;

/// The largest atlas this backend will upload, in bytes.
const AQSA_LAWHA: u64 = 64 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Resolving the entry points
// ---------------------------------------------------------------------------

/// Where an OpenGL entry point's address comes from.
///
/// A trait rather than a concrete loader for the same reason
/// [`crate::gl::KhattafGl::bi_dawall`] takes an already-resolved table: a
/// backend is testable exactly to the degree that it does not go looking for
/// its own inputs. [`WahdatGl`] is the real one; a test supplies its own and
/// then observes every call the draw makes.
pub trait MuhillGl: fmt::Debug {
    /// The address of one entry point, or [`None`] when the module has none.
    fn unwan(&self, ramz: &str) -> Option<*const c_void>;

    /// What to call this source in [`KhataTabaqa::MaktabaMafquda`].
    fn ism(&self) -> &str;
}

/// The OpenGL module the game is already using, resolved through its exports.
///
/// **Exports only.** `wglGetProcAddress` is not consulted and must not be: it
/// is specified to return null for the OpenGL 1.1 entry points, which is every
/// entry point in this file. The modern backend's resolution order is the
/// reverse of this one and each is right for its own set of symbols.
#[derive(Debug)]
pub struct WahdatGl {
    qaida: *mut c_void,
    ism: &'static str,
}

impl WahdatGl {
    /// The modules a fixed-function context can be behind, in the order tried.
    const MURASHAHAT: &'static [&'static str] = &[
        "opengl32.dll",
        "libGL.so.1",
        "libGL.so",
        "/System/Library/Frameworks/OpenGL.framework/OpenGL",
    ];

    /// Finds the module the game already has, without loading one.
    ///
    /// A candidate is accepted only when it exports `glBegin`, which is the
    /// symbol that separates an OpenGL implementation from anything else and —
    /// unlike `glGetString` — is absent from a pure ES library that could never
    /// serve this backend anyway.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MaktabaMafquda`] naming every candidate that was tried.
    pub fn ijid() -> Result<Self, KhataTabaqa> {
        for ism in Self::MURASHAHAT {
            let Some(qaida) = taarib_haqn::mawqi::qaidat_wahda(ism) else {
                continue;
            };
            // SAFETY: `qaida` is the base `qaidat_wahda` just reported for a
            // module this process has loaded, which is `ramz_wahda`'s stated
            // precondition.
            if unsafe { taarib_haqn::mawqi::ramz_wahda(qaida, "glBegin") }.is_some() {
                return Ok(Self { qaida, ism });
            }
        }
        Err(KhataTabaqa::MaktabaMafquda {
            maktaba: Self::MURASHAHAT.join(", "),
            ramz: "glBegin".to_owned(),
        })
    }
}

impl MuhillGl for WahdatGl {
    fn unwan(&self, ramz: &str) -> Option<*const c_void> {
        // SAFETY: `self.qaida` is a base this type obtained from
        // `qaidat_wahda` and has not closed.
        unsafe { taarib_haqn::mawqi::ramz_wahda(self.qaida, ramz) }.map(<*mut c_void>::cast_const)
    }

    fn ism(&self) -> &str {
        self.ism
    }
}

// SAFETY: `qaida` is a module base — an integer the loader handed back — that
// this type reads and never writes through, and the module it names stays
// mapped for as long as the game renders. Moving the value to the render thread
// conveys no capability the process does not already have.
unsafe impl Send for WahdatGl {}

/// Reinterprets a resolved address as a function pointer of the wanted shape.
///
/// # Safety
///
/// `muashir` must be the address the platform returned for an OpenGL entry
/// point whose C signature is exactly `T`.
const unsafe fn ka_dalla<T: Copy>(muashir: *const c_void) -> Option<T> {
    if size_of::<T>() != size_of::<*const c_void>() {
        return None;
    }
    // SAFETY: `T` is pointer-sized by the check above and the caller guarantees
    // the signature; `transmute_copy` reads exactly that many bytes out of a
    // live local.
    Some(unsafe { core::mem::transmute_copy::<*const c_void, T>(&muashir) })
}

// ---------------------------------------------------------------------------
// The table
// ---------------------------------------------------------------------------

/// Declares the entry-point table and the one pass that fills it.
///
/// A macro for the reason [`crate::gl`]'s is one: the failure mode of two
/// hand-written lists is a field that is declared and never resolved, which
/// holds whatever the struct literal put there while the loader reports
/// success. Here a symbol exists in exactly one place and cannot be half-added.
macro_rules! jadwal_thabit {
    ( $( $haql:ident : $ramz:literal => $sigha:ty ; )+ ) => {
        /// Every fixed-function entry point the overlay calls, resolved once.
        ///
        /// All of them are OpenGL 1.1 and all of them are direct exports of the
        /// module. A symbol missing here means the module is not an OpenGL
        /// implementation at all, which is worth finding out while resolving
        /// rather than at the first draw.
        pub struct DawallThabit {
            $( $haql: $sigha, )+
            /// The context's version, as `(major, minor)`.
            isdar: (u32, u32),
            /// `GL_VENDOR`, `GL_RENDERER` and `GL_VERSION`, for the report.
            wasf: String,
        }

        impl DawallThabit {
            /// Resolves every entry point out of one source.
            ///
            /// # Errors
            ///
            /// [`KhataTabaqa::MaktabaMafquda`] naming the source and the first
            /// symbol that could not be resolved.
            pub fn min_muhill(muhill: &dyn MuhillGl) -> Result<Self, KhataTabaqa> {
                $(
                    let $haql = {
                        let mafquda = || KhataTabaqa::MaktabaMafquda {
                            maktaba: muhill.ism().to_owned(),
                            ramz: $ramz.to_owned(),
                        };
                        let muashir = muhill.unwan($ramz).ok_or_else(mafquda)?;
                        // SAFETY: every signature in the table below is
                        // transcribed from the OpenGL 1.1 specification, where
                        // all of these entry points are declared, so each
                        // resolution asks for the shape its own symbol has.
                        // `extern "system"` is `APIENTRY` on Windows and
                        // `extern "C"` everywhere else, which is what the
                        // specification's `GLAPIENTRY` expands to on each.
                        unsafe { ka_dalla::<$sigha>(muashir) }.ok_or_else(mafquda)?
                    };
                )+
                let mut dawall = Self { $( $haql, )+ isdar: (0, 0), wasf: String::new() };
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

jadwal_thabit! {
    get_error: "glGetError" => unsafe extern "system" fn() -> Adad;
    get_string: "glGetString" => unsafe extern "system" fn(Adad) -> *const u8;
    get_integerv: "glGetIntegerv" => unsafe extern "system" fn(Adad, *mut Sahih);
    enable: "glEnable" => unsafe extern "system" fn(Adad);
    disable: "glDisable" => unsafe extern "system" fn(Adad);
    push_attrib: "glPushAttrib" => unsafe extern "system" fn(Adad);
    pop_attrib: "glPopAttrib" => unsafe extern "system" fn();
    push_client_attrib: "glPushClientAttrib" => unsafe extern "system" fn(Adad);
    pop_client_attrib: "glPopClientAttrib" => unsafe extern "system" fn();
    matrix_mode: "glMatrixMode" => unsafe extern "system" fn(Adad);
    push_matrix: "glPushMatrix" => unsafe extern "system" fn();
    pop_matrix: "glPopMatrix" => unsafe extern "system" fn();
    load_identity: "glLoadIdentity" => unsafe extern "system" fn();
    ortho: "glOrtho" => unsafe extern "system" fn(Muda, Muda, Muda, Muda, Muda, Muda);
    viewport: "glViewport" => unsafe extern "system" fn(Sahih, Sahih, Sahih, Sahih);
    blend_func: "glBlendFunc" => unsafe extern "system" fn(Adad, Adad);
    depth_mask: "glDepthMask" => unsafe extern "system" fn(Mantiqi);
    color_mask: "glColorMask" => unsafe extern "system" fn(Mantiqi, Mantiqi, Mantiqi, Mantiqi);
    shade_model: "glShadeModel" => unsafe extern "system" fn(Adad);
    polygon_mode: "glPolygonMode" => unsafe extern "system" fn(Adad, Adad);
    gen_textures: "glGenTextures" => unsafe extern "system" fn(Sahih, *mut Adad);
    bind_texture: "glBindTexture" => unsafe extern "system" fn(Adad, Adad);
    delete_textures: "glDeleteTextures" => unsafe extern "system" fn(Sahih, *const Adad);
    tex_image2d: "glTexImage2D"
        => unsafe extern "system" fn(
            Adad, Sahih, Sahih, Sahih, Sahih, Sahih, Adad, Adad, *const c_void,
        );
    tex_parameteri: "glTexParameteri" => unsafe extern "system" fn(Adad, Adad, Sahih);
    tex_envi: "glTexEnvi" => unsafe extern "system" fn(Adad, Adad, Sahih);
    pixel_storei: "glPixelStorei" => unsafe extern "system" fn(Adad, Sahih);
    read_pixels: "glReadPixels"
        => unsafe extern "system" fn(Sahih, Sahih, Sahih, Sahih, Adad, Adad, *mut c_void);
    read_buffer: "glReadBuffer" => unsafe extern "system" fn(Adad);
    begin: "glBegin" => unsafe extern "system" fn(Adad);
    end: "glEnd" => unsafe extern "system" fn();
    color4f: "glColor4f" => unsafe extern "system" fn(f32, f32, f32, f32);
    tex_coord2f: "glTexCoord2f" => unsafe extern "system" fn(f32, f32);
    vertex2f: "glVertex2f" => unsafe extern "system" fn(f32, f32);
}

impl fmt::Debug for DawallThabit {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The addresses are elided: they are a module base plus an offset in
        // somebody else's process and they answer no question a reader of a
        // diagnostics bundle has.
        mukhraj
            .debug_struct("DawallThabit")
            .field("isdar", &self.isdar)
            .field("wasf", &self.wasf)
            .field("adad_dawall", &Self::adad_dawall())
            .finish_non_exhaustive()
    }
}

impl DawallThabit {
    /// The context's version, as `(major, minor)`.
    #[must_use]
    pub const fn isdar(&self) -> (u32, u32) {
        self.isdar
    }

    /// The vendor, renderer and version strings, for the diagnostics bundle.
    #[must_use]
    pub fn wasf(&self) -> &str {
        &self.wasf
    }

    /// A `glGetString` result as an owned Rust string, bounded.
    fn nass(&self, mawdu: Adad) -> String {
        const AQSA: usize = 4096;
        // SAFETY: `glGetString` takes a name and returns either null or a
        // pointer to a NUL-terminated, driver-owned string.
        let khaam = unsafe { (self.get_string)(mawdu) };
        if khaam.is_null() {
            return String::new();
        }
        let mut tul = 0usize;
        while tul < AQSA {
            // SAFETY: the driver's string is NUL-terminated, so every byte up
            // to and including the terminator is inside the allocation; the
            // scan stops there and is bounded regardless.
            if unsafe { khaam.add(tul).read() } == 0 {
                break;
            }
            tul += 1;
        }
        // SAFETY: `tul` bytes were just read one at a time from this pointer.
        let bayt = unsafe { core::slice::from_raw_parts(khaam, tul) };
        String::from_utf8_lossy(bayt).into_owned()
    }

    /// The context's version, parsed out of `GL_VERSION`.
    fn iqra_isdar(&self) -> (u32, u32) {
        let nass = self.nass(GL_VERSION);
        let mut raqmiya = nass.split(|harf: char| !harf.is_ascii_digit());
        let kabir = raqmiya
            .find(|juz: &&str| !juz.is_empty())
            .and_then(|juz| juz.parse().ok());
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
    /// `mawdu` must name a state the specification gives **exactly one** value.
    unsafe fn sahih(&self, mawdu: Adad) -> Sahih {
        let mut qeema: Sahih = 0;
        // SAFETY: the caller guarantees `mawdu` is single-valued, and `qeema`
        // is one live, aligned `GLint`.
        unsafe { (self.get_integerv)(mawdu, &raw mut qeema) };
        qeema
    }

    /// Four integers of state, which here is `GL_VIEWPORT` and nothing else.
    ///
    /// # Safety
    ///
    /// `mawdu` must name a state with **exactly four** values.
    unsafe fn sahih_arbaa(&self, mawdu: Adad) -> [Sahih; 4] {
        let mut qeema: [Sahih; 4] = [0; 4];
        // SAFETY: the caller guarantees the arity; `qeema` is four live,
        // aligned `GLint`s.
        unsafe { (self.get_integerv)(mawdu, qeema.as_mut_ptr()) };
        qeema
    }

    /// Drains the error queue and returns the first error it held.
    ///
    /// Bounded at thirty-two, because a driver that never returns
    /// `GL_NO_ERROR` would otherwise spin forever on a render thread — a hang
    /// the player experiences as their game freezing, caused by an overlay
    /// trying to be careful.
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

    /// Whether a stack has room for one more push.
    ///
    /// The two arguments are the state that reports the current depth and the
    /// one that reports the limit. Reading both rather than assuming the
    /// specification's floor is what makes this correct on a driver that offers
    /// more, and reading the *current* depth is what makes it correct against a
    /// game that has already pushed.
    ///
    /// # Safety
    ///
    /// Both names must be single-valued states.
    unsafe fn fiha_masaha(&self, hali: Adad, aqsa: Adad) -> bool {
        // SAFETY: the caller guarantees both names are single-valued.
        let (hali, aqsa) = unsafe { (self.sahih(hali), self.sahih(aqsa)) };
        hali >= 0 && aqsa > 0 && hali < aqsa
    }
}

/// The specification's own name for an error code.
const fn ism_khata(khata: Adad) -> &'static str {
    match khata {
        GL_NO_ERROR => "GL_NO_ERROR",
        GL_INVALID_ENUM => "GL_INVALID_ENUM",
        GL_INVALID_VALUE => "GL_INVALID_VALUE",
        GL_INVALID_OPERATION => "GL_INVALID_OPERATION",
        GL_STACK_OVERFLOW => "GL_STACK_OVERFLOW",
        GL_STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
        GL_OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
        _ => "an error code this build does not name",
    }
}

// ---------------------------------------------------------------------------
// The state the draw disturbs, and the stacks it is saved on
// ---------------------------------------------------------------------------

/// Everything the overlay is about to change, pushed onto the pipeline's own
/// stacks.
///
/// The fixed-function analogue of a Direct3D state block, and the instrument a
/// core profile removed. Three stacks are used and each is measured before it
/// is pushed:
///
/// * the **server attribute stack**, which `GL_ALL_ATTRIB_BITS` fills with every
///   enable, every blend and depth and stencil setting, every material, light,
///   fog and texture environment, and the current texture binding;
/// * the **client attribute stack**, which `GL_CLIENT_ALL_ATTRIB_BITS` fills
///   with the pixel-store modes and the vertex array pointers;
/// * the **three matrix stacks**, which the attribute stack does not touch at
///   all and which are the state a 2D overlay must replace outright.
///
/// Each field records whether its push actually happened, so a failure part way
/// through unwinds exactly what it did and nothing more. A `glPopAttrib` with
/// no matching push pops whatever the *game* pushed, which is worse than
/// anything this guard exists to prevent.
#[derive(Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag records whether one specific push happened, and the unwind reads them one \
              at a time; folding them into a bitfield is what breaks the one-flag-per-push \
              correspondence that makes a partial failure unwind exactly what it did"
)]
struct HifzThabit {
    /// The matrix mode the game had current, restored last.
    namat_sabiq: Adad,
    /// Whether `glPushAttrib` succeeded.
    sifat: bool,
    /// Whether `glPushClientAttrib` succeeded.
    sifat_zabun: bool,
    /// Whether the texture matrix was pushed.
    nasij: bool,
    /// Whether the projection matrix was pushed.
    isqat: bool,
    /// Whether the modelview matrix was pushed.
    namudhaj: bool,
}

impl HifzThabit {
    /// Pushes the two attribute stacks and nothing else.
    ///
    /// Used by the atlas upload and the capture, which change pixel-store modes
    /// and a texture binding and no matrix at all. Pushing the matrix stacks
    /// for those would be three more chances to overflow a stack that is only
    /// required to be two deep, in exchange for saving state nothing touched.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when either stack is full, which is a
    /// skipped operation rather than a fault: nothing has been changed.
    fn sifat_faqat(dawall: &DawallThabit) -> Result<Self, KhataTabaqa> {
        Self::ihfaz(dawall, None)
    }

    /// Pushes both attribute stacks and all three matrix stacks.
    ///
    /// The projection is replaced with an orthographic one in surface pixels,
    /// origin top-left, y increasing downward — the convention every rectangle
    /// in this crate uses. `glOrtho(0, w, h, 0, …)` is what puts the origin at
    /// the top left; the same call with `0, h` swapped draws the overlay
    /// mirrored vertically about the middle of the screen, correctly shaped.
    ///
    /// # Errors
    ///
    /// As [`HifzThabit::sifat_faqat`], plus the three matrix stacks.
    fn kaamil(dawall: &DawallThabit, sath: WasfSath) -> Result<Self, KhataTabaqa> {
        Self::ihfaz(dawall, Some(sath))
    }

    fn ihfaz(dawall: &DawallThabit, sath: Option<WasfSath>) -> Result<Self, KhataTabaqa> {
        // SAFETY: `GL_MATRIX_MODE` is a single-valued state in the
        // specification's transformation table.
        let namat_sabiq = unsafe { dawall.sahih(GL_MATRIX_MODE) };
        let mut hifz = Self {
            namat_sabiq: ka_adad(namat_sabiq),
            sifat: false,
            sifat_zabun: false,
            nasij: false,
            isqat: false,
            namudhaj: false,
        };

        let mumtali = |hala: &'static str| KhataTabaqa::MawridFashil {
            mawrid: "the OpenGL attribute or matrix stack",
            sabab: format!(
                "the {hala} stack is already at its limit, so the overlay cannot save what it \
                 would change; the frame is skipped rather than drawn, because a push that \
                 overflows is followed by a pop that takes the game's own saved state instead"
            ),
        };

        // SAFETY: every name below is single-valued, which is the arity
        // `fiha_masaha` documents, and every call after a successful check is a
        // push the stack has room for.
        unsafe {
            if !dawall.fiha_masaha(GL_ATTRIB_STACK_DEPTH, GL_MAX_ATTRIB_STACK_DEPTH) {
                return Err(mumtali("server attribute"));
            }
            (dawall.push_attrib)(GL_ALL_ATTRIB_BITS);
            hifz.sifat = true;

            if !dawall.fiha_masaha(
                GL_CLIENT_ATTRIB_STACK_DEPTH,
                GL_MAX_CLIENT_ATTRIB_STACK_DEPTH,
            ) {
                hifz.ustud(dawall);
                return Err(mumtali("client attribute"));
            }
            (dawall.push_client_attrib)(GL_CLIENT_ALL_ATTRIB_BITS);
            hifz.sifat_zabun = true;

            let Some(sath) = sath else {
                return Ok(hifz);
            };

            for (namat, hali, aqsa, ism) in [
                (
                    GL_TEXTURE,
                    GL_TEXTURE_STACK_DEPTH,
                    GL_MAX_TEXTURE_STACK_DEPTH,
                    "texture matrix",
                ),
                (
                    GL_PROJECTION,
                    GL_PROJECTION_STACK_DEPTH,
                    GL_MAX_PROJECTION_STACK_DEPTH,
                    "projection matrix",
                ),
                (
                    GL_MODELVIEW,
                    GL_MODELVIEW_STACK_DEPTH,
                    GL_MAX_MODELVIEW_STACK_DEPTH,
                    "modelview matrix",
                ),
            ] {
                (dawall.matrix_mode)(namat);
                if !dawall.fiha_masaha(hali, aqsa) {
                    hifz.ustud(dawall);
                    return Err(mumtali(ism));
                }
                (dawall.push_matrix)();
                (dawall.load_identity)();
                match namat {
                    GL_TEXTURE => hifz.nasij = true,
                    GL_PROJECTION => {
                        (dawall.ortho)(
                            0.0,
                            f64::from(sath.ard.max(1)),
                            f64::from(sath.irtifa.max(1)),
                            0.0,
                            -1.0,
                            1.0,
                        );
                        hifz.isqat = true;
                    },
                    _ => hifz.namudhaj = true,
                }
            }
        }

        Ok(hifz)
    }

    /// Pops exactly what was pushed, in the reverse order.
    ///
    /// Each matrix is popped with its own mode current, because `glPopMatrix`
    /// acts on the stack the current mode names and popping the modelview stack
    /// while the projection mode is current takes the wrong matrix off.
    fn ustud(&self, dawall: &DawallThabit) {
        // SAFETY: every call below writes context state and touches no memory
        // this crate owns. Each pop is guarded by the flag its own push set, so
        // nothing pops a stack this guard did not push.
        unsafe {
            if self.namudhaj {
                (dawall.matrix_mode)(GL_MODELVIEW);
                (dawall.pop_matrix)();
            }
            if self.isqat {
                (dawall.matrix_mode)(GL_PROJECTION);
                (dawall.pop_matrix)();
            }
            if self.nasij {
                (dawall.matrix_mode)(GL_TEXTURE);
                (dawall.pop_matrix)();
            }
            (dawall.matrix_mode)(self.namat_sabiq);
            if self.sifat_zabun {
                (dawall.pop_client_attrib)();
            }
            if self.sifat {
                (dawall.pop_attrib)();
            }
        }
    }
}

/// A `GLint` read out of state, as the `GLenum` the setter wants.
///
/// Bit-preserving by construction rather than by an `as`, so the workspace's
/// cast lints stay denied and the round trip cannot silently become a
/// saturating one.
const fn ka_adad(qeema: Sahih) -> Adad {
    Adad::from_ne_bytes(qeema.to_ne_bytes())
}

/// A pixel count as a float, for the vertex coordinates.
const fn qeema_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and glyph coordinates are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// A count as the `GLsizei` the texture and read calls take.
fn qeema_sahih(qeema: u32) -> Option<Sahih> {
    Sahih::try_from(qeema).ok()
}

/// The next power of two at or above a dimension.
///
/// Required rather than defensive. OpenGL 1.1 has no non-power-of-two textures
/// — `GL_ARB_texture_non_power_of_two` is an OpenGL 2.0 feature — so an atlas
/// page of any other size is a `GL_INVALID_VALUE` and a texture that was never
/// created. The page's own dimensions are kept beside the texture's so the
/// batch's atlas coordinates can be scaled onto the larger image.
const fn quwwat_ithnayn(qeema: u32) -> u32 {
    let mut natija: u32 = 1;
    while natija < qeema && natija < 0x4000_0000 {
        natija = natija.saturating_mul(2);
    }
    natija
}

/// The context current on this thread, as an integer, or [`None`] where the
/// platform will not say.
///
/// The single most important thing this backend can ask, and the one failure
/// the modern backend never has to think about: `wglSwapBuffers` takes a device
/// context and does **not** require a rendering context to be current on the
/// calling thread. A game that renders on one thread and swaps on another
/// reaches this overlay with no context at all, every GL call it makes is
/// discarded, and it presents a frame with nothing on it — silently, sixty
/// times a second, forever.
#[cfg(windows)]
#[allow(
    clippy::unnecessary_wraps,
    reason = "the Windows answer is always known and the non-Windows one never is, and the two \
              must share a signature so the caller reads `Some(0)` as `no context` and `None` as \
              `this platform will not say`. An `allow` rather than an `expect` because the lint \
              fires on one platform's build only"
)]
fn siyaq_hali() -> Option<u64> {
    // SAFETY: `wglGetCurrentContext` takes nothing, writes nothing, and returns
    // the calling thread's current rendering context or a null handle.
    let maqbad = unsafe { windows::Win32::Graphics::OpenGL::wglGetCurrentContext() };
    Some(maqbad.0 as usize as u64)
}

/// The context current on this thread, where the platform will not say.
#[cfg(not(windows))]
const fn siyaq_hali() -> Option<u64> {
    // `glXGetCurrentContext` would answer this on X11 and `eglGetCurrentContext`
    // on EGL, and each lives in a different module from the other. Rather than
    // guess which one a process is using, the answer is "unknown" and the draw
    // proceeds — on those platforms the swap function takes the drawable the
    // context was made current against, so the failure this guards against on
    // Windows does not arise the same way.
    None
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// The fixed-function OpenGL overlay renderer.
///
/// ## Why there is no `Drop`
///
/// Deleting a texture requires a current context, and a `Drop` runs wherever
/// the value happens to be dropped — after a terminal fault, on whatever thread
/// unwound, very possibly with no context current or with the wrong one.
/// Deleting name `7` in the wrong context deletes somebody else's object. So
/// release is [`Khattaf::ahmil`], called deliberately on the render thread, and
/// dropping this value without calling it leaks one texture into a process that
/// is ending anyway. That is the better of the two failures, and it is the same
/// judgement [`crate::gl`] makes.
#[derive(Debug)]
pub struct KhattafGlThabit {
    dawall: DawallThabit,
    /// The atlas texture name, or zero.
    nasij: Adad,
    /// The atlas page's declared dimensions.
    qiyas_lawha: (u32, u32),
    /// The texture's real dimensions, padded to powers of two.
    qiyas_nasij: (u32, u32),
    /// `GL_MAX_TEXTURE_SIZE`, read once.
    aqsa_nasij: u32,
    /// The drawable size the platform layer reported, when it knows one.
    qiyas_mubarmaj: Option<(u32, u32)>,
    /// The context this backend's object names belong to.
    siyaq: u64,
    /// The surface the last [`Khattaf::hayyi`] was given.
    sath: Option<WasfSath>,
    /// Whether the missing-context refusal has already been recorded.
    nabbaha_siyaq: bool,
    /// Whether to ask the platform which context is current before drawing.
    ///
    /// On for a backend that resolved its own entry points out of a real
    /// module, which is every production path. Off for one built over a table a
    /// caller supplied, because a caller that hands over its own function
    /// pointers has already said where the calls go — asking the platform then
    /// would be asking about a context those pointers have nothing to do with.
    yafhas_siyaq: bool,
    /// What has happened here, for the diagnostics bundle.
    athar: Vec<String>,
}

impl KhattafGlThabit {
    /// Resolves the entry points out of the module the game is already using.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MaktabaMafquda`] when no OpenGL module can be found, or
    /// when one is found and does not export an OpenGL 1.1 entry point this
    /// backend calls — which means it is not an OpenGL implementation.
    pub fn jadeed() -> Result<Self, KhataTabaqa> {
        let wahda = WahdatGl::ijid()?;
        let dawall = DawallThabit::min_muhill(&wahda)?;
        Ok(Self::bi_dawall_maa(dawall, true))
    }

    /// Prepares a backend over an already-resolved table.
    ///
    /// Exists so the renderer can be exercised against a table a caller built,
    /// which is the same separation [`crate::wajiha::Khattaf`] draws by not
    /// owning the hook: a backend is testable exactly to the degree that it
    /// does not go looking for its own inputs.
    #[must_use]
    pub fn bi_dawall(dawall: DawallThabit) -> Self {
        Self::bi_dawall_maa(dawall, false)
    }

    /// Prepares a backend over a table, saying whether to trust the platform's
    /// answer about which context is current.
    fn bi_dawall_maa(dawall: DawallThabit, yafhas_siyaq: bool) -> Self {
        let athar = vec![format!(
            "OpenGL {}.{} (fixed function): {}",
            dawall.isdar.0, dawall.isdar.1, dawall.wasf
        )];
        Self {
            dawall,
            nasij: 0,
            qiyas_lawha: (0, 0),
            qiyas_nasij: (0, 0),
            aqsa_nasij: 0,
            qiyas_mubarmaj: None,
            siyaq: 0,
            sath: None,
            nabbaha_siyaq: false,
            yafhas_siyaq,
            athar,
        }
    }

    /// The entry points this backend resolved.
    #[must_use]
    pub const fn dawall(&self) -> &DawallThabit {
        &self.dawall
    }

    /// What has happened to this backend, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// Records the drawable size the platform layer knows and OpenGL does not.
    ///
    /// The viewport is the usual stand-in and is wrong precisely when it
    /// matters: a game that renders its world into a half-height viewport for a
    /// letterboxed cutscene has a viewport that is not the surface, and an
    /// overlay sized to it puts the subtitles in the middle of the screen.
    pub const fn hadith_qiyas(&mut self, ard: u32, irtifa: u32) {
        self.qiyas_mubarmaj = if ard == 0 || irtifa == 0 {
            None
        } else {
            Some((ard, irtifa))
        };
    }

    /// Records which context the object names belong to.
    ///
    /// A game that destroys its context and makes a new one invalidates every
    /// name this backend holds. The names are then **forgotten rather than
    /// deleted**: `glDeleteTextures` in the new context would delete whatever
    /// object happens to hold that name there, which is the game's, and the
    /// symptom is one of the game's own textures turning black for no reason
    /// anybody can trace.
    pub fn sajjil_siyaq(&mut self, muarrif: u64) {
        if muarrif == 0 || muarrif == self.siyaq {
            return;
        }
        if self.siyaq != 0 {
            self.athar.push(format!(
                "the GL context changed from {:#x} to {muarrif:#x}; the texture name this \
                 backend held belongs to a context that no longer exists and was forgotten \
                 rather than deleted",
                self.siyaq
            ));
        }
        self.siyaq = muarrif;
        self.nasij = 0;
        self.qiyas_lawha = (0, 0);
        self.qiyas_nasij = (0, 0);
        self.sath = None;
    }

    /// The atlas coordinate scale, since the texture is always a power of two.
    const fn nisbat_lawha(&self) -> (f32, f32) {
        let (lawha_ard, lawha_irtifa) = self.qiyas_lawha;
        let (nasij_ard, nasij_irtifa) = self.qiyas_nasij;
        if nasij_ard == 0 || nasij_irtifa == 0 {
            return (1.0, 1.0);
        }
        (
            qeema_f32(lawha_ard) / qeema_f32(nasij_ard),
            qeema_f32(lawha_irtifa) / qeema_f32(nasij_irtifa),
        )
    }

    /// Sets the pipeline and submits the batch in immediate mode.
    ///
    /// Every state written here is inside `GL_ALL_ATTRIB_BITS` or is a matrix,
    /// which is what makes [`HifzThabit`] complete. Adding a write that falls
    /// outside both is the one way this backend can corrupt a renderer.
    ///
    /// Quads are submitted in runs of the same kind because `glEnable` is
    /// illegal between `glBegin` and `glEnd`, and whether a quad samples the
    /// atlas is `GL_TEXTURE_2D` here rather than a per-vertex attribute. Runs
    /// preserve submission order, which is the layering — there is no depth
    /// buffer and a plate emitted after its glyphs would cover them.
    ///
    /// There is **no half-pixel offset**, unlike [`crate::d3d8`]. OpenGL's fill
    /// rule samples a pixel at its centre, so a quad from zero to `w` under
    /// `glOrtho(0, w, …)` covers pixels zero to `w - 1` exactly. Applying
    /// Direct3D's offset here would blur every glyph by half a pixel.
    fn arsil_dufaat(&self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if self.nasij == 0 && lawha.qitaat.iter().any(|qita| qita.khareeta.is_some()) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the batch samples an atlas that has not been uploaded".to_owned(),
            });
        }
        let (Some(manfath_ard), Some(manfath_irtifa)) =
            (qeema_sahih(lawha.sath.ard), qeema_sahih(lawha.sath.irtifa))
        else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "viewport",
                sabab: format!(
                    "a {}×{} surface does not fit a GLsizei",
                    lawha.sath.ard, lawha.sath.irtifa
                ),
            });
        };
        let (nisbat_u, nisbat_v) = self.nisbat_lawha();
        let dawall = &self.dawall;

        // SAFETY: every call below writes context state or submits a vertex.
        // None reads memory this crate owns and none takes a pointer. The
        // `glBegin`/`glEnd` pairs below are balanced on every path: the loop
        // body has no early return in it, which is why the fallible work — the
        // atlas check and the viewport conversion — is all above this block.
        unsafe {
            (dawall.viewport)(0, 0, manfath_ard, manfath_irtifa);
            for hala in [
                GL_DEPTH_TEST,
                GL_CULL_FACE,
                GL_SCISSOR_TEST,
                GL_LIGHTING,
                GL_FOG,
                GL_ALPHA_TEST,
                GL_STENCIL_TEST,
                GL_COLOR_MATERIAL,
                GL_DITHER,
                // Texture coordinate generation would replace the coordinates
                // the batch carries with ones derived from the vertex position,
                // which draws every glyph as a slice of whatever the atlas
                // happens to hold at that spot.
                GL_TEXTURE_GEN_S,
                GL_TEXTURE_GEN_T,
            ] {
                (dawall.disable)(hala);
            }
            (dawall.depth_mask)(GL_FALSE);
            (dawall.color_mask)(GL_TRUE, GL_TRUE, GL_TRUE, GL_TRUE);
            (dawall.shade_model)(GL_SMOOTH);
            // A game left in wireframe would otherwise draw the overlay's quads
            // as outlines, which is a real and frequently reported symptom of
            // overlays that save the polygon mode and forget to set it.
            (dawall.polygon_mode)(GL_FRONT_AND_BACK, GL_FILL);

            (dawall.enable)(GL_BLEND);
            (dawall.blend_func)(GL_ONE, GL_ONE_MINUS_SRC_ALPHA);
            (dawall.tex_envi)(GL_TEXTURE_ENV, GL_TEXTURE_ENV_MODE, GL_MODULATE);
            (dawall.bind_texture)(GL_TEXTURE_2D, self.nasij);

            let mut masturat_alaan: Option<bool> = None;
            let mut maftuh = false;
            for qita in &lawha.qitaat {
                let QitaRasm {
                    mawdi,
                    khareeta,
                    lawn,
                } = *qita;
                if mawdi.ard == 0 || mawdi.irtifa == 0 {
                    continue;
                }
                let masturat = khareeta.is_some();
                if masturat_alaan != Some(masturat) {
                    if maftuh {
                        (dawall.end)();
                        maftuh = false;
                    }
                    if masturat {
                        (dawall.enable)(GL_TEXTURE_2D);
                    } else {
                        (dawall.disable)(GL_TEXTURE_2D);
                    }
                    masturat_alaan = Some(masturat);
                }
                if !maftuh {
                    (dawall.begin)(GL_QUADS);
                    maftuh = true;
                }

                let [ahmar, akhdar, azraq, shaffaf] = crate::d3d8::lawn_marmuz(lawn);
                (dawall.color4f)(ahmar, akhdar, azraq, shaffaf);

                let yasar = qeema_f32(mawdi.yasar);
                let aala = qeema_f32(mawdi.aala);
                let yameen = yasar + qeema_f32(mawdi.ard);
                let asfal = aala + qeema_f32(mawdi.irtifa);
                let (u0, v0, u1, v1) = khareeta.map_or((0.0, 0.0, 0.0, 0.0), |khareeta| {
                    (
                        khareeta.yasar * nisbat_u,
                        khareeta.aala * nisbat_v,
                        (khareeta.yasar + khareeta.ard) * nisbat_u,
                        (khareeta.aala + khareeta.irtifa) * nisbat_v,
                    )
                });

                for (u, v, s, a) in [
                    (u0, v0, yasar, aala),
                    (u1, v0, yameen, aala),
                    (u1, v1, yameen, asfal),
                    (u0, v1, yasar, asfal),
                ] {
                    (dawall.tex_coord2f)(u, v);
                    (dawall.vertex2f)(s, a);
                }
            }
            if maftuh {
                (dawall.end)();
            }
        }
        Ok(())
    }

    /// Creates the atlas texture and writes the page into it, padded.
    fn arfa_nasij(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        let mubattan = (quwwat_ithnayn(ard), quwwat_ithnayn(irtifa));
        if self.aqsa_nasij != 0 && (mubattan.0 > self.aqsa_nasij || mubattan.1 > self.aqsa_nasij) {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas edge, rounded up to a power of two",
                qeema: u64::from(mubattan.0.max(mubattan.1)),
                saqf: u64::from(self.aqsa_nasij),
            });
        }
        let (Some(mubattan_ard), Some(mubattan_irtifa)) =
            (qeema_sahih(mubattan.0), qeema_sahih(mubattan.1))
        else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "a {}×{} texture does not fit a GLsizei",
                    mubattan.0, mubattan.1
                ),
            });
        };

        // The padded image is built on the CPU rather than uploaded in two
        // calls, because `glTexSubImage2D` is an OpenGL 1.1 entry point this
        // table would then also have to carry and because the padding has to be
        // *initialised* either way: a glyph whose rectangle touches the edge of
        // the page samples the texel beyond it under linear filtering, and an
        // uninitialised texel there is whatever the driver's allocator left.
        let masdar_ard = usize::try_from(ard).unwrap_or(0);
        let masdar_irtifa = usize::try_from(irtifa).unwrap_or(0);
        let hadaf_ard = usize::try_from(mubattan.0).unwrap_or(0);
        let hadaf_irtifa = usize::try_from(mubattan.1).unwrap_or(0);
        let mut mubattana = vec![0u8; hadaf_ard.saturating_mul(hadaf_irtifa).saturating_mul(4)];
        for satr in 0..masdar_irtifa.min(hadaf_irtifa) {
            let masdar = satr.saturating_mul(masdar_ard).saturating_mul(4);
            let hadaf = satr.saturating_mul(hadaf_ard).saturating_mul(4);
            let tul = masdar_ard.min(hadaf_ard).saturating_mul(4);
            let (Some(min), Some(ila)) = (
                bayt.get(masdar..masdar.saturating_add(tul)),
                mubattana.get_mut(hadaf..hadaf + tul),
            ) else {
                continue;
            };
            ila.copy_from_slice(min);
        }

        if self.nasij == 0 {
            let mut nasij: Adad = 0;
            // SAFETY: `nasij` is one live, aligned `GLuint` and the count of
            // one matches it exactly.
            unsafe { (self.dawall.gen_textures)(1, &raw mut nasij) };
            if nasij == 0 {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "glyph atlas texture",
                    sabab: "glGenTextures returned name zero, which means no context is current \
                            on this thread"
                        .to_owned(),
                });
            }
            self.nasij = nasij;
        }

        // `GL_CLAMP_TO_EDGE` arrived in OpenGL 1.2. On 1.1 the only clamp is
        // `GL_CLAMP`, which blends the border colour in at the edge texel — a
        // faint dark line along one side of every glyph that touches the edge
        // of its cell. The newer token is used where the context has it and the
        // older one where it does not, rather than passing an enum the driver
        // would reject with `GL_INVALID_ENUM`.
        let itar = if self.dawall.isdar.0 > 1 || self.dawall.isdar.1 >= 2 {
            GL_CLAMP_TO_EDGE
        } else {
            GL_CLAMP
        };

        // SAFETY: the texture name was created above or on an earlier upload
        // and has not been deleted. `glTexImage2D` reads exactly
        // `mubattan.0 * mubattan.1 * 4` bytes at an alignment of one from
        // `mubattana`, whose length is that product by construction.
        unsafe {
            (self.dawall.bind_texture)(GL_TEXTURE_2D, self.nasij);
            (self.dawall.pixel_storei)(GL_UNPACK_ALIGNMENT, 1);
            (self.dawall.tex_image2d)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA8,
                mubattan_ard,
                mubattan_irtifa,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                mubattana.as_ptr().cast::<c_void>(),
            );
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, itar);
            (self.dawall.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, itar);
        }

        self.qiyas_lawha = (ard, irtifa);
        self.qiyas_nasij = mubattan;
        Ok(())
    }

    /// Reads one rectangle of the presented frame back into memory.
    ///
    /// `glReadPixels` measures from the bottom-left of the framebuffer and
    /// every rectangle in this crate is measured from the top-left, so the y is
    /// converted **and** the rows are reversed. Both halves are needed:
    /// flipping without moving the origin reads the wrong band of the screen,
    /// and moving the origin without flipping reads the right band upside down.
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
            qeema_sahih(mintaqa.yasar),
            qeema_sahih(asfal),
            qeema_sahih(mintaqa.ard),
            qeema_sahih(mintaqa.irtifa),
        ) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region's pixel coordinates do not fit a GLint".to_owned(),
            });
        };

        let satr_bayt = usize::try_from(mintaqa.ard).unwrap_or(0).saturating_mul(4);
        let tul = satr_bayt.saturating_mul(usize::try_from(mintaqa.irtifa).unwrap_or(0));
        if tul == 0 {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the region has no pixels to read".to_owned(),
            });
        }

        let mut khaam = vec![0u8; tul];
        // SAFETY: the back buffer is selected immediately before the read, so
        // the rectangle is inside the framebuffer `sath` measured. `khaam` is
        // exactly `ard * irtifa * 4` writable bytes at an alignment of one,
        // which with `GL_RGBA`/`GL_UNSIGNED_BYTE` and a pack alignment of one
        // is exactly what the driver writes. Both pixel-store modes and the
        // read buffer are on the stacks the caller pushed.
        unsafe {
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
        }

        let mut maqlub = Vec::with_capacity(tul);
        for satr in khaam.chunks_exact(satr_bayt).rev() {
            maqlub.extend_from_slice(satr);
        }
        Ok(maqlub)
    }

    /// Deletes the atlas and forgets its name.
    fn atlif(&mut self) {
        if self.nasij != 0 {
            // SAFETY: the name was created by this backend in this context and
            // has not been deleted; the count matches the single-element array.
            unsafe { (self.dawall.delete_textures)(1, &raw const self.nasij) };
            self.nasij = 0;
        }
        self.qiyas_lawha = (0, 0);
        self.qiyas_nasij = (0, 0);
        self.sath = None;
    }

    /// Runs one piece of work with the game's state on the pipeline's stacks.
    ///
    /// The sequence is fixed and is the same discipline [`crate::gl`] keeps
    /// with a different instrument: drain the error queue so what is found
    /// afterwards is the overlay's own, push, do the work, pop **whether the
    /// work succeeded or not**, and only then ask whether the restore itself
    /// produced an error.
    ///
    /// Draining first is itself a state change and is done knowingly. OpenGL's
    /// error flags are cleared by reading them, so a game that called
    /// `glGetError` after its own swap would see nothing where the overlay
    /// consumed a flag. The alternative is worse in both directions: the
    /// overlay either disables itself over an error the game left behind, or it
    /// cannot tell its own errors from anyone's and the check is theatre.
    fn bi_hifz<T>(
        &mut self,
        matrices: Option<WasfSath>,
        radd: RaddFasad,
        amal: impl FnOnce(&mut Self) -> Result<T, KhataTabaqa>,
    ) -> Result<T, KhataTabaqa> {
        let sabiq = self.dawall.ifragh();
        if sabiq != GL_NO_ERROR {
            self.athar.push(format!(
                "{} was already set when the overlay was entered; it belongs to the game and was \
                 cleared so the overlay's own check means something",
                ism_khata(sabiq)
            ));
        }

        let hifz = match matrices {
            Some(sath) => HifzThabit::kaamil(&self.dawall, sath),
            None => HifzThabit::sifat_faqat(&self.dawall),
        }?;
        let natija = amal(self);
        hifz.ustud(&self.dawall);

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
                hala: "the fixed-function OpenGL attribute, client-attribute and matrix stacks",
                sabab,
            }),
            RaddFasad::Mawrid => Err(KhataTabaqa::MawridFashil {
                mawrid: "an OpenGL object",
                sabab,
            }),
            RaddFasad::Iltiqat => Err(KhataTabaqa::IltiqatFashil { sabab }),
        }
    }
}

/// What a GL error surviving the restore means for the overlay.
///
/// The distinction is [`Khattaf`]'s, not this module's: a failure that leaves
/// state consistent is retried next frame, and one that does not disables the
/// overlay for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaddFasad {
    /// Report [`KhataTabaqa::MawridFashil`] — the frame is skipped and retried.
    Mawrid,
    /// Report [`KhataTabaqa::HalaGhayrMustaada`] — the overlay stops for good.
    Hala,
    /// Report [`KhataTabaqa::IltiqatFashil`] — the read-back did not complete.
    Iltiqat,
}

impl Khattaf for KhattafGlThabit {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::OpenGlThabit
    }

    /// The surface, as far as a fixed-function context will admit to having one.
    ///
    /// The size comes from the platform layer when it has told us
    /// ([`KhattafGlThabit::hadith_qiyas`]) and from `GL_VIEWPORT` when it has
    /// not, which is the best OpenGL itself offers on any version.
    ///
    /// The format is [`SighatSath::Rgba8`] because the capture format in OpenGL
    /// is a property of the *request* rather than of the surface:
    /// `glReadPixels` is told what to produce and the driver converts. This
    /// backend asks for `GL_RGBA`/`GL_UNSIGNED_BYTE`, so RGBA8 is what comes
    /// back, including from a sixteen-bit visual.
    ///
    /// `sirgb` is always false, and unlike the modern backend it is not read
    /// from anywhere: `GL_FRAMEBUFFER_SRGB` arrived in OpenGL 3.0 and a
    /// fixed-function context cannot have it. Nothing will apply a transfer
    /// function to what the overlay writes, which is what that field means
    /// operationally, and [`crate::d3d8::lawn_marmuz`] applies it on the CPU
    /// instead.
    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        // SAFETY: `GL_VIEWPORT` is a four-valued state in the specification's
        // viewport table on every version of OpenGL.
        let [_, _, manfath_ard, manfath_irtifa] = unsafe { self.dawall.sahih_arbaa(GL_VIEWPORT) };
        let (ard, irtifa) = self.qiyas_mubarmaj.unwrap_or_else(|| {
            (
                u32::try_from(manfath_ard).unwrap_or(0),
                u32::try_from(manfath_irtifa).unwrap_or(0),
            )
        });
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the drawable measures {ard}×{irtifa}, which a window being resized and a \
                     context that is not current on this thread both produce"
                ),
            });
        }
        Ok(WasfSath {
            ard,
            irtifa,
            sigha: SighatSath::Rgba8,
            sirgb: false,
        })
    }

    /// Reads the one implementation limit this backend needs and records the
    /// surface.
    ///
    /// Nothing is created here. The atlas is created on upload and there is no
    /// program, no vertex array and no buffer to build — which is the whole
    /// difference between this backend and the modern one, stated as an empty
    /// function rather than as a paragraph.
    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        if sath.ard == 0 || sath.irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "a {}×{} surface has no pixels to draw on",
                    sath.ard, sath.irtifa
                ),
            });
        }
        if self.aqsa_nasij == 0 {
            // SAFETY: `GL_MAX_TEXTURE_SIZE` is a single-valued implementation
            // limit, and reading it changes nothing that would need saving.
            let khaam = unsafe { self.dawall.sahih(GL_MAX_TEXTURE_SIZE) };
            self.aqsa_nasij = u32::try_from(khaam).unwrap_or(0);
        }
        if self.sath != Some(sath) {
            self.athar
                .push(format!("surface {}×{}", sath.ard, sath.irtifa));
        }
        self.sath = Some(sath);
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("an atlas of {ard}×{irtifa} has no texels"),
            });
        }
        let Some(matlub) = u64::from(ard)
            .checked_mul(u64::from(irtifa))
            .and_then(|bikselat| bikselat.checked_mul(4))
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

        let bayt = bayt.to_vec();
        self.bi_hifz(None, RaddFasad::Mawrid, move |hadha| {
            hadha.arfa_nasij(&bayt, ard, irtifa)
        })
    }

    /// Pushes the game's state, draws, pops it, and audits the pop.
    ///
    /// The audit has no counterpart in Direct3D, where a failed state operation
    /// is a value the call site can look at. OpenGL sets a flag in a queue and
    /// returns nothing, so this is the one API where a restore that did not
    /// take is genuinely undetectable unless somebody asks — and if nobody
    /// asks, the game carries on rendering with the overlay's blend function,
    /// the overlay's texture environment or the overlay's projection, and every
    /// frame after it is wrong in a way that looks like the game's own bug.
    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if lawha.qitaat.len() > AQSA_QITAAT {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: format!(
                    "the batch carries {} quads, above this build's ceiling of {AQSA_QITAAT}; a \
                     batch this large is a fault in whatever built it rather than a frame worth \
                     drawing",
                    lawha.qitaat.len()
                ),
            });
        }
        if self.yafhas_siyaq && siyaq_hali() == Some(0) {
            // The one failure mode this backend has that the modern one does
            // not, and the reason it is caught here rather than left to produce
            // a blank frame: `wglSwapBuffers` does not require a rendering
            // context to be current on the calling thread, and every GL call
            // made without one is discarded without an error.
            if !self.nabbaha_siyaq {
                self.nabbaha_siyaq = true;
                self.athar.push(
                    "this game swaps buffers on a thread with no current OpenGL context, so the \
                     overlay has nothing to draw through; nothing was changed and no frame will \
                     carry Arabic until that changes"
                        .to_owned(),
                );
            }
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "the OpenGL context",
                sabab: "the buffer swap was reached on a thread with no current rendering \
                        context, so every call the overlay would make is discarded"
                    .to_owned(),
            });
        }

        let sath = lawha.sath;
        self.bi_hifz(Some(sath), RaddFasad::Hala, |hadha| {
            hadha.arsil_dufaat(lawha)
        })
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = Khattaf::sath(self)?;
        let wasf = || {
            format!(
                "{}×{} at {},{}",
                mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala
            )
        };
        let (Some(yameen), Some(asfal)) = (
            mintaqa.yasar.checked_add(mintaqa.ard),
            mintaqa.aala.checked_add(mintaqa.irtifa),
        ) else {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: wasf(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        };
        if mintaqa.ard == 0 || mintaqa.irtifa == 0 || yameen > sath.ard || asfal > sath.irtifa {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: wasf(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        }

        self.bi_hifz(None, RaddFasad::Iltiqat, |hadha| {
            hadha.iqra_bikselat(mintaqa, sath)
        })
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        if self.nasij == 0 {
            return Ok(());
        }
        let _ = self.dawall.ifragh();
        self.atlif();
        let baqi = self.dawall.ifragh();
        if baqi == GL_NO_ERROR {
            self.athar
                .push("released the overlay's glyph atlas".to_owned());
            return Ok(());
        }
        let sabab = format!(
            "{} was set while releasing the overlay's atlas, on {}",
            ism_khata(baqi),
            self.dawall.wasf
        );
        self.athar.push(sabab.clone());
        Err(KhataTabaqa::MawridFashil {
            mawrid: "the overlay's glyph atlas",
            sabab,
        })
    }
}

// ---------------------------------------------------------------------------
// Which of the two OpenGL backends this context is for
// ---------------------------------------------------------------------------

/// Chooses between the modern OpenGL backend and this one.
///
/// The module list cannot decide this and neither can the platform: every
/// OpenGL process on Windows loads `opengl32.dll` whether its context is a 4.6
/// core profile or a 1.1 compatibility one. What decides it is the context, and
/// the question it is asked here is the honest one — *what version does it
/// report* — rather than a probe for one symbol, because a driver that exports
/// `glCreateShader` from `opengl32.dll` while the current context is 1.1 would
/// answer that probe yes and then refuse every call.
///
/// A context reporting 3.0 or later is offered to [`crate::gl`] first, and this
/// backend takes it only if that one refuses — which happens on a 3.x
/// *compatibility* context whose driver did not expose a 3.3 core entry point.
/// A context below 3.0 comes straight here, because the modern backend needs
/// vertex array objects and those are 3.0.
///
/// **Must be called with the game's context current on this thread**, which
/// inside a buffer-swap hook it is on every platform that guarantees it at all.
///
/// # Errors
///
/// [`KhataTabaqa::MaktabaMafquda`] when no OpenGL module can be found, or when
/// both backends refuse — in which case the message carries both refusals,
/// because "OpenGL is not supported" without saying which entry point was
/// missing is unactionable in a bug report.
pub fn ikhtar() -> Result<Box<dyn Khattaf>, KhataTabaqa> {
    let wahda = WahdatGl::ijid()?;
    let dawall = DawallThabit::min_muhill(&wahda)?;

    if dawall.isdar.0 >= 3 {
        match crate::gl::KhattafGl::jadeed() {
            Ok(khattaf) => return Ok(Box::new(khattaf)),
            Err(khata) => {
                let mut thabit = KhattafGlThabit::bi_dawall_maa(dawall, true);
                thabit.athar.push(format!(
                    "the context reports OpenGL {}.{} and the modern backend still refused it, so \
                     the fixed-function path was taken: {khata}",
                    thabit.dawall.isdar.0, thabit.dawall.isdar.1
                ));
                return Ok(Box::new(thabit));
            },
        }
    }

    Ok(Box::new(KhattafGlThabit::bi_dawall_maa(dawall, true)))
}

// ---------------------------------------------------------------------------
// What can be decided before the overlay draws
// ---------------------------------------------------------------------------

/// What the overlay will be able to do on a fixed-function context here.
///
/// This report gets more precise the later it is called, and says which of its
/// findings needed a context. Called from the bootstrap, before a frame, it can
/// establish that an OpenGL module is present and exports every 1.1 entry point
/// this backend needs. Called from inside a buffer swap it can also establish
/// the two facts that actually stop this backend — a thread with no current
/// context, and a colour-index pixel format — and those are stated as unknown
/// rather than as absent when there is no context to ask.
///
/// Two kinds of "unknown", and they are reported differently. No context on
/// this thread *yet* is the expected state before a game draws; the report says
/// so and the verdict is not narrowed, because the first frame settles it. A
/// context that **is** current and will not describe its pixel format is a
/// question that was asked and not answered, and since the answer could be
/// the colour-index refusal, the verdict is [`crate::qudra::HukmQudra::Majhula`]
/// with the call that refused named — never "supported" over a format nothing
/// read.
#[must_use]
pub fn qudra() -> crate::qudra::QudratTarkeeb {
    use crate::qudra::{MilShasha, QudratTarkeeb, SababQudra};

    let mut taqreer = QudratTarkeeb::jadeeda(WajihatRusum::OpenGlThabit, MilShasha::KhilalAlJihaz);

    let wahda = match WahdatGl::ijid() {
        Ok(wahda) => wahda,
        Err(khata) => {
            return taqreer.maa(SababQudra::mustaheela(
                "لا توجد مكتبة أوبن‌جي‌إل محمّلة في هذه اللعبة، فلا يوجد ما تُركَّب عليه الطبقة.",
                format!("no OpenGL module is loaded in this process: {khata}"),
            ));
        },
    };

    match DawallThabit::min_muhill(&wahda) {
        Ok(dawall) => {
            taqreer = taqreer.maa(SababQudra::kamila(
                format!(
                    "تحقّقت الطبقة من {} نقطة دخول ثابتة في {}.",
                    DawallThabit::adad_dawall(),
                    wahda.ism()
                ),
                format!(
                    "all {} fixed-function entry points resolved out of {}",
                    DawallThabit::adad_dawall(),
                    wahda.ism()
                ),
            ));
            if dawall.isdar.0 >= 3 {
                taqreer = taqreer.maa(SababQudra::kamila(
                    format!(
                        "يبلّغ السياق عن أوبن‌جي‌إل {}.{}، وستُجرَّب الواجهة الحديثة أوّلًا.",
                        dawall.isdar.0, dawall.isdar.1
                    ),
                    format!(
                        "the context reports OpenGL {}.{}, so the modern backend is tried first \
                         and this one is the fallback",
                        dawall.isdar.0, dawall.isdar.1
                    ),
                ));
            }
        },
        Err(khata) => {
            return taqreer.maa(SababQudra::mustaheela(
                "المكتبة الموجودة لا تصدّر إحدى نقاط دخول أوبن‌جي‌إل ١٫١ التي تحتاجها الطبقة.",
                format!("the module found does not export an OpenGL 1.1 entry point: {khata}"),
            ));
        },
    }

    qudrat_siyaq(taqreer)
}

/// Adds the findings that need a current context, or says they are unknown.
#[cfg(windows)]
fn qudrat_siyaq(taqreer: crate::qudra::QudratTarkeeb) -> crate::qudra::QudratTarkeeb {
    use crate::qudra::SababQudra;
    use windows::Win32::Graphics::OpenGL::{
        DescribePixelFormat, GetPixelFormat, PFD_TYPE_COLORINDEX, PIXELFORMATDESCRIPTOR,
        wglGetCurrentDC,
    };

    if siyaq_hali() == Some(0) {
        return taqreer.maa(SababQudra::kamila(
            "لا يوجد سياق أوبن‌جي‌إل حالي على هذا الخيط الآن، وهو المتوقَّع قبل بدء اللعبة \
             بالرسم. تُحسَم الحالة التي تمنع الرسم فعلًا — تبديل المخزن على خيط بلا سياق — عند \
             أوّل إطار وتُعلَن حينها.",
            "no OpenGL context is current on this thread right now, which is expected before the \
             game starts drawing; the condition that actually stops this backend — a buffer swap \
             reached on a thread with no context — is decided at the first frame and reported \
             then",
        ));
    }

    // A context is current, so each of the three calls below is owed an answer.
    // One that refuses leaves the colour-index question — the one that can stop
    // this backend outright — unasked, and the report says exactly that.
    //
    // SAFETY: `wglGetCurrentDC` takes nothing and returns the device context
    // the current rendering context was made current against, or a null handle.
    let hdc = unsafe { wglGetCurrentDC() };
    if hdc.is_invalid() {
        return taqreer.maa(SababQudra::majhula(
            "يوجد سياق أوبن‌جي‌إل حالي على هذا الخيط، لكن wglGetCurrentDC لم يُرجع سياق جهاز، \
             فلم يتسنَّ سؤاله عن صيغة البكسل. لا يُعرف بعدُ إن كانت اللعبة تعرض بألوان مفهرسة.",
            "an OpenGL context is current on this thread, but wglGetCurrentDC returned no device \
             context, so the pixel format could not be asked for. Whether this game presents \
             through a colour-index format — the one thing that stops this backend — is not \
             known",
        ));
    }
    // SAFETY: `hdc` is the live device context from the call above.
    let fahras = unsafe { GetPixelFormat(hdc) };
    if fahras <= 0 {
        return taqreer.maa(SababQudra::majhula(
            format!(
                "أرجع GetPixelFormat القيمة {fahras} لسياق الجهاز الحالي، فلا فهرس صيغة يُسأل \
                 عنه. لا يُعرف بعدُ إن كانت اللعبة تعرض بألوان مفهرسة."
            ),
            format!(
                "GetPixelFormat returned {fahras} for the current device context, so there is no \
                 format index to describe. Whether this game presents through a colour-index \
                 format is not known"
            ),
        ));
    }

    let mut wasf = PIXELFORMATDESCRIPTOR::default();
    let hajm = u16::try_from(size_of::<PIXELFORMATDESCRIPTOR>()).unwrap_or(0);
    wasf.nSize = hajm;
    wasf.nVersion = 1;
    // SAFETY: `hdc` is live, `fahras` is a format index it reported, the byte
    // count is this structure's own size, and the out-pointer addresses a live
    // local of exactly that type.
    let natija = unsafe { DescribePixelFormat(hdc, fahras, u32::from(hajm), Some(&raw mut wasf)) };
    if natija == 0 {
        return taqreer.maa(SababQudra::majhula(
            format!(
                "رفض DescribePixelFormat وصف صيغة البكسل رقم {fahras} لسياق الجهاز الحالي. لا \
                 يُعرف بعدُ إن كانت اللعبة تعرض بألوان مفهرسة."
            ),
            format!(
                "DescribePixelFormat would not describe pixel format {fahras} for the current \
                 device context. Whether this game presents through a colour-index format is \
                 not known"
            ),
        ));
    }

    if wasf.iPixelType == PFD_TYPE_COLORINDEX {
        return taqreer.maa(SababQudra::mustaheela(
            "تعرض هذه اللعبة بصيغة ألوان مفهرسة (لوحة ٢٥٦ لونًا)، ولا سبيل لرسم أطلس شفّاف \
             بأربع قنوات فوقها.",
            "this game presents through a colour-index pixel format — a 256-entry palette — and \
             there is no way to draw a four-channel premultiplied atlas over one",
        ));
    }
    taqreer.maa(SababQudra::kamila(
        "صيغة البكسل التي تعرض بها اللعبة صيغة ألوان مباشرة، وهي ما تحتاجه الطبقة.",
        "the game presents through an RGBA pixel format, which is what the overlay needs",
    ))
}

/// Adds the findings that need a current context, where the platform has no way
/// to ask for them.
#[cfg(not(windows))]
fn qudrat_siyaq(taqreer: crate::qudra::QudratTarkeeb) -> crate::qudra::QudratTarkeeb {
    use crate::qudra::SababQudra;

    taqreer.maa(SababQudra::kamila(
        "لا تُتاح على هذه المنصة معرفة صيغة البكسل التي أنشئ بها السياق قبل الرسم، وتُحسم عند \
         أوّل إطار.",
        "this platform offers no way to ask which pixel format the context was created with \
         before drawing; it is decided at the first frame",
    ))
}

// ---------------------------------------------------------------------------
// Proving the save and the restore round-trip, without a driver
// ---------------------------------------------------------------------------

#[cfg(test)]
#[expect(
    clippy::panic,
    reason = "a test reports failure by panicking; the lint is written for library code"
)]
mod ikhtibar {
    use core::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    /// The stub table is global, so the tests take turns.
    static DAWR: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    /// Every entry point the backend called, in order.
    static SIJILL: parking_lot::Mutex<Vec<String>> = parking_lot::Mutex::new(Vec::new());
    /// Whether the stub context reports a full projection matrix stack.
    static ISQAT_MUMTALI: AtomicBool = AtomicBool::new(false);

    fn sajjil(satr: impl Into<String>) {
        SIJILL.lock().push(satr.into());
    }

    fn sijill() -> Vec<String> {
        SIJILL.lock().clone()
    }

    /// A `GLenum` as the `GLint` `glGetIntegerv` reports it as.
    const fn ka_sahih(qeema: Adad) -> Sahih {
        Sahih::from_ne_bytes(qeema.to_ne_bytes())
    }

    unsafe extern "system" fn get_error() -> Adad {
        GL_NO_ERROR
    }

    unsafe extern "system" fn get_string(_mawdu: Adad) -> *const u8 {
        core::ptr::null()
    }

    unsafe extern "system" fn get_integerv(mawdu: Adad, qeema: *mut Sahih) {
        let mumtali = ISQAT_MUMTALI.load(Ordering::Acquire);
        let qiyam: &[Sahih] = match mawdu {
            GL_VIEWPORT => &[0, 0, 1280, 720],
            GL_MATRIX_MODE => &[ka_sahih(GL_MODELVIEW)],
            GL_MAX_ATTRIB_STACK_DEPTH | GL_MAX_CLIENT_ATTRIB_STACK_DEPTH => &[16],
            GL_MAX_MODELVIEW_STACK_DEPTH => &[32],
            GL_MAX_PROJECTION_STACK_DEPTH | GL_MAX_TEXTURE_STACK_DEPTH => &[2],
            GL_PROJECTION_STACK_DEPTH => {
                if mumtali {
                    &[2]
                } else {
                    &[0]
                }
            },
            GL_MAX_TEXTURE_SIZE => &[2048],
            _ => &[0],
        };
        for (fahras, qanat) in qiyam.iter().enumerate() {
            // SAFETY: the caller passes an array with room for the arity its
            // own name has, and this stub answers exactly that arity.
            unsafe { qeema.add(fahras).write(*qanat) };
        }
    }

    unsafe extern "system" fn enable(hala: Adad) {
        sajjil(format!("enable:{hala:#06x}"));
    }
    unsafe extern "system" fn disable(hala: Adad) {
        sajjil(format!("disable:{hala:#06x}"));
    }
    unsafe extern "system" fn push_attrib(_qina: Adad) {
        sajjil("push_attrib");
    }
    unsafe extern "system" fn pop_attrib() {
        sajjil("pop_attrib");
    }
    unsafe extern "system" fn push_client_attrib(_qina: Adad) {
        sajjil("push_client_attrib");
    }
    unsafe extern "system" fn pop_client_attrib() {
        sajjil("pop_client_attrib");
    }
    unsafe extern "system" fn matrix_mode(namat: Adad) {
        sajjil(format!("matrix_mode:{namat:#06x}"));
    }
    unsafe extern "system" fn push_matrix() {
        sajjil("push_matrix");
    }
    unsafe extern "system" fn pop_matrix() {
        sajjil("pop_matrix");
    }
    unsafe extern "system" fn load_identity() {
        sajjil("load_identity");
    }
    unsafe extern "system" fn ortho(
        _yasar: Muda,
        _yameen: Muda,
        _asfal: Muda,
        _aala: Muda,
        _adna: Muda,
        _aqsa: Muda,
    ) {
        sajjil("ortho");
    }
    unsafe extern "system" fn viewport(_s: Sahih, _a: Sahih, _ard: Sahih, _irtifa: Sahih) {
        sajjil("viewport");
    }
    unsafe extern "system" fn blend_func(_masdar: Adad, _hadaf: Adad) {
        sajjil("blend_func");
    }
    unsafe extern "system" fn depth_mask(_qina: Mantiqi) {
        sajjil("depth_mask");
    }
    unsafe extern "system" fn color_mask(_r: Mantiqi, _g: Mantiqi, _b: Mantiqi, _a: Mantiqi) {
        sajjil("color_mask");
    }
    unsafe extern "system" fn shade_model(_namat: Adad) {
        sajjil("shade_model");
    }
    unsafe extern "system" fn polygon_mode(_wajh: Adad, _namat: Adad) {
        sajjil("polygon_mode");
    }
    unsafe extern "system" fn gen_textures(_adad: Sahih, asma: *mut Adad) {
        sajjil("gen_textures");
        // SAFETY: the caller passes one live `GLuint`.
        unsafe { asma.write(7) };
    }
    unsafe extern "system" fn bind_texture(_hadaf: Adad, ism: Adad) {
        sajjil(format!("bind_texture:{ism}"));
    }
    unsafe extern "system" fn delete_textures(_adad: Sahih, _asma: *const Adad) {
        sajjil("delete_textures");
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "the arity is `glTexImage2D`'s own and cannot differ from it; `allow` rather \
                  than `expect` because whether the lint fires on an `extern` declaration has \
                  differed between targets"
    )]
    unsafe extern "system" fn tex_image2d(
        _hadaf: Adad,
        _mustawa: Sahih,
        _dakhili: Sahih,
        _ard: Sahih,
        _irtifa: Sahih,
        _itar: Sahih,
        _sigha: Adad,
        _naw: Adad,
        _bayt: *const c_void,
    ) {
        sajjil("tex_image2d");
    }
    unsafe extern "system" fn tex_parameteri(_hadaf: Adad, ism: Adad, _qeema: Sahih) {
        sajjil(format!("tex_parameteri:{ism:#06x}"));
    }
    unsafe extern "system" fn tex_envi(_hadaf: Adad, _ism: Adad, _qeema: Sahih) {
        sajjil("tex_envi");
    }
    unsafe extern "system" fn pixel_storei(_ism: Adad, _qeema: Sahih) {
        sajjil("pixel_storei");
    }
    unsafe extern "system" fn read_pixels(
        _s: Sahih,
        _a: Sahih,
        _ard: Sahih,
        _irtifa: Sahih,
        _sigha: Adad,
        _naw: Adad,
        _bayt: *mut c_void,
    ) {
        sajjil("read_pixels");
    }
    unsafe extern "system" fn read_buffer(_mahfaza: Adad) {
        sajjil("read_buffer");
    }
    unsafe extern "system" fn begin(_namat: Adad) {
        sajjil("begin");
    }
    unsafe extern "system" fn end() {
        sajjil("end");
    }
    unsafe extern "system" fn color4f(_r: f32, _g: f32, _b: f32, _a: f32) {
        sajjil("color4f");
    }
    unsafe extern "system" fn tex_coord2f(_u: f32, _v: f32) {
        sajjil("tex_coord2f");
    }
    unsafe extern "system" fn vertex2f(_s: f32, _a: f32) {
        sajjil("vertex2f");
    }

    /// A table of stubs that answer the way OpenGL 1.1 says a context must.
    #[derive(Debug)]
    struct MuhillIkhtibar;

    impl MuhillGl for MuhillIkhtibar {
        fn unwan(&self, ramz: &str) -> Option<*const c_void> {
            let unwan: *const () = match ramz {
                "glGetError" => get_error as *const (),
                "glGetString" => get_string as *const (),
                "glGetIntegerv" => get_integerv as *const (),
                "glEnable" => enable as *const (),
                "glDisable" => disable as *const (),
                "glPushAttrib" => push_attrib as *const (),
                "glPopAttrib" => pop_attrib as *const (),
                "glPushClientAttrib" => push_client_attrib as *const (),
                "glPopClientAttrib" => pop_client_attrib as *const (),
                "glMatrixMode" => matrix_mode as *const (),
                "glPushMatrix" => push_matrix as *const (),
                "glPopMatrix" => pop_matrix as *const (),
                "glLoadIdentity" => load_identity as *const (),
                "glOrtho" => ortho as *const (),
                "glViewport" => viewport as *const (),
                "glBlendFunc" => blend_func as *const (),
                "glDepthMask" => depth_mask as *const (),
                "glColorMask" => color_mask as *const (),
                "glShadeModel" => shade_model as *const (),
                "glPolygonMode" => polygon_mode as *const (),
                "glGenTextures" => gen_textures as *const (),
                "glBindTexture" => bind_texture as *const (),
                "glDeleteTextures" => delete_textures as *const (),
                "glTexImage2D" => tex_image2d as *const (),
                "glTexParameteri" => tex_parameteri as *const (),
                "glTexEnvi" => tex_envi as *const (),
                "glPixelStorei" => pixel_storei as *const (),
                "glReadPixels" => read_pixels as *const (),
                "glReadBuffer" => read_buffer as *const (),
                "glBegin" => begin as *const (),
                "glEnd" => end as *const (),
                "glColor4f" => color4f as *const (),
                "glTexCoord2f" => tex_coord2f as *const (),
                "glVertex2f" => vertex2f as *const (),
                _ => return None,
            };
            Some(unwan.cast::<c_void>())
        }

        #[allow(
            clippy::unnecessary_literal_bound,
            reason = "the signature is `MuhillGl::ism`'s and an impl may not widen it; the real \
                      implementation returns a field, which is what the trait's lifetime is for"
        )]
        fn ism(&self) -> &str {
            "a stub OpenGL 1.1 table"
        }
    }

    /// A backend over the stub table, with the log cleared.
    fn khattaf() -> KhattafGlThabit {
        SIJILL.lock().clear();
        let dawall = match DawallThabit::min_muhill(&MuhillIkhtibar) {
            Ok(dawall) => dawall,
            Err(khata) => panic!("the stub table would not resolve: {khata}"),
        };
        KhattafGlThabit::bi_dawall(dawall)
    }

    /// One batch: a plate, two glyphs, a plate.
    fn dufaa(sath: WasfSath) -> LawhatRasm {
        use crate::wajiha::MustatilNisbi;
        let lawh = QitaRasm {
            mawdi: MustatilBiksel {
                yasar: 10,
                aala: 20,
                ard: 100,
                irtifa: 16,
            },
            khareeta: None,
            lawn: [1.0, 1.0, 1.0, 1.0],
        };
        let shakl = QitaRasm {
            khareeta: Some(MustatilNisbi {
                yasar: 0.25,
                aala: 0.5,
                ard: 0.25,
                irtifa: 0.25,
            }),
            ..lawh
        };
        LawhatRasm {
            qitaat: vec![lawh, shakl, shakl, lawh],
            sath,
        }
    }

    /// How many times a token appears in the log.
    fn adad(sijill: &[String], token: &str) -> usize {
        sijill.iter().filter(|satr| satr.as_str() == token).count()
    }

    /// Every push the draw makes is matched by exactly one pop, in reverse.
    ///
    /// This is the rung the brief calls "state capture and restore round-trips",
    /// executed rather than inferred. The stub is not a driver — it answers the
    /// state queries the way the specification says a context must and records
    /// what it was told — so what is proved is the *sequence*, which is the part
    /// that corrupts a renderer when it is wrong.
    #[test]
    fn hifz_yadfa_wa_yastarid_bil_tarteeb() {
        let _dawr = DAWR.lock();
        ISQAT_MUMTALI.store(false, Ordering::Release);

        let mut khattaf = khattaf();
        let sath = WasfSath {
            ard: 1280,
            irtifa: 720,
            sigha: SighatSath::Rgba8,
            sirgb: false,
        };
        if let Err(khata) = khattaf.hayyi(sath) {
            panic!("the backend would not initialise: {khata}");
        }
        if let Err(khata) = khattaf.arfa_lawha(&[0u8; 16], 2, 2) {
            panic!("the atlas would not upload: {khata}");
        }
        SIJILL.lock().clear();
        if let Err(khata) = khattaf.irsim(&dufaa(sath)) {
            panic!("the draw refused: {khata}");
        }

        let sijill = sijill();
        assert_eq!(
            adad(&sijill, "push_attrib"),
            1,
            "the server attribute stack was not pushed"
        );
        assert_eq!(
            adad(&sijill, "pop_attrib"),
            1,
            "the server attribute stack was not popped"
        );
        assert_eq!(
            adad(&sijill, "push_client_attrib"),
            1,
            "the client stack was not pushed"
        );
        assert_eq!(
            adad(&sijill, "pop_client_attrib"),
            1,
            "the client stack was not popped"
        );
        assert_eq!(
            adad(&sijill, "push_matrix"),
            3,
            "three matrix stacks must each be pushed once"
        );
        assert_eq!(
            adad(&sijill, "pop_matrix"),
            3,
            "three matrix stacks must each be popped once"
        );
        assert_eq!(
            adad(&sijill, "ortho"),
            1,
            "the projection must be replaced exactly once"
        );

        let bidaya: Vec<&str> = sijill.iter().take(12).map(String::as_str).collect();
        assert_eq!(
            bidaya,
            vec![
                "push_attrib",
                "push_client_attrib",
                "matrix_mode:0x1702",
                "push_matrix",
                "load_identity",
                "matrix_mode:0x1701",
                "push_matrix",
                "load_identity",
                "ortho",
                "matrix_mode:0x1700",
                "push_matrix",
                "load_identity",
            ],
            "the save did not happen in the order the restore reverses"
        );

        let nihaya: Vec<&str> = sijill
            .iter()
            .rev()
            .take(9)
            .rev()
            .map(String::as_str)
            .collect();
        assert_eq!(
            nihaya,
            vec![
                "matrix_mode:0x1700",
                "pop_matrix",
                "matrix_mode:0x1701",
                "pop_matrix",
                "matrix_mode:0x1702",
                "pop_matrix",
                // The mode the stub reported at entry, restored last of the
                // matrix work and before either attribute stack.
                "matrix_mode:0x1700",
                "pop_client_attrib",
                "pop_attrib",
            ],
            "the restore did not reverse the save"
        );
    }

    /// A full projection stack is a skipped frame, not an overflowed stack.
    ///
    /// The projection stack is required to be only two deep. A game that has
    /// already pushed one leaves one slot, and a `glPushMatrix` past the end
    /// sets `GL_STACK_OVERFLOW`, does nothing, and lets the matching
    /// `glPopMatrix` take the game's own matrix instead. This test is what says
    /// the overlay measures first — and that what it had already pushed is
    /// unwound rather than left on the stack.
    #[test]
    fn makdas_al_isqat_al_mumtali_yatruk_al_itar() {
        let _dawr = DAWR.lock();
        ISQAT_MUMTALI.store(true, Ordering::Release);

        let mut khattaf = khattaf();
        let sath = WasfSath {
            ard: 1280,
            irtifa: 720,
            sigha: SighatSath::Rgba8,
            sirgb: false,
        };
        let _ = khattaf.hayyi(sath);
        SIJILL.lock().clear();

        let natija = khattaf.irsim(&dufaa(sath));
        ISQAT_MUMTALI.store(false, Ordering::Release);

        let Err(khata) = natija else {
            panic!("the draw went ahead with no room on the projection matrix stack");
        };
        assert!(
            matches!(khata, KhataTabaqa::MawridFashil { .. }),
            "a full stack is a skipped frame, not a fault, and was reported as {khata}"
        );

        let sijill = sijill();
        assert_eq!(
            adad(&sijill, "begin"),
            0,
            "the overlay drew after refusing to save state"
        );
        assert_eq!(
            adad(&sijill, "push_matrix"),
            1,
            "only the texture matrix should have been pushed before the refusal"
        );
        assert_eq!(
            adad(&sijill, "pop_matrix"),
            1,
            "the texture matrix that was pushed before the refusal was not popped back"
        );
        assert_eq!(
            adad(&sijill, "push_attrib"),
            adad(&sijill, "pop_attrib"),
            "attribs unbalanced"
        );
        assert_eq!(
            adad(&sijill, "push_client_attrib"),
            adad(&sijill, "pop_client_attrib"),
            "client attribs unbalanced"
        );
    }

    /// Plates and glyphs go out in runs, and the runs follow submission order.
    #[test]
    fn dufaat_tatba_al_alwah() {
        let _dawr = DAWR.lock();
        ISQAT_MUMTALI.store(false, Ordering::Release);

        let mut khattaf = khattaf();
        let sath = WasfSath {
            ard: 1280,
            irtifa: 720,
            sigha: SighatSath::Rgba8,
            sirgb: false,
        };
        let _ = khattaf.hayyi(sath);
        if let Err(khata) = khattaf.arfa_lawha(&[0u8; 16], 2, 2) {
            panic!("the atlas would not upload: {khata}");
        }
        SIJILL.lock().clear();
        if let Err(khata) = khattaf.irsim(&dufaa(sath)) {
            panic!("the draw refused: {khata}");
        }

        let sijill = sijill();
        assert_eq!(
            adad(&sijill, "begin"),
            3,
            "plate, two glyphs, plate is three runs"
        );
        assert_eq!(adad(&sijill, "end"), 3, "every glBegin must be closed");
        assert_eq!(adad(&sijill, "color4f"), 4, "one colour per quad");
        assert_eq!(adad(&sijill, "vertex2f"), 16, "four quads of four vertices");

        let takhattut: Vec<&str> = sijill
            .iter()
            .filter(|satr| {
                satr.as_str() == "enable:0x0de1"
                    || satr.as_str() == "disable:0x0de1"
                    || satr.as_str() == "begin"
            })
            .map(String::as_str)
            .collect();
        assert_eq!(
            takhattut,
            vec![
                "disable:0x0de1",
                "begin",
                "enable:0x0de1",
                "begin",
                "disable:0x0de1",
                "begin",
            ],
            "texturing must be switched outside glBegin and follow the plate boundaries"
        );
    }

    /// The atlas is padded to a power of two and the coordinates follow it.
    #[test]
    fn lawha_tubattan_ila_quwwat_ithnayn() {
        assert_eq!(quwwat_ithnayn(1), 1);
        assert_eq!(quwwat_ithnayn(2), 2);
        assert_eq!(quwwat_ithnayn(3), 4);
        assert_eq!(quwwat_ithnayn(1024), 1024);
        assert_eq!(quwwat_ithnayn(1025), 2048);

        let _dawr = DAWR.lock();
        let mut khattaf = khattaf();
        let sath = WasfSath {
            ard: 640,
            irtifa: 480,
            sigha: SighatSath::Rgba8,
            sirgb: false,
        };
        let _ = khattaf.hayyi(sath);
        // A three-by-three page: OpenGL 1.1 will not take it and the backend
        // uploads a four-by-four texture with the page in its corner.
        if let Err(khata) = khattaf.arfa_lawha(&[0u8; 36], 3, 3) {
            panic!("a non-power-of-two page must be padded, not refused: {khata}");
        }
        assert_eq!(
            khattaf.qiyas_lawha,
            (3, 3),
            "the page's own size must be remembered"
        );
        assert_eq!(
            khattaf.qiyas_nasij,
            (4, 4),
            "the texture must be the next power of two"
        );
        let (u, v) = khattaf.nisbat_lawha();
        assert!(
            (u - 0.75).abs() < f32::EPSILON && (v - 0.75).abs() < f32::EPSILON,
            "a three-of-four page must scale its coordinates by three quarters, and scaled by \
             {u} and {v}"
        );
    }
}
