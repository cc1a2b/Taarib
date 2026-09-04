//! خطّاف دايركت٣د ١٢ — the overlay recorded into its own command lists.
//!
//! D3D12 removes the problem [`crate::d3d11`] spends most of its length on and
//! replaces it with a different one. There is no immediate context to leave
//! misconfigured: a command list is a private recording, the overlay's binds go
//! into the overlay's list, and nothing the game recorded is disturbed. That is
//! why [`KhattafD3D12::irsim`] has no save-and-restore step and why this file
//! has no equivalent of `HalatMasar`.
//!
//! What it has instead is **lifetime against GPU progress**. In D3D11 the
//! runtime tracks which resources a submitted draw still needs and refuses to
//! free them early. In D3D12 nothing does. A command allocator reset while the
//! GPU is still executing the list it holds corrupts that list; an upload buffer
//! released while a queued draw still reads it produces a page fault inside the
//! driver, attributed to the game. Both are silent until they are not, and both
//! are ordinary if the overlay simply does the obvious thing every frame.
//!
//! So every resource on the draw path is per-backbuffer, every frame records the
//! fence value it was submitted at, and nothing belonging to frame *n* is
//! touched again until the fence says frame *n* finished. That single rule is
//! what `KhattafD3D12::intazir` enforces and what every other method in this
//! file is arranged around, including [`Khattaf::ahmil`], which waits for the
//! whole queue to drain before it drops anything at all.
//!
//! When that wait does not come back, the overlay stops. `crate::khata` names
//! the case: "a D3D12 fence says the overlay's resources are still in flight
//! when they were about to be freed" is one of the two conditions
//! [`KhataTabaqa::HalaGhayrMustaada`] exists for, and it is terminal for the
//! session rather than a frame to retry — because the retry would run against
//! the same unconfirmed fence and reach the same reset.
//!
//! ## Why the command queue is passed in
//!
//! A swap chain can hand over its device. It cannot hand over the queue that
//! presents it — `IDXGISwapChain3` has no method for it, and there is no way to
//! ask D3D12 which of a game's queues a given swap chain belongs to. Submitting
//! the overlay's list on a queue the game does not present from would draw into
//! a backbuffer the compositor never looks at, on a timeline nothing
//! synchronises against.
//!
//! That is the whole reason `crate::khataf` hooks
//! `ID3D12CommandQueue::ExecuteCommandLists` in the first place: not to inspect
//! anything the game submits, but to learn which queue object exists. The hook
//! captures the first direct queue it sees and hands it here, and this backend
//! takes it as a constructor parameter rather than trying to discover it —
//! because a backend that guessed would be a backend that worked on every game
//! with one queue and failed silently on every game with two.
//!
//! ## The one thing the hook has to do for this backend
//!
//! `ResizeBuffers` refuses while anybody holds a backbuffer, and this backend
//! holds every one of them. The hook calls
//! [`crate::wajiha::Tabaqa::qabl_taghyeer_hajm`] before it forwards the resize,
//! which reaches [`KhattafD3D12::atliq_khalfiyat`] through
//! [`Khattaf::atliq_sath`] — a trait method rather than only an inherent one,
//! because the hook holds a `dyn Khattaf` and cannot name this type. That
//! drains the queue and drops the frames, which is the same wait every other
//! release in this file performs, arrived at from the game's thread instead of
//! the overlay's.
//!
//! ## Descriptors, and why there are so few
//!
//! One RTV per backbuffer, one SRV for the atlas, one root constant buffer view
//! for the projection, one static sampler. The sampler is static — baked into
//! the root signature — because a sampler descriptor heap is a second
//! shader-visible heap, `SetDescriptorHeaps` accepts at most one of each type,
//! and setting one would replace whatever the game had bound in a way that
//! outlives the overlay's own command list on some drivers.

use std::fmt;
use std::mem::{self, ManuallyDrop};

use windows::Win32::Foundation::{CloseHandle, HANDLE, RECT, WAIT_OBJECT_0};
use windows::Win32::Graphics::Direct3D::{D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST, ID3DBlob};
use windows::Win32::Graphics::Direct3D12::{
    D3D_ROOT_SIGNATURE_VERSION_1, D3D12_BLEND_DESC, D3D12_BLEND_INV_SRC_ALPHA, D3D12_BLEND_ONE,
    D3D12_BLEND_OP_ADD, D3D12_BOX, D3D12_CACHED_PIPELINE_STATE, D3D12_COLOR_WRITE_ENABLE_ALL,
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMPARISON_FUNC_ALWAYS, D3D12_COMPARISON_FUNC_NEVER,
    D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF, D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT,
    D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_CULL_MODE_NONE,
    D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING, D3D12_DEPTH_STENCIL_DESC, D3D12_DEPTH_STENCILOP_DESC,
    D3D12_DEPTH_WRITE_MASK_ZERO, D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE, D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
    D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_DESCRIPTOR_RANGE, D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
    D3D12_FENCE_FLAG_NONE, D3D12_FILL_MODE_SOLID, D3D12_FILTER_MIN_MAG_MIP_LINEAR,
    D3D12_GRAPHICS_PIPELINE_STATE_DESC, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
    D3D12_HEAP_TYPE_DEFAULT, D3D12_HEAP_TYPE_READBACK, D3D12_HEAP_TYPE_UPLOAD,
    D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED, D3D12_INDEX_BUFFER_VIEW,
    D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA, D3D12_INPUT_ELEMENT_DESC, D3D12_INPUT_LAYOUT_DESC,
    D3D12_LOGIC_OP_NOOP, D3D12_MEMORY_POOL_UNKNOWN, D3D12_PIPELINE_STATE_FLAG_NONE,
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT, D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE, D3D12_RANGE,
    D3D12_RASTERIZER_DESC, D3D12_RENDER_TARGET_BLEND_DESC, D3D12_RESOURCE_BARRIER,
    D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
    D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC,
    D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE,
    D3D12_RESOURCE_STATE_COPY_DEST, D3D12_RESOURCE_STATE_COPY_SOURCE,
    D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATES,
    D3D12_RESOURCE_TRANSITION_BARRIER, D3D12_ROOT_DESCRIPTOR, D3D12_ROOT_DESCRIPTOR_TABLE,
    D3D12_ROOT_PARAMETER, D3D12_ROOT_PARAMETER_0, D3D12_ROOT_PARAMETER_TYPE_CBV,
    D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, D3D12_ROOT_SIGNATURE_DESC,
    D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
    D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS,
    D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS,
    D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS, D3D12_SHADER_BYTECODE,
    D3D12_SHADER_RESOURCE_VIEW_DESC, D3D12_SHADER_RESOURCE_VIEW_DESC_0,
    D3D12_SHADER_VISIBILITY_ALL, D3D12_SHADER_VISIBILITY_PIXEL, D3D12_SRV_DIMENSION_TEXTURE2D,
    D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK, D3D12_STATIC_SAMPLER_DESC, D3D12_STENCIL_OP_KEEP,
    D3D12_STREAM_OUTPUT_DESC, D3D12_TEX2D_SRV, D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
    D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
    D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    D3D12_TEXTURE_DATA_PITCH_ALIGNMENT, D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
    D3D12_TEXTURE_LAYOUT_UNKNOWN, D3D12_VERTEX_BUFFER_VIEW,
    D3D12_VIEWPORT, D3D12SerializeRootSignature, ID3D12CommandAllocator, ID3D12CommandList,
    ID3D12CommandQueue, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
    ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource, ID3D12RootSignature,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R32_FLOAT, DXGI_FORMAT_R32_UINT,
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_UNKNOWN,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::IDXGISwapChain3;
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
use windows::core::{Interface, PCWSTR, s};

use crate::d3d11::{
    IZAHAT_KHALT, IZAHAT_KHAREETA, IZAHAT_LAWN, IZAHAT_MAWDI, KHATWAT_RAS, MusarrifHlsl,
    QITA_LIL_DUFA, Ras, SAQF_LAWHA, ThawabitIsqat, imla_qita, madaa_f32, sigha_min_dxgi,
};
use crate::khata::{KhataTabaqa, tul_u64};
use crate::wajiha::{
    Khattaf, LawhatRasm, MustatilBiksel, SighatSath, WajihatRusum, WasfSath,
};

/// How long a fence wait is given before it is treated as a fault.
///
/// Not `INFINITE`, and the difference matters more here than anywhere else in
/// this crate: this wait runs on a game's render thread inside its `Present`
/// call. A wait that never returns is a game the player cannot close, cannot
/// alt-tab out of, and has to kill from a task manager — which is a far worse
/// outcome than an overlay that switches itself off. One second is longer than
/// any frame a GPU that is still working will take and shorter than the driver's
/// own timeout detection, so a wait that reaches it has already lost.
const MUHLAT_INTIZAR: u32 = 1_000;

/// The longer wait [`Khattaf::ahmil`] gives the queue before it gives up.
///
/// Shutdown is the one place a stall is tolerable, because there is no next
/// frame to be late for and the alternative is releasing memory a GPU is reading.
const MUHLAT_KHUMUL: u32 = 5_000;

/// The largest batch this build will draw in one frame, in quads.
///
/// A ceiling exists because the per-frame vertex buffer grows to fit the batch,
/// and a growth path with no bound is an allocation the caller controls the size
/// of. A quarter of a million quads is two orders of magnitude past a screen
/// full of dialogue at any font size.
const SAQF_QITA: usize = 262_144;

/// How many bytes a placed texture footprint's rows are aligned to.
///
/// `D3D12_TEXTURE_DATA_PITCH_ALIGNMENT`. Named here because it appears in both
/// directions — the atlas upload writes to it and the frame capture reads past
/// it — and a hard-coded 256 in two places is a 256 that gets changed in one.
const MUHADHAT_SAF: u32 = 256;

// The pitch alignment above is the SDK's, restated. If the SDK ever disagreed
// the upload would write rows the copy does not read.
const _: () = {
    assert!(
        MUHADHAT_SAF == D3D12_TEXTURE_DATA_PITCH_ALIGNMENT,
        "the row pitch alignment does not match the SDK"
    );
};

/// A Win32 event handle that closes itself and can move between threads.
///
/// [`Khattaf`] is `Send`, and a bare [`HANDLE`] is not — it is a raw pointer, so
/// the auto trait does not apply. Wrapping it here rather than storing the
/// numeric value and rebuilding the handle at each use means the close happens
/// in exactly one place, which is the only way a handle leak stays impossible
/// across the several error paths that abandon construction half way through.
#[derive(Debug)]
struct HadathIntizar(HANDLE);

// SAFETY: a Win32 event handle is a kernel object reference, not a pointer into
// this process's address space. Every operation on it — `SetEventOnCompletion`,
// `WaitForSingleObject`, `CloseHandle` — is documented as callable from any
// thread, and this wrapper hands out no interior reference that could be used
// concurrently: the handle is only ever read through `&self` and closed once, in
// `Drop`, when no other reference exists.
unsafe impl Send for HadathIntizar {}

impl Drop for HadathIntizar {
    fn drop(&mut self) {
        // SAFETY: the handle came from `CreateEventW` and has not been closed —
        // this is the only place that closes it, and it runs once. The result is
        // discarded because a failing `CloseHandle` at teardown has no remedy
        // and no caller left to tell.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// A command allocator and the list recorded into it.
///
/// Paired because in D3D12 they are not independent: a list is reset onto an
/// allocator, and resetting the allocator while that list is executing is the
/// corruption this file exists to avoid. Keeping them in one struct means there
/// is no arrangement of fields where a list is reset onto somebody else's
/// allocator by accident.
#[derive(Debug)]
struct ZawjAwamir {
    mukhassis: ID3D12CommandAllocator,
    qaima: ID3D12GraphicsCommandList,
}

/// Everything that belongs to one backbuffer.
///
/// One of these per swap chain buffer, indexed by
/// `IDXGISwapChain3::GetCurrentBackBufferIndex`. `qeema` is the fence value the
/// queue was signalled with when this frame's list was last submitted, and it is
/// the only thing standing between a reset and a list the GPU is still reading.
#[derive(Debug)]
struct ItarD3D12 {
    khalfiya: ID3D12Resource,
    mawdi_hadaf: D3D12_CPU_DESCRIPTOR_HANDLE,
    zawj: ZawjAwamir,
    ruus: ID3D12Resource,
    masar_ruus: u64,
    siat_ruus: usize,
    qeema: u64,
}

/// A non-owning view of an interface, for the descriptor structs that hold one
/// as [`ManuallyDrop`].
///
/// D3D12's descriptor structures declare their resource fields as
/// `ManuallyDrop<Option<T>>` because the C structs are plain borrowed pointers:
/// filling one in does not transfer ownership and the runtime does not release
/// what it finds there. Cloning into it and dropping the `ManuallyDrop` by hand
/// is possible and is wrong the first time an error path returns early, so the
/// borrow is made explicit instead — the caller keeps the real reference alive
/// across the call, which every use in this file does.
fn muara<T: Interface>(shay: &T) -> ManuallyDrop<Option<T>> {
    // SAFETY: every `windows` interface wrapper is a `#[repr(transparent)]`
    // newtype over a single non-null pointer, `Option<T>` of one is the same
    // size by niche optimisation, and `ManuallyDrop` adds nothing. The copy
    // therefore reinterprets one pointer-sized value as another of the same
    // layout, and because it is a `ManuallyDrop` it will never be released — so
    // the reference count is untouched and `shay` remains the sole owner.
    unsafe { mem::transmute_copy(shay) }
}

/// A transition barrier for one resource, covering all its subresources.
///
/// The resource is borrowed rather than owned, so the caller must keep it alive
/// until `ResourceBarrier` has been recorded. Every call site in this file holds
/// the resource in a local for exactly that reason.
fn hajiz_intiqal(
    mawrid: &ID3D12Resource,
    min: D3D12_RESOURCE_STATES,
    ila: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                pResource: muara(mawrid),
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: min,
                StateAfter: ila,
            }),
        },
    }
}

/// The heap properties for a plain, CPU-visible upload buffer.
const fn khasais_rafa() -> D3D12_HEAP_PROPERTIES {
    D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_UPLOAD,
        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
        CreationNodeMask: 1,
        VisibleNodeMask: 1,
    }
}

/// The heap properties for GPU-local memory.
const fn khasais_iftiradi() -> D3D12_HEAP_PROPERTIES {
    D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_DEFAULT,
        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
        CreationNodeMask: 1,
        VisibleNodeMask: 1,
    }
}

/// The heap properties for a buffer the CPU reads back from.
const fn khasais_qira() -> D3D12_HEAP_PROPERTIES {
    D3D12_HEAP_PROPERTIES {
        Type: D3D12_HEAP_TYPE_READBACK,
        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
        CreationNodeMask: 1,
        VisibleNodeMask: 1,
    }
}

/// A linear buffer resource description of the given byte length.
const fn wasf_mukhazzan(tul: u64) -> D3D12_RESOURCE_DESC {
    D3D12_RESOURCE_DESC {
        Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
        Alignment: 0,
        Width: tul,
        Height: 1,
        DepthOrArraySize: 1,
        MipLevels: 1,
        Format: DXGI_FORMAT_UNKNOWN,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        // A buffer is always row-major; naming anything else here is rejected at
        // creation rather than at first use, which is the reason this is a
        // function and not a literal repeated at four call sites.
        Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
        Flags: D3D12_RESOURCE_FLAG_NONE,
    }
}

/// Rounds a length up to the alignment D3D12 requires for placed rows.
///
/// Written with a mask rather than a division because integer division is denied
/// workspace-wide and because the alignment is a power of two, where the mask is
/// what the division would compile to anyway.
const fn hadhi(qeema: u64, muhadhat: u64) -> u64 {
    let baqi = muhadhat.saturating_sub(1);
    qeema.saturating_add(baqi) & !baqi
}

/// The Direct3D 12 overlay backend.
///
/// Holds the game's device, the command queue the hook captured, the swap chain,
/// and one set of frame resources per backbuffer. Everything on the draw path is
/// per-frame; everything shared — root signature, pipeline state, index buffer,
/// projection, atlas — is either immutable once built or only rebuilt with the
/// whole queue drained.
pub struct KhattafD3D12 {
    jihaz: ID3D12Device,
    saff: ID3D12CommandQueue,
    silsila: IDXGISwapChain3,
    musarrif: MusarrifHlsl,
    bayt_ras: Vec<u8>,
    bayt_biksel: Vec<u8>,
    tawqee: Option<ID3D12RootSignature>,
    halat_rasm: Option<ID3D12PipelineState>,
    kawmat_ahdaf: Option<ID3D12DescriptorHeap>,
    kawmat_wasf: Option<ID3D12DescriptorHeap>,
    khatwat_hadaf: usize,
    itarat: Vec<ItarD3D12>,
    naql: Option<ZawjAwamir>,
    faharis: Option<ID3D12Resource>,
    thawabit: Option<ID3D12Resource>,
    masar_thawabit: u64,
    lawha: Option<ID3D12Resource>,
    hajiz: Option<ID3D12Fence>,
    hadath: Option<HadathIntizar>,
    qeemat_hajiz: u64,
    sath: Option<WasfSath>,
    musawwada: Vec<Ras>,
}

impl fmt::Debug for KhattafD3D12 {
    fn fmt(&self, mukhraj: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Written by hand rather than derived. The derive would print every
        // interface pointer in the struct, and this type is what a diagnostics
        // bundle names when it says which backend was live — a line of thirty
        // raw addresses is not what anybody reading that bundle needs, and the
        // addresses are somebody else's process's.
        mukhraj
            .debug_struct("KhattafD3D12")
            .field("sath", &self.sath)
            .field("itarat", &self.itarat.len())
            .field("qeemat_hajiz", &self.qeemat_hajiz)
            .field("lawha", &self.lawha.is_some())
            .field("jahiz", &self.halat_rasm.is_some())
            .finish()
    }
}

impl KhattafD3D12 {
    /// Builds a backend around a swap chain and the queue that presents it.
    ///
    /// Both parameters are required and neither can be derived from the other.
    /// `IDXGISwapChain::GetDevice` on a D3D12 swap chain returns the object it
    /// was created with, which for D3D12 is the command queue rather than the
    /// device — so the device is taken from the queue, through
    /// `ID3D12DeviceChild::GetDevice`, which is the one path that cannot name
    /// the wrong device.
    ///
    /// The queue itself has to come from `crate::khataf`'s
    /// `ExecuteCommandLists` hook. See the module header: there is no API that
    /// answers "which queue presents this swap chain", and submitting on the
    /// wrong one produces an overlay that draws into a buffer nobody displays.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the queue will not name its device,
    /// [`KhataTabaqa::SathTaghayyar`] when the swap chain will not describe
    /// itself, and [`KhataTabaqa::MaktabaMafquda`] when the HLSL compiler is not
    /// on this machine.
    pub fn min_silsila_wa_saff(
        silsila: &IDXGISwapChain3,
        saff: &ID3D12CommandQueue,
    ) -> Result<Self, KhataTabaqa> {
        let mut jihaz: Option<ID3D12Device> = None;
        // SAFETY: `saff` is a live command queue, which is a device child, and
        // `GetDevice` performs a QueryInterface into the out-pointer, which
        // addresses a local initialised to `None`.
        unsafe { saff.GetDevice(&raw mut jihaz) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "the command queue's D3D12 device",
                sabab: khata.to_string(),
            }
        })?;
        let Some(jihaz) = jihaz else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "the command queue's D3D12 device",
                sabab: "the queue reported success and named no device".to_owned(),
            });
        };

        // SAFETY: `silsila` is a live swap chain and `GetDesc` fills a stack
        // description the binding owns. Called here so a swap chain that will
        // not describe itself is refused before any hook goes live.
        unsafe { silsila.GetDesc() }.map_err(|khata| KhataTabaqa::SathTaghayyar {
            sabab: format!("the swap chain would not describe itself: {khata}"),
        })?;

        let musarrif = MusarrifHlsl::ihdar()?;

        Ok(Self {
            jihaz,
            saff: saff.clone(),
            silsila: silsila.clone(),
            musarrif,
            bayt_ras: Vec::new(),
            bayt_biksel: Vec::new(),
            tawqee: None,
            halat_rasm: None,
            kawmat_ahdaf: None,
            kawmat_wasf: None,
            khatwat_hadaf: 0,
            itarat: Vec::new(),
            naql: None,
            faharis: None,
            thawabit: None,
            masar_thawabit: 0,
            lawha: None,
            hajiz: None,
            hadath: None,
            qeemat_hajiz: 0,
            sath: None,
            musawwada: Vec::new(),
        })
    }

    /// Waits until the fence has reached a value, or gives up.
    ///
    /// The whole safety argument of this backend passes through here. Every
    /// caller that is about to reset an allocator, replace a resource or free
    /// one first asks this whether the work that used it has finished, and every
    /// one of them treats a refusal as a reason not to proceed.
    ///
    /// A value of zero means "nothing was ever submitted for this", which is the
    /// state of every frame before its first draw, and is not waited on.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the fence or its event is missing or
    /// the runtime refuses to signal the event, and when the wait reaches
    /// [`MUHLAT_INTIZAR`] without the fence advancing — at which point the GPU
    /// has been busy for longer than any frame takes and the caller must not
    /// assume the work is done.
    fn intazir(&self, qeema: u64, muhla: u32) -> Result<(), KhataTabaqa> {
        if qeema == 0 {
            return Ok(());
        }
        let (Some(hajiz), Some(hadath)) = (self.hajiz.as_ref(), self.hadath.as_ref()) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay fence",
                sabab: "the backend was asked to synchronise before its fence was built"
                    .to_owned(),
            });
        };

        // SAFETY: `hajiz` is a live fence. `GetCompletedValue` reads a value and
        // has no preconditions beyond the object being alive.
        if unsafe { hajiz.GetCompletedValue() } >= qeema {
            return Ok(());
        }

        // SAFETY: `hajiz` is live and `hadath.0` is an event handle from
        // `CreateEventW` that this backend owns and has not closed.
        unsafe { hajiz.SetEventOnCompletion(qeema, hadath.0) }.map_err(|khata| {
            KhataTabaqa::MawridFashil {
                mawrid: "overlay fence",
                sabab: format!("the completion event could not be armed: {khata}"),
            }
        })?;

        // SAFETY: the same live event handle, with a finite timeout.
        let natija = unsafe { WaitForSingleObject(hadath.0, muhla) };
        if natija == WAIT_OBJECT_0 {
            return Ok(());
        }
        Err(KhataTabaqa::MawridFashil {
            mawrid: "overlay fence",
            sabab: format!(
                "the GPU did not reach fence value {qeema} within {muhla} ms (wait returned {})",
                natija.0
            ),
        })
    }

    /// Releases every reference this backend holds to a swap chain backbuffer.
    ///
    /// `IDXGISwapChain::ResizeBuffers` fails with `DXGI_ERROR_INVALID_CALL` if
    /// anybody still holds a backbuffer, and this backend holds all of them —
    /// one per frame, for the life of the surface. A game resizing its window
    /// with the overlay installed would get a refusal from a call that has never
    /// refused before.
    ///
    /// So `crate::khataf` hooks `ResizeBuffers` and calls this immediately
    /// before forwarding. The queue is drained first, because the frames being
    /// dropped hold command lists that may still be executing — the same rule as
    /// everywhere else in this file, applied to the one case where the caller is
    /// the game's own resize rather than the overlay's own frame.
    ///
    /// Recovery is this backend's own. [`crate::wajiha::Tabaqa`] rebuilds when
    /// the surface's *description* changes, and a `ResizeBuffers` that keeps the
    /// same width, height and format changes nothing it can see — which is what
    /// a borderless fullscreen transition on the monitor the window already
    /// filled does. So the surface is forgotten here and [`Khattaf::irsim`]
    /// notices it is missing and rebuilds.
    ///
    /// Deliberately not part of [`Khattaf`], for the same reason as its D3D11
    /// counterpart: the OpenGL and Vulkan backends have nothing to release here,
    /// and a trait method two of four implementations leave empty is a trait
    /// method nobody can tell is load-bearing.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HalaGhayrMustaada`] when the queue will not drain, which
    /// is terminal for the same reason it is terminal everywhere else: the
    /// backbuffers are about to be released and the fence has not said the GPU
    /// is finished with them.
    pub fn atliq_khalfiyat(&mut self) -> Result<(), KhataTabaqa> {
        let qeema = match self.ashir() {
            Ok(qeema) => qeema,
            // No fence means nothing was ever submitted, so nothing is holding
            // anything. This is the ordinary state of a backend that was
            // constructed and never drew.
            Err(_) if self.hajiz.is_none() => 0,
            Err(khata) => {
                return Err(KhataTabaqa::HalaGhayrMustaada {
                    hala: "the overlay's fence-tracked resource lifetimes",
                    sabab: format!(
                        "the queue would not signal before the backbuffers were released: {khata}"
                    ),
                });
            }
        };
        self.intazir_qabl_massa(qeema, MUHLAT_INTIZAR)?;
        self.itarat.clear();
        self.kawmat_ahdaf = None;
        self.khatwat_hadaf = 0;
        self.sath = None;
        Ok(())
    }

    /// Waits before something is reset, overwritten or freed, and treats a
    /// failure as terminal.
    ///
    /// [`crate::khata`] names this case by hand: "a D3D12 fence says the
    /// overlay's resources are still in flight when they were about to be
    /// freed" is listed there as a state-corruption risk rather than as a bad
    /// frame, and the prescribed response is to disable the overlay for the
    /// session. That is what [`KhataTabaqa::HalaGhayrMustaada`] does, and it is
    /// why this exists as a separate call from [`KhattafD3D12::intazir`]: the
    /// same timeout means "skip this frame" when nothing was about to be
    /// touched and "stop now" when something was.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::HalaGhayrMustaada`] for anything
    /// [`KhattafD3D12::intazir`] refuses.
    fn intazir_qabl_massa(&self, qeema: u64, muhla: u32) -> Result<(), KhataTabaqa> {
        self.intazir(qeema, muhla).map_err(|khata| KhataTabaqa::HalaGhayrMustaada {
            hala: "the overlay's fence-tracked resource lifetimes",
            sabab: khata.to_string(),
        })
    }

    /// Signals the queue with a fresh value and returns it.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the fence is missing or the queue
    /// refuses the signal, which on a live device means the device is going away.
    fn ashir(&mut self) -> Result<u64, KhataTabaqa> {
        let Some(hajiz) = self.hajiz.as_ref() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay fence",
                sabab: "the backend was asked to signal before its fence was built".to_owned(),
            });
        };
        let qeema = self.qeemat_hajiz.saturating_add(1);
        // SAFETY: `saff` is the live command queue the hook captured and `hajiz`
        // is this backend's own fence. Signalling is queued behind whatever the
        // queue is already executing, which is exactly the ordering every wait
        // in this file depends on.
        unsafe { self.saff.Signal(hajiz, qeema) }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay fence",
            sabab: format!("the queue would not signal: {khata}"),
        })?;
        self.qeemat_hajiz = qeema;
        Ok(qeema)
    }

    /// Waits until everything this backend ever submitted has finished.
    ///
    /// Called before anything shared is rebuilt or released. It is a full stall
    /// and it is meant to be: the operations that need it — a resize, an atlas
    /// change, shutdown — happen at most a handful of times in a session, and
    /// the alternative is per-resource lifetime tracking that would be wrong
    /// once and silent about it.
    ///
    /// # Errors
    ///
    /// Whatever [`KhattafD3D12::ashir`] and [`KhattafD3D12::intazir`] refuse.
    fn intazir_khumul(&mut self, muhla: u32) -> Result<(), KhataTabaqa> {
        if self.hajiz.is_none() {
            // Nothing was ever submitted, so nothing is in flight. This is the
            // ordinary case during the first `hayyi` and during a teardown of a
            // backend that never finished initializing.
            return Ok(());
        }
        let qeema = self.ashir()?;
        self.intazir(qeema, muhla)
    }

    /// Drops everything [`Khattaf::hayyi`] builds, keeping the atlas.
    ///
    /// The caller is responsible for having drained the queue first. This
    /// function deliberately does not do it itself: `ahmil` has a different
    /// timeout and a different answer to a wait that fails, and folding both
    /// into one place would mean one of them getting the wrong one.
    fn atlif_masar(&mut self) {
        self.itarat.clear();
        self.naql = None;
        self.kawmat_ahdaf = None;
        self.kawmat_wasf = None;
        self.khatwat_hadaf = 0;
        self.halat_rasm = None;
        self.tawqee = None;
        self.faharis = None;
        self.thawabit = None;
        self.masar_thawabit = 0;
        self.sath = None;
    }

    /// Compiles the shared HLSL at shader model 5.0, once per process.
    ///
    /// The same source [`crate::d3d11`] uses, at a higher profile: D3D12 will
    /// not accept a `vs_4_0` blob, and 5.0 is the floor every D3D12-capable
    /// driver supports. Cached for the same reason as on D3D11 — `hayyi` runs
    /// again on every resolution change and a recompile there is a stall on the
    /// render thread.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] carrying the compiler's diagnostics.
    fn ihdar_ramz(&mut self) -> Result<(), KhataTabaqa> {
        if self.bayt_ras.is_empty() {
            self.bayt_ras =
                self.musarrif.sarrif("overlay vertex shader", s!("ras"), s!("vs_5_0"))?;
        }
        if self.bayt_biksel.is_empty() {
            self.bayt_biksel =
                self.musarrif.sarrif("overlay pixel shader", s!("biksel"), s!("ps_5_0"))?;
        }
        Ok(())
    }

    /// Creates the fence and the event the waits are built on.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when either cannot be created. There is no
    /// degraded mode without them: a backend that cannot tell when the GPU has
    /// finished cannot safely reset anything, so this failing means the backend
    /// does not run at all.
    fn ibni_hajiz(&mut self) -> Result<(), KhataTabaqa> {
        if self.hajiz.is_none() {
            // SAFETY: `jihaz` is the live device. `CreateFence` returns an owned
            // interface or an error; nothing is written through a raw pointer.
            let hajiz: ID3D12Fence = unsafe { self.jihaz.CreateFence(0, D3D12_FENCE_FLAG_NONE) }
                .map_err(|khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay fence",
                    sabab: khata.to_string(),
                })?;
            self.hajiz = Some(hajiz);
            self.qeemat_hajiz = 0;
        }
        if self.hadath.is_none() {
            // SAFETY: no security attributes, a manual-reset flag of false so
            // the event auto-resets after each wait, an unsignalled initial
            // state, and no name — a named event could be opened by another
            // process, which an overlay has no reason to allow.
            let hadath = unsafe { CreateEventW(None, false, false, PCWSTR::null()) }.map_err(
                |khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay fence event",
                    sabab: khata.to_string(),
                },
            )?;
            self.hadath = Some(HadathIntizar(hadath));
        }
        Ok(())
    }

    /// Serializes and creates the root signature.
    ///
    /// One root CBV for the projection, one one-entry descriptor table for the
    /// atlas, one static sampler. The three shader stages the overlay does not
    /// use are denied access rather than left permitted, which lets the driver
    /// lay the signature out more tightly and, more usefully, makes a future
    /// edit that adds a geometry shader fail at creation instead of at draw.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] carrying the serializer's own diagnostics
    /// blob when it produced one.
    fn ibni_tawqee(&mut self) -> Result<(), KhataTabaqa> {
        let nitaq = D3D12_DESCRIPTOR_RANGE {
            RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
            NumDescriptors: 1,
            BaseShaderRegister: 0,
            RegisterSpace: 0,
            OffsetInDescriptorsFromTableStart: 0,
        };
        let muamilat = [
            D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR { ShaderRegister: 0, RegisterSpace: 0 },
                },
                // Visible to both stages: the vertex shader reads the matrix and
                // the pixel shader reads the encode flag out of the same buffer.
                ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
            },
            D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                        NumDescriptorRanges: 1,
                        pDescriptorRanges: &raw const nitaq,
                    },
                },
                ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
            },
        ];
        let akhidh = D3D12_STATIC_SAMPLER_DESC {
            Filter: D3D12_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            MipLODBias: 0.0,
            MaxAnisotropy: 0,
            ComparisonFunc: D3D12_COMPARISON_FUNC_NEVER,
            BorderColor: D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
            MinLOD: 0.0,
            MaxLOD: 0.0,
            ShaderRegister: 0,
            RegisterSpace: 0,
            ShaderVisibility: D3D12_SHADER_VISIBILITY_PIXEL,
        };
        let wasf = D3D12_ROOT_SIGNATURE_DESC {
            NumParameters: 2,
            pParameters: muamilat.as_ptr(),
            NumStaticSamplers: 1,
            pStaticSamplers: &raw const akhidh,
            Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS
                | D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS,
        };

        let mut kutla: Option<ID3DBlob> = None;
        let mut bayan: Option<ID3DBlob> = None;
        // SAFETY: `wasf` borrows `muamilat`, `nitaq` and `akhidh`, all of which
        // are live locals for the whole call, and both out-pointers address
        // locals initialised to `None`.
        let natija = unsafe {
            D3D12SerializeRootSignature(
                &raw const wasf,
                D3D_ROOT_SIGNATURE_VERSION_1,
                &raw mut kutla,
                Some(&raw mut bayan),
            )
        };

        if let Err(khata) = natija {
            let tafsil = bayan.as_ref().map(nass_kutla).unwrap_or_default();
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay root signature",
                sabab: if tafsil.is_empty() {
                    khata.to_string()
                } else {
                    format!("{khata}: {tafsil}")
                },
            });
        }

        let Some(kutla) = kutla else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay root signature",
                sabab: "the serializer reported success and produced no blob".to_owned(),
            });
        };

        // SAFETY: `kutla` is a live blob and the pointer and length it reports
        // describe its own allocation, which outlives the borrow taken here.
        let bayt = unsafe {
            let asas = kutla.GetBufferPointer().cast::<u8>();
            let tul = kutla.GetBufferSize();
            if asas.is_null() || tul == 0 {
                &[][..]
            } else {
                core::slice::from_raw_parts(asas, tul)
            }
        };
        if bayt.is_empty() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay root signature",
                sabab: "the serializer produced an empty blob".to_owned(),
            });
        }

        // SAFETY: `jihaz` is live and `bayt` is a serialized root signature the
        // call above produced, borrowed for the duration of this call only.
        let tawqee: ID3D12RootSignature = unsafe { self.jihaz.CreateRootSignature(0, bayt) }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay root signature",
                sabab: khata.to_string(),
            })?;
        self.tawqee = Some(tawqee);
        Ok(())
    }

    /// Creates the pipeline state object for one backbuffer format.
    ///
    /// Rebuilt on every `hayyi` because `RTVFormats` is baked into it. A PSO
    /// kept across a format change is a PSO the runtime rejects at draw time,
    /// once per frame, inside the game's present call.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the root signature is missing or the
    /// device refuses the pipeline.
    fn ibni_halat_rasm(&mut self, sigha: DXGI_FORMAT) -> Result<(), KhataTabaqa> {
        let Some(tawqee) = self.tawqee.as_ref() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay pipeline state",
                sabab: "the pipeline was asked for before its root signature existed".to_owned(),
            });
        };

        let anasir = [
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_MAWDI,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHAREETA,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: s!("COLOR"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_LAWN,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D12_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 1,
                Format: DXGI_FORMAT_R32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: IZAHAT_KHALT,
                InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];

        // The same premultiplied blend as the D3D11 backend, for the same
        // reason: `saff` hands over coverage already multiplied into the colour.
        let khalt_hadaf = D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true.into(),
            LogicOpEnable: false.into(),
            SrcBlend: D3D12_BLEND_ONE,
            DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D12_BLEND_OP_ADD,
            SrcBlendAlpha: D3D12_BLEND_ONE,
            DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D12_BLEND_OP_ADD,
            LogicOp: D3D12_LOGIC_OP_NOOP,
            RenderTargetWriteMask: u8::try_from(D3D12_COLOR_WRITE_ENABLE_ALL.0).unwrap_or(0x0F),
        };
        let tazlil = D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: D3D12_STENCIL_OP_KEEP,
            StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
            StencilPassOp: D3D12_STENCIL_OP_KEEP,
            StencilFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        };

        let wasf = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
            pRootSignature: muara(tawqee),
            VS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: self.bayt_ras.as_ptr().cast(),
                BytecodeLength: self.bayt_ras.len(),
            },
            PS: D3D12_SHADER_BYTECODE {
                pShaderBytecode: self.bayt_biksel.as_ptr().cast(),
                BytecodeLength: self.bayt_biksel.len(),
            },
            DS: D3D12_SHADER_BYTECODE::default(),
            HS: D3D12_SHADER_BYTECODE::default(),
            GS: D3D12_SHADER_BYTECODE::default(),
            StreamOutput: D3D12_STREAM_OUTPUT_DESC::default(),
            BlendState: D3D12_BLEND_DESC {
                AlphaToCoverageEnable: false.into(),
                IndependentBlendEnable: false.into(),
                RenderTarget: [khalt_hadaf; 8],
            },
            SampleMask: u32::MAX,
            RasterizerState: D3D12_RASTERIZER_DESC {
                FillMode: D3D12_FILL_MODE_SOLID,
                CullMode: D3D12_CULL_MODE_NONE,
                FrontCounterClockwise: false.into(),
                DepthBias: 0,
                DepthBiasClamp: 0.0,
                SlopeScaledDepthBias: 0.0,
                DepthClipEnable: true.into(),
                MultisampleEnable: false.into(),
                AntialiasedLineEnable: false.into(),
                ForcedSampleCount: 0,
                ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
            },
            // Depth test and depth write both off, and no DSV bound at all, so
            // the overlay cannot stamp itself into a depth buffer the game reads
            // on its next frame.
            DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
                DepthEnable: false.into(),
                DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
                DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                StencilEnable: false.into(),
                StencilReadMask: 0,
                StencilWriteMask: 0,
                FrontFace: tazlil,
                BackFace: tazlil,
            },
            InputLayout: D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: anasir.as_ptr(),
                NumElements: u32::try_from(anasir.len()).unwrap_or(0),
            },
            IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
            PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
            NumRenderTargets: 1,
            RTVFormats: core::array::from_fn(|fahras| {
                if fahras == 0 { sigha } else { DXGI_FORMAT_UNKNOWN }
            }),
            DSVFormat: DXGI_FORMAT_UNKNOWN,
            // A D3D12 flip-model swap chain cannot be multisampled, so the
            // overlay never faces the resolve the D3D11 capture path has to.
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            NodeMask: 0,
            CachedPSO: D3D12_CACHED_PIPELINE_STATE::default(),
            Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
        };

        // SAFETY: `wasf` borrows `anasir`, both shader byte vectors and
        // `tawqee`, all of which are alive for the duration of this call, and
        // the device copies everything it needs out of the description before
        // returning.
        let halat: ID3D12PipelineState =
            unsafe { self.jihaz.CreateGraphicsPipelineState(&raw const wasf) }.map_err(
                |khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay pipeline state",
                    sabab: khata.to_string(),
                },
            )?;
        self.halat_rasm = Some(halat);
        Ok(())
    }
}

impl KhattafD3D12 {
    /// Creates the render target heap and the shader-visible atlas heap.
    ///
    /// The RTV heap is not shader visible — render target views never are — and
    /// the CBV/SRV heap holds exactly one descriptor, because the overlay reads
    /// one texture and nothing else. A larger heap would cost nothing and would
    /// invite a future edit to put a second thing in it without thinking about
    /// which command list has it bound.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when either heap cannot be created.
    fn ibni_kawmat(&mut self, adad_ilqa: u32) -> Result<(), KhataTabaqa> {
        let wasf_ahdaf = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            NumDescriptors: adad_ilqa,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        };
        // SAFETY: `jihaz` is live and the description is a fully initialised
        // local the device copies out of before returning.
        let kawmat_ahdaf: ID3D12DescriptorHeap =
            unsafe { self.jihaz.CreateDescriptorHeap(&raw const wasf_ahdaf) }.map_err(
                |khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay render target descriptor heap",
                    sabab: khata.to_string(),
                },
            )?;

        // SAFETY: `jihaz` is live; this reads a hardware constant and has no
        // preconditions beyond the device existing.
        let khatwa = unsafe {
            self.jihaz.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV)
        };

        let wasf_wasfiyat = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
            NumDescriptors: 1,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
            NodeMask: 0,
        };
        // SAFETY: as above.
        let kawmat_wasf: ID3D12DescriptorHeap =
            unsafe { self.jihaz.CreateDescriptorHeap(&raw const wasf_wasfiyat) }.map_err(
                |khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay shader descriptor heap",
                    sabab: khata.to_string(),
                },
            )?;

        self.kawmat_ahdaf = Some(kawmat_ahdaf);
        self.kawmat_wasf = Some(kawmat_wasf);
        self.khatwat_hadaf = usize::try_from(khatwa).unwrap_or(0);
        Ok(())
    }

    /// Creates an upload-heap buffer of the given vertex capacity.
    ///
    /// Returns the resource and its GPU virtual address, which is read once here
    /// rather than at every draw: `GetGPUVirtualAddress` is a virtual call into
    /// the driver and it cannot change for the life of a committed resource.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the device refuses the allocation.
    fn ibni_ruus(&self, adad_ruus: usize) -> Result<(ID3D12Resource, u64), KhataTabaqa> {
        let tul = tul_u64(adad_ruus).saturating_mul(u64::from(KHATWAT_RAS)).max(1);
        let khasais = khasais_rafa();
        let wasf = wasf_mukhazzan(tul);
        let mut mawrid: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals, there is no optimized clear
        // value for a buffer, and the out-pointer addresses a local initialised
        // to `None`. `GENERIC_READ` is the only state an upload-heap resource
        // may be created in and the only one it may ever be in.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &raw mut mawrid,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay vertex buffer",
            sabab: khata.to_string(),
        })?;
        let Some(mawrid) = mawrid else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay vertex buffer",
                sabab: "the device reported success and produced no resource".to_owned(),
            });
        };
        // SAFETY: `mawrid` is a live committed buffer resource, which is the one
        // case where this address is defined and stable.
        let masar = unsafe { mawrid.GetGPUVirtualAddress() };
        Ok((mawrid, masar))
    }

    /// Creates the per-backbuffer resources and their render target views.
    ///
    /// One allocator and one list per backbuffer, not one of each shared. A
    /// single allocator reset while the GPU is still reading the list it holds
    /// is the classic D3D12 corruption, and it does not reproduce on a fast
    /// machine — the frame finishes before the next reset — so it ships. Having
    /// one per frame plus a fence value per frame means the mistake cannot be
    /// made at all.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::SathTaghayyar`] when a backbuffer cannot be fetched, which
    /// is what a swap chain mid-resize reports, and
    /// [`KhataTabaqa::MawridFashil`] for everything else.
    fn ibni_itarat(&mut self, adad_ilqa: u32) -> Result<(), KhataTabaqa> {
        let Some(kawma) = self.kawmat_ahdaf.as_ref() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay frame resources",
                sabab: "the frames were asked for before their descriptor heap existed"
                    .to_owned(),
            });
        };
        // SAFETY: `kawma` is a live descriptor heap; this reads its base handle,
        // which is a plain address and is copied out immediately so the borrow
        // does not have to outlive the loop below.
        let asas = unsafe { kawma.GetCPUDescriptorHandleForHeapStart() };

        let mut itarat = Vec::with_capacity(usize::try_from(adad_ilqa).unwrap_or(0));
        for fahras in 0..adad_ilqa {
            // SAFETY: `silsila` is live and `fahras` is below the buffer count
            // the swap chain itself reported. `GetBuffer` QueryInterfaces the
            // backbuffer and returns an owned reference or an error.
            let khalfiya: ID3D12Resource = unsafe { self.silsila.GetBuffer(fahras) }.map_err(
                |khata| KhataTabaqa::SathTaghayyar {
                    sabab: format!("backbuffer {fahras} could not be fetched: {khata}"),
                },
            )?;

            let izaha = usize::try_from(fahras).unwrap_or(0).saturating_mul(self.khatwat_hadaf);
            let mawdi_hadaf = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: asas.ptr.saturating_add(izaha) };
            // SAFETY: `khalfiya` is a live texture, the null description asks
            // the runtime to take the resource's own format, and `mawdi_hadaf`
            // is inside the heap created for exactly `adad_ilqa` descriptors.
            unsafe { self.jihaz.CreateRenderTargetView(&khalfiya, None, mawdi_hadaf) };

            // SAFETY: `jihaz` is live. Both calls return owned interfaces or
            // errors; `CreateCommandList` hands back a list already in the
            // recording state, which is why it is closed immediately below —
            // every later use begins with `Reset`, and `Reset` on an open list
            // is refused.
            let mukhassis: ID3D12CommandAllocator =
                unsafe { self.jihaz.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }
                    .map_err(|khata| KhataTabaqa::MawridFashil {
                        mawrid: "overlay command allocator",
                        sabab: khata.to_string(),
                    })?;
            // SAFETY: `mukhassis` is the allocator created immediately above and
            // is alive for the call; no initial pipeline state is named because
            // the pipeline is set at every `Reset`.
            let qaima: ID3D12GraphicsCommandList = unsafe {
                self.jihaz.CreateCommandList(
                    0,
                    D3D12_COMMAND_LIST_TYPE_DIRECT,
                    &mukhassis,
                    None,
                )
            }
            .map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay command list",
                sabab: khata.to_string(),
            })?;
            // SAFETY: `qaima` is the list created immediately above and is open.
            unsafe { qaima.Close() }.map_err(|khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay command list",
                sabab: format!("the freshly created list would not close: {khata}"),
            })?;

            let (ruus, masar_ruus) = self.ibni_ruus(RUUS_IBTIDAIYA)?;
            itarat.push(ItarD3D12 {
                khalfiya,
                mawdi_hadaf,
                zawj: ZawjAwamir { mukhassis, qaima },
                ruus,
                masar_ruus,
                siat_ruus: RUUS_IBTIDAIYA,
                // Nothing has been submitted for this frame yet, and zero is how
                // `intazir` is told so.
                qeema: 0,
            });
        }

        self.itarat = itarat;
        Ok(())
    }

    /// Creates the shared index buffer.
    ///
    /// One pattern of six indices per quad, repeated [`QITA_LIL_DUFA`] times,
    /// uploaded once and never touched again. Thirty-two bit indices rather than
    /// the sixteen the D3D11 backend uses: this buffer is written once instead
    /// of every frame, so the bandwidth argument that makes sixteen bits worth
    /// it there does not apply here, and thirty-two removes the only ceiling
    /// that could ever silently truncate a chunk.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the buffer cannot be created or
    /// written.
    fn ibni_faharis(&mut self) -> Result<(), KhataTabaqa> {
        let mut khaam: Vec<u32> = Vec::with_capacity(QITA_LIL_DUFA.saturating_mul(6));
        for qita in 0..QITA_LIL_DUFA {
            let asas = u32::try_from(qita.saturating_mul(4)).unwrap_or(0);
            khaam.extend_from_slice(&[
                asas,
                asas.saturating_add(1),
                asas.saturating_add(2),
                asas,
                asas.saturating_add(2),
                asas.saturating_add(3),
            ]);
        }

        let tul = tul_u64(khaam.len()).saturating_mul(4).max(1);
        let khasais = khasais_rafa();
        let wasf = wasf_mukhazzan(tul);
        let mut mawrid: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals and the out-pointer
        // addresses a local initialised to `None`.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &raw mut mawrid,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay index buffer",
            sabab: khata.to_string(),
        })?;
        let Some(mawrid) = mawrid else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay index buffer",
                sabab: "the device reported success and produced no resource".to_owned(),
            });
        };

        // SAFETY: `khaam` is a live `Vec<u32>`; the pointer and length describe
        // exactly its allocation reinterpreted as bytes, which is sound for a
        // type with no padding and no invalid bit patterns.
        let bayt = unsafe {
            core::slice::from_raw_parts(
                khaam.as_ptr().cast::<u8>(),
                khaam.len().saturating_mul(4),
            )
        };
        iktub_fi(&mawrid, bayt, "overlay index buffer")?;

        self.faharis = Some(mawrid);
        Ok(())
    }

    /// Creates the projection constant buffer and writes it once.
    ///
    /// Written once because the projection only depends on the surface, and the
    /// surface changing is what brings the whole of `hayyi` round again. A
    /// per-frame rewrite would be a per-frame write to memory a queued draw may
    /// still be reading, which is the precise hazard this backend is arranged
    /// around.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the buffer cannot be created or
    /// written.
    fn ibni_thawabit(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        // A root constant buffer view's address has to be 256-byte aligned, and
        // committed resources are far more aligned than that — but the *size*
        // is what the runtime validates, so it is rounded here.
        let tul = hadhi(
            tul_u64(mem::size_of::<ThawabitIsqat>()),
            u64::from(D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT),
        );
        let khasais = khasais_rafa();
        let wasf = wasf_mukhazzan(tul);
        let mut mawrid: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals and the out-pointer
        // addresses a local initialised to `None`.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &raw mut mawrid,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay projection constant buffer",
            sabab: khata.to_string(),
        })?;
        let Some(mawrid) = mawrid else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay projection constant buffer",
                sabab: "the device reported success and produced no resource".to_owned(),
            });
        };

        let thawabit = ThawabitIsqat::min_sath(sath);
        // SAFETY: `thawabit` is a live local of exactly the size being read, and
        // `ThawabitIsqat` is a `#[repr(C)]` struct of `f32` arrays, which has no
        // padding and no invalid bit patterns.
        let bayt = unsafe {
            core::slice::from_raw_parts(
                (&raw const thawabit).cast::<u8>(),
                mem::size_of::<ThawabitIsqat>(),
            )
        };
        iktub_fi(&mawrid, bayt, "overlay projection constant buffer")?;

        // SAFETY: `mawrid` is a live committed buffer.
        self.masar_thawabit = unsafe { mawrid.GetGPUVirtualAddress() };
        self.thawabit = Some(mawrid);
        Ok(())
    }

    /// Creates the allocator and list used for work that is not on the frame
    /// cadence.
    ///
    /// The atlas upload and the frame capture both need a command list, and both
    /// happen at moments unrelated to which backbuffer is current. Borrowing a
    /// frame's pair for them would mean a capture that outlives the frame resets
    /// an allocator the GPU is still reading — the same fault, arrived at from a
    /// different direction.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when either object cannot be created.
    fn ibni_naql(&mut self) -> Result<(), KhataTabaqa> {
        if self.naql.is_some() {
            return Ok(());
        }
        // SAFETY: `jihaz` is live and returns an owned interface or an error.
        let mukhassis: ID3D12CommandAllocator =
            unsafe { self.jihaz.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT) }.map_err(
                |khata| KhataTabaqa::MawridFashil {
                    mawrid: "overlay transfer allocator",
                    sabab: khata.to_string(),
                },
            )?;
        // SAFETY: `mukhassis` is the allocator created immediately above.
        let qaima: ID3D12GraphicsCommandList = unsafe {
            self.jihaz.CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &mukhassis, None)
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay transfer command list",
            sabab: khata.to_string(),
        })?;
        // SAFETY: `qaima` is open, as every freshly created list is.
        unsafe { qaima.Close() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay transfer command list",
            sabab: format!("the freshly created list would not close: {khata}"),
        })?;
        self.naql = Some(ZawjAwamir { mukhassis, qaima });
        Ok(())
    }

    /// Writes the atlas descriptor into the shader-visible heap.
    ///
    /// Does nothing when either the heap or the atlas is missing, which is not a
    /// failure: the atlas can be uploaded before the first `hayyi` and the heap
    /// can be rebuilt after one, so both orders happen and both are recovered by
    /// whichever of the two runs second calling this again.
    ///
    /// # Errors
    ///
    /// Infallible in practice — `CreateShaderResourceView` returns nothing — so
    /// this returns a result only so the two call sites read the same as their
    /// neighbours.
    fn sajjil_ru2yat_lawha(&self) -> Result<(), KhataTabaqa> {
        let (Some(kawma), Some(lawha)) = (self.kawmat_wasf.as_ref(), self.lawha.as_ref()) else {
            return Ok(());
        };
        let wasf = D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                    PlaneSlice: 0,
                    ResourceMinLODClamp: 0.0,
                },
            },
        };
        // SAFETY: `kawma` is a live heap with one descriptor and `lawha` is the
        // live atlas texture the description matches. The handle is the heap's
        // own base, so it is inside the heap by construction.
        unsafe {
            let mawdi = kawma.GetCPUDescriptorHandleForHeapStart();
            self.jihaz.CreateShaderResourceView(lawha, Some(&raw const wasf), mawdi);
        }
        Ok(())
    }

    /// Grows one frame's vertex buffer to hold a batch, if it does not already.
    ///
    /// Safe only because the caller has already waited on that frame's fence:
    /// replacing the buffer drops the previous one, and dropping a resource a
    /// queued draw still reads is the fault this whole file is arranged to
    /// prevent. The wait is the precondition, and it is the caller's because the
    /// caller is doing it anyway before it resets the allocator.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the larger buffer cannot be allocated.
    fn wassi_ruus(&mut self, fahras: usize, matlub: usize) -> Result<(), KhataTabaqa> {
        let hali = self.itarat.get(fahras).map_or(0, |itar| itar.siat_ruus);
        if hali >= matlub {
            return Ok(());
        }
        // Rounded up to a power of two so a batch that grows by one glyph a
        // frame does not reallocate on every one of them.
        let jadeed = matlub.checked_next_power_of_two().unwrap_or(matlub).max(RUUS_IBTIDAIYA);
        let (ruus, masar_ruus) = self.ibni_ruus(jadeed)?;
        if let Some(itar) = self.itarat.get_mut(fahras) {
            itar.ruus = ruus;
            itar.masar_ruus = masar_ruus;
            itar.siat_ruus = jadeed;
        }
        Ok(())
    }

    /// Closes a list, submits it on the captured queue, and signals the fence.
    ///
    /// The three always happen together and always in this order. Submitting
    /// without signalling would leave nothing for [`KhattafD3D12::intazir`] to
    /// wait on, which is the same as having no synchronisation at all while
    /// looking like there is some.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when the list will not close, which happens
    /// before anything is submitted and leaves nothing running, and
    /// [`KhataTabaqa::HalaGhayrMustaada`] when the queue will not signal *after*
    /// a submit, which leaves work running that nothing can wait on.
    fn nafidh(&mut self, qaima: &ID3D12GraphicsCommandList) -> Result<u64, KhataTabaqa> {
        // SAFETY: `qaima` is a live list that was reset and recorded into.
        unsafe { qaima.Close() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay command list",
            sabab: format!("the recorded list would not close: {khata}"),
        })?;

        let qawaim = [Some(ID3D12CommandList::from(qaima.clone()))];
        // SAFETY: `saff` is the live queue the hook captured, and `qawaim` holds
        // one closed list belonging to the same device. The array outlives the
        // call; the queue does not retain the borrow past it.
        unsafe { self.saff.ExecuteCommandLists(&qawaim) };

        // Past this point work is running on the GPU. A signal that does not
        // happen leaves that work with nothing tracking it, so every later reset
        // and every later release would be guesswork — which is precisely the
        // condition `khata.rs` names as terminal.
        self.ashir().map_err(|khata| KhataTabaqa::HalaGhayrMustaada {
            hala: "the overlay's fence-tracked resource lifetimes",
            sabab: format!(
                "a command list was submitted and the queue would not signal the fence \
                 that tracks it: {khata}"
            ),
        })
    }
}

/// How many vertices a frame's buffer starts out able to hold.
///
/// One full chunk. Batches larger than this grow the buffer; batches smaller
/// than it — which is every batch a dialogue box produces — never allocate.
const RUUS_IBTIDAIYA: usize = 16_384;

// Four vertices per quad, and the initial capacity must cover one whole chunk or
// the very first frame reallocates.
const _: () = {
    assert!(RUUS_IBTIDAIYA == QITA_LIL_DUFA * 4, "the initial vertex capacity is wrong");
};

/// Copies bytes into an upload-heap resource.
///
/// The read range is empty because nothing here ever reads what is already
/// there, and telling the runtime so is what lets it skip a cache invalidate on
/// the architectures where that costs something. `Unmap` is given a null written
/// range, which means "assume everything changed" — the honest answer, since the
/// whole buffer was just overwritten.
///
/// # Errors
///
/// [`KhataTabaqa::MawridFashil`] when the map is refused or returns no pointer.
fn iktub_fi(mawrid: &ID3D12Resource, bayt: &[u8], mawdu: &'static str) -> Result<(), KhataTabaqa> {
    let mut hadaf: *mut core::ffi::c_void = core::ptr::null_mut();
    let mada = D3D12_RANGE { Begin: 0, End: 0 };
    // SAFETY: `mawrid` is a live upload-heap buffer, which is the only resource
    // kind `Map` accepts here; subresource zero is the whole buffer, and both
    // pointers address live locals.
    unsafe { mawrid.Map(0, Some(&raw const mada), Some(&raw mut hadaf)) }.map_err(|khata| {
        KhataTabaqa::MawridFashil { mawrid: mawdu, sabab: format!("the map was refused: {khata}") }
    })?;

    if hadaf.is_null() {
        // SAFETY: the map succeeded, so the resource is mapped and must be
        // unmapped even though the pointer it produced is unusable.
        unsafe { mawrid.Unmap(0, None) };
        return Err(KhataTabaqa::MawridFashil {
            mawrid: mawdu,
            sabab: "the map succeeded and returned no pointer".to_owned(),
        });
    }

    // SAFETY: the buffer was created with at least `bayt.len()` bytes by every
    // caller — each computes the length it passes here from the same value it
    // sized the resource with — and the mapping covers the whole subresource.
    // Source and destination are distinct allocations.
    unsafe {
        core::ptr::copy_nonoverlapping(bayt.as_ptr(), hadaf.cast::<u8>(), bayt.len());
        mawrid.Unmap(0, None);
    }
    Ok(())
}

/// A blob's contents as a printable string.
///
/// Used only for the compiler's and serializer's diagnostics, which are ASCII
/// with a trailing NUL. Read lossily rather than validated, because a message
/// that cannot be decoded is still worth showing.
fn nass_kutla(kutla: &ID3DBlob) -> String {
    // SAFETY: `kutla` is a live blob; the pointer and length it reports describe
    // its own allocation and are read only for the duration of this borrow.
    let nass = unsafe {
        let asas = kutla.GetBufferPointer().cast::<u8>();
        let tul = kutla.GetBufferSize();
        if asas.is_null() || tul == 0 {
            String::new()
        } else {
            String::from_utf8_lossy(core::slice::from_raw_parts(asas, tul)).into_owned()
        }
    };
    nass.trim_end_matches('\0').trim().to_owned()
}

impl Khattaf for KhattafD3D12 {
    fn wajiha(&self) -> WajihatRusum {
        WajihatRusum::Direct3D12
    }

    /// Waits for the GPU to finish with every frame's resources, then drops
    /// the backbuffer references the swap chain is about to invalidate.
    ///
    /// The wait is the part that cannot be skipped. `ResizeBuffers` will
    /// destroy the backbuffers the moment the last reference goes, and a
    /// command list still reading one of them is a device removal a frame
    /// later with a stack that names nothing.
    fn atliq_sath(&mut self) -> Result<(), KhataTabaqa> {
        self.atliq_khalfiyat()
    }

    fn sath(&self) -> Result<WasfSath, KhataTabaqa> {
        // SAFETY: `silsila` is the live swap chain this backend was built
        // around; `GetDesc` fills a stack description the binding owns.
        let wasf = unsafe { self.silsila.GetDesc() }.map_err(|khata| {
            KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain would not describe itself: {khata}"),
            }
        })?;

        let ard = wasf.BufferDesc.Width;
        let irtifa = wasf.BufferDesc.Height;
        if ard == 0 || irtifa == 0 {
            return Err(KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain reports a {ard}×{irtifa} surface"),
            });
        }

        let (sigha, sirgb) = sigha_min_dxgi(wasf.BufferDesc.Format)?;
        Ok(WasfSath { ard, irtifa, sigha, sirgb })
    }

    fn hayyi(&mut self, sath: WasfSath) -> Result<(), KhataTabaqa> {
        // The queue is drained before anything is released, and the release
        // happens before anything is allocated. Both halves matter: the frames
        // being dropped hold command lists the GPU may still be executing, and
        // the render target views being dropped are views of backbuffers DXGI is
        // about to free out from under them.
        self.intazir_khumul(MUHLAT_INTIZAR)?;
        self.atlif_masar();

        self.ibni_hajiz()?;

        // SAFETY: `silsila` is live and `GetDesc` fills a stack description.
        let wasf = unsafe { self.silsila.GetDesc() }.map_err(|khata| {
            KhataTabaqa::SathTaghayyar {
                sabab: format!("the swap chain would not describe itself: {khata}"),
            }
        })?;
        let adad_ilqa = wasf.BufferCount.max(1);

        self.ihdar_ramz()?;
        self.ibni_tawqee()?;
        // The pipeline is built against the swap chain's own format rather than
        // against `sath.sigha`, which has already lost the `_SRGB` suffix. A
        // pipeline whose render target format disagrees with the view bound to
        // it is refused at draw time, once per frame, inside somebody's game.
        self.ibni_halat_rasm(wasf.BufferDesc.Format)?;
        self.ibni_kawmat(adad_ilqa)?;
        self.ibni_itarat(adad_ilqa)?;
        self.ibni_faharis()?;
        self.ibni_thawabit(sath)?;
        self.ibni_naql()?;
        // Re-registered because the heap it lived in was just replaced. An atlas
        // uploaded before this resize would otherwise be a texture with no
        // descriptor, which draws as whatever the heap slot happens to hold.
        self.sajjil_ru2yat_lawha()?;

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
        let khatwa_masdar = ard.saturating_mul(4);
        let matlub = u64::from(khatwa_masdar).saturating_mul(u64::from(irtifa));
        if tul_u64(bayt.len()) < matlub {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: format!(
                    "a {ard}×{irtifa} RGBA8 atlas needs {matlub} bytes and {} were supplied",
                    bayt.len()
                ),
            });
        }

        // The atlas being replaced may be bound in a list the GPU has not
        // finished. Nothing here is allowed to run until it has.
        self.ibni_hajiz()?;
        self.intazir_khumul(MUHLAT_INTIZAR)?;
        self.ibni_naql()?;

        let wasf_lawha = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: u64::from(ard),
            Height: irtifa,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };
        let khasais = khasais_iftiradi();
        let mut lawha: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals, a texture with no render
        // target or depth flags takes no optimized clear value, and the
        // out-pointer addresses a local initialised to `None`. `COPY_DEST` is
        // the state the upload below expects to find it in.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf_lawha,
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &raw mut lawha,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas texture",
            sabab: khata.to_string(),
        })?;
        let Some(lawha) = lawha else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "the device reported success and produced no texture".to_owned(),
            });
        };

        let mut takhtit = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut adad_sufuf: u32 = 0;
        let mut tul_saf: u64 = 0;
        let mut al_kul: u64 = 0;
        // SAFETY: `wasf_lawha` is a live local describing the texture that was
        // just created from it, subresource zero is its only one, and all four
        // out-pointers address live locals.
        unsafe {
            self.jihaz.GetCopyableFootprints(
                &raw const wasf_lawha,
                0,
                1,
                0,
                Some(&raw mut takhtit),
                Some(&raw mut adad_sufuf),
                Some(&raw mut tul_saf),
                Some(&raw mut al_kul),
            );
        }
        if al_kul == 0 || takhtit.Footprint.RowPitch == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: "the device described a zero-sized copy footprint".to_owned(),
            });
        }

        // The staging copy is repacked from the caller's tight rows into the
        // driver's aligned ones. `RowPitch` is a multiple of 256 and the atlas
        // width almost never is, so a straight `copy_from_slice` of the whole
        // buffer would put every row after the first at the wrong offset — the
        // glyphs would still be there and every one of them would be sheared.
        let khatwa_hadaf = usize::try_from(takhtit.Footprint.RowPitch).unwrap_or(0);
        let khatwa_masdar = usize::try_from(khatwa_masdar).unwrap_or(0);
        let irtifa_usize = usize::try_from(irtifa).unwrap_or(0);
        if khatwa_hadaf < khatwa_masdar || khatwa_masdar == 0 {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas texture",
                sabab: format!(
                    "the device asked for a {khatwa_hadaf}-byte row pitch for a \
                     {khatwa_masdar}-byte row"
                ),
            });
        }
        let mut mubattan = vec![0_u8; usize::try_from(al_kul).unwrap_or(0)];
        for (saf, masdar) in bayt.chunks_exact(khatwa_masdar).take(irtifa_usize).enumerate() {
            let bidaya = saf.saturating_mul(khatwa_hadaf);
            let Some(hadaf) = mubattan.get_mut(bidaya..bidaya.saturating_add(khatwa_masdar))
            else {
                return Err(KhataTabaqa::MawridFashil {
                    mawrid: "glyph atlas texture",
                    sabab: "the copy footprint is smaller than the atlas it must hold".to_owned(),
                });
            };
            hadaf.copy_from_slice(masdar);
        }

        let khasais_rafaa = khasais_rafa();
        let wasf_rafaa = wasf_mukhazzan(al_kul);
        let mut rafaa: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals and the out-pointer
        // addresses a local initialised to `None`.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais_rafaa,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf_rafaa,
                D3D12_RESOURCE_STATE_GENERIC_READ,
                None,
                &raw mut rafaa,
            )
        }
        .map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "glyph atlas upload buffer",
            sabab: khata.to_string(),
        })?;
        let Some(rafaa) = rafaa else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas upload buffer",
                sabab: "the device reported success and produced no buffer".to_owned(),
            });
        };
        iktub_fi(&rafaa, &mubattan, "glyph atlas upload buffer")?;

        let Some(naql) = self.naql.as_ref() else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay transfer command list",
                sabab: "the transfer pair was not built".to_owned(),
            });
        };
        let mukhassis = naql.mukhassis.clone();
        let qaima = naql.qaima.clone();

        // SAFETY: the queue was drained above, so no list this allocator holds
        // is executing and the reset is defined. The list is reset onto the same
        // allocator it was created with.
        unsafe { mukhassis.Reset() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay transfer allocator",
            sabab: khata.to_string(),
        })?;
        // SAFETY: `qaima` is closed — every path that opens it closes it before
        // returning — and `mukhassis` was just reset.
        unsafe { qaima.Reset(&mukhassis, None) }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay transfer command list",
            sabab: khata.to_string(),
        })?;

        let hadaf_naskh = D3D12_TEXTURE_COPY_LOCATION {
            pResource: muara(&lawha),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
        };
        let masdar_naskh = D3D12_TEXTURE_COPY_LOCATION {
            pResource: muara(&rafaa),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { PlacedFootprint: takhtit },
        };
        let hajiz = hajiz_intiqal(
            &lawha,
            D3D12_RESOURCE_STATE_COPY_DEST,
            D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        );

        // SAFETY: `lawha` and `rafaa` are live for this whole block, which is
        // what the non-owning `pResource` copies inside the two locations and
        // the barrier require. The footprint is the one the device itself
        // produced for this texture, and the upload buffer was filled to exactly
        // its total byte count.
        unsafe {
            qaima.CopyTextureRegion(&raw const hadaf_naskh, 0, 0, 0, &raw const masdar_naskh, None);
            qaima.ResourceBarrier(&[hajiz]);
        }

        let qeema = self.nafidh(&qaima)?;
        // Waited on before `rafaa` goes out of scope. Dropping the upload buffer
        // while the copy is still queued is exactly the fault this backend is
        // built to avoid, and it is one `?` away from happening here.
        self.intazir_qabl_massa(qeema, MUHLAT_INTIZAR)?;

        self.lawha = Some(lawha);
        self.sajjil_ru2yat_lawha()
    }

    /// Records and submits the overlay's own command list.
    ///
    /// There is no state to save and restore. A D3D12 command list is a private
    /// recording: the binds below go into the overlay's list, the game's lists
    /// are untouched, and nothing this method does can leave the game's pipeline
    /// describing Taarib's buffers. That is the whole of what D3D12 gives back
    /// for what it takes away, and it is why this method is a third the length
    /// of its D3D11 counterpart.
    ///
    /// What it takes away is the runtime's resource lifetime tracking. The
    /// hazards here are all the same shape: reset an allocator whose list is
    /// still executing, overwrite a vertex buffer a queued draw still reads,
    /// release a texture the GPU has not finished with. None of them fault
    /// immediately, all of them corrupt something, and the defence is the fence
    /// wait at the top of this method — frame *n* is not touched until the fence
    /// says frame *n* finished.
    ///
    /// That is also where [`KhataTabaqa::HalaGhayrMustaada`] comes from on this
    /// backend. It does not mean "state was left wrong"; it means the fence did
    /// not confirm that frame *n* had finished and the overlay was about to
    /// reset its allocator anyway. `crate::khata` lists that exact case as
    /// terminal, and the caller disabling the overlay for the session is the
    /// only response that does not compound it.
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
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay batch",
                sabab: "the batch was positioned against a surface this backend is not \
                        currently built for"
                    .to_owned(),
            });
        }
        if lawha.qitaat.len() > SAQF_QITA {
            // `HajmMufrit` would be the more precise variant and it is not the
            // one the trait documents for this method. The number is put in the
            // message instead, so nothing is lost but the code.
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay batch",
                sabab: format!(
                    "a batch of {} quads is above this build's ceiling of {SAQF_QITA}",
                    lawha.qitaat.len()
                ),
            });
        }

        let (Some(tawqee), Some(halat), Some(kawma), Some(faharis)) = (
            self.tawqee.clone(),
            self.halat_rasm.clone(),
            self.kawmat_wasf.clone(),
            self.faharis.clone(),
        ) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay pipeline",
                sabab: "the backend was asked to draw before its resources were built".to_owned(),
            });
        };
        // Unconditional here, unlike the D3D11 backend, and for a reason that is
        // D3D12's alone: the root signature declares a descriptor table the
        // pixel shader reads, every root parameter a shader references has to be
        // bound before a draw, and the descriptor in that table has to have been
        // created. There is no "no atlas" draw on this API — only a heap slot
        // holding whatever was there before, which is exactly the read the debug
        // layer calls undefined and the hardware calls whatever it likes.
        if self.lawha.is_none() {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "glyph atlas",
                sabab: "the D3D12 backend cannot draw before an atlas has been uploaded"
                    .to_owned(),
            });
        }

        // SAFETY: `silsila` is live; this reads an index and has no
        // preconditions beyond the swap chain existing.
        let fahras = unsafe { self.silsila.GetCurrentBackBufferIndex() };
        let fahras = usize::try_from(fahras).unwrap_or(0);

        // Everything below this line reuses frame `fahras`. Nothing below this
        // line is safe until the wait has returned.
        let qeema_sabiqa = self.itarat.get(fahras).map_or(0, |itar| itar.qeema);
        self.intazir_qabl_massa(qeema_sabiqa, MUHLAT_INTIZAR)?;

        let matlub = lawha.qitaat.len().saturating_mul(4);
        self.wassi_ruus(fahras, matlub)?;

        let Some(itar) = self.itarat.get(fahras) else {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay frame resources",
                sabab: format!("the swap chain named backbuffer {fahras}, which has no frame"),
            });
        };
        let mukhassis = itar.zawj.mukhassis.clone();
        let qaima = itar.zawj.qaima.clone();
        let ruus = itar.ruus.clone();
        let masar_ruus = itar.masar_ruus;
        let siat_ruus = itar.siat_ruus;
        let mawdi_hadaf = itar.mawdi_hadaf;
        let khalfiya = itar.khalfiya.clone();

        let mut musawwada = mem::take(&mut self.musawwada);
        musawwada.clear();
        for qita in &lawha.qitaat {
            imla_qita(&mut musawwada, qita);
        }
        let natija = self.arsim_bi(
            &musawwada,
            lawha,
            &MawaridItar {
                mukhassis: &mukhassis,
                qaima: &qaima,
                ruus: &ruus,
                masar_ruus,
                siat_ruus,
                mawdi_hadaf,
                khalfiya: &khalfiya,
                tawqee: &tawqee,
                halat: &halat,
                kawma: &kawma,
                faharis: &faharis,
            },
        );
        musawwada.clear();
        self.musawwada = musawwada;

        let qeema = natija?;
        if let Some(itar) = self.itarat.get_mut(fahras) {
            itar.qeema = qeema;
        }
        Ok(())
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

        self.ibni_hajiz()?;
        self.ibni_naql()?;

        // SAFETY: `silsila` is live and the index it reports is its own.
        let fahras = unsafe { self.silsila.GetCurrentBackBufferIndex() };
        // SAFETY: `silsila` is live and `fahras` is the index it just named.
        let khalfiya: ID3D12Resource = unsafe { self.silsila.GetBuffer(fahras) }.map_err(
            |khata| KhataTabaqa::IltiqatFashil {
                sabab: format!("backbuffer {fahras} could not be fetched: {khata}"),
            },
        )?;
        // SAFETY: `khalfiya` is a live resource; `GetDesc` returns a value.
        let wasf_khalfiya = unsafe { khalfiya.GetDesc() };

        // The footprint is asked for against a texture the size of the *region*,
        // not the size of the backbuffer, so the readback buffer is only as
        // large as what is being captured. A region capture that allocated a
        // whole 4K frame every time would be a region capture in name only.
        let wasf_mintaqa = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: u64::from(mintaqa.ard),
            Height: mintaqa.irtifa,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: wasf_khalfiya.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };
        let mut takhtit = D3D12_PLACED_SUBRESOURCE_FOOTPRINT::default();
        let mut adad_sufuf: u32 = 0;
        let mut tul_saf: u64 = 0;
        let mut al_kul: u64 = 0;
        // SAFETY: `wasf_mintaqa` is a live local, subresource zero is its only
        // one, and all four out-pointers address live locals.
        unsafe {
            self.jihaz.GetCopyableFootprints(
                &raw const wasf_mintaqa,
                0,
                1,
                0,
                Some(&raw mut takhtit),
                Some(&raw mut adad_sufuf),
                Some(&raw mut tul_saf),
                Some(&raw mut al_kul),
            );
        }
        if al_kul == 0 {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the device described a zero-sized copy footprint".to_owned(),
            });
        }

        let khasais = khasais_qira();
        let wasf_qira = wasf_mukhazzan(al_kul);
        let mut qira: Option<ID3D12Resource> = None;
        // SAFETY: both descriptions are live locals and the out-pointer
        // addresses a local initialised to `None`. `COPY_DEST` is the only
        // state a readback-heap resource may be created in.
        unsafe {
            self.jihaz.CreateCommittedResource(
                &raw const khasais,
                D3D12_HEAP_FLAG_NONE,
                &raw const wasf_qira,
                D3D12_RESOURCE_STATE_COPY_DEST,
                None,
                &raw mut qira,
            )
        }
        .map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back buffer could not be created: {khata}"),
        })?;
        let Some(qira) = qira else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the read-back buffer was not produced".to_owned(),
            });
        };

        let Some(naql) = self.naql.as_ref() else {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: "the transfer pair was not built".to_owned(),
            });
        };
        let mukhassis = naql.mukhassis.clone();
        let qaima = naql.qaima.clone();

        // The transfer pair may still be executing an earlier capture or an
        // atlas upload. Resetting its allocator before that finished is the
        // corruption; waiting on the last value this backend signalled is the
        // cheapest thing that cannot be wrong.
        self.intazir_qabl_massa(self.qeemat_hajiz, MUHLAT_INTIZAR)?;

        // SAFETY: the wait above returned, so nothing this allocator holds is
        // executing. The list is reset onto the allocator it was created with.
        unsafe { mukhassis.Reset() }.map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the transfer allocator would not reset: {khata}"),
        })?;
        // SAFETY: `qaima` is closed on every path that touches it.
        unsafe { qaima.Reset(&mukhassis, None) }.map_err(|khata| KhataTabaqa::IltiqatFashil {
            sabab: format!("the transfer list would not reset: {khata}"),
        })?;

        let ila_masdar = hajiz_intiqal(
            &khalfiya,
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
        );
        let ila_ard = hajiz_intiqal(
            &khalfiya,
            D3D12_RESOURCE_STATE_COPY_SOURCE,
            D3D12_RESOURCE_STATE_PRESENT,
        );
        let hadaf_naskh = D3D12_TEXTURE_COPY_LOCATION {
            pResource: muara(&qira),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { PlacedFootprint: takhtit },
        };
        let masdar_naskh = D3D12_TEXTURE_COPY_LOCATION {
            pResource: muara(&khalfiya),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
        };
        let sunduq = D3D12_BOX {
            left: mintaqa.yasar,
            top: mintaqa.aala,
            front: 0,
            right: hudud_ard,
            bottom: hudud_irtifa,
            back: 1,
        };

        // SAFETY: `khalfiya` and `qira` outlive this block, which is what the
        // non-owning `pResource` copies in the barriers and copy locations
        // require. The backbuffer is in `PRESENT` because the hook runs after
        // the game finished its own frame, and it is put back before the list is
        // submitted — presenting a resource in `COPY_SOURCE` is a device removal.
        // The box was checked against the surface at the head of this function.
        unsafe {
            qaima.ResourceBarrier(&[ila_masdar]);
            qaima.CopyTextureRegion(
                &raw const hadaf_naskh,
                0,
                0,
                0,
                &raw const masdar_naskh,
                Some(&raw const sunduq),
            );
            qaima.ResourceBarrier(&[ila_ard]);
        }

        let qeema = self.nafidh(&qaima)?;
        // Terminal on failure for the same reason as everywhere else: the next
        // statement reads the buffer the copy writes into, and the one after
        // that drops it.
        self.intazir_qabl_massa(qeema, MUHLAT_INTIZAR)?;

        jami_sufuf(&qira, &takhtit, mintaqa, sath.sigha, al_kul)
    }

    fn ahmil(&mut self) -> Result<(), KhataTabaqa> {
        let mut khata: Option<KhataTabaqa> = None;

        if self.intazir_khumul(MUHLAT_KHUMUL).is_err() {
            // SAFETY: `jihaz` is the live device this backend was built around.
            let hayy = unsafe { self.jihaz.GetDeviceRemovedReason() }.is_ok();
            if hayy {
                // The device is alive and the queue still has not drained, which
                // means something this backend submitted is genuinely still
                // running. Freeing its memory now is the one thing that must not
                // happen, so it is given a second, longer wait before this gives
                // up and says so.
                if let Err(thani) = self.intazir(self.qeemat_hajiz, MUHLAT_KHUMUL) {
                    khata = Some(thani);
                }
            }
            // A removed device is the ordinary case on this path: nothing is
            // executing on it, every resource that lived on it is already
            // invalid, and holding references would pin this module inside the
            // game for the rest of the process's life.
        }

        // Idempotent, and safe to reach with nothing built — which is the common
        // case, because a backend whose `hayyi` failed is exactly the backend
        // that gets torn down.
        self.atlif_masar();
        self.lawha = None;
        self.hajiz = None;
        self.hadath = None;
        self.qeemat_hajiz = 0;
        self.bayt_ras = Vec::new();
        self.bayt_biksel = Vec::new();
        self.musawwada = Vec::new();

        khata.map_or(Ok(()), Err)
    }
}

impl Drop for KhattafD3D12 {
    fn drop(&mut self) {
        // A backend dropped without `ahmil` having been called would release
        // command lists, upload buffers and an atlas with no idea whether the
        // GPU had finished reading them. `Tabaqa` calls `ahmil` on every path it
        // controls; this is for the paths it does not, such as a constructor
        // that failed after this value existed.
        let _ = self.ahmil();
    }
}

/// The borrowed resources one recorded frame needs.
///
/// Bundled into a struct rather than passed as eleven parameters because eleven
/// parameters of four interchangeable interface types is a call site where two
/// arguments can be swapped and everything still compiles.
#[derive(Debug)]
struct MawaridItar<'a> {
    mukhassis: &'a ID3D12CommandAllocator,
    qaima: &'a ID3D12GraphicsCommandList,
    ruus: &'a ID3D12Resource,
    masar_ruus: u64,
    siat_ruus: usize,
    mawdi_hadaf: D3D12_CPU_DESCRIPTOR_HANDLE,
    khalfiya: &'a ID3D12Resource,
    tawqee: &'a ID3D12RootSignature,
    halat: &'a ID3D12PipelineState,
    kawma: &'a ID3D12DescriptorHeap,
    faharis: &'a ID3D12Resource,
}

impl KhattafD3D12 {
    /// Records the overlay's draw into one frame's list and submits it.
    ///
    /// Returns the fence value the submission was signalled at, which the caller
    /// stores against the frame — that value is what the *next* use of this same
    /// frame waits on, and losing it would mean the next reset had nothing to
    /// check against.
    ///
    /// # Errors
    ///
    /// [`KhataTabaqa::MawridFashil`] when a reset, a map or a close is refused.
    /// Every one of those happens before anything is submitted and leaves the
    /// game's own rendering untouched, because none of it was ever recorded into
    /// a list the game owns. [`KhataTabaqa::HalaGhayrMustaada`] only from the
    /// submit itself — see `nafidh` for why that one is terminal.
    fn arsim_bi(
        &mut self,
        musawwada: &[Ras],
        lawha: &LawhatRasm,
        mawarid: &MawaridItar<'_>,
    ) -> Result<u64, KhataTabaqa> {
        let khatwa_ras = usize::try_from(KHATWAT_RAS).unwrap_or(mem::size_of::<Ras>());
        let matlub = musawwada.len();
        if matlub > mawarid.siat_ruus {
            return Err(KhataTabaqa::MawridFashil {
                mawrid: "overlay vertex buffer",
                sabab: format!(
                    "the batch needs {matlub} vertices and the frame's buffer holds {}",
                    mawarid.siat_ruus
                ),
            });
        }

        // SAFETY: the caller waited on this frame's fence before calling, so no
        // list this allocator holds is executing and the reset is defined. The
        // list is reset onto the allocator it was created with, and with the
        // overlay's own pipeline as its initial state — which is also what
        // clears every binding the previous recording left in it.
        unsafe { mawarid.mukhassis.Reset() }.map_err(|khata| KhataTabaqa::MawridFashil {
            mawrid: "overlay command allocator",
            sabab: khata.to_string(),
        })?;
        // SAFETY: `qaima` is closed — it is closed at creation and closed again
        // by `nafidh` at the end of every recording — and `mukhassis` was just
        // reset.
        unsafe { mawarid.qaima.Reset(mawarid.mukhassis, Some(mawarid.halat)) }.map_err(
            |khata| KhataTabaqa::MawridFashil {
                mawrid: "overlay command list",
                sabab: khata.to_string(),
            },
        )?;

        // SAFETY: `Ras` is `#[repr(C)]` and holds nothing but `f32` arrays and
        // one `f32`, so it has no padding bytes and no invalid bit patterns; the
        // slice therefore covers exactly `len * size_of::<Ras>()` initialised
        // bytes, which is what the length below says.
        let bayt = unsafe {
            core::slice::from_raw_parts(
                musawwada.as_ptr().cast::<u8>(),
                musawwada.len().saturating_mul(mem::size_of::<Ras>()),
            )
        };
        // Safe to overwrite because of the same wait: this buffer belongs to the
        // frame whose fence the caller already cleared, so no queued draw is
        // still reading it.
        iktub_fi(mawarid.ruus, bayt, "overlay vertex buffer")?;

        let manzar = D3D12_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: madaa_f32(lawha.sath.ard),
            Height: madaa_f32(lawha.sath.irtifa),
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        // D3D12 has no "scissoring disabled" state. A command list with no
        // scissor rectangle set rasterizes nothing at all, which looks exactly
        // like a draw that never happened — so the full surface is set here
        // every frame rather than assumed.
        let maqas = RECT {
            left: 0,
            top: 0,
            right: i32::try_from(lawha.sath.ard).unwrap_or(i32::MAX),
            bottom: i32::try_from(lawha.sath.irtifa).unwrap_or(i32::MAX),
        };
        // SAFETY: `faharis` is the live, committed index buffer this call
        // borrows, which is the one resource kind whose GPU address is defined
        // and stable. Reading it from the borrow rather than from a cached copy
        // is what makes "the buffer this view addresses is alive for the whole
        // recording" a fact the borrow checker enforces rather than a comment.
        let masar_faharis = unsafe { mawarid.faharis.GetGPUVirtualAddress() };
        let manzur_faharis = D3D12_INDEX_BUFFER_VIEW {
            BufferLocation: masar_faharis,
            SizeInBytes: u32::try_from(QITA_LIL_DUFA.saturating_mul(24)).unwrap_or(0),
            Format: DXGI_FORMAT_R32_UINT,
        };
        let ila_hadaf = hajiz_intiqal(
            mawarid.khalfiya,
            D3D12_RESOURCE_STATE_PRESENT,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
        );
        let ila_ard = hajiz_intiqal(
            mawarid.khalfiya,
            D3D12_RESOURCE_STATE_RENDER_TARGET,
            D3D12_RESOURCE_STATE_PRESENT,
        );
        let amil_khalt = [0.0_f32; 4];

        // SAFETY: every interface used below is borrowed from a local the caller
        // holds for the whole call, the two barriers name the backbuffer that
        // outlives this block, `mawdi_hadaf` is the descriptor this frame's
        // render target view was written into, and the index buffer view covers
        // exactly the buffer `ibni_faharis` filled. The projection buffer is
        // reached through `self`, which is mutably borrowed for the whole call
        // and therefore cannot drop it mid-recording.
        unsafe {
            mawarid.qaima.ResourceBarrier(&[ila_hadaf]);

            mawarid.qaima.SetGraphicsRootSignature(Some(mawarid.tawqee));
            mawarid.qaima.SetDescriptorHeaps(&[Some(mawarid.kawma.clone())]);
            mawarid.qaima.SetGraphicsRootConstantBufferView(0, self.masar_thawabit);
            let jadwal = mawarid.kawma.GetGPUDescriptorHandleForHeapStart();
            mawarid.qaima.SetGraphicsRootDescriptorTable(1, jadwal);

            mawarid.qaima.OMSetRenderTargets(1, Some(&raw const mawarid.mawdi_hadaf), false, None);
            mawarid.qaima.OMSetBlendFactor(Some(&amil_khalt));
            mawarid.qaima.OMSetStencilRef(0);
            mawarid.qaima.RSSetViewports(&[manzar]);
            mawarid.qaima.RSSetScissorRects(&[maqas]);
            mawarid.qaima.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            mawarid.qaima.IASetIndexBuffer(Some(&raw const manzur_faharis));

            for (raqm, dufa) in lawha.qitaat.chunks(QITA_LIL_DUFA).enumerate() {
                // Every chunk but the last is full, so a chunk's first vertex is
                // at `raqm * QITA_LIL_DUFA * 4` — the same arithmetic the writer
                // above used implicitly by appending them in order.
                let izaha = raqm
                    .saturating_mul(QITA_LIL_DUFA)
                    .saturating_mul(4)
                    .saturating_mul(khatwa_ras);
                let tul = dufa.len().saturating_mul(4).saturating_mul(khatwa_ras);
                let manzur = D3D12_VERTEX_BUFFER_VIEW {
                    BufferLocation: mawarid.masar_ruus.saturating_add(tul_u64(izaha)),
                    SizeInBytes: u32::try_from(tul).unwrap_or(0),
                    StrideInBytes: KHATWAT_RAS,
                };
                mawarid.qaima.IASetVertexBuffers(0, Some(&[manzur]));
                let adad = u32::try_from(dufa.len().saturating_mul(6)).unwrap_or(0);
                mawarid.qaima.DrawIndexedInstanced(adad, 1, 0, 0, 0);
            }

            // Put back before the list is submitted. Presenting a backbuffer
            // that is still in `RENDER_TARGET` is a device removal, attributed
            // to the game.
            mawarid.qaima.ResourceBarrier(&[ila_ard]);
        }

        self.nafidh(mawarid.qaima)
    }
}

/// Copies a captured region out of a mapped read-back buffer, dropping the
/// pitch.
///
/// D3D12 aligns every placed footprint's rows to
/// [`MUHADHAT_SAF`] bytes, so a 1000-pixel-wide BGRA row occupies 4096 bytes in
/// the buffer and 4000 of them are the image. Reading it as one contiguous block
/// produces a picture that shears further left on every row, which the
/// recognizer reports as unreadable text rather than as a stride bug.
/// [`Khattaf::iltaqit`] promises tightly packed bytes, so the repack happens
/// here.
///
/// # Errors
///
/// [`KhataTabaqa::IltiqatFashil`] when the mapping is refused, produces no
/// pointer, or describes a buffer too small for the rows it claims to hold.
fn jami_sufuf(
    qira: &ID3D12Resource,
    takhtit: &D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    mintaqa: MustatilBiksel,
    sigha: SighatSath,
    al_kul: u64,
) -> Result<Vec<u8>, KhataTabaqa> {
    let tul_saf = mintaqa.ard.saturating_mul(sigha.bayt_lil_biksel());
    let tul_saf = usize::try_from(tul_saf).unwrap_or(usize::MAX);
    let khatwa = usize::try_from(takhtit.Footprint.RowPitch).unwrap_or(usize::MAX);
    let irtifa = usize::try_from(mintaqa.irtifa).unwrap_or(0);
    let tul_kul = usize::try_from(al_kul).unwrap_or(0);
    if tul_saf == 0 || khatwa < tul_saf {
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: format!(
                "the read-back footprint reports a {khatwa}-byte pitch for a {tul_saf}-byte row"
            ),
        });
    }

    let mut asas: *mut core::ffi::c_void = core::ptr::null_mut();
    let mada = D3D12_RANGE { Begin: 0, End: tul_kul };
    // SAFETY: `qira` is a live read-back buffer with one subresource, the read
    // range names exactly the bytes the copy wrote, and both pointers address
    // live locals.
    unsafe { qira.Map(0, Some(&raw const mada), Some(&raw mut asas)) }.map_err(|khata| {
        KhataTabaqa::IltiqatFashil {
            sabab: format!("the read-back buffer could not be mapped: {khata}"),
        }
    })?;

    // Nothing was written by the CPU, and telling `Unmap` so is what lets the
    // driver skip a flush of a buffer that is about to be freed.
    let la_shay = D3D12_RANGE { Begin: 0, End: 0 };

    if asas.is_null() {
        // SAFETY: the map succeeded, so the resource is mapped and must be
        // unmapped even though the pointer it produced is unusable.
        unsafe { qira.Unmap(0, Some(&raw const la_shay)) };
        return Err(KhataTabaqa::IltiqatFashil {
            sabab: "the read-back mapping has no pointer".to_owned(),
        });
    }

    // SAFETY: `asas` is the base of a mapping covering the whole subresource,
    // whose byte length is `al_kul` — the total the device itself reported for
    // this footprint — and that length is passed along so the copy can bound
    // every row against it.
    let natija = unsafe { naskh_sufuf(asas.cast::<u8>(), tul_kul, khatwa, tul_saf, irtifa) };

    // SAFETY: the map above succeeded and this is its matching unmap, run before
    // the result is inspected so no error path leaves the buffer mapped.
    unsafe { qira.Unmap(0, Some(&raw const la_shay)) };
    natija
}

/// Copies `irtifa` rows of `tul_saf` bytes out of a mapping, dropping the pitch.
///
/// # Safety
///
/// `asas` must point at `tul_kul` readable, initialised bytes that stay mapped
/// for the whole call. Every row this reads is bounds-checked against `tul_kul`
/// before it is formed, so a pitch or height that does not fit the mapping
/// produces an error rather than a read past the end.
unsafe fn naskh_sufuf(
    asas: *const u8,
    tul_kul: usize,
    khatwa: usize,
    tul_saf: usize,
    irtifa: usize,
) -> Result<Vec<u8>, KhataTabaqa> {
    let mut khuruj = Vec::with_capacity(tul_saf.saturating_mul(irtifa));
    for saf in 0..irtifa {
        let bidaya = saf.saturating_mul(khatwa);
        if bidaya.saturating_add(tul_saf) > tul_kul {
            return Err(KhataTabaqa::IltiqatFashil {
                sabab: format!(
                    "row {saf} would end at {} in a {tul_kul}-byte read-back buffer",
                    bidaya.saturating_add(tul_saf)
                ),
            });
        }
        // SAFETY: the caller guarantees `tul_kul` readable bytes from `asas`,
        // and the check immediately above establishes that this row's last byte
        // is inside that range.
        let bayt = unsafe { core::slice::from_raw_parts(asas.add(bidaya), tul_saf) };
        khuruj.extend_from_slice(bayt);
    }
    Ok(khuruj)
}
