//! خطّاف دايركت٣د ١١ — the overlay drawn through the game's own D3D11 device.
//!
//! This is the backend the largest number of games reach. It is also the one
//! with the sharpest failure mode, because D3D11's immediate context is a single
//! mutable object the game owns for the whole frame: every bind the overlay
//! performs replaces one of the game's, and the game does not re-bind what it
//! believes is already bound. An overlay that draws and walks away leaves the
//! next draw call reading Taarib's vertex buffer through Taarib's input layout.
//! So [`KhattafD3D11::irsim`] reads the full pipeline before it touches
//! anything, and writes all of it back on every exit path including the ones
//! that failed.
//!
//! ## Why the shader compiler is resolved at runtime
//!
//! The obvious way to get DXBC into this file is `D3DCompile`, which lives
//! behind the `windows` crate's `Win32_Graphics_Direct3D_Fxc` feature. That
//! feature is not in this workspace's enabled list, and adding it would be the
//! wrong fix for two separate reasons.
//!
//! The first is that the feature only produces a link-time import of
//! `d3dcompiler_47.dll`. A process that does not have that DLL — and a shipped
//! game is under no obligation to carry it, because the shipping-time answer is
//! to compile shaders offline — would fail to load Taarib's module at all, with
//! a loader error naming a file the player has never heard of and no way for the
//! overlay to report anything. Resolving the export by hand turns that into a
//! [`KhataTabaqa::MaktabaMafquda`] the capability report can print.
//!
//! The second is that a build script producing pre-compiled bytecode would need
//! `fxc` on the build machine, which means the overlay could not be built on
//! Linux, which is where most of this workspace is built.
//!
//! So the module carries HLSL text, finds `d3dcompiler_47.dll` if the process
//! already has it and loads it if not, resolves `D3DCompile` through
//! `GetProcAddress`, and declares the signature itself. The declaration is
//! transcribed from `d3dcompiler.h` once, on `DallatTasrif`, and is the one
//! place in this file where getting a type wrong would not be a compile error.
//!
//! ## Premultiplied alpha, and the sRGB correction that goes with it
//!
//! Glyph coverage arrives from `saff` already premultiplied, so the blend is
//! `ONE, INV_SRC_ALPHA` rather than `SRC_ALPHA, INV_SRC_ALPHA`. That part is
//! mechanical. The part that is not is the transfer function: the quads carry
//! *linear* colour, and a backbuffer whose format has no `_SRGB` suffix holds
//! sRGB-encoded values that the output-merger will not encode for us. Blending
//! linear text into that buffer produces Arabic that is visibly too dark on a
//! light background while every individual step looks right, which is the single
//! most common way an overlay is wrong. The pixel shader therefore encodes when
//! the target format says nobody else will — see [`WasfSath::sirgb`].
//!
//! ## The one thing the hook has to do for this backend
//!
//! `ResizeBuffers` refuses while anybody holds a backbuffer, and this backend
//! holds one in its render target view for the whole session. The hook calls
//! [`crate::wajiha::Tabaqa::qabl_taghyeer_hajm`] before it forwards the resize
//! and nothing else; that reaches [`KhattafD3D11::atliq_khalfiyat`] through
//! [`Khattaf::atliq_sath`], which is why the release is a trait method and not
//! only an inherent one — the hook holds a `dyn Khattaf` and cannot name this
//! type. There is no state to hand over and no ordering to get right beyond
//! "before". The rebuild afterwards is this backend's own, because a resize that
//! keeps the same width, height and format is invisible to everything above
//! this file — see that method for why.
//!
//! ## What [`crate::d3d12`] takes from this file
//!
//! The shader source, the vertex layout, the quad filler, the projection
//! constants and the compiler loader are `pub(crate)` and the D3D12 backend uses
//! them rather than carrying its own. That is not sharing for its own sake: the
//! two backends draw the same glyphs out of the same atlas onto the same
//! surface, and a second copy of the shader is a second place where Arabic could
//! render differently on one API than the other, with nothing to catch it but a
//! player noticing. The one thing that genuinely differs — how the geometry
//! reaches the GPU — is what each file writes for itself.
//!
//! ## The reference counts the `*Get*` calls hand back
//!
//! `ID3D11DeviceContext::IAGetVertexBuffers` and its twenty siblings `AddRef`
//! every interface they write out. A C++ overlay that forgets the matching
//! `Release` leaks a few dozen objects per frame, which is a well-known bug with
//! a well-known symptom — a game that runs fine for ten minutes and then does
//! not. Here the release is the `Drop` of the saved-state struct: every
//! out-param is a `windows` wrapper that owns its reference, the struct is a
//! plain local, and `mem_forget` is denied workspace-wide, so there is no edit
//! that keeps the references alive by accident.

use core::ffi::c_void;
use std::fmt;
use std::mem;

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::{
    D3D_PRIMITIVE_TOPOLOGY, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D_SHADER_MACRO,
    D3D_SRV_DIMENSION_TEXTURE2D, ID3DBlob,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_SHADER_RESOURCE,
    D3D11_BIND_VERTEX_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE,
    D3D11_BLEND_OP_ADD, D3D11_BOX, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL,
    D3D11_COMPARISON_ALWAYS, D3D11_COMPARISON_NEVER, D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE,
    D3D11_CULL_NONE, D3D11_DEPTH_STENCIL_DESC, D3D11_DEPTH_STENCILOP_DESC,
    D3D11_DEPTH_WRITE_MASK_ZERO, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
    D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_MAP_READ, D3D11_MAP_WRITE_DISCARD,
    D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_SAMPLER_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0,
    D3D11_STENCIL_OP_KEEP, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC,
    D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC, D3D11_USAGE_IMMUTABLE,
    D3D11_USAGE_STAGING, D3D11_VIEWPORT, ID3D11BlendState, ID3D11Buffer, ID3D11ClassInstance,
    ID3D11DepthStencilState, ID3D11DepthStencilView, ID3D11Device, ID3D11DeviceContext,
    ID3D11DomainShader, ID3D11GeometryShader, ID3D11HullShader, ID3D11InputLayout,
    ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView, ID3D11SamplerState,
    ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_R10G10B10A2_UNORM,
    DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R16G16B16A16_FLOAT, DXGI_FORMAT_R32_FLOAT,
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use windows::core::{HRESULT, Interface, PCSTR, s};

use crate::khata::{KhataTabaqa, tul_u64};
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

/// The largest glyph atlas this build will upload, on either axis.
///
/// `D3D11_REQ_TEXTURE2D_U_OR_V_DIMENSION` is the hardware ceiling at feature
/// level 11, and the check is written against it rather than against a smaller
/// number Taarib picked: a refused atlas that the card would have accepted is a
/// missing script the user cannot explain.
pub(crate) const SAQF_LAWHA: u32 = 16_384;

/// How many quads one draw call covers.
///
/// The vertex buffer is sized to this and a batch larger than it is drawn in
/// several passes rather than refused. Four thousand quads is sixteen thousand
/// vertices, which keeps the index buffer inside `R16_UINT` — a 32-bit index
/// buffer would double the bandwidth of the one buffer the overlay re-uploads
/// every frame, to carry a batch size no dialogue box produces.
pub(crate) const QITA_LIL_DUFA: usize = 4_096;

/// How many class instances a `*GetShader` call may write.
///
/// D3D11 documents the array as needing room for up to 256 entries even though
/// `D3D11_SHADER_MAX_INTERFACES` is 253, and a short array is a buffer overrun
/// written by the runtime rather than by this crate. The larger number is used.
const ADAD_MUTAJASSIDAT: usize = 256;

/// How many render target slots the output merger has.
const ADAD_AHDAF: usize = 8;

/// How many viewports and scissor rectangles the rasterizer holds.
const ADAD_MANAZIR: usize = 16;

/// Where the surface position starts, in bytes, inside [`Ras`].
pub(crate) const IZAHAT_MAWDI: u32 = 0;

/// Where the atlas coordinates start inside [`Ras`].
pub(crate) const IZAHAT_KHAREETA: u32 = 8;

/// Where the premultiplied colour starts inside [`Ras`].
pub(crate) const IZAHAT_LAWN: u32 = 16;

/// Where the atlas blend weight starts inside [`Ras`].
pub(crate) const IZAHAT_KHALT: u32 = 32;

/// How many bytes one vertex occupies.
pub(crate) const KHATWAT_RAS: u32 = 36;

// A field reordered without these offsets following it is a vertex layout that
// reads the wrong bytes: the text would still draw, in the wrong place, in the
// wrong colour, with no error anywhere. The agreement is checked at compile time
// instead of trusted.
const _: () = {
    assert!(size_of::<Ras>() == KHATWAT_RAS as usize, "Ras is not 36 bytes");
    assert!(mem::offset_of!(Ras, mawdi) == IZAHAT_MAWDI as usize, "Ras.mawdi moved");
    assert!(mem::offset_of!(Ras, khareeta) == IZAHAT_KHAREETA as usize, "Ras.khareeta moved");
    assert!(mem::offset_of!(Ras, lawn) == IZAHAT_LAWN as usize, "Ras.lawn moved");
    assert!(mem::offset_of!(Ras, khalt) == IZAHAT_KHALT as usize, "Ras.khalt moved");
};

/// The overlay's whole shader program, as HLSL text.
///
/// Both entry points live in one string because they share `Marhala`, and a
/// stage signature that disagrees between two strings is a link error at draw
/// time rather than at compile time.
///
/// Compiled at `vs_4_0` / `ps_4_0`, which every D3D11 device at feature level
/// `10_0` and above accepts. Feature level `9_x` would need the `_level_9_1`
/// profiles and a different constant buffer layout; a game running the D3D11
/// runtime against `9_x` hardware is not a game this overlay is for, and
/// pretending otherwise would mean carrying a second shader nobody could test.
pub(crate) const MASDAR_HLSL: &str = r"
cbuffer Thawabit : register(b0)
{
    float4x4 isqat;
    float4   dabt;
};

struct Dakhil
{
    float2 mawdi    : POSITION;
    float2 khareeta : TEXCOORD0;
    float4 lawn     : COLOR0;
    float  khalt    : TEXCOORD1;
};

struct Marhala
{
    float4 mawdi    : SV_POSITION;
    float2 khareeta : TEXCOORD0;
    float4 lawn     : COLOR0;
    float  khalt    : TEXCOORD1;
};

Texture2D    lawha  : register(t0);
SamplerState akhidh : register(s0);

Marhala ras(Dakhil d)
{
    Marhala m;
    m.mawdi    = mul(isqat, float4(d.mawdi, 0.0f, 1.0f));
    m.khareeta = d.khareeta;
    m.lawn     = d.lawn;
    m.khalt    = d.khalt;
    return m;
}

float3 ishfir(float3 khatti)
{
    float3 wati = khatti * 12.92f;
    float3 aali = 1.055f * pow(max(khatti, 0.0f), 1.0f / 2.4f) - 0.055f;
    return lerp(wati, aali, step(0.0031308f, khatti));
}

float4 biksel(Marhala m) : SV_TARGET
{
    float4 min_lawha = lawha.Sample(akhidh, m.khareeta);
    float4 masdar    = lerp(float4(1.0f, 1.0f, 1.0f, 1.0f), min_lawha, m.khalt);
    float4 natija    = m.lawn * masdar;
    if (dabt.x > 0.5f)
    {
        float  alfa  = max(natija.a, 0.00001f);
        float3 mufak = saturate(natija.rgb / alfa);
        natija.rgb   = ishfir(mufak) * alfa;
    }
    return natija;
}
";

/// `D3DCOMPILE_ENABLE_STRICTNESS | D3DCOMPILE_OPTIMIZATION_LEVEL3`, plus
/// row-major matrix packing.
///
/// Strictness is on because the legacy relaxations it disables are all things a
/// shader written today should not rely on, and row-major packing is on because
/// [`ThawabitIsqat`] is written row by row and the two must agree — the failure
/// when they do not is a transposed projection, which draws the whole overlay
/// mirrored about the diagonal and looks like a layout bug.
const AALAM_TASRIF: u32 = 0x0000_0008 | 0x0000_0800 | 0x0000_8000;

/// The signature of `D3DCompile`, declared here because the `windows` crate's
/// `Win32_Graphics_Direct3D_Fxc` feature is not enabled in this workspace.
///
/// Transcribed from `d3dcompiler.h`:
///
/// ```c
/// HRESULT WINAPI D3DCompile(
///     LPCVOID pSrcData, SIZE_T SrcDataSize, LPCSTR pSourceName,
///     const D3D_SHADER_MACRO *pDefines, ID3DInclude *pInclude,
///     LPCSTR pEntrypoint, LPCSTR pTarget, UINT Flags1, UINT Flags2,
///     ID3DBlob **ppCode, ID3DBlob **ppErrorMsgs);
/// ```
///
/// `pInclude` is `*mut c_void` rather than a typed interface because the overlay
/// never passes one: there is exactly one translation unit and it is a `&'static
/// str` in this file.
type DallatTasrif = unsafe extern "system" fn(
    *const c_void,
    usize,
    PCSTR,
    *const D3D_SHADER_MACRO,
    *mut c_void,
    PCSTR,
    PCSTR,
    u32,
    u32,
    *mut *mut c_void,
    *mut *mut c_void,
) -> HRESULT;

/// The HLSL compiler, resolved out of `d3dcompiler_47.dll` at runtime.
///
/// Shared with [`crate::d3d12`], which needs the same function against the same
/// DLL and would otherwise carry a second copy of the loader — two copies being
/// two chances for one of them to stop checking a return value.
#[derive(Clone, Copy)]
pub(crate) struct MusarrifHlsl {
    dalla: DallatTasrif,
}

impl fmt::Debug for MusarrifHlsl {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The address is deliberately not printed. It is a module base plus an
        // offset in somebody else's process, it appears in diagnostics bundles
        // users paste in public, and it says nothing this line does not.
        mukhraj.write_str("MusarrifHlsl(d3dcompiler_47.dll!D3DCompile)")
    }
}

impl MusarrifHlsl {
    /// Finds `D3DCompile`, loading `d3dcompiler_47.dll` if the process has not.
    ///
    /// `GetModuleHandleA` is tried first so that a game which already has the
    /// compiler loaded — every game that builds shaders at startup, which is
    /// most of them — costs nothing but a lookup. `LoadLibraryA` is the fallback
    /// and is the case that can fail.
    ///
    /// The module reference is deliberately never released. Releasing it would
    /// leave [`MusarrifHlsl::dalla`] pointing into an unmapped page the moment
    /// the game's own reference went away, and there is no benefit to weigh
    /// against that: `d3dcompiler_47.dll` is a Microsoft-signed redistributable
    /// that costs a few hundred kilobytes of shared, already-paged code.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MaktabaMafquda`] when the DLL is not present or does not
    /// export `D3DCompile`, which is the whole reason this is resolved by hand
    /// rather than imported.
    pub(crate) fn ihdar() -> Result<Self, KhataTabaqa> {
        // SAFETY: `GetModuleHandleA` reads a NUL-terminated ASCII literal
        // produced by `s!` and returns a borrowed, non-owning handle or an
        // error. Nothing is written and no ownership is taken.
        let wahda = unsafe { GetModuleHandleA(s!("d3dcompiler_47.dll")) };
        let wahda = match wahda {
            Ok(wahda) => wahda,
            // SAFETY: same NUL-terminated literal. `LoadLibraryA` takes a
            // module reference that this process holds for its lifetime, which
            // is stated in the doc comment above and is why it is never freed.
            Err(_) => unsafe { LoadLibraryA(s!("d3dcompiler_47.dll")) }.map_err(|khata| {
                KhataTabaqa::MaktabaMafquda {
                    maktaba: format!("d3dcompiler_47.dll ({khata})"),
                    ramz: "D3DCompile".to_owned(),
                }
            })?,
        };

        // SAFETY: `wahda` is a live module handle from the call immediately
        // above, and the name is a NUL-terminated ASCII literal. The returned
        // `FARPROC` is `None` when the export is absent, which is checked.
        let ramz = unsafe { GetProcAddress(wahda, s!("D3DCompile")) };
        let Some(ramz) = ramz else {
            return Err(KhataTabaqa::MaktabaMafquda {
                maktaba: "d3dcompiler_47.dll".to_owned(),
                ramz: "D3DCompile".to_owned(),
            });
        };

        // SAFETY: `ramz` is the address of `D3DCompile` inside a loaded
        // `d3dcompiler_47.dll`, whose exported signature is the one transcribed
        // on `DallatTasrif` from `d3dcompiler.h`. Both sides are `extern
        // "system"` function pointers of identical size, so the transmute is a
        // reinterpretation of one pointer as another with a matching ABI.
        let dalla = unsafe {
            mem::transmute::<unsafe extern "system" fn() -> isize, DallatTasrif>(ramz)
        };
        Ok(Self { dalla })
    }

    /// Compiles one entry point out of [`MASDAR_HLSL`] and returns its bytecode.
    ///
    /// The bytecode is returned as an owned `Vec<u8>` rather than as the
    /// `ID3DBlob` the compiler produced, because the blob is a COM object from a
    /// DLL this crate resolved by hand and keeping one alive for the lifetime of
    /// the backend would mean the backend outliving the compiler is a use after
    /// free rather than a copy that is simply already made.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] naming the entry point, carrying the
    /// compiler's own diagnostics when it produced any. A shader that will not
    /// compile is a bug in this file rather than anything the user did, and the
    /// message is what makes that visible in a report.
    pub(crate) fn sarrif(
        self,
        mawrid: &'static str,
        madkhal: PCSTR,
        hadaf: PCSTR,
    ) -> Result<Vec<u8>, KhataTabaqa> {
        let mut ramz: *mut c_void = core::ptr::null_mut();
        let mut akhta: *mut c_void = core::ptr::null_mut();

        // SAFETY: `MASDAR_HLSL` is a live `&'static str`, so the pointer and
        // length describe a readable region for the duration of the call.
        // `madkhal` and `hadaf` are NUL-terminated ASCII literals. The defines
        // and include pointers are null, which `D3DCompile` documents as "none".
        // Both out-pointers address live locals and are read back below.
        let natija = unsafe {
            (self.dalla)(
                MASDAR_HLSL.as_ptr().cast::<c_void>(),
                MASDAR_HLSL.len(),
                s!("taarib-tabaqa.hlsl"),
                core::ptr::null(),
                core::ptr::null_mut(),
                madkhal,
                hadaf,
                AALAM_TASRIF,
                0,
                &raw mut ramz,
                &raw mut akhta,
            )
        };

        let bayan = if akhta.is_null() {
            None
        } else {
            // SAFETY: `D3DCompile` returned a non-null `ID3DBlob*` with a
            // reference this call owns. `from_raw` takes that reference, so the
            // blob is released when `kutla` is dropped at the end of this block.
            let kutla = unsafe { ID3DBlob::from_raw(akhta) };
            // SAFETY: `GetBufferPointer` and `GetBufferSize` describe the blob's
            // own allocation, which is alive while `kutla` is. The compiler's
            // diagnostics are ASCII with a trailing NUL, so the bytes are read
            // as a lossy UTF-8 string and trimmed rather than assumed valid.
            let nass = unsafe {
                let asas = kutla.GetBufferPointer().cast::<u8>();
                let tul = kutla.GetBufferSize();
                if asas.is_null() || tul == 0 {
                    String::new()
                } else {
                    String::from_utf8_lossy(core::slice::from_raw_parts(asas, tul)).into_owned()
                }
            };
            let nass = nass.trim_end_matches('\0').trim().to_owned();
            if nass.is_empty() { None } else { Some(nass) }
        };

        if let Err(khata) = natija.ok() {
            if !ramz.is_null() {
                // SAFETY: a non-null code blob alongside a failing HRESULT is
                // unusual but permitted; the reference is taken and dropped so
                // it is not leaked on the error path.
                drop(unsafe { ID3DBlob::from_raw(ramz) });
            }
            return Err(KhataTabaqa::MawridFashil {
                mawrid,
                sabab: bayan.map_or_else(|| khata.to_string(), |bayan| format!("{khata}: {bayan}")),
            });
        }

        if ramz.is_null() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid,
                sabab: "the compiler reported success and produced no bytecode".to_owned(),
            });
        }

        // SAFETY: `ramz` is a non-null `ID3DBlob*` the compiler produced and
        // this call owns; `from_raw` takes that reference and releases it when
        // `kutla` drops at the end of the function.
        let kutla = unsafe { ID3DBlob::from_raw(ramz) };
        // SAFETY: the blob's buffer is alive for as long as `kutla` is, and the
        // pointer and length it reports describe exactly that buffer.
        let bayt = unsafe {
            let asas = kutla.GetBufferPointer().cast::<u8>();
            let tul = kutla.GetBufferSize();
            if asas.is_null() || tul == 0 {
                Vec::new()
            } else {
                core::slice::from_raw_parts(asas, tul).to_vec()
            }
        };

        if bayt.is_empty() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid,
                sabab: "the compiler produced an empty bytecode blob".to_owned(),
            });
        }
        Ok(bayt)
    }
}

/// One corner of one quad, in the layout the input assembler is told about.
///
/// `khalt` is the weight the pixel shader gives the atlas sample: one for a
/// glyph, zero for a backing plate. It is a per-vertex float rather than a
/// second pipeline state because a batch mixes plates and glyphs freely and
/// splitting the draw on that boundary would turn one call into dozens.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct Ras {
    /// Surface position in pixels, origin top-left.
    mawdi: [f32; 2],
    /// Atlas coordinates, zero to one.
    khareeta: [f32; 2],
    /// Premultiplied linear RGBA.
    lawn: [f32; 4],
    /// One to sample the atlas, zero to use the colour alone.
    khalt: f32,
}

/// What the vertex and pixel shaders share, as one constant buffer.
///
/// Sixteen-byte aligned by construction: a four-by-four matrix followed by one
/// `float4`, which is eighty bytes. D3D11 rejects a constant buffer whose size
/// is not a multiple of sixteen, and it rejects it at creation rather than at
/// draw, so getting this wrong is caught immediately — which is the only reason
/// the padding is a real field carrying real data instead of dead bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct ThawabitIsqat {
    /// Pixels to clip space, row-major to match [`AALAM_TASRIF`].
    isqat: [[f32; 4]; 4],
    /// `x` is one when the shader must encode sRGB itself; the rest is reserved.
    dabt: [f32; 4],
}

impl ThawabitIsqat {
    /// The projection for a surface, and whether the shader must encode.
    ///
    /// The matrix maps a pixel with the origin at the top left onto clip space,
    /// which is what every position in [`crate::wajiha::QitaRasm`] is expressed
    /// in. Building it here rather than at the call site means the y flip exists
    /// once: a second copy that forgot it would draw the overlay upside down and
    /// still pass every dimension check.
    pub(crate) fn min_sath(sath: WasfSath) -> Self {
        let ard = madaa_f32(sath.ard.max(1));
        let irtifa = madaa_f32(sath.irtifa.max(1));
        Self {
            isqat: [
                [2.0 / ard, 0.0, 0.0, -1.0],
                [0.0, -2.0 / irtifa, 0.0, 1.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            // The hardware encodes for an `_SRGB` target and for nothing else.
            // A float target is scRGB and is already linear, which
            // `SighatSath::wasi_al_mada` is the authority on.
            dabt: [
                if sath.sirgb || sath.sigha.wasi_al_mada() { 0.0 } else { 1.0 },
                0.0,
                0.0,
                0.0,
            ],
        }
    }
}

/// The class instances one shader stage had bound.
///
/// Carried as an owning array rather than as raw pointers because every entry
/// the runtime wrote is a reference this crate now holds, and the array's `Drop`
/// is what returns them. See the note about `*Get*` in the module header.
#[derive(Debug)]
struct Mutajassidat {
    qaima: [Option<ID3D11ClassInstance>; ADAD_MUTAJASSIDAT],
    adad: u32,
}

impl Mutajassidat {
    /// An empty array for a `*GetShader` call to fill.
    fn jadeeda() -> Self {
        Self { qaima: core::array::from_fn(|_| None), adad: 0 }
    }

    /// The prefix the runtime actually wrote.
    ///
    /// Empty when the stage had no class instances, which is the case for every
    /// shader that does not use interfaces — that is to say, almost all of them.
    fn shariha(&self) -> &[Option<ID3D11ClassInstance>] {
        let adad = usize::try_from(self.adad).unwrap_or(0).min(ADAD_MUTAJASSIDAT);
        self.qaima.get(..adad).unwrap_or(&[])
    }
}

/// Everything the overlay is about to overwrite on the immediate context.
///
/// The list is exactly what [`KhattafD3D11::irsim`] binds, at exactly the slots
/// it binds them, and nothing else. That is a deliberate reading of "complete":
/// the overlay writes vertex buffer slot zero and never touches the other
/// thirty-one, so saving all thirty-two would be sixty-two reference count
/// operations every frame in service of restoring values that were never
/// changed. What is here is what would otherwise be wrong.
#[derive(Debug)]
struct HalatMasar {
    takhtit: Option<ID3D11InputLayout>,
    tobologia: D3D_PRIMITIVE_TOPOLOGY,
    ruus: Option<ID3D11Buffer>,
    khatwa: u32,
    izaha: u32,
    faharis: Option<ID3D11Buffer>,
    sighat_faharis: DXGI_FORMAT,
    izahat_faharis: u32,
    thawabit_ras: Option<ID3D11Buffer>,
    thawabit_biksel: Option<ID3D11Buffer>,
    mawrid_biksel: Option<ID3D11ShaderResourceView>,
    akhidh_biksel: Option<ID3D11SamplerState>,
    shader_ras: Option<ID3D11VertexShader>,
    mutajassidat_ras: Mutajassidat,
    shader_biksel: Option<ID3D11PixelShader>,
    mutajassidat_biksel: Mutajassidat,
    shader_handasi: Option<ID3D11GeometryShader>,
    mutajassidat_handasi: Mutajassidat,
    shader_qishri: Option<ID3D11HullShader>,
    mutajassidat_qishri: Mutajassidat,
    shader_majali: Option<ID3D11DomainShader>,
    mutajassidat_majali: Mutajassidat,
    khalt: Option<ID3D11BlendState>,
    amil_khalt: [f32; 4],
    qina_ayyina: u32,
    umq: Option<ID3D11DepthStencilState>,
    marji_tazlil: u32,
    ahdaf: [Option<ID3D11RenderTargetView>; ADAD_AHDAF],
    ru2yat_umq: Option<ID3D11DepthStencilView>,
    munaqqit: Option<ID3D11RasterizerState>,
    manazir: [D3D11_VIEWPORT; ADAD_MANAZIR],
    adad_manazir: u32,
    maqassat: [RECT; ADAD_MANAZIR],
    adad_maqassat: u32,
}

impl HalatMasar {
    /// Reads the pipeline as the game left it.
    ///
    /// Every out-parameter starts as `None` or as a zeroed value, which matters:
    /// the runtime writes a raw pointer straight into the slot, and a slot that
    /// already held an interface would have leaked it. Starting empty is what
    /// makes "the wrapper owns what was written" true.
    fn iltaqit(siyaq: &ID3D11DeviceContext) -> Self {
        let mut hala = Self {
            takhtit: None,
            tobologia: D3D_PRIMITIVE_TOPOLOGY::default(),
            ruus: None,
            khatwa: 0,
            izaha: 0,
            faharis: None,
            sighat_faharis: DXGI_FORMAT::default(),
            izahat_faharis: 0,
            thawabit_ras: None,
            thawabit_biksel: None,
            mawrid_biksel: None,
            akhidh_biksel: None,
            shader_ras: None,
            mutajassidat_ras: Mutajassidat::jadeeda(),
            shader_biksel: None,
            mutajassidat_biksel: Mutajassidat::jadeeda(),
            shader_handasi: None,
            mutajassidat_handasi: Mutajassidat::jadeeda(),
            shader_qishri: None,
            mutajassidat_qishri: Mutajassidat::jadeeda(),
            shader_majali: None,
            mutajassidat_majali: Mutajassidat::jadeeda(),
            khalt: None,
            amil_khalt: [0.0; 4],
            qina_ayyina: 0,
            umq: None,
            marji_tazlil: 0,
            ahdaf: core::array::from_fn(|_| None),
            ru2yat_umq: None,
            munaqqit: None,
            manazir: [D3D11_VIEWPORT::default(); ADAD_MANAZIR],
            adad_manazir: 0,
            maqassat: [RECT::default(); ADAD_MANAZIR],
            adad_maqassat: 0,
        };

        // SAFETY: `siyaq` is a live immediate context. Every pointer below
        // addresses a field of `hala`, which is a local that outlives the block,
        // and every slice is a real array of the length the runtime is told.
        // The counts handed to `RSGetViewports` and `RSGetScissorRects` are the
        // array capacities, which is what those two read on the way in.
        unsafe {
            hala.takhtit = siyaq.IAGetInputLayout().ok();
            hala.tobologia = siyaq.IAGetPrimitiveTopology();
            siyaq.IAGetVertexBuffers(
                0,
                1,
                Some(&raw mut hala.ruus),
                Some(&raw mut hala.khatwa),
                Some(&raw mut hala.izaha),
            );
            siyaq.IAGetIndexBuffer(
                Some(&raw mut hala.faharis),
                Some(&raw mut hala.sighat_faharis),
                Some(&raw mut hala.izahat_faharis),
            );

            // Moved out rather than cloned: the interface the runtime wrote into
            // the array already carries the reference this struct is taking
            // over, and cloning it would add a second one for nothing.
            let mut wahid = [None];
            siyaq.VSGetConstantBuffers(0, Some(&mut wahid));
            hala.thawabit_ras = wahid.into_iter().next().flatten();
            let mut wahid = [None];
            siyaq.PSGetConstantBuffers(0, Some(&mut wahid));
            hala.thawabit_biksel = wahid.into_iter().next().flatten();

            let mut mawarid = [None];
            siyaq.PSGetShaderResources(0, Some(&mut mawarid));
            hala.mawrid_biksel = mawarid.into_iter().next().flatten();
            let mut akhidhat = [None];
            siyaq.PSGetSamplers(0, Some(&mut akhidhat));
            hala.akhidh_biksel = akhidhat.into_iter().next().flatten();

            siyaq.VSGetShader(
                &raw mut hala.shader_ras,
                Some(hala.mutajassidat_ras.qaima.as_mut_ptr()),
                Some(&raw mut hala.mutajassidat_ras.adad),
            );
            siyaq.PSGetShader(
                &raw mut hala.shader_biksel,
                Some(hala.mutajassidat_biksel.qaima.as_mut_ptr()),
                Some(&raw mut hala.mutajassidat_biksel.adad),
            );
            siyaq.GSGetShader(
                &raw mut hala.shader_handasi,
                Some(hala.mutajassidat_handasi.qaima.as_mut_ptr()),
                Some(&raw mut hala.mutajassidat_handasi.adad),
            );
            siyaq.HSGetShader(
                &raw mut hala.shader_qishri,
                Some(hala.mutajassidat_qishri.qaima.as_mut_ptr()),
                Some(&raw mut hala.mutajassidat_qishri.adad),
            );
            siyaq.DSGetShader(
                &raw mut hala.shader_majali,
                Some(hala.mutajassidat_majali.qaima.as_mut_ptr()),
                Some(&raw mut hala.mutajassidat_majali.adad),
            );

            siyaq.OMGetBlendState(
                Some(&raw mut hala.khalt),
                Some(&mut hala.amil_khalt),
                Some(&raw mut hala.qina_ayyina),
            );
            siyaq.OMGetDepthStencilState(
                Some(&raw mut hala.umq),
                Some(&raw mut hala.marji_tazlil),
            );
            siyaq.OMGetRenderTargets(Some(&mut hala.ahdaf), Some(&raw mut hala.ru2yat_umq));

            hala.munaqqit = siyaq.RSGetState().ok();
            hala.adad_manazir = ADAD_MANAZIR_U32;
            siyaq.RSGetViewports(&raw mut hala.adad_manazir, Some(hala.manazir.as_mut_ptr()));
            hala.adad_maqassat = ADAD_MANAZIR_U32;
            siyaq.RSGetScissorRects(&raw mut hala.adad_maqassat, Some(hala.maqassat.as_mut_ptr()));
        }

        hala
    }

    /// Writes the pipeline back exactly as it was read.
    ///
    /// Every one of these setters returns nothing and cannot fail — D3D11 state
    /// changes are recorded, not validated, and the runtime has no way to refuse
    /// one. That is why the check that the restore *worked* is not here but in
    /// [`KhattafD3D11::irsim`], where the device is asked whether it was removed
    /// while all of this was happening. A removed device is the one condition
    /// under which none of these calls did anything, and it is the only
    /// detectable restore failure this API has.
    fn istaid(&self, siyaq: &ID3D11DeviceContext) {
        // SAFETY: `siyaq` is the same live immediate context the state was read
        // from. Every interface passed back is one this struct owns, every slice
        // is borrowed from a field that outlives the call, and the viewport and
        // scissor counts are clamped to the arrays they index.
        unsafe {
            siyaq.IASetInputLayout(self.takhtit.as_ref());
            siyaq.IASetPrimitiveTopology(self.tobologia);
            siyaq.IASetVertexBuffers(
                0,
                1,
                Some(&raw const self.ruus),
                Some(&raw const self.khatwa),
                Some(&raw const self.izaha),
            );
            siyaq.IASetIndexBuffer(
                self.faharis.as_ref(),
                self.sighat_faharis,
                self.izahat_faharis,
            );

            siyaq.VSSetConstantBuffers(0, Some(core::slice::from_ref(&self.thawabit_ras)));
            siyaq.PSSetConstantBuffers(0, Some(core::slice::from_ref(&self.thawabit_biksel)));
            siyaq.PSSetShaderResources(0, Some(core::slice::from_ref(&self.mawrid_biksel)));
            siyaq.PSSetSamplers(0, Some(core::slice::from_ref(&self.akhidh_biksel)));

            siyaq.VSSetShader(self.shader_ras.as_ref(), Some(self.mutajassidat_ras.shariha()));
            siyaq.PSSetShader(
                self.shader_biksel.as_ref(),
                Some(self.mutajassidat_biksel.shariha()),
            );
            siyaq.GSSetShader(
                self.shader_handasi.as_ref(),
                Some(self.mutajassidat_handasi.shariha()),
            );
            siyaq.HSSetShader(
                self.shader_qishri.as_ref(),
                Some(self.mutajassidat_qishri.shariha()),
            );
            siyaq.DSSetShader(
                self.shader_majali.as_ref(),
                Some(self.mutajassidat_majali.shariha()),
            );

            siyaq.OMSetBlendState(self.khalt.as_ref(), Some(&self.amil_khalt), self.qina_ayyina);
            siyaq.OMSetDepthStencilState(self.umq.as_ref(), self.marji_tazlil);
            siyaq.OMSetRenderTargets(Some(&self.ahdaf), self.ru2yat_umq.as_ref());

            siyaq.RSSetState(self.munaqqit.as_ref());
            let adad = usize::try_from(self.adad_manazir).unwrap_or(0).min(ADAD_MANAZIR);
            siyaq.RSSetViewports(self.manazir.get(..adad));
            let adad = usize::try_from(self.adad_maqassat).unwrap_or(0).min(ADAD_MANAZIR);
            siyaq.RSSetScissorRects(self.maqassat.get(..adad));
        }
    }
}

/// [`ADAD_MANAZIR`] as the `u32` the rasterizer getters take.
const ADAD_MANAZIR_U32: u32 = 16;

// The two spellings of the viewport count must agree or the getters are told a
// capacity the arrays do not have, which the runtime would write past.
const _: () = {
    assert!(ADAD_MANAZIR == ADAD_MANAZIR_U32 as usize, "viewport counts disagree");
};

/// The Direct3D 11 overlay backend.
///
/// Holds the game's device and immediate context, the swap chain it was found
/// through, and every object the overlay draws with. The surface-sized ones are
/// rebuilt by [`Khattaf::hayyi`]; the atlas deliberately is not, because losing
/// the glyphs on an alt-tab would leave the overlay with nothing to draw and no
/// way to ask for it back until the font changed.
#[derive(Debug)]
pub struct KhattafD3D11 {
    jihaz: ID3D11Device,
    siyaq: ID3D11DeviceContext,
    silsila: IDXGISwapChain,
    musarrif: MusarrifHlsl,
    bayt_ras: Vec<u8>,
    bayt_biksel: Vec<u8>,
    shader_ras: Option<ID3D11VertexShader>,
    shader_biksel: Option<ID3D11PixelShader>,
    takhtit: Option<ID3D11InputLayout>,
    ruus: Option<ID3D11Buffer>,
    faharis: Option<ID3D11Buffer>,
    thawabit: Option<ID3D11Buffer>,
    khalt: Option<ID3D11BlendState>,
    munaqqit: Option<ID3D11RasterizerState>,
    umq: Option<ID3D11DepthStencilState>,
    akhidh: Option<ID3D11SamplerState>,
    lawha: Option<ID3D11Texture2D>,
    ru2yat_lawha: Option<ID3D11ShaderResourceView>,
    hadaf: Option<ID3D11RenderTargetView>,
    sath: Option<WasfSath>,
    musawwada: Vec<Ras>,
}

impl KhattafD3D11 {
    /// Builds a backend around a swap chain the hook already found.
    ///
    /// Everything here is read from the swap chain rather than created: the
    /// device is the game's, the context is the game's immediate context, and
    /// the overlay never creates a second one. A backend with its own device
    /// would be a backend drawing into a texture that has to be composited back,
    /// which is the design this crate exists to avoid.
    ///
    /// The HLSL compiler is resolved here, before any hook goes live, so a
    /// machine without `d3dcompiler_47.dll` produces a capability report instead
    /// of a first frame that fails inside a game's present call.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the swap chain will not hand over its
    /// device or its immediate context, [`KhataTabaqa::SathTaghayyar`] when it
    /// will not describe itself, and [`KhataTabaqa::MaktabaMafquda`] when the
    /// shader compiler is not on this machine.
    pub fn min_silsila(silsila: &IDXGISwapChain) -> Result<Self, KhataTabaqa> {
        // SAFETY: `silsila` is a live swap chain the caller obtained from the
        // process it is hooking. `GetDevice` performs a QueryInterface for the
        // requested IID and returns an owned reference or an error; nothing is
        // written through a raw pointer.
        let jihaz = unsafe { silsila.GetDevice::<ID3D11Device>() }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "the swap chain's D3D11 device",
                sabab: khata.to_string(),
            }
        })?;

        // SAFETY: `jihaz` is the live device from the call above.
        // `GetImmediateContext` returns an owned reference to the one context
        // the device has, or an error.
        let siyaq = unsafe { jihaz.GetImmediateContext() }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "the device's immediate context",
                sabab: khata.to_string(),
            }
        })?;

        // SAFETY: `silsila` is live and `GetDesc` fills a caller-owned
        // `DXGI_SWAP_CHAIN_DESC` the binding allocates on the stack.
        unsafe { silsila.GetDesc() }.map_err(|khata| KhataTabaqa::SathTaghayyar {
            sabab: format!("the swap chain would not describe itself: {khata}"),
        })?;

        let musarrif = MusarrifHlsl::ihdar()?;

        Ok(Self {
            jihaz,
            siyaq,
            silsila: silsila.clone(),
            musarrif,
            bayt_ras: Vec::new(),
            bayt_biksel: Vec::new(),
            shader_ras: None,
            shader_biksel: None,
            takhtit: None,
            ruus: None,
            faharis: None,
            thawabit: None,
            khalt: None,
            munaqqit: None,
            umq: None,
            akhidh: None,
            lawha: None,
            ru2yat_lawha: None,
            hadaf: None,
            sath: None,
            musawwada: Vec::new(),
        })
    }

    /// Releases the overlay's reference to the swap chain's backbuffer.
    ///
    /// `IDXGISwapChain::ResizeBuffers` fails with `DXGI_ERROR_INVALID_CALL` if
    /// *anybody* still holds a reference to a backbuffer, and this backend holds
    /// one for the whole session in its render target view. A game that resizes
    /// its window with the overlay installed would therefore get a refusal from
    /// a function that has never refused before, and would very reasonably treat
    /// that as fatal.
    ///
    /// So `crate::khataf` hooks `ResizeBuffers` and calls this immediately
    /// before forwarding, which is the whole of what the overlay has to do about
    /// resizing.
    ///
    /// Recovery is this backend's own, not the caller's, and that is the part
    /// worth stating: [`crate::wajiha::Tabaqa`] rebuilds when the surface's
    /// *description* changes, and a `ResizeBuffers` that keeps the same width,
    /// height and format changes nothing it can see. A borderless fullscreen
    /// transition on the monitor the window was already filling does exactly
    /// that. So the surface is forgotten here and [`Khattaf::irsim`] notices it
    /// is missing and rebuilds, rather than waiting for a notification that in
    /// that case never comes.
    ///
    /// Deliberately not part of [`Khattaf`]: the other two backends have no
    /// swap chain and nothing to release, and a trait method three of four
    /// implementations left empty would be a trait method nobody could tell was
    /// load-bearing.
    pub fn atliq_khalfiyat(&mut self) {
        self.hadaf = None;
        self.sath = None;
    }

    /// Drops every object [`Khattaf::hayyi`] creates, keeping the atlas.
    ///
    /// Called at the head of `hayyi` and from [`Khattaf::ahmil`]. Assigning
    /// `None` is the release: each field is a `windows` wrapper that owns one
    /// reference, and dropping it is the `Release`. Doing it as one function
    /// rather than inline in two places is what stops the two lists diverging,
    /// which is how a resize ends up leaking one buffer per resolution change.
    fn atlif_masar(&mut self) {
        self.hadaf = None;
        self.thawabit = None;
        self.ruus = None;
        self.faharis = None;
        self.takhtit = None;
        self.shader_ras = None;
        self.shader_biksel = None;
        self.khalt = None;
        self.munaqqit = None;
        self.umq = None;
        self.akhidh = None;
        self.sath = None;
    }

    /// Compiles the two shaders, once per process.
    ///
    /// The bytecode is cached in `bayt_ras` and `bayt_biksel` because `hayyi`
    /// runs on every resolution change and on every fullscreen transition, and
    /// `D3DCompile` is milliseconds of work on the render thread. A player
    /// alt-tabbing out of a borderless game changes the surface twice in quick
    /// succession; recompiling both times is a stall they would feel.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] carrying the compiler's diagnostics.
    fn ihdar_ramz(&mut self) -> Result<(), KhataTabaqa> {
        if self.bayt_ras.is_empty() {
            self.bayt_ras =
                self.musarrif.sarrif("overlay vertex shader", s!("ras"), s!("vs_4_0"))?;
        }
        if self.bayt_biksel.is_empty() {
            self.bayt_biksel =
                self.musarrif.sarrif("overlay pixel shader", s!("biksel"), s!("ps_4_0"))?;
        }
        Ok(())
    }

    /// Creates the shaders, the input layout and the three buffers.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] naming whichever object the device refused.
    fn ibni_masar(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        self.ihdar_ramz()?;

        let mut shader_ras = None;
        // SAFETY: `bayt_ras` holds DXBC the compiler produced from
        // `MASDAR_HLSL`, the class-linkage parameter is absent, and the
        // out-pointer addresses a local initialised to `None`, so the reference
        // the runtime writes is owned by that local.
        unsafe {
            self.jihaz.CreateVertexShader(&self.bayt_ras, None, Some(&raw mut shader_ras))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay vertex shader",
            sabab: khata.to_string(),
        })?;

        let mut shader_biksel = None;
        // SAFETY: as above, for the pixel stage.
        unsafe {
            self.jihaz.CreatePixelShader(&self.bayt_biksel, None, Some(&raw mut shader_biksel))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay pixel shader",
            sabab: khata.to_string(),
        })?;

        // The semantics and offsets here are checked against `Ras` by the
        // `const` assertions near the top of this file rather than by eye.
        let anasir = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_MAWDI,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHAREETA,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("COLOR"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_LAWN,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 1,
                Format: DXGI_FORMAT_R32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHALT,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];

        let mut takhtit = None;
        // SAFETY: the element array and the vertex bytecode are both live for
        // the call, and the out-pointer addresses a local initialised to `None`.
        // The runtime validates the layout against the shader's input signature
        // and fails rather than accepting a mismatch.
        unsafe {
            self.jihaz.CreateInputLayout(&anasir, &self.bayt_ras, Some(&raw mut takhtit))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay input layout",
            sabab: khata.to_string(),
        })?;

        let wasf_ruus = D3D11_BUFFER_DESC {
            ByteWidth: KHATWAT_RAS.saturating_mul(RUUS_LIL_DUFA),
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0.cast_unsigned(),
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let mut ruus = None;
        // SAFETY: the description is a fully initialised local, there is no
        // initial data for a dynamic buffer, and the out-pointer addresses a
        // local initialised to `None`.
        unsafe { self.jihaz.CreateBuffer(&raw const wasf_ruus, None, Some(&raw mut ruus)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay vertex buffer",
                sabab: khata.to_string(),
            })?;

        // The index buffer is the same six-index pattern repeated, so it is
        // built once and never touched again. Rebuilding it per frame alongside
        // the vertices is the obvious shape and it uploads the same bytes every
        // frame for no reason.
        let mut faharis_khaam: Vec<u16> = Vec::with_capacity(QITA_LIL_DUFA * 6);
        for qita in 0..QITA_LIL_DUFA {
            let asas = u16::try_from(qita * 4).unwrap_or(0);
            faharis_khaam.extend_from_slice(&[
                asas,
                asas.saturating_add(1),
                asas.saturating_add(2),
                asas,
                asas.saturating_add(2),
                asas.saturating_add(3),
            ]);
        }
        let wasf_faharis = D3D11_BUFFER_DESC {
            ByteWidth: u32::try_from(faharis_khaam.len() * size_of::<u16>()).unwrap_or(0),
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let bidaya_faharis = D3D11_SUBRESOURCE_DATA {
            pSysMem: faharis_khaam.as_ptr().cast::<c_void>(),
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut faharis = None;
        // SAFETY: `faharis_khaam` outlives the call, so the initial-data pointer
        // is valid for the `ByteWidth` bytes the description names — the two are
        // computed from the same vector. The out-pointer addresses a local.
        unsafe {
            self.jihaz.CreateBuffer(
                &raw const wasf_faharis,
                Some(&raw const bidaya_faharis),
                Some(&raw mut faharis),
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay index buffer",
            sabab: khata.to_string(),
        })?;

        let thawabit_khaam = ThawabitIsqat::min_sath(sath);
        let wasf_thawabit = D3D11_BUFFER_DESC {
            ByteWidth: u32::try_from(size_of::<ThawabitIsqat>()).unwrap_or(0),
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let bidaya_thawabit = D3D11_SUBRESOURCE_DATA {
            pSysMem: (&raw const thawabit_khaam).cast::<c_void>(),
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut thawabit = None;
        // SAFETY: `thawabit_khaam` is a live local of exactly the size the
        // description declares, and the out-pointer addresses a local.
        unsafe {
            self.jihaz.CreateBuffer(
                &raw const wasf_thawabit,
                Some(&raw const bidaya_thawabit),
                Some(&raw mut thawabit),
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay projection constant buffer",
            sabab: khata.to_string(),
        })?;

        self.shader_ras = shader_ras;
        self.shader_biksel = shader_biksel;
        self.takhtit = takhtit;
        self.ruus = ruus;
        self.faharis = faharis;
        self.thawabit = thawabit;
        Ok(())
    }

    /// Creates the four fixed-function state objects.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] naming whichever state the device refused.
    fn ibni_halat(&mut self) -> Result<(), KhataTabaqa> {
        // Premultiplied alpha. `SRC_ALPHA, INV_SRC_ALPHA` would be the reflex
        // and it is wrong here: `saff` hands over coverage already multiplied
        // into the colour, and multiplying it a second time makes every glyph
        // edge darker than its interior — a halo that reads as a bad font.
        let khalt_hadaf = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true.into(),
            SrcBlend: D3D11_BLEND_ONE,
            DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: D3D11_BLEND_ONE,
            DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: u8::try_from(D3D11_COLOR_WRITE_ENABLE_ALL.0).unwrap_or(0x0F),
        };
        let wasf_khalt = D3D11_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            IndependentBlendEnable: false.into(),
            RenderTarget: [khalt_hadaf; ADAD_AHDAF],
        };
        let mut khalt = None;
        // SAFETY: the description is a fully initialised local and the
        // out-pointer addresses a local initialised to `None`.
        unsafe { self.jihaz.CreateBlendState(&raw const wasf_khalt, Some(&raw mut khalt)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay blend state",
                sabab: khata.to_string(),
            })?;

        // Scissor off and culling off. Culling off because the overlay's quads
        // are generated in one winding and a game that left `FrontCounterClock-
        // wise` set would otherwise cull every one of them, which looks exactly
        // like the overlay never ran.
        let wasf_munaqqit = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            FrontCounterClockwise: false.into(),
            DepthBias: 0,
            DepthBiasClamp: 0.0,
            SlopeScaledDepthBias: 0.0,
            DepthClipEnable: true.into(),
            ScissorEnable: false.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };
        let mut munaqqit = None;
        // SAFETY: as above.
        unsafe {
            self.jihaz.CreateRasterizerState(&raw const wasf_munaqqit, Some(&raw mut munaqqit))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay rasterizer state",
            sabab: khata.to_string(),
        })?;

        // Depth test *and* depth write both off. Turning off only the test is
        // the common half-fix: the overlay then draws on top correctly and
        // stamps its own quads into the depth buffer, and the game's next frame
        // finds a wall of geometry at the near plane where the subtitles were.
        let tazlil = D3D11_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D11_STENCIL_OP_KEEP,
            StencilDepthFailOp: D3D11_STENCIL_OP_KEEP,
            StencilPassOp: D3D11_STENCIL_OP_KEEP,
            StencilFunc: D3D11_COMPARISON_ALWAYS,
        };
        let wasf_umq = D3D11_DEPTH_STENCIL_DESC {
            DepthEnable: false.into(),
            DepthWriteMask: D3D11_DEPTH_WRITE_MASK_ZERO,
            DepthFunc: D3D11_COMPARISON_ALWAYS,
            StencilEnable: false.into(),
            StencilReadMask: 0,
            StencilWriteMask: 0,
            FrontFace: tazlil,
            BackFace: tazlil,
        };
        let mut umq = None;
        // SAFETY: as above.
        unsafe {
            self.jihaz.CreateDepthStencilState(&raw const wasf_umq, Some(&raw mut umq))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay depth-stencil state",
            sabab: khata.to_string(),
        })?;

        // Clamped addressing so a glyph whose atlas rectangle sits against the
        // edge cannot bleed a neighbouring glyph's pixels into itself, and
        // linear filtering because the overlay is drawn at whatever scale the
        // control panel's font size asks for rather than always at one to one.
        let wasf_akhidh = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MipLODBias: 0.0,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            BorderColor: [0.0; 4],
            MinLOD: 0.0,
            MaxLOD: 0.0,
        };
        let mut akhidh = None;
        // SAFETY: as above.
        unsafe {
            self.jihaz.CreateSamplerState(&raw const wasf_akhidh, Some(&raw mut akhidh))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay sampler state",
            sabab: khata.to_string(),
        })?;

        self.khalt = khalt;
        self.munaqqit = munaqqit;
        self.umq = umq;
        self.akhidh = akhidh;
        Ok(())
    }

    /// Creates a render target view over the swap chain's current backbuffer.
    ///
    /// The view is created with a null description so it inherits the buffer's
    /// own format, `_SRGB` suffix included. Naming a format here would be the
    /// place a resize to a different format silently kept drawing through a view
    /// that no longer matches.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SathTaghayyar`] when the backbuffer cannot be fetched,
    /// which is what a swap chain mid-resize reports, and
    /// [`KhataTabaqa::MawridFashil`] when the view itself will not be created.
    fn ibni_hadaf(&mut self) -> Result<(), KhataTabaqa> {
        // SAFETY: `silsila` is live. `GetBuffer` performs a QueryInterface on
        // the named buffer and returns an owned reference or an error.
        let khalfiya = unsafe { self.silsila.GetBuffer::<ID3D11Texture2D>(0) }.map_err(
            |khata| KhataTabaqa::SathTaghayyar {
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;

        let mut hadaf = None;
        // SAFETY: `khalfiya` is a live texture, the null description asks the
        // runtime to take the resource's own format, and the out-pointer
        // addresses a local initialised to `None`.
        unsafe { self.jihaz.CreateRenderTargetView(&khalfiya, None, Some(&raw mut hadaf)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay render target view",
                sabab: khata.to_string(),
            })?;

        self.hadaf = hadaf;
        Ok(())
    }

    /// Draws every chunk of a batch through an already-bound pipeline.
    ///
    /// Split out of [`Khattaf::irsim`] so that the save, the draw and the
    /// restore are three visibly separate things: the restore must run whatever
    /// this returns, and a draw written inline with early returns in it is how
    /// that stops being true.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when a resource is missing or the vertex
    /// buffer cannot be mapped.
    fn arsim_dufaat(
        &self,
        musawwada: &mut Vec<Ras>,
        lawha: &LawhatRasm,
    ) -> Result<(), KhataTabaqa> {
        let (
            Some(shader_ras),
            Some(shader_biksel),
            Some(takhtit),
            Some(ruus),
            Some(faharis),
            Some(thawabit),
            Some(khalt),
            Some(munaqqit),
            Some(umq),
            Some(akhidh),
            Some(hadaf),
        ) = (
            self.shader_ras.as_ref(),
            self.shader_biksel.as_ref(),
            self.takhtit.as_ref(),
            self.ruus.as_ref(),
            self.faharis.as_ref(),
            self.thawabit.as_ref(),
            self.khalt.as_ref(),
            self.munaqqit.as_ref(),
            self.umq.as_ref(),
            self.akhidh.as_ref(),
            self.hadaf.as_ref(),
        )
        else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay pipeline",
                sabab: "the backend was asked to draw before its resources were built".to_owned(),
            });
        };

        // A batch that samples the atlas without one uploaded would draw
        // rectangles of whatever t0 happens to contain, which on some drivers is
        // the previous frame. Refusing is the only honest answer.
        if self.ru2yat_lawha.is_none() && lawha.qitaat.iter().any(|qita| qita.khareeta.is_some()) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the batch samples an atlas that has not been uploaded".to_owned(),
            });
        }

        let manzar = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: madaa_f32(lawha.sath.ard),
            Height: madaa_f32(lawha.sath.irtifa),
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        let ruus_marbut = [Some(ruus.clone())];
        let khatwa = KHATWAT_RAS;
        let izaha = 0_u32;
        let amil_khalt = [0.0_f32; 4];

        // SAFETY: every interface bound below is one this backend owns and holds
        // alive for the whole function, every slice is a live local, and the two
        // raw pointers handed to `IASetVertexBuffers` address locals declared
        // immediately above. Binding cannot fail; it is recorded, not validated.
        unsafe {
            self.siyaq.IASetInputLayout(Some(takhtit));
            self.siyaq.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.siyaq.IASetVertexBuffers(
                0,
                1,
                Some(ruus_marbut.as_ptr()),
                Some(&raw const khatwa),
                Some(&raw const izaha),
            );
            self.siyaq.IASetIndexBuffer(Some(faharis), DXGI_FORMAT_R16_UINT, 0);

            self.siyaq.VSSetShader(Some(shader_ras), None);
            self.siyaq.VSSetConstantBuffers(0, Some(&[Some(thawabit.clone())]));
            self.siyaq.PSSetShader(Some(shader_biksel), None);
            self.siyaq.PSSetConstantBuffers(0, Some(&[Some(thawabit.clone())]));
            self.siyaq.PSSetShaderResources(0, Some(core::slice::from_ref(&self.ru2yat_lawha)));
            self.siyaq.PSSetSamplers(0, Some(&[Some(akhidh.clone())]));

            // The three stages the overlay does not use are cleared rather than
            // left alone. A game with a geometry shader still bound would run it
            // over the overlay's triangles, and whatever it emitted would not be
            // Arabic.
            self.siyaq.GSSetShader(None, None);
            self.siyaq.HSSetShader(None, None);
            self.siyaq.DSSetShader(None, None);

            self.siyaq.OMSetBlendState(Some(khalt), Some(&amil_khalt), u32::MAX);
            self.siyaq.OMSetDepthStencilState(Some(umq), 0);
            self.siyaq.OMSetRenderTargets(Some(&[Some(hadaf.clone())]), None);

            self.siyaq.RSSetState(Some(munaqqit));
            self.siyaq.RSSetViewports(Some(&[manzar]));
            // Cleared, not left: the rasterizer state above disables scissoring,
            // but a driver that honours a stale rectangle anyway would clip the
            // overlay to wherever the game last drew a minimap.
            self.siyaq.RSSetScissorRects(Some(&[]));
        }

        for dufa in lawha.qitaat.chunks(QITA_LIL_DUFA) {
            musawwada.clear();
            for qita in dufa {
                imla_qita(musawwada, qita);
            }
            if musawwada.is_empty() {
                continue;
            }

            let mut marsum = D3D11_MAPPED_SUBRESOURCE::default();
            // SAFETY: `ruus` is the dynamic vertex buffer this backend created
            // with CPU write access, and `WRITE_DISCARD` is the documented map
            // type for it. The out-pointer addresses a live local.
            unsafe {
                self.siyaq.Map(ruus, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&raw mut marsum))
            }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay vertex buffer",
                sabab: format!("the buffer could not be mapped for writing: {khata}"),
            })?;

            if marsum.pData.is_null() {
                // SAFETY: the map above succeeded, so the buffer is mapped and
                // must be unmapped before returning even though the pointer is
                // unusable.
                unsafe { self.siyaq.Unmap(ruus, 0) };
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay vertex buffer",
                    sabab: "the map succeeded and returned no pointer".to_owned(),
                });
            }

            // SAFETY: the buffer was created with room for `RUUS_LIL_DUFA`
            // vertices and `musawwada` holds at most four per quad in a chunk of
            // at most `QITA_LIL_DUFA` quads, so the copy is inside the mapping.
            // Source and destination are distinct allocations.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    musawwada.as_ptr(),
                    marsum.pData.cast::<Ras>(),
                    musawwada.len(),
                );
                self.siyaq.Unmap(ruus, 0);
            }

            // Six indices per quad, and a chunk is at most `QITA_LIL_DUFA`
            // quads, so this cannot overflow a `u32`.
            let adad = u32::try_from(dufa.len().saturating_mul(6)).unwrap_or(0);
            // SAFETY: the pipeline is bound above, the index buffer holds
            // exactly `QITA_LIL_DUFA * 6` indices, and `adad` is bounded by
            // that. The vertices those indices reach were written immediately
            // above.
            unsafe { self.siyaq.DrawIndexed(adad, 0, 0) };
        }

        Ok(())
    }
}

impl Khattaf for KhattafD3D11 {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Direct3D11
    }

    /// Drops the render target view, which is the only backbuffer reference
    /// this backend holds.
    ///
    /// Infallible here, unlike on D3D12: a D3D11 view is released by dropping
    /// it and the immediate context has no outstanding GPU work to wait on
    /// that the runtime does not already track. The trait's signature carries
    /// a `Result` because D3D12 genuinely needs one.
    fn atliq_sath(&mut self) -> Result<(), KhataTabaqa> {
        self.atliq_khalfiyat();
        Ok(())
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        // SAFETY: `silsila` is the live swap chain this backend was built
        // around, and `GetDesc` fills a stack description the binding owns.
        let wasf = unsafe { self.silsila.GetDesc() }.map_err(|khata| {
            KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain would not describe itself: {khata}"),
            }
        })?;

        let ard = wasf.BufferDesc.Width;
        let irtifa = wasf.BufferDesc.Height;
        if ard == 0 || irtifa == 0 {
            // A zero dimension is what a swap chain reports between a
            // `ResizeBuffers` and the mode change completing. It is a skipped
            // frame, not a fault, which is exactly what `SathTaghayyar` means.
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain reports a {ard}×{irtifa} surface"),
            });
        }

        let (sigha, sirgb) = sigha_min_dxgi(wasf.BufferDesc.Format)?;
        Ok(WasfSath { ard, irtifa, sigha, sirgb })
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        // Released before anything is allocated, and released unconditionally:
        // this runs both at startup, where there is nothing to release, and
        // after a resize, where holding the old render target view for one more
        // allocation would be holding a view of a backbuffer DXGI has already
        // freed.
        self.atlif_masar();

        self.ibni_masar(sath)?;
        self.ibni_halat()?;
        self.ibni_hadaf()?;

        // The scratch vertex buffer is grown once rather than per frame. It is
        // the only allocation on the draw path and it is made here, off it.
        let matlub = QITA_LIL_DUFA.saturating_mul(4);
        if self.musawwada.capacity() < matlub {
            self.musawwada.reserve(matlub.saturating_sub(self.musawwada.capacity()));
        }

        self.sath = Some(sath);
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if ard > SAQF_LAWHA {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas width",
                qeema: u64::from(ard),
                saqf: u64::from(SAQF_LAWHA),
            });
        }
        if irtifa > SAQF_LAWHA {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas height",
                qeema: u64::from(irtifa),
                saqf: u64::from(SAQF_LAWHA),
            });
        }
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: format!("a {ard}×{irtifa} atlas has no pixels to upload"),
            });
        }

        // Four bytes per pixel, stated by the trait. The check is on the whole
        // buffer rather than per row because a short buffer would be read past
        // the end by the driver, inside the game's process, with no error.
        let khatwa = ard.saturating_mul(4);
        let matlub = u64::from(khatwa).saturating_mul(u64::from(irtifa));
        if tul_u64(bayt.len()) < matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: format!(
                    "a {ard}×{irtifa} RGBA8 atlas needs {matlub} bytes and {} were supplied",
                    bayt.len()
                ),
            });
        }

        let wasf = D3D11_TEXTURE2D_DESC {
            Width: ard,
            Height: irtifa,
            MipLevels: 1,
            ArraySize: 1,
            // The atlas is premultiplied linear coverage, not colour anybody
            // encoded, so it is uploaded as plain `UNORM` and the shader is left
            // to do the one transfer this backend does at all.
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            // Immutable rather than default: the atlas is replaced wholesale
            // when it changes and never partially written, and immutable is the
            // usage that lets a driver put it wherever it likes.
            Usage: D3D11_USAGE_IMMUTABLE,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let bidaya = D3D11_SUBRESOURCE_DATA {
            pSysMem: bayt.as_ptr().cast::<c_void>(),
            SysMemPitch: khatwa,
            SysMemSlicePitch: 0,
        };

        let mut lawha = None;
        // SAFETY: `bayt` outlives the call and was checked above to hold at
        // least `khatwa * irtifa` bytes, which is exactly what the row pitch and
        // the height in the description tell the runtime to read. The
        // out-pointer addresses a local initialised to `None`.
        unsafe {
            self.jihaz.CreateTexture2D(
                &raw const wasf,
                Some(&raw const bidaya),
                Some(&raw mut lawha),
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: khata.to_string(),
        })?;

        let Some(lawha) = lawha else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "the runtime reported success and produced no texture".to_owned(),
            });
        };

        let wasf_ru2ya = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
            },
        };
        let mut ru2ya = None;
        // SAFETY: `lawha` is the texture created immediately above, the
        // description names the same format and the one mip level it has, and
        // the out-pointer addresses a local initialised to `None`.
        unsafe {
            self.jihaz.CreateShaderResourceView(
                &lawha,
                Some(&raw const wasf_ru2ya),
                Some(&raw mut ru2ya),
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas view",
            sabab: khata.to_string(),
        })?;

        // Both replaced together. Assigning the view while keeping the previous
        // texture would leave a view of an atlas nothing else references, which
        // draws the old glyphs until the driver notices.
        self.lawha = Some(lawha);
        self.ru2yat_lawha = ru2ya;
        Ok(())
    }

    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if self.sath.is_none() {
            // Reached after `atliq_khalfiyat` let the game resize. The caller
            // only calls `hayyi` when the surface description changes, and a
            // resize that keeps the same width, height and format changes
            // nothing it can see — so the rebuild is done here, against the
            // surface the caller already checked this batch was built for.
            self.hayyi(lawha.sath)?;
        }
        if self.sath != Some(lawha.sath) {
            // The caller checks this too. It is checked again because the check
            // that matters is against the surface this backend's resources were
            // *built* for, which is what this field records, and those are two
            // different facts on the frame a resize is discovered.
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay batch",
                sabab: "the batch was positioned against a surface this backend is not \
                        currently built for"
                    .to_owned(),
            });
        }

        // Read before anything is bound. The context is cloned rather than
        // borrowed so the draw below can take `&self` without the borrow checker
        // and the restore fighting over the same field.
        let siyaq = self.siyaq.clone();
        let hala = HalatMasar::iltaqit(&siyaq);

        let mut musawwada = mem::take(&mut self.musawwada);
        let natija = self.arsim_dufaat(&mut musawwada, lawha);
        musawwada.clear();
        self.musawwada = musawwada;

        // Unconditional, and before the result is inspected. There is no error
        // this backend can return that makes leaving the game's pipeline pointed
        // at Taarib's buffers the better outcome.
        hala.istaid(&siyaq);
        drop(hala);

        // Every D3D11 state setter returns nothing and cannot refuse, so the
        // restore above either happened or the device stopped existing part way
        // through it. That second case is the whole of what
        // `HalaGhayrMustaada` means here, and it is checked rather than assumed
        // because a removed device is exactly what a driver reset during an
        // overlay draw produces.
        //
        // SAFETY: `jihaz` is the live device this backend was built around.
        if let Err(khata) = unsafe { self.jihaz.GetDeviceRemovedReason() } {
            return Err(KhataTabaqa::HalaGhayrMustaada {
                hala: "the Direct3D 11 pipeline",
                sabab: format!("the device was removed while the overlay drew: {khata}"),
            });
        }

        natija
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = self.sath()?;
        let hudud_ard = mintaqa.yasar.saturating_add(mintaqa.ard);
        let hudud_irtifa = mintaqa.aala.saturating_add(mintaqa.irtifa);
        if mintaqa.ard == 0
            || mintaqa.irtifa == 0
            || hudud_ard > sath.ard
            || hudud_irtifa > sath.irtifa
        {
            return Err(KhataTabaqa::MintaqaKharij {
                mintaqa: format!(
                    "{}×{} at ({}, {})",
                    mintaqa.ard, mintaqa.irtifa, mintaqa.yasar, mintaqa.aala
                ),
                ard: sath.ard,
                irtifa: sath.irtifa,
            });
        }

        // SAFETY: `silsila` is live; `GetBuffer` returns an owned reference.
        let khalfiya = unsafe { self.silsila.GetBuffer::<ID3D11Texture2D>(0) }.map_err(
            |khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;

        let mut wasf_khalfiya = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `khalfiya` is a live texture and the out-pointer addresses a
        // fully initialised local of exactly the type the runtime writes.
        unsafe { khalfiya.GetDesc(&raw mut wasf_khalfiya) };

        // A multisampled backbuffer cannot be the source of a copy into a
        // single-sampled staging texture, and the copy fails silently on some
        // drivers rather than returning an error. Resolving first is not an
        // optimisation; it is the only way this path works at all on a game that
        // presents MSAA directly.
        let masdar = if wasf_khalfiya.SampleDesc.Count > 1 {
            let wasf_hall = D3D11_TEXTURE2D_DESC {
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_DEFAULT,
                // A default-usage texture with no bind flags at all is rejected
                // by the runtime, so the resolve target carries the one flag
                // every swap chain format is guaranteed to support. Nothing
                // samples it; it exists for one `CopySubresourceRegion`.
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0.cast_unsigned(),
                CPUAccessFlags: 0,
                MiscFlags: 0,
                MipLevels: 1,
                ArraySize: 1,
                ..wasf_khalfiya
            };
            let mut hall = None;
            // SAFETY: the description is a live local, there is no initial data,
            // and the out-pointer addresses a local initialised to `None`.
            unsafe { self.jihaz.CreateTexture2D(&raw const wasf_hall, None, Some(&raw mut hall)) }
                .map_err(|khata| KhataTabaqa::IltiqatFashil {
                    sabab: format!("the multisample resolve target could not be created: {khata}"),
                })?;
            let Some(hall) = hall else {
                return Err(KhataTabaqa::IltiqatFashil {
                    sabab: "the multisample resolve target was not produced".to_owned(),
                });
            };
            // SAFETY: both textures are live, both have one subresource at index
            // zero, and the format is the backbuffer's own on both sides, which
            // is what `ResolveSubresource` requires.
            unsafe {
                self.siyaq.ResolveSubresource(&hall, 0, &khalfiya, 0, wasf_khalfiya.Format);
            }
            hall
        } else {
            khalfiya
        };

        let wasf_marhala = D3D11_TEXTURE2D_DESC {
            Width: mintaqa.ard,
            Height: mintaqa.irtifa,
            MipLevels: 1,
            ArraySize: 1,
            Format: wasf_khalfiya.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0.cast_unsigned(),
            MiscFlags: 0,
        };
        let mut marhala = None;
        // SAFETY: the description is a live local, a staging texture takes no
        // initial data, and the out-pointer addresses a local set to `None`.
        unsafe {
            self.jihaz.CreateTexture2D(&raw const wasf_marhala, None, Some(&raw mut marhala))
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back texture could not be created: {khata}"),
        })?;
        let Some(marhala) = marhala else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back texture was not produced".to_owned(),
            });
        };

        let sunduq = D3D11_BOX {
            left: mintaqa.yasar,
            top: mintaqa.aala,
            front: 0,
            right: hudud_ard,
            bottom: hudud_irtifa,
            back: 1,
        };
        // SAFETY: both resources are live and single-subresource, and the box
        // was checked against the surface dimensions at the head of this
        // function, so it names a region that exists in the source and fits the
        // destination exactly.
        unsafe {
            self.siyaq.CopySubresourceRegion(
                &marhala,
                0,
                0,
                0,
                0,
                &masdar,
                0,
                Some(&raw const sunduq),
            );
        }

        let mut marsum = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: `marhala` is the staging texture created above with CPU read
        // access, which is the one usage `D3D11_MAP_READ` accepts. The
        // out-pointer addresses a live local.
        unsafe { self.siyaq.Map(&marhala, 0, D3D11_MAP_READ, 0, Some(&raw mut marsum)) }.map_err(
            |khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("the read-back texture could not be mapped: {khata}"),
            },
        )?;

        let natija = jami_sufuf(&marsum, mintaqa, sath.sigha);

        // SAFETY: the map above succeeded, so the subresource is mapped and must
        // be unmapped exactly once. This runs before the result is inspected so
        // that an error on the copy path does not leave it mapped.
        unsafe { self.siyaq.Unmap(&marhala, 0) };

        natija
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        // Idempotent by construction: every field is an `Option` and assigning
        // `None` twice is assigning `None`. The second call is the common one —
        // `Tabaqa::shaghghil` tears a backend down when `hayyi` failed, and then
        // the caller drops it, which runs this again.
        self.atlif_masar();
        self.lawha = None;
        self.ru2yat_lawha = None;
        self.bayt_ras = Vec::new();
        self.bayt_biksel = Vec::new();
        self.musawwada = Vec::new();
        Ok(())
    }
}

/// How many vertices the dynamic vertex buffer holds.
pub(crate) const RUUS_LIL_DUFA: u32 = 16_384;

// Four vertices per quad, and the vertex buffer must hold a whole chunk of
// them. A chunk size raised without this following it would map a buffer
// smaller than the copy that follows.
const _: () = {
    assert!(RUUS_LIL_DUFA as usize == QITA_LIL_DUFA * 4, "the vertex buffer is the wrong size");
};

/// Appends one quad's four vertices to the scratch buffer.
///
/// The winding is top-left, top-right, bottom-right, bottom-left, which the
/// index pattern built in [`KhattafD3D11::ibni_masar`] cuts into two triangles.
/// Both halves are here in one file for the same reason the two shader entry
/// points are in one string: a winding that disagrees with an index pattern
/// draws nothing and looks like a bind that did not take.
pub(crate) fn imla_qita(musawwada: &mut Vec<Ras>, qita: &QitaRasm) {
    let yasar = madaa_f32(qita.mawdi.yasar);
    let aala = madaa_f32(qita.mawdi.aala);
    let yameen = yasar + madaa_f32(qita.mawdi.ard);
    let asfal = aala + madaa_f32(qita.mawdi.irtifa);

    // A plate carries no atlas rectangle, and rather than branch in the shader
    // on a sentinel coordinate the weight is carried per vertex. Zero means the
    // sample is ignored, so the coordinates below are never read and are set to
    // the atlas origin only because a NaN in a vertex buffer is a real way to
    // lose a whole draw call on some drivers.
    let (u0, v0, u1, v1, khalt) = match qita.khareeta {
        Some(khareeta) => (
            khareeta.yasar,
            khareeta.aala,
            khareeta.yasar + khareeta.ard,
            khareeta.aala + khareeta.irtifa,
            1.0,
        ),
        None => (0.0, 0.0, 0.0, 0.0, 0.0),
    };

    musawwada.extend_from_slice(&[
        Ras { mawdi: [yasar, aala], khareeta: [u0, v0], lawn: qita.lawn, khalt },
        Ras { mawdi: [yameen, aala], khareeta: [u1, v0], lawn: qita.lawn, khalt },
        Ras { mawdi: [yameen, asfal], khareeta: [u1, v1], lawn: qita.lawn, khalt },
        Ras { mawdi: [yasar, asfal], khareeta: [u0, v1], lawn: qita.lawn, khalt },
    ]);
}

/// Copies a mapped region out row by row, dropping the pitch.
///
/// `RowPitch` is the distance between the starts of two rows in the mapping,
/// which the driver rounds up to whatever alignment it prefers — 256 bytes is
/// common and 512 happens. It is *not* the row's width, and a read that treats
/// it as one produces an image that shears progressively to one side, which
/// looks like a capture bug in the recognizer rather than a stride bug here.
/// [`Khattaf::iltaqit`] promises tightly packed bytes, so the repacking happens
/// once, here.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the mapping has no pointer or its pitch
/// is shorter than one row of the region, both of which mean the copy did not
/// produce what was asked for.
fn jami_sufuf(
    marsum: &D3D11_MAPPED_SUBRESOURCE,
    mintaqa: MustatilBiksel,
    sigha: SighatSath,
) -> Result<Vec<u8>, KhataTabaqa> {
    if marsum.pData.is_null() {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: "the read-back mapping has no pointer".to_owned(),
        });
    }

    let tul_saf = mintaqa.ard.saturating_mul(sigha.bayt_lil_biksel());
    let tul_saf = usize::try_from(tul_saf).unwrap_or(usize::MAX);
    let khatwa = usize::try_from(marsum.RowPitch).unwrap_or(usize::MAX);
    if khatwa < tul_saf || tul_saf == 0 {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: format!(
                "the read-back mapping reports a {khatwa}-byte pitch for a {tul_saf}-byte row"
            ),
        });
    }

    let irtifa = usize::try_from(mintaqa.irtifa).unwrap_or(0);
    let mut khuruj = Vec::with_capacity(tul_saf.saturating_mul(irtifa));
    for saf in 0..irtifa {
        // SAFETY: the mapping covers `RowPitch * height` bytes for the staging
        // texture that was created at exactly this region's dimensions, so a row
        // start at `saf * khatwa` with `saf` below the height is inside it, and
        // `tul_saf` bytes from there are inside that row because `khatwa` was
        // checked to be at least `tul_saf`. The mapping outlives this function,
        // which runs before `Unmap`.
        let bayt = unsafe {
            let asas = marsum.pData.cast::<u8>().add(saf.saturating_mul(khatwa));
            core::slice::from_raw_parts(asas, tul_saf)
        };
        khuruj.extend_from_slice(bayt);
    }
    Ok(khuruj)
}

/// A surface dimension as a float, without a lossy-looking cast per call site.
///
/// The same conversion `crate::wajiha` makes, made again here because that one
/// is private to its module and a `pub` version would be a public API whose only
/// purpose is to satisfy a lint.
pub(crate) const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and atlas dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// Reduces a DXGI format to what the capture path needs, and whether the
/// hardware will encode for us.
///
/// Shared with [`crate::d3d12`], which faces the same swap chain formats through
/// a different API. Six formats are recognised because six are what a DXGI swap
/// chain is permitted to present; everything else is a
/// [`KhataTabaqa::SighaGhayrMaduma`] naming the number, because a format nobody
/// has seen is more useful in a bug report as a number than as "unsupported".
///
/// # Errors
///
/// [`KhataTabaqa::SighaGhayrMaduma`] for any format outside the six.
pub(crate) fn sigha_min_dxgi(sigha: DXGI_FORMAT) -> Result<(SighatSath, bool), KhataTabaqa> {
    match sigha {
        DXGI_FORMAT_B8G8R8A8_UNORM => Ok((SighatSath::Bgra8, false)),
        DXGI_FORMAT_B8G8R8A8_UNORM_SRGB => Ok((SighatSath::Bgra8, true)),
        DXGI_FORMAT_R8G8B8A8_UNORM => Ok((SighatSath::Rgba8, false)),
        DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => Ok((SighatSath::Rgba8, true)),
        // Ten-bit has no `_SRGB` spelling; a swap chain presenting it in the
        // default colour space holds sRGB-encoded values, so the shader encodes.
        // A game that has switched the swap chain to HDR10 is presenting PQ
        // through this same format and the overlay's text will be too dark
        // against it — that is a real limitation of this build and it is stated
        // here rather than hidden behind a branch that pretends otherwise.
        DXGI_FORMAT_R10G10B10A2_UNORM => Ok((SighatSath::Rgb10a2, false)),
        // scRGB, which is linear by definition — so `sirgb` is false, meaning
        // "the values are not stored non-linearly", which is what
        // `WasfSath::sirgb` documents itself as. The overlay knows not to encode
        // into it through `SighatSath::wasi_al_mada` instead, which is the same
        // flag the capture path reads to know it must tone-map before
        // recognition. Two facts, two fields, neither standing in for the other.
        DXGI_FORMAT_R16G16B16A16_FLOAT => Ok((SighatSath::Rgba16f, false)),
        _ => Err(KhataTabaqa::SighaGhayrMaduma { sigha: format!("DXGI_FORMAT({})", sigha.0) }),
    }
}
