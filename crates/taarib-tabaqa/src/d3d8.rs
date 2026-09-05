//! خطّاف دايركت٣د ٨ — the overlay drawn through a fixed-function device from 2001.
//!
//! Direct3D 8 is not a smaller Direct3D 9 and this backend is not a smaller
//! [`crate::d3d11`]. Three differences decide the whole design and each of them
//! is the kind of thing that looks like a detail until it corrupts somebody's
//! frame.
//!
//! **Presentation is reached through the device, not through a swap chain.**
//! There is no DXGI here and no `IDXGISwapChain` to hook. `Present` is method
//! fifteen of `IDirect3DDevice8`, the game calls it on the device it created,
//! and that is the one slot this backend needs. `Reset` — method fourteen — is
//! hooked as well, for the same reason [`crate::d3d11`] needs `ResizeBuffers`:
//! a state block created before a reset is invalid after it, and the only
//! moment it can be released is before the call reaches the runtime.
//!
//! **State is saved with a state block, never by reading it back.** D3D8's
//! `Get*` state queries are documented to fail on a device created with
//! `D3DCREATE_PUREDEVICE`, and a pure device is what a game asks for when it
//! wants the driver to stop tracking state on its behalf. An overlay that read
//! the render states back would work on most machines and silently restore
//! garbage on the fastest ones. `CreateStateBlock(D3DSBT_ALL)` once, then
//! `CaptureStateBlock` and `ApplyStateBlock` around every draw, is correct on
//! both — and it covers render states, texture stage states, transforms, the
//! viewport, the stream sources, the textures and the shader handles in one
//! call, which is more than any hand-written list would have kept in step.
//!
//! What a state block does **not** cover is the render target and the depth
//! stencil surface, so those two are saved and restored by hand around it.
//!
//! **There is no shader and therefore no transfer function.** [`crate::d3d11`]
//! encodes sRGB in its pixel shader when the target format will not; the fixed
//! function pipeline has nowhere to put that arithmetic. So the conversion is
//! made on the CPU, once per quad, in [`lawn_d3d`] — and the blend that follows
//! happens in the framebuffer's own encoding, which is exactly the space the
//! game's own interface was blended in. That is the honest target here: not
//! physical correctness in a pipeline that has no notion of it, but agreement
//! with the game the overlay is drawing over.
//!
//! ## What is actually in the process
//!
//! A great many D3D8 titles that still run do so through a community
//! replacement `d3d8.dll` — `d3d8to9`, or DXVK's own — sitting in the game's
//! directory. The device the game holds is then that wrapper's C++ object
//! implementing `IDirect3DDevice8`, forwarding to a real Direct3D 9 or Vulkan
//! device underneath.
//!
//! This backend hooks and draws through **the `IDirect3DDevice8` the game
//! itself calls**, wrapper or not, and that is a deliberate choice rather than
//! the path of least resistance. Reaching past the wrapper to the D3D9 device
//! underneath would mean the overlay changing state the wrapper believes it
//! owns: the wrapper caches what the game set, the overlay would set something
//! else, and the wrapper's next translated call would restore its own idea of
//! the state over the overlay's — or worse, the overlay's over the game's. At
//! the D3D8 interface the overlay and the game speak the same vocabulary, the
//! state block is the wrapper's own state block, and whatever the wrapper does
//! underneath is its business.
//!
//! The cost is stated rather than hidden: on a wrapper, "the state was
//! restored" means the wrapper's `ApplyStateBlock` said so. [`QudratD3D8`]
//! reports which of the two is present before the overlay is enabled.
//!
//! ## The vtable is verified before it is trusted
//!
//! Nothing in `windows` binds Direct3D 8 — the API predates every binding
//! generator this workspace could use — so [`JadwalJihaz8`] is transcribed from
//! `d3d8.h` by hand, ninety-six slots of it. A transcription error there is a
//! call into the wrong method inside somebody's game, which is a crash whose
//! stack names nothing.
//!
//! So it is not trusted. [`tahaqquq_jadwal`] exercises five slots whose answers
//! are predictable — `TestCooperativeLevel`, `GetCreationParameters`,
//! `GetDisplayMode`, and a `SetRenderState`/`GetRenderState` round trip through
//! a state nobody else uses — and refuses to install if any of them disagrees.
//! It runs against a device created for the purpose, before a single slot is
//! replaced.

use core::ffi::c_void;
use core::fmt;

use crate::khata::KhataTabaqa;
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

// ---------------------------------------------------------------------------
// The scalar vocabulary
// ---------------------------------------------------------------------------

/// `HRESULT`, as Direct3D 8 returns it.
///
/// Kept as a plain `i32` rather than the `windows` crate's wrapper because this
/// module compiles on every platform: the interface declarations, the vertex
/// packing, the colour conversion and the hook bookkeeping are all
/// platform-independent, and only the two functions that open `d3d8.dll` are
/// not. That is what lets the hook install, the unhook refusal and the state
/// sequencing be exercised on a machine with no Direct3D at all.
type Natija8 = i32;

/// Whether an `HRESULT` reports success.
const fn najah(natija: Natija8) -> bool {
    natija >= 0
}

/// `D3DERR_DEVICELOST`.
const D3DERR_DEVICELOST: Natija8 = -0x7789_F798;
/// `D3DERR_DEVICENOTRESET`.
const D3DERR_DEVICENOTRESET: Natija8 = -0x7789_F797;
/// `D3DERR_INVALIDCALL`.
const D3DERR_INVALIDCALL: Natija8 = -0x7789_F794;

// The three codes above are written as negative decimals so the compiler
// computes them, and checked here against the hexadecimal spellings that appear
// in `d3d8.h` and in every bug report anybody will ever paste.
const _: () = {
    assert!(D3DERR_DEVICELOST.cast_unsigned() == 0x8876_0868, "D3DERR_DEVICELOST is wrong");
    assert!(D3DERR_DEVICENOTRESET.cast_unsigned() == 0x8876_0869, "D3DERR_DEVICENOTRESET is wrong");
    assert!(D3DERR_INVALIDCALL.cast_unsigned() == 0x8876_086C, "D3DERR_INVALIDCALL is wrong");
};

/// A `RECT`, declared here because this module compiles off Windows too.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Mustatil8 {
    /// Left edge.
    yasar: i32,
    /// Top edge.
    aala: i32,
    /// One past the right edge.
    yameen: i32,
    /// One past the bottom edge.
    asfal: i32,
}

/// A `POINT`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Nuqta8 {
    /// Horizontal.
    s: i32,
    /// Vertical.
    a: i32,
}

/// `D3DVIEWPORT8`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct Manzar8 {
    /// Left edge in pixels.
    s: u32,
    /// Top edge in pixels.
    a: u32,
    /// Width in pixels.
    ard: u32,
    /// Height in pixels.
    irtifa: u32,
    /// Near depth.
    adna_umq: f32,
    /// Far depth.
    aqsa_umq: f32,
}

/// `D3DSURFACE_DESC`, in Direct3D 8's own field order.
///
/// Not Direct3D 9's. The eighth version carries a `Size` between `Pool` and
/// `MultiSampleType` and has no `MultiSampleQuality`; the ninth dropped the one
/// and added the other. Reading a D3D8 surface through a D3D9 description
/// returns the height where the width belongs, which is a correctly sized
/// capture of the wrong rectangle rather than an error.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct WasfSath8 {
    /// `D3DFORMAT`.
    sigha: u32,
    /// `D3DRESOURCETYPE`.
    naw: u32,
    /// Usage flags.
    istikhdam: u32,
    /// `D3DPOOL`.
    hawd: u32,
    /// The surface's size in bytes.
    hajm: u32,
    /// `D3DMULTISAMPLE_TYPE`.
    taadud: u32,
    /// Width in pixels.
    ard: u32,
    /// Height in pixels.
    irtifa: u32,
}

/// `D3DLOCKED_RECT`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MustatilMaqful8 {
    /// Distance between the starts of two rows, in bytes.
    khatwa: i32,
    /// The first byte of the first row.
    bayt: *mut c_void,
}

/// `D3DDISPLAYMODE`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct NamatArd8 {
    /// Width in pixels.
    ard: u32,
    /// Height in pixels.
    irtifa: u32,
    /// Refresh rate in hertz, or zero for the default.
    taraddud: u32,
    /// `D3DFORMAT`.
    sigha: u32,
}

/// `D3DDEVICE_CREATION_PARAMETERS`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MuallimatInsha8 {
    /// Which adapter the device was created on.
    muhawwil: u32,
    /// `D3DDEVTYPE`.
    naw: u32,
    /// The focus window.
    nafidha: *mut c_void,
    /// `D3DCREATE_*` flags.
    aalam: u32,
}

impl Default for MustatilMaqful8 {
    fn default() -> Self {
        Self { khatwa: 0, bayt: core::ptr::null_mut() }
    }
}

impl Default for MuallimatInsha8 {
    fn default() -> Self {
        Self { muhawwil: 0, naw: 0, nafidha: core::ptr::null_mut(), aalam: 0 }
    }
}

/// `D3DPRESENT_PARAMETERS`, in Direct3D 8's own field order.
///
/// Thirteen fields. Direct3D 9 inserted `MultiSampleQuality` after
/// `MultiSampleType` and made it fourteen, which is why this is spelled out
/// here rather than shared with anything.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct MuallimatArd8 {
    /// Backbuffer width, or zero to take the window's.
    ard: u32,
    /// Backbuffer height, or zero to take the window's.
    irtifa: u32,
    /// `D3DFORMAT`.
    sigha: u32,
    /// How many backbuffers.
    adad: u32,
    /// `D3DMULTISAMPLE_TYPE`.
    taadud: u32,
    /// `D3DSWAPEFFECT`.
    athar: u32,
    /// The device window.
    nafidha: *mut c_void,
    /// Non-zero for windowed.
    munafadha: i32,
    /// Non-zero to have the runtime make a depth buffer.
    umq_tilqai: i32,
    /// The depth format, when the runtime is making one.
    sighat_umq: u32,
    /// `D3DPRESENTFLAG_*`.
    aalam: u32,
    /// Fullscreen refresh rate.
    taraddud: u32,
    /// Fullscreen presentation interval.
    fasil: u32,
}

// ---------------------------------------------------------------------------
// The constants, at the values d3d8.h gives them
// ---------------------------------------------------------------------------

/// `D3D_SDK_VERSION` for Direct3D 8.
///
/// Named only where a device is created, which is Windows.
///
/// Not 32, which is Direct3D 9's. `Direct3DCreate8` returns null for any other
/// value, and a null there is indistinguishable from "this machine has no
/// Direct3D 8" unless the number is right.
#[cfg(windows)]
const ISDAR_SDK_8: u32 = 220;

/// `D3DADAPTER_DEFAULT`.
///
/// Named only where a device is created or verified, which is Windows and the
/// tests that stand in for it; the declarations themselves compile everywhere
/// and reference none of the creation vocabulary.
#[cfg(any(windows, test))]
const MUHAWWIL_IFTIRADI: u32 = 0;
/// `D3DDEVTYPE_HAL`.
#[cfg(any(windows, test))]
const NAW_JIHAZ_HAL: u32 = 1;
/// `D3DCREATE_SOFTWARE_VERTEXPROCESSING`.
#[cfg(windows)]
const INSHA_RUUS_BARMAJI: u32 = 0x0000_0020;
/// `D3DCREATE_PUREDEVICE`, the flag that makes every `Get*` state query fail.
const INSHA_NAQI: u32 = 0x0000_0010;
/// `D3DCREATE_MULTITHREADED`.
const INSHA_MUTAADID: u32 = 0x0000_0004;
/// `D3DCREATE_HARDWARE_VERTEXPROCESSING`.
const INSHA_RUUS_ARIDA: u32 = 0x0000_0040;

/// `D3DSWAPEFFECT_DISCARD`.
#[cfg(windows)]
const TABDIL_IHMAL: u32 = 1;
/// `D3DBACKBUFFER_TYPE_MONO`.
const KHALFIYA_UHADIYA: u32 = 0;
/// `D3DPOOL_MANAGED`, which survives a device reset.
const HAWD_MUDAR: u32 = 1;
/// `D3DSTATEBLOCKTYPE` `D3DSBT_ALL`.
const KUTLA_KAAMILA: u32 = 1;
/// `D3DPT_TRIANGLELIST`.
const RASM_MUTHALLATHAT: u32 = 4;
/// `D3DLOCK_READONLY`.
const QUFL_QIRA_FAQAT: u32 = 0x0000_0010;
/// `D3DMULTISAMPLE_NONE`.
const TAADUD_LA_SHAY: u32 = 0;

/// `D3DFVF_XYZRHW | D3DFVF_DIFFUSE | D3DFVF_TEX1`.
///
/// Pre-transformed position, one packed colour, one two-float texture
/// coordinate. Pre-transformed because the overlay's quads are already in
/// surface pixels and running them through a world-view-projection the game
/// left behind is the one way to draw Arabic somewhere other than where it was
/// measured.
const FVF_TABAQA: u32 = 0x0004 | 0x0040 | 0x0100;

/// `D3DFMT_A8R8G8B8`, the atlas's format and the only one it is uploaded in.
const SIGHA_A8R8G8B8: u32 = 21;
/// `D3DFMT_X8R8G8B8`.
const SIGHA_X8R8G8B8: u32 = 22;
/// `D3DFMT_R5G6B5`.
const SIGHA_R5G6B5: u32 = 23;
/// `D3DFMT_X1R5G5B5`.
const SIGHA_X1R5G5B5: u32 = 24;
/// `D3DFMT_A1R5G5B5`.
const SIGHA_A1R5G5B5: u32 = 25;
/// `D3DFMT_A2R10G10B10`.
const SIGHA_A2R10G10B10: u32 = 35;

/// The render states this backend writes, at `d3d8.h`'s numbering.
mod hala {
    /// `D3DRS_ZENABLE`.
    pub(super) const UMQ: u32 = 7;
    /// `D3DRS_FILLMODE`.
    pub(super) const MIL: u32 = 8;
    /// `D3DRS_SHADEMODE`.
    pub(super) const TAZLIL: u32 = 9;
    /// `D3DRS_ZWRITEENABLE`.
    pub(super) const KITABAT_UMQ: u32 = 14;
    /// `D3DRS_ALPHATESTENABLE`.
    pub(super) const IKHTIBAR_SHAFAFIYA: u32 = 15;
    /// `D3DRS_SRCBLEND`.
    pub(super) const MASDAR_MAZJ: u32 = 19;
    /// `D3DRS_DESTBLEND`.
    pub(super) const HADAF_MAZJ: u32 = 20;
    /// `D3DRS_CULLMODE`.
    pub(super) const HADHF: u32 = 22;
    /// `D3DRS_DITHERENABLE`.
    pub(super) const TASHWISH: u32 = 26;
    /// `D3DRS_ALPHABLENDENABLE`.
    pub(super) const MAZJ: u32 = 27;
    /// `D3DRS_FOGENABLE`.
    pub(super) const DABAB: u32 = 28;
    /// `D3DRS_SPECULARENABLE`.
    pub(super) const LAMAAN: u32 = 29;
    /// `D3DRS_STENCILENABLE`.
    pub(super) const QALIB: u32 = 52;
    /// `D3DRS_TEXTUREFACTOR`, written and read by the vtable self-check alone.
    pub(super) const AAMIL_NASIJ: u32 = 60;
    /// `D3DRS_CLIPPING`.
    pub(super) const QASS: u32 = 136;
    /// `D3DRS_LIGHTING`.
    pub(super) const IDAA: u32 = 137;
    /// `D3DRS_VERTEXBLEND`.
    pub(super) const MAZJ_RUUS: u32 = 151;
    /// `D3DRS_CLIPPLANEENABLE`.
    pub(super) const MUSTAWAYAT_QASS: u32 = 152;
    /// `D3DRS_COLORWRITEENABLE`.
    pub(super) const KITABAT_LAWN: u32 = 168;
    /// `D3DRS_BLENDOP`.
    pub(super) const AMALIYAT_MAZJ: u32 = 171;
}

/// The texture stage states this backend writes, at `d3d8.h`'s numbering.
mod marhala {
    /// `D3DTSS_COLOROP`.
    pub(super) const AMALIYAT_LAWN: u32 = 1;
    /// `D3DTSS_COLORARG1`.
    pub(super) const MUAMIL_LAWN_1: u32 = 2;
    /// `D3DTSS_COLORARG2`.
    pub(super) const MUAMIL_LAWN_2: u32 = 3;
    /// `D3DTSS_ALPHAOP`.
    pub(super) const AMALIYAT_SHAFAFIYA: u32 = 4;
    /// `D3DTSS_ALPHAARG1`.
    pub(super) const MUAMIL_SHAFAFIYA_1: u32 = 5;
    /// `D3DTSS_ALPHAARG2`.
    pub(super) const MUAMIL_SHAFAFIYA_2: u32 = 6;
    /// `D3DTSS_TEXCOORDINDEX`.
    pub(super) const FAHRAS_IHDATHIYAT: u32 = 11;
    /// `D3DTSS_ADDRESSU`.
    pub(super) const UNWAN_S: u32 = 13;
    /// `D3DTSS_ADDRESSV`.
    pub(super) const UNWAN_A: u32 = 14;
    /// `D3DTSS_MAGFILTER`.
    pub(super) const MURASHAH_TAKBIR: u32 = 16;
    /// `D3DTSS_MINFILTER`.
    pub(super) const MURASHAH_TASGHIR: u32 = 17;
    /// `D3DTSS_MIPFILTER`.
    pub(super) const MURASHAH_TADARRUJ: u32 = 18;
    /// `D3DTSS_TEXTURETRANSFORMFLAGS`.
    pub(super) const TAHWIL: u32 = 24;
}

/// `D3DTOP_DISABLE`.
const AMAL_MUATTAL: u32 = 1;
/// `D3DTOP_SELECTARG2`.
const AMAL_IKHTAR_2: u32 = 3;
/// `D3DTOP_MODULATE`.
const AMAL_DARB: u32 = 4;
/// `D3DTA_DIFFUSE`.
const MUAMIL_MUNTASHIR: u32 = 0;
/// `D3DTA_TEXTURE`.
const MUAMIL_NASIJ: u32 = 2;
/// `D3DTEXF_NONE`.
const MURASHAH_LA_SHAY: u32 = 0;
/// `D3DTEXF_LINEAR`.
const MURASHAH_KHATTI: u32 = 2;
/// `D3DTADDRESS_CLAMP`.
const UNWAN_MUQAYYAD: u32 = 3;
/// `D3DBLEND_ONE`.
const MAZJ_WAHID: u32 = 2;
/// `D3DBLEND_INVSRCALPHA`.
const MAZJ_MAQLUB: u32 = 6;
/// `D3DBLENDOP_ADD`.
const MAZJ_JAM: u32 = 1;
/// `D3DCULL_NONE`.
const HADHF_LA_SHAY: u32 = 1;
/// `D3DFILL_SOLID`.
const MIL_MUSMAT: u32 = 3;
/// `D3DSHADE_GOURAUD`.
const TAZLIL_MUTADARRIJ: u32 = 2;
/// `D3DZB_FALSE`.
const UMQ_MUTLAQ: u32 = 0;
/// Every colour channel writable.
const KITABAT_KAAMILA: u32 = 0x0F;
/// `D3DTTFF_DISABLE`.
const TAHWIL_MUATTAL: u32 = 0;

// ---------------------------------------------------------------------------
// The method tables, transcribed from d3d8.h
// ---------------------------------------------------------------------------

/// A slot this backend never calls.
///
/// Present so the table's field order is the interface's slot order and can be
/// read against the header one line at a time. Typing every one of the
/// ninety-six would be ninety-six more chances to write a signature wrong for a
/// method nothing calls.
type KhanaMuhmala = *const c_void;

/// `IDirect3DDevice8`'s method table, all ninety-six slots.
///
/// The field order **is** the slot order. Nothing here may be reordered,
/// inserted into, or removed, and [`tahaqquq_jadwal`] is what checks that at
/// runtime rather than trusting this comment.
#[repr(C)]
#[allow(
    dead_code,
    reason = "the untyped slots exist to give the typed ones their correct offsets; a field the \
              compiler calls unread is one whose only job is to occupy a machine word between two \
              methods this backend does call. `allow` rather than `expect`: which slots are read \
              differs between the platform that has Direct3D 8 and the platforms that only \
              compile the declarations, and an expectation would be unfulfilled on one of them"
)]
struct JadwalJihaz8 {
    /// Slot 0, `IUnknown::QueryInterface`.
    istifsar: KhanaMuhmala,
    /// Slot 1, `IUnknown::AddRef`.
    idafa: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Slot 2, `IUnknown::Release`.
    itlaq: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Slot 3, `TestCooperativeLevel`.
    ikhtibar_taawun: unsafe extern "system" fn(*mut c_void) -> Natija8,
    /// Slot 4, `GetAvailableTextureMem`.
    dhakirat_nasij: KhanaMuhmala,
    /// Slot 5, `ResourceManagerDiscardBytes`.
    ihmal_bayt: KhanaMuhmala,
    /// Slot 6, `GetDirect3D`.
    ijlib_d3d: KhanaMuhmala,
    /// Slot 7, `GetDeviceCaps`.
    qudrat_jihaz: KhanaMuhmala,
    /// Slot 8, `GetDisplayMode`.
    namat_ard: unsafe extern "system" fn(*mut c_void, *mut NamatArd8) -> Natija8,
    /// Slot 9, `GetCreationParameters`.
    muallimat_insha: unsafe extern "system" fn(*mut c_void, *mut MuallimatInsha8) -> Natija8,
    /// Slot 10, `SetCursorProperties`.
    khasais_muashir: KhanaMuhmala,
    /// Slot 11, `SetCursorPosition`.
    mawdi_muashir: KhanaMuhmala,
    /// Slot 12, `ShowCursor`.
    izhar_muashir: KhanaMuhmala,
    /// Slot 13, `CreateAdditionalSwapChain`.
    silsila_idafiya: KhanaMuhmala,
    /// Slot 14, `Reset`.
    tasfir: KhanaMuhmala,
    /// Slot 15, `Present`.
    taqdeem: KhanaMuhmala,
    /// Slot 16, `GetBackBuffer`.
    khalfiya: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut c_void) -> Natija8,
    /// Slot 17, `GetRasterStatus`.
    hala_masah: KhanaMuhmala,
    /// Slot 18, `SetGammaRamp`.
    tadarruj_jama: KhanaMuhmala,
    /// Slot 19, `GetGammaRamp`.
    ijlib_tadarruj: KhanaMuhmala,
    /// Slot 20, `CreateTexture`.
    insha_nasij:
        unsafe extern "system" fn(*mut c_void, u32, u32, u32, u32, u32, u32, *mut *mut c_void)
            -> Natija8,
    /// Slot 21, `CreateVolumeTexture`.
    insha_nasij_hajmi: KhanaMuhmala,
    /// Slot 22, `CreateCubeTexture`.
    insha_nasij_mukaab: KhanaMuhmala,
    /// Slot 23, `CreateVertexBuffer`.
    insha_mahfazat_ruus: KhanaMuhmala,
    /// Slot 24, `CreateIndexBuffer`.
    insha_mahfazat_faharis: KhanaMuhmala,
    /// Slot 25, `CreateRenderTarget`.
    insha_hadaf: KhanaMuhmala,
    /// Slot 26, `CreateDepthStencilSurface`.
    insha_sath_umq: KhanaMuhmala,
    /// Slot 27, `CreateImageSurface`.
    insha_sath_sura:
        unsafe extern "system" fn(*mut c_void, u32, u32, u32, *mut *mut c_void) -> Natija8,
    /// Slot 28, `CopyRects`.
    nasakh_mustatilat: unsafe extern "system" fn(
        *mut c_void,
        *mut c_void,
        *const Mustatil8,
        u32,
        *mut c_void,
        *const Nuqta8,
    ) -> Natija8,
    /// Slot 29, `UpdateTexture`.
    tahdith_nasij: KhanaMuhmala,
    /// Slot 30, `GetFrontBuffer`.
    amamiya: KhanaMuhmala,
    /// Slot 31, `SetRenderTarget`.
    dai_hadaf: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> Natija8,
    /// Slot 32, `GetRenderTarget`.
    ijlib_hadaf: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Natija8,
    /// Slot 33, `GetDepthStencilSurface`.
    ijlib_sath_umq: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> Natija8,
    /// Slot 34, `BeginScene`.
    ibda_mashhad: unsafe extern "system" fn(*mut c_void) -> Natija8,
    /// Slot 35, `EndScene`.
    inhi_mashhad: unsafe extern "system" fn(*mut c_void) -> Natija8,
    /// Slot 36, `Clear`.
    imsah: KhanaMuhmala,
    /// Slot 37, `SetTransform`.
    dai_tahwil: KhanaMuhmala,
    /// Slot 38, `GetTransform`.
    ijlib_tahwil: KhanaMuhmala,
    /// Slot 39, `MultiplyTransform`.
    darb_tahwil: KhanaMuhmala,
    /// Slot 40, `SetViewport`.
    dai_manzar: unsafe extern "system" fn(*mut c_void, *const Manzar8) -> Natija8,
    /// Slot 41, `GetViewport`.
    ijlib_manzar: unsafe extern "system" fn(*mut c_void, *mut Manzar8) -> Natija8,
    /// Slot 42, `SetMaterial`.
    dai_khama: KhanaMuhmala,
    /// Slot 43, `GetMaterial`.
    ijlib_khama: KhanaMuhmala,
    /// Slot 44, `SetLight`.
    dai_daw: KhanaMuhmala,
    /// Slot 45, `GetLight`.
    ijlib_daw: KhanaMuhmala,
    /// Slot 46, `LightEnable`.
    tafeel_daw: KhanaMuhmala,
    /// Slot 47, `GetLightEnable`.
    hala_daw: KhanaMuhmala,
    /// Slot 48, `SetClipPlane`.
    dai_mustawa: KhanaMuhmala,
    /// Slot 49, `GetClipPlane`.
    ijlib_mustawa: KhanaMuhmala,
    /// Slot 50, `SetRenderState`.
    dai_hala: unsafe extern "system" fn(*mut c_void, u32, u32) -> Natija8,
    /// Slot 51, `GetRenderState`.
    ijlib_hala: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> Natija8,
    /// Slot 52, `BeginStateBlock`.
    ibda_kutla: KhanaMuhmala,
    /// Slot 53, `EndStateBlock`.
    inhi_kutla: KhanaMuhmala,
    /// Slot 54, `ApplyStateBlock`.
    tatbiq_kutla: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 55, `CaptureStateBlock`.
    iltiqat_kutla: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 56, `DeleteStateBlock`.
    hadhf_kutla: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 57, `CreateStateBlock`.
    insha_kutla: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> Natija8,
    /// Slot 58, `SetClipStatus`.
    dai_hala_qass: KhanaMuhmala,
    /// Slot 59, `GetClipStatus`.
    ijlib_hala_qass: KhanaMuhmala,
    /// Slot 60, `GetTexture`.
    ijlib_nasij: KhanaMuhmala,
    /// Slot 61, `SetTexture`.
    dai_nasij: unsafe extern "system" fn(*mut c_void, u32, *mut c_void) -> Natija8,
    /// Slot 62, `GetTextureStageState`.
    ijlib_hala_marhala: unsafe extern "system" fn(*mut c_void, u32, u32, *mut u32) -> Natija8,
    /// Slot 63, `SetTextureStageState`.
    dai_hala_marhala: unsafe extern "system" fn(*mut c_void, u32, u32, u32) -> Natija8,
    /// Slot 64, `ValidateDevice`.
    tahaqquq_jihaz: KhanaMuhmala,
    /// Slot 65, `GetInfo`.
    ijlib_bayan: KhanaMuhmala,
    /// Slot 66, `SetPaletteEntries`.
    dai_lawha_alwan: KhanaMuhmala,
    /// Slot 67, `GetPaletteEntries`.
    ijlib_lawha_alwan: KhanaMuhmala,
    /// Slot 68, `SetCurrentTexturePalette`.
    dai_lawha_haliya: KhanaMuhmala,
    /// Slot 69, `GetCurrentTexturePalette`.
    ijlib_lawha_haliya: KhanaMuhmala,
    /// Slot 70, `DrawPrimitive`.
    irsim: KhanaMuhmala,
    /// Slot 71, `DrawIndexedPrimitive`.
    irsim_mufahras: KhanaMuhmala,
    /// Slot 72, `DrawPrimitiveUP`.
    irsim_mubashir:
        unsafe extern "system" fn(*mut c_void, u32, u32, *const c_void, u32) -> Natija8,
    /// Slot 73, `DrawIndexedPrimitiveUP`.
    irsim_mufahras_mubashir: KhanaMuhmala,
    /// Slot 74, `ProcessVertices`.
    muaalajat_ruus: KhanaMuhmala,
    /// Slot 75, `CreateVertexShader`.
    insha_shifrat_ruus: KhanaMuhmala,
    /// Slot 76, `SetVertexShader`, which in Direct3D 8 also carries the FVF.
    dai_shifrat_ruus: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 77, `GetVertexShader`.
    ijlib_shifrat_ruus: KhanaMuhmala,
    /// Slot 78, `DeleteVertexShader`.
    hadhf_shifrat_ruus: KhanaMuhmala,
    /// Slot 79, `SetVertexShaderConstant`.
    dai_thabit_ruus: KhanaMuhmala,
    /// Slot 80, `GetVertexShaderConstant`.
    ijlib_thabit_ruus: KhanaMuhmala,
    /// Slot 81, `GetVertexShaderDeclaration`.
    tasrih_shifrat_ruus: KhanaMuhmala,
    /// Slot 82, `GetVertexShaderFunction`.
    dallat_shifrat_ruus: KhanaMuhmala,
    /// Slot 83, `SetStreamSource`.
    dai_majra: KhanaMuhmala,
    /// Slot 84, `GetStreamSource`.
    ijlib_majra: KhanaMuhmala,
    /// Slot 85, `SetIndices`.
    dai_faharis: KhanaMuhmala,
    /// Slot 86, `GetIndices`.
    ijlib_faharis: KhanaMuhmala,
    /// Slot 87, `CreatePixelShader`.
    insha_shifrat_biksel: KhanaMuhmala,
    /// Slot 88, `SetPixelShader`.
    dai_shifrat_biksel: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 89, `GetPixelShader`.
    ijlib_shifrat_biksel: KhanaMuhmala,
    /// Slot 90, `DeletePixelShader`.
    hadhf_shifrat_biksel: KhanaMuhmala,
    /// Slot 91, `SetPixelShaderConstant`.
    dai_thabit_biksel: KhanaMuhmala,
    /// Slot 92, `GetPixelShaderFunction`.
    dallat_shifrat_biksel: KhanaMuhmala,
    /// Slot 93, `DrawRectPatch`.
    ruqaat_mustatila: KhanaMuhmala,
    /// Slot 94, `DrawTriPatch`.
    ruqaat_muthallatha: KhanaMuhmala,
    /// Slot 95, `DeletePatch`.
    hadhf_ruqaa: KhanaMuhmala,
}

/// The vtable slot of `IDirect3DDevice8::Present`.
///
/// `IUnknown` contributes three, then `TestCooperativeLevel`,
/// `GetAvailableTextureMem`, `ResourceManagerDiscardBytes`, `GetDirect3D`,
/// `GetDeviceCaps`, `GetDisplayMode`, `GetCreationParameters`,
/// `SetCursorProperties`, `SetCursorPosition`, `ShowCursor`,
/// `CreateAdditionalSwapChain` and `Reset` — twelve. Fifteen.
///
/// Written as a derivation because the number fifteen is unfalsifiable in
/// review and the derivation is checkable against `d3d8.h` by anybody who
/// doubts it.
pub const KHANAT_TAQDEEM_8: usize = 3 + 12;

/// The vtable slot of `IDirect3DDevice8::Reset`.
///
/// One before `Present`. Hooked for the same reason [`crate::d3d11`] hooks
/// `ResizeBuffers`: a state block does not survive a device reset, and the only
/// moment it can be released is before the call reaches the runtime.
pub const KHANAT_TASFIR_8: usize = KHANAT_TAQDEEM_8 - 1;

// The two slot constants and the table must agree, or the hook replaces one
// method believing it is another. Checked at compile time against the field
// offsets rather than asserted in prose.
const _: () = {
    assert!(
        core::mem::offset_of!(JadwalJihaz8, taqdeem)
            == KHANAT_TAQDEEM_8 * size_of::<*const c_void>(),
        "Present is not at the slot the hook writes"
    );
    assert!(
        core::mem::offset_of!(JadwalJihaz8, tasfir)
            == KHANAT_TASFIR_8 * size_of::<*const c_void>(),
        "Reset is not at the slot the hook writes"
    );
    assert!(
        size_of::<JadwalJihaz8>() == 96 * size_of::<*const c_void>(),
        "IDirect3DDevice8 does not have ninety-six methods"
    );
};

/// `IDirect3DSurface8`'s method table.
#[repr(C)]
#[allow(dead_code, reason = "as `JadwalJihaz8`: the untyped slots carry the offsets")]
struct JadwalSath8 {
    /// Slot 0, `QueryInterface`.
    istifsar: KhanaMuhmala,
    /// Slot 1, `AddRef`.
    idafa: KhanaMuhmala,
    /// Slot 2, `Release`.
    itlaq: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Slot 3, `GetDevice`.
    ijlib_jihaz: KhanaMuhmala,
    /// Slot 4, `SetPrivateData`.
    dai_bayanat: KhanaMuhmala,
    /// Slot 5, `GetPrivateData`.
    ijlib_bayanat: KhanaMuhmala,
    /// Slot 6, `FreePrivateData`.
    tahrir_bayanat: KhanaMuhmala,
    /// Slot 7, `GetContainer`.
    ijlib_hawiya: KhanaMuhmala,
    /// Slot 8, `GetDesc`.
    wasf: unsafe extern "system" fn(*mut c_void, *mut WasfSath8) -> Natija8,
    /// Slot 9, `LockRect`.
    aqfil: unsafe extern "system" fn(
        *mut c_void,
        *mut MustatilMaqful8,
        *const Mustatil8,
        u32,
    ) -> Natija8,
    /// Slot 10, `UnlockRect`.
    ifta: unsafe extern "system" fn(*mut c_void) -> Natija8,
}

/// `IDirect3DTexture8`'s method table.
#[repr(C)]
#[allow(dead_code, reason = "as `JadwalJihaz8`: the untyped slots carry the offsets")]
struct JadwalNasij8 {
    /// Slot 0, `QueryInterface`.
    istifsar: KhanaMuhmala,
    /// Slot 1, `AddRef`.
    idafa: KhanaMuhmala,
    /// Slot 2, `Release`.
    itlaq: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Slot 3, `GetDevice`.
    ijlib_jihaz: KhanaMuhmala,
    /// Slot 4, `SetPrivateData`.
    dai_bayanat: KhanaMuhmala,
    /// Slot 5, `GetPrivateData`.
    ijlib_bayanat: KhanaMuhmala,
    /// Slot 6, `FreePrivateData`.
    tahrir_bayanat: KhanaMuhmala,
    /// Slot 7, `SetPriority`.
    dai_awlawiya: KhanaMuhmala,
    /// Slot 8, `GetPriority`.
    ijlib_awlawiya: KhanaMuhmala,
    /// Slot 9, `PreLoad`.
    tahmil_musbaq: KhanaMuhmala,
    /// Slot 10, `GetType`.
    naw: KhanaMuhmala,
    /// Slot 11, `SetLOD`.
    dai_mustawa: KhanaMuhmala,
    /// Slot 12, `GetLOD`.
    ijlib_mustawa: KhanaMuhmala,
    /// Slot 13, `GetLevelCount`.
    adad_mustawayat: KhanaMuhmala,
    /// Slot 14, `GetLevelDesc`.
    wasf_mustawa: KhanaMuhmala,
    /// Slot 15, `GetSurfaceLevel`.
    sath_mustawa: KhanaMuhmala,
    /// Slot 16, `LockRect`.
    aqfil: unsafe extern "system" fn(
        *mut c_void,
        u32,
        *mut MustatilMaqful8,
        *const Mustatil8,
        u32,
    ) -> Natija8,
    /// Slot 17, `UnlockRect`.
    ifta: unsafe extern "system" fn(*mut c_void, u32) -> Natija8,
    /// Slot 18, `AddDirtyRect`.
    mustatil_muttasikh: KhanaMuhmala,
}

/// `IDirect3D8`'s method table, for the throwaway device the probe creates.
#[repr(C)]
#[allow(dead_code, reason = "as `JadwalJihaz8`: the untyped slots carry the offsets")]
struct JadwalD3d8 {
    /// Slot 0, `QueryInterface`.
    istifsar: KhanaMuhmala,
    /// Slot 1, `AddRef`.
    idafa: KhanaMuhmala,
    /// Slot 2, `Release`.
    itlaq: unsafe extern "system" fn(*mut c_void) -> u32,
    /// Slot 3, `RegisterSoftwareDevice`.
    tasjil_jihaz: KhanaMuhmala,
    /// Slot 4, `GetAdapterCount`.
    adad_muhawwilat: KhanaMuhmala,
    /// Slot 5, `GetAdapterIdentifier`.
    muarrif_muhawwil: KhanaMuhmala,
    /// Slot 6, `GetAdapterModeCount`.
    adad_anmat: KhanaMuhmala,
    /// Slot 7, `EnumAdapterModes`.
    tada_anmat: KhanaMuhmala,
    /// Slot 8, `GetAdapterDisplayMode`.
    namat_ard: unsafe extern "system" fn(*mut c_void, u32, *mut NamatArd8) -> Natija8,
    /// Slot 9, `CheckDeviceType`.
    fahs_naw: KhanaMuhmala,
    /// Slot 10, `CheckDeviceFormat`.
    fahs_sigha: KhanaMuhmala,
    /// Slot 11, `CheckDeviceMultiSampleType`.
    fahs_taadud: KhanaMuhmala,
    /// Slot 12, `CheckDepthStencilMatch`.
    fahs_umq: KhanaMuhmala,
    /// Slot 13, `GetDeviceCaps`.
    qudrat: KhanaMuhmala,
    /// Slot 14, `GetAdapterMonitor`.
    shashat_muhawwil: KhanaMuhmala,
    /// Slot 15, `CreateDevice`.
    insha_jihaz: unsafe extern "system" fn(
        *mut c_void,
        u32,
        u32,
        *mut c_void,
        u32,
        *mut MuallimatArd8,
        *mut *mut c_void,
    ) -> Natija8,
}

// ---------------------------------------------------------------------------
// Calling through the tables
// ---------------------------------------------------------------------------

/// A live `IDirect3DDevice8`, reached through its own method table.
///
/// The table pointer is re-read on every call rather than cached, and that is
/// deliberate: a second overlay may have replaced a slot since the last frame,
/// and calling through the slot as it stands now is what keeps a hook chain
/// intact. This backend never calls `Present` or `Reset`, so re-reading cannot
/// reach its own thunk.
#[derive(Clone, Copy)]
struct Jihaz8(*mut c_void);

impl fmt::Debug for Jihaz8 {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The address is deliberately not printed: it is a heap pointer inside
        // somebody else's process, it appears in diagnostics bundles users
        // paste in public, and it answers no question a reader has.
        mukhraj.write_str("IDirect3DDevice8")
    }
}

impl Jihaz8 {
    /// Wraps a device pointer.
    ///
    /// # Safety
    ///
    /// `khaam` must be a live `IDirect3DDevice8` instance for as long as this
    /// value is used. Every method below reads the first machine word as a
    /// method table and calls through it, which against anything else is a call
    /// to an arbitrary address.
    const unsafe fn jadeed(khaam: *mut c_void) -> Self {
        Self(khaam)
    }

    /// The instance's method table.
    const fn jadwal(self) -> *const JadwalJihaz8 {
        // SAFETY: this type's invariant is that `self.0` is a live COM
        // interface, whose first machine word is its method table pointer by
        // the ABI every Microsoft compiler implements.
        unsafe { self.0.cast::<*const JadwalJihaz8>().read() }
    }

    /// Takes a reference, so the device outlives the frame it was found in.
    fn idafa(self) -> u32 {
        // SAFETY: the table is the live instance's and `AddRef` takes only the
        // `this` pointer.
        unsafe { ((*self.jadwal()).idafa)(self.0) }
    }

    /// Gives the reference back.
    fn itlaq(self) -> u32 {
        // SAFETY: as `idafa`. The caller is responsible for calling this once
        // per `idafa` and never after the last one.
        unsafe { ((*self.jadwal()).itlaq)(self.0) }
    }

    /// `TestCooperativeLevel`.
    fn ikhtibar_taawun(self) -> Natija8 {
        // SAFETY: as `idafa`; the method takes only `this`.
        unsafe { ((*self.jadwal()).ikhtibar_taawun)(self.0) }
    }

    /// `GetCreationParameters`.
    fn muallimat_insha(self) -> Option<MuallimatInsha8> {
        let mut muallimat = MuallimatInsha8::default();
        // SAFETY: the out-parameter addresses a fully initialised local of
        // exactly the layout `d3d8.h` gives `D3DDEVICE_CREATION_PARAMETERS`.
        let natija = unsafe { ((*self.jadwal()).muallimat_insha)(self.0, &raw mut muallimat) };
        najah(natija).then_some(muallimat)
    }

    /// `GetDisplayMode`.
    fn namat_ard(self) -> Option<NamatArd8> {
        let mut namat = NamatArd8::default();
        // SAFETY: the out-parameter addresses a live local of the layout
        // `d3d8.h` gives `D3DDISPLAYMODE`.
        let natija = unsafe { ((*self.jadwal()).namat_ard)(self.0, &raw mut namat) };
        najah(natija).then_some(namat)
    }

    /// `SetRenderState`.
    fn dai_hala(self, hala: u32, qeema: u32) -> Natija8 {
        // SAFETY: both arguments are plain `DWORD`s and the method writes
        // nothing through a pointer.
        unsafe { ((*self.jadwal()).dai_hala)(self.0, hala, qeema) }
    }

    /// `GetRenderState`, which fails on a device created `D3DCREATE_PUREDEVICE`.
    fn ijlib_hala(self, hala: u32) -> Option<u32> {
        let mut qeema: u32 = 0;
        // SAFETY: the out-parameter addresses one live, aligned `DWORD`.
        let natija = unsafe { ((*self.jadwal()).ijlib_hala)(self.0, hala, &raw mut qeema) };
        najah(natija).then_some(qeema)
    }

    /// `SetTextureStageState`.
    fn dai_hala_marhala(self, marhala: u32, hala: u32, qeema: u32) -> Natija8 {
        // SAFETY: three plain `DWORD`s; nothing is written through a pointer.
        unsafe { ((*self.jadwal()).dai_hala_marhala)(self.0, marhala, hala, qeema) }
    }

    /// `SetTexture`, with a null pointer meaning "no texture on this stage".
    fn dai_nasij(self, marhala: u32, nasij: *mut c_void) -> Natija8 {
        // SAFETY: `nasij` is either null or a live `IDirect3DBaseTexture8` this
        // backend created; the runtime takes its own reference.
        unsafe { ((*self.jadwal()).dai_nasij)(self.0, marhala, nasij) }
    }

    /// `SetVertexShader`, which in Direct3D 8 takes an FVF code or a handle.
    fn dai_shifrat_ruus(self, ramz: u32) -> Natija8 {
        // SAFETY: one plain `DWORD`.
        unsafe { ((*self.jadwal()).dai_shifrat_ruus)(self.0, ramz) }
    }

    /// `SetPixelShader`, with zero meaning the fixed-function path.
    fn dai_shifrat_biksel(self, ramz: u32) -> Natija8 {
        // SAFETY: one plain `DWORD`.
        unsafe { ((*self.jadwal()).dai_shifrat_biksel)(self.0, ramz) }
    }

    /// `SetViewport`.
    fn dai_manzar(self, manzar: &Manzar8) -> Natija8 {
        // SAFETY: the pointer addresses a live local of `D3DVIEWPORT8`'s layout
        // that outlives the call, and the runtime only reads it.
        unsafe { ((*self.jadwal()).dai_manzar)(self.0, core::ptr::from_ref(manzar)) }
    }

    /// `BeginScene`.
    fn ibda_mashhad(self) -> Natija8 {
        // SAFETY: the method takes only `this`.
        unsafe { ((*self.jadwal()).ibda_mashhad)(self.0) }
    }

    /// `EndScene`.
    fn inhi_mashhad(self) -> Natija8 {
        // SAFETY: the method takes only `this`.
        unsafe { ((*self.jadwal()).inhi_mashhad)(self.0) }
    }

    /// `CreateStateBlock`.
    fn insha_kutla(self, naw: u32) -> Option<u32> {
        let mut ramz: u32 = 0;
        // SAFETY: the out-parameter addresses one live, aligned `DWORD`.
        let natija = unsafe { ((*self.jadwal()).insha_kutla)(self.0, naw, &raw mut ramz) };
        (najah(natija) && ramz != 0).then_some(ramz)
    }

    /// `CaptureStateBlock`.
    fn iltiqat_kutla(self, ramz: u32) -> Natija8 {
        // SAFETY: one plain `DWORD` token this backend created.
        unsafe { ((*self.jadwal()).iltiqat_kutla)(self.0, ramz) }
    }

    /// `ApplyStateBlock`.
    fn tatbiq_kutla(self, ramz: u32) -> Natija8 {
        // SAFETY: one plain `DWORD` token this backend created.
        unsafe { ((*self.jadwal()).tatbiq_kutla)(self.0, ramz) }
    }

    /// `DeleteStateBlock`.
    fn hadhf_kutla(self, ramz: u32) -> Natija8 {
        // SAFETY: one plain `DWORD` token this backend created and has not
        // deleted.
        unsafe { ((*self.jadwal()).hadhf_kutla)(self.0, ramz) }
    }

    /// `GetBackBuffer`, which takes a reference the caller must release.
    fn khalfiya(self, fahras: u32) -> Option<Sath8> {
        let mut khaam: *mut c_void = core::ptr::null_mut();
        // SAFETY: the out-parameter addresses a live local initialised to null,
        // which the runtime overwrites with an owned reference on success.
        let natija =
            unsafe { ((*self.jadwal()).khalfiya)(self.0, fahras, KHALFIYA_UHADIYA, &raw mut khaam) };
        if !najah(natija) || khaam.is_null() {
            return None;
        }
        // SAFETY: the runtime just wrote a live `IDirect3DSurface8` with a
        // reference this call now owns.
        Some(unsafe { Sath8::jadeed(khaam) })
    }

    /// `GetRenderTarget`, which takes a reference the caller must release.
    fn ijlib_hadaf(self) -> Option<Sath8> {
        let mut khaam: *mut c_void = core::ptr::null_mut();
        // SAFETY: as `khalfiya`.
        let natija = unsafe { ((*self.jadwal()).ijlib_hadaf)(self.0, &raw mut khaam) };
        if !najah(natija) || khaam.is_null() {
            return None;
        }
        // SAFETY: as `khalfiya`.
        Some(unsafe { Sath8::jadeed(khaam) })
    }

    /// `GetDepthStencilSurface`, which answers nothing when there is no depth
    /// buffer — a legitimate configuration rather than a failure.
    fn ijlib_sath_umq(self) -> Option<Sath8> {
        let mut khaam: *mut c_void = core::ptr::null_mut();
        // SAFETY: as `khalfiya`.
        let natija = unsafe { ((*self.jadwal()).ijlib_sath_umq)(self.0, &raw mut khaam) };
        if !najah(natija) || khaam.is_null() {
            return None;
        }
        // SAFETY: as `khalfiya`.
        Some(unsafe { Sath8::jadeed(khaam) })
    }

    /// `SetRenderTarget`, with a null depth surface meaning "no depth buffer".
    fn dai_hadaf(self, hadaf: *mut c_void, umq: *mut c_void) -> Natija8 {
        // SAFETY: both pointers are either null or live `IDirect3DSurface8`
        // instances the caller holds a reference to for the duration of the
        // call; the runtime takes its own.
        unsafe { ((*self.jadwal()).dai_hadaf)(self.0, hadaf, umq) }
    }

    /// `CreateTexture`, in the managed pool so it survives a device reset.
    fn insha_nasij(&self, ard: u32, irtifa: u32, sigha: u32) -> Option<Nasij8> {
        let mut khaam: *mut c_void = core::ptr::null_mut();
        // SAFETY: the out-parameter addresses a live local initialised to null;
        // every other argument is a plain integer.
        let natija = unsafe {
            ((*self.jadwal()).insha_nasij)(
                self.0, ard, irtifa, 1, 0, sigha, HAWD_MUDAR, &raw mut khaam,
            )
        };
        if !najah(natija) || khaam.is_null() {
            return None;
        }
        // SAFETY: the runtime just wrote a live `IDirect3DTexture8` with a
        // reference this call now owns.
        Some(unsafe { Nasij8::jadeed(khaam) })
    }

    /// `CreateImageSurface`, which always lands in system memory.
    fn insha_sath_sura(self, ard: u32, irtifa: u32, sigha: u32) -> Option<Sath8> {
        let mut khaam: *mut c_void = core::ptr::null_mut();
        // SAFETY: as `insha_nasij`.
        let natija =
            unsafe { ((*self.jadwal()).insha_sath_sura)(self.0, ard, irtifa, sigha, &raw mut khaam) };
        if !najah(natija) || khaam.is_null() {
            return None;
        }
        // SAFETY: as `insha_nasij`.
        Some(unsafe { Sath8::jadeed(khaam) })
    }

    /// `CopyRects` over the whole surface.
    ///
    /// The whole surface rather than the region that is wanted, and that is not
    /// laziness. Direct3D 8 permits a render target to be copied into system
    /// memory, and drivers of the period disagree about whether a *rectangle*
    /// of one may be — several accept the call and write nothing. Copying
    /// everything and cropping on the way out is the form every driver
    /// implements, and the crop is a row loop that was already there.
    fn nasakh_kaamil(self, masdar: &Sath8, hadaf: &Sath8) -> Natija8 {
        // SAFETY: both surfaces are live and held by the caller for the
        // duration of the call. A null rectangle array with a count of zero is
        // the documented spelling of "the whole surface", and the destination
        // point array is null for the same reason.
        unsafe {
            ((*self.jadwal()).nasakh_mustatilat)(
                self.0,
                masdar.0,
                core::ptr::null(),
                0,
                hadaf.0,
                core::ptr::null(),
            )
        }
    }

    /// `DrawPrimitiveUP`, one triangle list straight out of a Rust slice.
    ///
    /// No vertex buffer, and that is the point. A `D3DPOOL_DEFAULT` vertex
    /// buffer would have to be released before every `Reset` and recreated
    /// after it; user-pointer geometry is copied by the runtime as it is
    /// submitted, so this backend holds no resource a reset can invalidate
    /// except its state block, which the `Reset` hook releases.
    fn irsim_mubashir(self, muthallathat: u32, ruus: &[RasQadim]) -> Natija8 {
        // SAFETY: `ruus` is a live slice of `#[repr(C)]` values with no padding
        // and no uninitialised byte, `muthallathat` is `ruus.len() / 3`, which
        // the caller establishes, and the stride is `RasQadim`'s own size. The
        // runtime reads exactly `muthallathat * 3 * stride` bytes, which is at
        // most the slice's length.
        unsafe {
            ((*self.jadwal()).irsim_mubashir)(
                self.0,
                RASM_MUTHALLATHAT,
                muthallathat,
                ruus.as_ptr().cast::<c_void>(),
                KHATWAT_RAS_QADIM,
            )
        }
    }
}

/// A live `IDirect3DSurface8`, released when it is dropped.
///
/// A guard rather than a raw pointer because every path that acquires one has
/// an early return in it: a `GetBackBuffer` whose description will not read, a
/// `CopyRects` that fails, a lock that returns no pointer. Each of those would
/// otherwise leak a reference per frame, which is the classic overlay bug whose
/// symptom is a game that runs for ten minutes and then does not.
#[derive(Debug)]
struct Sath8(*mut c_void);

impl Sath8 {
    /// Takes ownership of a reference the runtime just wrote.
    ///
    /// # Safety
    ///
    /// `khaam` must be a live `IDirect3DSurface8` carrying a reference this
    /// value is now responsible for releasing exactly once.
    const unsafe fn jadeed(khaam: *mut c_void) -> Self {
        Self(khaam)
    }

    /// The instance's method table.
    const fn jadwal(&self) -> *const JadwalSath8 {
        // SAFETY: this type's invariant is a live COM interface, whose first
        // machine word is its method table pointer.
        unsafe { self.0.cast::<*const JadwalSath8>().read() }
    }

    /// `GetDesc`.
    fn wasf(&self) -> Option<WasfSath8> {
        let mut wasf = WasfSath8::default();
        // SAFETY: the out-parameter addresses a live local of exactly the
        // layout `d3d8.h` gives `D3DSURFACE_DESC`.
        let natija = unsafe { ((*self.jadwal()).wasf)(self.0, &raw mut wasf) };
        najah(natija).then_some(wasf)
    }

    /// `LockRect` over the whole surface, for reading.
    fn aqfil_lil_qira(&self) -> Option<MustatilMaqful8> {
        let mut maqful = MustatilMaqful8::default();
        // SAFETY: the out-parameter addresses a live local; a null rectangle is
        // the documented spelling of "the whole surface".
        let natija = unsafe {
            ((*self.jadwal()).aqfil)(self.0, &raw mut maqful, core::ptr::null(), QUFL_QIRA_FAQAT)
        };
        if !najah(natija) || maqful.bayt.is_null() {
            return None;
        }
        Some(maqful)
    }

    /// `UnlockRect`.
    fn ifta(&self) -> Natija8 {
        // SAFETY: the method takes only `this`, and this is called exactly once
        // per successful lock.
        unsafe { ((*self.jadwal()).ifta)(self.0) }
    }
}

impl Drop for Sath8 {
    fn drop(&mut self) {
        // SAFETY: this type owns exactly one reference, taken when it was
        // constructed and given back here, once, because `Sath8` is neither
        // `Clone` nor `Copy` and the field is not reachable afterwards.
        unsafe {
            let _ = ((*self.jadwal()).itlaq)(self.0);
        }
    }
}

/// A live `IDirect3DTexture8`, released when it is dropped.
#[derive(Debug)]
struct Nasij8(*mut c_void);

impl Nasij8 {
    /// Takes ownership of a reference the runtime just wrote.
    ///
    /// # Safety
    ///
    /// As [`Sath8::jadeed`], for `IDirect3DTexture8`.
    const unsafe fn jadeed(khaam: *mut c_void) -> Self {
        Self(khaam)
    }

    /// The instance's method table.
    const fn jadwal(&self) -> *const JadwalNasij8 {
        // SAFETY: this type's invariant is a live COM interface.
        unsafe { self.0.cast::<*const JadwalNasij8>().read() }
    }

    /// `LockRect` over the whole of mip level zero, for writing.
    fn aqfil(&self) -> Option<MustatilMaqful8> {
        let mut maqful = MustatilMaqful8::default();
        // SAFETY: the out-parameter addresses a live local; a null rectangle
        // means the whole level, and a flag of zero means a read-write lock,
        // which is what a managed texture's system copy accepts.
        let natija =
            unsafe { ((*self.jadwal()).aqfil)(self.0, 0, &raw mut maqful, core::ptr::null(), 0) };
        if !najah(natija) || maqful.bayt.is_null() {
            return None;
        }
        Some(maqful)
    }

    /// `UnlockRect` on mip level zero.
    fn ifta(&self) -> Natija8 {
        // SAFETY: called exactly once per successful lock, on the same level.
        unsafe { ((*self.jadwal()).ifta)(self.0, 0) }
    }
}

impl Drop for Nasij8 {
    fn drop(&mut self) {
        // SAFETY: as `Sath8::drop` — one reference, given back once.
        unsafe {
            let _ = ((*self.jadwal()).itlaq)(self.0);
        }
    }
}

// ---------------------------------------------------------------------------
// The geometry, and the colour that has nowhere else to be converted
// ---------------------------------------------------------------------------

/// One corner of one quad, in the layout `FVF_TABAQA` describes.
///
/// Twenty-eight bytes: four floats of pre-transformed position, one packed
/// colour, two floats of texture coordinate. `#[repr(C)]` is load-bearing — the
/// whole slice is handed to `DrawPrimitiveUP` and read by the fixed-function
/// input assembler at fixed offsets, so a layout the compiler were free to
/// reorder would put the colour where the position is expected.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RasQadim {
    /// Screen position and the reciprocal homogeneous w, which is always one.
    mawdi: [f32; 4],
    /// `D3DCOLOR`: alpha, red, green, blue, one byte each, high byte first.
    lawn: u32,
    /// Atlas coordinates, zero to one over the texture that was created.
    khareeta: [f32; 2],
}

/// How many bytes one vertex occupies, as `DrawPrimitiveUP`'s stride.
const KHATWAT_RAS_QADIM: u32 = 28;

// The stride the runtime is told and the type it is reading must agree, or the
// second vertex of every quad is read from the middle of the first.
const _: () = {
    assert!(
        size_of::<RasQadim>() == KHATWAT_RAS_QADIM as usize,
        "RasQadim is not twenty-eight bytes"
    );
    assert!(core::mem::offset_of!(RasQadim, mawdi) == 0, "the FVF wants position first");
    assert!(core::mem::offset_of!(RasQadim, lawn) == 16, "the FVF wants the colour after XYZRHW");
    assert!(
        core::mem::offset_of!(RasQadim, khareeta) == 20,
        "the FVF wants the texture coordinate last"
    );
};

/// One run of consecutive quads that all sample the atlas, or all do not.
///
/// The fixed-function pipeline decides "multiply the texture by the vertex
/// colour" or "take the vertex colour alone" with a texture stage state, not
/// with a per-vertex attribute the way every shader backend in this crate does.
/// Changing that state per quad would be one state change and one draw call per
/// glyph; changing it per run is one per plate boundary, and submission order —
/// which is the layering, because there is no depth buffer — is preserved
/// either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DufaaQadima {
    /// Whether this run's quads sample the atlas.
    masturat: bool,
    /// The first vertex, as an index into the scratch buffer.
    bidaya: usize,
    /// How many vertices, always a multiple of six.
    adad: usize,
}

/// One linear channel encoded to sRGB.
///
/// The real piecewise transfer function, matching the fragment stage in
/// [`crate::gl`] and the pixel shader in [`crate::d3d11`]. A 2.2 power here
/// would disagree with both in exactly the near-black range antialiased glyph
/// edges live in, which is where a difference is visible.
fn ila_sirgb(khatti: f32) -> f32 {
    let amin = khatti.clamp(0.0, 1.0);
    if amin <= 0.003_130_8 { amin * 12.92 } else { 1.055f32.mul_add(amin.powf(1.0 / 2.4), -0.055) }
}

/// A zero-to-one channel as the byte a `D3DCOLOR` carries.
fn bayt_min_f32(qeema: f32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is clamped into [0, 255] immediately before the conversion, so it is \
                  a non-negative integer below 256 by the time it is narrowed"
    )]
    {
        (qeema * 255.0).round().clamp(0.0, 255.0) as u8
    }
}

/// A premultiplied linear colour as the `D3DCOLOR` the fixed function pipeline
/// interpolates.
///
/// This is where the transfer function lives on this backend, and the reason it
/// lives here rather than in a shader is that there is no shader. The
/// arithmetic is the same one [`crate::d3d11`]'s pixel shader performs — divide
/// the alpha out, encode, multiply it back — moved to the CPU and paid once per
/// quad instead of once per fragment.
///
/// The blend that follows happens in the framebuffer's own encoding. That is
/// not an approximation of a physically correct composite; it is the composite
/// the game's own interface was drawn with, and matching the game is the target
/// on an API that predates the question.
pub(crate) fn lawn_d3d(lawn: [f32; 4]) -> u32 {
    let [ahmar, akhdar, azraq, shaffaf] = lawn_marmuz(lawn);
    let qanat = |qeema: f32| u32::from(bayt_min_f32(qeema));
    (qanat(shaffaf) << 24) | (qanat(ahmar) << 16) | (qanat(akhdar) << 8) | qanat(azraq)
}

/// A premultiplied **linear** colour as a premultiplied **encoded** one.
///
/// Divide the alpha out, apply the transfer function, multiply it back. Shared
/// with [`crate::gl_thabit`] rather than copied, and the sharing is the point:
/// the two fixed-function backends are making one decision — that the blend
/// happens in the framebuffer's own encoding, because neither pipeline has
/// anywhere to put a transfer function and because that is the space the games
/// of this generation blended their own interfaces in — and a second copy would
/// be a second place it could drift.
///
/// Skipping the un-premultiply is the mistake this exists to prevent. It leaves
/// every partly covered pixel at the colour a fully covered one would have,
/// which over an antialiased glyph edge reads as text bitten out of its own
/// outline.
#[must_use]
pub fn lawn_marmuz(lawn: [f32; 4]) -> [f32; 4] {
    let alfa = lawn.get(3).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    if alfa <= f32::EPSILON {
        // Nothing to un-premultiply by, and nothing that would be drawn. A
        // divide here would produce infinities that a driver turns into a
        // vertex colour of whatever the register held.
        return [0.0; 4];
    }
    let qanat = |fahras: usize| -> f32 {
        let khaam = lawn.get(fahras).copied().unwrap_or(0.0).clamp(0.0, alfa);
        ila_sirgb(khaam / alfa) * alfa
    };
    [qanat(0), qanat(1), qanat(2), alfa]
}

/// A pixel count as a float, for the vertex positions.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and glyph coordinates are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// Turns one batch into triangles and the runs they are drawn in.
///
/// Six vertices per quad rather than four and an index buffer: `DrawPrimitiveUP`
/// takes a vertex list and nothing else, and the alternative —
/// `DrawIndexedPrimitiveUP` — would need a base vertex per run for no saving a
/// batch this size can feel.
///
/// The half-pixel offset is applied here and nowhere else. Direct3D 8 samples a
/// pixel at its top-left corner rather than its centre, so a quad placed at
/// integer coordinates covers the pixel above and to the left of the one it was
/// measured for. Every glyph would be half a pixel out, which reads as a font
/// that is slightly soft rather than as a placement bug.
fn ibni_ruus(
    lawha: &LawhatRasm,
    nisbat: (f32, f32),
    ruus: &mut Vec<RasQadim>,
    dufaat: &mut Vec<DufaaQadima>,
) {
    ruus.clear();
    dufaat.clear();
    let (nisbat_u, nisbat_v) = nisbat;

    for qita in &lawha.qitaat {
        let QitaRasm { mawdi, khareeta, lawn } = *qita;
        if mawdi.ard == 0 || mawdi.irtifa == 0 {
            continue;
        }
        let yasar = madaa_f32(mawdi.yasar) - 0.5;
        let aala = madaa_f32(mawdi.aala) - 0.5;
        let yameen = yasar + madaa_f32(mawdi.ard);
        let asfal = aala + madaa_f32(mawdi.irtifa);
        let masturat = khareeta.is_some();
        let (u0, v0, u1, v1) = khareeta.map_or((0.0, 0.0, 0.0, 0.0), |khareeta| {
            (
                khareeta.yasar * nisbat_u,
                khareeta.aala * nisbat_v,
                (khareeta.yasar + khareeta.ard) * nisbat_u,
                (khareeta.aala + khareeta.irtifa) * nisbat_v,
            )
        });
        let packed = lawn_d3d(lawn);

        let ras = |s: f32, a: f32, u: f32, v: f32| RasQadim {
            mawdi: [s, a, 0.0, 1.0],
            lawn: packed,
            khareeta: [u, v],
        };
        let bidaya = ruus.len();
        ruus.extend_from_slice(&[
            ras(yasar, aala, u0, v0),
            ras(yameen, aala, u1, v0),
            ras(yameen, asfal, u1, v1),
            ras(yasar, aala, u0, v0),
            ras(yameen, asfal, u1, v1),
            ras(yasar, asfal, u0, v1),
        ]);

        match dufaat.last_mut() {
            Some(akhira) if akhira.masturat == masturat => {
                akhira.adad = akhira.adad.saturating_add(6);
            }
            _ => dufaat.push(DufaaQadima { masturat, bidaya, adad: 6 }),
        }
    }
}

// ---------------------------------------------------------------------------
// Reading a Direct3D 8 backbuffer
// ---------------------------------------------------------------------------

/// The name a Direct3D 8 backbuffer format goes by, for a refusal or a report.
const fn ism_sigha(sigha: u32) -> &'static str {
    match sigha {
        SIGHA_A8R8G8B8 => "D3DFMT_A8R8G8B8",
        SIGHA_X8R8G8B8 => "D3DFMT_X8R8G8B8",
        SIGHA_R5G6B5 => "D3DFMT_R5G6B5",
        SIGHA_X1R5G5B5 => "D3DFMT_X1R5G5B5",
        SIGHA_A1R5G5B5 => "D3DFMT_A1R5G5B5",
        SIGHA_A2R10G10B10 => "D3DFMT_A2R10G10B10",
        _ => "a D3DFORMAT this build does not name",
    }
}

/// How many bytes one pixel of a backbuffer format occupies.
///
/// [`None`] for a format a Direct3D 8 swap chain is not permitted to present,
/// which is what makes the capture path's refusal specific.
const fn bayt_lil_biksel8(sigha: u32) -> Option<usize> {
    match sigha {
        SIGHA_R5G6B5 | SIGHA_X1R5G5B5 | SIGHA_A1R5G5B5 => Some(2),
        SIGHA_A8R8G8B8 | SIGHA_X8R8G8B8 | SIGHA_A2R10G10B10 => Some(4),
        _ => None,
    }
}

/// What the recogniser is told the capture is in.
///
/// Always [`SighatSath::Bgra8`], and that is a statement about the *reader*
/// rather than about the surface — the same reading [`crate::gl`] makes for the
/// same reason. Direct3D 8 presents six formats, three of them sixteen bits
/// wide, and none of them survives being handed to a recogniser unconverted.
/// So [`ansikh_saf`] widens every one of them on the way out and this is what
/// comes back, on a 16-bit desktop and a 10-bit one alike.
///
/// # Errors
///
/// [`KhataTabaqa::SighaGhayrMaduma`] for a format outside the six, which a
/// backbuffer cannot legally be and which is therefore worth reporting as the
/// number it is rather than converting on a guess.
fn sigha_min_d3d8(sigha: u32) -> Result<SighatSath, KhataTabaqa> {
    if bayt_lil_biksel8(sigha).is_some() {
        Ok(SighatSath::Bgra8)
    } else {
        Err(KhataTabaqa::SighaGhayrMaduma { sigha: format!("D3DFORMAT({sigha})") })
    }
}

/// Five bits expanded to eight, by replication rather than by a shift alone.
///
/// `v << 3` maps 31 to 248 and leaves the top of the range unreachable, so a
/// white pixel in a 16-bit frame reads as light grey and the recogniser's
/// contrast normalisation starts from a picture that is already wrong. Copying
/// the high bits back into the low ones maps 31 to 255 exactly.
const fn khams_ila_thaman(qeema: u32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the expression is masked to eight bits by construction: a five-bit input \
                  shifted left three and or-ed with the same input shifted right two cannot \
                  exceed 255"
    )]
    {
        (((qeema & 0x1F) << 3) | ((qeema & 0x1F) >> 2)) as u8
    }
}

/// Six bits expanded to eight, by the same replication.
const fn sitt_ila_thaman(qeema: u32) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a six-bit input shifted left two and or-ed with the same input shifted right \
                  four cannot exceed 255"
    )]
    {
        (((qeema & 0x3F) << 2) | ((qeema & 0x3F) >> 4)) as u8
    }
}

/// One pixel of a locked backbuffer row, as BGRA8.
///
/// [`None`] when the row is shorter than the pixel it is being asked for, which
/// is what a driver reporting a pitch narrower than its own surface produces
/// and which the caller turns into a capture failure rather than reading past
/// the end of somebody's mapping.
fn biksel_bgra8(saf: &[u8], sigha: u32, fahras: usize) -> Option<[u8; 4]> {
    let saa = bayt_lil_biksel8(sigha)?;
    let bidaya = fahras.checked_mul(saa)?;
    let khaam = saf.get(bidaya..bidaya.checked_add(saa)?)?;

    if saa == 2 {
        let (Some(adna), Some(aala)) = (khaam.first(), khaam.get(1)) else {
            return None;
        };
        let qeema = u32::from(*adna) | (u32::from(*aala) << 8);
        return Some(match sigha {
            SIGHA_R5G6B5 => [
                khams_ila_thaman(qeema),
                sitt_ila_thaman(qeema >> 5),
                khams_ila_thaman(qeema >> 11),
                0xFF,
            ],
            // Both fifteen-bit formats carry the channels identically; only the
            // top bit differs, and on the `X` spelling it is undefined rather
            // than opaque, so it is forced rather than read.
            _ => [
                khams_ila_thaman(qeema),
                khams_ila_thaman(qeema >> 5),
                khams_ila_thaman(qeema >> 10),
                if sigha == SIGHA_A1R5G5B5 && (qeema & 0x8000) == 0 { 0x00 } else { 0xFF },
            ],
        });
    }

    let (Some(b0), Some(b1), Some(b2), Some(b3)) =
        (khaam.first(), khaam.get(1), khaam.get(2), khaam.get(3))
    else {
        return None;
    };
    if sigha == SIGHA_A2R10G10B10 {
        let qeema = u32::from(*b0)
            | (u32::from(*b1) << 8)
            | (u32::from(*b2) << 16)
            | (u32::from(*b3) << 24);
        // The top eight bits of each ten-bit channel. Two bits of precision are
        // dropped and the alternative — dithering on a render thread — would
        // cost more than the recogniser can use.
        let qanat = |izaha: u32| -> u8 {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the shift leaves at most eight bits after the mask, so the value is \
                          below 256 before it is narrowed"
            )]
            {
                ((qeema >> izaha) & 0x3FF).wrapping_shr(2) as u8
            }
        };
        // Two bits of alpha expanded by replication: 0, 85, 170, 255.
        let shaffaf = u8::try_from((qeema >> 30) & 0x3).unwrap_or(0).saturating_mul(0x55);
        return Some([qanat(0), qanat(10), qanat(20), shaffaf]);
    }

    // `A8R8G8B8` and `X8R8G8B8` are already blue, green, red, alpha in memory.
    // The `X` spelling's fourth byte is undefined, so it is forced opaque.
    Some([*b0, *b1, *b2, if sigha == SIGHA_X8R8G8B8 { 0xFF } else { *b3 }])
}

/// Copies one row of a region out of a locked surface, widening as it goes.
///
/// Returns whether the whole row was available. A short row is a driver that
/// reported a pitch it did not honour, and the caller turns that into
/// [`KhataTabaqa::IltiqatFashil`] rather than shipping a half-filled image the
/// recogniser would read as a blank screen.
fn ansikh_saf(saf: &[u8], sigha: u32, yasar: u32, ard: u32, kharj: &mut Vec<u8>) -> bool {
    let Ok(yasar) = usize::try_from(yasar) else {
        return false;
    };
    let Ok(ard) = usize::try_from(ard) else {
        return false;
    };
    for amud in 0..ard {
        let Some(fahras) = yasar.checked_add(amud) else {
            return false;
        };
        let Some(biksel) = biksel_bgra8(saf, sigha, fahras) else {
            return false;
        };
        kharj.extend_from_slice(&biksel);
    }
    true
}

// ---------------------------------------------------------------------------
// Trusting ninety-six hand-written slots, but verifying five of them
// ---------------------------------------------------------------------------

/// The render state the self-check writes and reads back.
///
/// `D3DRS_TEXTUREFACTOR` is chosen because nothing in this backend uses it and
/// because it is a full thirty-two-bit value rather than a boolean or a small
/// enumeration, so a slot that answered from the wrong state would have to
/// coincide across all thirty-two bits to pass.
const AAYINA_TAHAQQUQ: u32 = 0x5A_A5_3C_C3;

/// Exercises five slots whose answers are predictable, before any is replaced.
///
/// The method table in this file is transcribed from `d3d8.h` by hand because
/// nothing binds Direct3D 8. A transcription error is a call to the wrong
/// method inside somebody's game, so the table is checked against a device
/// rather than against a comment:
///
/// * `TestCooperativeLevel` at slot three must answer with one of the four
///   codes it is documented to return and nothing else;
/// * `GetCreationParameters` at slot nine must report the adapter and device
///   type the device was actually created with;
/// * `GetDisplayMode` at slot eight must report a mode with real dimensions;
/// * `SetRenderState` at fifty and `GetRenderState` at fifty-one must round
///   trip a thirty-two-bit value, which pins both;
/// * `CreateStateBlock` at fifty-seven and `DeleteStateBlock` at fifty-six must
///   produce and accept a token, which is the pair every draw depends on.
///
/// The fourth check is skipped on a device created `D3DCREATE_PUREDEVICE`,
/// where `GetRenderState` is documented to fail — which is itself why this
/// backend saves state with a block and never by reading it back.
///
/// # Errors
///
/// [`KhataTabaqa::JadwalGhayrMawjud`] naming the check that disagreed. It is
/// deliberately not [`KhataTabaqa::KhatfFashil`]: nothing has been hooked at
/// the point this runs, and the message a user sees should say the table could
/// not be trusted rather than that a hook failed.
fn tahaqquq_jadwal(jihaz: Jihaz8, mutawaqqa: Option<&MuallimatInsha8>) -> Result<(), KhataTabaqa> {
    let radd = |sabab: String| KhataTabaqa::JadwalGhayrMawjud {
        wajiha: "IDirect3DDevice8".to_owned(),
        sabab,
    };

    let taawun = jihaz.ikhtibar_taawun();
    if !matches!(taawun, 0 | D3DERR_DEVICELOST | D3DERR_DEVICENOTRESET | D3DERR_INVALIDCALL) {
        return Err(radd(format!(
            "TestCooperativeLevel answered {taawun:#010x}, which is not one of the codes the \
             method is documented to return; slot three is not the method this build thinks it is"
        )));
    }

    let Some(muallimat) = jihaz.muallimat_insha() else {
        return Err(radd(
            "GetCreationParameters refused, and it cannot: the method reads four fields the \
             runtime has held since the device was created"
                .to_owned(),
        ));
    };
    if muallimat.naw == 0 || muallimat.naw > 3 || muallimat.muhawwil > 63 {
        return Err(radd(format!(
            "GetCreationParameters reported adapter {} and device type {}, neither of which is a \
             value Direct3D 8 produces",
            muallimat.muhawwil, muallimat.naw
        )));
    }
    if let Some(mutawaqqa) = mutawaqqa
        && (mutawaqqa.muhawwil != muallimat.muhawwil || mutawaqqa.naw != muallimat.naw)
    {
        return Err(radd(format!(
            "this device was created on adapter {} as device type {} and GetCreationParameters \
             reports adapter {} and type {}",
            mutawaqqa.muhawwil, mutawaqqa.naw, muallimat.muhawwil, muallimat.naw
        )));
    }

    match jihaz.namat_ard() {
        Some(namat) if namat.ard > 0 && namat.irtifa > 0 && namat.ard <= 65_535 => {}
        Some(namat) => {
            return Err(radd(format!(
                "GetDisplayMode reported a {}×{} desktop, which is not a mode any adapter has",
                namat.ard, namat.irtifa
            )));
        }
        None => return Err(radd("GetDisplayMode refused".to_owned())),
    }

    let naqi = muallimat.aalam & INSHA_NAQI != 0;
    if !naqi {
        let sabiqa = jihaz.ijlib_hala(hala::AAMIL_NASIJ);
        if sabiqa.is_none() {
            return Err(radd(
                "GetRenderState refused on a device that was not created D3DCREATE_PUREDEVICE, \
                 where it is documented to work"
                    .to_owned(),
            ));
        }
        let _ = jihaz.dai_hala(hala::AAMIL_NASIJ, AAYINA_TAHAQQUQ);
        let maqru = jihaz.ijlib_hala(hala::AAMIL_NASIJ);
        if let Some(sabiqa) = sabiqa {
            let _ = jihaz.dai_hala(hala::AAMIL_NASIJ, sabiqa);
        }
        if maqru != Some(AAYINA_TAHAQQUQ) {
            return Err(radd(format!(
                "D3DRS_TEXTUREFACTOR was set to {AAYINA_TAHAQQUQ:#010x} and read back as {}; \
                 slots fifty and fifty-one are not SetRenderState and GetRenderState",
                maqru.map_or_else(|| "a refusal".to_owned(), |qeema| format!("{qeema:#010x}"))
            )));
        }
    }

    let Some(ramz) = jihaz.insha_kutla(KUTLA_KAAMILA) else {
        return Err(radd(
            "CreateStateBlock(D3DSBT_ALL) produced no token, and this backend cannot save the \
             game's state any other way on a device that may be pure"
                .to_owned(),
        ));
    };
    let hadhf = jihaz.hadhf_kutla(ramz);
    if !najah(hadhf) {
        return Err(radd(format!(
            "DeleteStateBlock refused the token CreateStateBlock had just produced ({hadhf:#010x})"
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// The most quads one batch may carry.
///
/// Sixteen thousand is more text than fits on a 4K screen at a readable size,
/// and six vertices of twenty-eight bytes each puts the scratch buffer at under
/// three mebibytes. A batch above it is a fault in whatever built it rather
/// than a frame worth drawing.
const AQSA_QITAAT_8: usize = 16_384;

/// The largest atlas this backend will hand to a Direct3D 8 device, per axis.
///
/// Four thousand and ninety-six is the ceiling the last generation of hardware
/// this API shipped for reached, and most of it stopped at two thousand and
/// forty-eight. The check exists because `CreateTexture` failing at eight
/// thousand is a resource failure the player experiences as an overlay that
/// draws nothing, and a refusal that names the number is something a bug report
/// can act on.
const SAQF_LAWHA_8: u32 = 4_096;

/// The Direct3D 8 overlay backend.
///
/// Holds the game's device with a reference of its own, one state block, the
/// atlas texture in the managed pool, and a system-memory surface the capture
/// path reads through. Nothing here lives in `D3DPOOL_DEFAULT`, which is what
/// makes a device reset survivable: the state block is released by the `Reset`
/// hook through [`Khattaf::atliq_sath`], and everything else is either managed
/// or in system memory and comes through untouched.
pub struct KhattafD3D8 {
    jihaz: Jihaz8,
    /// Whether the reference taken at construction has been given back.
    mutlaq: bool,
    /// Whether the device was created `D3DCREATE_PUREDEVICE`.
    naqi: bool,
    /// Whether the device was created `D3DCREATE_MULTITHREADED`.
    mutaadid: bool,
    /// The `D3DSBT_ALL` block the draw captures into and applies back.
    kutla: Option<u32>,
    /// The atlas, in the managed pool.
    nasij: Option<Nasij8>,
    /// The atlas page's declared dimensions, which the batch's coordinates are
    /// normalized against.
    qiyas_lawha: (u32, u32),
    /// The texture's real dimensions, which may be padded to powers of two.
    qiyas_nasij: (u32, u32),
    /// The system-memory surface `CopyRects` reads the frame into.
    marhala: Option<Sath8>,
    /// That surface's width, height and format.
    qiyas_marhala: (u32, u32, u32),
    /// The surface the last [`Khattaf::hayyi`] was given.
    sath: Option<WasfSath>,
    /// The vertex scratch buffer, reused across frames.
    ruus: Vec<RasQadim>,
    /// The runs that scratch buffer is drawn in.
    dufaat: Vec<DufaaQadima>,
    /// What has happened here, for the diagnostics bundle.
    athar: Vec<String>,
}

#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "the fields the lint names are COM interface pointers, which is exactly what the \
              claim below is about: they are not `Send` on their own and this type asserts that \
              moving them is sound because it never dereferences one off the thread that calls \
              into it. Rewriting them as a thread-safe type would be asserting something \
              stronger and false — a Direct3D 8 device is not thread-safe at the runtime's level"
)]
// SAFETY: every field is a pointer to a COM object, an integer or a `Vec`, none
// of which this type dereferences off the thread that calls into it. As with
// `crate::gl::KhattafGl`, that is a statement about memory and not about the
// API: a Direct3D 8 device created without `D3DCREATE_MULTITHREADED` may be
// used from one thread only, and moving this value to another thread and
// drawing there is undefined at the runtime's level even though it is sound at
// Rust's. The hook calls this only from inside `Present`, which runs on the
// thread the game renders on, and `mutaadid` records which of the two kinds of
// device is underneath so the capability report can say so.
unsafe impl Send for KhattafD3D8 {}

impl fmt::Debug for KhattafD3D8 {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        mukhraj
            .debug_struct("KhattafD3D8")
            .field("naqi", &self.naqi)
            .field("mutaadid", &self.mutaadid)
            .field("kutla", &self.kutla)
            .field("qiyas_lawha", &self.qiyas_lawha)
            .field("qiyas_nasij", &self.qiyas_nasij)
            .field("sath", &self.sath)
            .finish_non_exhaustive()
    }
}

impl KhattafD3D8 {
    /// Builds a backend around the device the hook was reached through.
    ///
    /// Takes a reference of its own, because the pointer arrives as the `this`
    /// of a `Present` call and is only guaranteed live for that call. Verifies
    /// the method table before anything else, because every later call in this
    /// file goes through it.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::JadwalGhayrMawjud`] when the table does not behave as
    /// `d3d8.h` says it should, which means this build must not call through
    /// it; see [`tahaqquq_jadwal`].
    ///
    /// # Safety
    ///
    /// `khaam` must point at a live `IDirect3DDevice8` instance. Nothing here
    /// can check that: the first machine word of whatever it addresses is read
    /// as a method table and called through.
    pub unsafe fn min_jihaz(khaam: *mut c_void) -> Result<Self, KhataTabaqa> {
        if khaam.is_null() {
            return Err(KhataTabaqa::JadwalGhayrMawjud {
                wajiha: "IDirect3DDevice8".to_owned(),
                sabab: "the present hook was reached with a null device".to_owned(),
            });
        }
        // SAFETY: the caller guarantees a live instance, which is exactly this
        // constructor's own precondition.
        let jihaz = unsafe { Jihaz8::jadeed(khaam) };

        tahaqquq_jadwal(jihaz, None)?;

        let muallimat = jihaz.muallimat_insha().ok_or_else(|| {
            KhataTabaqa::JadwalGhayrMawjud {
                wajiha: "IDirect3DDevice8".to_owned(),
                sabab: "GetCreationParameters refused after the table had been verified"
                    .to_owned(),
            }
        })?;
        let naqi = muallimat.aalam & INSHA_NAQI != 0;
        let mutaadid = muallimat.aalam & INSHA_MUTAADID != 0;

        let _ = jihaz.idafa();
        let athar = vec![format!(
            "Direct3D 8 device: adapter {}, type {}, behaviour {:#010x}, {} vertex \
             processing{}{}",
            muallimat.muhawwil,
            muallimat.naw,
            muallimat.aalam,
            if muallimat.aalam & INSHA_RUUS_ARIDA == 0 { "software or mixed" } else { "hardware" },
            if naqi { ", pure (no state may be read back)" } else { "" },
            if mutaadid { ", multithreaded" } else { "" }
        )];

        Ok(Self {
            jihaz,
            mutlaq: false,
            naqi,
            mutaadid,
            kutla: None,
            nasij: None,
            qiyas_lawha: (0, 0),
            qiyas_nasij: (0, 0),
            marhala: None,
            qiyas_marhala: (0, 0, 0),
            sath: None,
            ruus: Vec::new(),
            dufaat: Vec::new(),
            athar,
        })
    }

    /// Whether the device forbids reading its state back.
    #[must_use]
    pub const fn naqi(&self) -> bool {
        self.naqi
    }

    /// What has happened to this backend, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        &self.athar
    }

    /// The atlas coordinate scale, when the texture was padded past the page.
    fn nisbat_lawha(&self) -> (f32, f32) {
        let (lawha_ard, lawha_irtifa) = self.qiyas_lawha;
        let (nasij_ard, nasij_irtifa) = self.qiyas_nasij;
        if nasij_ard == 0 || nasij_irtifa == 0 {
            return (1.0, 1.0);
        }
        (
            madaa_f32(lawha_ard) / madaa_f32(nasij_ard),
            madaa_f32(lawha_irtifa) / madaa_f32(nasij_irtifa),
        )
    }

    /// Sets everything the draw needs and issues one call per run.
    ///
    /// Every state written here is inside what `D3DSBT_ALL` captures, which is
    /// what makes the restore complete. Adding a write to this function that
    /// falls outside a state block — a render target, a depth surface — without
    /// saving it beside the block is the one way this backend can corrupt a
    /// renderer.
    fn arsil_dufaat(&self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        let jihaz = self.jihaz;
        let nasij = self.nasij.as_ref().map_or(core::ptr::null_mut(), |nasij| nasij.0);
        if nasij.is_null() && self.dufaat.iter().any(|dufaa| dufaa.masturat) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the batch samples an atlas that has not been uploaded".to_owned(),
            });
        }

        let manzar = Manzar8 {
            s: 0,
            a: 0,
            ard: sath.ard,
            irtifa: sath.irtifa,
            adna_umq: 0.0,
            aqsa_umq: 1.0,
        };
        let natija = jihaz.dai_manzar(&manzar);
        if !najah(natija) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay viewport",
                sabab: format!("SetViewport refused a {}×{} viewport: {natija:#010x}", sath.ard, sath.irtifa),
            });
        }

        // Fixed function, no shaders of anybody's. `SetVertexShader` with an
        // FVF code is Direct3D 8's spelling of what Direct3D 9 split into
        // `SetFVF`, and it replaces whatever vertex shader handle the game had.
        let _ = jihaz.dai_shifrat_ruus(FVF_TABAQA);
        let _ = jihaz.dai_shifrat_biksel(0);

        for (hala, qeema) in [
            (hala::UMQ, UMQ_MUTLAQ),
            (hala::KITABAT_UMQ, 0),
            (hala::IKHTIBAR_SHAFAFIYA, 0),
            (hala::QALIB, 0),
            (hala::HADHF, HADHF_LA_SHAY),
            (hala::MIL, MIL_MUSMAT),
            (hala::TAZLIL, TAZLIL_MUTADARRIJ),
            (hala::DABAB, 0),
            (hala::IDAA, 0),
            (hala::LAMAAN, 0),
            (hala::TASHWISH, 0),
            (hala::MAZJ_RUUS, 0),
            (hala::MUSTAWAYAT_QASS, 0),
            (hala::KITABAT_LAWN, KITABAT_KAAMILA),
            // Clipping stays on. Pre-transformed vertices are clipped against
            // the viewport rather than a frustum, and a quad the batch builder
            // placed partly off-screen must be cut at the edge rather than
            // wrapped by whatever the rasteriser does with coordinates it was
            // told not to clip.
            (hala::QASS, 1),
            (hala::MAZJ, 1),
            (hala::MASDAR_MAZJ, MAZJ_WAHID),
            (hala::HADAF_MAZJ, MAZJ_MAQLUB),
            (hala::AMALIYAT_MAZJ, MAZJ_JAM),
        ] {
            let _ = jihaz.dai_hala(hala, qeema);
        }

        for (marhala, qeema) in [
            (marhala::MUAMIL_LAWN_1, MUAMIL_NASIJ),
            (marhala::MUAMIL_LAWN_2, MUAMIL_MUNTASHIR),
            (marhala::MUAMIL_SHAFAFIYA_1, MUAMIL_NASIJ),
            (marhala::MUAMIL_SHAFAFIYA_2, MUAMIL_MUNTASHIR),
            (marhala::FAHRAS_IHDATHIYAT, 0),
            (marhala::TAHWIL, TAHWIL_MUATTAL),
            (marhala::UNWAN_S, UNWAN_MUQAYYAD),
            (marhala::UNWAN_A, UNWAN_MUQAYYAD),
            (marhala::MURASHAH_TAKBIR, MURASHAH_KHATTI),
            (marhala::MURASHAH_TASGHIR, MURASHAH_KHATTI),
            (marhala::MURASHAH_TADARRUJ, MURASHAH_LA_SHAY),
        ] {
            let _ = jihaz.dai_hala_marhala(0, marhala, qeema);
        }
        // The second stage is disabled rather than left alone: a game with a
        // detail map still configured there would multiply it over every glyph.
        let _ = jihaz.dai_hala_marhala(1, marhala::AMALIYAT_LAWN, AMAL_MUATTAL);
        let _ = jihaz.dai_hala_marhala(1, marhala::AMALIYAT_SHAFAFIYA, AMAL_MUATTAL);
        let _ = jihaz.dai_nasij(0, nasij);

        let mut masturat_alaan: Option<bool> = None;
        for dufaa in &self.dufaat {
            if masturat_alaan != Some(dufaa.masturat) {
                let amal = if dufaa.masturat { AMAL_DARB } else { AMAL_IKHTAR_2 };
                let _ = jihaz.dai_hala_marhala(0, marhala::AMALIYAT_LAWN, amal);
                let _ = jihaz.dai_hala_marhala(0, marhala::AMALIYAT_SHAFAFIYA, amal);
                masturat_alaan = Some(dufaa.masturat);
            }

            let Some(shariha) = self.ruus.get(dufaa.bidaya..dufaa.bidaya + dufaa.adad) else {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay geometry",
                    sabab: "a draw run names vertices the scratch buffer does not hold".to_owned(),
                });
            };
            #[expect(
                clippy::integer_division,
                reason = "the run's vertex count is a multiple of six by construction in \
                          `ibni_ruus`, so dividing by three is exact and names the triangle count \
                          `DrawPrimitiveUP` takes"
            )]
            let muthallathat = shariha.len() / 3;
            let Ok(muthallathat) = u32::try_from(muthallathat) else {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay geometry",
                    sabab: "a draw run holds more triangles than a UINT can express".to_owned(),
                });
            };
            if muthallathat == 0 {
                continue;
            }
            let natija = jihaz.irsim_mubashir(muthallathat, shariha);
            if !najah(natija) {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay draw call",
                    sabab: format!("DrawPrimitiveUP refused {muthallathat} triangles: {natija:#010x}"),
                });
            }
        }

        Ok(())
    }

    /// Points the device at the backbuffer, draws, and gives the scene back.
    ///
    /// `BeginScene` is called here because the overlay runs from inside
    /// `Present`, which is after the game's own `EndScene`. A game that is
    /// still inside a scene at that point — which a few engines are, presenting
    /// from a nested call — answers `D3DERR_INVALIDCALL`, and that is not a
    /// failure: it means drawing is already permitted and the matching
    /// `EndScene` is the game's to call, not this backend's.
    fn arsim_dakhil(&self, sath: WasfSath, hadaf: &Sath8) -> Result<(), KhataTabaqa> {
        let jihaz = self.jihaz;
        let natija = jihaz.dai_hadaf(hadaf.0, core::ptr::null_mut());
        if !najah(natija) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay render target",
                sabab: format!("SetRenderTarget refused the backbuffer: {natija:#010x}"),
            });
        }

        let ibtida = jihaz.ibda_mashhad();
        let bada = najah(ibtida);
        if !bada && ibtida != D3DERR_INVALIDCALL {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay scene",
                sabab: format!("BeginScene refused: {ibtida:#010x}"),
            });
        }

        let natija = self.arsil_dufaat(sath);

        if bada {
            let _ = jihaz.inhi_mashhad();
        }
        natija
    }

    /// Creates the atlas texture and writes the page into it.
    ///
    /// Padded to powers of two when the card demands it. Hardware from this
    /// API's own generation frequently reports `D3DPTEXTURECAPS_POW2`, and a
    /// `CreateTexture` refused at 1000×1000 is an overlay that draws no glyphs
    /// at all. The page's own dimensions are kept separately so
    /// [`KhattafD3D8::nisbat_lawha`] can scale the batch's coordinates onto the
    /// larger texture — without which every glyph would sample the wrong
    /// rectangle, which is a wrong letter rather than a missing one.
    fn ansha_nasij(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        let mut nasij = self.jihaz.insha_nasij(ard, irtifa, SIGHA_A8R8G8B8);
        let mut qiyas = (ard, irtifa);
        if nasij.is_none() {
            let mubattan = (quwwat_ithnayn(ard), quwwat_ithnayn(irtifa));
            if mubattan != qiyas {
                nasij = self.jihaz.insha_nasij(mubattan.0, mubattan.1, SIGHA_A8R8G8B8);
                if nasij.is_some() {
                    qiyas = mubattan;
                    self.athar.push(format!(
                        "the device refused a {ard}×{irtifa} atlas and took {}×{}, so this card \
                         wants powers of two",
                        mubattan.0, mubattan.1
                    ));
                }
            }
        }
        let Some(nasij) = nasij else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "CreateTexture refused a {ard}×{irtifa} A8R8G8B8 texture in the managed pool, \
                     and refused {}×{} as well",
                    quwwat_ithnayn(ard),
                    quwwat_ithnayn(irtifa)
                ),
            });
        };

        let Some(maqful) = nasij.aqfil() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "LockRect produced no pointer for a texture that had just been created"
                    .to_owned(),
            });
        };
        let khatwa = usize::try_from(maqful.khatwa).unwrap_or(0);
        let saf_matlub = usize::try_from(qiyas.0).unwrap_or(0).saturating_mul(4);
        if khatwa < saf_matlub || saf_matlub == 0 {
            let _ = nasij.ifta();
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "the locked atlas reports a {khatwa}-byte pitch for a {saf_matlub}-byte row"
                ),
            });
        }

        // The page arrives red-green-blue-alpha and `D3DFMT_A8R8G8B8` is
        // blue-green-red-alpha in memory, so the two colour channels are
        // exchanged on the way in. Uploading without the exchange produces
        // glyphs that are the right shape in the wrong colour, which on a
        // premultiplied coverage page — where all three channels carry the same
        // value — is invisible until somebody tints the text.
        let mut saf = vec![0u8; khatwa];
        let masdar_ard = usize::try_from(ard).unwrap_or(0);
        let masdar_irtifa = usize::try_from(irtifa).unwrap_or(0);
        let hadaf_irtifa = usize::try_from(qiyas.1).unwrap_or(0);
        for satr in 0..hadaf_irtifa {
            saf.fill(0);
            if satr < masdar_irtifa {
                for amud in 0..masdar_ard {
                    let masdar =
                        satr.saturating_mul(masdar_ard).saturating_add(amud).saturating_mul(4);
                    let hadaf = amud.saturating_mul(4);
                    let (Some(khaam), Some(makan)) =
                        (bayt.get(masdar..masdar + 4), saf.get_mut(hadaf..hadaf + 4))
                    else {
                        continue;
                    };
                    let (Some(ahmar), Some(akhdar), Some(azraq), Some(shaffaf)) =
                        (khaam.first(), khaam.get(1), khaam.get(2), khaam.get(3))
                    else {
                        continue;
                    };
                    makan.copy_from_slice(&[*azraq, *akhdar, *ahmar, *shaffaf]);
                }
            }
            // SAFETY: the lock covers `khatwa` bytes for each of `qiyas.1`
            // rows — that is what a locked texture level is — and `satr` is
            // below `qiyas.1`, so the destination is inside the mapping.
            // `saf` is exactly `khatwa` bytes and is a distinct allocation.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    saf.as_ptr(),
                    maqful.bayt.cast::<u8>().add(satr.saturating_mul(khatwa)),
                    khatwa,
                );
            }
        }

        let ifta = nasij.ifta();
        if !najah(ifta) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("UnlockRect refused after the atlas was written: {ifta:#010x}"),
            });
        }

        // Both replaced together: a texture assigned while the old dimensions
        // were kept would have every glyph sampling the previous page's grid.
        self.nasij = Some(nasij);
        self.qiyas_lawha = (ard, irtifa);
        self.qiyas_nasij = qiyas;
        Ok(())
    }

    /// The system-memory surface the capture reads through, sized to the frame.
    ///
    /// Cached across frames because it is a full-surface allocation — eight
    /// mebibytes at 1920×1080 — and the capture path is called whenever the
    /// change-detection gate opens, which on a dialogue-heavy scene is often.
    /// It lives in system memory, so a device reset does not invalidate it; it
    /// is dropped in [`Khattaf::atliq_sath`] anyway, because a reset is exactly
    /// when its dimensions stop matching.
    fn marhala_li(&mut self, ard: u32, irtifa: u32, sigha: u32) -> Result<(), KhataTabaqa> {
        if self.marhala.is_some() && self.qiyas_marhala == (ard, irtifa, sigha) {
            return Ok(());
        }
        self.marhala = None;
        let Some(sath) = self.jihaz.insha_sath_sura(ard, irtifa, sigha) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "CreateImageSurface refused a {ard}×{irtifa} {} system-memory surface",
                    ism_sigha(sigha)
                ),
            });
        };
        self.marhala = Some(sath);
        self.qiyas_marhala = (ard, irtifa, sigha);
        Ok(())
    }
}

/// The next power of two at or above a dimension, capped at this build's ceiling.
const fn quwwat_ithnayn(qeema: u32) -> u32 {
    let mut natija: u32 = 1;
    while natija < qeema && natija < SAQF_LAWHA_8 {
        natija = natija.saturating_mul(2);
    }
    natija
}

impl Khattaf for KhattafD3D8 {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Direct3D8
    }

    /// Releases the state block, which is what a device reset destroys.
    ///
    /// Called from the `Reset` hook before the call reaches the runtime, for
    /// the same reason [`crate::d3d11`] is called from `ResizeBuffers`: a state
    /// block is device state, `Reset` invalidates every token, and applying a
    /// stale one afterwards restores whatever the runtime now has at that
    /// index. The atlas and the read-back surface survive — one is managed and
    /// the other is in system memory — but the read-back surface is dropped
    /// too, because a reset is precisely when its dimensions stop matching.
    fn atliq_sath(&mut self) -> Result<(), KhataTabaqa> {
        if let Some(kutla) = self.kutla.take() {
            let natija = self.jihaz.hadhf_kutla(kutla);
            if !najah(natija) {
                self.athar.push(format!(
                    "DeleteStateBlock answered {natija:#010x} before a reset; the token is \
                     forgotten either way, because a reset invalidates it"
                ));
            }
        }
        self.marhala = None;
        self.qiyas_marhala = (0, 0, 0);
        self.sath = None;
        Ok(())
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        let Some(khalfiya) = self.jihaz.khalfiya(0) else {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: "the device would not hand over backbuffer zero, which is what it answers \
                        between a reset and the mode change completing"
                    .to_owned(),
            });
        };
        let Some(wasf) = khalfiya.wasf() else {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: "the backbuffer would not describe itself".to_owned(),
            });
        };
        if wasf.ard == 0 || wasf.irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("the backbuffer reports a {}×{} surface", wasf.ard, wasf.irtifa),
            });
        }

        Ok(WasfSath {
            ard: wasf.ard,
            irtifa: wasf.irtifa,
            sigha: sigha_min_d3d8(wasf.sigha)?,
            // Direct3D 8 has no sRGB render target and no `SRGBWRITEENABLE`,
            // which Direct3D 9 added. Nothing encodes on write, so this is
            // false in the operational sense the field carries — "the hardware
            // will not apply the transfer function for you" — and `lawn_d3d`
            // applies it on the CPU instead.
            sirgb: false,
        })
    }

    /// Creates the state block, which is the only object the surface's identity
    /// affects.
    ///
    /// Nothing this backend owns is sized against the surface: the geometry is
    /// user-pointer, the atlas is sized by the font, and the read-back surface
    /// is built on demand. So a resolution change creates nothing new — it
    /// re-creates the state block that the `Reset` hook released, and records
    /// the surface everything else is checked against.
    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        if sath.ard == 0 || sath.irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("a {}×{} surface has no pixels to draw on", sath.ard, sath.irtifa),
            });
        }
        if self.kutla.is_none() {
            let Some(kutla) = self.jihaz.insha_kutla(KUTLA_KAAMILA) else {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay state block",
                    sabab: "CreateStateBlock(D3DSBT_ALL) produced no token; without one this \
                            backend cannot put the game's pipeline back and will not draw"
                        .to_owned(),
                });
            };
            self.kutla = Some(kutla);
        }
        if self.sath != Some(sath) {
            self.athar.push(format!(
                "surface {}×{} {}",
                sath.ard,
                sath.irtifa,
                sath.sigha.ism()
            ));
        }
        self.sath = Some(sath);
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("a {ard}×{irtifa} atlas has no texels to upload"),
            });
        }
        if ard > SAQF_LAWHA_8 || irtifa > SAQF_LAWHA_8 {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas edge",
                qeema: u64::from(ard.max(irtifa)),
                saqf: u64::from(SAQF_LAWHA_8),
            });
        }
        let matlub = u64::from(ard).saturating_mul(u64::from(irtifa)).saturating_mul(4);
        if crate::khata::tul_u64(bayt.len()) < matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "a {ard}×{irtifa} RGBA8 atlas needs {matlub} bytes and {} were supplied",
                    bayt.len()
                ),
            });
        }

        self.nasij = None;
        self.qiyas_lawha = (0, 0);
        self.qiyas_nasij = (0, 0);
        self.ansha_nasij(bayt, ard, irtifa)
    }

    /// Captures the pipeline into a state block, draws, and applies it back.
    ///
    /// The capture is what makes the draw permissible at all: it happens before
    /// a single state is written, and a capture that fails means the overlay
    /// stands down for the frame rather than changing state it could not put
    /// back. That ordering is the whole of this backend's safety argument and
    /// is why the failure is [`KhataTabaqa::MawridFashil`] — recoverable,
    /// nothing changed — rather than terminal.
    ///
    /// A failed *apply* is the opposite and is terminal, unless the device was
    /// lost while the overlay drew, which is a resize in disguise and is
    /// reported as one.
    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if lawha.qitaat.len() > AQSA_QITAAT_8 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "draw batch",
                sabab: format!(
                    "the batch carries {} quads, above this build's ceiling of {AQSA_QITAAT_8}",
                    lawha.qitaat.len()
                ),
            });
        }
        if self.sath.is_none() {
            // Reached after `atliq_sath` let the game reset. The caller only
            // calls `hayyi` when the surface description changes, and a reset
            // that keeps the same width, height and format changes nothing it
            // can see — so the rebuild happens here.
            self.hayyi(lawha.sath)?;
        }
        if self.sath != Some(lawha.sath) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay batch",
                sabab: "the batch was positioned against a surface this backend is not currently \
                        built for"
                    .to_owned(),
            });
        }
        let Some(kutla) = self.kutla else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay state block",
                sabab: "the backend was asked to draw with no state block, so nothing it changed \
                        could be put back"
                    .to_owned(),
            });
        };

        let mut ruus = core::mem::take(&mut self.ruus);
        let mut dufaat = core::mem::take(&mut self.dufaat);
        ibni_ruus(lawha, self.nisbat_lawha(), &mut ruus, &mut dufaat);
        self.ruus = ruus;
        self.dufaat = dufaat;
        if self.dufaat.is_empty() {
            // Every quad had a zero dimension, which a shaped line with no
            // visible glyphs produces. Capturing and applying a whole state
            // block to draw nothing would be the expensive way to do nothing.
            return Ok(());
        }

        // Acquired before anything is captured or changed, so a device that
        // will not name its own surfaces costs a skipped frame rather than a
        // half-restored pipeline.
        let (Some(hadaf_sabiq), Some(khalfiya)) =
            (self.jihaz.ijlib_hadaf(), self.jihaz.khalfiya(0))
        else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay render target",
                sabab: "the device would not name its current render target or its backbuffer"
                    .to_owned(),
            });
        };
        let umq_sabiq = self.jihaz.ijlib_sath_umq();

        let iltiqat = self.jihaz.iltiqat_kutla(kutla);
        if !najah(iltiqat) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay state block",
                sabab: format!(
                    "CaptureStateBlock answered {iltiqat:#010x}; nothing was changed, because a \
                     pipeline that cannot be saved is one this backend does not touch"
                ),
            });
        }

        let natija = self.arsim_dakhil(lawha.sath, &khalfiya);

        // Unconditional, and before the result is inspected. There is no error
        // this backend can return that makes leaving the game's pipeline
        // pointed at Taarib's vertices the better outcome.
        let mut aalik: Vec<String> = Vec::new();
        let umq_khaam = umq_sabiq.as_ref().map_or(core::ptr::null_mut(), |sath| sath.0);
        let istirdad = self.jihaz.dai_hadaf(hadaf_sabiq.0, umq_khaam);
        if !najah(istirdad) {
            aalik.push(format!("SetRenderTarget answered {istirdad:#010x}"));
        }
        let tatbiq = self.jihaz.tatbiq_kutla(kutla);
        if !najah(tatbiq) {
            aalik.push(format!("ApplyStateBlock answered {tatbiq:#010x}"));
        }
        drop(khalfiya);
        drop(umq_sabiq);
        drop(hadaf_sabiq);

        if aalik.is_empty() {
            return natija;
        }
        let sabab = aalik.join("; ");
        if matches!(istirdad | tatbiq, D3DERR_DEVICELOST | D3DERR_DEVICENOTRESET)
            || matches!(
                self.jihaz.ikhtibar_taawun(),
                D3DERR_DEVICELOST | D3DERR_DEVICENOTRESET
            )
        {
            // The device went away while the overlay drew. Nothing was left
            // wrong, because everything the state block described is about to
            // be recreated by the game's own `Reset` — which the hook will
            // catch, which is what drops the block.
            self.sath = None;
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("the device was lost while the overlay drew: {sabab}"),
            });
        }
        Err(KhataTabaqa::HalaGhayrMustaada {
            hala: "the Direct3D 8 device's render target and its D3DSBT_ALL state block",
            sabab,
        })
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = Khattaf::sath(self)?;
        let wasf_mintaqa = || {
            format!("{}×{} at {},{}", mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala)
        };
        let (Some(yameen), Some(asfal)) = (
            mintaqa.yasar.checked_add(mintaqa.ard),
            mintaqa.aala.checked_add(mintaqa.irtifa),
        ) else {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: wasf_mintaqa(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        };
        if mintaqa.ard == 0 || mintaqa.irtifa == 0 || yameen > sath.ard || asfal > sath.irtifa {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: wasf_mintaqa(),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        }

        let Some(khalfiya) = self.jihaz.khalfiya(0) else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the device would not hand over backbuffer zero".to_owned(),
            });
        };
        let Some(wasf) = khalfiya.wasf() else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the backbuffer would not describe itself".to_owned(),
            });
        };
        if wasf.taadud != TAADUD_LA_SHAY {
            // Direct3D 8 has no `ResolveSubresource` and `CopyRects` refuses a
            // multisampled source outright. There is no path from a
            // multisampled backbuffer to system memory in this API, so the
            // overlay draws and does not read. Said plainly rather than
            // returning an empty image the recogniser would find nothing in.
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "this game presents a multisampled backbuffer (D3DMULTISAMPLE_TYPE {}), and \
                     Direct3D 8 has no way to resolve one into system memory; the overlay can \
                     draw over this game and cannot read its screen",
                    wasf.taadud
                ),
            });
        }
        if bayt_lil_biksel8(wasf.sigha).is_none() {
            return Err(KhataTabaqa::SighaGhayrMaduma { sigha: format!("D3DFORMAT({})", wasf.sigha) });
        }

        self.marhala_li(wasf.ard, wasf.irtifa, wasf.sigha)?;
        let Some(marhala) = self.marhala.as_ref() else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back surface was not produced".to_owned(),
            });
        };
        let nasakh = self.jihaz.nasakh_kaamil(&khalfiya, marhala);
        if !najah(nasakh) {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "CopyRects refused to copy a {} backbuffer into system memory: {nasakh:#010x}",
                    ism_sigha(wasf.sigha)
                ),
            });
        }

        let Some(maqful) = marhala.aqfil_lil_qira() else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back surface would not lock".to_owned(),
            });
        };
        let natija = jami_sufuf8(&maqful, wasf, mintaqa);
        let ifta = marhala.ifta();
        if !najah(ifta) {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!("UnlockRect refused after the frame was read: {ifta:#010x}"),
            });
        }
        natija
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        // Idempotent by construction: every resource is an `Option` and the
        // device reference is released behind a flag. The second call is the
        // common one — `Tabaqa::shaghghil` tears a backend down when `hayyi`
        // failed, and then the caller drops it.
        let _ = Khattaf::atliq_sath(self);
        self.nasij = None;
        self.qiyas_lawha = (0, 0);
        self.qiyas_nasij = (0, 0);
        self.ruus = Vec::new();
        self.dufaat = Vec::new();
        if !self.mutlaq {
            self.mutlaq = true;
            let _ = self.jihaz.itlaq();
        }
        Ok(())
    }
}

/// Copies a region out of a locked read-back surface, widening as it goes.
///
/// The pitch is the distance between two rows in the mapping and is **not** the
/// row's width: drivers round it up to whatever alignment they prefer. A read
/// that treats it as the width produces an image that shears progressively to
/// one side, which looks like a recogniser bug rather than a stride bug here.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the mapping's pitch is shorter than one
/// row of the surface, or when a row runs out before the region does — both of
/// which mean the copy did not produce what was asked for.
fn jami_sufuf8(
    maqful: &MustatilMaqful8,
    wasf: WasfSath8,
    mintaqa: MustatilBiksel,
) -> Result<Vec<u8>, KhataTabaqa> {
    let Some(saa) = bayt_lil_biksel8(wasf.sigha) else {
        return Err(KhataTabaqa::SighaGhayrMaduma { sigha: format!("D3DFORMAT({})", wasf.sigha) });
    };
    let khatwa = usize::try_from(maqful.khatwa).unwrap_or(0);
    let saf_matlub = usize::try_from(wasf.ard).unwrap_or(0).saturating_mul(saa);
    if khatwa < saf_matlub || saf_matlub == 0 {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: format!(
                "the read-back surface reports a {khatwa}-byte pitch for a {saf_matlub}-byte row"
            ),
        });
    }

    let irtifa = usize::try_from(mintaqa.irtifa).unwrap_or(0);
    let aala = usize::try_from(mintaqa.aala).unwrap_or(0);
    let mut kharj = Vec::with_capacity(
        usize::try_from(mintaqa.ard).unwrap_or(0).saturating_mul(irtifa).saturating_mul(4),
    );
    for satr in 0..irtifa {
        let mawdi = aala.saturating_add(satr).saturating_mul(khatwa);
        // SAFETY: the mapping covers `khatwa` bytes for each of the surface's
        // own rows, and `aala + satr` is below that height because the region
        // was checked against the surface before this was called. `khatwa`
        // bytes from the row's start are therefore inside the mapping, and the
        // slice does not outlive the `UnlockRect` that follows in the caller.
        let saf = unsafe {
            core::slice::from_raw_parts(maqful.bayt.cast::<u8>().add(mawdi), khatwa)
        };
        if !ansikh_saf(saf, wasf.sigha, mintaqa.yasar, mintaqa.ard, &mut kharj) {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "row {satr} of the read-back surface ended before the region did, which \
                     means the pitch the driver reported is not the pitch it wrote"
                ),
            });
        }
    }
    Ok(kharj)
}

// ---------------------------------------------------------------------------
// The hook
// ---------------------------------------------------------------------------

/// `IDirect3DDevice8::Present`, as a callable pointer.
///
/// `HRESULT Present(const RECT*, const RECT*, HWND, const RGNDATA*)`. The
/// region argument is a pointer this module never reads and passes through
/// untouched, so it is spelled as an opaque address rather than given a type
/// whose layout would then have to be right.
type DallatTaqdeem8 = unsafe extern "system" fn(
    *mut c_void,
    *const Mustatil8,
    *const Mustatil8,
    *mut c_void,
    *const c_void,
) -> Natija8;

/// `IDirect3DDevice8::Reset`, as a callable pointer.
type DallatTasfir8 = unsafe extern "system" fn(*mut c_void, *mut MuallimatArd8) -> Natija8;

/// What a thunk calls when a frame reaches it, given the device it arrived on.
pub type NidaTaqdeem8 = fn(*mut c_void);

/// What a thunk calls before the game's own `Reset` reaches the runtime.
pub type NidaTasfir8 = fn();

/// The captured `IDirect3DDevice8::Present`.
static ASL_TAQDEEM_8: crate::khataf::AslMahfuz = crate::khataf::AslMahfuz::jadeed();

/// The captured `IDirect3DDevice8::Reset`.
static ASL_TASFIR_8: crate::khataf::AslMahfuz = crate::khataf::AslMahfuz::jadeed();

/// The frame callback, stored as an address because a thunk has no state.
static NIDA_TAQDEEM: crate::khataf::AslMahfuz = crate::khataf::AslMahfuz::jadeed();

/// The reset callback, stored the same way.
static NIDA_TASFIR: crate::khataf::AslMahfuz = crate::khataf::AslMahfuz::jadeed();

/// How many calls are currently inside this module's thunks.
static DUKHUL_8: crate::khataf::AaddadDukhul = crate::khataf::AaddadDukhul::jadeed();

/// `IDirect3DDevice8::Present`, which is where the overlay becomes possible.
unsafe extern "system" fn thunk_taqdeem_8(
    jihaz: *mut c_void,
    masdar: *const Mustatil8,
    hadaf: *const Mustatil8,
    nafidha: *mut c_void,
    mintaqa: *const c_void,
) -> Natija8 {
    let _hirasa = crate::khataf::HirasatDukhul::udkhul(&DUKHUL_8);

    let nida = NIDA_TAQDEEM.iqra();
    if !nida.is_null() {
        // SAFETY: `nida` was stored by `TarkeebD3D8::rakkib` from a
        // `NidaTaqdeem8`, which is a plain `fn` pointer of exactly this shape,
        // and it is cleared before the installation that stored it is dropped.
        let nida = unsafe { core::mem::transmute::<*mut c_void, NidaTaqdeem8>(nida) };
        nida(jihaz);
    }

    let asl = ASL_TAQDEEM_8.iqra();
    if asl.is_null() {
        // The slot has already been restored and this call was in flight. There
        // is nothing left to forward to; answering success is the only benign
        // value, because a game that treats a failed `Present` as fatal would
        // be taken down by the overlay's own teardown.
        return 0;
    }
    // SAFETY: `asl` is the pointer this module read out of the slot it
    // replaced, so it is `Present`'s own entry point with this exact signature
    // and calling convention, and the module owning it is still mapped because
    // the game is inside a call to it.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTaqdeem8>(asl) };
    // SAFETY: the arguments are the ones the game passed, unmodified.
    unsafe { asl(jihaz, masdar, hadaf, nafidha, mintaqa) }
}

/// `IDirect3DDevice8::Reset`, hooked so the state block can be released first.
///
/// A `D3DSBT_ALL` block is device state and every token is invalidated by a
/// reset. Applying a stale one afterwards restores whatever the runtime now
/// holds at that index, which is somebody else's state block or nothing at all
/// — and the symptom is the game's own rendering going wrong on the frame after
/// a resolution change, with nothing pointing back here.
unsafe extern "system" fn thunk_tasfir_8(
    jihaz: *mut c_void,
    muallimat: *mut MuallimatArd8,
) -> Natija8 {
    let _hirasa = crate::khataf::HirasatDukhul::udkhul(&DUKHUL_8);

    let nida = NIDA_TASFIR.iqra();
    if !nida.is_null() {
        // SAFETY: as in `thunk_taqdeem_8`, for a `NidaTasfir8`.
        let nida = unsafe { core::mem::transmute::<*mut c_void, NidaTasfir8>(nida) };
        nida();
    }

    let asl = ASL_TASFIR_8.iqra();
    if asl.is_null() {
        return 0;
    }
    // SAFETY: as in `thunk_taqdeem_8`, for `Reset`'s signature.
    let asl = unsafe { core::mem::transmute::<*mut c_void, DallatTasfir8>(asl) };
    // SAFETY: the presentation parameters are the game's own, passed through
    // untouched — the overlay never changes what a game resets to.
    unsafe { asl(jihaz, muallimat) }
}

/// The two replaced slots of an `IDirect3DDevice8`, and what puts them back.
///
/// Removes its hooks on [`Drop`], like every other hook record in this product,
/// and refuses to restore a slot that no longer holds Taarib's own thunk —
/// which is [`taarib_haqn::jadwal::KhatfJadwal`]'s rule and not this module's.
/// That refusal is not hypothetical for this API in particular: a Direct3D 8
/// game that still runs in 2020s is very often one with two or three community
/// proxy libraries already stacked on its device.
#[derive(Debug)]
pub struct TarkeebD3D8 {
    jadwal: taarib_haqn::jadwal::KhatfJadwal,
}

impl TarkeebD3D8 {
    /// Replaces `Present` and `Reset` in a device's method table.
    ///
    /// The table is verified before anything is written — see
    /// [`tahaqquq_jadwal`] — because ninety-six hand-transcribed slots is
    /// ninety-six chances to hook a method that is not the one this build
    /// believes it is.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::JadwalGhayrMawjud`] when the verification disagrees, and
    /// [`KhataTabaqa::KhatfFashil`] naming the method when a slot cannot be
    /// written — which on Windows is usually an anti-tamper product, and on a
    /// game with proxy libraries already installed is the sign that one of them
    /// hooked the same slot between the read and the write.
    ///
    /// # Safety
    ///
    /// `jihaz` must point at a live `IDirect3DDevice8` whose method table is
    /// the one the game's own device uses — which for a COM class it always is,
    /// because a method table belongs to the class and not to the instance.
    /// `nida_taqdeem` and `nida_tasfir` must outlive the installation, which
    /// for a `fn` item is automatic.
    pub unsafe fn rakkib(
        jihaz: *mut c_void,
        nida_taqdeem: NidaTaqdeem8,
        nida_tasfir: NidaTasfir8,
    ) -> Result<Self, KhataTabaqa> {
        // SAFETY: the caller guarantees a live instance.
        let mudaqqiq = unsafe { Jihaz8::jadeed(jihaz) };
        let muallimat = mudaqqiq.muallimat_insha();
        tahaqquq_jadwal(mudaqqiq, muallimat.as_ref())?;

        // Stored before the first slot is replaced. A thunk that went live and
        // found no callback would silently drop the frame that was going to
        // start the overlay, and the next one would too.
        NIDA_TAQDEEM.ihfaz(nida_taqdeem as *const () as *mut c_void);
        NIDA_TASFIR.ihfaz(nida_tasfir as *const () as *mut c_void);

        let mut jadwal =
            taarib_haqn::jadwal::KhatfJadwal::jadeed("IDirect3DDevice8".to_owned());
        // SAFETY: `jihaz` is a live COM interface, so its first machine word is
        // its method table pointer.
        let lawh = unsafe { taarib_haqn::jadwal::jadwal_min_wajiha(jihaz) };

        // SAFETY: `lawh` is an `IDirect3DDevice8` table, which the const
        // assertions in this file prove has more than `KHANAT_TAQDEEM_8`
        // entries; `thunk_taqdeem_8` is a `'static` function with `Present`'s
        // exact signature and calling convention, and it calls the original
        // through `ASL_TAQDEEM_8`, which is stored immediately below and before
        // any game thread can reach the slot.
        let asl = unsafe {
            jadwal.ikhtif(
                lawh,
                KHANAT_TAQDEEM_8,
                thunk_taqdeem_8 as *const () as *mut c_void,
                "IDirect3DDevice8::Present",
            )
        }
        .map_err(|khata| KhataTabaqa::KhatfFashil {
            mawdi: "IDirect3DDevice8::Present".to_owned(),
            sabab: khata.to_string(),
        })?;
        ASL_TAQDEEM_8.ihfaz(asl);

        // SAFETY: as above, for `Reset` at `KHANAT_TASFIR_8` and
        // `thunk_tasfir_8`, which carries that method's signature and calls the
        // original through `ASL_TASFIR_8`.
        let asl = unsafe {
            jadwal.ikhtif(
                lawh,
                KHANAT_TASFIR_8,
                thunk_tasfir_8 as *const () as *mut c_void,
                "IDirect3DDevice8::Reset",
            )
        }
        .map_err(|khata| KhataTabaqa::KhatfFashil {
            mawdi: "IDirect3DDevice8::Reset".to_owned(),
            sabab: khata.to_string(),
        })?;
        ASL_TASFIR_8.ihfaz(asl);

        Ok(Self { jadwal })
    }

    /// How many slots are currently replaced.
    #[must_use]
    pub const fn adad(&self) -> usize {
        self.jadwal.adad()
    }

    /// What has happened to these hooks, for the diagnostics bundle.
    #[must_use]
    pub fn athar(&self) -> &[String] {
        self.jadwal.athar()
    }

    /// Restores both slots, and refuses to restore one somebody else has taken.
    ///
    /// The saved originals are cleared **after** the restore rather than
    /// before, and only when the restore succeeded: a call already inside a
    /// thunk still has to reach the game's own `Present`, and clearing the
    /// original first would make it return success without presenting a frame.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::FakkKhatfFashil`] naming every slot that could not be put
    /// back, which includes the slot another overlay hooked after Taarib did.
    /// It means this module cannot be unloaded, and the user is told that a
    /// restart is what removes it.
    pub fn fukk(&mut self) -> Result<(), KhataTabaqa> {
        let natija = self.jadwal.fukk();
        if natija.is_ok() {
            NIDA_TAQDEEM.imsah();
            NIDA_TASFIR.imsah();
        }
        natija.map_err(|khata| KhataTabaqa::FakkKhatfFashil {
            mawdi: "IDirect3DDevice8".to_owned(),
            sabab: khata.to_string(),
        })
    }

    /// How many calls are still inside this module's thunks.
    ///
    /// Teardown waits for this to reach zero. Restoring a slot stops *new*
    /// calls from entering and does nothing about one already past the entry
    /// and about to execute the next instruction in a module being unmapped.
    #[must_use]
    pub fn dukhul() -> usize {
        DUKHUL_8.qaim()
    }
}

impl Drop for TarkeebD3D8 {
    fn drop(&mut self) {
        if self.jadwal.adad() > 0 {
            let _ = self.fukk();
        }
    }
}

// ---------------------------------------------------------------------------
// A device made only to be read, and destroyed
// ---------------------------------------------------------------------------

/// `D3DCREATE_FPU_PRESERVE`.
///
/// Not optional here, and the reason is the least obvious hazard in this file.
/// Creating a Direct3D 8 device switches the x87 control word to single
/// precision unless this flag is passed — for the whole process, not for the
/// device. A throwaway device created inside a running game without it would
/// change the results of the game's own floating-point arithmetic from that
/// moment on, which is a physics engine that behaves differently after the
/// overlay was installed and nothing anybody could trace back here.
#[cfg(windows)]
const INSHA_HIFZ_FASILA: u32 = 0x0000_0002;

/// A Direct3D 8 object, a device and a hidden window, all destroyed on drop.
///
/// Exists for one purpose: there is no API for "give me the method table of the
/// device this process is using", and the documented way to find one is to make
/// your own. The pointers read out of it are the game's, because a method table
/// belongs to the class and not to the instance.
#[cfg(windows)]
#[derive(Debug)]
pub struct JihazMuaqqat8 {
    jihaz: *mut c_void,
    d3d: *mut c_void,
    _nafidha: crate::khataf::NafidhaMuaqqata,
}

#[cfg(windows)]
impl JihazMuaqqat8 {
    /// The device, for reading its method table.
    #[must_use]
    pub const fn jihaz(&self) -> *mut c_void {
        self.jihaz
    }
}

#[cfg(windows)]
impl Drop for JihazMuaqqat8 {
    fn drop(&mut self) {
        if !self.jihaz.is_null() {
            // SAFETY: `jihaz` is the live device this type created and owns the
            // only reference to; this is the one `Release` for it, and the
            // field is not reachable afterwards because `self` is dropping.
            unsafe {
                let _ = Jihaz8::jadeed(self.jihaz).itlaq();
            }
        }
        if !self.d3d.is_null() {
            // SAFETY: `d3d` is the live `IDirect3D8` this type created, whose
            // first machine word is its method table. One `Release`, once.
            unsafe {
                let jadwal = self.d3d.cast::<*const JadwalD3d8>().read();
                let _ = ((*jadwal).itlaq)(self.d3d);
            }
        }
    }
}

/// Creates a throwaway Direct3D 8 device against a hidden window.
///
/// `d3d8.dll` is looked up rather than loaded: a process that never used
/// Direct3D 8 must not gain it because Taarib asked a question. That is the
/// same rule [`crate::khataf::hal_yumkin`] follows, for the same reason.
///
/// # Errors
///
/// [`KhataTabaqa::MaktabaMafquda`] when `d3d8.dll` is not in the process or
/// does not export `Direct3DCreate8`, and
/// [`KhataTabaqa::JadwalGhayrMawjud`] when no device can be created — a
/// headless session, an adapter with no Direct3D 8 support left in its driver,
/// or a wrapper that refuses a second device.
#[cfg(windows)]
pub fn jihaz_muaqqat() -> Result<JihazMuaqqat8, KhataTabaqa> {
    use taarib_haqn::mawqi::{qaidat_wahda, ramz_wahda};

    /// `IDirect3D8* WINAPI Direct3DCreate8(UINT SDKVersion)`.
    type DallatInsha8 = unsafe extern "system" fn(u32) -> *mut c_void;

    let Some(qaida) = qaidat_wahda("d3d8.dll") else {
        return Err(KhataTabaqa::MaktabaMafquda {
            maktaba: "d3d8.dll".to_owned(),
            ramz: "the module itself".to_owned(),
        });
    };
    // SAFETY: `qaida` is the base `qaidat_wahda` just reported for a module the
    // process has loaded, which is `ramz_wahda`'s stated precondition.
    let Some(ramz) = (unsafe { ramz_wahda(qaida, "Direct3DCreate8") }) else {
        return Err(KhataTabaqa::MaktabaMafquda {
            maktaba: "d3d8.dll".to_owned(),
            ramz: "Direct3DCreate8".to_owned(),
        });
    };
    // SAFETY: `ramz` is the address `d3d8.dll` exports for `Direct3DCreate8`,
    // whose signature has been `IDirect3D8*(UINT)` since the API shipped.
    let insha = unsafe { core::mem::transmute::<*mut c_void, DallatInsha8>(ramz) };

    // SAFETY: `Direct3DCreate8` takes an SDK version and returns an interface
    // with a reference this call owns, or null for a version it does not know.
    let d3d = unsafe { insha(ISDAR_SDK_8) };
    if d3d.is_null() {
        return Err(KhataTabaqa::JadwalGhayrMawjud {
            wajiha: "IDirect3D8".to_owned(),
            sabab: format!(
                "Direct3DCreate8({ISDAR_SDK_8}) produced nothing, which is what a d3d8.dll built \
                 for a different SDK version answers"
            ),
        });
    }

    let nafidha = crate::khataf::nafidha_muaqqata()?;
    let mut muallimat = MuallimatArd8 {
        ard: 8,
        irtifa: 8,
        // `D3DFMT_UNKNOWN` is legal windowed and means "whatever the desktop
        // is", which is what keeps this from having to match a display mode.
        sigha: 0,
        adad: 1,
        taadud: TAADUD_LA_SHAY,
        athar: TABDIL_IHMAL,
        nafidha: nafidha.maqbad().0,
        munafadha: 1,
        umq_tilqai: 0,
        sighat_umq: 0,
        aalam: 0,
        taraddud: 0,
        fasil: 0,
    };
    let mut jihaz: *mut c_void = core::ptr::null_mut();

    // SAFETY: `d3d` is the live interface from above, whose first machine word
    // is its method table; the window handle is alive for the whole call
    // because `nafidha` outlives it; and both out-parameters address live
    // locals. `INSHA_HIFZ_FASILA` is passed for the reason its own constant
    // documents and is not optional inside a running game.
    let natija = unsafe {
        let jadwal = d3d.cast::<*const JadwalD3d8>().read();
        ((*jadwal).insha_jihaz)(
            d3d,
            MUHAWWIL_IFTIRADI,
            NAW_JIHAZ_HAL,
            nafidha.maqbad().0,
            INSHA_RUUS_BARMAJI | INSHA_HIFZ_FASILA,
            &raw mut muallimat,
            &raw mut jihaz,
        )
    };

    let mabni = JihazMuaqqat8 { jihaz, d3d, _nafidha: nafidha };
    if !najah(natija) || mabni.jihaz.is_null() {
        return Err(KhataTabaqa::JadwalGhayrMawjud {
            wajiha: "IDirect3DDevice8".to_owned(),
            sabab: format!(
                "CreateDevice answered {natija:#010x}; no throwaway device could be made to read \
                 the method table from"
            ),
        });
    }
    Ok(mabni)
}

// ---------------------------------------------------------------------------
// What can be decided before the overlay is enabled
// ---------------------------------------------------------------------------

/// What the overlay will be able to do on Direct3D 8 in this process.
///
/// Answered before anything is hooked, so the screen where a user turns the
/// overlay on can say it. Four things are established here and only the first
/// two are guesses anybody would make:
///
/// 1. whether `d3d8.dll` is in the process and exports what the hook needs;
/// 2. whether a device can be created at all, and whether the ninety-six-slot
///    method table in this file behaves as `d3d8.h` says it should;
/// 3. whether the `d3d8.dll` answering is Microsoft's or a community
///    replacement, which changes what "the state was restored" means;
/// 4. the two limits that are properties of the API rather than of this
///    machine — a multisampled backbuffer cannot be read, and a game that
///    presents through an additional swap chain is never reached.
#[cfg(windows)]
#[must_use]
pub fn qudra() -> crate::qudra::QudratTarkeeb {
    use crate::qudra::{MilShasha, QudratTarkeeb, SababQudra};
    use taarib_haqn::mawqi::qaidat_wahda;

    let mut taqreer =
        QudratTarkeeb::jadeeda(WajihatRusum::Direct3D8, MilShasha::KhilalAlJihaz);

    if qaidat_wahda("d3d8.dll").is_none() {
        return taqreer.maa(SababQudra::mustaheela(
            "المكتبة d3d8.dll غير محمّلة في هذه اللعبة، فلا يوجد جهاز عرض من الجيل الثامن \
             تُركَّب عليه الطبقة.",
            "d3d8.dll is not loaded in this process, so there is no Direct3D 8 device for the \
             overlay to attach to",
        ));
    }

    match jihaz_muaqqat() {
        Ok(mabni) => {
            // SAFETY: `mabni` holds a live device it created and owns.
            let jihaz = unsafe { Jihaz8::jadeed(mabni.jihaz()) };
            let muallimat = jihaz.muallimat_insha();
            if let Err(khata) = tahaqquq_jadwal(jihaz, muallimat.as_ref()) {
                taqreer = taqreer.maa(SababQudra::mustaheela(
                    "جدول دوال الجهاز في هذه اللعبة لا يتصرّف كما ينصّ عليه توثيق دايركت٣د ٨، \
                     ولن تُركَّب الطبقة عليه.",
                    format!(
                        "the Direct3D 8 method table in this process does not behave as the API \
                         documents, so nothing will be hooked: {khata}"
                    ),
                ));
                return taqreer;
            }
            taqreer = taqreer.maa(SababQudra::kamila(
                "أُنشئ جهاز اختباري وتحقّقت الطبقة من جدول دواله قبل تركيب أي خطّاف.",
                "a throwaway device was created and its method table verified before anything was \
                 hooked",
            ));
        }
        Err(khata) => {
            return taqreer.maa(SababQudra::mustaheela(
                "تعذّر إنشاء جهاز دايركت٣د ٨ على هذا الحاسوب، ولا يمكن قراءة جدول الدوال بدونه.",
                format!("no Direct3D 8 device could be created on this machine: {khata}"),
            ));
        }
    }

    if qaidat_wahda("d3d9.dll").is_some() || qaidat_wahda("dxvk_d3d8.dll").is_some() {
        taqreer = taqreer.maa(SababQudra::kamila(
            "يبدو أنّ d3d8.dll في هذه اللعبة بديل مجتمعي يترجم إلى الجيل التاسع أو إلى فولكان. \
             تُرسم الطبقة عبر واجهة الجيل الثامن نفسها التي تستخدمها اللعبة، وحفظ الحالة \
             واسترجاعها يمرّان عبر ذلك البديل.",
            "the d3d8.dll answering here appears to be a community replacement translating to \
             Direct3D 9 or Vulkan; the overlay draws through the same Direct3D 8 interface the \
             game itself calls, and saving and restoring state goes through that wrapper",
        ));
    }

    taqreer
        .maa(SababQudra::kamila(
            "إن كانت اللعبة تعرض إطارًا متعدّد العيّنات فلا سبيل في دايركت٣د ٨ لقراءته، وستُرسم \
             الطبقة دون أن تقرأ الشاشة. تُعلن الطبقة ذلك عند أوّل محاولة قراءة.",
            "if this game presents a multisampled backbuffer there is no way to read it in \
             Direct3D 8 — the overlay will draw and will not recognise anything — and it says so \
             on the first capture rather than returning a blank image",
        ))
        .maa(SababQudra::kamila(
            "إن كانت اللعبة تعرض عبر سلسلة عرض إضافية بدل جهازها فلن يصل الخطّاف إليها ولن تبدأ \
             الطبقة أصلًا.",
            "if this game presents through an additional swap chain rather than through its \
             device, the hook is never reached and the overlay simply never starts",
        ))
}

/// What the overlay will be able to do on Direct3D 8 here — nothing, off Windows.
#[cfg(not(windows))]
#[must_use]
pub fn qudra() -> crate::qudra::QudratTarkeeb {
    use crate::qudra::{MilShasha, QudratTarkeeb, SababQudra};

    QudratTarkeeb::jadeeda(WajihatRusum::Direct3D8, MilShasha::LaShay).maa(
        SababQudra::mustaheela(
            "دايركت٣د ٨ واجهة خاصة بويندوز، ولا وجود لها على هذه المنصة.",
            "Direct3D 8 is a Windows API and does not exist on this platform",
        ),
    )
}

// ---------------------------------------------------------------------------
// Proving what can be proved without a game
// ---------------------------------------------------------------------------

#[cfg(test)]
#[expect(
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test reports failure by panicking, and every index below is into an array whose \
              length is a constant in the same function"
)]
mod ikhtibar {
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

    use super::*;

    /// The hook tests share this module's statics, so they take turns.
    static DAWR: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

    /// How many times the frame callback has been reached.
    static NIDA_MARRAT: AtomicUsize = AtomicUsize::new(0);
    /// How many times the stub's own `Present` has been reached.
    static ASL_MARRAT: AtomicUsize = AtomicUsize::new(0);
    /// How many times the reset callback has been reached.
    static TASFIR_MARRAT: AtomicUsize = AtomicUsize::new(0);
    /// The stub device's one render state.
    static HALA_MUZAYYAFA: AtomicU32 = AtomicU32::new(0);

    /// A page of ninety-six pointers, mapped so the hook may write into it.
    ///
    /// Mapped rather than boxed on purpose: the write guard this product uses
    /// restores a page to read-and-execute when it is done, which is right for
    /// a mapped image's method table and would take write permission away from
    /// whatever else the allocator had put on the same heap page.
    struct SafhatJadwal {
        asas: *mut c_void,
        tul: usize,
    }

    impl SafhatJadwal {
        /// Reserves and commits one page, readable and writable.
        #[cfg(windows)]
        fn jadeeda() -> Self {
            use windows::Win32::System::Memory::{
                MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
            };

            let tul = 4096;
            // SAFETY: a null base with a non-zero size asks the allocator to
            // choose an address; the return is checked for null below.
            let asas = unsafe {
                VirtualAlloc(None, tul, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
            };
            assert!(!asas.is_null(), "the stub vtable page could not be reserved");
            Self { asas, tul }
        }

        /// Maps one page, readable and writable.
        #[cfg(not(windows))]
        fn jadeeda() -> Self {
            let tul = 4096;
            // SAFETY: an anonymous private mapping with a null hint and a
            // non-zero length is the documented way to ask for fresh pages, and
            // the return is checked against `MAP_FAILED` below.
            let asas = unsafe {
                libc::mmap(
                    core::ptr::null_mut(),
                    tul,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            assert!(
                !core::ptr::eq(asas, libc::MAP_FAILED),
                "the stub vtable page could not be mapped"
            );
            Self { asas, tul }
        }

        /// The table, as the hook will see it.
        fn khanat(&self) -> *mut *mut c_void {
            self.asas.cast::<*mut c_void>()
        }

        /// Makes the page writable again after the hook's guard sealed it.
        #[cfg(windows)]
        fn iftah(&self) {
            use windows::Win32::System::Memory::{
                PAGE_PROTECTION_FLAGS, PAGE_READWRITE, VirtualProtect,
            };

            let mut sabiqa = PAGE_PROTECTION_FLAGS(0);
            // SAFETY: the span is this type's own allocation, whole, and
            // `sabiqa` is a live out-parameter.
            let natija =
                unsafe { VirtualProtect(self.asas, self.tul, PAGE_READWRITE, &raw mut sabiqa) };
            assert!(natija.is_ok(), "the stub vtable page could not be made writable again");
        }

        /// Makes the page writable again after the hook's guard sealed it.
        #[cfg(not(windows))]
        fn iftah(&self) {
            // SAFETY: the span is this type's own mapping, whole.
            let natija = unsafe {
                libc::mprotect(self.asas, self.tul, libc::PROT_READ | libc::PROT_WRITE)
            };
            assert_eq!(natija, 0, "the stub vtable page could not be made writable again");
        }

        /// Writes one slot.
        fn ida(&self, khana: usize, qeema: *mut c_void) {
            assert!(khana < 96, "slot {khana} is past the end of an IDirect3DDevice8 table");
            // SAFETY: `khana` is below ninety-six and the mapping holds five
            // hundred and twelve pointers, so the offset is inside it. The page
            // is writable: `jadeeda` maps it so and `iftah` puts it back.
            unsafe { self.khanat().add(khana).write(qeema) };
        }

        /// Reads one slot.
        fn iqra(&self, khana: usize) -> *mut c_void {
            assert!(khana < 96, "slot {khana} is past the end of an IDirect3DDevice8 table");
            // SAFETY: as `ida`, and a read needs only the mapping's read
            // permission, which no guard removes.
            unsafe { self.khanat().add(khana).read() }
        }
    }

    impl Drop for SafhatJadwal {
        #[cfg(windows)]
        fn drop(&mut self) {
            use windows::Win32::System::Memory::{MEM_RELEASE, VirtualFree};

            // SAFETY: the reservation this type made, released once. A size of
            // zero is what `MEM_RELEASE` requires, and the base is the one
            // `VirtualAlloc` returned.
            unsafe {
                let _ = VirtualFree(self.asas, 0, MEM_RELEASE);
            }
        }

        #[cfg(not(windows))]
        fn drop(&mut self) {
            // SAFETY: the mapping this type made, unmapped once.
            unsafe {
                let _ = libc::munmap(self.asas, self.tul);
            }
        }
    }

    unsafe extern "system" fn taawun_muzayyaf(_jihaz: *mut c_void) -> Natija8 {
        0
    }

    unsafe extern "system" fn namat_muzayyaf(
        _jihaz: *mut c_void,
        namat: *mut NamatArd8,
    ) -> Natija8 {
        // SAFETY: the caller — this module's own verification — passes a live
        // local of exactly this type.
        unsafe {
            namat.write(NamatArd8 { ard: 1920, irtifa: 1080, taraddud: 60, sigha: SIGHA_X8R8G8B8 });
        }
        0
    }

    unsafe extern "system" fn insha_muzayyaf(
        _jihaz: *mut c_void,
        muallimat: *mut MuallimatInsha8,
    ) -> Natija8 {
        // SAFETY: as `namat_muzayyaf`.
        unsafe {
            muallimat.write(MuallimatInsha8 {
                muhawwil: MUHAWWIL_IFTIRADI,
                naw: NAW_JIHAZ_HAL,
                nafidha: core::ptr::null_mut(),
                aalam: INSHA_RUUS_ARIDA,
            });
        }
        0
    }

    unsafe extern "system" fn dai_hala_muzayyaf(
        _jihaz: *mut c_void,
        _hala: u32,
        qeema: u32,
    ) -> Natija8 {
        HALA_MUZAYYAFA.store(qeema, Ordering::Release);
        0
    }

    unsafe extern "system" fn ijlib_hala_muzayyaf(
        _jihaz: *mut c_void,
        _hala: u32,
        qeema: *mut u32,
    ) -> Natija8 {
        // SAFETY: as `namat_muzayyaf`, for one `DWORD`.
        unsafe { qeema.write(HALA_MUZAYYAFA.load(Ordering::Acquire)) };
        0
    }

    unsafe extern "system" fn insha_kutla_muzayyaf(
        _jihaz: *mut c_void,
        _naw: u32,
        ramz: *mut u32,
    ) -> Natija8 {
        // SAFETY: as `namat_muzayyaf`, for one `DWORD`.
        unsafe { ramz.write(7) };
        0
    }

    unsafe extern "system" fn hadhf_kutla_muzayyaf(_jihaz: *mut c_void, _ramz: u32) -> Natija8 {
        0
    }

    /// A `GetRenderState` that answers from somewhere other than where
    /// `SetRenderState` wrote, which is what a table off by one slot looks like.
    unsafe extern "system" fn ijlib_hala_kadhib(
        _jihaz: *mut c_void,
        _hala: u32,
        qeema: *mut u32,
    ) -> Natija8 {
        // SAFETY: the caller passes a live local `DWORD`.
        unsafe { qeema.write(0) };
        0
    }

    unsafe extern "system" fn taqdeem_asli(
        _jihaz: *mut c_void,
        _masdar: *const Mustatil8,
        _hadaf: *const Mustatil8,
        _nafidha: *mut c_void,
        _mintaqa: *const c_void,
    ) -> Natija8 {
        let _ = ASL_MARRAT.fetch_add(1, Ordering::AcqRel);
        0
    }

    unsafe extern "system" fn tasfir_asli(
        _jihaz: *mut c_void,
        _muallimat: *mut MuallimatArd8,
    ) -> Natija8 {
        0
    }

    /// Somebody else's hook, landing on the same slot after Taarib's.
    unsafe extern "system" fn taqdeem_ghareeb(
        _jihaz: *mut c_void,
        _masdar: *const Mustatil8,
        _hadaf: *const Mustatil8,
        _nafidha: *mut c_void,
        _mintaqa: *const c_void,
    ) -> Natija8 {
        0
    }

    fn nida_itar(_jihaz: *mut c_void) {
        let _ = NIDA_MARRAT.fetch_add(1, Ordering::AcqRel);
    }

    fn nida_tasfir() {
        let _ = TASFIR_MARRAT.fetch_add(1, Ordering::AcqRel);
    }

    /// A stub device: a table this test owns, and an object pointing at it.
    fn jadwal_muzayyaf() -> SafhatJadwal {
        let safha = SafhatJadwal::jadeeda();
        safha.ida(3, taawun_muzayyaf as *const () as *mut c_void);
        safha.ida(8, namat_muzayyaf as *const () as *mut c_void);
        safha.ida(9, insha_muzayyaf as *const () as *mut c_void);
        safha.ida(KHANAT_TASFIR_8, tasfir_asli as *const () as *mut c_void);
        safha.ida(KHANAT_TAQDEEM_8, taqdeem_asli as *const () as *mut c_void);
        safha.ida(50, dai_hala_muzayyaf as *const () as *mut c_void);
        safha.ida(51, ijlib_hala_muzayyaf as *const () as *mut c_void);
        safha.ida(56, hadhf_kutla_muzayyaf as *const () as *mut c_void);
        safha.ida(57, insha_kutla_muzayyaf as *const () as *mut c_void);
        safha
    }

    /// Calls slot fifteen the way a game would.
    fn qaddim(kaain: *mut c_void) {
        // SAFETY: `kaain` addresses a live local holding the stub table's
        // address, which is the shape of a COM instance; slot fifteen holds
        // either the stub's own `Present` or Taarib's thunk, both of which have
        // this signature.
        unsafe {
            let jadwal = kaain.cast::<*mut *mut c_void>().read();
            let dalla = core::mem::transmute::<*mut c_void, DallatTaqdeem8>(
                jadwal.add(KHANAT_TAQDEEM_8).read(),
            );
            let _ = dalla(
                kaain,
                core::ptr::null(),
                core::ptr::null(),
                core::ptr::null_mut(),
                core::ptr::null(),
            );
        }
    }

    /// The hook installs, the thunk is reached, the original still runs, and
    /// unhooking puts the slot back.
    ///
    /// This is the rung the brief calls "hooks install and unhook against a
    /// stub device vtable you construct", executed rather than inferred. What
    /// it does not prove is anything about a real Direct3D 8 device: the stub
    /// answers the verification the way `d3d8.h` says a device must, which is
    /// the contract being tested, not a driver.
    #[test]
    fn khatf_thumma_fakk() {
        let _dawr = DAWR.lock();
        NIDA_MARRAT.store(0, Ordering::Release);
        ASL_MARRAT.store(0, Ordering::Release);

        let safha = jadwal_muzayyaf();
        let asli = safha.iqra(KHANAT_TAQDEEM_8);
        let mut kaain: *mut c_void = safha.khanat().cast::<c_void>();
        let jihaz = (&raw mut kaain).cast::<c_void>();

        // SAFETY: `jihaz` addresses a live local whose first machine word is
        // the stub table, which is the layout a COM instance has, and the table
        // answers every method the verification calls.
        let mut tarkeeb = match unsafe { TarkeebD3D8::rakkib(jihaz, nida_itar, nida_tasfir) } {
            Ok(tarkeeb) => tarkeeb,
            Err(khata) => panic!("the hook would not install against a stub table: {khata}"),
        };
        assert_eq!(tarkeeb.adad(), 2, "Present and Reset are two slots");
        assert_ne!(
            safha.iqra(KHANAT_TAQDEEM_8),
            asli,
            "slot fifteen still holds the stub's own Present, so nothing was hooked"
        );

        qaddim(jihaz);
        assert_eq!(NIDA_MARRAT.load(Ordering::Acquire), 1, "the frame callback was not reached");
        assert_eq!(
            ASL_MARRAT.load(Ordering::Acquire),
            1,
            "the thunk did not forward to the original, so the game's frame would never present"
        );

        match tarkeeb.fukk() {
            Ok(()) => {}
            Err(khata) => panic!("the hook would not come off: {khata}"),
        }
        assert_eq!(
            safha.iqra(KHANAT_TAQDEEM_8),
            asli,
            "slot fifteen was not restored to what it held before"
        );

        safha.iftah();
        qaddim(jihaz);
        assert_eq!(
            NIDA_MARRAT.load(Ordering::Acquire),
            1,
            "the callback was reached after the hook was removed"
        );
        assert_eq!(ASL_MARRAT.load(Ordering::Acquire), 2, "the stub's own Present was not reached");
    }

    /// A slot another overlay took after Taarib is left alone, and said so.
    ///
    /// The rule is [`taarib_haqn::jadwal::KhatfJadwal`]'s and this is the test
    /// that it holds through this module's own installation path. Writing
    /// Taarib's saved original over somebody else's hook would leave that
    /// overlay calling into a thunk about to be unmapped, and there is no
    /// correct fix from inside the process — whoever hooked last has to unhook
    /// first.
    #[test]
    fn fakk_yarfud_khana_akhadhaha_ghayruna() {
        let _dawr = DAWR.lock();

        let safha = jadwal_muzayyaf();
        let asli = safha.iqra(KHANAT_TAQDEEM_8);
        let mut kaain: *mut c_void = safha.khanat().cast::<c_void>();
        let jihaz = (&raw mut kaain).cast::<c_void>();

        // SAFETY: as in `khatf_thumma_fakk`.
        let mut tarkeeb = match unsafe { TarkeebD3D8::rakkib(jihaz, nida_itar, nida_tasfir) } {
            Ok(tarkeeb) => tarkeeb,
            Err(khata) => panic!("the hook would not install against a stub table: {khata}"),
        };

        // Somebody else's overlay lands on the same slot, after ours.
        safha.iftah();
        let ghareeb = taqdeem_ghareeb as *const () as *mut c_void;
        safha.ida(KHANAT_TAQDEEM_8, ghareeb);

        let natija = tarkeeb.fukk();
        let Err(khata) = natija else {
            panic!("unhooking reported success over a slot somebody else had taken");
        };
        assert!(
            matches!(khata, KhataTabaqa::FakkKhatfFashil { .. }),
            "the refusal must be FakkKhatfFashil, and was {khata}"
        );
        assert_eq!(
            safha.iqra(KHANAT_TAQDEEM_8),
            ghareeb,
            "Taarib overwrote another overlay's hook with its own saved original"
        );
        assert_ne!(safha.iqra(KHANAT_TAQDEEM_8), asli, "the slot was restored anyway");
        // Reset was hooked second and popped first, so it was restored before
        // the refusal was reached. Both halves matter: the refusal is per slot,
        // not per installation.
        assert_eq!(
            safha.iqra(KHANAT_TASFIR_8),
            tasfir_asli as *const () as *mut c_void,
            "the slot nobody else had taken was left hooked"
        );
    }

    /// The verification refuses a table whose slots answer implausibly.
    #[test]
    fn tahaqquq_yarfud_jadwalan_khatian() {
        let _dawr = DAWR.lock();

        let safha = jadwal_muzayyaf();
        // `GetRenderState` answering from somewhere other than where
        // `SetRenderState` wrote is exactly what a table off by one slot looks
        // like, and it is the check that catches a transcription error in the
        // ninety-six.
        safha.ida(51, ijlib_hala_kadhib as *const () as *mut c_void);

        let mut kaain: *mut c_void = safha.khanat().cast::<c_void>();
        let jihaz = (&raw mut kaain).cast::<c_void>();
        // SAFETY: as in `khatf_thumma_fakk`.
        let natija = unsafe { TarkeebD3D8::rakkib(jihaz, nida_itar, nida_tasfir) };
        let Err(khata) = natija else {
            panic!("the hook installed over a table whose SetRenderState does not round trip");
        };
        assert!(
            matches!(khata, KhataTabaqa::JadwalGhayrMawjud { .. }),
            "a table that does not verify must be JadwalGhayrMawjud, and was {khata}"
        );
        assert_eq!(
            safha.iqra(KHANAT_TAQDEEM_8),
            taqdeem_asli as *const () as *mut c_void,
            "a slot was replaced before the table had been verified"
        );
    }

    /// The colour conversion encodes and re-premultiplies, and survives zero
    /// alpha.
    #[test]
    fn lawn_yatarammaz_wa_yubqi_al_darb() {
        // Opaque mid grey: un-premultiplying by one is a no-op, so this is the
        // transfer function on its own. Linear 0.5 encodes to about 0.7354,
        // which is 188 of 255.
        let ramadi = lawn_d3d([0.5, 0.5, 0.5, 1.0]);
        assert_eq!(ramadi >> 24, 0xFF, "an opaque colour must stay opaque");
        for izaha in [16, 8, 0] {
            let qanat = (ramadi >> izaha) & 0xFF;
            assert!(
                (187..=189).contains(&qanat),
                "linear 0.5 must encode to about 188 and became {qanat}"
            );
        }

        // Half-covered white: premultiplied linear is (0.5, 0.5, 0.5, 0.5).
        // Un-premultiplying gives one, which encodes to one, which
        // re-premultiplied is 0.5 — so every channel is 128, not 188. A
        // conversion that skipped the un-premultiply would produce 188 here and
        // draw every antialiased glyph edge too bright.
        let nisf = lawn_d3d([0.5, 0.5, 0.5, 0.5]);
        assert_eq!(nisf >> 24, 128, "half alpha must round to 128");
        for izaha in [16, 8, 0] {
            let qanat = (nisf >> izaha) & 0xFF;
            assert!(
                (127..=129).contains(&qanat),
                "a half-covered white texel must stay premultiplied at about 128, and became \
                 {qanat}"
            );
        }

        assert_eq!(lawn_d3d([0.0, 0.0, 0.0, 0.0]), 0, "a transparent quad must pack to zero");
    }

    /// The batch becomes six vertices a quad, in runs that follow the plates.
    #[test]
    fn dufaat_tatba_al_alwah() {
        use crate::wajiha::{MustatilNisbi, SighatSath};

        let sath = WasfSath { ard: 640, irtifa: 480, sigha: SighatSath::Bgra8, sirgb: false };
        let lawh = QitaRasm {
            mawdi: MustatilBiksel { yasar: 10, aala: 20, ard: 100, irtifa: 16 },
            khareeta: None,
            lawn: [1.0, 1.0, 1.0, 1.0],
        };
        let shakl = QitaRasm {
            khareeta: Some(MustatilNisbi { yasar: 0.25, aala: 0.5, ard: 0.25, irtifa: 0.25 }),
            ..lawh
        };
        let lawha = LawhatRasm { qitaat: vec![lawh, shakl, shakl, lawh], sath };

        let mut ruus = Vec::new();
        let mut dufaat = Vec::new();
        ibni_ruus(&lawha, (1.0, 1.0), &mut ruus, &mut dufaat);

        assert_eq!(ruus.len(), 24, "four quads are twenty-four vertices");
        assert_eq!(dufaat.len(), 3, "plate, two glyphs, plate is three runs");
        assert_eq!(dufaat[0], DufaaQadima { masturat: false, bidaya: 0, adad: 6 });
        assert_eq!(dufaat[1], DufaaQadima { masturat: true, bidaya: 6, adad: 12 });
        assert_eq!(dufaat[2], DufaaQadima { masturat: false, bidaya: 18, adad: 6 });

        // The half-pixel offset, which nothing else in this crate applies and
        // which Direct3D 8 requires of pre-transformed vertices.
        assert!(
            (ruus[0].mawdi[0] - 9.5).abs() < f32::EPSILON,
            "the first vertex is at {} and must be half a pixel left of ten",
            ruus[0].mawdi[0]
        );
        assert!(
            (ruus[0].mawdi[1] - 19.5).abs() < f32::EPSILON,
            "the first vertex is at {} and must be half a pixel above twenty",
            ruus[0].mawdi[1]
        );
        assert!(
            (ruus[0].mawdi[3] - 1.0).abs() < f32::EPSILON,
            "the reciprocal homogeneous w of a pre-transformed vertex is one"
        );

        // A padded texture rescales the atlas coordinates and nothing else.
        let mut ruus_mubattana = Vec::new();
        let mut dufaat_mubattana = Vec::new();
        ibni_ruus(&lawha, (0.5, 0.25), &mut ruus_mubattana, &mut dufaat_mubattana);
        assert!(
            (ruus_mubattana[6].khareeta[0] - 0.125).abs() < f32::EPSILON,
            "a half-width padding must halve u, and u became {}",
            ruus_mubattana[6].khareeta[0]
        );
        assert!(
            (ruus_mubattana[6].khareeta[1] - 0.125).abs() < f32::EPSILON,
            "a quarter-height padding must quarter v, and v became {}",
            ruus_mubattana[6].khareeta[1]
        );
    }

    /// Every backbuffer format Direct3D 8 can present widens to BGRA8.
    #[test]
    fn kul_sigha_khalfiya_tatawassa() {
        // Opaque white in each of the five formats a swap chain may carry.
        for (sigha, bayt) in [
            (SIGHA_A8R8G8B8, vec![0xFF, 0xFF, 0xFF, 0xFF]),
            (SIGHA_X8R8G8B8, vec![0xFF, 0xFF, 0xFF, 0x00]),
            (SIGHA_R5G6B5, vec![0xFF, 0xFF]),
            (SIGHA_X1R5G5B5, vec![0xFF, 0x7F]),
            (SIGHA_A1R5G5B5, vec![0xFF, 0xFF]),
            (SIGHA_A2R10G10B10, vec![0xFF, 0xFF, 0xFF, 0xFF]),
        ] {
            let mut kharj = Vec::new();
            assert!(
                ansikh_saf(&bayt, sigha, 0, 1, &mut kharj),
                "{} did not convert at all",
                ism_sigha(sigha)
            );
            assert_eq!(
                kharj,
                vec![0xFF, 0xFF, 0xFF, 0xFF],
                "white in {} widened to {kharj:?} rather than to opaque white; a five-bit \
                 channel shifted rather than replicated is what makes white read as grey",
                ism_sigha(sigha)
            );
        }

        // A row that ends before the region does is a refusal rather than a
        // short image, because a half-filled capture reads as a blank screen.
        let mut kharj = Vec::new();
        assert!(
            !ansikh_saf(&[0xFF, 0xFF], SIGHA_R5G6B5, 0, 4, &mut kharj),
            "a two-byte row must not answer four pixels"
        );
    }

    /// The two slot constants agree with the method table's own field offsets.
    ///
    /// The `const` assertions in this file already prove it at compile time;
    /// this is the same fact stated where a test run reports it, because a
    /// compile-time assertion that never fires is invisible in a test summary
    /// and this is the single number a reviewer most wants to see checked.
    #[test]
    fn khanat_taqdeem_wa_tasfir() {
        assert_eq!(KHANAT_TAQDEEM_8, 15, "IDirect3DDevice8::Present is slot fifteen");
        assert_eq!(KHANAT_TASFIR_8, 14, "IDirect3DDevice8::Reset is slot fourteen");
        assert_eq!(
            size_of::<JadwalJihaz8>(),
            96 * size_of::<*const c_void>(),
            "IDirect3DDevice8 has ninety-six methods"
        );
    }
}
