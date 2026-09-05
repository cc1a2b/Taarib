//! خطّاف دايركت٣د ١٠ — the generation between, and the one that shares a hook.
//!
//! Direct3D 10 is the narrowest of the older backends and the one closest in
//! shape to [`crate::d3d11`]. That closeness is the whole design: **the hook is
//! not new**. DXGI owns the swap chain on both generations, `IDXGISwapChain` is
//! the same interface with the same method table, and the `Present` slot the
//! eleventh backend is reached through is the slot a tenth-generation game
//! presents through. So this file installs nothing. It is a [`Khattaf`] built
//! from a swap chain the existing hook already caught, and the only question it
//! answers differently is which device that swap chain hands back.
//!
//! ## What actually differs, and why each of them matters
//!
//! **There is no device context.** `ID3D10Device` is both. Every `*Set*` and
//! `*Get*` in [`crate::d3d11`] that goes through `ID3D11DeviceContext` goes
//! through the device here, which is a rename and nothing more.
//!
//! **The pipeline is three stages, not six.** No hull, no domain, no compute.
//! [`HalatMasar10`] is [`crate::d3d11`]'s saved-state list minus the two
//! tessellation stages and minus the class instances, because Direct3D 10 has
//! no shader interfaces either — `VSSetShader` takes a shader and nothing else.
//! Saving state the API does not have would be four `Get` calls per frame that
//! could only ever return nothing.
//!
//! **The viewport is integers.** `D3D10_VIEWPORT` carries `INT` and `UINT`
//! where `D3D11_VIEWPORT` carries floats. A viewport structure copied from the
//! eleventh backend would be four bit patterns reinterpreted as coordinates,
//! which is a viewport somewhere near zero and an overlay that never appears.
//!
//! **The blend description is one set of factors with eight enables**, where
//! Direct3D 11 has eight independent descriptions. Both spell the same
//! premultiplied blend; only the structure differs.
//!
//! **Mapping is on the resource, not the device.** `ID3D10Buffer::Map` and
//! `ID3D10Texture2D::Map` replace `ID3D11DeviceContext::Map`, and the texture's
//! version returns the mapping by value rather than through an out-parameter.
//!
//! ## What this file deliberately does not carry
//!
//! The shader source, the vertex layout, the quad filler, the projection
//! constants, the DXGI format table and the runtime HLSL compiler are
//! [`crate::d3d11`]'s and are used from there rather than copied — which is the
//! same arrangement [`crate::d3d12`] has with that file, for the same reason.
//! The three DXGI backends draw the same glyphs out of the same atlas onto the
//! same surface at the same shader model, `vs_4_0` and `ps_4_0`, which
//! Direct3D 10 introduced and Direct3D 11 kept. A second copy of the shader
//! would be a second place Arabic could render differently on one generation
//! than another, with nothing to catch it but a player noticing.
//!
//! ## Which backend a DXGI swap chain belongs to
//!
//! A process can carry `d3d10.dll` and `d3d11.dll` at once — a game with a
//! renderer for each, a launcher, an overlay somebody else installed. The
//! module list cannot separate them and this file does not try:
//! [`KhattafD3D10::min_silsila`] asks the swap chain for an `ID3D10Device` and
//! declines when it does not have one, which is exactly the case where
//! [`crate::d3d11`] should have it instead. The refusal is a
//! [`KhataTabaqa::ApiGhayrMadum`] rather than a failure, because nothing went
//! wrong: the game is simply an eleventh-generation game.

use core::ffi::c_void;
use std::mem;

use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D::{
    D3D_PRIMITIVE_TOPOLOGY, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, D3D_SRV_DIMENSION_TEXTURE2D,
};
use windows::Win32::Graphics::Direct3D10::{
    D3D10_BIND_CONSTANT_BUFFER, D3D10_BIND_INDEX_BUFFER, D3D10_BIND_SHADER_RESOURCE,
    D3D10_BIND_VERTEX_BUFFER, D3D10_BLEND_DESC, D3D10_BLEND_INV_SRC_ALPHA, D3D10_BLEND_ONE,
    D3D10_BLEND_OP_ADD, D3D10_BOX, D3D10_BUFFER_DESC, D3D10_COLOR_WRITE_ENABLE_ALL,
    D3D10_COMPARISON_ALWAYS, D3D10_COMPARISON_NEVER, D3D10_CPU_ACCESS_READ, D3D10_CPU_ACCESS_WRITE,
    D3D10_CULL_NONE, D3D10_DEPTH_STENCIL_DESC, D3D10_DEPTH_STENCILOP_DESC,
    D3D10_DEPTH_WRITE_MASK_ZERO, D3D10_FILL_SOLID, D3D10_FILTER_MIN_MAG_MIP_LINEAR,
    D3D10_INPUT_ELEMENT_DESC, D3D10_INPUT_PER_VERTEX_DATA, D3D10_MAP_READ, D3D10_MAP_WRITE_DISCARD,
    D3D10_RASTERIZER_DESC, D3D10_SAMPLER_DESC, D3D10_SHADER_RESOURCE_VIEW_DESC,
    D3D10_SHADER_RESOURCE_VIEW_DESC_0, D3D10_STENCIL_OP_KEEP, D3D10_SUBRESOURCE_DATA,
    D3D10_TEX2D_SRV, D3D10_TEXTURE2D_DESC, D3D10_TEXTURE_ADDRESS_CLAMP, D3D10_USAGE_DEFAULT,
    D3D10_USAGE_DYNAMIC, D3D10_USAGE_IMMUTABLE, D3D10_USAGE_STAGING, D3D10_VIEWPORT,
    ID3D10BlendState, ID3D10Buffer, ID3D10DepthStencilState, ID3D10DepthStencilView, ID3D10Device,
    ID3D10GeometryShader, ID3D10InputLayout, ID3D10PixelShader, ID3D10RasterizerState,
    ID3D10RenderTargetView, ID3D10SamplerState, ID3D10ShaderResourceView, ID3D10Texture2D,
    ID3D10VertexShader,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_FLOAT,
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain;
use windows::core::s;

use crate::d3d11::{
    IZAHAT_KHALT, IZAHAT_KHAREETA, IZAHAT_LAWN, IZAHAT_MAWDI, KHATWAT_RAS, MusarrifHlsl,
    QITA_LIL_DUFA, RUUS_LIL_DUFA, Ras, SAQF_LAWHA, ThawabitIsqat, imla_qita, sigha_min_dxgi,
};
use crate::khata::{KhataTabaqa, tul_u64};
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, SighatSath, WajihatRusum, WasfSath,
};

/// How many render target slots the output merger has.
///
/// `D3D10_SIMULTANEOUS_RENDER_TARGET_COUNT`, spelled as the number the arrays
/// in this file are sized by so a reader can check the two against each other.
const ADAD_AHDAF: usize = 8;

/// How many viewports and scissor rectangles the rasterizer holds.
const ADAD_MANAZIR: usize = 16;

/// [`ADAD_MANAZIR`] as the `u32` the rasterizer getters take.
const ADAD_MANAZIR_U32: u32 = 16;

// The two spellings must agree or the getters are told a capacity the arrays do
// not have, which the runtime would write past.
const _: () = {
    assert!(ADAD_MANAZIR == ADAD_MANAZIR_U32 as usize, "viewport counts disagree");
};

/// Everything the overlay is about to overwrite on the device.
///
/// The list is exactly what [`KhattafD3D10::irsim`] binds, at exactly the slots
/// it binds them, and nothing else — the same reading of "complete" that
/// [`crate::d3d11`] makes. What is absent relative to that file is absent
/// because Direct3D 10 does not have it: there are no hull or domain stages and
/// there are no class instances, so there is nothing to save for either.
///
/// Every interface the `*Get*` calls write out carries a reference this struct
/// now owns, and the release is this struct's `Drop`. `mem_forget` is denied
/// workspace-wide, so there is no edit that keeps those references alive by
/// accident — which is the leak that makes a game run fine for ten minutes and
/// then not.
#[derive(Debug)]
struct HalatMasar10 {
    takhtit: Option<ID3D10InputLayout>,
    tobologia: D3D_PRIMITIVE_TOPOLOGY,
    ruus: Option<ID3D10Buffer>,
    khatwa: u32,
    izaha: u32,
    faharis: Option<ID3D10Buffer>,
    sighat_faharis: DXGI_FORMAT,
    izahat_faharis: u32,
    thawabit_ras: Option<ID3D10Buffer>,
    thawabit_biksel: Option<ID3D10Buffer>,
    mawrid_biksel: Option<ID3D10ShaderResourceView>,
    akhidh_biksel: Option<ID3D10SamplerState>,
    shader_ras: Option<ID3D10VertexShader>,
    shader_biksel: Option<ID3D10PixelShader>,
    shader_handasi: Option<ID3D10GeometryShader>,
    khalt: Option<ID3D10BlendState>,
    amil_khalt: [f32; 4],
    qina_ayyina: u32,
    umq: Option<ID3D10DepthStencilState>,
    marji_tazlil: u32,
    ahdaf: [Option<ID3D10RenderTargetView>; ADAD_AHDAF],
    ru2yat_umq: Option<ID3D10DepthStencilView>,
    munaqqit: Option<ID3D10RasterizerState>,
    manazir: [D3D10_VIEWPORT; ADAD_MANAZIR],
    adad_manazir: u32,
    maqassat: [RECT; ADAD_MANAZIR],
    adad_maqassat: u32,
}

impl HalatMasar10 {
    /// Reads the pipeline as the game left it.
    ///
    /// Every out-parameter starts as `None` or as a zeroed value, which matters:
    /// the runtime writes a raw pointer straight into the slot, and a slot that
    /// already held an interface would have leaked it.
    fn iltaqit(jihaz: &ID3D10Device) -> Self {
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
            shader_biksel: None,
            shader_handasi: None,
            khalt: None,
            amil_khalt: [0.0; 4],
            qina_ayyina: 0,
            umq: None,
            marji_tazlil: 0,
            ahdaf: core::array::from_fn(|_| None),
            ru2yat_umq: None,
            munaqqit: None,
            manazir: [D3D10_VIEWPORT::default(); ADAD_MANAZIR],
            adad_manazir: 0,
            maqassat: [RECT::default(); ADAD_MANAZIR],
            adad_maqassat: 0,
        };

        // SAFETY: `jihaz` is a live device. Every pointer below addresses a
        // field of `hala`, which is a local that outlives the block, and every
        // slice is a real array of the length the runtime is told. The counts
        // handed to `RSGetViewports` and `RSGetScissorRects` are the array
        // capacities, which is what those two read on the way in.
        unsafe {
            hala.takhtit = jihaz.IAGetInputLayout().ok();
            hala.tobologia = jihaz.IAGetPrimitiveTopology();
            jihaz.IAGetVertexBuffers(
                0,
                1,
                Some(&raw mut hala.ruus),
                Some(&raw mut hala.khatwa),
                Some(&raw mut hala.izaha),
            );
            jihaz.IAGetIndexBuffer(
                Some(&raw mut hala.faharis),
                Some(&raw mut hala.sighat_faharis),
                Some(&raw mut hala.izahat_faharis),
            );

            // Moved out rather than cloned: the interface the runtime wrote
            // into the array already carries the reference this struct is
            // taking over, and cloning it would add a second one for nothing.
            let mut wahid = [None];
            jihaz.VSGetConstantBuffers(0, Some(&mut wahid));
            hala.thawabit_ras = wahid.into_iter().next().flatten();
            let mut wahid = [None];
            jihaz.PSGetConstantBuffers(0, Some(&mut wahid));
            hala.thawabit_biksel = wahid.into_iter().next().flatten();

            let mut mawarid = [None];
            jihaz.PSGetShaderResources(0, Some(&mut mawarid));
            hala.mawrid_biksel = mawarid.into_iter().next().flatten();
            let mut akhidhat = [None];
            jihaz.PSGetSamplers(0, Some(&mut akhidhat));
            hala.akhidh_biksel = akhidhat.into_iter().next().flatten();

            hala.shader_ras = jihaz.VSGetShader().ok();
            hala.shader_biksel = jihaz.PSGetShader().ok();
            hala.shader_handasi = jihaz.GSGetShader().ok();

            jihaz.OMGetBlendState(
                Some(&raw mut hala.khalt),
                Some(&mut hala.amil_khalt),
                Some(&raw mut hala.qina_ayyina),
            );
            jihaz.OMGetDepthStencilState(
                Some(&raw mut hala.umq),
                Some(&raw mut hala.marji_tazlil),
            );
            jihaz.OMGetRenderTargets(Some(&mut hala.ahdaf), Some(&raw mut hala.ru2yat_umq));

            hala.munaqqit = jihaz.RSGetState().ok();
            hala.adad_manazir = ADAD_MANAZIR_U32;
            jihaz.RSGetViewports(&raw mut hala.adad_manazir, Some(hala.manazir.as_mut_ptr()));
            hala.adad_maqassat = ADAD_MANAZIR_U32;
            jihaz.RSGetScissorRects(&raw mut hala.adad_maqassat, Some(hala.maqassat.as_mut_ptr()));
        }

        hala
    }

    /// Writes the pipeline back exactly as it was read.
    ///
    /// Every one of these setters returns nothing and cannot fail — Direct3D 10
    /// state changes are recorded, not validated. That is why the check that
    /// the restore *worked* is not here but in [`KhattafD3D10::irsim`], where
    /// the device is asked whether it was removed while all of this was
    /// happening. A removed device is the one condition under which none of
    /// these calls did anything, and it is the only detectable restore failure
    /// this API has.
    fn istaid(&self, jihaz: &ID3D10Device) {
        // SAFETY: `jihaz` is the same live device the state was read from.
        // Every interface passed back is one this struct owns, every slice is
        // borrowed from a field that outlives the call, and the viewport and
        // scissor counts are clamped to the arrays they index.
        unsafe {
            jihaz.IASetInputLayout(self.takhtit.as_ref());
            jihaz.IASetPrimitiveTopology(self.tobologia);
            jihaz.IASetVertexBuffers(
                0,
                1,
                Some(&raw const self.ruus),
                Some(&raw const self.khatwa),
                Some(&raw const self.izaha),
            );
            jihaz.IASetIndexBuffer(
                self.faharis.as_ref(),
                self.sighat_faharis,
                self.izahat_faharis,
            );

            // `from_ref` rather than a one-element array of clones: the
            // setters borrow the slice for the duration of the call and take
            // their own references, so cloning four interfaces per frame buys
            // eight reference-count operations and nothing else.
            jihaz.VSSetConstantBuffers(0, Some(core::slice::from_ref(&self.thawabit_ras)));
            jihaz.PSSetConstantBuffers(0, Some(core::slice::from_ref(&self.thawabit_biksel)));
            jihaz.PSSetShaderResources(0, Some(core::slice::from_ref(&self.mawrid_biksel)));
            jihaz.PSSetSamplers(0, Some(core::slice::from_ref(&self.akhidh_biksel)));

            jihaz.VSSetShader(self.shader_ras.as_ref());
            jihaz.PSSetShader(self.shader_biksel.as_ref());
            jihaz.GSSetShader(self.shader_handasi.as_ref());

            jihaz.OMSetBlendState(self.khalt.as_ref(), &self.amil_khalt, self.qina_ayyina);
            jihaz.OMSetDepthStencilState(self.umq.as_ref(), self.marji_tazlil);
            jihaz.OMSetRenderTargets(Some(&self.ahdaf), self.ru2yat_umq.as_ref());

            jihaz.RSSetState(self.munaqqit.as_ref());
            let adad = usize::try_from(self.adad_manazir).unwrap_or(0).min(ADAD_MANAZIR);
            jihaz.RSSetViewports(self.manazir.get(..adad));
            let adad = usize::try_from(self.adad_maqassat).unwrap_or(0).min(ADAD_MANAZIR);
            jihaz.RSSetScissorRects(self.maqassat.get(..adad));
        }
    }
}

/// The Direct3D 10 overlay backend.
///
/// Holds the game's device and the swap chain it was found through, and every
/// object the overlay draws with. The surface-sized ones are rebuilt by
/// [`Khattaf::hayyi`]; the atlas deliberately is not, because losing the glyphs
/// on an alt-tab would leave the overlay with nothing to draw and no way to ask
/// for it back until the font changed.
#[derive(Debug)]
pub struct KhattafD3D10 {
    jihaz: ID3D10Device,
    silsila: IDXGISwapChain,
    musarrif: MusarrifHlsl,
    bayt_ras: Vec<u8>,
    bayt_biksel: Vec<u8>,
    shader_ras: Option<ID3D10VertexShader>,
    shader_biksel: Option<ID3D10PixelShader>,
    takhtit: Option<ID3D10InputLayout>,
    ruus: Option<ID3D10Buffer>,
    faharis: Option<ID3D10Buffer>,
    thawabit: Option<ID3D10Buffer>,
    khalt: Option<ID3D10BlendState>,
    munaqqit: Option<ID3D10RasterizerState>,
    umq: Option<ID3D10DepthStencilState>,
    akhidh: Option<ID3D10SamplerState>,
    lawha: Option<ID3D10Texture2D>,
    ru2yat_lawha: Option<ID3D10ShaderResourceView>,
    hadaf: Option<ID3D10RenderTargetView>,
    sath: Option<WasfSath>,
    musawwada: Vec<Ras>,
}

impl KhattafD3D10 {
    /// Builds a backend around a swap chain the existing DXGI hook found.
    ///
    /// Everything here is read from the swap chain rather than created: the
    /// device is the game's and the overlay never makes a second one.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::ApiGhayrMadum`] when the swap chain's device is not a
    /// Direct3D 10 one — which is not a failure and is the ordinary case for an
    /// eleventh- or twelfth-generation game, and is what tells the caller to
    /// try the other backend. [`KhataTabaqa::SathTaghayyar`] when the swap
    /// chain will not describe itself, and [`KhataTabaqa::MaktabaMafquda`] when
    /// the shader compiler is not on this machine.
    pub fn min_silsila(silsila: &IDXGISwapChain) -> Result<Self, KhataTabaqa> {
        // SAFETY: `silsila` is a live swap chain the caller obtained from the
        // process it is hooking. `GetDevice` performs a QueryInterface for the
        // requested IID and returns an owned reference or an error; nothing is
        // written through a raw pointer.
        let jihaz = unsafe { silsila.GetDevice::<ID3D10Device>() }.map_err(|khata| {
            KhataTabaqa::ApiGhayrMadum {
                api: format!(
                    "this swap chain's device is not an ID3D10Device ({khata}), so this game \
                     belongs to another Direct3D generation's backend"
                ),
            }
        })?;

        // SAFETY: `silsila` is live and `GetDesc` fills a caller-owned
        // description the binding allocates on the stack.
        unsafe { silsila.GetDesc() }.map_err(|khata| KhataTabaqa::SathTaghayyar {
            sabab: format!("the swap chain would not describe itself: {khata}"),
        })?;

        let musarrif = MusarrifHlsl::ihdar()?;

        Ok(Self {
            jihaz,
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
    /// `IDXGISwapChain::ResizeBuffers` fails with `DXGI_ERROR_INVALID_CALL`
    /// while *anybody* holds a reference to a backbuffer, and this backend
    /// holds one for the whole session in its render target view. The reasoning
    /// and the recovery are [`crate::d3d11::KhattafD3D11::atliq_khalfiyat`]'s
    /// exactly — including the part that is easy to omit: the surface is
    /// forgotten here, so a resize that keeps the same width, height and format
    /// still rebuilds, because [`crate::wajiha::Tabaqa`] cannot see that kind.
    pub fn atliq_khalfiyat(&mut self) {
        self.hadaf = None;
        self.sath = None;
    }

    /// Drops every object [`Khattaf::hayyi`] creates, keeping the atlas.
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
    /// `vs_4_0` and `ps_4_0` — the profiles Direct3D 10 introduced, which
    /// Direct3D 11 kept and which [`crate::d3d11`] therefore also compiles.
    /// Cached in `bayt_ras` and `bayt_biksel` because `hayyi` runs on every
    /// resolution change and `D3DCompile` is milliseconds of work on the render
    /// thread.
    fn ihdar_ramz(&mut self) -> Result<(), KhataTabaqa> {
        if self.bayt_ras.is_empty() {
            self.bayt_ras = self.musarrif.sarrif("overlay vertex shader", s!("ras"), s!("vs_4_0"))?;
        }
        if self.bayt_biksel.is_empty() {
            self.bayt_biksel =
                self.musarrif.sarrif("overlay pixel shader", s!("biksel"), s!("ps_4_0"))?;
        }
        Ok(())
    }

    /// Creates the shaders, the input layout and the three buffers.
    fn ibni_masar(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        self.ihdar_ramz()?;

        let mut shader_ras = None;
        // SAFETY: `bayt_ras` holds DXBC the compiler produced from the shared
        // HLSL, and the out-pointer addresses a local initialised to `None`, so
        // the reference the runtime writes is owned by that local.
        unsafe { self.jihaz.CreateVertexShader(&self.bayt_ras, Some(&raw mut shader_ras)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay vertex shader",
                sabab: khata.to_string(),
            })?;

        let mut shader_biksel = None;
        // SAFETY: as above, for the pixel stage.
        unsafe { self.jihaz.CreatePixelShader(&self.bayt_biksel, Some(&raw mut shader_biksel)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay pixel shader",
                sabab: khata.to_string(),
            })?;

        // The offsets come from `crate::d3d11`, where they are checked against
        // the vertex type by `const` assertion. Spelling them again here would
        // be a second place they could drift from the struct the GPU reads.
        let anasir = [
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_MAWDI,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHAREETA,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("COLOR"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_LAWN,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D10_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 1,
                Format: DXGI_FORMAT_R32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHALT,
                InputSlotClass: D3D10_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];

        let mut takhtit = None;
        // SAFETY: the element array and the vertex bytecode are both live for
        // the call, and the out-pointer addresses a local initialised to
        // `None`. The runtime validates the layout against the shader's input
        // signature and fails rather than accepting a mismatch.
        unsafe { self.jihaz.CreateInputLayout(&anasir, &self.bayt_ras, Some(&raw mut takhtit)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay input layout",
                sabab: khata.to_string(),
            })?;

        let wasf_ruus = D3D10_BUFFER_DESC {
            ByteWidth: KHATWAT_RAS.saturating_mul(RUUS_LIL_DUFA),
            Usage: D3D10_USAGE_DYNAMIC,
            BindFlags: D3D10_BIND_VERTEX_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: D3D10_CPU_ACCESS_WRITE.0.cast_unsigned(),
            MiscFlags: 0,
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
        // built once and never touched again.
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
        let wasf_faharis = D3D10_BUFFER_DESC {
            ByteWidth: u32::try_from(faharis_khaam.len() * size_of::<u16>()).unwrap_or(0),
            Usage: D3D10_USAGE_IMMUTABLE,
            BindFlags: D3D10_BIND_INDEX_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let bidaya_faharis = D3D10_SUBRESOURCE_DATA {
            pSysMem: faharis_khaam.as_ptr().cast::<c_void>(),
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut faharis = None;
        // SAFETY: `faharis_khaam` outlives the call, so the initial-data
        // pointer is valid for the `ByteWidth` bytes the description names —
        // the two are computed from the same vector.
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
        let wasf_thawabit = D3D10_BUFFER_DESC {
            ByteWidth: u32::try_from(size_of::<ThawabitIsqat>()).unwrap_or(0),
            Usage: D3D10_USAGE_DEFAULT,
            BindFlags: D3D10_BIND_CONSTANT_BUFFER.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let bidaya_thawabit = D3D10_SUBRESOURCE_DATA {
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
    /// The one structural difference from [`crate::d3d11`] is here:
    /// `D3D10_BLEND_DESC` carries **one** set of blend factors with a per-target
    /// enable array, where `D3D11_BLEND_DESC` carries eight independent
    /// descriptions. The blend the two spell is identical.
    fn ibni_halat(&mut self) -> Result<(), KhataTabaqa> {
        // Premultiplied alpha. `SRC_ALPHA, INV_SRC_ALPHA` would be the reflex
        // and it is wrong here: `saff` hands over coverage already multiplied
        // into the colour, and multiplying it a second time makes every glyph
        // edge darker than its interior — a halo that reads as a bad font.
        let wasf_khalt = D3D10_BLEND_DESC {
            AlphaToCoverageEnable: false.into(),
            BlendEnable: [true.into(); ADAD_AHDAF],
            SrcBlend: D3D10_BLEND_ONE,
            DestBlend: D3D10_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D10_BLEND_OP_ADD,
            SrcBlendAlpha: D3D10_BLEND_ONE,
            DestBlendAlpha: D3D10_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D10_BLEND_OP_ADD,
            RenderTargetWriteMask: [u8::try_from(D3D10_COLOR_WRITE_ENABLE_ALL.0).unwrap_or(0x0F);
                ADAD_AHDAF],
        };
        let mut khalt = None;
        // SAFETY: the description is a fully initialised local and the
        // out-pointer addresses a local initialised to `None`.
        unsafe { self.jihaz.CreateBlendState(&raw const wasf_khalt, Some(&raw mut khalt)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay blend state",
                sabab: khata.to_string(),
            })?;

        // Culling off because the overlay's quads are generated in one winding
        // and a game that left `FrontCounterClockwise` set would otherwise cull
        // every one of them, which looks exactly like the overlay never ran.
        let wasf_munaqqit = D3D10_RASTERIZER_DESC {
            FillMode: D3D10_FILL_SOLID,
            CullMode: D3D10_CULL_NONE,
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
        let tazlil = D3D10_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D10_STENCIL_OP_KEEP,
            StencilDepthFailOp: D3D10_STENCIL_OP_KEEP,
            StencilPassOp: D3D10_STENCIL_OP_KEEP,
            StencilFunc: D3D10_COMPARISON_ALWAYS,
        };
        let wasf_umq = D3D10_DEPTH_STENCIL_DESC {
            DepthEnable: false.into(),
            DepthWriteMask: D3D10_DEPTH_WRITE_MASK_ZERO,
            DepthFunc: D3D10_COMPARISON_ALWAYS,
            StencilEnable: false.into(),
            StencilReadMask: 0,
            StencilWriteMask: 0,
            FrontFace: tazlil,
            BackFace: tazlil,
        };
        let mut umq = None;
        // SAFETY: as above.
        unsafe { self.jihaz.CreateDepthStencilState(&raw const wasf_umq, Some(&raw mut umq)) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay depth-stencil state",
                sabab: khata.to_string(),
            })?;

        // Clamped addressing so a glyph whose atlas rectangle sits against the
        // edge cannot bleed a neighbouring glyph's pixels into itself.
        let wasf_akhidh = D3D10_SAMPLER_DESC {
            Filter: D3D10_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D10_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D10_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D10_TEXTURE_ADDRESS_CLAMP,
            MipLODBias: 0.0,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D10_COMPARISON_NEVER,
            BorderColor: [0.0; 4],
            MinLOD: 0.0,
            MaxLOD: 0.0,
        };
        let mut akhidh = None;
        // SAFETY: as above.
        unsafe { self.jihaz.CreateSamplerState(&raw const wasf_akhidh, Some(&raw mut akhidh)) }
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
    /// place a resize to a different one silently kept drawing through a view
    /// that no longer matches.
    fn ibni_hadaf(&mut self) -> Result<(), KhataTabaqa> {
        // SAFETY: `silsila` is live. `GetBuffer` performs a QueryInterface on
        // the named buffer and returns an owned reference or an error.
        let khalfiya = unsafe { self.silsila.GetBuffer::<ID3D10Texture2D>(0) }.map_err(
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
        // rectangles of whatever t0 happens to contain, which on some drivers
        // is the previous frame. Refusing is the only honest answer.
        if self.ru2yat_lawha.is_none() && lawha.qitaat.iter().any(|qita| qita.khareeta.is_some()) {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the batch samples an atlas that has not been uploaded".to_owned(),
            });
        }

        // Integers, not floats. This is the one structure a reader coming from
        // `crate::d3d11` will copy without looking, and the result is a
        // viewport built from four bit patterns near zero.
        let manzar = D3D10_VIEWPORT {
            TopLeftX: 0,
            TopLeftY: 0,
            Width: lawha.sath.ard,
            Height: lawha.sath.irtifa,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        let ruus_marbut = [Some(ruus.clone())];
        let khatwa = KHATWAT_RAS;
        let izaha = 0_u32;
        let amil_khalt = [0.0_f32; 4];

        // SAFETY: every interface bound below is one this backend owns and
        // holds alive for the whole function, every slice is a live local, and
        // the two raw pointers handed to `IASetVertexBuffers` address locals
        // declared immediately above. Binding cannot fail; it is recorded, not
        // validated.
        unsafe {
            self.jihaz.IASetInputLayout(Some(takhtit));
            self.jihaz.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            self.jihaz.IASetVertexBuffers(
                0,
                1,
                Some(ruus_marbut.as_ptr()),
                Some(&raw const khatwa),
                Some(&raw const izaha),
            );
            self.jihaz.IASetIndexBuffer(Some(faharis), DXGI_FORMAT_R16_UINT, 0);

            self.jihaz.VSSetShader(Some(shader_ras));
            self.jihaz.VSSetConstantBuffers(0, Some(&[Some(thawabit.clone())]));
            self.jihaz.PSSetShader(Some(shader_biksel));
            self.jihaz.PSSetConstantBuffers(0, Some(&[Some(thawabit.clone())]));
            self.jihaz.PSSetShaderResources(0, Some(core::slice::from_ref(&self.ru2yat_lawha)));
            self.jihaz.PSSetSamplers(0, Some(&[Some(akhidh.clone())]));

            // The one stage the overlay does not use is cleared rather than
            // left alone. A game with a geometry shader still bound would run
            // it over the overlay's triangles, and whatever it emitted would
            // not be Arabic.
            self.jihaz.GSSetShader(None);

            self.jihaz.OMSetBlendState(Some(khalt), &amil_khalt, u32::MAX);
            self.jihaz.OMSetDepthStencilState(Some(umq), 0);
            self.jihaz.OMSetRenderTargets(Some(&[Some(hadaf.clone())]), None);

            self.jihaz.RSSetState(Some(munaqqit));
            self.jihaz.RSSetViewports(Some(&[manzar]));
            // Cleared, not left: the rasterizer state above disables
            // scissoring, but a driver that honours a stale rectangle anyway
            // would clip the overlay to wherever the game last drew a minimap.
            self.jihaz.RSSetScissorRects(Some(&[]));
        }

        for dufa in lawha.qitaat.chunks(QITA_LIL_DUFA) {
            musawwada.clear();
            for qita in dufa {
                imla_qita(musawwada, qita);
            }
            if musawwada.is_empty() {
                continue;
            }

            let mut makan: *mut c_void = core::ptr::null_mut();
            // SAFETY: `ruus` is the dynamic vertex buffer this backend created
            // with CPU write access, `WRITE_DISCARD` is the documented map type
            // for it, and the out-pointer addresses a live local.
            unsafe { ruus.Map(D3D10_MAP_WRITE_DISCARD, 0, &raw mut makan) }.map_err(|khata| {
                KhataTabaqa::MawridFashil {
                    mawrid: "overlay vertex buffer",
                    sabab: format!("the buffer could not be mapped for writing: {khata}"),
                }
            })?;

            if makan.is_null() {
                // SAFETY: the map above succeeded, so the buffer is mapped and
                // must be unmapped before returning even though the pointer is
                // unusable.
                unsafe { ruus.Unmap() };
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "overlay vertex buffer",
                    sabab: "the map succeeded and returned no pointer".to_owned(),
                });
            }

            // SAFETY: the buffer was created with room for `RUUS_LIL_DUFA`
            // vertices and `musawwada` holds at most four per quad in a chunk
            // of at most `QITA_LIL_DUFA` quads, so the copy is inside the
            // mapping. Source and destination are distinct allocations.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    musawwada.as_ptr(),
                    makan.cast::<Ras>(),
                    musawwada.len(),
                );
                ruus.Unmap();
            }

            // Six indices per quad, and a chunk is at most `QITA_LIL_DUFA`
            // quads, so this cannot overflow a `u32`.
            let adad = u32::try_from(dufa.len().saturating_mul(6)).unwrap_or(0);
            // SAFETY: the pipeline is bound above, the index buffer holds
            // exactly `QITA_LIL_DUFA * 6` indices, and `adad` is bounded by
            // that. The vertices those indices reach were written immediately
            // above.
            unsafe { self.jihaz.DrawIndexed(adad, 0, 0) };
        }

        Ok(())
    }
}

impl Khattaf for KhattafD3D10 {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Direct3D10
    }

    /// Drops the render target view, which is the only backbuffer reference
    /// this backend holds.
    ///
    /// Infallible, as on Direct3D 11: a view is released by dropping it and the
    /// device has no outstanding submission to wait on that the runtime does
    /// not already track. The trait's signature carries a `Result` because
    /// Direct3D 12 genuinely needs one.
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
        // allocation would be holding a view of a backbuffer DXGI has freed.
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

        let wasf = D3D10_TEXTURE2D_DESC {
            Width: ard,
            Height: irtifa,
            MipLevels: 1,
            ArraySize: 1,
            // The atlas is premultiplied linear coverage, not colour anybody
            // encoded, so it is uploaded as plain `UNORM` and the shader is
            // left to do the one transfer this backend does at all.
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            // Immutable rather than default: the atlas is replaced wholesale
            // when it changes and never partially written.
            Usage: D3D10_USAGE_IMMUTABLE,
            BindFlags: D3D10_BIND_SHADER_RESOURCE.0.cast_unsigned(),
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let bidaya = D3D10_SUBRESOURCE_DATA {
            pSysMem: bayt.as_ptr().cast::<c_void>(),
            SysMemPitch: khatwa,
            SysMemSlicePitch: 0,
        };

        // SAFETY: `bayt` outlives the call and was checked above to hold at
        // least `khatwa * irtifa` bytes, which is exactly what the row pitch
        // and the height in the description tell the runtime to read.
        let lawha = unsafe {
            self.jihaz.CreateTexture2D(&raw const wasf, Some(&raw const bidaya))
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: khata.to_string(),
        })?;

        let wasf_ru2ya = D3D10_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D10_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D10_TEX2D_SRV { MostDetailedMip: 0, MipLevels: 1 },
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
            // nothing it can see — so the rebuild is done here.
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

        // Read before anything is bound. The device is cloned rather than
        // borrowed so the draw below can take `&self` without the borrow
        // checker and the restore fighting over the same field.
        let jihaz = self.jihaz.clone();
        let hala = HalatMasar10::iltaqit(&jihaz);

        let mut musawwada = mem::take(&mut self.musawwada);
        let natija = self.arsim_dufaat(&mut musawwada, lawha);
        musawwada.clear();
        self.musawwada = musawwada;

        // Unconditional, and before the result is inspected. There is no error
        // this backend can return that makes leaving the game's pipeline
        // pointed at Taarib's buffers the better outcome.
        hala.istaid(&jihaz);
        drop(hala);

        // Every Direct3D 10 state setter returns nothing and cannot refuse, so
        // the restore above either happened or the device stopped existing part
        // way through it. That second case is the whole of what
        // `HalaGhayrMustaada` means here, and it is checked rather than assumed
        // because a removed device is exactly what a driver reset during an
        // overlay draw produces.
        //
        // SAFETY: `jihaz` is the live device this backend was built around.
        if let Err(khata) = unsafe { self.jihaz.GetDeviceRemovedReason() } {
            return Err(KhataTabaqa::HalaGhayrMustaada {
                hala: "the Direct3D 10 pipeline",
                sabab: format!("the device was removed while the overlay drew: {khata}"),
            });
        }

        natija
    }

    fn iltaqit(&mut self, mintaqa: MustatilBiksel) -> Result<Vec<u8>, KhataTabaqa> {
        let sath = Khattaf::sath(self)?;
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
        let khalfiya = unsafe { self.silsila.GetBuffer::<ID3D10Texture2D>(0) }.map_err(
            |khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("the backbuffer could not be fetched: {khata}"),
            },
        )?;

        let mut wasf_khalfiya = D3D10_TEXTURE2D_DESC::default();
        // SAFETY: `khalfiya` is a live texture and the out-pointer addresses a
        // fully initialised local of exactly the type the runtime writes.
        unsafe { khalfiya.GetDesc(&raw mut wasf_khalfiya) };

        // A multisampled backbuffer cannot be the source of a copy into a
        // single-sampled staging texture, and the copy fails silently on some
        // drivers rather than returning an error. Resolving first is not an
        // optimisation; it is the only way this path works at all on a game
        // that presents MSAA directly.
        let masdar = if wasf_khalfiya.SampleDesc.Count > 1 {
            let wasf_hall = D3D10_TEXTURE2D_DESC {
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D10_USAGE_DEFAULT,
                // A default-usage texture with no bind flags at all is rejected
                // by the runtime, so the resolve target carries the one flag
                // every swap chain format is guaranteed to support. Nothing
                // samples it; it exists for one `CopySubresourceRegion`.
                BindFlags: D3D10_BIND_SHADER_RESOURCE.0.cast_unsigned(),
                CPUAccessFlags: 0,
                MiscFlags: 0,
                MipLevels: 1,
                ArraySize: 1,
                ..wasf_khalfiya
            };
            // SAFETY: the description is a live local and there is no initial
            // data for a render target.
            let hall = unsafe { self.jihaz.CreateTexture2D(&raw const wasf_hall, None) }
                .map_err(|khata| KhataTabaqa::IltiqatFashil {
                    sabab: format!("the multisample resolve target could not be created: {khata}"),
                })?;
            // SAFETY: both textures are live, both have one subresource at
            // index zero, and the format is the backbuffer's own on both sides,
            // which is what `ResolveSubresource` requires.
            unsafe {
                self.jihaz.ResolveSubresource(&hall, 0, &khalfiya, 0, wasf_khalfiya.Format);
            }
            hall
        } else {
            khalfiya
        };

        let wasf_marhala = D3D10_TEXTURE2D_DESC {
            Width: mintaqa.ard,
            Height: mintaqa.irtifa,
            MipLevels: 1,
            ArraySize: 1,
            Format: wasf_khalfiya.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D10_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D10_CPU_ACCESS_READ.0.cast_unsigned(),
            MiscFlags: 0,
        };
        // SAFETY: the description is a live local and a staging texture takes
        // no initial data.
        let marhala = unsafe { self.jihaz.CreateTexture2D(&raw const wasf_marhala, None) }
            .map_err(|khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("the read-back texture could not be created: {khata}"),
            })?;

        let sunduq = D3D10_BOX {
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
            self.jihaz.CopySubresourceRegion(
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

        // SAFETY: `marhala` is the staging texture created above with CPU read
        // access, which is the one usage `D3D10_MAP_READ` accepts. Direct3D 10
        // maps on the resource rather than on a context and returns the mapping
        // by value.
        let marsum = unsafe { marhala.Map(0, D3D10_MAP_READ, 0) }.map_err(|khata| {
            KhataTabaqa::IltiqatFashil {
                sabab: format!("the read-back texture could not be mapped: {khata}"),
            }
        })?;

        let natija = jami_sufuf10(marsum.pData, marsum.RowPitch, mintaqa, sath.sigha);

        // SAFETY: the map above succeeded, so subresource zero is mapped and
        // must be unmapped exactly once. This runs before the result is
        // inspected so that an error on the copy path does not leave it mapped.
        unsafe { marhala.Unmap(0) };

        natija
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        // Idempotent by construction: every field is an `Option` and assigning
        // `None` twice is assigning `None`. The second call is the common one —
        // `Tabaqa::shaghghil` tears a backend down when `hayyi` failed, and
        // then the caller drops it, which runs this again.
        self.atlif_masar();
        self.lawha = None;
        self.ru2yat_lawha = None;
        self.bayt_ras = Vec::new();
        self.bayt_biksel = Vec::new();
        self.musawwada = Vec::new();
        Ok(())
    }
}

/// Copies a mapped region out row by row, dropping the pitch.
///
/// `RowPitch` is the distance between the starts of two rows in the mapping,
/// which the driver rounds up to whatever alignment it prefers. It is *not* the
/// row's width, and a read that treats it as one produces an image that shears
/// progressively to one side — which looks like a capture bug in the recogniser
/// rather than a stride bug here. [`Khattaf::iltaqit`] promises tightly packed
/// bytes, so the repacking happens once, here.
///
/// The same function exists in [`crate::d3d11`] over that API's own mapping
/// type. It is written again rather than shared because the two mapping
/// structures are different types with the same two fields, and a shared
/// version would take the pointer and the pitch as loose arguments — which is
/// exactly the shape that lets a caller pass the height where the pitch belongs.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the mapping has no pointer or its pitch
/// is shorter than one row of the region, both of which mean the copy did not
/// produce what was asked for.
fn jami_sufuf10(
    bayt: *mut c_void,
    khatwa: u32,
    mintaqa: MustatilBiksel,
    sigha: SighatSath,
) -> Result<Vec<u8>, KhataTabaqa> {
    if bayt.is_null() {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: "the read-back mapping has no pointer".to_owned(),
        });
    }

    let tul_saf = mintaqa.ard.saturating_mul(sigha.bayt_lil_biksel());
    let tul_saf = usize::try_from(tul_saf).unwrap_or(usize::MAX);
    let khatwa = usize::try_from(khatwa).unwrap_or(usize::MAX);
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
        // SAFETY: the mapping covers `khatwa * height` bytes for the staging
        // texture that was created at exactly this region's dimensions, so a
        // row start at `saf * khatwa` with `saf` below the height is inside it,
        // and `tul_saf` bytes from there are inside that row because `khatwa`
        // was checked to be at least `tul_saf`. The mapping outlives this
        // function, which runs before `Unmap`.
        let shariha = unsafe {
            let asas = bayt.cast::<u8>().add(saf.saturating_mul(khatwa));
            core::slice::from_raw_parts(asas, tul_saf)
        };
        khuruj.extend_from_slice(shariha);
    }
    Ok(khuruj)
}

// ---------------------------------------------------------------------------
// What can be decided before the overlay is enabled
// ---------------------------------------------------------------------------

/// What the overlay will be able to do on Direct3D 10 in this process.
///
/// Three things are established, and the third is the one that matters:
///
/// 1. whether a Direct3D 10 module is loaded at all;
/// 2. whether `d3d11.dll` is loaded beside it, which is the ordinary shape of a
///    game with two renderers and means this backend may not be the one that
///    ends up drawing;
/// 3. that composition happens through the game's own device, so exclusive
///    fullscreen is composed over exactly as windowed mode is — which is the
///    question anybody actually asks about an overlay, and which has the same
///    answer on every API in this crate for the same reason.
///
/// No throwaway device is created. Unlike Direct3D 8, there is nothing to
/// verify: the method table is the `windows` crate's generated binding and the
/// hook is [`crate::d3d11`]'s, already proven against the same interface. A
/// device made here would cost a driver initialisation inside somebody's game
/// to learn nothing.
#[must_use]
pub fn qudra() -> crate::qudra::QudratTarkeeb {
    use crate::qudra::{MilShasha, QudratTarkeeb, SababQudra};
    use taarib_haqn::mawqi::qaidat_wahda;

    let mut taqreer =
        QudratTarkeeb::jadeeda(WajihatRusum::Direct3D10, MilShasha::KhilalAlJihaz);

    let ashira = qaidat_wahda("d3d10.dll").is_some();
    let ashira_wahid = qaidat_wahda("d3d10_1.dll").is_some();
    if !ashira && !ashira_wahid {
        return taqreer.maa(SababQudra::mustaheela(
            "لا توجد مكتبة دايركت٣د ١٠ محمّلة في هذه اللعبة.",
            "neither d3d10.dll nor d3d10_1.dll is loaded in this process",
        ));
    }

    taqreer = taqreer.maa(SababQudra::kamila(
        if ashira_wahid {
            "تستخدم اللعبة دايركت٣د ١٠٫١، وجهازه يجيب على طلب واجهة الجيل العاشر لأنّها أصله."
        } else {
            "تستخدم اللعبة دايركت٣د ١٠."
        },
        if ashira_wahid {
            "this process carries d3d10_1.dll, whose device answers a QueryInterface for \
             ID3D10Device because ID3D10Device1 derives from it"
        } else {
            "this process carries d3d10.dll"
        },
    ));

    if qaidat_wahda("d3d11.dll").is_some() || qaidat_wahda("d3d12.dll").is_some() {
        taqreer = taqreer.maa(SababQudra::kamila(
            "توجد مكتبة جيل أحدث محمّلة كذلك. تسأل الطبقة سلسلة العرض عن جهازها عند أوّل إطار، \
             فإن لم يكن جهاز الجيل العاشر تولّت الواجهة الأخرى الرسم — والخطّاف واحد في \
             الحالتين.",
            "a newer Direct3D module is loaded as well; the overlay asks the swap chain for its \
             device on the first frame and hands the game to the other backend when it is not a \
             Direct3D 10 one — the hook is the same either way",
        ));
    }

    taqreer.maa(SababQudra::kamila(
        "يشترك هذا الجيل مع الجيل الحادي عشر في خطّاف واحد على سلسلة عرض دي‌إكس‌جي‌آي نفسها، \
         فلا يُركَّب شيء إضافي على اللعبة.",
        "this generation shares one hook with Direct3D 11 on the same DXGI swap chain, so nothing \
         additional is installed in the game",
    ))
}
