//! خطّاف دايركت٣د ٩ — the overlay drawn through a game's own Direct3D 9 device.
//!
//! The four backends that came before this one all speak to a programmable
//! pipeline. D3D9 does not, and almost nothing in this file is a translation of
//! [`crate::d3d11`] with different type names. What is shared is the batch, the
//! premultiplied blend and the [`Khattaf`] contract; everything else is written
//! for an API whose state is a hundred and fifty numbered render states, whose
//! device can stop existing when the player presses alt-tab, and whose games
//! shipped between roughly 2002 and 2012 — the densest era of story-driven PC
//! games, and until now the one Taarib could not draw over at all.
//!
//! ## `Present`, not `EndScene`
//!
//! Both are candidate hook points and the choice is not a preference.
//!
//! `EndScene` is called **once per completed scene**, not once per frame. A game
//! that renders shadow maps, a reflection pass and a post-processing chain calls
//! it three or four times before it presents, each time with a different surface
//! bound as the render target. An overlay on `EndScene` therefore draws several
//! times a frame, into whatever the game happened to be rendering into — a
//! shadow map, most often, where the Arabic is invisible and the cost is real.
//! The usual patch for that is to compare the current render target against the
//! backbuffer and skip when they differ, which is a heuristic that fails on the
//! games that render their UI into an offscreen target and composite it.
//!
//! `Present` is called exactly once per frame, and at the moment it is called
//! the backbuffer holds the finished picture. The one thing it costs is that the
//! scene is closed, so this backend opens its own with `BeginScene` and closes
//! it again — three extra calls a frame, in exchange for never guessing which
//! surface the frame is.
//!
//! `IDirect3DSwapChain9::Present` is **not** hooked, and that is a real
//! limitation rather than an oversight: a game that renders into an additional
//! swap chain of its own gets no overlay. Almost nothing does — additional swap
//! chains are a multi-monitor and editor-preview feature — and hooking a second
//! present path would mean two entry points racing to start one overlay.
//!
//! ## The state block, and why it is not an explicit save list
//!
//! [`crate::d3d11`] enumerates every bind it makes and writes all of it back.
//! That works because D3D11's pipeline is a fixed set of about twenty slots. The
//! equivalent list for D3D9 is the union of the render states, the texture stage
//! states of eight stages, the sampler states of sixteen samplers, the three
//! transform matrices the fixed-function pipeline reads, the material, the
//! lights, the clip planes, the FVF or vertex declaration, the stream sources,
//! the indices and both shader stages with their constant files. Written out by
//! hand it is several hundred `Get`/`Set` pairs, and a single one omitted is a
//! game that renders wrongly from the first overlay frame with nothing to
//! attribute it to.
//!
//! D3D9 provides the mechanism itself. [`D3DSBT_ALL`] is a device-captured
//! snapshot of exactly that union, `Capture` refreshes it and `Apply` writes it
//! back, and the runtime — not this file — is the authority on what the union
//! contains. It is used here for three reasons and the third is the decisive
//! one:
//!
//! 1. it cannot go stale when a driver or a runtime update adds state;
//! 2. `Apply` returns an `HRESULT`, so a failed restore is *detectable*, which
//!    is more than [`crate::d3d11`] gets — there the only observable restore
//!    failure is the device having been removed underneath it;
//! 3. **a pure device has no `Get` calls.** `D3DCREATE_PUREDEVICE` is documented
//!    as not supporting `Get*` for anything a state block can hold, and it is
//!    what a game asks for when it wants the driver to stop tracking state on
//!    its behalf. An explicit save list would silently read nothing on those
//!    games and write nothing back. State blocks are the one save-and-restore
//!    that works on a pure device, which is why the answer here is not "either
//!    is fine".
//!
//! Two things a `D3DSBT_ALL` block does **not** carry, and both are restored by
//! hand below: the render target and the depth-stencil surface. They are
//! resource bindings rather than state, and an overlay that assumed the block
//! covered them would leave the game rendering into the backbuffer for the rest
//! of the session.
//!
//! ## Device loss is the thing that makes this API different
//!
//! A D3D9 device can be **lost**. Alt-tab out of exclusive fullscreen, a
//! resolution change, a power event, a driver reset — and every method on the
//! device starts returning `D3DERR_DEVICELOST`. Nothing can be drawn until the
//! game calls `Reset`, and `Reset` fails while *anybody* holds a
//! `D3DPOOL_DEFAULT` resource, an explicit render target, an additional swap
//! chain, **or a state block**.
//!
//! That last one is what an overlay gets wrong. The state block this backend
//! creates once and reuses every frame is exactly the object that makes the
//! game's own `Reset` fail with `D3DERR_INVALIDCALL`, on a call that has never
//! failed the game before, in a code path the game very reasonably treats as
//! fatal. So `Reset` is hooked as well as `Present`, and the hook reaches
//! [`Khattaf::atliq_sath`] before the call goes to the runtime — the same trait
//! method DXGI's `ResizeBuffers` uses, for the same structural reason.
//!
//! Everything else this backend allocates is chosen so that there is nothing
//! *else* to release:
//!
//! * **no vertex or index buffer.** Geometry reaches the GPU through
//!   `DrawIndexedPrimitiveUP`, which takes user memory. A `D3DPOOL_DEFAULT`
//!   dynamic vertex buffer would be faster per draw and would have to be
//!   released and recreated around every device loss; at the batch sizes a
//!   dialogue box produces — a few thousand vertices — the runtime's copy is not
//!   measurable next to a 2002-era game's own frame, and the entire class of
//!   "the overlay held a buffer through a `Reset`" bugs does not exist.
//! * **no held backbuffer.** The render target is fetched inside the draw and
//!   dropped before it returns, so there is no reference to release when the
//!   game resizes.
//! * **the atlas is `D3DPOOL_MANAGED`**, which the runtime backs with a system
//!   memory copy and restores across `Reset` by itself. That is the whole
//!   reason managed exists and it is the one pool that survives a lost device.
//!
//! The exception is `IDirect3DDevice9Ex`, where `D3DPOOL_MANAGED` is rejected
//! outright. There the atlas is a dynamic `D3DPOOL_DEFAULT` texture, it does not
//! survive `ResetEx`, and this backend keeps the uploaded bytes so it can put
//! them back. The asymmetry is deliberate and it is why [`KhattafD3D9`] holds a
//! copy of the atlas on one path and not on the other: paying a megabyte of a
//! game's memory on every device would be paying it for a case that only 9Ex
//! has.
//!
//! ## The fixed-function pipeline, and where sRGB is done instead
//!
//! There is no pixel shader here. A `ps_2_0` overlay would refuse to draw on a
//! device created with `D3DCREATE_SOFTWARE_VERTEXPROCESSING` against hardware
//! that predates it, and those are exactly the games this backend exists for.
//! So the draw is the fixed-function texture cascade: stage zero modulates the
//! atlas sample by the vertex colour, stage one is disabled, and the vertex
//! format is `D3DFVF_XYZRHW`, which bypasses the transform pipeline entirely so
//! that the game's own world, view and projection matrices are never read and
//! never depended on.
//!
//! That leaves the transfer function with nowhere to live. [`crate::d3d11`]'s
//! pixel shader encodes linear colour to sRGB when the target format says nobody
//! else will, and a D3D9 backbuffer *always* says that: there is no `_SRGB`
//! swap-chain format in this API, and `D3DRS_SRGBWRITEENABLE` only converts
//! after blending on hardware that reports `D3DPMISCCAPS_POSTBLENDSRGBCONVERT`,
//! which is not a bet worth making on a driver from this era.
//!
//! The encode therefore happens on the CPU, in [`lawn_d3d`], and it is *exact*
//! rather than an approximation of the shader. A quad's colour is constant
//! across the quad, so encoding it per vertex produces the same pixels the
//! shader would: unpremultiply, apply the real piecewise sRGB curve,
//! re-premultiply, pack. The atlas sample stays a linear coverage multiplier,
//! which is what the game's own antialiased text is blended with, so Taarib's
//! Arabic and the game's English are composited in the same space.
//!
//! ## Sixteen-bit backbuffers
//!
//! `D3DFMT_R5G6B5` and `D3DFMT_X1R5G5B5` are real swap-chain formats that real
//! games of this era present. [`crate::wajiha::SighatSath`] has no member for
//! them, deliberately — it is the reduced set the recognizer's converter
//! understands, and a fifth member would be a fifth branch in a conversion that
//! is written once for four backends.
//!
//! So this backend widens instead: [`KhattafD3D9::sath`] reports
//! [`SighatSath::Bgra8`] for a sixteen-bit surface and [`KhattafD3D9::iltaqit`]
//! expands five and six bit channels to eight during the read-back, with the
//! low bits replicated so that full-scale stays full-scale. Nothing is lost —
//! the widening is exact in the direction it goes — and the alternative would
//! have been to refuse the capture, which on this API means refusing the tier.
//!
//! ## The proxy already in the directory
//!
//! Games of this era attract third-party proxy DLLs — a `dinput8.dll` that is
//! really a mod loader, a `d3d9.dll` that is really a shader injector — and one
//! of them being present is the normal case rather than the exceptional one.
//! Two rules follow, and both are enforced rather than documented:
//!
//! * Taarib does not take a proxy slot another product has taken. That is
//!   [`crate::istitlaa`], and it is answered **before** anything is installed.
//! * Taarib does not restore a vtable pointer it did not install. That is
//!   [`crate::khataf::Khataf::fukk`], which reads the slot back and leaves it
//!   alone when it holds somebody else's thunk. Chaining onto an existing hook
//!   is fine and is what happens; unhooking out of order is what corrupts a
//!   process, and it is refused.

use core::ffi::c_void;
use std::mem;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Direct3D9::{
    D3DBACKBUFFER_TYPE_MONO, D3DBLEND_INVSRCALPHA, D3DBLEND_ONE, D3DBLENDOP_ADD, D3DCAPS9,
    D3DCREATE_PUREDEVICE, D3DCULL_NONE, D3DDEVICE_CREATION_PARAMETERS, D3DFILL_SOLID,
    D3DFMT_A2R10G10B10, D3DFMT_A8R8G8B8, D3DFMT_A16B16G16R16F, D3DFMT_INDEX16, D3DFMT_R5G6B5,
    D3DFMT_X1R5G5B5, D3DFMT_X8R8G8B8, D3DFORMAT, D3DFVF_DIFFUSE, D3DFVF_TEX1, D3DFVF_XYZRHW,
    D3DLOCK_DISCARD, D3DLOCK_READONLY, D3DLOCKED_RECT, D3DMULTISAMPLE_NONE, D3DPOOL,
    D3DPOOL_DEFAULT, D3DPOOL_MANAGED, D3DPOOL_SYSTEMMEM, D3DPRESENT_PARAMETERS, D3DPT_TRIANGLELIST,
    D3DRENDERSTATETYPE, D3DRS_ALPHABLENDENABLE, D3DRS_ALPHATESTENABLE, D3DRS_ANTIALIASEDLINEENABLE,
    D3DRS_BLENDOP, D3DRS_CLIPPING, D3DRS_CLIPPLANEENABLE, D3DRS_COLORWRITEENABLE, D3DRS_CULLMODE,
    D3DRS_DESTBLEND, D3DRS_FILLMODE, D3DRS_FOGENABLE, D3DRS_INDEXEDVERTEXBLENDENABLE,
    D3DRS_LIGHTING, D3DRS_SCISSORTESTENABLE, D3DRS_SEPARATEALPHABLENDENABLE, D3DRS_SHADEMODE,
    D3DRS_SRCBLEND, D3DRS_SRGBWRITEENABLE, D3DRS_STENCILENABLE, D3DRS_VERTEXBLEND, D3DRS_ZENABLE,
    D3DRS_ZWRITEENABLE, D3DSAMP_ADDRESSU, D3DSAMP_ADDRESSV, D3DSAMP_MAGFILTER, D3DSAMP_MINFILTER,
    D3DSAMP_MIPFILTER, D3DSAMP_SRGBTEXTURE, D3DSAMPLERSTATETYPE, D3DSBT_ALL, D3DSHADE_GOURAUD,
    D3DSURFACE_DESC, D3DTA_DIFFUSE, D3DTA_TEXTURE, D3DTADDRESS_CLAMP, D3DTEXF_LINEAR, D3DTEXF_NONE,
    D3DTOP_DISABLE, D3DTOP_MODULATE, D3DTOP_SELECTARG1, D3DTSS_ALPHAARG1, D3DTSS_ALPHAARG2,
    D3DTSS_ALPHAOP, D3DTSS_COLORARG1, D3DTSS_COLORARG2, D3DTSS_COLOROP, D3DTSS_TEXCOORDINDEX,
    D3DTSS_TEXTURETRANSFORMFLAGS, D3DTTFF_DISABLE, D3DUSAGE_DYNAMIC, D3DVBF_DISABLE, D3DVIEWPORT9,
    D3DZB_FALSE, IDirect3DBaseTexture9, IDirect3DDevice9, IDirect3DDevice9Ex,
    IDirect3DPixelShader9, IDirect3DStateBlock9, IDirect3DSurface9, IDirect3DTexture9,
    IDirect3DVertexShader9,
};
use windows::core::{HRESULT, Interface};

use crate::khata::{KhataTabaqa, tul_u64};
use crate::qudra::{MilShasha, QudratTarkeeb, SababQudra};
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, QitaRasm, SighatSath, WajihatRusum, WasfSath,
};

// ---------------------------------------------------------------------------
// The error codes the headers define and the bindings do not
// ---------------------------------------------------------------------------

/// `_FACD3D`, shifted into place with the severity bit, as `d3d9.h` builds it.
///
/// `MAKE_D3DHRESULT(code)` is `MAKE_HRESULT(1, 0x876, code)`, which is
/// `0x8000_0000 | (0x876 << 16) | code`. The `windows` crate binds D3D9's types
/// and functions and none of its `HRESULT` constants, so the six this backend
/// has to distinguish are derived here rather than written as magic numbers —
/// the derivation is checkable against the header and the numbers are not.
const ASAS_KHATA: i32 = 0x8876_0000_u32.cast_signed();

/// `MAKE_D3DHRESULT`, as a `const fn`.
const fn khata_d3d(ramz: u32) -> HRESULT {
    HRESULT(ASAS_KHATA | ramz.cast_signed())
}

/// `D3DERR_DEVICELOST` — the device cannot draw and cannot yet be reset.
const KHATA_JIHAZ_MAFQUD: HRESULT = khata_d3d(2152);

/// `D3DERR_DEVICENOTRESET` — the device is ready for the game to reset it.
const KHATA_JIHAZ_LAM_YUAAD: HRESULT = khata_d3d(2153);

/// `D3DERR_DEVICEREMOVED` — a 9Ex device whose adapter is gone.
const KHATA_JIHAZ_MUZAL: HRESULT = khata_d3d(2160);

/// `D3DERR_DEVICEHUNG` — a 9Ex device the driver reset out from under the game.
const KHATA_JIHAZ_MUAALLAQ: HRESULT = khata_d3d(2164);

/// `D3DERR_DRIVERINTERNALERROR` — nothing recovers from this one.
const KHATA_DAAKHILI: HRESULT = khata_d3d(2087);

/// `D3DERR_NOTFOUND` — what `GetDepthStencilSurface` answers when there is none.
const KHATA_GHAYR_MAWJUD: HRESULT = khata_d3d(2150);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The atlas dimension this build refuses outright, before asking the device.
///
/// Every Direct3D 9 device at shader model 2 or above reports at least 2048 and
/// in practice 4096, and the real ceiling is read from [`D3DCAPS9`] per device
/// so a card that allows more is allowed to. This is the floor below which the
/// caps read is not even attempted, because a request past it is a bug in the
/// feeder rather than a hardware limit.
pub const SAQF_LAWHA_MUTLAQ: u32 = 16_384;

/// How many quads one `DrawIndexedPrimitiveUP` call covers.
///
/// The same number [`crate::d3d11`] uses, and for the same arithmetic: four
/// thousand and ninety-six quads is sixteen thousand three hundred and
/// eighty-four vertices, which is the largest run a `D3DFMT_INDEX16` index
/// array can address. Spelled again rather than imported because the D3D11
/// constant is sized against a vertex *buffer* this backend does not have, and
/// a shared name would imply the two are one decision.
const QITA_LIL_DUFA: usize = 4_096;

/// The vertex format the input assembler is told about.
///
/// `XYZRHW` is the whole reason the fixed-function transform stages are never
/// touched: a vertex declared with it is already in screen space, so the world,
/// view and projection matrices the game has loaded are not read, not saved and
/// not restored.
const SIGHAT_RAS: u32 = D3DFVF_XYZRHW | D3DFVF_DIFFUSE | D3DFVF_TEX1;

/// Where the atlas coordinates start, in bytes, inside [`RasD3D9`].
const IZAHAT_KHAREETA: usize = 20;

/// How many bytes one vertex occupies.
const KHATWAT_RAS: u32 = 28;

/// All four colour channels, as `D3DRS_COLORWRITEENABLE` spells them.
///
/// `D3DCOLORWRITEENABLE_RED | GREEN | BLUE | ALPHA`, which the header defines as
/// bits zero through three.
const KITABAT_ALWAN: u32 = 0x0F;

// A field reordered without these offsets following it is a vertex layout the
// fixed-function pipeline reads the wrong bytes through: the text would still
// draw, in the wrong place, in the wrong colour, with no error anywhere. The
// FVF fixes the order — position, then diffuse, then one texture coordinate —
// so the agreement is checked at compile time rather than trusted.
const _: () = {
    assert!(
        size_of::<RasD3D9>() == KHATWAT_RAS as usize,
        "RasD3D9 is not 28 bytes"
    );
    assert!(mem::offset_of!(RasD3D9, mawdi) == 0, "RasD3D9.mawdi moved");
    assert!(mem::offset_of!(RasD3D9, lawn) == 16, "RasD3D9.lawn moved");
    assert!(
        mem::offset_of!(RasD3D9, khareeta) == IZAHAT_KHAREETA,
        "RasD3D9.khareeta moved"
    );
};

/// One corner of one quad, in the layout `SIGHAT_RAS` describes.
///
/// The field order is not a choice. A flexible vertex format has a fixed
/// ordering — position, then the blending and normal fields nothing here uses,
/// then diffuse, then the texture coordinates — and a struct written in any
/// other order is read as though it were written in this one.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RasD3D9 {
    /// Screen position and the reciprocal homogeneous w, which is always one.
    mawdi: [f32; 4],
    /// Premultiplied, sRGB-encoded `D3DCOLOR`: `0xAARRGGBB`.
    lawn: u32,
    /// Atlas coordinates, zero to one.
    khareeta: [f32; 2],
}

// ---------------------------------------------------------------------------
// Colour
// ---------------------------------------------------------------------------

/// One linear channel encoded to sRGB.
///
/// The real piecewise transfer function, matching [`crate::d3d11`]'s `ishfir`
/// and the GL fragment stage's `ila_sirgb` term for term. A 2.2 power would
/// disagree with both in exactly the near-black range antialiased glyph edges
/// live in, which is where a wrong curve is visible and nowhere else.
fn ila_sirgb(khatti: f32) -> f32 {
    let amin = khatti.max(0.0);
    if amin <= 0.003_130_8 {
        amin * 12.92
    } else {
        1.055_f32.mul_add(amin.powf(1.0 / 2.4), -0.055)
    }
}

/// A premultiplied linear RGBA quad colour as a premultiplied sRGB `D3DCOLOR`.
///
/// This is where [`crate::d3d11`]'s pixel shader went. That shader receives a
/// premultiplied linear colour, multiplies it by the atlas sample, divides the
/// result by its own alpha to recover the unpremultiplied colour, encodes, and
/// multiplies the alpha back in. Every step of that except the atlas multiply is
/// constant across a quad, so doing it once per vertex on the processor gives
/// the same pixels the shader gives — exactly, not approximately — and leaves
/// the fixed-function cascade with nothing to do but modulate.
///
/// The atlas sample stays linear coverage on purpose. It is a weight, not a
/// colour, and it is the same weight the game's own antialiased text is blended
/// with, so the two composite in one space.
///
/// Public because it is the one piece of this backend whose correctness can be
/// checked without a graphics device, and checking it is worth more than the
/// symmetry of keeping every helper private: if this is right, the colour the
/// fixed-function cascade is handed is right, and everything left is the driver
/// multiplying two numbers. `examples/d3d9_burhan.rs` draws a whole frame
/// through it offscreen for exactly that reason.
#[must_use]
pub fn lawn_d3d(lawn: [f32; 4]) -> u32 {
    let alfa = lawn.get(3).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    // A fully transparent quad has no colour to recover and no pixels to draw.
    // Dividing by its alpha would produce infinities in a vertex buffer, which
    // some drivers answer by dropping the whole draw call rather than the quad.
    let qisma = if alfa > 0.000_01 { alfa } else { 1.0 };

    let qanat = |fahras: usize| -> u32 {
        let khatti = (lawn.get(fahras).copied().unwrap_or(0.0) / qisma).clamp(0.0, 1.0);
        thaman_bit(ila_sirgb(khatti) * alfa)
    };

    // `0xAARRGGBB`, which is what `D3DCOLOR` is on every target this builds for.
    (thaman_bit(alfa) << 24) | (qanat(0) << 16) | (qanat(1) << 8) | qanat(2)
}

/// A zero-to-one channel as the eight bits a `D3DCOLOR` carries.
fn thaman_bit(qeema: f32) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped into [0, 255] on the line before the conversion, so the value is a \
                  non-negative integer inside u32's range"
    )]
    {
        (qeema * 255.0).round().clamp(0.0, 255.0) as u32
    }
}

/// A surface dimension as a float, without a lossy-looking cast per call site.
const fn madaa_f32(qeema: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "surface and atlas dimensions are below 2^24, where u32 to f32 is exact"
    )]
    {
        qeema as f32
    }
}

/// Appends one quad's four vertices to the scratch buffer.
///
/// The winding is top-left, top-right, bottom-right, bottom-left, matching the
/// index pattern [`faharis_thabita`] builds, and matching
/// [`crate::d3d11::imla_qita`] so that the two backends cannot disagree about
/// which corner is which.
///
/// The half-pixel subtraction is the one thing here that is D3D9's alone. A
/// vertex declared `D3DFVF_XYZRHW` addresses the *corner* of the pixel grid
/// while the rasterizer samples at pixel *centres*, so a quad placed at integer
/// coordinates covers each pixel by half and the whole overlay comes out soft
/// and shifted down and right by half a pixel. Subtracting a half puts the
/// glyph's texels back on the screen's texels, which for text at interface
/// sizes is the difference between crisp and blurred.
fn imla_qita_d3d9(musawwada: &mut Vec<RasD3D9>, qita: &QitaRasm) {
    let yasar = madaa_f32(qita.mawdi.yasar) - 0.5;
    let aala = madaa_f32(qita.mawdi.aala) - 0.5;
    let yameen = yasar + madaa_f32(qita.mawdi.ard);
    let asfal = aala + madaa_f32(qita.mawdi.irtifa);
    let lawn = lawn_d3d(qita.lawn);

    // A plate carries no atlas rectangle. Its texture coordinates are never
    // sampled — the stage cascade for a plate run selects the diffuse colour
    // and reads no texture at all — and they are set to the atlas origin rather
    // than left uninitialised because a NaN in a vertex stream is a real way to
    // lose a whole draw call on some drivers of this era.
    let (u0, v0, u1, v1) = qita.khareeta.map_or((0.0, 0.0, 0.0, 0.0), |khareeta| {
        (
            khareeta.yasar,
            khareeta.aala,
            khareeta.yasar + khareeta.ard,
            khareeta.aala + khareeta.irtifa,
        )
    });

    musawwada.extend_from_slice(&[
        RasD3D9 {
            mawdi: [yasar, aala, 0.0, 1.0],
            lawn,
            khareeta: [u0, v0],
        },
        RasD3D9 {
            mawdi: [yameen, aala, 0.0, 1.0],
            lawn,
            khareeta: [u1, v0],
        },
        RasD3D9 {
            mawdi: [yameen, asfal, 0.0, 1.0],
            lawn,
            khareeta: [u1, v1],
        },
        RasD3D9 {
            mawdi: [yasar, asfal, 0.0, 1.0],
            lawn,
            khareeta: [u0, v1],
        },
    ]);
}

/// The six-index pattern for a run of quads, built once.
///
/// `DrawIndexedPrimitiveUP` takes the indices from user memory on every call, so
/// there is no index buffer to create and none to release around a device loss —
/// but the array itself never changes, so it is built at initialization rather
/// than rebuilt per frame beside the vertices.
fn faharis_thabita() -> Vec<u16> {
    let mut faharis: Vec<u16> = Vec::with_capacity(QITA_LIL_DUFA.saturating_mul(6));
    for qita in 0..QITA_LIL_DUFA {
        let asas = u16::try_from(qita.saturating_mul(4)).unwrap_or(0);
        faharis.extend_from_slice(&[
            asas,
            asas.saturating_add(1),
            asas.saturating_add(2),
            asas,
            asas.saturating_add(2),
            asas.saturating_add(3),
        ]);
    }
    faharis
}

// ---------------------------------------------------------------------------
// The backend
// ---------------------------------------------------------------------------

/// The Direct3D 9 overlay backend.
///
/// Holds the game's device and the two objects the overlay cannot draw without:
/// a full-device state block and the glyph atlas. Nothing else is retained
/// between frames, which is what makes device loss survivable — see this
/// module's header.
#[derive(Debug)]
pub struct KhattafD3D9 {
    jihaz: IDirect3DDevice9,
    /// The same device under its 9Ex face, when the game created one.
    ///
    /// Held rather than re-queried because it decides two things on every
    /// frame: which cooperative-level call to make, and which pool the atlas
    /// belongs in.
    jihaz_ex: Option<IDirect3DDevice9Ex>,
    /// The focus window the device was created against.
    ///
    /// `IDirect3DDevice9Ex::CheckDeviceState` takes one, and the answer differs
    /// per window: a 9Ex device is "occluded" with respect to a window another
    /// window covers, which is a real state and not an error.
    nafidha: HWND,
    /// Whether the device was created with `D3DCREATE_PUREDEVICE`.
    naqi: bool,
    /// The largest atlas dimension this device accepts.
    saqf_lawha: u32,
    /// The full-device snapshot, captured before every draw and applied after.
    hala: Option<IDirect3DStateBlock9>,
    /// The glyph atlas.
    lawha: Option<IDirect3DTexture9>,
    /// The last atlas upload, kept only when the pool will not survive a reset.
    ///
    /// [`None`] on every plain D3D9 device, where the atlas is
    /// `D3DPOOL_MANAGED` and the runtime restores it without being asked.
    bayt_lawha: Option<(Vec<u8>, u32, u32)>,
    sath: Option<WasfSath>,
    /// The scratch vertex array, grown once and reused.
    musawwada: Vec<RasD3D9>,
    /// The index pattern, built once.
    faharis: Vec<u16>,
}

#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "the COM interfaces and the window handle are exactly the fields the assertion \
              below is about, and it states what is being asserted and what is not"
)]
// SAFETY: every field is a COM interface pointer, a window handle, an integer
// or an owned allocation, and this type dereferences none of them off the
// thread that calls into it. That is a statement about memory and deliberately
// not about the API: a Direct3D 9 device created without
// `D3DCREATE_MULTITHREADED` may be used from one thread only, so moving this
// value to another thread and drawing there would be undefined at the runtime's
// level even though it is sound at Rust's. Nothing does — the hook reaches this
// only from inside `Present`, which the game calls on its render thread — and
// the reason the assertion is needed at all is that `Khattaf` is `Send` so that
// `Tabaqa` can be owned by a `Mutex` the bootstrap holds across threads.
unsafe impl Send for KhattafD3D9 {}

impl KhattafD3D9 {
    /// Builds a backend around a device the hook already found.
    ///
    /// Everything is read from the device rather than created: the overlay never
    /// makes a second D3D9 device, because a second device would mean rendering
    /// into a texture that has to be composited back — the design this crate
    /// exists to avoid.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the device will not report its own
    /// creation parameters or its capabilities, and
    /// [`KhataTabaqa::SathTaghayyar`] when it will not hand over a backbuffer,
    /// which is what a device that is already lost looks like.
    pub fn min_jihaz(jihaz: &IDirect3DDevice9) -> Result<Self, KhataTabaqa> {
        let mut muallimat = D3DDEVICE_CREATION_PARAMETERS::default();
        // SAFETY: `jihaz` is a live device the caller obtained from the process
        // it is hooking, and the out-pointer addresses a fully initialised local
        // of exactly the type the runtime writes.
        unsafe { jihaz.GetCreationParameters(&raw mut muallimat) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "the device's creation parameters",
                sabab: khata.to_string(),
            }
        })?;

        let mut qudurat = D3DCAPS9::default();
        // SAFETY: as above, for the capability structure.
        unsafe { jihaz.GetDeviceCaps(&raw mut qudurat) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "the device's capabilities",
                sabab: khata.to_string(),
            }
        })?;

        // SAFETY: `jihaz` is live. `GetBackBuffer` performs a QueryInterface on
        // the swap chain's buffer and returns an owned reference or an error;
        // the reference is dropped at the end of this statement because the
        // backend never holds one between frames.
        drop(
            // SAFETY: as above.
            unsafe { jihaz.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }.map_err(|khata| {
                KhataTabaqa::SathTaghayyar {
                    sabab: format!("the device would not hand over its backbuffer: {khata}"),
                }
            })?,
        );

        // Both dimensions matter and the smaller one is the real ceiling: an
        // atlas is square in this product, and a device that allows 4096 across
        // and 2048 down would accept a page neither dimension check caught.
        let saqf = qudurat
            .MaxTextureWidth
            .min(qudurat.MaxTextureHeight)
            .min(SAQF_LAWHA_MUTLAQ);

        Ok(Self {
            jihaz: jihaz.clone(),
            jihaz_ex: jihaz.cast::<IDirect3DDevice9Ex>().ok(),
            nafidha: muallimat.hFocusWindow,
            naqi: muallimat.BehaviorFlags & D3DCREATE_PUREDEVICE.cast_unsigned() != 0,
            saqf_lawha: saqf,
            hala: None,
            lawha: None,
            bayt_lawha: None,
            sath: None,
            musawwada: Vec::new(),
            faharis: Vec::new(),
        })
    }

    /// What this device can and cannot do, before a frame is drawn.
    ///
    /// Produced the moment a backend is built and carried into the session log.
    /// Every finding in it is one that would otherwise be discovered as a frame
    /// that silently did not draw, and "the overlay is on and nothing appears"
    /// is the one failure this tier must never produce without an explanation.
    ///
    /// [`crate::istitlaa`] answers the same shape of question *before* anything
    /// is installed, from the game's own files. This one answers the half that
    /// only a live device knows: which of the two device kinds the game made,
    /// whether it is presenting exclusively, and whether its backbuffer can be
    /// read at all.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SathTaghayyar`] when the device will not describe its
    /// backbuffer or its swap chain, which is what a lost device answers. The
    /// caller retries on the next frame.
    pub fn qudra(&self) -> Result<QudratTarkeeb, KhataTabaqa> {
        // SAFETY: `jihaz` is the live device this backend was built around.
        // `GetSwapChain` returns an owned reference to the implicit swap chain.
        let silsila =
            unsafe { self.jihaz.GetSwapChain(0) }.map_err(|khata| KhataTabaqa::SathTaghayyar {
                sabab: format!("the device would not hand over its swap chain: {khata}"),
            })?;

        let mut muallimat = D3DPRESENT_PARAMETERS::default();
        // SAFETY: `silsila` is the live swap chain from the line above, and the
        // out-pointer addresses a fully initialised local of the runtime's type.
        unsafe { silsila.GetPresentParameters(&raw mut muallimat) }.map_err(|khata| {
            KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain would not describe itself: {khata}"),
            }
        })?;

        let wasf = self.wasf_khalfiya()?;
        let hasri = !muallimat.Windowed.as_bool();

        // Composition is through the device on this API as on every other in
        // this crate: the overlay draws into the frame the game is about to
        // present, so exclusive fullscreen is composed over exactly as windowed
        // mode is.
        let mut taqrir = QudratTarkeeb::jadeeda(WajihatRusum::Direct3D9, MilShasha::KhilalAlJihaz);

        // Direct3D 9 lets a game create additional swap chains and present
        // through `IDirect3DSwapChain9::Present` rather than the device's own.
        // The device hook never fires for those frames. Having more than the
        // implicit swap chain is the only evidence of it available, and it is
        // evidence rather than proof — most games that make a second one still
        // present through the device — so this narrows the report rather than
        // stopping the overlay.
        //
        // SAFETY: `jihaz` is the live device; this call takes nothing and
        // returns a count.
        let silasil = unsafe { self.jihaz.GetNumberOfSwapChains() };
        if silasil > 1 {
            taqrir = taqrir.maa(SababQudra::naqisa(
                format!(
                    "أنشأت اللعبة {silasil} سلاسل عرض. إن عرضت عبر سلسلة خاصة بها لا عبر                      الجهاز، فلن يصل خطّاف الطبقة إلى تلك الإطارات."
                ),
                format!(
                    "the game has created {silasil} swap chains. Frames it presents through a                      swap chain of its own rather than through the device do not reach the                      overlay's hook, and would carry no Arabic."
                ),
            ));
        }

        taqrir = taqrir.maa(SababQudra::kamila(
            format!(
                "الجهاز {} ويعرض في وضع {}.",
                if self.jihaz_ex.is_some() {
                    "ممتد (9Ex)"
                } else {
                    "عادي"
                },
                if hasri {
                    "ملء الشاشة الحصري"
                } else {
                    "النافذة"
                }
            ),
            format!(
                "the game made an {} device and is presenting {}",
                if self.jihaz_ex.is_some() {
                    "IDirect3DDevice9Ex"
                } else {
                    "IDirect3DDevice9"
                },
                if hasri {
                    "in exclusive fullscreen"
                } else {
                    "windowed"
                }
            ),
        ));

        if self.naqi {
            taqrir = taqrir.maa(SababQudra::kamila(
                "أُنشئ الجهاز بوضع D3DCREATE_PUREDEVICE، ولا يجيب على استعلامات الحالة. تحفظ                  الطبقة حالة اللعبة وتعيدها عبر كتلة حالة، وهي الطريقة الوحيدة التي تعمل هنا.",
                "the device was created with D3DCREATE_PUREDEVICE and answers no Get* call for                  state a state block can hold. The overlay saves and restores through a state                  block, which is the one mechanism that works on such a device.",
            ));
        }

        if hasri && self.jihaz_ex.is_none() {
            taqrir = taqrir.maa(SababQudra::naqisa(
                "هذا جهاز Direct3D 9 عادي في وضع ملء الشاشة الحصري، وسيُفقد عند كل تبديل نافذة.                  لا تُرسم الطبقة — ولا اللعبة نفسها — حتى تعيد اللعبة تهيئة الجهاز.",
                "this is a plain Direct3D 9 device in exclusive fullscreen, so it is lost on                  every alt-tab. Nothing draws until the game resets it — not the overlay and                  not the game — and the overlay cannot reset it, because the device is the                  game's.",
            ));
        }

        if wasf.MultiSampleType != D3DMULTISAMPLE_NONE {
            taqrir = taqrir.maa(SababQudra::kamila(
                "الصورة المعروضة متعددة العينات، فتمر كل قراءة للشاشة بسطح وسيط لحلّها أولًا.                  الرسم غير متأثر.",
                "the backbuffer is multisampled, so every screen read resolves through an                  intermediate surface first. Drawing is unaffected.",
            ));
        }

        match sigha_min_d3d9(wasf.Format) {
            Ok(_) if matches!(wasf.Format, D3DFMT_R5G6B5 | D3DFMT_X1R5G5B5) => {
                taqrir = taqrir.maa(SababQudra::kamila(
                    "الصورة بست عشرة بتة لكل بكسل، وتُوسَّع إلى ثماني بتات لكل قناة أثناء                      القراءة دون فقد.",
                    "the backbuffer is sixteen bits per pixel and is widened to eight bits per                      channel during read-back, losslessly.",
                ));
            },
            Ok(_) => {},
            Err(_) => {
                taqrir = taqrir.maa(SababQudra::naqisa(
                    format!(
                        "تعذّرت قراءة صيغة الصورة D3DFORMAT({}) على هذا الجهاز، فلا يوجد نص                          تقرؤه الطبقة وتترجمه.",
                        wasf.Format.0
                    ),
                    format!(
                        "D3DFORMAT({}) cannot be read back by this build, so there is nothing                          for the recognizer to read: Arabic can be drawn over this game and                          nothing can be translated on it.",
                        wasf.Format.0
                    ),
                ));
            },
        }

        Ok(taqrir)
    }

    /// The backbuffer's own description.
    ///
    /// Read every time rather than cached: it is the authority on the surface's
    /// size and format, and a cached copy is how an overlay draws a batch built
    /// for the previous resolution for one frame after a reset.
    fn wasf_khalfiya(&self) -> Result<D3DSURFACE_DESC, KhataTabaqa> {
        // SAFETY: `jihaz` is the live device this backend was built around, and
        // `GetBackBuffer` returns an owned reference dropped at the end of this
        // function.
        let khalfiya = unsafe { self.jihaz.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }.map_err(
            |khata| KhataTabaqa::SathTaghayyar {
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;

        let mut wasf = D3DSURFACE_DESC::default();
        // SAFETY: `khalfiya` is the live surface from above and the out-pointer
        // addresses a fully initialised local of the runtime's own type.
        unsafe { khalfiya.GetDesc(&raw mut wasf) }.map_err(|khata| KhataTabaqa::SathTaghayyar {
            sabab: format!("the backbuffer would not describe itself: {khata}"),
        })?;
        Ok(wasf)
    }

    /// Whether the device is in a state that can be drawn through.
    ///
    /// The two cooperative-level calls are not interchangeable.
    /// `TestCooperativeLevel` is the only answer a plain device has, and on a
    /// 9Ex device it is documented to always return `S_OK` — so an overlay that
    /// asked it on 9Ex would believe an occluded or removed device was fine.
    /// `CheckDeviceState` is 9Ex's replacement and it takes the window the
    /// answer is about, which is why the focus window is kept.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SathTaghayyar`] naming which of the three recoverable
    /// states the device is in. All three are skipped frames rather than
    /// faults: the game owns the device and only the game may reset it.
    fn hala_taawuniya(&self) -> Result<(), KhataTabaqa> {
        let natija = match self.jihaz_ex.as_ref() {
            // SAFETY: `mumtadd` is the same live device under its 9Ex face, and
            // `nafidha` is the focus window it was created against — a handle
            // the game owns for at least as long as the device does.
            Some(mumtadd) => unsafe { mumtadd.CheckDeviceState(self.nafidha) },
            // SAFETY: `jihaz` is the live device this backend was built around.
            None => unsafe { self.jihaz.TestCooperativeLevel() },
        };

        let Err(khata) = natija else {
            return Ok(());
        };

        // `S_PRESENT_OCCLUDED` and `S_PRESENT_MODE_CHANGED` are success codes
        // and never arrive here; everything that does is a real refusal, and the
        // four worth naming are named so a log says which.
        let sabab = match khata.code() {
            KHATA_JIHAZ_MAFQUD => {
                "the device is lost; nothing can be drawn until the game resets it".to_owned()
            },
            KHATA_JIHAZ_LAM_YUAAD => {
                "the device is lost and ready to be reset, which is the game's call to make"
                    .to_owned()
            },
            KHATA_JIHAZ_MUZAL => "the adapter this device was created on is gone".to_owned(),
            KHATA_JIHAZ_MUAALLAQ => {
                "the driver reset this device out from under the game".to_owned()
            },
            KHATA_DAAKHILI => "the driver reported an internal error".to_owned(),
            ramz => format!("the device reports {ramz:?}"),
        };
        Err(KhataTabaqa::SathTaghayyar { sabab })
    }

    /// Which pool the atlas belongs in on this device.
    ///
    /// `D3DPOOL_MANAGED` is refused outright by a 9Ex device — the pool does not
    /// exist there — so the choice is not an optimisation and cannot be made
    /// once for both.
    const fn hawd_lawha(&self) -> D3DPOOL {
        if self.jihaz_ex.is_some() {
            D3DPOOL_DEFAULT
        } else {
            D3DPOOL_MANAGED
        }
    }

    /// Releases everything that would make the game's own `Reset` fail.
    ///
    /// The state block always, and the atlas only when it is in
    /// `D3DPOOL_DEFAULT` — which is the 9Ex path, where the bytes are kept so
    /// [`KhattafD3D9::hayyi`] can put the page back afterwards.
    fn atliq_qabl_iaada(&mut self) {
        self.hala = None;
        if self.jihaz_ex.is_some() {
            self.lawha = None;
        }
        self.sath = None;
    }
}

impl KhattafD3D9 {
    /// Binds the fixed-function pipeline the overlay draws through.
    ///
    /// Every one of these is a state a `D3DSBT_ALL` block holds, so every one of
    /// them is put back by the `Apply` in [`Khattaf::irsim`]. What is *not*
    /// here is as deliberate as what is: no transform is set, because
    /// `D3DFVF_XYZRHW` bypasses the transform stages, and a backend that set the
    /// world, view and projection matrices would be a backend that depended on
    /// having read the game's first.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] on the first state the device refuses. A
    /// refusal here leaves the pipeline half-bound, which is why the caller
    /// applies the state block whatever this returns.
    fn irbut_hala(&self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        let radd = |mawrid: &'static str| {
            move |khata: windows::core::Error| KhataTabaqa::MawridFashil {
                mawrid,
                sabab: khata.to_string(),
            }
        };

        // SAFETY: `jihaz` is the live device this backend was built around and
        // every value below is a documented member of the enumeration its
        // parameter names. D3D9 validates state at draw time rather than at set
        // time, so these record rather than take effect, and each returns an
        // `HRESULT` that is checked.
        unsafe {
            self.jihaz
                .SetFVF(SIGHAT_RAS)
                .map_err(radd("overlay vertex format"))?;
            // Both stages cleared. A game with a pixel shader still bound would
            // run it over the overlay's triangles, and whatever it produced
            // would not be Arabic.
            self.jihaz
                .SetVertexShader(None::<&IDirect3DVertexShader9>)
                .map_err(radd("overlay vertex stage"))?;
            self.jihaz
                .SetPixelShader(None::<&IDirect3DPixelShader9>)
                .map_err(radd("overlay pixel stage"))?;

            // Premultiplied alpha, as everywhere else in this crate. `SRCALPHA`
            // would be the reflex and it is wrong: `saff` hands over coverage
            // already multiplied into the colour, and multiplying it again
            // darkens every glyph edge relative to its interior.
            let hala = |ism: D3DRENDERSTATETYPE, qeema: u32| {
                self.jihaz
                    .SetRenderState(ism, qeema)
                    .map_err(radd("overlay render state"))
            };
            hala(D3DRS_ALPHABLENDENABLE, 1)?;
            hala(D3DRS_SRCBLEND, D3DBLEND_ONE.0.cast_unsigned())?;
            hala(D3DRS_DESTBLEND, D3DBLEND_INVSRCALPHA.0.cast_unsigned())?;
            hala(D3DRS_BLENDOP, D3DBLENDOP_ADD.0.cast_unsigned())?;
            // Off, so the alpha channel takes the same factors the colour does.
            // A game that left it on with its own alpha factors would have the
            // overlay writing an alpha nobody asked for into the backbuffer.
            hala(D3DRS_SEPARATEALPHABLENDENABLE, 0)?;
            hala(D3DRS_ALPHATESTENABLE, 0)?;

            // Depth test *and* depth write both off. Turning off only the test
            // is the common half-fix: the overlay then draws on top correctly
            // and stamps its quads into the depth buffer, and the game's next
            // frame finds a wall of geometry at the near plane where the
            // subtitles were.
            hala(D3DRS_ZENABLE, D3DZB_FALSE.0.cast_unsigned())?;
            hala(D3DRS_ZWRITEENABLE, 0)?;
            hala(D3DRS_STENCILENABLE, 0)?;

            // Culling off because the overlay's quads are generated in one
            // winding, and a game that left a front-face convention set would
            // otherwise cull every one of them — which looks exactly like the
            // overlay never ran.
            hala(D3DRS_CULLMODE, D3DCULL_NONE.0.cast_unsigned())?;
            hala(D3DRS_FILLMODE, D3DFILL_SOLID.0.cast_unsigned())?;
            hala(D3DRS_SHADEMODE, D3DSHADE_GOURAUD.0.cast_unsigned())?;
            // Cleared, not left: a stale scissor rectangle would clip the
            // overlay to wherever the game last drew a minimap.
            hala(D3DRS_SCISSORTESTENABLE, 0)?;
            hala(D3DRS_CLIPPING, 1)?;
            hala(D3DRS_CLIPPLANEENABLE, 0)?;
            hala(D3DRS_LIGHTING, 0)?;
            hala(D3DRS_FOGENABLE, 0)?;
            hala(D3DRS_VERTEXBLEND, D3DVBF_DISABLE.0.cast_unsigned())?;
            hala(D3DRS_INDEXEDVERTEXBLENDENABLE, 0)?;
            hala(D3DRS_ANTIALIASEDLINEENABLE, 0)?;
            hala(D3DRS_COLORWRITEENABLE, KITABAT_ALWAN)?;
            // Explicitly off. The encode is done on the processor in `lawn_d3d`
            // because this state only converts after blending on hardware that
            // reports `D3DPMISCCAPS_POSTBLENDSRGBCONVERT`, and a game that left
            // it on would have the driver encode an already-encoded colour.
            hala(D3DRS_SRGBWRITEENABLE, 0)?;

            // Stage one disabled terminates the cascade. Without it the
            // pipeline keeps combining through whatever the game left in stages
            // one to seven.
            self.jihaz
                .SetTextureStageState(1, D3DTSS_COLOROP, D3DTOP_DISABLE.0.cast_unsigned())
                .map_err(radd("overlay texture stage"))?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_TEXCOORDINDEX, 0)
                .map_err(radd("overlay texture stage"))?;
            self.jihaz
                .SetTextureStageState(
                    0,
                    D3DTSS_TEXTURETRANSFORMFLAGS,
                    D3DTTFF_DISABLE.0.cast_unsigned(),
                )
                .map_err(radd("overlay texture stage"))?;

            // Clamped addressing so a glyph whose atlas rectangle sits against
            // the page edge cannot bleed a neighbouring glyph into itself, and
            // linear filtering because the overlay draws at whatever scale the
            // control panel's font size asks for rather than always one to one.
            let akhidh = |ism: D3DSAMPLERSTATETYPE, qeema: u32| {
                self.jihaz
                    .SetSamplerState(0, ism, qeema)
                    .map_err(radd("overlay sampler state"))
            };
            akhidh(D3DSAMP_MINFILTER, D3DTEXF_LINEAR.0.cast_unsigned())?;
            akhidh(D3DSAMP_MAGFILTER, D3DTEXF_LINEAR.0.cast_unsigned())?;
            akhidh(D3DSAMP_MIPFILTER, D3DTEXF_NONE.0.cast_unsigned())?;
            akhidh(D3DSAMP_ADDRESSU, D3DTADDRESS_CLAMP.0.cast_unsigned())?;
            akhidh(D3DSAMP_ADDRESSV, D3DTADDRESS_CLAMP.0.cast_unsigned())?;
            // The atlas is premultiplied linear coverage, not colour anybody
            // encoded, so the sampler must not decode it.
            akhidh(D3DSAMP_SRGBTEXTURE, 0)?;

            let manzar = D3DVIEWPORT9 {
                X: 0,
                Y: 0,
                Width: sath.ard,
                Height: sath.irtifa,
                MinZ: 0.0,
                MaxZ: 1.0,
            };
            self.jihaz
                .SetViewport(&raw const manzar)
                .map_err(radd("overlay viewport"))?;
        }
        Ok(())
    }

    /// Switches the texture cascade between a glyph run and a plate run.
    ///
    /// This is what [`crate::d3d11`]'s per-vertex `khalt` weight becomes on a
    /// pipeline with no shader to weigh anything in. There, a batch mixing
    /// plates and glyphs is one draw call because the weight rides on the
    /// vertex; here the two need different stage operations, so the batch is cut
    /// into runs of like quads and each run is its own call.
    ///
    /// The cut is cheap because the feeder already emits in runs: a plate,
    /// then every glyph that sits on it, then the next plate. A line of Arabic
    /// is one plate call and one glyph call.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the device refuses a stage state.
    fn ibdal_marhala(&self, masturat: bool) -> Result<(), KhataTabaqa> {
        let radd = |khata: windows::core::Error| KhataTabaqa::MawridFashil {
            mawrid: "overlay texture stage",
            sabab: khata.to_string(),
        };
        let (amaliya, awwal) = if masturat {
            (D3DTOP_MODULATE, D3DTA_TEXTURE)
        } else {
            // Nothing is sampled for a plate, so the operation selects the
            // vertex colour outright rather than modulating it by a texel this
            // quad's coordinates do not address.
            (D3DTOP_SELECTARG1, D3DTA_DIFFUSE)
        };

        // SAFETY: `jihaz` is the live device and every argument is a documented
        // member of the enumeration its parameter names.
        unsafe {
            self.jihaz
                .SetTextureStageState(0, D3DTSS_COLOROP, amaliya.0.cast_unsigned())
                .map_err(radd)?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_COLORARG1, awwal)
                .map_err(radd)?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_COLORARG2, D3DTA_DIFFUSE)
                .map_err(radd)?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_ALPHAOP, amaliya.0.cast_unsigned())
                .map_err(radd)?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_ALPHAARG1, awwal)
                .map_err(radd)?;
            self.jihaz
                .SetTextureStageState(0, D3DTSS_ALPHAARG2, D3DTA_DIFFUSE)
                .map_err(radd)?;
        }
        Ok(())
    }

    /// Draws every run of a batch through an already-bound pipeline.
    ///
    /// Split out of [`Khattaf::irsim`] so that the capture, the draw and the
    /// apply are three visibly separate things: the apply must run whatever this
    /// returns, and a draw written inline with early returns in it is how that
    /// stops being true.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when a resource is missing or the device
    /// refuses a state or a draw.
    fn arsim_dufaat(
        &self,
        musawwada: &mut Vec<RasD3D9>,
        lawha: &LawhatRasm,
    ) -> Result<(), KhataTabaqa> {
        // A batch that samples the atlas without one uploaded would draw
        // rectangles of whatever stage zero happens to hold, which on some
        // drivers is the game's last texture. Refusing is the only honest answer.
        let yastir = lawha.qitaat.iter().any(|qita| qita.khareeta.is_some());
        if self.lawha.is_none() && yastir {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the batch samples an atlas that has not been uploaded".to_owned(),
            });
        }

        self.irbut_hala(lawha.sath)?;

        // `as_deref` rather than a cast: `IDirect3DTexture9` derefs to
        // `IDirect3DBaseTexture9`, which is the interface `SetTexture` takes,
        // and a `QueryInterface` to reach it would be a reference count per
        // frame for a conversion the type system already knows is free.
        let masdar: Option<&IDirect3DBaseTexture9> = self.lawha.as_deref();
        // SAFETY: `jihaz` is the live device and `masdar` is either a texture
        // this backend created or nothing. The call returns an `HRESULT` either
        // way, which is checked.
        unsafe { self.jihaz.SetTexture(0, masdar) }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas",
            sabab: khata.to_string(),
        })?;

        let mut marhala: Option<bool> = None;
        let mut bidaya = 0_usize;
        while bidaya < lawha.qitaat.len() {
            let Some(baqi) = lawha.qitaat.get(bidaya..) else {
                break;
            };
            let Some(awwal) = baqi.first() else {
                break;
            };
            let masturat = awwal.khareeta.is_some();
            // One run is the longest stretch of like quads, capped at what a
            // sixteen-bit index array can address.
            let tul = baqi
                .iter()
                .take(QITA_LIL_DUFA)
                .take_while(|qita| qita.khareeta.is_some() == masturat)
                .count();
            let Some(dufa) = baqi.get(..tul) else {
                break;
            };

            if marhala != Some(masturat) {
                self.ibdal_marhala(masturat)?;
                marhala = Some(masturat);
            }

            musawwada.clear();
            for qita in dufa {
                imla_qita_d3d9(musawwada, qita);
            }
            self.irsim_dufa(musawwada, dufa.len())?;
            bidaya = bidaya.saturating_add(tul.max(1));
        }
        Ok(())
    }

    /// One `DrawIndexedPrimitiveUP` over a run of like quads.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the device refuses the draw.
    fn irsim_dufa(&self, musawwada: &[RasD3D9], adad_qitaat: usize) -> Result<(), KhataTabaqa> {
        if musawwada.is_empty() || adad_qitaat == 0 {
            return Ok(());
        }
        let ruus = u32::try_from(musawwada.len()).unwrap_or(0);
        let muthallathat = u32::try_from(adad_qitaat.saturating_mul(2)).unwrap_or(0);
        let faharis = self
            .faharis
            .get(..adad_qitaat.saturating_mul(6))
            .unwrap_or(&[]);
        if ruus == 0 || muthallathat == 0 || faharis.is_empty() {
            return Ok(());
        }

        // SAFETY: `faharis` holds exactly six indices per quad in this run, and
        // every index it holds is below `ruus` because `faharis_thabita` builds
        // them as four per quad in order and this slice covers the same quads
        // `musawwada` was filled from. Both pointers address live slices that
        // outlive the call — the runtime copies out of them before returning —
        // and the stride is the compile-time-checked size of `RasD3D9`.
        unsafe {
            self.jihaz.DrawIndexedPrimitiveUP(
                D3DPT_TRIANGLELIST,
                0,
                ruus,
                muthallathat,
                faharis.as_ptr().cast::<c_void>(),
                D3DFMT_INDEX16,
                musawwada.as_ptr().cast::<c_void>(),
                KHATWAT_RAS,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay draw",
            sabab: khata.to_string(),
        })
    }
}

impl KhattafD3D9 {
    /// Points the device at the backbuffer, opens a scene, draws, closes it.
    ///
    /// Everything that mutates device state is inside this one function, so
    /// [`Khattaf::irsim`]'s restore path is unconditional and has one thing to
    /// undo rather than several interleaved with early returns.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the render target cannot be bound,
    /// the scene cannot be opened, or the draw is refused. All three are
    /// recoverable: the caller restores and skips the frame.
    fn arsim_kamil(
        &self,
        khalfiya: &IDirect3DSurface9,
        musawwada: &mut Vec<RasD3D9>,
        lawha: &LawhatRasm,
    ) -> Result<(), KhataTabaqa> {
        // SAFETY: `jihaz` is the live device and `khalfiya` is its own
        // backbuffer, fetched by the caller and alive for this whole call. The
        // depth-stencil is cleared rather than left, because a surface sized for
        // a different render target is a refusal at draw time.
        unsafe {
            self.jihaz
                .SetRenderTarget(0, khalfiya)
                .map_err(|khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay render target",
                    sabab: khata.to_string(),
                })?;
            self.jihaz
                .SetDepthStencilSurface(None::<&IDirect3DSurface9>)
                .map_err(|khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay depth-stencil binding",
                    sabab: khata.to_string(),
                })?;
        }

        // A draw outside a scene is refused by the runtime, and `Present` is
        // called with the scene closed — so the overlay opens one of its own.
        // A refusal here means something else in the process already has a
        // scene open, which is a skipped frame rather than a fault.
        // SAFETY: `jihaz` is the live device.
        unsafe { self.jihaz.BeginScene() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay scene",
            sabab: format!("a scene could not be opened to draw the overlay in: {khata}"),
        })?;

        let natija = self.arsim_dufaat(musawwada, lawha);

        // Unconditional: a scene left open makes the game's own `Present` fail.
        // SAFETY: the `BeginScene` above succeeded, so exactly one `EndScene` is
        // owed and this is it.
        let khatm = unsafe { self.jihaz.EndScene() };
        natija?;
        khatm.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay scene",
            sabab: format!("the overlay's scene could not be closed: {khata}"),
        })
    }
}

impl Khattaf for KhattafD3D9 {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Direct3D9
    }

    /// Releases the state block, and the atlas when its pool will not survive.
    ///
    /// Reached from the `Reset` hook, before the call goes to the runtime.
    /// `Reset` fails with `D3DERR_INVALIDCALL` while a state block exists, and
    /// the failure would land on the game rather than on Taarib — see this
    /// module's header.
    fn atliq_sath(&mut self) -> Result<(), KhataTabaqa> {
        self.atliq_qabl_iaada();
        Ok(())
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        // Asked first, and every frame. A lost device answers every other call
        // with the same error, and reading the cooperative level is the one
        // that says *why* rather than leaving the caller to infer it from a
        // backbuffer that would not come.
        self.hala_taawuniya()?;

        let wasf = self.wasf_khalfiya()?;
        if wasf.Width == 0 || wasf.Height == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!(
                    "the backbuffer reports a {}×{} surface",
                    wasf.Width, wasf.Height
                ),
            });
        }

        let sigha = sigha_min_d3d9(wasf.Format)?;
        Ok(WasfSath {
            ard: wasf.Width,
            irtifa: wasf.Height,
            sigha,
            // Direct3D 9 has no sRGB swap-chain format and this backend leaves
            // `D3DRS_SRGBWRITEENABLE` off, so nothing downstream encodes for the
            // overlay. `lawn_d3d` does it instead. See this module's header.
            sirgb: false,
        })
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        // Released before anything is allocated, and released unconditionally:
        // this runs both at startup, where there is nothing to release, and
        // after a reset, where holding the previous state block through the
        // allocation of a new one would be holding an object the device has
        // already invalidated.
        self.hala = None;

        // SAFETY: `jihaz` is the live device this backend was built around, and
        // `D3DSBT_ALL` is the documented type for a full-device snapshot. The
        // call is made outside any `BeginScene`/`EndScene` pair — `Present`, the
        // only place this is reached from, is called with the scene closed.
        let hala = unsafe { self.jihaz.CreateStateBlock(D3DSBT_ALL) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "overlay state block",
                sabab: khata.to_string(),
            }
        })?;
        self.hala = Some(hala);

        if self.faharis.is_empty() {
            self.faharis = faharis_thabita();
        }
        // The scratch vertex array is grown once rather than per frame. It is
        // the only allocation on the draw path and it is made here, off it.
        let matlub = QITA_LIL_DUFA.saturating_mul(4);
        if self.musawwada.capacity() < matlub {
            self.musawwada
                .reserve(matlub.saturating_sub(self.musawwada.capacity()));
        }

        // The 9Ex path only: a `D3DPOOL_DEFAULT` atlas did not survive the reset
        // that brought us here, and the bytes were kept for exactly this moment.
        if self.lawha.is_none()
            && let Some((bayt, ard, irtifa)) = self.bayt_lawha.take()
        {
            let natija = self.arfa_lawha(&bayt, ard, irtifa);
            self.bayt_lawha = Some((bayt, ard, irtifa));
            natija?;
        }

        self.sath = Some(sath);
        Ok(())
    }

    fn arfa_lawha(&mut self, bayt: &[u8], ard: u32, irtifa: u32) -> Result<(), KhataTabaqa> {
        if ard > self.saqf_lawha || irtifa > self.saqf_lawha {
            return Err(KhataTabaqa::HajmMufrit {
                haql: "glyph atlas dimension",
                qeema: u64::from(ard.max(irtifa)),
                saqf: u64::from(self.saqf_lawha),
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
        // the end by this function, inside the game's process, with no error.
        let matlub = u64::from(ard)
            .saturating_mul(4)
            .saturating_mul(u64::from(irtifa));
        if tul_u64(bayt.len()) < matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: format!(
                    "a {ard}×{irtifa} RGBA8 atlas needs {matlub} bytes and {} were supplied",
                    bayt.len()
                ),
            });
        }

        let hawd = self.hawd_lawha();
        // A managed texture is written once and restored by the runtime; a
        // default-pool one has to be dynamic to be lockable at all.
        let istikhdam = if hawd == D3DPOOL_DEFAULT {
            D3DUSAGE_DYNAMIC.cast_unsigned()
        } else {
            0
        };

        let mut lawha: Option<IDirect3DTexture9> = None;
        // SAFETY: `jihaz` is the live device, the out-pointer addresses a local
        // initialised to `None` so the reference the runtime writes is owned by
        // it, and the shared-handle parameter is null, which the runtime
        // documents as "not shared".
        unsafe {
            self.jihaz.CreateTexture(
                ard,
                irtifa,
                1,
                istikhdam,
                D3DFMT_A8R8G8B8,
                hawd,
                &raw mut lawha,
                core::ptr::null_mut(),
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

        let alam = if hawd == D3DPOOL_DEFAULT {
            D3DLOCK_DISCARD.cast_unsigned()
        } else {
            0
        };
        let mut marsum = D3DLOCKED_RECT::default();
        // SAFETY: `lawha` is the texture created immediately above with one mip
        // level, the out-pointer addresses a live local, and a null rectangle
        // asks for the whole surface.
        unsafe { lawha.LockRect(0, &raw mut marsum, core::ptr::null(), alam) }.map_err(
            |khata| KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("the atlas texture could not be locked for writing: {khata}"),
            },
        )?;

        let natija = uktub_lawha(&marsum, bayt, ard, irtifa);

        // SAFETY: the lock above succeeded, so exactly one unlock is owed. It
        // runs before the result is inspected so an error on the copy path does
        // not leave the texture locked.
        let fakk = unsafe { lawha.UnlockRect(0) };
        natija?;
        fakk.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: format!("the atlas texture could not be unlocked: {khata}"),
        })?;

        self.lawha = Some(lawha);
        // Kept only where the pool will not survive a reset. On a plain device
        // the managed copy the runtime holds is the copy, and a second one here
        // would be a megabyte of a game's memory spent on nothing.
        self.bayt_lawha = if hawd == D3DPOOL_DEFAULT {
            Some((bayt.to_vec(), ard, irtifa))
        } else {
            None
        };
        Ok(())
    }

    fn irsim(&mut self, lawha: &LawhatRasm) -> Result<(), KhataTabaqa> {
        if lawha.khali() {
            return Ok(());
        }
        if self.sath.is_none() {
            // Reached after `atliq_sath` let the game reset the device. The
            // caller only calls `hayyi` when the surface *description* changes,
            // and a reset that keeps the same width, height and format changes
            // nothing it can see — so the rebuild is done here, against the
            // surface the caller already checked this batch was built for.
            self.hayyi(lawha.sath)?;
        }
        if self.sath != Some(lawha.sath) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay batch",
                sabab: "the batch was positioned against a surface this backend is not \
                        currently built for"
                    .to_owned(),
            });
        }

        let Some(hala) = self.hala.clone() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay state block",
                sabab: "the backend was asked to draw before its state block was built".to_owned(),
            });
        };

        // Everything up to the first `Set` reads and does not write, so a
        // failure in this block is a skipped frame with the game's pipeline
        // exactly as it left it.
        //
        // SAFETY: `hala` is the state block this backend created against the
        // live device, and `Capture` refreshes it from that device's current
        // state without changing any of it.
        unsafe { hala.Capture() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay state block",
            sabab: format!("the device's state could not be captured: {khata}"),
        })?;

        // SAFETY: `jihaz` is the live device; `GetBackBuffer` and
        // `GetRenderTarget` return owned references dropped at the end of this
        // function, which is what keeps this backend from holding a backbuffer
        // across a reset.
        let khalfiya = unsafe { self.jihaz.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }.map_err(
            |khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay render target",
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;
        // SAFETY: as above. The render target is not state-block state, so it
        // is saved and restored by hand — see this module's header.
        let hadaf_qadim = unsafe { self.jihaz.GetRenderTarget(0) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "overlay render target",
                sabab: format!("the game's render target could not be read: {khata}"),
            }
        })?;
        // SAFETY: as above. A device with no depth-stencil bound answers
        // `D3DERR_NOTFOUND`, which is the ordinary case for a game that has
        // finished its scene and not an error.
        let umq_qadim = match unsafe { self.jihaz.GetDepthStencilSurface() } {
            Ok(umq) => Some(umq),
            Err(khata) if khata.code() == KHATA_GHAYR_MAWJUD => None,
            Err(khata) => {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay depth-stencil binding",
                    sabab: format!("the game's depth-stencil surface could not be read: {khata}"),
                });
            },
        };

        let mut musawwada = mem::take(&mut self.musawwada);
        let natija = self.arsim_kamil(&khalfiya, &mut musawwada, lawha);
        musawwada.clear();
        self.musawwada = musawwada;

        // Unconditional, and before the result is inspected. There is no error
        // this backend can return that makes leaving the game's device pointed
        // at the overlay's state the better outcome.
        //
        // The order is not interchangeable. `SetRenderTarget` resets the
        // viewport as a documented side effect, so the render target goes back
        // *before* the state block is applied — applying first and then setting
        // the target would restore the game's viewport and immediately discard
        // it. The depth-stencil follows the render target because a surface
        // sized for a target that is not yet bound is refused.
        let mut fasad: Vec<String> = Vec::new();
        // SAFETY: `hadaf_qadim` is the surface the game had bound, read a few
        // lines above and alive for this whole function.
        if let Err(khata) = unsafe { self.jihaz.SetRenderTarget(0, &hadaf_qadim) } {
            fasad.push(format!("the render target could not be put back: {khata}"));
        }
        // SAFETY: `umq_qadim` is the surface the game had bound, or nothing.
        if let Err(khata) = unsafe { self.jihaz.SetDepthStencilSurface(umq_qadim.as_ref()) } {
            fasad.push(format!(
                "the depth-stencil surface could not be put back: {khata}"
            ));
        }
        // SAFETY: `hala` holds the snapshot `Capture` took at the head of this
        // function, from this same live device.
        if let Err(khata) = unsafe { hala.Apply() } {
            fasad.push(format!(
                "the captured device state could not be re-applied: {khata}"
            ));
        }

        if !fasad.is_empty() {
            // Terminal, and terminal immediately. Unlike D3D11, where a failed
            // restore is only visible as a removed device, D3D9 says so: these
            // three calls each return an `HRESULT` and the game is now drawing
            // with state it did not set.
            return Err(KhataTabaqa::HalaGhayrMustaada {
                hala: "the Direct3D 9 device state",
                sabab: fasad.join("; "),
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

        let wasf = self
            .wasf_khalfiya()
            .map_err(|khata| KhataTabaqa::IltiqatFashil {
                sabab: khata.to_string(),
            })?;

        // SAFETY: `jihaz` is the live device; the reference is dropped when this
        // function returns.
        let khalfiya = unsafe { self.jihaz.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO) }.map_err(
            |khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;

        let sunduq = mustatil_win(mintaqa)?;

        // The region is copied into a render target of its own size first, for
        // two reasons that both end in the same call. `GetRenderTargetData`
        // refuses a multisampled source outright, and it copies whole surfaces
        // rather than rectangles — so a direct read-back of a dialogue box would
        // be a read-back of the entire screen. `StretchRect` with
        // `D3DTEXF_NONE` is the documented multisample resolve and it takes a
        // source rectangle, which answers both.
        let mut hall: Option<IDirect3DSurface9> = None;
        // SAFETY: the out-pointer addresses a local initialised to `None` and
        // the shared-handle parameter is null. `lockable` is false because
        // nothing locks this surface; it is read through
        // `GetRenderTargetData`.
        unsafe {
            self.jihaz.CreateRenderTarget(
                mintaqa.ard,
                mintaqa.irtifa,
                wasf.Format,
                D3DMULTISAMPLE_NONE,
                0,
                false,
                &raw mut hall,
                core::ptr::null_mut(),
            )
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back render target could not be created: {khata}"),
        })?;
        let Some(hall) = hall else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back render target was not produced".to_owned(),
            });
        };

        // SAFETY: both surfaces are live, the source rectangle was checked
        // against the surface dimensions at the head of this function, and the
        // destination is the same size with no multisampling — which is exactly
        // the shape `StretchRect` documents for a resolve. `D3DTEXF_NONE` is
        // required for one and correct for a one-to-one copy.
        unsafe {
            self.jihaz.StretchRect(
                &khalfiya,
                &raw const sunduq,
                &hall,
                core::ptr::null(),
                D3DTEXF_NONE,
            )
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the region could not be copied out of the backbuffer: {khata}"),
        })?;

        let mut marhala: Option<IDirect3DSurface9> = None;
        // SAFETY: as the render target above. `D3DPOOL_SYSTEMMEM` is the one
        // pool `GetRenderTargetData` accepts as a destination.
        unsafe {
            self.jihaz.CreateOffscreenPlainSurface(
                mintaqa.ard,
                mintaqa.irtifa,
                wasf.Format,
                D3DPOOL_SYSTEMMEM,
                &raw mut marhala,
                core::ptr::null_mut(),
            )
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back surface could not be created: {khata}"),
        })?;
        let Some(marhala) = marhala else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back surface was not produced".to_owned(),
            });
        };

        // SAFETY: `hall` is a render target and `marhala` is a system-memory
        // surface of the same size and format, which is what this call requires
        // of its two arguments.
        unsafe { self.jihaz.GetRenderTargetData(&hall, &marhala) }.map_err(|khata| {
            KhataTabaqa::IltiqatFashil {
                sabab: format!("the region could not be read back from the GPU: {khata}"),
            }
        })?;

        let mut marsum = D3DLOCKED_RECT::default();
        // SAFETY: `marhala` is the system-memory surface created above, the
        // out-pointer addresses a live local, and a null rectangle asks for the
        // whole surface, which is the region.
        unsafe {
            marhala.LockRect(
                &raw mut marsum,
                core::ptr::null(),
                D3DLOCK_READONLY.cast_unsigned(),
            )
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back surface could not be locked: {khata}"),
        })?;

        let natija = jami_sufuf(&marsum, mintaqa, wasf.Format);

        // SAFETY: the lock above succeeded, so exactly one unlock is owed. It
        // runs before the result is inspected so an error on the copy path does
        // not leave the surface locked.
        let fakk = unsafe { marhala.UnlockRect() };
        let bayt = natija?;
        fakk.map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back surface could not be unlocked: {khata}"),
        })?;
        Ok(bayt)
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        // Idempotent by construction: every field is an `Option` or a `Vec` and
        // clearing one twice is clearing it. The second call is the common one —
        // `Tabaqa::shaghghil` tears a backend down when `hayyi` failed, and then
        // the caller drops it, which runs this again.
        self.hala = None;
        self.lawha = None;
        self.bayt_lawha = None;
        self.sath = None;
        self.musawwada = Vec::new();
        self.faharis = Vec::new();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Formats, and the two conversions this API needs that no other one does
// ---------------------------------------------------------------------------

/// Reduces a Direct3D 9 backbuffer format to what the capture path needs.
///
/// Four formats are accepted and the reasoning differs across them.
///
/// `A8R8G8B8` and `X8R8G8B8` are the ordinary case. A `D3DCOLOR` is `0xAARRGGBB`
/// as a word, which on every target this builds for puts blue at the lowest
/// address — so the bytes are `B`, `G`, `R`, `A` and the format is
/// [`SighatSath::Bgra8`], not `Rgba8`. Getting that backwards produces a
/// recognizer that reads red text as blue and finds nothing.
///
/// `R5G6B5` and `X1R5G5B5` are the era's other real answer, and they have no
/// member in [`SighatSath`]. They are accepted anyway and widened to eight bits
/// per channel during the read-back — see [`jami_sufuf`]. Refusing them would
/// mean refusing tier 3 on a class of games that is exactly what this backend
/// was written for, and the widening loses nothing.
///
/// `A2R10G10B10` is refused, and it is worth saying why rather than folding it
/// into "unsupported". [`SighatSath::Rgb10a2`] is DXGI's layout — red in the low
/// bits, alpha in the high two — and D3D9's `A2R10G10B10` is the reverse. They
/// are not the same format under two names, and reporting one as the other would
/// hand the converter a picture with its red and blue channels exchanged and its
/// alpha mixed into red. `A16B16G16R16F` is refused for the plainer reason that
/// a 9Ex float swap chain is scRGB the recognizer's tone mapper has never seen
/// from this API.
///
/// # Errors
///
/// [`KhataTabaqa::SighaGhayrMaduma`] naming the format's own number, because a
/// format nobody has seen is more useful in a bug report as a number than as
/// the word "unsupported".
pub fn sigha_min_d3d9(sigha: D3DFORMAT) -> Result<SighatSath, KhataTabaqa> {
    match sigha {
        D3DFMT_A8R8G8B8 | D3DFMT_X8R8G8B8 | D3DFMT_R5G6B5 | D3DFMT_X1R5G5B5 => {
            Ok(SighatSath::Bgra8)
        },
        D3DFMT_A2R10G10B10 => Err(KhataTabaqa::SighaGhayrMaduma {
            sigha: "D3DFMT_A2R10G10B10, whose channel order is the reverse of the ten-bit format \
                    this build converts"
                .to_owned(),
        }),
        D3DFMT_A16B16G16R16F => Err(KhataTabaqa::SighaGhayrMaduma {
            sigha: "D3DFMT_A16B16G16R16F".to_owned(),
        }),
        _ => Err(KhataTabaqa::SighaGhayrMaduma {
            sigha: format!("D3DFORMAT({})", sigha.0),
        }),
    }
}

/// How many bytes one pixel of a format this build accepts occupies.
const fn bayt_lil_biksel(sigha: D3DFORMAT) -> usize {
    match sigha {
        D3DFMT_R5G6B5 | D3DFMT_X1R5G5B5 => 2,
        _ => 4,
    }
}

/// A pixel rectangle as the `RECT` Direct3D 9 takes.
///
/// # Errors
///
/// [`KhataTabaqa::MintaqaKharij`] when an edge does not fit a signed 32-bit
/// integer, which a rectangle on a real surface never does and a corrupted
/// region file might.
fn mustatil_win(mintaqa: MustatilBiksel) -> Result<RECT, KhataTabaqa> {
    let hadd = |qeema: u32| -> Result<i32, KhataTabaqa> {
        i32::try_from(qeema).map_err(|_| KhataTabaqa::MintaqaKharij {
            mintaqa: format!("an edge at {qeema}"),
            ard: mintaqa.ard,
            irtifa: mintaqa.irtifa,
        })
    };
    Ok(RECT {
        left: hadd(mintaqa.yasar)?,
        top: hadd(mintaqa.aala)?,
        right: hadd(mintaqa.yasar.saturating_add(mintaqa.ard))?,
        bottom: hadd(mintaqa.aala.saturating_add(mintaqa.irtifa))?,
    })
}

/// Writes the atlas page into a locked texture, row by row, swizzled.
///
/// Two things are being handled and only one of them is obvious.
///
/// `Pitch` is the distance between the starts of two rows in the mapping, which
/// the driver rounds up to whatever alignment it prefers. It is *not* the row's
/// width, and a write that treats it as one shears the atlas progressively to
/// one side — which shows up as glyphs sampling their neighbours rather than as
/// anything that looks like a stride bug.
///
/// The swizzle is the D3D9-specific half. The trait hands over premultiplied
/// RGBA8 in row order, and `D3DFMT_A8R8G8B8` stores blue at the lowest address,
/// so red and blue are exchanged on the way in. Today the page is coverage
/// replicated into all four channels and the exchange is a no-op — but relying
/// on that would make this file quietly wrong the first time the atlas carried a
/// colour, and the exchange costs nothing.
///
/// # Errors
///
/// [`KhataTabaqa::MawridFashil`] when the mapping has no pointer or reports a
/// pitch shorter than one row, both of which mean the lock did not produce what
/// was asked for.
fn uktub_lawha(
    marsum: &D3DLOCKED_RECT,
    bayt: &[u8],
    ard: u32,
    irtifa: u32,
) -> Result<(), KhataTabaqa> {
    if marsum.pBits.is_null() {
        return Err(KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: "the atlas mapping has no pointer".to_owned(),
        });
    }
    let tul_saf = usize::try_from(ard).unwrap_or(0).saturating_mul(4);
    let khatwa = usize::try_from(marsum.Pitch).unwrap_or(0);
    if tul_saf == 0 || khatwa < tul_saf {
        return Err(KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: format!(
                "the atlas mapping reports a {khatwa}-byte pitch for a {tul_saf}-byte row"
            ),
        });
    }

    for saf in 0..usize::try_from(irtifa).unwrap_or(0) {
        let bidaya = saf.saturating_mul(tul_saf);
        let Some(masdar) = bayt.get(bidaya..bidaya.saturating_add(tul_saf)) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!("the atlas page ends before row {saf}"),
            });
        };
        // SAFETY: the mapping covers `Pitch * irtifa` bytes for a texture
        // created at exactly `ard`×`irtifa`, so a row start at `saf * khatwa`
        // with `saf` below the height is inside it, and `tul_saf` bytes from
        // there are inside that row because `khatwa` was checked to be at least
        // `tul_saf`. The mapping outlives this function, which runs before the
        // matching unlock.
        let hadaf = unsafe {
            let asas = marsum.pBits.cast::<u8>().add(saf.saturating_mul(khatwa));
            core::slice::from_raw_parts_mut(asas, tul_saf)
        };
        for (texel_in, texel_out) in masdar.chunks_exact(4).zip(hadaf.chunks_exact_mut(4)) {
            let (Some(r), Some(g), Some(b), Some(a)) = (
                texel_in.first(),
                texel_in.get(1),
                texel_in.get(2),
                texel_in.get(3),
            ) else {
                continue;
            };
            // B, G, R, A — the byte order `D3DFMT_A8R8G8B8` names from the
            // high end of a word down.
            texel_out.copy_from_slice(&[*b, *g, *r, *a]);
        }
    }
    Ok(())
}

/// Copies a locked read-back out row by row, dropping the pitch and widening.
///
/// [`Khattaf::iltaqit`] promises tightly packed bytes in the format
/// [`Khattaf::sath`] reported, so both of this function's jobs happen here
/// rather than in the recognizer.
///
/// The pitch is dropped for the reason [`uktub_lawha`] states in the other
/// direction. The widening is the sixteen-bit case: a `R5G6B5` or `X1R5G5B5`
/// surface is expanded to `B`, `G`, `R`, `A` bytes with the channel's own high
/// bits replicated into the low ones — `v << 3 | v >> 2` for five bits — so that
/// a full-scale channel stays full-scale instead of landing at two hundred and
/// forty-eight. A linear shift alone would darken the whole capture by three
/// percent, which is small enough that nobody would look for it and large enough
/// to move a threshold the recognizer's preprocessor picks.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the mapping has no pointer or its pitch
/// is shorter than one row of the region.
fn jami_sufuf(
    marsum: &D3DLOCKED_RECT,
    mintaqa: MustatilBiksel,
    sigha: D3DFORMAT,
) -> Result<Vec<u8>, KhataTabaqa> {
    if marsum.pBits.is_null() {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: "the read-back mapping has no pointer".to_owned(),
        });
    }

    let bayt_biksel = bayt_lil_biksel(sigha);
    let ard = usize::try_from(mintaqa.ard).unwrap_or(0);
    let irtifa = usize::try_from(mintaqa.irtifa).unwrap_or(0);
    let tul_saf = ard.saturating_mul(bayt_biksel);
    let khatwa = usize::try_from(marsum.Pitch).unwrap_or(0);
    if tul_saf == 0 || khatwa < tul_saf {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: format!(
                "the read-back mapping reports a {khatwa}-byte pitch for a {tul_saf}-byte row"
            ),
        });
    }

    let mut kharj = Vec::with_capacity(ard.saturating_mul(irtifa).saturating_mul(4));
    for saf in 0..irtifa {
        // SAFETY: the mapping covers `Pitch * irtifa` bytes for a surface
        // created at exactly this region's dimensions, so a row start at
        // `saf * khatwa` with `saf` below the height is inside it, and `tul_saf`
        // bytes from there are inside that row because `khatwa` was checked to
        // be at least `tul_saf`. The mapping outlives this function, which runs
        // before the matching unlock.
        let bayt = unsafe {
            let asas = marsum.pBits.cast::<u8>().add(saf.saturating_mul(khatwa));
            core::slice::from_raw_parts(asas, tul_saf)
        };
        match sigha {
            D3DFMT_R5G6B5 => wassi_565(bayt, &mut kharj),
            D3DFMT_X1R5G5B5 => wassi_555(bayt, &mut kharj),
            // Already `B`, `G`, `R`, `A`. The `X` in `X8R8G8B8` is an
            // undefined byte rather than an alpha, and it is passed through as
            // whatever the driver left there: the recognizer converts BGRA by
            // dropping alpha, so a byte nobody defined is a byte nobody reads.
            _ => kharj.extend_from_slice(bayt),
        }
    }
    Ok(kharj)
}

/// Expands one row of `R5G6B5` into `B`, `G`, `R`, `A` bytes.
fn wassi_565(bayt: &[u8], kharj: &mut Vec<u8>) {
    for biksel in bayt.chunks_exact(2) {
        let (Some(adna), Some(aala)) = (biksel.first(), biksel.get(1)) else {
            continue;
        };
        let qeema = u16::from(*adna) | (u16::from(*aala) << 8);
        let qit = |izaha: u32, qina: u16| u8::try_from((qeema >> izaha) & qina).unwrap_or(0);
        let ahmar = khams_ila_thaman(qit(11, 0x1F));
        let akhdar = sitt_ila_thaman(qit(5, 0x3F));
        let azraq = khams_ila_thaman(qit(0, 0x1F));
        kharj.extend_from_slice(&[azraq, akhdar, ahmar, 0xFF]);
    }
}

/// Expands one row of `X1R5G5B5` into `B`, `G`, `R`, `A` bytes.
fn wassi_555(bayt: &[u8], kharj: &mut Vec<u8>) {
    for biksel in bayt.chunks_exact(2) {
        let (Some(adna), Some(aala)) = (biksel.first(), biksel.get(1)) else {
            continue;
        };
        let qeema = u16::from(*adna) | (u16::from(*aala) << 8);
        let qit = |izaha: u32| u8::try_from((qeema >> izaha) & 0x1F).unwrap_or(0);
        let ahmar = khams_ila_thaman(qit(10));
        let akhdar = khams_ila_thaman(qit(5));
        let azraq = khams_ila_thaman(qit(0));
        kharj.extend_from_slice(&[azraq, akhdar, ahmar, 0xFF]);
    }
}

/// Five bits to eight, with the high bits replicated into the low ones.
const fn khams_ila_thaman(qeema: u8) -> u8 {
    (qeema << 3) | (qeema >> 2)
}

/// Six bits to eight, with the high bits replicated into the low ones.
const fn sitt_ila_thaman(qeema: u8) -> u8 {
    (qeema << 2) | (qeema >> 4)
}
